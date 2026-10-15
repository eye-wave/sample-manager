//! Win32: a child window.
use super::*;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{HMODULE, POINT};
use windows_sys::Win32::Graphics::Gdi::ScreenToClient;
use windows_sys::Win32::System::LibraryLoader::*;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub const API: GuiApiType<'static> = GuiApiType::WIN32;
pub const EXTS: [&CStr; 2] = [khr::surface::NAME, khr::win32_surface::NAME];

// Raw handles stored as usize so `Native` is Send + Sync (the render thread only needs the HWND).
pub struct Native {
    hwnd: usize,
    hinst: usize,
}

impl Native {
    pub fn new(parent: &Window, w: u32, h: u32) -> Result<Self, PluginError> {
        let parent = parent
            .as_win32_hwnd()
            .ok_or(PluginError::Message("not a Win32 parent"))?;
        unsafe {
            // Our own DLL's handle (not the host exe's), found from the address of this function.
            let mut hinst: HMODULE = null_mut();
            GetModuleHandleExW(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS
                    | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                Self::new as usize as *const u16,
                &mut hinst,
            );

            let class: Vec<u16> = "SampleManagerGui\0".encode_utf16().collect();
            let mut wc: WNDCLASSEXW = std::mem::zeroed();
            wc.cbSize = std::mem::size_of::<WNDCLASSEXW>() as u32;
            wc.lpfnWndProc = Some(DefWindowProcW);
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
                return Err(PluginError::Message("CreateWindowExW failed"));
            }
            Ok(Self {
                hwnd: hwnd as usize,
                hinst: hinst as usize,
            })
        }
    }

    pub fn show(&self) -> Result<(), PluginError> {
        unsafe { ShowWindow(self.hwnd as _, SW_SHOW) };
        Ok(())
    }

    pub fn hide(&self) -> Result<(), PluginError> {
        unsafe { ShowWindow(self.hwnd as _, SW_HIDE) };
        Ok(())
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

impl Drop for Native {
    fn drop(&mut self) {
        unsafe { DestroyWindow(self.hwnd as _) };
    }
}
