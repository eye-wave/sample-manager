//! Makes shell/assets/font.rgb (an MSDF atlas, 3 bytes per pixel) and shell/assets/font.rs (the
//! glyph table) for printable ASCII, with msdfgen.
//!   usage (from shell/): cargo run -p msdf-atlas -- FONT.ttf   (or .otf, or .woff)
//! Every glyph is drawn at the same scale (`EM` px per em) into its own cell, the cells are packed
//! in rows. A cell is the glyph's bounding box plus `PAD` px, so the quads of font.rs cover whole
//! cells.
use msdfgen::{Bitmap, FillRule, FontExt, Framing, MsdfGeneratorConfig, Projection, Rgb, Vector2};
use std::fmt::Write as _;
use std::io::Read as _;

const EM: f64 = 32.0; // atlas px per em
const RANGE: f64 = 4.0; // distance range in px, 0.5 +- RANGE/2 px maps to 0..1 (shader.wgsl PX_RANGE)
const PAD: i32 = 3; // px around each glyph, >= RANGE/2
const ATLAS_W: usize = 256;
const CORNER_ANGLE: f64 = 3.0; // msdfgen's default: edges meeting at a sharper angle get colours

struct Cell {
    adv: f64,
    plane: [f64; 4],  // em: left, bottom, right, top
    rect: [usize; 4], // atlas px: x, y, w, h
    rgb: Vec<u8>,     // w * h * 3, top row first
}

/// A .woff is an sfnt (a .ttf) with zlib-compressed tables; this puts the sfnt back together.
fn unwoff(w: &[u8]) -> Vec<u8> {
    let be16 = |o: usize| u16::from_be_bytes([w[o], w[o + 1]]);
    let be32 = |o: usize| u32::from_be_bytes([w[o], w[o + 1], w[o + 2], w[o + 3]]) as usize;
    let n = be16(12) as usize; // tables
    let log = n.ilog2() as usize;
    let search = (1 << log) * 16;
    let mut out = w[4..8].to_vec(); // flavor: TrueType or CFF outlines
    for v in [n, search, log, n * 16 - search] {
        out.extend((v as u16).to_be_bytes());
    }
    let (mut body, tables) = (vec![], 12 + n * 16);
    for i in 0..n {
        let e = 44 + i * 20; // WOFF table directory: tag, offset, compressed, original, checksum
        let (offset, compressed, original) = (be32(e + 4), be32(e + 8), be32(e + 12));
        let raw = &w[offset..offset + compressed];
        let mut table = vec![];
        if compressed < original {
            flate2::read::ZlibDecoder::new(raw)
                .read_to_end(&mut table)
                .unwrap();
        } else {
            table.extend(raw);
        }
        out.extend(&w[e..e + 4]); // sfnt table record: tag, checksum, offset, length
        out.extend(&w[e + 16..e + 20]);
        out.extend(((tables + body.len()) as u32).to_be_bytes());
        out.extend((original as u32).to_be_bytes());
        body.extend(table);
        body.resize(body.len().next_multiple_of(4), 0);
    }
    out.extend(body);
    out
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: msdf-atlas FONT.ttf");
    let mut data = std::fs::read(&path).unwrap();
    assert!(
        !data.starts_with(b"wOF2"),
        "woff2 is not supported: use the .woff or the .ttf"
    );
    if data.starts_with(b"wOFF") {
        data = unwoff(&data);
    }
    let face = ttf_parser::Face::parse(&data, 0).unwrap();
    let k = EM / face.units_per_em() as f64;

    let (mut cells, mut x, mut y, mut row) = (vec![], 0, 0, 0);
    for cp in 32u8..=126 {
        let gid = face.glyph_index(cp as char).unwrap();
        let adv = face.glyph_hor_advance(gid).unwrap() as f64 / face.units_per_em() as f64;
        let Some(mut shape) = face.glyph_shape(gid) else {
            // blank (space): only the advance
            cells.push(Cell {
                adv,
                plane: [0.0; 4],
                rect: [0; 4],
                rgb: vec![],
            });
            continue;
        };
        shape.edge_coloring_simple(CORNER_ANGLE, 0);
        // the cell, in px around the glyph's origin (y up)
        let b = shape.get_bound();
        let gx0 = (b.left * k).floor() as i32 - PAD;
        let gy0 = (b.bottom * k).floor() as i32 - PAD;
        let gx1 = (b.right * k).ceil() as i32 + PAD;
        let gy1 = (b.top * k).ceil() as i32 + PAD;
        let (w, h) = ((gx1 - gx0) as usize, (gy1 - gy0) as usize);

        // pixel = k * (font unit + translate), so that cell pixel (0, 0) is its bottom left corner.
        // The range is in font units here, not px.
        let framing = Framing {
            projection: Projection::new(
                Vector2::new(k, k),
                Vector2::new(-gx0 as f64 / k, -gy0 as f64 / k),
            ),
            range: RANGE / k,
        };
        let config = MsdfGeneratorConfig::default();
        let mut bitmap = Bitmap::<Rgb<f32>>::new(w as u32, h as u32);
        shape.generate_msdf(&mut bitmap, &framing, &config);
        shape.correct_sign(&mut bitmap, &framing, FillRule::default());
        shape.correct_msdf_error(&mut bitmap, &framing, &config);

        // msdfgen's rows start at the bottom, the atlas' at the top
        bitmap.flip_y();
        let rgb = bitmap
            .pixels()
            .iter()
            .flat_map(|p| [p.r, p.g, p.b].map(|v| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8))
            .collect();
        if x + w > ATLAS_W {
            (x, y, row) = (0, y + row + 1, 0);
        }
        cells.push(Cell {
            adv,
            plane: [gx0, gy0, gx1, gy1].map(|v| v as f64 / EM),
            rect: [x, y, w, h],
            rgb,
        });
        (x, row) = (x + w + 1, row.max(h));
    }

    let atlas_h = y + row;
    let mut atlas = vec![0u8; ATLAS_W * atlas_h * 3];
    for c in &cells {
        let [cx, cy, w, h] = c.rect;
        for r in 0..h {
            let dst = ((cy + r) * ATLAS_W + cx) * 3;
            atlas[dst..dst + w * 3].copy_from_slice(&c.rgb[r * w * 3..(r + 1) * w * 3]);
        }
    }
    std::fs::write("assets/font.rgb", &atlas).unwrap();

    let name = std::path::Path::new(&path)
        .file_name()
        .unwrap()
        .to_string_lossy();
    let mut rs = format!("// generated by tools/msdf from {name}: do not edit\n");
    writeln!(
        rs,
        "pub const ATLAS_W: u32 = {ATLAS_W};\npub const ATLAS_H: u32 = {atlas_h};"
    )
    .unwrap();
    writeln!(rs, "pub const PX_RANGE: f32 = {RANGE:.1};").unwrap();
    writeln!(
        rs,
        "/// ASCII 32..=126\npub static GLYPHS: [Glyph; {}] = [",
        cells.len()
    )
    .unwrap();
    for c in &cells {
        let [l, b, r, t] = c.plane;
        writeln!(
            rs,
            "    Glyph {{ adv: {:.5}, plane: [{l:.5}, {b:.5}, {r:.5}, {t:.5}], rect: {:?} }},",
            c.adv, c.rect
        )
        .unwrap();
    }
    rs.push_str("];\n");
    std::fs::write("assets/font.rs", rs).unwrap();
    println!("atlas {ATLAS_W}x{atlas_h}, {} glyphs", cells.len());
}
