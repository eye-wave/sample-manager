//! Standalone app: a window (raw X11 / Win32, see window.rs), with the renderer on its own thread
//! (same as the plugin).
use sample_manager::icons::in_grip;
use sample_manager::render::{H, MIN_H, MIN_W, W, pack_size, render};
use sample_manager::window::{EXTS, Window};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};

fn main() {
    let win = Window::new("sample-manager", (W, H), (MIN_W, MIN_H), in_grip);
    // only read where Vulkan does not report the window size itself (Wayland); X11 / Win32 do
    let size = AtomicU64::new(pack_size(W, H));
    let stop = AtomicBool::new(false);
    std::thread::scope(|s| {
        s.spawn(|| unsafe {
            render(
                &EXTS.map(|e| e.as_ptr()),
                |e, i| win.surface(e, i),
                &size,
                &stop,
            )
        });
        win.run();
        stop.store(true, Relaxed); // the scope joins the render thread
    });
}
