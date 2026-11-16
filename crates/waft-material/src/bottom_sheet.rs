//! Flutter counterpart: material_ui's `bottom_sheet.dart`.

use std::any::TypeId;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animatable, Animation, AnimationBehavior, AnimationController, AnimationRef, AnimationStatus,
    AnimationStyle, AnyAnimation, ColorTween, Curve, CurveTween, CurvedAnimation, Curves, Drive,
    ProxyAnimation, Split, k_always_dismissed_animation,
};
use waft_embedder::{
    Clip, Color, Offset, SemanticsHitTestBehavior, Size, TargetPlatform, TextBaseline,
};
use waft_foundation::{
    App, CompleterFuture, Handle, Listenable, OptionRetained, PRECISION_ERROR_TOLERANCE, Ref,
    ValueChanged, ValueNotifier, VoidCallback,
};
use waft_gestures::{
    DragEndDetails, DragGestureRecognizer, DragStartDetails, DragUpdateDetails,
    GestureDragEndCallback, GestureDragStartCallback, GestureDragUpdateCallback, PointerEnterEvent,
    PointerExitEvent, VerticalDragGestureRecognizer,
};
use waft_painting::{
    AlignmentGeometry, AnyColor, BorderRadius, BoxDecoration, EdgeInsets, EdgeInsetsGeometry,
    ShapeBorderRef,
};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, BoxParentData, Constraints,
    PaintingContext, RenderBox, RenderBoxData, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin, RenderShiftedBox,
};
use waft_widgets::{
    Align, AnimatedModalBarrier, BuildContext, Builder, CapturedThemes, Center, ClipRect,
    ConstrainedBox, Container, DraggableScrollableNotification, GestureRecognizerFactory,
    GestureRecognizerFactoryWithHandlers, GlobalKey, InheritedTheme, IntoWidget, KeyRef,
    ListenableBuilder, LocalHistoryRoute, LocalHistoryRouteData, MediaQuery, ModalBarrier,
    ModalRoute, ModalRouteData, MouseRegion, Navigator, NotificationListener, OverlayRoute,
    OverlayRouteData, Padding, PopupRoute, PredictiveBackRoute, RawGestureDetector,
    RenderObjectWidget, Route, RouteData, RouteResult, RouteSettingsRef, SafeArea, Semantics,
    SingleChildRenderObjectWidget, SizedBox, Stack, State, StateData, StatefulWidget,
    StatelessWidget, TransitionRoute, TransitionRouteData, WidgetBuilder, WidgetRef, WidgetState,
    WidgetStateProperty, WidgetStates, debug_check_has_media_query,
};

use crate::bottom_sheet_theme::{BottomSheetThemeData, BottomSheetThemeDataGetters};
use crate::colors::Colors;
use crate::constants::K_MIN_INTERACTIVE_DIMENSION;
use crate::debug::{debug_check_has_material_localizations, debug_check_has_scaffold};
use crate::generated::bottom_sheet_defaults_m3::BottomSheetDefaultsM3;
use crate::material::Material;
use crate::material_localizations::MaterialLocalizations;
use crate::motion::Easing;
use crate::scaffold::{PersistentBottomSheetController, Scaffold};
use crate::theme::Theme;

const K_BOTTOM_SHEET_ENTER_DURATION: Duration = Duration::from_millis(250);
const K_BOTTOM_SHEET_EXIT_DURATION: Duration = Duration::from_millis(200);
const K_MIN_FLING_VELOCITY: f64 = 700.0;
const K_CLOSE_PROGRESS_THRESHOLD: f64 = 0.5;
const K_DEFAULT_SCROLL_CONTROL_DISABLED_MAX_HEIGHT_RATIO: f64 = 9.0 / 16.0;

/// Dart's `_kModalBottomSheetCurve`.
fn k_modal_bottom_sheet_curve() -> Rc<dyn Curve> {
    Easing::legacy_decelerate()
}

/// A callback for when the user begins dragging the bottom sheet.
///
/// Used by [`BottomSheet::on_drag_start`].
pub type BottomSheetDragStartHandler = Rc<dyn Fn(&mut App, DragStartDetails)>;

/// A callback for when the user stops dragging the bottom sheet.
///
/// The `bool` is Dart's named `isClosing`.
///
/// Used by [`BottomSheet::on_drag_end`].
pub type BottomSheetDragEndHandler = Rc<dyn Fn(&mut App, DragEndDetails, bool)>;

/// A Material Design bottom sheet.
///
/// There are two kinds of bottom sheets in Material Design:
///
///  * _Persistent_. A persistent bottom sheet shows information that
///    supplements the primary content of the app. A persistent bottom sheet
///    remains visible even when the user interacts with other parts of the app.
///    Persistent bottom sheets can be created and displayed with the
///    `ScaffoldState::show_bottom_sheet` function or by specifying the
///    `Scaffold::bottom_sheet` constructor parameter.
///
///  * _Modal_. A modal bottom sheet is an alternative to a menu or a dialog and
///    prevents the user from interacting with the rest of the app. Modal bottom
///    sheets can be created and displayed with the [`show_modal_bottom_sheet`]
///    function.
///
/// The [`BottomSheet`] widget itself is rarely used directly. Instead, prefer to
/// create a persistent bottom sheet with `ScaffoldState::show_bottom_sheet` or
/// `Scaffold::bottom_sheet`, and a modal bottom sheet with [`show_modal_bottom_sheet`].
///
/// See also:
///
///  * [`show_bottom_sheet`] and `ScaffoldState::show_bottom_sheet`, for showing
///    non-modal "persistent" bottom sheets.
///  * [`show_modal_bottom_sheet`], which can be used to display a modal bottom
///    sheet.
///  * [`BottomSheetThemeData`], which can be used to customize the default
///    bottom sheet property values.
///  * The Material 2 spec at <https://m2.material.io/components/sheets-bottom>.
///  * The Material 3 spec at <https://m3.material.io/components/bottom-sheets/overview>.
pub struct BottomSheet {
    pub key: Option<KeyRef>,

    /// The animation controller that controls the bottom sheet's entrance and
    /// exit animations.
    ///
    /// The BottomSheet widget will manipulate the position of this animation, it
    /// is not just a passive observer.
    pub animation_controller: Option<Ref<AnimationController>>,

    /// Called when the bottom sheet begins to close.
    ///
    /// A bottom sheet might be prevented from closing (e.g., by user
    /// interaction) even after this callback is called. For this reason, this
    /// callback might be call multiple times for a given bottom sheet.
    pub on_closing: VoidCallback,

    /// A builder for the contents of the sheet.
    ///
    /// The bottom sheet will wrap the widget produced by this builder in a
    /// [`Material`] widget.
    pub builder: WidgetBuilder,

    /// If true, the bottom sheet can be dragged up and down and dismissed by
    /// swiping downwards.
    ///
    /// If [`show_drag_handle`](Self::show_drag_handle) is true, this only applies to the
    /// content below the drag handle, because the drag handle is always draggable.
    ///
    /// Default is true.
    ///
    /// If this is true, the [`animation_controller`](Self::animation_controller) must not be
    /// null. Use [`BottomSheet::create_animation_controller`] to create one, or provide
    /// another AnimationController.
    pub enable_drag: bool,

    /// Specifies whether a drag handle is shown.
    ///
    /// The drag handle appears at the top of the bottom sheet. The default color is
    /// `ColorScheme.onSurfaceVariant` with an opacity of 0.4 and can be customized
    /// using [`drag_handle_color`](Self::drag_handle_color). The default size is
    /// `Size(32,4)` and can be customized with [`drag_handle_size`](Self::drag_handle_size).
    ///
    /// If null, then the value of [`BottomSheetThemeData::show_drag_handle`] is used. If
    /// that is also null, defaults to false.
    ///
    /// If this is true, the [`animation_controller`](Self::animation_controller) must not be
    /// null. Use [`BottomSheet::create_animation_controller`] to create one, or provide
    /// another AnimationController.
    pub show_drag_handle: Option<bool>,

    /// The bottom sheet drag handle's color.
    ///
    /// Defaults to [`BottomSheetThemeData::drag_handle_color`].
    /// If that is also null, defaults to `ColorScheme.onSurfaceVariant`.
    pub drag_handle_color: Option<AnyColor>,

    /// Defaults to [`BottomSheetThemeData::drag_handle_size`].
    /// If that is also null, defaults to Size(32, 4).
    pub drag_handle_size: Option<Size>,

    /// Called when the user begins dragging the bottom sheet vertically, if
    /// [`enable_drag`](Self::enable_drag) is true.
    ///
    /// Would typically be used to change the bottom sheet animation curve so
    /// that it tracks the user's finger accurately.
    pub on_drag_start: Option<BottomSheetDragStartHandler>,

    /// Called when the user stops dragging the bottom sheet, if
    /// [`enable_drag`](Self::enable_drag) is true.
    ///
    /// Would typically be used to reset the bottom sheet animation curve, so
    /// that it animates non-linearly. Called before [`on_closing`](Self::on_closing) if the
    /// bottom sheet is closing.
    pub on_drag_end: Option<BottomSheetDragEndHandler>,

    /// The bottom sheet's background color.
    ///
    /// Defines the bottom sheet's `Material.color`.
    ///
    /// Defaults to null and falls back to [`Material`]'s default.
    pub background_color: Option<Color>,

    /// The color of the shadow below the sheet.
    ///
    /// If this property is null, then [`BottomSheetThemeData::shadow_color`] of
    /// `ThemeData.bottomSheetTheme` is used. If that is also null, the default value
    /// is transparent.
    ///
    /// See also:
    ///
    ///  * [`elevation`](Self::elevation), which defines the size of the shadow below the
    ///    sheet.
    ///  * [`shape`](Self::shape), which defines the shape of the sheet and its shadow.
    pub shadow_color: Option<Color>,

    /// The z-coordinate at which to place this material relative to its parent.
    ///
    /// This controls the size of the shadow below the material.
    ///
    /// Defaults to 0. The value is non-negative.
    pub elevation: Option<f64>,

    /// The shape of the bottom sheet.
    ///
    /// Defines the bottom sheet's `Material.shape`.
    ///
    /// Defaults to null and falls back to [`Material`]'s default.
    pub shape: Option<ShapeBorderRef>,

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defines the bottom sheet's `Material.clipBehavior`.
    ///
    /// Use this property to enable clipping of content when the bottom sheet has
    /// a custom [`shape`](Self::shape) and the content can extend past this shape. For
    /// example, a bottom sheet with rounded corners and an edge-to-edge `Image` at the
    /// top.
    ///
    /// If this property is null then [`BottomSheetThemeData::clip_behavior`] of
    /// `ThemeData.bottomSheetTheme` is used. If that's null then the behavior
    /// will be [`Clip::None`].
    pub clip_behavior: Option<Clip>,

    /// Defines minimum and maximum sizes for a [`BottomSheet`].
    ///
    /// If null, then the ambient `ThemeData.bottomSheetTheme`'s
    /// [`BottomSheetThemeData::constraints`] will be used. If that
    /// is null and `ThemeData.useMaterial3` is true, then the bottom sheet
    /// will have a max width of 640dp. If `ThemeData.useMaterial3` is false, then
    /// the bottom sheet's size will be constrained by its parent
    /// (usually a `Scaffold`). In this case, consider limiting the width by
    /// setting smaller constraints for large screens.
    ///
    /// If constraints are specified (either in this property or in the
    /// theme), the bottom sheet will be aligned to the bottom-center of
    /// the available space. Otherwise, no alignment is applied.
    pub constraints: Option<BoxConstraints>,
}

impl BottomSheet {
    /// Creates a bottom sheet.
    ///
    /// Typically, bottom sheets are created implicitly by
    /// `ScaffoldState::show_bottom_sheet`, for persistent bottom sheets, or by
    /// [`show_modal_bottom_sheet`], for modal bottom sheets.
    pub fn new(on_closing: VoidCallback, builder: WidgetBuilder) -> BottomSheet {
        BottomSheet {
            key: None,
            animation_controller: None,
            on_closing,
            builder,
            enable_drag: true,
            show_drag_handle: None,
            drag_handle_color: None,
            drag_handle_size: None,
            on_drag_start: None,
            on_drag_end: None,
            background_color: None,
            shadow_color: None,
            elevation: None,
            shape: None,
            clip_behavior: None,
            constraints: None,
        }
    }

    /// Dart `BottomSheet(key:)`.
    pub fn key(mut self, key: KeyRef) -> BottomSheet {
        self.key = Some(key);
        self
    }

    /// Dart `BottomSheet(animationController:)`.
    pub fn animation_controller(
        mut self,
        animation_controller: Ref<AnimationController>,
    ) -> BottomSheet {
        self.animation_controller = Some(animation_controller);
        self
    }

    /// Dart `BottomSheet(enableDrag:)`.
    pub fn enable_drag(mut self, enable_drag: bool) -> BottomSheet {
        self.enable_drag = enable_drag;
        self
    }

    /// Dart `BottomSheet(showDragHandle:)`.
    pub fn show_drag_handle(mut self, show_drag_handle: bool) -> BottomSheet {
        self.show_drag_handle = Some(show_drag_handle);
        self
    }

    /// Dart `BottomSheet(dragHandleColor:)`.
    pub fn drag_handle_color(mut self, drag_handle_color: impl Into<AnyColor>) -> BottomSheet {
        self.drag_handle_color = Some(drag_handle_color.into());
        self
    }

    /// Dart `BottomSheet(dragHandleSize:)`.
    pub fn drag_handle_size(mut self, drag_handle_size: Size) -> BottomSheet {
        self.drag_handle_size = Some(drag_handle_size);
        self
    }

    /// Dart `BottomSheet(onDragStart:)`.
    pub fn on_drag_start(
        mut self,
        on_drag_start: impl Fn(&mut App, DragStartDetails) + 'static,
    ) -> BottomSheet {
        self.on_drag_start = Some(Rc::new(on_drag_start));
        self
    }

    /// Dart `BottomSheet(onDragEnd:)`.
    pub fn on_drag_end(
        mut self,
        on_drag_end: impl Fn(&mut App, DragEndDetails, bool) + 'static,
    ) -> BottomSheet {
        self.on_drag_end = Some(Rc::new(on_drag_end));
        self
    }

    /// Dart `BottomSheet(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> BottomSheet {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `BottomSheet(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> BottomSheet {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `BottomSheet(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> BottomSheet {
        debug_assert!(elevation >= 0.0);
        self.elevation = Some(elevation);
        self
    }

    /// Dart `BottomSheet(shape:)`.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> BottomSheet {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `BottomSheet(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> BottomSheet {
        self.clip_behavior = Some(clip_behavior);
        self
    }

    /// Dart `BottomSheet(constraints:)`.
    pub fn constraints(mut self, constraints: BoxConstraints) -> BottomSheet {
        self.constraints = Some(constraints);
        self
    }

    /// Creates an [`AnimationController`] suitable for a
    /// [`BottomSheet::animation_controller`].
    ///
    /// This API is available as a convenience for a Material compliant bottom sheet
    /// animation. If alternative animation durations are required, a different
    /// animation controller could be provided.
    pub fn create_animation_controller(
        app: &mut App,
        vsync: impl waft_scheduler::TickerProvider,
        sheet_animation_style: Option<&AnimationStyle>,
    ) -> Ref<AnimationController> {
        AnimationController::new(
            app,
            None,
            Some(
                sheet_animation_style
                    .and_then(|style| style.duration)
                    .unwrap_or(K_BOTTOM_SHEET_ENTER_DURATION),
            ),
            Some(
                sheet_animation_style
                    .and_then(|style| style.reverse_duration)
                    .unwrap_or(K_BOTTOM_SHEET_EXIT_DURATION),
            ),
            0.0,
            1.0,
            AnimationBehavior::Normal,
            vsync,
        )
    }
}

impl Debug for BottomSheet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BottomSheet")
            .field("key", &self.key)
            .field("animationController", &self.animation_controller)
            .field("enableDrag", &self.enable_drag)
            .field("showDragHandle", &self.show_drag_handle)
            .field("dragHandleColor", &self.drag_handle_color)
            .field("dragHandleSize", &self.drag_handle_size)
            .field("backgroundColor", &self.background_color)
            .field("shadowColor", &self.shadow_color)
            .field("elevation", &self.elevation)
            .field("shape", &self.shape)
            .field("clipBehavior", &self.clip_behavior)
            .field("constraints", &self.constraints)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for BottomSheet {
    type State = BottomSheetState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> BottomSheetState {
        BottomSheetState {
            state: StateData::new(),
            child_key: GlobalKey::new(),
            drag_handle_states: WidgetStates::new(),
        }
    }
}

/// Dart's `_BottomSheetState`.
pub struct BottomSheetState {
    state: StateData<BottomSheet>,
    /// Dart's `GlobalKey(debugLabel: 'BottomSheet child')`.
    child_key: GlobalKey,
    drag_handle_states: WidgetStates,
}

impl BottomSheetState {
    fn child_height(self: Handle<Self>, app: &mut App) -> f64 {
        let child_key = app.get(self).child_key.clone();
        let render_box = child_key
            .current_context(app)
            .expect("the sheet's Material is mounted")
            .find_render_object(app)
            .and_then(|render_object| render_object.as_box())
            .expect("a Material renders a box");
        render_box.size(app).height()
    }

    fn dismiss_underway(self: Handle<Self>, app: &App) -> bool {
        self.animation_controller(app).status(app) == AnimationStatus::Reverse
    }

    /// Dart's `widget.animationController!`.
    fn animation_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        self.widget(app)
            .animation_controller
            .handle()
            .expect("the sheet has an animation controller")
    }

    fn handle_drag_start(self: Handle<Self>, app: &mut App, details: DragStartDetails) {
        self.set_state(app, |state| {
            state.drag_handle_states.insert(WidgetState::Dragged);
        });
        if let Some(on_drag_start) = self.widget(app).on_drag_start.clone() {
            on_drag_start(app, details);
        }
    }

    fn debug_assert_has_controller(self: Handle<Self>, app: &App) {
        let widget = self.widget(app);
        debug_assert!(
            (widget.enable_drag || widget.show_drag_handle.unwrap_or(false))
                && widget.animation_controller.is_some(),
            "'BottomSheet.animationController' cannot be null when 'BottomSheet.enableDrag' or \
             'BottomSheet.showDragHandle' is true. Use 'BottomSheet.createAnimationController' to \
             create one, or provide another AnimationController."
        );
    }

    fn handle_drag_update(self: Handle<Self>, app: &mut App, details: DragUpdateDetails) {
        self.debug_assert_has_controller(app);
        if self.dismiss_underway(app) {
            return;
        }
        let controller = self.animation_controller(app);
        let child_height = self.child_height(app);
        let primary_delta = details
            .primary_delta
            .expect("a vertical drag has a primary delta");
        let value = controller.value(app) - primary_delta / child_height;
        controller.set_value(app, value);
    }

    fn handle_drag_end(self: Handle<Self>, app: &mut App, details: DragEndDetails) {
        self.debug_assert_has_controller(app);
        if self.dismiss_underway(app) {
            return;
        }
        self.set_state(app, |state| {
            state.drag_handle_states.remove(&WidgetState::Dragged);
        });
        let controller = self.animation_controller(app);
        let mut is_closing = false;
        if details.velocity.pixels_per_second.dy() > K_MIN_FLING_VELOCITY {
            let fling_velocity = -details.velocity.pixels_per_second.dy() / self.child_height(app);
            if controller.value(app) > 0.0 {
                controller.fling(app, fling_velocity, None, None);
            }
            if fling_velocity < 0.0 {
                is_closing = true;
            }
        } else if controller.value(app) < K_CLOSE_PROGRESS_THRESHOLD {
            if controller.value(app) > 0.0 {
                controller.fling(app, -1.0, None, None);
            }
            is_closing = true;
        } else {
            controller.forward(app, None);
        }

        if let Some(on_drag_end) = self.widget(app).on_drag_end.clone() {
            on_drag_end(app, details, is_closing);
        }

        if is_closing {
            let on_closing = self.widget(app).on_closing.clone();
            on_closing(app);
        }
    }

    /// Dart's `extentChanged`.
    pub fn extent_changed(
        self: Handle<Self>,
        app: &mut App,
        notification: &DraggableScrollableNotification,
    ) -> bool {
        if (notification.extent - notification.min_extent).abs() < PRECISION_ERROR_TOLERANCE
            && notification.should_close_on_min_extent
        {
            let on_closing = self.widget(app).on_closing.clone();
            on_closing(app);
        }
        false
    }

    fn handle_drag_handle_hover(self: Handle<Self>, app: &mut App, hovering: bool) {
        if hovering
            != app
                .get(self)
                .drag_handle_states
                .contains(&WidgetState::Hovered)
        {
            self.set_state(app, |state| {
                if hovering {
                    state.drag_handle_states.insert(WidgetState::Hovered);
                } else {
                    state.drag_handle_states.remove(&WidgetState::Hovered);
                }
            });
        }
    }
}

impl State for BottomSheetState {
    type Widget = BottomSheet;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = Theme::of(app, context);
        let bottom_sheet_theme = theme.bottom_sheet_theme.clone();
        let use_material3 = theme.use_material3;
        let defaults: Box<dyn BottomSheetThemeDataGetters> = if use_material3 {
            Box::new(BottomSheetDefaultsM3::new(context))
        } else {
            Box::new(BottomSheetThemeData::new())
        };
        let widget_ref = context.widget(app).clone();
        let widget = waft_widgets::downcast_widget::<BottomSheet>(&*widget_ref)
            .expect("a BottomSheetState builds its BottomSheet");
        let constraints = widget
            .constraints
            .or(bottom_sheet_theme.constraints)
            .or_else(|| defaults.constraints(app));
        let color = widget
            .background_color
            .or(bottom_sheet_theme.background_color)
            .or_else(|| defaults.background_color(app));
        let surface_tint_color = bottom_sheet_theme
            .surface_tint_color
            .or_else(|| defaults.surface_tint_color(app));
        let shadow_color = widget
            .shadow_color
            .or(bottom_sheet_theme.shadow_color)
            .or_else(|| defaults.shadow_color(app));
        let elevation = widget
            .elevation
            .or(bottom_sheet_theme.elevation)
            .or_else(|| defaults.elevation(app))
            .unwrap_or(0.0);
        let shape = widget
            .shape
            .clone()
            .or_else(|| bottom_sheet_theme.shape.clone())
            .or_else(|| defaults.shape(app));
        let clip_behavior = widget
            .clip_behavior
            .or(bottom_sheet_theme.clip_behavior)
            .unwrap_or(Clip::None);
        let show_drag_handle = widget
            .show_drag_handle
            .unwrap_or(widget.enable_drag && bottom_sheet_theme.show_drag_handle.unwrap_or(false));
        let enable_drag = widget.enable_drag;
        let builder = Rc::clone(&widget.builder);
        let drag_handle_color = widget.drag_handle_color.clone();
        let drag_handle_size = widget.drag_handle_size;
        let on_closing = Rc::clone(&widget.on_closing);

        let mut drag_handle: Option<WidgetRef> = None;
        if show_drag_handle {
            let handle = DragHandle {
                on_semantics_tap: Some(on_closing),
                handle_hover: Rc::new(move |app, hovering| {
                    self.handle_drag_handle_hover(app, hovering)
                }),
                states: app.get(self).drag_handle_states.clone(),
                drag_handle_color,
                drag_handle_size,
            }
            .into_widget();
            // Only add [_BottomSheetGestureDetector] to the drag handle when the rest of the
            // bottom sheet is not draggable. If the whole bottom sheet is draggable,
            // no need to add it.
            drag_handle = Some(if !enable_drag {
                self.gesture_detector(handle)
            } else {
                handle
            });
        }

        let child_key = app.get(self).child_key.clone();
        let content = if !show_drag_handle {
            builder(app, context)
        } else {
            Stack::new()
                .alignment(AlignmentGeometry::TOP_CENTER)
                .children([
                    drag_handle.expect("built when the drag handle shows"),
                    Padding::new(EdgeInsetsGeometry::only(
                        0.0,
                        K_MIN_INTERACTIVE_DIMENSION,
                        0.0,
                        0.0,
                    ))
                    .child(builder(app, context))
                    .into_widget(),
                ])
                .into_widget()
        };
        let mut material = Material::new()
            .key(child_key.into())
            .elevation(elevation)
            .clip_behavior(clip_behavior)
            .child(
                NotificationListener::<DraggableScrollableNotification>::new(content)
                    .on_notification(move |app, notification| {
                        self.extent_changed(app, notification)
                    })
                    .into_widget(),
            );
        material.color = color;
        material.surface_tint_color = surface_tint_color;
        material.shadow_color = shadow_color;
        material.shape = shape;
        let mut bottom_sheet = material.into_widget();

        if let Some(constraints) = constraints {
            bottom_sheet = Align::new()
                .alignment(AlignmentGeometry::BOTTOM_CENTER)
                .height_factor(1.0)
                .child(ConstrainedBox::new(constraints).child(bottom_sheet))
                .into_widget();
        }

        if !enable_drag {
            bottom_sheet
        } else {
            self.gesture_detector(bottom_sheet)
        }
    }
}

impl BottomSheetState {
    /// Dart's `_BottomSheetGestureDetector` around `child`, with this state's drag handlers.
    fn gesture_detector(self: Handle<Self>, child: WidgetRef) -> WidgetRef {
        BottomSheetGestureDetector {
            child,
            on_vertical_drag_start: Rc::new(move |app, details| {
                self.handle_drag_start(app, details)
            }),
            on_vertical_drag_update: Rc::new(move |app, details| {
                self.handle_drag_update(app, details)
            }),
            on_vertical_drag_end: Rc::new(move |app, details| self.handle_drag_end(app, details)),
        }
        .into_widget()
    }
}

// PERSISTENT BOTTOM SHEETS

// See scaffold.rs

/// Dart's `_DragHandle`.
struct DragHandle {
    on_semantics_tap: Option<VoidCallback>,
    handle_hover: ValueChanged<bool>,
    states: WidgetStates,
    drag_handle_color: Option<AnyColor>,
    drag_handle_size: Option<Size>,
}

impl Debug for DragHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DragHandle")
            .field("states", &self.states)
            .field("dragHandleColor", &self.drag_handle_color)
            .field("dragHandleSize", &self.drag_handle_size)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for DragHandle {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let bottom_sheet_theme = Theme::of(app, context).bottom_sheet_theme.clone();
        let m3_defaults = BottomSheetDefaultsM3::new(context);
        let handle_size = self
            .drag_handle_size
            .or(bottom_sheet_theme.drag_handle_size)
            .or_else(|| m3_defaults.drag_handle_size(app))
            .expect("the Material 3 defaults have a drag handle size");
        let color =
            <dyn WidgetStateProperty<Color>>::resolve_as(&self.drag_handle_color, &self.states)
                .or_else(|| {
                    <dyn WidgetStateProperty<Color>>::resolve_as(
                        &bottom_sheet_theme.drag_handle_color,
                        &self.states,
                    )
                })
                .or_else(|| m3_defaults.drag_handle_color(app))
                .map(|color| color.color());
        let mut decoration =
            BoxDecoration::new().border_radius(BorderRadius::circular(handle_size.height() / 2.0));
        decoration.color = color.map(Into::into);

        let enter_hover = Rc::clone(&self.handle_hover);
        let exit_hover = Rc::clone(&self.handle_hover);
        let mut semantics = Semantics::new()
            .label(<dyn MaterialLocalizations>::of(app, context).modal_barrier_dismiss_label())
            .container(true)
            .button(true);
        if let Some(on_semantics_tap) = self.on_semantics_tap.clone() {
            semantics = semantics.on_tap(on_semantics_tap);
        }
        MouseRegion::new()
            .on_enter(Rc::new(move |app: &mut App, _event: PointerEnterEvent| {
                enter_hover(app, true)
            }))
            .on_exit(Rc::new(move |app: &mut App, _event: PointerExitEvent| {
                exit_hover(app, false)
            }))
            .child(
                semantics.child(
                    SizedBox::new()
                        .width(handle_size.width().max(K_MIN_INTERACTIVE_DIMENSION))
                        .height(handle_size.height().max(K_MIN_INTERACTIVE_DIMENSION))
                        .child(
                            Center::new().child(
                                Container::new()
                                    .height(handle_size.height())
                                    .width(handle_size.width())
                                    .decoration(decoration),
                            ),
                        ),
                ),
            )
            .into_widget()
    }
}

/// Dart's `_BottomSheetLayoutWithSizeListener`.
struct BottomSheetLayoutWithSizeListener {
    on_child_size_changed: ValueChanged<Size>,
    animation_value: f64,
    is_scroll_controlled: bool,
    scroll_control_disabled_max_height_ratio: f64,
    child: Option<WidgetRef>,
}

impl Debug for BottomSheetLayoutWithSizeListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BottomSheetLayoutWithSizeListener")
            .field("animationValue", &self.animation_value)
            .field("isScrollControlled", &self.is_scroll_controlled)
            .field(
                "scrollControlDisabledMaxHeightRatio",
                &self.scroll_control_disabled_max_height_ratio,
            )
            .finish_non_exhaustive()
    }
}

impl RenderObjectWidget for BottomSheetLayoutWithSizeListener {
    type RenderObject = RenderBottomSheetLayoutWithSizeListener;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderBottomSheetLayoutWithSizeListener::new(
            app,
            None,
            Rc::clone(&self.on_child_size_changed),
            self.animation_value,
            self.is_scroll_controlled,
            self.scroll_control_disabled_max_height_ratio,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderBottomSheetLayoutWithSizeListener>,
    ) {
        render_object.set_on_child_size_changed(app, Rc::clone(&self.on_child_size_changed));
        render_object.set_animation_value(app, self.animation_value);
        render_object.set_is_scroll_controlled(app, self.is_scroll_controlled);
        render_object.set_scroll_control_disabled_max_height_ratio(
            app,
            self.scroll_control_disabled_max_height_ratio,
        );
    }
}

impl SingleChildRenderObjectWidget for BottomSheetLayoutWithSizeListener {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// Dart's `_RenderBottomSheetLayoutWithSizeListener`.
struct RenderBottomSheetLayoutWithSizeListener {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    last_size: Size,
    on_child_size_changed: ValueChanged<Size>,
    animation_value: f64,
    is_scroll_controlled: bool,
    scroll_control_disabled_max_height_ratio: f64,
}

impl RenderBottomSheetLayoutWithSizeListener {
    fn new(
        app: &mut App,
        child: Option<AnyRenderBox>,
        on_child_size_changed: ValueChanged<Size>,
        animation_value: f64,
        is_scroll_controlled: bool,
        scroll_control_disabled_max_height_ratio: f64,
    ) -> RenderHandle<RenderBottomSheetLayoutWithSizeListener> {
        let this = RenderHandle::new_box(
            app,
            RenderBottomSheetLayoutWithSizeListener {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                last_size: Size::ZERO,
                on_child_size_changed,
                animation_value,
                is_scroll_controlled,
                scroll_control_disabled_max_height_ratio,
            },
        );
        this.set_child(app, child);
        this
    }

    fn set_on_child_size_changed(
        self: RenderHandle<Self>,
        app: &mut App,
        new_callback: ValueChanged<Size>,
    ) {
        if Rc::ptr_eq(&self.get(app).on_child_size_changed, &new_callback) {
            return;
        }

        self.get_mut(app).on_child_size_changed = new_callback;
        self.mark_needs_layout(app);
    }

    fn animation_value(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).animation_value
    }

    fn set_animation_value(self: RenderHandle<Self>, app: &mut App, new_value: f64) {
        if self.get(app).animation_value == new_value {
            return;
        }

        self.get_mut(app).animation_value = new_value;
        self.mark_needs_layout(app);
    }

    fn is_scroll_controlled(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).is_scroll_controlled
    }

    fn set_is_scroll_controlled(self: RenderHandle<Self>, app: &mut App, new_value: bool) {
        if self.get(app).is_scroll_controlled == new_value {
            return;
        }

        self.get_mut(app).is_scroll_controlled = new_value;
        self.mark_needs_layout(app);
    }

    fn scroll_control_disabled_max_height_ratio(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).scroll_control_disabled_max_height_ratio
    }

    fn set_scroll_control_disabled_max_height_ratio(
        self: RenderHandle<Self>,
        app: &mut App,
        new_value: f64,
    ) {
        if self.get(app).scroll_control_disabled_max_height_ratio == new_value {
            return;
        }

        self.get_mut(app).scroll_control_disabled_max_height_ratio = new_value;
        self.mark_needs_layout(app);
    }

    fn get_constraints_for_child(
        self: RenderHandle<Self>,
        app: &App,
        constraints: BoxConstraints,
    ) -> BoxConstraints {
        BoxConstraints::new()
            .min_width(constraints.max_width)
            .max_width(constraints.max_width)
            .max_height(if self.is_scroll_controlled(app) {
                constraints.max_height
            } else {
                constraints.max_height * self.scroll_control_disabled_max_height_ratio(app)
            })
    }

    fn get_position_for_child(
        self: RenderHandle<Self>,
        app: &App,
        size: Size,
        child_size: Size,
    ) -> Offset {
        Offset::new(
            0.0,
            size.height() - child_size.height() * self.animation_value(app),
        )
    }
}

impl RenderObjectWithChildMixin for RenderBottomSheetLayoutWithSizeListener {
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

impl RenderShiftedBox for RenderBottomSheetLayoutWithSizeListener {}

impl RenderObject for RenderBottomSheetLayoutWithSizeListener {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let size = constraints.biggest();
        self.set_size(app, size);
        let Some(child) = self.child(app) else {
            return;
        };

        let child_constraints = self.get_constraints_for_child(app, constraints);
        debug_assert!(child_constraints.debug_assert_is_valid(true));
        child.layout(app, child_constraints, !child_constraints.is_tight());
        let child_size = if child_constraints.is_tight() {
            child_constraints.smallest()
        } else {
            child.size(app)
        };
        child.parent_data_of_mut::<BoxParentData>(app).offset =
            self.get_position_for_child(app, size, child_size);

        if self.get(app).last_size != child_size {
            self.get_mut(app).last_size = child_size;
            let on_child_size_changed = Rc::clone(&self.get(app).on_child_size_changed);
            on_child_size_changed(app, child_size);
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

impl RenderBox for RenderBottomSheetLayoutWithSizeListener {
    waft_rendering::render_box_accessors!();

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, _app: &mut App, _height: f64) -> f64 {
        0.0
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, _app: &mut App, _height: f64) -> f64 {
        0.0
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, _app: &mut App, _width: f64) -> f64 {
        0.0
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, _app: &mut App, _width: f64) -> f64 {
        0.0
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        _app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        constraints.biggest()
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let child_constraints = self.get_constraints_for_child(app, constraints);
        let result = child.get_dry_baseline(app, child_constraints, baseline)?;
        let child_size = if child_constraints.is_tight() {
            child_constraints.smallest()
        } else {
            child.get_dry_layout(app, child_constraints)
        };
        Some(
            result
                + self
                    .get_position_for_child(app, constraints.biggest(), child_size)
                    .dy(),
        )
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
}

/// Dart's `_ModalBottomSheet<T>`.
struct ModalBottomSheet {
    route: Handle<ModalBottomSheetRoute>,
    is_scroll_controlled: bool,
    scroll_control_disabled_max_height_ratio: f64,
    background_color: Option<Color>,
    elevation: Option<f64>,
    shape: Option<ShapeBorderRef>,
    clip_behavior: Option<Clip>,
    constraints: Option<BoxConstraints>,
    enable_drag: bool,
    show_drag_handle: bool,
    animation_style: Option<AnimationStyle>,
}

impl ModalBottomSheet {
    fn new(route: Handle<ModalBottomSheetRoute>) -> ModalBottomSheet {
        ModalBottomSheet {
            route,
            is_scroll_controlled: false,
            scroll_control_disabled_max_height_ratio:
                K_DEFAULT_SCROLL_CONTROL_DISABLED_MAX_HEIGHT_RATIO,
            background_color: None,
            elevation: None,
            shape: None,
            clip_behavior: None,
            constraints: None,
            enable_drag: true,
            show_drag_handle: false,
            animation_style: None,
        }
    }
}

impl Debug for ModalBottomSheet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModalBottomSheet")
            .field("route", &self.route)
            .field("isScrollControlled", &self.is_scroll_controlled)
            .field(
                "scrollControlDisabledMaxHeightRatio",
                &self.scroll_control_disabled_max_height_ratio,
            )
            .field("backgroundColor", &self.background_color)
            .field("elevation", &self.elevation)
            .field("shape", &self.shape)
            .field("clipBehavior", &self.clip_behavior)
            .field("constraints", &self.constraints)
            .field("enableDrag", &self.enable_drag)
            .field("showDragHandle", &self.show_drag_handle)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for ModalBottomSheet {
    type State = ModalBottomSheetState;

    fn create_state(&self) -> ModalBottomSheetState {
        ModalBottomSheetState {
            state: StateData::new(),
            sheet_animation: None,
            curved_sheet_animation: None,
            drag_end_curved_animation: None,
        }
    }
}

/// Dart's `_ModalBottomSheetState<T>`.
struct ModalBottomSheetState {
    state: StateData<ModalBottomSheet>,
    /// Dart's `late final _sheetAnimation`, set in `init_state`.
    sheet_animation: Option<Ref<ProxyAnimation>>,
    /// Dart's `late final _curvedSheetAnimation`, set in `init_state`.
    curved_sheet_animation: Option<Ref<CurvedAnimation>>,
    // The CurvedAnimation created for the most recent drag end, tracked so it can
    // be disposed when it is replaced or the state is disposed.
    drag_end_curved_animation: Option<Ref<CurvedAnimation>>,
}

impl ModalBottomSheetState {
    fn sheet_animation(self: Handle<Self>, app: &App) -> Handle<ProxyAnimation> {
        app.get(self)
            .sheet_animation
            .handle()
            .expect("set in init_state")
    }

    fn curved_sheet_animation(self: Handle<Self>, app: &App) -> Handle<CurvedAnimation> {
        app.get(self)
            .curved_sheet_animation
            .handle()
            .expect("set in init_state")
    }

    /// Dart's `widget.route.animation!`.
    fn route_animation(self: Handle<Self>, app: &App) -> AnimationRef<f64> {
        ModalRoute::animation(self.widget(app).route, app)
            .and_then(|animation| animation.upgrade(app))
            .expect("an installed route")
    }

    fn get_route_label(app: &App, localizations: &dyn MaterialLocalizations) -> String {
        match app.platform().target_platform() {
            TargetPlatform::IOS | TargetPlatform::MacOS => String::new(),
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => localizations.dialog_label(),
        }
    }

    fn get_new_clip_details(top_layer_size: Size) -> EdgeInsets {
        EdgeInsets::from_ltrb(0.0, 0.0, 0.0, top_layer_size.height())
    }

    fn handle_drag_start(self: Handle<Self>, app: &mut App, _details: DragStartDetails) {
        // Allow the bottom sheet to track the user's finger accurately.
        let route_animation = self.route_animation(app);
        self.sheet_animation(app)
            .set_parent(app, Some(route_animation));
        // The CurvedAnimation from the previous drag end is no longer the parent of
        // [_sheetAnimation], so dispose it to avoid leaking its listeners on the
        // route animation.
        if let Some(drag_end_curved_animation) = app.get_mut(self).drag_end_curved_animation.take()
        {
            drag_end_curved_animation.dispose(app);
        }
    }

    fn handle_drag_end(
        self: Handle<Self>,
        app: &mut App,
        _details: DragEndDetails,
        _is_closing: bool,
    ) {
        let route_animation = self.route_animation(app);
        let current_progress = route_animation.value(app);

        // Rebind the animation using CurvedAnimation and Split so the
        // remaining transition continues smoothly from the exact point
        // where the drag gesture ended.
        let old_drag_end_curved_animation = app.get(self).drag_end_curved_animation.handle();
        let animation_style = self.widget(app).animation_style.clone();
        let curve = animation_style
            .as_ref()
            .and_then(|style| style.curve.clone())
            .unwrap_or_else(k_modal_bottom_sheet_curve);
        let reverse_curve = animation_style
            .as_ref()
            .and_then(|style| style.reverse_curve.clone())
            .unwrap_or_else(k_modal_bottom_sheet_curve);
        let drag_end_curved_animation = CurvedAnimation::new(
            app,
            route_animation,
            Rc::new(Split::new(current_progress, Curves::linear(), curve)),
            Some(Rc::new(Split::new(
                current_progress,
                Curves::linear(),
                reverse_curve,
            ))),
        );
        app.get_mut(self).drag_end_curved_animation = Some(drag_end_curved_animation.clone());
        self.sheet_animation(app)
            .set_parent(app, Some(drag_end_curved_animation.erase()));
        // Dispose the previous drag-end CurvedAnimation only after detaching it
        // from [_sheetAnimation], so it is never disposed while still in use.
        if let Some(old_drag_end_curved_animation) = old_drag_end_curved_animation {
            old_drag_end_curved_animation.dispose(app);
        }
    }
}

impl State for ModalBottomSheetState {
    type Widget = ModalBottomSheet;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let route_animation = self.route_animation(app);
        let animation_style = self.widget(app).animation_style.clone();
        let curved_sheet_animation = CurvedAnimation::new(
            app,
            route_animation,
            animation_style
                .as_ref()
                .and_then(|style| style.curve.clone())
                .unwrap_or_else(k_modal_bottom_sheet_curve),
            Some(
                animation_style
                    .as_ref()
                    .and_then(|style| style.reverse_curve.clone())
                    .unwrap_or_else(k_modal_bottom_sheet_curve),
            ),
        );
        let sheet_animation = ProxyAnimation::new(app, Some(curved_sheet_animation.erase()));
        let this = app.get_mut(self);
        this.curved_sheet_animation = Some(curved_sheet_animation);
        this.sheet_animation = Some(sheet_animation);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &ModalBottomSheet) {
        if cfg!(debug_assertions) {
            let widget = self.widget(app);
            assert!(old_widget.route == widget.route);
            let curved_sheet_animation = app.get(self.curved_sheet_animation(app));
            let curve = widget
                .animation_style
                .as_ref()
                .and_then(|style| style.curve.clone())
                .unwrap_or_else(k_modal_bottom_sheet_curve);
            assert!(Rc::ptr_eq(&curved_sheet_animation.curve, &curve));
            let reverse_curve = widget
                .animation_style
                .as_ref()
                .and_then(|style| style.reverse_curve.clone())
                .unwrap_or_else(k_modal_bottom_sheet_curve);
            assert!(
                curved_sheet_animation
                    .reverse_curve
                    .as_ref()
                    .is_some_and(|curve| Rc::ptr_eq(curve, &reverse_curve))
            );
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        // Detach to avoid leaking listeners on the route animation.
        let always_dismissed = k_always_dismissed_animation(app);
        self.sheet_animation(app)
            .set_parent(app, Some(always_dismissed));
        let curved_sheet_animation = self.curved_sheet_animation(app);
        curved_sheet_animation.dispose(app);
        if let Some(drag_end_curved_animation) = app.get(self).drag_end_curved_animation.handle() {
            drag_end_curved_animation.dispose(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_media_query(app, context));
        debug_assert!(debug_check_has_material_localizations(app, context));
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let route_label = Self::get_route_label(app, &*localizations);

        let widget = self.widget(app);
        let route = widget.route;
        let mut bottom_sheet = BottomSheet::new(
            Rc::new(move |app: &mut App| {
                if Route::is_current(route, app) {
                    Navigator::pop(app, context, None);
                }
            }),
            Rc::clone(&app.get(route).builder),
        )
        .enable_drag(widget.enable_drag)
        .show_drag_handle(widget.show_drag_handle)
        .on_drag_start(move |app, details| self.handle_drag_start(app, details))
        .on_drag_end(move |app, details, is_closing| {
            self.handle_drag_end(app, details, is_closing)
        });
        bottom_sheet.animation_controller = app.get(route).animation_controller.clone();
        bottom_sheet.background_color = widget.background_color;
        bottom_sheet.elevation = widget.elevation;
        bottom_sheet.shape = widget.shape.clone();
        bottom_sheet.clip_behavior = widget.clip_behavior;
        bottom_sheet.constraints = widget.constraints;
        let is_scroll_controlled = widget.is_scroll_controlled;
        let scroll_control_disabled_max_height_ratio =
            widget.scroll_control_disabled_max_height_ratio;

        let sheet_animation = self.sheet_animation(app);
        let listenable: Rc<dyn Listenable> = Rc::new(
            app.get(self)
                .sheet_animation
                .as_ref()
                .expect("set in init_state")
                .erase::<AnyAnimation<f64>>(),
        );
        ListenableBuilder::new(listenable, move |app, _context, child| {
            let animation_value = sheet_animation.value(app);
            Semantics::new()
                .scopes_route(true)
                .names_route(true)
                .label(route_label.clone())
                .explicit_child_nodes(true)
                .child(ClipRect::new().child(BottomSheetLayoutWithSizeListener {
                    on_child_size_changed: Rc::new(move |app, size| {
                        route.did_change_barrier_semantics_clip(
                            app,
                            ModalBottomSheetState::get_new_clip_details(size),
                        );
                    }),
                    animation_value,
                    is_scroll_controlled,
                    scroll_control_disabled_max_height_ratio,
                    child: child.cloned(),
                }))
                .into_widget()
        })
        .child(bottom_sheet)
        .into_widget()
    }
}

/// A route that represents a Material Design modal bottom sheet.
///
/// A modal bottom sheet is an alternative to a menu or a dialog and prevents
/// the user from interacting with the rest of the app.
///
/// A closely related widget is a persistent bottom sheet, which shows
/// information that supplements the primary content of the app without
/// preventing the user from interacting with the app. Persistent bottom sheets
/// can be created and displayed with the [`show_bottom_sheet`] function or the
/// `ScaffoldState::show_bottom_sheet` method.
///
/// The [`is_scroll_controlled`](Self::is_scroll_controlled) parameter specifies whether this
/// is a route for a bottom sheet that will utilize `DraggableScrollableSheet`. Consider
/// setting this parameter to true if this bottom sheet has
/// a scrollable child, such as a `ListView` or a `GridView`,
/// to have the bottom sheet be draggable.
///
/// The [`is_dismissible`](Self::is_dismissible) parameter specifies whether the bottom sheet
/// will be dismissed when user taps on the scrim.
///
/// The [`enable_drag`](Self::enable_drag) parameter specifies whether the bottom sheet can be
/// dragged up and down and dismissed by swiping downwards.
///
/// The [`use_safe_area`](Self::use_safe_area) parameter specifies whether the sheet will
/// avoid system intrusions on the top, left, and right. If false, no [`SafeArea`] is added;
/// and [`MediaQuery::remove_padding`] is applied to the top,
/// so that system intrusions at the top will not be avoided by a [`SafeArea`]
/// inside the bottom sheet either.
/// Defaults to false.
///
/// The optional [`background_color`](Self::background_color), [`elevation`](Self::elevation),
/// [`shape`](Self::shape), [`clip_behavior`](Self::clip_behavior),
/// [`constraints`](Self::constraints) and
/// [`transition_animation_controller`](Self::transition_animation_controller)
/// parameters can be passed in to customize the appearance and behavior of
/// modal bottom sheets (see the documentation for these on [`BottomSheet`]
/// for more details).
///
/// The [`transition_animation_controller`](Self::transition_animation_controller) controls
/// the bottom sheet's entrance and exit animations. It's up to the owner of the controller to
/// call [`AnimationController::dispose`] when the controller is no longer needed.
///
/// The optional `settings` parameter sets the `RouteSettings` of the modal bottom sheet
/// sheet. This is particularly useful in the case that a user wants to observe
/// [`PopupRoute`]s within a `NavigatorObserver`.
///
/// See also:
///
///  * [`show_modal_bottom_sheet`], which is a way to display a ModalBottomSheetRoute.
///  * [`BottomSheet`], which becomes the parent of the widget returned by the
///    function passed as the `builder` argument to [`show_modal_bottom_sheet`].
///  * [`show_bottom_sheet`] and `ScaffoldState::show_bottom_sheet`, for showing
///    non-modal bottom sheets.
///  * `DraggableScrollableSheet`, creates a bottom sheet that grows
///    and then becomes scrollable once it reaches its maximum size.
///  * The Material 2 spec at <https://m2.material.io/components/sheets-bottom>.
///  * The Material 3 spec at <https://m3.material.io/components/bottom-sheets/overview>.
pub struct ModalBottomSheetRoute {
    route: RouteData,
    overlay_route: OverlayRouteData,
    transition_route: TransitionRouteData,
    local_history: LocalHistoryRouteData,
    modal_route: ModalRouteData,

    /// A builder for the contents of the sheet.
    ///
    /// The bottom sheet will wrap the widget produced by this builder in a
    /// [`Material`] widget.
    pub builder: WidgetBuilder,

    /// Stores a list of captured [`InheritedTheme`]s that are wrapped around the
    /// bottom sheet.
    ///
    /// Consider setting this attribute when the [`ModalBottomSheetRoute`]
    /// is created through `Navigator::push` and its friends.
    pub captured_themes: Option<CapturedThemes>,

    /// Specifies whether this is a route for a bottom sheet that will utilize
    /// `DraggableScrollableSheet`.
    ///
    /// Consider setting this parameter to true if this bottom sheet has
    /// a scrollable child, such as a `ListView` or a `GridView`,
    /// to have the bottom sheet be draggable.
    pub is_scroll_controlled: bool,

    /// The max height constraint ratio for the bottom sheet
    /// when [`is_scroll_controlled`](Self::is_scroll_controlled) is set to false,
    /// no ratio will be applied when [`is_scroll_controlled`](Self::is_scroll_controlled) is
    /// set to true.
    ///
    /// Defaults to 9 / 16.
    pub scroll_control_disabled_max_height_ratio: f64,

    /// The bottom sheet's background color.
    ///
    /// Defines the bottom sheet's `Material.color`.
    ///
    /// If this property is not provided, it falls back to [`Material`]'s default.
    pub background_color: Option<Color>,

    /// The z-coordinate at which to place this material relative to its parent.
    ///
    /// This controls the size of the shadow below the material.
    ///
    /// Defaults to 0, must not be negative.
    pub elevation: Option<f64>,

    /// The shape of the bottom sheet.
    ///
    /// Defines the bottom sheet's `Material.shape`.
    ///
    /// If this property is not provided, it falls back to [`Material`]'s default.
    pub shape: Option<ShapeBorderRef>,

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defines the bottom sheet's `Material.clipBehavior`.
    ///
    /// Use this property to enable clipping of content when the bottom sheet has
    /// a custom [`shape`](Self::shape) and the content can extend past this shape. For
    /// example, a bottom sheet with rounded corners and an edge-to-edge `Image` at the
    /// top.
    ///
    /// If this property is null, the [`BottomSheetThemeData::clip_behavior`] of
    /// `ThemeData.bottomSheetTheme` is used. If that's null, the behavior defaults to
    /// [`Clip::None`] will be [`Clip::None`].
    pub clip_behavior: Option<Clip>,

    /// Defines minimum and maximum sizes for a [`BottomSheet`].
    ///
    /// If null, the ambient `ThemeData.bottomSheetTheme`'s
    /// [`BottomSheetThemeData::constraints`] will be used. If that
    /// is null and `ThemeData.useMaterial3` is true, then the bottom sheet
    /// will have a max width of 640dp. If `ThemeData.useMaterial3` is false, then
    /// the bottom sheet's size will be constrained by its parent
    /// (usually a `Scaffold`). In this case, consider limiting the width by
    /// setting smaller constraints for large screens.
    ///
    /// If constraints are specified (either in this property or in the
    /// theme), the bottom sheet will be aligned to the bottom-center of
    /// the available space. Otherwise, no alignment is applied.
    pub constraints: Option<BoxConstraints>,

    /// Specifies the color of the modal barrier that darkens everything below the
    /// bottom sheet.
    ///
    /// Defaults to `Colors.black54` if not provided.
    pub modal_barrier_color: Option<Color>,

    /// Specifies whether the bottom sheet will be dismissed
    /// when user taps on the scrim.
    ///
    /// If true, the bottom sheet will be dismissed when user taps on the scrim.
    ///
    /// Defaults to true.
    pub is_dismissible: bool,

    /// Specifies whether the bottom sheet can be dragged up and down
    /// and dismissed by swiping downwards.
    ///
    /// If true, the bottom sheet can be dragged up and down and dismissed by
    /// swiping downwards.
    ///
    /// This applies to the content below the drag handle, if showDragHandle is true.
    ///
    /// Defaults is true.
    pub enable_drag: bool,

    /// Specifies whether a drag handle is shown.
    ///
    /// The drag handle appears at the top of the bottom sheet. The default color is
    /// `ColorScheme.onSurfaceVariant` with an opacity of 0.4 and can be customized
    /// using dragHandleColor. The default size is `Size(32,4)` and can be customized
    /// with dragHandleSize.
    ///
    /// If null, then the value of [`BottomSheetThemeData::show_drag_handle`] is used. If
    /// that is also null, defaults to false.
    pub show_drag_handle: Option<bool>,

    /// The animation controller that controls the bottom sheet's entrance and
    /// exit animations.
    ///
    /// The BottomSheet widget will manipulate the position of this animation, it
    /// is not just a passive observer.
    pub transition_animation_controller: Option<Ref<AnimationController>>,

    /// Whether to avoid system intrusions on the top, left, and right.
    ///
    /// If true, a [`SafeArea`] is inserted to keep the bottom sheet away from
    /// system intrusions at the top, left, and right sides of the screen.
    ///
    /// If false, the bottom sheet will extend through any system intrusions
    /// at the top, left, and right.
    ///
    /// If false, then moreover [`MediaQuery::remove_padding`] will be used
    /// to remove top padding, so that a [`SafeArea`] widget inside the bottom
    /// sheet will have no effect at the top edge. If this is undesired, consider
    /// setting [`use_safe_area`](Self::use_safe_area) to true. Alternatively, wrap the
    /// [`SafeArea`] in a [`MediaQuery`] that restates an ambient `MediaQueryData` from outside
    /// [`builder`](Self::builder).
    ///
    /// In either case, the bottom sheet extends all the way to the bottom of
    /// the screen, including any system intrusions.
    ///
    /// The default is false.
    pub use_safe_area: bool,

    /// Used to override the modal bottom sheet animation duration and reverse
    /// animation duration.
    ///
    /// If `AnimationStyle.duration` is provided, it will be used to override
    /// the modal bottom sheet animation duration in the underlying
    /// [`BottomSheet::create_animation_controller`].
    ///
    /// If `AnimationStyle.reverseDuration` is provided, it will be used to
    /// override the modal bottom sheet reverse animation duration in the
    /// underlying [`BottomSheet::create_animation_controller`].
    ///
    /// To disable the modal bottom sheet animation, use `AnimationStyle.noAnimation`.
    pub sheet_animation_style: Option<AnimationStyle>,

    /// The semantic hint text that informs users what will happen if they
    /// tap on the widget. Announced in the format of 'Double tap to ...'.
    ///
    /// If the field is null, the default hint will be used, which results in
    /// announcement of 'Double tap to activate'.
    ///
    /// See also:
    ///
    ///  * [`ModalRoute::barrier_dismissible`], which controls the behavior of the barrier
    ///    when tapped.
    ///  * [`ModalBarrier`], which uses this field as onTapHint when it has an onTap action.
    pub barrier_on_tap_hint: Option<String>,

    clip_details_notifier: Ref<ValueNotifier<EdgeInsets>>,

    barrier_label: Option<String>,

    animation_controller: Option<Ref<AnimationController>>,
}

impl ModalBottomSheetRoute {
    /// A modal bottom sheet route; Dart's optional arguments are the setters.
    pub fn new(
        app: &mut App,
        builder: WidgetBuilder,
        is_scroll_controlled: bool,
    ) -> Ref<ModalBottomSheetRoute> {
        let route = RouteData::new(app, None, None);
        let transition_route = TransitionRouteData::new(app);
        let modal_route = ModalRouteData::new(app);
        let clip_details_notifier = app.create(ValueNotifier::new(EdgeInsets::ZERO));
        app.create(ModalBottomSheetRoute {
            route,
            overlay_route: OverlayRouteData::new(),
            transition_route,
            local_history: LocalHistoryRouteData::new(),
            modal_route,
            builder,
            captured_themes: None,
            is_scroll_controlled,
            scroll_control_disabled_max_height_ratio:
                K_DEFAULT_SCROLL_CONTROL_DISABLED_MAX_HEIGHT_RATIO,
            background_color: None,
            elevation: None,
            shape: None,
            clip_behavior: None,
            constraints: None,
            modal_barrier_color: None,
            is_dismissible: true,
            enable_drag: true,
            show_drag_handle: None,
            transition_animation_controller: None,
            use_safe_area: false,
            sheet_animation_style: None,
            barrier_on_tap_hint: None,
            clip_details_notifier,
            barrier_label: None,
            animation_controller: None,
        })
    }

    /// Dart `ModalBottomSheetRoute(capturedThemes:)`.
    pub fn captured_themes(
        self: Ref<Self>,
        app: &mut App,
        captured_themes: CapturedThemes,
    ) -> Ref<Self> {
        app.get_mut(&self).captured_themes = Some(captured_themes);
        self
    }

    /// Dart `ModalBottomSheetRoute(barrierLabel:)`.
    pub fn barrier_label(self: Ref<Self>, app: &mut App, barrier_label: String) -> Ref<Self> {
        app.get_mut(&self).barrier_label = Some(barrier_label);
        self
    }

    /// Dart `ModalBottomSheetRoute(barrierOnTapHint:)`.
    pub fn barrier_on_tap_hint(
        self: Ref<Self>,
        app: &mut App,
        barrier_on_tap_hint: String,
    ) -> Ref<Self> {
        app.get_mut(&self).barrier_on_tap_hint = Some(barrier_on_tap_hint);
        self
    }

    /// Dart `ModalBottomSheetRoute(backgroundColor:)`.
    pub fn background_color(self: Ref<Self>, app: &mut App, background_color: Color) -> Ref<Self> {
        app.get_mut(&self).background_color = Some(background_color);
        self
    }

    /// Dart `ModalBottomSheetRoute(elevation:)`.
    pub fn elevation(self: Ref<Self>, app: &mut App, elevation: f64) -> Ref<Self> {
        app.get_mut(&self).elevation = Some(elevation);
        self
    }

    /// Dart `ModalBottomSheetRoute(shape:)`.
    pub fn shape(self: Ref<Self>, app: &mut App, shape: impl Into<ShapeBorderRef>) -> Ref<Self> {
        app.get_mut(&self).shape = Some(shape.into());
        self
    }

    /// Dart `ModalBottomSheetRoute(clipBehavior:)`.
    pub fn clip_behavior(self: Ref<Self>, app: &mut App, clip_behavior: Clip) -> Ref<Self> {
        app.get_mut(&self).clip_behavior = Some(clip_behavior);
        self
    }

    /// Dart `ModalBottomSheetRoute(constraints:)`.
    pub fn constraints(self: Ref<Self>, app: &mut App, constraints: BoxConstraints) -> Ref<Self> {
        app.get_mut(&self).constraints = Some(constraints);
        self
    }

    /// Dart `ModalBottomSheetRoute(modalBarrierColor:)`.
    pub fn modal_barrier_color(
        self: Ref<Self>,
        app: &mut App,
        modal_barrier_color: Color,
    ) -> Ref<Self> {
        app.get_mut(&self).modal_barrier_color = Some(modal_barrier_color);
        self
    }

    /// Dart `ModalBottomSheetRoute(isDismissible:)`.
    pub fn is_dismissible(self: Ref<Self>, app: &mut App, is_dismissible: bool) -> Ref<Self> {
        app.get_mut(&self).is_dismissible = is_dismissible;
        self
    }

    /// Dart `ModalBottomSheetRoute(enableDrag:)`.
    pub fn enable_drag(self: Ref<Self>, app: &mut App, enable_drag: bool) -> Ref<Self> {
        app.get_mut(&self).enable_drag = enable_drag;
        self
    }

    /// Dart `ModalBottomSheetRoute(showDragHandle:)`.
    pub fn show_drag_handle(self: Ref<Self>, app: &mut App, show_drag_handle: bool) -> Ref<Self> {
        app.get_mut(&self).show_drag_handle = Some(show_drag_handle);
        self
    }

    /// Dart `ModalBottomSheetRoute(scrollControlDisabledMaxHeightRatio:)`.
    pub fn scroll_control_disabled_max_height_ratio(
        self: Ref<Self>,
        app: &mut App,
        scroll_control_disabled_max_height_ratio: f64,
    ) -> Ref<Self> {
        app.get_mut(&self).scroll_control_disabled_max_height_ratio =
            scroll_control_disabled_max_height_ratio;
        self
    }

    /// Dart `ModalBottomSheetRoute(settings:)`.
    pub fn settings(self: Ref<Self>, app: &mut App, settings: RouteSettingsRef) -> Ref<Self> {
        self.route_data_mut(app).set_settings(settings);
        self
    }

    /// Dart `ModalBottomSheetRoute(requestFocus:)`.
    pub fn request_focus(self: Ref<Self>, app: &mut App, request_focus: bool) -> Ref<Self> {
        self.route_data_mut(app)
            .set_request_focus(Some(request_focus));
        self
    }

    /// Dart `ModalBottomSheetRoute(transitionAnimationController:)`.
    pub fn transition_animation_controller(
        self: Ref<Self>,
        app: &mut App,
        transition_animation_controller: Ref<AnimationController>,
    ) -> Ref<Self> {
        app.get_mut(&self).transition_animation_controller = Some(transition_animation_controller);
        self
    }

    /// Dart `ModalBottomSheetRoute(useSafeArea:)`.
    pub fn use_safe_area(self: Ref<Self>, app: &mut App, use_safe_area: bool) -> Ref<Self> {
        app.get_mut(&self).use_safe_area = use_safe_area;
        self
    }

    /// Dart `ModalBottomSheetRoute(sheetAnimationStyle:)`.
    pub fn sheet_animation_style(
        self: Ref<Self>,
        app: &mut App,
        sheet_animation_style: AnimationStyle,
    ) -> Ref<Self> {
        app.get_mut(&self).sheet_animation_style = Some(sheet_animation_style);
        self
    }

    /// Updates the details regarding how the `SemanticsNode.rect` (focus) of
    /// the barrier for this [`ModalBottomSheetRoute`] should be clipped.
    ///
    /// Returns true if the clipDetails did change and false otherwise.
    fn did_change_barrier_semantics_clip(
        self: Handle<Self>,
        app: &mut App,
        new_clip_details: EdgeInsets,
    ) -> bool {
        let clip_details_notifier = app.get(self).clip_details_notifier.clone();
        if *app.get(&clip_details_notifier).value() == new_clip_details {
            return false;
        }
        clip_details_notifier.set_value(app, new_clip_details);
        true
    }
}

// Dart's `ModalBottomSheetRoute.dispose` disposes the clip notifier and runs `super.dispose()`,
// so this impl writes out `modal_route_overrides!` with that `dispose`.
impl Route for ModalBottomSheetRoute {
    waft_widgets::route_accessors!();

    fn as_transition_route(self: Handle<Self>) -> Option<waft_widgets::AnyTransitionRoute> {
        Some(TransitionRoute::as_transition_route(self))
    }

    fn as_modal_route(self: Handle<Self>) -> Option<waft_widgets::AnyModalRoute> {
        Some(ModalRoute::as_modal_route(self))
    }

    fn overlay_entries(self: Handle<Self>, app: &App) -> Vec<Handle<waft_widgets::OverlayEntry>> {
        OverlayRoute::overlay_entries(self, app)
    }

    fn install(self: Handle<Self>, app: &mut App) {
        ModalRoute::install(self, app)
    }

    fn did_push(self: Handle<Self>, app: &mut App) -> waft_scheduler::TickerFuture {
        ModalRoute::did_push(self, app)
    }

    fn did_add(self: Handle<Self>, app: &mut App) {
        ModalRoute::did_add(self, app)
    }

    fn did_replace(self: Handle<Self>, app: &mut App, old_route: Option<waft_widgets::AnyRoute>) {
        TransitionRoute::did_replace(self, app, old_route)
    }

    fn will_pop(self: Handle<Self>, app: &mut App) -> waft_widgets::RoutePopDisposition {
        LocalHistoryRoute::will_pop(self, app)
    }

    fn pop_disposition(self: Handle<Self>, app: &mut App) -> waft_widgets::RoutePopDisposition {
        ModalRoute::pop_disposition(self, app)
    }

    fn on_pop_invoked_with_result(
        self: Handle<Self>,
        app: &mut App,
        did_pop: bool,
        result: RouteResult,
    ) {
        ModalRoute::on_pop_invoked_with_result(self, app, did_pop, result)
    }

    fn will_handle_pop_internally(self: Handle<Self>, app: &App) -> bool {
        LocalHistoryRoute::will_handle_pop_internally(self, app)
    }

    fn did_pop(self: Handle<Self>, app: &mut App, result: RouteResult) -> bool {
        LocalHistoryRoute::did_pop(self, app, result)
    }

    fn did_pop_next(self: Handle<Self>, app: &mut App, next_route: waft_widgets::AnyRoute) {
        ModalRoute::did_pop_next(self, app, next_route)
    }

    fn did_change_next(
        self: Handle<Self>,
        app: &mut App,
        next_route: Option<waft_widgets::AnyRoute>,
    ) {
        ModalRoute::did_change_next(self, app, next_route)
    }

    fn did_change_previous(
        self: Handle<Self>,
        app: &mut App,
        previous_route: Option<waft_widgets::AnyRoute>,
    ) {
        ModalRoute::did_change_previous(self, app, previous_route)
    }

    fn changed_internal_state(self: Handle<Self>, app: &mut App) {
        ModalRoute::changed_internal_state(self, app)
    }

    fn changed_external_state(self: Handle<Self>, app: &mut App) {
        ModalRoute::changed_external_state(self, app)
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let clip_details_notifier = app.get(self).clip_details_notifier.clone();
        clip_details_notifier.dispose(app);
        TransitionRoute::dispose(self, app)
    }
}

impl OverlayRoute for ModalBottomSheetRoute {
    waft_widgets::modal_overlay_route_overrides!();
}

impl PredictiveBackRoute for ModalBottomSheetRoute {
    waft_widgets::modal_predictive_back_route_overrides!(ModalRoute::pop_gesture_enabled);
}

impl TransitionRoute for ModalBottomSheetRoute {
    waft_widgets::modal_transition_route_overrides!();

    fn transition_duration(self: Handle<Self>, app: &mut App) -> Duration {
        let route = app.get(self);
        route
            .transition_animation_controller
            .as_ref()
            .and_then(|controller| app.get(controller).duration)
            .or(route
                .sheet_animation_style
                .as_ref()
                .and_then(|style| style.duration))
            .unwrap_or(K_BOTTOM_SHEET_ENTER_DURATION)
    }

    fn reverse_transition_duration(self: Handle<Self>, app: &mut App) -> Duration {
        let route = app.get(self);
        route
            .transition_animation_controller
            .as_ref()
            .and_then(|controller| app.get(controller).reverse_duration)
            .or(route
                .transition_animation_controller
                .as_ref()
                .and_then(|controller| app.get(controller).duration))
            .or(route
                .sheet_animation_style
                .as_ref()
                .and_then(|style| style.reverse_duration))
            .unwrap_or(K_BOTTOM_SHEET_EXIT_DURATION)
    }

    fn opaque(self: Handle<Self>, app: &App) -> bool {
        PopupRoute::opaque(self, app)
    }

    fn allow_snapshotting(self: Handle<Self>, app: &App) -> bool {
        PopupRoute::allow_snapshotting(self, app)
    }

    fn create_animation_controller(self: Handle<Self>, app: &mut App) -> Ref<AnimationController> {
        debug_assert!(app.get(self).animation_controller.is_none());
        let animation_controller = if let Some(transition_animation_controller) =
            app.get(self).transition_animation_controller.clone()
        {
            self.transition_route_data_mut(app)
                .will_dispose_animation_controller = false;
            transition_animation_controller
        } else {
            let navigator = self
                .as_route()
                .navigator(app)
                .expect("an installed route has a navigator");
            let sheet_animation_style = app.get(self).sheet_animation_style.clone();
            BottomSheet::create_animation_controller(app, navigator, sheet_animation_style.as_ref())
        };
        app.get_mut(self).animation_controller = Some(animation_controller.clone());
        animation_controller
    }
}

impl LocalHistoryRoute for ModalBottomSheetRoute {
    waft_widgets::local_history_route_accessors!();
}

impl ModalRoute for ModalBottomSheetRoute {
    waft_widgets::modal_route_accessors!();

    fn barrier_dismissible(self: Handle<Self>, app: &App) -> bool {
        app.get(self).is_dismissible
    }

    fn barrier_label(self: Handle<Self>, app: &App) -> Option<String> {
        app.get(self).barrier_label.clone()
    }

    fn barrier_color(self: Handle<Self>, app: &App) -> Option<Color> {
        Some(app.get(self).modal_barrier_color.unwrap_or(Colors::BLACK54))
    }

    fn maintain_state(self: Handle<Self>, app: &App) -> bool {
        PopupRoute::maintain_state(self, app)
    }

    fn build_page(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        animation: AnimationRef<f64>,
        secondary_animation: AnimationRef<f64>,
    ) -> WidgetRef {
        let _ = (animation, secondary_animation);
        let content = Builder::new(move |app, context| {
            let theme = Theme::of(app, context);
            let sheet_theme = theme.bottom_sheet_theme.clone();
            let defaults: Box<dyn BottomSheetThemeDataGetters> = if theme.use_material3 {
                Box::new(BottomSheetDefaultsM3::new(context))
            } else {
                Box::new(BottomSheetThemeData::new())
            };
            let route = app.get(self);
            let mut modal_bottom_sheet = ModalBottomSheet::new(self);
            modal_bottom_sheet.animation_style = route.sheet_animation_style.clone();
            let (background_color, elevation) = (route.background_color, route.elevation);
            modal_bottom_sheet.background_color = background_color
                .or(sheet_theme.modal_background_color)
                .or(sheet_theme.background_color)
                .or_else(|| defaults.background_color(app));
            modal_bottom_sheet.elevation = elevation
                .or(sheet_theme.modal_elevation)
                .or(sheet_theme.elevation)
                .or_else(|| defaults.modal_elevation(app));
            let route = app.get(self);
            modal_bottom_sheet.shape = route.shape.clone();
            modal_bottom_sheet.clip_behavior = route.clip_behavior;
            modal_bottom_sheet.constraints = route.constraints;
            modal_bottom_sheet.is_scroll_controlled = route.is_scroll_controlled;
            modal_bottom_sheet.scroll_control_disabled_max_height_ratio =
                route.scroll_control_disabled_max_height_ratio;
            modal_bottom_sheet.enable_drag = route.enable_drag;
            modal_bottom_sheet.show_drag_handle = route
                .show_drag_handle
                .unwrap_or(route.enable_drag && sheet_theme.show_drag_handle.unwrap_or(false));
            modal_bottom_sheet.into_widget()
        })
        .into_widget();

        let bottom_sheet = if app.get(self).use_safe_area {
            SafeArea::new(content).bottom(false).into_widget()
        } else {
            MediaQuery::remove_padding(app, context, false, true, false, false, content)
                .into_widget()
        };

        // Prevent clicks inside the bottom sheet from passing through to the barrier
        let bottom_sheet = Semantics::new()
            .hit_test_behavior(SemanticsHitTestBehavior::Opaque)
            .child(bottom_sheet)
            .into_widget();

        match &app.get(self).captured_themes {
            Some(captured_themes) => captured_themes.wrap(bottom_sheet),
            None => bottom_sheet,
        }
    }

    fn build_modal_barrier(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let _ = context;
        let barrier_color = ModalRoute::barrier_color(self, app).expect("always has a color");
        let dismissible = ModalRoute::barrier_dismissible(self, app);
        let semantics_label = ModalRoute::barrier_label(self, app);
        let semantics_dismissible = ModalRoute::semantics_dismissible(self, app);
        let clip_details_notifier = app.get(self).clip_details_notifier.clone();
        let barrier_on_tap_hint = app.get(self).barrier_on_tap_hint.clone();
        if barrier_color.a != 0.0 && !ModalRoute::offstage(self, app) {
            // changedInternalState is called if barrierColor or offstage updates
            let transparent = barrier_color.with_values(Some(0.0), None, None, None, None);
            debug_assert!(barrier_color != transparent);
            let animation = TransitionRoute::animation(self, app)
                .and_then(|animation| animation.upgrade(app))
                .expect("an installed route");
            // changedInternalState is called if barrierColor updates
            let color_tween = ColorTween::new(app, Some(transparent), Some(barrier_color));
            // changedInternalState is called if barrierCurve updates
            let curve = ModalRoute::barrier_curve(self, app);
            let curve_tween = CurveTween::new(app, curve);
            let color = animation.drive(app, color_tween.chain(curve_tween));
            let mut barrier = AnimatedModalBarrier::new(color)
                // changedInternalState is called if barrierDismissible updates
                .dismissible(dismissible)
                // changedInternalState is called if barrierLabel updates
                .maybe_semantics_label(semantics_label)
                .barrier_semantics_dismissible(semantics_dismissible)
                .clip_details_notifier(clip_details_notifier);
            barrier.semantics_on_tap_hint = barrier_on_tap_hint;
            barrier.into_widget()
        } else {
            let mut barrier = ModalBarrier::new()
                // changedInternalState is called if barrierDismissible updates
                .dismissible(dismissible)
                // changedInternalState is called if barrierLabel updates
                .maybe_semantics_label(semantics_label)
                .barrier_semantics_dismissible(semantics_dismissible)
                .clip_details_notifier(clip_details_notifier);
            barrier.semantics_on_tap_hint = barrier_on_tap_hint;
            barrier.into_widget()
        }
    }
}

impl PopupRoute for ModalBottomSheetRoute {}

/// Shows a modal Material Design bottom sheet.
///
/// A modal bottom sheet is an alternative to a menu or a dialog and prevents
/// the user from interacting with the rest of the app.
///
/// A closely related widget is a persistent bottom sheet, which shows
/// information that supplements the primary content of the app without
/// preventing the user from interacting with the app. Persistent bottom sheets
/// can be created and displayed with the [`show_bottom_sheet`] function or the
/// `ScaffoldState::show_bottom_sheet` method.
///
/// The `is_scroll_controlled` parameter specifies whether this is a route for
/// a bottom sheet that will utilize `DraggableScrollableSheet`. Consider
/// setting this parameter to true if this bottom sheet has
/// a scrollable child, such as a `ListView` or a `GridView`,
/// to have the bottom sheet be draggable.
///
/// The `is_dismissible` parameter specifies whether the bottom sheet will be
/// dismissed when user taps on the scrim.
///
/// The `enable_drag` parameter specifies whether the bottom sheet can be
/// dragged up and down and dismissed by swiping downwards.
///
/// The `use_safe_area` parameter specifies whether the sheet will avoid system
/// intrusions on the top, left, and right. If false, no [`SafeArea`] is added;
/// and [`MediaQuery::remove_padding`] is applied to the top,
/// so that system intrusions at the top will not be avoided by a [`SafeArea`]
/// inside the bottom sheet either.
/// Defaults to false.
///
/// The optional `background_color`, `elevation`, `shape`, `clip_behavior`,
/// `constraints` and `transition_animation_controller`
/// parameters can be passed in to customize the appearance and behavior of
/// modal bottom sheets (see the documentation for these on [`BottomSheet`]
/// for more details).
///
/// The `transition_animation_controller` controls the bottom sheet's entrance and
/// exit animations. It's up to the owner of the controller to call
/// [`AnimationController::dispose`] when the controller is no longer needed.
///
/// The optional `route_settings` parameter sets the `RouteSettings` of the modal bottom
/// sheet sheet. This is particularly useful in the case that a user wants to observe
/// [`PopupRoute`]s within a `NavigatorObserver`.
///
/// The `context` argument is used to look up the [`Navigator`] and [`Theme`] for
/// the bottom sheet. It is only used when the method is called. Its
/// corresponding widget can be safely removed from the tree before the bottom
/// sheet is closed.
///
/// The `use_root_navigator` parameter ensures that the root navigator is used to
/// display the [`BottomSheet`] when set to `true`. This is useful in the case
/// that a modal [`BottomSheet`] needs to be displayed above all other content
/// but the caller is inside another [`Navigator`].
///
/// Returns a `Future` that resolves to the value (if any) that was passed to
/// [`Navigator::pop`] when the modal bottom sheet was closed.
///
/// The 'barrier_label' parameter can be used to set a custom barrier label.
/// Will default to `MaterialLocalizations::modal_barrier_dismiss_label` of context
/// if not set.
///
/// The `sheet_animation_style` parameter is used to override the modal bottom sheet
/// animation duration and reverse animation duration.
///
/// The `request_focus` parameter is used to specify whether the bottom sheet should
/// request focus when shown.
///
/// If `AnimationStyle.duration` is provided, it will be used to override
/// the modal bottom sheet animation duration in the underlying
/// [`BottomSheet::create_animation_controller`].
///
/// If `AnimationStyle.reverseDuration` is provided, it will be used to
/// override the modal bottom sheet reverse animation duration in the
/// underlying [`BottomSheet::create_animation_controller`].
///
/// To disable the bottom sheet animation, use `AnimationStyle.noAnimation`.
///
/// See also:
///
///  * [`BottomSheet`], which becomes the parent of the widget returned by the
///    function passed as the `builder` argument to [`show_modal_bottom_sheet`].
///  * [`show_bottom_sheet`] and `ScaffoldState::show_bottom_sheet`, for showing
///    non-modal bottom sheets.
///  * `DraggableScrollableSheet`, creates a bottom sheet that grows
///    and then becomes scrollable once it reaches its maximum size.
///  * The Material 2 spec at <https://m2.material.io/components/sheets-bottom>.
///  * The Material 3 spec at <https://m3.material.io/components/bottom-sheets/overview>.
///  * [`AnimationStyle`], which is used to override the modal bottom sheet
///    animation duration and reverse animation duration.
#[expect(
    clippy::too_many_arguments,
    reason = "Dart's named arguments; a free function has no receiver for the fluent setters"
)]
pub fn show_modal_bottom_sheet(
    app: &mut App,
    context: BuildContext,
    builder: WidgetBuilder,
    background_color: Option<Color>,
    barrier_label: Option<String>,
    elevation: Option<f64>,
    shape: Option<ShapeBorderRef>,
    clip_behavior: Option<Clip>,
    constraints: Option<BoxConstraints>,
    barrier_color: Option<Color>,
    is_scroll_controlled: bool,
    scroll_control_disabled_max_height_ratio: f64,
    use_root_navigator: bool,
    is_dismissible: bool,
    enable_drag: bool,
    show_drag_handle: Option<bool>,
    use_safe_area: bool,
    route_settings: Option<RouteSettingsRef>,
    transition_animation_controller: Option<Ref<AnimationController>>,
    sheet_animation_style: Option<AnimationStyle>,
    request_focus: Option<bool>,
) -> CompleterFuture<RouteResult> {
    debug_assert!(debug_check_has_media_query(app, context));
    debug_assert!(debug_check_has_material_localizations(app, context));

    let navigator = Navigator::of(app, context, use_root_navigator);
    let localizations = <dyn MaterialLocalizations>::of(app, context);
    let navigator_context = navigator.context(app);
    let captured_themes = <dyn InheritedTheme>::capture(app, context, Some(navigator_context));
    let modal_barrier_color = barrier_color.or(Theme::of(app, context)
        .bottom_sheet_theme
        .modal_barrier_color);
    let route = ModalBottomSheetRoute::new(app, builder, is_scroll_controlled)
        .captured_themes(app, captured_themes)
        .scroll_control_disabled_max_height_ratio(app, scroll_control_disabled_max_height_ratio)
        .barrier_label(
            app,
            barrier_label.unwrap_or_else(|| localizations.scrim_label()),
        )
        .barrier_on_tap_hint(
            app,
            localizations.scrim_on_tap_hint(&localizations.bottom_sheet_label()),
        )
        .is_dismissible(app, is_dismissible)
        .enable_drag(app, enable_drag)
        .use_safe_area(app, use_safe_area);
    let mut route = route;
    let this = app.get_mut(&route);
    this.background_color = background_color;
    this.elevation = elevation;
    this.shape = shape;
    this.clip_behavior = clip_behavior;
    this.constraints = constraints;
    this.modal_barrier_color = modal_barrier_color;
    this.show_drag_handle = show_drag_handle;
    this.transition_animation_controller = transition_animation_controller;
    this.sheet_animation_style = sheet_animation_style;
    if let Some(route_settings) = route_settings {
        route = route.settings(app, route_settings);
    }
    if let Some(request_focus) = request_focus {
        route = route.request_focus(app, request_focus);
    }
    navigator.push(app, route.into_erased())
}

/// Shows a Material Design bottom sheet in the nearest [`Scaffold`] ancestor. To
/// show a persistent bottom sheet, use the `Scaffold::bottom_sheet`.
///
/// Returns a controller that can be used to close and otherwise manipulate the
/// bottom sheet.
///
/// The optional `background_color`, `elevation`, `shape`, `clip_behavior`,
/// `constraints` and `transition_animation_controller`
/// parameters can be passed in to customize the appearance and behavior of
/// persistent bottom sheets (see the documentation for these on [`BottomSheet`]
/// for more details).
///
/// The `enable_drag` parameter specifies whether the bottom sheet can be
/// dragged up and down and dismissed by swiping downwards.
///
/// The `sheet_animation_style` parameter is used to override the bottom sheet
/// animation duration and reverse animation duration.
///
/// If `AnimationStyle.duration` is provided, it will be used to override
/// the bottom sheet animation duration in the underlying
/// [`BottomSheet::create_animation_controller`].
///
/// If `AnimationStyle.reverseDuration` is provided, it will be used to
/// override the bottom sheet reverse animation duration in the underlying
/// [`BottomSheet::create_animation_controller`].
///
/// To disable the bottom sheet animation, use `AnimationStyle.noAnimation`.
///
/// To rebuild the bottom sheet (e.g. if it is stateful), call
/// `PersistentBottomSheetController::set_state` on the controller returned by
/// this method.
///
/// The new bottom sheet becomes a `LocalHistoryEntry` for the enclosing
/// [`ModalRoute`] and a back button is added to the app bar of the [`Scaffold`]
/// that closes the bottom sheet.
///
/// To create a persistent bottom sheet that is not a `LocalHistoryEntry` and
/// does not add a back button to the enclosing Scaffold's app bar, use the
/// `Scaffold::bottom_sheet` constructor parameter.
///
/// A closely related widget is a modal bottom sheet, which is an alternative
/// to a menu or a dialog and prevents the user from interacting with the rest
/// of the app. Modal bottom sheets can be created and displayed with the
/// [`show_modal_bottom_sheet`] function.
///
/// The `context` argument is used to look up the [`Scaffold`] for the bottom
/// sheet. It is only used when the method is called. Its corresponding widget
/// can be safely removed from the tree before the bottom sheet is closed.
///
/// See also:
///
///  * [`BottomSheet`], which becomes the parent of the widget returned by the
///    `builder`.
///  * [`show_modal_bottom_sheet`], which can be used to display a modal bottom
///    sheet.
///  * [`Scaffold::of`], for information about how to obtain the [`BuildContext`].
///  * The Material 2 spec at <https://m2.material.io/components/sheets-bottom>.
///  * The Material 3 spec at <https://m3.material.io/components/bottom-sheets/overview>.
///  * [`AnimationStyle`], which is used to override the bottom sheet animation
///    duration and reverse animation duration.
#[expect(
    clippy::too_many_arguments,
    reason = "Dart's named arguments; a free function has no receiver for the fluent setters"
)]
pub fn show_bottom_sheet(
    app: &mut App,
    context: BuildContext,
    builder: WidgetBuilder,
    background_color: Option<Color>,
    elevation: Option<f64>,
    shape: Option<ShapeBorderRef>,
    clip_behavior: Option<Clip>,
    constraints: Option<BoxConstraints>,
    enable_drag: Option<bool>,
    show_drag_handle: Option<bool>,
    transition_animation_controller: Option<Ref<AnimationController>>,
    sheet_animation_style: Option<AnimationStyle>,
) -> Rc<PersistentBottomSheetController> {
    debug_assert!(debug_check_has_scaffold(app, context));

    Scaffold::of(app, context).show_bottom_sheet(
        app,
        builder,
        background_color,
        elevation,
        shape,
        clip_behavior,
        constraints,
        enable_drag,
        show_drag_handle,
        transition_animation_controller,
        sheet_animation_style,
    )
}

/// Dart's `_BottomSheetGestureDetector`.
struct BottomSheetGestureDetector {
    child: WidgetRef,
    on_vertical_drag_start: GestureDragStartCallback,
    on_vertical_drag_update: GestureDragUpdateCallback,
    on_vertical_drag_end: GestureDragEndCallback,
}

impl Debug for BottomSheetGestureDetector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BottomSheetGestureDetector")
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for BottomSheetGestureDetector {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let on_vertical_drag_start = Rc::clone(&self.on_vertical_drag_start);
        let on_vertical_drag_update = Rc::clone(&self.on_vertical_drag_update);
        let on_vertical_drag_end = Rc::clone(&self.on_vertical_drag_end);
        RawGestureDetector::new()
            .gestures(vec![(
                TypeId::of::<VerticalDragGestureRecognizer>(),
                GestureRecognizerFactoryWithHandlers::<VerticalDragGestureRecognizer>::new(
                    VerticalDragGestureRecognizer::new,
                    move |app, instance| {
                        instance.set_on_start(app, Some(Rc::clone(&on_vertical_drag_start)));
                        instance.set_on_update(app, Some(Rc::clone(&on_vertical_drag_update)));
                        instance.set_on_end(app, Some(Rc::clone(&on_vertical_drag_end)));
                        instance.set_only_accept_drag_on_threshold(app, true);
                    },
                )
                .into_factory(),
            )])
            .child(self.child.clone())
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use waft_painting::ShapeBorder;

    use waft_embedder::{Rect, TargetPlatform};
    use waft_widgets::{AnyElement, Notification, RouteSettings};

    use super::*;
    use crate::scaffold::ScaffoldState;
    use crate::test_support::{AhemText, WidgetTester, material_app, with_main_thread_stack};
    use crate::theme_data::ThemeData;
    use waft_test::{assert_no_new_objects, object_snapshot};

    fn tester() -> WidgetTester {
        let tester = WidgetTester::on(TargetPlatform::Android);
        tester.install_fonts();
        tester
    }

    fn m3(tester: &WidgetTester) -> ThemeData {
        ThemeData::new(&tester.cell.borrow())
    }

    fn m2(tester: &WidgetTester) -> ThemeData {
        ThemeData::builder()
            .use_material3(false)
            .build(&tester.cell.borrow())
    }

    /// Dart's `const Text('BottomSheet')` as the tests lay it out in Ahem.
    fn text(text: &'static str) -> WidgetRef {
        AhemText::new(text).into_widget()
    }

    fn sheet_builder(content: &'static str) -> WidgetBuilder {
        Rc::new(move |_app, _context| text(content))
    }

    /// A slot a builder writes its context into: Dart's `late BuildContext savedContext`.
    #[derive(Clone, Default)]
    struct Saved(Rc<Cell<Option<BuildContext>>>);

    impl Saved {
        fn get(&self) -> BuildContext {
            self.0.get().expect("the builder ran")
        }

        /// Dart's `Builder(builder: (context) { saved = context; return child; })`.
        fn builder(&self, child: impl Fn() -> WidgetRef + 'static) -> WidgetRef {
            let slot = Rc::clone(&self.0);
            Builder::new(move |_app, context| {
                slot.set(Some(context));
                child()
            })
            .into_widget()
        }
    }

    /// `MaterialApp(home: Builder(builder: (context) { savedContext = context; return Container(); }))`.
    fn app_saving_context(tester: &WidgetTester, saved: &Saved) -> WidgetRef {
        material_app(m3(tester), saved.builder(|| Container::new().into_widget()))
    }

    /// `MaterialApp(home: Scaffold(key: scaffoldKey, body: const Center(child: Text('body'))))`.
    fn app_with_scaffold(theme: ThemeData, scaffold_key: &GlobalKey) -> WidgetRef {
        material_app(
            theme,
            Scaffold::new()
                .key(scaffold_key.clone().into())
                .body(Center::new().child(text("body")))
                .into_widget(),
        )
    }

    /// `showModalBottomSheet`'s arguments with Dart's defaults.
    struct Modal {
        background_color: Option<Color>,
        barrier_label: Option<String>,
        elevation: Option<f64>,
        shape: Option<ShapeBorderRef>,
        clip_behavior: Option<Clip>,
        constraints: Option<BoxConstraints>,
        barrier_color: Option<Color>,
        is_scroll_controlled: bool,
        scroll_control_disabled_max_height_ratio: f64,
        use_root_navigator: bool,
        is_dismissible: bool,
        enable_drag: bool,
        show_drag_handle: Option<bool>,
        use_safe_area: bool,
        route_settings: Option<RouteSettingsRef>,
        transition_animation_controller: Option<Ref<AnimationController>>,
        sheet_animation_style: Option<AnimationStyle>,
        request_focus: Option<bool>,
    }

    impl Default for Modal {
        fn default() -> Modal {
            Modal {
                background_color: None,
                barrier_label: None,
                elevation: None,
                shape: None,
                clip_behavior: None,
                constraints: None,
                barrier_color: None,
                is_scroll_controlled: false,
                scroll_control_disabled_max_height_ratio:
                    K_DEFAULT_SCROLL_CONTROL_DISABLED_MAX_HEIGHT_RATIO,
                use_root_navigator: false,
                is_dismissible: true,
                enable_drag: true,
                show_drag_handle: None,
                use_safe_area: false,
                route_settings: None,
                transition_animation_controller: None,
                sheet_animation_style: None,
                request_focus: None,
            }
        }
    }

    impl Modal {
        fn show(
            self,
            tester: &WidgetTester,
            context: BuildContext,
            builder: WidgetBuilder,
        ) -> CompleterFuture<RouteResult> {
            self.show_in(&mut tester.cell.borrow_mut(), context, builder)
        }

        fn show_in(
            self,
            app: &mut App,
            context: BuildContext,
            builder: WidgetBuilder,
        ) -> CompleterFuture<RouteResult> {
            show_modal_bottom_sheet(
                app,
                context,
                builder,
                self.background_color,
                self.barrier_label,
                self.elevation,
                self.shape,
                self.clip_behavior,
                self.constraints,
                self.barrier_color,
                self.is_scroll_controlled,
                self.scroll_control_disabled_max_height_ratio,
                self.use_root_navigator,
                self.is_dismissible,
                self.enable_drag,
                self.show_drag_handle,
                self.use_safe_area,
                self.route_settings,
                self.transition_animation_controller,
                self.sheet_animation_style,
                self.request_focus,
            )
        }
    }

    /// `ScaffoldState.showBottomSheet`'s optional arguments with Dart's defaults.
    #[derive(Default)]
    struct Persistent {
        constraints: Option<BoxConstraints>,
        enable_drag: Option<bool>,
        show_drag_handle: Option<bool>,
        transition_animation_controller: Option<Ref<AnimationController>>,
        sheet_animation_style: Option<AnimationStyle>,
    }

    impl Persistent {
        fn show(
            self,
            tester: &WidgetTester,
            scaffold: Handle<ScaffoldState>,
            builder: WidgetBuilder,
        ) -> Rc<PersistentBottomSheetController> {
            let mut app = tester.cell.borrow_mut();
            scaffold.show_bottom_sheet(
                &mut app,
                builder,
                None,
                None,
                None,
                None,
                self.constraints,
                self.enable_drag,
                self.show_drag_handle,
                self.transition_animation_controller,
                self.sheet_animation_style,
            )
        }
    }

    fn scaffold_state(tester: &WidgetTester, key: &GlobalKey) -> Handle<ScaffoldState> {
        key.current_state::<ScaffoldState>(&tester.cell.borrow_mut())
            .expect("the scaffold is mounted")
    }

    /// Sets the flag when `future` completes: Dart's `.then((_) { called = true; })`.
    fn flag_when_complete<T: Clone + 'static>(
        tester: &WidgetTester,
        future: CompleterFuture<T>,
    ) -> Rc<Cell<bool>> {
        let called = Rc::new(Cell::new(false));
        let flag = Rc::clone(&called);
        future.then(&mut tester.cell.borrow_mut(), move |_app, _| flag.set(true));
        called
    }

    fn found(tester: &WidgetTester, content: &str) -> usize {
        tester.find_ahem_text(content).len()
    }

    fn only(found: Vec<AnyElement>) -> AnyElement {
        assert_eq!(found.len(), 1, "one widget found");
        found[0]
    }

    fn rect_of_text(tester: &WidgetTester, content: &str) -> Rect {
        tester.global_rect(tester.render_box_of(only(tester.find_ahem_text(content))))
    }

    fn center_of_text(tester: &WidgetTester, content: &str) -> Offset {
        rect_of_text(tester, content).center()
    }

    /// Dart's `tester.drag(find.text(content), offset)`.
    fn drag_text(tester: &mut WidgetTester, content: &str, offset: Offset) {
        let start = center_of_text(tester, content);
        tester.drag_from(start, offset, 1);
    }

    /// Dart's `tester.fling(find.text(content), offset, speed)`.
    fn fling_text(tester: &mut WidgetTester, content: &str, offset: Offset, speed: f64) {
        let start = center_of_text(tester, content);
        tester.fling_from(start, offset, speed, 1);
    }

    /// Pumps and ensures that the BottomSheet animates non-linearly; Dart's
    /// `checkNonLinearAnimation`.
    fn check_non_linear_animation(tester: &mut WidgetTester) {
        let first_position = center_of_text(tester, "BottomSheet");
        tester.pump(Duration::from_millis(30));
        let second_position = center_of_text(tester, "BottomSheet");
        tester.pump(Duration::from_millis(30));
        let third_position = center_of_text(tester, "BottomSheet");

        let dy_delta1 = second_position.dy() - first_position.dy();
        let dy_delta2 = third_position.dy() - second_position.dy();

        // If the animation were linear, these two values would be the same.
        assert!((dy_delta1 - dy_delta2).abs() > 0.1);
    }

    /// Dart's 'Swiping down a BottomSheet should dismiss it by default', 'when enableDrag is
    /// false' and 'when enableDrag is true'.
    fn swipe_persistent(enable_drag: Option<bool>, dismissed: bool) {
        let mut tester = tester();
        let scaffold_key = GlobalKey::new();
        tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));
        tester.pump(Duration::ZERO);
        assert_eq!(found(&tester, "BottomSheet"), 0);

        let scaffold = scaffold_state(&tester, &scaffold_key);
        let controller = Persistent {
            enable_drag,
            ..Persistent::default()
        }
        .show(
            &tester,
            scaffold,
            Rc::new(|_app, _context| {
                SizedBox::new()
                    .height(200.0)
                    .child(text("BottomSheet"))
                    .into_widget()
            }),
        );
        let then_called = flag_when_complete(&tester, controller.closed());

        tester.pump_and_settle();
        assert!(!then_called.get());
        assert_eq!(found(&tester, "BottomSheet"), 1);

        // Swipe the bottom sheet to dismiss it.
        drag_text(&mut tester, "BottomSheet", Offset::new(0.0, 150.0));
        tester.pump_and_settle(); // Bottom sheet dismiss animation.
        assert_eq!(then_called.get(), dismissed);
        assert_eq!(found(&tester, "BottomSheet"), if dismissed { 0 } else { 1 });
    }

    #[test]
    fn swiping_down_a_bottom_sheet_dismisses_it_by_default() {
        with_main_thread_stack(|| swipe_persistent(None, true));
    }

    #[test]
    fn swiping_down_a_bottom_sheet_does_not_dismiss_it_when_enable_drag_is_false() {
        with_main_thread_stack(|| swipe_persistent(Some(false), false));
    }

    #[test]
    fn swiping_down_a_bottom_sheet_dismisses_it_when_enable_drag_is_true() {
        with_main_thread_stack(|| swipe_persistent(Some(true), true));
    }

    /// Dart's 'Tapping on a BottomSheet should not trigger a rebuild when enableDrag is true'.
    #[test]
    fn tapping_a_draggable_bottom_sheet_does_not_rebuild_it() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));
            tester.pump(Duration::ZERO);
            let build_count = Rc::new(Cell::new(0));
            assert_eq!(build_count.get(), 0);
            assert_eq!(found(&tester, "BottomSheet"), 0);

            let scaffold = scaffold_state(&tester, &scaffold_key);
            let count = Rc::clone(&build_count);
            Persistent {
                enable_drag: Some(true),
                ..Persistent::default()
            }
            .show(
                &tester,
                scaffold,
                Rc::new(move |_app, _context| {
                    count.set(count.get() + 1);
                    SizedBox::new()
                        .height(200.0)
                        .child(text("BottomSheet"))
                        .into_widget()
                }),
            );

            tester.pump_and_settle();
            assert_eq!(build_count.get(), 1);
            assert_eq!(found(&tester, "BottomSheet"), 1);

            // Tap on bottom sheet should not trigger a rebuild.
            let center = center_of_text(&tester, "BottomSheet");
            tester.tap(center, 1);
            tester.pump_and_settle();
            assert_eq!(build_count.get(), 1);
            assert_eq!(found(&tester, "BottomSheet"), 1);
        });
    }

    /// Dart's first 'Modal BottomSheet builder should only be called once'.
    #[test]
    fn a_modal_bottom_sheet_builder_is_called_once() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved = Saved::default();
            tester.pump_widget(app_saving_context(&tester, &saved));

            let num_builder_calls = Rc::new(Cell::new(0));
            let calls = Rc::clone(&num_builder_calls);
            Modal {
                is_dismissible: false,
                ..Modal::default()
            }
            .show(
                &tester,
                saved.get(),
                Rc::new(move |_app, _context| {
                    calls.set(calls.get() + 1);
                    text("BottomSheet")
                }),
            );

            tester.pump_and_settle();
            assert_eq!(num_builder_calls.get(), 1);

            // Swipe the bottom sheet to dismiss it.
            drag_text(&mut tester, "BottomSheet", Offset::new(0.0, 150.0));
            tester.pump_and_settle(); // Bottom sheet dismiss animation.
            assert_eq!(num_builder_calls.get(), 1);
        });
    }

    /// Dart's 'Tapping on a modal BottomSheet should not dismiss it'.
    #[test]
    fn tapping_on_a_modal_bottom_sheet_does_not_dismiss_it() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved = Saved::default();
            tester.pump_widget(app_saving_context(&tester, &saved));
            tester.pump(Duration::ZERO);
            assert_eq!(found(&tester, "BottomSheet"), 0);

            let future = Modal::default().show(&tester, saved.get(), sheet_builder("BottomSheet"));
            let then_called = flag_when_complete(&tester, future);

            tester.pump_and_settle();
            assert_eq!(found(&tester, "BottomSheet"), 1);
            assert!(!then_called.get());

            // Tap on the bottom sheet itself, it should not be dismissed
            let center = center_of_text(&tester, "BottomSheet");
            tester.tap(center, 1);
            tester.pump_and_settle();
            assert_eq!(found(&tester, "BottomSheet"), 1);
            assert!(!then_called.get());
        });
    }

    /// Dart's 'Tapping outside a modal BottomSheet should dismiss it by default', 'when
    /// isDismissible=true' and 'should not dismiss it when isDismissible=false'.
    fn tap_outside_modal(is_dismissible: bool) {
        let mut tester = tester();
        let saved = Saved::default();
        tester.pump_widget(app_saving_context(&tester, &saved));
        tester.pump(Duration::ZERO);
        assert_eq!(found(&tester, "BottomSheet"), 0);

        let future = Modal {
            is_dismissible,
            ..Modal::default()
        }
        .show(&tester, saved.get(), sheet_builder("BottomSheet"));
        let then_called = flag_when_complete(&tester, future);

        tester.pump_and_settle();
        assert_eq!(found(&tester, "BottomSheet"), 1);
        assert!(!then_called.get());

        // Tap above the bottom sheet to dismiss it.
        tester.tap(Offset::new(20.0, 20.0), 1);
        tester.pump_and_settle(); // Bottom sheet dismiss animation.
        assert_eq!(then_called.get(), is_dismissible);
        assert_eq!(
            found(&tester, "BottomSheet"),
            if is_dismissible { 0 } else { 1 }
        );
    }

    #[test]
    fn tapping_outside_a_modal_bottom_sheet_dismisses_it_by_default() {
        with_main_thread_stack(|| tap_outside_modal(Modal::default().is_dismissible));
    }

    #[test]
    fn tapping_outside_a_modal_bottom_sheet_dismisses_it_when_is_dismissible_is_true() {
        with_main_thread_stack(|| tap_outside_modal(true));
    }

    #[test]
    fn tapping_outside_a_modal_bottom_sheet_does_not_dismiss_it_when_is_dismissible_is_false() {
        with_main_thread_stack(|| tap_outside_modal(false));
    }

    /// Dart's 'Verify that the BottomSheet animates non-linearly'.
    #[test]
    fn the_bottom_sheet_animates_non_linearly() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved = Saved::default();
            tester.pump_widget(app_saving_context(&tester, &saved));
            tester.pump(Duration::ZERO);
            assert_eq!(found(&tester, "BottomSheet"), 0);

            Modal::default().show(&tester, saved.get(), sheet_builder("BottomSheet"));
            tester.pump(Duration::ZERO);

            check_non_linear_animation(&mut tester);
            tester.pump_and_settle();

            // Tap above the bottom sheet to dismiss it.
            tester.tap(Offset::new(20.0, 20.0), 1);
            tester.pump(Duration::ZERO);
            check_non_linear_animation(&mut tester);
            tester.pump_and_settle(); // Bottom sheet dismiss animation.
            assert_eq!(found(&tester, "BottomSheet"), 0);
        });
    }

    /// Dart's 'Swiping down a modal BottomSheet should dismiss it by default', 'should not
    /// dismiss it when enableDrag is false' and 'should dismiss it when enableDrag is true'.
    fn swipe_modal(enable_drag: bool, dismissed: bool) {
        let mut tester = tester();
        let saved = Saved::default();
        tester.pump_widget(app_saving_context(&tester, &saved));
        tester.pump(Duration::ZERO);
        assert_eq!(found(&tester, "BottomSheet"), 0);

        let future = Modal {
            is_dismissible: false,
            enable_drag,
            ..Modal::default()
        }
        .show(&tester, saved.get(), sheet_builder("BottomSheet"));
        let then_called = flag_when_complete(&tester, future);

        tester.pump_and_settle();
        assert_eq!(found(&tester, "BottomSheet"), 1);
        assert!(!then_called.get());

        // Swipe the bottom sheet to dismiss it.
        drag_text(&mut tester, "BottomSheet", Offset::new(0.0, 150.0));
        tester.pump_and_settle(); // Bottom sheet dismiss animation.
        assert_eq!(then_called.get(), dismissed);
        assert_eq!(found(&tester, "BottomSheet"), if dismissed { 0 } else { 1 });
    }

    #[test]
    fn swiping_down_a_modal_bottom_sheet_dismisses_it_by_default() {
        with_main_thread_stack(|| swipe_modal(Modal::default().enable_drag, true));
    }

    #[test]
    fn swiping_down_a_modal_bottom_sheet_does_not_dismiss_it_when_enable_drag_is_false() {
        with_main_thread_stack(|| swipe_modal(false, false));
    }

    #[test]
    fn swiping_down_a_modal_bottom_sheet_dismisses_it_when_enable_drag_is_true() {
        with_main_thread_stack(|| swipe_modal(true, true));
    }

    /// Dart's 'Verify that a downwards fling dismisses a persistent BottomSheet'.
    #[test]
    fn a_downwards_fling_dismisses_a_persistent_bottom_sheet() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));
            assert_eq!(found(&tester, "BottomSheet"), 0);

            let scaffold = scaffold_state(&tester, &scaffold_key);
            let controller = Persistent::default().show(
                &tester,
                scaffold,
                Rc::new(|_app, _context| {
                    Container::new()
                        .margin(EdgeInsetsGeometry::all(40.0))
                        .child(text("BottomSheet"))
                        .into_widget()
                }),
            );
            let then_called = flag_when_complete(&tester, controller.closed());

            assert!(!then_called.get());
            assert_eq!(found(&tester, "BottomSheet"), 0);

            tester.pump(Duration::ZERO); // bottom sheet show animation starts

            assert!(!then_called.get());
            assert_eq!(found(&tester, "BottomSheet"), 1);

            tester.pump(Duration::from_secs(1)); // animation done

            assert!(!then_called.get());
            assert_eq!(found(&tester, "BottomSheet"), 1);

            // The fling below must be such that the velocity estimation examines an
            // offset greater than the kTouchSlop. Too slow or too short a distance, and
            // it won't trigger. Also, it must not be so much that it drags the bottom
            // sheet off the screen, or we won't see it after we pump!
            fling_text(&mut tester, "BottomSheet", Offset::new(0.0, 50.0), 2000.0);
            tester.pump(Duration::ZERO); // drain the microtask queue (Future completion callback)

            assert!(then_called.get());
            assert_eq!(found(&tester, "BottomSheet"), 1);

            tester.pump(Duration::ZERO); // bottom sheet dismiss animation starts

            assert!(then_called.get());
            assert_eq!(found(&tester, "BottomSheet"), 1);

            tester.pump(Duration::from_secs(1)); // animation done

            assert!(then_called.get());
            assert_eq!(found(&tester, "BottomSheet"), 0);
        });
    }

    /// Dart's 'Verify that dragging past the bottom dismisses a persistent BottomSheet'.
    #[test]
    fn dragging_past_the_bottom_dismisses_a_persistent_bottom_sheet() {
        with_main_thread_stack(|| {
            // This is a regression test for https://github.com/flutter/flutter/issues/5528
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));

            let scaffold = scaffold_state(&tester, &scaffold_key);
            Persistent::default().show(
                &tester,
                scaffold,
                Rc::new(|_app, _context| {
                    Container::new()
                        .margin(EdgeInsetsGeometry::all(40.0))
                        .child(text("BottomSheet"))
                        .into_widget()
                }),
            );

            tester.pump(Duration::ZERO); // bottom sheet show animation starts
            tester.pump(Duration::from_secs(1)); // animation done
            assert_eq!(found(&tester, "BottomSheet"), 1);

            fling_text(&mut tester, "BottomSheet", Offset::new(0.0, 400.0), 1000.0);
            tester.pump(Duration::ZERO); // drain the microtask queue (Future completion callback)
            tester.pump(Duration::ZERO); // bottom sheet dismiss animation starts
            tester.pump(Duration::from_secs(1)); // animation done

            assert_eq!(found(&tester, "BottomSheet"), 0);
        });
    }

    /// Dispatches Dart's `DraggableScrollableNotification(extent: 0.20000000001, minExtent:
    /// 0.2, maxExtent: 1.0, initialExtent: 0.5, context: notificationContext)` from `context`.
    fn dispatch_nearly_minimum_extent(tester: &WidgetTester, context: BuildContext) {
        let mut app = tester.cell.borrow_mut();
        DraggableScrollableNotification::new(0.20000000001, 0.2, 1.0, 0.5, context)
            .dispatch(&mut app, Some(context));
    }

    /// Dart's 'Standard BottomSheet closes when its extent is nearly its minimum'.
    #[test]
    fn a_standard_bottom_sheet_closes_when_its_extent_is_nearly_its_minimum() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));

            let notification_context = Saved::default();
            let saving = notification_context.clone();
            let scaffold = scaffold_state(&tester, &scaffold_key);
            Persistent::default().show(
                &tester,
                scaffold,
                Rc::new(move |_app, _context| saving.builder(|| text("BottomSheet"))),
            );
            tester.pump_and_settle();

            dispatch_nearly_minimum_extent(&tester, notification_context.get());
            tester.pump_and_settle();

            assert_eq!(found(&tester, "BottomSheet"), 0);
        });
    }

    /// Dart's 'Modal BottomSheet closes when its extent is nearly its minimum'.
    #[test]
    fn a_modal_bottom_sheet_closes_when_its_extent_is_nearly_its_minimum() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));

            let notification_context = Saved::default();
            let saving = notification_context.clone();
            let context = scaffold_key
                .current_context(&tester.cell.borrow_mut())
                .expect("the scaffold is mounted");
            Modal::default().show(
                &tester,
                context,
                Rc::new(move |_app, _context| saving.builder(|| text("BottomSheet"))),
            );
            tester.pump_and_settle();

            dispatch_nearly_minimum_extent(&tester, notification_context.get());
            tester.pump_and_settle();

            assert_eq!(found(&tester, "BottomSheet"), 0);
        });
    }

    /// flutter_test's `TestVSync`: vends ordinary tickers, in no view.
    struct TestVSync;

    impl waft_scheduler::TickerProvider for TestVSync {
        fn create_ticker(
            &self,
            app: &mut App,
            on_tick: waft_scheduler::TickerCallback,
        ) -> Ref<waft_scheduler::Ticker> {
            waft_scheduler::Ticker::new(app, on_tick, None)
        }
    }

    /// `AnimationController(vsync: const TestVSync(), duration: 2s, reverseDuration: 2s)`.
    fn two_second_controller(tester: &WidgetTester) -> Ref<AnimationController> {
        AnimationController::new(
            &mut tester.cell.borrow_mut(),
            None,
            Some(Duration::from_secs(2)),
            Some(Duration::from_secs(2)),
            0.0,
            1.0,
            AnimationBehavior::Normal,
            TestVSync,
        )
    }

    fn rect_of_key(tester: &WidgetTester, key: &GlobalKey) -> Rect {
        let render = tester.render_object(key).as_box().expect("a render box");
        tester.global_rect(render)
    }

    fn mounted(tester: &WidgetTester, key: &GlobalKey) -> bool {
        key.current_context(&tester.cell.borrow_mut()).is_some()
    }

    fn pop(tester: &WidgetTester, context: BuildContext) {
        Navigator::pop(&mut tester.cell.borrow_mut(), context, None);
    }

    /// `MaterialApp(home: Scaffold(key:, body: Center(child: Text('body'))))`'s context.
    fn scaffold_context(tester: &WidgetTester, key: &GlobalKey) -> BuildContext {
        key.current_context(&tester.cell.borrow_mut())
            .expect("the scaffold is mounted")
    }

    /// Dart's 'Disposing app while bottom sheet is disappearing does not crash'.
    #[test]
    fn disposing_the_app_while_a_bottom_sheet_is_disappearing_does_not_crash() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved = Saved::default();
            tester.pump_widget(app_saving_context(&tester, &saved));

            tester.pump(Duration::ZERO);
            assert_eq!(found(&tester, "BottomSheet"), 0);

            // Bring up bottom sheet.
            let future = Modal::default().show(&tester, saved.get(), sheet_builder("BottomSheet"));
            let then_called = flag_when_complete(&tester, future);
            tester.pump_and_settle();
            assert_eq!(found(&tester, "BottomSheet"), 1);
            assert!(!then_called.get());

            // Start closing animation of Bottom sheet.
            pop(&tester, saved.get());
            tester.pump(Duration::ZERO);

            // Dispose app by replacing it with a container. This shouldn't crash.
            tester.pump_widget(Container::new().into_widget());
        });
    }

    /// Dart's 'Verify that accessibleNavigation has no impact on the BottomSheet animation'.
    #[test]
    fn accessible_navigation_has_no_impact_on_the_bottom_sheet_animation() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/121098
            let mut tester = tester();
            let saved = Saved::default();
            let home = saved.builder(|| Center::new().child(text("Test")).into_widget());
            let app = material_app(m3(&tester), home);
            // Dart's `MaterialApp(builder:)` wraps the navigator; a `MediaQuery` around the
            // home is what that builder's descendants see.
            let _ = app;
            let home = Builder::new({
                let saved = saved.clone();
                move |app, context| {
                    let data = MediaQuery::of(app, context)
                        .copy_with()
                        .accessible_navigation(true);
                    MediaQuery::new(
                        data,
                        saved.builder(|| Center::new().child(text("Test")).into_widget()),
                    )
                    .into_widget()
                }
            })
            .into_widget();
            tester.pump_widget(material_app(m3(&tester), home));

            tester.pump(Duration::ZERO);
            assert_eq!(found(&tester, "BottomSheet"), 0);

            Modal::default().show(&tester, saved.get(), sheet_builder("BottomSheet"));
            tester.pump(Duration::ZERO);

            check_non_linear_animation(&mut tester);
            tester.pump_and_settle();
        });
    }

    /// Dart's tree for 'modal BottomSheet has no top MediaQuery' and 'can insert a SafeArea':
    /// a navigator under a `MediaQuery` with 50 pixels of padding all round, its page saving
    /// its context.
    fn padded_navigator(outer: &Saved) -> WidgetRef {
        let outer = outer.clone();
        waft_widgets::Localizations::new(
            waft_embedder::Locale::new("en").country_code("US"),
            vec![
                waft_widgets::DefaultWidgetsLocalizations::delegate(),
                crate::material_localizations::DefaultMaterialLocalizations::delegate(),
            ],
        )
        .child(waft_widgets::Directionality::new(
            waft_embedder::TextDirection::Ltr,
            MediaQuery::new(
                waft_widgets::MediaQueryData::new()
                    .padding(EdgeInsets::all(50.0))
                    .size(Size::new(400.0, 600.0)),
                Navigator::new().on_generate_route(move |app, _settings| {
                    let outer = outer.clone();
                    let page_builder: waft_widgets::RoutePageBuilder =
                        Rc::new(move |_app, context, _animation, _secondary_animation| {
                            outer.0.set(Some(context));
                            Container::new().into_widget()
                        });
                    Some((waft_widgets::PageRouteBuilder::new(app, page_builder)).into_erased())
                }),
            )
            .into_widget(),
        ))
        .into_widget()
    }

    fn padding_of(tester: &WidgetTester, context: BuildContext) -> EdgeInsets {
        MediaQuery::of(&mut tester.cell.borrow_mut(), context).padding
    }

    /// Dart's 'modal BottomSheet has no top MediaQuery'.
    #[test]
    fn a_modal_bottom_sheet_has_no_top_media_query_padding() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let outer = Saved::default();
            let inner = Saved::default();
            tester.pump_widget(padded_navigator(&outer));

            let saving = inner.clone();
            Modal::default().show(
                &tester,
                outer.get(),
                Rc::new(move |_app, context| {
                    saving.0.set(Some(context));
                    Container::new().into_widget()
                }),
            );
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_secs(1));

            assert_eq!(padding_of(&tester, outer.get()), EdgeInsets::all(50.0));
            assert_eq!(
                padding_of(&tester, inner.get()),
                EdgeInsets::only(50.0, 0.0, 50.0, 50.0)
            );
        });
    }

    /// Dart's 'modal BottomSheet can insert a SafeArea'.
    #[test]
    fn a_modal_bottom_sheet_can_insert_a_safe_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let outer = Saved::default();
            let inner = Saved::default();
            tester.pump_widget(padded_navigator(&outer));

            let inner_builder = |inner: &Saved| -> WidgetBuilder {
                let saving = inner.clone();
                Rc::new(move |_app, context| {
                    saving.0.set(Some(context));
                    Container::new().into_widget()
                })
            };

            // Without a SafeArea (useSafeArea is false by default)
            Modal::default().show(&tester, outer.get(), inner_builder(&inner));
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_secs(1));

            // Top padding is consumed and there is no SafeArea
            assert_eq!(padding_of(&tester, inner.get()).top, 0.0);
            assert!(tester.find_by_type::<SafeArea>().is_empty());

            // With a SafeArea
            Modal {
                use_safe_area: true,
                ..Modal::default()
            }
            .show(&tester, outer.get(), inner_builder(&inner));
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_secs(1));

            // A SafeArea is inserted, with left / top / right true but bottom false.
            let safe_area = only(tester.find_by_type::<SafeArea>());
            tester.widget(safe_area, |safe_area: &SafeArea| {
                assert!(safe_area.left);
                assert!(safe_area.top);
                assert!(safe_area.right);
                assert!(!safe_area.bottom);
            });

            // Because that SafeArea is inserted, no left / top / right padding remains
            // for `builder` to consume. Bottom padding does remain.
            assert_eq!(
                padding_of(&tester, inner.get()),
                EdgeInsets::from_ltrb(0.0, 0.0, 0.0, 50.0)
            );
        });
    }

    /// Dart's 'Verify that visual properties are passed through'.
    #[test]
    fn visual_properties_are_passed_through_to_the_modal_bottom_sheet() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            let color = Colors::PINK.primary();
            let elevation = 9.0;
            let shape: ShapeBorderRef =
                ShapeBorderRef::new(waft_painting::BeveledRectangleBorder::new(
                    waft_painting::BorderSide::NONE,
                    waft_painting::BorderRadiusGeometry::all(waft_embedder::Radius::circular(12.0)),
                ));
            let clip_behavior = Clip::AntiAlias;
            let barrier_color = Colors::RED.primary();

            tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));

            Modal {
                background_color: Some(color),
                barrier_color: Some(barrier_color),
                elevation: Some(elevation),
                shape: Some(shape.clone()),
                clip_behavior: Some(clip_behavior),
                ..Modal::default()
            }
            .show(
                &tester,
                scaffold_context(&tester, &scaffold_key),
                sheet_builder("BottomSheet"),
            );

            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_secs(1));

            let bottom_sheet = only(tester.find_by_type::<BottomSheet>());
            tester.widget(bottom_sheet, |bottom_sheet: &BottomSheet| {
                assert_eq!(bottom_sheet.background_color, Some(color));
                assert_eq!(bottom_sheet.elevation, Some(elevation));
                assert!(**bottom_sheet.shape.as_ref().expect("a shape") == *shape);
                assert_eq!(bottom_sheet.clip_behavior, Some(clip_behavior));
            });

            let modal_barrier = *tester
                .find_by_type::<ModalBarrier>()
                .last()
                .expect("a modal barrier");
            tester.widget(modal_barrier, |modal_barrier: &ModalBarrier| {
                assert_eq!(modal_barrier.color, Some(barrier_color));
            });
        });
    }

    /// The `Material` under the only `BottomSheet`: Dart's `find.descendant(of:
    /// find.byType(BottomSheet), matching: find.byType(Material))`.
    fn sheet_material(tester: &WidgetTester) -> AnyElement {
        let sheet = only(tester.find_by_type::<BottomSheet>());
        only(
            tester
                .find_by_type::<Material>()
                .into_iter()
                .filter(|material| tester.ancestors(*material).contains(&sheet))
                .collect(),
        )
    }

    /// Dart's 'Material3 - BottomSheet uses fallback values'.
    #[test]
    fn material3_bottom_sheet_uses_fallback_values() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let surface_color = Colors::PINK.primary();
            let surface_tint_color = Colors::BLUE.primary();
            let default_shape: ShapeBorderRef =
                ShapeBorderRef::new(waft_painting::RoundedRectangleBorder::new(
                    waft_painting::BorderSide::NONE,
                    waft_painting::BorderRadiusGeometry::vertical(
                        waft_embedder::Radius::circular(28.0),
                        waft_embedder::Radius::ZERO,
                    ),
                ));

            let theme = ThemeData::builder()
                .color_scheme(
                    crate::color_scheme::ColorScheme::light()
                        .with_surface(surface_color)
                        .with_surface_tint(surface_tint_color),
                )
                .build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Scaffold::new()
                    .body(BottomSheet::new(
                        Rc::new(|_app| {}),
                        Rc::new(|_app, _context| Container::new().into_widget()),
                    ))
                    .into_widget(),
            ));

            let material = sheet_material(&tester);
            tester.widget(material, |material: &Material| {
                assert_eq!(material.color, Some(surface_color));
                // Surface tint is no longer used by default.
                assert_eq!(material.surface_tint_color, Some(Colors::TRANSPARENT));
                assert_eq!(material.elevation, 1.0);
                assert!(**material.shape.as_ref().expect("a shape") == *default_shape);
            });
            assert_eq!(
                tester.global_rect(tester.render_box_of(material)).width(),
                640.0
            );
        });
    }

    /// Dart's 'Material3 - BottomSheet has transparent shadow'.
    #[test]
    fn material3_bottom_sheet_has_a_transparent_shadow() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(material_app(
                m3(&tester),
                Scaffold::new()
                    .body(BottomSheet::new(
                        Rc::new(|_app| {}),
                        Rc::new(|_app, _context| Container::new().into_widget()),
                    ))
                    .into_widget(),
            ));

            let material = sheet_material(&tester);
            tester.widget(material, |material: &Material| {
                assert_eq!(material.shadow_color, Some(Colors::TRANSPARENT));
            });
        });
    }

    /// The box the only `_DragHandle` lays out: Dart's `find.bySemanticsLabel('Dismiss')`,
    /// whose `Semantics` wraps the handle's `SizedBox`.
    fn drag_handle_rect(tester: &WidgetTester) -> Rect {
        tester.global_rect(tester.render_box_of(only(tester.find_by_type::<DragHandle>())))
    }

    /// Dart's 'Drag handle interactive area size at minimum possible size' and 'at given
    /// dragHandleSize'.
    fn drag_handle_size(drag_handle_size: Size, expected: Size) {
        let mut tester = tester();
        let scaffold_key = GlobalKey::new();
        let theme = ThemeData::builder()
            .bottom_sheet_theme(BottomSheetThemeData::new().drag_handle_size(drag_handle_size))
            .build(&tester.cell.borrow());
        tester.pump_widget(material_app(
            theme,
            Scaffold::new()
                .key(scaffold_key.clone().into())
                .into_widget(),
        ));

        Modal {
            show_drag_handle: Some(true),
            ..Modal::default()
        }
        .show(
            &tester,
            scaffold_context(&tester, &scaffold_key),
            sheet_builder("BottomSheet"),
        );

        tester.pump(Duration::ZERO); // Bottom sheet show animation starts.
        tester.pump(Duration::from_secs(1)); // Animation done.

        assert_eq!(drag_handle_rect(&tester).size(), expected);
    }

    #[test]
    fn the_drag_handle_interactive_area_is_at_least_the_minimum_interactive_size() {
        with_main_thread_stack(|| {
            drag_handle_size(
                Size::new(20.0, 20.0),
                Size::new(K_MIN_INTERACTIVE_DIMENSION, K_MIN_INTERACTIVE_DIMENSION),
            )
        });
    }

    #[test]
    fn the_drag_handle_interactive_area_is_the_given_drag_handle_size() {
        with_main_thread_stack(|| drag_handle_size(Size::new(100.0, 50.0), Size::new(100.0, 50.0)));
    }

    /// The colour of the decorated `Container` under the only `_DragHandle`.
    fn drag_handle_color(tester: &WidgetTester) -> Option<Color> {
        let handle = only(tester.find_by_type::<DragHandle>());
        let decorated = only(
            tester
                .find_by_type::<Container>()
                .into_iter()
                .filter(|container| tester.ancestors(*container).contains(&handle))
                .filter(|container| {
                    tester.widget(*container, |container: &Container| {
                        container.decoration.is_some()
                    })
                })
                .collect(),
        );
        tester.widget(decorated, |container: &Container| {
            container
                .decoration
                .as_ref()
                .expect("the container has a decoration")
                .as_any()
                .downcast_ref::<BoxDecoration>()
                .expect("a BoxDecoration")
                .color
                .as_ref()
                .map(|color| color.color())
        })
    }

    /// Dart's `checkDragHandleAndColors` of 'Drag handle color can take WidgetStateProperty'.
    fn check_drag_handle_and_colors(
        tester: &mut WidgetTester,
        default_color: Color,
        hovering_color: Color,
    ) {
        tester.pump(Duration::ZERO); // bottom sheet show animation starts
        tester.pump(Duration::from_secs(1)); // animation done

        let drag_handle = drag_handle_rect(tester);
        assert_eq!(drag_handle.size(), Size::new(48.0, 48.0));
        let center = drag_handle.center();
        let edge = drag_handle.top_left() - Offset::new(1.0, 1.0);

        // Shows default drag handle color
        let mut gesture = WidgetTester::mouse(1, 0);
        tester.add_pointer(&mut gesture, edge);
        tester.pump(Duration::ZERO);
        assert_eq!(drag_handle_color(tester), Some(default_color));

        // Shows hovering drag handle color
        tester.move_to(&mut gesture, center, false);
        tester.pump(Duration::ZERO);
        assert_eq!(drag_handle_color(tester), Some(hovering_color));
        tester.send(&gesture, waft_embedder::PointerChange::Remove);
    }

    /// Dart's 'Drag handle color can take WidgetStateProperty'.
    #[test]
    fn the_theme_drag_handle_color_can_take_a_widget_state_property() {
        with_main_thread_stack(|| {
            let default_color = Colors::BLUE.primary();
            let hovering_color = Colors::GREEN.primary();
            let mut tester = tester();
            let build_scaffold = |tester: &WidgetTester, scaffold_key: &GlobalKey| {
                let theme = ThemeData::builder()
                    .bottom_sheet_theme(
                        BottomSheetThemeData::new().drag_handle_color(
                            waft_widgets::WidgetStateColor::resolve_with(move |states| {
                                if states.contains(&WidgetState::Hovered) {
                                    return hovering_color;
                                }
                                default_color
                            })
                            .into_any(),
                        ),
                    )
                    .build(&tester.cell.borrow());
                app_with_scaffold(theme, scaffold_key)
            };

            let scaffold_key = GlobalKey::new();
            let app = build_scaffold(&tester, &scaffold_key);
            tester.pump_widget(app);

            Modal {
                show_drag_handle: Some(true),
                ..Modal::default()
            }
            .show(
                &tester,
                scaffold_context(&tester, &scaffold_key),
                sheet_builder("BottomSheet"),
            );

            check_drag_handle_and_colors(&mut tester, default_color, hovering_color);

            tester.pump_widget(Container::new().into_widget()); // Reset
            let scaffold_key = GlobalKey::new();
            let app = build_scaffold(&tester, &scaffold_key);
            tester.pump_widget(app);

            Persistent {
                show_drag_handle: Some(true),
                ..Persistent::default()
            }
            .show(
                &tester,
                scaffold_state(&tester, &scaffold_key),
                Rc::new(|_app, _context| {
                    Builder::new(|_app, _context| {
                        SizedBox::new()
                            .height(200.0)
                            .child(text("Bottom Sheet"))
                            .into_widget()
                    })
                    .into_widget()
                }),
            );

            check_drag_handle_and_colors(&mut tester, default_color, hovering_color);
        });
    }

    /// Not a Dart case: what Dart's collector does with the tweens and the colour animation
    /// each build of the sheet's barrier replaces; each is freed once the next build replaces
    /// it.
    #[test]
    fn rebuilding_a_modal_bottom_sheet_barrier_leaves_no_new_arena_objects() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));
            let sheet_route: Rc<Cell<Option<waft_widgets::AnyModalRoute>>> = Rc::default();
            let route = Rc::clone(&sheet_route);
            Modal::default().show(
                &tester,
                scaffold_context(&tester, &scaffold_key),
                Rc::new(move |app, context| {
                    route.set(waft_widgets::AnyModalRoute::of(app, context));
                    text("BottomSheet")
                }),
            );
            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_secs(1));
            let sheet_route = sheet_route.get().expect("the sheet built");
            let rebuild_barrier = |tester: &mut WidgetTester| {
                sheet_route
                    .as_route()
                    .changed_internal_state(&mut tester.cell.borrow_mut());
                tester.pump(Duration::from_millis(16));
            };
            rebuild_barrier(&mut tester);

            let snapshot = object_snapshot(&tester.cell.borrow());
            rebuild_barrier(&mut tester);
            tester.cell.checkpoint();
            assert_no_new_objects(&tester.cell.borrow(), &snapshot);
        });
    }

    /// Dart's 'Verify that route settings can be set in the showModalBottomSheet'.
    #[test]
    fn route_settings_can_be_set_in_show_modal_bottom_sheet() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            let route_settings = RouteSettings::new()
                .name("route_name".to_string())
                .arguments(Rc::new("route_argument".to_string()));

            tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));

            let retrieved_route_settings: Rc<RefCell<Option<RouteSettingsRef>>> =
                Rc::new(RefCell::new(None));
            let retrieved = Rc::clone(&retrieved_route_settings);
            Modal {
                route_settings: Some(RouteSettingsRef::Settings(route_settings.clone())),
                ..Modal::default()
            }
            .show(
                &tester,
                scaffold_context(&tester, &scaffold_key),
                Rc::new(move |app, context| {
                    *retrieved.borrow_mut() =
                        waft_widgets::AnyModalRoute::settings_of(app, context);
                    text("BottomSheet")
                }),
            );

            tester.pump(Duration::ZERO);
            tester.pump(Duration::from_secs(1));

            match retrieved_route_settings.borrow().as_ref() {
                Some(RouteSettingsRef::Settings(settings)) => {
                    assert_eq!(settings.name, route_settings.name);
                    assert_eq!(
                        settings
                            .arguments
                            .as_ref()
                            .and_then(|arguments| arguments.downcast_ref::<String>())
                            .map(String::as_str),
                        Some("route_argument")
                    );
                }
                _ => panic!("the sheet's route has the given settings"),
            }
        });
    }

    /// Dart's 'Verify showModalBottomSheet use AnimationController if provided.' and 'the
    /// framework do not dispose the transitionAnimationController provided by user.'. Dart
    /// taps a `GestureDetector` that calls `showModalBottomSheet`; the case calls it.
    fn modal_with_controller(dispose_after: bool) {
        let mut tester = tester();
        let saved = Saved::default();
        let controller = two_second_controller(&tester);
        tester.pump_widget(material_app(
            m3(&tester),
            Scaffold::new()
                .body(saved.builder(|| SizedBox::new().height(100.0).width(100.0).into_widget()))
                .into_widget(),
        ));

        assert_eq!(found(&tester, "BottomSheet"), 0);

        // Opening animation will start after tapping
        Modal {
            // The default duration and reverseDuration is 1 second
            transition_animation_controller: Some(controller.clone()),
            ..Modal::default()
        }
        .show(&tester, saved.get(), sheet_builder("BottomSheet"));
        tester.pump(Duration::ZERO);

        assert_eq!(found(&tester, "BottomSheet"), 1);
        tester.pump(Duration::from_millis(2000));
        assert_eq!(found(&tester, "BottomSheet"), 1);

        // Tapping above the bottom sheet to dismiss it.
        tester.tap(Offset::new(20.0, 20.0), 1); // Closing animation will start after tapping
        tester.pump(Duration::ZERO);

        assert_eq!(found(&tester, "BottomSheet"), 1);
        tester.pump(Duration::from_millis(2000));
        // The bottom sheet should still be present at the very end of the animation.
        assert_eq!(found(&tester, "BottomSheet"), 1);

        tester.pump(Duration::from_millis(1));
        // The bottom sheet should not be showing any longer.
        assert_eq!(found(&tester, "BottomSheet"), 0);

        if dispose_after {
            // Double disposal panics.
            controller.dispose(&mut tester.cell.borrow_mut());
        }
    }

    #[test]
    fn show_modal_bottom_sheet_uses_the_animation_controller_it_is_given() {
        with_main_thread_stack(|| modal_with_controller(false));
    }

    #[test]
    fn the_framework_does_not_dispose_the_transition_animation_controller_a_caller_gives() {
        with_main_thread_stack(|| modal_with_controller(true));
    }

    /// Dart's `_StatusTestAnimationController.isListening`: whether any value or status
    /// listener is registered, which its `AnimationLazyListenerMixin` counts.
    fn is_listening(tester: &WidgetTester, controller: Handle<AnimationController>) -> bool {
        use waft_animation::{AnimationLocalListenersMixin, AnimationLocalStatusListenersMixin};
        let app = tester.cell.borrow();
        !controller.local_listeners_data(&app).is_empty()
            || !controller.local_status_listeners_data(&app).is_empty()
    }

    /// Dart's 'The framework removes all animation listeners from foreign controllers when
    /// disposing.'.
    #[test]
    fn a_dismissed_persistent_sheet_leaves_no_listener_on_a_callers_controller() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            let controller = two_second_controller(&tester);

            tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));

            tester.pump(Duration::ZERO);
            assert!(!is_listening(&tester, controller.handle()));

            Persistent {
                transition_animation_controller: Some(controller.clone()),
                ..Persistent::default()
            }
            .show(
                &tester,
                scaffold_state(&tester, &scaffold_key),
                Rc::new(|_app, _context| {
                    SizedBox::new()
                        .height(200.0)
                        .child(text("BottomSheet"))
                        .into_widget()
                }),
            );

            tester.pump_and_settle();
            assert!(is_listening(&tester, controller.handle()));
            assert_eq!(found(&tester, "BottomSheet"), 1);

            // Swipe the bottom sheet to dismiss it.
            drag_text(&mut tester, "BottomSheet", Offset::new(0.0, 150.0));
            tester.pump_and_settle(); // Bottom sheet dismiss animation.
            assert!(!is_listening(&tester, controller.handle()));
            assert_eq!(found(&tester, "BottomSheet"), 0);

            controller.dispose(&mut tester.cell.borrow_mut());
        });
    }

    /// Dart's 'Verify persistence BottomSheet use AnimationController if provided.' and 'The
    /// framework does not dispose of the transitionAnimationController provided by user.'.
    /// Dart's sheet is an `ElevatedButton` whose `onPressed` pops; the case pops from the
    /// sheet's context.
    #[test]
    fn a_persistent_bottom_sheet_uses_the_animation_controller_it_is_given() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved = Saved::default();
            let sheet = Saved::default();
            let controller = two_second_controller(&tester);
            tester.pump_widget(material_app(
                m3(&tester),
                Scaffold::new()
                    .body(
                        saved.builder(|| SizedBox::new().height(100.0).width(100.0).into_widget()),
                    )
                    .into_widget(),
            ));

            assert_eq!(found(&tester, "BottomSheet"), 0);

            let saving = sheet.clone();
            show_bottom_sheet(
                &mut tester.cell.borrow_mut(),
                saved.get(),
                Rc::new(move |_app, _context| saving.builder(|| text("BottomSheet"))),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                // The default duration and reverseDuration is 1 second
                Some(controller.clone()),
                None,
            );
            tester.pump(Duration::ZERO);

            assert_eq!(found(&tester, "BottomSheet"), 1);
            tester.pump(Duration::from_millis(2000));
            assert_eq!(found(&tester, "BottomSheet"), 1);

            // Dismiss it as the sheet's button would.
            pop(&tester, sheet.get()); // Closing animation will start after tapping
            tester.pump(Duration::ZERO);

            assert_eq!(found(&tester, "BottomSheet"), 1);
            tester.pump(Duration::from_millis(2000));
            // The bottom sheet should still be present at the very end of the animation.
            assert_eq!(found(&tester, "BottomSheet"), 1);

            tester.pump(Duration::from_millis(1));
            // The bottom sheet should not be showing any longer.
            assert_eq!(found(&tester, "BottomSheet"), 0);

            tester.pump_widget(SizedBox::shrink().into_widget());
            // Double dispose panics.
            controller.dispose(&mut tester.cell.borrow_mut());
        });
    }

    /// Dart's 'Each of the internal animation controllers should be disposed by the framework.'.
    #[test]
    fn each_internal_animation_controller_is_disposed_by_the_framework() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));

            let sheet = || -> WidgetBuilder {
                Rc::new(|_app, _context| {
                    Builder::new(|_app, _context| Container::new().height(200.0).into_widget())
                        .into_widget()
                })
            };
            let scaffold = scaffold_state(&tester, &scaffold_key);
            Persistent::default().show(&tester, scaffold, sheet());

            tester.pump(Duration::ZERO);
            assert_eq!(tester.find_by_type::<BottomSheet>().len(), 1);

            // The first sheet's animation is still running.

            // Trigger the second sheet will remove the first sheet from tree.
            Persistent::default().show(&tester, scaffold, sheet());
            tester.pump(Duration::ZERO);
            assert_eq!(tester.find_by_type::<BottomSheet>().len(), 1);

            // Remove the Scaffold from the tree. A ticker still running when its provider is
            // disposed panics.
            tester.pump_widget(SizedBox::shrink().into_widget());
        });
    }

    /// Dart's 'The old route entry should be removed when a new sheet popup'.
    #[test]
    fn the_old_route_entry_is_removed_when_a_new_sheet_pops_up() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/99627
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));

            let context = scaffold_context(&tester, &scaffold_key);
            let route = waft_widgets::AnyModalRoute::of(&mut tester.cell.borrow_mut(), context)
                .expect("the scaffold is in a route");
            let can_pop = |tester: &WidgetTester| route.can_pop(&mut tester.cell.borrow_mut());
            assert!(!can_pop(&tester));

            let sheet = || -> WidgetBuilder {
                Rc::new(|_app, _context| {
                    Builder::new(|_app, _context| Container::new().height(200.0).into_widget())
                        .into_widget()
                })
            };
            let scaffold = scaffold_state(&tester, &scaffold_key);
            Persistent::default().show(&tester, scaffold, sheet());

            tester.pump(Duration::ZERO);
            assert_eq!(tester.find_by_type::<BottomSheet>().len(), 1);
            assert!(can_pop(&tester));

            // Trigger the second sheet will remove the first sheet from tree.
            let sheet_controller = Persistent::default().show(&tester, scaffold, sheet());
            tester.pump(Duration::ZERO);
            assert_eq!(tester.find_by_type::<BottomSheet>().len(), 1);
            assert!(can_pop(&tester));

            sheet_controller.close(&mut tester.cell.borrow_mut());

            assert!(!can_pop(&tester));
        });
    }

    /// Dart's 'Calling PersistentBottomSheetController.close does not crash when it is not the
    /// current bottom sheet'. Dart's buttons call `showBottomSheet` and `close`; the case
    /// calls them.
    #[test]
    fn closing_a_persistent_bottom_sheet_that_is_not_the_current_one_does_not_crash() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/93717
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));
            let scaffold = scaffold_state(&tester, &scaffold_key);

            let sheet_controller1 =
                Persistent::default().show(&tester, scaffold, sheet_builder("BottomSheet 1"));
            tester.pump_and_settle();
            assert_eq!(found(&tester, "BottomSheet 1"), 1);

            Persistent::default().show(&tester, scaffold, sheet_builder("BottomSheet 2"));
            tester.pump_and_settle();
            assert_eq!(found(&tester, "BottomSheet 2"), 1);

            // This will throw an assertion if regressed
            sheet_controller1.close(&mut tester.cell.borrow_mut());
            tester.pump_and_settle();
            assert_eq!(found(&tester, "BottomSheet 2"), 1);
        });
    }

    /// Dart's 'ModalBottomSheetRoute shows BottomSheet correctly'.
    #[test]
    fn a_pushed_modal_bottom_sheet_route_shows_a_bottom_sheet() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved = Saved::default();
            tester.pump_widget(app_saving_context(&tester, &saved));

            tester.pump(Duration::ZERO);
            assert!(tester.find_by_type::<BottomSheet>().is_empty());

            // Bring up bottom sheet.
            {
                let mut app = tester.cell.borrow_mut();
                let navigator = Navigator::of(&mut app, saved.get(), false);
                let route = ModalBottomSheetRoute::new(
                    &mut app,
                    Rc::new(|_app, _context| Container::new().into_widget()),
                    false,
                );
                navigator.push(&mut app, route.erase());
            }
            tester.pump_and_settle();
            assert_eq!(tester.find_by_type::<BottomSheet>().len(), 1);
        });
    }

    /// Dart's `TextField(focusNode: focusNode)` for what the focus cases read of it: a `Focus`
    /// on `focus_node`, which is the node Dart's `getTextFieldFocusNode` finds.
    fn focusable(focus_node: waft_widgets::FocusNodeRef) -> WidgetRef {
        waft_widgets::Focus::new(SizedBox::new().height(48.0))
            .focus_node(focus_node.clone())
            .into_widget()
    }

    use waft_widgets::FocusNodeLeaf;

    fn has_focus(tester: &WidgetTester, focus_node: waft_widgets::AnyFocusNode) -> bool {
        focus_node.has_focus(&mut tester.cell.borrow_mut())
    }

    /// Dart's `navigator.push(ModalBottomSheetRoute(requestFocus:, isScrollControlled: false,
    /// builder: (context) => Container()))`.
    fn push_container_sheet(
        tester: &WidgetTester,
        context: BuildContext,
        request_focus: Option<bool>,
    ) {
        let mut app = tester.cell.borrow_mut();
        let navigator = Navigator::of(&mut app, context, false);
        let mut route = ModalBottomSheetRoute::new(
            &mut app,
            Rc::new(|_app, _context| Container::new().into_widget()),
            false,
        );
        if let Some(request_focus) = request_focus {
            route = route.request_focus(&mut app, request_focus);
        }
        drop(navigator.push(&mut app, route.erase()));
    }

    /// The height of the stand-in for Dart's two-item `BottomNavigationBar`.
    const BOTTOM_NAVIGATION_BAR_HEIGHT: f64 = 56.0;

    /// Dart's `MaterialApp(home: Scaffold(body: Navigator(onGenerateRoute: (settings) =>
    /// MaterialPageRoute(builder: (_) => _TestPage(useRootNavigator:))), bottomNavigationBar:
    /// BottomNavigationBar(..)))`. A `PageRouteBuilder` stands in for `MaterialPageRoute`, and a
    /// 56-tall box for the bar, which are not ported; `_TestPage`'s button is a `TextButton`.
    fn nested_navigator_app(tester: &WidgetTester, use_root_navigator: Option<bool>) -> WidgetRef {
        let inner_navigator = Navigator::new().on_generate_route(move |app, _settings| {
            let page_builder: waft_widgets::RoutePageBuilder =
                Rc::new(move |_app, context, _animation, _secondary_animation| {
                    Center::new()
                        .child(crate::text_button::TextButton::new(
                            Some(waft_foundation::Listener::new(move |app| {
                                drop(
                                    Modal {
                                        use_root_navigator: use_root_navigator.unwrap_or(false),
                                        ..Modal::default()
                                    }
                                    .show_in(
                                        app,
                                        context,
                                        sheet_builder("Modal bottom sheet"),
                                    ),
                                );
                            })),
                            Some(text("Show bottom sheet")),
                        ))
                        .into_widget()
                });
            Some((waft_widgets::PageRouteBuilder::new(app, page_builder)).into_erased())
        });
        material_app(
            m3(tester),
            Scaffold::new()
                .body(inner_navigator)
                .bottom_navigation_bar(SizedBox::new().height(BOTTOM_NAVIGATION_BAR_HEIGHT))
                .into_widget(),
        )
    }

    /// Dart's `tester.getBottomLeft(find.byType(BottomSheet)).dy`.
    fn bottom_sheet_bottom(tester: &WidgetTester) -> f64 {
        let sheet = only(tester.find_by_type::<BottomSheet>());
        tester.global_rect(tester.render_box_of(sheet)).bottom
    }

    /// Dart's 'showModalBottomSheet does not use root Navigator by default'.
    #[test]
    fn show_modal_bottom_sheet_uses_the_nearest_navigator_by_default() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(nested_navigator_app(&tester, None));

            let button = center_of_text(&tester, "Show bottom sheet");
            tester.tap(button, 1);
            tester.pump_and_settle();

            // Bottom sheet is displayed in correct position within the inner navigator
            // and above the BottomNavigationBar.
            assert_eq!(
                bottom_sheet_bottom(&tester),
                600.0 - BOTTOM_NAVIGATION_BAR_HEIGHT
            );
        });
    }

    /// Dart's 'showModalBottomSheet uses root Navigator when specified'.
    #[test]
    fn show_modal_bottom_sheet_uses_the_root_navigator_when_asked() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(nested_navigator_app(&tester, Some(true)));

            let button = center_of_text(&tester, "Show bottom sheet");
            tester.tap(button, 1);
            tester.pump_and_settle();

            // Bottom sheet is displayed in correct position above all content including
            // the BottomNavigationBar.
            assert_eq!(bottom_sheet_bottom(&tester), 600.0);
        });
    }

    /// Dart's 'Setting ModalBottomSheetRoute.requestFocus to false does not request focus on
    /// the bottom sheet'.
    #[test]
    fn a_modal_bottom_sheet_route_that_does_not_request_focus_leaves_it_where_it_is() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved = Saved::default();
            let focus_node = waft_widgets::FocusNode::new(&mut tester.cell.borrow_mut())
                .into_erased::<waft_widgets::AnyFocusNode>();
            tester.pump_widget(material_app(
                m3(&tester),
                Material::new()
                    .child(saved.builder({
                        let focus_node = focus_node.clone();
                        move || focusable(focus_node.clone())
                    }))
                    .into_widget(),
            ));
            tester.pump(Duration::ZERO);

            // Initially, there is no bottom sheet and the text field has no focus.
            assert!(tester.find_by_type::<BottomSheet>().is_empty());
            assert!(!has_focus(&tester, focus_node.handle()));

            // Request focus on the text field.
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump(Duration::ZERO);
            assert!(has_focus(&tester, focus_node.handle()));

            // Bring up bottom sheet.
            push_container_sheet(&tester, saved.get(), None);
            tester.pump(Duration::ZERO);

            // The bottom sheet is showing and the text field has lost focus.
            assert_eq!(tester.find_by_type::<BottomSheet>().len(), 1);
            assert!(!has_focus(&tester, focus_node.handle()));

            // Dismiss the bottom sheet.
            pop(&tester, saved.get());
            tester.pump(Duration::ZERO);

            // The bottom sheet is dismissed and the focus is shifted back to the text field.
            assert!(tester.find_by_type::<BottomSheet>().is_empty());
            assert!(has_focus(&tester, focus_node.handle()));

            // Bring up bottom sheet again with requestFocus to false.
            push_container_sheet(&tester, saved.get(), Some(false));
            tester.pump(Duration::ZERO);

            // The bottom sheet is showing and the text field still has focus.
            assert_eq!(tester.find_by_type::<BottomSheet>().len(), 1);
            assert!(has_focus(&tester, focus_node.handle()));
        });
    }

    /// Dart's 'requestFocus works correctly in showModalBottomSheet.'. Dart shows the sheet
    /// from the navigator's own context; a context under it reaches the same navigator.
    #[test]
    fn show_modal_bottom_sheet_requests_focus_only_when_asked() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved = Saved::default();
            let focus_node = waft_widgets::FocusNode::new(&mut tester.cell.borrow_mut())
                .into_erased::<waft_widgets::AnyFocusNode>();
            tester.pump_widget(material_app(
                m3(&tester),
                Scaffold::new()
                    .body(saved.builder({
                        let focus_node = focus_node.clone();
                        move || focusable(focus_node.clone())
                    }))
                    .into_widget(),
            ));
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump(Duration::ZERO);
            assert!(has_focus(&tester, focus_node.handle()));

            let sheet_scope_has_focus = |tester: &WidgetTester| {
                let sheet_text = only(tester.find_ahem_text("BottomSheet"));
                let mut app = tester.cell.borrow_mut();
                waft_widgets::FocusScope::of(&mut app, sheet_text, true)
                    .as_node()
                    .has_focus(&mut app)
            };

            drop(
                Modal {
                    request_focus: Some(true),
                    ..Modal::default()
                }
                .show(&tester, saved.get(), sheet_builder("BottomSheet")),
            );
            tester.pump_and_settle();
            assert!(sheet_scope_has_focus(&tester));
            assert!(!has_focus(&tester, focus_node.handle()));

            pop(&tester, saved.get());
            tester.pump_and_settle();
            assert!(has_focus(&tester, focus_node.handle()));

            drop(
                Modal {
                    request_focus: Some(false),
                    ..Modal::default()
                }
                .show(&tester, saved.get(), sheet_builder("BottomSheet")),
            );
            tester.pump_and_settle();
            assert!(!sheet_scope_has_focus(&tester));
            assert!(has_focus(&tester, focus_node.handle()));
        });
    }

    /// `MaterialApp(theme:, home: Scaffold(body: Builder(saving its context)))`.
    fn app_with_scaffold_body(theme: ThemeData, saved: &Saved) -> WidgetRef {
        material_app(
            theme,
            Scaffold::new()
                .body(saved.builder(|| Center::new().child(text("Press me")).into_widget()))
                .into_widget(),
        )
    }

    /// A `Placeholder(fallbackWidth: 800)` as it sizes: its fallback limits, filled.
    fn placeholder(fallback_width: f64) -> WidgetRef {
        waft_widgets::LimitedBox::new()
            .max_width(fallback_width)
            .max_height(400.0)
            .child(SizedBox::expand())
            .into_widget()
    }

    /// Dart's 'Material3 - Default constraints are max width 640'. Placeholder is not ported;
    /// the box stands in with its fallback sizing.
    #[test]
    fn material3_default_constraints_are_a_max_width_of_640() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            tester.pump_widget(material_app(
                m3(&tester),
                MediaQuery::new(
                    waft_widgets::MediaQueryData::new().size(Size::new(1000.0, 1000.0)),
                    Scaffold::new()
                        .body(Center::new().child(text("body")))
                        .bottom_sheet(
                            waft_widgets::KeyedSubtree::new(placeholder(800.0))
                                .key(key.clone().into()),
                        ),
                )
                .into_widget(),
            ));
            assert_eq!(rect_of_key(&tester, &key).width(), 640.0);
        });
    }

    /// Dart's 'Material2 - No constraints by default for bottomSheet property'.
    #[test]
    fn material2_has_no_constraints_by_default_for_the_bottom_sheet_property() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(material_app(
                m2(&tester),
                Scaffold::new()
                    .body(Center::new().child(text("body")))
                    .bottom_sheet(text("BottomSheet"))
                    .into_widget(),
            ));
            assert_eq!(found(&tester, "BottomSheet"), 1);
            assert_eq!(
                rect_of_text(&tester, "BottomSheet"),
                Rect::from_ltrb(0.0, 586.0, 154.0, 600.0)
            );
        });
    }

    /// Dart's 'No constraints by default for showBottomSheet'. Dart's `ElevatedButton` calls
    /// `showBottomSheet`; the case calls it.
    #[test]
    fn there_are_no_constraints_by_default_for_show_bottom_sheet() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved = Saved::default();
            tester.pump_widget(app_with_scaffold_body(m2(&tester), &saved));
            assert_eq!(found(&tester, "BottomSheet"), 0);
            let scaffold = Scaffold::of(&mut tester.cell.borrow_mut(), saved.get());
            Persistent::default().show(&tester, scaffold, sheet_builder("BottomSheet"));
            tester.pump_and_settle();
            assert_eq!(found(&tester, "BottomSheet"), 1);
            assert_eq!(
                rect_of_text(&tester, "BottomSheet"),
                Rect::from_ltrb(0.0, 586.0, 154.0, 600.0)
            );
        });
    }

    /// Dart's 'No constraints by default for showModalBottomSheet'.
    #[test]
    fn there_are_no_constraints_by_default_for_show_modal_bottom_sheet() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved = Saved::default();
            tester.pump_widget(app_with_scaffold_body(m2(&tester), &saved));
            assert_eq!(found(&tester, "BottomSheet"), 0);
            Modal::default().show(&tester, saved.get(), sheet_builder("BottomSheet"));
            tester.pump_and_settle();
            assert_eq!(found(&tester, "BottomSheet"), 1);
            assert_eq!(
                rect_of_text(&tester, "BottomSheet"),
                Rect::from_ltrb(0.0, 586.0, 800.0, 600.0)
            );
        });
    }

    fn max_width_theme(tester: &WidgetTester, max_width: f64) -> ThemeData {
        ThemeData::builder()
            .bottom_sheet_theme(
                BottomSheetThemeData::new().constraints(BoxConstraints::new().max_width(max_width)),
            )
            .build(&tester.cell.borrow())
    }

    /// Dart's 'Theme constraints used for showBottomSheet' and 'constraints param overrides
    /// theme for showBottomSheet'.
    fn persistent_constraints(constraints: Option<BoxConstraints>, sheet_max_width: f64) {
        let mut tester = tester();
        let saved = Saved::default();
        tester.pump_widget(app_with_scaffold_body(
            max_width_theme(&tester, 80.0),
            &saved,
        ));
        assert_eq!(found(&tester, "BottomSheet"), 0);
        let scaffold = Scaffold::of(&mut tester.cell.borrow_mut(), saved.get());
        Persistent {
            constraints,
            ..Persistent::default()
        }
        .show(&tester, scaffold, sheet_builder("BottomSheet"));
        tester.pump_and_settle();
        assert_eq!(found(&tester, "BottomSheet"), 1);

        // Should be centered and only 80dp wide.
        let bottom_sheet_rect = rect_of_text(&tester, "BottomSheet");
        assert_eq!(bottom_sheet_rect.left, 800.0 / 2.0 - sheet_max_width / 2.0);
        assert_eq!(bottom_sheet_rect.width(), sheet_max_width);
    }

    #[test]
    fn theme_constraints_are_used_for_show_bottom_sheet() {
        with_main_thread_stack(|| persistent_constraints(None, 80.0));
    }

    #[test]
    fn the_constraints_argument_overrides_the_theme_for_show_bottom_sheet() {
        with_main_thread_stack(|| {
            persistent_constraints(Some(BoxConstraints::new().max_width(100.0)), 100.0)
        });
    }

    /// Dart's 'Theme constraints used for showModalBottomSheet' and 'constraints param
    /// overrides theme for showModalBottomSheet'.
    fn modal_constraints(constraints: Option<BoxConstraints>, sheet_max_width: f64) {
        let mut tester = tester();
        let saved = Saved::default();
        tester.pump_widget(app_with_scaffold_body(
            max_width_theme(&tester, 80.0),
            &saved,
        ));
        assert_eq!(found(&tester, "BottomSheet"), 0);
        Modal {
            constraints,
            ..Modal::default()
        }
        .show(&tester, saved.get(), sheet_builder("BottomSheet"));
        tester.pump_and_settle();
        assert_eq!(found(&tester, "BottomSheet"), 1);

        // Should be centered and only 80dp wide.
        let bottom_sheet_rect = rect_of_text(&tester, "BottomSheet");
        assert_eq!(bottom_sheet_rect.left, 800.0 / 2.0 - sheet_max_width / 2.0);
        assert_eq!(bottom_sheet_rect.width(), sheet_max_width);
    }

    #[test]
    fn theme_constraints_are_used_for_show_modal_bottom_sheet() {
        with_main_thread_stack(|| modal_constraints(None, 80.0));
    }

    #[test]
    fn the_constraints_argument_overrides_the_theme_for_show_modal_bottom_sheet() {
        with_main_thread_stack(|| {
            modal_constraints(Some(BoxConstraints::new().max_width(100.0)), 100.0)
        });
    }

    /// Dart's 'scrollControlDisabledMaxHeightRatio' group.
    fn scroll_control_disabled_max_height_ratio(
        is_scroll_controlled: bool,
        scroll_control_disabled_max_height_ratio: f64,
    ) {
        let mut tester = tester();
        let saved = Saved::default();
        tester.pump_widget(app_with_scaffold_body(m3(&tester), &saved));
        Modal {
            is_scroll_controlled,
            scroll_control_disabled_max_height_ratio,
            ..Modal::default()
        }
        .show(
            &tester,
            saved.get(),
            Rc::new(|_app, _context| SizedBox::expand().child(text("BottomSheet")).into_widget()),
        );
        tester.pump_and_settle();
        assert_eq!(
            rect_of_text(&tester, "BottomSheet"),
            Rect::from_ltrb(
                80.0,
                600.0
                    * (if is_scroll_controlled {
                        0.0
                    } else {
                        1.0 - scroll_control_disabled_max_height_ratio
                    }),
                720.0,
                600.0,
            )
        );
    }

    #[test]
    fn the_scroll_control_disabled_max_height_ratio_works_at_9_16() {
        with_main_thread_stack(|| scroll_control_disabled_max_height_ratio(false, 9.0 / 16.0));
    }

    #[test]
    fn the_scroll_control_disabled_max_height_ratio_works_at_8_16() {
        with_main_thread_stack(|| scroll_control_disabled_max_height_ratio(false, 8.0 / 16.0));
    }

    #[test]
    fn the_scroll_control_disabled_max_height_ratio_works_when_scroll_controlled() {
        with_main_thread_stack(|| scroll_control_disabled_max_height_ratio(true, 8.0 / 16.0));
    }

    /// Dart's 'Verify that modalBarrierDismissLabel is used if provided' and 'from context is
    /// used if barrierLabel is not provided'.
    fn barrier_label(barrier_label: Option<&str>) {
        let mut tester = tester();
        let scaffold_key = GlobalKey::new();
        tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));
        let context = scaffold_context(&tester, &scaffold_key);
        Modal {
            barrier_label: barrier_label.map(str::to_string),
            ..Modal::default()
        }
        .show(&tester, context, sheet_builder("BottomSheet"));
        tester.pump(Duration::ZERO);
        tester.pump(Duration::from_secs(1));

        let expected = barrier_label.map(str::to_string).unwrap_or_else(|| {
            <dyn MaterialLocalizations>::of(&mut tester.cell.borrow_mut(), context).scrim_label()
        });
        let modal_barrier = *tester
            .find_by_type::<ModalBarrier>()
            .last()
            .expect("a modal barrier");
        tester.widget(modal_barrier, |modal_barrier: &ModalBarrier| {
            assert_eq!(
                modal_barrier.semantics_label.as_deref(),
                Some(expected.as_str())
            );
        });
    }

    #[test]
    fn the_given_barrier_label_is_used() {
        with_main_thread_stack(|| barrier_label(Some("custom label")));
    }

    #[test]
    fn the_scrim_label_is_used_when_no_barrier_label_is_given() {
        with_main_thread_stack(|| barrier_label(None));
    }

    /// The sheet of Dart's animation cases: `SizedBox.expand(child: ColoredBox(key: sheetKey,
    /// color: primary, child: FilledButton(onPressed: () => Navigator.pop(context))))`. The
    /// cases pop from the saved context instead of tapping the button.
    fn expanded_sheet(sheet_key: &GlobalKey, sheet: &Saved) -> WidgetBuilder {
        let sheet_key = sheet_key.clone();
        let sheet = sheet.clone();
        Rc::new(move |app, context| {
            sheet.0.set(Some(context));
            let primary = Theme::of(app, context).color_scheme.primary;
            SizedBox::expand()
                .child(
                    waft_widgets::ColoredBox::new(primary)
                        .key(sheet_key.clone().into())
                        .child(text("Close")),
                )
                .into_widget()
        })
    }

    fn top_of(tester: &WidgetTester, key: &GlobalKey) -> f64 {
        rect_of_key(tester, key).top
    }

    fn close_to(actual: f64, expected: f64, epsilon: f64) {
        assert!(
            (actual - expected).abs() <= epsilon,
            "{actual} is not within {epsilon} of {expected}"
        );
    }

    /// Dart's 'Bottom sheet animation can be customized'.
    #[test]
    fn the_bottom_sheet_animation_can_be_customized() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved = Saved::default();
            let sheet = Saved::default();
            let sheet_key = GlobalKey::new();

            // Test custom animation style.
            tester.pump_widget(app_with_scaffold_body(m3(&tester), &saved));
            let scaffold = Scaffold::of(&mut tester.cell.borrow_mut(), saved.get());
            Persistent {
                sheet_animation_style: Some(AnimationStyle::new(
                    None,
                    Some(Duration::from_millis(800)),
                    None,
                    Some(Duration::from_millis(400)),
                )),
                ..Persistent::default()
            }
            .show(&tester, scaffold, expanded_sheet(&sheet_key, &sheet));
            tester.pump(Duration::ZERO);
            // Advance the animation by 1/2 of the custom forward duration.
            tester.pump(Duration::from_millis(400));

            // The bottom sheet is partially visible.
            close_to(top_of(&tester, &sheet_key), 134.6, 0.1);

            // Advance the animation by 1/2 of the custom forward duration.
            tester.pump(Duration::from_millis(400));

            // The bottom sheet is fully visible.
            assert_eq!(top_of(&tester, &sheet_key), 0.0);

            // Dismiss the bottom sheet.
            pop(&tester, sheet.get());
            tester.pump(Duration::ZERO);
            // Advance the animation by 1/2 of the custom reverse duration.
            tester.pump(Duration::from_millis(200));

            // The bottom sheet is partially visible.
            close_to(top_of(&tester, &sheet_key), 134.6, 0.1);

            // Advance the animation by 1/2 of the custom reverse duration.
            tester.pump(Duration::from_millis(200));

            // The bottom sheet is dismissed.
            assert_eq!(top_of(&tester, &sheet_key), 600.0);

            // Test no animation style.
            tester.pump_widget(app_with_scaffold_body(m3(&tester), &saved));
            tester.pump_and_settle();
            let scaffold = Scaffold::of(&mut tester.cell.borrow_mut(), saved.get());
            Persistent {
                sheet_animation_style: Some(AnimationStyle::NO_ANIMATION),
                ..Persistent::default()
            }
            .show(&tester, scaffold, expanded_sheet(&sheet_key, &sheet));
            tester.pump(Duration::ZERO);

            // The bottom sheet is fully visible.
            assert_eq!(top_of(&tester, &sheet_key), 0.0);

            // Dismiss the bottom sheet.
            pop(&tester, sheet.get());
            tester.pump(Duration::ZERO);

            // The bottom sheet is dismissed.
            assert!(!mounted(&tester, &sheet_key));
        });
    }

    /// Dart's 'Modal bottom sheet default animation' and 'Modal bottom sheet animation can be
    /// customized': `forward`/`reverse` are half of each duration.
    fn modal_animation(style: Option<AnimationStyle>, half_forward: u64, half_reverse: u64) {
        let mut tester = tester();
        let saved = Saved::default();
        let sheet = Saved::default();
        let sheet_key = GlobalKey::new();
        tester.pump_widget(app_with_scaffold_body(m3(&tester), &saved));

        // Tap the 'X' to show the bottom sheet.
        Modal {
            sheet_animation_style: style,
            ..Modal::default()
        }
        .show(&tester, saved.get(), expanded_sheet(&sheet_key, &sheet));
        tester.pump(Duration::ZERO);
        // Advance the animation by 1/2 of the forward duration.
        tester.pump(Duration::from_millis(half_forward));

        // The modal bottom sheet is partially visible.
        close_to(top_of(&tester, &sheet_key), 316.7, 0.1);

        // Advance the animation by 1/2 of the forward duration.
        tester.pump(Duration::from_millis(half_forward));

        // The modal bottom sheet is fully visible.
        assert_eq!(top_of(&tester, &sheet_key), 262.5);

        // Dismiss the bottom sheet.
        pop(&tester, sheet.get());
        tester.pump(Duration::ZERO);
        // Advance the animation by 1/2 of the reverse duration.
        tester.pump(Duration::from_millis(half_reverse));

        // The modal bottom sheet is partially visible.
        close_to(top_of(&tester, &sheet_key), 316.7, 0.1);

        // Advance the animation by 1/2 of the reverse duration.
        tester.pump(Duration::from_millis(half_reverse));

        // The modal bottom sheet is dismissed.
        assert_eq!(top_of(&tester, &sheet_key), 600.0);
    }

    #[test]
    fn the_modal_bottom_sheet_default_animation() {
        with_main_thread_stack(|| modal_animation(None, 125, 100));
    }

    #[test]
    fn the_modal_bottom_sheet_animation_can_be_customized() {
        with_main_thread_stack(|| {
            modal_animation(
                Some(AnimationStyle::new(
                    None,
                    Some(Duration::from_millis(800)),
                    None,
                    Some(Duration::from_millis(400)),
                )),
                400,
                200,
            );

            // Test no animation style.
            let mut tester = tester();
            let saved = Saved::default();
            let sheet = Saved::default();
            let sheet_key = GlobalKey::new();
            tester.pump_widget(app_with_scaffold_body(m3(&tester), &saved));
            tester.pump_and_settle();
            Modal {
                sheet_animation_style: Some(AnimationStyle::NO_ANIMATION),
                ..Modal::default()
            }
            .show(&tester, saved.get(), expanded_sheet(&sheet_key, &sheet));
            tester.pump(Duration::ZERO);

            // The bottom sheet is fully visible.
            assert_eq!(top_of(&tester, &sheet_key), 262.5);

            // Dismiss the bottom sheet.
            pop(&tester, sheet.get());
            tester.pump(Duration::ZERO);

            // The bottom sheet is dismissed.
            assert!(!mounted(&tester, &sheet_key));
        });
    }

    /// Dart's 'BottomSheet does not crash at zero area'.
    #[test]
    fn a_bottom_sheet_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(material_app(
                m3(&tester),
                Scaffold::new()
                    .body(
                        Center::new().child(SizedBox::shrink().child(BottomSheet::new(
                            Rc::new(|_app| {}),
                            Rc::new(|_app, _context| text("X")),
                        ))),
                    )
                    .into_widget(),
            ));
            let sheet = only(tester.find_by_type::<BottomSheet>());
            assert_eq!(
                tester.global_rect(tester.render_box_of(sheet)).size(),
                Size::ZERO
            );
        });
    }

    /// Dart's 'ModalBottomSheet uses AnimationStyle curve' and 'reverseCurve'.
    #[test]
    fn a_modal_bottom_sheet_uses_the_animation_style_curve() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved = Saved::default();
            let sheet = Saved::default();
            let sheet_key = GlobalKey::new();
            tester.pump_widget(app_with_scaffold_body(m3(&tester), &saved));
            Modal {
                sheet_animation_style: Some(AnimationStyle::new(
                    Some(Curves::linear()),
                    None,
                    None,
                    None,
                )),
                ..Modal::default()
            }
            .show(&tester, saved.get(), expanded_sheet(&sheet_key, &sheet));
            tester.pump(Duration::ZERO);

            // Advance the animation by 50ms.
            tester.pump(Duration::from_millis(50));
            let open_extent1 = top_of(&tester, &sheet_key);

            // Advance the animation by an additional 50ms.
            tester.pump(Duration::from_millis(50));
            let open_extent2 = top_of(&tester, &sheet_key);

            // Advance the animation by an additional 50ms.
            tester.pump(Duration::from_millis(50));
            let open_extent3 = top_of(&tester, &sheet_key);

            // For the linear curve, the distance covered in each time interval should
            // be the same.
            close_to(
                open_extent1 - open_extent2,
                open_extent2 - open_extent3,
                0.1,
            );
            tester.pump_and_settle();

            // Dismiss the bottom sheet.
            pop(&tester, sheet.get());
            tester.pump_and_settle();
        });
    }

    #[test]
    fn a_modal_bottom_sheet_uses_the_animation_style_reverse_curve() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved = Saved::default();
            let sheet = Saved::default();
            let sheet_key = GlobalKey::new();
            tester.pump_widget(app_with_scaffold_body(m3(&tester), &saved));
            Modal {
                sheet_animation_style: Some(AnimationStyle::new(
                    None,
                    None,
                    Some(Curves::linear()),
                    None,
                )),
                ..Modal::default()
            }
            .show(&tester, saved.get(), expanded_sheet(&sheet_key, &sheet));
            tester.pump_and_settle();

            // Start the dismissal.
            pop(&tester, sheet.get());
            tester.pump(Duration::ZERO);

            // Advance the animation by 50ms during the close transition.
            tester.pump(Duration::from_millis(50));
            let close_extent1 = top_of(&tester, &sheet_key);

            // Advance the animation by an additional 50ms.
            tester.pump(Duration::from_millis(50));
            let close_extent2 = top_of(&tester, &sheet_key);

            // Advance the animation by an additional 50ms.
            tester.pump(Duration::from_millis(50));
            let close_extent3 = top_of(&tester, &sheet_key);

            // For the linear curve, the distance covered in each time interval should
            // be the same.
            close_to(
                close_extent2 - close_extent1,
                close_extent3 - close_extent2,
                0.1,
            );

            tester.pump_and_settle();
        });
    }

    /// Dart's 'ModalBottomSheet with AnimationStyle.noAnimation opens and closes immediately'.
    #[test]
    fn a_modal_bottom_sheet_without_animation_opens_and_closes_immediately() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved = Saved::default();
            let sheet = Saved::default();
            let sheet_key = GlobalKey::new();
            tester.pump_widget(app_with_scaffold_body(m3(&tester), &saved));
            let saving = sheet.clone();
            let key = sheet_key.clone();
            Modal {
                sheet_animation_style: Some(AnimationStyle::NO_ANIMATION),
                ..Modal::default()
            }
            .show(
                &tester,
                saved.get(),
                Rc::new(move |_app, context| {
                    saving.0.set(Some(context));
                    SizedBox::new()
                        .key(key.clone().into())
                        .child(text("Close"))
                        .into_widget()
                }),
            );
            tester.pump(Duration::ZERO);

            assert!(mounted(&tester, &sheet_key));

            pop(&tester, sheet.get());
            tester.pump(Duration::ZERO);

            assert!(!mounted(&tester, &sheet_key));
        });
    }

    /// Dart's 'ModalBottomSheet does not jump when drag gesture ends'.
    #[test]
    fn a_modal_bottom_sheet_does_not_jump_when_the_drag_gesture_ends() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/183299.
            let mut tester = tester();
            let saved = Saved::default();
            let sheet_key = GlobalKey::new();
            tester.pump_widget(app_with_scaffold_body(m3(&tester), &saved));
            let key = sheet_key.clone();
            Modal {
                show_drag_handle: Some(true),
                ..Modal::default()
            }
            .show(
                &tester,
                saved.get(),
                Rc::new(move |_app, _context| {
                    SizedBox::expand()
                        .key(key.clone().into())
                        .child(SizedBox::new())
                        .into_widget()
                }),
            );
            tester.pump_and_settle();

            let mut gesture = WidgetTester::touch(1);
            let center = drag_handle_rect(&tester).center();
            tester.down(&mut gesture, center);

            let by = |tester: &mut WidgetTester,
                      gesture: &mut crate::test_support::TestPointer,
                      dy: f64| {
                let at = gesture.position + Offset::new(0.0, dy);
                tester.move_to(gesture, at, true);
            };
            by(&mut tester, &mut gesture, 50.0);
            tester.pump(Duration::ZERO);

            by(&mut tester, &mut gesture, 50.0);
            tester.pump(Duration::ZERO);

            by(&mut tester, &mut gesture, 100.0);
            tester.pump(Duration::ZERO);

            let y_before_up = top_of(&tester, &sheet_key);

            tester.up(&gesture);
            tester.pump(Duration::ZERO);

            let y_after_up = top_of(&tester, &sheet_key);

            // The bottom sheet should not jump when the drag gesture ends.
            // Its position immediately after releasing the gesture should remain
            // approximately the same as the last dragged position, ensuring the
            // animation continues from the current visual offset.
            close_to(y_after_up, y_before_up, 0.1);
        });
    }

    // persistent_bottom_sheet_test.dart

    /// `ListView(children: [SizedBox(height: 100.0, child: Text(t)) for t in texts])` with
    /// the given setters.
    fn texts_list(texts: &[&'static str]) -> waft_widgets::ListView {
        waft_widgets::ListView::new().children(
            texts
                .iter()
                .map(|&content| {
                    SizedBox::new()
                        .height(100.0)
                        .child(text(content))
                        .into_widget()
                })
                .collect::<Vec<_>>(),
        )
    }

    /// `DraggableScrollableSheet(expand: false, builder: (_, controller) => ListView(controller:
    /// controller, shrinkWrap: true, children: One, Two, Three))`.
    fn draggable_texts(should_close_on_min_extent: Option<bool>) -> WidgetRef {
        let mut sheet =
            waft_widgets::DraggableScrollableSheet::new(Rc::new(|_app, _context, controller| {
                texts_list(&["One", "Two", "Three"])
                    .controller(controller)
                    .shrink_wrap(true)
                    .into_widget()
            }))
            .expand(false);
        if let Some(should_close_on_min_extent) = should_close_on_min_extent {
            sheet = sheet.should_close_on_min_extent(should_close_on_min_extent);
        }
        sheet.into_widget()
    }

    /// Whether a hit test at the centre of `element` reaches its render box: Dart's
    /// `hitTestable()` finder.
    fn hit_testable(tester: &WidgetTester, element: AnyElement) -> bool {
        let render = tester.render_box_of(element);
        let center = tester.global_rect(render).center();
        let mut app = tester.cell.borrow_mut();
        let view_id = app.platform().views()[0].id();
        let mut result = waft_gestures::HitTestResult::new();
        waft_gestures::GestureBinding::instance(&mut app).hit_test_in_view(
            &mut app,
            &mut result,
            center,
            view_id,
        );
        result.path().iter().any(|entry| {
            (entry.target() as &dyn std::any::Any)
                .downcast_ref::<waft_rendering::BoxHitTestEntry>()
                .is_some_and(|entry| entry.target() == render)
        })
    }

    /// Dart's 'Scaffold.bottomSheet update test'.
    #[test]
    fn the_scaffold_bottom_sheet_can_come_back_while_it_is_disappearing() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/83668
            let mut tester = tester();
            let build_frame = |tester: &WidgetTester, bottom_sheet: Option<WidgetRef>| {
                let mut scaffold = Scaffold::new().body(placeholder(800.0));
                if let Some(bottom_sheet) = bottom_sheet {
                    scaffold = scaffold.bottom_sheet(bottom_sheet);
                }
                material_app(m3(tester), scaffold.into_widget())
            };

            tester.pump_widget(build_frame(&tester, Some(text("I love Flutter!"))));
            tester.pump_widget(build_frame(&tester, None));

            // The disappearing animation has not yet been completed.
            tester.pump_widget(build_frame(&tester, Some(text("I love Flutter!"))));
        });
    }

    /// Dart's 'Verify that a BottomSheet can be rebuilt with ScaffoldFeatureController.setState()'.
    #[test]
    fn a_bottom_sheet_can_be_rebuilt_with_its_controller_set_state() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            let build_count = Rc::new(Cell::new(0));
            tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));

            let scaffold = scaffold_state(&tester, &scaffold_key);
            let count = Rc::clone(&build_count);
            let bottom_sheet = Persistent::default().show(
                &tester,
                scaffold,
                Rc::new(move |_app, _context| {
                    let count = Rc::clone(&count);
                    Builder::new(move |_app, _context| {
                        count.set(count.get() + 1);
                        Container::new().height(200.0).into_widget()
                    })
                    .into_widget()
                }),
            );

            tester.pump(Duration::ZERO);
            assert_eq!(build_count.get(), 1);
            bottom_sheet.set_state(&mut tester.cell.borrow_mut(), &mut |_app| {});
            tester.pump(Duration::ZERO);
            assert_eq!(build_count.get(), 2);
        });
    }

    /// Dart's 'Verify that a persistent BottomSheet cannot be dismissed'.
    #[test]
    fn a_persistent_bottom_sheet_cannot_be_dismissed() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(material_app(
                m3(&tester),
                Scaffold::new()
                    .body(Center::new().child(text("body")))
                    .bottom_sheet(draggable_texts(None))
                    .into_widget(),
            ));

            tester.pump_and_settle();

            assert_eq!(found(&tester, "Two"), 1);

            drag_text(&mut tester, "Two", Offset::new(0.0, 400.0));
            tester.pump_and_settle();

            assert_eq!(found(&tester, "Two"), 1);
        });
    }

    /// Dart's 'Verify that a scrollable BottomSheet can be dismissed'.
    #[test]
    fn a_scrollable_bottom_sheet_can_be_dismissed() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));

            let scaffold = scaffold_state(&tester, &scaffold_key);
            Persistent::default().show(
                &tester,
                scaffold,
                Rc::new(|_app, _context| {
                    texts_list(&["One", "Two", "Three"])
                        .shrink_wrap(true)
                        .primary(false)
                        .into_widget()
                }),
            );

            tester.pump_and_settle();

            assert_eq!(found(&tester, "Two"), 1);

            drag_text(&mut tester, "Two", Offset::new(0.0, 400.0));
            tester.pump_and_settle();

            assert_eq!(found(&tester, "Two"), 0);
        });
    }

    /// Dart's 'Verify DraggableScrollableSheet.shouldCloseOnMinExtent == false prevents
    /// dismissal' and 'Verify that a scrollControlled BottomSheet can be dismissed'.
    fn draggable_persistent(should_close_on_min_extent: Option<bool>, dismissed: bool) {
        let mut tester = tester();
        let scaffold_key = GlobalKey::new();
        tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));

        let scaffold = scaffold_state(&tester, &scaffold_key);
        Persistent::default().show(
            &tester,
            scaffold,
            Rc::new(move |_app, _context| draggable_texts(should_close_on_min_extent)),
        );

        tester.pump_and_settle();

        assert_eq!(found(&tester, "Two"), 1);

        drag_text(&mut tester, "Two", Offset::new(0.0, 400.0));
        tester.pump_and_settle();

        assert_eq!(found(&tester, "Two"), if dismissed { 0 } else { 1 });
    }

    #[test]
    fn should_close_on_min_extent_false_prevents_dismissing_a_draggable_sheet() {
        with_main_thread_stack(|| draggable_persistent(Some(false), false));
    }

    #[test]
    fn a_scroll_controlled_bottom_sheet_can_be_dismissed() {
        with_main_thread_stack(|| draggable_persistent(None, true));
    }

    /// Dart's 'Verify that a BottomSheet animates non-linearly' (persistent).
    #[test]
    fn a_persistent_bottom_sheet_animates_non_linearly() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));

            let check = |tester: &mut WidgetTester| {
                let first_position = center_of_text(tester, "One");
                tester.pump(Duration::from_millis(30));
                let second_position = center_of_text(tester, "One");
                tester.pump(Duration::from_millis(30));
                let third_position = center_of_text(tester, "One");

                let dy_delta1 = second_position.dy() - first_position.dy();
                let dy_delta2 = third_position.dy() - second_position.dy();

                // If the animation were linear, these two values would be the same.
                assert!((dy_delta1 - dy_delta2).abs() > 0.1);
            };

            let scaffold = scaffold_state(&tester, &scaffold_key);
            Persistent::default().show(
                &tester,
                scaffold,
                Rc::new(|_app, _context| {
                    texts_list(&["One", "Two", "Three"])
                        .shrink_wrap(true)
                        .primary(false)
                        .into_widget()
                }),
            );
            tester.pump(Duration::ZERO);
            check(&mut tester);

            tester.pump_and_settle();

            assert_eq!(found(&tester, "Two"), 1);

            drag_text(&mut tester, "Two", Offset::new(0.0, 200.0));
            check(&mut tester);
            tester.pump_and_settle();

            assert_eq!(found(&tester, "Two"), 0);
        });
    }

    /// `DraggableScrollableSheet(expand: false, builder: (_, controller) => ListView.builder(
    /// itemExtent: 50.0, itemCount: 50, itemBuilder: (_, index) => Text('Item $index'),
    /// controller: controller))`.
    fn draggable_items() -> waft_widgets::DraggableScrollableSheet {
        waft_widgets::DraggableScrollableSheet::new(Rc::new(|_app, _context, controller| {
            waft_widgets::ListView::builder(|_app, _context, index| {
                Some(AhemText::new(&format!("Item {index}")).into_widget())
            })
            .item_count(50)
            .build()
            .item_extent(50.0)
            .controller(controller)
            .into_widget()
        }))
        .expand(false)
    }

    /// `MaterialApp(home: Scaffold(appBar: AppBar(), body: Center(child: Text('body')),
    /// bottomSheet: bottomSheet, floatingActionButton: FloatingActionButton(onPressed: null,
    /// child: Text('fab'))))`.
    fn app_bar_scaffold(tester: &WidgetTester, bottom_sheet: Option<WidgetRef>) -> WidgetRef {
        let mut scaffold = Scaffold::new()
            .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                crate::app_bar::AppBar::new(),
            ))
            .body(Center::new().child(text("body")))
            .floating_action_button(
                crate::floating_action_button::FloatingActionButton::new(None).child(text("fab")),
            );
        if let Some(bottom_sheet) = bottom_sheet {
            scaffold = scaffold.bottom_sheet(bottom_sheet);
        }
        material_app(m3(tester), scaffold.into_widget())
    }

    /// Dart's `find.byType(T).hitTestable()`: how many `T`s a hit test at their centre reaches.
    fn hit_testable_count<T: 'static>(tester: &WidgetTester) -> usize {
        tester
            .find_by_type::<T>()
            .into_iter()
            .filter(|element| hit_testable(tester, *element))
            .count()
    }

    fn back_buttons(tester: &WidgetTester) -> usize {
        hit_testable_count::<crate::action_buttons::BackButton>(tester)
    }

    /// Dart's `tester.tap(find.byType(BackButton))`.
    fn tap_back_button(tester: &mut WidgetTester, pointer: i64) {
        let button = only(tester.find_by_type::<crate::action_buttons::BackButton>());
        let center = tester.global_rect(tester.render_box_of(button)).center();
        tester.tap(center, pointer);
    }

    /// Dart's 'Persistent draggableScrollableSheet localHistoryEntries test'.
    #[test]
    fn a_persistent_draggable_sheet_adds_and_removes_its_local_history_entry() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/110123
            let mut tester = tester();
            let draggable_scrollable_sheet = || {
                draggable_items()
                    .snap(true)
                    .initial_child_size(0.3)
                    .min_child_size(0.3)
                    .into_widget()
            };

            let frame = app_bar_scaffold(&tester, Some(draggable_scrollable_sheet()));
            tester.pump_widget(frame);
            tester.pump_and_settle();

            assert_eq!(back_buttons(&tester), 0);

            drag_text(&mut tester, "Item 2", Offset::new(0.0, -200.0));
            tester.pump_and_settle();
            // We've started to drag up, we should have a back button now for a11y
            assert_eq!(back_buttons(&tester), 1);

            fling_text(&mut tester, "Item 2", Offset::new(0.0, 200.0), 2000.0);
            tester.pump_and_settle();
            // BackButton should be hidden
            assert_eq!(back_buttons(&tester), 0);

            // Show the back button again
            drag_text(&mut tester, "Item 2", Offset::new(0.0, -200.0));
            tester.pump_and_settle();
            assert_eq!(back_buttons(&tester), 1);

            // Remove the draggableScrollableSheet should hide the back button
            let frame = app_bar_scaffold(&tester, None);
            tester.pump_widget(frame);
            assert_eq!(back_buttons(&tester), 0);
        });
    }

    /// Dart's 'Verify that a persistent BottomSheet can fling up and hide the fab'.
    #[test]
    fn a_persistent_bottom_sheet_can_fling_up_and_hide_the_fab() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let frame = app_bar_scaffold(&tester, Some(draggable_items().into_widget()));
            tester.pump_widget(frame);
            tester.pump_and_settle();
            let fabs = |tester: &WidgetTester| {
                tester
                    .find_by_type::<crate::floating_action_button::FloatingActionButton>()
                    .len()
            };
            let hit_testable_fabs = |tester: &WidgetTester| {
                hit_testable_count::<crate::floating_action_button::FloatingActionButton>(tester)
            };

            assert_eq!(found(&tester, "Item 2"), 1);
            assert_eq!(found(&tester, "Item 22"), 0);
            assert_eq!(fabs(&tester), 1);
            assert_eq!(hit_testable_fabs(&tester), 1);
            assert_eq!(back_buttons(&tester), 0);

            drag_text(&mut tester, "Item 2", Offset::new(0.0, -20.0));
            tester.pump_and_settle();

            assert_eq!(found(&tester, "Item 2"), 1);
            assert_eq!(found(&tester, "Item 22"), 0);
            assert_eq!(fabs(&tester), 1);
            assert_eq!(hit_testable_fabs(&tester), 1);

            fling_text(&mut tester, "Item 2", Offset::new(0.0, -600.0), 2000.0);
            tester.pump_and_settle();

            assert_eq!(found(&tester, "Item 2"), 0);
            assert_eq!(found(&tester, "Item 22"), 1);
            assert_eq!(fabs(&tester), 1);
            assert_eq!(hit_testable_fabs(&tester), 0);
        });
    }

    /// Dart's 'Verify that a back button resets a persistent BottomSheet'.
    #[test]
    fn a_back_button_resets_a_persistent_bottom_sheet() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let frame = app_bar_scaffold(&tester, Some(draggable_items().into_widget()));
            tester.pump_widget(frame);
            tester.pump_and_settle();

            assert_eq!(found(&tester, "Item 2"), 1);
            assert_eq!(found(&tester, "Item 22"), 0);
            assert_eq!(back_buttons(&tester), 0);

            drag_text(&mut tester, "Item 2", Offset::new(0.0, -20.0));
            tester.pump_and_settle();

            assert_eq!(found(&tester, "Item 2"), 1);
            assert_eq!(found(&tester, "Item 22"), 0);
            // We've started to drag up, we should have a back button now for a11y
            assert_eq!(back_buttons(&tester), 1);

            tap_back_button(&mut tester, 2);
            tester.pump_and_settle();

            assert_eq!(back_buttons(&tester), 0);
            assert_eq!(found(&tester, "Item 2"), 1);
            assert_eq!(found(&tester, "Item 22"), 0);

            fling_text(&mut tester, "Item 2", Offset::new(0.0, -600.0), 2000.0);
            tester.pump_and_settle();

            assert_eq!(found(&tester, "Item 2"), 0);
            assert_eq!(found(&tester, "Item 22"), 1);
            assert_eq!(back_buttons(&tester), 1);

            tap_back_button(&mut tester, 3);
            tester.pump_and_settle();

            assert_eq!(back_buttons(&tester), 0);
            assert_eq!(found(&tester, "Item 2"), 1);
            assert_eq!(found(&tester, "Item 22"), 0);
        });
    }

    /// Dart's 'Verify that a scrollable BottomSheet hides the fab when scrolled up'.
    #[test]
    fn a_scrollable_bottom_sheet_hides_the_fab_when_scrolled_up() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            tester.pump_widget(material_app(
                m3(&tester),
                Scaffold::new()
                    .key(scaffold_key.clone().into())
                    .body(Center::new().child(text("body")))
                    .floating_action_button(
                        crate::floating_action_button::FloatingActionButton::new(None)
                            .child(text("fab")),
                    )
                    .into_widget(),
            ));
            let fab = |tester: &WidgetTester| {
                only(tester.find_by_type::<crate::floating_action_button::FloatingActionButton>())
            };

            let scaffold = scaffold_state(&tester, &scaffold_key);
            Persistent::default().show(
                &tester,
                scaffold,
                Rc::new(|_app, _context| {
                    waft_widgets::DraggableScrollableSheet::new(Rc::new(
                        |_app, _context, controller| {
                            texts_list(&[
                                "One", "Two", "Three", "Three", "Three", "Three", "Three", "Three",
                                "Three", "Three", "Three",
                            ])
                            .controller(controller)
                            .shrink_wrap(true)
                            .into_widget()
                        },
                    ))
                    .expand(false)
                    .into_widget()
                }),
            );

            tester.pump_and_settle();

            assert_eq!(found(&tester, "Two"), 1);
            assert!(hit_testable(&tester, fab(&tester)));

            drag_text(&mut tester, "Two", Offset::new(0.0, -600.0));
            tester.pump_and_settle();

            assert_eq!(found(&tester, "Two"), 1);
            assert_eq!(
                tester
                    .find_by_type::<crate::floating_action_button::FloatingActionButton>()
                    .len(),
                1
            );
            assert!(!hit_testable(&tester, fab(&tester)));
        });
    }

    /// Dart's 'showBottomSheet()'.
    #[test]
    fn show_bottom_sheet_builds_the_sheet_once() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let key = GlobalKey::new();
            tester.pump_widget(material_app(
                m3(&tester),
                Scaffold::new()
                    .body(
                        waft_widgets::KeyedSubtree::new(placeholder(800.0)).key(key.clone().into()),
                    )
                    .into_widget(),
            ));

            let build_count = Rc::new(Cell::new(0));
            let count = Rc::clone(&build_count);
            let context = key
                .current_context(&tester.cell.borrow_mut())
                .expect("mounted");
            show_bottom_sheet(
                &mut tester.cell.borrow_mut(),
                context,
                Rc::new(move |_app, _context| {
                    let count = Rc::clone(&count);
                    Builder::new(move |_app, _context| {
                        count.set(count.get() + 1);
                        Container::new().height(200.0).into_widget()
                    })
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
            tester.pump(Duration::ZERO);
            assert_eq!(build_count.get(), 1);
        });
    }

    /// Dart's 'Scaffold removes top MediaQuery padding'.
    #[test]
    fn the_scaffold_removes_top_media_query_padding_from_a_bottom_sheet() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_context = Saved::default();
            let bottom_sheet_context = Saved::default();
            let saving = scaffold_context.clone();
            tester.pump_widget(material_app(
                m3(&tester),
                Builder::new(move |app, context| {
                    let data = MediaQuery::of(app, context)
                        .copy_with()
                        .padding(EdgeInsets::all(50.0));
                    let saving = saving.clone();
                    MediaQuery::new(
                        data,
                        Scaffold::new()
                            .resize_to_avoid_bottom_inset(false)
                            .body(saving.builder(|| Container::new().into_widget())),
                    )
                    .into_widget()
                })
                .into_widget(),
            ));

            tester.pump(Duration::ZERO);

            let saving = bottom_sheet_context.clone();
            show_bottom_sheet(
                &mut tester.cell.borrow_mut(),
                scaffold_context.get(),
                Rc::new(move |_app, context| {
                    saving.0.set(Some(context));
                    Container::new().into_widget()
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

            tester.pump(Duration::ZERO);

            assert_eq!(
                padding_of(&tester, bottom_sheet_context.get()),
                EdgeInsets::only(50.0, 0.0, 50.0, 50.0)
            );
        });
    }

    /// Dart's 'Scaffold.bottomSheet'. Dart's `ElevatedButton` in the sheet calls
    /// `showModalBottomSheet`; the case calls it from the button's context.
    #[test]
    fn a_scaffold_bottom_sheet_shows_and_its_modal_sheet_dismisses() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let bottom_sheet_key = GlobalKey::new();
            let button_context = Saved::default();
            let sheet = Container::new()
                .key(bottom_sheet_key.clone().into())
                .alignment(AlignmentGeometry::CENTER)
                .height(200.0)
                .child(button_context.builder(|| text("showModalBottomSheet")))
                .into_widget();
            tester.pump_widget(material_app(
                m2(&tester),
                Scaffold::new()
                    .body(placeholder(800.0))
                    .bottom_sheet(sheet)
                    .into_widget(),
            ));

            assert_eq!(found(&tester, "showModalBottomSheet"), 1);
            let rect = rect_of_key(&tester, &bottom_sheet_key);
            assert_eq!(rect.size(), Size::new(800.0, 200.0));
            assert_eq!(rect.top_left(), Offset::new(0.0, 400.0));

            // Show the modal bottomSheet
            Modal::default().show(
                &tester,
                button_context.get(),
                sheet_builder("modal bottom sheet"),
            );
            tester.pump_and_settle();
            assert_eq!(found(&tester, "modal bottom sheet"), 1);

            // Dismiss the modal bottomSheet by tapping above the sheet
            tester.tap(Offset::new(20.0, 20.0), 1);
            tester.pump_and_settle();
            assert_eq!(found(&tester, "modal bottom sheet"), 0);
            assert_eq!(found(&tester, "showModalBottomSheet"), 1);

            // Remove the persistent bottomSheet
            tester.pump_widget(material_app(
                m2(&tester),
                Scaffold::new().body(placeholder(800.0)).into_widget(),
            ));
            tester.pump_and_settle();
            assert_eq!(found(&tester, "showModalBottomSheet"), 0);
            assert!(!mounted(&tester, &bottom_sheet_key));
        });
    }

    /// Dart's 'Scaffold.bottomSheet should be updated without creating a new RO when the new
    /// widget has the same key and type.'.
    #[test]
    fn a_scaffold_bottom_sheet_of_the_same_type_keeps_its_render_object() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/71435
            let mut tester = tester();
            let build_frame = |tester: &WidgetTester, content: &'static str| {
                material_app(
                    m3(tester),
                    Scaffold::new()
                        .body(placeholder(800.0))
                        .bottom_sheet(text(content))
                        .into_widget(),
                )
            };

            tester.pump_widget(build_frame(&tester, "I love Flutter!"));
            let render_before_update =
                tester.render_box_of(only(tester.find_ahem_text("I love Flutter!")));

            tester.pump_widget(build_frame(&tester, "Flutter is the best!"));
            tester.pump_and_settle();
            let render_after_update =
                tester.render_box_of(only(tester.find_ahem_text("Flutter is the best!")));

            assert_eq!(render_before_update, render_after_update);
        });
    }

    /// Dart's 'Verify that visual properties are passed through' (persistent).
    #[test]
    fn visual_properties_are_passed_through_to_a_persistent_bottom_sheet() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            let color = Colors::PINK.primary();
            let elevation = 9.0;
            let shape: ShapeBorderRef =
                ShapeBorderRef::new(waft_painting::BeveledRectangleBorder::new(
                    waft_painting::BorderSide::NONE,
                    waft_painting::BorderRadiusGeometry::all(waft_embedder::Radius::circular(12.0)),
                ));
            let clip_behavior = Clip::AntiAlias;

            tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));

            let scaffold = scaffold_state(&tester, &scaffold_key);
            scaffold.show_bottom_sheet(
                &mut tester.cell.borrow_mut(),
                Rc::new(|_app, _context| {
                    texts_list(&["One", "Two", "Three"])
                        .shrink_wrap(true)
                        .primary(false)
                        .into_widget()
                }),
                Some(color),
                Some(elevation),
                Some(shape.clone()),
                Some(clip_behavior),
                None,
                None,
                None,
                None,
                None,
            );

            tester.pump_and_settle();

            let bottom_sheet = only(tester.find_by_type::<BottomSheet>());
            tester.widget(bottom_sheet, |bottom_sheet: &BottomSheet| {
                assert_eq!(bottom_sheet.background_color, Some(color));
                assert_eq!(bottom_sheet.elevation, Some(elevation));
                assert!(**bottom_sheet.shape.as_ref().expect("a shape") == *shape);
                assert_eq!(bottom_sheet.clip_behavior, Some(clip_behavior));
            });
        });
    }

    /// Dart's 'PersistentBottomSheetController.close dismisses the bottom sheet'.
    #[test]
    fn closing_the_persistent_bottom_sheet_controller_dismisses_the_sheet() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            tester.pump_widget(app_with_scaffold(m3(&tester), &scaffold_key));

            let scaffold = scaffold_state(&tester, &scaffold_key);
            let bottom_sheet = Persistent::default().show(
                &tester,
                scaffold,
                Rc::new(|_app, _context| {
                    Builder::new(|_app, _context| Container::new().height(200.0).into_widget())
                        .into_widget()
                }),
            );

            tester.pump(Duration::ZERO);
            assert_eq!(tester.find_by_type::<BottomSheet>().len(), 1);

            bottom_sheet.close(&mut tester.cell.borrow_mut());
            tester.pump(Duration::ZERO);
            assert!(tester.find_by_type::<BottomSheet>().is_empty());
        });
    }

    // persistent_bottom_sheet_test.dart's 'keyboard inset' group.

    /// The Material of the `index`th `BottomSheet` in tree order: Dart's
    /// `find.descendant(of: sheet, matching: find.byType(Material)).first`.
    fn material_of_sheet(tester: &WidgetTester, index: usize) -> Rect {
        let sheet = tester.find_by_type::<BottomSheet>()[index];
        let material = tester.descendants_of_type::<Material>(sheet)[0];
        tester.global_rect(tester.render_box_of(material))
    }

    // The Scaffold's persistent bottom sheet is _StandardBottomSheet ->
    // BottomSheet -> Material. This finds that Material.
    fn inset_sheet_material(tester: &WidgetTester) -> Rect {
        material_of_sheet(tester, 0)
    }

    /// Dart's `buildApp` of the group.
    struct KeyboardInsetApp {
        keyboard_height: f64,
        resize_to_avoid_bottom_inset: bool,
        bottom_navigation_bar: Option<WidgetRef>,
        floating_action_button: Option<WidgetRef>,
        floating_action_button_location: Option<crate::FloatingActionButtonLocationRef>,
        bottom_sheet: Option<WidgetRef>,
    }

    impl KeyboardInsetApp {
        fn new(keyboard_height: f64) -> KeyboardInsetApp {
            KeyboardInsetApp {
                keyboard_height,
                resize_to_avoid_bottom_inset: true,
                bottom_navigation_bar: None,
                floating_action_button: None,
                floating_action_button_location: None,
                bottom_sheet: None,
            }
        }

        /// The default sheet: `SizedBox(key: Key('content'), width: double.infinity,
        /// height: 100.0, child: Text('sheet'))`.
        fn content(content_key: &GlobalKey) -> WidgetRef {
            SizedBox::new()
                .key(content_key.clone().into())
                .width(f64::INFINITY)
                .height(100.0)
                .child(text("sheet"))
                .into_widget()
        }

        fn build(self, tester: &WidgetTester, content_key: &GlobalKey) -> WidgetRef {
            let mut scaffold = Scaffold::new()
                .resize_to_avoid_bottom_inset(self.resize_to_avoid_bottom_inset)
                .bottom_sheet(
                    self.bottom_sheet
                        .unwrap_or_else(|| KeyboardInsetApp::content(content_key)),
                );
            if let Some(bottom_navigation_bar) = self.bottom_navigation_bar {
                scaffold = scaffold.bottom_navigation_bar(bottom_navigation_bar);
            }
            if let Some(floating_action_button) = self.floating_action_button {
                scaffold = scaffold.floating_action_button(floating_action_button);
            }
            if let Some(location) = self.floating_action_button_location {
                scaffold = scaffold.floating_action_button_location(location);
            }
            material_app(
                m3(tester),
                MediaQuery::new(
                    waft_widgets::MediaQueryData::new().view_insets(EdgeInsets::only(
                        0.0,
                        0.0,
                        0.0,
                        self.keyboard_height,
                    )),
                    scaffold,
                )
                .into_widget(),
            )
        }
    }

    /// `MaterialApp(home: MediaQuery(viewInsets: bottom, child: Scaffold(key: scaffoldKey,
    /// body: const SizedBox.expand())))`.
    fn keyboard_app_with_scaffold(
        tester: &WidgetTester,
        keyboard_height: f64,
        scaffold_key: &GlobalKey,
    ) -> WidgetRef {
        material_app(
            m3(tester),
            MediaQuery::new(
                waft_widgets::MediaQueryData::new().view_insets(EdgeInsets::only(
                    0.0,
                    0.0,
                    0.0,
                    keyboard_height,
                )),
                Scaffold::new()
                    .key(scaffold_key.clone().into())
                    .body(SizedBox::expand()),
            )
            .into_widget(),
        )
    }

    /// `SizedBox(height: height, child: Text('sheet'))` as a sheet builder.
    fn sheet_of_height(height: f64) -> WidgetBuilder {
        Rc::new(move |_app, _context| {
            SizedBox::new()
                .height(height)
                .child(text("sheet"))
                .into_widget()
        })
    }

    fn docked_fab() -> WidgetRef {
        crate::floating_action_button::FloatingActionButton::new(Some(
            waft_foundation::Listener::new(|_| {}),
        ))
        .child(waft_widgets::Icon::new(Some(crate::icons::Icons::add())))
        .into_widget()
    }

    #[test]
    fn the_sheet_material_extends_behind_the_keyboard_and_the_content_stays_above() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let content = GlobalKey::new();
            tester.pump_widget(KeyboardInsetApp::new(0.0).build(&tester, &content));
            assert_eq!(rect_of_key(&tester, &content).top, 500.0);
            assert_eq!(rect_of_key(&tester, &content).bottom, 600.0);
            assert_eq!(inset_sheet_material(&tester).top, 500.0);
            assert_eq!(inset_sheet_material(&tester).bottom, 600.0);

            tester.pump_widget(KeyboardInsetApp::new(200.0).build(&tester, &content));
            // Content avoids the keyboard exactly as before.
            assert_eq!(rect_of_key(&tester, &content).top, 300.0);
            assert_eq!(rect_of_key(&tester, &content).bottom, 400.0);
            // The sheet's own Material now continues to the bottom of the screen.
            assert_eq!(inset_sheet_material(&tester).top, 300.0);
            assert_eq!(inset_sheet_material(&tester).bottom, 600.0);
        });
    }

    #[test]
    fn resize_to_avoid_bottom_inset_false_leaves_the_sheet_unchanged() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let content = GlobalKey::new();
            let mut app = KeyboardInsetApp::new(200.0);
            app.resize_to_avoid_bottom_inset = false;
            tester.pump_widget(app.build(&tester, &content));
            assert_eq!(rect_of_key(&tester, &content).top, 500.0);
            assert_eq!(rect_of_key(&tester, &content).bottom, 600.0);
            assert_eq!(inset_sheet_material(&tester).top, 500.0);
            assert_eq!(inset_sheet_material(&tester).bottom, 600.0);
        });
    }

    #[test]
    fn the_sheet_material_stops_above_a_bottom_navigation_bar() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let content = GlobalKey::new();
            let mut app = KeyboardInsetApp::new(200.0);
            app.bottom_navigation_bar = Some(SizedBox::new().height(50.0).into_widget());
            tester.pump_widget(app.build(&tester, &content));
            assert_eq!(rect_of_key(&tester, &content).top, 300.0);
            assert_eq!(rect_of_key(&tester, &content).bottom, 400.0);
            // Extends down to the top of the bottom navigation bar, not past it.
            assert_eq!(inset_sheet_material(&tester).top, 300.0);
            assert_eq!(inset_sheet_material(&tester).bottom, 550.0);
        });
    }

    #[test]
    fn a_docked_floating_action_button_straddles_the_top_edge_of_the_sheet_content() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let content = GlobalKey::new();
            let fab_center_y = |tester: &WidgetTester| {
                let fab =
                    tester.find_by_type::<crate::floating_action_button::FloatingActionButton>()[0];
                tester.global_rect(tester.render_box_of(fab)).center().dy()
            };
            let app = |keyboard_height: f64| {
                let mut app = KeyboardInsetApp::new(keyboard_height);
                app.floating_action_button = Some(docked_fab());
                app.floating_action_button_location =
                    Some(<dyn crate::FloatingActionButtonLocation>::center_docked());
                app
            };

            tester.pump_widget(app(0.0).build(&tester, &content));
            assert_eq!(fab_center_y(&tester), 500.0);

            tester.pump_widget(app(200.0).build(&tester, &content));
            tester.pump_and_settle();
            // Top of the content (300), not the top of the extended Material.
            assert_eq!(fab_center_y(&tester), 300.0);
        });
    }

    #[test]
    fn the_keyboard_appearing_keeps_the_sheet_contents_state_and_focus() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let content = GlobalKey::new();
            let focus_node = waft_widgets::FocusNode::new(&mut tester.cell.borrow_mut());
            let sheet = || {
                SizedBox::new()
                    .height(100.0)
                    .child(crate::text_field::TextField::new().focus_node(focus_node.erase()))
                    .into_widget()
            };
            let app = |keyboard_height: f64| {
                let mut app = KeyboardInsetApp::new(keyboard_height);
                app.bottom_sheet = Some(sheet());
                app
            };
            let has_focus =
                |tester: &WidgetTester| focus_node.has_focus(&mut tester.cell.borrow_mut());

            tester.pump_widget(app(0.0).build(&tester, &content));
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump(Duration::ZERO);
            assert!(has_focus(&tester));
            // The `EditableText`'s element, which holds its state.
            let before = only(tester.find_by_type::<waft_widgets::EditableText>());

            tester.pump_widget(app(200.0).build(&tester, &content));
            tester.pump(Duration::ZERO);
            assert!(has_focus(&tester));
            assert_eq!(
                only(tester.find_by_type::<waft_widgets::EditableText>()),
                before
            );
        });
    }

    #[test]
    fn a_shown_bottom_sheet_can_still_be_flung_closed_with_the_keyboard_up() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            tester.pump_widget(keyboard_app_with_scaffold(&tester, 200.0, &scaffold_key));
            let scaffold = scaffold_state(&tester, &scaffold_key);
            Persistent::default().show(&tester, scaffold, sheet_of_height(100.0));
            tester.pump_and_settle();
            assert_eq!(rect_of_text(&tester, "sheet").bottom, 400.0);
            assert_eq!(inset_sheet_material(&tester).bottom, 600.0);

            fling_text(&mut tester, "sheet", Offset::new(0.0, 300.0), 2000.0);
            tester.pump_and_settle();
            assert_eq!(found(&tester, "sheet"), 0);
        });
    }

    #[test]
    fn a_shown_bottom_sheet_follows_the_drag_with_the_keyboard_up() {
        with_main_thread_stack(|| {
            const KEYBOARD_HEIGHT: f64 = 200.0;
            const CONTENT_HEIGHT: f64 = 100.0;
            const MATERIAL_HEIGHT: f64 = CONTENT_HEIGHT + KEYBOARD_HEIGHT;
            // The content is at the top of the Material, which ends at the bottom of
            // the 600px tall test screen.
            const CONTENT_TOP: f64 = 600.0 - MATERIAL_HEIGHT;
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            tester.pump_widget(keyboard_app_with_scaffold(
                &tester,
                KEYBOARD_HEIGHT,
                &scaffold_key,
            ));
            let scaffold = scaffold_state(&tester, &scaffold_key);
            Persistent::default().show(&tester, scaffold, sheet_of_height(CONTENT_HEIGHT));
            tester.pump_and_settle();
            assert_eq!(rect_of_text(&tester, "sheet").top, CONTENT_TOP);
            assert_eq!(inset_sheet_material(&tester).height(), MATERIAL_HEIGHT);

            // The whole sheet moves with the finger.
            const DRAG_DISTANCE: f64 = 100.0;
            let mut gesture = WidgetTester::touch(1);
            let center = center_of_text(&tester, "sheet");
            tester.down(&mut gesture, center);
            let move_by = |tester: &mut WidgetTester,
                           gesture: &mut crate::test_support::TestPointer,
                           dy: f64| {
                let at = gesture.position + Offset::new(0.0, dy);
                tester.move_to(gesture, at, true);
            };
            move_by(&mut tester, &mut gesture, 20.0); // Past the touch slop.
            tester.pump(Duration::ZERO);
            move_by(&mut tester, &mut gesture, DRAG_DISTANCE);
            tester.pump(Duration::ZERO);
            assert_eq!(
                rect_of_text(&tester, "sheet").top,
                CONTENT_TOP + DRAG_DISTANCE
            );

            // More than half of the content, but less than half of the whole sheet,
            // including the part behind the keyboard: springs back open.
            tester.up(&gesture);
            tester.pump_and_settle();
            assert_eq!(rect_of_text(&tester, "sheet").top, CONTENT_TOP);

            // More than half of the whole sheet: closes.
            drag_text(
                &mut tester,
                "sheet",
                Offset::new(0.0, MATERIAL_HEIGHT * 0.75),
            );
            tester.pump_and_settle();
            assert_eq!(found(&tester, "sheet"), 0);
        });
    }

    #[test]
    fn the_max_height_constraint_bounds_the_content_not_the_material_behind_the_keyboard() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let scaffold_key = GlobalKey::new();
            tester.pump_widget(keyboard_app_with_scaffold(&tester, 200.0, &scaffold_key));
            let scaffold = scaffold_state(&tester, &scaffold_key);
            Persistent {
                constraints: Some(BoxConstraints::new().max_height(250.0)),
                ..Persistent::default()
            }
            .show(&tester, scaffold, sheet_of_height(400.0));
            tester.pump_and_settle();
            // The content gets the full 250 it is allowed, above the keyboard.
            assert_eq!(rect_of_text(&tester, "sheet").top, 150.0);
            assert_eq!(rect_of_text(&tester, "sheet").bottom, 400.0);
            // The Material is 250 + 200.
            assert_eq!(inset_sheet_material(&tester).top, 150.0);
            assert_eq!(inset_sheet_material(&tester).bottom, 600.0);
        });
    }

    #[test]
    fn taps_in_the_material_behind_the_keyboard_do_not_reach_the_content() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let content = GlobalKey::new();
            let tap_count = Rc::new(Cell::new(0));
            let mut app = KeyboardInsetApp::new(200.0);
            app.bottom_sheet = Some({
                let tap_count = Rc::clone(&tap_count);
                SizedBox::new()
                    .height(100.0)
                    .child(
                        waft_widgets::GestureDetector::new()
                            .behavior(waft_rendering::HitTestBehavior::Opaque)
                            .on_tap(waft_foundation::Listener::new(move |_| {
                                tap_count.set(tap_count.get() + 1)
                            }))
                            .child(text("sheet")),
                    )
                    .into_widget()
            });
            tester.pump_widget(app.build(&tester, &content));
            assert_eq!(rect_of_text(&tester, "sheet").bottom, 400.0);
            assert_eq!(inset_sheet_material(&tester).bottom, 600.0);

            // Inside the content.
            tester.tap(Offset::new(400.0, 350.0), 1);
            assert_eq!(tap_count.get(), 1);

            // In the inset region, which only the keyboard can reach: the sheet's
            // Material absorbs the tap (as the Scaffold's own Material did before it
            // was covered), and the content is not hit.
            tester.tap(Offset::new(400.0, 500.0), 2);
            assert_eq!(tap_count.get(), 1);
        });
    }

    #[test]
    fn a_scaffold_shorter_than_the_keyboard_inset_keeps_the_sheet_inside_its_bounds() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let content = GlobalKey::new();
            // A 250px tall Scaffold centered on the 600px screen, so it spans y 175..425.
            let nested = |tester: &WidgetTester, keyboard_height: f64| {
                let theme = ThemeData::builder()
                    .bottom_sheet_theme(
                        BottomSheetThemeData::new()
                            .constraints(BoxConstraints::tight_for(None, Some(150.0))),
                    )
                    .build(&tester.cell.borrow());
                material_app(
                    theme,
                    MediaQuery::new(
                        waft_widgets::MediaQueryData::new().view_insets(EdgeInsets::only(
                            0.0,
                            0.0,
                            0.0,
                            keyboard_height,
                        )),
                        Center::new().child(
                            SizedBox::new().width(320.0).height(250.0).child(
                                Scaffold::new()
                                    .body(SizedBox::expand())
                                    .bottom_sheet(SizedBox::expand().key(content.clone().into())),
                            ),
                        ),
                    )
                    .into_widget(),
                )
            };

            for keyboard_height in [0.0, 200.0, 400.0, 0.0] {
                tester.pump_widget(nested(&tester, keyboard_height));
                tester.pump_and_settle();
                // The content gets whatever is left above the keyboard, at most the
                // theme's 150, exactly as before.
                assert_eq!(
                    rect_of_key(&tester, &content).height(),
                    (250.0 - keyboard_height).clamp(0.0, 150.0)
                );
                // The Material always ends at the Scaffold's bottom edge and never
                // leaves the Scaffold, without any clipping.
                assert_eq!(inset_sheet_material(&tester).bottom, 425.0);
                assert!(inset_sheet_material(&tester).top >= 175.0);
            }
        });
    }

    #[test]
    fn a_bottom_sheet_given_as_the_scaffolds_bottom_sheet_is_not_extended_twice() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let content = GlobalKey::new();
            let mut app = KeyboardInsetApp::new(200.0);
            app.bottom_sheet = Some({
                let content = content.clone();
                BottomSheet::new(
                    Rc::new(|_app: &mut App| {}),
                    Rc::new(move |_app, _context| {
                        SizedBox::new()
                            .key(content.clone().into())
                            .width(f64::INFINITY)
                            .height(100.0)
                            .into_widget()
                    }),
                )
                .enable_drag(false)
                .into_widget()
            });
            tester.pump_widget(app.build(&tester, &content));
            assert_eq!(tester.find_by_type::<BottomSheet>().len(), 2);

            // Only the Scaffold's own sheet extends behind the keyboard. The user's
            // sheet, and its Material, end with the content.
            assert_eq!(rect_of_key(&tester, &content).bottom, 400.0);
            assert_eq!(material_of_sheet(&tester, 0).bottom, 600.0);
            assert_eq!(material_of_sheet(&tester, 1).bottom, 400.0);
        });
    }

    // bottom_sheet_theme_test.dart

    fn bottom_sheet_in_scaffold(theme: ThemeData, sheet: BottomSheet) -> WidgetRef {
        material_app(theme, Scaffold::new().body(sheet).into_widget())
    }

    fn plain_sheet() -> BottomSheet {
        BottomSheet::new(
            Rc::new(|_app| {}),
            Rc::new(|_app, _context| Container::new().into_widget()),
        )
    }

    /// Dart's `_bottomSheetTheme()`.
    fn orange_bottom_sheet_theme() -> BottomSheetThemeData {
        BottomSheetThemeData::new()
            .background_color(Colors::ORANGE.primary())
            .elevation(12.0)
            .shape(waft_painting::BeveledRectangleBorder::new(
                waft_painting::BorderSide::NONE,
                waft_painting::BorderRadiusGeometry::all(waft_embedder::Radius::circular(12.0)),
            ))
            .clip_behavior(Clip::AntiAlias)
    }

    fn shape_eq(a: Option<&dyn ShapeBorder>, b: Option<&dyn ShapeBorder>) -> bool {
        match (a, b) {
            (Some(a), Some(b)) => *a == *b,
            (None, None) => true,
            _ => false,
        }
    }

    /// Dart's 'Material3 - Passing no BottomSheetThemeData returns defaults'.
    #[test]
    fn material3_passing_no_bottom_sheet_theme_returns_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = m3(&tester);
            let surface_container_low = theme.color_scheme.surface_container_low();
            tester.pump_widget(bottom_sheet_in_scaffold(theme, plain_sheet()));

            let default_shape: ShapeBorderRef =
                ShapeBorderRef::new(waft_painting::RoundedRectangleBorder::new(
                    waft_painting::BorderSide::NONE,
                    waft_painting::BorderRadiusGeometry::vertical(
                        waft_embedder::Radius::circular(28.0),
                        waft_embedder::Radius::ZERO,
                    ),
                ));
            tester.widget(sheet_material(&tester), |material: &Material| {
                assert_eq!(material.color, Some(surface_container_low));
                assert_eq!(material.elevation, 1.0);
                assert!(shape_eq(material.shape.as_deref(), Some(&*default_shape)));
                assert_eq!(material.clip_behavior, Clip::None);
            });
        });
    }

    /// Dart's 'Material2 - Passing no BottomSheetThemeData returns defaults'.
    #[test]
    fn material2_passing_no_bottom_sheet_theme_returns_defaults() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(bottom_sheet_in_scaffold(m2(&tester), plain_sheet()));

            tester.widget(sheet_material(&tester), |material: &Material| {
                assert_eq!(material.color, None);
                assert_eq!(material.elevation, 0.0);
                assert!(material.shape.is_none());
                assert_eq!(material.clip_behavior, Clip::None);
            });
        });
    }

    /// Dart's 'BottomSheet uses values from BottomSheetThemeData'.
    #[test]
    fn a_bottom_sheet_uses_values_from_the_bottom_sheet_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let bottom_sheet_theme = orange_bottom_sheet_theme();
            let theme = ThemeData::builder()
                .bottom_sheet_theme(bottom_sheet_theme.clone())
                .build(&tester.cell.borrow());
            tester.pump_widget(bottom_sheet_in_scaffold(theme, plain_sheet()));

            tester.widget(sheet_material(&tester), |material: &Material| {
                assert_eq!(material.color, bottom_sheet_theme.background_color);
                assert_eq!(Some(material.elevation), bottom_sheet_theme.elevation);
                assert!(shape_eq(
                    material.shape.as_deref(),
                    bottom_sheet_theme.shape.as_deref()
                ));
                assert_eq!(
                    Some(material.clip_behavior),
                    bottom_sheet_theme.clip_behavior
                );
            });
        });
    }

    /// Dart's 'BottomSheet widget properties take priority over theme'.
    #[test]
    fn bottom_sheet_widget_properties_take_priority_over_the_theme() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let background_color = Colors::PURPLE.primary();
            let shadow_color = Colors::BLUE.primary();
            let elevation = 7.0;
            let shape: ShapeBorderRef =
                ShapeBorderRef::new(waft_painting::RoundedRectangleBorder::new(
                    waft_painting::BorderSide::NONE,
                    waft_painting::BorderRadiusGeometry::all(waft_embedder::Radius::circular(9.0)),
                ));
            let clip_behavior = Clip::HardEdge;

            let theme = ThemeData::builder()
                .bottom_sheet_theme(orange_bottom_sheet_theme())
                .build(&tester.cell.borrow());
            tester.pump_widget(bottom_sheet_in_scaffold(
                theme,
                plain_sheet()
                    .background_color(background_color)
                    .shadow_color(shadow_color)
                    .elevation(elevation)
                    .shape(shape.clone())
                    .clip_behavior(Clip::HardEdge),
            ));

            tester.widget(sheet_material(&tester), |material: &Material| {
                assert_eq!(material.color, Some(background_color));
                assert_eq!(material.shadow_color, Some(shadow_color));
                assert_eq!(material.elevation, elevation);
                assert!(shape_eq(material.shape.as_deref(), Some(&*shape)));
                assert_eq!(material.clip_behavior, clip_behavior);
            });
        });
    }

    /// Dart's `bottomSheetWithElevations`: a scaffold whose body's context the case shows
    /// sheets from, where Dart's `RawMaterialButton`s show them.
    fn bottom_sheet_with_elevations(
        tester: &WidgetTester,
        bottom_sheet_theme: BottomSheetThemeData,
        use_material3: bool,
        saved: &Saved,
    ) -> WidgetRef {
        let theme = ThemeData::builder()
            .bottom_sheet_theme(bottom_sheet_theme)
            .use_material3(use_material3)
            .build(&tester.cell.borrow());
        material_app(
            theme,
            Scaffold::new()
                .body(saved.builder(|| Container::new().into_widget()))
                .into_widget(),
        )
    }

    /// Dart's 'Show Modal' button.
    fn show_modal_from(tester: &WidgetTester, saved: &Saved) {
        Modal::default().show(
            tester,
            saved.get(),
            sheet_builder("This is a modal bottom sheet."),
        );
    }

    /// Dart's 'Show Persistent' button.
    fn show_persistent_from(tester: &WidgetTester, saved: &Saved) {
        show_bottom_sheet(
            &mut tester.cell.borrow_mut(),
            saved.get(),
            sheet_builder("This is a persistent bottom sheet."),
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
    }

    /// Dart's 'Modal bottom sheet-specific parameters are used for modal bottom sheets'.
    #[test]
    fn modal_specific_theme_parameters_are_used_for_modal_bottom_sheets() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved = Saved::default();
            let modal_elevation = 5.0;
            let persistent_elevation = 7.0;
            let modal_background_color = Colors::YELLOW.primary();
            let modal_barrier_color = Colors::BLUE.primary();
            let persistent_background_color = Colors::RED.primary();
            let bottom_sheet_theme = BottomSheetThemeData::new()
                .elevation(persistent_elevation)
                .modal_elevation(modal_elevation)
                .background_color(persistent_background_color)
                .modal_background_color(modal_background_color)
                .modal_barrier_color(modal_barrier_color);

            tester.pump_widget(bottom_sheet_with_elevations(
                &tester,
                bottom_sheet_theme,
                true,
                &saved,
            ));
            show_modal_from(&tester, &saved);
            tester.pump_and_settle();

            tester.widget(sheet_material(&tester), |material: &Material| {
                assert_eq!(material.elevation, modal_elevation);
                assert_eq!(material.color, Some(modal_background_color));
            });

            let modal_barrier = *tester
                .find_by_type::<ModalBarrier>()
                .last()
                .expect("a modal barrier");
            tester.widget(modal_barrier, |modal_barrier: &ModalBarrier| {
                assert_eq!(modal_barrier.color, Some(modal_barrier_color));
            });
        });
    }

    /// Dart's 'General bottom sheet parameters take priority over modal bottom
    /// sheet-specific parameters for persistent bottom sheets'.
    #[test]
    fn general_theme_parameters_win_over_modal_ones_for_persistent_bottom_sheets() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved = Saved::default();
            let modal_elevation = 5.0;
            let persistent_elevation = 7.0;
            let modal_background_color = Colors::YELLOW.primary();
            let persistent_background_color = Colors::RED.primary();
            let bottom_sheet_theme = BottomSheetThemeData::new()
                .elevation(persistent_elevation)
                .modal_elevation(modal_elevation)
                .background_color(persistent_background_color)
                .modal_background_color(modal_background_color);

            tester.pump_widget(bottom_sheet_with_elevations(
                &tester,
                bottom_sheet_theme,
                true,
                &saved,
            ));
            show_persistent_from(&tester, &saved);
            tester.pump_and_settle();

            tester.widget(sheet_material(&tester), |material: &Material| {
                assert_eq!(material.elevation, persistent_elevation);
                assert_eq!(material.color, Some(persistent_background_color));
            });
        });
    }

    /// Dart's "Material3 - Modal bottom sheet-specific parameters don't apply to persistent
    /// bottom sheets".
    #[test]
    fn material3_modal_theme_parameters_do_not_apply_to_persistent_bottom_sheets() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved = Saved::default();
            let bottom_sheet_theme = BottomSheetThemeData::new()
                .modal_elevation(5.0)
                .modal_background_color(Colors::YELLOW.primary());

            tester.pump_widget(bottom_sheet_with_elevations(
                &tester,
                bottom_sheet_theme,
                true,
                &saved,
            ));
            show_persistent_from(&tester, &saved);
            tester.pump_and_settle();

            let surface_container_low = Theme::of(&mut tester.cell.borrow_mut(), saved.get())
                .color_scheme
                .surface_container_low();
            tester.widget(sheet_material(&tester), |material: &Material| {
                assert_eq!(material.elevation, 1.0);
                assert_eq!(material.color, Some(surface_container_low));
            });
        });
    }

    /// Dart's "Material2 - Modal bottom sheet-specific parameters don't apply to persistent
    /// bottom sheets".
    #[test]
    fn material2_modal_theme_parameters_do_not_apply_to_persistent_bottom_sheets() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved = Saved::default();
            let bottom_sheet_theme = BottomSheetThemeData::new()
                .modal_elevation(5.0)
                .modal_background_color(Colors::YELLOW.primary());

            tester.pump_widget(bottom_sheet_with_elevations(
                &tester,
                bottom_sheet_theme,
                false,
                &saved,
            ));
            show_persistent_from(&tester, &saved);
            tester.pump_and_settle();

            tester.widget(sheet_material(&tester), |material: &Material| {
                assert_eq!(material.elevation, 0.0);
                assert_eq!(material.color, None);
            });
        });
    }

    /// Dart's 'Throw if enable drag without an animation controller'. Dart catches the
    /// `FlutterError` the drag reports; here the debug assertion unwinds out of the drag.
    #[test]
    #[should_panic(expected = "'BottomSheet.animationController' cannot be null")]
    fn dragging_a_draggable_bottom_sheet_without_an_animation_controller_fails() {
        with_main_thread_stack(|| {
            // Regression test for https://github.com/flutter/flutter/issues/89168
            let mut tester = tester();
            tester.pump_widget(material_app(
                m3(&tester),
                BottomSheet::new(
                    Rc::new(|_app| {}),
                    Rc::new(|_app, _context| {
                        Container::new()
                            .height(200.0)
                            .color(Colors::RED.primary())
                            .child(text("BottomSheet"))
                            .into_widget()
                    }),
                )
                .into_widget(),
            ));

            drag_text(&mut tester, "BottomSheet", Offset::new(0.0, 150.0));
        });
    }

    /// Not a Dart case: what Dart's collector does with the curved animations and the proxy
    /// `_ModalBottomSheetState` makes, and with the drag-end curve each drag replaces; each is
    /// freed once nothing holds it.
    #[test]
    fn a_modal_bottom_sheet_frees_its_animations_as_it_disposes_them() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let saved = Saved::default();
            tester.pump_widget(app_saving_context(&tester, &saved));
            tester.pump(Duration::ZERO);
            drop(Modal::default().show(&tester, saved.get(), sheet_builder("BottomSheet")));
            tester.pump_and_settle();
            let state = |tester: &WidgetTester| {
                let sheet = tester.find_by_type::<ModalBottomSheet>()[0];
                let app = tester.cell.borrow();
                sheet
                    .state_handle::<ModalBottomSheetState>(&app)
                    .expect("a ModalBottomSheet is stateful")
            };
            let drag_end_curve = |tester: &WidgetTester| {
                let state = state(tester);
                tester
                    .cell
                    .borrow()
                    .get(state)
                    .drag_end_curved_animation
                    .as_ref()
                    .expect("a drag ended")
                    .handle()
            };
            let (curved, proxy) = {
                let state = state(&tester);
                let app = tester.cell.borrow();
                (
                    state.curved_sheet_animation(&app),
                    state.sheet_animation(&app),
                )
            };

            drag_text(&mut tester, "BottomSheet", Offset::new(0.0, 20.0));
            tester.pump_and_settle();
            let first_drag_end = drag_end_curve(&tester);
            drag_text(&mut tester, "BottomSheet", Offset::new(0.0, 20.0));
            tester.pump_and_settle();
            let second_drag_end = drag_end_curve(&tester);
            tester.cell.checkpoint();
            assert!(!tester.cell.borrow().contains(first_drag_end));
            assert!(tester.cell.borrow().contains(second_drag_end));

            drag_text(&mut tester, "BottomSheet", Offset::new(0.0, 150.0));
            tester.pump_and_settle();
            assert_eq!(found(&tester, "BottomSheet"), 0);
            tester.cell.checkpoint();
            let app = tester.cell.borrow();
            assert!(!app.contains(second_drag_end));
            assert!(!app.contains(curved));
            assert!(!app.contains(proxy));
        });
    }
}
