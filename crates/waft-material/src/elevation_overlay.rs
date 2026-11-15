//! Flutter counterpart: material_ui's `elevation_overlay.dart`.

use waft_embedder::{Brightness, Color};
use waft_foundation::App;
use waft_widgets::BuildContext;

use crate::colors::Colors;
use crate::theme::Theme;

/// A utility class for dealing with the overlay color needed
/// to indicate elevation of surfaces.
///
/// Dart's `abstract final class ElevationOverlay`: it is never constructed, its members are
/// associated functions.
pub struct ElevationOverlay;

impl ElevationOverlay {
    /// Applies a surface tint color to a given container color to indicate
    /// the level of its elevation.
    ///
    /// With Material Design 3, some components will use a "surface tint" color
    /// overlay with an opacity applied to their base color to indicate they are
    /// elevated. The amount of opacity will vary with the elevation as described
    /// in: <https://m3.material.io/styles/color/the-color-system/color-roles>.
    ///
    /// If `surface_tint` is not null and not completely transparent (`Color::alpha`
    /// is 0), then the returned color will be the given `color` with the
    /// `surface_tint` of the appropriate opacity applied to it. Otherwise it will
    /// just return `color` unmodified.
    #[allow(deprecated)]
    pub fn apply_surface_tint(color: Color, surface_tint: Option<Color>, elevation: f64) -> Color {
        if let Some(surface_tint) = surface_tint
            && surface_tint != Colors::TRANSPARENT
        {
            return Color::alpha_blend(
                surface_tint.with_opacity(Self::surface_tint_opacity_for_elevation(elevation)),
                color,
            );
        }
        color
    }

    // Calculates the opacity of the surface tint color from the elevation by
    // looking it up in the legacy table of opacities, interpolating
    // between values as needed. If the elevation is outside the range of values
    // in the table it will clamp to the smallest or largest opacity.
    fn surface_tint_opacity_for_elevation(elevation: f64) -> f64 {
        if elevation < SURFACE_TINT_ELEVATION_OPACITIES[0].elevation {
            // Elevation less than the first entry, so just clamp it to the first one.
            return SURFACE_TINT_ELEVATION_OPACITIES[0].opacity;
        }

        // Walk the opacity list and find the closest match(es) for the elevation.
        let mut index = 0;
        while elevation >= SURFACE_TINT_ELEVATION_OPACITIES[index].elevation {
            // If we found it exactly or walked off the end of the list just return it.
            if elevation == SURFACE_TINT_ELEVATION_OPACITIES[index].elevation
                || index + 1 == SURFACE_TINT_ELEVATION_OPACITIES.len()
            {
                return SURFACE_TINT_ELEVATION_OPACITIES[index].opacity;
            }
            index += 1;
        }

        // Interpolate between the two opacity values
        let lower = &SURFACE_TINT_ELEVATION_OPACITIES[index - 1];
        let upper = &SURFACE_TINT_ELEVATION_OPACITIES[index];
        let t = (elevation - lower.elevation) / (upper.elevation - lower.elevation);
        lower.opacity + t * (upper.opacity - lower.opacity)
    }

    /// Applies an overlay color to a surface color to indicate
    /// the level of its elevation in a dark theme.
    ///
    /// If using Material Design 3, this type of color overlay is no longer used.
    /// Instead a "surface tint" overlay is used instead. See [`apply_surface_tint`](Self::apply_surface_tint),
    /// `ThemeData::use_material3` for more information.
    ///
    /// Material drop shadows can be difficult to see in a dark theme, so the
    /// elevation of a surface should be portrayed with an "overlay" in addition
    /// to the shadow. As the elevation of the component increases, the
    /// overlay increases in opacity. This function computes and applies this
    /// overlay to a given color as needed.
    ///
    /// If the ambient theme is dark (`ThemeData::brightness` is [`Brightness::Dark`]),
    /// and `ThemeData::apply_elevation_overlay_color` is true, and the given
    /// `color` is `ColorScheme::surface` then this will return a version of
    /// the `color` with a semi-transparent `ColorScheme::on_surface` overlaid
    /// on top of it. The opacity of the overlay is computed based on the
    /// `elevation`.
    ///
    /// Otherwise it will just return the `color` unmodified.
    ///
    /// See also:
    ///
    ///  * `ThemeData::apply_elevation_overlay_color` which controls the whether
    ///    an overlay color will be applied to indicate elevation.
    ///  * [`overlay_color`](Self::overlay_color) which computes the needed overlay color.
    ///  * `Material` which uses this to apply an elevation overlay to its surface.
    ///  * <https://material.io/design/color/dark-theme.html>, which specifies how
    ///    the overlay should be applied.
    #[allow(deprecated)]
    pub fn apply_overlay(
        app: &mut App,
        context: BuildContext,
        color: Color,
        elevation: f64,
    ) -> Color {
        let theme = Theme::of(app, context);
        if elevation > 0.0
            && theme.apply_elevation_overlay_color
            && theme.brightness() == Brightness::Dark
            && color.with_opacity(1.0) == theme.color_scheme.surface.with_opacity(1.0)
        {
            return Self::color_with_overlay(color, theme.color_scheme.on_surface, elevation);
        }
        color
    }

    /// Computes the appropriate overlay color used to indicate elevation in
    /// dark themes.
    ///
    /// If using Material Design 3, this type of color overlay is no longer used.
    /// Instead a "surface tint" overlay is used instead. See [`apply_surface_tint`](Self::apply_surface_tint),
    /// `ThemeData::use_material3` for more information.
    ///
    /// See also:
    ///
    ///  * <https://material.io/design/color/dark-theme.html#properties> which
    ///    specifies the exact overlay values for a given elevation.
    pub fn overlay_color(app: &mut App, context: BuildContext, elevation: f64) -> Color {
        let theme = Theme::of(app, context);
        Self::overlay_color_of(theme.color_scheme.on_surface, elevation)
    }

    /// Returns a color blended by laying a semi-transparent overlay (using the
    /// `overlay` color) on top of a surface (using the `surface` color).
    ///
    /// If using Material Design 3, this type of color overlay is no longer used.
    /// Instead a "surface tint" overlay is used instead. See [`apply_surface_tint`](Self::apply_surface_tint),
    /// `ThemeData::use_material3` for more information.
    ///
    /// The opacity of the overlay depends on `elevation`. As `elevation`
    /// increases, the opacity will also increase.
    ///
    /// See <https://material.io/design/color/dark-theme.html#properties>.
    pub fn color_with_overlay(surface: Color, overlay: Color, elevation: f64) -> Color {
        Color::alpha_blend(Self::overlay_color_of(overlay, elevation), surface)
    }

    /// Dart's private `_overlayColor`: applies an opacity to `color` based on `elevation`.
    #[allow(deprecated)]
    fn overlay_color_of(color: Color, elevation: f64) -> Color {
        // Compute the opacity for the given elevation
        // This formula matches the values in the spec:
        // https://material.io/design/color/dark-theme.html#properties
        let opacity = (4.5 * (elevation + 1.0).ln() + 2.0) / 100.0;
        color.with_opacity(opacity)
    }
}

/// Dart's private `_ElevationOpacity`: a data class to hold the opacity at a given elevation.
struct ElevationOpacity {
    elevation: f64,
    opacity: f64,
}

impl ElevationOpacity {
    const fn new(elevation: f64, opacity: f64) -> ElevationOpacity {
        ElevationOpacity { elevation, opacity }
    }
}

// Surface tint is no longer recommended by the Material 3 specification, but
// these values are preserved for backwards compatibility.
const SURFACE_TINT_ELEVATION_OPACITIES: [ElevationOpacity; 6] = [
    ElevationOpacity::new(0.0, 0.0),   // Elevation level 0
    ElevationOpacity::new(1.0, 0.05),  // Elevation level 1
    ElevationOpacity::new(3.0, 0.08),  // Elevation level 2
    ElevationOpacity::new(6.0, 0.11),  // Elevation level 3
    ElevationOpacity::new(8.0, 0.12),  // Elevation level 4
    ElevationOpacity::new(12.0, 0.14), // Elevation level 5
];

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use waft_widgets::IntoWidget;

    use super::*;
    use crate::test_support::{probe, test_cell, with_main_thread_stack};
    use crate::theme_data::ThemeData;

    const BASE_COLOR: Color = Color::new(0xff888888);
    const SURFACE_TINT_COLOR: Color = Color::new(0xff44ccff);

    #[allow(deprecated)]
    fn overlay_with_opacity(opacity: f64) -> Color {
        Color::alpha_blend(SURFACE_TINT_COLOR.with_opacity(opacity), BASE_COLOR)
    }

    fn tinted(elevation: f64) -> Color {
        ElevationOverlay::apply_surface_tint(BASE_COLOR, Some(SURFACE_TINT_COLOR), elevation)
    }

    #[test]
    fn apply_surface_tint_with_null_surface_tint_returns_given_color() {
        let result = ElevationOverlay::apply_surface_tint(BASE_COLOR, None, 42.0);

        assert_eq!(result, Color::new(0xFF888888));
    }

    #[test]
    fn apply_surface_tint_with_exact_elevation_levels_uses_the_right_opacity_overlay() {
        // Elevation level 0 (0.0) - should have opacity 0.0.
        assert_eq!(tinted(0.0), overlay_with_opacity(0.0));
        // Elevation level 1 (1.0) - should have opacity 0.05.
        assert_eq!(tinted(1.0), overlay_with_opacity(0.05));
        // Elevation level 2 (3.0) - should have opacity 0.08.
        assert_eq!(tinted(3.0), overlay_with_opacity(0.08));
        // Elevation level 3 (6.0) - should have opacity 0.11`.
        assert_eq!(tinted(6.0), overlay_with_opacity(0.11));
        // Elevation level 4 (8.0) - should have opacity 0.12.
        assert_eq!(tinted(8.0), overlay_with_opacity(0.12));
        // Elevation level 5 (12.0) - should have opacity 0.14.
        assert_eq!(tinted(12.0), overlay_with_opacity(0.14));
    }

    #[test]
    fn apply_surface_tint_with_elevation_lower_than_level_0_should_have_no_overlay() {
        assert_eq!(tinted(-42.0), overlay_with_opacity(0.0));
    }

    #[test]
    fn apply_surface_tint_with_elevation_higher_than_level_5_should_have_no_level_5_overlay() {
        // Elevation level 5 (12.0) - should have opacity 0.14.
        assert_eq!(tinted(42.0), overlay_with_opacity(0.14));
    }

    #[test]
    fn apply_surface_tint_with_elevation_between_two_levels_should_interpolate_the_opacity() {
        // Elevation between level 4 (8.0) and level 5 (12.0) should be interpolated
        // between the opacities 0.12 and 0.14.

        // One third (0.3): (elevation 9.2) -> (opacity 0.126)
        assert_eq!(tinted(9.2), overlay_with_opacity(0.126));
        // Half way (0.5): (elevation 10.0) -> (opacity 0.13)
        assert_eq!(tinted(10.0), overlay_with_opacity(0.13));
        // Two thirds (0.6): (elevation 10.4) -> (opacity 0.132)
        assert_eq!(tinted(10.4), overlay_with_opacity(0.132));
    }

    #[test]
    fn apply_surface_tint_with_a_transparent_surface_tint_returns_given_color() {
        let result =
            ElevationOverlay::apply_surface_tint(BASE_COLOR, Some(Colors::TRANSPARENT), 6.0);

        assert_eq!(result, BASE_COLOR);
    }

    fn dark_theme_with_overlay(app: &App) -> ThemeData {
        ThemeData::builder()
            .brightness(Brightness::Dark)
            .use_material3(false)
            .apply_elevation_overlay_color(true)
            .build(app)
    }

    fn overlay_results_under(
        theme_of: impl Fn(&App) -> ThemeData + 'static,
        color_of: impl Fn(&ThemeData) -> Color + 'static,
        elevation: f64,
    ) -> (Color, Color, Rc<ThemeData>) {
        let cell = test_cell();
        probe(
            &cell,
            move |app, child| Theme::new(theme_of(app), child).into_widget(),
            move |app, context| {
                let theme = Theme::of(app, context);
                let color = color_of(&theme);
                (
                    ElevationOverlay::apply_overlay(app, context, color, elevation),
                    ElevationOverlay::overlay_color(app, context, elevation),
                    theme,
                )
            },
        )
    }

    #[test]
    fn apply_overlay_overlays_the_surface_color_in_a_dark_theme() {
        with_main_thread_stack(|| {
            let (applied, overlay, theme) = overlay_results_under(
                dark_theme_with_overlay,
                |theme| theme.color_scheme.surface,
                4.0,
            );

            let on_surface = theme.color_scheme.on_surface;
            assert_eq!(
                applied,
                ElevationOverlay::color_with_overlay(theme.color_scheme.surface, on_surface, 4.0)
            );
            assert_ne!(applied, theme.color_scheme.surface);
            assert_eq!(overlay, ElevationOverlay::overlay_color_of(on_surface, 4.0));
        });
    }

    #[test]
    fn apply_overlay_leaves_other_colors_and_light_themes_and_zero_elevation_alone() {
        with_main_thread_stack(|| {
            let (other_color, _, theme) =
                overlay_results_under(dark_theme_with_overlay, |_| Color::new(0xff123456), 4.0);
            assert_eq!(other_color, Color::new(0xff123456));
            assert_eq!(theme.brightness(), Brightness::Dark);

            let (light, _, light_theme) = overlay_results_under(
                |app| ThemeData::light(app, Some(false)),
                |theme| theme.color_scheme.surface,
                4.0,
            );
            assert_eq!(light, light_theme.color_scheme.surface);

            let (flat, _, dark_theme) = overlay_results_under(
                dark_theme_with_overlay,
                |theme| theme.color_scheme.surface,
                0.0,
            );
            assert_eq!(flat, dark_theme.color_scheme.surface);
        });
    }

    #[test]
    #[allow(deprecated)]
    fn color_with_overlay_uses_the_dark_theme_opacity_curve() {
        let opacity = (4.5 * 2.0_f64.ln() + 2.0) / 100.0;

        assert_eq!(
            ElevationOverlay::color_with_overlay(BASE_COLOR, SURFACE_TINT_COLOR, 1.0),
            Color::alpha_blend(SURFACE_TINT_COLOR.with_opacity(opacity), BASE_COLOR)
        );
    }
}
