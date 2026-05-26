use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::time::Duration;

use atomic_float::AtomicF32;

use crate::LogErrorExt;
use crate::ipc::IPCMessage;

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct PlayerFlags: u8 {
        const PLAYING      = 1 << 0;
        const PAUSED       = 1 << 1;
        const STOPPED      = 1 << 2;
        const SEEK_PENDING = 1 << 3;
        const FLUSHING     = 1 << 4;
        const DRAINING     = 1 << 5;
        const LOOP         = 1 << 6;
    }
}

const STATE_MASK: PlayerFlags = PlayerFlags::PLAYING
    .union(PlayerFlags::PAUSED)
    .union(PlayerFlags::STOPPED);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

impl PlaybackState {
    pub fn from_flags(f: PlayerFlags) -> Self {
        if f.contains(PlayerFlags::PAUSED) {
            return PlaybackState::Paused;
        }
        if f.contains(PlayerFlags::STOPPED) {
            return PlaybackState::Stopped;
        }
        PlaybackState::Playing
    }
}

pub struct SharedAudioState {
    flags: AtomicU8,

    pub webview_sender: mpsc::Sender<IPCMessage>,

    pub sample_rate: AtomicU32,
    pub estimated_audio_len: AtomicU32,
    pub samples_played: AtomicU64,
    pub seek_target: AtomicU32,
    pub volume: AtomicF32,
    pub abort: AtomicBool,
    ready: Arc<(Mutex<bool>, Condvar)>,
}

impl SharedAudioState {
    pub fn new(webview_sender: mpsc::Sender<IPCMessage>) -> Arc<Self> {
        Arc::new(Self {
            webview_sender,
            flags: AtomicU8::new(PlayerFlags::PLAYING.bits()),
            sample_rate: AtomicU32::new(44100),
            estimated_audio_len: AtomicU32::new(0),
            samples_played: AtomicU64::new(0),
            seek_target: AtomicU32::new(0),
            volume: AtomicF32::new(1.0),
            abort: AtomicBool::new(false),
            ready: Arc::new((Mutex::new(false), Condvar::new())),
        })
    }

    pub fn set_state(&self, next_state: PlayerFlags) {
        self.flags
            .fetch_update(Ordering::Release, Ordering::Relaxed, |w| {
                let current = PlayerFlags::from_bits_retain(w);
                Some((current & !STATE_MASK | (next_state & STATE_MASK)).bits())
            })
            .sure("Failed to set PlayerFlags");
    }

    pub fn set_flag(&self, flag: PlayerFlags) {
        self.flags.fetch_or(flag.bits(), Ordering::Release);
    }

    pub fn is_paused(&self) -> bool {
        let flags = PlayerFlags::from_bits_retain(self.flags.load(Ordering::Relaxed));
        flags.contains(PlayerFlags::PAUSED)
    }

    pub fn clear_flag(&self, flag: PlayerFlags) {
        self.flags.fetch_and(!flag.bits(), Ordering::Release);
    }

    #[inline(always)]
    pub fn load_flags(&self) -> PlayerFlags {
        PlayerFlags::from_bits_retain(self.flags.load(Ordering::Relaxed))
    }

    pub fn set_not_ready(&self) {
        let (lock, _) = &*self.ready;
        *lock.lock().unwrap() = false;
    }

    pub fn set_ready(&self) {
        let (lock, cvar) = &*self.ready;
        *lock.lock().unwrap() = true;
        cvar.notify_all();
    }

    pub fn wait_until_ready(&self, timeout: Duration) -> bool {
        let (lock, cvar) = &*self.ready;

        let ready = lock.lock().unwrap();
        let result = cvar.wait_timeout_while(ready, timeout, |r| !*r).unwrap();

        !result.1.timed_out()
    }
}

pub struct PlayerHandle {
    pub shared: Arc<SharedAudioState>,
}

impl PlayerHandle {
    pub fn pause(&self) {
        self.shared.set_state(PlayerFlags::PAUSED);
    }

    pub fn resume(&self) {
        self.shared.set_state(PlayerFlags::PLAYING);
    }

    pub fn stop(&self) {
        self.shared.abort.store(true, Ordering::Release);

        self.shared.set_not_ready();
        self.shared.set_state(PlayerFlags::STOPPED);
        self.shared.clear_flag(PlayerFlags::DRAINING);
        self.shared.set_flag(PlayerFlags::FLUSHING);
    }

    pub fn seek(&self, millis: u32) {
        self.shared.seek_target.store(millis, Ordering::Release);
        self.shared.set_flag(PlayerFlags::SEEK_PENDING);
    }

    pub fn position(&self) -> f64 {
        let played = self.shared.samples_played.load(Ordering::Acquire);
        let len_ms = self.shared.estimated_audio_len.load(Ordering::Acquire);

        if len_ms == 0 {
            return 0.0;
        }

        let sample_rate = self.shared.sample_rate.load(Ordering::Relaxed) as u64;
        let len_samples = (len_ms as u64 * sample_rate) / 1000;
        let looping = self.shared.load_flags().contains(PlayerFlags::LOOP);

        let pos_in_loop = if looping {
            played % len_samples
        } else {
            played
        };

        (pos_in_loop as f64 / len_samples as f64).clamp(0.0, 1.0)
    }

    pub fn position_pretty(&self) -> String {
        let played = self.shared.samples_played.load(Ordering::Acquire);
        let len_ms = self.shared.estimated_audio_len.load(Ordering::Acquire);

        if len_ms == 0 {
            return "0:00 / 0:00".to_string();
        }

        let sample_rate = self.shared.sample_rate.load(Ordering::Relaxed) as u64;
        let played_ms = (played * 1000) / sample_rate;

        format_time(played_ms) + "/" + &format_time(len_ms as u64)
    }

    pub fn playback_state(&self) -> PlaybackState {
        PlaybackState::from_flags(self.shared.load_flags())
    }

    pub fn get_volume(&self) -> f32 {
        self.shared.volume.load(Ordering::Relaxed)
    }

    pub fn set_volume(&self, volume: f32) {
        self.shared
            .volume
            .store(volume.clamp(0.0, 2.0), Ordering::Release);
    }

    pub fn set_looping(&self, enabled: bool) {
        if enabled {
            self.shared.set_flag(PlayerFlags::LOOP);
        } else {
            self.shared.clear_flag(PlayerFlags::LOOP);
        }
    }

    pub fn is_looping(&self) -> bool {
        self.shared.load_flags().contains(PlayerFlags::LOOP)
    }
}

fn format_time(ms: u64) -> String {
    let total_seconds = ms / 1000;

    let seconds = total_seconds % 60;
    let minutes = (total_seconds / 60) % 60;
    let hours = total_seconds / 3600;

    if hours > 0 {
        return format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
    }

    format!("{:02}:{:02}", minutes, seconds)
}
