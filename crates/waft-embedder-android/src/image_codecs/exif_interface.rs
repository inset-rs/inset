//! Android's own EXIF reader, the framework's `android.media.ExifInterface`, reached over JNI from
//! the codecs' threads, each attached to the app's VM through the application
//! (`Application::with`) the first time it reads one, for the rest of the thread's life, as JNI
//! detaches a thread as it ends: the image's tags, by name, as `getAttribute` answers them
//! (`exif`). A Java exception is caught and cleared there, and the image fails to open. A file is read by its path; bytes in memory through a
//! `ByteArrayInputStream` over a copy of them in Java's heap.
//!
//! `ExifInterface` reads JPEG, PNG, WebP, HEIF, DNG and other raw formats, and AVIF where the
//! device's version does; in a format it does not read, or a file with no EXIF, it finds no tag,
//! and the image is upright with no metadata, which is what such a file says.

use jni::objects::{JObject, JString, JValue};
use jni::{Env, jni_sig, jni_str};
use waft_embedder::image_codecs::{ImageError, ImageErrorKind};

use super::decoding::Encoded;
use super::exif::{ExifTags, TAGS};
use crate::application::Application;

/// The tags of the image `encoded` holds.
pub(super) fn tags_of(
    application: &Application,
    encoded: &Encoded,
) -> Result<ExifTags, ImageError> {
    application
        .with(|env, _| {
            let exif = exif_interface(env, encoded)?;
            read_tags(env, &exif)
        })
        .map_err(|error| {
            ImageError::new(
                ImageErrorKind::Other,
                format!("ExifInterface could not read the image: {error}"),
            )
        })
}

/// An `ExifInterface` over the image's file or bytes.
fn exif_interface<'local>(
    env: &mut Env<'local>,
    encoded: &Encoded,
) -> jni::errors::Result<JObject<'local>> {
    match encoded {
        Encoded::Bytes(bytes) => {
            let array = env.byte_array_from_slice(bytes)?;
            let stream = env.new_object(
                jni_str!("java/io/ByteArrayInputStream"),
                jni_sig!("([B)V"),
                &[JValue::Object(&array)],
            )?;
            env.new_object(
                jni_str!("android/media/ExifInterface"),
                jni_sig!("(Ljava/io/InputStream;)V"),
                &[JValue::Object(&stream)],
            )
        }
        Encoded::File(path) => {
            let path = env.new_string(path.to_string_lossy())?;
            env.new_object(
                jni_str!("android/media/ExifInterface"),
                jni_sig!("(Ljava/lang/String;)V"),
                &[JValue::Object(&path)],
            )
        }
    }
}

/// Each of [`TAGS`] the image has, with its text.
fn read_tags(env: &mut Env<'_>, exif: &JObject<'_>) -> jni::errors::Result<ExifTags> {
    let mut tags = ExifTags::new();
    for tag in TAGS {
        let name = env.new_string(tag)?;
        let value = env
            .call_method(
                exif,
                jni_str!("getAttribute"),
                jni_sig!("(Ljava/lang/String;)Ljava/lang/String;"),
                &[JValue::Object(&name)],
            )?
            .l()?;
        if !value.is_null() {
            let value = env.cast_local::<JString>(value)?.try_to_string(env)?;
            tags.insert(tag, value);
        }
    }
    Ok(tags)
}
