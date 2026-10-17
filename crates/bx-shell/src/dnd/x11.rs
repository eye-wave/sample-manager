//! X11: the source side of XDND (version 5). While the mouse button is held a loop of our own
//! reads the events of the window's display: it tells the window under the mouse about the drag
//! (enter / position / leave), hands over the file name when that window asks for it, and ends
//! with the drop or when the button comes up over nothing that wants it.
#![allow(unsafe_op_in_unsafe_fn)] // all of it is Xlib
use std::ffi::{c_int, c_long, c_uchar};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::time::{Duration, Instant};
use x11::xlib;

const XA_ATOM: xlib::Atom = 4;

/// `file:///a%20b.wav` and a line end: what a drop target takes as a list of files.
fn uri_list(p: &Path) -> Vec<u8> {
    let mut out = b"file://".to_vec();
    for &b in p.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~".contains(&b) {
            out.push(b);
        } else {
            out.extend(format!("%{b:02X}").bytes());
        }
    }
    out.extend(b"\r\n");
    out
}

struct Atoms {
    aware: xlib::Atom,
    enter: xlib::Atom,
    position: xlib::Atom,
    status: xlib::Atom,
    leave: xlib::Atom,
    drop: xlib::Atom,
    finished: xlib::Atom,
    selection: xlib::Atom,
    copy: xlib::Atom,
    uri: xlib::Atom,
    targets: xlib::Atom,
}

unsafe fn atoms(dpy: *mut xlib::Display) -> Atoms {
    let a = |n: &std::ffi::CStr| xlib::XInternAtom(dpy, n.as_ptr(), 0);
    Atoms {
        aware: a(c"XdndAware"),
        enter: a(c"XdndEnter"),
        position: a(c"XdndPosition"),
        status: a(c"XdndStatus"),
        leave: a(c"XdndLeave"),
        drop: a(c"XdndDrop"),
        finished: a(c"XdndFinished"),
        selection: a(c"XdndSelection"),
        copy: a(c"XdndActionCopy"),
        uri: a(c"text/uri-list"),
        targets: a(c"TARGETS"),
    }
}

/// A client message (format 32) from `from` to `to`; `data[0]` is `from`.
unsafe fn send(dpy: *mut xlib::Display, to: xlib::Window, kind: xlib::Atom, data: [c_long; 5]) {
    let mut ev: xlib::XEvent = std::mem::zeroed();
    ev.client_message.type_ = xlib::ClientMessage;
    ev.client_message.send_event = 1;
    ev.client_message.display = dpy;
    ev.client_message.window = to;
    ev.client_message.message_type = kind;
    ev.client_message.format = 32;
    for (i, v) in data.iter().enumerate() {
        ev.client_message.data.set_long(i, *v);
    }
    xlib::XSendEvent(dpy, to, 0, xlib::NoEventMask, &mut ev);
    xlib::XFlush(dpy);
}

/// The XDND version `w` speaks, if it takes drops at all.
unsafe fn aware_version(
    dpy: *mut xlib::Display,
    w: xlib::Window,
    aware: xlib::Atom,
) -> Option<c_long> {
    let (mut ty, mut fmt, mut n, mut after, mut data) =
        (0, 0, 0, 0, std::ptr::null_mut::<c_uchar>());
    let r = xlib::XGetWindowProperty(
        dpy, w, aware, 0, 1, 0, XA_ATOM, &mut ty, &mut fmt, &mut n, &mut after, &mut data,
    );
    let v = (r == 0 && ty == XA_ATOM && fmt == 32 && n > 0 && !data.is_null())
        .then(|| *(data as *const c_long));
    if !data.is_null() {
        xlib::XFree(data as _);
    }
    v
}

/// The window that takes drops under the screen point (`x`, `y`) and its XDND version, `(0, 0)`
/// if there is none: the deepest window on the way down that has the `XdndAware` property.
unsafe fn target_at(
    dpy: *mut xlib::Display,
    aware: xlib::Atom,
    x: c_int,
    y: c_int,
) -> (xlib::Window, c_long) {
    let root = xlib::XDefaultRootWindow(dpy);
    let (mut cur, mut found) = (root, (0, 0));
    loop {
        let (mut dx, mut dy, mut child) = (0, 0, 0);
        xlib::XTranslateCoordinates(dpy, root, cur, x, y, &mut dx, &mut dy, &mut child);
        if cur != root
            && let Some(v) = aware_version(dpy, cur, aware)
        {
            found = (cur, v);
        }
        if child == 0 {
            return found;
        }
        cur = child;
    }
}

/// Answers the target's request for what is dragged: the file list, or what formats there are.
unsafe fn answer(
    dpy: *mut xlib::Display,
    r: &xlib::XSelectionRequestEvent,
    at: &Atoms,
    data: &[u8],
) {
    let prop = if r.property == 0 {
        r.target
    } else {
        r.property
    };
    let ok = if r.target == at.uri {
        xlib::XChangeProperty(
            dpy,
            r.requestor,
            prop,
            at.uri,
            8,
            xlib::PropModeReplace,
            data.as_ptr(),
            data.len() as c_int,
        );
        true
    } else if r.target == at.targets {
        let t = [at.uri, at.targets];
        xlib::XChangeProperty(
            dpy,
            r.requestor,
            prop,
            XA_ATOM,
            32,
            xlib::PropModeReplace,
            t.as_ptr() as *const c_uchar,
            2,
        );
        true
    } else {
        false
    };
    let mut ev: xlib::XEvent = std::mem::zeroed();
    ev.selection.type_ = xlib::SelectionNotify;
    ev.selection.display = dpy;
    ev.selection.requestor = r.requestor;
    ev.selection.selection = r.selection;
    ev.selection.target = r.target;
    ev.selection.property = if ok { prop } else { 0 };
    ev.selection.time = r.time;
    xlib::XSendEvent(dpy, r.requestor, 0, 0, &mut ev);
    xlib::XFlush(dpy);
}

/// What follows the mouse while dragging: a small pill with the file name, an override-redirect
/// window (the window manager leaves it alone) drawn with core Xlib, so no extra dependency.
struct Preview {
    win: xlib::Window,
    gc: xlib::GC,
    font: *mut xlib::XFontStruct,
}

unsafe fn preview_new(dpy: *mut xlib::Display, path: &Path) -> Preview {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    // the core font draws Latin-1 only: anything else shows as ?
    let label: Vec<u8> = name
        .chars()
        .take(40)
        .map(|c| {
            if c.is_ascii() && !c.is_ascii_control() {
                c as u8
            } else {
                b'?'
            }
        })
        .collect();
    let font = xlib::XLoadQueryFont(dpy, c"fixed".as_ptr());
    let tw = if font.is_null() {
        6 * label.len() as c_int
    } else {
        xlib::XTextWidth(font, label.as_ptr() as _, label.len() as c_int)
    };
    let (w, h) = (tw + 24, 24);
    // colours as 24 bit pixels (a TrueColor display, which is every one in use): panel and pink
    let root = xlib::XDefaultRootWindow(dpy);
    let win = xlib::XCreateSimpleWindow(dpy, root, 0, 0, w as u32, h as u32, 1, 0xf4b6c8, 0x443f4a);
    let mut a: xlib::XSetWindowAttributes = std::mem::zeroed();
    a.override_redirect = 1;
    xlib::XChangeWindowAttributes(dpy, win, xlib::CWOverrideRedirect, &mut a);
    let gc = xlib::XCreateGC(dpy, win, 0, std::ptr::null_mut());
    xlib::XSetForeground(dpy, gc, 0xd9d7de);
    if !font.is_null() {
        xlib::XSetFont(dpy, gc, (*font).fid);
    }
    preview_to_pointer(dpy, win);
    xlib::XMapRaised(dpy, win);
    xlib::XDrawString(
        dpy,
        win,
        gc,
        12,
        16,
        label.as_ptr() as _,
        label.len() as c_int,
    );
    xlib::XFlush(dpy);
    Preview { win, gc, font }
}

/// Moves the preview just below and right of the mouse (so that it is never what is dropped on).
unsafe fn preview_to_pointer(dpy: *mut xlib::Display, win: xlib::Window) {
    let root = xlib::XDefaultRootWindow(dpy);
    let (mut r, mut c, mut rx, mut ry, mut wx, mut wy, mut m) = (0, 0, 0, 0, 0, 0, 0);
    xlib::XQueryPointer(
        dpy, root, &mut r, &mut c, &mut rx, &mut ry, &mut wx, &mut wy, &mut m,
    );
    xlib::XMoveWindow(dpy, win, rx + 14, ry + 14);
}

unsafe fn preview_free(dpy: *mut xlib::Display, p: Preview) {
    xlib::XDestroyWindow(dpy, p.win);
    xlib::XFreeGC(dpy, p.gc);
    if !p.font.is_null() {
        xlib::XFreeFont(dpy, p.font);
    }
}

/// Drags `path` out of `win` (on display `dpy`, the thread's own connection) until the button is
/// up. A drop target that never answers is given up on after two seconds.
// no drag cursor feedback, no Escape; add a cursor from the XdndStatus if it matters
pub fn drag(dpy: *mut xlib::Display, win: xlib::Window, path: &Path) {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let data = uri_list(&path);
    unsafe {
        let at = atoms(dpy);
        xlib::XSetSelectionOwner(dpy, at.selection, win, xlib::CurrentTime);
        let preview = preview_new(dpy, &path);
        let mask = (xlib::ButtonReleaseMask | xlib::PointerMotionMask) as u32;
        let grab = xlib::XGrabPointer(
            dpy,
            win,
            0,
            mask,
            xlib::GrabModeAsync,
            xlib::GrabModeAsync,
            0,
            0,
            xlib::CurrentTime,
        );
        if grab != xlib::GrabSuccess as c_int {
            preview_free(dpy, preview);
            return;
        }
        // the window under the mouse that takes drops, whether it said it would take this, and
        // whether we wait for its answer to the last position
        let (mut target, mut accepted, mut waiting) = (0, false, false);
        let (mut asked, mut dropped, mut done) = (Instant::now(), None::<Instant>, false);
        let mut ev: xlib::XEvent = std::mem::zeroed();
        while !done {
            while xlib::XPending(dpy) > 0 {
                xlib::XNextEvent(dpy, &mut ev);
                match ev.get_type() {
                    xlib::MotionNotify if dropped.is_none() => {
                        let (x, y) = (ev.motion.x_root, ev.motion.y_root);
                        xlib::XMoveWindow(dpy, preview.win, x + 14, y + 14);
                        let (t, v) = target_at(dpy, at.aware, x, y);
                        if t != target {
                            if target != 0 {
                                send(dpy, target, at.leave, [win as c_long, 0, 0, 0, 0]);
                            }
                            (target, accepted, waiting) = (t, false, false);
                            if target != 0 {
                                let ver = v.min(5); // the version we speak with it
                                send(
                                    dpy,
                                    target,
                                    at.enter,
                                    [win as c_long, ver << 24, at.uri as c_long, 0, 0],
                                );
                            }
                        }
                        if target != 0 && (!waiting || asked.elapsed() > Duration::from_millis(100))
                        {
                            let pos = (x as c_long) << 16 | (y as c_long & 0xffff);
                            send(
                                dpy,
                                target,
                                at.position,
                                [
                                    win as c_long,
                                    0,
                                    pos,
                                    xlib::CurrentTime as c_long,
                                    at.copy as c_long,
                                ],
                            );
                            (waiting, asked) = (true, Instant::now());
                        }
                    }
                    xlib::ClientMessage => {
                        if ev.client_message.message_type == at.status {
                            accepted = ev.client_message.data.get_long(1) & 1 != 0;
                            waiting = false;
                        } else if ev.client_message.message_type == at.finished {
                            done = true;
                        }
                    }
                    xlib::SelectionRequest => answer(dpy, &ev.selection_request, &at, &data),
                    xlib::ButtonRelease if dropped.is_none() => {
                        xlib::XUnmapWindow(dpy, preview.win); // dropped or not, the drag is over
                        if target != 0 && accepted {
                            send(
                                dpy,
                                target,
                                at.drop,
                                [win as c_long, 0, xlib::CurrentTime as c_long, 0, 0],
                            );
                            dropped = Some(Instant::now()); // now wait for XdndFinished
                            xlib::XUngrabPointer(dpy, xlib::CurrentTime);
                        } else {
                            if target != 0 {
                                send(dpy, target, at.leave, [win as c_long, 0, 0, 0, 0]);
                            }
                            done = true;
                        }
                    }
                    _ => {}
                }
            }
            if dropped.is_some_and(|t| t.elapsed() > Duration::from_secs(2)) {
                done = true;
            }
            if !done {
                std::thread::sleep(Duration::from_millis(4));
            }
        }
        xlib::XUngrabPointer(dpy, xlib::CurrentTime);
        preview_free(dpy, preview);
        xlib::XSetSelectionOwner(dpy, at.selection, 0, xlib::CurrentTime);
        xlib::XFlush(dpy);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_uri_list_escapes_what_a_uri_cannot_hold() {
        let u = uri_list(Path::new("/home/a b/Müll #1.wav"));
        assert_eq!(u, b"file:///home/a%20b/M%C3%BCll%20%231.wav\r\n");
    }
}
