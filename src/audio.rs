use std::path::Path;

mod decode;
mod device;
mod handle;

use decode::{AudioDecoderHandle, DecodeError};
use device::AudioDevice;
use handle::{PlayerHandle, SharedAudioState};

pub use handle::PlaybackState;

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
    pub fn new() -> Self {
        let shared_state = SharedAudioState::new();
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

    pub fn seek(&self, millis: u32) -> AudioResult<()> {
        with_decoder!(self, decoder => {
            decoder.seek(millis);
            self.handle.seek(millis);

            Ok(())
        })
    }

    pub fn position(&self) -> u32 {
        self.handle.position()
    }

    pub fn playback_state(&self) -> PlaybackState {
        self.handle.playback_state()
    }
}
