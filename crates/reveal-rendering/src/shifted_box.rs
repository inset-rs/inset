//! Flutter counterpart: `rendering/shifted_box.dart` (`RenderShiftedBox`,
//! `RenderPadding`, `RenderAligningShiftedBox`, `RenderPositionedBox`).
//!
//! Intrinsics wait.

use reveal_embedder::{Offset, Size};
use reveal_foundation::App;
use reveal_painting::{
    Alignment, AlignmentGeometry, EdgeInsets, EdgeInsetsGeometry, TextDirection,
};

use crate::box_::{
    AnyRenderBox, BoxHitTestResult, BoxParentData, RenderBox, RenderBoxData,
    RenderObjectWithChildMixin,
};
use crate::object::{
    AnyRenderObject, RenderHandle, RenderObject, RenderObjectData, RenderObjectWithChildData,
};
use crate::painting_context::PaintingContext;

/// Abstract class for one-child-layout render boxes that provide control over
/// the child's position.
///
/// Flutter's `RenderShiftedBox`: the shared bodies. A leaf implements the marker and calls
/// these where Dart would run the inherited method: `RenderShiftedBox::paint(self, …)`.
pub trait RenderShiftedBox: RenderObjectWithChildMixin {
    /// Hit tests the child at its parent-data offset.
    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let Some(child) = self.child(app) else {
            return false;
        };
        let child_offset = child
            .as_object()
            .parent_data_of::<BoxParentData>(app)
            .offset;
        result.add_with_paint_offset(Some(child_offset), position, |result, transformed| {
            debug_assert_eq!(transformed, position - child_offset);
            child.hit_test(app, result, transformed)
        })
    }

    /// Paints the child at its parent-data offset.
    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if let Some(child) = self.child(app) {
            let child_offset = child
                .as_object()
                .parent_data_of::<BoxParentData>(app)
                .offset;
            context.paint_child(app, child.as_object(), child_offset + offset);
        }
    }
}

/// Insets its child by the given padding.
pub struct RenderPadding {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    resolved_padding_cache: Option<EdgeInsets>,
    padding: EdgeInsetsGeometry,
    text_direction: Option<TextDirection>,
}

impl RenderPadding {
    /// Creates a render object that insets its child.
    ///
    /// `padding` must have non-negative insets.
    pub fn new(
        app: &mut App,
        padding: EdgeInsetsGeometry,
        text_direction: Option<TextDirection>,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderPadding> {
        debug_assert!(padding.is_non_negative());
        let this = RenderHandle::new_box(
            app,
            RenderPadding {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                resolved_padding_cache: None,
                padding,
                text_direction,
            },
        );
        this.set_child(app, child);
        this
    }

    /// The amount to pad the child in each dimension.
    pub fn padding(self: RenderHandle<Self>, app: &App) -> EdgeInsetsGeometry {
        self.get(app).padding
    }

    /// Sets [`padding`](Self::padding).
    pub fn set_padding(self: RenderHandle<Self>, app: &mut App, value: EdgeInsetsGeometry) {
        debug_assert!(value.is_non_negative());
        if self.get(app).padding == value {
            return;
        }
        self.get_mut(app).padding = value;
        self.mark_need_resolution(app);
    }

    /// The text direction with which to resolve [`padding`](Self::padding).
    pub fn text_direction(self: RenderHandle<Self>, app: &App) -> Option<TextDirection> {
        self.get(app).text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    pub fn set_text_direction(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<TextDirection>,
    ) {
        if self.get(app).text_direction == value {
            return;
        }
        self.get_mut(app).text_direction = value;
        self.mark_need_resolution(app);
    }

    fn resolved_padding(&mut self) -> EdgeInsets {
        if let Some(cached) = self.resolved_padding_cache {
            return cached;
        }
        let resolved = self.padding.resolve(self.text_direction);
        debug_assert!(resolved.is_non_negative());
        self.resolved_padding_cache = Some(resolved);
        resolved
    }

    fn mark_need_resolution(self: RenderHandle<Self>, app: &mut App) {
        self.get_mut(app).resolved_padding_cache = None;
        self.mark_needs_layout(app);
    }
}

impl RenderObjectWithChildMixin for RenderPadding {
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

impl RenderShiftedBox for RenderPadding {}

impl RenderObject for RenderPadding {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let padding = self.get_mut(app).resolved_padding();
        let child = self.child(app);
        if child.is_none() {
            self.set_size(
                app,
                constraints.constrain(Size::new(padding.horizontal(), padding.vertical())),
            );
            return;
        }
        let child = child.expect("checked");
        let inner_constraints = constraints.deflate(EdgeInsetsGeometry::Insets(padding));
        child.layout(app, inner_constraints, true);
        child.parent_data_of_mut::<BoxParentData>(app).offset =
            Offset::new(padding.left, padding.top);
        self.set_size(
            app,
            constraints.constrain(Size::new(
                padding.horizontal() + child.size(app).width(),
                padding.vertical() + child.size(app).height(),
            )),
        );
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

impl RenderBox for RenderPadding {
    crate::render_box_accessors!();

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderShiftedBox::hit_test_children(self, app, result, position)
    }
}

/// The mixin fields of [`RenderAligningShiftedBox`].
pub struct RenderAligningShiftedBoxData {
    resolved_alignment: Option<Alignment>,
    alignment: AlignmentGeometry,
    text_direction: Option<TextDirection>,
}

impl RenderAligningShiftedBoxData {
    /// Flutter's constructor arguments; `alignment` defaults to center.
    pub const fn new(
        alignment: AlignmentGeometry,
        text_direction: Option<TextDirection>,
    ) -> RenderAligningShiftedBoxData {
        RenderAligningShiftedBoxData {
            resolved_alignment: None,
            alignment,
            text_direction,
        }
    }
}

/// Abstract class for one-child-layout render boxes that use a
/// [`AlignmentGeometry`] to align their children.
pub trait RenderAligningShiftedBox: RenderShiftedBox {
    /// Mixin field access.
    fn aligning_data(self: RenderHandle<Self>, app: &App) -> &RenderAligningShiftedBoxData;

    /// See [`aligning_data`](Self::aligning_data).
    fn aligning_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderAligningShiftedBoxData;

    /// The [`alignment`](Self::alignment) resolved against [`text_direction`](Self::text_direction).
    fn resolved_alignment(self: RenderHandle<Self>, app: &mut App) -> Alignment {
        let data = self.aligning_data(app);
        if let Some(resolved) = data.resolved_alignment {
            return resolved;
        }
        let resolved = data.alignment.resolve(data.text_direction);
        self.aligning_data_mut(app).resolved_alignment = Some(resolved);
        resolved
    }

    fn mark_need_resolution(self: RenderHandle<Self>, app: &mut App) {
        self.aligning_data_mut(app).resolved_alignment = None;
        self.mark_needs_layout(app);
    }

    /// How to align the child.
    ///
    /// The x and y values of the alignment control the horizontal and vertical
    /// alignment, respectively. An x value of -1.0 means that the left edge of
    /// the child is aligned with the left edge of the parent whereas an x value
    /// of 1.0 means that the right edge of the child is aligned with the right
    /// edge of the parent. Other values interpolate (and extrapolate) linearly.
    fn alignment(self: RenderHandle<Self>, app: &App) -> AlignmentGeometry {
        self.aligning_data(app).alignment
    }

    /// Sets [`alignment`](Self::alignment).
    fn set_alignment(self: RenderHandle<Self>, app: &mut App, value: AlignmentGeometry) {
        if self.aligning_data(app).alignment == value {
            return;
        }
        self.aligning_data_mut(app).alignment = value;
        self.mark_need_resolution(app);
    }

    /// The text direction with which to resolve [`alignment`](Self::alignment).
    fn text_direction(self: RenderHandle<Self>, app: &App) -> Option<TextDirection> {
        self.aligning_data(app).text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    fn set_text_direction(self: RenderHandle<Self>, app: &mut App, value: Option<TextDirection>) {
        if self.aligning_data(app).text_direction == value {
            return;
        }
        self.aligning_data_mut(app).text_direction = value;
        self.mark_need_resolution(app);
    }

    /// Apply the current [`alignment`](Self::alignment) to the child.
    ///
    /// Subclasses should call this method if they have a child, to have this
    /// class perform the actual alignment. If they don't have a child, they
    /// should not call this method.
    fn align_child(self: RenderHandle<Self>, app: &mut App) {
        let child = self.child(app).expect("align_child requires a child");
        debug_assert!(!child.as_object().debug_needs_layout(app));
        let offset = self
            .resolved_alignment(app)
            .along_offset(self.size(app) - child.size(app));
        child
            .as_object()
            .parent_data_of_mut::<BoxParentData>(app)
            .offset = offset;
    }
}

/// Positions its child using an [`AlignmentGeometry`].
///
/// For example, to align a box at the bottom right, you would pass this box a
/// tight constraint that is bigger than the child's natural size, with an
/// alignment of `Alignment::BOTTOM_RIGHT`.
///
/// By default, sizes to be as big as possible in both axes. If either axis is
/// unconstrained, then in that direction it will be sized to fit the child's
/// dimensions. Using `width_factor` and `height_factor` you can force this
/// latter behavior in all cases.
pub struct RenderPositionedBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    aligning: RenderAligningShiftedBoxData,
    width_factor: Option<f64>,
    height_factor: Option<f64>,
}

impl RenderPositionedBox {
    /// Creates a render object that positions its child.
    ///
    /// `width_factor` and `height_factor` must be non-negative when given.
    pub fn new(
        app: &mut App,
        alignment: AlignmentGeometry,
        text_direction: Option<TextDirection>,
        width_factor: Option<f64>,
        height_factor: Option<f64>,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<Self> {
        debug_assert!(width_factor.is_none_or(|factor| factor >= 0.0));
        debug_assert!(height_factor.is_none_or(|factor| factor >= 0.0));
        let this = RenderHandle::new_box(
            app,
            RenderPositionedBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                aligning: RenderAligningShiftedBoxData::new(alignment, text_direction),
                width_factor,
                height_factor,
            },
        );
        this.set_child(app, child);
        this
    }

    /// If non-null, sets its width to the child's width multiplied by this factor.
    ///
    /// Can be both greater and less than 1.0 but must be positive.
    pub fn width_factor(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        self.get(app).width_factor
    }

    /// Sets [`width_factor`](Self::width_factor).
    pub fn set_width_factor(self: RenderHandle<Self>, app: &mut App, value: Option<f64>) {
        debug_assert!(value.is_none_or(|factor| factor >= 0.0));
        if self.get(app).width_factor == value {
            return;
        }
        self.get_mut(app).width_factor = value;
        self.mark_needs_layout(app);
    }

    /// If non-null, sets its height to the child's height multiplied by this factor.
    ///
    /// Can be both greater and less than 1.0 but must be positive.
    pub fn height_factor(self: RenderHandle<Self>, app: &App) -> Option<f64> {
        self.get(app).height_factor
    }

    /// Sets [`height_factor`](Self::height_factor).
    pub fn set_height_factor(self: RenderHandle<Self>, app: &mut App, value: Option<f64>) {
        debug_assert!(value.is_none_or(|factor| factor >= 0.0));
        if self.get(app).height_factor == value {
            return;
        }
        self.get_mut(app).height_factor = value;
        self.mark_needs_layout(app);
    }
}

impl RenderObjectWithChildMixin for RenderPositionedBox {
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

impl RenderShiftedBox for RenderPositionedBox {}

impl RenderAligningShiftedBox for RenderPositionedBox {
    fn aligning_data(self: RenderHandle<Self>, app: &App) -> &RenderAligningShiftedBoxData {
        &self.get(app).aligning
    }

    fn aligning_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderAligningShiftedBoxData {
        &mut self.get_mut(app).aligning
    }
}

impl RenderObject for RenderPositionedBox {
    crate::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let (width_factor, height_factor) = {
            let this = self.get(app);
            (this.width_factor, this.height_factor)
        };
        let shrink_wrap_width = width_factor.is_some() || constraints.max_width == f64::INFINITY;
        let shrink_wrap_height = height_factor.is_some() || constraints.max_height == f64::INFINITY;
        if let Some(child) = self.child(app) {
            child.layout(app, constraints.loosen(), true);
            let child_size = child.size(app);
            self.set_size(
                app,
                constraints.constrain(Size::new(
                    if shrink_wrap_width {
                        child_size.width() * width_factor.unwrap_or(1.0)
                    } else {
                        f64::INFINITY
                    },
                    if shrink_wrap_height {
                        child_size.height() * height_factor.unwrap_or(1.0)
                    } else {
                        f64::INFINITY
                    },
                )),
            );
            self.align_child(app);
        } else {
            self.set_size(
                app,
                constraints.constrain(Size::new(
                    if shrink_wrap_width {
                        0.0
                    } else {
                        f64::INFINITY
                    },
                    if shrink_wrap_height {
                        0.0
                    } else {
                        f64::INFINITY
                    },
                )),
            );
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

impl RenderBox for RenderPositionedBox {
    crate::render_box_accessors!();

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderShiftedBox::hit_test_children(self, app, result, position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reveal_embedder::Size;

    use crate::box_::BoxConstraints;
    use crate::proxy_box::RenderConstrainedBox;

    #[test]
    fn padding_around_tight_child() {
        let mut app = App::new();
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(80.0, 40.0)), None);
        let padding = RenderPadding::new(
            &mut app,
            EdgeInsetsGeometry::all(8.0),
            None,
            Some(child.as_box()),
        );
        padding.layout(&mut app, BoxConstraints::new(), false);
        assert_eq!(child.size(&app), Size::new(80.0, 40.0));
        assert_eq!(padding.size(&app), Size::new(96.0, 56.0));
        assert_eq!(
            child.as_box().box_parent_data(&app).offset,
            Offset::new(8.0, 8.0)
        );
    }

    #[test]
    fn positioned_box_centers_its_child() {
        let mut app = App::new();
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(20.0, 20.0)), None);
        let positioned = RenderPositionedBox::new(
            &mut app,
            AlignmentGeometry::CENTER,
            None,
            None,
            None,
            Some(child.as_box()),
        );
        positioned.layout(
            &mut app,
            BoxConstraints::tight(Size::new(100.0, 100.0)),
            false,
        );
        assert_eq!(positioned.size(&app), Size::new(100.0, 100.0));
        assert_eq!(
            child.as_box().box_parent_data(&app).offset,
            Offset::new(40.0, 40.0)
        );
    }

    #[test]
    fn positioned_box_shrink_wraps_with_a_factor() {
        let mut app = App::new();
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(20.0, 20.0)), None);
        let positioned = RenderPositionedBox::new(
            &mut app,
            AlignmentGeometry::CENTER,
            None,
            Some(2.0),
            None,
            Some(child.as_box()),
        );
        positioned.layout(
            &mut app,
            BoxConstraints::new().max_width(100.0).max_height(100.0),
            false,
        );
        assert_eq!(positioned.size(&app), Size::new(40.0, 100.0));
        assert_eq!(
            child.as_box().box_parent_data(&app).offset,
            Offset::new(10.0, 40.0)
        );
    }

    #[test]
    fn padding_without_child() {
        let mut app = App::new();
        let padding = RenderPadding::new(&mut app, EdgeInsetsGeometry::all(10.0), None, None);
        padding.layout(
            &mut app,
            BoxConstraints::tight(Size::new(100.0, 100.0)),
            false,
        );
        assert_eq!(padding.size(&app), Size::new(100.0, 100.0));
    }
}
