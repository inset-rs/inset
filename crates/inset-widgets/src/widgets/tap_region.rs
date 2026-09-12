//! Flutter counterpart: `widgets/tap_region.dart`.

use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug};
use std::rc::Rc;

use inset_embedder::{Matrix4, Offset, Size, TextBaseline};
use inset_foundation::{App, Handle, HandleId};
use inset_gestures::{
    GestureArenaMember, GestureBinding, GestureDisposition, HitTestEntry, HitTestTarget,
    PointerDownEvent, PointerEvent, PointerUpEvent,
};
use inset_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, HitTestBehavior,
    PaintingContext, RenderBox, RenderBoxData, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin, RenderProxyBoxMixin,
    RenderProxyBoxWithHitTestBehavior,
};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, RenderObjectWidget, SingleChildRenderObjectWidget, WidgetRef,
};
use crate::widgets::editable_text::EditableText;
use crate::widgets::routes::AnyModalRoute;

/// Enable if you want verbose logging about tap region changes.
const K_DEBUG_TAP_REGION: bool = false;

fn tap_region_debug(message: &str, details: &[&str]) -> bool {
    if K_DEBUG_TAP_REGION {
        eprintln!("TAP REGION: {message}");
        for detail in details {
            eprintln!("    {detail}");
        }
    }
    true
}

/// Signature for a callback called for a [`PointerDownEvent`] relative to a [`TapRegion`].
///
/// See also:
///
///  * [`TapRegion::on_tap_outside`], which is of this type.
///  * [`TapRegion::on_tap_inside`], which is of this type.
///  * [`TapRegionUpCallback`], which is similar but for [`PointerUpEvent`]s.
pub type TapRegionCallback = Rc<dyn Fn(&mut App, PointerDownEvent)>;

/// Signature for a callback called for a [`PointerUpEvent`] relative to a [`TapRegion`].
///
/// See also:
///
///  * [`TapRegion::on_tap_up_outside`], which is of this type.
///  * [`TapRegion::on_tap_up_inside`], which is of this type.
///  * [`TapRegionCallback`], which is similar but for [`PointerDownEvent`]s.
pub type TapRegionUpCallback = Rc<dyn Fn(&mut App, PointerUpEvent)>;

/// Dart's `Object?` [`TapRegion::group_id`]: identity-comparable group keys.
///
/// [`TypeId`] matches [`EditableText::group_id`](crate::EditableText::group_id). [`HandleId`]
/// is a handle's identity when the group is an arena object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TapRegionGroupId {
    /// A type object, as `groupId: EditableText` in Dart.
    Type(TypeId),
    /// An arena object's identity.
    Handle(HandleId),
}

impl From<TypeId> for TapRegionGroupId {
    fn from(id: TypeId) -> TapRegionGroupId {
        TapRegionGroupId::Type(id)
    }
}

impl From<HandleId> for TapRegionGroupId {
    fn from(id: HandleId) -> TapRegionGroupId {
        TapRegionGroupId::Handle(id)
    }
}

/// An interface for registering and unregistering a [`RenderTapRegion`]
/// (typically created with a [`TapRegion`] widget) with a
/// [`RenderTapRegionSurface`] (typically created with a [`TapRegionSurface`]
/// widget).
pub trait TapRegionRegistry: RenderObject {
    /// Register the given [`RenderTapRegion`] with the registry.
    fn register_tap_region(
        self: RenderHandle<Self>,
        app: &mut App,
        region: RenderHandle<RenderTapRegion>,
    );

    /// Unregister the given [`RenderTapRegion`] with the registry.
    fn unregister_tap_region(
        self: RenderHandle<Self>,
        app: &mut App,
        region: RenderHandle<RenderTapRegion>,
    );

    /// Allows finding of the nearest [`TapRegionRegistry`], such as a
    /// [`RenderTapRegionSurface`].
    ///
    /// Panics if a [`TapRegionRegistry`] isn't found.
    ///
    /// Dart `TapRegionRegistry.of`.
    fn of(app: &App, context: BuildContext) -> RenderHandle<RenderTapRegionSurface> {
        Self::maybe_of(app, context).unwrap_or_else(|| {
            panic!(
                "TapRegionRegistry.of() was called with a context that does not contain a \
                 TapRegionSurface widget.\n\
                 No TapRegionSurface widget ancestor could be found starting from the context \
                 that was passed to TapRegionRegistry.of().\n\
                 The context used was:\n  {context:?}"
            )
        })
    }

    /// Allows finding of the nearest [`TapRegionRegistry`], such as a
    /// [`RenderTapRegionSurface`].
    ///
    /// Dart `TapRegionRegistry.maybeOf`.
    fn maybe_of(app: &App, context: BuildContext) -> Option<RenderHandle<RenderTapRegionSurface>> {
        context.find_ancestor_render_object_of_type::<RenderTapRegionSurface>(app)
    }
}

/// A widget that provides notification of a tap inside or outside of a set of
/// registered regions, without participating in the [gesture
/// disambiguation](https://flutter.dev/to/gesture-disambiguation)
/// system.
///
/// The regions are defined by adding [`TapRegion`] widgets to the widget tree
/// around the regions of interest, and they will register with this
/// [`TapRegionSurface`]. Each of the tap regions can optionally belong to a group
/// by assigning a [`TapRegion::group_id`], where all the regions with the same
/// group id act as if they were all one region.
///
/// When a tap down or tap up outside of a registered region or region group is
/// detected, its [`TapRegion::on_tap_outside`] or [`TapRegion::on_tap_up_outside`]
/// callback is called, respectively. If the tap is outside one member of a
/// group, but inside another, no notification is made.
///
/// When a tap down or tap up inside of a registered region or region group is
/// detected, its [`TapRegion::on_tap_inside`] or [`TapRegion::on_tap_up_inside`]
/// callback is called, respectively. If the tap is inside one member of a
/// group, all members are notified.
///
/// The [`TapRegionSurface`] should be defined at the highest level needed to
/// encompass the entire area where taps should be monitored. This is typically
/// around the entire app. If the entire app isn't covered, then taps outside of
/// the [`TapRegionSurface`] will be ignored and no [`TapRegion::on_tap_outside`] or
/// [`TapRegion::on_tap_up_outside`] calls will be made for those events.
///
/// [`TapRegion`]s register only with the nearest ancestor [`TapRegionSurface`].
///
/// See also:
///
///  * [`RenderTapRegionSurface`], the render object that is inserted into the
///    render tree by this widget.
#[derive(Debug)]
pub struct TapRegionSurface {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl TapRegionSurface {
    /// Creates a [`TapRegionSurface`].
    ///
    /// The [`child`](Self::child) attribute is required.
    pub fn new<K>(child: impl IntoWidget<K>) -> TapRegionSurface {
        TapRegionSurface {
            key: None,
            child: child.into_widget(),
        }
    }

    /// Dart `TapRegionSurface(key:)`.
    pub fn key(mut self, key: KeyRef) -> TapRegionSurface {
        self.key = Some(key);
        self
    }
}

impl RenderObjectWidget for TapRegionSurface {
    type RenderObject = RenderTapRegionSurface;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderTapRegionSurface::new(app, None).as_object()
    }

    fn update_render_object(
        &self,
        _app: &mut App,
        _context: BuildContext,
        _render_object: RenderHandle<RenderTapRegionSurface>,
    ) {
    }
}

impl SingleChildRenderObjectWidget for TapRegionSurface {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

/// Classified registered regions for one event.
struct ClassifiedTapRegions {
    inside: Vec<RenderHandle<RenderTapRegion>>,
    outside: Vec<RenderHandle<RenderTapRegion>>,
}

/// A render object that provides notification of a tap inside or outside of a
/// set of registered regions, without participating in the [gesture
/// disambiguation](https://flutter.dev/to/gesture-disambiguation) system
/// (other than to consume tap down events if [`TapRegion::consume_outside_taps`] is
/// true).
///
/// See [`TapRegionSurface`].
pub struct RenderTapRegionSurface {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    registered_regions: HashSet<RenderHandle<RenderTapRegion>>,
    group_id_to_regions: HashMap<TapRegionGroupId, HashSet<RenderHandle<RenderTapRegion>>>,
    dummy_tap: Handle<DummyTapRecognizer>,
}

impl RenderTapRegionSurface {
    /// Creates a [`RenderTapRegionSurface`].
    pub fn new(app: &mut App, child: Option<AnyRenderBox>) -> RenderHandle<RenderTapRegionSurface> {
        let dummy_tap = app.create(DummyTapRecognizer);
        let this = RenderHandle::new_box(
            app,
            RenderTapRegionSurface {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                registered_regions: HashSet::new(),
                group_id_to_regions: HashMap::new(),
                dummy_tap,
            },
        );
        this.set_child(app, child);
        this
    }

    /// Dart `TapRegionRegistry.of`.
    pub fn of(app: &App, context: BuildContext) -> RenderHandle<Self> {
        <Self as TapRegionRegistry>::of(app, context)
    }

    /// Dart `TapRegionRegistry.maybeOf`.
    pub fn maybe_of(app: &App, context: BuildContext) -> Option<RenderHandle<Self>> {
        <Self as TapRegionRegistry>::maybe_of(app, context)
    }

    /// Classifies registered tap regions as inside or outside based on which
    /// regions appear in the hit test path. Grouped regions are treated as a
    /// single unit: if any member of a group is hit, all members are considered
    /// inside.
    fn classify_regions(
        self: RenderHandle<Self>,
        app: &App,
        path: &[HandleId],
    ) -> ClassifiedTapRegions {
        let hit_ids: HashSet<HandleId> = path.iter().copied().collect();
        let registered = &self.get(app).registered_regions;
        debug_assert!(tap_region_debug(
            &format!(
                "Tap event hit {} descendants.",
                registered
                    .iter()
                    .filter(|region| hit_ids.contains(&region.handle().id()))
                    .count()
            ),
            &[],
        ));

        let mut inside_regions = HashSet::new();
        for region in registered {
            if !hit_ids.contains(&region.handle().id()) {
                continue;
            }
            match region.group_id(app) {
                None => {
                    inside_regions.insert(*region);
                }
                Some(group_id) => {
                    let members = self
                        .get(app)
                        .group_id_to_regions
                        .get(&group_id)
                        .expect("a grouped registered region has a group set");
                    inside_regions.extend(members.iter().copied());
                }
            }
        }
        // Materialize the outside list so callbacks that mutate registrations
        // during iteration do not affect the snapshot we are walking.
        let outside = registered
            .iter()
            .copied()
            .filter(|region| !inside_regions.contains(region))
            .collect();
        ClassifiedTapRegions {
            inside: inside_regions.into_iter().collect(),
            outside,
        }
    }

    /// Flutter's `handleEvent` after the cached hit-test path is recovered.
    fn handle_event_from_cache(
        self: RenderHandle<Self>,
        app: &mut App,
        event: &PointerEvent,
        path: &[HandleId],
    ) {
        debug_assert!({
            let all_enabled = self
                .get(app)
                .registered_regions
                .iter()
                .all(|region| region.enabled(app));
            if !all_enabled {
                panic!("A RenderTapRegion was registered when it was disabled.");
            }
            true
        });

        if !matches!(event, PointerEvent::Down(_) | PointerEvent::Up(_)) {
            return;
        }

        if self.get(app).registered_regions.is_empty() {
            debug_assert!(tap_region_debug(
                "Ignored tap event because no regions are registered.",
                &[],
            ));
            return;
        }

        let ClassifiedTapRegions { inside, outside } = self.classify_regions(app, path);

        let mut consume_outside_taps = false;
        for region in outside {
            match event {
                PointerEvent::Down(event) => {
                    debug_assert!(tap_region_debug(
                        &format!("Calling onTapOutside for {region:?}"),
                        &[],
                    ));
                    if let Some(callback) = region.on_tap_outside(app) {
                        callback(app, event.clone());
                    }
                }
                PointerEvent::Up(event) => {
                    debug_assert!(tap_region_debug(
                        &format!("Calling onTapUpOutside for {region:?}"),
                        &[],
                    ));
                    if let Some(callback) = region.on_tap_up_outside(app) {
                        callback(app, event.clone());
                    }
                }
                _ => {}
            }
            if region.consume_outside_taps(app) {
                debug_assert!(tap_region_debug(
                    &format!(
                        "Stopping tap propagation for {region:?} (and all of {:?})",
                        region.group_id(app)
                    ),
                    &[],
                ));
                consume_outside_taps = true;
            }
        }
        for region in inside {
            match event {
                PointerEvent::Down(event) => {
                    debug_assert!(tap_region_debug(
                        &format!("Calling onTapInside for {region:?}"),
                        &[],
                    ));
                    if let Some(callback) = region.on_tap_inside(app) {
                        callback(app, event.clone());
                    }
                }
                PointerEvent::Up(event) => {
                    debug_assert!(tap_region_debug(
                        &format!("Calling onTapUpInside for {region:?}"),
                        &[],
                    ));
                    if let Some(callback) = region.on_tap_up_inside(app) {
                        callback(app, event.clone());
                    }
                }
                _ => {}
            }
        }

        // If any of the "outside" regions have consumeOutsideTaps set, then stop
        // the propagation of the event through the gesture recognizer by adding it
        // to the recognizer and immediately resolving it.
        if consume_outside_taps && let PointerEvent::Down(event) = event {
            let dummy = DummyTapRecognizerMember(self.get(app).dummy_tap);
            GestureBinding::instance(app)
                .gesture_arena(app)
                .add(app, event.pointer, dummy)
                .resolve(app, GestureDisposition::Accepted);
        }
    }
}

impl TapRegionRegistry for RenderTapRegionSurface {
    fn register_tap_region(
        self: RenderHandle<Self>,
        app: &mut App,
        region: RenderHandle<RenderTapRegion>,
    ) {
        debug_assert!(tap_region_debug(
            &format!("Region {region:?} registered."),
            &[],
        ));
        debug_assert!(!self.get(app).registered_regions.contains(&region));
        self.get_mut(app).registered_regions.insert(region);
        if let Some(group_id) = region.group_id(app) {
            self.get_mut(app)
                .group_id_to_regions
                .entry(group_id)
                .or_default()
                .insert(region);
        }
    }

    fn unregister_tap_region(
        self: RenderHandle<Self>,
        app: &mut App,
        region: RenderHandle<RenderTapRegion>,
    ) {
        debug_assert!(tap_region_debug(
            &format!("Region {region:?} unregistered."),
            &[],
        ));
        debug_assert!(self.get(app).registered_regions.contains(&region));
        self.get_mut(app).registered_regions.remove(&region);
        if let Some(group_id) = region.group_id(app) {
            debug_assert!(self.get(app).group_id_to_regions.contains_key(&group_id));
            let empty = {
                let members = self
                    .get_mut(app)
                    .group_id_to_regions
                    .get_mut(&group_id)
                    .expect("checked");
                members.remove(&region);
                members.is_empty()
            };
            if empty {
                self.get_mut(app).group_id_to_regions.remove(&group_id);
            }
        }
    }
}

/// Dart's `Expando<BoxHitTestResult>` keyed by the surface's hit-test entry: the
/// path snapshot lives on the entry so each concurrent pointer keeps its own
/// classification input.
#[derive(Debug)]
struct TapRegionSurfaceHitTarget {
    surface: RenderHandle<RenderTapRegionSurface>,
    path: Vec<HandleId>,
}

impl HitTestTarget for TapRegionSurfaceHitTarget {
    fn handle_event(&self, app: &mut App, event: &PointerEvent, _entry: &HitTestEntry) {
        self.surface.handle_event_from_cache(app, event, &self.path);
    }

    fn retained_handle(&self) -> Option<HandleId> {
        Some(self.surface.handle().id())
    }
}

/// A dummy tap recognizer so that we don't have to deal with the lifecycle of
/// `TapGestureRecognizer`, since we're just going to immediately resolve it
/// anyhow.
struct DummyTapRecognizer;

#[derive(Clone, Copy)]
struct DummyTapRecognizerMember(Handle<DummyTapRecognizer>);

impl GestureArenaMember for DummyTapRecognizerMember {
    fn accept_gesture(&self, _app: &mut App, _pointer: i64) {}

    fn reject_gesture(&self, _app: &mut App, _pointer: i64) {}

    fn member_id(&self) -> HandleId {
        self.0.id()
    }
}

impl RenderObjectWithChildMixin for RenderTapRegionSurface {
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

impl RenderProxyBoxMixin for RenderTapRegionSurface {}

impl RenderProxyBoxWithHitTestBehavior for RenderTapRegionSurface {
    fn behavior(self: RenderHandle<Self>, _app: &App) -> HitTestBehavior {
        HitTestBehavior::DeferToChild
    }
}

impl RenderObject for RenderTapRegionSurface {
    inset_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
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

impl RenderBox for RenderTapRegionSurface {
    inset_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        if !self.size(app).contains(position) {
            return false;
        }

        let hit_target = RenderBox::hit_test_children(self, app, result, position)
            || RenderBox::hit_test_self(self, app, position);

        if hit_target {
            let path = result
                .path()
                .iter()
                .filter_map(|entry| entry.target().retained_handle())
                .collect();
            result.add(HitTestEntry::new(TapRegionSurfaceHitTarget {
                surface: self,
                path,
            }));
        }

        hit_target
    }

    fn hit_test_self(self: RenderHandle<Self>, app: &App, position: Offset) -> bool {
        RenderProxyBoxWithHitTestBehavior::hit_test_self(self, app, position)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// A widget that defines a region that can detect taps inside or outside of
/// itself and any group of regions it belongs to, without participating in the
/// [gesture
/// disambiguation](https://flutter.dev/to/gesture-disambiguation) system
/// (other than to consume tap down events if [`consume_outside_taps`](Self::consume_outside_taps) is true).
///
/// This widget indicates to the nearest ancestor [`TapRegionSurface`] that the
/// region occupied by its child will participate in the tap detection for that
/// surface.
///
/// If this region belongs to a group (by virtue of its [`group_id`](Self::group_id)), all the
/// regions in the group will act as one.
///
/// If there is no [`TapRegionSurface`] ancestor, [`TapRegion`] will do nothing.
///
/// [`TapRegion`] is aware of the routes in the navigator, so that [`on_tap_outside`](Self::on_tap_outside)
/// or [`on_tap_up_outside`](Self::on_tap_up_outside) isn't called after the user navigates to a different page.
pub struct TapRegion {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
    /// Whether or not this [`TapRegion`] is enabled as part of the composite region.
    pub enabled: bool,
    /// How to behave during hit testing when deciding how the hit test propagates
    /// to children and whether to consider targets behind this [`TapRegion`].
    ///
    /// Defaults to [`HitTestBehavior::DeferToChild`].
    pub behavior: HitTestBehavior,
    /// A callback to be invoked when a tap down is detected outside of this
    /// [`TapRegion`] and any other region with the same [`group_id`](Self::group_id), if any.
    pub on_tap_outside: Option<TapRegionCallback>,
    /// A callback to be invoked when a tap down is detected inside of this
    /// [`TapRegion`], or any other tap region with the same [`group_id`](Self::group_id), if any.
    pub on_tap_inside: Option<TapRegionCallback>,
    /// A callback to be invoked when a tap up is detected outside of this
    /// [`TapRegion`] and any other region with the same [`group_id`](Self::group_id), if any.
    pub on_tap_up_outside: Option<TapRegionUpCallback>,
    /// A callback to be invoked when a tap up is detected inside of this
    /// [`TapRegion`], or any other tap region with the same [`group_id`](Self::group_id), if any.
    pub on_tap_up_inside: Option<TapRegionUpCallback>,
    /// An optional group ID that groups [`TapRegion`]s together so that they
    /// operate as one region. If any member of a group is hit by a particular
    /// tap, then the [`on_tap_outside`](Self::on_tap_outside) / [`on_tap_up_outside`](Self::on_tap_up_outside) will not be called for
    /// any members of the group. If any member of the group is hit, then all
    /// members will have their [`on_tap_inside`](Self::on_tap_inside) / [`on_tap_up_inside`](Self::on_tap_up_inside) called.
    ///
    /// If the group id is `None`, then only this region is hit tested.
    pub group_id: Option<TapRegionGroupId>,
    /// If true, then the group that this region belongs to will stop the
    /// propagation of all events in the gesture arena.
    ///
    /// Defaults to false.
    pub consume_outside_taps: bool,
    /// An optional debug label to help with debugging in debug mode.
    ///
    /// Will be `None` in release mode.
    pub debug_label: Option<String>,
}

impl TapRegion {
    /// Creates a [`TapRegion`].
    ///
    /// The [`child`](Self::child) argument is required.
    pub fn new<K>(child: impl IntoWidget<K>) -> TapRegion {
        TapRegion {
            key: None,
            child: child.into_widget(),
            enabled: true,
            behavior: HitTestBehavior::DeferToChild,
            on_tap_outside: None,
            on_tap_inside: None,
            on_tap_up_outside: None,
            on_tap_up_inside: None,
            group_id: None,
            consume_outside_taps: false,
            debug_label: None,
        }
    }

    /// Dart `TapRegion(key:)`.
    pub fn key(mut self, key: KeyRef) -> TapRegion {
        self.key = Some(key);
        self
    }

    /// Dart `TapRegion(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> TapRegion {
        self.enabled = enabled;
        self
    }

    /// Dart `TapRegion(behavior:)`.
    pub fn behavior(mut self, behavior: HitTestBehavior) -> TapRegion {
        self.behavior = behavior;
        self
    }

    /// Dart `TapRegion(onTapOutside:)`.
    pub fn on_tap_outside(mut self, on_tap_outside: TapRegionCallback) -> TapRegion {
        self.on_tap_outside = Some(on_tap_outside);
        self
    }

    /// Dart `TapRegion(onTapInside:)`.
    pub fn on_tap_inside(mut self, on_tap_inside: TapRegionCallback) -> TapRegion {
        self.on_tap_inside = Some(on_tap_inside);
        self
    }

    /// Dart `TapRegion(onTapUpOutside:)`.
    pub fn on_tap_up_outside(mut self, on_tap_up_outside: TapRegionUpCallback) -> TapRegion {
        self.on_tap_up_outside = Some(on_tap_up_outside);
        self
    }

    /// Dart `TapRegion(onTapUpInside:)`.
    pub fn on_tap_up_inside(mut self, on_tap_up_inside: TapRegionUpCallback) -> TapRegion {
        self.on_tap_up_inside = Some(on_tap_up_inside);
        self
    }

    /// Dart `TapRegion(groupId:)`.
    pub fn group_id(mut self, group_id: impl Into<TapRegionGroupId>) -> TapRegion {
        self.group_id = Some(group_id.into());
        self
    }

    /// Dart `TapRegion(consumeOutsideTaps:)`.
    pub fn consume_outside_taps(mut self, consume_outside_taps: bool) -> TapRegion {
        self.consume_outside_taps = consume_outside_taps;
        self
    }

    /// Dart `TapRegion(debugLabel:)`.
    pub fn debug_label(mut self, debug_label: impl Into<String>) -> TapRegion {
        self.debug_label = if cfg!(debug_assertions) {
            Some(debug_label.into())
        } else {
            None
        };
        self
    }

    fn apply_to_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderTapRegion>,
    ) {
        let is_current = AnyModalRoute::is_current_of(app, context).unwrap_or(true);
        render_object.set_registry(app, RenderTapRegionSurface::maybe_of(app, context));
        render_object.set_enabled(app, self.enabled);
        render_object.set_consume_outside_taps(app, is_current && self.consume_outside_taps);
        render_object.set_behavior(app, self.behavior);
        render_object.set_group_id(app, self.group_id);
        render_object.set_on_tap_outside(
            app,
            if is_current {
                self.on_tap_outside.clone()
            } else {
                None
            },
        );
        render_object.set_on_tap_inside(app, self.on_tap_inside.clone());
        render_object.set_on_tap_up_outside(
            app,
            if is_current {
                self.on_tap_up_outside.clone()
            } else {
                None
            },
        );
        render_object.set_on_tap_up_inside(app, self.on_tap_up_inside.clone());
        if cfg!(debug_assertions) {
            render_object.set_debug_label(app, self.debug_label.clone());
        }
    }
}

impl RenderObjectWidget for TapRegion {
    type RenderObject = RenderTapRegion;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let is_current = AnyModalRoute::is_current_of(app, context).unwrap_or(true);
        let render_object = RenderTapRegion::new(
            app,
            RenderTapRegionSurface::maybe_of(app, context),
            self.enabled,
            is_current && self.consume_outside_taps,
            if is_current {
                self.on_tap_outside.clone()
            } else {
                None
            },
            self.on_tap_inside.clone(),
            if is_current {
                self.on_tap_up_outside.clone()
            } else {
                None
            },
            self.on_tap_up_inside.clone(),
            self.behavior,
            self.group_id,
            self.debug_label.clone(),
        );
        render_object.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderTapRegion>,
    ) {
        self.apply_to_render_object(app, context, render_object);
    }
}

impl SingleChildRenderObjectWidget for TapRegion {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

impl Debug for TapRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TapRegion")
            .field("key", &self.key)
            .field("enabled", &self.enabled)
            .field("behavior", &self.behavior)
            .field(
                "on_tap_outside",
                &self.on_tap_outside.as_ref().map(|_| ".."),
            )
            .field("on_tap_inside", &self.on_tap_inside.as_ref().map(|_| ".."))
            .field(
                "on_tap_up_outside",
                &self.on_tap_up_outside.as_ref().map(|_| ".."),
            )
            .field(
                "on_tap_up_inside",
                &self.on_tap_up_inside.as_ref().map(|_| ".."),
            )
            .field("group_id", &self.group_id)
            .field("consume_outside_taps", &self.consume_outside_taps)
            .field("debug_label", &self.debug_label)
            .field("child", &self.child)
            .finish()
    }
}

/// A render object that defines a region that can detect taps inside or outside
/// of itself and any group of regions it belongs to, without participating in
/// the [gesture
/// disambiguation](https://flutter.dev/to/gesture-disambiguation) system.
///
/// See [`TapRegion`].
pub struct RenderTapRegion {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    is_registered: bool,
    on_tap_outside: Option<TapRegionCallback>,
    on_tap_inside: Option<TapRegionCallback>,
    on_tap_up_outside: Option<TapRegionUpCallback>,
    on_tap_up_inside: Option<TapRegionUpCallback>,
    debug_label: Option<String>,
    enabled: bool,
    consume_outside_taps: bool,
    group_id: Option<TapRegionGroupId>,
    registry: Option<RenderHandle<RenderTapRegionSurface>>,
    behavior: HitTestBehavior,
}

impl RenderTapRegion {
    /// Creates a [`RenderTapRegion`].
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        app: &mut App,
        registry: Option<RenderHandle<RenderTapRegionSurface>>,
        enabled: bool,
        consume_outside_taps: bool,
        on_tap_outside: Option<TapRegionCallback>,
        on_tap_inside: Option<TapRegionCallback>,
        on_tap_up_outside: Option<TapRegionUpCallback>,
        on_tap_up_inside: Option<TapRegionUpCallback>,
        behavior: HitTestBehavior,
        group_id: Option<TapRegionGroupId>,
        debug_label: Option<String>,
    ) -> RenderHandle<RenderTapRegion> {
        let this = RenderHandle::new_box(
            app,
            RenderTapRegion {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                is_registered: false,
                on_tap_outside,
                on_tap_inside,
                on_tap_up_outside,
                on_tap_up_inside,
                debug_label: if cfg!(debug_assertions) {
                    debug_label
                } else {
                    None
                },
                enabled,
                consume_outside_taps,
                group_id,
                registry,
                behavior,
            },
        );
        this.set_child(app, None);
        this
    }

    /// A callback to be invoked when a tap down is detected outside of this
    /// [`RenderTapRegion`] and any other region with the same [`group_id`](Self::group_id), if any.
    pub fn on_tap_outside(self: RenderHandle<Self>, app: &App) -> Option<TapRegionCallback> {
        self.get(app).on_tap_outside.clone()
    }

    /// Sets [`on_tap_outside`](Self::on_tap_outside).
    pub fn set_on_tap_outside(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<TapRegionCallback>,
    ) {
        self.get_mut(app).on_tap_outside = value;
    }

    /// A callback to be invoked when a tap down is detected inside of this
    /// [`RenderTapRegion`], or any other tap region with the same [`group_id`](Self::group_id), if any.
    pub fn on_tap_inside(self: RenderHandle<Self>, app: &App) -> Option<TapRegionCallback> {
        self.get(app).on_tap_inside.clone()
    }

    /// Sets [`on_tap_inside`](Self::on_tap_inside).
    pub fn set_on_tap_inside(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<TapRegionCallback>,
    ) {
        self.get_mut(app).on_tap_inside = value;
    }

    /// A callback to be invoked when a tap up is detected outside of this
    /// [`RenderTapRegion`] and any other region with the same [`group_id`](Self::group_id), if any.
    pub fn on_tap_up_outside(self: RenderHandle<Self>, app: &App) -> Option<TapRegionUpCallback> {
        self.get(app).on_tap_up_outside.clone()
    }

    /// Sets [`on_tap_up_outside`](Self::on_tap_up_outside).
    pub fn set_on_tap_up_outside(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<TapRegionUpCallback>,
    ) {
        self.get_mut(app).on_tap_up_outside = value;
    }

    /// A callback to be invoked when a tap up is detected inside of this
    /// [`RenderTapRegion`], or any other tap region with the same [`group_id`](Self::group_id), if any.
    pub fn on_tap_up_inside(self: RenderHandle<Self>, app: &App) -> Option<TapRegionUpCallback> {
        self.get(app).on_tap_up_inside.clone()
    }

    /// Sets [`on_tap_up_inside`](Self::on_tap_up_inside).
    pub fn set_on_tap_up_inside(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<TapRegionUpCallback>,
    ) {
        self.get_mut(app).on_tap_up_inside = value;
    }

    /// A label used in debug builds. Will be `None` in release builds.
    pub fn debug_label(self: RenderHandle<Self>, app: &App) -> Option<String> {
        self.get(app).debug_label.clone()
    }

    /// Sets [`debug_label`](Self::debug_label).
    pub fn set_debug_label(self: RenderHandle<Self>, app: &mut App, value: Option<String>) {
        self.get_mut(app).debug_label = if cfg!(debug_assertions) { value } else { None };
    }

    /// Whether or not this region should participate in the composite region.
    pub fn enabled(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).enabled
    }

    /// Sets [`enabled`](Self::enabled).
    pub fn set_enabled(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).enabled != value {
            self.get_mut(app).enabled = value;
            self.mark_needs_layout(app);
        }
    }

    /// Whether or not the tap event that triggers a call to [`on_tap_outside`](Self::on_tap_outside)
    /// or [`on_tap_up_outside`](Self::on_tap_up_outside) will continue on to participate in the gesture arena.
    pub fn consume_outside_taps(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).consume_outside_taps
    }

    /// Sets [`consume_outside_taps`](Self::consume_outside_taps).
    pub fn set_consume_outside_taps(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).consume_outside_taps != value {
            self.get_mut(app).consume_outside_taps = value;
            self.mark_needs_layout(app);
        }
    }

    /// An optional group ID that groups [`RenderTapRegion`]s together so that they
    /// operate as one region.
    pub fn group_id(self: RenderHandle<Self>, app: &App) -> Option<TapRegionGroupId> {
        self.get(app).group_id
    }

    /// Sets [`group_id`](Self::group_id).
    pub fn set_group_id(self: RenderHandle<Self>, app: &mut App, value: Option<TapRegionGroupId>) {
        if self.get(app).group_id != value {
            // If the group changes, we need to unregister and re-register under the
            // new group. The re-registration happens automatically in layout().
            self.unregister_if_registered(app);
            self.get_mut(app).group_id = value;
            self.mark_needs_layout(app);
        }
    }

    /// The registry that this [`RenderTapRegion`] should register with.
    ///
    /// If the registry is `None`, then this region will not be registered
    /// anywhere, and will not do any tap detection.
    ///
    /// A [`RenderTapRegionSurface`] is a [`TapRegionRegistry`].
    pub fn registry(
        self: RenderHandle<Self>,
        app: &App,
    ) -> Option<RenderHandle<RenderTapRegionSurface>> {
        self.get(app).registry
    }

    /// Sets [`registry`](Self::registry).
    pub fn set_registry(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<RenderHandle<RenderTapRegionSurface>>,
    ) {
        if self.get(app).registry != value {
            self.unregister_if_registered(app);
            self.get_mut(app).registry = value;
            self.mark_needs_layout(app);
        }
    }

    /// Sets [`behavior`](RenderProxyBoxWithHitTestBehavior::behavior).
    pub fn set_behavior(self: RenderHandle<Self>, app: &mut App, value: HitTestBehavior) {
        self.get_mut(app).behavior = value;
    }

    fn unregister_if_registered(self: RenderHandle<Self>, app: &mut App) {
        if !self.get(app).is_registered {
            return;
        }
        let registry = self
            .get(app)
            .registry
            .expect("a registered region has a registry");
        registry.unregister_tap_region(app, self);
        self.get_mut(app).is_registered = false;
    }

    fn sync_registration(self: RenderHandle<Self>, app: &mut App) {
        let Some(registry) = self.get(app).registry else {
            return;
        };
        if self.get(app).is_registered {
            registry.unregister_tap_region(app, self);
        }
        let should_be_registered = self.get(app).enabled && self.get(app).registry.is_some();
        if should_be_registered {
            registry.register_tap_region(app, self);
        }
        self.get_mut(app).is_registered = should_be_registered;
    }
}

impl RenderObjectWithChildMixin for RenderTapRegion {
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

impl RenderProxyBoxMixin for RenderTapRegion {}

impl RenderProxyBoxWithHitTestBehavior for RenderTapRegion {
    fn behavior(self: RenderHandle<Self>, app: &App) -> HitTestBehavior {
        self.get(app).behavior
    }
}

impl RenderObject for RenderTapRegion {
    inset_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
        self.sync_registration(app);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        self.unregister_if_registered(app);
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
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

impl RenderBox for RenderTapRegion {
    inset_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxWithHitTestBehavior::hit_test(self, app, result, position)
    }

    fn hit_test_self(self: RenderHandle<Self>, app: &App, position: Offset) -> bool {
        RenderProxyBoxWithHitTestBehavior::hit_test_self(self, app, position)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// A [`TapRegion`] that adds its children to the tap region group for widgets
/// based on the [`EditableText`] text editing widget, such as `TextField` and
/// `CupertinoTextField`.
///
/// Widgets that are wrapped with a [`TextFieldTapRegion`] are considered to be
/// part of a text field for purposes of unfocus behavior. So, when the user
/// taps on them, the currently focused text field won't be unfocused by
/// default.
///
/// See also:
///
///  * [`TapRegion`], the widget that this widget uses to add widgets to the group
///    of text fields.
///
/// Dart's `class TextFieldTapRegion extends TapRegion`: a distinct widget type
/// whose behaviour is [`TapRegion`]'s with [`group_id`](TapRegion::group_id) defaulting to
/// [`TypeId::of::<EditableText>()`].
pub struct TextFieldTapRegion {
    /// See [`TapRegion::key`].
    pub key: Option<KeyRef>,
    /// See [`TapRegion::child`].
    pub child: WidgetRef,
    /// See [`TapRegion::enabled`].
    pub enabled: bool,
    /// See [`TapRegion::behavior`].
    pub behavior: HitTestBehavior,
    /// See [`TapRegion::on_tap_outside`].
    pub on_tap_outside: Option<TapRegionCallback>,
    /// See [`TapRegion::on_tap_inside`].
    pub on_tap_inside: Option<TapRegionCallback>,
    /// See [`TapRegion::on_tap_up_outside`].
    pub on_tap_up_outside: Option<TapRegionUpCallback>,
    /// See [`TapRegion::on_tap_up_inside`].
    pub on_tap_up_inside: Option<TapRegionUpCallback>,
    /// See [`TapRegion::group_id`]. Defaults to [`TypeId::of::<EditableText>()`].
    pub group_id: Option<TapRegionGroupId>,
    /// See [`TapRegion::consume_outside_taps`].
    pub consume_outside_taps: bool,
    /// See [`TapRegion::debug_label`].
    pub debug_label: Option<String>,
}

impl TextFieldTapRegion {
    /// Creates a [`TextFieldTapRegion`].
    ///
    /// The [`child`](Self::child) field is required.
    pub fn new<K>(child: impl IntoWidget<K>) -> TextFieldTapRegion {
        TextFieldTapRegion {
            key: None,
            child: child.into_widget(),
            enabled: true,
            behavior: HitTestBehavior::DeferToChild,
            on_tap_outside: None,
            on_tap_inside: None,
            on_tap_up_outside: None,
            on_tap_up_inside: None,
            group_id: Some(TapRegionGroupId::Type(TypeId::of::<EditableText>())),
            consume_outside_taps: false,
            debug_label: None,
        }
    }

    /// Dart `TextFieldTapRegion(key:)`.
    pub fn key(mut self, key: KeyRef) -> TextFieldTapRegion {
        self.key = Some(key);
        self
    }

    /// Dart `TextFieldTapRegion(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> TextFieldTapRegion {
        self.enabled = enabled;
        self
    }

    /// Dart `TextFieldTapRegion(behavior:)`.
    pub fn behavior(mut self, behavior: HitTestBehavior) -> TextFieldTapRegion {
        self.behavior = behavior;
        self
    }

    /// Dart `TextFieldTapRegion(onTapOutside:)`.
    pub fn on_tap_outside(mut self, on_tap_outside: TapRegionCallback) -> TextFieldTapRegion {
        self.on_tap_outside = Some(on_tap_outside);
        self
    }

    /// Dart `TextFieldTapRegion(onTapInside:)`.
    pub fn on_tap_inside(mut self, on_tap_inside: TapRegionCallback) -> TextFieldTapRegion {
        self.on_tap_inside = Some(on_tap_inside);
        self
    }

    /// Dart `TextFieldTapRegion(onTapUpOutside:)`.
    pub fn on_tap_up_outside(
        mut self,
        on_tap_up_outside: TapRegionUpCallback,
    ) -> TextFieldTapRegion {
        self.on_tap_up_outside = Some(on_tap_up_outside);
        self
    }

    /// Dart `TextFieldTapRegion(onTapUpInside:)`.
    pub fn on_tap_up_inside(mut self, on_tap_up_inside: TapRegionUpCallback) -> TextFieldTapRegion {
        self.on_tap_up_inside = Some(on_tap_up_inside);
        self
    }

    /// Dart `TextFieldTapRegion(groupId:)`.
    pub fn group_id(mut self, group_id: impl Into<TapRegionGroupId>) -> TextFieldTapRegion {
        self.group_id = Some(group_id.into());
        self
    }

    /// Dart `TextFieldTapRegion(consumeOutsideTaps:)`.
    pub fn consume_outside_taps(mut self, consume_outside_taps: bool) -> TextFieldTapRegion {
        self.consume_outside_taps = consume_outside_taps;
        self
    }

    /// Dart `TextFieldTapRegion(debugLabel:)`.
    pub fn debug_label(mut self, debug_label: impl Into<String>) -> TextFieldTapRegion {
        self.debug_label = if cfg!(debug_assertions) {
            Some(debug_label.into())
        } else {
            None
        };
        self
    }

    fn as_tap_region(&self) -> TapRegion {
        TapRegion {
            key: self.key.clone(),
            child: self.child.clone(),
            enabled: self.enabled,
            behavior: self.behavior,
            on_tap_outside: self.on_tap_outside.clone(),
            on_tap_inside: self.on_tap_inside.clone(),
            on_tap_up_outside: self.on_tap_up_outside.clone(),
            on_tap_up_inside: self.on_tap_up_inside.clone(),
            group_id: self.group_id,
            consume_outside_taps: self.consume_outside_taps,
            debug_label: self.debug_label.clone(),
        }
    }
}

impl RenderObjectWidget for TextFieldTapRegion {
    type RenderObject = RenderTapRegion;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        self.as_tap_region().create_render_object(app, context)
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderTapRegion>,
    ) {
        self.as_tap_region()
            .apply_to_render_object(app, context, render_object);
    }
}

impl SingleChildRenderObjectWidget for TextFieldTapRegion {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

impl Debug for TextFieldTapRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_tap_region().fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use inset_embedder::Offset;
    use inset_foundation::AppCell;
    use inset_gestures::{HitTestResult, PointerDownEvent, PointerEvent};
    use inset_rendering::{
        BoxHitTestResult, HitTestBehavior, RenderBox, RenderObjectWithChildMixin,
    };

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;

    fn tree(child: WidgetRef) -> WidgetRef {
        TapRegionSurface::new(child).into_widget()
    }

    #[test]
    fn registers_on_layout_and_unregisters_when_disabled() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            tree(
                TapRegion::new(SizedBox::expand())
                    .behavior(HitTestBehavior::Opaque)
                    .into_widget(),
            ),
        );
        harness.pump(&mut app);

        let surface = harness
            .render_root(&app)
            .child(&app)
            .expect("the surface")
            .as_object()
            .downcast::<RenderTapRegionSurface>(&app)
            .expect("RenderTapRegionSurface");
        let region = surface
            .child(&app)
            .expect("the region")
            .as_object()
            .downcast::<RenderTapRegion>(&app)
            .expect("RenderTapRegion");
        assert!(surface.get(&app).registered_regions.contains(&region));
        assert!(region.get(&app).is_registered);

        harness.set_child(
            &mut app,
            tree(
                TapRegion::new(SizedBox::expand())
                    .enabled(false)
                    .behavior(HitTestBehavior::Opaque)
                    .into_widget(),
            ),
        );
        harness.pump(&mut app);

        let surface = harness
            .render_root(&app)
            .child(&app)
            .expect("the surface")
            .as_object()
            .downcast::<RenderTapRegionSurface>(&app)
            .expect("RenderTapRegionSurface");
        assert!(surface.get(&app).registered_regions.is_empty());
    }

    #[test]
    fn hit_test_classifies_inside_and_outside() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let inside = Rc::new(Cell::new(0u32));
        let outside = Rc::new(Cell::new(0u32));
        let on_inside = {
            let inside = Rc::clone(&inside);
            Rc::new(move |_app: &mut App, _event: PointerDownEvent| {
                inside.set(inside.get() + 1);
            }) as TapRegionCallback
        };
        let on_outside = {
            let outside = Rc::clone(&outside);
            Rc::new(move |_app: &mut App, _event: PointerDownEvent| {
                outside.set(outside.get() + 1);
            }) as TapRegionCallback
        };

        let harness = Harness::mount(
            &mut app,
            tree(
                TapRegion::new(
                    SizedBox {
                        width: Some(50.0),
                        height: Some(50.0),
                        ..Default::default()
                    }
                    .into_widget(),
                )
                .behavior(HitTestBehavior::Opaque)
                .on_tap_inside(on_inside)
                .on_tap_outside(on_outside)
                .into_widget(),
            ),
        );
        harness.pump(&mut app);

        let surface = harness
            .render_root(&app)
            .child(&app)
            .expect("the surface")
            .as_object()
            .downcast::<RenderTapRegionSurface>(&app)
            .expect("RenderTapRegionSurface");

        let mut result = HitTestResult::new();
        {
            let mut box_result = BoxHitTestResult::wrap(&mut result);
            assert!(RenderBox::hit_test(
                surface,
                &mut app,
                &mut box_result,
                Offset::new(10.0, 10.0)
            ));
        }
        let event = PointerEvent::Down(PointerDownEvent {
            position: Offset::new(10.0, 10.0),
            ..PointerDownEvent::default()
        });
        for entry in result.path() {
            entry.target().handle_event(&mut app, &event, entry);
        }
        assert_eq!(inside.get(), 1);
        assert_eq!(outside.get(), 0);
    }

    #[test]
    fn text_field_tap_region_defaults_group_id_to_editable_text() {
        let region = TextFieldTapRegion::new(SizedBox::expand());
        assert_eq!(
            region.group_id,
            Some(TapRegionGroupId::Type(TypeId::of::<EditableText>()))
        );
    }
}
