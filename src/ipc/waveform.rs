use ahash::AHasher;
use std::hash::{Hash, Hasher};

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use crate::ipc::{IPCBody, IPCMessage, IPCResponse, ok};
use crate::state::app_paths;
use crate::{AStr, ipc_commands};

use crate::window::PROTOCOL;

#[cfg(target_os = "windows")]
fn command(cmd: &str) -> Command {
    let mut cmd = Command::new(cmd);
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000);
    cmd
}

#[cfg(not(target_os = "windows"))]
fn command(cmd: &str) -> Command {
    Command::new(cmd)
}

fn draw_waveform_ffmpeg(path: &str, ffpath: &str, outpath: &str) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::Read;
    use std::io::Write;
    use std::process::Stdio;

    let mut child = command(ffpath)
        .arg("-i")
        .arg(path)
        .args([
            "-filter_complex",
            "color=c=black:s=900x256 [bg]; [0:a] showwavespic=s=900x256:colors=white [fg]; [bg][fg] overlay=format=auto",
            "-frames:v", "1",
            "-c:v", "libwebp",
            "-q:v", "50",
            "-f", "webp",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    let mut output = Vec::new();
    child.stdout.take().unwrap().read_to_end(&mut output)?;

    let status = child.wait()?;
    if !status.success() {
        return Err(std::io::Error::other("FFmpeg command failed"));
    }

    let mut file = File::create(outpath)?;
    file.write_all(&output)?;

    Ok(())
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

#[cfg(target_os = "windows")]
fn thumbnail_uri(hashed: &str) -> String {
    format!("https://{PROTOCOL}._/thumb/{hashed}")
}

#[cfg(not(target_os = "windows"))]
fn thumbnail_uri(hashed: &str) -> String {
    format!("{PROTOCOL}://_/thumb/{hashed}")
}

fn draw_audio_file(body: IPCBody) -> IPCResponse {
    let ffmpeg_path = crate::with_state!(body, state, { state.get_config().ffmpeg_path.clone() });

    if ffmpeg_path.as_ref().map(|p| !p.exists()).unwrap_or(false) {
        crate::with_state_mut!(body, state, {
            state.update_config(|c| c.ffmpeg_path = None)
        })
    }

    let ffmpeg_path = ffmpeg_path.clone();

    std::thread::spawn(move || {
        let path = body.req.clone();

        let hashed = hash_path(&path);
        let thumb_path = thumbnail_path(&hashed, &app_paths::thumbnail_cache_path());
        let uri = thumbnail_uri(&hashed);

        if thumb_path.exists() {
            body.webview_sender
                .send(IPCMessage {
                    id: "read_audio",
                    payload: uri.clone(),
                })
                .ok();

            return;
        }

        let ffmpeg_path: Option<AStr> =
            ffmpeg_path.as_ref().and_then(|p| p.to_str()).map(Arc::from);

        if ffmpeg_path.is_none() {
            body.webview_sender
                .send(IPCMessage {
                    id: "read_audio",
                    payload: "ff-missing".to_string(),
                })
                .ok();

            return;
        }

        let ffmpeg_path = ffmpeg_path.unwrap().clone();

        match draw_waveform_ffmpeg(&path, &ffmpeg_path, &thumb_path.to_string_lossy()) {
            Ok(_) => {
                body.webview_sender
                    .send(IPCMessage {
                        id: "read_audio",
                        payload: uri,
                    })
                    .ok();
            }
            Err(err) => eprintln!("{err}"),
        }
    });

    ok()
}

ipc_commands! {
    IPC_WAVEFORM = [
        draw_audio_file
    ]
}
