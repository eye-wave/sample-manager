//! X11: a child window.
use super::*;
use std::ptr::null;
use x11::xlib;

pub const API: GuiApiType<'static> = GuiApiType::X11;
pub const EXTS: [&CStr; 2] = [khr::surface::NAME, khr::xlib_surface::NAME];

// Display* stored as usize so `Native` is Send + Sync. Xlib is not thread-safe without
// XInitThreads, so each thread gets its own connection: `dpy` is for the main thread (create,
// map, resize, query, destroy), `vdpy` is only handed to Vulkan on the render thread.
pub struct Native {
    dpy: usize,
    vdpy: usize,
    win: xlib::Window,
}

impl Native {
    pub fn new(parent: &Window, w: u32, h: u32) -> Result<Self, PluginError> {
        let parent = parent
            .as_x11_handle()
            .ok_or(PluginError::Message("not an X11 parent"))?;
        unsafe {
            // Own connections, so we never share the host's Display.
            let dpy = xlib::XOpenDisplay(null());
            let vdpy = xlib::XOpenDisplay(null());
            if dpy.is_null() || vdpy.is_null() {
                return Err(PluginError::Message("XOpenDisplay failed"));
            }
            let win = xlib::XCreateSimpleWindow(dpy, parent, 0, 0, w, h, 0, 0, 0);
            xlib::XFlush(dpy); // so `vdpy` can see the window
            Ok(Self {
                dpy: dpy as usize,
                vdpy: vdpy as usize,
                win,
            })
        }
    }

    pub fn show(&self) -> Result<(), PluginError> {
        unsafe {
            xlib::XMapWindow(self.dpy as _, self.win);
            xlib::XFlush(self.dpy as _);
        }
        Ok(())
    }

    pub fn hide(&self) -> Result<(), PluginError> {
        unsafe {
            xlib::XUnmapWindow(self.dpy as _, self.win);
            xlib::XFlush(self.dpy as _);
        }
        Ok(())
    }

    pub fn resize(&self, w: u32, h: u32) {
        unsafe {
            xlib::XResizeWindow(self.dpy as _, self.win, w, h);
            xlib::XFlush(self.dpy as _);
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

impl Drop for Native {
    fn drop(&mut self) {
        unsafe {
            xlib::XDestroyWindow(self.dpy as _, self.win);
            xlib::XCloseDisplay(self.dpy as _); // flushes the destroy
            xlib::XCloseDisplay(self.vdpy as _); // the render thread is gone by now
        }
    }
}
