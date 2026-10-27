//! Native child layout for the source flyout placement calculation.

use super::placement::{
    FlyoutPlacementMode, PointPlacementKind, calculate_placement, calculate_point_placement,
    calculate_submenu_placement,
};
use inset_embedder::{Offset, Rect, Size, TextDirection};
use inset_foundation::App;
use inset_rendering::*;
use inset_widgets::*;

/// Placement inputs copied into the render object for each popup layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct FlyoutLayoutSettings {
    /// Target bounds in root-overlay coordinates.
    pub target: Rect,

    /// Optional root-coordinate point used instead of the target bounds.
    pub position: Option<Offset>,

    /// Root-coordinate rectangle to avoid for point placement.
    pub exclusion: Rect,

    /// Cascading submenus use their source overlap and edge-fitting calculation.
    pub submenu: bool,

    /// Menu point placement starts at the point rather than centering around it.
    pub point_kind: PointPlacementKind,

    /// Requested placement mode.
    pub placement: FlyoutPlacementMode,

    /// Target's flow direction.
    pub direction: TextDirection,

    /// Minimum presenter size from its style.
    pub minimum: Size,

    /// Maximum presenter size from its style.
    pub maximum: Size,

    /// Available root rectangle after keyboard and safe-area insets.
    pub container: Rect,

    /// Whether source placement can try the remaining sides.
    pub allow_fallbacks: bool,
}

/// Measures the presenter before placing it, and lays it out again if the source resizes it.
#[derive(Debug)]
pub(super) struct FlyoutLayout {
    /// Inputs from FlyoutBase's current target and presenter style.
    pub settings: FlyoutLayoutSettings,

    /// Presenter to arrange in the available root area.
    pub child: WidgetRef,
}

impl RenderObjectWidget for FlyoutLayout {
    type RenderObject = RenderFlyoutLayout;

    fn create_render_object(&self, app: &mut App, _: BuildContext) -> AnyRenderObject {
        RenderHandle::new_box(
            app,
            RenderFlyoutLayout {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                settings: self.settings,
            },
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _: BuildContext,
        render: RenderHandle<RenderFlyoutLayout>,
    ) {
        if render.get(app).settings != self.settings {
            render.get_mut(app).settings = self.settings;
            render.as_object().mark_needs_layout(app);
        }
    }
}

impl SingleChildRenderObjectWidget for FlyoutLayout {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

/// Root-sized box that paints and hit-tests its child at the selected position.
pub(super) struct RenderFlyoutLayout {
    /// Native render identity and invalidation.
    render_object: RenderObjectData,

    /// Root-overlay box geometry.
    render_box: RenderBoxData,

    /// Single presenter child.
    child: RenderObjectWithChildData<AnyRenderBox>,

    /// Source placement inputs.
    settings: FlyoutLayoutSettings,
}

impl RenderObjectWithChildMixin for RenderFlyoutLayout {
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

impl RenderShiftedBox for RenderFlyoutLayout {}

impl RenderObject for RenderFlyoutLayout {
    inset_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let root_size = self.constraints(app).biggest();
        assert!(
            root_size.width().is_finite() && root_size.height().is_finite(),
            "flyout layout requires a bounded root overlay"
        );
        if let Some(child) = self.child(app) {
            let settings = self.get(app).settings;
            let width = settings
                .maximum
                .width()
                .min(settings.container.width())
                .max(0.0);
            let height = settings
                .maximum
                .height()
                .min(settings.container.height())
                .max(0.0);
            child.layout(
                app,
                BoxConstraints::new()
                    .min_width(settings.minimum.width().min(width))
                    .max_width(width)
                    .min_height(settings.minimum.height().min(height))
                    .max_height(height),
                true,
            );
            let rectangle = if settings.submenu {
                calculate_submenu_placement(
                    settings.target,
                    child.size(app),
                    settings.direction,
                    settings.container,
                )
            } else if let Some(point) = settings.position {
                calculate_point_placement(
                    settings.placement,
                    settings.direction,
                    point,
                    child.size(app),
                    settings.exclusion,
                    settings.container,
                    settings.point_kind,
                )
            } else {
                calculate_placement(
                    settings.placement,
                    settings.direction,
                    settings.target,
                    child.size(app),
                    settings.minimum,
                    settings.maximum,
                    settings.container,
                    settings.allow_fallbacks,
                )
            };
            child.layout(app, BoxConstraints::tight(rectangle.size()), true);
            child.parent_data_of_mut::<BoxParentData>(app).offset = rectangle.top_left();
        }
        self.set_size(app, root_size);
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

impl RenderBox for RenderFlyoutLayout {
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
        _: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        constraints.biggest()
    }
}
