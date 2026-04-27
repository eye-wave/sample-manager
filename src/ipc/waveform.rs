use ahash::AHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::ipc::{IPCBody, IPCMessage, IPCResponse, ok};
use crate::ipc_commands;
use crate::state::AppDirs;

use crate::window::PROTOCOL;

#[cfg(target_os = "windows")]
fn hide_console(cmd: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000);
}

fn decode_audio(path: &str, downsample_factor: usize) -> Option<Vec<u8>> {
    use std::io::Read;
    use std::process::{Command, Stdio};

    let mut cmd = Command::new("ffmpeg");
    hide_console(&mut cmd);

    let mut child = cmd
        .arg("-i")
        .arg(path)
        .args([
            "-f",
            "u8",
            "-ac",
            "1",
            "-hide_banner",
            "-loglevel",
            "error",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let mut raw_bytes = Vec::new();
    child.stdout.take()?.read_to_end(&mut raw_bytes).ok()?;

    if !child.wait().ok()?.success() {
        return None;
    }

    if raw_bytes.is_empty() {
        return None;
    }

    let mut output = Vec::with_capacity(raw_bytes.len() / downsample_factor);

    for i in (0..raw_bytes.len()).step_by(downsample_factor) {
        output.push(raw_bytes[i]);
    }

    Some(output)
}

pub fn draw_waveform(outpath: &Path, samples: &[u8], width: u32) -> bool {
    if samples.is_empty() || width < 1 {
        return false;
    }

    let slice_size = samples.len() / width as usize;
    if slice_size == 0 {
        return false;
    }

    let mut max_value = 0u8;

    for x in 0..width {
        let start_idx = x as usize * slice_size;
        let end_idx = (start_idx + slice_size).min(samples.len());

        let slice = stack_col(&samples[start_idx..end_idx]);

        let mut local_max = 0u8;
        for item in &slice {
            if *item > local_max {
                local_max = *item;
            }
        }

        if local_max > max_value {
            max_value = local_max;
        }
    }

    let norm = ((max_value as f32) * 0.6) / 256.0;
    let inv_norm = if norm != 0.0 { 1.0 / norm } else { 0.0 };

    let bg_color = image::Rgb::from([0, 0, 0]);
    let mut img = image::ImageBuffer::from_fn(width, 256, |_, _| bg_color);

    for x in 0..width {
        let start_idx = x as usize * slice_size;
        let end_idx = (start_idx + slice_size).min(samples.len());

        let slice = stack_col(&samples[start_idx..end_idx]);

        for (y, s) in slice.iter().enumerate() {
            let shade = (*s as f32 * inv_norm) as u8;
            let color = image::Rgb::from([shade, shade, shade]);

            img.put_pixel(x, y as u32, color);
        }
    }

    img.save_with_format(outpath, image::ImageFormat::WebP)
        .is_ok()
}

fn stack_col(samples: &[u8]) -> [u8; 256] {
    let half: i32 = 128;
    let size: usize = 256;

    let mut diff = [0i32; 257];

    for &x in samples {
        let x = x as i32;

        if x > half {
            let len = x - half;
            diff[half as usize] += 1;
            diff[(half + len) as usize] -= 1;
        } else if x < half {
            let len = half - x;
            diff[(half - len + 1) as usize] += 1;
            diff[(half + 1) as usize] -= 1;
        }
    }

    let mut slice = [0u8; 256];
    let mut cur: i32 = 0;

    for i in 0..size {
        cur += diff[i];
        slice[i] = cur.min(255) as u8;
    }

    slice
}

fn hash_path(path: &str) -> String {
    use base64::Engine;

    let mut hasher = AHasher::default();
    path.hash(&mut hasher);

    base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(hasher.finish().to_be_bytes())
}

fn thumbnail_path(hashed: &str, cache_path: &Path) -> PathBuf {
    cache_path.join(hashed)
}

fn thumbnail_uri(hashed: &str) -> String {
    format!("https://{PROTOCOL}._/thumb/{hashed}")
}

fn read_audio_file(body: IPCBody) -> IPCResponse {
    std::thread::spawn(move || {
        let path = body.req.as_ref();

        let hashed = hash_path(path);
        let thumb_path = thumbnail_path(&hashed, &AppDirs::thumbnail_cache_path());
        let uri = thumbnail_uri(&hashed);

        if thumb_path.exists() {
            body.webview_sender
                .send(IPCMessage {
                    id: "read_audio",
                    payload: uri.clone(),
                })
                .ok();
        }

        if let Some(samples) = decode_audio(path, 3)
            && draw_waveform(&thumb_path, &samples, 960)
        {
            body.webview_sender
                .send(IPCMessage {
                    id: "read_audio",
                    payload: uri,
                })
                .ok();
        }
    });

    ok()
}

ipc_commands! {
    IPC_WAVEFORM = [
        read_audio_file
    ]
}
