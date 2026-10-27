//! The background, border and corner radius every template's root `ContentPresenter`, `Border` or `Grid` carries: XAML `Background`, `BorderBrush`, `BorderThickness`, `CornerRadius`, `BackgroundSizing` and `Padding`.

use crate::Brush;
use inset_embedder::{Canvas, Clip, Offset, RRect, Radius, Size, TextBaseline};
use inset_foundation::App;
use inset_painting::{BorderRadius, ClipContext, EdgeInsets, EdgeInsetsGeometry};
use inset_rendering::*;
use inset_widgets::*;

/// XAML `BackgroundSizing`: whether the background stops at the border's inner edge or extends under it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BackgroundSizing {
    /// Paints the background inside the border's inner edge.
    #[default]
    InnerBorderEdge,
    /// Paints the background beneath the border up to its outer edge.
    OuterBorderEdge,
}

/// A rounded rectangle with a background brush and a border brush, the child inside the border and padding.
#[derive(Clone, Debug)]
pub struct ControlBorder {
    /// The content placed inside the border and padding.
    pub child: Option<WidgetRef>,
    /// The brush filling the area selected by `background_sizing`.
    pub background: Option<Brush>,
    /// The brush filling the ring between the inner and outer border outlines.
    pub border_brush: Option<Brush>,
    /// Left, top, right, bottom.
    pub border_thickness: [f64; 4],
    /// Top-left, top-right, bottom-right, bottom-left.
    pub corner_radius: [f64; 4],
    /// Whether the background extends beneath the border.
    pub background_sizing: BackgroundSizing,
    /// Left, top, right, bottom, inside the border.
    pub padding: [f64; 4],
}

impl ControlBorder {
    /// Creates border chrome with the supplied brushes and a one-pixel border.
    pub fn new(background: Brush, border_brush: Brush) -> ControlBorder {
        Self::new_optional(Some(background), Some(border_brush))
    }

    /// Preserves XAML's distinction between an absent brush and a transparent brush.
    pub fn new_optional(background: Option<Brush>, border_brush: Option<Brush>) -> Self {
        ControlBorder {
            child: None,
            background,
            border_brush,
            border_thickness: [1.0; 4],
            corner_radius: [0.0; 4],
            background_sizing: BackgroundSizing::InnerBorderEdge,
            padding: [0.0; 4],
        }
    }

    /// Sets the content inside the border and padding.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> ControlBorder {
        self.child = Some(child.into_widget());
        self
    }

    /// Sets a uniform border width on all four sides.
    pub fn border_thickness(mut self, thickness: f64) -> ControlBorder {
        self.border_thickness = [thickness; 4];
        self
    }

    /// XAML `BorderThickness`, in left, top, right, bottom order.
    pub fn border_thickness_ltrb(mut self, thickness: [f64; 4]) -> ControlBorder {
        self.border_thickness = thickness;
        self
    }

    /// Sets a uniform radius on all four corners.
    pub fn corner_radius(mut self, radius: f64) -> ControlBorder {
        self.corner_radius = [radius; 4];
        self
    }

    /// XAML `CornerRadius`, in top-left, top-right, bottom-right, bottom-left order.
    pub fn corner_radius_corners(mut self, radii: [f64; 4]) -> ControlBorder {
        self.corner_radius = radii;
        self
    }

    /// Sets whether the background fills beneath the border.
    pub fn background_sizing(mut self, sizing: BackgroundSizing) -> ControlBorder {
        self.background_sizing = sizing;
        self
    }

    /// Sets additional insets inside the border in left, top, right, bottom order.
    pub fn padding(mut self, padding: [f64; 4]) -> ControlBorder {
        self.padding = padding;
        self
    }
}

impl StatelessWidget for ControlBorder {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let [left, top, right, bottom] = self.padding;
        let thickness = self.border_thickness;
        let style = ControlBorderStyle {
            background: self.background,
            border_brush: self.border_brush,
            border_thickness: thickness,
            corner_radius: self.corner_radius,
            background_sizing: self.background_sizing,
        };
        let inner = Padding::new(EdgeInsetsGeometry::from_ltrb(
            left + thickness[0],
            top + thickness[1],
            right + thickness[2],
            bottom + thickness[3],
        ));
        let inner = match &self.child {
            Some(child) => inner.child(child.clone()),
            None => inner,
        };
        let [top_left, top_right, bottom_right, bottom_left] =
            self.corner_radius.map(Radius::circular);
        let rounded = self.corner_radius.iter().any(|radius| *radius > 0.0);
        ClipRRect::new()
            .border_radius(
                BorderRadius::only(top_left, top_right, bottom_left, bottom_right).into(),
            )
            .clip_behavior(if rounded { Clip::AntiAlias } else { Clip::None })
            .child(BorderPaint {
                style,
                child: inner.into_widget(),
            })
            .into_widget()
    }
}

/// Source brushes and geometry used for both painting and hit testing.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ControlBorderStyle {
    /// An absent background leaves the interior out of this element's hit area.
    background: Option<Brush>,

    /// An absent border leaves the stroke out of this element's hit area.
    border_brush: Option<Brush>,

    /// Width of the four border edges.
    border_thickness: [f64; 4],

    /// Physical corner radii in clockwise order.
    corner_radius: [f64; 4],

    /// Chooses whether the fill extends under the border.
    background_sizing: BackgroundSizing,
}

impl ControlBorderStyle {
    /// Uses the same rounded outlines for drawing and pointer hit testing.
    fn outlines(self, size: Size) -> (RRect, RRect) {
        let bounds = Offset::ZERO & size;
        let [top_left, top_right, bottom_right, bottom_left] =
            self.corner_radius.map(Radius::circular);
        let outer =
            RRect::from_rect_and_corners(bounds, top_left, top_right, bottom_right, bottom_left)
                .scale_radii();
        let [left, top, right, bottom] = self.border_thickness;
        let inner = EdgeInsets::from_ltrb(left, top, right, bottom).deflate_rrect(outer);
        (outer, inner)
    }

    /// CBorder::HitTestLocalInternalImpl for the brush shapes supported by this kit.
    fn hit_test(self, size: Size, point: Offset) -> bool {
        if size.width() <= 0.0 || size.height() <= 0.0 {
            return false;
        }
        let (outer, inner) = self.outlines(size);
        let in_outer = outer.contains(point);
        let in_inner = inner.contains(point);
        let background_hit = self.background.is_some()
            && match self.background_sizing {
                BackgroundSizing::InnerBorderEdge => in_inner,
                BackgroundSizing::OuterBorderEdge => in_outer,
            };
        background_hit || (self.border_brush.is_some() && in_outer && !in_inner)
    }

    /// Paints the fill followed by the stroke, before the child.
    fn paint(self, canvas: &mut Canvas, size: Size) {
        let bounds = Offset::ZERO & size;
        let (outer, inner) = self.outlines(size);
        let background_rect = match self.background_sizing {
            BackgroundSizing::InnerBorderEdge => inner,
            BackgroundSizing::OuterBorderEdge => outer,
        };
        if let Some(background) = self.background
            && (matches!(background, Brush::Acrylic(_))
                || background.representative_color().a > 0.0)
        {
            background.paint_rrect(canvas, background_rect, bounds);
        }
        if let Some(border) = self.border_brush
            && self
                .border_thickness
                .iter()
                .any(|thickness| *thickness > 0.0)
            && (matches!(border, Brush::Acrylic(_)) || border.representative_color().a > 0.0)
        {
            border.paint_drrect(canvas, outer, inner, bounds);
        }
    }
}

/// A render-object widget keeps hit testing tied to layout size rather than the last paint.
#[derive(Debug)]
struct BorderPaint {
    /// Current source geometry and brushes.
    style: ControlBorderStyle,

    /// Child with border thickness and padding already applied.
    child: WidgetRef,
}

impl RenderObjectWidget for BorderPaint {
    type RenderObject = RenderBorderPaint;

    fn create_render_object(&self, app: &mut App, _: BuildContext) -> AnyRenderObject {
        RenderHandle::new_box(
            app,
            RenderBorderPaint {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                style: self.style,
            },
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _: BuildContext,
        render: RenderHandle<RenderBorderPaint>,
    ) {
        if render.get(app).style != self.style {
            render.get_mut(app).style = self.style;
            render.as_object().mark_needs_paint(app);
        }
    }
}

impl SingleChildRenderObjectWidget for BorderPaint {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

/// Native layout proxy with WinUI's optional-brush hit-test rules.
struct RenderBorderPaint {
    /// Render identity and invalidation.
    render_object: RenderObjectData,

    /// Layout geometry used before any paint has occurred.
    render_box: RenderBoxData,

    /// The padded child.
    child: RenderObjectWithChildData<AnyRenderBox>,

    /// Source appearance and hit area.
    style: ControlBorderStyle,
}

impl RenderObjectWithChildMixin for RenderBorderPaint {
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

impl RenderProxyBoxMixin for RenderBorderPaint {}

impl RenderObject for RenderBorderPaint {
    inset_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
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

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let style = self.get(app).style;
        let size = self.size(app);
        let canvas = context.canvas();
        canvas.save();
        canvas.translate(offset.dx() as f32, offset.dy() as f32);
        style.paint(canvas, size);
        canvas.restore();
        RenderProxyBoxMixin::paint(self, app, context, offset);
    }
}

impl RenderBox for RenderBorderPaint {
    inset_rendering::render_box_accessors!();

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

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
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

    fn hit_test_self(self: RenderHandle<Self>, app: &App, position: Offset) -> bool {
        self.get(app).style.hit_test(self.size(app), position)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        // UIElement::HitTestRoundedCornerClip also clips descendants to the outer outline.
        let outer = self.get(app).style.outlines(self.size(app)).0;
        outer.contains(position)
            && RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }
}
