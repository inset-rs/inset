//! Flutter counterpart: `rendering/viewport_offset.dart`.

use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::time::Duration;

use reveal_animation::{Curve, Curves};
use reveal_foundation::{
    App, ChangeNotifier, ChangeNotifierData, CompleterFuture, Handle, HandleId, Listenable,
    ListenableObject, Listener,
};

/// The direction of a scroll, relative to the positive scroll offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollDirection {
    /// No scrolling is underway.
    Idle,

    /// Scrolling is happening in the negative scroll offset direction.
    Forward,

    /// Scrolling is happening in the positive scroll offset direction.
    Reverse,
}

/// Returns the opposite of the given [`ScrollDirection`].
///
/// [`ScrollDirection::Idle`] is unchanged.
pub fn flip_scroll_direction(direction: ScrollDirection) -> ScrollDirection {
    match direction {
        ScrollDirection::Idle => ScrollDirection::Idle,
        ScrollDirection::Forward => ScrollDirection::Reverse,
        ScrollDirection::Reverse => ScrollDirection::Forward,
    }
}

/// Which part of the content inside the viewport should be visible.
///
/// The [`pixels`](Self::pixels) value determines the scroll offset that the viewport uses to
/// select which part of its content to display. As the user scrolls the viewport, this value
/// changes, which changes the content that is displayed.
///
/// This object is a [`Listenable`] that notifies its listeners when
/// [`pixels`](Self::pixels) changes.
///
/// See also:
///
///  * [`crate::RenderViewportBase`], which is a render object that uses viewport offsets.
pub trait ViewportOffset: ChangeNotifier + Sized {
    /// The number of pixels to offset the children in the opposite of the axis direction.
    ///
    /// For example, if the axis direction is down, then the pixel value represents the number
    /// of logical pixels to move the children _up_ the screen. Similarly, if the axis
    /// direction is left, then the pixels value represents the number of logical pixels to
    /// move the children to _right_.
    ///
    /// This object notifies its listeners when this value changes (except when the value
    /// changes due to [`correct_by`](Self::correct_by)).
    fn pixels(self: Handle<Self>, app: &App) -> f64;

    /// Whether the [`pixels`](Self::pixels) property is available.
    fn has_pixels(self: Handle<Self>, app: &App) -> bool;

    /// Called when the viewport's extents are established.
    ///
    /// The argument is the dimension of the [`crate::RenderViewport`] in the main axis (e.g.
    /// the height, for a vertical viewport).
    ///
    /// This may be called redundantly, with the same value, each frame. This is called during
    /// layout for the viewport. If the viewport is configured to shrink-wrap its contents, it
    /// may be called several times, since the layout is repeated each time the scroll offset
    /// is corrected.
    ///
    /// If this is called, it is called before
    /// [`apply_content_dimensions`](Self::apply_content_dimensions), which will be called soon
    /// afterwards in the same layout phase.
    ///
    /// If applying the viewport dimensions changes the scroll offset, return `false`.
    /// Otherwise, return `true`. If you return `false`, the viewport will be laid out again
    /// with the new scroll offset. This is expensive.
    fn apply_viewport_dimension(self: Handle<Self>, app: &mut App, viewport_dimension: f64)
    -> bool;

    /// Called when the viewport's content extents are established.
    ///
    /// The arguments are the minimum and maximum scroll extents respectively. The minimum will
    /// be equal to or less than the maximum. In the case of slivers, the minimum will be equal
    /// to or less than zero, the maximum will be equal to or greater than zero.
    ///
    /// The maximum scroll extent has the viewport dimension subtracted from it. For instance,
    /// if there is 100.0 pixels of scrollable content, and the viewport is 80.0 pixels high,
    /// then the minimum scroll extent will typically be 0.0 and the maximum scroll extent will
    /// typically be 20.0, because there's only 20.0 pixels of actual scroll slack.
    ///
    /// If applying the content dimensions changes the scroll offset, return `false`.
    /// Otherwise, return `true`. If you return `false`, the viewport will be laid out again
    /// with the new scroll offset. This is expensive.
    ///
    /// This is called at least once each time the viewport is laid out, even if the values
    /// have not changed. It may be called many times if the scroll offset is corrected (if
    /// this returns `false`). This is always called after
    /// [`apply_viewport_dimension`](Self::apply_viewport_dimension), if that method is called.
    fn apply_content_dimensions(
        self: Handle<Self>,
        app: &mut App,
        min_scroll_extent: f64,
        max_scroll_extent: f64,
    ) -> bool;

    /// Apply a layout-time correction to the scroll offset.
    ///
    /// This method should change the [`pixels`](Self::pixels) value by `correction`, but
    /// without notifying listeners. It is called during layout by the viewport, before
    /// [`apply_content_dimensions`](Self::apply_content_dimensions). After this method is
    /// called, the layout will be recomputed and that may result in this method being called
    /// again, though this should be very rare.
    ///
    /// See also:
    ///
    ///  * [`jump_to`](Self::jump_to), for also changing the scroll position when not in
    ///    layout. It applies the change immediately and notifies its listeners.
    fn correct_by(self: Handle<Self>, app: &mut App, correction: f64);

    /// Jumps [`pixels`](Self::pixels) from its current value to the given value, without
    /// animation, and without checking if the new value is in range.
    ///
    /// See also:
    ///
    ///  * [`correct_by`](Self::correct_by), for changing the current offset in the middle of
    ///    layout and that defers the notification of its listeners until after layout.
    fn jump_to(self: Handle<Self>, app: &mut App, pixels: f64);

    /// Animates [`pixels`](Self::pixels) from its current value to the given value.
    ///
    /// The returned future will complete when the animation ends, whether it completed
    /// successfully or whether it was interrupted prematurely.
    ///
    /// The duration must not be zero. To jump to a particular value without an animation, use
    /// [`jump_to`](Self::jump_to).
    fn animate_to(
        self: Handle<Self>,
        app: &mut App,
        to: f64,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) -> CompleterFuture<()>;

    /// Calls [`jump_to`](Self::jump_to) if duration is `None` or zero, otherwise
    /// [`animate_to`](Self::animate_to) is called.
    ///
    /// If [`animate_to`](Self::animate_to) is called then `curve` defaults to
    /// [`Curves::ease`]. The `clamp` parameter is ignored by this stub implementation but
    /// implementors like `ScrollPosition` handle it by adjusting `to` to prevent over or
    /// underscroll.
    fn move_to(
        self: Handle<Self>,
        app: &mut App,
        to: f64,
        duration: Option<Duration>,
        curve: Option<Rc<dyn Curve>>,
        clamp: Option<bool>,
    ) -> CompleterFuture<()> {
        ViewportOffsetBase::move_to(self, app, to, duration, curve, clamp)
    }

    /// The direction in which the user is trying to change [`pixels`](Self::pixels), relative
    /// to the viewport's `axis_direction`.
    ///
    /// If the _user_ is not scrolling, this will return [`ScrollDirection::Idle`] even if
    /// there is (for example) a scroll activity currently animating the position.
    ///
    /// This is exposed in [`crate::SliverConstraints::user_scroll_direction`], which is used
    /// by some slivers to determine how to react to a change in scroll offset. For example,
    /// [`crate::RenderSliverFloatingPersistentHeader`] will only expand a floating app bar
    /// when the user scroll direction is in the positive scroll offset direction.
    fn user_scroll_direction(self: Handle<Self>, app: &App) -> ScrollDirection;

    /// Whether a viewport is allowed to change [`pixels`](Self::pixels) implicitly to respond
    /// to a call to [`crate::RenderObject::show_on_screen`].
    ///
    /// `show_on_screen` is, for example, used to bring a text field fully on screen after it
    /// has received focus. This property controls whether the viewport associated with this
    /// offset is allowed to change the offset's [`pixels`](Self::pixels) value to fulfill such
    /// a request.
    fn allow_implicit_scrolling(self: Handle<Self>, app: &App) -> bool;

    /// Add additional information to the given `description` for use by Dart's `toString`.
    ///
    /// An implementor that adds its own lines calls this one where Dart writes
    /// `super.debugFillDescription(description)`.
    fn debug_fill_description(self: Handle<Self>, app: &App, description: &mut Vec<String>) {
        if self.has_pixels(app) {
            description.push(format!("offset: {:.1}", self.pixels(app)));
        }
    }

    /// The type-erased `ViewportOffset` handle. Free: the vtable is a `const`, and the id is
    /// copied.
    fn as_viewport_offset(self: Handle<Self>) -> AnyViewportOffset {
        AnyViewportOffset {
            id: self.id(),
            vtable: const { &ViewportOffsetVTable::of::<Self>() },
        }
    }
}

/// The shared bodies of `ViewportOffset`: call one where Dart writes `super.…`.
pub struct ViewportOffsetBase;

impl ViewportOffsetBase {
    /// See [`ViewportOffset::move_to`].
    pub fn move_to<T: ViewportOffset>(
        this: Handle<T>,
        app: &mut App,
        to: f64,
        duration: Option<Duration>,
        curve: Option<Rc<dyn Curve>>,
        clamp: Option<bool>,
    ) -> CompleterFuture<()> {
        let _ = clamp;
        let Some(duration) = duration.filter(|duration| !duration.is_zero()) else {
            this.jump_to(app, to);
            return CompleterFuture::ready(());
        };
        this.animate_to(app, to, duration, curve.unwrap_or(Curves::ease()))
    }
}

/// The dispatch signature of [`ViewportOffset::animate_to`].
type AnimateToFn = fn(&mut App, HandleId, f64, Duration, Rc<dyn Curve>) -> CompleterFuture<()>;

/// The dispatch signature of [`ViewportOffset::move_to`].
type MoveToFn = fn(
    &mut App,
    HandleId,
    f64,
    Option<Duration>,
    Option<Rc<dyn Curve>>,
    Option<bool>,
) -> CompleterFuture<()>;

/// The vtable of an erased [`AnyViewportOffset`]: one `&'static` table per concrete
/// [`ViewportOffset`] type.
struct ViewportOffsetVTable {
    add_listener: fn(&mut App, HandleId, Listener),
    remove_listener: fn(&mut App, HandleId, &Listener),
    pixels: fn(&App, HandleId) -> f64,
    has_pixels: fn(&App, HandleId) -> bool,
    apply_viewport_dimension: fn(&mut App, HandleId, f64) -> bool,
    apply_content_dimensions: fn(&mut App, HandleId, f64, f64) -> bool,
    correct_by: fn(&mut App, HandleId, f64),
    jump_to: fn(&mut App, HandleId, f64),
    animate_to: AnimateToFn,
    move_to: MoveToFn,
    user_scroll_direction: fn(&App, HandleId) -> ScrollDirection,
    allow_implicit_scrolling: fn(&App, HandleId) -> bool,
}

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<T: 'static>(id: HandleId) -> Handle<T> {
    Handle::from_id(id)
}

impl ViewportOffsetVTable {
    const fn of<T: ViewportOffset>() -> ViewportOffsetVTable {
        ViewportOffsetVTable {
            add_listener: |app, id, listener| {
                ListenableObject::add_listener(resolve::<T>(id), app, listener)
            },
            remove_listener: |app, id, listener| {
                ListenableObject::remove_listener(resolve::<T>(id), app, listener)
            },
            pixels: |app, id| T::pixels(resolve(id), app),
            has_pixels: |app, id| T::has_pixels(resolve(id), app),
            apply_viewport_dimension: |app, id, dimension| {
                T::apply_viewport_dimension(resolve(id), app, dimension)
            },
            apply_content_dimensions: |app, id, min, max| {
                T::apply_content_dimensions(resolve(id), app, min, max)
            },
            correct_by: |app, id, correction| T::correct_by(resolve(id), app, correction),
            jump_to: |app, id, pixels| T::jump_to(resolve(id), app, pixels),
            animate_to: |app, id, to, duration, curve| {
                T::animate_to(resolve(id), app, to, duration, curve)
            },
            move_to: |app, id, to, duration, curve, clamp| {
                T::move_to(resolve(id), app, to, duration, curve, clamp)
            },
            user_scroll_direction: |app, id| T::user_scroll_direction(resolve(id), app),
            allow_implicit_scrolling: |app, id| T::allow_implicit_scrolling(resolve(id), app),
        }
    }
}

/// Erased [`ViewportOffset`]: what a field or parameter Dart types as `ViewportOffset` becomes.
///
/// Equality is Dart's `==` on an object reference.
#[derive(Clone, Copy)]
pub struct AnyViewportOffset {
    id: HandleId,
    vtable: &'static ViewportOffsetVTable,
}

impl AnyViewportOffset {
    /// See [`ViewportOffset::pixels`].
    pub fn pixels(self, app: &App) -> f64 {
        (self.vtable.pixels)(app, self.id)
    }

    /// See [`ViewportOffset::has_pixels`].
    pub fn has_pixels(self, app: &App) -> bool {
        (self.vtable.has_pixels)(app, self.id)
    }

    /// See [`ViewportOffset::apply_viewport_dimension`].
    pub fn apply_viewport_dimension(self, app: &mut App, viewport_dimension: f64) -> bool {
        (self.vtable.apply_viewport_dimension)(app, self.id, viewport_dimension)
    }

    /// See [`ViewportOffset::apply_content_dimensions`].
    pub fn apply_content_dimensions(
        self,
        app: &mut App,
        min_scroll_extent: f64,
        max_scroll_extent: f64,
    ) -> bool {
        (self.vtable.apply_content_dimensions)(app, self.id, min_scroll_extent, max_scroll_extent)
    }

    /// See [`ViewportOffset::correct_by`].
    pub fn correct_by(self, app: &mut App, correction: f64) {
        (self.vtable.correct_by)(app, self.id, correction)
    }

    /// See [`ViewportOffset::jump_to`].
    pub fn jump_to(self, app: &mut App, pixels: f64) {
        (self.vtable.jump_to)(app, self.id, pixels)
    }

    /// See [`ViewportOffset::animate_to`].
    pub fn animate_to(
        self,
        app: &mut App,
        to: f64,
        duration: Duration,
        curve: Rc<dyn Curve>,
    ) -> CompleterFuture<()> {
        (self.vtable.animate_to)(app, self.id, to, duration, curve)
    }

    /// See [`ViewportOffset::move_to`].
    pub fn move_to(
        self,
        app: &mut App,
        to: f64,
        duration: Option<Duration>,
        curve: Option<Rc<dyn Curve>>,
        clamp: Option<bool>,
    ) -> CompleterFuture<()> {
        (self.vtable.move_to)(app, self.id, to, duration, curve, clamp)
    }

    /// See [`ViewportOffset::user_scroll_direction`].
    pub fn user_scroll_direction(self, app: &App) -> ScrollDirection {
        (self.vtable.user_scroll_direction)(app, self.id)
    }

    /// See [`ViewportOffset::allow_implicit_scrolling`].
    pub fn allow_implicit_scrolling(self, app: &App) -> bool {
        (self.vtable.allow_implicit_scrolling)(app, self.id)
    }
}

impl PartialEq for AnyViewportOffset {
    fn eq(&self, other: &AnyViewportOffset) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyViewportOffset {}

impl Hash for AnyViewportOffset {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Debug for AnyViewportOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnyViewportOffset({:?})", self.id)
    }
}

impl Listenable for AnyViewportOffset {
    fn add_listener(&self, app: &mut App, listener: Listener) {
        (self.vtable.add_listener)(app, self.id, listener)
    }

    fn remove_listener(&self, app: &mut App, listener: &Listener) {
        (self.vtable.remove_listener)(app, self.id, listener)
    }
}

/// A [`ViewportOffset`] whose [`pixels`](ViewportOffset::pixels) value does not change unless
/// the viewport issues a correction.
///
/// Flutter's private `_FixedViewportOffset`, reached through the `ViewportOffset.fixed` and
/// `ViewportOffset.zero` factories.
#[derive(Debug, Default)]
pub struct FixedViewportOffset {
    change_notifier: ChangeNotifierData,
    pixels: f64,
}

impl FixedViewportOffset {
    /// Creates a viewport offset with the given [`pixels`](ViewportOffset::pixels) value.
    ///
    /// Dart's `ViewportOffset.fixed`.
    pub fn new(app: &mut App, value: f64) -> Handle<FixedViewportOffset> {
        app.create(FixedViewportOffset {
            change_notifier: ChangeNotifierData::new(),
            pixels: value,
        })
    }

    /// Creates a viewport offset with a [`pixels`](ViewportOffset::pixels) value of 0.0.
    ///
    /// Dart's `ViewportOffset.zero`.
    pub fn zero(app: &mut App) -> Handle<FixedViewportOffset> {
        FixedViewportOffset::new(app, 0.0)
    }
}

impl ChangeNotifier for FixedViewportOffset {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl ViewportOffset for FixedViewportOffset {
    fn pixels(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).pixels
    }

    fn has_pixels(self: Handle<Self>, _app: &App) -> bool {
        true
    }

    fn apply_viewport_dimension(
        self: Handle<Self>,
        _app: &mut App,
        _viewport_dimension: f64,
    ) -> bool {
        true
    }

    fn apply_content_dimensions(
        self: Handle<Self>,
        _app: &mut App,
        _min_scroll_extent: f64,
        _max_scroll_extent: f64,
    ) -> bool {
        true
    }

    fn correct_by(self: Handle<Self>, app: &mut App, correction: f64) {
        app.get_mut(self).pixels += correction;
    }

    fn jump_to(self: Handle<Self>, _app: &mut App, _pixels: f64) {
        // Do nothing, viewport is fixed.
    }

    fn animate_to(
        self: Handle<Self>,
        _app: &mut App,
        _to: f64,
        _duration: Duration,
        _curve: Rc<dyn Curve>,
    ) -> CompleterFuture<()> {
        CompleterFuture::ready(())
    }

    fn user_scroll_direction(self: Handle<Self>, _app: &App) -> ScrollDirection {
        ScrollDirection::Idle
    }

    fn allow_implicit_scrolling(self: Handle<Self>, _app: &App) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reveal_foundation::AppCell;

    #[test]
    fn a_fixed_offset_only_moves_by_correction() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let offset = FixedViewportOffset::new(&mut app, 12.0);
        assert_eq!(offset.pixels(&app), 12.0);
        assert!(offset.has_pixels(&app));
        offset.jump_to(&mut app, 40.0);
        assert_eq!(offset.pixels(&app), 12.0);
        offset.correct_by(&mut app, -5.0);
        assert_eq!(offset.pixels(&app), 7.0);
        assert_eq!(offset.user_scroll_direction(&app), ScrollDirection::Idle);
        assert!(!offset.allow_implicit_scrolling(&app));
    }

    #[test]
    fn the_erased_edge_dispatches_to_the_concrete_offset() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let offset = FixedViewportOffset::zero(&mut app).as_viewport_offset();
        assert_eq!(offset.pixels(&app), 0.0);
        assert!(offset.apply_viewport_dimension(&mut app, 100.0));
        assert!(offset.apply_content_dimensions(&mut app, 0.0, 200.0));
        let moved = offset.move_to(&mut app, 30.0, None, None, None);
        assert_eq!(offset.pixels(&app), 0.0);
        assert!(
            moved.is_completed(),
            "a jump hands back an already completed future"
        );
        let animated = offset.animate_to(&mut app, 30.0, Duration::from_millis(10), Curves::ease());
        assert!(
            animated.is_completed(),
            "a fixed offset's animation is over before it starts"
        );
    }
}
