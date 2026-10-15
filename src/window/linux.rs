//! X11: a top-level window. Wayland runs through XWayland.
use super::Grip;
use ash::{Entry, Instance, khr, vk};
use std::ffi::{CStr, CString, c_long};
use std::ptr::null;
use x11::xlib;

pub const EXTS: [&CStr; 2] = [khr::surface::NAME, khr::xlib_surface::NAME];

// Display* stored as usize so `Window` is Sync. Xlib is not thread-safe without XInitThreads, so
// each thread gets its own connection: `dpy` is for the thread of `new` / `run` / `drop`, `vdpy`
// is only handed to Vulkan on the render thread.
pub struct Window {
    dpy: usize,
    vdpy: usize,
    win: xlib::Window,
    root: xlib::Window,
    delete: xlib::Atom,
    moveresize: xlib::Atom,
    grip_cursor: xlib::Cursor,
    size: (u32, u32),
    grip: Grip,
}

impl Window {
    pub fn new(title: &str, size: (u32, u32), min: (u32, u32), grip: Grip) -> Self {
        unsafe {
            let (dpy, vdpy) = (xlib::XOpenDisplay(null()), xlib::XOpenDisplay(null()));
            assert!(!dpy.is_null() && !vdpy.is_null(), "XOpenDisplay failed");
            let atom = |name: &CStr| xlib::XInternAtom(dpy, name.as_ptr(), 0);
            let root = xlib::XDefaultRootWindow(dpy);
            let win = xlib::XCreateSimpleWindow(dpy, root, 0, 0, size.0, size.1, 0, 0, 0);
            xlib::XStoreName(dpy, win, CString::new(title).unwrap().as_ptr());
            let mut hints: xlib::XSizeHints = std::mem::zeroed();
            hints.flags = xlib::PMinSize;
            (hints.min_width, hints.min_height) = (min.0 as _, min.1 as _);
            xlib::XSetWMNormalHints(dpy, win, &mut hints);
            let mut delete = atom(c"WM_DELETE_WINDOW");
            xlib::XSetWMProtocols(dpy, win, &mut delete, 1);
            xlib::XSelectInput(
                dpy,
                win,
                xlib::StructureNotifyMask | xlib::PointerMotionMask | xlib::ButtonPressMask,
            );
            xlib::XMapWindow(dpy, win);
            xlib::XFlush(dpy); // so `vdpy` can see the window
            Self {
                dpy: dpy as usize,
                vdpy: vdpy as usize,
                win,
                root,
                delete,
                moveresize: atom(c"_NET_WM_MOVERESIZE"),
                grip_cursor: xlib::XCreateFontCursor(dpy, 14), // XC_bottom_right_corner
                size,
                grip,
            }
        }
    }

    pub fn run(&self) {
        let dpy = self.dpy as *mut xlib::Display;
        let (mut w, mut h) = self.size;
        unsafe {
            let mut ev: xlib::XEvent = std::mem::zeroed();
            loop {
                xlib::XNextEvent(dpy, &mut ev);
                match ev.get_type() {
                    xlib::ConfigureNotify => {
                        (w, h) = (ev.configure.width as u32, ev.configure.height as u32)
                    }
                    xlib::MotionNotify => {
                        let on = (self.grip)(ev.motion.x, ev.motion.y, w, h);
                        xlib::XDefineCursor(dpy, self.win, if on { self.grip_cursor } else { 0 });
                    }
                    xlib::ButtonPress
                        if ev.button.button == 1 && (self.grip)(ev.button.x, ev.button.y, w, h) =>
                    {
                        // the window manager takes over the drag: the mouse position on the
                        // screen, direction 4 (bottom right), button 1, "sent by an application"
                        xlib::XUngrabPointer(dpy, xlib::CurrentTime);
                        let mut data = xlib::ClientMessageData::new();
                        let args = [
                            ev.button.x_root as c_long,
                            ev.button.y_root as c_long,
                            4,
                            1,
                            1,
                        ];
                        for (i, a) in args.into_iter().enumerate() {
                            data.set_long(i, a);
                        }
                        let mut msg: xlib::XEvent = xlib::XClientMessageEvent {
                            type_: xlib::ClientMessage,
                            serial: 0,
                            send_event: 0,
                            display: dpy,
                            window: self.win,
                            message_type: self.moveresize,
                            format: 32,
                            data,
                        }
                        .into();
                        let mask = xlib::SubstructureRedirectMask | xlib::SubstructureNotifyMask;
                        xlib::XSendEvent(dpy, self.root, 0, mask, &mut msg);
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

    pub unsafe fn surface(&self, entry: &Entry, inst: &Instance) -> vk::SurfaceKHR {
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
