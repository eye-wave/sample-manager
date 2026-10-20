//! `SHBrowseForFolderW` (the shell's folder tree dialog) and `GetOpenFileNameW` (the file dialog).
//! Declared here like the OLE calls in `dnd/win.rs`, no crates.
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

const OFN_NOCHANGEDIR: u32 = 0x8;
const OFN_PATHMUSTEXIST: u32 = 0x800;
const OFN_FILEMUSTEXIST: u32 = 0x1000;

/// OPENFILENAMEW
#[repr(C)]
struct OpenFileName {
    size: u32,
    owner: *mut c_void,
    instance: *mut c_void,
    filter: *const u16,
    custom_filter: *mut u16,
    max_custom_filter: u32,
    filter_index: u32,
    file: *mut u16,
    max_file: u32,
    file_title: *mut u16,
    max_file_title: u32,
    initial_dir: *const u16,
    title: *const u16,
    flags: u32,
    file_offset: u16,
    file_extension: u16,
    def_ext: *const u16,
    cust_data: isize,
    hook: *mut c_void,
    template_name: *const u16,
    reserved: *mut c_void,
    reserved_dw: u32,
    flags_ex: u32,
}

#[link(name = "comdlg32")]
unsafe extern "system" {
    fn GetOpenFileNameW(info: *mut OpenFileName) -> i32;
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

/// Asks for one `.wasm` file; `None` if cancelled (`stop` is ignored: the dialog is modal).
pub fn pick_wasm(_stop: &AtomicBool) -> Option<PathBuf> {
    // pairs of "name\0pattern\0", the list ends with an extra 0
    let filter: Vec<u16> = "WebAssembly (*.wasm)\0*.wasm\0\0".encode_utf16().collect();
    let title: Vec<u16> = "Add plugin\0".encode_utf16().collect();
    let mut path = [0u16; 32768]; // long paths: MAX_PATH is not enough for every disk
    unsafe {
        let mut info: OpenFileName = std::mem::zeroed();
        info.size = std::mem::size_of::<OpenFileName>() as u32;
        info.filter = filter.as_ptr();
        info.file = path.as_mut_ptr();
        info.max_file = path.len() as u32;
        info.title = title.as_ptr();
        info.flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
        if GetOpenFileNameW(&mut info) == 0 {
            return None;
        }
    }
    let len = path.iter().position(|&c| c == 0).unwrap_or(path.len());
    (len > 0).then(|| PathBuf::from(std::ffi::OsString::from_wide(&path[..len])))
}
