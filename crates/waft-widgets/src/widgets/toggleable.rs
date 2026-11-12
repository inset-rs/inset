//! Flutter counterpart: `widgets/toggleable.dart`.
//!
//! [`ToggleableStateMixin`] is a mixin with fields ([`ToggleableStateMixinData`]) on a state
//! that is also a [`TickerProviderStateMixin`]. [`ToggleablePainter`] is a base class whose
//! fields are [`ToggleablePainterData`]; a painter is an arena `ChangeNotifier` and a
//! `CustomPaint` reaches it through [`ToggleablePainterRef`]. The `Semantics(enabled:)` wrapper
//! and the tap's `TapSemanticEvent` wait with accessibility.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animation, AnimationBehavior, AnimationController, AnyAnimation, CurvedAnimation, Curves,
};
use waft_embedder::{Canvas, Color, Offset, Paint, Size};
use waft_foundation::{App, ChangeNotifier, Handle, Listenable, Listener, ValueChanged};
use waft_gestures::{TapDownDetails, TapUpDetails};
use waft_rendering::CustomPainter;
use waft_scheduler::TickerProviderObject;
use waft_services::{MouseCursorRef, SystemMouseCursors};

use crate::framework::{IntoWidget, WidgetRef};
use crate::widgets::actions::{
    Action, ActivateIntent, AnyAction, CallbackAction, FocusableActionDetector,
};
use crate::widgets::basic::CustomPaint;
use crate::widgets::focus_manager::AnyFocusNode;
use crate::widgets::gesture_detector::GestureDetector;
use crate::widgets::ticker_provider::TickerProviderStateMixin;
use crate::widgets::widget_state::{WidgetState, WidgetStatePropertyRef, WidgetStates};

// Duration of the animation that moves the toggle from one state to another.
const K_TOGGLE_DURATION: Duration = Duration::from_millis(200);

// Duration of the fade animation for the reaction when focus and hover occur.
const K_REACTION_FADE_DURATION: Duration = Duration::from_millis(50);

// Default duration of the radial reaction animation.
const K_REACTION_ANIMATION_DURATION: Duration = Duration::from_millis(100);

/// The fields of Dart's `ToggleableStateMixin`, which a state holds under the field
/// `toggleable`.
///
/// The animations and the action are arena objects, created in
/// [`ToggleableStateMixin::init_state`] (Dart's `late` fields and `late final _actionMap`).
#[derive(Default)]
pub struct ToggleableStateMixinData {
    position_controller: Option<Handle<AnimationController>>,
    position: Option<Handle<CurvedAnimation>>,
    reaction_controller: Option<Handle<AnimationController>>,
    reaction: Option<Handle<CurvedAnimation>>,
    reaction_hover_fade: Option<Handle<CurvedAnimation>>,
    reaction_hover_fade_controller: Option<Handle<AnimationController>>,
    reaction_focus_fade: Option<Handle<CurvedAnimation>>,
    reaction_focus_fade_controller: Option<Handle<AnimationController>>,
    action_map: HashMap<TypeId, AnyAction>,
    down_position: Option<Offset>,
    focused: bool,
    hovering: bool,
}

impl ToggleableStateMixinData {
    /// The bag of a state whose `init_state` has not run yet.
    pub fn new() -> ToggleableStateMixinData {
        ToggleableStateMixinData::default()
    }
}

/// A mixin for `StatefulWidget`s that implement toggleable controls with toggle animations
/// (e.g. `Switch`es, `CupertinoSwitch`es, `Checkbox`es, `CupertinoCheckbox`es, `Radio`s, and
/// `CupertinoRadio`s).
///
/// The mixin implements the logic for toggling the control (e.g. when tapped) and provides a
/// series of animation controllers to transition the control from one state to another. It
/// does not have any opinion about the visual representation of the toggleable widget. The
/// visuals are defined by a [`CustomPainter`] passed to the
/// [`build_toggleable`](Self::build_toggleable). [`State`](crate::State) objects using this
/// mixin should call that method from their `build` method.
///
/// The state holds a [`ToggleableStateMixinData`] under the field `toggleable`, and its
/// `init_state` and `dispose` call this mixin's where Dart's `super` calls reach it:
/// [`init_state`](Self::init_state) first, then [`dispose`](Self::dispose) followed by
/// [`TickerProviderStateMixin::dispose`].
pub trait ToggleableStateMixin: TickerProviderStateMixin + TickerProviderObject {
    /// Mixin field access.
    fn toggleable_data(self: Handle<Self>, app: &App) -> &ToggleableStateMixinData;

    /// See [`toggleable_data`](Self::toggleable_data).
    fn toggleable_data_mut(self: Handle<Self>, app: &mut App) -> &mut ToggleableStateMixinData;

    /// Used by subclasses to manipulate the visual value of the control.
    ///
    /// Some controls respond to user input by updating their visual value. For example, the
    /// thumb of a switch moves from one position to another when dragged. These controls
    /// manipulate this animation controller to update their [`position`](Self::position) and
    /// eventually trigger an [`on_changed`](Self::on_changed) callback when the animation
    /// reaches either 0.0 or 1.0.
    fn position_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        self.toggleable_data(app)
            .position_controller
            .expect("created in init_state")
    }

    /// The visual value of the control.
    ///
    /// When the control is inactive, the [`value`](Self::value) is false and this animation
    /// has the value 0.0. When the control is active, the value is either true or tristate is
    /// true and the value is null. When the control is active the animation has a value of
    /// 1.0. When the control is changing from inactive to active (or vice versa),
    /// [`value`](Self::value) is the target value and this animation gradually updates from
    /// 0.0 to 1.0 (or vice versa).
    fn position(self: Handle<Self>, app: &App) -> Handle<CurvedAnimation> {
        self.toggleable_data(app)
            .position
            .expect("created in init_state")
    }

    /// Used by subclasses to control the radial reaction animation.
    ///
    /// Some controls have a radial ink reaction to user input. This animation controller can
    /// be used to start or stop these ink reactions.
    ///
    /// To paint the actual radial reaction, [`ToggleablePainter::paint_radial_reaction`] may
    /// be used.
    fn reaction_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        self.toggleable_data(app)
            .reaction_controller
            .expect("created in init_state")
    }

    /// The visual value of the radial reaction animation.
    ///
    /// Some controls have a radial ink reaction to user input. This animation controls the
    /// progress of these ink reactions.
    ///
    /// To paint the actual radial reaction, [`ToggleablePainter::paint_radial_reaction`] may
    /// be used.
    fn reaction(self: Handle<Self>, app: &App) -> Handle<CurvedAnimation> {
        self.toggleable_data(app)
            .reaction
            .expect("created in init_state")
    }

    /// Controls the radial reaction's opacity animation for hover changes.
    ///
    /// Some controls have a radial ink reaction to pointer hover. This animation controls
    /// these ink reaction fade-ins and fade-outs.
    ///
    /// To paint the actual radial reaction, [`ToggleablePainter::paint_radial_reaction`] may
    /// be used.
    fn reaction_hover_fade(self: Handle<Self>, app: &App) -> Handle<CurvedAnimation> {
        self.toggleable_data(app)
            .reaction_hover_fade
            .expect("created in init_state")
    }

    /// Controls the radial reaction's opacity animation for focus changes.
    ///
    /// Some controls have a radial ink reaction to focus. This animation controls these ink
    /// reaction fade-ins and fade-outs.
    ///
    /// To paint the actual radial reaction, [`ToggleablePainter::paint_radial_reaction`] may
    /// be used.
    fn reaction_focus_fade(self: Handle<Self>, app: &App) -> Handle<CurvedAnimation> {
        self.toggleable_data(app)
            .reaction_focus_fade
            .expect("created in init_state")
    }

    /// The amount of time a circular ink response should take to expand to its full size if a
    /// radial reaction is drawn using [`ToggleablePainter::paint_radial_reaction`].
    fn reaction_animation_duration(self: Handle<Self>, app: &App) -> Option<Duration> {
        let _ = app;
        Some(K_REACTION_ANIMATION_DURATION)
    }

    /// Whether [`value`](Self::value) of this control can be changed by user interaction.
    ///
    /// The control is considered interactive if the [`on_changed`](Self::on_changed) callback
    /// is non-null. If the callback is null, then the control is disabled, and
    /// non-interactive. A disabled checkbox, for example, is displayed using a grey color and
    /// its value cannot be changed.
    fn is_interactive(self: Handle<Self>, app: &App) -> bool {
        self.on_changed(app).is_some()
    }

    /// Called when the control changes value.
    ///
    /// If the control is tapped, [`on_changed`](Self::on_changed) is called immediately with
    /// the new value.
    ///
    /// The control is considered interactive (see [`is_interactive`](Self::is_interactive)) if
    /// this callback is non-null. If the callback is null, then the control is disabled, and
    /// non-interactive. A disabled checkbox, for example, is displayed using a grey color and
    /// its value cannot be changed.
    fn on_changed(self: Handle<Self>, app: &App) -> Option<ValueChanged<Option<bool>>>;

    /// False if this control is "inactive" (not checked, off, or unselected).
    ///
    /// If value is true then the control "active" (checked, on, or selected). If tristate is
    /// true and value is null, then the control is considered to be in its third or
    /// "indeterminate" state.
    ///
    /// When the value changes, this object starts the
    /// [`position_controller`](Self::position_controller) and [`position`](Self::position)
    /// animations to animate the visual appearance of the control to the new value.
    fn value(self: Handle<Self>, app: &App) -> Option<bool>;

    /// If true, [`value`](Self::value) can be true, false, or null, otherwise
    /// [`value`](Self::value) must be true or false.
    ///
    /// When [`tristate`](Self::tristate) is true and [`value`](Self::value) is null, then the
    /// control is considered to be in its third or "indeterminate" state.
    fn tristate(self: Handle<Self>, app: &App) -> bool;

    /// The mixin's part of `State::init_state`; the state calls it first, where Dart's
    /// `super.initState()` reaches the mixin.
    fn init_state(self: Handle<Self>, app: &mut App) {
        let position_value = if self.value(app) == Some(false) {
            0.0
        } else {
            1.0
        };
        let position_controller = AnimationController::create(
            app,
            Some(position_value),
            Some(K_TOGGLE_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let position = CurvedAnimation::create(
            app,
            position_controller.as_animation(),
            Curves::ease_in(),
            Some(Curves::ease_out()),
        );
        let reaction_duration = self
            .reaction_animation_duration(app)
            .unwrap_or(K_REACTION_ANIMATION_DURATION);
        let reaction_controller = AnimationController::create(
            app,
            None,
            Some(reaction_duration),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let reaction = CurvedAnimation::create(
            app,
            reaction_controller.as_animation(),
            Curves::fast_out_slow_in(),
            None,
        );
        let fade_value = {
            let data = self.toggleable_data(app);
            if data.hovering || data.focused {
                1.0
            } else {
                0.0
            }
        };
        let reaction_hover_fade_controller = AnimationController::create(
            app,
            Some(fade_value),
            Some(K_REACTION_FADE_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let reaction_hover_fade = CurvedAnimation::create(
            app,
            reaction_hover_fade_controller.as_animation(),
            Curves::fast_out_slow_in(),
            None,
        );
        let reaction_focus_fade_controller = AnimationController::create(
            app,
            Some(fade_value),
            Some(K_REACTION_FADE_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let reaction_focus_fade = CurvedAnimation::create(
            app,
            reaction_focus_fade_controller.as_animation(),
            Curves::fast_out_slow_in(),
            None,
        );
        let activate = CallbackAction::<ActivateIntent>::new(
            app,
            Rc::new(move |app: &mut App, _intent: &ActivateIntent| {
                self.handle_tap(app);
                None
            }),
        );
        let data = self.toggleable_data_mut(app);
        data.position_controller = Some(position_controller);
        data.position = Some(position);
        data.reaction_controller = Some(reaction_controller);
        data.reaction = Some(reaction);
        data.reaction_hover_fade_controller = Some(reaction_hover_fade_controller);
        data.reaction_hover_fade = Some(reaction_hover_fade);
        data.reaction_focus_fade_controller = Some(reaction_focus_fade_controller);
        data.reaction_focus_fade = Some(reaction_focus_fade);
        data.action_map
            .insert(TypeId::of::<ActivateIntent>(), activate.as_action());
    }

    /// Runs the [`position`](Self::position) animation to transition the Toggleable's
    /// appearance to match [`value`](Self::value).
    ///
    /// This method must be called whenever [`value`](Self::value) changes to ensure that the
    /// visual representation of the Toggleable matches the current [`value`](Self::value).
    fn animate_to_value(self: Handle<Self>, app: &mut App) {
        let position_controller = self.position_controller(app);
        let value = self.value(app);
        if self.tristate(app) {
            if value.is_none() {
                position_controller.set_value(app, 0.0);
            }
            if value.unwrap_or(true) {
                position_controller.forward(app, None);
            } else {
                position_controller.reverse(app, None);
            }
        } else if value.unwrap_or(false) {
            position_controller.forward(app, None);
        } else {
            position_controller.reverse(app, None);
        }
    }

    /// The mixin's part of `State::dispose`; the state calls it where Dart's `super.dispose()`
    /// reaches the mixin, then [`TickerProviderStateMixin::dispose`].
    fn dispose(self: Handle<Self>, app: &mut App) {
        let data = std::mem::take(self.toggleable_data_mut(app));
        let pairs = [
            (data.position_controller, data.position),
            (data.reaction_controller, data.reaction),
            (
                data.reaction_hover_fade_controller,
                data.reaction_hover_fade,
            ),
            (
                data.reaction_focus_fade_controller,
                data.reaction_focus_fade,
            ),
        ];
        for (controller, animation) in pairs {
            let controller = controller.expect("created in init_state");
            let animation = animation.expect("created in init_state");
            controller.dispose(app);
            animation.dispose(app);
            app.destroy(animation);
            app.destroy(controller);
        }
        for action in data.action_map.into_values() {
            app.destroy(action.id());
        }
    }

    /// The most recent [`Offset`] at which a pointer touched the Toggleable.
    ///
    /// This is null if currently no pointer is touching the Toggleable or if
    /// [`is_interactive`](Self::is_interactive) is false.
    fn down_position(self: Handle<Self>, app: &App) -> Option<Offset> {
        self.toggleable_data(app).down_position
    }

    /// Describes the current [`WidgetState`] of the Toggleable.
    ///
    /// The returned set will include:
    ///
    ///  * [`WidgetState::Disabled`], if [`is_interactive`](Self::is_interactive) is false
    ///  * [`WidgetState::Hovered`], if a pointer is hovering over the Toggleable
    ///  * [`WidgetState::Focused`], if the Toggleable has input focus
    ///  * [`WidgetState::Selected`], if [`value`](Self::value) is true or null
    fn states(self: Handle<Self>, app: &App) -> WidgetStates {
        let mut states = WidgetStates::new();
        if !self.is_interactive(app) {
            states.insert(WidgetState::Disabled);
        }
        if self.toggleable_data(app).hovering {
            states.insert(WidgetState::Hovered);
        }
        if self.toggleable_data(app).focused {
            states.insert(WidgetState::Focused);
        }
        if self.value(app).unwrap_or(true) {
            states.insert(WidgetState::Selected);
        }
        states
    }

    /// Typically wraps a `painter` that draws the actual visuals of the Toggleable with logic
    /// to toggle it.
    ///
    /// Use [`build_toggleable_with_child`](Self::build_toggleable_with_child) if one would
    /// like to provide a widget instead of [`CustomPainter`].
    ///
    /// If drawing a radial ink reaction is desired (in Material Design for example), consider
    /// providing a [`ToggleablePainter`] as a `painter`, which implements logic to draw a
    /// radial ink reaction for this control. The painter is usually configured with the
    /// [`reaction`](Self::reaction), [`position`](Self::position),
    /// [`reaction_hover_fade`](Self::reaction_hover_fade), and
    /// [`reaction_focus_fade`](Self::reaction_focus_fade) animation provided by this mixin. It
    /// is expected to draw the visuals of the Toggleable based on the current value of these
    /// animations. The animations are triggered by this mixin to transition the Toggleable
    /// from one state to another.
    ///
    /// Material Toggleables must provide a `mouse_cursor` which resolves to a mouse cursor
    /// based on the current [`WidgetState`] of the Toggleable. Cupertino Toggleables may not
    /// provide a `mouse_cursor`. If no `mouse_cursor` is provided,
    /// [`SystemMouseCursors::BASIC`] will be used as the `mouse_cursor` across all
    /// [`WidgetState`]s.
    ///
    /// This method must be called from the `build` method of the state that uses this mixin.
    /// The returned widget must be returned from the build method - potentially after wrapping
    /// it in other widgets.
    #[allow(clippy::too_many_arguments)]
    fn build_toggleable(
        self: Handle<Self>,
        app: &mut App,
        focus_node: Option<AnyFocusNode>,
        on_focus_change: Option<ValueChanged<bool>>,
        autofocus: bool,
        mouse_cursor: Option<WidgetStatePropertyRef<MouseCursorRef>>,
        size: Size,
        painter: impl CustomPainter,
    ) -> WidgetRef {
        self.build_toggleable_with_child(
            app,
            focus_node,
            on_focus_change,
            autofocus,
            mouse_cursor,
            CustomPaint::new().size(size).painter(painter).into_widget(),
        )
    }

    /// Typically wraps a child that draws the actual visuals of the Toggleable with logic to
    /// toggle it.
    ///
    /// If drawing a radial ink reaction is desired (in Material Design for example), consider
    /// providing a [`CustomPaint`] with a [`ToggleablePainter`] as its painter, which
    /// implements logic to draw a radial ink reaction for this control. The painter is usually
    /// configured with the [`reaction`](Self::reaction), [`position`](Self::position),
    /// [`reaction_hover_fade`](Self::reaction_hover_fade), and
    /// [`reaction_focus_fade`](Self::reaction_focus_fade) animation provided by this mixin. It
    /// is expected to draw the visuals of the Toggleable based on the current value of these
    /// animations. The animations are triggered by this mixin to transition the Toggleable
    /// from one state to another.
    ///
    /// Material Toggleables must provide a `mouse_cursor` which resolves to a mouse cursor
    /// based on the current [`WidgetState`] of the Toggleable. Cupertino Toggleables may not
    /// provide a `mouse_cursor`. If no `mouse_cursor` is provided,
    /// [`SystemMouseCursors::BASIC`] will be used as the `mouse_cursor` across all
    /// [`WidgetState`]s.
    ///
    /// This method must be called from the `build` method of the state that uses this mixin.
    /// The returned widget must be returned from the build method - potentially after wrapping
    /// it in other widgets.
    fn build_toggleable_with_child(
        self: Handle<Self>,
        app: &mut App,
        focus_node: Option<AnyFocusNode>,
        on_focus_change: Option<ValueChanged<bool>>,
        autofocus: bool,
        mouse_cursor: Option<WidgetStatePropertyRef<MouseCursorRef>>,
        child: WidgetRef,
    ) -> WidgetRef {
        let is_interactive = self.is_interactive(app);
        let effective_mouse_cursor = match mouse_cursor {
            Some(mouse_cursor) => mouse_cursor.resolve(&self.states(app)),
            None => SystemMouseCursors::BASIC.into(),
        };
        let mut detector = FocusableActionDetector::new(self.tap_detector(app).child(child))
            .actions(self.toggleable_data(app).action_map.clone())
            .autofocus(autofocus)
            .enabled(is_interactive)
            .on_show_focus_highlight(move |app, focused| {
                self.handle_focus_highlight_changed(app, focused)
            })
            .on_show_hover_highlight(move |app, hovering| self.handle_hover_changed(app, hovering))
            .mouse_cursor(effective_mouse_cursor);
        if let Some(focus_node) = focus_node {
            detector = detector.focus_node(focus_node);
        }
        if let Some(on_focus_change) = on_focus_change {
            detector = detector.on_focus_change(move |app, focused| on_focus_change(app, focused));
        }
        // `Semantics(enabled: isInteractive)` around the child waits with accessibility.
        detector.into_widget()
    }
}

/// Dart's private members of `ToggleableStateMixin`.
trait ToggleableStateMixinPrivate: ToggleableStateMixin {
    /// The `GestureDetector` Dart builds inline: the tap handlers only while interactive.
    fn tap_detector(self: Handle<Self>, app: &App) -> GestureDetector {
        if !self.is_interactive(app) {
            return GestureDetector::new();
        }
        GestureDetector::new()
            .on_tap_down(Rc::new(move |app: &mut App, details| {
                self.handle_tap_down(app, details)
            }))
            .on_tap(Listener::new(move |app| self.handle_tap(app)))
            .on_tap_up(Rc::new(move |app: &mut App, details| {
                self.handle_tap_end(app, Some(details))
            }))
            .on_tap_cancel(Listener::new(move |app| self.handle_tap_end(app, None)))
    }

    fn handle_tap_down(self: Handle<Self>, app: &mut App, details: TapDownDetails) {
        if self.is_interactive(app) {
            self.toggleable_data_mut(app).down_position = Some(details.local_position);
            self.set_state(app, |_| {});
            self.reaction_controller(app).forward(app, None);
        }
    }

    fn handle_tap(self: Handle<Self>, app: &mut App) {
        if !self.is_interactive(app) {
            return;
        }
        let on_changed = self.on_changed(app).expect("an interactive toggleable");
        match self.value(app) {
            Some(false) => on_changed(app, Some(true)),
            Some(true) => {
                let new_value = if self.tristate(app) {
                    None
                } else {
                    Some(false)
                };
                on_changed(app, new_value);
            }
            None => on_changed(app, Some(false)),
        }
        // `sendSemanticsEvent(const TapSemanticEvent())` waits with accessibility.
    }

    fn handle_tap_end(self: Handle<Self>, app: &mut App, _details: Option<TapUpDetails>) {
        if self.toggleable_data(app).down_position.is_some() {
            self.toggleable_data_mut(app).down_position = None;
            self.set_state(app, |_| {});
        }
        self.reaction_controller(app).reverse(app, None);
    }

    fn handle_focus_highlight_changed(self: Handle<Self>, app: &mut App, focused: bool) {
        if focused != self.toggleable_data(app).focused {
            self.toggleable_data_mut(app).focused = focused;
            self.set_state(app, |_| {});
            let controller = self
                .toggleable_data(app)
                .reaction_focus_fade_controller
                .expect("created in init_state");
            if focused {
                controller.forward(app, None);
            } else {
                controller.reverse(app, None);
            }
        }
    }

    fn handle_hover_changed(self: Handle<Self>, app: &mut App, hovering: bool) {
        if hovering != self.toggleable_data(app).hovering {
            self.toggleable_data_mut(app).hovering = hovering;
            self.set_state(app, |_| {});
            let controller = self
                .toggleable_data(app)
                .reaction_hover_fade_controller
                .expect("created in init_state");
            if hovering {
                controller.forward(app, None);
            } else {
                controller.reverse(app, None);
            }
        }
    }
}

impl<S: ToggleableStateMixin> ToggleableStateMixinPrivate for S {}

/// The fields of Dart's `ToggleablePainter`, which a painter holds under the field
/// `toggleable_painter`. Dart's `late` getters read them with a `!`; here
/// [`ToggleablePainter`]'s getters panic on a field no setter has written.
#[derive(Default)]
pub struct ToggleablePainterData {
    position: Option<AnyAnimation<f64>>,
    reaction: Option<AnyAnimation<f64>>,
    reaction_focus_fade: Option<AnyAnimation<f64>>,
    reaction_hover_fade: Option<AnyAnimation<f64>>,
    active_color: Option<Color>,
    inactive_color: Option<Color>,
    inactive_reaction_color: Option<Color>,
    reaction_color: Option<Color>,
    hover_color: Option<Color>,
    focus_color: Option<Color>,
    splash_radius: Option<f64>,
    down_position: Option<Offset>,
    is_focused: Option<bool>,
    is_hovered: Option<bool>,
    is_active: Option<bool>,
}

impl ToggleablePainterData {
    /// The bag of a painter no setter has configured yet.
    pub fn new() -> ToggleablePainterData {
        ToggleablePainterData::default()
    }
}

/// A base class for a [`CustomPainter`] that may be passed to
/// [`ToggleableStateMixin::build_toggleable`] to draw the visual representation of a
/// Toggleable.
///
/// Implementors must implement the [`paint`](Self::paint) method to draw the actual visuals of
/// the Toggleable.
///
/// If drawing a radial ink reaction is desired (in Material Design for example),
/// implementors may call [`paint_radial_reaction`](Self::paint_radial_reaction) in their
/// [`paint`](Self::paint) method.
///
/// A painter is an arena `ChangeNotifier` holding a [`ToggleablePainterData`] under the field
/// `toggleable_painter`; a `CustomPaint` holds it as a [`ToggleablePainterRef`]. A painter
/// that overrides `dispose` ends its own by calling this trait's
/// [`dispose`](Self::dispose), Dart's `super.dispose()`.
pub trait ToggleablePainter: ChangeNotifier {
    /// Base class field access.
    fn toggleable_painter_data(self: Handle<Self>, app: &App) -> &ToggleablePainterData;

    /// See [`toggleable_painter_data`](Self::toggleable_painter_data).
    fn toggleable_painter_data_mut(self: Handle<Self>, app: &mut App)
    -> &mut ToggleablePainterData;

    /// The visual value of the control.
    ///
    /// Usually set to [`ToggleableStateMixin::position`].
    fn position(self: Handle<Self>, app: &App) -> AnyAnimation<f64> {
        self.toggleable_painter_data(app)
            .position
            .expect("set before painting")
    }

    /// Sets [`position`](Self::position).
    fn set_position(self: Handle<Self>, app: &mut App, value: AnyAnimation<f64>) {
        let old = self.toggleable_painter_data(app).position;
        if old == Some(value) {
            return;
        }
        let notify = Listener::handle_method(self, Handle::<Self>::notify_listeners);
        if let Some(old) = old {
            old.remove_listener(app, &notify);
        }
        value.add_listener(app, notify);
        self.toggleable_painter_data_mut(app).position = Some(value);
        self.notify_listeners(app);
    }

    /// The visual value of the radial reaction animation.
    ///
    /// Usually set to [`ToggleableStateMixin::reaction`].
    fn reaction(self: Handle<Self>, app: &App) -> AnyAnimation<f64> {
        self.toggleable_painter_data(app)
            .reaction
            .expect("set before painting")
    }

    /// Sets [`reaction`](Self::reaction).
    fn set_reaction(self: Handle<Self>, app: &mut App, value: AnyAnimation<f64>) {
        let old = self.toggleable_painter_data(app).reaction;
        if old == Some(value) {
            return;
        }
        let notify = Listener::handle_method(self, Handle::<Self>::notify_listeners);
        if let Some(old) = old {
            old.remove_listener(app, &notify);
        }
        value.add_listener(app, notify);
        self.toggleable_painter_data_mut(app).reaction = Some(value);
        self.notify_listeners(app);
    }

    /// Controls the radial reaction's opacity animation for focus changes.
    ///
    /// Usually set to [`ToggleableStateMixin::reaction_focus_fade`].
    fn reaction_focus_fade(self: Handle<Self>, app: &App) -> AnyAnimation<f64> {
        self.toggleable_painter_data(app)
            .reaction_focus_fade
            .expect("set before painting")
    }

    /// Sets [`reaction_focus_fade`](Self::reaction_focus_fade).
    fn set_reaction_focus_fade(self: Handle<Self>, app: &mut App, value: AnyAnimation<f64>) {
        let old = self.toggleable_painter_data(app).reaction_focus_fade;
        if old == Some(value) {
            return;
        }
        let notify = Listener::handle_method(self, Handle::<Self>::notify_listeners);
        if let Some(old) = old {
            old.remove_listener(app, &notify);
        }
        value.add_listener(app, notify);
        self.toggleable_painter_data_mut(app).reaction_focus_fade = Some(value);
        self.notify_listeners(app);
    }

    /// Controls the radial reaction's opacity animation for hover changes.
    ///
    /// Usually set to [`ToggleableStateMixin::reaction_hover_fade`].
    fn reaction_hover_fade(self: Handle<Self>, app: &App) -> AnyAnimation<f64> {
        self.toggleable_painter_data(app)
            .reaction_hover_fade
            .expect("set before painting")
    }

    /// Sets [`reaction_hover_fade`](Self::reaction_hover_fade).
    fn set_reaction_hover_fade(self: Handle<Self>, app: &mut App, value: AnyAnimation<f64>) {
        let old = self.toggleable_painter_data(app).reaction_hover_fade;
        if old == Some(value) {
            return;
        }
        let notify = Listener::handle_method(self, Handle::<Self>::notify_listeners);
        if let Some(old) = old {
            old.remove_listener(app, &notify);
        }
        value.add_listener(app, notify);
        self.toggleable_painter_data_mut(app).reaction_hover_fade = Some(value);
        self.notify_listeners(app);
    }

    /// The color that should be used in the active state (i.e., when
    /// [`ToggleableStateMixin::value`] is true).
    ///
    /// For example, a checkbox should use this color when checked.
    fn active_color(self: Handle<Self>, app: &App) -> Color {
        self.toggleable_painter_data(app)
            .active_color
            .expect("set before painting")
    }

    /// Sets [`active_color`](Self::active_color).
    fn set_active_color(self: Handle<Self>, app: &mut App, value: Color) {
        if self.toggleable_painter_data(app).active_color == Some(value) {
            return;
        }
        self.toggleable_painter_data_mut(app).active_color = Some(value);
        self.notify_listeners(app);
    }

    /// The color that should be used in the inactive state (i.e., when
    /// [`ToggleableStateMixin::value`] is false).
    ///
    /// For example, a checkbox should use this color when unchecked.
    fn inactive_color(self: Handle<Self>, app: &App) -> Color {
        self.toggleable_painter_data(app)
            .inactive_color
            .expect("set before painting")
    }

    /// Sets [`inactive_color`](Self::inactive_color).
    fn set_inactive_color(self: Handle<Self>, app: &mut App, value: Color) {
        if self.toggleable_painter_data(app).inactive_color == Some(value) {
            return;
        }
        self.toggleable_painter_data_mut(app).inactive_color = Some(value);
        self.notify_listeners(app);
    }

    /// The color that should be used for the reaction when the toggleable is inactive.
    ///
    /// Used when the toggleable needs to change the reaction color/transparency that is
    /// displayed when the toggleable is inactive and tapped.
    fn inactive_reaction_color(self: Handle<Self>, app: &App) -> Color {
        self.toggleable_painter_data(app)
            .inactive_reaction_color
            .expect("set before painting")
    }

    /// Sets [`inactive_reaction_color`](Self::inactive_reaction_color).
    fn set_inactive_reaction_color(self: Handle<Self>, app: &mut App, value: Color) {
        if self.toggleable_painter_data(app).inactive_reaction_color == Some(value) {
            return;
        }
        self.toggleable_painter_data_mut(app)
            .inactive_reaction_color = Some(value);
        self.notify_listeners(app);
    }

    /// The color that should be used for the reaction when the toggleable is active.
    ///
    /// Used when the toggleable needs to change the reaction color/transparency that is
    /// displayed when the toggleable is active and tapped.
    fn reaction_color(self: Handle<Self>, app: &App) -> Color {
        self.toggleable_painter_data(app)
            .reaction_color
            .expect("set before painting")
    }

    /// Sets [`reaction_color`](Self::reaction_color).
    fn set_reaction_color(self: Handle<Self>, app: &mut App, value: Color) {
        if self.toggleable_painter_data(app).reaction_color == Some(value) {
            return;
        }
        self.toggleable_painter_data_mut(app).reaction_color = Some(value);
        self.notify_listeners(app);
    }

    /// The color that should be used for the reaction when [`is_hovered`](Self::is_hovered)
    /// is true.
    ///
    /// Used when the toggleable needs to change the reaction color/transparency, when it is
    /// being hovered over.
    fn hover_color(self: Handle<Self>, app: &App) -> Color {
        self.toggleable_painter_data(app)
            .hover_color
            .expect("set before painting")
    }

    /// Sets [`hover_color`](Self::hover_color).
    fn set_hover_color(self: Handle<Self>, app: &mut App, value: Color) {
        if self.toggleable_painter_data(app).hover_color == Some(value) {
            return;
        }
        self.toggleable_painter_data_mut(app).hover_color = Some(value);
        self.notify_listeners(app);
    }

    /// The color that should be used for the reaction when [`is_focused`](Self::is_focused)
    /// is true.
    ///
    /// Used when the toggleable needs to change the reaction color/transparency, when it has
    /// focus.
    fn focus_color(self: Handle<Self>, app: &App) -> Color {
        self.toggleable_painter_data(app)
            .focus_color
            .expect("set before painting")
    }

    /// Sets [`focus_color`](Self::focus_color).
    fn set_focus_color(self: Handle<Self>, app: &mut App, value: Color) {
        if self.toggleable_painter_data(app).focus_color == Some(value) {
            return;
        }
        self.toggleable_painter_data_mut(app).focus_color = Some(value);
        self.notify_listeners(app);
    }

    /// The splash radius for the radial reaction.
    fn splash_radius(self: Handle<Self>, app: &App) -> f64 {
        self.toggleable_painter_data(app)
            .splash_radius
            .expect("set before painting")
    }

    /// Sets [`splash_radius`](Self::splash_radius).
    fn set_splash_radius(self: Handle<Self>, app: &mut App, value: f64) {
        if self.toggleable_painter_data(app).splash_radius == Some(value) {
            return;
        }
        self.toggleable_painter_data_mut(app).splash_radius = Some(value);
        self.notify_listeners(app);
    }

    /// The [`Offset`] within the Toggleable at which a pointer touched the Toggleable.
    ///
    /// This is null if currently no pointer is touching the Toggleable.
    ///
    /// Usually set to [`ToggleableStateMixin::down_position`].
    fn down_position(self: Handle<Self>, app: &App) -> Option<Offset> {
        self.toggleable_painter_data(app).down_position
    }

    /// Sets [`down_position`](Self::down_position).
    fn set_down_position(self: Handle<Self>, app: &mut App, value: Option<Offset>) {
        if self.toggleable_painter_data(app).down_position == value {
            return;
        }
        self.toggleable_painter_data_mut(app).down_position = value;
        self.notify_listeners(app);
    }

    /// True if this toggleable has the input focus.
    fn is_focused(self: Handle<Self>, app: &App) -> bool {
        self.toggleable_painter_data(app)
            .is_focused
            .expect("set before painting")
    }

    /// Sets [`is_focused`](Self::is_focused).
    fn set_is_focused(self: Handle<Self>, app: &mut App, value: Option<bool>) {
        if self.toggleable_painter_data(app).is_focused == value {
            return;
        }
        self.toggleable_painter_data_mut(app).is_focused = value;
        self.notify_listeners(app);
    }

    /// True if this toggleable is being hovered over by a pointer.
    fn is_hovered(self: Handle<Self>, app: &App) -> bool {
        self.toggleable_painter_data(app)
            .is_hovered
            .expect("set before painting")
    }

    /// Sets [`is_hovered`](Self::is_hovered).
    fn set_is_hovered(self: Handle<Self>, app: &mut App, value: Option<bool>) {
        if self.toggleable_painter_data(app).is_hovered == value {
            return;
        }
        self.toggleable_painter_data_mut(app).is_hovered = value;
        self.notify_listeners(app);
    }

    /// Determines whether the toggleable shows as active.
    fn is_active(self: Handle<Self>, app: &App) -> bool {
        self.toggleable_painter_data(app)
            .is_active
            .expect("set before painting")
    }

    /// Sets [`is_active`](Self::is_active).
    fn set_is_active(self: Handle<Self>, app: &mut App, value: Option<bool>) {
        if self.toggleable_painter_data(app).is_active == value {
            return;
        }
        self.toggleable_painter_data_mut(app).is_active = value;
        self.notify_listeners(app);
    }

    /// Draws the Toggleable into `size`; Dart's abstract `CustomPainter.paint`.
    fn paint(self: Handle<Self>, app: &mut App, canvas: &mut Canvas, size: Size);

    /// Used by implementors to paint the radial ink reaction for this control.
    ///
    /// The reaction is painted on the given canvas at the given offset. The origin is the
    /// center point of the reaction (usually distinct from the
    /// [`down_position`](Self::down_position) at which the user interacted with the control).
    fn paint_radial_reaction(
        self: Handle<Self>,
        app: &App,
        canvas: &mut Canvas,
        offset: Offset,
        origin: Offset,
    ) {
        let reaction = self.reaction(app);
        let reaction_focus_fade = self.reaction_focus_fade(app);
        let reaction_hover_fade = self.reaction_hover_fade(app);
        if !reaction.is_dismissed(app)
            || !reaction_focus_fade.is_dismissed(app)
            || !reaction_hover_fade.is_dismissed(app)
        {
            let reaction_color = Color::lerp(
                Color::lerp(
                    Color::lerp(
                        Some(self.inactive_reaction_color(app)),
                        Some(self.reaction_color(app)),
                        self.position(app).value(app),
                    ),
                    Some(self.hover_color(app)),
                    reaction_hover_fade.value(app),
                ),
                Some(self.focus_color(app)),
                reaction_focus_fade.value(app),
            )
            .expect("lerping non-null colors");
            let reaction_paint = Paint::from_color(reaction_color.into());
            let splash_radius = self.splash_radius(app);
            // Dart's `Tween<double>(begin: 0.0, end: splashRadius).evaluate(reaction)`.
            let reaction_radius = if self.is_focused(app) || self.is_hovered(app) {
                splash_radius
            } else {
                splash_radius * reaction.value(app)
            };
            if reaction_radius > 0.0 {
                canvas.draw_circle(origin + offset, reaction_radius as f32, &reaction_paint);
            }
        }
    }

    /// Removes the listeners the animation setters added; Dart's `ToggleablePainter.dispose`,
    /// which a painter's own `dispose` calls last.
    fn dispose(self: Handle<Self>, app: &mut App) {
        let notify = Listener::handle_method(self, Handle::<Self>::notify_listeners);
        let data = self.toggleable_painter_data(app);
        let animations = [
            data.position,
            data.reaction,
            data.reaction_focus_fade,
            data.reaction_hover_fade,
        ];
        for animation in animations.into_iter().flatten() {
            animation.remove_listener(app, &notify);
        }
        app.get_mut(self).change_notifier_data_mut().dispose();
    }
}

/// A [`ToggleablePainter`] as the [`CustomPainter`] a `CustomPaint` holds: the painter is an
/// arena object, so the value the widget carries is its handle plus the [`Listenable`] the
/// render object subscribes to (Dart's painter is its own `repaint`).
pub struct ToggleablePainterRef<P: ToggleablePainter> {
    painter: Handle<P>,
    repaint: Rc<dyn Listenable>,
}

impl<P: ToggleablePainter> ToggleablePainterRef<P> {
    /// Wraps a [`ToggleablePainter`] as a [`CustomPainter`].
    pub fn new(painter: Handle<P>) -> ToggleablePainterRef<P> {
        ToggleablePainterRef {
            painter,
            repaint: Rc::new(painter),
        }
    }

    /// The painter itself.
    pub fn painter(&self) -> Handle<P> {
        self.painter
    }
}

impl<P: ToggleablePainter> CustomPainter for ToggleablePainterRef<P> {
    fn repaint(&self) -> Option<&Rc<dyn Listenable>> {
        Some(&self.repaint)
    }

    fn paint(&self, app: &mut App, canvas: &mut Canvas, size: Size) {
        ToggleablePainter::paint(self.painter, app, canvas, size);
    }

    fn should_repaint(&self, _app: &App, _old_delegate: &dyn CustomPainter) -> bool {
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl<P: ToggleablePainter> Debug for ToggleablePainterRef<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{:?}", std::any::type_name::<P>(), self.painter.id())
    }
}
