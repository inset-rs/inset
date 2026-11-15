//! Flutter counterpart: material_ui's `expansion_tile_theme.dart`.

use waft_animation::AnimationStyle;
use waft_embedder::{Clip, Color};
use waft_foundation::App;
use waft_painting::{AlignmentGeometry, EdgeInsetsGeometry, ShapeBorder};
use waft_widgets::{BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef};

use crate::theme::Theme;

/// Used with [`ExpansionTileTheme`] to define default property values for
/// descendant `ExpansionTile` widgets.
///
/// Descendant widgets obtain the current [`ExpansionTileThemeData`] object
/// using [`ExpansionTileTheme::of`]. Instances of [`ExpansionTileThemeData`] can
/// be customized with [`ExpansionTileThemeData::copy_with`].
///
/// A [`ExpansionTileThemeData`] is often specified as part of the
/// overall `Theme` with `ThemeData::expansion_tile_theme`.
///
/// All [`ExpansionTileThemeData`] properties are `None` by default.
/// When a theme property is `None`, the `ExpansionTile`  will provide its own
/// default based on the overall `Theme`'s text theme and
/// color scheme. See the individual `ExpansionTile` properties for details.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the
///    application.
///  * [`ExpansionTileTheme`] which overrides the default [`ExpansionTileTheme`]
///    of its `ExpansionTile` descendants.
///  * `ThemeData::text_theme`, text with a color that contrasts with the card
///    and canvas colors.
///  * `ThemeData::color_scheme`, the thirteen colors that most Material widget
///    default colors are based on.
#[derive(Clone, Debug, Default)]
pub struct ExpansionTileThemeData {
    /// Overrides the default value of `ExpansionTile::background_color`.
    pub background_color: Option<Color>,

    /// Overrides the default value of `ExpansionTile::collapsed_background_color`.
    pub collapsed_background_color: Option<Color>,

    /// Overrides the default value of `ExpansionTile::tile_padding`.
    pub tile_padding: Option<EdgeInsetsGeometry>,

    /// Overrides the default value of `ExpansionTile::expanded_alignment`.
    pub expanded_alignment: Option<AlignmentGeometry>,

    /// Overrides the default value of `ExpansionTile::children_padding`.
    pub children_padding: Option<EdgeInsetsGeometry>,

    /// Overrides the default value of `ExpansionTile::icon_color`.
    pub icon_color: Option<Color>,

    /// Overrides the default value of `ExpansionTile::collapsed_icon_color`.
    pub collapsed_icon_color: Option<Color>,

    /// Overrides the default value of `ExpansionTile::text_color`.
    pub text_color: Option<Color>,

    /// Overrides the default value of `ExpansionTile::collapsed_text_color`.
    pub collapsed_text_color: Option<Color>,

    /// Overrides the default value of `ExpansionTile::shape`.
    pub shape: Option<Box<dyn ShapeBorder>>,

    /// Overrides the default value of `ExpansionTile::collapsed_shape`.
    pub collapsed_shape: Option<Box<dyn ShapeBorder>>,

    /// Overrides the default value of `ExpansionTile::clip_behavior`.
    pub clip_behavior: Option<Clip>,

    /// Overrides the default value of `ExpansionTile::expansion_animation_style`.
    pub expansion_animation_style: Option<AnimationStyle>,
}

impl ExpansionTileThemeData {
    /// Creates a [`ExpansionTileThemeData`]; Dart's optional named arguments are the fluent
    /// setters.
    pub fn new() -> ExpansionTileThemeData {
        ExpansionTileThemeData::default()
    }

    /// Dart `ExpansionTileThemeData(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> ExpansionTileThemeData {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `ExpansionTileThemeData(collapsedBackgroundColor:)`.
    pub fn collapsed_background_color(
        mut self,
        collapsed_background_color: Color,
    ) -> ExpansionTileThemeData {
        self.collapsed_background_color = Some(collapsed_background_color);
        self
    }

    /// Dart `ExpansionTileThemeData(tilePadding:)`.
    pub fn tile_padding(
        mut self,
        tile_padding: impl Into<EdgeInsetsGeometry>,
    ) -> ExpansionTileThemeData {
        self.tile_padding = Some(tile_padding.into());
        self
    }

    /// Dart `ExpansionTileThemeData(expandedAlignment:)`.
    pub fn expanded_alignment(
        mut self,
        expanded_alignment: impl Into<AlignmentGeometry>,
    ) -> ExpansionTileThemeData {
        self.expanded_alignment = Some(expanded_alignment.into());
        self
    }

    /// Dart `ExpansionTileThemeData(childrenPadding:)`.
    pub fn children_padding(
        mut self,
        children_padding: impl Into<EdgeInsetsGeometry>,
    ) -> ExpansionTileThemeData {
        self.children_padding = Some(children_padding.into());
        self
    }

    /// Dart `ExpansionTileThemeData(iconColor:)`.
    pub fn icon_color(mut self, icon_color: Color) -> ExpansionTileThemeData {
        self.icon_color = Some(icon_color);
        self
    }

    /// Dart `ExpansionTileThemeData(collapsedIconColor:)`.
    pub fn collapsed_icon_color(mut self, collapsed_icon_color: Color) -> ExpansionTileThemeData {
        self.collapsed_icon_color = Some(collapsed_icon_color);
        self
    }

    /// Dart `ExpansionTileThemeData(textColor:)`.
    pub fn text_color(mut self, text_color: Color) -> ExpansionTileThemeData {
        self.text_color = Some(text_color);
        self
    }

    /// Dart `ExpansionTileThemeData(collapsedTextColor:)`.
    pub fn collapsed_text_color(mut self, collapsed_text_color: Color) -> ExpansionTileThemeData {
        self.collapsed_text_color = Some(collapsed_text_color);
        self
    }

    /// Dart `ExpansionTileThemeData(shape:)`.
    pub fn shape(mut self, shape: impl Into<Box<dyn ShapeBorder>>) -> ExpansionTileThemeData {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `ExpansionTileThemeData(collapsedShape:)`.
    pub fn collapsed_shape(
        mut self,
        collapsed_shape: impl Into<Box<dyn ShapeBorder>>,
    ) -> ExpansionTileThemeData {
        self.collapsed_shape = Some(collapsed_shape.into());
        self
    }

    /// Dart `ExpansionTileThemeData(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> ExpansionTileThemeData {
        self.clip_behavior = Some(clip_behavior);
        self
    }

    /// Dart `ExpansionTileThemeData(expansionAnimationStyle:)`.
    pub fn expansion_animation_style(
        mut self,
        expansion_animation_style: AnimationStyle,
    ) -> ExpansionTileThemeData {
        self.expansion_animation_style = Some(expansion_animation_style);
        self
    }

    /// Creates a copy of this object with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().icon_color(c)`).
    pub fn copy_with(&self) -> ExpansionTileThemeData {
        self.clone()
    }

    /// Linearly interpolate between ExpansionTileThemeData objects.
    pub fn lerp(
        a: Option<&ExpansionTileThemeData>,
        b: Option<&ExpansionTileThemeData>,
        t: f64,
    ) -> Option<ExpansionTileThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(ExpansionTileThemeData {
            background_color: Color::lerp(
                a.and_then(|a| a.background_color),
                b.and_then(|b| b.background_color),
                t,
            ),
            collapsed_background_color: Color::lerp(
                a.and_then(|a| a.collapsed_background_color),
                b.and_then(|b| b.collapsed_background_color),
                t,
            ),
            tile_padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.tile_padding),
                b.and_then(|b| b.tile_padding),
                t,
            ),
            expanded_alignment: AlignmentGeometry::lerp(
                a.and_then(|a| a.expanded_alignment),
                b.and_then(|b| b.expanded_alignment),
                t,
            ),
            children_padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.children_padding),
                b.and_then(|b| b.children_padding),
                t,
            ),
            icon_color: Color::lerp(
                a.and_then(|a| a.icon_color),
                b.and_then(|b| b.icon_color),
                t,
            ),
            collapsed_icon_color: Color::lerp(
                a.and_then(|a| a.collapsed_icon_color),
                b.and_then(|b| b.collapsed_icon_color),
                t,
            ),
            text_color: Color::lerp(
                a.and_then(|a| a.text_color),
                b.and_then(|b| b.text_color),
                t,
            ),
            collapsed_text_color: Color::lerp(
                a.and_then(|a| a.collapsed_text_color),
                b.and_then(|b| b.collapsed_text_color),
                t,
            ),
            shape: <dyn ShapeBorder>::lerp(
                a.and_then(|a| a.shape.as_deref()),
                b.and_then(|b| b.shape.as_deref()),
                t,
            ),
            collapsed_shape: <dyn ShapeBorder>::lerp(
                a.and_then(|a| a.collapsed_shape.as_deref()),
                b.and_then(|b| b.collapsed_shape.as_deref()),
                t,
            ),
            clip_behavior: pick(a, b, t, |theme| theme.clip_behavior),
            expansion_animation_style: pick(a, b, t, |theme| {
                theme.expansion_animation_style.clone()
            }),
        })
    }
}

/// Dart's `t < 0.5 ? a?.field : b?.field`.
fn pick<T>(
    a: Option<&ExpansionTileThemeData>,
    b: Option<&ExpansionTileThemeData>,
    t: f64,
    field: impl Fn(&ExpansionTileThemeData) -> Option<T>,
) -> Option<T> {
    if t < 0.5 {
        a.and_then(&field)
    } else {
        b.and_then(&field)
    }
}

impl PartialEq for ExpansionTileThemeData {
    fn eq(&self, other: &ExpansionTileThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.background_color == self.background_color
            && other.collapsed_background_color == self.collapsed_background_color
            && other.tile_padding == self.tile_padding
            && other.expanded_alignment == self.expanded_alignment
            && other.children_padding == self.children_padding
            && other.icon_color == self.icon_color
            && other.collapsed_icon_color == self.collapsed_icon_color
            && other.text_color == self.text_color
            && other.collapsed_text_color == self.collapsed_text_color
            && other.shape == self.shape
            && other.collapsed_shape == self.collapsed_shape
            && other.clip_behavior == self.clip_behavior
            && other.expansion_animation_style == self.expansion_animation_style
    }
}

/// Overrides the default [`ExpansionTileTheme`] of its `ExpansionTile` descendants.
///
/// See also:
///
///  * [`ExpansionTileThemeData`], which is used to configure this theme.
///  * `ThemeData::expansion_tile_theme`, which can be used to override the default
///    [`ExpansionTileTheme`] for `ExpansionTile`s below the overall `Theme`.
#[derive(Debug)]
pub struct ExpansionTileTheme {
    pub key: Option<KeyRef>,
    /// Specifies color, alignment, and text style values for
    /// descendant `ExpansionTile` widgets.
    pub data: ExpansionTileThemeData,
    pub child: WidgetRef,
}

impl ExpansionTileTheme {
    /// Applies the given theme [`data`](Self::data) to [`child`](Self::child).
    pub fn new<K>(data: ExpansionTileThemeData, child: impl IntoWidget<K>) -> ExpansionTileTheme {
        ExpansionTileTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `ExpansionTileTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> ExpansionTileTheme {
        self.key = Some(key);
        self
    }

    /// Retrieves the [`ExpansionTileThemeData`] from the closest ancestor [`ExpansionTileTheme`].
    ///
    /// If there is no enclosing [`ExpansionTileTheme`] widget, then
    /// `ThemeData::expansion_tile_theme` is used.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = ExpansionTileTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> ExpansionTileThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<ExpansionTileTheme>(app) {
            Some(inherited_theme) => inherited_theme.data.clone(),
            None => Theme::of(app, context).expansion_tile_theme.clone(),
        }
    }
}

impl InheritedTheme for ExpansionTileTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        ExpansionTileTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for ExpansionTileTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &ExpansionTileTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::Color;
    use waft_painting::{Alignment, Border, BorderSide, EdgeInsets};
    use waft_widgets::{Center, GlobalKey, Text};

    use super::*;
    use crate::colors::Colors;
    use crate::expansion_tile::ExpansionTile;
    use crate::expansion_tile::tests::{
        animation_style_case, expand_more_rect, last_padding_rect, list_tile_rect, probe, pump_app,
        rect_of, sized, test_icon, test_text, tester, text_rect, tile_material,
    };
    use crate::material::Material;
    use crate::test_support::{AhemText, with_main_thread_stack};
    use crate::theme_data::ThemeData;

    /// "ExpansionTileThemeData copyWith, ==, hashCode basics".
    #[test]
    fn expansion_tile_theme_data_copy_with_equality_basics() {
        assert_eq!(
            ExpansionTileThemeData::new(),
            ExpansionTileThemeData::new().copy_with()
        );
    }

    /// "ExpansionTileThemeData lerp special cases".
    #[test]
    fn expansion_tile_theme_data_lerp_special_cases() {
        assert_eq!(ExpansionTileThemeData::lerp(None, None, 0.0), None);
        let data = ExpansionTileThemeData::new();
        // Dart's `identical`: the value comes back as it was.
        assert_eq!(
            ExpansionTileThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data.clone())
        );
    }

    /// "ExpansionTileThemeData defaults".
    #[test]
    fn expansion_tile_theme_data_defaults() {
        let theme = ExpansionTileThemeData::new();
        assert_eq!(theme.background_color, None);
        assert_eq!(theme.collapsed_background_color, None);
        assert_eq!(theme.tile_padding, None);
        assert_eq!(theme.expanded_alignment, None);
        assert_eq!(theme.children_padding, None);
        assert_eq!(theme.icon_color, None);
        assert_eq!(theme.collapsed_icon_color, None);
        assert_eq!(theme.text_color, None);
        assert_eq!(theme.collapsed_text_color, None);
        assert!(theme.shape.is_none());
        assert!(theme.collapsed_shape.is_none());
        assert_eq!(theme.clip_behavior, None);
        assert_eq!(theme.expansion_animation_style, None);
    }

    /// `Border(top: BorderSide(color: color), bottom: BorderSide(color: color))`.
    fn vertical_border(color: Color) -> Border {
        let side = BorderSide {
            color,
            ..BorderSide::default()
        };
        Border {
            top: side,
            bottom: side,
            ..Border::default()
        }
    }

    fn icon_color() -> Color {
        Colors::GREEN.primary()
    }

    fn collapsed_icon_color() -> Color {
        Colors::BLUE.primary()
    }

    const TEXT_COLOR: Color = Colors::BLACK;
    const COLLAPSED_TEXT_COLOR: Color = Colors::WHITE;

    /// The theme both "ExpansionTileTheme" cases put in `ThemeData.expansionTileTheme`.
    fn full_theme(clip_behavior: Clip) -> ExpansionTileThemeData {
        ExpansionTileThemeData::new()
            .background_color(Colors::ORANGE.primary())
            .collapsed_background_color(Colors::RED.primary())
            .tile_padding(EdgeInsets::from_ltrb(8.0, 12.0, 4.0, 10.0))
            .expanded_alignment(Alignment::CENTER_RIGHT)
            .children_padding(EdgeInsets::all(20.0))
            .icon_color(icon_color())
            .collapsed_icon_color(collapsed_icon_color())
            .text_color(TEXT_COLOR)
            .collapsed_text_color(COLLAPSED_TEXT_COLOR)
            .shape(vertical_border(Colors::RED.primary()))
            .collapsed_shape(vertical_border(Colors::GREEN.primary()))
            .clip_behavior(clip_behavior)
    }

    /// The title and trailing positions both "ExpansionTileTheme" cases check after the
    /// theme's tile padding is applied.
    fn assert_tile_padding(tester: &crate::test_support::WidgetTester, title: &str) {
        let title_rect = text_rect(tester, title);
        let trailing_rect = expand_more_rect(tester);
        let list_tile_rect = list_tile_rect(tester);
        let taller_widget = if title_rect.height() > trailing_rect.height() {
            title_rect
        } else {
            trailing_rect
        };

        // Check the positions of title and trailing Widgets, after padding is applied.
        assert_eq!(list_tile_rect.left, title_rect.left - 8.0);
        assert_eq!(list_tile_rect.right, trailing_rect.right + 4.0);

        // Calculate the remaining height of ListTile from the default height.
        let remaining_height = 56.0 - taller_widget.height();
        assert_eq!(
            list_tile_rect.top,
            taller_widget.top - remaining_height / 2.0 - 12.0
        );
        assert_eq!(
            list_tile_rect.bottom,
            taller_widget.bottom + remaining_height / 2.0 + 10.0
        );
    }

    fn boxed(shape: Border) -> Option<Box<dyn ShapeBorder>> {
        Some(Box::new(shape))
    }

    /// "ExpansionTileTheme - collapsed".
    #[test]
    fn expansion_tile_theme_collapsed() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let icon = probe();
            let text = probe();
            let clip_behavior = Clip::AntiAlias;

            pump_app(
                &mut tester,
                |app| {
                    ThemeData::builder()
                        .expansion_tile_theme(full_theme(clip_behavior))
                        .build(app)
                },
                Material::new().child(
                    Center::new().child(
                        ExpansionTile::new(test_text("Collapsed Tile", &text))
                            .trailing(test_icon(&icon))
                            .children([Text::new("Tile 1").into_widget()]),
                    ),
                ),
            );

            // When a custom shape is provided, ExpansionTile will use the
            // Material widget to draw the shape and background color
            // instead of a Container.
            let (shape, color, material_clip_behavior) = tile_material(&tester);

            // ExpansionTile should have Clip.antiAlias as clipBehavior.
            assert_eq!(material_clip_behavior, clip_behavior);

            // Check the tile's collapsed background color when collapsedBackgroundColor is
            // applied.
            assert_eq!(color, Some(Colors::RED.primary()));

            assert_tile_padding(&tester, "Collapsed Tile");

            // Check the collapsed icon color when iconColor is applied.
            assert_eq!(icon.get(), Some(collapsed_icon_color()));
            // Check the collapsed text color when textColor is applied.
            assert_eq!(text.get(), Some(COLLAPSED_TEXT_COLOR));
            // Check the collapsed ShapeBorder when shape is applied.
            assert_eq!(shape, boxed(vertical_border(Colors::GREEN.primary())));
        });
    }

    /// "ExpansionTileTheme - expanded".
    #[test]
    fn expansion_tile_theme_expanded() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let icon = probe();
            let text = probe();
            let clip_behavior = Clip::None;

            pump_app(
                &mut tester,
                |app| {
                    ThemeData::builder()
                        .expansion_tile_theme(full_theme(clip_behavior))
                        .build(app)
                },
                Material::new().child(
                    Center::new().child(
                        ExpansionTile::new(test_text("Expanded Tile", &text))
                            .initially_expanded(true)
                            .trailing(test_icon(&icon))
                            .children([AhemText::new("Tile 1").into_widget()]),
                    ),
                ),
            );

            // When a custom shape is provided, ExpansionTile will use the
            // Material widget to draw the shape and background color
            // instead of a Container.
            let (shape, color, material_clip_behavior) = tile_material(&tester);
            // Check the tile's background color when backgroundColor is applied.
            assert_eq!(color, Some(Colors::ORANGE.primary()));

            assert_tile_padding(&tester, "Expanded Tile");

            // Check the expanded icon color when iconColor is applied.
            assert_eq!(icon.get(), Some(icon_color()));
            // Check the expanded text color when textColor is applied.
            assert_eq!(text.get(), Some(TEXT_COLOR));
            // Check the expanded ShapeBorder when shape is applied.
            assert_eq!(shape, boxed(vertical_border(Colors::RED.primary())));
            // Check the clipBehavior when shape is applied.
            assert_eq!(material_clip_behavior, clip_behavior);

            // Check the child position when expandedAlignment is applied.
            let child = tester.find_ahem_text("Tile 1");
            assert_eq!(child.len(), 1, "one \"Tile 1\"");
            let child_rect = rect_of(&tester, child[0]);
            assert_eq!(child_rect.right, 800.0 - 20.0);
            assert_eq!(child_rect.left, 800.0 - child_rect.width() - 20.0);

            // Check the child padding when childrenPadding is applied.
            let padding_rect = last_padding_rect(&tester);
            assert_eq!(child_rect.top, padding_rect.top + 20.0);
            assert_eq!(child_rect.left, padding_rect.left + 20.0);
            assert_eq!(child_rect.right, padding_rect.right - 20.0);
            assert_eq!(child_rect.bottom, padding_rect.bottom - 20.0);
        });
    }

    /// "Override ExpansionTile animation using ExpansionTileThemeData.AnimationStyle".
    #[test]
    fn override_expansion_tile_animation_using_expansion_tile_theme_data_animation_style() {
        with_main_thread_stack(|| {
            animation_style_case(
                |tester, key: &GlobalKey, animation_style| {
                    let mut expansion_tile_theme = ExpansionTileThemeData::new();
                    expansion_tile_theme.expansion_animation_style = animation_style;
                    pump_app(
                        tester,
                        |app| {
                            ThemeData::builder()
                                .expansion_tile_theme(expansion_tile_theme)
                                .build(app)
                        },
                        Material::new().child(
                            Center::new().child(
                                ExpansionTile::new(test_text("title", &probe()))
                                    .key(key.clone().into())
                                    .children([sized(100.0, 100.0)]),
                            ),
                        ),
                    );
                },
                false,
            );
        });
    }
}
