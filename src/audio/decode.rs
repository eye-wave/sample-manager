use std::path::Path;
use std::sync::mpsc::{self, Sender};
use std::{path::PathBuf, sync::Arc, time::Duration};

use cpal::SupportedStreamConfig;
use ringbuf::traits::{Observer, Producer};
use symphonia::core::formats::{SeekMode, SeekTo};
use symphonia::core::{
    audio::{AudioBuffer, AudioBufferRef, Signal},
    codecs::{CODEC_TYPE_NULL, DecoderOptions},
    errors::Error as SymphoniaError,
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
    units::Time,
};

use super::device::RingProd;
use super::handle::{PlayerFlags, SharedAudioState};

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("")]
    NoSampleRate,

    #[error("")]
    NoTrack,

    #[error("")]
    InvalidTimeSeek,

    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Symph(#[from] SymphoniaError),
}

pub enum DecodeCommand {
    Start { path: PathBuf },
    Pause,
    Resume,
    Seek { millis: u32 },
    Stop,
}

pub struct AudioDecoderHandle {
    tx: Sender<DecodeCommand>,
}

impl AudioDecoderHandle {
    pub fn new(
        rb_prod: RingProd,
        audio_state: Arc<SharedAudioState>,
        stream_config: SupportedStreamConfig,
    ) -> Result<Self, DecodeError> {
        let (tx, rx) = mpsc::channel::<DecodeCommand>();

        std::thread::Builder::new()
            .name("decode-thread".into())
            .spawn(move || {
                decode_thread_loop(rx, rb_prod, audio_state, stream_config).ok();
            })?;

        Ok(Self { tx })
    }

    pub fn start(&self, path: &impl AsRef<Path>) {
        let _ = self.tx.send(DecodeCommand::Start {
            path: path.as_ref().into(),
        });
    }

    pub fn pause(&self) {
        let _ = self.tx.send(DecodeCommand::Pause);
    }

    pub fn resume(&self) {
        let _ = self.tx.send(DecodeCommand::Resume);
    }

    pub fn seek(&self, millis: u32) {
        let _ = self.tx.send(DecodeCommand::Seek { millis });
    }

    pub fn stop(&self) {
        let _ = self.tx.send(DecodeCommand::Stop);
    }
}
fn decode_thread_loop(
    rx: std::sync::mpsc::Receiver<DecodeCommand>,
    mut rb_prod: RingProd,
    audio_state: Arc<SharedAudioState>,
    stream_config: SupportedStreamConfig,
) -> Result<(), DecodeError> {
    let mut current = None;

    loop {
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                DecodeCommand::Start { path } => {
                    current = Some(init_decoder(&path, &stream_config)?);
                    audio_state.set_state(PlayerFlags::PLAYING);
                }

                DecodeCommand::Pause => {
                    audio_state.set_state(PlayerFlags::PAUSED);
                }

                DecodeCommand::Resume => {
                    audio_state.set_state(PlayerFlags::PLAYING);
                }

                DecodeCommand::Seek { millis } => {
                    if let Some(dec) = &mut current {
                        perform_seek(dec, millis).ok();
                    }
                }

                DecodeCommand::Stop => {
                    current = None;
                    audio_state.set_state(PlayerFlags::STOPPED);
                }
            }
        }

        let Some(decoder_state) = &mut current else {
            std::thread::sleep(Duration::from_millis(10));
            continue;
        };

        if audio_state.is_paused() {
            std::thread::sleep(Duration::from_millis(10));
            continue;
        }

        if rb_prod.vacant_len() < 4096 {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }

        if let Err(e) = decode_one_packet(decoder_state, &mut rb_prod, &audio_state) {
            eprintln!("decode error: {e}");
            current = None;
        }
    }
}

struct DecoderState {
    format: Box<dyn symphonia::core::formats::FormatReader>,
    decoder: Box<dyn symphonia::core::codecs::Decoder>,
    track_id: u32,
    sample_rate: u32,
    resample_ratio: u32,
    channels: usize,
    resample_buf: Vec<f32>,
}

fn decode_one_packet(
    state: &mut DecoderState,
    rb_prod: &mut RingProd,
    shared_state: &Arc<SharedAudioState>,
) -> Result<(), Box<dyn std::error::Error>> {
    use symphonia::core::errors::Error as SymphoniaError;

    let packet = match state.format.next_packet() {
        Ok(p) => p,
        Err(SymphoniaError::IoError(_)) | Err(SymphoniaError::ResetRequired) => {
            return Err("end of stream".into());
        }
        Err(e) => {
            return Err(format!("format error: {e}").into());
        }
    };

    if packet.track_id() != state.track_id {
        return Ok(());
    }

    let pos = packet.ts() as u32 / (state.sample_rate / 1000);
    shared_state
        .position_millis
        .store(pos, std::sync::atomic::Ordering::Release);

    let decoded = match state.decoder.decode(&packet) {
        Ok(d) => d,
        Err(SymphoniaError::DecodeError(_)) => {
            return Ok(());
        }
        Err(e) => {
            return Err(format!("decoder fatal: {e}").into());
        }
    };

    let interleaved = audio_buffer_to_f32(&decoded);

    let src_channels = decoded.spec().channels.count();
    let src_frames = interleaved.len() / src_channels;
    let dst_frames = (src_frames as u32 * state.resample_ratio) as usize;

    state.resample_buf.clear();

    for dst_i in 0..dst_frames {
        let src_pos = dst_i as u32 / state.resample_ratio;
        let src_frame = src_pos as usize;
        let frac = src_pos - src_frame as u32;
        let nxt_frame = (src_frame + 1).min(src_frames - 1);

        for ch in 0..state.channels {
            let sc = ch.min(src_channels - 1);

            let a = interleaved[src_frame * src_channels + sc];
            let b = interleaved[nxt_frame * src_channels + sc];

            state.resample_buf.push(a + frac as f32 * (b - a));
        }
    }

    rb_prod.push_slice(&state.resample_buf);

    Ok(())
}

fn init_decoder(
    path: &impl AsRef<Path>,
    stream_config: &SupportedStreamConfig,
) -> Result<DecoderState, DecodeError> {
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.as_ref().extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or(DecodeError::NoTrack)?
        .clone();

    let track_id = track.id;

    let decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default())?;

    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or(DecodeError::NoSampleRate)?;

    let output_rate = stream_config.config().sample_rate;
    let resample_ratio = output_rate / sample_rate;

    let channels = stream_config.channels() as usize;

    Ok(DecoderState {
        format,
        decoder,
        track_id,
        sample_rate,
        resample_ratio,
        channels,
        resample_buf: Vec::new(),
    })
}

fn perform_seek(state: &mut DecoderState, millis: u32) -> Result<(), Box<dyn std::error::Error>> {
    let secs = (millis / 1000) as u8;
    let ns = (millis % 1000) * 1_000_000;

    let time = Time::from_ss(secs, ns).ok_or(DecodeError::InvalidTimeSeek)?;

    let seek_to = SeekTo::Time {
        time,
        track_id: Some(state.track_id),
    };

    state.format.seek(SeekMode::Accurate, seek_to)?;
    state.decoder.reset();
    state.resample_buf.clear();

    Ok(())
}

fn audio_buffer_to_f32(buf: &AudioBufferRef<'_>) -> Vec<f32> {
    let spec = *buf.spec();
    let frames = buf.frames();

    let mut converted = AudioBuffer::<f32>::new(frames as u64, spec);

    buf.convert(&mut converted);

    let channels = converted.spec().channels.count();
    let mut out = Vec::with_capacity(frames * channels);

    for f in 0..frames {
        for c in 0..channels {
            out.push(converted.chan(c)[f]);
        }
    }

    out
}
