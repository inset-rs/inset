//! Flutter counterpart: `widgets/modal_barrier.dart`.

use std::any::TypeId;
use std::fmt::{self, Debug};
use std::rc::Rc;

use reveal_animation::AnyAnimation;
use reveal_embedder::Color;
use reveal_foundation::{App, Handle, Listenable, Listener, ValueNotifier};
use reveal_gestures::{
    BaseTapData, BaseTapGestureRecognizer, BaseTapLeaf, BaseTapLeafData, GestureDisposition,
    GestureRecognizer, GestureRecognizerData, K_PRESS_TIMEOUT, OneSequenceData, PointerCancelEvent,
    PointerDownEvent, PointerEvent, PointerUpEvent, PrimaryPointerData,
    PrimaryPointerGestureRecognizer, PrimaryPointerLeaf, PrimaryPointerLeafData, RecognizerLeaf,
    RecognizerLeafData, UNSET_TOUCH_SLOP,
};
use reveal_painting::{EdgeInsets, Matrix4};
use reveal_rendering::{BoxConstraints, HitTestBehavior};
use reveal_services::SystemMouseCursors;

use crate::framework::{BuildContext, IntoWidget, KeyRef, StatelessWidget, WidgetRef};
use crate::widgets::basic::{ColoredBox, ConstrainedBox, MouseRegion};
use crate::widgets::gesture_detector::{
    GestureRecognizerFactories, GestureRecognizerFactory, RawGestureDetector,
};
use crate::widgets::navigator::Navigator;
use crate::widgets::transitions::AnimatedWidget;

/// A widget that prevents the user from interacting with widgets behind itself.
///
/// The modal barrier is the scrim that is rendered behind each route, which generally prevents
/// the user from interacting with the route below the current route, and normally partially
/// obscures such routes.
///
/// For example, when a dialog is on the screen, the page below the dialog is usually darkened
/// by the modal barrier.
///
/// See also:
///
///  * [`ModalRoute`](crate::ModalRoute), which indirectly uses this widget.
///  * [`AnimatedModalBarrier`], which is similar but takes an animated
///    [`color`](Self::color) instead of a single color value.
pub struct ModalBarrier {
    pub key: Option<KeyRef>,
    /// If non-`None`, fill the barrier with this color.
    pub color: Option<Color>,
    /// Specifies if the barrier will be dismissed when the user taps on it.
    ///
    /// If true, and [`on_dismiss`](Self::on_dismiss) is non-`None`,
    /// [`on_dismiss`](Self::on_dismiss) will be called, otherwise the current route will be
    /// popped from the ambient [`Navigator`].
    ///
    /// If false, tapping on the barrier has no effect.
    pub dismissible: bool,
    /// Called when the barrier is being dismissed.
    ///
    /// If non-`None`, [`on_dismiss`](Self::on_dismiss) will be called in place of popping the
    /// current route. It is up to the callback to handle dismissing the barrier.
    ///
    /// If `None`, the ambient [`Navigator`]'s current route will be popped.
    ///
    /// This field is ignored if [`dismissible`](Self::dismissible) is false.
    pub on_dismiss: Option<Listener>,
    /// Whether the modal barrier semantics are included in the semantics tree.
    pub barrier_semantics_dismissible: Option<bool>,
    /// Semantics label used for the barrier if it is [`dismissible`](Self::dismissible).
    ///
    /// The semantics label is read out by accessibility tools when the barrier is focused.
    pub semantics_label: Option<String>,
    /// Contains a value of type [`EdgeInsets`] that specifies how the semantics node rectangle
    /// of the widget should be clipped.
    pub clip_details_notifier: Option<Handle<ValueNotifier<EdgeInsets>>>,
    /// This hint text instructs users what they are able to do when they tap on the
    /// [`ModalBarrier`].
    pub semantics_on_tap_hint: Option<String>,
}

impl ModalBarrier {
    /// Creates a widget that blocks user interaction; Dart's optional arguments are the
    /// setters.
    ///
    /// Dart's defaults: `dismissible` true, `barrier_semantics_dismissible` true.
    pub fn new() -> ModalBarrier {
        ModalBarrier {
            key: None,
            color: None,
            dismissible: true,
            on_dismiss: None,
            barrier_semantics_dismissible: Some(true),
            semantics_label: None,
            clip_details_notifier: None,
            semantics_on_tap_hint: None,
        }
    }

    /// Dart `ModalBarrier(key:)`.
    pub fn key(mut self, key: KeyRef) -> ModalBarrier {
        self.key = Some(key);
        self
    }

    /// Dart `ModalBarrier(color:)`.
    pub fn color(mut self, color: Color) -> ModalBarrier {
        self.color = Some(color);
        self
    }

    /// Dart `ModalBarrier(dismissible:)`.
    pub fn dismissible(mut self, dismissible: bool) -> ModalBarrier {
        self.dismissible = dismissible;
        self
    }

    /// Dart `ModalBarrier(onDismiss:)`.
    pub fn on_dismiss(mut self, on_dismiss: Listener) -> ModalBarrier {
        self.on_dismiss = Some(on_dismiss);
        self
    }

    /// Dart `ModalBarrier(barrierSemanticsDismissible:)`.
    pub fn barrier_semantics_dismissible(
        mut self,
        barrier_semantics_dismissible: bool,
    ) -> ModalBarrier {
        self.barrier_semantics_dismissible = Some(barrier_semantics_dismissible);
        self
    }

    /// [`barrier_semantics_dismissible`](Self::barrier_semantics_dismissible), for a caller
    /// that already holds the `Option` Dart's named argument carries.
    pub fn maybe_barrier_semantics_dismissible(
        mut self,
        barrier_semantics_dismissible: Option<bool>,
    ) -> ModalBarrier {
        self.barrier_semantics_dismissible = barrier_semantics_dismissible;
        self
    }

    /// Dart `ModalBarrier(semanticsLabel:)`.
    pub fn semantics_label(mut self, semantics_label: String) -> ModalBarrier {
        self.semantics_label = Some(semantics_label);
        self
    }

    /// [`semantics_label`](Self::semantics_label), for a caller that already holds the
    /// `Option` Dart's named argument carries.
    pub fn maybe_semantics_label(mut self, semantics_label: Option<String>) -> ModalBarrier {
        self.semantics_label = semantics_label;
        self
    }

    /// Dart `ModalBarrier(clipDetailsNotifier:)`.
    pub fn clip_details_notifier(
        mut self,
        clip_details_notifier: Handle<ValueNotifier<EdgeInsets>>,
    ) -> ModalBarrier {
        self.clip_details_notifier = Some(clip_details_notifier);
        self
    }

    /// Dart `ModalBarrier(semanticsOnTapHint:)`.
    pub fn semantics_on_tap_hint(mut self, semantics_on_tap_hint: String) -> ModalBarrier {
        self.semantics_on_tap_hint = Some(semantics_on_tap_hint);
        self
    }
}

impl Default for ModalBarrier {
    fn default() -> ModalBarrier {
        ModalBarrier::new()
    }
}

impl Debug for ModalBarrier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModalBarrier")
            .field("color", &self.color)
            .field("dismissible", &self.dismissible)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for ModalBarrier {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, context: BuildContext) -> WidgetRef {
        let dismissible = self.dismissible;
        let on_dismiss = self.on_dismiss.clone();
        let handle_dismiss = Listener::new(move |app: &mut App| {
            if !dismissible {
                return;
            }
            match on_dismiss.clone() {
                Some(on_dismiss) => on_dismiss.call(app),
                None => {
                    Navigator::maybe_pop(app, context, None);
                }
            }
        });

        let mut constrained = ConstrainedBox::new(BoxConstraints::expand(None, None));
        if let Some(color) = self.color {
            constrained = constrained.child(ColoredBox::new(color));
        }
        let barrier = MouseRegion::new()
            .cursor(SystemMouseCursors::BASIC.into())
            .child(constrained);

        ModalBarrierGestureDetector {
            child: barrier.into_widget(),
            on_dismiss: handle_dismiss,
        }
        .into_widget()
    }
}

/// A widget that prevents the user from interacting with widgets behind itself, and can be
/// configured with an animated color value.
///
/// This widget is similar to [`ModalBarrier`] except that it takes an animated
/// [`color`](Self::color) instead of a single color.
///
/// See also:
///
///  * [`ModalRoute`](crate::ModalRoute), which uses this widget.
pub struct AnimatedModalBarrier {
    pub key: Option<KeyRef>,
    /// If non-`None`, fill the barrier with this color.
    pub color: AnyAnimation<Option<Color>>,
    /// Whether touching the barrier will pop the current route off the [`Navigator`].
    pub dismissible: bool,
    /// Semantics label used for the barrier if it is [`dismissible`](Self::dismissible).
    pub semantics_label: Option<String>,
    /// Whether the modal barrier semantics are included in the semantics tree.
    pub barrier_semantics_dismissible: Option<bool>,
    /// Called when the barrier is being dismissed.
    pub on_dismiss: Option<Listener>,
    /// See [`ModalBarrier::clip_details_notifier`].
    pub clip_details_notifier: Option<Handle<ValueNotifier<EdgeInsets>>>,
    /// See [`ModalBarrier::semantics_on_tap_hint`].
    pub semantics_on_tap_hint: Option<String>,
}

impl AnimatedModalBarrier {
    /// Creates a widget that blocks user interaction; Dart's optional arguments are the
    /// setters.
    ///
    /// Dart's default: `dismissible` true.
    pub fn new(color: AnyAnimation<Option<Color>>) -> AnimatedModalBarrier {
        AnimatedModalBarrier {
            key: None,
            color,
            dismissible: true,
            semantics_label: None,
            barrier_semantics_dismissible: None,
            on_dismiss: None,
            clip_details_notifier: None,
            semantics_on_tap_hint: None,
        }
    }

    /// Dart `AnimatedModalBarrier(key:)`.
    pub fn key(mut self, key: KeyRef) -> AnimatedModalBarrier {
        self.key = Some(key);
        self
    }

    /// Dart `AnimatedModalBarrier(dismissible:)`.
    pub fn dismissible(mut self, dismissible: bool) -> AnimatedModalBarrier {
        self.dismissible = dismissible;
        self
    }

    /// Dart `AnimatedModalBarrier(semanticsLabel:)`.
    pub fn semantics_label(mut self, semantics_label: String) -> AnimatedModalBarrier {
        self.semantics_label = Some(semantics_label);
        self
    }

    /// [`semantics_label`](Self::semantics_label), for a caller that already holds the
    /// `Option` Dart's named argument carries.
    pub fn maybe_semantics_label(
        mut self,
        semantics_label: Option<String>,
    ) -> AnimatedModalBarrier {
        self.semantics_label = semantics_label;
        self
    }

    /// Dart `AnimatedModalBarrier(barrierSemanticsDismissible:)`.
    pub fn barrier_semantics_dismissible(
        mut self,
        barrier_semantics_dismissible: bool,
    ) -> AnimatedModalBarrier {
        self.barrier_semantics_dismissible = Some(barrier_semantics_dismissible);
        self
    }

    /// Dart `AnimatedModalBarrier(onDismiss:)`.
    pub fn on_dismiss(mut self, on_dismiss: Listener) -> AnimatedModalBarrier {
        self.on_dismiss = Some(on_dismiss);
        self
    }

    /// Dart `AnimatedModalBarrier(clipDetailsNotifier:)`.
    pub fn clip_details_notifier(
        mut self,
        clip_details_notifier: Handle<ValueNotifier<EdgeInsets>>,
    ) -> AnimatedModalBarrier {
        self.clip_details_notifier = Some(clip_details_notifier);
        self
    }

    /// Dart `AnimatedModalBarrier(semanticsOnTapHint:)`.
    pub fn semantics_on_tap_hint(mut self, semantics_on_tap_hint: String) -> AnimatedModalBarrier {
        self.semantics_on_tap_hint = Some(semantics_on_tap_hint);
        self
    }
}

impl Debug for AnimatedModalBarrier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnimatedModalBarrier")
            .field("dismissible", &self.dismissible)
            .finish_non_exhaustive()
    }
}

impl AnimatedWidget for AnimatedModalBarrier {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn listenable(&self) -> Rc<dyn Listenable> {
        Rc::new(self.color)
    }

    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let mut barrier = ModalBarrier::new()
            .dismissible(self.dismissible)
            .maybe_semantics_label(self.semantics_label.clone())
            .maybe_barrier_semantics_dismissible(self.barrier_semantics_dismissible);
        if let Some(color) = self.color.value(app) {
            barrier = barrier.color(color);
        }
        if let Some(on_dismiss) = self.on_dismiss.clone() {
            barrier = barrier.on_dismiss(on_dismiss);
        }
        if let Some(clip_details_notifier) = self.clip_details_notifier {
            barrier = barrier.clip_details_notifier(clip_details_notifier);
        }
        if let Some(semantics_on_tap_hint) = self.semantics_on_tap_hint.clone() {
            barrier = barrier.semantics_on_tap_hint(semantics_on_tap_hint);
        }
        barrier.into_widget()
    }
}

/// Recognizes a tap down by any pointer button.
///
/// It is similar to `TapGestureRecognizer::on_tap_down`, but accepts any single button, which
/// means the gesture also takes part in gesture arenas.
struct AnyTapGestureRecognizer {
    recognizer: GestureRecognizerData,
    one_sequence: OneSequenceData,
    primary: PrimaryPointerData,
    base_tap: BaseTapData,
    on_any_tap_up: Option<Listener>,
}

impl AnyTapGestureRecognizer {
    fn new(app: &mut App) -> Handle<AnyTapGestureRecognizer> {
        app.create(AnyTapGestureRecognizer {
            recognizer: GestureRecognizerData::new(),
            one_sequence: OneSequenceData::new(),
            primary: PrimaryPointerData::new(
                Some(K_PRESS_TIMEOUT),
                Some(UNSET_TOUCH_SLOP),
                Some(UNSET_TOUCH_SLOP),
            ),
            base_tap: BaseTapData::new(),
            on_any_tap_up: None,
        })
    }
}

impl RecognizerLeafData for AnyTapGestureRecognizer {
    fn recognizer(&self) -> &GestureRecognizerData {
        &self.recognizer
    }

    fn recognizer_mut(&mut self) -> &mut GestureRecognizerData {
        &mut self.recognizer
    }

    fn one_sequence(&self) -> &OneSequenceData {
        &self.one_sequence
    }

    fn one_sequence_mut(&mut self) -> &mut OneSequenceData {
        &mut self.one_sequence
    }
}

impl PrimaryPointerLeafData for AnyTapGestureRecognizer {
    fn primary(&self) -> &PrimaryPointerData {
        &self.primary
    }

    fn primary_mut(&mut self) -> &mut PrimaryPointerData {
        &mut self.primary
    }
}

impl BaseTapLeafData for AnyTapGestureRecognizer {
    fn base_tap(&self) -> &BaseTapData {
        &self.base_tap
    }

    fn base_tap_mut(&mut self) -> &mut BaseTapData {
        &mut self.base_tap
    }
}

impl RecognizerLeaf for AnyTapGestureRecognizer {
    fn add_allowed_pointer(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        BaseTapGestureRecognizer::add_allowed_pointer(self, app, event);
    }

    fn handle_non_allowed_pointer(self: Handle<Self>, app: &mut App, event: &PointerDownEvent) {
        PrimaryPointerGestureRecognizer::handle_non_allowed_pointer(self, app, event);
    }

    fn start_tracking_pointer(
        self: Handle<Self>,
        app: &mut App,
        pointer: i64,
        transform: Option<Matrix4>,
    ) {
        BaseTapGestureRecognizer::start_tracking_pointer(self, app, pointer, transform);
    }

    fn handle_event(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        PrimaryPointerGestureRecognizer::handle_event(self, app, event);
    }

    fn is_pointer_allowed(self: Handle<Self>, app: &App, event: &PointerDownEvent) -> bool {
        if app.get(self).on_any_tap_up.is_none() {
            return false;
        }
        GestureRecognizer::is_pointer_allowed(self, app, event)
    }

    fn accept_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
        BaseTapGestureRecognizer::accept_gesture(self, app, pointer);
    }

    fn reject_gesture(self: Handle<Self>, app: &mut App, pointer: i64) {
        BaseTapGestureRecognizer::reject_gesture(self, app, pointer);
    }

    fn did_stop_tracking_last_pointer(self: Handle<Self>, app: &mut App, pointer: i64) {
        PrimaryPointerGestureRecognizer::did_stop_tracking_last_pointer(self, app, pointer);
    }

    fn resolve(self: Handle<Self>, app: &mut App, disposition: GestureDisposition) {
        BaseTapGestureRecognizer::resolve(self, app, disposition);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        PrimaryPointerGestureRecognizer::dispose(self, app);
    }

    fn debug_description(self: Handle<Self>) -> &'static str {
        "any tap"
    }
}

impl PrimaryPointerLeaf for AnyTapGestureRecognizer {
    fn handle_primary_pointer(self: Handle<Self>, app: &mut App, event: PointerEvent) {
        BaseTapGestureRecognizer::handle_primary_pointer(self, app, event);
    }

    fn did_exceed_deadline(self: Handle<Self>, app: &mut App) {
        BaseTapGestureRecognizer::did_exceed_deadline(self, app);
    }
}

impl BaseTapLeaf for AnyTapGestureRecognizer {
    fn handle_tap_down(self: Handle<Self>, _app: &mut App, _down: &PointerDownEvent) {
        // Do nothing.
    }

    fn handle_tap_up(
        self: Handle<Self>,
        app: &mut App,
        _down: &PointerDownEvent,
        _up: &PointerUpEvent,
    ) {
        if let Some(on_any_tap_up) = app.get(self).on_any_tap_up.clone() {
            GestureRecognizer::invoke_callback(self, app, "onAnyTapUp", move |app| {
                on_any_tap_up.call(app)
            });
        }
    }

    fn handle_tap_cancel(
        self: Handle<Self>,
        _app: &mut App,
        _down: &PointerDownEvent,
        _cancel: Option<&PointerCancelEvent>,
        _reason: &str,
    ) {
        // Do nothing.
    }
}

/// Dart's `_AnyTapGestureRecognizerFactory`.
struct AnyTapGestureRecognizerFactory {
    on_any_tap_up: Option<Listener>,
}

impl GestureRecognizerFactory<AnyTapGestureRecognizer> for AnyTapGestureRecognizerFactory {
    fn constructor(&self, app: &mut App) -> Handle<AnyTapGestureRecognizer> {
        AnyTapGestureRecognizer::new(app)
    }

    fn initializer(&self, app: &mut App, instance: Handle<AnyTapGestureRecognizer>) {
        app.get_mut(instance).on_any_tap_up = self.on_any_tap_up.clone();
    }
}

/// A gesture detector used by [`ModalBarrier`]. It only has one callback, which recognizes a
/// tap up unconditionally.
struct ModalBarrierGestureDetector {
    /// The widget below this widget in the tree.
    child: WidgetRef,
    /// Immediately called when an event that should dismiss the modal barrier has happened.
    on_dismiss: Listener,
}

impl Debug for ModalBarrierGestureDetector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ModalBarrierGestureDetector")
    }
}

impl StatelessWidget for ModalBarrierGestureDetector {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let gestures: GestureRecognizerFactories = vec![(
            TypeId::of::<AnyTapGestureRecognizer>(),
            AnyTapGestureRecognizerFactory {
                on_any_tap_up: Some(self.on_dismiss.clone()),
            }
            .into_factory(),
        )];

        RawGestureDetector::new()
            .gestures(gestures)
            .behavior(HitTestBehavior::Opaque)
            .child(self.child.clone())
            .into_widget()
    }
}
