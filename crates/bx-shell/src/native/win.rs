//! Win32: a child window.
use super::*;
use crate::gpu::{NO_MOUSE, pack_mouse};
use crate::input::{Event, Events, Key, moved};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};
use windows_sys::Win32::Foundation::{HMODULE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
use windows_sys::Win32::System::LibraryLoader::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, ReleaseCapture, SetCapture, SetFocus, TME_LEAVE,
    TRACKMOUSEEVENT, TrackMouseEvent, VK_CONTROL, VK_DELETE, VK_END, VK_HOME, VK_LBUTTON, VK_LEFT,
    VK_RIGHT,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// Plain values: where these constants live differs between windows-sys versions.
const WM_GETDLGCODE: u32 = 0x0087;
const WM_MOUSELEAVE: u32 = 0x02A3;
const DLGC_WANTALLKEYS: isize = 0x0004;

/// Where `proc` puts the input the host's message loop delivers to the window (`Native::attach`).
/// Not polled on a host timer: not every host has one (FL Studio does not seem to).
struct Hooks {
    events: Arc<Events>,
    mouse: Arc<AtomicU64>,
    /// A WM_MOUSELEAVE is pending.
    tracking: AtomicBool,
    /// A press on a file: where, and the file; a drag out of the window once the mouse moves.
    press: Mutex<Option<(i32, i32, std::path::PathBuf)>>,
}

unsafe extern "system" fn proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        let hooks = GetWindowLongPtrW(h, GWLP_USERDATA) as *const Hooks;
        if hooks.is_null() {
            return DefWindowProcW(h, m, w, l); // not attached yet
        }
        let hooks = &*hooks;
        let push = |e| hooks.events.push(e);
        match m {
            WM_MOUSEMOVE => {
                let (x, y) = (l as i16 as i32, (l >> 16) as i16 as i32);
                hooks.mouse.store(pack_mouse(x, y), Relaxed);
                if !hooks.tracking.swap(true, Relaxed) {
                    let mut t = TRACKMOUSEEVENT {
                        cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: h,
                        dwHoverTime: 0,
                    };
                    TrackMouseEvent(&mut t);
                }
                let start = {
                    let mut g = hooks.press.lock().unwrap();
                    if g.as_ref().is_some_and(|&(x0, y0, _)| moved(x0, y0, x, y)) {
                        g.take()
                    } else {
                        None
                    }
                };
                if let Some((_, _, path)) = start {
                    ReleaseCapture(); // DoDragDrop takes the mouse itself
                    push(Event::Release);
                    crate::dnd::drag(&path); // returns when it is dropped
                }
                0
            }
            WM_MOUSELEAVE => {
                hooks.tracking.store(false, Relaxed);
                hooks.mouse.store(NO_MOUSE, Relaxed);
                0
            }
            // the host owns the keyboard focus until we take it: without this, typing never gets here
            WM_LBUTTONDOWN => {
                SetFocus(h);
                SetCapture(h); // so that the release arrives wherever the mouse is
                let (x, y) = (l as i16 as i32, (l >> 16) as i16 as i32);
                push(Event::Click(x, y));
                *hooks.press.lock().unwrap() = hooks.events.drag_source().map(|p| (x, y, p));
                0
            }
            WM_LBUTTONUP | WM_CAPTURECHANGED => {
                push(Event::Release);
                *hooks.press.lock().unwrap() = None;
                if m == WM_LBUTTONUP {
                    ReleaseCapture();
                }
                0
            }
            // whole notches only, a smooth wheel (delta < 120) does not scroll
            WM_MOUSEWHEEL => {
                push(Event::Scroll(-((w >> 16) as i16 as i32) / 120));
                0
            }
            // ask for the keys the host would otherwise take for its own dialog handling
            WM_GETDLGCODE => DLGC_WANTALLKEYS,
            // typed text: UTF-16 units (characters outside the BMP arrive as two
            // surrogates and are dropped); Backspace, Enter and Escape are control characters here
            WM_CHAR => {
                match char::from_u32(w as u32) {
                    Some('\u{8}') => push(Event::Key(Key::Backspace)),
                    Some('\u{7f}') => push(Event::Key(Key::CtrlBackspace)),
                    Some('\u{1}') => push(Event::Key(Key::CtrlA)),
                    Some('\r') => push(Event::Key(Key::Enter)),
                    Some('\u{1b}') => push(Event::Key(Key::Escape)),
                    Some(c) if !c.is_control() => push(Event::Char(c)),
                    _ => {}
                }
                0
            }
            WM_KEYDOWN => {
                match w as u16 {
                    VK_DELETE if GetKeyState(VK_CONTROL as i32) < 0 => {
                        push(Event::Key(Key::CtrlDelete))
                    }
                    VK_DELETE => push(Event::Key(Key::Delete)),
                    VK_LEFT => push(Event::Key(Key::Left)),
                    VK_RIGHT => push(Event::Key(Key::Right)),
                    VK_HOME => push(Event::Key(Key::Home)),
                    VK_END => push(Event::Key(Key::End)),
                    _ => {}
                }
                DefWindowProcW(h, m, w, l) // so that WM_CHAR follows
            }
            _ => DefWindowProcW(h, m, w, l),
        }
    }
}

// Raw handles stored as usize so `Native` is Send + Sync (the render thread only needs the HWND).
pub struct Native {
    hwnd: usize,
    hinst: usize,
    hooks: Mutex<Option<Arc<Hooks>>>,
}

impl Native {
    pub fn new(parent: usize, w: u32, h: u32) -> Result<Self, &'static str> {
        unsafe {
            // Our own DLL's handle (not the host exe's), found from the address of this function.
            let mut hinst: HMODULE = null_mut();
            GetModuleHandleExW(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
                    | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                Self::new as *const () as usize as *const u16,
                &mut hinst,
            );

            let class: Vec<u16> = "SampleManagerGui\0".encode_utf16().collect();
            let mut wc: WNDCLASSEXW = std::mem::zeroed();
            wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
            wc.lpfnWndProc = Some(proc);
            wc.hInstance = hinst;
            wc.lpszClassName = class.as_ptr();
            // fails harmlessly if another plugin instance already registered it; never
            // unregistered. Unregister on DLL unload if that ever matters.
            RegisterClassExW(&wc);

            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                null(),
                WS_CHILD | WS_CLIPSIBLINGS | WS_CLIPCHILDREN,
                0,
                0,
                w as i32,
                h as i32,
                parent as _,
                null_mut(),
                hinst,
                null(),
            );
            if hwnd.is_null() {
                return Err("CreateWindowExW failed");
            }
            Ok(Self {
                hwnd: hwnd as usize,
                hinst: hinst as usize,
                hooks: Mutex::new(None),
            })
        }
    }

    pub fn show(&self) {
        unsafe { ShowWindow(self.hwnd as _, SW_SHOW) };
    }

    pub fn hide(&self) {
        unsafe { ShowWindow(self.hwnd as _, SW_HIDE) };
    }

    pub fn resize(&self, w: u32, h: u32) {
        unsafe {
            SetWindowPos(
                self.hwnd as _,
                null_mut(),
                0,
                0,
                w as i32,
                h as i32,
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        };
    }

    /// From now on the window puts its clicks, keys, wheel turns and mouse position into `events`
    /// and `mouse` (see `pack_mouse`) as the host's message loop delivers them.
    pub fn attach(&self, events: Arc<Events>, mouse: Arc<AtomicU64>) {
        let hooks = Arc::new(Hooks {
            events,
            mouse,
            tracking: AtomicBool::new(false),
            press: Mutex::new(None),
        });
        // the window's count, given back in `Drop`
        let raw = Arc::into_raw(hooks.clone()) as isize;
        unsafe { SetWindowLongPtrW(self.hwnd as _, GWLP_USERDATA, raw) };
        *self.hooks.lock().unwrap() = Some(hooks);
    }

    // GetAsyncKeyState is the physical left button, so swapped mouse buttons are not
    // handled.
    pub fn pointer(&self) -> Pointer {
        unsafe {
            let mut p = POINT { x: 0, y: 0 };
            GetCursorPos(&mut p);
            let (sx, sy) = (p.x, p.y);
            ScreenToClient(self.hwnd as _, &mut p);
            Pointer {
                x: p.x,
                y: p.y,
                sx,
                sy,
                down: GetAsyncKeyState(VK_LBUTTON as i32) < 0,
            }
        }
    }
}

unsafe impl Target for Native {
    fn surface(&self, entry: &Entry, inst: &Instance) -> vk::SurfaceKHR {
        unsafe {
            khr::win32_surface::Instance::new(entry, inst)
                .create_win32_surface(
                    &vk::Win32SurfaceCreateInfoKHR::default()
                        .hinstance(self.hinst as _)
                        .hwnd(self.hwnd as _),
                    None,
                )
                .unwrap()
        }
    }

    fn set_cursor(&self, cursor: u8) {
        // the class cursor, which Windows uses on the next mouse move
        unsafe {
            let id = match cursor {
                crate::gpu::CURSOR_HAND => IDC_HAND,
                crate::gpu::CURSOR_TEXT => IDC_IBEAM,
                crate::gpu::CURSOR_RESIZE => IDC_SIZENWSE,
                _ => IDC_ARROW,
            };
            let c = LoadCursorW(null_mut(), id);
            SetClassLongPtrW(self.hwnd as _, GCLP_HCURSOR, c as isize);
        }
    }
}

impl Drop for Native {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd as _);
            // the window is gone, so `proc` can no longer see the pointer
            if let Some(h) = self.hooks.lock().unwrap().take() {
                drop(Arc::from_raw(Arc::as_ptr(&h)));
            }
        }
    }
}
