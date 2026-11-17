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
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        if self.header.is_none() && self.footer.is_none() {
            return self.child.clone();
        }

        // The child is not positioned so that the stack can size itself to it along an
        // unbounded axis (for example in a horizontally scrolling list view), where a stack of
        // only positioned children would try to be infinitely large. Along bounded axes, the
        // child still fills the available space.
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
    use waft_embedder::{Size, TextBaseline};
    use waft_foundation::ValueKey;
    use waft_painting::{Alignment, Axis, BoxDecoration};
    use waft_widgets::testing::{WidgetTester, find, test_widgets};
    use waft_widgets::{
        Align, Center, ColoredBox, ConstrainedBox, Container, DecoratedBox, Directionality, Icon,
        LayoutBuilder, ListView, SizedBox, Text, TextDirection,
    };

    use super::*;
    use crate::colors::Colors;
    use crate::grid_tile_bar::GridTileBar;
    use crate::icons::Icons;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    /// Dart's `pumpWidget(MaterialApp(home: home))`.
    fn pump_home<K>(tester: &mut WidgetTester, home: impl IntoWidget<K>) {
        let theme = ThemeData::builder().build(&tester.cell.borrow());
        tester.pump_widget(material_app(theme, home.into_widget()));
    }

    fn key(name: &'static str) -> KeyRef {
        KeyRef::from(ValueKey::new(name))
    }

    fn tile_box(tester: &WidgetTester) -> AnyRenderBox {
        tester
            .render_object(find::by_type::<GridTile>())
            .as_box()
            .expect("a grid tile is a box")
    }

    #[test]
    fn grid_tile_control_test() {
        test_widgets(|tester| {
            let header_key = key("header");
            let footer_key = key("footer");

            pump_home(
                tester,
                GridTile::new(DecoratedBox::new(
                    BoxDecoration::new().color(Colors::GREEN.shade500()),
                ))
                .header(
                    GridTileBar::new()
                        .key(header_key.clone())
                        .leading(Icon::new(Some(Icons::thumb_up())))
                        .title(Text::new("Header"))
                        .subtitle(Text::new("Subtitle"))
                        .trailing(Icon::new(Some(Icons::thumb_up()))),
                )
                .footer(
                    GridTileBar::new()
                        .key(footer_key.clone())
                        .title(Text::new("Footer"))
                        .background_color(Colors::BLACK38),
                ),
            );

            assert_eq!(find::text("Header").evaluate(tester).len(), 1);
            assert_eq!(find::text("Footer").evaluate(tester).len(), 1);

            assert!(
                tester.get_bottom_left(find::by_key(header_key)).dy()
                    < tester.get_top_left(find::by_key(footer_key)).dy()
            );

            tester.pump_widget(Directionality::new(
                TextDirection::Ltr,
                GridTile::new(Text::new("Simple")),
            ));

            assert_eq!(find::text("Simple").evaluate(tester).len(), 1);
        });
    }

    #[test]
    fn grid_tile_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            pump_home(
                tester,
                Center::new().child(SizedBox::shrink().child(GridTile::new(Text::new("X")))),
            );
            assert_eq!(tester.get_size(find::by_type::<GridTile>()), Size::ZERO);
        });
    }

    #[test]
    fn grid_tile_with_header_and_footer_sizes_itself_to_its_child_when_the_width_is_unbounded() {
        test_widgets(|tester| {
            pump_home(
                tester,
                Center::new().child(
                    SizedBox::new().height(100.0).child(
                        ListView::builder(|_app, _context, _index| {
                            Some(
                                GridTile::new(
                                    Container::new()
                                        .height(100.0)
                                        .width(80.0)
                                        .color(Colors::RED.primary()),
                                )
                                .header(Text::new("Header"))
                                .footer(Text::new("Footer"))
                                .into_widget(),
                            )
                        })
                        .item_count(1)
                        .build()
                        .scroll_direction(Axis::Horizontal),
                    ),
                ),
            );

            assert_eq!(
                tester.get_size(find::by_type::<GridTile>()),
                Size::new(80.0, 100.0)
            );
            assert_eq!(find::text("Header").evaluate(tester).len(), 1);
            assert_eq!(find::text("Footer").evaluate(tester).len(), 1);
        });
    }

    #[test]
    fn grid_tile_child_fills_the_tile_when_the_constraints_are_tight() {
        test_widgets(|tester| {
            let child_key = key("child");

            pump_home(
                tester,
                Center::new().child(
                    SizedBox::new().width(200.0).height(150.0).child(
                        GridTile::new(
                            ColoredBox::new(Colors::RED.primary()).key(child_key.clone()),
                        )
                        .header(Text::new("Header"))
                        .footer(Text::new("Footer")),
                    ),
                ),
            );

            assert_eq!(
                tester.get_size(find::by_type::<GridTile>()),
                Size::new(200.0, 150.0)
            );
            assert_eq!(
                tester.get_size(find::by_key(child_key)),
                Size::new(200.0, 150.0)
            );
        });
    }

    #[test]
    fn grid_tile_child_fills_the_tile_when_the_constraints_are_bounded_but_loose() {
        test_widgets(|tester| {
            let child_key = key("child");

            pump_home(
                tester,
                Center::new().child(
                    ConstrainedBox::new(BoxConstraints::new().max_width(200.0).max_height(150.0))
                        .child(
                            GridTile::new(
                                Container::new()
                                    .key(child_key.clone())
                                    .width(80.0)
                                    .height(100.0)
                                    .color(Colors::RED.primary()),
                            )
                            .header(Text::new("Header"))
                            .footer(Text::new("Footer")),
                        ),
                ),
            );

            assert_eq!(
                tester.get_size(find::by_type::<GridTile>()),
                Size::new(200.0, 150.0)
            );
            assert_eq!(
                tester.get_size(find::by_key(child_key)),
                Size::new(200.0, 150.0)
            );
        });
    }

    #[test]
    fn grid_tile_dry_baseline_matches_its_baseline_when_the_constraints_are_bounded_but_loose() {
        test_widgets(|tester| {
            let constraints = BoxConstraints::new().max_width(200.0).max_height(150.0);

            pump_home(
                tester,
                Center::new().child(
                    ConstrainedBox::new(constraints).child(
                        GridTile::new(
                            // Shrink-wraps the text vertically under loose constraints, but
                            // centers it vertically when forced to fill the tile.
                            Align::new()
                                .alignment(Alignment::CENTER_LEFT.into())
                                .height_factor(1.0)
                                .child(Text::new("X")),
                        )
                        .footer(Text::new("Footer")),
                    ),
                ),
            );

            // The tile's baseline is the baseline of the child's text, which is above
            // the footer's.
            let text = tester
                .render_object(find::text("X"))
                .as_box()
                .expect("a text is a box");
            let text_baseline = text
                .get_dry_baseline(
                    &mut tester.cell.borrow_mut(),
                    BoxConstraints::new(),
                    TextBaseline::Alphabetic,
                )
                .expect("a text has a baseline");
            let expected_baseline = tester.get_top_left(find::text("X")).dy()
                - tester.get_top_left(find::by_type::<GridTile>()).dy()
                + text_baseline;
            assert_eq!(
                tester.get_size(find::by_type::<GridTile>()),
                Size::new(200.0, 150.0)
            );

            let tile = tester
                .render_object(find::descendant(
                    find::by_type::<GridTile>(),
                    find::by_type::<Stack>(),
                ))
                .as_box()
                .expect("a stack is a box");
            assert_eq!(
                tile.get_dry_baseline(
                    &mut tester.cell.borrow_mut(),
                    constraints,
                    TextBaseline::Alphabetic
                ),
                Some(expected_baseline)
            );
        });
    }

    #[test]
    fn grid_tile_with_header_and_footer_reports_zero_intrinsic_sizes() {
        test_widgets(|tester| {
            pump_home(
                tester,
                GridTile::new(SizedBox::new().width(80.0).height(100.0))
                    .header(Text::new("Header"))
                    .footer(Text::new("Footer")),
            );

            let tile = tile_box(tester);
            let mut app = tester.cell.borrow_mut();
            assert_eq!(tile.get_min_intrinsic_width(&mut app, 150.0), 0.0);
            assert_eq!(tile.get_max_intrinsic_width(&mut app, 150.0), 0.0);
            assert_eq!(tile.get_min_intrinsic_height(&mut app, 200.0), 0.0);
            assert_eq!(tile.get_max_intrinsic_height(&mut app, 200.0), 0.0);
        });
    }

    #[test]
    fn grid_tile_dry_layout_does_not_query_its_child_when_both_axes_are_bounded() {
        test_widgets(|tester| {
            pump_home(
                tester,
                GridTile::new(
                    LayoutBuilder::new(|_app, _context, _constraints| {
                        SizedBox::new().into_widget()
                    })
                    .into_widget(),
                )
                .header(Text::new("Header"))
                .footer(Text::new("Footer")),
            );

            let tile = tile_box(tester);
            let mut app = tester.cell.borrow_mut();
            assert_eq!(
                tile.get_dry_layout(
                    &mut app,
                    BoxConstraints::new().max_width(200.0).max_height(150.0)
                ),
                Size::new(200.0, 150.0)
            );
            assert_eq!(
                tile.get_dry_layout(
                    &mut app,
                    BoxConstraints::tight_for(Some(200.0), Some(150.0))
                ),
                Size::new(200.0, 150.0)
            );
        });
    }
}
