use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc;

use image::{ImageBuffer, Rgb};

use crate::ipc::IPCMessage;
use crate::plugins::PluginId;
use crate::state::config::FFPathsRef;
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

fn draw_waveform_ffmpeg(
    input: WaveformData,
    ffpaths: FFPathsRef,
    outpath: &str,
) -> std::io::Result<()> {
    let mut cmd = command(ffpaths.ffmpeg);

    match input {
        WaveformData::Path(path) => {
            cmd.arg("-i").arg(path);
        }

        WaveformData::Bytes(format, _) => {
            cmd.arg("-f").arg(format).arg("-i").arg("pipe:0");
        }
    }

    let duration = get_duration(&input, ffpaths.ffprobe).unwrap_or(3.0);
    println!("{duration}");

    const WIDTH: &str = "900";

    if duration < 2.0 {
        return draw_waveform_custom(&input, ffpaths, outpath, duration);
    }

    let filter = format!(
        "color=c=black:s={}x256 [bg]; \
           [0:a] showwavespic=s={}x256:colors=white [fg]; \
           [bg][fg] overlay=format=auto",
        WIDTH, WIDTH
    );

    let mut child = cmd
        .args([
            "-filter_complex",
            &filter,
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

fn get_duration(input: &WaveformData, ffprobe: &str) -> std::io::Result<f32> {
    let mut cmd = command(ffprobe);

    match input {
        WaveformData::Path(path) => {
            cmd.args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                path,
            ]);
        }

        WaveformData::Bytes(format, _) => {
            cmd.args([
                "-v",
                "error",
                "-f",
                format,
                "-show_entries",
                "format=duration",
                "-of",
                "default=noprint_wrappers=1:nokey=1",
                "pipe:0",
            ]);
        }
    }

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;

    if let Some(bytes) = input.bytes()
        && let Some(mut stdin) = child.stdin.take()
    {
        stdin.write_all(bytes)?;
    }

    let output = child.wait_with_output()?;

    let text = String::from_utf8_lossy(&output.stdout);

    Ok(text.trim().parse::<f32>().unwrap_or(0.0))
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
    ffpaths: Option<FFPathsRef>,
) -> std::io::Result<DrawAudioMessage> {
    let hashed = hash_path(plugin_id, path_or_uri);
    let thumb_path = thumbnail_path(&hashed);
    let uri = thumbnail_uri(None, &hashed);

    if thumb_path.exists() {
        return Ok(DrawAudioMessage::Exists { uri });
    }

    if ffpaths.is_none() {
        return Ok(DrawAudioMessage::FFmpegMissing);
    }

    draw_waveform_ffmpeg(input, ffpaths.unwrap(), &thumb_path.to_string_lossy())?;

    Ok(DrawAudioMessage::Result { uri })
}

fn draw_waveform_custom(
    input: &WaveformData,
    ffpaths: FFPathsRef,
    outpath: &str,
    duration: f32,
) -> std::io::Result<()> {
    const WIDTH: u32 = 900;
    const HEIGHT: u32 = 256;
    const MID: u32 = HEIGHT / 2;

    let sample_rate = (WIDTH as f32 / duration).ceil() as u32;

    let mut cmd = command(ffpaths.ffmpeg);

    match input {
        WaveformData::Path(path) => {
            cmd.arg("-i").arg(path);
        }
        WaveformData::Bytes(fmt, _) => {
            cmd.arg("-f").arg(fmt).arg("-i").arg("pipe:0");
        }
    }

    cmd.args([
        "-ac",
        "1",
        "-ar",
        &sample_rate.to_string(),
        "-f",
        "s8",
        "pipe:1",
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::null());

    let mut child = cmd.spawn()?;

    if let Some(bytes) = input.bytes()
        && let Some(mut stdin) = child.stdin.take()
    {
        stdin.write_all(bytes)?;
    }

    let raw = child.wait_with_output()?.stdout;

    let mut img = ImageBuffer::from_pixel(WIDTH, HEIGHT, Rgb([0u8, 0u8, 0u8]));

    let total_samples = raw.len();
    if total_samples == 0 {
        let webp = encode_webp(&img)?;
        std::fs::write(outpath, webp)?;
        return Ok(());
    }

    let spp = total_samples as f64 / WIDTH as f64;

    let sample_to_y = |s: i8| -> u32 {
        let norm = (s as f32 + 128.0) / 255.0;
        let y = (norm * (HEIGHT - 1) as f32).round() as u32;
        y.min(HEIGHT - 1)
    };

    let col_range = |col: u32| -> (i8, i8) {
        let start = (col as f64 * spp) as usize;
        let end = ((col as f64 + 1.0) * spp).ceil() as usize;
        let end = end.min(total_samples);
        if start >= end {
            return (0, 0);
        }
        let mut mn = i8::MAX;
        let mut mx = i8::MIN;
        for &b in &raw[start..end] {
            let s = b as i8;
            if s < mn {
                mn = s;
            }
            if s > mx {
                mx = s;
            }
        }
        (mn, mx)
    };

    let draw_vline = |img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, x: u32, y0: u32, y1: u32| {
        let (top, bot) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };
        for y in top..=bot {
            img.put_pixel(x, y, Rgb([255u8, 255u8, 255u8]));
        }
    };

    let draw_line =
        |img: &mut ImageBuffer<Rgb<u8>, Vec<u8>>, x0: i32, y0: i32, x1: i32, y1: i32| {
            let dx = (x1 - x0).abs();
            let dy = -(y1 - y0).abs();
            let sx: i32 = if x0 < x1 { 1 } else { -1 };
            let sy: i32 = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;
            let (mut x, mut y) = (x0, y0);
            loop {
                if x >= 0 && x < WIDTH as i32 && y >= 0 && y < HEIGHT as i32 {
                    img.put_pixel(x as u32, y as u32, Rgb([255u8, 255u8, 255u8]));
                }
                if x == x1 && y == y1 {
                    break;
                }
                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y += sy;
                }
            }
        };

    const BRIDGE_THRESHOLD: u32 = 4;

    let mut prev_top: Option<u32> = None;
    let mut prev_bot: Option<u32> = None;

    for col in 0..WIDTH {
        let (mn, mx) = col_range(col);
        let y_top = sample_to_y(mx);
        let y_bot = sample_to_y(mn);

        if let (Some(pt), Some(pb)) = (prev_top, prev_bot) {
            let gap_top = if y_top > pb {
                y_top - pb
            } else {
                pt.saturating_sub(y_bot)
            };
            if gap_top > BRIDGE_THRESHOLD {
                let prev_mid = ((pt + pb) / 2) as i32;
                let cur_mid = ((y_top + y_bot) / 2) as i32;
                draw_line(&mut img, col as i32 - 1, prev_mid, col as i32, cur_mid);
            }
        }

        draw_vline(&mut img, col, y_top, y_bot);

        prev_top = Some(y_top);
        prev_bot = Some(y_bot);
    }

    if raw.iter().all(|&b| (b as i8).abs() < 2) {
        for x in 0..WIDTH {
            img.put_pixel(x, MID, Rgb([80, 80, 80]));
        }
    }

    let webp = encode_webp(&img)?;
    std::fs::write(outpath, webp)?;
    Ok(())
}

fn encode_webp(img: &ImageBuffer<Rgb<u8>, Vec<u8>>) -> std::io::Result<Vec<u8>> {
    let mut buf = std::io::Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::WebP)
        .map_err(std::io::Error::other)?;
    Ok(buf.into_inner())
}
