//! Flutter counterpart: `widgets/overflow_bar.dart`.

use std::any::{Any, TypeId};
use std::fmt;

use waft_embedder::{Offset, Size, TextBaseline, TextDirection};
use waft_foundation::{App, Handle};
use waft_painting::VerticalDirection;
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BaselineOffset, BoxConstraints, BoxHitTestResult, BoxParentData,
    ContainerBoxParentData, ContainerParentData, ContainerParentDataMixin,
    ContainerRenderObjectData, ContainerRenderObjectMixin, MainAxisAlignment, PaintingContext,
    ParentData, PipelineOwner, RenderBox, RenderBoxContainerDefaultsMixin, RenderBoxData,
    RenderHandle, RenderObject, RenderObjectData,
};

use crate::framework::{
    BuildContext, KeyRef, MultiChildRenderObjectWidget, RenderObjectWidget, WidgetRef,
};
use crate::widgets::basic::Directionality;

/// Defines the horizontal alignment of [`OverflowBar`] children when they're laid out in an
/// overflow column.
///
/// This value must be interpreted relative to the ambient [`TextDirection`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OverflowBarAlignment {
    /// Each child is left-aligned for [`TextDirection::Ltr`], right-aligned for
    /// [`TextDirection::Rtl`].
    Start,
    /// Each child is right-aligned for [`TextDirection::Ltr`], left-aligned for
    /// [`TextDirection::Rtl`].
    End,
    /// Each child is horizontally centered.
    Center,
}

/// A widget that lays out its [`children`](Self::children) in a row unless they "overflow"
/// the available horizontal space, in which case it lays them out in a column instead.
///
/// This widget's width will expand to contain its children and the specified
/// [`spacing`](Self::spacing) until it overflows. The overflow column will consume all of the
/// available width. The [`overflow_alignment`](Self::overflow_alignment) defines how each
/// child will be aligned within the overflow column and the
/// [`overflow_spacing`](Self::overflow_spacing) defines the gap between each child.
///
/// The order that the children appear in the horizontal layout is defined by the
/// [`text_direction`](Self::text_direction), just like the `Row` widget. If the layout
/// overflows, then children's order within their column is specified by
/// [`overflow_direction`](Self::overflow_direction) instead.
#[derive(Debug)]
pub struct OverflowBar {
    pub key: Option<KeyRef>,
    /// The width of the gap between [`children`](Self::children) for the default horizontal
    /// layout.
    ///
    /// If the horizontal layout overflows, then [`overflow_spacing`](Self::overflow_spacing) is
    /// used instead.
    ///
    /// Defaults to 0.0.
    pub spacing: f64,
    /// Defines the [`children`](Self::children)'s horizontal layout according to the same
    /// rules as for `Row::main_axis_alignment`.
    ///
    /// If this property is `None` then the overflow bar will behave like a `Row` with
    /// `MainAxisSize::Min` and the children will be laid out at the start of the row.
    ///
    /// Defaults to `None`.
    pub alignment: Option<MainAxisAlignment>,
    /// The height of the gap between [`children`](Self::children) in the vertical "overflow"
    /// layout.
    ///
    /// This parameter is only used if the horizontal layout overflows, i.e. if there isn't
    /// enough horizontal room for the [`children`](Self::children) and
    /// [`spacing`](Self::spacing).
    ///
    /// Defaults to 0.0.
    pub overflow_spacing: f64,
    /// The horizontal alignment of the [`children`](Self::children) within the vertical
    /// "overflow" layout.
    ///
    /// This parameter is only used if the horizontal layout overflows, i.e. if there isn't
    /// enough horizontal room for the [`children`](Self::children) and
    /// [`spacing`](Self::spacing). In that case the overflow bar will expand to fill the
    /// available width and it will layout its [`children`](Self::children) in a column. The
    /// horizontal alignment of each child within that column is defined by this parameter
    /// and depends on the [`text_direction`](Self::text_direction).
    ///
    /// Defaults to [`OverflowBarAlignment::Start`].
    pub overflow_alignment: OverflowBarAlignment,
    /// Defines the order that the [`children`](Self::children) appear in, if the horizontal
    /// layout overflows.
    ///
    /// This parameter is only used if the horizontal layout overflows, i.e. if there isn't
    /// enough horizontal room for the [`children`](Self::children) and
    /// [`spacing`](Self::spacing).
    ///
    /// If the children do not fit into a single row, then they are arranged in a column. The
    /// first child is at the top of the column if this property is set to
    /// [`VerticalDirection::Down`], since it "starts" at the top and "ends" at the bottom. On
    /// the other hand, the first child will be at the bottom of the column if this property
    /// is set to [`VerticalDirection::Up`], since it "starts" at the bottom and "ends" at the
    /// top.
    ///
    /// Defaults to [`VerticalDirection::Down`].
    pub overflow_direction: VerticalDirection,
    /// Determines the order that the [`children`](Self::children) appear in for the default
    /// horizontal layout, and the interpretation of [`OverflowBarAlignment::Start`] and
    /// [`OverflowBarAlignment::End`] for the vertical overflow layout.
    ///
    /// For the default horizontal layout, if this is [`TextDirection::Rtl`] then the last
    /// child will be at the left.
    ///
    /// This parameter defaults to `None`, in which case it defaults to the ambient
    /// [`Directionality`].
    pub text_direction: Option<TextDirection>,
    pub children: Vec<WidgetRef>,
}

impl OverflowBar {
    /// Constructs an [`OverflowBar`]; Dart's optional named arguments are the setters.
    pub fn new() -> OverflowBar {
        OverflowBar {
            key: None,
            spacing: 0.0,
            alignment: None,
            overflow_spacing: 0.0,
            overflow_alignment: OverflowBarAlignment::Start,
            overflow_direction: VerticalDirection::Down,
            text_direction: None,
            children: Vec::new(),
        }
    }

    /// Dart `OverflowBar(key:)`.
    pub fn key(mut self, key: KeyRef) -> OverflowBar {
        self.key = Some(key);
        self
    }

    /// Dart `OverflowBar(spacing:)`.
    pub fn spacing(mut self, spacing: f64) -> OverflowBar {
        self.spacing = spacing;
        self
    }

    /// Dart `OverflowBar(alignment:)`.
    pub fn alignment(mut self, alignment: MainAxisAlignment) -> OverflowBar {
        self.alignment = Some(alignment);
        self
    }

    /// Dart `OverflowBar(overflowSpacing:)`.
    pub fn overflow_spacing(mut self, overflow_spacing: f64) -> OverflowBar {
        self.overflow_spacing = overflow_spacing;
        self
    }

    /// Dart `OverflowBar(overflowAlignment:)`.
    pub fn overflow_alignment(mut self, overflow_alignment: OverflowBarAlignment) -> OverflowBar {
        self.overflow_alignment = overflow_alignment;
        self
    }

    /// Dart `OverflowBar(overflowDirection:)`.
    pub fn overflow_direction(mut self, overflow_direction: VerticalDirection) -> OverflowBar {
        self.overflow_direction = overflow_direction;
        self
    }

    /// Dart `OverflowBar(textDirection:)`.
    pub fn text_direction(mut self, text_direction: TextDirection) -> OverflowBar {
        self.text_direction = Some(text_direction);
        self
    }

    /// Dart `OverflowBar(children:)`.
    pub fn children(mut self, children: Vec<WidgetRef>) -> OverflowBar {
        self.children = children;
        self
    }
}

impl Default for OverflowBar {
    fn default() -> OverflowBar {
        OverflowBar::new()
    }
}

impl RenderObjectWidget for OverflowBar {
    type RenderObject = RenderOverflowBar;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = self
            .text_direction
            .unwrap_or_else(|| Directionality::of(app, context));
        RenderOverflowBar::new(
            app,
            self.spacing,
            self.alignment,
            self.overflow_spacing,
            self.overflow_alignment,
            self.overflow_direction,
            text_direction,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderOverflowBar>,
    ) {
        render_object.set_spacing(app, self.spacing);
        render_object.set_alignment(app, self.alignment);
        render_object.set_overflow_spacing(app, self.overflow_spacing);
        render_object.set_overflow_alignment(app, self.overflow_alignment);
        render_object.set_overflow_direction(app, self.overflow_direction);
        let text_direction = self
            .text_direction
            .unwrap_or_else(|| Directionality::of(app, context));
        render_object.set_text_direction(app, text_direction);
    }
}

impl MultiChildRenderObjectWidget for OverflowBar {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

/// Dart's `_OverflowBarParentData`.
#[derive(Debug, Default)]
pub struct OverflowBarParentData {
    box_parent_data: BoxParentData,
    container_parent_data: ContainerParentData<AnyRenderBox>,
}

impl OverflowBarParentData {
    const fn new() -> OverflowBarParentData {
        OverflowBarParentData {
            box_parent_data: BoxParentData::new(),
            container_parent_data: ContainerParentData::new(),
        }
    }
}

impl ParentData for OverflowBarParentData {
    fn detach(&mut self) {
        ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<OverflowBarParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<OverflowBarParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide_mut(id)
    }
}

impl ContainerParentDataMixin for OverflowBarParentData {
    type ChildType = AnyRenderBox;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderBox> {
        &self.container_parent_data
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderBox> {
        &mut self.container_parent_data
    }
}

impl ContainerBoxParentData for OverflowBarParentData {
    fn box_parent_data(&self) -> &BoxParentData {
        &self.box_parent_data
    }

    fn box_parent_data_mut(&mut self) -> &mut BoxParentData {
        &mut self.box_parent_data
    }
}

impl fmt::Display for OverflowBarParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.box_parent_data, f)
    }
}

/// Dart's private `_RenderOverflowBar`: public only because it is [`OverflowBar`]'s render
/// object type.
pub struct RenderOverflowBar {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    spacing: f64,
    alignment: Option<MainAxisAlignment>,
    overflow_spacing: f64,
    overflow_alignment: OverflowBarAlignment,
    overflow_direction: VerticalDirection,
    text_direction: TextDirection,
}

/// Dart's `childAfter` or `childBefore`, chosen by the overflow direction.
type NextChild = fn(RenderHandle<RenderOverflowBar>, &App, AnyRenderBox) -> Option<AnyRenderBox>;

impl RenderOverflowBar {
    fn new(
        app: &mut App,
        spacing: f64,
        alignment: Option<MainAxisAlignment>,
        overflow_spacing: f64,
        overflow_alignment: OverflowBarAlignment,
        overflow_direction: VerticalDirection,
        text_direction: TextDirection,
    ) -> RenderHandle<RenderOverflowBar> {
        RenderHandle::new_box(
            app,
            RenderOverflowBar {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                spacing,
                alignment,
                overflow_spacing,
                overflow_alignment,
                overflow_direction,
                text_direction,
            },
        )
    }

    /// See [`OverflowBar::spacing`].
    pub fn spacing(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).spacing
    }

    /// Sets [`spacing`](Self::spacing).
    pub fn set_spacing(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).spacing == value {
            return;
        }
        self.get_mut(app).spacing = value;
        self.mark_needs_layout(app);
    }

    /// See [`OverflowBar::alignment`].
    pub fn alignment(self: RenderHandle<Self>, app: &App) -> Option<MainAxisAlignment> {
        self.get(app).alignment
    }

    /// Sets [`alignment`](Self::alignment).
    pub fn set_alignment(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<MainAxisAlignment>,
    ) {
        if self.get(app).alignment == value {
            return;
        }
        self.get_mut(app).alignment = value;
        self.mark_needs_layout(app);
    }

    /// See [`OverflowBar::overflow_spacing`].
    pub fn overflow_spacing(self: RenderHandle<Self>, app: &App) -> f64 {
        self.get(app).overflow_spacing
    }

    /// Sets [`overflow_spacing`](Self::overflow_spacing).
    pub fn set_overflow_spacing(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).overflow_spacing == value {
            return;
        }
        self.get_mut(app).overflow_spacing = value;
        self.mark_needs_layout(app);
    }

    /// See [`OverflowBar::overflow_alignment`].
    pub fn overflow_alignment(self: RenderHandle<Self>, app: &App) -> OverflowBarAlignment {
        self.get(app).overflow_alignment
    }

    /// Sets [`overflow_alignment`](Self::overflow_alignment).
    pub fn set_overflow_alignment(
        self: RenderHandle<Self>,
        app: &mut App,
        value: OverflowBarAlignment,
    ) {
        if self.get(app).overflow_alignment == value {
            return;
        }
        self.get_mut(app).overflow_alignment = value;
        self.mark_needs_layout(app);
    }

    /// See [`OverflowBar::overflow_direction`].
    pub fn overflow_direction(self: RenderHandle<Self>, app: &App) -> VerticalDirection {
        self.get(app).overflow_direction
    }

    /// Sets [`overflow_direction`](Self::overflow_direction).
    pub fn set_overflow_direction(
        self: RenderHandle<Self>,
        app: &mut App,
        value: VerticalDirection,
    ) {
        if self.get(app).overflow_direction == value {
            return;
        }
        self.get_mut(app).overflow_direction = value;
        self.mark_needs_layout(app);
    }

    /// See [`OverflowBar::text_direction`].
    pub fn text_direction(self: RenderHandle<Self>, app: &App) -> TextDirection {
        self.get(app).text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    pub fn set_text_direction(self: RenderHandle<Self>, app: &mut App, value: TextDirection) {
        if self.get(app).text_direction == value {
            return;
        }
        self.get_mut(app).text_direction = value;
        self.mark_needs_layout(app);
    }

    /// The shared body of the two height intrinsics: Dart repeats it with
    /// `getMinIntrinsicHeight` and `getMaxIntrinsicHeight`.
    fn compute_intrinsic_height(
        self: RenderHandle<Self>,
        app: &mut App,
        width: f64,
        child_height: fn(AnyRenderBox, &mut App, f64) -> f64,
    ) -> f64 {
        let mut child = self.first_child(app);
        if child.is_none() {
            return 0.0;
        }
        let child_count = self.child_count(app) as f64;
        let mut bar_width = 0.0;
        while let Some(current) = child {
            bar_width += current.get_min_intrinsic_width(app, f64::INFINITY);
            child = self.child_after(app, current);
        }
        bar_width += self.spacing(app) * (child_count - 1.0);
        let mut height: f64 = 0.0;
        if bar_width > width {
            child = self.first_child(app);
            while let Some(current) = child {
                height += child_height(current, app, width);
                child = self.child_after(app, current);
            }
            height + self.overflow_spacing(app) * (child_count - 1.0)
        } else {
            child = self.first_child(app);
            while let Some(current) = child {
                height = height.max(child_height(current, app, width));
                child = self.child_after(app, current);
            }
            height
        }
    }

    /// The shared body of the two width intrinsics.
    fn compute_intrinsic_width(
        self: RenderHandle<Self>,
        app: &mut App,
        child_width: fn(AnyRenderBox, &mut App, f64) -> f64,
    ) -> f64 {
        let mut child = self.first_child(app);
        if child.is_none() {
            return 0.0;
        }
        let mut width = 0.0;
        while let Some(current) = child {
            width += child_width(current, app, f64::INFINITY);
            child = self.child_after(app, current);
        }
        width + self.spacing(app) * (self.child_count(app) as f64 - 1.0)
    }
}

impl ContainerRenderObjectMixin for RenderOverflowBar {
    type ChildType = AnyRenderBox;
    type ParentDataType = OverflowBarParentData;

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

impl RenderBoxContainerDefaultsMixin for RenderOverflowBar {}

impl RenderObject for RenderOverflowBar {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let mut child = self.first_child(app);
        if child.is_none() {
            self.set_size(app, constraints.smallest());
            return;
        }

        let child_constraints = constraints.loosen();
        let mut children_width = 0.0;
        let mut max_child_height: f64 = 0.0;
        let mut max_child_width: f64 = 0.0;
        while let Some(current) = child {
            current.layout(app, child_constraints, true);
            let child_size = current.size(app);
            children_width += child_size.width();
            max_child_height = max_child_height.max(child_size.height());
            max_child_width = max_child_width.max(child_size.width());
            child = self.child_after(app, current);
        }
        // Dart computes the widest child and leaves it unused.
        let _ = max_child_width;

        let rtl = self.text_direction(app) == TextDirection::Rtl;
        let child_count = self.child_count(app) as f64;
        let spacing = self.spacing(app);
        let actual_width = children_width + spacing * (child_count - 1.0);
        if actual_width > constraints.max_width {
            // Overflow vertical layout
            let down = self.overflow_direction(app) == VerticalDirection::Down;
            child = if down {
                self.first_child(app)
            } else {
                self.last_child(app)
            };
            let next_child: NextChild = if down {
                |this, app, child| this.child_after(app, child)
            } else {
                |this, app, child| this.child_before(app, child)
            };
            let overflow_alignment = self.overflow_alignment(app);
            let overflow_spacing = self.overflow_spacing(app);
            let mut y = 0.0;
            while let Some(current) = child {
                let child_width = current.size(app).width();
                let x = match overflow_alignment {
                    OverflowBarAlignment::Center => (constraints.max_width - child_width) / 2.0,
                    OverflowBarAlignment::Start => {
                        if rtl {
                            constraints.max_width - child_width
                        } else {
                            0.0
                        }
                    }
                    OverflowBarAlignment::End => {
                        if rtl {
                            0.0
                        } else {
                            constraints.max_width - child_width
                        }
                    }
                };
                current
                    .as_object()
                    .parent_data_of_mut::<OverflowBarParentData>(app)
                    .set_offset(Offset::new(x, y));
                y += current.size(app).height() + overflow_spacing;
                child = next_child(self, app, current);
            }
            self.set_size(
                app,
                constraints.constrain(Size::new(constraints.max_width, y - overflow_spacing)),
            );
        } else {
            // Default horizontal layout
            child = self.first_child(app);
            let first_child_width = child.expect("a first child").size(app).width();
            let alignment = self.alignment(app);
            let overall_width = if alignment.is_none() {
                actual_width
            } else {
                constraints.max_width
            };
            self.set_size(
                app,
                constraints.constrain(Size::new(overall_width, max_child_height)),
            );
            let size = self.size(app);
            // initial value: origin of the first child
            let mut x;
            // space between children
            let mut layout_spacing = spacing;
            match alignment {
                None | Some(MainAxisAlignment::Start) => {
                    x = if rtl {
                        size.width() - first_child_width
                    } else {
                        0.0
                    };
                }
                Some(MainAxisAlignment::Center) => {
                    let half_remaining_width = (size.width() - actual_width) / 2.0;
                    x = if rtl {
                        size.width() - half_remaining_width - first_child_width
                    } else {
                        half_remaining_width
                    };
                }
                Some(MainAxisAlignment::End) => {
                    x = if rtl {
                        actual_width - first_child_width
                    } else {
                        size.width() - actual_width
                    };
                }
                Some(MainAxisAlignment::SpaceBetween) => {
                    layout_spacing = (size.width() - children_width) / (child_count - 1.0);
                    x = if rtl {
                        size.width() - first_child_width
                    } else {
                        0.0
                    };
                }
                Some(MainAxisAlignment::SpaceAround) => {
                    layout_spacing = if child_count > 0.0 {
                        (size.width() - children_width) / child_count
                    } else {
                        0.0
                    };
                    x = if rtl {
                        size.width() - layout_spacing / 2.0 - first_child_width
                    } else {
                        layout_spacing / 2.0
                    };
                }
                Some(MainAxisAlignment::SpaceEvenly) => {
                    layout_spacing = (size.width() - children_width) / (child_count + 1.0);
                    x = if rtl {
                        size.width() - layout_spacing - first_child_width
                    } else {
                        layout_spacing
                    };
                }
            }
            while let Some(current) = child {
                let child_size = current.size(app);
                current
                    .as_object()
                    .parent_data_of_mut::<OverflowBarParentData>(app)
                    .set_offset(Offset::new(
                        x,
                        (max_child_height - child_size.height()) / 2.0,
                    ));
                // x is the horizontal origin of child. To advance x to the next child's
                // origin for LTR: add the width of the current child. To advance x to
                // the origin of the next child for RTL: subtract the width of the next
                // child (if there is one).
                if !rtl {
                    x += child_size.width() + layout_spacing;
                }
                child = self.child_after(app, current);
                if rtl && let Some(next) = child {
                    x -= next.size(app).width() + layout_spacing;
                }
            }
        }
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
        self.default_paint(app, context, offset);
    }
}

impl RenderBox for RenderOverflowBar {
    waft_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<OverflowBarParentData>(app) {
            child.set_parent_data(app, OverflowBarParentData::new());
        }
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        self.compute_intrinsic_height(app, width, |child, app, width| {
            child.get_min_intrinsic_height(app, width)
        })
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        self.compute_intrinsic_height(app, width, |child, app, width| {
            child.get_max_intrinsic_height(app, width)
        })
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, _height: f64) -> f64 {
        self.compute_intrinsic_width(app, |child, app, height| {
            child.get_min_intrinsic_width(app, height)
        })
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, _height: f64) -> f64 {
        self.compute_intrinsic_width(app, |child, app, height| {
            child.get_max_intrinsic_width(app, height)
        })
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        self.default_compute_distance_to_highest_actual_baseline(app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child_constraints = constraints.loosen();
        let (next, start_child): (NextChild, Option<AnyRenderBox>) =
            match self.overflow_direction(app) {
                VerticalDirection::Down => (
                    |this, app, child| this.child_after(app, child),
                    self.first_child(app),
                ),
                VerticalDirection::Up => (
                    |this, app, child| this.child_before(app, child),
                    self.last_child(app),
                ),
            };
        let overflow_spacing = self.overflow_spacing(app);
        let mut max_child_height = 0.0;
        let mut y = 0.0;
        let mut children_width = 0.0;
        let mut min_horizontal_baseline = BaselineOffset::NO_BASELINE;
        let mut vertical_baseline = BaselineOffset::NO_BASELINE;
        let mut child = start_child;
        while let Some(current) = child {
            let child_size = current.get_dry_layout(app, child_constraints);
            let height_diff = child_size.height() - max_child_height;
            if height_diff > 0.0 {
                min_horizontal_baseline = min_horizontal_baseline + height_diff / 2.0;
                max_child_height = child_size.height();
            }
            let baseline_offset =
                BaselineOffset(current.get_dry_baseline(app, child_constraints, baseline));
            if baseline_offset.offset().is_some() {
                if vertical_baseline.offset().is_none() {
                    vertical_baseline = baseline_offset + y;
                }
                min_horizontal_baseline = min_horizontal_baseline
                    .min_of(baseline_offset + (max_child_height - child_size.height()));
            }
            y += child_size.height() + overflow_spacing;
            children_width += child_size.width();
            child = next(self, app, current);
        }
        debug_assert!(
            vertical_baseline.offset().is_none() == min_horizontal_baseline.offset().is_none()
        );
        if children_width + self.spacing(app) * (self.child_count(app) as f64 - 1.0)
            > constraints.max_width
        {
            vertical_baseline.offset()
        } else {
            min_horizontal_baseline.offset()
        }
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let mut child = self.first_child(app);
        if child.is_none() {
            return constraints.smallest();
        }
        let child_constraints = constraints.loosen();
        let overflow_spacing = self.overflow_spacing(app);
        let mut children_width = 0.0;
        let mut max_child_height: f64 = 0.0;
        let mut y = 0.0;
        while let Some(current) = child {
            let child_size = current.get_dry_layout(app, child_constraints);
            children_width += child_size.width();
            max_child_height = max_child_height.max(child_size.height());
            y += child_size.height() + overflow_spacing;
            child = self.child_after(app, current);
        }
        let actual_width =
            children_width + self.spacing(app) * (self.child_count(app) as f64 - 1.0);
        if actual_width > constraints.max_width {
            constraints.constrain(Size::new(constraints.max_width, y - overflow_spacing))
        } else {
            let overall_width = if self.alignment(app).is_none() {
                actual_width
            } else {
                constraints.max_width
            };
            constraints.constrain(Size::new(overall_width, max_child_height))
        }
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
    use std::rc::Rc;

    use waft_embedder::Rect;
    use waft_foundation::AppCell;
    use waft_painting::AlignmentGeometry;

    use super::*;
    use crate::framework::{GlobalKey, IntoWidget, LabeledGlobalKey};
    use crate::test_harness::Harness;
    use crate::widgets::basic::{
        Align, Center, Column, ConstrainedBox, IntrinsicHeight, IntrinsicWidth, Row, SizedBox,
    };
    use crate::widgets::container::Container;

    fn keyed(key: &LabeledGlobalKey, width: f64, height: f64) -> WidgetRef {
        SizedBox::new()
            .key(Rc::new(key.clone()))
            .width(width)
            .height(height)
            .into_widget()
    }

    fn rect_of(app: &mut App, key: &LabeledGlobalKey) -> Rect {
        let render_box = key
            .current_context(app)
            .expect("mounted")
            .find_render_object(app)
            .expect("a render object")
            .as_box()
            .expect("a box");
        render_box.local_to_global(app, Offset::ZERO, None) & render_box.size(app)
    }

    /// The three keyed children's rectangles, all checked as Dart's frames check them.
    fn assert_rects(app: &mut App, keys: &[LabeledGlobalKey; 3], expected: [Rect; 3]) {
        let actual = [
            rect_of(app, &keys[0]),
            rect_of(app, &keys[1]),
            rect_of(app, &keys[2]),
        ];
        assert_eq!(actual, expected);
    }

    fn mount(app: &mut App, harness: &mut Option<Harness>, tree: WidgetRef) {
        match harness {
            Some(harness) => {
                harness.set_child(app, tree);
                harness.pump(app);
            }
            None => {
                let mounted = Harness::mount(app, tree);
                mounted.pump(app);
                *harness = Some(mounted);
            }
        }
    }

    fn bar_size(app: &App, harness: &Option<Harness>) -> Size {
        let root = harness.as_ref().expect("mounted").render_root(app);
        let mut found = None;
        fn walk(app: &App, node: AnyRenderObject, found: &mut Option<Size>) {
            if let Some(bar) = node.downcast::<RenderOverflowBar>(app) {
                *found = Some(bar.size(app));
                return;
            }
            node.visit_children(app, &mut |child| {
                if found.is_none() {
                    walk(app, child, found);
                }
            });
        }
        walk(app, root.as_object(), &mut found);
        found.expect("an overflow bar")
    }

    /// "OverflowBar documented defaults".
    #[test]
    fn the_documented_defaults() {
        let bar = OverflowBar::new();
        assert_eq!(bar.spacing, 0.0);
        assert_eq!(bar.alignment, None);
        assert_eq!(bar.overflow_spacing, 0.0);
        assert_eq!(bar.overflow_direction, VerticalDirection::Down);
        assert_eq!(bar.text_direction, None);
        assert!(bar.children.is_empty());
    }

    /// "Empty OverflowBar".
    #[test]
    fn an_empty_bar_takes_the_smallest_size() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let mut harness = None;
        let size = Size::new(16.0, 24.0);
        mount(
            &mut app,
            &mut harness,
            Directionality::new(
                TextDirection::Ltr,
                Center::new().child(
                    ConstrainedBox::new(BoxConstraints::tight(size)).child(OverflowBar::new()),
                ),
            )
            .into_widget(),
        );
        assert_eq!(bar_size(&app, &harness), size);
        mount(
            &mut app,
            &mut harness,
            Directionality::new(TextDirection::Ltr, Center::new().child(OverflowBar::new()))
                .into_widget(),
        );
        assert_eq!(bar_size(&app, &harness), Size::ZERO);
    }

    /// "OverflowBar horizontal layout".
    #[test]
    fn a_bar_that_fits_lays_its_children_out_in_a_centred_row() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let keys = [
            <dyn GlobalKey>::new(),
            <dyn GlobalKey>::new(),
            <dyn GlobalKey>::new(),
        ];
        let mut harness = None;
        let mut frame = |app: &mut App, spacing: f64, text_direction: TextDirection| {
            let tree = Directionality::new(
                text_direction,
                Align::new().alignment(AlignmentGeometry::TOP_LEFT).child(
                    OverflowBar::new().spacing(spacing).children(vec![
                        keyed(&keys[0], 48.0, 48.0),
                        keyed(&keys[1], 64.0, 64.0),
                        keyed(&keys[2], 32.0, 32.0),
                    ]),
                ),
            )
            .into_widget();
            mount(app, &mut harness, tree);
        };
        frame(&mut app, 0.0, TextDirection::Ltr);
        assert_rects(
            &mut app,
            &keys,
            [
                Rect::from_ltrb(0.0, 8.0, 48.0, 56.0),
                Rect::from_ltrb(48.0, 0.0, 112.0, 64.0),
                Rect::from_ltrb(112.0, 16.0, 144.0, 48.0),
            ],
        );
        frame(&mut app, 10.0, TextDirection::Ltr);
        assert_rects(
            &mut app,
            &keys,
            [
                Rect::from_ltrb(0.0, 8.0, 48.0, 56.0),
                Rect::from_ltrb(58.0, 0.0, 122.0, 64.0),
                Rect::from_ltrb(132.0, 16.0, 164.0, 48.0),
            ],
        );
        // Children appear in reverse order for RTL.
        frame(&mut app, 0.0, TextDirection::Rtl);
        assert_rects(
            &mut app,
            &keys,
            [
                Rect::from_ltrb(96.0, 8.0, 144.0, 56.0),
                Rect::from_ltrb(32.0, 0.0, 96.0, 64.0),
                Rect::from_ltrb(0.0, 16.0, 32.0, 48.0),
            ],
        );
        frame(&mut app, 10.0, TextDirection::Rtl);
        assert_rects(
            &mut app,
            &keys,
            [
                Rect::from_ltrb(116.0, 8.0, 164.0, 56.0),
                Rect::from_ltrb(42.0, 0.0, 106.0, 64.0),
                Rect::from_ltrb(0.0, 16.0, 32.0, 48.0),
            ],
        );
    }

    /// "OverflowBar vertical layout".
    #[test]
    fn a_bar_that_overflows_lays_its_children_out_in_an_aligned_column() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let keys = [
            <dyn GlobalKey>::new(),
            <dyn GlobalKey>::new(),
            <dyn GlobalKey>::new(),
        ];
        let mut harness = None;
        let mut frame = |app: &mut App,
                         overflow_spacing: f64,
                         overflow_direction: VerticalDirection,
                         overflow_alignment: OverflowBarAlignment,
                         text_direction: TextDirection| {
            let tree = Directionality::new(
                text_direction,
                Align::new().alignment(AlignmentGeometry::TOP_LEFT).child(
                    ConstrainedBox::new(BoxConstraints::loose(Size::new(100.0, f64::INFINITY)))
                        .child(
                            OverflowBar::new()
                                .overflow_spacing(overflow_spacing)
                                .overflow_alignment(overflow_alignment)
                                .overflow_direction(overflow_direction)
                                .children(vec![
                                    keyed(&keys[0], 48.0, 48.0),
                                    keyed(&keys[1], 64.0, 64.0),
                                    keyed(&keys[2], 32.0, 32.0),
                                ]),
                        ),
                ),
            )
            .into_widget();
            mount(app, &mut harness, tree);
        };
        use OverflowBarAlignment::{Center as Middle, End, Start};
        use VerticalDirection::{Down, Up};

        // Children are left aligned.
        // Children are left aligned.
        frame(&mut app, 0.0, Down, Start, TextDirection::Ltr);
        assert_rects(
            &mut app,
            &keys,
            [
                Rect::from_ltrb(0.0, 0.0, 48.0, 48.0),
                Rect::from_ltrb(0.0, 48.0, 64.0, 112.0),
                Rect::from_ltrb(0.0, 112.0, 32.0, 144.0),
            ],
        );
        frame(&mut app, 0.0, Down, End, TextDirection::Rtl);
        assert_rects(
            &mut app,
            &keys,
            [
                Rect::from_ltrb(0.0, 0.0, 48.0, 48.0),
                Rect::from_ltrb(0.0, 48.0, 64.0, 112.0),
                Rect::from_ltrb(0.0, 112.0, 32.0, 144.0),
            ],
        );
        // Spaced children are left aligned.
        frame(&mut app, 10.0, Down, Start, TextDirection::Ltr);
        assert_rects(
            &mut app,
            &keys,
            [
                Rect::from_ltrb(0.0, 0.0, 48.0, 48.0),
                Rect::from_ltrb(0.0, 58.0, 64.0, 122.0),
                Rect::from_ltrb(0.0, 132.0, 32.0, 164.0),
            ],
        );
        // Left-aligned children appear in reverse order for VerticalDirection::Up.
        frame(&mut app, 0.0, Up, Start, TextDirection::Ltr);
        assert_rects(
            &mut app,
            &keys,
            [
                Rect::from_ltrb(0.0, 96.0, 48.0, 144.0),
                Rect::from_ltrb(0.0, 32.0, 64.0, 96.0),
                Rect::from_ltrb(0.0, 0.0, 32.0, 32.0),
            ],
        );
        frame(&mut app, 10.0, Up, Start, TextDirection::Ltr);
        assert_rects(
            &mut app,
            &keys,
            [
                Rect::from_ltrb(0.0, 116.0, 48.0, 164.0),
                Rect::from_ltrb(0.0, 42.0, 64.0, 106.0),
                Rect::from_ltrb(0.0, 0.0, 32.0, 32.0),
            ],
        );
        // Children are right aligned.
        frame(&mut app, 0.0, Down, End, TextDirection::Ltr);
        assert_rects(
            &mut app,
            &keys,
            [
                Rect::from_ltrb(52.0, 0.0, 100.0, 48.0),
                Rect::from_ltrb(36.0, 48.0, 100.0, 112.0),
                Rect::from_ltrb(68.0, 112.0, 100.0, 144.0),
            ],
        );
        frame(&mut app, 0.0, Down, Start, TextDirection::Rtl);
        assert_rects(
            &mut app,
            &keys,
            [
                Rect::from_ltrb(52.0, 0.0, 100.0, 48.0),
                Rect::from_ltrb(36.0, 48.0, 100.0, 112.0),
                Rect::from_ltrb(68.0, 112.0, 100.0, 144.0),
            ],
        );
        // Children are centered.
        frame(&mut app, 0.0, Down, Middle, TextDirection::Ltr);
        assert_rects(
            &mut app,
            &keys,
            [
                Rect::from_ltrb(26.0, 0.0, 74.0, 48.0),
                Rect::from_ltrb(18.0, 48.0, 82.0, 112.0),
                Rect::from_ltrb(34.0, 112.0, 66.0, 144.0),
            ],
        );
    }

    fn intrinsic_children() -> Vec<WidgetRef> {
        vec![
            SizedBox::new().width(48.0).height(50.0).into_widget(),
            SizedBox::new().width(64.0).height(25.0).into_widget(),
            SizedBox::new().width(32.0).height(75.0).into_widget(),
        ]
    }

    /// "OverflowBar intrinsic width" and "OverflowBar intrinsic height".
    #[test]
    fn the_intrinsics_follow_the_row_or_the_column() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let mut harness = None;
        let bar = || {
            OverflowBar::new()
                .spacing(4.0)
                .overflow_spacing(8.0)
                .children(intrinsic_children())
        };
        for (width, expected) in [(300.0, 152.0), (150.0, 150.0)] {
            let tree = Directionality::new(
                TextDirection::Ltr,
                Center::new().child(
                    Container::new()
                        .width(width)
                        .alignment(AlignmentGeometry::TOP_LEFT)
                        .child(IntrinsicWidth::new().child(bar())),
                ),
            )
            .into_widget();
            mount(&mut app, &mut harness, tree);
            assert_eq!(bar_size(&app, &harness).width(), expected, "width {width}");
        }
        harness = None;
        for (width, expected) in [(300.0, 75.0), (150.0, 166.0)] {
            let tree = Directionality::new(
                TextDirection::Ltr,
                Center::new().child(
                    Container::new()
                        .width(width)
                        .alignment(AlignmentGeometry::TOP_LEFT)
                        .child(IntrinsicHeight::new().child(bar())),
                ),
            )
            .into_widget();
            mount(&mut app, &mut harness, tree);
            assert_eq!(bar_size(&app, &harness).height(), expected, "width {width}");
        }
    }

    /// "OverflowBar is wider that its intrinsic width".
    #[test]
    fn a_bar_wider_than_its_intrinsic_width_starts_its_row_at_the_start() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let keys = [
            <dyn GlobalKey>::new(),
            <dyn GlobalKey>::new(),
            <dyn GlobalKey>::new(),
        ];
        let mut harness = None;
        for (text_direction, lefts) in [
            (TextDirection::Ltr, [0.0, 60.0, 130.0]),
            (TextDirection::Rtl, [250.0, 180.0, 100.0]),
        ] {
            let tree = Directionality::new(
                text_direction,
                // The test root's constraints are loose where Flutter's test view's are tight, so
                // the box gives the bar the whole view itself.
                SizedBox::new().width(300.0).height(200.0).child(
                    OverflowBar::new().spacing(10.0).children(vec![
                        keyed(&keys[0], 50.0, 50.0),
                        keyed(&keys[1], 60.0, 50.0),
                        keyed(&keys[2], 70.0, 50.0),
                    ]),
                ),
            )
            .into_widget();
            mount(&mut app, &mut harness, tree);
            assert_eq!(bar_size(&app, &harness), Size::new(300.0, 200.0));
            for (key, left) in keys.iter().zip(lefts) {
                assert_eq!(rect_of(&mut app, key).left, left, "{text_direction:?}");
            }
        }
    }

    /// "OverflowBar with alignment should match Row with mainAxisAlignment".
    #[test]
    fn an_aligned_bar_matches_a_row_with_that_main_axis_alignment() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let mut harness = None;
        for alignment in [
            MainAxisAlignment::Start,
            MainAxisAlignment::Center,
            MainAxisAlignment::End,
            MainAxisAlignment::SpaceBetween,
            MainAxisAlignment::SpaceAround,
            MainAxisAlignment::SpaceEvenly,
        ] {
            for text_direction in [TextDirection::Ltr, TextDirection::Rtl] {
                let bar_keys = [
                    <dyn GlobalKey>::new(),
                    <dyn GlobalKey>::new(),
                    <dyn GlobalKey>::new(),
                ];
                let row_keys = [
                    <dyn GlobalKey>::new(),
                    <dyn GlobalKey>::new(),
                    <dyn GlobalKey>::new(),
                ];
                let children = |keys: &[LabeledGlobalKey; 3]| {
                    vec![
                        keyed(&keys[0], 50.0, 50.0),
                        keyed(&keys[1], 70.0, 50.0),
                        keyed(&keys[2], 80.0, 50.0),
                    ]
                };
                let tree = Directionality::new(
                    text_direction,
                    Column::new().children(vec![
                        OverflowBar::new()
                            .alignment(alignment)
                            .children(children(&bar_keys))
                            .into_widget(),
                        Row::new()
                            .main_axis_alignment(alignment)
                            .children(children(&row_keys))
                            .into_widget(),
                    ]),
                )
                .into_widget();
                mount(&mut app, &mut harness, tree);
                assert_eq!(bar_size(&app, &harness), Size::new(300.0, 50.0));
                for (bar_key, row_key) in bar_keys.iter().zip(&row_keys) {
                    let bar_rect = rect_of(&mut app, bar_key);
                    let row_rect = rect_of(&mut app, row_key);
                    assert_eq!(bar_rect.size(), row_rect.size());
                    assert_eq!(
                        (bar_rect.left, bar_rect.right),
                        (row_rect.left, row_rect.right),
                        "{alignment:?} {text_direction:?}"
                    );
                }
            }
        }
    }

    /// "OverflowBar does not crash at zero area".
    #[test]
    fn a_bar_at_zero_area_takes_no_space() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        waft_painting::PaintingBinding::instance(&mut app).install_fonts(&mut app, |fonts| {
            fonts.add_source(valo_system_fonts::SystemFonts::load());
        });
        let mut harness = None;
        mount(
            &mut app,
            &mut harness,
            Directionality::new(
                TextDirection::Ltr,
                Center::new().child(SizedBox::shrink().child(OverflowBar::new().children(vec![
                    crate::Text::new("X").into_widget(),
                    crate::Text::new("Y").into_widget(),
                ]))),
            )
            .into_widget(),
        );
        assert_eq!(bar_size(&app, &harness), Size::ZERO);
    }
}
