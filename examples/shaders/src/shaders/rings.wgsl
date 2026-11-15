// Rings over a pattern image, painted with `Paint::from_shader`: a paint shader in valo's form,
// whose position is the painted box's own coordinates and whose colour is premultiplied.

struct Uniforms {
    u_size: vec2<f32>,
    u_ring_width: f32,
    u_color: vec4<f32>,
}
@group(2) @binding(0) var<uniform> u: Uniforms;
@group(1) @binding(0) var u_pattern: texture_2d<f32>;
@group(1) @binding(1) var u_pattern_sampler: sampler;

const TAU: f32 = 6.2831853;

fn shade(position: vec2<f32>) -> vec4<f32> {
    let centre = u.u_size * 0.5;
    let ring = 0.5 + 0.5 * cos(distance(position, centre) / u.u_ring_width * TAU);
    let pattern = textureSampleLevel(u_pattern, u_pattern_sampler, position / u.u_size, 0.0);
    return mix(pattern, u.u_color, ring * 0.6);
}
