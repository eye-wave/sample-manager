// The shader for shell::gpu's mesh pass (see `gpu::Content::mesh_shader`): vertex colours, and one
// matrix that is rewritten every frame.
struct U {
    mvp: mat4x4<f32>,
}
@group(0) @binding(2) var<uniform> u: U;

struct V {
    @builtin(position) p: vec4<f32>,
    @location(0) c: vec3<f32>,
}

@vertex
fn vs(@location(0) p: vec3<f32>, @location(1) c: vec3<f32>) -> V {
    return V(u.mvp * vec4(p, 1.0), c);
}

@fragment
fn fs(v: V) -> @location(0) vec4<f32> {
    return vec4(v.c, 1.0);
}
