//! Flutter `rendering/image_filter_config.dart`.

use std::rc::Rc;

use inset_embedder::{ImageFilter, Rect};

/// Contextual information used when resolving an [`ImageFilterConfig`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageFilterContext {
    /// The bounds of the render object the filter is applied to.
    pub bounds: Rect,
}

/// A description of an [`ImageFilter`] that is resolved against an [`ImageFilterContext`]
/// at paint time.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ImageFilterConfig {
    /// A configuration that always resolves to the given filter (Dart's default constructor).
    Direct(ImageFilter),
    /// A Gaussian blur, optionally bounded to the render object's bounds.
    Blur {
        /// The standard deviation along the horizontal axis.
        sigma_x: f64,
        /// The standard deviation along the vertical axis.
        sigma_y: f64,
        /// Whether the blur is confined to the context's bounds.
        bounded: bool,
    },
    /// `outer` applied to the result of `inner`; Dart's `ImageFilterConfig.compose`.
    Compose {
        /// The configuration that receives the filtered result of `inner`.
        outer: Rc<ImageFilterConfig>,
        /// The configuration that receives the original input.
        inner: Rc<ImageFilterConfig>,
    },
}

impl ImageFilterConfig {
    /// Creates a configuration that resolves to `filter` unchanged.
    ///
    /// This is also how a [`inset_embedder::ColorFilter`] is wrapped, since Dart's
    /// `ColorFilter` is itself a `ui.ImageFilter`: pass `ImageFilter::Color(filter)`.
    pub const fn new(filter: ImageFilter) -> ImageFilterConfig {
        ImageFilterConfig::Direct(filter)
    }

    /// Creates a blur configuration; Dart's named arguments are the setters.
    pub const fn blur() -> ImageFilterConfig {
        ImageFilterConfig::Blur {
            sigma_x: 0.0,
            sigma_y: 0.0,
            bounded: false,
        }
    }

    /// Composes the `inner` configuration with `outer`, to combine their effects.
    ///
    /// Creates a single [`ImageFilterConfig`] that when applied, has the same effect as
    /// subsequently applying `inner` and `outer`, i.e. `result = outer(inner(source))`.
    pub fn compose(outer: ImageFilterConfig, inner: ImageFilterConfig) -> ImageFilterConfig {
        ImageFilterConfig::Compose {
            outer: Rc::new(outer),
            inner: Rc::new(inner),
        }
    }

    /// Dart `ImageFilterConfig.blur(sigmaX:)`.
    pub fn sigma_x(self, value: f64) -> ImageFilterConfig {
        match self {
            ImageFilterConfig::Blur {
                sigma_y, bounded, ..
            } => ImageFilterConfig::Blur {
                sigma_x: value,
                sigma_y,
                bounded,
            },
            other => other,
        }
    }

    /// Dart `ImageFilterConfig.blur(sigmaY:)`.
    pub fn sigma_y(self, value: f64) -> ImageFilterConfig {
        match self {
            ImageFilterConfig::Blur {
                sigma_x, bounded, ..
            } => ImageFilterConfig::Blur {
                sigma_x,
                sigma_y: value,
                bounded,
            },
            other => other,
        }
    }

    /// Dart `ImageFilterConfig.blur(bounded:)`.
    pub fn bounded(self, value: bool) -> ImageFilterConfig {
        match self {
            ImageFilterConfig::Blur {
                sigma_x, sigma_y, ..
            } => ImageFilterConfig::Blur {
                sigma_x,
                sigma_y,
                bounded: value,
            },
            other => other,
        }
    }

    /// Resolves this configuration into an [`ImageFilter`] for the given context.
    pub fn resolve(&self, context: ImageFilterContext) -> ImageFilter {
        match self {
            ImageFilterConfig::Direct(filter) => filter.clone(),
            ImageFilterConfig::Blur {
                sigma_x,
                sigma_y,
                bounded,
            } => {
                let filter = ImageFilter::blur(*sigma_x, *sigma_y);
                if *bounded {
                    filter.bounds(context.bounds)
                } else {
                    filter
                }
            }
            ImageFilterConfig::Compose { outer, inner } => {
                ImageFilter::compose(outer.resolve(context), inner.resolve(context))
            }
        }
    }

    /// The filter this configuration was created from, for a direct configuration.
    pub fn filter(&self) -> Option<&ImageFilter> {
        match self {
            ImageFilterConfig::Direct(filter) => Some(filter),
            ImageFilterConfig::Blur { .. } | ImageFilterConfig::Compose { .. } => None,
        }
    }

    /// A short description for debugging.
    pub fn debug_short_description(&self) -> String {
        match self {
            ImageFilterConfig::Direct(filter) => filter.debug_short_description(),
            ImageFilterConfig::Blur {
                sigma_x,
                sigma_y,
                bounded,
            } => {
                let bounded = if *bounded { "bounded" } else { "unbounded" };
                format!("blur({sigma_x}, {sigma_y}, {bounded})")
            }
            ImageFilterConfig::Compose { outer, inner } => format!(
                "{} -> {}",
                inner.debug_short_description(),
                outer.debug_short_description()
            ),
        }
    }
}

impl std::fmt::Display for ImageFilterConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageFilterConfig::Direct(filter) => {
                write!(f, "ImageFilterConfig({})", filter.debug_short_description())
            }
            ImageFilterConfig::Blur { .. } => {
                write!(f, "ImageFilterConfig.{}", self.debug_short_description())
            }
            ImageFilterConfig::Compose { .. } => write!(
                f,
                "ImageFilterConfig.compose(source -> {} -> result)",
                self.debug_short_description()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use inset_embedder::{BlendMode, Color, ColorFilter};

    use super::*;

    const CONTEXT: ImageFilterContext = ImageFilterContext {
        bounds: Rect::from_ltwh(0.0, 0.0, 10.0, 20.0),
    };

    #[test]
    fn a_bounded_blur_resolves_against_the_context_bounds() {
        let config = ImageFilterConfig::blur().sigma_x(3.0).sigma_y(4.0);
        assert_eq!(
            config.clone().resolve(CONTEXT),
            ImageFilter::blur(3.0, 4.0),
            "an unbounded blur carries no bounds"
        );
        assert_eq!(
            config.bounded(true).resolve(CONTEXT),
            ImageFilter::blur(3.0, 4.0).bounds(CONTEXT.bounds)
        );
    }

    #[test]
    fn compose_resolves_both_sides_and_only_a_direct_config_reports_its_filter() {
        let saturation = ImageFilter::Color(ColorFilter::saturation(1.8));
        let config = ImageFilterConfig::compose(
            ImageFilterConfig::new(saturation.clone()),
            ImageFilterConfig::blur().sigma_x(5.0).bounded(true),
        );
        assert_eq!(
            config.resolve(CONTEXT),
            ImageFilter::compose(
                saturation,
                ImageFilter::blur(5.0, 0.0).bounds(CONTEXT.bounds),
            )
        );
        assert_eq!(config.filter(), None);
        let direct = ImageFilterConfig::new(ImageFilter::blur(1.0, 1.0));
        assert_eq!(direct.filter(), Some(&ImageFilter::blur(1.0, 1.0)));
    }

    #[test]
    fn a_composed_description_reads_inner_then_outer() {
        let config = ImageFilterConfig::compose(
            ImageFilterConfig::new(ImageFilter::Color(ColorFilter::mode(
                Color::new(0xFF112233),
                BlendMode::SrcOver,
            ))),
            ImageFilterConfig::blur().sigma_x(5.0).sigma_y(5.0),
        );
        assert_eq!(
            config.debug_short_description(),
            "blur(5, 5, unbounded) -> ColorFilter.mode(Color(alpha: 1.0000, red: 0.0667, \
             green: 0.1333, blue: 0.2000, colorSpace: ColorSpace.sRGB), SrcOver)"
        );
    }
}
