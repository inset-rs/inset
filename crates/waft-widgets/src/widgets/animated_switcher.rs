//! Flutter counterpart: `widgets/animated_switcher.dart`.

use std::any::TypeId;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animation, AnimationBehavior, AnimationController, AnimationRef, AnimationStatusListener,
    Curve, CurvedAnimation, Curves,
};
use waft_foundation::{App, Handle, HandleId, ValueKey};
use waft_painting::AlignmentGeometry;
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef, can_update,
    keys_equal,
};
use crate::widgets::basic::{KeyedSubtree, Stack};
use crate::widgets::ticker_provider::{TickerProviderStateMixin, TickerProviderStateMixinData};
use crate::widgets::transitions::FadeTransition;
use waft_foundation::Ref;

/// Internal representation of a child that, now or in the past, was set on the
/// [`AnimatedSwitcher::child`] field, but is now in the process of
/// transitioning. The internal representation includes fields that we don't want
/// to expose to the public API (like the controller).
struct ChildEntry {
    /// The animation controller for the child's transition.
    controller: Ref<AnimationController>,

    /// The (curved) animation being used to drive the transition.
    animation: Ref<CurvedAnimation>,

    /// The currently built transition for this child.
    transition: WidgetRef,

    /// The widget's child at the time this entry was created or updated.
    /// Used to rebuild the transition if necessary.
    widget_child: WidgetRef,
}

/// Signature for builders used to generate custom transitions for
/// [`AnimatedSwitcher`].
///
/// The `child` should be transitioning in when the `animation` is running in
/// the forward direction.
///
/// The function should return a widget which wraps the given `child`. It may
/// also use the `animation` to inform its transition.
///
/// [`AnimatedSwitcher`] compares the builder it is given with the one it had, as Dart
/// compares two function values: [`new`](Self::new) equals only its clones;
/// [`function`](Self::function) equals any other built from the same function, as two
/// tear-offs of one static method are identical; [`handle_method`](Self::handle_method) equals
/// any other built from the same handle and function, as two tear-offs of one method on one
/// object are (see the porting skill's callback-identity pattern).
#[derive(Clone)]
pub struct AnimatedSwitcherTransitionBuilder {
    builder: Rc<AnimatedSwitcherTransitionBuilderFn>,
    identity: AnimatedSwitcherTransitionBuilderIdentity,
}

/// The function an [`AnimatedSwitcherTransitionBuilder`] calls.
type AnimatedSwitcherTransitionBuilderFn =
    dyn Fn(&mut App, &WidgetRef, AnimationRef<f64>) -> WidgetRef;

/// Equality key: closures compare by allocation, functions by the `TypeId` of the function
/// item, handle-methods by `(handle, TypeId of the function item)`.
#[derive(Clone, Copy, PartialEq)]
enum AnimatedSwitcherTransitionBuilderIdentity {
    Closure,
    Function(TypeId),
    HandleMethod(HandleId, TypeId),
}

impl AnimatedSwitcherTransitionBuilder {
    /// Wraps a closure; it equals only its clones.
    pub fn new(
        builder: impl Fn(&mut App, &WidgetRef, AnimationRef<f64>) -> WidgetRef + 'static,
    ) -> AnimatedSwitcherTransitionBuilder {
        AnimatedSwitcherTransitionBuilder {
            builder: Rc::new(builder),
            identity: AnimatedSwitcherTransitionBuilderIdentity::Closure,
        }
    }

    /// A function, comparable like a Dart tear-off of a static method.
    ///
    /// Pass the function by name. A fn pointer or capturing closure is rejected: it has no
    /// canonical identity.
    pub fn function<F>(builder: F) -> AnimatedSwitcherTransitionBuilder
    where
        F: Fn(&mut App, &WidgetRef, AnimationRef<f64>) -> WidgetRef + 'static,
    {
        const {
            assert!(
                std::mem::size_of::<F>() == 0,
                "pass the function by name: a fn pointer or capturing closure has no canonical \
                 identity"
            )
        };
        AnimatedSwitcherTransitionBuilder {
            builder: Rc::new(builder),
            identity: AnimatedSwitcherTransitionBuilderIdentity::Function(TypeId::of::<F>()),
        }
    }

    /// A handle's associated function, comparable like a Dart method tear-off such as a
    /// state's `_buildTransition`.
    ///
    /// Pass the function by name; see [`Listener::handle_method`](waft_foundation::Listener::handle_method)
    /// for the identity rule. A fn pointer or capturing closure is rejected: it has no canonical
    /// identity.
    pub fn handle_method<T: 'static, F>(this: Handle<T>, f: F) -> AnimatedSwitcherTransitionBuilder
    where
        F: Fn(Handle<T>, &mut App, &WidgetRef, AnimationRef<f64>) -> WidgetRef + 'static,
    {
        const {
            assert!(
                std::mem::size_of::<F>() == 0,
                "pass the function by name: a fn pointer or capturing closure has no canonical \
                 identity"
            )
        };
        AnimatedSwitcherTransitionBuilder {
            builder: Rc::new(move |app: &mut App, child: &WidgetRef, animation| {
                f(this, app, child, animation)
            }),
            identity: AnimatedSwitcherTransitionBuilderIdentity::HandleMethod(
                this.id(),
                TypeId::of::<F>(),
            ),
        }
    }

    /// Calls the builder.
    pub fn call(
        &self,
        app: &mut App,
        child: &WidgetRef,
        animation: AnimationRef<f64>,
    ) -> WidgetRef {
        (self.builder)(app, child, animation)
    }
}

impl PartialEq for AnimatedSwitcherTransitionBuilder {
    fn eq(&self, other: &AnimatedSwitcherTransitionBuilder) -> bool {
        match (self.identity, other.identity) {
            (
                AnimatedSwitcherTransitionBuilderIdentity::Closure,
                AnimatedSwitcherTransitionBuilderIdentity::Closure,
            ) => Rc::ptr_eq(&self.builder, &other.builder),
            (mine, theirs) => mine == theirs,
        }
    }
}

impl Debug for AnimatedSwitcherTransitionBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AnimatedSwitcherTransitionBuilder")
    }
}

/// Signature for builders used to generate custom layouts for
/// [`AnimatedSwitcher`].
///
/// The builder should return a widget which contains the given children, laid
/// out as desired. The builder should be able to handle an empty list of
/// `previous_children`, or a `None` `current_child`.
///
/// The `previous_children` list is sorted with the oldest at the beginning and
/// the newest at the end. It does not include the `current_child`.
pub type AnimatedSwitcherLayoutBuilder =
    Rc<dyn Fn(&mut App, Option<&WidgetRef>, &[WidgetRef]) -> WidgetRef>;

/// A widget that by default does a cross-fade between a new widget and the
/// widget previously set on the [`AnimatedSwitcher`] as a child.
///
/// If they are swapped fast enough (i.e. before [`duration`](Self::duration) elapses), more
/// than one previous child can exist and be transitioning out while the newest one is
/// transitioning in.
///
/// If the "new" child is the same widget type and key as the "old" child, but
/// with different parameters, then [`AnimatedSwitcher`] will *not* do a
/// transition between them, since as far as the framework is concerned, they
/// are the same widget and the existing widget can be updated with the new
/// parameters. To force the transition to occur, set a key on each child
/// widget that you wish to be considered unique (typically a `ValueKey` on the
/// widget data that distinguishes this child from the others).
///
/// The same key can be used for a new child as was used for an already-outgoing
/// child; the two will not be considered related. (For example, if a progress
/// indicator with key A is first shown, then an image with key B, then another
/// progress indicator with key A again, all in rapid succession, then the old
/// progress indicator and the image will be fading out while a new progress
/// indicator is fading in.)
///
/// The type of transition can be changed from a cross-fade to a custom
/// transition by setting the [`transition_builder`](Self::transition_builder).
///
/// See also:
///
///  * `AnimatedCrossFade`, which only fades between two children, but also
///    interpolates their sizes, and is reversible.
///  * `AnimatedOpacity`, which can be used to switch between nothingness and
///    a given child by fading the child in and out.
///  * [`FadeTransition`], which [`AnimatedSwitcher`] uses to perform the transition.
pub struct AnimatedSwitcher {
    pub key: Option<KeyRef>,

    /// The current child widget to display. If there was a previous child, then
    /// that child will be faded out using the [`switch_out_curve`](Self::switch_out_curve),
    /// while the new child is faded in with the [`switch_in_curve`](Self::switch_in_curve),
    /// over the [`duration`](Self::duration).
    ///
    /// If there was no previous child, then this child will fade in using the
    /// [`switch_in_curve`](Self::switch_in_curve) over the [`duration`](Self::duration).
    ///
    /// The child is considered to be "new" if it has a different type or key
    /// (see `Widget.canUpdate`).
    ///
    /// To change the kind of transition used, see
    /// [`transition_builder`](Self::transition_builder).
    pub child: Option<WidgetRef>,

    /// The duration of the transition from the old [`child`](Self::child) value to the new
    /// one.
    ///
    /// This duration is applied to the given [`child`](Self::child) when that property is set
    /// to a new child. The same duration is used when fading out, unless
    /// [`reverse_duration`](Self::reverse_duration) is set. Changing
    /// [`duration`](Self::duration) will not affect the durations of transitions already in
    /// progress.
    pub duration: Duration,

    /// The duration of the transition from the new [`child`](Self::child) value to the old
    /// one.
    ///
    /// This duration is applied to the given [`child`](Self::child) when that property is set
    /// to a new child. Changing [`reverse_duration`](Self::reverse_duration) will not affect
    /// the durations of transitions already in progress.
    ///
    /// If not set, then the value of [`duration`](Self::duration) is used by default.
    pub reverse_duration: Option<Duration>,

    /// The animation curve to use when transitioning in a new [`child`](Self::child).
    ///
    /// This curve is applied to the given [`child`](Self::child) when that property is set to
    /// a new child. Changing [`switch_in_curve`](Self::switch_in_curve) will not affect the
    /// curve of a transition already in progress.
    ///
    /// The [`switch_out_curve`](Self::switch_out_curve) is used when fading out, except that
    /// if [`child`](Self::child) is changed while the current child is in the middle of
    /// fading in, [`switch_in_curve`](Self::switch_in_curve) will be run in reverse from that
    /// point instead of jumping to the corresponding point on
    /// [`switch_out_curve`](Self::switch_out_curve).
    pub switch_in_curve: Rc<dyn Curve>,

    /// The animation curve to use when transitioning a previous [`child`](Self::child) out.
    ///
    /// This curve is applied to the [`child`](Self::child) when the child is faded in (or
    /// when the widget is created, for the first child). Changing
    /// [`switch_out_curve`](Self::switch_out_curve) will not affect the curves of
    /// already-visible widgets, it only affects the curves of future children.
    ///
    /// If [`child`](Self::child) is changed while the current child is in the middle of
    /// fading in, [`switch_in_curve`](Self::switch_in_curve) will be run in reverse from that
    /// point instead of jumping to the corresponding point on
    /// [`switch_out_curve`](Self::switch_out_curve).
    pub switch_out_curve: Rc<dyn Curve>,

    /// A function that wraps a new [`child`](Self::child) with an animation that transitions
    /// the [`child`](Self::child) in when the animation runs in the forward direction and out
    /// when the animation runs in the reverse direction. This is only called
    /// when a new [`child`](Self::child) is set (not for each build), or when a new
    /// [`transition_builder`](Self::transition_builder) is set. If a new
    /// [`transition_builder`](Self::transition_builder) is set, then the transition is
    /// rebuilt for the current child and all previous children using the new
    /// [`transition_builder`](Self::transition_builder).
    ///
    /// The default is [`AnimatedSwitcher::default_transition_builder`].
    ///
    /// The animation provided to the builder has the [`duration`](Self::duration) and
    /// [`switch_in_curve`](Self::switch_in_curve) or
    /// [`switch_out_curve`](Self::switch_out_curve) applied as provided when the
    /// corresponding [`child`](Self::child) was first provided.
    ///
    /// See also:
    ///
    ///  * [`AnimatedSwitcherTransitionBuilder`] for more information about
    ///    how a transition builder should function.
    pub transition_builder: AnimatedSwitcherTransitionBuilder,

    /// A function that wraps all of the children that are transitioning out, and
    /// the [`child`](Self::child) that's transitioning in, with a widget that lays all of them
    /// out. This is called every time this widget is built.
    ///
    /// The default is [`AnimatedSwitcher::default_layout_builder`].
    ///
    /// See also:
    ///
    ///  * [`AnimatedSwitcherLayoutBuilder`] for more information about
    ///    how a layout builder should function.
    pub layout_builder: AnimatedSwitcherLayoutBuilder,
}

impl Debug for AnimatedSwitcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnimatedSwitcher")
            .field("key", &self.key)
            .field("child", &self.child)
            .field("duration", &self.duration)
            .field("reverse_duration", &self.reverse_duration)
            .finish_non_exhaustive()
    }
}

impl AnimatedSwitcher {
    /// Creates an [`AnimatedSwitcher`].
    pub fn new(duration: Duration) -> AnimatedSwitcher {
        AnimatedSwitcher {
            key: None,
            child: None,
            duration,
            reverse_duration: None,
            switch_in_curve: Curves::linear(),
            switch_out_curve: Curves::linear(),
            transition_builder: AnimatedSwitcherTransitionBuilder::function(
                AnimatedSwitcher::default_transition_builder,
            ),
            layout_builder: Rc::new(AnimatedSwitcher::default_layout_builder),
        }
    }

    /// Dart `AnimatedSwitcher(key:)`.
    pub fn key(mut self, key: KeyRef) -> AnimatedSwitcher {
        self.key = Some(key);
        self
    }

    /// Dart `AnimatedSwitcher(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> AnimatedSwitcher {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart `AnimatedSwitcher(reverseDuration:)`.
    pub fn reverse_duration(mut self, reverse_duration: Duration) -> AnimatedSwitcher {
        self.reverse_duration = Some(reverse_duration);
        self
    }

    /// Dart `AnimatedSwitcher(switchInCurve:)`.
    pub fn switch_in_curve(mut self, switch_in_curve: Rc<dyn Curve>) -> AnimatedSwitcher {
        self.switch_in_curve = switch_in_curve;
        self
    }

    /// Dart `AnimatedSwitcher(switchOutCurve:)`.
    pub fn switch_out_curve(mut self, switch_out_curve: Rc<dyn Curve>) -> AnimatedSwitcher {
        self.switch_out_curve = switch_out_curve;
        self
    }

    /// Dart `AnimatedSwitcher(transitionBuilder:)`.
    pub fn transition_builder(
        mut self,
        transition_builder: AnimatedSwitcherTransitionBuilder,
    ) -> AnimatedSwitcher {
        self.transition_builder = transition_builder;
        self
    }

    /// Dart `AnimatedSwitcher(layoutBuilder:)`.
    pub fn layout_builder(
        mut self,
        layout_builder: impl Fn(&mut App, Option<&WidgetRef>, &[WidgetRef]) -> WidgetRef + 'static,
    ) -> AnimatedSwitcher {
        self.layout_builder = Rc::new(layout_builder);
        self
    }

    /// The transition builder used as the default value of
    /// [`transition_builder`](Self::transition_builder).
    ///
    /// The new child is given a [`FadeTransition`] which increases opacity as
    /// the animation goes from 0.0 to 1.0, and decreases when the animation is
    /// reversed.
    ///
    /// This is an [`AnimatedSwitcherTransitionBuilder`] function.
    pub fn default_transition_builder(
        _app: &mut App,
        child: &WidgetRef,
        animation: AnimationRef<f64>,
    ) -> WidgetRef {
        let key = ValueKey::new(child.key().cloned());
        FadeTransition::new(animation)
            .key(key.into())
            .child(child.clone())
            .into_widget()
    }

    /// The layout builder used as the default value of
    /// [`layout_builder`](Self::layout_builder).
    ///
    /// The new child is placed in a [`Stack`] that sizes itself to match the
    /// largest of the child or a previous child. The children are centered on
    /// each other.
    ///
    /// This is an [`AnimatedSwitcherLayoutBuilder`] function.
    pub fn default_layout_builder(
        _app: &mut App,
        current_child: Option<&WidgetRef>,
        previous_children: &[WidgetRef],
    ) -> WidgetRef {
        Stack::new()
            .alignment(AlignmentGeometry::CENTER)
            .children(previous_children.iter().chain(current_child).cloned())
            .into_widget()
    }
}

impl StatefulWidget for AnimatedSwitcher {
    type State = AnimatedSwitcherState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> AnimatedSwitcherState {
        AnimatedSwitcherState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            current_entry: None,
            outgoing_entries: Vec::new(),
            outgoing_widgets: Some(Vec::new()),
            child_number: 0,
        }
    }
}

/// Dart's `_AnimatedSwitcherState`.
///
/// `outgoing_entries` is Dart's insertion-ordered `Set<_ChildEntry>`: an entry is only ever
/// added once, so a `Vec` keeps the order and the set's identity.
pub struct AnimatedSwitcherState {
    state: StateData<AnimatedSwitcher>,
    ticker_provider: TickerProviderStateMixinData,
    current_entry: Option<ChildEntry>,
    outgoing_entries: Vec<ChildEntry>,
    outgoing_widgets: Option<Vec<WidgetRef>>,
    child_number: i64,
}

impl Debug for AnimatedSwitcherState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnimatedSwitcherState")
            .finish_non_exhaustive()
    }
}

impl TickerProviderStateMixin for AnimatedSwitcherState {
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

impl TickerProviderObject for AnimatedSwitcherState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

/// Dart's `_updateTransitionForEntry`: the entry's transition again, in the same key, over its
/// current `widget_child`.
fn update_transition_for_entry(
    app: &mut App,
    transition_builder: &AnimatedSwitcherTransitionBuilder,
    entry: &mut ChildEntry,
) {
    let key = entry.transition.key().cloned();
    let child = transition_builder.call(app, &entry.widget_child, entry.animation.erase());
    let mut transition = KeyedSubtree::new(child);
    transition.key = key;
    entry.transition = transition.into_widget();
}

impl AnimatedSwitcherState {
    fn add_entry_for_new_child(self: Handle<Self>, app: &mut App, animate: bool) {
        debug_assert!(animate || app.get(self).current_entry.is_none());
        if let Some(current_entry) = app.get_mut(self).current_entry.take() {
            debug_assert!(animate);
            let controller = current_entry.controller.handle();
            let state = app.get_mut(self);
            state.outgoing_entries.push(current_entry);
            controller.reverse(app, None);
            self.mark_child_widget_cache_as_dirty(app);
        }
        let widget = self.widget(app);
        let Some(child) = widget.child.clone() else {
            return;
        };
        let duration = widget.duration;
        let reverse_duration = widget.reverse_duration;
        let switch_in_curve = Rc::clone(&widget.switch_in_curve);
        let switch_out_curve = Rc::clone(&widget.switch_out_curve);
        let builder = widget.transition_builder.clone();
        let controller = AnimationController::new(
            app,
            None,
            Some(duration),
            reverse_duration,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let animation = CurvedAnimation::new(
            app,
            controller.erase(),
            switch_in_curve,
            Some(switch_out_curve),
        );
        let entry = self.new_entry(app, child, &builder, controller.clone(), animation);
        app.get_mut(self).current_entry = Some(entry);
        if animate {
            controller.forward(app, None);
        } else {
            debug_assert!(app.get(self).outgoing_entries.is_empty());
            controller.set_value(app, 1.0);
        }
    }

    fn new_entry(
        self: Handle<Self>,
        app: &mut App,
        child: WidgetRef,
        builder: &AnimatedSwitcherTransitionBuilder,
        controller: Ref<AnimationController>,
        animation: Ref<CurvedAnimation>,
    ) -> ChildEntry {
        let child_number = app.get(self).child_number;
        let transition =
            KeyedSubtree::wrap(builder.call(app, &child, animation.erase()), child_number)
                .into_widget();
        // The listener lives on the animation: it names the entry's objects without keeping
        // them, and the entry it removes was their keeper.
        let (controller_handle, animation_handle) = (controller.handle(), animation.handle());
        animation.as_animation().add_status_listener(
            app,
            AnimationStatusListener::new(move |status, app| {
                let (controller, animation) = (controller_handle, animation_handle);
                if status.is_dismissed() {
                    self.set_state(app, |state| {
                        debug_assert!(
                            state
                                .outgoing_entries
                                .iter()
                                .any(|entry| entry.controller == controller)
                        );
                        state
                            .outgoing_entries
                            .retain(|entry| entry.controller != controller);
                        state.outgoing_widgets = None;
                    });
                    controller.dispose(app);
                    animation.dispose(app);
                }
            }),
        );
        ChildEntry {
            widget_child: child,
            transition,
            animation,
            controller,
        }
    }

    fn mark_child_widget_cache_as_dirty(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).outgoing_widgets = None;
    }

    fn rebuild_outgoing_widgets_if_needed(self: Handle<Self>, app: &mut App) {
        let state = app.get_mut(self);
        if state.outgoing_widgets.is_none() {
            state.outgoing_widgets = Some(
                state
                    .outgoing_entries
                    .iter()
                    .map(|entry| entry.transition.clone())
                    .collect(),
            );
        }
        debug_assert_eq!(
            state.outgoing_entries.len(),
            state.outgoing_widgets.as_ref().map_or(0, Vec::len)
        );
    }

    /// The transitions of the current entry and every outgoing one, rebuilt with the widget's
    /// `transition_builder`.
    fn update_transitions_for_all_entries(self: Handle<Self>, app: &mut App) {
        let builder = self.widget(app).transition_builder.clone();
        let mut outgoing_entries = std::mem::take(&mut app.get_mut(self).outgoing_entries);
        for entry in &mut outgoing_entries {
            update_transition_for_entry(app, &builder, entry);
        }
        app.get_mut(self).outgoing_entries = outgoing_entries;
        if let Some(mut current_entry) = app.get_mut(self).current_entry.take() {
            update_transition_for_entry(app, &builder, &mut current_entry);
            app.get_mut(self).current_entry = Some(current_entry);
        }
    }
}

impl State for AnimatedSwitcherState {
    type Widget = AnimatedSwitcher;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.add_entry_for_new_child(app, false);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &AnimatedSwitcher) {
        // If the transition builder changed, then update all of the previous
        // transitions.
        if self.widget(app).transition_builder != old_widget.transition_builder {
            self.update_transitions_for_all_entries(app);
            self.mark_child_widget_cache_as_dirty(app);
        }

        let widget_child = self.widget(app).child.clone();
        let has_new_child = widget_child.is_some();
        let has_old_child = app.get(self).current_entry.is_some();
        let child_changed = match (&widget_child, &app.get(self).current_entry) {
            (Some(child), Some(entry)) => !can_update(&**child, &*entry.widget_child),
            _ => false,
        };
        if has_new_child != has_old_child || child_changed {
            // Child has changed, fade current entry out and add new entry.
            app.get_mut(self).child_number += 1;
            self.add_entry_for_new_child(app, true);
        } else if let (Some(widget_child), true) = (widget_child, has_old_child) {
            debug_assert!(has_old_child && has_new_child);
            // Child has been updated. Make sure we update the child widget and
            // transition in current_entry even though we're not going to start a new
            // animation, but keep the key from the previous transition so that we
            // update the transition instead of replacing it.
            let builder = self.widget(app).transition_builder.clone();
            let mut current_entry = app
                .get_mut(self)
                .current_entry
                .take()
                .expect("has_old_child");
            current_entry.widget_child = widget_child;
            update_transition_for_entry(app, &builder, &mut current_entry); // uses entry.widget_child
            app.get_mut(self).current_entry = Some(current_entry);
            self.mark_child_widget_cache_as_dirty(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let state = app.get_mut(self);
        let entries: Vec<(Handle<AnimationController>, Handle<CurvedAnimation>)> = state
            .current_entry
            .iter()
            .chain(&state.outgoing_entries)
            .map(|entry| (entry.controller.handle(), entry.animation.handle()))
            .collect();
        for (controller, animation) in entries {
            controller.dispose(app);
            animation.dispose(app);
        }
        TickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        self.rebuild_outgoing_widgets_if_needed(app);
        let layout_builder = Rc::clone(&self.widget(app).layout_builder);
        let state = app.get(self);
        let current_transition = state
            .current_entry
            .as_ref()
            .map(|entry| entry.transition.clone());
        let current_key = current_transition
            .as_ref()
            .and_then(|transition| transition.key());
        let mut previous_children: Vec<WidgetRef> = Vec::new();
        for outgoing in state.outgoing_widgets.iter().flatten() {
            if !keys_equal(outgoing.key(), current_key)
                && !previous_children
                    .iter()
                    .any(|previous| Rc::ptr_eq(previous, outgoing))
            {
                previous_children.push(outgoing.clone());
            }
        }
        layout_builder(app, current_transition.as_ref(), &previous_children)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use waft_foundation::{AppCell, UniqueKey};

    use waft_embedder::{Size, TextDirection};

    use super::*;
    use crate::testing::{
        Finder, WidgetTester, assert_more_or_less_equals_within, expect, find, finds_n_widgets,
        finds_nothing, finds_one_widget, test_widgets,
    };
    use crate::widgets::basic::{Center, Column, Directionality, SizedBox};
    use crate::widgets::transitions::{Animated, ScaleTransition, SizeTransition};

    const DURATION: Duration = Duration::from_millis(100);

    fn ms(milliseconds: u64) -> Duration {
        Duration::from_millis(milliseconds)
    }

    /// The `FadeTransition`s the switcher builds: Dart's `find.byType(FadeTransition)`.
    fn fade_transitions() -> Finder {
        find::by_type::<FadeTransition>()
    }

    /// Dart's `tester.widget<FadeTransition>(finder).opacity.value`.
    fn opacity(tester: &WidgetTester, finder: Finder) -> f64 {
        let opacity = tester.widget(finder, |transition: &FadeTransition| {
            transition.opacity.clone()
        });
        opacity.value(&tester.cell.borrow())
    }

    fn keyed_box(key: &KeyRef) -> WidgetRef {
        SizedBox::new().key(key.clone()).into_widget()
    }

    fn switcher(child: Option<WidgetRef>) -> AnimatedSwitcher {
        let switcher = AnimatedSwitcher::new(DURATION);
        match child {
            Some(child) => switcher.child(child),
            None => switcher,
        }
    }

    fn unique_key() -> KeyRef {
        UniqueKey::new().into()
    }

    #[test]
    fn fades_in_a_new_child() {
        test_widgets(|tester| {
            let (one, two, three) = (unique_key(), unique_key(), unique_key());
            tester.pump_widget(switcher(Some(keyed_box(&one))));
            expect(tester, &fade_transitions(), finds_one_widget());
            assert_eq!(opacity(tester, fade_transitions().first()), 1.0);

            tester.pump_widget(switcher(Some(keyed_box(&two))));
            tester.pump_by(ms(50));
            expect(tester, &fade_transitions(), finds_n_widgets(2));
            assert_eq!(opacity(tester, fade_transitions().first()), 0.5);

            tester.pump_widget(switcher(Some(keyed_box(&three))));
            tester.pump_by(ms(10));
            assert_more_or_less_equals_within(opacity(tester, fade_transitions().at(0)), 0.4, 0.01);
            assert_more_or_less_equals_within(opacity(tester, fade_transitions().at(1)), 0.4, 0.01);
            assert_more_or_less_equals_within(opacity(tester, fade_transitions().at(2)), 0.1, 0.01);
            tester.pump_and_settle();
        });
    }

    #[test]
    fn can_handle_back_to_back_changes() {
        test_widgets(|tester| {
            let (one, two, three) = (unique_key(), unique_key(), unique_key());
            tester.pump_widget(switcher(Some(keyed_box(&one))));
            expect(tester, &find::by_key(one.clone()), finds_one_widget());
            expect(tester, &find::by_key(two.clone()), finds_nothing());
            expect(tester, &find::by_key(three.clone()), finds_nothing());

            tester.pump_widget(switcher(Some(keyed_box(&two))));
            expect(tester, &find::by_key(one.clone()), finds_one_widget());
            expect(tester, &find::by_key(two.clone()), finds_one_widget());
            expect(tester, &find::by_key(three.clone()), finds_nothing());

            tester.pump_widget(switcher(Some(keyed_box(&three))));
            expect(tester, &find::by_key(one.clone()), finds_one_widget());
            expect(tester, &find::by_key(two.clone()), finds_nothing());
            expect(tester, &find::by_key(three.clone()), finds_one_widget());
        });
    }

    #[test]
    fn does_not_transition_in_a_new_child_of_the_same_type() {
        test_widgets(|tester| {
            let plain = |width: f64| SizedBox::new().width(width).into_widget();
            tester.pump_widget(switcher(Some(plain(1.0))));
            expect(tester, &fade_transitions(), finds_one_widget());
            assert_eq!(opacity(tester, fade_transitions().first()), 1.0);

            tester.pump_widget(switcher(Some(plain(2.0))));
            tester.pump_by(ms(50));
            expect(tester, &fade_transitions(), finds_one_widget());
            assert_eq!(opacity(tester, fade_transitions().first()), 1.0);
            tester.pump_and_settle();
        });
    }

    #[test]
    fn handles_null_children() {
        test_widgets(|tester| {
            let plain = || SizedBox::new().into_widget();
            tester.pump_widget(switcher(None));
            expect(tester, &fade_transitions(), finds_nothing());

            tester.pump_widget(switcher(Some(plain())));
            tester.pump_by(ms(50));
            assert_eq!(opacity(tester, fade_transitions().first()), 0.5);
            tester.pump_and_settle();

            tester.pump_widget(switcher(Some(plain())));
            expect(tester, &fade_transitions(), finds_one_widget());
            assert_eq!(opacity(tester, fade_transitions().first()), 1.0);

            tester.pump_widget(switcher(None));
            tester.pump_by(ms(50));
            assert_eq!(opacity(tester, fade_transitions().first()), 0.5);

            tester.pump_widget(switcher(None));
            tester.pump_by(ms(50));
            assert_eq!(opacity(tester, fade_transitions().first()), 0.0);

            tester.pump_and_settle();
        });
    }

    #[test]
    fn does_not_start_any_animations_after_dispose() {
        test_widgets(|tester| {
            tester.pump_widget(switcher(Some(SizedBox::new().into_widget())));
            tester.pump_by(ms(50));

            // Change the widget tree in the middle of the animation.
            tester.pump_widget(SizedBox::new().width(1.0));
            assert_eq!(tester.pump_and_settle(), 1);
        });
    }

    /// The curved animation each child entry drives is freed once the entry leaves, on the
    /// outgoing child's dismissal and with the state, as Dart's collector frees it.
    #[test]
    fn frees_each_entrys_curved_animation_once_its_entry_leaves() {
        test_widgets(|tester| {
            let (one, two) = (unique_key(), unique_key());
            tester.pump_widget(switcher(Some(keyed_box(&one))));
            let current_animation = |tester: &WidgetTester| {
                let element = tester.element(find::by_type::<AnimatedSwitcher>());
                let app = tester.cell.borrow();
                let state = element
                    .state_handle::<AnimatedSwitcherState>(&app)
                    .expect("a mounted AnimatedSwitcher");
                app.get(state)
                    .current_entry
                    .as_ref()
                    .expect("a child entry")
                    .animation
                    .handle()
            };
            let first = current_animation(tester);

            tester.pump_widget(switcher(Some(keyed_box(&two))));
            tester.pump_by(ms(200));
            let second = current_animation(tester);
            tester.cell.checkpoint();
            assert!(
                !tester.cell.borrow().contains(first),
                "the outgoing entry's"
            );
            assert!(tester.cell.borrow().contains(second));

            tester.pump_widget(SizedBox::new());
            tester.cell.checkpoint();
            assert!(
                !tester.cell.borrow().contains(second),
                "the current entry's, at dispose"
            );
        });
    }

    #[test]
    fn uses_custom_layout() {
        test_widgets(|tester| {
            let widget = switcher(Some(SizedBox::new().into_widget())).layout_builder(
                |_app, current_child, previous_children| {
                    Column::new()
                        .children(previous_children.iter().chain(current_child).cloned())
                        .into_widget()
                },
            );
            tester.pump_widget(widget);
            expect(tester, &find::by_type::<Column>(), finds_one_widget());
        });
    }

    /// A layout builder that records the children it was given, `current_child` first.
    fn recording_layout(
        found_children: &Rc<RefCell<Vec<WidgetRef>>>,
    ) -> impl Fn(&mut App, Option<&WidgetRef>, &[WidgetRef]) -> WidgetRef + 'static {
        let found_children = Rc::clone(found_children);
        move |_app, current_child, previous_children| {
            let children: Vec<WidgetRef> = current_child
                .into_iter()
                .chain(previous_children)
                .cloned()
                .collect();
            *found_children.borrow_mut() = children.clone();
            Column::new().children(children).into_widget()
        }
    }

    fn as_keyed_subtree(widget: &WidgetRef) -> &KeyedSubtree {
        widget
            .as_any()
            .downcast_ref::<KeyedSubtree>()
            .expect("a KeyedSubtree")
    }

    #[test]
    fn uses_custom_transitions() {
        test_widgets(|tester| {
            let found_children = Rc::new(RefCell::new(Vec::new()));
            let transition_builder =
                AnimatedSwitcherTransitionBuilder::new(|_app, child, animation| {
                    SizeTransition::new(animation)
                        .child(child.clone())
                        .into_widget()
                });
            let tree = |child: Option<WidgetRef>| {
                let mut switcher = switcher(child)
                    .layout_builder(recording_layout(&found_children))
                    .transition_builder(transition_builder.clone());
                switcher.key = None;
                Directionality::new(TextDirection::Rtl, switcher).into_widget()
            };
            tester.pump_widget(tree(Some(SizedBox::new().into_widget())));
            expect(tester, &find::by_type::<Column>(), finds_one_widget());
            for child in found_children.borrow().iter() {
                as_keyed_subtree(child);
            }

            tester.pump_widget(tree(None));
            tester.pump_by(ms(50));
            assert!(!found_children.borrow().is_empty());
            for child in found_children.borrow().iter() {
                assert!(
                    as_keyed_subtree(child)
                        .child
                        .as_any()
                        .is::<Animated<SizeTransition>>()
                );
            }
        });
    }

    /// Dart's `StatefulTest` with its static `generation`.
    #[derive(Debug)]
    struct StatefulTest {
        key: KeyRef,
        generation: Rc<Cell<u32>>,
    }

    impl StatefulWidget for StatefulTest {
        type State = StatefulTestState;

        fn key(&self) -> Option<&KeyRef> {
            Some(&self.key)
        }

        fn create_state(&self) -> StatefulTestState {
            StatefulTestState {
                state: StateData::new(),
            }
        }
    }

    struct StatefulTestState {
        state: StateData<StatefulTest>,
    }

    impl State for StatefulTestState {
        type Widget = StatefulTest;
        crate::state_accessors!();

        fn init_state(self: Handle<Self>, app: &mut App) {
            let generation = &self.widget(app).generation;
            generation.set(generation.get() + 1);
        }

        fn build(self: Handle<Self>, _app: &mut App, _context: BuildContext) -> WidgetRef {
            SizedBox::new().into_widget()
        }
    }

    #[test]
    fn does_not_reset_state_of_the_children_in_transitions() {
        test_widgets(|tester| {
            let generation = Rc::new(Cell::new(0));
            let stateful = |key: &KeyRef| {
                switcher(Some(
                    StatefulTest {
                        key: key.clone(),
                        generation: Rc::clone(&generation),
                    }
                    .into_widget(),
                ))
            };
            let (one, two, three) = (unique_key(), unique_key(), unique_key());
            tester.pump_widget(stateful(&one));
            expect(tester, &fade_transitions(), finds_one_widget());
            assert_eq!(opacity(tester, fade_transitions().first()), 1.0);
            assert_eq!(generation.get(), 1);

            tester.pump_widget(stateful(&two));
            tester.pump_by(ms(50));
            expect(tester, &fade_transitions(), finds_n_widgets(2));
            assert_eq!(opacity(tester, fade_transitions().first()), 0.5);
            assert_eq!(generation.get(), 2);

            tester.pump_widget(stateful(&three));
            tester.pump_by(ms(10));
            assert_eq!(generation.get(), 3);
            assert_more_or_less_equals_within(opacity(tester, fade_transitions().at(0)), 0.4, 0.01);
            assert_more_or_less_equals_within(opacity(tester, fade_transitions().at(1)), 0.4, 0.01);
            assert_more_or_less_equals_within(opacity(tester, fade_transitions().at(2)), 0.1, 0.01);
            tester.pump_and_settle();
            assert_eq!(generation.get(), 3);
        });
    }

    #[test]
    fn updates_widgets_without_animating_if_they_are_isomorphic() {
        test_widgets(|tester| {
            let sized = |width: f64| {
                Directionality::new(
                    TextDirection::Rtl,
                    switcher(Some(SizedBox::new().width(width).into_widget())),
                )
            };
            tester.pump_widget(sized(1.0));
            tester.pump_by(ms(10));
            assert_eq!(opacity(tester, fade_transitions().first()), 1.0);
            let width = |tester: &WidgetTester| {
                tester.widget(find::by_type::<SizedBox>(), |sized_box: &SizedBox| {
                    sized_box.width
                })
            };
            assert_eq!(width(tester), Some(1.0));

            tester.pump_widget(sized(2.0));
            tester.pump_by(ms(20));
            assert_eq!(opacity(tester, fade_transitions().first()), 1.0);
            assert_eq!(width(tester), Some(2.0));
        });
    }

    #[test]
    fn updates_previous_child_transitions_if_the_transition_builder_changes() {
        test_widgets(|tester| {
            let found_children = Rc::new(RefCell::new(Vec::new()));
            let tree =
                |key: &KeyRef, transition_builder: Option<AnimatedSwitcherTransitionBuilder>| {
                    let mut switcher = switcher(Some(keyed_box(key)))
                        .layout_builder(recording_layout(&found_children));
                    if let Some(transition_builder) = transition_builder {
                        switcher = switcher.transition_builder(transition_builder);
                    }
                    switcher
                };
            let (one, two, three, four) = (unique_key(), unique_key(), unique_key(), unique_key());
            // Insert three unique children so that we have some previous children.
            tester.pump_widget(tree(&one, None));
            tester.pump_by(ms(10));
            tester.pump_widget(tree(&two, None));
            tester.pump_by(ms(10));
            tester.pump_widget(tree(&three, None));
            tester.pump_by(ms(10));

            assert_eq!(found_children.borrow().len(), 3);
            for child in found_children.borrow().iter() {
                assert!(
                    as_keyed_subtree(child)
                        .child
                        .as_any()
                        .is::<FadeTransition>()
                );
            }

            // Now set a new transition builder and make sure all the previous
            // transitions are replaced.
            let scale = AnimatedSwitcherTransitionBuilder::new(|_app, child, animation| {
                ScaleTransition::new(animation)
                    .child(child.clone())
                    .into_widget()
            });
            tester.pump_widget(tree(&four, Some(scale)));
            tester.pump_by(ms(10));

            assert_eq!(found_children.borrow().len(), 3);
            for child in found_children.borrow().iter() {
                assert!(
                    as_keyed_subtree(child)
                        .child
                        .as_any()
                        .is::<Animated<ScaleTransition>>()
                );
            }
        });
    }

    #[test]
    fn does_not_duplicate_animations_if_the_same_child_is_entered_twice() {
        test_widgets(|tester| {
            let (one, two): (KeyRef, KeyRef) = (
                waft_foundation::ValueKey::new("1".to_string()).into(),
                waft_foundation::ValueKey::new("2".to_string()).into(),
            );
            let tree = |key: &KeyRef| {
                Directionality::new(
                    TextDirection::Ltr,
                    AnimatedSwitcher::new(Duration::from_millis(1000)).child(keyed_box(key)),
                )
            };
            tester.pump_widget(tree(&one));
            tester.pump_widget(tree(&two));
            tester.pump_widget(tree(&one));
            tester.pump_by(ms(1000));
            expect(tester, &find::by_key(one.clone()), finds_one_widget());
        });
    }

    #[test]
    fn does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            let tree = |child: Option<WidgetRef>| {
                let mut switcher = AnimatedSwitcher::new(Duration::from_micros(500));
                if let Some(child) = child {
                    switcher = switcher.child(child);
                }
                Directionality::new(
                    TextDirection::Ltr,
                    Center::new().child(SizedBox::shrink().child(switcher)),
                )
            };
            let switcher_size =
                |tester: &WidgetTester| tester.get_size(find::by_type::<AnimatedSwitcher>());
            tester.pump_widget(tree(None));
            assert_eq!(switcher_size(tester), Size::ZERO);

            let key =
                |name: &str| -> KeyRef { waft_foundation::ValueKey::new(name.to_string()).into() };
            tester.pump_widget(tree(Some(keyed_box(&key("x")))));
            assert_eq!(switcher_size(tester), Size::ZERO);

            tester.pump_widget(tree(Some(keyed_box(&key("y")))));
            assert_eq!(switcher_size(tester), Size::ZERO);
        });
    }

    #[test]
    fn the_default_builders_compare_as_the_same_function() {
        let first = AnimatedSwitcher::new(DURATION);
        let second = AnimatedSwitcher::new(DURATION);
        assert!(first.transition_builder == second.transition_builder);
        let custom =
            AnimatedSwitcherTransitionBuilder::new(|_app, child, _animation| child.clone());
        assert!(custom != first.transition_builder);
        assert!(custom == custom.clone());
        let _ = AppCell::new();
    }

    #[test]
    fn two_tear_offs_of_one_method_on_one_object_compare_equal() {
        struct Host;
        fn build_transition(
            _this: Handle<Host>,
            _app: &mut App,
            child: &WidgetRef,
            _animation: AnimationRef<f64>,
        ) -> WidgetRef {
            child.clone()
        }
        fn other_build_transition(
            _this: Handle<Host>,
            _app: &mut App,
            child: &WidgetRef,
            _animation: AnimationRef<f64>,
        ) -> WidgetRef {
            child.clone()
        }

        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let host = app.create(Host);
        let other_host = app.create(Host);
        let tear_off =
            AnimatedSwitcherTransitionBuilder::handle_method(host.handle(), build_transition);

        assert!(
            tear_off
                == AnimatedSwitcherTransitionBuilder::handle_method(
                    host.handle(),
                    build_transition
                )
        );
        assert!(
            tear_off
                != AnimatedSwitcherTransitionBuilder::handle_method(
                    other_host.handle(),
                    build_transition
                )
        );
        assert!(
            tear_off
                != AnimatedSwitcherTransitionBuilder::handle_method(
                    host.handle(),
                    other_build_transition
                )
        );
        assert!(tear_off != AnimatedSwitcher::new(DURATION).transition_builder);
        assert!(
            tear_off
                != AnimatedSwitcherTransitionBuilder::new(|_app, child, _animation| child.clone())
        );
    }
}
