//! Flutter counterpart: material_ui's `switch_list_tile.dart`.
//!
//! The thumb images (`activeThumbImage`, `inactiveThumbImage` and their error listeners)
//! wait with [`Switch`]'s, on painting's `DecorationImage`.
#![allow(deprecated)]

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::Color;
use waft_foundation::{App, Ref, ValueChanged};
use waft_gestures::{DragStartBehavior, GestureTapCallback};
use waft_painting::{EdgeInsetsGeometry, ShapeBorderRef};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, ExcludeFocus, FocusNodeRef, Icon, IntoWidget, KeyRef, MergeSemantics,
    StatelessWidget, WidgetRef, WidgetState, WidgetStatePropertyRef, WidgetStates,
    WidgetStatesController,
};

use crate::list_tile::{ListTile, ListTileControlAffinity};
use crate::list_tile_theme::ListTileTheme;
use crate::switch::Switch;
use crate::switch_theme::SwitchTheme;
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};

/// Dart's `_SwitchListTileType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SwitchListTileType {
    Material,
    Adaptive,
}

/// A [`ListTile`] with a [`Switch`]. In other words, a switch with a label.
///
/// The entire list tile is interactive: tapping anywhere in the tile toggles
/// the switch. Tapping and dragging the [`Switch`] also triggers the
/// [`on_changed`](Self::on_changed) callback.
///
/// To ensure that [`on_changed`](Self::on_changed) correctly triggers, the state passed
/// into [`value`](Self::value) must be properly managed. This is typically done by invoking
/// `State::set_state` in [`on_changed`](Self::on_changed) to toggle the state value.
///
/// The [`value`](Self::value), [`on_changed`](Self::on_changed) and
/// [`active_thumb_color`](Self::active_thumb_color) properties of this widget are identical to
/// the similarly-named properties on the [`Switch`] widget.
///
/// The [`title`](Self::title), [`subtitle`](Self::subtitle),
/// [`is_three_line`](Self::is_three_line), and [`dense`](Self::dense) properties are like
/// those of the same name on [`ListTile`].
///
/// The [`selected`](Self::selected) property on this widget is similar to the
/// [`ListTile::selected`] property. This tile's [`active_thumb_color`](Self::active_thumb_color)
/// is used for the selected item's text color, or the theme's `SwitchThemeData::overlay_color`
/// if [`active_thumb_color`](Self::active_thumb_color) is null.
///
/// This widget does not coordinate the [`selected`](Self::selected) state and the
/// [`value`](Self::value); to have the list tile appear selected when the
/// switch button is on, use the same value for both.
///
/// The switch is shown on the right by default in left-to-right languages (i.e.
/// in the [`ListTile::trailing`] slot) which can be changed using
/// [`control_affinity`](Self::control_affinity). The [`secondary`](Self::secondary) widget is
/// placed in the [`ListTile::leading`] slot.
///
/// This widget requires a `Material` widget ancestor in the tree to paint
/// itself on, which is typically provided by the app's `Scaffold`.
/// The [`tile_color`](Self::tile_color), and [`selected_tile_color`](Self::selected_tile_color)
/// are not painted by the [`SwitchListTile`] itself but by the `Material` widget ancestor. In
/// this case, one can wrap a `Material` widget around the [`SwitchListTile`].
///
/// ## Performance considerations when wrapping [`SwitchListTile`] with `Material`
///
/// Wrapping a large number of [`SwitchListTile`]s individually with `Material`s
/// is expensive. Consider only wrapping the [`SwitchListTile`]s that require it
/// or include a common `Material` ancestor where possible.
///
/// To show the [`SwitchListTile`] as disabled, pass null as the
/// [`on_changed`](Self::on_changed) callback.
///
/// ## SwitchListTile isn't exactly what I want
///
/// If the way SwitchListTile pads and positions its elements isn't quite what
/// you're looking for, you can create custom labeled switch widgets by
/// combining [`Switch`] with other widgets, such as `Text`, `Padding` and
/// `InkWell`.
///
/// See also:
///
///  * [`ListTileTheme`], which can be used to affect the style of list tiles,
///    including switch list tiles.
///  * `CheckboxListTile`, a similar widget for checkboxes.
///  * `RadioListTile`, a similar widget for radio buttons.
///  * [`ListTile`] and [`Switch`], the widgets from which this widget is made.
pub struct SwitchListTile {
    pub key: Option<KeyRef>,
    /// Whether this switch is checked.
    pub value: bool,
    /// Called when the user toggles the switch on or off.
    ///
    /// The switch passes the new value to the callback but does not actually
    /// change state until the parent widget rebuilds the switch tile with the
    /// new value.
    ///
    /// If null, the switch will be displayed as disabled.
    pub on_changed: Option<ValueChanged<bool>>,
    /// The color to use when this switch is on.
    ///
    /// Defaults to `ColorScheme::secondary` of the current [`Theme`].
    #[deprecated(
        note = "Use activeThumbColor or activeTrackColor instead. This feature was deprecated after v3.31.0-2.0.pre."
    )]
    pub active_color: Option<Color>,
    /// The color to use on the thumb when this switch is on.
    ///
    /// Defaults to `ColorScheme::secondary` of the current [`Theme`].
    pub active_thumb_color: Option<Color>,
    /// The color to use on the track when this switch is on.
    ///
    /// Defaults to `ColorScheme::secondary` with the opacity set at 50%.
    ///
    /// Ignored if created with [`SwitchListTile::adaptive`].
    pub active_track_color: Option<Color>,
    /// The color to use on the thumb when this switch is off.
    ///
    /// Defaults to the colors described in the Material design specification.
    ///
    /// Ignored if created with [`SwitchListTile::adaptive`].
    pub inactive_thumb_color: Option<Color>,
    /// The color to use on the track when this switch is off.
    ///
    /// Defaults to the colors described in the Material design specification.
    ///
    /// Ignored if created with [`SwitchListTile::adaptive`].
    pub inactive_track_color: Option<Color>,
    /// The color of this switch's thumb.
    ///
    /// Resolved in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then the value of [`active_thumb_color`](Self::active_thumb_color) is used in
    /// the selected state and [`inactive_thumb_color`](Self::inactive_thumb_color) in the
    /// default state. If that is also null, then the value of `SwitchThemeData::thumb_color`
    /// is used. If that is also null, The default value is used.
    pub thumb_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// The color of this switch's track.
    ///
    /// Resolved in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then the value of [`active_track_color`](Self::active_track_color) is used in
    /// the selected state and [`inactive_track_color`](Self::inactive_track_color) in the
    /// default state. If that is also null, then the value of `SwitchThemeData::track_color`
    /// is used. If that is also null, then the default value is used.
    pub track_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// The outline color of this switch's track.
    ///
    /// The [`ListTile`] will be focused when this [`SwitchListTile`] requests focus,
    /// so the focused outline color of the switch will be ignored.
    ///
    /// In Material 3, the outline color defaults to transparent in the selected
    /// state and `ColorScheme::outline` in the unselected state. In Material 2,
    /// the [`Switch`] track has no outline.
    pub track_outline_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// The icon to use on the thumb of this switch
    ///
    /// Resolved in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then the value of `SwitchThemeData::thumb_icon` is used. If this is
    /// also null, then the [`Switch`] does not have any icons on the thumb.
    pub thumb_icon: Option<WidgetStatePropertyRef<Option<Rc<Icon>>>>,
    /// Configures the minimum size of the tap target.
    ///
    /// defaults to [`MaterialTapTargetSize::ShrinkWrap`].
    pub material_tap_target_size: Option<MaterialTapTargetSize>,
    /// Determines the way that drag start behavior is handled.
    pub drag_start_behavior: DragStartBehavior,
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
    /// If null, then the value of `SwitchThemeData::mouse_cursor` is used. If that
    /// is also null, then `WidgetStateMouseCursor::clickable` is used.
    pub mouse_cursor: Option<MouseCursorRef>,
    /// The color for the switch's `Material`.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Pressed`].
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///
    /// If null, then the value of [`active_thumb_color`](Self::active_thumb_color) with alpha
    /// `K_RADIAL_REACTION_ALPHA` and [`hover_color`](Self::hover_color) is used in the pressed
    /// and hovered state. If that is also null, the value of `SwitchThemeData::overlay_color`
    /// is used. If that is also null, then the default value is used in the pressed and
    /// hovered state.
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// The splash radius of the circular `Material` ink response.
    ///
    /// If null, then the value of `SwitchThemeData::splash_radius` is used. If that
    /// is also null, then `K_RADIAL_REACTION_RADIUS` is used.
    pub splash_radius: Option<f64>,
    /// An optional focus node to use as the focus node for this widget.
    pub focus_node: Option<FocusNodeRef>,
    /// Controls the interactive states of the backing [`ListTile`].
    pub states_controller: Option<Ref<WidgetStatesController>>,
    /// Handler called when the focus changes.
    ///
    /// Called with true if this widget's node gains focus, and false if it loses
    /// focus.
    pub on_focus_change: Option<ValueChanged<bool>>,
    /// True if this widget will be selected as the initial focus when no other
    /// node in its scope is currently focused.
    pub autofocus: bool,
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
    /// A widget to display on the opposite side of the tile from the switch.
    ///
    /// Typically an `Icon` widget.
    pub secondary: Option<WidgetRef>,
    /// Whether this list tile is intended to display three lines of text.
    ///
    /// If null, the value from `ListTileThemeData::is_three_line` is used.
    /// If that is also null, the value from `ThemeData::list_tile_theme` is used.
    /// If still null, the default value is `false`.
    pub is_three_line: Option<bool>,
    /// Whether this list tile is part of a vertically dense list.
    ///
    /// If this property is null then its value is based on `ListTileThemeData::dense`.
    pub dense: Option<bool>,
    /// The tile's internal padding.
    ///
    /// Insets a [`SwitchListTile`]'s contents: its [`title`](Self::title),
    /// [`subtitle`](Self::subtitle), [`secondary`](Self::secondary), and [`Switch`] widgets.
    ///
    /// If null, [`ListTile`]'s default of `EdgeInsets.symmetric(horizontal: 16.0)`
    /// is used.
    pub content_padding: Option<EdgeInsetsGeometry>,
    /// Whether to render icons and text in the [`active_thumb_color`](Self::active_thumb_color).
    ///
    /// No effort is made to automatically coordinate the [`selected`](Self::selected) state
    /// and the [`value`](Self::value) state. To have the list tile appear selected when the
    /// switch is on, pass the same value to both.
    ///
    /// Normally, this property is left to its default value, false.
    pub selected: bool,
    /// If adaptive, creates the switch with [`Switch::adaptive`].
    switch_list_tile_type: SwitchListTileType,
    /// Defines the position of control and [`secondary`](Self::secondary), relative to text.
    ///
    /// By default, the value of [`control_affinity`](Self::control_affinity) is
    /// [`ListTileControlAffinity::Platform`].
    pub control_affinity: Option<ListTileControlAffinity>,
    /// Defines the tile's `InkWell::custom_border` and `Ink::decoration` shape.
    pub shape: Option<ShapeBorderRef>,
    /// If non-null, defines the background color when [`SwitchListTile::selected`] is true.
    pub selected_tile_color: Option<Color>,
    /// Defines how compact the list tile's layout will be.
    pub visual_density: Option<VisualDensity>,
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
    /// The color for the tile's `Material` when a pointer is hovering over it.
    pub hover_color: Option<Color>,
    /// Whether to apply the ambient `CupertinoThemeData`.
    ///
    /// Read through [`effective_apply_cupertino_theme`](Self::effective_apply_cupertino_theme).
    pub apply_cupertino_theme: Option<bool>,
    /// Whether to add button:true to the semantics if onTap is provided.
    /// This is a temporary flag to help changing the behavior of ListTile onTap semantics.
    pub internal_add_semantic_for_on_tap: bool,
}

/// The setters of `SwitchListTile`'s optional arguments that hold a value.
macro_rules! switch_list_tile_setters {
    ($($field:ident: $ty:ty => $name:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `SwitchListTile(", $name, ":)`.")]
            pub fn $field(mut self, $field: impl Into<$ty>) -> SwitchListTile {
                self.$field = Some($field.into());
                self
            }
        )*
    };
}

/// The setters of `SwitchListTile`'s optional arguments that are widgets.
macro_rules! switch_list_tile_widget_setters {
    ($($field:ident => $name:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `SwitchListTile(", $name, ":)`.")]
            pub fn $field<K>(mut self, $field: impl IntoWidget<K>) -> SwitchListTile {
                self.$field = Some($field.into_widget());
                self
            }
        )*
    };
}

impl SwitchListTile {
    /// Creates a combination of a list tile and a switch.
    ///
    /// The switch tile itself does not maintain any state. Instead, when the
    /// state of the switch changes, the widget calls the [`on_changed`](Self::on_changed)
    /// callback. Most widgets that use a switch will listen for the
    /// [`on_changed`](Self::on_changed) callback and rebuild the switch tile with a new
    /// [`value`](Self::value) to update the visual appearance of the switch.
    ///
    /// The following arguments are required:
    ///
    /// * `value` determines whether this switch is on or off.
    /// * `on_changed` is called when the user toggles the switch on or off.
    ///
    /// Dart's `SwitchListTile` has no `applyCupertinoTheme` parameter: its
    /// `applyCupertinoTheme` is false, so an
    /// [`apply_cupertino_theme`](Self::apply_cupertino_theme) set on it is ignored
    /// ([`effective_apply_cupertino_theme`](Self::effective_apply_cupertino_theme)).
    pub fn new(value: bool, on_changed: Option<ValueChanged<bool>>) -> SwitchListTile {
        SwitchListTile::with_type(value, on_changed, SwitchListTileType::Material)
    }

    /// Creates a Material [`ListTile`] with an adaptive [`Switch`], following
    /// Material design's
    /// [Cross-platform guidelines](https://material.io/design/platform-guidance/cross-platform-adaptation.html).
    ///
    /// This widget uses [`Switch::adaptive`] to change the graphics of the switch
    /// component based on the ambient `ThemeData::platform`. On iOS and macOS, a
    /// `CupertinoSwitch` will be used. On other platforms a Material design
    /// [`Switch`] will be used.
    ///
    /// If a `CupertinoSwitch` is created, the following parameters are
    /// ignored: [`active_track_color`](Self::active_track_color),
    /// [`inactive_thumb_color`](Self::inactive_thumb_color),
    /// [`inactive_track_color`](Self::inactive_track_color).
    pub fn adaptive(value: bool, on_changed: Option<ValueChanged<bool>>) -> SwitchListTile {
        SwitchListTile::with_type(value, on_changed, SwitchListTileType::Adaptive)
    }

    fn with_type(
        value: bool,
        on_changed: Option<ValueChanged<bool>>,
        switch_list_tile_type: SwitchListTileType,
    ) -> SwitchListTile {
        SwitchListTile {
            key: None,
            value,
            on_changed,
            active_color: None,
            active_thumb_color: None,
            active_track_color: None,
            inactive_thumb_color: None,
            inactive_track_color: None,
            thumb_color: None,
            track_color: None,
            track_outline_color: None,
            thumb_icon: None,
            material_tap_target_size: None,
            drag_start_behavior: DragStartBehavior::Start,
            mouse_cursor: None,
            overlay_color: None,
            splash_radius: None,
            focus_node: None,
            states_controller: None,
            on_focus_change: None,
            autofocus: false,
            tile_color: None,
            title: None,
            subtitle: None,
            secondary: None,
            is_three_line: None,
            dense: None,
            content_padding: None,
            selected: false,
            switch_list_tile_type,
            control_affinity: None,
            shape: None,
            selected_tile_color: None,
            visual_density: None,
            enable_feedback: None,
            horizontal_title_gap: None,
            min_vertical_padding: None,
            min_leading_width: None,
            min_tile_height: None,
            hover_color: None,
            apply_cupertino_theme: None,
            internal_add_semantic_for_on_tap: false,
        }
    }

    /// Dart `SwitchListTile(key:)`.
    pub fn key(mut self, key: KeyRef) -> SwitchListTile {
        self.key = Some(key);
        self
    }

    /// Dart `SwitchListTile(activeColor:)`.
    #[deprecated(
        note = "Use activeThumbColor or activeTrackColor instead. This feature was deprecated after v3.31.0-2.0.pre."
    )]
    pub fn active_color(mut self, active_color: Color) -> SwitchListTile {
        self.active_color = Some(active_color);
        self
    }

    switch_list_tile_setters!(
        active_thumb_color: Color => "activeThumbColor",
        active_track_color: Color => "activeTrackColor",
        inactive_thumb_color: Color => "inactiveThumbColor",
        inactive_track_color: Color => "inactiveTrackColor",
        thumb_color: WidgetStatePropertyRef<Option<Color>> => "thumbColor",
        track_color: WidgetStatePropertyRef<Option<Color>> => "trackColor",
        track_outline_color: WidgetStatePropertyRef<Option<Color>> => "trackOutlineColor",
        thumb_icon: WidgetStatePropertyRef<Option<Rc<Icon>>> => "thumbIcon",
        material_tap_target_size: MaterialTapTargetSize => "materialTapTargetSize",
        mouse_cursor: MouseCursorRef => "mouseCursor",
        overlay_color: WidgetStatePropertyRef<Option<Color>> => "overlayColor",
        splash_radius: f64 => "splashRadius",
        focus_node: FocusNodeRef => "focusNode",
        states_controller: Ref<WidgetStatesController> => "statesController",
        on_focus_change: ValueChanged<bool> => "onFocusChange",
        apply_cupertino_theme: bool => "applyCupertinoTheme",
        tile_color: Color => "tileColor",
        is_three_line: bool => "isThreeLine",
        dense: bool => "dense",
        content_padding: EdgeInsetsGeometry => "contentPadding",
        control_affinity: ListTileControlAffinity => "controlAffinity",
        shape: ShapeBorderRef => "shape",
        selected_tile_color: Color => "selectedTileColor",
        visual_density: VisualDensity => "visualDensity",
        enable_feedback: bool => "enableFeedback",
        horizontal_title_gap: f64 => "horizontalTitleGap",
        min_vertical_padding: f64 => "minVerticalPadding",
        min_leading_width: f64 => "minLeadingWidth",
        min_tile_height: f64 => "minTileHeight",
        hover_color: Color => "hoverColor",
    );

    switch_list_tile_widget_setters!(
        title => "title",
        subtitle => "subtitle",
        secondary => "secondary",
    );

    /// Dart `SwitchListTile(dragStartBehavior:)`.
    pub fn drag_start_behavior(mut self, drag_start_behavior: DragStartBehavior) -> SwitchListTile {
        self.drag_start_behavior = drag_start_behavior;
        self
    }

    /// Dart `SwitchListTile(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> SwitchListTile {
        self.autofocus = autofocus;
        self
    }

    /// Dart `SwitchListTile(internalAddSemanticForOnTap:)`.
    pub fn internal_add_semantic_for_on_tap(
        mut self,
        internal_add_semantic_for_on_tap: bool,
    ) -> SwitchListTile {
        self.internal_add_semantic_for_on_tap = internal_add_semantic_for_on_tap;
        self
    }

    /// Dart `SwitchListTile(selected:)`.
    pub fn selected(mut self, selected: bool) -> SwitchListTile {
        self.selected = selected;
        self
    }

    /// Dart's `applyCupertinoTheme` field: what
    /// [`apply_cupertino_theme`](Self::apply_cupertino_theme) was given, and false for a tile
    /// made with [`SwitchListTile::new`], whose constructor sets it so.
    pub fn effective_apply_cupertino_theme(&self) -> Option<bool> {
        match self.switch_list_tile_type {
            SwitchListTileType::Material => Some(false),
            SwitchListTileType::Adaptive => self.apply_cupertino_theme,
        }
    }

    /// The `Switch.adaptive` or `Switch` Dart's `build` puts in the control slot.
    fn build_switch(&self) -> Switch {
        let mut switch = match self.switch_list_tile_type {
            SwitchListTileType::Adaptive => {
                let mut switch = Switch::adaptive(self.value, self.on_changed.clone());
                switch.apply_cupertino_theme = self.effective_apply_cupertino_theme();
                switch
            }
            SwitchListTileType::Material => Switch::new(self.value, self.on_changed.clone()),
        }
        .material_tap_target_size(
            self.material_tap_target_size
                .unwrap_or(MaterialTapTargetSize::ShrinkWrap),
        )
        .autofocus(self.autofocus)
        .drag_start_behavior(self.drag_start_behavior);
        switch.active_color = self.active_color;
        switch.active_thumb_color = self.active_thumb_color;
        switch.active_track_color = self.active_track_color;
        switch.inactive_track_color = self.inactive_track_color;
        switch.inactive_thumb_color = self.inactive_thumb_color;
        switch.on_focus_change = self.on_focus_change.clone();
        switch.thumb_color = self.thumb_color.clone();
        switch.track_color = self.track_color.clone();
        switch.track_outline_color = self.track_outline_color.clone();
        switch.thumb_icon = self.thumb_icon.clone();
        switch.mouse_cursor = self.mouse_cursor.clone();
        switch.splash_radius = self.splash_radius;
        switch.overlay_color = self.overlay_color.clone();
        switch
    }

    /// Dart's `onTap` closure: flips this tile's value.
    fn handle_tap(&self) -> Option<GestureTapCallback> {
        let on_changed = Rc::clone(self.on_changed.as_ref()?);
        let value = self.value;
        Some(GestureTapCallback::new(move |app: &mut App| {
            on_changed(app, !value);
        }))
    }
}

impl Debug for SwitchListTile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SwitchListTile")
            .field("value", &if self.value { "on" } else { "off" })
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

impl StatelessWidget for SwitchListTile {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(self.is_three_line != Some(true) || self.subtitle.is_some());
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let control = ExcludeFocus::new(self.build_switch()).into_widget();

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
        let switch_theme = SwitchTheme::of(app, context);
        let mut states = WidgetStates::new();
        if self.selected {
            states.insert(WidgetState::Selected);
        }
        let effective_active_color = self
            .active_thumb_color
            .or(self.active_color)
            .or_else(|| {
                switch_theme
                    .thumb_color
                    .as_ref()
                    .and_then(|thumb_color| thumb_color.resolve(&states))
            })
            .unwrap_or(theme.color_scheme.secondary);
        let mut list_tile = ListTile::new()
            .selected_color(effective_active_color)
            .enabled(self.on_changed.is_some())
            .selected(self.selected)
            .autofocus(self.autofocus);
        list_tile.leading = leading;
        list_tile.title = self.title.clone();
        list_tile.subtitle = self.subtitle.clone();
        list_tile.trailing = trailing;
        list_tile.is_three_line = self.is_three_line;
        list_tile.dense = self.dense;
        list_tile.content_padding = self.content_padding;
        list_tile.on_tap = self.handle_tap();
        list_tile.selected_tile_color = self.selected_tile_color;
        list_tile.shape = self.shape.clone();
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
        list_tile.hover_color = self.hover_color;
        list_tile.internal_add_semantic_for_on_tap = self.internal_add_semantic_for_on_tap;
        MergeSemantics::new().child(list_tile).into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use waft_embedder::{Offset, PointerChange, Size, TargetPlatform, TextDirection};
    use waft_painting::{AlignmentGeometry, EdgeInsets, EdgeInsetsDirectional};
    use waft_services::SystemMouseCursors;
    use waft_widgets::{
        Center, Container, Directionality, FocusHighlightStrategy, GlobalKey, ListView, SizedBox,
        StatefulBuilder, Text, WidgetStateProperty, downcast_widget,
    };

    use waft_widgets::testing::{TestPointer, WidgetTester, test_widgets};

    use super::*;
    use crate::checkbox_list_tile::tests::{
        IS_THREE_LINE_FRAMES, check_control_is_skipped_by_traversal,
        check_default_horizontal_title_gap, check_horizontal_title_gap_0,
        check_horizontal_title_gap_with_visual_density, check_min_leading_width_60,
        check_min_tile_height, check_min_vertical_padding_80, check_on_focus_change,
        expected_three_line_rects, focus_of, has_primary_focus, ink_rect_colors,
        ink_well_custom_border, ink_well_enable_feedback, is_three_line_rects, nth,
        pump_in_material, pump_wrapped, rect_of, right_rounded_border, text_center,
    };
    use crate::colors::Colors;
    use crate::constants::K_RADIAL_REACTION_ALPHA;
    use crate::icons::Icons;
    use crate::list_tile::tests::{m2, m3, text_rect};
    use crate::list_tile_theme::ListTileThemeData;
    use crate::material::Material;
    use crate::scaffold::Scaffold;
    use crate::switch::tests::{Drawn, c, circles, drawn, hover, ignore, press, tap};
    use crate::switch_theme::SwitchThemeData;
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::pump_app;

    /// The colour of the last thing the switch draws: its thumb.
    fn thumb(tester: &WidgetTester) -> Option<u32> {
        drawn(tester).last().and_then(Drawn::color)
    }

    /// The colour of the first thing the switch draws: its track.
    fn track(tester: &WidgetTester) -> Option<u32> {
        drawn(tester).first().and_then(Drawn::color)
    }

    /// The colours of the outlines the switch strokes.
    fn strokes(tester: &WidgetTester) -> Vec<u32> {
        drawn(tester)
            .into_iter()
            .filter_map(|drawn| match drawn {
                Drawn::Stroke(color, ..) => Some(color),
                _ => None,
            })
            .collect()
    }

    /// A tile whose value a `StatefulBuilder` keeps and whose `on_changed` sets.
    fn stateful_tile(
        value: &Rc<Cell<bool>>,
        configure: impl Fn(SwitchListTile) -> SwitchListTile + 'static,
    ) -> WidgetRef {
        let value = Rc::clone(value);
        StatefulBuilder::new(move |_app, _context, set_state| {
            let value_now = Rc::clone(&value);
            configure(SwitchListTile::new(
                value.get(),
                Some(Rc::new(move |app: &mut App, new_value: bool| {
                    let value = Rc::clone(&value_now);
                    set_state(app, &mut |_app| value.set(new_value));
                })),
            ))
            .into_widget()
        })
        .into_widget()
    }

    /// The selected/disabled state frames: whether the tile is enabled and on.
    const STATE_FRAMES: [(bool, bool); 4] =
        [(false, false), (false, true), (true, false), (true, true)];

    /// Dart's `getXxxColor` of the state cases: one colour per state frame.
    fn state_colors(
        [
            inactive_disabled,
            active_disabled,
            inactive_enabled,
            active_enabled,
        ]: [Color; 4],
    ) -> WidgetStatePropertyRef<Option<Color>> {
        <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
            Some(if states.contains(&WidgetState::Disabled) {
                if states.contains(&WidgetState::Selected) {
                    active_disabled
                } else {
                    inactive_disabled
                }
            } else if states.contains(&WidgetState::Selected) {
                active_enabled
            } else {
                inactive_enabled
            })
        })
    }

    /// Dart's `getXxxColor` of the hover cases: `hovered` when hovered, else transparent.
    fn hovered_colors(hovered: Color) -> WidgetStatePropertyRef<Option<Color>> {
        <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
            Some(if states.contains(&WidgetState::Hovered) {
                hovered
            } else {
                Colors::TRANSPARENT
            })
        })
    }

    const STATE_COLORS: [Color; 4] = [
        Color::new(0xFF000004),
        Color::new(0xFF000002),
        Color::new(0xFF000003),
        Color::new(0xFF000001),
    ];

    #[test]
    fn switch_list_tile_control_test() {
        test_widgets(|tester| {
            let log = Rc::new(RefCell::new(Vec::new()));
            let on_changed: ValueChanged<bool> = {
                let log = Rc::clone(&log);
                Rc::new(move |_app, value| log.borrow_mut().push(Some(value)))
            };
            pump_wrapped(tester, |_app| {
                SwitchListTile::new(true, Some(on_changed))
                    .title(Text::new("Hello"))
                    .into_widget()
            });
            let hello = text_center(tester, "Hello");
            tester.tap(hello, 1);
            log.borrow_mut().push(None);
            tap(tester);
            assert_eq!(*log.borrow(), [Some(false), None, Some(false)]);
        });
    }

    #[test]
    fn material2_switch_list_tile_has_the_right_colors() {
        test_widgets(|tester| {
            use crate::switch::tests::{M2_SHADOWS, colors};

            let value = Rc::new(Cell::new(false));
            let host = stateful_tile(&value, |tile| {
                tile.active_color(Colors::RED.shade500())
                    .active_track_color(Colors::GREEN.shade500())
                    .inactive_thumb_color(Colors::YELLOW.shade500())
                    .inactive_track_color(Colors::BLUE.shade500())
            });
            let root = {
                let app = tester.cell.borrow();
                waft_widgets::MediaQuery::new(
                    waft_widgets::MediaQueryData::new().padding(EdgeInsets::all(8.0)),
                    crate::theme::Theme::new(
                        m2(&app),
                        Directionality::new(TextDirection::Ltr, Material::new().child(host)),
                    ),
                )
                .into_widget()
            };
            // The track, the three thumb shadows and the thumb; the transparent track outline,
            // Dart's second `..rrect()`, is not recorded.
            let sequence =
                |track: Color, thumb: Color| [&[c(track)][..], &M2_SHADOWS, &[c(thumb)]].concat();
            tester.pump_widget(root);
            assert_eq!(
                colors(&drawn(tester)),
                sequence(Colors::BLUE.shade500(), Colors::YELLOW.shade500())
            );

            tap(tester);
            tester.pump_and_settle();
            assert_eq!(
                colors(&drawn(tester)),
                sequence(Colors::GREEN.shade500(), Colors::RED.shade500())
            );
        });
    }

    #[test]
    fn material3_switch_list_tile_has_the_right_colors() {
        test_widgets(|tester| {
            let value = Rc::new(Cell::new(false));
            let host = stateful_tile(&value, |tile| {
                tile.active_color(Colors::RED.shade500())
                    .active_track_color(Colors::GREEN.shade500())
                    .inactive_thumb_color(Colors::YELLOW.shade500())
                    .inactive_track_color(Colors::BLUE.shade500())
            });
            let root = {
                let app = tester.cell.borrow();
                waft_widgets::MediaQuery::new(
                    waft_widgets::MediaQueryData::new().padding(EdgeInsets::all(8.0)),
                    crate::theme::Theme::new(
                        m3(&app),
                        Directionality::new(TextDirection::Ltr, Material::new().child(host)),
                    ),
                )
                .into_widget()
            };
            tester.pump_widget(root);
            assert_eq!(track(tester), Some(c(Colors::BLUE.shade500())));
            assert_eq!(thumb(tester), Some(c(Colors::YELLOW.shade500())));

            tap(tester);
            tester.pump_and_settle();
            assert_eq!(track(tester), Some(c(Colors::GREEN.shade500())));
            assert_eq!(thumb(tester), Some(c(Colors::RED.shade500())));
        });
    }

    #[test]
    fn switch_list_tile_adaptive_only_uses_material_switch() {
        test_widgets(|tester| {
            let value = Rc::new(Cell::new(false));
            for platform in [
                TargetPlatform::IOS,
                TargetPlatform::MacOS,
                TargetPlatform::Android,
                TargetPlatform::Fuchsia,
                TargetPlatform::Linux,
                TargetPlatform::Windows,
            ] {
                value.set(false);
                let host = {
                    let value = Rc::clone(&value);
                    StatefulBuilder::new(move |_app, _context, set_state| {
                        let value_now = Rc::clone(&value);
                        Material::new()
                            .child(Center::new().child(SwitchListTile::adaptive(
                                value.get(),
                                Some(Rc::new(move |app: &mut App, new_value: bool| {
                                    let value = Rc::clone(&value_now);
                                    set_state(app, &mut |_app| value.set(new_value));
                                })),
                            )))
                            .into_widget()
                    })
                    .into_widget()
                };
                pump_app(
                    tester,
                    move |app| ThemeData::builder().platform(platform).build(app),
                    |_app| host,
                );
                assert!(
                    tester
                        .find_by_type::<waft_cupertino::CupertinoSwitch>()
                        .is_empty()
                );
                assert_eq!(tester.find_by_type::<Switch>().len(), 1);
                assert!(!value.get(), "on {platform:?}");

                let at = rect_of::<SwitchListTile>(tester, 0).center();
                tester.tap(at, 1);
                assert!(value.get(), "on {platform:?}");
            }
        });
    }

    #[test]
    fn switch_list_tile_content_padding() {
        test_widgets(|tester| {
            let frame = |tester: &mut WidgetTester, text_direction: TextDirection| {
                let theme = m3(&tester.cell.borrow());
                tester.pump_widget(crate::list_tile::tests::bare(
                    theme,
                    Directionality::new(
                        text_direction,
                        Material::new().child(
                            Container::new()
                                .alignment(AlignmentGeometry::TOP_LEFT)
                                .child(
                                    SwitchListTile::new(true, ignore())
                                        .content_padding(EdgeInsetsDirectional::only(
                                            10.0, 30.0, 20.0, 40.0,
                                        ))
                                        .secondary(Text::new("L"))
                                        .title(Text::new("title")),
                                ),
                        ),
                    ),
                ));
            };

            frame(tester, TextDirection::Ltr);
            assert_eq!(text_rect(tester, "L").left, 10.0); // contentPadding.start = 10
            assert_eq!(rect_of::<Switch>(tester, 0).right, 780.0); // 800 - contentPadding.end

            frame(tester, TextDirection::Rtl);
            assert_eq!(rect_of::<Switch>(tester, 0).left, 20.0); // contentPadding.end = 20
            assert_eq!(text_rect(tester, "L").right, 790.0); // 800 - contentPadding.start
        });
    }

    #[test]
    fn switch_list_tile_forwards_states_controller_to_list_tile() {
        test_widgets(|tester| {
            let controller = WidgetStatesController::new(&mut tester.cell.borrow_mut(), None);
            pump_in_material(tester, m3, |_app| {
                ListView::new()
                    .children([SwitchListTile::new(true, ignore())
                        .title(Text::new("Switch"))
                        .states_controller(controller.clone())
                        .into_widget()])
                    .into_widget()
            });
            let forwarded = tester.widget(nth::<ListTile>(tester, 0), |tile: &ListTile| {
                tile.states_controller.clone()
            });
            assert_eq!(forwarded, Some(controller));
        });
    }

    #[test]
    fn switch_list_tile_can_autofocus_unless_disabled() {
        test_widgets(|tester| {
            let child_key = GlobalKey::new();
            for (on_changed, focused) in [(ignore(), true), (None, false)] {
                let key = child_key.clone();
                pump_in_material(tester, m3, move |_app| {
                    ListView::new()
                        .children([SwitchListTile::new(true, on_changed)
                            .title(Text::new("A").key(key.into()))
                            .autofocus(true)
                            .into_widget()])
                        .into_widget()
                });
                tester.pump();
                let node = focus_of(tester, &child_key);
                assert_eq!(has_primary_focus(tester, node), focused);
            }
        });
    }

    /// The `leading` and `trailing` of the tile's `ListTile`.
    fn slots(tester: &WidgetTester) -> (Option<WidgetRef>, Option<WidgetRef>) {
        tester.widget(nth::<ListTile>(tester, 0), |tile: &ListTile| {
            (tile.leading.clone(), tile.trailing.clone())
        })
    }

    /// Whether `slot` is an `ExcludeFocus` around a `Switch`.
    fn is_excluded_switch(slot: &Option<WidgetRef>) -> bool {
        slot.as_ref()
            .and_then(|slot| downcast_widget::<ExcludeFocus>(&**slot))
            .is_some_and(|exclude_focus| downcast_widget::<Switch>(&*exclude_focus.child).is_some())
    }

    fn is_icon(slot: &Option<WidgetRef>) -> bool {
        slot.as_ref()
            .is_some_and(|slot| downcast_widget::<Icon>(&**slot).is_some())
    }

    fn info_tile() -> SwitchListTile {
        SwitchListTile::new(true, None)
            .secondary(Icon::new(Some(Icons::info())))
            .title(Text::new("Title"))
    }

    #[test]
    fn switch_list_tile_control_affinity_test() {
        test_widgets(|tester| {
            pump_in_material(tester, m3, |_app| {
                info_tile()
                    .control_affinity(ListTileControlAffinity::Leading)
                    .into_widget()
            });
            // When controlAffinity is ListTileControlAffinity.leading, the position of
            // Switch is at leading edge and SwitchListTile.secondary at trailing edge.
            let (leading, trailing) = slots(tester);
            assert!(is_excluded_switch(&leading));
            assert!(is_icon(&trailing));
        });
    }

    #[test]
    fn switch_list_tile_control_affinity_default_value_test() {
        test_widgets(|tester| {
            pump_in_material(tester, m3, |_app| info_tile().into_widget());
            // By default, value of controlAffinity is ListTileControlAffinity.platform,
            // where the position of SwitchListTile.secondary is at leading edge and Switch
            // at trailing edge. This also covers test for ListTileControlAffinity.trailing.
            let (leading, trailing) = slots(tester);
            assert!(is_excluded_switch(&trailing));
            assert!(is_icon(&leading));
        });
    }

    #[test]
    fn switch_list_tile_respects_shape() {
        test_widgets(|tester| {
            pump_in_material(tester, m3, |_app| {
                SwitchListTile::new(true, None)
                    .title(Text::new("Title"))
                    .shape(ShapeBorderRef::new(right_rounded_border()))
                    .into_widget()
            });
            let shape: ShapeBorderRef = ShapeBorderRef::new(right_rounded_border());
            assert!(ink_well_custom_border(tester).as_deref() == Some(&*shape));
        });
    }

    #[test]
    fn switch_list_tile_respects_tile_color() {
        test_widgets(|tester| {
            let tile_color = Colors::RED.shade500();
            pump_wrapped(tester, |_app| {
                Center::new()
                    .child(
                        SwitchListTile::new(false, None)
                            .title(Text::new("Title"))
                            .tile_color(tile_color),
                    )
                    .into_widget()
            });
            assert!(ink_rect_colors(tester).contains(&tile_color.to_argb32()));
        });
    }

    #[test]
    fn switch_list_tile_respects_selected_tile_color() {
        test_widgets(|tester| {
            let selected_tile_color = Colors::GREEN.shade500();
            pump_wrapped(tester, |_app| {
                Center::new()
                    .child(
                        SwitchListTile::new(false, None)
                            .title(Text::new("Title"))
                            .selected(true)
                            .selected_tile_color(selected_tile_color),
                    )
                    .into_widget()
            });
            assert!(ink_rect_colors(tester).contains(&selected_tile_color.to_argb32()));
        });
    }

    #[test]
    fn switch_list_tile_selected_item_text_color() {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/pull/76909
            const ACTIVE_COLOR: Color = Color::new(0xff00ff00);
            // Dart pumps the same frame twice, `activeColor` given and `thumbColor` not.
            for _ in 0..2 {
                pump_app(
                    tester,
                    |app| {
                        ThemeData::builder()
                            .switch_theme(SwitchThemeData::new().thumb_color(
                                <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|_| None),
                            ))
                            .build(app)
                    },
                    |_app| {
                        Scaffold::new()
                            .body(
                                Center::new().child(
                                    SwitchListTile::new(true, ignore())
                                        .active_color(ACTIVE_COLOR)
                                        .selected(true)
                                        .title(Text::new("title")),
                                ),
                            )
                            .into_widget()
                    },
                );
                assert_eq!(tester.text_color("title"), ACTIVE_COLOR);
            }
        });
    }

    #[test]
    fn switch_list_tile_respects_visual_density() {
        test_widgets(|tester| {
            pump_wrapped(tester, |_app| {
                Center::new()
                    .child(
                        SwitchListTile::new(false, ignore())
                            .autofocus(true)
                            .visual_density(VisualDensity::STANDARD),
                    )
                    .into_widget()
            });
            tester.pump_and_settle();
            assert_eq!(
                rect_of::<SwitchListTile>(tester, 0).size(),
                Size::new(800.0, 56.0)
            );
        });
    }

    #[test]
    fn switch_list_tile_respects_focus_node() {
        test_widgets(|tester| {
            let child_key = GlobalKey::new();
            let key = child_key.clone();
            pump_wrapped(tester, move |_app| {
                Center::new()
                    .child(
                        SwitchListTile::new(false, ignore()).title(Text::new("A").key(key.into())),
                    )
                    .into_widget()
            });
            tester.pump();
            let tile_node = focus_of(tester, &child_key);
            tile_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump(); // Let the focus take effect.
            let node = focus_of(tester, &child_key);
            assert!(has_primary_focus(tester, node));
            assert!(has_primary_focus(tester, tile_node));
        });
    }

    #[test]
    fn switch_list_tile_on_focus_change_callback() {
        test_widgets(|tester| {
            check_on_focus_change(tester, |node, on_focus_change| {
                SwitchListTile::new(true, ignore())
                    .focus_node(node)
                    .on_focus_change(on_focus_change)
                    .into_widget()
            });
        });
    }

    #[test]
    fn switch_list_tile_adaptive_on_focus_change_callback() {
        test_widgets(|tester| {
            check_on_focus_change(tester, |node, on_focus_change| {
                SwitchListTile::adaptive(true, ignore())
                    .focus_node(node)
                    .on_focus_change(on_focus_change)
                    .into_widget()
            });
        });
    }

    #[test]
    fn switch_list_tile_respects_enable_feedback() {
        test_widgets(|tester| {
            for enable_feedback in [false, true] {
                pump_wrapped(tester, |_app| {
                    Center::new()
                        .child(
                            SwitchListTile::new(false, ignore()).enable_feedback(enable_feedback),
                        )
                        .into_widget()
                });
                assert_eq!(ink_well_enable_feedback(tester), enable_feedback);
            }
        });
    }

    #[test]
    fn switch_list_tile_respects_hover_color() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let orange = Colors::ORANGE.shade500();
            let tile_key = key.clone();
            pump_wrapped(tester, move |_app| {
                Center::new()
                    .child(
                        SizedBox::new().width(500.0).height(100.0).child(
                            Material::new().color(Colors::WHITE).child(
                                SwitchListTile::new(false, ignore())
                                    .key(tile_key.into())
                                    .hover_color(orange)
                                    .title(Text::new("A")),
                            ),
                        ),
                    )
                    .into_widget()
            });

            // Start hovering
            let mut gesture = TestPointer::mouse(1, 0);
            let center = tester.center_of(&key);
            tester.add_pointer(&mut gesture, center);
            tester.pump();
            tester.pump_and_settle();
            // Dart's `Material.of(tester.element(find.byKey(key)))`: the white material, inside
            // the one `wrap` gives.
            let materials = tester.all::<crate::material::RenderInkFeatures>();
            let mut canvas = waft_embedder::Canvas::new();
            materials[1].paint_ink_features(
                &mut tester.cell.borrow_mut(),
                &mut canvas,
                Offset::ZERO,
            );
            assert!(crate::test_support::Drawn::draws(&canvas.build()).contains(
                &crate::test_support::Drawn::Rect(
                    orange.to_argb32(),
                    waft_embedder::Rect::from_ltrb(0.0, 0.0, 500.0, 100.0)
                )
            ));
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn material2_switch_list_tile_respects_thumb_color_in_active_enabled_states() {
        test_widgets(|tester| {
            let thumb_color = state_colors(STATE_COLORS);
            for ((enabled, selected), expected) in STATE_FRAMES.into_iter().zip(STATE_COLORS) {
                let thumb_color = Rc::clone(&thumb_color);
                pump_in_material(tester, m2, move |_app| {
                    SwitchListTile::new(selected, if enabled { ignore() } else { None })
                        .thumb_color(thumb_color)
                        .into_widget()
                });
                tester.pump_and_settle();
                assert_eq!(thumb(tester), Some(c(expected)), "{enabled} {selected}");
            }
        });
    }

    #[test]
    fn material3_switch_list_tile_respects_thumb_color_in_active_enabled_states() {
        test_widgets(|tester| {
            let thumb_color = state_colors(STATE_COLORS);
            for ((enabled, selected), expected) in STATE_FRAMES.into_iter().zip(STATE_COLORS) {
                let thumb_color = Rc::clone(&thumb_color);
                pump_in_material(tester, m3, move |_app| {
                    SwitchListTile::new(selected, if enabled { ignore() } else { None })
                        .thumb_color(thumb_color)
                        .into_widget()
                });
                tester.pump_and_settle();
                assert_eq!(thumb(tester), Some(c(expected)), "{enabled} {selected}");
            }
        });
    }

    #[test]
    fn material2_switch_list_tile_respects_thumb_color_in_hovered_pressed_states() {
        test_widgets(|tester| {
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const HOVERED_THUMB_COLOR: Color = Color::new(0xFF4CAF50);
            const PRESSED_THUMB_COLOR: Color = Color::new(0xFFF44336);
            let thumb_color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
                Some(if states.contains(&WidgetState::Pressed) {
                    PRESSED_THUMB_COLOR
                } else if states.contains(&WidgetState::Hovered) {
                    HOVERED_THUMB_COLOR
                } else {
                    Colors::TRANSPARENT
                })
            });
            pump_in_material(tester, m2, |_app| {
                SwitchListTile::new(false, ignore())
                    .thumb_color(thumb_color)
                    .into_widget()
            });
            tester.pump_and_settle();

            // Start hovering
            let gesture = hover(tester);
            tester.pump_and_settle();
            assert_eq!(thumb(tester), Some(c(HOVERED_THUMB_COLOR)));

            // On pressed state
            let touch = press(tester);
            tester.pump_and_settle();
            assert_eq!(thumb(tester), Some(c(PRESSED_THUMB_COLOR)));
            tester.up(&touch);
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn material3_switch_list_tile_respects_thumb_color_in_hovered_pressed_states() {
        test_widgets(|tester| {
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const HOVERED_THUMB_COLOR: Color = Color::new(0xFF4CAF50);
            const PRESSED_THUMB_COLOR: Color = Color::new(0xFFF44336);
            let thumb_color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
                Some(if states.contains(&WidgetState::Pressed) {
                    PRESSED_THUMB_COLOR
                } else if states.contains(&WidgetState::Hovered) {
                    HOVERED_THUMB_COLOR
                } else {
                    Colors::TRANSPARENT
                })
            });
            pump_in_material(tester, m3, |_app| {
                SwitchListTile::new(false, ignore())
                    .thumb_color(thumb_color)
                    .into_widget()
            });
            tester.pump_and_settle();

            // Start hovering
            let gesture = hover(tester);
            tester.pump_and_settle();
            assert_eq!(thumb(tester), Some(c(HOVERED_THUMB_COLOR)));

            // On pressed state
            let touch = press(tester);
            tester.pump_and_settle();
            assert_eq!(thumb(tester), Some(c(PRESSED_THUMB_COLOR)));
            tester.up(&touch);
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn switch_list_tile_respects_track_color_in_active_enabled_states() {
        test_widgets(|tester| {
            let track_color = state_colors(STATE_COLORS);
            for (index, ((enabled, selected), expected)) in
                STATE_FRAMES.into_iter().zip(STATE_COLORS).enumerate()
            {
                let track_color = Rc::clone(&track_color);
                pump_wrapped(tester, move |_app| {
                    SwitchListTile::new(selected, if enabled { ignore() } else { None })
                        .track_color(track_color)
                        .into_widget()
                });
                if index > 0 {
                    tester.pump_and_settle();
                }
                assert_eq!(track(tester), Some(c(expected)), "{enabled} {selected}");
            }
        });
    }

    #[test]
    fn switch_list_tile_respects_track_color_in_hovered_states() {
        test_widgets(|tester| {
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const HOVERED_TRACK_COLOR: Color = Color::new(0xFF4CAF50);
            pump_wrapped(tester, |_app| {
                SwitchListTile::new(false, ignore())
                    .track_color(hovered_colors(HOVERED_TRACK_COLOR))
                    .into_widget()
            });
            tester.pump_and_settle();

            // Start hovering
            let gesture = hover(tester);
            tester.pump_and_settle();
            assert_eq!(track(tester), Some(c(HOVERED_TRACK_COLOR)));
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn switch_list_tile_respects_thumb_icon_m3() {
        test_widgets(|tester| {
            crate::icons::install_material_icon_font(&mut tester.cell.borrow_mut());
            let active_icon = Rc::new(Icon::new(Some(Icons::check())));
            let inactive_icon = Rc::new(Icon::new(Some(Icons::close())));
            let thumb_icon = |active_icon: Option<Rc<Icon>>, inactive_icon: Option<Rc<Icon>>| {
                <dyn WidgetStateProperty<Option<Rc<Icon>>>>::resolve_with(move |states| {
                    if states.contains(&WidgetState::Selected) {
                        return active_icon.clone();
                    }
                    inactive_icon.clone()
                })
            };
            let frame = |tester: &mut WidgetTester,
                         active: bool,
                         active_icon: Option<Rc<Icon>>,
                         inactive_icon: Option<Rc<Icon>>| {
                let thumb_icon = thumb_icon(active_icon, inactive_icon);
                pump_app(tester, m3, |app| {
                    let theme = m3(app);
                    crate::list_tile::tests::bare(
                        theme,
                        Material::new()
                            .child(SwitchListTile::new(active, ignore()).thumb_icon(thumb_icon)),
                    )
                });
                tester.pump_and_settle();
                let resolved = |states: &WidgetStates| {
                    tester.widget(nth::<Switch>(tester, 0), |switch: &Switch| {
                        switch
                            .thumb_icon
                            .as_ref()
                            .and_then(|thumb_icon| thumb_icon.resolve(states))
                    })
                };
                let selected: WidgetStates = [WidgetState::Selected].into_iter().collect();
                let glyphs: Vec<(f64, f64)> = drawn(tester)
                    .into_iter()
                    .filter_map(|drawn| match drawn {
                        Drawn::Glyphs(left, top) => Some((left, top)),
                        _ => None,
                    })
                    .collect();
                (resolved(&selected), resolved(&WidgetStates::new()), glyphs)
            };
            // The glyph's ink is inside the 16x16 icon square Dart paints its paragraph at.
            let inside = |glyphs: &[(f64, f64)], left: f64, top: f64| matches!(glyphs, [(x, y)] if *x >= left && *x < left + 16.0 && *y >= top && *y < top + 16.0);
            let same = |icon: Option<Rc<Icon>>, expected: &Rc<Icon>| {
                icon.is_some_and(|icon| Rc::ptr_eq(&icon, expected))
            };

            // active icon shows when switch is on.
            let (selected, _, glyphs) = frame(tester, true, Some(Rc::clone(&active_icon)), None);
            assert!(same(selected, &active_icon));
            assert!(inside(&glyphs, 32.0, 12.0), "{glyphs:?}");

            // inactive icon shows when switch is off.
            let (_, unselected, glyphs) =
                frame(tester, false, None, Some(Rc::clone(&inactive_icon)));
            assert!(same(unselected, &inactive_icon));
            assert!(inside(&glyphs, 12.0, 12.0), "{glyphs:?}");

            // active icon doesn't show when switch is off.
            let (selected, _, glyphs) = frame(tester, false, Some(Rc::clone(&active_icon)), None);
            assert!(same(selected, &active_icon));
            assert_eq!(glyphs, []);

            // inactive icon doesn't show when switch is on.
            let (_, unselected, glyphs) =
                frame(tester, true, None, Some(Rc::clone(&inactive_icon)));
            assert!(same(unselected, &inactive_icon));
            assert_eq!(glyphs, []);

            // without icon
            let (_, _, glyphs) = frame(tester, false, None, None);
            assert_eq!(glyphs, []);
        });
    }

    fn check_material_tap_target_size(
        tester: &mut WidgetTester,
        theme: fn(&App) -> ThemeData,
        width: f64,
    ) {
        for (size, height) in [
            (MaterialTapTargetSize::Padded, 48.0),
            (MaterialTapTargetSize::ShrinkWrap, 40.0),
        ] {
            pump_in_material(tester, theme, move |_app| {
                SwitchListTile::new(false, ignore())
                    .material_tap_target_size(size)
                    .into_widget()
            });
            let given = tester.widget(nth::<Switch>(tester, 0), |switch: &Switch| {
                switch.material_tap_target_size
            });
            assert_eq!(given, Some(size));
            assert_eq!(
                rect_of::<Switch>(tester, 0).size(),
                Size::new(width, height)
            );
        }
    }

    #[test]
    fn material2_switch_list_tile_respects_material_tap_target_size() {
        test_widgets(|tester| check_material_tap_target_size(tester, m2, 59.0));
    }

    #[test]
    fn material3_switch_list_tile_respects_material_tap_target_size() {
        test_widgets(|tester| check_material_tap_target_size(tester, m3, 60.0));
    }

    #[test]
    fn material2_switch_list_tile_adaptive_respects_apply_cupertino_theme() {
        test_widgets(|tester| {
            for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
                for (apply_cupertino_theme, expected) in
                    [(true, 0xFF2196F3_u32), (false, 0xFF34C759)]
                {
                    pump_in_material(
                        tester,
                        move |app| {
                            ThemeData::builder()
                                .use_material3(false)
                                .platform(platform)
                                .build(app)
                        },
                        move |_app| {
                            SwitchListTile::adaptive(true, ignore())
                                .apply_cupertino_theme(apply_cupertino_theme)
                                .into_widget()
                        },
                    );
                    tester.pump_and_settle();
                    assert_eq!(tester.find_by_type::<Switch>().len(), 1);
                    assert!(
                        drawn(tester)
                            .iter()
                            .any(|drawn| drawn.color() == Some(expected)),
                        "{platform:?} {apply_cupertino_theme}: {:?}",
                        drawn(tester)
                    );
                }
            }
        });
    }

    #[test]
    fn material3_switch_list_tile_adaptive_respects_apply_cupertino_theme() {
        test_widgets(|tester| {
            for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
                for (apply_cupertino_theme, expected) in
                    [(true, 0xFF6750A4_u32), (false, 0xFF34C759)]
                {
                    pump_in_material(
                        tester,
                        move |app| ThemeData::builder().platform(platform).build(app),
                        move |_app| {
                            SwitchListTile::adaptive(true, ignore())
                                .apply_cupertino_theme(apply_cupertino_theme)
                                .into_widget()
                        },
                    );
                    tester.pump_and_settle();
                    assert_eq!(tester.find_by_type::<Switch>().len(), 1);
                    assert!(
                        drawn(tester)
                            .iter()
                            .any(|drawn| drawn.color() == Some(expected)),
                        "{platform:?} {apply_cupertino_theme}: {:?}",
                        drawn(tester)
                    );
                }
            }
        });
    }

    #[test]
    fn switch_list_tile_passes_the_value_of_drag_start_behavior_to_switch() {
        test_widgets(|tester| {
            for drag_start_behavior in [DragStartBehavior::Start, DragStartBehavior::Down] {
                pump_wrapped(tester, move |_app| {
                    SwitchListTile::new(false, ignore())
                        .drag_start_behavior(drag_start_behavior)
                        .into_widget()
                });
                let given = tester.widget(nth::<Switch>(tester, 0), |switch: &Switch| {
                    switch.drag_start_behavior
                });
                assert_eq!(given, drag_start_behavior);
            }
        });
    }

    #[test]
    fn switch_on_switch_list_tile_changes_mouse_cursor_when_hovered() {
        test_widgets(|tester| {
            // Test SwitchListTile.adaptive() constructor
            pump_wrapped(tester, |_app| {
                SwitchListTile::adaptive(false, ignore())
                    .mouse_cursor(MouseCursorRef::from(SystemMouseCursors::TEXT))
                    .into_widget()
            });
            let mut gesture = TestPointer::mouse(1, 0);
            let center = rect_of::<Switch>(tester, 0).center();
            tester.add_pointer(&mut gesture, center);
            tester.pump();
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::TEXT.into());

            // Test SwitchListTile() constructor
            pump_wrapped(tester, |_app| {
                SwitchListTile::new(false, ignore())
                    .mouse_cursor(MouseCursorRef::from(SystemMouseCursors::FORBIDDEN))
                    .into_widget()
            });
            let center = rect_of::<Switch>(tester, 0).center();
            tester.move_to(&mut gesture, center, false);
            assert_eq!(
                tester.active_cursor(1),
                SystemMouseCursors::FORBIDDEN.into()
            );

            // Test default cursor
            pump_wrapped(tester, |_app| {
                SwitchListTile::new(false, ignore()).into_widget()
            });
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::CLICK.into());

            // Test default cursor when disabled
            pump_wrapped(tester, |_app| {
                SwitchListTile::new(false, None).into_widget()
            });
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::BASIC.into());
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn switch_with_splash_radius_set() {
        test_widgets(|tester| {
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const SPLASH_RADIUS: f64 = 35.0;
            pump_wrapped(tester, |_app| {
                SwitchListTile::new(false, ignore())
                    .splash_radius(SPLASH_RADIUS)
                    .into_widget()
            });
            tester.pump_and_settle();
            let gesture = hover(tester);
            tester.pump_and_settle();
            assert!(!circles(&drawn(tester), SPLASH_RADIUS).is_empty());
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn the_overlay_color_for_the_thumb_of_the_switch_resolves_in_active_pressed_hovered_states() {
        test_widgets(|tester| {
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const ACTIVE_THUMB_COLOR: Color = Color::new(0xFF000000);
            const INACTIVE_THUMB_COLOR: Color = Color::new(0xFF000010);
            const ACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000001);
            const INACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000002);
            const HOVER_OVERLAY_COLOR: Color = Color::new(0xFF000003);
            const HOVER_COLOR: Color = Color::new(0xFF000005);
            let frame = |tester: &mut WidgetTester, active: bool, use_overlay: bool| {
                pump_app(tester, m3, move |_app| {
                    let mut tile = SwitchListTile::new(active, ignore())
                        .thumb_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                            |states| {
                                Some(if states.contains(&WidgetState::Selected) {
                                    ACTIVE_THUMB_COLOR
                                } else {
                                    INACTIVE_THUMB_COLOR
                                })
                            },
                        ))
                        .hover_color(HOVER_COLOR);
                    if use_overlay {
                        tile = tile.overlay_color(
                            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
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
                            }),
                        );
                    }
                    Scaffold::new().body(tile).into_widget()
                });
            };
            for (active, use_overlay, expected) in [
                (
                    false,
                    false,
                    INACTIVE_THUMB_COLOR.with_alpha(K_RADIAL_REACTION_ALPHA),
                ),
                (
                    true,
                    false,
                    ACTIVE_THUMB_COLOR.with_alpha(K_RADIAL_REACTION_ALPHA),
                ),
                (false, true, INACTIVE_PRESSED_OVERLAY_COLOR),
                (true, true, ACTIVE_PRESSED_OVERLAY_COLOR),
            ] {
                frame(tester, active, use_overlay);
                let touch = press(tester);
                tester.pump_and_settle();
                assert_eq!(
                    circles(&drawn(tester), crate::constants::K_RADIAL_REACTION_RADIUS),
                    [c(expected)],
                    "active {active}, overlay {use_overlay}"
                );
                tester.up(&touch);
                tester.pump_and_settle();
            }

            frame(tester, false, true);
            tester.pump_and_settle();

            // Start hovering
            let gesture = hover(tester);
            tester.pump_and_settle();
            assert_eq!(
                circles(&drawn(tester), crate::constants::K_RADIAL_REACTION_RADIUS),
                [c(HOVER_OVERLAY_COLOR)]
            );
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn switch_list_tile_respects_track_outline_color_in_active_enabled_states() {
        test_widgets(|tester| {
            let track_outline_color = state_colors(STATE_COLORS);
            for ((enabled, selected), expected) in STATE_FRAMES.into_iter().zip(STATE_COLORS) {
                let track_outline_color = Rc::clone(&track_outline_color);
                pump_wrapped(tester, move |_app| {
                    SwitchListTile::new(selected, if enabled { ignore() } else { None })
                        .track_outline_color(track_outline_color)
                        .into_widget()
                });
                tester.pump_and_settle();
                assert!(matches!(drawn(tester).first(), Some(Drawn::Fill(..))));
                assert_eq!(strokes(tester), [c(expected)], "{enabled} {selected}");
            }
        });
    }

    #[test]
    fn switch_list_tile_respects_track_outline_color_in_hovered_state() {
        test_widgets(|tester| {
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const HOVERED_TRACK_COLOR: Color = Color::new(0xFF4CAF50);
            pump_wrapped(tester, |_app| {
                SwitchListTile::new(false, ignore())
                    .track_outline_color(hovered_colors(HOVERED_TRACK_COLOR))
                    .into_widget()
            });
            tester.pump_and_settle();

            // Start hovering
            let gesture = hover(tester);
            tester.pump_and_settle();
            assert_eq!(strokes(tester), [c(HOVERED_TRACK_COLOR)]);
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    #[test]
    fn switch_list_tile_control_widget_should_not_request_focus_on_traversal() {
        test_widgets(|tester| {
            check_control_is_skipped_by_traversal(tester, |title| {
                let mut tile = SwitchListTile::new(true, ignore());
                tile.title = Some(title);
                tile.into_widget()
            });
        });
    }

    #[test]
    fn switch_list_tile_uses_list_tile_theme_control_affinity() {
        test_widgets(|tester| {
            for (control_affinity, expected) in [
                (ListTileControlAffinity::Leading, Offset::new(92.0, 16.0)),
                (ListTileControlAffinity::Trailing, Offset::new(16.0, 16.0)),
                (ListTileControlAffinity::Platform, Offset::new(16.0, 16.0)),
            ] {
                pump_in_material(tester, m3, |_app| {
                    ListTileTheme::new(
                        SwitchListTile::new(true, ignore()).title(Text::new("SwitchListTile")),
                    )
                    .data(ListTileThemeData::new().control_affinity(control_affinity))
                    .into_widget()
                });
                assert_eq!(text_rect(tester, "SwitchListTile").top_left(), expected);
            }
        });
    }

    fn three_line_tile(is_three_line: Option<bool>, subtitle: &str) -> WidgetRef {
        three_line_tile_of(SwitchListTile::new(false, None), is_three_line, subtitle)
    }

    fn adaptive_three_line_tile(is_three_line: Option<bool>, subtitle: &str) -> WidgetRef {
        three_line_tile_of(
            SwitchListTile::adaptive(false, None),
            is_three_line,
            subtitle,
        )
    }

    fn three_line_tile_of(
        tile: SwitchListTile,
        is_three_line: Option<bool>,
        subtitle: &str,
    ) -> WidgetRef {
        let mut tile = tile.title(Text::new("A")).subtitle(Text::new(subtitle));
        tile.is_three_line = is_three_line;
        tile.into_widget()
    }

    fn check_is_three_line(
        tester: &mut WidgetTester,
        platform: TargetPlatform,
        tile: fn(Option<bool>, &str) -> WidgetRef,
    ) {
        for (frame, three_line) in IS_THREE_LINE_FRAMES {
            assert_eq!(
                is_three_line_rects::<Switch, SwitchListTile>(tester, platform, frame, tile),
                expected_three_line_rects(
                    three_line,
                    800.0 - 60.0 - 24.0,
                    Size::new(60.0, 40.0),
                    [130.0, 300.0 + 16.0]
                ),
                "{frame:?}"
            );
        }
    }

    #[test]
    fn switch_list_tile_is_three_line() {
        test_widgets(|tester| {
            check_is_three_line(tester, TargetPlatform::Android, three_line_tile)
        });
    }

    #[test]
    fn switch_list_tile_adaptive_is_three_line() {
        test_widgets(|tester| {
            check_is_three_line(tester, TargetPlatform::IOS, adaptive_three_line_tile)
        });
    }

    fn check_active_thumb_color(
        tester: &mut WidgetTester,
        make: fn(bool, Option<ValueChanged<bool>>) -> SwitchListTile,
    ) {
        let root = {
            let app = tester.cell.borrow();
            crate::list_tile::tests::bare(
                m3(&app),
                Material::new().child(
                    make(true, ignore())
                        .selected(true)
                        .active_color(Colors::RED.shade500())
                        .active_thumb_color(Colors::YELLOW.shade500())
                        .active_track_color(Colors::GREEN.shade500())
                        .title(Text::new("title")),
                ),
            )
        };
        tester.pump_widget(root);
        assert_eq!(track(tester), Some(c(Colors::GREEN.shade500())));
        assert_eq!(thumb(tester), Some(c(Colors::YELLOW.shade500())));
        assert_eq!(tester.text_color("title"), Colors::YELLOW.shade500());
    }

    #[test]
    fn material3_switch_list_tile_active_thumb_color() {
        test_widgets(|tester| check_active_thumb_color(tester, SwitchListTile::new));
    }

    #[test]
    fn material3_switch_list_tile_adaptive_active_thumb_color() {
        test_widgets(|tester| check_active_thumb_color(tester, SwitchListTile::adaptive));
    }

    #[test]
    fn switch_list_tile_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                Scaffold::new()
                    .body(
                        Center::new()
                            .child(SizedBox::shrink().child(SwitchListTile::new(true, ignore()))),
                    )
                    .into_widget()
            });
            assert_eq!(rect_of::<SwitchListTile>(tester, 0).size(), Size::ZERO);
        });
    }

    fn leading_title_tile() -> SwitchListTile {
        SwitchListTile::new(true, ignore())
            .control_affinity(ListTileControlAffinity::Leading)
            .title(Text::new("title"))
    }

    #[test]
    fn switch_list_tile_horizontal_title_gap_0() {
        test_widgets(|tester| {
            check_horizontal_title_gap_0(tester, [76.0, 724.0], |gap| {
                let mut tile = leading_title_tile();
                tile.horizontal_title_gap = gap;
                tile.into_widget()
            });
        });
    }

    #[test]
    fn switch_list_tile_default_horizontal_title_gap_and_min_leading_width() {
        test_widgets(|tester| {
            check_default_horizontal_title_gap(tester, [92.0, 708.0], || {
                leading_title_tile().into_widget()
            });
        });
    }

    #[test]
    fn switch_list_tile_horizontal_title_gap_with_visual_density() {
        test_widgets(|tester| {
            check_horizontal_title_gap_with_visual_density(tester, 78.0, || {
                leading_title_tile()
                    .visual_density(VisualDensity::new(VisualDensity::MINIMUM_DENSITY, 0.0))
                    .horizontal_title_gap(10.0)
                    .into_widget()
            });
        });
    }

    fn min_vertical_padding_tile(padding: Option<f64>) -> WidgetRef {
        let mut tile = SwitchListTile::new(true, ignore()).title(Text::new("title"));
        tile.min_vertical_padding = padding;
        tile.into_widget()
    }

    #[test]
    fn switch_list_tile_min_vertical_padding_80_material_3() {
        test_widgets(|tester| {
            // 80 + 80 + 24(Title) = 184
            check_min_vertical_padding_80(tester, m3, 184.0, min_vertical_padding_tile);
        });
    }

    #[test]
    fn switch_list_tile_min_vertical_padding_80_material_2() {
        test_widgets(|tester| {
            // 80 + 80 + 16(Title) = 176
            check_min_vertical_padding_80(tester, m2, 176.0, min_vertical_padding_tile);
        });
    }

    #[test]
    fn switch_list_tile_min_leading_width_60() {
        test_widgets(|tester| {
            check_min_leading_width_60(tester, |width| {
                let mut tile = leading_title_tile();
                tile.min_leading_width = width;
                tile.into_widget()
            });
        });
    }

    #[test]
    fn switch_list_tile_min_tile_height() {
        test_widgets(|tester| {
            check_min_tile_height(tester, |min_tile_height| {
                let mut tile = SwitchListTile::new(true, ignore());
                tile.min_tile_height = min_tile_height;
                tile.into_widget()
            });
        });
    }
}
