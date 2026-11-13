//! An icon image as Android makes a `Bitmap` of it: non-premultiplied ARGB colours row by row,
//! alpha in the top byte, as `Bitmap.createBitmap(int[], int, int, Bitmap.Config.ARGB_8888)` takes
//! them. A notification's image and a task's icon are both made so.

use jni::Env;
use jni::objects::JIntArray;
use jni::sys::jint;
use waft_embedder::IconImage;

/// `image`'s pixels as a Java `int[]` of the platform's colours.
pub(crate) fn int_array<'local>(
    env: &mut Env<'local>,
    image: &IconImage,
) -> jni::errors::Result<JIntArray<'local>> {
    let colors = argb_colors(&image.rgba);
    let pixels = JIntArray::new(env, colors.len())?;
    pixels.set_region(env, 0, &colors)?;
    Ok(pixels)
}

/// RGBA bytes as the platform's colours, alpha in the top byte.
fn argb_colors(rgba: &[u8]) -> Vec<jint> {
    let (pixels, _) = rgba.as_chunks::<4>();
    pixels
        .iter()
        .map(|&[red, green, blue, alpha]| i32::from_be_bytes([alpha, red, green, blue]))
        .collect()
}
