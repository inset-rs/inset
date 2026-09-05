//! Flutter counterpart: `widgets/expansible.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use reveal_animation::{
    Animation, AnimationBehavior, AnimationController, AnimationStyle, AnyAnimation, Curve,
    CurvedAnimation, Curves, Tween,
};
use reveal_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, ListenableObject, Listener,
};
use reveal_rendering::MainAxisSize;
use reveal_scheduler::{Ticker, TickerCallback, TickerProviderObject};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::basic::{Align, ClipRect, Column, Offstage};
use crate::widgets::page_storage::PageStorage;
use crate::widgets::ticker_provider::{
    SingleTickerProviderStateMixin, SingleTickerProviderStateMixinData, TickerMode,
};
use crate::widgets::transitions::AnimatedBuilder;

/// The type of the callback that returns the header or body of an [`Expansible`].
///
/// The `animation` argument exposes the underlying expanding or collapsing
/// animation, which has a value of 0 when the [`Expansible`] is completely
/// collapsed and 1 when it is completely expanded. This can be used to drive
/// animations that sync up with the expanding or collapsing animation, such as
/// rotating an icon.
///
/// See also:
///
///   * [`Expansible::header_builder`], which is of this type.
///   * [`Expansible::body_builder`], which is also of this type.
pub type ExpansibleComponentBuilder =
    Rc<dyn Fn(&mut App, BuildContext, AnyAnimation<f64>) -> WidgetRef>;

/// The type of the callback that uses the header and body of an [`Expansible`]
/// widget to build the widget.
///
/// The `header` argument is the header returned by [`Expansible::header_builder`].
/// The `body` argument is the body returned by [`Expansible::body_builder`] wrapped
/// in an [`Offstage`] to hide the body when the [`Expansible`] is collapsed.
///
/// The `animation` argument exposes the underlying expanding or collapsing
/// animation, which has a value of 0 when the [`Expansible`] is completely
/// collapsed and 1 when it is completely expanded. This can be used to drive
/// animations that sync up with the expanding or collapsing animation, such as
/// rotating an icon.
///
/// See also:
///
///   * [`Expansible::expansible_builder`], which is of this type.
pub type ExpansibleBuilder =
    Rc<dyn Fn(&mut App, BuildContext, &WidgetRef, &WidgetRef, AnyAnimation<f64>) -> WidgetRef>;

/// A controller for managing the expansion state of an [`Expansible`].
///
/// This is a [`ChangeNotifier`] that notifies its listeners if the value of
/// [`is_expanded`](Self::is_expanded) changes.
///
/// This controller provides methods to programmatically expand or collapse the
/// widget, and it allows external components to query the current expansion
/// state.
///
/// The controller's [`expand`](Self::expand) and [`collapse`](Self::collapse) methods cause
/// the [`Expansible`] to rebuild, so they may not be called from a build method.
///
/// Remember to [`dispose`](Self::dispose) of the [`ExpansibleController`] when it is no
/// longer needed. This will ensure all resources used by the object are discarded.
#[derive(Debug, Default)]
pub struct ExpansibleController {
    change_notifier: ChangeNotifierData,
    is_expanded: bool,
}

impl ExpansibleController {
    /// Creates a controller to be used with [`Expansible::controller`].
    pub fn new(app: &mut App) -> Handle<ExpansibleController> {
        app.create(ExpansibleController::default())
    }

    fn set_expansion_state(self: Handle<Self>, app: &mut App, new_value: bool) {
        if new_value != app.get(self).is_expanded {
            app.get_mut(self).is_expanded = new_value;
            self.notify_listeners(app);
        }
    }

    /// Whether the expansible widget built with this controller is in expanded
    /// state.
    ///
    /// This property doesn't take the animation into account. It reports `true`
    /// even if the expansion animation is not completed.
    ///
    /// To be notified when this property changes, add a listener to the
    /// controller.
    ///
    /// See also:
    ///
    ///  * [`expand`](Self::expand), which expands the expansible widget.
    ///  * [`collapse`](Self::collapse), which collapses the expansible widget.
    pub fn is_expanded(self: Handle<Self>, app: &App) -> bool {
        app.get(self).is_expanded
    }

    /// Expands the [`Expansible`] that was built with this controller.
    ///
    /// If the widget is already in the expanded state (see
    /// [`is_expanded`](Self::is_expanded)), calling this method has no effect.
    ///
    /// Calling this method may cause the [`Expansible`] to rebuild, so it may
    /// not be called from a build method.
    ///
    /// Calling this method will notify registered listeners of this controller
    /// that the expansion state has changed.
    ///
    /// See also:
    ///
    ///  * [`collapse`](Self::collapse), which collapses the expansible widget.
    ///  * [`is_expanded`](Self::is_expanded) to check whether the expansible widget is
    ///    expanded.
    pub fn expand(self: Handle<Self>, app: &mut App) {
        self.set_expansion_state(app, true);
    }

    /// Collapses the [`Expansible`] that was built with this controller.
    ///
    /// If the widget is already in the collapsed state (see
    /// [`is_expanded`](Self::is_expanded)), calling this method has no effect.
    ///
    /// Calling this method may cause the [`Expansible`] to rebuild, so it may not
    /// be called from a build method.
    ///
    /// Calling this method will notify registered listeners of this controller
    /// that the expansion state has changed.
    ///
    /// See also:
    ///
    ///  * [`expand`](Self::expand), which expands the [`Expansible`].
    ///  * [`is_expanded`](Self::is_expanded) to check whether the [`Expansible`] is expanded.
    pub fn collapse(self: Handle<Self>, app: &mut App) {
        self.set_expansion_state(app, false);
    }

    /// Convenience method for toggling the current [`is_expanded`](Self::is_expanded) status.
    ///
    /// Calling this method may cause the [`Expansible`] to rebuild, so it may not
    /// be called from a build method.
    ///
    /// Calling this method will notify registered listeners of this controller
    /// that the expansion state has changed.
    ///
    /// See also:
    ///
    ///  * [`expand`](Self::expand), which expands the [`Expansible`].
    ///  * [`collapse`](Self::collapse), which collapses the [`Expansible`].
    ///  * [`is_expanded`](Self::is_expanded) to check whether the [`Expansible`] is expanded.
    pub fn toggle(self: Handle<Self>, app: &mut App) {
        if self.is_expanded(app) {
            self.collapse(app);
        } else {
            self.expand(app);
        }
    }

    /// Discards any resources used by the object; the handle is stale afterwards.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).change_notifier.dispose();
        app.destroy(self);
    }

    /// Finds the [`ExpansibleController`] for the closest [`Expansible`] instance
    /// that encloses the given context.
    ///
    /// If no [`Expansible`] encloses the given context, calling this method panics.
    ///
    /// To return `None` if there is no [`Expansible`] use [`maybe_of`](Self::maybe_of)
    /// instead.
    ///
    /// Typical usage of the [`of`](Self::of) function is to call it from
    /// within the `build` method of a descendant of an [`Expansible`].
    pub fn of(app: &App, context: BuildContext) -> Handle<ExpansibleController> {
        ExpansibleController::maybe_of(app, context).expect(
            "ExpansibleController::of() called with a context that does not contain an \
             Expansible. No Expansible ancestor could be found starting from the context that \
             was passed to ExpansibleController::of(). This usually happens when the context \
             provided is from the same StatefulWidget as that whose build function actually \
             creates the Expansible widget being sought.",
        )
    }

    /// Finds the [`Expansible`] from the closest instance of this class that
    /// encloses the given context and returns its [`ExpansibleController`].
    ///
    /// If no [`Expansible`] encloses the given context then returns `None`.
    /// To panic instead, use [`of`](Self::of).
    ///
    /// See also:
    ///
    ///  * [`of`](Self::of), a similar function to this one that panics if no [`Expansible`]
    ///    encloses the given context.
    pub fn maybe_of(app: &App, context: BuildContext) -> Option<Handle<ExpansibleController>> {
        context
            .find_ancestor_state_of_type::<ExpansibleState>(app)
            .map(|state| state.widget(app).controller)
    }
}

impl ChangeNotifier for ExpansibleController {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

/// A [`StatefulWidget`] that expands and collapses.
///
/// An [`Expansible`] consists of a header, which is always shown, and a
/// body, which is hidden in its collapsed state and shown in its expanded
/// state.
///
/// The [`Expansible`] is expanded or collapsed with an animation driven by an
/// [`AnimationController`]. When the widget is expanded, the height of its body
/// animates from 0 to its fully expanded height.
///
/// This widget is typically used with `ListView` to create an "expand /
/// collapse" list entry. When used with scrolling widgets like `ListView`, a
/// unique `PageStorageKey` must be specified as the [`key`](Self::key), to enable the
/// [`Expansible`] to save and restore its expanded state when it is scrolled
/// in and out of view.
///
/// Provide [`header_builder`](Self::header_builder) and
/// [`body_builder`](Self::body_builder) callbacks to build the header and body widgets. An
/// additional [`expansible_builder`](Self::expansible_builder) callback can be provided to
/// further customize the layout of the widget.
///
/// The [`Expansible`] does not inherently toggle the expansion state. To toggle
/// the expansion state, call [`ExpansibleController::expand`] and
/// [`ExpansibleController::collapse`] as needed, most typically when the header
/// returned in [`header_builder`](Self::header_builder) is tapped.
///
/// See also:
///
///  * `ExpansionTile`, a Material-styled widget that expands and collapses.
pub struct Expansible {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// Expands and collapses the widget.
    ///
    /// The controller manages the expansion state and toggles the expansion.
    pub controller: Handle<ExpansibleController>,

    /// Builds the always-displayed header.
    ///
    /// Many use cases involve toggling the expansion state when this header is
    /// tapped. To toggle the expansion state, call [`ExpansibleController::expand`]
    /// or [`ExpansibleController::collapse`].
    pub header_builder: ExpansibleComponentBuilder,

    /// Builds the collapsible body.
    ///
    /// When this widget is expanded, the height of its body animates from 0 to
    /// its fully extended height.
    pub body_builder: ExpansibleComponentBuilder,

    /// Used to override the expansion animation curve and duration.
    ///
    /// If [`AnimationStyle::duration`] is provided, it will be used instead of
    /// [`duration`](Self::duration). If not provided, [`duration`](Self::duration) is used,
    /// which defaults to 200ms.
    ///
    /// If [`AnimationStyle::curve`] is provided, it will be used to override
    /// [`curve`](Self::curve). If it is `None`, then [`curve`](Self::curve) will be used.
    /// Otherwise, defaults to [`Curves::ease`].
    ///
    /// If [`AnimationStyle::reverse_curve`] is provided, it will be used to
    /// override [`reverse_curve`](Self::reverse_curve). If it is `None`, then
    /// [`reverse_curve`](Self::reverse_curve) will be used.
    ///
    /// To disable the theme animation, use [`AnimationStyle::NO_ANIMATION`].
    pub animation_style: Option<AnimationStyle>,

    /// The duration of the expansion animation.
    ///
    /// Defaults to a duration of 200ms.
    ///
    /// This property is deprecated, use [`animation_style`](Self::animation_style) instead.
    #[deprecated(
        note = "Use animation_style instead. This feature was deprecated after v3.38.0-0.2.pre."
    )]
    pub duration: Duration,

    /// The curve of the expansion animation.
    ///
    /// Defaults to [`Curves::ease`].
    ///
    /// This property is deprecated, use [`animation_style`](Self::animation_style) instead.
    #[deprecated(
        note = "Use animation_style instead. This feature was deprecated after v3.38.0-0.2.pre."
    )]
    pub curve: Rc<dyn Curve>,

    /// The reverse curve of the expansion animation.
    ///
    /// If `None`, uses [`curve`](Self::curve) in both directions.
    ///
    /// This property is deprecated, use [`animation_style`](Self::animation_style) instead.
    #[deprecated(
        note = "Use animation_style instead. This feature was deprecated after v3.38.0-0.2.pre."
    )]
    pub reverse_curve: Option<Rc<dyn Curve>>,

    /// Whether the state of the body is maintained when the widget expands or
    /// collapses.
    ///
    /// If true, the body is kept in the tree while the widget is
    /// collapsed. Otherwise, the body is removed from the tree when the
    /// widget is collapsed and recreated upon expansion.
    ///
    /// Defaults to true.
    pub maintain_state: bool,

    /// Builds the widget with the results of [`header_builder`](Self::header_builder) and
    /// [`body_builder`](Self::body_builder).
    ///
    /// Defaults to placing the header and body in a [`Column`].
    pub expansible_builder: ExpansibleBuilder,
}

impl Expansible {
    /// Creates an instance of [`Expansible`]; Dart's optional named arguments are the
    /// setters.
    #[allow(deprecated)]
    pub fn new(
        header_builder: ExpansibleComponentBuilder,
        body_builder: ExpansibleComponentBuilder,
        controller: Handle<ExpansibleController>,
    ) -> Expansible {
        Expansible {
            key: None,
            controller,
            header_builder,
            body_builder,
            animation_style: None,
            duration: Duration::from_millis(200),
            curve: Curves::ease(),
            reverse_curve: None,
            maintain_state: true,
            expansible_builder: Rc::new(Expansible::default_expansible_builder),
        }
    }

    /// Dart `Expansible(key:)`.
    pub fn key(mut self, key: KeyRef) -> Expansible {
        self.key = Some(key);
        self
    }

    /// Dart `Expansible(expansibleBuilder:)`.
    pub fn expansible_builder(mut self, expansible_builder: ExpansibleBuilder) -> Expansible {
        self.expansible_builder = expansible_builder;
        self
    }

    /// Dart `Expansible(animationStyle:)`.
    pub fn animation_style(mut self, animation_style: AnimationStyle) -> Expansible {
        self.animation_style = Some(animation_style);
        self
    }

    /// Dart `Expansible(duration:)`.
    #[deprecated(
        note = "Use animation_style instead. This feature was deprecated after v3.38.0-0.2.pre."
    )]
    #[allow(deprecated)]
    pub fn duration(mut self, duration: Duration) -> Expansible {
        self.duration = duration;
        self
    }

    /// Dart `Expansible(curve:)`.
    #[deprecated(
        note = "Use animation_style instead. This feature was deprecated after v3.38.0-0.2.pre."
    )]
    #[allow(deprecated)]
    pub fn curve(mut self, curve: Rc<dyn Curve>) -> Expansible {
        self.curve = curve;
        self
    }

    /// Dart `Expansible(reverseCurve:)`.
    #[deprecated(
        note = "Use animation_style instead. This feature was deprecated after v3.38.0-0.2.pre."
    )]
    #[allow(deprecated)]
    pub fn reverse_curve(mut self, reverse_curve: Rc<dyn Curve>) -> Expansible {
        self.reverse_curve = Some(reverse_curve);
        self
    }

    /// Dart `Expansible(maintainState:)`.
    pub fn maintain_state(mut self, maintain_state: bool) -> Expansible {
        self.maintain_state = maintain_state;
        self
    }

    /// Dart's `_defaultExpansibleBuilder`.
    fn default_expansible_builder(
        _app: &mut App,
        _context: BuildContext,
        header: &WidgetRef,
        body: &WidgetRef,
        _animation: AnyAnimation<f64>,
    ) -> WidgetRef {
        Column::new()
            .main_axis_size(MainAxisSize::Min)
            .children([header.clone(), body.clone()])
            .into_widget()
    }
}

impl Debug for Expansible {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Expansible")
            .field("controller", &self.controller)
            .field("maintain_state", &self.maintain_state)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for Expansible {
    type State = ExpansibleState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ExpansibleState {
        ExpansibleState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::default(),
            animation_controller: None,
            height_factor_tween: None,
            height_factor: None,
        }
    }
}

/// Dart's `_ExpansibleState`.
pub struct ExpansibleState {
    state: StateData<Expansible>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    /// Created in `init_state`, because a state is created without the [`App`].
    animation_controller: Option<Handle<AnimationController>>,
    height_factor_tween: Option<Handle<Tween<f64>>>,
    height_factor: Option<Handle<CurvedAnimation>>,
}

impl ExpansibleState {
    fn animation_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .animation_controller
            .expect("created in init_state")
    }

    fn height_factor(self: Handle<Self>, app: &App) -> Handle<CurvedAnimation> {
        app.get(self).height_factor.expect("created in init_state")
    }

    #[allow(deprecated)]
    fn duration(widget: &Expansible) -> Duration {
        widget
            .animation_style
            .as_ref()
            .and_then(|style| style.duration)
            .unwrap_or(widget.duration)
    }

    #[allow(deprecated)]
    fn curve(widget: &Expansible) -> Rc<dyn Curve> {
        widget
            .animation_style
            .as_ref()
            .and_then(|style| style.curve.clone())
            .unwrap_or_else(|| widget.curve.clone())
    }

    #[allow(deprecated)]
    fn reverse_curve(widget: &Expansible) -> Option<Rc<dyn Curve>> {
        widget
            .animation_style
            .as_ref()
            .and_then(|style| style.reverse_curve.clone())
            .or_else(|| widget.reverse_curve.clone())
    }

    fn toggle_expansion(self: Handle<Self>, app: &mut App) {
        let controller = self.widget(app).controller;
        let animation_controller = self.animation_controller(app);
        if controller.is_expanded(app) {
            animation_controller.forward(app, None);
        } else {
            let reversal = animation_controller.reverse(app, None);
            reversal.when_complete(
                app,
                Listener::new(move |app| {
                    if !self.mounted(app) {
                        return;
                    }
                    self.set_state(app, |_state| {
                        // Rebuild without the body.
                    });
                }),
            );
        }
        let is_expanded = controller.is_expanded(app);
        let context = self.context(app);
        if let Some(bucket) = PageStorage::maybe_of(app, context) {
            bucket.write_state(app, context, Rc::new(is_expanded), None);
        }
        self.set_state(app, |_state| {
            // Rebuild with the header and the animating body.
        });
    }
}

impl SingleTickerProviderStateMixin for ExpansibleState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }

    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

/// Dart's `vsync: this`.
impl TickerProviderObject for ExpansibleState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for ExpansibleState {
    type Widget = Expansible;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let duration = ExpansibleState::duration(self.widget(app));
        let animation_controller = AnimationController::create(
            app,
            None,
            Some(duration),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        app.get_mut(self).animation_controller = Some(animation_controller);
        let controller = self.widget(app).controller;
        let context = self.context(app);
        let stored = PageStorage::maybe_of(app, context)
            .and_then(|bucket| bucket.read_state(app, context, None))
            .and_then(|data| data.downcast_ref::<bool>().copied());
        let initially_expanded = stored.unwrap_or_else(|| controller.is_expanded(app));
        if initially_expanded {
            animation_controller.set_value(app, 1.0);
            controller.expand(app);
        } else {
            controller.collapse(app);
        }
        let height_factor_tween = Tween::new(app, Some(0.0), Some(1.0));
        let parent = animation_controller.drive(app, height_factor_tween);
        let curve = ExpansibleState::curve(self.widget(app));
        let reverse_curve = ExpansibleState::reverse_curve(self.widget(app));
        let height_factor = CurvedAnimation::create(app, parent, curve, reverse_curve);
        let state = app.get_mut(self);
        state.height_factor_tween = Some(height_factor_tween);
        state.height_factor = Some(height_factor);
        controller.add_listener(app, Listener::handle_method(self, Self::toggle_expansion));
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &Expansible) {
        let old_duration = ExpansibleState::duration(old_widget);
        let old_curve = ExpansibleState::curve(old_widget);
        let old_reverse_curve = ExpansibleState::reverse_curve(old_widget);
        let curve = ExpansibleState::curve(self.widget(app));
        let reverse_curve = ExpansibleState::reverse_curve(self.widget(app));
        let duration = ExpansibleState::duration(self.widget(app));

        let height_factor = self.height_factor(app);
        if !Rc::ptr_eq(&curve, &old_curve) {
            app.get_mut(height_factor).curve = curve;
        }
        if !curves_identical(&reverse_curve, &old_reverse_curve) {
            app.get_mut(height_factor).reverse_curve = reverse_curve;
        }
        if duration != old_duration {
            app.get_mut(self.animation_controller(app)).duration = Some(duration);
        }
        let controller = self.widget(app).controller;
        if controller != old_widget.controller {
            old_widget
                .controller
                .remove_listener(app, &Listener::handle_method(self, Self::toggle_expansion));
            controller.add_listener(app, Listener::handle_method(self, Self::toggle_expansion));
            if old_widget.controller.is_expanded(app) != controller.is_expanded(app) {
                self.toggle_expansion(app);
            }
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let controller = self.widget(app).controller;
        controller.remove_listener(app, &Listener::handle_method(self, Self::toggle_expansion));
        let animation_controller = self.animation_controller(app);
        animation_controller.dispose(app);
        let height_factor = self.height_factor(app);
        height_factor.dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
        let state = app.get_mut(self);
        let height_factor_tween = state.height_factor_tween.take();
        state.animation_controller = None;
        state.height_factor = None;
        app.destroy(animation_controller);
        app.destroy(height_factor);
        if let Some(height_factor_tween) = height_factor_tween {
            app.destroy(height_factor_tween);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let animation_controller = self.animation_controller(app);
        let controller = self.widget(app).controller;
        let is_expanded = controller.is_expanded(app);
        debug_assert!(!animation_controller.is_dismissed(app) || !is_expanded);
        let closed = !is_expanded && animation_controller.is_dismissed(app);
        let should_remove_body = closed && !self.widget(app).maintain_state;

        let animation = animation_controller.view();
        let body_builder = self.widget(app).body_builder.clone();
        let body = body_builder(app, context, animation);
        let result = Offstage::new()
            .offstage(closed)
            .child(TickerMode::new(!closed, body))
            .into_widget();

        let header_builder = self.widget(app).header_builder.clone();
        let expansible_builder = self.widget(app).expansible_builder.clone();
        let height_factor = self.height_factor(app);
        let mut builder = AnimatedBuilder::new(Rc::new(animation), move |app, context, child| {
            let header = header_builder(app, context, animation);
            let mut align = Align::new().height_factor(height_factor.value(app));
            if let Some(child) = child {
                align = align.child(child.clone());
            }
            let body = ClipRect::new().child(align).into_widget();
            expansible_builder(app, context, &header, &body, animation)
        });
        if !should_remove_body {
            builder = builder.child(result);
        }
        builder.into_widget()
    }
}

/// Dart's `Curve? != Curve?`, which is identity on the `const` curve instances.
fn curves_identical(a: &Option<Rc<dyn Curve>>, b: &Option<Rc<dyn Curve>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use reveal_embedder::{Size, TextDirection};
    use reveal_rendering::{
        AnyRenderObject, BoxConstraints, RenderBox, RenderConstrainedBox, RenderHandle,
        RenderOffstage,
    };
    use reveal_scheduler::SchedulerBinding;

    use super::*;
    use crate::framework::{LeafRenderObjectWidget, RenderObjectWidget};
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Directionality, SizedBox};
    use crate::widgets::page_storage::{PageStorageBucket, PageStorageKey};

    /// `SizedBox` in miniature, so a subtree's height is easy to read back.
    #[derive(Debug)]
    struct Sized {
        size: Size,
    }

    impl RenderObjectWidget for Sized {
        type RenderObject = RenderConstrainedBox;

        fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
            RenderConstrainedBox::new(app, BoxConstraints::tight(self.size), None).as_object()
        }

        fn update_render_object(
            &self,
            app: &mut App,
            _context: BuildContext,
            render_object: RenderHandle<RenderConstrainedBox>,
        ) {
            render_object.set_additional_constraints(app, BoxConstraints::tight(self.size));
        }
    }

    impl LeafRenderObjectWidget for Sized {}

    fn sized(width: f64, height: f64) -> Sized {
        Sized {
            size: Size::new(width, height),
        }
    }

    fn header_builder() -> ExpansibleComponentBuilder {
        Rc::new(|_app, _context, _animation| sized(100.0, 20.0).into_widget())
    }

    fn body_builder(builds: &Rc<Cell<u32>>) -> ExpansibleComponentBuilder {
        let builds = Rc::clone(builds);
        Rc::new(move |_app, _context, _animation| {
            builds.set(builds.get() + 1);
            sized(100.0, 40.0).into_widget()
        })
    }

    fn expansible(controller: Handle<ExpansibleController>, builds: &Rc<Cell<u32>>) -> Expansible {
        Expansible::new(header_builder(), body_builder(builds), controller)
    }

    fn mount(app: &mut App, widget: Expansible) -> Harness {
        Harness::mount(
            app,
            Directionality::new(TextDirection::Ltr, widget).into_widget(),
        )
    }

    fn children(app: &App, object: AnyRenderObject) -> Vec<AnyRenderObject> {
        let mut children = Vec::new();
        object.visit_children(app, &mut |child| children.push(child));
        children
    }

    fn only_child(app: &App, object: AnyRenderObject) -> Option<AnyRenderObject> {
        children(app, object).first().copied()
    }

    /// The `Column` the default expansible builder returns.
    fn column(harness: &Harness, app: &App) -> AnyRenderObject {
        harness
            .render_root(app)
            .child(app)
            .expect("a child")
            .as_object()
    }

    /// The `Column`'s second child: the `ClipRect` wrapping the body.
    fn body_clip(harness: &Harness, app: &App) -> AnyRenderObject {
        children(app, column(harness, app))[1]
    }

    fn body_height(harness: &Harness, app: &App) -> f64 {
        body_clip(harness, app)
            .as_box()
            .expect("a box")
            .size(app)
            .height()
    }

    /// The `Align` under the body's `ClipRect`.
    fn body_align(harness: &Harness, app: &App) -> AnyRenderObject {
        only_child(app, body_clip(harness, app)).expect("the Align")
    }

    fn offstage(harness: &Harness, app: &App) -> bool {
        only_child(app, body_align(harness, app))
            .expect("the Offstage")
            .downcast::<RenderOffstage>(app)
            .expect("a RenderOffstage")
            .offstage(app)
    }

    /// Runs frames until the expansion animation settles.
    fn settle(harness: &Harness, app: &mut App, from: Duration) -> Duration {
        let mut at = from;
        for _ in 0..20 {
            at += Duration::from_millis(20);
            SchedulerBinding::handle_begin_frame(app, Some(at));
            app.drain_microtasks();
            SchedulerBinding::handle_draw_frame(app);
            app.drain_microtasks();
            harness.pump(app);
        }
        at
    }

    // expansible_test.dart 'Expansible expands and collapses'
    #[test]
    fn an_expansible_expands_and_collapses_through_its_controller() {
        let mut app = App::new();
        let controller = ExpansibleController::new(&mut app);
        let builds = Rc::new(Cell::new(0));
        let harness = mount(&mut app, expansible(controller, &builds));
        harness.pump(&mut app);
        assert!(!controller.is_expanded(&app));
        assert_eq!(body_height(&harness, &app), 0.0);
        assert!(offstage(&harness, &app), "the body is hidden when closed");

        controller.expand(&mut app);
        harness.pump(&mut app);
        assert!(controller.is_expanded(&app));
        assert!(!offstage(&harness, &app), "the body is on stage at once");
        let at = settle(&harness, &mut app, Duration::ZERO);
        assert_eq!(body_height(&harness, &app), 40.0);

        controller.collapse(&mut app);
        harness.pump(&mut app);
        settle(&harness, &mut app, at);
        assert_eq!(body_height(&harness, &app), 0.0);
        assert!(offstage(&harness, &app), "offstage again once dismissed");

        controller.dispose(&mut app);
    }

    // expansible_test.dart 'Expansible maintainState'
    #[test]
    fn a_collapsed_expansible_removes_its_body_when_maintain_state_is_false() {
        let mut app = App::new();
        let controller = ExpansibleController::new(&mut app);
        let builds = Rc::new(Cell::new(0));
        let harness = mount(
            &mut app,
            expansible(controller, &builds).maintain_state(false),
        );
        harness.pump(&mut app);
        assert!(
            only_child(&app, body_align(&harness, &app)).is_none(),
            "the body is not in the tree while closed"
        );

        controller.expand(&mut app);
        harness.pump(&mut app);
        assert!(
            only_child(&app, body_align(&harness, &app)).is_some(),
            "expanding puts it back"
        );
        assert!(!offstage(&harness, &app));
        let _ = builds;

        controller.dispose(&mut app);
    }

    #[test]
    fn a_collapsed_expansible_keeps_its_body_when_maintain_state_is_true() {
        let mut app = App::new();
        let controller = ExpansibleController::new(&mut app);
        let builds = Rc::new(Cell::new(0));
        let harness = mount(&mut app, expansible(controller, &builds));
        harness.pump(&mut app);
        assert_eq!(builds.get(), 1, "the body is built but offstage");
        assert!(offstage(&harness, &app));

        controller.dispose(&mut app);
    }

    // expansible_test.dart 'Expansible restores state from PageStorage'
    #[test]
    fn an_expansible_restores_its_expanded_state_from_page_storage() {
        let mut app = App::new();
        let bucket = PageStorageBucket::new(&mut app);
        let controller = ExpansibleController::new(&mut app);
        let builds = Rc::new(Cell::new(0));
        let page = |controller| {
            PageStorage::new(
                bucket,
                Directionality::new(
                    TextDirection::Ltr,
                    expansible(controller, &builds).key(PageStorageKey::new("tile").into_key()),
                ),
            )
            .into_widget()
        };
        let harness = Harness::mount(&mut app, page(controller));
        harness.pump(&mut app);
        controller.expand(&mut app);
        harness.pump(&mut app);
        settle(&harness, &mut app, Duration::ZERO);
        assert_eq!(body_height(&harness, &app), 40.0);

        // A fresh controller and state, as a tile scrolled out of view and back gets.
        let replacement = ExpansibleController::new(&mut app);
        harness.set_child(&mut app, SizedBox::new().into_widget());
        harness.pump(&mut app);
        harness.set_child(&mut app, page(replacement));
        harness.pump(&mut app);
        assert!(
            replacement.is_expanded(&app),
            "the bucket told the new state it was open"
        );
        assert_eq!(body_height(&harness, &app), 40.0);

        controller.dispose(&mut app);
        replacement.dispose(&mut app);
    }

    // expansible_test.dart 'ExpansibleController.of'
    #[test]
    fn expansible_controller_of_finds_the_enclosing_expansible() {
        let mut app = App::new();
        let controller = ExpansibleController::new(&mut app);
        let found: Rc<Cell<Option<Handle<ExpansibleController>>>> = Rc::new(Cell::new(None));
        let header: ExpansibleComponentBuilder = Rc::new({
            let found = Rc::clone(&found);
            move |app, context, _animation| {
                found.set(Some(ExpansibleController::of(app, context)));
                sized(100.0, 20.0).into_widget()
            }
        });
        let builds = Rc::new(Cell::new(0));
        let harness = mount(
            &mut app,
            Expansible::new(header, body_builder(&builds), controller),
        );
        harness.pump(&mut app);
        assert_eq!(found.get(), Some(controller));

        controller.dispose(&mut app);
    }

    #[test]
    fn a_custom_expansible_builder_replaces_the_default_column() {
        let mut app = App::new();
        let controller = ExpansibleController::new(&mut app);
        let builds = Rc::new(Cell::new(0));
        let widget = expansible(controller, &builds).expansible_builder(Rc::new(
            |_app, _context, _header, body, _animation| body.clone(),
        ));
        let harness = mount(&mut app, widget);
        harness.pump(&mut app);
        // Only the body's `ClipRect` is left under the root.
        assert_eq!(
            column(&harness, &app)
                .as_box()
                .expect("a box")
                .size(&app)
                .height(),
            0.0
        );

        controller.dispose(&mut app);
    }
}
