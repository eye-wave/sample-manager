use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc;

use crate::AStr;
use crate::ipc::IPCMessage;
use crate::plugins::PluginId;
use crate::state::samples::utils::{hash_path, thumbnail_path, thumbnail_uri};

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

pub enum WaveformData<'a> {
    Path(&'a str),
    Bytes(&'a str, &'a [u8]),
}

impl<'p> WaveformData<'p> {
    fn bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(_, b) => Some(b),
            Self::Path(_) => None,
        }
    }
}

fn draw_waveform_ffmpeg(input: WaveformData, ffpath: &str, outpath: &str) -> std::io::Result<()> {
    let mut cmd = command(ffpath);

    match input {
        WaveformData::Path(path) => {
            cmd.arg("-i").arg(path);
        }

        WaveformData::Bytes(format, _) => {
            cmd.arg("-f").arg(format).arg("-i").arg("pipe:0");
        }
    }

    let mut child = cmd
        .args([
            "-filter_complex",
            "color=c=black:s=900x256 [bg]; \
             [0:a] showwavespic=s=900x256:colors=white [fg]; \
             [bg][fg] overlay=format=auto",
            "-frames:v",
            "1",
            "-c:v",
            "libwebp",
            "-q:v",
            "50",
            "-f",
            "webp",
            "pipe:1",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    if let Some(bytes) = input.bytes()
        && let Some(mut stdin) = child.stdin.take()
    {
        stdin.write_all(bytes)?;
    }

    let output = child.wait_with_output()?.stdout;

    std::fs::write(outpath, output)?;

    Ok(())
}

pub enum DrawAudioMessage {
    Exists { uri: String },
    FFmpegMissing,
    Result { uri: String },
}

impl DrawAudioMessage {
    pub fn send_to_webview(
        &self,
        sender: mpsc::Sender<IPCMessage>,
    ) -> Result<(), mpsc::SendError<IPCMessage>> {
        match self {
            DrawAudioMessage::Exists { uri } => sender.send(IPCMessage {
                id: "read_audio",
                payload: uri.clone(),
            }),
            DrawAudioMessage::FFmpegMissing => sender.send(IPCMessage {
                id: "read_audio",
                payload: "ff-missing".to_string(),
            }),
            DrawAudioMessage::Result { uri } => sender.send(IPCMessage {
                id: "read_audio",
                payload: uri.clone(),
            }),
        }
    }
}

pub fn draw_audio_and_save(
    plugin_id: Option<&PluginId>,
    path_or_uri: &str,
    input: WaveformData,
    ffmpeg_path: Option<AStr>,
) -> std::io::Result<DrawAudioMessage> {
    let hashed = hash_path(plugin_id, path_or_uri);
    let thumb_path = thumbnail_path(&hashed);
    let uri = thumbnail_uri(None, &hashed);

    if thumb_path.exists() {
        return Ok(DrawAudioMessage::Exists { uri });
    }

    if ffmpeg_path.is_none() {
        return Ok(DrawAudioMessage::FFmpegMissing);
    }

    let ffmpeg_path = ffmpeg_path.unwrap().clone();
    draw_waveform_ffmpeg(input, &ffmpeg_path, &thumb_path.to_string_lossy())?;

    Ok(DrawAudioMessage::Result { uri })
}
