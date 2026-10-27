//! BreadcrumbLayout.cpp: measure every item, then retain the suffix that fits beside the ellipsis.

use inset_embedder::{Offset, Size, TextDirection};
use inset_foundation::{App, Handle};
use inset_rendering::*;
use inset_widgets::*;
use std::{
    any::{Any, TypeId},
    fmt,
    rc::Rc,
};

/// Result of BreadcrumbLayout's measure and arrange decisions; index zero is the ellipsis.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct BreadcrumbLayoutInfo {
    /// Whether the ellipsis occupies space in the bar.
    pub ellipsis: bool,

    /// First visible ordinary item, using indices that include the ellipsis.
    pub first: usize,
}

/// Receives the completed layout before input can inspect the hidden item list.
pub(crate) type BreadcrumbLayoutCallback = Rc<dyn Fn(&mut App, BreadcrumbLayoutInfo)>;

/// Source non-virtualizing layout; hidden children keep native offstage layout semantics.
pub(crate) struct BreadcrumbLayout {
    /// Ellipsis followed by the ordinary items in source order.
    pub children: Vec<WidgetRef>,

    /// Direction inherited from the bar.
    pub direction: TextDirection,

    /// Updates the owner's overflow information without rebuilding during layout.
    pub on_layout: BreadcrumbLayoutCallback,
}

impl fmt::Debug for BreadcrumbLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BreadcrumbLayout")
            .field("children", &self.children.len())
            .finish()
    }
}

impl RenderObjectWidget for BreadcrumbLayout {
    type RenderObject = RenderBreadcrumbLayout;

    fn create_render_object(&self, app: &mut App, _: BuildContext) -> AnyRenderObject {
        RenderHandle::new_box(
            app,
            RenderBreadcrumbLayout {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                direction: self.direction,
                on_layout: self.on_layout.clone(),
            },
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _: BuildContext,
        render: RenderHandle<Self::RenderObject>,
    ) {
        render.get_mut(app).direction = self.direction;
        render.get_mut(app).on_layout = self.on_layout.clone();
        RenderBox::mark_needs_layout(render, app);
    }
}

impl MultiChildRenderObjectWidget for BreadcrumbLayout {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

/// Assigned position and whether this retained child participates in painting and hit testing.
#[derive(Debug)]
pub(crate) struct BreadcrumbParentData {
    /// Native offset.
    box_parent_data: BoxParentData,

    /// Native ordered child links.
    container_parent_data: ContainerParentData<AnyRenderBox>,

    /// Hidden children are laid out but not painted or hit tested, like Offstage.
    visible: bool,
}

impl Default for BreadcrumbParentData {
    fn default() -> Self {
        Self {
            box_parent_data: BoxParentData::new(),
            container_parent_data: ContainerParentData::new(),
            visible: false,
        }
    }
}

impl ParentData for BreadcrumbParentData {
    fn detach(&mut self) {
        ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<Self>() {
            Some(self)
        } else {
            self.box_parent_data.provide(id)
        }
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<Self>() {
            Some(self)
        } else {
            self.box_parent_data.provide_mut(id)
        }
    }
}

impl ContainerParentDataMixin for BreadcrumbParentData {
    type ChildType = AnyRenderBox;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderBox> {
        &self.container_parent_data
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderBox> {
        &mut self.container_parent_data
    }
}

impl ContainerBoxParentData for BreadcrumbParentData {
    fn box_parent_data(&self) -> &BoxParentData {
        &self.box_parent_data
    }

    fn box_parent_data_mut(&mut self) -> &mut BoxParentData {
        &mut self.box_parent_data
    }
}

impl fmt::Display for BreadcrumbParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}; visible={}", self.box_parent_data, self.visible)
    }
}

/// Native renderer for BreadcrumbLayout's two source passes.
pub(crate) struct RenderBreadcrumbLayout {
    /// Native render identity and invalidation.
    render_object: RenderObjectData,

    /// Native box constraints and size.
    render_box: RenderBoxData,

    /// Children in source order.
    container: ContainerRenderObjectData<AnyRenderBox>,

    /// Mirrors arrangement without changing source indices.
    direction: TextDirection,

    /// Reports the computed visible suffix.
    on_layout: BreadcrumbLayoutCallback,
}

/// MeasureOverride and GetFirstBreadcrumbBarItemToArrange, including the always-visible last item.
fn measure(sizes: &[Size], width: f64) -> (Size, BreadcrumbLayoutInfo) {
    let desired = Size::new(
        sizes.iter().skip(1).map(|s| s.width()).sum(),
        sizes
            .iter()
            .skip(1)
            .fold(0.0_f64, |height, s| height.max(s.height())),
    );
    let mut info = BreadcrumbLayoutInfo {
        ellipsis: desired.width() > width,
        first: 1,
    };
    if info.ellipsis && sizes.len() > 1 {
        info.first = sizes.len() - 1;
        let mut used = sizes[0].width() + sizes[info.first].width();
        for index in (1..info.first).rev() {
            if used + sizes[index].width() > width {
                break;
            }
            used += sizes[index].width();
            info.first = index;
        }
    }
    (desired, info)
}

impl RenderBreadcrumbLayout {
    /// Visits retained children, including hidden ones, in their original order.
    fn children(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderBox> {
        let mut result = Vec::new();
        let mut child = self.first_child(app);
        while let Some(node) = child {
            result.push(node);
            child = self.child_after(app, node);
        }
        result
    }
}

impl ContainerRenderObjectMixin for RenderBreadcrumbLayout {
    type ChildType = AnyRenderBox;
    type ParentDataType = BreadcrumbParentData;

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

impl RenderBoxContainerDefaultsMixin for RenderBreadcrumbLayout {}

impl RenderObject for RenderBreadcrumbLayout {
    inset_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let children = self.children(app);
        let sizes: Vec<_> = children
            .iter()
            .map(|child| {
                child.layout(app, constraints.loosen(), true);
                child.size(app)
            })
            .collect();
        let (desired, info) = measure(&sizes, constraints.max_width);
        let size = constraints.constrain(desired);
        let height = sizes
            .iter()
            .enumerate()
            .filter(|(i, _)| (*i == 0 && info.ellipsis) || (*i > 0 && *i >= info.first))
            .fold(0.0_f64, |height, (_, size)| height.max(size.height()));
        let rtl = self.get(app).direction == TextDirection::Rtl;
        let mut offset = 0.0;
        for (index, child) in children.iter().enumerate() {
            let visible = if index == 0 {
                info.ellipsis
            } else {
                index >= info.first
            };
            if visible {
                child.layout(
                    app,
                    BoxConstraints::tight(Size::new(sizes[index].width(), height)),
                    true,
                );
            }
            let position = Offset::new(
                if rtl {
                    size.width() - offset - sizes[index].width()
                } else {
                    offset
                },
                0.0,
            );
            let data = child.parent_data_of_mut::<BreadcrumbParentData>(app);
            data.visible = visible;
            data.set_offset(if visible { position } else { Offset::ZERO });
            if visible {
                offset += sizes[index].width();
            }
        }
        self.set_size(app, size);
        let callback = self.get(app).on_layout.clone();
        callback(app, info);
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor);
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        ContainerRenderObjectMixin::did_attach(self, app, owner);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::did_detach(self, app);
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::redepth_children(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        for child in self.children(app) {
            let data = child
                .as_object()
                .parent_data_of::<BreadcrumbParentData>(app);
            if data.visible {
                let position = offset + data.offset();
                context.paint_child(app, child.as_object(), position);
            }
        }
    }
}

impl RenderBox for RenderBreadcrumbLayout {
    inset_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<BreadcrumbParentData>(app) {
            child.set_parent_data(app, BreadcrumbParentData::default());
        }
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let sizes: Vec<_> = self
            .children(app)
            .iter()
            .map(|child| child.get_dry_layout(app, constraints.loosen()))
            .collect();
        constraints.constrain(measure(&sizes, constraints.max_width).0)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        for child in self.children(app).into_iter().rev() {
            let data = child
                .as_object()
                .parent_data_of::<BreadcrumbParentData>(app);
            if data.visible {
                let offset = data.offset();
                if result.add_with_paint_offset(Some(offset), position, |result, position| {
                    child.hit_test(app, result, position)
                }) {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffix_fits_with_ellipsis_and_always_retains_current_item() {
        let sizes = [
            Size::new(20.0, 20.0),
            Size::new(70.0, 20.0),
            Size::new(40.0, 20.0),
            Size::new(50.0, 20.0),
        ];
        assert!(!measure(&sizes, 160.0).1.ellipsis);
        assert_eq!(
            measure(&sizes, 110.0).1,
            BreadcrumbLayoutInfo {
                ellipsis: true,
                first: 2
            }
        );
        assert_eq!(measure(&sizes, 30.0).1.first, 3);
        assert_eq!(measure(&[], 0.0).0, Size::ZERO);
    }
}
