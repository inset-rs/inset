//! Flutter counterpart: material_ui's `grid_tile.dart`.

use waft_embedder::{Offset, Size, TextBaseline};
use waft_foundation::App;
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, ChildLayoutHelper, Constraints, PaintingContext,
    RenderBox, RenderBoxData, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin, RenderProxyBoxMixin,
};
use waft_widgets::{
    BuildContext, IntoWidget, KeyRef, Positioned, RenderObjectWidget,
    SingleChildRenderObjectWidget, Stack, StackFit, StatelessWidget, WidgetRef,
};

/// A tile in a Material Design grid list.
///
/// A grid list is a `GridView` of tiles in a vertical and horizontal array. Each tile
/// typically contains some visually rich content (such as an image) together with a
/// [`GridTileBar`](crate::GridTileBar) in either the header or the footer.
///
/// See also:
///
///  * `GridView`, which is a scrollable grid of tiles.
///  * [`GridTileBar`](crate::GridTileBar), which is typically used in either the header or the
///    footer.
#[derive(Debug)]
pub struct GridTile {
    pub key: Option<KeyRef>,
    /// The widget to show over the top of this grid tile.
    ///
    /// Typically a [`GridTileBar`](crate::GridTileBar).
    pub header: Option<WidgetRef>,
    /// The widget to show over the bottom of this grid tile.
    ///
    /// Typically a [`GridTileBar`](crate::GridTileBar).
    pub footer: Option<WidgetRef>,
    /// The widget that fills the tile.
    pub child: WidgetRef,
}

impl GridTile {
    /// Creates a grid tile; Dart's optional named arguments are the setters.
    pub fn new<K>(child: impl IntoWidget<K>) -> GridTile {
        GridTile {
            key: None,
            header: None,
            footer: None,
            child: child.into_widget(),
        }
    }

    /// Dart `GridTile(key:)`.
    pub fn key(mut self, key: KeyRef) -> GridTile {
        self.key = Some(key);
        self
    }

    /// Dart `GridTile(header:)`.
    pub fn header<K>(mut self, header: impl IntoWidget<K>) -> GridTile {
        self.header = Some(header.into_widget());
        self
    }

    /// Dart `GridTile(footer:)`.
    pub fn footer<K>(mut self, footer: impl IntoWidget<K>) -> GridTile {
        self.footer = Some(footer.into_widget());
        self
    }
}

impl StatelessWidget for GridTile {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        if self.header.is_none() && self.footer.is_none() {
            return self.child.clone();
        }

        // The child is not positioned so that the stack can size itself to it along an
        // unbounded axis (for example in a horizontally scrolling list view), where a stack of
        // only positioned children would try to be infinitely large.
        let mut children = vec![FillBoundedAxes::new(self.child.clone()).into_widget()];
        if let Some(header) = &self.header {
            children.push(
                Positioned::new(header.clone())
                    .top(0.0)
                    .left(0.0)
                    .right(0.0)
                    .into_widget(),
            );
        }
        if let Some(footer) = &self.footer {
            children.push(
                Positioned::new(footer.clone())
                    .left(0.0)
                    .bottom(0.0)
                    .right(0.0)
                    .into_widget(),
            );
        }
        Stack::new()
            .fit(StackFit::Passthrough)
            .children(children)
            .into_widget()
    }
}

/// Makes its child as big as the incoming constraints allow along every bounded axis, and lets
/// the child pick its own size along unbounded axes. Dart's `_FillBoundedAxes`.
#[derive(Debug)]
struct FillBoundedAxes {
    child: WidgetRef,
}

impl FillBoundedAxes {
    fn new(child: WidgetRef) -> FillBoundedAxes {
        FillBoundedAxes { child }
    }
}

impl RenderObjectWidget for FillBoundedAxes {
    type RenderObject = RenderFillBoundedAxes;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderFillBoundedAxes::new(app).as_object()
    }
}

impl SingleChildRenderObjectWidget for FillBoundedAxes {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

/// Dart's `_RenderFillBoundedAxes`.
struct RenderFillBoundedAxes {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
}

impl RenderFillBoundedAxes {
    fn new(app: &mut App) -> RenderHandle<RenderFillBoundedAxes> {
        RenderHandle::new_box(
            app,
            RenderFillBoundedAxes {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
            },
        )
    }

    fn fill_bounded_axes(constraints: BoxConstraints) -> BoxConstraints {
        constraints
            .copy_with()
            .min_width(if constraints.has_bounded_width() {
                constraints.max_width
            } else {
                constraints.min_width
            })
            .min_height(if constraints.has_bounded_height() {
                constraints.max_height
            } else {
                constraints.min_height
            })
    }
}

impl RenderObjectWithChildMixin for RenderFillBoundedAxes {
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

impl RenderProxyBoxMixin for RenderFillBoundedAxes {}

impl RenderObject for RenderFillBoundedAxes {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let child_constraints = RenderFillBoundedAxes::fill_bounded_axes(constraints);
        let Some(child) = self.child(app) else {
            let size = constraints.constrain(child_constraints.smallest());
            self.set_size(app, size);
            return;
        };
        let child_size = ChildLayoutHelper::layout_child(app, child, child_constraints);
        self.set_size(app, constraints.constrain(child_size));
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
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

impl RenderBox for RenderFillBoundedAxes {
    waft_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut waft_embedder::Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut waft_rendering::BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    /// The tile used to report zero intrinsic sizes because its child was positioned, which
    /// the stack ignores for intrinsics. This keeps doing so.
    fn compute_min_intrinsic_width(self: RenderHandle<Self>, _app: &mut App, _height: f64) -> f64 {
        0.0
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, _app: &mut App, _height: f64) -> f64 {
        0.0
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, _app: &mut App, _width: f64) -> f64 {
        0.0
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, _app: &mut App, _width: f64) -> f64 {
        0.0
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let child_constraints = RenderFillBoundedAxes::fill_bounded_axes(constraints);
        if child_constraints.is_tight() {
            return child_constraints.smallest();
        }
        let child_size = match self.child(app) {
            Some(child) => child.get_dry_layout(app, child_constraints),
            None => child_constraints.smallest(),
        };
        constraints.constrain(child_size)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child_constraints = RenderFillBoundedAxes::fill_bounded_axes(constraints);
        self.child(app)?
            .get_dry_baseline(app, child_constraints, baseline)
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::Offset;
    use waft_widgets::testing::{WidgetTester, find, test_widgets};
    use waft_widgets::{Center, SizedBox};

    use super::*;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    /// Dart's `pumpWidget(MaterialApp(home: Center(child: tile)))`.
    fn pump(tester: &mut WidgetTester, tile: GridTile) {
        let theme = ThemeData::builder().build(&tester.cell.borrow());
        tester.pump_widget(material_app(theme, Center::new().child(tile).into_widget()));
    }

    #[test]
    fn a_tile_without_a_header_or_footer_is_its_child() {
        test_widgets(|tester| {
            pump(tester, GridTile::new(SizedBox::square(Some(100.0))));

            expect_no_stack(tester);
            let child = find::by_type::<SizedBox>();
            assert_eq!(
                tester.get_size(child.first()),
                waft_embedder::Size::new(100.0, 100.0)
            );
        });
    }

    #[test]
    fn a_header_and_a_footer_overlay_a_child_that_fills_the_tile() {
        test_widgets(|tester| {
            let theme = ThemeData::builder().build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(
                        SizedBox::square(Some(100.0)).child(
                            GridTile::new(SizedBox::square(Some(100.0)))
                                .header(SizedBox::square(Some(20.0)))
                                .footer(SizedBox::square(Some(20.0))),
                        ),
                    )
                    .into_widget(),
            ));

            let tile = tester.get_top_left(find::by_type::<Stack>());
            let twenty: Vec<_> = find::by_type::<SizedBox>()
                .evaluate(tester)
                .into_iter()
                .filter(|element| tester.get_size(*element).height() == 20.0)
                .collect();
            assert_eq!(twenty.len(), 2, "a header and a footer");
            assert_eq!(
                tester.get_top_left(twenty[0]),
                tile,
                "the header is at the tile's top left"
            );
            assert_eq!(
                tester.get_top_left(twenty[1]),
                tile + Offset::new(0.0, 80.0),
                "the footer is at the tile's bottom left"
            );
        });
    }

    /// Dart's `expect(find.byType(Stack), findsNothing)`.
    fn expect_no_stack(tester: &WidgetTester) {
        assert!(
            find::by_type::<Stack>().evaluate(tester).is_empty(),
            "a tile without a header or footer builds no stack"
        );
    }
}
