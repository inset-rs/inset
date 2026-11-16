//! Flutter counterpart: `widgets/sliver_fill.dart`.
//!
//! `SliverFillRemaining.debugFillProperties` waits for diagnostics.

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::{Matrix4, Offset, clamp_double};
use waft_foundation::App;
use waft_painting::{Axis, EdgeInsets};
use waft_rendering::{
    AnyRenderObject, AnyRenderSliver, PaintingContext, RenderHandle, RenderObject,
    RenderObjectData, RenderObjectWithChildData, RenderObjectWithChildMixin, RenderSliver,
    RenderSliverData, RenderSliverEdgeInsetsPadding, RenderSliverFillRemaining,
    RenderSliverFillRemainingAndOverscroll, RenderSliverFillRemainingWithScrollable,
    RenderSliverFillViewport, SliverConstraints, SliverHitTestResult,
};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, RenderObjectWidget, SingleChildRenderObjectWidget,
    StatelessWidget, WidgetRef,
};
use crate::widgets::scroll_delegate::SliverChildDelegateRef;
use crate::widgets::sliver::{
    SliverMultiBoxAdaptorWidget, SliverWithKeepAliveWidget, child_manager_of,
};

/// A sliver that contains multiple box children that each fills the viewport.
///
/// _To learn more about slivers, see `CustomScrollView::slivers`._
///
/// [`SliverFillViewport`] places its children in a linear array along the main axis. Each child
/// is sized to fill the viewport, both in the main and cross axis.
///
/// See also:
///
///  * [`crate::SliverFixedExtentList`], which has a configurable
///    [`item_extent`](crate::SliverFixedExtentList::item_extent).
///  * `SliverPrototypeExtentList`, which is similar to [`crate::SliverFixedExtentList`] except
///    that it uses a prototype list item instead of a pixel value to define the main axis extent
///    of each item.
///  * [`crate::SliverList`], which does not require its children to have the same extent in the
///    main axis.
#[derive(Debug)]
pub struct SliverFillViewport {
    pub key: Option<KeyRef>,

    /// The fraction of the viewport that each child should fill in the main axis.
    ///
    /// If this fraction is less than 1.0, more than one child will be visible at once. If this
    /// fraction is greater than 1.0, each child will be larger than the viewport in the main
    /// axis.
    pub viewport_fraction: f64,

    /// Whether to add padding to both ends of the list.
    ///
    /// If this is set to true and [`viewport_fraction`](Self::viewport_fraction) < 1.0, padding
    /// will be added such that the first and last child slivers will be in the center of the
    /// viewport when scrolled all the way to the start or end, respectively. You may want to set
    /// this to false if this [`SliverFillViewport`] is not the only widget along this main axis,
    /// such as in a `CustomScrollView` with multiple children.
    ///
    /// If [`viewport_fraction`](Self::viewport_fraction) is greater than one, this option has no
    /// effect. Defaults to true.
    pub pad_ends: bool,

    /// The delegate that provides the children for this widget.
    pub delegate: SliverChildDelegateRef,

    /// Whether the pages answer implicit scrolling, as `PageView::allow_implicit_scrolling`
    /// describes.
    pub allow_implicit_scrolling: bool,
}

impl SliverFillViewport {
    /// Creates a sliver whose box children that each fill the viewport.
    pub fn new(delegate: SliverChildDelegateRef) -> SliverFillViewport {
        SliverFillViewport {
            key: None,
            viewport_fraction: 1.0,
            pad_ends: true,
            delegate,
            allow_implicit_scrolling: true,
        }
    }

    /// Dart `SliverFillViewport(key:)`.
    pub fn key(mut self, key: KeyRef) -> SliverFillViewport {
        self.key = Some(key);
        self
    }

    /// Dart `SliverFillViewport(viewportFraction:)`.
    pub fn viewport_fraction(mut self, viewport_fraction: f64) -> SliverFillViewport {
        debug_assert!(viewport_fraction > 0.0);
        self.viewport_fraction = viewport_fraction;
        self
    }

    /// Dart `SliverFillViewport(padEnds:)`.
    pub fn pad_ends(mut self, pad_ends: bool) -> SliverFillViewport {
        self.pad_ends = pad_ends;
        self
    }

    /// Dart `SliverFillViewport(allowImplicitScrolling:)`.
    pub fn allow_implicit_scrolling(
        mut self,
        allow_implicit_scrolling: bool,
    ) -> SliverFillViewport {
        self.allow_implicit_scrolling = allow_implicit_scrolling;
        self
    }
}

impl StatelessWidget for SliverFillViewport {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let viewport_fraction = if self.pad_ends {
            clamp_double(1.0 - self.viewport_fraction, 0.0, 1.0) / 2.0
        } else {
            0.0
        };
        SliverFractionalPadding::new(viewport_fraction)
            .sliver(SliverFillViewportRenderObjectWidget::new(
                Rc::clone(&self.delegate),
                self.viewport_fraction,
                self.allow_implicit_scrolling,
            ))
            .into_widget()
    }
}

/// Dart's `_SliverFillViewportRenderObjectWidget`.
#[derive(Debug)]
struct SliverFillViewportRenderObjectWidget {
    delegate: SliverChildDelegateRef,
    viewport_fraction: f64,
    allow_implicit_scrolling: bool,
}

impl SliverFillViewportRenderObjectWidget {
    fn new(
        delegate: SliverChildDelegateRef,
        viewport_fraction: f64,
        allow_implicit_scrolling: bool,
    ) -> SliverFillViewportRenderObjectWidget {
        debug_assert!(viewport_fraction > 0.0);
        SliverFillViewportRenderObjectWidget {
            delegate,
            viewport_fraction,
            allow_implicit_scrolling,
        }
    }
}

impl RenderObjectWidget for SliverFillViewportRenderObjectWidget {
    type RenderObject = RenderSliverFillViewport;

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let child_manager = child_manager_of::<SliverFillViewportRenderObjectWidget>(app, context);
        RenderSliverFillViewport::new(
            app,
            child_manager,
            self.viewport_fraction,
            self.allow_implicit_scrolling,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderSliverFillViewport>,
    ) {
        render_object.set_viewport_fraction(app, self.viewport_fraction);
        render_object.set_allow_implicit_scrolling(app, self.allow_implicit_scrolling);
    }
}

impl SliverWithKeepAliveWidget for SliverFillViewportRenderObjectWidget {}

impl SliverMultiBoxAdaptorWidget for SliverFillViewportRenderObjectWidget {
    fn delegate(&self) -> &SliverChildDelegateRef {
        &self.delegate
    }
}

/// Dart's `_SliverFractionalPadding`.
#[derive(Debug)]
struct SliverFractionalPadding {
    viewport_fraction: f64,
    sliver: Option<WidgetRef>,
}

impl SliverFractionalPadding {
    fn new(viewport_fraction: f64) -> SliverFractionalPadding {
        debug_assert!(viewport_fraction >= 0.0);
        debug_assert!(viewport_fraction <= 0.5);
        SliverFractionalPadding {
            viewport_fraction,
            sliver: None,
        }
    }

    fn sliver<K>(mut self, sliver: impl IntoWidget<K>) -> SliverFractionalPadding {
        self.sliver = Some(sliver.into_widget());
        self
    }
}

impl RenderObjectWidget for SliverFractionalPadding {
    type RenderObject = RenderSliverFractionalPadding;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderSliverFractionalPadding::new(app, self.viewport_fraction).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderSliverFractionalPadding>,
    ) {
        render_object.set_viewport_fraction(app, self.viewport_fraction);
    }
}

impl SingleChildRenderObjectWidget for SliverFractionalPadding {
    fn child(&self) -> Option<&WidgetRef> {
        self.sliver.as_ref()
    }
}

/// Dart's `_RenderSliverFractionalPadding`: pads its sliver on both ends of the main axis by
/// [`viewport_fraction`](Self::viewport_fraction) of the viewport's main axis extent.
struct RenderSliverFractionalPadding {
    render_object: RenderObjectData,
    render_sliver: RenderSliverData,
    child: RenderObjectWithChildData<AnyRenderSliver>,
    last_resolved_constraints: Option<SliverConstraints>,
    viewport_fraction: f64,
    resolved_padding: Option<EdgeInsets>,
}

impl RenderSliverFractionalPadding {
    fn new(app: &mut App, viewport_fraction: f64) -> RenderHandle<RenderSliverFractionalPadding> {
        debug_assert!(viewport_fraction <= 0.5);
        debug_assert!(viewport_fraction >= 0.0);
        RenderHandle::new_sliver(
            app,
            RenderSliverFractionalPadding {
                render_object: RenderObjectData::new(),
                render_sliver: RenderSliverData::new(),
                child: RenderObjectWithChildData::new(),
                last_resolved_constraints: None,
                viewport_fraction,
                resolved_padding: None,
            },
        )
    }

    fn set_viewport_fraction(self: RenderHandle<Self>, app: &mut App, new_value: f64) {
        if self.get(app).viewport_fraction == new_value {
            return;
        }
        self.get_mut(app).viewport_fraction = new_value;
        self.mark_needs_resolution(app);
    }

    fn mark_needs_resolution(self: RenderHandle<Self>, app: &mut App) {
        self.get_mut(app).resolved_padding = None;
        self.mark_needs_layout(app);
    }

    fn resolve(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let this = self.get(app);
        if this.resolved_padding.is_some() && this.last_resolved_constraints == Some(constraints) {
            return;
        }

        let padding_value = constraints.viewport_main_axis_extent * this.viewport_fraction;
        let resolved_padding = match constraints.axis() {
            Axis::Horizontal => EdgeInsets::symmetric(0.0, padding_value),
            Axis::Vertical => EdgeInsets::symmetric(padding_value, 0.0),
        };
        let this = self.get_mut(app);
        this.last_resolved_constraints = Some(constraints);
        this.resolved_padding = Some(resolved_padding);
    }
}

impl Debug for RenderSliverFractionalPadding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_RenderSliverFractionalPadding")
            .field("viewport_fraction", &self.viewport_fraction)
            .finish()
    }
}

impl RenderObjectWithChildMixin for RenderSliverFractionalPadding {
    type ChildType = AnyRenderSliver;

    fn child_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &RenderObjectWithChildData<AnyRenderSliver> {
        &self.get(app).child
    }

    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<AnyRenderSliver> {
        &mut self.get_mut(app).child
    }
}

impl RenderSliverEdgeInsetsPadding for RenderSliverFractionalPadding {
    fn resolved_padding(self: RenderHandle<Self>, app: &App) -> Option<EdgeInsets> {
        self.get(app).resolved_padding
    }
}

impl RenderObject for RenderSliverFractionalPadding {
    waft_rendering::render_object_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderSliverEdgeInsetsPadding::setup_parent_data(self, app, child)
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

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderSliverEdgeInsetsPadding::apply_paint_transform(self, app, child, transform)
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        self.resolve(app);
        RenderSliverEdgeInsetsPadding::perform_layout(self, app)
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderSliverEdgeInsetsPadding::paint(self, app, context, offset)
    }
}

impl RenderSliver for RenderSliverFractionalPadding {
    waft_rendering::render_sliver_accessors!();

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut SliverHitTestResult<'_>,
        main_axis_position: f64,
        cross_axis_position: f64,
    ) -> bool {
        RenderSliverEdgeInsetsPadding::hit_test_children(
            self,
            app,
            result,
            main_axis_position,
            cross_axis_position,
        )
    }

    fn child_main_axis_position(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
    ) -> f64 {
        RenderSliverEdgeInsetsPadding::child_main_axis_position(self, app, child)
    }

    fn child_cross_axis_position(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
    ) -> f64 {
        RenderSliverEdgeInsetsPadding::child_cross_axis_position(self, app, child)
    }

    fn child_scroll_offset(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
    ) -> Option<f64> {
        RenderSliverEdgeInsetsPadding::child_scroll_offset(self, app, child)
    }
}

/// A sliver that contains a single box child that fills the remaining space in the viewport.
///
/// _To learn more about slivers, see `CustomScrollView::slivers`._
///
/// [`SliverFillRemaining`] will size its [`child`](Self::child) to fill the viewport in the cross
/// axis. The extent of the sliver and its child's size in the main axis is computed
/// conditionally, described in further detail below.
///
/// Typically this will be the last sliver in a viewport, since (by definition) there is never
/// any room for anything beyond this sliver.
///
/// ## Main Axis Extent
///
/// ### When [`SliverFillRemaining`] has a scrollable child
///
/// The [`has_scroll_body`](Self::has_scroll_body) flag indicates whether the sliver's child has a
/// scrollable body. This value is never null, and defaults to true. A common example of this use
/// is a `NestedScrollView`. In this case, the sliver will size its child to fill the maximum
/// available extent. [`SliverFillRemaining`] will not constrain the scrollable area, as it could
/// potentially have an infinite depth. This is also true for use cases such as a `ScrollView`
/// when `ScrollView::shrink_wrap` is true.
///
/// ### When [`SliverFillRemaining`] does not have a scrollable child
///
/// When [`has_scroll_body`](Self::has_scroll_body) is set to false, the child's size is taken
/// into account when considering the extent to which it should fill the space. The extent to
/// which the preceding slivers have been scrolled is also taken into account in deciding how to
/// layout this sliver.
///
/// [`SliverFillRemaining`] will size its [`child`](Self::child) to fill the viewport in the main
/// axis if that space is larger than the child's extent, and the amount of space that has been
/// scrolled beforehand has not exceeded the main axis extent of the viewport.
///
/// [`SliverFillRemaining`] will defer to the size of its [`child`](Self::child) if the child's
/// size exceeds the remaining space in the viewport.
///
/// [`SliverFillRemaining`] will defer to the size of its [`child`](Self::child) if the
/// `SliverConstraints::preceding_scroll_extent` exceeded the length of the viewport's main axis.
///
/// For `ScrollPhysics` that allow overscroll, such as `BouncingScrollPhysics`, setting the
/// [`fill_overscroll`](Self::fill_overscroll) flag to true allows the size of the
/// [`child`](Self::child) to _stretch_, filling the overscroll area. It does this regardless of
/// the path chosen to provide the child's size.
///
/// See also:
///
///  * [`SliverFillViewport`], which sizes its children based on the size of the viewport,
///    regardless of what else is in the scroll view.
///  * [`crate::SliverList`], which shows a list of variable-sized children in a viewport.
#[derive(Debug)]
pub struct SliverFillRemaining {
    pub key: Option<KeyRef>,

    /// Box child widget that fills the remaining space in the viewport.
    ///
    /// The main [`SliverFillRemaining`] documentation contains more details.
    pub child: Option<WidgetRef>,

    /// Indicates whether the child has a scrollable body, this value cannot be null.
    ///
    /// Defaults to true such that the child will extend beyond the viewport and scroll, as seen
    /// in `NestedScrollView`.
    ///
    /// Setting this value to false will allow the child to fill the remainder of the viewport
    /// and not extend further. However, if the `SliverConstraints::preceding_scroll_extent`
    /// and/or the [`child`](Self::child)'s extent exceeds the size of the viewport, the sliver
    /// will defer to the child's size rather than overriding it.
    pub has_scroll_body: bool,

    /// Indicates whether the child should stretch to fill the overscroll area created by certain
    /// scroll physics, such as iOS' default scroll physics. This flag is only relevant when
    /// [`has_scroll_body`](Self::has_scroll_body) is false.
    ///
    /// Defaults to false, meaning that the default behavior is for the child to maintain its
    /// size and not extend into the overscroll area.
    pub fill_overscroll: bool,
}

impl Default for SliverFillRemaining {
    fn default() -> SliverFillRemaining {
        SliverFillRemaining {
            key: None,
            child: None,
            has_scroll_body: true,
            fill_overscroll: false,
        }
    }
}

impl SliverFillRemaining {
    /// Creates a sliver that fills the remaining space in the viewport.
    pub fn new() -> SliverFillRemaining {
        SliverFillRemaining::default()
    }

    /// Dart `SliverFillRemaining(key:)`.
    pub fn key(mut self, key: KeyRef) -> SliverFillRemaining {
        self.key = Some(key);
        self
    }

    /// Dart `SliverFillRemaining(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> SliverFillRemaining {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `SliverFillRemaining(hasScrollBody:)`.
    pub fn has_scroll_body(mut self, has_scroll_body: bool) -> SliverFillRemaining {
        self.has_scroll_body = has_scroll_body;
        self
    }

    /// Dart `SliverFillRemaining(fillOverscroll:)`.
    pub fn fill_overscroll(mut self, fill_overscroll: bool) -> SliverFillRemaining {
        self.fill_overscroll = fill_overscroll;
        self
    }
}

impl StatelessWidget for SliverFillRemaining {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let child = self.child.clone();
        if self.has_scroll_body {
            return SliverFillRemainingWithScrollable { child }.into_widget();
        }
        if !self.fill_overscroll {
            return SliverFillRemainingWithoutScrollable { child }.into_widget();
        }
        SliverFillRemainingAndOverscroll { child }.into_widget()
    }
}

/// Declares one of [`SliverFillRemaining`]'s private single-child widgets over its render
/// sliver.
macro_rules! sliver_fill_remaining_widget {
    ($(#[$doc:meta])* $widget:ident => $render_object:ident) => {
        $(#[$doc])*
        #[derive(Debug)]
        struct $widget {
            child: Option<WidgetRef>,
        }

        impl RenderObjectWidget for $widget {
            type RenderObject = $render_object;

            fn create_render_object(
                &self,
                app: &mut App,
                _context: BuildContext,
            ) -> AnyRenderObject {
                $render_object::new(app, None).as_object()
            }
        }

        impl SingleChildRenderObjectWidget for $widget {
            fn child(&self) -> Option<&WidgetRef> {
                self.child.as_ref()
            }
        }
    };
}

sliver_fill_remaining_widget!(
    /// Dart's `_SliverFillRemainingWithScrollable`.
    SliverFillRemainingWithScrollable => RenderSliverFillRemainingWithScrollable
);

sliver_fill_remaining_widget!(
    /// Dart's `_SliverFillRemainingWithoutScrollable`.
    SliverFillRemainingWithoutScrollable => RenderSliverFillRemaining
);

sliver_fill_remaining_widget!(
    /// Dart's `_SliverFillRemainingAndOverscroll`.
    SliverFillRemainingAndOverscroll => RenderSliverFillRemainingAndOverscroll
);

#[cfg(test)]
mod tests {
    use waft_foundation::Ref;

    use waft_embedder::{Color, Offset, Rect, Size, TargetPlatform, TextDirection};
    use waft_foundation::{Handle, Listener};
    use waft_painting::{AlignmentGeometry, AxisDirection, EdgeInsetsGeometry};
    use waft_rendering::{
        AnyRenderSliver, BoxConstraints, ContainerRenderObjectMixin, GrowthDirection, MainAxisSize,
        RenderSliverFixedExtentList, RenderSliverMultiBoxAdaptor, ScrollCacheExtent,
        ScrollDirection, SliverMultiBoxAdaptorParentData,
    };

    use super::*;
    use crate::framework::GlobalKey;
    use crate::testing::{
        TargetPlatformVariant, WidgetTester, expect, find, finds_nothing, finds_one_widget,
        test_widgets, test_widgets_variant,
    };
    use crate::widgets::basic::{
        Align, Center, ColoredBox, Column, Directionality, Flexible, Padding, SizedBox,
        SliverToBoxAdapter,
    };
    use crate::widgets::gesture_detector::GestureDetector;
    use crate::widgets::scroll_controller::{ScrollController, ScrollControllerLeaf};
    use crate::widgets::scroll_delegate::SliverChildListDelegate;
    use crate::widgets::scroll_view::CustomScrollView;
    use crate::widgets::scrollable::Scrollable;
    use crate::widgets::sliver::SliverFixedExtentList;
    use crate::widgets::text::Text;

    const DEBUG_RED: Color = Color::new(0xFFFF0000);

    fn ltr<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Directionality::new(TextDirection::Ltr, child).into_widget()
    }

    // ---- sliver_fill_viewport_test.dart ----

    /// The index of every child the first `T` has laid out whose extent overlaps its paint
    /// region, as `SliverMultiBoxAdaptorElement.debugVisitOnstageChildren` tests.
    fn onstage_children_of<T: RenderSliverMultiBoxAdaptor>(tester: &WidgetTester) -> Vec<i32> {
        let fill = *tester
            .all::<T>()
            .first()
            .expect("a mounted sliver of the type");
        let app = tester.cell.borrow();
        let constraints = fill.constraints(&app);
        let mut onstage = Vec::new();
        let mut child = fill.first_child(&app);
        while let Some(current) = child {
            let layout_offset = current
                .as_object()
                .parent_data_of::<SliverMultiBoxAdaptorParentData>(&app)
                .layout_offset()
                .expect("a laid out child");
            let extent = current.size(&app).height();
            if layout_offset < constraints.scroll_offset + constraints.remaining_paint_extent
                && layout_offset + extent > constraints.scroll_offset
            {
                onstage.push(fill.index_of(&app, current));
            }
            child = fill.child_after(&app, current);
        }
        onstage
    }

    /// The sliver the first `SliverFillViewport` builds: Dart's
    /// `tester.renderObject<RenderSliver>(find.byType(SliverFillViewport))`.
    fn fill_viewport_sliver(tester: &WidgetTester) -> AnyRenderSliver {
        tester
            .render_object(find::by_type::<SliverFillViewport>())
            .as_sliver()
            .expect("the fill viewport builds a sliver")
    }

    /// Dart's `tester.renderObject<RenderBox>(find.byType(ColoredBox).first).size.height`.
    fn first_colored_box_height(tester: &WidgetTester) -> f64 {
        tester
            .get_size(find::by_type::<ColoredBox>().first())
            .height()
    }

    fn fill_viewport_children(count: usize) -> SliverChildDelegateRef {
        SliverChildListDelegate::new(
            (0..count)
                .map(|i| {
                    ColoredBox::new(Color::new(0xff00ff00))
                        .child(Text::new(format!("{i}")).text_direction(TextDirection::Ltr))
                        .into_widget()
                })
                .collect(),
        )
        .add_automatic_keep_alives(false)
        .add_semantic_indexes(false)
        .into_delegate()
    }

    #[test]
    fn a_sliver_fill_viewport_sizes_each_child_to_the_viewport() {
        test_widgets(|tester| {
            tester.pump_widget(ltr(CustomScrollView::new().slivers([
                SliverFillViewport::new(fill_viewport_children(20)).into_widget(),
            ])));

            assert_eq!(first_colored_box_height(tester), 600.0);

            expect(tester, &find::text("0"), finds_one_widget());
            expect(tester, &find::text("1"), finds_nothing());
            expect(tester, &find::text("2"), finds_nothing());

            tester.drag(find::by_type::<Scrollable>(), Offset::new(0.0, -700.0));
            tester.pump();

            expect(tester, &find::text("0"), finds_nothing());
            expect(tester, &find::text("1"), finds_one_widget());
            expect(tester, &find::text("2"), finds_one_widget());
            expect(tester, &find::text("3"), finds_nothing());
            expect(tester, &find::text("4"), finds_nothing());

            tester.drag(find::by_type::<Scrollable>(), Offset::new(0.0, 200.0));
            tester.pump();

            expect(tester, &find::text("0"), finds_one_widget());
            expect(tester, &find::text("1"), finds_one_widget());
            expect(tester, &find::text("2"), finds_nothing());
            expect(tester, &find::text("3"), finds_nothing());

            tester.drag(find::by_type::<Scrollable>(), Offset::new(0.0, 700.0));
            tester.pump();

            assert_eq!(first_colored_box_height(tester), 600.0);

            expect(tester, &find::text("0"), finds_one_widget());
            expect(tester, &find::text("1"), finds_nothing());
            expect(tester, &find::text("2"), finds_nothing());
            expect(tester, &find::text("3"), finds_nothing());

            let padding = fill_viewport_sliver(tester);
            let fill = *tester
                .all::<RenderSliverFillViewport>()
                .first()
                .expect("a mounted sliver fill viewport");
            let app = tester.cell.borrow();
            // The padding and the fill viewport inside it get the same constraints and geometry.
            for (constraints, geometry) in [
                (padding.constraints(&app), padding.geometry(&app)),
                (fill.constraints(&app), fill.geometry(&app)),
            ] {
                assert_eq!(constraints.axis_direction, AxisDirection::Down);
                assert_eq!(constraints.growth_direction, GrowthDirection::Forward);
                assert_eq!(constraints.user_scroll_direction, ScrollDirection::Idle);
                assert_eq!(constraints.scroll_offset, 0.0);
                assert_eq!(constraints.preceding_scroll_extent, 0.0);
                assert_eq!(constraints.remaining_paint_extent, 600.0);
                assert_eq!(constraints.cross_axis_extent, 800.0);
                assert_eq!(constraints.cross_axis_direction, AxisDirection::Right);
                assert_eq!(constraints.viewport_main_axis_extent, 600.0);
                assert_eq!(constraints.remaining_cache_extent, 850.0);
                assert_eq!(constraints.cache_origin, 0.0);

                assert_eq!(geometry.scroll_extent, 12000.0);
                assert_eq!(geometry.paint_extent, 600.0);
                assert_eq!(geometry.max_paint_extent, 12000.0);
                assert!(geometry.has_visual_overflow);
                assert_eq!(geometry.cache_extent, 850.0);
            }

            // Currently live children: 0 to 1.
            let mut live = Vec::new();
            let mut child = fill.first_child(&app);
            while let Some(current) = child {
                live.push((
                    fill.index_of(&app, current),
                    current
                        .as_object()
                        .parent_data_of::<SliverMultiBoxAdaptorParentData>(&app)
                        .layout_offset(),
                    current.constraints(&app),
                    current.size(&app),
                ));
                child = fill.child_after(&app, current);
            }
            let page = Size::new(800.0, 600.0);
            assert_eq!(
                live,
                [
                    (0, Some(0.0), BoxConstraints::tight(page), page),
                    (1, Some(600.0), BoxConstraints::tight(page), page),
                ]
            );
        });
    }

    #[test]
    fn a_sliver_fill_viewport_pads_its_ends_by_its_viewport_fraction() {
        test_widgets(|tester| {
            let delegate = SliverChildListDelegate::new(vec![SizedBox::new().into_widget()])
                .add_automatic_keep_alives(false)
                .add_semantic_indexes(false)
                .into_delegate();

            tester.pump_widget(ltr(CustomScrollView::new().slivers([
                SliverFillViewport::new(Rc::clone(&delegate))
                    .viewport_fraction(0.5)
                    .into_widget(),
            ])));

            let box_with_padding = fill_viewport_sliver(tester);
            assert_eq!(
                box_with_padding
                    .geometry(&tester.cell.borrow())
                    .paint_extent,
                600.0
            );

            tester.pump_widget(ltr(CustomScrollView::new().slivers([
                SliverFillViewport::new(delegate)
                    .pad_ends(false)
                    .viewport_fraction(0.5)
                    .into_widget(),
            ])));

            let box_without_padding = fill_viewport_sliver(tester);
            assert_eq!(
                box_without_padding
                    .geometry(&tester.cell.borrow())
                    .paint_extent,
                300.0
            );
        });
    }

    // ---- sliver_fill_remaining_test.dart ----

    /// The test file's `sliverBox`: a 150 x 150 box.
    fn sliver_box() -> WidgetRef {
        SliverToBoxAdapter::new()
            .child(
                SizedBox::new()
                    .width(150.0)
                    .height(150.0)
                    .child(ColoredBox::new(DEBUG_RED)),
            )
            .into_widget()
    }

    /// The test file's `boilerplate`.
    fn boilerplate(
        slivers: Vec<WidgetRef>,
        controller: Option<Ref<ScrollController>>,
        scroll_direction: Axis,
    ) -> WidgetRef {
        let mut scroll_view = CustomScrollView::new()
            .scroll_direction(scroll_direction)
            .slivers(slivers);
        if let Some(controller) = controller {
            scroll_view = scroll_view.controller(controller.erase());
        }
        ltr(scroll_view)
    }

    fn jump_to(tester: &mut WidgetTester, controller: Handle<ScrollController>, value: f64) {
        controller.jump_to(&mut tester.cell.borrow_mut(), value);
        tester.pump();
    }

    fn offset(tester: &WidgetTester, controller: Handle<ScrollController>) -> f64 {
        controller.offset(&tester.cell.borrow())
    }

    #[test]
    fn a_scroll_body_with_no_siblings_fills_the_viewport() {
        test_widgets(|tester| {
            let controller = ScrollController::default(&mut tester.cell.borrow_mut());
            let key = GlobalKey::new();
            tester.pump_widget(boilerplate(
                vec![
                    SliverFillRemaining::new()
                        .child(SizedBox::new().key(key.clone().into()))
                        .into_widget(),
                ],
                Some(controller.clone()),
                Axis::Vertical,
            ));
            assert_eq!(tester.get_size(&key).height(), 600.0);

            for pixels in [50.0, -100.0, 0.0] {
                jump_to(tester, controller.handle(), pixels);
                assert_eq!(tester.get_size(&key).height(), 600.0, "at {pixels}");
            }
        });
    }

    #[test]
    fn a_scroll_body_with_one_sibling_fills_what_remains() {
        test_widgets(|tester| {
            let controller = ScrollController::default(&mut tester.cell.borrow_mut());
            let key = GlobalKey::new();
            tester.pump_widget(boilerplate(
                vec![
                    SliverToBoxAdapter::new()
                        .child(SizedBox::new().height(100.0))
                        .into_widget(),
                    SliverFillRemaining::new()
                        .child(SizedBox::new().key(key.clone().into()))
                        .into_widget(),
                ],
                Some(controller.clone()),
                Axis::Vertical,
            ));
            assert_eq!(tester.get_size(&key).height(), 500.0);

            for (pixels, height) in [(50.0, 550.0), (-100.0, 400.0), (0.0, 500.0)] {
                jump_to(tester, controller.handle(), pixels);
                assert_eq!(tester.get_size(&key).height(), height, "at {pixels}");
            }
        });
    }

    #[test]
    fn a_scroll_body_scrolls_beyond_the_viewport_extent() {
        test_widgets(|tester| {
            let controller = ScrollController::default(&mut tester.cell.borrow_mut());
            let fill_key = GlobalKey::new();
            tester.pump_widget(boilerplate(
                vec![
                    sliver_box(),
                    SliverFillRemaining::new()
                        .child(ColoredBox::new(DEBUG_RED).key(fill_key.clone().into()))
                        .into_widget(),
                ],
                Some(controller.clone()),
                Axis::Vertical,
            ));
            assert_eq!(offset(tester, controller.handle()), 0.0);
            controller.jump_to(&mut tester.cell.borrow_mut(), 150.0);
            tester.pump_and_settle();
            assert_eq!(offset(tester, controller.handle()), 150.0);
            assert_eq!(tester.get_rect(&fill_key).top, 0.0);
        });
    }

    /// `has correct semantics when`: past the viewport, a scroll body inside the cache extent is
    /// laid out at the size it asks for, and outside it at none. Accessibility is deferred, so
    /// this checks the layout the semantic nodes follow.
    #[test]
    fn a_scroll_body_past_the_viewport_is_laid_out_only_within_the_cache_extent() {
        // Viewport is 800x600
        const VIEWPORT_HEIGHT: f64 = 600.0;
        const CACHE_EXTENT: f64 = 250.0;
        for (preceding_height, height) in [
            // This sliver takes up entire viewport and leaves 250 remaining cacheExtent
            (VIEWPORT_HEIGHT, 100.0),
            // This sliver takes up entire viewport and leaves 0 remaining cacheExtent
            (VIEWPORT_HEIGHT + CACHE_EXTENT, 0.0),
        ] {
            test_widgets(|tester| {
                let key = GlobalKey::new();
                tester.pump_widget(ltr(CustomScrollView::new()
                    .scroll_cache_extent(ScrollCacheExtent::Pixels(CACHE_EXTENT))
                    .slivers([
                        SliverToBoxAdapter::new()
                            .child(
                                SizedBox::new()
                                    .width(150.0)
                                    .height(preceding_height)
                                    .child(ColoredBox::new(DEBUG_RED)),
                            )
                            .into_widget(),
                        SliverFillRemaining::new()
                            .child(SizedBox::new().key(key.clone().into()).height(100.0))
                            .into_widget(),
                    ])));
                assert_eq!(
                    tester.get_size(&key).height(),
                    height,
                    "after {preceding_height}"
                );
            });
        }
    }

    fn without_scroll_body(child: WidgetRef) -> Vec<WidgetRef> {
        vec![
            sliver_box(),
            SliverFillRemaining::new()
                .has_scroll_body(false)
                .child(child)
                .into_widget(),
        ]
    }

    #[test]
    fn a_body_that_does_not_scroll_does_not_extend_past_the_viewport() {
        test_widgets(|tester| {
            let controller = ScrollController::default(&mut tester.cell.borrow_mut());
            tester.pump_widget(boilerplate(
                without_scroll_body(ColoredBox::new(DEBUG_RED).into_widget()),
                Some(controller.clone()),
                Axis::Vertical,
            ));
            assert_eq!(offset(tester, controller.handle()), 0.0);
            controller.jump_to(&mut tester.cell.borrow_mut(), 150.0);
            tester.pump_and_settle();
            assert_eq!(offset(tester, controller.handle()), 0.0);
        });
    }

    #[test]
    fn a_child_without_a_size_is_sized_by_the_extent() {
        test_widgets(|tester| {
            let fill_key = GlobalKey::new();
            let slivers = || {
                without_scroll_body(
                    ColoredBox::new(DEBUG_RED)
                        .key(fill_key.clone().into())
                        .into_widget(),
                )
            };

            tester.pump_widget(boilerplate(slivers(), None, Axis::Vertical));
            assert_eq!(tester.get_size(&fill_key).height(), 450.0);

            tester.pump_widget(boilerplate(slivers(), None, Axis::Horizontal));
            assert_eq!(tester.get_size(&fill_key).width(), 650.0);
        });
    }

    #[test]
    fn the_extent_is_overridden_by_a_larger_child() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let slivers = || {
                without_scroll_body(
                    SizedBox::new()
                        .key(key.clone().into())
                        .width(1000.0)
                        .height(600.0)
                        .child(ColoredBox::new(DEBUG_RED))
                        .into_widget(),
                )
            };

            tester.pump_widget(boilerplate(slivers(), None, Axis::Vertical));
            assert_eq!(tester.get_size(&key).height(), 600.0);

            tester.pump_widget(boilerplate(slivers(), None, Axis::Horizontal));
            assert_eq!(tester.get_size(&key).width(), 1000.0);
        });
    }

    /// The test file's `_sizedTestButton`: a box the size of Material's default elevated button
    /// at the platform's visual density, `TestButton` itself being only a tap target.
    fn sized_test_button(platform: TargetPlatform, key: &GlobalKey) -> WidgetRef {
        let density_vertical = match platform {
            TargetPlatform::Android | TargetPlatform::IOS | TargetPlatform::Fuchsia => 0.0,
            TargetPlatform::Linux | TargetPlatform::MacOS | TargetPlatform::Windows => -2.0,
        };
        SizedBox::new()
            .key(key.clone().into())
            .width(116.0)
            .height(48.0 + density_vertical * 4.0)
            .child(ColoredBox::new(DEBUG_RED))
            .into_widget()
    }

    /// A button pinned to the bottom center of a keyed box.
    fn bottom_center_button(
        platform: TargetPlatform,
        key: &GlobalKey,
        button: &GlobalKey,
    ) -> WidgetRef {
        ColoredBox::new(DEBUG_RED)
            .key(key.clone().into())
            .child(
                Align::new()
                    .alignment(AlignmentGeometry::BOTTOM_CENTER)
                    .child(sized_test_button(platform, button)),
            )
            .into_widget()
    }

    /// A button centered in 50 of padding in a keyed box: 148 of intrinsic height on Android.
    fn padded_center_button(
        platform: TargetPlatform,
        key: &GlobalKey,
        button: &GlobalKey,
    ) -> WidgetRef {
        ColoredBox::new(DEBUG_RED)
            .key(key.clone().into())
            .child(
                Align::new().child(
                    Padding::new(EdgeInsetsGeometry::all(50.0))
                        .child(sized_test_button(platform, button)),
                ),
            )
            .into_widget()
    }

    fn five_fixed_extent_items() -> WidgetRef {
        SliverFixedExtentList::builder(
            |_, _, _| Some(ColoredBox::new(DEBUG_RED).into_widget()),
            150.0,
        )
        .item_count(5)
        .into_widget()
    }

    fn fill_without_scroll_body(fill_overscroll: bool, child: WidgetRef) -> WidgetRef {
        SliverFillRemaining::new()
            .has_scroll_body(false)
            .fill_overscroll(fill_overscroll)
            .child(child)
            .into_widget()
    }

    fn jump_to_max_scroll_extent(tester: &mut WidgetTester, controller: Handle<ScrollController>) {
        let max_scroll_extent = {
            let app = tester.cell.borrow();
            controller.position(&app).max_scroll_extent(&app)
        };
        jump_to(tester, controller, max_scroll_extent);
    }

    #[test]
    fn a_child_with_a_smaller_size_is_sized_by_the_extent() {
        test_widgets(|tester| {
            let (key, button) = (GlobalKey::new(), GlobalKey::new());
            let slivers = || {
                without_scroll_body(bottom_center_button(TargetPlatform::Android, &key, &button))
            };

            tester.pump_widget(boilerplate(slivers(), None, Axis::Vertical));
            assert_eq!(tester.get_size(&key).height(), 450.0);

            // Also check that the button alignment is true to expectations
            let button_rect = tester.get_rect(&button);
            assert_eq!(button_rect.bottom, 600.0);
            assert_eq!(button_rect.center().dx(), 400.0);

            // Check Axis.horizontal
            tester.pump_widget(boilerplate(slivers(), None, Axis::Horizontal));
            assert_eq!(tester.get_size(&key).width(), 650.0);
        });
    }

    #[test]
    fn the_extent_is_the_childs_once_the_preceding_slivers_pass_the_viewport() {
        test_widgets(|tester| {
            let (key, button) = (GlobalKey::new(), GlobalKey::new());
            tester.pump_widget(boilerplate(
                vec![
                    five_fixed_extent_items(),
                    fill_without_scroll_body(
                        false,
                        padded_center_button(TargetPlatform::Android, &key, &button),
                    ),
                ],
                None,
                Axis::Vertical,
            ));
            tester.drag(find::by_type::<Scrollable>(), Offset::new(0.0, -750.0));
            tester.pump();
            assert_eq!(tester.get_size(&key).height(), 148.0);

            // Also check that the button alignment is true to expectations
            let button_rect = tester.get_rect(&button);
            assert_eq!(button_rect.bottom, 550.0);
            assert_eq!(button_rect.center().dx(), 400.0);
        });
    }

    /// The column of `alignment with a flexible works`: a flexible, centered 100 x 100 logo above
    /// the sized test button.
    fn flexible_logo_and_button(
        platform: TargetPlatform,
        key: &GlobalKey,
        logo: &GlobalKey,
        button: &GlobalKey,
    ) -> WidgetRef {
        Column::new()
            .key(key.clone().into())
            .main_axis_size(MainAxisSize::Min)
            .children([
                Flexible::new(
                    Center::new().child(
                        SizedBox::new()
                            .key(logo.clone().into())
                            .width(100.0)
                            .height(100.0),
                    ),
                )
                .into_widget(),
                sized_test_button(platform, button),
            ])
            .into_widget()
    }

    /// `alignment with a flexible works`: a column whose flexible logo and button keep their
    /// places as the scroll view overscrolls.
    #[test]
    fn a_flexible_child_keeps_its_alignment() {
        test_widgets_variant(ios_and_macos(), |tester| {
            let platform = tester.target_platform();
            let density_vertical = if platform == TargetPlatform::MacOS {
                -2.0
            } else {
                0.0
            };
            let (key, logo, button) = (GlobalKey::new(), GlobalKey::new(), GlobalKey::new());
            tester.pump_widget(boilerplate(
                without_scroll_body(flexible_logo_and_button(platform, &key, &logo, &button)),
                None,
                Axis::Vertical,
            ));
            assert_eq!(tester.get_size(&key).height(), 450.0, "{platform:?}");

            // Check that the logo alignment is true to expectations
            assert_eq!(
                tester.get_size(&logo),
                Size::new(100.0, 100.0),
                "{platform:?}"
            );
            assert_eq!(
                tester.get_rect(&logo).center(),
                Offset::new(400.0, 351.0 - density_vertical * 2.0),
                "{platform:?}"
            );

            // Also check that the button alignment is true to expectations
            assert_eq!(
                tester.get_size(&button),
                Size::new(116.0, 48.0 + density_vertical * 4.0),
                "{platform:?}"
            );
            let button_rect = tester.get_rect(&button);
            assert_eq!(button_rect.bottom, 600.0, "{platform:?}");
            assert_eq!(button_rect.center().dx(), 400.0, "{platform:?}");

            // Overscroll and see that alignment and size is maintained
            tester.drag(find::by_type::<Scrollable>(), Offset::new(0.0, -50.0));
            tester.pump();
            assert_eq!(tester.get_size(&key).height(), 450.0, "{platform:?}");
            assert_eq!(
                tester.get_size(&logo),
                Size::new(100.0, 100.0),
                "{platform:?}"
            );
            assert!(tester.get_rect(&logo).center().dy() < 351.0, "{platform:?}");
            assert_eq!(
                tester.get_size(&button),
                Size::new(116.0, 48.0 + density_vertical * 4.0),
                "{platform:?}"
            );
            let button_rect = tester.get_rect(&button);
            assert!(button_rect.bottom < 600.0, "{platform:?}");
            assert_eq!(button_rect.center().dx(), 400.0, "{platform:?}");
        });
    }

    /// `alignment with a flexible works with fillOverscroll`: the logo stays centered in the
    /// column as it stretches into the overscroll, and the button stays at the bottom.
    #[test]
    fn a_flexible_child_keeps_its_alignment_as_it_fills_the_overscroll() {
        test_widgets_variant(ios_and_macos(), |tester| {
            let platform = tester.target_platform();
            let density_vertical = if platform == TargetPlatform::MacOS {
                -2.0
            } else {
                0.0
            };
            let (key, logo, button) = (GlobalKey::new(), GlobalKey::new(), GlobalKey::new());
            tester.pump_widget(boilerplate(
                vec![
                    sliver_box(),
                    fill_without_scroll_body(
                        true,
                        flexible_logo_and_button(platform, &key, &logo, &button),
                    ),
                ],
                None,
                Axis::Vertical,
            ));
            let assert_at_rest = |tester: &WidgetTester| {
                assert_eq!(tester.get_size(&key).height(), 450.0, "{platform:?}");

                // Check that the logo alignment is true to expectations.
                assert_eq!(
                    tester.get_size(&logo),
                    Size::new(100.0, 100.0),
                    "{platform:?}"
                );
                assert_eq!(
                    tester.get_rect(&logo).center(),
                    Offset::new(400.0, 351.0 - density_vertical * 2.0),
                    "{platform:?}"
                );

                // Also check that the button alignment is true to expectations.
                assert_eq!(
                    tester.get_size(&button),
                    Size::new(116.0, 48.0 + density_vertical * 4.0),
                    "{platform:?}"
                );
                let button_rect = tester.get_rect(&button);
                assert_eq!(button_rect.bottom, 600.0, "{platform:?}");
                assert_eq!(button_rect.center().dx(), 400.0, "{platform:?}");
            };
            assert_at_rest(tester);

            // Overscroll and see that logo alignment shifts to maintain center as
            // container stretches with overscroll, button remains aligned at the
            // bottom.
            tester.drag(find::by_type::<Scrollable>(), Offset::new(0.0, -50.0));
            tester.pump();
            assert!(tester.get_size(&key).height() > 450.0, "{platform:?}");
            assert_eq!(
                tester.get_size(&logo),
                Size::new(100.0, 100.0),
                "{platform:?}"
            );
            assert!(tester.get_rect(&logo).center().dy() < 351.0, "{platform:?}");
            assert_eq!(
                tester.get_size(&button),
                Size::new(116.0, 48.0 + density_vertical * 4.0),
                "{platform:?}"
            );
            let button_rect = tester.get_rect(&button);
            assert_eq!(button_rect.bottom, 600.0, "{platform:?}");
            assert_eq!(button_rect.center().dx(), 400.0, "{platform:?}");

            // Ensure overscroll retracts to original position when gesture is
            // released.
            tester.pump_and_settle();
            assert_at_rest(tester);
        });
    }

    fn overscrolling_slivers(fill_key: &GlobalKey, with_box: bool) -> Vec<WidgetRef> {
        let fill = SliverFillRemaining::new()
            .has_scroll_body(false)
            .fill_overscroll(true)
            .child(
                SizedBox::expand().child(ColoredBox::new(DEBUG_RED).key(fill_key.clone().into())),
            )
            .into_widget();
        if with_box {
            vec![sliver_box(), fill]
        } else {
            vec![fill]
        }
    }

    /// `TargetPlatformVariant(<TargetPlatform>{TargetPlatform.iOS, TargetPlatform.macOS})`.
    fn ios_and_macos() -> TargetPlatformVariant {
        TargetPlatformVariant::new([TargetPlatform::IOS, TargetPlatform::MacOS])
    }

    #[test]
    fn a_child_without_a_size_is_sized_by_the_extent_and_the_overscroll() {
        test_widgets_variant(ios_and_macos(), |tester| {
            let platform = tester.target_platform();
            let fill_key = GlobalKey::new();
            tester.pump_widget(boilerplate(
                overscrolling_slivers(&fill_key, true),
                None,
                Axis::Vertical,
            ));
            assert_eq!(tester.get_size(&fill_key).height(), 450.0, "{platform:?}");

            // Overscroll and check size
            tester.drag(find::by_type::<Scrollable>(), Offset::new(0.0, -50.0));
            tester.pump();
            assert!(tester.get_size(&fill_key).height() > 450.0, "{platform:?}");

            // Ensure overscroll retracts to original size after releasing gesture
            tester.pump_and_settle();
            assert_eq!(tester.get_size(&fill_key).height(), 450.0, "{platform:?}");
        });
    }

    #[test]
    fn a_child_stretches_into_overscroll_that_exceeds_the_preceding_extent() {
        test_widgets_variant(ios_and_macos(), |tester| {
            let platform = tester.target_platform();
            let controller = ScrollController::default(&mut tester.cell.borrow_mut());
            let fill_key = GlobalKey::new();
            tester.pump_widget(boilerplate(
                overscrolling_slivers(&fill_key, true),
                Some(controller.clone()),
                Axis::Vertical,
            ));
            assert_eq!(
                tester.get_rect(&fill_key),
                Rect::from_ltrb(0.0, 150.0, 800.0, 600.0),
                "{platform:?}"
            );

            // Overscroll by 200: the preceding box frees up 150 of paint room
            // and the remaining 50 become this sliver's scrollOffset.
            jump_to(tester, controller.handle(), 200.0);

            let rect = tester.get_rect(&fill_key);
            assert_eq!(rect.bottom, 600.0, "{platform:?}");
            assert_eq!(rect.height(), 650.0, "{platform:?}");

            jump_to(tester, controller.handle(), 0.0);
            assert_eq!(
                tester.get_rect(&fill_key),
                Rect::from_ltrb(0.0, 150.0, 800.0, 600.0),
                "{platform:?}"
            );
        });
    }

    #[test]
    fn a_lone_child_stretches_to_cover_the_whole_overscroll() {
        test_widgets_variant(ios_and_macos(), |tester| {
            let platform = tester.target_platform();
            let controller = ScrollController::default(&mut tester.cell.borrow_mut());
            let fill_key = GlobalKey::new();
            tester.pump_widget(boilerplate(
                overscrolling_slivers(&fill_key, false),
                Some(controller.clone()),
                Axis::Vertical,
            ));
            assert_eq!(
                tester.get_rect(&fill_key),
                Rect::from_ltrb(0.0, 0.0, 800.0, 600.0),
                "{platform:?}"
            );

            jump_to(tester, controller.handle(), 50.0);

            let rect = tester.get_rect(&fill_key);
            assert_eq!(rect.bottom, 600.0, "{platform:?}");
            assert_eq!(rect.height(), 650.0, "{platform:?}");

            jump_to(tester, controller.handle(), 0.0);
            assert_eq!(
                tester.get_rect(&fill_key),
                Rect::from_ltrb(0.0, 0.0, 800.0, 600.0),
                "{platform:?}"
            );
        });
    }

    #[test]
    fn fill_overscroll_is_ignored_where_scrolling_clamps() {
        test_widgets(|tester| {
            let fill_key = GlobalKey::new();
            tester.pump_widget(boilerplate(
                vec![
                    sliver_box(),
                    SliverFillRemaining::new()
                        .has_scroll_body(false)
                        .fill_overscroll(true)
                        .child(ColoredBox::new(DEBUG_RED).key(fill_key.clone().into()))
                        .into_widget(),
                ],
                None,
                Axis::Vertical,
            ));
            assert_eq!(tester.get_size(&fill_key).height(), 450.0);

            tester.drag(find::by_type::<Scrollable>(), Offset::new(0.0, -50.0));
            tester.pump();
            assert_eq!(tester.get_size(&fill_key).height(), 450.0);
        });
    }
    /// A lone stretching child in a reversed scroll view, with a tap target that answers
    /// whether a hit test at a point reaches it.
    fn reversed_overscrolling_view(
        controller: Ref<ScrollController>,
        scroll_direction: Axis,
        fill_key: &GlobalKey,
        taps: &Rc<std::cell::Cell<u32>>,
    ) -> WidgetRef {
        let taps = Rc::clone(taps);
        ltr(CustomScrollView::new()
            .scroll_direction(scroll_direction)
            .reverse(true)
            .controller(controller.erase())
            .slivers(vec![fill_without_scroll_body(
                true,
                SizedBox::expand()
                    .child(
                        GestureDetector::new()
                            .on_tap(Listener::new(move |_| taps.set(taps.get() + 1)))
                            .child(ColoredBox::new(DEBUG_RED).key(fill_key.clone().into())),
                    )
                    .into_widget(),
            )]))
    }

    #[test]
    fn a_child_in_a_reversed_scroll_view_stretches_towards_the_leading_edge() {
        test_widgets_variant(ios_and_macos(), |tester| {
            let platform = tester.target_platform();
            let controller = ScrollController::default(&mut tester.cell.borrow_mut());
            let fill_key = GlobalKey::new();
            let taps = Rc::new(std::cell::Cell::new(0));
            tester.pump_widget(reversed_overscrolling_view(
                controller.clone(),
                Axis::Vertical,
                &fill_key,
                &taps,
            ));
            assert_eq!(
                tester.get_rect(&fill_key),
                Rect::from_ltrb(0.0, 0.0, 800.0, 600.0),
                "{platform:?}"
            );

            jump_to(tester, controller.handle(), 50.0);

            let rect = tester.get_rect(&fill_key);
            assert_eq!(rect.top, 0.0, "{platform:?}");
            assert_eq!(rect.height(), 650.0, "{platform:?}");
            // `hitTestOnBinding(Offset(400.0, 25.0))` reaches the stretched child.
            tester.tap_at(Offset::new(400.0, 25.0), 1);
            assert_eq!(taps.get(), 1, "{platform:?}");

            jump_to(tester, controller.handle(), 0.0);
            assert_eq!(
                tester.get_rect(&fill_key),
                Rect::from_ltrb(0.0, 0.0, 800.0, 600.0),
                "{platform:?}"
            );
        });
    }

    #[test]
    fn a_child_in_a_reversed_horizontal_scroll_view_stretches_towards_the_leading_edge() {
        test_widgets_variant(ios_and_macos(), |tester| {
            let platform = tester.target_platform();
            let controller = ScrollController::default(&mut tester.cell.borrow_mut());
            let fill_key = GlobalKey::new();
            let taps = Rc::new(std::cell::Cell::new(0));
            tester.pump_widget(reversed_overscrolling_view(
                controller.clone(),
                Axis::Horizontal,
                &fill_key,
                &taps,
            ));
            assert_eq!(
                tester.get_rect(&fill_key),
                Rect::from_ltrb(0.0, 0.0, 800.0, 600.0),
                "{platform:?}"
            );

            jump_to(tester, controller.handle(), 50.0);

            let rect = tester.get_rect(&fill_key);
            assert_eq!(rect.left, 0.0, "{platform:?}");
            assert_eq!(rect.width(), 850.0, "{platform:?}");

            jump_to(tester, controller.handle(), 0.0);
            assert_eq!(
                tester.get_rect(&fill_key),
                Rect::from_ltrb(0.0, 0.0, 800.0, 600.0),
                "{platform:?}"
            );
        });
    }

    #[test]
    fn a_child_with_a_smaller_size_stretches_by_the_extent_and_the_overscroll() {
        test_widgets_variant(ios_and_macos(), |tester| {
            let platform = tester.target_platform();
            let (key, button) = (GlobalKey::new(), GlobalKey::new());
            tester.pump_widget(boilerplate(
                vec![
                    sliver_box(),
                    fill_without_scroll_body(true, bottom_center_button(platform, &key, &button)),
                ],
                None,
                Axis::Vertical,
            ));
            assert_eq!(tester.get_size(&key).height(), 450.0, "{platform:?}");

            tester.drag(find::by_type::<Scrollable>(), Offset::new(0.0, -50.0));
            tester.pump();
            assert!(tester.get_size(&key).height() > 450.0, "{platform:?}");

            // Also check that the button alignment is true to expectations
            let button_rect = tester.get_rect(&button);
            assert_eq!(button_rect.bottom, 600.0, "{platform:?}");
            assert_eq!(button_rect.center().dx(), 400.0, "{platform:?}");

            // Ensure overscroll retracts to original size after releasing gesture
            tester.pump_and_settle();
            assert_eq!(tester.get_size(&key).height(), 450.0, "{platform:?}");
        });
    }

    #[test]
    fn a_child_larger_than_the_extent_stretches_into_the_overscroll() {
        test_widgets_variant(ios_and_macos(), |tester| {
            let platform = tester.target_platform();
            let density_vertical = if platform == TargetPlatform::MacOS {
                -2.0
            } else {
                0.0
            };
            let controller = ScrollController::default(&mut tester.cell.borrow_mut());
            let (key, button) = (GlobalKey::new(), GlobalKey::new());
            tester.pump_widget(boilerplate(
                vec![
                    five_fixed_extent_items(),
                    fill_without_scroll_body(true, padded_center_button(platform, &key, &button)),
                ],
                Some(controller.clone()),
                Axis::Vertical,
            ));

            // Scroll to the end
            jump_to_max_scroll_extent(tester, controller.handle());
            assert_eq!(
                tester.get_size(&key).height(),
                148.0 + density_vertical * 4.0,
                "{platform:?}"
            );
            // Check that the button alignment is true to expectations
            let button_rect = tester.get_rect(&button);
            assert_eq!(button_rect.bottom, 550.0, "{platform:?}");
            assert_eq!(button_rect.center().dx(), 400.0, "{platform:?}");

            // Overscroll and check size
            tester.drag(find::by_type::<Scrollable>(), Offset::new(0.0, -50.0));
            tester.pump();
            assert!(tester.get_size(&key).height() > 148.0, "{platform:?}");

            // Check that the button alignment is true to expectations
            let button_rect = tester.get_rect(&button);
            assert!(button_rect.bottom < 550.0, "{platform:?}");
            assert_eq!(button_rect.center().dx(), 400.0, "{platform:?}");

            // Ensure overscroll retracts to original size after releasing gesture
            tester.pump_and_settle();
            assert_eq!(
                tester.get_size(&key).height(),
                148.0 + density_vertical * 4.0,
                "{platform:?}"
            );
        });
    }

    #[test]
    fn a_child_without_a_size_past_the_viewport_shows_only_in_the_overscroll() {
        test_widgets_variant(ios_and_macos(), |tester| {
            let platform = tester.target_platform();
            let controller = ScrollController::default(&mut tester.cell.borrow_mut());
            let key = GlobalKey::new();
            tester.pump_widget(boilerplate(
                vec![
                    five_fixed_extent_items(),
                    fill_without_scroll_body(
                        true,
                        ColoredBox::new(DEBUG_RED)
                            .key(key.clone().into())
                            .into_widget(),
                    ),
                ],
                Some(controller.clone()),
                Axis::Vertical,
            ));
            expect(tester, &find::by_global_key(&key), finds_nothing());
            assert!(!fixed_extent_item_is_onstage(tester, 4), "{platform:?}");

            // Scroll to bottom
            jump_to_max_scroll_extent(tester, controller.handle());
            // End of list
            expect(tester, &find::by_global_key(&key), finds_nothing());
            assert!(fixed_extent_item_is_onstage(tester, 4), "{platform:?}");

            // Overscroll
            tester.drag(find::by_type::<Scrollable>(), Offset::new(0.0, -50.0));
            tester.pump();
            expect(tester, &find::by_global_key(&key), finds_one_widget());
            assert!(fixed_extent_item_is_onstage(tester, 4), "{platform:?}");

            tester.pump_and_settle();
            expect(tester, &find::by_global_key(&key), finds_nothing());
            assert!(fixed_extent_item_is_onstage(tester, 4), "{platform:?}");
        });
    }

    /// Whether the fixed extent list's item `index` is onstage: `find.bySemanticsLabel` finds
    /// it, which waits for accessibility.
    fn fixed_extent_item_is_onstage(tester: &WidgetTester, index: i32) -> bool {
        onstage_children_of::<RenderSliverFixedExtentList>(tester).contains(&index)
    }

    #[test]
    fn a_child_without_a_size_past_the_viewport_ignores_fill_overscroll_where_scrolling_clamps() {
        test_widgets(|tester| {
            let controller = ScrollController::default(&mut tester.cell.borrow_mut());
            let key = GlobalKey::new();
            tester.pump_widget(boilerplate(
                vec![
                    five_fixed_extent_items(),
                    fill_without_scroll_body(
                        true,
                        ColoredBox::new(DEBUG_RED)
                            .key(key.clone().into())
                            .into_widget(),
                    ),
                ],
                Some(controller.clone()),
                Axis::Vertical,
            ));
            expect(tester, &find::by_global_key(&key), finds_nothing());
            assert!(!fixed_extent_item_is_onstage(tester, 4));

            // Scroll to bottom
            jump_to_max_scroll_extent(tester, controller.handle());

            // End of list
            expect(tester, &find::by_global_key(&key), finds_nothing());
            assert!(fixed_extent_item_is_onstage(tester, 4));

            // Overscroll
            tester.drag(find::by_type::<Scrollable>(), Offset::new(0.0, -50.0));
            tester.pump();

            expect(tester, &find::by_global_key(&key), finds_nothing());
            assert!(fixed_extent_item_is_onstage(tester, 4));
        });
    }

    #[test]
    fn a_sized_child_ignores_fill_overscroll_where_scrolling_clamps() {
        test_widgets(|tester| {
            let (key, button) = (GlobalKey::new(), GlobalKey::new());
            tester.pump_widget(boilerplate(
                vec![
                    sliver_box(),
                    fill_without_scroll_body(
                        true,
                        bottom_center_button(TargetPlatform::Android, &key, &button),
                    ),
                ],
                None,
                Axis::Vertical,
            ));
            assert_eq!(tester.get_size(&key).height(), 450.0);

            tester.drag(find::by_type::<Scrollable>(), Offset::new(0.0, -50.0));
            tester.pump();
            assert_eq!(tester.get_size(&key).height(), 450.0);

            // Also check that the button alignment is true to expectations
            let button_rect = tester.get_rect(&button);
            assert_eq!(button_rect.bottom, 600.0);
            assert_eq!(button_rect.center().dx(), 400.0);
        });
    }

    #[test]
    fn a_larger_child_ignores_fill_overscroll_where_scrolling_clamps() {
        test_widgets(|tester| {
            let controller = ScrollController::default(&mut tester.cell.borrow_mut());
            let (key, button) = (GlobalKey::new(), GlobalKey::new());
            tester.pump_widget(boilerplate(
                vec![
                    five_fixed_extent_items(),
                    fill_without_scroll_body(
                        true,
                        padded_center_button(TargetPlatform::Android, &key, &button),
                    ),
                ],
                Some(controller.clone()),
                Axis::Vertical,
            ));

            // Scroll to the end
            jump_to_max_scroll_extent(tester, controller.handle());
            assert_eq!(tester.get_size(&key).height(), 148.0);

            // Check that the button alignment is true to expectations
            let button_rect = tester.get_rect(&button);
            assert_eq!(button_rect.bottom, 550.0);
            assert_eq!(button_rect.center().dx(), 400.0);

            tester.drag(find::by_type::<Scrollable>(), Offset::new(0.0, -50.0));
            tester.pump();
            assert_eq!(tester.get_size(&key).height(), 148.0);

            // Check that the button alignment is still centered
            let button_rect = tester.get_rect(&button);
            assert_eq!(button_rect.bottom, 550.0);
            assert_eq!(button_rect.center().dx(), 400.0);
        });
    }
}
