//! X11: a top-level window. Wayland runs through XWayland.
use super::Grip;
use crate::gpu::{NO_MOUSE, Target, pack_mouse};
use ash::{Entry, Instance, khr, vk};
use std::ffi::CString;
use std::ptr::null;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use x11::xlib;

// Display* stored as usize so `Window` is Sync. Xlib is not thread-safe without XInitThreads, so
// each thread gets its own connection: `dpy` is for the thread of `new` / `run` / `drop`, `vdpy`
// is only handed to Vulkan on the render thread.
pub struct Window {
    dpy: usize,
    vdpy: usize,
    win: xlib::Window,
    delete: xlib::Atom,
    grip_cursor: xlib::Cursor,
    hand_cursor: xlib::Cursor,
    size: (u32, u32),
    min_w: u32,
    grip: Grip,
    presented: AtomicU64,
    mouse: AtomicU64,
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
                grip_cursor: xlib::XCreateFontCursor(dpy, 14), // XC_bottom_right_corner
                hand_cursor: xlib::XCreateFontCursor(dpy, 60), // XC_hand2
                size,
                min_w: min.0,
                grip,
                presented: AtomicU64::new(0),
                mouse: AtomicU64::new(NO_MOUSE),
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

    pub fn run(&self) {
        let dpy = self.dpy as *mut xlib::Display;
        let (mut w, mut h) = self.size;
        // a resize drag: the mouse x on the screen and the window width when it started
        let mut drag: Option<(i32, u32)> = None;
        // the cursor is the renderer's (see `set_cursor`) except on the handle
        let mut on_grip = false;
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
                        } else {
                            let on = (self.grip)(ev.motion.x, ev.motion.y, w, h);
                            if on != on_grip {
                                on_grip = on;
                                xlib::XDefineCursor(
                                    dpy,
                                    self.win,
                                    if on { self.grip_cursor } else { 0 },
                                );
                            }
                        }
                    }
                    xlib::ButtonPress
                        if ev.button.button == 1 && (self.grip)(ev.button.x, ev.button.y, w, h) =>
                    {
                        drag = Some((ev.button.x_root, w));
                    }
                    xlib::ButtonRelease => drag = None,
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

    fn set_cursor(&self, pointer: bool) {
        // the render thread's own connection, the cursor itself is a server object
        let (dpy, c) = (self.vdpy as *mut xlib::Display, self.hand_cursor);
        unsafe {
            xlib::XDefineCursor(dpy, self.win, if pointer { c } else { 0 });
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
