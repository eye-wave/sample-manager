use std::sync::{
    Arc,
    atomic::{AtomicU8, AtomicU32, Ordering},
};

use atomic_float::AtomicF32;

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct PlayerFlags: u8 {
        const PLAYING      = 1 << 0;
        const PAUSED       = 1 << 1;
        const STOPPED      = 1 << 2;
        const SEEK_PENDING = 1 << 3;
        const FLUSHING     = 1 << 4;
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

    pub position_millis: AtomicU32,
    pub seek_target: AtomicU32,
    pub volume: AtomicF32,
}

impl SharedAudioState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            flags: AtomicU8::new(PlayerFlags::PLAYING.bits()),
            seek_target: AtomicU32::new(0),
            position_millis: AtomicU32::new(0),
            volume: AtomicF32::new(1.0),
        })
    }

    pub fn set_state(&self, next_state: PlayerFlags) {
        self.flags
            .fetch_update(Ordering::Release, Ordering::Relaxed, |w| {
                let current = PlayerFlags::from_bits_retain(w);
                Some((current & !STATE_MASK | (next_state & STATE_MASK)).bits())
            })
            .ok();
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
        self.shared.set_state(PlayerFlags::STOPPED);
        self.shared.set_flag(PlayerFlags::FLUSHING);
    }

    pub fn seek(&self, millis: u32) {
        self.shared.seek_target.store(millis, Ordering::Release);
        self.shared.set_flag(PlayerFlags::SEEK_PENDING);
    }

    pub fn position(&self) -> u32 {
        self.shared.position_millis.load(Ordering::Acquire)
    }

    pub fn playback_state(&self) -> PlaybackState {
        PlaybackState::from_flags(self.shared.load_flags())
    }

    pub fn get_volume(&self) -> f32 {
        self.shared.volume.load(Ordering::Relaxed)
    }

    pub fn set_volume(&self, volume: f32) {
        self.shared.volume.store(volume, Ordering::Release);
    }
}
