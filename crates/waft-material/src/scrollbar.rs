//! Flutter counterpart: material_ui's `scrollbar.dart`.
//!
//! [`Scrollbar`] picks a `CupertinoScrollbar` on iOS and Dart's `_MaterialScrollbar`
//! otherwise, a `RawScrollbar` subclass whose state, `_MaterialScrollbarState`, extends
//! `RawScrollbarState` the way `CupertinoScrollbar`'s does: a [`RawScrollbarStateLeaf`]
//! that calls the base bodies on [`RawScrollbarState`] where Dart writes `super.…`.

use std::collections::HashSet;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{Animation, AnimationBehavior, AnimationController};
use waft_cupertino::CupertinoScrollbar;
use waft_embedder::{Brightness, Color, Offset, Radius, TargetPlatform};
use waft_foundation::{App, Handle, Listener, OptionRetained, Ref};
use waft_gestures::{PointerHoverEvent, Velocity};
use waft_painting::EdgeInsetsGeometry;
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_widgets::{
    BuildContext, Directionality, IntoWidget, KeyRef, MediaQuery, RawScrollbarData,
    RawScrollbarLeaf, RawScrollbarState, RawScrollbarStateData, RawScrollbarStateLeaf,
    ScrollControllerRef, ScrollNotificationPredicate, ScrollbarOrientation, State, StateData,
    StatefulWidget, StatelessWidget, TickerProviderStateMixin, TickerProviderStateMixinData,
    WidgetRef, WidgetState, WidgetStateProperty, WidgetStatePropertyRef, WidgetStates,
    default_scroll_notification_predicate,
};

use crate::color_scheme::ColorScheme;
use crate::scrollbar_theme::{ScrollbarTheme, ScrollbarThemeData};
use crate::theme::Theme;

const K_SCROLLBAR_THICKNESS: f64 = 8.0;
const K_SCROLLBAR_THICKNESS_WITH_TRACK: f64 = 12.0;
const K_SCROLLBAR_MARGIN: f64 = 2.0;
const K_SCROLLBAR_MIN_LENGTH: f64 = 48.0;
const K_SCROLLBAR_RADIUS: Radius = Radius::circular(8.0);
const K_SCROLLBAR_FADE_DURATION: Duration = Duration::from_millis(300);
const K_SCROLLBAR_TIME_TO_FADE: Duration = Duration::from_millis(600);

/// A Material Design scrollbar.
///
/// To add a scrollbar to a `ScrollView`, wrap the scroll view
/// widget in a [`Scrollbar`] widget.
///
/// For more information, see `RawScrollbar`.
///
/// Dynamically changes to a [`CupertinoScrollbar`], an iOS style scrollbar, by
/// default on the iOS platform.
///
/// The color of the Scrollbar thumb will change when [`WidgetState::Dragged`],
/// or [`WidgetState::Hovered`] on desktop and web platforms. These stateful
/// color choices can be changed using [`ScrollbarThemeData::thumb_color`].
///
/// When [`thumb_visibility`](Self::thumb_visibility) is true, the scrollbar thumb will
/// remain visible without the fade animation. This requires that a `ScrollController` is
/// provided to controller, or that the `PrimaryScrollController` is available.
///
/// When a `ScrollView::scroll_direction` is `Axis::Horizontal`, it is recommended
/// that the [`Scrollbar`] is always visible, since scrolling in the horizontal
/// axis is less discoverable.
///
/// A scrollbar track can be added using [`track_visibility`](Self::track_visibility). This
/// can also be drawn when triggered by a hover event, or based on any [`WidgetState`] by
/// using [`ScrollbarThemeData::track_visibility`].
///
/// The [`thickness`](Self::thickness) of the track and scrollbar thumb can be changed
/// dynamically in response to [`WidgetState`]s using [`ScrollbarThemeData::thickness`].
///
/// See also:
///
///  * `RawScrollbar`, a basic scrollbar that fades in and out, extended
///    by this class to add more animations and behaviors.
///  * [`ScrollbarTheme`], which configures the Scrollbar's appearance.
///  * [`CupertinoScrollbar`], an iOS style scrollbar.
///  * `ListView`, which displays a linear, scrollable list of children.
///  * `GridView`, which displays a 2 dimensional, scrollable array of children.
pub struct Scrollbar {
    pub key: Option<KeyRef>,

    /// The widget below this widget in the tree.
    ///
    /// The scrollbar will be stacked on top of this child. This child (and its
    /// subtree) should include a source of `ScrollNotification` notifications.
    pub child: WidgetRef,

    /// The `ScrollController` used to implement Scrollbar dragging.
    ///
    /// For more information, see `RawScrollbarData::controller`.
    pub controller: Option<ScrollControllerRef>,

    /// Indicates that the scrollbar thumb should be visible, even when a scroll
    /// is not underway.
    ///
    /// For more information, see `RawScrollbarData::thumb_visibility`.
    ///
    /// If this property is `None`, then [`ScrollbarThemeData::thumb_visibility`] of
    /// `ThemeData::scrollbar_theme` is used. If that is also `None`, the default value
    /// is false.
    ///
    /// If the thumb visibility is related to the scrollbar's material state,
    /// use the global [`ScrollbarThemeData::thumb_visibility`] or override the
    /// sub-tree's theme data.
    pub thumb_visibility: Option<bool>,

    /// Indicates that the scrollbar track should be visible.
    ///
    /// When true, the scrollbar track will always be visible so long as the thumb
    /// is visible. If the scrollbar thumb is not visible, the track will not be
    /// visible either.
    ///
    /// If this property is `None`, then [`ScrollbarThemeData::track_visibility`] of
    /// `ThemeData::scrollbar_theme` is used. If that is also `None`, the default value
    /// is false.
    ///
    /// If the track visibility is related to the scrollbar's material state,
    /// use the global [`ScrollbarThemeData::track_visibility`] or override the
    /// sub-tree's theme data.
    pub track_visibility: Option<bool>,

    /// The thickness of the scrollbar in the cross axis of the scrollable.
    ///
    /// If `None`, the default value is platform dependent. On [`TargetPlatform::Android`],
    /// the default thickness is 4.0 pixels. On [`TargetPlatform::IOS`],
    /// [`CupertinoScrollbar::DEFAULT_THICKNESS`] is used. The remaining platforms have a
    /// default thickness of 8.0 pixels.
    pub thickness: Option<f64>,

    /// The [`Radius`] of the scrollbar thumb's rounded rectangle corners.
    ///
    /// If `None`, the default value is platform dependent. On [`TargetPlatform::Android`],
    /// no radius is applied to the scrollbar thumb. On [`TargetPlatform::IOS`],
    /// [`CupertinoScrollbar::DEFAULT_RADIUS`] is used. The remaining platforms have a
    /// default [`Radius::circular`] of 8.0 pixels.
    pub radius: Option<Radius>,

    /// Whether the scrollbar should be interactive and respond to dragging on the
    /// thumb, or tapping in the track area.
    ///
    /// Does not apply to the [`CupertinoScrollbar`], which is always interactive to
    /// match native behavior. On Android, the scrollbar is not interactive by
    /// default.
    ///
    /// When false, the scrollbar will not respond to gesture or hover events,
    /// and will allow to click through it.
    ///
    /// Defaults to true when `None`, unless on Android, which will default to
    /// false when `None`.
    pub interactive: Option<bool>,

    /// A check that specifies whether a `ScrollNotification` should be
    /// handled by this widget.
    ///
    /// By default, checks whether `notification.depth() == 0`. That means if the
    /// scrollbar is wrapped around multiple scroll views, it only responds to the
    /// nearest one and shows the corresponding scrollbar thumb.
    pub notification_predicate: Option<ScrollNotificationPredicate>,

    /// Dictates the orientation of the scrollbar.
    ///
    /// [`ScrollbarOrientation::Top`] places the scrollbar on top of the screen.
    /// [`ScrollbarOrientation::Bottom`] places the scrollbar on the bottom of the screen.
    /// [`ScrollbarOrientation::Left`] places the scrollbar on the left of the screen.
    /// [`ScrollbarOrientation::Right`] places the scrollbar on the right of the screen.
    ///
    /// [`ScrollbarOrientation::Top`] and [`ScrollbarOrientation::Bottom`] can only be
    /// used with a vertical scroll. [`ScrollbarOrientation::Left`] and
    /// [`ScrollbarOrientation::Right`] can only be used with a horizontal scroll.
    ///
    /// For a vertical scroll the orientation defaults to
    /// [`ScrollbarOrientation::Right`] for `TextDirection::Ltr` and
    /// [`ScrollbarOrientation::Left`] for `TextDirection::Rtl`.
    /// For a horizontal scroll the orientation defaults to
    /// [`ScrollbarOrientation::Bottom`].
    pub scrollbar_orientation: Option<ScrollbarOrientation>,
}

impl Scrollbar {
    /// Creates a Material Design scrollbar that by default will connect to the
    /// closest Scrollable descendant of [`child`](Self::child).
    ///
    /// The [`child`](Self::child) should be a source of `ScrollNotification`
    /// notifications, typically a `Scrollable` widget.
    ///
    /// If the [`controller`](Self::controller) is `None`, the default behavior is to
    /// enable scrollbar dragging using the `PrimaryScrollController`.
    ///
    /// When `None`, [`thickness`](Self::thickness) defaults to 8.0 pixels on desktop and
    /// web, and 4.0 pixels when on mobile platforms. A `None` [`radius`](Self::radius) will
    /// result in a default of an 8.0 pixel circular radius about the corners of the
    /// scrollbar thumb, except for when executing on [`TargetPlatform::Android`], which
    /// will render the thumb without a radius.
    pub fn new<K>(child: impl IntoWidget<K>) -> Scrollbar {
        Scrollbar {
            key: None,
            child: child.into_widget(),
            controller: None,
            thumb_visibility: None,
            track_visibility: None,
            thickness: None,
            radius: None,
            interactive: None,
            notification_predicate: None,
            scrollbar_orientation: None,
        }
    }

    /// Dart `Scrollbar(key:)`.
    pub fn key(mut self, key: KeyRef) -> Scrollbar {
        self.key = Some(key);
        self
    }

    /// Dart `Scrollbar(controller:)`.
    pub fn controller(mut self, controller: ScrollControllerRef) -> Scrollbar {
        self.controller = Some(controller);
        self
    }

    /// Dart `Scrollbar(thumbVisibility:)`.
    pub fn thumb_visibility(mut self, thumb_visibility: bool) -> Scrollbar {
        self.thumb_visibility = Some(thumb_visibility);
        self
    }

    /// Dart `Scrollbar(trackVisibility:)`.
    pub fn track_visibility(mut self, track_visibility: bool) -> Scrollbar {
        self.track_visibility = Some(track_visibility);
        self
    }

    /// Dart `Scrollbar(thickness:)`.
    pub fn thickness(mut self, thickness: f64) -> Scrollbar {
        self.thickness = Some(thickness);
        self
    }

    /// Dart `Scrollbar(radius:)`.
    pub fn radius(mut self, radius: Radius) -> Scrollbar {
        self.radius = Some(radius);
        self
    }

    /// Dart `Scrollbar(notificationPredicate:)`.
    pub fn notification_predicate(
        mut self,
        notification_predicate: ScrollNotificationPredicate,
    ) -> Scrollbar {
        self.notification_predicate = Some(notification_predicate);
        self
    }

    /// Dart `Scrollbar(interactive:)`.
    pub fn interactive(mut self, interactive: bool) -> Scrollbar {
        self.interactive = Some(interactive);
        self
    }

    /// Dart `Scrollbar(scrollbarOrientation:)`.
    pub fn scrollbar_orientation(
        mut self,
        scrollbar_orientation: ScrollbarOrientation,
    ) -> Scrollbar {
        self.scrollbar_orientation = Some(scrollbar_orientation);
        self
    }
}

impl Debug for Scrollbar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scrollbar")
            .field("child", &self.child)
            .field("thumbVisibility", &self.thumb_visibility)
            .field("trackVisibility", &self.track_visibility)
            .field("thickness", &self.thickness)
            .field("radius", &self.radius)
            .field("interactive", &self.interactive)
            .field("scrollbarOrientation", &self.scrollbar_orientation)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for Scrollbar {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        if Theme::of(app, context).platform == TargetPlatform::IOS {
            let mut scrollbar = CupertinoScrollbar::new(self.child.clone())
                .thumb_visibility(self.thumb_visibility.unwrap_or(false))
                .thickness(
                    self.thickness
                        .unwrap_or(CupertinoScrollbar::DEFAULT_THICKNESS),
                )
                .thickness_while_dragging(
                    self.thickness
                        .unwrap_or(CupertinoScrollbar::DEFAULT_THICKNESS_WHILE_DRAGGING),
                )
                .radius(self.radius.unwrap_or(CupertinoScrollbar::DEFAULT_RADIUS))
                .radius_while_dragging(
                    self.radius
                        .unwrap_or(CupertinoScrollbar::DEFAULT_RADIUS_WHILE_DRAGGING),
                );
            if let Some(controller) = self.controller.clone() {
                scrollbar = scrollbar.controller(controller);
            }
            if let Some(notification_predicate) = &self.notification_predicate {
                scrollbar = scrollbar.notification_predicate(Rc::clone(notification_predicate));
            }
            if let Some(scrollbar_orientation) = self.scrollbar_orientation {
                scrollbar = scrollbar.scrollbar_orientation(scrollbar_orientation);
            }
            return scrollbar.into_widget();
        }
        let mut scrollbar =
            MaterialScrollbar::new(self.child.clone(), self.notification_predicate.clone());
        let raw_scrollbar = &mut scrollbar.raw_scrollbar;
        raw_scrollbar.controller = self.controller.clone();
        raw_scrollbar.thumb_visibility = self.thumb_visibility;
        raw_scrollbar.track_visibility = self.track_visibility;
        raw_scrollbar.thickness = self.thickness;
        raw_scrollbar.radius = self.radius;
        raw_scrollbar.interactive = self.interactive;
        raw_scrollbar.scrollbar_orientation = self.scrollbar_orientation;
        scrollbar.into_widget()
    }
}

/// Dart's `_MaterialScrollbar`: a `RawScrollbar` with Material's fade timing and its own
/// state.
struct MaterialScrollbar {
    raw_scrollbar: RawScrollbarData,
}

impl MaterialScrollbar {
    /// Dart's `_MaterialScrollbar` constructor with what it hands `super` itself; the
    /// arguments it forwards as `super.…` are written into
    /// [`raw_scrollbar`](Self::raw_scrollbar) by [`Scrollbar`].
    fn new(
        child: WidgetRef,
        notification_predicate: Option<ScrollNotificationPredicate>,
    ) -> MaterialScrollbar {
        let mut raw_scrollbar = RawScrollbarData::new(child);
        raw_scrollbar.fade_duration = K_SCROLLBAR_FADE_DURATION;
        raw_scrollbar.time_to_fade = K_SCROLLBAR_TIME_TO_FADE;
        raw_scrollbar.press_duration = Duration::ZERO;
        raw_scrollbar.notification_predicate = notification_predicate
            .unwrap_or_else(|| Rc::new(default_scroll_notification_predicate));
        MaterialScrollbar { raw_scrollbar }
    }
}

impl RawScrollbarLeaf for MaterialScrollbar {
    waft_widgets::raw_scrollbar_accessors!();
}

impl Debug for MaterialScrollbar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("_MaterialScrollbar")
    }
}

impl StatefulWidget for MaterialScrollbar {
    type State = MaterialScrollbarState;

    fn key(&self) -> Option<&KeyRef> {
        self.raw_scrollbar.key.as_ref()
    }

    fn create_state(&self) -> MaterialScrollbarState {
        MaterialScrollbarState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            raw_scrollbar_state: RawScrollbarStateData::new(),
            hover_animation_controller: None,
            drag_is_active: false,
            hover_is_active: false,
            color_scheme: None,
            scrollbar_theme: None,
            use_android_scrollbar: None,
        }
    }
}

/// Dart's `_MaterialScrollbarState`. Its `late` fields are `None` until `initState` and
/// `didChangeDependencies` set them.
struct MaterialScrollbarState {
    state: StateData<MaterialScrollbar>,
    ticker_provider: TickerProviderStateMixinData,
    raw_scrollbar_state: RawScrollbarStateData,
    hover_animation_controller: Option<Ref<AnimationController>>,
    drag_is_active: bool,
    hover_is_active: bool,
    color_scheme: Option<ColorScheme>,
    scrollbar_theme: Option<ScrollbarThemeData>,
    // On Android, scrollbars should match native appearance.
    use_android_scrollbar: Option<bool>,
}

impl MaterialScrollbarState {
    fn hover_animation_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .hover_animation_controller
            .handle()
            .expect("initState creates the controller")
    }

    fn color_scheme(self: Handle<Self>, app: &App) -> &ColorScheme {
        app.get(self)
            .color_scheme
            .as_ref()
            .expect("didChangeDependencies reads the color scheme")
    }

    fn scrollbar_theme(self: Handle<Self>, app: &App) -> &ScrollbarThemeData {
        app.get(self)
            .scrollbar_theme
            .as_ref()
            .expect("didChangeDependencies reads the scrollbar theme")
    }

    fn use_android_scrollbar(self: Handle<Self>, app: &App) -> bool {
        app.get(self)
            .use_android_scrollbar
            .expect("didChangeDependencies reads the platform")
    }

    fn track_visibility(self: Handle<Self>, app: &App) -> WidgetStatePropertyRef<bool> {
        let widget_track_visibility = self.widget(app).raw_scrollbar.track_visibility;
        let theme_track_visibility = self.scrollbar_theme(app).track_visibility.clone();
        <dyn WidgetStateProperty<bool>>::resolve_with(move |states: &WidgetStates| {
            widget_track_visibility
                .or_else(|| {
                    theme_track_visibility
                        .as_ref()
                        .and_then(|track_visibility| track_visibility.resolve(states))
                })
                .unwrap_or(false)
        })
    }

    fn states(self: Handle<Self>, app: &App) -> WidgetStates {
        let state = app.get(self);
        let mut states = HashSet::new();
        if state.drag_is_active {
            states.insert(WidgetState::Dragged);
        }
        if state.hover_is_active {
            states.insert(WidgetState::Hovered);
        }
        states
    }

    #[expect(deprecated, reason = "Dart calls the deprecated `Color.withOpacity`")]
    fn thumb_color(self: Handle<Self>, app: &mut App) -> WidgetStatePropertyRef<Color> {
        let color_scheme = self.color_scheme(app);
        let on_surface = color_scheme.on_surface;
        let brightness = color_scheme.brightness;
        let use_android_scrollbar = self.use_android_scrollbar(app);
        let (drag_color, hover_color, idle_color) = match brightness {
            Brightness::Light => (
                on_surface.with_opacity(0.6),
                on_surface.with_opacity(0.5),
                if use_android_scrollbar {
                    self.highlight_color(app).with_opacity(1.0)
                } else {
                    on_surface.with_opacity(0.1)
                },
            ),
            Brightness::Dark => (
                on_surface.with_opacity(0.75),
                on_surface.with_opacity(0.65),
                if use_android_scrollbar {
                    self.highlight_color(app).with_opacity(1.0)
                } else {
                    on_surface.with_opacity(0.3)
                },
            ),
        };

        let theme_thumb_color = self.scrollbar_theme(app).thumb_color.clone();
        let track_visibility = self.track_visibility(app);
        let hover_value = self.hover_animation_controller(app).value(app);
        <dyn WidgetStateProperty<Color>>::resolve_with(move |states: &WidgetStates| {
            let theme_thumb_color = || {
                theme_thumb_color
                    .as_ref()
                    .and_then(|thumb_color| thumb_color.resolve(states))
            };
            if states.contains(&WidgetState::Dragged) {
                return theme_thumb_color().unwrap_or(drag_color);
            }

            // If the track is visible, the thumb color hover animation is ignored and
            // changes immediately.
            if track_visibility.resolve(states) {
                return theme_thumb_color().unwrap_or(hover_color);
            }

            Color::lerp(
                Some(theme_thumb_color().unwrap_or(idle_color)),
                Some(theme_thumb_color().unwrap_or(hover_color)),
                hover_value,
            )
            .expect("both colors are given")
        })
    }

    fn highlight_color(self: Handle<Self>, app: &mut App) -> Color {
        let context = self.context(app);
        Theme::of(app, context).highlight_color
    }

    #[expect(deprecated, reason = "Dart calls the deprecated `Color.withOpacity`")]
    fn track_color(self: Handle<Self>, app: &mut App) -> WidgetStatePropertyRef<Color> {
        let color_scheme = self.color_scheme(app);
        let on_surface = color_scheme.on_surface;
        let brightness = color_scheme.brightness;
        let show_scrollbar = self.show_scrollbar(app);
        let track_visibility = self.track_visibility(app);
        let theme_track_color = self.scrollbar_theme(app).track_color.clone();
        <dyn WidgetStateProperty<Color>>::resolve_with(move |states: &WidgetStates| {
            if show_scrollbar && track_visibility.resolve(states) {
                return theme_track_color
                    .as_ref()
                    .and_then(|track_color| track_color.resolve(states))
                    .unwrap_or_else(|| match brightness {
                        Brightness::Light => on_surface.with_opacity(0.03),
                        Brightness::Dark => on_surface.with_opacity(0.05),
                    });
            }
            Color::new(0x00000000)
        })
    }

    #[expect(deprecated, reason = "Dart calls the deprecated `Color.withOpacity`")]
    fn track_border_color(self: Handle<Self>, app: &mut App) -> WidgetStatePropertyRef<Color> {
        let color_scheme = self.color_scheme(app);
        let on_surface = color_scheme.on_surface;
        let brightness = color_scheme.brightness;
        let show_scrollbar = self.show_scrollbar(app);
        let track_visibility = self.track_visibility(app);
        let theme_track_border_color = self.scrollbar_theme(app).track_border_color.clone();
        <dyn WidgetStateProperty<Color>>::resolve_with(move |states: &WidgetStates| {
            if show_scrollbar && track_visibility.resolve(states) {
                return theme_track_border_color
                    .as_ref()
                    .and_then(|track_border_color| track_border_color.resolve(states))
                    .unwrap_or_else(|| match brightness {
                        Brightness::Light => on_surface.with_opacity(0.1),
                        Brightness::Dark => on_surface.with_opacity(0.25),
                    });
            }
            Color::new(0x00000000)
        })
    }

    fn thickness(self: Handle<Self>, app: &App) -> WidgetStatePropertyRef<f64> {
        let widget_thickness = self.widget(app).raw_scrollbar.thickness;
        let theme_thickness = self.scrollbar_theme(app).thickness.clone();
        let track_visibility = self.track_visibility(app);
        let use_android_scrollbar = self.use_android_scrollbar(app);
        <dyn WidgetStateProperty<f64>>::resolve_with(move |states: &WidgetStates| {
            let thickness = || {
                widget_thickness.or_else(|| {
                    theme_thickness
                        .as_ref()
                        .and_then(|thickness| thickness.resolve(states))
                })
            };
            if states.contains(&WidgetState::Hovered) && track_visibility.resolve(states) {
                return thickness().unwrap_or(K_SCROLLBAR_THICKNESS_WITH_TRACK);
            }
            // The default scrollbar thickness is smaller on mobile.
            thickness()
                .unwrap_or(K_SCROLLBAR_THICKNESS / (if use_android_scrollbar { 2.0 } else { 1.0 }))
        })
    }
}

impl State for MaterialScrollbarState {
    type Widget = MaterialScrollbar;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        RawScrollbarState::init_state(self, app);
        let controller = AnimationController::new(
            app,
            None,
            Some(Duration::from_millis(200)),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        app.get_mut(self).hover_animation_controller = Some(controller.clone());
        controller.add_listener(
            app,
            Listener::new(move |app: &mut App| {
                self.update_scrollbar_painter(app);
            }),
        );
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let theme = Theme::of(app, context);
        let scrollbar_theme = ScrollbarTheme::of(app, context);
        let use_android_scrollbar = match theme.platform {
            TargetPlatform::Android => true,
            TargetPlatform::IOS
            | TargetPlatform::Linux
            | TargetPlatform::Fuchsia
            | TargetPlatform::MacOS
            | TargetPlatform::Windows => false,
        };
        let state = app.get_mut(self);
        state.color_scheme = Some(theme.color_scheme.clone());
        state.scrollbar_theme = Some(scrollbar_theme);
        state.use_android_scrollbar = Some(use_android_scrollbar);
        RawScrollbarState::did_change_dependencies(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &MaterialScrollbar) {
        RawScrollbarState::did_update_widget(self, app, old_widget);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let hover_animation_controller = self.hover_animation_controller(app);
        hover_animation_controller.dispose(app);
        RawScrollbarState::dispose(self, app);
        TickerProviderStateMixin::dispose(self, app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        RawScrollbarState::build(self, app, context)
    }
}

impl RawScrollbarStateLeaf for MaterialScrollbarState {
    waft_widgets::raw_scrollbar_state_accessors!();

    fn show_scrollbar(self: Handle<Self>, app: &mut App) -> bool {
        let states = self.states(app);
        self.widget(app)
            .raw_scrollbar
            .thumb_visibility
            .or_else(|| {
                self.scrollbar_theme(app)
                    .thumb_visibility
                    .as_ref()
                    .and_then(|thumb_visibility| thumb_visibility.resolve(&states))
            })
            .unwrap_or(false)
    }

    fn enable_gestures(self: Handle<Self>, app: &mut App) -> bool {
        self.widget(app)
            .raw_scrollbar
            .interactive
            .or(self.scrollbar_theme(app).interactive)
            .unwrap_or(!self.use_android_scrollbar(app))
    }

    fn update_scrollbar_painter(self: Handle<Self>, app: &mut App) {
        let states = self.states(app);
        let color = self.thumb_color(app).resolve(&states);
        let track_color = self.track_color(app).resolve(&states);
        let track_border_color = self.track_border_color(app).resolve(&states);
        let context = self.context(app);
        let text_direction = Directionality::of(app, context);
        let thickness = self.thickness(app).resolve(&states);
        let use_android_scrollbar = self.use_android_scrollbar(app);
        let scrollbar_theme = self.scrollbar_theme(app);
        let radius = self
            .widget(app)
            .raw_scrollbar
            .radius
            .or(scrollbar_theme.radius)
            .or(if use_android_scrollbar {
                None
            } else {
                Some(K_SCROLLBAR_RADIUS)
            });
        let cross_axis_margin =
            scrollbar_theme
                .cross_axis_margin
                .unwrap_or(if use_android_scrollbar {
                    0.0
                } else {
                    K_SCROLLBAR_MARGIN
                });
        let main_axis_margin = scrollbar_theme.main_axis_margin.unwrap_or(0.0);
        let min_length = scrollbar_theme
            .min_thumb_length
            .unwrap_or(K_SCROLLBAR_MIN_LENGTH);
        let padding = MediaQuery::padding_of(app, context);
        let scrollbar_orientation = self.widget(app).raw_scrollbar.scrollbar_orientation;
        let ignore_pointer = !self.enable_gestures(app);

        let painter = RawScrollbarState::scrollbar_painter(self, app);
        painter.set_color(app, color);
        painter.set_track_color(app, track_color);
        painter.set_track_border_color(app, track_border_color);
        painter.set_text_direction(app, Some(text_direction));
        painter.set_thickness(app, thickness);
        painter.set_radius(app, radius);
        painter.set_cross_axis_margin(app, cross_axis_margin);
        painter.set_main_axis_margin(app, main_axis_margin);
        painter.set_min_length(app, min_length);
        painter.set_padding(app, EdgeInsetsGeometry::from(padding));
        painter.set_scrollbar_orientation(app, scrollbar_orientation);
        painter.set_ignore_pointer(app, ignore_pointer);
    }

    fn handle_thumb_press_start(self: Handle<Self>, app: &mut App, local_position: Offset) {
        RawScrollbarState::handle_thumb_press_start(self, app, local_position);
        self.set_state(app, |state| {
            state.drag_is_active = true;
        });
    }

    fn handle_thumb_press_end(
        self: Handle<Self>,
        app: &mut App,
        local_position: Offset,
        velocity: Velocity,
    ) {
        RawScrollbarState::handle_thumb_press_end(self, app, local_position, velocity);
        self.set_state(app, |state| {
            state.drag_is_active = false;
        });
    }

    fn handle_hover(self: Handle<Self>, app: &mut App, event: &PointerHoverEvent) {
        RawScrollbarState::handle_hover(self, app, event);
        // Check if the position of the pointer falls over the painted scrollbar
        if RawScrollbarState::is_pointer_over_scrollbar(self, app, event.position, event.kind) {
            // Pointer is hovering over the scrollbar
            self.set_state(app, |state| {
                state.hover_is_active = true;
            });
            self.hover_animation_controller(app).forward(app, None);
        } else if app.get(self).hover_is_active {
            // Pointer was, but is no longer over painted scrollbar.
            self.set_state(app, |state| {
                state.hover_is_active = false;
            });
            self.hover_animation_controller(app).reverse(app, None);
        }
    }

    fn handle_hover_exit(self: Handle<Self>, app: &mut App) {
        RawScrollbarState::handle_hover_exit(self, app);
        self.set_state(app, |state| {
            state.hover_is_active = false;
        });
        self.hover_animation_controller(app).reverse(app, None);
    }
}

impl TickerProviderObject for MaterialScrollbarState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl TickerProviderStateMixin for MaterialScrollbarState {
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

impl Debug for MaterialScrollbarState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("_MaterialScrollbarState")
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::Cell;

    use waft_cupertino::CupertinoScrollbarState;
    use waft_embedder::Size;
    use waft_embedder::{Canvas, PointerDeviceKind, Rect};
    use waft_gestures::{GestureBinding, PointerEvent, PointerScrollEvent};
    use waft_painting::{AxisDirection, Border, BorderSide, BorderStyle, BoxDecoration};
    use waft_rendering::{HitTestBehavior, RenderViewport, RenderViewportBase};
    use waft_scheduler::SchedulerBinding;
    use waft_widgets::{
        Center, Column, Container, CustomScrollView, FixedScrollMetrics, GestureDetector,
        GlobalKey, Icon, ListView, MediaQueryData, PrimaryScrollController, ScrollBehavior,
        ScrollBehaviorRef, ScrollConfiguration, ScrollController, Scrollable, ScrollableDetails,
        SingleChildScrollView, SizedBox, SliverToBoxAdapter, StatefulBuilder, Text,
    };

    use waft_widgets::testing::{
        TargetPlatformVariant, TestPointer, WidgetTester, test_widgets, test_widgets_variant,
    };

    use super::*;
    use crate::app::{MaterialApp, MaterialScrollBehavior};
    use crate::floating_action_button::FloatingActionButton;
    use crate::icons::Icons;
    use crate::scaffold::Scaffold;
    use crate::test_support::{Drawn, material_app};
    use crate::theme_data::ThemeData;
    use waft_widgets::AnyScrollController;

    pub(crate) const K_SCROLLBAR_FADE_DURATION_FOR_TESTS: Duration = Duration::from_millis(300);
    pub(crate) const K_SCROLLBAR_TIME_TO_FADE_FOR_TESTS: Duration = Duration::from_millis(600);
    pub(crate) const K_ANDROID_THUMB_IDLE_COLOR: u32 = 0xffbcbcbc;
    pub(crate) const K_ANDROID_TRACK_DIMENSIONS: Rect = Rect::from_ltrb(796.0, 0.0, 800.0, 600.0);
    pub(crate) const K_DEFAULT_THUMB_RADIUS: Radius = Radius::circular(8.0);
    pub(crate) const K_DEFAULT_IDLE_THUMB_COLOR: u32 = 0x1a000000;
    pub(crate) const K_TRANSPARENT: u32 = 0x00000000;

    const ALL_PLATFORMS: [TargetPlatform; 6] = [
        TargetPlatform::Android,
        TargetPlatform::Fuchsia,
        TargetPlatform::IOS,
        TargetPlatform::Linux,
        TargetPlatform::MacOS,
        TargetPlatform::Windows,
    ];

    pub(crate) const DESKTOP: [TargetPlatform; 3] = [
        TargetPlatform::Linux,
        TargetPlatform::MacOS,
        TargetPlatform::Windows,
    ];

    pub(crate) fn get_starting_thumb_rect(is_android: bool) -> Rect {
        if is_android {
            // On Android the thumb is slightly different. The thumb is only 4 pixels wide,
            // and has no margin along the side of the viewport.
            Rect::from_ltrb(796.0, 0.0, 800.0, 90.0)
        } else {
            // The Material Design thumb is 8 pixels wide, with a 2
            // pixel margin to the right edge of the viewport.
            Rect::from_ltrb(790.0, 0.0, 798.0, 90.0)
        }
    }

    /// The tests' `NoScrollbarBehavior`, a `MaterialScrollBehavior` with no scrollbar of the
    /// scroll behavior's own over the one a case puts.
    #[derive(Debug)]
    struct NoScrollbarBehavior;

    impl ScrollBehavior for NoScrollbarBehavior {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn get_platform(&self, app: &mut App, context: BuildContext) -> TargetPlatform {
            MaterialScrollBehavior::new().get_platform(app, context)
        }

        fn build_scrollbar(
            &self,
            _app: &mut App,
            _context: BuildContext,
            child: WidgetRef,
            _details: &ScrollableDetails,
        ) -> WidgetRef {
            child
        }

        fn build_overscroll_indicator(
            &self,
            app: &mut App,
            context: BuildContext,
            child: WidgetRef,
            details: &ScrollableDetails,
        ) -> WidgetRef {
            MaterialScrollBehavior::new().build_overscroll_indicator(app, context, child, details)
        }
    }

    /// `ScrollConfiguration(behavior: const NoScrollbarBehavior(), child: child)`.
    pub(crate) fn no_scrollbar<K>(child: impl IntoWidget<K>) -> WidgetRef {
        ScrollConfiguration::new(ScrollBehaviorRef::new(NoScrollbarBehavior), child).into_widget()
    }

    /// `scrollbar_test.dart`'s `_buildBoilerplate`.
    fn build_boilerplate<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Directionality::new(
            waft_embedder::TextDirection::Ltr,
            MediaQuery::new(MediaQueryData::new(), no_scrollbar(child)).into_widget(),
        )
        .into_widget()
    }

    /// `SingleChildScrollView(controller: controller, child: SizedBox(width:, height:))`.
    pub(crate) fn scroll_view(
        controller: Option<ScrollControllerRef>,
        width: f64,
        height: f64,
    ) -> SingleChildScrollView {
        let view = SingleChildScrollView::new().child(SizedBox::new().width(width).height(height));
        match controller {
            Some(controller) => view.controller(controller.clone()),
            None => view,
        }
    }

    /// The Material scrollbar's state.
    fn material_state(tester: &WidgetTester) -> Handle<MaterialScrollbarState> {
        let element = *tester
            .find_by_type::<MaterialScrollbar>()
            .first()
            .expect("a Material scrollbar");
        element
            .state_handle::<MaterialScrollbarState>(&tester.cell.borrow())
            .expect("its state")
    }

    /// What the scrollbar paints over its child: Dart's `paints` matcher on
    /// `find.byType(Scrollbar)`, without the child's own draws.
    pub(crate) fn painted(tester: &WidgetTester) -> Vec<Drawn> {
        painted_by(tester, material_state(tester))
    }

    /// What the scrollbar painter of `state` paints over its child.
    fn painted_by<S: RawScrollbarStateLeaf>(tester: &WidgetTester, state: Handle<S>) -> Vec<Drawn>
    where
        S::Widget: RawScrollbarLeaf,
    {
        let mut app = tester.cell.borrow_mut();
        let painter = RawScrollbarState::scrollbar_painter(state, &app);
        let size = state
            .context(&app)
            .find_render_object(&app)
            .and_then(|render| render.as_box())
            .expect("the scrollbar's render box")
            .size(&app);
        let mut canvas = Canvas::new();
        painter.paint(&mut app, &mut canvas, size);
        Drawn::draws(&canvas.build())
    }

    /// Dart's `paints..rect(rect: track, color:)..line(p1:, p2:, color:)`: the track and
    /// its border, the first two draws.
    ///
    /// Valo records no op for a paint that draws nothing, so a transparent track and border
    /// leave only the thumb in `drawn`; their colours are then read from the painter, and the
    /// track's width from the thickness and margin it is laid out with.
    pub(crate) fn assert_track(
        tester: &WidgetTester,
        drawn: &[Drawn],
        rect: Rect,
        color: u32,
        border: (Offset, Offset),
        border_color: u32,
    ) {
        if color == K_TRANSPARENT && border_color == K_TRANSPARENT {
            assert_eq!(drawn.len(), 1, "only the thumb in {drawn:?}");
            let state = material_state(tester);
            let app = tester.cell.borrow();
            let painter = RawScrollbarState::scrollbar_painter(state, &app);
            assert_eq!(painter.track_color(&app).to_argb32(), K_TRANSPARENT);
            assert_eq!(painter.track_border_color(&app).to_argb32(), K_TRANSPARENT);
            assert_eq!(
                rect.width(),
                painter.thickness(&app) + 2.0 * painter.cross_axis_margin(&app),
                "the track's width"
            );
            return;
        }
        assert_eq!(
            drawn.first(),
            Some(&Drawn::Rect(color, rect)),
            "the track in {drawn:?}"
        );
        // A stroked path's recorded bounds take in the stroke, so the border is matched by
        // its centre and by covering both ends of the line.
        let line = Rect::from_points(border.0, border.1);
        let Some(Drawn::Path(painted_color, bounds)) = drawn.get(1) else {
            panic!("the track border in {drawn:?}");
        };
        assert_eq!(*painted_color, border_color, "the track border's colour");
        assert_eq!(bounds.center(), line.center(), "the track border's line");
        assert!(
            bounds.contains(border.0) && bounds.contains(border.1),
            "the track border {bounds:?} covers {line:?}"
        );
    }

    /// Dart's `paints..rect(rect: thumb, color:)`: a square thumb, the last draw.
    pub(crate) fn assert_thumb_rect(drawn: &[Drawn], rect: Rect, color: u32) {
        assert_eq!(
            drawn.last(),
            Some(&Drawn::Rect(color, rect)),
            "the thumb in {drawn:?}"
        );
    }

    /// Dart's `paints..rrect(rrect: RRect.fromRectAndRadius(rect, radius), color:)`: a
    /// rounded thumb, the last draw.
    pub(crate) fn assert_thumb_rrect(
        tester: &WidgetTester,
        rect: Rect,
        radius: Radius,
        color: u32,
    ) {
        let drawn = painted(tester);
        assert_eq!(
            drawn.last(),
            Some(&Drawn::Path(color, rect)),
            "the thumb in {drawn:?}"
        );
        let state = material_state(tester);
        let app = tester.cell.borrow();
        assert_eq!(
            RawScrollbarState::scrollbar_painter(state, &app).radius(&app),
            Some(radius)
        );
    }

    /// Whether the thumb is painted: Dart's `paints..rect()` against `isNot(paints..rect())`.
    pub(crate) fn paints_thumb(tester: &WidgetTester) -> bool {
        !painted(tester).is_empty()
    }

    /// Dart's `tester.startGesture(at)` and a settle.
    pub(crate) fn start_gesture(tester: &mut WidgetTester, at: Offset) -> TestPointer {
        let mut pointer = TestPointer::touch(1);
        tester.down(&mut pointer, at);
        pointer
    }

    /// Dart's `gesture.moveBy(by)`.
    pub(crate) fn move_by(tester: &mut WidgetTester, pointer: &mut TestPointer, by: Offset) {
        let at = pointer.position + by;
        tester.move_to(pointer, at, true);
    }

    /// Dart's `tester.createGesture(kind: mouse)`, `addPointer()` and `moveTo(at)`.
    pub(crate) fn hover_at(tester: &mut WidgetTester, at: Offset) -> TestPointer {
        let mut mouse = TestPointer::mouse(7, 0);
        tester.add_pointer(&mut mouse, Offset::ZERO);
        tester.move_to(&mut mouse, at, false);
        mouse
    }

    /// The scroll offset of `controller`.
    fn offset(tester: &WidgetTester, controller: AnyScrollController) -> f64 {
        controller.offset(&tester.cell.borrow())
    }

    fn controller(tester: &WidgetTester) -> ScrollControllerRef {
        ScrollController::default(&mut tester.cell.borrow_mut()).into_erased()
    }

    fn m2(tester: &WidgetTester) -> ThemeData {
        ThemeData::builder()
            .use_material3(false)
            .build(&tester.cell.borrow())
    }

    #[test]
    fn on_first_render_with_thumb_visibility_true_the_thumb_shows() {
        test_widgets(|tester| {
            let controller = controller(tester);
            let theme = ThemeData::new(&tester.cell.borrow());
            tester.pump_widget(build_boilerplate(Theme::new(
                theme,
                Scrollbar::new(scroll_view(Some(controller.clone()), 4000.0, 4000.0))
                    .thumb_visibility(true)
                    .controller(controller),
            )));
            tester.pump_and_settle();
            assert!(paints_thumb(tester));
        });
    }

    #[test]
    fn on_first_render_with_thumb_visibility_true_the_thumb_shows_with_primary_scroll_controller() {
        test_widgets(|tester| {
            let controller = controller(tester);
            let theme = ThemeData::new(&tester.cell.borrow());
            tester.pump_widget(build_boilerplate(Theme::new(
                theme,
                PrimaryScrollController::new(
                    controller,
                    Scrollbar::new(scroll_view(None, 4000.0, 4000.0).primary(true))
                        .thumb_visibility(true),
                ),
            )));
            tester.pump_and_settle();
            assert!(paints_thumb(tester));
        });
    }

    #[test]
    fn on_first_render_with_thumb_visibility_false_the_thumb_is_hidden() {
        test_widgets(|tester| {
            let controller = controller(tester);
            let theme = ThemeData::new(&tester.cell.borrow());
            tester.pump_widget(build_boilerplate(Theme::new(
                theme,
                Scrollbar::new(scroll_view(Some(controller.clone()), 4000.0, 4000.0))
                    .thumb_visibility(false)
                    .controller(controller),
            )));
            tester.pump_and_settle();
            assert!(!paints_thumb(tester));
        });
    }

    #[test]
    fn toggling_thumb_visibility_while_not_scrolling_fades_the_thumb_in_and_out() {
        test_widgets(|tester| {
            let controller = controller(tester);
            let view_with_scroll = |tester: &WidgetTester, thumb_visibility: bool| {
                let theme = ThemeData::new(&tester.cell.borrow());
                build_boilerplate(Theme::new(
                    theme,
                    Scrollbar::new(scroll_view(Some(controller.clone()), 4000.0, 4000.0))
                        .thumb_visibility(thumb_visibility)
                        .controller(controller.clone()),
                ))
            };
            let tree = view_with_scroll(tester, true);
            tester.pump_widget(tree);
            tester.pump_and_settle();
            assert!(paints_thumb(tester));

            let tree = view_with_scroll(tester, false);
            tester.pump_widget(tree);
            tester.pump_and_settle();
            assert!(!paints_thumb(tester));

            let tree = view_with_scroll(tester, true);
            tester.pump_widget(tree);
            tester.pump_and_settle();
            assert!(
                paints_thumb(tester),
                "set thumbVisibility: true, the thumb shows directly"
            );
        });
    }

    #[test]
    fn scrollbar_respects_thickness_and_radius() {
        test_widgets(|tester| {
            let controller = controller(tester);
            let view_with_scroll = |tester: &WidgetTester, radius: Option<Radius>| {
                let theme = ThemeData::new(&tester.cell.borrow());
                let mut scrollbar =
                    Scrollbar::new(scroll_view(Some(controller.clone()), 1600.0, 1200.0))
                        .controller(controller.clone())
                        .thickness(20.0);
                if let Some(radius) = radius {
                    scrollbar = scrollbar.radius(radius);
                }
                build_boilerplate(Theme::new(theme, scrollbar))
            };

            // Scroll a bit to cause the scrollbar thumb to be shown;
            // undo the scroll to put the thumb back at the top.
            let tree = view_with_scroll(tester, None);
            tester.pump_widget(tree);
            const SCROLL_AMOUNT: f64 = 10.0;
            let mut scroll_gesture = start_gesture(tester, Offset::new(400.0, 300.0));
            move_by(
                tester,
                &mut scroll_gesture,
                Offset::new(0.0, -SCROLL_AMOUNT),
            );
            tester.pump();
            tester.pump_by(Duration::from_millis(500));
            move_by(tester, &mut scroll_gesture, Offset::new(0.0, SCROLL_AMOUNT));
            tester.pump();
            tester.up(&scroll_gesture);
            tester.pump();

            // Long press on the scrollbar thumb and expect it to grow
            let drawn = painted(tester);
            assert_track(
                tester,
                &drawn,
                Rect::from_ltrb(780.0, 0.0, 800.0, 600.0),
                K_TRANSPARENT,
                (Offset::new(780.0, 0.0), Offset::new(780.0, 600.0)),
                K_TRANSPARENT,
            );
            assert_thumb_rect(
                &drawn,
                Rect::from_ltrb(780.0, 0.0, 800.0, 300.0),
                K_ANDROID_THUMB_IDLE_COLOR,
            );
            let tree = view_with_scroll(tester, Some(Radius::circular(10.0)));
            tester.pump_widget(tree);
            assert_thumb_rrect(
                tester,
                Rect::from_ltrb(780.0, 0.0, 800.0, 300.0),
                Radius::circular(10.0),
                K_ANDROID_THUMB_IDLE_COLOR,
            );

            tester.pump_and_settle();
        });
    }

    #[test]
    fn tapping_the_track_area_pages_the_scroll_view() {
        test_widgets(|tester| {
            let scroll_controller = controller(tester);
            tester.pump_widget(
                Directionality::new(
                    waft_embedder::TextDirection::Ltr,
                    MediaQuery::new(
                        MediaQueryData::new(),
                        Scrollbar::new(scroll_view(
                            Some(scroll_controller.clone()),
                            1000.0,
                            1000.0,
                        ))
                        .interactive(true)
                        .thumb_visibility(true)
                        .controller(scroll_controller.clone()),
                    )
                    .into_widget(),
                )
                .into_widget(),
            );

            tester.pump_and_settle();
            assert_eq!(offset(tester, scroll_controller.handle()), 0.0);
            let border = (Offset::new(796.0, 0.0), Offset::new(796.0, 600.0));
            let drawn = painted(tester);
            assert_track(
                tester,
                &drawn,
                K_ANDROID_TRACK_DIMENSIONS,
                K_TRANSPARENT,
                border,
                K_TRANSPARENT,
            );
            assert_thumb_rect(
                &drawn,
                Rect::from_ltrb(796.0, 0.0, 800.0, 360.0),
                K_ANDROID_THUMB_IDLE_COLOR,
            );

            // Tap on the track area below the thumb.
            tester.tap(Offset::new(796.0, 550.0), 1);
            tester.pump_and_settle();

            assert_eq!(offset(tester, scroll_controller.handle()), 400.0);
            let drawn = painted(tester);
            assert_track(
                tester,
                &drawn,
                K_ANDROID_TRACK_DIMENSIONS,
                K_TRANSPARENT,
                border,
                K_TRANSPARENT,
            );
            assert_thumb_rect(
                &drawn,
                Rect::from_ltrb(796.0, 240.0, 800.0, 600.0),
                K_ANDROID_THUMB_IDLE_COLOR,
            );

            // Tap on the track area above the thumb.
            tester.tap(Offset::new(796.0, 50.0), 1);
            tester.pump_and_settle();

            assert_eq!(offset(tester, scroll_controller.handle()), 0.0);
            let drawn = painted(tester);
            assert_track(
                tester,
                &drawn,
                K_ANDROID_TRACK_DIMENSIONS,
                K_TRANSPARENT,
                border,
                K_TRANSPARENT,
            );
            assert_thumb_rect(
                &drawn,
                Rect::from_ltrb(796.0, 0.0, 800.0, 360.0),
                K_ANDROID_THUMB_IDLE_COLOR,
            );
        });
    }

    #[test]
    fn scrollbar_never_goes_away_until_finger_lift() {
        test_widgets(|tester| {
            let theme = ThemeData::new(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Scrollbar::new(scroll_view(None, 4000.0, 4000.0)).into_widget(),
            ));
            let mut gesture = start_gesture(tester, Offset::new(400.0, 300.0));
            move_by(tester, &mut gesture, Offset::new(0.0, -20.0));
            tester.pump();
            // Scrollbar fully showing
            tester.pump_by(Duration::from_millis(500));
            let border = (Offset::new(796.0, 0.0), Offset::new(796.0, 600.0));
            let thumb = Rect::from_ltrb(796.0, 3.0, 800.0, 93.0);
            let drawn = painted(tester);
            assert_track(
                tester,
                &drawn,
                K_ANDROID_TRACK_DIMENSIONS,
                K_TRANSPARENT,
                border,
                K_TRANSPARENT,
            );
            assert_thumb_rect(&drawn, thumb, K_ANDROID_THUMB_IDLE_COLOR);

            tester.pump_by(Duration::from_secs(3));
            tester.pump_by(Duration::from_secs(3));
            // Still there.
            let drawn = painted(tester);
            assert_track(
                tester,
                &drawn,
                K_ANDROID_TRACK_DIMENSIONS,
                K_TRANSPARENT,
                border,
                K_TRANSPARENT,
            );
            assert_thumb_rect(&drawn, thumb, K_ANDROID_THUMB_IDLE_COLOR);

            tester.up(&gesture);
            tester.pump_by(K_SCROLLBAR_TIME_TO_FADE_FOR_TESTS);
            tester.pump_by(K_SCROLLBAR_FADE_DURATION_FOR_TESTS / 2);

            // Opacity going down now.
            let drawn = painted(tester);
            assert_track(
                tester,
                &drawn,
                K_ANDROID_TRACK_DIMENSIONS,
                K_TRANSPARENT,
                border,
                K_TRANSPARENT,
            );
            assert_thumb_rect(&drawn, thumb, 0xc6bcbcbc);
        });
    }

    #[test]
    fn scrollbar_thumb_can_be_dragged() {
        test_widgets(|tester| {
            let scroll_controller = controller(tester);
            let theme = m2(tester);
            tester.pump_widget(material_app(
                theme,
                PrimaryScrollController::new(
                    scroll_controller.clone(),
                    Scrollbar::new(scroll_view(None, 4000.0, 4000.0))
                        .interactive(true)
                        .thumb_visibility(true)
                        .controller(scroll_controller.clone()),
                )
                .into_widget(),
            ));
            tester.pump_and_settle();
            assert_eq!(offset(tester, scroll_controller.handle()), 0.0);
            let border = (Offset::new(796.0, 0.0), Offset::new(796.0, 600.0));
            let drawn = painted(tester);
            assert_track(
                tester,
                &drawn,
                K_ANDROID_TRACK_DIMENSIONS,
                K_TRANSPARENT,
                border,
                K_TRANSPARENT,
            );
            assert_thumb_rect(
                &drawn,
                get_starting_thumb_rect(true),
                K_ANDROID_THUMB_IDLE_COLOR,
            );

            // Drag the thumb down to scroll down.
            const SCROLL_AMOUNT: f64 = 10.0;
            let mut drag_scrollbar_gesture = start_gesture(tester, Offset::new(797.0, 45.0));
            tester.pump_and_settle();

            let drawn = painted(tester);
            assert_track(
                tester,
                &drawn,
                K_ANDROID_TRACK_DIMENSIONS,
                K_TRANSPARENT,
                border,
                K_TRANSPARENT,
            );
            // Drag color
            assert_thumb_rect(&drawn, get_starting_thumb_rect(true), 0x99000000);

            move_by(
                tester,
                &mut drag_scrollbar_gesture,
                Offset::new(0.0, SCROLL_AMOUNT),
            );
            tester.pump_and_settle();
            tester.up(&drag_scrollbar_gesture);
            tester.pump_and_settle();

            // The view has scrolled more than it would have by a swipe pointer of the
            // same distance.
            assert!(offset(tester, scroll_controller.handle()) > SCROLL_AMOUNT * 2.0);
            let drawn = painted(tester);
            assert_track(
                tester,
                &drawn,
                K_ANDROID_TRACK_DIMENSIONS,
                K_TRANSPARENT,
                border,
                K_TRANSPARENT,
            );
            assert_thumb_rect(
                &drawn,
                Rect::from_ltrb(796.0, 10.0, 800.0, 100.0),
                K_ANDROID_THUMB_IDLE_COLOR,
            );
        });
    }

    /// Dart's `MaterialApp(theme: theme, home: SingleChildScrollView(...))` on a desktop,
    /// where `MaterialScrollBehavior` puts a `Scrollbar` over the scroll view with the
    /// scroll view's controller.
    fn desktop_app(tester: &WidgetTester, theme: ThemeData) -> WidgetRef {
        let controller = controller(tester);
        material_app(
            theme,
            no_scrollbar(
                Scrollbar::new(scroll_view(Some(controller.clone()), 4000.0, 4000.0))
                    .controller(controller),
            ),
        )
    }

    #[test]
    fn scrollbar_thumb_color_completes_a_hover_animation() {
        for platform in DESKTOP {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let theme =
                    ThemeData::builder()
                        .use_material3(false)
                        .scrollbar_theme(ScrollbarThemeData::new().thumb_visibility(
                            <dyn WidgetStateProperty<Option<bool>>>::all(Some(true)),
                        ))
                        .build(&tester.cell.borrow());
                let app = desktop_app(tester, theme);
                tester.pump_widget(app);
                tester.pump_and_settle();
                assert_thumb_rrect(
                    tester,
                    get_starting_thumb_rect(false),
                    K_DEFAULT_THUMB_RADIUS,
                    K_DEFAULT_IDLE_THUMB_COLOR,
                );

                hover_at(tester, Offset::new(794.0, 5.0));
                tester.pump_and_settle();

                // Hover color
                assert_thumb_rrect(
                    tester,
                    get_starting_thumb_rect(false),
                    K_DEFAULT_THUMB_RADIUS,
                    0x80000000,
                );
            });
        }
    }

    /// The theme of the hover cases: Material 2 unless `use_material3`, the thumb always
    /// shown, the track only while hovered, and `thickness` if given.
    fn hover_track_theme(
        tester: &WidgetTester,
        use_material3: bool,
        thickness: Option<WidgetStatePropertyRef<Option<f64>>>,
    ) -> ThemeData {
        let mut scrollbar_theme = ScrollbarThemeData::new()
            .thumb_visibility(<dyn WidgetStateProperty<Option<bool>>>::all(Some(true)))
            .track_visibility(<dyn WidgetStateProperty<Option<bool>>>::resolve_with(
                |states: &WidgetStates| Some(states.contains(&WidgetState::Hovered)),
            ));
        if let Some(thickness) = thickness {
            scrollbar_theme = scrollbar_theme.thickness(thickness);
        }
        ThemeData::builder()
            .use_material3(use_material3)
            .scrollbar_theme(scrollbar_theme)
            .build(&tester.cell.borrow())
    }

    #[test]
    fn hover_animation_is_not_triggered_by_tap_gestures() {
        test_widgets_variant(
            TargetPlatformVariant::only(TargetPlatform::Linux),
            |tester| {
                let theme = hover_track_theme(tester, false, None);
                let app = desktop_app(tester, theme);
                tester.pump_widget(app);
                tester.pump_and_settle();
                assert_thumb_rrect(
                    tester,
                    get_starting_thumb_rect(false),
                    K_DEFAULT_THUMB_RADIUS,
                    K_DEFAULT_IDLE_THUMB_COLOR,
                );
                tester.tap(Offset::new(794.0, 5.0), 1);
                tester.pump_and_settle();

                // Tapping triggers a hover enter event. In this case, the Scrollbar should
                // be unchanged since it ignores hover events that aren't from a mouse.
                assert_thumb_rrect(
                    tester,
                    get_starting_thumb_rect(false),
                    K_DEFAULT_THUMB_RADIUS,
                    K_DEFAULT_IDLE_THUMB_COLOR,
                );

                // Now trigger hover with a mouse.
                hover_at(tester, Offset::new(794.0, 5.0));
                tester.pump();

                let drawn = painted(tester);
                assert_track(
                    tester,
                    &drawn,
                    Rect::from_ltrb(784.0, 0.0, 800.0, 600.0),
                    0x08000000,
                    (Offset::new(784.0, 0.0), Offset::new(784.0, 600.0)),
                    K_DEFAULT_IDLE_THUMB_COLOR,
                );
                // Scrollbar thumb is larger; hover color
                assert_thumb_rrect(
                    tester,
                    Rect::from_ltrb(786.0, 0.0, 798.0, 90.0),
                    K_DEFAULT_THUMB_RADIUS,
                    0x80000000,
                );
            },
        );
    }

    #[test]
    fn scrollbar_theme_data_thickness_replaces_hover_thickness() {
        for platform in DESKTOP {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let thickness = <dyn WidgetStateProperty<Option<f64>>>::resolve_with(
                    |states: &WidgetStates| {
                        if states.contains(&WidgetState::Hovered) {
                            return Some(40.0);
                        }
                        // Default thickness
                        Some(8.0)
                    },
                );
                let theme = hover_track_theme(tester, false, Some(thickness));
                let app = desktop_app(tester, theme);
                tester.pump_widget(app);
                tester.pump_and_settle();
                assert_thumb_rrect(
                    tester,
                    get_starting_thumb_rect(false),
                    K_DEFAULT_THUMB_RADIUS,
                    K_DEFAULT_IDLE_THUMB_COLOR,
                );

                hover_at(tester, Offset::new(794.0, 5.0));
                tester.pump();

                let drawn = painted(tester);
                assert_track(
                    tester,
                    &drawn,
                    Rect::from_ltrb(756.0, 0.0, 800.0, 600.0),
                    0x08000000,
                    (Offset::new(756.0, 0.0), Offset::new(756.0, 600.0)),
                    K_DEFAULT_IDLE_THUMB_COLOR,
                );
                // Scrollbar thumb is larger; hover color
                assert_thumb_rrect(
                    tester,
                    Rect::from_ltrb(758.0, 0.0, 798.0, 90.0),
                    K_DEFAULT_THUMB_RADIUS,
                    0x80000000,
                );
            });
        }
    }

    #[test]
    fn scrollbar_track_visibility_on_hovered() {
        for platform in DESKTOP {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let theme = hover_track_theme(tester, false, None);
                let app = desktop_app(tester, theme);
                tester.pump_widget(app);
                tester.pump_and_settle();
                assert_thumb_rrect(
                    tester,
                    get_starting_thumb_rect(false),
                    K_DEFAULT_THUMB_RADIUS,
                    K_DEFAULT_IDLE_THUMB_COLOR,
                );

                hover_at(tester, Offset::new(794.0, 5.0));
                tester.pump();

                let drawn = painted(tester);
                assert_track(
                    tester,
                    &drawn,
                    Rect::from_ltrb(784.0, 0.0, 800.0, 600.0),
                    0x08000000,
                    (Offset::new(784.0, 0.0), Offset::new(784.0, 600.0)),
                    K_DEFAULT_IDLE_THUMB_COLOR,
                );
                // Scrollbar thumb is larger; hover color
                assert_thumb_rrect(
                    tester,
                    Rect::from_ltrb(786.0, 0.0, 798.0, 90.0),
                    K_DEFAULT_THUMB_RADIUS,
                    0x80000000,
                );
            });
        }
    }

    #[test]
    fn adaptive_scrollbar() {
        test_widgets(|tester| {
            let view_with_scroll = |tester: &WidgetTester, platform: TargetPlatform| {
                let theme = ThemeData::builder()
                    .platform(platform)
                    .build(&tester.cell.borrow());
                build_boilerplate(Theme::new(
                    theme,
                    Scrollbar::new(scroll_view(None, 4000.0, 4000.0)),
                ))
            };

            let tree = view_with_scroll(tester, TargetPlatform::Android);
            tester.pump_widget(tree);
            tester.drag_from(Offset::new(400.0, 300.0), Offset::new(0.0, -10.0), 1);
            tester.pump();
            // Scrollbar fully showing
            tester.pump_by(Duration::from_millis(500));
            assert!(paints_thumb(tester));
            assert!(tester.find_by_type::<CupertinoScrollbar>().is_empty());

            let tree = view_with_scroll(tester, TargetPlatform::IOS);
            tester.pump_widget(tree);
            assert_eq!(tester.find_by_type::<CupertinoScrollbar>().len(), 1);
            assert!(tester.find_by_type::<MaterialScrollbar>().is_empty());
            tester.pump_and_settle();

            // Only iOS adapts: macOS keeps the Material scrollbar.
            let tree = view_with_scroll(tester, TargetPlatform::MacOS);
            tester.pump_widget(tree);
            assert!(tester.find_by_type::<CupertinoScrollbar>().is_empty());
            assert_eq!(tester.find_by_type::<MaterialScrollbar>().len(), 1);
        });
    }

    #[test]
    fn scrollbar_passes_controller_to_cupertino_scrollbar() {
        test_widgets_variant(TargetPlatformVariant::only(TargetPlatform::IOS), |tester| {
            let controller = controller(tester);
            let theme = ThemeData::new(&tester.cell.borrow());
            tester.pump_widget(build_boilerplate(Theme::new(
                theme,
                Scrollbar::new(scroll_view(Some(controller.clone()), 4000.0, 4000.0))
                    .controller(controller.clone()),
            )));
            let cupertino = *tester
                .find_by_type::<CupertinoScrollbar>()
                .first()
                .expect("a Cupertino scrollbar on iOS");
            let passed = tester.widget(cupertino, |scrollbar: &CupertinoScrollbar| {
                scrollbar.raw_scrollbar_data().controller.clone()
            });
            assert_eq!(passed, Some(controller));
        });
    }

    #[test]
    fn scrollbar_dragging_can_be_disabled() {
        test_widgets_variant(
            TargetPlatformVariant::only(TargetPlatform::Fuchsia),
            |tester| {
                let scroll_controller = controller(tester);
                let theme = m2(tester);
                tester.pump_widget(material_app(
                    theme,
                    PrimaryScrollController::new(
                        scroll_controller.clone(),
                        Scrollbar::new(scroll_view(None, 4000.0, 4000.0))
                            .interactive(false)
                            .thumb_visibility(true)
                            .controller(scroll_controller.clone()),
                    )
                    .into_widget(),
                ));
                tester.pump_and_settle();
                assert_eq!(offset(tester, scroll_controller.handle()), 0.0);
                let drawn = painted(tester);
                assert_track(
                    tester,
                    &drawn,
                    Rect::from_ltrb(788.0, 0.0, 800.0, 600.0),
                    K_TRANSPARENT,
                    (Offset::new(788.0, 0.0), Offset::new(788.0, 600.0)),
                    K_TRANSPARENT,
                );
                assert_thumb_rrect(
                    tester,
                    get_starting_thumb_rect(false),
                    K_DEFAULT_THUMB_RADIUS,
                    K_DEFAULT_IDLE_THUMB_COLOR,
                );

                // Try to drag the thumb down.
                const SCROLL_AMOUNT: f64 = 10.0;
                let mut drag_scrollbar_thumb_gesture =
                    start_gesture(tester, Offset::new(797.0, 45.0));
                tester.pump_and_settle();
                move_by(
                    tester,
                    &mut drag_scrollbar_thumb_gesture,
                    Offset::new(0.0, SCROLL_AMOUNT),
                );
                tester.pump_and_settle();
                tester.up(&drag_scrollbar_thumb_gesture);
                tester.pump_and_settle();
                // Dragging on the thumb does not change the offset.
                assert_eq!(offset(tester, scroll_controller.handle()), 0.0);

                // Drag in the track area to validate pass through to scrollable.
                let mut drag_pass_through_track = start_gesture(tester, Offset::new(797.0, 250.0));
                move_by(
                    tester,
                    &mut drag_pass_through_track,
                    Offset::new(0.0, -SCROLL_AMOUNT),
                );
                tester.pump_and_settle();
                tester.up(&drag_pass_through_track);
                tester.pump_and_settle();
                // The scroll view received the drag.
                assert_eq!(offset(tester, scroll_controller.handle()), SCROLL_AMOUNT);

                // Tap on the track to validate the scroll view will not page.
                tester.tap(Offset::new(797.0, 200.0), 1);
                tester.pump_and_settle();
                // The offset should not have changed.
                assert_eq!(offset(tester, scroll_controller.handle()), SCROLL_AMOUNT);
            },
        );
    }

    #[test]
    fn scrollbar_dragging_is_disabled_by_default_on_android() {
        test_widgets(|tester| {
            let tap_count = Rc::new(Cell::new(0));
            let scroll_controller = controller(tester);
            let theme = ThemeData::new(&tester.cell.borrow());
            let on_tap = {
                let tap_count = Rc::clone(&tap_count);
                Listener::new(move |_app: &mut App| tap_count.set(tap_count.get() + 1))
            };
            tester.pump_widget(material_app(
                theme,
                PrimaryScrollController::new(
                    scroll_controller.clone(),
                    Scrollbar::new(
                        SingleChildScrollView::new()
                            .drag_start_behavior(waft_gestures::DragStartBehavior::Down)
                            .child(
                                GestureDetector::new()
                                    .behavior(HitTestBehavior::Opaque)
                                    .on_tap(on_tap)
                                    .child(SizedBox::new().width(4000.0).height(4000.0)),
                            ),
                    )
                    .thumb_visibility(true)
                    .controller(scroll_controller.clone()),
                )
                .into_widget(),
            ));
            tester.pump_and_settle();
            assert_eq!(offset(tester, scroll_controller.handle()), 0.0);
            let border = (Offset::new(796.0, 0.0), Offset::new(796.0, 600.0));
            let drawn = painted(tester);
            assert_track(
                tester,
                &drawn,
                K_ANDROID_TRACK_DIMENSIONS,
                K_TRANSPARENT,
                border,
                K_TRANSPARENT,
            );
            assert_thumb_rect(
                &drawn,
                get_starting_thumb_rect(true),
                K_ANDROID_THUMB_IDLE_COLOR,
            );

            // Try to drag the thumb down.
            const SCROLL_AMOUNT: f64 = 50.0;
            // Dart's `dragFrom(touchSlopY: 0.0)`: one move, with no slop to cross.
            let drag_without_slop = |tester: &mut WidgetTester, offset: Offset| {
                let mut gesture = start_gesture(tester, Offset::new(797.0, 45.0));
                move_by(tester, &mut gesture, offset);
                tester.up(&gesture);
            };
            drag_without_slop(tester, Offset::new(0.0, SCROLL_AMOUNT));
            tester.pump_and_settle();
            // Dragging on the thumb does not change the offset.
            assert_eq!(offset(tester, scroll_controller.handle()), 0.0);
            assert_eq!(tap_count.get(), 0);

            // Try to drag up in the thumb area to validate pass through to scrollable.
            tester.drag_from(
                Offset::new(797.0, 45.0),
                Offset::new(0.0, -SCROLL_AMOUNT),
                1,
            );
            tester.pump_and_settle();
            // The scroll view received the drag.
            assert_eq!(offset(tester, scroll_controller.handle()), SCROLL_AMOUNT);
            assert_eq!(tap_count.get(), 0);

            // Drag in the track area to validate pass through to scrollable.
            drag_without_slop(tester, Offset::new(0.0, -SCROLL_AMOUNT));
            tester.pump_and_settle();
            // The scroll view received the drag.
            assert_eq!(
                offset(tester, scroll_controller.handle()),
                SCROLL_AMOUNT * 2.0
            );
            assert_eq!(tap_count.get(), 0);

            // Tap on the thumb to validate the scroll view receives a click.
            tester.tap(Offset::new(797.0, 45.0), 1);
            tester.pump_and_settle();
            assert_eq!(tap_count.get(), 1);

            // Tap on the track to validate the scroll view will not page and receives a click.
            tester.tap(Offset::new(797.0, 400.0), 1);
            tester.pump_and_settle();
            // The offset should not have changed.
            assert_eq!(
                offset(tester, scroll_controller.handle()),
                SCROLL_AMOUNT * 2.0
            );
            assert_eq!(tap_count.get(), 2);
        });
    }

    #[test]
    fn scrollbar_scroll_orientation_works_correctly() {
        test_widgets(|tester| {
            let scroll_controller = controller(tester);
            let theme = ThemeData::builder()
                .platform(TargetPlatform::Android)
                .build(&tester.cell.borrow());
            tester.pump_widget(build_boilerplate(Theme::new(
                theme,
                PrimaryScrollController::new(
                    scroll_controller.clone(),
                    Scrollbar::new(scroll_view(None, 4000.0, 4000.0))
                        .interactive(true)
                        .thumb_visibility(true)
                        .scrollbar_orientation(ScrollbarOrientation::Left)
                        .controller(scroll_controller),
                ),
            )));
            tester.pump_and_settle();

            let drawn = painted(tester);
            assert_track(
                tester,
                &drawn,
                Rect::from_ltrb(0.0, 0.0, 4.0, 600.0),
                K_TRANSPARENT,
                (Offset::new(4.0, 0.0), Offset::new(4.0, 600.0)),
                K_TRANSPARENT,
            );
            assert_thumb_rect(
                &drawn,
                Rect::from_ltrb(0.0, 0.0, 4.0, 90.0),
                K_ANDROID_THUMB_IDLE_COLOR,
            );
        });
    }

    #[test]
    fn scrollbar_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            let theme = ThemeData::new(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                waft_widgets::Center::new()
                    .child(SizedBox::shrink().child(Scrollbar::new(SingleChildScrollView::new())))
                    .into_widget(),
            ));
            let scrollbar = *tester
                .find_by_type::<Scrollbar>()
                .first()
                .expect("the scrollbar");
            let app = tester.cell.borrow();
            let size = scrollbar
                .find_render_object(&app)
                .and_then(|render| render.as_box())
                .expect("a render box")
                .size(&app);
            assert_eq!(size, waft_embedder::Size::ZERO);
        });
    }

    #[test]
    fn material_3_scrollbar_colors_come_from_the_color_scheme_on_surface() {
        test_widgets_variant(
            TargetPlatformVariant::only(TargetPlatform::Linux),
            |tester| {
                let theme = hover_track_theme(tester, true, None);
                let on_surface = theme.color_scheme.on_surface;
                let app = desktop_app(tester, theme);
                tester.pump_widget(app);
                tester.pump_and_settle();
                #[expect(deprecated, reason = "Dart calls the deprecated `Color.withOpacity`")]
                let idle = on_surface.with_opacity(0.1).to_argb32();
                assert_thumb_rrect(
                    tester,
                    get_starting_thumb_rect(false),
                    K_DEFAULT_THUMB_RADIUS,
                    idle,
                );

                hover_at(tester, Offset::new(794.0, 5.0));
                tester.pump();
                #[expect(deprecated, reason = "Dart calls the deprecated `Color.withOpacity`")]
                let (track, border, hover) = (
                    on_surface.with_opacity(0.03).to_argb32(),
                    on_surface.with_opacity(0.1).to_argb32(),
                    on_surface.with_opacity(0.5).to_argb32(),
                );
                let drawn = painted(tester);
                assert_track(
                    tester,
                    &drawn,
                    Rect::from_ltrb(784.0, 0.0, 800.0, 600.0),
                    track,
                    (Offset::new(784.0, 0.0), Offset::new(784.0, 600.0)),
                    border,
                );
                assert_thumb_rrect(
                    tester,
                    Rect::from_ltrb(786.0, 0.0, 798.0, 90.0),
                    K_DEFAULT_THUMB_RADIUS,
                    hover,
                );
            },
        );
    }

    #[test]
    fn a_hovered_thumb_without_a_track_fades_to_the_hover_color() {
        test_widgets_variant(
            TargetPlatformVariant::only(TargetPlatform::Windows),
            |tester| {
                let theme =
                    ThemeData::builder()
                        .use_material3(false)
                        .scrollbar_theme(ScrollbarThemeData::new().thumb_visibility(
                            <dyn WidgetStateProperty<Option<bool>>>::all(Some(true)),
                        ))
                        .build(&tester.cell.borrow());
                let app = desktop_app(tester, theme);
                tester.pump_widget(app);
                tester.pump_and_settle();

                let mouse = hover_at(tester, Offset::new(794.0, 5.0));
                assert_eq!(mouse.kind, PointerDeviceKind::Mouse);
                // Halfway through the 200ms hover animation the thumb is halfway between the
                // idle and the hover colour.
                tester.pump();
                tester.pump_by(Duration::from_millis(100));
                let halfway = Color::lerp(
                    Some(Color::new(K_DEFAULT_IDLE_THUMB_COLOR)),
                    Some(Color::new(0x80000000)),
                    0.5,
                )
                .expect("both colors are given")
                .to_argb32();
                assert_thumb_rrect(
                    tester,
                    get_starting_thumb_rect(false),
                    K_DEFAULT_THUMB_RADIUS,
                    halfway,
                );
            },
        );
    }

    // 'MaterialScrollBehavior flings on different platforms'.
    #[test]
    fn material_scroll_behavior_flings_on_different_platforms() {
        test_widgets(|tester| {
            let get_scroll_offset = |tester: &WidgetTester| {
                let viewport = *tester.all::<RenderViewport>().last().expect("a viewport");
                let app = tester.cell.borrow();
                viewport.offset(&app).pixels(&app)
            };
            let reset_scroll_offset = |tester: &WidgetTester| {
                let viewport = tester.first::<RenderViewport>();
                let mut app = tester.cell.borrow_mut();
                let position = viewport.offset(&app);
                position.jump_to(&mut app, 0.0);
            };
            let pump_test = |tester: &mut WidgetTester, platform: TargetPlatform| {
                let theme = ThemeData::builder()
                    .platform(platform)
                    .build(&tester.cell.borrow());
                let every_kind = HashSet::from([
                    PointerDeviceKind::Touch,
                    PointerDeviceKind::Mouse,
                    PointerDeviceKind::Stylus,
                    PointerDeviceKind::InvertedStylus,
                    PointerDeviceKind::Trackpad,
                    PointerDeviceKind::Unknown,
                ]);
                tester.pump_widget(
                    MaterialApp::new()
                        .scroll_behavior(ScrollBehaviorRef::new(
                            ScrollBehaviorRef::new(NoScrollbarBehavior)
                                .copy_with()
                                .drag_devices(every_kind),
                        ))
                        .theme(theme)
                        .home(
                            CustomScrollView::new().slivers([SliverToBoxAdapter::new()
                                .child(SizedBox::new().height(2000.0))
                                .into_widget()]),
                        )
                        .into_widget(),
                );
                tester.pump_by(Duration::from_secs(5)); // to let the theme animate
            };
            let fling = |tester: &mut WidgetTester| {
                let scrollable = *tester
                    .find_by_type::<Scrollable>()
                    .first()
                    .expect("a scrollable");
                let center = tester
                    .global_rect(tester.render_box_of(scrollable))
                    .center();
                tester.fling_from(center, Offset::new(0.0, -DRAG_OFFSET), 1000.0, 1);
            };

            const DRAG_OFFSET: f64 = 200.0;

            pump_test(tester, TargetPlatform::Android);
            fling(tester);
            assert_eq!(get_scroll_offset(tester), DRAG_OFFSET);
            tester.pump(); // trigger fling
            assert_eq!(get_scroll_offset(tester), DRAG_OFFSET);
            tester.pump_by(Duration::from_secs(5));
            let android_result = get_scroll_offset(tester);

            reset_scroll_offset(tester);

            pump_test(tester, TargetPlatform::IOS);
            fling(tester);
            // Scroll starts ease into the scroll on iOS.
            assert!((get_scroll_offset(tester) - 197.16666666666669).abs() < 1e-10);
            tester.pump(); // trigger fling
            assert!((get_scroll_offset(tester) - 197.16666666666669).abs() < 1e-10);
            tester.pump_by(Duration::from_secs(5));
            let ios_result = get_scroll_offset(tester);

            reset_scroll_offset(tester);

            pump_test(tester, TargetPlatform::MacOS);
            fling(tester);
            // Scroll starts ease into the scroll on iOS.
            assert!((get_scroll_offset(tester) - 197.16666666666669).abs() < 1e-10);
            tester.pump(); // trigger fling
            assert!((get_scroll_offset(tester) - 197.16666666666669).abs() < 1e-10);
            tester.pump_by(Duration::from_secs(5));
            let mac_os_result = get_scroll_offset(tester);

            // Android is slipperier than MacOS, so the scroll offset should be greater.
            assert!(android_result > mac_os_result);
            // iOS is slipperier than Android, so the scroll offset should be greater.
            assert!(ios_result > android_result);
            // iOS is slipperier than MacOS, so the scroll offset should be greater.
            assert!(ios_result > mac_os_result);
        });
    }

    // "Scrollbar doesn't show when tapping list".
    #[test]
    fn scrollbar_does_not_show_when_tapping_list() {
        test_widgets(|tester| {
            let rows = (0..8).map(|index| {
                SizedBox::new()
                    .height(40.0)
                    .child(Text::new(index.to_string()))
                    .into_widget()
            });
            tester.pump_widget(build_boilerplate(
                Center::new().child(
                    Container::new()
                        .decoration(BoxDecoration::new().border(Border::all(
                            Color::new(0xFFFFFF00),
                            1.0,
                            BorderStyle::Solid,
                            BorderSide::STROKE_ALIGN_INSIDE,
                        )))
                        .height(200.0)
                        .width(300.0)
                        .child(Scrollbar::new(ListView::new().children(rows))),
                ),
            ));

            let transient_callback_count = |tester: &WidgetTester| {
                SchedulerBinding::transient_callback_count(&mut tester.cell.borrow_mut())
            };
            assert_eq!(
                transient_callback_count(tester),
                0,
                "Building a list with a scrollbar triggered an animation."
            );
            let list = *tester.find_by_type::<ListView>().first().expect("a list");
            let center = tester.global_rect(tester.render_box_of(list)).center();
            tester.tap(center, 1);
            assert_eq!(
                transient_callback_count(tester),
                0,
                "Tapping a block with a scrollbar triggered an animation."
            );
            for _ in 0..4 {
                tester.pump_by(Duration::from_millis(200));
            }
            tester.drag_from(center, Offset::new(0.0, -10.0), 1);
            assert!(transient_callback_count(tester) > 0);
            for _ in 0..4 {
                tester.pump_by(Duration::from_millis(200));
            }
        });
    }

    // 'ScrollbarPainter does not divide by zero'.
    #[test]
    fn scrollbar_painter_does_not_divide_by_zero() {
        test_widgets(|tester| {
            tester.pump_widget(build_boilerplate(
                SizedBox::new()
                    .height(200.0)
                    .width(300.0)
                    .child(Scrollbar::new(
                        ListView::new().children([SizedBox::new()
                            .height(40.0)
                            .child(Text::new("0"))
                            .into_widget()]),
                    )),
            ));

            let scroll_painter = {
                let state = material_state(tester);
                RawScrollbarState::scrollbar_painter(state, &tester.cell.borrow())
            };
            // Dragging makes the scrollbar first appear.
            let text = *tester.find_text("0").first().expect("the text");
            let center = tester.global_rect(tester.render_box_of(text)).center();
            tester.drag_from(center, Offset::new(0.0, -10.0), 1);
            tester.pump_by(Duration::from_millis(200));
            tester.pump_by(Duration::from_millis(200));

            let mut app = tester.cell.borrow_mut();
            let metrics = FixedScrollMetrics::new(
                Some(0.0),
                Some(0.0),
                Some(0.0),
                Some(100.0),
                AxisDirection::Down,
                2.0,
            );
            scroll_painter.update(&mut app, Rc::new(metrics), AxisDirection::Down);

            let mut canvas = Canvas::new();
            scroll_painter.paint(&mut app, &mut canvas, Size::new(10.0, 100.0));

            // Scrollbar is not supposed to draw anything if there isn't enough content.
            assert!(canvas.build().ops().is_empty());
        });
    }

    // 'When thumbVisibility is true, must pass a controller or find PrimaryScrollController'
    // (Dart runs the case twice).
    #[test]
    #[should_panic(
        expected = "A ScrollController is required when Scrollbar.thumbVisibility is true."
    )]
    fn when_thumb_visibility_is_true_must_pass_a_controller_or_find_primary_scroll_controller() {
        test_widgets(|tester| {
            let theme = ThemeData::new(&tester.cell.borrow());
            tester.pump_widget(build_boilerplate(Theme::new(
                theme,
                Scrollbar::new(scroll_view(None, 4000.0, 4000.0)).thumb_visibility(true),
            )));
        });
    }

    // 'When thumbVisibility is true, must pass a controller that is attached to a scroll view
    // or find PrimaryScrollController' (Dart runs the case twice).
    #[test]
    #[should_panic(expected = "The Scrollbar's ScrollController has no ScrollPosition attached.")]
    fn when_thumb_visibility_is_true_must_pass_a_controller_that_is_attached_to_a_scroll_view() {
        test_widgets(|tester| {
            let controller = controller(tester);
            let theme = ThemeData::new(&tester.cell.borrow());
            tester.pump_widget(build_boilerplate(Theme::new(
                theme,
                Scrollbar::new(scroll_view(None, 4000.0, 4000.0))
                    .thumb_visibility(true)
                    .controller(controller),
            )));
        });
    }

    // 'Scrollbar.thumbVisibility triggers assertion when multiple ScrollPositions are
    // attached.'
    #[test]
    fn thumb_visibility_panics_when_the_controller_has_two_positions() {
        fn get_tab_content(scroll_controller: Option<ScrollControllerRef>) -> WidgetRef {
            let list =
                ListView::builder(|_app, _context, _index| Some(Text::new("Test").into_widget()))
                    .item_count(200)
                    .build();
            let list = match scroll_controller.clone() {
                Some(controller) => list.controller(controller),
                None => list,
            };
            let scrollbar = Scrollbar::new(list).thumb_visibility(true);
            match scroll_controller {
                Some(controller) => scrollbar.controller(controller),
                None => scrollbar,
            }
            .into_widget()
        }

        fn build_app(
            tester: &WidgetTester,
            scroll_controller: Option<ScrollControllerRef>,
        ) -> WidgetRef {
            let theme = ThemeData::new(&tester.cell.borrow());
            material_app(
                theme,
                crate::tab_controller::DefaultTabController::new(
                    2,
                    Scaffold::new().body(crate::tabs::TabBarView::new([
                        get_tab_content(scroll_controller.clone()),
                        get_tab_content(scroll_controller),
                    ])),
                )
                .into_widget(),
            )
        }

        /// The panic message of swiping to the second tab, resulting in two attached
        /// ScrollPositions during the transition: Dart's `tester.drag` and `pump`, then
        /// `tester.takeException()`.
        fn swipe_and_take_exception(tester: &mut WidgetTester) -> String {
            let first = tester.find_text("Test")[0];
            let center = {
                let render = tester.render_box_of(first);
                let app = tester.cell.borrow();
                let size = render.size(&app);
                render.local_to_global(&app, size.center(Offset::ZERO), None)
            };
            let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                tester.drag_from(center, Offset::new(-100.0, 0.0), 1);
                tester.pump();
            }))
            .expect_err("two attached positions panic");
            panicked
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| panicked.downcast_ref::<&str>().map(|text| text.to_string()))
                .expect("a panic message")
        }

        test_widgets(|tester| {
            // Asserts when using the PrimaryScrollController.
            let app = build_app(tester, None);
            tester.pump_widget(app);
            assert_eq!(
                swipe_and_take_exception(tester),
                "The PrimaryScrollController is attached to more than one ScrollPosition.\n\
                 The Scrollbar requires a single ScrollPosition in order to be painted.\n\
                 When Scrollbar.thumbVisibility is true, the associated ScrollController must \
                 only have one ScrollPosition attached.\n\
                 If a ScrollController has not been provided, the PrimaryScrollController is \
                 used by default on mobile platforms for ScrollViews with an Axis.vertical \
                 scroll direction.\n\
                 More than one ScrollView may have tried to use the PrimaryScrollController of \
                 the current context. ScrollView.primary can override this behavior."
            );
        });
        test_widgets(|tester| {
            // Asserts when using the ScrollController provided by the user; Dart's second app
            // has its own key, so a fresh tester stands for it.
            let scroll_controller = controller(tester);
            let app = build_app(tester, Some(scroll_controller.clone()));
            tester.pump_widget(app);
            assert_eq!(
                swipe_and_take_exception(tester),
                "The provided ScrollController is attached to more than one ScrollPosition.\n\
                 The Scrollbar requires a single ScrollPosition in order to be painted.\n\
                 When Scrollbar.thumbVisibility is true, the associated ScrollController must \
                 only have one ScrollPosition attached.\n\
                 The provided ScrollController cannot be shared by multiple ScrollView widgets."
            );
        });
    }

    /// The toggle cases' tree: a `Scaffold` whose `FloatingActionButton` flips the
    /// scrollbar's `thumbVisibility`, which starts as `thumb_visibility`.
    fn toggled_view(controller: ScrollControllerRef, thumb_visibility: bool) -> WidgetRef {
        let thumb_visibility = Rc::new(Cell::new(thumb_visibility));
        build_boilerplate(StatefulBuilder::new(move |app, _context, set_state| {
            let toggled = Rc::clone(&thumb_visibility);
            Theme::new(
                ThemeData::new(app),
                Scaffold::new()
                    .floating_action_button(
                        FloatingActionButton::new(Some(Listener::new(move |app: &mut App| {
                            let toggled = Rc::clone(&toggled);
                            set_state(app, &mut move |_app| toggled.set(!toggled.get()));
                        })))
                        .child(Icon::new(Some(Icons::threed_rotation()))),
                    )
                    .body(
                        Scrollbar::new(scroll_view(Some(controller.clone()), 4000.0, 4000.0))
                            .thumb_visibility(thumb_visibility.get())
                            .controller(controller.clone()),
                    ),
            )
            .into_widget()
        }))
    }

    /// Dart's `tester.fling(find.byType(SingleChildScrollView), Offset(0.0, -10.0), 10)`.
    fn fling_scroll_view(tester: &mut WidgetTester) {
        let view = *tester
            .find_by_type::<SingleChildScrollView>()
            .first()
            .expect("the scroll view");
        let center = tester.global_rect(tester.render_box_of(view)).center();
        tester.fling_from(center, Offset::new(0.0, -10.0), 10.0, 1);
    }

    /// Dart's `tester.tap(find.byType(FloatingActionButton))`.
    fn tap_floating_action_button(tester: &mut WidgetTester) {
        let button = *tester
            .find_by_type::<FloatingActionButton>()
            .first()
            .expect("the button");
        let center = tester.global_rect(tester.render_box_of(button)).center();
        tester.tap(center, 2);
    }

    // 'With thumbVisibility: true, fling a scroll. While it is still scrolling, set
    // thumbVisibility: false. The thumb should not fade out until the scrolling stops.'
    #[test]
    fn with_thumb_visibility_true_the_thumb_does_not_fade_out_until_a_fling_stops() {
        test_widgets(|tester| {
            let controller = controller(tester);
            tester.pump_widget(toggled_view(controller.clone(), true));
            tester.pump_and_settle();
            fling_scroll_view(tester);
            assert!(paints_thumb(tester));

            tap_floating_action_button(tester);
            tester.pump_and_settle();
            // Scrollbar is not showing after scroll finishes
            assert!(!paints_thumb(tester));
        });
    }

    // 'With thumbVisibility: false, set thumbVisibility: true. The thumb should be always
    // shown directly'.
    #[test]
    fn with_thumb_visibility_false_setting_it_true_shows_the_thumb_directly() {
        test_widgets(|tester| {
            let controller = controller(tester);
            tester.pump_widget(toggled_view(controller.clone(), false));
            tester.pump_and_settle();
            assert!(!paints_thumb(tester));

            tap_floating_action_button(tester);
            tester.pump_and_settle();
            // Scrollbar is not showing after scroll finishes
            assert!(paints_thumb(tester));
        });
    }

    // 'With thumbVisibility: false, fling a scroll. While it is still scrolling, set
    // thumbVisibility: true. The thumb should not fade even after the scrolling stops'.
    #[test]
    fn with_thumb_visibility_false_setting_it_true_during_a_fling_keeps_the_thumb() {
        test_widgets(|tester| {
            let controller = controller(tester);
            tester.pump_widget(toggled_view(controller.clone(), false));
            tester.pump_and_settle();
            assert!(!paints_thumb(tester));
            fling_scroll_view(tester);
            assert!(paints_thumb(tester));

            tap_floating_action_button(tester);
            tester.pump();
            assert!(paints_thumb(tester));

            // Wait for the timer delay to expire.
            tester.pump_by(Duration::from_millis(600)); // _kScrollbarTimeToFade
            tester.pump_and_settle();
            // Scrollbar thumb is showing after scroll finishes and timer ends.
            assert!(paints_thumb(tester));
        });
    }

    /// What the scrollbar under the `Scrollbar` keyed `key` paints over its child, whether
    /// it adapted to a `CupertinoScrollbar` or not.
    fn painted_under(tester: &WidgetTester, key: &GlobalKey) -> Vec<Drawn> {
        let keyed = key
            .current_context(&tester.cell.borrow_mut())
            .expect("the keyed scrollbar is mounted");
        let raw = tester
            .elements()
            .into_iter()
            .find(|element| {
                let app = tester.cell.borrow();
                let widget = element.widget(&app);
                let is_raw = waft_widgets::downcast_widget::<MaterialScrollbar>(&**widget)
                    .is_some()
                    || waft_widgets::downcast_widget::<CupertinoScrollbar>(&**widget).is_some();
                drop(app);
                is_raw && tester.ancestor_of_type::<Scrollbar>(*element) == keyed
            })
            .expect("the keyed scrollbar's raw scrollbar");
        let app = tester.cell.borrow();
        if let Some(state) = raw.state_handle::<MaterialScrollbarState>(&app) {
            drop(app);
            return painted_by(tester, state);
        }
        let state = raw
            .state_handle::<CupertinoScrollbarState>(&app)
            .expect("a Cupertino scrollbar's state");
        drop(app);
        painted_by(tester, state)
    }

    // "Scrollbar doesn't show when scroll the inner scrollable widget".
    #[test]
    fn scrollbar_does_not_show_when_scroll_the_inner_scrollable_widget() {
        for platform in ALL_PLATFORMS {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let key1 = GlobalKey::new();
                let key2 = GlobalKey::new();
                let outer_key = GlobalKey::new();
                let inner_key = GlobalKey::new();

                let theme = ThemeData::light(&tester.cell.borrow(), None);
                let theme_platform = theme.platform;

                tester.pump_widget(
                    Theme::new(
                        theme,
                        Directionality::new(
                            waft_embedder::TextDirection::Ltr,
                            MediaQuery::new(
                                MediaQueryData::new(),
                                no_scrollbar(
                                    Scrollbar::new(
                                        SingleChildScrollView::new()
                                            .key(outer_key.clone().into())
                                            .child(
                                                SizedBox::new()
                                                    .height(1000.0)
                                                    .width(f64::INFINITY)
                                                    .child(Column::new().children([
                                                        Scrollbar::new(
                                                            SizedBox::new()
                                                                .height(300.0)
                                                                .width(f64::INFINITY)
                                                                .child(
                                                                    SingleChildScrollView::new()
                                                                        .key(inner_key.clone().into())
                                                                        .child(
                                                                            SizedBox::new()
                                                                                .key(
                                                                                    waft_foundation::ValueKey::new(
                                                                                        "Inner scrollable".to_owned(),
                                                                                    )
                                                                                    .into(),
                                                                                )
                                                                                .height(1000.0)
                                                                                .width(f64::INFINITY),
                                                                        ),
                                                                ),
                                                        )
                                                        .key(key1.clone().into())
                                                        .into_widget(),
                                                    ])),
                                            ),
                                    )
                                    .key(key2.clone().into()),
                                ),
                            )
                            .into_widget(),
                        ),
                    )
                    .into_widget(),
                );

                // Drag the inner scrollable widget.
                let inner = tester.render_object(&inner_key).as_box().expect("a box");
                let center = tester.global_rect(inner).center();
                tester.drag_from(center, Offset::new(0.0, -25.0), 1);
                tester.pump();
                // Scrollbar fully showing.
                tester.pump_by(Duration::from_millis(500));

                // Valo records no op for the transparent track and its border, which Dart's
                // `paintsExactlyCountTimes` counts: what is left is the thumb, and the outer
                // scrollbar draws nothing (Dart's counts on `key2` take in `key1`'s draws).
                assert_eq!(painted_under(tester, &key2), []);
                let inner_drawn = painted_under(tester, &key1);
                match theme_platform {
                    TargetPlatform::Android => {
                        // On android, draws two rectangles for the track and thumb.
                        assert!(
                            matches!(inner_drawn.as_slice(), [Drawn::Rect(..)]),
                            "{inner_drawn:?}"
                        );
                    }
                    TargetPlatform::IOS
                    | TargetPlatform::Fuchsia
                    | TargetPlatform::Linux
                    | TargetPlatform::MacOS
                    | TargetPlatform::Windows => {
                        // Draws a rectangle for the track and a rounded rectangle for the thumb.
                        assert!(
                            matches!(inner_drawn.as_slice(), [Drawn::Path(..)]),
                            "{inner_drawn:?}"
                        );
                    }
                }
            });
        }
    }

    // 'Simultaneous dragging and pointer scrolling does not cause a crash'.
    #[test]
    fn simultaneous_dragging_and_pointer_scrolling_does_not_cause_a_crash() {
        test_widgets(|tester| {
            // Regression test for https://github.com/flutter/flutter/issues/70105
            let scroll_controller = controller(tester);
            let theme = m2(tester);
            tester.pump_widget(material_app(
                theme,
                PrimaryScrollController::new(
                    scroll_controller.clone(),
                    Scrollbar::new(scroll_view(None, 4000.0, 4000.0))
                        .interactive(true)
                        .thumb_visibility(true)
                        .controller(scroll_controller.clone()),
                )
                .into_widget(),
            ));
            tester.pump_and_settle();
            assert_eq!(offset(tester, scroll_controller.handle()), 0.0);
            let border = (Offset::new(796.0, 0.0), Offset::new(796.0, 600.0));
            let assert_painted = |tester: &WidgetTester, thumb: Rect, color: u32| {
                let drawn = painted(tester);
                assert_track(
                    tester,
                    &drawn,
                    K_ANDROID_TRACK_DIMENSIONS,
                    K_TRANSPARENT,
                    border,
                    K_TRANSPARENT,
                );
                assert_thumb_rect(&drawn, thumb, color);
            };
            assert_painted(
                tester,
                get_starting_thumb_rect(true),
                K_ANDROID_THUMB_IDLE_COLOR,
            );

            // Drag the thumb down to scroll down.
            const SCROLL_AMOUNT: f64 = 10.0;
            let mut drag_scrollbar_gesture = start_gesture(tester, Offset::new(797.0, 45.0));
            tester.pump_and_settle();

            // Drag color
            assert_painted(tester, get_starting_thumb_rect(true), 0x99000000);

            move_by(
                tester,
                &mut drag_scrollbar_gesture,
                Offset::new(0.0, SCROLL_AMOUNT),
            );
            tester.pump_and_settle();
            assert!(offset(tester, scroll_controller.handle()) > 10.0);
            let previous_offset = offset(tester, scroll_controller.handle());
            assert_painted(
                tester,
                Rect::from_ltrb(796.0, 10.0, 800.0, 100.0),
                0x99000000,
            );

            // Execute a pointer scroll while dragging (drag gesture has not come up yet)
            let pointer_scroll = |tester: &mut WidgetTester, at: Offset, delta: Offset| {
                let mut app = tester.cell.borrow_mut();
                GestureBinding::instance(&mut app).handle_pointer_event(
                    &mut app,
                    PointerEvent::Scroll(PointerScrollEvent {
                        kind: PointerDeviceKind::Mouse,
                        device: 2,
                        position: at,
                        scroll_delta: delta,
                        ..PointerScrollEvent::default()
                    }),
                );
                app.drain_microtasks();
            };
            pointer_scroll(tester, Offset::new(798.0, 15.0), Offset::new(0.0, 20.0));
            tester.pump_and_settle();

            // Scrolling while holding the drag on the scrollbar and still hovered over
            // the scrollbar should not have changed the scroll offset.
            assert_eq!(offset(tester, scroll_controller.handle()), previous_offset);
            assert_painted(
                tester,
                Rect::from_ltrb(796.0, 10.0, 800.0, 100.0),
                0x99000000,
            );

            // Drag is still being held, move pointer to be hovering over another area
            // of the scrollable (not over the scrollbar) and execute another pointer scroll
            let view = *tester
                .find_by_type::<SingleChildScrollView>()
                .first()
                .expect("the scroll view");
            let center = tester.global_rect(tester.render_box_of(view)).center();
            assert_eq!(center, Offset::new(400.0, 300.0));
            pointer_scroll(tester, center, Offset::new(0.0, -90.0));
            tester.pump_and_settle();
            // Scrolling while holding the drag on the scrollbar changed the offset
            assert_eq!(offset(tester, scroll_controller.handle()), 0.0);
            assert_painted(tester, Rect::from_ltrb(796.0, 0.0, 800.0, 90.0), 0x99000000);

            tester.up(&drag_scrollbar_gesture);
            tester.pump_and_settle();
            assert_eq!(offset(tester, scroll_controller.handle()), 0.0);
            assert_painted(tester, Rect::from_ltrb(796.0, 0.0, 800.0, 90.0), 0xffbcbcbc);
        });
    }

    /// Dart's collector frees the hover controller `_MaterialScrollbarState` makes once the
    /// state is gone; here it is freed with the state's reference to it.
    #[test]
    fn a_disposed_scrollbar_frees_its_hover_controller() {
        test_widgets(|tester| {
            let controller = controller(tester);
            let theme = ThemeData::new(&tester.cell.borrow());
            tester.pump_widget(build_boilerplate(Theme::new(
                theme,
                Scrollbar::new(scroll_view(Some(controller.clone()), 4000.0, 4000.0))
                    .controller(controller),
            )));
            let hover = {
                let scrollbar = tester.find_by_type::<MaterialScrollbar>()[0];
                let app = tester.cell.borrow();
                scrollbar
                    .state_handle::<MaterialScrollbarState>(&app)
                    .expect("a MaterialScrollbar is stateful")
                    .hover_animation_controller(&app)
            };

            tester.pump_widget(SizedBox::new().into_widget());
            tester.cell.checkpoint();
            assert!(!tester.cell.borrow().contains(hover));
        });
    }
}
