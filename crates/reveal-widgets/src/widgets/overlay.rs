//! Flutter counterpart: `widgets/overlay.dart`.

use std::any::{Any, TypeId};
use std::cell::Cell;
use std::fmt::{self, Debug, Display};
use std::rc::Rc;

use reveal_embedder::{Clip, Matrix4, Offset, Size, TextDirection};
use reveal_foundation::{App, Handle, ListenableObject, Listener, ValueNotifier};
use reveal_painting::{Alignment, AlignmentGeometry};
use reveal_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, Constraints,
    ContainerBoxParentData, ContainerParentData, ContainerParentDataMixin,
    ContainerRenderObjectData, ContainerRenderObjectMixin, PaintingContext, ParentData,
    PipelineOwner, RenderBox, RenderBoxData, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin, RenderObjectWithLayoutCallbackData,
    RenderObjectWithLayoutCallbackMixin, RenderProxyBoxMixin, StackParentData,
};
use reveal_scheduler::{
    FrameCallback, SchedulerBinding, SchedulerPhase, Ticker, TickerCallback, TickerProviderObject,
};

use crate::framework::{
    AnyElement, BuildContext, BuildScope, BuildScopeCallback, Element, ElementData, GlobalKey,
    InheritedWidget, IntoWidget, KeyRef, MultiChildRenderObjectElementBase,
    MultiChildRenderObjectElementData, MultiChildRenderObjectWidget, RenderObjectElement,
    RenderObjectElementData, RenderObjectElementWidget, RenderObjectWidget,
    SingleChildRenderObjectWidget, Slot, State, StateData, StatefulWidget, Widget, WidgetKind,
    WidgetRef, downcast_widget,
};
use crate::widgets::basic::{Builder, Directionality, WidgetBuilder};
use crate::widgets::media_query::MediaQuery;
use crate::widgets::ticker_provider::{
    TickerMode, TickerProviderStateMixin, TickerProviderStateMixinData,
};

/// The signature of the widget builder callback used in
/// [`OverlayPortal::overlay_child_layout_builder`].
pub type OverlayChildLayoutBuilder =
    Rc<dyn Fn(&mut App, BuildContext, OverlayChildLayoutInfo) -> WidgetRef>;

/// The additional layout information available to the
/// [`OverlayPortal::overlay_child_layout_builder`] callback.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlayChildLayoutInfo {
    child_size: Size,
    child_paint_transform: Matrix4,
    overlay_size: Size,
}

impl OverlayChildLayoutInfo {
    /// The size of [`OverlayPortal::child`] in its own coordinates.
    pub fn child_size(&self) -> Size {
        self.child_size
    }

    /// The paint transform of [`OverlayPortal::child`], in the target [`Overlay`]'s
    /// coordinates.
    pub fn child_paint_transform(&self) -> Matrix4 {
        self.child_paint_transform
    }

    /// The size of the target [`Overlay`] in its own coordinates.
    pub fn overlay_size(&self) -> Size {
        self.overlay_size
    }
}

// * OverlayEntry Implementation

/// A place in an [`Overlay`] that can contain a widget.
///
/// Overlay entries are inserted into an [`Overlay`] using the [`OverlayState::insert`] or
/// [`OverlayState::insert_all`] functions. To find the closest enclosing overlay for a given
/// [`BuildContext`], use the [`Overlay::of`] function.
///
/// An overlay entry can be in at most one overlay at a time. To remove an entry from its
/// overlay, call the [`remove`](Self::remove) function on the overlay entry.
///
/// Because an [`Overlay`] uses a stack layout, overlay entries can use `Positioned` to
/// position themselves within the overlay.
///
/// By default, if there is an entirely [`opaque`](Self::opaque) entry over this one, then this
/// one will not be included in the widget tree (in particular, stateful widgets within the
/// overlay entry will not be instantiated). To ensure that your overlay entry is still built
/// even if it is not visible, set [`maintain_state`](Self::maintain_state) to true. This is
/// more expensive, so should be done with care.
///
/// `Handle<OverlayEntry>` is a [`Listenable`](reveal_foundation::Listenable) that notifies
/// when the widget built by [`builder`](Self::builder) is mounted or unmounted, whose exact
/// state can be queried by [`mounted`](Self::mounted). After the owner of the [`OverlayEntry`] calls
/// [`remove`](Self::remove) and then [`dispose`](Self::dispose), the widget may not be
/// immediately removed from the widget tree. As a result listeners of the [`OverlayEntry`] can
/// get notified for one last time after the `dispose` call, when the widget is eventually
/// unmounted.
///
/// See also:
///
///  * [`Overlay`], a stack of entries that can be managed independently.
///  * [`OverlayState`], the current state of an Overlay.
pub struct OverlayEntry {
    /// This entry will include the widget built by this builder in the overlay at the entry's
    /// position.
    ///
    /// To cause this builder to be called again, call [`mark_needs_build`] on this overlay
    /// entry.
    ///
    /// [`mark_needs_build`]: Self::mark_needs_build
    pub builder: WidgetBuilder,
    opaque: bool,
    maintain_state: bool,
    can_size_overlay: bool,
    /// The currently mounted [`OverlayEntryWidgetState`] built using this [`OverlayEntry`].
    overlay_entry_state_notifier:
        Option<Handle<ValueNotifier<Option<Handle<OverlayEntryWidgetState>>>>>,
    overlay: Option<Handle<OverlayState>>,
    key: GlobalKey,
    disposed_by_owner: bool,
}

impl OverlayEntry {
    /// Creates an overlay entry.
    ///
    /// To insert the entry into an [`Overlay`], first find the overlay using [`Overlay::of`]
    /// and then call [`OverlayState::insert`]. To remove the entry, call
    /// [`remove`](Self::remove) on the overlay entry itself.
    ///
    /// Dart's defaults: `opaque` false, `maintain_state` false, `can_size_overlay` false.
    pub fn new(
        app: &mut App,
        builder: WidgetBuilder,
        opaque: bool,
        maintain_state: bool,
        can_size_overlay: bool,
    ) -> Handle<OverlayEntry> {
        let notifier = app.create(ValueNotifier::new(None));
        app.create(OverlayEntry {
            builder,
            opaque,
            maintain_state,
            can_size_overlay,
            overlay_entry_state_notifier: Some(notifier),
            overlay: None,
            key: GlobalKey::new(),
            disposed_by_owner: false,
        })
    }

    /// Whether this entry occludes the entire overlay.
    ///
    /// If an entry claims to be opaque, then, for efficiency, the overlay will skip building
    /// entries below that entry unless they have [`maintain_state`](Self::maintain_state) set.
    pub fn opaque(self: Handle<Self>, app: &App) -> bool {
        app.get(self).opaque
    }

    /// Sets [`opaque`](Self::opaque).
    pub fn set_opaque(self: Handle<Self>, app: &mut App, value: bool) {
        debug_assert!(!app.get(self).disposed_by_owner);
        if app.get(self).opaque == value {
            return;
        }
        app.get_mut(self).opaque = value;
        if let Some(overlay) = app.get(self).overlay {
            overlay.did_change_entry_opacity(app);
        }
    }

    /// Whether this entry must be included in the tree even if there is a fully
    /// [`opaque`](Self::opaque) entry above it.
    ///
    /// By default, if there is an entirely opaque entry over this one, then this one will not
    /// be included in the widget tree (in particular, stateful widgets within the overlay
    /// entry will not be instantiated). To ensure that your overlay entry is still built even
    /// if it is not visible, set this to true. This is more expensive, so should be done with
    /// care.
    ///
    /// This is used by the `Navigator` and `Route` objects to ensure that routes are kept
    /// around even when in the background.
    pub fn maintain_state(self: Handle<Self>, app: &App) -> bool {
        app.get(self).maintain_state
    }

    /// Sets [`maintain_state`](Self::maintain_state).
    pub fn set_maintain_state(self: Handle<Self>, app: &mut App, value: bool) {
        debug_assert!(!app.get(self).disposed_by_owner);
        if app.get(self).maintain_state == value {
            return;
        }
        app.get_mut(self).maintain_state = value;
        let overlay = app.get(self).overlay.expect("inserted into an Overlay");
        overlay.did_change_entry_opacity(app);
    }

    /// Whether the content of this [`OverlayEntry`] can be used to size the [`Overlay`].
    ///
    /// In most situations the overlay sizes itself based on its incoming constraints to be as
    /// large as possible. However, if that would result in an infinite size, it has to rely on
    /// one of its children to size itself. In this situation, the overlay will consult the
    /// topmost non-positioned overlay entry that has this property set to true, lay it out with
    /// the incoming [`BoxConstraints`] of the overlay, and force all other non-positioned
    /// overlay entries to have the same size.
    ///
    /// Overlay entries that set this to true must be able to handle unconstrained
    /// [`BoxConstraints`].
    pub fn can_size_overlay(self: Handle<Self>, app: &App) -> bool {
        app.get(self).can_size_overlay
    }

    /// Whether the [`OverlayEntry`] is currently mounted in the widget tree.
    ///
    /// The [`OverlayEntry`] notifies its listeners when this value changes.
    pub fn mounted(self: Handle<Self>, app: &App) -> bool {
        app.get(self)
            .overlay_entry_state_notifier
            .is_some_and(|notifier| app.get(notifier).value().is_some())
    }

    /// Remove this entry from the overlay.
    ///
    /// This should only be called once.
    ///
    /// This method removes this overlay entry from the overlay immediately. The UI will be
    /// updated in the same frame if this method is called before the overlay rebuild in this
    /// frame; otherwise, the UI will be updated in the next frame.
    pub fn remove(self: Handle<Self>, app: &mut App) {
        let overlay = app
            .get(self)
            .overlay
            .expect("An OverlayEntry should be removed only once.");
        debug_assert!(!app.get(self).disposed_by_owner);
        app.get_mut(self).overlay = None;
        if !overlay.mounted(app) {
            return;
        }

        app.get_mut(overlay).entries.retain(|entry| *entry != self);
        if SchedulerBinding::scheduler_phase(app) == SchedulerPhase::PersistentCallbacks {
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::handle_method(overlay, OverlayState::mark_dirty_after_frame),
            );
        } else {
            overlay.mark_dirty(app);
        }
    }

    /// Cause this entry to rebuild during the next pipeline flush.
    ///
    /// You need to call this function if the output of [`builder`](Self::builder) has changed.
    pub fn mark_needs_build(self: Handle<Self>, app: &mut App) {
        debug_assert!(!app.get(self).disposed_by_owner);
        let key = app.get(self).key.clone();
        if let Some(state) = key.current_state::<OverlayEntryWidgetState>(app) {
            state.mark_needs_build(app);
        }
    }

    fn did_unmount(self: Handle<Self>, app: &mut App) {
        debug_assert!(!self.mounted(app));
        if app.get(self).disposed_by_owner {
            self.dispose_notifier(app);
        }
    }

    fn dispose_notifier(self: Handle<Self>, app: &mut App) {
        if let Some(notifier) = app.get_mut(self).overlay_entry_state_notifier.take() {
            app.get_mut(notifier).dispose();
        }
    }

    /// Discards any resources used by this [`OverlayEntry`].
    ///
    /// The [`remove`](Self::remove) method must be called before this method if the
    /// [`OverlayEntry`] is inserted into an [`Overlay`].
    ///
    /// After this is called, the object is not in a usable state and should be discarded.
    /// However, the listeners registered may not be immediately released until the widget built
    /// using this [`OverlayEntry`] is unmounted from the widget tree.
    ///
    /// This method should only be called by the object's owner.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        debug_assert!(!app.get(self).disposed_by_owner);
        debug_assert!(
            app.get(self).overlay.is_none(),
            "An OverlayEntry must first be removed from the Overlay before dispose is called."
        );
        app.get_mut(self).disposed_by_owner = true;
        if !self.mounted(app) {
            // If we're still mounted when disposed, then this will be disposed in did_unmount,
            // to allow notifications to occur until the entry is unmounted.
            self.dispose_notifier(app);
        }
    }
}

impl ListenableObject for OverlayEntry {
    fn add_listener(self: Handle<Self>, app: &mut App, listener: Listener) {
        debug_assert!(!app.get(self).disposed_by_owner);
        if let Some(notifier) = app.get(self).overlay_entry_state_notifier {
            notifier.add_listener(app, listener);
        }
    }

    fn remove_listener(self: Handle<Self>, app: &mut App, listener: &Listener) {
        if let Some(notifier) = app.get(self).overlay_entry_state_notifier {
            notifier.remove_listener(app, listener);
        }
    }
}

impl Debug for OverlayEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OverlayEntry(opaque: {}; maintainState: {}){}",
            self.opaque,
            self.maintain_state,
            if self.disposed_by_owner {
                "(DISPOSED)"
            } else {
                ""
            }
        )
    }
}

/// Dart's `_OverlayEntryWidget`: the tree node one [`OverlayEntry`] builds into.
#[derive(Debug)]
struct OverlayEntryWidget {
    key: KeyRef,
    entry: Handle<OverlayEntry>,
    overlay_state: Handle<OverlayState>,
    ticker_enabled: bool,
}

impl StatefulWidget for OverlayEntryWidget {
    type State = OverlayEntryWidgetState;

    fn key(&self) -> Option<&KeyRef> {
        Some(&self.key)
    }

    fn create_state(&self) -> OverlayEntryWidgetState {
        OverlayEntryWidgetState {
            state: StateData::new(),
            theater: None,
            sorted_theater_siblings: None,
        }
    }
}

/// Dart's `_OverlayEntryWidgetState`.
struct OverlayEntryWidgetState {
    state: StateData<OverlayEntryWidget>,
    theater: Option<RenderHandle<RenderTheater>>,
    // Manages the stack of theater children whose paint order is sorted by their
    // z_order_index. The children added by OverlayPortal are added to this list, and they
    // will be shown *above* the OverlayEntry tied to this widget. The children with larger
    // z_order_index values (that is, those `show`n recently) will be painted last.
    //
    // This list is lazily created in `add`, and the entries are added/removed via
    // `add`/`remove`, called by OverlayPortals lower in the tree. `add` or `remove` does not
    // cause this widget to rebuild, the list will be read by RenderTheater as part of its
    // render child model. This would ideally be in a RenderObject but there may not be
    // RenderObjects between RenderTheater and the render subtree OverlayEntry builds.
    sorted_theater_siblings: Option<Vec<Handle<OverlayEntryLocation>>>,
}

impl OverlayEntryWidgetState {
    fn theater(self: Handle<Self>, app: &App) -> RenderHandle<RenderTheater> {
        app.get(self).theater.expect("mounted under an Overlay")
    }

    // Worst-case O(N), N being the number of children added to the top spot in the same
    // frame. This can be a bit expensive when there's a lot of global key reparenting in the
    // same frame but N is usually a small number.
    fn add(self: Handle<Self>, app: &mut App, child: Handle<OverlayEntryLocation>) {
        debug_assert!(self.mounted(app));
        let z_order_index = app.get(child).z_order_index;
        let existing = app
            .get(self)
            .sorted_theater_siblings
            .clone()
            .unwrap_or_default();
        debug_assert!(!existing.contains(&child));
        let mut insert_position = existing.len();
        while insert_position > 0
            && app.get(existing[insert_position - 1]).z_order_index > z_order_index
        {
            insert_position -= 1;
        }
        app.get_mut(self)
            .sorted_theater_siblings
            .get_or_insert_with(Vec::new)
            .insert(insert_position, child);
    }

    fn remove(self: Handle<Self>, app: &mut App, child: Handle<OverlayEntryLocation>) {
        let children = app.get_mut(self).sorted_theater_siblings.as_mut();
        debug_assert!(children.is_some());
        let Some(children) = children else { return };
        let position = children.iter().position(|other| *other == child);
        debug_assert!(position.is_some(), "the child was in the collection");
        if let Some(position) = position {
            children.remove(position);
        }
    }

    // The children in the child model in paint order (from farthest to the user to the
    // closest to the user).
    fn paint_order_children(
        self: Handle<Self>,
        app: &App,
    ) -> Vec<RenderHandle<RenderDeferredLayoutBox>> {
        self.create_child_list(app, false)
    }

    // The children in the child model in hit-test order (from closest to the user to the
    // farthest to the user).
    fn hit_test_order_children(
        self: Handle<Self>,
        app: &App,
    ) -> Vec<RenderHandle<RenderDeferredLayoutBox>> {
        self.create_child_list(app, true)
    }

    fn create_child_list(
        self: Handle<Self>,
        app: &App,
        reversed: bool,
    ) -> Vec<RenderHandle<RenderDeferredLayoutBox>> {
        let Some(children) = app.get(self).sorted_theater_siblings.as_ref() else {
            return Vec::new();
        };
        let mut boxes: Vec<RenderHandle<RenderDeferredLayoutBox>> = children
            .iter()
            .filter_map(|candidate| app.get(*candidate).overlay_child_render_box)
            .collect();
        if reversed {
            boxes.reverse();
        }
        boxes
    }

    fn mark_needs_build(self: Handle<Self>, app: &mut App) {
        // The state that changed is in the builder.
        self.set_state(app, |_state| {});
    }
}

impl State for OverlayEntryWidgetState {
    type Widget = OverlayEntryWidget;

    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let entry = self.widget(app).entry;
        let notifier = app
            .get(entry)
            .overlay_entry_state_notifier
            .expect("a live OverlayEntry");
        notifier.set_value(app, Some(self));
        let context = self.context(app);
        let theater = context
            .find_ancestor_render_object_of_type::<RenderTheater>(app)
            .expect("an _OverlayEntryWidget is built by an Overlay");
        app.get_mut(self).theater = Some(theater);
        debug_assert!(app.get(self).sorted_theater_siblings.is_none());
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &OverlayEntryWidget) {
        // OverlayState's build method always returns a RenderObjectWidget Theater, so it is
        // safe to assume that state equality implies render object equality.
        debug_assert!(old_widget.entry == self.widget(app).entry);
        let changed = old_widget.overlay_state != self.widget(app).overlay_state;
        if changed {
            let context = self.context(app);
            let new_theater = context
                .find_ancestor_render_object_of_type::<RenderTheater>(app)
                .expect("an _OverlayEntryWidget is built by an Overlay");
            debug_assert!(app.get(self).theater != Some(new_theater));
            app.get_mut(self).theater = Some(new_theater);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let entry = self.widget(app).entry;
        if let Some(notifier) = app.get(entry).overlay_entry_state_notifier {
            notifier.set_value(app, None);
        }
        entry.did_unmount(app);
        app.get_mut(self).sorted_theater_siblings = None;
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.widget(app);
        let (ticker_enabled, entry) = (widget.ticker_enabled, widget.entry);
        let theater = self.theater(app);
        let builder = Rc::clone(&app.get(entry).builder);
        TickerMode::new(
            ticker_enabled,
            // Use a Builder so that `entry.builder` can have access to RenderTheaterMarker::of.
            RenderTheaterMarker {
                theater,
                overlay_entry_widget_state: self,
                child: Builder { key: None, builder }.into_widget(),
            },
        )
        .into_widget()
    }
}

/// A stack of entries that can be managed independently.
///
/// Overlays let independent child widgets "float" visual elements on top of other widgets by
/// inserting them into the overlay's stack. The overlay lets each of these widgets manage their
/// participation in the overlay using [`OverlayEntry`] objects.
///
/// Although you can create an [`Overlay`] directly, it's most common to use the overlay created
/// by the `Navigator`. The navigator uses its overlay to manage the visual appearance of its
/// routes.
///
/// The [`Overlay`] widget uses a custom stack implementation, which is very similar to the
/// `Stack` widget. The main use case of [`Overlay`] is related to navigation and being able to
/// insert widgets on top of the pages in an app. For layout purposes unrelated to navigation,
/// consider using `Stack` instead.
///
/// An [`Overlay`] widget requires a [`Directionality`] widget to be in scope, so that it can
/// resolve direction-sensitive coordinates of any `Positioned::directional` children.
///
/// See also:
///
///  * [`OverlayEntry`], the class that is used for describing the overlay entries.
///  * [`OverlayState`], which is used to insert the entries into the overlay.
#[derive(Debug)]
pub struct Overlay {
    /// See [`Widget::key`].
    pub key: Option<KeyRef>,
    /// The entries to include in the overlay initially.
    ///
    /// These entries are only used when the [`OverlayState`] is initialized. If you are
    /// providing a new [`Overlay`] description for an overlay that's already in the tree, then
    /// the new entries are ignored.
    ///
    /// To add entries to an [`Overlay`] that is already in the tree, use [`Overlay::of`] to
    /// obtain the [`OverlayState`], and then use [`OverlayState::insert`] or
    /// [`OverlayState::insert_all`].
    ///
    /// To remove an entry from an [`Overlay`], use [`OverlayEntry::remove`].
    pub initial_entries: Vec<Handle<OverlayEntry>>,
    /// How to clip the entries that overflow the overlay.
    ///
    /// Defaults to [`Clip::HardEdge`].
    pub clip_behavior: Clip,
    /// Determines that the overlay should always size itself to content.
    ///
    /// Normally overlay will only size itself to content if the incoming constraints are
    /// infinite and there is an [`OverlayEntry`] that can size the overlay. Setting this to
    /// `true` will force this behavior even for finite (but possibly loose) constraints.
    ///
    /// Setting this to true requires an [`OverlayEntry`] that can size the overlay based on
    /// itself ([`OverlayEntry::can_size_overlay`] set to true). If not provided the overlay
    /// panics.
    pub always_size_to_content: bool,
}

impl Overlay {
    /// Creates an overlay; Dart's named arguments are the setters.
    ///
    /// The initial entries will be inserted into the overlay when its associated
    /// [`OverlayState`] is initialized.
    pub fn new() -> Overlay {
        Overlay::default()
    }

    /// Dart `Overlay(key:)`.
    pub fn key(mut self, key: KeyRef) -> Overlay {
        self.key = Some(key);
        self
    }

    /// Dart `Overlay(initialEntries:)`.
    pub fn initial_entries(
        mut self,
        entries: impl IntoIterator<Item = Handle<OverlayEntry>>,
    ) -> Overlay {
        self.initial_entries = entries.into_iter().collect();
        self
    }

    /// Dart `Overlay(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> Overlay {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `Overlay(alwaysSizeToContent:)`.
    pub fn always_size_to_content(mut self, always_size_to_content: bool) -> Overlay {
        self.always_size_to_content = always_size_to_content;
        self
    }

    /// Wrap the provided `child` in an [`Overlay`] to allow other visual elements (packed in
    /// [`OverlayEntry`]s) to float on top of the child.
    ///
    /// This is a convenience method over the regular [`Overlay`] constructor: it creates an
    /// [`Overlay`] and puts the provided `child` in an [`OverlayEntry`] at the bottom of that
    /// newly created Overlay.
    pub fn wrap<K>(child: impl IntoWidget<K>) -> WrappingOverlay {
        WrappingOverlay {
            key: None,
            clip_behavior: Clip::HardEdge,
            always_size_to_content: false,
            child: child.into_widget(),
        }
    }

    /// The [`OverlayState`] from the closest instance of [`Overlay`] that encloses the given
    /// context, and panics if one is not found.
    ///
    /// If `root_overlay` is set to true, the state from the furthest instance of this class is
    /// given instead. Useful for installing overlay entries above all subsequent instances of
    /// [`Overlay`].
    ///
    /// See also:
    ///
    /// * [`Overlay::maybe_of`] for a similar function that returns `None` if an [`Overlay`] is
    ///   not found.
    pub fn of(app: &mut App, context: BuildContext, root_overlay: bool) -> Handle<OverlayState> {
        Overlay::maybe_of(app, context, root_overlay).expect(
            "No Overlay widget found. Some widgets require an Overlay widget ancestor for \
             correct operation. The most common way to add an Overlay to an application is to \
             include a WidgetsApp or Navigator widget in the run_app() call.",
        )
    }

    /// The [`OverlayState`] from the closest instance of [`Overlay`] that encloses the given
    /// context, if any.
    ///
    /// If `root_overlay` is set to true, the state from the furthest instance of this class is
    /// given instead.
    ///
    /// See also:
    ///
    ///  * [`Overlay::of`] for a similar function that panics if an [`Overlay`] is not found.
    pub fn maybe_of(
        app: &mut App,
        context: BuildContext,
        root_overlay: bool,
    ) -> Option<Handle<OverlayState>> {
        let marker = RenderTheaterMarker::maybe_of(app, context, root_overlay, false)?;
        Some(marker.overlay_entry_widget_state.widget(app).overlay_state)
    }
}

impl Default for Overlay {
    fn default() -> Overlay {
        Overlay {
            key: None,
            initial_entries: Vec::new(),
            clip_behavior: Clip::HardEdge,
            always_size_to_content: false,
        }
    }
}

impl StatefulWidget for Overlay {
    type State = OverlayState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> OverlayState {
        OverlayState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            entries: Vec::new(),
        }
    }
}

/// The current state of an [`Overlay`].
///
/// Used to insert [`OverlayEntry`]s into the overlay using the [`insert`](Self::insert) and
/// [`insert_all`](Self::insert_all) functions.
pub struct OverlayState {
    state: StateData<Overlay>,
    ticker_provider: TickerProviderStateMixinData,
    entries: Vec<Handle<OverlayEntry>>,
}

impl OverlayState {
    fn insertion_index(
        self: Handle<Self>,
        app: &App,
        below: Option<Handle<OverlayEntry>>,
        above: Option<Handle<OverlayEntry>>,
    ) -> usize {
        debug_assert!(above.is_none() || below.is_none());
        let entries = &app.get(self).entries;
        if let Some(below) = below {
            return entries
                .iter()
                .position(|entry| *entry == below)
                .expect("`below` is in this Overlay");
        }
        if let Some(above) = above {
            return entries
                .iter()
                .position(|entry| *entry == above)
                .expect("`above` is in this Overlay")
                + 1;
        }
        entries.len()
    }

    fn debug_can_insert_entry(self: Handle<Self>, app: &App, entry: Handle<OverlayEntry>) -> bool {
        assert!(
            self.mounted(app),
            "Attempted to insert an OverlayEntry to an already disposed Overlay."
        );
        let current_overlay = app.get(entry).overlay;
        assert!(
            !app.get(self).entries.contains(&entry),
            "The specified entry is already present in the target Overlay. Consider calling \
             remove on the OverlayEntry before inserting it to a different Overlay."
        );
        assert!(
            current_overlay.is_none(),
            "The specified entry is already present in a different Overlay. Consider calling \
             remove on the OverlayEntry before inserting it to a different Overlay."
        );
        true
    }

    /// Insert the given entry into the overlay.
    ///
    /// If `below` is non-`None`, the entry is inserted just below `below`. If `above` is
    /// non-`None`, the entry is inserted just above `above`. Otherwise, the entry is inserted
    /// on top.
    ///
    /// It is an error to specify both `above` and `below`.
    pub fn insert(
        self: Handle<Self>,
        app: &mut App,
        entry: Handle<OverlayEntry>,
        below: Option<Handle<OverlayEntry>>,
        above: Option<Handle<OverlayEntry>>,
    ) {
        debug_assert!(self.debug_verify_insert_position(app, above, below, None));
        debug_assert!(self.debug_can_insert_entry(app, entry));
        app.get_mut(entry).overlay = Some(self);
        let index = self.insertion_index(app, below, above);
        self.set_state(app, |state| state.entries.insert(index, entry));
    }

    /// Insert all the entries in the given iterable.
    ///
    /// If `below` is non-`None`, the entries are inserted just below `below`. If `above` is
    /// non-`None`, the entries are inserted just above `above`. Otherwise, the entries are
    /// inserted on top.
    ///
    /// It is an error to specify both `above` and `below`.
    pub fn insert_all(
        self: Handle<Self>,
        app: &mut App,
        entries: Vec<Handle<OverlayEntry>>,
        below: Option<Handle<OverlayEntry>>,
        above: Option<Handle<OverlayEntry>>,
    ) {
        debug_assert!(self.debug_verify_insert_position(app, above, below, None));
        if cfg!(debug_assertions) {
            for entry in &entries {
                assert!(self.debug_can_insert_entry(app, *entry));
            }
        }
        if entries.is_empty() {
            return;
        }
        for entry in &entries {
            debug_assert!(app.get(*entry).overlay.is_none());
            app.get_mut(*entry).overlay = Some(self);
        }
        let index = self.insertion_index(app, below, above);
        self.set_state(app, move |state| {
            state.entries.splice(index..index, entries);
        });
    }

    fn debug_verify_insert_position(
        self: Handle<Self>,
        app: &App,
        above: Option<Handle<OverlayEntry>>,
        below: Option<Handle<OverlayEntry>>,
        new_entries: Option<&[Handle<OverlayEntry>]>,
    ) -> bool {
        assert!(
            above.is_none() || below.is_none(),
            "Only one of `above` and `below` may be specified."
        );
        let present = |entry: Handle<OverlayEntry>| {
            app.get(entry).overlay == Some(self)
                && app.get(self).entries.contains(&entry)
                && new_entries.is_none_or(|entries| entries.contains(&entry))
        };
        assert!(
            above.is_none_or(present),
            "The provided entry used for `above` must be present in the Overlay."
        );
        assert!(
            below.is_none_or(present),
            "The provided entry used for `below` must be present in the Overlay."
        );
        true
    }

    /// Remove all the entries listed in the given iterable, then reinsert them into the overlay
    /// in the given order.
    ///
    /// Entries mentioned in `new_entries` but absent from the overlay are inserted as if with
    /// [`insert_all`](Self::insert_all).
    ///
    /// Entries not mentioned in `new_entries` but present in the overlay are positioned as a
    /// group in the resulting list relative to the entries that were moved, as specified by one
    /// of `below` or `above`, which, if specified, must be one of the entries in `new_entries`:
    ///
    /// If `below` is non-`None`, the group is positioned just below `below`. If `above` is
    /// non-`None`, the group is positioned just above `above`. Otherwise, the group is left on
    /// top, with all the rearranged entries below.
    ///
    /// It is an error to specify both `above` and `below`.
    pub fn rearrange(
        self: Handle<Self>,
        app: &mut App,
        new_entries: Vec<Handle<OverlayEntry>>,
        below: Option<Handle<OverlayEntry>>,
        above: Option<Handle<OverlayEntry>>,
    ) {
        debug_assert!(self.debug_verify_insert_position(app, above, below, Some(&new_entries)));
        debug_assert!(
            new_entries.iter().all(|entry| {
                let overlay = app.get(*entry).overlay;
                overlay.is_none() || overlay == Some(self)
            }),
            "One or more of the specified entries are already present in another Overlay."
        );
        debug_assert!(
            new_entries.iter().all(|entry| {
                let entries = &app.get(self).entries;
                entries.iter().position(|other| other == entry)
                    == entries.iter().rposition(|other| other == entry)
            }),
            "One or more of the specified entries are specified multiple times."
        );
        if new_entries.is_empty() {
            return;
        }
        if app.get(self).entries == new_entries {
            return;
        }
        let old: Vec<Handle<OverlayEntry>> = app
            .get(self)
            .entries
            .iter()
            .copied()
            .filter(|entry| !new_entries.contains(entry))
            .collect();
        for entry in &new_entries {
            if app.get(*entry).overlay.is_none() {
                app.get_mut(*entry).overlay = Some(self);
            }
        }
        let index = {
            let entries = &new_entries;
            if let Some(below) = below {
                entries
                    .iter()
                    .position(|entry| *entry == below)
                    .expect("`below` is in `new_entries`")
            } else if let Some(above) = above {
                entries
                    .iter()
                    .position(|entry| *entry == above)
                    .expect("`above` is in `new_entries`")
                    + 1
            } else {
                entries.len()
            }
        };
        self.set_state(app, move |state| {
            state.entries.clear();
            state.entries.extend(new_entries);
            state.entries.splice(index..index, old);
        });
    }

    fn mark_dirty(self: Handle<Self>, app: &mut App) {
        if self.mounted(app) {
            self.set_state(app, |_state| {});
        }
    }

    fn mark_dirty_after_frame(self: Handle<Self>, app: &mut App, _time_stamp: std::time::Duration) {
        self.mark_dirty(app);
    }

    /// (DEBUG ONLY) Check whether a given entry is visible (i.e., not behind an opaque entry).
    ///
    /// This is an O(N) algorithm, and should not be necessary except for debug asserts. To
    /// avoid people depending on it, this function is implemented only in debug mode, and
    /// always returns false in release mode.
    pub fn debug_is_visible(self: Handle<Self>, app: &App, entry: Handle<OverlayEntry>) -> bool {
        let mut result = false;
        debug_assert!(app.get(self).entries.contains(&entry));
        if cfg!(debug_assertions) {
            let entries = app.get(self).entries.clone();
            for index in (1..entries.len()).rev() {
                let candidate = entries[index];
                if candidate == entry {
                    result = true;
                    break;
                }
                if candidate.opaque(app) {
                    break;
                }
            }
        }
        result
    }

    fn did_change_entry_opacity(self: Handle<Self>, app: &mut App) {
        // We use the opacity of the entry in our build function, which means our state has
        // changed.
        self.set_state(app, |_state| {});
    }
}

impl TickerProviderStateMixin for OverlayState {
    fn ticker_provider_data(self: Handle<Self>, app: &App) -> &TickerProviderStateMixinData {
        &app.get(self).ticker_provider
    }

    fn ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut TickerProviderStateMixinData {
        &mut app.get_mut(self).ticker_provider
    }
}

impl TickerProviderObject for OverlayState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for OverlayState {
    type Widget = Overlay;

    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let initial = self.widget(app).initial_entries.clone();
        self.insert_all(app, initial, None, None);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        // This list is filled backwards and then reversed below before it is added to the tree.
        let mut children: Vec<WidgetRef> = Vec::new();
        let mut onstage = true;
        let mut onstage_count = 0usize;
        for entry in app.get(self).entries.clone().into_iter().rev() {
            let key: KeyRef = Rc::new(app.get(entry).key.clone());
            if onstage {
                onstage_count += 1;
                children.push(
                    OverlayEntryWidget {
                        key,
                        overlay_state: self,
                        entry,
                        ticker_enabled: true,
                    }
                    .into_widget(),
                );
                if app.get(entry).opaque {
                    onstage = false;
                }
            } else if app.get(entry).maintain_state {
                children.push(
                    OverlayEntryWidget {
                        key,
                        overlay_state: self,
                        entry,
                        ticker_enabled: false,
                    }
                    .into_widget(),
                );
            }
        }
        let skip_count = children.len() - onstage_count;
        children.reverse();
        let widget = self.widget(app);
        Theater {
            skip_count,
            clip_behavior: widget.clip_behavior,
            always_size_to_content: widget.always_size_to_content,
            children,
        }
        .into_widget()
    }
}

/// Dart's `_WrappingOverlay`: the widget [`Overlay::wrap`] returns.
#[derive(Debug)]
pub struct WrappingOverlay {
    /// See [`Widget::key`].
    pub key: Option<KeyRef>,
    /// See [`Overlay::clip_behavior`].
    pub clip_behavior: Clip,
    /// See [`Overlay::always_size_to_content`].
    pub always_size_to_content: bool,
    /// The widget the created [`Overlay`]'s bottom entry builds.
    pub child: WidgetRef,
}

impl WrappingOverlay {
    /// Dart `Overlay.wrap(key:)`.
    pub fn key(mut self, key: KeyRef) -> WrappingOverlay {
        self.key = Some(key);
        self
    }

    /// Dart `Overlay.wrap(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> WrappingOverlay {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `Overlay.wrap(alwaysSizeToContent:)`.
    pub fn always_size_to_content(mut self, always_size_to_content: bool) -> WrappingOverlay {
        self.always_size_to_content = always_size_to_content;
        self
    }
}

impl StatefulWidget for WrappingOverlay {
    type State = WrappingOverlayState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> WrappingOverlayState {
        WrappingOverlayState {
            state: StateData::new(),
            entry: None,
        }
    }
}

/// Dart's `_WrappingOverlayState`.
pub struct WrappingOverlayState {
    state: StateData<WrappingOverlay>,
    entry: Option<Handle<OverlayEntry>>,
}

impl WrappingOverlayState {
    /// Dart's `late final _entry`, created on its first use.
    fn entry(self: Handle<Self>, app: &mut App) -> Handle<OverlayEntry> {
        if let Some(entry) = app.get(self).entry {
            return entry;
        }
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |app: &mut App, _context| self.widget(app).child.clone()),
            true,
            false,
            true,
        );
        app.get_mut(self).entry = Some(entry);
        entry
    }
}

impl State for WrappingOverlayState {
    type Widget = WrappingOverlay;

    crate::state_accessors!();

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &WrappingOverlay) {
        let entry = self.entry(app);
        entry.mark_needs_build(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let entry = self.entry(app);
        entry.remove(app);
        entry.dispose(app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let entry = self.entry(app);
        let widget = self.widget(app);
        Overlay {
            key: None,
            initial_entries: vec![entry],
            clip_behavior: widget.clip_behavior,
            always_size_to_content: widget.always_size_to_content,
        }
        .into_widget()
    }
}

/// Special version of a stack that doesn't lay out and render the first
/// [`skip_count`](Self::skip_count) children.
///
/// The first `skip_count` children are considered "offstage".
#[derive(Debug)]
struct Theater {
    skip_count: usize,
    clip_behavior: Clip,
    always_size_to_content: bool,
    children: Vec<WidgetRef>,
}

impl Theater {
    /// The tree node: a multi-child render object widget with its own element, outside the
    /// `IntoWidget` kinds.
    fn into_widget(self) -> WidgetRef {
        debug_assert!(self.children.len() >= self.skip_count);
        Rc::new(self)
    }
}

impl Widget for Theater {
    fn key(&self) -> Option<&KeyRef> {
        None
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        TheaterElement::create(app, this).as_element()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn widget_type(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Theater>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

impl RenderObjectWidget for Theater {
    type RenderObject = RenderTheater;

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::of(app, context);
        let render_object = RenderTheater::new(
            app,
            text_direction,
            self.skip_count,
            self.clip_behavior,
            self.always_size_to_content,
        );
        render_object.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderTheater>,
    ) {
        render_object.set_skip_count(app, self.skip_count);
        let text_direction = Directionality::of(app, context);
        render_object.set_text_direction(app, text_direction);
        render_object.set_clip_behavior(app, self.clip_behavior);
        render_object.set_always_size_to_content(app, self.always_size_to_content);
    }
}

impl MultiChildRenderObjectWidget for Theater {
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

/// Dart's `_TheaterElement`: stamps each child's [`TheaterParentData`] with the
/// [`OverlayEntry`] that built it.
struct TheaterElement {
    element: ElementData,
    render_object_element: RenderObjectElementData,
    multi_child: MultiChildRenderObjectElementData,
}

impl TheaterElement {
    fn create(app: &mut App, widget: WidgetRef) -> Handle<TheaterElement> {
        debug_assert!(downcast_widget::<Theater>(&*widget).is_some());
        app.create(TheaterElement {
            element: ElementData::new(widget),
            render_object_element: RenderObjectElementData::default(),
            multi_child: MultiChildRenderObjectElementData::default(),
        })
    }

    /// The [`OverlayEntry`] of the `OverlayEntryWidget` at the given slot's index.
    fn entry_at_slot(self: Handle<Self>, app: &App, slot: Option<&Slot>) -> Handle<OverlayEntry> {
        let index = <Self as MultiChildRenderObjectElementBase>::indexed_slot(slot).index;
        let widget = Self::widget_of(self.as_element().widget(app));
        downcast_widget::<OverlayEntryWidget>(&*widget.children[index])
            .expect("a Theater's children are _OverlayEntryWidgets")
            .entry
    }
}

impl RenderObjectElementWidget for TheaterElement {
    type Widget = Theater;

    fn widget_of(widget: &WidgetRef) -> &Theater {
        downcast_widget::<Theater>(&**widget).expect("a _TheaterElement holds a _Theater")
    }
}

impl RenderObjectElement for TheaterElement {
    fn render_object_element_data(self: Handle<Self>, app: &App) -> &RenderObjectElementData {
        &app.get(self).render_object_element
    }

    fn render_object_element_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectElementData {
        &mut app.get_mut(self).render_object_element
    }
}

impl MultiChildRenderObjectElementBase for TheaterElement {
    fn multi_child_data(self: Handle<Self>, app: &App) -> &MultiChildRenderObjectElementData {
        &app.get(self).multi_child
    }

    fn multi_child_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut MultiChildRenderObjectElementData {
        &mut app.get_mut(self).multi_child
    }
}

impl Element for TheaterElement {
    crate::element_accessors!();

    const IS_RENDER_OBJECT_ELEMENT: bool = true;

    fn render_object(self: Handle<Self>, app: &App) -> Option<AnyRenderObject> {
        self.render_object_element_data(app).render_object()
    }

    fn render_object_attaching_child(self: Handle<Self>, _app: &App) -> Option<AnyElement> {
        None
    }

    fn debug_doing_build(self: Handle<Self>, app: &App) -> bool {
        self.render_object_element_data(app).debug_doing_build()
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::perform_rebuild(self, app);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::deactivate(self, app);
    }

    fn unmount(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::unmount(self, app);
    }

    fn update_parent_data(self: Handle<Self>, app: &mut App, parent_data_element: AnyElement) {
        RenderObjectElement::update_parent_data(self, app, parent_data_element);
    }

    fn update_slot(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::update_slot(self, app, new_slot);
    }

    fn attach_render_object(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::attach_render_object(self, app, new_slot);
    }

    fn detach_render_object(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::detach_render_object(self, app);
    }

    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        MultiChildRenderObjectElementBase::visit_children(self, app, visitor);
    }

    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        MultiChildRenderObjectElementBase::forget_child(self, app, child);
    }

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        MultiChildRenderObjectElementBase::mount(self, app, parent, new_slot);
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        MultiChildRenderObjectElementBase::update(self, app, new_widget);
    }

    fn insert_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        MultiChildRenderObjectElementBase::insert_render_object_child(
            self,
            app,
            child,
            slot.clone(),
        );
        let entry = self.entry_at_slot(app, slot.as_ref());
        child
            .parent_data_of_mut::<TheaterParentData>(app)
            .overlay_entry = Some(entry);
    }

    fn move_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        old_slot: Option<Slot>,
        new_slot: Option<Slot>,
    ) {
        MultiChildRenderObjectElementBase::move_render_object_child(
            self,
            app,
            child,
            old_slot,
            new_slot.clone(),
        );
        if cfg!(debug_assertions) {
            let entry_at_new_slot = self.entry_at_slot(app, new_slot.as_ref());
            let parent_data = child.parent_data_of::<TheaterParentData>(app);
            assert!(parent_data.overlay_entry == Some(entry_at_new_slot));
        }
    }

    fn remove_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        MultiChildRenderObjectElementBase::remove_render_object_child(self, app, child, slot);
    }
}

/// Dart's `_TheaterParentData`: [`StackParentData`] plus the entry that created the child.
#[derive(Debug, Default)]
pub struct TheaterParentData {
    stack: StackParentData,
    /// The [`OverlayEntry`] that directly created this child.
    overlay_entry: Option<Handle<OverlayEntry>>,
}

impl TheaterParentData {
    /// Parent data for a child of a [`RenderTheater`] that no entry has claimed yet.
    pub fn new() -> TheaterParentData {
        TheaterParentData {
            stack: StackParentData::new(),
            overlay_entry: None,
        }
    }

    /// The stack half: the positioning a `Positioned` child writes.
    pub fn stack(&self) -> &StackParentData {
        &self.stack
    }

    /// See [`stack`](Self::stack).
    pub fn stack_mut(&mut self) -> &mut StackParentData {
        &mut self.stack
    }

    // An [`OverlayPortal`] makes its overlay child a render child of an ancestor
    // [`Overlay`]. Currently, to make sure the overlay child is painted after its
    // [`OverlayPortal`], and before the next [`OverlayEntry`] (which could be something that
    // should obstruct the overlay child, such as a `ModalRoute`) in the host [`Overlay`], the
    // paint order of each overlay child is managed by the [`OverlayEntry`] that hosts its
    // [`OverlayPortal`].
    //
    // The following methods are exposed to allow easy access to the overlay children's render
    // objects whose order is managed by `overlay_entry`, in the right order.

    // The entry's notifier is cleared in OverlayEntryWidgetState's dispose method. These are
    // only accessed during layout, paint and hit-test so the `expect` is safe.
    fn paint_order_children(&self, app: &App) -> Vec<RenderHandle<RenderDeferredLayoutBox>> {
        match self.child_model(app) {
            Some(child_model) => child_model.paint_order_children(app),
            None => Vec::new(),
        }
    }

    fn hit_test_order_children(&self, app: &App) -> Vec<RenderHandle<RenderDeferredLayoutBox>> {
        match self.child_model(app) {
            Some(child_model) => child_model.hit_test_order_children(app),
            None => Vec::new(),
        }
    }

    fn child_model(&self, app: &App) -> Option<Handle<OverlayEntryWidgetState>> {
        let notifier = app.get(self.overlay_entry?).overlay_entry_state_notifier?;
        Some((*app.get(notifier).value()).expect("the entry is mounted"))
    }

    // A convenience method for traversing `paint_order_children` with a render object
    // visitor.
    fn visit_overlay_portal_children_on_overlay_entry(
        &self,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        for child in self.paint_order_children(app) {
            visitor(child.as_object());
        }
    }
}

impl ParentData for TheaterParentData {
    fn detach(&mut self) {
        ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<TheaterParentData>() {
            return Some(self);
        }
        self.stack.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<TheaterParentData>() {
            return Some(self);
        }
        self.stack.provide_mut(id)
    }
}

impl ContainerParentDataMixin for TheaterParentData {
    type ChildType = AnyRenderBox;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderBox> {
        self.stack.container_parent_data()
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderBox> {
        self.stack.container_parent_data_mut()
    }
}

impl ContainerBoxParentData for TheaterParentData {
    fn box_parent_data(&self) -> &reveal_rendering::BoxParentData {
        ContainerBoxParentData::box_parent_data(&self.stack)
    }

    fn box_parent_data_mut(&mut self) -> &mut reveal_rendering::BoxParentData {
        ContainerBoxParentData::box_parent_data_mut(&mut self.stack)
    }
}

impl Display for TheaterParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.stack, f)
    }
}

/// `RenderStack.layoutPositionedChild` for a child whose parent data is a
/// [`TheaterParentData`].
fn layout_positioned_theater_child(
    app: &mut App,
    child: AnyRenderBox,
    size: Size,
    alignment: Alignment,
) {
    let child_constraints = {
        let parent_data = child.as_object().parent_data_of::<TheaterParentData>(app);
        debug_assert!(parent_data.stack.is_positioned());
        parent_data.stack.positioned_child_constraints(size)
    };
    child.layout(app, child_constraints, true);

    let child_size = child.size(app);
    let parent_data = child.as_object().parent_data_of::<TheaterParentData>(app);
    let x = match (parent_data.stack.left, parent_data.stack.right) {
        (Some(left), _) => left,
        (None, Some(right)) => size.width() - right - child_size.width(),
        (None, None) => alignment.along_offset(size - child_size).dx(),
    };
    let y = match (parent_data.stack.top, parent_data.stack.bottom) {
        (Some(top), _) => top,
        (None, Some(bottom)) => size.height() - bottom - child_size.height(),
        (None, None) => alignment.along_offset(size - child_size).dy(),
    };
    child
        .as_object()
        .parent_data_of_mut::<TheaterParentData>(app)
        .set_offset(Offset::new(x, y));
}

/// A `RenderBox` that sizes itself to its parent's size, implements the stack layout algorithm
/// and renders its children in the given [`theater`](Self::theater).
///
/// Flutter's `_RenderTheaterMixin`.
pub trait RenderTheaterMixin: RenderBox {
    /// The theater whose child model this box paints from.
    fn theater(self: RenderHandle<Self>, app: &App) -> RenderHandle<RenderTheater>;

    /// The children in paint order: from farthest to the user to the closest to the user.
    fn children_in_paint_order(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderBox>;

    /// The children in hit-test order: from closest to the user to the farthest.
    fn children_in_hit_test_order(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderBox>;

    /// The body of Flutter's `setupParentData` override.
    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<TheaterParentData>(app) {
            child.set_parent_data(app, TheaterParentData::new());
        }
    }

    /// Lays one child out with the stack layout algorithm.
    fn layout_child(
        self: RenderHandle<Self>,
        app: &mut App,
        child: AnyRenderBox,
        non_positioned_child_constraints: BoxConstraints,
    ) {
        let alignment = self.theater(app).resolved_alignment(app);
        let is_positioned = child
            .as_object()
            .parent_data_of::<TheaterParentData>(app)
            .stack
            .is_positioned();
        if !is_positioned {
            // Dart dispatches this through `RenderObject.layout`, which
            // `_RenderDeferredLayoutBox` overrides to hand its subtree to the pipeline
            // owner's dirty list instead of laying it out inside this tree walk.
            match child.as_object().downcast::<RenderDeferredLayoutBox>(app) {
                Some(deferred) => {
                    let parent = child
                        .as_object()
                        .parent(app)
                        .expect("a child being laid out has a parent");
                    deferred.do_layout_from(app, parent, non_positioned_child_constraints);
                }
                None => child.layout(app, non_positioned_child_constraints, true),
            }
            child
                .as_object()
                .parent_data_of_mut::<TheaterParentData>(app)
                .set_offset(Offset::ZERO);
        } else {
            debug_assert!(
                child
                    .as_object()
                    .downcast::<RenderDeferredLayoutBox>(app)
                    .is_none(),
                "all deferred layout boxes must be non-positioned children."
            );
            let size = self.size(app);
            layout_positioned_theater_child(app, child, size, alignment);
        }
    }

    /// The body of Flutter's `hitTestChildren` override.
    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        for child in self.children_in_hit_test_order(app) {
            let offset = child
                .as_object()
                .parent_data_of::<TheaterParentData>(app)
                .offset();
            let is_hit = result.add_with_paint_offset(Some(offset), position, |result, local| {
                child.hit_test(app, result, local)
            });
            if is_hit {
                return true;
            }
        }
        false
    }

    /// The body of Flutter's `paint` override.
    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        for child in self.children_in_paint_order(app) {
            let child_offset = child
                .as_object()
                .parent_data_of::<TheaterParentData>(app)
                .offset();
            context.paint_child(app, child.as_object(), child_offset + offset);
        }
    }
}

/// The render object of an [`Overlay`]: a stack that skips its first
/// [`skip_count`](Self::skip_count) children.
///
/// Flutter's `_RenderTheater`.
pub struct RenderTheater {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    alignment_cache: Option<Alignment>,
    text_direction: TextDirection,
    skip_count: usize,
    clip_behavior: Clip,
    always_size_to_content: bool,
    laying_out_size_determining_child: bool,
}

impl RenderTheater {
    /// Creates the render object of an [`Overlay`]. Add children with
    /// [`add`](ContainerRenderObjectMixin::add); Dart's constructor takes them.
    pub fn new(
        app: &mut App,
        text_direction: TextDirection,
        skip_count: usize,
        clip_behavior: Clip,
        always_size_to_content: bool,
    ) -> RenderHandle<RenderTheater> {
        RenderHandle::new_box(
            app,
            RenderTheater {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                alignment_cache: None,
                text_direction,
                skip_count,
                clip_behavior,
                always_size_to_content,
                laying_out_size_determining_child: false,
            },
        )
    }

    /// `AlignmentDirectional.topStart` resolved against
    /// [`text_direction`](Self::text_direction), cached until it changes.
    pub fn resolved_alignment(self: RenderHandle<Self>, app: &mut App) -> Alignment {
        if let Some(cached) = self.get(app).alignment_cache {
            return cached;
        }
        let resolved = AlignmentGeometry::TOP_START.resolve(Some(self.get(app).text_direction));
        self.get_mut(app).alignment_cache = Some(resolved);
        resolved
    }

    fn mark_need_resolution(self: RenderHandle<Self>, app: &mut App) {
        self.get_mut(app).alignment_cache = None;
        self.mark_needs_layout(app);
    }

    /// The text direction with which the stack alignment is resolved.
    pub fn text_direction(self: RenderHandle<Self>, app: &App) -> TextDirection {
        self.get(app).text_direction
    }

    /// Sets [`text_direction`](Self::text_direction).
    pub fn set_text_direction(self: RenderHandle<Self>, app: &mut App, value: TextDirection) {
        if self.get(app).text_direction == value {
            return;
        }
        self.get_mut(app).text_direction = value;
        self.mark_need_resolution(app);
    }

    /// How many children at the start of the child list are offstage.
    pub fn skip_count(self: RenderHandle<Self>, app: &App) -> usize {
        self.get(app).skip_count
    }

    /// Sets [`skip_count`](Self::skip_count).
    pub fn set_skip_count(self: RenderHandle<Self>, app: &mut App, value: usize) {
        if self.get(app).skip_count != value {
            self.get_mut(app).skip_count = value;
            self.mark_needs_layout(app);
        }
    }

    /// How to clip the children that overflow the overlay.
    ///
    /// Defaults to [`Clip::HardEdge`].
    pub fn clip_behavior(self: RenderHandle<Self>, app: &App) -> Clip {
        self.get(app).clip_behavior
    }

    /// Sets [`clip_behavior`](Self::clip_behavior).
    pub fn set_clip_behavior(self: RenderHandle<Self>, app: &mut App, value: Clip) {
        if value != self.get(app).clip_behavior {
            self.get_mut(app).clip_behavior = value;
            self.mark_needs_paint(app);
        }
    }

    /// See [`Overlay::always_size_to_content`].
    pub fn always_size_to_content(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).always_size_to_content
    }

    /// Sets [`always_size_to_content`](Self::always_size_to_content).
    pub fn set_always_size_to_content(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).always_size_to_content != value {
            self.get_mut(app).always_size_to_content = value;
            self.mark_needs_layout(app);
        }
    }

    // Adding/removing a deferred child does not affect the layout of the other children, or
    // that of the Overlay. Flutter suppresses the `markNeedsLayout` that `adoptChild` and
    // `dropChild` do here; `RenderBox::mark_needs_layout` has no callable base body outside
    // `reveal-rendering`, so the Overlay is relaid out instead (see `PORTING.md`).
    fn add_deferred_child(
        self: RenderHandle<Self>,
        app: &mut App,
        child: RenderHandle<RenderDeferredLayoutBox>,
    ) {
        self.adopt_child(app, child.as_object());
        // The Overlay still needs repainting when a deferred child is added.
        self.mark_needs_paint(app);

        // After adding `child` to the render tree, we want to make sure it will be laid out
        // in the same frame. This is done by calling mark_needs_layout on the layout
        // surrogate. This ensures `child` is added to the dirty list (see
        // RenderLayoutSurrogateProxyBox's perform_layout).
        let surrogate = child.get(app).layout_surrogate;
        surrogate.as_object().mark_needs_layout(app);
    }

    fn remove_deferred_child(
        self: RenderHandle<Self>,
        app: &mut App,
        child: RenderHandle<RenderDeferredLayoutBox>,
    ) {
        self.drop_child(app, child.as_object());
        // The Overlay still needs repainting when a deferred child is dropped. See the
        // comment in `add_deferred_child`.
        self.mark_needs_paint(app);
    }

    fn first_onstage_child(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderBox> {
        let skip_count = self.get(app).skip_count;
        if skip_count == self.child_count(app) {
            return None;
        }
        let mut child = self.first_child(app);
        for _ in 0..skip_count {
            let current = child.expect("skip_count is at most child_count");
            child = self.child_after(app, current);
        }
        child
    }

    fn last_onstage_child(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderBox> {
        if self.get(app).skip_count == self.child_count(app) {
            None
        } else {
            self.last_child(app)
        }
    }

    fn find_size_determining_child(self: RenderHandle<Self>, app: &App) -> AnyRenderBox {
        let mut child = self.last_onstage_child(app);
        while let Some(current) = child {
            let parent_data = current.as_object().parent_data_of::<TheaterParentData>(app);
            let can_size = parent_data
                .overlay_entry
                .is_some_and(|entry| app.get(entry).can_size_overlay);
            if can_size && !parent_data.stack.is_positioned() {
                return current;
            }
            child = self.child_before(app, current);
        }
        assert!(
            !self.get(app).always_size_to_content,
            "Overlay was asked to size itself to content but does not have a suitable child. \
             When `always_size_to_content` is true, the Overlay requires at least one \
             non-positioned `OverlayEntry` with `can_size_overlay` set to true to determine its \
             size."
        );
        panic!(
            "Overlay was given infinite constraints and cannot be sized by a suitable child. \
             The constraints given to the overlay would result in an illegal infinite size. To \
             avoid that, the Overlay tried to size itself to one of its children, but no \
             suitable non-positioned child that belongs to an OverlayEntry with \
             can_size_overlay set to true could be found."
        );
    }
}

impl ContainerRenderObjectMixin for RenderTheater {
    type ChildType = AnyRenderBox;
    type ParentDataType = TheaterParentData;

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

impl RenderTheaterMixin for RenderTheater {
    fn theater(self: RenderHandle<Self>, _app: &App) -> RenderHandle<RenderTheater> {
        self
    }

    fn children_in_paint_order(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderBox> {
        let mut children = Vec::new();
        let mut child = self.first_onstage_child(app);
        while let Some(current) = child {
            children.push(current);
            let deferred = current
                .as_object()
                .parent_data_of::<TheaterParentData>(app)
                .paint_order_children(app);
            children.extend(deferred.into_iter().map(|child| child.as_box()));
            child = self.child_after(app, current);
        }
        children
    }

    fn children_in_hit_test_order(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderBox> {
        let mut children = Vec::new();
        let mut child = self.last_onstage_child(app);
        let mut child_left = self.child_count(app) - self.get(app).skip_count;
        while let Some(current) = child {
            let deferred = current
                .as_object()
                .parent_data_of::<TheaterParentData>(app)
                .hit_test_order_children(app);
            children.extend(deferred.into_iter().map(|child| child.as_box()));
            children.push(current);
            child_left -= 1;
            child = if child_left == 0 {
                None
            } else {
                self.child_before(app, current)
            };
        }
        children
    }
}

impl RenderObject for RenderTheater {
    reveal_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let mut size_determining_child = None;
        if !self.get(app).always_size_to_content && constraints.biggest().is_finite() {
            self.set_size(app, constraints.biggest());
        } else {
            let child = self.find_size_determining_child(app);
            size_determining_child = Some(child);
            self.get_mut(app).laying_out_size_determining_child = true;
            self.layout_child(app, child, constraints);
            self.get_mut(app).laying_out_size_determining_child = false;
            let size = child.size(app);
            self.set_size(app, size);
        }

        // Equivalent to the BoxConstraints RenderStack uses for StackFit.expand.
        let non_positioned_child_constraints = BoxConstraints::tight(self.size(app));
        for child in self.children_in_paint_order(app) {
            if Some(child) != size_determining_child {
                self.layout_child(app, child, non_positioned_child_constraints);
            }
        }
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        let mut child = self.first_child(app);
        while let Some(current) = child {
            visitor(current.as_object());
            current
                .as_object()
                .parent_data_of::<TheaterParentData>(app)
                .visit_overlay_portal_children_on_overlay_entry(app, visitor);
            child = self.child_after(app, current);
        }
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        ContainerRenderObjectMixin::did_attach(self, app, owner);
        let mut child = self.first_child(app);
        while let Some(current) = child {
            let deferred = current
                .as_object()
                .parent_data_of::<TheaterParentData>(app)
                .paint_order_children(app);
            for deferred_child in deferred {
                deferred_child.as_object().attach(app, owner);
            }
            child = self.child_after(app, current);
        }
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::did_detach(self, app);
        let mut child = self.first_child(app);
        while let Some(current) = child {
            let deferred = current
                .as_object()
                .parent_data_of::<TheaterParentData>(app)
                .paint_order_children(app);
            for deferred_child in deferred {
                deferred_child.as_object().detach(app);
            }
            child = self.child_after(app, current);
        }
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        let mut children = Vec::new();
        RenderObject::visit_children(self, app, &mut |child| children.push(child));
        for child in children {
            self.as_object().redepth_child(app, child);
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let clip_behavior = self.clip_behavior(app);
        if clip_behavior != Clip::None {
            let size = self.size(app);
            context.push_clip_rect(
                app,
                offset,
                Offset::ZERO & size,
                |app, context, offset| RenderTheaterMixin::paint(self, app, context, offset),
                clip_behavior,
            );
        } else {
            RenderTheaterMixin::paint(self, app, context, offset);
        }
    }
}

impl RenderBox for RenderTheater {
    reveal_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderTheaterMixin::setup_parent_data(self, app, child)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderTheaterMixin::hit_test_children(self, app, result, position)
    }
}

/// Dart's `_RenderTheaterMarker`: the [`RenderTheater`] and the entry state an
/// [`Overlay`]'s subtree builds under.
#[derive(Debug)]
struct RenderTheaterMarker {
    theater: RenderHandle<RenderTheater>,
    overlay_entry_widget_state: Handle<OverlayEntryWidgetState>,
    child: WidgetRef,
}

/// The two fields of a [`RenderTheaterMarker`], copied out of the tree.
#[derive(Clone, Copy)]
struct TheaterLocation {
    theater: RenderHandle<RenderTheater>,
    overlay_entry_widget_state: Handle<OverlayEntryWidgetState>,
}

impl RenderTheaterMarker {
    fn of(app: &mut App, context: BuildContext, target_root_overlay: bool) -> TheaterLocation {
        RenderTheaterMarker::maybe_of(app, context, target_root_overlay, true).expect(
            "No Overlay widget found. This widget requires an Overlay widget ancestor. An \
             overlay lets widgets float on top of other widget children. To introduce an \
             Overlay widget, you can either directly include one, or use a widget that \
             contains an Overlay itself, such as a Navigator or WidgetsApp.",
        )
    }

    fn maybe_of(
        app: &mut App,
        context: BuildContext,
        target_root_overlay: bool,
        create_dependency: bool,
    ) -> Option<TheaterLocation> {
        let read = |app: &App, element: AnyElement| {
            let widget = element.widget(app);
            let marker = downcast_widget::<RenderTheaterMarker>(&**widget)
                .expect("a _RenderTheaterMarker element holds a _RenderTheaterMarker");
            TheaterLocation {
                theater: marker.theater,
                overlay_entry_widget_state: marker.overlay_entry_widget_state,
            }
        };

        if target_root_overlay {
            let nearest =
                context.get_element_for_inherited_widget_of_exact_type::<RenderTheaterMarker>(app);
            let ancestor = RenderTheaterMarker::root_marker_of(app, nearest)?;
            if create_dependency {
                context.depend_on_inherited_element(app, ancestor, None);
            }
            return Some(read(app, ancestor));
        }

        if create_dependency {
            context.depend_on_inherited_widget_of_exact_type::<RenderTheaterMarker>(app)?;
        }
        let element =
            context.get_element_for_inherited_widget_of_exact_type::<RenderTheaterMarker>(app)?;
        Some(read(app, element))
    }

    fn root_marker_of(app: &App, marker_element: Option<AnyElement>) -> Option<AnyElement> {
        let marker_element = marker_element?;
        let mut ancestor = None;
        marker_element.visit_ancestor_elements(app, &mut |element| {
            ancestor =
                element.get_element_for_inherited_widget_of_exact_type::<RenderTheaterMarker>(app);
            false
        });
        match ancestor {
            None => Some(marker_element),
            Some(ancestor) => RenderTheaterMarker::root_marker_of(app, Some(ancestor)),
        }
    }
}

impl InheritedWidget for RenderTheaterMarker {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &RenderTheaterMarker) -> bool {
        old_widget.theater != self.theater
            || old_widget.overlay_entry_widget_state != self.overlay_entry_widget_state
    }
}

// * OverlayPortal Implementation
//  OverlayPortal is inspired by the
//  [flutter_portal](https://pub.dev/packages/flutter_portal) package.
//
// ** RenderObject hierarchy
// The widget works by inserting its overlay child's render subtree directly under
// [`Overlay`]'s render object ([`RenderTheater`]).
//
// To ensure the overlay child render subtree does not do layout twice, the subtree must only
// perform layout after both its RenderTheater and the OverlayPortal's render object
// (RenderLayoutSurrogateProxyBox) have finished layout. This is handled by
// RenderDeferredLayoutBox.
//
// ** Z-Index of an overlay child
// OverlayEntryLocation is a (private) interface that allows an OverlayPortal to insert its
// overlay child into a specific Overlay, as well as specifying the paint order between the
// overlay child and other children of the RenderTheater.
//
// Since OverlayPortal is only allowed to target ancestor Overlays (RenderTheater must finish
// doing layout before RenderDeferredLayoutBox), the RenderTheater should typically be
// acquired using an InheritedWidget (currently, RenderTheaterMarker) in case the
// OverlayPortal gets reparented.

thread_local! {
    /// Dart's `OverlayPortalController._wallTime`.
    static WALL_TIME: Cell<i64> = const { Cell::new(i64::MIN) };
}

/// A class to show, hide and bring to top an [`OverlayPortal`]'s overlay child in the target
/// [`Overlay`].
///
/// An [`OverlayPortalController`] can only be given to at most one [`OverlayPortal`] at a
/// time. When an [`OverlayPortalController`] is moved from one [`OverlayPortal`] to another,
/// its [`is_showing`](Self::is_showing) state does not carry over.
///
/// [`show`](Self::show) and [`hide`](Self::hide) can be called even before the controller is
/// assigned to any [`OverlayPortal`], but they typically should not be called while the
/// widget tree is being rebuilt.
pub struct OverlayPortalController {
    attach_target: Option<Handle<OverlayPortalState>>,
    // A separate z_order_index to allow `show` or `hide` to be called when the controller is
    // not yet attached. Once this controller is attached, the attach target's z_order_index
    // will be used as the source of truth, and this variable will be set to None.
    z_order_index: Option<i64>,
    debug_label: Option<String>,
}

impl OverlayPortalController {
    /// Creates an [`OverlayPortalController`], optionally with a `debug_label`.
    pub fn new(app: &mut App, debug_label: Option<String>) -> Handle<OverlayPortalController> {
        app.create(OverlayPortalController {
            attach_target: None,
            z_order_index: None,
            debug_label,
        })
    }

    // Returns a unique and monotonically increasing timestamp that represents now.
    //
    // The value this method returns increments after each call.
    fn now(self: Handle<Self>, app: &App) -> i64 {
        let now = WALL_TIME.get() + 1;
        WALL_TIME.set(now);
        debug_assert!(app.get(self).z_order_index.is_none_or(|index| index < now));
        debug_assert!(
            app.get(self)
                .attach_target
                .and_then(|target| app.get(target).z_order_index)
                .is_none_or(|index| index < now)
        );
        now
    }

    /// Show the overlay child of the [`OverlayPortal`] this controller is attached to, at the
    /// top of the target [`Overlay`].
    ///
    /// When there is more than one [`OverlayPortal`] that targets the same [`Overlay`], the
    /// overlay child of the last [`OverlayPortal`] to have called
    /// [`show`](Self::show) appears at the top level, unobstructed.
    ///
    /// If [`is_showing`](Self::is_showing) is already true, calling this method brings the
    /// overlay child it controls to the top.
    ///
    /// This method should typically not be called while the widget tree is being rebuilt.
    pub fn show(self: Handle<Self>, app: &mut App) {
        let now = self.now(app);
        match app.get(self).attach_target {
            Some(state) => state.show(app, now),
            None => app.get_mut(self).z_order_index = Some(now),
        }
    }

    /// Hide the [`OverlayPortal`]'s overlay child.
    ///
    /// Once hidden, the overlay child will be removed from the widget tree the next time the
    /// widget tree rebuilds, and stateful widgets in the overlay child may lose state as a
    /// result.
    ///
    /// This method should typically not be called while the widget tree is being rebuilt.
    pub fn hide(self: Handle<Self>, app: &mut App) {
        match app.get(self).attach_target {
            Some(state) => state.hide(app),
            None => {
                debug_assert!(app.get(self).z_order_index.is_some());
                app.get_mut(self).z_order_index = None;
            }
        }
    }

    /// Whether the associated [`OverlayPortal`] should build and show its overlay child,
    /// using its [`overlay_child_builder`](OverlayPortal::overlay_child_builder).
    pub fn is_showing(self: Handle<Self>, app: &App) -> bool {
        match app.get(self).attach_target {
            Some(state) => app.get(state).z_order_index.is_some(),
            None => app.get(self).z_order_index.is_some(),
        }
    }

    /// Convenience method for toggling the current [`is_showing`](Self::is_showing) status.
    ///
    /// This method should typically not be called while the widget tree is being rebuilt.
    pub fn toggle(self: Handle<Self>, app: &mut App) {
        if self.is_showing(app) {
            self.hide(app);
        } else {
            self.show(app);
        }
    }
}

impl Debug for OverlayPortalController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OverlayPortalController")?;
        if let Some(label) = &self.debug_label {
            write!(f, "({label})")?;
        }
        if self.attach_target.is_none() {
            write!(f, " DETACHED")?;
        }
        Ok(())
    }
}

/// The location of the [`Overlay`] that an [`OverlayPortal`] renders its overlay child on.
///
/// This is typically used in [`OverlayPortal`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayChildLocation {
    /// The [`OverlayPortal`] renders its overlay child on the closest ancestor [`Overlay`]
    /// above the widget tree.
    NearestOverlay,

    /// The [`OverlayPortal`] renders its overlay child on the root [`Overlay`] above the
    /// widget tree.
    ///
    /// In case of multi-view apps, the root [`Overlay`] refers to the first Overlay below the
    /// `View`.
    RootOverlay,
}

/// A widget that renders its overlay child on an [`Overlay`].
///
/// The overlay child is initially hidden until [`OverlayPortalController::show`] is called on
/// the associated [`controller`](Self::controller). The [`OverlayPortal`] uses
/// [`overlay_child_builder`](Self::overlay_child_builder) to build its overlay child and
/// renders it on the specified [`Overlay`] as if it was inserted using an [`OverlayEntry`],
/// while it can depend on the same set of `InheritedWidget`s that this widget can depend on.
///
/// This widget requires an [`Overlay`] ancestor in the widget tree when its overlay child is
/// showing. The overlay child is rendered by the [`Overlay`] ancestor, not by the widget
/// itself. This allows the overlay child to float above other widgets, independent of its
/// position in the widget tree.
///
/// When [`OverlayPortalController::hide`] is called, the widget built using
/// [`overlay_child_builder`](Self::overlay_child_builder) will be removed from the widget
/// tree the next time the widget rebuilds. Stateful descendants in the overlay child subtree
/// may lose state as a result.
///
/// ### Paint Order
///
/// In an [`Overlay`], an overlay child is painted after the [`OverlayEntry`] associated with
/// its [`OverlayPortal`] (that is, the [`OverlayEntry`] closest to the [`OverlayPortal`] in
/// the widget tree, which usually represents the enclosing `Route`), and before the next
/// [`OverlayEntry`].
///
/// When an [`OverlayEntry`] has multiple associated [`OverlayPortal`]s, the paint order
/// between their overlay children is the order in which [`OverlayPortalController::show`] was
/// called. The last [`OverlayPortal`] to have called `show` gets to paint its overlay child
/// in the foreground.
///
/// ### Differences between [`OverlayPortal`] and [`OverlayEntry`]
///
/// The main difference between [`OverlayEntry`] and [`OverlayPortal`] is that [`OverlayEntry`]
/// builds its widget subtree as a child of the target [`Overlay`], while [`OverlayPortal`]
/// uses [`overlay_child_builder`](Self::overlay_child_builder) to build a child widget of
/// itself. This allows [`OverlayPortal`]'s overlay child to depend on the same set of
/// `InheritedWidget`s as [`OverlayPortal`], and it's also guaranteed that the overlay child
/// will not outlive its [`OverlayPortal`].
///
/// On the other hand, [`OverlayPortal`]'s implementation is more complex. For instance, it
/// does a bit more work than a regular widget during global key reparenting. If the content
/// to be shown on the [`Overlay`] doesn't benefit from being a part of [`OverlayPortal`]'s
/// subtree, consider using an [`OverlayEntry`] instead.
///
/// See also:
///
///  * [`OverlayEntry`], an alternative API for inserting widgets into an [`Overlay`].
///  * `Positioned`, which can be used to size and position the overlay child in relation to
///    the target [`Overlay`]'s boundaries.
pub struct OverlayPortal {
    /// See [`Widget::key`].
    pub key: Option<KeyRef>,
    /// The controller to show, hide and bring to top the overlay child.
    pub controller: Handle<OverlayPortalController>,
    /// A [`WidgetBuilder`] used to build a widget below this widget in the tree, that renders
    /// on the closest [`Overlay`].
    ///
    /// The said widget will only be built and shown in the closest [`Overlay`] once
    /// [`OverlayPortalController::show`] is called on the associated
    /// [`controller`](Self::controller). It will be painted in front of the [`OverlayEntry`]
    /// closest to this widget in the widget tree (which is usually the enclosing `Route`).
    ///
    /// The built overlay child widget is inserted below this widget in the widget tree,
    /// allowing it to depend on `InheritedWidget`s above it, and be notified when the
    /// `InheritedWidget`s change.
    ///
    /// Unlike [`child`](Self::child), the built overlay child can visually extend outside the
    /// bounds of this widget without being clipped, and receive hit-test events outside of
    /// this widget's bounds, as long as it does not extend outside of the [`Overlay`] on
    /// which it is rendered.
    pub overlay_child_builder: WidgetBuilder,
    /// A widget below this widget in the tree.
    pub child: Option<WidgetRef>,
    /// The [`Overlay`] that the widget returned from
    /// [`overlay_child_builder`](Self::overlay_child_builder) is attached to.
    pub overlay_location: OverlayChildLocation,
}

impl OverlayPortal {
    /// Creates an [`OverlayPortal`] that renders the widget `overlay_child_builder` builds on
    /// the closest [`Overlay`] when [`OverlayPortalController::show`] is called.
    ///
    /// The [`overlay_location`](Self::overlay_location) sets which [`Overlay`] this widget
    /// attaches the widget returned by `overlay_child_builder` to. Defaults to
    /// [`OverlayChildLocation::NearestOverlay`].
    pub fn new(
        controller: Handle<OverlayPortalController>,
        overlay_child_builder: impl Fn(&mut App, BuildContext) -> WidgetRef + 'static,
    ) -> OverlayPortal {
        OverlayPortal {
            key: None,
            controller,
            overlay_child_builder: Rc::new(overlay_child_builder),
            child: None,
            overlay_location: OverlayChildLocation::NearestOverlay,
        }
    }

    /// Creates an [`OverlayPortal`] that renders the widget `overlay_child_builder` builds on
    /// the root [`Overlay`] when [`OverlayPortalController::show`] is called.
    ///
    /// Deprecated in Flutter after v3.33.0-0.0.pre: use [`OverlayPortal::new`] with
    /// [`overlay_location`](Self::overlay_location) set to
    /// [`OverlayChildLocation::RootOverlay`] instead.
    #[deprecated(note = "Use OverlayPortal::new with an overlay_location of \
                OverlayChildLocation::RootOverlay instead. This feature was deprecated \
                after v3.33.0-0.0.pre.")]
    pub fn targets_root_overlay(
        controller: Handle<OverlayPortalController>,
        overlay_child_builder: impl Fn(&mut App, BuildContext) -> WidgetRef + 'static,
    ) -> OverlayPortal {
        OverlayPortal {
            key: None,
            controller,
            overlay_child_builder: Rc::new(overlay_child_builder),
            child: None,
            overlay_location: OverlayChildLocation::RootOverlay,
        }
    }

    /// Creates an [`OverlayPortal`] that renders the widget `overlay_child_builder` builds on
    /// the closest [`Overlay`] when [`OverlayPortalController::show`] is called.
    ///
    /// Developers can use `overlay_child_builder` to configure the overlay child based on the
    /// size and the location of [`child`](Self::child) within the target [`Overlay`], as well
    /// as the size of the [`Overlay`] itself. This allows the overlay child to, for example,
    /// always follow [`child`](Self::child) and at the same time resize itself based on how
    /// close it is to the edges of the [`Overlay`].
    ///
    /// The `overlay_child_builder` callback is called during layout. To ensure the paint
    /// transform of [`child`](Self::child) in relation to the target [`Overlay`] is
    /// up-to-date by then, all render objects between the [`OverlayPortal`] and the target
    /// [`Overlay`] must establish their paint transform during the layout phase, which most
    /// render objects do.
    ///
    /// The [`overlay_location`](Self::overlay_location) sets which [`Overlay`] this widget
    /// attaches the widget returned by `overlay_child_builder` to. Defaults to
    /// [`OverlayChildLocation::NearestOverlay`].
    pub fn overlay_child_layout_builder(
        controller: Handle<OverlayPortalController>,
        overlay_child_builder: impl Fn(&mut App, BuildContext, OverlayChildLayoutInfo) -> WidgetRef
        + 'static,
    ) -> OverlayPortal {
        let builder: OverlayChildLayoutBuilder = Rc::new(overlay_child_builder);
        OverlayPortal::new(controller, move |_app, _context| {
            OverlayChildLayoutBuilderWidget {
                builder: Rc::clone(&builder),
            }
            .into_widget()
        })
    }

    /// Dart `OverlayPortal(key:)`.
    pub fn key(mut self, key: KeyRef) -> OverlayPortal {
        self.key = Some(key);
        self
    }

    /// Dart `OverlayPortal(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> OverlayPortal {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `OverlayPortal(overlayLocation:)`.
    pub fn overlay_location(mut self, overlay_location: OverlayChildLocation) -> OverlayPortal {
        self.overlay_location = overlay_location;
        self
    }
}

impl Debug for OverlayPortal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OverlayPortal")
            .field("overlayLocation", &self.overlay_location)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for OverlayPortal {
    type State = OverlayPortalState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> OverlayPortalState {
        OverlayPortalState {
            state: StateData::new(),
            z_order_index: None,
            child_model_may_have_changed: true,
            location_cache: None,
        }
    }
}

/// Dart's `_OverlayPortalState`.
pub struct OverlayPortalState {
    state: StateData<OverlayPortal>,
    z_order_index: Option<i64>,
    // The location of the overlay child within the overlay. This object will be used as the
    // slot of the overlay child widget.
    //
    // The developer must call `show` to reveal the overlay so we can get a unique timestamp of
    // the user interaction for determining the z-index of the overlay child in the overlay.
    //
    // Avoid invalidating the cache if possible, since the framework uses identity to compare
    // slots, and OverlayEntryLocation is mutable. Changing slots can be relatively slow.
    child_model_may_have_changed: bool,
    location_cache: Option<Handle<OverlayEntryLocation>>,
}

impl OverlayPortalState {
    fn is_the_same_location(
        app: &App,
        location_cache: Handle<OverlayEntryLocation>,
        marker: &TheaterLocation,
    ) -> bool {
        let cached = app.get(location_cache);
        cached.child_model == marker.overlay_entry_widget_state && cached.theater == marker.theater
    }

    fn get_location(
        self: Handle<Self>,
        app: &mut App,
        z_order_index: i64,
        overlay_location: OverlayChildLocation,
    ) -> Handle<OverlayEntryLocation> {
        let cached_location = app.get(self).location_cache;
        let context = self.context(app);
        let target_root_overlay = overlay_location == OverlayChildLocation::RootOverlay;
        // Dart's `late final marker`: reading it depends on an inherited widget, which the
        // valid-cache path must not do.
        let mut marker = None;
        let is_cache_valid = cached_location.is_some_and(|cached| {
            !app.get(self).child_model_may_have_changed || {
                let found = RenderTheaterMarker::of(app, context, target_root_overlay);
                marker = Some(found);
                OverlayPortalState::is_the_same_location(app, cached, &found)
            }
        });
        app.get_mut(self).child_model_may_have_changed = false;
        if is_cache_valid {
            let cached = cached_location.expect("a valid cache holds a location");
            debug_assert!(app.get(cached).z_order_index == z_order_index);
            debug_assert!(cached.debug_is_location_valid(app));
            return cached;
        }
        // Otherwise invalidate the cache and create a new location.
        if let Some(cached) = cached_location {
            cached.debug_mark_location_invalid(app);
        }
        let marker = match marker {
            Some(marker) => marker,
            None => RenderTheaterMarker::of(app, context, target_root_overlay),
        };
        let new_location = OverlayEntryLocation::new(
            app,
            z_order_index,
            marker.overlay_entry_widget_state,
            marker.theater,
        );
        app.get_mut(self).location_cache = Some(new_location);
        new_location
    }

    fn setup_controller(
        self: Handle<Self>,
        app: &mut App,
        controller: Handle<OverlayPortalController>,
    ) {
        debug_assert!(
            app.get(controller).attach_target == Some(self)
                || !app
                    .get(controller)
                    .attach_target
                    .is_some_and(|target| target.mounted(app)),
            "Failed to attach the controller: it is already attached to another OverlayPortal."
        );
        let controller_z_order_index = app.get(controller).z_order_index;
        let z_order_index = app.get(self).z_order_index;
        let takes_controller_index = match (controller_z_order_index, z_order_index) {
            (_, None) => true,
            (Some(controller_index), Some(index)) => controller_index > index,
            (None, Some(_)) => false,
        };
        if takes_controller_index {
            app.get_mut(self).z_order_index = controller_z_order_index;
        }
        let controller_state = app.get_mut(controller);
        controller_state.z_order_index = None;
        controller_state.attach_target = Some(self);
    }

    fn show(self: Handle<Self>, app: &mut App, z_order_index: i64) {
        debug_assert!(
            SchedulerBinding::scheduler_phase(app) != SchedulerPhase::PersistentCallbacks,
            "OverlayPortalController::show should not be called during build."
        );
        self.set_state(app, |state| state.z_order_index = Some(z_order_index));
        if let Some(cached) = app.get(self).location_cache {
            cached.debug_mark_location_invalid(app);
        }
        app.get_mut(self).location_cache = None;
    }

    fn hide(self: Handle<Self>, app: &mut App) {
        debug_assert!(
            SchedulerBinding::scheduler_phase(app) != SchedulerPhase::PersistentCallbacks,
            "OverlayPortalController::hide should not be called during build."
        );
        self.set_state(app, |state| state.z_order_index = None);
        if let Some(cached) = app.get(self).location_cache {
            cached.debug_mark_location_invalid(app);
        }
        app.get_mut(self).location_cache = None;
    }
}

impl State for OverlayPortalState {
    type Widget = OverlayPortal;

    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = self.widget(app).controller;
        self.setup_controller(app, controller);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).child_model_may_have_changed = true;
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &OverlayPortal) {
        let (old_location, old_controller) = (old_widget.overlay_location, old_widget.controller);
        let widget = self.widget(app);
        let (new_location, new_controller) = (widget.overlay_location, widget.controller);
        let state = app.get_mut(self);
        state.child_model_may_have_changed =
            state.child_model_may_have_changed || old_location != new_location;
        if old_controller != new_controller {
            app.get_mut(old_controller).attach_target = None;
            self.setup_controller(app, new_controller);
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        debug_assert!(app.get(self.widget(app).controller).attach_target == Some(self));
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let controller = self.widget(app).controller;
        app.get_mut(controller).attach_target = None;
        if let Some(cached) = app.get(self).location_cache {
            cached.debug_mark_location_invalid(app);
        }
        app.get_mut(self).location_cache = None;
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let child = self.widget(app).child.clone();
        let Some(z_order_index) = app.get(self).z_order_index else {
            return RawOverlayPortal {
                overlay_location: None,
                overlay_child: None,
                child,
            }
            .into_widget();
        };

        let widget_location = self.widget(app).overlay_location;
        let overlay_location = self.get_location(app, z_order_index, widget_location);
        let child_model_context = app.get(overlay_location).child_model.context(app);
        let overlay_data = MediaQuery::of(app, child_model_context);
        let data = MediaQuery::of(app, context)
            .copy_with()
            .padding(overlay_data.padding)
            .view_insets(overlay_data.view_insets)
            .view_padding(overlay_data.view_padding);
        let builder = Rc::clone(&self.widget(app).overlay_child_builder);
        RawOverlayPortal {
            overlay_location: Some(overlay_location),
            overlay_child: Some(
                DeferredLayout {
                    child: MediaQuery::new(data, Builder { key: None, builder }).into_widget(),
                }
                .into_widget(),
            ),
            child,
        }
        .into_widget()
    }
}

/// A location in an [`Overlay`].
///
/// An [`OverlayEntryLocation`] determines the [`Overlay`] the associated [`OverlayPortal`]
/// should put its overlay child onto, as well as the overlay child's paint order in relation
/// to other contents painted on the [`Overlay`].
//
// An OverlayEntryLocation is a cursor pointing to a location in a particular Overlay's child
// model, and provides methods to insert/remove/move a RenderDeferredLayoutBox to/from its
// target theater.
//
// The occupant (a `RenderBox`) will be painted above the associated OverlayEntry, but below
// the OverlayEntry above that OverlayEntry.
//
// Additionally, `reattach_from_layout_surrogate` and `detach_from_layout_surrogate` are
// called when the overlay child's RawOverlayPortalElement activates/deactivates (for instance,
// during global key reparenting). RawOverlayPortalElement removes its overlay child's render
// object from the target RenderTheater when it deactivates and puts it back on activation.
// These two methods can be used to "hide" a child in the child model without removing it, when
// the child is expensive/difficult to re-insert at the correct location on activation.
//
// ### Equality
//
// An OverlayEntryLocation is used as an Element's slot. These three parts uniquely identify a
// place in an overlay's child model:
// - theater
// - child_model (the OverlayEntry)
// - z_order_index
//
// Since it is mutable, slots compare it by identity, and the same OverlayEntryLocation must
// not be used to represent more than one location.
struct OverlayEntryLocation {
    z_order_index: i64,
    child_model: Handle<OverlayEntryWidgetState>,
    theater: RenderHandle<RenderTheater>,
    overlay_child_render_box: Option<RenderHandle<RenderDeferredLayoutBox>>,
    // The identity a `Slot::Custom` compares: created with the location and never replaced.
    slot_token: Option<Rc<dyn Any>>,
    // Dart records the `StackTrace` of the first `_debugMarkLocationInvalid` call; only the
    // flag is kept here (see `PORTING.md`). The effect is not reversible: once marked invalid,
    // this object cannot be marked valid again.
    debug_location_invalid: bool,
}

impl OverlayEntryLocation {
    fn new(
        app: &mut App,
        z_order_index: i64,
        child_model: Handle<OverlayEntryWidgetState>,
        theater: RenderHandle<RenderTheater>,
    ) -> Handle<OverlayEntryLocation> {
        let this = app.create(OverlayEntryLocation {
            z_order_index,
            child_model,
            theater,
            overlay_child_render_box: None,
            slot_token: None,
            debug_location_invalid: false,
        });
        app.get_mut(this).slot_token = Some(Rc::new(this));
        this
    }

    /// The slot the overlay child element occupies: this location, compared by identity.
    fn slot(self: Handle<Self>, app: &App) -> Slot {
        Slot::Custom(Rc::clone(
            app.get(self)
                .slot_token
                .as_ref()
                .expect("the token is created with the location"),
        ))
    }

    /// The location a slot names, if it names one.
    fn from_slot(slot: &Slot) -> Option<Handle<OverlayEntryLocation>> {
        match slot {
            Slot::Custom(token) => token
                .downcast_ref::<Handle<OverlayEntryLocation>>()
                .copied(),
            _ => None,
        }
    }

    fn add_to_child_model(
        self: Handle<Self>,
        app: &mut App,
        child: RenderHandle<RenderDeferredLayoutBox>,
    ) {
        debug_assert!(
            app.get(self).overlay_child_render_box.is_none(),
            "Failed to add the child: this location is already occupied."
        );
        app.get_mut(self).overlay_child_render_box = Some(child);
        let (child_model, theater) = {
            let location = app.get(self);
            (location.child_model, location.theater)
        };
        child_model.add(app, self);
        theater.mark_needs_paint(app);
    }

    fn remove_from_child_model(
        self: Handle<Self>,
        app: &mut App,
        child: RenderHandle<RenderDeferredLayoutBox>,
    ) {
        debug_assert!(app.get(self).overlay_child_render_box == Some(child));
        app.get_mut(self).overlay_child_render_box = None;
        let (child_model, theater) = {
            let location = app.get(self);
            (location.child_model, location.theater)
        };
        child_model.remove(app, self);
        theater.mark_needs_paint(app);
    }

    fn add_child(self: Handle<Self>, app: &mut App, child: RenderHandle<RenderDeferredLayoutBox>) {
        debug_assert!(self.debug_is_location_valid(app));
        self.add_to_child_model(app, child);
        let theater = app.get(self).theater;
        theater.add_deferred_child(app, child);
        debug_assert!(child.parent(app) == Some(theater.as_object()));
    }

    fn remove_child(
        self: Handle<Self>,
        app: &mut App,
        child: RenderHandle<RenderDeferredLayoutBox>,
    ) {
        // This call is allowed even when this location is disposed.
        self.remove_from_child_model(app, child);
        let theater = app.get(self).theater;
        theater.remove_deferred_child(app, child);
        debug_assert!(child.parent(app).is_none());
    }

    fn move_child(
        self: Handle<Self>,
        app: &mut App,
        child: RenderHandle<RenderDeferredLayoutBox>,
        from_location: Handle<OverlayEntryLocation>,
    ) {
        debug_assert!(from_location != self);
        debug_assert!(self.debug_is_location_valid(app));
        let from_theater = app.get(from_location).theater;
        let from_model = app.get(from_location).child_model;
        let theater = app.get(self).theater;

        if from_theater != theater {
            from_theater.remove_deferred_child(app, child);
            theater.add_deferred_child(app, child);
        }

        if from_model != app.get(self).child_model
            || app.get(from_location).z_order_index != app.get(self).z_order_index
        {
            from_location.remove_from_child_model(app, child);
            self.add_to_child_model(app, child);
        }
    }

    /// Undoes [`detach_from_layout_surrogate`](Self::detach_from_layout_surrogate) by adding
    /// the given `child` back to the theater.
    ///
    /// This is called when the [`OverlayPortal`] is activated. This call is allowed even when
    /// this location is invalidated.
    fn reattach_from_layout_surrogate(
        self: Handle<Self>,
        app: &mut App,
        child: RenderHandle<RenderDeferredLayoutBox>,
    ) {
        debug_assert!(
            app.get(self).overlay_child_render_box.is_none(),
            "failed to reattach: detach_from_layout_surrogate must be called before \
             reattach_from_layout_surrogate."
        );
        let theater = app.get(self).theater;
        theater.add_deferred_child(app, child);
        app.get_mut(self).overlay_child_render_box = Some(child);
    }

    /// Removes the given `child` from the theater but keeps it in the child list (unlike
    /// [`remove_child`](Self::remove_child)).
    ///
    /// This is typically called when the [`OverlayPortal`] deactivates. Since every render
    /// object in the render tree must be attached, when an [`OverlayPortal`] deactivates, it
    /// must remove the overlay child from the render tree instead of just detaching it.
    ///
    /// This call is allowed even when this location is invalidated.
    fn detach_from_layout_surrogate(
        self: Handle<Self>,
        app: &mut App,
        child: RenderHandle<RenderDeferredLayoutBox>,
    ) {
        let theater = app.get(self).theater;
        theater.remove_deferred_child(app, child);
        app.get_mut(self).overlay_child_render_box = None;
    }

    // Panics if this location is already invalidated and shouldn't be used as an OverlayPortal
    // slot. Must be used in asserts.
    //
    // Generally, `debug_assert!(location.debug_is_location_valid(app))` should be used to
    // prevent invalid accesses to an invalid location. Exceptions to this rule are
    // `remove_child` and `detach_from_layout_surrogate`, which are called when the
    // OverlayPortal is being removed from the widget tree and may use the location information
    // to perform cleanup tasks.
    //
    // Another exception is `reattach_from_layout_surrogate`, which is called shortly after the
    // OverlayPortal activates because it's possible that the widget subtree hasn't been
    // rebuilt at that point, so we'll have to re-attach the overlay child render object using
    // a potentially outdated location.
    fn debug_is_location_valid(self: Handle<Self>, app: &App) -> bool {
        assert!(
            !app.get(self).debug_location_invalid,
            "this OverlayPortal location is already disposed."
        );
        true
    }

    fn debug_mark_location_invalid(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.debug_is_location_valid(app));
        if cfg!(debug_assertions) {
            app.get_mut(self).debug_location_invalid = true;
        }
    }
}

/// Dart's `_OverlayPortal`: the render object widget an [`OverlayPortal`] builds.
#[derive(Debug)]
struct RawOverlayPortal {
    overlay_child: Option<WidgetRef>,
    /// A widget below this widget in the tree.
    child: Option<WidgetRef>,
    overlay_location: Option<Handle<OverlayEntryLocation>>,
}

impl RawOverlayPortal {
    /// The tree node: a render object widget with its own element, outside the `IntoWidget`
    /// kinds.
    fn into_widget(self) -> WidgetRef {
        debug_assert!(self.overlay_child.is_none() || self.overlay_location.is_some());
        Rc::new(self)
    }
}

impl Widget for RawOverlayPortal {
    fn key(&self) -> Option<&KeyRef> {
        None
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        RawOverlayPortalElement::create(app, this).as_element()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn widget_type(&self) -> std::any::TypeId {
        std::any::TypeId::of::<RawOverlayPortal>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

impl RenderObjectWidget for RawOverlayPortal {
    type RenderObject = RenderLayoutSurrogateProxyBox;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderLayoutSurrogateProxyBox::new(app, self.overlay_location).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderLayoutSurrogateProxyBox>,
    ) {
        render_object.get_mut(app).overlay_location = self.overlay_location;
    }
}

/// Dart's `_OverlayPortalElement`: it holds the regular child and the overlay child, whose
/// slot is its [`OverlayEntryLocation`].
struct RawOverlayPortalElement {
    element: ElementData,
    render_object_element: RenderObjectElementData,
    overlay_child: Option<AnyElement>,
    child: Option<AnyElement>,
}

impl RawOverlayPortalElement {
    fn create(app: &mut App, widget: WidgetRef) -> Handle<RawOverlayPortalElement> {
        debug_assert!(downcast_widget::<RawOverlayPortal>(&*widget).is_some());
        app.create(RawOverlayPortalElement {
            element: ElementData::new(widget),
            render_object_element: RenderObjectElementData::default(),
            overlay_child: None,
            child: None,
        })
    }

    fn update_children(self: Handle<Self>, app: &mut App) {
        let this = self.as_element();
        let (child_widget, overlay_child_widget, overlay_location) = {
            let widget = Self::widget_of(this.widget(app));
            (
                widget.child.clone(),
                widget.overlay_child.clone(),
                widget.overlay_location,
            )
        };
        let child = app.get(self).child;
        let child = this.update_child(app, child, child_widget, None);
        app.get_mut(self).child = child;
        let slot = overlay_location.map(|location| location.slot(app));
        let overlay_child = app.get(self).overlay_child;
        let overlay_child = this.update_child(app, overlay_child, overlay_child_widget, slot);
        app.get_mut(self).overlay_child = overlay_child;
    }

    fn surrogate(self: Handle<Self>, app: &App) -> RenderHandle<RenderLayoutSurrogateProxyBox> {
        self.typed_render_object(app)
    }
}

impl RenderObjectElementWidget for RawOverlayPortalElement {
    type Widget = RawOverlayPortal;

    fn widget_of(widget: &WidgetRef) -> &RawOverlayPortal {
        downcast_widget::<RawOverlayPortal>(&**widget)
            .expect("a RawOverlayPortalElement holds a RawOverlayPortal")
    }
}

impl RenderObjectElement for RawOverlayPortalElement {
    fn render_object_element_data(self: Handle<Self>, app: &App) -> &RenderObjectElementData {
        &app.get(self).render_object_element
    }

    fn render_object_element_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectElementData {
        &mut app.get_mut(self).render_object_element
    }
}

impl Element for RawOverlayPortalElement {
    crate::element_accessors!();

    const IS_RENDER_OBJECT_ELEMENT: bool = true;

    fn render_object(self: Handle<Self>, app: &App) -> Option<AnyRenderObject> {
        self.render_object_element_data(app).render_object()
    }

    fn render_object_attaching_child(self: Handle<Self>, _app: &App) -> Option<AnyElement> {
        None
    }

    fn debug_doing_build(self: Handle<Self>, app: &App) -> bool {
        self.render_object_element_data(app).debug_doing_build()
    }

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        RenderObjectElement::mount(self, app, parent, new_slot);
        self.update_children(app);
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        RenderObjectElement::update(self, app, new_widget);
        self.update_children(app);
    }

    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        // The overlay child element does not have a key because the DeferredLayout widget does
        // not take a Key, so only the regular child can be taken during global key
        // reparenting.
        debug_assert!(app.get(self).child == Some(child));
        app.get_mut(self).child = None;
    }

    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        let state = app.get(self);
        let (child, overlay_child) = (state.child, state.overlay_child);
        if let Some(child) = child {
            visitor(child);
        }
        if let Some(overlay_child) = overlay_child {
            visitor(overlay_child);
        }
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::perform_rebuild(self, app);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::deactivate(self, app);
    }

    fn unmount(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::unmount(self, app);
    }

    fn update_parent_data(self: Handle<Self>, app: &mut App, parent_data_element: AnyElement) {
        RenderObjectElement::update_parent_data(self, app, parent_data_element);
    }

    fn update_slot(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::update_slot(self, app, new_slot);
    }

    fn attach_render_object(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::attach_render_object(self, app, new_slot);
    }

    fn detach_render_object(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::detach_render_object(self, app);
    }

    fn insert_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        debug_assert!(child.parent(app).is_none());
        let surrogate = self.surrogate(app);
        match slot.as_ref().and_then(OverlayEntryLocation::from_slot) {
            Some(location) => {
                let deferred = child
                    .downcast::<RenderDeferredLayoutBox>(app)
                    .expect("an overlay child is a DeferredLayout");
                // The deferred child is assigned in DeferredLayout's create_render_object.
                debug_assert!(surrogate.get(app).deferred_layout_child == Some(deferred));
                location.add_child(app, deferred);
            }
            None => surrogate.set_child(app, Some(child.as_box().expect("a box child"))),
        }
    }

    // The DeferredLayout widget does not have a key so there will be no reparenting between
    // the overlay child and the child, hence the non-null slots.
    fn move_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        old_slot: Option<Slot>,
        new_slot: Option<Slot>,
    ) {
        let old_location = old_slot
            .as_ref()
            .and_then(OverlayEntryLocation::from_slot)
            .expect("an overlay child moves between locations");
        let new_location = new_slot
            .as_ref()
            .and_then(OverlayEntryLocation::from_slot)
            .expect("an overlay child moves between locations");
        debug_assert!(new_location.debug_is_location_valid(app));
        let deferred = child
            .downcast::<RenderDeferredLayoutBox>(app)
            .expect("an overlay child is a DeferredLayout");
        new_location.move_child(app, deferred, old_location);
    }

    fn remove_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        let surrogate = self.surrogate(app);
        let Some(location) = slot.as_ref().and_then(OverlayEntryLocation::from_slot) else {
            surrogate.set_child(app, None);
            return;
        };
        let deferred = child
            .downcast::<RenderDeferredLayoutBox>(app)
            .expect("an overlay child is a DeferredLayout");
        debug_assert!(surrogate.get(app).deferred_layout_child == Some(deferred));
        location.remove_child(app, deferred);
        surrogate.get_mut(app).deferred_layout_child = None;
    }
}

/// Dart's `_DeferredLayout`: the overlay child's own render object widget.
#[derive(Debug)]
struct DeferredLayout {
    // This widget must not be given a key: reparenting between the overlay child and the child
    // is not supported.
    child: WidgetRef,
}

impl DeferredLayout {
    fn layout_parent(
        app: &App,
        context: BuildContext,
    ) -> RenderHandle<RenderLayoutSurrogateProxyBox> {
        context
            .find_ancestor_render_object_of_type::<RenderLayoutSurrogateProxyBox>(app)
            .expect("a _DeferredLayout is built by an OverlayPortal")
    }
}

impl RenderObjectWidget for DeferredLayout {
    type RenderObject = RenderDeferredLayoutBox;

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let parent = DeferredLayout::layout_parent(app, context);
        let render_object = RenderDeferredLayoutBox::new(app, parent);
        parent.get_mut(app).deferred_layout_child = Some(render_object);
        render_object.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderDeferredLayoutBox>,
    ) {
        debug_assert!(
            render_object.get(app).layout_surrogate == DeferredLayout::layout_parent(app, context)
        );
        debug_assert!(
            DeferredLayout::layout_parent(app, context)
                .get(app)
                .deferred_layout_child
                == Some(render_object)
        );
    }
}

impl SingleChildRenderObjectWidget for DeferredLayout {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

// This render object must be a child of a [`RenderTheater`]. It guarantees that it only does
// layout after the sizes of the render objects from its `layout_surrogate` (which must be a
// descendant of this object's parent) through the parent RenderTheater are known. To this end:
//
// 1. It's a relayout boundary, and adding it to the RenderTheater as a child never dirties its
//    RenderTheater. Instead, it is always added to the PipelineOwner's dirty list when it
//    needs layout (even for the initial layout when it is first added to the tree).
//
// 2. Its layout is driven through `do_layout_from` such that `perform_layout` does not do
//    anything when it is called from the tree walk, preventing the parent RenderTheater from
//    laying out this subtree prematurely (but this object may still be resized). Instead,
//    `mark_needs_layout` is called from within `do_layout_from` to schedule a layout update for
//    this relayout boundary when needed.
//
// When invoked from `PipelineOwner::flush_layout`, this object behaves like an [`Overlay`] that
// has only one entry.
struct RenderDeferredLayoutBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    layout_surrogate: RenderHandle<RenderLayoutSurrogateProxyBox>,
    /// Whether this box's layout is currently being driven by the theater's or the layout
    /// surrogate's `perform_layout`.
    doing_layout_from_tree_walk: bool,
    debug_mutations_locked: bool,
}

impl RenderDeferredLayoutBox {
    fn new(
        app: &mut App,
        layout_surrogate: RenderHandle<RenderLayoutSurrogateProxyBox>,
    ) -> RenderHandle<RenderDeferredLayoutBox> {
        RenderHandle::new_box(
            app,
            RenderDeferredLayoutBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                layout_surrogate,
                doing_layout_from_tree_walk: false,
                debug_mutations_locked: false,
            },
        )
    }

    fn do_layout_from(
        self: RenderHandle<Self>,
        app: &mut App,
        treewalk_parent: AnyRenderObject,
        constraints: BoxConstraints,
    ) {
        debug_assert!(!self.get(app).doing_layout_from_tree_walk);
        self.get_mut(app).doing_layout_from_tree_walk = true;
        self.as_box().layout(app, constraints, false);
        debug_assert!(self.get(app).doing_layout_from_tree_walk);
        self.get_mut(app).doing_layout_from_tree_walk = false;
        debug_assert!(!self.as_object().debug_needs_layout(app));

        // Instead of laying out this subtree via tree walk, add it to the dirty list. This
        // ensures:
        //
        //  1. this node will be laid out by the PipelineOwner *after* the two nodes it depends
        //     on (the theater and the layout surrogate) are laid out, as it has a greater
        //     depth value than its dependencies.
        //
        //  2. when the deferred child's child starts to do layout, the nodes from the layout
        //     surrogate to the theater (exclusive) have finished doing layout, so the deferred
        //     child's child can read their sizes and (usually) compute the paint transform of
        //     the regular child within the Overlay.
        //
        // Invoking mark_needs_layout as a layout callback allows this node to be merged back
        // to the PipelineOwner's dirty list in the right order, if it's not already dirty,
        // such that this subtree does not get laid out twice.
        //
        // Flutter skips this when the subtree is clean and the constraints did not change;
        // `RenderObject`'s dirty flag is not readable outside `reveal-rendering` (see
        // `PORTING.md`), so the subtree is always re-scheduled.
        treewalk_parent.invoke_layout_callback(app, |app| {
            RenderBox::mark_needs_layout(self, app);
        });
    }
}

impl RenderObjectWithChildMixin for RenderDeferredLayoutBox {
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

impl RenderProxyBoxMixin for RenderDeferredLayoutBox {}

impl RenderTheaterMixin for RenderDeferredLayoutBox {
    fn theater(self: RenderHandle<Self>, app: &App) -> RenderHandle<RenderTheater> {
        self.parent(app)
            .and_then(|parent| parent.downcast::<RenderTheater>(app))
            .expect("the parent of a deferred layout box is a RenderTheater")
    }

    fn children_in_paint_order(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderBox> {
        self.child(app).into_iter().collect()
    }

    fn children_in_hit_test_order(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderBox> {
        RenderTheaterMixin::children_in_paint_order(self, app)
    }
}

impl RenderObject for RenderDeferredLayoutBox {
    reveal_rendering::render_object_accessors!();

    fn sized_by_parent(self: RenderHandle<Self>, _app: &App) -> bool {
        true
    }

    fn perform_resize(self: RenderHandle<Self>, app: &mut App) {
        let size = self.constraints(app).biggest();
        self.set_size(app, size);
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        debug_assert!(!self.get(app).debug_mutations_locked);
        if self.get(app).doing_layout_from_tree_walk {
            return;
        }
        if cfg!(debug_assertions) {
            self.get_mut(app).debug_mutations_locked = true;
        }
        // This method is directly being invoked from `PipelineOwner::flush_layout`, or from
        // the layout surrogate's perform_layout.
        debug_assert!(self.parent(app).is_some());
        let Some(child) = self.child(app) else {
            return;
        };
        let constraints = self.constraints(app);
        debug_assert!(constraints.is_tight());
        self.layout_child(app, child, constraints);
        if cfg!(debug_assertions) {
            self.get_mut(app).debug_mutations_locked = false;
        }
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        // The layout surrogate can be adopted after this box enters the theater. Until then,
        // the surrogate has no owner and cannot redepth this child. Once the surrogate is
        // adopted, its own redepth_children will restore the depth invariant.
        let surrogate = self.get(app).layout_surrogate;
        if surrogate.as_object().attached(app) {
            surrogate.as_object().redepth_child(app, self.as_object());
        }
        if let Some(child) = self.child(app) {
            self.as_object().redepth_child(app, child.as_object());
        }
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

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderTheaterMixin::paint(self, app, context, offset);
    }
}

impl RenderBox for RenderDeferredLayoutBox {
    reveal_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderTheaterMixin::setup_parent_data(self, app, child);
    }

    // Flutter's override translates by the child's parent-data offset, which is what
    // `RenderBox::apply_paint_transform` already does; `RenderProxyBoxMixin`'s no-op one is
    // not used here.

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderTheaterMixin::hit_test_children(self, app, result, position)
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

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

// A proxy box that makes sure its deferred layout child has a greater depth than itself.
//
// This render object also conditionally attaches and detaches the associated
// RenderDeferredLayoutBox when itself attaches and detaches from its PipelineOwner. This
// guarantees that the deferred box's attached status is always kept in sync with both the
// surrogate and its parent theater.
struct RenderLayoutSurrogateProxyBox {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    // This variable is set as soon as the DeferredLayout widget creates it, and it is only set
    // to None when the DeferredLayout widget is being removed from the tree.
    deferred_layout_child: Option<RenderHandle<RenderDeferredLayoutBox>>,
    overlay_location: Option<Handle<OverlayEntryLocation>>,
    debug_is_first_attach: bool,
    did_detach_deferred_child: bool,
}

impl RenderLayoutSurrogateProxyBox {
    fn new(
        app: &mut App,
        overlay_location: Option<Handle<OverlayEntryLocation>>,
    ) -> RenderHandle<RenderLayoutSurrogateProxyBox> {
        RenderHandle::new_box(
            app,
            RenderLayoutSurrogateProxyBox {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                deferred_layout_child: None,
                overlay_location,
                debug_is_first_attach: true,
                did_detach_deferred_child: false,
            },
        )
    }
}

impl RenderObjectWithChildMixin for RenderLayoutSurrogateProxyBox {
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

impl RenderProxyBoxMixin for RenderLayoutSurrogateProxyBox {}

impl RenderObject for RenderLayoutSurrogateProxyBox {
    reveal_rendering::render_object_accessors!();

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        if let Some(child) = self.child(app) {
            child.as_object().attach(app, owner);
        }
        // If did_attach runs after did_detach_deferred_child is set to true then it is always
        // safe to put the deferred child back because the theater must be an ancestor of both
        // render objects.
        if self.get(app).did_detach_deferred_child {
            self.get_mut(app).did_detach_deferred_child = false;
            let deferred = self
                .get(app)
                .deferred_layout_child
                .expect("a detached deferred child was recorded");
            debug_assert!(!self.get(app).debug_is_first_attach);
            let location = self
                .get(app)
                .overlay_location
                .expect("a deferred child has a location");
            location.reattach_from_layout_surrogate(app, deferred);
        }
        if cfg!(debug_assertions) {
            self.get_mut(app).debug_is_first_attach = false;
        }
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        // Detaches the deferred child if this node is being detached, but only if the theater
        // isn't already detached (so the deferred child will be detached by the theater).
        if let Some(deferred) = self.get(app).deferred_layout_child
            && deferred.theater(app).as_object().attached(app)
        {
            let location = self
                .get(app)
                .overlay_location
                .expect("a deferred child has a location");
            location.detach_from_layout_surrogate(app, deferred);
            self.get_mut(app).did_detach_deferred_child = true;
        }
        if let Some(child) = self.child(app) {
            child.as_object().detach(app);
        }
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        if let Some(child) = self.child(app) {
            self.as_object().redepth_child(app, child.as_object());
        }
        // If the child is not attached yet, this method will be invoked by the child's real
        // parent (the theater) when it becomes attached.
        if let Some(deferred) = self.get(app).deferred_layout_child
            && deferred.as_object().attached(app)
        {
            self.as_object().redepth_child(app, deferred.as_object());
        }
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
        let Some(deferred_child) = self.get(app).deferred_layout_child else {
            return;
        };
        // To make sure all ancestors' perform_layout calls have returned when the deferred
        // child does layout, the deferred child needs to be put in the dirty list if it is
        // dirty, and the deferred child subtree made unreachable via the layout tree walk.
        let theater = deferred_child.theater(app);
        // If the theater is laying out the size-determining child, its size is not available
        // yet. Since the theater always lays out the size-determining child first and the
        // deferred child can never be size-determining, this method does not have to do
        // anything: the theater will update the constraints of the deferred child and resize
        // it or put it in the dirty list if needed.
        if !theater.get(app).laying_out_size_determining_child {
            let theater_constraints = theater.constraints(app);
            let box_size = if theater_constraints.biggest().is_finite() {
                theater_constraints.biggest()
            } else {
                theater.size(app)
            };
            deferred_child.do_layout_from(app, self.as_object(), BoxConstraints::tight(box_size));
        }
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

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderProxyBoxMixin::paint(self, app, context, offset);
    }
}

impl RenderBox for RenderLayoutSurrogateProxyBox {
    reveal_rendering::render_box_accessors!();

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

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

/// Dart's `_OverlayChildLayoutBuilder`: the widget
/// [`OverlayPortal::overlay_child_layout_builder`] builds as its overlay child.
///
/// Dart derives it from `AbstractLayoutBuilder<OverlayChildLayoutInfo>` and reuses
/// `layout_builder.dart`'s element; the crate's `LayoutBuilder` is not generic over the
/// layout information, so this is a concrete pair of its own (see `PORTING.md`).
struct OverlayChildLayoutBuilderWidget {
    builder: OverlayChildLayoutBuilder,
}

impl OverlayChildLayoutBuilderWidget {
    /// The tree node: a render object widget with its own element, outside the `IntoWidget`
    /// kinds.
    fn into_widget(self) -> WidgetRef {
        Rc::new(self)
    }

    /// Whether the builder needs to be called again even if the layout information is the
    /// same.
    fn update_should_rebuild(&self, old_widget: &OverlayChildLayoutBuilderWidget) -> bool {
        let _ = old_widget;
        true
    }
}

impl Debug for OverlayChildLayoutBuilderWidget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OverlayChildLayoutBuilder")
            .finish_non_exhaustive()
    }
}

impl Widget for OverlayChildLayoutBuilderWidget {
    fn key(&self) -> Option<&KeyRef> {
        None
    }

    fn create_element(&self, app: &mut App, this: WidgetRef) -> AnyElement {
        OverlayChildLayoutBuilderElement::create(app, this).as_element()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn widget_type(&self) -> std::any::TypeId {
        std::any::TypeId::of::<OverlayChildLayoutBuilderWidget>()
    }

    fn kind(&self) -> WidgetKind {
        WidgetKind::Other
    }
}

impl RenderObjectWidget for OverlayChildLayoutBuilderWidget {
    type RenderObject = RenderOverlayChildLayoutBuilder;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderOverlayChildLayoutBuilder::new(app).as_object()
    }

    // update_render_object is redundant with the logic in the element below.
    fn update_render_object(
        &self,
        _app: &mut App,
        _context: BuildContext,
        _render_object: RenderHandle<RenderOverlayChildLayoutBuilder>,
    ) {
    }

    fn did_unmount_render_object(
        &self,
        app: &mut App,
        render_object: RenderHandle<RenderOverlayChildLayoutBuilder>,
    ) {
        // Flutter cancels the per-frame callback in the render object's `dispose`, which is
        // not an override point here.
        render_object.cancel_frame_callback(app);
    }
}

/// The element of an [`OverlayChildLayoutBuilderWidget`]: it builds its child from within the
/// render object's layout, as `LayoutBuilderElement` does.
struct OverlayChildLayoutBuilderElement {
    element: ElementData,
    render_object_element: RenderObjectElementData,
    child: Option<AnyElement>,
    /// Dart's `late final _buildScope`, created right after the element so its
    /// `scheduleRebuild` can name it.
    build_scope: Option<Handle<BuildScope>>,
    deferred_callback_scheduled: bool,
    previous_layout_info: Option<OverlayChildLayoutInfo>,
    needs_build: bool,
}

impl OverlayChildLayoutBuilderElement {
    fn create(app: &mut App, widget: WidgetRef) -> Handle<OverlayChildLayoutBuilderElement> {
        debug_assert!(downcast_widget::<OverlayChildLayoutBuilderWidget>(&*widget).is_some());
        let this = app.create(OverlayChildLayoutBuilderElement {
            element: ElementData::new(widget),
            render_object_element: RenderObjectElementData::default(),
            child: None,
            build_scope: None,
            deferred_callback_scheduled: false,
            previous_layout_info: None,
            needs_build: true,
        });
        let build_scope = BuildScope::new(
            app,
            Some(Listener::handle_method(this, Self::schedule_rebuild)),
        );
        app.get_mut(this).build_scope = Some(build_scope);
        this
    }

    fn render_layout_builder(
        self: Handle<Self>,
        app: &App,
    ) -> RenderHandle<RenderOverlayChildLayoutBuilder> {
        self.typed_render_object(app)
    }

    fn schedule_rebuild(self: Handle<Self>, app: &mut App) {
        if app.get(self).deferred_callback_scheduled {
            return;
        }
        let defer_mark_needs_layout = match SchedulerBinding::scheduler_phase(app) {
            SchedulerPhase::Idle | SchedulerPhase::PostFrameCallbacks => true,
            SchedulerPhase::TransientCallbacks
            | SchedulerPhase::MidFrameMicrotasks
            | SchedulerPhase::PersistentCallbacks => false,
        };
        if !defer_mark_needs_layout {
            self.render_layout_builder(app)
                .schedule_layout_callback(app);
            return;
        }
        app.get_mut(self).deferred_callback_scheduled = true;
        SchedulerBinding::schedule_frame_callback(
            app,
            FrameCallback::new(move |app, _time_stamp| self.frame_callback(app)),
            false,
            true,
        );
    }

    fn frame_callback(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).deferred_callback_scheduled = false;
        if self.as_element().mounted(app) {
            self.render_layout_builder(app)
                .schedule_layout_callback(app);
        }
    }

    fn rebuild_with_constraints(self: Handle<Self>, app: &mut App) {
        let render_object = self.render_layout_builder(app);
        let layout_info = render_object.layout_info(app);
        let this = self.as_element();
        let needs_callback =
            app.get(self).needs_build || Some(layout_info) != app.get(self).previous_layout_info;
        let callback: Option<BuildScopeCallback> = needs_callback.then(|| {
            Box::new(move |app: &mut App| {
                debug_assert!(layout_info == render_object.layout_info(app));
                let builder = Rc::clone(&Self::widget_of(this.widget(app)).builder);
                let built = builder(app, this, layout_info);
                let child = app.get(self).child;
                let child = this.update_child(app, child, Some(built), None);
                debug_assert!(child.is_some());
                let state = app.get_mut(self);
                state.child = child;
                state.needs_build = false;
                state.previous_layout_info = Some(layout_info);
            }) as BuildScopeCallback
        });
        let owner = this.owner(app).expect("a laying-out element has an owner");
        owner.build_scope(app, this, callback);
    }
}

impl RenderObjectElementWidget for OverlayChildLayoutBuilderElement {
    type Widget = OverlayChildLayoutBuilderWidget;

    fn widget_of(widget: &WidgetRef) -> &OverlayChildLayoutBuilderWidget {
        downcast_widget::<OverlayChildLayoutBuilderWidget>(&**widget)
            .expect("an OverlayChildLayoutBuilderElement holds its widget")
    }
}

impl RenderObjectElement for OverlayChildLayoutBuilderElement {
    fn render_object_element_data(self: Handle<Self>, app: &App) -> &RenderObjectElementData {
        &app.get(self).render_object_element
    }

    fn render_object_element_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectElementData {
        &mut app.get_mut(self).render_object_element
    }
}

impl Element for OverlayChildLayoutBuilderElement {
    crate::element_accessors!();

    const IS_RENDER_OBJECT_ELEMENT: bool = true;

    fn render_object(self: Handle<Self>, app: &App) -> Option<AnyRenderObject> {
        self.render_object_element_data(app).render_object()
    }

    fn render_object_attaching_child(self: Handle<Self>, _app: &App) -> Option<AnyElement> {
        None
    }

    fn debug_doing_build(self: Handle<Self>, _app: &App) -> bool {
        false
    }

    fn build_scope(self: Handle<Self>, app: &App) -> Handle<BuildScope> {
        app.get(self).build_scope.expect("created with the element")
    }

    fn visit_children(self: Handle<Self>, app: &App, visitor: &mut dyn FnMut(AnyElement)) {
        if let Some(child) = app.get(self).child {
            visitor(child);
        }
    }

    fn forget_child(self: Handle<Self>, app: &mut App, child: AnyElement) {
        debug_assert!(app.get(self).child == Some(child));
        app.get_mut(self).child = None;
    }

    fn mount(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<AnyElement>,
        new_slot: Option<Slot>,
    ) {
        RenderObjectElement::mount(self, app, parent, new_slot); // Creates the renderObject.
        let render_object = self.render_layout_builder(app);
        render_object.update_callback(
            app,
            Listener::handle_method(self, Self::rebuild_with_constraints),
        );
    }

    fn update(self: Handle<Self>, app: &mut App, new_widget: WidgetRef) {
        let old_widget = self.as_element().widget(app).clone();
        debug_assert!(!Rc::ptr_eq(&old_widget, &new_widget));
        RenderObjectElement::update(self, app, new_widget.clone());
        let render_object = self.render_layout_builder(app);
        render_object.update_callback(
            app,
            Listener::handle_method(self, Self::rebuild_with_constraints),
        );
        if Self::widget_of(&new_widget).update_should_rebuild(Self::widget_of(&old_widget)) {
            app.get_mut(self).needs_build = true;
            render_object.schedule_layout_callback(app);
        }
    }

    fn mark_needs_build(self: Handle<Self>, app: &mut App) {
        self.render_layout_builder(app)
            .schedule_layout_callback(app);
        app.get_mut(self).needs_build = true;
    }

    fn perform_rebuild(self: Handle<Self>, app: &mut App) {
        self.render_layout_builder(app)
            .schedule_layout_callback(app);
        app.get_mut(self).needs_build = true;
        RenderObjectElement::perform_rebuild(self, app);
    }

    fn deactivate(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::deactivate(self, app);
    }

    fn unmount(self: Handle<Self>, app: &mut App) {
        let render_object = self.render_layout_builder(app);
        render_object.get_mut(app).callback = None;
        let build_scope = app.get_mut(self).build_scope.take();
        RenderObjectElement::unmount(self, app);
        if let Some(build_scope) = build_scope {
            app.destroy(build_scope);
        }
    }

    fn update_parent_data(self: Handle<Self>, app: &mut App, parent_data_element: AnyElement) {
        RenderObjectElement::update_parent_data(self, app, parent_data_element);
    }

    fn update_slot(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::update_slot(self, app, new_slot);
    }

    fn attach_render_object(self: Handle<Self>, app: &mut App, new_slot: Option<Slot>) {
        RenderObjectElement::attach_render_object(self, app, new_slot);
    }

    fn detach_render_object(self: Handle<Self>, app: &mut App) {
        RenderObjectElement::detach_render_object(self, app);
    }

    fn insert_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        slot: Option<Slot>,
    ) {
        debug_assert!(slot.is_none());
        let render_object = self.render_layout_builder(app);
        render_object.set_child(app, Some(child.as_box().expect("a box child")));
    }

    fn move_render_object_child(
        self: Handle<Self>,
        _app: &mut App,
        _child: AnyRenderObject,
        _old_slot: Option<Slot>,
        _new_slot: Option<Slot>,
    ) {
        debug_assert!(false, "a layout builder has one child");
    }

    fn remove_render_object_child(
        self: Handle<Self>,
        app: &mut App,
        child: AnyRenderObject,
        _slot: Option<Slot>,
    ) {
        let render_object = self.render_layout_builder(app);
        debug_assert!(render_object.child(app).map(|current| current.as_object()) == Some(child));
        render_object.set_child(app, None);
    }
}

// Dart's `_RenderLayoutBuilder`: a render box that
//  - has the same size and paint transform as its parent and its theater; in other words the
//    three boxes describe the same rect on screen.
//  - is a relayout boundary, and gets marked dirty for relayout every frame (but only when a
//    frame is already scheduled, and mark_needs_layout does not schedule a new frame since it
//    is called in a transient callback).
//  - runs a layout callback in perform_layout.
//
// Additionally, like RenderDeferredLayoutBox, this box also uses the stack layout algorithm so
// developers can use the Positioned widget.
struct RenderOverlayChildLayoutBuilder {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    layout_callback: RenderObjectWithLayoutCallbackData,
    callback: Option<Listener>,
    // The size here is the child size of the regular child in its own parent's coordinates.
    layout_info: Option<OverlayChildLayoutInfo>,
    callback_id: Option<i64>,
}

impl RenderOverlayChildLayoutBuilder {
    fn new(app: &mut App) -> RenderHandle<RenderOverlayChildLayoutBuilder> {
        RenderHandle::new_box(
            app,
            RenderOverlayChildLayoutBuilder {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                layout_callback: RenderObjectWithLayoutCallbackData::default(),
                callback: None,
                layout_info: None,
                callback_id: None,
            },
        )
    }

    /// The information the builder is given.
    fn layout_info(self: RenderHandle<Self>, app: &App) -> OverlayChildLayoutInfo {
        self.get(app)
            .layout_info
            .expect("the layout callback computes the layout information first")
    }

    /// Change the layout callback and schedule it (Dart's
    /// `RenderAbstractLayoutBuilderMixin.updateCallback`).
    fn update_callback(self: RenderHandle<Self>, app: &mut App, value: Listener) {
        if self.get(app).callback.as_ref() == Some(&value) {
            return;
        }
        self.get_mut(app).callback = Some(value);
        self.schedule_layout_callback(app);
    }

    fn compute_new_layout_info(self: RenderHandle<Self>, app: &App) -> OverlayChildLayoutInfo {
        let theater = self.theater(app);
        let parent = self
            .parent(app)
            .and_then(|parent| parent.downcast::<RenderDeferredLayoutBox>(app))
            .expect("the parent of an overlay child layout builder is a deferred layout box");
        let layout_surrogate = parent.get(app).layout_surrogate;
        debug_assert!(layout_surrogate.has_size(app));
        debug_assert!(
            layout_surrogate
                .child(app)
                .is_none_or(|child| child.size(app) == layout_surrogate.size(app))
        );
        debug_assert!(self.size(app) == theater.size(app));
        let overlay_portal_size = layout_surrogate.size(app);
        let paint_transform = layout_surrogate
            .as_object()
            .get_transform_to(app, Some(theater.as_object()));
        OverlayChildLayoutInfo {
            child_size: overlay_portal_size,
            child_paint_transform: paint_transform,
            overlay_size: self.size(app),
        }
    }

    fn frame_callback(self: RenderHandle<Self>, app: &mut App) {
        self.get_mut(app).callback_id = None;
        RenderBox::mark_needs_layout(self, app);
    }

    fn cancel_frame_callback(self: RenderHandle<Self>, app: &mut App) {
        if let Some(callback_id) = self.get_mut(app).callback_id.take() {
            SchedulerBinding::cancel_frame_callback_with_id(app, callback_id);
        }
    }
}

impl RenderObjectWithChildMixin for RenderOverlayChildLayoutBuilder {
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

impl RenderProxyBoxMixin for RenderOverlayChildLayoutBuilder {}

impl RenderObjectWithLayoutCallbackMixin for RenderOverlayChildLayoutBuilder {
    fn layout_callback_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &RenderObjectWithLayoutCallbackData {
        &self.get(app).layout_callback
    }

    fn layout_callback_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithLayoutCallbackData {
        &mut self.get_mut(app).layout_callback
    }

    fn layout_callback(self: RenderHandle<Self>, app: &mut App) {
        let layout_info = self.compute_new_layout_info(app);
        self.get_mut(app).layout_info = Some(layout_info);
        let callback = self
            .get(app)
            .callback
            .clone()
            .expect("a callback is set before layout");
        callback.call(app);
    }
}

impl RenderTheaterMixin for RenderOverlayChildLayoutBuilder {
    fn theater(self: RenderHandle<Self>, app: &App) -> RenderHandle<RenderTheater> {
        self.parent(app)
            .and_then(|parent| parent.downcast::<RenderDeferredLayoutBox>(app))
            .expect("the parent of an overlay child layout builder is a deferred layout box")
            .theater(app)
    }

    fn children_in_paint_order(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderBox> {
        self.child(app).into_iter().collect()
    }

    fn children_in_hit_test_order(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderBox> {
        RenderTheaterMixin::children_in_paint_order(self, app)
    }
}

impl RenderObject for RenderOverlayChildLayoutBuilder {
    reveal_rendering::render_object_accessors!();

    fn sized_by_parent(self: RenderHandle<Self>, _app: &App) -> bool {
        true
    }

    fn perform_resize(self: RenderHandle<Self>, app: &mut App) {
        let size = self.constraints(app).biggest();
        self.set_size(app, size);
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        self.run_layout_callback(app);
        if let Some(child) = self.child(app) {
            let constraints = self.constraints(app);
            self.layout_child(app, child, constraints);
        }
        debug_assert!(self.get(app).callback_id.is_none());
        if self.get(app).callback_id.is_none() {
            let callback_id = SchedulerBinding::schedule_frame_callback(
                app,
                FrameCallback::new(move |app, _time_stamp| self.frame_callback(app)),
                false,
                false,
            );
            self.get_mut(app).callback_id = Some(callback_id);
        }
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

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderTheaterMixin::paint(self, app, context, offset);
    }
}

impl RenderBox for RenderOverlayChildLayoutBuilder {
    reveal_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderTheaterMixin::setup_parent_data(self, app, child);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderTheaterMixin::hit_test_children(self, app, result, position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;
    use crate::widgets::media_query::MediaQueryData;
    use reveal_foundation::AppCell;

    fn test_entry(app: &mut App) -> Handle<OverlayEntry> {
        OverlayEntry::new(
            app,
            Rc::new(|_app, _context| SizedBox::shrink().into_widget()),
            false,
            false,
            false,
        )
    }

    fn mount_overlay(app: &mut App, entries: Vec<Handle<OverlayEntry>>) -> Harness {
        let tree = MediaQuery::new(
            MediaQueryData::new(),
            Directionality::new(TextDirection::Ltr, Overlay::new().initial_entries(entries)),
        )
        .into_widget();
        let harness = Harness::mount(app, tree);
        harness.pump(app);
        harness
    }

    fn theater_of(harness: &Harness, app: &App) -> RenderHandle<RenderTheater> {
        harness
            .render_root(app)
            .child(app)
            .expect("the overlay's render object")
            .as_object()
            .downcast::<RenderTheater>(app)
            .expect("a RenderTheater")
    }

    fn entry_of(app: &App, child: AnyRenderBox) -> Handle<OverlayEntry> {
        child
            .as_object()
            .parent_data_of::<TheaterParentData>(app)
            .overlay_entry
            .expect("a child built by an entry")
    }

    fn entries_in_render_order(
        app: &App,
        theater: RenderHandle<RenderTheater>,
    ) -> Vec<Handle<OverlayEntry>> {
        let mut order = Vec::new();
        let mut child = theater.first_child(app);
        while let Some(current) = child {
            order.push(entry_of(app, current));
            child = theater.child_after(app, current);
        }
        order
    }

    #[test]
    fn overlay_entries_are_inserted_rearranged_and_removed_in_order() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let (first, second) = (test_entry(&mut app), test_entry(&mut app));
        let harness = mount_overlay(&mut app, vec![first, second]);
        let theater = theater_of(&harness, &app);
        assert_eq!(entries_in_render_order(&app, theater), vec![first, second]);

        let overlay = app.get(first).overlay.expect("inserted by initial_entries");
        let third = test_entry(&mut app);
        overlay.insert(&mut app, third, Some(first), None);
        harness.pump(&mut app);
        assert_eq!(
            entries_in_render_order(&app, theater),
            vec![third, first, second]
        );

        let fourth = test_entry(&mut app);
        overlay.insert_all(&mut app, vec![fourth], None, None);
        harness.pump(&mut app);
        assert_eq!(
            entries_in_render_order(&app, theater),
            vec![third, first, second, fourth]
        );

        // `first` and `second` move to the bottom; the entries not mentioned stay on top.
        overlay.rearrange(&mut app, vec![second, first], None, None);
        harness.pump(&mut app);
        assert_eq!(
            entries_in_render_order(&app, theater),
            vec![second, first, third, fourth]
        );

        second.remove(&mut app);
        second.dispose(&mut app);
        harness.pump(&mut app);
        assert_eq!(
            entries_in_render_order(&app, theater),
            vec![first, third, fourth]
        );
    }

    #[test]
    fn an_opaque_entry_takes_the_entries_below_it_offstage() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let bottom = OverlayEntry::new(
            &mut app,
            Rc::new(|_app, _context| SizedBox::shrink().into_widget()),
            false,
            true,
            false,
        );
        let top = OverlayEntry::new(
            &mut app,
            Rc::new(|_app, _context| SizedBox::shrink().into_widget()),
            true,
            false,
            false,
        );
        let harness = mount_overlay(&mut app, vec![bottom, top]);
        let theater = theater_of(&harness, &app);

        // `bottom` is kept in the tree by maintain_state, but skipped when painting.
        assert_eq!(theater.child_count(&app), 2);
        assert_eq!(theater.skip_count(&app), 1);
        let painted = RenderTheaterMixin::children_in_paint_order(theater, &app);
        assert_eq!(painted.len(), 1);
        assert_eq!(entry_of(&app, painted[0]), top);

        // Without maintain_state an offstage entry is not built at all.
        bottom.set_maintain_state(&mut app, false);
        harness.pump(&mut app);
        assert_eq!(theater.child_count(&app), 1);
        assert_eq!(theater.skip_count(&app), 0);
        assert_eq!(entries_in_render_order(&app, theater), vec![top]);

        // Dropping the opacity brings it back onstage.
        top.set_opaque(&mut app, false);
        harness.pump(&mut app);
        assert_eq!(theater.skip_count(&app), 0);
        assert_eq!(
            RenderTheaterMixin::children_in_paint_order(theater, &app).len(),
            2
        );
    }

    #[test]
    fn an_overlay_portal_shows_and_hides_its_overlay_child_in_the_overlay() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let controller = OverlayPortalController::new(&mut app, None);
        let entry = OverlayEntry::new(
            &mut app,
            Rc::new(move |_app: &mut App, _context| {
                OverlayPortal::new(controller, |_app: &mut App, _context| {
                    SizedBox::square(Some(40.0)).into_widget()
                })
                .child(SizedBox::square(Some(10.0)))
                .into_widget()
            }),
            false,
            false,
            false,
        );
        let harness = mount_overlay(&mut app, vec![entry]);
        let theater = theater_of(&harness, &app);
        assert!(!controller.is_showing(&app));
        assert_eq!(
            RenderTheaterMixin::children_in_paint_order(theater, &app).len(),
            1
        );

        controller.show(&mut app);
        assert!(controller.is_showing(&app));
        harness.pump(&mut app);
        let painted = RenderTheaterMixin::children_in_paint_order(theater, &app);
        assert_eq!(painted.len(), 2);
        // The overlay child is a render child of the theater, painted above the entry.
        let deferred = painted[1]
            .as_object()
            .downcast::<RenderDeferredLayoutBox>(&app)
            .expect("the overlay child");
        assert_eq!(deferred.parent(&app), Some(theater.as_object()));
        assert_eq!(deferred.size(&app), theater.size(&app));
        assert_eq!(
            deferred.child(&app).expect("built").size(&app),
            theater.size(&app)
        );
        // The theater's child model still holds one entry child.
        assert_eq!(theater.child_count(&app), 1);

        controller.toggle(&mut app);
        assert!(!controller.is_showing(&app));
        harness.pump(&mut app);
        assert_eq!(
            RenderTheaterMixin::children_in_paint_order(theater, &app).len(),
            1
        );

        controller.toggle(&mut app);
        assert!(controller.is_showing(&app));
        harness.pump(&mut app);
        assert_eq!(
            RenderTheaterMixin::children_in_paint_order(theater, &app).len(),
            2
        );

        controller.hide(&mut app);
        harness.pump(&mut app);
        assert_eq!(
            RenderTheaterMixin::children_in_paint_order(theater, &app).len(),
            1
        );
    }
}
