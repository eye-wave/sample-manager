use crate::commands::IPCBody;
use crate::event::IPCMessage;
use crate::ipc_commands;

fn decode_audio(path: &str, downsample_factor: usize) -> Option<Vec<f32>> {
    use std::io::Read;
    use std::process::{Command, Stdio};

    let mut child = Command::new("ffmpeg")
        .arg("-i")
        .arg(path)
        .args([
            "-f",
            "f32le",
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

    let mut stdout = child.stdout.take()?;
    let mut raw_bytes = Vec::new();

    stdout.read_to_end(&mut raw_bytes).ok()?;

    let status = child.wait().ok()?;
    if !status.success() {
        return None;
    }

    if raw_bytes.len() < std::mem::size_of::<f32>() {
        return None;
    }

    let samples: &[f32] = bytemuck::cast_slice(&raw_bytes);

    if samples.is_empty() {
        return None;
    }

    let peak = samples
        .iter()
        .map(|s| s.abs())
        .fold(0.0_f32, f32::max)
        .max(1e-8);

    let mut output = Vec::with_capacity(samples.len() / downsample_factor);

    for i in (0..samples.len()).step_by(downsample_factor) {
        output.push(samples[i] / peak);
    }

    Some(output)
}

pub fn encode_sample(x: f32) -> u8 {
    let x = x.clamp(-1.0, 1.0);

    let i = (((x + 1.0) * 0.5) * 92.0).round() as u8;

    let mut v = i + 32;
    if v >= 92 {
        v += 1;
    }

    v
}

fn read_audio_file(body: IPCBody) -> Option<std::borrow::Cow<'static, [u8]>> {
    std::thread::spawn(move || {
        if let Some(samples) = decode_audio(body.req.as_ref(), 4) {
            let data = samples
                .iter()
                .map(|&s| encode_sample(s))
                .collect::<Vec<_>>();

            let payload = unsafe { str::from_utf8_unchecked(&data) }.to_string();

            body.webview_sender
                .send(IPCMessage {
                    id: "read_audio",
                    payload,
                })
                .ok();
        }
    });

    None
}

ipc_commands! {
    IPC_WAVEFORM = [
        read_audio_file
    ]
}
