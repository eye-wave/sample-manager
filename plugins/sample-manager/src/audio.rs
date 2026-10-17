//! Audio files: decoding (symphonia: wav, mp3, flac, ogg), the waveform peaks, and what the
//! editor and the plugin's audio thread share to play a sample (`Player`, `Voice`).
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::*};
use std::sync::{Arc, Mutex};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

/// How many peaks a clip keeps for its waveform: enough for the widest window, `Clip::columns`
/// merges them down to the columns actually drawn.
const PEAKS: usize = 8192;

/// A decoded file: always stereo (mono is doubled, channels past the second are dropped).
// the whole file in memory as f32 (8 bytes a frame), no streaming
#[derive(Default)]
pub struct Clip {
    pub rate: u32,
    pub frames: Vec<[f32; 2]>,
    /// Loudest absolute sample per slice of the clip, `PEAKS` slices (fewer for a tiny clip).
    peaks: Vec<f32>,
}

impl Clip {
    fn new(rate: u32, frames: Vec<[f32; 2]>) -> Self {
        let b = frames.len().min(PEAKS);
        let peaks = (0..b)
            .map(|i| {
                let s = &frames[i * frames.len() / b..(i + 1) * frames.len() / b];
                s.iter().flatten().fold(0.0, |m, x| f32::max(m, x.abs()))
            })
            .collect();
        Self {
            rate,
            frames,
            peaks,
        }
    }

    pub fn secs(&self) -> f32 {
        self.frames.len() as f32 / self.rate.max(1) as f32
    }

    /// The waveform as `n` columns, each the peak (0..=1 and a bit over) of its share of the clip.
    pub fn columns(&self, n: usize) -> Vec<f32> {
        let b = self.peaks.len();
        if b == 0 {
            return vec![0.0; n];
        }
        (0..n)
            .map(|i| {
                let a = i * b / n; // below b
                let z = ((i + 1) * b / n).max(a + 1);
                self.peaks[a..z].iter().fold(0.0, |m, &x| f32::max(m, x))
            })
            .collect()
    }
}

/// Decodes the whole file.
// runs on the caller's thread, a long file stalls the window while it decodes
pub fn decode(path: &Path) -> Result<Clip, Box<dyn std::error::Error>> {
    let mss = MediaSourceStream::new(Box::new(std::fs::File::open(path)?), Default::default());
    let mut hint = Hint::new();
    if let Some(x) = path.extension().and_then(|x| x.to_str()) {
        hint.with_extension(x);
    }
    let mut format = symphonia::default::get_probe().probe(
        &hint,
        mss,
        FormatOptions::default(),
        MetadataOptions::default(),
    )?;
    let track = format
        .default_track(TrackType::Audio)
        .ok_or("no audio in the file")?;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or("no audio in the file")?;
    let (id, rate) = (track.id, params.sample_rate.ok_or("unknown sample rate")?);
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())?;
    let (mut frames, mut buf) = (vec![], Vec::<f32>::new());
    loop {
        let packet = match format.next_packet() {
            Ok(Some(p)) => p,
            // the end of the stream (a cut off file is an I/O error)
            Ok(None) | Err(Error::IoError(_) | Error::ResetRequired) => break,
            Err(e) => return Err(e.into()),
        };
        if packet.track_id != id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(d) => {
                let ch = d.spec().channels().count().max(1);
                d.copy_to_vec_interleaved(&mut buf);
                frames.extend(buf.chunks_exact(ch).map(|f| [f[0], f[ch.min(2) - 1]]));
            }
            Err(Error::DecodeError(_)) => {} // a broken packet: skip it
            Err(e) => return Err(e.into()),
        }
    }
    if frames.is_empty() {
        return Err("no audio in the file".into());
    }
    Ok(Clip::new(rate, frames))
}

/// What the editor (which picks the sample) and the audio thread (which plays it) share. Plays
/// nothing in the standalone app: nobody runs a `Voice` there.
#[derive(Default)]
pub struct Player {
    /// The next clip. The audio thread swaps it with the one it plays, so the old clip is freed
    /// by the next `play` on the editor's thread and never on the audio thread.
    mailbox: Mutex<Option<Arc<Clip>>>,
    fresh: AtomicBool,
    /// The play position in frames of the clip, for the playhead.
    pos: AtomicUsize,
    /// The voice holds its position and plays silence.
    paused: AtomicBool,
    /// Playback mode: loop, else one shot (plays once, then waits paused at the start). One shot
    /// is the default.
    looping: AtomicBool,
    /// A pending jump (frame), taken by the voice at its next block.
    seek: AtomicUsize,
    seeking: AtomicBool,
}

impl Player {
    /// Plays `c` from its start instead of what plays now (an empty clip is silence).
    pub fn play(&self, c: Arc<Clip>) {
        *self.mailbox.lock().unwrap() = Some(c);
        self.pos.store(0, Relaxed);
        self.paused.store(false, Relaxed);
        self.seeking.store(false, Relaxed);
        self.fresh.store(true, Release);
    }

    /// Silence: the editor was closed.
    pub fn stop(&self) {
        self.play(Arc::default());
    }

    pub fn toggle_pause(&self) {
        self.paused.fetch_xor(true, Relaxed);
    }

    pub fn paused(&self) -> bool {
        self.paused.load(Relaxed)
    }

    pub fn looping(&self) -> bool {
        self.looping.load(Relaxed)
    }

    /// Switches between loop and one shot.
    pub fn toggle_loop(&self) {
        self.looping.fetch_xor(true, Relaxed);
    }

    /// Jumps to `frame` (the voice keeps it inside the clip); the playhead moves at once.
    pub fn seek(&self, frame: usize) {
        self.pos.store(frame, Relaxed);
        self.seek.store(frame, Relaxed);
        self.seeking.store(true, Release);
    }

    pub fn pos(&self) -> usize {
        self.pos.load(Relaxed)
    }
}

/// The audio thread's side of a `Player`: plays the clip in a loop. Never blocks or allocates.
pub struct Voice {
    player: Arc<Player>,
    clip: Option<Arc<Clip>>,
    /// The host's sample rate.
    rate: f64,
    /// Position in frames of the clip.
    at: f64,
}

impl Voice {
    pub fn new(player: Arc<Player>, rate: f64) -> Self {
        Self {
            player,
            clip: None,
            rate: if rate > 0.0 { rate } else { 44100.0 }, // 0 or NaN would make NaN samples
            at: 0.0,
        }
    }

    /// Fills one block (`left` and `right` are the same length).
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        // the editor is mid-`play`: the next block takes the new clip
        if self.player.fresh.load(Acquire)
            && let Ok(mut m) = self.player.mailbox.try_lock()
        {
            std::mem::swap(&mut *m, &mut self.clip);
            self.at = 0.0;
            self.player.fresh.store(false, Release);
        }
        let Some(c) = self.clip.as_ref().filter(|c| !c.frames.is_empty()) else {
            left.fill(0.0);
            right.fill(0.0);
            return;
        };
        let (n, step) = (c.frames.len(), c.rate as f64 / self.rate);
        if self.player.seeking.swap(false, Acquire) {
            self.at = self.player.seek.load(Relaxed).min(n - 1) as f64;
        }
        if self.player.paused() {
            left.fill(0.0);
            right.fill(0.0);
            return;
        }
        let looping = self.player.looping();
        for k in 0..left.len() {
            // linear interpolation between neighbours (also for another sample rate)
            let i = self.at as usize;
            let t = (self.at - i as f64) as f32;
            let (a, b) = (c.frames[i], c.frames[(i + 1) % n]);
            left[k] = a[0] + (b[0] - a[0]) * t;
            right[k] = a[1] + (b[1] - a[1]) * t;
            self.at += step;
            if self.at >= n as f64 {
                if looping {
                    self.at %= n as f64;
                } else {
                    // one shot, done: silence, and wait at the start for the play button
                    left[k + 1..].fill(0.0);
                    right[k + 1..].fill(0.0);
                    self.at = 0.0;
                    self.player.paused.store(true, Relaxed);
                    break;
                }
            }
        }
        self.player.pos.store(self.at as usize, Relaxed);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A 16-bit mono PCM WAV with these samples.
    pub(crate) fn wav(rate: u32, pcm: &[i16]) -> Vec<u8> {
        let data: Vec<u8> = pcm.iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut w = b"RIFF".to_vec();
        w.extend((36 + data.len() as u32).to_le_bytes());
        w.extend(b"WAVEfmt ");
        w.extend(16u32.to_le_bytes());
        w.extend([1, 0, 1, 0]); // PCM, 1 channel
        w.extend(rate.to_le_bytes());
        w.extend((rate * 2).to_le_bytes());
        w.extend([2, 0, 16, 0]);
        w.extend(b"data");
        w.extend((data.len() as u32).to_le_bytes());
        w.extend(data);
        w
    }

    #[test]
    fn decodes_a_wav_and_summarises_its_peaks() {
        let path = std::env::temp_dir().join(format!("sm-decode-{}.wav", std::process::id()));
        std::fs::write(&path, wav(8000, &[0, 16384, -32768, 8192])).unwrap();
        let c = decode(&path);
        std::fs::write(&path, b"not audio").unwrap();
        let bad = decode(&path);
        std::fs::remove_file(&path).unwrap();
        let c = c.unwrap();
        assert_eq!((c.rate, c.frames.len()), (8000, 4));
        assert_eq!(c.frames[1], [0.5, 0.5]); // mono doubled
        assert_eq!(c.columns(4), vec![0.0, 0.5, 1.0, 0.25]);
        assert_eq!(c.columns(2), vec![0.5, 1.0]); // merged: the louder of each pair
        assert_eq!(c.columns(8).len(), 8); // more columns than peaks repeats them
        assert_eq!(c.secs(), 0.0005);
        assert!(bad.is_err());
        assert_eq!(Clip::default().columns(3), vec![0.0; 3]);
    }

    #[test]
    fn the_voice_loops_resamples_and_swaps_clips() {
        let p = Arc::new(Player::default());
        p.toggle_loop();
        let mut v = Voice::new(p.clone(), 4.0);
        let (mut l, mut r) = ([1.0; 4], [1.0; 4]);
        v.render(&mut l, &mut r);
        assert_eq!((l, r), ([0.0; 4], [0.0; 4])); // nothing picked: silence
        // a 2 Hz clip on a 4 Hz host: every frame twice (half steps), looping
        let ramp = |a: f32, b: f32| Arc::new(Clip::new(2, vec![[a, -a], [b, -b]]));
        p.play(ramp(0.0, 1.0));
        v.render(&mut l, &mut r);
        assert_eq!(l, [0.0, 0.5, 1.0, 0.5]);
        assert_eq!(r, [0.0, -0.5, -1.0, -0.5]);
        assert_eq!(p.pos(), 0); // wrapped back to the start
        v.render(&mut l[..1], &mut r[..1]);
        assert_eq!(p.pos(), 0); // the half step is inside frame 0
        v.render(&mut l[..2], &mut r[..2]);
        assert_eq!(p.pos(), 1);
        p.play(ramp(0.5, 0.5)); // replaces it from the start
        v.render(&mut l, &mut r);
        assert_eq!(l, [0.5; 4]);
        p.play(Arc::default()); // an empty clip stops it
        v.render(&mut l, &mut r);
        assert_eq!(l, [0.0; 4]);
    }

    #[test]
    fn a_zero_host_rate_does_not_make_nan() {
        let p = Arc::new(Player::default());
        p.toggle_loop();
        let mut v = Voice::new(p.clone(), 0.0);
        p.play(Arc::new(Clip::new(2, vec![[1.0, 1.0], [1.0, 1.0]])));
        let (mut l, mut r) = ([0.0; 8], [0.0; 8]);
        v.render(&mut l, &mut r);
        assert!(l.iter().chain(&r).all(|x| x.is_finite()));
    }

    #[test]
    fn one_shot_plays_once_then_waits_at_the_start() {
        let p = Arc::new(Player::default());
        assert!(!p.looping()); // one shot is the default
        let mut v = Voice::new(p.clone(), 1.0);
        let (mut l, mut r) = ([9.0; 5], [9.0; 5]);
        p.play(Arc::new(Clip::new(
            1,
            vec![[1.0, -1.0], [2.0, -2.0], [3.0, -3.0]],
        )));
        v.render(&mut l, &mut r);
        assert_eq!(
            (l, r),
            ([1.0, 2.0, 3.0, 0.0, 0.0], [-1.0, -2.0, -3.0, 0.0, 0.0])
        );
        assert!(p.paused() && p.pos() == 0); // done: the play button shows
        v.render(&mut l, &mut r);
        assert_eq!(l, [0.0; 5]); // stays silent
        p.toggle_pause(); // play again, from the start
        v.render(&mut l[..2], &mut r[..2]);
        assert_eq!(l[..2], [1.0, 2.0]);
        p.toggle_loop(); // looping: wraps instead
        v.render(&mut l, &mut r);
        assert_eq!(l, [3.0, 1.0, 2.0, 3.0, 1.0]);
        assert!(!p.paused());
    }

    #[test]
    fn pause_holds_the_position_and_seek_jumps() {
        let p = Arc::new(Player::default());
        p.toggle_loop();
        let mut v = Voice::new(p.clone(), 1.0);
        let (mut l, mut r) = ([9.0; 2], [9.0; 2]);
        p.play(Arc::new(Clip::new(
            1,
            (0..4).map(|i| [i as f32; 2]).collect(),
        )));
        v.render(&mut l, &mut r);
        assert_eq!((l, p.pos()), ([0.0, 1.0], 2));
        p.toggle_pause();
        v.render(&mut l, &mut r);
        assert_eq!((l, r, p.pos()), ([0.0; 2], [0.0; 2], 2)); // silent, still at 2
        p.seek(3); // while paused: the playhead moves, nothing plays
        assert_eq!(p.pos(), 3);
        v.render(&mut l, &mut r);
        assert_eq!(l, [0.0; 2]);
        p.toggle_pause();
        v.render(&mut l, &mut r);
        assert_eq!((l, p.pos()), ([3.0, 0.0], 1)); // from frame 3, then it loops
        p.seek(100); // past the end: the last frame
        v.render(&mut l[..1], &mut r[..1]);
        assert_eq!(l[0], 3.0);
        p.toggle_pause();
        p.play(Arc::default()); // a new clip plays, whatever the pause was
        assert!(!p.paused());
        p.stop();
        v.render(&mut l, &mut r);
        assert_eq!(l, [0.0; 2]);
    }
}
