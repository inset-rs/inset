//! Flutter counterpart: `widgets/binding.dart` (`WidgetsBinding`, `RootWidget`,
//! `RootElement`, `runApp`).
//!
//! `WidgetsBindingObserver` and the observer callbacks (locale, metrics, lifecycle, memory,
//! back gestures), `performReassemble`, and the platform menu / windowing owners
//! wait. Lifecycle observer methods exist so a listener can override them;
//! `handleAppLifecycleStateChanged` waits.

use std::rc::Rc;
use std::time::Duration;

use inset_embedder::{AppExitResponse, AppLifecycleState, Locale, ViewFocusEvent};
use inset_foundation::{App, Handle, Listener, Timer};
use inset_rendering::{RendererBinding, RendererBindingOverridesObject};
use inset_scheduler::SchedulerBinding;

use crate::framework::{
    AnyElement, BuildOwner, Element, ElementBase, ElementData, IntoWidget, KeyRef, Slot, Widget,
    WidgetKind, WidgetRef, downcast_widget,
};
use crate::view::View;

/// The glue between the widgets layer and the Flutter engine.
///
/// The [`WidgetsBinding`] manages a single [`Element`] tree rooted at
/// [`root_element`](Self::root_element). This element tree is created by
/// [`attach_root_widget`](Self::attach_root_widget), and its build is scheduled onto the
/// renderer's frame: the tree is built before the render tree is laid out and painted, and
/// finalized after.
#[derive(Default)]
pub struct WidgetsBinding {
    build_owner: Option<Handle<BuildOwner>>,
    root_element: Option<AnyElement>,
    ready_to_produce_frames: bool,
    observers: Vec<WidgetsBindingObserverRef>,
}

/// Interface for classes that register with the Widgets layer binding.
///
/// This can be used by any class, not just widgets. It provides an interface which is used
/// by [`WidgetsBinding::add_observer`] and [`WidgetsBinding::remove_observer`] to notify
/// objects of changes in the environment, such as changes to the device metrics or
/// accessibility settings. It is used to implement features such as `MediaQuery`.
///
/// A `State` registers itself through the object twin
/// [`WidgetsBindingObserverObject`], whose handle is the observer. The route,
/// memory-pressure and back-gesture callbacks wait with their platform
/// events; only the ones the renderer raises, the locale list, and the lifecycle
/// observer methods are here. `handleAppLifecycleStateChanged` waits.
pub trait WidgetsBindingObserver {
    /// Called when the application's dimensions change. For example, when a phone is rotated.
    fn did_change_metrics(&self, app: &mut App) {
        let _ = app;
    }

    /// Called when the system tells the app that the user's locale has changed. For example,
    /// if the user changes the system language settings.
    ///
    /// This method exposes notifications from `PlatformDispatcher.onLocaleChanged`.
    fn did_change_locales(&self, app: &mut App, locales: Option<&[Locale]>) {
        let _ = (app, locales);
    }

    /// Called when the platform reports a change in the focus state of a view.
    fn did_change_view_focus(&self, app: &mut App, event: ViewFocusEvent) {
        let _ = (app, event);
    }

    /// Called when the platform's text scale factor changes.
    fn did_change_text_scale_factor(&self, app: &mut App) {
        let _ = app;
    }

    /// Called when the platform brightness changes.
    fn did_change_platform_brightness(&self, app: &mut App) {
        let _ = app;
    }

    /// Called when the system puts the app in the background or returns the app to the
    /// foreground.
    ///
    /// This method exposes notifications from `SystemChannels.lifecycle`.
    ///
    /// See also:
    ///
    ///  * [`AppLifecycleListener`](crate::AppLifecycleListener), an alternative API for
    ///    responding to application lifecycle changes.
    fn did_change_app_lifecycle_state(&self, app: &mut App, state: AppLifecycleState) {
        let _ = (app, state);
    }

    /// Called when a request is received from the system to exit the application.
    ///
    /// If any observer responds with [`AppExitResponse::Cancel`], it will cancel the exit.
    /// All observers will be asked before exiting.
    ///
    /// See also:
    ///
    ///  * `ServicesBinding::exit_application` for a function to call that will request
    ///    that the application exits.
    fn did_request_app_exit(&self, app: &mut App) -> AppExitResponse {
        let _ = app;
        AppExitResponse::Exit
    }

    /// Called when the system changes the set of currently active accessibility features.
    fn did_change_accessibility_features(&self, app: &mut App) {
        let _ = app;
    }
}

/// The object side of [`WidgetsBindingObserver`]: implement it on an arena object (a
/// `State`) and register `Rc::new(handle)`.
pub trait WidgetsBindingObserverObject: Sized + 'static {
    /// See [`WidgetsBindingObserver::did_change_metrics`].
    fn did_change_metrics(self: Handle<Self>, app: &mut App) {
        let _ = app;
    }

    /// See [`WidgetsBindingObserver::did_change_locales`].
    fn did_change_locales(self: Handle<Self>, app: &mut App, locales: Option<&[Locale]>) {
        let _ = (app, locales);
    }

    /// See [`WidgetsBindingObserver::did_change_view_focus`].
    fn did_change_view_focus(self: Handle<Self>, app: &mut App, event: ViewFocusEvent) {
        let _ = (app, event);
    }

    /// See [`WidgetsBindingObserver::did_change_text_scale_factor`].
    fn did_change_text_scale_factor(self: Handle<Self>, app: &mut App) {
        let _ = app;
    }

    /// See [`WidgetsBindingObserver::did_change_platform_brightness`].
    fn did_change_platform_brightness(self: Handle<Self>, app: &mut App) {
        let _ = app;
    }

    /// See [`WidgetsBindingObserver::did_change_app_lifecycle_state`].
    fn did_change_app_lifecycle_state(self: Handle<Self>, app: &mut App, state: AppLifecycleState) {
        let _ = (app, state);
    }

    /// See [`WidgetsBindingObserver::did_request_app_exit`].
    fn did_request_app_exit(self: Handle<Self>, app: &mut App) -> AppExitResponse {
        let _ = app;
        AppExitResponse::Exit
    }

    /// See [`WidgetsBindingObserver::did_change_accessibility_features`].
    fn did_change_accessibility_features(self: Handle<Self>, app: &mut App) {
        let _ = app;
    }
}

impl<T: WidgetsBindingObserverObject> WidgetsBindingObserver for Handle<T> {
    fn did_change_metrics(&self, app: &mut App) {
        T::did_change_metrics(*self, app);
    }

    fn did_change_locales(&self, app: &mut App, locales: Option<&[Locale]>) {
        T::did_change_locales(*self, app, locales);
    }

    fn did_change_view_focus(&self, app: &mut App, event: ViewFocusEvent) {
        T::did_change_view_focus(*self, app, event);
    }

    fn did_change_text_scale_factor(&self, app: &mut App) {
        T::did_change_text_scale_factor(*self, app);
    }

    fn did_change_platform_brightness(&self, app: &mut App) {
        T::did_change_platform_brightness(*self, app);
    }

    fn did_change_app_lifecycle_state(&self, app: &mut App, state: AppLifecycleState) {
        T::did_change_app_lifecycle_state(*self, app, state);
    }

    fn did_request_app_exit(&self, app: &mut App) -> AppExitResponse {
        T::did_request_app_exit(*self, app)
    }

    fn did_change_accessibility_features(&self, app: &mut App) {
        T::did_change_accessibility_features(*self, app);
    }
}

/// A registered observer; removed again by identity of the `Rc`.
pub type WidgetsBindingObserverRef = Rc<dyn WidgetsBindingObserver>;

impl WidgetsBinding {
    /// The current [`WidgetsBinding`], created on first use (Flutter's
    /// `WidgetsFlutterBinding.ensureInitialized`).
    ///
    /// Creating it creates the renderer, gesture, and scheduler bindings below it.
    pub fn instance(app: &mut App) -> Handle<WidgetsBinding> {
        let this: Handle<WidgetsBinding> = app.singleton();
        if app.get(this).build_owner.is_none() {
            let build_owner = BuildOwner::new(
                app,
                Some(Listener::new(|app| {
                    WidgetsBinding::instance(app).handle_build_scheduled(app)
                })),
            );
            app.get_mut(this).build_owner = Some(build_owner);
            // Flutter's `WidgetsBinding` overrides `RendererBinding.drawFrame`.
            RendererBinding::instance(app).set_overrides(app, Rc::new(this));
            // Flutter's `initInstances` assigns the dispatcher callbacks the binding handles.
            let callbacks = app.platform_callbacks_mut();
            callbacks.on_platform_brightness_changed = Some(Listener::new(|app| {
                WidgetsBinding::instance(app).handle_platform_brightness_changed(app)
            }));
            callbacks.on_view_focus_change = Some(Rc::new(|app, event| {
                WidgetsBinding::instance(app).handle_view_focus_changed(app, event)
            }));
            callbacks.on_locale_changed = Some(Listener::new(|app| {
                WidgetsBinding::instance(app).handle_locale_changed(app)
            }));
        }
        this
    }

    /// The object in charge of the focus tree.
    ///
    /// Rarely used directly. Instead, consider using `FocusScope::of` to obtain the
    /// `FocusScopeNode` for a given `BuildContext`.
    pub fn focus_manager(self: Handle<Self>, app: &App) -> Handle<crate::FocusManager> {
        self.build_owner(app).focus_manager(app)
    }

    /// The [`BuildOwner`] in charge of executing the build pipeline for the widget tree
    /// rooted at this binding.
    pub fn build_owner(self: Handle<Self>, app: &App) -> Handle<BuildOwner> {
        app.get(self)
            .build_owner
            .expect("WidgetsBinding::instance must run first")
    }

    fn handle_build_scheduled(self: Handle<Self>, app: &mut App) {
        // Flutter's debug checks against building during `drawFrame`'s locked phases wait
        // with diagnostics.
        SchedulerBinding::ensure_visual_update(app);
    }

    /// The [`Element`] that is at the root of the element tree hierarchy.
    ///
    /// This is initialized the first time [`run_app`] is called.
    pub fn root_element(self: Handle<Self>, app: &App) -> Option<AnyElement> {
        app.get(self).root_element
    }

    /// Whether the binding is ready to produce frames: Flutter's `framesEnabled` override
    /// waits for the root widget.
    pub fn ready_to_produce_frames(self: Handle<Self>, app: &App) -> bool {
        app.get(self).ready_to_produce_frames
    }

    /// Wraps the `root_widget` in a [`View`] widget for the implicit view, the way [`run_app`]
    /// does.
    ///
    /// # Panics
    ///
    /// If the platform has no implicit view.
    pub fn wrap_with_default_view(
        self: Handle<Self>,
        app: &mut App,
        root_widget: WidgetRef,
    ) -> WidgetRef {
        let view = app.platform().implicit_view().expect(
            "The app requested a view, but the platform did not provide one. This is likely \
             because the app called `run_app` to render its root widget, which expects the \
             platform to provide a default view to render into (the \"implicit\" view). Try \
             using `run_widget` instead of `run_app` to start your app.",
        );
        View::new(view, root_widget).into_widget()
    }

    /// Schedules a `Timer` for attaching the root widget.
    ///
    /// This is called by [`run_app`] to configure the widget tree. Consider using
    /// [`attach_root_widget`](Self::attach_root_widget) if you want to build the widget tree
    /// synchronously.
    pub fn schedule_attach_root_widget(self: Handle<Self>, app: &mut App, root_widget: WidgetRef) {
        Timer::new(
            app,
            Duration::ZERO,
            Listener::new(move |app| self.attach_root_widget(app, root_widget.clone())),
        );
    }

    /// Takes a widget and attaches it to the [`root_element`](Self::root_element), creating
    /// it if necessary.
    ///
    /// This is called by [`run_app`] to configure the widget tree.
    ///
    /// See also:
    ///
    ///  * [`run_app`], which bootstraps the widget tree.
    pub fn attach_root_widget(self: Handle<Self>, app: &mut App, root_widget: WidgetRef) {
        self.attach_to_build_owner(
            app,
            RootWidget::new()
                .child(root_widget)
                .debug_short_description("[root]"),
        );
    }

    /// Called by [`attach_root_widget`](Self::attach_root_widget) to attach the provided
    /// [`RootWidget`] to the [`build_owner`](Self::build_owner).
    ///
    /// This creates the [`root_element`](Self::root_element), if necessary, or re-uses an
    /// existing one.
    ///
    /// This method is rarely called directly, but it can be useful in tests to restore the
    /// element tree to a previous version by providing the [`RootWidget`] of that version.
    pub fn attach_to_build_owner(self: Handle<Self>, app: &mut App, widget: RootWidget) {
        let is_bootstrap_frame = app.get(self).root_element.is_none();
        app.get_mut(self).ready_to_produce_frames = true;
        let owner = self.build_owner(app);
        let current = app.get(self).root_element.map(|element| {
            element
                .downcast::<RootElement>(app)
                .expect("the root is a RootElement")
        });
        let root = widget.attach(app, owner, current);
        app.get_mut(self).root_element = Some(root.as_element());
        if is_bootstrap_frame {
            SchedulerBinding::ensure_visual_update(app);
        }
    }

    /// Registers the given object as a binding observer. Binding observers are notified when
    /// various application events occur, for example when the system locale changes.
    /// Generally, one widget in the widget tree registers itself as a binding observer, and
    /// converts the system state into inherited widgets.
    ///
    /// The observer is removed again with the same `Rc` (Dart's `Set.remove` by identity).
    pub fn add_observer(self: Handle<Self>, app: &mut App, observer: WidgetsBindingObserverRef) {
        app.get_mut(self).observers.push(observer);
    }

    /// Unregisters the given observer. This should be used sparingly as it is relatively
    /// expensive (O(N) in the number of registered observers).
    pub fn remove_observer(
        self: Handle<Self>,
        app: &mut App,
        observer: &WidgetsBindingObserverRef,
    ) -> bool {
        let observers = &mut app.get_mut(self).observers;
        let before = observers.len();
        observers.retain(|registered| !Rc::ptr_eq(registered, observer));
        observers.len() < before
    }

    #[cfg(test)]
    pub(crate) fn observer_count(self: Handle<Self>, app: &App) -> usize {
        app.get(self).observers.len()
    }

    /// Called when the platform's text scale factor changes: tells the observers.
    pub fn handle_text_scale_factor_changed(self: Handle<Self>, app: &mut App) {
        for observer in app.get(self).observers.clone() {
            observer.did_change_text_scale_factor(app);
        }
    }

    /// Notifies observers that the platform focus of a view has changed.
    pub fn handle_view_focus_changed(self: Handle<Self>, app: &mut App, event: ViewFocusEvent) {
        for observer in app.get(self).observers.clone() {
            observer.did_change_view_focus(app, event);
        }
    }

    /// Called when the platform brightness changes: tells the observers.
    pub fn handle_platform_brightness_changed(self: Handle<Self>, app: &mut App) {
        for observer in app.get(self).observers.clone() {
            observer.did_change_platform_brightness(app);
        }
    }

    /// Called when the system locale changes.
    ///
    /// Calls [`dispatch_locales_changed`](Self::dispatch_locales_changed) to notify the
    /// binding observers.
    ///
    /// This method exposes notifications from `PlatformDispatcher.onLocaleChanged`.
    pub fn handle_locale_changed(self: Handle<Self>, app: &mut App) {
        let locales = app.platform().locales();
        self.dispatch_locales_changed(app, Some(&locales));
    }

    /// Notify all the observers that the locale has changed (using
    /// [`WidgetsBindingObserver::did_change_locales`]), giving them the `locales` argument.
    ///
    /// This is called by [`handle_locale_changed`](Self::handle_locale_changed) when the
    /// `PlatformDispatcher.onLocaleChanged` notification is received.
    pub fn dispatch_locales_changed(self: Handle<Self>, app: &mut App, locales: Option<&[Locale]>) {
        for observer in app.get(self).observers.clone() {
            observer.did_change_locales(app, locales);
        }
    }

    /// Called when the set of active accessibility features changes: tells the observers.
    pub fn handle_accessibility_features_changed(self: Handle<Self>, app: &mut App) {
        for observer in app.get(self).observers.clone() {
            observer.did_change_accessibility_features(app);
        }
    }

    /// Whether the [`root_element`](Self::root_element) has been initialized.
    ///
    /// This will be false until [`run_app`] is called (or `WidgetTester.pumpWidget` is
    /// called in the context of a `TestWidgetsFlutterBinding`).
    pub fn is_root_widget_attached(self: Handle<Self>, app: &App) -> bool {
        app.get(self).root_element.is_some()
    }
}

/// Flutter's `WidgetsBinding.drawFrame`: build the dirty widgets, let the renderer draw,
/// then unmount what fell out of the tree.
impl RendererBindingOverridesObject for WidgetsBinding {
    fn will_draw_frame(self: Handle<Self>, app: &mut App) {
        if let Some(root_element) = app.get(self).root_element {
            let owner = self.build_owner(app);
            owner.build_scope(app, root_element, None);
        }
    }

    fn did_draw_frame(self: Handle<Self>, app: &mut App) {
        let owner = self.build_owner(app);
        owner.finalize_tree(app);
    }

    /// `WidgetsBinding.handleMetricsChanged` after `super`: tells the observers.
    fn did_handle_metrics_changed(self: Handle<Self>, app: &mut App) {
        for observer in app.get(self).observers.clone() {
            observer.did_change_metrics(app);
        }
    }
}

/// Inflate the given widget and attach it to the view.
///
/// The widget is given constraints during layout that force it to fill the entire view. If
/// you wish to align your widget to one side of the view (e.g., the top), consider using the
/// `Align` widget. If you wish to center your widget, you can also use the `Center` widget.
///
/// Calling [`run_app`] again will detach the previous root widget from the view and attach
/// the given widget in its place. The new widget tree is compared against the previous widget
/// tree and any differences are applied to the underlying render tree, similar to what
/// happens when a `StatefulWidget` rebuilds after calling `State::set_state`.
///
/// Initializes the binding using [`WidgetsBinding::instance`] if necessary.
///
/// The root widget is attached on the next timer turn, as Dart's `Timer.run` does, and the
/// first frame is scheduled.
pub fn run_app(app: &mut App, widget: WidgetRef) {
    let binding = WidgetsBinding::instance(app);
    let wrapped = binding.wrap_with_default_view(app, widget);
    run_widget_internal(app, binding, wrapped);
}

/// Inflate the given widget and bootstrap the widget tree.
///
/// Unlike [`run_app`], this method does not define a `View` widget: the given `widget`
/// must contain one (or the tree has no render tree to attach to).
pub fn run_widget(app: &mut App, widget: WidgetRef) {
    let binding = WidgetsBinding::instance(app);
    run_widget_internal(app, binding, widget);
}

fn run_widget_internal(app: &mut App, binding: Handle<WidgetsBinding>, widget: WidgetRef) {
    binding.schedule_attach_root_widget(app, widget);
    SchedulerBinding::schedule_warm_up_frame(app);
}

/// A wrapper widget that will be used as the root of the widget tree.
///
/// An instance of this widget is created by [`WidgetsBinding::attach_root_widget`].
///
/// It is used as the root of the widget tree, and provides a [`RootElement`] which does not
/// have a render object itself and instead delegates rendering to its child.
#[derive(Debug, Default)]
pub struct RootWidget {
    pub key: Option<KeyRef>,
    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,
    /// A short description of this widget used by debugging aids.
    pub debug_short_description: Option<String>,
}

impl RootWidget {
    /// Creates a `RootWidget`; Dart's named arguments are the setters.
    pub fn new() -> RootWidget {
        RootWidget::default()
    }

    /// Dart `RootWidget(key:)`.
    pub fn key(mut self, key: KeyRef) -> RootWidget {
        self.key = Some(key);
        self
    }

    /// Dart `RootWidget(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> RootWidget {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `RootWidget(debug_short_description:)`.
    pub fn debug_short_description(
        mut self,
        debug_short_description: impl Into<String>,
    ) -> RootWidget {
        self.debug_short_description = Some(debug_short_description.into());
        self
    }

    /// Inflate this widget and attaches the resulting [`RootElement`] to the provided
    /// [`BuildOwner`].
    ///
    /// If `element` is `None`, this function will create a new element. Otherwise, the given
    /// element will have an update scheduled to switch to this widget.
    ///
    /// Used by [`WidgetsBinding::attach_to_build_owner`] (which is indirectly called by
    /// [`run_app`]) to bootstrap applications.
    pub fn attach(
        self,
        app: &mut App,
        owner: Handle<BuildOwner>,
        element: Option<Handle<RootElement>>,
    ) -> Handle<RootElement> {
        let widget: WidgetRef = Rc::new(self);
        match element {
            None => {
                let mut created = None;
                owner.lock_state(app, |app| {
                    let element = RootElement::create(app, widget.clone());
                    element.as_element().assign_owner(app, owner);
                    created = Some(element);
                });
                let element = created.expect("created while locked");
                owner.build_scope(
                    app,
                    element.as_element(),
                    Some(Box::new(move |app| {
                        element.as_element().mount(app, None, None)
                    })),
                );
                element
            }
            Some(element) => {
                app.get_mut(element).new_widget = Some(widget);
                element.as_element().mark_needs_build(app);
                element
            }
        }
    }
}

impl Widget for RootWidget {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        RootElement::create(app, this).as_element()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn widget_type(&self) -> std::any::TypeId {
        std::any::TypeId::of::<RootWidget>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

/// The root of the element tree.
///
/// This element class is the instantiation of a [`RootWidget`]. It can be used only as the
/// root of an [`Element`] tree (it cannot be mounted into another [`Element`]; its parent
/// must be `None`).
///
/// In typical usage, it will be instantiated for a [`RootWidget`] by calling
/// [`RootWidget::attach`]. In this usage, it is normally instantiated by the bootstrapping
/// logic in the `WidgetsFlutterBinding` singleton created by [`run_app`].
pub struct RootElement {
    element: ElementData,
    child: Option<AnyElement>,
    new_widget: Option<WidgetRef>,
}

impl RootElement {
    fn create(app: &mut App, widget: WidgetRef) -> Handle<RootElement> {
        debug_assert!(downcast_widget::<RootWidget>(&*widget).is_some());
        app.create(RootElement {
            element: ElementData::new(widget),
            child: None,
            new_widget: None,
        })
    }

    /// The child element, if any.
    pub fn child(self: Handle<Self>, app: &App) -> Option<AnyElement> {
        app.get(self).child
    }

    fn rebuild_child(self: Handle<Self>, app: &mut App) {
        let child_widget = downcast_widget::<RootWidget>(&**self.as_element().widget(app))
            .expect("a RootElement holds a RootWidget")
            .child
            .clone();
        let child = app.get(self).child;
        let child = self
            .as_element()
            .update_child(app, child, child_widget, None);
        app.get_mut(self).child = child;
    }
}

impl Element for RootElement {
    crate::element_accessors!();

    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        if let Some(child) = app.get(self).child {
            visitor(child);
        }
    }

    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        debug_assert!(app.get(self).child == Some(child));
        app.get_mut(self).child = None;
    }

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        debug_assert!(parent.is_none()); // We are the root!
        debug_assert!(new_slot.is_none());
        ElementBase::mount(self, app, parent, new_slot);
        self.rebuild_child(app);
        debug_assert!(app.get(self).child.is_some());
        ElementBase::perform_rebuild(self, app); // clears the "dirty" flag
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        ElementBase::update(self, app, new_widget);
        self.rebuild_child(app);
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        if let Some(new_widget) = app.get_mut(self).new_widget.take() {
            Element::update(self, app, new_widget);
        }
        ElementBase::perform_rebuild(self, app);
        debug_assert!(app.get(self).new_widget.is_none());
    }

    fn debug_doing_build(self: Handle<Self>, _app: &App) -> bool {
        false // This element doesn't have a build phase.
    }

    fn debug_expects_render_object_for_slot(
        self: Handle<Self>,
        _app: &App,
        _slot: Option<&Slot>,
    ) -> bool {
        false
    }
}

/// The build owner a `GlobalKey` resolves against: the binding's.
pub(crate) fn current_build_owner(app: &mut App) -> Option<Handle<BuildOwner>> {
    let binding = WidgetsBinding::instance(app);
    app.get(binding).build_owner
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::Duration;

    use inset_embedder::{Size, TargetPlatform};
    use inset_foundation::{App, AppCell, Handle};
    use inset_rendering::{
        AnyRenderObject, BoxConstraints, RenderBox, RenderConstrainedBox, RenderHandle,
        RendererBinding,
    };
    use inset_scheduler::SchedulerBinding;
    use inset_test::{TestPlatform, TestView};

    use super::*;
    use crate::framework::{
        BuildContext, LeafRenderObjectWidget, RenderObjectWidget, State, StateData, StatefulWidget,
    };

    /// An [`App`] over one 800x600 view at 2x, with the view and the platform to read back.
    fn app_with_view() -> (Rc<AppCell>, Rc<TestView>, Rc<TestPlatform>) {
        let view = Rc::new(TestView::with_pixel_ratio(800.0, 600.0, 2.0));
        let platform = Rc::new(
            TestPlatform::new()
                .on(TargetPlatform::MacOS)
                .with_view(view.clone()),
        );
        (AppCell::with_platform(platform.clone()), view, platform)
    }

    fn pump_frame(app: &mut App, at: Duration) {
        SchedulerBinding::handle_begin_frame(app, Some(at));
        app.drain_microtasks();
        SchedulerBinding::handle_draw_frame(app);
        app.drain_microtasks();
    }

    #[derive(Debug)]
    struct Sized {
        size: Size,
    }

    impl RenderObjectWidget for Sized {
        type RenderObject = RenderConstrainedBox;

        fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
            RenderConstrainedBox::new(app, BoxConstraints::tight(self.size), None).as_object()
        }

        fn update_render_object(
            &self,
            app: &mut App,
            _context: BuildContext,
            render_object: RenderHandle<RenderConstrainedBox>,
        ) {
            render_object.set_additional_constraints(app, BoxConstraints::tight(self.size));
        }
    }

    impl LeafRenderObjectWidget for Sized {}

    #[derive(Debug)]
    struct Resizable {
        size: Rc<Cell<Size>>,
        state: Rc<Cell<Option<Handle<ResizableState>>>>,
    }

    impl StatefulWidget for Resizable {
        type State = ResizableState;

        fn create_state(&self) -> ResizableState {
            ResizableState {
                state: StateData::new(),
            }
        }
    }

    struct ResizableState {
        state: StateData<Resizable>,
    }

    impl State for ResizableState {
        type Widget = Resizable;
        crate::state_accessors!();

        fn init_state(self: Handle<Self>, app: &mut App) {
            self.widget(app).state.set(Some(self));
        }

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            Sized {
                size: self.widget(app).size.get(),
            }
            .into_widget()
        }
    }

    fn root_child_size(app: &mut App) -> Size {
        let render_view = RendererBinding::instance(app)
            .render_views(app)
            .into_iter()
            .next()
            .expect("run_app registered a render view");
        let child = render_view.child(app).expect("the view has a child");
        child.size(app)
    }

    #[test]
    fn run_app_attaches_on_the_next_timer_turn_and_draws_a_warm_up_frame() {
        let (cell, view, _platform) = app_with_view();
        let mut app = cell.borrow_mut();
        run_app(
            &mut app,
            Sized {
                size: Size::new(30.0, 20.0),
            }
            .into_widget(),
        );
        let binding = WidgetsBinding::instance(&mut app);
        assert!(!binding.is_root_widget_attached(&app));
        assert_eq!(view.presented(), 0);

        drop(app);
        cell.elapse(Duration::ZERO);
        let mut app = cell.borrow_mut();
        assert!(binding.is_root_widget_attached(&app));
        let root = binding
            .root_element(&app)
            .and_then(|element| element.downcast::<RootElement>(&app))
            .expect("the root element is a RootElement");
        assert!(root.child(&app).is_some());
        assert_eq!(
            view.presented(),
            1,
            "the warm-up frame drew without a host frame"
        );
        assert_eq!(root_child_size(&mut app), Size::new(400.0, 300.0));
    }

    #[test]
    fn set_state_schedules_a_frame_that_rebuilds_before_layout() {
        let (cell, view, platform) = app_with_view();
        let mut app = cell.borrow_mut();
        let size = Rc::new(Cell::new(Size::new(30.0, 20.0)));
        let state = Rc::new(Cell::new(None));
        run_app(
            &mut app,
            Resizable {
                size: Rc::clone(&size),
                state: Rc::clone(&state),
            }
            .into_widget(),
        );
        drop(app);
        cell.elapse(Duration::ZERO);
        let mut app = cell.borrow_mut();
        assert_eq!(view.presented(), 1, "the warm-up frame");
        let frames_before = platform.frames_requested();

        size.set(Size::new(50.0, 60.0));
        let state = state.get().expect("the state registered itself");
        state.set_state(&mut app, |_| {});
        assert_eq!(platform.frames_requested(), frames_before + 1);

        pump_frame(&mut app, Duration::from_millis(16));
        assert_eq!(view.presented(), 2);
        assert_eq!(root_child_size(&mut app), Size::new(400.0, 300.0));
        let root = WidgetsBinding::instance(&mut app)
            .root_element(&app)
            .expect("attached");
        let mut leaves = Vec::new();
        collect_leaves(&app, root, &mut leaves);
        assert_eq!(leaves.len(), 1);
    }

    fn collect_leaves(app: &App, element: AnyElement, leaves: &mut Vec<AnyElement>) {
        let mut children = Vec::new();
        element.visit_children(app, &mut |child| children.push(child));
        if children.is_empty() {
            leaves.push(element);
        }
        for child in children {
            collect_leaves(app, child, leaves);
        }
    }

    #[test]
    fn running_a_second_app_updates_the_root_in_place() {
        let (cell, _view, _platform) = app_with_view();
        let mut app = cell.borrow_mut();
        run_app(
            &mut app,
            Sized {
                size: Size::new(30.0, 20.0),
            }
            .into_widget(),
        );
        drop(app);
        cell.elapse(Duration::ZERO);
        let mut app = cell.borrow_mut();
        pump_frame(&mut app, Duration::ZERO);
        let binding = WidgetsBinding::instance(&mut app);
        let first_root = binding.root_element(&app);

        run_app(
            &mut app,
            Sized {
                size: Size::new(10.0, 10.0),
            }
            .into_widget(),
        );
        drop(app);
        cell.elapse(Duration::ZERO);
        let mut app = cell.borrow_mut();
        assert_eq!(binding.root_element(&app), first_root);
        pump_frame(&mut app, Duration::from_millis(16));
        assert_eq!(
            RendererBinding::instance(&mut app).render_views(&app).len(),
            1
        );
    }

    #[test]
    fn a_metrics_change_reaches_the_observers_and_the_media_query() {
        let (cell, view, _platform) = app_with_view();
        let mut app = cell.borrow_mut();
        let sizes = Rc::new(std::cell::RefCell::new(Vec::new()));
        let probe = crate::widgets::basic::Builder::new({
            let sizes = Rc::clone(&sizes);
            move |app, context| {
                sizes
                    .borrow_mut()
                    .push(crate::MediaQuery::size_of(app, context));
                Sized {
                    size: Size::new(1.0, 1.0),
                }
                .into_widget()
            }
        });
        run_app(&mut app, probe.into_widget());
        drop(app);
        cell.elapse(Duration::ZERO);
        let mut app = cell.borrow_mut();
        pump_frame(&mut app, Duration::ZERO);
        assert_eq!(*sizes.borrow(), vec![Size::new(400.0, 300.0)]);

        view.resize(1000.0, 400.0);
        RendererBinding::instance(&mut app).handle_metrics_changed(&mut app);
        pump_frame(&mut app, Duration::from_millis(16));
        assert_eq!(
            *sizes.borrow(),
            vec![Size::new(400.0, 300.0), Size::new(500.0, 200.0)]
        );
    }
}
