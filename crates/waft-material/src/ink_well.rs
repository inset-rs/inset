//! Flutter counterpart: material_ui's `ink_well.dart`.
//!
//! Dart's file types `MaterialStatesController`, and its features read `Color.alpha`; both
//! are deprecated, and kept.
#![allow(deprecated)]

use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::Hash;
use std::rc::Rc;
use std::time::Duration;

use waft_embedder::{Canvas, Color, Matrix4, Offset, Paint, RRect, Rect, TextDirection};
use waft_foundation::{
    App, Handle, HandleId, Listenable, ListenableObject, Listener, OptionRetained, Ref, Retained,
    Timer, ValueChanged,
};
use waft_gestures::{
    GestureLongPressCallback, GestureLongPressUpCallback, GestureTapCallback,
    GestureTapCancelCallback, GestureTapDownCallback, GestureTapUpCallback, PointerEnterEvent,
    PointerExitEvent, TapDownDetails, TapUpDetails,
};
use waft_painting::{BorderRadius, BoxShape, ShapeBorder, ShapeBorderRef, get_as_translation};
use waft_rendering::{AnyRenderBox, HitTestBehavior};
use waft_services::MouseCursorRef;
use waft_widgets::{
    ActionRef, Actions, ActivateIntent, AutomaticKeepAliveClientMixin,
    AutomaticKeepAliveClientMixinData, BuildContext, ButtonActivateIntent, CallbackAction,
    DefaultSelectionStyle, Directionality, Feedback, Focus, FocusHighlightMode, FocusManager,
    FocusNodeRef, GestureDetector, HighlightModeListener, InheritedWidget, IntoWidget, KeyRef,
    MediaQuery, MouseRegion, NavigationMode, Semantics, State, StateData, StatefulWidget,
    StatelessWidget, WidgetRef, WidgetState, WidgetStateMouseCursor, WidgetStateProperty,
    WidgetStatePropertyRef, WidgetStates, debug_check_has_directionality,
};

use crate::debug::debug_check_has_material;
use crate::ink_highlight::InkHighlight;
use crate::material::{
    AnyInkFeature, AnyMaterialInkController, InkFeatureLeaf, InkFeatureLeafData, InkFeatureObject,
    Material, RectCallback,
};
use crate::material_state::MaterialStatesController;
use crate::theme::Theme;

/// The fields of Dart's `InteractiveInkFeature`, which every interactive ink feature leaf
/// carries beside its [`InkFeatureData`](crate::InkFeatureData).
pub struct InteractiveInkFeatureData {
    color: Color,
    custom_border: Option<ShapeBorderRef>,
}

impl InteractiveInkFeatureData {
    /// Creates an InteractiveInkFeature's fields.
    pub fn new(color: Color, custom_border: Option<ShapeBorderRef>) -> InteractiveInkFeatureData {
        InteractiveInkFeatureData {
            color,
            custom_border,
        }
    }

    /// The custom border, borrowed.
    pub(crate) fn custom_border_ref(&self) -> Option<&dyn ShapeBorder> {
        self.custom_border.as_deref()
    }
}

/// The [`InteractiveInkFeatureData`] bag of every interactive ink feature leaf.
pub trait InteractiveInkFeatureLeafData: InkFeatureLeafData {
    /// The leaf's [`InteractiveInkFeature`] bag.
    fn interactive_ink_feature(&self) -> &InteractiveInkFeatureData;

    /// The leaf's [`InteractiveInkFeature`] bag, mutably.
    fn interactive_ink_feature_mut(&mut self) -> &mut InteractiveInkFeatureData;
}

/// The virtuals of Dart's abstract `InteractiveInkFeature`: an ink feature that displays a
/// [`color`](InteractiveInkFeature::color) "splash" in response to a user gesture that can be
/// confirmed or canceled.
///
/// Subclasses call [`confirm`](Self::confirm) when an input gesture is recognized. For
/// example a press event might trigger an ink feature that's confirmed
/// when the corresponding up event is seen.
///
/// Subclasses call [`cancel`](Self::cancel) when an input gesture is aborted before it
/// is recognized. For example a press event might trigger an ink feature
/// that's canceled when the pointer is dragged out of the reference
/// box.
///
/// The [`InkWell`] and [`InkResponse`] widgets generate instances of this
/// class.
pub trait InteractiveInkFeatureLeaf: InkFeatureLeaf + InteractiveInkFeatureLeafData {
    /// Called when the user input that triggered this feature's appearance was confirmed.
    ///
    /// Typically causes the ink to propagate faster across the material. By default this
    /// method does nothing.
    fn confirm(self: Handle<Self>, app: &mut App) {
        let _ = (self, app);
    }

    /// Called when the user input that triggered this feature's appearance was canceled.
    ///
    /// Typically causes the ink to gradually disappear. By default this method does
    /// nothing.
    fn cancel(self: Handle<Self>, app: &mut App) {
        let _ = (self, app);
    }
}

/// Mints the erased [`AnyInteractiveInkFeature`] of a leaf.
pub trait InteractiveInkFeatureObject: Sized + 'static {
    /// This feature as the erased [`AnyInteractiveInkFeature`] — what to pass where a Dart API
    /// takes an `InteractiveInkFeature`.
    fn as_interactive_ink_feature(self: Handle<Self>) -> AnyInteractiveInkFeature;
}

/// What a field Dart types as `InteractiveInkFeature` holds: an ink response's splashes.
pub type InteractiveInkFeatureRef = Retained<AnyInteractiveInkFeature>;

waft_foundation::erased_handle!(
    AnyInteractiveInkFeature: InteractiveInkFeatureLeaf
        => InteractiveInkFeatureObject::as_interactive_ink_feature
);

impl<F: InteractiveInkFeatureLeaf> InteractiveInkFeatureObject for F {
    fn as_interactive_ink_feature(self: Handle<Self>) -> AnyInteractiveInkFeature {
        AnyInteractiveInkFeature {
            id: self.id(),
            vtable: const { &InteractiveInkFeatureVTable::of::<F>() },
        }
    }
}

/// The vtable of an erased [`AnyInteractiveInkFeature`]: one `&'static` table per leaf type.
struct InteractiveInkFeatureVTable {
    type_name: fn() -> &'static str,
    as_ink_feature: fn(HandleId) -> AnyInkFeature,
    confirm: fn(&mut App, HandleId),
    cancel: fn(&mut App, HandleId),
    color: fn(&App, HandleId) -> Color,
    set_color: fn(&mut App, HandleId, Color),
    custom_border: fn(&App, HandleId) -> Option<ShapeBorderRef>,
    set_custom_border: fn(&mut App, HandleId, Option<ShapeBorderRef>),
}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<T: 'static>(id: HandleId) -> Handle<T> {
    Handle::from_id(id)
}

impl InteractiveInkFeatureVTable {
    const fn of<F: InteractiveInkFeatureLeaf>() -> InteractiveInkFeatureVTable {
        InteractiveInkFeatureVTable {
            type_name: std::any::type_name::<F>,
            as_ink_feature: |id| resolve::<F>(id).as_ink_feature(),
            confirm: |app, id| F::confirm(resolve(id), app),
            cancel: |app, id| F::cancel(resolve(id), app),
            color: |app, id| InteractiveInkFeature::color(resolve::<F>(id), app),
            set_color: |app, id, value| {
                InteractiveInkFeature::set_color(resolve::<F>(id), app, value)
            },
            custom_border: |app, id| InteractiveInkFeature::custom_border(resolve::<F>(id), app),
            set_custom_border: |app, id, value| {
                InteractiveInkFeature::set_custom_border(resolve::<F>(id), app, value)
            },
        }
    }
}

/// Erased `InteractiveInkFeature`: one identity and a static vtable.
///
/// Equality is Dart's `==` on an object reference.
#[derive(Clone, Copy)]
pub struct AnyInteractiveInkFeature {
    id: HandleId,
    vtable: &'static InteractiveInkFeatureVTable,
}

impl fmt::Debug for AnyInteractiveInkFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl AnyInteractiveInkFeature {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// Dart's `feature as T`: the typed handle when this feature is a `T`, else `None`.
    pub fn downcast<T: 'static>(self, app: &App) -> Option<Handle<T>> {
        app.handle::<T>(self.id)
    }

    /// This feature as the [`AnyInkFeature`] it is.
    pub fn as_ink_feature(self) -> AnyInkFeature {
        (self.vtable.as_ink_feature)(self.id)
    }

    /// See [`InteractiveInkFeatureLeaf::confirm`].
    pub fn confirm(self, app: &mut App) {
        (self.vtable.confirm)(app, self.id);
    }

    /// See [`InteractiveInkFeatureLeaf::cancel`].
    pub fn cancel(self, app: &mut App) {
        (self.vtable.cancel)(app, self.id);
    }

    /// See [`InkFeatureLeaf::dispose`].
    pub fn dispose(self, app: &mut App) {
        self.as_ink_feature().dispose(app);
    }

    /// See [`InteractiveInkFeature::color`].
    pub fn color(self, app: &App) -> Color {
        (self.vtable.color)(app, self.id)
    }

    /// See [`InteractiveInkFeature::set_color`].
    pub fn set_color(self, app: &mut App, value: Color) {
        (self.vtable.set_color)(app, self.id, value);
    }

    /// See [`InteractiveInkFeature::custom_border`].
    pub fn custom_border(self, app: &App) -> Option<ShapeBorderRef> {
        (self.vtable.custom_border)(app, self.id)
    }

    /// See [`InteractiveInkFeature::set_custom_border`].
    pub fn set_custom_border(self, app: &mut App, value: Option<ShapeBorderRef>) {
        (self.vtable.set_custom_border)(app, self.id, value);
    }
}

/// Dart's abstract `InteractiveInkFeature`: an ink feature that displays a color "splash" in
/// response to a user gesture that can be confirmed or canceled.
///
/// Its bodies are associated fns taking the leaf's `Handle`; its virtuals are
/// [`InteractiveInkFeatureLeaf`]'s, and its fields are [`InteractiveInkFeatureData`].
pub struct InteractiveInkFeature;

impl InteractiveInkFeature {
    /// The ink's color.
    pub fn color<F: InteractiveInkFeatureLeaf>(this: Handle<F>, app: &App) -> Color {
        app.get(this).interactive_ink_feature().color
    }

    /// Sets [`color`](Self::color).
    pub fn set_color<F: InteractiveInkFeatureLeaf>(this: Handle<F>, app: &mut App, value: Color) {
        if value == InteractiveInkFeature::color(this, app) {
            return;
        }
        app.get_mut(this).interactive_ink_feature_mut().color = value;
        crate::material::InkFeature::controller(this, app).mark_needs_paint(app);
    }

    /// The ink's optional custom border.
    pub fn custom_border<F: InteractiveInkFeatureLeaf>(
        this: Handle<F>,
        app: &App,
    ) -> Option<ShapeBorderRef> {
        app.get(this)
            .interactive_ink_feature()
            .custom_border
            .clone()
    }

    /// Sets [`custom_border`](Self::custom_border).
    pub fn set_custom_border<F: InteractiveInkFeatureLeaf>(
        this: Handle<F>,
        app: &mut App,
        value: Option<ShapeBorderRef>,
    ) {
        if value == app.get(this).interactive_ink_feature().custom_border {
            return;
        }
        app.get_mut(this)
            .interactive_ink_feature_mut()
            .custom_border = value;
        crate::material::InkFeature::controller(this, app).mark_needs_paint(app);
    }

    /// Draws an ink splash or ink ripple on the passed in [`Canvas`].
    ///
    /// The `transform` argument is the [`Matrix4`] transform that typically
    /// shifts the coordinate space of the canvas to the space in which
    /// the ink circle is to be painted.
    ///
    /// `center` is the [`Offset`] from origin of the canvas where the center
    /// of the circle is drawn.
    ///
    /// `paint` takes a [`Paint`] object that describes the styles used to draw the ink circle.
    /// For example, `paint` can specify properties like color, strokewidth, colorFilter.
    ///
    /// `radius` is the radius of ink circle to be drawn on canvas.
    ///
    /// `clip_callback` is the callback used to obtain the [`Rect`] used for clipping the ink
    /// effect. If `clip_callback` is null, no clipping is performed on the ink circle.
    ///
    /// Clipping can happen in 3 different ways:
    ///  1. If `custom_border` is provided, it is used to determine the path
    ///     for clipping.
    ///  2. If `custom_border` is null, and `border_radius` is provided, the canvas
    ///     is clipped by an [`RRect`] created from `clip_callback` and `border_radius`.
    ///  3. If `border_radius` is the default [`BorderRadius::ZERO`], then the [`Rect`] provided
    ///     by `clip_callback` is used for clipping.
    ///
    /// `text_direction` is used by `custom_border` if it is non-null. This allows the
    /// `custom_border`'s path to be properly defined if it was the path was expressed in terms
    /// of "start" and "end" instead of "left" and "right".
    ///
    /// For examples on how the function is used, see [`InkSplash`](crate::InkSplash) and
    /// [`InkRipple`](crate::InkRipple).
    #[allow(clippy::too_many_arguments)]
    pub fn paint_ink_circle(
        app: &App,
        canvas: &mut Canvas,
        transform: &Matrix4,
        paint: &Paint,
        center: Offset,
        radius: f64,
        text_direction: Option<TextDirection>,
        custom_border: Option<&dyn ShapeBorder>,
        border_radius: BorderRadius,
        clip_callback: Option<&RectCallback>,
    ) {
        let origin_offset = get_as_translation(*transform);
        canvas.save();
        match origin_offset {
            None => canvas.concat(transform),
            Some(origin_offset) => {
                canvas.translate(origin_offset.dx() as f32, origin_offset.dy() as f32)
            }
        }
        if let Some(clip_callback) = clip_callback {
            let rect = clip_callback(app);
            if let Some(custom_border) = custom_border {
                let path = custom_border.get_outer_path(rect, text_direction);
                canvas.clip_path(
                    path.as_valo(),
                    path.fill_type,
                    waft_embedder::ClipOp::Intersect,
                    true,
                );
            } else if border_radius != BorderRadius::ZERO {
                clip_rrect(canvas, rrect_from_rect_and_corners(rect, border_radius));
            } else {
                canvas.clip_rect(rect, waft_embedder::ClipOp::Intersect, true);
            }
        }
        canvas.draw_circle(center, radius as f32, paint);
        canvas.restore();
    }
}

/// Dart's `RRect.fromRectAndCorners(rect, topLeft: …, topRight: …, bottomLeft: …,
/// bottomRight: …)` with a border radius's corners.
pub(crate) fn rrect_from_rect_and_corners(rect: Rect, border_radius: BorderRadius) -> RRect {
    RRect::from_rect_and_corners(
        rect,
        border_radius.top_left,
        border_radius.top_right,
        border_radius.bottom_right,
        border_radius.bottom_left,
    )
}

/// Dart's `canvas.clipRRect(rrect)`.
pub(crate) fn clip_rrect(canvas: &mut Canvas, rrect: RRect) {
    canvas.clip_rrect_radii_elliptical(
        rrect.outer_rect(),
        waft_embedder::rrect_radii_elliptical(rrect),
        waft_embedder::ClipOp::Intersect,
        true,
    );
}

/// An encapsulation of an [`InteractiveInkFeature`] constructor used by
/// [`InkWell`], [`InkResponse`], and [`ThemeData`](crate::ThemeData).
///
/// Interactive ink feature implementations should provide a static
/// `splash_factory` value that's an instance of this class. The `splash_factory`
/// can be used to configure an [`InkWell`], [`InkResponse`] or
/// [`ThemeData`](crate::ThemeData).
///
/// See also:
///
///  * [`InkSplash::splash_factory`](crate::InkSplash::splash_factory)
///  * [`InkRipple::splash_factory`](crate::InkRipple::splash_factory)
pub trait InteractiveInkFeatureFactory: fmt::Debug {
    /// The factory method.
    ///
    /// Subclasses should override this method to return a new instance of an
    /// [`InteractiveInkFeature`].
    #[allow(clippy::too_many_arguments)]
    fn create(
        &self,
        app: &mut App,
        controller: AnyMaterialInkController,
        reference_box: AnyRenderBox,
        position: Offset,
        color: Color,
        text_direction: TextDirection,
        contained_ink_well: bool,
        rect_callback: Option<RectCallback>,
        border_radius: Option<BorderRadius>,
        custom_border: Option<ShapeBorderRef>,
        radius: Option<f64>,
        on_removed: Option<Listener>,
    ) -> InteractiveInkFeatureRef;
}

/// A field Dart types as `InteractiveInkFeatureFactory`; equal by identity, as Dart's `const`
/// factories are.
pub type InteractiveInkFeatureFactoryRef = Rc<dyn InteractiveInkFeatureFactory>;

/// Dart's `==` on a factory, which no factory overrides: identity.
impl PartialEq for dyn InteractiveInkFeatureFactory {
    fn eq(&self, other: &dyn InteractiveInkFeatureFactory) -> bool {
        std::ptr::addr_eq(
            self as *const dyn InteractiveInkFeatureFactory,
            other as *const dyn InteractiveInkFeatureFactory,
        )
    }
}

/// Dart's `_ParentInkResponseProvider`.
#[derive(Debug)]
struct ParentInkResponseProvider {
    state: Handle<InkResponseState>,
    child: WidgetRef,
}

impl ParentInkResponseProvider {
    fn maybe_of(app: &mut App, context: BuildContext) -> Option<Handle<InkResponseState>> {
        context
            .depend_on_inherited_widget_of_exact_type::<ParentInkResponseProvider>(app)
            .map(|provider| provider.state)
    }
}

impl InheritedWidget for ParentInkResponseProvider {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &ParentInkResponseProvider) -> bool {
        self.state != old_widget.state
    }
}

/// Dart's `_GetRectCallback`.
pub type GetRectCallback = Rc<dyn Fn(&App, AnyRenderBox) -> Option<RectCallback>>;

/// Dart's `_CheckContext`.
pub type CheckContext = Rc<dyn Fn(&App, BuildContext) -> bool>;

/// An area of a [`Material`] that responds to touch. Has a configurable shape and
/// can be configured to clip splashes that extend outside its bounds or not.
///
/// For a variant of this widget that is specialized for rectangular areas that
/// always clip splashes, see [`InkWell`].
///
/// An [`InkResponse`] widget does two things when responding to a tap:
///
///  * It starts to animate a _highlight_. The shape of the highlight is
///    determined by [`highlight_shape`](Self::highlight_shape). If it is a
///    [`BoxShape::Circle`], the default, then the highlight is a circle of fixed size centered
///    in the [`InkResponse`]. If it is [`BoxShape::Rectangle`], then the highlight is a box
///    the size of the [`InkResponse`] itself, unless
///    [`get_rect_callback`](Self::get_rect_callback) is provided, in which case that callback
///    defines the rectangle. The color of the highlight is set by
///    [`highlight_color`](Self::highlight_color).
///
///  * Simultaneously, it starts to animate a _splash_. This is a growing circle
///    initially centered on the tap location. If this is a
///    [`contained_ink_well`](Self::contained_ink_well), the splash grows to the
///    [`radius`](Self::radius) while remaining centered at the tap location. Otherwise, the
///    splash migrates to the center of the box as it grows.
///
/// The main thing to notice is that the splashes happily exceed the bounds of the widget
/// (because [`contained_ink_well`](Self::contained_ink_well) is false).
///
/// The [`InkResponse`] widget must have a [`Material`] widget as an ancestor. The
/// [`Material`] widget is where the ink reactions are actually painted. This
/// matches the Material Design premise wherein the [`Material`] is what is
/// actually reacting to touches by spreading ink.
///
/// If a Widget uses this class directly, it should include the following line
/// at the top of its build function to call [`debug_check_has_material`]:
///
/// ```text
/// debug_assert!(debug_check_has_material(app, context));
/// ```
///
/// ## Troubleshooting
///
/// ### The ink splashes aren't visible!
///
/// If there is an opaque graphic, e.g. painted using a `Container`, `Image`, or
/// `DecoratedBox`, between the [`Material`] widget and the [`InkResponse`] widget,
/// then the splash won't be visible because it will be under the opaque graphic.
/// This is because ink splashes draw on the underlying [`Material`] itself, as
/// if the ink was spreading inside the material.
///
/// The [`Ink`](crate::Ink) widget can be used as a replacement for `Image`, `Container`, or
/// `DecoratedBox` to ensure that the image or decoration also paints in the
/// [`Material`] itself, below the ink.
///
/// If this is not possible for some reason, e.g. because you are using an
/// opaque `CustomPaint` widget, alternatively consider using a second
/// [`Material`] above the opaque widget but below the [`InkResponse`] (as an
/// ancestor to the ink response). The [`MaterialType::Transparency`](crate::MaterialType)
/// material kind can be used for this purpose.
///
/// See also:
///
///  * [`GestureDetector`], for listening for gestures without ink splashes.
///  * `ElevatedButton` and `TextButton`, two kinds of buttons in Material Design.
///  * `IconButton`, which combines [`InkResponse`] with an `Icon`.
#[derive(Clone)]
pub struct InkResponse {
    pub key: Option<KeyRef>,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,

    /// Called when the user taps this part of the material.
    pub on_tap: Option<GestureTapCallback>,

    /// Called when the user taps down this part of the material.
    pub on_tap_down: Option<GestureTapDownCallback>,

    /// Called when the user releases a tap that was started on this part of the
    /// material. [`on_tap`](Self::on_tap) is called immediately after.
    pub on_tap_up: Option<GestureTapUpCallback>,

    /// Called when the user cancels a tap that was started on this part of the
    /// material.
    pub on_tap_cancel: Option<GestureTapCallback>,

    /// Called when the user double taps this part of the material.
    pub on_double_tap: Option<GestureTapCallback>,

    /// Called when the user long-presses on this part of the material.
    pub on_long_press: Option<GestureLongPressCallback>,

    /// Called when the user lifts their finger after a long press on the button.
    ///
    /// This callback is triggered at the end of a long press gesture, specifically
    /// after the user holds a long press and then releases it. It does not include
    /// position details.
    ///
    /// Common use cases include performing an action only after the long press completes,
    /// such as displaying a context menu or confirming a held gesture.
    ///
    /// See also:
    ///  * [`on_long_press`](Self::on_long_press), which is triggered when the long press
    ///    gesture is first recognized.
    pub on_long_press_up: Option<GestureLongPressUpCallback>,

    /// Called when the user taps this part of the material with a secondary button.
    ///
    /// See also:
    ///
    ///  * `K_SECONDARY_BUTTON`, the button this callback responds to.
    pub on_secondary_tap: Option<GestureTapCallback>,

    /// Called when the user taps down on this part of the material with a
    /// secondary button.
    ///
    /// See also:
    ///
    ///  * `K_SECONDARY_BUTTON`, the button this callback responds to.
    pub on_secondary_tap_down: Option<GestureTapDownCallback>,

    /// Called when the user releases a secondary button tap that was started on
    /// this part of the material. [`on_secondary_tap`](Self::on_secondary_tap) is called
    /// immediately after.
    ///
    /// See also:
    ///
    ///  * [`on_secondary_tap`](Self::on_secondary_tap), a handler triggered right after this
    ///    one that doesn't pass any details about the tap.
    ///  * `K_SECONDARY_BUTTON`, the button this callback responds to.
    pub on_secondary_tap_up: Option<GestureTapUpCallback>,

    /// Called when the user cancels a secondary button tap that was started on
    /// this part of the material.
    ///
    /// See also:
    ///
    ///  * `K_SECONDARY_BUTTON`, the button this callback responds to.
    pub on_secondary_tap_cancel: Option<GestureTapCallback>,

    /// Called when the user taps down on this part of the material with a
    /// tertiary button.
    ///
    /// A common use case for the tertiary button is opening a link in a new tab
    /// or window with a middle click of a mouse.
    ///
    /// See also:
    ///
    ///  * `K_TERTIARY_BUTTON`, the button this callback responds to.
    pub on_tertiary_tap_down: Option<GestureTapDownCallback>,

    /// Called when the user releases a tertiary button tap that was started on
    /// this part of the material.
    ///
    /// See also:
    ///
    ///  * [`on_tertiary_tap_down`](Self::on_tertiary_tap_down), which is called when the
    ///    tertiary button tap starts.
    ///  * `K_TERTIARY_BUTTON`, the button this callback responds to.
    pub on_tertiary_tap_up: Option<GestureTapUpCallback>,

    /// Called when the user cancels a tertiary button tap that was started on
    /// this part of the material.
    ///
    /// See also:
    ///
    ///  * `K_TERTIARY_BUTTON`, the button this callback responds to.
    pub on_tertiary_tap_cancel: Option<GestureTapCancelCallback>,

    /// Called when this part of the material either becomes highlighted or stops
    /// being highlighted.
    ///
    /// The value passed to the callback is true if this part of the material has
    /// become highlighted and false if this part of the material has stopped
    /// being highlighted.
    ///
    /// If all of [`on_tap`](Self::on_tap), [`on_double_tap`](Self::on_double_tap),
    /// [`on_long_press`](Self::on_long_press), and
    /// [`on_long_press_up`](Self::on_long_press_up) become null while a
    /// gesture is ongoing, then [`on_tap_cancel`](Self::on_tap_cancel) will be fired and
    /// [`on_highlight_changed`](Self::on_highlight_changed) will be fired with the value false
    /// _during the build_. This means, for instance, that in that scenario `State::set_state`
    /// cannot be called.
    pub on_highlight_changed: Option<ValueChanged<bool>>,

    /// Called when a pointer enters or exits the ink response area.
    ///
    /// The value passed to the callback is true if a pointer has entered this
    /// part of the material and false if a pointer has exited this part of the
    /// material.
    pub on_hover: Option<ValueChanged<bool>>,

    /// The cursor for a mouse pointer when it enters or is hovering over the
    /// widget.
    ///
    /// If [`mouse_cursor`](Self::mouse_cursor) is a [`WidgetStateMouseCursor`],
    /// `WidgetStateProperty::resolve` is used for the following [`WidgetState`]s:
    ///
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If this property is null, [`WidgetStateMouseCursor::adaptive_clickable`] will be used.
    pub mouse_cursor: Option<MouseCursorRef>,

    /// Whether this ink response should be clipped its bounds.
    ///
    /// This flag also controls whether the splash migrates to the center of the
    /// [`InkResponse`] or not. If [`contained_ink_well`](Self::contained_ink_well) is true,
    /// the splash remains centered around the tap location. If it is false, the splash
    /// migrates to the center of the [`InkResponse`] as it grows.
    ///
    /// See also:
    ///
    ///  * [`highlight_shape`](Self::highlight_shape), the shape of the focus, hover, and
    ///    pressed highlights.
    ///  * [`border_radius`](Self::border_radius), which controls the corners when the box is a
    ///    rectangle.
    ///  * [`get_rect_callback`](Self::get_rect_callback), which controls the size and position
    ///    of the box when it is a rectangle.
    pub contained_ink_well: bool,

    /// The shape (e.g., circle, rectangle) to use for the highlight drawn around
    /// this part of the material when pressed, hovered over, or focused.
    ///
    /// The same shape is used for the pressed highlight (see
    /// [`highlight_color`](Self::highlight_color)), the focus highlight (see
    /// [`focus_color`](Self::focus_color)), and the hover highlight (see
    /// [`hover_color`](Self::hover_color)).
    ///
    /// If the shape is [`BoxShape::Circle`], then the highlight is centered on the
    /// [`InkResponse`]. If the shape is [`BoxShape::Rectangle`], then the highlight
    /// fills the [`InkResponse`], or the rectangle provided by
    /// [`get_rect_callback`](Self::get_rect_callback) if the callback is specified.
    ///
    /// See also:
    ///
    ///  * [`contained_ink_well`](Self::contained_ink_well), which controls clipping behavior.
    ///  * [`border_radius`](Self::border_radius), which controls the corners when the box is a
    ///    rectangle.
    ///  * [`highlight_color`](Self::highlight_color), the color of the highlight.
    ///  * [`get_rect_callback`](Self::get_rect_callback), which controls the size and position
    ///    of the box when it is a rectangle.
    pub highlight_shape: BoxShape,

    /// The radius of the ink splash.
    ///
    /// Splashes grow up to this size. By default, this size is determined from
    /// the size of the rectangle provided by [`get_rect_callback`](Self::get_rect_callback),
    /// or the size of the [`InkResponse`] itself.
    ///
    /// See also:
    ///
    ///  * [`splash_color`](Self::splash_color), the color of the splash.
    ///  * [`splash_factory`](Self::splash_factory), which defines the appearance of the
    ///    splash.
    pub radius: Option<f64>,

    /// The border radius of the containing rectangle. This is effective only if
    /// [`highlight_shape`](Self::highlight_shape) is [`BoxShape::Rectangle`].
    ///
    /// If this is null, it is interpreted as [`BorderRadius::ZERO`].
    pub border_radius: Option<BorderRadius>,

    /// The custom clip border.
    ///
    /// If this is null, the ink response will not clip its content.
    pub custom_border: Option<ShapeBorderRef>,

    /// The color of the ink response when the parent widget is focused. If this
    /// property is null then the focus color of the theme,
    /// `ThemeData::focus_color`, will be used.
    ///
    /// See also:
    ///
    ///  * [`highlight_shape`](Self::highlight_shape), the shape of the focus, hover, and
    ///    pressed highlights.
    ///  * [`hover_color`](Self::hover_color), the color of the hover highlight.
    ///  * [`splash_color`](Self::splash_color), the color of the splash.
    ///  * [`splash_factory`](Self::splash_factory), which defines the appearance of the
    ///    splash.
    pub focus_color: Option<Color>,

    /// The color of the ink response when a pointer is hovering over it. If this
    /// property is null then the hover color of the theme,
    /// `ThemeData::hover_color`, will be used.
    ///
    /// See also:
    ///
    ///  * [`highlight_shape`](Self::highlight_shape), the shape of the focus, hover, and
    ///    pressed highlights.
    ///  * [`highlight_color`](Self::highlight_color), the color of the pressed highlight.
    ///  * [`focus_color`](Self::focus_color), the color of the focus highlight.
    ///  * [`splash_color`](Self::splash_color), the color of the splash.
    ///  * [`splash_factory`](Self::splash_factory), which defines the appearance of the
    ///    splash.
    pub hover_color: Option<Color>,

    /// The highlight color of the ink response when pressed. If this property is
    /// null then the highlight color of the theme, `ThemeData::highlight_color`,
    /// will be used.
    ///
    /// See also:
    ///
    ///  * [`hover_color`](Self::hover_color), the color of the hover highlight.
    ///  * [`focus_color`](Self::focus_color), the color of the focus highlight.
    ///  * [`highlight_shape`](Self::highlight_shape), the shape of the focus, hover, and
    ///    pressed highlights.
    ///  * [`splash_color`](Self::splash_color), the color of the splash.
    ///  * [`splash_factory`](Self::splash_factory), which defines the appearance of the
    ///    splash.
    pub highlight_color: Option<Color>,

    /// Defines the ink response focus, hover, and splash colors.
    ///
    /// This default null property can be used as an alternative to
    /// [`focus_color`](Self::focus_color), [`hover_color`](Self::hover_color),
    /// [`highlight_color`](Self::highlight_color), and [`splash_color`](Self::splash_color).
    /// If non-null, it is resolved against one of [`WidgetState::Focused`],
    /// [`WidgetState::Hovered`], and [`WidgetState::Pressed`]. It's convenient to use when the
    /// parent widget can pass along its own WidgetStateProperty value for the overlay color.
    ///
    /// [`WidgetState::Pressed`] triggers a ripple (an ink splash), per
    /// the current Material Design spec. The [`overlay_color`](Self::overlay_color) doesn't map
    /// a state to [`highlight_color`](Self::highlight_color) because a separate highlight is
    /// not used by the current design guidelines. See
    /// <https://material.io/design/interaction/states.html#pressed>
    ///
    /// If the overlay color is null or resolves to null, then
    /// [`focus_color`](Self::focus_color), [`hover_color`](Self::hover_color),
    /// [`splash_color`](Self::splash_color) and their defaults are used instead.
    ///
    /// See also:
    ///
    ///  * The Material Design specification for overlay colors and how they
    ///    match a component's state:
    ///    <https://material.io/design/interaction/states.html#anatomy>.
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The splash color of the ink response. If this property is null then the
    /// splash color of the theme, `ThemeData::splash_color`, will be used.
    ///
    /// See also:
    ///
    ///  * [`splash_factory`](Self::splash_factory), which defines the appearance of the
    ///    splash.
    ///  * [`radius`](Self::radius), the (maximum) size of the ink splash.
    ///  * [`highlight_color`](Self::highlight_color), the color of the highlight.
    pub splash_color: Option<Color>,

    /// Defines the appearance of the splash.
    ///
    /// Defaults to the value of the theme's splash factory: `ThemeData::splash_factory`.
    ///
    /// See also:
    ///
    ///  * [`radius`](Self::radius), the (maximum) size of the ink splash.
    ///  * [`splash_color`](Self::splash_color), the color of the splash.
    ///  * [`highlight_color`](Self::highlight_color), the color of the highlight.
    ///  * [`InkSplash::splash_factory`](crate::InkSplash::splash_factory), which defines the
    ///    default splash.
    ///  * [`InkRipple::splash_factory`](crate::InkRipple::splash_factory), which defines a
    ///    splash that spreads out more aggressively than the default.
    pub splash_factory: Option<InteractiveInkFeatureFactoryRef>,

    /// Whether detected gestures should provide acoustic and/or haptic feedback.
    ///
    /// For example, on Android a tap will produce a clicking sound and a
    /// long-press will produce a short vibration, when feedback is enabled.
    ///
    /// See also:
    ///
    ///  * [`Feedback`] for providing platform-specific feedback to certain actions.
    pub enable_feedback: bool,

    /// Whether to exclude the gestures introduced by this widget from the
    /// semantics tree.
    ///
    /// For example, a long-press gesture for showing a tooltip is usually
    /// excluded because the tooltip itself is included in the semantics
    /// tree directly and so having a gesture to show it would result in
    /// duplication of information.
    pub exclude_from_semantics: bool,

    /// Handler called when the focus changes.
    ///
    /// Called with true if this widget's node gains focus, and false if it loses
    /// focus.
    pub on_focus_change: Option<ValueChanged<bool>>,

    /// True if this widget will be selected as the initial focus when no other
    /// node in its scope is currently focused.
    pub autofocus: bool,

    /// An optional focus node to use as the focus node for this widget.
    pub focus_node: Option<FocusNodeRef>,

    /// If true, this widget may request the primary focus.
    pub can_request_focus: bool,

    /// Represents the interactive "state" of this widget in terms of
    /// a set of [`WidgetState`]s, like [`WidgetState::Pressed`] and
    /// [`WidgetState::Focused`].
    ///
    /// Classes based on this one can provide their own
    /// `WidgetStatesController` to which they've added listeners.
    /// They can also update the controller's `WidgetStatesController::value`
    /// however, this may only be done when it's safe to call
    /// `State::set_state`, like in an event handler.
    pub states_controller: Option<Ref<MaterialStatesController>>,

    /// The duration of the animation that animates the hover effect.
    ///
    /// The default is 50ms.
    pub hover_duration: Option<Duration>,

    /// Dart's `getRectCallback` override point, which a widget that specializes
    /// [`InkResponse`] fills through
    /// [`override_get_rect_callback`](Self::override_get_rect_callback);
    /// [`get_rect_callback`](Self::get_rect_callback) calls it.
    pub get_rect_callback: GetRectCallback,

    /// Dart's `debugCheckContext` override point, which a widget that specializes
    /// [`InkResponse`] fills through
    /// [`override_debug_check_context`](Self::override_debug_check_context).
    ///
    /// Dart's method is `@mustCallSuper`: a filler also runs
    /// [`InkResponse::debug_check_context`].
    pub debug_check_context: CheckContext,
}

impl fmt::Debug for InkResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InkResponse")
            .field("contained_ink_well", &self.contained_ink_well)
            .field("highlight_shape", &self.highlight_shape)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl Default for InkResponse {
    fn default() -> InkResponse {
        InkResponse {
            key: None,
            child: None,
            on_tap: None,
            on_tap_down: None,
            on_tap_up: None,
            on_tap_cancel: None,
            on_double_tap: None,
            on_long_press: None,
            on_long_press_up: None,
            on_secondary_tap: None,
            on_secondary_tap_down: None,
            on_secondary_tap_up: None,
            on_secondary_tap_cancel: None,
            on_tertiary_tap_down: None,
            on_tertiary_tap_up: None,
            on_tertiary_tap_cancel: None,
            on_highlight_changed: None,
            on_hover: None,
            mouse_cursor: None,
            contained_ink_well: false,
            highlight_shape: BoxShape::Circle,
            radius: None,
            border_radius: None,
            custom_border: None,
            focus_color: None,
            hover_color: None,
            highlight_color: None,
            overlay_color: None,
            splash_color: None,
            splash_factory: None,
            enable_feedback: true,
            exclude_from_semantics: false,
            on_focus_change: None,
            autofocus: false,
            focus_node: None,
            can_request_focus: true,
            states_controller: None,
            hover_duration: None,
            get_rect_callback: Rc::new(|_app, _reference_box| None),
            debug_check_context: Rc::new(InkResponse::debug_check_context),
        }
    }
}

/// The fluent setters of Dart's `InkResponse` named arguments, defined on `InkResponse` and
/// on `InkWell`, whose constructor passes them on as `super.` arguments.
macro_rules! ink_response_setters {
    ($name:ident, $($field:ident: $ty:ty),* $(,)?) => {
        $(
            #[doc = concat!("Dart `", stringify!($name), "(", stringify!($field), ":)`.")]
            pub fn $field(mut self, $field: $ty) -> $name {
                self.ink_response_mut().$field = Some($field);
                self
            }
        )*
    };
}

/// The setters whose Dart field is not nullable.
macro_rules! ink_response_value_setters {
    ($name:ident, $($field:ident: $ty:ty),* $(,)?) => {
        $(
            #[doc = concat!("Dart `", stringify!($name), "(", stringify!($field), ":)`.")]
            pub fn $field(mut self, $field: $ty) -> $name {
                self.ink_response_mut().$field = $field;
                self
            }
        )*
    };
}

/// Every setter `InkResponse` and `InkWell` share.
macro_rules! ink_response_shared_setters {
    ($name:ident) => {
        ink_response_setters!(
            $name,
            key: KeyRef,
            on_tap: GestureTapCallback,
            on_tap_down: GestureTapDownCallback,
            on_tap_up: GestureTapUpCallback,
            on_tap_cancel: GestureTapCallback,
            on_double_tap: GestureTapCallback,
            on_long_press: GestureLongPressCallback,
            on_long_press_up: GestureLongPressUpCallback,
            on_secondary_tap: GestureTapCallback,
            on_secondary_tap_up: GestureTapUpCallback,
            on_secondary_tap_down: GestureTapDownCallback,
            on_secondary_tap_cancel: GestureTapCallback,
            on_tertiary_tap_up: GestureTapUpCallback,
            on_tertiary_tap_down: GestureTapDownCallback,
            on_tertiary_tap_cancel: GestureTapCancelCallback,
            on_highlight_changed: ValueChanged<bool>,
            on_hover: ValueChanged<bool>,
            mouse_cursor: MouseCursorRef,
            radius: f64,
            border_radius: BorderRadius,
            focus_color: Color,
            hover_color: Color,
            highlight_color: Color,
            overlay_color: WidgetStatePropertyRef<Option<Color>>,
            splash_color: Color,
            splash_factory: InteractiveInkFeatureFactoryRef,
            on_focus_change: ValueChanged<bool>,
            focus_node: FocusNodeRef,
            states_controller: Ref<MaterialStatesController>,
            hover_duration: Duration,
        );
        ink_response_value_setters!(
            $name,
            enable_feedback: bool,
            exclude_from_semantics: bool,
            can_request_focus: bool,
            autofocus: bool,
        );

        #[doc = concat!("Dart `", stringify!($name), "(child:)`.")]
        pub fn child<K>(mut self, child: impl IntoWidget<K>) -> $name {
            self.ink_response_mut().child = Some(child.into_widget());
            self
        }

        #[doc = concat!("Dart `", stringify!($name), "(customBorder:)`.")]
        pub fn custom_border(mut self, custom_border: impl Into<ShapeBorderRef>) -> $name {
            self.ink_response_mut().custom_border = Some(custom_border.into());
            self
        }

        /// Dart's override of `getRectCallback` in a subclass: see
        /// [`InkResponse::get_rect_callback`].
        pub fn override_get_rect_callback(
            mut self,
            get_rect_callback: impl Fn(&App, AnyRenderBox) -> Option<RectCallback> + 'static,
        ) -> $name {
            self.ink_response_mut().get_rect_callback = Rc::new(get_rect_callback);
            self
        }

        /// Dart's override of `debugCheckContext` in a subclass: see
        /// [`InkResponse::debug_check_context`].
        ///
        /// Dart's method is `@mustCallSuper`: `debug_check_context` also runs
        /// [`InkResponse::debug_check_context`].
        pub fn override_debug_check_context(
            mut self,
            debug_check_context: impl Fn(&App, BuildContext) -> bool + 'static,
        ) -> $name {
            self.ink_response_mut().debug_check_context = Rc::new(debug_check_context);
            self
        }
    };
}

impl InkResponse {
    /// Creates an area of a [`Material`] that responds to touch; Dart's named arguments are
    /// the setters.
    ///
    /// Must have an ancestor [`Material`] widget in which to cause ink reactions.
    pub fn new() -> InkResponse {
        InkResponse::default()
    }

    fn ink_response_mut(&mut self) -> &mut InkResponse {
        self
    }

    ink_response_shared_setters!(InkResponse);

    ink_response_value_setters!(
        InkResponse,
        contained_ink_well: bool,
        highlight_shape: BoxShape,
    );

    /// The rectangle to use for the highlight effect and for clipping
    /// the splash effects if [`contained_ink_well`](Self::contained_ink_well) is true.
    ///
    /// This method is intended to be overridden by descendants that
    /// specialize [`InkResponse`] for unusual cases. For example,
    /// `TableRowInkWell` implements this method to return the rectangle
    /// corresponding to the row that the widget is in.
    ///
    /// The default behavior returns null, which is equivalent to
    /// returning the reference_box argument's bounding box (though
    /// slightly more efficient).
    pub fn get_rect_callback(
        &self,
        app: &App,
        reference_box: AnyRenderBox,
    ) -> Option<RectCallback> {
        (self.get_rect_callback)(app, reference_box)
    }

    /// Asserts that the given context satisfies the prerequisites for
    /// this class.
    ///
    /// This method is intended to be overridden by descendants that
    /// specialize [`InkResponse`] for unusual cases. For example,
    /// `TableRowInkWell` implements this method to verify that the widget is
    /// in a table.
    pub fn debug_check_context(app: &App, context: BuildContext) -> bool {
        debug_assert!(debug_check_has_material(app, context));
        debug_assert!(debug_check_has_directionality(
            app, context, None, None, None
        ));
        true
    }

    fn build_state_widget(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let parent_state = ParentInkResponseProvider::maybe_of(app, context);
        InkResponseStateWidget {
            response: self.clone(),
            parent_state,
        }
        .into_widget()
    }
}

impl StatelessWidget for InkResponse {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        self.build_state_widget(app, context)
    }
}

/// Dart's `_InkResponseStateWidget`: the [`InkResponse`]'s configuration, which it passes on
/// field for field, and its parent's state.
#[derive(Debug)]
struct InkResponseStateWidget {
    response: InkResponse,
    parent_state: Option<Handle<InkResponseState>>,
}

impl StatefulWidget for InkResponseStateWidget {
    type State = InkResponseState;

    fn create_state(&self) -> InkResponseState {
        InkResponseState {
            state: StateData::new(),
            automatic_keep_alive_client: AutomaticKeepAliveClientMixinData::new(),
            splashes: None,
            current_splash: None,
            hovering: false,
            highlights: HashMap::new(),
            action_map: HashMap::new(),
            internal_states_controller: None,
            active_children: Vec::new(),
            activation_timer: None,
            has_focus: false,
            focus_highlight_mode_listener: None,
        }
    }
}

/// Used to index the allocated highlights for the different types of highlights
/// in [`InkResponseState`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum HighlightType {
    Pressed,
    Hover,
    Focus,
}

/// Dart's `_InkResponseState`.
struct InkResponseState {
    state: StateData<InkResponseStateWidget>,
    automatic_keep_alive_client: AutomaticKeepAliveClientMixinData,
    splashes: Option<HashSet<InteractiveInkFeatureRef>>,
    current_splash: Option<InteractiveInkFeatureRef>,
    hovering: bool,
    highlights: HashMap<HighlightType, Option<Ref<InkHighlight>>>,
    action_map: HashMap<TypeId, ActionRef>,
    internal_states_controller: Option<Ref<MaterialStatesController>>,
    active_children: Vec<Handle<InkResponseState>>,
    activation_timer: Option<Timer>,
    has_focus: bool,
    focus_highlight_mode_listener: Option<HighlightModeListener>,
}

impl InkResponseState {
    const ACTIVATION_DURATION: Duration = Duration::from_millis(100);

    fn highlights_exist(self: Handle<Self>, app: &App) -> bool {
        app.get(self).highlights.values().any(Option::is_some)
    }

    fn mark_child_ink_response_pressed(
        self: Handle<Self>,
        app: &mut App,
        child_state: Handle<InkResponseState>,
        value: bool,
    ) {
        let last_any_pressed = self.any_child_ink_response_pressed(app);
        let active_children = &mut app.get_mut(self).active_children;
        if value {
            active_children.push(child_state);
        } else if let Some(index) = active_children
            .iter()
            .position(|child| *child == child_state)
        {
            active_children.remove(index);
        }
        let now_any_pressed = self.any_child_ink_response_pressed(app);
        if now_any_pressed != last_any_pressed
            && let Some(parent_state) = self.widget(app).parent_state
        {
            parent_state.mark_child_ink_response_pressed(app, self, now_any_pressed);
        }
    }

    fn any_child_ink_response_pressed(self: Handle<Self>, app: &App) -> bool {
        !app.get(self).active_children.is_empty()
    }

    fn activate_on_intent(self: Handle<Self>, app: &mut App) {
        if let Some(timer) = app.get_mut(self).activation_timer.take() {
            timer.cancel(app);
        }
        let context = self.context(app);
        self.start_new_splash(app, None, Some(context));
        if let Some(current_splash) = app.get_mut(self).current_splash.take() {
            current_splash.confirm(app);
        }
        if let Some(on_tap) = self.widget(app).response.on_tap.clone() {
            if self.widget(app).response.enable_feedback {
                Feedback::for_tap(app, context);
            }
            on_tap.call(app);
        }
        // Delay the call to `updateHighlight` to simulate a pressed delay
        // and give MaterialStatesController listeners a chance to react.
        let timer = Timer::new(
            app,
            Self::ACTIVATION_DURATION,
            Listener::new(move |app| {
                self.update_highlight(app, HighlightType::Pressed, false, true);
            }),
        );
        app.get_mut(self).activation_timer = Some(timer);
    }

    /// Dart's `simulateTap`, which the semantics tap handler calls.
    fn simulate_tap(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        self.start_new_splash(app, None, Some(context));
        self.handle_tap(app);
    }

    /// Dart's `simulateLongPress`, which the semantics long-press handler calls.
    fn simulate_long_press(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        self.start_new_splash(app, None, Some(context));
        self.handle_long_press(app);
    }

    fn handle_states_controller_change(self: Handle<Self>, app: &mut App) {
        // Force a rebuild to resolve widget.overlayColor, widget.mouseCursor
        self.set_state(app, |_state| {});
    }

    fn states_controller(self: Handle<Self>, app: &App) -> Handle<MaterialStatesController> {
        self.widget(app)
            .response
            .states_controller
            .handle()
            .or(app.get(self).internal_states_controller.handle())
            .expect("created in init_states_controller")
    }

    fn states(self: Handle<Self>, app: &App) -> WidgetStates {
        app.get(self.states_controller(app)).value().clone()
    }

    fn init_states_controller(self: Handle<Self>, app: &mut App) {
        if self.widget(app).response.states_controller.is_none() {
            let controller = MaterialStatesController::new(app, None);
            app.get_mut(self).internal_states_controller = Some(controller);
        }
        let enabled = self.enabled(app);
        let states_controller = self.states_controller(app);
        states_controller.update(app, WidgetState::Disabled, !enabled);
        states_controller.add_listener(
            app,
            Listener::handle_method(self, Self::handle_states_controller_change),
        );
    }

    fn get_fade_duration_for_type(
        self: Handle<Self>,
        app: &App,
        r#type: HighlightType,
    ) -> Duration {
        match r#type {
            HighlightType::Pressed => Duration::from_millis(200),
            HighlightType::Hover | HighlightType::Focus => self
                .widget(app)
                .response
                .hover_duration
                .unwrap_or(Duration::from_millis(50)),
        }
    }

    fn update_highlight(
        self: Handle<Self>,
        app: &mut App,
        r#type: HighlightType,
        value: bool,
        call_on_hover: bool,
    ) {
        let highlight = app
            .get(self)
            .highlights
            .get(&r#type)
            .and_then(|highlight| highlight.handle());
        let handle_ink_removal = Listener::new(move |app| {
            debug_assert!(
                app.get(self)
                    .highlights
                    .get(&r#type)
                    .and_then(|highlight| highlight.handle())
                    .is_some()
            );
            app.get_mut(self).highlights.insert(r#type, None);
            self.update_keep_alive(app);
        });

        let states_controller = self.states_controller(app);
        match r#type {
            HighlightType::Pressed => {
                states_controller.update(app, WidgetState::Pressed, value);
            }
            HighlightType::Hover => {
                if call_on_hover {
                    states_controller.update(app, WidgetState::Hovered, value);
                }
            }
            HighlightType::Focus => {
                // see handleFocusUpdate()
            }
        }

        if r#type == HighlightType::Pressed
            && let Some(parent_state) = self.widget(app).parent_state
        {
            parent_state.mark_child_ink_response_pressed(app, self, value);
        }
        if value == highlight.is_some_and(|highlight| highlight.active(app)) {
            return;
        }

        if value {
            match highlight {
                None => {
                    let response = self.widget(app).response.clone();
                    let states = self.states(app);
                    let context = self.context(app);
                    let resolved_overlay_color = response
                        .overlay_color
                        .as_ref()
                        .and_then(|overlay_color| overlay_color.resolve(&states))
                        .unwrap_or_else(|| match r#type {
                            // Use the backwards compatible defaults
                            HighlightType::Pressed => response
                                .highlight_color
                                .unwrap_or_else(|| Theme::of(app, context).highlight_color),
                            HighlightType::Focus => response
                                .focus_color
                                .unwrap_or_else(|| Theme::of(app, context).focus_color),
                            HighlightType::Hover => response
                                .hover_color
                                .unwrap_or_else(|| Theme::of(app, context).hover_color),
                        });
                    let reference_box = context
                        .find_render_object(app)
                        .and_then(|render_object| render_object.as_box())
                        .expect("an ink response has a render box");
                    let enabled = self.enabled(app);
                    let controller = Material::of(app, context);
                    let rect_callback = response.get_rect_callback(app, reference_box);
                    let text_direction = Directionality::of(app, context);
                    let fade_duration = self.get_fade_duration_for_type(app, r#type);
                    let created = InkHighlight::new(
                        app,
                        controller,
                        reference_box,
                        if enabled {
                            resolved_overlay_color
                        } else {
                            resolved_overlay_color.with_alpha(0)
                        },
                        text_direction,
                        response.highlight_shape,
                        response.radius,
                        response.border_radius,
                        response.custom_border.clone(),
                        rect_callback,
                        Some(handle_ink_removal),
                        fade_duration,
                    );
                    app.get_mut(self).highlights.insert(r#type, Some(created));
                    self.update_keep_alive(app);
                }
                Some(highlight) => highlight.activate(app),
            }
        } else {
            highlight
                .expect("an active highlight exists")
                .deactivate(app);
        }
        debug_assert!(
            value
                == app
                    .get(self)
                    .highlights
                    .get(&r#type)
                    .and_then(|highlight| highlight.handle())
                    .is_some_and(|highlight| highlight.active(app))
        );

        match r#type {
            HighlightType::Pressed => {
                if let Some(on_highlight_changed) =
                    self.widget(app).response.on_highlight_changed.clone()
                {
                    on_highlight_changed(app, value);
                }
            }
            HighlightType::Hover => {
                if call_on_hover && let Some(on_hover) = self.widget(app).response.on_hover.clone()
                {
                    on_hover(app, value);
                }
            }
            HighlightType::Focus => {}
        }
    }

    fn update_highlights_and_splashes(self: Handle<Self>, app: &mut App) {
        let custom_border = self.widget(app).response.custom_border.clone();
        let highlights: Vec<Handle<InkHighlight>> = app
            .get(self)
            .highlights
            .values()
            .flatten()
            .map(Retained::handle)
            .collect();
        for highlight in highlights {
            InteractiveInkFeature::set_custom_border(highlight, app, custom_border.clone());
        }
        if let Some(current_splash) = app.get(self).current_splash.handle() {
            current_splash.set_custom_border(app, custom_border.clone());
        }

        let splashes: Vec<AnyInteractiveInkFeature> = app
            .get(self)
            .splashes
            .iter()
            .flatten()
            .map(Retained::handle)
            .collect();
        for ink_feature in splashes {
            ink_feature.set_custom_border(app, custom_border.clone());
        }
    }

    fn create_splash(
        self: Handle<Self>,
        app: &mut App,
        global_position: Offset,
    ) -> InteractiveInkFeatureRef {
        let context = self.context(app);
        let ink_controller = Material::of(app, context);
        let reference_box = context
            .find_render_object(app)
            .and_then(|render_object| render_object.as_box())
            .expect("an ink response has a render box");
        let position = reference_box.global_to_local(app, global_position, None);
        let response = self.widget(app).response.clone();
        let states = self.states(app);
        let color = response
            .overlay_color
            .as_ref()
            .and_then(|overlay_color| overlay_color.resolve(&states))
            .or(response.splash_color)
            .unwrap_or_else(|| Theme::of(app, context).splash_color);
        let rect_callback = if response.contained_ink_well {
            response.get_rect_callback(app, reference_box)
        } else {
            None
        };
        let border_radius = response.border_radius;
        let custom_border = response.custom_border.clone();

        let splash: Rc<std::cell::Cell<Option<AnyInteractiveInkFeature>>> = Rc::default();
        let on_removed = Listener::new({
            let splash = Rc::clone(&splash);
            move |app| {
                let splash = splash.get();
                if let Some(splashes) = &mut app.get_mut(self).splashes {
                    debug_assert!(splash.is_some_and(|splash| splashes.contains(&splash)));
                    if let Some(splash) = splash {
                        splashes.remove(&splash);
                    }
                    if app.get(self).current_splash.handle() == splash {
                        app.get_mut(self).current_splash = None;
                    }
                    self.update_keep_alive(app);
                } // else we're probably in deactivate()
            }
        });

        let splash_factory = response
            .splash_factory
            .clone()
            .unwrap_or_else(|| Theme::of(app, context).splash_factory.clone());
        let text_direction = Directionality::of(app, context);
        let created = splash_factory.create(
            app,
            ink_controller,
            reference_box,
            position,
            color,
            text_direction,
            response.contained_ink_well,
            rect_callback,
            border_radius,
            custom_border,
            response.radius,
            Some(on_removed),
        );
        splash.set(Some(created.handle()));
        created
    }

    fn handle_focus_highlight_mode_change(
        self: Handle<Self>,
        app: &mut App,
        _mode: FocusHighlightMode,
    ) {
        if !self.mounted(app) {
            return;
        }
        self.update_focus_highlights(app);
        self.set_state(app, |_state| {});
    }

    fn should_show_focus(self: Handle<Self>, app: &mut App) -> bool {
        let context = self.context(app);
        match MediaQuery::maybe_navigation_mode_of(app, context) {
            Some(NavigationMode::Traditional) | None => {
                self.enabled(app) && app.get(self).has_focus
            }
            Some(NavigationMode::Directional) => app.get(self).has_focus,
        }
    }

    fn update_focus_highlights(self: Handle<Self>, app: &mut App) {
        let show_focus = match FocusManager::instance(app).highlight_mode(app) {
            FocusHighlightMode::Touch => false,
            FocusHighlightMode::Traditional => self.should_show_focus(app),
        };
        self.update_highlight(app, HighlightType::Focus, show_focus, true);
    }

    fn handle_focus_update(self: Handle<Self>, app: &mut App, has_focus: bool) {
        app.get_mut(self).has_focus = has_focus;
        // Set here rather than updateHighlight because this widget's
        // (WidgetState) states include WidgetState.focused if
        // the InkWell _has_ the focus, rather than if it's showing
        // the focus per FocusManager.instance.highlightMode.
        self.states_controller(app)
            .update(app, WidgetState::Focused, has_focus);
        self.update_focus_highlights(app);
        if let Some(on_focus_change) = self.widget(app).response.on_focus_change.clone() {
            on_focus_change(app, has_focus);
        }
    }

    fn handle_any_tap_down(self: Handle<Self>, app: &mut App, details: &TapDownDetails) {
        if self.any_child_ink_response_pressed(app) {
            return;
        }
        self.start_new_splash(app, Some(details), None);
    }

    fn handle_tap_down(self: Handle<Self>, app: &mut App, details: TapDownDetails) {
        self.handle_any_tap_down(app, &details);
        if let Some(on_tap_down) = self.widget(app).response.on_tap_down.clone() {
            on_tap_down(app, details);
        }
    }

    fn handle_tap_up(self: Handle<Self>, app: &mut App, details: TapUpDetails) {
        if let Some(on_tap_up) = self.widget(app).response.on_tap_up.clone() {
            on_tap_up(app, details);
        }
    }

    fn handle_secondary_tap_down(self: Handle<Self>, app: &mut App, details: TapDownDetails) {
        self.handle_any_tap_down(app, &details);
        if let Some(on_secondary_tap_down) = self.widget(app).response.on_secondary_tap_down.clone()
        {
            on_secondary_tap_down(app, details);
        }
    }

    fn handle_secondary_tap_up(self: Handle<Self>, app: &mut App, details: TapUpDetails) {
        if let Some(on_secondary_tap_up) = self.widget(app).response.on_secondary_tap_up.clone() {
            on_secondary_tap_up(app, details);
        }
    }

    fn handle_tertiary_tap_down(self: Handle<Self>, app: &mut App, details: TapDownDetails) {
        self.handle_any_tap_down(app, &details);
        if let Some(on_tertiary_tap_down) = self.widget(app).response.on_tertiary_tap_down.clone() {
            on_tertiary_tap_down(app, details);
        }
    }

    fn handle_tertiary_tap_up(self: Handle<Self>, app: &mut App, details: TapUpDetails) {
        if let Some(current_splash) = app.get_mut(self).current_splash.take() {
            current_splash.confirm(app);
        }
        self.update_highlight(app, HighlightType::Pressed, false, true);
        if let Some(on_tertiary_tap_up) = self.widget(app).response.on_tertiary_tap_up.clone() {
            on_tertiary_tap_up(app, details);
        }
    }

    fn handle_tertiary_tap_cancel(self: Handle<Self>, app: &mut App) {
        if let Some(current_splash) = app.get_mut(self).current_splash.take() {
            current_splash.cancel(app);
        }
        if let Some(on_tertiary_tap_cancel) =
            self.widget(app).response.on_tertiary_tap_cancel.clone()
        {
            on_tertiary_tap_cancel.call(app);
        }
        self.update_highlight(app, HighlightType::Pressed, false, true);
    }

    fn start_new_splash(
        self: Handle<Self>,
        app: &mut App,
        details: Option<&TapDownDetails>,
        context: Option<BuildContext>,
    ) {
        debug_assert!(details.is_some() || context.is_some());

        let global_position = match context {
            Some(context) => {
                let reference_box = context
                    .find_render_object(app)
                    .and_then(|render_object| render_object.as_box())
                    .expect("an ink response has a render box");
                debug_assert!(
                    reference_box.has_size(app),
                    "InkResponse must be done with layout before starting a splash."
                );
                let center = reference_box.as_object().paint_bounds(app).center();
                reference_box.local_to_global(app, center, None)
            }
            None => details.expect("asserted above").global_position,
        };
        self.states_controller(app)
            .update(app, WidgetState::Pressed, true); // ... before creating the splash
        let splash = self.create_splash(app, global_position);
        app.get_mut(self)
            .splashes
            .get_or_insert_with(HashSet::new)
            .insert(splash.clone());
        if let Some(current_splash) = app.get(self).current_splash.handle() {
            current_splash.cancel(app);
        }
        app.get_mut(self).current_splash = Some(splash);
        self.update_keep_alive(app);
        self.update_highlight(app, HighlightType::Pressed, true, true);
    }

    fn handle_tap(self: Handle<Self>, app: &mut App) {
        if let Some(current_splash) = app.get_mut(self).current_splash.take() {
            current_splash.confirm(app);
        }
        self.update_highlight(app, HighlightType::Pressed, false, true);
        if let Some(on_tap) = self.widget(app).response.on_tap.clone() {
            if self.widget(app).response.enable_feedback {
                let context = self.context(app);
                Feedback::for_tap(app, context);
            }
            on_tap.call(app);
        }
    }

    fn handle_tap_cancel(self: Handle<Self>, app: &mut App) {
        if let Some(current_splash) = app.get_mut(self).current_splash.take() {
            current_splash.cancel(app);
        }
        if let Some(on_tap_cancel) = self.widget(app).response.on_tap_cancel.clone() {
            on_tap_cancel.call(app);
        }
        self.update_highlight(app, HighlightType::Pressed, false, true);
    }

    /// Dart's `handleDoubleTap`, which the gesture detector calls once it recognizes double
    /// taps.
    #[allow(dead_code)]
    fn handle_double_tap(self: Handle<Self>, app: &mut App) {
        if let Some(current_splash) = app.get_mut(self).current_splash.take() {
            current_splash.confirm(app);
        }
        self.update_highlight(app, HighlightType::Pressed, false, true);
        if let Some(on_double_tap) = self.widget(app).response.on_double_tap.clone() {
            on_double_tap.call(app);
        }
    }

    fn handle_long_press(self: Handle<Self>, app: &mut App) {
        if let Some(current_splash) = app.get_mut(self).current_splash.take() {
            current_splash.confirm(app);
        }
        if let Some(on_long_press) = self.widget(app).response.on_long_press.clone() {
            if self.widget(app).response.enable_feedback {
                let context = self.context(app);
                Feedback::for_long_press(app, context);
            }
            on_long_press.call(app);
        }
    }

    fn handle_long_press_up(self: Handle<Self>, app: &mut App) {
        if let Some(current_splash) = app.get_mut(self).current_splash.take() {
            current_splash.confirm(app);
        }
        if let Some(on_long_press_up) = self.widget(app).response.on_long_press_up.clone() {
            on_long_press_up.call(app);
        }
    }

    fn handle_secondary_tap(self: Handle<Self>, app: &mut App) {
        if let Some(current_splash) = app.get_mut(self).current_splash.take() {
            current_splash.confirm(app);
        }
        self.update_highlight(app, HighlightType::Pressed, false, true);
        if let Some(on_secondary_tap) = self.widget(app).response.on_secondary_tap.clone() {
            on_secondary_tap.call(app);
        }
    }

    fn handle_secondary_tap_cancel(self: Handle<Self>, app: &mut App) {
        if let Some(current_splash) = app.get_mut(self).current_splash.take() {
            current_splash.cancel(app);
        }
        if let Some(on_secondary_tap_cancel) =
            self.widget(app).response.on_secondary_tap_cancel.clone()
        {
            on_secondary_tap_cancel.call(app);
        }
        self.update_highlight(app, HighlightType::Pressed, false, true);
    }

    fn is_widget_enabled(widget: &InkResponseStateWidget) -> bool {
        Self::primary_button_enabled(widget)
            || Self::secondary_button_enabled(widget)
            || Self::tertiary_button_enabled(widget)
    }

    fn primary_button_enabled(widget: &InkResponseStateWidget) -> bool {
        let response = &widget.response;
        response.on_tap.is_some()
            || response.on_double_tap.is_some()
            || response.on_long_press.is_some()
            || response.on_long_press_up.is_some()
            || response.on_tap_up.is_some()
            || response.on_tap_down.is_some()
    }

    fn secondary_button_enabled(widget: &InkResponseStateWidget) -> bool {
        let response = &widget.response;
        response.on_secondary_tap.is_some()
            || response.on_secondary_tap_up.is_some()
            || response.on_secondary_tap_down.is_some()
    }

    fn tertiary_button_enabled(widget: &InkResponseStateWidget) -> bool {
        let response = &widget.response;
        response.on_tertiary_tap_up.is_some() || response.on_tertiary_tap_down.is_some()
    }

    fn enabled(self: Handle<Self>, app: &App) -> bool {
        Self::is_widget_enabled(self.widget(app))
    }

    fn primary_enabled(self: Handle<Self>, app: &App) -> bool {
        Self::primary_button_enabled(self.widget(app))
    }

    fn secondary_enabled(self: Handle<Self>, app: &App) -> bool {
        Self::secondary_button_enabled(self.widget(app))
    }

    fn tertiary_enabled(self: Handle<Self>, app: &App) -> bool {
        Self::tertiary_button_enabled(self.widget(app))
    }

    fn handle_mouse_enter(self: Handle<Self>, app: &mut App, _event: PointerEnterEvent) {
        app.get_mut(self).hovering = true;
        if self.enabled(app) {
            self.handle_hover_change(app);
        }
    }

    fn handle_mouse_exit(self: Handle<Self>, app: &mut App, _event: PointerExitEvent) {
        app.get_mut(self).hovering = false;
        // If the exit occurs after we've been disabled, we still
        // want to take down the highlights and run widget.onHover.
        self.handle_hover_change(app);
    }

    fn handle_hover_change(self: Handle<Self>, app: &mut App) {
        let hovering = app.get(self).hovering;
        self.update_highlight(app, HighlightType::Hover, hovering, true);
    }

    fn can_request_focus(self: Handle<Self>, app: &mut App) -> bool {
        let context = self.context(app);
        match MediaQuery::maybe_navigation_mode_of(app, context) {
            Some(NavigationMode::Traditional) | None => {
                self.enabled(app) && self.widget(app).response.can_request_focus
            }
            Some(NavigationMode::Directional) => true,
        }
    }
}

impl AutomaticKeepAliveClientMixin for InkResponseState {
    fn automatic_keep_alive_client_data(
        self: Handle<Self>,
        app: &App,
    ) -> &AutomaticKeepAliveClientMixinData {
        &app.get(self).automatic_keep_alive_client
    }

    fn automatic_keep_alive_client_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut AutomaticKeepAliveClientMixinData {
        &mut app.get_mut(self).automatic_keep_alive_client
    }

    fn want_keep_alive(self: Handle<Self>, app: &App) -> bool {
        self.highlights_exist(app)
            || app
                .get(self)
                .splashes
                .as_ref()
                .is_some_and(|splashes| !splashes.is_empty())
    }
}

impl State for InkResponseState {
    type Widget = InkResponseStateWidget;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        AutomaticKeepAliveClientMixin::init_state(self, app);
        let activate = CallbackAction::<ActivateIntent>::new(
            app,
            Rc::new(move |app: &mut App, _intent: &ActivateIntent| {
                self.activate_on_intent(app);
                None
            }),
        );
        let button_activate = CallbackAction::<ButtonActivateIntent>::new(
            app,
            Rc::new(move |app: &mut App, _intent: &ButtonActivateIntent| {
                self.activate_on_intent(app);
                None
            }),
        );
        let action_map = &mut app.get_mut(self).action_map;
        action_map.insert(TypeId::of::<ActivateIntent>(), activate.into_erased());
        action_map.insert(
            TypeId::of::<ButtonActivateIntent>(),
            button_activate.into_erased(),
        );
        self.init_states_controller(app);
        let listener: HighlightModeListener =
            Rc::new(move |app, mode| self.handle_focus_highlight_mode_change(app, mode));
        app.get_mut(self).focus_highlight_mode_listener = Some(Rc::clone(&listener));
        FocusManager::instance(app).add_highlight_mode_listener(app, listener);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &InkResponseStateWidget) {
        let widget = &self.widget(app).response;
        let old = &old_widget.response;
        if widget.states_controller != old.states_controller {
            if let Some(old_states_controller) = old.states_controller.clone() {
                old_states_controller.remove_listener(
                    app,
                    &Listener::handle_method(self, Self::handle_states_controller_change),
                );
            }
            if self.widget(app).response.states_controller.is_some()
                && let Some(internal) = app.get_mut(self).internal_states_controller.take()
            {
                internal.dispose(app);
            }
            self.init_states_controller(app);
        }
        let widget = &self.widget(app).response;
        if widget.radius != old.radius
            || widget.highlight_shape != old.highlight_shape
            || widget.border_radius != old.border_radius
        {
            if let Some(hover_highlight) = app
                .get(self)
                .highlights
                .get(&HighlightType::Hover)
                .and_then(|highlight| highlight.handle())
            {
                InkFeatureLeaf::dispose(hover_highlight, app);
                let hovering = app.get(self).hovering;
                self.update_highlight(app, HighlightType::Hover, hovering, false);
            }
            if let Some(focus_highlight) = app
                .get(self)
                .highlights
                .get(&HighlightType::Focus)
                .and_then(|highlight| highlight.handle())
            {
                InkFeatureLeaf::dispose(focus_highlight, app);
                // Do not call updateFocusHighlights() here because it is called below
            }
        }
        let custom_border_changed = self.widget(app).response.custom_border != old.custom_border;
        if custom_border_changed {
            self.update_highlights_and_splashes(app);
        }
        let enabled = self.enabled(app);
        if enabled != Self::is_widget_enabled(old_widget) {
            let states_controller = self.states_controller(app);
            states_controller.update(app, WidgetState::Disabled, !enabled);
            if !enabled {
                states_controller.update(app, WidgetState::Pressed, false);
                // Remove the existing hover highlight immediately when enabled is false.
                // Do not rely on updateHighlight or InkHighlight.deactivate to not break
                // the expected lifecycle which is updating _hovering when the mouse exit.
                // Manually updating _hovering here or calling InkHighlight.deactivate
                // will lead to onHover not being called or call when it is not allowed.
                if let Some(hover_highlight) = app
                    .get(self)
                    .highlights
                    .get(&HighlightType::Hover)
                    .and_then(|highlight| highlight.handle())
                {
                    InkFeatureLeaf::dispose(hover_highlight, app);
                }
            }
            // Don't call widget.onHover because many widgets, including the button
            // widgets, apply setState to an ancestor context from onHover.
            let hovering = app.get(self).hovering;
            self.update_highlight(app, HighlightType::Hover, hovering, false);
        }
        self.update_focus_highlights(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(listener) = app.get_mut(self).focus_highlight_mode_listener.take() {
            FocusManager::instance(app).remove_highlight_mode_listener(app, &listener);
        }
        self.states_controller(app).remove_listener(
            app,
            &Listener::handle_method(self, Self::handle_states_controller_change),
        );
        if let Some(internal) = app.get_mut(self).internal_states_controller.take() {
            internal.dispose(app);
        }
        if let Some(timer) = app.get_mut(self).activation_timer.take() {
            timer.cancel(app);
        }
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        if let Some(splashes) = app.get_mut(self).splashes.take() {
            for splash in splashes {
                splash.dispose(app);
            }
            app.get_mut(self).current_splash = None;
        }
        debug_assert!(app.get(self).current_splash.is_none());
        let highlights: Vec<HighlightType> = app.get(self).highlights.keys().copied().collect();
        for highlight in highlights {
            if let Some(ink) = app
                .get(self)
                .highlights
                .get(&highlight)
                .and_then(|highlight| highlight.handle())
            {
                InkFeatureLeaf::dispose(ink, app);
            }
            app.get_mut(self).highlights.insert(highlight, None);
        }
        if let Some(parent_state) = self.widget(app).parent_state {
            parent_state.mark_child_ink_response_pressed(app, self, false);
        }
        AutomaticKeepAliveClientMixin::deactivate(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let response = self.widget(app).response.clone();
        debug_assert!((response.debug_check_context)(app, context));
        AutomaticKeepAliveClientMixin::build(self, app); // See AutomaticKeepAliveClientMixin.

        let theme = Theme::of(app, context);
        let highlightable_states = [
            WidgetState::Focused,
            WidgetState::Hovered,
            WidgetState::Pressed,
        ];
        let non_highlightable_states: WidgetStates = self
            .states(app)
            .into_iter()
            .filter(|state| !highlightable_states.contains(state))
            .collect();
        // Each highlightable state will be resolved separately to get the corresponding color.
        // For this resolution to be correct, the non-highlightable states should be preserved.
        let with = |state: WidgetState| {
            let mut states = non_highlightable_states.clone();
            states.insert(state);
            states
        };
        let pressed = with(WidgetState::Pressed);
        let focused = with(WidgetState::Focused);
        let hovered = with(WidgetState::Hovered);

        let get_highlight_color_for_type = |r#type: HighlightType| -> Color {
            let overlay = |states: &WidgetStates| {
                response
                    .overlay_color
                    .as_ref()
                    .and_then(|overlay_color| overlay_color.resolve(states))
            };
            match r#type {
                // The pressed state triggers a ripple (ink splash), per the current
                // Material Design spec. A separate highlight is no longer used.
                // See https://material.io/design/interaction/states.html#pressed
                HighlightType::Pressed => overlay(&pressed)
                    .or(response.highlight_color)
                    .unwrap_or(theme.highlight_color),
                HighlightType::Focus => overlay(&focused)
                    .or(response.focus_color)
                    .unwrap_or(theme.focus_color),
                HighlightType::Hover => overlay(&hovered)
                    .or(response.hover_color)
                    .unwrap_or(theme.hover_color),
            }
        };

        let highlights: Vec<(HighlightType, Option<Handle<InkHighlight>>)> = app
            .get(self)
            .highlights
            .iter()
            .map(|(r#type, highlight)| (*r#type, highlight.handle()))
            .collect();
        for (r#type, highlight) in highlights {
            if let Some(highlight) = highlight {
                InteractiveInkFeature::set_color(
                    highlight,
                    app,
                    get_highlight_color_for_type(r#type),
                );
            }
        }

        if let Some(current_splash) = app.get(self).current_splash.handle() {
            let states = self.states(app);
            let color = response
                .overlay_color
                .as_ref()
                .and_then(|overlay_color| overlay_color.resolve(&states))
                .or(response.splash_color)
                .unwrap_or(theme.splash_color);
            current_splash.set_color(app, color);
        }

        let states = self.states(app);
        let mouse_cursor: MouseCursorRef = response
            .mouse_cursor
            .clone()
            .unwrap_or_else(|| Rc::new(WidgetStateMouseCursor::adaptive_clickable()));
        let effective_mouse_cursor =
            <dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(&mouse_cursor, &states);

        let primary_enabled = self.primary_enabled(app);
        let secondary_enabled = self.secondary_enabled(app);
        let tertiary_enabled = self.tertiary_enabled(app);
        let mut detector = GestureDetector::new().behavior(HitTestBehavior::Opaque);
        if primary_enabled {
            detector = detector
                .on_tap_down(Rc::new(move |app, details| {
                    self.handle_tap_down(app, details)
                }))
                .on_tap_up(Rc::new(move |app, details| {
                    self.handle_tap_up(app, details)
                }))
                .on_tap(Listener::new(move |app| self.handle_tap(app)))
                .on_tap_cancel(Listener::new(move |app| self.handle_tap_cancel(app)));
        }
        // `onDoubleTap: handleDoubleTap` waits with `DoubleTapGestureRecognizer` (PORTING.md).
        if response.on_long_press.is_some() {
            detector =
                detector.on_long_press(Listener::new(move |app| self.handle_long_press(app)));
        }
        if response.on_long_press_up.is_some() {
            detector =
                detector.on_long_press_up(Listener::new(move |app| self.handle_long_press_up(app)));
        }
        if secondary_enabled {
            detector = detector
                .on_secondary_tap_down(Rc::new(move |app, details| {
                    self.handle_secondary_tap_down(app, details)
                }))
                .on_secondary_tap_up(Rc::new(move |app, details| {
                    self.handle_secondary_tap_up(app, details)
                }))
                .on_secondary_tap(Listener::new(move |app| self.handle_secondary_tap(app)))
                .on_secondary_tap_cancel(Listener::new(move |app| {
                    self.handle_secondary_tap_cancel(app)
                }));
        }
        if tertiary_enabled {
            detector = detector
                .on_tertiary_tap_down(Rc::new(move |app, details| {
                    self.handle_tertiary_tap_down(app, details)
                }))
                .on_tertiary_tap_up(Rc::new(move |app, details| {
                    self.handle_tertiary_tap_up(app, details)
                }))
                .on_tertiary_tap_cancel(Listener::new(move |app| {
                    self.handle_tertiary_tap_cancel(app)
                }));
        }
        if let Some(child) = response.child.clone() {
            detector = detector.child(child);
        }

        let can_request_focus = self.can_request_focus(app);
        let mut semantics = Semantics::new();
        if !(response.exclude_from_semantics || response.on_tap.is_none()) {
            semantics = semantics.on_tap(Rc::new(move |app: &mut App| self.simulate_tap(app)));
        }
        if !(response.exclude_from_semantics || response.on_long_press.is_none()) {
            semantics = semantics
                .on_long_press(Rc::new(move |app: &mut App| self.simulate_long_press(app)));
        }
        let selection_style = DefaultSelectionStyle::merge(semantics.child(detector))
            .mouse_cursor(effective_mouse_cursor.clone());
        let region = MouseRegion::new()
            .cursor(effective_mouse_cursor)
            .on_enter(Rc::new(move |app, event| {
                self.handle_mouse_enter(app, event)
            }))
            .on_exit(Rc::new(move |app, event| {
                self.handle_mouse_exit(app, event)
            }))
            .child(selection_style);
        let mut focus = Focus::new(region)
            .can_request_focus(can_request_focus)
            .on_focus_change(move |app, has_focus| self.handle_focus_update(app, has_focus))
            .autofocus(response.autofocus);
        if let Some(focus_node) = response.focus_node {
            focus = focus.focus_node(focus_node);
        }
        ParentInkResponseProvider {
            state: self,
            child: Actions::new(app.get(self).action_map.clone(), focus).into_widget(),
        }
        .into_widget()
    }
}

/// A rectangular area of a [`Material`] that responds to touch.
///
/// For a variant of this widget that does not clip splashes, see [`InkResponse`].
///
/// The [`InkWell`] widget must have a [`Material`] widget as an ancestor. The
/// [`Material`] widget is where the ink reactions are actually painted. This
/// matches the Material Design premise wherein the [`Material`] is what is
/// actually reacting to touches by spreading ink.
///
/// If a Widget uses this class directly, it should include the following line
/// at the top of its build function to call [`debug_check_has_material`]:
///
/// ```text
/// debug_assert!(debug_check_has_material(app, context));
/// ```
///
/// ## Troubleshooting
///
/// ### The ink splashes aren't visible!
///
/// If there is an opaque graphic, e.g. painted using a `Container`, `Image`, or
/// `DecoratedBox`, between the [`Material`] widget and the [`InkWell`] widget, then
/// the splash won't be visible because it will be under the opaque graphic.
/// This is because ink splashes draw on the underlying [`Material`] itself, as
/// if the ink was spreading inside the material.
///
/// The [`Ink`](crate::Ink) widget can be used as a replacement for `Image`, `Container`, or
/// `DecoratedBox` to ensure that the image or decoration also paints in the
/// [`Material`] itself, below the ink.
///
/// If this is not possible for some reason, e.g. because you are using an
/// opaque `CustomPaint` widget, alternatively consider using a second
/// [`Material`] above the opaque widget but below the [`InkWell`] (as an
/// ancestor to the ink well). The [`MaterialType::Transparency`](crate::MaterialType)
/// material kind can be used for this purpose.
///
/// ### InkWell isn't clipping properly
///
/// If you want to clip an InkWell or any [`Ink`](crate::Ink) widgets you need to keep in mind
/// that the [`Material`] that the Ink will be printed on is responsible for clipping.
/// This means you can't wrap the [`Ink`](crate::Ink) widget in a clipping widget directly,
/// since this will leave the [`Material`] not clipped (and by extension the printed
/// [`Ink`](crate::Ink) widgets as well).
///
/// An easy solution is to deliberately wrap the [`Ink`](crate::Ink) widgets you want to clip
/// in a [`Material`], and wrap that in a clipping widget instead. See [`Ink`](crate::Ink) for
/// an example.
///
/// ### The ink splashes don't track the size of an animated container
/// If the size of an InkWell's [`Material`] ancestor changes while the InkWell's
/// splashes are expanding, you may notice that the splashes aren't clipped
/// correctly. This can't be avoided.
///
/// An InkWell's splashes will not properly update to conform to changes if the
/// size of its underlying [`Material`], where the splashes are rendered, changes
/// during animation. You should avoid using InkWells within [`Material`] widgets
/// that are changing size.
///
/// See also:
///
///  * [`GestureDetector`], for listening for gestures without ink splashes.
///  * `ElevatedButton` and `TextButton`, two kinds of buttons in Material Design.
///  * [`InkResponse`], a variant of [`InkWell`] that doesn't force a rectangular
///    shape on the ink reaction.
#[derive(Clone, Debug)]
pub struct InkWell {
    /// Dart's `extends InkResponse`: the fields it inherits.
    pub ink_response: InkResponse,
}

impl Default for InkWell {
    fn default() -> InkWell {
        InkWell {
            ink_response: InkResponse {
                contained_ink_well: true,
                highlight_shape: BoxShape::Rectangle,
                ..InkResponse::default()
            },
        }
    }
}

impl InkWell {
    /// Creates an ink well; Dart's named arguments are the setters.
    ///
    /// Must have an ancestor [`Material`] widget in which to cause ink reactions.
    pub fn new() -> InkWell {
        InkWell::default()
    }

    fn ink_response_mut(&mut self) -> &mut InkResponse {
        &mut self.ink_response
    }

    ink_response_shared_setters!(InkWell);
}

impl StatelessWidget for InkWell {
    fn key(&self) -> Option<&KeyRef> {
        self.ink_response.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        self.ink_response.build_state_widget(app, context)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use waft_embedder::{Rect, TargetPlatform, valo};
    use waft_gestures::{K_SECONDARY_BUTTON, K_TERTIARY_BUTTON};
    use waft_widgets::{
        Align, Builder, Center, FocusHighlightStrategy, FocusNode, GlobalKey, MediaQueryData,
        Overlay, OverlayEntry, OverlayPortal, OverlayPortalController, Padding, Positioned,
        RepaintBoundary, Row, SizedBox, WidgetStatesController,
    };

    use super::*;
    use crate::colors::Colors;
    use crate::material::RenderInkFeatures;
    use crate::no_splash::NoSplash;
    use crate::test_support::{Drawn, PaintInspection, WidgetTester, with_main_thread_stack};
    use crate::theme_data::ThemeData;

    fn tester() -> WidgetTester {
        WidgetTester::on(TargetPlatform::Android)
    }

    type Log = Rc<RefCell<Vec<String>>>;

    fn logger(log: &Log, entry: &'static str) -> Listener {
        let log = Rc::clone(log);
        Listener::new(move |_app| log.borrow_mut().push(entry.to_string()))
    }

    fn logged<T: 'static>(log: &Log, entry: &'static str) -> ValueChanged<T> {
        let log = Rc::clone(log);
        Rc::new(move |_app, _details| log.borrow_mut().push(entry.to_string()))
    }

    fn take(log: &Log) -> Vec<String> {
        std::mem::take(&mut log.borrow_mut())
    }

    fn ltr<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Directionality::new(TextDirection::Ltr, child).into_widget()
    }

    /// `Material(child: Directionality(ltr, child))`.
    fn material_ltr(child: WidgetRef) -> WidgetRef {
        Material::new()
            .child(Directionality::new(TextDirection::Ltr, child))
            .into_widget()
    }

    /// `Center(child: SizedBox.square(dimension: 100, child: child))`.
    fn centered_square(child: impl IntoWidget<waft_widgets::StatelessKind>) -> WidgetRef {
        Center::new()
            .child(SizedBox::square(Some(100.0)).child(child))
            .into_widget()
    }

    /// `Align(topLeft, SizedBox.square(dimension: 100, child: child))`.
    fn top_left_square(child: impl IntoWidget<waft_widgets::StatelessKind>) -> WidgetRef {
        Align::new()
            .alignment(waft_painting::AlignmentGeometry::TOP_LEFT)
            .child(SizedBox::square(Some(100.0)).child(child))
            .into_widget()
    }

    fn ink_drawn(tester: &WidgetTester) -> Vec<Drawn> {
        tester.ink_drawn()
    }

    fn rects(drawn: &[Drawn]) -> Vec<(u32, Rect)> {
        drawn
            .iter()
            .filter_map(|drawn| match drawn {
                Drawn::Rect(color, rect) => Some((*color, *rect)),
                _ => None,
            })
            .collect()
    }

    fn paths(drawn: &[Drawn]) -> Vec<(u32, Rect)> {
        drawn
            .iter()
            .filter_map(|drawn| match drawn {
                Drawn::Path(color, bounds) => Some((*color, *bounds)),
                _ => None,
            })
            .collect()
    }

    /// The highlights of the material with the color each paints now and its reference box's
    /// rect in the view: Dart's `paints..rect` where the color is transparent, which valo
    /// records no draw for.
    fn highlights(tester: &WidgetTester) -> Vec<(u32, Rect)> {
        let render = tester.first::<RenderInkFeatures>();
        let app = tester.cell.borrow();
        render
            .debug_ink_features(&app)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|feature| feature.downcast::<InkHighlight>(&app))
            .map(|highlight| {
                let reference_box = crate::material::InkFeature::reference_box(highlight, &app);
                let origin = reference_box.local_to_global(&app, Offset::ZERO, None);
                (
                    highlight.debug_paint_color(&app).value(),
                    origin & reference_box.size(&app),
                )
            })
            .collect()
    }

    const CENTER_RECT: Rect = Rect::from_ltrb(350.0, 250.0, 450.0, 350.0);
    const TOP_LEFT_RECT: Rect = Rect::from_ltrb(0.0, 0.0, 100.0, 100.0);

    fn center_of_view() -> Offset {
        WidgetTester::SIZE.center(Offset::ZERO)
    }

    fn set_highlight_strategy(tester: &WidgetTester, strategy: FocusHighlightStrategy) {
        let mut app = tester.cell.borrow_mut();
        FocusManager::instance(&mut app).set_highlight_strategy(&mut app, strategy);
    }

    fn hover(tester: &mut WidgetTester, at: Offset) -> crate::test_support::TestPointer {
        let mut mouse = WidgetTester::mouse(1, 0);
        tester.add_pointer(&mut mouse, Offset::ZERO);
        tester.move_to(&mut mouse, at, false);
        tester.pump_and_settle();
        mouse
    }

    fn focus_node(tester: &WidgetTester) -> FocusNodeRef {
        FocusNode::new(&mut tester.cell.borrow_mut()).into_erased()
    }

    #[test]
    fn ink_well_gestures_control_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let log: Log = Rc::default();
            tester.pump_widget(ltr(Material::new().child(
                Center::new().child(
                    InkWell::new()
                        .on_tap(logger(&log, "tap"))
                        .on_long_press(logger(&log, "long-press"))
                        .on_long_press_up(logger(&log, "long-press-up"))
                        .on_tap_down(logged(&log, "tap-down"))
                        .on_tap_up(logged(&log, "tap-up"))
                        .on_tap_cancel(logger(&log, "tap-cancel")),
                ),
            )));

            tester.tap(center_of_view(), 1);
            tester.pump(Duration::from_secs(1));
            assert_eq!(take(&log), ["tap-down", "tap-up", "tap"]);

            tester.long_press(center_of_view(), 4);
            assert_eq!(
                take(&log),
                ["tap-down", "tap-cancel", "long-press", "long-press-up"]
            );

            let mut gesture = WidgetTester::touch(5);
            tester.down(&mut gesture, center_of_view());
            tester.pump(Duration::from_millis(100));
            assert_eq!(*log.borrow(), ["tap-down"]);
            tester.up(&gesture);
            tester.pump(Duration::from_secs(1));
            assert_eq!(take(&log), ["tap-down", "tap-up", "tap"]);

            let mut gesture = WidgetTester::touch(6);
            tester.down(&mut gesture, center_of_view());
            tester.pump(Duration::from_millis(100));
            tester.move_to(
                &mut gesture,
                center_of_view() + Offset::new(0.0, 200.0),
                true,
            );
            tester.cancel(&gesture);
            assert_eq!(take(&log), ["tap-down", "tap-cancel"]);
        });
    }

    #[test]
    fn ink_well_only_on_tap_down_enables_gestures() {
        // Regression test for https://github.com/flutter/flutter/issues/96030
        with_main_thread_stack(|| {
            let mut tester = tester();
            let down_tapped = Rc::new(Cell::new(false));
            tester.pump_widget(ltr(Material::new().child(Center::new().child(
                InkWell::new().on_tap_down({
                    let down_tapped = Rc::clone(&down_tapped);
                    Rc::new(move |_app, _details| down_tapped.set(true))
                }),
            ))));
            tester.tap(center_of_view(), 1);
            assert!(down_tapped.get());
        });
    }

    /// Dart sends the shortcuts' keys; here the intents are invoked at a context under the
    /// ink well, where the focused `Shortcuts` would invoke them.
    #[test]
    fn ink_well_invokes_activation_actions_when_expected() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let log: Log = Rc::default();
            let inner: Rc<Cell<Option<BuildContext>>> = Rc::default();
            tester.pump_widget(ltr(Material::new().child(
                Center::new().child(
                    InkWell::new()
                        .autofocus(true)
                        .on_tap(logger(&log, "tap"))
                        .child(Builder::new({
                            let inner = Rc::clone(&inner);
                            move |_app, context| {
                                inner.set(Some(context));
                                SizedBox::shrink().into_widget()
                            }
                        })),
                ),
            )));
            let context = inner.get().expect("built");
            Actions::invoke(&mut tester.cell.borrow_mut(), context, &ActivateIntent);
            tester.pump(Duration::ZERO);
            assert_eq!(take(&log), ["tap"]);
            Actions::invoke(
                &mut tester.cell.borrow_mut(),
                context,
                &ButtonActivateIntent,
            );
            tester.pump(Duration::ZERO);
            assert_eq!(take(&log), ["tap"]);
        });
    }

    #[test]
    fn ink_well_on_long_press_up_callback_is_triggered() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let was_called = Rc::new(Cell::new(false));
            tester.pump_widget(material_ltr(
                InkWell::new()
                    .on_long_press(Listener::new(|_app| {}))
                    .on_long_press_up({
                        let was_called = Rc::clone(&was_called);
                        Listener::new(move |_app| was_called.set(true))
                    })
                    .child(SizedBox::new().width(100.0).height(100.0))
                    .into_widget(),
            ));
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, Offset::new(50.0, 50.0));
            tester.pump(Duration::from_secs(1));
            tester.up(&gesture);
            tester.pump_and_settle();
            assert!(was_called.get());
        });
    }

    #[test]
    fn long_press_and_tap_on_disabled_should_not_throw() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(material_ltr(
                Center::new().child(InkWell::new()).into_widget(),
            ));
            tester.tap(center_of_view(), 1);
            tester.pump(Duration::from_secs(1));
            tester.long_press(center_of_view(), 1);
            tester.pump(Duration::from_secs(1));
        });
    }

    fn colored_ink_well() -> InkWell {
        InkWell::new()
            .hover_color(Color::new(0xff00ff00))
            .splash_color(Color::new(0xffff0000))
            .focus_color(Color::new(0xff0000ff))
            .highlight_color(Color::new(0xf00fffff))
            .on_tap(Listener::new(|_app| {}))
            .on_long_press(Listener::new(|_app| {}))
            .on_long_press_up(Listener::new(|_app| {}))
            .on_hover(Rc::new(|_app, _hover| {}))
    }

    fn overlay_ink_well() -> InkWell {
        InkWell::new()
            .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                |states| {
                    if states.contains(&WidgetState::Hovered) {
                        return Some(Color::new(0xff00ff00));
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(Color::new(0xff0000ff));
                    }
                    if states.contains(&WidgetState::Pressed) {
                        return Some(Color::new(0xf00fffff));
                    }
                    Some(Color::new(0xffbadbad)) // Shouldn't happen.
                },
            ))
            .on_tap(Listener::new(|_app| {}))
            .on_long_press(Listener::new(|_app| {}))
            .on_long_press_up(Listener::new(|_app| {}))
            .on_hover(Rc::new(|_app, _hover| {}))
    }

    #[test]
    fn ink_well_changes_color_on_hover() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(material_ltr(centered_square(colored_ink_well())));
            hover(&mut tester, center_of_view());
            assert!(rects(&ink_drawn(&tester)).contains(&(0xff00ff00, CENTER_RECT)));
        });
    }

    #[test]
    fn ink_well_changes_color_on_hover_with_overlay_color() {
        // Same test as 'ink well changes color on hover' except that the
        // hover color is specified with the overlayColor parameter.
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(material_ltr(centered_square(overlay_ink_well())));
            hover(&mut tester, center_of_view());
            assert!(rects(&ink_drawn(&tester)).contains(&(0xff00ff00, CENTER_RECT)));
        });
    }

    #[test]
    fn ink_response_changes_color_on_focus() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            set_highlight_strategy(&tester, FocusHighlightStrategy::AlwaysTraditional);
            let focus_node = focus_node(&tester);
            tester.pump_widget(material_ltr(centered_square(
                colored_ink_well().focus_node(focus_node.clone()),
            )));
            tester.pump_and_settle();
            assert!(rects(&ink_drawn(&tester)).is_empty());
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert!(rects(&ink_drawn(&tester)).contains(&(0xff0000ff, CENTER_RECT)));
        });
    }

    #[test]
    fn ink_response_changes_color_on_focus_with_overlay_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            set_highlight_strategy(&tester, FocusHighlightStrategy::AlwaysTraditional);
            let focus_node = focus_node(&tester);
            tester.pump_widget(material_ltr(centered_square(
                overlay_ink_well().focus_node(focus_node.clone()),
            )));
            tester.pump_and_settle();
            assert!(rects(&ink_drawn(&tester)).is_empty());
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert!(rects(&ink_drawn(&tester)).contains(&(0xff0000ff, CENTER_RECT)));
        });
    }

    #[test]
    fn ink_well_changes_color_on_pressed_with_overlay_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let pressed_color = Color::new(0xffdd00ff);
            tester.pump_widget(material_ltr(top_left_square(
                InkWell::new()
                    .splash_factory(NoSplash::splash_factory())
                    .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                        move |states| {
                            if states.contains(&WidgetState::Pressed) {
                                return Some(pressed_color);
                            }
                            Some(Color::new(0xffbadbad)) // Shouldn't happen.
                        },
                    ))
                    .on_tap(Listener::new(|_app| {})),
            )));
            tester.pump_and_settle();
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, TOP_LEFT_RECT.center());
            assert!(
                highlights(&tester).contains(&(pressed_color.with_alpha(0).value(), TOP_LEFT_RECT))
            );
            tester.pump_and_settle(); // Let the press highlight animation finish.
            assert!(rects(&ink_drawn(&tester)).contains(&(pressed_color.value(), TOP_LEFT_RECT)));
            tester.up(&gesture);
        });
    }

    const SELECTED_HOVERED_COLOR: u32 = 0xff00ff00;
    const SELECTED_FOCUSED_COLOR: u32 = 0xff0000ff;
    const SELECTED_PRESSED_COLOR: u32 = 0xff00ffff;

    /// The boilerplate of `Ink well overlayColor resolution respects WidgetState.selected`.
    fn selected_boilerplate(tester: &WidgetTester, focus_node: Option<FocusNodeRef>) -> WidgetRef {
        let states = WidgetStates::from([WidgetState::Selected]);
        let states_controller =
            WidgetStatesController::new(&mut tester.cell.borrow_mut(), Some(states));
        let mut ink_well = InkWell::new()
            .splash_factory(NoSplash::splash_factory())
            .states_controller(states_controller)
            .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                |states| {
                    if states.contains(&WidgetState::Selected) {
                        if states.contains(&WidgetState::Pressed) {
                            return Some(Color::new(SELECTED_PRESSED_COLOR));
                        }
                        if states.contains(&WidgetState::Hovered) {
                            return Some(Color::new(SELECTED_HOVERED_COLOR));
                        }
                        if states.contains(&WidgetState::Focused) {
                            return Some(Color::new(SELECTED_FOCUSED_COLOR));
                        }
                        Some(Color::new(0xffbadbad)) // Shouldn't happen.
                    } else {
                        Some(Color::new(0xff000000))
                    }
                },
            ))
            .on_tap(Listener::new(|_app| {}));
        if let Some(focus_node) = focus_node {
            ink_well = ink_well.focus_node(focus_node);
        }
        material_ltr(top_left_square(ink_well))
    }

    #[test]
    fn ink_well_overlay_color_resolution_respects_selected_when_focused() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            set_highlight_strategy(&tester, FocusHighlightStrategy::AlwaysTraditional);
            let focus_node = focus_node(&tester);
            let widget = selected_boilerplate(&tester, Some(focus_node.clone()));
            tester.pump_widget(widget);
            tester.pump_and_settle();
            assert!(rects(&ink_drawn(&tester)).is_empty());
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert!(rects(&ink_drawn(&tester)).contains(&(SELECTED_FOCUSED_COLOR, TOP_LEFT_RECT)));
        });
    }

    #[test]
    fn ink_well_overlay_color_resolution_respects_selected_when_hovered() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let widget = selected_boilerplate(&tester, None);
            tester.pump_widget(widget);
            tester.pump_and_settle();
            hover(&mut tester, TOP_LEFT_RECT.center());
            assert!(rects(&ink_drawn(&tester)).contains(&(SELECTED_HOVERED_COLOR, TOP_LEFT_RECT)));
        });
    }

    #[test]
    fn ink_well_overlay_color_resolution_respects_selected_when_pressed() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let widget = selected_boilerplate(&tester, None);
            tester.pump_widget(widget);
            tester.pump_and_settle();
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, TOP_LEFT_RECT.center());
            assert!(highlights(&tester).contains(&(
                Color::new(SELECTED_PRESSED_COLOR).with_alpha(0).value(),
                TOP_LEFT_RECT
            )));
            tester.pump_and_settle(); // Let the press highlight animation finish.
            assert!(rects(&ink_drawn(&tester)).contains(&(SELECTED_PRESSED_COLOR, TOP_LEFT_RECT)));
            tester.up(&gesture);
        });
    }

    /// An `M2` theme over `child`, as `MaterialApp(theme: ThemeData(useMaterial3: false))`.
    fn m2(tester: &WidgetTester, child: WidgetRef) -> WidgetRef {
        let data = ThemeData::builder()
            .use_material3(false)
            .build(&tester.cell.borrow());
        Theme::new(data, child).into_widget()
    }

    fn assert_splash_color(ink_well: InkWell) {
        let mut tester = tester();
        set_highlight_strategy(&tester, FocusHighlightStrategy::AlwaysTouch);
        let widget = m2(&tester, material_ltr(centered_square(ink_well)));
        tester.pump_widget(widget);
        tester.pump_and_settle();
        let mut gesture = WidgetTester::touch(1);
        tester.down(&mut gesture, center_of_view());
        tester.pump(Duration::from_millis(200)); // unconfirmed splash is well underway
        // The splash starts at radius 0 on this frame; valo records no empty circle, so one
        // more frame grows it to where it is drawn.
        tester.pump(Duration::from_millis(16));
        let circles = paths(&ink_drawn(&tester));
        assert!(
            circles.iter().any(|(color, bounds)| *color == 0xffff0000
                && bounds.center() == Offset::new(50.0, 50.0)),
            "{circles:?}"
        );
        tester.up(&gesture);
    }

    #[test]
    fn ink_response_splash_color_matches_splash_color_parameter() {
        with_main_thread_stack(|| assert_splash_color(colored_ink_well()));
    }

    #[test]
    fn ink_response_splash_color_matches_resolved_overlay_color_for_pressed() {
        with_main_thread_stack(|| {
            assert_splash_color(
                InkWell::new()
                    .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                        |states| {
                            if states.contains(&WidgetState::Hovered) {
                                return Some(Color::new(0xff00ff00));
                            }
                            if states.contains(&WidgetState::Focused) {
                                return Some(Color::new(0xff0000ff));
                            }
                            if states.contains(&WidgetState::Pressed) {
                                return Some(Color::new(0xffff0000));
                            }
                            Some(Color::new(0xffbadbad)) // Shouldn't happen.
                        },
                    ))
                    .on_tap(Listener::new(|_app| {}))
                    .on_long_press(Listener::new(|_app| {}))
                    .on_long_press_up(Listener::new(|_app| {}))
                    .on_hover(Rc::new(|_app, _hover| {})),
            )
        });
    }

    #[test]
    fn ink_response_uses_radius_for_focus_highlight() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            set_highlight_strategy(&tester, FocusHighlightStrategy::AlwaysTraditional);
            let focus_node = focus_node(&tester);
            tester.pump_widget(material_ltr(
                Center::new()
                    .child(
                        SizedBox::square(Some(100.0)).child(
                            InkResponse::new()
                                .focus_node(focus_node.clone())
                                .radius(20.0)
                                .focus_color(Color::new(0xff0000ff))
                                .on_tap(Listener::new(|_app| {})),
                        ),
                    )
                    .into_widget(),
            ));
            tester.pump_and_settle();
            assert!(paths(&ink_drawn(&tester)).is_empty());
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            let circles = paths(&ink_drawn(&tester));
            assert_eq!(circles.len(), 1);
            let (color, bounds) = circles[0];
            assert_eq!(color, 0xff0000ff);
            assert_eq!(bounds.width(), 40.0, "radius 20");
        });
    }

    #[test]
    fn ink_well_uses_border_radius_for_focus_highlight() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            set_highlight_strategy(&tester, FocusHighlightStrategy::AlwaysTraditional);
            let focus_node = focus_node(&tester);
            tester.pump_widget(material_ltr(centered_square(
                InkWell::new()
                    .focus_node(focus_node.clone())
                    .border_radius(BorderRadius::all(waft_embedder::Radius::circular(10.0)))
                    .focus_color(Color::new(0xff0000ff))
                    .on_tap(Listener::new(|_app| {})),
            )));
            tester.pump_and_settle();
            assert!(paths(&ink_drawn(&tester)).is_empty());

            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert_eq!(paths(&ink_drawn(&tester)), [(0xff0000ff, CENTER_RECT)]);
        });
    }

    #[test]
    fn ink_well_uses_border_radius_for_hover_highlight() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(material_ltr(centered_square(
                InkWell::new()
                    .border_radius(BorderRadius::all(waft_embedder::Radius::circular(10.0)))
                    .hover_color(Color::new(0xff00ff00))
                    .on_tap(Listener::new(|_app| {})),
            )));
            tester.pump_and_settle();
            assert!(paths(&ink_drawn(&tester)).is_empty());

            // Hover the ink well.
            let mut mouse = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut mouse, center_of_view());
            tester.pump_and_settle();
            assert_eq!(paths(&ink_drawn(&tester)), [(0xff00ff00, CENTER_RECT)]);
        });
    }

    #[test]
    fn ink_response_does_not_change_color_on_focus_when_on_touch_device() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            set_highlight_strategy(&tester, FocusHighlightStrategy::AlwaysTouch);
            let focus_node = focus_node(&tester);
            tester.pump_widget(material_ltr(centered_square(
                colored_ink_well().focus_node(focus_node.clone()),
            )));
            tester.pump_and_settle();
            assert!(rects(&ink_drawn(&tester)).is_empty());
            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert!(rects(&ink_drawn(&tester)).is_empty());
        });
    }

    fn active_cursor(tester: &WidgetTester) -> MouseCursorRef {
        let mut app = tester.cell.borrow_mut();
        let tracker = waft_rendering::RendererBinding::instance(&mut app).mouse_tracker(&app);
        tracker
            .debug_device_active_cursor(&app, 1)
            .expect("the mouse is over the view")
    }

    #[test]
    fn ink_well_mouse_cursor_changes_cursor_on_hover() {
        with_main_thread_stack(|| {
            use waft_services::SystemMouseCursors;
            let mut tester = tester();
            let in_region = |child: WidgetRef| {
                material_ltr(
                    MouseRegion::new()
                        .cursor(SystemMouseCursors::FORBIDDEN.into())
                        .child(child)
                        .into_widget(),
                )
            };
            let expect = |tester: &mut WidgetTester, child: WidgetRef, cursor: MouseCursorRef| {
                tester.pump_widget(in_region(child));
                assert!(*active_cursor(tester) == *cursor);
            };
            let basic: MouseCursorRef = SystemMouseCursors::BASIC.into();
            // Dart adds the mouse before the first frame; here it is added over the first tree,
            // so the tracker has the view's render tree to hit test.
            tester.pump_widget(in_region(SizedBox::shrink().into_widget()));
            let mut mouse = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut mouse, Offset::new(1.0, 1.0));

            // Test argument works
            expect(
                &mut tester,
                InkWell::new()
                    .mouse_cursor(SystemMouseCursors::CELL.into())
                    .on_tap(Listener::new(|_app| {}))
                    .into_widget(),
                SystemMouseCursors::CELL.into(),
            );
            // Test default of InkWell()
            expect(
                &mut tester,
                InkWell::new()
                    .on_tap(Listener::new(|_app| {}))
                    .into_widget(),
                basic.clone(),
            );
            // Test disabled
            expect(&mut tester, InkWell::new().into_widget(), basic.clone());
            // Test default of InkResponse()
            expect(
                &mut tester,
                InkResponse::new()
                    .on_tap(Listener::new(|_app| {}))
                    .into_widget(),
                basic.clone(),
            );
            // Test disabled
            expect(&mut tester, InkResponse::new().into_widget(), basic);
        });
    }

    /// Records the haptic feedback the platform is asked for.
    #[derive(Default)]
    struct FeedbackTester {
        haptics: RefCell<Vec<waft_embedder::HapticFeedbackType>>,
    }

    impl waft_embedder::Haptics for FeedbackTester {
        fn feedback(&self, kind: waft_embedder::HapticFeedbackType) {
            self.haptics.borrow_mut().push(kind);
        }
    }

    /// Dart's `feedback` group, counting haptics; the click system sound waits with
    /// `SystemSound`, so its count is not checked.
    fn feedback_case(enable_feedback: bool) -> usize {
        let feedback = Rc::new(FeedbackTester::default());
        let platform = waft_test::TestPlatform::new()
            .on(TargetPlatform::Android)
            .with_haptics(Rc::clone(&feedback) as Rc<dyn waft_embedder::Haptics>);
        let mut tester = WidgetTester::with_platform(platform);
        tester.pump_widget(material_ltr(
            Center::new()
                .child(
                    InkWell::new()
                        .on_tap(Listener::new(|_app| {}))
                        .on_long_press(Listener::new(|_app| {}))
                        .on_long_press_up(Listener::new(|_app| {}))
                        .enable_feedback(enable_feedback),
                )
                .into_widget(),
        ));
        tester.tap(center_of_view(), 1);
        tester.pump(Duration::from_secs(1));
        assert!(feedback.haptics.borrow().is_empty());
        tester.long_press(center_of_view(), 1);
        tester.pump(Duration::from_secs(1));
        feedback.haptics.borrow().len()
    }

    #[test]
    fn feedback_enabled_by_default() {
        with_main_thread_stack(|| assert_eq!(feedback_case(true), 1));
    }

    #[test]
    fn feedback_disabled() {
        with_main_thread_stack(|| assert_eq!(feedback_case(false), 0));
    }

    fn focus_case(navigation_mode: Option<waft_widgets::NavigationMode>) -> bool {
        let mut tester = tester();
        set_highlight_strategy(&tester, FocusHighlightStrategy::AlwaysTouch);
        let focus_node = focus_node(&tester);
        let frame = |ink_well: InkWell| {
            let child = ltr(ink_well
                .focus_node(focus_node.clone())
                .child(SizedBox::new()));
            let child = match navigation_mode {
                Some(mode) => waft_widgets::MediaQuery::new(
                    MediaQueryData::default().navigation_mode(mode),
                    child,
                )
                .into_widget(),
                None => child,
            };
            Material::new().child(child).into_widget()
        };
        tester.pump_widget(frame(
            InkWell::new()
                .autofocus(true)
                .on_tap(Listener::new(|_app| {}))
                .on_long_press(Listener::new(|_app| {}))
                .on_long_press_up(Listener::new(|_app| {}))
                .on_hover(Rc::new(|_app, _hover| {})),
        ));
        tester.pump_and_settle();
        assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
        tester.pump_widget(frame(InkWell::new()));
        tester.pump_and_settle();
        focus_node.has_primary_focus(&tester.cell.borrow())
    }

    #[test]
    fn ink_response_does_not_focus_when_disabled() {
        with_main_thread_stack(|| assert!(!focus_case(None)));
    }

    #[test]
    fn ink_response_accepts_focus_when_disabled_in_directional_navigation_mode() {
        with_main_thread_stack(|| {
            assert!(focus_case(Some(waft_widgets::NavigationMode::Directional)))
        });
    }

    #[test]
    fn when_ink_wells_are_nested_only_the_inner_one_is_triggered_by_tap_splash() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let inner_key = GlobalKey::new();
            let padded_ink_well = |key: Option<&GlobalKey>, child: WidgetRef| {
                let mut ink_well = InkWell::new()
                    .on_tap(Listener::new(|_app| {}))
                    .child(Padding::new(waft_painting::EdgeInsets::all(50.0).into()).child(child));
                if let Some(key) = key {
                    ink_well = ink_well.key(key.clone().into());
                }
                ink_well.into_widget()
            };
            let tree = padded_ink_well(
                None,
                padded_ink_well(
                    None,
                    padded_ink_well(
                        Some(&inner_key),
                        SizedBox::new().width(50.0).height(50.0).into_widget(),
                    ),
                ),
            );
            let widget = m2(
                &tester,
                material_ltr(Center::new().child(tree).into_widget()),
            );
            tester.pump_widget(widget);
            let inner = tester.center_of(&inner_key);
            let circles = |tester: &WidgetTester| paths(&ink_drawn(tester)).len();

            // Press
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, inner);
            tester.pump(Duration::from_millis(200));
            tester.pump(Duration::from_millis(16)); // valo records no circle of radius 0
            assert_eq!(circles(&tester), 1);

            // Up
            tester.up(&gesture);
            tester.pump_and_settle();
            assert!(ink_drawn(&tester).is_empty());

            // Press again
            tester.down(&mut gesture, inner);
            tester.pump(Duration::from_millis(200));
            tester.pump(Duration::from_millis(16)); // valo records no circle of radius 0
            assert_eq!(circles(&tester), 1);

            // Cancel
            tester.cancel(&gesture);
            tester.pump_and_settle();
            assert!(ink_drawn(&tester).is_empty());

            // Press again
            tester.down(&mut gesture, inner);
            tester.pump(Duration::from_millis(200));
            tester.pump(Duration::from_millis(16)); // valo records no circle of radius 0
            assert_eq!(circles(&tester), 1);

            // Use a second pointer to press
            let mut gesture2 = WidgetTester::touch(2);
            tester.down(&mut gesture2, inner);
            tester.pump(Duration::from_millis(200));
            tester.pump(Duration::from_millis(16)); // valo records no circle of radius 0
            assert_eq!(circles(&tester), 1);
            tester.up(&gesture2);
        });
    }

    #[test]
    fn disabled_and_hovered_inkwell_responds_to_mouse_exit() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let on_hover_count = Rc::new(Cell::new(0));
            let hover_value = Rc::new(Cell::new(None));
            let build_frame = |enabled: bool| {
                let mut ink_well = InkWell::new().on_hover({
                    let on_hover_count = Rc::clone(&on_hover_count);
                    let hover_value = Rc::clone(&hover_value);
                    Rc::new(move |_app, value| {
                        on_hover_count.set(on_hover_count.get() + 1);
                        hover_value.set(Some(value));
                    })
                });
                if enabled {
                    ink_well = ink_well.on_tap(Listener::new(|_app| {}));
                }
                material_ltr(centered_square(ink_well))
            };

            tester.pump_widget(build_frame(true));
            let mut mouse = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut mouse, Offset::ZERO);

            tester.move_to(&mut mouse, center_of_view(), false);
            tester.pump_and_settle();
            assert_eq!(on_hover_count.get(), 1);
            assert_eq!(hover_value.get(), Some(true));

            tester.pump_widget(build_frame(false));
            tester.pump_and_settle();
            tester.move_to(&mut mouse, Offset::ZERO, false);
            // Even though the InkWell has been disabled, the mouse-exit still
            // causes onHover(false) to be called.
            assert_eq!(on_hover_count.get(), 2);
            assert_eq!(hover_value.get(), Some(false));

            tester.move_to(&mut mouse, center_of_view(), false);
            tester.pump_and_settle();
            // We no longer see hover events because the InkWell is disabled
            // and it's no longer in the "hovering" state.
            assert_eq!(on_hover_count.get(), 2);
            assert_eq!(hover_value.get(), Some(false));

            tester.pump_widget(build_frame(true));
            tester.pump_and_settle();
            // The InkWell was enabled while it contained the mouse, however
            // we do not call onHover() because it may call setState().
            assert_eq!(on_hover_count.get(), 2);
            assert_eq!(hover_value.get(), Some(false));

            tester.move_to(&mut mouse, center_of_view() - Offset::new(1.0, 1.0), false);
            tester.pump_and_settle();
            // Moving the mouse a little within the InkWell doesn't change anything.
            assert_eq!(on_hover_count.get(), 2);
            assert_eq!(hover_value.get(), Some(false));
        });
    }

    #[test]
    fn hovered_ink_well_draws_a_transparent_highlight_when_disabled() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let build_frame = |enabled: bool| {
                let mut ink_well = InkWell::new()
                    .on_hover(Rc::new(|_app, _value| {}))
                    .hover_color(Color::new(0xff00ff00));
                if enabled {
                    ink_well = ink_well.on_tap(Listener::new(|_app| {}));
                }
                material_ltr(centered_square(ink_well))
            };

            tester.pump_widget(build_frame(true));
            // Hover the enabled InkWell.
            hover(&mut tester, center_of_view());
            assert!(rects(&ink_drawn(&tester)).contains(&(0xff00ff00, CENTER_RECT)));

            // Disable the hovered InkWell.
            tester.pump_widget(build_frame(false));
            tester.pump_and_settle();
            assert!(highlights(&tester).contains(&(0x0000ff00, CENTER_RECT)));
        });
    }

    #[test]
    fn ink_well_disposes_states_controller() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let tap_count = Rc::new(Cell::new(0));
            let build_frame = |states_controller: Option<Ref<MaterialStatesController>>| {
                let mut ink_well = InkWell::new()
                    .on_tap({
                        let tap_count = Rc::clone(&tap_count);
                        Listener::new(move |_app| tap_count.set(tap_count.get() + 1))
                    })
                    .child(SizedBox::new().width(100.0).height(20.0));
                if let Some(states_controller) = states_controller {
                    ink_well = ink_well.states_controller(states_controller);
                }
                material_ltr(Center::new().child(ink_well).into_widget())
            };

            let controller = MaterialStatesController::new(&mut tester.cell.borrow_mut(), None);
            let pressed_count = Rc::new(Cell::new(0));
            controller.add_listener(
                &mut tester.cell.borrow_mut(),
                Listener::new({
                    let pressed_count = Rc::clone(&pressed_count);
                    {
                        let controller = controller.clone();
                        move |app| {
                            if app.get(&controller).value().contains(&WidgetState::Pressed) {
                                pressed_count.set(pressed_count.get() + 1);
                            }
                        }
                    }
                }),
            );

            tester.pump_widget(build_frame(Some(controller.clone())));
            tester.tap(center_of_view(), 1);
            tester.pump_and_settle();
            assert_eq!(tap_count.get(), 1);
            assert_eq!(pressed_count.get(), 1);

            tester.pump_widget(build_frame(None));
            tester.tap(center_of_view(), 1);
            tester.pump_and_settle();
            assert_eq!(tap_count.get(), 2);
            assert_eq!(pressed_count.get(), 1);

            tester.pump_widget(build_frame(Some(controller.clone())));
            tester.tap(center_of_view(), 1);
            tester.pump_and_settle();
            assert_eq!(tap_count.get(), 3);
            assert_eq!(pressed_count.get(), 2);
        });
    }

    #[test]
    fn ink_well_overlay_color_opacity_fades_from_0xff_when_hover_ends() {
        // Regression test for https://github.com/flutter/flutter/issues/110266
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(material_ltr(centered_square(
                InkWell::new()
                    .overlay_color(<dyn WidgetStateProperty<Option<Color>>>::resolve_with(
                        |states| {
                            states
                                .contains(&WidgetState::Hovered)
                                .then(|| Color::new(0xff00ff00))
                        },
                    ))
                    .on_tap(Listener::new(|_app| {}))
                    .on_long_press(Listener::new(|_app| {}))
                    .on_long_press_up(Listener::new(|_app| {}))
                    .on_hover(Rc::new(|_app, _hover| {})),
            )));
            let mut mouse = hover(&mut tester, center_of_view());
            tester.move_to(&mut mouse, Offset::new(10.0, 10.0), false); // fade out the overlay
            tester.pump(Duration::ZERO); // trigger the fade out animation
            // Fadeout begins with the MaterialStates.hovered overlay color
            assert!(rects(&ink_drawn(&tester)).contains(&(0xff00ff00, CENTER_RECT)));
            // 50ms fadeout is 50% complete, overlay color alpha goes from 0xff to 0x80
            tester.pump(Duration::from_millis(25));
            assert!(
                rects(&ink_drawn(&tester)).contains(&(0x8000ff00, CENTER_RECT)),
                "{:?}",
                ink_drawn(&tester)
            );
        });
    }

    #[test]
    fn ink_well_secondary_tap_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let log: Log = Rc::default();
            tester.pump_widget(ltr(Material::new().child(
                Center::new().child(
                    InkWell::new()
                        .on_secondary_tap(logger(&log, "secondary-tap"))
                        .on_secondary_tap_down(logged(&log, "secondary-tap-down"))
                        .on_secondary_tap_up(logged(&log, "secondary-tap-up"))
                        .on_secondary_tap_cancel(logger(&log, "secondary-tap-cancel")),
                ),
            )));

            let mut mouse = WidgetTester::mouse(1, K_SECONDARY_BUTTON);
            tester.down(&mut mouse, center_of_view());
            tester.up(&mouse);
            assert_eq!(
                take(&log),
                ["secondary-tap-down", "secondary-tap-up", "secondary-tap"]
            );

            let mut mouse = WidgetTester::mouse(2, K_SECONDARY_BUTTON);
            tester.down(&mut mouse, center_of_view());
            tester.move_to(&mut mouse, Offset::new(100.0, 100.0), true);
            tester.up(&mouse);
            assert_eq!(take(&log), ["secondary-tap-down", "secondary-tap-cancel"]);
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/124328.
    #[test]
    fn ink_well_secondary_tap_should_not_draw_a_splash_when_no_secondary_callbacks_are_defined() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(ltr(Material::new().child(
                Center::new().child(InkWell::new().on_tap(Listener::new(|_app| {}))),
            )));
            let mut mouse = WidgetTester::mouse(1, K_SECONDARY_BUTTON);
            tester.down(&mut mouse, center_of_view());
            tester.pump(Duration::from_millis(200));

            // No splash should be painted.
            assert!(paths(&ink_drawn(&tester)).is_empty());
            tester.up(&mouse);
        });
    }

    #[test]
    fn ink_well_tertiary_tap_test() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let log: Log = Rc::default();
            tester.pump_widget(ltr(Material::new().child(
                Center::new().child(
                    InkWell::new()
                        .on_tertiary_tap_down(logged(&log, "tertiary-tap-down"))
                        .on_tertiary_tap_up(logged(&log, "tertiary-tap-up"))
                        .on_tertiary_tap_cancel(logger(&log, "tertiary-tap-cancel")),
                ),
            )));

            let mut mouse = WidgetTester::mouse(1, K_TERTIARY_BUTTON);
            tester.down(&mut mouse, center_of_view());
            tester.up(&mouse);
            assert_eq!(take(&log), ["tertiary-tap-down", "tertiary-tap-up"]);

            let mut mouse = WidgetTester::mouse(2, K_TERTIARY_BUTTON);
            tester.down(&mut mouse, center_of_view());
            tester.move_to(&mut mouse, Offset::new(100.0, 100.0), true);
            tester.up(&mouse);
            assert_eq!(take(&log), ["tertiary-tap-down", "tertiary-tap-cancel"]);
        });
    }

    #[test]
    fn ink_well_tertiary_tap_highlights_only_when_a_tertiary_callback_is_defined() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let log: Rc<RefCell<Vec<bool>>> = Rc::default();
            let build_frame = |with_tertiary_tap_down: bool, with_tertiary_tap_up: bool| {
                let mut ink_well = InkWell::new()
                    .on_tap(Listener::new(|_app| {}))
                    .on_highlight_changed({
                        let log = Rc::clone(&log);
                        Rc::new(move |_app, value| log.borrow_mut().push(value))
                    });
                if with_tertiary_tap_down {
                    ink_well = ink_well.on_tertiary_tap_down(Rc::new(|_app, _details| {}));
                }
                if with_tertiary_tap_up {
                    ink_well = ink_well.on_tertiary_tap_up(Rc::new(|_app, _details| {}));
                }
                ltr(Material::new().child(
                    Center::new().child(SizedBox::new().width(100.0).height(100.0).child(ink_well)),
                ))
            };

            // Each of onTertiaryTapDown and onTertiaryTapUp, alone or together,
            // enables tertiary button handling.
            for (with_tertiary_tap_down, with_tertiary_tap_up) in
                [(false, true), (true, false), (true, true)]
            {
                tester.pump_widget(build_frame(with_tertiary_tap_down, with_tertiary_tap_up));
                let mut mouse = WidgetTester::mouse(1, K_TERTIARY_BUTTON);
                tester.down(&mut mouse, center_of_view());
                tester.pump(Duration::from_millis(200));
                assert_eq!(std::mem::take(&mut *log.borrow_mut()), [true]);

                tester.up(&mouse);
                tester.pump(Duration::from_millis(200));
                assert_eq!(std::mem::take(&mut *log.borrow_mut()), [false]);
            }

            // Without a tertiary callback, a tertiary button press is ignored.
            tester.pump_widget(build_frame(false, false));
            let mut mouse = WidgetTester::mouse(1, K_TERTIARY_BUTTON);
            tester.down(&mut mouse, center_of_view());
            tester.pump(Duration::from_millis(200));
            assert!(log.borrow().is_empty());
            tester.up(&mouse);
        });
    }

    #[test]
    fn try_out_hover_duration_property() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let log: Log = Rc::default();
            tester.pump_widget(ltr(Material::new().child(
                Center::new().child(
                    InkWell::new()
                        .hover_duration(Duration::from_millis(1000))
                        .on_tap(logger(&log, "tap")),
                ),
            )));
            tester.tap(center_of_view(), 1);
            tester.pump(Duration::from_secs(1));
            assert_eq!(take(&log), ["tap"]);
        });
    }

    /// Dart sends the shortcut's key; here the intent is invoked under the ink well.
    #[test]
    fn ink_well_activation_action_does_not_end_immediately() {
        // Regression test for https://github.com/flutter/flutter/issues/132377.
        with_main_thread_stack(|| {
            let mut tester = tester();
            let controller = MaterialStatesController::new(&mut tester.cell.borrow_mut(), None);
            let inner: Rc<Cell<Option<BuildContext>>> = Rc::default();
            tester.pump_widget(ltr(Material::new().child(
                Center::new().child(
                    InkWell::new()
                        .autofocus(true)
                        .on_tap(Listener::new(|_app| {}))
                        .states_controller(controller.clone())
                        .child(Builder::new({
                            let inner = Rc::clone(&inner);
                            move |_app, context| {
                                inner.set(Some(context));
                                SizedBox::new().width(10.0).height(10.0).into_widget()
                            }
                        })),
                ),
            )));

            // Invoke the InkWell activation action.
            let context = inner.get().expect("built");
            Actions::invoke(
                &mut tester.cell.borrow_mut(),
                context,
                &ButtonActivateIntent,
            );

            // The InkWell is in pressed state.
            tester.pump(Duration::from_millis(99));
            assert!(
                tester
                    .cell
                    .borrow()
                    .get(&controller)
                    .value()
                    .contains(&WidgetState::Pressed)
            );

            tester.pump_and_settle();
            assert!(
                !tester
                    .cell
                    .borrow()
                    .get(&controller)
                    .value()
                    .contains(&WidgetState::Pressed)
            );
        });
    }

    fn zero_area_size(response: WidgetRef) -> waft_embedder::Size {
        let mut tester = tester();
        let key = GlobalKey::new();
        tester.pump_widget(material_ltr(
            Center::new()
                .child(SizedBox::shrink().key(key.clone().into()).child(response))
                .into_widget(),
        ));
        tester
            .render_object(&key)
            .as_box()
            .expect("a box")
            .size(&tester.cell.borrow())
    }

    #[test]
    fn ink_response_does_not_crash_in_zero_area() {
        with_main_thread_stack(|| {
            assert_eq!(
                zero_area_size(InkResponse::new().into_widget()),
                waft_embedder::Size::ZERO
            )
        });
    }

    #[test]
    fn ink_well_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            assert_eq!(
                zero_area_size(InkWell::new().into_widget()),
                waft_embedder::Size::ZERO
            )
        });
    }

    /// The ops the material's ink features paint, which Dart's `paints` matchers read from
    /// its recording canvas.
    fn ink_ops(tester: &WidgetTester) -> Vec<valo::Op> {
        tester.ink_list().ops().to_vec()
    }

    fn op_argb(color: valo::Color) -> u32 {
        let channel = |value: f32| (value * 255.0).round() as u32;
        (channel(color.a) << 24)
            | (channel(color.r) << 16)
            | (channel(color.g) << 8)
            | channel(color.b)
    }

    fn to_rect(rect: valo::Rect) -> Rect {
        Rect::from_ltwh(
            f64::from(rect.x),
            f64::from(rect.y),
            f64::from(rect.width),
            f64::from(rect.height),
        )
    }

    /// Dart's `drawCircle` calls: each one's color and radius.
    fn circles(ops: &[valo::Op]) -> Vec<(u32, f64)> {
        ops.iter()
            .filter_map(|op| match op {
                valo::Op::DrawPath { path, paint, .. } => path
                    .as_oval()
                    .map(|oval| (op_argb(paint.color), f64::from(oval.width) / 2.0)),
                _ => None,
            })
            .collect()
    }

    /// Dart's `drawRRect` calls: each one's color, rect and corner radii.
    fn rrects(ops: &[valo::Op]) -> Vec<(u32, Rect, [[f32; 2]; 4])> {
        ops.iter()
            .filter_map(|op| match op {
                valo::Op::DrawPath { path, paint, .. } => path
                    .as_round_rect()
                    .map(|(rect, radii)| (op_argb(paint.color), to_rect(rect), radii)),
                _ => None,
            })
            .collect()
    }

    /// Dart's `clipPath` calls: each one's path.
    ///
    /// valo records Dart's `clipRect` as an axis-aligned `ClipPath` too, such as the clip to
    /// the material's bounds that `RenderInkFeatures` paints its features under; Dart's
    /// matcher counts those as `#clipRect`, so they are left out.
    fn clip_paths(ops: &[valo::Op]) -> Vec<std::sync::Arc<valo::Path>> {
        ops.iter()
            .filter_map(|op| match op {
                valo::Op::ClipPath {
                    path,
                    is_axis_aligned_rect: false,
                    ..
                } => Some(std::sync::Arc::clone(path)),
                _ => None,
            })
            .collect()
    }

    /// The path of `RRect.fromRectAndRadius(rect, Radius.circular(radius))`.
    fn rrect_path(rect: Rect, radius: f64) -> std::sync::Arc<valo::Path> {
        let mut builder = valo::PathBuilder::new();
        builder.rrect_radii(
            valo::Rect::new(
                rect.left as f32,
                rect.top as f32,
                rect.width() as f32,
                rect.height() as f32,
            ),
            [radius as f32; 4],
        );
        builder.build()
    }

    /// Dart's `coversSameAreaAs(expected, areaToCompare: area, sampleSize: sample_size)`: on
    /// a `sample_size` by `sample_size` grid over `area`, every point is inside both paths or
    /// outside both.
    ///
    /// The grid starts at `area`'s top left. Dart's matcher starts it at the origin of the
    /// plane (and adds a random jitter), so for an area away from the origin it samples beside
    /// both paths; sampling the area itself checks what the case means, which is stronger.
    fn covers_same_area_as(
        actual: &valo::Path,
        expected: &valo::Path,
        area: Rect,
        sample_size: usize,
    ) -> bool {
        (0..sample_size).all(|i| {
            (0..sample_size).all(|j| {
                let x = area.left + area.width() * i as f64 / sample_size as f64;
                let y = area.top + area.height() * j as f64 / sample_size as f64;
                let point = valo::Point::new(x as f32, y as f32);
                actual.contains(point, valo::FillRule::NonZero)
                    == expected.contains(point, valo::FillRule::NonZero)
            })
        })
    }

    /// How many of `clips` match Dart's `clipPath(pathMatcher: coversSameAreaAs(rrect of
    /// radius, areaToCompare: rect.inflate(20.0), sampleSize: 100))`; `paints` followed by
    /// that step `n` times passes when at least `n` do, in order.
    fn clips_covering_rrect(
        clips: &[std::sync::Arc<valo::Path>],
        rect: Rect,
        radius: f64,
    ) -> usize {
        let expected = rrect_path(rect, radius);
        clips
            .iter()
            .filter(|clip| covers_same_area_as(clip, &expected, rect.inflate(20.0), 100))
            .count()
    }

    fn mouse_region<K>(child: impl IntoWidget<K>) -> WidgetRef {
        MouseRegion::new().child(child).into_widget()
    }

    /// `Align(topLeft, SizedBox.square(dimension: 100, child: MouseRegion(child: child)))`.
    fn top_left_mouse_square<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Align::new()
            .alignment(waft_painting::AlignmentGeometry::TOP_LEFT)
            .child(SizedBox::square(Some(100.0)).child(mouse_region(child)))
            .into_widget()
    }

    fn circle_border() -> waft_painting::CircleBorder {
        waft_painting::CircleBorder::default()
    }

    fn rounded(radius: f64) -> waft_painting::RoundedRectangleBorder {
        waft_painting::RoundedRectangleBorder::new(
            waft_painting::BorderSide::NONE,
            BorderRadius::all(waft_embedder::Radius::circular(radius)).into(),
        )
    }

    #[test]
    fn ink_well_custom_border_clips_for_focus_highlight() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            set_highlight_strategy(&tester, FocusHighlightStrategy::AlwaysTraditional);
            let focus_node = focus_node(&tester);
            tester.pump_widget(material_ltr(top_left_mouse_square(
                InkWell::new()
                    .focus_node(focus_node.clone())
                    .border_radius(BorderRadius::all(waft_embedder::Radius::circular(10.0)))
                    .custom_border(circle_border())
                    .hover_color(Color::new(0xff00ff00))
                    .on_tap(Listener::new(|_app| {})),
            )));
            tester.pump_and_settle();
            let ops = ink_ops(&tester);
            assert_eq!(clip_paths(&ops).len(), 0);
            assert_eq!(rrects(&ops).len(), 0);

            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            let ops = ink_ops(&tester);
            assert_eq!(clip_paths(&ops).len(), 1);
            assert_eq!(rrects(&ops).len(), 1);

            // Create a rounded rectangle path with a radius that makes it similar to the custom
            // border circle.
            // The ink well custom border path should match the rounded rectangle path.
            assert_eq!(
                clips_covering_rrect(&clip_paths(&ops), TOP_LEFT_RECT, 50.0),
                1
            );
            assert_eq!(
                clips_covering_rrect(&clip_paths(&ops), TOP_LEFT_RECT, 20.0),
                0
            );
        });
    }

    #[test]
    fn ink_well_custom_border_clips_for_hover_highlight() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            tester.pump_widget(material_ltr(top_left_mouse_square(
                InkWell::new()
                    .border_radius(BorderRadius::all(waft_embedder::Radius::circular(10.0)))
                    .custom_border(circle_border())
                    .hover_color(Color::new(0xff00ff00))
                    .on_tap(Listener::new(|_app| {})),
            )));
            tester.pump_and_settle();
            let ops = ink_ops(&tester);
            assert_eq!(clip_paths(&ops).len(), 0);
            assert_eq!(rrects(&ops).len(), 0);

            // Hover the ink well.
            let mut mouse = WidgetTester::mouse(1, 0);
            tester.add_pointer(&mut mouse, TOP_LEFT_RECT.center());
            tester.pump_and_settle();
            let ops = ink_ops(&tester);
            assert_eq!(clip_paths(&ops).len(), 1);
            assert_eq!(rrects(&ops).len(), 1);

            // The ink well custom border path should match the rounded rectangle path.
            assert_eq!(
                clips_covering_rrect(&clip_paths(&ops), TOP_LEFT_RECT, 50.0),
                1
            );
        });
    }

    #[test]
    fn ink_response_radius_can_be_updated() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            set_highlight_strategy(&tester, FocusHighlightStrategy::AlwaysTraditional);
            let focus_node = focus_node(&tester);
            let boilerplate = |radius: f64| {
                material_ltr(centered_square(
                    InkResponse::new()
                        .focus_node(focus_node.clone())
                        .radius(radius)
                        .focus_color(Color::new(0xff0000ff))
                        .on_tap(Listener::new(|_app| {})),
                ))
            };

            tester.pump_widget(boilerplate(10.0));
            tester.pump_and_settle();
            assert_eq!(circles(&ink_ops(&tester)).len(), 0);

            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert_eq!(circles(&ink_ops(&tester)), [(0xff0000ff, 10.0)]);

            tester.pump_widget(boilerplate(20.0));
            tester.pump_and_settle();
            assert_eq!(circles(&ink_ops(&tester)), [(0xff0000ff, 20.0)]);
        });
    }

    #[test]
    fn ink_response_highlight_shape_can_be_updated() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            set_highlight_strategy(&tester, FocusHighlightStrategy::AlwaysTraditional);
            let focus_node = focus_node(&tester);
            let boilerplate = |shape: BoxShape| {
                material_ltr(centered_square(
                    InkResponse::new()
                        .focus_node(focus_node.clone())
                        .highlight_shape(shape)
                        .border_radius(BorderRadius::all(waft_embedder::Radius::circular(10.0)))
                        .focus_color(Color::new(0xff0000ff))
                        .on_tap(Listener::new(|_app| {})),
                ))
            };

            tester.pump_widget(boilerplate(BoxShape::Circle));
            tester.pump_and_settle();
            let ops = ink_ops(&tester);
            assert_eq!(circles(&ops).len(), 0);
            assert_eq!(rrects(&ops).len(), 0);

            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            let ops = ink_ops(&tester);
            assert_eq!(circles(&ops).len(), 1);
            assert_eq!(rrects(&ops).len(), 0);

            tester.pump_widget(boilerplate(BoxShape::Rectangle));
            tester.pump_and_settle();
            let ops = ink_ops(&tester);
            assert_eq!(circles(&ops).len(), 0);
            assert_eq!(rrects(&ops).len(), 1);
        });
    }

    #[test]
    fn ink_well_border_radius_can_be_updated() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            set_highlight_strategy(&tester, FocusHighlightStrategy::AlwaysTraditional);
            let focus_node = focus_node(&tester);
            let boilerplate = |radius: f64| {
                material_ltr(centered_square(
                    InkWell::new()
                        .focus_node(focus_node.clone())
                        .border_radius(BorderRadius::all(waft_embedder::Radius::circular(radius)))
                        .focus_color(Color::new(0xff0000ff))
                        .on_tap(Listener::new(|_app| {})),
                ))
            };

            tester.pump_widget(boilerplate(10.0));
            tester.pump_and_settle();
            assert_eq!(rrects(&ink_ops(&tester)).len(), 0);

            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert_eq!(
                rrects(&ink_ops(&tester)),
                [(0xff0000ff, CENTER_RECT, [[10.0; 2]; 4])]
            );

            tester.pump_widget(boilerplate(30.0));
            tester.pump_and_settle();
            assert_eq!(
                rrects(&ink_ops(&tester)),
                [(0xff0000ff, CENTER_RECT, [[30.0; 2]; 4])]
            );
        });
    }

    #[test]
    fn ink_well_custom_border_can_be_updated() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            set_highlight_strategy(&tester, FocusHighlightStrategy::AlwaysTraditional);
            let focus_node = focus_node(&tester);
            let boilerplate = |radius: f64| {
                material_ltr(top_left_mouse_square(
                    InkWell::new()
                        .focus_node(focus_node.clone())
                        .custom_border(rounded(radius))
                        .hover_color(Color::new(0xff00ff00))
                        .on_tap(Listener::new(|_app| {})),
                ))
            };

            tester.pump_widget(boilerplate(20.0));
            tester.pump_and_settle();
            assert_eq!(clip_paths(&ink_ops(&tester)).len(), 0);

            focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            let clips = clip_paths(&ink_ops(&tester));
            assert_eq!(clips.len(), 1);
            assert_eq!(clips_covering_rrect(&clips, TOP_LEFT_RECT, 20.0), 1);

            tester.pump_widget(boilerplate(40.0));
            tester.pump_and_settle();
            let clips = clip_paths(&ink_ops(&tester));
            assert!(clips_covering_rrect(&clips, TOP_LEFT_RECT, 40.0) >= 1);
        });
    }

    #[test]
    fn ink_well_splash_custom_border_can_be_updated() {
        // Regression test for https://github.com/flutter/flutter/issues/121626.
        with_main_thread_stack(|| {
            let mut tester = tester();
            let focus_node = focus_node(&tester);
            let boilerplate = |tester: &WidgetTester, radius: f64| {
                m2(
                    tester,
                    material_ltr(top_left_mouse_square(
                        InkWell::new()
                            .focus_node(focus_node.clone())
                            .custom_border(rounded(radius))
                            .on_tap(Listener::new(|_app| {})),
                    )),
                )
            };

            let widget = boilerplate(&tester, 20.0);
            tester.pump_widget(widget);
            tester.pump_and_settle();
            assert_eq!(clip_paths(&ink_ops(&tester)).len(), 0);

            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, TOP_LEFT_RECT.center());
            tester.pump(Duration::from_millis(200)); // Unconfirmed splash is well underway.
            let clips = clip_paths(&ink_ops(&tester));
            assert_eq!(clips.len(), 2); // Splash and highlight.

            // Check that the splash and the highlight are correctly clipped.
            assert!(clips_covering_rrect(&clips, TOP_LEFT_RECT, 20.0) >= 2);

            let widget = boilerplate(&tester, 40.0);
            tester.pump_widget(widget);
            tester.pump_and_settle();

            // Check that the splash and the highlight are correctly clipped.
            let clips = clip_paths(&ink_ops(&tester));
            assert!(clips_covering_rrect(&clips, TOP_LEFT_RECT, 40.0) >= 2);

            tester.up(&gesture);
        });
    }

    /// `InkWell(key: key, onTap: () {}, child: Padding(padding: EdgeInsets.all(50), child))`.
    fn padded_ink_well(key: Option<&GlobalKey>, child: Option<WidgetRef>) -> WidgetRef {
        let mut padding = Padding::new(waft_painting::EdgeInsets::all(50.0).into());
        if let Some(child) = child {
            padding = padding.child(child);
        }
        let mut ink_well = InkWell::new()
            .on_tap(Listener::new(|_app| {}))
            .child(padding);
        if let Some(key) = key {
            ink_well = ink_well.key(key.clone().into());
        }
        ink_well.into_widget()
    }

    /// The draws a pump of `duration` leaves, after the frame valo needs to see a splash that
    /// starts at radius 0 on the frame Dart's case reads.
    fn pump_splash(tester: &mut WidgetTester, duration: Duration) {
        tester.pump(duration);
        tester.pump(Duration::from_millis(16)); // valo records no circle of radius 0
    }

    fn circle_count(tester: &WidgetTester) -> usize {
        circles(&ink_ops(tester)).len()
    }

    fn top_left_of(tester: &WidgetTester, key: &GlobalKey) -> Offset {
        let render = tester.render_object(key).as_box().expect("a render box");
        let app = tester.cell.borrow();
        render.local_to_global(&app, Offset::ZERO, None)
    }

    /// `Material(Directionality(ltr, Align(topLeft, SizedBox(width, height, child))))` under
    /// the M2 theme.
    fn m2_top_left(tester: &WidgetTester, width: f64, height: f64, child: WidgetRef) -> WidgetRef {
        m2(
            tester,
            material_ltr(
                Align::new()
                    .alignment(waft_painting::AlignmentGeometry::TOP_LEFT)
                    .child(SizedBox::new().width(width).height(height).child(child))
                    .into_widget(),
            ),
        )
    }

    #[test]
    fn reparenting_parent_should_allow_both_inkwells_to_show_splash_afterwards() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let middle_key = GlobalKey::new();
            let inner_key = GlobalKey::new();
            let widget = m2_top_left(
                &tester,
                200.0,
                100.0,
                Row::new()
                    .children([
                        padded_ink_well(
                            Some(&middle_key),
                            Some(padded_ink_well(Some(&inner_key), None)),
                        ),
                        SizedBox::new().into_widget(),
                    ])
                    .into_widget(),
            );
            tester.pump_widget(widget);

            // Press
            let mut gesture1 = WidgetTester::touch(1);
            let inner = tester.center_of(&inner_key);
            tester.down(&mut gesture1, inner);
            pump_splash(&mut tester, Duration::from_millis(200));
            assert_eq!(circle_count(&tester), 1);

            // Reparent parent
            let widget = m2_top_left(
                &tester,
                200.0,
                100.0,
                Row::new()
                    .children([
                        padded_ink_well(Some(&inner_key), None),
                        padded_ink_well(Some(&middle_key), None),
                    ])
                    .into_widget(),
            );
            tester.pump_widget(widget);

            // Up
            tester.up(&gesture1);
            tester.pump_and_settle();
            assert!(ink_ops(&tester).is_empty(), "{:?}", ink_ops(&tester));

            // Press the previous parent
            let middle = tester.center_of(&middle_key);
            tester.down(&mut gesture1, middle);
            pump_splash(&mut tester, Duration::from_millis(200));
            assert_eq!(circle_count(&tester), 1);

            // Use a second pointer to press the previous child
            let mut gesture2 = WidgetTester::touch(2);
            let inner = tester.center_of(&inner_key);
            tester.down(&mut gesture2, inner);
            pump_splash(&mut tester, Duration::from_millis(200));
            assert_eq!(circle_count(&tester), 2);

            // Finish gesture to release resources.
            tester.up(&gesture1);
            tester.up(&gesture2);
            tester.pump_and_settle();
        });
    }

    #[test]
    fn parent_inkwell_does_not_block_child_inkwells_from_splashes() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let middle_key = GlobalKey::new();
            let inner_key = GlobalKey::new();
            let tree = padded_ink_well(
                None,
                Some(padded_ink_well(
                    Some(&middle_key),
                    Some(padded_ink_well(
                        Some(&inner_key),
                        Some(SizedBox::new().width(50.0).height(50.0).into_widget()),
                    )),
                )),
            );
            let widget = m2(
                &tester,
                material_ltr(Center::new().child(tree).into_widget()),
            );
            tester.pump_widget(widget);

            // Press middle
            let mut gesture1 = WidgetTester::touch(1);
            let middle = top_left_of(&tester, &middle_key) + Offset::new(1.0, 1.0);
            tester.down(&mut gesture1, middle);
            pump_splash(&mut tester, Duration::from_millis(200));
            assert_eq!(circle_count(&tester), 1);

            // Press inner
            let mut gesture2 = WidgetTester::touch(2);
            let inner = tester.center_of(&inner_key);
            tester.down(&mut gesture2, inner);
            pump_splash(&mut tester, Duration::from_millis(200));
            assert_eq!(circle_count(&tester), 2);

            // Finish gesture to release resources.
            tester.up(&gesture1);
            tester.up(&gesture2);
            tester.pump_and_settle();
        });
    }

    #[test]
    fn parent_inkwell_can_count_the_number_of_pressed_children_to_prevent_splash() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let parent_key = GlobalKey::new();
            let left_key = GlobalKey::new();
            let right_key = GlobalKey::new();
            let child_well = |key: &GlobalKey| {
                SizedBox::square(Some(50.0))
                    .child(
                        InkWell::new()
                            .key(key.clone().into())
                            .on_tap(Listener::new(|_app| {})),
                    )
                    .into_widget()
            };
            let parent = InkWell::new()
                .key(parent_key.clone().into())
                .on_tap(Listener::new(|_app| {}))
                .child(Center::new().child(
                    SizedBox::new().width(100.0).height(50.0).child(
                        Row::new().children([child_well(&left_key), child_well(&right_key)]),
                    ),
                ));
            let widget = m2(&tester, material_ltr(centered_square(parent)));
            tester.pump_widget(widget);

            let parent_position = top_left_of(&tester, &parent_key) + Offset::new(1.0, 1.0);

            // Press left child
            let mut gesture1 = WidgetTester::touch(1);
            let left = tester.center_of(&left_key);
            tester.down(&mut gesture1, left);
            pump_splash(&mut tester, Duration::from_millis(200));
            assert_eq!(circle_count(&tester), 1);

            // Press right child
            let mut gesture2 = WidgetTester::touch(2);
            let right = tester.center_of(&right_key);
            tester.down(&mut gesture2, right);
            pump_splash(&mut tester, Duration::from_millis(200));
            assert_eq!(circle_count(&tester), 2);

            // Press parent
            let mut gesture3 = WidgetTester::touch(3);
            tester.down(&mut gesture3, parent_position);
            pump_splash(&mut tester, Duration::from_millis(200));
            assert_eq!(circle_count(&tester), 2);
            tester.up(&gesture3);

            // Release left child
            tester.up(&gesture1);
            tester.pump_and_settle();
            assert_eq!(circle_count(&tester), 1);

            // Press parent
            tester.down(&mut gesture3, parent_position);
            pump_splash(&mut tester, Duration::from_millis(200));
            assert_eq!(circle_count(&tester), 1);
            tester.up(&gesture3);

            // Release right child
            tester.up(&gesture2);
            tester.pump_and_settle();
            assert_eq!(circle_count(&tester), 0);

            // Press parent
            tester.down(&mut gesture3, parent_position);
            pump_splash(&mut tester, Duration::from_millis(200));
            assert_eq!(circle_count(&tester), 1);
            tester.up(&gesture3);
        });
    }

    #[test]
    fn when_ink_wells_are_reparented_the_old_parent_can_display_splash_while_the_new_parent_can_not()
     {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let inner_key = GlobalKey::new();
            let left_key = GlobalKey::new();
            let right_key = GlobalKey::new();
            let inner = || {
                InkWell::new()
                    .key(inner_key.clone().into())
                    .on_tap(Listener::new(|_app| {}))
                    .into_widget()
            };
            let double_ink_well_row =
                |tester: &WidgetTester,
                 left_width: f64,
                 right_width: f64,
                 left_child: Option<WidgetRef>,
                 right_child: Option<WidgetRef>| {
                    let side = |key: &GlobalKey, width: f64, child: Option<WidgetRef>| {
                        let mut inner_box = SizedBox::new().width(left_width).height(50.0);
                        if let Some(child) = child {
                            inner_box = inner_box.child(child);
                        }
                        SizedBox::new()
                            .width(width)
                            .height(100.0)
                            .child(
                                InkWell::new()
                                    .key(key.clone().into())
                                    .on_tap(Listener::new(|_app| {}))
                                    .child(Center::new().child(inner_box)),
                            )
                            .into_widget()
                    };
                    m2_top_left(
                        tester,
                        left_width + right_width,
                        100.0,
                        Row::new()
                            .children([
                                side(&left_key, left_width, left_child),
                                side(&right_key, right_width, right_child),
                            ])
                            .into_widget(),
                    )
                };

            let widget = double_ink_well_row(&tester, 110.0, 90.0, Some(inner()), None);
            tester.pump_widget(widget);

            // Press inner
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, Offset::new(100.0, 50.0));
            pump_splash(&mut tester, Duration::from_millis(200));
            assert_eq!(circle_count(&tester), 1);

            // Switch side
            let widget = double_ink_well_row(&tester, 90.0, 110.0, None, Some(inner()));
            tester.pump_widget(widget);
            assert_eq!(circle_count(&tester), 0);

            // A second pointer presses inner
            let mut gesture2 = WidgetTester::touch(2);
            tester.down(&mut gesture2, Offset::new(100.0, 50.0));
            pump_splash(&mut tester, Duration::from_millis(200));
            assert_eq!(circle_count(&tester), 1);

            tester.up(&gesture);
            tester.up(&gesture2);
            tester.pump_and_settle();

            // Press inner
            tester.down(&mut gesture, Offset::new(100.0, 50.0));
            pump_splash(&mut tester, Duration::from_millis(200));
            assert_eq!(circle_count(&tester), 1);

            // Press left
            tester.down(&mut gesture2, Offset::new(50.0, 50.0));
            pump_splash(&mut tester, Duration::from_millis(200));
            assert_eq!(circle_count(&tester), 2);

            tester.up(&gesture);
            tester.up(&gesture2);
        });
    }

    #[test]
    fn ink_wells_splash_starts_before_tap_is_confirmed_and_disappears_after_tap_is_canceled() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let inner_key = GlobalKey::new();
            let tree = GestureDetector::new()
                .on_horizontal_drag_start(Rc::new(|_app, _details| {}))
                .child(centered_square(
                    InkWell::new().on_tap(Listener::new(|_app| {})).child(
                        Center::new().child(
                            SizedBox::square(Some(50.0)).child(
                                InkWell::new()
                                    .key(inner_key.clone().into())
                                    .on_tap(Listener::new(|_app| {})),
                            ),
                        ),
                    ),
                ));
            let widget = m2(&tester, material_ltr(tree.into_widget()));
            tester.pump_widget(widget);

            // Press
            let mut gesture = WidgetTester::touch(1);
            let inner = tester.center_of(&inner_key);
            tester.down(&mut gesture, inner);
            pump_splash(&mut tester, Duration::from_millis(200));
            assert_eq!(circle_count(&tester), 1);

            // Scroll upward
            let moved = inner + Offset::new(0.0, -100.0);
            tester.move_to(&mut gesture, moved, true);
            tester.pump_and_settle();
            assert!(ink_ops(&tester).is_empty(), "{:?}", ink_ops(&tester));

            // Up
            tester.up(&gesture);
            tester.pump_and_settle();
            assert!(ink_ops(&tester).is_empty(), "{:?}", ink_ops(&tester));

            // Press again
            tester.down(&mut gesture, inner);
            pump_splash(&mut tester, Duration::from_millis(200));
            assert_eq!(circle_count(&tester), 1);
        });
    }

    #[test]
    fn the_ink_well_widget_on_overlay_portal_does_not_throw() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let controller = {
                let mut app = tester.cell.borrow_mut();
                let controller = OverlayPortalController::new(&mut app, None);
                controller.show(&mut app);
                controller
            };
            let overlay_entry = OverlayEntry::new(
                &mut tester.cell.borrow_mut(),
                Rc::new(move |_app: &mut App, _context| {
                    Center::new()
                        .child(
                            SizedBox::square(Some(100.0)).child(
                                // The material partially overlaps the overlayChild.
                                // This is to verify that the `overlayChild`'s ink
                                // features aren't clipped by it.
                                Material::new()
                                    .color(Colors::BLACK)
                                    .child(OverlayPortal::new(
                                        controller.clone(),
                                        |_app, _context| {
                                            Positioned::new(
                                                InkWell::new()
                                                    .splash_color(Colors::RED.primary())
                                                    .on_tap(Listener::new(|_app| {}))
                                                    .child(SizedBox::square(Some(100.0))),
                                            )
                                            .right(0.0)
                                            .bottom(0.0)
                                            .into_widget()
                                        },
                                    )),
                            ),
                        )
                        .into_widget()
                }),
                false,
                false,
                false,
            );
            tester.pump_widget(
                Center::new()
                    .child(
                        RepaintBoundary::new().child(
                            SizedBox::square(Some(200.0)).child(ltr(
                                Overlay::new().initial_entries([overlay_entry.clone()])
                            )),
                        ),
                    )
                    .into_widget(),
            );

            let ink_well = tester.find_by_type::<InkWell>()[0];
            let center = {
                let render = tester.render_box_of(ink_well);
                tester.global_rect(render).center()
            };
            let mut gesture = WidgetTester::touch(1);
            tester.down(&mut gesture, center);
            tester.pump(Duration::ZERO); // start gesture
            tester.pump(Duration::from_secs(2));

            tester.up(&gesture);
            overlay_entry.remove(&mut tester.cell.borrow_mut());
            overlay_entry.dispose(&mut tester.cell.borrow_mut());
        });
    }
}
