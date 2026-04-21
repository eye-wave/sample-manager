use arc_swap::ArcSwap;
use assert_no_alloc::*;

use std::sync::{Arc, atomic::Ordering};

use crossbeam_channel::Receiver;

use super::event::AudioEvent;
use super::resample::interpolate;
use super::{PlayerFlags, PlayerProps, SharedAudioBuffer};

#[cfg(debug_assertions)]
#[global_allocator]
static A: AllocDisabler = AllocDisabler;

pub fn audio_loop<S>(data: &mut [S], state: AudioLoopState)
where
    S: cpal::Sample + cpal::FromSample<f32>,
{
    let shared = state.shared.load();
    let volume = state.props.volume.load(Ordering::Relaxed);

    let ratio = state.props.get_playback_rate(shared.sample_rate);

    if let Ok(msg) = state.rx.try_recv()
        && let AudioEvent::Stop = msg
    {
        state.props.position.store(0.0, Ordering::Relaxed);
        state
            .props
            .clear_flag(PlayerFlags::IS_PLAYING, Ordering::Relaxed);
    }

    assert_no_alloc(|| {
        let mut pos = state.props.position.load(Ordering::Relaxed);

        if !state
            .props
            .get_flag(super::PlayerFlags::IS_PLAYING, Ordering::Relaxed)
        {
            data.fill(S::EQUILIBRIUM);
            return;
        }

        let channels = shared.channels.len();
        if channels == 0 {
            data.fill(S::EQUILIBRIUM);
            return;
        }

        let len = shared.channels[0].len() as f64;

        for frame in data.chunks_mut(channels) {
            for (ch, out_sample) in frame.iter_mut().enumerate() {
                let chan_data = &shared.channels[ch];
                let sample = interpolate(chan_data, pos) * volume;

                *out_sample = S::from_sample(sample);
            }

            pos += ratio;
            if pos >= len {
                pos = 0.0;
            }
        }

        state.props.position.store(pos, Ordering::SeqCst);
    });
}

#[derive(Clone)]
pub(super) struct AudioLoopState {
    pub rx: Arc<Receiver<AudioEvent>>,
    pub shared: Arc<ArcSwap<SharedAudioBuffer>>,
    pub props: Arc<PlayerProps>,
}

#[macro_export]
macro_rules! build_stream_match {
    ($device:expr, $props: expr, $config:expr, $state:expr, $err_fn:expr, { $( $fmt:path => $ty:ty ),* $(,)? }) => {{
        use $crate::player::audio_loop::audio_loop;

        match $device.default_output_config().unwrap().sample_format() {
            $(
                $fmt => $device.build_output_stream(
                    $config,
                    move |data: &mut [$ty], _| audio_loop(data, $props),
                    $err_fn,
                    None,
                ),
            )*
            other => panic!("Unsupported sample format {:?}", other),
        }
    }};
}
