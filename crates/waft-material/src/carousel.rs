//! Flutter counterpart: material_ui's `carousel.dart`.
//!
//! `CarouselView` and its scrolling machinery: the two slivers with their render objects
//! (items of a fixed extent that shrink at the edges, and items sized by weights), the
//! snapping physics, the position that keeps its offset in items, and the controller.
//!
//! Dart's render objects read their `constraints` inside the `itemExtentBuilder` they pass to
//! `RenderSliverFixedExtentBoxAdaptor`. Here the builder takes no `App`, so each render object
//! hands the adaptor a builder over a snapshot of the constraints it reads; the constraints do
//! not change during one layout.

#![allow(clippy::too_many_arguments)]

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::Curve;
use waft_embedder::{Clip, Color, clamp_double};
use waft_foundation::{
    App, ChangeNotifier, ChangeNotifierData, CompleterFuture, Handle, ListenableObject, Listener,
    OptionRetained, PRECISION_ERROR_TOLERANCE, Ref, ValueChanged, wait_all,
};
use waft_painting::{
    Axis, AxisDirection, BorderRadiusGeometry, EdgeInsets, RoundedRectangleBorder, ShapeBorderRef,
};
use waft_physics::{ScrollSpringSimulation, Simulation, Tolerance};
use waft_rendering::{
    AnyRenderBox, ContainerRenderObjectData, ContainerRenderObjectMixin, ItemExtentBuilder,
    RenderHandle, RenderObjectData, RenderSliver, RenderSliverBoxChildManager, RenderSliverData,
    RenderSliverFixedExtentBoxAdaptor, RenderSliverFixedExtentBoxAdaptorData,
    RenderSliverMultiBoxAdaptor, RenderSliverMultiBoxAdaptorData, ScrollCacheExtent,
    SliverConstraints, SliverGeometry, SliverLayoutDimensions, SliverMultiBoxAdaptorParentData,
    ViewportOffset, fixed_extent_box_adaptor_leaf, render_sliver_fixed_extent_accessors,
};
use waft_widgets::{
    AnyScrollPosition, BuildContext, FixedScrollMetrics, GestureDetector, IntoWidget, KeyRef,
    LayoutBuilder, NullableIndexedWidgetBuilder, Padding, RenderObjectWidget,
    ScrollActivityDelegate, ScrollConfiguration, ScrollContext, ScrollControllerData,
    ScrollControllerLeaf, ScrollMetrics, ScrollPhysics, ScrollPhysicsBase, ScrollPhysicsRef,
    ScrollPosition, ScrollPositionBase, ScrollPositionData, ScrollPositionRef,
    ScrollPositionWithSingleContext, ScrollPositionWithSingleContextData,
    ScrollPositionWithSingleContextLeaf, SliverChildBuilderDelegate, SliverChildDelegateRef,
    SliverMultiBoxAdaptorWidget, SliverWithKeepAliveWidget, Stack, StackFit, State, StateData,
    StatefulWidget, WidgetRef, WidgetState, WidgetStateProperty, WidgetStatePropertyRef,
    child_manager_of,
};

use crate::carousel_theme::CarouselViewTheme;
use crate::colors::Colors;
use crate::ink_well::InkWell;
use crate::material::Material;
use crate::theme::Theme;

/// Signature of the builder `CarouselView.builder` and `CarouselView.weightedBuilder` take: it
/// answers `None` for an index beyond the last item.
pub type CarouselItemBuilder = NullableIndexedWidgetBuilder;

/// A Material Design carousel widget.
///
/// The [`CarouselView`] presents a scrollable list of items, each of which can dynamically
/// change size based on the chosen layout.
///
/// Material Design 3 introduced 4 carousel layouts:
///
///  * Multi-browse and uncontained: [`CarouselView::new`] with an
///    [`item_extent`](Self::item_extent) — the items have the same maximum extent and shrink to
///    a smaller one as they reach the edges.
///  * Hero and center-aligned hero: [`CarouselView::weighted`], whose items' extents follow
///    the [`flex_weights`](Self::flex_weights).
///
/// The [`CarouselView::builder`] and [`CarouselView::weighted_builder`] constructors build their
/// items lazily; with [`infinite`](Self::infinite) set, the list repeats its items forever.
///
/// The properties of the items' `Material` that are `None` come from the
/// [`CarouselViewTheme`](crate::CarouselViewTheme), and then from the defaults documented on
/// each field.
///
/// See also:
///
///  * [`CarouselController`], which controls the scroll position of this widget.
///  * `PageView`, which is a scrollable list that works page by page.
pub struct CarouselView {
    pub key: Option<KeyRef>,
    /// The amount of space surrounding each carousel item.
    ///
    /// Defaults to `EdgeInsets::all(4.0)`.
    pub padding: Option<EdgeInsets>,

    /// The background color for each carousel item.
    ///
    /// Defaults to `ColorScheme::surface`.
    pub background_color: Option<Color>,

    /// The z-coordinate of each carousel item.
    ///
    /// Defaults to 0.0.
    pub elevation: Option<f64>,

    /// The shape of each carousel item's [`Material`].
    ///
    /// Defaults to a `RoundedRectangleBorder` with a circular corner radius of 28.0.
    pub shape: Option<ShapeBorderRef>,

    /// The clip behavior for each carousel item.
    ///
    /// Defaults to [`Clip::AntiAlias`].
    pub item_clip_behavior: Option<Clip>,

    /// The highlight color to indicate the carousel items are in pressed, hovered or focused
    /// states.
    ///
    /// Defaults to `ColorScheme::onSurface` with opacities of 0.1 (pressed), 0.08 (hovered)
    /// and 0.1 (focused).
    pub overlay_color: Option<WidgetStatePropertyRef<Option<Color>>>,

    /// The minimum allowable extent (size) in the main axis for carousel items in the
    /// viewport.
    ///
    /// As the carousel scrolls, the first visible item is pinned and gradually shrinks to
    /// [`shrink_extent`](Self::shrink_extent) before disappearing.
    ///
    /// Defaults to 0.0.
    pub shrink_extent: f64,

    /// The scroll position snaps to the nearest item when scrolling ends.
    ///
    /// Defaults to false.
    pub item_snapping: bool,

    /// An object that can be used to control the position to which this scroll view is
    /// scrolled.
    pub controller: Option<Ref<CarouselController>>,

    /// The [`Axis`] along which the scroll view's offset increases with each item.
    ///
    /// Defaults to [`Axis::Horizontal`].
    pub scroll_direction: Axis,

    /// Whether the carousel list scrolls in the reading direction.
    ///
    /// Defaults to false.
    pub reverse: bool,

    /// Whether the items scroll past the position of the largest weight so that every item
    /// has a chance to be the largest. Only meaningful for the weighted constructors.
    ///
    /// Defaults to true.
    pub consume_max_weight: bool,

    /// Called when one of the items is tapped, with its index.
    pub on_tap: Option<ValueChanged<i32>>,

    /// Whether the items show the ink splash and the overlay colors when they are tapped,
    /// hovered or focused.
    ///
    /// Defaults to true.
    pub enable_splash: bool,

    /// The extent the children are forced to have in the main axis. Set by
    /// [`CarouselView::new`] and [`CarouselView::builder`].
    pub item_extent: Option<f64>,

    /// The weights that determine the extent of each item. Set by [`CarouselView::weighted`]
    /// and [`CarouselView::weighted_builder`].
    pub flex_weights: Option<Vec<i32>>,

    /// The child widgets for the carousel; empty for the builder constructors.
    pub children: Vec<WidgetRef>,

    /// Called when the leading item changes while the carousel is scrolled.
    pub on_index_changed: Option<ValueChanged<i32>>,

    /// The builder of the builder constructors, which build the items lazily.
    pub item_builder: Option<CarouselItemBuilder>,

    /// The number of items the builder constructors can build; `None` means as many as the
    /// builder answers.
    pub item_count: Option<i32>,

    /// Whether the carousel loops its items forever.
    ///
    /// Defaults to false.
    pub infinite: bool,
}

impl CarouselView {
    fn base() -> CarouselView {
        CarouselView {
            key: None,
            padding: None,
            background_color: None,
            elevation: None,
            shape: None,
            item_clip_behavior: None,
            overlay_color: None,
            shrink_extent: 0.0,
            item_snapping: false,
            controller: None,
            scroll_direction: Axis::Horizontal,
            reverse: false,
            consume_max_weight: true,
            on_tap: None,
            enable_splash: true,
            item_extent: None,
            flex_weights: None,
            children: Vec::new(),
            on_index_changed: None,
            item_builder: None,
            item_count: None,
            infinite: false,
        }
    }

    /// Creates a Material Design carousel whose items all have the extent `item_extent`.
    pub fn new(item_extent: f64, children: Vec<WidgetRef>) -> CarouselView {
        CarouselView {
            item_extent: Some(item_extent),
            children,
            ..CarouselView::base()
        }
    }

    /// Creates a Material Design carousel whose items' extents follow `flex_weights`.
    ///
    /// The length of `flex_weights` is how many items fit in the viewport at a time.
    pub fn weighted(flex_weights: Vec<i32>, children: Vec<WidgetRef>) -> CarouselView {
        CarouselView {
            flex_weights: Some(flex_weights),
            children,
            ..CarouselView::base()
        }
    }

    /// Creates a Material Design carousel that builds its items lazily, each with the extent
    /// `item_extent`.
    pub fn builder(item_extent: f64, item_builder: CarouselItemBuilder) -> CarouselView {
        CarouselView {
            item_extent: Some(item_extent),
            item_builder: Some(item_builder),
            ..CarouselView::base()
        }
    }

    /// Creates a Material Design carousel that builds its items lazily, with extents that
    /// follow `flex_weights`.
    pub fn weighted_builder(
        flex_weights: Vec<i32>,
        item_builder: CarouselItemBuilder,
    ) -> CarouselView {
        CarouselView {
            flex_weights: Some(flex_weights),
            item_builder: Some(item_builder),
            ..CarouselView::base()
        }
    }

    /// Dart `CarouselView(key:)`.
    pub fn key(mut self, key: KeyRef) -> CarouselView {
        self.key = Some(key);
        self
    }

    /// Dart `CarouselView(padding:)`.
    pub fn padding(mut self, padding: EdgeInsets) -> CarouselView {
        self.padding = Some(padding);
        self
    }

    /// Dart `CarouselView(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> CarouselView {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `CarouselView(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> CarouselView {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `CarouselView(shape:)`.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> CarouselView {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `CarouselView(itemClipBehavior:)`.
    pub fn item_clip_behavior(mut self, item_clip_behavior: Clip) -> CarouselView {
        self.item_clip_behavior = Some(item_clip_behavior);
        self
    }

    /// Dart `CarouselView(overlayColor:)`.
    pub fn overlay_color(
        mut self,
        overlay_color: WidgetStatePropertyRef<Option<Color>>,
    ) -> CarouselView {
        self.overlay_color = Some(overlay_color);
        self
    }

    /// Dart `CarouselView(itemSnapping:)`.
    pub fn item_snapping(mut self, item_snapping: bool) -> CarouselView {
        self.item_snapping = item_snapping;
        self
    }

    /// Dart `CarouselView(shrinkExtent:)`.
    pub fn shrink_extent(mut self, shrink_extent: f64) -> CarouselView {
        self.shrink_extent = shrink_extent;
        self
    }

    /// Dart `CarouselView(controller:)`.
    pub fn controller(mut self, controller: Ref<CarouselController>) -> CarouselView {
        self.controller = Some(controller);
        self
    }

    /// Dart `CarouselView(scrollDirection:)`.
    pub fn scroll_direction(mut self, scroll_direction: Axis) -> CarouselView {
        self.scroll_direction = scroll_direction;
        self
    }

    /// Dart `CarouselView(reverse:)`.
    pub fn reverse(mut self, reverse: bool) -> CarouselView {
        self.reverse = reverse;
        self
    }

    /// Dart `CarouselView.weighted(consumeMaxWeight:)`.
    pub fn consume_max_weight(mut self, consume_max_weight: bool) -> CarouselView {
        self.consume_max_weight = consume_max_weight;
        self
    }

    /// Dart `CarouselView(onTap:)`.
    pub fn on_tap(mut self, on_tap: ValueChanged<i32>) -> CarouselView {
        self.on_tap = Some(on_tap);
        self
    }

    /// Dart `CarouselView(enableSplash:)`.
    pub fn enable_splash(mut self, enable_splash: bool) -> CarouselView {
        self.enable_splash = enable_splash;
        self
    }

    /// Dart `CarouselView(onIndexChanged:)`.
    pub fn on_index_changed(mut self, on_index_changed: ValueChanged<i32>) -> CarouselView {
        self.on_index_changed = Some(on_index_changed);
        self
    }

    /// Dart `CarouselView.builder(itemCount:)`.
    pub fn item_count(mut self, item_count: i32) -> CarouselView {
        self.item_count = Some(item_count);
        self
    }

    /// Dart `CarouselView(infinite:)`.
    pub fn infinite(mut self, infinite: bool) -> CarouselView {
        self.infinite = infinite;
        self
    }
}

impl Debug for CarouselView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CarouselView")
            .field("item_extent", &self.item_extent)
            .field("flex_weights", &self.flex_weights)
            .field("children", &self.children.len())
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CarouselView {
    type State = CarouselViewState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CarouselViewState {
        CarouselViewState {
            state: StateData::new(),
            item_extent: None,
            internal_controller: None,
            last_reported_leading_item: 0,
        }
    }
}

/// Dart's `_CarouselViewState`.
pub struct CarouselViewState {
    state: StateData<CarouselView>,
    item_extent: Option<f64>,
    internal_controller: Option<Ref<CarouselController>>,
    last_reported_leading_item: i32,
}

impl CarouselViewState {
    fn flex_weights(self: Handle<Self>, app: &App) -> Option<Vec<i32>> {
        self.widget(app).flex_weights.clone()
    }

    fn consume_max_weight(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).consume_max_weight
    }

    /// Dart's `widget.controller ?? _internalController!`.
    fn controller(self: Handle<Self>, app: &App) -> Handle<CarouselController> {
        match self.widget(app).controller.as_ref() {
            Some(controller) => controller.handle(),
            None => app
                .get(self)
                .internal_controller
                .handle()
                .expect("an internal controller exists while the widget has none"),
        }
    }

    fn carousel_position(self: Handle<Self>, app: &App) -> Handle<CarouselPosition> {
        self.controller(app).carousel_position(app)
    }

    fn handle_scroll(self: Handle<Self>, app: &mut App) {
        let Some(on_index_changed) = self.widget(app).on_index_changed.clone() else {
            return;
        };

        let current_leading_index = self.carousel_position(app).leading_item(app);

        if current_leading_index != app.get(self).last_reported_leading_item {
            app.get_mut(self).last_reported_leading_item = current_leading_index;
            on_index_changed(app, current_leading_index);
        }
    }

    // For weighted carousel, the initialItem means the index of the item to occupy the first
    // maximum weight in flexWeights. To get the initial leading item, it should be initialItem -
    // index of the first max weight in flexWeights. So it might be negative when initialItem
    // value is small but the first max weight index is large. In that case, the initial leading
    // item should be 0.
    fn get_initial_leading_item(self: Handle<Self>, app: &App) -> i32 {
        let initial_item = self.controller(app).initial_item(app);
        if let Some(flex_weights) = self.flex_weights(app) {
            let first_max_weight_index = index_of_max(&flex_weights);
            return (initial_item - first_max_weight_index).max(0);
        }
        initial_item
    }

    fn build_carousel_item(self: Handle<Self>, app: &mut App, mut index: i32) -> WidgetRef {
        let widget = self.widget(app);
        // For infinite scrolling, wrap the index to the actual children range.
        if widget.infinite && !widget.children.is_empty() {
            index %= widget.children.len() as i32;
        }
        let context = self.context(app);
        let carousel_theme = CarouselViewTheme::of(app, context);
        let color_scheme = Theme::of(app, context).color_scheme.clone();
        let widget = self.widget(app);
        let effective_padding = widget
            .padding
            .or(carousel_theme.padding)
            .unwrap_or(EdgeInsets::all(4.0));
        let effective_background_color = widget
            .background_color
            .or(carousel_theme.background_color)
            .unwrap_or(color_scheme.surface);
        let effective_elevation = widget.elevation.or(carousel_theme.elevation).unwrap_or(0.0);
        let effective_shape: ShapeBorderRef = widget
            .shape
            .clone()
            .or_else(|| carousel_theme.shape.clone().map(ShapeBorderRef::from))
            .unwrap_or_else(|| {
                ShapeBorderRef::new(RoundedRectangleBorder::new(
                    waft_painting::BorderSide::NONE,
                    BorderRadiusGeometry::circular(28.0),
                ))
            });
        let effective_item_clip_behavior = widget
            .item_clip_behavior
            .or(carousel_theme.item_clip_behavior)
            .unwrap_or(Clip::AntiAlias);
        let effective_overlay_color = widget
            .overlay_color
            .clone()
            .or_else(|| carousel_theme.overlay_color.clone())
            .unwrap_or_else(|| {
                let on_surface = color_scheme.on_surface;
                <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                    if states.contains(&WidgetState::Pressed) {
                        return Some(on_surface.with_alpha(0x1a));
                    }
                    if states.contains(&WidgetState::Hovered) {
                        return Some(on_surface.with_alpha(0x14));
                    }
                    if states.contains(&WidgetState::Focused) {
                        return Some(on_surface.with_alpha(0x1a));
                    }
                    None
                })
            });
        let enable_splash = widget.enable_splash;
        let on_tap = widget.on_tap.clone();
        let mut contents = widget.children[index as usize].clone();

        if enable_splash {
            let on_tap = on_tap.clone();
            let ink_well = InkWell::new()
                .overlay_color(effective_overlay_color)
                .on_tap(Listener::new(move |app: &mut App| {
                    if let Some(on_tap) = &on_tap {
                        on_tap(app, index);
                    }
                }));
            contents = Stack::new()
                .fit(StackFit::Expand)
                .children(vec![
                    contents,
                    Material::new()
                        .color(Colors::TRANSPARENT)
                        .child(ink_well)
                        .into_widget(),
                ])
                .into_widget();
        } else if let Some(on_tap) = on_tap {
            contents = GestureDetector::new()
                .on_tap(Listener::new(move |app: &mut App| on_tap(app, index)))
                .child(contents)
                .into_widget();
        }

        Padding::new(effective_padding.into())
            .child(
                Material::new()
                    .clip_behavior(effective_item_clip_behavior)
                    .color(effective_background_color)
                    .elevation(effective_elevation)
                    .shape(effective_shape)
                    .child(contents),
            )
            .into_widget()
    }

    fn build_sliver_carousel(self: Handle<Self>, app: &mut App) -> WidgetRef {
        let widget = self.widget(app);
        let infinite = widget.infinite;
        let item_builder = widget.item_builder.clone();
        let item_count = widget.item_count;
        let children_len = widget.children.len() as i32;
        let shrink_extent = widget.shrink_extent;
        // Determine the child count and builder based on whether we're using lazy loading
        let child_count = if infinite {
            None
        } else if item_builder.is_some() {
            item_count
        } else {
            Some(children_len)
        };

        let effective_builder: NullableIndexedWidgetBuilder = match item_builder {
            Some(item_builder) => match item_count {
                Some(item_count) if infinite && item_count > 0 => {
                    Rc::new(move |app: &mut App, context: BuildContext, index: i32| {
                        item_builder(app, context, index % item_count)
                    })
                }
                _ => item_builder,
            },
            None => Rc::new(move |app: &mut App, _context: BuildContext, index: i32| {
                Some(self.build_carousel_item(app, index))
            }),
        };
        let mut delegate = SliverChildBuilderDelegate::from_builder(effective_builder);
        if let Some(child_count) = child_count {
            delegate = delegate.child_count(child_count);
        }
        let delegate = delegate.into_delegate();

        if let Some(item_extent) = app.get(self).item_extent {
            return SliverFixedExtentCarousel {
                key: None,
                delegate,
                item_extent,
                min_extent: shrink_extent,
                infinite,
            }
            .into_widget();
        }

        let flex_weights = self.flex_weights(app);
        debug_assert!(
            flex_weights
                .as_ref()
                .is_some_and(|weights| weights.iter().all(|weight| *weight > 0)),
            "flexWeights is null or it contains non-positive integers"
        );
        SliverWeightedCarousel {
            key: None,
            delegate,
            consume_max_weight: self.consume_max_weight(app),
            shrink_extent,
            weights: Rc::new(flex_weights.expect("asserted above")),
            infinite,
        }
        .into_widget()
    }
}

/// The index of the first greatest weight.
fn index_of_max(weights: &[i32]) -> i32 {
    let max_weight = weights.iter().max().expect("the weights are not empty");
    weights
        .iter()
        .position(|weight| weight == max_weight)
        .expect("the maximum is in the list") as i32
}

impl State for CarouselViewState {
    type Widget = CarouselView;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let item_extent = self.widget(app).item_extent;
        app.get_mut(self).item_extent = item_extent;
        if self.widget(app).controller.is_none() {
            let controller = CarouselController::new(app, 0);
            app.get_mut(self).internal_controller = Some(controller);
        }
        let initial_leading_item = self.get_initial_leading_item(app);
        app.get_mut(self).last_reported_leading_item = initial_leading_item;
        let controller = self.controller(app);
        controller.attach_state(app, self);
        controller.add_listener(app, Listener::handle_method(self, Self::handle_scroll));
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &CarouselView) {
        let widget = self.widget(app);
        let new_controller = widget.controller.clone();
        let controller_changed = widget.controller != old_widget.controller;
        let flex_weights_changed = widget.flex_weights != old_widget.flex_weights;
        let item_extent = widget.item_extent;
        let item_extent_changed = widget.item_extent != old_widget.item_extent;
        let consume_max_weight_changed = widget.consume_max_weight != old_widget.consume_max_weight;
        if controller_changed {
            if let Some(old_controller) = &old_widget.controller {
                old_controller.handle().detach_state(app, self);
            }
            if let Some(new_controller) = new_controller {
                if let Some(internal_controller) = app.get(self).internal_controller.handle() {
                    internal_controller.detach_state(app, self);
                }
                app.get_mut(self).internal_controller = None;
                new_controller.handle().attach_state(app, self);
            } else {
                // widget.controller == null && oldWidget.controller != null
                debug_assert!(app.get(self).internal_controller.is_none());
                let controller = CarouselController::new(app, 0);
                app.get_mut(self).internal_controller = Some(controller);
                self.controller(app).attach_state(app, self);
            }
        }
        if flex_weights_changed {
            let flex_weights = self.flex_weights(app);
            self.carousel_position(app)
                .set_flex_weights(app, flex_weights);
        }
        if item_extent_changed {
            app.get_mut(self).item_extent = item_extent;
            self.carousel_position(app)
                .set_item_extent(app, item_extent);
        }
        if consume_max_weight_changed {
            let consume_max_weight = self.consume_max_weight(app);
            self.carousel_position(app)
                .set_consume_max_weight(app, consume_max_weight);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        controller.remove_listener(app, &Listener::handle_method(self, Self::handle_scroll));
        controller.detach_state(app, self);
        if let Some(internal_controller) = app.get(self).internal_controller.handle() {
            ScrollControllerLeaf::dispose(internal_controller, app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let item_snapping = self.widget(app).item_snapping;
        let physics: ScrollPhysicsRef = if item_snapping {
            Rc::new(CarouselScrollPhysics::new())
        } else {
            ScrollConfiguration::of(app, context).get_scroll_physics(app, context)
        };
        let controller = match self.widget(app).controller.clone() {
            Some(controller) => controller,
            None => app
                .get(self)
                .internal_controller
                .clone()
                .expect("an internal controller exists while the widget has none"),
        };

        LayoutBuilder::new(move |app: &mut App, _context: BuildContext, constraints| {
            let widget = self.widget(app);
            let scroll_direction = widget.scroll_direction;
            let reverse = widget.reverse;
            let main_axis_extent = match scroll_direction {
                Axis::Horizontal => constraints.constraints().max_width,
                Axis::Vertical => constraints.constraints().max_height,
            };

            let item_extent = widget
                .item_extent
                .map(|item_extent| clamp_double(item_extent, 0.0, main_axis_extent));
            app.get_mut(self).item_extent = item_extent;
            waft_widgets::CustomScrollView::new()
                .scroll_direction(scroll_direction)
                .reverse(reverse)
                .controller(controller.clone().into_erased())
                .physics(Rc::clone(&physics))
                .clip_behavior(Clip::AntiAlias)
                .scroll_cache_extent(ScrollCacheExtent::Viewport(0.0))
                .slivers(vec![self.build_sliver_carousel(app)])
                .into_widget()
        })
        .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// _SliverFixedExtentCarousel

/// Dart's `_SliverFixedExtentCarousel`.
struct SliverFixedExtentCarousel {
    key: Option<KeyRef>,
    delegate: SliverChildDelegateRef,
    item_extent: f64,
    min_extent: f64,
    infinite: bool,
}

impl RenderObjectWidget for SliverFixedExtentCarousel {
    type RenderObject = RenderSliverFixedExtentCarousel;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
    ) -> waft_rendering::AnyRenderObject {
        RenderSliverFixedExtentCarousel::new(
            app,
            child_manager_of::<SliverFixedExtentCarousel>(app, context),
            self.min_extent,
            self.item_extent,
            self.infinite,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderSliverFixedExtentCarousel>,
    ) {
        render_object.set_max_extent(app, self.item_extent);
        render_object.set_min_extent(app, self.min_extent);
        render_object.set_infinite(app, self.infinite);
    }
}

impl SliverWithKeepAliveWidget for SliverFixedExtentCarousel {}

impl SliverMultiBoxAdaptorWidget for SliverFixedExtentCarousel {
    fn delegate(&self) -> &SliverChildDelegateRef {
        &self.delegate
    }
}

impl Debug for SliverFixedExtentCarousel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_SliverFixedExtentCarousel")
            .field("item_extent", &self.item_extent)
            .field("min_extent", &self.min_extent)
            .field("infinite", &self.infinite)
            .finish_non_exhaustive()
    }
}

/// What `_RenderSliverFixedExtentCarousel` reads of its constraints, taken when the adaptor asks
/// for the item extent builder.
#[derive(Clone, Copy)]
struct FixedCarouselExtents {
    scroll_offset: f64,
    remaining_paint_extent: f64,
    max_extent: f64,
    min_extent: f64,
}

impl FixedCarouselExtents {
    fn of(
        constraints: SliverConstraints,
        max_extent: f64,
        min_extent: f64,
    ) -> FixedCarouselExtents {
        FixedCarouselExtents {
            scroll_offset: constraints.scroll_offset,
            remaining_paint_extent: constraints.remaining_paint_extent,
            max_extent,
            min_extent,
        }
    }

    // This implements the [itemExtentBuilder] callback.
    fn build_item_extent(&self, index: i32) -> f64 {
        if self.max_extent == 0.0 {
            return self.max_extent;
        }

        let first_visible_index = (self.scroll_offset / self.max_extent).floor() as i32;

        // Calculate how many items have been completely scroll off screen.
        let offscreen_items = (self.scroll_offset / self.max_extent).floor();

        // If an item is partially off screen and partially on screen,
        // `constraints.scrollOffset` must be greater than
        // `offscreenItems * maxExtent`, so the difference between these two is how
        // much the current first visible item is off screen.
        let offscreen_extent = self.scroll_offset - offscreen_items * self.max_extent;

        // If there is not enough space to place the last visible item but the remaining
        // space is larger than `minExtent`, the extent for last item should be at
        // least the remaining extent to ensure a smooth size transition.
        let effective_min_extent =
            dart_remainder(self.remaining_paint_extent, self.max_extent).max(self.min_extent);

        // Two special cases are the first and last visible items. Other items' extent
        // should all return `maxExtent`.
        if index == first_visible_index {
            let effective_extent = self.max_extent - offscreen_extent;
            return effective_extent.max(effective_min_extent);
        }

        let scroll_offset_for_last_index = self.scroll_offset + self.remaining_paint_extent;
        if index == self.max_child_index_for_scroll_offset(scroll_offset_for_last_index) {
            return clamp_double(
                scroll_offset_for_last_index - self.max_extent * f64::from(index),
                effective_min_extent,
                self.max_extent,
            );
        }

        self.max_extent
    }

    fn index_to_layout_offset(&self, index: i32) -> f64 {
        if self.max_extent == 0.0 {
            return self.max_extent;
        }

        let first_visible_index = (self.scroll_offset / self.max_extent).floor() as i32;

        // If there is not enough space to place the last visible item but the remaining
        // space is larger than `minExtent`, the extent for last item should be at
        // least the remaining extent to make sure a smooth size transition.
        let effective_min_extent =
            dart_remainder(self.remaining_paint_extent, self.max_extent).max(self.min_extent);
        if index == first_visible_index {
            let first_visible_item_extent = self.build_item_extent(index);

            // If the first item is collapsed to be less than `effectiveMinExtent`,
            // then it should stop changing its size and should start to scroll off screen.
            if first_visible_item_extent <= effective_min_extent {
                return self.max_extent * f64::from(index) - effective_min_extent + self.max_extent;
            }
            return self.scroll_offset;
        }
        self.max_extent * f64::from(index)
    }

    fn min_child_index_for_scroll_offset(&self, scroll_offset: f64) -> i32 {
        if self.max_extent == 0.0 {
            return 0;
        }

        let first_visible_index = (scroll_offset / self.max_extent).floor() as i32;
        first_visible_index.max(0)
    }

    fn max_child_index_for_scroll_offset(&self, scroll_offset: f64) -> i32 {
        if self.max_extent > 0.0 {
            let actual = scroll_offset / self.max_extent - 1.0;
            let round = actual.round();
            if (actual * self.max_extent - round * self.max_extent).abs()
                < PRECISION_ERROR_TOLERANCE
            {
                return (round as i32).max(0);
            }
            return (actual.ceil() as i32).max(0);
        }
        0
    }
}

/// Dart's `%` on doubles: the remainder, with the sign of the dividend.
fn dart_remainder(dividend: f64, divisor: f64) -> f64 {
    let result = dividend % divisor;
    if result == 0.0 { 0.0 } else { result }
}

/// Dart's `_RenderSliverFixedExtentCarousel`.
struct RenderSliverFixedExtentCarousel {
    render_object: RenderObjectData,
    render_sliver: RenderSliverData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    adaptor: RenderSliverMultiBoxAdaptorData,
    fixed_extent: RenderSliverFixedExtentBoxAdaptorData,
    max_extent: f64,
    min_extent: f64,
    infinite: bool,
}

impl RenderSliverFixedExtentCarousel {
    fn new(
        app: &mut App,
        child_manager: Rc<dyn RenderSliverBoxChildManager>,
        min_extent: f64,
        max_extent: f64,
        infinite: bool,
    ) -> RenderHandle<RenderSliverFixedExtentCarousel> {
        RenderHandle::new_sliver(
            app,
            RenderSliverFixedExtentCarousel {
                render_object: RenderObjectData::new(),
                render_sliver: RenderSliverData::new(),
                container: ContainerRenderObjectData::new(),
                adaptor: RenderSliverMultiBoxAdaptorData::new(child_manager),
                fixed_extent: RenderSliverFixedExtentBoxAdaptorData::new(),
                max_extent,
                min_extent,
                infinite,
            },
        )
    }

    fn set_max_extent(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).max_extent == value {
            return;
        }
        self.get_mut(app).max_extent = value;
        self.mark_needs_layout(app);
    }

    fn set_min_extent(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).min_extent == value {
            return;
        }
        self.get_mut(app).min_extent = value;
        self.mark_needs_layout(app);
    }

    fn set_infinite(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).infinite == value {
            return;
        }
        self.get_mut(app).infinite = value;
        self.mark_needs_layout(app);
    }

    fn extents(self: RenderHandle<Self>, app: &App) -> FixedCarouselExtents {
        let this = self.get(app);
        FixedCarouselExtents::of(self.constraints(app), this.max_extent, this.min_extent)
    }
}

impl RenderSliverFixedExtentBoxAdaptor for RenderSliverFixedExtentCarousel {
    render_sliver_fixed_extent_accessors!();

    fn item_extent(self: RenderHandle<Self>, _app: &App) -> Option<f64> {
        None
    }

    fn item_extent_builder(self: RenderHandle<Self>, app: &App) -> Option<ItemExtentBuilder> {
        let extents = self.extents(app);
        Some(Rc::new(
            move |index: i32, _dimensions: SliverLayoutDimensions| {
                Some(extents.build_item_extent(index))
            },
        ))
    }

    fn index_to_layout_offset(self: RenderHandle<Self>, app: &App, index: i32) -> f64 {
        self.extents(app).index_to_layout_offset(index)
    }

    fn get_min_child_index_for_scroll_offset(
        self: RenderHandle<Self>,
        app: &App,
        scroll_offset: f64,
    ) -> i32 {
        self.extents(app)
            .min_child_index_for_scroll_offset(scroll_offset)
    }

    fn get_max_child_index_for_scroll_offset(
        self: RenderHandle<Self>,
        app: &App,
        scroll_offset: f64,
    ) -> i32 {
        self.extents(app)
            .max_child_index_for_scroll_offset(scroll_offset)
    }
}

fixed_extent_box_adaptor_leaf!(RenderSliverFixedExtentCarousel);

// ---------------------------------------------------------------------------------------------
// _SliverWeightedCarousel

/// Dart's `_SliverWeightedCarousel`.
struct SliverWeightedCarousel {
    key: Option<KeyRef>,
    delegate: SliverChildDelegateRef,
    // Determine whether extra scroll offset should be calculate so that every
    // item have a chance to scroll to the maximum extent.
    //
    // This is useful when the leading/trailing items have smaller weights, such
    // as [1, 7], and [3, 2, 1].
    consume_max_weight: bool,
    // The starting extent for items when they gradually show on/off screen.
    //
    // This is useful to avoid a hairline shape. This value should also smaller
    // than the last item extent to make sure a smooth transition. So in calculation,
    // this is limited to [0, weight for the last visible item].
    shrink_extent: f64,
    // The layout arrangement.
    //
    // When items are laying out, each item will be arranged based on the order of
    // the weights and the extent is based on the corresponding weight out of the
    // sum of weights. The length of weights means how many items we can put in the
    // view at a time.
    weights: Rc<Vec<i32>>,
    // Whether the carousel should loop infinitely.
    infinite: bool,
}

impl RenderObjectWidget for SliverWeightedCarousel {
    type RenderObject = RenderSliverWeightedCarousel;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
    ) -> waft_rendering::AnyRenderObject {
        RenderSliverWeightedCarousel::new(
            app,
            child_manager_of::<SliverWeightedCarousel>(app, context),
            self.consume_max_weight,
            self.shrink_extent,
            Rc::clone(&self.weights),
            self.infinite,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderSliverWeightedCarousel>,
    ) {
        render_object.set_consume_max_weight(app, self.consume_max_weight);
        render_object.set_shrink_extent(app, self.shrink_extent);
        render_object.set_weights(app, Rc::clone(&self.weights));
        render_object.set_infinite(app, self.infinite);
    }
}

impl SliverWithKeepAliveWidget for SliverWeightedCarousel {}

impl SliverMultiBoxAdaptorWidget for SliverWeightedCarousel {
    fn delegate(&self) -> &SliverChildDelegateRef {
        &self.delegate
    }
}

impl Debug for SliverWeightedCarousel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_SliverWeightedCarousel")
            .field("weights", &self.weights)
            .field("consume_max_weight", &self.consume_max_weight)
            .field("shrink_extent", &self.shrink_extent)
            .field("infinite", &self.infinite)
            .finish_non_exhaustive()
    }
}

/// What `_RenderSliverWeightedCarousel` reads of its constraints and fields, taken when the
/// adaptor asks for the item extent builder.
#[derive(Clone)]
struct WeightedCarouselExtents {
    scroll_offset: f64,
    remaining_paint_extent: f64,
    viewport_main_axis_extent: f64,
    consume_max_weight: bool,
    shrink_extent: f64,
    weights: Rc<Vec<i32>>,
    infinite: bool,
    child_count: Option<i32>,
}

impl WeightedCarouselExtents {
    fn max_weight(&self) -> i32 {
        *self
            .weights
            .iter()
            .max()
            .expect("the weights are not empty")
    }

    fn min_weight(&self) -> i32 {
        *self
            .weights
            .iter()
            .min()
            .expect("the weights are not empty")
    }

    // To ge the extent unit based on the viewport extent and the sum of weights.
    fn extent_unit(&self) -> f64 {
        self.viewport_main_axis_extent / f64::from(self.weights.iter().sum::<i32>())
    }

    fn first_child_extent(&self) -> f64 {
        f64::from(self.weights[0]) * self.extent_unit()
    }

    fn max_child_extent(&self) -> f64 {
        f64::from(self.max_weight()) * self.extent_unit()
    }

    fn min_child_extent(&self) -> f64 {
        f64::from(self.min_weight()) * self.extent_unit()
    }

    // The shrink extent for first and last visible items should be no larger
    // than [minChildExtent] to ensure a smooth transition.
    fn effective_shrink_extent(&self) -> f64 {
        clamp_double(self.shrink_extent, 0.0, self.min_child_extent())
    }

    // The scroll offset in whole first-child extents, rounded the way the position rounds items.
    fn scroll_in_first_children(&self) -> i32 {
        let actual = self.scroll_offset / self.first_child_extent();
        let round = actual.round();
        if (actual - round).abs() < PRECISION_ERROR_TOLERANCE {
            round as i32
        } else {
            actual.floor() as i32
        }
    }

    // The index of the first visible item. The returned value can be negative when
    // the leading items with smaller weights need to be fully expanded. For example,
    // assuming a weights [1, 7, 1], when item 0 is expanding to the maximum size
    // (with weight 7), we leave some space before item 0 assuming there is another
    // item -1 as the first visible item.
    fn first_visible_item_index(&self) -> i32 {
        // If constraints.viewportMainAxisExtent is 0, firstChildExtent will be 0 and cause
        // division error.
        if self.viewport_main_axis_extent == 0.0 {
            return 0;
        }
        let max_weight = self.max_weight();
        let mut smaller_weight_count = 0;
        for weight in self.weights.iter() {
            if *weight == max_weight {
                break;
            }
            smaller_weight_count += 1;
        }

        let index = self.scroll_in_first_children();
        if self.consume_max_weight {
            index - smaller_weight_count
        } else {
            index
        }
    }

    // This value indicates the scrolling progress of items following the first
    // item. It informs them how much the first item has moved off-screen,
    // enabling them to adjust their sizes (grow or shrink) accordingly.
    fn first_visible_item_offscreen_extent(&self) -> f64 {
        // If constraints.viewportMainAxisExtent is 0, firstChildExtent will be 0 and cause
        // division error.
        if self.viewport_main_axis_extent == 0.0 {
            return 0.0;
        }
        self.scroll_offset - f64::from(self.scroll_in_first_children()) * self.first_child_extent()
    }

    // Given the off-screen extent for the first visible item, we can know the
    // on-screen extent for the first visible item.
    fn distance_to_leading_edge(&self) -> f64 {
        self.first_child_extent() - self.first_visible_item_offscreen_extent()
    }

    // This is to implement the itemExtentBuilder callback to return each item extent
    // while scrolling.
    //
    // The given `index` is compared with `_firstVisibleItemIndex` to know how
    // many items are placed before the current one in the view.
    fn build_item_extent(&self, index: i32) -> f64 {
        // If constraints.viewportMainAxisExtent is 0, firstChildExtent will be 0 and cause
        // division error.
        if self.viewport_main_axis_extent == 0.0 {
            return 0.0;
        }

        let first_visible_item_index = self.first_visible_item_index();
        if index == first_visible_item_index {
            return self
                .distance_to_leading_edge()
                .max(self.effective_shrink_extent());
        }
        // Calculate the extents of items located within the range defined by the
        // weights array relative to the first visible item. This allows us to
        // precisely determine each item's extent based on its initial extent
        // (calculated from the weights) and the scrolling progress (the off-screen
        // portion of the first item).
        if index > first_visible_item_index
            && index - first_visible_item_index < self.weights.len() as i32
        {
            debug_assert!(index - first_visible_item_index < self.weights.len() as i32);
            let curr_index_on_weight_list = (index - first_visible_item_index) as usize;
            let curr_weight = self.weights[curr_index_on_weight_list];
            let initial_extent = self.extent_unit() * f64::from(curr_weight); // initial extent
            let progress = self.first_visible_item_offscreen_extent() / self.first_child_extent();

            let prev_weight = self.weights[curr_index_on_weight_list - 1];
            let final_increase =
                f64::from(prev_weight - curr_weight) / f64::from(self.max_weight());
            return initial_extent + final_increase * progress * self.max_child_extent();
        }
        // Calculate the extents of items located beyond the range defined by the
        // weights array relative to the first visible item. During scrolling transition,
        // it is possible that the number of visible items is larger than the length
        // of `weights`. The extra item extent should be calculated here to fill
        // the remaining space.
        if index > first_visible_item_index
            && index - first_visible_item_index + 1 > self.weights.len() as i32
        {
            let mut visible_items_total_extent = self.distance_to_leading_edge();
            for i in (first_visible_item_index + 1)..index {
                visible_items_total_extent += self.build_item_extent(i);
            }
            return (self.remaining_paint_extent - visible_items_total_extent)
                .max(self.effective_shrink_extent());
        }
        self.min_child_extent().max(self.effective_shrink_extent())
    }

    // Given an index, this method returns the layout offset for the item. The `index`
    // is firstly compared to `_firstVisibleItemIndex` and compute the distance
    // between them, then compute all the current extents for items that are located
    // in front.
    fn index_to_layout_offset(&self, index: i32) -> f64 {
        let first_visible_item_index = self.first_visible_item_index();
        if index == first_visible_item_index {
            if self.distance_to_leading_edge() <= self.effective_shrink_extent() {
                return self.scroll_offset - self.effective_shrink_extent()
                    + self.distance_to_leading_edge();
            }
            return self.scroll_offset;
        }
        let mut visible_items_total_extent = self.distance_to_leading_edge();
        for i in (first_visible_item_index + 1)..index {
            visible_items_total_extent += self.build_item_extent(i);
        }
        self.scroll_offset + visible_items_total_extent
    }

    fn max_child_index_for_scroll_offset(&self) -> i32 {
        let child_count = self.child_count;

        // For infinite scrolling, calculate how many items fit in the viewport
        if self.infinite && child_count.is_none() {
            let mut visible_items_total_extent = self.distance_to_leading_edge();
            let mut index = self.first_visible_item_index() + 1;
            // Calculate upper bound based on viewport extent and minimum possible item extent.
            // In worst case, all items would be at minimum extent i.e. minChildExtent.
            let safe_min_extent = self.min_child_extent().max(1.0);
            let estimated_upper_bound = self.first_visible_item_index()
                + (self.viewport_main_axis_extent / safe_min_extent).ceil() as i32;
            while visible_items_total_extent < self.viewport_main_axis_extent
                && index < estimated_upper_bound
            {
                visible_items_total_extent += self.build_item_extent(index);
                if visible_items_total_extent >= self.viewport_main_axis_extent {
                    return index;
                }
                index += 1;
            }
            return index;
        }

        if let Some(child_count) = child_count {
            let mut visible_items_total_extent = self.distance_to_leading_edge();
            for i in (self.first_visible_item_index() + 1)..child_count {
                visible_items_total_extent += self.build_item_extent(i);
                if visible_items_total_extent >= self.viewport_main_axis_extent {
                    return i;
                }
            }
        }
        child_count.unwrap_or(0)
    }
}

/// Dart's `_RenderSliverWeightedCarousel`: a sliver that places its box children in a linear
/// array and constrains them to have the corresponding weight which is determined by its
/// weights.
struct RenderSliverWeightedCarousel {
    render_object: RenderObjectData,
    render_sliver: RenderSliverData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    adaptor: RenderSliverMultiBoxAdaptorData,
    fixed_extent: RenderSliverFixedExtentBoxAdaptorData,
    consume_max_weight: bool,
    shrink_extent: f64,
    weights: Rc<Vec<i32>>,
    infinite: bool,
}

impl RenderSliverWeightedCarousel {
    fn new(
        app: &mut App,
        child_manager: Rc<dyn RenderSliverBoxChildManager>,
        consume_max_weight: bool,
        shrink_extent: f64,
        weights: Rc<Vec<i32>>,
        infinite: bool,
    ) -> RenderHandle<RenderSliverWeightedCarousel> {
        RenderHandle::new_sliver(
            app,
            RenderSliverWeightedCarousel {
                render_object: RenderObjectData::new(),
                render_sliver: RenderSliverData::new(),
                container: ContainerRenderObjectData::new(),
                adaptor: RenderSliverMultiBoxAdaptorData::new(child_manager),
                fixed_extent: RenderSliverFixedExtentBoxAdaptorData::new(),
                consume_max_weight,
                shrink_extent,
                weights,
                infinite,
            },
        )
    }

    fn set_consume_max_weight(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).consume_max_weight == value {
            return;
        }
        self.get_mut(app).consume_max_weight = value;
        self.mark_needs_layout(app);
    }

    fn set_shrink_extent(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).shrink_extent == value {
            return;
        }
        self.get_mut(app).shrink_extent = value;
        self.mark_needs_layout(app);
    }

    fn set_weights(self: RenderHandle<Self>, app: &mut App, value: Rc<Vec<i32>>) {
        if *self.get(app).weights == *value {
            return;
        }
        self.get_mut(app).weights = value;
        self.mark_needs_layout(app);
    }

    fn set_infinite(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).infinite == value {
            return;
        }
        self.get_mut(app).infinite = value;
        self.mark_needs_layout(app);
    }

    fn extents(self: RenderHandle<Self>, app: &App) -> WeightedCarouselExtents {
        let this = self.get(app);
        let constraints = self.constraints(app);
        WeightedCarouselExtents {
            scroll_offset: constraints.scroll_offset,
            remaining_paint_extent: constraints.remaining_paint_extent,
            viewport_main_axis_extent: constraints.viewport_main_axis_extent,
            consume_max_weight: this.consume_max_weight,
            shrink_extent: this.shrink_extent,
            weights: Rc::clone(&this.weights),
            infinite: this.infinite,
            child_count: self.child_manager(app).estimated_child_count(app),
        }
    }
}

impl RenderSliverFixedExtentBoxAdaptor for RenderSliverWeightedCarousel {
    render_sliver_fixed_extent_accessors!();

    fn item_extent(self: RenderHandle<Self>, _app: &App) -> Option<f64> {
        None
    }

    fn item_extent_builder(self: RenderHandle<Self>, app: &App) -> Option<ItemExtentBuilder> {
        let extents = self.extents(app);
        Some(Rc::new(
            move |index: i32, _dimensions: SliverLayoutDimensions| {
                Some(extents.build_item_extent(index))
            },
        ))
    }

    fn index_to_layout_offset(self: RenderHandle<Self>, app: &App, index: i32) -> f64 {
        self.extents(app).index_to_layout_offset(index)
    }

    fn get_min_child_index_for_scroll_offset(
        self: RenderHandle<Self>,
        app: &App,
        _scroll_offset: f64,
    ) -> i32 {
        self.extents(app).first_visible_item_index().max(0)
    }

    fn get_max_child_index_for_scroll_offset(
        self: RenderHandle<Self>,
        app: &App,
        _scroll_offset: f64,
    ) -> i32 {
        self.extents(app).max_child_index_for_scroll_offset()
    }

    fn compute_max_scroll_offset(
        self: RenderHandle<Self>,
        app: &mut App,
        _constraints: SliverConstraints,
    ) -> f64 {
        if self.get(app).infinite {
            return f64::INFINITY;
        }
        let child_count = self.child_manager(app).child_count(app);
        f64::from(child_count) * self.extents(app).max_child_extent()
    }

    // This method is mostly the same as its parent class [RenderSliverFixedExtentList].
    // The difference is when we allow some space before the leading items or after
    // the trailing items with smaller weights, we leave extra scroll offset.
    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let child_manager = self.child_manager(app);
        child_manager.did_start_layout(app);
        child_manager.set_did_underflow(app, false);

        let scroll_offset = constraints.scroll_offset + constraints.cache_origin;
        debug_assert!(scroll_offset >= 0.0);
        let remaining_extent = constraints.remaining_cache_extent;
        debug_assert!(remaining_extent >= 0.0);
        let target_end_scroll_offset = scroll_offset + remaining_extent;

        let first_index = self.get_min_child_index_for_scroll_offset(app, scroll_offset);
        let target_last_index = target_end_scroll_offset
            .is_finite()
            .then(|| self.get_max_child_index_for_scroll_offset(app, target_end_scroll_offset));

        if self.first_child(app).is_some() {
            let leading_garbage = self.calculate_leading_garbage(app, first_index);
            let trailing_garbage = match target_last_index {
                Some(last_index) => self.calculate_trailing_garbage(app, last_index),
                None => 0,
            };
            self.collect_garbage(app, leading_garbage, trailing_garbage);
        } else {
            self.collect_garbage(app, 0, 0);
        }

        if self.first_child(app).is_none() {
            let layout_offset = self.index_to_layout_offset(app, first_index);
            if !self.add_initial_child(app, first_index, layout_offset) {
                // There are either no children, or we are past the end of all our children.
                let max = if first_index <= 0 {
                    0.0
                } else {
                    self.compute_max_scroll_offset(app, constraints)
                };
                self.set_geometry(
                    app,
                    SliverGeometry::builder()
                        .scroll_extent(max)
                        .max_paint_extent(max)
                        .build(),
                );
                child_manager.did_finish_layout(app);
                return;
            }
        }

        let mut trailing_child_with_layout: Option<AnyRenderBox> = None;

        let first_child = self.first_child(app).expect("added above");
        let mut index = self.index_of(app, first_child) - 1;
        while index >= first_index {
            let child_constraints = self.child_constraints(app, index);
            let Some(child) = self.insert_and_layout_leading_child(app, child_constraints, false)
            else {
                // Items before the previously first child are no longer present. Reset the
                // scroll offset to offset all items prior and up to the missing item. Let the
                // parent re-lay-out everything.
                let correction = self.index_to_layout_offset(app, index);
                self.set_geometry(
                    app,
                    SliverGeometry::builder()
                        .scroll_offset_correction(correction)
                        .build(),
                );
                return;
            };
            let layout_offset = self.index_to_layout_offset(app, index);
            let child_parent_data = child
                .as_object()
                .parent_data_of_mut::<SliverMultiBoxAdaptorParentData>(app);
            child_parent_data.set_layout_offset(Some(layout_offset));
            debug_assert_eq!(child_parent_data.index, Some(index));
            trailing_child_with_layout.get_or_insert(child);
            index -= 1;
        }

        if trailing_child_with_layout.is_none() {
            let first_child = self.first_child(app).expect("the child list is not empty");
            let child_constraints = self.child_constraints(app, self.index_of(app, first_child));
            first_child.layout(app, child_constraints, false);
            let layout_offset = self.index_to_layout_offset(app, first_index);
            first_child
                .as_object()
                .parent_data_of_mut::<SliverMultiBoxAdaptorParentData>(app)
                .set_layout_offset(Some(layout_offset));
            trailing_child_with_layout = Some(first_child);
        }

        // From the last item to the firstly encountered max item
        let extents = self.extents(app);
        let mut extra_layout_offset = 0.0;
        if extents.consume_max_weight {
            let max_weight = extents.max_weight();
            for weight in extents.weights.iter().rev() {
                if *weight == max_weight {
                    break;
                }
                extra_layout_offset += f64::from(*weight) * extents.extent_unit();
            }
        }

        let mut estimated_max_scroll_offset = f64::INFINITY;
        // Layout visible items after the first visible item.
        let mut index = self.index_of(app, trailing_child_with_layout.expect("set above")) + 1;
        while target_last_index.is_none_or(|last_index| index <= last_index) {
            let after = trailing_child_with_layout.expect("set above");
            let mut child = self.child_after(app, after);
            if child.is_none_or(|current| self.index_of(app, current) != index) {
                let child_constraints = self.child_constraints(app, index);
                child = self.insert_and_layout_child(app, child_constraints, Some(after), false);
                if child.is_none() {
                    // We have run out of children.
                    estimated_max_scroll_offset =
                        self.index_to_layout_offset(app, index) + extra_layout_offset;
                    break;
                }
            } else {
                let child_constraints = self.child_constraints(app, index);
                child
                    .expect("checked")
                    .layout(app, child_constraints, false);
            }
            let child = child.expect("set above");
            trailing_child_with_layout = Some(child);
            debug_assert_eq!(
                child
                    .as_object()
                    .parent_data_of::<SliverMultiBoxAdaptorParentData>(app)
                    .index,
                Some(index)
            );
            let layout_offset = self.index_to_layout_offset(app, index);
            child
                .as_object()
                .parent_data_of_mut::<SliverMultiBoxAdaptorParentData>(app)
                .set_layout_offset(Some(layout_offset));
            index += 1;
        }

        let last_child = self.last_child(app).expect("the child list is not empty");
        let last_index = self.index_of(app, last_child);
        let leading_scroll_offset = self.index_to_layout_offset(app, first_index);
        let infinite = self.get(app).infinite;
        let trailing_scroll_offset;

        if !infinite && last_index + 1 == child_manager.child_count(app) {
            let mut trailing = self.index_to_layout_offset(app, last_index);

            trailing += (f64::from(*extents.weights.last().expect("not empty"))
                * extents.extent_unit())
            .max(extents.build_item_extent(last_index));
            trailing += extra_layout_offset;
            estimated_max_scroll_offset = trailing;
            trailing_scroll_offset = trailing;
        } else {
            trailing_scroll_offset = self.index_to_layout_offset(app, last_index + 1);
        }

        debug_assert!(self.debug_assert_child_list_is_non_empty_and_contiguous(app));
        debug_assert_eq!(
            self.index_of(app, self.first_child(app).expect("not empty")),
            first_index
        );
        debug_assert!(target_last_index.is_none_or(|target| last_index <= target));

        estimated_max_scroll_offset =
            estimated_max_scroll_offset.min(self.estimate_max_scroll_offset(
                app,
                constraints,
                Some(first_index),
                Some(last_index),
                Some(leading_scroll_offset),
                Some(trailing_scroll_offset),
            ));

        let from = if extents.consume_max_weight {
            0.0
        } else {
            leading_scroll_offset
        };
        let paint_extent =
            self.calculate_paint_offset(app, constraints, from, trailing_scroll_offset);
        let cache_extent =
            self.calculate_cache_offset(app, constraints, from, trailing_scroll_offset);

        let target_end_scroll_offset_for_paint =
            constraints.scroll_offset + constraints.remaining_paint_extent;
        let target_last_index_for_paint =
            target_end_scroll_offset_for_paint.is_finite().then(|| {
                self.get_max_child_index_for_scroll_offset(app, target_end_scroll_offset_for_paint)
            });

        self.set_geometry(
            app,
            SliverGeometry::builder()
                .scroll_extent(estimated_max_scroll_offset)
                .paint_extent(paint_extent)
                .cache_extent(cache_extent)
                .max_paint_extent(estimated_max_scroll_offset)
                // Conservative to avoid flickering away the clip during scroll.
                .has_visual_overflow(
                    target_last_index_for_paint.is_some_and(|target| last_index >= target)
                        || constraints.scroll_offset > 0.0,
                )
                .build(),
        );

        // We may have started the layout while scrolled to the end, which would not expose a new
        // child.
        if estimated_max_scroll_offset == trailing_scroll_offset {
            child_manager.set_did_underflow(app, true);
        }
        child_manager.did_finish_layout(app);
    }
}

fixed_extent_box_adaptor_leaf!(RenderSliverWeightedCarousel);

// ---------------------------------------------------------------------------------------------
// CarouselScrollPhysics

/// Scroll physics used by a [`CarouselView`].
///
/// These physics cause the carousel to snap to item boundaries.
///
/// See also:
///
///  * [`ScrollPhysics`], the base trait which defines the API for scrolling physics.
///  * `PageScrollPhysics`, scroll physics used by a `PageView`.
#[derive(Default)]
pub struct CarouselScrollPhysics {
    /// See [`ScrollPhysics::parent`].
    pub parent: Option<ScrollPhysicsRef>,
}

impl CarouselScrollPhysics {
    /// Creates physics for a [`CarouselView`].
    pub fn new() -> CarouselScrollPhysics {
        CarouselScrollPhysics::default()
    }

    /// Dart `CarouselScrollPhysics(parent:)`.
    pub fn with_parent(mut self, parent: ScrollPhysicsRef) -> CarouselScrollPhysics {
        self.parent = Some(parent);
        self
    }

    fn get_target_pixels(
        &self,
        position: &CarouselMetrics,
        tolerance: Tolerance,
        velocity: f64,
    ) -> f64 {
        let fraction = match position.item_extent {
            Some(item_extent) => item_extent / position.viewport_dimension(),
            None => {
                let flex_weights = position
                    .flex_weights
                    .as_ref()
                    .expect("a carousel position has an item extent or flex weights");
                f64::from(flex_weights[0]) / f64::from(flex_weights.iter().sum::<i32>())
            }
        };

        let item_width = position.viewport_dimension() * fraction;

        let actual = position.pixels().max(0.0) / item_width;
        let round = actual.round();
        let mut item = if (actual - round).abs() < PRECISION_ERROR_TOLERANCE {
            round
        } else {
            actual
        };
        if velocity < -tolerance.velocity {
            item -= 0.5;
        } else if velocity > tolerance.velocity {
            item += 0.5;
        }
        item.round() * item_width
    }
}

impl ScrollPhysics for CarouselScrollPhysics {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn parent(&self) -> Option<&ScrollPhysicsRef> {
        self.parent.as_ref()
    }

    fn apply_to(&self, ancestor: Option<ScrollPhysicsRef>) -> ScrollPhysicsRef {
        Rc::new(CarouselScrollPhysics {
            parent: ScrollPhysicsBase::build_parent(self, ancestor),
        })
    }

    fn create_ballistic_simulation(
        &self,
        position: &dyn ScrollMetrics,
        velocity: f64,
    ) -> Option<Box<dyn Simulation>> {
        let metrics = position.as_any().downcast_ref::<CarouselMetrics>().expect(
            "CarouselScrollPhysics can only be used with Scrollables that uses the \
                 CarouselController",
        );
        if (velocity <= 0.0 && metrics.pixels() <= metrics.min_scroll_extent())
            || (velocity >= 0.0 && metrics.pixels() >= metrics.max_scroll_extent())
        {
            return ScrollPhysicsBase::create_ballistic_simulation(self, metrics, velocity);
        }

        let tolerance = self.tolerance_for(metrics);
        let target = self.get_target_pixels(metrics, tolerance, velocity);
        if target != metrics.pixels() {
            return Some(Box::new(ScrollSpringSimulation::new(
                self.spring(),
                metrics.pixels(),
                target,
                velocity,
                tolerance,
            )));
        }
        None
    }

    fn allow_implicit_scrolling(&self) -> bool {
        true
    }
}

impl Debug for CarouselScrollPhysics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.parent {
            None => write!(f, "CarouselScrollPhysics"),
            Some(parent) => write!(f, "CarouselScrollPhysics -> {parent:?}"),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// _CarouselMetrics

/// Dart's `_CarouselMetrics`: the snapshot a carousel's position hands the physics, which
/// answers Dart's `position is _CarouselPosition`.
#[derive(Clone)]
struct CarouselMetrics {
    fixed: FixedScrollMetrics,
    item_extent: Option<f64>,
    flex_weights: Option<Vec<i32>>,
    /// Dart's `consumeMaxWeight`, which the physics do not read.
    #[allow(dead_code)]
    consume_max_weight: Option<bool>,
}

impl ScrollMetrics for CarouselMetrics {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn min_scroll_extent(&self) -> f64 {
        self.fixed.min_scroll_extent()
    }

    fn max_scroll_extent(&self) -> f64 {
        self.fixed.max_scroll_extent()
    }

    fn has_content_dimensions(&self) -> bool {
        self.fixed.has_content_dimensions()
    }

    fn pixels(&self) -> f64 {
        self.fixed.pixels()
    }

    fn has_pixels(&self) -> bool {
        self.fixed.has_pixels()
    }

    fn viewport_dimension(&self) -> f64 {
        self.fixed.viewport_dimension()
    }

    fn has_viewport_dimension(&self) -> bool {
        self.fixed.has_viewport_dimension()
    }

    fn axis_direction(&self) -> AxisDirection {
        self.fixed.axis_direction()
    }

    fn device_pixel_ratio(&self) -> f64 {
        self.fixed.device_pixel_ratio()
    }
}

impl Debug for CarouselMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CarouselMetrics({:.1}..[{:.1}]..{:.1})",
            self.extent_before(),
            self.extent_inside(),
            self.extent_after()
        )
    }
}

// ---------------------------------------------------------------------------------------------
// _CarouselPosition

/// Dart's `_CarouselPosition`: a [`ScrollPositionWithSingleContextLeaf`] that keeps its offset in
/// items.
struct CarouselPosition {
    change_notifier: ChangeNotifierData,
    scroll_position: ScrollPositionData,
    single_context: ScrollPositionWithSingleContextData,
    item_to_show_on_startup: f64,
    item_count: Option<i32>,
    infinite: bool,
    // When the viewport has a zero-size, the item can not
    // be retrieved by `getItemFromPixels`, so we need to cache the item
    // for use when resizing the viewport to non-zero next time.
    cached_item: Option<f64>,
    consume_max_weight: bool,
    item_extent: Option<f64>,
    flex_weights: Option<Vec<i32>>,
}

impl CarouselPosition {
    fn new(
        app: &mut App,
        physics: ScrollPhysicsRef,
        context: Rc<dyn ScrollContext>,
        initial_item: i32,
        item_extent: Option<f64>,
        flex_weights: Option<Vec<i32>>,
        consume_max_weight: bool,
        infinite: bool,
        item_count: Option<i32>,
        old_position: Option<AnyScrollPosition>,
    ) -> Ref<CarouselPosition> {
        debug_assert!(
            (flex_weights.is_some() && item_extent.is_none())
                || (flex_weights.is_none() && item_extent.is_some())
        );
        let scroll_position = ScrollPositionData::new(app, physics, context, true, None);
        let this = app.create(CarouselPosition {
            change_notifier: ChangeNotifierData::new(),
            scroll_position,
            single_context: ScrollPositionWithSingleContextData::new(),
            item_to_show_on_startup: f64::from(initial_item),
            item_count,
            infinite,
            cached_item: None,
            consume_max_weight,
            item_extent,
            flex_weights,
        });
        ScrollPositionWithSingleContext::init(this.handle(), app, None, old_position);
        this
    }

    fn set_item_count(self: Handle<Self>, app: &mut App, value: Option<i32>) {
        if app.get(self).item_count == value {
            return;
        }
        app.get_mut(self).item_count = value;
    }

    fn set_infinite(self: Handle<Self>, app: &mut App, value: bool) {
        if app.get(self).infinite == value {
            return;
        }
        app.get_mut(self).infinite = value;
    }

    fn set_consume_max_weight(self: Handle<Self>, app: &mut App, value: bool) {
        if app.get(self).consume_max_weight == value {
            return;
        }
        if self.has_pixels(app) && app.get(self).flex_weights.is_some() {
            let flex_weights = app.get(self).flex_weights.clone();
            let leading_item = self.update_leading_item(app, flex_weights.as_deref(), value);
            let item_extent = app.get(self).item_extent;
            let new_pixel =
                self.get_pixels_from_item(app, leading_item, flex_weights.as_deref(), item_extent);
            self.force_pixels(app, new_pixel);
        }
        app.get_mut(self).consume_max_weight = value;
    }

    fn set_item_extent(self: Handle<Self>, app: &mut App, value: Option<f64>) {
        if app.get(self).item_extent == value {
            return;
        }
        if self.has_pixels(app)
            && app.get(self).item_extent.is_some()
            && self.viewport_dimension(app) != 0.0
        {
            let leading_item =
                self.get_item_from_pixels(app, self.pixels(app), self.viewport_dimension(app));
            let flex_weights = app.get(self).flex_weights.clone();
            let new_pixel =
                self.get_pixels_from_item(app, leading_item, flex_weights.as_deref(), value);
            self.force_pixels(app, new_pixel);
        }
        app.get_mut(self).item_extent = value;
    }

    fn set_flex_weights(self: Handle<Self>, app: &mut App, value: Option<Vec<i32>>) {
        if app.get(self).flex_weights == value {
            return;
        }
        let old_weights = app.get(self).flex_weights.clone();
        if self.has_pixels(app) && old_weights.is_some() {
            let consume_max_weight = app.get(self).consume_max_weight;
            let leading_item = self.update_leading_item(app, value.as_deref(), consume_max_weight);
            let item_extent = app.get(self).item_extent;
            let new_pixel =
                self.get_pixels_from_item(app, leading_item, value.as_deref(), item_extent);
            self.force_pixels(app, new_pixel);
        }
        app.get_mut(self).flex_weights = value;
    }

    // The index of the leading item in the carousel.
    // `getItemFromPixels` may return a fractional value (e.g., 0.6 when mid-scroll).
    // Use `toInt()` to truncate the fractional part, ensuring the leading item
    // only advances after fully crossing the next item's boundary.
    fn leading_item(self: Handle<Self>, app: &App) -> i32 {
        let mut leading_item =
            self.get_item_from_pixels(app, self.pixels(app), self.viewport_dimension(app)) as i32;
        let this = app.get(self);
        // When `consumeMaxWeight` is true, there is some reserved space before
        // item 0 so that item 0 can be expanded to occupy the maximum
        // weight while scrolling. The way how consumeMaxWeight works is that we assume
        // there are some "invisible" items before the first visible item. Therefore,
        // to calculate the correct visible leading item, we need to offset the leading
        // item by the index of the maximum weight.
        //
        // The subtraction may cause negative number for leading item. In this case,
        // constrain the leading item to 0.
        if this.consume_max_weight
            && let Some(flex_weights) = &this.flex_weights
        {
            leading_item = (leading_item - index_of_max(flex_weights)).max(0);
        }
        // For infinite scrolling, wrap the index to the range [0, itemCount - 1].
        if this.infinite
            && let Some(item_count) = this.item_count
            && item_count > 0
        {
            leading_item %= item_count;
        }
        leading_item
    }

    fn update_leading_item(
        self: Handle<Self>,
        app: &App,
        new_flex_weights: Option<&[i32]>,
        new_consume_max_weight: bool,
    ) -> f64 {
        let this = app.get(self);
        let max_item;
        if self.has_pixels(app)
            && let Some(flex_weights) = &this.flex_weights
        {
            let leading_item =
                self.get_item_from_pixels(app, self.pixels(app), self.viewport_dimension(app));
            max_item = if this.consume_max_weight {
                leading_item
            } else {
                leading_item + f64::from(index_of_max(flex_weights))
            };
        } else {
            if !new_consume_max_weight {
                return this.item_to_show_on_startup;
            }
            max_item = this.item_to_show_on_startup;
        }
        if let Some(new_flex_weights) = new_flex_weights
            && !new_consume_max_weight
        {
            let max_weight = new_flex_weights.iter().max().expect("not empty");
            let mut smaller_weights = 0;
            for weight in new_flex_weights {
                if weight == max_weight {
                    break;
                }
                smaller_weights += 1;
            }
            return max_item - f64::from(smaller_weights);
        }
        max_item
    }

    fn get_item_from_pixels(
        self: Handle<Self>,
        app: &App,
        pixels: f64,
        viewport_dimension: f64,
    ) -> f64 {
        debug_assert!(viewport_dimension > 0.0);
        let this = app.get(self);
        let fraction = match this.item_extent {
            Some(item_extent) => item_extent / viewport_dimension,
            None => {
                // If itemExtent is null, flexWeights cannot be null.
                let flex_weights = this
                    .flex_weights
                    .as_ref()
                    .expect("a carousel position has an item extent or flex weights");
                f64::from(flex_weights[0]) / f64::from(flex_weights.iter().sum::<i32>())
            }
        };

        let actual = pixels.max(0.0) / (viewport_dimension * fraction);
        let round = actual.round();
        if (actual - round).abs() < PRECISION_ERROR_TOLERANCE {
            return round;
        }
        actual
    }

    fn get_pixels_from_item(
        self: Handle<Self>,
        app: &App,
        item: f64,
        flex_weights: Option<&[i32]>,
        item_extent: Option<f64>,
    ) -> f64 {
        let viewport_dimension = self.viewport_dimension(app);
        if viewport_dimension == 0.0 {
            return 0.0;
        }
        let fraction = match item_extent {
            Some(item_extent) => item_extent / viewport_dimension,
            None => {
                // If itemExtent is null, flexWeights cannot be null.
                let flex_weights =
                    flex_weights.expect("a carousel position has an item extent or flex weights");
                f64::from(flex_weights[0]) / f64::from(flex_weights.iter().sum::<i32>())
            }
        };

        item * viewport_dimension * fraction
    }

    fn get_cycle_length_in_pixels(self: Handle<Self>, app: &App) -> f64 {
        let this = app.get(self);
        let Some(item_count) = this.item_count else {
            return 0.0;
        };
        if item_count <= 0
            || !self.has_viewport_dimension(app)
            || self.viewport_dimension(app) == 0.0
        {
            return 0.0;
        }
        let viewport_dimension = self.viewport_dimension(app);
        let fraction = if let Some(item_extent) = this.item_extent {
            item_extent / viewport_dimension
        } else if let Some(flex_weights) = &this.flex_weights {
            f64::from(flex_weights[0]) / f64::from(flex_weights.iter().sum::<i32>())
        } else {
            return 0.0;
        };
        f64::from(item_count) * viewport_dimension * fraction
    }
}

impl ChangeNotifier for CarouselPosition {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl ViewportOffset for CarouselPosition {
    fn pixels(self: Handle<Self>, app: &App) -> f64 {
        ScrollPositionBase::pixels(self, app)
    }

    fn has_pixels(self: Handle<Self>, app: &App) -> bool {
        ScrollPositionBase::has_pixels(self, app)
    }

    fn apply_viewport_dimension(
        self: Handle<Self>,
        app: &mut App,
        viewport_dimension: f64,
    ) -> bool {
        let old_viewport_dimensions = self
            .has_viewport_dimension(app)
            .then(|| self.viewport_dimension(app));
        if Some(viewport_dimension) == old_viewport_dimensions {
            return true;
        }
        let result = ScrollPositionBase::apply_viewport_dimension(self, app, viewport_dimension);
        let old_pixels = self.has_pixels(app).then(|| self.pixels(app));
        let flex_weights = app.get(self).flex_weights.clone();
        let item = match old_pixels {
            None => {
                let consume_max_weight = app.get(self).consume_max_weight;
                self.update_leading_item(app, flex_weights.as_deref(), consume_max_weight)
            }
            // If resize from zero, we should use the _cachedItem to recover the state.
            Some(_) if old_viewport_dimensions == Some(0.0) => app
                .get(self)
                .cached_item
                .expect("a position resized from zero cached its item"),
            Some(old_pixels) => self.get_item_from_pixels(
                app,
                old_pixels,
                old_viewport_dimensions.unwrap_or(viewport_dimension),
            ),
        };
        let item_extent = app.get(self).item_extent;
        let new_pixels = self.get_pixels_from_item(app, item, flex_weights.as_deref(), item_extent);
        // If the viewportDimension is zero, cache the item
        // in case the viewport is resized to be non-zero.
        app.get_mut(self).cached_item = (viewport_dimension == 0.0).then_some(item);

        if Some(new_pixels) != old_pixels {
            self.correct_pixels(app, new_pixels);
            return false;
        }
        result
    }

    fn apply_content_dimensions(
        self: Handle<Self>,
        app: &mut App,
        min_scroll_extent: f64,
        max_scroll_extent: f64,
    ) -> bool {
        // For infinite scrolling, dynamically add cycles when approaching the boundary.
        // This eliminates the need for a large hardcoded starting offset.
        let infinite = app.get(self).infinite;
        if infinite && self.has_pixels(app) {
            let cycle_length = self.get_cycle_length_in_pixels(app);
            let pixels = self.pixels(app);
            if cycle_length > 0.0 && pixels < cycle_length {
                // When scroll position drops below one cycle, add cycles to maintain buffer.
                // This allows seamless backward scrolling without hitting the boundary.
                let cycles_to_add = ((cycle_length - pixels) / cycle_length).ceil();
                self.correct_pixels(app, pixels + cycles_to_add * cycle_length);
                // Indicate position was corrected and layout should rerun.
                return false;
            }
        }
        ScrollPositionBase::apply_content_dimensions(
            self,
            app,
            if infinite { 0.0 } else { min_scroll_extent },
            max_scroll_extent,
        )
    }

    fn correct_by(self: Handle<Self>, app: &mut App, correction: f64) {
        ScrollPositionBase::correct_by(self, app, correction);
    }

    fn jump_to(self: Handle<Self>, app: &mut App, value: f64) {
        ScrollPositionWithSingleContextLeaf::jump_to(self, app, value);
    }

    fn animate_to(
        self: Handle<Self>,
        app: &mut App,
        to: f64,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) -> CompleterFuture<()> {
        ScrollPositionWithSingleContextLeaf::animate_to(self, app, to, duration, curve)
    }

    fn move_to(
        self: Handle<Self>,
        app: &mut App,
        to: f64,
        duration: Option<Duration>,
        curve: Option<Rc<dyn Curve>>,
        clamp: Option<bool>,
    ) -> CompleterFuture<()> {
        ScrollPositionBase::move_to(self, app, to, duration, curve, clamp)
    }

    fn user_scroll_direction(self: Handle<Self>, app: &App) -> waft_rendering::ScrollDirection {
        ScrollPositionWithSingleContextLeaf::user_scroll_direction(self, app)
    }

    fn allow_implicit_scrolling(self: Handle<Self>, app: &App) -> bool {
        ScrollPositionBase::allow_implicit_scrolling(self, app)
    }
}

impl ScrollPosition for CarouselPosition {
    waft_widgets::scroll_position_with_single_context_overrides!();

    fn copy_with(self: Handle<Self>, app: &App) -> Rc<dyn ScrollMetrics> {
        let has_content_dimensions = self.has_content_dimensions(app);
        let this = app.get(self);
        Rc::new(CarouselMetrics {
            fixed: FixedScrollMetrics::new(
                has_content_dimensions.then(|| self.min_scroll_extent(app)),
                has_content_dimensions.then(|| self.max_scroll_extent(app)),
                self.has_pixels(app).then(|| self.pixels(app)),
                self.has_viewport_dimension(app)
                    .then(|| self.viewport_dimension(app)),
                ScrollPosition::axis_direction(self, app),
                self.device_pixel_ratio(app),
            ),
            item_extent: this.item_extent,
            flex_weights: this.flex_weights.clone(),
            consume_max_weight: Some(this.consume_max_weight),
        })
    }
}

impl ScrollActivityDelegate for CarouselPosition {
    waft_widgets::scroll_position_with_single_context_delegate_overrides!();
}

impl ScrollPositionWithSingleContextLeaf for CarouselPosition {
    waft_widgets::scroll_position_with_single_context_accessors!();

    fn absorb(self: Handle<Self>, app: &mut App, other: AnyScrollPosition) {
        ScrollPositionWithSingleContext::absorb(self, app, other);

        let Some(other) = other.downcast::<CarouselPosition>(app) else {
            return;
        };

        let (cached_item, item_extent) = {
            let other = app.get(other);
            (other.cached_item, other.item_extent)
        };
        let this = app.get_mut(self);
        this.cached_item = cached_item;
        this.item_extent = item_extent;
    }
}

// ---------------------------------------------------------------------------------------------
// CarouselController

/// A controller for [`CarouselView`].
///
/// Using a carousel controller helps to show the first visible item on the carousel list.
pub struct CarouselController {
    change_notifier: ChangeNotifierData,
    scroll_controller: ScrollControllerData,
    /// The item that will be the leading item when the carousel is first built.
    ///
    /// For a weighted carousel, it is the item that occupies the first maximum weight.
    initial_item: i32,
    carousel_state: Option<Handle<CarouselViewState>>,
}

impl CarouselController {
    /// Creates a carousel controller.
    pub fn new(app: &mut App, initial_item: i32) -> Ref<CarouselController> {
        app.create(CarouselController {
            change_notifier: ChangeNotifierData::new(),
            scroll_controller: ScrollControllerData::new(0.0, true, None, None, None),
            initial_item,
            carousel_state: None,
        })
    }

    /// The item that will be the leading item when the carousel is first built.
    pub fn initial_item(self: Handle<Self>, app: &App) -> i32 {
        app.get(self).initial_item
    }

    /// The index of the current leading item of the controlled [`CarouselView`].
    ///
    /// Reading it panics before a [`CarouselView`] is built with this controller, and while
    /// several carousels are attached to it.
    pub fn leading_item(self: Handle<Self>, app: &App) -> i32 {
        let positions = self.positions(app);
        debug_assert!(
            !positions.is_empty(),
            "CarouselController.leadingItem cannot be accessed before a CarouselView is built \
             with it."
        );
        debug_assert!(
            positions.len() == 1,
            "CarouselController.leadingItem cannot be read when multiple CarouselViews are \
             attached to the same controller."
        );
        self.carousel_position(app).leading_item(app)
    }

    /// Dart's `this.position as _CarouselPosition`.
    fn carousel_position(self: Handle<Self>, app: &App) -> Handle<CarouselPosition> {
        self.position(app)
            .downcast::<CarouselPosition>(app)
            .expect("a CarouselController creates the positions of the CarouselViews it controls")
    }

    fn attach_state(self: Handle<Self>, app: &mut App, anchor: Handle<CarouselViewState>) {
        app.get_mut(self).carousel_state = Some(anchor);
    }

    fn detach_state(self: Handle<Self>, app: &mut App, anchor: Handle<CarouselViewState>) {
        if app.get(self).carousel_state == Some(anchor) {
            app.get_mut(self).carousel_state = None;
        }
    }

    /// Animates the controlled carousel so that the item at `index` is the leading item (or, for
    /// a weighted carousel with `consume_max_weight` unset, the one with the largest weight).
    ///
    /// The returned future resolves when the animation completes; it resolves at once without a
    /// controlled carousel. Dart's defaults are a duration of 300 milliseconds and `Curves.ease`.
    pub fn animate_to_item(
        self: Handle<Self>,
        app: &mut App,
        index: i32,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) -> CompleterFuture<()> {
        let Some(carousel_state) = app.get(self).carousel_state else {
            return CompleterFuture::ready(());
        };
        if !self.has_clients(app) {
            return CompleterFuture::ready(());
        }

        let has_flex_weights = carousel_state
            .flex_weights(app)
            .is_some_and(|weights| !weights.is_empty());
        let widget = carousel_state.widget(app);
        let index = if widget.item_builder.is_some() {
            match widget.item_count {
                Some(item_count) => index.clamp(0, item_count - 1),
                None => 0,
            }
        } else {
            index.clamp(0, widget.children.len() as i32 - 1)
        };

        let futures: Vec<CompleterFuture<()>> = self
            .positions(app)
            .into_iter()
            .map(|position| {
                let position = position
                    .downcast::<CarouselPosition>(app)
                    .expect("a CarouselController's positions are carousel positions");
                let target = self.get_target_offset(app, position, index, has_flex_weights);
                ViewportOffset::animate_to(position, app, target, duration, Rc::clone(&curve))
            })
            .collect();
        CompleterFuture::spawn(app, async move {
            wait_all(futures).await;
        })
    }

    fn get_target_offset(
        self: Handle<Self>,
        app: &App,
        position: Handle<CarouselPosition>,
        index: i32,
        has_flex_weights: bool,
    ) -> f64 {
        let carousel_state = app.get(self).carousel_state.expect("checked by the caller");
        if !has_flex_weights {
            let target_in_first_cycle = f64::from(index)
                * app
                    .get(carousel_state)
                    .item_extent
                    .expect("a carousel without weights has an item extent");
            if !carousel_state.widget(app).infinite {
                return target_in_first_cycle;
            }
            return self.adjust_for_infinite_cycle(app, position, target_in_first_cycle);
        }

        let weights = carousel_state
            .flex_weights(app)
            .expect("has_flex_weights was checked");
        let total_weight: i32 = weights.iter().sum();
        let dimension = position.viewport_dimension(app);

        let max_weight_index = index_of_max(&weights);
        let mut leading_index = if carousel_state.consume_max_weight(app) {
            index
        } else {
            index - max_weight_index
        };
        let widget = carousel_state.widget(app);
        if widget.item_builder.is_some() {
            leading_index = match widget.item_count {
                Some(item_count) => leading_index.clamp(0, item_count - 1),
                None => 0,
            };
        } else {
            let item_count = widget.children.len() as i32;
            leading_index = leading_index.clamp(0, item_count - 1);
        }

        let target_in_first_cycle = dimension
            * (f64::from(weights[0]) / f64::from(total_weight))
            * f64::from(leading_index);
        if !widget.infinite {
            return target_in_first_cycle;
        }
        self.adjust_for_infinite_cycle(app, position, target_in_first_cycle)
    }

    fn adjust_for_infinite_cycle(
        self: Handle<Self>,
        app: &App,
        position: Handle<CarouselPosition>,
        target_in_first_cycle: f64,
    ) -> f64 {
        let cycle_length = position.get_cycle_length_in_pixels(app);
        if cycle_length <= 0.0 {
            return target_in_first_cycle;
        }
        let current_pixels = position.pixels(app);
        // Determine which cycle the current position is in.
        let current_cycle_start = (current_pixels / cycle_length).floor() * cycle_length;
        // Candidate target in the same cycle as the current position.
        let same_cycle_target = current_cycle_start + target_in_first_cycle;

        // Always scroll forward: pick the first target at or ahead of current position.
        if same_cycle_target >= current_pixels {
            return same_cycle_target;
        }
        same_cycle_target + cycle_length
    }

    fn get_item_count(self: Handle<Self>, app: &App) -> Option<i32> {
        let carousel_state = app.get(self).carousel_state?;
        let widget = carousel_state.widget(app);
        if widget.item_builder.is_some() {
            return widget.item_count;
        }
        Some(widget.children.len() as i32)
    }
}

impl ChangeNotifier for CarouselController {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl ScrollControllerLeaf for CarouselController {
    waft_widgets::scroll_controller_accessors!();

    fn create_scroll_position(
        self: Handle<Self>,
        app: &mut App,
        physics: ScrollPhysicsRef,
        context: Rc<dyn ScrollContext>,
        old_position: Option<AnyScrollPosition>,
    ) -> ScrollPositionRef {
        let carousel_state = app
            .get(self)
            .carousel_state
            .expect("a CarouselController creates positions for an attached CarouselView");
        let initial_item = app.get(self).initial_item;
        let item_extent = app.get(carousel_state).item_extent;
        let consume_max_weight = carousel_state.consume_max_weight(app);
        let flex_weights = carousel_state.flex_weights(app);
        let infinite = carousel_state.widget(app).infinite;
        let item_count = self.get_item_count(app);
        CarouselPosition::new(
            app,
            physics,
            context,
            initial_item,
            item_extent,
            flex_weights,
            consume_max_weight,
            infinite,
            item_count,
            old_position,
        )
        .into_erased()
    }

    fn attach(self: Handle<Self>, app: &mut App, position: ScrollPositionRef) {
        let attached = position.handle();
        waft_widgets::ScrollController::attach(self.as_controller(), app, position);
        let carousel_position = attached
            .downcast::<CarouselPosition>(app)
            .expect("a CarouselController attaches the positions it creates");
        let carousel_state = app
            .get(self)
            .carousel_state
            .expect("a CarouselController attaches positions of an attached CarouselView");
        let flex_weights = carousel_state.flex_weights(app);
        let item_extent = app.get(carousel_state).item_extent;
        let consume_max_weight = carousel_state.consume_max_weight(app);
        let infinite = carousel_state.widget(app).infinite;
        let item_count = self.get_item_count(app);
        carousel_position.set_flex_weights(app, flex_weights);
        carousel_position.set_item_extent(app, item_extent);
        carousel_position.set_consume_max_weight(app, consume_max_weight);
        carousel_position.set_infinite(app, infinite);
        carousel_position.set_item_count(app, item_count);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use waft_animation::Curves;
    use waft_embedder::Size;
    use waft_embedder::{Offset, Rect};
    use waft_painting::{BorderRadiusGeometry, BorderSide, StadiumBorder};
    use waft_rendering::RenderBox;
    use waft_rendering::testing::{PaintCall, paints};
    use waft_widgets::testing::{
        Finder, TestPointer, WidgetTester, expect, find, finds_nothing, finds_one_widget,
        test_widgets,
    };
    use waft_widgets::{
        Center, Padding, ScrollBehaviorRef, ScrollConfiguration, SizedBox, Text, WidgetStates,
    };

    use super::*;
    use crate::scaffold::Scaffold;
    use crate::test_support::{PaintInspection, material_app};
    use crate::theme_data::ThemeData;

    fn text_items(count: i32) -> Vec<WidgetRef> {
        (0..count)
            .map(|index| {
                Center::new()
                    .child(Text::new(format!("Item {index}")))
                    .into_widget()
            })
            .collect()
    }

    fn pump_carousel(tester: &mut WidgetTester, carousel: CarouselView) {
        let theme = ThemeData::builder().build(&tester.cell.borrow());
        tester.pump_widget(material_app(
            theme,
            Scaffold::new().body(carousel).into_widget(),
        ));
    }

    /// What the first `_RenderInkFeatures` paints with its subtree: Dart's `paints` on it.
    fn ink_painted_ops(tester: &WidgetTester) -> Vec<waft_embedder::valo::Op> {
        tester.painted_ops_deep(
            tester
                .first::<crate::material::RenderInkFeatures>()
                .as_object(),
        )
    }

    /// Dart's `getItem`.
    fn get_item(index: i32) -> Finder {
        find::descendant(
            find::by_type::<CarouselView>(),
            find::ancestor(
                find::text(&format!("Item {index}")),
                find::by_type::<Padding>(),
            ),
        )
    }

    fn assert_item_rect(tester: &WidgetTester, index: i32, expected: Rect) {
        waft_widgets::testing::assert_more_or_less_equals_within(
            tester.get_rect(get_item(index)),
            expected,
            1e-6,
        );
    }

    fn expect_items(tester: &WidgetTester, present: &[i32], absent: &[i32]) {
        for index in present {
            expect(tester, &get_item(*index), finds_one_widget());
        }
        for index in absent {
            expect(tester, &get_item(*index), finds_nothing());
        }
    }

    #[test]
    fn carousel_view_defaults() {
        test_widgets(|tester| {
            pump_carousel(tester, CarouselView::new(200.0, text_items(10)));
            let theme = ThemeData::builder().build(&tester.cell.borrow());

            let material = find::descendant(
                find::by_type::<CarouselView>(),
                find::by_type::<crate::material::Material>(),
            )
            .first();
            tester.widget(material, |material: &crate::material::Material| {
                assert_eq!(material.clip_behavior, Clip::AntiAlias);
                assert_eq!(material.color, Some(theme.color_scheme.surface));
                assert_eq!(material.elevation, 0.0);
                assert_eq!(
                    material.shape,
                    Some(ShapeBorderRef::new(RoundedRectangleBorder::new(
                        BorderSide::NONE,
                        BorderRadiusGeometry::circular(28.0)
                    )))
                );
            });
        });
    }

    #[test]
    fn carousel_view_items_customization() {
        test_widgets(|tester| {
            let key = waft_widgets::GlobalKey::new();
            let mut children = text_items(10);
            children[0] = Center::new()
                .key(key.clone().into())
                .child(Center::new().child(Text::new("Item 0")))
                .into_widget();
            let overlay_color = <dyn WidgetStateProperty<Option<Color>>>::resolve_with(|states| {
                if states.contains(&WidgetState::Pressed) {
                    return Some(Colors::YELLOW.primary());
                }
                if states.contains(&WidgetState::Hovered) {
                    return Some(Colors::RED.primary());
                }
                if states.contains(&WidgetState::Focused) {
                    return Some(Colors::PURPLE.primary());
                }
                None
            });
            pump_carousel(
                tester,
                CarouselView::new(200.0, children)
                    .padding(EdgeInsets::all(20.0))
                    .background_color(Colors::AMBER.primary())
                    .elevation(10.0)
                    .shape(StadiumBorder::new(BorderSide::NONE))
                    .overlay_color(overlay_color),
            );

            let carousel_view_material = find::descendant(
                find::by_type::<CarouselView>(),
                find::by_type::<crate::material::Material>(),
            )
            .first();
            // Padding is 20 on both side.
            assert_eq!(
                tester.get_size(carousel_view_material.clone()).width(),
                200.0 - 20.0 - 20.0
            );
            tester.widget(
                carousel_view_material.clone(),
                |material: &crate::material::Material| {
                    assert_eq!(material.color, Some(Colors::AMBER.primary()));
                    assert_eq!(material.elevation, 10.0);
                    assert_eq!(
                        material.shape,
                        Some(ShapeBorderRef::new(StadiumBorder::new(BorderSide::NONE)))
                    );
                },
            );

            // On hovered.
            let mut mouse = TestPointer::mouse(1, waft_gestures::K_PRIMARY_MOUSE_BUTTON);
            let center = tester.get_center(&key);
            tester.add_pointer(&mut mouse, Offset::ZERO);
            tester.move_to(&mut mouse, center, false);
            tester.pump_and_settle();
            assert!(
                paints(
                    &ink_painted_ops(tester),
                    &[PaintCall::rect(Some(Colors::RED.primary()))]
                )
                .is_ok()
            );

            // On pressed.
            tester.pump_and_settle();
            tester.down(&mut mouse, center);
            tester.pump_and_settle();
            for op in ink_painted_ops(tester) {
                eprintln!("DBG {op:?}");
            }
            assert!(
                paints(
                    &ink_painted_ops(tester),
                    &[
                        PaintCall::rect(None),
                        PaintCall::rect(Some(Colors::YELLOW.primary()))
                    ]
                )
                .is_ok()
            );

            tester.pump_and_settle();
            tester.up(&mouse);
            tester.remove_pointer(&mouse);

            // On focused.
            let overlay_color = tester.widget(
                find::descendant(carousel_view_material, find::by_type::<InkWell>()),
                |ink_well: &InkWell| ink_well.ink_response.overlay_color.clone(),
            );
            assert_eq!(
                overlay_color
                    .expect("an overlay color")
                    .resolve(&WidgetStates::from([WidgetState::Focused])),
                Some(Colors::PURPLE.primary())
            );
        });
    }

    #[test]
    fn carousel_view_respects_on_tap() {
        test_widgets(|tester| {
            let tap_index = Rc::new(Cell::new(0));
            pump_carousel(
                tester,
                CarouselView::new(50.0, text_items(10)).on_tap({
                    let tap_index = Rc::clone(&tap_index);
                    Rc::new(move |_app, index| tap_index.set(index))
                }),
            );

            tester.tap(find::ancestor(find::text("Item 1"), find::by_type::<Stack>()).first());
            tester.pump();
            assert_eq!(tap_index.get(), 1);

            tester.tap(find::ancestor(find::text("Item 2"), find::by_type::<Stack>()).first());
            tester.pump();
            assert_eq!(tap_index.get(), 2);
        });
    }

    #[test]
    fn carousel_view_layout_uncontained_layout() {
        test_widgets(|tester| {
            pump_carousel(tester, CarouselView::new(250.0, text_items(10)));

            expect(tester, &find::text("Item 0"), finds_one_widget());
            assert_item_rect(tester, 0, Rect::from_ltrb(0.0, 0.0, 250.0, 600.0));
            expect(tester, &find::text("Item 1"), finds_one_widget());
            assert_item_rect(tester, 1, Rect::from_ltrb(250.0, 0.0, 500.0, 600.0));
            expect(tester, &find::text("Item 2"), finds_one_widget());
            assert_item_rect(tester, 2, Rect::from_ltrb(500.0, 0.0, 750.0, 600.0));
            expect(tester, &find::text("Item 3"), finds_one_widget());
            assert_item_rect(tester, 3, Rect::from_ltrb(750.0, 0.0, 800.0, 600.0));
            expect(tester, &find::text("Item 4"), finds_nothing());
        });
    }

    #[test]
    fn carousel_view_weighted_layout() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::weighted(vec![4, 3, 2, 1], text_items(10)),
            );

            // Item widths are 4/10, 3/10, 2/10 and 1/10 of the viewport.
            assert_item_rect(tester, 0, Rect::from_ltrb(0.0, 0.0, 320.0, 600.0));
            assert_item_rect(tester, 1, Rect::from_ltrb(320.0, 0.0, 560.0, 600.0));
            assert_item_rect(tester, 2, Rect::from_ltrb(560.0, 0.0, 720.0, 600.0));
            assert_item_rect(tester, 3, Rect::from_ltrb(720.0, 0.0, 800.0, 600.0));
            expect(tester, &find::text("Item 4"), finds_nothing());

            // Test shorter weight list.
            pump_carousel(tester, CarouselView::weighted(vec![7, 1], text_items(10)));
            tester.pump_and_settle();

            // Item widths are 7/8 and 1/8 of the viewport.
            assert_item_rect(tester, 0, Rect::from_ltrb(0.0, 0.0, 700.0, 600.0));
            assert_item_rect(tester, 1, Rect::from_ltrb(700.0, 0.0, 800.0, 600.0));
            expect(tester, &find::text("Item 2"), finds_nothing());
        });
    }

    #[test]
    fn carousel_controller_initial_item() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 5);
            pump_carousel(
                tester,
                CarouselView::new(400.0, text_items(10)).controller(controller),
            );

            assert_item_rect(tester, 5, Rect::from_ltrb(0.0, 0.0, 400.0, 600.0));
            assert_item_rect(tester, 6, Rect::from_ltrb(400.0, 0.0, 800.0, 600.0));
            expect_items(tester, &[5, 6], &[4, 7]);
        });
    }

    #[test]
    fn carousel_view_weighted_respects_carousel_controller_initial_item() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 5);
            pump_carousel(
                tester,
                CarouselView::weighted(vec![7, 1], text_items(10)).controller(controller),
            );

            assert_item_rect(tester, 5, Rect::from_ltrb(0.0, 0.0, 700.0, 600.0));
            assert_item_rect(tester, 6, Rect::from_ltrb(700.0, 0.0, 800.0, 600.0));
            expect_items(tester, &[5, 6], &[4, 7]);
        });
    }

    #[test]
    fn the_initial_item_should_be_the_first_item_with_expanded_size() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 5);
            pump_carousel(
                tester,
                CarouselView::weighted(vec![1, 8, 1], text_items(10)).controller(controller),
            );

            // Item 5 should have be the expanded item.
            assert_item_rect(tester, 5, Rect::from_ltrb(80.0, 0.0, 720.0, 600.0));
            assert_item_rect(tester, 6, Rect::from_ltrb(720.0, 0.0, 800.0, 600.0));
            assert_item_rect(tester, 4, Rect::from_ltrb(0.0, 0.0, 80.0, 600.0));
            expect_items(tester, &[4, 5, 6], &[7]);
        });
    }

    #[test]
    fn carousel_view_respects_item_snapping() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::new(300.0, text_items(10)).item_snapping(true),
            );
            let check_original_expectations = |tester: &WidgetTester| {
                expect_items(tester, &[0, 1, 2], &[3]);
            };

            check_original_expectations(tester);

            // Snap back to the original item.
            tester.drag(get_item(0), Offset::new(-150.0, 0.0));
            tester.pump_and_settle();
            check_original_expectations(tester);

            // Snap back to the original item.
            tester.drag(get_item(0), Offset::new(100.0, 0.0));
            tester.pump_and_settle();
            check_original_expectations(tester);

            // Snap to the next item.
            tester.drag(get_item(0), Offset::new(-200.0, 0.0));
            tester.pump_and_settle();
            expect_items(tester, &[1, 2, 3], &[0]);
        });
    }

    #[test]
    fn carousel_view_weighted_respects_item_snapping() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::weighted(vec![1, 7], text_items(10))
                    .item_snapping(true)
                    .consume_max_weight(false),
            );
            let check_original_expectations = |tester: &WidgetTester| {
                expect_items(tester, &[0, 1], &[2]);
            };

            check_original_expectations(tester);

            // Snap back to the original item.
            tester.drag(get_item(0), Offset::new(-20.0, 0.0));
            tester.pump_and_settle();
            check_original_expectations(tester);

            // Snap back to the original item.
            tester.drag(get_item(0), Offset::new(50.0, 0.0));
            tester.pump_and_settle();
            check_original_expectations(tester);

            // Snap to the next item.
            tester.drag(get_item(0), Offset::new(-70.0, 0.0));
            tester.pump_and_settle();
            expect_items(tester, &[1, 2], &[0, 3]);
        });
    }

    #[test]
    fn carousel_view_respects_item_snapping_when_fling() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::new(300.0, text_items(10)).item_snapping(true),
            );

            // Show item 0, 1, and 2.
            expect_items(tester, &[0, 1, 2], &[3]);

            // Snap to the next item. Show item 1, 2 and 3.
            tester.fling(get_item(0), Offset::new(-100.0, 0.0), 800.0);
            tester.pump_and_settle();
            expect_items(tester, &[1, 2, 3], &[0, 4]);

            // Snap to the next item. Show item 2, 3 and 4.
            tester.fling(get_item(1), Offset::new(-100.0, 0.0), 800.0);
            tester.pump_and_settle();
            expect_items(tester, &[2, 3, 4], &[0, 1, 5]);

            // Fling back to the previous item. Show item 1, 2 and 3.
            tester.fling(get_item(2), Offset::new(100.0, 0.0), 800.0);
            tester.pump_and_settle();
            expect_items(tester, &[1, 2, 3], &[4]);
        });
    }

    #[test]
    fn carousel_view_respects_reverse() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::new(200.0, text_items(10)).reverse(true),
            );
            tester.pump_and_settle();

            // Item 0 should be placed on the end of the screen.
            assert_item_rect(tester, 0, Rect::from_ltrb(600.0, 0.0, 800.0, 600.0));
            // Item 1 should be placed before item 0.
            assert_item_rect(tester, 1, Rect::from_ltrb(400.0, 0.0, 600.0, 600.0));
            assert_item_rect(tester, 2, Rect::from_ltrb(200.0, 0.0, 400.0, 600.0));
            assert_item_rect(tester, 3, Rect::from_ltrb(0.0, 0.0, 200.0, 600.0));
        });
    }

    #[test]
    fn carousel_view_weighted_respects_reverse() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::weighted(vec![4, 3, 2, 1], text_items(10)).reverse(true),
            );
            tester.pump_and_settle();

            let item0_width = 80.0 * 4.0;
            let item1_width = 80.0 * 3.0;
            let item2_width = 80.0 * 2.0;
            assert_item_rect(
                tester,
                0,
                Rect::from_ltrb(800.0 - item0_width, 0.0, 800.0, 600.0),
            );
            assert_item_rect(
                tester,
                1,
                Rect::from_ltrb(
                    800.0 - item0_width - item1_width,
                    0.0,
                    800.0 - item0_width,
                    600.0,
                ),
            );
            assert_item_rect(
                tester,
                2,
                Rect::from_ltrb(
                    800.0 - item0_width - item1_width - item2_width,
                    0.0,
                    800.0 - item0_width - item1_width,
                    600.0,
                ),
            );
            assert_item_rect(
                tester,
                3,
                Rect::from_ltrb(
                    0.0,
                    0.0,
                    800.0 - item0_width - item1_width - item2_width,
                    600.0,
                ),
            );
        });
    }

    #[test]
    fn carousel_view_weighted_respects_consume_max_weight() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::weighted(vec![1, 2, 4, 2, 1], text_items(10)).item_snapping(true),
            );

            assert_item_rect(tester, 0, Rect::from_ltrb(240.0, 0.0, 560.0, 600.0));

            for _ in 0..7 {
                tester.drag(find::by_type::<CarouselView>(), Offset::new(-80.0, 0.0));
                tester.pump_and_settle();
            }
            expect(tester, &get_item(9), finds_one_widget());
            assert_item_rect(tester, 9, Rect::from_ltrb(720.0, 0.0, 800.0, 600.0));

            for _ in 0..2 {
                tester.drag(find::by_type::<CarouselView>(), Offset::new(-80.0, 0.0));
                tester.pump_and_settle();
            }
            expect(tester, &get_item(9), finds_one_widget());
            assert_item_rect(tester, 9, Rect::from_ltrb(240.0, 0.0, 560.0, 600.0));
        });
    }

    #[test]
    fn the_initial_item_stays_when_the_flex_weights_is_updated() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 3);
            let build_carousel = |tester: &mut WidgetTester, flex_weights: Vec<i32>| {
                pump_carousel(
                    tester,
                    CarouselView::weighted(flex_weights, text_items(20))
                        .controller(controller.clone())
                        .item_snapping(true),
                );
            };

            build_carousel(tester, vec![1, 1, 6, 1, 1]);
            tester.pump_and_settle();
            expect(tester, &find::text("Item 0"), finds_nothing());
            for index in 1..=5 {
                expect(
                    tester,
                    &find::text(&format!("Item {index}")),
                    finds_one_widget(),
                );
            }
            let rect3 = tester.get_rect(get_item(3));
            assert_eq!(rect3.center().dx(), 400.0);
            assert_eq!(rect3.center().dy(), 300.0);
            expect(tester, &find::text("Item 6"), finds_nothing());

            build_carousel(tester, vec![7, 1]);
            tester.pump_and_settle();
            expect(tester, &find::text("Item 2"), finds_nothing());
            expect(tester, &find::text("Item 3"), finds_one_widget());
            expect(tester, &find::text("Item 4"), finds_one_widget());
            expect(tester, &find::text("Item 5"), finds_nothing());
            assert_item_rect(tester, 3, Rect::from_ltrb(0.0, 0.0, 700.0, 600.0));
            assert_item_rect(tester, 4, Rect::from_ltrb(700.0, 0.0, 800.0, 600.0));
        });
    }

    #[test]
    fn the_initial_item_stays_when_the_item_extent_is_updated() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 3);
            let build_carousel = |tester: &mut WidgetTester, item_extent: f64| {
                pump_carousel(
                    tester,
                    CarouselView::new(item_extent, text_items(20))
                        .controller(controller.clone())
                        .item_snapping(true),
                );
            };

            for item_extent in [234.0, 400.0, 100.0] {
                build_carousel(tester, item_extent);
                tester.pump_and_settle();
                let bottom_right = tester.get_rect(get_item(3)).bottom_right();
                assert_eq!(bottom_right.dx(), item_extent);
                assert_eq!(bottom_right.dy(), 600.0);
            }
        });
    }

    #[test]
    fn carousel_view_on_tap_is_clickable() {
        test_widgets(|tester| {
            let tapped_index = Rc::new(Cell::new(-1));
            pump_carousel(
                tester,
                CarouselView::new(350.0, text_items(3)).on_tap({
                    let tapped_index = Rc::clone(&tapped_index);
                    Rc::new(move |_app, index| tapped_index.set(index))
                }),
            );
            tester.pump_and_settle();

            tester.tap(find::text("Item 1"));
            tester.pump_and_settle();
            assert_eq!(tapped_index.get(), 1);

            tester.tap(find::text("Item 2"));
            tester.pump_and_settle();
            assert_eq!(tapped_index.get(), 2);
        });
    }

    #[test]
    fn carousel_view_with_enable_splash_false_children_are_directly_interactive() {
        test_widgets(|tester| {
            let button_pressed = Rc::new(Cell::new(false));
            let children = (0..3)
                .map(|index| {
                    let button_pressed = Rc::clone(&button_pressed);
                    Center::new()
                        .child(crate::elevated_button::ElevatedButton::new(
                            Some(Listener::new(move |_app| button_pressed.set(true))),
                            Some(Text::new(format!("Button {index}"))),
                        ))
                        .into_widget()
                })
                .collect();
            pump_carousel(
                tester,
                CarouselView::new(350.0, children).enable_splash(false),
            );
            tester.pump_and_settle();

            tester.tap(find::text("Button 1"));
            assert!(button_pressed.get());
        });
    }

    #[test]
    fn carousel_view_with_enable_splash_true_children_are_not_directly_interactive() {
        test_widgets(|tester| {
            let button_pressed = Rc::new(Cell::new(false));
            let children = (0..3)
                .map(|index| {
                    let button_pressed = Rc::clone(&button_pressed);
                    Center::new()
                        .child(crate::elevated_button::ElevatedButton::new(
                            Some(Listener::new(move |_app| button_pressed.set(true))),
                            Some(Text::new(format!("Button {index}"))),
                        ))
                        .into_widget()
                })
                .collect();
            pump_carousel(tester, CarouselView::new(350.0, children));
            tester.pump_and_settle();

            tester.tap_at(tester.get_center(find::text("Button 1")), 1);
            assert!(!button_pressed.get());
        });
    }

    #[test]
    fn carousel_view_does_not_crash_if_item_extent_is_zero() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::new(
                    0.0,
                    vec![
                        waft_widgets::Container::new()
                            .color(Colors::RED.primary())
                            .width(100.0)
                            .height(100.0)
                            .into_widget(),
                    ],
                ),
            );
        });
    }

    #[test]
    fn carousel_view_builder_creates_items_lazily() {
        test_widgets(|tester| {
            let built_items = Rc::new(RefCell::new(Vec::<i32>::new()));
            let item_builder: CarouselItemBuilder = {
                let built_items = Rc::clone(&built_items);
                Rc::new(move |_app, _context, index| {
                    built_items.borrow_mut().push(index);
                    Some(
                        Center::new()
                            .child(Text::new(format!("Item {index}")))
                            .into_widget(),
                    )
                })
            };
            pump_carousel(
                tester,
                CarouselView::builder(300.0, item_builder).item_count(1000),
            );

            // Only visible items should be built initially.
            assert!(built_items.borrow().len() < 10);
            assert!(built_items.borrow().contains(&0));
            assert!(built_items.borrow().contains(&1));

            // Scroll to a far item.
            tester.drag(find::by_type::<CarouselView>(), Offset::new(-2000.0, 0.0));
            tester.pump_and_settle();

            // Clear built items to see what's built after scrolling.
            built_items.borrow_mut().clear();

            // Force rebuild by scrolling a bit more.
            tester.drag(find::by_type::<CarouselView>(), Offset::new(-300.0, 0.0));
            tester.pump();

            // Should have built new items, not the initial ones.
            assert!(!built_items.borrow().is_empty());
            assert!(built_items.borrow().iter().all(|index| *index > 3));
        });
    }

    /// Dart's `runCarouselTest`: `animateToItem` to the middle item and back to the first.
    fn run_carousel_test(
        flex_weights: Vec<i32>,
        consume_max_weight: bool,
        number_of_children: i32,
        scroll_direction: Axis,
        reverse: bool,
    ) {
        test_widgets(move |tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 0);
            let carousel = if flex_weights.is_empty() {
                CarouselView::new(300.0, text_items(number_of_children))
            } else {
                CarouselView::weighted(flex_weights.clone(), text_items(number_of_children))
                    .consume_max_weight(consume_max_weight)
            };
            pump_carousel(
                tester,
                carousel
                    .scroll_direction(scroll_direction)
                    .reverse(reverse)
                    .controller(controller.clone())
                    .item_snapping(true),
            );

            // The index of the middle item, reversed for a reversed list.
            let middle_index = if reverse {
                number_of_children - 1 - (f64::from(number_of_children) / 2.0).round() as i32
            } else {
                (f64::from(number_of_children) / 2.0).round() as i32
            };
            let animate = |tester: &mut WidgetTester, index: i32| {
                drop(controller.handle().animate_to_item(
                    &mut tester.cell.borrow_mut(),
                    index,
                    Duration::from_millis(100),
                    Curves::ease_in_out(),
                ));
            };
            animate(tester, middle_index);
            tester.pump_and_settle();

            // Verify that the middle item is visible.
            expect(
                tester,
                &find::text(&format!("Item {middle_index}")),
                finds_one_widget(),
            );

            // Scroll to the first item.
            animate(tester, 0);
            tester.pump_and_settle();

            // Verify that the first item is visible.
            expect(tester, &find::text("Item 0"), finds_one_widget());
        });
    }

    #[test]
    fn carousel_controller_animate_to_item_weighted_horizontal_not_reversed() {
        run_carousel_test(vec![7, 1], true, 20, Axis::Horizontal, false);
    }

    #[test]
    fn carousel_controller_animate_to_item_weighted_horizontal_reversed() {
        run_carousel_test(vec![7, 1], true, 20, Axis::Horizontal, true);
    }

    #[test]
    fn carousel_controller_animate_to_item_weighted_vertical_not_reversed() {
        run_carousel_test(vec![7, 1], true, 20, Axis::Vertical, false);
    }

    #[test]
    fn carousel_controller_animate_to_item_weighted_vertical_reversed() {
        run_carousel_test(vec![7, 1], true, 20, Axis::Vertical, true);
    }

    #[test]
    fn carousel_controller_animate_to_item_weighted_not_consuming_max_weight() {
        run_carousel_test(vec![1, 7], false, 20, Axis::Horizontal, false);
    }

    #[test]
    fn carousel_controller_animate_to_item_not_weighted() {
        run_carousel_test(Vec::new(), true, 20, Axis::Horizontal, false);
    }

    #[test]
    fn carousel_view_on_index_changed_reports_the_leading_item_after_animation() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 0);
            let leading_index = Rc::new(Cell::new(0));
            pump_carousel(
                tester,
                CarouselView::new(300.0, text_items(6))
                    .controller(controller.clone())
                    .on_index_changed({
                        let leading_index = Rc::clone(&leading_index);
                        Rc::new(move |_app, index| leading_index.set(index))
                    }),
            );
            tester.pump_and_settle();

            drop(controller.handle().animate_to_item(
                &mut tester.cell.borrow_mut(),
                3,
                Duration::from_millis(100),
                Curves::ease_in_out(),
            ));
            tester.pump_and_settle();
            assert_eq!(leading_index.get(), 3);
            assert_eq!(controller.handle().leading_item(&tester.cell.borrow()), 3);
        });
    }

    #[test]
    fn carousel_view_infinite_animate_to_item_beyond_the_item_count() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 0);
            pump_carousel(
                tester,
                CarouselView::new(200.0, text_items(3))
                    .infinite(true)
                    .controller(controller.clone()),
            );

            // Verify animating to an index beyond the array length.
            drop(controller.handle().animate_to_item(
                &mut tester.cell.borrow_mut(),
                5,
                Duration::from_millis(300),
                Curves::ease(),
            ));
            tester.pump_and_settle();

            // Should show last item 2 times based on size.
            assert!(find::text_containing("Item 2").evaluate(tester).len() >= 2);
        });
    }

    #[test]
    fn carousel_view_infinite_scrolling() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::new(200.0, text_items(3)).infinite(true),
            );

            // Initial state: viewport is 800 wide, each item is 200 wide.
            // Visible items: Item 0 at [0-200], Item 1 at [200-400], Item 2 at [400-600],
            // Item 0 at [600-800] (wraps)
            expect(
                tester,
                &find::text("Item 0"),
                waft_widgets::testing::finds_n_widgets(2),
            );
            expect(tester, &find::text("Item 1"), finds_one_widget());
            expect(tester, &find::text("Item 2"), finds_one_widget());
            assert_eq!(
                tester.get_rect(get_item(0).first()),
                Rect::from_ltrb(0.0, 0.0, 200.0, 600.0)
            );
            assert_eq!(
                tester.get_rect(get_item(1).first()),
                Rect::from_ltrb(200.0, 0.0, 400.0, 600.0)
            );
            assert_eq!(
                tester.get_rect(get_item(2).first()),
                Rect::from_ltrb(400.0, 0.0, 600.0, 600.0)
            );

            // Scroll forward by 400 pixels (dragging left).
            tester.drag(find::by_type::<CarouselView>(), Offset::new(-400.0, 0.0));
            tester.pump_and_settle();

            // Visible items: Item 2 at [0-200], Item 0 at [200-400], Item 1 at [400-600],
            // Item 2 at [600-800].
            expect(tester, &find::text("Item 0"), finds_one_widget());
            expect(tester, &find::text("Item 1"), finds_one_widget());
            expect(
                tester,
                &find::text("Item 2"),
                waft_widgets::testing::finds_n_widgets(2),
            );
            assert_eq!(
                tester.get_rect(get_item(2).first()),
                Rect::from_ltrb(0.0, 0.0, 200.0, 600.0)
            );
            assert_eq!(
                tester.get_rect(get_item(0).first()),
                Rect::from_ltrb(200.0, 0.0, 400.0, 600.0)
            );
            assert_eq!(
                tester.get_rect(get_item(1).first()),
                Rect::from_ltrb(400.0, 0.0, 600.0, 600.0)
            );
        });
    }

    #[test]
    fn carousel_view_item_clip_behavior_defaults_to_anti_alias() {
        test_widgets(|tester| {
            pump_carousel(tester, CarouselView::new(300.0, text_items(3)));
            let material = find::descendant(
                find::by_type::<CarouselView>(),
                find::by_type::<crate::material::Material>(),
            )
            .first();
            tester.widget(material, |material: &crate::material::Material| {
                assert_eq!(material.clip_behavior, Clip::AntiAlias);
            });
        });
    }

    #[test]
    fn carousel_view_item_clip_behavior_respects_custom_item_clip_behavior() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::new(300.0, text_items(3)).item_clip_behavior(Clip::HardEdge),
            );
            let material = find::descendant(
                find::by_type::<CarouselView>(),
                find::by_type::<crate::material::Material>(),
            )
            .first();
            tester.widget(material, |material: &crate::material::Material| {
                assert_eq!(material.clip_behavior, Clip::HardEdge);
            });
        });
    }

    /// Dart's `tester.drag(finder, offset, kind: PointerDeviceKind.trackpad)`: a pan-zoom gesture
    /// whose pan is the whole `offset`, with no touch slop taken off it.
    fn trackpad_drag(tester: &mut WidgetTester, target: Finder, offset: Offset) {
        use waft_embedder::{PointerChange, PointerData, PointerDataPacket, PointerDeviceKind};
        let start = tester.get_center(target);
        let pointer = tester.next_pointer();
        let ratio = tester.device_pixel_ratio();
        let send = |tester: &mut WidgetTester, change: PointerChange, pan: Offset| {
            {
                let mut app = tester.cell.borrow_mut();
                waft_gestures::GestureBinding::instance(&mut app).handle_pointer_data_packet(
                    &mut app,
                    PointerDataPacket::new(vec![PointerData {
                        change,
                        kind: PointerDeviceKind::Trackpad,
                        time_stamp: Duration::ZERO,
                        device: pointer,
                        pointer_identifier: pointer,
                        physical_x: start.dx() * ratio,
                        physical_y: start.dy() * ratio,
                        pan_x: pan.dx() * ratio,
                        pan_y: pan.dy() * ratio,
                        pan_delta_x: pan.dx() * ratio,
                        pan_delta_y: pan.dy() * ratio,
                        scale: 1.0,
                        ..PointerData::default()
                    }]),
                );
                app.drain_microtasks();
            }
            tester.cell.checkpoint();
        };
        send(tester, PointerChange::PanZoomStart, Offset::ZERO);
        send(tester, PointerChange::PanZoomUpdate, offset);
        send(tester, PointerChange::PanZoomEnd, offset);
    }

    #[test]
    fn carousel_view_respects_scrolling_direction_vertical() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::new(200.0, text_items(10))
                    .padding(EdgeInsets::ZERO)
                    .scroll_direction(Axis::Vertical),
            );
            tester.pump_and_settle();

            expect_items(tester, &[0, 1, 2], &[3]);
            assert_item_rect(tester, 0, Rect::from_ltrb(0.0, 0.0, 800.0, 200.0));

            trackpad_drag(
                tester,
                find::by_type::<CarouselView>(),
                Offset::new(0.0, -200.0),
            );
            tester.pump_and_settle();
            expect_items(tester, &[3], &[0]);
        });
    }

    #[test]
    fn carousel_view_weighted_respects_scrolling_direction_vertical() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::weighted(vec![3, 2, 1], text_items(10))
                    .padding(EdgeInsets::ZERO)
                    .scroll_direction(Axis::Vertical),
            );
            tester.pump_and_settle();

            expect_items(tester, &[0, 1, 2], &[3]);
            assert_item_rect(tester, 0, Rect::from_ltrb(0.0, 0.0, 800.0, 300.0));

            trackpad_drag(
                tester,
                find::by_type::<CarouselView>(),
                Offset::new(0.0, -300.0),
            );
            tester.pump_and_settle();
            expect_items(tester, &[3], &[0]);
        });
    }

    #[test]
    fn carousel_view_weighted_respects_scrolling_direction_vertical_and_item_snapping() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::weighted(vec![3, 2, 1], text_items(10))
                    .item_snapping(true)
                    .scroll_direction(Axis::Vertical),
            );
            tester.pump_and_settle();

            expect_items(tester, &[0, 1, 2], &[3]);
            assert_item_rect(tester, 0, Rect::from_ltrb(0.0, 0.0, 800.0, 300.0));

            // Less than half of the leading item: it goes back to its position.
            trackpad_drag(
                tester,
                find::by_type::<CarouselView>(),
                Offset::new(0.0, -149.0),
            );
            tester.pump_and_settle();
            expect_items(tester, &[0, 1, 2], &[3]);

            // More than half of it: it keeps scrolling and is gone when the animation ends.
            trackpad_drag(
                tester,
                find::by_type::<CarouselView>(),
                Offset::new(0.0, -151.0),
            );
            tester.pump_and_settle();
            expect_items(tester, &[3], &[0]);
        });
    }

    #[test]
    fn carousel_view_weighted_respects_reverse_and_vertical_scroll_direction() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::weighted(vec![4, 3, 2, 1], text_items(10))
                    .reverse(true)
                    .scroll_direction(Axis::Vertical),
            );
            tester.pump_and_settle();

            let item0_height = 60.0 * 4.0;
            let item1_height = 60.0 * 3.0;
            let item2_height = 60.0 * 2.0;
            expect_items(tester, &[0, 1, 2, 3], &[]);
            assert_item_rect(
                tester,
                0,
                Rect::from_ltrb(0.0, 600.0 - item0_height, 800.0, 600.0),
            );
            assert_item_rect(
                tester,
                1,
                Rect::from_ltrb(
                    0.0,
                    600.0 - item0_height - item1_height,
                    800.0,
                    600.0 - item0_height,
                ),
            );
            assert_item_rect(
                tester,
                2,
                Rect::from_ltrb(
                    0.0,
                    600.0 - item0_height - item1_height - item2_height,
                    800.0,
                    600.0 - item0_height - item1_height,
                ),
            );
            assert_item_rect(
                tester,
                3,
                Rect::from_ltrb(
                    0.0,
                    0.0,
                    800.0,
                    600.0 - item0_height - item1_height - item2_height,
                ),
            );
        });
    }

    #[test]
    fn carousel_view_weighted_respects_reverse_vertical_scroll_direction_and_item_snapping() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::weighted(vec![4, 3, 2, 1], text_items(10))
                    .reverse(true)
                    .scroll_direction(Axis::Vertical)
                    .item_snapping(true),
            );
            tester.pump_and_settle();

            expect_items(tester, &[0, 1, 2, 3], &[4]);
            assert_item_rect(tester, 0, Rect::from_ltrb(0.0, 360.0, 800.0, 600.0));

            trackpad_drag(
                tester,
                find::by_type::<CarouselView>(),
                Offset::new(0.0, 240.0 / 2.0 - 1.0),
            );
            tester.pump_and_settle();
            expect_items(tester, &[0, 1, 2, 3], &[4]);

            trackpad_drag(
                tester,
                find::by_type::<CarouselView>(),
                Offset::new(0.0, 240.0 / 2.0 + 1.0),
            );
            tester.pump_and_settle();
            expect_items(tester, &[4], &[0]);
        });
    }

    #[test]
    fn carousel_view_respects_shrink_extent() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::new(350.0, text_items(10)).shrink_extent(300.0),
            );
            tester.pump_and_settle();

            assert_item_rect(tester, 0, Rect::from_ltrb(0.0, 0.0, 350.0, 600.0));
            assert_item_rect(tester, 1, Rect::from_ltrb(350.0, 0.0, 700.0, 600.0));
            // The extent of item 2 is 300, and only 100 is on screen.
            assert_item_rect(tester, 2, Rect::from_ltrb(700.0, 0.0, 1000.0, 600.0));

            trackpad_drag(
                tester,
                find::by_type::<CarouselView>(),
                Offset::new(-50.0, 0.0),
            );
            tester.pump();
            // Item 0 is pinned and changes size from 350 to 300.
            assert_item_rect(tester, 0, Rect::from_ltrb(0.0, 0.0, 300.0, 600.0));
            // Dragging on, the extent of item 0 stays 300 and part of it leaves the screen.
            trackpad_drag(
                tester,
                find::by_type::<CarouselView>(),
                Offset::new(-50.0, 0.0),
            );
            tester.pump();
            assert_item_rect(tester, 0, Rect::from_ltrb(-50.0, 0.0, 250.0, 600.0));
        });
    }

    #[test]
    fn while_scrolling_one_extra_item_shows_at_the_end_of_the_screen_during_items_transition() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::weighted(vec![1, 2, 4, 2, 1], text_items(10))
                    .consume_max_weight(false),
            );
            tester.pump_and_settle();

            expect_items(tester, &[0, 1, 2, 3, 4], &[]);

            // Dragging the first item to the middle: the progress of its size change is 50%,
            // and its original width is 80.
            trackpad_drag(tester, get_item(0), Offset::new(-40.0, 0.0));
            tester.pump();
            let width =
                |tester: &WidgetTester, index: i32| tester.get_rect(get_item(index)).width();
            assert_eq!(width(tester, 0), 40.0);
            assert_eq!(width(tester, 1), 160.0 - 0.5 * (160.0 - 80.0));
            assert_eq!(width(tester, 2), 320.0 - 0.5 * (320.0 - 160.0));
            assert_eq!(width(tester, 3), 160.0 + 0.5 * (320.0 - 160.0));
            assert_eq!(width(tester, 4), 80.0 + 0.5 * (160.0 - 80.0));

            let sum: f64 = (0..5).map(|index| width(tester, index)).sum();
            assert!(sum < 800.0);
            let difference = 800.0 - sum;

            // One more item shows to fill the rest of the viewport.
            expect_items(tester, &[5], &[]);
            assert_eq!(width(tester, 5), difference);
        });
    }

    fn texts_of(count: i32) -> Vec<WidgetRef> {
        (0..count)
            .map(|index| Text::new(format!("Item {index}")).into_widget())
            .collect()
    }

    fn scrollable_state(tester: &WidgetTester) -> Handle<waft_widgets::ScrollableState> {
        let scrollable = tester.element(find::by_type::<waft_widgets::Scrollable>());
        scrollable
            .state_handle::<waft_widgets::ScrollableState>(&tester.cell.borrow())
            .expect("a Scrollable element holds a ScrollableState")
    }

    fn found_n(tester: &WidgetTester, text: &str, count: usize) {
        expect(
            tester,
            &find::text(text),
            waft_widgets::testing::finds_n_widgets(count),
        );
    }

    fn animate_to(
        tester: &mut WidgetTester,
        controller: &Ref<CarouselController>,
        item: i32,
        milliseconds: u64,
    ) {
        drop(controller.handle().animate_to_item(
            &mut tester.cell.borrow_mut(),
            item,
            Duration::from_millis(milliseconds),
            Curves::linear(),
        ));
        tester.pump_and_settle();
    }

    fn leading_item(tester: &WidgetTester, controller: &Ref<CarouselController>) -> i32 {
        controller.handle().leading_item(&tester.cell.borrow())
    }

    fn offset(tester: &WidgetTester, controller: &Ref<CarouselController>) -> f64 {
        controller.handle().offset(&tester.cell.borrow())
    }

    /// Dart's `buildCarousel` of the `onIndexChanged` group.
    fn index_carousel(
        weighted: Option<Vec<i32>>,
        reverse: bool,
        item_snapping: bool,
        item_count: i32,
        controller: &Ref<CarouselController>,
        leading_index: &Rc<Cell<i32>>,
    ) -> CarouselView {
        let carousel = match weighted {
            Some(weights) => CarouselView::weighted(weights, texts_of(item_count)),
            None => CarouselView::new(300.0, texts_of(item_count)),
        };
        let leading_index = Rc::clone(leading_index);
        carousel
            .reverse(reverse)
            .item_snapping(item_snapping)
            .controller(controller.clone())
            .on_index_changed(Rc::new(move |_app, index| leading_index.set(index)))
    }

    #[test]
    fn carousel_view_shows_the_correct_item_after_animation() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 0);
            let leading_index = Rc::new(Cell::new(0));
            pump_carousel(
                tester,
                index_carousel(None, false, false, 6, &controller, &leading_index),
            );
            tester.pump_and_settle();

            animate_to(tester, &controller, 3, 200);
            assert_eq!(leading_item(tester, &controller), 3);
            assert_eq!(leading_index.get(), 3);

            animate_to(tester, &controller, 1, 200);
            assert_eq!(leading_item(tester, &controller), 1);
            assert_eq!(leading_index.get(), 1);
        });
    }

    #[test]
    fn carousel_view_weighted_shows_the_correct_item_after_animation_with_symmetric_flex_weights() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 0);
            let leading_index = Rc::new(Cell::new(0));
            pump_carousel(
                tester,
                index_carousel(
                    Some(vec![2, 5, 2]),
                    false,
                    false,
                    6,
                    &controller,
                    &leading_index,
                ),
            );
            tester.pump_and_settle();

            // Item 4 goes to the first position with the max weight (5), so item 3 leads.
            animate_to(tester, &controller, 4, 200);
            assert_eq!(leading_item(tester, &controller), 3);
            assert_eq!(leading_index.get(), 3);
            expect(tester, &find::text("Item 3"), finds_one_widget());
        });
    }

    #[test]
    fn carousel_view_weighted_shows_the_correct_item_after_animation_with_asymmetric_flex_weights()
    {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 0);
            let leading_index = Rc::new(Cell::new(0));
            pump_carousel(
                tester,
                index_carousel(
                    Some(vec![1, 2, 3, 4]),
                    false,
                    false,
                    6,
                    &controller,
                    &leading_index,
                ),
            );
            tester.pump_and_settle();

            animate_to(tester, &controller, 2, 200);
            assert_eq!(leading_item(tester, &controller), 0);
            assert_eq!(leading_index.get(), 0);
        });
    }

    #[test]
    fn carousel_view_shows_the_correct_item_after_dragging() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 0);
            let leading_index = Rc::new(Cell::new(0));
            pump_carousel(
                tester,
                index_carousel(
                    Some(vec![2, 5, 2]),
                    false,
                    true,
                    5,
                    &controller,
                    &leading_index,
                ),
            );
            tester.pump_and_settle();

            tester.drag(find::by_type::<CarouselView>(), Offset::new(-300.0, 0.0));
            tester.pump_and_settle();
            assert_eq!(leading_item(tester, &controller), 1);
            assert_eq!(leading_index.get(), 1);
        });
    }

    #[test]
    fn carousel_view_with_reverse_reports_the_correct_leading_item_after_animation() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 0);
            let leading_index = Rc::new(Cell::new(0));
            pump_carousel(
                tester,
                index_carousel(None, true, false, 6, &controller, &leading_index),
            );
            tester.pump_and_settle();

            animate_to(tester, &controller, 2, 200);
            assert_eq!(leading_item(tester, &controller), 2);
            assert_eq!(leading_index.get(), 2);
        });
    }

    #[test]
    fn carousel_view_weighted_with_reverse_reports_the_correct_leading_item_after_drag() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 0);
            let leading_index = Rc::new(Cell::new(0));
            pump_carousel(
                tester,
                index_carousel(
                    Some(vec![2, 5, 2]),
                    true,
                    true,
                    5,
                    &controller,
                    &leading_index,
                ),
            );
            tester.pump_and_settle();

            tester.drag(find::by_type::<CarouselView>(), Offset::new(300.0, 0.0));
            tester.pump_and_settle();
            assert_eq!(leading_item(tester, &controller), 1);
            assert_eq!(leading_index.get(), 1);
        });
    }

    #[test]
    fn carousel_view_infinite_scrolling_supports_bidirectional_scrolling() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::new(200.0, text_items(3)).infinite(true),
            );

            found_n(tester, "Item 0", 2);
            found_n(tester, "Item 1", 1);
            found_n(tester, "Item 2", 1);
            let initial_item0_left = tester.get_rect(get_item(0).first()).left;

            tester.drag(find::by_type::<CarouselView>(), Offset::new(200.0, 0.0));
            tester.pump_and_settle();

            // After scrolling backward, item 2 appears twice.
            found_n(tester, "Item 2", 2);
            found_n(tester, "Item 0", 1);
            found_n(tester, "Item 1", 1);
            let after_item2_left = tester.get_rect(get_item(2).first()).left;
            waft_widgets::testing::assert_more_or_less_equals_within(
                after_item2_left,
                initial_item0_left,
                10.0,
            );

            tester.drag(find::by_type::<CarouselView>(), Offset::new(-200.0, 0.0));
            tester.pump_and_settle();
            found_n(tester, "Item 0", 2);
            found_n(tester, "Item 1", 1);
            found_n(tester, "Item 2", 1);
        });
    }

    #[test]
    fn carousel_view_weighted_infinite_scrolling() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::weighted(vec![1, 2, 1], text_items(3)).infinite(true),
            );

            found_n(tester, "Item 0", 1);
            found_n(tester, "Item 1", 1);
            // Item 2 wraps to fill the padding area.
            found_n(tester, "Item 2", 1);
            assert_eq!(
                tester.get_rect(get_item(2).first()),
                Rect::from_ltrb(0.0, 0.0, 200.0, 600.0)
            );
            assert_eq!(
                tester.get_rect(get_item(0).first()),
                Rect::from_ltrb(200.0, 0.0, 600.0, 600.0)
            );
            assert_eq!(
                tester.get_rect(get_item(1).first()),
                Rect::from_ltrb(600.0, 0.0, 800.0, 600.0)
            );

            tester.drag(find::by_type::<CarouselView>(), Offset::new(-400.0, 0.0));
            tester.pump_and_settle();
            found_n(tester, "Item 1", 1);
            found_n(tester, "Item 2", 1);
        });
    }

    #[test]
    fn carousel_view_weighted_with_hero_layout_and_infinite_scrolling() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::weighted(vec![1, 7, 1], text_items(6)).infinite(true),
            );

            found_n(tester, "Item 0", 1);
            found_n(tester, "Item 1", 1);
            expect(tester, &find::text("Item 2"), finds_nothing());
            let rect0 = tester.get_rect(get_item(0).first());
            waft_widgets::testing::assert_more_or_less_equals_within(rect0.left, 800.0 / 9.0, 0.1);
            waft_widgets::testing::assert_more_or_less_equals_within(
                rect0.width(),
                800.0 * 7.0 / 9.0,
                0.1,
            );

            tester.drag(find::by_type::<CarouselView>(), Offset::new(-622.0, 0.0));
            tester.pump_and_settle();
            found_n(tester, "Item 1", 1);
            found_n(tester, "Item 2", 1);

            tester.drag(find::by_type::<CarouselView>(), Offset::new(622.0, 0.0));
            tester.pump_and_settle();
            found_n(tester, "Item 0", 1);
            found_n(tester, "Item 1", 1);

            tester.drag(find::by_type::<CarouselView>(), Offset::new(200.0, 0.0));
            tester.pump_and_settle();
            found_n(tester, "Item 5", 1);
        });
    }

    #[test]
    fn carousel_view_weighted_with_multi_browse_layout_and_infinite_scrolling() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::weighted(vec![1, 2, 3, 2, 1], text_items(10))
                    .infinite(true)
                    .consume_max_weight(false),
            );

            found_n(tester, "Item 0", 1);
            found_n(tester, "Item 1", 1);
            found_n(tester, "Item 2", 1);

            tester.drag(find::by_type::<CarouselView>(), Offset::new(-89.0, 0.0));
            tester.pump_and_settle();
            found_n(tester, "Item 1", 1);
            found_n(tester, "Item 2", 1);
            found_n(tester, "Item 3", 1);

            tester.drag(find::by_type::<CarouselView>(), Offset::new(200.0, 0.0));
            tester.pump_and_settle();
            found_n(tester, "Item 0", 1);
        });
    }

    #[test]
    fn carousel_view_infinite_leading_item_wraps_correctly_when_scrolling_forwards() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 0);
            let reported = Rc::new(RefCell::new(Vec::new()));
            pump_carousel(
                tester,
                CarouselView::new(200.0, text_items(5))
                    .item_snapping(true)
                    .infinite(true)
                    .controller(controller.clone())
                    .on_index_changed({
                        let reported = Rc::clone(&reported);
                        Rc::new(move |_app, index| reported.borrow_mut().push(index))
                    }),
            );

            let initial_item = leading_item(tester, &controller);
            for i in 1..=7 {
                tester.drag(find::by_type::<CarouselView>(), Offset::new(-200.0, 0.0));
                tester.pump_and_settle();
                assert_eq!(leading_item(tester, &controller), (initial_item + i) % 5);
            }

            assert!(reported.borrow().len() >= 7);
            assert!(
                reported
                    .borrow()
                    .iter()
                    .all(|index| (0..=4).contains(index))
            );
        });
    }

    #[test]
    fn carousel_view_infinite_leading_item_wraps_correctly_when_scrolling_backwards() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 0);
            let reported = Rc::new(RefCell::new(Vec::new()));
            pump_carousel(
                tester,
                CarouselView::new(200.0, text_items(5))
                    .item_snapping(true)
                    .infinite(true)
                    .controller(controller.clone())
                    .on_index_changed({
                        let reported = Rc::clone(&reported);
                        Rc::new(move |_app, index| reported.borrow_mut().push(index))
                    }),
            );

            let initial_item = leading_item(tester, &controller);
            for i in 1..=7 {
                tester.drag(find::by_type::<CarouselView>(), Offset::new(200.0, 0.0));
                tester.pump_and_settle();
                let expected = ((initial_item - i) % 5 + 5) % 5;
                assert_eq!(leading_item(tester, &controller), expected);
            }

            assert!(reported.borrow().len() >= 7);
            assert!(
                reported
                    .borrow()
                    .iter()
                    .all(|index| (0..=4).contains(index))
            );
        });
    }

    #[test]
    fn carousel_view_infinite_animate_to_item_scrolls_forward_to_next_item() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 0);
            pump_carousel(
                tester,
                CarouselView::new(200.0, text_items(5))
                    .item_snapping(true)
                    .infinite(true)
                    .controller(controller.clone()),
            );

            let initial_offset = offset(tester, &controller);
            let initial_item = leading_item(tester, &controller);
            animate_to(tester, &controller, initial_item + 1, 200);
            assert_eq!(leading_item(tester, &controller), (initial_item + 1) % 5);
            assert!(offset(tester, &controller) > initial_offset);
        });
    }

    #[test]
    fn carousel_view_weighted_infinite_animate_to_item_scrolls_forward_to_next_item() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 0);
            pump_carousel(
                tester,
                CarouselView::weighted(vec![1, 7, 1], text_items(5))
                    .item_snapping(true)
                    .infinite(true)
                    .controller(controller.clone()),
            );

            let initial_offset = offset(tester, &controller);
            animate_to(tester, &controller, 1, 200);
            assert!(offset(tester, &controller) > initial_offset);
            found_n(tester, "Item 1", 1);
        });
    }

    #[test]
    fn carousel_view_infinite_animate_to_item_scrolls_forward_even_to_item_just_behind() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 0);
            pump_carousel(
                tester,
                CarouselView::new(200.0, text_items(5))
                    .item_snapping(true)
                    .infinite(true)
                    .controller(controller.clone()),
            );

            animate_to(tester, &controller, 2, 200);
            assert_eq!(leading_item(tester, &controller), 2);
            let offset_at_item2 = offset(tester, &controller);

            animate_to(tester, &controller, 1, 200);
            assert_eq!(leading_item(tester, &controller), 1);
            assert!(offset(tester, &controller) > offset_at_item2);
        });
    }

    #[test]
    fn carousel_view_weighted_infinite_animate_to_item_scrolls_forward_even_to_item_just_behind() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 0);
            pump_carousel(
                tester,
                CarouselView::weighted(vec![1, 7, 1], text_items(5))
                    .item_snapping(true)
                    .infinite(true)
                    .controller(controller.clone()),
            );

            animate_to(tester, &controller, 2, 200);
            let offset_at_item2 = offset(tester, &controller);

            animate_to(tester, &controller, 1, 200);
            assert!(offset(tester, &controller) > offset_at_item2);
            found_n(tester, "Item 1", 1);
        });
    }

    fn rtl_carousel(reverse: bool) -> CarouselView {
        let item = |index: i32| {
            Center::new()
                .key(waft_foundation::ValueKey::new(index).into())
                .child(Text::new(format!("Item {index}")))
                .into_widget()
        };
        CarouselView::new(600.0, vec![item(0), item(1)]).reverse(reverse)
    }

    fn pump_rtl_carousel(tester: &mut WidgetTester, reverse: bool) {
        let theme = ThemeData::builder().build(&tester.cell.borrow());
        tester.pump_widget(material_app(
            theme,
            waft_widgets::Directionality::new(
                waft_painting::TextDirection::Rtl,
                Scaffold::new().body(rtl_carousel(reverse)),
            )
            .into_widget(),
        ));
        tester.pump_and_settle();
    }

    #[test]
    fn scrolls_correctly_in_rtl() {
        test_widgets(|tester| {
            pump_rtl_carousel(tester, false);
            let item0 = find::by_key(waft_foundation::ValueKey::new(0));
            let before = tester.get_center(item0.clone()).dx();

            tester.drag(find::by_type::<CarouselView>(), Offset::new(500.0, 0.0));
            tester.pump_and_settle();

            let after = tester.get_center(item0).dx();
            assert!(after > before);
            let axis_direction = scrollable_state(tester).axis_direction(&tester.cell.borrow());
            assert_eq!(axis_direction, AxisDirection::Left);
        });
    }

    #[test]
    fn scrolls_correctly_in_rtl_with_reverse() {
        test_widgets(|tester| {
            pump_rtl_carousel(tester, true);
            let item0 = find::by_key(waft_foundation::ValueKey::new(0));
            let before = tester.get_center(item0.clone()).dx();

            tester.drag(find::by_type::<CarouselView>(), Offset::new(-500.0, 0.0));
            tester.pump_and_settle();

            let after = tester.get_center(item0).dx();
            assert!(after < before);
            let axis_direction = scrollable_state(tester).axis_direction(&tester.cell.borrow());
            assert_eq!(axis_direction, AxisDirection::Right);
        });
    }

    #[test]
    fn snaps_back_to_item_boundary_after_partial_scroll() {
        test_widgets(|tester| {
            pump_carousel(
                tester,
                CarouselView::new(
                    600.0,
                    vec![
                        SizedBox::new().width(600.0).height(600.0).into_widget(),
                        SizedBox::new().width(600.0).height(600.0).into_widget(),
                    ],
                )
                .item_snapping(true),
            );

            tester.drag(find::by_type::<CarouselView>(), Offset::new(-100.0, 0.0));
            tester.pump_and_settle();

            let position = scrollable_state(tester).position(&tester.cell.borrow());
            assert_eq!(position.pixels(&tester.cell.borrow()), 0.0);
        });
    }

    #[test]
    fn carousel_view_weighted_does_not_crash_at_finite_trailing_scroll_offset() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 0);
            tester.set_surface_size(Some(waft_embedder::Size::new(750.0, 1000.0)));
            pump_carousel(
                tester,
                CarouselView::weighted(vec![1, 7], text_items(5)).controller(controller.clone()),
            );

            controller
                .handle()
                .jump_to(&mut tester.cell.borrow_mut(), 461.5);
            tester.pump();
            tester.set_surface_size(None);
        });
    }

    #[test]
    fn carousel_controller_animate_to_item_weighted_horizontal_reversed_one_to_seven() {
        run_carousel_test(vec![1, 7], true, 20, Axis::Horizontal, true);
    }

    #[test]
    fn carousel_controller_animate_to_item_weighted_vertical_not_reversed_one_to_seven() {
        run_carousel_test(vec![1, 7], false, 20, Axis::Vertical, false);
    }

    #[test]
    fn carousel_controller_animate_to_item_weighted_vertical_reversed_not_consuming_max_weight() {
        run_carousel_test(vec![1, 7], false, 20, Axis::Vertical, true);
    }

    #[test]
    fn carousel_controller_animate_to_item_weighted_vertical_reversed_one_to_seven() {
        run_carousel_test(vec![1, 7], true, 20, Axis::Vertical, true);
    }

    #[test]
    fn carousel_controller_animate_to_item_horizontal_reversed() {
        run_carousel_test(Vec::new(), true, 10, Axis::Horizontal, true);
    }

    #[test]
    fn carousel_controller_animate_to_item_vertical_not_reversed() {
        run_carousel_test(Vec::new(), true, 10, Axis::Vertical, false);
    }

    #[test]
    fn carousel_controller_animate_to_item_vertical_reversed() {
        run_carousel_test(Vec::new(), true, 10, Axis::Vertical, true);
    }

    #[test]
    fn carousel_view_positions_items_correctly() {
        test_widgets(|tester| {
            let number_of_children = 5;
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 0);
            pump_carousel(
                tester,
                CarouselView::weighted(vec![2, 3, 1], text_items(number_of_children))
                    .controller(controller.clone())
                    .item_snapping(true),
            );
            tester.pump_and_settle();

            let carousel_rect = tester.get_rect(find::by_type::<CarouselView>());
            for i in 0..number_of_children {
                drop(controller.handle().animate_to_item(
                    &mut tester.cell.borrow_mut(),
                    i,
                    Duration::from_millis(300),
                    Curves::ease_in_out(),
                ));
                tester.pump_and_settle();

                expect(
                    tester,
                    &find::text(&format!("Item {i}")),
                    finds_one_widget(),
                );
                let item_rect = tester.get_rect(find::text(&format!("Item {i}")));
                assert!(item_rect.left >= carousel_rect.left);
                assert!(item_rect.right <= carousel_rect.right);
            }
        });
    }

    #[test]
    fn carousel_view_infinite_animate_to_item_wraps_to_the_item_behind() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 0);
            pump_carousel(
                tester,
                CarouselView::new(200.0, text_items(3))
                    .infinite(true)
                    .controller(controller.clone()),
            );

            drop(controller.handle().animate_to_item(
                &mut tester.cell.borrow_mut(),
                5,
                Duration::from_millis(300),
                Curves::ease(),
            ));
            tester.pump_and_settle();

            expect(
                tester,
                &find::text_containing("Item 2"),
                waft_widgets::testing::finds_at_least_n_widgets(2),
            );
        });
    }

    #[test]
    fn carousel_view_weighted_builder_creates_items_lazily_with_flex_weights() {
        test_widgets(|tester| {
            let built_items = Rc::new(RefCell::new(Vec::<i32>::new()));
            let item_builder: CarouselItemBuilder = {
                let built_items = Rc::clone(&built_items);
                Rc::new(move |_app, _context, index| {
                    built_items.borrow_mut().push(index);
                    Some(
                        Center::new()
                            .child(Text::new(format!("Item {index}")))
                            .into_widget(),
                    )
                })
            };
            pump_carousel(
                tester,
                CarouselView::weighted_builder(vec![2, 3, 1], item_builder).item_count(1000),
            );

            assert!(built_items.borrow().len() < 10);
            assert!(built_items.borrow().contains(&0));
            assert!(built_items.borrow().contains(&1));

            tester.drag(find::by_type::<CarouselView>(), Offset::new(-2000.0, 0.0));
            tester.pump_and_settle();
            built_items.borrow_mut().clear();

            tester.drag(find::by_type::<CarouselView>(), Offset::new(-300.0, 0.0));
            tester.pump();
            assert!(!built_items.borrow().is_empty());
            assert!(built_items.borrow().iter().all(|index| *index > 3));
        });
    }

    fn sized_item() -> WidgetRef {
        SizedBox::expand()
            .key(waft_foundation::ValueKey::new("item").into())
            .into_widget()
    }

    #[test]
    fn item_extent_can_be_set_to_infinity() {
        test_widgets(|tester| {
            pump_carousel(tester, CarouselView::new(f64::INFINITY, vec![sized_item()]));

            let size = tester.get_size(find::by_key(waft_foundation::ValueKey::new("item")));
            // The default horizontal padding is 4 on each side.
            assert_eq!(size.width(), 800.0 - 8.0);
        });
    }

    #[test]
    fn item_extent_is_applied_when_viewport_dimension_is_updated() {
        test_widgets(|tester| {
            let item_extent = 60.0;
            for (index, size) in [Size::new(0.0, 0.0), Size::new(500.0, 400.0)]
                .into_iter()
                .enumerate()
            {
                tester.set_surface_size(Some(size));
                let behavior = ScrollBehaviorRef::new(
                    ScrollBehaviorRef::new(waft_widgets::PlainScrollBehavior)
                        .copy_with()
                        .scrollbars(index % 2 == 0),
                );
                let theme = ThemeData::builder().build(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Scaffold::new()
                        .body(Center::new().child(ScrollConfiguration::new(
                            behavior,
                            CarouselView::new(item_extent, vec![sized_item()]),
                        )))
                        .into_widget(),
                ));
            }

            let size = tester.get_size(find::by_key(waft_foundation::ValueKey::new("item")));
            assert_eq!(size.width(), item_extent - 8.0);
            tester.set_surface_size(None);
        });
    }

    #[test]
    fn carousel_view_weighted_weights_are_applied_when_viewport_dimension_is_updated() {
        test_widgets(|tester| {
            let controller = CarouselController::new(&mut tester.cell.borrow_mut(), 1);
            let (first_weight, second_weight) = (2, 3);
            let surface_width = 500.0;
            for (index, size) in [Size::new(0.0, 0.0), Size::new(surface_width, 400.0)]
                .into_iter()
                .enumerate()
            {
                tester.set_surface_size(Some(size));
                let behavior = ScrollBehaviorRef::new(
                    ScrollBehaviorRef::new(waft_widgets::PlainScrollBehavior)
                        .copy_with()
                        .scrollbars(index % 2 == 0),
                );
                let theme = ThemeData::builder().build(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Scaffold::new()
                        .body(
                            Center::new().child(ScrollConfiguration::new(
                                behavior,
                                CarouselView::weighted(
                                    vec![first_weight, second_weight],
                                    text_items(20),
                                )
                                .controller(controller.clone()),
                            )),
                        )
                        .into_widget(),
                ));
            }

            let total_weight = f64::from(first_weight + second_weight);
            expect(tester, &find::text("Item 0"), finds_one_widget());
            expect(tester, &find::text("Item 1"), finds_one_widget());
            assert_eq!(
                tester.get_rect(get_item(0)).width(),
                surface_width * f64::from(first_weight) / total_weight
            );
            assert_eq!(
                tester.get_rect(get_item(1)).width(),
                surface_width * f64::from(second_weight) / total_weight
            );
            tester.set_surface_size(None);
        });
    }

    #[test]
    fn the_shrink_extent_should_keep_the_same_when_the_item_is_tapped() {
        test_widgets(|tester| {
            let theme = ThemeData::builder().build(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Scaffold::new()
                    .body(
                        Center::new().child(
                            waft_widgets::ConstrainedBox::new(waft_rendering::BoxConstraints {
                                max_height: 200.0,
                                ..waft_rendering::BoxConstraints::new()
                            })
                            .child(
                                CarouselView::new(330.0, text_items(20))
                                    .on_tap(Rc::new(|_app, _index| {})),
                            ),
                        ),
                    )
                    .into_widget(),
            ));
            tester.pump_and_settle();

            assert_eq!(tester.get_rect(get_item(0)).width(), 330.0);

            tester.tap(find::ancestor(
                find::text("Item 1"),
                find::by_type::<waft_widgets::Stack>(),
            ));
            tester.pump_and_settle();

            assert_eq!(tester.get_rect(get_item(0)).width(), 330.0);
            assert_eq!(tester.get_rect(get_item(1)).width(), 330.0);
            // Item 2 shrinks: its width is 800 - 330 - 330.
            assert_eq!(tester.get_rect(get_item(2)).width(), 140.0);
        });
    }
}
