use std::sync::Arc;
use std::sync::atomic::Ordering;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, SupportedStreamConfig};
use ringbuf::traits::{Consumer, Observer};
use ringbuf::wrap::caching::Caching;
use ringbuf::{HeapRb, SharedRb, storage::Heap, traits::Split};

use super::handle::{PlayerFlags, SharedAudioState};

pub struct AudioDevice {
    _stream: Stream,
    pub config: SupportedStreamConfig,
}

#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("")]
    UnsupportedSampleFormat,

    #[error("")]
    NoOutputDevice,
}

pub type RingProd = Caching<Arc<SharedRb<Heap<f32>>>, true, false>;
pub type RingCons = Caching<Arc<SharedRb<Heap<f32>>>, false, true>;

const RING_CAPACITY: usize = 48_000 * 2 * 4;

impl AudioDevice {
    pub fn new(
        shared_state: &Arc<SharedAudioState>,
    ) -> Result<(RingProd, Self), Box<dyn std::error::Error>> {
        let rb = HeapRb::<f32>::new(RING_CAPACITY);
        let (rb_prod, rb_cons) = rb.split();

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(DeviceError::NoOutputDevice)?;

        let config = device.default_output_config()?;

        macro_rules! build_stream {
            ($T:ty) => {{
                let shared_cb = Arc::clone(&shared_state);
                let mut rb_cons = rb_cons;

                device.build_output_stream(
                    &config.config(),
                    move |data: &mut [$T], _info: &cpal::OutputCallbackInfo| {
                        audio_loop(data, &shared_cb, &mut rb_cons)
                    },
                    |err| eprintln!("cpal stream error: {err}"),
                    None,
                )?
            }};
        }

        let sample_format = config.sample_format();

        let _stream = match sample_format {
            cpal::SampleFormat::F32 => build_stream!(f32),
            cpal::SampleFormat::F64 => build_stream!(f64),
            cpal::SampleFormat::I8 => build_stream!(i8),
            cpal::SampleFormat::I16 => build_stream!(i16),
            cpal::SampleFormat::I32 => build_stream!(i32),
            cpal::SampleFormat::U8 => build_stream!(u8),
            cpal::SampleFormat::U16 => build_stream!(u16),
            cpal::SampleFormat::U32 => build_stream!(u32),
            _ => return Err(Box::new(DeviceError::UnsupportedSampleFormat)),
        };

        _stream.play().ok();

        Ok((rb_prod, Self { config, _stream }))
    }
}

fn audio_loop<S>(data: &mut [S], shared_state: &Arc<SharedAudioState>, rb_cons: &mut RingCons)
where
    S: cpal::Sample + cpal::FromSample<f32>,
{
    let f = shared_state.load_flags();

    let paused = f.contains(PlayerFlags::PAUSED);
    let flushing = f.contains(PlayerFlags::FLUSHING);

    if paused || flushing {
        if flushing {
            let n = data.len().min(rb_cons.occupied_len());
            for _ in 0..n {
                rb_cons.try_pop();
            }
        }
        for s in data.iter_mut() {
            *s = S::EQUILIBRIUM;
        }
        return;
    }

    let volume = shared_state.volume.load(Ordering::SeqCst);

    for s in data.iter_mut() {
        *s = match rb_cons.try_pop() {
            Some(f) => S::from_sample(f * volume),
            None => S::EQUILIBRIUM,
        };
    }
}
