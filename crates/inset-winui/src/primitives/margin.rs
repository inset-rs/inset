//! Signed FrameworkElement margin from framework.cpp MeasureCore and ArrangeCore.

use inset_embedder::{Offset, Size, TextBaseline};
use inset_foundation::App;
use inset_painting::EdgeInsetsGeometry;
use inset_rendering::*;
use inset_widgets::*;

/// Reserves signed outside space while leaving the child's paint unclipped.
///
/// Unlike native Padding, negative sides enlarge the child's available layout space.
/// Width and height returned to the parent never become negative.
#[derive(Debug)]
pub struct Margin {
    /// Identifies this layout wrapper across rebuilds.
    pub key: Option<KeyRef>,

    /// Left, top, right and bottom logical pixels; each must be finite.
    pub margin: [f64; 4],

    /// The element arranged inside the signed margin.
    pub child: WidgetRef,
}

impl Margin {
    /// Wraps a child in signed FrameworkElement margin.
    pub fn new<K>(margin: [f64; 4], child: impl IntoWidget<K>) -> Self {
        assert!(
            margin.iter().all(|v| v.is_finite()),
            "Margin sides must be finite"
        );
        Self {
            key: None,
            margin,
            child: child.into_widget(),
        }
    }

    /// Sets the wrapper identity.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }
}

impl RenderObjectWidget for Margin {
    type RenderObject = RenderMargin;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _: BuildContext) -> AnyRenderObject {
        RenderHandle::new_box(
            app,
            RenderMargin {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                margin: self.margin,
            },
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _: BuildContext,
        object: RenderHandle<RenderMargin>,
    ) {
        if object.get(app).margin != self.margin {
            object.get_mut(app).margin = self.margin;
            object.as_object().mark_needs_layout(app);
        }
    }
}

impl SingleChildRenderObjectWidget for Margin {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

/// Native shifted-child layout with signed outside insets.
pub struct RenderMargin {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    margin: [f64; 4],
}

impl RenderMargin {
    /// Source MeasureCore subtracts total margins before measuring the element.
    fn inner(self: RenderHandle<Self>, app: &App, constraints: BoxConstraints) -> BoxConstraints {
        let [l, t, r, b] = self.get(app).margin;
        constraints.deflate(EdgeInsetsGeometry::only(l, t, r, b))
    }

    /// Source MeasureCore adds signed margins, then clamps the reported desired size.
    fn outer(
        self: RenderHandle<Self>,
        app: &App,
        constraints: BoxConstraints,
        child: Size,
    ) -> Size {
        let [l, t, r, b] = self.get(app).margin;
        constraints.constrain(Size::new(
            (child.width() + l + r).max(0.0),
            (child.height() + t + b).max(0.0),
        ))
    }
}

impl RenderObjectWithChildMixin for RenderMargin {
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

impl RenderShiftedBox for RenderMargin {}

impl RenderObject for RenderMargin {
    inset_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let measured = if let Some(child) = self.child(app) {
            child.layout(app, self.inner(app, constraints), true);
            let [l, t, _, _] = self.get(app).margin;
            child.parent_data_of_mut::<BoxParentData>(app).offset = Offset::new(l, t);
            child.size(app)
        } else {
            Size::ZERO
        };
        self.set_size(app, self.outer(app, constraints, measured));
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

impl RenderBox for RenderMargin {
    inset_rendering::render_box_accessors!();

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderShiftedBox::hit_test_children(self, app, result, position)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let size = self
            .child(app)
            .map(|c| c.get_dry_layout(app, self.inner(app, constraints)))
            .unwrap_or(Size::ZERO);
        self.outer(app, constraints, size)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderShiftedBox::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        self.child(app)?
            .get_dry_baseline(app, self.inner(app, constraints), baseline)
            .map(|b| b + self.get(app).margin[1])
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let [l, t, r, b] = self.get(app).margin;
        (self
            .child(app)
            .map(|c| c.get_min_intrinsic_width(app, (height - t - b).max(0.0)))
            .unwrap_or(0.0)
            + l
            + r)
            .max(0.0)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let [l, t, r, b] = self.get(app).margin;
        (self
            .child(app)
            .map(|c| c.get_max_intrinsic_width(app, (height - t - b).max(0.0)))
            .unwrap_or(0.0)
            + l
            + r)
            .max(0.0)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let [l, t, r, b] = self.get(app).margin;
        (self
            .child(app)
            .map(|c| c.get_min_intrinsic_height(app, (width - l - r).max(0.0)))
            .unwrap_or(0.0)
            + t
            + b)
            .max(0.0)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let [l, t, r, b] = self.get(app).margin;
        (self
            .child(app)
            .map(|c| c.get_max_intrinsic_height(app, (width - l - r).max(0.0)))
            .unwrap_or(0.0)
            + t
            + b)
            .max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inset_foundation::AppCell;

    #[test]
    fn signed_margin_matches_dry_intrinsic_and_actual_layout() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let child =
            RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(40.0, 20.0)), None)
                .as_box();
        let margin = RenderHandle::new_box(
            &mut app,
            RenderMargin {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                margin: [-4.0, 2.0, -10.0, 3.0],
            },
        );
        margin.set_child(&mut app, Some(child));
        let constraints = BoxConstraints::new().max_width(100.0).max_height(100.0);
        assert_eq!(
            margin.as_box().get_dry_layout(&mut app, constraints),
            Size::new(26.0, 25.0)
        );
        assert_eq!(
            margin.as_box().get_min_intrinsic_width(&mut app, 100.0),
            26.0
        );
        assert_eq!(
            margin.as_box().get_max_intrinsic_height(&mut app, 100.0),
            25.0
        );
        margin.as_box().layout(&mut app, constraints, false);
        assert_eq!(margin.size(&app), Size::new(26.0, 25.0));
        assert_eq!(
            child.parent_data_of_mut::<BoxParentData>(&mut app).offset,
            Offset::new(-4.0, 2.0)
        );
    }
}
