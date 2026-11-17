//! Flutter counterpart: material_ui's `button_style_button.dart`.
//!
//! Dart's abstract `ButtonStyleButton` widget is two pieces a subclass holds and implements:
//! the [`ButtonStyleButtonData`] bag of its fields, and the [`ButtonStyleButton`] trait with
//! its override points [`default_style_of`](ButtonStyleButton::default_style_of) and
//! [`theme_style_of`](ButtonStyleButton::theme_style_of). Every subclass keeps its own widget
//! type and shares Dart's `_ButtonStyleState` as [`ButtonStyleState<Self>`].
//!
//! # Writing a subclass
//!
//! A subclass (`ElevatedButton`, `TextButton`, …) is a struct with the bag under the field
//! `button_style_button` and its own fields after it:
//!
//! ```text
//! pub struct ElevatedButton {
//!     button_style_button: ButtonStyleButtonData,
//!     add_padding: bool,
//! }
//!
//! impl ElevatedButton {
//!     pub fn new<K>(
//!         on_pressed: Option<Listener>,
//!         child: Option<impl IntoWidget<K>>,
//!     ) -> ElevatedButton {
//!         ElevatedButton {
//!             button_style_button: ButtonStyleButtonData::new(
//!                 on_pressed,
//!                 child.map(|child| child.into_widget()),
//!             ),
//!             add_padding: false,
//!         }
//!     }
//!
//!     // The `super.` arguments of the Dart constructor, by their field names.
//!     crate::button_style_button_setters!(
//!         ElevatedButton;
//!         key, on_long_press, on_hover, on_focus_change, style, focus_node, autofocus,
//!         clip_behavior, states_controller, is_selected,
//!     );
//! }
//!
//! impl ButtonStyleButton for ElevatedButton {
//!     crate::button_style_button_accessors!();
//!
//!     fn default_style_of(&self, app: &mut App, context: BuildContext) -> Box<dyn ButtonStyleGetters> { … }
//!     fn theme_style_of(&self, app: &mut App, context: BuildContext) -> Option<ButtonStyle> { … }
//! }
//!
//! impl StatefulWidget for ElevatedButton {
//!     type State = ButtonStyleState<ElevatedButton>;
//!
//!     fn key(&self) -> Option<&KeyRef> {
//!         self.button_style_button.key.as_ref()
//!     }
//!
//!     fn create_state(&self) -> ButtonStyleState<ElevatedButton> {
//!         ButtonStyleState::new()
//!     }
//! }
//!
//! impl Debug for ElevatedButton {
//!     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//!         Debug::fmt(&self.button_style_button, f)
//!     }
//! }
//! ```
//!
//! A default the Dart subclass gives a `super.` argument (`super.autofocus = false`,
//! `super.clipBehavior = Clip.none`) is set on the bag in its `new`. Dart's static helpers
//! are `<dyn ButtonStyleButton>::all_or_null`, `::default_color` and `::scaled_padding`.
#![allow(deprecated)]

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    AnimationBehavior, AnimationController, AnimationLocalStatusListenersMixin, AnimationStatus,
    AnimationStatusListener,
};
use waft_embedder::{Clip, Color, Offset, Size, TextBaseline};
use waft_foundation::{App, Handle, Listenable, Listener, OptionRetained, Ref, ValueChanged};
use waft_painting::{Alignment, EdgeInsets, EdgeInsetsGeometry, ShapeBorderRef, force_to_point};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, BoxParentData,
    ChildLayoutHelper, ChildLayouter, PaintingContext, RenderBox, RenderBoxBase, RenderBoxData,
    RenderHandle, RenderObject, RenderObjectData, RenderObjectWithChildData,
    RenderObjectWithChildMixin, RenderShiftedBox,
};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_services::MouseCursorRef;
use waft_widgets::{
    Align, BuildContext, ConstrainedBox, FocusNodeRef, IconTheme, IconThemeData, IntoWidget,
    KeyRef, Padding, RenderObjectWidget, Semantics, SingleChildRenderObjectWidget, State,
    StateData, StatefulWidget, TickerProviderStateMixin, TickerProviderStateMixinData, WidgetRef,
    WidgetState, WidgetStateMouseCursor, WidgetStateProperty, WidgetStatePropertyRef, WidgetStates,
    downcast_widget,
};

use crate::button_style::ButtonStyle;
use crate::button_style::ButtonStyleGetters;
use crate::colors::Colors;
use crate::constants::K_MIN_INTERACTIVE_DIMENSION;
use crate::ink_well::InkWell;
use crate::material::{Material, MaterialType};
use crate::material_state::{MaterialStatePropertyAll, MaterialStatesController};
use crate::theme::{AnimatedTheme, Theme};
use crate::theme_data::MaterialTapTargetSize;
use crate::tooltip::Tooltip;

/// Determines the alignment of the icon within the widgets such as:
///   - `ElevatedButton::icon`,
///   - `FilledButton::icon`,
///   - `FilledButton::tonal_icon`.
///   - `OutlinedButton::icon`,
///   - `TextButton::icon`,
///
/// The effect of `icon_alignment` depends on `TextDirection`. If text direction is
/// `TextDirection::Ltr` then [`IconAlignment::Start`] and [`IconAlignment::End`] align the
/// icon on the left or right respectively.  If text direction is `TextDirection::Rtl` the
/// the alignments are reversed.
///
/// Defaults to [`IconAlignment::Start`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconAlignment {
    /// The icon is placed at the start of the button.
    Start,

    /// The icon is placed at the end of the button.
    End,
}

/// The fields of Dart's abstract `ButtonStyleButton`, which every subclass carries under the
/// field `button_style_button`.
#[derive(Clone)]
pub struct ButtonStyleButtonData {
    /// See [`Widget::key`](waft_widgets::Widget::key).
    pub key: Option<KeyRef>,

    /// Called when the button is tapped or otherwise activated.
    ///
    /// If this callback and [`on_long_press`](Self::on_long_press) are null, then the button
    /// will be disabled.
    ///
    /// See also:
    ///
    ///  * [`ButtonStyleButton::enabled`], which is true if the button is enabled.
    pub on_pressed: Option<Listener>,

    /// Called when the button is long-pressed.
    ///
    /// If this callback and [`on_pressed`](Self::on_pressed) are null, then the button will
    /// be disabled.
    ///
    /// See also:
    ///
    ///  * [`ButtonStyleButton::enabled`], which is true if the button is enabled.
    pub on_long_press: Option<Listener>,

    /// Called when a pointer enters or exits the button response area.
    ///
    /// The value passed to the callback is true if a pointer has entered this
    /// part of the material and false if a pointer has exited this part of the
    /// material.
    pub on_hover: Option<ValueChanged<bool>>,

    /// Handler called when the focus changes.
    ///
    /// Called with true if this widget's node gains focus, and false if it loses
    /// focus.
    pub on_focus_change: Option<ValueChanged<bool>>,

    /// Customizes this button's appearance.
    ///
    /// Non-null properties of this style override the corresponding
    /// properties in [`ButtonStyleButton::theme_style_of`] and
    /// [`ButtonStyleButton::default_style_of`]. [`WidgetStateProperty`]s that resolve to
    /// non-null values will similarly override the corresponding [`WidgetStateProperty`]s in
    /// [`ButtonStyleButton::theme_style_of`] and [`ButtonStyleButton::default_style_of`].
    ///
    /// Null by default.
    pub style: Option<ButtonStyle>,

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to [`Clip::None`] unless [`ButtonStyle::background_builder`] or
    /// [`ButtonStyle::foreground_builder`] is specified. In those
    /// cases the default is [`Clip::AntiAlias`].
    pub clip_behavior: Option<Clip>,

    /// An optional focus node to use as the focus node for this widget.
    pub focus_node: Option<FocusNodeRef>,

    /// True if this widget will be selected as the initial focus when no other
    /// node in its scope is currently focused.
    pub autofocus: bool,

    /// Represents the interactive "state" of this widget in terms of a set of
    /// [`WidgetState`]s, like [`WidgetState::Pressed`] and [`WidgetState::Focused`].
    ///
    /// Classes based on this one can provide their own [`MaterialStatesController`] to which
    /// they've added listeners. They can also update the controller's value however, this may
    /// only be done when it's safe to call `State::set_state`, like in an event handler.
    pub states_controller: Option<Ref<MaterialStatesController>>,

    /// Whether this button is in the selected state.
    ///
    /// If null, the button is not selectable. Otherwise, this value is the source
    /// of truth for the button's selected state. It is written to
    /// [`WidgetState::Selected`] when the button is first built, when a new
    /// [`states_controller`](Self::states_controller) is provided, and whenever
    /// [`is_selected`](Self::is_selected) changes. It also
    /// controls the button's selected semantics. Tapping the button does not
    /// change [`is_selected`](Self::is_selected); update it in
    /// [`on_pressed`](Self::on_pressed) to create a toggle button.
    ///
    /// Avoid also updating [`WidgetState::Selected`] through
    /// [`states_controller`](Self::states_controller). That changes only the button's
    /// appearance, not its semantics, and is overwritten the next time
    /// [`is_selected`](Self::is_selected) changes.
    pub is_selected: Option<bool>,

    /// Determine whether this subtree represents a button.
    ///
    /// If this is null, the screen reader will not announce "button" when this
    /// is focused. This is useful for `MenuItemButton` and `SubmenuButton` when we
    /// traverse the menu system.
    ///
    /// Defaults to true.
    pub is_semantic_button: Option<bool>,

    /// Determines the alignment of the icon within the button; see [`IconAlignment`].
    #[deprecated(
        note = "Remove this parameter as it is now ignored. Use ButtonStyle.iconAlignment instead. This feature was deprecated after v3.28.0-1.0.pre."
    )]
    pub icon_alignment: Option<IconAlignment>,

    /// Text that describes the action that will occur when the button is pressed or
    /// hovered over.
    ///
    /// This text is displayed when the user long-presses or hovers over the button
    /// in a tooltip. This string is also used for accessibility.
    ///
    /// If null, the button will not display a tooltip.
    pub tooltip: Option<String>,

    /// Typically the button's label.
    ///
    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,
}

impl ButtonStyleButtonData {
    /// The fields a subclass's constructor passes on, with Dart's `required` `onPressed` and
    /// `child`; the rest start at the base class's defaults and are set by
    /// [`button_style_button_setters!`](crate::button_style_button_setters).
    pub fn new(on_pressed: Option<Listener>, child: Option<WidgetRef>) -> ButtonStyleButtonData {
        ButtonStyleButtonData {
            key: None,
            on_pressed,
            on_long_press: None,
            on_hover: None,
            on_focus_change: None,
            style: None,
            clip_behavior: None,
            focus_node: None,
            autofocus: false,
            states_controller: None,
            is_selected: None,
            is_semantic_button: Some(true),
            icon_alignment: None,
            tooltip: None,
            child,
        }
    }

    /// Whether the button is enabled or disabled; see [`ButtonStyleButton::enabled`].
    pub fn enabled(&self) -> bool {
        self.on_pressed.is_some() || self.on_long_press.is_some()
    }
}

impl Debug for ButtonStyleButtonData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut properties = f.debug_struct("ButtonStyleButton");
        if !self.enabled() {
            properties.field("enabled", &"disabled");
        }
        if self.is_selected == Some(true) {
            properties.field("selected", &"selected");
        }
        if let Some(style) = &self.style {
            properties.field("style", style);
        }
        if self.focus_node.is_some() {
            properties.field("focus_node", &"FocusNode");
        }
        properties.finish()
    }
}

/// The [`ButtonStyleButtonData`] accessor a [`ButtonStyleButton`] implementor writes; the bag
/// lives under the field `button_style_button`.
#[macro_export]
macro_rules! button_style_button_accessors {
    () => {
        fn button_style_button_data(&self) -> &$crate::ButtonStyleButtonData {
            &self.button_style_button
        }
    };
}

/// The fluent setters of the `super.` arguments a subclass's Dart constructor takes, named by
/// their [`ButtonStyleButtonData`] fields: `button_style_button_setters!(TextButton; key,
/// style, autofocus)`. Each writes the field of the bag under `button_style_button`.
#[macro_export]
macro_rules! button_style_button_setters {
    ($name:ident; $($field:ident),* $(,)?) => {
        $($crate::button_style_button_setters!(@setter $name $field);)*
    };
    (@setter $name:ident key) => {
        $crate::button_style_button_setters!(@option $name key: ::waft_widgets::KeyRef => "key");
    };
    (@setter $name:ident on_long_press) => {
        $crate::button_style_button_setters!(
            @option $name on_long_press: ::waft_foundation::Listener => "onLongPress"
        );
    };
    (@setter $name:ident on_hover) => {
        $crate::button_style_button_setters!(
            @option $name on_hover: ::waft_foundation::ValueChanged<bool> => "onHover"
        );
    };
    (@setter $name:ident on_focus_change) => {
        $crate::button_style_button_setters!(
            @option $name on_focus_change: ::waft_foundation::ValueChanged<bool> => "onFocusChange"
        );
    };
    (@setter $name:ident style) => {
        $crate::button_style_button_setters!(@option $name style: $crate::ButtonStyle => "style");
    };
    (@setter $name:ident focus_node) => {
        $crate::button_style_button_setters!(
            @option $name focus_node: ::waft_widgets::FocusNodeRef => "focusNode"
        );
    };
    (@setter $name:ident clip_behavior) => {
        $crate::button_style_button_setters!(
            @option $name clip_behavior: ::waft_embedder::Clip => "clipBehavior"
        );
    };
    (@setter $name:ident states_controller) => {
        $crate::button_style_button_setters!(
            @option $name states_controller:
                ::waft_foundation::Ref<::waft_widgets::WidgetStatesController> => "statesController"
        );
    };
    (@setter $name:ident is_selected) => {
        #[doc = concat!("Dart `", stringify!($name), "(isSelected:)`.")]
        #[allow(clippy::wrong_self_convention)]
        pub fn is_selected(mut self, is_selected: bool) -> $name {
            self.button_style_button.is_selected = Some(is_selected);
            self
        }
    };
    (@setter $name:ident is_semantic_button) => {
        #[doc = concat!("Dart `", stringify!($name), "(isSemanticButton:)`.")]
        #[allow(clippy::wrong_self_convention)]
        pub fn is_semantic_button(mut self, is_semantic_button: bool) -> $name {
            self.button_style_button.is_semantic_button = Some(is_semantic_button);
            self
        }
    };
    (@setter $name:ident tooltip) => {
        #[doc = concat!("Dart `", stringify!($name), "(tooltip:)`.")]
        pub fn tooltip(mut self, tooltip: impl Into<String>) -> $name {
            self.button_style_button.tooltip = Some(tooltip.into());
            self
        }
    };
    (@setter $name:ident autofocus) => {
        #[doc = concat!("Dart `", stringify!($name), "(autofocus:)`.")]
        pub fn autofocus(mut self, autofocus: bool) -> $name {
            self.button_style_button.autofocus = autofocus;
            self
        }
    };
    (@option $name:ident $field:ident: $ty:ty => $dart:literal) => {
        #[doc = concat!("Dart `", stringify!($name), "(", $dart, ":)`.")]
        pub fn $field(mut self, $field: $ty) -> $name {
            self.button_style_button.$field = Some($field);
            self
        }
    };
}

/// The base widget for buttons whose style is defined by a [`ButtonStyle`] object; Dart's
/// abstract `ButtonStyleButton`, whose state is [`ButtonStyleState`].
///
/// Concrete subclasses must override [`default_style_of`](Self::default_style_of) and
/// [`theme_style_of`](Self::theme_style_of). A subclass is its own widget type: see the
/// module documentation for its shape.
///
/// See also:
///  * `ElevatedButton`, a filled button whose material elevates when pressed.
///  * `FilledButton`, a filled button that doesn't elevate when pressed.
///  * `FilledButton::tonal`, a filled button variant that uses a secondary fill color.
///  * `OutlinedButton`, a button with an outlined border and no fill color.
///  * `TextButton`, a button with no outline or fill color.
///  * <https://m3.material.io/components/buttons/overview>, an overview of each of
///    the Material Design button types and how they should be used in designs.
pub trait ButtonStyleButton: 'static {
    /// The [`ButtonStyleButtonData`] bag
    /// ([`button_style_button_accessors!`](crate::button_style_button_accessors)).
    fn button_style_button_data(&self) -> &ButtonStyleButtonData;

    /// Returns a [`ButtonStyle`] that's based primarily on the `Theme`'s
    /// `ThemeData::text_theme` and `ThemeData::color_scheme`, but has most values
    /// filled out (non-null).
    ///
    /// The returned style can be overridden by the [`ButtonStyleButtonData::style`]
    /// parameter and by the style returned by [`theme_style_of`](Self::theme_style_of) that
    /// some button-specific themes like `TextButtonTheme` or `ElevatedButtonTheme` override.
    /// For example the default style of the `TextButton` subclass can be overridden with its
    /// `TextButton::style` constructor parameter, or with a `TextButtonTheme`.
    ///
    /// Concrete button subclasses should return a [`ButtonStyle`] with as many
    /// non-null properties as possible, where all of the non-null
    /// [`WidgetStateProperty`] properties resolve to non-null values.
    ///
    /// ## Properties that can be null
    ///
    /// Some properties, like [`ButtonStyle::fixed_size`] would override other values
    /// in the same [`ButtonStyle`] if set, so they are allowed to be null.  Here is
    /// a summary of properties that are allowed to be null when returned in the
    /// [`ButtonStyle`] returned by this function, an why:
    ///
    /// - [`ButtonStyle::fixed_size`] because it would override other values in the
    ///   same [`ButtonStyle`], like [`ButtonStyle::maximum_size`].
    /// - [`ButtonStyle::side`] because null is a valid value for a button that has
    ///   no side. `OutlinedButton` returns a non-null default for this, however.
    /// - [`ButtonStyle::background_builder`] and [`ButtonStyle::foreground_builder`]
    ///   because they would override the [`ButtonStyle::foreground_color`] and
    ///   [`ButtonStyle::background_color`] of the same [`ButtonStyle`].
    ///
    /// See also:
    ///
    /// * [`theme_style_of`](Self::theme_style_of), returns the ButtonStyle of this button's
    ///   component theme.
    fn default_style_of(&self, app: &mut App, context: BuildContext)
    -> Box<dyn ButtonStyleGetters>;

    /// Returns the ButtonStyle that belongs to the button's component theme.
    ///
    /// The returned style can be overridden by the [`ButtonStyleButtonData::style`]
    /// parameter.
    ///
    /// Concrete button subclasses should return the ButtonStyle for the
    /// nearest subclass-specific inherited theme, and if no such theme
    /// exists, then the same value from the overall `Theme`.
    ///
    /// See also:
    ///
    ///  * [`default_style_of`](Self::default_style_of), Returns the default [`ButtonStyle`]
    ///    for this button.
    fn theme_style_of(&self, app: &mut App, context: BuildContext) -> Option<ButtonStyle>;

    /// Whether the button is enabled or disabled.
    ///
    /// Buttons are disabled by default. To enable a button, set its
    /// [`on_pressed`](ButtonStyleButtonData::on_pressed) or
    /// [`on_long_press`](ButtonStyleButtonData::on_long_press) properties to a non-null value.
    fn enabled(&self) -> bool {
        self.button_style_button_data().enabled()
    }
}

impl dyn ButtonStyleButton {
    /// Returns null if `value` is null, otherwise `WidgetStatePropertyAll<T>(value)`.
    ///
    /// A convenience method for subclasses.
    pub fn all_or_null<T: Clone + Debug + PartialEq + 'static>(
        value: Option<T>,
    ) -> Option<WidgetStatePropertyRef<Option<T>>> {
        value.map(|value| {
            Rc::new(MaterialStatePropertyAll::new(Some(value))) as WidgetStatePropertyRef<_>
        })
    }

    /// Returns null if `enabled` and `disabled` are null.
    /// Otherwise, returns a [`WidgetStateProperty`] that resolves to `disabled`
    /// when [`WidgetState::Disabled`] is active, and `enabled` otherwise.
    ///
    /// A convenience method for subclasses.
    pub fn default_color(
        enabled: Option<Color>,
        disabled: Option<Color>,
    ) -> Option<WidgetStatePropertyRef<Option<Color>>> {
        enabled.or(disabled)?;
        Some(<dyn WidgetStateProperty<Option<Color>>>::from_map(vec![
            (WidgetState::Disabled.into(), disabled),
            (WidgetState::any(), enabled),
        ]))
    }

    /// A convenience method used by subclasses in the framework, that returns an
    /// interpolated value based on the `font_size_multiplier` parameter:
    ///
    ///  * 0 - 1 `geometry1x`
    ///  * 1 - 2 lerp(`geometry1x`, `geometry2x`, `font_size_multiplier` - 1)
    ///  * 2 - 3 lerp(`geometry2x`, `geometry3x`, `font_size_multiplier` - 2)
    ///  * otherwise `geometry3x`
    ///
    /// This method is used by the framework for estimating the default paddings to
    /// use on a button with a text label, when the system text scaling setting
    /// changes. It's usually supplied with empirical `geometry1x`, `geometry2x`,
    /// `geometry3x` values adjusted for different system text scaling values, when
    /// the unscaled font size is set to 14.0 (the default `TextTheme::label_large`
    /// value).
    ///
    /// The `font_size_multiplier` argument, for historical reasons, is the default
    /// font size specified in the [`ButtonStyle`], scaled by the ambient font
    /// scaler, then divided by 14.0 (the default font size used in buttons).
    pub fn scaled_padding(
        geometry1x: EdgeInsetsGeometry,
        geometry2x: EdgeInsetsGeometry,
        geometry3x: EdgeInsetsGeometry,
        font_size_multiplier: f64,
    ) -> EdgeInsetsGeometry {
        match font_size_multiplier {
            multiplier if multiplier <= 1.0 => geometry1x,
            multiplier if multiplier < 2.0 => {
                EdgeInsetsGeometry::lerp(Some(geometry1x), Some(geometry2x), multiplier - 1.0)
                    .expect("both sides")
            }
            multiplier if multiplier < 3.0 => {
                EdgeInsetsGeometry::lerp(Some(geometry2x), Some(geometry3x), multiplier - 2.0)
                    .expect("both sides")
            }
            _ => geometry3x,
        }
    }
}

/// The base [`State`] class for buttons whose style is defined by a [`ButtonStyle`] object;
/// Dart's private `_ButtonStyleState`, which a subclass names as its `StatefulWidget::State`.
///
/// See also:
///
///  * [`ButtonStyleButton`], the widget for which this class is the [`State`].
///  * `ElevatedButton`, a filled button whose material elevates when pressed.
///  * `FilledButton`, a filled ButtonStyleButton that doesn't elevate when pressed.
///  * `OutlinedButton`, similar to `TextButton`, but with an outline.
///  * `TextButton`, a simple button without a shadow.
pub struct ButtonStyleState<W>
where
    W: ButtonStyleButton + StatefulWidget<State = ButtonStyleState<W>>,
{
    state: StateData<W>,
    ticker_provider: TickerProviderStateMixinData,
    controller: Option<Ref<AnimationController>>,
    elevation: Option<f64>,
    background_color: Option<Color>,
    internal_states_controller: Option<Ref<MaterialStatesController>>,
}

impl<W> ButtonStyleState<W>
where
    W: ButtonStyleButton + StatefulWidget<State = ButtonStyleState<W>>,
{
    /// The state a subclass's `create_state` returns.
    pub fn new() -> ButtonStyleState<W> {
        ButtonStyleState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            controller: None,
            elevation: None,
            background_color: None,
            internal_states_controller: None,
        }
    }

    fn handle_states_controller_change(self: Handle<Self>, app: &mut App) {
        // Force a rebuild to resolve WidgetStateProperty properties
        self.set_state(app, |_state| {});
    }

    fn states_controller(self: Handle<Self>, app: &App) -> Ref<MaterialStatesController> {
        self.widget(app)
            .button_style_button_data()
            .states_controller
            .clone()
            .or_else(|| app.get(self).internal_states_controller.clone())
            .expect("created in init_states_controller")
    }

    fn init_states_controller(self: Handle<Self>, app: &mut App) {
        if self
            .widget(app)
            .button_style_button_data()
            .states_controller
            .is_none()
        {
            let controller = MaterialStatesController::new(app, None);
            app.get_mut(self).internal_states_controller = Some(controller);
        }
        let enabled = self.widget(app).enabled();
        let is_selected = self.widget(app).button_style_button_data().is_selected;
        let states_controller = self.states_controller(app);
        states_controller.update(app, WidgetState::Disabled, !enabled);
        if let Some(is_selected) = is_selected {
            states_controller.update(app, WidgetState::Selected, is_selected);
        }
        states_controller.add_listener(
            app,
            Listener::handle_method(self, Self::handle_states_controller_change),
        );
    }

    fn handle_controller_status(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        if status == AnimationStatus::Completed {
            self.set_state(app, |_state| {}); // Rebuild with the final background color.
        }
    }
}

impl<W> Default for ButtonStyleState<W>
where
    W: ButtonStyleButton + StatefulWidget<State = ButtonStyleState<W>>,
{
    fn default() -> ButtonStyleState<W> {
        ButtonStyleState::new()
    }
}

impl<W> TickerProviderStateMixin for ButtonStyleState<W>
where
    W: ButtonStyleButton + StatefulWidget<State = ButtonStyleState<W>>,
{
    fn ticker_provider_data(self: Handle<Self>, app: &App) -> &TickerProviderStateMixinData {
        &app.get(self).ticker_provider
    }

    fn ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut TickerProviderStateMixinData {
        &mut app.get_mut(self).ticker_provider
    }
}

/// Dart's `vsync: this`.
impl<W> TickerProviderObject for ButtonStyleState<W>
where
    W: ButtonStyleButton + StatefulWidget<State = ButtonStyleState<W>>,
{
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl<W> State for ButtonStyleState<W>
where
    W: ButtonStyleButton + StatefulWidget<State = ButtonStyleState<W>>,
{
    type Widget = W;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.init_states_controller(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &W) {
        let old = old_widget.button_style_button_data();
        let widget_states_controller = self
            .widget(app)
            .button_style_button_data()
            .states_controller
            .clone();
        if widget_states_controller != old.states_controller {
            if let Some(old_states_controller) = old.states_controller.clone() {
                old_states_controller.remove_listener(
                    app,
                    &Listener::handle_method(self, Self::handle_states_controller_change),
                );
            }
            if widget_states_controller.is_some()
                && let Some(internal) = app.get_mut(self).internal_states_controller.take()
            {
                internal.dispose(app);
            }
            self.init_states_controller(app);
        }
        let enabled = self.widget(app).enabled();
        if enabled != old_widget.enabled() {
            let states_controller = self.states_controller(app);
            states_controller.update(app, WidgetState::Disabled, !enabled);
            if !enabled {
                // The button may have been disabled while a press gesture is currently underway.
                states_controller.update(app, WidgetState::Pressed, false);
            }
        }
        let is_selected = self.widget(app).button_style_button_data().is_selected;
        if is_selected != old.is_selected {
            let states_controller = self.states_controller(app);
            states_controller.update(app, WidgetState::Selected, is_selected.unwrap_or(false));
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.states_controller(app).remove_listener(
            app,
            &Listener::handle_method(self, Self::handle_states_controller_change),
        );
        if let Some(internal) = app.get_mut(self).internal_states_controller.take() {
            internal.dispose(app);
        }
        if let Some(controller) = app.get(self).controller.handle() {
            controller.dispose(app);
        }
        TickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = Theme::of(app, context);
        let icon_theme = IconTheme::of(app, context);
        let widget_ref = context.widget(app).clone();
        let widget =
            downcast_widget::<W>(&*widget_ref).expect("a ButtonStyleState builds its button");
        let button = widget.button_style_button_data();
        let styles = ButtonStyleLayers {
            widget_style: button.style.clone(),
            theme_style: widget.theme_style_of(app, context),
            default_style: widget.default_style_of(app, context),
        };
        let states = app.get(&self.states_controller(app)).value().clone();

        let resolved_elevation = styles.resolve(app, &states, |style, app| style.elevation(app));
        let resolved_text_style = styles.resolve(app, &states, |style, app| style.text_style(app));
        let mut resolved_background_color =
            styles.resolve(app, &states, |style, app| style.background_color(app));
        let resolved_foreground_color =
            styles.resolve(app, &states, |style, app| style.foreground_color(app));
        let resolved_shadow_color =
            styles.resolve(app, &states, |style, app| style.shadow_color(app));
        let resolved_surface_tint_color =
            styles.resolve(app, &states, |style, app| style.surface_tint_color(app));
        let resolved_padding = styles.resolve(app, &states, |style, app| style.padding(app));
        let resolved_minimum_size =
            styles.resolve(app, &states, |style, app| style.minimum_size(app));
        let resolved_fixed_size = styles.resolve(app, &states, |style, app| style.fixed_size(app));
        let resolved_maximum_size =
            styles.resolve(app, &states, |style, app| style.maximum_size(app));
        let resolved_icon_color = styles.effective_icon_color(app, &states);
        let resolved_icon_size = styles.resolve(app, &states, |style, app| style.icon_size(app));
        let resolved_side = styles.resolve(app, &states, |style, app| style.side(app));
        let resolved_shape = styles.resolve(app, &states, |style, app| style.shape(app));

        let mouse_cursors = styles.properties(app, |style, app| style.mouse_cursor(app));
        let mouse_cursor = mouse_cursor(move |states| {
            ButtonStyleLayers::effective_resolved_value(&mouse_cursors, states)
        });

        let overlay_colors = styles.properties(app, |style, app| style.overlay_color(app));
        let overlay_color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
            ButtonStyleLayers::effective_resolved_value(&overlay_colors, states)
        });

        let resolved_visual_density =
            styles.effective_value(app, |style, app| style.visual_density(app));
        let resolved_tap_target_size =
            styles.effective_value(app, |style, app| style.tap_target_size(app));
        let resolved_animation_duration =
            styles.effective_value(app, |style, app| style.animation_duration(app));
        let resolved_enable_feedback = styles
            .effective_value(app, |style, app| style.enable_feedback(app))
            .unwrap_or(true);
        let resolved_alignment = styles.effective_value(app, |style, app| style.alignment(app));
        let resolved_visual_density =
            resolved_visual_density.expect("the default style has a visual density");
        let density_adjustment = resolved_visual_density.base_size_adjustment();
        let resolved_splash_factory =
            styles.effective_value(app, |style, app| style.splash_factory(app));
        let resolved_background_builder =
            styles.effective_value(app, |style, app| style.background_builder(app));
        let resolved_foreground_builder =
            styles.effective_value(app, |style, app| style.foreground_builder(app));

        let effective_clip_behavior = button.clip_behavior.unwrap_or(
            if resolved_background_builder.is_some() || resolved_foreground_builder.is_some() {
                Clip::AntiAlias
            } else {
                Clip::None
            },
        );

        let resolved_minimum_size =
            resolved_minimum_size.expect("the default style has a minimum size");
        let resolved_maximum_size =
            resolved_maximum_size.expect("the default style has a maximum size");
        let mut effective_constraints = resolved_visual_density.effective_constraints(
            BoxConstraints::new()
                .min_width(resolved_minimum_size.width())
                .min_height(resolved_minimum_size.height())
                .max_width(resolved_maximum_size.width())
                .max_height(resolved_maximum_size.height()),
        );
        if let Some(resolved_fixed_size) = resolved_fixed_size {
            let size = effective_constraints.constrain(resolved_fixed_size);
            if size.width().is_finite() {
                effective_constraints = effective_constraints
                    .min_width(size.width())
                    .max_width(size.width());
            }
            if size.height().is_finite() {
                effective_constraints = effective_constraints
                    .min_height(size.height())
                    .max_height(size.height());
            }
        }

        // Per the Material Design team: don't allow the VisualDensity
        // adjustment to reduce the width of the left/right padding. If we
        // did, VisualDensity.compact, the default for desktop/web, would
        // reduce the horizontal padding to zero.
        let dy = density_adjustment.dy();
        let dx = density_adjustment.dx().max(0.0);
        let padding = resolved_padding
            .expect("the default style has a padding")
            .add(EdgeInsets::from_ltrb(dx, dy, dx, dy).into())
            .clamp(&EdgeInsets::ZERO.into(), &EdgeInsetsGeometry::INFINITY);

        // If an opaque button's background is becoming translucent while its
        // elevation is changing, change the elevation first. Material implicitly
        // animates its elevation but not its color. SKIA renders non-zero
        // elevations as a shadow colored fill behind the Material's background.
        let resolved_animation_duration =
            resolved_animation_duration.expect("the default style has an animation duration");
        let elevation = app.get(self).elevation;
        let background_color = app.get(self).background_color;
        if resolved_animation_duration > Duration::ZERO
            && let Some(background_color) = background_color
            && elevation.is_some()
            && elevation != resolved_elevation
            && background_color.value()
                != resolved_background_color
                    .expect("a background color to compare with")
                    .value()
            && background_color.opacity() == 1.0
            && resolved_background_color
                .expect("a background color to compare with")
                .opacity()
                < 1.0
            && resolved_elevation == Some(0.0)
        {
            let controller = app.get(self).controller.handle();
            if controller.map(|controller| app.get(controller).duration)
                != Some(Some(resolved_animation_duration))
            {
                if let Some(controller) = controller {
                    controller.dispose(app);
                }
                let controller = AnimationController::new(
                    app,
                    None,
                    Some(resolved_animation_duration),
                    None,
                    0.0,
                    1.0,
                    AnimationBehavior::Normal,
                    self,
                );
                controller.add_status_listener(
                    app,
                    AnimationStatusListener::handle_method(self, Self::handle_controller_status),
                );
                app.get_mut(self).controller = Some(controller);
            }
            resolved_background_color = Some(background_color); // Defer changing the background color.
            let controller = app.get(self).controller.handle().expect("created above");
            controller.set_value(app, 0.0);
            controller.forward(app, None);
        }
        let this = app.get_mut(self);
        this.elevation = resolved_elevation;
        this.background_color = resolved_background_color;

        let mut align = Align::new()
            .alignment(resolved_alignment.expect("the default style has an alignment"))
            .width_factor(1.0)
            .height_factor(1.0);
        align.child = match &resolved_foreground_builder {
            Some(foreground_builder) => {
                Some(foreground_builder.call(app, context, &states, button.child.clone()))
            }
            None => button.child.clone(),
        };
        let mut result = Padding::new(padding).child(align).into_widget();
        if let Some(background_builder) = &resolved_background_builder {
            result = background_builder.call(app, context, &states, Some(result));
        }

        let resolved_shape = resolved_shape.expect("the default style has a shape");
        let mut icon_theme_data = IconThemeData::new();
        if let Some(resolved_icon_color) = resolved_icon_color {
            icon_theme_data = icon_theme_data.color(resolved_icon_color);
        }
        if let Some(resolved_icon_size) = resolved_icon_size {
            icon_theme_data = icon_theme_data.size(resolved_icon_size);
        }
        let custom_border: ShapeBorderRef = resolved_shape.copy_with(resolved_side).into();
        let mut ink_well = InkWell::new()
            .mouse_cursor(MouseCursorRef::from(mouse_cursor))
            .enable_feedback(resolved_enable_feedback)
            .can_request_focus(widget.enabled())
            .autofocus(button.autofocus)
            .overlay_color(overlay_color)
            .highlight_color(Colors::TRANSPARENT)
            .custom_border(custom_border)
            .states_controller(self.states_controller(app))
            .child(result);
        ink_well.ink_response.on_tap = button.on_pressed.clone();
        ink_well.ink_response.on_long_press = button.on_long_press.clone();
        ink_well.ink_response.on_hover = button.on_hover.clone();
        ink_well.ink_response.focus_node = button.focus_node.clone();
        ink_well.ink_response.on_focus_change = button.on_focus_change.clone();
        ink_well.ink_response.splash_factory = resolved_splash_factory;
        let mut result = AnimatedTheme::new(
            theme
                .copy_with()
                .icon_theme(icon_theme.merge(Some(&icon_theme_data)))
                .build(),
            ink_well,
        )
        .duration(resolved_animation_duration)
        .into_widget();

        if let Some(tooltip) = &button.tooltip {
            result = Tooltip::new()
                .message(tooltip.clone())
                .child(result)
                .into_widget();
        }

        let min_size =
            match resolved_tap_target_size.expect("the default style has a tap target size") {
                MaterialTapTargetSize::Padded => {
                    let min_size = Size::new(
                        K_MIN_INTERACTIVE_DIMENSION + density_adjustment.dx(),
                        K_MIN_INTERACTIVE_DIMENSION + density_adjustment.dy(),
                    );
                    debug_assert!(min_size.width() >= 0.0);
                    debug_assert!(min_size.height() >= 0.0);
                    min_size
                }
                MaterialTapTargetSize::ShrinkWrap => Size::ZERO,
            };

        let shape: ShapeBorderRef = resolved_shape.copy_with(resolved_side).into();
        let mut material = Material::new()
            .elevation(resolved_elevation.expect("the default style has an elevation"))
            .shape(shape)
            .r#type(if resolved_background_color.is_none() {
                MaterialType::Transparency
            } else {
                MaterialType::Button
            })
            .animation_duration(resolved_animation_duration)
            .clip_behavior(effective_clip_behavior)
            .border_on_foreground(false)
            .child(result);
        material.text_style =
            resolved_text_style.map(|text_style| match resolved_foreground_color {
                Some(resolved_foreground_color) => {
                    text_style.copy_with().color(resolved_foreground_color)
                }
                None => text_style.copy_with(),
            });
        material.color = resolved_background_color;
        material.shadow_color = resolved_shadow_color;
        material.surface_tint_color = resolved_surface_tint_color;

        let mut semantics = Semantics::new().container(true).enabled(widget.enabled());
        if let Some(is_semantic_button) = button.is_semantic_button {
            semantics = semantics.button(is_semantic_button);
        }
        if let Some(is_selected) = button.is_selected {
            semantics = semantics.selected(is_selected);
        }
        semantics
            .child(InputPadding {
                min_size,
                child: Some(
                    ConstrainedBox::new(effective_constraints)
                        .child(material)
                        .into_widget(),
                ),
            })
            .into_widget()
    }
}

/// The three styles `_ButtonStyleState.build` reads, highest precedence first: the button's
/// own, its component theme's and its default. Its methods are Dart's local functions
/// `effectiveValue`, `resolve` and `effectiveIconColor`.
struct ButtonStyleLayers {
    widget_style: Option<ButtonStyle>,
    theme_style: Option<ButtonStyle>,
    default_style: Box<dyn ButtonStyleGetters>,
}

impl ButtonStyleLayers {
    /// The widget's, the theme's and the default style's value of one property, which Dart's
    /// `effectiveValue` reads from all three.
    fn properties<T>(
        &self,
        app: &mut App,
        get_property: impl Fn(&dyn ButtonStyleGetters, &mut App) -> Option<T>,
    ) -> [Option<T>; 3] {
        let widget_value = self
            .widget_style
            .as_ref()
            .and_then(|style| get_property(style, app));
        let theme_value = self
            .theme_style
            .as_ref()
            .and_then(|style| get_property(style, app));
        let default_value = get_property(&*self.default_style, app);
        [widget_value, theme_value, default_value]
    }

    fn effective_value<T>(
        &self,
        app: &mut App,
        get_property: impl Fn(&dyn ButtonStyleGetters, &mut App) -> Option<T>,
    ) -> Option<T> {
        let [widget_value, theme_value, default_value] = self.properties(app, get_property);
        widget_value.or(theme_value).or(default_value)
    }

    fn resolve<T: 'static>(
        &self,
        app: &mut App,
        states: &WidgetStates,
        get_property: impl Fn(
            &dyn ButtonStyleGetters,
            &mut App,
        ) -> Option<WidgetStatePropertyRef<Option<T>>>,
    ) -> Option<T> {
        let properties = self.properties(app, get_property);
        ButtonStyleLayers::effective_resolved_value(&properties, states)
    }

    /// Dart's `effectiveValue((style) => style?.property?.resolve(states))` over the three
    /// styles' properties, read when the button was built: the property a button resolves later,
    /// without the `App`, is a `WidgetStateProperty` it built with them.
    fn effective_resolved_value<T: 'static>(
        properties: &[Option<WidgetStatePropertyRef<Option<T>>>; 3],
        states: &WidgetStates,
    ) -> Option<T> {
        properties
            .iter()
            .flatten()
            .find_map(|property| property.resolve(states))
    }

    fn effective_icon_color(&self, app: &mut App, states: &WidgetStates) -> Option<Color> {
        let resolve = |property: Option<WidgetStatePropertyRef<Option<Color>>>| {
            property.and_then(|property| property.resolve(states))
        };
        self.widget_style
            .as_ref()
            .and_then(|style| resolve(style.icon_color.clone()))
            .or_else(|| {
                self.theme_style
                    .as_ref()
                    .and_then(|style| resolve(style.icon_color.clone()))
            })
            .or_else(|| {
                self.widget_style
                    .as_ref()
                    .and_then(|style| resolve(style.foreground_color.clone()))
            })
            .or_else(|| {
                self.theme_style
                    .as_ref()
                    .and_then(|style| resolve(style.foreground_color.clone()))
            })
            .or_else(|| resolve(self.default_style.icon_color(app)))
            // Fallback to foregroundColor if iconColor is null.
            .or_else(|| resolve(self.default_style.foreground_color(app)))
    }
}

/// Dart's `_MouseCursor`: a [`WidgetStateMouseCursor`] that resolves through
/// `resolve_callback`, which must answer.
fn mouse_cursor(
    resolve_callback: impl Fn(&WidgetStates) -> Option<MouseCursorRef> + 'static,
) -> WidgetStateMouseCursor {
    WidgetStateMouseCursor::resolve_with(
        move |states| resolve_callback(states).expect("a button style resolves a mouse cursor"),
        Some("ButtonStyleButton_MouseCursor"),
    )
}

/// A widget to pad the area around a [`ButtonStyleButton`]'s inner `Material`.
///
/// Redirect taps that occur in the padded area around the child to the center
/// of the child. This increases the size of the button and the button's
/// "tap target", but not its material or its ink splashes.
#[derive(Debug)]
struct InputPadding {
    min_size: Size,
    child: Option<WidgetRef>,
}

impl RenderObjectWidget for InputPadding {
    type RenderObject = RenderInputPadding;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderInputPadding::new(app, self.min_size, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderInputPadding>,
    ) {
        render_object.set_min_size(app, self.min_size);
    }
}

impl SingleChildRenderObjectWidget for InputPadding {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// Dart's `_RenderInputPadding`.
struct RenderInputPadding {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    min_size: Size,
}

impl RenderInputPadding {
    fn new(
        app: &mut App,
        min_size: Size,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderInputPadding> {
        let this = RenderHandle::new_box(
            app,
            RenderInputPadding {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                min_size,
            },
        );
        this.set_child(app, child);
        this
    }

    fn min_size(self: RenderHandle<Self>, app: &App) -> Size {
        self.get(app).min_size
    }

    fn set_min_size(self: RenderHandle<Self>, app: &mut App, value: Size) {
        if self.get(app).min_size == value {
            return;
        }
        self.get_mut(app).min_size = value;
        RenderBox::mark_needs_layout(self, app);
    }

    fn compute_size(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        layout_child: ChildLayouter,
    ) -> Size {
        if let Some(child) = self.child(app) {
            let child_size = layout_child(app, child, constraints);
            let min_size = self.min_size(app);
            let width = child_size.width().max(min_size.width());
            let height = child_size.height().max(min_size.height());
            return constraints.constrain(Size::new(width, height));
        }
        Size::ZERO
    }
}

impl RenderObjectWithChildMixin for RenderInputPadding {
    type ChildType = AnyRenderBox;

    fn child_data(self: RenderHandle<Self>, app: &App) -> &RenderObjectWithChildData<AnyRenderBox> {
        &self.get(app).child
    }

    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
        &mut self.get_mut(app).child
    }
}

impl RenderShiftedBox for RenderInputPadding {}

impl RenderObject for RenderInputPadding {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let size = self.compute_size(app, constraints, ChildLayoutHelper::layout_child);
        self.set_size(app, size);
        if let Some(child) = self.child(app) {
            let child_size = child.size(app);
            child.parent_data_of_mut::<BoxParentData>(app).offset =
                Alignment::CENTER.along_offset(size - child_size);
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderShiftedBox::paint(self, app, context, offset);
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child.as_object());
        }
    }
}

impl RenderBox for RenderInputPadding {
    waft_rendering::render_box_accessors!();

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        if let Some(child) = self.child(app) {
            return child
                .get_min_intrinsic_width(app, height)
                .max(self.min_size(app).width());
        }
        0.0
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        if let Some(child) = self.child(app) {
            return child
                .get_min_intrinsic_height(app, width)
                .max(self.min_size(app).height());
        }
        0.0
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        if let Some(child) = self.child(app) {
            return child
                .get_max_intrinsic_width(app, height)
                .max(self.min_size(app).width());
        }
        0.0
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        if let Some(child) = self.child(app) {
            return child
                .get_max_intrinsic_height(app, width)
                .max(self.min_size(app).height());
        }
        0.0
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        self.compute_size(app, constraints, ChildLayoutHelper::dry_layout_child)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let result = child.get_dry_baseline(app, constraints, baseline)?;
        let child_size = child.get_dry_layout(app, constraints);
        let size = self.as_box().get_dry_layout(app, constraints);
        Some(result + Alignment::CENTER.along_offset(size - child_size).dy())
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderShiftedBox::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderShiftedBox::hit_test_children(self, app, result, position)
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        if RenderBoxBase::hit_test(self, app, result, position) {
            return true;
        }
        let child = self.child(app).expect("an input padding has a child");
        let center = child.size(app).center(Offset::ZERO);
        result.add_with_raw_transform(Some(force_to_point(center)), center, |result, position| {
            debug_assert!(
                (position.dx() - center.dx()).abs() <= center.dx().abs() * f32::EPSILON as f64
                    && (position.dy() - center.dy()).abs()
                        <= center.dy().abs() * f32::EPSILON as f64,
                "the forced transform maps every hit to the child's center"
            );
            child.hit_test(app, result, center)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use waft_painting::OutlinedBorderRef;

    use waft_embedder::{Radius, Rect, TextDirection};
    use waft_painting::{AlignmentGeometry, BorderRadius, BoxDecoration, RoundedRectangleBorder};
    use waft_rendering::{
        RenderClipPath, RenderCustomClip, RenderDecoratedBox, RenderPhysicalModelBase,
        RenderPhysicalShape,
    };
    use waft_services::SystemMouseCursors;
    use waft_widgets::{
        Builder, Center, Column, DecoratedBox, DefaultTextStyle, Directionality, FocusNode,
        GlobalKey, MainAxisSize, SizedBox, WidgetStatePropertyAll,
    };

    use waft_widgets::testing::{TestPointer, WidgetTester, test_widgets};

    use super::*;
    use crate::button_style::ButtonLayerBuilder;
    use crate::color_scheme::ColorScheme;
    use crate::constants::K_THEME_CHANGE_DURATION;
    use crate::ink_ripple::InkRipple;
    use crate::theme_data::{ThemeData, VisualDensity};

    /// A leaf of [`ButtonStyleButton`] whose default style is Dart's `ElevatedButton`'s in
    /// Material 2 at a text scale of 1, with no component theme: the base class's behaviour
    /// is what the cases below test, as Flutter's do through a concrete button.
    struct TestButton {
        button_style_button: ButtonStyleButtonData,
    }

    impl TestButton {
        fn new<K>(on_pressed: Option<Listener>, child: impl IntoWidget<K>) -> TestButton {
            TestButton {
                button_style_button: ButtonStyleButtonData::new(
                    on_pressed,
                    Some(child.into_widget()),
                ),
            }
        }

        crate::button_style_button_setters!(
            TestButton;
            key, on_long_press, style, focus_node, states_controller, is_selected,
        );
    }

    fn all<T: Clone + Debug + PartialEq + 'static>(value: T) -> WidgetStatePropertyRef<Option<T>> {
        Rc::new(WidgetStatePropertyAll::new(Some(value)))
    }

    impl ButtonStyleButton for TestButton {
        crate::button_style_button_accessors!();

        // `ElevatedButton.defaultStyleOf`'s Material 2 branch, through `styleFrom`.
        fn default_style_of(
            &self,
            app: &mut App,
            context: BuildContext,
        ) -> Box<dyn ButtonStyleGetters> {
            let theme = Theme::of(app, context);
            let color_scheme = &theme.color_scheme;
            let on_primary = color_scheme.on_primary;
            Box::new(
                ButtonStyle::new()
                    .text_style(Rc::new(WidgetStatePropertyAll::new(
                        theme.text_theme.label_large.clone(),
                    )))
                    .background_color(
                        <dyn ButtonStyleButton>::default_color(
                            Some(color_scheme.primary),
                            Some(color_scheme.on_surface.with_opacity(0.12)),
                        )
                        .expect("both colors"),
                    )
                    .foreground_color(
                        <dyn ButtonStyleButton>::default_color(
                            Some(on_primary),
                            Some(color_scheme.on_surface.with_opacity(0.38)),
                        )
                        .expect("both colors"),
                    )
                    .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::from_map(vec![
                        (
                            WidgetState::Pressed.into(),
                            Some(on_primary.with_opacity(0.1)),
                        ),
                        (
                            WidgetState::Hovered.into(),
                            Some(on_primary.with_opacity(0.08)),
                        ),
                        (
                            WidgetState::Focused.into(),
                            Some(on_primary.with_opacity(0.1)),
                        ),
                        (WidgetState::any(), None),
                    ]))
                    .shadow_color(all(theme.shadow_color))
                    .elevation(<dyn WidgetStateProperty<Option<f64>>>::from_map(vec![
                        (WidgetState::Disabled.into(), Some(0.0)),
                        (WidgetState::Pressed.into(), Some(2.0 + 6.0)),
                        (WidgetState::Hovered.into(), Some(2.0 + 2.0)),
                        (WidgetState::Focused.into(), Some(2.0 + 2.0)),
                        (WidgetState::any(), Some(2.0)),
                    ]))
                    .padding(all(EdgeInsetsGeometry::symmetric(0.0, 16.0)))
                    .minimum_size(all(Size::new(64.0, 36.0)))
                    .maximum_size(all(Size::INFINITE))
                    .shape(all(OutlinedBorderRef::new(RoundedRectangleBorder::new(
                        waft_painting::BorderSide::NONE,
                        BorderRadius::all(Radius::circular(4.0)).into(),
                    ))))
                    .mouse_cursor(<dyn WidgetStateProperty<Option<MouseCursorRef>>>::from_map(
                        vec![
                            (
                                WidgetState::Disabled.into(),
                                Some(SystemMouseCursors::BASIC.into()),
                            ),
                            (WidgetState::any(), Some(SystemMouseCursors::BASIC.into())),
                        ],
                    ))
                    .visual_density(theme.visual_density)
                    .tap_target_size(theme.material_tap_target_size)
                    .animation_duration(K_THEME_CHANGE_DURATION)
                    .enable_feedback(true)
                    .alignment(AlignmentGeometry::CENTER)
                    .splash_factory(InkRipple::splash_factory()),
            )
        }

        fn theme_style_of(&self, _app: &mut App, _context: BuildContext) -> Option<ButtonStyle> {
            None
        }
    }

    impl StatefulWidget for TestButton {
        type State = ButtonStyleState<TestButton>;

        fn key(&self) -> Option<&KeyRef> {
            self.button_style_button.key.as_ref()
        }

        fn create_state(&self) -> ButtonStyleState<TestButton> {
            ButtonStyleState::new()
        }
    }

    impl Debug for TestButton {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            Debug::fmt(&self.button_style_button, f)
        }
    }

    fn ltr<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Directionality::new(TextDirection::Ltr, child).into_widget()
    }

    /// `MaterialApp(theme: theme, home: child)`, as far as a button reads it.
    fn themed<K>(tester: &WidgetTester, theme: ThemeData, child: impl IntoWidget<K>) -> WidgetRef {
        let _ = tester;
        Theme::new(theme, ltr(child)).into_widget()
    }

    fn material2(tester: &WidgetTester) -> ThemeData {
        ThemeData::builder()
            .use_material3(false)
            .build(&tester.cell.borrow())
    }

    fn pressed() -> Listener {
        Listener::new(|_app| {})
    }

    fn rect_of(tester: &WidgetTester, key: &GlobalKey) -> Rect {
        let render = tester.render_object(key).as_box().expect("a render box");
        let app = tester.cell.borrow();
        render.local_to_global(&app, Offset::ZERO, None) & render.size(&app)
    }

    fn sized(width: f64, height: f64) -> SizedBox {
        SizedBox::new().width(width).height(height)
    }

    fn counter(count: &Rc<Cell<usize>>) -> Listener {
        let count = Rc::clone(count);
        Listener::new(move |_app| count.set(count.get() + 1))
    }

    fn states(states: &[WidgetState]) -> WidgetStates {
        states.iter().copied().collect()
    }

    fn value(tester: &WidgetTester, controller: Handle<MaterialStatesController>) -> WidgetStates {
        tester.cell.borrow().get(controller).value().clone()
    }

    fn physical_shape(tester: &WidgetTester) -> (f64, Color) {
        let shape = tester.first::<RenderPhysicalShape>();
        let app = tester.cell.borrow();
        (shape.elevation(&app), shape.color(&app))
    }

    #[test]
    fn default_color_is_null_without_colors_and_resolves_disabled_apart() {
        assert!(<dyn ButtonStyleButton>::default_color(None, None).is_none());
        let red = Color::new(0xffff0000);
        let blue = Color::new(0xff0000ff);
        let color =
            <dyn ButtonStyleButton>::default_color(Some(red), Some(blue)).expect("two colors");
        assert_eq!(color.resolve(&WidgetStates::new()), Some(red));
        assert_eq!(color.resolve(&states(&[WidgetState::Disabled])), Some(blue));
        let only_disabled =
            <dyn ButtonStyleButton>::default_color(None, Some(blue)).expect("one color");
        assert_eq!(only_disabled.resolve(&WidgetStates::new()), None);
        assert!(<dyn ButtonStyleButton>::all_or_null::<f64>(None).is_none());
        assert_eq!(
            <dyn ButtonStyleButton>::all_or_null(Some(2.0))
                .expect("a value")
                .resolve(&WidgetStates::new()),
            Some(2.0)
        );
    }

    #[test]
    fn scaled_padding_interpolates_between_the_three_geometries() {
        let one = EdgeInsetsGeometry::all(12.0);
        let two = EdgeInsetsGeometry::all(8.0);
        let three = EdgeInsetsGeometry::all(4.0);
        let scaled =
            |multiplier| <dyn ButtonStyleButton>::scaled_padding(one, two, three, multiplier);
        assert!(scaled(0.5) == one);
        assert!(scaled(1.0) == one);
        assert!(scaled(1.5) == EdgeInsetsGeometry::all(10.0));
        assert!(scaled(2.5) == EdgeInsetsGeometry::all(6.0));
        assert!(scaled(3.0) == three);
    }

    // elevated_button_test.dart 'ElevatedButton onPressed and onLongPress callbacks are
    // distinctly recognized'.
    #[test]
    fn a_tap_and_a_long_press_reach_their_own_callbacks() {
        test_widgets(|tester| {
            let did_press_button = Rc::new(Cell::new(0));
            let did_long_press_button = Rc::new(Cell::new(0));
            let key = GlobalKey::new();
            tester.pump_widget(ltr(Center::new().child(
                TestButton::new(Some(counter(&did_press_button)), sized(100.0, 20.0))
                    .on_long_press(counter(&did_long_press_button))
                    .key(key.clone().into()),
            )));
            let center = tester.get_center(&key);

            assert_eq!(did_press_button.get(), 0);
            tester.tap_at(center, 1);
            tester.pump_and_settle();
            assert_eq!(did_press_button.get(), 1);

            assert_eq!(did_long_press_button.get(), 0);
            tester.long_press_at(center, 2);
            tester.pump_and_settle();
            assert_eq!(did_long_press_button.get(), 1);
        });
    }

    // elevated_button_test.dart 'ElevatedButton size is configurable by
    // ThemeData.materialTapTargetSize'.
    #[test]
    fn the_theme_tap_target_size_pads_the_button_to_48() {
        test_widgets(|tester| {
            // Specifying minimumSize to mimic the original minimumSize for
            // RaisedButton so that the corresponding button size matches
            // the original version of this test.
            let style = ButtonStyle::new().minimum_size(all(Size::new(88.0, 36.0)));
            let build_frame =
                |tester: &WidgetTester, tap_target_size: MaterialTapTargetSize, key: &GlobalKey| {
                    let theme = ThemeData::builder()
                        .use_material3(false)
                        .material_tap_target_size(tap_target_size)
                        .build(&tester.cell.borrow());
                    themed(
                        tester,
                        theme,
                        Center::new().child(
                            TestButton::new(Some(pressed()), sized(50.0, 8.0))
                                .key(key.clone().into())
                                .style(style.clone()),
                        ),
                    )
                };

            let key1 = GlobalKey::new();
            let frame = build_frame(tester, MaterialTapTargetSize::Padded, &key1);
            tester.pump_widget(frame);
            assert_eq!(tester.get_size(&key1), Size::new(88.0, 48.0));

            let key2 = GlobalKey::new();
            let frame = build_frame(tester, MaterialTapTargetSize::ShrinkWrap, &key2);
            tester.pump_widget(frame);
            assert_eq!(tester.get_size(&key2), Size::new(88.0, 36.0));
        });
    }

    // elevated_button_test.dart 'ElevatedButton has no clip by default': nothing in the
    // button clips, so it paints no `clipPath`.
    #[test]
    fn a_button_does_not_clip_by_default() {
        test_widgets(|tester| {
            tester.pump_widget(ltr(TestButton::new(Some(pressed()), sized(10.0, 10.0))));
            let shape = tester.first::<RenderPhysicalShape>();
            assert_eq!(
                RenderCustomClip::clip_behavior(shape, &tester.cell.borrow()),
                Clip::None
            );
            assert!(tester.all::<RenderClipPath>().is_empty());
        });
    }

    // elevated_button_test.dart 'ElevatedButton responds to density changes.'; the `Text`
    // child is the 56x14 box FlutterTest lays 'Text' out in.
    #[test]
    fn visual_density_grows_and_shrinks_the_button_around_its_child() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let child_key = GlobalKey::new();
            let build_test =
                |tester: &mut WidgetTester, visual_density: VisualDensity, use_text: bool| {
                    let child = if use_text {
                        sized(56.0, 14.0).key(child_key.clone().into())
                    } else {
                        sized(100.0, 100.0).key(child_key.clone().into())
                    };
                    let theme = material2(tester);
                    let frame = Theme::new(
                        theme,
                        Directionality::new(
                            TextDirection::Rtl,
                            Center::new().child(
                                TestButton::new(Some(pressed()), child)
                                    .style(
                                        ButtonStyle::new()
                                            .visual_density(visual_density)
                                            // Specifying minimumSize to mimic the original
                                            // minimumSize for RaisedButton so that the
                                            // corresponding button size matches the original
                                            // version of this test.
                                            .minimum_size(all(Size::new(88.0, 36.0))),
                                    )
                                    .key(key.clone().into()),
                            ),
                        ),
                    )
                    .into_widget();
                    tester.pump_widget(frame);
                };

            build_test(tester, VisualDensity::STANDARD, false);
            let child_rect = rect_of(tester, &child_key);
            tester.pump_and_settle();
            assert_eq!(tester.get_size(&key), Size::new(132.0, 100.0));
            assert_eq!(child_rect, Rect::from_ltrb(350.0, 250.0, 450.0, 350.0));

            build_test(tester, VisualDensity::new(3.0, 3.0), false);
            tester.pump_and_settle();
            assert_eq!(tester.get_size(&key), Size::new(156.0, 124.0));
            assert_eq!(
                rect_of(tester, &child_key),
                Rect::from_ltrb(350.0, 250.0, 450.0, 350.0)
            );

            build_test(tester, VisualDensity::new(-3.0, -3.0), false);
            tester.pump_and_settle();
            assert_eq!(tester.get_size(&key), Size::new(132.0, 100.0));
            assert_eq!(
                rect_of(tester, &child_key),
                Rect::from_ltrb(350.0, 250.0, 450.0, 350.0)
            );

            build_test(tester, VisualDensity::STANDARD, true);
            tester.pump_and_settle();
            assert_eq!(tester.get_size(&key), Size::new(88.0, 48.0));
            assert_eq!(
                rect_of(tester, &child_key),
                Rect::from_ltrb(372.0, 293.0, 428.0, 307.0)
            );

            build_test(tester, VisualDensity::new(3.0, 3.0), true);
            tester.pump_and_settle();
            assert_eq!(tester.get_size(&key), Size::new(112.0, 60.0));
            assert_eq!(
                rect_of(tester, &child_key),
                Rect::from_ltrb(372.0, 293.0, 428.0, 307.0)
            );

            build_test(tester, VisualDensity::new(-3.0, -3.0), true);
            tester.pump_and_settle();
            assert_eq!(tester.get_size(&key), Size::new(88.0, 36.0));
            assert_eq!(
                rect_of(tester, &child_key),
                Rect::from_ltrb(372.0, 293.0, 428.0, 307.0)
            );
        });
    }

    // elevated_button_test.dart 'Elevated buttons animate elevation before color on disable'.
    #[test]
    fn a_disabled_button_lowers_its_elevation_before_it_changes_color() {
        // This is a regression test for https://github.com/flutter/flutter/issues/387
        test_widgets(|tester| {
            let color_scheme = ColorScheme::light();
            let background_color = color_scheme.primary;
            let disabled_background_color = color_scheme.on_surface.with_opacity(0.12);
            let build_frame = |tester: &WidgetTester, enabled: bool| {
                let theme = ThemeData::from(color_scheme.clone())
                    .use_material3(false)
                    .build(&tester.cell.borrow());
                themed(
                    tester,
                    theme,
                    Center::new().child(TestButton::new(enabled.then(pressed), sized(40.0, 10.0))),
                )
            };

            // Default elevation is 2, background color is primary.
            let frame = build_frame(tester, true);
            tester.pump_widget(frame);
            assert_eq!(physical_shape(tester), (2.0, background_color));

            // Disabled elevation animates to 0 over 200ms, THEN the background
            // color changes to onSurface.withOpacity(0.12)
            let frame = build_frame(tester, false);
            tester.pump_widget(frame);
            tester.pump_by(Duration::from_millis(50));
            let (elevation, color) = physical_shape(tester);
            assert!(elevation < 2.0);
            assert_eq!(color, background_color);
            tester.pump_by(Duration::from_millis(150));
            assert_eq!(physical_shape(tester), (0.0, background_color));
            tester.pump_and_settle();
            assert_eq!(physical_shape(tester), (0.0, disabled_background_color));
        });
    }

    // elevated_button_test.dart 'Fixed size ElevatedButtons'.
    #[test]
    fn a_fixed_size_sets_each_finite_dimension() {
        test_widgets(|tester| {
            let keys = [GlobalKey::new(), GlobalKey::new(), GlobalKey::new()];
            let fixed_sizes = [
                Size::new(100.0, 100.0),
                Size::from_width(200.0),
                Size::from_height(200.0),
            ];
            let buttons = keys.iter().zip(fixed_sizes).map(|(key, fixed_size)| {
                TestButton::new(Some(pressed()), sized(20.0, 10.0))
                    .key(key.clone().into())
                    .style(ButtonStyle::new().fixed_size(all(fixed_size)))
                    .into_widget()
            });
            let theme = ThemeData::new(&tester.cell.borrow());
            let frame = themed(
                tester,
                theme,
                Column::new()
                    .main_axis_size(MainAxisSize::Min)
                    .children(buttons.collect::<Vec<_>>()),
            );
            tester.pump_widget(frame);

            assert_eq!(tester.get_size(&keys[0]), Size::new(100.0, 100.0));
            assert_eq!(tester.get_size(&keys[1]).width(), 200.0);
            assert_eq!(tester.get_size(&keys[2]).height(), 200.0);
        });
    }

    // elevated_button_test.dart 'Fixed size ElevatedButton, same as minimumSize == maximumSize'.
    #[test]
    fn a_fixed_size_sizes_like_equal_minimum_and_maximum_sizes() {
        test_widgets(|tester| {
            let fixed = GlobalKey::new();
            let bounded = GlobalKey::new();
            let theme = ThemeData::new(&tester.cell.borrow());
            let frame = themed(
                tester,
                theme,
                Column::new().main_axis_size(MainAxisSize::Min).children([
                    TestButton::new(Some(pressed()), sized(20.0, 10.0))
                        .key(fixed.clone().into())
                        .style(ButtonStyle::new().fixed_size(all(Size::new(200.0, 200.0))))
                        .into_widget(),
                    TestButton::new(Some(pressed()), sized(20.0, 10.0))
                        .key(bounded.clone().into())
                        .style(
                            ButtonStyle::new()
                                .minimum_size(all(Size::new(200.0, 200.0)))
                                .maximum_size(all(Size::new(200.0, 200.0))),
                        )
                        .into_widget(),
                ]),
            );
            tester.pump_widget(frame);

            assert_eq!(tester.get_size(&fixed), Size::new(200.0, 200.0));
            assert_eq!(tester.get_size(&bounded), Size::new(200.0, 200.0));
        });
    }

    // elevated_button_test.dart 'ElevatedButton statesController'.
    #[test]
    fn a_states_controller_follows_hover_press_and_disable() {
        test_widgets(|tester| {
            let count = Rc::new(Cell::new(0));
            let controller = MaterialStatesController::new(&mut tester.cell.borrow_mut(), None);
            controller.add_listener(&mut tester.cell.borrow_mut(), counter(&count));
            let key = GlobalKey::new();
            let build_frame = |tester: &WidgetTester, enabled: bool| {
                let theme = ThemeData::new(&tester.cell.borrow());
                themed(
                    tester,
                    theme,
                    Center::new().child(
                        TestButton::new(enabled.then(pressed), sized(40.0, 10.0))
                            .key(key.clone().into())
                            .states_controller(controller.clone()),
                    ),
                )
            };

            let frame = build_frame(tester, true);
            tester.pump_widget(frame);

            assert_eq!(value(tester, controller.handle()), states(&[]));
            assert_eq!(count.get(), 0);

            let center = tester.get_center(&key);
            let mut gesture: TestPointer = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, center, false);
            tester.pump_and_settle();

            assert_eq!(
                value(tester, controller.handle()),
                states(&[WidgetState::Hovered])
            );
            assert_eq!(count.get(), 1);

            tester.move_to(&mut gesture, Offset::ZERO, false);
            tester.pump_and_settle();

            assert_eq!(value(tester, controller.handle()), states(&[]));
            assert_eq!(count.get(), 2);

            tester.move_to(&mut gesture, center, false);
            tester.pump_and_settle();

            assert_eq!(
                value(tester, controller.handle()),
                states(&[WidgetState::Hovered])
            );
            assert_eq!(count.get(), 3);

            tester.send(&gesture, waft_embedder::PointerChange::Down);
            tester.pump_and_settle();

            assert_eq!(
                value(tester, controller.handle()),
                states(&[WidgetState::Hovered, WidgetState::Pressed])
            );
            assert_eq!(count.get(), 4);

            tester.up(&gesture);
            tester.pump_and_settle();

            assert_eq!(
                value(tester, controller.handle()),
                states(&[WidgetState::Hovered])
            );
            assert_eq!(count.get(), 5);

            tester.move_to(&mut gesture, Offset::ZERO, false);
            tester.pump_and_settle();

            assert_eq!(value(tester, controller.handle()), states(&[]));
            assert_eq!(count.get(), 6);

            tester.down(&mut gesture, center);
            tester.pump_and_settle();
            assert_eq!(
                value(tester, controller.handle()),
                states(&[WidgetState::Hovered, WidgetState::Pressed])
            );
            assert_eq!(count.get(), 8); // adds hovered and pressed - two changes

            // If the button is rebuilt disabled, then the pressed state is
            // removed.
            let frame = build_frame(tester, false);
            tester.pump_widget(frame);
            tester.pump_and_settle();
            assert_eq!(
                value(tester, controller.handle()),
                states(&[WidgetState::Hovered, WidgetState::Disabled])
            );
            assert_eq!(count.get(), 10); // removes pressed and adds disabled - two changes
            tester.move_to(&mut gesture, Offset::ZERO, true);
            tester.pump_and_settle();
            assert_eq!(
                value(tester, controller.handle()),
                states(&[WidgetState::Disabled])
            );
            assert_eq!(count.get(), 11);
        });
    }

    // elevated_button_test.dart 'Disabled ElevatedButton statesController'.
    #[test]
    fn a_disabled_button_marks_its_states_controller_disabled() {
        test_widgets(|tester| {
            let count = Rc::new(Cell::new(0));
            let controller = MaterialStatesController::new(&mut tester.cell.borrow_mut(), None);
            controller.add_listener(&mut tester.cell.borrow_mut(), counter(&count));
            let theme = ThemeData::new(&tester.cell.borrow());
            let frame = themed(
                tester,
                theme,
                Center::new().child(
                    TestButton::new(None, sized(40.0, 10.0)).states_controller(controller.clone()),
                ),
            );
            tester.pump_widget(frame);
            assert_eq!(
                value(tester, controller.handle()),
                states(&[WidgetState::Disabled])
            );
            assert_eq!(count.get(), 1);
        });
    }

    // elevated_button_test.dart 'ElevatedButton isSelected updates a provided statesController'.
    #[test]
    fn is_selected_writes_the_selected_state_of_a_provided_controller() {
        test_widgets(|tester| {
            let controller = MaterialStatesController::new(&mut tester.cell.borrow_mut(), None);
            let build_button = |tester: &WidgetTester, is_selected: Option<bool>| {
                let mut button = TestButton::new(Some(pressed()), sized(40.0, 10.0))
                    .states_controller(controller.clone());
                button.button_style_button.is_selected = is_selected;
                let theme = ThemeData::new(&tester.cell.borrow());
                themed(tester, theme, Center::new().child(button))
            };

            let frame = build_button(tester, Some(true));
            tester.pump_widget(frame);
            assert!(value(tester, controller.handle()).contains(&WidgetState::Selected));

            let frame = build_button(tester, Some(false));
            tester.pump_widget(frame);
            assert!(!value(tester, controller.handle()).contains(&WidgetState::Selected));

            let frame = build_button(tester, Some(true));
            tester.pump_widget(frame);
            assert!(value(tester, controller.handle()).contains(&WidgetState::Selected));

            let frame = build_button(tester, None);
            tester.pump_widget(frame);
            assert!(!value(tester, controller.handle()).contains(&WidgetState::Selected));
        });
    }

    fn decorated(color: Color, child: Option<WidgetRef>) -> WidgetRef {
        let mut decorated = DecoratedBox::new(BoxDecoration::new().color(color));
        decorated.child = child;
        decorated.into_widget()
    }

    fn decoration_colors(tester: &WidgetTester) -> Vec<Option<Color>> {
        let decorations = tester.all::<RenderDecoratedBox>();
        let app = tester.cell.borrow();
        decorations
            .into_iter()
            .map(|render| {
                render
                    .decoration(&app)
                    .as_any()
                    .downcast_ref::<BoxDecoration>()
                    .expect("a box decoration")
                    .color
                    .as_ref()
                    .map(|color| color.color())
            })
            .collect()
    }

    fn is_inside(tester: &WidgetTester, ancestor: AnyRenderObject, key: &GlobalKey) -> bool {
        let app = tester.cell.borrow();
        let mut node = key
            .current_context(&app)
            .and_then(|context| context.find_render_object(&app));
        while let Some(current) = node {
            if current == ancestor {
                return true;
            }
            node = current.parent(&app);
        }
        false
    }

    // elevated_button_test.dart 'ElevatedButton backgroundBuilder and foregroundBuilder'.
    #[test]
    fn the_layer_builders_wrap_the_button_child_behind_and_in_front() {
        test_widgets(|tester| {
            let background_color = Color::new(0xFF000011);
            let foreground_color = Color::new(0xFF000022);
            let child_key = GlobalKey::new();

            tester.pump_widget(ltr(TestButton::new(
                Some(pressed()),
                sized(40.0, 10.0).key(child_key.clone().into()),
            )
            .style(
                ButtonStyle::new()
                    .background_builder(ButtonLayerBuilder::new(
                        move |_app, _context, _states, child| decorated(background_color, child),
                    ))
                    .foreground_builder(ButtonLayerBuilder::new(
                        move |_app, _context, _states, child| decorated(foreground_color, child),
                    )),
            )));

            assert_eq!(
                decoration_colors(tester),
                vec![Some(background_color), Some(foreground_color)]
            );
            let decorations = tester.all::<RenderDecoratedBox>();
            assert!(is_inside(tester, decorations[0].as_object(), &child_key));
            assert!(is_inside(tester, decorations[1].as_object(), &child_key));
        });
    }

    // elevated_button_test.dart 'ElevatedButton backgroundBuilder drops button child and
    // foregroundBuilder return value'.
    #[test]
    fn a_background_builder_that_drops_its_child_drops_the_whole_foreground() {
        test_widgets(|tester| {
            let background_color = Color::new(0xFF000011);
            let foreground_color = Color::new(0xFF000022);
            let child_key = GlobalKey::new();

            tester.pump_widget(ltr(TestButton::new(
                Some(pressed()),
                sized(40.0, 10.0).key(child_key.clone().into()),
            )
            .style(
                ButtonStyle::new()
                    .background_builder(ButtonLayerBuilder::new(
                        move |_app, _context, _states, _child| decorated(background_color, None),
                    ))
                    .foreground_builder(ButtonLayerBuilder::new(
                        move |_app, _context, _states, _child| decorated(foreground_color, None),
                    )),
            )));

            assert_eq!(decoration_colors(tester), vec![Some(background_color)]);
            assert!(
                child_key
                    .current_context(&tester.cell.borrow_mut())
                    .is_none()
            );
        });
    }

    // elevated_button_test.dart 'ElevatedButton foregroundBuilder drops button child'.
    #[test]
    fn a_foreground_builder_that_drops_its_child_drops_the_button_child() {
        test_widgets(|tester| {
            let foreground_color = Color::new(0xFF000022);
            let child_key = GlobalKey::new();

            tester.pump_widget(ltr(TestButton::new(
                Some(pressed()),
                sized(40.0, 10.0).key(child_key.clone().into()),
            )
            .style(
                ButtonStyle::new().foreground_builder(ButtonLayerBuilder::new(
                    move |_app, _context, _states, _child| decorated(foreground_color, None),
                )),
            )));

            assert_eq!(decoration_colors(tester), vec![Some(foreground_color)]);
            assert!(
                child_key
                    .current_context(&tester.cell.borrow_mut())
                    .is_none()
            );
        });
    }

    // elevated_button_test.dart 'ElevatedButton foreground and background builders are
    // applied to the correct states'.
    #[test]
    fn the_layer_builders_see_the_button_states() {
        test_widgets(|tester| {
            let foreground_states = Rc::new(RefCell::new(WidgetStates::new()));
            let background_states = Rc::new(RefCell::new(WidgetStates::new()));
            let focus_node = FocusNode::new(&mut tester.cell.borrow_mut())
                .into_erased::<waft_widgets::AnyFocusNode>();
            let key = GlobalKey::new();
            let record = |seen: &Rc<RefCell<WidgetStates>>| {
                let seen = Rc::clone(seen);
                ButtonLayerBuilder::new(move |_app, _context, states, child| {
                    *seen.borrow_mut() = states.clone();
                    child.expect("the button's child")
                })
            };
            let theme = ThemeData::new(&tester.cell.borrow());
            let frame = themed(
                tester,
                theme,
                Center::new().child(
                    TestButton::new(Some(pressed()), sized(40.0, 10.0))
                        .key(key.clone().into())
                        .focus_node(focus_node.clone())
                        .style(
                            ButtonStyle::new()
                                .background_builder(record(&background_states))
                                .foreground_builder(record(&foreground_states)),
                        ),
                ),
            );
            tester.pump_widget(frame);

            // Default.
            assert!(background_states.borrow().is_empty());
            assert!(foreground_states.borrow().is_empty());

            let focused_states = states(&[WidgetState::Focused]);
            let focused_hovered_states = states(&[WidgetState::Focused, WidgetState::Hovered]);
            let focused_hovered_pressed_states = states(&[
                WidgetState::Focused,
                WidgetState::Hovered,
                WidgetState::Pressed,
            ]);

            // Focused.
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert_eq!(*background_states.borrow(), focused_states);
            assert_eq!(*foreground_states.borrow(), focused_states);

            // Hovered.
            let center = tester.get_center(&key);
            let mut gesture = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, center, false);
            tester.pump_and_settle();
            assert_eq!(*background_states.borrow(), focused_hovered_states);
            assert_eq!(*foreground_states.borrow(), focused_hovered_states);

            // Highlighted (pressed).
            tester.send(&gesture, waft_embedder::PointerChange::Down);
            tester.pump(); // Start the splash and highlight animations.
            // Wait for splash and highlight to be well under way.
            tester.pump_by(Duration::from_millis(800));
            assert_eq!(*background_states.borrow(), focused_hovered_pressed_states);
            assert_eq!(*foreground_states.borrow(), focused_hovered_pressed_states);
        });
    }

    /// The colours a probe in the button's child reads: the ambient icon theme's and the
    /// default text style's.
    type Seen = Rc<RefCell<(Option<Color>, Option<Color>)>>;

    fn color_probe(seen: &Seen) -> Builder {
        let seen = Rc::clone(seen);
        Builder::new(move |app, context| {
            let icon_color = IconTheme::of(app, context).color.map(|color| color.color());
            let text_color = DefaultTextStyle::of(app, context)
                .style
                .color
                .map(|color| color.color());
            *seen.borrow_mut() = (icon_color, text_color);
            sized(40.0, 10.0).into_widget()
        })
    }

    // elevated_button_test.dart 'ElevatedButton icon uses provided foregroundColor over
    // default icon color'.
    #[test]
    fn the_icon_color_falls_back_to_the_given_foreground_color() {
        test_widgets(|tester| {
            let foreground_color = Color::new(0xFFFF1234);
            let seen: Seen = Rc::default();
            let theme = ThemeData::new(&tester.cell.borrow());
            let frame = themed(
                tester,
                theme,
                Center::new().child(
                    TestButton::new(Some(pressed()), color_probe(&seen))
                        .style(ButtonStyle::new().foreground_color(all(foreground_color))),
                ),
            );
            tester.pump_widget(frame);
            assert_eq!(seen.borrow().0, Some(foreground_color));
        });
    }

    // elevated_button_test.dart 'ElevatedButton text and icon respect animation duration'.
    #[test]
    fn the_text_and_icon_colors_animate_over_the_animation_duration() {
        test_widgets(|tester| {
            let hovered_color = Color::new(0xFFFF0000);
            let idle_color = Color::new(0xFF000000);
            let halfway = hovered_color.with_values(None, Some(0.5), None, None, None);
            let seen: Seen = Rc::default();
            let key = GlobalKey::new();
            let color = || {
                <dyn WidgetStateProperty<Option<Color>>>::from_map(vec![
                    (WidgetState::Hovered.into(), Some(hovered_color)),
                    (WidgetState::any(), Some(idle_color)),
                ])
            };
            let build_button = |tester: &WidgetTester, animation_duration: Option<Duration>| {
                let mut style = ButtonStyle::new()
                    .icon_color(color())
                    .foreground_color(color());
                style.animation_duration = animation_duration;
                let theme = ThemeData::new(&tester.cell.borrow());
                themed(
                    tester,
                    theme,
                    Material::new().child(
                        Center::new().child(
                            TestButton::new(Some(pressed()), color_probe(&seen))
                                .key(key.clone().into())
                                .style(style),
                        ),
                    ),
                )
            };

            // Test default animation duration.
            let frame = build_button(tester, None);
            tester.pump_widget(frame);

            assert_eq!(*seen.borrow(), (Some(idle_color), Some(idle_color)));

            let button_center = tester.get_center(&key);
            let mut gesture = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, button_center, false);

            tester.pump();
            tester.pump_by(Duration::from_millis(100));
            assert_eq!(*seen.borrow(), (Some(halfway), Some(halfway)));

            tester.pump();
            tester.pump_by(Duration::from_millis(200));
            assert_eq!(*seen.borrow(), (Some(hovered_color), Some(hovered_color)));

            tester.send(&gesture, waft_embedder::PointerChange::Remove);

            // Test custom animation duration.
            let frame = build_button(tester, Some(Duration::from_secs(2)));
            tester.pump_widget(frame);
            tester.pump_and_settle();

            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, button_center, false);

            tester.pump();
            tester.pump_by(Duration::from_secs(1));
            assert_eq!(*seen.borrow(), (Some(halfway), Some(halfway)));

            tester.pump();
            tester.pump_by(Duration::from_secs(1));
            assert_eq!(*seen.borrow(), (Some(hovered_color), Some(hovered_color)));
        });
    }

    // elevated_button_test.dart 'ElevatedButton does not crash at zero area'.
    #[test]
    fn a_button_in_a_zero_area_has_zero_size() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let theme = ThemeData::new(&tester.cell.borrow());
            let frame = themed(
                tester,
                theme,
                Center::new().child(SizedBox::shrink().child(
                    TestButton::new(Some(pressed()), sized(10.0, 10.0)).key(key.clone().into()),
                )),
            );
            tester.pump_widget(frame);
            assert_eq!(tester.get_size(&key), Size::ZERO);
        });
    }

    #[test]
    fn a_tap_in_the_padded_tap_target_reaches_the_button() {
        test_widgets(|tester| {
            let taps = Rc::new(Cell::new(0));
            let key = GlobalKey::new();
            let frame = themed(
                tester,
                material2(tester),
                Center::new().child(
                    TestButton::new(Some(counter(&taps)), sized(100.0, 20.0))
                        .key(key.clone().into()),
                ),
            );
            tester.pump_widget(frame);
            // The button's material is 36 high and centred in the 48 high tap target.
            let rect = rect_of(tester, &key);
            assert_eq!(rect.height(), 48.0);
            tester.tap_at(Offset::new(rect.center().dx(), rect.top + 2.0), 1);
            tester.pump_and_settle();
            assert_eq!(taps.get(), 1);
        });
    }

    /// The `Semantics` around each button's `_InputPadding` bounds its hit test, so the padding
    /// of the later button in a column does not take a tap on the earlier one.
    #[test]
    fn a_tap_on_the_first_of_two_buttons_in_a_column_reaches_it() {
        test_widgets(|tester| {
            let first_taps = Rc::new(Cell::new(0));
            let second_taps = Rc::new(Cell::new(0));
            let first = GlobalKey::new();
            let frame = themed(
                tester,
                material2(tester),
                Center::new().child(
                    Column::new().main_axis_size(MainAxisSize::Min).children([
                        TestButton::new(Some(counter(&first_taps)), sized(100.0, 20.0))
                            .key(first.clone().into())
                            .into_widget(),
                        TestButton::new(Some(counter(&second_taps)), sized(100.0, 20.0))
                            .into_widget(),
                    ]),
                ),
            );
            tester.pump_widget(frame);
            let rect = rect_of(tester, &first);
            tester.tap_at(rect.center(), 1);
            tester.pump_and_settle();
            assert_eq!(first_taps.get(), 1);
            assert_eq!(second_taps.get(), 0);
        });
    }

    #[test]
    fn a_button_wraps_its_input_padding_in_a_container_semantics() {
        test_widgets(|tester| {
            let frame = themed(
                tester,
                material2(tester),
                Center::new()
                    .child(TestButton::new(Some(pressed()), sized(100.0, 20.0)).is_selected(true)),
            );
            tester.pump_widget(frame);
            let semantics = tester.first::<waft_rendering::RenderSemanticsAnnotations>();
            let app = tester.cell.borrow();
            use waft_rendering::SemanticsAnnotationsMixin;
            assert!(semantics.container(&app));
            let properties = semantics.properties(&app);
            assert_eq!(properties.button, Some(true));
            assert_eq!(properties.enabled, Some(true));
            assert_eq!(properties.selected, Some(true));
        });
    }
}
