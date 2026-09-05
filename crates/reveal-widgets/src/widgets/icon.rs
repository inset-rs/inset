//! Flutter counterpart: `widgets/icon.dart`.
//!
//! `Icon.semanticLabel` and the `Semantics` / `ExcludeSemantics` wrappers around the glyph
//! wait on accessibility; `debugCheckHasDirectionality` waits on diagnostics.

use reveal_embedder::{
    BlendMode, FontVariation, FontWeight, Matrix4, Paint, Shadow, TextDirection,
    TextLeadingDistribution,
};
use reveal_foundation::App;
use reveal_painting::{
    AlignmentGeometry, AnyColor, K_DEFAULT_FONT_SIZE, TextOverflow, TextSpan, TextStyle,
};

use crate::framework::{BuildContext, IntoWidget, KeyRef, StatelessWidget, WidgetRef};
use crate::widgets::basic::{Center, Directionality, SizedBox, Transform};
use crate::widgets::icon_data::IconData;
use crate::widgets::icon_theme::IconTheme;
use crate::widgets::media_query::MediaQuery;
use crate::widgets::text::RichText;

/// A graphical icon widget drawn with a glyph from a font described in
/// an [`IconData`] such as material's predefined `IconData`s in `Icons`.
///
/// Icons are not interactive. For an interactive icon, consider material's
/// `IconButton`.
///
/// There must be an ambient [`Directionality`] widget when using [`Icon`].
/// Typically this is introduced automatically by the `WidgetsApp` or
/// `MaterialApp`.
///
/// This widget assumes that the rendered icon is squared. Non-squared icons may
/// render incorrectly.
///
/// This example shows how to create a `Row` of [`Icon`]s in different colors and
/// sizes:
///
/// ```text
/// Row::new().children(vec![
///     Icon::new(Some(Icons::FAVORITE)).color(PINK).size(24.0).into_widget(),
///     Icon::new(Some(Icons::AUDIOTRACK)).color(GREEN).size(30.0).into_widget(),
///     Icon::new(Some(Icons::BEACH_ACCESS)).color(BLUE).size(36.0).into_widget(),
/// ])
/// ```
///
/// See also:
///
///  * `IconButton`, for interactive icons.
///  * `Icons`, for the list of available Material Icons for use with this class.
///  * [`IconTheme`], which provides ambient configuration for icons.
///  * `ImageIcon`, for showing icons from `AssetImage`s or other `ImageProvider`s.
#[derive(Debug)]
pub struct Icon {
    pub key: Option<KeyRef>,
    /// The icon to display. The available icons are described in `Icons`.
    ///
    /// The icon can be `None`, in which case the widget will render as an empty
    /// space of the specified [`size`](Self::size).
    pub icon: Option<IconData>,
    /// The size of the icon in logical pixels.
    ///
    /// Icons occupy a square with width and height equal to size.
    ///
    /// Defaults to the nearest [`IconTheme`]'s `IconThemeData::size`.
    ///
    /// If this [`Icon`] is being placed inside an `IconButton`, then use
    /// `IconButton.iconSize` instead, so that the `IconButton` can make the splash
    /// area the appropriate size as well. The `IconButton` uses an [`IconTheme`] to
    /// pass down the size to the [`Icon`].
    pub size: Option<f64>,
    /// The fill for drawing the icon.
    ///
    /// Requires the underlying icon font to support the `FILL` [`FontVariation`]
    /// axis, otherwise has no effect. Variable font filenames often indicate
    /// the supported axes. Must be between 0.0 (unfilled) and 1.0 (filled),
    /// inclusive.
    ///
    /// Can be used to convey a state transition for animation or interaction.
    ///
    /// Defaults to nearest [`IconTheme`]'s `IconThemeData::fill`.
    ///
    /// See also:
    ///  * [`weight`](Self::weight), for controlling stroke weight.
    ///  * [`grade`](Self::grade), for controlling stroke weight in a more granular way.
    ///  * [`optical_size`](Self::optical_size), for controlling optical size.
    pub fill: Option<f64>,
    /// The stroke weight for drawing the icon.
    ///
    /// Requires the underlying icon font to support the `wght` [`FontVariation`]
    /// axis, otherwise has no effect. Variable font filenames often indicate
    /// the supported axes. Must be greater than 0.
    ///
    /// Defaults to nearest [`IconTheme`]'s `IconThemeData::weight`.
    ///
    /// See also:
    ///  * [`fill`](Self::fill), for controlling fill.
    ///  * [`grade`](Self::grade), for controlling stroke weight in a more granular way.
    ///  * [`optical_size`](Self::optical_size), for controlling optical size.
    pub weight: Option<f64>,
    /// The grade (granular stroke weight) for drawing the icon.
    ///
    /// Requires the underlying icon font to support the `GRAD` [`FontVariation`]
    /// axis, otherwise has no effect. Variable font filenames often indicate
    /// the supported axes. Can be negative.
    ///
    /// Grade and [`weight`](Self::weight) both affect a symbol's stroke weight
    /// (thickness), but grade has a smaller impact on the size of the symbol.
    ///
    /// Grade is also available in some text fonts. One can match grade levels
    /// between text and symbols for a harmonious visual effect. For example, if
    /// the text font has a -25 grade value, the symbols can match it with a
    /// suitable value, say -25.
    ///
    /// Defaults to nearest [`IconTheme`]'s `IconThemeData::grade`.
    ///
    /// See also:
    ///  * [`fill`](Self::fill), for controlling fill.
    ///  * [`weight`](Self::weight), for controlling stroke weight in a less granular way.
    ///  * [`optical_size`](Self::optical_size), for controlling optical size.
    pub grade: Option<f64>,
    /// The optical size for drawing the icon.
    ///
    /// Requires the underlying icon font to support the `opsz` [`FontVariation`]
    /// axis, otherwise has no effect. Variable font filenames often indicate
    /// the supported axes. Must be greater than 0.
    ///
    /// For an icon to look the same at different sizes, the stroke weight
    /// (thickness) must change as the icon size scales. Optical size offers a way
    /// to automatically adjust the stroke weight as icon size changes.
    ///
    /// Defaults to nearest [`IconTheme`]'s `IconThemeData::optical_size`.
    ///
    /// See also:
    ///  * [`fill`](Self::fill), for controlling fill.
    ///  * [`weight`](Self::weight), for controlling stroke weight.
    ///  * [`grade`](Self::grade), for controlling stroke weight in a more granular way.
    pub optical_size: Option<f64>,
    /// The color to use when drawing the icon.
    ///
    /// Defaults to the nearest [`IconTheme`]'s `IconThemeData::color`.
    ///
    /// The color (whether specified explicitly here or obtained from the
    /// [`IconTheme`]) will be further adjusted by the nearest [`IconTheme`]'s
    /// `IconThemeData::opacity`.
    pub color: Option<AnyColor>,
    /// A list of [`Shadow`]s that will be painted underneath the icon.
    ///
    /// Multiple shadows are supported to replicate lighting from multiple light
    /// sources.
    ///
    /// Shadows must be in the same order for [`Icon`] to be considered as
    /// equivalent as order produces differing transparency.
    ///
    /// Defaults to the nearest [`IconTheme`]'s `IconThemeData::shadows`.
    pub shadows: Option<Vec<Shadow>>,
    /// The text direction to use for rendering the icon.
    ///
    /// If this is `None`, the ambient [`Directionality`] is used instead.
    ///
    /// Some icons follow the reading direction. For example, "back" buttons point
    /// left in left-to-right environments and right in right-to-left
    /// environments. Such icons have their [`IconData::match_text_direction`] field
    /// set to true, and the [`Icon`] widget uses the
    /// [`text_direction`](Self::text_direction) to determine the orientation in which to draw
    /// the icon.
    ///
    /// This property has no effect if the [`icon`](Self::icon)'s
    /// [`IconData::match_text_direction`] field is false, but for consistency a text direction
    /// value must always be specified, either directly using this property or using
    /// [`Directionality`].
    pub text_direction: Option<TextDirection>,
    /// Whether to scale the size of this widget using the ambient [`MediaQuery`]'s
    /// `TextScaler`.
    ///
    /// This is specially useful when you have an icon associated with a text, as
    /// scaling the text without scaling the icon would result in a confusing
    /// interface.
    ///
    /// Defaults to the nearest [`IconTheme`]'s `IconThemeData::apply_text_scaling`.
    pub apply_text_scaling: Option<bool>,
    /// The [`BlendMode`] to apply to the foreground of the icon.
    ///
    /// Defaults to [`BlendMode::SrcOver`].
    pub blend_mode: Option<BlendMode>,
    /// The typeface thickness to use when painting the text (e.g., bold).
    pub font_weight: Option<FontWeight>,
}

impl Icon {
    /// Creates an icon; Dart's optional named arguments are the setters.
    pub fn new(icon: Option<IconData>) -> Icon {
        Icon {
            key: None,
            icon,
            size: None,
            fill: None,
            weight: None,
            grade: None,
            optical_size: None,
            color: None,
            shadows: None,
            text_direction: None,
            apply_text_scaling: None,
            blend_mode: None,
            font_weight: None,
        }
    }

    /// Dart `Icon(key:)`.
    pub fn key(mut self, key: KeyRef) -> Icon {
        self.key = Some(key);
        self
    }

    /// Dart `Icon(size:)`.
    pub fn size(mut self, size: f64) -> Icon {
        self.size = Some(size);
        self
    }

    /// Dart `Icon(fill:)`; must be between 0.0 and 1.0, inclusive.
    pub fn fill(mut self, fill: f64) -> Icon {
        debug_assert!((0.0..=1.0).contains(&fill));
        self.fill = Some(fill);
        self
    }

    /// Dart `Icon(weight:)`; must be greater than 0.
    pub fn weight(mut self, weight: f64) -> Icon {
        debug_assert!(weight > 0.0);
        self.weight = Some(weight);
        self
    }

    /// Dart `Icon(grade:)`.
    pub fn grade(mut self, grade: f64) -> Icon {
        self.grade = Some(grade);
        self
    }

    /// Dart `Icon(opticalSize:)`; must be greater than 0.
    pub fn optical_size(mut self, optical_size: f64) -> Icon {
        debug_assert!(optical_size > 0.0);
        self.optical_size = Some(optical_size);
        self
    }

    /// Dart `Icon(color:)`.
    pub fn color(mut self, color: impl Into<AnyColor>) -> Icon {
        self.color = Some(color.into());
        self
    }

    /// Dart `Icon(shadows:)`.
    pub fn shadows(mut self, shadows: Vec<Shadow>) -> Icon {
        self.shadows = Some(shadows);
        self
    }

    /// Dart `Icon(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> Icon {
        self.text_direction = Some(text_direction);
        self
    }

    /// Dart `Icon(applyTextScaling:)`.
    pub fn apply_text_scaling(mut self, apply_text_scaling: bool) -> Icon {
        self.apply_text_scaling = Some(apply_text_scaling);
        self
    }

    /// Dart `Icon(blendMode:)`.
    pub fn blend_mode(mut self, blend_mode: BlendMode) -> Icon {
        self.blend_mode = Some(blend_mode);
        self
    }

    /// Dart `Icon(fontWeight:)`.
    pub fn font_weight(mut self, font_weight: FontWeight) -> Icon {
        self.font_weight = Some(font_weight);
        self
    }

    /// The glyph's `TextStyle`: the variable-font axes, the resolved color or foreground
    /// paint, and the metrics that centre the body in the icon's square.
    fn font_style(&self, icon: &IconData, resolved: ResolvedIcon) -> TextStyle {
        let axes = [
            ("FILL", resolved.fill),
            ("wght", resolved.weight),
            ("GRAD", resolved.grade),
            ("opsz", resolved.optical_size),
        ];
        let font_variations = axes
            .into_iter()
            .filter_map(|(axis, value)| value.map(|value| FontVariation::new(axis, value)))
            .collect();
        let mut style = TextStyle::new()
            .font_variations(font_variations)
            .inherit(false)
            .font_size(resolved.size)
            // Makes sure the font's body is vertically centered within the
            // size x size square.
            .height(1.0)
            .leading_distribution(TextLeadingDistribution::Even);
        if let Some(color) = resolved.color {
            style = style.color(color);
        }
        if let Some(font_family) = &icon.font_family {
            style = style.font_family(font_family.clone());
        }
        if let Some(font_weight) = self.font_weight {
            style = style.font_weight(font_weight);
        }
        if let Some(font_package) = &icon.font_package {
            style = style.package(font_package.clone());
        }
        if let Some(font_family_fallback) = &icon.font_family_fallback {
            style = style.font_family_fallback(font_family_fallback.clone());
        }
        if let Some(shadows) = resolved.shadows {
            style = style.shadows(shadows);
        }
        if let Some(foreground) = resolved.foreground {
            style = style.foreground(foreground);
        }
        style
    }
}

/// What `Icon.build` resolves from the widget and the ambient [`IconTheme`] before it
/// composes the glyph's [`TextStyle`]. Dart's locals, grouped so they travel together.
struct ResolvedIcon {
    size: f64,
    fill: Option<f64>,
    weight: Option<f64>,
    grade: Option<f64>,
    optical_size: Option<f64>,
    shadows: Option<Vec<Shadow>>,
    color: Option<AnyColor>,
    foreground: Option<Paint>,
}

impl StatelessWidget for Icon {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let text_direction = self
            .text_direction
            .unwrap_or_else(|| Directionality::of(app, context));

        let icon_theme = IconTheme::of(app, context);

        let apply_text_scaling = self
            .apply_text_scaling
            .or(icon_theme.apply_text_scaling)
            .unwrap_or(false);

        let tentative_icon_size = self.size.or(icon_theme.size).unwrap_or(K_DEFAULT_FONT_SIZE);

        let icon_size = if apply_text_scaling {
            MediaQuery::text_scaler_of(app, context).scale(tentative_icon_size)
        } else {
            tentative_icon_size
        };

        let icon_fill = self.fill.or(icon_theme.fill);
        let icon_weight = self.weight.or(icon_theme.weight);
        let icon_grade = self.grade.or(icon_theme.grade);
        let icon_optical_size = self.optical_size.or(icon_theme.optical_size);
        let icon_shadows = self.shadows.clone().or(icon_theme.shadows.clone());

        let Some(icon) = &self.icon else {
            return SizedBox::new()
                .width(icon_size)
                .height(icon_size)
                .into_widget();
        };

        let icon_opacity = icon_theme.opacity.unwrap_or(1.0);
        let mut icon_color = Some(self.color.clone().unwrap_or_else(|| {
            icon_theme
                .color
                .clone()
                .expect("IconTheme::of returns a concrete theme")
        }));
        let mut foreground = None;
        if icon_opacity != 1.0 {
            let color = icon_color.as_ref().expect("the color is resolved above");
            // Dart's `withOpacity`, deprecated there too: its rounding to an 8-bit alpha is
            // part of the behaviour being ported.
            #[allow(deprecated)]
            let faded = color.with_opacity(color.opacity() * icon_opacity);
            icon_color = Some(faded.into());
        }
        if let Some(blend_mode) = self.blend_mode {
            foreground = Some(Paint {
                blend_mode,
                color: icon_color
                    .as_ref()
                    .expect("the color is resolved above")
                    .color()
                    .into(),
                ..Paint::default()
            });
            // Cannot provide both a color and a foreground.
            icon_color = None;
        }

        let font_style = self.font_style(
            icon,
            ResolvedIcon {
                size: icon_size,
                fill: icon_fill,
                weight: icon_weight,
                grade: icon_grade,
                optical_size: icon_optical_size,
                shadows: icon_shadows,
                color: icon_color,
                foreground,
            },
        );

        let code_point = char::from_u32(icon.code_point)
            .expect("an IconData code point is a Unicode scalar value");
        let mut icon_widget = RichText::new(
            TextSpan::new()
                .text(code_point.to_string())
                .style(font_style)
                .into_span(),
        )
        // Never clip.
        .overflow(TextOverflow::Visible)
        // Since we already fetched it for the assert...
        .text_direction(text_direction)
        .into_widget();

        if icon.match_text_direction {
            match text_direction {
                TextDirection::Rtl => {
                    icon_widget = Transform::new(Matrix4::scale(-1.0, 1.0))
                        .alignment(AlignmentGeometry::CENTER)
                        .transform_hit_tests(false)
                        .child(icon_widget)
                        .into_widget();
                }
                TextDirection::Ltr => {}
            }
        }

        SizedBox::new()
            .width(icon_size)
            .height(icon_size)
            .child(Center::new().child(icon_widget))
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use reveal_embedder::Color;
    use reveal_embedder::Size;
    use reveal_foundation::AppCell;
    use reveal_painting::{PaintingBinding, TextScaler};
    use reveal_rendering::{
        AnyRenderBox, BoxConstraints, RenderBox, RenderConstrainedBox, RenderHandle, RenderObject,
        RenderObjectWithChildMixin, RenderParagraph, RenderPositionedBox, RenderTransform,
    };

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::icon_theme_data::IconThemeData;
    use crate::widgets::media_query::{MediaQuery, MediaQueryData};

    /// What the shell does at start-up: the app-wide fonts, here the OS fonts.
    fn install_fonts(app: &mut App) {
        let binding = PaintingBinding::instance(app);
        if !binding.has_fonts(app) {
            binding.install_fonts(app, |fonts| {
                fonts.add_source(valo_system_fonts::SystemFonts::load());
            });
        }
    }

    fn glyph() -> IconData {
        IconData::new(0xe900)
            .font_family("TestIcons")
            .font_package("test_icons")
    }

    fn typed_child<P, C: RenderObject>(parent: RenderHandle<P>, app: &App) -> RenderHandle<C>
    where
        P: RenderObject + RenderObjectWithChildMixin<ChildType = AnyRenderBox>,
    {
        parent
            .child(app)
            .expect("a child")
            .as_object()
            .downcast::<C>(app)
            .expect("the expected render object")
    }

    fn square_under_root(harness: &Harness, app: &App) -> RenderHandle<RenderConstrainedBox> {
        harness
            .render_root(app)
            .child(app)
            .expect("a child")
            .as_object()
            .downcast::<RenderConstrainedBox>(app)
            .expect("Icon builds a SizedBox")
    }

    /// Mounts `icon` under an ambient `Directionality` and `IconTheme`, and pumps a frame.
    fn mount(
        app: &mut App,
        text_direction: TextDirection,
        theme: IconThemeData,
        icon: Icon,
    ) -> Harness {
        install_fonts(app);
        let harness = Harness::mount(
            app,
            Directionality::new(text_direction, IconTheme::new(theme, icon)).into_widget(),
        );
        harness.pump(app);
        harness
    }

    #[test]
    fn an_icon_builds_a_centred_glyph_in_a_square_of_its_size() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let theme = IconThemeData::fallback().color(Color::from_argb(255, 255, 0, 0));
        let harness = mount(
            &mut app,
            TextDirection::Ltr,
            theme,
            Icon::new(Some(glyph())).size(30.0),
        );

        let square = square_under_root(&harness, &app);
        assert_eq!(
            square.additional_constraints(&app),
            BoxConstraints::tight_for(Some(30.0), Some(30.0))
        );
        assert_eq!(square.size(&app), Size::new(30.0, 30.0));

        let center = typed_child::<_, RenderPositionedBox>(square, &app);
        let paragraph = typed_child::<_, RenderParagraph>(center, &app);
        assert_eq!(
            paragraph.text(&app).to_plain_text(true, true),
            "\u{e900}",
            "the code point is the glyph's text"
        );
        let style = paragraph
            .text(&app)
            .style()
            .cloned()
            .expect("Icon always styles its span");
        assert!(!style.inherit);
        assert_eq!(style.font_size, Some(30.0));
        assert_eq!(style.height, Some(1.0));
        assert_eq!(
            style.font_family.as_deref(),
            Some("packages/test_icons/TestIcons"),
            "the icon's font package prefixes its family"
        );
        assert_eq!(
            style.color.as_ref().map(AnyColor::color),
            Some(Color::from_argb(255, 255, 0, 0)),
            "the ambient IconTheme's color"
        );
        assert_eq!(style.font_variations.as_deref().map(<[_]>::len), Some(4));
        assert!(style.foreground.is_none());
    }

    #[test]
    fn an_icon_defaults_its_size_and_axes_to_the_ambient_theme() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let theme = IconThemeData::fallback().size(18.0).weight(700.0);
        let harness = mount(
            &mut app,
            TextDirection::Ltr,
            theme,
            Icon::new(Some(glyph())),
        );
        let square = square_under_root(&harness, &app);
        assert_eq!(square.size(&app), Size::new(18.0, 18.0));

        let center = typed_child::<_, RenderPositionedBox>(square, &app);
        let paragraph = typed_child::<_, RenderParagraph>(center, &app);
        let style = paragraph
            .text(&app)
            .style()
            .cloned()
            .expect("Icon always styles its span");
        assert_eq!(style.font_size, Some(18.0));
        let weight = style
            .font_variations
            .as_ref()
            .expect("the axes are always written")
            .iter()
            .find(|variation| variation.axis == "wght")
            .expect("the theme's weight");
        assert_eq!(weight.value, 700.0);
    }

    #[test]
    fn an_icon_without_an_icon_data_is_an_empty_square() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = mount(
            &mut app,
            TextDirection::Ltr,
            IconThemeData::fallback(),
            Icon::new(None).size(42.0),
        );
        let square = square_under_root(&harness, &app);
        assert_eq!(square.size(&app), Size::new(42.0, 42.0));
        assert!(square.child(&app).is_none(), "no glyph is built");
    }

    #[test]
    fn a_direction_matching_icon_is_mirrored_in_rtl() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let mirrored = glyph().match_text_direction(true);
        let cell_ltr = AppCell::new();
        let mut app_ltr = cell_ltr.borrow_mut();
        let ltr = mount(
            &mut app_ltr,
            TextDirection::Ltr,
            IconThemeData::fallback(),
            Icon::new(Some(mirrored.clone())),
        );
        let square = square_under_root(&ltr, &app_ltr);
        let center = typed_child::<_, RenderPositionedBox>(square, &app_ltr);
        assert!(
            center
                .child(&app_ltr)
                .expect("a child")
                .as_object()
                .downcast::<RenderParagraph>(&app_ltr)
                .is_some(),
            "no transform in left-to-right"
        );

        let rtl = mount(
            &mut app,
            TextDirection::Rtl,
            IconThemeData::fallback(),
            Icon::new(Some(mirrored)),
        );
        let square = square_under_root(&rtl, &app);
        let center = typed_child::<_, RenderPositionedBox>(square, &app);
        let transform = typed_child::<_, RenderTransform>(center, &app);
        assert!(!transform.transform_hit_tests(&app));
        assert_eq!(transform.alignment(&app), Some(AlignmentGeometry::CENTER));
        typed_child::<_, RenderParagraph>(transform, &app);
    }

    #[test]
    fn apply_text_scaling_scales_the_square_with_the_media_query() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        install_fonts(&mut app);
        let media = MediaQueryData::new().text_scaler(TextScaler::linear(2.0));
        let harness = Harness::mount(
            &mut app,
            Directionality::new(
                TextDirection::Ltr,
                MediaQuery::new(
                    media,
                    IconTheme::new(
                        IconThemeData::fallback().size(10.0),
                        Icon::new(Some(glyph())).apply_text_scaling(true),
                    ),
                )
                .into_widget(),
            )
            .into_widget(),
        );
        harness.pump(&mut app);
        let square = square_under_root(&harness, &app);
        assert_eq!(
            square.size(&app),
            Size::new(20.0, 20.0),
            "the text scaler doubles the icon's size"
        );
    }

    #[test]
    fn a_blend_mode_moves_the_color_into_a_foreground_paint() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let theme = IconThemeData::fallback().color(Color::from_argb(255, 0, 0, 255));
        let harness = mount(
            &mut app,
            TextDirection::Ltr,
            theme,
            Icon::new(Some(glyph())).blend_mode(BlendMode::Multiply),
        );
        let square = square_under_root(&harness, &app);
        let center = typed_child::<_, RenderPositionedBox>(square, &app);
        let paragraph = typed_child::<_, RenderParagraph>(center, &app);
        let style = paragraph
            .text(&app)
            .style()
            .cloned()
            .expect("Icon always styles its span");
        assert!(
            style.color.is_none(),
            "a color and a foreground are exclusive"
        );
        let foreground = style.foreground.expect("the blend mode makes a paint");
        assert_eq!(foreground.blend_mode, BlendMode::Multiply);
        assert_eq!(foreground.color, Color::from_argb(255, 0, 0, 255).into());
    }

    #[test]
    fn the_theme_s_opacity_fades_the_glyph_color() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let theme = IconThemeData::fallback()
            .color(Color::from_argb(255, 0, 0, 255))
            .opacity(0.5);
        let harness = mount(
            &mut app,
            TextDirection::Ltr,
            theme,
            Icon::new(Some(glyph())),
        );
        let square = square_under_root(&harness, &app);
        let center = typed_child::<_, RenderPositionedBox>(square, &app);
        let paragraph = typed_child::<_, RenderParagraph>(center, &app);
        let style = paragraph
            .text(&app)
            .style()
            .cloned()
            .expect("Icon always styles its span");
        assert_eq!(
            style.color.as_ref().map(AnyColor::color),
            Some(Color::from_argb(128, 0, 0, 255))
        );
    }
}
