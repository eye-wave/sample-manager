//! Raw pixels for [`crate::gpu`]: no file formats, an image is `w * h * 4` bytes of RGBA (sRGB
//! values, straight alpha). They live in one sheet of `GRID` x `GRID` cells of `SLOT` x `SLOT` px;
//! an image takes a cell (so it is at most `SLOT` px each way) and is drawn by its cell number.
// one fixed sheet, 64 images; a bigger sheet or a packer if that is ever too few.

pub const SLOT: u32 = 128;
const GRID: u32 = 8;
pub const SHEET: u32 = SLOT * GRID;

pub struct Images {
    /// Goes up whenever the pixels change; `gpu::render` uploads the sheet when it moved.
    pub version: u64,
    /// The sheet, `SHEET` x `SHEET` RGBA.
    pub rgba: Vec<u8>,
    /// Size of the image in each cell, (0, 0) when the cell is free.
    size: [(u32, u32); (GRID * GRID) as usize],
}

impl Default for Images {
    fn default() -> Self {
        Self {
            version: 0,
            rgba: vec![0; (SHEET * SHEET * 4) as usize],
            size: [(0, 0); (GRID * GRID) as usize],
        }
    }
}

impl Images {
    /// Puts a `w` x `h` image (`rgba` is `w * h * 4` bytes, rows top to bottom) in a free cell
    /// and returns the cell.
    pub fn add(&mut self, w: u32, h: u32, rgba: &[u8]) -> Result<usize, String> {
        if w == 0 || h == 0 || w > SLOT || h > SLOT || rgba.len() != (w * h * 4) as usize {
            return Err(format!(
                "an image is 1 to {SLOT} px each way and 4 bytes a pixel (got {w}x{h}, {} bytes)",
                rgba.len()
            ));
        }
        let slot = self
            .size
            .iter()
            .position(|s| s.0 == 0)
            .ok_or("no room for another image")?;
        let row = (w * 4) as usize;
        for (y, src) in rgba.chunks_exact(row).enumerate() {
            let at = self.offset(slot, y as u32);
            self.rgba[at..at + row].copy_from_slice(src);
        }
        self.size[slot] = (w, h);
        self.version += 1;
        Ok(slot)
    }

    /// Frees a cell; a cell that is not in use is left alone.
    pub fn remove(&mut self, slot: usize) {
        let Some(&(w, h)) = self.size.get(slot).filter(|s| s.0 != 0) else {
            return;
        };
        for y in 0..h {
            let at = self.offset(slot, y);
            self.rgba[at..at + (w * 4) as usize].fill(0);
        }
        self.size[slot] = (0, 0);
        self.version += 1;
    }

    /// Byte offset of row `y` of the cell `slot`.
    fn offset(&self, slot: usize, y: u32) -> usize {
        let (cx, cy) = (slot as u32 % GRID * SLOT, slot as u32 / GRID * SLOT);
        (((cy + y) * SHEET + cx) * 4) as usize
    }

    /// The image of a cell as `[u0, v0, u1, v1]`, inset half a texel so that the linear filter
    /// never reads a neighbour; `None` for a free cell.
    pub fn uv(&self, slot: usize) -> Option<[f32; 4]> {
        let &(w, h) = self.size.get(slot).filter(|s| s.0 != 0)?;
        let (x, y) = (
            (slot as u32 % GRID * SLOT) as f32,
            (slot as u32 / GRID * SLOT) as f32,
        );
        let s = SHEET as f32;
        Some([
            (x + 0.5) / s,
            (y + 0.5) / s,
            (x + w as f32 - 0.5) / s,
            (y + h as f32 - 0.5) / s,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn images_land_in_their_cell_and_cells_are_reused() {
        let mut im = Images::default();
        let a = im.add(2, 2, &[1; 16]).unwrap();
        let b = im.add(1, 1, &[2; 4]).unwrap();
        assert_eq!((a, b, im.version), (0, 1, 2));
        assert_eq!(im.rgba[0..8], [1; 8]); // row 0 of cell 0
        let row1 = (SHEET * 4) as usize; // row 1 of the sheet: still cell 0
        assert_eq!(im.rgba[row1..row1 + 8], [1; 8]);
        assert_eq!(im.rgba[(SLOT * 4) as usize..][..4], [2; 4]); // cell 1 starts SLOT px in
        im.remove(a);
        assert!(im.rgba[0..8] == [0; 8] && im.uv(a).is_none() && im.uv(b).is_some());
        assert_eq!(im.add(1, 1, &[3; 4]), Ok(0));
        // bad sizes
        assert!(im.add(0, 1, &[]).is_err() && im.add(129, 1, &[0; 516]).is_err());
        assert!(im.add(2, 2, &[0; 15]).is_err());
        let [u0, _, u1, _] = im.uv(b).unwrap();
        assert!(u0 > SLOT as f32 / SHEET as f32 && u1 > u0 - 1.0);
        // full
        let mut full = Images::default();
        for _ in 0..64 {
            full.add(1, 1, &[0; 4]).unwrap();
        }
        assert!(full.add(1, 1, &[0; 4]).is_err());
    }
}
