//! Splice's "scrambled" mp3 samples. A downloaded file is a 28 byte header and the mp3 data:
//! bytes 2..10 are a little endian size, bytes 10..28 a key. The first `size` bytes of the data and
//! the `size` bytes after the next `size` ones are XORed with the key (repeated). Ported from the
//! website's JavaScript.
use std::ops::Range;

const HEADER: usize = 28;
const KEY: Range<usize> = 10..HEADER;

/// The number of frames of silence at the start of a decoded Splice preview: LAME's encoder delay
/// (576) plus the mp3 decoder delay (529). The previews carry no gapless header, so a decoder does
/// not strip it.
pub const MP3_START_DELAY: u32 = 576 + 529;

/// XORs `data[start..end]` (cut to the data) with `key`, which starts again at `start`.
fn xor(data: &mut [u8], start: u64, end: u64, key: &[u8]) {
    let len = data.len() as u64;
    let (start, end) = (start.min(len) as usize, end.min(len) as usize);
    for (n, b) in data[start..end.max(start)].iter_mut().enumerate() {
        *b ^= key[n % key.len()];
    }
}

/// The mp3 inside a scrambled Splice file.
pub fn unscramble(mut file: Vec<u8>) -> Result<Vec<u8>, String> {
    if file.len() <= HEADER {
        return Err("too short to be a Splice sample".into());
    }
    let size = file[2..10]
        .iter()
        .rev()
        .fold(0u64, |s, &b| s.wrapping_mul(256).wrapping_add(b as u64));
    let key = file[KEY].to_vec();
    let mut audio = file.split_off(HEADER);
    xor(&mut audio, 0, size, &key);
    let second = size.saturating_mul(2);
    xor(&mut audio, second, second.saturating_add(size), &key);
    Ok(audio)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_unscrambles_what_the_website_script_does() {
        // worked by hand: key 1..=18, size 4: bytes 0..4 and 8..12 of the data are XORed
        let key: [u8; 18] = std::array::from_fn(|i| i as u8 + 1);
        let mut file = vec![9, 9, 4, 0, 0, 0, 0, 0, 0, 0];
        file.extend(key);
        file.extend([0x10; 14]);
        let mut want = vec![
            0x10 ^ 1,
            0x10 ^ 2,
            0x10 ^ 3,
            0x10 ^ 4,
            0x10,
            0x10,
            0x10,
            0x10,
        ];
        want.extend([0x10 ^ 1, 0x10 ^ 2, 0x10 ^ 3, 0x10 ^ 4, 0x10, 0x10]);
        assert_eq!(unscramble(file).unwrap(), want);
    }

    #[test]
    fn the_key_repeats_and_sizes_past_the_end_are_cut() {
        let audio: Vec<u8> = (0..200u32).map(|i| (i * 31) as u8).collect();
        let key: [u8; 18] = std::array::from_fn(|i| 0xA0 + i as u8);
        // the scrambled data, byte by byte as the website's script defines it
        let scramble = |size: u64| {
            let mut w = audio.clone();
            for i in 0..w.len() as u64 {
                if i < size {
                    w[i as usize] ^= key[(i % 18) as usize];
                }
                if (size * 2..size * 3).contains(&i) {
                    w[i as usize] ^= key[((i - size * 2) % 18) as usize];
                }
            }
            w
        };
        let file = |size: u64| {
            let mut f = vec![1, 2];
            f.extend(size.to_le_bytes());
            f.extend(key);
            f.extend(scramble(size));
            f
        };
        for size in [0, 1, 17, 18, 19, 50, 67, 100, 199, 200, 1000] {
            assert_eq!(unscramble(file(size)).unwrap(), audio, "size {size}");
        }
        // a size too big to be real: no panic
        let mut f = file(5);
        f[2..10].copy_from_slice(&[0xFF; 8]);
        assert_eq!(unscramble(f).unwrap().len(), audio.len());
        assert!(unscramble(vec![0; 28]).is_err() && unscramble(vec![]).is_err());
    }
}
