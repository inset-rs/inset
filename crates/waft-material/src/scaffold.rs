//! Flutter counterpart: material_ui's `scaffold.dart`.
//!
//! `_BodyBoxConstraints` and `_BottomSheetBoxConstraints` are `BoxConstraints` subclass
//! instances ([`AnyBoxConstraints`]). They reach the body's and the sheet's `LayoutBuilder`
//! only because nothing but `LayoutId` and `MediaQuery` sits between the scaffold's layout and
//! those builders: a render object in between would hand on the plain value, and the builders'
//! casts would panic.

use std::any::Any;
use std::cell::Cell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animatable, Animation, AnimationController, AnimationMin, AnimationStatus,
    AnimationStatusListener, AnimationStyle, AnyAnimation, Curve, CurveTween, CurvedAnimation,
    Curves, Interval, Split, TrainHoppingAnimation, Tween,
};
use waft_embedder::{Clip, Color, Offset, Rect, Size, TargetPlatform, TextDirection, clamp_double};
use waft_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Completer, CompleterFuture, Handle, HandleId,
    Listenable, Listener, PRECISION_ERROR_TOLERANCE, Timer, UniqueKey, ValueKey, VoidCallback,
};
use waft_gestures::{DragEndDetails, DragStartDetails, GestureBinding, HitTestResult};
use waft_painting::{
    AlignmentDirectional, AlignmentGeometry, Border, BoxDecoration, EdgeInsets, EdgeInsetsGeometry,
    ShapeBorder,
};
use waft_rendering::{
    AnyBoxConstraints, BoxConstraints, BoxConstraintsExtension, BoxHitTestEntry, ChildLayoutId,
    HitTestBehavior, MultiChildLayoutChildren, MultiChildLayoutDelegate,
};
use waft_scheduler::{Ticker, TickerCallback, TickerFuture, TickerProviderObject};
use waft_widgets::{
    Action, ActionData, Actions, Align, AnyModalRoute, BuildContext, Builder, ConstrainedBox,
    Container, CustomMultiChildLayout, DismissAction, DismissIntent, DraggableScrollableActuator,
    DraggableScrollableNotification, FadeTransition, GlobalKey, InheritedModel, InheritedModelKind,
    InheritedWidget, IntoWidget, IntrinsicHeight, KeyRef, KeyedSubtree, LayoutBuilder, LayoutId,
    ListenableBuilder, LocalHistoryEntry, MediaQuery, MetaData, ModalBarrier, NotificationListener,
    OverflowBar, OverflowBarAlignment, Padding, PreferredSizeWidgetRef, PrimaryScrollController,
    RestorableBool, RestorableProperty, RestorableValue, RestorationBucket, RestorationMixin,
    RestorationMixinData, RotationTransition, SafeArea, ScaleTransition,
    ScrollNotificationObserver, Semantics, SizedBox, Stack, State, StateData, StateSetter,
    StatefulBuilder, StatefulBuilderState, StatefulWidget, StatelessWidget,
    TickerProviderStateMixin, TickerProviderStateMixinData, View, WidgetBuilder, WidgetRef,
    WidgetsBinding, WidgetsBindingObserverObject, WidgetsBindingObserverRef, downcast_widget,
};

use crate::app_bar::AppBar;
use crate::banner::{MaterialBanner, MaterialBannerClosedReason};
use crate::banner_theme::MaterialBannerTheme;
use crate::bottom_sheet::BottomSheet;
use crate::colors::Colors;
#[expect(
    deprecated,
    reason = "Dart's `_standardBottomSheetCurve` is the deprecated `standardEasing`"
)]
use crate::curves::standard_easing;
use crate::divider::Divider;
use crate::drawer::{DrawerAlignment, DrawerCallback, DrawerController, DrawerControllerState};
use crate::flexible_space_bar::FlexibleSpaceBar;
use crate::floating_action_button::FloatingActionButton;
use crate::floating_action_button_location::{
    FloatingActionButtonAnimator, FloatingActionButtonAnimatorRef, FloatingActionButtonLocation,
    FloatingActionButtonLocationRef, K_FLOATING_ACTION_BUTTON_SEGUE,
    K_FLOATING_ACTION_BUTTON_TURN_INTERVAL,
};
use crate::material::{Material, destroy_lingering};
use crate::snack_bar::{SnackBar, SnackBarClosedReason};
use crate::snack_bar_theme::{SnackBarBehavior, SnackBarTheme};
use crate::theme::Theme;

/// Dart's `_kDefaultFloatingActionButtonLocation`.
fn default_floating_action_button_location() -> FloatingActionButtonLocationRef {
    <dyn FloatingActionButtonLocation>::end_float()
}

/// Dart's `_kDefaultFloatingActionButtonAnimator`.
fn default_floating_action_button_animator() -> FloatingActionButtonAnimatorRef {
    <dyn FloatingActionButtonAnimator>::scaling()
}

/// Dart's `_standardBottomSheetCurve`.
#[expect(
    deprecated,
    reason = "Dart's `_standardBottomSheetCurve` is the deprecated `standardEasing`"
)]
fn standard_bottom_sheet_curve() -> Rc<dyn Curve> {
    standard_easing()
}

// When the top of the BottomSheet crosses this threshold, it will start to
// shrink the FAB and show a scrim.
const K_BOTTOM_SHEET_DOMINATES_PERCENTAGE: f64 = 0.3;
const K_MIN_BOTTOM_SHEET_SCRIM_OPACITY: f64 = 0.1;
const K_MAX_BOTTOM_SHEET_SCRIM_OPACITY: f64 = 0.6;

/// Dart's `_ScaffoldSlot`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum ScaffoldSlot {
    Body,
    AppBar,
    BodyScrim,
    BottomSheet,
    SnackBar,
    MaterialBanner,
    PersistentFooter,
    BottomNavigationBar,
    FloatingActionButton,
    Drawer,
    EndDrawer,
    StatusBar,
}

impl ScaffoldSlot {
    /// The [`ChildLayoutId`] this slot is known by, Dart's enum value as the `LayoutId.id`.
    fn id(self) -> ChildLayoutId {
        Rc::new(ValueKey::new(self))
    }
}

/// Manages [`SnackBar`]s and [`MaterialBanner`]s for descendant [`Scaffold`]s.
///
/// This class provides APIs for showing snack bars and material banners at the
/// bottom and top of the screen, respectively.
///
/// To display one of these notifications, obtain the [`ScaffoldMessengerState`]
/// for the current [`BuildContext`] via [`ScaffoldMessenger::of`] and use the
/// [`ScaffoldMessengerState::show_snack_bar`] or the
/// [`ScaffoldMessengerState::show_material_banner`] functions.
///
/// When the [`ScaffoldMessenger`] has nested [`Scaffold`] descendants, the
/// ScaffoldMessenger will only present the notification to the root Scaffold of
/// the subtree of Scaffolds. In order to show notifications for the inner, nested
/// Scaffolds, set a new scope by instantiating a new ScaffoldMessenger in
/// between the levels of nesting.
///
/// See also:
///
///  * [`SnackBar`], which is a temporary notification typically shown near the
///    bottom of the app using the [`ScaffoldMessengerState::show_snack_bar`] method.
///  * [`MaterialBanner`], which is a temporary notification typically shown at the
///    top of the app using the [`ScaffoldMessengerState::show_material_banner`] method.
///  * [`debug_check_has_scaffold_messenger`](crate::debug_check_has_scaffold_messenger),
///    which asserts that the given context has a [`ScaffoldMessenger`] ancestor.
#[derive(Debug)]
pub struct ScaffoldMessenger {
    pub key: Option<KeyRef>,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl ScaffoldMessenger {
    /// Creates a widget that manages [`SnackBar`]s for [`Scaffold`] descendants.
    pub fn new<K>(child: impl IntoWidget<K>) -> ScaffoldMessenger {
        ScaffoldMessenger {
            key: None,
            child: child.into_widget(),
        }
    }

    /// Dart `ScaffoldMessenger(key:)`.
    pub fn key(mut self, key: KeyRef) -> ScaffoldMessenger {
        self.key = Some(key);
        self
    }

    /// The state from the closest instance of this class that encloses the given
    /// context.
    ///
    /// Typical usage of the [`ScaffoldMessenger::of`] function is to call it in
    /// response to a user gesture or an application state change.
    ///
    /// A less elegant but more expedient solution is to assign a [`GlobalKey`] to the
    /// [`ScaffoldMessenger`], then use the `key.current_state` method to obtain the
    /// [`ScaffoldMessengerState`] rather than using the [`ScaffoldMessenger::of`]
    /// function. The `MaterialApp.scaffoldMessengerKey` refers to the root
    /// ScaffoldMessenger that is provided by default.
    ///
    /// If there is no [`ScaffoldMessenger`] in scope, then this will assert in
    /// debug mode, and throw an exception in release mode.
    ///
    /// See also:
    ///
    ///  * [`maybe_of`](Self::maybe_of), which is a similar function but will return `None`
    ///    instead of throwing if there is no [`ScaffoldMessenger`] ancestor.
    ///  * [`debug_check_has_scaffold_messenger`](crate::debug_check_has_scaffold_messenger),
    ///    which asserts that the given context has a [`ScaffoldMessenger`] ancestor.
    pub fn of(app: &mut App, context: BuildContext) -> Handle<ScaffoldMessengerState> {
        debug_assert!(crate::debug::debug_check_has_scaffold_messenger(
            app, context
        ));

        let scope = context
            .depend_on_inherited_widget_of_exact_type::<ScaffoldMessengerScope>(app)
            .expect("checked by debug_check_has_scaffold_messenger");
        scope.scaffold_messenger_state
    }

    /// The state from the closest instance of this class that encloses the given
    /// context, if any.
    ///
    /// Will return `None` if a [`ScaffoldMessenger`] is not found in the given context.
    ///
    /// See also:
    ///
    ///  * [`of`](Self::of), which is a similar function, except that it will throw an
    ///    exception if a [`ScaffoldMessenger`] is not found in the given context.
    pub fn maybe_of(
        app: &mut App,
        context: BuildContext,
    ) -> Option<Handle<ScaffoldMessengerState>> {
        let scope = context.depend_on_inherited_widget_of_exact_type::<ScaffoldMessengerScope>(app);
        scope.map(|scope| scope.scaffold_messenger_state)
    }
}

impl StatefulWidget for ScaffoldMessenger {
    type State = ScaffoldMessengerState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ScaffoldMessengerState {
        ScaffoldMessengerState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            scaffolds: Vec::new(),
            material_banners: VecDeque::new(),
            material_banner_controller: None,
            snack_bars: VecDeque::new(),
            snack_bar_controller: None,
            snack_bar_timer: None,
            accessible_navigation: false,
        }
    }
}

/// State for a [`ScaffoldMessenger`].
///
/// A [`ScaffoldMessengerState`] object can be used to [`show_snack_bar`](Self::show_snack_bar)
/// or [`show_material_banner`](Self::show_material_banner) for every registered [`Scaffold`]
/// that is a descendant of the associated [`ScaffoldMessenger`]. Scaffolds will register to
/// receive [`SnackBar`]s and [`MaterialBanner`]s from their closest ScaffoldMessenger
/// ancestor.
///
/// Typically obtained via [`ScaffoldMessenger::of`].
pub struct ScaffoldMessengerState {
    state: StateData<ScaffoldMessenger>,
    ticker_provider: TickerProviderStateMixinData,
    /// Dart's `LinkedHashSet<ScaffoldState>`: insertion order, each scaffold once.
    scaffolds: Vec<Handle<ScaffoldState>>,
    material_banners:
        VecDeque<Rc<ScaffoldFeatureController<MaterialBanner, MaterialBannerClosedReason>>>,
    material_banner_controller: Option<Handle<AnimationController>>,
    snack_bars: VecDeque<Rc<ScaffoldFeatureController<SnackBar, SnackBarClosedReason>>>,
    snack_bar_controller: Option<Handle<AnimationController>>,
    snack_bar_timer: Option<Timer>,
    /// Dart's `late bool _accessibleNavigation`, set before the first build.
    accessible_navigation: bool,
}

impl ScaffoldMessengerState {
    fn register(self: Handle<Self>, app: &mut App, scaffold: Handle<ScaffoldState>) {
        if !app.get(self).scaffolds.contains(&scaffold) {
            app.get_mut(self).scaffolds.push(scaffold);
        }

        if self.is_root(app, scaffold) {
            if !app.get(self).snack_bars.is_empty() {
                scaffold.update_snack_bar(app);
            }

            if !app.get(self).material_banners.is_empty() {
                scaffold.update_material_banner(app);
            }
        }
    }

    fn unregister(self: Handle<Self>, app: &mut App, scaffold: Handle<ScaffoldState>) {
        let scaffolds = &mut app.get_mut(self).scaffolds;
        let position = scaffolds
            .iter()
            .position(|registered| *registered == scaffold);
        // ScaffoldStates should only be removed once.
        debug_assert!(position.is_some());
        if let Some(position) = position {
            scaffolds.remove(position);
        }
    }

    fn update_scaffolds(self: Handle<Self>, app: &mut App) {
        let scaffolds = app.get(self).scaffolds.clone();
        for scaffold in scaffolds {
            if self.is_root(app, scaffold) {
                scaffold.update_snack_bar(app);
                scaffold.update_material_banner(app);
            }
        }
    }

    // Nested Scaffolds are handled by the ScaffoldMessenger by only presenting a
    // MaterialBanner or SnackBar in the root Scaffold of the nested set.
    fn is_root(self: Handle<Self>, app: &App, scaffold: Handle<ScaffoldState>) -> bool {
        let parent = scaffold
            .context(app)
            .find_ancestor_state_of_type::<ScaffoldState>(app);
        match parent {
            None => true,
            Some(parent) => !app.get(self).scaffolds.contains(&parent),
        }
    }

    // SNACKBAR API

    /// Shows a [`SnackBar`] across all registered [`Scaffold`]s. Scaffolds register
    /// to receive snack bars from their closest [`ScaffoldMessenger`] ancestor.
    /// If there are several registered scaffolds the snack bar is shown
    /// simultaneously on all of them.
    ///
    /// A scaffold can show at most one snack bar at a time. If this function is
    /// called while another snack bar is already visible, the given snack bar
    /// will be added to a queue and displayed after the earlier snack bars have
    /// closed.
    ///
    /// To control how long a [`SnackBar`] remains visible, use [`SnackBar::duration`].
    ///
    /// To remove the [`SnackBar`] with an exit animation, use
    /// [`hide_current_snack_bar`](Self::hide_current_snack_bar) or call
    /// [`ScaffoldFeatureController::close`] on the returned [`ScaffoldFeatureController`]. To
    /// remove a [`SnackBar`] suddenly (without an animation), use
    /// [`remove_current_snack_bar`](Self::remove_current_snack_bar).
    ///
    /// See [`ScaffoldMessenger::of`] for information about how to obtain the
    /// [`ScaffoldMessengerState`].
    ///
    /// ## Relative positioning of floating SnackBars
    ///
    /// A [`SnackBar`] with [`SnackBar::behavior`] set to [`SnackBarBehavior::Floating`] is
    /// positioned above the widgets provided to [`Scaffold::floating_action_button`],
    /// [`Scaffold::persistent_footer_buttons`], and [`Scaffold::bottom_navigation_bar`].
    /// If some or all of these widgets take up enough space such that the SnackBar
    /// would not be visible when positioned above them, an error will be thrown.
    /// In this case, consider constraining the size of these widgets to allow room for
    /// the SnackBar to be visible.
    ///
    /// If [`AnimationStyle::duration`] is provided in the `snack_bar_animation_style`
    /// parameter, it will be used to override the snackbar show animation duration.
    /// Otherwise, defaults to 250ms.
    ///
    /// If [`AnimationStyle::reverse_duration`] is provided in the
    /// `snack_bar_animation_style` parameter, it will be used to override the snackbar hide
    /// animation duration. Otherwise, defaults to 250ms.
    ///
    /// To disable the snackbar animation, use `AnimationStyle::no_animation()`.
    pub fn show_snack_bar(
        self: Handle<Self>,
        app: &mut App,
        snack_bar: SnackBar,
        snack_bar_animation_style: Option<AnimationStyle>,
    ) -> Rc<ScaffoldFeatureController<SnackBar, SnackBarClosedReason>> {
        debug_assert!(
            !app.get(self).scaffolds.is_empty(),
            "ScaffoldMessenger.showSnackBar was called, but there are currently no \
             descendant Scaffolds to present to."
        );
        self.did_update_animation_style(app, snack_bar_animation_style.as_ref());
        if app.get(self).snack_bar_controller.is_none() {
            let controller = SnackBar::create_animation_controller(
                app,
                self,
                snack_bar_animation_style
                    .as_ref()
                    .and_then(|style| style.duration),
                snack_bar_animation_style
                    .as_ref()
                    .and_then(|style| style.reverse_duration),
            );
            controller.add_status_listener(
                app,
                AnimationStatusListener::handle_method(
                    self,
                    ScaffoldMessengerState::handle_snack_bar_status_changed,
                ),
            );
            app.get_mut(self).snack_bar_controller = Some(controller);
        }
        let snack_bar_controller = self.snack_bar_controller(app);
        if app.get(self).snack_bars.is_empty() {
            debug_assert!(snack_bar_controller.is_dismissed(app));
            snack_bar_controller.forward(app, None);
        }
        let controller = Rc::new_cyclic(
            |controller: &std::rc::Weak<
                ScaffoldFeatureController<SnackBar, SnackBarClosedReason>,
            >| {
                let controller = controller.clone();
                ScaffoldFeatureController::new(
                    // We provide a fallback key so that if back-to-back snackbars happen to
                    // match in structure, material ink splashes and highlights don't survive
                    // from one to the next.
                    snack_bar.with_animation(
                        snack_bar_controller.view(),
                        Some(KeyRef::Local(Rc::new(UniqueKey::new()))),
                    ),
                    Completer::new(),
                    Rc::new(move |app: &mut App| {
                        debug_assert!(
                            app.get(self)
                                .snack_bars
                                .front()
                                .zip(controller.upgrade())
                                .is_some_and(|(first, controller)| Rc::ptr_eq(first, &controller))
                        );
                        self.hide_current_snack_bar(app, SnackBarClosedReason::Hide);
                    }),
                    // SnackBar doesn't use a builder function so setState() wouldn't rebuild it
                    None,
                )
            },
        );
        let added = Rc::clone(&controller);
        let shown = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.set_state(app, move |state| {
                state.snack_bars.push_back(added);
            });
            self.update_scaffolds(app);
        }));
        if let Err(exception) = shown {
            if cfg!(debug_assertions) && is_set_state_during_build(&*exception) {
                panic!(
                    "The showSnackBar() method cannot be called during build.\n\
                     The showSnackBar() method was called during build, which is prohibited \
                     as showing snack bars requires updating state. Updating state is not \
                     possible during build.\n\
                     Instead of calling showSnackBar() during build, call it directly in your \
                     on tap (and related) callbacks. If you need to immediately show a snack \
                     bar, make the call in initState() or didChangeDependencies() instead. \
                     Otherwise, you can also schedule a post-frame callback using \
                     SchedulerBinding.addPostFrameCallback to show the snack bar after the \
                     current frame."
                );
            }
            std::panic::resume_unwind(exception);
        }

        controller
    }

    fn did_update_animation_style(
        self: Handle<Self>,
        app: &mut App,
        snack_bar_animation_style: Option<&AnimationStyle>,
    ) {
        if let Some(snack_bar_animation_style) = snack_bar_animation_style {
            let controller = app.get(self).snack_bar_controller;
            let (duration, reverse_duration) = match controller {
                Some(controller) => {
                    let controller = app.get(controller);
                    (controller.duration, controller.reverse_duration)
                }
                None => (None, None),
            };
            if duration != snack_bar_animation_style.duration
                || reverse_duration != snack_bar_animation_style.reverse_duration
            {
                if let Some(controller) = controller {
                    controller.dispose(app);
                }
                app.get_mut(self).snack_bar_controller = None;
            }
        }
    }

    fn snack_bar_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .snack_bar_controller
            .expect("a snack bar was shown")
    }

    fn material_banner_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .material_banner_controller
            .expect("a material banner was shown")
    }

    fn handle_snack_bar_status_changed(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        match status {
            AnimationStatus::Dismissed => {
                debug_assert!(!app.get(self).snack_bars.is_empty());
                self.set_state(app, |state| {
                    state.snack_bars.pop_front();
                });
                self.update_scaffolds(app);
                if !app.get(self).snack_bars.is_empty() {
                    self.snack_bar_controller(app).forward(app, None);
                }
            }
            AnimationStatus::Completed => {
                self.set_state(app, |state| {
                    debug_assert!(state.snack_bar_timer.is_none());
                    // build will create a new timer if necessary to dismiss the snackBar.
                });
                self.update_scaffolds(app);
            }
            AnimationStatus::Forward | AnimationStatus::Reverse => {}
        }
    }

    /// Removes the current [`SnackBar`] (if any) immediately from registered
    /// [`Scaffold`]s.
    ///
    /// The removed snack bar does not run its normal exit animation. If there are
    /// any queued snack bars, they begin their entrance animation immediately.
    ///
    /// Dart's default `reason` is [`SnackBarClosedReason::Remove`].
    pub fn remove_current_snack_bar(
        self: Handle<Self>,
        app: &mut App,
        reason: SnackBarClosedReason,
    ) {
        let Some(first) = app.get(self).snack_bars.front().cloned() else {
            return;
        };
        let completer = first.completer.clone();
        if !completer.is_completed() {
            completer.complete(app, reason);
        }
        if let Some(timer) = app.get_mut(self).snack_bar_timer.take() {
            timer.cancel(app);
        }
        // This will trigger the animation's status callback.
        self.snack_bar_controller(app).set_value(app, 0.0);
    }

    /// Removes the current [`SnackBar`] by running its normal exit animation.
    ///
    /// The closed completer is called after the animation is complete.
    ///
    /// Dart's default `reason` is [`SnackBarClosedReason::Hide`].
    pub fn hide_current_snack_bar(self: Handle<Self>, app: &mut App, reason: SnackBarClosedReason) {
        if app.get(self).snack_bars.is_empty() || self.snack_bar_controller(app).is_dismissed(app) {
            return;
        }
        let completer = app
            .get(self)
            .snack_bars
            .front()
            .expect("checked above")
            .completer
            .clone();
        let controller = self.snack_bar_controller(app);
        if app.get(self).accessible_navigation {
            controller.set_value(app, 0.0);
            completer.complete(app, reason);
        } else {
            controller.reverse(app, None).when_complete(
                app,
                Listener::new(move |app| {
                    debug_assert!(self.mounted(app));
                    if !completer.is_completed() {
                        completer.complete(app, reason);
                    }
                }),
            );
        }
        if let Some(timer) = app.get_mut(self).snack_bar_timer.take() {
            timer.cancel(app);
        }
    }

    /// Removes all the snackBars currently in queue by clearing the queue
    /// and running normal exit animation on the current snackBar.
    pub fn clear_snack_bars(self: Handle<Self>, app: &mut App) {
        if app.get(self).snack_bars.is_empty() || self.snack_bar_controller(app).is_dismissed(app) {
            return;
        }
        let snack_bars = &mut app.get_mut(self).snack_bars;
        let current_snackbar = snack_bars.pop_front().expect("checked above");
        snack_bars.clear();
        snack_bars.push_back(current_snackbar);
        self.hide_current_snack_bar(app, SnackBarClosedReason::Hide);
    }

    // MATERIAL BANNER API

    /// Shows a [`MaterialBanner`] across all registered [`Scaffold`]s. Scaffolds register
    /// to receive material banners from their closest [`ScaffoldMessenger`] ancestor.
    /// If there are several registered scaffolds the material banner is shown
    /// simultaneously on all of them.
    ///
    /// A scaffold can show at most one material banner at a time. If this function is
    /// called while another material banner is already visible, the given material banner
    /// will be added to a queue and displayed after the earlier material banners have
    /// closed.
    ///
    /// To remove the [`MaterialBanner`] with an exit animation, use
    /// [`hide_current_material_banner`](Self::hide_current_material_banner) or call
    /// [`ScaffoldFeatureController::close`] on the returned [`ScaffoldFeatureController`]. To
    /// remove a [`MaterialBanner`] suddenly (without an animation), use
    /// [`remove_current_material_banner`](Self::remove_current_material_banner).
    ///
    /// When this [`ScaffoldMessenger`] is above the `Navigator`, such as the one
    /// created by `MaterialApp`, a [`MaterialBanner`] stays visible after the route
    /// that showed it is popped, and that route's [`BuildContext`] is deactivated.
    /// Because of this, callbacks in [`MaterialBanner::actions`] should not call
    /// [`ScaffoldMessenger::of`] with a [`BuildContext`] from the route that showed the
    /// banner. Instead, wrap the action widget in a [`Builder`] to obtain an active
    /// [`BuildContext`], or save the [`ScaffoldMessengerState`] reference before calling
    /// [`show_material_banner`](Self::show_material_banner).
    ///
    /// See [`ScaffoldMessenger::of`] for information about how to obtain the
    /// [`ScaffoldMessengerState`].
    pub fn show_material_banner(
        self: Handle<Self>,
        app: &mut App,
        material_banner: MaterialBanner,
    ) -> Rc<ScaffoldFeatureController<MaterialBanner, MaterialBannerClosedReason>> {
        debug_assert!(
            !app.get(self).scaffolds.is_empty(),
            "ScaffoldMessenger.showMaterialBanner was called, but there are currently no \
             descendant Scaffolds to present to."
        );
        if app.get(self).material_banner_controller.is_none() {
            let controller = MaterialBanner::create_animation_controller(app, self);
            controller.add_status_listener(
                app,
                AnimationStatusListener::handle_method(
                    self,
                    ScaffoldMessengerState::handle_material_banner_status_changed,
                ),
            );
            app.get_mut(self).material_banner_controller = Some(controller);
        }
        let material_banner_controller = self.material_banner_controller(app);
        if app.get(self).material_banners.is_empty() {
            debug_assert!(material_banner_controller.is_dismissed(app));
            material_banner_controller.forward(app, None);
        }
        let controller = Rc::new_cyclic(
            |controller: &std::rc::Weak<
                ScaffoldFeatureController<MaterialBanner, MaterialBannerClosedReason>,
            >| {
                let controller = controller.clone();
                ScaffoldFeatureController::new(
                    // We provide a fallback key so that if back-to-back material banners happen
                    // to match in structure, material ink splashes and highlights don't survive
                    // from one to the next.
                    material_banner.with_animation(
                        material_banner_controller.view(),
                        Some(KeyRef::Local(Rc::new(UniqueKey::new()))),
                    ),
                    Completer::new(),
                    Rc::new(move |app: &mut App| {
                        debug_assert!(
                            app.get(self)
                                .material_banners
                                .front()
                                .zip(controller.upgrade())
                                .is_some_and(|(first, controller)| Rc::ptr_eq(first, &controller))
                        );
                        self.hide_current_material_banner(app, MaterialBannerClosedReason::Hide);
                    }),
                    // MaterialBanner doesn't use a builder function so setState() wouldn't
                    // rebuild it
                    None,
                )
            },
        );
        let added = Rc::clone(&controller);
        self.set_state(app, move |state| {
            state.material_banners.push_back(added);
        });
        self.update_scaffolds(app);
        controller
    }

    fn handle_material_banner_status_changed(
        self: Handle<Self>,
        app: &mut App,
        status: AnimationStatus,
    ) {
        match status {
            AnimationStatus::Dismissed => {
                debug_assert!(!app.get(self).material_banners.is_empty());
                self.set_state(app, |state| {
                    state.material_banners.pop_front();
                });
                self.update_scaffolds(app);
                if !app.get(self).material_banners.is_empty() {
                    self.material_banner_controller(app).forward(app, None);
                }
            }
            AnimationStatus::Completed => {
                self.update_scaffolds(app);
            }
            AnimationStatus::Forward | AnimationStatus::Reverse => {}
        }
    }

    /// Removes the current [`MaterialBanner`] (if any) immediately from registered
    /// [`Scaffold`]s.
    ///
    /// The removed material banner does not run its normal exit animation. If there are
    /// any queued material banners, they begin their entrance animation immediately.
    ///
    /// Dart's default `reason` is [`MaterialBannerClosedReason::Remove`].
    pub fn remove_current_material_banner(
        self: Handle<Self>,
        app: &mut App,
        reason: MaterialBannerClosedReason,
    ) {
        let Some(first) = app.get(self).material_banners.front().cloned() else {
            return;
        };
        let completer = first.completer.clone();
        if !completer.is_completed() {
            completer.complete(app, reason);
        }

        // This will trigger the animation's status callback.
        self.material_banner_controller(app).set_value(app, 0.0);
    }

    /// Removes the current [`MaterialBanner`] by running its normal exit animation.
    ///
    /// The closed completer is called after the animation is complete.
    ///
    /// Dart's default `reason` is [`MaterialBannerClosedReason::Hide`].
    pub fn hide_current_material_banner(
        self: Handle<Self>,
        app: &mut App,
        reason: MaterialBannerClosedReason,
    ) {
        if app.get(self).material_banners.is_empty()
            || self.material_banner_controller(app).is_dismissed(app)
        {
            return;
        }
        let completer = app
            .get(self)
            .material_banners
            .front()
            .expect("checked above")
            .completer
            .clone();
        let controller = self.material_banner_controller(app);
        if app.get(self).accessible_navigation {
            controller.set_value(app, 0.0);
            completer.complete(app, reason);
        } else {
            controller.reverse(app, None).when_complete(
                app,
                Listener::new(move |app| {
                    debug_assert!(self.mounted(app));
                    if !completer.is_completed() {
                        completer.complete(app, reason);
                    }
                }),
            );
        }
    }

    /// Removes all the [`MaterialBanner`]s currently in queue by clearing the queue
    /// and running normal exit animation on the current [`MaterialBanner`].
    pub fn clear_material_banners(self: Handle<Self>, app: &mut App) {
        if app.get(self).material_banners.is_empty()
            || self.material_banner_controller(app).is_dismissed(app)
        {
            return;
        }
        let material_banners = &mut app.get_mut(self).material_banners;
        let current_material_banner = material_banners.pop_front().expect("checked above");
        material_banners.clear();
        material_banners.push_back(current_material_banner);
        self.hide_current_material_banner(app, MaterialBannerClosedReason::Hide);
    }
}

impl TickerProviderStateMixin for ScaffoldMessengerState {
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
impl TickerProviderObject for ScaffoldMessengerState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for ScaffoldMessengerState {
    type Widget = ScaffoldMessenger;
    waft_widgets::state_accessors!();

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        app.get_mut(self).accessible_navigation =
            MediaQuery::accessible_navigation_of(app, context);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(waft_widgets::debug_check_has_media_query(app, context));
        app.get_mut(self).accessible_navigation =
            MediaQuery::accessible_navigation_of(app, context);

        if let Some(first) = app.get(self).snack_bars.front().cloned() {
            let route = AnyModalRoute::of(app, context);
            if route.is_none_or(|route| route.as_route().is_current(app)) {
                let controller = self.snack_bar_controller(app);
                if controller.is_completed(app) && app.get(self).snack_bar_timer.is_none() {
                    let snack_bar = first.widget();
                    let persist = snack_bar.effective_persist();
                    let timer = Timer::new(
                        app,
                        snack_bar.duration,
                        Listener::new(move |app| {
                            debug_assert!(controller.is_forward_or_completed(app));
                            // Look up MediaQuery again in case the setting changed.
                            if persist {
                                return;
                            }
                            self.hide_current_snack_bar(app, SnackBarClosedReason::Timeout);
                        }),
                    );
                    app.get_mut(self).snack_bar_timer = Some(timer);
                }
            }
        }

        ScaffoldMessengerScope {
            scaffold_messenger_state: self,
            child: self.widget(app).child.clone(),
        }
        .into_widget()
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(controller) = app.get(self).material_banner_controller {
            controller.dispose(app);
        }
        if let Some(controller) = app.get(self).snack_bar_controller {
            controller.dispose(app);
        }
        if let Some(timer) = app.get_mut(self).snack_bar_timer.take() {
            timer.cancel(app);
        }
        TickerProviderStateMixin::dispose(self, app);
    }
}

/// Dart's `_ScaffoldMessengerScope`.
#[derive(Debug)]
struct ScaffoldMessengerScope {
    scaffold_messenger_state: Handle<ScaffoldMessengerState>,
    child: WidgetRef,
}

impl InheritedWidget for ScaffoldMessengerScope {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old: &ScaffoldMessengerScope) -> bool {
        self.scaffold_messenger_state != old.scaffold_messenger_state
    }
}

/// The geometry of the [`Scaffold`] after all its contents have been laid out
/// except the `FloatingActionButton`.
///
/// The [`Scaffold`] passes this pre-layout geometry to its
/// [`FloatingActionButtonLocation`], which produces an `Offset` that the
/// [`Scaffold`] uses to position the `FloatingActionButton`.
///
/// For a description of the [`Scaffold`]'s geometry after it has
/// finished laying out, see the [`ScaffoldGeometry`].
#[derive(Clone, Copy, Debug)]
pub struct ScaffoldPrelayoutGeometry {
    /// The [`Size`] of [`Scaffold::floating_action_button`].
    ///
    /// If [`Scaffold::floating_action_button`] is null, this will be [`Size::ZERO`].
    pub floating_action_button_size: Size,

    /// The [`Size`] of the [`Scaffold`]'s [`BottomSheet`].
    ///
    /// If the [`Scaffold`] is not currently showing a [`BottomSheet`],
    /// this will be [`Size::ZERO`].
    pub bottom_sheet_size: Size,

    /// The vertical distance from the Scaffold's origin to the bottom of
    /// [`Scaffold::body`].
    ///
    /// This is useful in a [`FloatingActionButtonLocation`] designed to
    /// place the `FloatingActionButton` at the bottom of the screen, while
    /// keeping it above the [`BottomSheet`], the [`Scaffold::bottom_navigation_bar`],
    /// or the keyboard.
    ///
    /// The [`Scaffold::body`] is laid out with respect to [`min_insets`](Self::min_insets)
    /// already. This means that a [`FloatingActionButtonLocation`] does not need to factor in
    /// `EdgeInsets.bottom` of [`min_insets`](Self::min_insets) when aligning a
    /// `FloatingActionButton` to [`content_bottom`](Self::content_bottom).
    pub content_bottom: f64,

    /// The vertical distance from the [`Scaffold`]'s origin to the top of
    /// [`Scaffold::body`].
    ///
    /// This is useful in a [`FloatingActionButtonLocation`] designed to
    /// place the `FloatingActionButton` at the top of the screen, while
    /// keeping it below the `Scaffold.appBar`.
    ///
    /// The [`Scaffold::body`] is laid out with respect to [`min_insets`](Self::min_insets)
    /// already. This means that a [`FloatingActionButtonLocation`] does not need to factor in
    /// `EdgeInsets.top` of [`min_insets`](Self::min_insets) when aligning a
    /// `FloatingActionButton` to [`content_top`](Self::content_top).
    pub content_top: f64,

    /// The minimum padding to inset the `FloatingActionButton` by for it
    /// to remain visible.
    ///
    /// This value is the result of calling `MediaQueryData.padding` in the
    /// [`Scaffold`]'s [`BuildContext`],
    /// and is useful for insetting the `FloatingActionButton` to avoid features like
    /// the system status bar or the keyboard.
    ///
    /// If [`Scaffold::resize_to_avoid_bottom_inset`] is set to false,
    /// `EdgeInsets.bottom` of [`min_insets`](Self::min_insets) will be 0.0.
    pub min_insets: EdgeInsets,

    /// The minimum padding to inset interactive elements to be within a safe,
    /// un-obscured space.
    ///
    /// This value reflects the `MediaQueryData.viewPadding` of the [`Scaffold`]'s
    /// [`BuildContext`] when [`Scaffold::resize_to_avoid_bottom_inset`] is false or and
    /// the `MediaQueryData.viewInsets` > 0.0. This helps distinguish between
    /// different types of obstructions on the screen, such as software keyboards
    /// and physical device notches.
    pub min_view_padding: EdgeInsets,

    /// The [`Size`] of the whole [`Scaffold`].
    ///
    /// If the [`Size`] of the [`Scaffold`]'s contents is modified by values such as
    /// [`Scaffold::resize_to_avoid_bottom_inset`] or the keyboard opening, then the
    /// [`scaffold_size`](Self::scaffold_size) will not reflect those changes.
    ///
    /// This means that [`FloatingActionButtonLocation`]s designed to reposition
    /// the `FloatingActionButton` based on events such as the keyboard popping
    /// up should use [`min_insets`](Self::min_insets) to make sure that the
    /// `FloatingActionButton` is inset by enough to remain visible.
    ///
    /// See [`min_insets`](Self::min_insets) and `MediaQueryData.padding` for more information
    /// on the appropriate insets to apply.
    pub scaffold_size: Size,

    /// The [`Size`] of the [`Scaffold`]'s [`SnackBar`].
    ///
    /// If the [`Scaffold`] is not showing a [`SnackBar`], this will be [`Size::ZERO`].
    pub snack_bar_size: Size,

    /// The [`Size`] of the [`Scaffold`]'s [`MaterialBanner`].
    ///
    /// If the [`Scaffold`] is not showing a [`MaterialBanner`], this will be [`Size::ZERO`].
    pub material_banner_size: Size,

    /// The [`TextDirection`] of the [`Scaffold`]'s [`BuildContext`].
    pub text_direction: TextDirection,
}

impl ScaffoldPrelayoutGeometry {
    /// Abstract const constructor. This constructor enables subclasses to provide
    /// const constructors so that they can be used in const expressions.
    #[expect(
        clippy::too_many_arguments,
        reason = "Dart's constructor takes every field as a required argument"
    )]
    pub const fn new(
        bottom_sheet_size: Size,
        content_bottom: f64,
        content_top: f64,
        floating_action_button_size: Size,
        min_insets: EdgeInsets,
        min_view_padding: EdgeInsets,
        scaffold_size: Size,
        snack_bar_size: Size,
        material_banner_size: Size,
        text_direction: TextDirection,
    ) -> ScaffoldPrelayoutGeometry {
        ScaffoldPrelayoutGeometry {
            floating_action_button_size,
            bottom_sheet_size,
            content_bottom,
            content_top,
            min_insets,
            min_view_padding,
            scaffold_size,
            snack_bar_size,
            material_banner_size,
            text_direction,
        }
    }
}

/// A snapshot of a transition between two [`FloatingActionButtonLocation`]s.
///
/// [`ScaffoldState`] uses this to seamlessly change transition animations
/// when a running [`FloatingActionButtonLocation`] transition is interrupted by a new
/// transition.
struct TransitionSnapshotFabLocation {
    begin: FloatingActionButtonLocationRef,
    end: FloatingActionButtonLocationRef,
    animator: FloatingActionButtonAnimatorRef,
    progress: f64,
}

impl FloatingActionButtonLocation for TransitionSnapshotFabLocation {
    fn get_offset(&self, scaffold_geometry: &ScaffoldPrelayoutGeometry) -> Offset {
        self.animator.get_offset(
            self.begin.get_offset(scaffold_geometry),
            self.end.get_offset(scaffold_geometry),
            self.progress,
        )
    }
}

impl Debug for TransitionSnapshotFabLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "_TransitionSnapshotFabLocation(begin: {:?}, end: {:?}, progress: {})",
            self.begin, self.end, self.progress
        )
    }
}

/// Geometry information for [`Scaffold`] components after layout is finished.
///
/// To get a notifier for the scaffold geometry of a given
/// [`BuildContext`], use [`Scaffold::geometry_of`].
///
/// The ScaffoldGeometry is only available during the paint phase, because
/// its value is computed during the animation and layout phases prior to painting.
///
/// For an example of using the [`ScaffoldGeometry`], see the `BottomAppBar`,
/// which uses the [`ScaffoldGeometry`] to paint a notch around the
/// `FloatingActionButton`.
///
/// For information about the [`Scaffold`]'s geometry that is used while laying
/// out the `FloatingActionButton`, see [`ScaffoldPrelayoutGeometry`].
#[derive(Clone, Copy, Debug, Default)]
pub struct ScaffoldGeometry {
    /// The distance from the [`Scaffold`]'s top edge to the top edge of the
    /// rectangle in which the [`Scaffold::bottom_navigation_bar`] bar is laid out.
    ///
    /// Null if [`Scaffold::bottom_navigation_bar`] is null.
    pub bottom_navigation_bar_top: Option<f64>,

    /// The [`Scaffold::floating_action_button`]'s bounding rectangle.
    ///
    /// This is null when there is no floating action button showing.
    pub floating_action_button_area: Option<Rect>,
}

impl ScaffoldGeometry {
    /// Create an object that describes the geometry of a [`Scaffold`].
    pub const fn new() -> ScaffoldGeometry {
        ScaffoldGeometry {
            bottom_navigation_bar_top: None,
            floating_action_button_area: None,
        }
    }

    /// Dart `ScaffoldGeometry(bottomNavigationBarTop:)`.
    pub fn bottom_navigation_bar_top(mut self, bottom_navigation_bar_top: f64) -> ScaffoldGeometry {
        self.bottom_navigation_bar_top = Some(bottom_navigation_bar_top);
        self
    }

    /// Dart `ScaffoldGeometry(floatingActionButtonArea:)`.
    pub fn floating_action_button_area(
        mut self,
        floating_action_button_area: Rect,
    ) -> ScaffoldGeometry {
        self.floating_action_button_area = Some(floating_action_button_area);
        self
    }

    fn scale_floating_action_button(self, scale_factor: f64) -> ScaffoldGeometry {
        if scale_factor == 1.0 {
            return self;
        }

        if scale_factor == 0.0 {
            return ScaffoldGeometry {
                bottom_navigation_bar_top: self.bottom_navigation_bar_top,
                floating_action_button_area: None,
            };
        }

        let area = self
            .floating_action_button_area
            .expect("a scaled floating action button has an area");
        let scaled_button = Rect::lerp(Some(area.center() & Size::ZERO), Some(area), scale_factor)
            .expect("both rects are given");
        self.copy_with().floating_action_button_area(scaled_button)
    }

    /// Creates a copy of this [`ScaffoldGeometry`] but with the given fields replaced with
    /// the new values.
    pub fn copy_with(&self) -> ScaffoldGeometry {
        *self
    }
}

/// Dart's `_ScaffoldGeometryNotifier`: the [`ScaffoldGeometry`] a scaffold publishes to
/// [`Scaffold::geometry_of`] as its layout and its floating action button's scale change.
pub struct ScaffoldGeometryNotifier {
    change_notifier: ChangeNotifierData,
    context: BuildContext,
    floating_action_button_scale: Option<f64>,
    geometry: ScaffoldGeometry,
}

impl ScaffoldGeometryNotifier {
    fn new(app: &mut App, geometry: ScaffoldGeometry, context: BuildContext) -> Handle<Self> {
        app.create(ScaffoldGeometryNotifier {
            change_notifier: ChangeNotifierData::default(),
            context,
            floating_action_button_scale: None,
            geometry,
        })
    }

    /// The scaffold's geometry, with the floating action button's area scaled by its current
    /// scale. Dart's `value` getter.
    ///
    /// # Panics
    ///
    /// In a debug build, outside the paint phase.
    pub fn value(self: Handle<Self>, app: &App) -> ScaffoldGeometry {
        if cfg!(debug_assertions) {
            let render_object = app.get(self).context.find_render_object(app);
            let doing_paint = render_object
                .and_then(|render_object| render_object.owner(app))
                .is_some_and(|owner| owner.debug_doing_paint(app));
            assert!(
                doing_paint,
                "Scaffold.geometryOf() must only be accessed during the paint phase.\n\
                 The ScaffoldGeometry is only available during the paint phase, because \
                 its value is computed during the animation and layout phases prior to painting."
            );
        }

        let notifier = app.get(self);
        notifier.geometry.scale_floating_action_button(
            notifier
                .floating_action_button_scale
                .expect("the floating action button transition sets a scale"),
        )
    }

    fn update_with(
        self: Handle<Self>,
        app: &mut App,
        bottom_navigation_bar_top: Option<f64>,
        floating_action_button_area: Option<Rect>,
        floating_action_button_scale: Option<f64>,
    ) {
        let notifier = app.get_mut(self);
        notifier.floating_action_button_scale =
            floating_action_button_scale.or(notifier.floating_action_button_scale);
        notifier.geometry = ScaffoldGeometry {
            bottom_navigation_bar_top: bottom_navigation_bar_top
                .or(notifier.geometry.bottom_navigation_bar_top),
            floating_action_button_area: floating_action_button_area
                .or(notifier.geometry.floating_action_button_area),
        };
        self.notify_listeners(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).change_notifier.dispose();
    }
}

impl ChangeNotifier for ScaffoldGeometryNotifier {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl Debug for ScaffoldGeometryNotifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_ScaffoldGeometryNotifier")
            .field(
                "floatingActionButtonScale",
                &self.floating_action_button_scale,
            )
            .field("geometry", &self.geometry)
            .finish_non_exhaustive()
    }
}

// Used to communicate the height of the Scaffold's bottomNavigationBar and
// persistentFooterButtons to the LayoutBuilder which builds the Scaffold's body.
//
// Scaffold expects a _BodyBoxConstraints to be passed to the _BodyBuilder
// widget's LayoutBuilder, see _ScaffoldLayout.performLayout(). The BoxConstraints
// methods that construct new BoxConstraints objects, like copyWith() have not
// been overridden here because we expect the _BodyBoxConstraintsObject to be
// passed along unmodified to the LayoutBuilder. If that changes in the future
// then _BodyBuilder will assert.
#[derive(Debug)]
struct BodyBoxConstraints {
    bottom_widgets_height: f64,
    app_bar_height: f64,
    material_banner_height: f64,
}

impl BodyBoxConstraints {
    /// Dart's `_BodyBoxConstraints(...)`: the instance, whose plain value is
    /// `super(maxWidth:, maxHeight:)`.
    #[expect(
        clippy::new_ret_no_self,
        reason = "a BoxConstraints subclass instance is an AnyBoxConstraints"
    )]
    fn new(
        max_width: f64,
        max_height: f64,
        bottom_widgets_height: f64,
        app_bar_height: f64,
        material_banner_height: f64,
    ) -> AnyBoxConstraints {
        debug_assert!(bottom_widgets_height >= 0.0);
        debug_assert!(app_bar_height >= 0.0);
        debug_assert!(material_banner_height >= 0.0);
        AnyBoxConstraints::with_shared(
            BoxConstraints::new()
                .max_width(max_width)
                .max_height(max_height),
            Rc::new(BodyBoxConstraints {
                bottom_widgets_height,
                app_bar_height,
                material_banner_height,
            }),
        )
    }
}

impl BoxConstraintsExtension for BodyBoxConstraints {
    fn as_any(&self) -> &dyn Any {
        self
    }

    // RenderObject.layout() will only short-circuit its call to its performLayout
    // method if the new layout constraints are not == to the current constraints.
    // If the height of the bottom widgets has changed, even though the constraints'
    // min and max values have not, we still want performLayout to happen.
    fn eq_extension(&self, other: &dyn BoxConstraintsExtension) -> bool {
        other
            .as_any()
            .downcast_ref::<BodyBoxConstraints>()
            .is_some_and(|other| {
                other.material_banner_height == self.material_banner_height
                    && other.bottom_widgets_height == self.bottom_widgets_height
                    && other.app_bar_height == self.app_bar_height
            })
    }
}

// Used to communicate the keyboard inset computed by _ScaffoldLayout to the
// persistent bottom sheet, in the same way _BodyBoxConstraints communicates
// layout information to the body.
#[derive(Debug)]
struct BottomSheetBoxConstraints {
    // The distance between the bottom of the sheet's content, which avoids the
    // keyboard, and the bottom edge of the sheet's Material, which extends
    // behind it. Zero when the keyboard is closed or when
    // Scaffold.resizeToAvoidBottomInset is false.
    keyboard_inset: f64,
}

impl BottomSheetBoxConstraints {
    /// Dart's `_BottomSheetBoxConstraints(...)`: the instance, whose plain value is
    /// `super(maxWidth:, maxHeight:)`.
    #[expect(
        clippy::new_ret_no_self,
        reason = "a BoxConstraints subclass instance is an AnyBoxConstraints"
    )]
    fn new(max_width: f64, max_height: f64, keyboard_inset: f64) -> AnyBoxConstraints {
        debug_assert!(keyboard_inset >= 0.0);
        AnyBoxConstraints::with_shared(
            BoxConstraints::new()
                .max_width(max_width)
                .max_height(max_height),
            Rc::new(BottomSheetBoxConstraints { keyboard_inset }),
        )
    }
}

impl BoxConstraintsExtension for BottomSheetBoxConstraints {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn eq_extension(&self, other: &dyn BoxConstraintsExtension) -> bool {
        other
            .as_any()
            .downcast_ref::<BottomSheetBoxConstraints>()
            .is_some_and(|other| other.keyboard_inset == self.keyboard_inset)
    }
}

// Provides _BottomSheetBoxConstraints.keyboardInset to the _StandardBottomSheets
// in the Scaffold's bottom sheet slot, which are built before layout.
#[derive(Debug)]
struct BottomSheetKeyboardInset {
    inset: f64,
    child: WidgetRef,
}

impl BottomSheetKeyboardInset {
    fn of(app: &mut App, context: BuildContext) -> f64 {
        context
            .depend_on_inherited_widget_of_exact_type::<BottomSheetKeyboardInset>(app)
            .map_or(0.0, |scope| scope.inset)
    }
}

impl InheritedWidget for BottomSheetKeyboardInset {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &BottomSheetKeyboardInset) -> bool {
        self.inset != old_widget.inset
    }
}

// Used when Scaffold.extendBody is true to wrap the scaffold's body in a MediaQuery
// whose padding accounts for the height of the bottomNavigationBar and/or the
// persistentFooterButtons.
//
// The bottom widgets' height is passed along via the _BodyBoxConstraints parameter.
// The constraints parameter is constructed in_ScaffoldLayout.performLayout().
#[derive(Debug)]
struct BodyBuilder {
    body: WidgetRef,
    extend_body: bool,
    extend_body_behind_app_bar: bool,
}

impl BodyBuilder {
    fn new(extend_body: bool, extend_body_behind_app_bar: bool, body: WidgetRef) -> BodyBuilder {
        BodyBuilder {
            body,
            extend_body,
            extend_body_behind_app_bar,
        }
    }
}

impl StatelessWidget for BodyBuilder {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        if !self.extend_body && !self.extend_body_behind_app_bar {
            return self.body.clone();
        }

        let extend_body = self.extend_body;
        let extend_body_behind_app_bar = self.extend_body_behind_app_bar;
        let body = self.body.clone();
        LayoutBuilder::new(move |app, context, constraints| {
            let body_constraints = constraints
                .extension::<BodyBoxConstraints>()
                .expect("the scaffold lays out its body with _BodyBoxConstraints");
            let metrics = MediaQuery::of(app, context);

            let bottom = if extend_body {
                f64::max(
                    metrics.padding.bottom,
                    body_constraints.bottom_widgets_height,
                )
            } else {
                metrics.padding.bottom
            };

            let top = if extend_body_behind_app_bar {
                f64::max(
                    metrics.padding.top,
                    body_constraints.app_bar_height + body_constraints.material_banner_height,
                )
            } else {
                metrics.padding.top
            };

            let padding = metrics
                .padding
                .copy_with(None, Some(top), None, Some(bottom));
            MediaQuery::new(metrics.copy_with().padding(padding), body.clone()).into_widget()
        })
        .into_widget()
    }
}

/// Dart's `_ScaffoldLayout`.
struct ScaffoldLayout {
    extend_body: bool,
    extend_body_behind_app_bar: bool,
    min_insets: EdgeInsets,
    min_view_padding: EdgeInsets,
    text_direction: TextDirection,
    geometry_notifier: Handle<ScaffoldGeometryNotifier>,

    previous_floating_action_button_location: FloatingActionButtonLocationRef,
    current_floating_action_button_location: FloatingActionButtonLocationRef,
    floating_action_button_move_animation: AnyAnimation<f64>,
    /// Dart's `super(relayout: floatingActionButtonMoveAnimation)`.
    relayout: Rc<dyn Listenable>,
    floating_action_button_motion_animator: FloatingActionButtonAnimatorRef,

    is_snack_bar_floating: bool,
    snack_bar_width: Option<f64>,

    extend_body_behind_material_banner: bool,
}

impl ScaffoldLayout {
    /// Whether the snack bar sits above a floating action button at `location`; Dart's
    /// switch over the standard locations, which compares them by identity.
    fn shows_snack_bar_above_fab(location: &FloatingActionButtonLocationRef) -> bool {
        let top_locations = [
            <dyn FloatingActionButtonLocation>::start_top(),
            <dyn FloatingActionButtonLocation>::center_top(),
            <dyn FloatingActionButtonLocation>::end_top(),
            <dyn FloatingActionButtonLocation>::mini_start_top(),
            <dyn FloatingActionButtonLocation>::mini_center_top(),
            <dyn FloatingActionButtonLocation>::mini_end_top(),
        ];
        !top_locations
            .iter()
            .any(|top_location| Rc::ptr_eq(top_location, location))
    }
}

impl MultiChildLayoutDelegate for ScaffoldLayout {
    fn relayout(&self) -> Option<&Rc<dyn Listenable>> {
        Some(&self.relayout)
    }

    fn perform_layout(&self, app: &mut App, children: &mut MultiChildLayoutChildren, size: Size) {
        let loose_constraints = BoxConstraints::loose(size);

        // This part of the layout has the same effect as putting the app bar and
        // body in a column and making the body flexible. What's different is that
        // in this case the app bar appears _after_ the body in the stacking order,
        // so the app bar's shadow is drawn on top of the body.

        let full_width_constraints = loose_constraints.tighten(Some(size.width()), None);
        let bottom = size.height();
        let mut content_top = 0.0;
        let mut bottom_widgets_height = 0.0;
        let mut app_bar_height = 0.0;

        if children.has_child(&ScaffoldSlot::AppBar.id()) {
            app_bar_height = children
                .layout_child(app, &ScaffoldSlot::AppBar.id(), full_width_constraints)
                .height();
            content_top = if self.extend_body_behind_app_bar {
                0.0
            } else {
                app_bar_height
            };
            children.position_child(app, &ScaffoldSlot::AppBar.id(), Offset::ZERO);
        }

        let mut bottom_navigation_bar_top = None;
        if children.has_child(&ScaffoldSlot::BottomNavigationBar.id()) {
            let bottom_navigation_bar_height = children
                .layout_child(
                    app,
                    &ScaffoldSlot::BottomNavigationBar.id(),
                    full_width_constraints,
                )
                .height();
            bottom_widgets_height += bottom_navigation_bar_height;
            let top = f64::max(0.0, bottom - bottom_widgets_height);
            bottom_navigation_bar_top = Some(top);
            children.position_child(
                app,
                &ScaffoldSlot::BottomNavigationBar.id(),
                Offset::new(0.0, top),
            );
        }

        if children.has_child(&ScaffoldSlot::PersistentFooter.id()) {
            let footer_constraints = BoxConstraints::new()
                .max_width(full_width_constraints.max_width)
                .max_height(f64::max(0.0, bottom - bottom_widgets_height - content_top));
            let persistent_footer_height = children
                .layout_child(
                    app,
                    &ScaffoldSlot::PersistentFooter.id(),
                    footer_constraints,
                )
                .height();
            bottom_widgets_height += persistent_footer_height;
            children.position_child(
                app,
                &ScaffoldSlot::PersistentFooter.id(),
                Offset::new(0.0, f64::max(0.0, bottom - bottom_widgets_height)),
            );
        }

        let mut material_banner_size = Size::ZERO;
        if children.has_child(&ScaffoldSlot::MaterialBanner.id()) {
            material_banner_size = children.layout_child(
                app,
                &ScaffoldSlot::MaterialBanner.id(),
                full_width_constraints,
            );
            children.position_child(
                app,
                &ScaffoldSlot::MaterialBanner.id(),
                Offset::new(0.0, app_bar_height),
            );

            // Push content down only if elevation is 0.
            if !self.extend_body_behind_material_banner {
                content_top += material_banner_size.height();
            }
        }

        // Set the content bottom to account for the greater of the height of any
        // bottom-anchored material widgets or of the keyboard or other
        // bottom-anchored system UI.
        let content_bottom = f64::max(
            0.0,
            bottom - f64::max(self.min_insets.bottom, bottom_widgets_height),
        );

        // When the keyboard lifts the bottom sheet's content above it, the sheet's
        // Material keeps extending down to the edge otherwise available to the
        // sheet (above any bottom bars), so that a translucent keyboard shows the
        // sheet's color rather than the Scaffold's background.
        let bottom_sheet_keyboard_inset =
            f64::max(0.0, bottom - bottom_widgets_height - content_bottom);

        if children.has_child(&ScaffoldSlot::Body.id()) {
            let mut body_max_height = f64::max(0.0, content_bottom - content_top);

            // When extendBody is true, the body is visible underneath the bottom widgets.
            // This does not apply when the area is obscured by the device keyboard.
            if self.extend_body && self.min_insets.bottom <= bottom_widgets_height {
                body_max_height += bottom_widgets_height;
                body_max_height = clamp_double(
                    body_max_height,
                    0.0,
                    loose_constraints.max_height - content_top,
                );
                debug_assert!(
                    body_max_height <= f64::max(0.0, loose_constraints.max_height - content_top)
                );
            } else {
                bottom_widgets_height = 0.0;
            }

            let body_constraints = BodyBoxConstraints::new(
                full_width_constraints.max_width,
                body_max_height,
                bottom_widgets_height,
                app_bar_height,
                material_banner_size.height(),
            );
            children.layout_child(app, &ScaffoldSlot::Body.id(), body_constraints);
            children.position_child(app, &ScaffoldSlot::Body.id(), Offset::new(0.0, content_top));
        }

        // The BottomSheet and the SnackBar are anchored to the bottom of the parent,
        // they're as wide as the parent and are given their intrinsic height. The
        // only difference is that SnackBar appears on the top side of the
        // BottomNavigationBar while the BottomSheet is stacked on top of it.
        //
        // If all three elements are present then either the center of the FAB straddles
        // the top edge of the BottomSheet or the bottom of the FAB is
        // kFloatingActionButtonMargin above the SnackBar, whichever puts the FAB
        // the farthest above the bottom of the parent. If only the FAB is has a
        // non-zero height then it's inset from the parent's right and bottom edges
        // by kFloatingActionButtonMargin.

        let mut bottom_sheet_size = Size::ZERO;
        let mut snack_bar_size = Size::ZERO;
        if children.has_child(&ScaffoldSlot::BodyScrim.id()) {
            let bottom_sheet_scrim_constraints = BoxConstraints::new()
                .max_width(full_width_constraints.max_width)
                .max_height(content_bottom);
            children.layout_child(
                app,
                &ScaffoldSlot::BodyScrim.id(),
                bottom_sheet_scrim_constraints,
            );
            children.position_child(app, &ScaffoldSlot::BodyScrim.id(), Offset::ZERO);
        }

        // Set the size of the SnackBar early if the behavior is fixed so
        // the FAB can be positioned correctly.
        if children.has_child(&ScaffoldSlot::SnackBar.id()) && !self.is_snack_bar_floating {
            snack_bar_size =
                children.layout_child(app, &ScaffoldSlot::SnackBar.id(), full_width_constraints);
        }

        if children.has_child(&ScaffoldSlot::BottomSheet.id()) {
            let bottom_sheet_bottom = content_bottom + bottom_sheet_keyboard_inset;
            let bottom_sheet_constraints = BottomSheetBoxConstraints::new(
                full_width_constraints.max_width,
                f64::max(0.0, bottom_sheet_bottom - content_top),
                bottom_sheet_keyboard_inset,
            );
            let sheet_size = children.layout_child(
                app,
                &ScaffoldSlot::BottomSheet.id(),
                bottom_sheet_constraints,
            );
            children.position_child(
                app,
                &ScaffoldSlot::BottomSheet.id(),
                Offset::new(
                    (size.width() - sheet_size.width()) / 2.0,
                    bottom_sheet_bottom - sheet_size.height(),
                ),
            );
            // The floating action button is positioned relative to the sheet's
            // content, which ends at contentBottom, not to the Material behind the
            // keyboard.
            bottom_sheet_size = Size::new(
                sheet_size.width(),
                f64::max(0.0, sheet_size.height() - bottom_sheet_keyboard_inset),
            );
        }

        let mut floating_action_button_rect = None;
        if children.has_child(&ScaffoldSlot::FloatingActionButton.id()) {
            let fab_size = children.layout_child(
                app,
                &ScaffoldSlot::FloatingActionButton.id(),
                loose_constraints,
            );

            // To account for the FAB position being changed, we'll animate between
            // the old and new positions.
            let current_geometry = ScaffoldPrelayoutGeometry {
                bottom_sheet_size,
                content_bottom,
                // [appBarHeight] should be used instead of [contentTop] because
                // ScaffoldPrelayoutGeometry.contentTop must not be affected by
                // [extendBodyBehindAppBar].
                content_top: app_bar_height,
                floating_action_button_size: fab_size,
                min_insets: self.min_insets,
                scaffold_size: size,
                snack_bar_size,
                material_banner_size,
                text_direction: self.text_direction,
                min_view_padding: self.min_view_padding,
            };
            let current_fab_offset = self
                .current_floating_action_button_location
                .get_offset(&current_geometry);
            let previous_fab_offset = self
                .previous_floating_action_button_location
                .get_offset(&current_geometry);
            let fab_offset = self.floating_action_button_motion_animator.get_offset(
                previous_fab_offset,
                current_fab_offset,
                self.floating_action_button_move_animation.value(app),
            );
            children.position_child(app, &ScaffoldSlot::FloatingActionButton.id(), fab_offset);
            floating_action_button_rect = Some(fab_offset & fab_size);
        }

        if children.has_child(&ScaffoldSlot::SnackBar.id()) {
            let has_custom_width = self
                .snack_bar_width
                .is_some_and(|snack_bar_width| snack_bar_width < size.width());
            if snack_bar_size == Size::ZERO {
                snack_bar_size = children.layout_child(
                    app,
                    &ScaffoldSlot::SnackBar.id(),
                    if has_custom_width {
                        loose_constraints
                    } else {
                        full_width_constraints
                    },
                );
            }

            let show_above_fab = ScaffoldLayout::shows_snack_bar_above_fab(
                &self.current_floating_action_button_location,
            );
            let floating_action_button_rect = floating_action_button_rect
                .expect("a scaffold always has a floating action button slot");
            let snack_bar_y_offset_base = if floating_action_button_rect.size() != Size::ZERO
                && self.is_snack_bar_floating
                && show_above_fab
            {
                match bottom_navigation_bar_top {
                    Some(bottom_navigation_bar_top) => {
                        f64::min(bottom_navigation_bar_top, floating_action_button_rect.top)
                    }
                    None => floating_action_button_rect.top,
                }
            } else {
                // SnackBarBehavior.fixed applies a SafeArea automatically.
                // SnackBarBehavior.floating does not since the positioning is affected
                // if there is a FloatingActionButton (see condition above). If there is
                // no FAB, make sure we account for safe space when the SnackBar is
                // floating.
                let safe_y_offset_base = size.height() - self.min_view_padding.bottom;
                if self.is_snack_bar_floating {
                    f64::min(content_bottom, safe_y_offset_base)
                } else {
                    content_bottom
                }
            };

            let x_offset = if has_custom_width {
                (size.width() - self.snack_bar_width.expect("has a custom width")) / 2.0
            } else {
                0.0
            };
            children.position_child(
                app,
                &ScaffoldSlot::SnackBar.id(),
                Offset::new(x_offset, snack_bar_y_offset_base - snack_bar_size.height()),
            );

            // Whether a floating SnackBar has been offset too high.
            //
            // To improve the developer experience, this assert is done after the call to
            // positionChild. if we assert sooner the SnackBar is visible because its defaults
            // position is (0,0) and it can cause confusion to the user as the error message
            // states that the SnackBar is off screen.
            if cfg!(debug_assertions) && self.is_snack_bar_floating {
                let snack_bar_visible = (snack_bar_y_offset_base - snack_bar_size.height()) >= 0.0;
                assert!(
                    snack_bar_visible,
                    "Floating SnackBar presented off screen.\n\
                     A SnackBar with behavior property set to SnackBarBehavior.floating is fully \
                     or partially off screen because some or all the widgets provided to \
                     Scaffold.floatingActionButton, Scaffold.persistentFooterButtons and \
                     Scaffold.bottomNavigationBar take up too much vertical space.\n\
                     Consider constraining the size of these widgets to allow room for the \
                     SnackBar to be visible."
                );
            }
        }

        if children.has_child(&ScaffoldSlot::StatusBar.id()) {
            children.layout_child(
                app,
                &ScaffoldSlot::StatusBar.id(),
                full_width_constraints.tighten(None, Some(self.min_insets.top)),
            );
            children.position_child(app, &ScaffoldSlot::StatusBar.id(), Offset::ZERO);
        }

        if children.has_child(&ScaffoldSlot::Drawer.id()) {
            children.layout_child(app, &ScaffoldSlot::Drawer.id(), BoxConstraints::tight(size));
            children.position_child(app, &ScaffoldSlot::Drawer.id(), Offset::ZERO);
        }

        if children.has_child(&ScaffoldSlot::EndDrawer.id()) {
            children.layout_child(
                app,
                &ScaffoldSlot::EndDrawer.id(),
                BoxConstraints::tight(size),
            );
            children.position_child(app, &ScaffoldSlot::EndDrawer.id(), Offset::ZERO);
        }

        self.geometry_notifier.update_with(
            app,
            bottom_navigation_bar_top,
            floating_action_button_rect,
            None,
        );
    }

    fn should_relayout(&self, old_delegate: &dyn MultiChildLayoutDelegate) -> bool {
        let old_delegate = old_delegate
            .as_any()
            .downcast_ref::<ScaffoldLayout>()
            .expect("the scaffold always lays out with a ScaffoldLayout");
        old_delegate.min_insets != self.min_insets
            || old_delegate.min_view_padding != self.min_view_padding
            || old_delegate.text_direction != self.text_direction
            || !Rc::ptr_eq(
                &old_delegate.previous_floating_action_button_location,
                &self.previous_floating_action_button_location,
            )
            || !Rc::ptr_eq(
                &old_delegate.current_floating_action_button_location,
                &self.current_floating_action_button_location,
            )
            || old_delegate.extend_body != self.extend_body
            || old_delegate.extend_body_behind_app_bar != self.extend_body_behind_app_bar
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Debug for ScaffoldLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_ScaffoldLayout")
            .field("minInsets", &self.min_insets)
            .field("minViewPadding", &self.min_view_padding)
            .field("textDirection", &self.text_direction)
            .field(
                "currentFloatingActionButtonLocation",
                &self.current_floating_action_button_location,
            )
            .finish_non_exhaustive()
    }
}

/// Handler for scale and rotation animations in the `FloatingActionButton`.
///
/// Currently, there are two types of `FloatingActionButton` animations:
///
/// * Entrance/Exit animations, which this widget triggers
///   when the `FloatingActionButton` is added, updated, or removed.
/// * Motion animations, which are triggered by the [`Scaffold`]
///   when its [`FloatingActionButtonLocation`] is updated.
struct FloatingActionButtonTransition {
    child: Option<WidgetRef>,
    fab_move_animation: AnyAnimation<f64>,
    fab_motion_animator: FloatingActionButtonAnimatorRef,
    geometry_notifier: Handle<ScaffoldGeometryNotifier>,

    /// Controls the current child widget.child as it exits.
    current_controller: Handle<AnimationController>,
}

impl Debug for FloatingActionButtonTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_FloatingActionButtonTransition")
            .field("child", &self.child)
            .field("fabMotionAnimator", &self.fab_motion_animator)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for FloatingActionButtonTransition {
    type State = FloatingActionButtonTransitionState;

    fn create_state(&self) -> FloatingActionButtonTransitionState {
        FloatingActionButtonTransitionState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            previous_controller: None,
            previous_exit_scale_animation: None,
            previous_exit_rotation_curved_animation: None,
            current_entrance_scale_animation: None,
            previous_scale_animation: None,
            previous_rotation_animation: None,
            current_scale_animation: None,
            extended_current_scale_animation: None,
            current_rotation_animation: None,
            previous_child: None,
            made_animations: Vec::new(),
        }
    }
}

/// Dart's `static final _FloatingActionButtonTransitionState._entranceTurnTween`, kept per
/// [`App`] and made on first use; shared by every transition, so never destroyed.
#[derive(Default)]
struct EntranceTurnTween {
    tweens: Option<(Handle<Tween<f64>>, Handle<CurveTween>)>,
}

/// Dart's `_FloatingActionButtonTransitionState`.
struct FloatingActionButtonTransitionState {
    state: StateData<FloatingActionButtonTransition>,
    ticker_provider: TickerProviderStateMixinData,
    // The animations applied to the Floating Action Button when it is entering or exiting.
    // Controls the previous widget.child as it exits.
    previous_controller: Option<Handle<AnimationController>>,
    previous_exit_scale_animation: Option<Handle<CurvedAnimation>>,
    previous_exit_rotation_curved_animation: Option<Handle<CurvedAnimation>>,
    current_entrance_scale_animation: Option<Handle<CurvedAnimation>>,
    previous_scale_animation: Option<AnyAnimation<f64>>,
    previous_rotation_animation: Option<Handle<TrainHoppingAnimation>>,
    // The animations to run, considering the widget's fabMoveAnimation and the
    // current/previous entrance/exit animations.
    current_scale_animation: Option<AnyAnimation<f64>>,
    extended_current_scale_animation: Option<AnyAnimation<f64>>,
    current_rotation_animation: Option<Handle<TrainHoppingAnimation>>,
    previous_child: Option<WidgetRef>,
    /// The tweens, driven animations and `AnimationMin`s the last `update_animations` made that
    /// no field above owns, which Dart's collector frees once the animations using them are
    /// replaced; `free_made_animations` frees them then.
    made_animations: Vec<HandleId>,
}

impl FloatingActionButtonTransitionState {
    fn previous_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .previous_controller
            .expect("set in init_state")
    }

    /// Dart's `static final _entranceTurnTween`.
    fn entrance_turn_tween(app: &mut App) -> impl Animatable<f64> + Clone + 'static {
        let statics = app.singleton::<EntranceTurnTween>();
        let (tween, curve_tween) = match app.get(statics).tweens {
            Some(tweens) => tweens,
            None => {
                let tween = Tween::new(
                    app,
                    Some(1.0 - K_FLOATING_ACTION_BUTTON_TURN_INTERVAL),
                    Some(1.0),
                );
                let curve_tween = CurveTween::new(app, Curves::ease_in());
                app.get_mut(statics).tweens = Some((tween, curve_tween));
                (tween, curve_tween)
            }
        };
        tween.chain(curve_tween)
    }

    fn dispose_animations(self: Handle<Self>, app: &mut App) {
        if let Some(animation) = app.get(self).previous_rotation_animation {
            animation.dispose(app);
            destroy_lingering(app, animation);
        }
        if let Some(animation) = app.get(self).current_rotation_animation {
            animation.dispose(app);
            destroy_lingering(app, animation);
        }
        self.free_made_animations(app);
    }

    /// Frees what the last `update_animations` made, once its two scale animations no longer
    /// call `on_progress_changed`; the transitions that still listen to them let go when they
    /// rebuild, before the entries go at the next checkpoint.
    fn free_made_animations(self: Handle<Self>, app: &mut App) {
        let this = app.get(self);
        for animation in [this.previous_scale_animation, this.current_scale_animation]
            .into_iter()
            .flatten()
        {
            animation.remove_listener(
                app,
                &Listener::handle_method(
                    self,
                    FloatingActionButtonTransitionState::on_progress_changed,
                ),
            );
        }
        for made in std::mem::take(&mut app.get_mut(self).made_animations) {
            destroy_lingering(app, made);
        }
    }

    /// Dart's `curvedAnimation?.dispose()`, then frees it.
    fn dispose_curved_animation(app: &mut App, animation: Option<Handle<CurvedAnimation>>) {
        if let Some(animation) = animation {
            animation.dispose(app);
            destroy_lingering(app, animation);
        }
    }

    fn update_animations(self: Handle<Self>, app: &mut App) {
        let previous_controller = self.previous_controller(app);
        FloatingActionButtonTransitionState::dispose_curved_animation(
            app,
            app.get(self).previous_exit_scale_animation,
        );
        // Get the animations for exit and entrance.
        let previous_exit_scale_animation =
            CurvedAnimation::create(app, previous_controller.view(), Curves::ease_in(), None);
        app.get_mut(self).previous_exit_scale_animation = Some(previous_exit_scale_animation);
        FloatingActionButtonTransitionState::dispose_curved_animation(
            app,
            app.get(self).previous_exit_rotation_curved_animation,
        );
        let previous_exit_rotation_curved_animation =
            CurvedAnimation::create(app, previous_controller.view(), Curves::ease_in(), None);
        app.get_mut(self).previous_exit_rotation_curved_animation =
            Some(previous_exit_rotation_curved_animation);

        let previous_exit_rotation_tween = Tween::new(app, Some(1.0), Some(1.0));
        let previous_exit_rotation_animation = previous_exit_rotation_tween
            .animate(app, previous_exit_rotation_curved_animation.as_animation());

        FloatingActionButtonTransitionState::dispose_curved_animation(
            app,
            app.get(self).current_entrance_scale_animation,
        );
        let current_controller = self.widget(app).current_controller;
        let current_entrance_scale_animation =
            CurvedAnimation::create(app, current_controller.view(), Curves::ease_in(), None);
        app.get_mut(self).current_entrance_scale_animation = Some(current_entrance_scale_animation);
        let entrance_turn_tween = FloatingActionButtonTransitionState::entrance_turn_tween(app);
        let current_entrance_rotation_animation =
            current_controller.drive(app, entrance_turn_tween);
        app.get_mut(self).made_animations.extend([
            previous_exit_rotation_tween.into(),
            previous_exit_rotation_animation.id(),
            current_entrance_rotation_animation.id(),
        ]);

        // Get the animations for when the FAB is moving.
        let fab_motion_animator = Rc::clone(&self.widget(app).fab_motion_animator);
        let fab_move_animation = self.widget(app).fab_move_animation;
        let move_scale_animation = fab_motion_animator.get_scale_animation(app, fab_move_animation);
        let move_rotation_animation =
            fab_motion_animator.get_rotation_animation(app, fab_move_animation);

        // Aggregate the animations.
        let (
            previous_scale_animation,
            current_scale_animation,
            previous_rotation,
            current_rotation,
        ) = if Rc::ptr_eq(
            &fab_motion_animator,
            &<dyn FloatingActionButtonAnimator>::no_animation(),
        ) {
            (
                move_scale_animation,
                move_scale_animation,
                TrainHoppingAnimation::create(app, move_rotation_animation, None, None),
                TrainHoppingAnimation::create(app, move_rotation_animation, None, None),
            )
        } else {
            let previous_scale = app
                .create(AnimationMin::new(
                    move_scale_animation,
                    previous_exit_scale_animation.as_animation(),
                ))
                .as_animation();
            let current_scale = app
                .create(AnimationMin::new(
                    move_scale_animation,
                    current_entrance_scale_animation.as_animation(),
                ))
                .as_animation();
            app.get_mut(self)
                .made_animations
                .extend([previous_scale.id(), current_scale.id()]);
            (
                previous_scale,
                current_scale,
                TrainHoppingAnimation::create(
                    app,
                    previous_exit_rotation_animation,
                    Some(move_rotation_animation),
                    None,
                ),
                TrainHoppingAnimation::create(
                    app,
                    current_entrance_rotation_animation,
                    Some(move_rotation_animation),
                    None,
                ),
            )
        };
        let this = app.get_mut(self);
        this.previous_scale_animation = Some(previous_scale_animation);
        this.current_scale_animation = Some(current_scale_animation);
        this.previous_rotation_animation = Some(previous_rotation);
        this.current_rotation_animation = Some(current_rotation);

        let interval_tween =
            CurveTween::new(app, Rc::new(Interval::new(0.0, 0.1, Curves::linear())));
        let extended_current_scale_animation = current_scale_animation.drive(app, interval_tween);
        let this = app.get_mut(self);
        this.extended_current_scale_animation = Some(extended_current_scale_animation);
        this.made_animations
            .extend([interval_tween.into(), extended_current_scale_animation.id()]);
        current_scale_animation.add_listener(
            app,
            Listener::handle_method(
                self,
                FloatingActionButtonTransitionState::on_progress_changed,
            ),
        );
        previous_scale_animation.add_listener(
            app,
            Listener::handle_method(
                self,
                FloatingActionButtonTransitionState::on_progress_changed,
            ),
        );
    }

    fn handle_previous_animation_status_changed(
        self: Handle<Self>,
        app: &mut App,
        status: AnimationStatus,
    ) {
        if self.widget(app).child.is_some() && status.is_dismissed() {
            let current_controller = self.widget(app).current_controller;
            debug_assert!(current_controller.is_dismissed(app));
            current_controller.forward(app, None);
        }
        self.set_state(app, |_| {});
    }

    fn is_extended_floating_action_button(widget: Option<&WidgetRef>) -> bool {
        widget
            .and_then(|widget| downcast_widget::<FloatingActionButton>(&**widget))
            .is_some_and(|button| button.is_extended)
    }

    fn on_progress_changed(self: Handle<Self>, app: &mut App) {
        let this = app.get(self);
        let previous = this
            .previous_scale_animation
            .expect("set by update_animations");
        let current = this
            .current_scale_animation
            .expect("set by update_animations");
        let scale = f64::max(previous.value(app), current.value(app));
        self.update_geometry_scale(app, scale);
    }

    fn update_geometry_scale(self: Handle<Self>, app: &mut App, scale: f64) {
        let geometry_notifier = self.widget(app).geometry_notifier;
        geometry_notifier.update_with(app, None, None, Some(scale));
    }
}

impl TickerProviderStateMixin for FloatingActionButtonTransitionState {
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
impl TickerProviderObject for FloatingActionButtonTransitionState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for FloatingActionButtonTransitionState {
    type Widget = FloatingActionButtonTransition;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let previous_controller = AnimationController::create(
            app,
            None,
            Some(K_FLOATING_ACTION_BUTTON_SEGUE),
            None,
            0.0,
            1.0,
            waft_animation::AnimationBehavior::Normal,
            self,
        );
        previous_controller.add_status_listener(
            app,
            AnimationStatusListener::handle_method(
                self,
                FloatingActionButtonTransitionState::handle_previous_animation_status_changed,
            ),
        );
        app.get_mut(self).previous_controller = Some(previous_controller);
        self.update_animations(app);

        if self.widget(app).child.is_some() {
            // If we start out with a child, have the child appear fully visible instead
            // of animating in.
            self.widget(app).current_controller.set_value(app, 1.0);
            // With FloatingActionButtonAnimator.noAnimation, floatingActionButtonScale is null.
            // Default to a scale of 1.0 to ensure the button remains visible.
            self.update_geometry_scale(app, 1.0);
        } else {
            // If we start without a child we update the geometry object with a
            // floating action button scale of 0, as it is not showing on the screen.
            self.update_geometry_scale(app, 0.0);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.previous_controller(app).dispose(app);
        FloatingActionButtonTransitionState::dispose_curved_animation(
            app,
            app.get(self).previous_exit_scale_animation,
        );
        FloatingActionButtonTransitionState::dispose_curved_animation(
            app,
            app.get(self).previous_exit_rotation_curved_animation,
        );
        FloatingActionButtonTransitionState::dispose_curved_animation(
            app,
            app.get(self).current_entrance_scale_animation,
        );
        self.dispose_animations(app);
        TickerProviderStateMixin::dispose(self, app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &FloatingActionButtonTransition,
    ) {
        let widget = self.widget(app);
        if !Rc::ptr_eq(&old_widget.fab_motion_animator, &widget.fab_motion_animator)
            || old_widget.fab_move_animation != widget.fab_move_animation
        {
            self.dispose_animations(app);
            // Get the right scale and rotation animations to use for this widget.
            self.update_animations(app);
        }
        let widget = self.widget(app);
        let old_child_is_null = old_widget.child.is_none();
        let new_child_is_null = widget.child.is_none();
        if old_child_is_null == new_child_is_null
            && old_widget.child.as_ref().and_then(|child| child.key())
                == widget.child.as_ref().and_then(|child| child.key())
        {
            return;
        }
        let previous_controller = self.previous_controller(app);
        if previous_controller.is_dismissed(app) {
            let current_controller = widget.current_controller;
            let current_value = current_controller.value(app);
            if current_value == 0.0 || old_widget.child.is_none() {
                // The current child hasn't started its entrance animation yet. We can
                // just skip directly to the new child's entrance.
                app.get_mut(self).previous_child = None;
                if self.widget(app).child.is_some() {
                    current_controller.forward(app, None);
                }
            } else {
                // Otherwise, we need to copy the state from the current controller to
                // the previous controller and run an exit animation for the previous
                // widget before running the entrance animation for the new child.
                app.get_mut(self).previous_child = old_widget.child.clone();
                previous_controller.set_value(app, current_value);
                previous_controller.reverse(app, None);
                current_controller.set_value(app, 0.0);
            }
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let this = app.get(self);
        let previous_scale_animation = this.previous_scale_animation.expect("update_animations");
        let previous_rotation_animation =
            this.previous_rotation_animation.expect("update_animations");
        let current_scale_animation = this.current_scale_animation.expect("update_animations");
        let extended_current_scale_animation = this
            .extended_current_scale_animation
            .expect("update_animations");
        let current_rotation_animation =
            this.current_rotation_animation.expect("update_animations");
        let previous_child = this.previous_child.clone();
        let child = self.widget(app).child.clone();

        let mut children = Vec::new();
        if !self.previous_controller(app).is_dismissed(app) {
            if FloatingActionButtonTransitionState::is_extended_floating_action_button(
                previous_child.as_ref(),
            ) {
                let mut fade = FadeTransition::new(previous_scale_animation);
                if let Some(previous_child) = previous_child {
                    fade = fade.child(previous_child);
                }
                children.push(fade.into_widget());
            } else {
                let mut rotation =
                    RotationTransition::new(previous_rotation_animation.as_animation());
                if let Some(previous_child) = previous_child {
                    rotation = rotation.child(previous_child);
                }
                children.push(
                    ScaleTransition::new(previous_scale_animation)
                        .child(rotation)
                        .into_widget(),
                );
            }
        }
        if FloatingActionButtonTransitionState::is_extended_floating_action_button(child.as_ref()) {
            let mut fade = FadeTransition::new(current_scale_animation);
            if let Some(child) = child {
                fade = fade.child(child);
            }
            children.push(
                ScaleTransition::new(extended_current_scale_animation)
                    .child(fade)
                    .into_widget(),
            );
        } else {
            let mut rotation = RotationTransition::new(current_rotation_animation.as_animation());
            if let Some(child) = child {
                rotation = rotation.child(child);
            }
            children.push(
                ScaleTransition::new(current_scale_animation)
                    .child(rotation)
                    .into_widget(),
            );
        }
        Stack::new()
            .alignment(AlignmentGeometry::CENTER_RIGHT)
            .children(children)
            .into_widget()
    }
}

/// Dart's `Scaffold.bottomSheetScrimBuilder`: builds the scrim behind a bottom sheet from
/// the scaffold's scrim animation, or nothing.
pub type BottomSheetScrimBuilder =
    Rc<dyn Fn(&mut App, BuildContext, AnyAnimation<f64>) -> Option<WidgetRef>>;

/// Implements the basic Material Design visual layout structure.
///
/// This class provides APIs for showing drawers and bottom sheets.
///
/// To display a persistent bottom sheet, obtain the
/// [`ScaffoldState`] for the current [`BuildContext`] via [`Scaffold::of`] and use the
/// [`ScaffoldState::show_bottom_sheet`] function.
///
/// ## Scaffold layout, the keyboard, and display "notches"
///
/// The scaffold will expand to fill the available space. That usually
/// means that it will occupy its entire window or device screen. When
/// the device's keyboard appears the Scaffold's ancestor [`MediaQuery`]
/// widget's `MediaQueryData.viewInsets` changes and the Scaffold will
/// be rebuilt. By default the scaffold's [`body`](Self::body) is resized to make
/// room for the keyboard. To prevent the resize set
/// [`resize_to_avoid_bottom_inset`](Self::resize_to_avoid_bottom_inset) to false. In either
/// case the focused widget will be scrolled into view if it's within a scrollable
/// container.
///
/// The `MediaQueryData.padding` value defines areas that might
/// not be completely visible, like the display "notch" on the iPhone
/// X. The scaffold's [`body`](Self::body) is not inset by this padding value
/// although an `appBar` or [`bottom_navigation_bar`](Self::bottom_navigation_bar) will
/// typically cause the body to avoid the padding. The [`SafeArea`]
/// widget can be used within the scaffold's body to avoid areas
/// like display notches.
///
/// ## Floating action button with a draggable scrollable bottom sheet
///
/// If [`Scaffold::bottom_sheet`] is a `DraggableScrollableSheet`,
/// [`Scaffold::floating_action_button`] is set, and the bottom sheet is dragged to
/// cover greater than 70% of the Scaffold's height, two things happen in parallel:
///
///   * Scaffold starts to show scrim (see [`ScaffoldState::show_body_scrim`]), and
///   * [`Scaffold::floating_action_button`] is scaled down through an animation with a
///     `Curves.easeIn`, and disappears when the bottom sheet covers the entire Scaffold.
///
/// And as soon as the bottom sheet is dragged down to cover less than 70% of the
/// [`Scaffold`], the scrim disappears and [`Scaffold::floating_action_button`] animates back
/// to its normal size.
///
/// ## Troubleshooting
///
/// ### Nested Scaffolds
///
/// The Scaffold is designed to be a top level container for
/// a `MaterialApp`. This means that adding a Scaffold
/// to each route on a Material app will provide the app with
/// Material's basic visual layout structure.
///
/// It is typically not necessary to nest Scaffolds. For example, in a
/// tabbed UI, where the [`bottom_navigation_bar`](Self::bottom_navigation_bar) is a `TabBar`
/// and the body is a `TabBarView`, you might be tempted to make each tab bar
/// view a scaffold with a differently titled AppBar. Rather, it would be
/// better to add a listener to the `TabController` that updates the
/// AppBar.
///
/// Although there are some use cases, like a presentation app that
/// shows embedded flutter content, where nested scaffolds are
/// appropriate, it's best to avoid nesting scaffolds.
///
/// See also:
///
///  * `AppBar`, which is a horizontal bar typically shown at the top of an app
///    using the `appBar` property.
///  * `BottomAppBar`, which is a horizontal bar typically shown at the bottom
///    of an app using the [`bottom_navigation_bar`](Self::bottom_navigation_bar) property.
///  * [`FloatingActionButton`], which is a circular button typically shown in the
///    bottom right corner of the app using the
///    [`floating_action_button`](Self::floating_action_button) property.
///  * `Drawer`, which is a vertical panel that is typically displayed to the
///    left of the body (and often hidden on phones) using the [`drawer`](Self::drawer)
///    property.
///  * `BottomNavigationBar`, which is a horizontal array of buttons typically
///    shown along the bottom of the app using the
///    [`bottom_navigation_bar`](Self::bottom_navigation_bar) property.
///  * [`BottomSheet`], which is an overlay typically shown near the bottom of the
///    app. A bottom sheet can either be persistent, in which case it is shown
///    using the [`ScaffoldState::show_bottom_sheet`] method, or modal, in which case
///    it is shown using the `show_modal_bottom_sheet` function.
pub struct Scaffold {
    pub key: Option<KeyRef>,

    /// If true, and [`bottom_navigation_bar`](Self::bottom_navigation_bar) or
    /// [`persistent_footer_buttons`](Self::persistent_footer_buttons) is specified, then the
    /// [`body`](Self::body) extends to the bottom of the Scaffold, instead of only extending to
    /// the top of the [`bottom_navigation_bar`](Self::bottom_navigation_bar) or the
    /// [`persistent_footer_buttons`](Self::persistent_footer_buttons).
    ///
    /// If true, a [`MediaQuery`] widget whose bottom padding matches the height of the
    /// [`bottom_navigation_bar`](Self::bottom_navigation_bar) will be added above the
    /// scaffold's [`body`](Self::body).
    ///
    /// This property is often useful when the
    /// [`bottom_navigation_bar`](Self::bottom_navigation_bar) has a non-rectangular shape, like
    /// `CircularNotchedRectangle`, which adds a [`FloatingActionButton`] sized notch to the top
    /// edge of the bar. In this case specifying `extend_body: true` ensures that scaffold's
    /// body will be visible through the bottom navigation bar's notch.
    ///
    /// See also:
    ///
    ///  * [`extend_body_behind_app_bar`](Self::extend_body_behind_app_bar), which extends the
    ///    height of the body to the top of the scaffold.
    pub extend_body: bool,

    /// Whether the drawer's barrier can be dismissed by tapping on it. Defaults to true.
    pub drawer_barrier_dismissible: bool,

    /// If true, and an [`app_bar`](Self::app_bar) is specified, then the height of the
    /// [`body`](Self::body) is extended to include the height of the app bar and the top of
    /// the body is aligned with the top of the app bar.
    ///
    /// This is useful if the app bar's `AppBar.background_color` is not completely opaque.
    ///
    /// This property is false by default.
    ///
    /// See also:
    ///
    ///  * [`extend_body`](Self::extend_body), which extends the height of the body to the
    ///    bottom of the scaffold.
    pub extend_body_behind_app_bar: bool,

    /// An app bar to display at the top of the scaffold.
    pub app_bar: Option<PreferredSizeWidgetRef>,

    /// The primary content of the scaffold.
    ///
    /// Displayed below the `appBar`, above the bottom of the ambient
    /// [`MediaQuery`]'s `MediaQueryData.viewInsets`, and behind the
    /// [`floating_action_button`](Self::floating_action_button) and
    /// [`drawer`](Self::drawer). If [`resize_to_avoid_bottom_inset`](Self::resize_to_avoid_bottom_inset)
    /// is false then the body is not resized when the onscreen keyboard appears,
    /// i.e. it is not inset by `viewInsets.bottom`.
    ///
    /// The widget in the body of the scaffold is positioned at the top-left of
    /// the available space between the app bar and the bottom of the scaffold. To
    /// center this widget instead, consider putting it in a `Center` widget and
    /// having that be the body. To expand this widget instead, consider
    /// putting it in a [`SizedBox::expand`].
    ///
    /// If you have a column of widgets that should normally fit on the screen,
    /// but may overflow and would in such cases need to scroll, consider using a
    /// `ListView` as the body of the scaffold. This is also a good choice for
    /// the case where your body is a scrollable list.
    pub body: Option<WidgetRef>,

    /// A button displayed floating above [`body`](Self::body), in the bottom right corner.
    ///
    /// Typically a [`FloatingActionButton`].
    pub floating_action_button: Option<WidgetRef>,

    /// Responsible for determining where the
    /// [`floating_action_button`](Self::floating_action_button) should go.
    ///
    /// If null, the [`ScaffoldState`] will use the default location,
    /// `<dyn FloatingActionButtonLocation>::end_float()`.
    pub floating_action_button_location: Option<FloatingActionButtonLocationRef>,

    /// Animator to move the [`floating_action_button`](Self::floating_action_button) to a new
    /// [`floating_action_button_location`](Self::floating_action_button_location).
    ///
    /// If null, the [`ScaffoldState`] will use the default animator,
    /// `<dyn FloatingActionButtonAnimator>::scaling()`.
    pub floating_action_button_animator: Option<FloatingActionButtonAnimatorRef>,

    /// A set of buttons that are displayed at the bottom of the scaffold.
    ///
    /// Typically this is a list of `TextButton` widgets. These buttons are
    /// persistently visible, even if the [`body`](Self::body) of the scaffold scrolls.
    ///
    /// These widgets will be wrapped in an [`OverflowBar`].
    ///
    /// The [`persistent_footer_buttons`](Self::persistent_footer_buttons) are rendered above
    /// the [`bottom_navigation_bar`](Self::bottom_navigation_bar) but below the
    /// [`body`](Self::body).
    pub persistent_footer_buttons: Option<Vec<WidgetRef>>,

    /// The alignment of the [`persistent_footer_buttons`](Self::persistent_footer_buttons)
    /// inside the [`OverflowBar`].
    ///
    /// Defaults to `AlignmentDirectional::CENTER_END`.
    pub persistent_footer_alignment: AlignmentDirectional,

    /// Decoration for the container that holds the
    /// [`persistent_footer_buttons`](Self::persistent_footer_buttons).
    ///
    /// By default, this container has a top border with a width of 1.0, created by
    /// [`Divider::create_border_side`].
    ///
    /// See also:
    ///
    ///  * [`persistent_footer_buttons`](Self::persistent_footer_buttons), which defines the
    ///    buttons to show in the footer.
    ///  * [`persistent_footer_alignment`](Self::persistent_footer_alignment), which defines
    ///    the alignment of the footer buttons.
    pub persistent_footer_decoration: Option<BoxDecoration>,

    /// A panel displayed to the side of the [`body`](Self::body), often hidden on mobile
    /// devices. Swipes in from either left-to-right ([`TextDirection::Ltr`]) or
    /// right-to-left ([`TextDirection::Rtl`])
    ///
    /// Typically a `Drawer`.
    ///
    /// To open the drawer, use the [`ScaffoldState::open_drawer`] function.
    ///
    /// To close the drawer, use either [`ScaffoldState::close_drawer`], `Navigator.pop`
    /// or press the escape key on the keyboard.
    pub drawer: Option<WidgetRef>,

    /// Optional callback that is called when the [`Scaffold::drawer`] is opened or closed.
    pub on_drawer_changed: Option<DrawerCallback>,

    /// A panel displayed to the side of the [`body`](Self::body), often hidden on mobile
    /// devices. Swipes in from right-to-left ([`TextDirection::Ltr`]) or left-to-right
    /// ([`TextDirection::Rtl`])
    ///
    /// Typically a `Drawer`.
    ///
    /// To open the drawer, use the [`ScaffoldState::open_end_drawer`] function.
    ///
    /// To close the drawer, use either [`ScaffoldState::close_end_drawer`], `Navigator.pop`
    /// or press the escape key on the keyboard.
    pub end_drawer: Option<WidgetRef>,

    /// Optional callback that is called when the [`Scaffold::end_drawer`] is opened or
    /// closed.
    pub on_end_drawer_changed: Option<DrawerCallback>,

    /// The color to use for the scrim that obscures primary content while a drawer is open.
    ///
    /// If this is null, then `DrawerThemeData.scrimColor` is used. If that
    /// is also null, then it defaults to `Colors.black54`.
    pub drawer_scrim_color: Option<Color>,

    /// A builder for the widget that obscures the [`body`](Self::body) when a draggable
    /// [`bottom_sheet`](Self::bottom_sheet) covers most of the screen. Dart's default is
    /// `_defaultBottomSheetScrimBuilder`.
    pub bottom_sheet_scrim_builder: BottomSheetScrimBuilder,

    /// The color of the [`Material`] widget that underlies the entire Scaffold.
    ///
    /// The theme's `ThemeData.scaffoldBackgroundColor` by default.
    pub background_color: Option<Color>,

    /// A bottom navigation bar to display at the bottom of the scaffold.
    ///
    /// Snack bars slide from underneath the bottom navigation bar while bottom
    /// sheets are stacked on top.
    ///
    /// The [`bottom_navigation_bar`](Self::bottom_navigation_bar) is rendered below the
    /// [`persistent_footer_buttons`](Self::persistent_footer_buttons) and the
    /// [`body`](Self::body).
    pub bottom_navigation_bar: Option<WidgetRef>,

    /// The persistent bottom sheet to display.
    ///
    /// A persistent bottom sheet shows information that supplements the primary
    /// content of the app. A persistent bottom sheet remains visible even when
    /// the user interacts with other parts of the app.
    ///
    /// A closely related widget is a modal bottom sheet, which is an alternative
    /// to a menu or a dialog and prevents the user from interacting with the rest
    /// of the app. Modal bottom sheets can be created and displayed with the
    /// `show_modal_bottom_sheet` function.
    ///
    /// Unlike the persistent bottom sheet displayed by
    /// [`ScaffoldState::show_bottom_sheet`], this bottom sheet is not a `LocalHistoryEntry`
    /// and cannot be dismissed with the scaffold appbar's back button.
    ///
    /// If a persistent bottom sheet created with [`ScaffoldState::show_bottom_sheet`]
    /// is already visible, it must be closed before building the Scaffold with a new
    /// [`bottom_sheet`](Self::bottom_sheet).
    ///
    /// The value of [`bottom_sheet`](Self::bottom_sheet) can be any widget at all. It's
    /// unlikely to actually be a [`BottomSheet`], which is used by the implementations of
    /// [`ScaffoldState::show_bottom_sheet`] and `show_modal_bottom_sheet`. Typically it's a
    /// widget that includes [`Material`].
    pub bottom_sheet: Option<WidgetRef>,

    /// If true the [`body`](Self::body) and the scaffold's floating widgets should size
    /// themselves to avoid the onscreen keyboard whose height is defined by the
    /// ambient [`MediaQuery`]'s `MediaQueryData.viewInsets` `bottom` property.
    ///
    /// For example, if there is an onscreen keyboard displayed above the
    /// scaffold, the body can be resized to avoid overlapping the keyboard, which
    /// prevents widgets inside the body from being obscured by the keyboard.
    ///
    /// Defaults to true.
    pub resize_to_avoid_bottom_inset: Option<bool>,

    /// Whether this scaffold is being displayed at the top of the screen.
    ///
    /// If true then the height of the `appBar` will be extended by the height
    /// of the screen's status bar, i.e. the top padding for [`MediaQuery`].
    ///
    /// The default value of this property, like the default value of
    /// `AppBar.primary`, is true.
    pub primary: bool,

    /// Determines the way that drag start behavior is handled.
    pub drawer_drag_start_behavior: waft_gestures::DragStartBehavior,

    /// The width of the area within which a horizontal swipe will open the
    /// drawer.
    ///
    /// By default, the value used is 20.0 added to the padding edge of
    /// `MediaQuery.paddingOf(context)` that corresponds to the surrounding
    /// [`TextDirection`]. This ensures that the drag area for notched devices is
    /// not obscured.
    pub drawer_edge_drag_width: Option<f64>,

    /// Determines if the [`Scaffold::drawer`] can be opened with a drag
    /// gesture on mobile.
    ///
    /// On desktop platforms, the drawer is not draggable.
    ///
    /// By default, the drag gesture is enabled on mobile.
    pub drawer_enable_open_drag_gesture: bool,

    /// Determines if the [`Scaffold::end_drawer`] can be opened with a
    /// gesture on mobile.
    ///
    /// On desktop platforms, the drawer is not draggable.
    ///
    /// By default, the drag gesture is enabled on mobile.
    pub end_drawer_enable_open_drag_gesture: bool,

    /// Restoration ID to save and restore the state of the [`Scaffold`].
    ///
    /// If it is non-null, the scaffold will persist and restore whether the
    /// [`drawer`](Self::drawer) and [`end_drawer`](Self::end_drawer) was open or closed.
    ///
    /// The state of this widget is persisted in a [`RestorationBucket`] claimed
    /// from the surrounding `RestorationScope` using the provided restoration ID.
    pub restoration_id: Option<String>,
}

impl Scaffold {
    /// Creates a visual scaffold for Material Design widgets.
    pub fn new() -> Scaffold {
        Scaffold {
            key: None,
            extend_body: false,
            drawer_barrier_dismissible: true,
            extend_body_behind_app_bar: false,
            app_bar: None,
            body: None,
            floating_action_button: None,
            floating_action_button_location: None,
            floating_action_button_animator: None,
            persistent_footer_buttons: None,
            persistent_footer_alignment: AlignmentDirectional::CENTER_END,
            persistent_footer_decoration: None,
            drawer: None,
            on_drawer_changed: None,
            end_drawer: None,
            on_end_drawer_changed: None,
            drawer_scrim_color: None,
            bottom_sheet_scrim_builder: Rc::new(Scaffold::default_bottom_sheet_scrim_builder),
            background_color: None,
            bottom_navigation_bar: None,
            bottom_sheet: None,
            resize_to_avoid_bottom_inset: None,
            primary: true,
            drawer_drag_start_behavior: waft_gestures::DragStartBehavior::Start,
            drawer_edge_drag_width: None,
            drawer_enable_open_drag_gesture: true,
            end_drawer_enable_open_drag_gesture: true,
            restoration_id: None,
        }
    }

    /// Dart `Scaffold(key:)`.
    pub fn key(mut self, key: KeyRef) -> Scaffold {
        self.key = Some(key);
        self
    }

    /// Dart `Scaffold(appBar:)`.
    pub fn app_bar(mut self, app_bar: PreferredSizeWidgetRef) -> Scaffold {
        self.app_bar = Some(app_bar);
        self
    }

    /// Dart `Scaffold(body:)`.
    pub fn body<K>(mut self, body: impl IntoWidget<K>) -> Scaffold {
        self.body = Some(body.into_widget());
        self
    }

    /// Dart `Scaffold(floatingActionButton:)`.
    pub fn floating_action_button<K>(
        mut self,
        floating_action_button: impl IntoWidget<K>,
    ) -> Scaffold {
        self.floating_action_button = Some(floating_action_button.into_widget());
        self
    }

    /// Dart `Scaffold(floatingActionButtonLocation:)`.
    pub fn floating_action_button_location(
        mut self,
        floating_action_button_location: FloatingActionButtonLocationRef,
    ) -> Scaffold {
        self.floating_action_button_location = Some(floating_action_button_location);
        self
    }

    /// Dart `Scaffold(floatingActionButtonAnimator:)`.
    pub fn floating_action_button_animator(
        mut self,
        floating_action_button_animator: FloatingActionButtonAnimatorRef,
    ) -> Scaffold {
        self.floating_action_button_animator = Some(floating_action_button_animator);
        self
    }

    /// Dart `Scaffold(persistentFooterButtons:)`.
    pub fn persistent_footer_buttons(
        mut self,
        persistent_footer_buttons: Vec<WidgetRef>,
    ) -> Scaffold {
        self.persistent_footer_buttons = Some(persistent_footer_buttons);
        self
    }

    /// Dart `Scaffold(persistentFooterAlignment:)`.
    pub fn persistent_footer_alignment(
        mut self,
        persistent_footer_alignment: AlignmentDirectional,
    ) -> Scaffold {
        self.persistent_footer_alignment = persistent_footer_alignment;
        self
    }

    /// Dart `Scaffold(persistentFooterDecoration:)`.
    pub fn persistent_footer_decoration(
        mut self,
        persistent_footer_decoration: BoxDecoration,
    ) -> Scaffold {
        self.persistent_footer_decoration = Some(persistent_footer_decoration);
        self
    }

    /// Dart `Scaffold(drawer:)`.
    pub fn drawer<K>(mut self, drawer: impl IntoWidget<K>) -> Scaffold {
        self.drawer = Some(drawer.into_widget());
        self
    }

    /// Dart `Scaffold(onDrawerChanged:)`.
    pub fn on_drawer_changed(
        mut self,
        on_drawer_changed: impl Fn(&mut App, bool) + 'static,
    ) -> Scaffold {
        self.on_drawer_changed = Some(Rc::new(on_drawer_changed));
        self
    }

    /// Dart `Scaffold(endDrawer:)`.
    pub fn end_drawer<K>(mut self, end_drawer: impl IntoWidget<K>) -> Scaffold {
        self.end_drawer = Some(end_drawer.into_widget());
        self
    }

    /// Dart `Scaffold(onEndDrawerChanged:)`.
    pub fn on_end_drawer_changed(
        mut self,
        on_end_drawer_changed: impl Fn(&mut App, bool) + 'static,
    ) -> Scaffold {
        self.on_end_drawer_changed = Some(Rc::new(on_end_drawer_changed));
        self
    }

    /// Dart `Scaffold(bottomNavigationBar:)`.
    pub fn bottom_navigation_bar<K>(
        mut self,
        bottom_navigation_bar: impl IntoWidget<K>,
    ) -> Scaffold {
        self.bottom_navigation_bar = Some(bottom_navigation_bar.into_widget());
        self
    }

    /// Dart `Scaffold(bottomSheet:)`.
    pub fn bottom_sheet<K>(mut self, bottom_sheet: impl IntoWidget<K>) -> Scaffold {
        self.bottom_sheet = Some(bottom_sheet.into_widget());
        self
    }

    /// Dart `Scaffold(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> Scaffold {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `Scaffold(resizeToAvoidBottomInset:)`.
    pub fn resize_to_avoid_bottom_inset(mut self, resize_to_avoid_bottom_inset: bool) -> Scaffold {
        self.resize_to_avoid_bottom_inset = Some(resize_to_avoid_bottom_inset);
        self
    }

    /// Dart `Scaffold(primary:)`.
    pub fn primary(mut self, primary: bool) -> Scaffold {
        self.primary = primary;
        self
    }

    /// Dart `Scaffold(drawerDragStartBehavior:)`.
    pub fn drawer_drag_start_behavior(
        mut self,
        drawer_drag_start_behavior: waft_gestures::DragStartBehavior,
    ) -> Scaffold {
        self.drawer_drag_start_behavior = drawer_drag_start_behavior;
        self
    }

    /// Dart `Scaffold(extendBody:)`.
    pub fn extend_body(mut self, extend_body: bool) -> Scaffold {
        self.extend_body = extend_body;
        self
    }

    /// Dart `Scaffold(drawerBarrierDismissible:)`.
    pub fn drawer_barrier_dismissible(mut self, drawer_barrier_dismissible: bool) -> Scaffold {
        self.drawer_barrier_dismissible = drawer_barrier_dismissible;
        self
    }

    /// Dart `Scaffold(extendBodyBehindAppBar:)`.
    pub fn extend_body_behind_app_bar(mut self, extend_body_behind_app_bar: bool) -> Scaffold {
        self.extend_body_behind_app_bar = extend_body_behind_app_bar;
        self
    }

    /// Dart `Scaffold(drawerScrimColor:)`.
    pub fn drawer_scrim_color(mut self, drawer_scrim_color: Color) -> Scaffold {
        self.drawer_scrim_color = Some(drawer_scrim_color);
        self
    }

    /// Dart `Scaffold(bottomSheetScrimBuilder:)`.
    pub fn bottom_sheet_scrim_builder(
        mut self,
        bottom_sheet_scrim_builder: impl Fn(
            &mut App,
            BuildContext,
            AnyAnimation<f64>,
        ) -> Option<WidgetRef>
        + 'static,
    ) -> Scaffold {
        self.bottom_sheet_scrim_builder = Rc::new(bottom_sheet_scrim_builder);
        self
    }

    /// Dart `Scaffold(drawerEdgeDragWidth:)`.
    pub fn drawer_edge_drag_width(mut self, drawer_edge_drag_width: f64) -> Scaffold {
        self.drawer_edge_drag_width = Some(drawer_edge_drag_width);
        self
    }

    /// Dart `Scaffold(drawerEnableOpenDragGesture:)`.
    pub fn drawer_enable_open_drag_gesture(
        mut self,
        drawer_enable_open_drag_gesture: bool,
    ) -> Scaffold {
        self.drawer_enable_open_drag_gesture = drawer_enable_open_drag_gesture;
        self
    }

    /// Dart `Scaffold(endDrawerEnableOpenDragGesture:)`.
    pub fn end_drawer_enable_open_drag_gesture(
        mut self,
        end_drawer_enable_open_drag_gesture: bool,
    ) -> Scaffold {
        self.end_drawer_enable_open_drag_gesture = end_drawer_enable_open_drag_gesture;
        self
    }

    /// Dart `Scaffold(restorationId:)`.
    pub fn restoration_id(mut self, restoration_id: impl Into<String>) -> Scaffold {
        self.restoration_id = Some(restoration_id.into());
        self
    }

    /// Finds the [`ScaffoldState`] from the closest instance of this class that
    /// encloses the given context.
    ///
    /// If no instance of this class encloses the given context, will cause an
    /// assert in debug mode, and throw an exception in release mode.
    ///
    /// This method can be expensive (it walks the element tree).
    ///
    /// Typical usage of the [`Scaffold::of`] function is to call it from within the
    /// `build` method of a child of a [`Scaffold`].
    ///
    /// When the [`Scaffold`] is actually created in the same `build` function, the
    /// `context` argument to the `build` function can't be used to find the
    /// [`Scaffold`] (since it's "above" the widget being returned in the widget
    /// tree). In such cases, the following technique with a [`Builder`] can be used
    /// to provide a new scope with a [`BuildContext`] that is "under" the
    /// [`Scaffold`].
    ///
    /// A more efficient solution is to split your build function into several
    /// widgets. This introduces a new context from which you can obtain the
    /// [`Scaffold`]. In this solution, you would have an outer widget that creates
    /// the [`Scaffold`] populated by instances of your new inner widgets, and then
    /// in these inner widgets you would use [`Scaffold::of`].
    ///
    /// A less elegant but more expedient solution is assign a [`GlobalKey`] to the
    /// [`Scaffold`], then use the `key.current_state` method to obtain the
    /// [`ScaffoldState`] rather than using the [`Scaffold::of`] function.
    ///
    /// If there is no [`Scaffold`] in scope, then this will throw an exception.
    /// To return null if there is no [`Scaffold`], use [`maybe_of`](Self::maybe_of).
    pub fn of(app: &mut App, context: BuildContext) -> Handle<ScaffoldState> {
        let result = context.find_ancestor_state_of_type::<ScaffoldState>(app);
        if let Some(result) = result {
            return result;
        }
        panic!(
            "Scaffold.of() called with a context that does not contain a Scaffold.\n\
             No Scaffold ancestor could be found starting from the context that was passed to \
             Scaffold.of(). This usually happens when the context provided is from the same \
             StatefulWidget as that whose build function actually creates the Scaffold widget \
             being sought.\n\
             There are several ways to avoid this problem. The simplest is to use a Builder to \
             get a context that is \"under\" the Scaffold. For an example of this, please see \
             the documentation for Scaffold.of():\n  \
             https://api.flutter.dev/flutter/material/Scaffold/of.html\n\
             A more efficient solution is to split your build function into several widgets. \
             This introduces a new context from which you can obtain the Scaffold. In this \
             solution, you would have an outer widget that creates the Scaffold populated by \
             instances of your new inner widgets, and then in these inner widgets you would use \
             Scaffold.of().\n\
             A less elegant but more expedient solution is assign a GlobalKey to the Scaffold, \
             then use the key.currentState property to obtain the ScaffoldState rather than \
             using the Scaffold.of() function.\n\
             The context used was:\n  {}",
            context.widget(app).type_name()
        );
    }

    /// Finds the [`ScaffoldState`] from the closest instance of this class that
    /// encloses the given context.
    ///
    /// If no instance of this class encloses the given context, will return `None`.
    /// To throw an exception instead, use [`of`](Self::of) instead of this function.
    ///
    /// This method can be expensive (it walks the element tree).
    ///
    /// See also:
    ///
    ///  * [`of`](Self::of), a similar function to this one that throws if no instance
    ///    encloses the given context. Also includes some sample code in its
    ///    documentation.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<Handle<ScaffoldState>> {
        context.find_ancestor_state_of_type::<ScaffoldState>(app)
    }

    /// Returns a notifier of the [`ScaffoldGeometry`] for the [`Scaffold`] that encloses
    /// the given context.
    ///
    /// The geometry notifier is a [`ChangeNotifier`] whose
    /// [`value`](ScaffoldGeometryNotifier::value) is the [`ScaffoldGeometry`] of the closest
    /// [`Scaffold`] ancestor of the given context; read it only while painting.
    ///
    /// This method can be expensive (it walks the element tree).
    ///
    /// For an example of using this, see the `BottomAppBar`, which uses it to paint a notch
    /// around the `FloatingActionButton`.
    ///
    /// # Panics
    ///
    /// If no [`Scaffold`] encloses `context`.
    pub fn geometry_of(app: &mut App, context: BuildContext) -> Handle<ScaffoldGeometryNotifier> {
        let scaffold_scope =
            ScaffoldScope::inherit_from(app, context, Some(ScaffoldAspect::Geometry));
        match scaffold_scope {
            Some(scaffold_scope) => scaffold_scope.geometry_notifier,
            None => panic!(
                "Scaffold.geometryOf() called with a context that does not contain a Scaffold.\n\
                 This usually happens when the context provided is from the same StatefulWidget \
                 as that whose build function actually creates the Scaffold widget being \
                 sought.\n\
                 There are several ways to avoid this problem. The simplest is to use a Builder \
                 to get a context that is \"under\" the Scaffold. For an example of this, please \
                 see the documentation for Scaffold.of():\n  \
                 https://api.flutter.dev/flutter/material/Scaffold/of.html\n\
                 A more efficient solution is to split your build function into several \
                 widgets. This introduces a new context from which you can obtain the Scaffold. \
                 In this solution, you would have an outer widget that creates the Scaffold \
                 populated by instances of your new inner widgets, and then in these inner \
                 widgets you would use Scaffold.geometryOf().\n\
                 The context used was:\n  {}",
                context.widget(app).type_name()
            ),
        }
    }

    /// Whether the Scaffold that most tightly encloses the given context has a
    /// drawer.
    ///
    /// If this is being used during a build (for example to decide whether to
    /// show an "open drawer" button), set the `register_for_updates` argument to
    /// true. This will then set up an `InheritedWidget` relationship with the
    /// [`Scaffold`] so that the client widget gets rebuilt whenever the
    /// [`has_drawer`](Self::has_drawer) value changes.
    ///
    /// Dart's default `register_for_updates` is true.
    ///
    /// See also:
    ///
    ///  * [`Scaffold::of`], which provides access to the [`ScaffoldState`] object as a
    ///    whole, from which you can show bottom sheets, and so forth.
    ///  * [`ScaffoldState::open_drawer`] and [`ScaffoldState::open_end_drawer`],
    ///    which open the drawer.
    pub fn has_drawer(app: &mut App, context: BuildContext, register_for_updates: bool) -> bool {
        if register_for_updates {
            let scaffold =
                ScaffoldScope::inherit_from(app, context, Some(ScaffoldAspect::HasDrawer));
            scaffold.is_some_and(|scaffold| scaffold.has_drawer)
        } else {
            let scaffold = context.find_ancestor_state_of_type::<ScaffoldState>(app);
            scaffold.is_some_and(|scaffold| scaffold.has_drawer(app))
        }
    }

    fn default_bottom_sheet_scrim_builder(
        _app: &mut App,
        _context: BuildContext,
        animation: AnyAnimation<f64>,
    ) -> Option<WidgetRef> {
        Some(
            ListenableBuilder::new(Rc::new(animation), move |app, _context, _child| {
                let extent_remaining =
                    K_BOTTOM_SHEET_DOMINATES_PERCENTAGE * (1.0 - animation.value(app));
                let floating_button_visibility_value =
                    extent_remaining * K_BOTTOM_SHEET_DOMINATES_PERCENTAGE * 10.0;

                let opacity = f64::max(
                    K_MIN_BOTTOM_SHEET_SCRIM_OPACITY,
                    K_MAX_BOTTOM_SHEET_SCRIM_OPACITY - floating_button_visibility_value,
                );

                #[expect(deprecated, reason = "Dart calls the deprecated `withOpacity`")]
                let color = Colors::BLACK.with_opacity(opacity);
                ModalBarrier::new()
                    .dismissible(false)
                    .color(color)
                    .into_widget()
            })
            .into_widget(),
        )
    }
}

impl Default for Scaffold {
    fn default() -> Scaffold {
        Scaffold::new()
    }
}

impl Debug for Scaffold {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scaffold")
            .field("appBar", &self.app_bar)
            .field("body", &self.body)
            .field("floatingActionButton", &self.floating_action_button)
            .field("drawer", &self.drawer)
            .field("endDrawer", &self.end_drawer)
            .field("bottomNavigationBar", &self.bottom_navigation_bar)
            .field("bottomSheet", &self.bottom_sheet)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for Scaffold {
    type State = ScaffoldState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ScaffoldState {
        ScaffoldState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            restoration: RestorationMixinData::new(),
            observer: None,
            drawer_key: GlobalKey::new(),
            end_drawer_key: GlobalKey::new(),
            body_key: GlobalKey::new(),
            status_bar_key: GlobalKey::new(),
            app_bar_max_height: None,
            drawer_opened: None,
            end_drawer_opened: None,
            scaffold_messenger: None,
            messenger_snack_bar: None,
            messenger_material_banner: None,
            dismissed_bottom_sheets: Vec::new(),
            current_bottom_sheet: None,
            current_bottom_sheet_key: GlobalKey::new(),
            persistent_sheet_history_entry: None,
            floating_action_button_move_controller: None,
            floating_action_button_animator: None,
            previous_floating_action_button_location: None,
            floating_action_button_location: None,
            floating_action_button_visibility_controller: None,
            geometry_notifier: None,
            bottom_sheet_scrim_animation_controller: None,
            show_body_scrim: false,
            dismiss_drawer_action: None,
        }
    }
}

/// State for a [`Scaffold`].
///
/// Can display [`BottomSheet`]s. Retrieve a [`ScaffoldState`] from the current
/// [`BuildContext`] using [`Scaffold::of`].
pub struct ScaffoldState {
    state: StateData<Scaffold>,
    ticker_provider: TickerProviderStateMixinData,
    restoration: RestorationMixinData,
    /// The `WidgetsBindingObserver` this state registers: an `Rc` of its own handle, made once.
    observer: Option<WidgetsBindingObserverRef>,

    // DRAWER API
    /// Dart's `GlobalKey<DrawerControllerState>`.
    drawer_key: GlobalKey,
    /// Dart's `GlobalKey<DrawerControllerState>`.
    end_drawer_key: GlobalKey,

    body_key: GlobalKey,
    status_bar_key: GlobalKey,

    app_bar_max_height: Option<f64>,

    /// Dart's `final RestorableBool _drawerOpened = RestorableBool(false)`, made in
    /// `init_state` because a property is an arena object.
    drawer_opened: Option<Handle<RestorableBool>>,
    /// Dart's `_endDrawerOpened`, made in `init_state`.
    end_drawer_opened: Option<Handle<RestorableBool>>,

    // Used for both the snackbar and material banner APIs
    scaffold_messenger: Option<Handle<ScaffoldMessengerState>>,

    // SNACKBAR API
    messenger_snack_bar: Option<Rc<ScaffoldFeatureController<SnackBar, SnackBarClosedReason>>>,

    // MATERIAL BANNER API

    // The _messengerMaterialBanner represents the current MaterialBanner being managed by
    // the ScaffoldMessenger, instead of the Scaffold.
    messenger_material_banner:
        Option<Rc<ScaffoldFeatureController<MaterialBanner, MaterialBannerClosedReason>>>,

    // PERSISTENT BOTTOM SHEET API

    // Contains bottom sheets that may still be animating out of view.
    // Important if the app/user takes an action that could repeatedly show a
    // bottom sheet.
    /// Dart's `List<_StandardBottomSheet>`, compared by identity.
    dismissed_bottom_sheets: Vec<WidgetRef>,
    current_bottom_sheet: Option<Rc<PersistentBottomSheetController>>,
    current_bottom_sheet_key: GlobalKey,
    persistent_sheet_history_entry: Option<Handle<LocalHistoryEntry>>,

    // Floating Action Button API
    floating_action_button_move_controller: Option<Handle<AnimationController>>,
    floating_action_button_animator: Option<FloatingActionButtonAnimatorRef>,
    previous_floating_action_button_location: Option<FloatingActionButtonLocationRef>,
    floating_action_button_location: Option<FloatingActionButtonLocationRef>,

    floating_action_button_visibility_controller: Option<Handle<AnimationController>>,

    // INTERNALS
    geometry_notifier: Option<Handle<ScaffoldGeometryNotifier>>,

    bottom_sheet_scrim_animation_controller: Option<Handle<AnimationController>>,
    show_body_scrim: bool,
    /// The `_DismissDrawerAction` Dart's build makes anew for the scaffold's `Builder`, kept for
    /// the state's life so builds do not fill the arena; the `Builder` gives it its context.
    dismiss_drawer_action: Option<Handle<DismissDrawerAction>>,
}

impl ScaffoldState {
    // DRAWER API

    /// Whether this scaffold has a non-null [`Scaffold::app_bar`].
    pub fn has_app_bar(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).app_bar.is_some()
    }

    /// Whether this scaffold has a non-null [`Scaffold::drawer`].
    pub fn has_drawer(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).drawer.is_some()
    }

    /// Whether this scaffold has a non-null [`Scaffold::end_drawer`].
    pub fn has_end_drawer(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).end_drawer.is_some()
    }

    /// Whether this scaffold has a non-null [`Scaffold::floating_action_button`].
    pub fn has_floating_action_button(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).floating_action_button.is_some()
    }

    /// The max height the [`Scaffold::app_bar`] uses.
    ///
    /// This is based on the appBar preferred height plus the top padding.
    pub fn app_bar_max_height(self: Handle<Self>, app: &App) -> Option<f64> {
        app.get(self).app_bar_max_height
    }

    fn drawer_opened(self: Handle<Self>, app: &App) -> Handle<RestorableBool> {
        app.get(self).drawer_opened.expect("made in init_state")
    }

    fn end_drawer_opened(self: Handle<Self>, app: &App) -> Handle<RestorableBool> {
        app.get(self).end_drawer_opened.expect("made in init_state")
    }

    /// Whether the [`Scaffold::drawer`] is opened.
    ///
    /// See also:
    ///
    ///  * [`ScaffoldState::open_drawer`], which opens the [`Scaffold::drawer`] of a
    ///    [`Scaffold`].
    pub fn is_drawer_open(self: Handle<Self>, app: &App) -> bool {
        *RestorableValue::value(self.drawer_opened(app), app)
    }

    /// Whether the drawer's barrier can be dismissed by tapping on it.
    pub fn is_drawer_barrier_dismissible(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).drawer_barrier_dismissible
    }

    /// Whether the [`Scaffold::end_drawer`] is opened.
    ///
    /// See also:
    ///
    ///  * [`ScaffoldState::open_end_drawer`], which opens the [`Scaffold::end_drawer`] of a
    ///    [`Scaffold`].
    pub fn is_end_drawer_open(self: Handle<Self>, app: &App) -> bool {
        *RestorableValue::value(self.end_drawer_opened(app), app)
    }

    fn drawer_opened_callback(self: Handle<Self>, app: &mut App, is_opened: bool) {
        let drawer_key = app.get(self).drawer_key.clone();
        if self.is_drawer_open(app) != is_opened
            && drawer_key
                .current_state::<DrawerControllerState>(app)
                .is_some()
        {
            RestorableValue::set_value(self.drawer_opened(app), app, is_opened);
            self.set_state(app, |_| {});
            if let Some(on_drawer_changed) = self.widget(app).on_drawer_changed.clone() {
                on_drawer_changed(app, is_opened);
            }
        }
    }

    fn end_drawer_opened_callback(self: Handle<Self>, app: &mut App, is_opened: bool) {
        let end_drawer_key = app.get(self).end_drawer_key.clone();
        if self.is_end_drawer_open(app) != is_opened
            && end_drawer_key
                .current_state::<DrawerControllerState>(app)
                .is_some()
        {
            RestorableValue::set_value(self.end_drawer_opened(app), app, is_opened);
            self.set_state(app, |_| {});
            if let Some(on_end_drawer_changed) = self.widget(app).on_end_drawer_changed.clone() {
                on_end_drawer_changed(app, is_opened);
            }
        }
    }

    /// Opens the [`Drawer`](crate::Drawer) (if any).
    ///
    /// If the scaffold has a non-null [`Scaffold::drawer`], this function will cause
    /// the drawer to begin its entrance animation.
    ///
    /// Normally this is not needed since the [`Scaffold`] automatically shows an
    /// appropriate `IconButton`, and handles the edge-swipe gesture, to show the
    /// drawer.
    ///
    /// To close the drawer, use either [`ScaffoldState::close_drawer`] or
    /// `Navigator.pop`.
    pub fn open_drawer(self: Handle<Self>, app: &mut App) {
        let end_drawer_key = app.get(self).end_drawer_key.clone();
        if let Some(end_drawer) = end_drawer_key.current_state::<DrawerControllerState>(app)
            && self.is_end_drawer_open(app)
        {
            end_drawer.close(app);
        }
        let drawer_key = app.get(self).drawer_key.clone();
        if let Some(drawer) = drawer_key.current_state::<DrawerControllerState>(app) {
            drawer.open(app);
        }
    }

    /// Opens the end side [`Drawer`](crate::Drawer) (if any).
    ///
    /// If the scaffold has a non-null [`Scaffold::end_drawer`], this function will cause
    /// the end side drawer to begin its entrance animation.
    ///
    /// Normally this is not needed since the [`Scaffold`] automatically shows an
    /// appropriate `IconButton`, and handles the edge-swipe gesture, to show the
    /// drawer.
    ///
    /// To close the end side drawer, use either [`ScaffoldState::close_end_drawer`] or
    /// `Navigator.pop`.
    pub fn open_end_drawer(self: Handle<Self>, app: &mut App) {
        let drawer_key = app.get(self).drawer_key.clone();
        if let Some(drawer) = drawer_key.current_state::<DrawerControllerState>(app)
            && self.is_drawer_open(app)
        {
            drawer.close(app);
        }
        let end_drawer_key = app.get(self).end_drawer_key.clone();
        if let Some(end_drawer) = end_drawer_key.current_state::<DrawerControllerState>(app) {
            end_drawer.open(app);
        }
    }

    // This is used to update the _messengerSnackBar by the ScaffoldMessenger.
    fn update_snack_bar(self: Handle<Self>, app: &mut App) {
        let scaffold_messenger = app
            .get(self)
            .scaffold_messenger
            .expect("a registered scaffold has a messenger");
        let messenger_snack_bar = app.get(scaffold_messenger).snack_bars.front().cloned();

        if !option_rc_ptr_eq(&app.get(self).messenger_snack_bar, &messenger_snack_bar) {
            self.set_state(app, move |state| {
                state.messenger_snack_bar = messenger_snack_bar;
            });
        }
    }

    // This is used to update the _messengerMaterialBanner by the ScaffoldMessenger.
    fn update_material_banner(self: Handle<Self>, app: &mut App) {
        let scaffold_messenger = app
            .get(self)
            .scaffold_messenger
            .expect("a registered scaffold has a messenger");
        let messenger_material_banner = app
            .get(scaffold_messenger)
            .material_banners
            .front()
            .cloned();

        if !option_rc_ptr_eq(
            &app.get(self).messenger_material_banner,
            &messenger_material_banner,
        ) {
            self.set_state(app, move |state| {
                state.messenger_material_banner = messenger_material_banner;
            });
        }
    }

    // PERSISTENT BOTTOM SHEET API

    fn maybe_build_persistent_bottom_sheet(self: Handle<Self>, app: &mut App) {
        if self.widget(app).bottom_sheet.is_some() && app.get(self).current_bottom_sheet.is_none() {
            // The new _currentBottomSheet is not a local history entry so a "back" button
            // will not be added to the Scaffold's appbar and the bottom sheet will not
            // support drag or swipe to dismiss.
            let animation_controller = BottomSheet::create_animation_controller(app, self, None);
            animation_controller.set_value(app, 1.0);
            let persistent_bottom_sheet_extent_changed =
                move |app: &mut App, notification: &DraggableScrollableNotification| -> bool {
                    if notification.extent - notification.initial_extent > PRECISION_ERROR_TOLERANCE
                    {
                        if app.get(self).persistent_sheet_history_entry.is_none() {
                            let notification_context = notification.context;
                            let entry = LocalHistoryEntry::new(app).on_remove(
                                app,
                                Listener::new(move |app| {
                                    DraggableScrollableActuator::reset(app, notification_context);
                                    self.show_body_scrim(app, false, 0.0);
                                    self.floating_action_button_visibility_controller(app)
                                        .set_value(app, 1.0);
                                    app.get_mut(self).persistent_sheet_history_entry = None;
                                }),
                            );
                            app.get_mut(self).persistent_sheet_history_entry = Some(entry);
                            let context = self.context(app);
                            AnyModalRoute::of(app, context)
                                .expect("a persistent bottom sheet inside a modal route")
                                .add_local_history_entry(app, entry);
                        }
                    } else if let Some(entry) = app.get(self).persistent_sheet_history_entry {
                        entry.remove(app);
                    }
                    false
                };

            // Stop the animation and unmount the dismissed sheets from the tree immediately,
            // otherwise may cause duplicate GlobalKey assertion if the sheet sub-tree contains
            // GlobalKey widgets.
            if !app.get(self).dismissed_bottom_sheets.is_empty() {
                let sheets = app.get(self).dismissed_bottom_sheets.clone();
                for sheet in sheets {
                    let animation_controller = downcast_widget::<StandardBottomSheet>(&*sheet)
                        .expect("a dismissed sheet is a standard bottom sheet")
                        .animation_controller;
                    animation_controller.reset(app);
                }
                debug_assert!(app.get(self).dismissed_bottom_sheets.is_empty());
            }

            let current_bottom_sheet_key = app.get(self).current_bottom_sheet_key.clone();
            let persistent_bottom_sheet_extent_changed =
                Rc::new(persistent_bottom_sheet_extent_changed);
            let builder: WidgetBuilder = Rc::new(move |_app, _context| {
                let persistent_bottom_sheet_extent_changed =
                    Rc::clone(&persistent_bottom_sheet_extent_changed);
                NotificationListener::<DraggableScrollableNotification>::new(
                    DraggableScrollableActuator::new(
                        StatefulBuilder::new(move |app, _context, _set_state| {
                            self.widget(app)
                                .bottom_sheet
                                .clone()
                                .unwrap_or_else(|| SizedBox::shrink().into_widget())
                        })
                        .key(current_bottom_sheet_key.clone().into()),
                    ),
                )
                .on_notification(move |app, notification| {
                    persistent_bottom_sheet_extent_changed(app, notification)
                })
                .into_widget()
            });
            let current_bottom_sheet = self.build_bottom_sheet(
                app,
                builder,
                true,
                animation_controller,
                BottomSheetOptions::default(),
            );
            app.get_mut(self).current_bottom_sheet = Some(current_bottom_sheet);
        }
    }

    fn close_current_bottom_sheet(self: Handle<Self>, app: &mut App) {
        if let Some(current_bottom_sheet) = app.get(self).current_bottom_sheet.clone() {
            if !current_bottom_sheet.is_local_history_entry {
                (current_bottom_sheet.feature.close)(app);
            }
            if cfg!(debug_assertions)
                && let Some(current_bottom_sheet) = app.get(self).current_bottom_sheet.clone()
            {
                current_bottom_sheet
                    .feature
                    .completer
                    .future()
                    .then(app, move |app, ()| {
                        debug_assert!(app.get(self).current_bottom_sheet.is_none());
                    });
            }
        }
    }

    /// Closes [`Scaffold::drawer`] if it is currently opened.
    ///
    /// See [`ScaffoldState::open_drawer`] to open the [`Scaffold::drawer`].
    pub fn close_drawer(self: Handle<Self>, app: &mut App) {
        if self.has_drawer(app) && self.is_drawer_open(app) {
            let drawer_key = app.get(self).drawer_key.clone();
            drawer_key
                .current_state::<DrawerControllerState>(app)
                .expect("an open drawer is mounted")
                .close(app);
        }
    }

    /// Closes [`Scaffold::end_drawer`] if it is currently opened.
    ///
    /// See [`ScaffoldState::open_end_drawer`] to open the [`Scaffold::end_drawer`].
    pub fn close_end_drawer(self: Handle<Self>, app: &mut App) {
        if self.has_end_drawer(app) && self.is_end_drawer_open(app) {
            let end_drawer_key = app.get(self).end_drawer_key.clone();
            end_drawer_key
                .current_state::<DrawerControllerState>(app)
                .expect("an open drawer is mounted")
                .close(app);
        }
    }

    fn update_persistent_bottom_sheet(self: Handle<Self>, app: &mut App) {
        let current_bottom_sheet_key = app.get(self).current_bottom_sheet_key.clone();
        current_bottom_sheet_key
            .current_state::<StatefulBuilderState>(app)
            .expect("the persistent bottom sheet is mounted")
            .set_state(app, |_| {});
    }

    fn build_bottom_sheet(
        self: Handle<Self>,
        app: &mut App,
        builder: WidgetBuilder,
        is_persistent: bool,
        animation_controller: Handle<AnimationController>,
        options: BottomSheetOptions,
    ) -> Rc<PersistentBottomSheetController> {
        debug_assert!(
            !(self.widget(app).bottom_sheet.is_some()
                && is_persistent
                && app.get(self).current_bottom_sheet.is_some()),
            "Scaffold.bottomSheet cannot be specified while a bottom sheet \
             displayed with showBottomSheet() is still visible.\n\
             Rebuild the Scaffold with a null bottomSheet before calling showBottomSheet()."
        );

        let completer = Completer::<()>::new();
        let bottom_sheet_key = GlobalKey::new();
        // Dart's `late _StandardBottomSheet bottomSheet`, read by the closures below.
        let bottom_sheet: Rc<std::cell::OnceCell<WidgetRef>> = Rc::new(std::cell::OnceCell::new());

        let removed_entry = Rc::new(Cell::new(false));
        let doing_dispose = Rc::new(Cell::new(false));

        let remove_persistent_sheet_history_entry_if_needed = move |app: &mut App| {
            debug_assert!(is_persistent);
            if let Some(entry) = app.get(self).persistent_sheet_history_entry {
                entry.remove(app);
                app.get_mut(self).persistent_sheet_history_entry = None;
            }
        };

        let remove_current_bottom_sheet: VoidCallback = Rc::new({
            let removed_entry = Rc::clone(&removed_entry);
            let bottom_sheet = Rc::clone(&bottom_sheet);
            let bottom_sheet_key = bottom_sheet_key.clone();
            let completer = completer.clone();
            move |app: &mut App| {
                removed_entry.set(true);
                let Some(current_bottom_sheet) = app.get(self).current_bottom_sheet.clone() else {
                    return;
                };
                let bottom_sheet = Rc::clone(bottom_sheet.get().expect("built below"));
                debug_assert!(Rc::ptr_eq(
                    &current_bottom_sheet.feature.widget,
                    &bottom_sheet
                ));
                debug_assert!(
                    bottom_sheet_key
                        .current_state::<StandardBottomSheetState>(app)
                        .is_some()
                );
                self.show_floating_action_button(app);

                if is_persistent {
                    remove_persistent_sheet_history_entry_if_needed(app);
                }

                bottom_sheet_key
                    .current_state::<StandardBottomSheetState>(app)
                    .expect("checked above")
                    .close(app);
                app.get_mut(self).show_body_scrim = false;
                self.bottom_sheet_scrim_animation_controller(app)
                    .set_value(app, 0.0);
                self.set_state(app, |state| {
                    state.current_bottom_sheet = None;
                });

                if !animation_controller.is_dismissed(app) {
                    app.get_mut(self).dismissed_bottom_sheets.push(bottom_sheet);
                }
                completer.complete(app, ());
            }
        });

        let entry = if is_persistent {
            None
        } else {
            let removed_entry = Rc::clone(&removed_entry);
            let doing_dispose = Rc::clone(&doing_dispose);
            let bottom_sheet = Rc::clone(&bottom_sheet);
            let remove_current_bottom_sheet = Rc::clone(&remove_current_bottom_sheet);
            Some(LocalHistoryEntry::new(app).on_remove(
                app,
                Listener::new(move |app| {
                    let current_is_this_sheet = app
                        .get(self)
                        .current_bottom_sheet
                        .as_ref()
                        .is_some_and(|current| {
                            bottom_sheet
                                .get()
                                .is_some_and(|sheet| Rc::ptr_eq(&current.feature.widget, sheet))
                        });
                    if !removed_entry.get() && current_is_this_sheet && !doing_dispose.get() {
                        remove_current_bottom_sheet(app);
                    }
                }),
            ))
        };

        let remove_entry_if_needed = {
            let removed_entry = Rc::clone(&removed_entry);
            Rc::new(move |app: &mut App| {
                if !is_persistent && !removed_entry.get() {
                    debug_assert!(entry.is_some());
                    entry.expect("not persistent").remove(app);
                    removed_entry.set(true);
                }
            })
        };

        let sheet: WidgetRef = StandardBottomSheet {
            key: bottom_sheet_key.clone().into(),
            animation_controller,
            enable_drag: options.enable_drag.unwrap_or(!is_persistent),
            show_drag_handle: options.show_drag_handle,
            on_closing: Some(Rc::new({
                let bottom_sheet = Rc::clone(&bottom_sheet);
                let remove_entry_if_needed = Rc::clone(&remove_entry_if_needed);
                move |app: &mut App| {
                    let Some(current_bottom_sheet) = app.get(self).current_bottom_sheet.clone()
                    else {
                        return;
                    };
                    debug_assert!(bottom_sheet.get().is_some_and(|sheet| Rc::ptr_eq(
                        &current_bottom_sheet.feature.widget,
                        sheet
                    )));
                    remove_entry_if_needed(app);
                }
            })),
            on_dismissed: Some(Rc::new({
                let bottom_sheet = Rc::clone(&bottom_sheet);
                move |app: &mut App| {
                    let sheet = bottom_sheet.get().expect("built below");
                    let position = app
                        .get(self)
                        .dismissed_bottom_sheets
                        .iter()
                        .position(|dismissed| Rc::ptr_eq(dismissed, sheet));
                    if let Some(position) = position {
                        self.set_state(app, |state| {
                            state.dismissed_bottom_sheets.remove(position);
                        });
                    }
                }
            })),
            on_dispose: Some(Rc::new({
                let doing_dispose = Rc::clone(&doing_dispose);
                let remove_entry_if_needed = Rc::clone(&remove_entry_if_needed);
                let should_dispose_animation_controller =
                    options.should_dispose_animation_controller;
                move |app: &mut App| {
                    doing_dispose.set(true);
                    remove_entry_if_needed(app);
                    if should_dispose_animation_controller {
                        animation_controller.dispose(app);
                    }
                }
            })),
            builder,
            is_persistent,
            background_color: options.background_color,
            elevation: options.elevation,
            shape: options.shape,
            clip_behavior: options.clip_behavior,
            constraints: options.constraints,
        }
        .into_widget();
        assert!(bottom_sheet.set(Rc::clone(&sheet)).is_ok());

        if !is_persistent {
            let context = self.context(app);
            AnyModalRoute::of(app, context)
                .expect("a bottom sheet inside a modal route")
                .add_local_history_entry(app, entry.expect("not persistent"));
        }

        let close: VoidCallback = match entry {
            Some(entry) => Rc::new(move |app: &mut App| entry.remove(app)),
            None => remove_current_bottom_sheet,
        };
        let set_state: StateSetter = Rc::new(move |app: &mut App, f: &mut dyn FnMut(&mut App)| {
            if let Some(state) = bottom_sheet_key.current_state::<StandardBottomSheetState>(app) {
                f(app);
                state.set_state(app, |_| {});
            }
        });
        Rc::new(PersistentBottomSheetController {
            feature: ScaffoldFeatureController {
                widget: sheet,
                completer,
                close,
                set_state: Some(set_state),
                _widget_type: PhantomData,
            },
            is_local_history_entry: !is_persistent,
        })
    }

    /// Shows a Material Design bottom sheet in the nearest [`Scaffold`]. To show
    /// a persistent bottom sheet, use the [`Scaffold::bottom_sheet`].
    ///
    /// Returns a controller that can be used to close and otherwise manipulate the
    /// bottom sheet.
    ///
    /// To rebuild the bottom sheet (e.g. if it is stateful), call
    /// [`PersistentBottomSheetController::set_state`] on the controller returned by
    /// this method.
    ///
    /// The new bottom sheet becomes a `LocalHistoryEntry` for the enclosing
    /// `ModalRoute` and a back button is added to the app bar of the [`Scaffold`]
    /// that closes the bottom sheet.
    ///
    /// The `transition_animation_controller` controls the bottom sheet's entrance and
    /// exit animations. It's up to the owner of the controller to call
    /// [`AnimationController::dispose`] when the controller is no longer needed.
    ///
    /// To create a persistent bottom sheet that is not a `LocalHistoryEntry` and
    /// does not add a back button to the enclosing Scaffold's app bar, use the
    /// [`Scaffold::bottom_sheet`] constructor parameter.
    ///
    /// A persistent bottom sheet shows information that supplements the primary
    /// content of the app. A persistent bottom sheet remains visible even when
    /// the user interacts with other parts of the app.
    ///
    /// A closely related widget is a modal bottom sheet, which is an alternative
    /// to a menu or a dialog and prevents the user from interacting with the rest
    /// of the app. Modal bottom sheets can be created and displayed with the
    /// `show_modal_bottom_sheet` function.
    ///
    /// The `sheet_animation_style` parameter is used to override the bottom sheet
    /// animation duration and reverse animation duration.
    ///
    /// If [`AnimationStyle::duration`] is provided, it will be used to override
    /// the bottom sheet animation duration in the underlying
    /// [`BottomSheet::create_animation_controller`].
    ///
    /// If [`AnimationStyle::reverse_duration`] is provided, it will be used to
    /// override the bottom sheet reverse animation duration in the underlying
    /// [`BottomSheet::create_animation_controller`].
    ///
    /// To disable the bottom sheet animation, use `AnimationStyle::no_animation()`.
    ///
    /// See also:
    ///
    ///  * [`BottomSheet`], which becomes the parent of the widget returned by the
    ///    `builder`.
    ///  * `show_bottom_sheet`, which calls this method given a [`BuildContext`].
    ///  * `show_modal_bottom_sheet`, which can be used to display a modal bottom
    ///    sheet.
    ///  * [`Scaffold::of`], for information about how to obtain the [`ScaffoldState`].
    ///  * The Material 2 spec at <https://m2.material.io/components/sheets-bottom>.
    ///  * The Material 3 spec at <https://m3.material.io/components/bottom-sheets/overview>.
    #[expect(
        clippy::too_many_arguments,
        reason = "Dart's named arguments, in Dart's order"
    )]
    pub fn show_bottom_sheet(
        self: Handle<Self>,
        app: &mut App,
        builder: WidgetBuilder,
        background_color: Option<Color>,
        elevation: Option<f64>,
        shape: Option<Box<dyn ShapeBorder>>,
        clip_behavior: Option<Clip>,
        constraints: Option<BoxConstraints>,
        enable_drag: Option<bool>,
        show_drag_handle: Option<bool>,
        transition_animation_controller: Option<Handle<AnimationController>>,
        sheet_animation_style: Option<AnimationStyle>,
    ) -> Rc<PersistentBottomSheetController> {
        debug_assert!(
            self.widget(app).bottom_sheet.is_none(),
            "Scaffold.bottomSheet cannot be specified while a bottom sheet \
             displayed with showBottomSheet() is still visible.\n\
             Rebuild the Scaffold with a null bottomSheet before calling showBottomSheet()."
        );
        debug_assert!({
            let context = self.context(app);
            waft_widgets::debug_check_has_media_query(app, context)
        });

        self.close_current_bottom_sheet(app);
        let controller = transition_animation_controller.unwrap_or_else(|| {
            BottomSheet::create_animation_controller(app, self, sheet_animation_style.as_ref())
        });
        controller.forward(app, None);
        let current_bottom_sheet = self.build_bottom_sheet(
            app,
            builder,
            false,
            controller,
            BottomSheetOptions {
                background_color,
                elevation,
                shape,
                clip_behavior,
                constraints,
                enable_drag,
                show_drag_handle,
                should_dispose_animation_controller: transition_animation_controller.is_none(),
            },
        );
        self.set_state(app, |state| {
            state.current_bottom_sheet = Some(current_bottom_sheet);
        });
        app.get(self)
            .current_bottom_sheet
            .clone()
            .expect("set above")
    }

    // Floating Action Button API

    fn floating_action_button_move_controller(
        self: Handle<Self>,
        app: &App,
    ) -> Handle<AnimationController> {
        app.get(self)
            .floating_action_button_move_controller
            .expect("made in init_state")
    }

    fn floating_action_button_visibility_controller(
        self: Handle<Self>,
        app: &App,
    ) -> Handle<AnimationController> {
        app.get(self)
            .floating_action_button_visibility_controller
            .expect("made in init_state")
    }

    fn floating_action_button_animator(
        self: Handle<Self>,
        app: &App,
    ) -> FloatingActionButtonAnimatorRef {
        Rc::clone(
            app.get(self)
                .floating_action_button_animator
                .as_ref()
                .expect("set in init_state"),
        )
    }

    fn show_floating_action_button(self: Handle<Self>, app: &mut App) -> TickerFuture {
        self.floating_action_button_visibility_controller(app)
            .forward(app, None)
    }

    // Moves the Floating Action Button to the new Floating Action Button Location.
    fn move_floating_action_button(
        self: Handle<Self>,
        app: &mut App,
        new_location: FloatingActionButtonLocationRef,
    ) {
        let mut previous_location = app.get(self).floating_action_button_location.clone();
        let mut restart_animation_from = 0.0;
        let move_controller = self.floating_action_button_move_controller(app);
        // If the Floating Action Button is moving right now, we need to start from a snapshot
        // of the current transition.
        if move_controller.is_animating(app) {
            let animator = self.floating_action_button_animator(app);
            let this = app.get(self);
            previous_location = Some(Rc::new(TransitionSnapshotFabLocation {
                begin: Rc::clone(
                    this.previous_floating_action_button_location
                        .as_ref()
                        .expect("set in init_state"),
                ),
                end: Rc::clone(
                    this.floating_action_button_location
                        .as_ref()
                        .expect("set in init_state"),
                ),
                animator: Rc::clone(&animator),
                progress: move_controller.value(app),
            }));
            restart_animation_from = animator.get_animation_restart(move_controller.value(app));
        }

        self.set_state(app, move |state| {
            state.previous_floating_action_button_location = previous_location;
            state.floating_action_button_location = Some(new_location);
        });

        // Animate the motion even when the fab is null so that if the exit animation is
        // running, the old fab will start the motion transition while it exits instead of
        // jumping to the new position.
        move_controller.forward(app, Some(restart_animation_from));
    }

    // iOS FEATURES - status bar tap, back gesture

    // On iOS, if `primary` is true, tapping the status bar scrolls the app's primary
    // scrollable to the top. We implement this by looking up the primary scroll controller
    // and scrolling it to the top when tapped.
    /// Dart's `WidgetsBindingObserver.handleStatusBarTap` override.
    pub fn handle_status_bar_tap(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.widget(app).primary);
        let context = self.context(app);
        let primary_scroll_controller = PrimaryScrollController::maybe_of(app, context);
        let status_bar_key = app.get(self).status_bar_key.clone();
        if let Some(primary_scroll_controller) = primary_scroll_controller
            && primary_scroll_controller.has_clients(app)
            // TODO(LongCatIsLooong): the iOS embedder used to send status bar tap
            // evets as fake touches at Offset.zero, such that at most one Scaffold
            // (usually the foreground primary Scaffold) can handle the status bar
            // tap event, thanks to hit-testing and gesture disambiguation.
            // To keep that behavior, this widget performs an additional hit-test here
            // to make sure the status bar tap is only handled if the scaffold is
            // hit-testable (thus in the foreground)
            // Switch to a better solution when available:
            // https://github.com/flutter/flutter/issues/182403
            && HitTestableAtOrigin::hit_testable_at_origin(app, &status_bar_key)
        {
            primary_scroll_controller.animate_to(
                app,
                0.0,
                Duration::from_millis(1000),
                Curves::ease_out_circ(),
            );
        }
    }

    // INTERNALS

    fn geometry_notifier(self: Handle<Self>, app: &App) -> Handle<ScaffoldGeometryNotifier> {
        app.get(self).geometry_notifier.expect("made in init_state")
    }

    fn resize_to_avoid_bottom_inset(self: Handle<Self>, app: &App) -> bool {
        self.widget(app)
            .resize_to_avoid_bottom_inset
            .unwrap_or(true)
    }

    fn observer(self: Handle<Self>, app: &mut App) -> WidgetsBindingObserverRef {
        if let Some(observer) = app.get(self).observer.clone() {
            return observer;
        }
        let observer: WidgetsBindingObserverRef = Rc::new(self);
        app.get_mut(self).observer = Some(observer.clone());
        observer
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "Dart's named arguments, in Dart's order"
    )]
    fn add_if_non_null(
        self: Handle<Self>,
        app: &mut App,
        children: &mut Vec<WidgetRef>,
        child: Option<WidgetRef>,
        child_id: ScaffoldSlot,
        remove_left_padding: bool,
        remove_top_padding: bool,
        remove_right_padding: bool,
        remove_bottom_padding: bool,
        remove_bottom_inset: bool,
        maintain_bottom_view_padding: bool,
    ) {
        let context = self.context(app);
        let mut data = MediaQuery::of(app, context).remove_padding(
            remove_left_padding,
            remove_top_padding,
            remove_right_padding,
            remove_bottom_padding,
        );
        if remove_bottom_inset {
            data = data.remove_view_insets(false, false, false, true);
        }

        if maintain_bottom_view_padding && data.view_insets.bottom != 0.0 {
            let padding = data
                .padding
                .copy_with(None, None, None, Some(data.view_padding.bottom));
            data = data.copy_with().padding(padding);
        }

        if let Some(child) = child {
            children.push(
                LayoutId::new(child_id.id(), MediaQuery::new(data, child).into_widget())
                    .into_widget(),
            );
        }
    }

    fn build_end_drawer(
        self: Handle<Self>,
        app: &mut App,
        children: &mut Vec<WidgetRef>,
        text_direction: TextDirection,
    ) {
        if let Some(end_drawer) = self.widget(app).end_drawer.clone() {
            debug_assert!(self.has_end_drawer(app));
            let widget = self.widget(app);
            let mut controller = DrawerController::new(end_drawer, DrawerAlignment::End)
                .key(app.get(self).end_drawer_key.clone())
                .drawer_callback(Rc::new(move |app: &mut App, is_opened: bool| {
                    self.end_drawer_opened_callback(app, is_opened)
                }))
                .drag_start_behavior(widget.drawer_drag_start_behavior)
                .enable_open_drag_gesture(widget.end_drawer_enable_open_drag_gesture)
                .drawer_barrier_dismissible(widget.drawer_barrier_dismissible);
            if let Some(scrim_color) = widget.drawer_scrim_color {
                controller = controller.scrim_color(scrim_color);
            }
            if let Some(edge_drag_width) = widget.drawer_edge_drag_width {
                controller = controller.edge_drag_width(edge_drag_width);
            }
            let controller = controller.is_drawer_open(self.is_end_drawer_open(app));
            self.add_if_non_null(
                app,
                children,
                Some(controller.into_widget()),
                ScaffoldSlot::EndDrawer,
                // remove the side padding from the side we're not touching
                text_direction == TextDirection::Ltr,
                false,
                text_direction == TextDirection::Rtl,
                false,
                false,
                false,
            );
        }
    }

    fn build_drawer(
        self: Handle<Self>,
        app: &mut App,
        children: &mut Vec<WidgetRef>,
        text_direction: TextDirection,
    ) {
        if let Some(drawer) = self.widget(app).drawer.clone() {
            debug_assert!(self.has_drawer(app));
            let widget = self.widget(app);
            let mut controller = DrawerController::new(drawer, DrawerAlignment::Start)
                .key(app.get(self).drawer_key.clone())
                .drawer_callback(Rc::new(move |app: &mut App, is_opened: bool| {
                    self.drawer_opened_callback(app, is_opened)
                }))
                .drag_start_behavior(widget.drawer_drag_start_behavior)
                .enable_open_drag_gesture(widget.drawer_enable_open_drag_gesture)
                .drawer_barrier_dismissible(widget.drawer_barrier_dismissible);
            if let Some(scrim_color) = widget.drawer_scrim_color {
                controller = controller.scrim_color(scrim_color);
            }
            if let Some(edge_drag_width) = widget.drawer_edge_drag_width {
                controller = controller.edge_drag_width(edge_drag_width);
            }
            let controller = controller.is_drawer_open(self.is_drawer_open(app));
            self.add_if_non_null(
                app,
                children,
                Some(controller.into_widget()),
                ScaffoldSlot::Drawer,
                // remove the side padding from the side we're not touching
                text_direction == TextDirection::Rtl,
                false,
                text_direction == TextDirection::Ltr,
                false,
                false,
                false,
            );
        }
    }

    fn bottom_sheet_scrim_animation_controller(
        self: Handle<Self>,
        app: &App,
    ) -> Handle<AnimationController> {
        app.get(self)
            .bottom_sheet_scrim_animation_controller
            .expect("made in init_state")
    }

    /// Whether to show a [`ModalBarrier`] over the body of the scaffold.
    ///
    /// The `value` parameter must not be null.
    pub fn show_body_scrim(self: Handle<Self>, app: &mut App, value: bool, animation_value: f64) {
        if app.get(self).show_body_scrim != value {
            self.set_state(app, |state| {
                state.show_body_scrim = value;
            });
        }
        let controller = self.bottom_sheet_scrim_animation_controller(app);
        if controller.value(app) != animation_value {
            controller.set_value(app, animation_value);
        }
    }
}

/// The optional arguments [`ScaffoldState::build_bottom_sheet`] passes on to the sheet.
#[derive(Default)]
struct BottomSheetOptions {
    background_color: Option<Color>,
    elevation: Option<f64>,
    shape: Option<Box<dyn ShapeBorder>>,
    clip_behavior: Option<Clip>,
    constraints: Option<BoxConstraints>,
    enable_drag: Option<bool>,
    show_drag_handle: Option<bool>,
    /// Dart's `shouldDisposeAnimationController = true`.
    should_dispose_animation_controller: bool,
}

impl TickerProviderStateMixin for ScaffoldState {
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
impl TickerProviderObject for ScaffoldState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl RestorationMixin for ScaffoldState {
    waft_widgets::restoration_mixin_accessors!();

    fn restoration_id(self: Handle<Self>, app: &App) -> Option<&str> {
        self.widget(app).restoration_id.as_deref()
    }

    fn restore_state(
        self: Handle<Self>,
        app: &mut App,
        _old_bucket: Option<Handle<RestorationBucket>>,
        _initial_restore: bool,
    ) {
        let drawer_opened = self.drawer_opened(app);
        self.register_for_restoration(app, drawer_opened.as_property(), "drawer_open");
        let end_drawer_opened = self.end_drawer_opened(app);
        self.register_for_restoration(app, end_drawer_opened.as_property(), "end_drawer_open");
    }
}

impl WidgetsBindingObserverObject for ScaffoldState {}

impl State for ScaffoldState {
    type Widget = Scaffold;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let drawer_opened = RestorableBool::new(app, false);
        let end_drawer_opened = RestorableBool::new(app, false);
        let context = self.context(app);
        let geometry_notifier =
            ScaffoldGeometryNotifier::new(app, ScaffoldGeometry::new(), context);
        let floating_action_button_location = self
            .widget(app)
            .floating_action_button_location
            .clone()
            .unwrap_or_else(default_floating_action_button_location);
        let floating_action_button_animator = self
            .widget(app)
            .floating_action_button_animator
            .clone()
            .unwrap_or_else(default_floating_action_button_animator);
        let floating_action_button_move_controller = AnimationController::create(
            app,
            Some(1.0),
            Some(K_FLOATING_ACTION_BUTTON_SEGUE * 2),
            None,
            0.0,
            1.0,
            waft_animation::AnimationBehavior::Normal,
            self,
        );

        let floating_action_button_visibility_controller = AnimationController::create(
            app,
            None,
            Some(K_FLOATING_ACTION_BUTTON_SEGUE),
            None,
            0.0,
            1.0,
            waft_animation::AnimationBehavior::Normal,
            self,
        );

        let bottom_sheet_scrim_animation_controller = AnimationController::create(
            app,
            None,
            None,
            None,
            0.0,
            1.0,
            waft_animation::AnimationBehavior::Normal,
            self,
        );
        let dismiss_drawer_action = DismissDrawerAction::new(app, context);
        let this = app.get_mut(self);
        this.dismiss_drawer_action = Some(dismiss_drawer_action);
        this.drawer_opened = Some(drawer_opened);
        this.end_drawer_opened = Some(end_drawer_opened);
        this.geometry_notifier = Some(geometry_notifier);
        this.previous_floating_action_button_location =
            Some(Rc::clone(&floating_action_button_location));
        this.floating_action_button_location = Some(floating_action_button_location);
        this.floating_action_button_animator = Some(floating_action_button_animator);
        this.floating_action_button_move_controller = Some(floating_action_button_move_controller);
        this.floating_action_button_visibility_controller =
            Some(floating_action_button_visibility_controller);
        this.bottom_sheet_scrim_animation_controller =
            Some(bottom_sheet_scrim_animation_controller);
        if self.widget(app).primary {
            let observer = self.observer(app);
            WidgetsBinding::instance(app).add_observer(app, observer);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &Scaffold) {
        self.did_update_restoration_id(app);
        // Update the Floating Action Button Animator, and then schedule the Floating Action
        // Button for repositioning.
        if !option_rc_ptr_eq(
            &self.widget(app).floating_action_button_animator,
            &old_widget.floating_action_button_animator,
        ) {
            let animator = self
                .widget(app)
                .floating_action_button_animator
                .clone()
                .unwrap_or_else(default_floating_action_button_animator);
            app.get_mut(self).floating_action_button_animator = Some(animator);
        }
        if !option_rc_ptr_eq(
            &self.widget(app).floating_action_button_location,
            &old_widget.floating_action_button_location,
        ) {
            let location = self
                .widget(app)
                .floating_action_button_location
                .clone()
                .unwrap_or_else(default_floating_action_button_location);
            self.move_floating_action_button(app, location);
        }
        if !option_rc_ptr_eq(&self.widget(app).bottom_sheet, &old_widget.bottom_sheet) {
            debug_assert!(
                !(self.widget(app).bottom_sheet.is_some()
                    && app
                        .get(self)
                        .current_bottom_sheet
                        .as_ref()
                        .is_some_and(|sheet| sheet.is_local_history_entry)),
                "Scaffold.bottomSheet cannot be specified while a bottom sheet displayed \
                 with showBottomSheet() is still visible.\n\
                 Use the PersistentBottomSheetController \
                 returned by showBottomSheet() to close the old bottom sheet before creating \
                 a Scaffold with a (non null) bottomSheet."
            );
            if self.widget(app).bottom_sheet.is_none() {
                self.close_current_bottom_sheet(app);
            } else if self.widget(app).bottom_sheet.is_some() && old_widget.bottom_sheet.is_none() {
                self.maybe_build_persistent_bottom_sheet(app);
            } else {
                self.update_persistent_bottom_sheet(app);
            }
        }
        match (old_widget.primary, self.widget(app).primary) {
            (true, false) => {
                let observer = self.observer(app);
                WidgetsBinding::instance(app).remove_observer(app, &observer);
            }
            (false, true) => {
                let observer = self.observer(app);
                WidgetsBinding::instance(app).add_observer(app, observer);
            }
            (true, true) | (false, false) => {}
        }
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        // Using maybeOf is valid here since both the Scaffold and ScaffoldMessenger
        // are currently available for managing SnackBars.
        let context = self.context(app);
        let current_scaffold_messenger = ScaffoldMessenger::maybe_of(app, context);
        // If our ScaffoldMessenger has changed, unregister with the old one first.
        if let Some(scaffold_messenger) = app.get(self).scaffold_messenger
            && current_scaffold_messenger != Some(scaffold_messenger)
        {
            scaffold_messenger.unregister(app, self);
        }
        // Register with the current ScaffoldMessenger, if there is one.
        app.get_mut(self).scaffold_messenger = current_scaffold_messenger;
        if let Some(scaffold_messenger) = current_scaffold_messenger {
            scaffold_messenger.register(app, self);
        }

        self.maybe_build_persistent_bottom_sheet(app);
        self.did_change_dependencies_restoration(app);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        let observer = self.observer(app);
        WidgetsBinding::instance(app).remove_observer(app, &observer);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
        if self.widget(app).primary {
            let observer = self.observer(app);
            WidgetsBinding::instance(app).add_observer(app, observer);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.geometry_notifier(app).dispose(app);
        self.floating_action_button_move_controller(app)
            .dispose(app);
        self.floating_action_button_visibility_controller(app)
            .dispose(app);
        if let Some(scaffold_messenger) = app.get(self).scaffold_messenger {
            scaffold_messenger.unregister(app, self);
        }
        RestorableProperty::dispose(self.drawer_opened(app), app);
        RestorableProperty::dispose(self.end_drawer_opened(app), app);
        self.bottom_sheet_scrim_animation_controller(app)
            .dispose(app);
        let dismiss_drawer_action = app
            .get(self)
            .dismiss_drawer_action
            .expect("set in init_state");
        destroy_lingering(app, dismiss_drawer_action);
        self.dispose_restoration(app);
        TickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(waft_widgets::debug_check_has_media_query(app, context));
        debug_assert!(waft_widgets::debug_check_has_directionality(
            app, context, None, None, None
        ));
        let theme_data = Theme::of(app, context);
        let text_direction = waft_widgets::Directionality::of(app, context);

        let mut children = Vec::new();
        let body = self.widget(app).body.clone().map(|body| {
            BodyBuilder::new(
                self.widget(app).extend_body,
                self.widget(app).extend_body_behind_app_bar,
                KeyedSubtree::new(body)
                    .key(app.get(self).body_key.clone().into())
                    .into_widget(),
            )
            .into_widget()
        });
        let remove_bottom_padding = self.widget(app).bottom_navigation_bar.is_some()
            || self.widget(app).persistent_footer_buttons.is_some();
        let resize_to_avoid_bottom_inset = self.resize_to_avoid_bottom_inset(app);
        let has_app_bar = self.widget(app).app_bar.is_some();
        self.add_if_non_null(
            app,
            &mut children,
            body,
            ScaffoldSlot::Body,
            false,
            has_app_bar,
            false,
            remove_bottom_padding,
            resize_to_avoid_bottom_inset,
            false,
        );
        if app.get(self).show_body_scrim {
            let scrim_builder = Rc::clone(&self.widget(app).bottom_sheet_scrim_builder);
            let scrim_animation = self.bottom_sheet_scrim_animation_controller(app).view();
            let scrim = scrim_builder(app, context, scrim_animation);
            self.add_if_non_null(
                app,
                &mut children,
                scrim,
                ScaffoldSlot::BodyScrim,
                true,
                true,
                true,
                true,
                false,
                false,
            );
        }

        if let Some(app_bar) = self.widget(app).app_bar.clone() {
            let top_padding = if self.widget(app).primary {
                MediaQuery::padding_of(app, context).top
            } else {
                0.0
            };
            let app_bar_max_height =
                AppBar::preferred_height_for(app, context, &app_bar.preferred_size()) + top_padding;
            app.get_mut(self).app_bar_max_height = Some(app_bar_max_height);
            debug_assert!(app_bar_max_height >= 0.0 && app_bar_max_height.is_finite());
            self.add_if_non_null(
                app,
                &mut children,
                Some(
                    ConstrainedBox::new(BoxConstraints::new().max_height(app_bar_max_height))
                        .child(
                            FlexibleSpaceBar::create_settings(app_bar_max_height, app_bar).build(),
                        )
                        .into_widget(),
                ),
                ScaffoldSlot::AppBar,
                false,
                false,
                false,
                true,
                false,
                false,
            );
        }

        let mut is_snack_bar_floating = false;
        let mut snack_bar_width = None;

        if app.get(self).current_bottom_sheet.is_some()
            || !app.get(self).dismissed_bottom_sheets.is_empty()
        {
            let this = app.get(self);
            let mut sheets = this.dismissed_bottom_sheets.to_vec();
            if let Some(current_bottom_sheet) = &this.current_bottom_sheet {
                sheets.push(current_bottom_sheet.feature.widget.clone());
            }
            let stack = LayoutBuilder::new(move |_app, _context, constraints| {
                BottomSheetKeyboardInset {
                    inset: constraints
                        .extension::<BottomSheetBoxConstraints>()
                        .expect("the scaffold lays out its bottom sheet with _BottomSheetBoxConstraints")
                        .keyboard_inset,
                    child: Stack::new()
                        .alignment(AlignmentGeometry::BOTTOM_CENTER)
                        .children(sheets.clone())
                        .into_widget(),
                }
                .into_widget()
            })
            .into_widget();
            self.add_if_non_null(
                app,
                &mut children,
                Some(stack),
                ScaffoldSlot::BottomSheet,
                false,
                true,
                false,
                resize_to_avoid_bottom_inset,
                false,
                false,
            );
        }

        // SnackBar set by ScaffoldMessenger
        if let Some(messenger_snack_bar) = app.get(self).messenger_snack_bar.clone() {
            let snack_bar_theme = SnackBarTheme::of(app, context);
            let snack_bar_behavior = messenger_snack_bar
                .widget()
                .behavior
                .or(snack_bar_theme.behavior)
                .unwrap_or(SnackBarBehavior::Fixed);
            is_snack_bar_floating = snack_bar_behavior == SnackBarBehavior::Floating;
            snack_bar_width = messenger_snack_bar.widget().width.or(snack_bar_theme.width);

            self.add_if_non_null(
                app,
                &mut children,
                Some(messenger_snack_bar.widget.clone()),
                ScaffoldSlot::SnackBar,
                false,
                true,
                false,
                remove_bottom_padding,
                false,
                !resize_to_avoid_bottom_inset,
            );
        }

        let mut extend_body_behind_material_banner = false;
        // MaterialBanner set by ScaffoldMessenger
        if let Some(messenger_material_banner) = app.get(self).messenger_material_banner.clone() {
            let banner_theme = MaterialBannerTheme::of(app, context);
            let elevation = messenger_material_banner
                .widget()
                .elevation
                .or(banner_theme.elevation)
                .unwrap_or(0.0);
            extend_body_behind_material_banner = elevation != 0.0;

            self.add_if_non_null(
                app,
                &mut children,
                Some(messenger_material_banner.widget.clone()),
                ScaffoldSlot::MaterialBanner,
                false,
                has_app_bar,
                false,
                true,
                false,
                !resize_to_avoid_bottom_inset,
            );
        }

        if let Some(persistent_footer_buttons) = self.widget(app).persistent_footer_buttons.clone()
        {
            let decoration = match self.widget(app).persistent_footer_decoration.clone() {
                Some(decoration) => decoration,
                None => BoxDecoration::new().border(Border {
                    top: Divider::create_border_side(app, Some(context), None, Some(1.0)),
                    ..Border::default()
                }),
            };
            let footer = Container::new()
                .decoration(decoration)
                .child(
                    SafeArea::new(
                        IntrinsicHeight::new().child(
                            Padding::new(EdgeInsetsGeometry::all(8.0)).child(
                                Align::new()
                                    .alignment(self.widget(app).persistent_footer_alignment.into())
                                    .child(
                                        OverflowBar::new()
                                            .spacing(8.0)
                                            .overflow_alignment(OverflowBarAlignment::End)
                                            .children(persistent_footer_buttons),
                                    ),
                            ),
                        ),
                    )
                    .top(false),
                )
                .into_widget();
            let remove_bottom_padding = self.widget(app).bottom_navigation_bar.is_some();
            self.add_if_non_null(
                app,
                &mut children,
                Some(footer),
                ScaffoldSlot::PersistentFooter,
                false,
                true,
                false,
                remove_bottom_padding,
                false,
                !resize_to_avoid_bottom_inset,
            );
        }

        if let Some(bottom_navigation_bar) = self.widget(app).bottom_navigation_bar.clone() {
            self.add_if_non_null(
                app,
                &mut children,
                Some(bottom_navigation_bar),
                ScaffoldSlot::BottomNavigationBar,
                false,
                true,
                false,
                false,
                false,
                !resize_to_avoid_bottom_inset,
            );
        }

        let fab_transition = FloatingActionButtonTransition {
            child: self.widget(app).floating_action_button.clone(),
            fab_move_animation: self.floating_action_button_move_controller(app).view(),
            fab_motion_animator: self.floating_action_button_animator(app),
            geometry_notifier: self.geometry_notifier(app),
            current_controller: self.floating_action_button_visibility_controller(app),
        };
        self.add_if_non_null(
            app,
            &mut children,
            Some(fab_transition.into_widget()),
            ScaffoldSlot::FloatingActionButton,
            true,
            true,
            true,
            true,
            false,
            false,
        );

        let status_bar = match theme_data.platform {
            TargetPlatform::IOS | TargetPlatform::MacOS => self.widget(app).primary.then(|| {
                HitTestableAtOrigin::new(app.get(self).status_bar_key.clone()).into_widget()
            }),
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => None,
        };

        self.add_if_non_null(
            app,
            &mut children,
            status_bar,
            ScaffoldSlot::StatusBar,
            false,
            true,
            false,
            true,
            false,
            false,
        );

        if self.is_end_drawer_open(app) {
            self.build_drawer(app, &mut children, text_direction);
            self.build_end_drawer(app, &mut children, text_direction);
        } else {
            self.build_end_drawer(app, &mut children, text_direction);
            self.build_drawer(app, &mut children, text_direction);
        }

        // The minimum insets for contents of the Scaffold to keep visible.
        let view_insets_bottom = MediaQuery::view_insets_of(app, context).bottom;
        let min_insets = MediaQuery::padding_of(app, context).copy_with(
            None,
            None,
            None,
            Some(if resize_to_avoid_bottom_inset {
                view_insets_bottom
            } else {
                0.0
            }),
        );

        // The minimum viewPadding for interactive elements positioned by the
        // Scaffold to keep within safe interactive areas.
        let min_view_padding = MediaQuery::view_padding_of(app, context).copy_with(
            None,
            None,
            None,
            (resize_to_avoid_bottom_inset && view_insets_bottom != 0.0).then_some(0.0),
        );

        let move_animation = self.floating_action_button_move_controller(app).view();
        let extend_body = self.widget(app).extend_body;
        let extend_body_behind_app_bar = self.widget(app).extend_body_behind_app_bar;
        let this = app.get(self);
        let delegate = Rc::new(ScaffoldLayout {
            extend_body,
            extend_body_behind_app_bar,
            min_insets,
            min_view_padding,
            current_floating_action_button_location: Rc::clone(
                this.floating_action_button_location
                    .as_ref()
                    .expect("set in init_state"),
            ),
            floating_action_button_move_animation: move_animation,
            relayout: Rc::new(move_animation),
            floating_action_button_motion_animator: Rc::clone(
                this.floating_action_button_animator
                    .as_ref()
                    .expect("set in init_state"),
            ),
            geometry_notifier: this.geometry_notifier.expect("made in init_state"),
            previous_floating_action_button_location: Rc::clone(
                this.previous_floating_action_button_location
                    .as_ref()
                    .expect("set in init_state"),
            ),
            text_direction,
            is_snack_bar_floating,
            extend_body_behind_material_banner,
            snack_bar_width,
        });

        let dismiss_drawer_action = app
            .get(self)
            .dismiss_drawer_action
            .expect("set in init_state");
        ScaffoldScope {
            has_drawer: self.has_drawer(app),
            has_end_drawer: self.has_end_drawer(app),
            is_drawer_open: self.is_drawer_open(app),
            is_end_drawer_open: self.is_end_drawer_open(app),
            geometry_notifier: self.geometry_notifier(app),
            child: ScrollNotificationObserver::new(
                Material::new()
                    .color(
                        self.widget(app)
                            .background_color
                            .unwrap_or(theme_data.scaffold_background_color),
                    )
                    .child(Builder::new(move |app, context| {
                        app.get_mut(dismiss_drawer_action).context = context;
                        let mut actions = HashMap::new();
                        actions.insert(
                            <DismissDrawerAction as Action>::intent_type(),
                            Action::as_action(dismiss_drawer_action),
                        );
                        Actions::new(
                            actions,
                            CustomMultiChildLayout::new(delegate.clone())
                                .children(children.clone()),
                        )
                        .into_widget()
                    })),
            )
            .into_widget(),
        }
        .into_widget()
    }
}

/// Whether a panic is the framework's `setState` during build: Dart's
/// `exception is FlutterError` whose summary is 'setState() or markNeedsBuild() called during
/// build.'.
fn is_set_state_during_build(exception: &(dyn Any + Send)) -> bool {
    let message = exception
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| exception.downcast_ref::<String>().map(String::as_str));
    message.is_some_and(|message| {
        message.starts_with("setState() or markNeedsBuild() called during build.")
    })
}

fn option_rc_ptr_eq<T: ?Sized>(a: &Option<Rc<T>>, b: &Option<Rc<T>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

/// Dart's `_DismissDrawerAction`.
struct DismissDrawerAction {
    action: ActionData,
    /// The context of the scaffold's `Builder`, which sets it on every build; until the first,
    /// the scaffold's own, which nothing can invoke the action from.
    context: BuildContext,
}

impl DismissDrawerAction {
    fn new(app: &mut App, context: BuildContext) -> Handle<DismissDrawerAction> {
        app.create(DismissDrawerAction {
            action: ActionData::new(),
            context,
        })
    }
}

impl Action for DismissDrawerAction {
    type Intent = DismissIntent;

    waft_widgets::action_accessors!();

    fn is_enabled(self: Handle<Self>, app: &mut App, _intent: &DismissIntent) -> bool {
        let context = app.get(self).context;
        let scaffold = Scaffold::of(app, context);
        (scaffold.is_drawer_open(app) || scaffold.is_end_drawer_open(app))
            && scaffold.is_drawer_barrier_dismissible(app)
    }

    fn invoke(self: Handle<Self>, app: &mut App, intent: &DismissIntent) -> Option<Rc<dyn Any>> {
        let context = app.get(self).context;
        let scaffold = Scaffold::of(app, context);
        if Action::is_enabled(self, app, intent) {
            scaffold.close_drawer(app);
            scaffold.close_end_drawer(app);
        }
        None
    }
}

impl DismissAction for DismissDrawerAction {}

/// An interface for controlling a feature of a [`Scaffold`].
///
/// Commonly obtained from [`ScaffoldMessengerState::show_snack_bar`] or
/// [`ScaffoldState::show_bottom_sheet`].
///
/// Dart compares controllers by identity; here they are shared as `Rc`s and compared with
/// `Rc::ptr_eq`.
pub struct ScaffoldFeatureController<T, U> {
    /// Dart's `_widget`, as the tree node the scaffold places.
    widget: WidgetRef,
    completer: Completer<U>,

    /// Remove the feature (e.g., bottom sheet, snack bar, or material banner) from the
    /// scaffold.
    pub close: VoidCallback,

    /// Mark the feature (e.g., bottom sheet or snack bar) as needing to rebuild.
    pub set_state: Option<StateSetter>,

    _widget_type: PhantomData<T>,
}

impl<T: 'static, U: Clone + 'static> ScaffoldFeatureController<T, U> {
    fn new<K>(
        widget: impl IntoWidget<K>,
        completer: Completer<U>,
        close: VoidCallback,
        set_state: Option<StateSetter>,
    ) -> ScaffoldFeatureController<T, U> {
        ScaffoldFeatureController {
            widget: widget.into_widget(),
            completer,
            close,
            set_state,
            _widget_type: PhantomData,
        }
    }

    /// Dart's `_widget`.
    fn widget(&self) -> &T {
        downcast_widget::<T>(&*self.widget).expect("the controller's widget keeps its type")
    }

    /// Completes when the feature controlled by this object is no longer visible.
    pub fn closed(&self) -> CompleterFuture<U> {
        self.completer.future()
    }
}

impl<T, U> Debug for ScaffoldFeatureController<T, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ScaffoldFeatureController")
            .field("widget", &self.widget)
            .finish_non_exhaustive()
    }
}

/// Dart's `_StandardBottomSheet`.
struct StandardBottomSheet {
    key: KeyRef,
    // we control it, but it must be disposed by whoever created it.
    animation_controller: Handle<AnimationController>,
    enable_drag: bool,
    show_drag_handle: Option<bool>,
    on_closing: Option<VoidCallback>,
    on_dismissed: Option<VoidCallback>,
    on_dispose: Option<VoidCallback>,
    builder: WidgetBuilder,
    is_persistent: bool,
    background_color: Option<Color>,
    elevation: Option<f64>,
    shape: Option<Box<dyn ShapeBorder>>,
    clip_behavior: Option<Clip>,
    constraints: Option<BoxConstraints>,
}

impl Debug for StandardBottomSheet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_StandardBottomSheet")
            .field("enableDrag", &self.enable_drag)
            .field("isPersistent", &self.is_persistent)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for StandardBottomSheet {
    type State = StandardBottomSheetState;

    fn key(&self) -> Option<&KeyRef> {
        Some(&self.key)
    }

    fn create_state(&self) -> StandardBottomSheetState {
        StandardBottomSheetState {
            state: StateData::new(),
            animation_curve: standard_bottom_sheet_curve(),
        }
    }
}

/// Dart's `_StandardBottomSheetState`.
struct StandardBottomSheetState {
    state: StateData<StandardBottomSheet>,
    animation_curve: Rc<dyn Curve>,
}

impl StandardBottomSheetState {
    fn close(self: Handle<Self>, app: &mut App) {
        let animation_controller = self.widget(app).animation_controller;
        let on_closing = self.widget(app).on_closing.clone();
        animation_controller.reverse(app, None);
        if let Some(on_closing) = on_closing {
            on_closing(app);
        }
    }

    fn handle_drag_start(self: Handle<Self>, app: &mut App, _details: DragStartDetails) {
        // Allow the bottom sheet to track the user's finger accurately.
        app.get_mut(self).animation_curve = Curves::linear();
    }

    fn handle_drag_end(
        self: Handle<Self>,
        app: &mut App,
        _details: DragEndDetails,
        _is_closing: bool,
    ) {
        // Allow the bottom sheet to animate smoothly from its current position.
        let value = self.widget(app).animation_controller.value(app);
        app.get_mut(self).animation_curve = Rc::new(Split::new(
            value,
            Curves::linear(),
            standard_bottom_sheet_curve(),
        ));
    }

    fn handle_status_change(self: Handle<Self>, app: &mut App, status: AnimationStatus) {
        if status.is_dismissed()
            && let Some(on_dismissed) = self.widget(app).on_dismissed.clone()
        {
            on_dismissed(app);
        }
    }

    fn extent_changed(
        self: Handle<Self>,
        app: &mut App,
        notification: &DraggableScrollableNotification,
    ) -> bool {
        let extent_remaining = 1.0 - notification.extent;
        let context = self.context(app);
        let scaffold = Scaffold::of(app, context);
        if extent_remaining < K_BOTTOM_SHEET_DOMINATES_PERCENTAGE {
            scaffold
                .floating_action_button_visibility_controller(app)
                .set_value(
                    app,
                    extent_remaining * K_BOTTOM_SHEET_DOMINATES_PERCENTAGE * 10.0,
                );

            let scrim_animation_value =
                1.0 - extent_remaining / K_BOTTOM_SHEET_DOMINATES_PERCENTAGE;
            scaffold.show_body_scrim(app, true, scrim_animation_value);
        } else {
            scaffold
                .floating_action_button_visibility_controller(app)
                .set_value(app, 1.0);
            scaffold.show_body_scrim(app, false, 0.0);
        }
        // If the Scaffold.bottomSheet != null, we're a persistent bottom sheet.
        if (notification.extent - notification.min_extent).abs() < PRECISION_ERROR_TOLERANCE
            && scaffold.widget(app).bottom_sheet.is_none()
            && notification.should_close_on_min_extent
        {
            self.close(app);
        }
        false
    }
}

impl State for StandardBottomSheetState {
    type Widget = StandardBottomSheet;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let animation_controller = self.widget(app).animation_controller;
        debug_assert!(animation_controller.is_forward_or_completed(app));
        animation_controller.add_status_listener(
            app,
            AnimationStatusListener::handle_method(
                self,
                StandardBottomSheetState::handle_status_change,
            ),
        );
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let animation_controller = self.widget(app).animation_controller;
        let on_dispose = self.widget(app).on_dispose.clone();
        animation_controller.remove_status_listener(
            app,
            &AnimationStatusListener::handle_method(
                self,
                StandardBottomSheetState::handle_status_change,
            ),
        );
        if let Some(on_dispose) = on_dispose {
            on_dispose(app);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &StandardBottomSheet) {
        debug_assert!(self.widget(app).animation_controller == old_widget.animation_controller);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        // Non-zero only while the keyboard lifts this sheet, see _ScaffoldLayout.
        let keyboard_inset = BottomSheetKeyboardInset::of(app, context);
        // The constraints describe the content, so they grow by the part of the
        // Material that extends behind the keyboard. This mirrors BottomSheet's own
        // lookup; its defaults, used when both are null, don't limit the height.
        let theme_constraints = Theme::of(app, context).bottom_sheet_theme.constraints;
        let widget = self.widget(app);
        let constraints = widget.constraints.or(theme_constraints);

        let animation_controller = widget.animation_controller;
        let is_persistent = widget.is_persistent;
        let builder = Rc::clone(&widget.builder);
        let mut bottom_sheet = BottomSheet::new(
            widget
                .on_closing
                .clone()
                .expect("a standard bottom sheet has an onClosing"),
            Rc::new(move |app: &mut App, context: BuildContext| {
                // Extends the Material behind the keyboard. Always present, so the
                // content keeps its state when the keyboard shows or hides.
                Padding::new(EdgeInsetsGeometry::only(0.0, 0.0, 0.0, keyboard_inset))
                    .child(builder(app, context))
                    .into_widget()
            }),
        )
        .animation_controller(animation_controller)
        .enable_drag(widget.enable_drag)
        .on_drag_start(move |app: &mut App, details: DragStartDetails| {
            self.handle_drag_start(app, details)
        })
        .on_drag_end(
            move |app: &mut App, details: DragEndDetails, is_closing: bool| {
                self.handle_drag_end(app, details, is_closing)
            },
        );
        if let Some(show_drag_handle) = widget.show_drag_handle {
            bottom_sheet = bottom_sheet.show_drag_handle(show_drag_handle);
        }
        if let Some(background_color) = widget.background_color {
            bottom_sheet = bottom_sheet.background_color(background_color);
        }
        if let Some(elevation) = widget.elevation {
            bottom_sheet = bottom_sheet.elevation(elevation);
        }
        if let Some(shape) = widget.shape.clone() {
            bottom_sheet = bottom_sheet.shape(shape);
        }
        if let Some(clip_behavior) = widget.clip_behavior {
            bottom_sheet = bottom_sheet.clip_behavior(clip_behavior);
        }
        if let Some(constraints) = constraints {
            bottom_sheet = bottom_sheet.constraints(
                constraints
                    .min_height(constraints.min_height + keyboard_inset)
                    .max_height(constraints.max_height + keyboard_inset),
            );
        }

        ListenableBuilder::new(
            Rc::new(animation_controller.view()),
            move |app, _context, child| {
                let animation_curve = Rc::clone(&app.get(self).animation_curve);
                let mut align = Align::new()
                    .alignment(AlignmentDirectional::TOP_START.into())
                    .height_factor(animation_curve.transform(animation_controller.value(app)));
                if let Some(child) = child {
                    align = align.child(child.clone());
                }
                align.into_widget()
            },
        )
        .child({
            let mut semantics = Semantics::new().container(true);
            if !is_persistent {
                semantics = semantics.on_dismiss(Rc::new(move |app: &mut App| self.close(app)));
            }
            semantics.child(
                NotificationListener::<DraggableScrollableNotification>::new(bottom_sheet)
                    .on_notification(move |app, notification| {
                        self.extent_changed(app, notification)
                    })
                    .into_widget(),
            )
        })
        .into_widget()
    }
}

/// A [`ScaffoldFeatureController`] for standard bottom sheets.
///
/// This is the type of objects returned by [`ScaffoldState::show_bottom_sheet`].
///
/// This controller is used to display both standard and persistent bottom
/// sheets. A bottom sheet is only persistent if it is set as the
/// [`Scaffold::bottom_sheet`].
pub struct PersistentBottomSheetController {
    /// The [`ScaffoldFeatureController`] this controller extends, over Dart's private
    /// `_StandardBottomSheet`; [`closed`](Self::closed), [`close`](Self::close) and
    /// [`set_state`](Self::set_state) reach it.
    feature: ScaffoldFeatureController<StandardBottomSheet, ()>,
    is_local_history_entry: bool,
}

impl PersistentBottomSheetController {
    /// Completes when the bottom sheet is no longer visible.
    pub fn closed(&self) -> CompleterFuture<()> {
        self.feature.completer.future()
    }

    /// Removes the bottom sheet from the scaffold.
    pub fn close(&self, app: &mut App) {
        (self.feature.close)(app);
    }

    /// Marks the bottom sheet as needing to rebuild, after running `f`.
    pub fn set_state(&self, app: &mut App, f: &mut dyn FnMut(&mut App)) {
        if let Some(set_state) = &self.feature.set_state {
            set_state(app, f);
        }
    }
}

impl Debug for PersistentBottomSheetController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PersistentBottomSheetController")
            .field("isLocalHistoryEntry", &self.is_local_history_entry)
            .finish_non_exhaustive()
    }
}

/// Dart's `_ScaffoldAspect`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum ScaffoldAspect {
    Geometry,
    HasDrawer,
    DrawerExpanded,
    EndDrawerExpanded,
}

/// Dart's `@internal scaffoldDrawerExpandedOf`.
pub(crate) fn scaffold_drawer_expanded_of(app: &mut App, context: BuildContext) -> Option<bool> {
    ScaffoldScope::inherit_from(app, context, Some(ScaffoldAspect::DrawerExpanded))
        .and_then(ScaffoldScope::drawer_expanded)
}

/// Dart's `@internal scaffoldEndDrawerExpandedOf`.
pub(crate) fn scaffold_end_drawer_expanded_of(
    app: &mut App,
    context: BuildContext,
) -> Option<bool> {
    ScaffoldScope::inherit_from(app, context, Some(ScaffoldAspect::EndDrawerExpanded))
        .and_then(ScaffoldScope::end_drawer_expanded)
}

/// Dart's `_ScaffoldScope`.
#[derive(Debug)]
struct ScaffoldScope {
    has_drawer: bool,
    has_end_drawer: bool,
    is_drawer_open: bool,
    is_end_drawer_open: bool,
    geometry_notifier: Handle<ScaffoldGeometryNotifier>,
    child: WidgetRef,
}

impl ScaffoldScope {
    fn drawer_expanded(&self) -> Option<bool> {
        self.has_drawer.then_some(self.is_drawer_open)
    }

    fn end_drawer_expanded(&self) -> Option<bool> {
        self.has_end_drawer.then_some(self.is_end_drawer_open)
    }

    /// The erased widget: an [`InheritedModel`] has two `IntoWidget` impls, so the kind is
    /// named here.
    fn into_widget(self) -> WidgetRef {
        IntoWidget::<InheritedModelKind>::into_widget(self)
    }
}

impl InheritedWidget for ScaffoldScope {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &ScaffoldScope) -> bool {
        self.geometry_notifier != old_widget.geometry_notifier
            || self.has_drawer != old_widget.has_drawer
            || self.has_end_drawer != old_widget.has_end_drawer
            || self.is_drawer_open != old_widget.is_drawer_open
            || self.is_end_drawer_open != old_widget.is_end_drawer_open
    }
}

impl InheritedModel for ScaffoldScope {
    type Aspect = ScaffoldAspect;

    fn update_should_notify_dependent(
        &self,
        old_widget: &ScaffoldScope,
        dependencies: &HashSet<ScaffoldAspect>,
    ) -> bool {
        (dependencies.contains(&ScaffoldAspect::Geometry)
            && self.geometry_notifier != old_widget.geometry_notifier)
            || (dependencies.contains(&ScaffoldAspect::HasDrawer)
                && self.has_drawer != old_widget.has_drawer)
            || (dependencies.contains(&ScaffoldAspect::DrawerExpanded)
                && self.drawer_expanded() != old_widget.drawer_expanded())
            || (dependencies.contains(&ScaffoldAspect::EndDrawerExpanded)
                && self.end_drawer_expanded() != old_widget.end_drawer_expanded())
    }
}

/// Dart's `_HitTestableAtOrigin`.
#[derive(Debug)]
struct HitTestableAtOrigin {
    global_key: GlobalKey,
}

impl HitTestableAtOrigin {
    fn new(global_key: GlobalKey) -> HitTestableAtOrigin {
        HitTestableAtOrigin { global_key }
    }

    fn hit_testable_at_origin(app: &mut App, key: &GlobalKey) -> bool {
        let Some(context) = key.current_context(app) else {
            debug_assert!(
                false,
                "BuildContext associated with {key:?} is not mounted. \
                 If you see this in a test, this is likely because the test was trying \
                 to simulate status bar tap on a non-iOS platform"
            );
            return false;
        };
        let render_object = context
            .find_render_object(app)
            .expect("a mounted MetaData has a render object")
            .as_box()
            .expect("MetaData creates a RenderMetaData");
        let view_id = View::of(app, context).id();
        let mut result = HitTestResult::new();
        GestureBinding::instance(app).hit_test_in_view(app, &mut result, Offset::ZERO, view_id);
        result.path().iter().any(|entry| {
            (entry.target() as &dyn Any)
                .downcast_ref::<BoxHitTestEntry>()
                .is_some_and(|entry| entry.target() == render_object)
        })
    }
}

impl StatelessWidget for HitTestableAtOrigin {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        MetaData::new()
            .key(self.global_key.clone().into())
            .behavior(HitTestBehavior::Translucent)
            .child(SizedBox::expand())
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    //! The behaviour cases of `test/scaffold_test.dart`. Where Dart's widget is not ported, a
    //! case uses the widget's layout as Dart's has it: `placeholder` is Dart's `Placeholder`
    //! (a `LimitedBox` of 400 that fills what it is given).

    use std::cell::{Cell, RefCell};

    use waft_embedder::{Rect, TargetPlatform};
    use waft_painting::BorderSide;
    use waft_widgets::{
        Center, ColoredBox, Column, ConstrainedBox, CustomPaint, Directionality,
        DraggableScrollableController, DraggableScrollableSheet, LimitedBox, MediaQueryData,
        SingleChildScrollView,
    };

    use super::*;
    use crate::elevated_button::ElevatedButton;
    use crate::test_support::{WidgetTester, material_app, with_main_thread_stack};
    use crate::text_button::TextButton;
    use crate::theme_data::ThemeData;

    fn tester_on(platform: TargetPlatform) -> WidgetTester {
        let tester = WidgetTester::on(platform);
        tester.install_fonts();
        tester
    }

    fn tester() -> WidgetTester {
        tester_on(TargetPlatform::Android)
    }

    /// `tester.pumpWidget(MaterialApp(home: home(app)))`.
    fn pump_app(tester: &mut WidgetTester, home: impl FnOnce(&mut App) -> WidgetRef) {
        let root = {
            let mut app = tester.cell.borrow_mut();
            let theme = ThemeData::new(&app);
            material_app(theme, home(&mut app))
        };
        tester.pump_widget(root);
    }

    /// Dart's `Directionality(textDirection: direction, child: MediaQuery(data: data, child:))`.
    fn directional(
        direction: TextDirection,
        data: MediaQueryData,
        child: impl IntoWidget<waft_widgets::StatefulKind>,
    ) -> WidgetRef {
        Directionality::new(direction, MediaQuery::new(data, child).into_widget()).into_widget()
    }

    /// Dart's `Placeholder()`: a `LimitedBox(maxWidth: 400, maxHeight: 400)` around a
    /// `CustomPaint(size: Size.infinite)`, which fills what it is given.
    fn placeholder(key: Option<&GlobalKey>) -> WidgetRef {
        let mut limited = LimitedBox::new()
            .max_width(400.0)
            .max_height(400.0)
            .child(SizedBox::expand());
        if let Some(key) = key {
            limited = limited.key(key.clone().into());
        }
        limited.into_widget()
    }

    fn rect_of(tester: &WidgetTester, key: &GlobalKey) -> Rect {
        let render = tester.render_object(key).as_box().expect("a render box");
        tester.global_rect(render)
    }

    /// Dart's `tester.getCenter`/`getTopLeft`/`getBottomLeft`..: the point `at(size)` of
    /// `key`'s box, in global coordinates.
    fn point_of(tester: &WidgetTester, key: &GlobalKey, at: impl FnOnce(Size) -> Offset) -> Offset {
        let render = tester.render_object(key).as_box().expect("a render box");
        let app = tester.cell.borrow();
        let size = render.size(&app);
        render.local_to_global(&app, at(size), None)
    }

    fn center_of(tester: &WidgetTester, key: &GlobalKey) -> Offset {
        point_of(tester, key, |size| size.center(Offset::ZERO))
    }

    /// Dart's exact `Offset` comparison for a point the floating action button's scale and
    /// rotation transitions map: the host's matrices are f32, so the point is exact only to
    /// f32 (waft-widgets' `Matrix4Tween` entry).
    #[track_caller]
    fn assert_through_transforms(actual: Offset, expected: Offset) {
        assert!(
            (actual.dx() - expected.dx()).abs() < 1e-3
                && (actual.dy() - expected.dy()).abs() < 1e-3,
            "{} {} is not {} {}",
            actual.dx(),
            actual.dy(),
            expected.dx(),
            expected.dy()
        );
    }

    fn size_of(tester: &WidgetTester, key: &GlobalKey) -> Size {
        rect_of(tester, key).size()
    }

    /// Dart's `tester.state(find.byType(Scaffold))`, through the scaffold's key.
    fn scaffold_state(tester: &WidgetTester, key: &GlobalKey) -> Handle<ScaffoldState> {
        key.current_state::<ScaffoldState>(&mut tester.cell.borrow_mut())
            .expect("a mounted scaffold")
    }

    fn transient_callback_count(tester: &WidgetTester) -> usize {
        waft_scheduler::SchedulerBinding::transient_callback_count(&mut tester.cell.borrow_mut())
    }

    // 'extendBodyBehindAppBar change should not cause the body widget lose state'.
    #[test]
    fn switching_extend_body_behind_app_bar_keeps_the_bodys_state() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/103741
            let mut tester = tester();
            use waft_widgets::ScrollControllerLeaf;
            let controller = waft_widgets::ScrollController::default(&mut tester.cell.borrow_mut());
            let frame = |extend_body_behind_app_bar: bool| {
                directional(
                    TextDirection::Ltr,
                    MediaQueryData::new(),
                    Scaffold::new()
                        .extend_body_behind_app_bar(extend_body_behind_app_bar)
                        .resize_to_avoid_bottom_inset(false)
                        .body(
                            SingleChildScrollView::new()
                                .controller(controller.as_controller())
                                // Dart's `FlutterLogo(size: 1107)`: a box of that size.
                                .child(SizedBox::square(Some(1107.0))),
                        ),
                )
            };
            let pixels =
                |tester: &WidgetTester| controller.as_controller().offset(&tester.cell.borrow());

            tester.pump_widget(frame(true));
            assert_eq!(pixels(&tester), 0.0);

            controller.jump_to(&mut tester.cell.borrow_mut(), 100.0);
            tester.pump(Duration::ZERO);
            assert_eq!(pixels(&tester), 100.0);

            tester.pump_widget(frame(false));
            assert_eq!(pixels(&tester), 100.0);
        });
    }

    // 'Scaffold large bottom padding test'.
    #[test]
    fn a_keyboard_taller_than_the_scaffold_gives_the_body_no_height() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let body_key = GlobalKey::new();
            let frame = |bottom: f64| {
                directional(
                    TextDirection::Ltr,
                    MediaQueryData::new().view_insets(EdgeInsets::only(0.0, 0.0, 0.0, bottom)),
                    Scaffold::new().body(Container::new().key(body_key.clone().into())),
                )
            };
            tester.pump_widget(frame(700.0));
            assert_eq!(size_of(&tester, &body_key), Size::new(800.0, 0.0));

            tester.pump_widget(frame(500.0));
            assert_eq!(size_of(&tester, &body_key), Size::new(800.0, 100.0));

            tester.pump_widget(boilerplate(
                MediaQuery::new(
                    MediaQueryData::new().view_insets(EdgeInsets::only(0.0, 0.0, 0.0, 580.0)),
                    with_title_app_bar(Scaffold::new())
                        .body(Container::new().key(body_key.clone().into())),
                )
                .into_widget(),
            ));
            assert_eq!(size_of(&tester, &body_key), Size::new(800.0, 0.0));
        });
    }

    fn body_size_case(body: fn(&GlobalKey) -> WidgetRef) -> (Size, Offset) {
        let mut result = (Size::ZERO, Offset::ZERO);
        with_main_thread_stack(|| {
            let mut tester = tester();
            let test_key = GlobalKey::new();
            tester.pump_widget(directional(
                TextDirection::Ltr,
                MediaQueryData::new(),
                Scaffold::new().body(body(&test_key)),
            ));
            let rect = rect_of(&tester, &test_key);
            result = (rect.size(), rect.top_left());
        });
        result
    }

    // 'body size with container'.
    #[test]
    fn a_container_body_fills_the_scaffold() {
        let (size, origin) =
            body_size_case(|key| Container::new().key(key.clone().into()).into_widget());
        assert_eq!(size, Size::new(800.0, 600.0));
        assert_eq!(origin, Offset::ZERO);
    }

    // 'body size with sized container'.
    #[test]
    fn a_sized_container_body_keeps_its_height_at_the_top() {
        let (size, origin) = body_size_case(|key| {
            Container::new()
                .key(key.clone().into())
                .height(100.0)
                .into_widget()
        });
        assert_eq!(size, Size::new(800.0, 100.0));
        assert_eq!(origin, Offset::ZERO);
    }

    // 'body size with centered container'.
    #[test]
    fn a_centered_container_body_fills_the_scaffold() {
        let (size, origin) = body_size_case(|key| {
            Center::new()
                .child(Container::new().key(key.clone().into()))
                .into_widget()
        });
        assert_eq!(size, Size::new(800.0, 600.0));
        assert_eq!(origin, Offset::ZERO);
    }

    // 'body size with button'.
    #[test]
    fn a_button_body_keeps_its_minimum_size_at_the_top_left() {
        let (size, origin) = body_size_case(|key| {
            TextButton::new(
                Some(Listener::new(|_| {})),
                Some(waft_widgets::Text::new("")),
            )
            .key(key.clone().into())
            .into_widget()
        });
        assert_eq!(size, Size::new(64.0, 48.0));
        assert_eq!(origin, Offset::ZERO);
    }

    // 'body size with extendBody'.
    #[test]
    fn extend_body_lays_the_body_out_under_the_bottom_navigation_bar() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let body_key = GlobalKey::new();
            let media_query_bottom = Rc::new(Cell::new(f64::NAN));
            let frame = |tester: &WidgetTester,
                         extend_body: bool,
                         resize_to_avoid_bottom_inset: Option<bool>,
                         view_inset_bottom: f64| {
                let body_key = body_key.clone();
                let media_query_bottom = Rc::clone(&media_query_bottom);
                let mut scaffold = Scaffold::new()
                    .extend_body(extend_body)
                    .body(Builder::new(move |app, context| {
                        media_query_bottom.set(MediaQuery::padding_of(app, context).bottom);
                        Container::new().key(body_key.clone().into()).into_widget()
                    }))
                    // Dart's `BottomAppBar(child: SizedBox(height: 48.0))`, which Material 2
                    // lays out 48 tall; `BottomAppBar` is not ported.
                    .bottom_navigation_bar(SizedBox::new().height(48.0));
                if let Some(resize_to_avoid_bottom_inset) = resize_to_avoid_bottom_inset {
                    scaffold = scaffold.resize_to_avoid_bottom_inset(resize_to_avoid_bottom_inset);
                }
                let theme = ThemeData::builder()
                    .use_material3(false)
                    .build(&tester.cell.borrow());
                material_app(
                    theme,
                    MediaQuery::new(
                        MediaQueryData::new().view_insets(EdgeInsets::only(
                            0.0,
                            0.0,
                            0.0,
                            view_inset_bottom,
                        )),
                        scaffold,
                    )
                    .into_widget(),
                )
            };

            tester.pump_widget(frame(&tester, true, None, 0.0));
            assert_eq!(size_of(&tester, &body_key), Size::new(800.0, 600.0));
            assert_eq!(media_query_bottom.get(), 48.0);

            tester.pump_widget(frame(&tester, false, None, 0.0));
            assert_eq!(size_of(&tester, &body_key), Size::new(800.0, 552.0)); // 552 = 600 - 48 (BAB height)
            assert_eq!(media_query_bottom.get(), 0.0);

            // If resizeToAvoidBottomInsets is false, same results as if it was unspecified (null).
            tester.pump_widget(frame(&tester, true, Some(false), 100.0));
            assert_eq!(size_of(&tester, &body_key), Size::new(800.0, 600.0));
            assert_eq!(media_query_bottom.get(), 48.0);

            tester.pump_widget(frame(&tester, false, Some(false), 100.0));
            assert_eq!(size_of(&tester, &body_key), Size::new(800.0, 552.0));
            assert_eq!(media_query_bottom.get(), 0.0);

            // If resizeToAvoidBottomInsets is true and viewInsets.bottom is > the bottom
            // navigation bar's height then the body always resizes and the MediaQuery
            // isn't adjusted. This case corresponds to the keyboard appearing.
            tester.pump_widget(frame(&tester, true, Some(true), 100.0));
            assert_eq!(size_of(&tester, &body_key), Size::new(800.0, 500.0));
            assert_eq!(media_query_bottom.get(), 0.0);

            tester.pump_widget(frame(&tester, false, Some(true), 100.0));
            assert_eq!(size_of(&tester, &body_key), Size::new(800.0, 500.0));
            assert_eq!(media_query_bottom.get(), 0.0);
        });
    }

    // 'body size with extendBodyBehindAppBar'.
    #[test]
    fn extend_body_behind_app_bar_lays_the_body_out_under_the_app_bar() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let app_bar_key = GlobalKey::new();
            let body_key = GlobalKey::new();

            const APP_BAR_HEIGHT: f64 = 100.0;
            const WINDOW_PADDING_TOP: f64 = 24.0;
            let media_query_top = Rc::new(Cell::new(f64::NAN));

            let frame = |extend_body_behind_app_bar: bool,
                         has_app_bar: bool,
                         fixed_height_app_bar: bool| {
                let body_key = body_key.clone();
                let media_query_top = Rc::clone(&media_query_top);
                let mut scaffold = Scaffold::new()
                    .extend_body_behind_app_bar(extend_body_behind_app_bar)
                    .body(Builder::new(move |app, context| {
                        media_query_top.set(MediaQuery::padding_of(app, context).top);
                        Container::new().key(body_key.clone().into()).into_widget()
                    }));
                if has_app_bar {
                    scaffold = scaffold.app_bar(PreferredSizeWidgetRef::new(
                        waft_widgets::PreferredSize::new(
                            Size::from_height(APP_BAR_HEIGHT),
                            Container::new().constraints(
                                BoxConstraints::new().min_height(APP_BAR_HEIGHT).max_height(
                                    if fixed_height_app_bar {
                                        APP_BAR_HEIGHT
                                    } else {
                                        f64::INFINITY
                                    },
                                ),
                            ),
                        )
                        .key(app_bar_key.clone().into()),
                    ));
                }
                directional(
                    TextDirection::Ltr,
                    MediaQueryData::new().padding(EdgeInsets::only(
                        0.0,
                        WINDOW_PADDING_TOP,
                        0.0,
                        0.0,
                    )),
                    scaffold,
                )
            };
            let app_bar_found = |tester: &WidgetTester| {
                let mut app = tester.cell.borrow_mut();
                app_bar_key.current_context(&mut app).is_some()
            };

            // When an appbar is provided, the Scaffold's body is built within a
            // MediaQuery with padding.top = 0, and the appBar's maxHeight is
            // constrained to its preferredSize.height + the original MediaQuery
            // padding.top. When extendBodyBehindAppBar is true, an additional
            // inner MediaQuery is added around the Scaffold's body with padding.top
            // equal to the overall height of the appBar. See _BodyBuilder in
            // material/scaffold.dart.

            tester.pump_widget(frame(true, true, false));
            assert_eq!(size_of(&tester, &body_key), Size::new(800.0, 600.0));
            assert_eq!(
                size_of(&tester, &app_bar_key),
                Size::new(800.0, APP_BAR_HEIGHT + WINDOW_PADDING_TOP)
            );
            assert_eq!(media_query_top.get(), APP_BAR_HEIGHT + WINDOW_PADDING_TOP);

            tester.pump_widget(frame(true, false, false));
            assert_eq!(size_of(&tester, &body_key), Size::new(800.0, 600.0));
            assert!(!app_bar_found(&tester));
            assert_eq!(media_query_top.get(), WINDOW_PADDING_TOP);

            tester.pump_widget(frame(false, true, false));
            assert_eq!(
                size_of(&tester, &body_key),
                Size::new(800.0, 600.0 - APP_BAR_HEIGHT - WINDOW_PADDING_TOP)
            );
            assert_eq!(
                size_of(&tester, &app_bar_key),
                Size::new(800.0, APP_BAR_HEIGHT + WINDOW_PADDING_TOP)
            );
            assert_eq!(media_query_top.get(), 0.0);

            tester.pump_widget(frame(false, false, false));
            assert_eq!(size_of(&tester, &body_key), Size::new(800.0, 600.0));
            assert!(!app_bar_found(&tester));
            assert_eq!(media_query_top.get(), WINDOW_PADDING_TOP);

            tester.pump_widget(frame(true, true, true));
            assert_eq!(size_of(&tester, &body_key), Size::new(800.0, 600.0));
            assert_eq!(
                size_of(&tester, &app_bar_key),
                Size::new(800.0, APP_BAR_HEIGHT)
            );
            assert_eq!(media_query_top.get(), APP_BAR_HEIGHT);

            tester.pump_widget(frame(true, false, true));
            assert_eq!(size_of(&tester, &body_key), Size::new(800.0, 600.0));
            assert!(!app_bar_found(&tester));
            assert_eq!(media_query_top.get(), WINDOW_PADDING_TOP);

            tester.pump_widget(frame(false, true, true));
            assert_eq!(
                size_of(&tester, &body_key),
                Size::new(800.0, 600.0 - APP_BAR_HEIGHT)
            );
            assert_eq!(
                size_of(&tester, &app_bar_key),
                Size::new(800.0, APP_BAR_HEIGHT)
            );
            assert_eq!(media_query_top.get(), 0.0);

            tester.pump_widget(frame(false, false, true));
            assert_eq!(size_of(&tester, &body_key), Size::new(800.0, 600.0));
            assert!(!app_bar_found(&tester));
            assert_eq!(media_query_top.get(), WINDOW_PADDING_TOP);
        });
    }

    // 'Nested scaffold body insets'.
    #[test]
    fn nested_scaffolds_inset_the_body_unless_both_opt_out() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let body_key = GlobalKey::new();
            let frame = |inner: Option<bool>, outer: Option<bool>| {
                let body_key = body_key.clone();
                let mut inner_scaffold =
                    Scaffold::new().body(Container::new().key(body_key.into()));
                if let Some(inner) = inner {
                    inner_scaffold = inner_scaffold.resize_to_avoid_bottom_inset(inner);
                }
                let inner_scaffold = Rc::new(std::cell::RefCell::new(Some(inner_scaffold)));
                let mut outer_scaffold =
                    Scaffold::new().body(Builder::new(move |_app, _context| {
                        inner_scaffold
                            .borrow_mut()
                            .take()
                            .map(IntoWidget::into_widget)
                            .unwrap_or_else(|| SizedBox::shrink().into_widget())
                    }));
                if let Some(outer) = outer {
                    outer_scaffold = outer_scaffold.resize_to_avoid_bottom_inset(outer);
                }
                directional(
                    TextDirection::Ltr,
                    MediaQueryData::new().view_insets(EdgeInsets::only(0.0, 0.0, 0.0, 100.0)),
                    outer_scaffold,
                )
            };
            for (inner, outer, height) in [
                (Some(true), Some(true), 500.0),
                (Some(false), Some(true), 500.0),
                (Some(true), Some(false), 500.0),
                // This is the only case where the body is not bottom inset.
                (Some(false), Some(false), 600.0),
                // resizeToAvoidBottomInset default is true
                (None, None, 500.0),
                (None, Some(false), 500.0),
                (Some(false), None, 500.0),
            ] {
                tester.pump_widget(frame(inner, outer));
                assert_eq!(
                    size_of(&tester, &body_key),
                    Size::new(800.0, height),
                    "inner {inner:?}, outer {outer:?}"
                );
            }
        });
    }

    // 'Floating action button directionality'.
    #[test]
    fn the_floating_action_button_sits_at_the_end_for_the_text_direction() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let fab_key = GlobalKey::new();
            let frame = |direction: TextDirection| {
                directional(
                    direction,
                    MediaQueryData::new().view_insets(EdgeInsets::only(0.0, 0.0, 0.0, 200.0)),
                    Scaffold::new().floating_action_button(
                        FloatingActionButton::new(None)
                            .key(fab_key.clone().into())
                            .child(waft_widgets::Text::new("1")),
                    ),
                )
            };
            tester.pump_widget(frame(TextDirection::Ltr));
            assert_through_transforms(center_of(&tester, &fab_key), Offset::new(756.0, 356.0));

            tester.pump_widget(frame(TextDirection::Rtl));
            assert_eq!(transient_callback_count(&tester), 0);
            assert_through_transforms(center_of(&tester, &fab_key), Offset::new(44.0, 356.0));
        });
    }

    // 'Floating Action Button bottom padding not consumed by viewInsets'.
    #[test]
    fn the_keyboard_does_not_consume_the_floating_action_buttons_bottom_view_padding() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let fab_key = GlobalKey::new();
            let frame = |data: MediaQueryData| {
                directional(
                    TextDirection::Ltr,
                    data,
                    Scaffold::new()
                        .resize_to_avoid_bottom_inset(false)
                        .body(Container::new())
                        .floating_action_button(placeholder(Some(&fab_key))),
                )
            };
            tester.pump_widget(frame(
                MediaQueryData::new().view_padding(EdgeInsets::only(0.0, 0.0, 0.0, 20.0)),
            ));
            let initial_point = center_of(&tester, &fab_key);
            let bottom_left = point_of(&tester, &fab_key, |size| size.bottom_left(Offset::ZERO));
            assert_through_transforms(
                Offset::new(
                    bottom_left.dx(),
                    600.0 - 20.0 - crate::K_FLOATING_ACTION_BUTTON_MARGIN,
                ),
                bottom_left,
            );

            // Consume bottom padding - as if by the keyboard opening
            tester.pump_widget(frame(
                MediaQueryData::new()
                    .view_padding(EdgeInsets::only(0.0, 0.0, 0.0, 20.0))
                    .view_insets(EdgeInsets::only(0.0, 0.0, 0.0, 300.0)),
            ));
            let final_point = center_of(&tester, &fab_key);
            assert_eq!(initial_point, final_point);
        });
    }

    // 'viewPadding change should trigger _ScaffoldLayout re-layout'.
    #[test]
    fn a_view_padding_change_lays_the_scaffold_out_again() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let frame = |bottom: f64| {
                directional(
                    TextDirection::Ltr,
                    MediaQueryData::new().view_padding(EdgeInsets::only(0.0, 0.0, 0.0, bottom)),
                    Scaffold::new()
                        .resize_to_avoid_bottom_inset(false)
                        .body(Container::new())
                        .floating_action_button(placeholder(None)),
                )
            };
            tester.pump_widget(frame(300.0));
            let layout = tester.first::<waft_rendering::RenderCustomMultiChildLayoutBox>();
            assert!(
                !waft_rendering::RenderBox::as_object(layout)
                    .debug_needs_layout(&tester.cell.borrow())
            );

            // Dart's `pumpWidget(.., phase: EnginePhase.build)`: build without laying out.
            {
                let mut app = tester.cell.borrow_mut();
                let view = app.platform().views().into_iter().next().expect("one view");
                let root = waft_widgets::View::new(view, frame(400.0)).into_widget();
                let binding = WidgetsBinding::instance(&mut app);
                binding.attach_root_widget(&mut app, root);
                let root_element = binding.root_element(&app).expect("a root element");
                let build_owner = binding.build_owner(&app);
                build_owner.build_scope(&mut app, root_element, None);
            }
            assert!(
                waft_rendering::RenderBox::as_object(layout)
                    .debug_needs_layout(&tester.cell.borrow())
            );
        });
    }

    // 'BottomSheet bottom padding is not consumed by viewInsets'.
    #[test]
    fn the_keyboard_does_not_move_a_bottom_sheet_that_does_not_avoid_it() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let sheet_key = GlobalKey::new();
            let frame = |data: MediaQueryData| {
                directional(
                    TextDirection::Ltr,
                    data,
                    Scaffold::new()
                        .resize_to_avoid_bottom_inset(false)
                        .body(Container::new())
                        .bottom_sheet(placeholder(Some(&sheet_key))),
                )
            };
            tester.pump_widget(frame(
                MediaQueryData::new().padding(EdgeInsets::only(0.0, 0.0, 0.0, 20.0)),
            ));
            let initial_point = center_of(&tester, &sheet_key);
            // Consume bottom padding - as if by the keyboard opening
            tester.pump_widget(frame(
                MediaQueryData::new()
                    .view_padding(EdgeInsets::only(0.0, 0.0, 0.0, 20.0))
                    .view_insets(EdgeInsets::only(0.0, 0.0, 0.0, 300.0)),
            ));
            let final_point = center_of(&tester, &sheet_key);
            assert_eq!(initial_point, final_point);
        });
    }

    fn tall_body() -> WidgetRef {
        SingleChildScrollView::new()
            .child(
                Container::new()
                    .color(Colors::AMBER.primary())
                    .height(5000.0)
                    .child(waft_widgets::Text::new("body")),
            )
            .into_widget()
    }

    fn center_of_element(tester: &WidgetTester, element: waft_widgets::AnyElement) -> Offset {
        tester.global_rect(tester.render_box_of(element)).center()
    }

    // 'Persistent bottom buttons are persistent'.
    #[test]
    fn persistent_footer_buttons_stay_tappable_after_the_body_scrolls() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let did_press_button = Rc::new(Cell::new(false));
            pump_app(&mut tester, |_app| {
                let did_press_button = Rc::clone(&did_press_button);
                Scaffold::new()
                    .body(tall_body())
                    .persistent_footer_buttons(vec![
                        TextButton::new(
                            Some(Listener::new(move |_| did_press_button.set(true))),
                            Some(waft_widgets::Text::new("X")),
                        )
                        .into_widget(),
                    ])
                    .into_widget()
            });

            let scroll_view = tester.find_by_type::<SingleChildScrollView>()[0];
            let start = center_of_element(&tester, scroll_view);
            tester.drag_from(start, Offset::new(0.0, -1000.0), 1);
            tester.pump(Duration::ZERO);
            assert!(!did_press_button.get());
            let x = tester.find_text("X")[0];
            let at = center_of_element(&tester, x);
            tester.tap(at, 2);
            assert!(did_press_button.get());
        });
    }

    // 'Persistent bottom buttons alignment'.
    #[test]
    fn persistent_footer_buttons_follow_the_footer_alignment() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let build_app = |tester: &mut WidgetTester, alignment: AlignmentDirectional| {
                pump_app(tester, |_app| {
                    Scaffold::new()
                        .body(tall_body())
                        .persistent_footer_alignment(alignment)
                        .persistent_footer_buttons(vec![
                            TextButton::new(
                                Some(Listener::new(|_| {})),
                                Some(waft_widgets::Text::new("X")),
                            )
                            .into_widget(),
                        ])
                        .into_widget()
                });
            };
            let footer_button = |tester: &WidgetTester| {
                let button = tester.find_by_type::<TextButton>()[0];
                tester.global_rect(tester.render_box_of(button))
            };

            build_app(&mut tester, AlignmentDirectional::CENTER_END);
            assert_eq!(footer_button(&tester).right, 800.0 - 8.0);

            build_app(&mut tester, AlignmentDirectional::CENTER);
            assert_eq!(footer_button(&tester).center().dx(), 800.0 / 2.0);

            build_app(&mut tester, AlignmentDirectional::CENTER_START);
            assert_eq!(footer_button(&tester).left, 8.0);
        });
    }

    /// Dart's `find.ancestor(of: find.byType(OverflowBar), matching: find.byType(W)).first`.
    fn overflow_bar_ancestor<W: 'static>(tester: &WidgetTester) -> waft_widgets::AnyElement {
        let overflow_bar = tester.find_by_type::<OverflowBar>()[0];
        tester.ancestor_of_type::<W>(overflow_bar)
    }

    // 'Persistent bottom buttons apply media padding'.
    #[test]
    fn persistent_footer_buttons_apply_the_media_padding() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(directional(
                TextDirection::Ltr,
                MediaQueryData::new().padding(EdgeInsets::only(10.0, 20.0, 30.0, 40.0)),
                Scaffold::new()
                    .body(tall_body())
                    .persistent_footer_buttons(vec![placeholder(None)]),
            ));

            let buttons_bar = overflow_bar_ancestor::<Padding>(&tester);
            let rect = tester.global_rect(tester.render_box_of(buttons_bar));
            assert_eq!(rect.bottom_left(), Offset::new(10.0, 560.0));
            assert_eq!(rect.bottom_right(), Offset::new(770.0, 560.0));
        });
    }

    // 'Persistent bottom buttons bottom padding is not consumed by viewInsets'.
    #[test]
    fn the_keyboard_does_not_move_persistent_footer_buttons_that_do_not_avoid_it() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let button_key = GlobalKey::new();
            let frame = |data: MediaQueryData| {
                directional(
                    TextDirection::Ltr,
                    data,
                    Scaffold::new()
                        .resize_to_avoid_bottom_inset(false)
                        .body(Container::new())
                        .persistent_footer_buttons(vec![placeholder(Some(&button_key))]),
                )
            };
            tester.pump_widget(frame(
                MediaQueryData::new().padding(EdgeInsets::only(0.0, 0.0, 0.0, 20.0)),
            ));
            let initial_point = center_of(&tester, &button_key);
            // Consume bottom padding - as if by the keyboard opening
            tester.pump_widget(frame(
                MediaQueryData::new()
                    .view_padding(EdgeInsets::only(0.0, 0.0, 0.0, 20.0))
                    .view_insets(EdgeInsets::only(0.0, 0.0, 0.0, 300.0)),
            ));
            let final_point = center_of(&tester, &button_key);
            assert_eq!(initial_point, final_point);
        });
    }

    // 'Persistent bottom buttons can apply decoration'.
    #[test]
    fn persistent_footer_buttons_take_the_footer_decoration() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(directional(
                TextDirection::Ltr,
                MediaQueryData::new().padding(EdgeInsets::only(10.0, 20.0, 30.0, 40.0)),
                Scaffold::new()
                    .body(tall_body())
                    .persistent_footer_decoration(BoxDecoration::new().border(Border {
                        top: BorderSide {
                            color: Colors::RED.primary(),
                            ..BorderSide::default()
                        },
                        ..Border::default()
                    }))
                    .persistent_footer_buttons(vec![placeholder(None)]),
            ));

            let persistent_footer = overflow_bar_ancestor::<Container>(&tester);
            let top_color = tester.widget(persistent_footer, |container: &Container| {
                let decoration = container.decoration.as_ref().expect("a decoration");
                let decoration = decoration
                    .as_any()
                    .downcast_ref::<BoxDecoration>()
                    .expect("a BoxDecoration");
                decoration.border.as_ref().expect("a border").top().color
            });
            assert_eq!(top_color, Colors::RED.primary());
        });
    }

    // 'Scaffold background color defaults to ColorScheme.surface'.
    #[test]
    fn the_background_colour_defaults_to_the_colour_schemes_surface() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = {
                let app = tester.cell.borrow();
                let base = ThemeData::new(&app);
                #[allow(deprecated)]
                let color_scheme = base
                    .color_scheme
                    .copy_with()
                    .with_surface(Colors::ORANGE.primary())
                    .with_background(Colors::GREEN.primary());
                ThemeData::builder().color_scheme(color_scheme).build(&app)
            };
            let surface = theme.color_scheme.surface;
            let root = material_app(
                theme,
                Scaffold::new().body(SizedBox::expand()).into_widget(),
            );
            tester.pump_widget(root);

            let scaffold = tester.find_by_type::<Scaffold>()[0];
            let material = *tester
                .find_by_type::<Material>()
                .iter()
                .find(|material| tester.ancestors(**material).contains(&scaffold))
                .expect("the scaffold's material");
            let color = tester.widget(material, |material: &Material| material.color);
            assert_eq!(color, Some(surface));
        });
    }

    // 'Body height remains Scaffold height when keyboard is smaller than bottomNavigationBar
    // and extendBody is true'.
    #[test]
    fn an_extended_body_keeps_the_scaffolds_height_under_a_keyboard_shorter_than_the_bar() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let body_key = GlobalKey::new();
            let frame = |tester: &mut WidgetTester, keyboard_height: f64| {
                let body_key = body_key.clone();
                pump_app(tester, move |_app| {
                    Builder::new(move |app, context| {
                        MediaQuery::new(
                            MediaQuery::of(app, context)
                                .copy_with()
                                .view_insets(EdgeInsets::only(0.0, 0.0, 0.0, keyboard_height)),
                            Scaffold::new()
                                .extend_body(true)
                                .body(SizedBox::expand().key(body_key.clone().into()))
                                .bottom_navigation_bar(SizedBox::new().height(100.0)),
                        )
                        .into_widget()
                    })
                    .into_widget()
                });
            };

            frame(&mut tester, 0.0);
            assert_eq!(size_of(&tester, &body_key).height(), 600.0);

            frame(&mut tester, 100.0);
            assert_eq!(size_of(&tester, &body_key).height(), 600.0);

            frame(&mut tester, 200.0);
            assert_eq!(size_of(&tester, &body_key).height(), 400.0);
        });
    }

    /// What a drawer callback last said, Dart's captured `bool`.
    #[derive(Clone, Default)]
    struct Flag(Rc<Cell<bool>>);

    impl Flag {
        fn setter(&self) -> impl Fn(&mut App, bool) + 'static {
            let flag = Rc::clone(&self.0);
            move |_app, value| flag.set(value)
        }

        fn get(&self) -> bool {
            self.0.get()
        }
    }

    fn drawer_scaffold(
        key: &GlobalKey,
        is_drawer_open: &Flag,
        is_end_drawer_open: &Flag,
    ) -> Scaffold {
        Scaffold::new()
            .key(key.clone().into())
            .drawer(Container::new().color(Colors::BLUE.primary()))
            .on_drawer_changed(is_drawer_open.setter())
            .end_drawer(Container::new().color(Colors::GREEN.primary()))
            .on_end_drawer_changed(is_end_drawer_open.setter())
            .body(Container::new())
    }

    // 'Scaffold drawer callback test'.
    #[test]
    fn opening_one_drawer_closes_the_other_and_reports_both() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let (is_drawer_open, is_end_drawer_open) = (Flag::default(), Flag::default());
            pump_app(&mut tester, |_app| {
                drawer_scaffold(&key, &is_drawer_open, &is_end_drawer_open).into_widget()
            });
            let scaffold_state = scaffold_state(&tester, &key);

            scaffold_state.open_drawer(&mut tester.cell.borrow_mut());
            tester.pump_and_settle();
            assert!(is_drawer_open.get());
            scaffold_state.open_end_drawer(&mut tester.cell.borrow_mut());
            tester.pump_and_settle();
            assert!(!is_drawer_open.get());

            scaffold_state.open_end_drawer(&mut tester.cell.borrow_mut());
            tester.pump_and_settle();
            assert!(is_end_drawer_open.get());
            scaffold_state.open_drawer(&mut tester.cell.borrow_mut());
            tester.pump_and_settle();
            assert!(!is_end_drawer_open.get());
        });
    }

    // 'Scaffold drawer callback test - only call when changed'.
    #[test]
    fn a_fling_too_short_to_open_a_drawer_calls_no_drawer_callback() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let on_drawer_changed_called = Flag::default();
            let on_end_drawer_changed_called = Flag::default();
            pump_app(&mut tester, |_app| {
                let drawer_called = Rc::clone(&on_drawer_changed_called.0);
                let end_drawer_called = Rc::clone(&on_end_drawer_changed_called.0);
                Scaffold::new()
                    .key(key.clone().into())
                    .drawer(Container::new().color(Colors::BLUE.primary()))
                    .on_drawer_changed(move |_app, _is_open| drawer_called.set(true))
                    .end_drawer(Container::new().color(Colors::GREEN.primary()))
                    .on_end_drawer_changed(move |_app, _is_open| end_drawer_called.set(true))
                    .body(Container::new())
                    .into_widget()
            });

            tester.fling_from(Offset::ZERO, Offset::new(10.0, 0.0), 10.0, 1);
            assert!(!on_drawer_changed_called.get());

            tester.pump_and_settle();

            let width = WidgetTester::SIZE.width();
            tester.fling_from(
                Offset::new(width - 1.0, 0.0),
                Offset::new(-10.0, 0.0),
                10.0,
                2,
            );
            tester.pump_and_settle();
            assert!(!on_end_drawer_changed_called.get());
        });
    }

    // 'Drawer can be dismissed with escape keyboard shortcut'.
    #[test]
    fn the_escape_key_dismisses_an_open_drawer() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let (is_drawer_open, is_end_drawer_open) = (Flag::default(), Flag::default());
            pump_app(&mut tester, |_app| {
                drawer_scaffold(&key, &is_drawer_open, &is_end_drawer_open).into_widget()
            });
            let scaffold_state = scaffold_state(&tester, &key);
            let escape = |tester: &mut WidgetTester| {
                tester.send_key_event(
                    waft_services::PhysicalKeyboardKey::ESCAPE,
                    waft_services::LogicalKeyboardKey::ESCAPE,
                );
            };

            scaffold_state.open_drawer(&mut tester.cell.borrow_mut());
            tester.pump_and_settle();
            assert!(is_drawer_open.get());
            assert!(!is_end_drawer_open.get());

            // Try to dismiss the drawer with the shortcut key
            escape(&mut tester);
            tester.pump_and_settle();
            assert!(!is_drawer_open.get());
            assert!(!is_end_drawer_open.get());

            scaffold_state.open_end_drawer(&mut tester.cell.borrow_mut());
            tester.pump_and_settle();
            assert!(!is_drawer_open.get());
            assert!(is_end_drawer_open.get());

            // Try to dismiss the drawer with the shortcut key
            escape(&mut tester);
            tester.pump_and_settle();
            assert!(!is_drawer_open.get());
            assert!(!is_end_drawer_open.get());
        });
    }

    /// `AppBar(centerTitle: true, title: Text('Title'))`, the app bar of Dart's drawer cases.
    fn centered_title_app_bar() -> PreferredSizeWidgetRef {
        PreferredSizeWidgetRef::new(
            AppBar::new()
                .center_title(true)
                .title(waft_widgets::Text::new("Title")),
        )
    }

    /// `Scaffold(drawer: Drawer(child: Text('Drawer')), body: Text(body), appBar:
    /// AppBar(centerTitle: true, title: Text('Title')))`.
    fn text_drawer_scaffold(key: &GlobalKey, body: &str) -> Scaffold {
        Scaffold::new()
            .key(key.clone().into())
            .drawer(crate::drawer::Drawer::new().child(waft_widgets::Text::new("Drawer")))
            .body(waft_widgets::Text::new(body))
            .app_bar(centered_title_app_bar())
    }

    /// Dart's `tester.tap(find.byType(IconButton).first)`: the app bar's drawer button.
    fn tap_first_icon_button(tester: &mut WidgetTester, pointer: i64) {
        let button = *tester
            .find_by_type::<crate::icon_button::IconButton>()
            .first()
            .expect("an IconButton");
        let at = center_of_element(tester, button);
        tester.tap(at, pointer);
    }

    // 'Drawer opens correctly with custom edgeDragWidth' .
    #[test]
    fn a_wider_edge_drag_width_opens_the_drawer_from_further_in() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            // The default edge drag width is 20.0.
            pump_app(&mut tester, |_app| {
                text_drawer_scaffold(&key, "Scaffold body").into_widget()
            });
            let scaffold_state = scaffold_state(&tester, &key);
            assert!(!scaffold_state.is_drawer_open(&tester.cell.borrow()));

            tester.drag_from(Offset::new(35.0, 100.0), Offset::new(300.0, 0.0), 1);
            tester.pump_and_settle();
            assert!(!scaffold_state.is_drawer_open(&tester.cell.borrow()));

            pump_app(&mut tester, |_app| {
                text_drawer_scaffold(&key, "Scaffold Body")
                    .drawer_edge_drag_width(40.0)
                    .into_widget()
            });
            let scaffold_state = self::scaffold_state(&tester, &key);
            assert!(!scaffold_state.is_drawer_open(&tester.cell.borrow()));

            tester.drag_from(Offset::new(35.0, 100.0), Offset::new(300.0, 0.0), 2);
            tester.pump_and_settle();
            assert!(scaffold_state.is_drawer_open(&tester.cell.borrow()));
        });
    }

    const SIMULATED_NOTCH_SIZE: f64 = 40.0;

    // 'Drawer opens correctly with padding from MediaQuery (LTR)' .
    #[test]
    fn the_left_padding_widens_the_drawers_edge_drag_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            pump_app(&mut tester, |_app| {
                text_drawer_scaffold(&key, "Scaffold Body").into_widget()
            });
            let scaffold_state = scaffold_state(&tester, &key);
            assert!(!scaffold_state.is_drawer_open(&tester.cell.borrow()));

            tester.drag_from(
                Offset::new(SIMULATED_NOTCH_SIZE + 15.0, 100.0),
                Offset::new(300.0, 0.0),
                1,
            );
            tester.pump_and_settle();
            assert!(!scaffold_state.is_drawer_open(&tester.cell.borrow()));

            pump_app(&mut tester, |_app| {
                MediaQuery::new(
                    MediaQueryData::new().padding(EdgeInsets::only(
                        SIMULATED_NOTCH_SIZE,
                        0.0,
                        0.0,
                        0.0,
                    )),
                    text_drawer_scaffold(&key, "Scaffold Body"),
                )
                .into_widget()
            });
            let scaffold_state = self::scaffold_state(&tester, &key);
            assert!(!scaffold_state.is_drawer_open(&tester.cell.borrow()));

            tester.drag_from(
                Offset::new(SIMULATED_NOTCH_SIZE + 15.0, 100.0),
                Offset::new(300.0, 0.0),
                2,
            );
            tester.pump_and_settle();
            assert!(scaffold_state.is_drawer_open(&tester.cell.borrow()));
        });
    }

    // 'Drawer opens correctly with padding from MediaQuery (RTL)' .
    #[test]
    fn the_right_padding_widens_the_drawers_edge_drag_area_right_to_left() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            pump_app(&mut tester, |_app| {
                text_drawer_scaffold(&key, "Scaffold Body").into_widget()
            });
            let scaffold_width = size_of(&tester, &key).width();
            let scaffold_state = scaffold_state(&tester, &key);
            assert!(!scaffold_state.is_drawer_open(&tester.cell.borrow()));

            tester.drag_from(
                Offset::new(scaffold_width - SIMULATED_NOTCH_SIZE - 15.0, 100.0),
                Offset::new(-300.0, 0.0),
                1,
            );
            tester.pump_and_settle();
            assert!(!scaffold_state.is_drawer_open(&tester.cell.borrow()));

            pump_app(&mut tester, |_app| {
                MediaQuery::new(
                    MediaQueryData::new().padding(EdgeInsets::only(
                        0.0,
                        0.0,
                        SIMULATED_NOTCH_SIZE,
                        0.0,
                    )),
                    Directionality::new(
                        TextDirection::Rtl,
                        text_drawer_scaffold(&key, "Scaffold body"),
                    ),
                )
                .into_widget()
            });
            let scaffold_state = self::scaffold_state(&tester, &key);
            assert!(!scaffold_state.is_drawer_open(&tester.cell.borrow()));

            tester.drag_from(
                Offset::new(scaffold_width - SIMULATED_NOTCH_SIZE - 15.0, 100.0),
                Offset::new(-300.0, 0.0),
                2,
            );
            tester.pump_and_settle();
            assert!(scaffold_state.is_drawer_open(&tester.cell.borrow()));
        });
    }

    // 'Drawer does not open with a drag gesture when it is disabled on mobile'.
    #[test]
    fn a_disabled_drag_gesture_does_not_open_the_drawer_but_still_closes_it_on_mobile() {
        for platform in [
            TargetPlatform::Android,
            TargetPlatform::Fuchsia,
            TargetPlatform::IOS,
        ] {
            with_main_thread_stack(|| {
                let mut tester = tester_on(platform);
                let key = GlobalKey::new();
                pump_app(&mut tester, |_app| {
                    text_drawer_scaffold(&key, "Scaffold Body").into_widget()
                });
                let scaffold_state = scaffold_state(&tester, &key);
                assert!(!scaffold_state.is_drawer_open(&tester.cell.borrow()));

                // Test that we can open the drawer with a drag gesture when
                // `Scaffold.drawerEnableDragGesture` is true.
                tester.drag_from(Offset::new(0.0, 100.0), Offset::new(300.0, 0.0), 1);
                tester.pump_and_settle();
                assert!(
                    scaffold_state.is_drawer_open(&tester.cell.borrow()),
                    "{platform:?}"
                );

                tester.drag_from(Offset::new(300.0, 100.0), Offset::new(-300.0, 0.0), 2);
                tester.pump_and_settle();
                assert!(
                    !scaffold_state.is_drawer_open(&tester.cell.borrow()),
                    "{platform:?}"
                );

                pump_app(&mut tester, |_app| {
                    text_drawer_scaffold(&key, "Scaffold body")
                        .drawer_enable_open_drag_gesture(false)
                        .into_widget()
                });
                let scaffold_state = self::scaffold_state(&tester, &key);
                assert!(!scaffold_state.is_drawer_open(&tester.cell.borrow()));

                // Test that we cannot open the drawer with a drag gesture when
                // `Scaffold.drawerEnableDragGesture` is false.
                tester.drag_from(Offset::new(0.0, 100.0), Offset::new(300.0, 0.0), 3);
                tester.pump_and_settle();
                assert!(
                    !scaffold_state.is_drawer_open(&tester.cell.borrow()),
                    "{platform:?}"
                );

                // Test that we can close drawer with a drag gesture when
                // `Scaffold.drawerEnableDragGesture` is false.
                tap_first_icon_button(&mut tester, 5);
                tester.pump_and_settle();
                assert!(
                    scaffold_state.is_drawer_open(&tester.cell.borrow()),
                    "{platform:?}"
                );

                tester.drag_from(Offset::new(300.0, 100.0), Offset::new(-300.0, 0.0), 4);
                tester.pump_and_settle();
                assert!(
                    !scaffold_state.is_drawer_open(&tester.cell.borrow()),
                    "{platform:?}"
                );
            });
        }
    }

    // 'Drawer does not open with a drag gesture on desktop'.
    #[test]
    fn the_drawer_does_not_follow_drags_on_desktop() {
        for platform in [
            TargetPlatform::Linux,
            TargetPlatform::MacOS,
            TargetPlatform::Windows,
        ] {
            with_main_thread_stack(|| {
                let mut tester = tester_on(platform);
                let key = GlobalKey::new();
                pump_app(&mut tester, |_app| {
                    text_drawer_scaffold(&key, "Scaffold Body").into_widget()
                });
                let scaffold_state = scaffold_state(&tester, &key);
                assert!(!scaffold_state.is_drawer_open(&tester.cell.borrow()));

                // Test that we cannot open the drawer with a drag gesture.
                tester.drag_from(Offset::new(0.0, 100.0), Offset::new(300.0, 0.0), 1);
                tester.pump_and_settle();
                assert!(
                    !scaffold_state.is_drawer_open(&tester.cell.borrow()),
                    "{platform:?}"
                );

                // Test that we can open the drawer with a tap gesture on drawer icon button.
                tap_first_icon_button(&mut tester, 5);
                tester.pump_and_settle();
                assert!(
                    scaffold_state.is_drawer_open(&tester.cell.borrow()),
                    "{platform:?}"
                );

                // Test that we cannot close the drawer with a drag gesture.
                tester.drag_from(Offset::new(300.0, 100.0), Offset::new(-300.0, 0.0), 2);
                tester.pump_and_settle();
                assert!(
                    scaffold_state.is_drawer_open(&tester.cell.borrow()),
                    "{platform:?}"
                );

                // Test that we can close the drawer with a tap gesture in the body.
                tester.tap(Offset::new(500.0, 300.0), 3);
                tester.pump_and_settle();
                assert!(
                    !scaffold_state.is_drawer_open(&tester.cell.borrow()),
                    "{platform:?}"
                );
            });
        }
    }

    // 'End drawer does not open with a drag gesture when it is disabled'.
    #[test]
    fn a_disabled_drag_gesture_does_not_open_the_end_drawer_but_still_closes_it() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let end_drawer_scaffold = |key: &GlobalKey, body: &str| {
                Scaffold::new()
                    .key(key.clone().into())
                    .end_drawer(
                        crate::drawer::Drawer::new().child(waft_widgets::Text::new("Drawer")),
                    )
                    .body(waft_widgets::Text::new(body))
                    .app_bar(centered_title_app_bar())
            };
            let screen_width = WidgetTester::SIZE.width();
            pump_app(&mut tester, |_app| {
                end_drawer_scaffold(&key, "Scaffold Body").into_widget()
            });
            let scaffold_state = scaffold_state(&tester, &key);
            assert!(!scaffold_state.is_end_drawer_open(&tester.cell.borrow()));

            // Test that we can open the end drawer with a drag gesture when
            // `Scaffold.endDrawerEnableDragGesture` is true.
            tester.drag_from(
                Offset::new(screen_width - 1.0, 100.0),
                Offset::new(-300.0, 0.0),
                1,
            );
            tester.pump_and_settle();
            assert!(scaffold_state.is_end_drawer_open(&tester.cell.borrow()));

            tester.drag_from(
                Offset::new(screen_width - 300.0, 100.0),
                Offset::new(300.0, 0.0),
                2,
            );
            tester.pump_and_settle();
            assert!(!scaffold_state.is_end_drawer_open(&tester.cell.borrow()));

            pump_app(&mut tester, |_app| {
                end_drawer_scaffold(&key, "Scaffold body")
                    .end_drawer_enable_open_drag_gesture(false)
                    .into_widget()
            });
            let scaffold_state = self::scaffold_state(&tester, &key);
            assert!(!scaffold_state.is_end_drawer_open(&tester.cell.borrow()));

            // Test that we cannot open the end drawer with a drag gesture when
            // `Scaffold.endDrawerEnableDragGesture` is false.
            tester.drag_from(
                Offset::new(screen_width - 1.0, 100.0),
                Offset::new(-300.0, 0.0),
                3,
            );
            tester.pump_and_settle();
            assert!(!scaffold_state.is_end_drawer_open(&tester.cell.borrow()));

            // Test that we can close the end drawer a with drag gesture when
            // `Scaffold.endDrawerEnableDragGesture` is false.
            tap_first_icon_button(&mut tester, 5);
            tester.pump_and_settle();
            assert!(scaffold_state.is_end_drawer_open(&tester.cell.borrow()));

            tester.drag_from(
                Offset::new(screen_width - 300.0, 100.0),
                Offset::new(300.0, 0.0),
                4,
            );
            tester.pump_and_settle();
            assert!(!scaffold_state.is_end_drawer_open(&tester.cell.borrow()));
        });
    }

    /// Dart's `_GeometryListener`: listens to `Scaffold.geometryOf` and caches its value
    /// while painting, the only phase it may be read in.
    /// What a [`GeometryCachePainter`] last read, shared with the case.
    type GeometryCache = Rc<Cell<Option<ScaffoldGeometry>>>;

    #[derive(Debug)]
    struct GeometryListener {
        cache: GeometryCache,
        num_notifications: Rc<Cell<usize>>,
    }

    impl GeometryListener {
        fn new() -> (GeometryListener, GeometryCache, Rc<Cell<usize>>) {
            let cache = Rc::new(Cell::new(None));
            let num_notifications = Rc::new(Cell::new(0));
            (
                GeometryListener {
                    cache: Rc::clone(&cache),
                    num_notifications: Rc::clone(&num_notifications),
                },
                cache,
                num_notifications,
            )
        }
    }

    impl StatefulWidget for GeometryListener {
        type State = GeometryListenerState;

        fn create_state(&self) -> GeometryListenerState {
            let num_notifications = Rc::clone(&self.num_notifications);
            GeometryListenerState {
                state: StateData::new(),
                geometry_listenable: None,
                on_geometry_changed: Listener::new(move |_app| {
                    num_notifications.set(num_notifications.get() + 1)
                }),
            }
        }
    }

    struct GeometryListenerState {
        state: StateData<GeometryListener>,
        geometry_listenable: Option<Handle<ScaffoldGeometryNotifier>>,
        on_geometry_changed: Listener,
    }

    impl State for GeometryListenerState {
        type Widget = GeometryListener;
        waft_widgets::state_accessors!();

        fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
            let context = self.context(app);
            let new_listenable = Scaffold::geometry_of(app, context);
            if app.get(self).geometry_listenable == Some(new_listenable) {
                return;
            }
            let on_geometry_changed = app.get(self).on_geometry_changed.clone();
            if let Some(old) = app.get(self).geometry_listenable {
                old.remove_listener(app, &on_geometry_changed);
            }
            new_listenable.add_listener(app, on_geometry_changed);
            app.get_mut(self).geometry_listenable = Some(new_listenable);
        }

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            let notifier = app
                .get(self)
                .geometry_listenable
                .expect("set by did_change_dependencies");
            CustomPaint::new()
                .painter(GeometryCachePainter {
                    notifier,
                    repaint: Rc::new(notifier),
                    cache: Rc::clone(&self.widget(app).cache),
                })
                .into_widget()
        }
    }

    /// Dart's `_GeometryCachePainter`.
    struct GeometryCachePainter {
        notifier: Handle<ScaffoldGeometryNotifier>,
        repaint: Rc<dyn waft_foundation::Listenable>,
        cache: GeometryCache,
    }

    impl waft_rendering::CustomPainter for GeometryCachePainter {
        fn repaint(&self) -> Option<&Rc<dyn waft_foundation::Listenable>> {
            Some(&self.repaint)
        }

        fn paint(&self, app: &mut App, _canvas: &mut waft_embedder::Canvas, _size: Size) {
            self.cache.set(Some(self.notifier.value(app)));
        }

        fn should_repaint(
            &self,
            _app: &App,
            _old_delegate: &dyn waft_rendering::CustomPainter,
        ) -> bool {
            true
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    fn expanded_listener(listener: GeometryListener) -> WidgetRef {
        ConstrainedBox::new(BoxConstraints::expand(None, Some(80.0)))
            .child(listener)
            .into_widget()
    }

    #[track_caller]
    fn assert_rect_through_transforms(actual: Rect, expected: Rect) {
        assert_through_transforms(actual.top_left(), expected.top_left());
        assert_through_transforms(actual.bottom_right(), expected.bottom_right());
    }

    // 'ScaffoldGeometry bottomNavigationBar'.
    #[test]
    fn the_geometry_reports_the_bottom_navigation_bars_top() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let (listener, cache, _) = GeometryListener::new();
            pump_app(&mut tester, |_app| {
                Scaffold::new()
                    .body(Container::new())
                    .bottom_navigation_bar(
                        ConstrainedBox::new(BoxConstraints::expand(None, Some(80.0)))
                            .key(key.clone().into())
                            .child(listener),
                    )
                    .into_widget()
            });
            let navigation_height = size_of(&tester, &key).height();
            let geometry = cache.get().expect("painted");
            assert_eq!(
                geometry.bottom_navigation_bar_top,
                Some(WidgetTester::SIZE.height() - navigation_height)
            );
        });
    }

    // 'ScaffoldGeometry no bottomNavigationBar'.
    #[test]
    fn the_geometry_has_no_bottom_navigation_bar_top_without_a_bar() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (listener, cache, _) = GeometryListener::new();
            pump_app(&mut tester, |_app| {
                Scaffold::new()
                    .body(expanded_listener(listener))
                    .into_widget()
            });
            let geometry = cache.get().expect("painted");
            assert_eq!(geometry.bottom_navigation_bar_top, None);
        });
    }

    fn fab_with_listener(key: &GlobalKey, listener: GeometryListener) -> WidgetRef {
        FloatingActionButton::new(Some(Listener::new(|_| {})))
            .key(key.clone().into())
            .child(listener)
            .into_widget()
    }

    // 'ScaffoldGeometry floatingActionButton'.
    #[test]
    fn the_geometry_reports_the_floating_action_buttons_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let (listener, cache, _) = GeometryListener::new();
            pump_app(&mut tester, |_app| {
                Scaffold::new()
                    .body(Container::new())
                    .floating_action_button(fab_with_listener(&key, listener))
                    .into_widget()
            });
            let fab_rect = rect_of(&tester, &key);
            let geometry = cache.get().expect("painted");
            assert_rect_through_transforms(
                geometry.floating_action_button_area.expect("a fab area"),
                fab_rect,
            );
        });
    }

    // 'ScaffoldGeometry no floatingActionButton'.
    #[test]
    fn the_geometry_has_no_floating_action_button_area_without_a_button() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (listener, cache, _) = GeometryListener::new();
            pump_app(&mut tester, |_app| {
                Scaffold::new()
                    .body(expanded_listener(listener))
                    .into_widget()
            });
            let geometry = cache.get().expect("painted");
            assert_eq!(geometry.floating_action_button_area, None);
        });
    }

    fn rotation_turns(tester: &WidgetTester) -> f64 {
        let rotation = tester.find_by_type::<waft_widgets::Animated<RotationTransition>>()[0];
        let turns = tester.widget(
            rotation,
            |rotation: &waft_widgets::Animated<RotationTransition>| rotation.0.turns(),
        );
        turns.value(&tester.cell.borrow())
    }

    // 'ScaffoldGeometry floatingActionButton entrance/exit animation'.
    #[test]
    fn the_geometry_scales_the_floating_action_button_area_while_it_enters() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let (first_listener, _, _) = GeometryListener::new();
            pump_app(&mut tester, |_app| {
                Scaffold::new()
                    .body(expanded_listener(first_listener))
                    .into_widget()
            });

            let (listener, cache, _) = GeometryListener::new();
            pump_app(&mut tester, |_app| {
                Scaffold::new()
                    .body(Container::new())
                    .floating_action_button(fab_with_listener(&key, listener))
                    .into_widget()
            });

            tester.pump(Duration::from_millis(50));

            let geometry = cache.get().expect("painted");
            let transitioning_fab_rect = geometry.floating_action_button_area.expect("an area");

            let transitioning_rotation = rotation_turns(&tester);

            tester.pump(Duration::from_secs(3));
            let geometry = cache.get().expect("painted");
            let fab_rect = rect_of(&tester, &key);

            let completed_rotation = rotation_turns(&tester);

            assert!(transitioning_rotation < 1.0);

            assert_eq!(completed_rotation, 1.0);

            let area = geometry.floating_action_button_area.expect("an area");
            assert_rect_through_transforms(area, fab_rect);

            assert_eq!(area.center(), transitioning_fab_rect.center());

            assert!(area.width() > transitioning_fab_rect.width());

            assert!(area.height() > transitioning_fab_rect.height());
        });
    }

    // 'ScaffoldGeometry change notifications'.
    #[test]
    fn the_geometry_notifies_on_layout_and_while_the_button_animates() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let (listener, _, num_notifications) = GeometryListener::new();
            let mut num_notifications_at_last_frame = 0;
            pump_app(&mut tester, |_app| {
                Scaffold::new()
                    .body(expanded_listener(listener))
                    .into_widget()
            });

            assert!(num_notifications.get() > num_notifications_at_last_frame);
            num_notifications_at_last_frame = num_notifications.get();

            let (fab_listener, _, _) = GeometryListener::new();
            pump_app(&mut tester, |_app| {
                Scaffold::new()
                    .body(Container::new())
                    .floating_action_button(fab_with_listener(&key, fab_listener))
                    .into_widget()
            });

            assert!(num_notifications.get() > num_notifications_at_last_frame);
            num_notifications_at_last_frame = num_notifications.get();

            tester.pump(Duration::from_millis(50));

            assert!(num_notifications.get() > num_notifications_at_last_frame);
            num_notifications_at_last_frame = num_notifications.get();

            tester.pump(Duration::from_secs(3));

            assert!(num_notifications.get() > num_notifications_at_last_frame);
        });
    }

    // 'Floating action entrance/exit animation'.
    #[test]
    fn a_new_floating_action_button_animates_in_and_the_first_one_does_not() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let fab = |key: &str, label: &str| {
                FloatingActionButton::new(None)
                    .key(KeyRef::Local(Rc::new(ValueKey::new(key.to_string()))))
                    .child(waft_widgets::Text::new(label))
            };
            pump_app(&mut tester, |_app| {
                Scaffold::new()
                    .floating_action_button(fab("one", "1"))
                    .into_widget()
            });

            assert_eq!(transient_callback_count(&tester), 0);

            pump_app(&mut tester, |_app| {
                Scaffold::new()
                    .floating_action_button(fab("two", "2"))
                    .into_widget()
            });

            assert!(transient_callback_count(&tester) > 0);
            tester.pump_widget(Container::new().into_widget());
            assert_eq!(transient_callback_count(&tester), 0);

            pump_app(&mut tester, |_app| Scaffold::new().into_widget());

            assert_eq!(transient_callback_count(&tester), 0);

            pump_app(&mut tester, |_app| {
                Scaffold::new()
                    .floating_action_button(fab("one", "1"))
                    .into_widget()
            });

            assert!(transient_callback_count(&tester) > 0);
        });
    }

    /// Dart's `find.byType(ElevatedButton)` tapped at its centre.
    fn tap_elevated_button(tester: &mut WidgetTester, label: &str, pointer: i64) {
        let text = *tester.find_text(label).first().expect("the button's label");
        let button = tester.ancestor_of_type::<ElevatedButton>(text);
        let at = center_of_element(tester, button);
        tester.tap(at, pointer);
    }

    fn fab_top_left(tester: &WidgetTester) -> Offset {
        let fab = tester.find_by_type::<FloatingActionButton>()[0];
        let render = tester.render_box_of(fab);
        let app = tester.cell.borrow();
        render.local_to_global(&app, Offset::ZERO, None)
    }

    // 'FAB default entrance and exit animations'.
    #[test]
    fn the_floating_action_button_scales_in_and_out_by_default() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let show_fab = Rc::new(Cell::new(false));
            pump_app(&mut tester, |_app| {
                let show_fab = Rc::clone(&show_fab);
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let toggle = {
                        let show_fab = Rc::clone(&show_fab);
                        Listener::new(move |app| {
                            let show_fab = Rc::clone(&show_fab);
                            set_state(app, &mut move |_app| show_fab.set(!show_fab.get()));
                        })
                    };
                    let mut scaffold = Scaffold::new().body(ElevatedButton::new(
                        Some(toggle),
                        Some(waft_widgets::Text::new("Toggle FAB")),
                    ));
                    if show_fab.get() {
                        scaffold = scaffold.floating_action_button(
                            FloatingActionButton::new(Some(Listener::new(|_| {})))
                                .child(waft_widgets::Icon::new(Some(crate::Icons::add()))),
                        );
                    }
                    scaffold.into_widget()
                })
                .into_widget()
            });

            // FAB is not visible.
            assert!(tester.find_by_type::<FloatingActionButton>().is_empty());

            // Tap the button to show the FAB.
            tap_elevated_button(&mut tester, "Toggle FAB", 1);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(100)); // Advance the animation by 100ms.
            // FAB is partially animated in.
            assert!(
                (fab_top_left(&tester).dx() - 743.8).abs() < 0.1,
                "{}",
                fab_top_left(&tester).dx()
            );

            tester.pump(Duration::from_millis(100)); // Advance the animation by 100ms.
            // FAB is fully animated in.
            assert_through_transforms(
                fab_top_left(&tester),
                Offset::new(728.0, fab_top_left(&tester).dy()),
            );

            // Tap the button to hide the FAB.
            tap_elevated_button(&mut tester, "Toggle FAB", 2);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(100)); // Advance the animation by 100ms.
            // FAB is partially animated out.
            assert!(
                (fab_top_left(&tester).dx() - 747.1).abs() < 0.1,
                "{}",
                fab_top_left(&tester).dx()
            );

            tester.pump(Duration::from_millis(100)); // Advance the animation by 100ms.
            // FAB is fully animated out.
            assert_through_transforms(
                fab_top_left(&tester),
                Offset::new(756.0, fab_top_left(&tester).dy()),
            );

            tester.pump(Duration::from_millis(50)); // Advance the animation by 50ms.
            // FAB is not visible.
            assert!(tester.find_by_type::<FloatingActionButton>().is_empty());
        });
    }

    // 'FAB default entrance and exit animations can be disabled'.
    #[test]
    fn the_no_animation_animator_shows_hides_and_moves_the_button_at_once() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let show_fab = Rc::new(Cell::new(false));
            let fab_location =
                Rc::new(RefCell::new(<dyn FloatingActionButtonLocation>::end_float()));
            pump_app(&mut tester, |_app| {
                let show_fab = Rc::clone(&show_fab);
                let fab_location = Rc::clone(&fab_location);
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let toggle = {
                        let show_fab = Rc::clone(&show_fab);
                        let set_state = Rc::clone(&set_state);
                        Listener::new(move |app| {
                            let show_fab = Rc::clone(&show_fab);
                            set_state(app, &mut move |_app| show_fab.set(!show_fab.get()));
                        })
                    };
                    let update_location = {
                        let fab_location = Rc::clone(&fab_location);
                        Listener::new(move |app| {
                            let fab_location = Rc::clone(&fab_location);
                            set_state(app, &mut move |_app| {
                                *fab_location.borrow_mut() =
                                    <dyn FloatingActionButtonLocation>::center_float();
                            });
                        })
                    };
                    let mut scaffold = Scaffold::new()
                        // Disable FAB animations.
                        .floating_action_button_animator(
                            <dyn FloatingActionButtonAnimator>::no_animation(),
                        )
                        .floating_action_button_location(Rc::clone(&fab_location.borrow()))
                        .body(
                            Column::new().children([
                                ElevatedButton::new(
                                    Some(toggle),
                                    Some(waft_widgets::Text::new("Toggle FAB")),
                                )
                                .into_widget(),
                                ElevatedButton::new(
                                    Some(update_location),
                                    Some(waft_widgets::Text::new("Update FAB Location")),
                                )
                                .into_widget(),
                            ]),
                        );
                    if show_fab.get() {
                        scaffold = scaffold.floating_action_button(
                            FloatingActionButton::new(Some(Listener::new(|_| {})))
                                .child(waft_widgets::Icon::new(Some(crate::Icons::add()))),
                        );
                    }
                    scaffold.into_widget()
                })
                .into_widget()
            });

            // FAB is not visible.
            assert!(tester.find_by_type::<FloatingActionButton>().is_empty());

            // Tap the button to show the FAB.
            tap_elevated_button(&mut tester, "Toggle FAB", 1);
            tester.pump(Duration::ZERO);
            // FAB is visible.
            assert_eq!(fab_top_left(&tester).dx(), 728.0);

            // Tap the button to hide the FAB.
            tap_elevated_button(&mut tester, "Toggle FAB", 2);
            tester.pump(Duration::ZERO);
            // FAB is not visible.
            assert!(tester.find_by_type::<FloatingActionButton>().is_empty());

            // Tap the button to show the FAB.
            tap_elevated_button(&mut tester, "Toggle FAB", 3);
            tester.pump(Duration::ZERO);
            // FAB is visible.
            assert_eq!(fab_top_left(&tester).dx(), 728.0);

            // Tap the update location button.
            tap_elevated_button(&mut tester, "Update FAB Location", 4);
            tester.pump(Duration::ZERO);

            // FAB is visible at the new location.
            assert_eq!(fab_top_left(&tester).dx(), 372.0);

            // Tap the button to hide the FAB.
            tap_elevated_button(&mut tester, "Toggle FAB", 5);
            tester.pump(Duration::ZERO);
            // FAB is not visible.
            assert!(tester.find_by_type::<FloatingActionButton>().is_empty());
        });
    }

    // 'ScaffoldMessenger showSnackBar throws an intuitive error message if called during build'.
    #[test]
    #[should_panic(expected = "The showSnackBar() method cannot be called during build.\n")]
    fn show_snack_bar_during_build_explains_where_to_call_it_instead() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_app(&mut tester, |_app| {
                Scaffold::new()
                    .body(Builder::new(|app, context| {
                        ScaffoldMessenger::of(app, context).show_snack_bar(
                            app,
                            SnackBar::new(waft_widgets::Text::new("SnackBar")),
                            None,
                        );
                        SizedBox::shrink().into_widget()
                    }))
                    .into_widget()
            });
        });
    }

    fn panic_message(panic: Box<dyn Any + Send>) -> String {
        panic
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| panic.downcast_ref::<&str>().map(|text| text.to_string()))
            .unwrap_or_default()
    }

    /// Runs `call` at the context of a `Builder` keyed `key`, catching its panic.
    fn panic_at(
        tester: &WidgetTester,
        key: &GlobalKey,
        call: impl FnOnce(&mut App, BuildContext),
    ) -> String {
        let mut app = tester.cell.borrow_mut();
        let context = key.current_context(&mut app).expect("a mounted builder");
        let outcome =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| call(&mut app, context)));
        panic_message(outcome.expect_err("the call panics"))
    }

    fn keyed_builder(key: &GlobalKey) -> WidgetRef {
        Builder::new(|_app, _context| Container::new().into_widget())
            .key(key.clone().into())
            .into_widget()
    }

    // 'Call to Scaffold.of() without context' (Dart calls it while building; here it is
    // called at the builder's context, which is where Dart's build runs it).
    #[test]
    fn scaffold_of_without_a_scaffold_explains_how_to_reach_one() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            pump_app(&mut tester, |_app| keyed_builder(&key));
            let message = panic_at(&tester, &key, |app, context| {
                Scaffold::of(app, context);
            });
            assert!(
                message.starts_with(
                    "Scaffold.of() called with a context that does not contain a Scaffold.\n\
                     No Scaffold ancestor could be found starting from the context that was \
                     passed to Scaffold.of(). This usually happens when the context provided is \
                     from the same StatefulWidget as that whose build function actually creates \
                     the Scaffold widget being sought.\n\
                     There are several ways to avoid this problem. The simplest is to use a \
                     Builder to get a context that is \"under\" the Scaffold. For an example of \
                     this, please see the documentation for Scaffold.of():\n  \
                     https://api.flutter.dev/flutter/material/Scaffold/of.html\n\
                     A more efficient solution is to split your build function into several \
                     widgets. This introduces a new context from which you can obtain the \
                     Scaffold. In this solution, you would have an outer widget that creates the \
                     Scaffold populated by instances of your new inner widgets, and then in these \
                     inner widgets you would use Scaffold.of().\n\
                     A less elegant but more expedient solution is assign a GlobalKey to the \
                     Scaffold, then use the key.currentState property to obtain the \
                     ScaffoldState rather than using the Scaffold.of() function.\n\
                     The context used was:\n"
                ),
                "{message}"
            );
            assert!(message.contains("Builder"), "{message}");
        });
    }

    // 'Call to Scaffold.geometryOf() without context'.
    #[test]
    fn geometry_of_without_a_scaffold_explains_how_to_reach_one() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            pump_app(&mut tester, |_app| keyed_builder(&key));
            let message = panic_at(&tester, &key, |app, context| {
                Scaffold::geometry_of(app, context);
            });
            assert!(
                message.starts_with(
                    "Scaffold.geometryOf() called with a context that does not contain a \
                     Scaffold.\n\
                     This usually happens when the context provided is from the same \
                     StatefulWidget as that whose build function actually creates the Scaffold \
                     widget being sought.\n\
                     There are several ways to avoid this problem. The simplest is to use a \
                     Builder to get a context that is \"under\" the Scaffold. For an example of \
                     this, please see the documentation for Scaffold.of():\n  \
                     https://api.flutter.dev/flutter/material/Scaffold/of.html\n\
                     A more efficient solution is to split your build function into several \
                     widgets. This introduces a new context from which you can obtain the \
                     Scaffold. In this solution, you would have an outer widget that creates the \
                     Scaffold populated by instances of your new inner widgets, and then in these \
                     inner widgets you would use Scaffold.geometryOf().\n\
                     The context used was:\n"
                ),
                "{message}"
            );
            assert!(message.contains("Builder"), "{message}");
        });
    }

    // 'FloatingActionButton always keeps the same position regardless of extendBodyBehindAppBar'.
    #[test]
    fn the_floating_action_button_keeps_its_place_when_the_body_extends_behind_the_app_bar() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let fab_center_y = |tester: &mut WidgetTester, extend_body_behind_app_bar: bool| {
                pump_app(tester, |_app| {
                    Scaffold::new()
                        .app_bar(PreferredSizeWidgetRef::new(AppBar::new()))
                        .floating_action_button(
                            FloatingActionButton::new(Some(Listener::new(|_| {})))
                                .child(waft_widgets::Icon::new(Some(crate::icons::Icons::add()))),
                        )
                        .floating_action_button_location(
                            <dyn FloatingActionButtonLocation>::end_top(),
                        )
                        .extend_body_behind_app_bar(extend_body_behind_app_bar)
                        .into_widget()
                });
                let fab = tester.find_by_type::<FloatingActionButton>()[0];
                let render = tester.render_box_of(fab);
                tester.global_rect(render).center().dy()
            };

            let default_offset_y = fab_center_y(&mut tester, false);
            let extended_body_offset_y = fab_center_y(&mut tester, true);

            assert_eq!(default_offset_y, extended_body_offset_y);
        });
    }

    /// `Directionality(MediaQuery(Scaffold(body: Builder(builder: (context) =>
    /// GestureDetector(key: tapTarget, onTap: on_tap(context), behavior: opaque, child:
    /// SizedBox(100, 100))))))`.
    fn tap_target_scaffold(
        tap_target: &GlobalKey,
        with_media_query: bool,
        on_tap: impl Fn(&mut App, BuildContext) + 'static,
    ) -> WidgetRef {
        let on_tap = Rc::new(on_tap);
        let tap_target = tap_target.clone();
        let scaffold = Scaffold::new().body(Builder::new(move |_app, context| {
            let on_tap = Rc::clone(&on_tap);
            waft_widgets::GestureDetector::new()
                .key(tap_target.clone().into())
                .on_tap(Listener::new(move |app| on_tap(app, context)))
                .behavior(HitTestBehavior::Opaque)
                .child(SizedBox::new().width(100.0).height(100.0))
                .into_widget()
        }));
        if with_media_query {
            directional(TextDirection::Ltr, MediaQueryData::new(), scaffold)
        } else {
            Directionality::new(TextDirection::Ltr, scaffold).into_widget()
        }
    }

    // 'ScaffoldMessenger.maybeOf can return null if not found'.
    #[test]
    fn scaffold_messenger_maybe_of_is_none_without_a_messenger() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let tap_target = GlobalKey::new();
            let scaffold_messenger = Rc::new(Cell::new(Some(None)));
            tester.pump_widget(tap_target_scaffold(&tap_target, true, {
                let scaffold_messenger = Rc::clone(&scaffold_messenger);
                move |app, context| {
                    scaffold_messenger.set(Some(ScaffoldMessenger::maybe_of(app, context)));
                }
            }));
            let at = tester.center_of(&tap_target);
            tester.tap(at, 1);
            tester.pump(Duration::ZERO);
            assert_eq!(scaffold_messenger.get(), Some(None));
        });
    }

    /// `TextButton(onPressed:, child: Text(label))`.
    fn text_button(label: &'static str, on_pressed: impl Fn(&mut App) + 'static) -> WidgetRef {
        TextButton::new(
            Some(Listener::new(on_pressed)),
            Some(waft_widgets::Text::new(label)),
        )
        .into_widget()
    }

    /// Dart's `tester.tap(find.text(label))`, which skips offstage widgets.
    fn tap_onstage_text(tester: &mut WidgetTester, label: &str, pointer: i64) {
        let found = tester.find_onstage_text(label);
        assert_eq!(found.len(), 1, "one onstage {label:?}");
        let at = tester.global_rect(tester.render_box_of(found[0])).center();
        tester.tap(at, pointer);
    }

    // 'ScaffoldMessenger checks for nesting when a new Scaffold is registered'. A
    // `PageRouteBuilder` stands in for the `MaterialPageRoute` Dart pushes, which is not ported.
    #[test]
    fn the_messenger_checks_for_nesting_when_a_new_scaffold_registers() {
        // Regression test for https://github.com/flutter/flutter/issues/77251
        with_main_thread_stack(|| {
            let mut tester = tester();
            const SNACK_BAR_CONTENT: &str = "SnackBar Content";
            pump_app(&mut tester, |_app| {
                Builder::new(|_app, context| {
                    Scaffold::new()
                        .body(Scaffold::new().body(text_button("Push route", move |app| {
                            let page_builder: waft_widgets::RoutePageBuilder =
                                Rc::new(|_app, context, _, _| {
                                    Scaffold::new()
                                        .body(Column::new().children([
                                            text_button("Show SnackBar", move |app| {
                                                let snack_bar = SnackBar::new(
                                                    waft_widgets::Text::new(SNACK_BAR_CONTENT),
                                                )
                                                .behavior(SnackBarBehavior::Floating);
                                                ScaffoldMessenger::of(app, context)
                                                    .show_snack_bar(app, snack_bar, None);
                                            }),
                                            text_button("Pop route", move |app| {
                                                waft_widgets::Navigator::pop(app, context, None);
                                            }),
                                        ]))
                                        .into_widget()
                                });
                            let route = waft_widgets::PageRouteBuilder::new(app, page_builder);
                            drop(waft_widgets::Navigator::push(
                                app,
                                context,
                                waft_widgets::Route::as_route(route),
                            ));
                        })))
                        .into_widget()
                })
                .into_widget()
            });

            let snack_bars =
                |tester: &WidgetTester| tester.find_onstage_text(SNACK_BAR_CONTENT).len();
            assert_eq!(snack_bars(&tester), 0);
            tap_onstage_text(&mut tester, "Push route", 1);
            tester.pump_and_settle();
            assert_eq!(snack_bars(&tester), 0);
            assert_eq!(tester.find_onstage_text("Pop route").len(), 1);

            // Show SnackBar on second page
            tap_onstage_text(&mut tester, "Show SnackBar", 2);
            tester.pump(Duration::ZERO);
            assert_eq!(snack_bars(&tester), 1);
            // Pop the second page, the SnackBar completes a hero animation to the next route.
            // If we have not handled the nested Scaffolds properly, this will throw an
            // exception as duplicate SnackBars on the first route would have a common hero tag.
            tap_onstage_text(&mut tester, "Pop route", 3);
            tester.pump(Duration::ZERO);
            // There are SnackBars two during the execution of the hero animation.
            assert_eq!(snack_bars(&tester), 2);
            tester.pump_and_settle();
            assert_eq!(snack_bars(&tester), 1);
            // Allow the SnackBar to animate out
            tester.pump(Duration::from_secs(4));
            tester.pump_and_settle();
            assert_eq!(snack_bars(&tester), 0);
        });
    }

    // 'ScaffoldMessenger.of will assert if not found'.
    #[test]
    fn scaffold_messenger_of_asserts_without_a_messenger() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let tap_target = GlobalKey::new();
            tester.pump_widget(tap_target_scaffold(&tap_target, false, |app, context| {
                ScaffoldMessenger::of(app, context);
            }));
            let at = tester.center_of(&tap_target);
            let outcome =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| tester.tap(at, 1)));
            let message = panic_message(outcome.expect_err("ScaffoldMessenger.of panics"));
            assert!(
                message.starts_with("No ScaffoldMessenger widget found.\n"),
                "{message}"
            );
            // Dart's `runtimeType` is the widget's `Debug` here, as in debug_check_has_material;
            // the missing-ancestor description waits (debug.rs's Deferred line).
            assert!(
                message.contains("\nBuilder")
                    && message.contains(" widgets require a ScaffoldMessenger widget ancestor.\n"),
                "{message}"
            );
            assert!(
                message.ends_with(
                    "Typically, the ScaffoldMessenger widget is introduced by the MaterialApp at \
                     the top of your application widget tree."
                ),
                "{message}"
            );
        });
    }

    // 'Default ScaffoldMessenger can access ambient theme'.
    #[test]
    fn the_default_scaffold_messenger_sees_the_apps_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let (theme, color_scheme) = {
                let app = tester.cell.borrow();
                let color_scheme =
                    crate::ColorScheme::from_seed(Colors::DEEP_PURPLE.primary()).build();
                let theme = ThemeData::builder()
                    .color_scheme(color_scheme.clone())
                    .visual_density(crate::VisualDensity::COMFORTABLE)
                    .build(&app);
                (theme, color_scheme)
            };
            tester.pump_widget(material_app(theme, SizedBox::shrink().into_widget()));

            let messenger = tester.find_by_type::<ScaffoldMessenger>()[0];
            let mut app = tester.cell.borrow_mut();
            let messenger_theme = Theme::of(&mut app, messenger);
            assert!(messenger_theme.color_scheme == color_scheme);
            assert_eq!(
                messenger_theme.visual_density,
                crate::VisualDensity::COMFORTABLE
            );
        });
    }

    /// `MaterialApp(home: Scaffold(body: Builder(builder: (context) => ElevatedButton(
    /// onPressed: show the snack bar, child: Text('Show SnackBar')))))`.
    fn pump_snack_bar_app(tester: &mut WidgetTester, style: Option<AnimationStyle>) {
        pump_app(tester, move |_app| {
            Scaffold::new()
                .body(Builder::new(move |_app, context| {
                    let style = style.clone();
                    ElevatedButton::new(
                        Some(Listener::new(move |app| {
                            ScaffoldMessenger::of(app, context).show_snack_bar(
                                app,
                                SnackBar::new(crate::test_support::AhemText::new(
                                    "I am a snack bar.",
                                ))
                                .show_close_icon(true),
                                style.clone(),
                            );
                        })),
                        Some(waft_widgets::Text::new("Show SnackBar")),
                    )
                    .into_widget()
                }))
                .into_widget()
        });
    }

    fn snack_bar_text_top(tester: &WidgetTester) -> f64 {
        let text = *tester
            .find_ahem_text("I am a snack bar.")
            .first()
            .expect("the snack bar's text");
        tester.global_rect(tester.render_box_of(text)).top
    }

    #[track_caller]
    fn assert_close_to(actual: f64, expected: f64, epsilon: f64) {
        assert!(
            (actual - expected).abs() <= epsilon,
            "{actual} is not {expected}"
        );
    }

    fn tap_close_icon(tester: &mut WidgetTester, pointer: i64) {
        let icon_button = tester.find_by_type::<crate::IconButton>()[0];
        let at = center_of_element(tester, icon_button);
        tester.tap(at, pointer);
    }

    // 'ScaffoldMessenger showSnackBar default animation'.
    #[test]
    fn the_snack_bar_slides_in_and_out_in_250_milliseconds_by_default() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_snack_bar_app(&mut tester, None);

            // Tap the button to show the SnackBar.
            tap_elevated_button(&mut tester, "Show SnackBar", 1);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(125)); // Advance the animation by 125ms.

            // The SnackBar is partially visible.
            assert_close_to(snack_bar_text_top(&tester), 576.7, 0.1);

            tester.pump(Duration::from_millis(125)); // Advance the animation by 125ms.

            // The SnackBar is fully visible.
            assert_close_to(snack_bar_text_top(&tester), 566.0, 0.1);

            // Tap the close button to dismiss the SnackBar.
            tap_close_icon(&mut tester, 2);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(125)); // Advance the animation by 125ms.

            // The SnackBar is partially visible.
            assert_close_to(snack_bar_text_top(&tester), 576.7, 0.1);

            tester.pump(Duration::from_millis(125)); // Advance the animation by 125ms.

            // The SnackBar is dismissed.
            assert_close_to(snack_bar_text_top(&tester), 614.0, 0.1);
        });
    }

    // 'ScaffoldMessenger showSnackBar animation can be customized'.
    #[test]
    fn the_snack_bar_animation_style_sets_its_durations() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_snack_bar_app(
                &mut tester,
                Some(AnimationStyle::new(
                    None,
                    Some(Duration::from_millis(1200)),
                    None,
                    Some(Duration::from_millis(600)),
                )),
            );

            // Tap the button to show the SnackBar.
            tap_elevated_button(&mut tester, "Show SnackBar", 1);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(300)); // Advance the animation by 300ms.

            // The SnackBar is partially visible.
            assert_close_to(snack_bar_text_top(&tester), 602.6, 0.1);

            tester.pump(Duration::from_millis(300)); // Advance the animation by 300ms.

            // The SnackBar is partially visible.
            assert_close_to(snack_bar_text_top(&tester), 576.7, 0.1);

            tester.pump(Duration::from_millis(600)); // Advance the animation by 600ms.

            // The SnackBar is fully visible.
            assert_close_to(snack_bar_text_top(&tester), 566.0, 0.1);

            // Tap the close button to dismiss the SnackBar.
            tap_close_icon(&mut tester, 2);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(300)); // Advance the animation by 300ns.

            // The SnackBar is partially visible.
            assert_close_to(snack_bar_text_top(&tester), 576.7, 0.1);

            tester.pump(Duration::from_millis(300)); // Advance the animation by 300ms.

            // The SnackBar is dismissed.
            assert_close_to(snack_bar_text_top(&tester), 614.0, 0.1);
        });
    }

    // 'Updated snackBarAnimationStyle updates snack bar animation'.
    #[test]
    fn a_new_snack_bar_animation_style_replaces_the_old_durations() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            // Test custom animation style.
            pump_snack_bar_app(
                &mut tester,
                Some(AnimationStyle::new(
                    None,
                    Some(Duration::from_millis(800)),
                    None,
                    Some(Duration::from_millis(400)),
                )),
            );

            // Tap the button to show the SnackBar.
            tap_elevated_button(&mut tester, "Show SnackBar", 1);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(400)); // Advance the animation by 400ms.

            // The SnackBar is partially visible.
            assert_close_to(snack_bar_text_top(&tester), 576.7, 0.1);

            tester.pump(Duration::from_millis(400)); // Advance the animation by 400ms.

            // The SnackBar is fully visible.
            assert_close_to(snack_bar_text_top(&tester), 566.0, 0.1);

            // Tap the close button to dismiss the SnackBar.
            tap_close_icon(&mut tester, 2);
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_millis(400)); // Advance the animation by 400ms.

            // The SnackBar is dismissed.
            assert_close_to(snack_bar_text_top(&tester), 614.0, 0.1);

            // Test no animation style.
            pump_snack_bar_app(&mut tester, Some(AnimationStyle::NO_ANIMATION));
            tester.pump_and_settle();

            // Tap the button to show the SnackBar.
            tap_elevated_button(&mut tester, "Show SnackBar", 3);
            tester.pump(Duration::ZERO);

            // The SnackBar is fully visible.
            assert_close_to(snack_bar_text_top(&tester), 566.0, 0.1);

            // Tap the close button to dismiss the SnackBar.
            tap_close_icon(&mut tester, 4);
            tester.pump(Duration::ZERO);

            // The SnackBar is dismissed.
            assert!(tester.find_ahem_text("I am a snack bar.").is_empty());
        });
    }

    // 'snackBarAnimationStyle with only reverseDuration uses default forward duration'.
    #[test]
    fn a_snack_bar_animation_style_with_only_a_reverse_duration_keeps_the_default_forward_one() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            // Test custom animation style with only reverseDuration.
            pump_snack_bar_app(
                &mut tester,
                Some(AnimationStyle::new(
                    None,
                    None,
                    None,
                    Some(Duration::from_millis(400)),
                )),
            );

            // Tap the button to show the SnackBar.
            tap_elevated_button(&mut tester, "Show SnackBar", 1);
            tester.pump(Duration::ZERO);
            // Advance the animation by 1/2 of the default forward duration.
            tester.pump(Duration::from_millis(125));

            // The SnackBar is partially visible.
            assert_close_to(snack_bar_text_top(&tester), 576.7, 0.1);

            // Advance the animation by 1/2 of the default forward duration.
            tester.pump(Duration::from_millis(125)); // Advance the animation by 125ms.

            // The SnackBar is fully visible.
            assert_close_to(snack_bar_text_top(&tester), 566.0, 0.1);

            // Tap the close button to dismiss the SnackBar.
            tap_close_icon(&mut tester, 2);
            tester.pump(Duration::ZERO);
            // Advance the animation by 1/2 of the reverse duration.
            tester.pump(Duration::from_millis(200));

            // The SnackBar is partially visible.
            assert_close_to(snack_bar_text_top(&tester), 576.7, 0.1);

            // Advance the animation by 1/2 of the reverse duration.
            tester.pump(Duration::from_millis(200)); // Advance the animation by 200ms.

            // The SnackBar is dismissed.
            assert_close_to(snack_bar_text_top(&tester), 614.0, 0.1);
        });
    }

    fn show_sheet(
        app: &mut App,
        scaffold: Handle<ScaffoldState>,
        builder: impl Fn(&mut App, BuildContext) -> WidgetRef + 'static,
        sheet_animation_style: Option<AnimationStyle>,
    ) -> Rc<PersistentBottomSheetController> {
        scaffold.show_bottom_sheet(
            app,
            Rc::new(builder),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            sheet_animation_style,
        )
    }

    // 'showBottomSheet() while Scaffold has bottom sheet'.
    #[test]
    fn show_bottom_sheet_while_the_scaffold_has_a_bottom_sheet_asserts() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            pump_app(&mut tester, |_app| {
                Scaffold::new()
                    .key(key.clone().into())
                    .body(Center::new().child(Container::new()))
                    .bottom_sheet(waft_widgets::Text::new("Bottom sheet"))
                    .into_widget()
            });
            let scaffold = scaffold_state(&tester, &key);
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                show_sheet(
                    &mut tester.cell.borrow_mut(),
                    scaffold,
                    |_app, _context| Container::new().into_widget(),
                    None,
                );
            }));
            let message = panic_message(outcome.expect_err("showBottomSheet asserts"));
            assert_eq!(
                message,
                "Scaffold.bottomSheet cannot be specified while a bottom sheet displayed with \
                 showBottomSheet() is still visible.\n\
                 Rebuild the Scaffold with a null bottomSheet before calling showBottomSheet()."
            );
        });
    }

    // 'didUpdate bottomSheet while a previous bottom sheet is still displayed'.
    #[test]
    fn giving_the_scaffold_a_bottom_sheet_while_a_shown_one_is_visible_asserts() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            let button_key = GlobalKey::new();
            let state = Rc::new(Cell::new(0));
            pump_app(&mut tester, |_app| {
                let state = Rc::clone(&state);
                let (key, button_key) = (key.clone(), button_key.clone());
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let state_on_press = Rc::clone(&state);
                    let mut scaffold = Scaffold::new()
                        .key(key.clone().into())
                        .body(Container::new())
                        .floating_action_button(
                            FloatingActionButton::new(Some(Listener::new(move |app| {
                                state_on_press.set(state_on_press.get() + 1);
                                set_state(app, &mut |_app| {});
                            })))
                            .key(button_key.clone().into()),
                        );
                    if state.get() != 0 {
                        scaffold = scaffold.bottom_sheet(SizedBox::new());
                    }
                    scaffold.into_widget()
                })
                .into_widget()
            });
            let scaffold = scaffold_state(&tester, &key);
            show_sheet(
                &mut tester.cell.borrow_mut(),
                scaffold,
                |_app, _context| Container::new().into_widget(),
                None,
            );
            let at = tester.center_of(&button_key);
            tester.tap(at, 1);
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                tester.pump(Duration::ZERO);
            }));
            let message = panic_message(outcome.expect_err("didUpdateWidget asserts"));
            assert_eq!(
                message,
                "Scaffold.bottomSheet cannot be specified while a bottom sheet displayed with \
                 showBottomSheet() is still visible.\n\
                 Use the PersistentBottomSheetController returned by showBottomSheet() to close \
                 the old bottom sheet before creating a Scaffold with a (non null) bottomSheet."
            );
        });
    }

    /// `Scaffold(body: Builder(builder: (context) => GestureDetector(onTap: () =>
    /// Scaffold.of(context).showBottomSheet(..), child: Text('X'))))` with Dart's sheet: a
    /// `ColoredBox` keyed `sheet_key` filling the sheet, around a `FilledButton` that pops.
    fn pump_bottom_sheet_app(
        tester: &mut WidgetTester,
        sheet_key: &GlobalKey,
        sheet_animation_style: Option<AnimationStyle>,
    ) {
        let sheet_key = sheet_key.clone();
        pump_app(tester, move |_app| {
            Scaffold::new()
                .body(Builder::new(move |_app, context| {
                    let sheet_key = sheet_key.clone();
                    let sheet_animation_style = sheet_animation_style.clone();
                    waft_widgets::GestureDetector::new()
                        .on_tap(Listener::new(move |app| {
                            let sheet_key = sheet_key.clone();
                            let scaffold = Scaffold::of(app, context);
                            show_sheet(
                                app,
                                scaffold,
                                move |app, context| {
                                    let primary = Theme::of(app, context).color_scheme.primary;
                                    SizedBox::expand()
                                        .child(
                                            ColoredBox::new(primary)
                                                .key(sheet_key.clone().into())
                                                .child(crate::FilledButton::new(
                                                    Some(Listener::new(move |app| {
                                                        waft_widgets::Navigator::pop(
                                                            app, context, None,
                                                        )
                                                    })),
                                                    Some(waft_widgets::Text::new("Close")),
                                                )),
                                        )
                                        .into_widget()
                                },
                                sheet_animation_style.clone(),
                            );
                        }))
                        .child(waft_widgets::Text::new("X"))
                        .into_widget()
                }))
                .into_widget()
        });
    }

    fn tap_text(tester: &mut WidgetTester, text: &str, pointer: i64) {
        let element = *tester.find_text(text).first().expect("the text");
        let at = center_of_element(tester, element);
        tester.tap(at, pointer);
    }

    // 'Scaffold showBottomSheet default animation'.
    #[test]
    fn show_bottom_sheet_animates_in_250_and_out_200_milliseconds_by_default() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let sheet_key = GlobalKey::new();
            pump_bottom_sheet_app(&mut tester, &sheet_key, None);

            // Tap the 'X' to show the bottom sheet.
            tap_text(&mut tester, "X", 1);
            tester.pump(Duration::ZERO);
            // Advance the animation by 1/2 of the default forward duration.
            tester.pump(Duration::from_millis(125));

            // The bottom sheet is partially visible.
            assert_close_to(rect_of(&tester, &sheet_key).top, 134.6, 0.1);

            // Advance the animation by 1/2 of the default forward duration.
            tester.pump(Duration::from_millis(125));

            // The bottom sheet is fully visible.
            assert_eq!(rect_of(&tester, &sheet_key).top, 0.0);

            // Dismiss the bottom sheet.
            tap_text(&mut tester, "Close", 2);
            tester.pump(Duration::ZERO);
            // Advance the animation by 1/2 of the default reverse duration.
            tester.pump(Duration::from_millis(100));

            // The bottom sheet is partially visible.
            assert_close_to(rect_of(&tester, &sheet_key).top, 134.6, 0.1);

            // Advance the animation by 1/2 of the default reverse duration.
            tester.pump(Duration::from_millis(100));

            // The bottom sheet is dismissed.
            assert_eq!(rect_of(&tester, &sheet_key).top, 600.0);
        });
    }

    // 'Scaffold showBottomSheet animation can be customized'.
    #[test]
    fn the_sheet_animation_style_sets_show_bottom_sheets_durations() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let sheet_key = GlobalKey::new();
            // Test custom animation style.
            pump_bottom_sheet_app(
                &mut tester,
                &sheet_key,
                Some(AnimationStyle::new(
                    None,
                    Some(Duration::from_millis(800)),
                    None,
                    Some(Duration::from_millis(400)),
                )),
            );
            tap_text(&mut tester, "X", 1);
            tester.pump(Duration::ZERO);
            // Advance the animation by 1/2 of the custom forward duration.
            tester.pump(Duration::from_millis(400));

            // The bottom sheet is partially visible.
            assert_close_to(rect_of(&tester, &sheet_key).top, 134.6, 0.1);

            // Advance the animation by 1/2 of the custom forward duration.
            tester.pump(Duration::from_millis(400));

            // The bottom sheet is fully visible.
            assert_eq!(rect_of(&tester, &sheet_key).top, 0.0);

            // Dismiss the bottom sheet.
            tap_text(&mut tester, "Close", 2);
            tester.pump(Duration::ZERO);
            // Advance the animation by 1/2 of the custom reverse duration.
            tester.pump(Duration::from_millis(200));

            // The bottom sheet is partially visible.
            assert_close_to(rect_of(&tester, &sheet_key).top, 134.6, 0.1);

            // Advance the animation by 1/2 of the custom reverse duration.
            tester.pump(Duration::from_millis(200));

            // The bottom sheet is dismissed.
            assert_eq!(rect_of(&tester, &sheet_key).top, 600.0);

            // Test no animation style.
            pump_bottom_sheet_app(&mut tester, &sheet_key, Some(AnimationStyle::NO_ANIMATION));
            tester.pump_and_settle();
            tap_text(&mut tester, "X", 3);
            tester.pump(Duration::ZERO);

            // The bottom sheet is fully visible.
            assert_eq!(rect_of(&tester, &sheet_key).top, 0.0);

            // Dismiss the bottom sheet.
            tap_text(&mut tester, "Close", 4);
            tester.pump(Duration::ZERO);

            // The bottom sheet is dismissed.
            assert!(
                sheet_key
                    .current_context(&mut tester.cell.borrow_mut())
                    .is_none()
            );
        });
    }

    fn is_mounted(tester: &WidgetTester, key: &GlobalKey) -> bool {
        key.current_context(&mut tester.cell.borrow_mut()).is_some()
    }

    // 'can rebuild and remove bottomSheet at the same time'.
    #[test]
    fn a_bottom_sheet_on_its_way_out_survives_a_rebuild() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme_is_light = Rc::new(Cell::new(true));
            let default_bottom_sheet = Rc::new(Cell::new(Some(true)));
            let bottom_sheet_key1 = GlobalKey::new();
            let bottom_sheet_key2 = GlobalKey::new();
            let set_state: Rc<RefCell<Option<StateSetter>>> = Rc::new(RefCell::new(None));

            let root = StatefulBuilder::new({
                let theme_is_light = Rc::clone(&theme_is_light);
                let default_bottom_sheet = Rc::clone(&default_bottom_sheet);
                let set_state = Rc::clone(&set_state);
                let (key1, key2) = (bottom_sheet_key1.clone(), bottom_sheet_key2.clone());
                move |app, _context, state_setter| {
                    *set_state.borrow_mut() = Some(state_setter);
                    let theme = if theme_is_light.get() {
                        ThemeData::new(app)
                    } else {
                        ThemeData::dark(app, None)
                    };
                    let sheet = |key: &GlobalKey, color: Color| {
                        Container::new()
                            .key(key.clone().into())
                            .width(f64::INFINITY)
                            .height(100.0)
                            .color(color)
                            .child(waft_widgets::Text::new("BottomSheet"))
                            .into_widget()
                    };
                    let mut scaffold = Scaffold::new().body(placeholder(None));
                    match default_bottom_sheet.get() {
                        None => {}
                        Some(true) => {
                            scaffold = scaffold.bottom_sheet(sheet(&key1, Colors::BLUE.primary()))
                        }
                        Some(false) => {
                            scaffold = scaffold.bottom_sheet(sheet(&key2, Colors::RED.primary()))
                        }
                    }
                    // Dart's MaterialApp animates a theme change, which is what keeps the sheet
                    // from rebuilding until its exit animation ends.
                    material_app(theme, scaffold.into_widget())
                }
            })
            .into_widget();
            tester.pump_widget(root);
            let set = |tester: &WidgetTester, change: &mut dyn FnMut(&mut App)| {
                let set_state = set_state.borrow().clone().expect("built");
                set_state(&mut tester.cell.borrow_mut(), change);
            };

            assert!(is_mounted(&tester, &bottom_sheet_key1));
            assert!(!is_mounted(&tester, &bottom_sheet_key2));

            // Change to the other bottomSheet.
            set(&tester, &mut |_app| default_bottom_sheet.set(Some(false)));
            assert!(is_mounted(&tester, &bottom_sheet_key1));
            assert!(!is_mounted(&tester, &bottom_sheet_key2));
            tester.pump_and_settle();
            assert!(!is_mounted(&tester, &bottom_sheet_key1));
            assert!(is_mounted(&tester, &bottom_sheet_key2));

            // Set bottomSheet to null, which starts its exit animation.
            set(&tester, &mut |_app| default_bottom_sheet.set(None));
            assert!(!is_mounted(&tester, &bottom_sheet_key1));
            assert!(is_mounted(&tester, &bottom_sheet_key2));

            // While the bottomSheet is on the way out, change the theme to cause it to
            // rebuild.
            set(&tester, &mut |_app| theme_is_light.set(false));
            assert!(!is_mounted(&tester, &bottom_sheet_key1));
            assert!(is_mounted(&tester, &bottom_sheet_key2));

            // The most recent bottomSheet remains on screen during the exit animation.
            tester.pump(Duration::from_millis(200));
            assert!(!is_mounted(&tester, &bottom_sheet_key1));
            assert!(is_mounted(&tester, &bottom_sheet_key2));

            // After animating out, the bottomSheet is gone.
            tester.pump_and_settle();
            assert!(!is_mounted(&tester, &bottom_sheet_key1));
            assert!(!is_mounted(&tester, &bottom_sheet_key2));
        });
    }

    fn draggable_sheet(
        controller: Handle<DraggableScrollableController>,
        child: fn() -> WidgetRef,
    ) -> DraggableScrollableSheet {
        DraggableScrollableSheet::new(Rc::new(move |_app, _context, scroll_controller| {
            SingleChildScrollView::new()
                .controller(scroll_controller)
                .child(child())
                .into_widget()
        }))
        .expand(false)
        .controller(controller)
    }

    /// Dart's `find.descendant(of: find.byType(Scaffold), matching: find.byType(ModalBarrier))`.
    fn modal_barriers(tester: &WidgetTester) -> Vec<waft_widgets::AnyElement> {
        let scaffold = tester.find_by_type::<Scaffold>()[0];
        tester
            .find_by_type::<ModalBarrier>()
            .into_iter()
            .filter(|barrier| tester.ancestors(*barrier).contains(&scaffold))
            .collect()
    }

    // 'showBottomSheet removes scrim when draggable sheet is dismissed'.
    #[test]
    fn closing_a_shown_draggable_sheet_removes_its_scrim() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let draggable_controller =
                DraggableScrollableController::new(&mut tester.cell.borrow_mut());
            let scaffold_key = GlobalKey::new();
            pump_app(&mut tester, |_app| {
                Scaffold::new()
                    .key(scaffold_key.clone().into())
                    .body(Center::new().child(waft_widgets::Text::new("body")))
                    .into_widget()
            });

            let scaffold = scaffold_state(&tester, &scaffold_key);
            let sheet_controller = show_sheet(
                &mut tester.cell.borrow_mut(),
                scaffold,
                move |_app, _context| {
                    draggable_sheet(draggable_controller, || placeholder(None)).into_widget()
                },
                None,
            );

            tester.pump(Duration::ZERO);
            assert_eq!(tester.find_by_type::<BottomSheet>().len(), 1);

            // The scrim is not present yet.
            assert!(modal_barriers(&tester).is_empty());

            // Expand the sheet to 80% of parent height to show the scrim.
            draggable_controller.jump_to(&mut tester.cell.borrow_mut(), 0.8);
            tester.pump(Duration::ZERO);
            assert_eq!(modal_barriers(&tester).len(), 1);

            // Dismiss the sheet.
            sheet_controller.close(&mut tester.cell.borrow_mut());
            tester.pump_and_settle();

            // The scrim should be gone.
            assert!(modal_barriers(&tester).is_empty());
        });
    }

    const K_BOTTOM_SHEET_DOMINATES: f64 = 0.3;

    // 'Floating action button shrinks when bottom sheet becomes dominant'.
    #[test]
    fn the_floating_action_button_shrinks_as_a_draggable_bottom_sheet_dominates() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let draggable_controller =
                DraggableScrollableController::new(&mut tester.cell.borrow_mut());
            pump_app(&mut tester, |_app| {
                Scaffold::new()
                    .floating_action_button(
                        FloatingActionButton::new(None)
                            .key(KeyRef::Local(Rc::new(ValueKey::new("one".to_string()))))
                            .child(waft_widgets::Text::new("1")),
                    )
                    .bottom_sheet(draggable_sheet(draggable_controller, || {
                        SizedBox::new().into_widget()
                    }))
                    .into_widget()
            });

            let get_scale = |tester: &WidgetTester| {
                let scale = tester.find_by_type::<waft_widgets::Animated<ScaleTransition>>()[0];
                let scale = tester
                    .widget(scale, |scale: &waft_widgets::Animated<ScaleTransition>| {
                        scale.0.scale()
                    });
                scale.value(&tester.cell.borrow())
            };

            for i in 0..=10 {
                let extent = f64::from(i) / 10.0;
                draggable_controller.jump_to(&mut tester.cell.borrow_mut(), extent);

                let extent_remaining = 1.0 - extent;
                if extent_remaining < K_BOTTOM_SHEET_DOMINATES {
                    let vis_value = extent_remaining * K_BOTTOM_SHEET_DOMINATES * 10.0;
                    // since FAB uses easeIn curve, we're testing this by using the fact that
                    // easeIn curve is always less than or equal to x=y curve.
                    assert!(get_scale(&tester) <= vis_value, "extent {extent}");
                } else {
                    assert_eq!(get_scale(&tester), 1.0, "extent {extent}");
                }
            }
        });
    }

    // 'Scaffold shows scrim when bottom sheet becomes dominant'.
    #[test]
    fn the_scaffold_shows_a_scrim_as_a_draggable_bottom_sheet_dominates() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let draggable_controller =
                DraggableScrollableController::new(&mut tester.cell.borrow_mut());
            const K_MIN_BOTTOM_SHEET_SCRIM_OPACITY: f64 = 0.1;
            const K_MAX_BOTTOM_SHEET_SCRIM_OPACITY: f64 = 0.6;
            pump_app(&mut tester, |_app| {
                Scaffold::new()
                    .bottom_sheet(draggable_sheet(draggable_controller, || {
                        SizedBox::new().into_widget()
                    }))
                    .into_widget()
            });

            let get_opacity = |tester: &WidgetTester| {
                let barrier = modal_barriers(tester)[0];
                tester.widget(barrier, |barrier: &ModalBarrier| {
                    barrier.color.expect("a coloured barrier").a
                })
            };
            let get_expected_opacity = |vis_value: f64| {
                f64::max(
                    K_MIN_BOTTOM_SHEET_SCRIM_OPACITY,
                    K_MAX_BOTTOM_SHEET_SCRIM_OPACITY - vis_value,
                )
            };

            for i in 0..=10 {
                let extent = f64::from(i) / 10.0;
                draggable_controller.jump_to(&mut tester.cell.borrow_mut(), extent);
                tester.pump(Duration::ZERO);

                let extent_remaining = 1.0 - extent;
                if extent_remaining < K_BOTTOM_SHEET_DOMINATES {
                    let vis_value = extent_remaining * K_BOTTOM_SHEET_DOMINATES * 10.0;

                    assert_eq!(modal_barriers(&tester).len(), 1, "extent {extent}");
                    assert_close_to(get_opacity(&tester), get_expected_opacity(vis_value), 0.02);
                } else {
                    assert!(modal_barriers(&tester).is_empty(), "extent {extent}");
                }
            }
        });
    }

    // 'Scaffold uses bottomSheetScrimBuilder if defined'.
    #[test]
    fn the_scaffold_builds_its_scrim_with_the_bottom_sheet_scrim_builder() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let draggable_controller =
                DraggableScrollableController::new(&mut tester.cell.borrow_mut());
            let scrim_key = GlobalKey::new();
            pump_app(&mut tester, |_app| {
                let scrim_key = scrim_key.clone();
                Scaffold::new()
                    .bottom_sheet_scrim_builder(move |_app, _context, animation| {
                        let scrim_key = scrim_key.clone();
                        Some(
                            ListenableBuilder::new(
                                Rc::new(animation),
                                move |app, _context, _child| {
                                    #[allow(deprecated)]
                                    let color = Colors::BLACK.with_opacity(animation.value(app));
                                    ColoredBox::new(color)
                                        .key(scrim_key.clone().into())
                                        .into_widget()
                                },
                            )
                            .into_widget(),
                        )
                    })
                    .bottom_sheet(draggable_sheet(draggable_controller, || {
                        SizedBox::new().into_widget()
                    }))
                    .into_widget()
            });

            let get_opacity = |tester: &WidgetTester| {
                let scrim = scrim_key
                    .current_context(&mut tester.cell.borrow_mut())
                    .expect("a scrim");
                tester.widget(scrim, |scrim: &ColoredBox| scrim.color.color().a)
            };

            for i in 0..=10 {
                let extent = f64::from(i) / 10.0;
                draggable_controller.jump_to(&mut tester.cell.borrow_mut(), extent);
                tester.pump(Duration::ZERO);

                let extent_remaining = 1.0 - extent;
                if extent_remaining < K_BOTTOM_SHEET_DOMINATES {
                    let animation_value = 1.0 - extent_remaining / K_BOTTOM_SHEET_DOMINATES;

                    assert!(modal_barriers(&tester).is_empty(), "extent {extent}");
                    assert!(is_mounted(&tester, &scrim_key), "extent {extent}");
                    assert_close_to(get_opacity(&tester), animation_value, 0.02);
                } else {
                    assert!(!is_mounted(&tester, &scrim_key), "extent {extent}");
                }
            }
        });
    }

    // "Closing bottom sheet & removing FAB at the same time doesn't throw assertion".
    #[test]
    fn closing_a_bottom_sheet_while_removing_the_floating_action_button_does_not_assert() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let bottom_sheet_key = GlobalKey::new();
            let controller: Rc<RefCell<Option<Rc<PersistentBottomSheetController>>>> =
                Rc::new(RefCell::new(None));
            let show = Rc::new(Cell::new(true));
            let fab_key = GlobalKey::new();
            let root = StatefulBuilder::new({
                let controller = Rc::clone(&controller);
                let show = Rc::clone(&show);
                let bottom_sheet_key = bottom_sheet_key.clone();
                let fab_key = fab_key.clone();
                move |app, _context, set_state| {
                    let controller = Rc::clone(&controller);
                    let bottom_sheet_key = bottom_sheet_key.clone();
                    let mut scaffold = Scaffold::new().body(Center::new().child(Builder::new(
                        move |_app, context| {
                            let controller = Rc::clone(&controller);
                            let bottom_sheet_key = bottom_sheet_key.clone();
                            ElevatedButton::new(
                                Some(Listener::new(move |app| {
                                    let current = controller.borrow_mut().take();
                                    match current {
                                        None => {
                                            let bottom_sheet_key = bottom_sheet_key.clone();
                                            let shown = crate::show_bottom_sheet(
                                                app,
                                                context,
                                                Rc::new(move |_app, _context| {
                                                    Container::new()
                                                        .key(bottom_sheet_key.clone().into())
                                                        .height(200.0)
                                                        .into_widget()
                                                }),
                                                None,
                                                None,
                                                None,
                                                None,
                                                None,
                                                None,
                                                None,
                                                None,
                                                None,
                                            );
                                            *controller.borrow_mut() = Some(shown);
                                        }
                                        Some(current) => current.close(app),
                                    }
                                })),
                                Some(waft_widgets::Text::new("BottomSheet")),
                            )
                            .into_widget()
                        },
                    )));
                    if show.get() {
                        let show = Rc::clone(&show);
                        scaffold = scaffold.floating_action_button(
                            FloatingActionButton::new(Some(Listener::new(move |app| {
                                let show = Rc::clone(&show);
                                set_state(app, &mut move |_app| show.set(false));
                            })))
                            .key(fab_key.clone().into()),
                        );
                    }
                    material_app(ThemeData::new(app), scaffold.into_widget())
                }
            })
            .into_widget();
            tester.pump_widget(root);

            // Show bottom sheet.
            tap_elevated_button(&mut tester, "BottomSheet", 1);
            tester.pump_and_settle();

            // Bottom sheet and FAB are visible.
            assert_eq!(tester.find_by_type::<FloatingActionButton>().len(), 1);
            assert!(is_mounted(&tester, &bottom_sheet_key));

            // Close bottom sheet while removing FAB.
            let at = tester.center_of(&fab_key);
            tester.tap(at, 2);
            tester.pump(Duration::ZERO); // start animation
            tap_elevated_button(&mut tester, "BottomSheet", 3);
            // Let the animation finish.
            tester.pump_and_settle();

            // Bottom sheet and FAB are gone.
            assert!(tester.find_by_type::<FloatingActionButton>().is_empty());
            assert!(!is_mounted(&tester, &bottom_sheet_key));
        });
    }

    /// Dart's `buildStatusBarTestApp`.
    fn build_status_bar_test_app(tester: &mut WidgetTester) {
        pump_app(tester, |_app| {
            MediaQuery::new(
                // status bar
                MediaQueryData::new().padding(EdgeInsets::only(0.0, 25.0, 0.0, 0.0)),
                Scaffold::new().body(
                    waft_widgets::CustomScrollView::new()
                        .primary(true)
                        .slivers([
                            crate::app_bar::SliverAppBar::new()
                                .title(waft_widgets::Text::new("Title"))
                                .into_widget(),
                            waft_widgets::SliverList::builder(|_app, _context, index| {
                                Some(
                                    SizedBox::new()
                                        .height(100.0)
                                        .child(waft_widgets::Text::new(format!("{index}")))
                                        .into_widget(),
                                )
                            })
                            .item_count(20)
                            .build()
                            .into_widget(),
                        ]),
                ),
            )
            .into_widget()
        });
    }

    /// Dart's `tester.state<ScrollableState>(find.byType(Scrollable)).position`.
    fn scrollable_position(tester: &WidgetTester) -> waft_widgets::AnyScrollPosition {
        let scrollables = tester.find_by_type::<waft_widgets::Scrollable>();
        assert_eq!(scrollables.len(), 1, "one Scrollable");
        let app = tester.cell.borrow();
        scrollables[0]
            .state_handle::<waft_widgets::ScrollableState>(&app)
            .expect("a Scrollable element holds a ScrollableState")
            .position(&app)
    }

    // 'Tapping the status bar scrolls to top with ease out curve animation': the binding has
    // no status-bar-tap callback yet, so the case calls `ScaffoldState::handle_status_bar_tap`
    // where Dart's `simulateStatusBarTap` reaches it through the binding.
    #[test]
    fn tapping_the_status_bar_scrolls_the_primary_scroll_view_to_the_top_easing_out() {
        with_main_thread_stack(|| {
            let mut tester = tester_on(TargetPlatform::IOS);
            const DURATION: u64 = 1000;
            let stops = [0.842, 0.959, 0.993, 1.0];
            const SCROLL_OFFSET: f64 = 1000.0;

            build_status_bar_test_app(&mut tester);
            let position = scrollable_position(&tester);
            position.jump_to(&mut tester.cell.borrow_mut(), SCROLL_OFFSET);

            let scaffold = only_scaffold_state(&tester);
            scaffold.handle_status_bar_tap(&mut tester.cell.borrow_mut());
            tester.pump(Duration::ZERO);
            assert_eq!(position.pixels(&tester.cell.borrow()), SCROLL_OFFSET);

            for (i, stop) in stops.iter().enumerate() {
                tester.pump(Duration::from_millis(DURATION / stops.len() as u64));
                // Scroll pixel position is very long double, compare with floored int
                // pixel position
                assert_eq!(
                    position.pixels(&tester.cell.borrow()) as i64,
                    (SCROLL_OFFSET * (1.0 - stop)) as i64,
                    "stop {i}"
                );
            }

            // Finally stops at the top.
            assert_eq!(position.pixels(&tester.cell.borrow()), 0.0);
        });
    }

    /// Dart's `tester.state<ScaffoldState>(find.byType(Scaffold))`.
    fn only_scaffold_state(tester: &WidgetTester) -> Handle<ScaffoldState> {
        let scaffolds = tester.find_by_type::<Scaffold>();
        assert_eq!(scaffolds.len(), 1, "one Scaffold");
        scaffolds[0]
            .state_handle::<ScaffoldState>(&tester.cell.borrow())
            .expect("a Scaffold element holds a ScaffoldState")
    }

    /// A location as a `ValueNotifier`'s value, equal by identity as Dart's locations are.
    #[derive(Clone)]
    struct LocationValue(FloatingActionButtonLocationRef);

    impl PartialEq for LocationValue {
        fn eq(&self, other: &LocationValue) -> bool {
            Rc::ptr_eq(&self.0, &other.0)
        }
    }

    /// Whether the `Builder` that builds the scaffold's `CustomMultiChildLayout`, under its
    /// `ScrollNotificationObserver`, is dirty: Dart's `checkDirty`.
    fn scaffold_layout_builder_dirty(app: &mut App) -> Option<bool> {
        let root = WidgetsBinding::instance(app).root_element(app)?;
        let mut pending = vec![root];
        let mut layout = None;
        while let Some(element) = pending.pop() {
            if downcast_widget::<CustomMultiChildLayout>(&**element.widget(app)).is_some() {
                layout = Some(element);
                break;
            }
            element.visit_children(app, &mut |child| pending.push(child));
        }
        let layout = layout?;
        let mut builder = None;
        let mut ancestor = layout.parent(app);
        while let Some(current) = ancestor {
            if downcast_widget::<ScrollNotificationObserver>(&**current.widget(app)).is_some() {
                break;
            }
            if builder.is_none() && downcast_widget::<Builder>(&**current.widget(app)).is_some() {
                builder = Some(current);
            }
            ancestor = current.parent(app);
        }
        builder.map(|builder| builder.dirty(app))
    }

    // 'Scaffold FAB animates without rebuilding'.
    #[test]
    fn moving_the_floating_action_button_does_not_rebuild_the_scaffolds_layout() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let dirty = Rc::new(Cell::new(None));
            let fab_location =
                tester
                    .cell
                    .borrow_mut()
                    .create(waft_foundation::ValueNotifier::new(LocationValue(
                        <dyn FloatingActionButtonLocation>::center_docked(),
                    )));
            let builder_key = GlobalKey::new();
            pump_app(&mut tester, |_app| {
                let dirty = Rc::clone(&dirty);
                waft_widgets::ValueListenableBuilder::new(
                    Rc::new(fab_location),
                    move |app, _context, location: &LocationValue, _child| {
                        dirty.set(scaffold_layout_builder_dirty(app));

                        Scaffold::new()
                            .body(SizedBox::expand())
                            .floating_action_button(FloatingActionButton::new(Some(Listener::new(
                                |_| {},
                            ))))
                            .floating_action_button_location(Rc::clone(&location.0))
                            .into_widget()
                    },
                )
                .key(builder_key.clone().into())
                .into_widget()
            });

            // Switch the FAB location to trigger the animation.
            fab_location.set_value(
                &mut tester.cell.borrow_mut(),
                LocationValue(<dyn FloatingActionButtonLocation>::end_float()),
            );
            tester.pump(crate::Durations::SHORT1);

            // Trigger a rebuild to update the "dirty" value.
            {
                let mut app = tester.cell.borrow_mut();
                let element = builder_key.current_context(&mut app).expect("mounted");
                element.mark_needs_build(&mut app);
            }
            tester.pump(crate::Durations::SHORT1);

            // The element that builds the Scaffold's CustomMultiChildLayout should not be dirty.
            assert_eq!(dirty.get(), Some(false));
        });
    }

    // The cases that waited for `AppBar`.

    /// `Scaffold(appBar: AppBar(title: Text('Title')), ..)`.
    fn with_title_app_bar(scaffold: Scaffold) -> Scaffold {
        scaffold.app_bar(PreferredSizeWidgetRef::new(
            AppBar::new().title(waft_widgets::Text::new("Title")),
        ))
    }

    /// Dart's `boilerplate`: `Localizations(en-us, ..)` and a left-to-right `Directionality`.
    fn boilerplate(child: WidgetRef) -> WidgetRef {
        waft_widgets::Localizations::new(
            waft_embedder::Locale::new("en").country_code("us"),
            vec![
                waft_widgets::DefaultWidgetsLocalizations::delegate(),
                crate::material_localizations::DefaultMaterialLocalizations::delegate(),
            ],
        )
        .child(Directionality::new(TextDirection::Ltr, child))
        .into_widget()
    }

    // 'Scaffold control test'.
    #[test]
    fn the_app_bar_takes_its_height_from_the_top_of_the_body() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let body_key = GlobalKey::new();
            pump_app(&mut tester, |_app| {
                with_title_app_bar(Scaffold::new())
                    .body(Container::new().key(body_key.clone().into()))
                    .into_widget()
            });
            assert_eq!(size_of(&tester, &body_key), Size::new(800.0, 544.0));

            let insets = MediaQueryData::new().view_insets(EdgeInsets::only(0.0, 0.0, 0.0, 100.0));
            tester.pump_widget(boilerplate(
                MediaQuery::new(
                    insets.clone(),
                    with_title_app_bar(Scaffold::new())
                        .body(Container::new().key(body_key.clone().into())),
                )
                .into_widget(),
            ));
            assert_eq!(size_of(&tester, &body_key), Size::new(800.0, 444.0));

            tester.pump_widget(boilerplate(
                MediaQuery::new(
                    insets,
                    with_title_app_bar(Scaffold::new())
                        .body(Container::new().key(body_key.clone().into()))
                        .resize_to_avoid_bottom_inset(false),
                )
                .into_widget(),
            ));
            assert_eq!(size_of(&tester, &body_key), Size::new(800.0, 544.0));
        });
    }

    /// Dart's `tester.widget<Icon>(find.byType(Icon)).icon`: the one onstage `Icon`.
    fn only_onstage_icon(tester: &WidgetTester) -> Option<waft_widgets::IconData> {
        let icons: Vec<_> = tester
            .find_by_type::<waft_widgets::Icon>()
            .into_iter()
            .filter(|icon| tester.is_onstage(*icon))
            .collect();
        assert_eq!(icons.len(), 1);
        tester.widget(icons[0], |icon: &waft_widgets::Icon| icon.icon.clone())
    }

    /// Dart's `Navigator.push(context, route)` of a page route whose page is `page`.
    fn push_page(
        tester: &WidgetTester,
        context: BuildContext,
        fullscreen_dialog: bool,
        page: fn() -> WidgetRef,
    ) {
        let mut app = tester.cell.borrow_mut();
        let page_builder: waft_widgets::RoutePageBuilder =
            Rc::new(move |_app, _context, _, _| page());
        let route = waft_widgets::PageRouteBuilder::new(&mut app, page_builder)
            .fullscreen_dialog(&mut app, fullscreen_dialog);
        drop(waft_widgets::Navigator::push(
            &mut app,
            context,
            waft_widgets::Route::as_route(route),
        ));
    }

    /// Dart's `back arrow` group's `expectBackIcon`, with the scaffold pushed from the home
    /// route as `Navigator.pushNamed` does.
    fn back_icon_on(platform: TargetPlatform) -> Option<waft_widgets::IconData> {
        let mut icon = None;
        with_main_thread_stack(|| {
            let mut tester = tester_on(platform);
            let root_key = GlobalKey::new();
            pump_app(&mut tester, |_app| {
                Container::new()
                    .key(root_key.clone().into())
                    .child(waft_widgets::Text::new("Home"))
                    .into_widget()
            });
            let context = root_key
                .current_context(&mut tester.cell.borrow_mut())
                .expect("the home route is mounted");
            push_page(&tester, context, false, || {
                Scaffold::new()
                    .app_bar(PreferredSizeWidgetRef::new(AppBar::new()))
                    .body(waft_widgets::Text::new("Scaffold"))
                    .into_widget()
            });
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_secs(1));
            icon = only_onstage_icon(&tester);
        });
        icon
    }

    #[test]
    fn the_back_arrow_is_the_android_arrow_on_android_and_fuchsia() {
        for platform in [TargetPlatform::Android, TargetPlatform::Fuchsia] {
            assert_eq!(
                back_icon_on(platform),
                Some(crate::Icons::arrow_back()),
                "on {platform:?}"
            );
        }
    }

    #[test]
    fn the_back_arrow_is_the_ios_arrow_on_ios_and_macos() {
        for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
            assert_eq!(
                back_icon_on(platform),
                Some(crate::Icons::arrow_back_ios_new_rounded()),
                "on {platform:?}"
            );
        }
    }

    // 'Close button shows correctly with PageRouteBuilder', on every platform. Dart's
    // `MaterialPageRoute` and custom page route cases wait for `MaterialPageRoute`.
    #[test]
    fn a_fullscreen_dialog_page_route_gets_a_close_button() {
        for platform in [
            TargetPlatform::Android,
            TargetPlatform::Fuchsia,
            TargetPlatform::IOS,
            TargetPlatform::Linux,
            TargetPlatform::MacOS,
            TargetPlatform::Windows,
        ] {
            with_main_thread_stack(|| {
                let mut tester = tester_on(platform);
                let body_key = GlobalKey::new();
                pump_app(&mut tester, |_app| {
                    Scaffold::new()
                        .app_bar(PreferredSizeWidgetRef::new(AppBar::new()))
                        .body(waft_widgets::Text::new("Page 1").key(body_key.clone().into()))
                        .into_widget()
                });
                let context = body_key
                    .current_context(&mut tester.cell.borrow_mut())
                    .expect("the first page is mounted");
                push_page(&tester, context, true, || {
                    Scaffold::new()
                        .app_bar(PreferredSizeWidgetRef::new(AppBar::new()))
                        .body(waft_widgets::Text::new("Page 2"))
                        .into_widget()
                });
                tester.pump(Duration::ZERO);
                tester.pump(Duration::from_secs(1));

                assert_eq!(
                    only_onstage_icon(&tester),
                    Some(crate::Icons::close()),
                    "didn't find close icon for pageRouteBuilder on {platform:?}"
                );
                let close_key = waft_widgets::StandardComponentType::CloseButton.key();
                let close_buttons = {
                    let elements = tester.elements();
                    let app = tester.cell.borrow();
                    elements
                        .into_iter()
                        .filter(|element| {
                            waft_widgets::keys_equal(element.widget(&app).key(), Some(&close_key))
                        })
                        .count()
                };
                assert_eq!(
                    close_buttons, 1,
                    "didn't find close button for pageRouteBuilder on {platform:?}"
                );
            });
        }
    }

    #[test]
    fn has_app_bar_and_app_bar_max_height_follow_the_app_bar() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            tester.pump_widget(boilerplate(
                MediaQuery::new(
                    MediaQueryData::new().padding(EdgeInsets::only(0.0, 20.0, 0.0, 0.0)),
                    with_title_app_bar(Scaffold::new().key(scaffold_key.clone().into())),
                )
                .into_widget(),
            ));
            let state = scaffold_state(&tester, &scaffold_key);
            {
                let app = tester.cell.borrow();
                assert!(state.has_app_bar(&app));
                assert_eq!(
                    state.app_bar_max_height(&app),
                    Some(K_TOOLBAR_HEIGHT_FOR_TESTS + 20.0)
                );
            }

            tester.pump_widget(boilerplate(
                MediaQuery::new(
                    MediaQueryData::new().padding(EdgeInsets::only(0.0, 20.0, 0.0, 0.0)),
                    Scaffold::new().key(scaffold_key.clone().into()),
                )
                .into_widget(),
            ));
            let app = tester.cell.borrow();
            assert!(!state.has_app_bar(&app));
        });
    }

    const K_TOOLBAR_HEIGHT_FOR_TESTS: f64 = crate::constants::K_TOOLBAR_HEIGHT;
}
