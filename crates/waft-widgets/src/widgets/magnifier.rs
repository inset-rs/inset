//! Flutter counterpart: `widgets/magnifier.dart`.
//!
//! `RawMagnifier` paints its lens through `_RenderMagnification`, which in Flutter pushes a
//! `BackdropFilterLayer` with `ImageFilter.matrix`. The embedder cannot build a matrix filter
//! (valo has none), so the render object keeps the transform's geometry and paints its child
//! without the layer: the lens shows the content under it, unscaled. Pushing the layer is the
//! one change once valo has the filter; see `PORTING.md`.

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::sync::Arc;

use waft_animation::AnimationController;
use waft_embedder::{Clip, Matrix4, Offset, Path, PathBuilder, Rect, Size, TextBaseline};
use waft_foundation::{App, Handle, Listener, ValueNotifier};
use waft_painting::{
    Alignment, AlignmentGeometry, BoxShadow, RoundedRectangleBorder, ShapeBorder, ShapeDecoration,
};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, CustomClipper,
    PaintingContext, RenderBox, RenderBoxData, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin, RenderProxyBoxMixin,
};
use waft_scheduler::TickerFuture;

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, RenderObjectWidget, SingleChildRenderObjectWidget, State,
    StatelessWidget, WidgetRef,
};
use crate::widgets::basic::{ClipPath, IgnorePointer, Opacity, SizedBox, Stack, WidgetBuilder};
use crate::widgets::container::DecoratedBox;
use crate::widgets::inherited_theme::InheritedTheme;
use crate::widgets::navigator::Navigator;
use crate::widgets::overlay::{Overlay, OverlayEntry};

/// Signature for a builder that builds a [`WidgetRef`] with a [`MagnifierController`].
///
/// The builder is called exactly once per magnifier.
///
/// If the `controller` parameter's [`MagnifierController::animation_controller`]
/// field is set (by the builder) to an [`AnimationController`], the
/// [`MagnifierController`] will drive the animation during entry and exit.
///
/// The `magnifier_info` parameter is updated with new [`MagnifierInfo`] instances
/// during the lifetime of the built magnifier, e.g. as the user moves their
/// finger around the text field.
pub type MagnifierBuilder = Rc<
    dyn Fn(
        &mut App,
        BuildContext,
        Handle<MagnifierController>,
        Handle<ValueNotifier<MagnifierInfo>>,
    ) -> Option<WidgetRef>,
>;

/// A data class that contains the geometry information of text layouts
/// and selection gestures, used to position magnifiers.
#[derive(Clone, Copy, PartialEq)]
pub struct MagnifierInfo {
    /// The offset of the gesture position that the magnifier should be shown at.
    pub global_gesture_position: Offset,
    /// The rect of the current line the magnifier should be shown at, without
    /// taking into account any padding of the field; only the position of the
    /// first and last character.
    pub current_line_boundaries: Rect,
    /// The rect of the handle that the magnifier should follow.
    pub caret_rect: Rect,
    /// The bounds of the entire text field that the magnifier is bound to.
    pub field_bounds: Rect,
}

impl MagnifierInfo {
    /// Constructs a [`MagnifierInfo`] from provided geometry values.
    pub const fn new(
        global_gesture_position: Offset,
        caret_rect: Rect,
        field_bounds: Rect,
        current_line_boundaries: Rect,
    ) -> MagnifierInfo {
        MagnifierInfo {
            global_gesture_position,
            caret_rect,
            field_bounds,
            current_line_boundaries,
        }
    }

    /// Const [`MagnifierInfo`] with all values set to 0.
    pub const EMPTY: MagnifierInfo = MagnifierInfo {
        global_gesture_position: Offset::ZERO,
        caret_rect: Rect::ZERO,
        current_line_boundaries: Rect::ZERO,
        field_bounds: Rect::ZERO,
    };
}

impl Debug for MagnifierInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MagnifierInfo(position: {:?}, line: {:?}, caret: {:?}, field: {:?})",
            self.global_gesture_position,
            self.current_line_boundaries,
            self.caret_rect,
            self.field_bounds
        )
    }
}

/// A configuration object for a magnifier (e.g. in a text field).
///
/// In general, most features of the magnifier can be configured by controlling
/// the widgets built by the [`magnifier_builder`](Self::magnifier_builder).
#[derive(Clone)]
pub struct TextMagnifierConfiguration {
    magnifier_builder: Option<MagnifierBuilder>,
    /// Whether a magnifier should show the text editing handles or not.
    ///
    /// This flag is used by `SelectionOverlay.showMagnifier` to control the order
    /// of layers in the rendering; specifically, whether to place the layer
    /// containing the handles above or below the layer containing the magnifier
    /// in the [`Overlay`].
    pub should_display_handles_in_magnifier: bool,
}

impl TextMagnifierConfiguration {
    /// Constructs a [`TextMagnifierConfiguration`] from parts.
    ///
    /// If [`magnifier_builder`](Self::magnifier_builder) is not set, a default
    /// [`MagnifierBuilder`] will be used that does not build a magnifier.
    pub const fn new() -> TextMagnifierConfiguration {
        TextMagnifierConfiguration {
            magnifier_builder: None,
            should_display_handles_in_magnifier: true,
        }
    }

    /// The builder callback that creates the widget that renders the magnifier.
    pub fn magnifier_builder(&self) -> MagnifierBuilder {
        self.magnifier_builder
            .clone()
            .unwrap_or_else(|| Rc::new(none))
    }

    /// Dart `TextMagnifierConfiguration(magnifierBuilder:)`.
    pub fn with_magnifier_builder(
        mut self,
        magnifier_builder: MagnifierBuilder,
    ) -> TextMagnifierConfiguration {
        self.magnifier_builder = Some(magnifier_builder);
        self
    }

    /// Dart `TextMagnifierConfiguration(shouldDisplayHandlesInMagnifier:)`.
    pub fn should_display_handles_in_magnifier(
        mut self,
        should_display_handles_in_magnifier: bool,
    ) -> TextMagnifierConfiguration {
        self.should_display_handles_in_magnifier = should_display_handles_in_magnifier;
        self
    }

    pub(crate) fn same_configuration(&self, other: &TextMagnifierConfiguration) -> bool {
        self.should_display_handles_in_magnifier == other.should_display_handles_in_magnifier
            && match (&self.magnifier_builder, &other.magnifier_builder) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                _ => false,
            }
    }

    /// A constant for a [`TextMagnifierConfiguration`] that is disabled, meaning it
    /// never builds anything, regardless of platform.
    pub const DISABLED: TextMagnifierConfiguration = TextMagnifierConfiguration::new();
}

impl Default for TextMagnifierConfiguration {
    fn default() -> TextMagnifierConfiguration {
        TextMagnifierConfiguration::new()
    }
}

fn none(
    _app: &mut App,
    _context: BuildContext,
    _controller: Handle<MagnifierController>,
    _magnifier_info: Handle<ValueNotifier<MagnifierInfo>>,
) -> Option<WidgetRef> {
    None
}

/// A controller for a magnifier.
///
/// [`MagnifierController`]'s main benefit over holding a raw [`OverlayEntry`] is that
/// [`MagnifierController`] will handle logic around waiting for a magnifier to animate in or out.
///
/// If a magnifier chooses to have an entry / exit animation, it should provide the animation
/// controller to [`MagnifierController::animation_controller`]. [`MagnifierController`] will then drive
/// the [`AnimationController`] and wait for it to be complete before removing it from the
/// [`Overlay`].
///
/// To check the status of the magnifier, see [`MagnifierController::shown`].
pub struct MagnifierController {
    /// The controller that will be driven in / out when show / hide is triggered,
    /// respectively.
    pub animation_controller: Option<Handle<AnimationController>>,
    overlay_entry: Option<Handle<OverlayEntry>>,
}

impl MagnifierController {
    /// If there is no in / out animation for the magnifier, [`animation_controller`](Self::animation_controller)
    /// should be left unset.
    pub fn new(app: &mut App) -> Handle<MagnifierController> {
        app.create(MagnifierController {
            animation_controller: None,
            overlay_entry: None,
        })
    }

    /// Dart `MagnifierController(animationController:)`.
    pub fn animation_controller(
        self: Handle<Self>,
        app: &mut App,
        animation_controller: Handle<AnimationController>,
    ) -> Handle<Self> {
        animation_controller.set_value(app, 0.0);
        app.get_mut(self).animation_controller = Some(animation_controller);
        self
    }

    /// The magnifier's [`OverlayEntry`], if currently in the overlay.
    ///
    /// This is exposed so that other overlay entries can be positioned above or
    /// below this [`overlay_entry`](Self::overlay_entry). Anything in the paint order after the
    /// `RawMagnifier` in this [`OverlayEntry`] will not be displayed in the
    /// magnifier; if it is desired for an overlay entry to be displayed in the
    /// magnifier, it *must* be positioned below the magnifier.
    ///
    /// To check if a magnifier is in the overlay, use [`shown`](Self::shown). The
    /// [`overlay_entry`](Self::overlay_entry) field may be non-null even when the magnifier is
    /// not visible.
    pub fn overlay_entry(self: Handle<Self>, app: &App) -> Option<Handle<OverlayEntry>> {
        app.get(self).overlay_entry
    }

    /// Whether the magnifier is currently being shown.
    ///
    /// This is false when nothing is in the overlay, when the
    /// [`animation_controller`](Self::animation_controller) is in the
    /// [`waft_animation::AnimationStatus::Dismissed`] state, or when
    /// the [`animation_controller`](Self::animation_controller) is animating out (i.e. in the
    /// [`waft_animation::AnimationStatus::Reverse`] state).
    ///
    /// It is true in the opposite cases, i.e. when the overlay is not empty, and
    /// either the [`animation_controller`](Self::animation_controller) is `None`, in the
    /// [`waft_animation::AnimationStatus::Completed`] state, or in the
    /// [`waft_animation::AnimationStatus::Forward`]
    /// state.
    pub fn shown(self: Handle<Self>, app: &App) -> bool {
        app.get(self).overlay_entry.is_some()
            && app
                .get(self)
                .animation_controller
                .is_none_or(|controller| controller.status(app).is_forward_or_completed())
    }

    /// Displays the magnifier.
    ///
    /// Returns a future that completes when the magnifier is fully shown, i.e. done
    /// with its entry animation.
    ///
    /// To control what overlays are shown in the magnifier, use `below`. See
    /// [`overlay_entry`](Self::overlay_entry) for more details on how to utilize `below`.
    ///
    /// If the magnifier already exists (i.e. [`overlay_entry`](Self::overlay_entry) is `Some`),
    /// then [`show`](Self::show) will replace the old overlay without playing an exit
    /// animation. Consider awaiting [`hide`](Self::hide) first, to animate from the old
    /// magnifier to the new one.
    pub fn show(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        builder: WidgetBuilder,
        below: Option<Handle<OverlayEntry>>,
    ) -> TickerFuture {
        if let Some(entry) = app.get(self).overlay_entry {
            entry.remove(app);
            entry.dispose(app);
        }

        let overlay_state = Overlay::of(app, context, true);

        let to = Navigator::maybe_of(app, context, false).map(|navigator| navigator.context(app));
        let captured_themes = <dyn InheritedTheme>::capture(app, context, to);

        let overlay_entry = OverlayEntry::new(
            app,
            Rc::new(move |app: &mut App, context: BuildContext| {
                captured_themes.wrap(builder(app, context))
            }),
            false,
            false,
            false,
        );
        app.get_mut(self).overlay_entry = Some(overlay_entry);
        overlay_state.insert(app, overlay_entry, below, None);

        if let Some(controller) = app.get(self).animation_controller {
            controller.forward(app, None)
        } else {
            TickerFuture::complete()
        }
    }

    /// Schedules a hide of the magnifier.
    ///
    /// If this [`MagnifierController`] has an [`animation_controller`](Self::animation_controller),
    /// then [`hide`](Self::hide) reverses the animation controller and waits
    /// for the animation to complete. Then, if `remove_from_overlay`
    /// is true, remove the magnifier from the overlay.
    ///
    /// In general, `remove_from_overlay` should be true, unless
    /// the magnifier needs to preserve states between shows / hides.
    ///
    /// See also:
    ///
    ///  * [`remove_from_overlay`](Self::remove_from_overlay) which removes the [`OverlayEntry`] from the [`Overlay`]
    ///    synchronously.
    pub fn hide(self: Handle<Self>, app: &mut App, remove_from_overlay: bool) -> TickerFuture {
        if app.get(self).overlay_entry.is_none() {
            return TickerFuture::complete();
        }

        if let Some(controller) = app.get(self).animation_controller {
            let future = controller.reverse(app, None);
            if remove_from_overlay {
                future.when_complete(
                    app,
                    Listener::handle_method(self, MagnifierController::remove_from_overlay),
                );
            }
            return future;
        }

        if remove_from_overlay {
            self.remove_from_overlay(app);
        }
        TickerFuture::complete()
    }

    /// Remove the [`OverlayEntry`] from the [`Overlay`].
    ///
    /// This method removes the [`OverlayEntry`] synchronously,
    /// regardless of exit animation: this leads to abrupt removals
    /// of [`OverlayEntry`]s with animations.
    ///
    /// To allow the [`OverlayEntry`] to play its exit animation, consider calling
    /// [`hide`](Self::hide) instead, with `remove_from_overlay` set to true, and optionally await
    /// the returned future.
    pub fn remove_from_overlay(self: Handle<Self>, app: &mut App) {
        if let Some(entry) = app.get(self).overlay_entry {
            entry.remove(app);
            entry.dispose(app);
        }
        app.get_mut(self).overlay_entry = None;
    }

    /// A utility for calculating a new [`Rect`] from `rect` such that
    /// `rect` is fully constrained within `bounds`.
    ///
    /// Any point in the output rect is guaranteed to also be a point contained in `bounds`.
    ///
    /// It is a runtime error for `rect`.width to be greater than `bounds`.width,
    /// and it is also an error for `rect`.height to be greater than `bounds`.height.
    ///
    /// This algorithm translates `rect` the shortest distance such that it is entirely within
    /// `bounds`.
    ///
    /// If `rect` is already within `bounds`, no shift will be applied to `rect` and
    /// `rect` will be returned as-is.
    ///
    /// It is perfectly valid for the output rect to have a point along the edge of the
    /// `bounds`. If the desired output rect requires that no edges are parallel to edges
    /// of `bounds`, see [`Rect::deflate`] by 1 on `bounds` to achieve this effect.
    pub fn shift_within_bounds(rect: Rect, bounds: Rect) -> Rect {
        debug_assert!(
            rect.width() <= bounds.width(),
            "attempted to shift {rect:?} within {bounds:?}, but the rect has a greater width.",
        );
        debug_assert!(
            rect.height() <= bounds.height(),
            "attempted to shift {rect:?} within {bounds:?}, but the rect has a greater height.",
        );

        let mut rect_shift = Offset::ZERO;
        if rect.left < bounds.left {
            rect_shift = rect_shift + Offset::new(bounds.left - rect.left, 0.0);
        } else if rect.right > bounds.right {
            rect_shift = rect_shift + Offset::new(bounds.right - rect.right, 0.0);
        }

        if rect.top < bounds.top {
            rect_shift = rect_shift + Offset::new(0.0, bounds.top - rect.top);
        } else if rect.bottom > bounds.bottom {
            rect_shift = rect_shift + Offset::new(0.0, bounds.bottom - rect.bottom);
        }

        rect.shift(rect_shift)
    }
}

/// The decorations to put around the loupe in a [`RawMagnifier`].
///
/// See also:
///
///  * `Decoration`, a more general solution for [`DecoratedBox`].
#[derive(Clone, Debug)]
pub struct MagnifierDecoration {
    // TODO(ianh): deprecate [opacity] (moving it to [RawMagnifier]), and then
    // once [opacity] can be removed, replace [MagnifierDecoration] with a
    // `typedef` to [ShapeDecoration] and make anywhere that accepts a
    // [MagnifierDecoration] accept a [ShapeDecoration] instead. This would allow
    // magnifiers that don't offset the shadows to use the decoration to paint
    // over the loupe rather than having to have a Stack of widgets to do so.
    /// The opacity of the magnifier and decorations around the magnifier.
    ///
    /// When this is 1.0, the magnified image shows in the [`shape`](Self::shape) of the
    /// magnifier. When this is less than 1.0, the magnified image is transparent
    /// and shows through the unmagnified background.
    ///
    /// Generally this is only useful for animating the magnifier in and out, as a
    /// transparent magnifier looks quite confusing.
    pub opacity: f64,
    /// A list of shadows cast by the [`shape`](Self::shape).
    ///
    /// If the shadows are offset, consider setting [`RawMagnifier::clip_behavior`] to
    /// [`Clip::HardEdge`] (or similar) to ensure the shadow does not occlude the
    /// magnifier (the shadow is drawn above the magnifier).
    ///
    /// If the shadows are _not_ offset, consider using `BlurStyle::Outer` in the
    /// shadows instead, to avoid having to introduce a clip.
    ///
    /// In the event that [`shape`](Self::shape) consists of a stack of borders, the shadow
    /// is drawn using the bounds of the last one.
    ///
    /// See also:
    ///
    ///  * `kElevationToShadow`, which defines some shadows for Material design.
    ///    Those shadows use `BlurStyle::Normal` and may need to be converted to
    ///    `BlurStyle::Outer` for use with [`MagnifierDecoration`].
    pub shadows: Option<Vec<BoxShadow>>,
    /// The shape of the magnifier and the outline (border) around it.
    ///
    /// Shapes can be stacked (using `plus`). In that case, the magnifier and shadow are
    /// drawn according to the outside edge of the last shape, with the borders painted on
    /// top.
    pub shape: Box<dyn ShapeBorder>,
}

impl MagnifierDecoration {
    /// Constructs a [`MagnifierDecoration`].
    ///
    /// By default, [`MagnifierDecoration`] is a rectangular magnifier with no
    /// shadows, and fully opaque.
    pub fn new() -> MagnifierDecoration {
        MagnifierDecoration::default()
    }

    /// Dart `MagnifierDecoration(opacity:)`.
    pub fn opacity(mut self, opacity: f64) -> MagnifierDecoration {
        self.opacity = opacity;
        self
    }

    /// Dart `MagnifierDecoration(shadows:)`.
    pub fn shadows(mut self, shadows: Vec<BoxShadow>) -> MagnifierDecoration {
        self.shadows = Some(shadows);
        self
    }

    /// Dart `MagnifierDecoration(shape:)`.
    pub fn shape(mut self, shape: impl Into<Box<dyn ShapeBorder>>) -> MagnifierDecoration {
        self.shape = shape.into();
        self
    }
}

impl Default for MagnifierDecoration {
    fn default() -> MagnifierDecoration {
        MagnifierDecoration {
            opacity: 1.0,
            shadows: None,
            shape: Box::new(RoundedRectangleBorder::default()),
        }
    }
}

impl PartialEq for MagnifierDecoration {
    fn eq(&self, other: &MagnifierDecoration) -> bool {
        self.opacity == other.opacity
            && self.shadows == other.shadows
            && self.shape.eq_shape(&*other.shape)
    }
}

/// A common base class for magnifiers.
///
/// This magnifying glass is useful for scenarios on mobile devices where
/// the user's finger may be covering part of the screen where a granular
/// action is being performed, such as navigating a small cursor with a drag
/// gesture, on an image or text.
///
/// A magnifier can be conveniently managed by [`MagnifierController`], which handles
/// showing and hiding the magnifier, with an optional entry / exit animation.
///
/// See:
/// * [`MagnifierController`], a controller to handle magnifiers in an overlay.
#[derive(Debug)]
pub struct RawMagnifier {
    pub key: Option<KeyRef>,
    /// An optional widget to position inside the len of the [`RawMagnifier`].
    ///
    /// This is positioned over the [`RawMagnifier`] - it may be useful for tinting the
    /// [`RawMagnifier`], or drawing a crosshair-like UI.
    pub child: Option<WidgetRef>,
    /// This magnifier's decoration.
    ///
    /// This sets the shape of the loupe, plus any borders and shadows that it
    /// casts. The default has no border and no shadow; combined with the default
    /// [`magnification_scale`](Self::magnification_scale) of 1.0, this results in the
    /// magnifier having no visible effect.
    ///
    /// If the [`decoration`](Self::decoration) has a [`MagnifierDecoration::shadows`] that
    /// uses offset shadows or uses a `BlurStyle` that would obscure the magnified image,
    /// consider setting [`clip_behavior`](Self::clip_behavior) to [`Clip::HardEdge`] (or
    /// similar) to ensure the magnified image is visible.
    pub decoration: MagnifierDecoration,
    /// Whether and how to clip the parts of [`decoration`](Self::decoration) that render
    /// inside the loupe.
    ///
    /// Defaults to [`Clip::None`].
    ///
    /// See the discussion at [`decoration`](Self::decoration).
    pub clip_behavior: Clip,
    /// The offset of the magnifier from [`RawMagnifier`]'s center.
    ///
    /// For example, if [`RawMagnifier`] is globally positioned at Offset(100, 100),
    /// and [`focal_point_offset`](Self::focal_point_offset) is Offset(-20, -20), then
    /// [`RawMagnifier`] will see the content at global offset (80, 80).
    ///
    /// If left as [`Offset::ZERO`], the [`RawMagnifier`] will show the content that
    /// is directly below it.
    pub focal_point_offset: Offset,
    /// How "zoomed in" the magnification subject is in the lens.
    ///
    /// The default is 1.0, which is no magnification.
    pub magnification_scale: f64,
    /// The size of the magnifier.
    ///
    /// This does not include the border from the [`decoration`](Self::decoration); it only
    /// includes the size of the magnifier.
    pub size: Size,
}

impl RawMagnifier {
    /// Constructs a [`RawMagnifier`].
    ///
    /// By default, this magnifier uses the default [`MagnifierDecoration`] (which
    /// draws nothing), the focal point is directly under the magnifier, and there
    /// is no magnification; this means that a default magnifier will be entirely
    /// invisible to the naked eye, painting exactly what is under it, exactly
    /// where it was painted originally.
    pub fn new(size: Size) -> RawMagnifier {
        RawMagnifier {
            key: None,
            child: None,
            decoration: MagnifierDecoration::new(),
            clip_behavior: Clip::None,
            focal_point_offset: Offset::ZERO,
            magnification_scale: 1.0,
            size,
        }
    }

    /// Dart `RawMagnifier(key:)`.
    pub fn key(mut self, key: KeyRef) -> RawMagnifier {
        self.key = Some(key);
        self
    }

    /// Dart `RawMagnifier(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> RawMagnifier {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `RawMagnifier(decoration:)`.
    pub fn decoration(mut self, decoration: MagnifierDecoration) -> RawMagnifier {
        self.decoration = decoration;
        self
    }

    /// Dart `RawMagnifier(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> RawMagnifier {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `RawMagnifier(focalPointOffset:)`.
    pub fn focal_point_offset(mut self, focal_point_offset: Offset) -> RawMagnifier {
        self.focal_point_offset = focal_point_offset;
        self
    }

    /// Dart `RawMagnifier(magnificationScale:)`.
    pub fn magnification_scale(mut self, magnification_scale: f64) -> RawMagnifier {
        debug_assert!(
            magnification_scale != 0.0,
            "Magnification scale of 0 results in undefined behavior."
        );
        self.magnification_scale = magnification_scale;
        self
    }
}

impl StatelessWidget for RawMagnifier {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let mut lens = SizedBox::from_size(Some(self.size));
        lens.child = self.child.clone();
        let mut decoration = ShapeDecoration::new(self.decoration.shape.clone());
        decoration.shadows = self.decoration.shadows.clone();
        Stack::new()
            .clip_behavior(Clip::None)
            .alignment(AlignmentGeometry::CENTER)
            .children([
                // The magnified image is clipped to the outer path of the shape.
                ClipPath::shape(self.decoration.shape.clone())
                    .child(
                        Opacity::new(self.decoration.opacity).child(
                            Magnifier::new()
                                .focal_point_offset(self.focal_point_offset)
                                .magnification_scale(self.magnification_scale)
                                .child(lens),
                        ),
                    )
                    .into_widget(),
                // Because `BackdropFilter` will filter any widgets before it, we apply
                // these styles after (i.e. in a younger sibling) to avoid the magnifier
                // from seeing its own styling.
                IgnorePointer::new()
                    .child(
                        Opacity::new(self.decoration.opacity).child(
                            ClipPath::new()
                                .clip_behavior(self.clip_behavior)
                                .clipper(Rc::new(NegativeClip::new(self.decoration.shape.clone())))
                                .child(
                                    DecoratedBox::new(decoration)
                                        .child(SizedBox::from_size(Some(self.size))),
                                ),
                        ),
                    )
                    .into_widget(),
            ])
            .into_widget()
    }
}

/// A clip that renders everything except the inside of a shape.
///
/// Dart's `_NegativeClip` fills `Rect.largest` and the shape's inner path with the even-odd
/// rule. A clip here is always non-zero (`waft-painting`'s `PORTING.md`), so the rectangle is
/// wound against the inner path instead: the two cancel inside the shape, which leaves the
/// same hole for an inner path of one contour.
struct NegativeClip {
    shape: Box<dyn ShapeBorder>,
}

impl NegativeClip {
    fn new(shape: Box<dyn ShapeBorder>) -> NegativeClip {
        NegativeClip { shape }
    }
}

impl CustomClipper<Arc<Path>> for NegativeClip {
    fn get_clip(&self, size: Size) -> Arc<Path> {
        let hole = self.shape.get_inner_path(Offset::ZERO & size, None);
        let mut path = PathBuilder::new();
        add_rect_wound_against(&mut path, Rect::LARGEST, &hole);
        path.append(&hole, &Matrix4::IDENTITY);
        path.build()
    }

    fn should_reclip(&self, old_clipper: &dyn CustomClipper<Arc<Path>>) -> bool {
        let Some(old_clipper) = old_clipper.as_any().downcast_ref::<NegativeClip>() else {
            return true;
        };
        !old_clipper.shape.eq_shape(&*self.shape)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Adds `rect` as a closed contour turning the other way from `hole`, so that under the
/// non-zero rule `hole` cuts out of it.
fn add_rect_wound_against(path: &mut PathBuilder, rect: Rect, hole: &Path) {
    let mut corners = [
        (rect.left as f32, rect.top as f32),
        (rect.right as f32, rect.top as f32),
        (rect.right as f32, rect.bottom as f32),
        (rect.left as f32, rect.bottom as f32),
    ];
    if winds_clockwise(hole) {
        corners.reverse();
    }
    path.move_to(corners[0]);
    for corner in &corners[1..] {
        path.line_to(*corner);
    }
    path.close();
}

/// Whether `path` turns clockwise on the y-down plane: the sign of its area by the shoelace
/// formula, over every contour.
fn winds_clockwise(path: &Path) -> bool {
    let signed_area: f32 = path
        .flatten(1.0)
        .iter()
        .map(|contour| {
            contour
                .points
                .windows(2)
                .map(|edge| edge[0].x * edge[1].y - edge[1].x * edge[0].y)
                .sum::<f32>()
        })
        .sum();
    signed_area >= 0.0
}

/// Dart's `_Magnifier`: the render object widget of the lens.
#[derive(Debug)]
struct Magnifier {
    /// The Offset that the center of the [`Magnifier`] points to, relative
    /// to the center of the magnifier.
    focal_point_offset: Offset,
    /// The enlarge multiplier of the magnification.
    ///
    /// If equal to 1.0, the content in the magnifier is true to its real size.
    /// If greater than 1.0, the content appears bigger in the magnifier.
    magnification_scale: f64,
    child: Option<WidgetRef>,
}

impl Magnifier {
    fn new() -> Magnifier {
        Magnifier {
            focal_point_offset: Offset::ZERO,
            magnification_scale: 1.0,
            child: None,
        }
    }

    fn focal_point_offset(mut self, focal_point_offset: Offset) -> Magnifier {
        self.focal_point_offset = focal_point_offset;
        self
    }

    fn magnification_scale(mut self, magnification_scale: f64) -> Magnifier {
        self.magnification_scale = magnification_scale;
        self
    }

    fn child<K>(mut self, child: impl IntoWidget<K>) -> Magnifier {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for Magnifier {
    type RenderObject = RenderMagnification;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderMagnification::new(app, self.focal_point_offset, self.magnification_scale, None)
            .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderMagnification>,
    ) {
        render_object.set_focal_point_offset(app, self.focal_point_offset);
        render_object.set_magnification_scale(app, self.magnification_scale);
    }
}

impl SingleChildRenderObjectWidget for Magnifier {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// Dart's `_RenderMagnification`: a proxy box that shows, over its child, what was painted
/// before it, moved and scaled by [`magnification_matrix`](Self::magnification_matrix).
///
/// Waiting on Valo: the backdrop needs `ImageFilter.matrix`, so [`paint`](RenderObject::paint)
/// paints the child alone and the content under the lens shows unscaled.
struct RenderMagnification {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    focal_point_offset: Offset,
    magnification_scale: f64,
}

impl RenderMagnification {
    fn new(
        app: &mut App,
        focal_point_offset: Offset,
        magnification_scale: f64,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderMagnification> {
        let this = RenderHandle::new_box(
            app,
            RenderMagnification {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                focal_point_offset,
                magnification_scale,
            },
        );
        this.set_child(app, child);
        this
    }

    fn focal_point_offset(self: RenderHandle<Self>, app: &App) -> Offset {
        self.get(app).focal_point_offset
    }

    fn set_focal_point_offset(self: RenderHandle<Self>, app: &mut App, value: Offset) {
        if self.focal_point_offset(app) == value {
            return;
        }
        self.get_mut(app).focal_point_offset = value;
        self.mark_needs_paint(app);
    }

    fn magnification_scale(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).magnification_scale
    }

    fn set_magnification_scale(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.magnification_scale(app) == value {
            return;
        }
        self.get_mut(app).magnification_scale = value;
        self.mark_needs_paint(app);
    }

    /// The transform Dart's `paint` hands `ImageFilter.matrix` for the backdrop painted at
    /// `offset`: a scale by [`magnification_scale`](Self::magnification_scale) about this
    /// box's center, which brings the focal point under the center.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "waiting on Valo: ImageFilter.matrix for a backdrop"
        )
    )]
    fn magnification_matrix(self: RenderHandle<Self>, app: &App, offset: Offset) -> Matrix4 {
        let this_center = Alignment::CENTER.along_size(self.size(app)) + offset;
        let magnification_scale = self.magnification_scale(app);
        let focal_point_offset = self.focal_point_offset(app);
        let scale = magnification_scale as f32;
        Matrix4::translation(
            (magnification_scale * (-focal_point_offset.dx() - this_center.dx()) + this_center.dx())
                as f32,
            (magnification_scale * (-focal_point_offset.dy() - this_center.dy()) + this_center.dy())
                as f32,
        )
        .then(&Matrix4::from_flutter_array(&[
            scale, 0.0, 0.0, 0.0, //
            0.0, scale, 0.0, 0.0, //
            0.0, 0.0, scale, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ]))
    }
}

impl RenderObjectWithChildMixin for RenderMagnification {
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

impl RenderProxyBoxMixin for RenderMagnification {}

impl RenderObject for RenderMagnification {
    waft_rendering::render_object_accessors!();

    fn always_needs_compositing(self: RenderHandle<Self>, _app: &App) -> bool {
        true
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        // Dart pushes a `BackdropFilterLayer` of `ImageFilter.matrix(magnification_matrix)`
        // around this call; valo has no matrix filter.
        RenderProxyBoxMixin::paint(self, app, context, offset);
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

impl RenderBox for RenderMagnification {
    waft_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use waft_animation::{AnimationBehavior, AnimationStatus};
    use waft_embedder::{BlurStyle, Color, FillRule, Radius, TextDirection};
    use waft_foundation::UniqueKey;
    use waft_painting::{BorderRadius, BorderSide, BorderStyle, CircleBorder};
    use waft_rendering::{
        RenderClipPath, RenderCustomClip, RenderDecoratedBox, RenderOpacity, RenderStack,
    };
    use waft_scheduler::{Ticker, TickerCallback, TickerProvider};

    use super::*;
    use crate::binding::WidgetsBinding;
    use crate::framework::{AnyElement, GlobalKey};
    use crate::test_harness::{binding_cell, binding_mount, binding_pump};
    use crate::widgets::basic::{Center, Directionality, Positioned};
    use crate::widgets::text::Text;

    /// flutter_test's `TestVSync`: vends ordinary tickers, in no view.
    struct TestVSync;

    impl TickerProvider for TestVSync {
        fn create_ticker(&self, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
            Ticker::new(app, on_tick, None)
        }
    }

    fn elements(app: &mut App) -> Vec<AnyElement> {
        fn visit(app: &App, element: AnyElement, found: &mut Vec<AnyElement>) {
            found.push(element);
            for child in element.children(app) {
                visit(app, child, found);
            }
        }
        let mut found = Vec::new();
        let root = WidgetsBinding::instance(app)
            .root_element(app)
            .expect("a mounted tree");
        visit(app, root, &mut found);
        found
    }

    /// The render objects of the mounted tree that are an `R`, in tree order.
    fn render_objects<R: RenderObject>(app: &mut App) -> Vec<RenderHandle<R>> {
        fn visit<R: RenderObject>(
            app: &App,
            node: AnyRenderObject,
            found: &mut Vec<RenderHandle<R>>,
        ) {
            if let Some(handle) = node.downcast::<R>(app) {
                found.push(handle);
            }
            node.visit_children(app, &mut |child| visit(app, child, found));
        }
        let root = WidgetsBinding::instance(app)
            .root_element(app)
            .and_then(|root| root.find_render_object(app))
            .expect("a laid out tree");
        let mut found = Vec::new();
        visit(app, root, &mut found);
        found
    }

    /// The render box of the first element whose widget `matches`.
    fn render_box_where(app: &mut App, matches: impl Fn(&WidgetRef) -> bool) -> AnyRenderBox {
        elements(app)
            .into_iter()
            .find(|element| matches(element.widget(app)))
            .and_then(|element| element.find_render_object(app))
            .and_then(|render_object| render_object.as_box())
            .expect("a mounted render box")
    }

    fn has_key(widget: &WidgetRef, key: &KeyRef) -> bool {
        widget.key() == Some(key)
    }

    /// Dart's `TestWidgetsApp(home: Placeholder())`: an overlay around a box whose context the
    /// magnifier is shown from.
    fn app_with_overlay(cell: &waft_foundation::AppCell, home: GlobalKey) {
        binding_mount(
            cell,
            Directionality::new(
                TextDirection::Ltr,
                Overlay::wrap(SizedBox::new().key(home.into())),
            )
            .into_widget(),
        );
    }

    fn raw_magnifier_builder(magnifier_key: KeyRef) -> WidgetBuilder {
        Rc::new(move |_app: &mut App, _context: BuildContext| {
            RawMagnifier::new(Size::new(100.0, 100.0))
                .key(magnifier_key.clone())
                .into_widget()
        })
    }

    fn at(milliseconds: u64) -> Duration {
        Duration::from_millis(milliseconds)
    }

    #[test]
    fn a_raw_magnifier_is_as_large_as_its_size() {
        let cell = binding_cell();
        binding_mount(
            &cell,
            Directionality::new(
                TextDirection::Ltr,
                Center::new().child(RawMagnifier::new(Size::new(100.0, 80.0))),
            )
            .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let magnifier = render_box_where(&mut app, |widget| widget.as_any().is::<RawMagnifier>());
        assert_eq!(magnifier.size(&app), Size::new(100.0, 80.0));
        let lens = render_objects::<RenderMagnification>(&mut app);
        assert_eq!(lens.len(), 1);
        assert_eq!(lens[0].size(&app), Size::new(100.0, 80.0));
    }

    /// What the shell does at start-up: the app-wide fonts, here the OS fonts.
    fn install_fonts(app: &mut App) {
        let binding = waft_painting::PaintingBinding::instance(app);
        if !binding.has_fonts(app) {
            binding.install_fonts(app, |fonts| {
                fonts.add_source(valo_system_fonts::SystemFonts::load());
            });
        }
    }

    #[test]
    fn a_raw_magnifier_does_not_crash_at_zero_area() {
        let cell = binding_cell();
        install_fonts(&mut cell.borrow_mut());
        binding_mount(
            &cell,
            Directionality::new(
                TextDirection::Ltr,
                Center::new().child(
                    SizedBox::shrink()
                        .child(RawMagnifier::new(Size::square(2.0)).child(Text::new("X"))),
                ),
            )
            .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let magnifier = render_box_where(&mut app, |widget| widget.as_any().is::<RawMagnifier>());
        assert_eq!(magnifier.size(&app), Size::ZERO);
    }

    #[test]
    fn the_lens_takes_the_focal_point_and_the_magnification_scale_of_the_magnifier() {
        let cell = binding_cell();
        let magnifier = |focal_point_offset: Offset, magnification_scale: f64| {
            Directionality::new(
                TextDirection::Ltr,
                Center::new().child(
                    RawMagnifier::new(Size::new(100.0, 100.0))
                        .focal_point_offset(focal_point_offset)
                        .magnification_scale(magnification_scale),
                ),
            )
            .into_widget()
        };
        binding_mount(&cell, magnifier(Offset::new(50.0, 50.0), 2.0));
        let mut app = cell.borrow_mut();
        let lens = render_objects::<RenderMagnification>(&mut app)[0];
        assert_eq!(lens.focal_point_offset(&app), Offset::new(50.0, 50.0));
        assert_eq!(lens.magnification_scale(&app), 2.0);
        assert!(lens.as_object().needs_compositing(&app));

        let view = crate::test_harness::platform_view(&app);
        WidgetsBinding::instance(&mut app).attach_root_widget(
            &mut app,
            crate::view::View::new(view, magnifier(Offset::new(-20.0, 10.0), 3.0)).into_widget(),
        );
        binding_pump(&mut app, at(16));
        assert_eq!(lens.focal_point_offset(&app), Offset::new(-20.0, 10.0));
        assert_eq!(lens.magnification_scale(&app), 3.0);
    }

    #[test]
    fn the_magnification_matrix_scales_the_focal_point_into_the_center_of_the_lens() {
        // Dart's 'should render with correct focal point and decoration' setup: a lens of
        // 100 x 100 at (200, 200) that looks 50 right and 50 down, at twice the size.
        let cell = binding_cell();
        binding_mount(
            &cell,
            Directionality::new(
                TextDirection::Ltr,
                Center::new().child(
                    RawMagnifier::new(Size::new(100.0, 100.0))
                        .focal_point_offset(Offset::new(50.0, 50.0))
                        .magnification_scale(2.0),
                ),
            )
            .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let lens = render_objects::<RenderMagnification>(&mut app)[0];
        let matrix = lens.magnification_matrix(&app, Offset::new(200.0, 200.0));

        let center = waft_embedder::valo::Point::new(250.0, 250.0);
        let focal_point = waft_embedder::valo::Point::new(300.0, 300.0);
        assert_eq!(matrix.map_point(focal_point), center);
        let one_right_of_focal_point = waft_embedder::valo::Point::new(301.0, 300.0);
        assert_eq!(
            matrix.map_point(one_right_of_focal_point),
            waft_embedder::valo::Point::new(252.0, 250.0)
        );
        assert_eq!(
            matrix.to_flutter_array(),
            [
                2.0, 0.0, 0.0, 0.0, //
                0.0, 2.0, 0.0, 0.0, //
                0.0, 0.0, 2.0, 0.0, //
                -350.0, -350.0, 0.0, 1.0,
            ]
        );
    }

    #[test]
    fn the_decoration_is_painted_over_the_lens_through_a_negative_clip() {
        let cell = binding_cell();
        let shadow = BoxShadow::new(
            Color::new(0xFFFFFF00),
            Offset::new(5.0, 5.0),
            10.0,
            10.0,
            BlurStyle::Normal,
        );
        let shape = RoundedRectangleBorder::new(
            BorderSide::new(Color::new(0xFF000000), 2.0, BorderStyle::Solid, -1.0),
            BorderRadius::all(Radius::circular(10.0)).into(),
        );
        binding_mount(
            &cell,
            Directionality::new(
                TextDirection::Ltr,
                Center::new().child(
                    RawMagnifier::new(Size::new(100.0, 100.0))
                        .clip_behavior(Clip::HardEdge)
                        .decoration(
                            MagnifierDecoration::new()
                                .shadows(vec![shadow])
                                .opacity(0.5)
                                .shape(shape),
                        ),
                ),
            )
            .into_widget(),
        );
        let mut app = cell.borrow_mut();

        let opacities = render_objects::<RenderOpacity>(&mut app);
        assert_eq!(opacities.len(), 2, "the lens and the decoration both fade");
        assert!(opacities.iter().all(|opacity| opacity.opacity(&app) == 0.5));

        let decorated = render_objects::<RenderDecoratedBox>(&mut app);
        assert_eq!(decorated.len(), 1);
        let decoration = decorated[0]
            .decoration(&app)
            .as_any()
            .downcast_ref::<ShapeDecoration>()
            .expect("a ShapeDecoration");
        assert_eq!(decoration.shadows, Some(vec![shadow]));
        assert!(decoration.shape.eq_shape(&shape));
        assert_eq!(decorated[0].size(&app), Size::new(100.0, 100.0));

        let clips = render_objects::<RenderClipPath>(&mut app);
        assert_eq!(clips.len(), 2);
        let (lens_clip, decoration_clip) = (clips[0], clips[1]);
        assert_eq!(lens_clip.clip_behavior(&app), Clip::AntiAlias);
        assert_eq!(decoration_clip.clip_behavior(&app), Clip::HardEdge);
        let negative = decoration_clip
            .clipper(&app)
            .expect("the negative clip")
            .get_clip(Size::new(100.0, 100.0));
        assert!(negative.contains((-50.0, -50.0).into(), FillRule::NonZero));
        assert!(negative.contains((150.0, 50.0).into(), FillRule::NonZero));
        assert!(!negative.contains((50.0, 50.0).into(), FillRule::NonZero));

        let stack = render_objects::<RenderStack>(&mut app);
        assert_eq!(stack[0].size(&app), Size::new(100.0, 100.0));
    }

    #[test]
    fn the_negative_clip_cuts_out_the_inside_of_a_shape_whichever_way_it_turns() {
        let size = Size::new(100.0, 60.0);
        for shape in [
            Box::new(RoundedRectangleBorder::default()) as Box<dyn ShapeBorder>,
            Box::new(RoundedRectangleBorder::new(
                BorderSide::NONE,
                BorderRadius::all(Radius::elliptical(60.0, 50.0)).into(),
            )),
            Box::new(CircleBorder::default()),
        ] {
            let clip = NegativeClip::new(shape.clone()).get_clip(size);
            let hole = shape.get_inner_path(Offset::ZERO & size, None);
            for point in [(50.0, 30.0), (40.0, 25.0), (60.0, 35.0)] {
                assert_eq!(
                    clip.contains(point.into(), FillRule::NonZero),
                    !hole.contains(point.into(), FillRule::NonZero),
                    "{shape:?} at {point:?}"
                );
            }
            for point in [(-1.0e6, -1.0e6), (1.0e6, 30.0), (50.0, 61.0)] {
                assert!(
                    clip.contains(point.into(), FillRule::NonZero),
                    "{shape:?} at {point:?}"
                );
            }
        }

        let mut counter_clockwise = PathBuilder::new();
        counter_clockwise
            .move_to((0.0, 0.0))
            .line_to((0.0, 10.0))
            .line_to((10.0, 10.0))
            .line_to((10.0, 0.0))
            .close();
        let hole = counter_clockwise.build();
        let mut path = PathBuilder::new();
        add_rect_wound_against(&mut path, Rect::LARGEST, &hole);
        path.append(&hole, &Matrix4::IDENTITY);
        let clip = path.build();
        assert!(!clip.contains((5.0, 5.0).into(), FillRule::NonZero));
        assert!(clip.contains((20.0, 5.0).into(), FillRule::NonZero));
    }

    #[test]
    fn the_negative_clip_reclips_only_for_another_shape() {
        let square = NegativeClip::new(Box::new(RoundedRectangleBorder::default()));
        let same = NegativeClip::new(Box::new(RoundedRectangleBorder::default()));
        let round = NegativeClip::new(Box::new(CircleBorder::default()));
        assert!(!same.should_reclip(&square));
        assert!(round.should_reclip(&square));
    }

    #[test]
    fn magnifier_decorations_with_the_same_fields_are_equal() {
        assert_eq!(MagnifierDecoration::new(), MagnifierDecoration::default());
        assert_eq!(MagnifierDecoration::new().opacity, 1.0);
        assert_eq!(MagnifierDecoration::new().shadows, None);
        assert_ne!(
            MagnifierDecoration::new(),
            MagnifierDecoration::new().opacity(0.5)
        );
        assert_ne!(
            MagnifierDecoration::new(),
            MagnifierDecoration::new().shape(CircleBorder::default())
        );
        assert_ne!(
            MagnifierDecoration::new(),
            MagnifierDecoration::new().shadows(Vec::new())
        );
    }

    #[test]
    fn should_immediately_remove_from_overlay_on_no_animation_controller() {
        let cell = binding_cell();
        let home = GlobalKey::new();
        app_with_overlay(&cell, home.clone());
        let mut app = cell.borrow_mut();
        let context = home.current_context(&mut app).expect("mounted");
        let magnifier_controller = MagnifierController::new(&mut app);

        magnifier_controller.show(
            &mut app,
            context,
            raw_magnifier_builder(UniqueKey::new().into()),
            None,
        );
        binding_pump(&mut app, at(0));
        assert!(magnifier_controller.overlay_entry(&app).is_some());

        magnifier_controller.hide(&mut app, true);
        binding_pump(&mut app, at(16));
        assert!(magnifier_controller.overlay_entry(&app).is_none());
    }

    #[test]
    fn should_update_shown_based_on_animation_status() {
        let cell = binding_cell();
        let home = GlobalKey::new();
        app_with_overlay(&cell, home.clone());
        let mut app = cell.borrow_mut();
        let context = home.current_context(&mut app).expect("mounted");
        let duration = Duration::from_secs(120);
        let animation_controller = AnimationController::create(
            &mut app,
            None,
            Some(duration),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            TestVSync,
        );
        let magnifier_controller =
            MagnifierController::new(&mut app).animation_controller(&mut app, animation_controller);

        magnifier_controller.show(
            &mut app,
            context,
            raw_magnifier_builder(UniqueKey::new().into()),
            None,
        );
        binding_pump(&mut app, at(0));

        // No time has passed, so the animation controller has not completed.
        assert_eq!(animation_controller.status(&app), AnimationStatus::Forward);
        assert!(magnifier_controller.shown(&app));

        binding_pump(&mut app, duration);
        binding_pump(&mut app, duration + at(16));
        assert_eq!(
            animation_controller.status(&app),
            AnimationStatus::Completed
        );
        assert!(magnifier_controller.shown(&app));

        magnifier_controller.hide(&mut app, true);
        binding_pump(&mut app, duration + at(32));
        assert_eq!(animation_controller.status(&app), AnimationStatus::Reverse);
        assert!(!magnifier_controller.shown(&app));

        binding_pump(&mut app, duration * 2 + at(32));
        binding_pump(&mut app, duration * 2 + at(48));
        assert_eq!(
            animation_controller.status(&app),
            AnimationStatus::Dismissed
        );
        assert!(!magnifier_controller.shown(&app));
    }

    #[test]
    fn should_insert_below_below_widget() {
        let cell = binding_cell();
        let home = GlobalKey::new();
        app_with_overlay(&cell, home.clone());
        let mut app = cell.borrow_mut();
        let context = home.current_context(&mut app).expect("mounted");
        let magnifier_controller = MagnifierController::new(&mut app);
        let fake_magnifier: KeyRef = UniqueKey::new().into();
        let fake_before: KeyRef = UniqueKey::new().into();

        let before_key = fake_before.clone();
        let fake_before_overlay_entry = OverlayEntry::new(
            &mut app,
            Rc::new(move |_app: &mut App, _context: BuildContext| {
                SizedBox::new().key(before_key.clone()).into_widget()
            }),
            false,
            false,
            false,
        );
        Overlay::of(&mut app, context, false).insert(
            &mut app,
            fake_before_overlay_entry,
            None,
            None,
        );
        let magnifier_key = fake_magnifier.clone();
        magnifier_controller.show(
            &mut app,
            context,
            Rc::new(move |_app: &mut App, _context: BuildContext| {
                SizedBox::new().key(magnifier_key.clone()).into_widget()
            }),
            Some(fake_before_overlay_entry),
        );
        binding_pump(&mut app, at(0));

        let keyed: Vec<KeyRef> = elements(&mut app)
            .into_iter()
            .filter_map(|element| element.widget(&app).key().cloned())
            .filter(|key| key == &fake_magnifier || key == &fake_before)
            .collect();
        // Expect the magnifier to be the first child, even though it was inserted
        // after the fakeBefore.
        assert_eq!(keyed, vec![fake_magnifier, fake_before]);

        magnifier_controller.remove_from_overlay(&mut app);
        fake_before_overlay_entry.remove(&mut app);
        fake_before_overlay_entry.dispose(&mut app);
    }

    #[test]
    fn should_insert_newly_built_widget_without_animating_out_if_overlay_is_not_none() {
        let cell = binding_cell();
        let home = GlobalKey::new();
        app_with_overlay(&cell, home.clone());
        let mut app = cell.borrow_mut();
        let context = home.current_context(&mut app).expect("mounted");
        let magnifier_controller = MagnifierController::new(&mut app);
        let first: KeyRef = UniqueKey::new().into();
        let second: KeyRef = UniqueKey::new().into();

        magnifier_controller.show(
            &mut app,
            context,
            raw_magnifier_builder(first.clone()),
            None,
        );
        binding_pump(&mut app, at(0));
        binding_pump(&mut app, at(60_000));

        magnifier_controller.show(
            &mut app,
            context,
            raw_magnifier_builder(second.clone()),
            None,
        );
        binding_pump(&mut app, at(60_016));

        let count = |app: &mut App, key: &KeyRef| {
            elements(app)
                .into_iter()
                .filter(|element| has_key(element.widget(app), key))
                .count()
        };
        assert_eq!(count(&mut app, &second), 1);
        assert_eq!(count(&mut app, &first), 0);
        magnifier_controller.remove_from_overlay(&mut app);
    }

    #[test]
    fn shift_within_bounds_moves_a_rect_the_shortest_way_into_the_bounds() {
        let bounds = Rect::from_ltrb(0.0, 0.0, 100.0, 100.0);
        for (input, output) in [
            (
                Rect::from_ltrb(-100.0, -100.0, -80.0, -80.0),
                Rect::from_ltrb(0.0, 0.0, 20.0, 20.0),
            ),
            (
                Rect::from_ltrb(0.0, 0.0, 20.0, 20.0),
                Rect::from_ltrb(0.0, 0.0, 20.0, 20.0),
            ),
            (
                Rect::from_ltrb(110.0, 0.0, 120.0, 10.0),
                Rect::from_ltrb(90.0, 0.0, 100.0, 10.0),
            ),
            (
                Rect::from_ltrb(110.0, 110.0, 120.0, 120.0),
                Rect::from_ltrb(90.0, 90.0, 100.0, 100.0),
            ),
        ] {
            assert_eq!(
                MagnifierController::shift_within_bounds(input, bounds),
                output,
                "should shift {input:?} to {output:?} for bounds {bounds:?}"
            );
        }
    }

    #[test]
    fn raw_magnifier_sits_where_it_is_positioned() {
        let cell = binding_cell();
        let key: KeyRef = UniqueKey::new().into();
        binding_mount(
            &cell,
            Directionality::new(
                TextDirection::Ltr,
                Stack::new().children([Positioned::new(
                    RawMagnifier::new(Size::new(40.0, 40.0)).key(key.clone()),
                )
                .left(20.0)
                .top(30.0)
                .into_widget()]),
            )
            .into_widget(),
        );
        let mut app = cell.borrow_mut();
        let magnifier = render_box_where(&mut app, |widget| has_key(widget, &key));
        assert_eq!(
            magnifier.local_to_global(&app, Offset::ZERO, None),
            Offset::new(20.0, 30.0)
        );
    }
}
