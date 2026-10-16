//! Standalone app: a window (see shell::window) with the renderer on its own thread (same as the
//! plugin).
use bx_shell::window::Window;
use sample_manager::icons::in_grip;
use sample_manager::render::{H, MIN_H, MIN_W, W, pack_size, run};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};

fn main() {
    let win = Window::new("sample-manager", (W, H), (MIN_W, MIN_H), in_grip);
    // only read where Vulkan does not report the window size itself (Wayland); X11 / Win32 do
    let size = AtomicU64::new(pack_size(W, H));
    let stop = AtomicBool::new(false);
    std::thread::scope(|s| {
        s.spawn(|| {
            run(
                &win,
                &size,
                &stop,
                win.presented(),
                win.mouse(),
                win.events(),
            )
        });
        win.run();
        stop.store(true, Relaxed); // the scope joins the render thread
    });
}
