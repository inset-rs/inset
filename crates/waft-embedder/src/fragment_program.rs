//! Flutter counterpart: dart:ui `FragmentProgram`, and the engine's
//! `lib/ui/painting/fragment_program.cc` that reflects its uniforms.
//!
//! A program is WGSL in valo's form (`valo::RuntimeEffect`): one `shade` function, its uniforms
//! one struct, its images texture and sampler pairs. Flutter's API counts a program's floats
//! packed tightly in field order, as its GLSL declares them, while WGSL places each field at the
//! alignment its type asks: in a struct of a `vec4`, a `vec4`, a `vec2`, an `f32` and a `vec2`,
//! float 11 is at byte 48, not 44. The invariant a later editor must keep: a float index means
//! what it means in Flutter, and [`FragmentProgram::uniform_bytes`] is the one place that maps
//! it to WGSL's bytes, as Impeller's `padding_layout` does for its own backends.
//!
//! Every program made is listed for the hosts, which prepare each on their renderers before
//! their next draw ([`PreparedPrograms`]), as the engine posts `CacheRuntimeStage` to its raster
//! thread when a program loads.

use std::fmt;
use std::sync::{Arc, Mutex, Weak};

use crate::FragmentShader;
use crate::painting::valo;

/// An instance of [`FragmentProgram`] creates [`Shader`](crate::Shader) objects (as used by
/// [`Paint::shader`](crate::Paint)), through the [`FragmentShader`]s it hands out.
#[derive(Debug)]
pub struct FragmentProgram {
    effect: Arc<valo::RuntimeEffect>,
    /// The float uniforms in field order, each with its first index and its size in floats:
    /// Dart's `_uniformInfo` as `_getUniformFloatInfo` reads it.
    uniform_info: Vec<UniformFloatInfo>,
    /// The byte in the uniform struct of each float index: what Impeller's `padding_layout`
    /// spells out.
    float_offsets: Vec<u32>,
}

/// One float uniform's place among the tightly packed floats: Dart's `_UniformFloatInfo`, with
/// the name it is looked up by.
#[derive(Debug)]
struct UniformFloatInfo {
    name: String,
    index: usize,
    size: usize,
}

impl FragmentProgram {
    /// Creates a fragment program from WGSL in valo's form, a paint shader's: Flutter's
    /// `FragmentProgram.fromAsset`, for a shader built into the code that uses it, typically
    /// with `include_str!`.
    ///
    /// The constructed object should then be reused via the
    /// [`fragment_shader`](Self::fragment_shader) method to create shader objects that can be
    /// used by [`Paint::shader`](crate::Paint).
    ///
    /// # Errors
    ///
    /// Where naga rejects the WGSL, where it breaks valo's contract, or where a uniform is not
    /// one Flutter's API sets: a float, a float vector or a square float matrix, or a
    /// fixed-size array of one of those.
    pub fn from_wgsl(wgsl: &str) -> Result<Arc<FragmentProgram>, valo::EffectError> {
        let effect = valo::RuntimeEffect::shader(wgsl)?;
        let (uniform_info, float_offsets) = float_layout(effect.uniforms())?;
        list_for_the_hosts(&effect);
        Ok(Arc::new(FragmentProgram {
            effect,
            uniform_info,
            float_offsets,
        }))
    }

    /// Returns a fresh instance of [`FragmentShader`].
    pub fn fragment_shader(self: &Arc<Self>) -> FragmentShader {
        FragmentShader::new(Arc::clone(self))
    }

    pub(crate) fn effect(&self) -> &Arc<valo::RuntimeEffect> {
        &self.effect
    }

    /// Dart's `_uniformFloatCount`.
    pub(crate) fn uniform_float_count(&self) -> usize {
        self.float_offsets.len()
    }

    /// Dart's `_samplerCount`.
    pub(crate) fn sampler_count(&self) -> usize {
        self.effect.image_names().len()
    }

    /// Dart's `_hasUniform`.
    fn has_uniform(&self, name: &str) -> bool {
        self.uniform_info.iter().any(|entry| entry.name == name)
            || self.effect.image_names().iter().any(|entry| entry == name)
    }

    /// Dart's `_getImageSamplerIndex`.
    pub(crate) fn get_image_sampler_index(&self, name: &str) -> Result<usize, UniformError> {
        match self
            .effect
            .image_names()
            .iter()
            .position(|entry| entry == name)
        {
            Some(index) => Ok(index),
            None if self.has_uniform(name) => Err(UniformError::NotAnImageSampler {
                name: name.to_owned(),
            }),
            None => Err(UniformError::NoUniform {
                name: name.to_owned(),
            }),
        }
    }

    /// Dart's `_getUniformFloatInfo`: the first index of the uniform named `name` and its size,
    /// both in floats.
    pub(crate) fn get_uniform_float_info(
        &self,
        name: &str,
    ) -> Result<(usize, usize), UniformError> {
        self.uniform_info
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| (entry.index, entry.size))
            .ok_or_else(|| UniformError::NoUniform {
                name: name.to_owned(),
            })
    }

    /// `floats`, the program's tightly packed floats, laid out as WGSL lays out its uniform
    /// struct, the padding zero: Impeller's `RuntimeEffectContents::EmplaceUniform`.
    pub(crate) fn uniform_bytes(&self, floats: &[f32]) -> Vec<u8> {
        let mut bytes = vec![0; self.effect.uniforms().size as usize];
        for (value, &offset) in floats.iter().zip(&self.float_offsets) {
            let at = offset as usize;
            bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes
    }
}

/// Each float uniform of `uniforms` with its place among the tightly packed floats, and the byte
/// of each of those floats in the struct.
fn float_layout(
    uniforms: &valo::UniformLayout,
) -> Result<(Vec<UniformFloatInfo>, Vec<u32>), valo::EffectError> {
    let mut uniform_info = Vec::new();
    let mut float_offsets = Vec::new();
    for field in &uniforms.fields {
        let offsets = float_offsets_of(field)?;
        uniform_info.push(UniformFloatInfo {
            name: field.name.clone(),
            index: float_offsets.len(),
            size: offsets.len(),
        });
        float_offsets.extend(offsets);
    }
    Ok((uniform_info, float_offsets))
}

/// The byte of each of `field`'s floats in the uniform struct, in the order Flutter's tightly
/// packed indices count them: an array's elements one after the other, a matrix's columns.
fn float_offsets_of(field: &valo::UniformField) -> Result<Vec<u32>, valo::EffectError> {
    let element = element_float_offsets(field.ty).ok_or_else(|| {
        valo::EffectError::Contract(format!(
            "`{}` is not a uniform a fragment program sets: those are floats, float vectors, \
             square float matrices and fixed-size arrays of them",
            field.name
        ))
    })?;
    let (count, stride) = field
        .array
        .map_or((1, 0), |array| (array.count, array.stride));
    Ok((0..count)
        .flat_map(|item| {
            element
                .iter()
                .map(move |offset| field.offset + item * stride + offset)
        })
        .collect())
}

/// Where each float of one value of `ty` lies from the value's start, as WGSL lays the type out
/// in a uniform struct: a matrix column by column, each column a vector aligned as one.
fn element_float_offsets(ty: valo::UniformType) -> Option<&'static [u32]> {
    use valo::UniformType;
    match ty {
        UniformType::F32 => Some(&[0]),
        UniformType::Vec2F => Some(&[0, 4]),
        UniformType::Vec3F => Some(&[0, 4, 8]),
        UniformType::Vec4F => Some(&[0, 4, 8, 12]),
        UniformType::Mat2x2F => Some(&[0, 4, 8, 12]),
        UniformType::Mat3x3F => Some(&[0, 4, 8, 16, 20, 24, 32, 36, 40]),
        UniformType::Mat4x4F => {
            Some(&[0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60])
        }
        UniformType::I32
        | UniformType::U32
        | UniformType::Vec2I
        | UniformType::Vec2U
        | UniformType::Other { .. } => None,
    }
}

/// Why a uniform could not be looked up: the `ArgumentError`s and `IndexError`s Dart throws, each
/// displayed with Dart's message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UniformError {
    /// The program has no uniform of that name.
    NoUniform { name: String },
    /// The uniform has another number of floats than the slot asked for.
    WrongSize {
        name: String,
        size: usize,
        expected: usize,
    },
    /// A float slot past the uniform's floats.
    IndexOutOfBounds { name: String, index: usize },
    /// An array asked of a uniform whose floats do not divide into its elements.
    NotAMultiple {
        name: String,
        size: usize,
        element_size: usize,
    },
    /// An image sampler asked of a float uniform.
    NotAnImageSampler { name: String },
}

impl fmt::Display for UniformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UniformError::NoUniform { name } => write!(f, "No uniform named \"{name}\"."),
            UniformError::WrongSize {
                name,
                size,
                expected,
            } => write!(f, "Uniform `{name}` has size {size}, not size {expected}."),
            UniformError::IndexOutOfBounds { name, index } => {
                write!(f, "Index `{index}` out of bounds for `{name}`.")
            }
            UniformError::NotAMultiple {
                name,
                size,
                element_size,
            } => write!(
                f,
                "Uniform size ({size}) for \"{name}\" is not a multiple of {element_size}."
            ),
            UniformError::NotAnImageSampler { name } => {
                write!(f, "Uniform \"{name}\" is not an image sampler.")
            }
        }
    }
}

impl std::error::Error for UniformError {}

/// The effects of every program made in this process, the first made first.
static PROGRAMS_MADE: Mutex<Vec<Weak<valo::RuntimeEffect>>> = Mutex::new(Vec::new());

/// Lists `effect` among the programs made, for each host's renderers to prepare.
fn list_for_the_hosts(effect: &Arc<valo::RuntimeEffect>) {
    PROGRAMS_MADE
        .lock()
        .expect("the list of programs made")
        .push(Arc::downgrade(effect));
}

/// Which of the process's fragment programs one renderer has prepared: what a host keeps beside
/// each renderer it draws with, to prepare every program made on it before its next draw, as the
/// engine runs `CacheRuntimeStage` on its raster thread when a program loads.
#[derive(Debug, Default)]
pub struct PreparedPrograms {
    /// How many of the programs made the renderer has been handed.
    handed: usize,
}

impl PreparedPrograms {
    /// A renderer that has prepared none.
    pub fn new() -> PreparedPrograms {
        PreparedPrograms::default()
    }

    /// Hands `prepare` the effect of each program made since the last call, the first made
    /// first, leaving out those no one holds any longer: a host prepares each with
    /// `valo::Context::prepare_runtime_effect` for the format it draws into.
    pub fn prepare_new(&mut self, mut prepare: impl FnMut(&Arc<valo::RuntimeEffect>)) {
        let made: Vec<Arc<valo::RuntimeEffect>> = {
            let programs = PROGRAMS_MADE.lock().expect("the list of programs made");
            let new = programs.get(self.handed..).unwrap_or_default();
            self.handed = programs.len();
            new.iter().filter_map(Weak::upgrade).collect()
        };
        for effect in &made {
            prepare(effect);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Material's ink sparkle's uniforms, in its GLSL file's order.
    const INK_SPARKLE_UNIFORMS: &str = "
struct Uniforms {
    u_color: vec4<f32>,
    u_composite_1: vec4<f32>,
    u_center: vec2<f32>,
    u_max_radius: f32,
    u_resolution_scale: vec2<f32>,
    u_noise_scale: vec2<f32>,
    u_noise_phase: f32,
    u_circle1: vec2<f32>,
    u_circle2: vec2<f32>,
    u_circle3: vec2<f32>,
    u_rotation1: vec2<f32>,
    u_rotation2: vec2<f32>,
    u_rotation3: vec2<f32>,
}
@group(2) @binding(0) var<uniform> u: Uniforms;

fn shade(position: vec2<f32>) -> vec4<f32> {
    return u.u_color * u.u_composite_1.x + vec4<f32>(u.u_resolution_scale, u.u_rotation3);
}
";

    /// Flutter's float 11, the first of `u_resolution_scale`, follows `u_max_radius` at float
    /// 10, but WGSL aligns a `vec2` to eight bytes: it is at byte 48, not 44.
    #[test]
    fn a_float_index_lands_where_wgsl_lays_the_uniform_out() {
        let program = FragmentProgram::from_wgsl(INK_SPARKLE_UNIFORMS).expect("valid program");
        assert_eq!(program.uniform_float_count(), 28);
        assert_eq!(
            program.get_uniform_float_info("u_resolution_scale"),
            Ok((11, 2))
        );
        let floats: Vec<f32> = (0..28).map(|index| index as f32).collect();
        let bytes = program.uniform_bytes(&floats);
        let float_at = |byte: usize| f32::from_le_bytes(bytes[byte..byte + 4].try_into().unwrap());
        assert_eq!(float_at(40), 10.0, "u_max_radius");
        assert_eq!(float_at(44), 0.0, "padding");
        assert_eq!(float_at(48), 11.0, "u_resolution_scale.x");
        assert_eq!(float_at(64), 15.0, "u_noise_phase");
        assert_eq!(float_at(72), 16.0, "u_circle1.x");
        assert_eq!(float_at(116), 27.0, "u_rotation3.y");
        assert_eq!(bytes.len(), 128);
    }

    /// A `mat3x3` keeps each column in sixteen bytes, and an array its stride: Impeller's
    /// padding layout of a GLSL `vec3` array.
    #[test]
    fn a_matrix_and_an_array_skip_their_padding() {
        let program = FragmentProgram::from_wgsl(
            "struct U { m: mat3x3<f32>, colors: array<vec3<f32>, 2>, last: f32 }
             @group(2) @binding(0) var<uniform> u: U;
             fn shade(position: vec2<f32>) -> vec4<f32> {
                 return vec4<f32>(u.m[0] + u.colors[1], u.last);
             }",
        )
        .expect("valid program");
        assert_eq!(program.get_uniform_float_info("m"), Ok((0, 9)));
        assert_eq!(program.get_uniform_float_info("colors"), Ok((9, 6)));
        assert_eq!(program.get_uniform_float_info("last"), Ok((15, 1)));
        assert_eq!(
            program.float_offsets,
            [0, 4, 8, 16, 20, 24, 32, 36, 40, 48, 52, 56, 64, 68, 72, 80]
        );
    }

    #[test]
    fn an_integer_uniform_is_not_one_a_program_sets() {
        let error = FragmentProgram::from_wgsl(
            "struct U { count: i32 }
             @group(2) @binding(0) var<uniform> u: U;
             fn shade(position: vec2<f32>) -> vec4<f32> { return vec4<f32>(f32(u.count)); }",
        )
        .expect_err("an integer uniform");
        assert!(
            matches!(&error, valo::EffectError::Contract(message) if message.contains("`count`")),
            "{error}"
        );
    }

    /// Dart's `fromAsset throws an exception on invalid data`.
    #[test]
    fn invalid_wgsl_is_refused_with_valos_message() {
        assert!(matches!(
            FragmentProgram::from_wgsl("fn shade(position: vec2<f32>) -> vec4<f32> { oops }"),
            Err(valo::EffectError::Parse(_))
        ));
    }

    #[test]
    fn a_renderer_is_handed_each_program_once() {
        let mut prepared = PreparedPrograms::new();
        prepared.prepare_new(|_| {});
        let program = FragmentProgram::from_wgsl(
            "fn shade(position: vec2<f32>) -> vec4<f32> { return vec4<f32>(1.0); }",
        )
        .expect("valid program");
        let mut handed = Vec::new();
        prepared.prepare_new(|effect| handed.push(effect.id()));
        assert!(handed.contains(&program.effect().id()));
        let mut again = Vec::new();
        prepared.prepare_new(|effect| again.push(effect.id()));
        assert!(!again.contains(&program.effect().id()));
    }
}
