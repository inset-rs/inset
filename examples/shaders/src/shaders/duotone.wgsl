// A child in two colours, dark where it is dark and light where it is light, as an
// `ImageFilter::shader` that `ImageFiltered` applies: the input is the child's layer.

struct Uniforms {
    u_size: vec2<f32>,
    u_dark: vec4<f32>,
    u_light: vec4<f32>,
}
@group(2) @binding(0) var<uniform> u: Uniforms;
@group(1) @binding(0) var u_texture: texture_2d<f32>;
@group(1) @binding(1) var u_texture_sampler: sampler;

fn shade(position: vec2<f32>) -> vec4<f32> {
    let color = textureSampleLevel(u_texture, u_texture_sampler, position / u.u_size, 0.0);
    if color.a <= 0.0 {
        return vec4<f32>(0.0);
    }
    let luminance = dot(color.rgb / color.a, vec3<f32>(0.2126, 0.7152, 0.0722));
    return mix(u.u_dark, u.u_light, luminance) * color.a;
}
