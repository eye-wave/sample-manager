use std::{
    path::Path,
    sync::{atomic::Ordering, mpsc},
};

mod decode;
mod device;
mod handle;

use decode::{AudioDecoderHandle, DecodeError};
use device::AudioDevice;
use handle::{PlayerHandle, SharedAudioState};

pub use handle::PlaybackState;

use crate::ipc::IPCMessage;

pub struct AudioPlayer {
    handle: PlayerHandle,
    decoder: Option<AudioDecoderHandle>,

    _device: Option<AudioDevice>,
}

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("")]
    PlayerOffline,

    #[error("{0}")]
    Decode(#[from] DecodeError),
}

type AudioResult<T> = std::result::Result<T, AudioError>;

macro_rules! with_decoder {
    ($self:ident, $dec:ident => $body:block) => {{
        let $dec = $self.decoder.as_ref().ok_or(AudioError::PlayerOffline)?;
        $body
    }};
}

impl AudioPlayer {
    pub fn new(rx: mpsc::Sender<IPCMessage>) -> Self {
        let shared_state = SharedAudioState::new(rx);
        let audio_handle = PlayerHandle {
            shared: shared_state.clone(),
        };

        let mut audio_device = None;
        let mut audio_decoder = None;

        match AudioDevice::new(&shared_state.clone()) {
            Ok((rb_prod, device)) => {
                match AudioDecoderHandle::new(rb_prod, shared_state.clone(), device.config.clone())
                {
                    Ok(decoder) => audio_decoder = Some(decoder),
                    Err(err) => eprintln!("{err}"),
                }

                audio_device = Some(device);
            }
            Err(err) => eprintln!("{err}"),
        }

        Self {
            handle: audio_handle,
            decoder: audio_decoder,
            _device: audio_device,
        }
    }

    pub fn play(&self, path: &impl AsRef<Path>) -> AudioResult<()> {
        self.stop().ok();

        self.handle
            .shared
            .samples_played
            .store(0, Ordering::Release);
        self.handle
            .shared
            .estimated_audio_len
            .store(0, Ordering::Release);

        with_decoder!(self, decoder => {
            decoder.start(path);
            self.handle.resume();

            Ok(())
        })
    }

    pub fn pause(&self) -> AudioResult<()> {
        with_decoder!(self, decoder => {
            decoder.pause();
            self.handle.pause();

            Ok(())
        })
    }

    pub fn resume(&self) -> AudioResult<()> {
        with_decoder!(self, decoder => {
            decoder.resume();
            self.handle.resume();

            Ok(())
        })
    }

    pub fn stop(&self) -> AudioResult<()> {
        with_decoder!(self, decoder => {
            decoder.stop();
            self.handle.stop();

            Ok(())
        })
    }

    pub fn seek(&self, pos: f64) -> AudioResult<()> {
        let len = self
            .handle
            .shared
            .estimated_audio_len
            .load(Ordering::Relaxed);

        let millis = (len as f64 * pos) as u32;

        with_decoder!(self, decoder => {
            decoder.seek(millis);
            self.handle.seek(millis);

            Ok(())
        })
    }

    pub fn position(&self) -> f64 {
        self.handle.position()
    }

    pub fn position_pretty(&self) -> String {
        self.handle.position_pretty()
    }

    pub fn playback_state(&self) -> PlaybackState {
        self.handle.playback_state()
    }

    pub fn get_volume(&self) -> f32 {
        self.handle.get_volume()
    }

    pub fn set_volume(&self, volume: f32) {
        self.handle.set_volume(volume)
    }
}
