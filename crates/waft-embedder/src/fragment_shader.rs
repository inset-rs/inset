//! Flutter counterpart: dart:ui `FragmentShader`, its uniform slots and `UniformArray`, and the
//! engine's `lib/ui/painting/fragment_shader.cc` (`ReusableFragmentShader`) that holds a shader's
//! floats and images.
//!
//! A shader keeps Flutter's tightly packed floats; a slot is a `Copy` handle on one float index,
//! looked up by name once and kept beside the shader, whose `set` takes the shader it writes. The
//! invariant a later editor must keep: a [`Shader`] or a filter made from a shader copies its
//! floats and images at that moment, as the engine copies its uniform data into the display list,
//! so the shader can change for the next frame without changing what was drawn.

use std::ops::Index;
use std::sync::Arc;

use crate::fragment_program::{FragmentProgram, UniformError};
use crate::painting::{FilterQuality, Image, Shader, valo};

/// A binding to a uniform of type float. Calling [`set`](Self::set) on this object updates a
/// float uniform's value.
///
/// ```ignore
/// shader.get_uniform_float("uColor", Some(0))?.set(&mut shader, 1.0);
/// ```
///
/// See also: [`FragmentShader::get_uniform_float`], how [`UniformFloatSlot`] instances are
/// acquired.
#[derive(Clone, Copy, Debug)]
pub struct UniformFloatSlot {
    /// The effect of the program the slot was looked up in, which a shader it writes is of.
    program: u64,
    shader_index: usize,
    index: usize,
}

impl UniformFloatSlot {
    /// Set the float value of the bound uniform in `shader`, one of the program's.
    pub fn set(self, shader: &mut FragmentShader, val: f64) {
        debug_assert_eq!(
            self.program,
            shader.program.effect().id(),
            "a slot sets a shader of the program it was looked up in"
        );
        shader.set_float_at(self.shader_index, val);
    }

    /// VisibleForTesting: This is the index of this uniform in the shader's float uniform buffer.
    pub fn shader_index(self) -> usize {
        self.shader_index
    }

    /// The offset into the bound uniform. For example, 1 for `.y` or 2 for `.b`.
    pub fn index(self) -> usize {
        self.index
    }
}

/// A binding to a uniform of type vec2. Calling [`set`](Self::set) on this object updates the
/// uniform's value.
///
/// See also: [`FragmentShader::get_uniform_vec2`].
#[derive(Clone, Copy, Debug)]
pub struct UniformVec2Slot {
    x_slot: UniformFloatSlot,
    y_slot: UniformFloatSlot,
}

impl UniformVec2Slot {
    /// Set the float value of the bound uniform.
    pub fn set(self, shader: &mut FragmentShader, x: f64, y: f64) {
        self.x_slot.set(shader, x);
        self.y_slot.set(shader, y);
    }
}

/// A binding to a uniform of type vec3. Calling [`set`](Self::set) on this object updates the
/// uniform's value.
///
/// See also: [`FragmentShader::get_uniform_vec3`].
#[derive(Clone, Copy, Debug)]
pub struct UniformVec3Slot {
    x_slot: UniformFloatSlot,
    y_slot: UniformFloatSlot,
    z_slot: UniformFloatSlot,
}

impl UniformVec3Slot {
    /// Set the float value of the bound uniform.
    pub fn set(self, shader: &mut FragmentShader, x: f64, y: f64, z: f64) {
        self.x_slot.set(shader, x);
        self.y_slot.set(shader, y);
        self.z_slot.set(shader, z);
    }
}

/// A binding to a uniform of type vec4. Calling [`set`](Self::set) on this object updates the
/// uniform's value.
///
/// See also: [`FragmentShader::get_uniform_vec4`].
#[derive(Clone, Copy, Debug)]
pub struct UniformVec4Slot {
    x_slot: UniformFloatSlot,
    y_slot: UniformFloatSlot,
    z_slot: UniformFloatSlot,
    w_slot: UniformFloatSlot,
}

impl UniformVec4Slot {
    /// Set the float value of the bound uniform.
    pub fn set(self, shader: &mut FragmentShader, x: f64, y: f64, z: f64, w: f64) {
        self.x_slot.set(shader, x);
        self.y_slot.set(shader, y);
        self.z_slot.set(shader, z);
        self.w_slot.set(shader, w);
    }
}

/// A binding to a uniform of type mat2. Calling [`set`](Self::set) on this object updates the
/// uniform's value.
///
/// See also: [`FragmentShader::get_uniform_mat2`].
#[derive(Clone, Copy, Debug)]
pub struct UniformMat2Slot {
    /// `_m00`, `_m01`, `_m10`, `_m11`.
    slots: [UniformFloatSlot; 4],
}

impl UniformMat2Slot {
    /// Set the float value of the matrix in row-major order.
    pub fn set(self, shader: &mut FragmentShader, m00: f64, m01: f64, m10: f64, m11: f64) {
        set_all(&self.slots, shader, [m00, m01, m10, m11]);
    }
}

/// A binding to a uniform of type mat3. Calling [`set`](Self::set) on this object updates the
/// uniform's value.
///
/// See also: [`FragmentShader::get_uniform_mat3`].
#[derive(Clone, Copy, Debug)]
pub struct UniformMat3Slot {
    /// `_m00` to `_m22`, row after row.
    slots: [UniformFloatSlot; 9],
}

impl UniformMat3Slot {
    /// Set the float value of the matrix in row-major order.
    #[expect(
        clippy::too_many_arguments,
        reason = "Dart's nine entries, one argument each"
    )]
    pub fn set(
        self,
        shader: &mut FragmentShader,
        m00: f64,
        m01: f64,
        m02: f64,
        m10: f64,
        m11: f64,
        m12: f64,
        m20: f64,
        m21: f64,
        m22: f64,
    ) {
        set_all(
            &self.slots,
            shader,
            [m00, m01, m02, m10, m11, m12, m20, m21, m22],
        );
    }
}

/// A binding to a uniform of type mat4. Calling [`set`](Self::set) on this object updates the
/// uniform's value.
///
/// See also: [`FragmentShader::get_uniform_mat4`].
#[derive(Clone, Copy, Debug)]
pub struct UniformMat4Slot {
    /// `_m00` to `_m33`, row after row.
    slots: [UniformFloatSlot; 16],
}

impl UniformMat4Slot {
    /// Set the float value of the matrix in row-major order.
    #[expect(
        clippy::too_many_arguments,
        reason = "Dart's sixteen entries, one argument each"
    )]
    pub fn set(
        self,
        shader: &mut FragmentShader,
        m00: f64,
        m01: f64,
        m02: f64,
        m03: f64,
        m10: f64,
        m11: f64,
        m12: f64,
        m13: f64,
        m20: f64,
        m21: f64,
        m22: f64,
        m23: f64,
        m30: f64,
        m31: f64,
        m32: f64,
        m33: f64,
    ) {
        set_all(
            &self.slots,
            shader,
            [
                m00, m01, m02, m03, m10, m11, m12, m13, m20, m21, m22, m23, m30, m31, m32, m33,
            ],
        );
    }
}

/// Sets each of `slots` to the value beside it, in order.
fn set_all<const N: usize>(
    slots: &[UniformFloatSlot; N],
    shader: &mut FragmentShader,
    values: [f64; N],
) {
    for (slot, value) in slots.iter().zip(values) {
        slot.set(shader, value);
    }
}

/// An array of bindings to uniforms of the same type `T`. Access elements via `[]` and set them
/// individually.
///
/// ```ignore
/// let colors = shader.get_uniform_vec4_array("uColorArray")?;
/// colors[0].set(&mut shader, 1.0, 0.0, 1.0, 0.3);
/// ```
///
/// See also: [`FragmentShader::get_uniform_float_array`], how `UniformArray`s are acquired.
#[derive(Clone, Debug)]
pub struct UniformArray<T> {
    elements: Vec<T>,
}

impl<T> UniformArray<T> {
    /// The number of Uniforms in the UniformArray.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Whether the array has no elements.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

/// Access an element of the UniformArray.
impl<T> Index<usize> for UniformArray<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        &self.elements[index]
    }
}

/// A binding to a shader's image sampler. Calling [`set`](Self::set) on this object updates a
/// sampler's bound image.
#[derive(Clone, Copy, Debug)]
pub struct ImageSamplerSlot {
    /// The effect of the program the slot was looked up in.
    program: u64,
    shader_index: usize,
}

impl ImageSamplerSlot {
    /// Set the [`Image`] value for the bound sampler associated with this slot, sampled at
    /// [`FilterQuality::None`].
    pub fn set(self, shader: &mut FragmentShader, val: &Image) {
        debug_assert_eq!(
            self.program,
            shader.program.effect().id(),
            "a slot sets a shader of the program it was looked up in"
        );
        shader.set_image_sampler(self.shader_index, val, FilterQuality::None);
    }

    /// VisibleForTesting: This is the index one would use with
    /// [`FragmentShader::set_image_sampler`] for this sampler.
    pub fn shader_index(self) -> usize {
        self.shader_index
    }
}

/// A [`Shader`] generated from a [`FragmentProgram`].
///
/// Instances of this class can be obtained from the [`FragmentProgram::fragment_shader`] method.
/// The float uniforms list is initialized to the size expected by the shader and is zero-filled.
/// Uniforms of float type can then be set by name with
/// [`get_uniform_float`](Self::get_uniform_float), [`get_uniform_vec2`](Self::get_uniform_vec2),
/// [`get_uniform_vec3`](Self::get_uniform_vec3), [`get_uniform_vec4`](Self::get_uniform_vec4),
/// and related methods. Sampler uniforms are set by calling
/// [`set_image_sampler`](Self::set_image_sampler).
///
/// A [`FragmentShader`] can be re-used, and this is an efficient way to avoid allocating and
/// re-initializing the uniform buffer and samplers. However, if two [`FragmentShader`] objects
/// with different float uniforms or samplers are required to exist simultaneously, they must be
/// obtained from two different calls to [`FragmentProgram::fragment_shader`].
///
/// A paint draws with the shader as it is when the paint is made, `Shader::from(&shader)`.
#[derive(Debug)]
pub struct FragmentShader {
    program: Arc<FragmentProgram>,
    floats: Vec<f32>,
    /// Each image sampler's image and how it is sampled; `None` until it is set.
    samplers: Vec<Option<valo::EffectImage>>,
}

impl FragmentShader {
    pub(crate) fn new(program: Arc<FragmentProgram>) -> FragmentShader {
        FragmentShader {
            floats: vec![0.0; program.uniform_float_count()],
            samplers: vec![None; program.sampler_count()],
            program,
        }
    }

    /// Sets the float uniform at `index` to `value`.
    ///
    /// Deprecated: Prefer setting uniforms by name with
    /// [`get_uniform_float`](Self::get_uniform_float),
    /// [`get_uniform_vec2`](Self::get_uniform_vec2),
    /// [`get_uniform_vec3`](Self::get_uniform_vec3),
    /// [`get_uniform_vec4`](Self::get_uniform_vec4),
    /// [`get_uniform_mat2`](Self::get_uniform_mat2),
    /// [`get_uniform_mat3`](Self::get_uniform_mat3),
    /// [`get_uniform_mat4`](Self::get_uniform_mat4), or their array counterparts such as
    /// [`get_uniform_float_array`](Self::get_uniform_float_array). Those methods don't require
    /// manually computing the index of each uniform and are robust to changes in the order or
    /// layout of uniforms in the fragment program.
    ///
    /// The index for each uniform is determined by the order of the uniforms as defined in the
    /// fragment program's uniform struct, ignoring any images, each counted by its floats packed
    /// tightly. For data types that are composed of multiple floats such as a `vec4`, more than
    /// one call to [`set_float`](Self::set_float) is required.
    ///
    /// For example, given the following uniforms in a fragment program:
    ///
    /// ```wgsl
    /// struct Uniforms { uScale: f32, uMagnitude: vec2<f32>, uColor: vec4<f32> }
    /// ```
    ///
    /// `uScale` is at index 0, `uMagnitude` occupies indices 1 and 2, and `uColor` occupies
    /// indices 3 through 6, wherever WGSL places them in the struct's bytes.
    ///
    /// Any float uniforms that are left uninitialized will default to `0`.
    ///
    /// # Panics
    ///
    /// Where `index` is past the program's floats.
    #[deprecated(
        note = "Use get_uniform_float, get_uniform_vec2, get_uniform_vec3, get_uniform_vec4, or \
                a related method to set uniforms by name instead. This feature was deprecated \
                after v3.51.0-0.1.pre."
    )]
    pub fn set_float(&mut self, index: usize, value: f64) {
        self.set_float_at(index, value);
    }

    /// Dart's `_setFloat`.
    fn set_float_at(&mut self, index: usize, value: f64) {
        self.floats[index] = value as f32;
    }

    /// Dart's `_getSlotsForUniform`.
    fn get_slots_for_uniform(
        &self,
        name: &str,
        size: usize,
    ) -> Result<Vec<UniformFloatSlot>, UniformError> {
        let (index, info_size) = self.program.get_uniform_float_info(name)?;
        if info_size != size {
            return Err(UniformError::WrongSize {
                name: name.to_owned(),
                size: info_size,
                expected: size,
            });
        }
        Ok((0..size).map(|i| self.slot(i, index + i)).collect())
    }

    /// The float slot of component `index` at `shader_index`.
    fn slot(&self, index: usize, shader_index: usize) -> UniformFloatSlot {
        UniformFloatSlot {
            program: self.program.effect().id(),
            shader_index,
            index,
        }
    }

    /// Access the float binding for uniform named `name` with optional offset `index`, 0 when
    /// `None`. Example `index` values: 1 for `foo.y`, 2 for `foo.b`.
    pub fn get_uniform_float(
        &self,
        name: &str,
        index: Option<usize>,
    ) -> Result<UniformFloatSlot, UniformError> {
        let index = index.unwrap_or(0);
        let (shader_index, size) = self.program.get_uniform_float_info(name)?;
        if index >= size {
            return Err(UniformError::IndexOutOfBounds {
                name: name.to_owned(),
                index,
            });
        }
        Ok(self.slot(index, shader_index + index))
    }

    /// Access the float binding for a vec2 uniform named `name`.
    pub fn get_uniform_vec2(&self, name: &str) -> Result<UniformVec2Slot, UniformError> {
        let slots = self.get_slots_for_uniform(name, 2)?;
        Ok(UniformVec2Slot {
            x_slot: slots[0],
            y_slot: slots[1],
        })
    }

    /// Access the float binding for a vec3 uniform named `name`.
    pub fn get_uniform_vec3(&self, name: &str) -> Result<UniformVec3Slot, UniformError> {
        let slots = self.get_slots_for_uniform(name, 3)?;
        Ok(UniformVec3Slot {
            x_slot: slots[0],
            y_slot: slots[1],
            z_slot: slots[2],
        })
    }

    /// Access the float binding for a vec4 uniform named `name`.
    pub fn get_uniform_vec4(&self, name: &str) -> Result<UniformVec4Slot, UniformError> {
        let slots = self.get_slots_for_uniform(name, 4)?;
        Ok(UniformVec4Slot {
            x_slot: slots[0],
            y_slot: slots[1],
            z_slot: slots[2],
            w_slot: slots[3],
        })
    }

    /// Access the float binding for a mat2 uniform named `name`.
    pub fn get_uniform_mat2(&self, name: &str) -> Result<UniformMat2Slot, UniformError> {
        let slots = self.get_slots_for_uniform(name, 4)?;
        Ok(UniformMat2Slot {
            slots: to_array(&slots),
        })
    }

    /// Access the float binding for a mat3 uniform named `name`.
    pub fn get_uniform_mat3(&self, name: &str) -> Result<UniformMat3Slot, UniformError> {
        let slots = self.get_slots_for_uniform(name, 9)?;
        Ok(UniformMat3Slot {
            slots: to_array(&slots),
        })
    }

    /// Access the float binding for a mat4 uniform named `name`.
    pub fn get_uniform_mat4(&self, name: &str) -> Result<UniformMat4Slot, UniformError> {
        let slots = self.get_slots_for_uniform(name, 16)?;
        Ok(UniformMat4Slot {
            slots: to_array(&slots),
        })
    }

    /// Dart's `_getUniformArray`.
    fn get_uniform_array<T>(
        &self,
        name: &str,
        element_size: usize,
        element_factory: impl Fn(&[UniformFloatSlot]) -> T,
    ) -> Result<UniformArray<T>, UniformError> {
        let (index, size) = self.program.get_uniform_float_info(name)?;
        if size % element_size != 0 {
            return Err(UniformError::NotAMultiple {
                name: name.to_owned(),
                size,
                element_size,
            });
        }
        let num_elements = size / element_size;
        let elements = (0..num_elements)
            .map(|i| {
                let slots: Vec<UniformFloatSlot> = (0..element_size)
                    .map(|j| self.slot(j, index + i * element_size + j))
                    .collect();
                element_factory(&slots)
            })
            .collect();
        Ok(UniformArray { elements })
    }

    /// Access the binding for a float array uniform named `name`.
    ///
    /// WGSL keeps no float array in a uniform struct, so a GLSL `float uValues[8]` is an
    /// `array<vec4<f32>, 2>`, whose eight floats this reads as eight elements.
    pub fn get_uniform_float_array(
        &self,
        name: &str,
    ) -> Result<UniformArray<UniformFloatSlot>, UniformError> {
        self.get_uniform_array(name, 1, |components| components[0])
    }

    /// Access the binding for a vec2 array uniform named `name`.
    pub fn get_uniform_vec2_array(
        &self,
        name: &str,
    ) -> Result<UniformArray<UniformVec2Slot>, UniformError> {
        self.get_uniform_array(name, 2, |components| UniformVec2Slot {
            x_slot: components[0],
            y_slot: components[1],
        })
    }

    /// Access the binding for a vec3 array uniform named `name`.
    pub fn get_uniform_vec3_array(
        &self,
        name: &str,
    ) -> Result<UniformArray<UniformVec3Slot>, UniformError> {
        self.get_uniform_array(name, 3, |components| UniformVec3Slot {
            x_slot: components[0],
            y_slot: components[1],
            z_slot: components[2],
        })
    }

    /// Access the binding for a vec4 array uniform named `name`.
    pub fn get_uniform_vec4_array(
        &self,
        name: &str,
    ) -> Result<UniformArray<UniformVec4Slot>, UniformError> {
        self.get_uniform_array(name, 4, |components| UniformVec4Slot {
            x_slot: components[0],
            y_slot: components[1],
            z_slot: components[2],
            w_slot: components[3],
        })
    }

    /// Access the binding for a mat2 array uniform named `name`.
    pub fn get_uniform_mat2_array(
        &self,
        name: &str,
    ) -> Result<UniformArray<UniformMat2Slot>, UniformError> {
        self.get_uniform_array(name, 4, |components| UniformMat2Slot {
            slots: to_array(components),
        })
    }

    /// Access the binding for a mat3 array uniform named `name`.
    pub fn get_uniform_mat3_array(
        &self,
        name: &str,
    ) -> Result<UniformArray<UniformMat3Slot>, UniformError> {
        self.get_uniform_array(name, 9, |components| UniformMat3Slot {
            slots: to_array(components),
        })
    }

    /// Access the binding for a mat4 array uniform named `name`.
    pub fn get_uniform_mat4_array(
        &self,
        name: &str,
    ) -> Result<UniformArray<UniformMat4Slot>, UniformError> {
        self.get_uniform_array(name, 16, |components| UniformMat4Slot {
            slots: to_array(components),
        })
    }

    /// Access the [`ImageSamplerSlot`] binding associated with the sampler named `name`: the
    /// texture's name in the WGSL, as its pair's sampler is named after it.
    ///
    /// The index provided to [`set_image_sampler`](Self::set_image_sampler) is the index of the
    /// image defined in the fragment program, excluding all non-image uniforms.
    pub fn get_image_sampler(&self, name: &str) -> Result<ImageSamplerSlot, UniformError> {
        let index = self.program.get_image_sampler_index(name)?;
        Ok(ImageSamplerSlot {
            program: self.program.effect().id(),
            shader_index: index,
        })
    }

    /// Sets the sampler uniform at `index` to `image`.
    ///
    /// The index provided to [`set_image_sampler`](Self::set_image_sampler) is the index of the
    /// image defined in the fragment program, excluding all non-image uniforms.
    ///
    /// The `filter_quality` argument sets the quality level used to sample the image; Flutter's
    /// default is [`FilterQuality::None`].
    ///
    /// All the sampler uniforms that a shader expects must be provided before a paint is made
    /// from it.
    ///
    /// # Panics
    ///
    /// Where `index` is past the program's images: "Sampler index out of bounds".
    pub fn set_image_sampler(
        &mut self,
        index: usize,
        image: &Image,
        filter_quality: FilterQuality,
    ) {
        let Some(sampler) = self.samplers.get_mut(index) else {
            panic!("Sampler index out of bounds");
        };
        *sampler = Some(valo::EffectImage {
            image: Some(image.clone()),
            sampling: filter_quality.to_valo(),
        });
    }

    /// Dart's `_floats.length`.
    pub(crate) fn float_count(&self) -> usize {
        self.floats.len()
    }

    /// Dart's `_validateSamplers`: whether every image is set.
    fn validate_samplers(&self) -> bool {
        self.samplers.iter().all(Option::is_some)
    }

    /// Dart's `_validateImageFilter`: whether there is a first image, left for the filter's
    /// input, and every other is set.
    pub(crate) fn validate_image_filter(&self) -> bool {
        match self.samplers.split_first() {
            Some((_, others)) => others.iter().all(Option::is_some),
            None => false,
        }
    }

    /// The shader as a filter's: its floats and images now, its first image left for the
    /// filter's input, sampled with `input_sampling`. `ReusableFragmentShader::as_image_filter`.
    pub(crate) fn as_image_filter(&self, input_sampling: valo::Sampling) -> valo::RuntimeShader {
        let mut images = self.effect_images();
        if let Some(input) = images.first_mut() {
            *input = valo::EffectImage {
                image: None,
                sampling: input_sampling,
            };
        }
        self.runtime_shader(images)
    }

    /// The images, an unset one left empty.
    fn effect_images(&self) -> Vec<valo::EffectImage> {
        self.samplers
            .iter()
            .map(|sampler| {
                sampler.clone().unwrap_or(valo::EffectImage {
                    image: None,
                    sampling: FilterQuality::None.to_valo(),
                })
            })
            .collect()
    }

    /// The program's effect with the floats as they are now and `images`.
    fn runtime_shader(&self, images: Vec<valo::EffectImage>) -> valo::RuntimeShader {
        let uniforms = self.program.uniform_bytes(&self.floats);
        valo::RuntimeShader::new(Arc::clone(self.program.effect()), uniforms, images)
            .expect("a shader's floats and images are its program's")
    }
}

/// The shader as a paint draws it, its floats and images copied now:
/// `ReusableFragmentShader::shader`, as `Paint.shader` takes a `FragmentShader`.
///
/// # Panics
///
/// Where an image is not set, as Dart's `Paint.shader` setter throws in a debug build.
impl From<&FragmentShader> for Shader {
    fn from(shader: &FragmentShader) -> Shader {
        assert!(
            shader.validate_samplers(),
            "Invalid FragmentShader: missing sampler"
        );
        Shader::runtime(shader.runtime_shader(shader.effect_images()))
            .expect("a shader with every image set is a paint shader")
    }
}

/// `slots` as an array of `N`, as the matrix slots hold them.
fn to_array<const N: usize>(slots: &[UniformFloatSlot]) -> [UniformFloatSlot; N] {
    slots
        .try_into()
        .expect("as many slots as the matrix has entries")
}

#[cfg(test)]
mod tests {
    //! Ported from the engine's `testing/dart/fragment_shader_test.dart`, its GLSL fixtures
    //! (`lib/ui/fixtures/shaders`) written again in WGSL. WGSL keeps no `float` or `vec2` array in
    //! a uniform struct, so those fixtures' arrays are arrays of `vec4`, the same floats.

    use super::*;
    use crate::painting::ImageFilter;

    fn program(wgsl: &str) -> Arc<FragmentProgram> {
        FragmentProgram::from_wgsl(wgsl).expect("a valid program")
    }

    const FLOAT_UNIFORM: &str = "
struct U { color_r: f32 }
@group(2) @binding(0) var<uniform> u: U;
fn shade(position: vec2<f32>) -> vec4<f32> { return vec4<f32>(u.color_r, 0.0, 0.0, 1.0); }
";

    const VEC2_UNIFORM: &str = "
struct U { color_rg: vec2<f32> }
@group(2) @binding(0) var<uniform> u: U;
fn shade(position: vec2<f32>) -> vec4<f32> { return vec4<f32>(u.color_rg, 0.0, 1.0); }
";

    const VEC3_UNIFORM: &str = "
struct U { color_rgb: vec3<f32> }
@group(2) @binding(0) var<uniform> u: U;
fn shade(position: vec2<f32>) -> vec4<f32> { return vec4<f32>(u.color_rgb, 1.0); }
";

    const VEC4_UNIFORM: &str = "
struct U { color_rgba: vec4<f32> }
@group(2) @binding(0) var<uniform> u: U;
fn shade(position: vec2<f32>) -> vec4<f32> { return u.color_rgba; }
";

    /// `uniforms.frag`: a uniform of each kind, the arrays after the ones it draws with.
    const UNIFORMS: &str = "
struct U {
    iFloatUniform: f32,
    iVec2Uniform: vec2<f32>,
    iMat2Uniform: mat2x2<f32>,
    iVec3Uniform: vec3<f32>,
    iVec4Uniform: vec4<f32>,
    iFloatArrayUniform: array<vec4<f32>, 3>,
    iVec3ArrayUniform: array<vec3<f32>, 3>,
}
@group(2) @binding(0) var<uniform> u: U;
fn shade(position: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(u.iFloatUniform, u.iVec2Uniform, u.iMat2Uniform[1][1]);
}
";

    /// `uniform_ordering.frag`: floats around an image, which counts apart from them.
    const UNIFORM_ORDERING: &str = "
struct U { a: f32, b: f32, c: f32 }
@group(2) @binding(0) var<uniform> u: U;
@group(1) @binding(0) var u_texture: texture_2d<f32>;
@group(1) @binding(1) var u_texture_sampler: sampler;
fn shade(position: vec2<f32>) -> vec4<f32> {
    let sampled = textureSampleLevel(u_texture, u_texture_sampler, vec2<f32>(0.5), 0.0);
    if u.a == 1.0 && u.b == 2.0 && u.c == 3.0 {
        return vec4<f32>(0.0, 1.0, 0.0, 1.0) + sampled * 0.0;
    }
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
";

    /// `blue_green_sampler.frag`: the right edge of a half blue, half green image.
    const BLUE_GREEN_SAMPLER: &str = "
@group(1) @binding(0) var iChild: texture_2d<f32>;
@group(1) @binding(1) var iChild_sampler: sampler;
fn shade(position: vec2<f32>) -> vec4<f32> {
    let color = textureSampleLevel(iChild, iChild_sampler, vec2<f32>(1.0, 0.0), 0.0);
    return vec4<f32>(color.rgb, 1.0);
}
";

    const NO_UNIFORMS: &str =
        "fn shade(position: vec2<f32>) -> vec4<f32> { return vec4<f32>(0.0, 1.0, 0.0, 1.0); }";

    /// `missing_size.frag`: an image, but no size first.
    const MISSING_SIZE: &str = "
@group(1) @binding(0) var u_texture: texture_2d<f32>;
@group(1) @binding(1) var u_texture_sampler: sampler;
fn shade(position: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(u_texture, u_texture_sampler, position / vec2<f32>(100.0), 0.0).bgra;
}
";

    /// `missing_texture.frag`: a size, but no image.
    const MISSING_TEXTURE: &str = "
struct U { u_size: vec2<f32> }
@group(2) @binding(0) var<uniform> u: U;
fn shade(position: vec2<f32>) -> vec4<f32> { return vec4<f32>(u.u_size.x, u.u_size.y, 0.0, 1.0); }
";

    /// `filter_shader.frag`: the input with its red and blue swapped.
    const FILTER_SHADER: &str = "
struct U { u_size: vec2<f32> }
@group(2) @binding(0) var<uniform> u: U;
@group(1) @binding(0) var u_texture: texture_2d<f32>;
@group(1) @binding(1) var u_texture_sampler: sampler;
fn shade(position: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(u_texture, u_texture_sampler, position / u.u_size, 0.0).bgra;
}
";

    /// Dart's `FragmentProgram uniform info`.
    #[test]
    fn a_slot_indexes_the_floats_packed_tightly() {
        let shader = program(UNIFORMS).fragment_shader();
        let slots = [
            shader.get_uniform_float("iFloatUniform", None),
            shader.get_uniform_float("iVec2Uniform", Some(0)),
            shader.get_uniform_float("iVec2Uniform", Some(1)),
            shader.get_uniform_float("iMat2Uniform", Some(0)),
            shader.get_uniform_float("iMat2Uniform", Some(1)),
            shader.get_uniform_float("iMat2Uniform", Some(2)),
            shader.get_uniform_float("iMat2Uniform", Some(3)),
        ];
        for (i, slot) in slots.into_iter().enumerate() {
            assert_eq!(slot.expect("a float slot").shader_index(), i);
        }
    }

    /// Dart's `getUniformFloat offset overflow`; an underflow is not a `usize`.
    #[test]
    fn a_float_slot_past_the_uniform_is_out_of_bounds() {
        let shader = program(FLOAT_UNIFORM).fragment_shader();
        let error = shader
            .get_uniform_float("color_r", Some(2))
            .expect_err("past the uniform");
        assert_eq!(error.to_string(), "Index `2` out of bounds for `color_r`.");
    }

    /// Dart's `wrong datatype` cases, each asking a slot of another size than the uniform's.
    #[test]
    fn a_slot_of_another_size_is_refused() {
        let vec2 = program(VEC2_UNIFORM).fragment_shader();
        let vec3 = program(VEC3_UNIFORM).fragment_shader();
        let vec4 = program(VEC4_UNIFORM).fragment_shader();
        let message = |error: UniformError| error.to_string();
        assert!(
            message(vec3.get_uniform_vec2("color_rgb").unwrap_err())
                .contains("`color_rgb` has size 3, not size 2.")
        );
        assert!(
            message(vec2.get_uniform_vec3("color_rg").unwrap_err())
                .contains("`color_rg` has size 2, not size 3.")
        );
        assert!(
            message(vec3.get_uniform_vec4("color_rgb").unwrap_err())
                .contains("`color_rgb` has size 3, not size 4.")
        );
        assert!(
            message(vec3.get_uniform_mat2("color_rgb").unwrap_err())
                .contains("`color_rgb` has size 3, not size 4.")
        );
        assert!(
            message(vec3.get_uniform_mat3("color_rgb").unwrap_err())
                .contains("Uniform `color_rgb` has size 3, not size 9.")
        );
        assert!(
            message(vec3.get_uniform_mat4("color_rgb").unwrap_err())
                .contains("Uniform `color_rgb` has size 3, not size 16.")
        );
        assert!(
            message(vec3.get_uniform_vec2_array("color_rgb").unwrap_err())
                .contains("Uniform size (3) for \"color_rgb\" is not a multiple of 2.")
        );
        assert!(
            message(vec4.get_uniform_vec3_array("color_rgba").unwrap_err())
                .contains("Uniform size (4) for \"color_rgba\" is not a multiple of 3.")
        );
        for (array, size) in [
            (vec3.get_uniform_vec4_array("color_rgb").map(|_| ()), 4),
            (vec3.get_uniform_mat2_array("color_rgb").map(|_| ()), 4),
            (vec3.get_uniform_mat3_array("color_rgb").map(|_| ()), 9),
            (vec3.get_uniform_mat4_array("color_rgb").map(|_| ()), 16),
        ] {
            assert!(message(array.unwrap_err()).contains(&format!(
                "Uniform size (3) for \"color_rgb\" is not a multiple of {size}."
            )));
        }
    }

    /// Dart's `FragmentProgram getImageSampler`, `… unknown` and `… wrong type`.
    #[test]
    fn an_image_sampler_is_looked_up_by_its_textures_name() {
        let shader = program(UNIFORM_ORDERING).fragment_shader();
        let slot = shader.get_image_sampler("u_texture").expect("the image");
        assert_eq!(slot.shader_index(), 0);
        assert!(
            shader
                .get_image_sampler("unknown")
                .unwrap_err()
                .to_string()
                .contains("No uniform named \"unknown\".")
        );
        assert!(
            shader
                .get_image_sampler("b")
                .unwrap_err()
                .to_string()
                .contains("Uniform \"b\" is not an image sampler.")
        );
    }

    /// Dart's `FragmentShader with sampler asserts if sampler is missing when assigned to paint`.
    #[test]
    #[should_panic(expected = "Invalid FragmentShader")]
    fn a_paint_needs_every_image_set() {
        let shader = program(BLUE_GREEN_SAMPLER).fragment_shader();
        let _paint = Shader::from(&shader);
    }

    /// Dart's `ImageFilter.shader errors if shader does not have correct uniform layout`.
    #[test]
    fn a_shader_filter_needs_a_size_first_and_an_image() {
        let cases = [
            (NO_UNIFORMS, true, true),
            (MISSING_SIZE, true, false),
            (MISSING_TEXTURE, false, true),
        ];
        for (wgsl, float_error, sampler_error) in cases {
            let shader = program(wgsl).fragment_shader();
            let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                ImageFilter::shader(&shader, FilterQuality::None);
            }))
            .expect_err("the filter is refused");
            let message = panicked
                .downcast_ref::<String>()
                .cloned()
                .unwrap_or_default();
            assert_eq!(
                message.contains("shader has fewer than two float"),
                float_error,
                "{message}"
            );
            assert_eq!(
                message.contains("shader is missing a sampler uniform"),
                sampler_error,
                "{message}"
            );
        }
    }

    /// Dart's `ImageFilter.shader equality checks consider uniform values`.
    #[test]
    #[expect(deprecated, reason = "Dart's test sets the float by index")]
    fn a_shader_filter_equals_another_of_the_same_uniforms_and_quality() {
        let mut shader = program(FILTER_SHADER).fragment_shader();
        let filter = ImageFilter::shader(&shader, FilterQuality::None);
        assert_eq!(filter, filter.clone());
        assert_eq!(filter, ImageFilter::shader(&shader, FilterQuality::None));
        let low_quality = ImageFilter::shader(&shader, FilterQuality::Low);
        assert_ne!(filter, low_quality);
        assert_eq!(
            low_quality,
            ImageFilter::shader(&shader, FilterQuality::Low)
        );
        shader.set_float(0, 1.0);
        assert_ne!(filter, ImageFilter::shader(&shader, FilterQuality::None));
    }

    /// A slot keeps nothing of the shader it was looked up on, so the next frame's shader of the
    /// same program takes the same slot.
    #[test]
    fn a_slot_sets_any_shader_of_its_program() {
        let program = program(VEC4_UNIFORM);
        let first = program.fragment_shader();
        let color = first.get_uniform_vec4("color_rgba").expect("a vec4");
        let mut next = program.fragment_shader();
        color.set(&mut next, 0.1, 0.2, 0.3, 0.4);
        assert_eq!(next.floats, [0.1, 0.2, 0.3, 0.4]);
        assert_eq!(first.floats, [0.0; 4]);
    }

    #[cfg(feature = "test-support")]
    mod drawn {
        //! The engine's tests that draw: each through `drawPaint` into a small image, here a
        //! rectangle of the shader over black, so an output that is not opaque reads back as its
        //! premultiplied colour.

        use super::*;
        use crate::painting::{Canvas, Paint};
        use crate::test_support::{drawn_pixels, image_from_pixels, picture_to_image};

        /// The program's shader drawn over black into `width` by `height` pixels, its first row.
        fn rendered(shader: &FragmentShader, width: u32, height: u32) -> Vec<[u8; 4]> {
            let whole = valo::Rect::new(0.0, 0.0, width as f32, height as f32);
            let mut canvas = Canvas::new();
            canvas.draw_rect(whole, &Paint::from_color(valo::Color::BLACK));
            canvas.draw_rect(whole, &Paint::from_shader(Shader::from(shader)));
            let pixels = drawn_pixels(canvas.build(), [width, height]);
            pixels.as_chunks::<4>().0[..width as usize].to_vec()
        }

        /// Dart's `_expectShaderRendersColor`, at its image dimension of 4.
        fn expect_renders(shader: &FragmentShader, color: [u8; 4]) {
            for pixel in rendered(shader, 4, 4) {
                assert_eq!(pixel, color);
            }
        }

        /// Dart's `_expectShaderRendersBarcode`, one colour a pixel, compared by red, green and
        /// blue within 0.06.
        fn expect_barcode(shader: &FragmentShader, colors: &[[u8; 4]]) {
            let width = colors.len() as u32;
            for (i, (pixel, expected)) in rendered(shader, width, width)
                .iter()
                .zip(colors)
                .enumerate()
            {
                for channel in 0..3 {
                    let difference =
                        (f64::from(pixel[channel]) - f64::from(expected[channel])).abs() / 255.0;
                    assert!(
                        difference <= 0.06,
                        "colour {i}: {pixel:?}, expected {expected:?}"
                    );
                }
            }
        }

        /// Dart's `Color.fromARGB` as red, green, blue and alpha bytes.
        const fn argb(a: u8, r: u8, g: u8, b: u8) -> [u8; 4] {
            [r, g, b, a]
        }

        /// A channel as Dart's `Color.r` answers it.
        fn channel(value: u8) -> f64 {
            f64::from(value) / 255.0
        }

        /// The channels of `colors` in order, red, green, blue and, with `alpha`, alpha.
        fn channels(colors: &[[u8; 4]], alpha: bool) -> Vec<f64> {
            let take = if alpha { 4 } else { 3 };
            colors
                .iter()
                .flat_map(|color| color[..take].iter().map(|&value| channel(value)))
                .collect()
        }

        #[expect(deprecated, reason = "Dart's tests set the floats by index")]
        fn set_floats(shader: &mut FragmentShader, from: usize, values: &[f64]) {
            for (i, value) in values.iter().enumerate() {
                shader.set_float(from + i, *value);
            }
        }

        const GREEN: [u8; 4] = [0, 255, 0, 255];
        const BLACK: [u8; 4] = [0, 0, 0, 255];

        #[test]
        fn a_float_is_set_by_index_and_by_name() {
            let mut shader = program(FLOAT_UNIFORM).fragment_shader();
            set_floats(&mut shader, 0, &[1.0]);
            expect_renders(&shader, argb(255, 255, 0, 0));
            let color = argb(255, 50, 0, 0);
            shader
                .get_uniform_float("color_r", None)
                .unwrap()
                .set(&mut shader, channel(color[0]));
            expect_renders(&shader, color);
        }

        #[test]
        fn a_vec2_is_set_by_index_and_by_name() {
            let mut shader = program(VEC2_UNIFORM).fragment_shader();
            set_floats(&mut shader, 0, &[1.0, 1.0]);
            expect_renders(&shader, argb(255, 255, 255, 0));
            let color = argb(255, 50, 50, 0);
            shader.get_uniform_vec2("color_rg").unwrap().set(
                &mut shader,
                channel(color[0]),
                channel(color[1]),
            );
            expect_renders(&shader, color);
        }

        #[test]
        fn a_vec3_is_set_by_index_and_by_name() {
            let mut shader = program(VEC3_UNIFORM).fragment_shader();
            let color = argb(255, 67, 42, 12);
            set_floats(&mut shader, 0, &channels(&[color], false));
            expect_renders(&shader, color);
            let color = argb(255, 42, 67, 12);
            shader.get_uniform_vec3("color_rgb").unwrap().set(
                &mut shader,
                channel(color[0]),
                channel(color[1]),
                channel(color[2]),
            );
            expect_renders(&shader, color);
        }

        #[test]
        fn a_vec4_is_set_by_index_and_by_name() {
            let mut shader = program(VEC4_UNIFORM).fragment_shader();
            let color = argb(255, 67, 42, 12);
            set_floats(&mut shader, 0, &channels(&[color], true));
            expect_renders(&shader, color);
            let color = argb(255, 12, 37, 27);
            shader.get_uniform_vec4("color_rgba").unwrap().set(
                &mut shader,
                channel(color[0]),
                channel(color[1]),
                channel(color[2]),
                channel(color[3]),
            );
            expect_renders(&shader, color);
        }

        #[test]
        fn a_mat2_is_set_by_index_and_by_name() {
            let mut shader = program(
                "struct U { color_rgba: mat2x2<f32> }
                 @group(2) @binding(0) var<uniform> u: U;
                 fn shade(position: vec2<f32>) -> vec4<f32> {
                     return vec4<f32>(u.color_rgba[0], u.color_rgba[1]);
                 }",
            )
            .fragment_shader();
            let color = argb(255, 67, 42, 12);
            set_floats(&mut shader, 0, &channels(&[color], true));
            expect_renders(&shader, color);
            let color = argb(255, 12, 37, 27);
            shader.get_uniform_mat2("color_rgba").unwrap().set(
                &mut shader,
                channel(color[0]),
                channel(color[1]),
                channel(color[2]),
                channel(color[3]),
            );
            expect_renders(&shader, color);
        }

        const MAT3_UNIFORM: &str = "
struct U { colors: mat3x3<f32> }
@group(2) @binding(0) var<uniform> u: U;
fn shade(position: vec2<f32>) -> vec4<f32> {
    let uv = position / 3.0;
    if uv.x < 0.3 { return vec4<f32>(u.colors[0], 1.0); }
    if uv.x < 0.6 { return vec4<f32>(u.colors[1], 1.0); }
    return vec4<f32>(u.colors[2], 1.0);
}
";

        #[test]
        fn a_mat3_is_set_by_index_and_by_name() {
            let mut shader = program(MAT3_UNIFORM).fragment_shader();
            let colors = [
                argb(255, 67, 42, 12),
                argb(255, 11, 22, 96),
                argb(255, 8, 16, 67),
            ];
            set_floats(&mut shader, 0, &channels(&colors, false));
            expect_barcode(&shader, &colors);
            let colors = [colors[1], colors[2], colors[0]];
            let c = channels(&colors, false);
            shader.get_uniform_mat3("colors").unwrap().set(
                &mut shader,
                c[0],
                c[1],
                c[2],
                c[3],
                c[4],
                c[5],
                c[6],
                c[7],
                c[8],
            );
            expect_barcode(&shader, &colors);
        }

        const MAT4_UNIFORM: &str = "
struct U { colors: mat4x4<f32> }
@group(2) @binding(0) var<uniform> u: U;
fn shade(position: vec2<f32>) -> vec4<f32> {
    let uv = position / 4.0;
    if uv.x < 0.25 { return u.colors[0]; }
    if uv.x < 0.5 { return u.colors[1]; }
    if uv.x < 0.75 { return u.colors[2]; }
    return u.colors[3];
}
";

        #[test]
        fn a_mat4_is_set_by_index_and_by_name() {
            let mut shader = program(MAT4_UNIFORM).fragment_shader();
            let colors = [
                argb(6, 67, 42, 12),
                argb(33, 11, 22, 96),
                argb(99, 8, 16, 67),
                argb(120, 11, 22, 96),
            ];
            set_floats(&mut shader, 0, &channels(&colors, true));
            expect_barcode(&shader, &colors);
            let colors = [
                argb(78, 11, 22, 96),
                argb(255, 8, 16, 67),
                argb(99, 11, 22, 96),
                argb(46, 67, 42, 12),
            ];
            let c = channels(&colors, true);
            shader.get_uniform_mat4("colors").unwrap().set(
                &mut shader,
                c[0],
                c[1],
                c[2],
                c[3],
                c[4],
                c[5],
                c[6],
                c[7],
                c[8],
                c[9],
                c[10],
                c[11],
                c[12],
                c[13],
                c[14],
                c[15],
            );
            expect_barcode(&shader, &colors);
        }

        #[test]
        fn a_float_array_is_set_by_index_and_by_name() {
            let mut shader = program(
                "struct U { color_array: array<vec4<f32>, 1> }
                 @group(2) @binding(0) var<uniform> u: U;
                 fn shade(position: vec2<f32>) -> vec4<f32> { return u.color_array[0]; }",
            )
            .fragment_shader();
            let color = argb(255, 11, 22, 96);
            set_floats(&mut shader, 0, &channels(&[color], true));
            expect_renders(&shader, color);
            let color = argb(255, 96, 11, 22);
            let color_rgba = shader.get_uniform_float_array("color_array").unwrap();
            assert_eq!(color_rgba.len(), 4);
            for (i, value) in channels(&[color], true).into_iter().enumerate() {
                color_rgba[i].set(&mut shader, value);
            }
            expect_renders(&shader, color);
        }

        #[test]
        fn a_vec2_array_is_set_by_index_and_by_name() {
            let mut shader = program(
                "struct U { color_array: array<vec4<f32>, 1> }
                 @group(2) @binding(0) var<uniform> u: U;
                 fn shade(position: vec2<f32>) -> vec4<f32> {
                     return vec4<f32>(u.color_array[0].xy, u.color_array[0].zw);
                 }",
            )
            .fragment_shader();
            let color = argb(255, 67, 42, 12);
            set_floats(&mut shader, 0, &channels(&[color], true));
            expect_renders(&shader, color);
            let color = argb(255, 1, 73, 26);
            let c = channels(&[color], true);
            let color_rgba = shader.get_uniform_vec2_array("color_array").unwrap();
            color_rgba[0].set(&mut shader, c[0], c[1]);
            color_rgba[1].set(&mut shader, c[2], c[3]);
            expect_renders(&shader, color);
        }

        #[test]
        fn a_vec3_array_is_set_by_index_and_by_name() {
            let mut shader = program(
                "struct U { u_size: vec2<f32>, color_array: array<vec3<f32>, 2> }
                 @group(2) @binding(0) var<uniform> u: U;
                 fn shade(position: vec2<f32>) -> vec4<f32> {
                     let uv = position / u.u_size;
                     if uv.x < 0.5 { return vec4<f32>(u.color_array[0], 1.0); }
                     return vec4<f32>(u.color_array[1], 1.0);
                 }",
            )
            .fragment_shader();
            let colors = [argb(255, 67, 42, 12), argb(255, 11, 22, 96)];
            set_floats(&mut shader, 0, &[2.0, 2.0]);
            set_floats(&mut shader, 2, &channels(&colors, false));
            expect_barcode(&shader, &colors);
            let colors = [colors[1], colors[0]];
            let c = channels(&colors, false);
            shader
                .get_uniform_vec2("u_size")
                .unwrap()
                .set(&mut shader, 2.0, 2.0);
            let gpu_colors = shader.get_uniform_vec3_array("color_array").unwrap();
            gpu_colors[0].set(&mut shader, c[0], c[1], c[2]);
            gpu_colors[1].set(&mut shader, c[3], c[4], c[5]);
            expect_barcode(&shader, &colors);
        }

        #[test]
        fn a_vec4_array_is_set_by_index_and_by_name() {
            let mut shader = program(
                "struct U { u_size: vec2<f32>, color_array: array<vec4<f32>, 2> }
                 @group(2) @binding(0) var<uniform> u: U;
                 fn shade(position: vec2<f32>) -> vec4<f32> {
                     let uv = position / u.u_size;
                     if uv.x < 0.5 { return u.color_array[0]; }
                     return u.color_array[1];
                 }",
            )
            .fragment_shader();
            let colors = [argb(77, 67, 42, 12), argb(51, 11, 22, 96)];
            set_floats(&mut shader, 0, &[2.0, 2.0]);
            set_floats(&mut shader, 2, &channels(&colors, true));
            expect_barcode(&shader, &colors);
            let colors = [argb(51, 11, 22, 96), argb(77, 67, 42, 12)];
            let c = channels(&colors, true);
            shader
                .get_uniform_vec2("u_size")
                .unwrap()
                .set(&mut shader, 2.0, 2.0);
            let gpu_colors = shader.get_uniform_vec4_array("color_array").unwrap();
            gpu_colors[0].set(&mut shader, c[0], c[1], c[2], c[3]);
            gpu_colors[1].set(&mut shader, c[4], c[5], c[6], c[7]);
            expect_barcode(&shader, &colors);
        }

        #[test]
        fn a_mat2_array_is_set_by_index_and_by_name() {
            let mut shader = program(
                "struct U { colors: array<mat2x2<f32>, 2> }
                 @group(2) @binding(0) var<uniform> u: U;
                 fn shade(position: vec2<f32>) -> vec4<f32> {
                     if position.x < 1.0 { return vec4<f32>(u.colors[0][0], u.colors[0][1]); }
                     return vec4<f32>(u.colors[1][0], u.colors[1][1]);
                 }",
            )
            .fragment_shader();
            let colors = [argb(77, 67, 42, 12), argb(51, 11, 22, 96)];
            set_floats(&mut shader, 0, &channels(&colors, true));
            expect_barcode(&shader, &colors);
            let colors = [argb(51, 11, 22, 96), argb(77, 67, 42, 12)];
            let c = channels(&colors, true);
            let gpu_colors = shader.get_uniform_mat2_array("colors").unwrap();
            gpu_colors[0].set(&mut shader, c[0], c[1], c[2], c[3]);
            gpu_colors[1].set(&mut shader, c[4], c[5], c[6], c[7]);
            expect_barcode(&shader, &colors);
        }

        #[test]
        fn a_mat3_array_is_set_by_index_and_by_name() {
            let mut shader = program(
                "struct U { colors: array<mat3x3<f32>, 2> }
                 @group(2) @binding(0) var<uniform> u: U;
                 fn shade(position: vec2<f32>) -> vec4<f32> {
                     let column = u32(position.x);
                     return vec4<f32>(u.colors[column / 3u][column % 3u], 1.0);
                 }",
            )
            .fragment_shader();
            let colors = [
                argb(255, 67, 42, 12),
                argb(255, 11, 22, 96),
                argb(255, 2, 4, 6),
                argb(255, 8, 10, 12),
                argb(255, 14, 16, 18),
                argb(255, 20, 22, 24),
            ];
            set_floats(&mut shader, 0, &channels(&colors, false));
            expect_barcode(&shader, &colors);
            let colors = [
                colors[3], colors[4], colors[5], colors[0], colors[1], colors[2],
            ];
            let c = channels(&colors, false);
            let gpu_colors = shader.get_uniform_mat3_array("colors").unwrap();
            gpu_colors[0].set(
                &mut shader,
                c[0],
                c[1],
                c[2],
                c[3],
                c[4],
                c[5],
                c[6],
                c[7],
                c[8],
            );
            gpu_colors[1].set(
                &mut shader,
                c[9],
                c[10],
                c[11],
                c[12],
                c[13],
                c[14],
                c[15],
                c[16],
                c[17],
            );
            expect_barcode(&shader, &colors);
        }

        #[test]
        fn a_mat4_array_is_set_by_index_and_by_name() {
            let mut shader = program(
                "struct U { colors: array<mat4x4<f32>, 2> }
                 @group(2) @binding(0) var<uniform> u: U;
                 fn shade(position: vec2<f32>) -> vec4<f32> {
                     let column = u32(position.x);
                     return u.colors[column / 4u][column % 4u];
                 }",
            )
            .fragment_shader();
            let colors = [
                argb(31, 8, 16, 67),
                argb(29, 11, 22, 96),
                argb(43, 32, 34, 36),
                argb(41, 26, 28, 30),
                argb(39, 20, 22, 24),
                argb(37, 14, 16, 18),
                argb(35, 8, 10, 12),
                argb(33, 2, 4, 6),
            ];
            set_floats(&mut shader, 0, &channels(&colors, true));
            expect_barcode(&shader, &colors);
            let colors = [
                colors[1], colors[0], colors[7], colors[6], colors[5], colors[4], colors[3],
                colors[2],
            ];
            let c = channels(&colors, true);
            let gpu_colors = shader.get_uniform_mat4_array("colors").unwrap();
            for (element, values) in c.as_chunks::<16>().0.iter().enumerate() {
                gpu_colors[element].set(
                    &mut shader,
                    values[0],
                    values[1],
                    values[2],
                    values[3],
                    values[4],
                    values[5],
                    values[6],
                    values[7],
                    values[8],
                    values[9],
                    values[10],
                    values[11],
                    values[12],
                    values[13],
                    values[14],
                    values[15],
                );
            }
            expect_barcode(&shader, &colors);
        }

        /// A 10x10 image whose left half is blue and right half green, from pixels: Dart's
        /// `_createBlueGreenImage`.
        fn blue_green_image() -> Image {
            let mut pixels = Vec::new();
            for _y in 0..10 {
                for x in 0..10 {
                    let blue_or_green: [u8; 4] = if x < 5 {
                        [0, 0, 255, 255]
                    } else {
                        [0, 255, 0, 255]
                    };
                    pixels.extend(blue_or_green);
                }
            }
            image_from_pixels([10, 10], &pixels)
        }

        /// The same image drawn from a picture: Dart's `_createBlueGreenImageSync`.
        fn blue_green_image_sync() -> Image {
            let mut canvas = Canvas::new();
            canvas.draw_rect(
                valo::Rect::new(0.0, 0.0, 5.0, 10.0),
                &Paint::from_color(valo::Color::rgb(0.0, 0.0, 1.0)),
            );
            canvas.draw_rect(
                valo::Rect::new(5.0, 0.0, 5.0, 10.0),
                &Paint::from_color(valo::Color::rgb(0.0, 1.0, 0.0)),
            );
            picture_to_image(&canvas.build(), [10, 10])
        }

        /// Dart's `FragmentShader setSampler throws with out-of-bounds index`.
        #[test]
        #[should_panic(expected = "Sampler index out of bounds")]
        fn an_image_past_the_programs_is_refused() {
            let mut shader = program(BLUE_GREEN_SAMPLER).fragment_shader();
            shader.set_image_sampler(1, &blue_green_image(), FilterQuality::None);
        }

        /// Dart's `FragmentShader blue-green image renders green`, and its `GPU image` case.
        #[test]
        fn an_image_is_sampled_whether_uploaded_or_drawn() {
            for image in [blue_green_image(), blue_green_image_sync()] {
                let mut shader = program(BLUE_GREEN_SAMPLER).fragment_shader();
                shader.set_image_sampler(0, &image, FilterQuality::None);
                expect_renders(&shader, GREEN);
            }
        }

        /// Dart's `Reused FragmentShader simple shader renders correctly`, over `functions.frag`.
        #[test]
        fn a_shader_draws_again_with_its_new_floats() {
            let mut shader = program(
                "struct U { a: f32 }
                 @group(2) @binding(0) var<uniform> u: U;
                 fn add_a(x: f32) -> f32 { return x + u.a; }
                 fn pair_with_a(x: f32) -> vec2<f32> { return vec2<f32>(x, u.a); }
                 fn composed_function(x: f32) -> vec3<f32> { return vec3<f32>(add_a(x), pair_with_a(x)); }
                 fn multi_param(x: f32, y: f32, z: f32) -> f32 { return x * y * z * u.a; }
                 fn shade(position: vec2<f32>) -> vec4<f32> {
                     var x = add_a(0.0);
                     let v3 = composed_function(x);
                     x = multi_param(v3.x, v3.y, v3.z);
                     return vec4<f32>(0.0, x / 2.0, 0.0, 1.0);
                 }",
            )
            .fragment_shader();
            set_floats(&mut shader, 0, &[1.0]);
            expect_renders(&shader, GREEN);
            set_floats(&mut shader, 0, &[0.0]);
            expect_renders(&shader, BLACK);
        }

        /// Dart's `FragmentShader Uniforms are sorted correctly`, over `uniforms_sorted.frag`'s
        /// uniforms: floats and vectors mixed, so WGSL pads between them.
        #[test]
        fn floats_set_in_order_reach_the_uniforms_in_order() {
            let mut shader = program(
                "struct U {
                     u_color: vec4<f32>,
                     u_alpha: f32,
                     u_sparkle_color: vec4<f32>,
                     u_sparkle_alpha: f32,
                     u_blur: f32,
                     u_center: vec2<f32>,
                     u_radius_scale: f32,
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
                 @group(2) @binding(0) var<uniform> u: U;
                 fn shade(position: vec2<f32>) -> vec4<f32> {
                     if u.u_color.x > u.u_alpha || u.u_alpha > u.u_sparkle_color.x
                         || u.u_sparkle_color.x > u.u_sparkle_alpha || u.u_sparkle_alpha > u.u_blur
                         || u.u_blur > u.u_center.x || u.u_center.x > u.u_radius_scale
                         || u.u_radius_scale > u.u_max_radius
                         || u.u_max_radius > u.u_resolution_scale.x
                         || u.u_resolution_scale.x > u.u_noise_scale.x
                         || u.u_noise_scale.x > u.u_noise_phase || u.u_noise_phase > u.u_circle1.x
                         || u.u_circle1.x > u.u_circle2.x || u.u_circle2.x > u.u_circle3.x
                         || u.u_circle3.x > u.u_rotation1.x || u.u_rotation1.x > u.u_rotation2.x
                         || u.u_rotation2.x > u.u_rotation3.x {
                         return vec4<f32>(1.0, 0.0, 0.0, 1.0);
                     }
                     return vec4<f32>(0.0, 1.0, 0.0, 1.0);
                 }",
            )
            .fragment_shader();
            let ordered: Vec<f64> = (0..32).map(f64::from).collect();
            set_floats(&mut shader, 0, &ordered);
            expect_renders(&shader, GREEN);
        }

        /// Dart's `FragmentShader Uniforms with interleaved textures are sorted`.
        #[test]
        fn floats_count_apart_from_the_images_between_them() {
            let mut shader = program(UNIFORM_ORDERING).fragment_shader();
            set_floats(&mut shader, 0, &[1.0, 2.0, 3.0]);
            shader.set_image_sampler(0, &blue_green_image_sync(), FilterQuality::None);
            expect_renders(&shader, GREEN);
        }

        /// Dart's `FragmentShader fromAsset accepts a shader with no uniforms`.
        #[test]
        fn a_program_with_no_uniforms_draws() {
            expect_renders(&program(NO_UNIFORMS).fragment_shader(), GREEN);
        }

        /// Dart's `FragmentShader with uniforms renders correctly`.
        #[test]
        fn the_floats_set_are_the_colour_drawn() {
            let mut shader = program(UNIFORMS).fragment_shader();
            set_floats(&mut shader, 0, &[0.0, 0.25, 0.75, 0.0, 0.0, 0.0, 1.0]);
            let pixel = rendered(&shader, 1, 1)[0];
            let epsilon = 0.5 / 255.0 + 1e-6;
            for (value, expected) in pixel.iter().zip([0.0, 0.25, 0.75, 1.0]) {
                assert!((channel(*value) - expected).abs() <= epsilon, "{pixel:?}");
            }
        }

        /// Dart's `FragmentShader shader with mat2 uniform renders correctly`.
        #[test]
        fn a_mat2_set_by_index_is_read_column_by_column() {
            let mut shader = program(
                "struct U { uMat: mat2x2<f32> }
                 @group(2) @binding(0) var<uniform> u: U;
                 fn shade(position: vec2<f32>) -> vec4<f32> {
                     let m = u.uMat;
                     if abs(m[0][0] - 4.0) < 0.01 && abs(m[0][1] - 8.0) < 0.01
                         && abs(m[1][0] - 16.0) < 0.01 && abs(m[1][1] - 32.0) < 0.01 {
                         return vec4<f32>(0.0, 1.0, 0.0, 1.0);
                     }
                     return vec4<f32>(1.0, 0.0, 0.0, 1.0);
                 }",
            )
            .fragment_shader();
            set_floats(&mut shader, 0, &[4.0, 8.0, 16.0, 32.0]);
            expect_renders(&shader, GREEN);
        }

        /// Dart's `ImageFilter.shader can be applied to canvas operations`: red through a filter
        /// that swaps red and blue is blue.
        #[test]
        fn a_shader_filter_filters_a_layer() {
            let shader = program(FILTER_SHADER).fragment_shader();
            let filter = ImageFilter::shader(&shader, FilterQuality::None);
            let mut canvas = Canvas::new();
            canvas.save_layer(
                None,
                &Paint {
                    image_filter: Some(filter.to_valo()),
                    ..Paint::default()
                },
            );
            canvas.draw_rect(
                valo::Rect::new(0.0, 0.0, 1.0, 1.0),
                &Paint::from_color(valo::Color::rgb(1.0, 0.0, 0.0)),
            );
            canvas.restore();
            let pixels = drawn_pixels(canvas.build(), [1, 1]);
            assert_eq!(pixels, [0, 0, 255, 255]);
        }
    }
}
