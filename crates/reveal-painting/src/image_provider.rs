//! Flutter counterpart: `painting/image_provider.dart` (`ImageConfiguration`)
//! and the load surface of `services/asset_bundle.dart`.
//!
//! Flutter's `ImageProvider` / `ImageStream` / `ImageCache` stack is deferred.

use std::fmt::{self, Debug};
use std::rc::Rc;

use reveal_embedder::{Locale, Size, TargetPlatform};

use crate::basic_types::TextDirection;

/// Bytes for a named asset.
///
/// Flutter `AssetBundle.load` is a `Future<ByteData>` plus string/structured
/// helpers, caching, and network fallback. This is the surface a host or test
/// can implement completely; async decode, resolution-aware variants, and the
/// image cache are deferred.
pub trait AssetBundle: Debug {
    /// Retrieve a binary resource. Throws in Dart if the asset is not found;
    /// here that is [`Err`].
    fn load(&self, key: &str) -> Result<Vec<u8>, String>;
}

/// Configuration information passed to an image loader.
///
/// All the arguments are optional. Configuration information is merely
/// advisory and best-effort.
pub struct ImageConfiguration {
    /// The preferred [`AssetBundle`] to use if the loader needs one and does
    /// not have one already selected.
    pub bundle: Option<Rc<dyn AssetBundle>>,
    /// The device pixel ratio where the image will be shown.
    pub device_pixel_ratio: Option<f64>,
    /// The language and region for which to select the image.
    pub locale: Option<Locale>,
    /// The reading direction of the language for which to select the image.
    pub text_direction: Option<TextDirection>,
    /// The size at which the image will be rendered.
    pub size: Option<Size>,
    /// The [`TargetPlatform`] for which assets should be used.
    pub platform: Option<TargetPlatform>,
}

impl ImageConfiguration {
    /// An image configuration that provides no additional information.
    ///
    /// Useful when resolving an image without any context.
    pub const EMPTY: ImageConfiguration = ImageConfiguration {
        bundle: None,
        device_pixel_ratio: None,
        locale: None,
        text_direction: None,
        size: None,
        platform: None,
    };

    /// Creates an object holding the configuration information for an image
    /// loader.
    pub fn new(
        bundle: Option<Rc<dyn AssetBundle>>,
        device_pixel_ratio: Option<f64>,
        locale: Option<Locale>,
        text_direction: Option<TextDirection>,
        size: Option<Size>,
        platform: Option<TargetPlatform>,
    ) -> ImageConfiguration {
        ImageConfiguration {
            bundle,
            device_pixel_ratio,
            locale,
            text_direction,
            size,
            platform,
        }
    }

    /// Creates an object holding the configuration information for an image
    /// loader.
    ///
    /// All the arguments are optional. Configuration information is merely
    /// advisory and best-effort.
    pub fn copy_with(
        &self,
        bundle: Option<Rc<dyn AssetBundle>>,
        device_pixel_ratio: Option<f64>,
        locale: Option<Locale>,
        text_direction: Option<TextDirection>,
        size: Option<Size>,
        platform: Option<TargetPlatform>,
    ) -> ImageConfiguration {
        ImageConfiguration {
            bundle: bundle.or_else(|| self.bundle.clone()),
            device_pixel_ratio: device_pixel_ratio.or(self.device_pixel_ratio),
            locale: locale.or_else(|| self.locale.clone()),
            text_direction: text_direction.or(self.text_direction),
            size: size.or(self.size),
            platform: platform.or(self.platform),
        }
    }
}

impl Default for ImageConfiguration {
    fn default() -> ImageConfiguration {
        ImageConfiguration::EMPTY
    }
}

impl Clone for ImageConfiguration {
    fn clone(&self) -> ImageConfiguration {
        ImageConfiguration {
            bundle: self.bundle.clone(),
            device_pixel_ratio: self.device_pixel_ratio,
            locale: self.locale.clone(),
            text_direction: self.text_direction,
            size: self.size,
            platform: self.platform,
        }
    }
}

impl PartialEq for ImageConfiguration {
    fn eq(&self, other: &ImageConfiguration) -> bool {
        bundles_eq(self.bundle.as_ref(), other.bundle.as_ref())
            && self.device_pixel_ratio == other.device_pixel_ratio
            && self.locale == other.locale
            && self.text_direction == other.text_direction
            && self.size == other.size
            && self.platform == other.platform
    }
}

fn bundles_eq(a: Option<&Rc<dyn AssetBundle>>, b: Option<&Rc<dyn AssetBundle>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

impl Debug for ImageConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fields = Vec::new();
        if self.bundle.is_some() {
            fields.push(format!("bundle: {:?}", self.bundle));
        }
        if let Some(dpr) = self.device_pixel_ratio {
            fields.push(format!("devicePixelRatio: {dpr:.1}"));
        }
        if let Some(locale) = &self.locale {
            fields.push(format!("locale: {locale}"));
        }
        if let Some(text_direction) = self.text_direction {
            fields.push(format!("textDirection: {text_direction:?}"));
        }
        if let Some(size) = self.size {
            fields.push(format!("size: {size:?}"));
        }
        if let Some(platform) = self.platform {
            fields.push(format!("platform: {platform:?}"));
        }
        write!(f, "ImageConfiguration({})", fields.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reveal_embedder::TargetPlatform;

    use crate::basic_types::TextDirection;

    #[derive(Debug)]
    struct MemoryBundle {
        bytes: Vec<u8>,
    }

    impl AssetBundle for MemoryBundle {
        fn load(&self, _key: &str) -> Result<Vec<u8>, String> {
            Ok(self.bytes.clone())
        }
    }

    #[test]
    fn empty_and_copy_with() {
        let empty = ImageConfiguration::EMPTY;
        assert_eq!(empty, ImageConfiguration::default());
        let sized = empty.copy_with(
            None,
            Some(2.0),
            Some(Locale::new("ar")),
            Some(TextDirection::Rtl),
            Some(Size::new(10.0, 20.0)),
            Some(TargetPlatform::IOS),
        );
        assert_eq!(sized.device_pixel_ratio, Some(2.0));
        assert_eq!(sized.locale, Some(Locale::new("ar")));
        assert_eq!(sized.text_direction, Some(TextDirection::Rtl));
        assert_eq!(sized.size, Some(Size::new(10.0, 20.0)));
        assert_eq!(sized.platform, Some(TargetPlatform::IOS));
        assert!(sized.bundle.is_none());
    }

    #[test]
    fn bundle_identity() {
        let a: Rc<dyn AssetBundle> = Rc::new(MemoryBundle {
            bytes: b"one".to_vec(),
        });
        let b: Rc<dyn AssetBundle> = Rc::new(MemoryBundle {
            bytes: b"one".to_vec(),
        });
        let left = ImageConfiguration::new(Some(a.clone()), None, None, None, None, None);
        let same = ImageConfiguration::new(Some(a), None, None, None, None, None);
        let other = ImageConfiguration::new(Some(b), None, None, None, None, None);
        assert_eq!(left, same);
        assert_ne!(left, other);
        assert_eq!(left.bundle.as_ref().unwrap().load("k").unwrap(), b"one");
    }
}
