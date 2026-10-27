//! InfoBarPanel.cpp measure and arrange rules, including orientation-specific attached margins.

use inset_embedder::{Offset, Rect, Size};
use inset_foundation::{App, Handle};
use inset_rendering::*;
use inset_widgets::*;
use std::{
    any::{Any, TypeId},
    fmt,
};

/// Attached margins used when the panel lays out horizontally or vertically.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InfoBarPanelMargins {
    /// Left, top, right, bottom margins in horizontal orientation.
    pub horizontal: [f64; 4],

    /// Left, top, right, bottom margins in vertical orientation.
    pub vertical: [f64; 4],
}

/// A panel that switches to vertical layout when its banner content no longer fits.
#[derive(Debug, Default)]
pub struct InfoBarPanel {
    /// Stable identity across rebuilds.
    pub key: Option<KeyRef>,

    /// Padding in left, top, right, bottom order for horizontal layout.
    pub horizontal_orientation_padding: [f64; 4],

    /// Padding in left, top, right, bottom order for vertical layout.
    pub vertical_orientation_padding: [f64; 4],

    /// The containing banner's minimum height minus this panel's top and bottom margins.
    pub parent_min_height: f64,

    /// Children with InfoBarPanelChild attached margins.
    pub children: Vec<WidgetRef>,
}

impl InfoBarPanel {
    /// Creates an empty panel.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets widget identity.
    pub fn key(mut self, value: KeyRef) -> Self {
        self.key = Some(value);
        self
    }

    /// Sets HorizontalOrientationPadding.
    pub fn horizontal_orientation_padding(mut self, value: [f64; 4]) -> Self {
        self.horizontal_orientation_padding = value;
        self
    }

    /// Sets VerticalOrientationPadding.
    pub fn vertical_orientation_padding(mut self, value: [f64; 4]) -> Self {
        self.vertical_orientation_padding = value;
        self
    }

    /// Supplies the parent height used by MeasureOverride's wrapped-content check.
    pub fn parent_min_height(mut self, value: f64) -> Self {
        self.parent_min_height = value;
        self
    }

    /// Sets Children in source order.
    pub fn children(mut self, value: impl IntoIterator<Item = WidgetRef>) -> Self {
        self.children = value.into_iter().collect();
        self
    }
}

impl RenderObjectWidget for InfoBarPanel {
    type RenderObject = RenderInfoBarPanel;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderHandle::new_box(
            app,
            RenderInfoBarPanel {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                horizontal_padding: self.horizontal_orientation_padding,
                vertical_padding: self.vertical_orientation_padding,
                parent_min_height: self.parent_min_height,
            },
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render: RenderHandle<RenderInfoBarPanel>,
    ) {
        let state = render.get_mut(app);
        if state.horizontal_padding != self.horizontal_orientation_padding
            || state.vertical_padding != self.vertical_orientation_padding
            || state.parent_min_height != self.parent_min_height
        {
            state.horizontal_padding = self.horizontal_orientation_padding;
            state.vertical_padding = self.vertical_orientation_padding;
            state.parent_min_height = self.parent_min_height;
            RenderBox::mark_needs_layout(render, app);
        }
    }
}

impl MultiChildRenderObjectWidget for InfoBarPanel {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

/// Places a child with InfoBarPanel's orientation-specific margins.
#[derive(Debug)]
pub struct InfoBarPanelChild {
    /// Stable identity across rebuilds.
    pub key: Option<KeyRef>,

    /// Margins associated with this child.
    pub margins: InfoBarPanelMargins,

    /// Content measured by the panel.
    pub child: WidgetRef,
}

impl InfoBarPanelChild {
    /// Attaches zero margins to a child.
    pub fn new<K>(child: impl IntoWidget<K>) -> Self {
        Self {
            key: None,
            margins: InfoBarPanelMargins::default(),
            child: child.into_widget(),
        }
    }

    /// Sets widget identity.
    pub fn key(mut self, value: KeyRef) -> Self {
        self.key = Some(value);
        self
    }

    /// Sets HorizontalOrientationMargin.
    pub fn horizontal_orientation_margin(mut self, value: [f64; 4]) -> Self {
        self.margins.horizontal = value;
        self
    }

    /// Sets VerticalOrientationMargin.
    pub fn vertical_orientation_margin(mut self, value: [f64; 4]) -> Self {
        self.margins.vertical = value;
        self
    }
}

impl ParentDataWidget for InfoBarPanelChild {
    type ParentData = InfoBarPanelParentData;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn apply_parent_data(&self, app: &mut App, render_object: AnyRenderObject) {
        let data = render_object.parent_data_of_mut::<InfoBarPanelParentData>(app);
        if data.margins != self.margins {
            data.margins = self.margins;
            if let Some(parent) = render_object.parent(app) {
                parent.mark_needs_layout(app);
            }
        }
    }
}

/// The two orientation margins attached to an InfoBarPanel child.
#[derive(Debug)]
pub struct InfoBarPanelParentData {
    /// Position assigned by ArrangeOverride.
    box_parent_data: BoxParentData,

    /// Links in the panel's ordered child list.
    container_parent_data: ContainerParentData<AnyRenderBox>,

    /// Margins selected by the panel orientation.
    pub margins: InfoBarPanelMargins,
}

impl InfoBarPanelParentData {
    /// Initializes an unattached child with zero margins.
    pub fn new() -> InfoBarPanelParentData {
        InfoBarPanelParentData {
            box_parent_data: BoxParentData::new(),
            container_parent_data: ContainerParentData::new(),
            margins: InfoBarPanelMargins::default(),
        }
    }
}

impl Default for InfoBarPanelParentData {
    fn default() -> InfoBarPanelParentData {
        InfoBarPanelParentData::new()
    }
}

impl ParentData for InfoBarPanelParentData {
    fn detach(&mut self) {
        ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<InfoBarPanelParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<InfoBarPanelParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide_mut(id)
    }
}

impl ContainerParentDataMixin for InfoBarPanelParentData {
    type ChildType = AnyRenderBox;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderBox> {
        &self.container_parent_data
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderBox> {
        &mut self.container_parent_data
    }
}

impl ContainerBoxParentData for InfoBarPanelParentData {
    fn box_parent_data(&self) -> &BoxParentData {
        &self.box_parent_data
    }

    fn box_parent_data_mut(&mut self) -> &mut BoxParentData {
        &mut self.box_parent_data
    }
}

impl fmt::Display for InfoBarPanelParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}; {}", self.margins, self.box_parent_data)
    }
}

/// InfoBarPanel's native render object; children keep their desired sizes between measure and arrange.
pub struct RenderInfoBarPanel {
    /// Native render object identity and dirty state.
    render_object: RenderObjectData,

    /// Box geometry and constraints.
    render_box: RenderBoxData,

    /// Linked child list.
    container: ContainerRenderObjectData<AnyRenderBox>,

    /// HorizontalOrientationPadding.
    horizontal_padding: [f64; 4],

    /// VerticalOrientationPadding.
    vertical_padding: [f64; 4],

    /// Parent MinHeight minus panel vertical margins.
    parent_min_height: f64,
}

/// Measurement result retained for ArrangeOverride.
#[derive(Debug)]
struct PanelMeasure {
    /// Whether children stack vertically.
    vertical: bool,

    /// Unconstrained desired panel size.
    desired: Size,
}

/// Transcribes MeasureOverride's aggregation and orientation decision.
fn measure_panel(
    sizes: &[Size],
    margins: &[InfoBarPanelMargins],
    width: f64,
    parent_min_height: f64,
    horizontal_padding: [f64; 4],
    vertical_padding: [f64; 4],
) -> PanelMeasure {
    let mut total_width = 0.0_f64;
    let mut total_height = 0.0_f64;
    let mut widest = 0.0_f64;
    let mut tallest = 0.0_f64;
    let mut tallest_horizontal = 0.0_f64;
    let mut items = 0;

    for (size, margin) in sizes.iter().zip(margins) {
        if size.width() != 0.0 && size.height() != 0.0 {
            total_width += size.width()
                + if items > 0 { margin.horizontal[0] } else { 0.0 }
                + if items < sizes.len() - 1 {
                    margin.horizontal[2]
                } else {
                    0.0
                };
            total_height += size.height()
                + if items > 0 { margin.vertical[1] } else { 0.0 }
                + if items < sizes.len() - 1 {
                    margin.vertical[3]
                } else {
                    0.0
                };
            widest = widest.max(size.width());
            tallest = tallest.max(size.height());
            tallest_horizontal =
                tallest_horizontal.max(size.height() + margin.horizontal[1] + margin.horizontal[3]);
            items += 1;
        }
    }

    let vertical = items == 1
        || total_width > width
        || (parent_min_height > 0.0 && tallest_horizontal > parent_min_height);
    let padding = if vertical {
        vertical_padding
    } else {
        horizontal_padding
    };
    let desired = Size::new(
        (if vertical { widest } else { total_width }) + padding[0] + padding[2],
        (if vertical { total_height } else { tallest }) + padding[1] + padding[3],
    );
    PanelMeasure { vertical, desired }
}

/// Transcribes ArrangeOverride, including the last horizontal child's extra space.
fn arrange_panel(
    sizes: &[Size],
    margins: &[InfoBarPanelMargins],
    width: f64,
    vertical: bool,
    padding: [f64; 4],
) -> Vec<Rect> {
    let mut offset = if vertical { padding[1] } else { padding[0] };
    let mut has_previous = false;
    sizes
        .iter()
        .zip(margins)
        .enumerate()
        .map(|(index, (size, margins))| {
            if size.width() == 0.0 || size.height() == 0.0 {
                return Rect::ZERO;
            }

            let margin = if vertical {
                margins.vertical
            } else {
                margins.horizontal
            };
            offset += if has_previous {
                margin[if vertical { 1 } else { 0 }]
            } else {
                0.0
            };
            let rect = if vertical {
                Rect::from_ltwh(padding[0] + margin[0], offset, size.width(), size.height())
            } else {
                Rect::from_ltwh(
                    offset,
                    padding[1] + margin[1],
                    if index + 1 == sizes.len() {
                        size.width().max(width - offset)
                    } else {
                        size.width()
                    },
                    size.height(),
                )
            };
            offset += if vertical {
                size.height() + margin[3]
            } else {
                size.width() + margin[2]
            };
            has_previous = true;
            rect
        })
        .collect()
}

impl RenderInfoBarPanel {
    /// Children and their attached margins in source order.
    fn entries(
        self: RenderHandle<Self>,
        app: &App,
    ) -> (Vec<AnyRenderBox>, Vec<InfoBarPanelMargins>) {
        let mut children = Vec::new();
        let mut margins = Vec::new();
        let mut child = self.first_child(app);
        while let Some(current) = child {
            children.push(current);
            margins.push(
                current
                    .as_object()
                    .parent_data_of::<InfoBarPanelParentData>(app)
                    .margins,
            );
            child = self.child_after(app, current);
        }
        (children, margins)
    }

    /// Measures children using actual layout or dry layout, with the same source aggregation.
    fn measure(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        dry: bool,
    ) -> (
        Vec<AnyRenderBox>,
        Vec<Size>,
        Vec<InfoBarPanelMargins>,
        PanelMeasure,
    ) {
        let (children, margins) = self.entries(app);
        let available = constraints.loosen();
        let sizes: Vec<_> = children
            .iter()
            .map(|child| {
                if dry {
                    child.get_dry_layout(app, available)
                } else {
                    child.layout(app, available, true);
                    child.size(app)
                }
            })
            .collect();
        let state = self.get(app);
        let measured = measure_panel(
            &sizes,
            &margins,
            constraints.max_width,
            state.parent_min_height,
            state.horizontal_padding,
            state.vertical_padding,
        );
        (children, sizes, margins, measured)
    }
}

impl ContainerRenderObjectMixin for RenderInfoBarPanel {
    type ChildType = AnyRenderBox;
    type ParentDataType = InfoBarPanelParentData;

    fn container_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &ContainerRenderObjectData<AnyRenderBox> {
        &self.get(app).container
    }

    fn container_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut ContainerRenderObjectData<AnyRenderBox> {
        &mut self.get_mut(app).container
    }
}

impl RenderBoxContainerDefaultsMixin for RenderInfoBarPanel {}

impl RenderObject for RenderInfoBarPanel {
    inset_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let (children, sizes, margins, measured) = self.measure(app, constraints, false);
        let size = constraints.constrain(measured.desired);
        let state = self.get(app);
        let padding = if measured.vertical {
            state.vertical_padding
        } else {
            state.horizontal_padding
        };
        let rectangles = arrange_panel(&sizes, &margins, size.width(), measured.vertical, padding);

        for (child, rectangle) in children.iter().zip(rectangles) {
            child.layout(app, BoxConstraints::tight(rectangle.size()), true);
            child
                .parent_data_of_mut::<InfoBarPanelParentData>(app)
                .set_offset(rectangle.top_left());
        }
        self.set_size(app, size);
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor)
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        ContainerRenderObjectMixin::did_attach(self, app, owner)
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::did_detach(self, app)
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::redepth_children(self, app)
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        self.default_paint(app, context, offset)
    }
}

impl RenderBox for RenderInfoBarPanel {
    inset_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<InfoBarPanelParentData>(app) {
            child.set_parent_data(app, InfoBarPanelParentData::new());
        }
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let (_, _, _, measured) = self.measure(app, constraints, true);
        constraints.constrain(measured.desired)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        self.default_hit_test_children(app, result, position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_visible_child_uses_vertical_margins_even_at_wide_width() {
        let sizes = [Size::new(30.0, 20.0), Size::ZERO];
        let margins = [InfoBarPanelMargins::default(); 2];
        let measured = measure_panel(
            &sizes,
            &margins,
            500.0,
            48.0,
            [0.0; 4],
            [1.0, 2.0, 3.0, 4.0],
        );
        assert!(measured.vertical);
        assert_eq!(measured.desired, Size::new(34.0, 26.0));
    }

    #[test]
    fn wrapped_height_forces_vertical_layout_even_when_widths_fit() {
        let sizes = [Size::new(40.0, 20.0), Size::new(100.0, 40.0)];
        let margins = [InfoBarPanelMargins {
            horizontal: [0.0, 8.0, 0.0, 8.0],
            vertical: [0.0; 4],
        }; 2];
        assert!(measure_panel(&sizes, &margins, 500.0, 48.0, [0.0; 4], [0.0; 4]).vertical);
        assert!(!measure_panel(&sizes, &margins, 500.0, 0.0, [0.0; 4], [0.0; 4]).vertical);
        assert!(measure_panel(&sizes, &margins, 139.0, 0.0, [0.0; 4], [0.0; 4]).vertical);
    }

    #[test]
    fn last_actual_horizontal_child_receives_remaining_width() {
        let sizes = [Size::new(30.0, 20.0), Size::new(40.0, 20.0)];
        let margins = [InfoBarPanelMargins {
            horizontal: [7.0, 3.0, 5.0, 0.0],
            vertical: [0.0; 4],
        }; 2];
        let rectangles = arrange_panel(&sizes, &margins, 200.0, false, [2.0, 4.0, 9.0, 0.0]);
        assert_eq!(rectangles[0], Rect::from_ltwh(2.0, 7.0, 30.0, 20.0));
        assert_eq!(rectangles[1], Rect::from_ltwh(44.0, 7.0, 156.0, 20.0));
    }

    #[test]
    fn zero_sized_last_child_does_not_transfer_its_extra_space_to_the_previous_child() {
        let sizes = [Size::new(30.0, 20.0), Size::new(40.0, 20.0), Size::ZERO];
        let margins = [InfoBarPanelMargins::default(); 3];
        let rectangles = arrange_panel(&sizes, &margins, 200.0, false, [0.0; 4]);
        assert_eq!(rectangles[1].width(), 40.0);
        assert_eq!(rectangles[2], Rect::ZERO);
    }
}
