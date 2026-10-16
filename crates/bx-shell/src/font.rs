//! MSDF text: glyph quads for the atlas made by tools/msdf (printable ASCII).
//! No layouting: `text` puts one glyph after the other on a single line, no kerning, no wrapping.
//! The quads go to gpu.rs as `[x, y, u, v]` vertices.
#![allow(dead_code)] // PX_RANGE is only checked by a test (the shader has its own copy)

use crate::gpu::Atlas;

pub struct Glyph {
    /// Advance in em.
    pub adv: f32,
    /// Quad (atlas rect incl. padding) in em from the origin on the baseline: left, bottom,
    /// right, top (y up).
    pub plane: [f32; 4],
    /// Atlas rect in px: x, y, w, h (zero-sized for blanks).
    pub rect: [u16; 4],
}

include!("../assets/font.rs"); // ATLAS_W, ATLAS_H, PX_RANGE, GLYPHS

/// The atlas for `gpu::render`. It holds distances, so it is sampled as UNORM, not sRGB.
pub fn atlas() -> Atlas {
    let rgba = include_bytes!("../assets/font.rgb")
        .chunks(3)
        .flat_map(|c| [c[0], c[1], c[2], 255])
        .collect();
    Atlas {
        w: ATLAS_W,
        h: ATLAS_H,
        rgba,
    }
}

/// Appends 6 vertices per char of `s`: it starts at `origin` (px from the top left of the `w` x `h`
/// px surface, on the baseline) and is `size` px per em. Chars outside ASCII draw as `?`.
pub fn text(
    out: &mut Vec<[f32; 4]>,
    s: &str,
    (mut x, y): (f32, f32),
    size: f32,
    (w, h): (u32, u32),
) {
    let v = |px: f32, py: f32, u: f32, v: f32| {
        [px / w as f32 * 2.0 - 1.0, py / h as f32 * 2.0 - 1.0, u, v]
    };
    for c in s.chars() {
        let g = GLYPHS
            .get((c as usize).wrapping_sub(32))
            .unwrap_or(&GLYPHS[(b'?' - 32) as usize]);
        let [l, b, r, t] = g.plane;
        let (x0, x1, y0, y1) = (x + l * size, x + r * size, y - t * size, y - b * size);
        let [ax, ay, aw, ah] = g.rect.map(f32::from);
        let (u0, u1, v0, v1) = (
            ax / ATLAS_W as f32,
            (ax + aw) / ATLAS_W as f32,
            ay / ATLAS_H as f32,
            (ay + ah) / ATLAS_H as f32,
        );
        out.extend([
            v(x0, y0, u0, v0),
            v(x1, y0, u1, v0),
            v(x0, y1, u0, v1),
            v(x1, y0, u1, v0),
            v(x1, y1, u1, v1),
            v(x0, y1, u0, v1),
        ]);
        x += g.adv * size;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_vertices_per_char_inside_the_atlas() {
        let mut o = vec![];
        text(&mut o, "Hi ~é", (10.0, 20.0), 16.0, (400, 300)); // é falls back to ?
        assert_eq!(o.len(), 30);
        assert!(
            o.iter()
                .all(|v| (0.0..=1.0).contains(&v[2]) && (0.0..=1.0).contains(&v[3]))
        );
    }

    #[test]
    fn shader_and_atlas_agree_on_the_range() {
        let wgsl = include_str!("shader.wgsl");
        assert!(wgsl.contains(&format!("PX_RANGE: f32 = {PX_RANGE:.1};")));
        assert_eq!(
            include_bytes!("../assets/font.rgb").len(),
            (ATLAS_W * ATLAS_H * 3) as usize
        );
    }
}
