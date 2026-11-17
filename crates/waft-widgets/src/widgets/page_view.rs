//! Flutter counterpart: `widgets/page_view.dart`.
//!
//! `PageView`'s diagnostics wait for diagnostics.

#![allow(clippy::too_many_arguments)]

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::Curve;
use waft_embedder::{Clip, clamp_double};
use waft_foundation::{
    App, ChangeNotifier, ChangeNotifierData, CompleterFuture, Handle, OptionRetained,
    PRECISION_ERROR_TOLERANCE, ValueChanged,
};
use waft_gestures::DragStartBehavior;
use waft_painting::{Axis, AxisDirection, flip_axis_direction, text_direction_to_axis_direction};
use waft_physics::{ScrollSpringSimulation, Simulation, Tolerance};
use waft_rendering::{
    AnyRenderObject, HitTestBehavior, ScrollCacheExtent, ScrollDirection, ViewportOffset,
    ViewportOffsetRef,
};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::basic::Directionality;
use crate::widgets::notification_listener::NotificationListener;
use crate::widgets::page_storage::PageStorage;
use crate::widgets::scroll_activity::ScrollActivityDelegate;
use crate::widgets::scroll_configuration::{ScrollBehaviorRef, ScrollConfiguration};
use crate::widgets::scroll_context::ScrollContext;
use crate::widgets::scroll_controller::{
    ScrollController, ScrollControllerCallback, ScrollControllerData, ScrollControllerLeaf,
};
use crate::widgets::scroll_delegate::{
    ChildIndexGetter, NullableIndexedWidgetBuilder, SliverChildBuilderDelegate,
    SliverChildDelegateRef, SliverChildListDelegate,
};
use crate::widgets::scroll_metrics::{FixedScrollMetrics, ScrollMetrics};
use crate::widgets::scroll_notification::{ScrollNotification, ScrollUpdateNotification};
use crate::widgets::scroll_physics::{ScrollPhysics, ScrollPhysicsBase, ScrollPhysicsRef};
use crate::widgets::scroll_position::{
    AnyScrollPosition, ScrollPosition, ScrollPositionAlignmentPolicy, ScrollPositionBase,
    ScrollPositionData, ScrollPositionRef,
};
use crate::widgets::scroll_position_with_single_context::{
    ScrollPositionWithSingleContext, ScrollPositionWithSingleContextData,
    ScrollPositionWithSingleContextLeaf,
};
use crate::widgets::scrollable::{Scrollable, ViewportBuilder};
use crate::widgets::sliver_fill::SliverFillViewport;
use crate::widgets::viewport::Viewport;
use waft_foundation::Ref;

// ---------------------------------------------------------------------------------------------
// PageController

/// A controller for [`PageView`].
///
/// A page controller lets you manipulate which page is visible in a [`PageView`]. In addition to
/// being able to control the pixel offset of the content inside the [`PageView`], a
/// [`PageController`] also lets you control the offset in terms of pages, which are increments
/// of the viewport size.
///
/// See also:
///
///  * [`PageView`], which is the widget this object controls.
pub struct PageController {
    change_notifier: ChangeNotifierData,
    scroll_controller: ScrollControllerData,
    initial_page: i32,
    keep_page: bool,
    viewport_fraction: f64,
}

impl PageController {
    /// Creates a page controller.
    ///
    /// Dart's defaults: `initial_page` 0, `keep_page` true, `viewport_fraction` 1.0, no
    /// `on_attach` and no `on_detach`; [`PageController::default`] makes that one.
    pub fn new(
        app: &mut App,
        initial_page: i32,
        keep_page: bool,
        viewport_fraction: f64,
        on_attach: Option<ScrollControllerCallback>,
        on_detach: Option<ScrollControllerCallback>,
    ) -> Ref<PageController> {
        debug_assert!(viewport_fraction > 0.0);
        app.create(PageController {
            change_notifier: ChangeNotifierData::new(),
            scroll_controller: ScrollControllerData::new(0.0, true, None, on_attach, on_detach),
            initial_page,
            keep_page,
            viewport_fraction,
        })
    }

    /// Dart's `PageController()`, with every argument at its default.
    pub fn default(app: &mut App) -> Ref<PageController> {
        PageController::new(app, 0, true, 1.0, None, None)
    }

    /// The page to show when first creating the [`PageView`].
    pub fn initial_page(self: Handle<Self>, app: &App) -> i32 {
        app.get(self).initial_page
    }

    /// Save the current [`page`](Self::page) with [`PageStorage`] and restore it if this
    /// controller's scrollable is recreated.
    ///
    /// If this property is set to false, the current [`page`](Self::page) is never saved and
    /// [`initial_page`](Self::initial_page) is always used to initialize the scroll offset. If
    /// true (the default), the initial page is used the first time the controller's scrollable
    /// is created, since there's isn't a page to restore yet. Subsequently the saved page is
    /// restored and [`initial_page`](Self::initial_page) is ignored.
    ///
    /// See also:
    ///
    ///  * `PageStorageKey`, which should be used when more than one scrollable appears in the
    ///    same route, to distinguish the [`PageStorage`] locations used to save scroll offsets.
    pub fn keep_page(self: Handle<Self>, app: &App) -> bool {
        app.get(self).keep_page
    }

    /// The fraction of the viewport that each page should occupy.
    ///
    /// Defaults to 1.0, which means each page fills the viewport in the scrolling direction.
    pub fn viewport_fraction(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).viewport_fraction
    }

    /// The current page displayed in the controlled [`PageView`].
    ///
    /// There are circumstances that this [`PageController`] can't know the current page.
    /// Reading [`page`](Self::page) will panic in the following cases:
    ///
    /// 1. No [`PageView`] is currently using this [`PageController`]. Once a [`PageView`]
    ///    starts using this [`PageController`], the new [`page`](Self::page) position will be
    ///    derived:
    ///
    ///    * First, based on the attached [`PageView`]'s [`BuildContext`] and the position saved
    ///      at that context's [`PageStorage`] if [`keep_page`](Self::keep_page) is true.
    ///    * Second, from the [`PageController`]'s [`initial_page`](Self::initial_page).
    ///
    /// 2. More than one [`PageView`] using the same [`PageController`].
    ///
    /// The [`has_clients`](ScrollControllerLeaf::has_clients) property can be used to check if a
    /// [`PageView`] is attached prior to accessing [`page`](Self::page).
    pub fn page(self: Handle<Self>, app: &App) -> Option<f64> {
        let positions = self.positions(app);
        debug_assert!(
            !positions.is_empty(),
            "PageController.page cannot be accessed before a PageView is built with it."
        );
        debug_assert!(
            positions.len() == 1,
            "The page property cannot be read when multiple PageViews are attached to the same \
             PageController."
        );
        self.page_position(app).page(app)
    }

    /// Dart's `this.position as _PagePosition`.
    fn page_position(self: Handle<Self>, app: &App) -> Handle<PagePosition> {
        self.position(app)
            .downcast::<PagePosition>(app)
            .expect("a PageController creates the positions of the PageViews it controls")
    }

    fn debug_check_page_controller_attached(self: Handle<Self>, app: &App) -> bool {
        let positions = self.positions(app);
        debug_assert!(
            !positions.is_empty(),
            "PageController is not attached to a PageView."
        );
        debug_assert!(
            positions.len() == 1,
            "Multiple PageViews are attached to the same PageController."
        );
        true
    }

    /// Animates the controlled [`PageView`] from the current page to the given page.
    ///
    /// The animation lasts for the given duration and follows the given curve. The returned
    /// future resolves when the animation completes.
    pub fn animate_to_page(
        self: Handle<Self>,
        app: &mut App,
        page: i32,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) -> CompleterFuture<()> {
        debug_assert!(self.debug_check_page_controller_attached(app));
        let position = self.page_position(app);
        if app.get(position).cached_page.is_some() {
            app.get_mut(position).cached_page = Some(f64::from(page));
            return CompleterFuture::ready(());
        }

        if !position.has_viewport_dimension(app) {
            app.get_mut(position).page_to_use_on_startup = f64::from(page);
            return CompleterFuture::ready(());
        }

        let to = position.get_pixels_from_page(app, f64::from(page));
        ViewportOffset::animate_to(position, app, to, duration, curve)
    }

    /// Changes which page is displayed in the controlled [`PageView`].
    ///
    /// Jumps the page position from its current value to the given value, without animation,
    /// and without checking if the new value is in range.
    pub fn jump_to_page(self: Handle<Self>, app: &mut App, page: i32) {
        debug_assert!(self.debug_check_page_controller_attached(app));
        let position = self.page_position(app);
        if app.get(position).cached_page.is_some() {
            app.get_mut(position).cached_page = Some(f64::from(page));
            return;
        }

        if !position.has_viewport_dimension(app) {
            app.get_mut(position).page_to_use_on_startup = f64::from(page);
            return;
        }

        let to = position.get_pixels_from_page(app, f64::from(page));
        ViewportOffset::jump_to(position, app, to);
    }

    /// Animates the controlled [`PageView`] to the next page.
    ///
    /// The animation lasts for the given duration and follows the given curve. The returned
    /// future resolves when the animation completes.
    pub fn next_page(
        self: Handle<Self>,
        app: &mut App,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) -> CompleterFuture<()> {
        let page = self
            .page(app)
            .expect("an attached page view has a page")
            .round() as i32;
        self.animate_to_page(app, page + 1, duration, curve)
    }

    /// Animates the controlled [`PageView`] to the previous page.
    ///
    /// The animation lasts for the given duration and follows the given curve. The returned
    /// future resolves when the animation completes.
    pub fn previous_page(
        self: Handle<Self>,
        app: &mut App,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) -> CompleterFuture<()> {
        let page = self
            .page(app)
            .expect("an attached page view has a page")
            .round() as i32;
        self.animate_to_page(app, page - 1, duration, curve)
    }
}

impl ChangeNotifier for PageController {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl ScrollControllerLeaf for PageController {
    crate::scroll_controller_accessors!();

    fn create_scroll_position(
        self: Handle<Self>,
        app: &mut App,
        physics: ScrollPhysicsRef,
        context: Rc<dyn ScrollContext>,
        old_position: Option<AnyScrollPosition>,
    ) -> ScrollPositionRef {
        let this = app.get(self);
        let (initial_page, keep_page, viewport_fraction) =
            (this.initial_page, this.keep_page, this.viewport_fraction);
        PagePosition::new(
            app,
            physics,
            context,
            initial_page,
            keep_page,
            viewport_fraction,
            old_position,
        )
        .into_erased()
    }

    fn attach(self: Handle<Self>, app: &mut App, position: ScrollPositionRef) {
        let attached = position.handle();
        ScrollController::attach(self.as_controller(), app, position);
        let page_position = attached
            .downcast::<PagePosition>(app)
            .expect("a PageController attaches the positions it creates");
        let viewport_fraction = app.get(self).viewport_fraction;
        page_position.set_viewport_fraction(app, viewport_fraction);
    }
}

// ---------------------------------------------------------------------------------------------
// PageMetrics

/// Metrics for a [`PageView`].
///
/// The metrics are available on [`ScrollNotification`]s generated from [`PageView`]s.
#[derive(Clone, Copy)]
pub struct PageMetrics {
    fixed: FixedScrollMetrics,
    viewport_fraction: f64,
    // Set only on the snapshot a page view's position makes of itself
    // (`ScrollPosition::copy_with`), which stands for the position where Dart hands the
    // position itself to the physics: `PageScrollPhysics` answers it as Dart's
    // `position is _PagePosition`. Every copy drops it, as Dart's copy is no position.
    position: Option<PagePositionSnapshot>,
}

/// What a `_PagePosition` answers beyond its [`PageMetrics`] when its snapshot was taken.
#[derive(Clone, Copy)]
struct PagePositionSnapshot {
    cached_page: Option<f64>,
}

impl PageMetrics {
    /// Creates an immutable snapshot of values associated with a [`PageView`].
    pub fn new(
        min_scroll_extent: Option<f64>,
        max_scroll_extent: Option<f64>,
        pixels: Option<f64>,
        viewport_dimension: Option<f64>,
        axis_direction: AxisDirection,
        viewport_fraction: f64,
        device_pixel_ratio: f64,
    ) -> PageMetrics {
        PageMetrics {
            fixed: FixedScrollMetrics::new(
                min_scroll_extent,
                max_scroll_extent,
                pixels,
                viewport_dimension,
                axis_direction,
                device_pixel_ratio,
            ),
            viewport_fraction,
            position: None,
        }
    }

    /// Dart's `PageMetrics.copyWith`; the `with_*` setters on the copy replace its values.
    pub fn copy_with(&self) -> PageMetrics {
        PageMetrics {
            position: None,
            ..*self
        }
    }

    /// Dart `copyWith(minScrollExtent:)`.
    pub fn with_min_scroll_extent(mut self, min_scroll_extent: f64) -> PageMetrics {
        self.fixed = self.fixed.with_min_scroll_extent(min_scroll_extent);
        self
    }

    /// Dart `copyWith(maxScrollExtent:)`.
    pub fn with_max_scroll_extent(mut self, max_scroll_extent: f64) -> PageMetrics {
        self.fixed = self.fixed.with_max_scroll_extent(max_scroll_extent);
        self
    }

    /// Dart `copyWith(pixels:)`.
    pub fn with_pixels(mut self, pixels: f64) -> PageMetrics {
        self.fixed = self.fixed.with_pixels(pixels);
        self
    }

    /// Dart `copyWith(viewportDimension:)`.
    pub fn with_viewport_dimension(mut self, viewport_dimension: f64) -> PageMetrics {
        self.fixed = self.fixed.with_viewport_dimension(viewport_dimension);
        self
    }

    /// Dart `copyWith(axisDirection:)`.
    pub fn with_axis_direction(mut self, axis_direction: AxisDirection) -> PageMetrics {
        self.fixed = self.fixed.with_axis_direction(axis_direction);
        self
    }

    /// Dart `copyWith(viewportFraction:)`.
    pub fn with_viewport_fraction(mut self, viewport_fraction: f64) -> PageMetrics {
        self.viewport_fraction = viewport_fraction;
        self
    }

    /// Dart `copyWith(devicePixelRatio:)`.
    pub fn with_device_pixel_ratio(mut self, device_pixel_ratio: f64) -> PageMetrics {
        self.fixed = self.fixed.with_device_pixel_ratio(device_pixel_ratio);
        self
    }

    /// The current page displayed in the [`PageView`].
    pub fn page(&self) -> Option<f64> {
        Some(
            clamp_double(
                self.pixels(),
                self.min_scroll_extent(),
                self.max_scroll_extent(),
            )
            .max(0.0)
                / (self.viewport_dimension() * self.viewport_fraction).max(1.0),
        )
    }

    /// The fraction of the viewport that each page occupies.
    ///
    /// Used to compute [`page`](Self::page) from the current [`pixels`](ScrollMetrics::pixels).
    pub fn viewport_fraction(&self) -> f64 {
        self.viewport_fraction
    }

    /// Dart's `_PagePosition.page`, answered from the position's snapshot.
    fn position_page(&self, position: PagePositionSnapshot) -> Option<f64> {
        if !self.has_pixels() {
            return None;
        }
        if self.has_content_dimensions() {
            Some(position.cached_page.unwrap_or_else(|| {
                page_from_pixels(
                    clamp_double(
                        self.pixels(),
                        self.min_scroll_extent(),
                        self.max_scroll_extent(),
                    ),
                    self.viewport_dimension(),
                    self.viewport_fraction,
                )
            }))
        } else {
            None
        }
    }

    /// Dart's `_PagePosition.getPixelsFromPage`, answered from the position's snapshot.
    fn position_pixels_from_page(&self, page: f64) -> f64 {
        pixels_from_page(page, self.viewport_dimension(), self.viewport_fraction)
    }
}

impl ScrollMetrics for PageMetrics {
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

impl Debug for PageMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PageMetrics({:.1}..[{:.1}]..{:.1})",
            self.extent_before(),
            self.extent_inside(),
            self.extent_after()
        )
    }
}

// ---------------------------------------------------------------------------------------------
// _PagePosition

/// Dart's `_PagePosition`: a [`ScrollPositionWithSingleContextLeaf`] that keeps its offset in
/// pages of [`viewport_fraction`](Self::viewport_fraction) of the viewport.
struct PagePosition {
    change_notifier: ChangeNotifierData,
    scroll_position: ScrollPositionData,
    single_context: ScrollPositionWithSingleContextData,
    page_to_use_on_startup: f64,
    // When the viewport has a zero-size, the `page` can not
    // be retrieved by `getPageFromPixels`, so we need to cache the page
    // for use when resizing the viewport to non-zero next time.
    cached_page: Option<f64>,
    viewport_fraction: f64,
}

impl PagePosition {
    fn new(
        app: &mut App,
        physics: ScrollPhysicsRef,
        context: Rc<dyn ScrollContext>,
        initial_page: i32,
        keep_page: bool,
        viewport_fraction: f64,
        old_position: Option<AnyScrollPosition>,
    ) -> Ref<PagePosition> {
        debug_assert!(viewport_fraction > 0.0);
        let scroll_position = ScrollPositionData::new(app, physics, context, keep_page, None);
        let this = app.create(PagePosition {
            change_notifier: ChangeNotifierData::new(),
            scroll_position,
            single_context: ScrollPositionWithSingleContextData::new(),
            page_to_use_on_startup: f64::from(initial_page),
            cached_page: None,
            viewport_fraction,
        });
        ScrollPositionWithSingleContext::init(this.handle(), app, None, old_position);
        this
    }

    /// The fraction of the viewport that each page occupies.
    fn viewport_fraction(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).viewport_fraction
    }

    fn set_viewport_fraction(self: Handle<Self>, app: &mut App, value: f64) {
        if app.get(self).viewport_fraction == value {
            return;
        }
        let old_page = self.page(app);
        app.get_mut(self).viewport_fraction = value;
        if let Some(old_page) = old_page {
            let pixels = self.get_pixels_from_page(app, old_page);
            self.force_pixels(app, pixels);
        }
    }

    /// Dart's `_initialPageOffset`; see [`initial_page_offset`].
    fn initial_page_offset(self: Handle<Self>, app: &App) -> f64 {
        initial_page_offset(self.viewport_dimension(app), self.viewport_fraction(app))
    }

    fn get_page_from_pixels(
        self: Handle<Self>,
        app: &App,
        pixels: f64,
        viewport_dimension: f64,
    ) -> f64 {
        page_from_pixels(pixels, viewport_dimension, self.viewport_fraction(app))
    }

    fn get_pixels_from_page(self: Handle<Self>, app: &App, page: f64) -> f64 {
        pixels_from_page(
            page,
            self.viewport_dimension(app),
            self.viewport_fraction(app),
        )
    }

    fn page(self: Handle<Self>, app: &App) -> Option<f64> {
        if !self.has_pixels(app) {
            return None;
        }
        debug_assert!(
            self.has_content_dimensions(app) || !self.have_dimensions(app),
            "Page value is only available after content dimensions are established."
        );
        if self.has_content_dimensions(app) || self.have_dimensions(app) {
            Some(app.get(self).cached_page.unwrap_or_else(|| {
                self.get_page_from_pixels(
                    app,
                    clamp_double(
                        self.pixels(app),
                        self.min_scroll_extent(app),
                        self.max_scroll_extent(app),
                    ),
                    self.viewport_dimension(app),
                )
            }))
        } else {
            None
        }
    }

    /// The page this position saves, Dart's `_cachedPage ?? getPageFromPixels(pixels,
    /// viewportDimension)`.
    fn page_to_save(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).cached_page.unwrap_or_else(|| {
            self.get_page_from_pixels(app, self.pixels(app), self.viewport_dimension(app))
        })
    }
}

// The amount of offset that will be added to [minScrollExtent] and subtracted
// from [maxScrollExtent], such that every page will properly snap to the center
// of the viewport when viewportFraction is greater than 1.
//
// The value is 0 if viewportFraction is less than or equal to 1, larger than 0
// otherwise.
fn initial_page_offset(viewport_dimension: f64, viewport_fraction: f64) -> f64 {
    (viewport_dimension * (viewport_fraction - 1.0) / 2.0).max(0.0)
}

/// Dart's `_PagePosition.getPageFromPixels`, over the values it reads.
fn page_from_pixels(pixels: f64, viewport_dimension: f64, viewport_fraction: f64) -> f64 {
    debug_assert!(viewport_dimension > 0.0);
    let actual = (pixels - initial_page_offset(viewport_dimension, viewport_fraction)).max(0.0)
        / (viewport_dimension * viewport_fraction);
    let round = actual.round();
    if (actual - round).abs() < PRECISION_ERROR_TOLERANCE {
        return round;
    }
    actual
}

/// Dart's `_PagePosition.getPixelsFromPage`, over the values it reads.
fn pixels_from_page(page: f64, viewport_dimension: f64, viewport_fraction: f64) -> f64 {
    page * viewport_dimension * viewport_fraction
        + initial_page_offset(viewport_dimension, viewport_fraction)
}

impl ChangeNotifier for PagePosition {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl ViewportOffset for PagePosition {
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
        let page = match old_pixels {
            None => app.get(self).page_to_use_on_startup,
            // If resize from zero, we should use the _cachedPage to recover the state.
            Some(_) if old_viewport_dimensions == Some(0.0) => app
                .get(self)
                .cached_page
                .expect("a position resized from zero cached its page"),
            Some(old_pixels) => self.get_page_from_pixels(
                app,
                old_pixels,
                old_viewport_dimensions.expect("a position with pixels had a viewport dimension"),
            ),
        };
        let new_pixels = self.get_pixels_from_page(app, page);

        // If the viewportDimension is zero, cache the page
        // in case the viewport is resized to be non-zero.
        app.get_mut(self).cached_page = (viewport_dimension == 0.0).then_some(page);

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
        let initial_page_offset = self.initial_page_offset(app);
        let new_min_scroll_extent = min_scroll_extent + initial_page_offset;
        ScrollPositionBase::apply_content_dimensions(
            self,
            app,
            new_min_scroll_extent,
            new_min_scroll_extent.max(max_scroll_extent - initial_page_offset),
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

    fn user_scroll_direction(self: Handle<Self>, app: &App) -> ScrollDirection {
        ScrollPositionWithSingleContextLeaf::user_scroll_direction(self, app)
    }

    fn allow_implicit_scrolling(self: Handle<Self>, app: &App) -> bool {
        ScrollPositionBase::allow_implicit_scrolling(self, app)
    }
}

impl ScrollPosition for PagePosition {
    crate::scroll_position_with_single_context_overrides!();

    fn copy_with(self: Handle<Self>, app: &App) -> Rc<dyn ScrollMetrics> {
        let has_content_dimensions = self.has_content_dimensions(app);
        let mut metrics = PageMetrics::new(
            has_content_dimensions.then(|| self.min_scroll_extent(app)),
            has_content_dimensions.then(|| self.max_scroll_extent(app)),
            self.has_pixels(app).then(|| self.pixels(app)),
            self.has_viewport_dimension(app)
                .then(|| self.viewport_dimension(app)),
            ScrollPosition::axis_direction(self, app),
            self.viewport_fraction(app),
            self.device_pixel_ratio(app),
        );
        metrics.position = Some(PagePositionSnapshot {
            cached_page: app.get(self).cached_page,
        });
        Rc::new(metrics)
    }

    fn ensure_visible(
        self: Handle<Self>,
        app: &mut App,
        object: AnyRenderObject,
        alignment: f64,
        duration: Duration,
        curve: Rc<dyn Curve>,
        alignment_policy: ScrollPositionAlignmentPolicy,
        _target_render_object: Option<AnyRenderObject>,
    ) -> CompleterFuture<()> {
        // Since the _PagePosition is intended to cover the available space within
        // its viewport, stop trying to move the target render object to the center
        // - otherwise, could end up changing which page is visible and moving the
        // targetRenderObject out of the viewport.
        ScrollPositionBase::ensure_visible(
            self,
            app,
            object,
            alignment,
            duration,
            curve,
            alignment_policy,
            None,
        )
    }

    fn save_scroll_offset(self: Handle<Self>, app: &mut App) {
        let storage_context = self.context(app).storage_context(app);
        if let Some(bucket) = PageStorage::maybe_of(app, storage_context) {
            let page = self.page_to_save(app);
            bucket.write_state(app, storage_context, Rc::new(page), None);
        }
    }

    fn restore_scroll_offset(self: Handle<Self>, app: &mut App) {
        if !self.has_pixels(app) {
            let storage_context = self.context(app).storage_context(app);
            let value = PageStorage::maybe_of(app, storage_context)
                .and_then(|bucket| bucket.read_state(app, storage_context, None))
                .and_then(|data| data.downcast_ref::<f64>().copied());
            if let Some(value) = value {
                app.get_mut(self).page_to_use_on_startup = value;
            }
        }
    }

    fn save_offset(self: Handle<Self>, app: &mut App) {
        let page = self.page_to_save(app);
        self.context(app).save_offset(app, page);
    }

    fn restore_offset(self: Handle<Self>, app: &mut App, offset: f64, initial_restore: bool) {
        if initial_restore {
            app.get_mut(self).page_to_use_on_startup = offset;
        } else {
            let pixels = self.get_pixels_from_page(app, offset);
            ViewportOffset::jump_to(self, app, pixels);
        }
    }
}

impl ScrollActivityDelegate for PagePosition {
    crate::scroll_position_with_single_context_delegate_overrides!();
}

impl ScrollPositionWithSingleContextLeaf for PagePosition {
    crate::scroll_position_with_single_context_accessors!();

    fn absorb(self: Handle<Self>, app: &mut App, other: AnyScrollPosition) {
        ScrollPositionWithSingleContext::absorb(self, app, other);
        debug_assert!(app.get(self).cached_page.is_none());

        let Some(other) = other.downcast::<PagePosition>(app) else {
            return;
        };

        if let Some(cached_page) = app.get(other).cached_page {
            app.get_mut(self).cached_page = Some(cached_page);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// _ForceImplicitScrollPhysics

/// Dart's `_ForceImplicitScrollPhysics`: answers
/// [`allow_implicit_scrolling`](ScrollPhysics::allow_implicit_scrolling) with its own value.
struct ForceImplicitScrollPhysics {
    allow_implicit_scrolling: bool,
    parent: Option<ScrollPhysicsRef>,
}

impl ForceImplicitScrollPhysics {
    fn new(allow_implicit_scrolling: bool) -> ForceImplicitScrollPhysics {
        ForceImplicitScrollPhysics {
            allow_implicit_scrolling,
            parent: None,
        }
    }
}

impl ScrollPhysics for ForceImplicitScrollPhysics {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn parent(&self) -> Option<&ScrollPhysicsRef> {
        self.parent.as_ref()
    }

    fn apply_to(&self, ancestor: Option<ScrollPhysicsRef>) -> ScrollPhysicsRef {
        Rc::new(ForceImplicitScrollPhysics {
            allow_implicit_scrolling: self.allow_implicit_scrolling,
            parent: ScrollPhysicsBase::build_parent(self, ancestor),
        })
    }

    fn allow_implicit_scrolling(&self) -> bool {
        self.allow_implicit_scrolling
    }
}

impl Debug for ForceImplicitScrollPhysics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.parent {
            None => write!(f, "_ForceImplicitScrollPhysics"),
            Some(parent) => write!(f, "_ForceImplicitScrollPhysics -> {parent:?}"),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// PageScrollPhysics

/// Scroll physics used by a [`PageView`].
///
/// These physics cause the page view to snap to page boundaries.
///
/// See also:
///
///  * [`ScrollPhysics`], the base trait which defines the API for scrolling physics.
///  * [`PageView::physics`], which can override the physics used by a page view.
#[derive(Default)]
pub struct PageScrollPhysics {
    /// See [`ScrollPhysics::parent`].
    pub parent: Option<ScrollPhysicsRef>,
}

impl PageScrollPhysics {
    /// Creates physics for a [`PageView`].
    pub fn new() -> PageScrollPhysics {
        PageScrollPhysics::default()
    }

    /// Dart `PageScrollPhysics(parent:)`.
    pub fn with_parent(mut self, parent: ScrollPhysicsRef) -> PageScrollPhysics {
        self.parent = Some(parent);
        self
    }

    // The physics are handed a page view's position as the snapshot its `copy_with` makes;
    // `page_position` is Dart's `position is _PagePosition` on it.
    fn get_page(&self, position: &dyn ScrollMetrics) -> f64 {
        if let Some((metrics, snapshot)) = page_position(position) {
            return metrics
                .position_page(snapshot)
                .expect("a page position with dimensions has a page");
        }
        position.pixels() / position.viewport_dimension()
    }

    fn get_pixels(&self, position: &dyn ScrollMetrics, page: f64) -> f64 {
        if let Some((metrics, _)) = page_position(position) {
            return metrics.position_pixels_from_page(page);
        }
        page * position.viewport_dimension()
    }

    fn get_target_pixels(
        &self,
        position: &dyn ScrollMetrics,
        tolerance: Tolerance,
        velocity: f64,
    ) -> f64 {
        let mut page = self.get_page(position);
        if velocity < -tolerance.velocity {
            page -= 0.5;
        } else if velocity > tolerance.velocity {
            page += 0.5;
        }
        self.get_pixels(position, page.round())
    }
}

/// The page metrics and the position's snapshot when `position` is a page view's position.
fn page_position(position: &dyn ScrollMetrics) -> Option<(&PageMetrics, PagePositionSnapshot)> {
    let metrics = position.as_any().downcast_ref::<PageMetrics>()?;
    metrics.position.map(|snapshot| (metrics, snapshot))
}

impl ScrollPhysics for PageScrollPhysics {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn parent(&self) -> Option<&ScrollPhysicsRef> {
        self.parent.as_ref()
    }

    fn apply_to(&self, ancestor: Option<ScrollPhysicsRef>) -> ScrollPhysicsRef {
        Rc::new(PageScrollPhysics {
            parent: ScrollPhysicsBase::build_parent(self, ancestor),
        })
    }

    fn create_ballistic_simulation(
        &self,
        position: &dyn ScrollMetrics,
        velocity: f64,
    ) -> Option<Box<dyn Simulation>> {
        // If we're out of range and not headed back in range, defer to the parent
        // ballistics, which should put us back in range at a page boundary.
        if (velocity <= 0.0 && position.pixels() <= position.min_scroll_extent())
            || (velocity >= 0.0 && position.pixels() >= position.max_scroll_extent())
        {
            return ScrollPhysicsBase::create_ballistic_simulation(self, position, velocity);
        }
        let tolerance = self.tolerance_for(position);
        let target = self.get_target_pixels(position, tolerance, velocity);
        if target != position.pixels() {
            return Some(Box::new(ScrollSpringSimulation::new(
                self.spring(),
                position.pixels(),
                target,
                velocity,
                tolerance,
            )));
        }
        None
    }

    fn allow_implicit_scrolling(&self) -> bool {
        false
    }

    fn allow_selection_edge_scrolling(&self) -> bool {
        false
    }
}

impl Debug for PageScrollPhysics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.parent {
            None => write!(f, "PageScrollPhysics"),
            Some(parent) => write!(f, "PageScrollPhysics -> {parent:?}"),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// PageView

/// A scrollable list that works page by page.
///
/// Each child of a page view is forced to be the same size as the viewport.
///
/// You can use a [`PageController`] to control which page is visible in the view. In addition
/// to being able to control the pixel offset of the content inside the [`PageView`], a
/// [`PageController`] also lets you control the offset in terms of pages, which are increments
/// of the viewport size.
///
/// The [`PageController`] can also be used to control the
/// [`PageController::initial_page`], which determines which page is shown when the
/// [`PageView`] is first constructed, and the [`PageController::viewport_fraction`], which
/// determines the size of the pages as a fraction of the viewport size.
///
/// ## Persisting the scroll position during a session
///
/// Scroll views attempt to persist their scroll position using [`PageStorage`]. For a
/// [`PageView`], this can be disabled by setting [`PageController::keep_page`] to false on the
/// [`controller`](Self::controller). If it is enabled, using a `PageStorageKey` for the
/// [`key`](Self::key) of this widget is recommended to help disambiguate different scroll views
/// from each other.
///
/// See also:
///
///  * [`PageController`], which controls which page is visible in the view.
///  * `SingleChildScrollView`, when you need to make a single child scrollable.
///  * `ListView`, for a scrollable list of boxes.
///  * `GridView`, for a scrollable grid of boxes.
///  * [`ScrollNotification`] and [`NotificationListener`], which can be used to watch the
///    scroll position without using a `ScrollController`.
pub struct PageView {
    pub key: Option<KeyRef>,

    /// Controls whether the widget's pages will respond to `RenderObject.showOnScreen`, which
    /// will allow for implicit accessibility scrolling.
    ///
    /// With this flag set to false, when accessibility focus reaches the end of the current
    /// page and the user attempts to move it to the next element, the focus will traverse to
    /// the next widget outside of the page view.
    ///
    /// With this flag set to true, when accessibility focus reaches the end of the current page
    /// and user attempts to move it to the next element, focus will traverse to the next page
    /// in the page view.
    pub allow_implicit_scrolling: bool,

    /// The amount of content to lay out around the visible pages; `None` until given.
    ///
    /// In [`PageView`], the default uses [`ScrollCacheExtent::Viewport`], where the value
    /// represents the number of viewport lengths to cache beyond the visible area.
    ///
    /// When [`PageController::viewport_fraction`] is 1.0 (the default), this is equivalent to
    /// the number of pages. For example, `ScrollCacheExtent::Viewport(2.0)` caches 2 pages
    /// before and after the visible page.
    ///
    /// When [`PageController::viewport_fraction`] is less than 1.0, multiple pages may be
    /// visible in a single viewport, so `ScrollCacheExtent::Viewport(1.0)` may cache more than
    /// one additional page in each direction.
    ///
    /// [`ScrollCacheExtent::Pixels`] can also be used to specify the cache extent in logical
    /// pixels instead of viewport sizes.
    ///
    /// If it is given, its value must be consistent with
    /// [`allow_implicit_scrolling`](Self::allow_implicit_scrolling): the value must be greater
    /// than 0.0 when [`allow_implicit_scrolling`](Self::allow_implicit_scrolling) is true, and
    /// must be 0.0 when it is false.
    ///
    /// Defaults to `ScrollCacheExtent::Viewport(1.0)` if
    /// [`allow_implicit_scrolling`](Self::allow_implicit_scrolling) is true, and
    /// `ScrollCacheExtent::Viewport(0.0)` if it is false; read it through
    /// [`effective_scroll_cache_extent`](Self::effective_scroll_cache_extent).
    pub scroll_cache_extent: Option<ScrollCacheExtent>,

    /// Restoration ID to save and restore the scroll offset of the scrollable.
    pub restoration_id: Option<String>,

    /// The [`Axis`] along which the scroll view's offset increases with each page.
    ///
    /// For the direction in which active scrolling may be occurring, see
    /// `ScrollDirection`.
    ///
    /// Defaults to [`Axis::Horizontal`].
    pub scroll_direction: Axis,

    /// Whether the page view scrolls in the reading direction.
    ///
    /// For example, if the reading direction is left-to-right and
    /// [`scroll_direction`](Self::scroll_direction) is [`Axis::Horizontal`], then the page view
    /// scrolls from left to right when [`reverse`](Self::reverse) is false and from right to
    /// left when [`reverse`](Self::reverse) is true.
    ///
    /// Similarly, if [`scroll_direction`](Self::scroll_direction) is [`Axis::Vertical`], then
    /// the page view scrolls from top to bottom when [`reverse`](Self::reverse) is false and
    /// from bottom to top when [`reverse`](Self::reverse) is true.
    ///
    /// Defaults to false.
    pub reverse: bool,

    /// An object that can be used to control the position to which this page view is scrolled.
    pub controller: Option<Ref<PageController>>,

    /// How the page view should respond to user input.
    ///
    /// For example, determines how the page view continues to animate after the user stops
    /// dragging the page view.
    ///
    /// The physics are modified to snap to page boundaries using [`PageScrollPhysics`] prior
    /// to being used.
    ///
    /// If an explicit [`ScrollBehaviorRef`] is provided to
    /// [`scroll_behavior`](Self::scroll_behavior), the [`ScrollPhysics`] provided by that
    /// behavior will take precedence after [`physics`](Self::physics).
    ///
    /// Defaults to matching platform conventions.
    pub physics: Option<ScrollPhysicsRef>,

    /// Set to false to disable page snapping, useful for custom scroll behavior.
    ///
    /// If the [`pad_ends`](Self::pad_ends) is false and [`PageController::viewport_fraction`] <
    /// 1.0, the page will snap to the beginning of the viewport; otherwise, the page will snap
    /// to the center of the viewport.
    pub page_snapping: bool,

    /// Called whenever the page in the center of the viewport changes.
    pub on_page_changed: Option<ValueChanged<i32>>,

    /// A delegate that provides the children for the [`PageView`].
    ///
    /// The [`PageView::custom`] constructor lets you specify this delegate explicitly. The
    /// [`PageView::new`] and [`PageView::builder`] constructors create a
    /// [`children_delegate`](Self::children_delegate) that wraps the given list and builder,
    /// respectively.
    pub children_delegate: SliverChildDelegateRef,

    /// Determines the way that drag start behavior is handled.
    pub drag_start_behavior: DragStartBehavior,

    /// The content will be clipped (or not) according to this option.
    ///
    /// Defaults to [`Clip::HardEdge`].
    pub clip_behavior: Clip,

    /// Defines the behavior of gesture detector used in this [`Scrollable`].
    ///
    /// Defaults to [`HitTestBehavior::Opaque`].
    pub hit_test_behavior: HitTestBehavior,

    /// A [`ScrollBehaviorRef`] that will be applied to this widget individually.
    ///
    /// The scroll behavior of the inherited [`ScrollConfiguration`] will be modified by default
    /// to not apply a scrollbar.
    pub scroll_behavior: Option<ScrollBehaviorRef>,

    /// Whether to add padding to both ends of the list.
    ///
    /// If this is set to true and [`PageController::viewport_fraction`] < 1.0, padding will be
    /// added such that the first and last child slivers will be in the center of the viewport
    /// when scrolled all the way to the start or end, respectively.
    ///
    /// If [`PageController::viewport_fraction`] >= 1.0, this property has no effect.
    ///
    /// This property defaults to true.
    pub pad_ends: bool,
}

impl Default for PageView {
    fn default() -> PageView {
        PageView::custom(SliverChildListDelegate::new(Vec::new()).into_delegate())
    }
}

impl PageView {
    /// Creates a scrollable list that works page by page from an explicit list of widgets,
    /// which the [`children`](Self::children) setter supplies.
    ///
    /// This constructor is appropriate for page views with a small number of children because
    /// constructing the list requires doing work for every child that could possibly be
    /// displayed in the page view, instead of just those children that are actually visible.
    pub fn new() -> PageView {
        PageView::default()
    }

    /// Creates a scrollable list that works page by page using widgets that are created on
    /// demand.
    ///
    /// This constructor is appropriate for page views with a large (or infinite) number of
    /// children because the builder is called only for those children that are actually
    /// visible.
    ///
    /// Providing an [`item_count`](PageViewBuilder::item_count) lets the [`PageView`] compute
    /// the maximum scroll extent.
    ///
    /// The item builder will be called only with indices greater than or equal to zero and less
    /// than the item count.
    pub fn builder(
        item_builder: impl Fn(&mut App, BuildContext, i32) -> Option<WidgetRef> + 'static,
    ) -> PageViewBuilder {
        PageViewBuilder::new(Rc::new(item_builder))
    }

    /// Creates a scrollable list that works page by page with a custom child model.
    pub fn custom(children_delegate: SliverChildDelegateRef) -> PageView {
        PageView {
            key: None,
            allow_implicit_scrolling: false,
            scroll_cache_extent: None,
            restoration_id: None,
            scroll_direction: Axis::Horizontal,
            reverse: false,
            controller: None,
            physics: None,
            page_snapping: true,
            on_page_changed: None,
            children_delegate,
            drag_start_behavior: DragStartBehavior::Start,
            clip_behavior: Clip::HardEdge,
            hit_test_behavior: HitTestBehavior::Opaque,
            scroll_behavior: None,
            pad_ends: true,
        }
    }

    /// Dart `PageView(children:)`, which wraps the list in a `SliverChildListDelegate`.
    pub fn children(mut self, children: impl IntoIterator<Item = WidgetRef>) -> PageView {
        self.children_delegate =
            SliverChildListDelegate::new(children.into_iter().collect()).into_delegate();
        self
    }

    /// Dart `PageView(key:)`.
    pub fn key(mut self, key: KeyRef) -> PageView {
        self.key = Some(key);
        self
    }

    /// Dart `PageView(scrollDirection:)`.
    pub fn scroll_direction(mut self, scroll_direction: Axis) -> PageView {
        self.scroll_direction = scroll_direction;
        self
    }

    /// Dart `PageView(reverse:)`.
    pub fn reverse(mut self, reverse: bool) -> PageView {
        self.reverse = reverse;
        self
    }

    /// Dart `PageView(controller:)`.
    pub fn controller(mut self, controller: Ref<PageController>) -> PageView {
        self.controller = Some(controller);
        self
    }

    /// Dart `PageView(physics:)`.
    pub fn physics(mut self, physics: ScrollPhysicsRef) -> PageView {
        self.physics = Some(physics);
        self
    }

    /// Dart `PageView(pageSnapping:)`.
    pub fn page_snapping(mut self, page_snapping: bool) -> PageView {
        self.page_snapping = page_snapping;
        self
    }

    /// Dart `PageView(onPageChanged:)`.
    pub fn on_page_changed(
        mut self,
        on_page_changed: impl Fn(&mut App, i32) + 'static,
    ) -> PageView {
        self.on_page_changed = Some(Rc::new(on_page_changed));
        self
    }

    /// Dart `PageView(dragStartBehavior:)`.
    pub fn drag_start_behavior(mut self, drag_start_behavior: DragStartBehavior) -> PageView {
        self.drag_start_behavior = drag_start_behavior;
        self
    }

    /// Dart `PageView(allowImplicitScrolling:)`.
    pub fn allow_implicit_scrolling(mut self, allow_implicit_scrolling: bool) -> PageView {
        self.allow_implicit_scrolling = allow_implicit_scrolling;
        self
    }

    /// Dart `PageView(scrollCacheExtent:)`.
    pub fn scroll_cache_extent(mut self, scroll_cache_extent: ScrollCacheExtent) -> PageView {
        self.scroll_cache_extent = Some(scroll_cache_extent);
        self
    }

    /// Dart `PageView(restorationId:)`.
    pub fn restoration_id(mut self, restoration_id: impl Into<String>) -> PageView {
        self.restoration_id = Some(restoration_id.into());
        self
    }

    /// Dart `PageView(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> PageView {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `PageView(hitTestBehavior:)`.
    pub fn hit_test_behavior(mut self, hit_test_behavior: HitTestBehavior) -> PageView {
        self.hit_test_behavior = hit_test_behavior;
        self
    }

    /// Dart `PageView(scrollBehavior:)`.
    pub fn scroll_behavior(mut self, scroll_behavior: ScrollBehaviorRef) -> PageView {
        self.scroll_behavior = Some(scroll_behavior);
        self
    }

    /// Dart `PageView(padEnds:)`.
    pub fn pad_ends(mut self, pad_ends: bool) -> PageView {
        self.pad_ends = pad_ends;
        self
    }

    /// Dart's `scrollCacheExtent` field, as its constructor fills it.
    pub fn effective_scroll_cache_extent(&self) -> ScrollCacheExtent {
        self.scroll_cache_extent
            .unwrap_or(ScrollCacheExtent::Viewport(
                if self.allow_implicit_scrolling {
                    1.0
                } else {
                    0.0
                },
            ))
    }
}

impl Debug for PageView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PageView")
            .field("scrollDirection", &self.scroll_direction)
            .field("reverse", &self.reverse)
            .field("controller", &self.controller)
            .field("physics", &self.physics)
            .field("pageSnapping", &self.page_snapping)
            .field("allowImplicitScrolling", &self.allow_implicit_scrolling)
            .finish()
    }
}

impl StatefulWidget for PageView {
    type State = PageViewState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(
            self.scroll_cache_extent
                .is_none_or(|extent| (extent.value() > 0.0) == self.allow_implicit_scrolling),
            "scrollCacheExtent and allowImplicitScrolling must be consistent: scrollCacheExtent \
             must be greater than 0.0 when allowImplicitScrolling is true, and must be 0.0 when \
             allowImplicitScrolling is false."
        );
    }

    fn create_state(&self) -> PageViewState {
        PageViewState {
            state: StateData::new(),
            last_reported_page: 0,
            controller: None,
        }
    }
}

/// Dart's `PageView.builder(..)` arguments that go to its delegate; the value converts to the
/// [`PageView`].
pub struct PageViewBuilder {
    delegate: SliverChildBuilderDelegate,
}

impl PageViewBuilder {
    fn new(item_builder: NullableIndexedWidgetBuilder) -> PageViewBuilder {
        PageViewBuilder {
            delegate: SliverChildBuilderDelegate::from_builder(item_builder),
        }
    }

    /// Dart `itemCount:`.
    pub fn item_count(mut self, item_count: i32) -> PageViewBuilder {
        self.delegate = self.delegate.child_count(item_count);
        self
    }

    /// Dart `findChildIndexCallback:`.
    ///
    /// It corresponds to `SliverChildBuilderDelegate::find_child_index_callback`. If `None`, a
    /// child widget may not map to its existing render object when the order of children
    /// returned from the children builder changes. This may result in state-loss. This
    /// callback needs to be implemented if the order of the children may change at a later
    /// time.
    pub fn find_child_index_callback(mut self, value: ChildIndexGetter) -> PageViewBuilder {
        self.delegate = self.delegate.find_child_index_callback(value);
        self
    }

    /// The [`PageView`] this builder describes, whose other arguments its setters take.
    pub fn build(self) -> PageView {
        PageView::custom(self.delegate.into_delegate())
    }
}

impl IntoWidget<crate::framework::StatefulKind> for PageViewBuilder {
    fn into_widget(self) -> WidgetRef {
        self.build().into_widget()
    }
}

/// The state of a [`PageView`]; Dart's `_PageViewState`.
pub struct PageViewState {
    state: StateData<PageView>,
    last_reported_page: i32,
    controller: Option<Ref<PageController>>,
}

impl PageViewState {
    fn controller(self: Handle<Self>, app: &App) -> Handle<PageController> {
        app.get(self)
            .controller
            .handle()
            .expect("the controller is set in init_state")
    }

    fn init_controller(self: Handle<Self>, app: &mut App) {
        let controller = match self.widget(app).controller.clone() {
            Some(controller) => controller,
            None => PageController::default(app),
        };
        app.get_mut(self).controller = Some(controller);
    }

    fn get_direction(self: Handle<Self>, app: &mut App, context: BuildContext) -> AxisDirection {
        let widget = self.widget(app);
        let reverse = widget.reverse;
        match widget.scroll_direction {
            Axis::Horizontal => {
                let text_direction = Directionality::of(app, context);
                let axis_direction = text_direction_to_axis_direction(text_direction);
                if reverse {
                    flip_axis_direction(axis_direction)
                } else {
                    axis_direction
                }
            }
            Axis::Vertical => {
                if reverse {
                    AxisDirection::Up
                } else {
                    AxisDirection::Down
                }
            }
        }
    }

    /// The `onNotification` of the listener `build` puts above the scrollable.
    fn handle_scroll_notification(
        self: Handle<Self>,
        app: &mut App,
        notification: &dyn ScrollNotification,
    ) -> bool {
        let on_page_changed = self.widget(app).on_page_changed.clone();
        if notification.depth() == 0
            && let Some(on_page_changed) = on_page_changed
            && notification.as_any().is::<ScrollUpdateNotification>()
        {
            let metrics = notification
                .metrics()
                .as_any()
                .downcast_ref::<PageMetrics>()
                .expect("a page view's position reports page metrics");
            let current_page = metrics.page().expect("page metrics have a page").round() as i32;
            if current_page != app.get(self).last_reported_page {
                app.get_mut(self).last_reported_page = current_page;
                on_page_changed(app, current_page);
            }
        }
        false
    }
}

impl State for PageViewState {
    type Widget = PageView;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.init_controller(app);
        let initial_page = self.controller(app).initial_page(app);
        app.get_mut(self).last_reported_page = initial_page;
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if self.widget(app).controller.is_none() {
            ScrollControllerLeaf::dispose(self.controller(app), app);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &PageView) {
        if old_widget.controller != self.widget(app).controller {
            if old_widget.controller.is_none() {
                ScrollControllerLeaf::dispose(self.controller(app), app);
            }
            self.init_controller(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let axis_direction = self.get_direction(app, context);
        let widget = self.widget(app);
        let allow_implicit_scrolling = widget.allow_implicit_scrolling;
        let page_snapping = widget.page_snapping;
        let widget_physics = widget.physics.clone();
        let widget_scroll_behavior = widget.scroll_behavior.clone();
        let base_physics = widget_physics.or_else(|| {
            widget_scroll_behavior
                .as_ref()
                .map(|behavior| behavior.get_scroll_physics(app, context))
        });
        let physics =
            ForceImplicitScrollPhysics::new(allow_implicit_scrolling).apply_to(if page_snapping {
                Some(PageScrollPhysics::new().apply_to(base_physics))
            } else {
                base_physics
            });

        let widget = self.widget(app);
        let drag_start_behavior = widget.drag_start_behavior;
        let restoration_id = widget.restoration_id.clone();
        let hit_test_behavior = widget.hit_test_behavior;
        let scroll_cache_extent = widget.effective_scroll_cache_extent();
        let clip_behavior = widget.clip_behavior;
        let children_delegate = Rc::clone(&widget.children_delegate);
        let pad_ends = widget.pad_ends;
        let scroll_behavior = match widget_scroll_behavior {
            Some(scroll_behavior) => scroll_behavior,
            None => ScrollBehaviorRef::new(
                ScrollConfiguration::of(app, context)
                    .copy_with()
                    .scrollbars(false),
            ),
        };
        let controller = app
            .get(self)
            .controller
            .clone()
            .expect("the controller is set in init_state");
        let viewport_fraction = controller.viewport_fraction(app);

        let viewport_builder: ViewportBuilder =
            Rc::new(move |_app, _context, position: ViewportOffsetRef| {
                Viewport::new(position)
                    .scroll_cache_extent(scroll_cache_extent)
                    .axis_direction(axis_direction)
                    .clip_behavior(clip_behavior)
                    .slivers(vec![
                        SliverFillViewport::new(Rc::clone(&children_delegate))
                            .viewport_fraction(viewport_fraction)
                            .pad_ends(pad_ends)
                            .allow_implicit_scrolling(allow_implicit_scrolling)
                            .into_widget(),
                    ])
                    .into_widget()
            });
        let mut scrollable = Scrollable::new(viewport_builder)
            .drag_start_behavior(drag_start_behavior)
            .axis_direction(axis_direction)
            .controller(controller.into_erased())
            .physics(physics)
            .hit_test_behavior(hit_test_behavior)
            .scroll_behavior(scroll_behavior);
        if let Some(restoration_id) = restoration_id {
            scrollable = scrollable.restoration_id(restoration_id);
        }

        NotificationListener::<dyn ScrollNotification>::new(scrollable)
            .on_notification(
                move |app: &mut App, notification: &dyn ScrollNotification| {
                    self.handle_scroll_notification(app, notification)
                },
            )
            .into_widget()
    }
}

/// The part of `flutter_test`'s `WidgetTester` the page view and sliver fill tests use: an
/// 800 x 600 view at pixel ratio 3, a fake clock that a pump advances, and pointers sent straight
/// to the gesture binding.
#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::rc::Rc;
    use std::time::Duration;

    use waft_animation::Curves;
    use waft_embedder::{Offset, Size, TextDirection};
    use waft_rendering::{RenderHandle, RenderSliver, RenderSliverFillViewport};

    use waft_embedder::{Color, TargetPlatform};
    use waft_foundation::{Listener, ValueKey};
    use waft_rendering::{RenderBox, RenderViewport, RenderViewportBase};

    use super::*;
    use crate::testing::{
        Finder, TargetPlatformVariant, TestPointer, WidgetTester, assert_more_or_less_equals,
        expect, find, finds_nothing, finds_one_widget, test_widgets, test_widgets_variant,
    };
    use crate::widgets::basic::{Builder, Center, Column, Expanded, OverflowBox, SizedBox};
    use crate::widgets::container::Container;
    use crate::widgets::gesture_detector::GestureDetector;
    use crate::widgets::overlay::{Overlay, OverlayEntry};
    use crate::widgets::page_storage::{PageStorage, PageStorageBucket, PageStorageKey};
    use crate::widgets::scroll_physics::{BouncingScrollPhysics, ClampingScrollPhysics};
    use crate::widgets::text::Text;

    const K_BLUE: Color = Color::new(0xFF0000FF);
    const K_GREEN: Color = Color::new(0xFF00FF00);

    /// `states.dart`'s `kStates`.
    const K_STATES: [&str; 50] = [
        "Alabama",
        "Alaska",
        "Arizona",
        "Arkansas",
        "California",
        "Colorado",
        "Connecticut",
        "Delaware",
        "Florida",
        "Georgia",
        "Hawaii",
        "Idaho",
        "Illinois",
        "Indiana",
        "Iowa",
        "Kansas",
        "Kentucky",
        "Louisiana",
        "Maine",
        "Maryland",
        "Massachusetts",
        "Michigan",
        "Minnesota",
        "Mississippi",
        "Missouri",
        "Montana",
        "Nebraska",
        "Nevada",
        "New Hampshire",
        "New Jersey",
        "New Mexico",
        "New York",
        "North Carolina",
        "North Dakota",
        "Ohio",
        "Oklahoma",
        "Oregon",
        "Pennsylvania",
        "Rhode Island",
        "South Carolina",
        "South Dakota",
        "Tennessee",
        "Texas",
        "Utah",
        "Vermont",
        "Virginia",
        "Washington",
        "West Virginia",
        "Wisconsin",
        "Wyoming",
    ];

    fn index_of(state: &str) -> i32 {
        K_STATES
            .iter()
            .position(|candidate| *candidate == state)
            .expect("a state") as i32
    }

    /// `kStates.map((state) => Text(state))`.
    fn states() -> Vec<WidgetRef> {
        K_STATES
            .iter()
            .map(|&state| Text::new(state).into_widget())
            .collect()
    }

    fn ltr<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Directionality::new(TextDirection::Ltr, child).into_widget()
    }

    fn centered<K>(width: f64, height: f64, child: impl IntoWidget<K>) -> WidgetRef {
        ltr(Center::new().child(SizedBox::new().width(width).height(height).child(child)))
    }

    fn fill_viewport(tester: &WidgetTester) -> RenderHandle<RenderSliverFillViewport> {
        *tester
            .all::<RenderSliverFillViewport>()
            .first()
            .expect("a mounted page view")
    }

    fn page(tester: &WidgetTester, controller: Handle<PageController>) -> Option<f64> {
        controller.page(&tester.cell.borrow())
    }

    fn new_controller(
        tester: &WidgetTester,
        initial_page: i32,
        viewport_fraction: f64,
    ) -> Ref<PageController> {
        PageController::new(
            &mut tester.cell.borrow_mut(),
            initial_page,
            true,
            viewport_fraction,
            None,
            None,
        )
    }

    #[test]
    #[should_panic(expected = "PageController.page cannot be accessed before a PageView is built")]
    fn a_page_controller_cannot_return_its_page_while_unattached() {
        test_widgets(|tester| {
            let controller = PageController::default(&mut tester.cell.borrow_mut());
            page(tester, controller.handle());
        });
    }

    #[test]
    fn a_page_view_drags_and_flings_from_page_to_page() {
        test_widgets(|tester| {
            let log: Rc<RefCell<Vec<&str>>> = Rc::new(RefCell::new(Vec::new()));
            tester.pump_widget(ltr(PageView::new()
                .drag_start_behavior(DragStartBehavior::Down)
                .children(K_STATES.iter().map(|&state| {
                    let log = Rc::clone(&log);
                    GestureDetector::new()
                        .drag_start_behavior(DragStartBehavior::Down)
                        .on_tap(Listener::new(move |_| log.borrow_mut().push(state)))
                        .child(
                            Container::new()
                                .height(200.0)
                                .color(K_BLUE)
                                .child(Text::new(state)),
                        )
                        .into_widget()
                }))));

            tester.tap(find::text("Alabama"));
            assert_eq!(*log.borrow(), ["Alabama"]);
            log.borrow_mut().clear();

            expect(tester, &find::text("Alaska"), finds_nothing());

            tester.drag(find::by_type::<PageView>(), Offset::new(-20.0, 0.0));
            tester.pump();

            expect(tester, &find::text("Alabama"), finds_one_widget());
            expect(tester, &find::text("Alaska"), finds_one_widget());
            expect(tester, &find::text("Arizona"), finds_nothing());

            tester.pump_and_settle();

            expect(tester, &find::text("Alabama"), finds_one_widget());
            expect(tester, &find::text("Alaska"), finds_nothing());

            tester.drag(find::by_type::<PageView>(), Offset::new(-401.0, 0.0));
            tester.pump_and_settle();

            expect(tester, &find::text("Alabama"), finds_nothing());
            expect(tester, &find::text("Alaska"), finds_one_widget());
            expect(tester, &find::text("Arizona"), finds_nothing());

            tester.tap(find::text("Alaska"));
            assert_eq!(*log.borrow(), ["Alaska"]);
            log.borrow_mut().clear();

            tester.fling(
                find::by_type::<PageView>(),
                Offset::new(-200.0, 0.0),
                1000.0,
            );
            tester.pump_and_settle();

            expect(tester, &find::text("Alabama"), finds_nothing());
            expect(tester, &find::text("Alaska"), finds_nothing());
            expect(tester, &find::text("Arizona"), finds_one_widget());

            tester.fling(find::by_type::<PageView>(), Offset::new(200.0, 0.0), 1000.0);
            tester.pump_and_settle();

            expect(tester, &find::text("Alabama"), finds_nothing());
            expect(tester, &find::text("Alaska"), finds_one_widget());
            expect(tester, &find::text("Arizona"), finds_nothing());
        });
    }

    #[test]
    fn a_page_controller_steps_to_the_next_and_previous_page_across_a_resize() {
        test_widgets(|tester| {
            let controller = new_controller(tester, 4, 1.0);
            tester.pump_widget(centered(
                600.0,
                400.0,
                PageView::new()
                    .controller(controller.clone())
                    .children(states()),
            ));

            expect(tester, &find::text("California"), finds_one_widget());

            drop(controller.next_page(
                &mut tester.cell.borrow_mut(),
                Duration::from_millis(150),
                Curves::ease(),
            ));
            tester.pump_and_settle();

            expect(tester, &find::text("Colorado"), finds_one_widget());

            tester.pump_widget(centered(
                300.0,
                400.0,
                PageView::new()
                    .controller(controller.clone())
                    .children(states()),
            ));

            expect(tester, &find::text("Colorado"), finds_one_widget());

            drop(controller.previous_page(
                &mut tester.cell.borrow_mut(),
                Duration::from_millis(150),
                Curves::ease(),
            ));
            tester.pump_and_settle();

            expect(tester, &find::text("California"), finds_one_widget());
        });
    }

    #[test]
    fn the_page_is_stable_across_resizes() {
        test_widgets(|tester| {
            tester.pump_widget(centered(600.0, 400.0, PageView::new().children(states())));

            expect(tester, &find::text("Alabama"), finds_one_widget());

            tester.drag(find::by_type::<PageView>(), Offset::new(-1250.0, 0.0));
            tester.pump_and_settle();

            expect(tester, &find::text("Arizona"), finds_one_widget());

            tester.pump_widget(centered(250.0, 100.0, PageView::new().children(states())));

            expect(tester, &find::text("Arizona"), finds_one_widget());

            tester.pump_widget(centered(450.0, 400.0, PageView::new().children(states())));

            expect(tester, &find::text("Arizona"), finds_one_widget());
        });
    }

    #[test]
    fn next_and_previous_page_resolve_when_their_animation_ends() {
        test_widgets(|tester| {
            let controller = PageController::default(&mut tester.cell.borrow_mut());
            tester.pump_widget(ltr(PageView::new()
                .controller(controller.clone())
                .children(states())));

            let next_page = controller.next_page(
                &mut tester.cell.borrow_mut(),
                Duration::from_millis(150),
                Curves::ease(),
            );

            assert!(!next_page.is_completed());
            tester.pump_by(Duration::from_millis(200));
            assert!(!next_page.is_completed());
            tester.pump_by(Duration::from_millis(200));
            assert!(next_page.is_completed());

            let previous_page = controller.previous_page(
                &mut tester.cell.borrow_mut(),
                Duration::from_millis(150),
                Curves::ease(),
            );

            assert!(!previous_page.is_completed());
            tester.pump_by(Duration::from_millis(200));
            assert!(!previous_page.is_completed());
            tester.pump_by(Duration::from_millis(200));
            assert!(previous_page.is_completed());
        });
    }

    #[test]
    fn a_page_view_in_a_zero_size_container_builds_its_first_page() {
        test_widgets(|tester| {
            tester
                .pump_widget(ltr(Center::new()
                    .child(SizedBox::shrink().child(PageView::new().children(states())))));

            expect(
                tester,
                &find::text("Alabama").skip_offstage(false),
                finds_one_widget(),
            );

            tester.pump_widget(centered(200.0, 200.0, PageView::new().children(states())));

            expect(tester, &find::text("Alabama"), finds_one_widget());
        });
    }

    fn sized_page_view(size: Size, controller: &Ref<PageController>) -> WidgetRef {
        ltr(Center::new().child(
            SizedBox::from_size(Some(size)).child(
                PageView::new()
                    .controller(controller.clone())
                    .on_page_changed(|_, _| {})
                    .children(states()),
            ),
        ))
    }

    #[test]
    fn a_resize_from_a_zero_size_viewport_keeps_the_page() {
        test_widgets(|tester| {
            let controller = new_controller(tester, 1, 1.0);

            // The pageView have a zero viewport, so nothing display.
            tester.pump_widget(sized_page_view(Size::ZERO, &controller));
            expect(tester, &find::text("Alabama"), finds_nothing());
            expect(
                tester,
                &find::text("Alabama").skip_offstage(false),
                finds_one_widget(),
            );

            // Resize from zero viewport to non-zero, the controller's initialPage 1 will display.
            tester.pump_widget(sized_page_view(Size::new(200.0, 200.0), &controller));
            expect(tester, &find::text("Alaska"), finds_one_widget());

            // Jump to page 'Iowa'.
            controller.jump_to_page(&mut tester.cell.borrow_mut(), index_of("Iowa"));
            tester.pump();
            expect(tester, &find::text("Iowa"), finds_one_widget());

            // Resize to zero viewport again, nothing display.
            tester.pump_widget(sized_page_view(Size::ZERO, &controller));
            expect(tester, &find::text("Iowa"), finds_nothing());

            // Resize from zero to non-zero, the pageView should not lose state, so the page 'Iowa'
            // show again.
            tester.pump_widget(sized_page_view(Size::new(200.0, 200.0), &controller));
            expect(tester, &find::text("Iowa"), finds_one_widget());
        });
    }

    #[test]
    fn the_controller_changes_the_page_of_a_zero_size_viewport() {
        test_widgets(|tester| {
            let controller = new_controller(tester, 1, 1.0);

            // The pageView have a zero viewport, so nothing display.
            tester.pump_widget(sized_page_view(Size::ZERO, &controller));
            assert_eq!(tester.get_size(find::by_type::<PageView>()), Size::ZERO);
            expect(tester, &find::text("Alabama"), finds_nothing());
            expect(
                tester,
                &find::text("Alabama").skip_offstage(false),
                finds_one_widget(),
            );

            // Change the page through the page controller when zero viewport
            drop(controller.animate_to_page(
                &mut tester.cell.borrow_mut(),
                index_of("Iowa"),
                Duration::from_millis(300),
                Curves::ease(),
            ));
            assert_eq!(
                page(tester, controller.handle()),
                Some(f64::from(index_of("Iowa")))
            );

            controller.jump_to_page(&mut tester.cell.borrow_mut(), index_of("Illinois"));
            assert_eq!(
                page(tester, controller.handle()),
                Some(f64::from(index_of("Illinois")))
            );

            // Resize from zero viewport to non-zero, the latest state should not lost.
            tester.pump_widget(sized_page_view(Size::new(200.0, 200.0), &controller));
            assert_eq!(
                page(tester, controller.handle()),
                Some(f64::from(index_of("Illinois")))
            );
            expect(tester, &find::text("Illinois"), finds_one_widget());
        });
    }

    fn logged_page_view(log: &Rc<RefCell<Vec<i32>>>, page_snapping: bool) -> WidgetRef {
        let log = Rc::clone(log);
        ltr(PageView::new()
            .page_snapping(page_snapping)
            .on_page_changed(move |_, page| log.borrow_mut().push(page))
            .children(states()))
    }

    #[test]
    fn the_page_changes_at_the_halfway_point() {
        test_widgets(|tester| {
            let log = Rc::new(RefCell::new(Vec::new()));
            tester.pump_widget(logged_page_view(&log, true));

            assert!(log.borrow().is_empty());

            let mut gesture = TestPointer::touch(1);
            tester.down(&mut gesture, Offset::new(100.0, 100.0));
            // The page view is 800.0 wide, so this move is just short of halfway.
            tester.move_by(&mut gesture, Offset::new(-380.0, 0.0), true);

            assert!(log.borrow().is_empty());

            // We've crossed the halfway mark.
            tester.move_by(&mut gesture, Offset::new(-40.0, 0.0), true);

            assert_eq!(*log.borrow(), vec![1]);
            log.borrow_mut().clear();

            // Moving a bit more should not generate redundant notifications.
            tester.move_by(&mut gesture, Offset::new(-40.0, 0.0), true);

            assert!(log.borrow().is_empty());

            for _ in 0..3 {
                tester.move_by(&mut gesture, Offset::new(-40.0, 0.0), true);
                tester.pump();
            }

            assert!(log.borrow().is_empty());

            tester.up(&gesture);
            tester.pump_and_settle();

            assert!(log.borrow().is_empty());

            expect(tester, &find::text("Alabama"), finds_nothing());
            expect(tester, &find::text("Alaska"), finds_one_widget());
        });
    }

    #[test]
    fn bouncing_ballistics_settle_back_on_the_first_page_without_overshooting() {
        test_widgets(|tester| {
            let log = Rc::new(RefCell::new(Vec::new()));
            let controller = new_controller(tester, 0, 0.9);
            let build = |size: Option<Size>| -> WidgetRef {
                let log = Rc::clone(&log);
                let page_view = ltr(PageView::new()
                    .controller(controller.clone())
                    .on_page_changed(move |_, page| log.borrow_mut().push(page))
                    .physics(Rc::new(BouncingScrollPhysics::new()))
                    .children(states()));
                match size {
                    Some(size) => OverflowBox::new()
                        .min_width(size.width())
                        .min_height(size.height())
                        .max_width(size.width())
                        .max_height(size.height())
                        .child(page_view)
                        .into_widget(),
                    None => page_view,
                }
            };

            tester.pump_widget(build(None));
            assert!(log.borrow().is_empty());

            // Fling right to move to a non-existent page at the beginning of the
            // PageView, and confirm that the PageView settles back on the first page.
            tester.fling(find::by_type::<PageView>(), Offset::new(100.0, 0.0), 800.0);
            tester.pump_and_settle();
            assert!(log.borrow().is_empty());

            expect(tester, &find::text("Alabama"), finds_one_widget());
            expect(tester, &find::text("Alaska"), finds_one_widget());
            expect(tester, &find::text("Arizona"), finds_nothing());

            // Try again with a Cupertino "Plus" device size.
            tester.pump_widget(build(Some(Size::new(414.0, 736.0))));
            assert!(log.borrow().is_empty());

            tester.fling(find::by_type::<PageView>(), Offset::new(100.0, 0.0), 800.0);
            tester.pump_and_settle();
            assert!(log.borrow().is_empty());

            expect(tester, &find::text("Alabama"), finds_one_widget());
            expect(tester, &find::text("Alaska"), finds_one_widget());
            expect(tester, &find::text("Arizona"), finds_nothing());
        });
    }

    /// The viewport fraction tests' `PageView.builder`: a page per label, a `Container` of
    /// height 200, blue or green by the index's parity, showing the label.
    fn built_page_view(controller: &Ref<PageController>, labels: Vec<String>) -> WidgetRef {
        let item_count = labels.len() as i32;
        ltr(PageView::builder(move |_, _, index| {
            Some(
                Container::new()
                    .height(200.0)
                    .color(if index % 2 == 0 { K_BLUE } else { K_GREEN })
                    .child(Text::new(labels[index as usize].clone()))
                    .into_widget(),
            )
        })
        .item_count(item_count)
        .build()
        .controller(controller.clone()))
    }

    /// `kStates`, as the viewport fraction tests label their pages.
    fn state_labels() -> Vec<String> {
        K_STATES.iter().map(|state| state.to_string()).collect()
    }

    /// `index.toString()` for `count` pages.
    fn index_labels(count: i32) -> Vec<String> {
        (0..count).map(|index| index.to_string()).collect()
    }

    #[test]
    fn pages_fill_their_viewport_fraction() {
        test_widgets(|tester| {
            let controller = new_controller(tester, 0, 7.0 / 8.0);
            tester.pump_widget(built_page_view(&controller, state_labels()));

            assert_eq!(
                tester.get_top_left(find::text("Alabama")),
                Offset::new(50.0, 0.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("Alaska")),
                Offset::new(750.0, 0.0)
            );

            controller.jump_to_page(&mut tester.cell.borrow_mut(), 10);
            tester.pump();

            assert_eq!(
                tester.get_top_left(find::text("Georgia")),
                Offset::new(-650.0, 0.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("Hawaii")),
                Offset::new(50.0, 0.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("Idaho")),
                Offset::new(750.0, 0.0)
            );

            let controller = new_controller(tester, 0, 39.0 / 40.0);
            tester.pump_widget(built_page_view(&controller, state_labels()));

            assert_eq!(
                tester.get_top_left(find::text("Georgia")),
                Offset::new(-770.0, 0.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("Hawaii")),
                Offset::new(10.0, 0.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("Idaho")),
                Offset::new(790.0, 0.0)
            );
        });
    }

    #[test]
    fn page_snapping_can_be_disabled_and_reenabled() {
        test_widgets(|tester| {
            let log = Rc::new(RefCell::new(Vec::new()));
            tester.pump_widget(logged_page_view(&log, true));
            assert!(log.borrow().is_empty());

            // Drag more than halfway to the next page, to confirm the default behavior.
            let mut gesture = TestPointer::touch(1);
            tester.down(&mut gesture, Offset::new(100.0, 100.0));
            // The page view is 800.0 wide, so this move is just beyond halfway.
            tester.move_by(&mut gesture, Offset::new(-420.0, 0.0), true);

            assert_eq!(*log.borrow(), vec![1]);
            log.borrow_mut().clear();

            // Release the gesture, confirm that the page settles on the next.
            tester.up(&gesture);
            tester.pump_and_settle();

            expect(tester, &find::text("Alabama"), finds_nothing());
            expect(tester, &find::text("Alaska"), finds_one_widget());

            // Disable page snapping, and try moving halfway. Confirm it doesn't snap.
            tester.pump_widget(logged_page_view(&log, false));
            let mut gesture = TestPointer::touch(1);
            tester.down(&mut gesture, Offset::new(100.0, 100.0));
            // Move just beyond halfway, again.
            tester.move_by(&mut gesture, Offset::new(-420.0, 0.0), true);

            // Page notifications still get sent.
            assert_eq!(*log.borrow(), vec![2]);
            log.borrow_mut().clear();

            // Release the gesture, confirm that both pages are visible.
            tester.up(&gesture);
            tester.pump_and_settle();

            expect(tester, &find::text("Alabama"), finds_nothing());
            expect(tester, &find::text("Alaska"), finds_one_widget());
            expect(tester, &find::text("Arizona"), finds_one_widget());
            expect(tester, &find::text("Arkansas"), finds_nothing());

            // Now re-enable snapping, confirm that we've settled on a page.
            tester.pump_widget(logged_page_view(&log, true));
            tester.pump_and_settle();

            assert!(log.borrow().is_empty());

            expect(tester, &find::text("Alaska"), finds_nothing());
            expect(tester, &find::text("Arizona"), finds_one_widget());
            expect(tester, &find::text("Arkansas"), finds_nothing());
        });
    }

    #[test]
    fn a_small_viewport_fraction_shows_several_pages() {
        test_widgets(|tester| {
            let controller = new_controller(tester, 0, 1.0 / 8.0);
            tester.pump_widget(built_page_view(&controller, state_labels()));

            assert_eq!(
                tester.get_top_left(find::text("Alabama")),
                Offset::new(350.0, 0.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("Alaska")),
                Offset::new(450.0, 0.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("Arizona")),
                Offset::new(550.0, 0.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("Arkansas")),
                Offset::new(650.0, 0.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("California")),
                Offset::new(750.0, 0.0)
            );

            controller.jump_to_page(&mut tester.cell.borrow_mut(), 10);
            tester.pump();

            assert_eq!(
                tester.get_top_left(find::text("Connecticut")),
                Offset::new(-50.0, 0.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("Delaware")),
                Offset::new(50.0, 0.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("Florida")),
                Offset::new(150.0, 0.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("Georgia")),
                Offset::new(250.0, 0.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("Hawaii")),
                Offset::new(350.0, 0.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("Idaho")),
                Offset::new(450.0, 0.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("Illinois")),
                Offset::new(550.0, 0.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("Indiana")),
                Offset::new(650.0, 0.0)
            );
            assert_eq!(
                tester.get_top_left(find::text("Iowa")),
                Offset::new(750.0, 0.0)
            );
        });
    }

    #[test]
    fn a_large_viewport_fraction_centers_pages_wider_than_the_viewport() {
        test_widgets(|tester| {
            let controller = new_controller(tester, 0, 5.0 / 4.0);
            tester.pump_widget(built_page_view(&controller, state_labels()));

            assert_eq!(
                tester.get_top_left(find::text("Alabama")),
                Offset::new(-100.0, 0.0)
            );
            assert_eq!(
                tester.get_bottom_right(find::text("Alabama")),
                Offset::new(900.0, 600.0)
            );

            controller.jump_to_page(&mut tester.cell.borrow_mut(), 10);
            tester.pump();

            assert_eq!(
                tester.get_top_left(find::text("Hawaii")),
                Offset::new(-100.0, 0.0)
            );
        });
    }

    #[test]
    fn a_new_controller_with_a_larger_viewport_fraction_centers_its_page() {
        test_widgets(|tester| {
            let old_controller = new_controller(tester, 0, 5.0 / 4.0);
            tester.pump_widget(built_page_view(&old_controller, state_labels()));

            assert_eq!(
                tester.get_top_left(find::text("Alabama")),
                Offset::new(-100.0, 0.0)
            );

            let new_controller = new_controller(tester, 0, 4.0);
            tester.pump_widget(built_page_view(&new_controller, state_labels()));
            new_controller.jump_to_page(&mut tester.cell.borrow_mut(), 10);
            tester.pump();

            assert_eq!(
                tester.get_top_left(find::text("Hawaii")),
                Offset::new(-(4.0 - 1.0) * 800.0 / 2.0, 0.0)
            );
        });
    }

    #[test]
    fn a_large_viewport_fraction_scrolls_to_the_last_page_and_snaps() {
        test_widgets(|tester| {
            let controller = new_controller(tester, 0, 5.0 / 4.0);
            tester.pump_widget(built_page_view(&controller, index_labels(3)));

            assert_eq!(
                tester.get_center(find::text("0")),
                Offset::new(400.0, 300.0)
            );

            controller.jump_to_page(&mut tester.cell.borrow_mut(), 2);
            tester.pump();
            tester.pump_and_settle();

            assert_eq!(
                tester.get_center(find::text("2")),
                Offset::new(400.0, 300.0)
            );
        });
    }

    /// Not in page_view_test.dart: Dart's `PageScrollPhysics` count a page view's pages in its
    /// own pages (`_PagePosition.page`, `getPixelsFromPage`), here through the snapshot its
    /// position's `copy_with` makes.
    #[test]
    fn a_large_viewport_fraction_settles_on_the_page_dragged_past_halfway() {
        test_widgets(|tester| {
            let controller = new_controller(tester, 0, 5.0 / 4.0);
            tester.pump_widget(built_page_view(&controller, index_labels(3)));

            // Pages are 1000 wide and start 100 in: 600 dragged is past halfway to page 1, whose
            // pixels are 1100, where whole viewports would have counted page 1 at 800.
            tester.drag(find::by_type::<PageView>(), Offset::new(-600.0, 0.0));
            tester.pump_and_settle();

            assert_eq!(page(tester, controller.handle()), Some(1.0));
            assert_eq!(
                tester.get_center(find::text("1")),
                Offset::new(400.0, 300.0)
            );
        });
    }

    #[test]
    fn the_current_item_remains_centered_on_constraint_change() {
        test_widgets(|tester| {
            let controller = new_controller(tester, K_STATES.len() as i32 - 1, 0.5);
            // Verifies that the last item is centered on screen.
            let verify_centered = |tester: &WidgetTester| {
                assert_more_or_less_equals(
                    tester.get_center(find::text(K_STATES[K_STATES.len() - 1])),
                    Offset::new(400.0, 300.0),
                );
            };

            tester.pump_widget(sized_page_view(Size::new(300.0, 300.0), &controller));
            tester.pump_and_settle();

            verify_centered(tester);

            tester.pump_widget(sized_page_view(Size::new(200.0, 300.0), &controller));
            tester.pump_and_settle();

            verify_centered(tester);
        });
    }

    #[test]
    fn a_jump_past_the_last_page_does_not_report_a_page_change() {
        test_widgets(|tester| {
            let controller = new_controller(tester, K_STATES.len() as i32 - 1, 1.0);
            let change_index = Rc::new(Cell::new(0));
            let changed = Rc::clone(&change_index);
            tester.pump_widget(ltr(PageView::new()
                .controller(controller.clone())
                .children(states())
                .on_page_changed(move |_, page| changed.set(page))));

            // try to move beyond max range
            controller.jump_to_page(&mut tester.cell.borrow_mut(), K_STATES.len() as i32 * 2);
            // change index should be zero, shouldn't fire onPageChanged
            assert_eq!(change_index.get(), 0);
            tester.pump();
            assert_eq!(change_index.get(), 0);
        });
    }

    fn stored_page_view(
        bucket: Ref<PageStorageBucket>,
        key: &'static str,
        controller: Ref<PageController>,
    ) -> WidgetRef {
        ltr(PageStorage::new(
            bucket,
            PageView::new()
                .key(PageStorageKey::new(key).into_key())
                .controller(controller)
                .children((0..3).map(|_| SizedBox::expand().into_widget())),
        ))
    }

    /// The message of the assert that reading `controller.page` fails.
    fn page_assert_message(tester: &WidgetTester, controller: Handle<PageController>) -> String {
        let error = catch_unwind(AssertUnwindSafe(|| page(tester, controller)))
            .expect_err("reading the page asserts");
        match error.downcast::<&str>() {
            Ok(message) => message.to_string(),
            Err(error) => *error.downcast::<String>().expect("a panic message"),
        }
    }

    const PAGE_BEFORE_BUILD: &str =
        "PageController.page cannot be accessed before a PageView is built with it.";

    #[test]
    fn a_page_view_restores_its_page_from_page_storage() {
        test_widgets(|tester| {
            let controller = PageController::default(&mut tester.cell.borrow_mut());
            assert_eq!(
                page_assert_message(tester, controller.handle()),
                PAGE_BEFORE_BUILD
            );
            let bucket = PageStorageBucket::new(&mut tester.cell.borrow_mut());
            tester.pump_widget(stored_page_view(
                bucket.clone(),
                "PageView",
                controller.clone(),
            ));
            assert_eq!(page(tester, controller.handle()), Some(0.0));
            controller.jump_to_page(&mut tester.cell.borrow_mut(), 2);
            // Flutter settles in 2 pumps; its second is the semantics update the jump asks for, and
            // accessibility is deferred, so this settles in 1.
            assert_eq!(tester.pump_and_settle_by(Duration::from_secs(60)), 1);
            assert_eq!(page(tester, controller.handle()), Some(2.0));
            tester.pump_widget(PageStorage::new(bucket.clone(), SizedBox::new()));
            assert_eq!(
                page_assert_message(tester, controller.handle()),
                PAGE_BEFORE_BUILD
            );
            tester.pump_widget(stored_page_view(
                bucket.clone(),
                "PageView",
                controller.clone(),
            ));
            assert_eq!(page(tester, controller.handle()), Some(2.0));

            let controller2 =
                PageController::new(&mut tester.cell.borrow_mut(), 0, false, 1.0, None, None);
            tester.pump_widget(stored_page_view(
                bucket,
                "Check it again against your list and see consistency!",
                controller2.clone(),
            ));
            assert_eq!(page(tester, controller2.handle()), Some(0.0));
        });
    }

    #[test]
    fn page_metrics_report_their_page() {
        let page = PageMetrics::new(
            Some(100.0),
            Some(200.0),
            Some(150.0),
            Some(25.0),
            AxisDirection::Right,
            1.0,
            3.0,
        );
        assert_eq!(page.page(), Some(6.0));
        let page2 = page.copy_with().with_pixels(page.pixels() - 100.0);
        assert_eq!(page2.page(), Some(4.0));
    }

    #[test]
    fn the_page_controller_rounds_away_a_precision_error() {
        test_widgets(|tester| {
            let controller = PageController::default(&mut tester.cell.borrow_mut());
            tester.pump_widget(ltr(PageView::new()
                .controller(controller.clone())
                .children((0..3).map(|_| SizedBox::expand().into_widget()))));
            // Simulate precision error.
            let position = controller.position(&tester.cell.borrow());
            position.jump_to(&mut tester.cell.borrow_mut(), 799.99999999999);
            assert_eq!(page(tester, controller.handle()), Some(1.0));
        });
    }

    #[test]
    fn pad_ends_reaches_the_sliver_fill_viewport() {
        test_widgets(|tester| {
            // PageView() defaults to true.
            tester.pump_widget(ltr(PageView::new()));
            assert!(tester.widget(
                find::by_type::<SliverFillViewport>().skip_offstage(false),
                |fill: &SliverFillViewport| fill.pad_ends
            ));

            // PageView(padEnds: false) is propagated properly.
            tester.pump_widget(ltr(PageView::new().pad_ends(false)));
            assert!(!tester.widget(
                find::by_type::<SliverFillViewport>().skip_offstage(false),
                |fill: &SliverFillViewport| fill.pad_ends
            ));
        });
    }

    #[test]
    fn the_page_controller_hears_its_position_attach_and_detach() {
        test_widgets(|tester| {
            let attach = Rc::new(Cell::new(0));
            let detach = Rc::new(Cell::new(0));
            let (attached, detached) = (Rc::clone(&attach), Rc::clone(&detach));
            let controller = PageController::new(
                &mut tester.cell.borrow_mut(),
                0,
                true,
                1.0,
                Some(Rc::new(move |_, _| attached.set(attached.get() + 1))),
                Some(Rc::new(move |_, _| detached.set(detached.get() + 1))),
            );
            tester.pump_widget(ltr(Center::new().child(
                PageView::new()
                    .controller(controller.clone())
                    .physics(
                        PageScrollPhysics::new()
                            .apply_to(Some(Rc::new(ClampingScrollPhysics::new()))),
                    )
                    .children((0..3).map(|_| Center::new().into_widget())),
            )));
            tester.pump_and_settle();

            assert_eq!(attach.get(), 1);
            assert_eq!(detach.get(), 0);

            tester.pump_widget(SizedBox::new());
            tester.pump_and_settle();

            assert_eq!(attach.get(), 1);
            assert_eq!(detach.get(), 1);
        });
    }

    // ---- page_view_test.dart: "PageView handles change of controller" ----

    fn keyed_page_view(
        key: &crate::framework::GlobalKey,
        controller: Option<Ref<PageController>>,
    ) -> WidgetRef {
        let mut page_view = PageView::new()
            .key(key.clone().into())
            .children((0..3).map(|i| Center::new().child(Text::new(i.to_string())).into_widget()));
        if let Some(controller) = controller {
            page_view = page_view.controller(controller.clone());
        }
        ltr(page_view)
    }

    /// `testPageViewWithController`: the first page onstage follows the controller's jumps when
    /// it controls the page view, and its jumps panic when it does not.
    fn check_page_view_with_controller(
        tester: &mut WidgetTester,
        controller: Ref<PageController>,
        controls: bool,
    ) {
        let current_visible_page = |tester: &WidgetTester| -> i32 {
            tester
                .widget(find::by_type::<Text>().first(), |text: &Text| {
                    text.data.clone()
                })
                .expect("the page's Text has data")
                .parse()
                .expect("the page's Text is its index")
        };
        let initial_page_in_view = current_visible_page(tester);
        for i in 0..3 {
            if controls {
                controller.jump_to_page(&mut tester.cell.borrow_mut(), i);
                tester.pump_and_settle();
                assert_eq!(current_visible_page(tester), i);
            } else {
                let jumped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    controller.jump_to_page(&mut tester.cell.borrow_mut(), i);
                }));
                assert!(jumped.is_err());
                assert_eq!(current_visible_page(tester), initial_page_in_view);
            }
        }
    }

    #[test]
    fn a_page_view_follows_a_controller_given_after_none() {
        test_widgets(|tester| {
            let key = crate::framework::GlobalKey::new();
            let controller = PageController::default(&mut tester.cell.borrow_mut());
            tester.pump_widget(keyed_page_view(&key, None));
            tester.pump_widget(keyed_page_view(&key, Some(controller.clone())));
            check_page_view_with_controller(tester, controller, true);
        });
    }

    #[test]
    fn a_page_view_follows_its_new_controller_and_not_its_old_one() {
        test_widgets(|tester| {
            let key = crate::framework::GlobalKey::new();
            let controller1 = PageController::default(&mut tester.cell.borrow_mut());
            let controller2 = PageController::default(&mut tester.cell.borrow_mut());
            tester.pump_widget(keyed_page_view(&key, Some(controller1.clone())));
            check_page_view_with_controller(tester, controller1.clone(), true);
            tester.pump_widget(keyed_page_view(&key, Some(controller2.clone())));
            check_page_view_with_controller(tester, controller1, false);
            check_page_view_with_controller(tester, controller2, true);
        });
    }

    #[test]
    fn a_page_view_leaves_a_controller_taken_away() {
        test_widgets(|tester| {
            let key = crate::framework::GlobalKey::new();
            let controller = PageController::default(&mut tester.cell.borrow_mut());
            tester.pump_widget(keyed_page_view(&key, Some(controller.clone())));
            check_page_view_with_controller(tester, controller.clone(), true);
            tester.pump_widget(keyed_page_view(&key, None));
            check_page_view_with_controller(tester, controller, false);
        });
    }

    #[test]
    fn a_page_view_without_a_controller_rebuilds_without_one() {
        test_widgets(|tester| {
            let key = crate::framework::GlobalKey::new();
            tester.pump_widget(keyed_page_view(&key, None));
            tester.pump_widget(keyed_page_view(&key, None));
        });
    }

    // ---- page_view_test.dart: scrollCacheExtent ----

    fn cached_page_view(
        controller: Ref<PageController>,
        allow_implicit_scrolling: bool,
        scroll_cache_extent: Option<ScrollCacheExtent>,
    ) -> WidgetRef {
        let mut page_view = PageView::new()
            .controller(controller.clone())
            .allow_implicit_scrolling(allow_implicit_scrolling)
            .children((0..3).map(|i| {
                SizedBox::new()
                    .key(ValueKey::new(i).into())
                    .child(Text::new(format!("Page {i}")))
                    .into_widget()
            }));
        if let Some(scroll_cache_extent) = scroll_cache_extent {
            page_view = page_view.scroll_cache_extent(scroll_cache_extent);
        }
        ltr(page_view)
    }

    /// Dart's `find.text(text, skipOffstage: false)`.
    fn built_text(text: &str) -> Finder {
        find::text(text).skip_offstage(false)
    }

    #[test]
    fn a_page_view_lays_out_its_scroll_cache_extent() {
        test_widgets(|tester| {
            let controller = PageController::default(&mut tester.cell.borrow_mut());
            tester.pump_widget(cached_page_view(
                controller,
                true,
                Some(ScrollCacheExtent::Viewport(1.0)),
            ));

            expect(tester, &find::text("Page 0"), finds_one_widget());
            expect(tester, &built_text("Page 1"), finds_one_widget());
            expect(tester, &built_text("Page 2"), finds_nothing());
        });
    }

    #[test]
    fn the_scroll_cache_extent_follows_allow_implicit_scrolling_by_default() {
        test_widgets(|tester| {
            let controller = PageController::default(&mut tester.cell.borrow_mut());

            // Default (allowImplicitScrolling: false) -> scrollCacheExtent: viewport(0.0)
            tester.pump_widget(cached_page_view(controller.clone(), false, None));
            expect(tester, &built_text("Page 1"), finds_nothing());

            // allowImplicitScrolling: true -> scrollCacheExtent: viewport(1.0)
            tester.pump_widget(cached_page_view(controller, true, None));
            expect(tester, &built_text("Page 1"), finds_one_widget());
        });
    }

    #[test]
    fn a_page_view_updates_its_scroll_cache_extent() {
        test_widgets(|tester| {
            let controller = PageController::default(&mut tester.cell.borrow_mut());

            // Start with viewport(1.0)
            tester.pump_widget(cached_page_view(
                controller.clone(),
                true,
                Some(ScrollCacheExtent::Viewport(1.0)),
            ));
            expect(tester, &built_text("Page 1"), finds_one_widget());

            // Update to viewport(2.0)
            tester.pump_widget(cached_page_view(
                controller.clone(),
                true,
                Some(ScrollCacheExtent::Viewport(2.0)),
            ));
            expect(tester, &built_text("Page 1"), finds_one_widget());
            expect(tester, &built_text("Page 2"), finds_one_widget());

            // Update back to viewport(1.0)
            tester.pump_widget(cached_page_view(
                controller,
                true,
                Some(ScrollCacheExtent::Viewport(1.0)),
            ));
            expect(tester, &built_text("Page 1"), finds_one_widget());
            expect(tester, &built_text("Page 2"), finds_nothing());
        });
    }

    #[test]
    #[should_panic(expected = "scrollCacheExtent and allowImplicitScrolling must be consistent")]
    fn a_zero_scroll_cache_extent_with_implicit_scrolling_panics_when_made_a_widget() {
        PageView::new()
            .allow_implicit_scrolling(true)
            .scroll_cache_extent(ScrollCacheExtent::Viewport(0.0))
            .children(states())
            .into_widget();
    }

    #[test]
    #[should_panic(expected = "scrollCacheExtent and allowImplicitScrolling must be consistent")]
    fn a_positive_scroll_cache_extent_without_implicit_scrolling_panics_when_made_a_widget() {
        PageView::new()
            .scroll_cache_extent(ScrollCacheExtent::Viewport(2.0))
            .children(states())
            .into_widget();
    }

    // ---- not in page_view_test.dart: `_getDirection` ----

    #[test]
    fn a_reversed_page_view_in_a_right_to_left_text_scrolls_to_the_right() {
        test_widgets(|tester| {
            tester.pump_widget(Directionality::new(
                TextDirection::Rtl,
                PageView::new().reverse(true).children(states()),
            ));
            assert_eq!(
                fill_viewport(tester)
                    .constraints(&tester.cell.borrow())
                    .axis_direction,
                AxisDirection::Right
            );

            tester.pump_widget(Directionality::new(
                TextDirection::Rtl,
                PageView::new().children(states()),
            ));
            assert_eq!(
                fill_viewport(tester)
                    .constraints(&tester.cell.borrow())
                    .axis_direction,
                AxisDirection::Left
            );
            assert_eq!(
                tester.get_top_left(find::text("Alabama")),
                Offset::new(0.0, 0.0)
            );

            tester.pump_widget(ltr(PageView::new()
                .scroll_direction(Axis::Vertical)
                .reverse(true)
                .children(states())));
            assert_eq!(
                fill_viewport(tester)
                    .constraints(&tester.cell.borrow())
                    .axis_direction,
                AxisDirection::Up
            );
        });
    }
    // ---- page_view_test.dart, the rest ----

    #[test]
    fn a_page_view_builder_asks_its_find_child_index_callback_on_update() {
        test_widgets(|tester| {
            let finder_called = Rc::new(Cell::new(false));
            let page_view = |item_count: i32| {
                let finder_called = Rc::clone(&finder_called);
                ltr(PageView::builder(|_, _, index| {
                    Some(
                        SizedBox::new()
                            .key(ValueKey::new(index.to_string()).into())
                            .height(2000.0)
                            .into_widget(),
                    )
                })
                .item_count(item_count)
                .find_child_index_callback(Rc::new(move |_| {
                    finder_called.set(true);
                    None
                }))
                .build())
            };
            tester.pump_widget(page_view(7));
            assert!(!finder_called.get());

            // Trigger update.
            tester.pump_widget(page_view(77));
            assert!(finder_called.get());
        });
    }

    #[test]
    fn a_page_view_does_not_squish_when_overscrolled() {
        let variant = TargetPlatformVariant::new([TargetPlatform::IOS, TargetPlatform::MacOS]);
        test_widgets_variant(variant, |tester| {
            let platform = tester.target_platform();
            tester.pump_widget(ltr(PageView::new().children((0..10).map(|i| {
                Container::new()
                    .key(ValueKey::new(i).into())
                    .color(K_BLUE)
                    .into_widget()
            }))));
            let size_of_first =
                |tester: &WidgetTester| tester.get_size(find::by_key(ValueKey::new(0)));
            let left_of_first =
                |tester: &WidgetTester| tester.get_top_left(find::by_key(ValueKey::new(0))).dx();

            assert_eq!(left_of_first(tester), 0.0);
            assert_eq!(size_of_first(tester), Size::new(800.0, 600.0));

            // Going into overscroll.
            tester.drag(find::by_type::<PageView>(), Offset::new(100.0, 0.0));
            tester.pump();

            assert!(left_of_first(tester) > 0.0);
            assert_eq!(size_of_first(tester), Size::new(800.0, 600.0));

            // Easing overscroll past overscroll limit.
            let back = if platform == TargetPlatform::MacOS {
                -500.0
            } else {
                -200.0
            };
            tester.drag(find::by_type::<PageView>(), Offset::new(back, 0.0));
            tester.pump();

            assert!(left_of_first(tester) < 0.0);
            assert_eq!(size_of_first(tester), Size::new(800.0, 600.0));
        });
    }

    #[test]
    fn all_visible_pages_receive_taps() {
        test_widgets(|tester| {
            let controller = new_controller(tester, 0, 1.0 / 4.0);
            let tapped_index = Rc::new(Cell::new(-1));
            let tapped = Rc::clone(&tapped_index);
            tester.pump_widget(ltr(PageView::builder(move |_, _, index| {
                let tapped = Rc::clone(&tapped);
                Some(
                    GestureDetector::new()
                        .on_tap(Listener::new(move |_| tapped.set(index)))
                        .child(SizedBox::expand().child(Text::new(index.to_string())))
                        .into_widget(),
                )
            })
            .item_count(20)
            .build()
            .controller(controller.clone())));

            // The first 3 items should be visible and tappable.
            for index in [0, 1, 2] {
                expect(tester, &find::text(&index.to_string()), finds_one_widget());
                // The center of page 2's x-coordinate is 800, so we have to manually
                // offset it a bit to make sure the tap lands within the screen.
                let center =
                    tester.get_center(find::text(&index.to_string())) - Offset::new(3.0, 0.0);
                let pointer = tester.next_pointer();
                tester.tap_at(center, pointer);
                assert_eq!(tapped_index.get(), index);
            }

            controller.jump_to_page(&mut tester.cell.borrow_mut(), 19);
            tester.pump();
            // The last 3 items should be visible and tappable.
            for index in [17, 18, 19] {
                expect(tester, &find::text(&index.to_string()), finds_one_widget());
                tester.tap(find::text(&index.to_string()));
                assert_eq!(tapped_index.get(), index);
            }
        });
    }

    #[test]
    fn a_page_view_gives_its_viewport_its_clip_behavior() {
        test_widgets(|tester| {
            let page_view =
                || PageView::new().children([SizedBox::new().height(2000.0).into_widget()]);
            tester.pump_widget(ltr(page_view()));

            // 1st, check that the render object has received the default clip behavior.
            let viewport = *tester
                .all::<RenderViewport>()
                .first()
                .expect("a mounted viewport");
            assert_eq!(
                viewport.clip_behavior(&tester.cell.borrow()),
                Clip::HardEdge
            );

            // 3rd, pump a new widget to check that the render object can update its clip behavior.
            tester.pump_widget(ltr(page_view().clip_behavior(Clip::AntiAlias)));
            assert_eq!(
                viewport.clip_behavior(&tester.cell.borrow()),
                Clip::AntiAlias
            );
        });
    }

    #[test]
    fn a_jump_to_the_last_of_many_narrow_pages_survives_a_precision_error() {
        test_widgets(|tester| {
            let controller = new_controller(tester, 152, 1.0);
            tester.pump_widget(
                Center::new().child(
                    SizedBox::new()
                        .width(392.72727272727275)
                        .child(ltr(PageView::builder(|_, _, _| {
                            Some(SizedBox::new().into_widget())
                        })
                        .item_count(366)
                        .build()
                        .controller(controller.clone()))),
                ),
            );

            controller.jump_to_page(&mut tester.cell.borrow_mut(), 365);
            tester.pump();
        });
    }

    fn three_pages() -> impl Iterator<Item = WidgetRef> {
        (0..3).map(|_| SizedBox::expand().into_widget())
    }

    fn two_page_views(controller: &Ref<PageController>) -> WidgetRef {
        ltr(Column::new().children([
            Expanded::new(
                PageView::new()
                    .controller(controller.clone())
                    .children(three_pages()),
            )
            .into_widget(),
            Expanded::new(
                PageView::new()
                    .controller(controller.clone())
                    .children(three_pages()),
            )
            .into_widget(),
        ]))
    }

    #[test]
    #[should_panic(expected = "Multiple PageViews are attached to the same PageController.")]
    fn animate_to_page_asserts_when_its_controller_has_two_page_views() {
        test_widgets(|tester| {
            let controller = PageController::default(&mut tester.cell.borrow_mut());
            tester.pump_widget(two_page_views(&controller));

            drop(controller.animate_to_page(
                &mut tester.cell.borrow_mut(),
                2,
                Duration::from_millis(300),
                Curves::ease(),
            ));
        });
    }

    #[test]
    #[should_panic(expected = "Multiple PageViews are attached to the same PageController.")]
    fn jump_to_page_asserts_when_its_controller_has_two_page_views() {
        test_widgets(|tester| {
            let controller = PageController::default(&mut tester.cell.borrow_mut());
            tester.pump_widget(two_page_views(&controller));

            controller.jump_to_page(&mut tester.cell.borrow_mut(), 2);
        });
    }

    /// The message a call panics with, or `None` if it returns.
    fn panic_message(call: impl FnOnce()) -> Option<String> {
        let payload = std::panic::catch_unwind(std::panic::AssertUnwindSafe(call)).err()?;
        Some(
            payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| {
                    payload
                        .downcast_ref::<&str>()
                        .map(|message| message.to_string())
                })
                .unwrap_or_default(),
        )
    }

    fn page_view_with(controller: Option<&Ref<PageController>>) -> WidgetRef {
        let mut page_view = PageView::new().children(three_pages());
        if let Some(controller) = controller {
            page_view = page_view.controller(controller.clone());
        }
        ltr(page_view)
    }

    #[test]
    fn animate_to_page_asserts_only_while_its_controller_is_unattached() {
        test_widgets(|tester| {
            let controller = PageController::default(&mut tester.cell.borrow_mut());

            // pageController is not attached to PageView
            tester.pump_widget(page_view_with(None));
            let animate = |tester: &WidgetTester| {
                drop(controller.animate_to_page(
                    &mut tester.cell.borrow_mut(),
                    2,
                    Duration::from_millis(300),
                    Curves::ease(),
                ));
            };
            assert_eq!(
                panic_message(|| animate(tester)).as_deref(),
                Some("PageController is not attached to a PageView.")
            );

            // pageController is attached to PageView
            tester.pump_widget(page_view_with(Some(&controller)));
            assert_eq!(panic_message(|| animate(tester)), None);
        });
    }

    #[test]
    fn jump_to_page_asserts_only_while_its_controller_is_unattached() {
        test_widgets(|tester| {
            let controller = PageController::default(&mut tester.cell.borrow_mut());

            // pageController is not attached to PageView
            tester.pump_widget(page_view_with(None));
            let jump =
                |tester: &WidgetTester| controller.jump_to_page(&mut tester.cell.borrow_mut(), 2);
            assert_eq!(
                panic_message(|| jump(tester)).as_deref(),
                Some("PageController is not attached to a PageView.")
            );

            // pageController is attached to PageView
            tester.pump_widget(page_view_with(Some(&controller)));
            assert_eq!(panic_message(|| jump(tester)), None);
        });
    }

    #[test]
    fn the_page_is_none_until_the_content_dimensions_are_known() {
        test_widgets(|tester| {
            let controller = PageController::default(&mut tester.cell.borrow_mut());
            let current_page_is_none = Rc::new(Cell::new(None));
            let page_view = || {
                let current_page_is_none = Rc::clone(&current_page_is_none);
                let observed = controller.handle();
                ltr(Column::new().children([Expanded::new(
                    PageView::new()
                        .controller(controller.clone())
                        .children([Builder::new(move |app, _| {
                            current_page_is_none.set(Some(observed.page(app).is_none()));
                            Center::new().into_widget()
                        })
                        .into_widget()]),
                )
                .into_widget()]))
            };
            tester.pump_widget(page_view());
            assert_eq!(current_page_is_none.get(), Some(true));

            // A rebuild, as the test's `setState` asks for.
            tester.pump_widget(page_view());
            assert_eq!(current_page_is_none.get(), Some(false));
            assert!(page(tester, controller.handle()).is_some());
        });
    }

    /// A page view in an offstage overlay entry that keeps its state, under an opaque one: built
    /// and never laid out, as the page of a route under another route is.
    fn page_view_before_layout(
        tester: &WidgetTester,
        controller: Ref<PageController>,
    ) -> WidgetRef {
        let mut app = tester.cell.borrow_mut();
        let below = OverlayEntry::new(
            &mut app,
            Rc::new(move |_, _| {
                PageView::new()
                    .controller(controller.clone())
                    .children((0..2).map(|_| SizedBox::expand().into_widget()))
                    .into_widget()
            }),
            false,
            true,
            false,
        );
        let above = OverlayEntry::new(
            &mut app,
            Rc::new(|_, _| SizedBox::shrink().into_widget()),
            true,
            false,
            false,
        );
        ltr(Overlay::new().initial_entries([below, above]))
    }

    #[test]
    fn jump_to_page_before_layout_does_not_panic() {
        test_widgets(|tester| {
            let controller = PageController::default(&mut tester.cell.borrow_mut());
            let page_view = page_view_before_layout(tester, controller.clone());
            tester.pump_widget(page_view);
            assert!(!viewport_was_laid_out(tester));

            controller.jump_to_page(&mut tester.cell.borrow_mut(), 1);
        });
    }

    #[test]
    fn animate_to_page_before_layout_does_not_panic() {
        test_widgets(|tester| {
            let controller = PageController::default(&mut tester.cell.borrow_mut());
            let page_view = page_view_before_layout(tester, controller.clone());
            tester.pump_widget(page_view);
            assert!(!viewport_was_laid_out(tester));

            drop(controller.animate_to_page(
                &mut tester.cell.borrow_mut(),
                1,
                Duration::from_millis(50),
                Curves::bounce_in(),
            ));
        });
    }

    /// Whether the page view's viewport has been laid out: the test's premise is that it has not.
    fn viewport_was_laid_out(tester: &WidgetTester) -> bool {
        tester
            .all::<RenderViewport>()
            .first()
            .is_some_and(|viewport| viewport.has_size(&tester.cell.borrow()))
    }

    #[test]
    fn show_on_screen_scrolls_a_page_view_that_allows_implicit_scrolling() {
        for scroll_cache_extent in [Some(ScrollCacheExtent::Viewport(1.0)), None] {
            test_widgets(|tester| {
                let controller = PageController::default(&mut tester.cell.borrow_mut());
                tester.pump_widget(cached_page_view_of(
                    controller.clone(),
                    scroll_cache_extent,
                    4,
                ));

                // Page 2 (index 1) is within the implicit cache extent.
                let target_finder = find::text("Page 2").skip_offstage(false);
                expect(tester, &target_finder, finds_one_widget());
                let target = tester.render_object(target_finder);
                target.show_on_screen(
                    &mut tester.cell.borrow_mut(),
                    None,
                    None,
                    Duration::ZERO,
                    Curves::ease(),
                );
                tester.pump_and_settle();

                // Should scroll.
                assert_eq!(
                    page(tester, controller.handle()),
                    Some(1.0),
                    "{scroll_cache_extent:?}"
                );
            });
        }
    }

    fn cached_page_view_of(
        controller: Ref<PageController>,
        scroll_cache_extent: Option<ScrollCacheExtent>,
        page_count: usize,
    ) -> WidgetRef {
        let mut page_view = PageView::new().controller(controller.clone());
        if let Some(scroll_cache_extent) = scroll_cache_extent {
            page_view = page_view.scroll_cache_extent(scroll_cache_extent);
        }
        ltr(page_view
            .allow_implicit_scrolling(true)
            .children((1..=page_count).map(|i| Text::new(format!("Page {i}")).into_widget())))
    }

    // ---- not in page_view_test.dart: `PageScrollPhysics` targets ----

    /// Pins `PageScrollPhysics` where its whole-viewport pages agree with Dart's
    /// `_PagePosition` branch: a viewport fraction of 1.0 and a position in range. The target is
    /// where the spring settles.
    #[test]
    fn page_scroll_physics_settle_on_the_nearest_page_or_the_next_one_flung_to() {
        let physics = PageScrollPhysics::new();
        let target = |pixels: f64, velocity: f64| {
            let metrics = FixedScrollMetrics::new(
                Some(0.0),
                Some(2400.0),
                Some(pixels),
                Some(800.0),
                AxisDirection::Right,
                3.0,
            );
            physics
                .create_ballistic_simulation(&metrics, velocity)
                .map(|simulation| simulation.x(10.0).round())
        };

        // Released short of halfway: back to the page it left.
        assert_eq!(target(300.0, 0.0), Some(0.0));
        assert_eq!(target(1100.0, 0.0), Some(800.0));
        // Released past halfway: on to the next page.
        assert_eq!(target(500.0, 0.0), Some(800.0));
        // Flung either way: on to the page it was flung towards.
        assert_eq!(target(100.0, 1000.0), Some(800.0));
        assert_eq!(target(700.0, -1000.0), Some(0.0));
        assert_eq!(target(1700.0, -1000.0), Some(1600.0));
        // Already on a page: no simulation.
        assert_eq!(target(800.0, 0.0), None);

        // The same targets through a page view whose position its controller reports in pages.
        test_widgets(|tester| {
            let controller = PageController::default(&mut tester.cell.borrow_mut());
            tester.pump_widget(ltr(PageView::new()
                .controller(controller.clone())
                .children(states())));
            tester.drag(find::by_type::<PageView>(), Offset::new(-300.0, 0.0));
            tester.pump_and_settle();
            assert_eq!(page(tester, controller.handle()), Some(0.0));
            tester.drag(find::by_type::<PageView>(), Offset::new(-500.0, 0.0));
            tester.pump_and_settle();
            assert_eq!(page(tester, controller.handle()), Some(1.0));
            tester.fling(
                find::by_type::<PageView>(),
                Offset::new(-100.0, 0.0),
                1000.0,
            );
            tester.pump_and_settle();
            assert_eq!(page(tester, controller.handle()), Some(2.0));
        });
    }
}
