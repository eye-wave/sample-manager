//! `SHBrowseForFolderW`: the shell's folder tree dialog. Declared here like the OLE calls in
//! `dnd/win.rs`, no crates.
#![allow(unsafe_op_in_unsafe_fn)]
use std::ffi::c_void;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::sync::atomic::AtomicBool;

const BIF_RETURNONLYFSDIRS: u32 = 0x1;
const BIF_NEWDIALOGSTYLE: u32 = 0x40; // resizable, with a "make new folder" button; needs OLE
const MAX_PATH: usize = 260;

#[repr(C)]
struct BrowseInfo {
    owner: *mut c_void,
    root: *const c_void,
    display_name: *mut u16,
    title: *const u16,
    flags: u32,
    callback: *mut c_void,
    lparam: isize,
    image: i32,
}

#[link(name = "shell32")]
unsafe extern "system" {
    fn SHBrowseForFolderW(info: *mut BrowseInfo) -> *mut c_void;
    fn SHGetPathFromIDListW(pidl: *mut c_void, path: *mut u16) -> i32;
}
#[link(name = "ole32")]
unsafe extern "system" {
    fn OleInitialize(reserved: *mut c_void) -> i32;
    fn CoTaskMemFree(p: *mut c_void);
}

/// Asks for a folder; `None` if cancelled (`stop` is ignored: the dialog is modal).
pub fn pick_folder(_stop: &AtomicBool) -> Option<PathBuf> {
    let title: Vec<u16> = "Add folder\0".encode_utf16().collect();
    let (mut name, mut path) = ([0u16; MAX_PATH], [0u16; MAX_PATH]);
    unsafe {
        OleInitialize(null_mut()); // this thread's apartment; S_FALSE if already done, harmless
        let mut info = BrowseInfo {
            owner: null_mut(),
            root: null(),
            display_name: name.as_mut_ptr(),
            title: title.as_ptr(),
            flags: BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE,
            callback: null_mut(),
            lparam: 0,
            image: 0,
        };
        let pidl = SHBrowseForFolderW(&mut info);
        if pidl.is_null() {
            return None;
        }
        let ok = SHGetPathFromIDListW(pidl, path.as_mut_ptr()) != 0;
        CoTaskMemFree(pidl);
        let len = path.iter().position(|&c| c == 0).unwrap_or(MAX_PATH);
        (ok && len > 0).then(|| PathBuf::from(std::ffi::OsString::from_wide(&path[..len])))
    }
}
