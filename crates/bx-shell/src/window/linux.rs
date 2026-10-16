//! X11: a top-level window. Wayland runs through XWayland.
use super::Grip;
use crate::gpu::{CURSOR_HAND, CURSOR_RESIZE, CURSOR_TEXT, NO_MOUSE, Target, pack_mouse};
use crate::input::{Event, Events, Key};
use ash::{Entry, Instance, khr, vk};
use std::ffi::CString;
use std::ptr::null;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use x11::xlib;

// Display* stored as usize so `Window` is Sync. Xlib is not thread-safe without XInitThreads, so
// each thread gets its own connection: `dpy` is for the thread of `new` / `run` / `drop`, `vdpy`
// is only handed to Vulkan on the render thread.
pub struct Window {
    dpy: usize,
    vdpy: usize,
    win: xlib::Window,
    delete: xlib::Atom,
    /// Hand, I-beam and resize corner (the arrow is none).
    cursors: [xlib::Cursor; 3],
    /// A resize drag is on: the cursor stays the corner even when the mouse is off the handle.
    dragging: AtomicBool,
    size: (u32, u32),
    min_w: u32,
    grip: Grip,
    presented: AtomicU64,
    mouse: AtomicU64,
    events: Events,
}

/// Makes the window `w` x `h`. The size hints say min = max, so the window manager offers no way
/// to resize by the edges, and they are updated first so that it accepts the new size.
unsafe fn set_size(dpy: *mut xlib::Display, win: xlib::Window, w: u32, h: u32) {
    unsafe {
        let mut hints: xlib::XSizeHints = std::mem::zeroed();
        hints.flags = xlib::PMinSize | xlib::PMaxSize;
        (hints.min_width, hints.min_height) = (w as _, h as _);
        (hints.max_width, hints.max_height) = (w as _, h as _);
        xlib::XSetWMNormalHints(dpy, win, &mut hints);
        xlib::XResizeWindow(dpy, win, w, h);
    }
}

/// Turns an X11 key press into `Key` / `Char` events.
pub(crate) unsafe fn push_key(events: &Events, ev: &mut xlib::XKeyEvent) {
    let mut buf = [0u8; 8];
    let mut sym = 0;
    let n = unsafe {
        xlib::XLookupString(
            ev,
            buf.as_mut_ptr() as _,
            buf.len() as _,
            &mut sym,
            std::ptr::null_mut(),
        )
    };
    use x11::keysym::*;
    #[allow(non_upper_case_globals)] // the keysym constants are named like in X11
    let ctrl = ev.state & xlib::ControlMask != 0;
    let key = match sym as u32 {
        XK_BackSpace if ctrl => Some(Key::CtrlBackspace),
        XK_Delete if ctrl => Some(Key::CtrlDelete),
        XK_a | XK_A if ctrl => Some(Key::CtrlA),
        XK_BackSpace => Some(Key::Backspace),
        XK_Delete => Some(Key::Delete),
        XK_Left => Some(Key::Left),
        XK_Right => Some(Key::Right),
        XK_Home => Some(Key::Home),
        XK_End => Some(Key::End),
        XK_Return | XK_KP_Enter => Some(Key::Enter),
        XK_Escape => Some(Key::Escape),
        _ => None,
    };
    if let Some(k) = key {
        events.push(Event::Key(k));
    } else {
        // ponytail: Latin-1 only (XLookupString), Xutf8LookupString needs an input method
        for &b in &buf[..n.max(0) as usize] {
            if b >= 32 && b != 127 {
                events.push(Event::Char(b as char));
            }
        }
    }
}

impl Window {
    pub fn new(title: &str, size: (u32, u32), min: (u32, u32), grip: Grip) -> Self {
        unsafe {
            let (dpy, vdpy) = (xlib::XOpenDisplay(null()), xlib::XOpenDisplay(null()));
            assert!(!dpy.is_null() && !vdpy.is_null(), "XOpenDisplay failed");
            let root = xlib::XDefaultRootWindow(dpy);
            let win = xlib::XCreateSimpleWindow(dpy, root, 0, 0, size.0, size.1, 0, 0, 0);
            xlib::XStoreName(dpy, win, CString::new(title).unwrap().as_ptr());
            // flicker: no background (the server would paint it over our frame on every resize),
            // and the old contents stay put until the next frame replaces them
            xlib::XSetWindowBackgroundPixmap(dpy, win, 0);
            let mut attrs: xlib::XSetWindowAttributes = std::mem::zeroed();
            attrs.bit_gravity = xlib::NorthWestGravity;
            xlib::XChangeWindowAttributes(dpy, win, xlib::CWBitGravity, &mut attrs);
            set_size(dpy, win, size.0, size.1);
            let mut delete = xlib::XInternAtom(dpy, c"WM_DELETE_WINDOW".as_ptr(), 0);
            xlib::XSetWMProtocols(dpy, win, &mut delete, 1);
            xlib::XSelectInput(
                dpy,
                win,
                xlib::StructureNotifyMask
                    | xlib::PointerMotionMask
                    | xlib::LeaveWindowMask
                    | xlib::KeyPressMask
                    | xlib::ButtonPressMask
                    | xlib::ButtonReleaseMask,
            );
            xlib::XMapWindow(dpy, win);
            xlib::XFlush(dpy); // so `vdpy` can see the window
            Self {
                dpy: dpy as usize,
                vdpy: vdpy as usize,
                win,
                delete,
                cursors: [
                    xlib::XCreateFontCursor(dpy, 60),  // XC_hand2
                    xlib::XCreateFontCursor(dpy, 152), // XC_xterm
                    xlib::XCreateFontCursor(dpy, 14),  // XC_bottom_right_corner
                ],
                dragging: AtomicBool::new(false),
                size,
                min_w: min.0,
                grip,
                presented: AtomicU64::new(0),
                mouse: AtomicU64::new(NO_MOUSE),
                events: Events::new(),
            }
        }
    }

    /// For the renderer (`render`'s `presented`). X11 does not wait for it: the missing
    /// background is what keeps the resize calm.
    pub fn presented(&self) -> &AtomicU64 {
        &self.presented
    }

    /// For the renderer (`render`'s mouse): where the mouse is on the window, for hover effects.
    pub fn mouse(&self) -> &AtomicU64 {
        &self.mouse
    }

    /// For the renderer (`render`'s events): what is clicked and typed.
    pub fn events(&self) -> &Events {
        &self.events
    }

    pub fn run(&self) {
        let dpy = self.dpy as *mut xlib::Display;
        let (mut w, mut h) = self.size;
        // a resize drag: the mouse x on the screen and the window width when it started
        let mut drag: Option<(i32, u32)> = None;
        unsafe {
            let mut ev: xlib::XEvent = std::mem::zeroed();
            loop {
                xlib::XNextEvent(dpy, &mut ev);
                match ev.get_type() {
                    xlib::ConfigureNotify => {
                        (w, h) = (ev.configure.width as u32, ev.configure.height as u32)
                    }
                    // the button is held, so these keep coming to this window even outside of it
                    xlib::LeaveNotify => self.mouse.store(NO_MOUSE, Relaxed),
                    xlib::MotionNotify => {
                        self.mouse
                            .store(pack_mouse(ev.motion.x, ev.motion.y), Relaxed);
                        if let Some((x0, w0)) = drag {
                            let nw =
                                (w0 as i32 + ev.motion.x_root - x0).max(self.min_w as i32) as u32;
                            let nh = nw * self.size.1 / self.size.0;
                            if (nw, nh) != (w, h) {
                                set_size(dpy, self.win, nw, nh);
                            }
                        }
                    }
                    xlib::KeyPress => push_key(&self.events, &mut ev.key),
                    xlib::ButtonPress if ev.button.button == 1 => {
                        self.events.push(Event::Click(ev.button.x, ev.button.y));
                        if (self.grip)(ev.button.x, ev.button.y, w, h) {
                            drag = Some((ev.button.x_root, w));
                            self.dragging.store(true, Relaxed);
                        }
                    }
                    xlib::ButtonPress if matches!(ev.button.button, 4 | 5) => {
                        self.events
                            .push(Event::Scroll(if ev.button.button == 4 { -1 } else { 1 }));
                    }
                    xlib::ButtonRelease => {
                        self.events.push(Event::Release);
                        drag = None;
                        self.dragging.store(false, Relaxed);
                    }
                    xlib::ClientMessage
                        if ev.client_message.data.get_long(0) as xlib::Atom == self.delete =>
                    {
                        return;
                    }
                    _ => {}
                }
            }
        }
    }
}

unsafe impl Target for Window {
    fn surface(&self, entry: &Entry, inst: &Instance) -> vk::SurfaceKHR {
        unsafe {
            khr::xlib_surface::Instance::new(entry, inst)
                .create_xlib_surface(
                    &vk::XlibSurfaceCreateInfoKHR::default()
                        .dpy(self.vdpy as _)
                        .window(self.win),
                    None,
                )
                .unwrap()
        }
    }

    fn set_cursor(&self, cursor: u8) {
        // the render thread's own connection, the cursor itself is a server object
        let dpy = self.vdpy as *mut xlib::Display;
        let cursor = if self.dragging.load(Relaxed) {
            CURSOR_RESIZE
        } else {
            cursor
        };
        let c = match cursor {
            CURSOR_HAND => self.cursors[0],
            CURSOR_TEXT => self.cursors[1],
            CURSOR_RESIZE => self.cursors[2],
            _ => 0,
        };
        unsafe {
            xlib::XDefineCursor(dpy, self.win, c);
            xlib::XFlush(dpy);
        }
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            xlib::XDestroyWindow(self.dpy as _, self.win);
            xlib::XCloseDisplay(self.dpy as _); // flushes the destroy
            xlib::XCloseDisplay(self.vdpy as _); // the render thread is gone by now
        }
    }
}
