//! Win32: OLE drag and drop, the source side. `DoDragDrop` runs the drag (its own message loop)
//! and asks two COM objects made here: the drop source (when the drag is over) and the data
//! object (a `CF_HDROP` list with one file). The few API functions it needs are declared here
//! instead of taken from windows-sys, whose module layout changes between versions.
#![allow(unsafe_op_in_unsafe_fn, non_snake_case)] // COM
use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering::*};

#[repr(C)]
#[derive(PartialEq, Clone, Copy)]
struct Guid(u32, u16, u16, [u8; 8]);
const IID_UNKNOWN: Guid = Guid(0, 0, 0, [0xC0, 0, 0, 0, 0, 0, 0, 0x46]);
const IID_DATA_OBJECT: Guid = Guid(0x10e, 0, 0, [0xC0, 0, 0, 0, 0, 0, 0, 0x46]);
const IID_DROP_SOURCE: Guid = Guid(0x121, 0, 0, [0xC0, 0, 0, 0, 0, 0, 0, 0x46]);

#[repr(C)]
struct FormatEtc {
    cf: u16,
    ptd: *mut c_void,
    aspect: u32,
    index: i32,
    tymed: u32,
}
#[repr(C)]
struct StgMedium {
    tymed: u32,
    h_global: *mut c_void,
    release: *mut c_void,
}

const CF_HDROP: u16 = 15;
const TYMED_HGLOBAL: u32 = 1;
const DVASPECT_CONTENT: u32 = 1;
const GMEM_MOVEABLE: u32 = 2;
const DROPEFFECT_COPY_LINK: u32 = 1 | 4;
const S_OK: i32 = 0;
const E_NOINTERFACE: i32 = 0x8000_4002_u32 as i32;
const E_NOTIMPL: i32 = 0x8000_4001_u32 as i32;
const E_OUTOFMEMORY: i32 = 0x8007_000E_u32 as i32;
const DV_E_FORMATETC: i32 = 0x8004_0064_u32 as i32;
const OLE_E_ADVISENOTSUPPORTED: i32 = 0x8004_0003_u32 as i32;
const DRAGDROP_S_DROP: i32 = 0x0004_0100;
const DRAGDROP_S_CANCEL: i32 = 0x0004_0101;
const DRAGDROP_S_USEDEFAULTCURSORS: i32 = 0x0004_0102;
const MK_LBUTTON: u32 = 1;

#[link(name = "ole32")]
unsafe extern "system" {
    fn OleInitialize(reserved: *mut c_void) -> i32;
    fn DoDragDrop(data: *mut c_void, source: *mut c_void, effects: u32, effect: *mut u32) -> i32;
}
#[link(name = "shell32")]
unsafe extern "system" {
    fn SHCreateStdEnumFmtEtc(n: u32, formats: *const FormatEtc, out: *mut *mut c_void) -> i32;
}
#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassExW(c: *const WndClass) -> u16;
    #[allow(clippy::too_many_arguments)]
    fn CreateWindowExW(
        ex: u32,
        class: *const u16,
        name: *const u16,
        style: u32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        parent: *mut c_void,
        menu: *mut c_void,
        inst: *mut c_void,
        param: *mut c_void,
    ) -> *mut c_void;
    fn DefWindowProcW(h: *mut c_void, m: u32, w: usize, l: isize) -> isize;
    fn DestroyWindow(h: *mut c_void) -> i32;
    fn ShowWindow(h: *mut c_void, cmd: i32) -> i32;
    fn SetWindowPos(
        h: *mut c_void,
        after: *mut c_void,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: u32,
    ) -> i32;
    fn SetLayeredWindowAttributes(h: *mut c_void, key: u32, alpha: u8, flags: u32) -> i32;
    fn GetCursorPos(p: *mut [i32; 2]) -> i32;
    fn GetDC(h: *mut c_void) -> *mut c_void;
    fn ReleaseDC(h: *mut c_void, dc: *mut c_void) -> i32;
    fn FillRect(dc: *mut c_void, r: *const [i32; 4], brush: *mut c_void) -> i32;
    fn DrawTextW(dc: *mut c_void, s: *const u16, n: i32, r: *mut [i32; 4], fmt: u32) -> i32;
}
#[link(name = "gdi32")]
unsafe extern "system" {
    fn CreateSolidBrush(c: u32) -> *mut c_void;
    fn DeleteObject(o: *mut c_void) -> i32;
    fn SelectObject(dc: *mut c_void, o: *mut c_void) -> *mut c_void;
    fn GetStockObject(i: i32) -> *mut c_void;
    fn SetTextColor(dc: *mut c_void, c: u32) -> u32;
    fn SetBkMode(dc: *mut c_void, mode: i32) -> i32;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetModuleHandleW(name: *const u16) -> *mut c_void;
    fn GlobalAlloc(flags: u32, bytes: usize) -> *mut c_void;
    fn GlobalLock(h: *mut c_void) -> *mut c_void;
    fn GlobalUnlock(h: *mut c_void) -> i32;
}

// ---- what follows the mouse: a pill with the file name (a click-through window on top of
// everything, drawn once with GDI; moved by the drop source, which is called as the mouse moves)

#[repr(C)]
struct WndClass {
    size: u32,
    style: u32,
    proc_: unsafe extern "system" fn(*mut c_void, u32, usize, isize) -> isize,
    cls_extra: i32,
    wnd_extra: i32,
    inst: *mut c_void,
    icon: *mut c_void,
    cursor: *mut c_void,
    background: *mut c_void,
    menu: *const u16,
    class: *const u16,
    icon_sm: *mut c_void,
}

/// The preview window while a drag runs, 0 otherwise (one drag at a time).
static PREVIEW: AtomicUsize = AtomicUsize::new(0);

unsafe extern "system" fn preview_proc(h: *mut c_void, m: u32, w: usize, l: isize) -> isize {
    DefWindowProcW(h, m, w, l)
}

/// COLORREF (0x00BBGGRR) of 0xRRGGBB.
const fn bgr(c: u32) -> u32 {
    (c & 0xff) << 16 | c & 0xff00 | c >> 16
}

unsafe fn preview_new(name: &str) -> usize {
    const DT: u32 = 0x20 | 0x800 | 0x8000; // single line, no & prefix, end ellipsis
    let inst = GetModuleHandleW(std::ptr::null());
    let class: Vec<u16> = "SampleManagerDrag\0".encode_utf16().collect();
    let wc = WndClass {
        size: size_of::<WndClass>() as u32,
        style: 0,
        proc_: preview_proc,
        cls_extra: 0,
        wnd_extra: 0,
        inst,
        icon: null_mut(),
        cursor: null_mut(),
        background: null_mut(),
        menu: std::ptr::null(),
        class: class.as_ptr(),
        icon_sm: null_mut(),
    };
    RegisterClassExW(&wc); // fails harmlessly when an earlier drag registered it
    let text: Vec<u16> = name.encode_utf16().collect();
    // the size of the text in the GUI font
    let screen = GetDC(null_mut());
    let old = SelectObject(screen, GetStockObject(17)); // DEFAULT_GUI_FONT
    let mut r = [0, 0, 260, 0];
    DrawTextW(screen, text.as_ptr(), text.len() as i32, &mut r, DT | 0x400); // DT_CALCRECT
    SelectObject(screen, old);
    ReleaseDC(null_mut(), screen);
    let (w, h) = (r[2] - r[0] + 24, r[3] - r[1] + 12);
    // layered, topmost, click-through, never active: WS_EX_LAYERED | TOPMOST | TOOLWINDOW | NOACTIVATE | TRANSPARENT
    let ex = 0x8_0000 | 0x8 | 0x80 | 0x800_0000 | 0x20;
    let hwnd = CreateWindowExW(
        ex,
        class.as_ptr(),
        std::ptr::null(),
        0x8000_0000,
        0,
        0,
        w,
        h,
        null_mut(),
        null_mut(),
        inst,
        null_mut(),
    );
    if hwnd.is_null() {
        return 0;
    }
    SetLayeredWindowAttributes(hwnd, 0, 235, 2); // LWA_ALPHA
    ShowWindow(hwnd, 4); // SW_SHOWNOACTIVATE
    let dc = GetDC(hwnd);
    let (edge, face) = (
        CreateSolidBrush(bgr(0xf4b6c8)),
        CreateSolidBrush(bgr(0x443f4a)),
    );
    FillRect(dc, &[0, 0, w, h], edge);
    FillRect(dc, &[1, 1, w - 1, h - 1], face);
    SelectObject(dc, GetStockObject(17));
    SetBkMode(dc, 1); // TRANSPARENT
    SetTextColor(dc, bgr(0xd9d7de));
    let mut area = [12, 6, w - 12, h - 6];
    DrawTextW(dc, text.as_ptr(), text.len() as i32, &mut area, DT | 0x4); // DT_VCENTER
    ReleaseDC(hwnd, dc);
    DeleteObject(edge);
    DeleteObject(face);
    hwnd as usize
}

/// Puts the preview just below and right of the mouse.
unsafe fn preview_follow() {
    let h = PREVIEW.load(Relaxed);
    let mut p = [0, 0];
    if h != 0 && GetCursorPos(&mut p) != 0 {
        // HWND_TOPMOST is -1; SWP_NOSIZE | SWP_NOACTIVATE
        SetWindowPos(
            h as _,
            -1isize as *mut c_void,
            p[0] + 16,
            p[1] + 16,
            0,
            0,
            0x1 | 0x10,
        );
    }
}

type Qi<T> = unsafe extern "system" fn(*mut T, *const Guid, *mut *mut c_void) -> i32;
type Ref<T> = unsafe extern "system" fn(*mut T) -> u32;

// ---- the data object: one file as CF_HDROP

#[repr(C)]
struct DataVtbl {
    qi: Qi<Data>,
    add_ref: Ref<Data>,
    release: Ref<Data>,
    get_data: unsafe extern "system" fn(*mut Data, *const FormatEtc, *mut StgMedium) -> i32,
    get_data_here: unsafe extern "system" fn(*mut Data, *const FormatEtc, *mut StgMedium) -> i32,
    query_get_data: unsafe extern "system" fn(*mut Data, *const FormatEtc) -> i32,
    canonical: unsafe extern "system" fn(*mut Data, *const FormatEtc, *mut FormatEtc) -> i32,
    set_data: unsafe extern "system" fn(*mut Data, *const FormatEtc, *mut StgMedium, i32) -> i32,
    enum_format: unsafe extern "system" fn(*mut Data, u32, *mut *mut c_void) -> i32,
    advise:
        unsafe extern "system" fn(*mut Data, *const FormatEtc, u32, *mut c_void, *mut u32) -> i32,
    unadvise: unsafe extern "system" fn(*mut Data, u32) -> i32,
    enum_advise: unsafe extern "system" fn(*mut Data, *mut *mut c_void) -> i32,
}
#[repr(C)]
struct Data {
    vtbl: *const DataVtbl,
    refs: AtomicU32,
    /// The path as UTF-16 with two nulls at the end (the end of the name, the end of the list).
    wide: Vec<u16>,
}

unsafe fn wants_hdrop(f: *const FormatEtc) -> bool {
    (*f).cf == CF_HDROP && (*f).tymed & TYMED_HGLOBAL != 0 && (*f).aspect == DVASPECT_CONTENT
}

unsafe extern "system" fn data_qi(
    this: *mut Data,
    riid: *const Guid,
    out: *mut *mut c_void,
) -> i32 {
    if *riid == IID_UNKNOWN || *riid == IID_DATA_OBJECT {
        *out = this as *mut c_void;
        (*this).refs.fetch_add(1, Relaxed);
        S_OK
    } else {
        *out = null_mut();
        E_NOINTERFACE
    }
}
unsafe extern "system" fn data_add_ref(this: *mut Data) -> u32 {
    (*this).refs.fetch_add(1, Relaxed) + 1
}
unsafe extern "system" fn data_release(this: *mut Data) -> u32 {
    let n = (*this).refs.fetch_sub(1, AcqRel) - 1;
    if n == 0 {
        drop(Box::from_raw(this));
    }
    n
}
/// A DROPFILES header (the list starts 20 bytes in, no point, wide chars) and the path.
unsafe extern "system" fn data_get(this: *mut Data, f: *const FormatEtc, m: *mut StgMedium) -> i32 {
    if !wants_hdrop(f) {
        return DV_E_FORMATETC;
    }
    let w = &(*this).wide;
    let h = GlobalAlloc(GMEM_MOVEABLE, 20 + w.len() * 2);
    if h.is_null() {
        return E_OUTOFMEMORY;
    }
    let p = GlobalLock(h) as *mut u8;
    if p.is_null() {
        return E_OUTOFMEMORY;
    }
    let head: [u32; 5] = [20, 0, 0, 0, 1];
    std::ptr::copy_nonoverlapping(head.as_ptr() as *const u8, p, 20);
    std::ptr::copy_nonoverlapping(w.as_ptr() as *const u8, p.add(20), w.len() * 2);
    GlobalUnlock(h);
    *m = StgMedium {
        tymed: TYMED_HGLOBAL,
        h_global: h,
        release: null_mut(),
    };
    S_OK
}
unsafe extern "system" fn data_get_here(
    _: *mut Data,
    _: *const FormatEtc,
    _: *mut StgMedium,
) -> i32 {
    E_NOTIMPL
}
unsafe extern "system" fn data_query(_: *mut Data, f: *const FormatEtc) -> i32 {
    if wants_hdrop(f) { S_OK } else { DV_E_FORMATETC }
}
unsafe extern "system" fn data_canonical(
    _: *mut Data,
    _: *const FormatEtc,
    _: *mut FormatEtc,
) -> i32 {
    E_NOTIMPL
}
unsafe extern "system" fn data_set(
    _: *mut Data,
    _: *const FormatEtc,
    _: *mut StgMedium,
    _: i32,
) -> i32 {
    E_NOTIMPL
}
unsafe extern "system" fn data_enum(_: *mut Data, dir: u32, out: *mut *mut c_void) -> i32 {
    if dir != 1 {
        return E_NOTIMPL; // DATADIR_GET only
    }
    let f = FormatEtc {
        cf: CF_HDROP,
        ptd: null_mut(),
        aspect: DVASPECT_CONTENT,
        index: -1,
        tymed: TYMED_HGLOBAL,
    };
    SHCreateStdEnumFmtEtc(1, &f, out)
}
unsafe extern "system" fn data_advise(
    _: *mut Data,
    _: *const FormatEtc,
    _: u32,
    _: *mut c_void,
    _: *mut u32,
) -> i32 {
    OLE_E_ADVISENOTSUPPORTED
}
unsafe extern "system" fn data_unadvise(_: *mut Data, _: u32) -> i32 {
    OLE_E_ADVISENOTSUPPORTED
}
unsafe extern "system" fn data_enum_advise(_: *mut Data, _: *mut *mut c_void) -> i32 {
    OLE_E_ADVISENOTSUPPORTED
}

static DATA_VTBL: DataVtbl = DataVtbl {
    qi: data_qi,
    add_ref: data_add_ref,
    release: data_release,
    get_data: data_get,
    get_data_here: data_get_here,
    query_get_data: data_query,
    canonical: data_canonical,
    set_data: data_set,
    enum_format: data_enum,
    advise: data_advise,
    unadvise: data_unadvise,
    enum_advise: data_enum_advise,
};

// ---- the drop source: the drag ends when the left button is up (or Escape)

#[repr(C)]
struct SourceVtbl {
    qi: Qi<Source>,
    add_ref: Ref<Source>,
    release: Ref<Source>,
    query_continue: unsafe extern "system" fn(*mut Source, i32, u32) -> i32,
    give_feedback: unsafe extern "system" fn(*mut Source, u32) -> i32,
}
#[repr(C)]
struct Source {
    vtbl: *const SourceVtbl,
    refs: AtomicU32,
}

unsafe extern "system" fn source_qi(
    this: *mut Source,
    riid: *const Guid,
    out: *mut *mut c_void,
) -> i32 {
    if *riid == IID_UNKNOWN || *riid == IID_DROP_SOURCE {
        *out = this as *mut c_void;
        (*this).refs.fetch_add(1, Relaxed);
        S_OK
    } else {
        *out = null_mut();
        E_NOINTERFACE
    }
}
unsafe extern "system" fn source_add_ref(this: *mut Source) -> u32 {
    (*this).refs.fetch_add(1, Relaxed) + 1
}
unsafe extern "system" fn source_release(this: *mut Source) -> u32 {
    let n = (*this).refs.fetch_sub(1, AcqRel) - 1;
    if n == 0 {
        drop(Box::from_raw(this));
    }
    n
}
unsafe extern "system" fn source_continue(_: *mut Source, escape: i32, keys: u32) -> i32 {
    preview_follow();
    if escape != 0 {
        DRAGDROP_S_CANCEL
    } else if keys & MK_LBUTTON == 0 {
        DRAGDROP_S_DROP
    } else {
        S_OK
    }
}
unsafe extern "system" fn source_feedback(_: *mut Source, _: u32) -> i32 {
    preview_follow();
    DRAGDROP_S_USEDEFAULTCURSORS
}

static SOURCE_VTBL: SourceVtbl = SourceVtbl {
    qi: source_qi,
    add_ref: source_add_ref,
    release: source_release,
    query_continue: source_continue,
    give_feedback: source_feedback,
};

/// Drags `path` until the left button is up. The caller lets go of the mouse capture first.
pub fn drag(path: &Path) {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0, 0]).collect();
    unsafe {
        OleInitialize(null_mut()); // already done (S_FALSE) by most hosts, harmless
        let data = Box::into_raw(Box::new(Data {
            vtbl: &DATA_VTBL,
            refs: AtomicU32::new(1),
            wide,
        }));
        let source = Box::into_raw(Box::new(Source {
            vtbl: &SOURCE_VTBL,
            refs: AtomicU32::new(1),
        }));
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let preview = preview_new(&name);
        PREVIEW.store(preview, Relaxed);
        preview_follow();
        let mut effect = 0;
        DoDragDrop(data as _, source as _, DROPEFFECT_COPY_LINK, &mut effect);
        PREVIEW.store(0, Relaxed);
        if preview != 0 {
            DestroyWindow(preview as _);
        }
        data_release(data);
        source_release(source);
    }
}
