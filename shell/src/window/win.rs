//! Win32: a top-level window.
use super::Grip;
use crate::gpu::{Target, pack_size};
use ash::{Entry, Instance, khr, vk};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// no WS_THICKFRAME / WS_MAXIMIZEBOX: the edges do not resize
const STYLE: u32 = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE;

// ponytail: the window procedure reads this instead of per-window state, so there is one window
// per process; GWLP_USERDATA if that ever matters.
static STATE: OnceLock<(Grip, (u32, u32), (u32, u32))> = OnceLock::new(); // grip, min, aspect

/// The size of the last frame the renderer presented, 0 until the first.
static PRESENTED: AtomicU64 = AtomicU64::new(0);

/// A resize drag: the mouse x (client coordinates) and the client width when it started.
static DRAG: Mutex<Option<(i32, u32)>> = Mutex::new(None);

// Raw handles stored as usize so `Window` is Sync (the render thread only needs the HWND).
pub struct Window {
    hwnd: usize,
    hinst: usize,
}

/// Client size -> size of the whole window.
fn outer((w, h): (u32, u32)) -> (i32, i32) {
    let mut r = RECT {
        left: 0,
        top: 0,
        right: w as _,
        bottom: h as _,
    };
    unsafe { AdjustWindowRect(&mut r, STYLE, 0) };
    (r.right - r.left, r.bottom - r.top)
}

unsafe extern "system" fn proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        let Some(&(grip, min, aspect)) = STATE.get() else {
            return DefWindowProcW(h, m, w, l);
        };
        match m {
            WM_CLOSE => {
                PostQuitMessage(0); // `run` returns; the window is destroyed by Drop
                0
            }
            // flicker: before the user sees the new size, wait (briefly) for a frame of that size,
            // else Windows shows the old one stretched or with black borders
            WM_SIZE => {
                let want = pack_size((l & 0xffff) as u32, ((l >> 16) & 0xffff) as u32);
                if w as u32 != SIZE_MINIMIZED && PRESENTED.load(Relaxed) != 0 {
                    let t = Instant::now();
                    while PRESENTED.load(Relaxed) != want && t.elapsed() < Duration::from_millis(50)
                    {
                        std::thread::sleep(Duration::from_millis(1));
                    }
                }
                0
            }
            WM_ERASEBKGND => 1, // the renderer paints everything
            // dragging the handle: the width follows the mouse, the height keeps the aspect ratio
            WM_LBUTTONDOWN => {
                let (x, y) = (l as i16 as i32, (l >> 16) as i16 as i32);
                let mut r: RECT = std::mem::zeroed();
                GetClientRect(h, &mut r);
                if grip(x, y, r.right as u32, r.bottom as u32) {
                    *DRAG.lock().unwrap() = Some((x, r.right as u32));
                    SetCapture(h);
                }
                0
            }
            WM_MOUSEMOVE => {
                let drag = *DRAG.lock().unwrap(); // copied: SetWindowPos re-enters this procedure
                if let Some((x0, w0)) = drag {
                    let x = l as i16 as i32;
                    let nw = (w0 as i32 + x - x0).max(min.0 as i32) as u32;
                    let (ow, oh) = outer((nw, nw * aspect.1 / aspect.0));
                    SetWindowPos(
                        h,
                        null_mut(),
                        0,
                        0,
                        ow,
                        oh,
                        SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
                0
            }
            WM_LBUTTONUP | WM_CAPTURECHANGED => {
                *DRAG.lock().unwrap() = None;
                if m == WM_LBUTTONUP {
                    ReleaseCapture();
                }
                0
            }
            WM_SETCURSOR => {
                let mut p: POINT = std::mem::zeroed();
                let mut r: RECT = std::mem::zeroed();
                GetCursorPos(&mut p);
                ScreenToClient(h, &mut p);
                GetClientRect(h, &mut r);
                let on = DRAG.lock().unwrap().is_some()
                    || (l as u32 & 0xffff == HTCLIENT
                        && grip(p.x, p.y, r.right as u32, r.bottom as u32));
                if on {
                    SetCursor(LoadCursorW(null_mut(), IDC_SIZENWSE));
                    1
                } else {
                    DefWindowProcW(h, m, w, l)
                }
            }
            _ => DefWindowProcW(h, m, w, l),
        }
    }
}

impl Window {
    pub fn new(title: &str, size: (u32, u32), min: (u32, u32), grip: Grip) -> Self {
        STATE
            .set((grip, min, size))
            .ok()
            .expect("one window per process");
        unsafe {
            let hinst = GetModuleHandleW(null());
            let class: Vec<u16> = "SampleManager\0".encode_utf16().collect();
            let mut wc: WNDCLASSEXW = std::mem::zeroed();
            wc.cbSize = size_of::<WNDCLASSEXW>() as u32;
            wc.lpfnWndProc = Some(proc);
            wc.hInstance = hinst;
            wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
            wc.lpszClassName = class.as_ptr();
            RegisterClassExW(&wc);
            let title: Vec<u16> = title.encode_utf16().chain([0]).collect();
            let (w, h) = outer(size);
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                title.as_ptr(),
                STYLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                w,
                h,
                null_mut(),
                null_mut(),
                hinst,
                null(),
            );
            assert!(!hwnd.is_null(), "CreateWindowExW failed");
            Self {
                hwnd: hwnd as usize,
                hinst: hinst as usize,
            }
        }
    }

    /// For the renderer (`render`'s `presented`): `WM_SIZE` waits for it, see `proc`.
    pub fn presented(&self) -> &AtomicU64 {
        &PRESENTED
    }

    pub fn run(&self) {
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                DispatchMessageW(&msg);
            }
        }
    }
}

unsafe impl Target for Window {
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
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe { DestroyWindow(self.hwnd as _) };
    }
}
