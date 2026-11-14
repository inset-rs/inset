// What lies behind, in square cells, as an `ImageFilter::shader` over a backdrop: by Flutter's
// convention the input is the first image and its size in pixels the first uniform, which the
// renderer sets, and position is in the input's pixels.

struct Uniforms {
    u_size: vec2<f32>,
    u_cell: f32,
}
@group(2) @binding(0) var<uniform> u: Uniforms;
@group(1) @binding(0) var u_texture: texture_2d<f32>;
@group(1) @binding(1) var u_texture_sampler: sampler;

fn shade(position: vec2<f32>) -> vec4<f32> {
    let cell_centre = (floor(position / u.u_cell) + 0.5) * u.u_cell;
    return textureSampleLevel(u_texture, u_texture_sampler, cell_centre / u.u_size, 0.0);
}
