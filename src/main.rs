//! Standalone app: a window (see shell::window) with the renderer on its own thread (same as the
//! plugin).
use bx_shell::window::Window;
use sample_manager::audio::{Player, Voice};
use sample_manager::icons::in_grip;
use sample_manager::render::{H, MIN_H, MIN_W, W, pack_size, run};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};

/// Plays what `player` is given on the default output device, until the stream is dropped. `None`
/// (silence, the GUI still works) without a device or without f32 output.
// f32 only, no device choice; add a conversion / picker if a machine needs one
fn audio(player: Arc<Player>) -> Option<cpal::Stream> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let dev = cpal::default_host().default_output_device()?;
    let cfg = dev.default_output_config().ok()?;
    if cfg.sample_format() != cpal::SampleFormat::F32 {
        eprintln!("audio: the output is not f32, playing nothing");
        return None;
    }
    let ch = cfg.channels() as usize;
    let mut voice = Voice::new(player, cfg.sample_rate().0 as f64);
    let (mut l, mut r) = ([0.0f32; 1024], [0.0f32; 1024]);
    let stream = dev
        .build_output_stream(
            &cfg.config(),
            move |out: &mut [f32], _| {
                for chunk in out.chunks_mut(1024 * ch) {
                    let n = chunk.len() / ch;
                    voice.render(&mut l[..n], &mut r[..n]);
                    for (i, f) in chunk.chunks_exact_mut(ch).enumerate() {
                        f.fill(0.0);
                        if ch == 1 {
                            f[0] = (l[i] + r[i]) / 2.0;
                        } else {
                            (f[0], f[1]) = (l[i], r[i]);
                        }
                    }
                }
            },
            |e| eprintln!("audio: {e}"),
            None,
        )
        .ok()?;
    stream.play().ok()?;
    Some(stream)
}

fn main() {
    let win = Window::new("sample-manager", (W, H), (MIN_W, MIN_H), in_grip);
    // only read where Vulkan does not report the window size itself (Wayland); X11 / Win32 do
    let size = AtomicU64::new(pack_size(W, H));
    let stop = AtomicBool::new(false);
    let player = Arc::<Player>::default();
    let _audio = audio(player.clone()); // dropped (and silent) when the window closes
    std::thread::scope(|s| {
        s.spawn(|| {
            run(
                &win,
                &size,
                &stop,
                win.presented(),
                win.mouse(),
                win.events(),
                &player,
            )
        });
        win.run();
        stop.store(true, Relaxed); // the scope joins the render thread
    });
}
