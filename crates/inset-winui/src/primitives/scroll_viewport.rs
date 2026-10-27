//! Native reveal scrolling used by WinUI control templates.
//!
//! This is kit infrastructure, not a reduced implementation of XAML ScrollViewer.
//! A controller can drive either the single-child viewport here or a native virtualized
//! ListView/CustomScrollView inside ScrollMetricsObserver.

use inset_animation::Curves;
use inset_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle, Listenable, Listener};
use inset_painting::{Axis, EdgeInsetsGeometry};
use inset_scheduler::FrameCallback;
use inset_widgets::*;
use std::time::Duration;

use super::after_next_frame;

/// A snapshot of one native scrolling axis, in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollViewportMetrics {
    /// Current position relative to the beginning of the content.
    pub offset: f64,

    /// Maximum in-range offset after laying out the viewport and content.
    pub scrollable_length: f64,

    /// Visible length along the scrolling axis.
    pub viewport_length: f64,

    /// Total scrollable content length, including the visible viewport.
    pub extent_length: f64,

    /// Whether a drag, wheel sequence, ballistic motion or commanded animation is active.
    pub is_scrolling: bool,
}

/// Owns one reveal ScrollController and reports offset and dimension changes together.
///
/// One controller must be attached to at most one viewport at a time. The caller owns
/// its lifetime and disposes it after the scrolling child has unmounted.
pub struct ScrollViewportController {
    notifier: ChangeNotifierData,
    native: Handle<ScrollController>,
    native_listener: Option<Listener>,
    reported: ScrollViewportMetrics,
    report_pending: bool,
    disposed: bool,
}

impl ScrollViewportController {
    /// Creates an unattached controller at offset zero.
    pub fn new(app: &mut App) -> Handle<Self> {
        let native = ScrollController::default(app);
        Self::with_native(app, native)
    }

    /// Creates an unattached controller whose viewports start at `offset`.
    pub fn with_initial_offset(app: &mut App, offset: f64) -> Handle<Self> {
        let native = ScrollController::new(app, offset, false, None, None, None);
        Self::with_native(app, native)
    }

    /// Wraps `native`, reporting its changes.
    fn with_native(app: &mut App, native: Handle<ScrollController>) -> Handle<Self> {
        let controller = app.create(Self {
            notifier: ChangeNotifierData::new(),
            native,
            native_listener: None,
            reported: ScrollViewportMetrics::default(),
            report_pending: false,
            disposed: false,
        });
        let listener = Listener::new(move |app| controller.schedule_report(app));
        native.add_listener(app, listener.clone());
        app.get_mut(controller).native_listener = Some(listener);
        controller
    }

    /// The native controller to pass to ListView, CustomScrollView or another reveal scroller.
    pub fn native_controller(self: Handle<Self>, app: &App) -> AnyScrollController {
        app.get(self).native.as_controller()
    }

    /// Whether a scrolling child has attached its scroll position.
    pub fn has_clients(self: Handle<Self>, app: &App) -> bool {
        self.native_controller(app).has_clients(app)
    }

    /// Current metrics, or zero lengths before the viewport's first layout.
    pub fn metrics(self: Handle<Self>, app: &App) -> ScrollViewportMetrics {
        let controller = self.native_controller(app);
        if !controller.has_clients(app) {
            return ScrollViewportMetrics::default();
        }
        let position = controller.position(app);
        if !position.has_content_dimensions(app) || !position.has_viewport_dimension(app) {
            return ScrollViewportMetrics::default();
        }
        let minimum = position.min_scroll_extent(app);
        let maximum = position.max_scroll_extent(app);
        let viewport = position.viewport_dimension(app);
        ScrollViewportMetrics {
            offset: position.pixels(app) - minimum,
            scrollable_length: (maximum - minimum).max(0.0),
            viewport_length: viewport,
            extent_length: (maximum - minimum).max(0.0) + viewport,
            is_scrolling: *app.get(position.is_scrolling_notifier(app)).value(),
        }
    }

    /// Moves to a clamped offset; returns false before layout or when already at that offset.
    ///
    /// Animated commands use reveal ScrollAction's 100 ms ease-in-out motion. Windows
    /// supplies the original ChangeView animation through DirectManipulation.
    pub fn change_view(
        self: Handle<Self>,
        app: &mut App,
        offset: f64,
        disable_animation: bool,
    ) -> bool {
        assert!(offset.is_finite(), "scroll offset must be finite");
        let controller = self.native_controller(app);
        if !controller.has_clients(app) {
            return false;
        }
        let position = controller.position(app);
        if !position.has_content_dimensions(app) {
            return false;
        }
        let target = (position.min_scroll_extent(app) + offset).clamp(
            position.min_scroll_extent(app),
            position.max_scroll_extent(app),
        );
        if (position.pixels(app) - target).abs() < f64::EPSILON {
            return false;
        }
        if disable_animation {
            position.jump_to(app, target);
        } else {
            drop(position.move_to(
                app,
                target,
                Some(Duration::from_millis(100)),
                Some(Curves::ease_in_out()),
                Some(true),
            ));
        }
        self.schedule_report(app);
        true
    }

    /// Applies a relative offset using the same clamping and animation as change_view.
    pub fn scroll_by(
        self: Handle<Self>,
        app: &mut App,
        delta: f64,
        disable_animation: bool,
    ) -> bool {
        self.change_view(app, self.metrics(app).offset + delta, disable_animation)
    }

    /// Releases the native controller and its listeners; call after its viewport unmounts.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        let state = app.get_mut(self);
        if state.disposed {
            return;
        }
        state.disposed = true;
        let native = state.native;
        let listener = state.native_listener.take();
        if let Some(listener) = listener {
            native.remove_listener(app, &listener);
        }
        ScrollControllerLeaf::dispose(native, app);
        app.get_mut(self).notifier.dispose();
    }

    /// Coalesces native offset, activity and layout notifications after the current frame.
    fn schedule_report(self: Handle<Self>, app: &mut App) {
        if app.get(self).disposed || app.get(self).report_pending {
            return;
        }
        app.get_mut(self).report_pending = true;
        after_next_frame(
            app,
            // A controller is in no view of its own.
            None,
            FrameCallback::new(move |app, _| {
                if !app.contains(self) || app.get(self).disposed {
                    return;
                }
                app.get_mut(self).report_pending = false;
                let metrics = self.metrics(app);
                if metrics != app.get(self).reported {
                    app.get_mut(self).reported = metrics;
                    self.notify_listeners(app);
                }
            }),
        );
    }
}

impl ChangeNotifier for ScrollViewportController {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.notifier
    }
}

/// Observes the first descendant scroller without inserting another viewport.
///
/// This lets TabView supply a native virtualized list while its surrounding template
/// observes the same metrics as the simple ScrollViewport used by other controls.
#[derive(Debug)]
pub struct ScrollMetricsObserver {
    /// Identity of this observer in its parent's child list.
    pub key: Option<KeyRef>,

    /// Controller attached to the immediate descendant scroller.
    pub controller: Handle<ScrollViewportController>,

    /// A native ListView, CustomScrollView or other scrolling widget.
    pub child: WidgetRef,
}

impl ScrollMetricsObserver {
    /// Observes layout and activity notifications from the scrolling child.
    pub fn new<K>(controller: Handle<ScrollViewportController>, child: impl IntoWidget<K>) -> Self {
        Self {
            key: None,
            controller,
            child: child.into_widget(),
        }
    }

    /// Sets this observer's identity.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }
}

impl StatelessWidget for ScrollMetricsObserver {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let controller = self.controller;
        let behavior = ScrollConfiguration::of(app, context)
            .copy_with()
            .scrollbars(false);
        ScrollConfiguration::new(
            ScrollBehaviorRef::new(behavior),
            NotificationListener::<ScrollMetricsNotification>::new(
                NotificationListener::<dyn ScrollNotification>::new(self.child.clone())
                    .on_notification(move |app, notification| {
                        if notification.depth() == 0 {
                            controller.schedule_report(app);
                        }
                        false
                    })
                    .into_widget(),
            )
            .on_notification(move |app, notification| {
                if notification.depth() == 0 {
                    controller.schedule_report(app);
                }
                false
            })
            .into_widget(),
        )
        .into_widget()
    }
}

/// Single-child scrolling with reveal's wheel, trackpad, keyboard and focus-reveal behavior.
///
/// Control templates with virtualized content should use ScrollMetricsObserver around
/// their native list instead of nesting it inside this widget.
#[derive(Debug)]
pub struct ScrollViewport {
    /// Identity of this viewport in its parent's child list.
    pub key: Option<KeyRef>,

    /// Axis on which the content may exceed the viewport.
    pub axis: Axis,

    /// Controller owned by the enclosing control or application.
    pub controller: Handle<ScrollViewportController>,

    /// Content of this viewport.
    pub child: WidgetRef,

    /// Space inside the scrolling content; defaults to zero.
    pub padding: EdgeInsetsGeometry,
}

impl ScrollViewport {
    /// Creates a native single-axis viewport.
    pub fn new<K>(
        axis: Axis,
        controller: Handle<ScrollViewportController>,
        child: impl IntoWidget<K>,
    ) -> Self {
        Self {
            key: None,
            axis,
            controller,
            child: child.into_widget(),
            padding: EdgeInsetsGeometry::ZERO,
        }
    }

    /// Sets this viewport's identity.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }

    /// Sets the inset around scrolling content.
    pub fn padding(mut self, padding: EdgeInsetsGeometry) -> Self {
        self.padding = padding;
        self
    }
}

impl StatelessWidget for ScrollViewport {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, _: BuildContext) -> WidgetRef {
        ScrollMetricsObserver::new(
            self.controller,
            SingleChildScrollView::new()
                .scroll_direction(self.axis)
                .controller(self.controller.native_controller(app))
                .padding(self.padding)
                .child(self.child.clone()),
        )
        .into_widget()
    }
}
