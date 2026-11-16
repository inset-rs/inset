//! `CommandBar`'s `ContentRoot` grid and `CommandBar::MeasureOverrideForDynamicOverflow`: the `ContentControl`, the `PrimaryItemsControl` row of primary commands against the right, and the `MoreButton` in its own column, with the primary commands that do not fit moved into the overflow as the bar measures.
//!
//! WinUI moves commands between collections during measure and measures again until layout settles; here every primary command stays a child of the panel and the panel runs the same measure, then [`DynamicOverflow::measure`], again in one layout until nothing moves, showing the commands still in the bar. The commands it moved are laid out but neither painted nor hit, and the bar's state is told after the layout to rebuild them into the overflow. A later editor must keep the panel the only writer of the model during layout, and the state its only reader until then.

use super::dynamic_overflow::{DynamicOverflow, OverflowMeasure};
use super::layout_root::AppBarMetrics;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use waft_embedder::{Matrix4, Offset, Size};
use waft_foundation::App;
use waft_rendering::*;
use waft_widgets::*;

/// How the `MoreButton` is shown: `EffectiveOverflowButtonVisibility` from what the state knows when it builds, and the overflow's contents, which only the layout knows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MoreButtonVisibility {
    /// Shown whatever the overflow holds.
    pub(crate) shown: bool,
    /// `OverflowButtonVisibility` is `Auto`: shown while the overflow holds anything.
    pub(crate) when_overflowing: bool,
}

/// What the panel lays out by.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ContentRootLayout {
    /// `DynamicOverflowStates`: `DynamicOverflowEnabled` swaps the columns' widths, so the primary commands get what the content leaves.
    pub(crate) dynamic_overflow_enabled: bool,
    pub(crate) more_button: MoreButtonVisibility,
    /// `AppBarMoreButtonColumnMinWidth`.
    pub(crate) more_button_column_min_width: f64,
    /// `ContentRoot.MinHeight` and `PrimaryItemsControl.MinHeight`: `AppBarThemeCompactHeight`.
    pub(crate) min_height: f64,
    /// The `MoreButton`'s `VerticalAlignment` is `Stretch`, in the open states and their transitions.
    pub(crate) more_button_stretches: bool,
    /// `FlowDirection` is right to left: the grid's columns run from the right.
    pub(crate) rtl: bool,
}

/// `ContentRoot`'s children: `ContentControl`, every primary command, `MoreButton`.
pub(crate) struct ContentRoot {
    pub(crate) layout: ContentRootLayout,
    pub(crate) overflow: Rc<RefCell<DynamicOverflow>>,
    pub(crate) metrics: Rc<AppBarMetrics>,
    /// Told after a layout that moved commands or raised `DynamicOverflowItemsChanging`.
    pub(crate) overflow_changed: Rc<dyn Fn(&mut App)>,
    pub(crate) children: Vec<WidgetRef>,
}

impl fmt::Debug for ContentRoot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContentRoot")
            .field("layout", &self.layout)
            .field("children", &self.children.len())
            .finish_non_exhaustive()
    }
}

impl RenderObjectWidget for ContentRoot {
    type RenderObject = RenderContentRoot;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderHandle::new_box(
            app,
            RenderContentRoot {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                layout: self.layout,
                overflow: self.overflow.clone(),
                metrics: self.metrics.clone(),
                overflow_changed: self.overflow_changed.clone(),
                shown_more_button: false,
            },
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render: RenderHandle<RenderContentRoot>,
    ) {
        let panel = render.get_mut(app);
        panel.overflow = self.overflow.clone();
        panel.metrics = self.metrics.clone();
        panel.overflow_changed = self.overflow_changed.clone();
        if panel.layout != self.layout {
            panel.layout = self.layout;
        }
        // The model may have been reset with new commands; the panel measures again.
        render.as_object().mark_needs_layout(app);
    }
}

impl MultiChildRenderObjectWidget for ContentRoot {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

#[derive(Debug, Default)]
pub(crate) struct ContentRootParentData {
    box_parent_data: BoxParentData,
    container_parent_data: ContainerParentData<AnyRenderBox>,
}

impl ParentData for ContentRootParentData {
    fn detach(&mut self) {
        ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<ContentRootParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<ContentRootParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide_mut(id)
    }
}

impl ContainerParentDataMixin for ContentRootParentData {
    type ChildType = AnyRenderBox;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderBox> {
        &self.container_parent_data
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderBox> {
        &mut self.container_parent_data
    }
}

impl ContainerBoxParentData for ContentRootParentData {
    fn box_parent_data(&self) -> &BoxParentData {
        &self.box_parent_data
    }

    fn box_parent_data_mut(&mut self) -> &mut BoxParentData {
        &mut self.box_parent_data
    }
}

impl fmt::Display for ContentRootParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.box_parent_data)
    }
}

/// The sizes one measure of the panel found.
struct Measure {
    more_button_shown: bool,
    more_button: Size,
    /// The `MoreButton`'s column: `Auto` with its minimum width.
    more_column: f64,
    content: Size,
    /// Each primary command's desired size.
    commands: Vec<Size>,
    /// The width of the commands still in the bar.
    commands_width: f64,
    commands_height: f64,
}

/// The render object behind [`ContentRoot`].
pub(crate) struct RenderContentRoot {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    layout: ContentRootLayout,
    overflow: Rc<RefCell<DynamicOverflow>>,
    metrics: Rc<AppBarMetrics>,
    overflow_changed: Rc<dyn Fn(&mut App)>,
    shown_more_button: bool,
}

impl RenderContentRoot {
    fn children(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderBox> {
        let mut children = Vec::new();
        let mut child = self.first_child(app);
        while let Some(current) = child {
            children.push(current);
            child = self.child_after(app, current);
        }
        children
    }

    /// The children painted and hit, in paint order: the content, the commands still in the bar, the `MoreButton` when shown.
    fn shown(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderBox> {
        let children = self.children(app);
        let Some((content, rest)) = children.split_first() else {
            return Vec::new();
        };
        let Some((more, commands)) = rest.split_last() else {
            return vec![*content];
        };
        let overflow = self.get(app).overflow.clone();
        let overflow = overflow.borrow();
        let mut shown = vec![*content];
        shown.extend(
            overflow
                .dynamic_primary
                .iter()
                .filter_map(|index| commands.get(*index).copied()),
        );
        if self.get(app).shown_more_button {
            shown.push(*more);
        }
        shown
    }

    /// `Grid::MeasureOverride` of `ContentRoot` and the inner grid at `width`, the `Auto` columns first.
    fn measure(self: RenderHandle<Self>, app: &mut App, width: f64) -> Measure {
        let layout = self.get(app).layout;
        let children = self.children(app);
        let (content, more) = (children[0], children[children.len() - 1]);
        let commands = &children[1..children.len() - 1];
        let unbounded = BoxConstraints::new();
        let (dynamic_primary, overflowing) = {
            let overflow = self.get(app).overflow.borrow();
            (
                overflow.dynamic_primary.clone(),
                !overflow.dynamic_secondary.is_empty(),
            )
        };
        let more_button_shown =
            layout.more_button.shown || (layout.more_button.when_overflowing && overflowing);
        more.layout(app, unbounded, true);
        let more_button = if more_button_shown {
            more.size(app)
        } else {
            Size::ZERO
        };
        let more_column = more_button.width().max(layout.more_button_column_min_width);
        // The `StackPanel` measures each command without a width limit.
        let mut sizes = Vec::with_capacity(commands.len());
        for command in commands {
            command.layout(app, unbounded, true);
            sizes.push(command.size(app));
        }
        let commands_width: f64 = dynamic_primary.iter().map(|i| sizes[*i].width()).sum();
        let commands_height = dynamic_primary
            .iter()
            .map(|i| sizes[*i].height())
            .fold(layout.min_height, f64::max);
        let content_size = if layout.dynamic_overflow_enabled || !width.is_finite() {
            content.layout(app, unbounded, true);
            content.size(app)
        } else {
            // `ContentControlColumnDefinition` is star: the content gets what the commands leave.
            let room = (width - more_column - commands_width).max(0.0);
            content.layout(app, BoxConstraints::new().max_width(room), true);
            content.size(app)
        };
        Measure {
            more_button_shown,
            more_button,
            more_column,
            content: content_size,
            commands: sizes,
            commands_width,
            commands_height,
        }
    }

    /// The widths `MeasureOverrideForDynamicOverflow` compares for one measure.
    fn overflow_measure(&self, measure: &Measure, width: f64) -> OverflowMeasure {
        let primary_column = (width - measure.more_column - measure.content.width()).max(0.0);
        OverflowMeasure {
            available: width,
            // `DesiredSize` is at most the available size.
            available_primary: measure.commands_width.min(primary_column),
            content_root_desired: measure.content.width()
                + measure.commands_width
                + measure.more_column,
            primary_desired: measure.commands_width,
        }
    }
}

impl ContainerRenderObjectMixin for RenderContentRoot {
    type ChildType = AnyRenderBox;
    type ParentDataType = ContentRootParentData;

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

impl RenderBoxContainerDefaultsMixin for RenderContentRoot {}

impl RenderObject for RenderContentRoot {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let width = constraints.max_width;
        let layout = self.get(app).layout;
        let overflow = self.get(app).overflow.clone();
        let mut measure = self.measure(app, width);
        let mut moved = false;
        if layout.dynamic_overflow_enabled && width.is_finite() {
            // WinUI measures again after each move until the layout settles.
            for _ in 0..measure.commands.len() + 3 {
                let widths: Vec<f64> = measure.commands.iter().map(|size| size.width()).collect();
                let changed = overflow
                    .borrow_mut()
                    .measure(self.get(app).overflow_measure(&measure, width), &|index| {
                        widths[index]
                    });
                if !changed {
                    break;
                }
                moved = true;
                measure = self.measure(app, width);
            }
        }
        self.get_mut(app).shown_more_button = measure.more_button_shown;

        // ArrangeOverride
        let children = self.children(app);
        let (content, more) = (children[0], children[children.len() - 1]);
        let commands = &children[1..children.len() - 1];
        let full_width = if width.is_finite() {
            width
        } else {
            measure.content.width() + measure.commands_width + measure.more_column
        };
        let height = measure
            .content
            .height()
            .max(measure.commands_height)
            .max(measure.more_button.height())
            .max(layout.min_height);
        let size = constraints.constrain(Size::new(full_width, height));
        let first_column = (size.width() - measure.more_column).max(0.0);
        let rtl = layout.rtl;
        // A child at `offset` in left-to-right columns; right to left, its mirror image.
        let set_offset = |app: &mut App, child: AnyRenderBox, offset: Offset| {
            let offset = if rtl {
                Offset::new(
                    size.width() - offset.dx() - child.size(app).width(),
                    offset.dy(),
                )
            } else {
                offset
            };
            child
                .parent_data_of_mut::<ContentRootParentData>(app)
                .set_offset(offset);
        };
        // ContentControl: `HorizontalContentAlignment` Left, `VerticalContentAlignment` Top.
        set_offset(app, content, Offset::ZERO);
        // PrimaryItemsControl: `HorizontalAlignment` Right in its column.
        let commands_column_start = if layout.dynamic_overflow_enabled {
            measure.content.width()
        } else {
            (first_column - measure.commands_width).max(0.0)
        };
        let commands_column = (first_column - commands_column_start).max(0.0);
        let mut x = commands_column_start + (commands_column - measure.commands_width).max(0.0);
        let dynamic_primary = overflow.borrow().dynamic_primary.clone();
        let stretches: Vec<bool> = overflow.borrow().separators();
        for index in dynamic_primary {
            let command = commands[index];
            // A separator stretches to the row; a button keeps its `VerticalAlignment` of Top.
            if stretches[index] {
                command.layout(
                    app,
                    BoxConstraints::tight(Size::new(measure.commands[index].width(), height)),
                    true,
                );
            }
            set_offset(app, command, Offset::new(x, 0.0));
            x += measure.commands[index].width();
        }
        let metrics = self.get(app).metrics.clone();
        if measure.more_button_shown {
            let more_height = if layout.more_button_stretches {
                more.layout(
                    app,
                    BoxConstraints::tight(Size::new(measure.more_button.width(), height)),
                    true,
                );
                height
            } else {
                measure.more_button.height()
            };
            set_offset(app, more, Offset::new(first_column, 0.0));
            let origin = more
                .as_object()
                .parent_data_of::<ContentRootParentData>(app)
                .offset();
            metrics.more_button.set(Some(
                origin & Size::new(measure.more_button.width(), more_height),
            ));
        } else {
            metrics.more_button.set(None);
        }
        metrics.more_button_shown.set(measure.more_button_shown);
        self.set_size(app, size);
        if moved || !overflow.borrow().raised.is_empty() {
            let changed = self.get(app).overflow_changed.clone();
            changed(app);
        }
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor)
    }

    fn did_attach(
        self: RenderHandle<Self>,
        app: &mut App,
        owner: waft_foundation::Handle<PipelineOwner>,
    ) {
        ContainerRenderObjectMixin::did_attach(self, app, owner);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::did_detach(self, app);
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
        for child in self.shown(app) {
            let child_offset = child
                .as_object()
                .parent_data_of::<ContentRootParentData>(app)
                .offset();
            context.paint_child(app, child.as_object(), offset + child_offset);
        }
    }
}

impl RenderBox for RenderContentRoot {
    waft_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<ContentRootParentData>(app) {
            child.set_parent_data(app, ContentRootParentData::default());
        }
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        let offset = child.parent_data_of::<ContentRootParentData>(app).offset();
        *transform = transform.then(&Matrix4::translation(
            offset.dx() as f32,
            offset.dy() as f32,
        ));
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        _app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        constraints.smallest()
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        for child in self.shown(app).into_iter().rev() {
            let child_offset = child
                .as_object()
                .parent_data_of::<ContentRootParentData>(app)
                .offset();
            let hit = result.add_with_paint_offset(
                Some(child_offset),
                position,
                |result, transformed| child.hit_test(app, result, transformed),
            );
            if hit {
                return true;
            }
        }
        false
    }
}
