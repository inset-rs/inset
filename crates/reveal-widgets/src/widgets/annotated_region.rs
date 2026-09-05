//! Flutter counterpart: `widgets/annotated_region.dart`.

use std::fmt::Debug;
use std::rc::Rc;

use reveal_foundation::App;
use reveal_rendering::{AnyRenderObject, RenderAnnotatedRegion, RenderBox, RenderHandle};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, RenderObjectWidget, SingleChildRenderObjectWidget, WidgetRef,
};

/// Annotates a region of the layer tree with a value.
///
/// See also:
///
///  * [`BoundaryLayer::find`](reveal_rendering::BoundaryLayer::find), for how this value is
///    retrieved.
///  * [`AnnotatedRegionLayer`](reveal_rendering::AnnotatedRegionLayer), the layer pushed into
///    the recording.
#[derive(Debug)]
pub struct AnnotatedRegion<T> {
    pub key: Option<KeyRef>,
    /// A value which can be retrieved using
    /// [`BoundaryLayer::find`](reveal_rendering::BoundaryLayer::find).
    ///
    /// Dart's `T value`; it is shared rather than copied so the recording can carry it.
    pub value: Rc<T>,
    /// If false, the layer pushed into the tree will not be provided with a size.
    ///
    /// An [`AnnotatedRegionLayer`](reveal_rendering::AnnotatedRegionLayer) with a size checks
    /// that the offset provided in [`BoundaryLayer::find`](reveal_rendering::BoundaryLayer::find)
    /// is within the bounds, returning `None` otherwise.
    pub sized: bool,
    pub child: WidgetRef,
}

impl<T: PartialEq + Debug + 'static> AnnotatedRegion<T> {
    /// Creates a new annotated region to insert `value` into the layer tree.
    ///
    /// [`sized`](Self::sized) defaults to true and controls whether the annotated region will
    /// clip its child.
    pub fn new<K>(child: impl IntoWidget<K>, value: T) -> AnnotatedRegion<T> {
        AnnotatedRegion {
            key: None,
            value: Rc::new(value),
            sized: true,
            child: child.into_widget(),
        }
    }

    /// Dart `AnnotatedRegion(key:)`.
    pub fn key(mut self, key: KeyRef) -> AnnotatedRegion<T> {
        self.key = Some(key);
        self
    }

    /// Dart `AnnotatedRegion(sized:)`.
    pub fn sized(mut self, sized: bool) -> AnnotatedRegion<T> {
        self.sized = sized;
        self
    }
}

impl<T: PartialEq + Debug + 'static> RenderObjectWidget for AnnotatedRegion<T> {
    type RenderObject = RenderAnnotatedRegion<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderAnnotatedRegion::new(app, Rc::clone(&self.value), self.sized, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderAnnotatedRegion<T>>,
    ) {
        render_object.set_value(app, Rc::clone(&self.value));
        render_object.set_sized(app, self.sized);
    }
}

impl<T: PartialEq + Debug + 'static> SingleChildRenderObjectWidget for AnnotatedRegion<T> {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

#[cfg(test)]
mod tests {
    use reveal_embedder::Offset;
    use reveal_rendering::{AnnotationResult, BoundaryLayer};

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Center, SizedBox};

    /// The annotated value; two markers of the same type so one search finds both.
    #[derive(Debug, PartialEq)]
    struct Marker(u32);

    fn sized(width: f64, height: f64) -> WidgetRef {
        SizedBox {
            width: Some(width),
            height: Some(height),
            ..Default::default()
        }
        .into_widget()
    }

    /// The root recording of a mounted tree, which `find` searches.
    fn root_layer<'a>(harness: &Harness, app: &'a App) -> &'a BoundaryLayer {
        harness
            .render_root(app)
            .as_object()
            .debug_layer(app)
            .expect("the root has a layer")
    }

    /// A `sized` 100 x 50 region centred in the 300 x 200 view, so it spans (100, 75) to
    /// (200, 125).
    fn centred_region(sized_region: bool) -> WidgetRef {
        Center::new()
            .child(
                SizedBox {
                    width: Some(100.0),
                    height: Some(50.0),
                    child: Some(
                        AnnotatedRegion::new(SizedBox::expand(), Marker(1))
                            .sized(sized_region)
                            .into_widget(),
                    ),
                    ..Default::default()
                }
                .into_widget(),
            )
            .into_widget()
    }

    #[test]
    fn a_sized_region_answers_inside_its_bounds_and_nowhere_else() {
        let mut app = App::new();
        let harness = Harness::mount(&mut app, centred_region(true));
        harness.pump(&mut app);
        let layer = root_layer(&harness, &app);

        let inside = layer.find::<Marker>(&app, Offset::new(150.0, 100.0));
        assert_eq!(inside.as_deref(), Some(&Marker(1)));
        assert!(
            layer
                .find::<Marker>(&app, Offset::new(100.0, 75.0))
                .is_some(),
            "the top left corner is inside"
        );
        assert!(
            layer
                .find::<Marker>(&app, Offset::new(10.0, 10.0))
                .is_none(),
            "outside the region"
        );
        assert!(
            layer
                .find::<Marker>(&app, Offset::new(200.0, 100.0))
                .is_none(),
            "the right edge is excluded"
        );
    }

    #[test]
    fn an_unsized_region_answers_everywhere() {
        let mut app = App::new();
        let harness = Harness::mount(&mut app, centred_region(false));
        harness.pump(&mut app);
        let layer = root_layer(&harness, &app);

        assert_eq!(
            layer
                .find::<Marker>(&app, Offset::new(150.0, 100.0))
                .as_deref(),
            Some(&Marker(1))
        );
        assert_eq!(
            layer.find::<Marker>(&app, Offset::new(1.0, 1.0)).as_deref(),
            Some(&Marker(1)),
            "no size means no bounds check"
        );
    }

    #[test]
    fn a_find_of_another_type_finds_nothing() {
        let mut app = App::new();
        let harness = Harness::mount(&mut app, centred_region(true));
        harness.pump(&mut app);
        assert!(
            root_layer(&harness, &app)
                .find::<u32>(&app, Offset::new(150.0, 100.0))
                .is_none(),
            "the target type must be identical to the annotated type"
        );
    }

    #[test]
    fn nested_regions_answer_innermost_first() {
        let mut app = App::new();
        let inner = AnnotatedRegion::new(SizedBox::expand(), Marker(2));
        let tree = Center::new()
            .child(
                SizedBox {
                    width: Some(100.0),
                    height: Some(50.0),
                    child: Some(
                        AnnotatedRegion::new(
                            Center::new().child(
                                SizedBox {
                                    width: Some(40.0),
                                    height: Some(20.0),
                                    child: Some(inner.into_widget()),
                                    ..Default::default()
                                }
                                .into_widget(),
                            ),
                            Marker(1),
                        )
                        .into_widget(),
                    ),
                    ..Default::default()
                }
                .into_widget(),
            )
            .into_widget();
        let harness = Harness::mount(&mut app, tree);
        harness.pump(&mut app);
        let layer = root_layer(&harness, &app);

        let result: AnnotationResult<Marker> =
            layer.find_all_annotations(&app, Offset::new(150.0, 100.0));
        assert_eq!(
            result
                .annotations()
                .map(|marker| marker.0)
                .collect::<Vec<_>>(),
            vec![2, 1],
            "the innermost region is the most specific"
        );
        assert_eq!(
            result
                .entries()
                .iter()
                .map(|entry| entry.local_position)
                .collect::<Vec<_>>(),
            vec![Offset::new(20.0, 10.0), Offset::new(50.0, 25.0)],
            "each entry is in the coordinate space of its own region"
        );

        assert_eq!(
            layer
                .find::<Marker>(&app, Offset::new(150.0, 100.0))
                .as_deref(),
            Some(&Marker(2)),
            "`find` stops at the first"
        );

        let outer_only: AnnotationResult<Marker> =
            layer.find_all_annotations(&app, Offset::new(110.0, 80.0));
        assert_eq!(
            outer_only
                .annotations()
                .map(|marker| marker.0)
                .collect::<Vec<_>>(),
            vec![1],
            "outside the inner region, only the outer one answers"
        );
    }

    #[test]
    fn a_region_reconfigures_its_render_object_in_place() {
        let mut app = App::new();
        let region = |value: u32, sized_region: bool| {
            AnnotatedRegion::new(sized(10.0, 10.0), Marker(value))
                .sized(sized_region)
                .into_widget()
        };
        let harness = Harness::mount(&mut app, region(1, true));
        harness.pump(&mut app);
        let render_object = harness
            .render_root(&app)
            .child(&app)
            .expect("a child")
            .as_object()
            .downcast::<RenderAnnotatedRegion<Marker>>(&app)
            .expect("a RenderAnnotatedRegion");
        assert_eq!(*render_object.value(&app), Marker(1));
        assert!(render_object.sized(&app), "Dart's default");

        harness.set_child(&mut app, region(2, false));
        harness.pump(&mut app);
        assert_eq!(
            *render_object.value(&app),
            Marker(2),
            "the same render object is reconfigured"
        );
        assert!(!render_object.sized(&app));
    }
}
