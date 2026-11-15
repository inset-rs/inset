//! Flutter counterpart: material_ui's `checkbox_list_tile.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::Color;
use waft_foundation::{App, Ref, ValueChanged};
use waft_gestures::GestureTapCallback;
use waft_painting::{AnyBorderSide, EdgeInsetsGeometry, OutlinedBorderRef, ShapeBorderRef};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, ExcludeFocus, FocusNodeRef, IntoWidget, KeyRef, MergeSemantics, StatelessWidget,
    Transform, WidgetRef, WidgetState, WidgetStatePropertyRef, WidgetStates,
    WidgetStatesController,
};

use crate::checkbox::Checkbox;
use crate::checkbox_theme::CheckboxTheme;
use crate::list_tile::{ListTile, ListTileControlAffinity, ListTileTitleAlignment};
use crate::list_tile_theme::ListTileTheme;
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};

/// Dart's `_CheckboxType`.
#[derive(Clone, Copy, Debug)]
enum CheckboxType {
    Material,
    Adaptive,
}

/// A [`ListTile`] with a [`Checkbox`]. In other words, a checkbox with a label.
///
/// The entire list tile is interactive: tapping anywhere in the tile toggles
/// the checkbox.
///
/// The [`value`](Self::value), [`on_changed`](Self::on_changed),
/// [`active_color`](Self::active_color) and [`check_color`](Self::check_color) properties of
/// this widget are identical to the similarly-named properties on the [`Checkbox`] widget.
///
/// The [`title`](Self::title), [`subtitle`](Self::subtitle),
/// [`is_three_line`](Self::is_three_line), [`dense`](Self::dense), and
/// [`content_padding`](Self::content_padding) properties are like those of the same name on
/// [`ListTile`].
///
/// The [`selected`](Self::selected) property on this widget is similar to the
/// [`ListTile::selected`] property. This tile's [`active_color`](Self::active_color) is used
/// for the selected item's text color, or the theme's `CheckboxThemeData::overlay_color` if
/// [`active_color`](Self::active_color) is null.
///
/// This widget does not coordinate the [`selected`](Self::selected) state and the
/// [`value`](Self::value) state; to have the list tile appear selected when the checkbox is
/// checked, pass the same value to both.
///
/// The checkbox is shown on the right by default in left-to-right languages
/// (i.e. the trailing edge). This can be changed using
/// [`control_affinity`](Self::control_affinity). The [`secondary`](Self::secondary) widget is
/// placed on the opposite side. This maps to the [`ListTile::leading`] and
/// [`ListTile::trailing`] properties of [`ListTile`].
///
/// This widget requires a `Material` widget ancestor in the tree to paint
/// itself on, which is typically provided by the app's `Scaffold`.
/// The [`tile_color`](Self::tile_color), and [`selected_tile_color`](Self::selected_tile_color)
/// are not painted by the [`CheckboxListTile`] itself but by the `Material` widget ancestor.
/// In this case, one can wrap a `Material` widget around the [`CheckboxListTile`].
///
/// ## Performance considerations when wrapping [`CheckboxListTile`] with `Material`
///
/// Wrapping a large number of [`CheckboxListTile`]s individually with `Material`s
/// is expensive. Consider only wrapping the [`CheckboxListTile`]s that require it
/// or include a common `Material` ancestor where possible.
///
/// To show the [`CheckboxListTile`] as disabled, pass null as the
/// [`on_changed`](Self::on_changed) callback.
///
/// ## CheckboxListTile isn't exactly what I want
///
/// If the way CheckboxListTile pads and positions its elements isn't quite
/// what you're looking for, you can create custom labeled checkbox widgets by
/// combining [`Checkbox`] with other widgets, such as `Text`, `Padding` and
/// `InkWell`.
///
/// See also:
///
///  * [`ListTileTheme`], which can be used to affect the style of list tiles,
///    including checkbox list tiles.
///  * `RadioListTile`, a similar widget for radio buttons.
///  * `SwitchListTile`, a similar widget for switches.
///  * [`ListTile`] and [`Checkbox`], the widgets from which this widget is made.
pub struct CheckboxListTile {
    pub key: Option<KeyRef>,
    /// Whether this checkbox is checked.
    pub value: Option<bool>,
    /// Called when the value of the checkbox should change.
    ///
    /// The checkbox passes the new value to the callback but does not actually
    /// change state until the parent widget rebuilds the checkbox tile with the
    /// new value.
    ///
    /// If null, the checkbox will be displayed as disabled.
    pub on_changed: Option<ValueChanged<Option<bool>>>,
    /// The cursor for a mouse pointer when it enters or is hovering over the
    /// widget.
    ///
    /// If [`mouse_cursor`](Self::mouse_cursor) is a `WidgetStateMouseCursor`,
    /// `WidgetStateProperty::resolve` is used for the following [`WidgetState`]s:
    ///
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then the value of `CheckboxThemeData::mouse_cursor` is used. If
    /// that is also null, then `WidgetStateMouseCursor::clickable` is used.
    pub mouse_cursor: Option<MouseCursorRef>,
    /// The color to use when this checkbox is checked.
    ///
    /// Defaults to `ColorScheme::secondary` of the current [`Theme`].
    pub active_color: Option<Color>,
    /// The color that fills the checkbox.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then the value of [`active_color`](Self::active_color) is used in the selected
    /// state. If that is also null, the value of `CheckboxThemeData::fill_color`
    /// is used. If that is also null, then the default value is used.
    pub fill_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// The color to use for the check icon when this checkbox is checked.
    ///
    /// Defaults to Color(0xFFFFFFFF).
    pub check_color: Option<Color>,
    /// The color for the checkbox's `Material` when a pointer is hovering over it.
    pub hover_color: Option<Color>,
    /// The color for the checkbox's `Material`.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Pressed`].
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///
    /// If null, then the value of [`active_color`](Self::active_color) with alpha
    /// `K_RADIAL_REACTION_ALPHA` and [`hover_color`](Self::hover_color) is used in the pressed
    /// and hovered state. If that is also null, the value of
    /// `CheckboxThemeData::overlay_color` is used. If that is also null, then the default
    /// value is used in the pressed and hovered state.
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// The splash radius of the circular `Material` ink response.
    ///
    /// If null, then the value of `CheckboxThemeData::splash_radius` is used. If
    /// that is also null, then `K_RADIAL_REACTION_RADIUS` is used.
    pub splash_radius: Option<f64>,
    /// Configures the minimum size of the tap target.
    ///
    /// Defaults to [`MaterialTapTargetSize::ShrinkWrap`].
    pub material_tap_target_size: Option<MaterialTapTargetSize>,
    /// Defines how compact the list tile's layout will be.
    pub visual_density: Option<VisualDensity>,
    /// An optional focus node to use as the focus node for this widget.
    pub focus_node: Option<FocusNodeRef>,
    /// Controls the interactive states of the backing [`ListTile`].
    pub states_controller: Option<Ref<WidgetStatesController>>,
    /// True if this widget will be selected as the initial focus when no other
    /// node in its scope is currently focused.
    pub autofocus: bool,
    /// Defines the tile's `InkWell::custom_border` and `Ink::decoration` shape.
    pub shape: Option<ShapeBorderRef>,
    /// The color and width of the checkbox's border.
    ///
    /// The given value is passed directly to [`Checkbox::side`].
    ///
    /// If this property is null, then `CheckboxThemeData::side` of
    /// `ThemeData::checkbox_theme` is used. If that is also null, then the side
    /// will be width 2.
    pub side: Option<AnyBorderSide>,
    /// True if this checkbox wants to show an error state.
    ///
    /// Defaults to false.
    pub is_error: bool,
    /// Defines the background color of `ListTile` when [`selected`](Self::selected) is false.
    pub tile_color: Option<Color>,
    /// The primary content of the list tile.
    ///
    /// Typically a `Text` widget.
    pub title: Option<WidgetRef>,
    /// Additional content displayed below the title.
    ///
    /// Typically a `Text` widget.
    pub subtitle: Option<WidgetRef>,
    /// A widget to display on the opposite side of the tile from the checkbox.
    ///
    /// Typically an `Icon` widget.
    pub secondary: Option<WidgetRef>,
    /// Whether this list tile is intended to display three lines of text.
    ///
    /// If null then the ambient `ListTileThemeData::is_three_line` is used.
    /// If that is also null, the default value is `false`.
    pub is_three_line: Option<bool>,
    /// Whether this list tile is part of a vertically dense list.
    ///
    /// If this property is null then its value is based on `ListTileThemeData::dense`.
    pub dense: Option<bool>,
    /// Whether to render icons and text in the [`active_color`](Self::active_color).
    ///
    /// No effort is made to automatically coordinate the [`selected`](Self::selected) state
    /// and the [`value`](Self::value) state. To have the list tile appear selected when the
    /// checkbox is checked, pass the same value to both.
    ///
    /// Normally, this property is left to its default value, false.
    pub selected: bool,
    /// Where to place the control relative to the text.
    pub control_affinity: Option<ListTileControlAffinity>,
    /// Defines insets surrounding the tile's contents.
    ///
    /// This value will surround the [`Checkbox`], [`title`](Self::title),
    /// [`subtitle`](Self::subtitle), and [`secondary`](Self::secondary) widgets in
    /// [`CheckboxListTile`].
    ///
    /// When the value is null, the [`content_padding`](Self::content_padding) is
    /// `EdgeInsets.symmetric(horizontal: 16.0)`.
    pub content_padding: Option<EdgeInsetsGeometry>,
    /// If true the checkbox's [`value`](Self::value) can be true, false, or null.
    ///
    /// Checkbox displays a dash when its value is null.
    ///
    /// When a tri-state checkbox ([`tristate`](Self::tristate) is true) is tapped, its
    /// [`on_changed`](Self::on_changed) callback will be applied to true if the current value
    /// is false, to null if value is true, and to false if value is null (i.e. it cycles
    /// through false => true => null => false when tapped).
    ///
    /// If tristate is false (the default), [`value`](Self::value) must not be null.
    pub tristate: bool,
    /// The shape of the checkbox's `Material`.
    ///
    /// If this property is null then `CheckboxThemeData::shape` of `ThemeData::checkbox_theme`
    /// is used. If that's null then the shape will be a `RoundedRectangleBorder`
    /// with a circular corner radius of 1.0.
    pub checkbox_shape: Option<OutlinedBorderRef>,
    /// If non-null, defines the background color when [`CheckboxListTile::selected`] is true.
    pub selected_tile_color: Option<Color>,
    /// Handler called when the focus changes.
    ///
    /// Called with true if this widget's node gains focus, and false if it loses
    /// focus.
    pub on_focus_change: Option<ValueChanged<bool>>,
    /// Whether detected gestures should provide acoustic and/or haptic feedback.
    ///
    /// See also:
    ///
    ///  * `Feedback` for providing platform-specific feedback to certain actions.
    pub enable_feedback: Option<bool>,
    /// The horizontal gap between the titles and the leading/trailing widgets.
    pub horizontal_title_gap: Option<f64>,
    /// The minimum padding on the top and bottom of the title and subtitle widgets.
    pub min_vertical_padding: Option<f64>,
    /// The minimum width allocated for the `ListTile::leading` widget.
    pub min_leading_width: Option<f64>,
    /// The minimum height allocated for the `ListTile` widget.
    pub min_tile_height: Option<f64>,
    /// Whether the CheckboxListTile is interactive.
    ///
    /// If false, this list tile is styled with the disabled color from the
    /// current [`Theme`] and the [`ListTile::on_tap`] callback is
    /// inoperative.
    pub enabled: Option<bool>,
    /// Defines how [`ListTile::leading`] and [`ListTile::trailing`] are
    /// vertically aligned relative to the [`ListTile`]'s titles
    /// ([`ListTile::title`] and [`ListTile::subtitle`]).
    ///
    /// If this property is null then `ListTileThemeData::title_alignment`
    /// is used. If that is also null then [`ListTileTitleAlignment::ThreeLine`]
    /// is used.
    ///
    /// See also:
    ///
    /// * [`ListTileTheme::of`], which returns the nearest [`ListTileTheme`]'s
    ///   `ListTileThemeData`.
    pub title_alignment: Option<ListTileTitleAlignment>,
    /// Whether to add button:true to the semantics if onTap is provided.
    /// This is a temporary flag to help changing the behavior of ListTile onTap semantics.
    pub internal_add_semantic_for_on_tap: bool,
    /// Controls the scaling factor applied to the [`Checkbox`] within the [`CheckboxListTile`].
    ///
    /// Defaults to 1.0.
    pub checkbox_scale_factor: f64,
    /// The semantic label for the checkbox that will be announced by screen readers.
    ///
    /// This is announced by assistive technologies (e.g TalkBack/VoiceOver).
    ///
    /// This label does not show in the UI.
    pub checkbox_semantic_label: Option<String>,
    checkbox_type: CheckboxType,
}

/// The setters of `CheckboxListTile`'s optional arguments that hold a value.
macro_rules! checkbox_list_tile_setters {
    ($($field:ident: $ty:ty => $name:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `CheckboxListTile(", $name, ":)`.")]
            pub fn $field(mut self, $field: impl Into<$ty>) -> CheckboxListTile {
                self.$field = Some($field.into());
                self
            }
        )*
    };
}

/// The setters of `CheckboxListTile`'s optional arguments that are widgets.
macro_rules! checkbox_list_tile_widget_setters {
    ($($field:ident => $name:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `CheckboxListTile(", $name, ":)`.")]
            pub fn $field<K>(mut self, $field: impl IntoWidget<K>) -> CheckboxListTile {
                self.$field = Some($field.into_widget());
                self
            }
        )*
    };
}

impl CheckboxListTile {
    /// Creates a combination of a list tile and a checkbox.
    ///
    /// The checkbox tile itself does not maintain any state. Instead, when the
    /// state of the checkbox changes, the widget calls the [`on_changed`](Self::on_changed)
    /// callback. Most widgets that use a checkbox will listen for the
    /// [`on_changed`](Self::on_changed) callback and rebuild the checkbox tile with a new
    /// [`value`](Self::value) to update the visual appearance of the checkbox.
    ///
    /// The following arguments are required:
    ///
    /// * `value`, which determines whether the checkbox is checked. The `value`
    ///   can only be null if [`tristate`](Self::tristate) is true.
    /// * `on_changed`, which is called when the value of the checkbox should
    ///   change. It can be set to null to disable the checkbox.
    pub fn new(
        value: Option<bool>,
        on_changed: Option<ValueChanged<Option<bool>>>,
    ) -> CheckboxListTile {
        CheckboxListTile::with_type(value, on_changed, CheckboxType::Material)
    }

    /// Creates a combination of a list tile and a platform adaptive checkbox.
    ///
    /// The checkbox uses [`Checkbox::adaptive`] to show a `CupertinoCheckbox` for
    /// iOS platforms, or [`Checkbox`] for all others.
    ///
    /// All other properties are the same as [`CheckboxListTile`].
    pub fn adaptive(
        value: Option<bool>,
        on_changed: Option<ValueChanged<Option<bool>>>,
    ) -> CheckboxListTile {
        CheckboxListTile::with_type(value, on_changed, CheckboxType::Adaptive)
    }

    fn with_type(
        value: Option<bool>,
        on_changed: Option<ValueChanged<Option<bool>>>,
        checkbox_type: CheckboxType,
    ) -> CheckboxListTile {
        CheckboxListTile {
            key: None,
            value,
            on_changed,
            mouse_cursor: None,
            active_color: None,
            fill_color: None,
            check_color: None,
            hover_color: None,
            overlay_color: None,
            splash_radius: None,
            material_tap_target_size: None,
            visual_density: None,
            focus_node: None,
            states_controller: None,
            autofocus: false,
            shape: None,
            side: None,
            is_error: false,
            tile_color: None,
            title: None,
            subtitle: None,
            secondary: None,
            is_three_line: None,
            dense: None,
            selected: false,
            control_affinity: None,
            content_padding: None,
            tristate: false,
            checkbox_shape: None,
            selected_tile_color: None,
            on_focus_change: None,
            enable_feedback: None,
            horizontal_title_gap: None,
            min_vertical_padding: None,
            min_leading_width: None,
            min_tile_height: None,
            enabled: None,
            title_alignment: None,
            internal_add_semantic_for_on_tap: false,
            checkbox_scale_factor: 1.0,
            checkbox_semantic_label: None,
            checkbox_type,
        }
    }

    /// Dart `CheckboxListTile(key:)`.
    pub fn key(mut self, key: KeyRef) -> CheckboxListTile {
        self.key = Some(key);
        self
    }

    checkbox_list_tile_setters!(
        mouse_cursor: MouseCursorRef => "mouseCursor",
        active_color: Color => "activeColor",
        fill_color: WidgetStatePropertyRef<Option<Color>> => "fillColor",
        check_color: Color => "checkColor",
        hover_color: Color => "hoverColor",
        overlay_color: WidgetStatePropertyRef<Option<Color>> => "overlayColor",
        splash_radius: f64 => "splashRadius",
        material_tap_target_size: MaterialTapTargetSize => "materialTapTargetSize",
        visual_density: VisualDensity => "visualDensity",
        focus_node: FocusNodeRef => "focusNode",
        states_controller: Ref<WidgetStatesController> => "statesController",
        shape: ShapeBorderRef => "shape",
        side: AnyBorderSide => "side",
        tile_color: Color => "tileColor",
        is_three_line: bool => "isThreeLine",
        dense: bool => "dense",
        control_affinity: ListTileControlAffinity => "controlAffinity",
        content_padding: EdgeInsetsGeometry => "contentPadding",
        checkbox_shape: OutlinedBorderRef => "checkboxShape",
        selected_tile_color: Color => "selectedTileColor",
        on_focus_change: ValueChanged<bool> => "onFocusChange",
        enable_feedback: bool => "enableFeedback",
        horizontal_title_gap: f64 => "horizontalTitleGap",
        min_vertical_padding: f64 => "minVerticalPadding",
        min_leading_width: f64 => "minLeadingWidth",
        min_tile_height: f64 => "minTileHeight",
        enabled: bool => "enabled",
        title_alignment: ListTileTitleAlignment => "titleAlignment",
        checkbox_semantic_label: String => "checkboxSemanticLabel",
    );

    checkbox_list_tile_widget_setters!(
        title => "title",
        subtitle => "subtitle",
        secondary => "secondary",
    );

    /// Dart `CheckboxListTile(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> CheckboxListTile {
        self.autofocus = autofocus;
        self
    }

    /// Dart `CheckboxListTile(isError:)`.
    pub fn is_error(mut self, is_error: bool) -> CheckboxListTile {
        self.is_error = is_error;
        self
    }

    /// Dart `CheckboxListTile(selected:)`.
    pub fn selected(mut self, selected: bool) -> CheckboxListTile {
        self.selected = selected;
        self
    }

    /// Dart `CheckboxListTile(tristate:)`.
    pub fn tristate(mut self, tristate: bool) -> CheckboxListTile {
        self.tristate = tristate;
        self
    }

    /// Dart `CheckboxListTile(internalAddSemanticForOnTap:)`.
    pub fn internal_add_semantic_for_on_tap(
        mut self,
        internal_add_semantic_for_on_tap: bool,
    ) -> CheckboxListTile {
        self.internal_add_semantic_for_on_tap = internal_add_semantic_for_on_tap;
        self
    }

    /// Dart `CheckboxListTile(checkboxScaleFactor:)`.
    pub fn checkbox_scale_factor(mut self, checkbox_scale_factor: f64) -> CheckboxListTile {
        self.checkbox_scale_factor = checkbox_scale_factor;
        self
    }

    /// Dart's `_handleValueChange`, as the tap callback that reads this tile's value.
    fn handle_value_change(&self) -> GestureTapCallback {
        debug_assert!(self.on_changed.is_some());
        let on_changed = Rc::clone(self.on_changed.as_ref().expect("an enabled tile"));
        let value = self.value;
        let tristate = self.tristate;
        GestureTapCallback::new(move |app: &mut App| match value {
            Some(false) => on_changed(app, Some(true)),
            Some(true) => on_changed(app, if tristate { None } else { Some(false) }),
            None => on_changed(app, Some(false)),
        })
    }

    /// The `Checkbox` or `Checkbox.adaptive` Dart's `build` puts in the control slot.
    fn build_checkbox(&self) -> Checkbox {
        let on_changed = if self.enabled.unwrap_or(true) {
            self.on_changed.clone()
        } else {
            None
        };
        let mut checkbox = match self.checkbox_type {
            CheckboxType::Material => Checkbox::new(self.value, on_changed),
            CheckboxType::Adaptive => Checkbox::adaptive(self.value, on_changed),
        }
        .material_tap_target_size(
            self.material_tap_target_size
                .unwrap_or(MaterialTapTargetSize::ShrinkWrap),
        )
        .autofocus(self.autofocus)
        .tristate(self.tristate)
        .is_error(self.is_error);
        checkbox.mouse_cursor = self.mouse_cursor.clone();
        checkbox.active_color = self.active_color;
        checkbox.fill_color = self.fill_color.clone();
        checkbox.check_color = self.check_color;
        checkbox.hover_color = self.hover_color;
        checkbox.overlay_color = self.overlay_color.clone();
        checkbox.splash_radius = self.splash_radius;
        checkbox.shape = self.checkbox_shape.clone();
        checkbox.side = self.side.clone();
        checkbox.semantic_label = self.checkbox_semantic_label.clone();
        checkbox
    }
}

impl Debug for CheckboxListTile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CheckboxListTile")
            .field("value", &self.value)
            .field("tristate", &self.tristate)
            .field(
                "on_changed",
                &if self.on_changed.is_some() {
                    "enabled"
                } else {
                    "disabled"
                },
            )
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for CheckboxListTile {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(self.tristate || self.value.is_some());
        debug_assert!(self.is_three_line != Some(true) || self.subtitle.is_some());
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let mut control = ExcludeFocus::new(self.build_checkbox()).into_widget();
        if self.checkbox_scale_factor != 1.0 {
            control = Transform::scale(Some(self.checkbox_scale_factor), None, None)
                .child(control)
                .into_widget();
        }

        let list_tile_theme = ListTileTheme::of(app, context);
        let effective_control_affinity = self
            .control_affinity
            .or(list_tile_theme.control_affinity)
            .unwrap_or(ListTileControlAffinity::Platform);
        let (leading, trailing) = match effective_control_affinity {
            ListTileControlAffinity::Leading => (Some(control), self.secondary.clone()),
            ListTileControlAffinity::Trailing | ListTileControlAffinity::Platform => {
                (self.secondary.clone(), Some(control))
            }
        };

        let theme = Theme::of(app, context);
        let checkbox_theme = CheckboxTheme::of(app, context);
        let mut states = WidgetStates::new();
        if self.selected {
            states.insert(WidgetState::Selected);
        }
        let effective_active_color = self
            .active_color
            .or_else(|| {
                checkbox_theme
                    .fill_color
                    .as_ref()
                    .and_then(|fill_color| fill_color.resolve(&states))
            })
            .unwrap_or(theme.color_scheme.secondary);
        let mut list_tile = ListTile::new()
            .selected_color(effective_active_color)
            .enabled(self.enabled.unwrap_or(self.on_changed.is_some()))
            .selected(self.selected)
            .autofocus(self.autofocus);
        list_tile.leading = leading;
        list_tile.title = self.title.clone();
        list_tile.subtitle = self.subtitle.clone();
        list_tile.trailing = trailing;
        list_tile.is_three_line = self.is_three_line;
        list_tile.dense = self.dense;
        list_tile.on_tap = if self.on_changed.is_some() {
            Some(self.handle_value_change())
        } else {
            None
        };
        list_tile.content_padding = self.content_padding;
        list_tile.shape = self.shape.clone();
        list_tile.selected_tile_color = self.selected_tile_color;
        list_tile.tile_color = self.tile_color;
        list_tile.visual_density = self.visual_density;
        list_tile.focus_node = self.focus_node.clone();
        list_tile.states_controller = self.states_controller.clone();
        list_tile.on_focus_change = self.on_focus_change.clone();
        list_tile.enable_feedback = self.enable_feedback;
        list_tile.horizontal_title_gap = self.horizontal_title_gap;
        list_tile.min_vertical_padding = self.min_vertical_padding;
        list_tile.min_leading_width = self.min_leading_width;
        list_tile.min_tile_height = self.min_tile_height;
        list_tile.title_alignment = self.title_alignment;
        list_tile.internal_add_semantic_for_on_tap = self.internal_add_semantic_for_on_tap;
        MergeSemantics::new().child(list_tile).into_widget()
    }
}

#[cfg(test)]
#[allow(deprecated)]
pub(crate) mod tests {
    use std::cell::{Cell, RefCell};
    use std::time::Duration;
    use waft_painting::ShapeBorder;
    use waft_widgets::AnyFocusNode;

    use waft_embedder::TextDirection;
    use waft_embedder::{
        Matrix4, Offset, PointerChange, RRect, Radius, Rect, Size, TargetPlatform,
    };
    use waft_foundation::K_IS_WEB;
    use waft_painting::{
        AlignmentGeometry, BorderRadiusGeometry, BorderSide, EdgeInsets, RoundedRectangleBorder,
    };
    use waft_services::SystemMouseCursors;
    use waft_widgets::{
        AnyElement, Center, Column, Container, Directionality, FocusHighlightStrategy, FocusNode,
        GlobalKey, ListView, MouseRegion, SizedBox, StatefulBuilder, Text, UnfocusDisposition,
        WidgetStateMouseCursor, WidgetStateProperty,
    };

    use super::*;
    use crate::checkbox::tests::{
        Paints, assert_paints, checkbox_center, checkbox_size, hover, ignore, press,
    };
    use crate::checkbox_theme::CheckboxThemeData;
    use crate::colors::Colors;
    use crate::ink_well::InkWell;
    use crate::list_tile::tests::{THIRTEEN_LINES, bare, m2, m3, text_rect};
    use crate::list_tile_theme::ListTileThemeData;
    use crate::material::Material;
    use crate::scaffold::Scaffold;
    use crate::test_support::{AhemText, WidgetTester, with_main_thread_stack};
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{pump_app, tester};

    /// Dart's `wrap(child:)`: `MediaQuery(Directionality(ltr, Material(child)))`, under the
    /// default theme.
    pub(crate) fn pump_wrapped(
        tester: &mut WidgetTester,
        child: impl FnOnce(&mut App) -> WidgetRef,
    ) {
        let root = {
            let mut app = tester.cell.borrow_mut();
            let theme = m3(&app);
            bare(theme, Material::new().child(child(&mut app)))
        };
        tester.pump_widget(root);
    }

    /// `MaterialApp(theme: theme, home: Material(child: child))`.
    pub(crate) fn pump_in_material(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        child: impl FnOnce(&mut App) -> WidgetRef,
    ) {
        pump_app(tester, theme, |app| {
            Material::new().child(child(app)).into_widget()
        });
    }

    /// The `index`th element whose widget is a `W`: Dart's `find.byType(W).at(index)`.
    pub(crate) fn nth<W: 'static>(tester: &WidgetTester, index: usize) -> AnyElement {
        tester.find_by_type::<W>()[index]
    }

    /// Dart's `tester.getRect(find.byType(W).at(index))`.
    pub(crate) fn rect_of<W: 'static>(tester: &WidgetTester, index: usize) -> Rect {
        tester.global_rect(tester.render_box_of(nth::<W>(tester, index)))
    }

    /// Dart's `tester.getRect(find.text(text))`, over `Text`s.
    pub(crate) fn text_center(tester: &WidgetTester, text: &str) -> Offset {
        let found = tester.find_text(text);
        tester.global_rect(tester.render_box_of(found[0])).center()
    }

    /// Dart's `Focus.of(key.currentContext!)`.
    pub(crate) fn focus_of(tester: &WidgetTester, key: &GlobalKey) -> AnyFocusNode {
        let mut app = tester.cell.borrow_mut();
        let context = key.current_context(&app).expect("mounted");
        waft_widgets::Focus::of(&mut app, context, false, true)
    }

    pub(crate) fn has_primary_focus(tester: &WidgetTester, node: AnyFocusNode) -> bool {
        node.has_primary_focus(&tester.cell.borrow())
    }

    pub(crate) fn new_focus_node(tester: &WidgetTester) -> FocusNodeRef {
        FocusNode::new(&mut tester.cell.borrow_mut()).into_erased::<waft_widgets::AnyFocusNode>()
    }

    /// A focus-change callback that records the last value it was handed.
    pub(crate) fn record_focus(got_focus: &Rc<Cell<bool>>) -> ValueChanged<bool> {
        let got_focus = Rc::clone(got_focus);
        Rc::new(move |_app, focused| got_focus.set(focused))
    }

    /// The `customBorder` of the tile's `InkWell`.
    pub(crate) fn ink_well_custom_border(tester: &WidgetTester) -> Option<ShapeBorderRef> {
        tester.widget(nth::<InkWell>(tester, 0), |ink_well: &InkWell| {
            ink_well.ink_response.custom_border.clone()
        })
    }

    /// The `enableFeedback` of the tile's `InkWell`: what reaches the feedback calls, the
    /// click system sound waiting with `SystemSound`.
    pub(crate) fn ink_well_enable_feedback(tester: &WidgetTester) -> bool {
        tester.widget(nth::<InkWell>(tester, 0), |ink_well: &InkWell| {
            ink_well.ink_response.enable_feedback
        })
    }

    /// Dart's `RoundedRectangleBorder(borderRadius: BorderRadius.horizontal(right:
    /// Radius.circular(100)))`.
    pub(crate) fn right_rounded_border() -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::horizontal(None, Some(Radius::circular(100.0)), None, None),
        )
    }

    /// The colours of the rects the material's ink features draw: Dart's
    /// `find.byType(Material), paints..rect(color:)`.
    pub(crate) fn ink_rect_colors(tester: &WidgetTester) -> Vec<u32> {
        crate::list_tile::tests::ink_rect_colors(tester)
    }

    /// Dart's `tester.getSize(find.byType(ListTile))`.
    pub(crate) fn list_tile_size(tester: &WidgetTester) -> Size {
        rect_of::<ListTile>(tester, 0).size()
    }

    /// One frame of an "isThreeLine" case: the `ThemeData.listTileTheme`'s, the ambient
    /// `ListTileTheme`'s and the tile's `isThreeLine`.
    pub(crate) type IsThreeLineFrame = (Option<bool>, Option<bool>, Option<bool>);

    /// The frames of the "isThreeLine" cases: the `ThemeData.listTileTheme`'s, the ambient
    /// `ListTileTheme`'s and the tile's `isThreeLine`, and whether the tiles lay out in three
    /// lines.
    pub(crate) const IS_THREE_LINE_FRAMES: [(IsThreeLineFrame, bool); 11] = [
        ((None, None, None), false),
        ((Some(true), None, None), true),
        ((Some(false), Some(true), None), true),
        ((Some(true), Some(false), None), false),
        ((None, None, Some(true)), true),
        ((None, Some(true), Some(false)), false),
        ((Some(true), None, Some(false)), false),
        ((Some(true), Some(true), Some(false)), false),
        ((None, Some(false), Some(true)), true),
        ((Some(false), None, Some(true)), true),
        ((Some(false), Some(false), Some(true)), true),
    ];

    /// One frame of an "isThreeLine" case on a fresh tester (Dart's `MaterialApp(key:
    /// UniqueKey())`): `tile(is_three_line, subtitle)` twice in a `ListView`, the first with
    /// thirteen lines of subtitle; answers the two tiles' rects and their controls' (`C`).
    pub(crate) fn is_three_line_rects<C: 'static, W: 'static>(
        platform: TargetPlatform,
        (theme_data_is_three_line, theme_is_three_line, is_three_line): IsThreeLineFrame,
        tile: fn(Option<bool>, &str) -> WidgetRef,
    ) -> [Rect; 4] {
        let mut tester = tester();
        pump_in_material(
            &mut tester,
            |app| {
                let mut theme = ThemeData::builder().platform(platform);
                if let Some(is_three_line) = theme_data_is_three_line {
                    theme = theme
                        .list_tile_theme(ListTileThemeData::new().is_three_line(is_three_line));
                }
                theme.build(app)
            },
            |_app| {
                let list = ListView::new().children([
                    tile(is_three_line, THIRTEEN_LINES),
                    tile(is_three_line, "A"),
                ]);
                let mut theme = ListTileTheme::new(list);
                if let Some(is_three_line) = theme_is_three_line {
                    theme = theme.data(ListTileThemeData::new().is_three_line(is_three_line));
                }
                theme.into_widget()
            },
        );
        [
            rect_of::<W>(&tester, 0),
            rect_of::<C>(&tester, 0),
            rect_of::<W>(&tester, 1),
            rect_of::<C>(&tester, 1),
        ]
    }

    /// The rects an "isThreeLine" case expects of two tiles 300 and 72 (two-line) or 88
    /// (three-line) high whose `control_size` control sits at `left`, and at `two_line_tops`
    /// when two-line.
    pub(crate) fn expected_three_line_rects(
        three_line: bool,
        left: f64,
        control_size: Size,
        two_line_tops: [f64; 2],
    ) -> [Rect; 4] {
        const HEIGHT: f64 = 300.0;
        let (second_height, tops) = if three_line {
            (88.0, [8.0, HEIGHT + 8.0])
        } else {
            (72.0, two_line_tops)
        };
        [
            Rect::from_ltwh(0.0, 0.0, 800.0, HEIGHT),
            Rect::from_ltwh(left, tops[0], control_size.width(), control_size.height()),
            Rect::from_ltwh(0.0, HEIGHT, 800.0, second_height),
            Rect::from_ltwh(left, tops[1], control_size.width(), control_size.height()),
        ]
    }

    /// One frame of the spacing cases on a fresh tester: `MaterialApp(theme:,
    /// home: Directionality(text_direction, Material(ListTileTheme(data:,
    /// Container(alignment: topLeft, child: tile)))))`.
    pub(crate) fn spacing_frame(
        theme: fn(&App) -> ThemeData,
        text_direction: TextDirection,
        data: ListTileThemeData,
        tile: WidgetRef,
    ) -> WidgetTester {
        let mut tester = tester();
        pump_app(&mut tester, theme, |_app| {
            Directionality::new(
                text_direction,
                Material::new().child(
                    ListTileTheme::new(
                        Container::new()
                            .alignment(AlignmentGeometry::TOP_LEFT)
                            .child(tile),
                    )
                    .data(data),
                ),
            )
            .into_widget()
        });
        tester
    }

    /// Where the `title` starts (LTR) or ends (RTL).
    pub(crate) fn title_edge(tester: &WidgetTester, text_direction: TextDirection) -> f64 {
        let title = text_rect(tester, "title");
        match text_direction {
            TextDirection::Ltr => title.left,
            TextDirection::Rtl => title.right,
        }
    }

    /// The frames of the gap and width cases: the theme's value and the tile's.
    pub(crate) const THEME_AND_WIDGET_FRAMES: [(Option<f64>, Option<f64>); 3] = [
        (None, Some(0.0)),
        (Some(0.0), None),
        (Some(10.0), Some(0.0)),
    ];

    /// The frames of the cases that set 60 or 80: the theme's value and the tile's.
    pub(crate) fn theme_and_widget_frames(value: f64) -> [(Option<f64>, Option<f64>); 3] {
        [
            (None, Some(value)),
            (Some(value), None),
            (Some(0.0), Some(value)),
        ]
    }

    /// The vertical offsets of the control `C` and of `secondary_key`'s top right from the
    /// tile's top: what the "titleAlignment" cases compare.
    pub(crate) fn title_alignment_offsets<C: 'static>(
        tester: &WidgetTester,
        secondary_key: &GlobalKey,
    ) -> (f64, f64) {
        let tile = rect_of::<ListTile>(tester, 0);
        let control = rect_of::<C>(tester, 0);
        let render = tester
            .render_object(secondary_key)
            .as_box()
            .expect("a render box");
        let secondary = tester.global_rect(render);
        (control.top - tile.top, secondary.top - tile.top)
    }

    /// "titleAlignment position with title widget" and "... with title and subtitle widgets"
    /// for a tile `make(title_alignment, secondary)` with the control `C` leading: the expected
    /// offsets of each alignment given the tile's and the control's heights.
    pub(crate) fn check_title_alignment<C: 'static>(
        make: impl Fn(Option<ListTileTitleAlignment>, WidgetRef, bool) -> WidgetRef,
        with_subtitle: bool,
    ) {
        const SECONDARY_HEIGHT: f64 = 24.0;
        // The default vertical padding for material 3 is 8.0.
        const MIN_VERTICAL_PADDING: f64 = 8.0;
        let secondary_key = GlobalKey::new();
        let mut tester = tester();
        let frame = |tester: &mut WidgetTester, title_alignment| {
            let secondary = SizedBox::new()
                .key(secondary_key.clone().into())
                .width(24.0)
                .height(SECONDARY_HEIGHT)
                .into_widget();
            let tile = make(title_alignment, secondary, with_subtitle);
            pump_in_material(tester, m3, |_app| Center::new().child(tile).into_widget());
            assert_eq!(
                tester.widget(nth::<ListTile>(tester, 0), |tile: &ListTile| {
                    tile.title_alignment
                }),
                title_alignment
            );
            title_alignment_offsets::<C>(tester, &secondary_key)
        };

        let offsets = frame(&mut tester, None);
        let tile_height = list_tile_size(&tester).height();
        let control_height = rect_of::<C>(&tester, 0).height();
        // Leading and trailing widgets are centered vertically in the tile.
        let center = (
            tile_height / 2.0 - control_height / 2.0,
            tile_height / 2.0 - SECONDARY_HEIGHT / 2.0,
        );
        assert_eq!(offsets, center);
        assert_eq!(
            frame(&mut tester, Some(ListTileTitleAlignment::ThreeLine)),
            center
        );
        let title_height = if with_subtitle {
            // Leading and trailing widgets are positioned 16.0 pixels below the top of the
            // title widget.
            (16.0, 16.0)
        } else {
            ((tile_height - control_height) / 2.0, center.1)
        };
        assert_eq!(
            frame(&mut tester, Some(ListTileTitleAlignment::TitleHeight)),
            title_height
        );
        // Leading and trailing widgets are placed minVerticalPadding below the top of the
        // title widget.
        assert_eq!(
            frame(&mut tester, Some(ListTileTitleAlignment::Top)),
            (MIN_VERTICAL_PADDING, MIN_VERTICAL_PADDING)
        );
        assert_eq!(
            frame(&mut tester, Some(ListTileTitleAlignment::Center)),
            center
        );
        // Leading and trailing widgets are placed minVerticalPadding above the bottom of the
        // subtitle widget.
        assert_eq!(
            frame(&mut tester, Some(ListTileTitleAlignment::Bottom)),
            (
                tile_height - MIN_VERTICAL_PADDING - control_height,
                tile_height - MIN_VERTICAL_PADDING - SECONDARY_HEIGHT
            )
        );
    }

    /// A tile whose value a `StatefulBuilder` keeps and whose `on_changed` sets.
    fn stateful_tile(
        value: &Rc<Cell<Option<bool>>>,
        configure: impl Fn(CheckboxListTile) -> CheckboxListTile + 'static,
    ) -> WidgetRef {
        let value = Rc::clone(value);
        StatefulBuilder::new(move |_app, _context, set_state| {
            let value_now = Rc::clone(&value);
            configure(CheckboxListTile::new(
                value.get(),
                Some(Rc::new(move |app: &mut App, new_value: Option<bool>| {
                    let value = Rc::clone(&value_now);
                    set_state(app, &mut |_app| value.set(new_value));
                })),
            ))
            .into_widget()
        })
        .into_widget()
    }

    fn fill_when_selected(color: Option<Color>) -> CheckboxThemeData {
        CheckboxThemeData::new().fill_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
            move |states| {
                if states.contains(&WidgetState::Selected) {
                    color
                } else {
                    None
                }
            },
        ))
    }

    #[test]
    fn checkbox_list_tile_control_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let log = Rc::new(RefCell::new(Vec::new()));
            let on_changed: ValueChanged<Option<bool>> = {
                let log = Rc::clone(&log);
                Rc::new(move |_app, value| log.borrow_mut().push(Some(value)))
            };
            pump_wrapped(&mut tester, |_app| {
                CheckboxListTile::new(Some(true), Some(on_changed))
                    .title(Text::new("Hello"))
                    .into_widget()
            });
            let hello = text_center(&tester, "Hello");
            tester.tap(hello, 1);
            log.borrow_mut().push(None);
            let checkbox = checkbox_center(&tester);
            tester.tap(checkbox, 2);
            assert_eq!(*log.borrow(), [Some(Some(false)), None, Some(Some(false))]);
        });
    }

    #[test]
    fn checkbox_list_tile_forwards_states_controller_to_list_tile() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let controller = WidgetStatesController::new(&mut tester.cell.borrow_mut(), None);
            pump_in_material(&mut tester, m3, |_app| {
                CheckboxListTile::new(Some(true), ignore())
                    .title(Text::new("Checkbox"))
                    .states_controller(controller.clone())
                    .into_widget()
            });
            let forwarded = tester.widget(nth::<ListTile>(&tester, 0), |tile: &ListTile| {
                tile.states_controller.clone()
            });
            assert_eq!(forwarded, Some(controller));
        });
    }

    fn check_color_case(theme: fn(&App) -> ThemeData, check_box_border_color: Color) {
        with_main_thread_stack(|| {
            let mut tester = tester();
            for check_color in [None, Some(Color::new(0xFF000000))] {
                pump_in_material(&mut tester, theme, |_app| {
                    let mut tile = CheckboxListTile::new(Some(true), ignore());
                    tile.check_color = check_color;
                    tile.into_widget()
                });
                tester.pump_and_settle();
                assert_paints(
                    &tester,
                    &[
                        Paints::Rrect(check_box_border_color),
                        Paints::Path(check_color.unwrap_or(Color::new(0xFFFFFFFF))),
                    ],
                );
            }
        });
    }

    #[test]
    fn material2_checkbox_list_tile_check_color_test() {
        check_color_case(m2, Color::new(0xff2196f3));
    }

    #[test]
    fn material3_checkbox_list_tile_check_color_test() {
        check_color_case(m3, Color::new(0xff6750a4));
    }

    #[test]
    fn checkbox_list_tile_active_color_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            for (active_color, expected) in [
                (None, Color::new(0xFF000000)),
                (Some(Color::new(0xFFFFFFFF)), Color::new(0xFFFFFFFF)),
            ] {
                pump_wrapped(&mut tester, |app| {
                    let theme = ThemeData::builder()
                        .checkbox_theme(fill_when_selected(Some(Color::new(0xFF000000))))
                        .build(app);
                    let mut tile = CheckboxListTile::new(Some(true), ignore());
                    tile.active_color = active_color;
                    crate::theme::Theme::new(theme, tile).into_widget()
                });
                tester.pump_and_settle();
                assert_paints(&tester, &[Paints::Rrect(expected)]);
            }
        });
    }

    #[test]
    fn checkbox_list_tile_can_autofocus_unless_disabled() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let child_key = GlobalKey::new();
            for (on_changed, focused) in [(ignore(), true), (None, false)] {
                let key = child_key.clone();
                pump_wrapped(&mut tester, move |_app| {
                    CheckboxListTile::new(Some(true), on_changed)
                        .title(Text::new("Hello").key(key.into()))
                        .autofocus(true)
                        .into_widget()
                });
                tester.pump(Duration::ZERO);
                let node = focus_of(&tester, &child_key);
                assert_eq!(has_primary_focus(&tester, node), focused);
            }
        });
    }

    #[test]
    fn checkbox_list_tile_content_padding_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_wrapped(&mut tester, |_app| {
                Center::new()
                    .child(
                        CheckboxListTile::new(Some(false), None)
                            .title(AhemText::new("Title"))
                            .content_padding(EdgeInsets::from_ltrb(10.0, 18.0, 4.0, 2.0)),
                    )
                    .into_widget()
            });
            let padding_rect = rect_of::<waft_widgets::SafeArea>(&tester, 0);
            let checkbox_rect = rect_of::<Checkbox>(&tester, 0);
            let title_rect = text_rect(&tester, "Title");
            let taller = if checkbox_rect.height() > title_rect.height() {
                checkbox_rect
            } else {
                title_rect
            };

            // Check the offsets of Checkbox and title after padding is applied.
            assert_eq!(padding_rect.right, checkbox_rect.right + 4.0);
            assert_eq!(padding_rect.left, title_rect.left - 10.0);

            // Calculate the remaining height from the default ListTile height.
            let remaining_height = 56.0 - taller.height();
            assert_eq!(padding_rect.top, taller.top - remaining_height / 2.0 - 18.0);
            assert_eq!(
                padding_rect.bottom,
                taller.bottom + remaining_height / 2.0 + 2.0
            );
        });
    }

    #[test]
    fn checkbox_list_tile_tristate_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let value = Rc::new(Cell::new(Some(false)));
            let tristate = Rc::new(Cell::new(false));
            let host = {
                let tristate = Rc::clone(&tristate);
                stateful_tile(&value, move |tile| {
                    tile.title(Text::new("Title")).tristate(tristate.get())
                })
            };
            pump_wrapped(&mut tester, |_app| host);
            let shown = |tester: &WidgetTester| {
                tester.widget(nth::<Checkbox>(tester, 0), |checkbox: &Checkbox| {
                    checkbox.value
                })
            };
            let tap_checkbox = |tester: &mut WidgetTester| {
                let at = checkbox_center(tester);
                tester.tap(at, 1);
                tester.pump_and_settle();
            };
            let tap_tile = |tester: &mut WidgetTester| {
                let at = rect_of::<ListTile>(tester, 0).center();
                tester.tap(at, 1);
                tester.pump_and_settle();
            };
            assert_eq!(shown(&tester), Some(false));

            // Tap the checkbox when tristate is disabled.
            for expected in [Some(true), Some(false)] {
                tap_checkbox(&mut tester);
                assert_eq!(value.get(), expected);
            }
            // Tap the listTile when tristate is disabled.
            for expected in [Some(true), Some(false)] {
                tap_tile(&mut tester);
                assert_eq!(value.get(), expected);
            }

            // Enable tristate: Dart's flag is read by the next build, which a tap triggers.
            tristate.set(true);
            tester.pump_and_settle();
            assert_eq!(shown(&tester), Some(false));

            // Tap the checkbox when tristate is enabled.
            for expected in [Some(true), None, Some(false)] {
                tap_checkbox(&mut tester);
                assert_eq!(value.get(), expected);
            }
            // Tap the listTile when tristate is enabled.
            for expected in [Some(true), None, Some(false)] {
                tap_tile(&mut tester);
                assert_eq!(value.get(), expected);
            }
        });
    }

    #[test]
    fn checkbox_list_tile_respects_shape() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_wrapped(&mut tester, |_app| {
                CheckboxListTile::new(Some(false), None)
                    .title(Text::new("Title"))
                    .shape(ShapeBorderRef::new(right_rounded_border()))
                    .into_widget()
            });
            let shape: ShapeBorderRef = ShapeBorderRef::new(right_rounded_border());
            assert!(ink_well_custom_border(&tester).as_deref() == Some(&*shape));
        });
    }

    #[test]
    fn checkbox_list_tile_respects_tile_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let tile_color = Colors::RED.shade500();
            pump_wrapped(&mut tester, |_app| {
                Center::new()
                    .child(
                        CheckboxListTile::new(Some(false), None)
                            .title(Text::new("Title"))
                            .tile_color(tile_color),
                    )
                    .into_widget()
            });
            assert!(ink_rect_colors(&tester).contains(&tile_color.to_argb32()));
        });
    }

    #[test]
    fn checkbox_list_tile_respects_selected_tile_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let selected_tile_color = Colors::GREEN.shade500();
            pump_wrapped(&mut tester, |_app| {
                Center::new()
                    .child(
                        CheckboxListTile::new(Some(false), None)
                            .title(Text::new("Title"))
                            .selected(true)
                            .selected_tile_color(selected_tile_color),
                    )
                    .into_widget()
            });
            assert!(ink_rect_colors(&tester).contains(&selected_tile_color.to_argb32()));
        });
    }

    #[test]
    fn checkbox_list_tile_selected_item_text_color() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/pull/76908
            let mut tester = tester();
            const ACTIVE_COLOR: Color = Color::new(0xff00ff00);
            for (active_color, fill_color) in
                [(None, Some(ACTIVE_COLOR)), (Some(ACTIVE_COLOR), None)]
            {
                pump_app(
                    &mut tester,
                    move |app| {
                        ThemeData::builder()
                            .checkbox_theme(fill_when_selected(fill_color))
                            .build(app)
                    },
                    move |_app| {
                        let mut tile = CheckboxListTile::new(Some(true), ignore())
                            .selected(true)
                            .title(Text::new("title"));
                        tile.active_color = active_color;
                        Scaffold::new()
                            .body(Center::new().child(tile))
                            .into_widget()
                    },
                );
                assert_eq!(tester.text_color("title"), ACTIVE_COLOR);
            }
        });
    }

    #[test]
    fn checkbox_list_tile_respects_checkbox_shape_and_side() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let border = || {
                RoundedRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadiusGeometry::all(Radius::circular(5.0)),
                )
            };
            let rrect = |l: f64, t: f64, r: f64, b: f64, radius: f64| {
                RRect::from_ltrbr(l, t, r, b, Radius::circular(radius))
            };
            let frames = [
                (
                    BorderSide {
                        color: Color::new(0xfff44336),
                        ..BorderSide::default()
                    },
                    rrect(12.0, 12.0, 28.0, 28.0, 4.0),
                ),
                (
                    BorderSide {
                        width: 4.0,
                        color: Color::new(0xff424242),
                        ..BorderSide::default()
                    },
                    rrect(15.0, 15.0, 25.0, 25.0, 1.0),
                ),
            ];
            for (side, inner) in frames {
                pump_in_material(&mut tester, m3, move |_app| {
                    Center::new()
                        .child(
                            CheckboxListTile::new(Some(false), ignore())
                                .side(side)
                                .checkbox_shape(OutlinedBorderRef::new(border())),
                        )
                        .into_widget()
                });
                let side_any: AnyBorderSide = side.into();
                tester.widget(
                    nth::<CheckboxListTile>(&tester, 0),
                    |tile: &CheckboxListTile| {
                        assert!(tile.side.as_ref() == Some(&side_any));
                        let shape: &dyn ShapeBorder =
                            tile.checkbox_shape.as_deref().expect("a shape");
                        assert!(shape == &border() as &dyn ShapeBorder);
                    },
                );
                tester.widget(nth::<Checkbox>(&tester, 0), |checkbox: &Checkbox| {
                    assert!(checkbox.side.as_ref() == Some(&side_any));
                    let shape: &dyn ShapeBorder = checkbox.shape.as_deref().expect("a shape");
                    assert!(shape == &border() as &dyn ShapeBorder);
                });
                assert_paints(
                    &tester,
                    &[Paints::RingAt(
                        side.color,
                        rrect(11.0, 11.0, 29.0, 29.0, 5.0),
                        inner,
                    )],
                );
            }
        });
    }

    #[test]
    fn checkbox_list_tile_respects_visual_density() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_wrapped(&mut tester, |_app| {
                Center::new()
                    .child(
                        CheckboxListTile::new(Some(false), ignore())
                            .autofocus(true)
                            .visual_density(VisualDensity::STANDARD),
                    )
                    .into_widget()
            });
            tester.pump_and_settle();
            assert_eq!(
                rect_of::<CheckboxListTile>(&tester, 0).size(),
                Size::new(800.0, 56.0)
            );
        });
    }

    #[test]
    fn checkbox_list_tile_respects_focus_node() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let child_key = GlobalKey::new();
            let key = child_key.clone();
            pump_wrapped(&mut tester, move |_app| {
                Center::new()
                    .child(
                        CheckboxListTile::new(Some(false), ignore())
                            .title(Text::new("A").key(key.into())),
                    )
                    .into_widget()
            });
            tester.pump(Duration::ZERO);
            let tile_node = focus_of(&tester, &child_key);
            tile_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump(Duration::ZERO); // Let the focus take effect.
            let node = focus_of(&tester, &child_key);
            assert!(has_primary_focus(&tester, node));
            assert!(has_primary_focus(&tester, tile_node));
        });
    }

    /// Dart's "onFocusChange callback" case for a tile `make(focus_node, on_focus_change)`.
    pub(crate) fn check_on_focus_change(
        make: impl FnOnce(FocusNodeRef, ValueChanged<bool>) -> WidgetRef,
    ) {
        let mut tester = tester();
        let node = new_focus_node(&tester);
        let got_focus = Rc::new(Cell::new(false));
        let tile = make(node.clone(), record_focus(&got_focus));
        pump_in_material(&mut tester, m3, |_app| tile);

        node.request_focus(&mut tester.cell.borrow_mut(), None);
        tester.pump(Duration::ZERO);
        assert!(got_focus.get());
        assert!(node.has_focus(&mut tester.cell.borrow_mut()));

        node.unfocus(&mut tester.cell.borrow_mut(), UnfocusDisposition::Scope);
        tester.pump(Duration::ZERO);
        assert!(!got_focus.get());
        assert!(!node.has_focus(&mut tester.cell.borrow_mut()));
    }

    #[test]
    fn checkbox_list_tile_on_focus_change_callback() {
        with_main_thread_stack(|| {
            check_on_focus_change(|node, on_focus_change| {
                CheckboxListTile::new(Some(true), ignore())
                    .focus_node(node)
                    .on_focus_change(on_focus_change)
                    .into_widget()
            });
        });
    }

    #[test]
    fn checkbox_list_tile_can_be_disabled() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let value = Rc::new(Cell::new(Some(false)));
            let enabled = Rc::new(Cell::new(true));
            let host = {
                let value = Rc::clone(&value);
                let enabled = Rc::clone(&enabled);
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let (value_now, enabled_now) = (Rc::clone(&value), Rc::clone(&enabled));
                    CheckboxListTile::new(
                        value.get(),
                        Some(Rc::new(move |app: &mut App, new_value: Option<bool>| {
                            let (value, enabled) = (Rc::clone(&value_now), Rc::clone(&enabled_now));
                            set_state(app, &mut |_app| {
                                value.set(new_value);
                                enabled.set(!enabled.get());
                            });
                        })),
                    )
                    .title(Text::new("Title"))
                    .enabled(enabled.get())
                    .into_widget()
                })
                .into_widget()
            };
            pump_wrapped(&mut tester, |_app| host);
            let shown = |tester: &WidgetTester| {
                tester.widget(nth::<Checkbox>(tester, 0), |checkbox: &Checkbox| {
                    checkbox.value
                })
            };
            // verify initial values
            assert_eq!(shown(&tester), Some(false));
            assert!(enabled.get());

            // Tap the checkbox to disable CheckboxListTile
            let at = checkbox_center(&tester);
            tester.tap(at, 1);
            tester.pump_and_settle();
            assert_eq!(shown(&tester), Some(true));
            assert!(!enabled.get());
            tester.tap(at, 2);
            tester.pump_and_settle();
            assert_eq!(shown(&tester), Some(true));
        });
    }

    pub(crate) fn selected_grab_mouse_cursor() -> MouseCursorRef {
        WidgetStateMouseCursor::resolve_with(
            |states| {
                if states.contains(&WidgetState::Selected) {
                    SystemMouseCursors::GRAB.into()
                } else {
                    SystemMouseCursors::BASIC.into()
                }
            },
            Some("_SelectedGrabMouseCursor()"),
        )
        .into()
    }

    /// `wrap(child: MouseRegion(cursor: forbidden, child: tile))`.
    pub(crate) fn pump_in_forbidden_region(tester: &mut WidgetTester, tile: WidgetRef) {
        pump_wrapped(tester, |_app| {
            MouseRegion::new()
                .cursor(SystemMouseCursors::FORBIDDEN.into())
                .child(tile)
                .into_widget()
        });
    }

    #[test]
    fn checkbox_list_tile_respects_mouse_cursor_when_hovered() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            // Test Checkbox() constructor
            pump_in_forbidden_region(
                &mut tester,
                CheckboxListTile::new(Some(true), ignore())
                    .mouse_cursor(MouseCursorRef::from(SystemMouseCursors::TEXT))
                    .into_widget(),
            );
            let mut gesture = WidgetTester::mouse(1, 0);
            let center = checkbox_center(&tester);
            tester.add_pointer(&mut gesture, center);
            tester.pump(Duration::ZERO);
            tester.move_to(&mut gesture, center, false);
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::TEXT.into());

            // Test default cursor
            pump_wrapped(&mut tester, |_app| {
                CheckboxListTile::new(Some(true), ignore()).into_widget()
            });
            let expected: MouseCursorRef = if K_IS_WEB {
                SystemMouseCursors::CLICK.into()
            } else {
                SystemMouseCursors::BASIC.into()
            };
            assert_eq!(Some(tester.active_cursor(1)), Some(expected));

            // Test default cursor when disabled
            pump_in_forbidden_region(
                &mut tester,
                CheckboxListTile::new(Some(true), None).into_widget(),
            );
            let center = checkbox_center(&tester);
            tester.move_to(&mut gesture, center, false);
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::BASIC.into());

            // Test cursor when tristate
            pump_in_forbidden_region(
                &mut tester,
                CheckboxListTile::new(None, None)
                    .tristate(true)
                    .mouse_cursor(selected_grab_mouse_cursor())
                    .into_widget(),
            );
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::GRAB.into());

            tester.pump_and_settle();
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn checkbox_list_tile_respects_fill_color_in_enabled_disabled_states() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            const ACTIVE_ENABLED_FILL_COLOR: Color = Color::new(0xFF000001);
            const ACTIVE_DISABLED_FILL_COLOR: Color = Color::new(0xFF000002);
            let fill_color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
                if states.contains(&WidgetState::Disabled) {
                    return Some(ACTIVE_DISABLED_FILL_COLOR);
                }
                Some(ACTIVE_ENABLED_FILL_COLOR)
            });
            for (enabled, expected) in [
                (true, ACTIVE_ENABLED_FILL_COLOR),
                (false, ACTIVE_DISABLED_FILL_COLOR),
            ] {
                let fill_color = Rc::clone(&fill_color);
                pump_wrapped(&mut tester, move |_app| {
                    CheckboxListTile::new(Some(true), if enabled { ignore() } else { None })
                        .fill_color(fill_color)
                        .into_widget()
                });
                tester.pump_and_settle();
                assert_paints(&tester, &[Paints::Rrect(expected)]);
            }
        });
    }

    #[test]
    fn checkbox_list_tile_respects_fill_color_in_hovered_state() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const HOVERED_FILL_COLOR: Color = Color::new(0xFF000001);
            let fill_color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
                if states.contains(&WidgetState::Hovered) {
                    return Some(HOVERED_FILL_COLOR);
                }
                Some(Colors::TRANSPARENT)
            });
            pump_wrapped(&mut tester, |_app| {
                CheckboxListTile::new(Some(true), ignore())
                    .fill_color(fill_color)
                    .into_widget()
            });
            tester.pump_and_settle();

            // Start hovering
            let gesture = hover(&mut tester);
            tester.pump_and_settle();
            assert_paints(&tester, &[Paints::Rrect(HOVERED_FILL_COLOR)]);
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn checkbox_list_tile_respects_hover_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let value = Rc::new(Cell::new(Some(true)));
            let orange = Colors::ORANGE.shade500();
            let frame = |tester: &mut WidgetTester, enabled: bool| {
                let host = stateful_tile(&value, move |mut tile| {
                    if !enabled {
                        tile.on_changed = None;
                    }
                    tile.hover_color(orange)
                });
                pump_wrapped(tester, |_app| host);
                tester.pump_and_settle();
            };
            let unhovered = [Paints::Rrect(
                m3(&tester.cell.borrow()).color_scheme.primary,
            )];
            frame(&mut tester, true);
            assert_paints(&tester, &[Paints::StrokeWidth(2.0)]);
            assert_paints(&tester, &unhovered);

            // Start hovering
            let gesture = hover(&mut tester);
            frame(&mut tester, true);
            assert_paints(&tester, &[Paints::Circle(orange), Paints::StrokeWidth(2.0)]);

            // Check what happens when disabled.
            frame(&mut tester, false);
            crate::checkbox::tests::assert_does_not_paint(&tester, &[Paints::Circle(orange)]);
            assert_paints(&tester, &[Paints::StrokeWidth(2.0)]);
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    /// Dart's `Material.of(tester.element(find.byType(Checkbox)))`: the material the checkbox
    /// paints into, which Dart's `paints` matcher paints with its subtree.
    fn checkbox_material(tester: &WidgetTester) -> waft_rendering::AnyRenderObject {
        let checkbox = nth::<Checkbox>(tester, 0);
        let app = tester.cell.borrow();
        let ink = waft_widgets::LookupBoundary::find_ancestor_render_object_of_type::<
            crate::material::RenderInkFeatures,
        >(&app, checkbox)
        .expect("a Material above the checkbox");
        waft_rendering::RenderBox::as_object(ink)
    }

    #[test]
    fn material2_checkbox_list_tile_respects_overlay_color_in_active_pressed_hovered_states() {
        with_main_thread_stack(|| {
            use crate::radio::tests::{Paints, assert_paints, circle};

            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const FILL_COLOR: Color = Color::new(0xFF000000);
            const ACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000001);
            const INACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000002);
            const HOVER_OVERLAY_COLOR: Color = Color::new(0xFF000003);
            const HOVER_COLOR: Color = Color::new(0xFF000005);
            const SPLASH_RADIUS: f64 = 24.0;
            let overlay = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
                if states.contains(&WidgetState::Pressed) {
                    if states.contains(&WidgetState::Selected) {
                        return Some(ACTIVE_PRESSED_OVERLAY_COLOR);
                    }
                    return Some(INACTIVE_PRESSED_OVERLAY_COLOR);
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(HOVER_OVERLAY_COLOR);
                }
                None
            });
            let frame = |tester: &mut WidgetTester, active: bool, use_overlay: bool| {
                let overlay = Rc::clone(&overlay);
                pump_in_material(tester, m2, move |_app| {
                    let mut tile = CheckboxListTile::new(Some(active), ignore())
                        .fill_color(Rc::new(waft_widgets::WidgetStatePropertyAll::new(Some(
                            FILL_COLOR,
                        )))
                            as WidgetStatePropertyRef<Option<Color>>)
                        .hover_color(HOVER_COLOR)
                        .splash_radius(SPLASH_RADIUS);
                    if use_overlay {
                        tile = tile.overlay_color(overlay);
                    }
                    tile.into_widget()
                });
            };
            let any_circle = || Paints::Circle {
                color: None,
                radius: None,
                stroke_width: None,
            };
            let reaction = FILL_COLOR.with_alpha(crate::constants::K_RADIAL_REACTION_ALPHA);
            let mut presses = Vec::new();
            for (pointer, (active, use_overlay, expected)) in [
                (false, false, reaction),
                (true, false, reaction),
                (false, true, INACTIVE_PRESSED_OVERLAY_COLOR),
                (true, true, ACTIVE_PRESSED_OVERLAY_COLOR),
            ]
            .into_iter()
            .enumerate()
            {
                frame(&mut tester, active, use_overlay);
                presses.push(press(&mut tester, pointer as i64 + 2));
                tester.pump_and_settle();
                assert_paints(
                    &tester,
                    checkbox_material(&tester),
                    &[any_circle(), circle(expected).radius(SPLASH_RADIUS)],
                );
            }

            // Start hovering
            tester.pump_widget(Container::new().into_widget());
            frame(&mut tester, false, true);
            tester.pump_and_settle();
            let gesture = hover(&mut tester);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                checkbox_material(&tester),
                &[circle(HOVER_OVERLAY_COLOR).radius(SPLASH_RADIUS)],
            );

            // Finish gestures to release resources.
            for pressed in &presses {
                tester.up(pressed);
            }
            tester.pump_and_settle();
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn material3_checkbox_list_tile_respects_overlay_color_in_active_pressed_hovered_states() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const FILL_COLOR: Color = Color::new(0xFF000000);
            const ACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000001);
            const INACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000002);
            const HOVER_OVERLAY_COLOR: Color = Color::new(0xFF000003);
            const HOVER_COLOR: Color = Color::new(0xFF000005);
            const SPLASH_RADIUS: f64 = 24.0;
            let overlay = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
                if states.contains(&WidgetState::Pressed) {
                    if states.contains(&WidgetState::Selected) {
                        return Some(ACTIVE_PRESSED_OVERLAY_COLOR);
                    }
                    return Some(INACTIVE_PRESSED_OVERLAY_COLOR);
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(HOVER_OVERLAY_COLOR);
                }
                None
            });
            let frame = |tester: &mut WidgetTester, active: bool, use_overlay: bool| {
                let overlay = Rc::clone(&overlay);
                pump_in_material(tester, m3, move |_app| {
                    let mut tile = CheckboxListTile::new(Some(active), ignore())
                        .fill_color(Rc::new(waft_widgets::WidgetStatePropertyAll::new(Some(
                            FILL_COLOR,
                        )))
                            as WidgetStatePropertyRef<Option<Color>>)
                        .hover_color(HOVER_COLOR)
                        .splash_radius(SPLASH_RADIUS);
                    if use_overlay {
                        tile = tile.overlay_color(overlay);
                    }
                    tile.into_widget()
                });
            };
            let reaction = FILL_COLOR.with_alpha(crate::constants::K_RADIAL_REACTION_ALPHA);
            let mut presses = Vec::new();
            for (pointer, (active, use_overlay, expected)) in [
                (false, false, reaction),
                (true, false, reaction),
                (false, true, INACTIVE_PRESSED_OVERLAY_COLOR),
                (true, true, ACTIVE_PRESSED_OVERLAY_COLOR),
            ]
            .into_iter()
            .enumerate()
            {
                frame(&mut tester, active, use_overlay);
                presses.push(press(&mut tester, pointer as i64 + 2));
                tester.pump_and_settle();
                assert_paints(&tester, &[Paints::CircleRadius(expected, SPLASH_RADIUS)]);
            }

            // Start hovering
            tester.pump_widget(Container::new().into_widget());
            frame(&mut tester, false, true);
            tester.pump_and_settle();
            let gesture = hover(&mut tester);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                &[Paints::CircleRadius(HOVER_OVERLAY_COLOR, SPLASH_RADIUS)],
            );

            // Finish gestures to release resources.
            for pressed in &presses {
                tester.up(pressed);
            }
            tester.pump_and_settle();
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn checkbox_list_tile_respects_splash_radius() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const SPLASH_RADIUS: f64 = 30.0;
            let orange = Colors::ORANGE.shade500();
            pump_wrapped(&mut tester, |_app| {
                CheckboxListTile::new(Some(false), ignore())
                    .hover_color(orange)
                    .splash_radius(SPLASH_RADIUS)
                    .into_widget()
            });
            tester.pump_and_settle();
            let gesture = hover(&mut tester);
            tester.pump_and_settle();
            assert_paints(&tester, &[Paints::CircleRadius(orange, SPLASH_RADIUS)]);
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn checkbox_list_tile_respects_material_tap_target_size() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_wrapped(&mut tester, |_app| {
                CheckboxListTile::new(Some(true), ignore()).into_widget()
            });
            // default test
            assert_eq!(checkbox_size(&tester), Size::new(40.0, 40.0));

            pump_wrapped(&mut tester, |_app| {
                CheckboxListTile::new(Some(true), ignore())
                    .material_tap_target_size(MaterialTapTargetSize::Padded)
                    .into_widget()
            });
            assert_eq!(checkbox_size(&tester), Size::new(48.0, 48.0));
        });
    }

    #[test]
    fn material3_checkbox_list_tile_respects_is_error() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let colors = m3(&tester.cell.borrow()).color_scheme.clone();
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let value = Rc::new(Cell::new(Some(true)));
            let host = stateful_tile(&value, |tile| tile.is_error(true));

            // Default color
            tester.pump_widget(Container::new().into_widget());
            pump_in_material(&mut tester, m3, |_app| {
                Center::new().child(host).into_widget()
            });
            tester.pump_and_settle();
            assert_paints(
                &tester,
                &[Paints::Rrect(colors.error), Paints::Path(colors.on_error)],
            );

            // Start hovering
            let gesture = hover(&mut tester);
            tester.pump_and_settle();
            assert_paints(
                &tester,
                &[
                    Paints::Circle(colors.error.with_opacity(0.08)),
                    Paints::Rrect(colors.error),
                ],
            );
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn checkbox_list_tile_adaptive_shows_the_correct_checkbox_platform_widget() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            for (platform, cupertino) in [
                (TargetPlatform::IOS, 1),
                (TargetPlatform::MacOS, 1),
                (TargetPlatform::Android, 0),
                (TargetPlatform::Fuchsia, 0),
                (TargetPlatform::Linux, 0),
                (TargetPlatform::Windows, 0),
            ] {
                pump_in_material(
                    &mut tester,
                    move |app| ThemeData::builder().platform(platform).build(app),
                    |_app| {
                        Center::new()
                            .child(CheckboxListTile::adaptive(Some(false), ignore()))
                            .into_widget()
                    },
                );
                tester.pump_and_settle();
                assert_eq!(
                    tester
                        .find_by_type::<waft_cupertino::CupertinoCheckbox>()
                        .len(),
                    cupertino,
                    "{platform:?}"
                );
            }
        });
    }

    #[test]
    fn checkbox_list_tile_respects_enable_feedback() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            for enable_feedback in [false, true] {
                pump_wrapped(&mut tester, |_app| {
                    Center::new()
                        .child(
                            CheckboxListTile::new(Some(false), ignore())
                                .enable_feedback(enable_feedback),
                        )
                        .into_widget()
                });
                assert_eq!(ink_well_enable_feedback(&tester), enable_feedback);
            }
        });
    }

    /// Dart's "control widget should not request focus on traversal" case for two tiles
    /// `make(title)`.
    pub(crate) fn check_control_is_skipped_by_traversal(make: impl Fn(WidgetRef) -> WidgetRef) {
        let mut tester = tester();
        let first_child_key = GlobalKey::new();
        let second_child_key = GlobalKey::new();
        let first = make(
            Text::new("Hey")
                .key(first_child_key.clone().into())
                .into_widget(),
        );
        let second = make(
            Text::new("There")
                .key(second_child_key.clone().into())
                .into_widget(),
        );
        pump_in_material(&mut tester, m3, |_app| {
            Column::new().children([first, second]).into_widget()
        });
        tester.pump(Duration::ZERO);
        let first_node = focus_of(&tester, &first_child_key);
        first_node.request_focus(&mut tester.cell.borrow_mut(), None);
        tester.pump(Duration::ZERO);
        assert!(has_primary_focus(&tester, first_node));
        first_node.next_focus(&mut tester.cell.borrow_mut());
        tester.pump(Duration::ZERO);
        assert!(!has_primary_focus(
            &tester,
            focus_of(&tester, &first_child_key)
        ));
        assert!(has_primary_focus(
            &tester,
            focus_of(&tester, &second_child_key)
        ));
    }

    #[test]
    fn checkbox_list_tile_control_widget_should_not_request_focus_on_traversal() {
        with_main_thread_stack(|| {
            check_control_is_skipped_by_traversal(|title| {
                CheckboxListTile::new(Some(true), ignore())
                    .title(title)
                    .into_widget()
            });
        });
    }

    /// Dart's "uses ListTileTheme controlAffinity" case: where the control `C` of
    /// `make()` sits under each affinity.
    pub(crate) fn control_top_left_under<C: 'static>(
        make: impl Fn() -> WidgetRef,
        control_affinity: ListTileControlAffinity,
    ) -> Offset {
        let mut tester = tester();
        let tile = make();
        pump_in_material(&mut tester, m3, |_app| {
            ListTileTheme::new(tile)
                .data(ListTileThemeData::new().control_affinity(control_affinity))
                .into_widget()
        });
        rect_of::<C>(&tester, 0).top_left()
    }

    #[test]
    fn checkbox_list_tile_uses_list_tile_theme_control_affinity() {
        with_main_thread_stack(|| {
            let make = || CheckboxListTile::new(Some(false), ignore()).into_widget();
            for (control_affinity, expected) in [
                (ListTileControlAffinity::Trailing, Offset::new(736.0, 8.0)),
                (ListTileControlAffinity::Leading, Offset::new(16.0, 8.0)),
                (ListTileControlAffinity::Platform, Offset::new(736.0, 8.0)),
            ] {
                assert_eq!(
                    control_top_left_under::<Checkbox>(make, control_affinity),
                    expected
                );
            }
        });
    }

    #[test]
    fn checkbox_list_tile_renders_with_default_scale() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_in_material(&mut tester, m3, |_app| {
                CheckboxListTile::new(Some(false), None).into_widget()
            });
            let checkbox = nth::<Checkbox>(&tester, 0);
            let app = tester.cell.borrow();
            assert!(!tester.ancestors(checkbox).into_iter().any(|ancestor| {
                waft_widgets::downcast_widget::<Transform>(&**ancestor.widget(&app)).is_some()
            }));
        });
    }

    #[test]
    fn checkbox_list_tile_respects_checkbox_scale_factor() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            const SCALE: f64 = 1.5;
            pump_in_material(&mut tester, m3, |_app| {
                CheckboxListTile::new(Some(false), None)
                    .checkbox_scale_factor(SCALE)
                    .into_widget()
            });
            let transform = tester.ancestor_of_type::<Transform>(nth::<Checkbox>(&tester, 0));
            let matrix = tester.widget(transform, |transform: &Transform| transform.transform);
            assert_eq!(matrix, Matrix4::scale(SCALE as f32, SCALE as f32));
        });
    }

    fn three_line_tile(is_three_line: Option<bool>, subtitle: &str) -> WidgetRef {
        let mut tile = CheckboxListTile::new(Some(false), None)
            .title(AhemText::new("A"))
            .subtitle(AhemText::new(subtitle));
        tile.is_three_line = is_three_line;
        tile.into_widget()
    }

    fn adaptive_three_line_tile(is_three_line: Option<bool>, subtitle: &str) -> WidgetRef {
        let mut tile = CheckboxListTile::adaptive(Some(false), None)
            .title(AhemText::new("A"))
            .subtitle(AhemText::new(subtitle));
        tile.is_three_line = is_three_line;
        tile.into_widget()
    }

    #[test]
    fn checkbox_list_tile_is_three_line() {
        with_main_thread_stack(|| {
            for (frame, three_line) in IS_THREE_LINE_FRAMES {
                assert_eq!(
                    is_three_line_rects::<Checkbox, CheckboxListTile>(
                        TargetPlatform::Android,
                        frame,
                        three_line_tile
                    ),
                    expected_three_line_rects(
                        three_line,
                        800.0 - 40.0 - 24.0,
                        Size::new(40.0, 40.0),
                        [130.0, 300.0 + 16.0]
                    ),
                    "{frame:?}"
                );
            }
        });
    }

    #[test]
    fn checkbox_list_tile_adaptive_is_three_line() {
        with_main_thread_stack(|| {
            for (frame, three_line) in IS_THREE_LINE_FRAMES {
                assert_eq!(
                    is_three_line_rects::<Checkbox, CheckboxListTile>(
                        TargetPlatform::IOS,
                        frame,
                        adaptive_three_line_tile
                    ),
                    expected_three_line_rects(
                        three_line,
                        800.0 - 44.0 - 24.0,
                        Size::new(44.0, 44.0),
                        [128.0, 300.0 + 14.0]
                    ),
                    "{frame:?}"
                );
            }
        });
    }

    fn title_alignment_tile(
        title_alignment: Option<ListTileTitleAlignment>,
        secondary: WidgetRef,
        with_subtitle: bool,
    ) -> WidgetRef {
        let mut tile = CheckboxListTile::new(Some(true), ignore())
            .control_affinity(ListTileControlAffinity::Leading)
            .title(SizedBox::new().width(20.0).height(50.0))
            .secondary(secondary);
        if with_subtitle {
            tile = tile.subtitle(SizedBox::new().width(20.0).height(50.0));
        }
        tile.title_alignment = title_alignment;
        tile.into_widget()
    }

    #[test]
    fn title_alignment_position_with_title_widget() {
        with_main_thread_stack(|| {
            check_title_alignment::<Checkbox>(title_alignment_tile, false);
        });
    }

    #[test]
    fn title_alignment_position_with_title_and_subtitle_widgets() {
        with_main_thread_stack(|| {
            check_title_alignment::<Checkbox>(title_alignment_tile, true);
        });
    }

    /// Dart's "does not crash at zero area" case for `tile`.
    pub(crate) fn check_zero_area<W: 'static>(tile: WidgetRef) {
        let mut tester = tester();
        pump_app(&mut tester, m3, |app| {
            let theme = m3(app);
            Center::new()
                .child(
                    SizedBox::shrink()
                        .child(Scaffold::new().body(bare(theme, Material::new().child(tile)))),
                )
                .into_widget()
        });
        assert_eq!(rect_of::<W>(&tester, 0).size(), Size::ZERO);
    }

    #[test]
    fn checkbox_list_tile_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            check_zero_area::<CheckboxListTile>(
                CheckboxListTile::new(Some(true), ignore())
                    .title(Text::new("X"))
                    .into_widget(),
            );
        });
    }

    fn leading_title_tile() -> CheckboxListTile {
        CheckboxListTile::new(Some(true), ignore())
            .control_affinity(ListTileControlAffinity::Leading)
            .title(AhemText::new("title"))
    }

    /// Dart's "horizontalTitleGap = 0.0" case for `make(gap)`, whose title starts (LTR) and
    /// ends (RTL) at the given edges.
    pub(crate) fn check_horizontal_title_gap_0(
        [ltr_edge, rtl_edge]: [f64; 2],
        make: impl Fn(Option<f64>) -> WidgetRef,
    ) {
        for (text_direction, edge) in [
            (TextDirection::Ltr, ltr_edge),
            (TextDirection::Rtl, rtl_edge),
        ] {
            for (theme_gap, widget_gap) in THEME_AND_WIDGET_FRAMES {
                let mut data = ListTileThemeData::new();
                data.horizontal_title_gap = theme_gap;
                let tester = spacing_frame(m3, text_direction, data, make(widget_gap));
                assert_eq!(list_tile_size(&tester), Size::new(800.0, 56.0));
                assert_eq!(title_edge(&tester, text_direction), edge);
            }
        }
    }

    #[test]
    fn checkbox_list_tile_horizontal_title_gap_0() {
        with_main_thread_stack(|| {
            check_horizontal_title_gap_0([56.0, 744.0], |gap| {
                let mut tile = leading_title_tile();
                tile.horizontal_title_gap = gap;
                tile.into_widget()
            });
        });
    }

    /// Dart's "horizontalTitleGap = (default) && ListTile minLeadingWidth = (default)" case.
    pub(crate) fn check_default_horizontal_title_gap(
        [ltr_edge, rtl_edge]: [f64; 2],
        make: impl Fn() -> WidgetRef,
    ) {
        // horizontalTitleGap: ListTileDefaultValue.horizontalTitleGap (16.0)
        for (text_direction, edge) in [
            (TextDirection::Ltr, ltr_edge),
            (TextDirection::Rtl, rtl_edge),
        ] {
            let tester = spacing_frame(m3, text_direction, ListTileThemeData::new(), make());
            assert_eq!(list_tile_size(&tester), Size::new(800.0, 56.0));
            assert_eq!(title_edge(&tester, text_direction), edge);
        }
    }

    #[test]
    fn checkbox_list_tile_default_horizontal_title_gap_and_min_leading_width() {
        with_main_thread_stack(|| {
            check_default_horizontal_title_gap([72.0, 728.0], || {
                leading_title_tile().into_widget()
            });
        });
    }

    /// Dart's "horizontalTitleGap with visualDensity" case: twice the same frame, the title
    /// starting at `edge`.
    pub(crate) fn check_horizontal_title_gap_with_visual_density(
        edge: f64,
        make: impl Fn() -> WidgetRef,
    ) {
        let mut tester = spacing_frame(m3, TextDirection::Ltr, ListTileThemeData::new(), make());
        assert_eq!(list_tile_size(&tester), Size::new(800.0, 56.0));
        assert_eq!(title_edge(&tester, TextDirection::Ltr), edge);

        // Pump another frame of the same widget to ensure the underlying render
        // object did not cache the original horizontalTitleGap calculation based on the
        // visualDensity
        let tile = make();
        pump_app(&mut tester, m3, |_app| {
            Directionality::new(
                TextDirection::Ltr,
                Material::new().child(
                    ListTileTheme::new(
                        Container::new()
                            .alignment(AlignmentGeometry::TOP_LEFT)
                            .child(tile),
                    )
                    .data(ListTileThemeData::new()),
                ),
            )
            .into_widget()
        });
        assert_eq!(list_tile_size(&tester), Size::new(800.0, 56.0));
        assert_eq!(title_edge(&tester, TextDirection::Ltr), edge);
    }

    #[test]
    fn checkbox_list_tile_horizontal_title_gap_with_visual_density() {
        with_main_thread_stack(|| {
            check_horizontal_title_gap_with_visual_density(58.0, || {
                leading_title_tile()
                    .visual_density(VisualDensity::new(VisualDensity::MINIMUM_DENSITY, 0.0))
                    .horizontal_title_gap(10.0)
                    .into_widget()
            });
        });
    }

    /// Dart's "minVerticalPadding = 80.0" case under `theme`, with the tile `height` it
    /// expects.
    pub(crate) fn check_min_vertical_padding_80(
        theme: fn(&App) -> ThemeData,
        height: f64,
        make: impl Fn(Option<f64>) -> WidgetRef,
    ) {
        for text_direction in [TextDirection::Ltr, TextDirection::Rtl] {
            for (theme_padding, widget_padding) in theme_and_widget_frames(80.0) {
                let mut data = ListTileThemeData::new();
                data.min_vertical_padding = theme_padding;
                let tester = spacing_frame(theme, text_direction, data, make(widget_padding));
                assert_eq!(list_tile_size(&tester), Size::new(800.0, height));
            }
        }
    }

    fn min_vertical_padding_tile(padding: Option<f64>) -> WidgetRef {
        let mut tile = CheckboxListTile::new(Some(true), ignore()).title(AhemText::new("title"));
        tile.min_vertical_padding = padding;
        tile.into_widget()
    }

    #[test]
    fn checkbox_list_tile_min_vertical_padding_80_material_3() {
        // 80 + 80 + 24(Title) = 184
        with_main_thread_stack(|| {
            check_min_vertical_padding_80(m3, 184.0, min_vertical_padding_tile);
        });
    }

    #[test]
    fn checkbox_list_tile_min_vertical_padding_80_material_2() {
        // 80 + 80 + 16(Title) = 176
        with_main_thread_stack(|| {
            check_min_vertical_padding_80(m2, 176.0, min_vertical_padding_tile);
        });
    }

    /// Dart's "minLeadingWidth = 60.0" case.
    pub(crate) fn check_min_leading_width_60(make: impl Fn(Option<f64>) -> WidgetRef) {
        // 92.0 = 16.0(Default contentPadding) + 16.0(Default horizontalTitleGap) + 60.0
        // 708.0 = 800.0 - (16.0(Default contentPadding) + 16.0(Default horizontalTitleGap) + 60.0)
        for (text_direction, edge) in [(TextDirection::Ltr, 92.0), (TextDirection::Rtl, 708.0)] {
            for (theme_width, widget_width) in theme_and_widget_frames(60.0) {
                let mut data = ListTileThemeData::new();
                data.min_leading_width = theme_width;
                let tester = spacing_frame(m3, text_direction, data, make(widget_width));
                assert_eq!(list_tile_size(&tester), Size::new(800.0, 56.0));
                assert_eq!(title_edge(&tester, text_direction), edge);
            }
        }
    }

    #[test]
    fn checkbox_list_tile_min_leading_width_60() {
        with_main_thread_stack(|| {
            check_min_leading_width_60(|width| {
                let mut tile = leading_title_tile();
                tile.min_leading_width = width;
                tile.into_widget()
            });
        });
    }

    /// Dart's "minTileHeight" case.
    pub(crate) fn check_min_tile_height(make: impl Fn(Option<f64>) -> WidgetRef) {
        for (min_tile_height, height) in [(None, 56.0), (Some(30.0), 30.0), (Some(60.0), 60.0)] {
            let tester = spacing_frame(
                m3,
                TextDirection::Ltr,
                ListTileThemeData::new(),
                make(min_tile_height),
            );
            assert_eq!(list_tile_size(&tester), Size::new(800.0, height));
        }
    }

    #[test]
    fn checkbox_list_tile_min_tile_height() {
        with_main_thread_stack(|| {
            check_min_tile_height(|min_tile_height| {
                let mut tile = CheckboxListTile::new(Some(true), ignore());
                tile.min_tile_height = min_tile_height;
                tile.into_widget()
            });
        });
    }

    /// Dart's `assert(tristate || value != null)`.
    #[test]
    #[should_panic(expected = "self.tristate || self.value.is_some()")]
    fn a_checkbox_list_tile_that_is_not_tristate_without_a_value_panics_when_made_a_widget() {
        CheckboxListTile::new(None, ignore()).into_widget();
    }
}
