//! An [`IconImage`] as Android's `Bitmap`, fitted to the most the system shows of it.
//!
//! What the system keeps of an image, a task's icon or a shortcut's, crosses into its own process
//! in one transaction, which a large image would not fit in; an image is scaled down to what the
//! system shows before it is handed over.

use inset_embedder::IconImage;
use jni::objects::{JIntArray, JObject, JValue};
use jni::sys::jint;
use jni::{Env, jni_sig, jni_str};

/// RGBA bytes as the platform's colours, alpha in the top byte.
pub(crate) fn argb_colors(rgba: &[u8]) -> Vec<jint> {
    let (pixels, _) = rgba.as_chunks::<4>();
    pixels
        .iter()
        .map(|&[red, green, blue, alpha]| i32::from_be_bytes([alpha, red, green, blue]))
        .collect()
}

/// `image` as a `Bitmap`, scaled down to `largest` pixels on its longer side where it is larger.
pub(crate) fn fitted_bitmap<'local>(
    env: &mut Env<'local>,
    image: &IconImage,
    largest: i32,
) -> jni::errors::Result<JObject<'local>> {
    let bitmap = bitmap(env, image)?;
    let [width, height] = [image.width, image.height].map(|side| side as i32);
    let side = width.max(height);
    if side <= largest || largest <= 0 {
        return Ok(bitmap);
    }
    let scale = f64::from(largest) / f64::from(side);
    let [width, height] =
        [width, height].map(|side| ((f64::from(side) * scale).round() as i32).max(1));
    env.call_static_method(
        jni_str!("android/graphics/Bitmap"),
        jni_str!("createScaledBitmap"),
        jni_sig!("(Landroid/graphics/Bitmap;IIZ)Landroid/graphics/Bitmap;"),
        &[
            JValue::Object(&bitmap),
            JValue::Int(width),
            JValue::Int(height),
            JValue::Bool(true),
        ],
    )?
    .l()
}

/// `image` as a `Bitmap` of its own size.
fn bitmap<'local>(
    env: &mut Env<'local>,
    image: &IconImage,
) -> jni::errors::Result<JObject<'local>> {
    let colors = argb_colors(&image.rgba);
    let pixels = JIntArray::new(env, colors.len())?;
    pixels.set_region(env, 0, &colors)?;
    let argb = env
        .get_static_field(
            jni_str!("android/graphics/Bitmap$Config"),
            jni_str!("ARGB_8888"),
            jni_sig!("Landroid/graphics/Bitmap$Config;"),
        )?
        .l()?;
    env.call_static_method(
        jni_str!("android/graphics/Bitmap"),
        jni_str!("createBitmap"),
        jni_sig!("([IIILandroid/graphics/Bitmap$Config;)Landroid/graphics/Bitmap;"),
        &[
            JValue::Object(&pixels),
            JValue::Int(image.width as jint),
            JValue::Int(image.height as jint),
            JValue::Object(&argb),
        ],
    )?
    .l()
}
