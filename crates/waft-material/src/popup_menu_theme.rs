//! Flutter counterpart: material_ui's `popup_menu_theme.dart`.

use waft_embedder::{Color, lerp_double};
use waft_foundation::App;
use waft_painting::{EdgeInsetsGeometry, ShapeBorderRef, TextStyle};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef,
    WidgetStateProperty, WidgetStatePropertyRef,
};

use crate::theme::Theme;

/// Used to configure how the `PopupMenuButton` positions its popup menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupMenuPosition {
    /// Menu is positioned over the anchor.
    Over,

    /// Menu is positioned under the anchor.
    Under,
}

/// Defines the visual properties of the routes used to display popup menus
/// as well as `PopupMenuItem` and `PopupMenuDivider` widgets.
///
/// Descendant widgets obtain the current [`PopupMenuThemeData`] object
/// using [`PopupMenuTheme::of`]. Instances of [`PopupMenuThemeData`] can be
/// customized with [`PopupMenuThemeData::copy_with`].
///
/// Typically, a [`PopupMenuThemeData`] is specified as part of the
/// overall `Theme` with `ThemeData::popup_menu_theme`. Otherwise,
/// [`PopupMenuTheme`] can be used to configure its own widget subtree.
///
/// All [`PopupMenuThemeData`] properties are `null` by default.
/// If any of these properties are null, the popup menu will provide its
/// own defaults.
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug, Default)]
pub struct PopupMenuThemeData {
    /// The background color of the popup menu.
    pub color: Option<Color>,

    /// The shape of the popup menu.
    pub shape: Option<ShapeBorderRef>,

    /// If specified, the padding of the popup menu.
    ///
    /// If `PopupMenuButton::menu_padding` is provided, [`menu_padding`](Self::menu_padding) is
    /// ignored.
    pub menu_padding: Option<EdgeInsetsGeometry>,

    /// The elevation of the popup menu.
    pub elevation: Option<f64>,

    /// The color used to paint shadow below the popup menu.
    pub shadow_color: Option<Color>,

    /// The color used as an overlay on [`color`](Self::color) of the popup menu.
    pub surface_tint_color: Option<Color>,

    /// The text style of items in the popup menu.
    pub text_style: Option<TextStyle>,

    /// You can use this to specify a different style of the label
    /// when the popup menu item is enabled and disabled.
    pub label_text_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,

    /// If specified, defines the feedback property for `PopupMenuButton`.
    ///
    /// If `PopupMenuButton::enable_feedback` is provided,
    /// [`enable_feedback`](Self::enable_feedback) is ignored.
    pub enable_feedback: Option<bool>,

    /// {@macro material_ui.popupmenu.mouseCursor}
    ///
    /// If specified, overrides the default value of `PopupMenuItem::mouse_cursor`.
    pub mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,

    /// Whether the popup menu is positioned over or under the popup menu button.
    ///
    /// When not set, the position defaults to [`PopupMenuPosition::Over`] which makes the
    /// popup menu appear directly over the button that was used to create it.
    pub position: Option<PopupMenuPosition>,

    /// The color of the icon in the popup menu button.
    pub icon_color: Option<Color>,

    /// The size of the icon in the popup menu button.
    pub icon_size: Option<f64>,
}

crate::theme_data_getters::theme_data_getters! {
    /// Dart's getters of `PopupMenuThemeData`: the theme data answers with its fields, and a defaults
    /// subclass (Dart's `_...Defaults...(context)`) computes the ones it overrides when they
    /// are read.
    pub trait PopupMenuThemeDataGetters for PopupMenuThemeData, fields: popup_menu_theme_data {
        color: Option<Color>,
        shape: Option<ShapeBorderRef>,
        menu_padding: Option<EdgeInsetsGeometry>,
        elevation: Option<f64>,
        shadow_color: Option<Color>,
        surface_tint_color: Option<Color>,
        text_style: Option<TextStyle>,
        label_text_style: Option<WidgetStatePropertyRef<Option<TextStyle>>>,
        enable_feedback: Option<bool>,
        mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,
        position: Option<PopupMenuPosition>,
        icon_color: Option<Color>,
        icon_size: Option<f64>,
    }
}

impl PopupMenuThemeData {
    /// Creates the set of properties used to configure [`PopupMenuTheme`].
    pub fn new() -> PopupMenuThemeData {
        PopupMenuThemeData::default()
    }

    /// Dart `PopupMenuThemeData(color:)`.
    pub fn color(mut self, color: Color) -> PopupMenuThemeData {
        self.color = Some(color);
        self
    }

    /// Dart `PopupMenuThemeData(shape:)`.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> PopupMenuThemeData {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `PopupMenuThemeData(menuPadding:)`.
    pub fn menu_padding(
        mut self,
        menu_padding: impl Into<EdgeInsetsGeometry>,
    ) -> PopupMenuThemeData {
        let menu_padding = menu_padding.into();
        self.menu_padding = Some(menu_padding);
        self
    }

    /// Dart `PopupMenuThemeData(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> PopupMenuThemeData {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `PopupMenuThemeData(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> PopupMenuThemeData {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `PopupMenuThemeData(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> PopupMenuThemeData {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `PopupMenuThemeData(textStyle:)`.
    pub fn text_style(mut self, text_style: TextStyle) -> PopupMenuThemeData {
        self.text_style = Some(text_style);
        self
    }

    /// Dart `PopupMenuThemeData(labelTextStyle:)`.
    pub fn label_text_style(
        mut self,
        label_text_style: WidgetStatePropertyRef<Option<TextStyle>>,
    ) -> PopupMenuThemeData {
        self.label_text_style = Some(label_text_style);
        self
    }

    /// Dart `PopupMenuThemeData(enableFeedback:)`.
    pub fn enable_feedback(mut self, enable_feedback: bool) -> PopupMenuThemeData {
        self.enable_feedback = Some(enable_feedback);
        self
    }

    /// Dart `PopupMenuThemeData(mouseCursor:)`.
    pub fn mouse_cursor(
        mut self,
        mouse_cursor: WidgetStatePropertyRef<Option<MouseCursorRef>>,
    ) -> PopupMenuThemeData {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `PopupMenuThemeData(position:)`.
    pub fn position(mut self, position: PopupMenuPosition) -> PopupMenuThemeData {
        self.position = Some(position);
        self
    }

    /// Dart `PopupMenuThemeData(iconColor:)`.
    pub fn icon_color(mut self, icon_color: Color) -> PopupMenuThemeData {
        self.icon_color = Some(icon_color);
        self
    }

    /// Dart `PopupMenuThemeData(iconSize:)`.
    pub fn icon_size(mut self, icon_size: f64) -> PopupMenuThemeData {
        self.icon_size = Some(icon_size);
        self
    }

    /// Creates a copy of this object with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().elevation(2.0)`).
    pub fn copy_with(&self) -> PopupMenuThemeData {
        self.clone()
    }

    /// Linearly interpolate between two popup menu themes.
    ///
    /// If both arguments are null, then null is returned.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(
        a: Option<&PopupMenuThemeData>,
        b: Option<&PopupMenuThemeData>,
        t: f64,
    ) -> Option<PopupMenuThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(PopupMenuThemeData {
            color: Color::lerp(a.and_then(|a| a.color), b.and_then(|b| b.color), t),
            shape: ShapeBorderRef::lerp(
                a.and_then(|a| a.shape.as_ref()),
                b.and_then(|b| b.shape.as_ref()),
                t,
            ),
            menu_padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.menu_padding),
                b.and_then(|b| b.menu_padding),
                t,
            ),
            elevation: lerp_double(a.and_then(|a| a.elevation), b.and_then(|b| b.elevation), t),
            shadow_color: Color::lerp(
                a.and_then(|a| a.shadow_color),
                b.and_then(|b| b.shadow_color),
                t,
            ),
            surface_tint_color: Color::lerp(
                a.and_then(|a| a.surface_tint_color),
                b.and_then(|b| b.surface_tint_color),
                t,
            ),
            text_style: TextStyle::lerp(
                a.and_then(|a| a.text_style.as_ref()),
                b.and_then(|b| b.text_style.as_ref()),
                t,
            ),
            label_text_style: <dyn WidgetStateProperty<Option<TextStyle>>>::lerp_optional(
                a.and_then(|a| a.label_text_style.clone()),
                b.and_then(|b| b.label_text_style.clone()),
                t,
                |a, b, t| TextStyle::lerp(a.as_ref(), b.as_ref(), t),
            ),
            enable_feedback: pick(a, b, t, |theme| theme.enable_feedback),
            mouse_cursor: pick(a, b, t, |theme| theme.mouse_cursor.clone()),
            position: pick(a, b, t, |theme| theme.position),
            icon_color: Color::lerp(
                a.and_then(|a| a.icon_color),
                b.and_then(|b| b.icon_color),
                t,
            ),
            icon_size: lerp_double(a.and_then(|a| a.icon_size), b.and_then(|b| b.icon_size), t),
        })
    }
}

/// Dart's `t < 0.5 ? a?.field : b?.field`.
fn pick<T>(
    a: Option<&PopupMenuThemeData>,
    b: Option<&PopupMenuThemeData>,
    t: f64,
    field: impl Fn(&PopupMenuThemeData) -> Option<T>,
) -> Option<T> {
    if t < 0.5 {
        a.and_then(&field)
    } else {
        b.and_then(&field)
    }
}

impl PartialEq for PopupMenuThemeData {
    fn eq(&self, other: &PopupMenuThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.color == self.color
            && other.shape == self.shape
            && other.menu_padding == self.menu_padding
            && other.elevation == self.elevation
            && other.shadow_color == self.shadow_color
            && other.surface_tint_color == self.surface_tint_color
            && other.text_style == self.text_style
            && other.label_text_style == self.label_text_style
            && other.enable_feedback == self.enable_feedback
            && other.mouse_cursor == self.mouse_cursor
            && other.position == self.position
            && other.icon_color == self.icon_color
            && other.icon_size == self.icon_size
    }
}

/// An inherited widget that defines the configuration for
/// popup menus in this widget's subtree.
///
/// Values specified here are used for popup menu properties that are not
/// given an explicit non-null value.
#[derive(Debug)]
pub struct PopupMenuTheme {
    pub key: Option<KeyRef>,
    /// The properties for descendant popup menu widgets.
    pub data: PopupMenuThemeData,
    pub child: WidgetRef,
}

impl PopupMenuTheme {
    /// Creates a popup menu theme that controls the configurations for
    /// popup menus in its widget subtree.
    pub fn new<K>(data: PopupMenuThemeData, child: impl IntoWidget<K>) -> PopupMenuTheme {
        PopupMenuTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `PopupMenuTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> PopupMenuTheme {
        self.key = Some(key);
        self
    }

    /// The closest instance of this class's [`data`](Self::data) value that encloses the given
    /// context. If there is no ancestor, it returns `ThemeData::popup_menu_theme`.
    /// Applications can assume that the returned value will not be null.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = PopupMenuTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> PopupMenuThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<PopupMenuTheme>(app) {
            Some(popup_menu_theme) => popup_menu_theme.data.clone(),
            None => Theme::of(app, context).popup_menu_theme.clone(),
        }
    }
}

impl InheritedTheme for PopupMenuTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        PopupMenuTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for PopupMenuTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &PopupMenuTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    //! Ported from `test/popup_menu_theme_test.dart`; the `debugFillProperties` cases wait
    //! for diagnostics, and `hashCode` for `Hash`.

    use waft_embedder::{Offset, Radius, Size};
    use waft_painting::{
        BeveledRectangleBorder, BorderRadiusGeometry, BorderSide, RoundedRectangleBorder,
    };
    use waft_services::{SystemMouseCursor, SystemMouseCursors};
    use waft_widgets::{
        AnyElement, Column, DefaultTextStyle, GlobalKey, Padding, SingleChildScrollView, Text,
        WidgetState, WidgetStates,
    };

    use waft_widgets::testing::{TestPointer, WidgetTester, find, test_widgets};

    use super::*;
    use crate::colors::Colors;
    use crate::icons::Icons;
    use crate::list_tile::ListTile;
    use crate::material::Material;
    use crate::popup_menu::{
        CheckedPopupMenuItem, PopupMenuButton, PopupMenuDivider, PopupMenuEntryRef, PopupMenuItem,
    };
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m2, m3, pump_app};

    /// Builds `wrap(probe)` and returns what `read` saw at the probe's context.
    fn probe<T: 'static>(
        tester: &mut WidgetTester,
        wrap: impl FnOnce(&App, WidgetRef) -> WidgetRef,
        read: impl Fn(&mut App, BuildContext) -> T + 'static,
    ) -> T {
        let seen = std::rc::Rc::new(std::cell::RefCell::new(None));
        let probe = waft_widgets::Builder::new({
            let seen = std::rc::Rc::clone(&seen);
            move |app, context| {
                *seen.borrow_mut() = Some(read(app, context));
                waft_widgets::SizedBox::shrink().into_widget()
            }
        })
        .into_widget();
        let root = wrap(&tester.cell.borrow(), probe);
        tester.pump_widget(root);
        let value = seen.borrow_mut().take();
        value.expect("the probe built")
    }

    fn cursor_resolver() -> WidgetStatePropertyRef<Option<MouseCursorRef>> {
        <dyn WidgetStateProperty<Option<MouseCursorRef>>>::resolve_with(|states| {
            if states.contains(&WidgetState::Disabled) {
                return Some(SystemMouseCursors::CONTEXT_MENU.into());
            }
            Some(SystemMouseCursors::ALIAS.into())
        })
    }

    /// Dart's `_popupMenuThemeM2()`.
    fn popup_menu_theme_m2() -> PopupMenuThemeData {
        PopupMenuThemeData::new()
            .color(Colors::ORANGE.primary())
            .shape(BeveledRectangleBorder::new(
                BorderSide::NONE,
                BorderRadiusGeometry::all(Radius::circular(12.0)),
            ))
            .elevation(12.0)
            .text_style(
                TextStyle::new()
                    .color(Color::new(0xffffffff))
                    .text_baseline(waft_embedder::TextBaseline::Alphabetic),
            )
            .mouse_cursor(cursor_resolver())
    }

    /// Dart's `_popupMenuThemeM3()`.
    fn popup_menu_theme_m3() -> PopupMenuThemeData {
        PopupMenuThemeData::new()
            .color(Colors::ORANGE.primary())
            .shape(BeveledRectangleBorder::new(
                BorderSide::NONE,
                BorderRadiusGeometry::all(Radius::circular(12.0)),
            ))
            .menu_padding(EdgeInsetsGeometry::symmetric(9.0, 0.0))
            .elevation(12.0)
            .shadow_color(Color::new(0xff00ff00))
            .surface_tint_color(Color::new(0xff00ff00))
            .label_text_style(<dyn WidgetStateProperty<Option<TextStyle>>>::resolve_with(
                |states| {
                    if states.contains(&WidgetState::Disabled) {
                        return Some(
                            TextStyle::new()
                                .color(Color::new(0xfff99ff0))
                                .font_size(12.0),
                        );
                    }
                    Some(
                        TextStyle::new()
                            .color(Color::new(0xfff12099))
                            .font_size(17.0),
                    )
                },
            ))
            .mouse_cursor(cursor_resolver())
            .icon_color(Color::new(0xfff12099))
            .icon_size(17.0)
    }

    fn enabled() -> WidgetStates {
        WidgetStates::new()
    }

    fn disabled() -> WidgetStates {
        let mut states = WidgetStates::new();
        states.insert(WidgetState::Disabled);
        states
    }

    #[test]
    fn popup_menu_theme_data_copy_with_equality_basics() {
        assert_eq!(
            PopupMenuThemeData::new(),
            PopupMenuThemeData::new().copy_with()
        );
    }

    #[test]
    fn popup_menu_theme_data_lerp_special_cases() {
        assert!(PopupMenuThemeData::lerp(None, None, 0.0).is_none());
        let data = PopupMenuThemeData::new();
        assert_eq!(
            PopupMenuThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data)
        );
    }

    #[test]
    fn popup_menu_theme_data_null_fields_by_default() {
        let popup_menu_theme = PopupMenuThemeData::new();
        assert_eq!(popup_menu_theme.color, None);
        assert!(popup_menu_theme.shape.is_none());
        assert_eq!(popup_menu_theme.menu_padding, None);
        assert_eq!(popup_menu_theme.elevation, None);
        assert_eq!(popup_menu_theme.shadow_color, None);
        assert_eq!(popup_menu_theme.surface_tint_color, None);
        assert_eq!(popup_menu_theme.text_style, None);
        assert!(popup_menu_theme.label_text_style.is_none());
        assert_eq!(popup_menu_theme.enable_feedback, None);
        assert!(popup_menu_theme.mouse_cursor.is_none());
    }

    fn element_of(tester: &WidgetTester, key: &GlobalKey) -> AnyElement {
        key.current_context(&tester.cell.borrow())
            .expect("the keyed widget is mounted")
    }

    /// The last `Material` in the tree: the menu's.
    fn menu_material(tester: &WidgetTester) -> AnyElement {
        *find::by_type::<Material>()
            .evaluate(tester)
            .last()
            .expect("the menu's Material")
    }

    /// Dart's `find.descendant(of: find.byKey(key), matching: find.byType(DefaultTextStyle))
    /// .last`'s style.
    fn item_text_style(tester: &WidgetTester, key: &GlobalKey) -> TextStyle {
        let item = element_of(tester, key);
        let text_style = *tester
            .descendants_of_type::<DefaultTextStyle>(item)
            .last()
            .expect("a DefaultTextStyle in the item");
        tester.widget(text_style, |text_style: &DefaultTextStyle| {
            text_style.style.clone()
        })
    }

    /// The `n`th `ListTile`'s `title_text_style`.
    fn list_tile_title_text_style(tester: &WidgetTester, last: bool) -> Option<TextStyle> {
        let tiles = find::by_type::<ListTile>().evaluate(tester);
        let tile = if last { tiles.last() } else { tiles.first() };
        tester.widget(*tile.expect("a ListTile"), |tile: &ListTile| {
            tile.title_text_style.clone()
        })
    }

    fn menu_padding(tester: &WidgetTester) -> Option<EdgeInsetsGeometry> {
        let scroll_view = tester.element(find::by_type::<SingleChildScrollView>());
        tester.widget(scroll_view, |scroll_view: &SingleChildScrollView| {
            scroll_view.padding
        })
    }

    fn icon_color_and_size(tester: &WidgetTester) -> (Option<Color>, Size) {
        let more = Icons::adaptive().more(&tester.cell.borrow());
        let icon = tester.element(find::by_icon(&more).first());
        (
            tester.glyph_style(icon).color.as_deref().copied(),
            tester.get_rect(tester.render_box_of(icon)).size(),
        )
    }

    fn cursor(cursor: SystemMouseCursor) -> MouseCursorRef {
        cursor.into()
    }

    /// Dart's mouse gesture of the cases: added at the origin, moved over the item keyed
    /// `over`, and, with `down`, pressed over the item keyed `down`; the active cursor after
    /// each.
    fn cursors_over(
        tester: &mut WidgetTester,
        over: &GlobalKey,
        down: Option<&GlobalKey>,
    ) -> (MouseCursorRef, Option<MouseCursorRef>) {
        let mut gesture = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
        tester.add_pointer(&mut gesture, Offset::ZERO);
        let at = tester.get_center(over);
        tester.move_to(&mut gesture, at, false);
        tester.pump_and_settle();
        let over_cursor = tester.active_cursor(1);
        let down_cursor = down.map(|down| {
            let at = tester.get_center(down);
            tester.down(&mut gesture, at);
            tester.pump_and_settle();
            tester.active_cursor(1)
        });
        (over_cursor, down_cursor)
    }

    /// The column with the padded or bare button keyed `button_key` of the cases.
    fn button_column(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        padded: bool,
        button: PopupMenuButton<()>,
    ) {
        pump_app(tester, theme, move |_app| {
            let button = if padded {
                // The padding makes sure the menu has enough space around it to
                // get properly aligned when displayed (`_kMenuScreenPadding`).
                Padding::new(EdgeInsetsGeometry::all(8.0))
                    .child(button)
                    .into_widget()
            } else {
                button.into_widget()
            };
            Material::new()
                .child(Column::new().children(vec![button]))
                .into_widget()
        });
    }

    fn keyed_item(key: &GlobalKey, text: &str, enabled: bool) -> PopupMenuEntryRef<()> {
        PopupMenuEntryRef::new(
            PopupMenuItem::new(Some(Text::new(text)))
                .key(key.clone().into())
                .enabled(enabled),
        )
    }

    fn checked_item(checked: bool, text: &str) -> PopupMenuEntryRef<()> {
        PopupMenuEntryRef::new(
            CheckedPopupMenuItem::new()
                .checked(checked)
                .child(Text::new(text)),
        )
    }

    #[test]
    fn passing_no_popup_menu_theme_data_returns_defaults() {
        test_widgets(|tester| {
            let popup_button_key = GlobalKey::new();
            let enabled_popup_item_key = GlobalKey::new();
            let disabled_popup_item_key = GlobalKey::new();
            let theme = m3(&tester.cell.borrow());
            let (enabled_key, disabled_key) = (
                enabled_popup_item_key.clone(),
                disabled_popup_item_key.clone(),
            );
            button_column(
                tester,
                m3,
                true,
                PopupMenuButton::new(move |_app, _context| {
                    vec![
                        keyed_item(&enabled_key, "Enabled PopupMenuItem", true),
                        PopupMenuEntryRef::new(PopupMenuDivider::new()),
                        keyed_item(&disabled_key, "Disabled PopupMenuItem", false),
                        checked_item(false, "Unchecked item"),
                        checked_item(true, "Checked item"),
                    ]
                })
                .key(popup_button_key.clone().into()),
            );

            // Test default button icon color.
            assert_eq!(
                icon_color_and_size(tester).0,
                theme.icon_theme.color.as_deref().copied()
            );

            let at = tester.get_center(&popup_button_key);
            tester.tap_at(at, 1);
            tester.pump_and_settle();

            let button = menu_material(tester);
            tester.widget(button, |button: &Material| {
                assert_eq!(button.color, Some(theme.color_scheme.surface_container()));
                assert_eq!(button.shadow_color, Some(theme.color_scheme.shadow()));
                assert_eq!(button.surface_tint_color, Some(Colors::TRANSPARENT));
                let shape: ShapeBorderRef = ShapeBorderRef::new(RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadiusGeometry::all(Radius::circular(4.0)),
                ));
                assert_eq!(button.shape, Some(shape));
                assert_eq!(button.elevation, 3.0);
            });

            let label = item_text_style(tester, &enabled_popup_item_key);
            assert_eq!(label.font_family.as_deref(), Some("Roboto"));
            assert_eq!(
                label.color.as_deref().copied(),
                Some(theme.color_scheme.on_surface())
            );

            // Test disabled text color
            #[allow(deprecated)]
            let disabled_color = theme.color_scheme.on_surface().with_opacity(0.38);
            let label = item_text_style(tester, &disabled_popup_item_key);
            assert_eq!(label.color.as_deref().copied(), Some(disabled_color));

            let top_left_button = tester
                .get_rect(
                    tester.render_box_of(tester.element(find::by_type::<PopupMenuButton<()>>())),
                )
                .top_left();
            let top_left_menu = tester.get_rect(tester.render_box_of(button)).top_left();
            assert_eq!(top_left_menu, top_left_button);

            let (over, down) = cursors_over(
                tester,
                &disabled_popup_item_key,
                Some(&enabled_popup_item_key),
            );
            assert!(*over == *cursor(SystemMouseCursors::BASIC));
            assert!(*down.expect("pressed") == *cursor(SystemMouseCursors::BASIC));

            // Test unchecked CheckedPopupMenuItem label.
            let title = list_tile_title_text_style(tester, false);
            assert_eq!(
                title.and_then(|style| style.color.as_deref().copied()),
                Some(theme.color_scheme.on_surface())
            );

            // Test checked CheckedPopupMenuItem label.
            let title = list_tile_title_text_style(tester, true);
            assert_eq!(
                title.and_then(|style| style.color.as_deref().copied()),
                Some(theme.color_scheme.on_surface())
            );

            // Check popup menu padding.
            assert_eq!(
                menu_padding(tester),
                Some(EdgeInsetsGeometry::symmetric(8.0, 0.0))
            );
        });
    }

    #[test]
    fn popup_menu_uses_values_from_popup_menu_theme_data() {
        test_widgets(|tester| {
            let popup_menu_theme = popup_menu_theme_m3();
            let popup_button_key = GlobalKey::new();
            let enabled_popup_item_key = GlobalKey::new();
            let disabled_popup_item_key = GlobalKey::new();
            let (enabled_key, disabled_key) = (
                enabled_popup_item_key.clone(),
                disabled_popup_item_key.clone(),
            );
            let theme = popup_menu_theme.clone();
            button_column(
                tester,
                move |app| ThemeData::builder().popup_menu_theme(theme).build(app),
                false,
                PopupMenuButton::new(move |_app, _context| {
                    vec![
                        keyed_item(&disabled_key, "disabled", false),
                        PopupMenuEntryRef::new(PopupMenuDivider::new()),
                        PopupMenuEntryRef::new(
                            PopupMenuItem::new(Some(Text::new("enabled")))
                                .key(enabled_key.clone().into())
                                .on_tap(waft_foundation::Listener::new(|_app| {})),
                        ),
                        checked_item(false, "Unchecked item"),
                        checked_item(true, "Checked item"),
                    ]
                })
                // The padding is used in the positioning of the menu when the
                // position is `PopupMenuPosition.under`. Setting it to zero makes
                // it easier to test.
                .padding(EdgeInsetsGeometry::ZERO)
                .key(popup_button_key.clone().into()),
            );

            assert_eq!(
                icon_color_and_size(tester),
                (popup_menu_theme.icon_color, Size::new(17.0, 17.0))
            );

            let at = tester.get_center(&popup_button_key);
            tester.tap_at(at, 1);
            tester.pump_and_settle();

            let button = menu_material(tester);
            tester.widget(button, |button: &Material| {
                assert_eq!(button.color, Some(Colors::ORANGE.primary()));
                assert_eq!(button.surface_tint_color, Some(Color::new(0xff00ff00)));
                assert_eq!(button.shadow_color, Some(Color::new(0xff00ff00)));
                assert_eq!(button.shape, popup_menu_theme.shape);
                assert_eq!(button.elevation, 12.0);
            });

            let label_text_style = popup_menu_theme.label_text_style.clone().expect("set");
            assert_eq!(
                Some(item_text_style(tester, &enabled_popup_item_key)),
                label_text_style.resolve(&enabled())
            );

            // Test disabled text color
            assert_eq!(
                Some(item_text_style(tester, &disabled_popup_item_key)),
                label_text_style.resolve(&disabled())
            );

            let mouse_cursor = popup_menu_theme.mouse_cursor.clone().expect("set");
            let (over, down) = cursors_over(
                tester,
                &disabled_popup_item_key,
                Some(&enabled_popup_item_key),
            );
            assert!(*over == *mouse_cursor.resolve(&disabled()).expect("a cursor"));
            assert!(
                *down.expect("pressed") == *mouse_cursor.resolve(&enabled()).expect("a cursor")
            );

            // Test unchecked CheckedPopupMenuItem label.
            assert_eq!(
                list_tile_title_text_style(tester, false),
                label_text_style.resolve(&enabled())
            );

            // Test checked CheckedPopupMenuItem label.
            assert_eq!(
                list_tile_title_text_style(tester, true),
                label_text_style.resolve(&enabled())
            );

            // Check popup menu padding.
            assert_eq!(menu_padding(tester), popup_menu_theme.menu_padding);
        });
    }

    #[test]
    fn popup_menu_widget_properties_take_priority_over_theme() {
        test_widgets(|tester| {
            let popup_menu_theme = popup_menu_theme_m3();
            let popup_button_key = GlobalKey::new();
            let popup_item_key = GlobalKey::new();

            let color = Color::new(0xfff11fff);
            let surface_tint_color = Color::new(0xfff12fff);
            let shadow_color = Color::new(0xfff13fff);
            let shape = RoundedRectangleBorder::new(
                BorderSide::NONE,
                BorderRadiusGeometry::all(Radius::circular(9.0)),
            );
            let elevation = 7.0;
            let text_style = TextStyle::new()
                .color(Color::new(0xfff14fff))
                .font_size(19.0);
            let icon_color = Color::new(0xfff15fff);
            let icon_size = 21.5;

            let item_key = popup_item_key.clone();
            let style = text_style.clone();
            button_column(
                tester,
                move |app| {
                    ThemeData::builder()
                        .popup_menu_theme(popup_menu_theme)
                        .build(app)
                },
                false,
                PopupMenuButton::new(move |_app, _context| {
                    vec![
                        PopupMenuEntryRef::new(
                            PopupMenuItem::new(Some(Text::new("Example")))
                                .key(item_key.clone().into())
                                .label_text_style(
                                    <dyn WidgetStateProperty<Option<TextStyle>>>::all(Some(
                                        style.clone(),
                                    )),
                                )
                                .mouse_cursor(cursor(SystemMouseCursors::FORBIDDEN)),
                        ),
                        PopupMenuEntryRef::new(
                            CheckedPopupMenuItem::new()
                                .checked(true)
                                .label_text_style(
                                    <dyn WidgetStateProperty<Option<TextStyle>>>::all(Some(
                                        style.clone(),
                                    )),
                                )
                                .child(Text::new("Checked item")),
                        ),
                    ]
                })
                .key(popup_button_key.clone().into())
                .elevation(elevation)
                .shadow_color(shadow_color)
                .surface_tint_color(surface_tint_color)
                .color(color)
                .shape(shape)
                .menu_padding(EdgeInsetsGeometry::ZERO)
                .icon_color(icon_color)
                .icon_size(icon_size),
            );

            assert_eq!(
                icon_color_and_size(tester),
                (Some(icon_color), Size::new(icon_size, icon_size))
            );

            let at = tester.get_center(&popup_button_key);
            tester.tap_at(at, 1);
            tester.pump_and_settle();

            let button = menu_material(tester);
            tester.widget(button, |button: &Material| {
                assert_eq!(button.color, Some(color));
                let shape: ShapeBorderRef = ShapeBorderRef::new(shape);
                assert_eq!(button.shape, Some(shape));
                assert_eq!(button.elevation, elevation);
                assert_eq!(button.shadow_color, Some(shadow_color));
                assert_eq!(button.surface_tint_color, Some(surface_tint_color));
            });

            assert_eq!(item_text_style(tester, &popup_item_key), text_style);

            let (over, _) = cursors_over(tester, &popup_item_key, None);
            assert!(*over == *cursor(SystemMouseCursors::FORBIDDEN));

            // Test CheckedPopupMenuItem label.
            assert_eq!(list_tile_title_text_style(tester, false), Some(text_style));

            // Check popup menu padding.
            assert_eq!(menu_padding(tester), Some(EdgeInsetsGeometry::ZERO));
        });
    }

    #[test]
    fn material2_passing_no_popup_menu_theme_data_returns_defaults() {
        test_widgets(|tester| {
            let popup_button_key = GlobalKey::new();
            let enabled_popup_item_key = GlobalKey::new();
            let disabled_popup_item_key = GlobalKey::new();
            let theme = m2(&tester.cell.borrow());
            let (enabled_key, disabled_key) = (
                enabled_popup_item_key.clone(),
                disabled_popup_item_key.clone(),
            );
            button_column(
                tester,
                m2,
                true,
                PopupMenuButton::new(move |_app, _context| {
                    vec![
                        keyed_item(&enabled_key, "Enabled PopupMenuItem", true),
                        PopupMenuEntryRef::new(PopupMenuDivider::new()),
                        keyed_item(&disabled_key, "Disabled PopupMenuItem", false),
                    ]
                })
                .key(popup_button_key.clone().into()),
            );

            let at = tester.get_center(&popup_button_key);
            tester.tap_at(at, 1);
            tester.pump_and_settle();

            let button = menu_material(tester);
            tester.widget(button, |button: &Material| {
                assert_eq!(button.color, None);
                assert!(button.shape.is_none());
                assert_eq!(button.elevation, 8.0);
            });

            let enabled_text = item_text_style(tester, &enabled_popup_item_key);
            assert_eq!(enabled_text.font_family.as_deref(), Some("Roboto"));
            assert_eq!(
                enabled_text.color.as_deref().copied(),
                Some(Color::new(0xdd000000))
            );

            let disabled_text = item_text_style(tester, &disabled_popup_item_key);
            assert_eq!(
                disabled_text.color.as_deref().copied(),
                Some(theme.disabled_color)
            );

            let top_left_button = tester
                .get_rect(
                    tester.render_box_of(tester.element(find::by_type::<PopupMenuButton<()>>())),
                )
                .top_left();
            let top_left_menu = tester.get_rect(tester.render_box_of(button)).top_left();
            assert_eq!(top_left_menu, top_left_button);

            let (over, down) = cursors_over(
                tester,
                &disabled_popup_item_key,
                Some(&enabled_popup_item_key),
            );
            assert!(*over == *cursor(SystemMouseCursors::BASIC));
            assert!(*down.expect("pressed") == *cursor(SystemMouseCursors::BASIC));

            // Check popup menu padding.
            assert_eq!(
                menu_padding(tester),
                Some(EdgeInsetsGeometry::symmetric(8.0, 0.0))
            );
        });
    }

    #[test]
    fn material2_popup_menu_uses_values_from_popup_menu_theme_data() {
        test_widgets(|tester| {
            let popup_menu_theme = popup_menu_theme_m2();
            let popup_button_key = GlobalKey::new();
            let enabled_popup_item_key = GlobalKey::new();
            let disabled_popup_item_key = GlobalKey::new();
            let (enabled_key, disabled_key) = (
                enabled_popup_item_key.clone(),
                disabled_popup_item_key.clone(),
            );
            let theme = popup_menu_theme.clone();
            button_column(
                tester,
                move |app| {
                    ThemeData::builder()
                        .popup_menu_theme(theme)
                        .use_material3(false)
                        .build(app)
                },
                false,
                PopupMenuButton::new(move |_app, _context| {
                    vec![
                        keyed_item(&disabled_key, "disabled", false),
                        PopupMenuEntryRef::new(PopupMenuDivider::new()),
                        PopupMenuEntryRef::new(
                            PopupMenuItem::new(Some(Text::new("enabled")))
                                .key(enabled_key.clone().into())
                                .on_tap(waft_foundation::Listener::new(|_app| {})),
                        ),
                    ]
                })
                .padding(EdgeInsetsGeometry::ZERO)
                .key(popup_button_key.clone().into()),
            );

            let at = tester.get_center(&popup_button_key);
            tester.tap_at(at, 1);
            tester.pump_and_settle();

            let button = menu_material(tester);
            tester.widget(button, |button: &Material| {
                assert_eq!(button.color, popup_menu_theme.color);
                assert_eq!(button.shape, popup_menu_theme.shape);
                assert_eq!(Some(button.elevation), popup_menu_theme.elevation);
            });

            assert_eq!(
                Some(item_text_style(tester, &enabled_popup_item_key)),
                popup_menu_theme.text_style
            );

            let mouse_cursor = popup_menu_theme.mouse_cursor.clone().expect("set");
            let (over, down) = cursors_over(
                tester,
                &disabled_popup_item_key,
                Some(&enabled_popup_item_key),
            );
            assert!(*over == *mouse_cursor.resolve(&disabled()).expect("a cursor"));
            assert!(
                *down.expect("pressed") == *mouse_cursor.resolve(&enabled()).expect("a cursor")
            );
        });
    }

    #[test]
    fn material2_popup_menu_widget_properties_take_priority_over_theme() {
        test_widgets(|tester| {
            let popup_menu_theme = popup_menu_theme_m2();
            let popup_button_key = GlobalKey::new();
            let popup_item_key = GlobalKey::new();

            let color = Colors::PURPLE.primary();
            let surface_tint_color = Colors::AMBER.primary();
            let shadow_color = Colors::GREEN.primary();
            let shape = RoundedRectangleBorder::new(
                BorderSide::NONE,
                BorderRadiusGeometry::all(Radius::circular(9.0)),
            );
            let elevation = 7.0;
            let text_style = TextStyle::new()
                .color(Color::new(0xffffffef))
                .font_size(19.0);

            let item_key = popup_item_key.clone();
            let style = text_style.clone();
            // Dart's case builds the default theme, Material 3, with the Material 2 popup menu
            // theme.
            button_column(
                tester,
                move |app| {
                    ThemeData::builder()
                        .popup_menu_theme(popup_menu_theme)
                        .build(app)
                },
                false,
                PopupMenuButton::new(move |_app, _context| {
                    vec![PopupMenuEntryRef::new(
                        PopupMenuItem::new(Some(Text::new("Example")))
                            .key(item_key.clone().into())
                            .label_text_style(<dyn WidgetStateProperty<Option<TextStyle>>>::all(
                                Some(style.clone()),
                            ))
                            .mouse_cursor(cursor(SystemMouseCursors::FORBIDDEN)),
                    )]
                })
                .key(popup_button_key.clone().into())
                .elevation(elevation)
                .shadow_color(shadow_color)
                .surface_tint_color(surface_tint_color)
                .color(color)
                .shape(shape),
            );

            let at = tester.get_center(&popup_button_key);
            tester.tap_at(at, 1);
            tester.pump_and_settle();

            let button = menu_material(tester);
            tester.widget(button, |button: &Material| {
                assert_eq!(button.color, Some(color));
                let shape: ShapeBorderRef = ShapeBorderRef::new(shape);
                assert_eq!(button.shape, Some(shape));
                assert_eq!(button.elevation, elevation);
                assert_eq!(button.shadow_color, Some(shadow_color));
                assert_eq!(button.surface_tint_color, Some(surface_tint_color));
            });

            assert_eq!(item_text_style(tester, &popup_item_key), text_style);

            let (over, _) = cursors_over(tester, &popup_item_key, None);
            assert!(*over == *cursor(SystemMouseCursors::FORBIDDEN));
        });
    }

    #[test]
    fn of_returns_the_closest_popup_menu_theme_over_theme_data() {
        test_widgets(|tester| {
            let widget_theme = PopupMenuThemeData::new().elevation(7.0);
            let expected = widget_theme.clone();
            let seen = probe(
                tester,
                move |app, child| {
                    let data = ThemeData::builder()
                        .popup_menu_theme(PopupMenuThemeData::new().elevation(3.0))
                        .build(app);
                    Theme::new(data, PopupMenuTheme::new(widget_theme.clone(), child)).into_widget()
                },
                PopupMenuTheme::of,
            );
            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn of_returns_the_theme_data_popup_menu_theme_without_an_ancestor() {
        test_widgets(|tester| {
            let expected = PopupMenuThemeData::new().elevation(3.0);
            let theme_data_theme = expected.clone();
            let seen = probe(
                tester,
                move |app, child| {
                    let data = ThemeData::builder()
                        .popup_menu_theme(theme_data_theme.clone())
                        .build(app);
                    Theme::new(data, child).into_widget()
                },
                PopupMenuTheme::of,
            );
            assert_eq!(seen, expected);
        });
    }

    #[test]
    fn popup_menu_theme_data_lerps_each_field() {
        let a = PopupMenuThemeData::new()
            .color(Colors::BLACK)
            .elevation(2.0)
            .icon_size(10.0)
            .enable_feedback(true)
            .position(PopupMenuPosition::Over);
        let b = PopupMenuThemeData::new()
            .color(Colors::WHITE)
            .elevation(6.0)
            .icon_size(20.0)
            .enable_feedback(false)
            .position(PopupMenuPosition::Under);
        let halfway = PopupMenuThemeData::lerp(Some(&a), Some(&b), 0.5).expect("both sides");
        assert_eq!(
            halfway.color,
            Color::lerp(Some(Colors::BLACK), Some(Colors::WHITE), 0.5)
        );
        assert_eq!(halfway.elevation, Some(4.0));
        assert_eq!(halfway.icon_size, Some(15.0));
        assert_eq!(halfway.enable_feedback, Some(false));
        assert_eq!(halfway.position, Some(PopupMenuPosition::Under));
        let early = PopupMenuThemeData::lerp(Some(&a), Some(&b), 0.25).expect("both sides");
        assert_eq!(early.enable_feedback, Some(true));
        assert_eq!(early.position, Some(PopupMenuPosition::Over));
    }

    #[test]
    fn update_should_notify_only_when_the_data_differs() {
        let theme = |elevation: f64| {
            PopupMenuTheme::new(
                PopupMenuThemeData::new().elevation(elevation),
                waft_widgets::SizedBox::shrink(),
            )
        };
        let old = theme(1.0);
        assert!(!theme(1.0).update_should_notify(&old));
        assert!(theme(2.0).update_should_notify(&old));
    }
}
