//! The app's fragment programs, each made once from WGSL built into the binary, and what sets
//! their uniforms.

use std::sync::{Arc, LazyLock};

use waft_embedder::{
    Canvas, Color, FilterQuality, FragmentProgram, FragmentShader, Image, ImageFilter,
    ImageSamplerSlot, Paint, Shader, Size, UniformFloatSlot, UniformVec2Slot, UniformVec4Slot,
    valo,
};

static RINGS: LazyLock<Arc<FragmentProgram>> = LazyLock::new(|| {
    FragmentProgram::from_wgsl(include_str!("shaders/rings.wgsl")).expect("a valid program")
});

static PIXELATE: LazyLock<Arc<FragmentProgram>> = LazyLock::new(|| {
    FragmentProgram::from_wgsl(include_str!("shaders/pixelate.wgsl")).expect("a valid program")
});

static DUOTONE: LazyLock<Arc<FragmentProgram>> = LazyLock::new(|| {
    FragmentProgram::from_wgsl(include_str!("shaders/duotone.wgsl")).expect("a valid program")
});

/// The rings program's shader with its uniforms' slots, looked up once and kept beside it.
pub struct Rings {
    shader: FragmentShader,
    size: UniformVec2Slot,
    ring_width: UniformFloatSlot,
    color: UniformVec4Slot,
    pattern: ImageSamplerSlot,
}

impl Rings {
    pub fn new() -> Rings {
        let shader = RINGS.fragment_shader();
        Rings {
            size: shader
                .get_uniform_vec2("u_size")
                .expect("rings.wgsl's size"),
            ring_width: shader
                .get_uniform_float("u_ring_width", None)
                .expect("rings.wgsl's ring width"),
            color: shader
                .get_uniform_vec4("u_color")
                .expect("rings.wgsl's colour"),
            pattern: shader
                .get_image_sampler("u_pattern")
                .expect("rings.wgsl's pattern"),
            shader,
        }
    }

    /// Fills a box of `size` with rings over `pattern`, stretched to the box.
    pub fn paint(&mut self, canvas: &mut Canvas, size: Size, pattern: &Image) {
        let shader = &mut self.shader;
        self.size.set(shader, size.width(), size.height());
        self.ring_width.set(shader, 48.0);
        let color = Color::new(0xFF2A9D8F);
        self.color.set(shader, color.r, color.g, color.b, color.a);
        self.pattern.set(shader, pattern);
        let whole = valo::Rect::new(0.0, 0.0, size.width() as f32, size.height() as f32);
        canvas.draw_rect(whole, &Paint::from_shader(Shader::from(&*shader)));
    }
}

impl Default for Rings {
    fn default() -> Rings {
        Rings::new()
    }
}

/// What lies behind, in square cells `cell` pixels a side.
pub fn pixelate(cell: f64) -> ImageFilter {
    let mut shader = PIXELATE.fragment_shader();
    shader
        .get_uniform_float("u_cell", None)
        .expect("pixelate.wgsl's cell")
        .set(&mut shader, cell);
    ImageFilter::shader(&shader, FilterQuality::None)
}

/// The child in `dark` where it is dark and `light` where it is light.
pub fn duotone(dark: Color, light: Color) -> ImageFilter {
    let mut shader = DUOTONE.fragment_shader();
    for (name, color) in [("u_dark", dark), ("u_light", light)] {
        shader
            .get_uniform_vec4(name)
            .expect("duotone.wgsl's colours")
            .set(&mut shader, color.r, color.g, color.b, color.a);
    }
    ImageFilter::shader(&shader, FilterQuality::Low)
}
