struct V {
    @builtin(position) p: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs(@location(0) p: vec2<f32>, @location(1) uv: vec2<f32>) -> V {
    return V(vec4(p.x, -p.y, 0.0, 1.0), uv);
}

@group(0) @binding(0) var atlas: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

const COLOR: vec3<f32> = vec3(0.9, 0.9, 0.95);
const PX_RANGE: f32 = 4.0;

@fragment
fn fs_flat() -> @location(0) vec4<f32> {
    return vec4(COLOR, 1.0);
}

@fragment
fn fs_text(v: V) -> @location(0) vec4<f32> {
    let s = textureSample(atlas, samp, v.uv).rgb;
    let sd = max(min(s.r, s.g), min(max(s.r, s.g), s.b)) - 0.5;
    let atlas_px = vec2<f32>(textureDimensions(atlas));
    let screen_range = max(0.5 * dot(vec2(PX_RANGE) / atlas_px, 1.0 / fwidth(v.uv)), 1.0);

    return vec4(COLOR, clamp(sd * screen_range + 0.5, 0.0, 1.0));
}
