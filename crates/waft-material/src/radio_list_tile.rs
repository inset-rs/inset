//! Flutter counterpart: material_ui's `radio_list_tile.dart`.
//!
//! [`RadioListTileState`] is a [`RadioClient`] of the [`RadioGroup`] above it, so the group's
//! arrow keys move the tile's focus; the [`Radio`] it builds is excluded from focus and reads
//! the group through `_RadioRegistry`, an arena object the state creates on first use.
#![allow(deprecated)]

use std::fmt::{self, Debug};

use waft_embedder::Color;
use waft_foundation::{App, Handle, OptionRetained, Ref, ValueChanged};
use waft_gestures::GestureTapCallback;
use waft_painting::{AnyBorderSide, EdgeInsetsGeometry, ShapeBorderRef};
use waft_services::MouseCursorRef;
use waft_widgets::{
    AnyFocusNode, AnyRadioClient, AnyRadioGroupRegistry, BuildContext, ExcludeFocus, FocusNode,
    FocusNodeLeaf, FocusNodeRef, IntoWidget, KeyRef, MergeSemantics, RadioClient, RadioClientData,
    RadioGroup, RadioGroupRegistry, State, StateData, StatefulWidget, Transform, WidgetRef,
    WidgetState, WidgetStatePropertyRef, WidgetStates, WidgetStatesController,
};

use crate::list_tile::{ListTile, ListTileControlAffinity, ListTileTitleAlignment};
use crate::list_tile_theme::ListTileTheme;
use crate::radio::Radio;
use crate::radio_theme::RadioTheme;
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};

/// Dart's `_RadioType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RadioType {
    Material,
    Adaptive,
}

/// A [`ListTile`] with a [`Radio`]. In other words, a radio button with a label.
///
/// The entire list tile is interactive: tapping anywhere in the tile selects
/// the radio button.
///
/// This widget typically has a [`RadioGroup`] ancestor, which takes in a
/// [`RadioGroup::group_value`], and the [`RadioListTile`] under it with matching
/// [`value`](Self::value) will be selected.
///
/// The [`title`](Self::title), [`subtitle`](Self::subtitle),
/// [`is_three_line`](Self::is_three_line), and [`dense`](Self::dense) properties are like
/// those of the same name on [`ListTile`].
///
/// The [`selected`](Self::selected) property on this widget is similar to the
/// [`ListTile::selected`] property. The [`fill_color`](Self::fill_color) in the selected state
/// is used for the selected item's text color. If it is null, the
/// [`active_color`](Self::active_color) is used.
///
/// This widget does not coordinate the [`selected`](Self::selected) state and the
/// [`checked`](Self::checked) state; to have the list tile appear selected when the
/// radio button is the selected radio button, set [`selected`](Self::selected) to true
/// when [`value`](Self::value) matches [`RadioGroup::group_value`].
///
/// The radio button is shown on the left by default in left-to-right languages
/// (i.e. the leading edge). This can be changed using
/// [`control_affinity`](Self::control_affinity). The [`secondary`](Self::secondary) widget is
/// placed on the opposite side. This maps to the [`ListTile::leading`] and
/// [`ListTile::trailing`] properties of [`ListTile`].
///
/// This widget requires a `Material` widget ancestor in the tree to paint
/// itself on, which is typically provided by the app's `Scaffold`.
/// The [`tile_color`](Self::tile_color), and [`selected_tile_color`](Self::selected_tile_color)
/// are not painted by the [`RadioListTile`] itself but by the `Material` widget ancestor. In
/// this case, one can wrap a `Material` widget around the [`RadioListTile`].
///
/// ## Performance considerations when wrapping [`RadioListTile`] with `Material`
///
/// Wrapping a large number of [`RadioListTile`]s individually with `Material`s
/// is expensive. Consider only wrapping the [`RadioListTile`]s that require it
/// or include a common `Material` ancestor where possible.
///
/// ## RadioListTile isn't exactly what I want
///
/// If the way RadioListTile pads and positions its elements isn't quite what
/// you're looking for, you can create custom labeled radio widgets by
/// combining [`Radio`] with other widgets, such as `Text`, `Padding` and
/// `InkWell`.
///
/// See also:
///
///  * [`ListTileTheme`], which can be used to affect the style of list tiles,
///    including radio list tiles.
///  * `CheckboxListTile`, a similar widget for checkboxes.
///  * `SwitchListTile`, a similar widget for switches.
///  * [`ListTile`] and [`Radio`], the widgets from which this widget is made.
pub struct RadioListTile<T: Clone + PartialEq + 'static> {
    pub key: Option<KeyRef>,
    /// The value represented by this radio button.
    pub value: T,
    /// The currently selected value for this group of radio buttons.
    ///
    /// This radio button is considered selected if its [`value`](Self::value) matches the
    /// [`group_value`](Self::group_value).
    ///
    /// leave this unassigned or null if building this widget under [`RadioGroup`].
    ///
    /// This is deprecated, use a [`RadioGroup`] ancestor to manage group value instead.
    pub group_value: Option<T>,
    /// Called when the user selects this radio button.
    ///
    /// The radio button passes [`value`](Self::value) as a parameter to this callback. The
    /// radio button does not actually change state until the parent widget rebuilds the
    /// radio tile with the new [`group_value`](Self::group_value).
    ///
    /// If null, the radio button will be displayed as disabled.
    ///
    /// The provided callback will not be invoked if this radio button is already
    /// selected.
    ///
    /// The callback provided to [`on_changed`](Self::on_changed) should update the state of
    /// the parent `StatefulWidget` using the `State::set_state` method, so that the parent
    /// gets rebuilt.
    ///
    /// This is deprecated, use [`RadioGroup`] to handle value change instead.
    pub on_changed: Option<ValueChanged<Option<T>>>,
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
    /// If null, then the value of `RadioThemeData::mouse_cursor` is used.
    /// If that is also null, then `WidgetStateMouseCursor::clickable` is used.
    pub mouse_cursor: Option<MouseCursorRef>,
    /// Set to true if this radio list tile is allowed to be returned to an
    /// indeterminate state by selecting it again when selected.
    ///
    /// To indicate returning to an indeterminate state, [`on_changed`](Self::on_changed) will
    /// be called with null.
    ///
    /// If true, `RadioGroup::on_changed` is called with [`value`](Self::value) when selected
    /// while [`RadioGroup::group_value`] != [`value`](Self::value), and with null when
    /// selected again while [`RadioGroup::group_value`] == [`value`](Self::value).
    ///
    /// If false, `RadioGroup::on_changed` will be called with [`value`](Self::value) when it
    /// is selected while [`group_value`](Self::group_value) != [`value`](Self::value), and
    /// only by selecting another radio button in the group (i.e. changing the value of
    /// [`RadioGroup::group_value`]) can this radio list tile be unselected.
    ///
    /// The default is false.
    pub toggleable: bool,
    /// The color to use when this radio button is selected.
    ///
    /// Defaults to `ColorScheme::secondary` of the current [`Theme`].
    pub active_color: Option<Color>,
    /// The color that fills the radio button.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then the value of [`active_color`](Self::active_color) is used in the selected
    /// state. If that is also null, then the value of `RadioThemeData::fill_color` is used.
    /// If that is also null, then the default value is used.
    pub fill_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// Configures the minimum size of the tap target.
    ///
    /// Defaults to [`MaterialTapTargetSize::ShrinkWrap`].
    pub material_tap_target_size: Option<MaterialTapTargetSize>,
    /// The color for the radio's `Material` when a pointer is hovering over it.
    pub hover_color: Option<Color>,
    /// The color for the radio's `Material`.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Pressed`].
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///
    /// If null, then the value of [`active_color`](Self::active_color) with alpha
    /// `K_RADIAL_REACTION_ALPHA` and [`hover_color`](Self::hover_color) is used in the pressed
    /// and hovered state. If that is also null, the value of `SwitchThemeData::overlay_color`
    /// is used. If that is also null, then the default value is used in the pressed and
    /// hovered state.
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// The splash radius of the circular `Material` ink response.
    ///
    /// If null, then the value of `RadioThemeData::splash_radius` is used. If that
    /// is also null, then `K_RADIAL_REACTION_RADIUS` is used.
    pub splash_radius: Option<f64>,
    /// The primary content of the list tile.
    ///
    /// Typically a `Text` widget.
    pub title: Option<WidgetRef>,
    /// Additional content displayed below the title.
    ///
    /// Typically a `Text` widget.
    pub subtitle: Option<WidgetRef>,
    /// A widget to display on the opposite side of the tile from the radio button.
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
    /// Whether to render icons and text in the [`active_color`](Self::active_color).
    ///
    /// No effort is made to automatically coordinate the [`selected`](Self::selected) state
    /// and the [`checked`](Self::checked) state. To have the list tile appear selected when
    /// the radio button is the selected radio button, set [`selected`](Self::selected) to true
    /// when [`value`](Self::value) matches [`RadioGroup::group_value`].
    ///
    /// Normally, this property is left to its default value, false.
    pub selected: bool,
    /// Where to place the control relative to the text.
    pub control_affinity: Option<ListTileControlAffinity>,
    /// True if this widget will be selected as the initial focus when no other
    /// node in its scope is currently focused.
    pub autofocus: bool,
    /// Defines the insets surrounding the contents of the tile.
    ///
    /// Insets the [`Radio`], [`title`](Self::title), [`subtitle`](Self::subtitle), and
    /// [`secondary`](Self::secondary) widgets in [`RadioListTile`].
    ///
    /// When null, `EdgeInsets.symmetric(horizontal: 16.0)` is used.
    pub content_padding: Option<EdgeInsetsGeometry>,
    /// If specified, [`shape`](Self::shape) defines the shape of the [`RadioListTile`]'s
    /// `InkWell` border.
    pub shape: Option<ShapeBorderRef>,
    /// If specified, defines the background color for `RadioListTile` when
    /// [`RadioListTile::selected`] is false.
    pub tile_color: Option<Color>,
    /// If non-null, defines the background color when [`RadioListTile::selected`] is true.
    pub selected_tile_color: Option<Color>,
    /// Defines how compact the list tile's layout will be.
    pub visual_density: Option<VisualDensity>,
    /// An optional focus node to use as the focus node for this widget.
    pub focus_node: Option<FocusNodeRef>,
    /// Controls the interactive states of the backing [`ListTile`].
    pub states_controller: Option<Ref<WidgetStatesController>>,
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
    radio_type: RadioType,
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
    /// Whether to use the checkbox style for the `CupertinoRadio` control.
    ///
    /// Only usable under the [`RadioListTile::adaptive`] constructor. If set to
    /// true, on Apple platforms the radio button will appear as an iOS styled
    /// checkmark. Controls the `CupertinoRadio` through
    /// `CupertinoRadio::use_checkmark_style`.
    ///
    /// Defaults to false. Read through
    /// [`effective_use_cupertino_checkmark_style`](Self::effective_use_cupertino_checkmark_style).
    pub use_cupertino_checkmark_style: bool,
    /// Controls the scaling factor applied to the [`Radio`] within the [`RadioListTile`].
    ///
    /// Defaults to 1.0.
    pub radio_scale_factor: f64,
    /// Whether this widget is interactable.
    ///
    /// If not provided, this widget will be interactable if one of the following
    /// is true:
    ///
    /// * A [`on_changed`](Self::on_changed) is provided.
    /// * Having a [`RadioGroup`] with the same type T above this widget.
    ///
    /// If this is set to true, one of the above condition must also be true.
    /// Otherwise, an assertion error is thrown.
    pub enabled: Option<bool>,
    /// The color of the background of the radio button, in all [`WidgetState`]s.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then it is transparent in all states.
    pub radio_background_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    /// The side for the circular border of the radio button, in all
    /// [`WidgetState`]s.
    ///
    /// This property can be a `BorderSide` or a `WidgetStateBorderSide` to leverage
    /// widget state resolution.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then it defaults to a border using the fill color.
    pub radio_side: Option<AnyBorderSide>,
    /// The radius of the inner circle of the radio button, in all [`WidgetState`]s.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If null, then it defaults to `4.5` in all states.
    pub radio_inner_radius: Option<WidgetStatePropertyRef<Option<f64>>>,
}

/// The setters of `RadioListTile`'s optional arguments that hold a value.
macro_rules! radio_list_tile_setters {
    ($($field:ident: $ty:ty => $name:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `RadioListTile(", $name, ":)`.")]
            pub fn $field(mut self, $field: impl Into<$ty>) -> RadioListTile<T> {
                self.$field = Some($field.into());
                self
            }
        )*
    };
}

/// The setters of `RadioListTile`'s optional arguments that are widgets.
macro_rules! radio_list_tile_widget_setters {
    ($($field:ident => $name:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `RadioListTile(", $name, ":)`.")]
            pub fn $field<K>(mut self, $field: impl IntoWidget<K>) -> RadioListTile<T> {
                self.$field = Some($field.into_widget());
                self
            }
        )*
    };
}

impl<T: Clone + PartialEq + 'static> RadioListTile<T> {
    /// Creates a combination of a list tile and a radio button.
    ///
    /// This widget typically has a [`RadioGroup`] ancestor, which takes in a
    /// [`RadioGroup::group_value`], and the [`RadioListTile`] under it with matching
    /// [`value`](Self::value) will be selected.
    ///
    /// [`value`](Self::value) must be provided
    pub fn new(value: T) -> RadioListTile<T> {
        RadioListTile::with_type(value, RadioType::Material)
    }

    /// Creates a combination of a list tile and a platform adaptive radio.
    ///
    /// The checkbox uses [`Radio::adaptive`] to show a `CupertinoRadio` for
    /// iOS platforms, or [`Radio`] for all others.
    ///
    /// All other properties are the same as [`RadioListTile`].
    pub fn adaptive(value: T) -> RadioListTile<T> {
        RadioListTile::with_type(value, RadioType::Adaptive)
    }

    fn with_type(value: T, radio_type: RadioType) -> RadioListTile<T> {
        RadioListTile {
            key: None,
            value,
            group_value: None,
            on_changed: None,
            mouse_cursor: None,
            toggleable: false,
            active_color: None,
            fill_color: None,
            material_tap_target_size: None,
            hover_color: None,
            overlay_color: None,
            splash_radius: None,
            title: None,
            subtitle: None,
            secondary: None,
            is_three_line: None,
            dense: None,
            selected: false,
            control_affinity: None,
            autofocus: false,
            content_padding: None,
            shape: None,
            tile_color: None,
            selected_tile_color: None,
            visual_density: None,
            focus_node: None,
            states_controller: None,
            on_focus_change: None,
            enable_feedback: None,
            horizontal_title_gap: None,
            min_vertical_padding: None,
            min_leading_width: None,
            min_tile_height: None,
            radio_type,
            title_alignment: None,
            internal_add_semantic_for_on_tap: false,
            use_cupertino_checkmark_style: false,
            radio_scale_factor: 1.0,
            enabled: None,
            radio_background_color: None,
            radio_side: None,
            radio_inner_radius: None,
        }
    }

    /// Dart `RadioListTile(key:)`.
    pub fn key(mut self, key: KeyRef) -> RadioListTile<T> {
        self.key = Some(key);
        self
    }

    /// Dart `RadioListTile(groupValue:)`.
    #[deprecated(
        note = "Use a RadioGroup ancestor to manage group value instead. This feature was \
                deprecated after v3.32.0-0.0.pre."
    )]
    pub fn group_value(mut self, group_value: T) -> RadioListTile<T> {
        self.group_value = Some(group_value);
        self
    }

    /// Dart `RadioListTile(onChanged:)`.
    #[deprecated(
        note = "Use RadioGroup to handle value change instead. This feature was deprecated \
                after v3.32.0-0.0.pre."
    )]
    pub fn on_changed(mut self, on_changed: ValueChanged<Option<T>>) -> RadioListTile<T> {
        self.on_changed = Some(on_changed);
        self
    }

    radio_list_tile_setters!(
        mouse_cursor: MouseCursorRef => "mouseCursor",
        active_color: Color => "activeColor",
        fill_color: WidgetStatePropertyRef<Option<Color>> => "fillColor",
        hover_color: Color => "hoverColor",
        overlay_color: WidgetStatePropertyRef<Option<Color>> => "overlayColor",
        splash_radius: f64 => "splashRadius",
        material_tap_target_size: MaterialTapTargetSize => "materialTapTargetSize",
        is_three_line: bool => "isThreeLine",
        dense: bool => "dense",
        control_affinity: ListTileControlAffinity => "controlAffinity",
        content_padding: EdgeInsetsGeometry => "contentPadding",
        shape: ShapeBorderRef => "shape",
        tile_color: Color => "tileColor",
        selected_tile_color: Color => "selectedTileColor",
        visual_density: VisualDensity => "visualDensity",
        focus_node: FocusNodeRef => "focusNode",
        states_controller: Ref<WidgetStatesController> => "statesController",
        on_focus_change: ValueChanged<bool> => "onFocusChange",
        enable_feedback: bool => "enableFeedback",
        horizontal_title_gap: f64 => "horizontalTitleGap",
        min_vertical_padding: f64 => "minVerticalPadding",
        min_leading_width: f64 => "minLeadingWidth",
        min_tile_height: f64 => "minTileHeight",
        title_alignment: ListTileTitleAlignment => "titleAlignment",
        enabled: bool => "enabled",
        radio_background_color: WidgetStatePropertyRef<Option<Color>> => "radioBackgroundColor",
        radio_side: AnyBorderSide => "radioSide",
        radio_inner_radius: WidgetStatePropertyRef<Option<f64>> => "radioInnerRadius",
    );

    radio_list_tile_widget_setters!(
        title => "title",
        subtitle => "subtitle",
        secondary => "secondary",
    );

    /// Dart `RadioListTile(toggleable:)`.
    pub fn toggleable(mut self, toggleable: bool) -> RadioListTile<T> {
        self.toggleable = toggleable;
        self
    }

    /// Dart `RadioListTile(selected:)`.
    pub fn selected(mut self, selected: bool) -> RadioListTile<T> {
        self.selected = selected;
        self
    }

    /// Dart `RadioListTile(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> RadioListTile<T> {
        self.autofocus = autofocus;
        self
    }

    /// Dart `RadioListTile(internalAddSemanticForOnTap:)`.
    pub fn internal_add_semantic_for_on_tap(
        mut self,
        internal_add_semantic_for_on_tap: bool,
    ) -> RadioListTile<T> {
        self.internal_add_semantic_for_on_tap = internal_add_semantic_for_on_tap;
        self
    }

    /// Dart `RadioListTile(radioScaleFactor:)`.
    pub fn radio_scale_factor(mut self, radio_scale_factor: f64) -> RadioListTile<T> {
        self.radio_scale_factor = radio_scale_factor;
        self
    }

    /// Dart `RadioListTile.adaptive(useCupertinoCheckmarkStyle:)`.
    pub fn use_cupertino_checkmark_style(
        mut self,
        use_cupertino_checkmark_style: bool,
    ) -> RadioListTile<T> {
        self.use_cupertino_checkmark_style = use_cupertino_checkmark_style;
        self
    }

    /// Dart's `useCupertinoCheckmarkStyle` field: what
    /// [`use_cupertino_checkmark_style`](Self::use_cupertino_checkmark_style) was given, and
    /// false for a tile made with [`RadioListTile::new`], whose constructor sets it so.
    pub fn effective_use_cupertino_checkmark_style(&self) -> bool {
        match self.radio_type {
            RadioType::Material => false,
            RadioType::Adaptive => self.use_cupertino_checkmark_style,
        }
    }

    /// Whether this radio button is checked.
    ///
    /// To control this value, set [`value`](Self::value) and
    /// [`group_value`](Self::group_value) appropriately.
    #[deprecated(
        note = "Use RadioGroup.groupValue to find which radio is checked. This feature was \
                deprecated after v3.32.0-0.0.pre."
    )]
    pub fn checked(&self) -> bool {
        self.group_value.as_ref() == Some(&self.value)
    }
}

impl<T: Clone + PartialEq + 'static> Debug for RadioListTile<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RadioListTile")
            .field("toggleable", &self.toggleable)
            .field("selected", &self.selected)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + 'static> StatefulWidget for RadioListTile<T> {
    type State = RadioListTileState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(self.is_three_line != Some(true) || self.subtitle.is_some());
    }

    fn create_state(&self) -> RadioListTileState<T> {
        RadioListTileState {
            state: StateData::new(),
            radio_client: RadioClientData::new(),
            internal_focus_node: None,
            radio_registry: None,
        }
    }
}

/// Dart's `_RadioListTileState`; public because [`RadioListTile`]'s `StatefulWidget::State`
/// names it.
pub struct RadioListTileState<T: Clone + PartialEq + 'static> {
    state: StateData<RadioListTile<T>>,
    radio_client: RadioClientData<T>,
    internal_focus_node: Option<Ref<FocusNode>>,
    radio_registry: Option<Ref<RadioRegistry<T>>>,
}

impl<T: Clone + PartialEq + 'static> RadioListTileState<T> {
    /// Dart's `focusNode` getter, which makes the internal node on first read; the
    /// [`RadioClient`] side reads the node this made.
    fn focus_node(self: Handle<Self>, app: &mut App) -> FocusNodeRef {
        if let Some(focus_node) = self.widget(app).focus_node.clone() {
            return focus_node;
        }
        if let Some(focus_node) = app.get(self).internal_focus_node.as_ref() {
            return focus_node.erase();
        }
        let focus_node = FocusNode::new(app);
        app.get_mut(self).internal_focus_node = Some(focus_node.clone());
        focus_node.into_erased()
    }

    /// Dart's `_radioRegistry`, made on first read as Dart's `late final` is.
    fn radio_registry(self: Handle<Self>, app: &mut App) -> AnyRadioGroupRegistry<T> {
        let registry = match app.get(self).radio_registry.handle() {
            Some(registry) => registry,
            None => {
                let registry = app.create(RadioRegistry { state: self });
                app.get_mut(self).radio_registry = Some(registry.clone());
                registry.handle()
            }
        };
        registry.as_radio_group_registry()
    }

    fn checked(self: Handle<Self>, app: &App) -> bool {
        Some(self.radio_value(app)) == self.effective_group_value(app)
    }

    fn effective_group_value(self: Handle<Self>, app: &App) -> Option<T> {
        RadioClient::registry(self, app)
            .and_then(|registry| registry.group_value(app))
            .or_else(|| self.widget(app).group_value.clone())
    }

    fn handle_list_tile_tap(self: Handle<Self>, app: &mut App) {
        if !self.widget(app).toggleable && self.checked(app) {
            return;
        }
        let new_value = if self.checked(app) {
            None
        } else {
            Some(self.radio_value(app))
        };
        self.handle_change(app, new_value);
    }

    fn handle_change(self: Handle<Self>, app: &mut App, value: Option<T>) {
        if let Some(registry) = RadioClient::registry(self, app) {
            registry.on_changed(app, value.clone());
        }

        if let Some(on_changed) = self.widget(app).on_changed.clone() {
            on_changed(app, value);
        }
    }

    /// The `Radio<T>` or `Radio<T>.adaptive` Dart's `build` puts in the control slot.
    fn build_radio(self: Handle<Self>, app: &mut App) -> Radio<T> {
        let radio_value = self.radio_value(app);
        let radio_registry = self.radio_registry(app);
        let group_value = radio_registry.group_value(app);
        let enabled = RadioClient::enabled(self, app);
        let widget = self.widget(app);
        let mut radio = match widget.radio_type {
            RadioType::Material => Radio::new(radio_value),
            RadioType::Adaptive => Radio::adaptive(radio_value)
                .use_cupertino_checkmark_style(widget.effective_use_cupertino_checkmark_style()),
        }
        .toggleable(widget.toggleable)
        .material_tap_target_size(
            widget
                .material_tap_target_size
                .unwrap_or(MaterialTapTargetSize::ShrinkWrap),
        )
        .autofocus(widget.autofocus)
        .enabled(enabled)
        .group_registry(radio_registry);
        radio.group_value = group_value;
        radio.active_color = widget.active_color;
        radio.fill_color = widget.fill_color.clone();
        radio.mouse_cursor = widget.mouse_cursor.clone();
        radio.hover_color = widget.hover_color;
        radio.overlay_color = widget.overlay_color.clone();
        radio.splash_radius = widget.splash_radius;
        radio.background_color = widget.radio_background_color.clone();
        radio.side = widget.radio_side.clone();
        radio.inner_radius = widget.radio_inner_radius.clone();
        radio
    }
}

impl<T: Clone + PartialEq + 'static> RadioClient<T> for RadioListTileState<T> {
    waft_widgets::radio_client_accessors!(T);

    fn tristate(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).toggleable
    }

    fn radio_value(self: Handle<Self>, app: &App) -> T {
        self.widget(app).value.clone()
    }

    /// Dart's `enabled`, which answers `_enabled`.
    fn enabled(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        widget
            .enabled
            .unwrap_or(widget.on_changed.is_some() || RadioClient::registry(self, app).is_some())
    }

    fn focus_node(self: Handle<Self>, app: &App) -> AnyFocusNode {
        self.widget(app)
            .focus_node
            .handle()
            .or_else(|| {
                app.get(self)
                    .internal_focus_node
                    .as_ref()
                    .map(|node| AnyFocusNode::from(node.handle()))
            })
            .expect("the tile has built, which makes its focus node")
    }
}

impl<T: Clone + PartialEq + 'static> State for RadioListTileState<T> {
    type Widget = RadioListTile<T>;
    waft_widgets::state_accessors!();

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let registry = RadioGroup::<T>::maybe_of(app, context);
        self.set_registry(app, registry);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.set_registry(app, None);
        if let Some(focus_node) = app.get(self).internal_focus_node.handle() {
            focus_node.dispose(app);
        }
        // The radio below has unregistered already: children dispose first.
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(
            !self.widget(app).enabled.unwrap_or(false)
                || self.widget(app).on_changed.is_some()
                || RadioGroup::<T>::maybe_of(app, context).is_some(),
            "Radio is enabled but has no RadioListTile.onChange or registry above"
        );
        let mut control = ExcludeFocus::new(self.build_radio(app)).into_widget();

        let radio_scale_factor = self.widget(app).radio_scale_factor;
        if radio_scale_factor != 1.0 {
            control = Transform::scale(Some(radio_scale_factor), None, None)
                .child(control)
                .into_widget();
        }

        let list_tile_theme = ListTileTheme::of(app, context);
        let widget = self.widget(app);
        let effective_control_affinity = widget
            .control_affinity
            .or(list_tile_theme.control_affinity)
            .unwrap_or(ListTileControlAffinity::Platform);
        let (leading, trailing) = match effective_control_affinity {
            ListTileControlAffinity::Leading | ListTileControlAffinity::Platform => {
                (Some(control), widget.secondary.clone())
            }
            ListTileControlAffinity::Trailing => (widget.secondary.clone(), Some(control)),
        };
        let theme = Theme::of(app, context);
        let radio_theme_data = RadioTheme::of(app, context);
        let widget = self.widget(app);
        let mut states = WidgetStates::new();
        if widget.selected {
            states.insert(WidgetState::Selected);
        }
        let effective_active_color = widget
            .active_color
            .or_else(|| {
                radio_theme_data
                    .fill_color
                    .as_ref()
                    .and_then(|fill_color| fill_color.resolve(&states))
            })
            .unwrap_or(theme.color_scheme.secondary);
        let enabled = RadioClient::enabled(self, app);
        let focus_node = self.focus_node(app);
        let widget = self.widget(app);
        let mut list_tile = ListTile::new()
            .selected_color(effective_active_color)
            .enabled(enabled)
            .selected(widget.selected)
            .autofocus(widget.autofocus)
            .focus_node(focus_node);
        list_tile.leading = leading;
        list_tile.title = widget.title.clone();
        list_tile.subtitle = widget.subtitle.clone();
        list_tile.trailing = trailing;
        list_tile.is_three_line = widget.is_three_line;
        list_tile.dense = widget.dense;
        list_tile.shape = widget.shape.clone();
        list_tile.tile_color = widget.tile_color;
        list_tile.selected_tile_color = widget.selected_tile_color;
        list_tile.on_tap = if enabled {
            Some(GestureTapCallback::handle_method(
                self,
                Self::handle_list_tile_tap,
            ))
        } else {
            None
        };
        list_tile.content_padding = widget.content_padding;
        list_tile.visual_density = widget.visual_density;
        list_tile.states_controller = widget.states_controller.clone();
        list_tile.on_focus_change = widget.on_focus_change.clone();
        list_tile.enable_feedback = widget.enable_feedback;
        list_tile.horizontal_title_gap = widget.horizontal_title_gap;
        list_tile.min_vertical_padding = widget.min_vertical_padding;
        list_tile.min_leading_width = widget.min_leading_width;
        list_tile.min_tile_height = widget.min_tile_height;
        list_tile.title_alignment = widget.title_alignment;
        list_tile.internal_add_semantic_for_on_tap = widget.internal_add_semantic_for_on_tap;
        MergeSemantics::new().child(list_tile).into_widget()
    }
}

/// Dart's `_RadioRegistry`: hands the tile's [`Radio`] the effective group value and routes
/// its changes through the tile, without registering it anywhere.
struct RadioRegistry<T: Clone + PartialEq + 'static> {
    state: Handle<RadioListTileState<T>>,
}

impl<T: Clone + PartialEq + 'static> RadioGroupRegistry<T> for RadioRegistry<T> {
    fn group_value(self: Handle<Self>, app: &App) -> Option<T> {
        app.get(self).state.effective_group_value(app)
    }

    fn register_client(self: Handle<Self>, _app: &mut App, _radio: AnyRadioClient<T>) {}

    fn unregister_client(self: Handle<Self>, _app: &mut App, _radio: AnyRadioClient<T>) {}

    fn on_changed(self: Handle<Self>, app: &mut App, value: Option<T>) {
        let state = app.get(self).state;
        state.handle_change(app, value);
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use waft_embedder::{Matrix4, Offset, PointerChange, Size, TargetPlatform};
    use waft_foundation::K_IS_WEB;
    use waft_painting::{BorderSide, EdgeInsets};
    use waft_services::SystemMouseCursors;
    use waft_widgets::{
        Center, Container, FocusHighlightStrategy, GlobalKey, ListView, SizedBox, StatefulBuilder,
        Text, WidgetStateProperty,
    };

    use waft_widgets::testing::{TestPointer, WidgetTester, test_widgets};

    use super::*;
    use crate::checkbox_list_tile::tests::{
        IS_THREE_LINE_FRAMES, check_control_is_skipped_by_traversal,
        check_default_horizontal_title_gap, check_horizontal_title_gap_0,
        check_horizontal_title_gap_with_visual_density, check_min_leading_width_60,
        check_min_tile_height, check_min_vertical_padding_80, check_on_focus_change,
        check_title_alignment, expected_three_line_rects, focus_of, has_primary_focus,
        ink_rect_colors, ink_well_custom_border, ink_well_enable_feedback, is_three_line_rects,
        nth, pump_in_forbidden_region, pump_in_material, pump_wrapped, rect_of,
        right_rounded_border, text_center,
    };
    use crate::colors::Colors;
    use crate::constants::K_RADIAL_REACTION_ALPHA;
    use crate::list_tile::tests::{m2, m3, text_rect};
    use crate::list_tile_theme::ListTileThemeData;
    use crate::radio::tests::{
        Paints, any_rect, assert_paints, center_of_radio, circle, colors, hover_at, no_op,
        press_at, radio_material,
    };
    use crate::radio_theme::RadioThemeData;
    use crate::scaffold::Scaffold;
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::pump_app;

    fn log_into(log: &Rc<RefCell<Vec<Option<i32>>>>) -> ValueChanged<Option<i32>> {
        let log = Rc::clone(log);
        Rc::new(move |_app, value| log.borrow_mut().push(value))
    }

    /// Dart's `circle()`: a circle of any colour.
    fn any_circle() -> Paints {
        Paints::Circle {
            color: None,
            radius: None,
            stroke_width: None,
        }
    }

    /// Dart's three-tile `buildFrame`: `wrap(StatefulBuilder(Scaffold(ListView.builder(..))))`
    /// of `RadioListTile<int>`s 0, 1 and 2 on `selected_value`, each logging and setting the
    /// value it is changed to.
    fn three_tiles(
        selected_value: &Rc<Cell<Option<i32>>>,
        log: &Rc<RefCell<Vec<Option<i32>>>>,
        toggleable: bool,
    ) -> WidgetRef {
        let (selected_value, log) = (Rc::clone(selected_value), Rc::clone(log));
        StatefulBuilder::new(move |_app, _context, set_state| {
            let (selected_value, log) = (Rc::clone(&selected_value), Rc::clone(&log));
            Scaffold::new()
                .body(
                    ListView::builder(move |_app, _context, index| {
                        let (selected_now, log) = (Rc::clone(&selected_value), Rc::clone(&log));
                        let set_state = Rc::clone(&set_state);
                        let mut tile = RadioListTile::new(index)
                            .on_changed(Rc::new(move |app: &mut App, value: Option<i32>| {
                                log.borrow_mut().push(value);
                                let selected = Rc::clone(&selected_now);
                                set_state(app, &mut |_app| selected.set(value));
                            }))
                            .toggleable(toggleable)
                            .title(Text::new(index.to_string()));
                        tile.group_value = selected_value.get();
                        Some(tile.into_widget())
                    })
                    .item_count(3),
                )
                .into_widget()
        })
        .into_widget()
    }

    fn tap_text(tester: &mut WidgetTester, text: &str) {
        let at = text_center(tester, text);
        tester.tap(at, 1);
    }

    fn tap_radio(tester: &mut WidgetTester, index: usize) {
        let at = rect_of::<Radio<i32>>(tester, index).center();
        tester.tap(at, 1);
    }

    /// 'RadioListTile can be toggled when toggleable is set': Dart's case drives a bare
    /// `Radio`, as radio.rs's toggleable case does.
    #[test]
    fn radio_list_tile_can_be_toggled_when_toggleable_is_set() {
        test_widgets(|tester| {
            let theme = m3(&tester.cell.borrow());
            let key = GlobalKey::new();
            let log: Rc<RefCell<Vec<Option<i32>>>> = Rc::default();
            let wrap = |radio: Radio<i32>| {
                crate::theme::Theme::new(
                    theme.clone(),
                    crate::material::Material::new()
                        .child(Center::new().child(radio.key(key.clone().into()))),
                )
                .into_widget()
            };

            tester.pump_widget(wrap(
                Radio::new(1)
                    .group_value(2)
                    .on_changed(log_into(&log))
                    .toggleable(true),
            ));
            tester.tap(tester.center_of(&key), 1);
            assert_eq!(*log.borrow(), [Some(1)]);
            log.borrow_mut().clear();

            tester.pump_widget(wrap(
                Radio::new(1)
                    .group_value(1)
                    .on_changed(log_into(&log))
                    .toggleable(true),
            ));
            tester.tap(tester.center_of(&key), 2);
            assert_eq!(*log.borrow(), [None]);
            log.borrow_mut().clear();

            tester.pump_widget(wrap(
                Radio::new(1).on_changed(log_into(&log)).toggleable(true),
            ));
            tester.tap(tester.center_of(&key), 3);
            assert_eq!(*log.borrow(), [Some(1)]);
        });
    }

    #[test]
    fn radio_list_tile_should_initialize_according_to_group_value() {
        test_widgets(|tester| {
            let selected_value = Rc::new(Cell::new(None));
            let log = Rc::new(RefCell::new(Vec::new()));
            let checked = |tester: &WidgetTester| -> Vec<bool> {
                (0..3)
                    .map(|index| {
                        tester.widget(
                            nth::<RadioListTile<i32>>(tester, index),
                            |tile: &RadioListTile<i32>| tile.checked(),
                        )
                    })
                    .collect()
            };
            let frame = three_tiles(&selected_value, &log, false);
            pump_wrapped(tester, |_app| frame);
            assert_eq!(checked(tester), [false, false, false]);

            selected_value.set(Some(1));
            let frame = three_tiles(&selected_value, &log, false);
            pump_wrapped(tester, |_app| frame);
            assert_eq!(checked(tester), [false, true, false]);
        });
    }

    #[test]
    fn radio_list_tile_forwards_states_controller_to_list_tile() {
        test_widgets(|tester| {
            let controller = WidgetStatesController::new(&mut tester.cell.borrow_mut(), None);
            pump_wrapped(tester, |_app| {
                RadioListTile::new(1)
                    .group_value(1)
                    .on_changed(no_op())
                    .states_controller(controller.clone())
                    .into_widget()
            });
            let forwarded = tester.widget(nth::<ListTile>(tester, 0), |tile: &ListTile| {
                tile.states_controller.clone()
            });
            assert_eq!(forwarded, Some(controller));
        });
    }

    #[test]
    fn radio_list_tile_simple_control_test() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let title_key = GlobalKey::new();
            let log = Rc::new(RefCell::new(Vec::new()));
            let frame =
                |tester: &mut WidgetTester, group_value: i32, on_changed: bool, tap_title: bool| {
                    let (tile_key, tile_title_key, log) =
                        (key.clone(), title_key.clone(), Rc::clone(&log));
                    pump_wrapped(tester, move |_app| {
                        let mut tile = RadioListTile::new(1)
                            .key(tile_key.into())
                            .group_value(group_value)
                            .title(Text::new("Title").key(tile_title_key.into()));
                        if on_changed {
                            tile = tile.on_changed(log_into(&log));
                        }
                        tile.into_widget()
                    });
                    let at = if tap_title {
                        tester.center_of(&title_key)
                    } else {
                        tester.center_of(&key)
                    };
                    tester.tap(at, 1);
                };

            frame(tester, 2, true, false);
            assert_eq!(*log.borrow(), [Some(1)]);
            log.borrow_mut().clear();

            frame(tester, 1, true, false);
            assert!(log.borrow().is_empty());

            frame(tester, 2, false, false);
            assert!(log.borrow().is_empty());

            frame(tester, 2, true, true);
            assert_eq!(*log.borrow(), [Some(1)]);
        });
    }

    #[test]
    fn radio_list_tile_control_tests() {
        test_widgets(|tester| {
            let selected_value = Rc::new(Cell::new(None));
            let log = Rc::new(RefCell::new(Vec::new()));
            let separator = || log.borrow_mut().push(Some(-1));
            let frame = |tester: &mut WidgetTester| {
                let frame = three_tiles(&selected_value, &log, false);
                pump_wrapped(tester, |_app| frame);
            };

            // Tests for tapping between [Radio] and [ListTile]
            frame(tester);
            tap_text(tester, "1");
            separator();
            tap_radio(tester, 2);
            assert_eq!(*log.borrow(), [Some(1), Some(-1), Some(2)]);
            separator();
            tap_text(tester, "1");

            log.borrow_mut().clear();
            selected_value.set(None);

            // Tests for tapping across [Radio]s exclusively
            frame(tester);
            tap_radio(tester, 1);
            separator();
            tap_radio(tester, 2);
            assert_eq!(*log.borrow(), [Some(1), Some(-1), Some(2)]);

            log.borrow_mut().clear();
            selected_value.set(None);

            // Tests for tapping across [ListTile]s exclusively
            frame(tester);
            tap_text(tester, "1");
            separator();
            tap_text(tester, "2");
            assert_eq!(*log.borrow(), [Some(1), Some(-1), Some(2)]);
        });
    }

    /// "Selected RadioListTile should not trigger onChanged" and "... should trigger
    /// onChanged when toggleable".
    fn tap_selected_tile(
        tester: &mut WidgetTester,
        toggleable: bool,
        expected: [&[Option<i32>]; 3],
    ) {
        // Regression test for https://github.com/flutter/flutter/issues/30311
        let selected_value = Rc::new(Cell::new(None));
        let log = Rc::new(RefCell::new(Vec::new()));
        let frame = three_tiles(&selected_value, &log, toggleable);
        pump_wrapped(tester, |_app| frame);

        tap_text(tester, "0");
        tester.pump();
        assert_eq!(*log.borrow(), expected[0]);

        tap_text(tester, "0");
        tester.pump();
        assert_eq!(*log.borrow(), expected[1]);

        tap_radio(tester, 0);
        tester.pump();
        assert_eq!(*log.borrow(), expected[2]);
    }

    #[test]
    fn selected_radio_list_tile_should_not_trigger_on_changed() {
        test_widgets(|tester| {
            tap_selected_tile(tester, false, [&[Some(0)], &[Some(0)], &[Some(0)]]);
        });
    }

    #[test]
    fn selected_radio_list_tile_should_trigger_on_changed_when_toggleable() {
        test_widgets(|tester| {
            tap_selected_tile(
                tester,
                true,
                [&[Some(0)], &[Some(0), None], &[Some(0), None, Some(0)]],
            );
        });
    }

    #[test]
    fn radio_list_tile_can_autofocus_unless_disabled() {
        test_widgets(|tester| {
            let child_key = GlobalKey::new();
            for (enabled, focused) in [(true, true), (false, false)] {
                let key = child_key.clone();
                pump_wrapped(tester, move |_app| {
                    let mut tile = RadioListTile::new(1)
                        .group_value(2)
                        .title(Text::new("Title").key(key.into()))
                        .autofocus(true);
                    if enabled {
                        tile = tile.on_changed(no_op());
                    }
                    tile.into_widget()
                });
                tester.pump();
                let node = focus_of(tester, &child_key);
                assert_eq!(has_primary_focus(tester, node), focused);
            }
        });
    }

    #[test]
    fn radio_list_tile_content_padding_test() {
        test_widgets(|tester| {
            pump_wrapped(tester, |_app| {
                Center::new()
                    .child(
                        RadioListTile::new(true)
                            .group_value(true)
                            .title(Text::new("Title"))
                            .on_changed(no_op())
                            .content_padding(EdgeInsets::from_ltrb(8.0, 10.0, 15.0, 20.0)),
                    )
                    .into_widget()
            });
            let padding_rect = rect_of::<waft_widgets::SafeArea>(tester, 0);
            let radio_rect = rect_of::<Radio<bool>>(tester, 0);
            let title_rect = text_rect(tester, "Title");

            // Get the taller Rect of the Radio and Text widgets
            let taller_rect = if radio_rect.height() > title_rect.height() {
                radio_rect
            } else {
                title_rect
            };

            // Get the extra height between the tallerRect and ListTile height
            let extra_height = 56.0 - taller_rect.height();

            // Check for correct top and bottom padding
            assert_eq!(
                padding_rect.top,
                taller_rect.top - extra_height / 2.0 - 10.0
            );
            assert_eq!(
                padding_rect.bottom,
                taller_rect.bottom + extra_height / 2.0 + 20.0
            );

            // Check for correct left and right padding
            assert_eq!(padding_rect.left, radio_rect.left - 8.0);
            assert_eq!(padding_rect.right, title_rect.right + 15.0);
        });
    }

    #[test]
    fn radio_list_tile_respects_shape() {
        test_widgets(|tester| {
            pump_in_material(tester, m3, |_app| {
                RadioListTile::new(true)
                    .group_value(true)
                    .title(Text::new("Title"))
                    .shape(ShapeBorderRef::new(right_rounded_border()))
                    .into_widget()
            });
            let shape: ShapeBorderRef = ShapeBorderRef::new(right_rounded_border());
            assert!(ink_well_custom_border(tester).as_deref() == Some(&*shape));
        });
    }

    #[test]
    fn radio_list_tile_respects_tile_color() {
        test_widgets(|tester| {
            let tile_color = Colors::RED.shade500();
            pump_wrapped(tester, |_app| {
                Center::new()
                    .child(
                        RadioListTile::new(false)
                            .group_value(true)
                            .title(Text::new("Title"))
                            .tile_color(tile_color),
                    )
                    .into_widget()
            });
            assert!(ink_rect_colors(tester).contains(&tile_color.to_argb32()));
        });
    }

    #[test]
    fn radio_list_tile_respects_selected_tile_color() {
        test_widgets(|tester| {
            let selected_tile_color = Colors::GREEN.shade500();
            pump_wrapped(tester, |_app| {
                Center::new()
                    .child(
                        RadioListTile::new(false)
                            .group_value(true)
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
    fn radio_list_tile_selected_item_text_color() {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/pull/76906
            const ACTIVE_COLOR: Color = Color::new(0xff00ff00);
            for (active_color, fill_color) in
                [(None, Some(ACTIVE_COLOR)), (Some(ACTIVE_COLOR), None)]
            {
                pump_app(
                    tester,
                    move |app| {
                        ThemeData::builder()
                            .radio_theme(RadioThemeData::new().fill_color(
                                <dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                                    move |states| {
                                        if states.contains(&WidgetState::Selected) {
                                            fill_color
                                        } else {
                                            None
                                        }
                                    },
                                ),
                            ))
                            .build(app)
                    },
                    move |_app| {
                        let mut tile = RadioListTile::new(false)
                            .selected(true)
                            .title(Text::new("title"))
                            .group_value(true)
                            .on_changed(no_op());
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
    fn radio_list_tile_respects_visual_density() {
        test_widgets(|tester| {
            pump_wrapped(tester, |_app| {
                Center::new()
                    .child(
                        RadioListTile::new(false)
                            .group_value(true)
                            .on_changed(no_op())
                            .autofocus(true)
                            .visual_density(VisualDensity::STANDARD),
                    )
                    .into_widget()
            });
            tester.pump_and_settle();
            assert_eq!(
                rect_of::<RadioListTile<bool>>(tester, 0).size(),
                Size::new(800.0, 56.0)
            );
        });
    }

    #[test]
    fn radio_list_tile_respects_focus_node() {
        test_widgets(|tester| {
            let child_key = GlobalKey::new();
            let key = child_key.clone();
            pump_wrapped(tester, move |_app| {
                Center::new()
                    .child(
                        RadioListTile::new(false)
                            .group_value(true)
                            .title(Text::new("A").key(key.into()))
                            .on_changed(no_op()),
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
    fn radio_list_tile_on_focus_change_callback() {
        test_widgets(|tester| {
            check_on_focus_change(tester, |node, on_focus_change| {
                RadioListTile::new(true)
                    .focus_node(node)
                    .on_focus_change(on_focus_change)
                    .on_changed(no_op())
                    .group_value(true)
                    .into_widget()
            });
        });
    }

    #[test]
    fn radio_changes_mouse_cursor_when_hovered() {
        test_widgets(|tester| {
            // Test Radio() constructor
            pump_in_forbidden_region(
                tester,
                RadioListTile::new(1)
                    .mouse_cursor(MouseCursorRef::from(SystemMouseCursors::TEXT))
                    .on_changed(no_op())
                    .group_value(2)
                    .into_widget(),
            );
            let mut gesture = TestPointer::mouse(1, 0);
            let center = center_of_radio::<i32>(tester);
            tester.add_pointer(&mut gesture, center);
            tester.pump();
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::TEXT.into());

            // Test default cursor
            pump_in_forbidden_region(
                tester,
                RadioListTile::new(1)
                    .on_changed(no_op())
                    .group_value(2)
                    .into_widget(),
            );
            let expected: MouseCursorRef = if K_IS_WEB {
                SystemMouseCursors::CLICK.into()
            } else {
                SystemMouseCursors::BASIC.into()
            };
            assert_eq!(Some(tester.active_cursor(1)), Some(expected));

            // Test default cursor when disabled
            pump_in_forbidden_region(tester, RadioListTile::new(1).group_value(2).into_widget());
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::BASIC.into());
            tester.send(&gesture, PointerChange::Remove);
        });
    }

    /// A tile on a group value a `StatefulBuilder` keeps, set by its own `on_changed` when
    /// `enabled`.
    fn stateful_tile(
        group_value: &Rc<Cell<Option<i32>>>,
        enabled: bool,
        configure: impl Fn(RadioListTile<i32>) -> RadioListTile<i32> + 'static,
    ) -> WidgetRef {
        let group_value = Rc::clone(group_value);
        StatefulBuilder::new(move |_app, _context, set_state| {
            let mut tile = configure(RadioListTile::new(0));
            tile.group_value = group_value.get();
            if enabled {
                let group_value = Rc::clone(&group_value);
                tile = tile.on_changed(Rc::new(move |app: &mut App, value: Option<i32>| {
                    let group_value = Rc::clone(&group_value);
                    set_state(app, &mut |_app| group_value.set(value));
                }));
            }
            tile.into_widget()
        })
        .into_widget()
    }

    #[test]
    fn radio_list_tile_respects_fill_color_in_enabled_disabled_states() {
        test_widgets(|tester| {
            const ACTIVE_ENABLED_FILL_COLOR: Color = Color::new(0xFF000001);
            const ACTIVE_DISABLED_FILL_COLOR: Color = Color::new(0xFF000002);
            const INACTIVE_ENABLED_FILL_COLOR: Color = Color::new(0xFF000003);
            const INACTIVE_DISABLED_FILL_COLOR: Color = Color::new(0xFF000004);
            let fill_color = colors(|states| {
                if states.contains(&WidgetState::Disabled) {
                    if states.contains(&WidgetState::Selected) {
                        return ACTIVE_DISABLED_FILL_COLOR;
                    }
                    return INACTIVE_DISABLED_FILL_COLOR;
                }
                if states.contains(&WidgetState::Selected) {
                    return ACTIVE_ENABLED_FILL_COLOR;
                }
                INACTIVE_ENABLED_FILL_COLOR
            });
            let group_value = Rc::new(Cell::new(Some(0)));
            // The tile's transparent `Ink`, Dart's `..rect()`, is not recorded.
            let selected = |color: Color| [circle(color), circle(color)];
            let unselected = |color: Color| [circle(color).stroke_width(2.0)];
            for (value, enabled, expected) in [
                (0, true, selected(ACTIVE_ENABLED_FILL_COLOR).to_vec()),
                (1, true, unselected(INACTIVE_ENABLED_FILL_COLOR).to_vec()),
                (0, false, selected(ACTIVE_DISABLED_FILL_COLOR).to_vec()),
                (1, false, unselected(INACTIVE_DISABLED_FILL_COLOR).to_vec()),
            ] {
                group_value.set(Some(value));
                let fill_color = Rc::clone(&fill_color);
                let host = stateful_tile(&group_value, enabled, move |tile| {
                    tile.fill_color(Rc::clone(&fill_color))
                });
                pump_wrapped(tester, |_app| host);
                tester.pump_and_settle();
                assert_paints(tester, radio_material::<i32>(tester), &expected);
            }
        });
    }

    #[test]
    fn radio_list_tile_respects_fill_color_in_hovered_state() {
        test_widgets(|tester| {
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const HOVERED_FILL_COLOR: Color = Color::new(0xFF000001);
            let fill_color = colors(|states| {
                if states.contains(&WidgetState::Hovered) {
                    return HOVERED_FILL_COLOR;
                }
                Colors::TRANSPARENT
            });
            let group_value = Rc::new(Cell::new(Some(0)));
            let host = stateful_tile(&group_value, true, move |tile| {
                tile.fill_color(Rc::clone(&fill_color))
            });
            pump_wrapped(tester, |_app| host);
            tester.pump_and_settle();

            // Start hovering
            let center = center_of_radio::<i32>(tester);
            hover_at(tester, center);
            tester.pump_and_settle();
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[any_rect(), any_circle(), circle(HOVERED_FILL_COLOR)],
            );
        });
    }

    #[test]
    fn material3_radio_list_tile_respects_hover_color() {
        test_widgets(|tester| {
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let hover_color = Colors::ORANGE.shade500();
            let theme = m3(&tester.cell.borrow());
            let group_value = Rc::new(Cell::new(Some(0)));
            let frame = |tester: &mut WidgetTester, enabled: bool| {
                let host = stateful_tile(&group_value, enabled, move |tile| {
                    tile.hover_color(hover_color)
                });
                // Dart's `wrap(child: MaterialApp(home: host))` paints the tile on the outer
                // `Material`, across the route's repaint boundary, which this tester's pictures
                // do not reach; the tile is on a `Material` inside the route here.
                let theme = theme.clone();
                pump_in_material(tester, move |_app| theme, |_app| host);
                tester.pump();
                tester.pump_and_settle();
            };

            frame(tester, true);
            let primary = theme.color_scheme.primary;
            // The tile's transparent `Ink`, Dart's `..rect()`, is not recorded.
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(primary), circle(primary)],
            );

            // Start hovering
            let center = center_of_radio::<i32>(tester);
            hover_at(tester, center);

            // Check when the radio isn't selected.
            group_value.set(Some(1));
            frame(tester, true);
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[any_rect(), circle(hover_color)],
            );

            // Check when the radio is selected, but disabled.
            group_value.set(Some(0));
            frame(tester, false);
            let disabled = theme.color_scheme.on_surface.with_opacity(0.38);
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(disabled), circle(disabled)],
            );
        });
    }

    #[test]
    fn material3_radio_list_tile_respects_overlay_color_in_active_pressed_hovered_states() {
        test_widgets(|tester| {
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const FILL_COLOR: Color = Color::new(0xFF000000);
            const ACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000001);
            const INACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000002);
            const HOVER_OVERLAY_COLOR: Color = Color::new(0xFF000003);
            const HOVER_COLOR: Color = Color::new(0xFF000005);
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
                    let mut tile = RadioListTile::new(active)
                        .group_value(true)
                        .on_changed(no_op())
                        .fill_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
                            FILL_COLOR,
                        )))
                        .hover_color(HOVER_COLOR);
                    if use_overlay {
                        tile = tile.overlay_color(overlay);
                    }
                    tile.into_widget()
                });
            };
            let reaction = FILL_COLOR.with_alpha(K_RADIAL_REACTION_ALPHA);
            let pressed = |color: Color| {
                [
                    Paints::Rect {
                        color: Some(Color::new(0x66bcbcbc)),
                        rect: None,
                    },
                    circle(color).radius(20.0),
                ]
            };
            for (pointer, (active, use_overlay, expected)) in [
                (false, false, reaction),
                (true, false, reaction),
                (false, true, INACTIVE_PRESSED_OVERLAY_COLOR),
                (true, true, ACTIVE_PRESSED_OVERLAY_COLOR),
            ]
            .into_iter()
            .enumerate()
            {
                frame(tester, active, use_overlay);
                let center = center_of_radio::<bool>(tester);
                press_at(tester, center, pointer as i64 + 2);
                tester.pump_and_settle();
                assert_paints(tester, radio_material::<bool>(tester), &pressed(expected));
            }

            // Start hovering.
            let center = center_of_radio::<bool>(tester);
            hover_at(tester, center);
            tester.pump_and_settle();

            tester.pump_widget(Container::new().into_widget());
            frame(tester, false, true);
            tester.pump_and_settle();
            assert_paints(
                tester,
                radio_material::<bool>(tester),
                &[
                    Paints::Rect {
                        color: Some(Color::new(0x0a000000)),
                        rect: None,
                    },
                    circle(HOVER_OVERLAY_COLOR).radius(20.0),
                ],
            );
        });
    }

    #[test]
    fn material2_radio_list_tile_respects_overlay_color_in_active_pressed_hovered_states() {
        test_widgets(|tester| {
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const FILL_COLOR: Color = Color::new(0xFF000000);
            const ACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000001);
            const INACTIVE_PRESSED_OVERLAY_COLOR: Color = Color::new(0xFF000002);
            const HOVER_OVERLAY_COLOR: Color = Color::new(0xFF000003);
            const HOVER_COLOR: Color = Color::new(0xFF000005);
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
                    let mut tile = RadioListTile::new(active)
                        .group_value(true)
                        .on_changed(no_op())
                        .fill_color(<dyn WidgetStateProperty<Option<Color>>>::all(Some(
                            FILL_COLOR,
                        )))
                        .hover_color(HOVER_COLOR);
                    if use_overlay {
                        tile = tile.overlay_color(overlay);
                    }
                    tile.into_widget()
                });
            };
            let reaction = FILL_COLOR.with_alpha(K_RADIAL_REACTION_ALPHA);
            for (pointer, (active, use_overlay, expected)) in [
                (false, false, reaction),
                (true, false, reaction),
                (false, true, INACTIVE_PRESSED_OVERLAY_COLOR),
            ]
            .into_iter()
            .enumerate()
            {
                frame(tester, active, use_overlay);
                let center = center_of_radio::<bool>(tester);
                press_at(tester, center, pointer as i64 + 2);
                tester.pump_and_settle();
                assert_paints(
                    tester,
                    radio_material::<bool>(tester),
                    &[any_circle(), circle(expected).radius(20.0)],
                );
            }

            // Start hovering.
            let center = center_of_radio::<bool>(tester);
            hover_at(tester, center);
            tester.pump_and_settle();

            tester.pump_widget(Container::new().into_widget());
            frame(tester, false, true);
            tester.pump_and_settle();
            assert_paints(
                tester,
                radio_material::<bool>(tester),
                &[circle(HOVER_OVERLAY_COLOR).radius(20.0)],
            );
        });
    }

    #[test]
    fn material2_radio_list_tile_respects_hover_color() {
        test_widgets(|tester| {
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            let hover_color = Colors::ORANGE.shade500();
            let group_value = Rc::new(Cell::new(Some(0)));
            let frame = |tester: &mut WidgetTester, enabled: bool| {
                let host = stateful_tile(&group_value, enabled, move |tile| {
                    tile.hover_color(hover_color)
                });
                // As in the Material 3 case, the tile is on a `Material` inside the route.
                pump_in_material(tester, m2, |_app| host);
                tester.pump();
                tester.pump_and_settle();
            };

            frame(tester, true);
            let blue = Color::new(0xff2196f3);
            // The tile's transparent `Ink`, Dart's `..rect()`, is not recorded.
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(blue), circle(blue)],
            );

            // Start hovering
            let center = center_of_radio::<i32>(tester);
            hover_at(tester, center);

            // Check when the radio isn't selected.
            group_value.set(Some(1));
            frame(tester, true);
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[any_rect(), circle(hover_color)],
            );

            // Check when the radio is selected, but disabled.
            group_value.set(Some(0));
            frame(tester, false);
            let disabled = Color::new(0x61000000);
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(disabled), circle(disabled)],
            );
        });
    }

    #[test]
    fn radio_list_tile_respects_splash_radius() {
        test_widgets(|tester| {
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const SPLASH_RADIUS: f64 = 30.0;
            let orange = Colors::ORANGE.shade500();
            pump_wrapped(tester, |_app| {
                RadioListTile::new(0)
                    .on_changed(no_op())
                    .hover_color(orange)
                    .group_value(0)
                    .splash_radius(SPLASH_RADIUS)
                    .into_widget()
            });
            tester.pump_and_settle();
            let center = center_of_radio::<i32>(tester);
            hover_at(tester, center);
            tester.pump_and_settle();
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[circle(orange).radius(SPLASH_RADIUS)],
            );
        });
    }

    #[test]
    fn radio_respects_material_tap_target_size() {
        test_widgets(|tester| {
            pump_wrapped(tester, |_app| {
                RadioListTile::new(true)
                    .group_value(true)
                    .on_changed(no_op())
                    .into_widget()
            });
            // default test
            assert_eq!(
                rect_of::<Radio<bool>>(tester, 0).size(),
                Size::new(40.0, 40.0)
            );

            pump_wrapped(tester, |_app| {
                RadioListTile::new(true)
                    .material_tap_target_size(MaterialTapTargetSize::Padded)
                    .group_value(true)
                    .on_changed(no_op())
                    .into_widget()
            });
            assert_eq!(
                rect_of::<Radio<bool>>(tester, 0).size(),
                Size::new(48.0, 48.0)
            );
        });
    }

    #[test]
    fn radio_list_tile_control_widget_should_not_request_focus_on_traversal() {
        test_widgets(|tester| {
            check_control_is_skipped_by_traversal(tester, |title| {
                let mut tile = RadioListTile::new(true)
                    .group_value(true)
                    .on_changed(no_op());
                tile.title = Some(title);
                tile.into_widget()
            });
        });
    }

    #[test]
    fn radio_list_tile_adaptive_shows_the_correct_radio_platform_widget() {
        test_widgets(|tester| {
            for (platform, cupertino) in [
                (TargetPlatform::IOS, 1),
                (TargetPlatform::MacOS, 1),
                (TargetPlatform::Android, 0),
                (TargetPlatform::Fuchsia, 0),
                (TargetPlatform::Linux, 0),
                (TargetPlatform::Windows, 0),
            ] {
                pump_in_material(
                    tester,
                    move |app| ThemeData::builder().platform(platform).build(app),
                    |_app| {
                        Center::new()
                            .child(
                                RadioListTile::adaptive(1)
                                    .group_value(2)
                                    .on_changed(no_op()),
                            )
                            .into_widget()
                    },
                );
                tester.pump_and_settle();
                assert_eq!(
                    tester
                        .find_by_type::<waft_cupertino::CupertinoRadio<i32>>()
                        .len(),
                    cupertino,
                    "{platform:?}"
                );
            }
        });
    }

    #[test]
    fn radio_list_tile_respects_enable_feedback() {
        test_widgets(|tester| {
            for enable_feedback in [false, true] {
                pump_wrapped(tester, |_app| {
                    Center::new()
                        .child(
                            RadioListTile::new(false)
                                .group_value(true)
                                .selected(true)
                                .on_changed(no_op())
                                .enable_feedback(enable_feedback),
                        )
                        .into_widget()
                });
                assert_eq!(ink_well_enable_feedback(tester), enable_feedback);
            }
        });
    }

    #[test]
    fn radio_list_tile_uses_list_tile_theme_control_affinity() {
        test_widgets(|tester| {
            for (control_affinity, expected) in [
                (ListTileControlAffinity::Leading, Offset::new(72.0, 16.0)),
                (ListTileControlAffinity::Trailing, Offset::new(16.0, 16.0)),
                (ListTileControlAffinity::Platform, Offset::new(72.0, 16.0)),
            ] {
                pump_in_material(tester, m3, |_app| {
                    ListTileTheme::new(
                        RadioListTile::new(0.5)
                            .group_value(1.0)
                            .title(Text::new("RadioListTile"))
                            .on_changed(no_op()),
                    )
                    .data(ListTileThemeData::new().control_affinity(control_affinity))
                    .into_widget()
                });
                assert_eq!(text_rect(tester, "RadioListTile").top_left(), expected);
            }
        });
    }

    #[test]
    fn radio_list_tile_renders_with_default_scale() {
        test_widgets(|tester| {
            pump_in_material(tester, m3, |_app| {
                RadioListTile::new(false).group_value(false).into_widget()
            });
            let radio = nth::<Radio<bool>>(tester, 0);
            let app = tester.cell.borrow();
            assert!(!tester.ancestors(radio).into_iter().any(|ancestor| {
                waft_widgets::downcast_widget::<Transform>(&**ancestor.widget(&app)).is_some()
            }));
        });
    }

    #[test]
    fn radio_list_tile_respects_radio_scale_factor() {
        test_widgets(|tester| {
            const SCALE: f64 = 1.4;
            pump_in_material(tester, m3, |_app| {
                RadioListTile::new(false)
                    .group_value(false)
                    .radio_scale_factor(SCALE)
                    .into_widget()
            });
            let transform = tester.ancestor_of_type::<Transform>(nth::<Radio<bool>>(tester, 0));
            let matrix = tester.widget(transform, |transform: &Transform| transform.transform);
            assert_eq!(matrix, Matrix4::scale(SCALE as f32, SCALE as f32));
        });
    }

    fn three_line_tile(is_three_line: Option<bool>, subtitle: &str) -> WidgetRef {
        three_line_tile_of(RadioListTile::new(0), is_three_line, subtitle)
    }

    fn adaptive_three_line_tile(is_three_line: Option<bool>, subtitle: &str) -> WidgetRef {
        three_line_tile_of(RadioListTile::adaptive(0), is_three_line, subtitle)
    }

    fn three_line_tile_of(
        tile: RadioListTile<i32>,
        is_three_line: Option<bool>,
        subtitle: &str,
    ) -> WidgetRef {
        let group_value = if subtitle == "A" { 2 } else { 1 };
        let mut tile = tile
            .title(Text::new("A"))
            .subtitle(Text::new(subtitle))
            .group_value(group_value);
        tile.is_three_line = is_three_line;
        tile.into_widget()
    }

    #[test]
    fn radio_list_tile_is_three_line() {
        test_widgets(|tester| {
            for (frame, three_line) in IS_THREE_LINE_FRAMES {
                assert_eq!(
                    is_three_line_rects::<Radio<i32>, RadioListTile<i32>>(
                        tester,
                        TargetPlatform::Android,
                        frame,
                        three_line_tile
                    ),
                    expected_three_line_rects(
                        three_line,
                        16.0,
                        Size::new(40.0, 40.0),
                        [130.0, 300.0 + 16.0]
                    ),
                    "{frame:?}"
                );
            }
        });
    }

    #[test]
    fn radio_list_tile_adaptive_is_three_line() {
        test_widgets(|tester| {
            for (frame, three_line) in IS_THREE_LINE_FRAMES {
                assert_eq!(
                    is_three_line_rects::<Radio<i32>, RadioListTile<i32>>(
                        tester,
                        TargetPlatform::IOS,
                        frame,
                        adaptive_three_line_tile
                    ),
                    expected_three_line_rects(
                        three_line,
                        16.0,
                        Size::new(18.0, 18.0),
                        [141.0, 300.0 + 27.0]
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
        let mut tile = RadioListTile::new(true)
            .group_value(true)
            .on_changed(no_op())
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
        test_widgets(|tester| {
            check_title_alignment::<Radio<bool>>(tester, title_alignment_tile, false);
        });
    }

    #[test]
    fn title_alignment_position_with_title_and_subtitle_widgets() {
        test_widgets(|tester| {
            check_title_alignment::<Radio<bool>>(tester, title_alignment_tile, true);
        });
    }

    /// `wrap(StatefulBuilder(RadioGroup<int>(groupValue:, onChanged: set it,
    /// child: RadioListTile<int>(value: 0, ..))))`.
    fn grouped_tile(
        group_value: &Rc<Cell<Option<i32>>>,
        configure: impl Fn(RadioListTile<i32>) -> RadioListTile<i32> + 'static,
    ) -> WidgetRef {
        let group_value = Rc::clone(group_value);
        StatefulBuilder::new(move |_app, _context, set_state| {
            let group_value_now = Rc::clone(&group_value);
            let mut group = RadioGroup::new(
                Rc::new(move |app: &mut App, value: Option<i32>| {
                    let group_value = Rc::clone(&group_value_now);
                    set_state(app, &mut |_app| group_value.set(value));
                }),
                configure(RadioListTile::new(0)),
            );
            group.group_value = group_value.get();
            group.into_widget()
        })
        .into_widget()
    }

    #[test]
    fn radio_list_tile_respects_radio_background_color_in_enabled_disabled_states() {
        test_widgets(|tester| {
            const ACTIVE_ENABLED_BACKGROUND_COLOR: Color = Color::new(0xFF000001);
            const ACTIVE_DISABLED_BACKGROUND_COLOR: Color = Color::new(0xFF000002);
            const INACTIVE_ENABLED_BACKGROUND_COLOR: Color = Color::new(0xFF000003);
            const INACTIVE_DISABLED_BACKGROUND_COLOR: Color = Color::new(0xFF000004);
            let background_color = colors(|states| {
                if states.contains(&WidgetState::Disabled) {
                    if states.contains(&WidgetState::Selected) {
                        return ACTIVE_DISABLED_BACKGROUND_COLOR;
                    }
                    return INACTIVE_DISABLED_BACKGROUND_COLOR;
                }
                if states.contains(&WidgetState::Selected) {
                    return ACTIVE_ENABLED_BACKGROUND_COLOR;
                }
                INACTIVE_ENABLED_BACKGROUND_COLOR
            });
            let group_value = Rc::new(Cell::new(Some(0)));
            for (value, enabled, expected) in [
                (0, true, ACTIVE_ENABLED_BACKGROUND_COLOR),
                (1, true, INACTIVE_ENABLED_BACKGROUND_COLOR),
                (0, false, ACTIVE_DISABLED_BACKGROUND_COLOR),
                (1, false, INACTIVE_DISABLED_BACKGROUND_COLOR),
            ] {
                group_value.set(Some(value));
                let background_color = Rc::clone(&background_color);
                let host = grouped_tile(&group_value, move |tile| {
                    tile.radio_background_color(Rc::clone(&background_color))
                        .enabled(enabled)
                });
                pump_wrapped(tester, |_app| host);
                tester.pump_and_settle();
                // The tile's transparent `Ink`, Dart's `..rect()`, is not recorded.
                assert_paints(tester, radio_material::<i32>(tester), &[circle(expected)]);
            }
        });
    }

    #[test]
    fn radio_list_tile_respects_radio_background_color_in_hovered_state() {
        test_widgets(|tester| {
            tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
            const HOVERED_BACKGROUND_COLOR: Color = Color::new(0xFF000001);
            let background_color = colors(|states| {
                if states.contains(&WidgetState::Hovered) {
                    return HOVERED_BACKGROUND_COLOR;
                }
                Colors::TRANSPARENT
            });
            pump_in_material(tester, m3, |_app| {
                let mut group = RadioGroup::new(
                    Rc::new(|_app: &mut App, _value: Option<i32>| {}),
                    RadioListTile::new(0).radio_background_color(background_color),
                );
                group.group_value = Some(0);
                group.into_widget()
            });
            tester.pump_and_settle();

            // Start hovering
            let center = center_of_radio::<i32>(tester);
            hover_at(tester, center);
            tester.pump_and_settle();
            assert_paints(
                tester,
                radio_material::<i32>(tester),
                &[any_rect(), any_circle(), circle(HOVERED_BACKGROUND_COLOR)],
            );
        });
    }

    #[test]
    fn radio_side_is_passed_to_the_radio() {
        test_widgets(|tester| {
            let side = BorderSide {
                color: Colors::RED.primary(),
                width: 3.0,
                ..BorderSide::default()
            };
            pump_in_material(tester, m3, |_app| {
                Center::new()
                    .child(RadioListTile::new(true).radio_side(side))
                    .into_widget()
            });
            let passed = tester.widget(nth::<Radio<bool>>(tester, 0), |radio: &Radio<bool>| {
                radio.side.clone()
            });
            assert!(passed == Some(side.into()));
        });
    }

    #[test]
    fn radio_inner_radius_is_passed_to_the_radio() {
        test_widgets(|tester| {
            let inner_radius = <dyn WidgetStateProperty<Option<f64>>>::all(Some(6.0));
            let given = Rc::clone(&inner_radius);
            pump_in_material(tester, m3, |_app| {
                Center::new()
                    .child(RadioListTile::new(true).radio_inner_radius(given))
                    .into_widget()
            });
            let passed = tester.widget(nth::<Radio<bool>>(tester, 0), |radio: &Radio<bool>| {
                radio.inner_radius.clone()
            });
            assert!(passed.is_some_and(|passed| Rc::ptr_eq(&passed, &inner_radius)));
        });
    }

    #[test]
    fn radio_list_tile_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                Scaffold::new()
                    .body(Center::new().child(SizedBox::shrink().child(RadioListTile::new(true))))
                    .into_widget()
            });
            assert_eq!(rect_of::<RadioListTile<bool>>(tester, 0).size(), Size::ZERO);
        });
    }

    fn leading_title_tile() -> RadioListTile<i32> {
        RadioListTile::new(1)
            .group_value(1)
            .on_changed(no_op())
            .control_affinity(ListTileControlAffinity::Leading)
            .title(Text::new("title"))
    }

    #[test]
    fn radio_list_tile_horizontal_title_gap_0() {
        test_widgets(|tester| {
            check_horizontal_title_gap_0(tester, [56.0, 744.0], |gap| {
                let mut tile = leading_title_tile();
                tile.horizontal_title_gap = gap;
                tile.into_widget()
            });
        });
    }

    #[test]
    fn radio_list_tile_default_horizontal_title_gap_and_min_leading_width() {
        test_widgets(|tester| {
            check_default_horizontal_title_gap(tester, [72.0, 728.0], || {
                leading_title_tile().into_widget()
            });
        });
    }

    #[test]
    fn radio_list_tile_horizontal_title_gap_with_visual_density() {
        test_widgets(|tester| {
            check_horizontal_title_gap_with_visual_density(tester, 58.0, || {
                leading_title_tile()
                    .visual_density(VisualDensity::new(VisualDensity::MINIMUM_DENSITY, 0.0))
                    .horizontal_title_gap(10.0)
                    .into_widget()
            });
        });
    }

    fn min_vertical_padding_tile(padding: Option<f64>) -> WidgetRef {
        let mut tile = RadioListTile::new(1)
            .group_value(1)
            .on_changed(no_op())
            .title(Text::new("title"));
        tile.min_vertical_padding = padding;
        tile.into_widget()
    }

    #[test]
    fn radio_list_tile_min_vertical_padding_80_material_3() {
        test_widgets(|tester| {
            // 80 + 80 + 24(Title) = 184
            check_min_vertical_padding_80(tester, m3, 184.0, min_vertical_padding_tile);
        });
    }

    #[test]
    fn radio_list_tile_min_vertical_padding_80_material_2() {
        test_widgets(|tester| {
            // 80 + 80 + 16(Title) = 176
            check_min_vertical_padding_80(tester, m2, 176.0, min_vertical_padding_tile);
        });
    }

    #[test]
    fn radio_list_tile_min_leading_width_60() {
        test_widgets(|tester| {
            check_min_leading_width_60(tester, |width| {
                let mut tile = leading_title_tile();
                tile.min_leading_width = width;
                tile.into_widget()
            });
        });
    }

    #[test]
    fn radio_list_tile_min_tile_height() {
        test_widgets(|tester| {
            check_min_tile_height(tester, |min_tile_height| {
                let mut tile = RadioListTile::new(1).group_value(1).on_changed(no_op());
                tile.min_tile_height = min_tile_height;
                tile.into_widget()
            });
        });
    }
}
