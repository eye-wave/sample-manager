// Two fragment entry points over one vertex layout (position in NDC, uv):
//   fs_flat  solid vertex colour: rects, icons
//   fs_text  MSDF glyphs: the median of the three channels is the signed distance to the edge
struct V {
    @builtin(position) p: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) c: vec4<f32>,
}

@vertex
fn vs(@location(0) p: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) c: vec4<f32>) -> V {
    // meshes are y-down (like SVG); WGSL's NDC is y-up
    return V(vec4(p.x, -p.y, 0.0, 1.0), uv, c);
}

@group(0) @binding(0) var atlas: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

// distance range of the atlas in atlas px; keep equal to PX_RANGE in tools/msdf
const PX_RANGE: f32 = 4.0;

@fragment
fn fs_flat(v: V) -> @location(0) vec4<f32> {
    return v.c;
}

@fragment
fn fs_text(v: V) -> @location(0) vec4<f32> {
    let s = textureSample(atlas, samp, v.uv).rgb;
    let sd = max(min(s.r, s.g), min(max(s.r, s.g), s.b)) - 0.5;
    // the range in screen px: how much one atlas px is magnified, so edges stay 1 px wide at any size
    let atlas_px = vec2<f32>(textureDimensions(atlas));
    let screen_range = max(0.5 * dot(vec2(PX_RANGE) / atlas_px, 1.0 / fwidth(v.uv)), 1.0);
    return vec4(v.c.rgb, v.c.a * clamp(sd * screen_range + 0.5, 0.0, 1.0));
}
