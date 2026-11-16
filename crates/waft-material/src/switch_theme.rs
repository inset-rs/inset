//! Flutter counterpart: material_ui's `switch_theme.dart`.

use std::rc::Rc;

use waft_embedder::{Color, lerp_double};
use waft_foundation::App;
use waft_painting::EdgeInsetsGeometry;
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, Icon, InheritedWidget, IntoWidget, KeyRef, WidgetRef, WidgetStateProperty,
    WidgetStatePropertyRef,
};

use crate::theme::Theme;
use crate::theme_data::MaterialTapTargetSize;

/// Defines default property values for descendant `Switch` widgets.
///
/// Descendant widgets obtain the current [`SwitchThemeData`] object using
/// [`SwitchTheme::of`]. Instances of [`SwitchThemeData`] can be customized
/// with [`SwitchThemeData::copy_with`].
///
/// Typically a [`SwitchThemeData`] is specified as part of the overall `Theme`
/// with `ThemeData::switch_theme`.
///
/// All [`SwitchThemeData`] properties are `None` by default. When `None`, the
/// `Switch` will use the values from `ThemeData` if they exist, otherwise it
/// will provide its own defaults based on the overall `Theme`'s color_scheme.
/// See the individual `Switch` properties for details.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug, Default)]
pub struct SwitchThemeData {
    /// The color of a `Switch`'s thumb, resolved in the selected, hovered, focused and
    /// disabled states.
    ///
    /// If specified, overrides the default value of `Switch::thumb_color`.
    pub thumb_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The color of a `Switch`'s track, resolved in the selected, hovered, focused and
    /// disabled states.
    ///
    /// If specified, overrides the default value of `Switch::track_color`.
    pub track_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The outline color of a `Switch`'s track, resolved in the selected, hovered, focused
    /// and disabled states.
    ///
    /// If specified, overrides the default value of `Switch::track_outline_color`.
    pub track_outline_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The outline width of a `Switch`'s track, resolved in the selected, hovered, focused
    /// and disabled states.
    ///
    /// If specified, overrides the default value of `Switch::track_outline_width`.
    pub track_outline_width: Option<WidgetStatePropertyRef<Option<f64>>>,

    /// Configures the minimum size of the tap target.
    ///
    /// If specified, overrides the default value of
    /// `Switch::material_tap_target_size`.
    pub material_tap_target_size: Option<MaterialTapTargetSize>,

    /// The cursor for a mouse pointer when it enters or is hovering over the widget,
    /// resolved in the selected, hovered, focused and disabled states.
    ///
    /// If specified, overrides the default value of `Switch::mouse_cursor`.
    pub mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,

    /// The color for the switch's `Material`, resolved in the pressed, selected, hovered
    /// and focused states.
    ///
    /// If specified, overrides the default value of `Switch::overlay_color`.
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The splash radius of the circular `Material` ink response.
    ///
    /// If specified, overrides the default value of `Switch::splash_radius`.
    pub splash_radius: Option<f64>,

    /// The icon to use on the thumb of a switch, resolved in the selected, hovered, focused
    /// and disabled states.
    ///
    /// It is overridden by `Switch::thumb_icon`.
    pub thumb_icon: Option<WidgetStatePropertyRef<Option<Rc<Icon>>>>,

    /// If specified, overrides the default value of `Switch::padding`.
    pub padding: Option<EdgeInsetsGeometry>,
}

crate::theme_data_getters::theme_data_getters! {
    /// Dart's getters of `SwitchThemeData`: the theme data answers with its fields, and a defaults
    /// subclass (Dart's `_...Defaults...(context)`) computes the ones it overrides when they
    /// are read.
    pub trait SwitchThemeDataGetters for SwitchThemeData, fields: switch_theme_data {
        thumb_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        track_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        track_outline_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        track_outline_width: Option<WidgetStatePropertyRef<Option<f64>>>,
        material_tap_target_size: Option<MaterialTapTargetSize>,
        mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,
        overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
        splash_radius: Option<f64>,
        thumb_icon: Option<WidgetStatePropertyRef<Option<Rc<Icon>>>>,
        padding: Option<EdgeInsetsGeometry>,
    }
}

impl SwitchThemeData {
    /// Creates a theme that can be used for `ThemeData::switch_theme`; Dart's optional named
    /// arguments are the fluent setters.
    pub fn new() -> SwitchThemeData {
        SwitchThemeData::default()
    }

    /// Dart `SwitchThemeData(thumbColor:)`.
    pub fn thumb_color(
        mut self,
        thumb_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> SwitchThemeData {
        self.thumb_color = Some(thumb_color);
        self
    }

    /// Dart `SwitchThemeData(trackColor:)`.
    pub fn track_color(
        mut self,
        track_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> SwitchThemeData {
        self.track_color = Some(track_color);
        self
    }

    /// Dart `SwitchThemeData(trackOutlineColor:)`.
    pub fn track_outline_color(
        mut self,
        track_outline_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> SwitchThemeData {
        self.track_outline_color = Some(track_outline_color);
        self
    }

    /// Dart `SwitchThemeData(trackOutlineWidth:)`.
    pub fn track_outline_width(
        mut self,
        track_outline_width: WidgetStatePropertyRef<Option<f64>>,
    ) -> SwitchThemeData {
        self.track_outline_width = Some(track_outline_width);
        self
    }

    /// Dart `SwitchThemeData(materialTapTargetSize:)`.
    pub fn material_tap_target_size(
        mut self,
        material_tap_target_size: MaterialTapTargetSize,
    ) -> SwitchThemeData {
        self.material_tap_target_size = Some(material_tap_target_size);
        self
    }

    /// Dart `SwitchThemeData(mouseCursor:)`.
    pub fn mouse_cursor(
        mut self,
        mouse_cursor: WidgetStatePropertyRef<Option<MouseCursorRef>>,
    ) -> SwitchThemeData {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `SwitchThemeData(overlayColor:)`.
    pub fn overlay_color(
        mut self,
        overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> SwitchThemeData {
        self.overlay_color = Some(overlay_color);
        self
    }

    /// Dart `SwitchThemeData(splashRadius:)`.
    pub fn splash_radius(mut self, splash_radius: f64) -> SwitchThemeData {
        self.splash_radius = Some(splash_radius);
        self
    }

    /// Dart `SwitchThemeData(thumbIcon:)`.
    pub fn thumb_icon(
        mut self,
        thumb_icon: WidgetStatePropertyRef<Option<Rc<Icon>>>,
    ) -> SwitchThemeData {
        self.thumb_icon = Some(thumb_icon);
        self
    }

    /// Dart `SwitchThemeData(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> SwitchThemeData {
        self.padding = Some(padding.into());
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().splash_radius(1.0)`).
    pub fn copy_with(&self) -> SwitchThemeData {
        self.clone()
    }

    /// Linearly interpolate between two [`SwitchThemeData`]s.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(
        a: Option<&SwitchThemeData>,
        b: Option<&SwitchThemeData>,
        t: f64,
    ) -> SwitchThemeData {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        SwitchThemeData {
            thumb_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.thumb_color.clone()),
                b.and_then(|b| b.thumb_color.clone()),
                t,
                Color::lerp,
            ),
            track_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.track_color.clone()),
                b.and_then(|b| b.track_color.clone()),
                t,
                Color::lerp,
            ),
            track_outline_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.track_outline_color.clone()),
                b.and_then(|b| b.track_outline_color.clone()),
                t,
                Color::lerp,
            ),
            track_outline_width: <dyn WidgetStateProperty<Option<f64>>>::lerp_optional(
                a.and_then(|a| a.track_outline_width.clone()),
                b.and_then(|b| b.track_outline_width.clone()),
                t,
                lerp_double,
            ),
            material_tap_target_size: if t < 0.5 {
                a.and_then(|a| a.material_tap_target_size)
            } else {
                b.and_then(|b| b.material_tap_target_size)
            },
            mouse_cursor: if t < 0.5 {
                a.and_then(|a| a.mouse_cursor.clone())
            } else {
                b.and_then(|b| b.mouse_cursor.clone())
            },
            overlay_color: <dyn WidgetStateProperty<Option<Color>>>::lerp_optional(
                a.and_then(|a| a.overlay_color.clone()),
                b.and_then(|b| b.overlay_color.clone()),
                t,
                Color::lerp,
            ),
            splash_radius: lerp_double(
                a.and_then(|a| a.splash_radius),
                b.and_then(|b| b.splash_radius),
                t,
            ),
            thumb_icon: if t < 0.5 {
                a.and_then(|a| a.thumb_icon.clone())
            } else {
                b.and_then(|b| b.thumb_icon.clone())
            },
            padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.padding),
                b.and_then(|b| b.padding),
                t,
            ),
        }
    }
}

impl PartialEq for SwitchThemeData {
    fn eq(&self, other: &SwitchThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.thumb_color == self.thumb_color
            && other.track_color == self.track_color
            && other.track_outline_color == self.track_outline_color
            && other.track_outline_width == self.track_outline_width
            && other.material_tap_target_size == self.material_tap_target_size
            && other.mouse_cursor == self.mouse_cursor
            && other.overlay_color == self.overlay_color
            && other.splash_radius == self.splash_radius
            && other.thumb_icon == self.thumb_icon
            && other.padding == self.padding
    }
}

/// Applies a switch theme to descendant `Switch` widgets.
///
/// Descendant widgets obtain the current theme's [`SwitchTheme`] object using
/// [`SwitchTheme::of`]. When a widget uses [`SwitchTheme::of`], it is automatically
/// rebuilt if the theme later changes.
///
/// A switch theme can be specified as part of the overall Material theme using
/// `ThemeData::switch_theme`.
///
/// See also:
///
///  * [`SwitchThemeData`], which describes the actual configuration of a switch
///    theme.
#[derive(Debug)]
pub struct SwitchTheme {
    pub key: Option<KeyRef>,
    /// The properties used for all descendant `Switch` widgets.
    pub data: SwitchThemeData,
    pub child: WidgetRef,
}

impl SwitchTheme {
    /// Constructs a switch theme that configures all descendant `Switch` widgets.
    pub fn new<K>(data: SwitchThemeData, child: impl IntoWidget<K>) -> SwitchTheme {
        SwitchTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `SwitchTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> SwitchTheme {
        self.key = Some(key);
        self
    }

    /// Returns the configuration [`data`](Self::data) from the closest [`SwitchTheme`]
    /// ancestor. If there is no ancestor, it returns `ThemeData::switch_theme`.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = SwitchTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> SwitchThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<SwitchTheme>(app) {
            Some(switch_theme) => switch_theme.data.clone(),
            None => Theme::of(app, context).switch_theme.clone(),
        }
    }
}

impl InheritedWidget for SwitchTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &SwitchTheme) -> bool {
        self.data != old_widget.data
    }
}

#[cfg(test)]
mod tests {
    //! Ported from `test/switch_theme_test.dart`.

    use waft_embedder::Size;
    use waft_services::SystemMouseCursors;
    use waft_widgets::{Center, FocusHighlightStrategy, WidgetState, WidgetStates};

    use waft_widgets::testing::{WidgetTester, test_widgets};

    use super::*;
    use crate::icons::{Icons, install_material_icon_font};
    use crate::scaffold::Scaffold;
    use crate::switch::Switch;
    use crate::switch::tests::{
        Drawn, M2_SHADOWS, c, circles, colors, drawn, hover, ignore, press, switch_rect,
    };
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::pump_app;

    #[test]
    fn copy_with_and_equality_keep_an_empty_theme_data() {
        assert_eq!(SwitchThemeData::new(), SwitchThemeData::new().copy_with());
    }

    #[test]
    fn lerp_of_a_theme_data_with_itself_is_that_theme_data() {
        // A theme with a `thumb_color`: lerping it makes a new property, which compares by
        // identity, so only Dart's `identical` shortcut hands back an equal theme.
        let data = SwitchThemeData::new()
            .thumb_color(by_selected(Color::new(0xff000001), Color::new(0xff000002)));
        assert_eq!(SwitchThemeData::lerp(Some(&data), Some(&data), 0.5), data);
    }

    #[test]
    fn every_switch_theme_data_property_defaults_to_none() {
        let assert_defaults = |theme_data: &SwitchThemeData| {
            assert!(theme_data.thumb_color.is_none());
            assert!(theme_data.track_color.is_none());
            assert!(theme_data.track_outline_color.is_none());
            assert!(theme_data.track_outline_width.is_none());
            assert!(theme_data.mouse_cursor.is_none());
            assert!(theme_data.material_tap_target_size.is_none());
            assert!(theme_data.overlay_color.is_none());
            assert!(theme_data.splash_radius.is_none());
            assert!(theme_data.thumb_icon.is_none());
            assert!(theme_data.padding.is_none());
        };
        assert_defaults(&SwitchThemeData::new());
        let theme = SwitchTheme::new(SwitchThemeData::new(), waft_widgets::SizedBox::new());
        assert_defaults(&theme.data);
    }

    fn by_selected<T: Clone + 'static>(
        selected: T,
        default: T,
    ) -> WidgetStatePropertyRef<Option<T>> {
        <dyn WidgetStateProperty<Option<T>>>::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Selected) {
                return Some(selected.clone());
            }
            Some(default.clone())
        })
    }

    fn all<T: Clone + std::fmt::Debug + PartialEq + 'static>(
        value: T,
    ) -> WidgetStatePropertyRef<Option<T>> {
        <dyn WidgetStateProperty<Option<T>>>::all(Some(value))
    }

    /// The overlay both themeable cases give: the focus colour when focused, else the hover
    /// colour when hovered.
    fn focus_then_hover(focus: Color, hover: Color) -> WidgetStatePropertyRef<Option<Color>> {
        <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Focused) {
                return Some(focus);
            }
            if states.contains(&WidgetState::Hovered) {
                return Some(hover);
            }
            None
        })
    }

    /// `MaterialApp(theme: theme, home: Scaffold(body: switch))`.
    fn pump_scaffold(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        switch: impl IntoWidget<waft_widgets::StatelessKind> + 'static,
    ) {
        let home = Scaffold::new().body(switch).into_widget();
        pump_app(tester, theme, move |_app| home);
    }

    const DEFAULT_THUMB_COLOR: Color = Color::new(0xfffffff0);
    const SELECTED_THUMB_COLOR: Color = Color::new(0xfffffff1);
    const DEFAULT_TRACK_COLOR: Color = Color::new(0xfffffff2);
    const SELECTED_TRACK_COLOR: Color = Color::new(0xfffffff3);
    const DEFAULT_TRACK_OUTLINE_COLOR: Color = Color::new(0xfffffff4);
    const SELECTED_TRACK_OUTLINE_COLOR: Color = Color::new(0xfffffff5);
    const DEFAULT_TRACK_OUTLINE_WIDTH: f64 = 3.0;
    const SELECTED_TRACK_OUTLINE_WIDTH: f64 = 6.0;
    const FOCUS_OVERLAY_COLOR: Color = Color::new(0xfffffff4);
    const HOVER_OVERLAY_COLOR: Color = Color::new(0xfffffff5);
    const SPLASH_RADIUS: f64 = 1.0;

    /// Dart's 'Material2 - Switch is themeable' and 'Material3 - Switch is themeable'.
    #[test]
    fn a_switch_is_themeable() {
        for use_material3 in [false, true] {
            test_widgets(|tester| {
                install_material_icon_font(&mut tester.cell.borrow_mut());
                tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
                let icon1 = Rc::new(Icon::new(Some(Icons::check())));
                let icon2 = Rc::new(Icon::new(Some(Icons::close())));
                let switch_theme = SwitchThemeData::new()
                    .thumb_color(by_selected(SELECTED_THUMB_COLOR, DEFAULT_THUMB_COLOR))
                    .track_color(by_selected(SELECTED_TRACK_COLOR, DEFAULT_TRACK_COLOR))
                    .track_outline_color(by_selected(
                        SELECTED_TRACK_OUTLINE_COLOR,
                        DEFAULT_TRACK_OUTLINE_COLOR,
                    ))
                    .track_outline_width(by_selected(
                        SELECTED_TRACK_OUTLINE_WIDTH,
                        DEFAULT_TRACK_OUTLINE_WIDTH,
                    ))
                    .mouse_cursor(all(MouseCursorRef::from(SystemMouseCursors::TEXT)))
                    .material_tap_target_size(MaterialTapTargetSize::ShrinkWrap)
                    .overlay_color(focus_then_hover(FOCUS_OVERLAY_COLOR, HOVER_OVERLAY_COLOR))
                    .splash_radius(SPLASH_RADIUS)
                    .thumb_icon(by_selected(icon1, icon2));
                let theme = Rc::new(move |app: &App| {
                    ThemeData::builder()
                        .use_material3(use_material3)
                        .switch_theme(switch_theme.clone())
                        .build(app)
                });
                let pump = |tester: &mut WidgetTester, selected: bool, autofocus: bool| {
                    let theme = Rc::clone(&theme);
                    pump_scaffold(
                        tester,
                        move |app| theme(app),
                        Switch::new(selected, ignore())
                            .drag_start_behavior(waft_gestures::DragStartBehavior::Down)
                            .autofocus(autofocus),
                    );
                    tester.pump_and_settle();
                };
                let thumb_and_glyph = |painted: &[Drawn]| {
                    let glyphs = painted
                        .iter()
                        .filter(|drawn| matches!(drawn, Drawn::Glyphs(..)))
                        .count();
                    (painted.iter().rev().find_map(Drawn::color), glyphs)
                };

                // Switch.
                pump(tester, false, false);
                let painted = drawn(tester);
                assert_eq!(painted[0].color(), Some(c(DEFAULT_TRACK_COLOR)));
                assert!(matches!(painted[1], Drawn::Stroke(color, width, _)
                    if color == c(DEFAULT_TRACK_OUTLINE_COLOR)
                        && f64::from(width) == DEFAULT_TRACK_OUTLINE_WIDTH));
                if use_material3 {
                    assert_eq!(painted.len(), 4, "{painted:?}");
                    assert_eq!(thumb_and_glyph(&painted), (Some(c(DEFAULT_THUMB_COLOR)), 1));
                } else {
                    assert_eq!(
                        colors(&painted),
                        [
                            &[c(DEFAULT_TRACK_COLOR), c(DEFAULT_TRACK_OUTLINE_COLOR)][..],
                            &M2_SHADOWS[..],
                            &[c(DEFAULT_THUMB_COLOR)][..]
                        ]
                        .concat()
                    );
                }
                // Size from MaterialTapTargetSize.shrinkWrap.
                let width = if use_material3 { 60.0 } else { 59.0 };
                assert_eq!(switch_rect(tester).size(), Size::new(width, 40.0));

                // Selected switch.
                pump(tester, true, false);
                let painted = drawn(tester);
                assert_eq!(painted[0].color(), Some(c(SELECTED_TRACK_COLOR)));
                assert!(matches!(painted[1], Drawn::Stroke(color, width, _)
                    if color == c(SELECTED_TRACK_OUTLINE_COLOR)
                        && f64::from(width) == SELECTED_TRACK_OUTLINE_WIDTH));
                if use_material3 {
                    assert_eq!(
                        thumb_and_glyph(&painted),
                        (Some(c(SELECTED_THUMB_COLOR)), 1)
                    );
                } else {
                    assert_eq!(colors(&painted).last(), Some(&c(SELECTED_THUMB_COLOR)));
                }

                // Switch with hover.
                pump(tester, false, false);
                hover(tester);
                tester.pump_and_settle();
                assert_eq!(tester.active_cursor(1), SystemMouseCursors::TEXT.into());
                assert_eq!(
                    circles(&drawn(tester), SPLASH_RADIUS),
                    [c(HOVER_OVERLAY_COLOR)]
                );

                // Switch with focus.
                pump(tester, false, true);
                assert_eq!(
                    circles(&drawn(tester), SPLASH_RADIUS),
                    [c(FOCUS_OVERLAY_COLOR)]
                );
            });
        }
    }

    /// Dart's 'Material2 - Switch properties are taken over the theme values' and its
    /// Material 3 twin.
    #[test]
    fn the_switch_properties_win_over_the_theme_values() {
        for use_material3 in [false, true] {
            test_widgets(|tester| {
                const THEME_DEFAULT_THUMB_COLOR: Color = Color::new(0xfffffff0);
                const THEME_SELECTED_THUMB_COLOR: Color = Color::new(0xfffffff1);
                const THEME_DEFAULT_TRACK_COLOR: Color = Color::new(0xfffffff2);
                const THEME_SELECTED_TRACK_COLOR: Color = Color::new(0xfffffff3);
                const THEME_DEFAULT_OUTLINE_COLOR: Color = Color::new(0xfffffff6);
                const THEME_SELECTED_OUTLINE_COLOR: Color = Color::new(0xfffffff7);
                const THEME_FOCUS_OVERLAY_COLOR: Color = Color::new(0xfffffff4);
                const THEME_HOVER_OVERLAY_COLOR: Color = Color::new(0xfffffff5);

                const DEFAULT_THUMB: Color = Color::new(0xffffff0f);
                const SELECTED_THUMB: Color = Color::new(0xffffff1f);
                const DEFAULT_TRACK: Color = Color::new(0xffffff2f);
                const SELECTED_TRACK: Color = Color::new(0xffffff3f);
                const DEFAULT_OUTLINE: Color = Color::new(0xffffff6f);
                const SELECTED_OUTLINE: Color = Color::new(0xffffff7f);
                const DEFAULT_OUTLINE_WIDTH: f64 = 6.0;
                const SELECTED_OUTLINE_WIDTH: f64 = 8.0;
                const FOCUS_COLOR: Color = Color::new(0xffffff4f);
                const HOVER_COLOR: Color = Color::new(0xffffff5f);
                const SPLASH: f64 = 2.0;

                install_material_icon_font(&mut tester.cell.borrow_mut());
                tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
                let switch_theme = SwitchThemeData::new()
                    .thumb_color(by_selected(
                        THEME_SELECTED_THUMB_COLOR,
                        THEME_DEFAULT_THUMB_COLOR,
                    ))
                    .track_color(by_selected(
                        THEME_SELECTED_TRACK_COLOR,
                        THEME_DEFAULT_TRACK_COLOR,
                    ))
                    .track_outline_color(by_selected(
                        THEME_SELECTED_OUTLINE_COLOR,
                        THEME_DEFAULT_OUTLINE_COLOR,
                    ))
                    .track_outline_width(by_selected(7.0, 5.0))
                    .mouse_cursor(all(MouseCursorRef::from(SystemMouseCursors::CLICK)))
                    .material_tap_target_size(MaterialTapTargetSize::Padded)
                    .overlay_color(focus_then_hover(
                        THEME_FOCUS_OVERLAY_COLOR,
                        THEME_HOVER_OVERLAY_COLOR,
                    ))
                    .splash_radius(1.0)
                    .thumb_icon(<dyn WidgetStateProperty<Option<Rc<Icon>>>>::all(None));
                let theme = Rc::new(move |app: &App| {
                    ThemeData::builder()
                        .use_material3(use_material3)
                        .switch_theme(switch_theme.clone())
                        .build(app)
                });
                let add = Rc::new(Icon::new(Some(Icons::add())));
                let alarm = Rc::new(Icon::new(Some(Icons::access_alarm())));
                let pump = |tester: &mut WidgetTester, selected: bool, autofocus: bool| {
                    let theme = Rc::clone(&theme);
                    pump_scaffold(
                        tester,
                        move |app| theme(app),
                        Switch::new(selected, ignore())
                            .autofocus(autofocus)
                            .thumb_color(by_selected(SELECTED_THUMB, DEFAULT_THUMB))
                            .track_color(by_selected(SELECTED_TRACK, DEFAULT_TRACK))
                            .track_outline_color(by_selected(SELECTED_OUTLINE, DEFAULT_OUTLINE))
                            .track_outline_width(by_selected(
                                SELECTED_OUTLINE_WIDTH,
                                DEFAULT_OUTLINE_WIDTH,
                            ))
                            .mouse_cursor(SystemMouseCursors::TEXT)
                            .material_tap_target_size(MaterialTapTargetSize::ShrinkWrap)
                            .focus_color(FOCUS_COLOR)
                            .hover_color(HOVER_COLOR)
                            .splash_radius(SPLASH)
                            .thumb_icon(by_selected(Rc::clone(&add), Rc::clone(&alarm))),
                    );
                    tester.pump_and_settle();
                };
                // Material 2 paints the thumb's three shadows between the outline and the thumb.
                let thumb_index = if use_material3 { 2 } else { 5 };

                // Switch.
                pump(tester, false, false);
                let painted = drawn(tester);
                assert_eq!(painted[0].color(), Some(c(DEFAULT_TRACK)));
                assert!(matches!(painted[1], Drawn::Stroke(color, width, _)
                    if color == c(DEFAULT_OUTLINE) && f64::from(width) == DEFAULT_OUTLINE_WIDTH));
                assert_eq!(painted[thumb_index].color(), Some(c(DEFAULT_THUMB)));
                if use_material3 {
                    // Dart's `paragraph(offset: Offset(12, 12))`: the glyph's ink is in the
                    // 16x16 icon square there.
                    assert!(
                        matches!(painted[3], Drawn::Glyphs(x, y) if (12.0..28.0).contains(&x) && (12.0..28.0).contains(&y)),
                        "{:?}",
                        painted[3]
                    );
                }
                // Size from MaterialTapTargetSize.shrinkWrap.
                let width = if use_material3 { 60.0 } else { 59.0 };
                assert_eq!(switch_rect(tester).size(), Size::new(width, 40.0));

                // Selected switch.
                pump(tester, true, false);
                let painted = drawn(tester);
                assert_eq!(painted[0].color(), Some(c(SELECTED_TRACK)));
                assert!(matches!(painted[1], Drawn::Stroke(color, width, _)
                    if color == c(SELECTED_OUTLINE) && f64::from(width) == SELECTED_OUTLINE_WIDTH));
                assert_eq!(painted[thumb_index].color(), Some(c(SELECTED_THUMB)));

                // Switch with hover.
                pump(tester, false, false);
                hover(tester);
                tester.pump_and_settle();
                assert_eq!(tester.active_cursor(1), SystemMouseCursors::TEXT.into());
                assert_eq!(circles(&drawn(tester), SPLASH), [c(HOVER_COLOR)]);

                // Switch with focus.
                pump(tester, false, true);
                assert_eq!(circles(&drawn(tester), SPLASH), [c(FOCUS_COLOR)]);
            });
        }
    }

    /// Dart's 'Material2 - Switch active and inactive properties are taken over the theme
    /// values' and its Material 3 twin.
    #[test]
    #[allow(deprecated)]
    fn the_switch_active_and_inactive_colors_win_over_the_theme_values() {
        for use_material3 in [false, true] {
            test_widgets(|tester| {
                const DEFAULT_THUMB: Color = Color::new(0xffffff0f);
                const SELECTED_THUMB: Color = Color::new(0xffffff1f);
                const DEFAULT_TRACK: Color = Color::new(0xffffff2f);
                const SELECTED_TRACK: Color = Color::new(0xffffff3f);
                let outline = ThemeData::new(&tester.cell.borrow()).color_scheme.outline();
                let switch_theme = SwitchThemeData::new()
                    .thumb_color(by_selected(Color::new(0xfffffff1), Color::new(0xfffffff0)))
                    .track_color(by_selected(Color::new(0xfffffff3), Color::new(0xfffffff2)));
                for selected in [false, true] {
                    let switch_theme = switch_theme.clone();
                    pump_scaffold(
                        tester,
                        move |app| {
                            ThemeData::builder()
                                .use_material3(use_material3)
                                .switch_theme(switch_theme)
                                .build(app)
                        },
                        Switch::new(selected, ignore())
                            .active_color(SELECTED_THUMB)
                            .inactive_thumb_color(DEFAULT_THUMB)
                            .active_track_color(SELECTED_TRACK)
                            .inactive_track_color(DEFAULT_TRACK),
                    );
                    tester.pump_and_settle();
                    let (track, thumb) = if selected {
                        (SELECTED_TRACK, SELECTED_THUMB)
                    } else {
                        (DEFAULT_TRACK, DEFAULT_THUMB)
                    };
                    // The Material 2 outline, and the selected Material 3 one, are transparent
                    // and not recorded.
                    let expected = match (use_material3, selected) {
                        (false, _) => [&[c(track)][..], &M2_SHADOWS[..], &[c(thumb)][..]].concat(),
                        (true, true) => vec![c(track), c(thumb)],
                        (true, false) => vec![c(track), c(outline), c(thumb)],
                    };
                    assert_eq!(
                        colors(&drawn(tester)),
                        expected,
                        "M3 {use_material3}, selected {selected}"
                    );
                }
            });
        }
    }

    /// Dart's 'Material2 - Switch theme overlay color resolves in active/pressed states' and its
    /// Material 3 twin.
    #[test]
    fn the_theme_overlay_color_resolves_in_the_active_and_pressed_states() {
        for use_material3 in [false, true] {
            test_widgets(|tester| {
                const ACTIVE_PRESSED_OVERLAY: Color = Color::new(0xFF000001);
                const INACTIVE_PRESSED_OVERLAY: Color = Color::new(0xFF000002);
                const SPLASH: f64 = 24.0;
                let switch_theme = SwitchThemeData::new()
                    .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                        |states: &WidgetStates| {
                            if states.contains(&WidgetState::Pressed) {
                                if states.contains(&WidgetState::Selected) {
                                    return Some(ACTIVE_PRESSED_OVERLAY);
                                }
                                return Some(INACTIVE_PRESSED_OVERLAY);
                            }
                            None
                        },
                    ))
                    .splash_radius(SPLASH);

                for (active, expected) in [
                    (false, INACTIVE_PRESSED_OVERLAY),
                    (true, ACTIVE_PRESSED_OVERLAY),
                ] {
                    let switch_theme = switch_theme.clone();
                    pump_scaffold(
                        tester,
                        move |app| {
                            ThemeData::builder()
                                .use_material3(use_material3)
                                .switch_theme(switch_theme)
                                .build(app)
                        },
                        Switch::new(active, ignore()),
                    );
                    let touch = press(tester);
                    tester.pump_and_settle();
                    assert_eq!(
                        circles(&drawn(tester), SPLASH),
                        [c(expected)],
                        "M3 {use_material3}, active {active}"
                    );
                    tester.up(&touch);
                    tester.pump_and_settle();
                }
            });
        }
    }

    /// Dart's 'Material2 - Local SwitchTheme can override global SwitchTheme' and its
    /// Material 3 twin.
    #[test]
    fn a_local_switch_theme_overrides_the_global_one() {
        for use_material3 in [false, true] {
            test_widgets(|tester| {
                const LOCAL: Color = Color::new(0xffff0000);
                const LOCAL_OUTLINE_WIDTH: f64 = 4.0;
                let global = SwitchThemeData::new()
                    .thumb_color(all(Color::new(0xfffffff1)))
                    .track_color(all(Color::new(0xfffffff2)))
                    .track_outline_color(all(Color::new(0xfffffff3)))
                    .track_outline_width(all(6.0));
                let local = SwitchThemeData::new()
                    .thumb_color(all(LOCAL))
                    .track_color(all(LOCAL))
                    .track_outline_color(all(LOCAL))
                    .track_outline_width(all(LOCAL_OUTLINE_WIDTH));
                let home = Scaffold::new()
                    .body(SwitchTheme::new(local, Switch::new(true, ignore())))
                    .into_widget();
                pump_app(
                    tester,
                    move |app| {
                        ThemeData::builder()
                            .use_material3(use_material3)
                            .switch_theme(global)
                            .build(app)
                    },
                    move |_app| home,
                );
                tester.pump_and_settle();
                let painted = drawn(tester);
                assert_eq!(painted[0].color(), Some(c(LOCAL)));
                assert!(matches!(painted[1], Drawn::Stroke(color, width, _)
                    if color == c(LOCAL) && f64::from(width) == LOCAL_OUTLINE_WIDTH));
                let thumb_shadows = if use_material3 {
                    &[][..]
                } else {
                    &M2_SHADOWS[..]
                };
                assert_eq!(
                    colors(&painted),
                    [&[c(LOCAL), c(LOCAL)][..], thumb_shadows, &[c(LOCAL)][..]].concat(),
                    "M3 {use_material3}"
                );
            });
        }
    }

    #[test]
    fn the_switch_theme_padding_is_respected() {
        test_widgets(|tester| {
            for (padding, expected) in [
                (None, Size::new(60.0, 48.0)),
                (Some(EdgeInsetsGeometry::ZERO), Size::new(52.0, 48.0)),
                (Some(EdgeInsetsGeometry::all(4.0)), Size::new(60.0, 56.0)),
            ] {
                let mut switch_theme = SwitchThemeData::new();
                switch_theme.padding = padding;
                let home = Scaffold::new()
                    .body(Center::new().child(Switch::new(true, ignore())))
                    .into_widget();
                pump_app(
                    tester,
                    move |app| ThemeData::builder().switch_theme(switch_theme).build(app),
                    move |_app| home,
                );
                tester.pump_and_settle();
                assert_eq!(switch_rect(tester).size(), expected, "{padding:?}");
            }
        });
    }
}
