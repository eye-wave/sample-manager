//! Win32: a top-level window.
use super::Grip;
use ash::{Entry, Instance, khr, vk};
use std::ffi::CStr;
use std::ptr::{null, null_mut};
use std::sync::OnceLock;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub const EXTS: [&CStr; 2] = [khr::surface::NAME, khr::win32_surface::NAME];

const STYLE: u32 = WS_OVERLAPPEDWINDOW | WS_VISIBLE;

// ponytail: the window procedure reads this instead of per-window state, so there is one window
// per process; GWLP_USERDATA if that ever matters.
static STATE: OnceLock<(Grip, (u32, u32))> = OnceLock::new();

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
        let Some(&(grip, min)) = STATE.get() else {
            return DefWindowProcW(h, m, w, l);
        };
        match m {
            WM_CLOSE => {
                PostQuitMessage(0); // `run` returns; the window is destroyed by Drop
                0
            }
            WM_GETMINMAXINFO => {
                let (x, y) = outer(min);
                (*(l as *mut MINMAXINFO)).ptMinTrackSize = POINT { x, y };
                0
            }
            // the grip answers "bottom right sizing border": Windows does the drag and the cursor
            WM_NCHITTEST => {
                let mut p = POINT {
                    x: l as i16 as i32,
                    y: (l >> 16) as i16 as i32,
                };
                let mut r: RECT = std::mem::zeroed();
                ScreenToClient(h, &mut p);
                GetClientRect(h, &mut r);
                if grip(p.x, p.y, r.right as u32, r.bottom as u32) {
                    HTBOTTOMRIGHT as LRESULT
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
        STATE.set((grip, min)).ok().expect("one window per process");
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

    pub fn run(&self) {
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                DispatchMessageW(&msg);
            }
        }
    }

    pub unsafe fn surface(&self, entry: &Entry, inst: &Instance) -> vk::SurfaceKHR {
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
