//! Flutter counterpart: `widgets/gesture_detector.dart`.
//!
//! The recognizer factories, [`RawGestureDetector`] with its state, and [`GestureDetector`]
//! over the recognizers ported so far (tap, long press). The semantics half
//! (`excludeFromSemantics`, `semantics`, `SemanticsGestureDelegate`, `_GestureSemantics`) is
//! accessibility and waits.

use std::any::{TypeId, type_name};
use std::collections::HashSet;
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::rc::Rc;

use reveal_embedder::PointerDeviceKind;
use reveal_foundation::{App, Handle};
use reveal_gestures::{
    AnyGestureRecognizer, DeviceGestureSettings, GestureLongPressCallback,
    GestureLongPressCancelCallback, GestureLongPressDownCallback, GestureLongPressEndCallback,
    GestureLongPressMoveUpdateCallback, GestureLongPressStartCallback, GestureLongPressUpCallback,
    GestureRecognizerLeaf, GestureTapCallback, GestureTapCancelCallback, GestureTapDownCallback,
    GestureTapMoveCallback, GestureTapUpCallback, LongPressGestureRecognizer, PointerDownEvent,
    PointerPanZoomStartEvent, TapGestureRecognizer,
};
use reveal_rendering::HitTestBehavior;

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, StatelessWidget, WidgetRef,
};
use crate::widgets::basic::Listener;
use crate::widgets::media_query::MediaQuery;

/// Factory for creating gesture recognizers.
///
/// `T` is the type of gesture recognizer this class manages.
///
/// Used by [`RawGestureDetector::gestures`].
pub trait GestureRecognizerFactory<T: GestureRecognizerLeaf>: 'static {
    /// Must return an instance of T.
    fn constructor(&self, app: &mut App) -> Handle<T>;

    /// Must configure the given instance (which will have been created by
    /// [`constructor`](Self::constructor)).
    ///
    /// This normally means setting the callbacks.
    fn initializer(&self, app: &mut App, instance: Handle<T>);

    /// This factory as Dart's raw `GestureRecognizerFactory`: erased over `T`, the value of
    /// [`RawGestureDetector::gestures`].
    fn into_factory(self) -> GestureRecognizerFactoryRef
    where
        Self: Sized,
    {
        Rc::new(TypedGestureRecognizerFactory {
            factory: self,
            recognizer: PhantomData,
        })
    }
}

/// Signature for closures that implement [`GestureRecognizerFactory::constructor`].
pub type GestureRecognizerFactoryConstructor<T> = Rc<dyn Fn(&mut App) -> Handle<T>>;

/// Signature for closures that implement [`GestureRecognizerFactory::initializer`].
pub type GestureRecognizerFactoryInitializer<T> = Rc<dyn Fn(&mut App, Handle<T>)>;

/// Factory for creating gesture recognizers that delegates to callbacks.
///
/// Used by [`RawGestureDetector::gestures`].
pub struct GestureRecognizerFactoryWithHandlers<T> {
    constructor: GestureRecognizerFactoryConstructor<T>,
    initializer: GestureRecognizerFactoryInitializer<T>,
}

impl<T: GestureRecognizerLeaf> GestureRecognizerFactoryWithHandlers<T> {
    /// Creates a gesture recognizer factory with the given callbacks.
    pub fn new(
        constructor: impl Fn(&mut App) -> Handle<T> + 'static,
        initializer: impl Fn(&mut App, Handle<T>) + 'static,
    ) -> GestureRecognizerFactoryWithHandlers<T> {
        GestureRecognizerFactoryWithHandlers {
            constructor: Rc::new(constructor),
            initializer: Rc::new(initializer),
        }
    }
}

impl<T: GestureRecognizerLeaf> GestureRecognizerFactory<T>
    for GestureRecognizerFactoryWithHandlers<T>
{
    fn constructor(&self, app: &mut App) -> Handle<T> {
        (self.constructor)(app)
    }

    fn initializer(&self, app: &mut App, instance: Handle<T>) {
        (self.initializer)(app, instance)
    }
}

/// A [`GestureRecognizerFactory`] erased over its recognizer type: Dart's raw
/// `GestureRecognizerFactory`, the value of [`RawGestureDetector::gestures`]. Get one with
/// [`GestureRecognizerFactory::into_factory`].
pub trait AnyGestureRecognizerFactory: Debug {
    /// Dart's `_debugAssertTypeMatches`: whether `type_id` is the recognizer type this
    /// factory manages.
    fn debug_assert_type_matches(&self, type_id: TypeId) -> bool;

    /// See [`GestureRecognizerFactory::constructor`].
    fn constructor(&self, app: &mut App) -> AnyGestureRecognizer;

    /// See [`GestureRecognizerFactory::initializer`].
    fn initializer(&self, app: &mut App, instance: AnyGestureRecognizer);
}

/// A shared [`AnyGestureRecognizerFactory`].
pub type GestureRecognizerFactoryRef = Rc<dyn AnyGestureRecognizerFactory>;

/// Dart's `Map<Type, GestureRecognizerFactory>`: the factories in insertion order, each
/// under the `TypeId` of the recognizer it creates
/// (`TypeId::of::<TapGestureRecognizer>()`).
pub type GestureRecognizerFactories = Vec<(TypeId, GestureRecognizerFactoryRef)>;

/// The typed factory behind a [`GestureRecognizerFactoryRef`].
struct TypedGestureRecognizerFactory<T, F> {
    factory: F,
    recognizer: PhantomData<fn() -> T>,
}

impl<T: GestureRecognizerLeaf, F: GestureRecognizerFactory<T>> AnyGestureRecognizerFactory
    for TypedGestureRecognizerFactory<T, F>
{
    fn debug_assert_type_matches(&self, type_id: TypeId) -> bool {
        debug_assert!(
            type_id == TypeId::of::<T>(),
            "GestureRecognizerFactory of type {} was used where type {type_id:?} was specified.",
            type_name::<T>()
        );
        true
    }

    fn constructor(&self, app: &mut App) -> AnyGestureRecognizer {
        self.factory.constructor(app).as_recognizer()
    }

    fn initializer(&self, app: &mut App, instance: AnyGestureRecognizer) {
        let instance = instance
            .downcast::<T>(app)
            .expect("a factory initializes the recognizer type it constructed");
        self.factory.initializer(app, instance);
    }
}

impl<T, F> Debug for TypedGestureRecognizerFactory<T, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GestureRecognizerFactory<{}>", type_name::<T>())
    }
}

/// A widget that detects gestures.
///
/// Attempts to recognize gestures that correspond to its non-null callbacks.
///
/// If this widget has a child, it defers to that child for its sizing behavior.
/// If it does not have a child, it grows to fit the parent instead.
///
/// By default a GestureDetector with an invisible child ignores touches;
/// this behavior can be controlled with [`behavior`](Self::behavior).
///
/// See <http://flutter.dev/to/gestures> for additional information.
///
/// ### Troubleshooting
///
/// Why isn't my parent [`GestureDetector::on_tap`] method called?
///
/// Given a parent [`GestureDetector`] with an onTap callback, and a child
/// GestureDetector that also defines an onTap callback, when the inner
/// GestureDetector is tapped, both GestureDetectors send a `GestureRecognizer`
/// into the gesture arena. This is because the pointer coordinates are within the
/// bounds of both GestureDetectors. The child GestureDetector wins in this
/// scenario because it was the first to enter the arena, resolving as first come,
/// first served. The child onTap is called, and the parent's is not as the gesture has
/// been consumed.
/// For more information on gesture disambiguation see:
/// [Gesture disambiguation](https://flutter.dev/to/gesture-disambiguation).
///
/// Setting [`GestureDetector::behavior`] to [`HitTestBehavior::Opaque`]
/// or [`HitTestBehavior::Translucent`] has no impact on parent-child relationships:
/// both GestureDetectors send a GestureRecognizer into the gesture arena, only one wins.
///
/// Some callbacks (e.g. onTapDown) can fire before a recognizer wins the arena,
/// and others (e.g. onTapCancel) fire even when it loses the arena. Therefore,
/// the parent detector in the example above may call some of its callbacks even
/// though it loses in the arena.
///
/// ## Debugging
///
/// To see how large the hit test box of a [`GestureDetector`] is for debugging
/// purposes, set `debugPaintPointersEnabled` to true.
///
/// See also:
///
///  * `Listener`, a widget for listening to lower-level raw pointer events.
///  * `MouseRegion`, a widget that tracks the movement of mice, even when no
///    button is pressed.
///  * [`RawGestureDetector`], a widget that is used to detect custom gestures.
#[derive(Clone, Default)]
pub struct GestureDetector {
    pub key: Option<KeyRef>,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,

    /// A pointer that might cause a tap with a primary button has contacted the
    /// screen at a particular location.
    ///
    /// This is called after a short timeout, even if the winning gesture has not
    /// yet been selected. If the tap gesture wins, [`on_tap_up`](Self::on_tap_up) will be
    /// called, otherwise [`on_tap_cancel`](Self::on_tap_cancel) will be called.
    ///
    /// See also:
    ///
    ///  * `K_PRIMARY_BUTTON`, the button this callback responds to.
    pub on_tap_down: Option<GestureTapDownCallback>,

    /// A pointer that will trigger a tap with a primary button has stopped
    /// contacting the screen at a particular location.
    ///
    /// This triggers immediately before [`on_tap`](Self::on_tap) in the case of the tap
    /// gesture winning. If the tap gesture did not win,
    /// [`on_tap_cancel`](Self::on_tap_cancel) is called instead.
    ///
    /// See also:
    ///
    ///  * `K_PRIMARY_BUTTON`, the button this callback responds to.
    pub on_tap_up: Option<GestureTapUpCallback>,

    /// A tap with a primary button has occurred.
    ///
    /// This triggers when the tap gesture wins. If the tap gesture did not win,
    /// [`on_tap_cancel`](Self::on_tap_cancel) is called instead.
    ///
    /// See also:
    ///
    ///  * `K_PRIMARY_BUTTON`, the button this callback responds to.
    ///  * [`on_tap_up`](Self::on_tap_up), which is called at the same time but includes
    ///    details regarding the pointer position.
    pub on_tap: Option<GestureTapCallback>,

    /// A pointer that triggered a tap has moved.
    ///
    /// This triggers when the pointer moves after the tap gesture has been recognized.
    ///
    /// See also:
    ///
    ///  * `K_PRIMARY_BUTTON`, the button this callback responds to.
    pub on_tap_move: Option<GestureTapMoveCallback>,

    /// The pointer that previously triggered [`on_tap_down`](Self::on_tap_down) will not
    /// end up causing a tap.
    ///
    /// This is called after [`on_tap_down`](Self::on_tap_down), and instead of
    /// [`on_tap_up`](Self::on_tap_up) and [`on_tap`](Self::on_tap), if the tap gesture did
    /// not win.
    ///
    /// See also:
    ///
    ///  * `K_PRIMARY_BUTTON`, the button this callback responds to.
    pub on_tap_cancel: Option<GestureTapCancelCallback>,

    /// A tap with a secondary button has occurred.
    ///
    /// This triggers when the tap gesture wins. If the tap gesture did not win,
    /// [`on_secondary_tap_cancel`](Self::on_secondary_tap_cancel) is called instead.
    ///
    /// See also:
    ///
    ///  * `K_SECONDARY_BUTTON`, the button this callback responds to.
    ///  * [`on_secondary_tap_up`](Self::on_secondary_tap_up), which is called at the same
    ///    time but includes details regarding the pointer position.
    pub on_secondary_tap: Option<GestureTapCallback>,

    /// A pointer that might cause a tap with a secondary button has contacted the
    /// screen at a particular location.
    ///
    /// This is called after a short timeout, even if the winning gesture has not
    /// yet been selected. If the tap gesture wins,
    /// [`on_secondary_tap_up`](Self::on_secondary_tap_up) will be called, otherwise
    /// [`on_secondary_tap_cancel`](Self::on_secondary_tap_cancel) will be called.
    ///
    /// See also:
    ///
    ///  * `K_SECONDARY_BUTTON`, the button this callback responds to.
    pub on_secondary_tap_down: Option<GestureTapDownCallback>,

    /// A pointer that will trigger a tap with a secondary button has stopped
    /// contacting the screen at a particular location.
    ///
    /// This triggers in the case of the tap gesture winning. If the tap gesture
    /// did not win, [`on_secondary_tap_cancel`](Self::on_secondary_tap_cancel) is called
    /// instead.
    ///
    /// See also:
    ///
    ///  * [`on_secondary_tap`](Self::on_secondary_tap), a handler triggered right after
    ///    this one that doesn't pass any details about the tap.
    ///  * `K_SECONDARY_BUTTON`, the button this callback responds to.
    pub on_secondary_tap_up: Option<GestureTapUpCallback>,

    /// The pointer that previously triggered
    /// [`on_secondary_tap_down`](Self::on_secondary_tap_down) will not end up causing a
    /// tap.
    ///
    /// This is called after [`on_secondary_tap_down`](Self::on_secondary_tap_down), and
    /// instead of [`on_secondary_tap_up`](Self::on_secondary_tap_up), if the tap gesture
    /// did not win.
    ///
    /// See also:
    ///
    ///  * `K_SECONDARY_BUTTON`, the button this callback responds to.
    pub on_secondary_tap_cancel: Option<GestureTapCancelCallback>,

    /// A pointer that might cause a tap with a tertiary button has contacted the
    /// screen at a particular location.
    ///
    /// This is called after a short timeout, even if the winning gesture has not
    /// yet been selected. If the tap gesture wins,
    /// [`on_tertiary_tap_up`](Self::on_tertiary_tap_up) will be called, otherwise
    /// [`on_tertiary_tap_cancel`](Self::on_tertiary_tap_cancel) will be called.
    ///
    /// See also:
    ///
    ///  * `K_TERTIARY_BUTTON`, the button this callback responds to.
    pub on_tertiary_tap_down: Option<GestureTapDownCallback>,

    /// A pointer that will trigger a tap with a tertiary button has stopped
    /// contacting the screen at a particular location.
    ///
    /// This triggers in the case of the tap gesture winning. If the tap gesture
    /// did not win, [`on_tertiary_tap_cancel`](Self::on_tertiary_tap_cancel) is called
    /// instead.
    ///
    /// See also:
    ///
    ///  * `K_TERTIARY_BUTTON`, the button this callback responds to.
    pub on_tertiary_tap_up: Option<GestureTapUpCallback>,

    /// The pointer that previously triggered
    /// [`on_tertiary_tap_down`](Self::on_tertiary_tap_down) will not end up causing a tap.
    ///
    /// This is called after [`on_tertiary_tap_down`](Self::on_tertiary_tap_down), and
    /// instead of [`on_tertiary_tap_up`](Self::on_tertiary_tap_up), if the tap gesture did
    /// not win.
    ///
    /// See also:
    ///
    ///  * `K_TERTIARY_BUTTON`, the button this callback responds to.
    pub on_tertiary_tap_cancel: Option<GestureTapCancelCallback>,

    /// The pointer has contacted the screen with a primary button, which might
    /// be the start of a long-press.
    ///
    /// This triggers after the pointer down event.
    ///
    /// If the user completes the long-press, and this gesture wins,
    /// [`on_long_press_start`](Self::on_long_press_start) will be called after this
    /// callback. Otherwise, [`on_long_press_cancel`](Self::on_long_press_cancel) will be
    /// called after this callback.
    ///
    /// See also:
    ///
    ///  * `K_PRIMARY_BUTTON`, the button this callback responds to.
    ///  * [`on_secondary_long_press_down`](Self::on_secondary_long_press_down), a similar
    ///    callback but for a secondary button.
    ///  * [`on_tertiary_long_press_down`](Self::on_tertiary_long_press_down), a similar
    ///    callback but for a tertiary button.
    ///  * [`LongPressGestureRecognizer::set_on_long_press_down`], which exposes this
    ///    callback at the gesture layer.
    pub on_long_press_down: Option<GestureLongPressDownCallback>,

    /// A pointer that previously triggered [`on_long_press_down`](Self::on_long_press_down)
    /// will not end up causing a long-press.
    ///
    /// This triggers once the gesture loses if
    /// [`on_long_press_down`](Self::on_long_press_down) has previously been triggered.
    ///
    /// If the user completed the long-press, and the gesture won, then
    /// [`on_long_press_start`](Self::on_long_press_start) and
    /// [`on_long_press`](Self::on_long_press) are called instead.
    ///
    /// See also:
    ///
    ///  * `K_PRIMARY_BUTTON`, the button this callback responds to.
    ///  * [`LongPressGestureRecognizer::set_on_long_press_cancel`], which exposes this
    ///    callback at the gesture layer.
    pub on_long_press_cancel: Option<GestureLongPressCancelCallback>,

    /// Called when a long press gesture with a primary button has been recognized.
    ///
    /// Triggered when a pointer has remained in contact with the screen at the
    /// same location for a long period of time.
    ///
    /// This is equivalent to (and is called immediately after)
    /// [`on_long_press_start`](Self::on_long_press_start). The only difference between the
    /// two is that this callback does not contain details of the position at which the
    /// pointer initially contacted the screen.
    ///
    /// See also:
    ///
    ///  * `K_PRIMARY_BUTTON`, the button this callback responds to.
    ///  * [`LongPressGestureRecognizer::set_on_long_press`], which exposes this
    ///    callback at the gesture layer.
    pub on_long_press: Option<GestureLongPressCallback>,

    /// Called when a long press gesture with a primary button has been recognized.
    ///
    /// Triggered when a pointer has remained in contact with the screen at the
    /// same location for a long period of time.
    ///
    /// This is equivalent to (and is called immediately before)
    /// [`on_long_press`](Self::on_long_press). The only difference between the two is that
    /// this callback contains details of the position at which the pointer initially
    /// contacted the screen, whereas [`on_long_press`](Self::on_long_press) does not.
    ///
    /// See also:
    ///
    ///  * `K_PRIMARY_BUTTON`, the button this callback responds to.
    ///  * [`LongPressGestureRecognizer::set_on_long_press_start`], which exposes this
    ///    callback at the gesture layer.
    pub on_long_press_start: Option<GestureLongPressStartCallback>,

    /// A pointer has been drag-moved after a long-press with a primary button.
    ///
    /// See also:
    ///
    ///  * `K_PRIMARY_BUTTON`, the button this callback responds to.
    ///  * [`LongPressGestureRecognizer::set_on_long_press_move_update`], which exposes this
    ///    callback at the gesture layer.
    pub on_long_press_move_update: Option<GestureLongPressMoveUpdateCallback>,

    /// A pointer that has triggered a long-press with a primary button has
    /// stopped contacting the screen.
    ///
    /// This is equivalent to (and is called immediately after)
    /// [`on_long_press_end`](Self::on_long_press_end). The only difference between the two
    /// is that this callback does not contain details of the state of the pointer when it
    /// stopped contacting the screen.
    ///
    /// See also:
    ///
    ///  * `K_PRIMARY_BUTTON`, the button this callback responds to.
    ///  * [`LongPressGestureRecognizer::set_on_long_press_up`], which exposes this
    ///    callback at the gesture layer.
    pub on_long_press_up: Option<GestureLongPressUpCallback>,

    /// A pointer that has triggered a long-press with a primary button has
    /// stopped contacting the screen.
    ///
    /// This is equivalent to (and is called immediately before)
    /// [`on_long_press_up`](Self::on_long_press_up). The only difference between the two is
    /// that this callback contains details of the state of the pointer when it stopped
    /// contacting the screen, whereas [`on_long_press_up`](Self::on_long_press_up) does not.
    ///
    /// See also:
    ///
    ///  * `K_PRIMARY_BUTTON`, the button this callback responds to.
    ///  * [`LongPressGestureRecognizer::set_on_long_press_end`], which exposes this
    ///    callback at the gesture layer.
    pub on_long_press_end: Option<GestureLongPressEndCallback>,

    /// The pointer has contacted the screen with a secondary button, which might
    /// be the start of a long-press.
    ///
    /// This triggers after the pointer down event.
    ///
    /// If the user completes the long-press, and this gesture wins,
    /// [`on_secondary_long_press_start`](Self::on_secondary_long_press_start) will be called
    /// after this callback. Otherwise,
    /// [`on_secondary_long_press_cancel`](Self::on_secondary_long_press_cancel) will be
    /// called after this callback.
    ///
    /// See also:
    ///
    ///  * `K_SECONDARY_BUTTON`, the button this callback responds to.
    ///  * [`on_long_press_down`](Self::on_long_press_down), a similar callback but for a
    ///    secondary button.
    ///  * [`on_tertiary_long_press_down`](Self::on_tertiary_long_press_down), a similar
    ///    callback but for a tertiary button.
    ///  * [`LongPressGestureRecognizer::set_on_secondary_long_press_down`], which exposes
    ///    this callback at the gesture layer.
    pub on_secondary_long_press_down: Option<GestureLongPressDownCallback>,

    /// A pointer that previously triggered
    /// [`on_secondary_long_press_down`](Self::on_secondary_long_press_down) will not end up
    /// causing a long-press.
    ///
    /// This triggers once the gesture loses if
    /// [`on_secondary_long_press_down`](Self::on_secondary_long_press_down) has previously
    /// been triggered.
    ///
    /// If the user completed the long-press, and the gesture won, then
    /// [`on_secondary_long_press_start`](Self::on_secondary_long_press_start) and
    /// [`on_secondary_long_press`](Self::on_secondary_long_press) are called instead.
    ///
    /// See also:
    ///
    ///  * `K_SECONDARY_BUTTON`, the button this callback responds to.
    ///  * [`LongPressGestureRecognizer::set_on_secondary_long_press_cancel`], which exposes
    ///    this callback at the gesture layer.
    pub on_secondary_long_press_cancel: Option<GestureLongPressCancelCallback>,

    /// Called when a long press gesture with a secondary button has been
    /// recognized.
    ///
    /// Triggered when a pointer has remained in contact with the screen at the
    /// same location for a long period of time.
    ///
    /// This is equivalent to (and is called immediately after)
    /// [`on_secondary_long_press_start`](Self::on_secondary_long_press_start). The only
    /// difference between the two is that this callback does not contain details of the
    /// position at which the pointer initially contacted the screen.
    ///
    /// See also:
    ///
    ///  * `K_SECONDARY_BUTTON`, the button this callback responds to.
    ///  * [`LongPressGestureRecognizer::set_on_secondary_long_press`], which exposes
    ///    this callback at the gesture layer.
    pub on_secondary_long_press: Option<GestureLongPressCallback>,

    /// Called when a long press gesture with a secondary button has been
    /// recognized.
    ///
    /// Triggered when a pointer has remained in contact with the screen at the
    /// same location for a long period of time.
    ///
    /// This is equivalent to (and is called immediately before)
    /// [`on_secondary_long_press`](Self::on_secondary_long_press). The only difference
    /// between the two is that this callback contains details of the position at which the
    /// pointer initially contacted the screen, whereas
    /// [`on_secondary_long_press`](Self::on_secondary_long_press) does not.
    ///
    /// See also:
    ///
    ///  * `K_SECONDARY_BUTTON`, the button this callback responds to.
    ///  * [`LongPressGestureRecognizer::set_on_secondary_long_press_start`], which exposes
    ///    this callback at the gesture layer.
    pub on_secondary_long_press_start: Option<GestureLongPressStartCallback>,

    /// A pointer has been drag-moved after a long press with a secondary button.
    ///
    /// See also:
    ///
    ///  * `K_SECONDARY_BUTTON`, the button this callback responds to.
    ///  * [`LongPressGestureRecognizer::set_on_secondary_long_press_move_update`], which
    ///    exposes this callback at the gesture layer.
    pub on_secondary_long_press_move_update: Option<GestureLongPressMoveUpdateCallback>,

    /// A pointer that has triggered a long-press with a secondary button has
    /// stopped contacting the screen.
    ///
    /// This is equivalent to (and is called immediately after)
    /// [`on_secondary_long_press_end`](Self::on_secondary_long_press_end). The only
    /// difference between the two is that this callback does not contain details of the
    /// state of the pointer when it stopped contacting the screen.
    ///
    /// See also:
    ///
    ///  * `K_SECONDARY_BUTTON`, the button this callback responds to.
    ///  * [`LongPressGestureRecognizer::set_on_secondary_long_press_up`], which exposes
    ///    this callback at the gesture layer.
    pub on_secondary_long_press_up: Option<GestureLongPressUpCallback>,

    /// A pointer that has triggered a long-press with a secondary button has
    /// stopped contacting the screen.
    ///
    /// This is equivalent to (and is called immediately before)
    /// [`on_secondary_long_press_up`](Self::on_secondary_long_press_up). The only difference
    /// between the two is that this callback contains details of the state of the pointer
    /// when it stopped contacting the screen, whereas
    /// [`on_secondary_long_press_up`](Self::on_secondary_long_press_up) does not.
    ///
    /// See also:
    ///
    ///  * `K_SECONDARY_BUTTON`, the button this callback responds to.
    ///  * [`LongPressGestureRecognizer::set_on_secondary_long_press_end`], which exposes
    ///    this callback at the gesture layer.
    pub on_secondary_long_press_end: Option<GestureLongPressEndCallback>,

    /// The pointer has contacted the screen with a tertiary button, which might
    /// be the start of a long-press.
    ///
    /// This triggers after the pointer down event.
    ///
    /// If the user completes the long-press, and this gesture wins,
    /// [`on_tertiary_long_press_start`](Self::on_tertiary_long_press_start) will be called
    /// after this callback. Otherwise,
    /// [`on_tertiary_long_press_cancel`](Self::on_tertiary_long_press_cancel) will be called
    /// after this callback.
    ///
    /// See also:
    ///
    ///  * `K_TERTIARY_BUTTON`, the button this callback responds to.
    ///  * [`on_long_press_down`](Self::on_long_press_down), a similar callback but for a
    ///    primary button.
    ///  * [`on_secondary_long_press_down`](Self::on_secondary_long_press_down), a similar
    ///    callback but for a secondary button.
    ///  * [`LongPressGestureRecognizer::set_on_tertiary_long_press_down`], which exposes
    ///    this callback at the gesture layer.
    pub on_tertiary_long_press_down: Option<GestureLongPressDownCallback>,

    /// A pointer that previously triggered
    /// [`on_tertiary_long_press_down`](Self::on_tertiary_long_press_down) will not end up
    /// causing a long-press.
    ///
    /// This triggers once the gesture loses if
    /// [`on_tertiary_long_press_down`](Self::on_tertiary_long_press_down) has previously
    /// been triggered.
    ///
    /// If the user completed the long-press, and the gesture won, then
    /// [`on_tertiary_long_press_start`](Self::on_tertiary_long_press_start) and
    /// [`on_tertiary_long_press`](Self::on_tertiary_long_press) are called instead.
    ///
    /// See also:
    ///
    ///  * `K_TERTIARY_BUTTON`, the button this callback responds to.
    ///  * [`LongPressGestureRecognizer::set_on_tertiary_long_press_cancel`], which exposes
    ///    this callback at the gesture layer.
    pub on_tertiary_long_press_cancel: Option<GestureLongPressCancelCallback>,

    /// Called when a long press gesture with a tertiary button has been
    /// recognized.
    ///
    /// Triggered when a pointer has remained in contact with the screen at the
    /// same location for a long period of time.
    ///
    /// This is equivalent to (and is called immediately after)
    /// [`on_tertiary_long_press_start`](Self::on_tertiary_long_press_start). The only
    /// difference between the two is that this callback does not contain details of the
    /// position at which the pointer initially contacted the screen.
    ///
    /// See also:
    ///
    ///  * `K_TERTIARY_BUTTON`, the button this callback responds to.
    ///  * [`LongPressGestureRecognizer::set_on_tertiary_long_press`], which exposes
    ///    this callback at the gesture layer.
    pub on_tertiary_long_press: Option<GestureLongPressCallback>,

    /// Called when a long press gesture with a tertiary button has been
    /// recognized.
    ///
    /// Triggered when a pointer has remained in contact with the screen at the
    /// same location for a long period of time.
    ///
    /// This is equivalent to (and is called immediately before)
    /// [`on_tertiary_long_press`](Self::on_tertiary_long_press). The only difference between
    /// the two is that this callback contains details of the position at which the pointer
    /// initially contacted the screen, whereas
    /// [`on_tertiary_long_press`](Self::on_tertiary_long_press) does not.
    ///
    /// See also:
    ///
    ///  * `K_TERTIARY_BUTTON`, the button this callback responds to.
    ///  * [`LongPressGestureRecognizer::set_on_tertiary_long_press_start`], which exposes
    ///    this callback at the gesture layer.
    pub on_tertiary_long_press_start: Option<GestureLongPressStartCallback>,

    /// A pointer has been drag-moved after a long press with a tertiary button.
    ///
    /// See also:
    ///
    ///  * `K_TERTIARY_BUTTON`, the button this callback responds to.
    ///  * [`LongPressGestureRecognizer::set_on_tertiary_long_press_move_update`], which
    ///    exposes this callback at the gesture layer.
    pub on_tertiary_long_press_move_update: Option<GestureLongPressMoveUpdateCallback>,

    /// A pointer that has triggered a long-press with a tertiary button has
    /// stopped contacting the screen.
    ///
    /// This is equivalent to (and is called immediately after)
    /// [`on_tertiary_long_press_end`](Self::on_tertiary_long_press_end). The only difference
    /// between the two is that this callback does not contain details of the state of the
    /// pointer when it stopped contacting the screen.
    ///
    /// See also:
    ///
    ///  * `K_TERTIARY_BUTTON`, the button this callback responds to.
    ///  * [`LongPressGestureRecognizer::set_on_tertiary_long_press_up`], which exposes
    ///    this callback at the gesture layer.
    pub on_tertiary_long_press_up: Option<GestureLongPressUpCallback>,

    /// A pointer that has triggered a long-press with a tertiary button has
    /// stopped contacting the screen.
    ///
    /// This is equivalent to (and is called immediately before)
    /// [`on_tertiary_long_press_up`](Self::on_tertiary_long_press_up). The only difference
    /// between the two is that this callback contains details of the state of the pointer
    /// when it stopped contacting the screen, whereas
    /// [`on_tertiary_long_press_up`](Self::on_tertiary_long_press_up) does not.
    ///
    /// See also:
    ///
    ///  * `K_TERTIARY_BUTTON`, the button this callback responds to.
    ///  * [`LongPressGestureRecognizer::set_on_tertiary_long_press_end`], which exposes
    ///    this callback at the gesture layer.
    pub on_tertiary_long_press_end: Option<GestureLongPressEndCallback>,

    /// How this gesture detector should behave during hit testing when deciding
    /// how the hit test propagates to children and whether to consider targets
    /// behind this one.
    ///
    /// This defaults to [`HitTestBehavior::DeferToChild`] if [`child`](Self::child) is not
    /// `None` and [`HitTestBehavior::Translucent`] if child is `None`.
    ///
    /// See [`HitTestBehavior`] for the allowed values and their meanings.
    pub behavior: Option<HitTestBehavior>,

    /// The kind of devices that are allowed to be recognized.
    ///
    /// If set to `None`, events from all device types will be recognized. Defaults to
    /// `None`.
    pub supported_devices: Option<HashSet<PointerDeviceKind>>,
}

impl GestureDetector {
    /// Creates a widget that detects gestures; Dart's named arguments are the setters.
    pub fn new() -> GestureDetector {
        GestureDetector::default()
    }

    /// Dart `GestureDetector(key:)`.
    pub fn key(mut self, key: KeyRef) -> GestureDetector {
        self.key = Some(key);
        self
    }

    /// Dart `GestureDetector(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> GestureDetector {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `GestureDetector(on_tap_down:)`.
    pub fn on_tap_down(mut self, on_tap_down: GestureTapDownCallback) -> GestureDetector {
        self.on_tap_down = Some(on_tap_down);
        self
    }

    /// Dart `GestureDetector(on_tap_up:)`.
    pub fn on_tap_up(mut self, on_tap_up: GestureTapUpCallback) -> GestureDetector {
        self.on_tap_up = Some(on_tap_up);
        self
    }

    /// Dart `GestureDetector(on_tap:)`.
    pub fn on_tap(mut self, on_tap: GestureTapCallback) -> GestureDetector {
        self.on_tap = Some(on_tap);
        self
    }

    /// Dart `GestureDetector(on_tap_move:)`.
    pub fn on_tap_move(mut self, on_tap_move: GestureTapMoveCallback) -> GestureDetector {
        self.on_tap_move = Some(on_tap_move);
        self
    }

    /// Dart `GestureDetector(on_tap_cancel:)`.
    pub fn on_tap_cancel(mut self, on_tap_cancel: GestureTapCancelCallback) -> GestureDetector {
        self.on_tap_cancel = Some(on_tap_cancel);
        self
    }

    /// Dart `GestureDetector(on_secondary_tap:)`.
    pub fn on_secondary_tap(mut self, on_secondary_tap: GestureTapCallback) -> GestureDetector {
        self.on_secondary_tap = Some(on_secondary_tap);
        self
    }

    /// Dart `GestureDetector(on_secondary_tap_down:)`.
    pub fn on_secondary_tap_down(
        mut self,
        on_secondary_tap_down: GestureTapDownCallback,
    ) -> GestureDetector {
        self.on_secondary_tap_down = Some(on_secondary_tap_down);
        self
    }

    /// Dart `GestureDetector(on_secondary_tap_up:)`.
    pub fn on_secondary_tap_up(
        mut self,
        on_secondary_tap_up: GestureTapUpCallback,
    ) -> GestureDetector {
        self.on_secondary_tap_up = Some(on_secondary_tap_up);
        self
    }

    /// Dart `GestureDetector(on_secondary_tap_cancel:)`.
    pub fn on_secondary_tap_cancel(
        mut self,
        on_secondary_tap_cancel: GestureTapCancelCallback,
    ) -> GestureDetector {
        self.on_secondary_tap_cancel = Some(on_secondary_tap_cancel);
        self
    }

    /// Dart `GestureDetector(on_tertiary_tap_down:)`.
    pub fn on_tertiary_tap_down(
        mut self,
        on_tertiary_tap_down: GestureTapDownCallback,
    ) -> GestureDetector {
        self.on_tertiary_tap_down = Some(on_tertiary_tap_down);
        self
    }

    /// Dart `GestureDetector(on_tertiary_tap_up:)`.
    pub fn on_tertiary_tap_up(
        mut self,
        on_tertiary_tap_up: GestureTapUpCallback,
    ) -> GestureDetector {
        self.on_tertiary_tap_up = Some(on_tertiary_tap_up);
        self
    }

    /// Dart `GestureDetector(on_tertiary_tap_cancel:)`.
    pub fn on_tertiary_tap_cancel(
        mut self,
        on_tertiary_tap_cancel: GestureTapCancelCallback,
    ) -> GestureDetector {
        self.on_tertiary_tap_cancel = Some(on_tertiary_tap_cancel);
        self
    }

    /// Dart `GestureDetector(on_long_press_down:)`.
    pub fn on_long_press_down(
        mut self,
        on_long_press_down: GestureLongPressDownCallback,
    ) -> GestureDetector {
        self.on_long_press_down = Some(on_long_press_down);
        self
    }

    /// Dart `GestureDetector(on_long_press_cancel:)`.
    pub fn on_long_press_cancel(
        mut self,
        on_long_press_cancel: GestureLongPressCancelCallback,
    ) -> GestureDetector {
        self.on_long_press_cancel = Some(on_long_press_cancel);
        self
    }

    /// Dart `GestureDetector(on_long_press:)`.
    pub fn on_long_press(mut self, on_long_press: GestureLongPressCallback) -> GestureDetector {
        self.on_long_press = Some(on_long_press);
        self
    }

    /// Dart `GestureDetector(on_long_press_start:)`.
    pub fn on_long_press_start(
        mut self,
        on_long_press_start: GestureLongPressStartCallback,
    ) -> GestureDetector {
        self.on_long_press_start = Some(on_long_press_start);
        self
    }

    /// Dart `GestureDetector(on_long_press_move_update:)`.
    pub fn on_long_press_move_update(
        mut self,
        on_long_press_move_update: GestureLongPressMoveUpdateCallback,
    ) -> GestureDetector {
        self.on_long_press_move_update = Some(on_long_press_move_update);
        self
    }

    /// Dart `GestureDetector(on_long_press_up:)`.
    pub fn on_long_press_up(
        mut self,
        on_long_press_up: GestureLongPressUpCallback,
    ) -> GestureDetector {
        self.on_long_press_up = Some(on_long_press_up);
        self
    }

    /// Dart `GestureDetector(on_long_press_end:)`.
    pub fn on_long_press_end(
        mut self,
        on_long_press_end: GestureLongPressEndCallback,
    ) -> GestureDetector {
        self.on_long_press_end = Some(on_long_press_end);
        self
    }

    /// Dart `GestureDetector(on_secondary_long_press_down:)`.
    pub fn on_secondary_long_press_down(
        mut self,
        on_secondary_long_press_down: GestureLongPressDownCallback,
    ) -> GestureDetector {
        self.on_secondary_long_press_down = Some(on_secondary_long_press_down);
        self
    }

    /// Dart `GestureDetector(on_secondary_long_press_cancel:)`.
    pub fn on_secondary_long_press_cancel(
        mut self,
        on_secondary_long_press_cancel: GestureLongPressCancelCallback,
    ) -> GestureDetector {
        self.on_secondary_long_press_cancel = Some(on_secondary_long_press_cancel);
        self
    }

    /// Dart `GestureDetector(on_secondary_long_press:)`.
    pub fn on_secondary_long_press(
        mut self,
        on_secondary_long_press: GestureLongPressCallback,
    ) -> GestureDetector {
        self.on_secondary_long_press = Some(on_secondary_long_press);
        self
    }

    /// Dart `GestureDetector(on_secondary_long_press_start:)`.
    pub fn on_secondary_long_press_start(
        mut self,
        on_secondary_long_press_start: GestureLongPressStartCallback,
    ) -> GestureDetector {
        self.on_secondary_long_press_start = Some(on_secondary_long_press_start);
        self
    }

    /// Dart `GestureDetector(on_secondary_long_press_move_update:)`.
    pub fn on_secondary_long_press_move_update(
        mut self,
        on_secondary_long_press_move_update: GestureLongPressMoveUpdateCallback,
    ) -> GestureDetector {
        self.on_secondary_long_press_move_update = Some(on_secondary_long_press_move_update);
        self
    }

    /// Dart `GestureDetector(on_secondary_long_press_up:)`.
    pub fn on_secondary_long_press_up(
        mut self,
        on_secondary_long_press_up: GestureLongPressUpCallback,
    ) -> GestureDetector {
        self.on_secondary_long_press_up = Some(on_secondary_long_press_up);
        self
    }

    /// Dart `GestureDetector(on_secondary_long_press_end:)`.
    pub fn on_secondary_long_press_end(
        mut self,
        on_secondary_long_press_end: GestureLongPressEndCallback,
    ) -> GestureDetector {
        self.on_secondary_long_press_end = Some(on_secondary_long_press_end);
        self
    }

    /// Dart `GestureDetector(on_tertiary_long_press_down:)`.
    pub fn on_tertiary_long_press_down(
        mut self,
        on_tertiary_long_press_down: GestureLongPressDownCallback,
    ) -> GestureDetector {
        self.on_tertiary_long_press_down = Some(on_tertiary_long_press_down);
        self
    }

    /// Dart `GestureDetector(on_tertiary_long_press_cancel:)`.
    pub fn on_tertiary_long_press_cancel(
        mut self,
        on_tertiary_long_press_cancel: GestureLongPressCancelCallback,
    ) -> GestureDetector {
        self.on_tertiary_long_press_cancel = Some(on_tertiary_long_press_cancel);
        self
    }

    /// Dart `GestureDetector(on_tertiary_long_press:)`.
    pub fn on_tertiary_long_press(
        mut self,
        on_tertiary_long_press: GestureLongPressCallback,
    ) -> GestureDetector {
        self.on_tertiary_long_press = Some(on_tertiary_long_press);
        self
    }

    /// Dart `GestureDetector(on_tertiary_long_press_start:)`.
    pub fn on_tertiary_long_press_start(
        mut self,
        on_tertiary_long_press_start: GestureLongPressStartCallback,
    ) -> GestureDetector {
        self.on_tertiary_long_press_start = Some(on_tertiary_long_press_start);
        self
    }

    /// Dart `GestureDetector(on_tertiary_long_press_move_update:)`.
    pub fn on_tertiary_long_press_move_update(
        mut self,
        on_tertiary_long_press_move_update: GestureLongPressMoveUpdateCallback,
    ) -> GestureDetector {
        self.on_tertiary_long_press_move_update = Some(on_tertiary_long_press_move_update);
        self
    }

    /// Dart `GestureDetector(on_tertiary_long_press_up:)`.
    pub fn on_tertiary_long_press_up(
        mut self,
        on_tertiary_long_press_up: GestureLongPressUpCallback,
    ) -> GestureDetector {
        self.on_tertiary_long_press_up = Some(on_tertiary_long_press_up);
        self
    }

    /// Dart `GestureDetector(on_tertiary_long_press_end:)`.
    pub fn on_tertiary_long_press_end(
        mut self,
        on_tertiary_long_press_end: GestureLongPressEndCallback,
    ) -> GestureDetector {
        self.on_tertiary_long_press_end = Some(on_tertiary_long_press_end);
        self
    }

    /// Dart `GestureDetector(behavior:)`.
    pub fn behavior(mut self, behavior: HitTestBehavior) -> GestureDetector {
        self.behavior = Some(behavior);
        self
    }

    /// Dart `GestureDetector(supported_devices:)`.
    pub fn supported_devices(
        mut self,
        supported_devices: HashSet<PointerDeviceKind>,
    ) -> GestureDetector {
        self.supported_devices = Some(supported_devices);
        self
    }

    fn has_tap_callback(&self) -> bool {
        self.on_tap_down.is_some()
            || self.on_tap_up.is_some()
            || self.on_tap.is_some()
            || self.on_tap_cancel.is_some()
            || self.on_secondary_tap.is_some()
            || self.on_secondary_tap_down.is_some()
            || self.on_secondary_tap_up.is_some()
            || self.on_secondary_tap_cancel.is_some()
            || self.on_tertiary_tap_down.is_some()
            || self.on_tertiary_tap_up.is_some()
            || self.on_tertiary_tap_cancel.is_some()
    }

    fn has_long_press_callback(&self) -> bool {
        self.on_long_press_down.is_some()
            || self.on_long_press_cancel.is_some()
            || self.on_long_press.is_some()
            || self.on_long_press_start.is_some()
            || self.on_long_press_move_update.is_some()
            || self.on_long_press_up.is_some()
            || self.on_long_press_end.is_some()
            || self.on_secondary_long_press_down.is_some()
            || self.on_secondary_long_press_cancel.is_some()
            || self.on_secondary_long_press.is_some()
            || self.on_secondary_long_press_start.is_some()
            || self.on_secondary_long_press_move_update.is_some()
            || self.on_secondary_long_press_up.is_some()
            || self.on_secondary_long_press_end.is_some()
            || self.on_tertiary_long_press_down.is_some()
            || self.on_tertiary_long_press_cancel.is_some()
            || self.on_tertiary_long_press.is_some()
            || self.on_tertiary_long_press_start.is_some()
            || self.on_tertiary_long_press_move_update.is_some()
            || self.on_tertiary_long_press_up.is_some()
            || self.on_tertiary_long_press_end.is_some()
    }

    /// The `TapGestureRecognizer` factory of Dart's `build`; the closures capture the
    /// widget as Dart's capture `this`.
    fn tap_factory(
        &self,
        gesture_settings: Option<DeviceGestureSettings>,
    ) -> GestureRecognizerFactoryRef {
        let this = self.clone();
        let constructed = self.clone();
        GestureRecognizerFactoryWithHandlers::<TapGestureRecognizer>::new(
            move |app| {
                let instance = TapGestureRecognizer::new(app);
                instance.set_supported_devices(app, constructed.supported_devices.clone());
                instance
            },
            move |app, instance| {
                instance.set_on_tap_down(app, this.on_tap_down.clone());
                instance.set_on_tap_up(app, this.on_tap_up.clone());
                instance.set_on_tap(app, this.on_tap.clone());
                instance.set_on_tap_cancel(app, this.on_tap_cancel.clone());
                instance.set_on_secondary_tap(app, this.on_secondary_tap.clone());
                instance.set_on_secondary_tap_down(app, this.on_secondary_tap_down.clone());
                instance.set_on_secondary_tap_up(app, this.on_secondary_tap_up.clone());
                instance.set_on_secondary_tap_cancel(app, this.on_secondary_tap_cancel.clone());
                instance.set_on_tertiary_tap_down(app, this.on_tertiary_tap_down.clone());
                instance.set_on_tertiary_tap_up(app, this.on_tertiary_tap_up.clone());
                instance.set_on_tertiary_tap_cancel(app, this.on_tertiary_tap_cancel.clone());
                instance.set_gesture_settings(app, gesture_settings);
                instance.set_supported_devices(app, this.supported_devices.clone());
            },
        )
        .into_factory()
    }

    /// The `LongPressGestureRecognizer` factory of Dart's `build`.
    fn long_press_factory(
        &self,
        gesture_settings: Option<DeviceGestureSettings>,
    ) -> GestureRecognizerFactoryRef {
        let this = self.clone();
        let constructed = self.clone();
        GestureRecognizerFactoryWithHandlers::<LongPressGestureRecognizer>::new(
            move |app| {
                let instance = LongPressGestureRecognizer::new(app);
                instance.set_supported_devices(app, constructed.supported_devices.clone());
                instance
            },
            move |app, instance| {
                instance.set_on_long_press_down(app, this.on_long_press_down.clone());
                instance.set_on_long_press_cancel(app, this.on_long_press_cancel.clone());
                instance.set_on_long_press(app, this.on_long_press.clone());
                instance.set_on_long_press_start(app, this.on_long_press_start.clone());
                instance.set_on_long_press_move_update(app, this.on_long_press_move_update.clone());
                instance.set_on_long_press_up(app, this.on_long_press_up.clone());
                instance.set_on_long_press_end(app, this.on_long_press_end.clone());
                instance.set_on_secondary_long_press_down(
                    app,
                    this.on_secondary_long_press_down.clone(),
                );
                instance.set_on_secondary_long_press_cancel(
                    app,
                    this.on_secondary_long_press_cancel.clone(),
                );
                instance.set_on_secondary_long_press(app, this.on_secondary_long_press.clone());
                instance.set_on_secondary_long_press_start(
                    app,
                    this.on_secondary_long_press_start.clone(),
                );
                instance.set_on_secondary_long_press_move_update(
                    app,
                    this.on_secondary_long_press_move_update.clone(),
                );
                instance
                    .set_on_secondary_long_press_up(app, this.on_secondary_long_press_up.clone());
                instance
                    .set_on_secondary_long_press_end(app, this.on_secondary_long_press_end.clone());
                instance
                    .set_on_tertiary_long_press_down(app, this.on_tertiary_long_press_down.clone());
                instance.set_on_tertiary_long_press_cancel(
                    app,
                    this.on_tertiary_long_press_cancel.clone(),
                );
                instance.set_on_tertiary_long_press(app, this.on_tertiary_long_press.clone());
                instance.set_on_tertiary_long_press_start(
                    app,
                    this.on_tertiary_long_press_start.clone(),
                );
                instance.set_on_tertiary_long_press_move_update(
                    app,
                    this.on_tertiary_long_press_move_update.clone(),
                );
                instance.set_on_tertiary_long_press_up(app, this.on_tertiary_long_press_up.clone());
                instance
                    .set_on_tertiary_long_press_end(app, this.on_tertiary_long_press_end.clone());
                instance.set_gesture_settings(app, gesture_settings);
                instance.set_supported_devices(app, this.supported_devices.clone());
            },
        )
        .into_factory()
    }
}

impl StatelessWidget for GestureDetector {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let mut gestures: GestureRecognizerFactories = Vec::new();
        let gesture_settings = MediaQuery::maybe_gesture_settings_of(app, context);

        if self.has_tap_callback() {
            gestures.push((
                TypeId::of::<TapGestureRecognizer>(),
                self.tap_factory(gesture_settings),
            ));
        }

        if self.has_long_press_callback() {
            gestures.push((
                TypeId::of::<LongPressGestureRecognizer>(),
                self.long_press_factory(gesture_settings),
            ));
        }

        let mut detector = RawGestureDetector::new().gestures(gestures);
        if let Some(child) = &self.child {
            detector = detector.child(child.clone());
        }
        if let Some(behavior) = self.behavior {
            detector = detector.behavior(behavior);
        }
        detector.into_widget()
    }
}

impl Debug for GestureDetector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GestureDetector")
            .field("key", &self.key)
            .field("behavior", &self.behavior)
            .field("supportedDevices", &self.supported_devices)
            .field("child", &self.child)
            .finish()
    }
}

/// A widget that detects gestures described by the given gesture
/// factories.
///
/// For common gestures, use a [`GestureDetector`].
/// [`RawGestureDetector`] is useful primarily when developing your
/// own gesture recognizers.
///
/// Configuring the gesture recognizers requires a carefully constructed map, as
/// described in [`gestures`](Self::gestures).
///
/// See also:
///
///  * [`GestureDetector`], a less flexible but much simpler widget that does the same thing.
///  * `Listener`, a widget that reports raw pointer events.
///  * `GestureRecognizer`, the class that you extend to create a custom gesture recognizer.
#[derive(Debug, Default)]
pub struct RawGestureDetector {
    pub key: Option<KeyRef>,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,

    /// The gestures that this widget will attempt to recognize.
    ///
    /// This should be a map from `GestureRecognizer` subclasses to
    /// [`GestureRecognizerFactory`] subclasses specialized with the same type.
    ///
    /// This value can be late-bound at layout time using
    /// [`RawGestureDetectorState::replace_gesture_recognizers`].
    pub gestures: GestureRecognizerFactories,

    /// How this gesture detector should behave during hit testing.
    ///
    /// This defaults to [`HitTestBehavior::DeferToChild`] if [`child`](Self::child) is not
    /// `None` and [`HitTestBehavior::Translucent`] if child is `None`.
    pub behavior: Option<HitTestBehavior>,
}

impl RawGestureDetector {
    /// Creates a widget that detects gestures; Dart's named arguments are the setters.
    pub fn new() -> RawGestureDetector {
        RawGestureDetector::default()
    }

    /// Dart `RawGestureDetector(key:)`.
    pub fn key(mut self, key: KeyRef) -> RawGestureDetector {
        self.key = Some(key);
        self
    }

    /// Dart `RawGestureDetector(gestures:)`.
    pub fn gestures(mut self, gestures: GestureRecognizerFactories) -> RawGestureDetector {
        self.gestures = gestures;
        self
    }

    /// Dart `RawGestureDetector(behavior:)`.
    pub fn behavior(mut self, behavior: HitTestBehavior) -> RawGestureDetector {
        self.behavior = Some(behavior);
        self
    }

    /// Dart `RawGestureDetector(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> RawGestureDetector {
        self.child = Some(child.into_widget());
        self
    }
}

impl StatefulWidget for RawGestureDetector {
    type State = RawGestureDetectorState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> RawGestureDetectorState {
        RawGestureDetectorState {
            state: StateData::new(),
            recognizers: Some(Vec::new()),
        }
    }
}

/// Dart's `Map<Type, GestureRecognizer>`: the live recognizers, in the order of the
/// factories that made them.
type Recognizers = Vec<(TypeId, AnyGestureRecognizer)>;

/// Dart's `map[type]`.
fn recognizer_of_type(
    recognizers: &[(TypeId, AnyGestureRecognizer)],
    type_id: TypeId,
) -> Option<AnyGestureRecognizer> {
    recognizers
        .iter()
        .find(|(candidate, _)| *candidate == type_id)
        .map(|(_, recognizer)| *recognizer)
}

/// Dart's `map.containsKey(type)`.
fn contains_type(recognizers: &[(TypeId, AnyGestureRecognizer)], type_id: TypeId) -> bool {
    recognizer_of_type(recognizers, type_id).is_some()
}

/// State for a [`RawGestureDetector`].
pub struct RawGestureDetectorState {
    state: StateData<RawGestureDetector>,
    /// `None` after [`dispose`](State::dispose), as Dart nulls `_recognizers`.
    recognizers: Option<Recognizers>,
}

impl RawGestureDetectorState {
    /// This method can be called after the build phase, during the
    /// layout of the nearest descendant `RenderObjectWidget` of the
    /// gesture detector, to update the list of active gesture
    /// recognizers.
    ///
    /// The typical use case is `Scrollable`s, which put their viewport
    /// in their gesture detector, and then need to know the dimensions
    /// of the viewport and the viewport's child to determine whether
    /// the gesture detector should be enabled.
    ///
    /// The argument should follow the same conventions as
    /// [`RawGestureDetector::gestures`]. It acts like a temporary replacement for
    /// that value until the next build.
    pub fn replace_gesture_recognizers(
        self: Handle<Self>,
        app: &mut App,
        gestures: GestureRecognizerFactories,
    ) {
        debug_assert!(
            self.context(app)
                .find_render_object(app)
                .expect("a mounted RawGestureDetector has a render object")
                .owner(app)
                .expect("the render object is attached")
                .debug_doing_layout(app),
            "Unexpected call to replaceGestureRecognizers() method of RawGestureDetectorState. \
             The replaceGestureRecognizers() method can only be called during the layout phase. \
             To set the gesture recognizers at other times, trigger a new build using setState() \
             and provide the new gesture recognizers as constructor arguments to the \
             corresponding RawGestureDetector or GestureDetector object."
        );
        self.sync_all(app, &gestures);
    }

    fn sync_all(self: Handle<Self>, app: &mut App, gestures: &GestureRecognizerFactories) {
        debug_assert!(app.get(self).recognizers.is_some());
        let old_recognizers = app
            .get_mut(self)
            .recognizers
            .replace(Vec::new())
            .expect("_syncAll after dispose");
        for (type_id, factory) in gestures {
            debug_assert!(factory.debug_assert_type_matches(*type_id));
            debug_assert!(!contains_type(self.recognizers(app), *type_id));
            let recognizer = match recognizer_of_type(&old_recognizers, *type_id) {
                Some(existing) => existing,
                None => factory.constructor(app),
            };
            self.recognizers_mut(app).push((*type_id, recognizer));
            debug_assert!(
                recognizer.type_id() == *type_id,
                "GestureRecognizerFactory of type {type_id:?} created a GestureRecognizer of \
                 type {:?}. The GestureRecognizerFactory must be specialized with the type of \
                 the class that it returns from its constructor method.",
                recognizer.type_id()
            );
            factory.initializer(app, recognizer);
        }
        for (type_id, old_recognizer) in &old_recognizers {
            if !contains_type(self.recognizers(app), *type_id) {
                old_recognizer.dispose(app);
            }
        }
    }

    fn handle_pointer_down(self: Handle<Self>, app: &mut App, event: PointerDownEvent) {
        debug_assert!(app.get(self).recognizers.is_some());
        for recognizer in self.recognizer_handles(app) {
            recognizer.add_pointer(app, &event);
        }
    }

    fn handle_pointer_pan_zoom_start(
        self: Handle<Self>,
        app: &mut App,
        event: PointerPanZoomStartEvent,
    ) {
        debug_assert!(app.get(self).recognizers.is_some());
        for recognizer in self.recognizer_handles(app) {
            recognizer.add_pointer_pan_zoom(app, &event);
        }
    }

    fn default_behavior(self: Handle<Self>, app: &App) -> HitTestBehavior {
        if self.widget(app).child.is_none() {
            HitTestBehavior::Translucent
        } else {
            HitTestBehavior::DeferToChild
        }
    }

    /// Dart's `_recognizers!`.
    fn recognizers(self: Handle<Self>, app: &App) -> &[(TypeId, AnyGestureRecognizer)] {
        app.get(self)
            .recognizers
            .as_deref()
            .expect("_recognizers is null: the state is disposed")
    }

    /// See [`recognizers`](Self::recognizers).
    fn recognizers_mut(self: Handle<Self>, app: &mut App) -> &mut Recognizers {
        app.get_mut(self)
            .recognizers
            .as_mut()
            .expect("_recognizers is null: the state is disposed")
    }

    /// Dart's `_recognizers!.values`, copied out so the loop can hand `app` to each.
    fn recognizer_handles(self: Handle<Self>, app: &App) -> Vec<AnyGestureRecognizer> {
        self.recognizers(app)
            .iter()
            .map(|(_, recognizer)| *recognizer)
            .collect()
    }
}

impl State for RawGestureDetectorState {
    type Widget = RawGestureDetector;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let gestures = self.widget(app).gestures.clone();
        self.sync_all(app, &gestures);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &RawGestureDetector) {
        let gestures = self.widget(app).gestures.clone();
        self.sync_all(app, &gestures);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let recognizers = app
            .get_mut(self)
            .recognizers
            .take()
            .expect("dispose after dispose");
        for (_, recognizer) in recognizers {
            recognizer.dispose(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.widget(app);
        let behavior = widget
            .behavior
            .unwrap_or_else(|| self.default_behavior(app));
        let child = widget.child.clone();
        let mut listener = Listener::new()
            .on_pointer_down(Rc::new(move |app: &mut App, event: PointerDownEvent| {
                self.handle_pointer_down(app, event);
            }))
            .on_pointer_pan_zoom_start(Rc::new(
                move |app: &mut App, event: PointerPanZoomStartEvent| {
                    self.handle_pointer_pan_zoom_start(app, event);
                },
            ))
            .behavior(behavior);
        if let Some(child) = child {
            listener = listener.child(child);
        }
        listener.into_widget()
    }
}

impl Debug for RawGestureDetectorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(recognizers) = &self.recognizers else {
            return f.write_str("RawGestureDetectorState(DISPOSED)");
        };
        let gestures: Vec<&'static str> = recognizers
            .iter()
            .map(|(_, recognizer)| recognizer.debug_description())
            .collect();
        f.debug_struct("RawGestureDetectorState")
            .field("gestures", &gestures)
            .field("recognizers", recognizers)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use reveal_embedder::{Offset, Size, ViewId};
    use reveal_foundation::{App, AppCell, Handle, HandleId, Listener};
    use reveal_gestures::{
        GestureBinding, GestureBindingOverridesObject, HitTestResult, K_LONG_PRESS_TIMEOUT,
        PointerDownEvent, PointerEvent, PointerUpEvent, TapDownDetails, TapUpDetails,
    };
    use reveal_rendering::{
        AnyRenderObject, BoxConstraints, RenderBox, RenderBoxData, RenderConstrainedBox,
        RenderHandle, RenderObject, RenderObjectData, RenderObjectWithChildMixin,
        RenderPointerListener, RenderView,
    };

    use super::*;
    use crate::framework::{AnyElement, Element, LeafRenderObjectWidget, RenderObjectWidget};
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;

    // ---- the test tree ----

    /// Plays `RendererBinding`'s part: hit-tests the harness's render root before the
    /// gesture binding adds itself to the path.
    struct RootHitTester {
        root: RenderHandle<RenderView>,
    }

    impl GestureBindingOverridesObject for RootHitTester {
        fn hit_test(
            self: Handle<Self>,
            app: &mut App,
            result: &mut HitTestResult,
            position: Offset,
        ) {
            let root = app.get(self).root;
            root.hit_test(app, result, position);
        }

        fn hit_test_in_view(
            self: Handle<Self>,
            app: &mut App,
            result: &mut HitTestResult,
            position: Offset,
            _view_id: ViewId,
        ) {
            GestureBindingOverridesObject::hit_test(self, app, result, position);
        }

        fn will_dispatch_event(
            self: Handle<Self>,
            _app: &mut App,
            _event: &PointerEvent,
            _hit_test_result: Option<&HitTestResult>,
        ) {
        }
    }

    /// Mounts and pumps `child`, and points the gesture binding's hit test at the tree.
    fn mount(app: &mut App, child: WidgetRef) -> Harness {
        let harness = Harness::mount(app, child);
        harness.pump(app);
        let hit_tester = app.create(RootHitTester {
            root: harness.render_root(app),
        });
        GestureBinding::instance(app).set_overrides(app, Rc::new(hit_tester));
        harness
    }

    /// The `RawGestureDetectorState` of the first detector under the root, through the
    /// `GestureDetector` when there is one.
    fn detector_state(harness: &Harness, app: &App) -> Handle<RawGestureDetectorState> {
        let mut element = harness.root.as_element().children(app)[0];
        loop {
            if let Some(state) = element.state_handle::<RawGestureDetectorState>(app) {
                return state;
            }
            element = element.children(app)[0];
        }
    }

    fn press(app: &mut App, pointer: i64, position: Offset, kind: PointerDeviceKind) {
        GestureBinding::instance(app).handle_pointer_event(
            app,
            PointerEvent::Down(PointerDownEvent {
                pointer,
                position,
                kind,
                ..PointerDownEvent::default()
            }),
        );
        app.drain_microtasks();
    }

    fn release(app: &mut App, pointer: i64, position: Offset, kind: PointerDeviceKind) {
        GestureBinding::instance(app).handle_pointer_event(
            app,
            PointerEvent::Up(PointerUpEvent {
                pointer,
                position,
                kind,
                ..PointerUpEvent::default()
            }),
        );
        app.drain_microtasks();
    }

    fn tap_at(app: &mut App, pointer: i64, position: Offset, kind: PointerDeviceKind) {
        press(app, pointer, position, kind);
        release(app, pointer, position, kind);
    }

    /// `SizedBox` in miniature: a box that is never hit by itself.
    #[derive(Debug)]
    struct Sized {
        size: Size,
    }

    impl RenderObjectWidget for Sized {
        type RenderObject = RenderConstrainedBox;

        fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
            RenderConstrainedBox::new(app, BoxConstraints::tight(self.size), None).as_object()
        }
    }

    impl LeafRenderObjectWidget for Sized {}

    /// A leaf that runs a callback from `perform_layout`, the way a viewport reaches
    /// `replace_gesture_recognizers` during layout.
    struct LayoutProbe {
        on_layout: Rc<dyn Fn(&mut App)>,
    }

    impl Debug for LayoutProbe {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("LayoutProbe")
        }
    }

    impl RenderObjectWidget for LayoutProbe {
        type RenderObject = RenderLayoutProbe;

        fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
            RenderHandle::new_box(
                app,
                RenderLayoutProbe {
                    render_object: RenderObjectData::new(),
                    render_box: RenderBoxData::new(),
                    on_layout: Rc::clone(&self.on_layout),
                },
            )
            .as_object()
        }
    }

    impl LeafRenderObjectWidget for LayoutProbe {}

    struct RenderLayoutProbe {
        render_object: RenderObjectData,
        render_box: RenderBoxData,
        on_layout: Rc<dyn Fn(&mut App)>,
    }

    impl RenderObject for RenderLayoutProbe {
        reveal_rendering::render_object_accessors!();

        fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
            let on_layout = Rc::clone(&self.get(app).on_layout);
            on_layout(app);
            let size = self.constraints(app).biggest();
            self.set_size(app, size);
        }
    }

    impl RenderBox for RenderLayoutProbe {
        reveal_rendering::render_box_accessors!();
    }

    /// A tap factory that counts its constructions and initializations and records the
    /// recognizer it made.
    struct CountingTapFactory {
        constructions: Rc<Cell<u32>>,
        initializations: Rc<Cell<u32>>,
        constructed: Rc<Cell<Option<HandleId>>>,
        taps: Rc<Cell<u32>>,
    }

    impl CountingTapFactory {
        fn detector(&self) -> WidgetRef {
            let constructions = Rc::clone(&self.constructions);
            let constructed = Rc::clone(&self.constructed);
            let initializations = Rc::clone(&self.initializations);
            let taps = Rc::clone(&self.taps);
            let factory = GestureRecognizerFactoryWithHandlers::<TapGestureRecognizer>::new(
                move |app| {
                    constructions.set(constructions.get() + 1);
                    let recognizer = TapGestureRecognizer::new(app);
                    constructed.set(Some(recognizer.id()));
                    recognizer
                },
                move |app, instance| {
                    initializations.set(initializations.get() + 1);
                    let taps = Rc::clone(&taps);
                    instance.set_on_tap(
                        app,
                        Some(Listener::new(move |_app| taps.set(taps.get() + 1))),
                    );
                },
            );
            RawGestureDetector::new()
                .gestures(vec![(
                    TypeId::of::<TapGestureRecognizer>(),
                    factory.into_factory(),
                )])
                .into_widget()
        }
    }

    // ---- GestureDetector ----

    /// A widget that changes the shape of its subtree on the press, so the `Listener` under it
    /// is re-inflated while the pointer is down.
    #[derive(Debug)]
    struct SwapOnPress {
        ups: Rc<Cell<u32>>,
    }

    struct SwapOnPressState {
        state: StateData<SwapOnPress>,
        swapped: bool,
    }

    impl StatefulWidget for SwapOnPress {
        type State = SwapOnPressState;
        fn create_state(&self) -> SwapOnPressState {
            SwapOnPressState {
                state: StateData::new(),
                swapped: false,
            }
        }
    }

    impl State for SwapOnPressState {
        type Widget = SwapOnPress;
        crate::state_accessors!();
        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            let ups = Rc::clone(&self.widget(app).ups);
            let listener = crate::Listener::new()
                .behavior(HitTestBehavior::Opaque)
                .on_pointer_down(Rc::new(move |app: &mut App, _event| {
                    self.set_state(app, |state| state.swapped = true);
                }))
                .on_pointer_up(Rc::new(move |_app: &mut App, _event| {
                    ups.set(ups.get() + 1);
                }))
                .child(Sized {
                    size: Size::new(100.0, 100.0),
                });
            if app.get(self).swapped {
                SizedBox::new().child(listener).into_widget()
            } else {
                listener.into_widget()
            }
        }
    }

    fn pointer_listener_under(app: &App, root: AnyElement) -> AnyRenderObject {
        let mut pending = vec![root];
        while let Some(element) = pending.pop() {
            if let Some(object) = element.render_object(app)
                && object.downcast::<RenderPointerListener>(app).is_some()
            {
                return object;
            }
            pending.extend(element.children(app));
        }
        panic!("no pointer listener under the root");
    }

    /// Flutter's gesture binding replays the press's hit-test path for the release, and the
    /// `RenderPointerListener` it names stays alive in Dart even after a rebuild disposed it. The
    /// arena retains it for as long as the path does, so the release still reaches it.
    #[test]
    fn a_listener_rebuilt_mid_press_still_receives_the_release() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let ups = Rc::new(Cell::new(0));
        let harness = mount(
            &mut app,
            SwapOnPress {
                ups: Rc::clone(&ups),
            }
            .into_widget(),
        );
        let pressed_listener = pointer_listener_under(&app, harness.root.as_element());

        press(
            &mut app,
            1,
            Offset::new(20.0, 30.0),
            PointerDeviceKind::Touch,
        );
        harness.pump(&mut app);
        let rebuilt_listener = pointer_listener_under(&app, harness.root.as_element());
        assert_ne!(
            rebuilt_listener, pressed_listener,
            "the press swapped the subtree"
        );
        assert!(
            app.is_disposed(pressed_listener.id()),
            "disposed by the rebuild"
        );
        assert!(
            app.contains(pressed_listener.id()),
            "kept by the cached path"
        );

        release(
            &mut app,
            1,
            Offset::new(20.0, 30.0),
            PointerDeviceKind::Touch,
        );
        assert_eq!(ups.get(), 1, "the release reached the pressed listener");
        assert!(
            !app.contains(pressed_listener.id()),
            "vacated with the path"
        );
    }

    #[test]
    fn a_gesture_detector_fires_its_tap_callbacks_end_to_end() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
        let down_log = Rc::clone(&log);
        let up_log = Rc::clone(&log);
        let tap_log = Rc::clone(&log);
        mount(
            &mut app,
            GestureDetector::new()
                .on_tap_down(Rc::new(move |_app, details: TapDownDetails| {
                    down_log
                        .borrow_mut()
                        .push(format!("down {:?}", details.local_position));
                }))
                .on_tap_up(Rc::new(move |_app, details: TapUpDetails| {
                    up_log
                        .borrow_mut()
                        .push(format!("up {:?}", details.global_position));
                }))
                .on_tap(Listener::new(move |_app| {
                    tap_log.borrow_mut().push("tap".into())
                }))
                .behavior(HitTestBehavior::Opaque)
                .child(Sized {
                    size: Size::new(100.0, 100.0),
                })
                .into_widget(),
        );

        tap_at(
            &mut app,
            1,
            Offset::new(20.0, 30.0),
            PointerDeviceKind::Touch,
        );
        assert_eq!(
            *log.borrow(),
            ["down Offset(20.0, 30.0)", "up Offset(20.0, 30.0)", "tap"]
        );
    }

    #[test]
    fn a_childless_detector_defaults_to_translucent_and_receives_the_tap() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let taps = Rc::new(Cell::new(0));
        let count = Rc::clone(&taps);
        mount(
            &mut app,
            GestureDetector::new()
                .on_tap(Listener::new(move |_app| count.set(count.get() + 1)))
                .into_widget(),
        );

        tap_at(
            &mut app,
            1,
            Offset::new(10.0, 10.0),
            PointerDeviceKind::Touch,
        );
        assert_eq!(taps.get(), 1);
    }

    #[test]
    fn a_detector_with_a_child_defers_to_it_and_misses_when_the_child_is_not_hit() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let taps = Rc::new(Cell::new(0));
        let count = Rc::clone(&taps);
        mount(
            &mut app,
            GestureDetector::new()
                .on_tap(Listener::new(move |_app| count.set(count.get() + 1)))
                .child(Sized {
                    size: Size::new(100.0, 100.0),
                })
                .into_widget(),
        );

        tap_at(
            &mut app,
            1,
            Offset::new(10.0, 10.0),
            PointerDeviceKind::Touch,
        );
        assert_eq!(taps.get(), 0);
    }

    #[test]
    fn a_long_press_is_recognized_after_the_timeout_and_the_tap_is_cancelled() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let log: Rc<RefCell<Vec<&'static str>>> = Rc::new(RefCell::new(Vec::new()));
        let tap_log = Rc::clone(&log);
        let cancel_log = Rc::clone(&log);
        let long_press_log = Rc::clone(&log);
        let up_log = Rc::clone(&log);
        mount(
            &mut app,
            GestureDetector::new()
                .on_tap(Listener::new(move |_app| tap_log.borrow_mut().push("tap")))
                .on_tap_cancel(Listener::new(move |_app| {
                    cancel_log.borrow_mut().push("tap_cancel")
                }))
                .on_long_press(Listener::new(move |_app| {
                    long_press_log.borrow_mut().push("long_press")
                }))
                .on_long_press_up(Listener::new(move |_app| {
                    up_log.borrow_mut().push("long_press_up")
                }))
                .into_widget(),
        );

        press(
            &mut app,
            1,
            Offset::new(10.0, 10.0),
            PointerDeviceKind::Touch,
        );
        assert!(log.borrow().is_empty());
        drop(app);
        cell.elapse(K_LONG_PRESS_TIMEOUT);
        let mut app = cell.borrow_mut();
        assert_eq!(*log.borrow(), ["tap_cancel", "long_press"]);
        release(
            &mut app,
            1,
            Offset::new(10.0, 10.0),
            PointerDeviceKind::Touch,
        );
        assert_eq!(*log.borrow(), ["tap_cancel", "long_press", "long_press_up"]);
    }

    #[test]
    fn supported_devices_is_respected() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let taps = Rc::new(Cell::new(0));
        let count = Rc::clone(&taps);
        mount(
            &mut app,
            GestureDetector::new()
                .on_tap(Listener::new(move |_app| count.set(count.get() + 1)))
                .supported_devices(HashSet::from([PointerDeviceKind::Mouse]))
                .into_widget(),
        );

        tap_at(
            &mut app,
            1,
            Offset::new(10.0, 10.0),
            PointerDeviceKind::Touch,
        );
        assert_eq!(taps.get(), 0, "a touch is not a supported device");
        tap_at(
            &mut app,
            2,
            Offset::new(10.0, 10.0),
            PointerDeviceKind::Mouse,
        );
        assert_eq!(taps.get(), 1);
    }

    // ---- RawGestureDetector ----

    #[test]
    fn rebuilding_reuses_the_recognizer_reruns_the_initializer_and_drops_it_when_removed() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let factory = CountingTapFactory {
            constructions: Rc::new(Cell::new(0)),
            initializations: Rc::new(Cell::new(0)),
            constructed: Rc::new(Cell::new(None)),
            taps: Rc::new(Cell::new(0)),
        };
        let harness = mount(&mut app, factory.detector());
        assert_eq!(factory.constructions.get(), 1);
        assert_eq!(factory.initializations.get(), 1);
        let recognizer = factory
            .constructed
            .get()
            .expect("constructed at init_state");

        harness.set_child(&mut app, factory.detector());
        harness.pump(&mut app);
        assert_eq!(factory.constructions.get(), 1, "the recognizer is reused");
        assert_eq!(
            factory.initializations.get(),
            2,
            "the initializer runs again"
        );
        tap_at(
            &mut app,
            1,
            Offset::new(10.0, 10.0),
            PointerDeviceKind::Touch,
        );
        assert_eq!(factory.taps.get(), 1);

        harness.set_child(&mut app, RawGestureDetector::new().into_widget());
        harness.pump(&mut app);
        assert!(
            !app.contains(recognizer),
            "a type no longer listed is disposed"
        );
        tap_at(
            &mut app,
            2,
            Offset::new(10.0, 10.0),
            PointerDeviceKind::Touch,
        );
        assert_eq!(factory.taps.get(), 1);
    }

    #[test]
    fn replace_gesture_recognizers_swaps_the_recognizers_during_layout() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let armed = Rc::new(Cell::new(false));
        let state_slot: Rc<Cell<Option<Handle<RawGestureDetectorState>>>> =
            Rc::new(Cell::new(None));
        let long_presses = Rc::new(Cell::new(0));
        let on_layout: Rc<dyn Fn(&mut App)> = {
            let armed = Rc::clone(&armed);
            let state_slot = Rc::clone(&state_slot);
            let long_presses = Rc::clone(&long_presses);
            Rc::new(move |app: &mut App| {
                if !armed.get() {
                    return;
                }
                let count = Rc::clone(&long_presses);
                let factory =
                    GestureRecognizerFactoryWithHandlers::<LongPressGestureRecognizer>::new(
                        LongPressGestureRecognizer::new,
                        move |app, instance| {
                            let count = Rc::clone(&count);
                            instance.set_on_long_press(
                                app,
                                Some(Listener::new(move |_app| count.set(count.get() + 1))),
                            );
                        },
                    );
                let state = state_slot.get().expect("mounted");
                state.replace_gesture_recognizers(
                    app,
                    vec![(
                        TypeId::of::<LongPressGestureRecognizer>(),
                        factory.into_factory(),
                    )],
                );
            })
        };
        let factory = CountingTapFactory {
            constructions: Rc::new(Cell::new(0)),
            initializations: Rc::new(Cell::new(0)),
            constructed: Rc::new(Cell::new(None)),
            taps: Rc::new(Cell::new(0)),
        };
        let tap_factory = GestureRecognizerFactoryWithHandlers::<TapGestureRecognizer>::new(
            {
                let constructed = Rc::clone(&factory.constructed);
                move |app| {
                    let recognizer = TapGestureRecognizer::new(app);
                    constructed.set(Some(recognizer.id()));
                    recognizer
                }
            },
            {
                let taps = Rc::clone(&factory.taps);
                move |app, instance| {
                    let taps = Rc::clone(&taps);
                    instance.set_on_tap(
                        app,
                        Some(Listener::new(move |_app| taps.set(taps.get() + 1))),
                    );
                }
            },
        );
        let harness = mount(
            &mut app,
            RawGestureDetector::new()
                .gestures(vec![(
                    TypeId::of::<TapGestureRecognizer>(),
                    tap_factory.into_factory(),
                )])
                .behavior(HitTestBehavior::Opaque)
                .child(LayoutProbe { on_layout })
                .into_widget(),
        );
        let state = detector_state(&harness, &app);
        state_slot.set(Some(state));
        let tap_recognizer = factory
            .constructed
            .get()
            .expect("constructed at init_state");
        assert_eq!(state.recognizers(&app).len(), 1);

        // Dirty the probe so the next layout pass, inside `flush_layout`, runs the swap.
        armed.set(true);
        let listener = harness
            .render_root(&app)
            .child(&app)
            .expect("the detector's listener")
            .as_object()
            .downcast::<RenderPointerListener>(&app)
            .expect("a RenderPointerListener");
        listener
            .child(&app)
            .expect("the probe")
            .as_object()
            .mark_needs_layout(&mut app);
        let pipeline = harness
            .render_root(&app)
            .as_object()
            .owner(&app)
            .expect("attached");
        pipeline.flush_layout(&mut app);
        armed.set(false);

        assert!(
            !app.contains(tap_recognizer),
            "the tap recognizer was disposed"
        );
        let recognizers = state.recognizers(&app);
        assert_eq!(recognizers.len(), 1);
        assert_eq!(recognizers[0].0, TypeId::of::<LongPressGestureRecognizer>());
        assert_eq!(recognizers[0].1.debug_description(), "long press");

        press(
            &mut app,
            1,
            Offset::new(10.0, 10.0),
            PointerDeviceKind::Touch,
        );
        drop(app);
        cell.elapse(K_LONG_PRESS_TIMEOUT);
        let mut app = cell.borrow_mut();
        release(
            &mut app,
            1,
            Offset::new(10.0, 10.0),
            PointerDeviceKind::Touch,
        );
        assert_eq!(long_presses.get(), 1);
        assert_eq!(factory.taps.get(), 0);
    }

    #[test]
    fn unmounting_disposes_the_recognizers() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = mount(
            &mut app,
            GestureDetector::new()
                .on_tap(Listener::new(|_app| {}))
                .on_long_press(Listener::new(|_app| {}))
                .into_widget(),
        );
        let state = detector_state(&harness, &app);
        let recognizers: Vec<HandleId> = state
            .recognizer_handles(&app)
            .into_iter()
            .map(AnyGestureRecognizer::id)
            .collect();
        assert_eq!(recognizers.len(), 2);
        assert_eq!(
            format!("{:?}", app.get(state)),
            format!(
                "RawGestureDetectorState {{ gestures: [\"tap\", \"long press\"], recognizers: {:?} }}",
                state.recognizers(&app)
            )
        );

        harness.set_child(
            &mut app,
            Sized {
                size: Size::new(5.0, 5.0),
            }
            .into_widget(),
        );
        harness.pump(&mut app);
        for recognizer in recognizers {
            assert!(!app.contains(recognizer), "dispose freed the recognizer");
        }
        assert!(
            !app.contains(state),
            "the state slot is freed after dispose"
        );
    }
}
