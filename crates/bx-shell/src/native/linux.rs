//! X11: a child window.
use super::*;
use std::ptr::null;
use x11::xlib;

// Display* stored as usize so `Native` is Send + Sync. Xlib is not thread-safe without
// XInitThreads, so each thread gets its own connection: `dpy` is for the main thread (create,
// map, resize, query, destroy), `vdpy` is only handed to Vulkan on the render thread.
pub struct Native {
    dpy: usize,
    vdpy: usize,
    win: xlib::Window,
    /// Hand, I-beam and resize corner (the arrow is none).
    cursors: [xlib::Cursor; 3],
}

impl Native {
    pub fn new(parent: usize, w: u32, h: u32) -> Result<Self, &'static str> {
        unsafe {
            // Own connections, so we never share the host's Display.
            let dpy = xlib::XOpenDisplay(null());
            let vdpy = xlib::XOpenDisplay(null());
            if dpy.is_null() || vdpy.is_null() {
                return Err("XOpenDisplay failed");
            }
            let win = xlib::XCreateSimpleWindow(dpy, parent as xlib::Window, 0, 0, w, h, 0, 0, 0);
            xlib::XSetWindowBackgroundPixmap(dpy, win, 0); // None: the server must not clear on resize
            // the host never forwards input to a child window; ask the server for it directly
            xlib::XSelectInput(
                dpy,
                win,
                xlib::KeyPressMask | xlib::ButtonPressMask | xlib::ButtonReleaseMask,
            );
            xlib::XFlush(dpy); // so `vdpy` can see the window
            Ok(Self {
                dpy: dpy as usize,
                vdpy: vdpy as usize,
                win,
                cursors: [
                    xlib::XCreateFontCursor(dpy, 60),  // XC_hand2
                    xlib::XCreateFontCursor(dpy, 152), // XC_xterm
                    xlib::XCreateFontCursor(dpy, 14),  // XC_bottom_right_corner
                ],
            })
        }
    }

    pub fn show(&self) {
        unsafe {
            xlib::XMapWindow(self.dpy as _, self.win);
            xlib::XFlush(self.dpy as _);
        }
    }

    pub fn hide(&self) {
        unsafe {
            xlib::XUnmapWindow(self.dpy as _, self.win);
            xlib::XFlush(self.dpy as _);
        }
    }

    pub fn resize(&self, w: u32, h: u32) {
        unsafe {
            xlib::XResizeWindow(self.dpy as _, self.win, w, h);
            xlib::XFlush(self.dpy as _);
        }
    }

    /// Drains this window's key and click events into `events`. A click also takes the keyboard
    /// focus: the host's window has it otherwise, and typing would never get here.
    pub fn poll(&self, events: &crate::input::Events) {
        let dpy = self.dpy as *mut xlib::Display;
        unsafe {
            let mut ev: xlib::XEvent = std::mem::zeroed();
            while xlib::XPending(dpy) > 0 {
                xlib::XNextEvent(dpy, &mut ev);
                match ev.get_type() {
                    xlib::ButtonPress if ev.button.button == 1 => {
                        xlib::XSetInputFocus(
                            dpy,
                            self.win,
                            xlib::RevertToParent,
                            xlib::CurrentTime,
                        );
                        events.push(crate::input::Event::Click(ev.button.x, ev.button.y));
                    }
                    xlib::KeyPress => crate::window::push_key(events, &mut ev.key),
                    _ => {}
                }
            }
        }
    }

    pub fn pointer(&self) -> Pointer {
        let (mut root, mut child, mut mask) = (0, 0, 0);
        let (mut sx, mut sy, mut x, mut y) = (0, 0, 0, 0);
        unsafe {
            xlib::XQueryPointer(
                self.dpy as _,
                self.win,
                &mut root,
                &mut child,
                &mut sx,
                &mut sy,
                &mut x,
                &mut y,
                &mut mask,
            );
        }
        Pointer {
            x,
            y,
            sx,
            sy,
            down: mask & xlib::Button1Mask != 0,
        }
    }
}

unsafe impl Target for Native {
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
        let c = match cursor {
            crate::gpu::CURSOR_HAND => self.cursors[0],
            crate::gpu::CURSOR_TEXT => self.cursors[1],
            crate::gpu::CURSOR_RESIZE => self.cursors[2],
            _ => 0,
        };
        unsafe {
            xlib::XDefineCursor(dpy, self.win, c);
            xlib::XFlush(dpy);
        }
    }
}

impl Drop for Native {
    fn drop(&mut self) {
        unsafe {
            xlib::XDestroyWindow(self.dpy as _, self.win);
            xlib::XCloseDisplay(self.dpy as _); // flushes the destroy
            xlib::XCloseDisplay(self.vdpy as _); // the render thread is gone by now
        }
    }
}
