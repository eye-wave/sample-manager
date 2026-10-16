//! The spinning rainbow cube: its mesh, and the matrix that spins it and keeps it centered
//! between the title and the icon row. The shader is cube.wgsl; shell::gpu draws it.
use bx_shader::include_shader;

use crate::icons::{TITLE_BAND, icons_top};

/// A 4x4 matrix, column-major (what WGSL's `mat4x4` is in memory).
pub type Mat = [f32; 16];

/// SPIR-V of cube.wgsl, made by build.rs.
pub const SHADER: &[u8] = include_shader!("cube.wgsl");

/// Full saturation and value: `h` in 0..1 goes once through the rainbow.
fn hue(h: f32) -> [f32; 3] {
    let f = |n: f32| {
        let k = (n + h * 6.0) % 6.0;
        1.0 - k.min(4.0 - k).clamp(0.0, 1.0)
    };
    [f(5.0), f(3.0), f(1.0)]
}

/// The cube of half side 1 as 12 triangles, `[x, y, z, r, g, b]`. Every corner has a hue of its
/// own, so the faces blend through the rainbow.
pub fn mesh() -> Vec<[f32; 6]> {
    let mut out = vec![];
    for face in 0..6 {
        // the axis the face is perpendicular to, and the two that span it
        let (n, u, v) = (face % 3, (face + 1) % 3, (face + 2) % 3);
        let side = if face < 3 { 1.0 } else { -1.0 };
        let corner = |a: f32, b: f32| {
            let mut p = [0.0; 3];
            (p[n], p[u], p[v]) = (side, a, b);
            let bit = |i: usize| (p[i] > 0.0) as u32;
            let [r, g, b] = hue((bit(0) + 2 * bit(1) + 4 * bit(2)) as f32 / 8.0);
            [p[0], p[1], p[2], r, g, b]
        };
        let q = [
            corner(-1.0, -1.0),
            corner(1.0, -1.0),
            corner(1.0, 1.0),
            corner(-1.0, 1.0),
        ];
        out.extend([q[0], q[1], q[2], q[0], q[2], q[3]]);
    }
    out
}

fn mul(a: &Mat, b: &Mat) -> Mat {
    let mut o = [0.0; 16];
    for c in 0..4 {
        for r in 0..4 {
            o[c * 4 + r] = (0..4).map(|k| a[k * 4 + r] * b[c * 4 + k]).sum();
        }
    }
    o
}

fn rot_x(a: f32) -> Mat {
    let (s, c) = a.sin_cos();
    [1., 0., 0., 0., 0., c, s, 0., 0., -s, c, 0., 0., 0., 0., 1.]
}

fn rot_y(a: f32) -> Mat {
    let (s, c) = a.sin_cos();
    [c, 0., -s, 0., 0., 1., 0., 0., s, 0., c, 0., 0., 0., 0., 1.]
}

/// The matrix for the cube in a `w` x `h` px window, `t` seconds after the start: model, view and
/// projection (WGSL conventions: right-handed, y up, z 0..1) and a shift that moves the cube to
/// the middle of the free space between the title and the icons.
pub fn mvp(w: u32, h: u32, t: f32) -> Mat {
    let (w, h) = (w as f32, h as f32);
    let (top, bottom) = (TITLE_BAND, icons_top(h));
    let (cx, cy) = (w / 2.0, (top + bottom) / 2.0);
    let (focal, dist, near, far) = (1.0 / 25f32.to_radians().tan(), 3.0, 0.1, 10.0);

    // half a side in px; the corners reach up to 1.8 times as far (perspective included), which
    // still fits the free height at this size
    let side_px = 0.24 * (bottom - top);
    // an object of half size `s` at distance `dist` is `s * focal / dist * h / 2` px
    let s = side_px * 2.0 * dist / (focal * h);
    let scale = [s, 0., 0., 0., 0., s, 0., 0., 0., 0., s, 0., 0., 0., 0., 1.];
    let model = mul(&rot_x(t * 0.6 + 0.4), &mul(&rot_y(t * 0.9), &scale));
    let view = [
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., -dist, 1.,
    ];
    let proj = [
        focal / (w / h),
        0.,
        0.,
        0.,
        0.,
        focal,
        0.,
        0.,
        0.,
        0.,
        far / (near - far),
        -1.,
        0.,
        0.,
        near * far / (near - far),
        0.,
    ];
    // x' = x + shift * w, so that after the division by w the cube is `shift` further (in NDC)
    let mut shift = [
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
    ];
    (shift[12], shift[13]) = (cx / w * 2.0 - 1.0, 1.0 - cy / h * 2.0);
    mul(&shift, &mul(&proj, &mul(&view, &model)))
}
