//! Flutter counterpart: the `Visibility` half of `widgets/indexed_stack.dart`.
//!
//! Flutter deleted `widgets/visibility.dart` and moved `Visibility`, `SliverVisibility` and
//! their shared `_VisibilityScope` next to `IndexedStack`. `IndexedStack` and the scope live
//! in `basic.rs`, where the rest of `indexed_stack.dart` was ported; `Visibility` keeps the
//! file name Flutter used until then. `SliverVisibility` waits on the sliver widgets it
//! builds (see `PORTING.md`).

use inset_embedder::{Matrix4, Offset, Size, TextBaseline};
use inset_foundation::App;
use inset_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, PaintingContext, RenderBox,
    RenderBoxData, RenderHandle, RenderObject, RenderObjectData, RenderObjectWithChildData,
    RenderObjectWithChildMixin, RenderProxyBoxMixin,
};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, RenderObjectWidget, SingleChildRenderObjectWidget,
    StatelessWidget, WidgetRef, downcast_widget,
};
use crate::widgets::basic::{IgnorePointer, Offstage, SizedBox, VisibilityScope};
use crate::widgets::focus_scope::ExcludeFocus;
use crate::widgets::ticker_provider::TickerMode;

/// Whether to show or hide a child.
///
/// By default, the [`visible`](Self::visible) property controls whether the
/// [`child`](Self::child) is included in the subtree or not; when it is not
/// [`visible`](Self::visible), the [`replacement`](Self::replacement) child (typically a
/// zero-sized box) is included instead.
///
/// A variety of flags can be used to tweak exactly how the child is hidden.
/// (Changing the flags dynamically is discouraged, as it can cause the
/// [`child`](Self::child) subtree to be rebuilt, with any state in the subtree being
/// discarded. Typically, only the [`visible`](Self::visible) flag is changed dynamically.)
///
/// These widgets provide some of the facets of this one:
///
///  * [`Opacity`](crate::Opacity), which can stop its child from being painted.
///  * [`Offstage`], which can stop its child from being laid out or painted.
///  * [`TickerMode`], which can stop its child from being animated.
///  * `ExcludeSemantics`, which can hide the child from accessibility tools.
///  * [`IgnorePointer`], which can disable touch interactions with the child.
///
/// Using this widget is not necessary to hide children. The simplest way to hide a child is
/// just to not include it, or, if a child _must_ be given (e.g. because the parent is a
/// stateless widget) then to use [`SizedBox::shrink`] instead of the child that would
/// otherwise be included.
///
/// See also:
///
///  * `AnimatedSwitcher`, which can fade from one child to the next as the subtree changes.
///  * `AnimatedCrossFade`, which can fade between two specific children.
///  * `SliverVisibility`, the sliver equivalent of this widget.
#[derive(Debug)]
pub struct Visibility {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,

    /// The widget to show or hide, as controlled by [`visible`](Self::visible).
    pub child: WidgetRef,

    /// The widget to use when the child is not [`visible`](Self::visible), assuming that none
    /// of the `maintain` flags (in particular, [`maintain_state`](Self::maintain_state)) are
    /// set.
    ///
    /// The normal behavior is to replace the widget with a zero by zero box
    /// ([`SizedBox::shrink`]).
    ///
    /// See also:
    ///
    ///  * `AnimatedCrossFade`, which can animate between two children.
    pub replacement: WidgetRef,

    /// Switches between showing the [`child`](Self::child) or hiding it.
    ///
    /// The `maintain` flags should be set to the same values regardless of the state of the
    /// [`visible`](Self::visible) property, otherwise they will not operate correctly
    /// (specifically, the state will be lost regardless of the state of
    /// [`maintain_state`](Self::maintain_state) whenever any of the `maintain` flags are
    /// changed, since doing so will result in a subtree shape change).
    ///
    /// Unless [`maintain_state`](Self::maintain_state) is set, the [`child`](Self::child)
    /// subtree will be disposed (removed from the tree) while hidden.
    pub visible: bool,

    /// Whether to maintain the `State` objects of the [`child`](Self::child) subtree when it
    /// is not [`visible`](Self::visible).
    ///
    /// Keeping the state of the subtree is potentially expensive (because it means all the
    /// objects are still in memory; their resources are not released). It should only be
    /// maintained if it cannot be recreated on demand. One example of when the state would be
    /// maintained is if the child subtree contains a [`Navigator`](crate::Navigator), since
    /// that widget maintains elaborate state that cannot be recreated on the fly.
    ///
    /// If this property is true, an [`Offstage`] widget is used to hide the child instead of
    /// replacing it with [`replacement`](Self::replacement).
    ///
    /// If this property is false, then [`maintain_animation`](Self::maintain_animation) must
    /// also be false.
    ///
    /// If this property is false, then [`maintain_focusability`](Self::maintain_focusability)
    /// must also be false.
    ///
    /// Dynamically changing this value may cause the current state of the subtree to be lost
    /// (and a new instance of the subtree, with new `State` objects, to be immediately created
    /// if [`visible`](Self::visible) is true).
    pub maintain_state: bool,

    /// Whether to maintain animations within the [`child`](Self::child) subtree when it is not
    /// [`visible`](Self::visible).
    ///
    /// To set this, [`maintain_state`](Self::maintain_state) must also be set.
    ///
    /// Keeping animations active when the widget is not visible is even more expensive than
    /// only maintaining the state.
    ///
    /// One example when this might be useful is if the subtree is animating its layout in time
    /// with an `AnimationController`, and the result of that layout is being used to influence
    /// some other logic. If this flag is false, then any `AnimationController`s hosted inside
    /// the [`child`](Self::child) subtree will be muted while the [`visible`](Self::visible)
    /// flag is false.
    ///
    /// If this property is true, no [`TickerMode`] widget is used.
    ///
    /// If this property is false, then [`maintain_size`](Self::maintain_size) must also be
    /// false.
    ///
    /// Dynamically changing this value may cause the current state of the subtree to be lost
    /// (and a new instance of the subtree, with new `State` objects, to be immediately created
    /// if [`visible`](Self::visible) is true).
    pub maintain_animation: bool,

    /// Whether to maintain space for where the widget would have been.
    ///
    /// To set this, [`maintain_animation`](Self::maintain_animation) and
    /// [`maintain_state`](Self::maintain_state) must also be set.
    ///
    /// Maintaining the size when the widget is not [`visible`](Self::visible) is not notably
    /// more expensive than just keeping animations running without maintaining the size, and
    /// may in some circumstances be slightly cheaper if the subtree is simple and the
    /// [`visible`](Self::visible) property is frequently toggled, since it avoids triggering a
    /// layout change when the [`visible`](Self::visible) property is toggled. If the
    /// [`child`](Self::child) subtree is not trivial then it is significantly cheaper to not
    /// even keep the state (see [`maintain_state`](Self::maintain_state)).
    ///
    /// If this property is false, [`Offstage`] is used.
    ///
    /// If this property is false, then [`maintain_semantics`](Self::maintain_semantics) and
    /// [`maintain_interactivity`](Self::maintain_interactivity) must also be false.
    ///
    /// Dynamically changing this value may cause the current state of the subtree to be lost
    /// (and a new instance of the subtree, with new `State` objects, to be immediately created
    /// if [`visible`](Self::visible) is true).
    ///
    /// See also:
    ///
    ///  * `AnimatedOpacity` and [`FadeTransition`](crate::FadeTransition), which apply
    ///    animations to the opacity for a more subtle effect.
    pub maintain_size: bool,

    /// Whether to maintain the semantics for the widget when it is hidden (e.g. for
    /// accessibility).
    ///
    /// To set this, [`maintain_size`](Self::maintain_size) must also be set.
    ///
    /// By default, with [`maintain_semantics`](Self::maintain_semantics) set to false, the
    /// [`child`](Self::child) is not visible to accessibility tools when it is hidden from the
    /// user. If this flag is set to true, then accessibility tools will report the widget as
    /// if it was present.
    ///
    /// Nothing reads this flag while accessibility is deferred (see `PORTING.md`).
    pub maintain_semantics: bool,

    /// Whether to allow the widget to be interactive when hidden.
    ///
    /// To set this, [`maintain_size`](Self::maintain_size) must also be set.
    ///
    /// By default, with [`maintain_interactivity`](Self::maintain_interactivity) set to false,
    /// touch events cannot reach the [`child`](Self::child) when it is hidden from the user.
    /// If this flag is set to true, then touch events will nonetheless be passed through.
    pub maintain_interactivity: bool,

    /// Whether to allow the widget to receive focus when hidden. Only in effect if
    /// [`visible`](Self::visible) is false.
    ///
    /// To set this to true, [`maintain_state`](Self::maintain_state) must also be set to true.
    ///
    /// By default, with [`maintain_focusability`](Self::maintain_focusability) set to false,
    /// focus events cannot reach the [`child`](Self::child) when this widget is not
    /// [`visible`](Self::visible) because an [`ExcludeFocus`] widget is used to exclude the
    /// child subtree from the focus tree. If this flag is set to true, then focus events will
    /// reach the child subtree.
    pub maintain_focusability: bool,
}

impl Visibility {
    /// Control whether the given [`child`](Self::child) is [`visible`](Self::visible); Dart's
    /// optional named arguments are the setters.
    ///
    /// The [`maintain_semantics`](Self::maintain_semantics) and
    /// [`maintain_interactivity`](Self::maintain_interactivity) flags can only be set if
    /// [`maintain_size`](Self::maintain_size) is set.
    ///
    /// The [`maintain_size`](Self::maintain_size) flag can only be set if
    /// [`maintain_animation`](Self::maintain_animation) is set.
    ///
    /// The [`maintain_animation`](Self::maintain_animation) flag can only be set if
    /// [`maintain_state`](Self::maintain_state) is set.
    pub fn new<K>(child: impl IntoWidget<K>) -> Visibility {
        Visibility {
            key: None,
            child: child.into_widget(),
            replacement: SizedBox::shrink().into_widget(),
            visible: true,
            maintain_state: false,
            maintain_animation: false,
            maintain_size: false,
            maintain_semantics: false,
            maintain_interactivity: false,
            maintain_focusability: false,
        }
    }

    /// Control whether the given [`child`](Self::child) is [`visible`](Self::visible).
    ///
    /// This is equivalent to [`Visibility::new`] with all "maintain" fields set to true. This
    /// constructor should be used in place of an [`Opacity`](crate::Opacity) widget that only
    /// takes on values of `0.0` or `1.0`, as it avoids extra compositing when fully opaque.
    pub fn maintain<K>(child: impl IntoWidget<K>) -> Visibility {
        Visibility {
            key: None,
            child: child.into_widget(),
            // Unused since maintain_state is always true.
            replacement: SizedBox::shrink().into_widget(),
            visible: true,
            maintain_state: true,
            maintain_animation: true,
            maintain_size: true,
            maintain_semantics: true,
            maintain_interactivity: true,
            maintain_focusability: true,
        }
    }

    /// Dart `Visibility(key:)`.
    pub fn key(mut self, key: KeyRef) -> Visibility {
        self.key = Some(key);
        self
    }

    /// Dart `Visibility(replacement:)`.
    pub fn replacement<K>(mut self, replacement: impl IntoWidget<K>) -> Visibility {
        self.replacement = replacement.into_widget();
        self
    }

    /// Dart `Visibility(visible:)`.
    pub fn visible(mut self, visible: bool) -> Visibility {
        self.visible = visible;
        self
    }

    /// Dart `Visibility(maintainState:)`.
    pub fn maintain_state(mut self, maintain_state: bool) -> Visibility {
        self.maintain_state = maintain_state;
        self
    }

    /// Dart `Visibility(maintainAnimation:)`.
    pub fn maintain_animation(mut self, maintain_animation: bool) -> Visibility {
        self.maintain_animation = maintain_animation;
        self
    }

    /// Dart `Visibility(maintainSize:)`.
    pub fn maintain_size(mut self, maintain_size: bool) -> Visibility {
        self.maintain_size = maintain_size;
        self
    }

    /// Dart `Visibility(maintainSemantics:)`.
    pub fn maintain_semantics(mut self, maintain_semantics: bool) -> Visibility {
        self.maintain_semantics = maintain_semantics;
        self
    }

    /// Dart `Visibility(maintainInteractivity:)`.
    pub fn maintain_interactivity(mut self, maintain_interactivity: bool) -> Visibility {
        self.maintain_interactivity = maintain_interactivity;
        self
    }

    /// Dart `Visibility(maintainFocusability:)`.
    pub fn maintain_focusability(mut self, maintain_focusability: bool) -> Visibility {
        self.maintain_focusability = maintain_focusability;
        self
    }

    /// Tells the visibility state of an element in the tree based off its ancestor
    /// [`Visibility`] elements.
    ///
    /// If there's one or more [`Visibility`] widgets in the ancestor tree, this will return
    /// true if and only if all of those widgets have [`visible`](Self::visible) set to true.
    /// If there is no [`Visibility`] widget in the ancestor tree of the specified build
    /// context, this will return true.
    ///
    /// This will register a dependency from the specified context on any [`Visibility`]
    /// elements in the ancestor tree, such that if any of their visibilities changes, the
    /// specified context will be rebuilt.
    pub fn of(app: &mut App, context: BuildContext) -> bool {
        let mut is_visible = true;
        let mut ancestor_context = context;
        let mut ancestor =
            ancestor_context.get_element_for_inherited_widget_of_exact_type::<VisibilityScope>(app);
        while is_visible && let Some(current) = ancestor {
            let widget = context.depend_on_inherited_element(app, current, None);
            let scope = downcast_widget::<VisibilityScope>(&*widget).expect("a VisibilityScope");
            is_visible = scope.is_visible;
            current.visit_ancestor_elements(app, &mut |parent| {
                ancestor_context = parent;
                false
            });
            ancestor = ancestor_context
                .get_element_for_inherited_widget_of_exact_type::<VisibilityScope>(app);
        }
        is_visible
    }

    /// Dart's constructor asserts, which the fluent setters can only complete at build time
    /// (see `PORTING.md`).
    fn debug_check_maintain_flags(&self) {
        debug_assert!(
            self.maintain_state || !self.maintain_animation,
            "Cannot maintain animations if the state is not also maintained."
        );
        debug_assert!(
            self.maintain_animation || !self.maintain_size,
            "Cannot maintain size if animations are not maintained."
        );
        debug_assert!(
            self.maintain_size || !self.maintain_semantics,
            "Cannot maintain semantics if size is not maintained."
        );
        debug_assert!(
            self.maintain_size || !self.maintain_interactivity,
            "Cannot maintain interactivity if size is not maintained."
        );
        debug_assert!(
            self.maintain_state || !self.maintain_focusability,
            "Cannot maintain focusability if the state is not also maintained."
        );
    }
}

impl StatelessWidget for Visibility {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        self.debug_check_maintain_flags();
        let mut result = ExcludeFocus::new(self.child.clone())
            .excluding(!self.visible && !self.maintain_focusability)
            .into_widget();
        if self.maintain_size {
            result = RawVisibility::new(self.visible)
                .child(
                    IgnorePointer::new()
                        .ignoring(!self.visible && !self.maintain_interactivity)
                        .child(result),
                )
                .into_widget();
        } else {
            debug_assert!(!self.maintain_interactivity);
            debug_assert!(!self.maintain_semantics);
            debug_assert!(!self.maintain_size);
            if self.maintain_state {
                if !self.maintain_animation {
                    result = TickerMode::new(self.visible, result).into_widget();
                }
                result = Offstage::new()
                    .offstage(!self.visible)
                    .child(result)
                    .into_widget();
            } else {
                debug_assert!(!self.maintain_animation);
                debug_assert!(!self.maintain_state);
                result = if self.visible {
                    self.child.clone()
                } else {
                    self.replacement.clone()
                };
            }
        }
        VisibilityScope {
            is_visible: self.visible,
            child: result,
        }
        .into_widget()
    }
}

/// A widget that conditionally hides its child, but without the forced compositing of
/// [`Opacity`](crate::Opacity).
///
/// A fully opaque [`Opacity`](crate::Opacity) widget is required to leave its opacity layer in
/// the layer tree. This forces all parent render objects to also composite, which can break a
/// simple scene into many different layers. This can be significantly more expensive, so the
/// issue is avoided by a specialized render object that does not ever force compositing.
///
/// Dart's `_Visibility`, which takes this name because [`Visibility`] holds its own.
#[derive(Debug)]
struct RawVisibility {
    visible: bool,
    child: Option<WidgetRef>,
}

impl RawVisibility {
    fn new(visible: bool) -> RawVisibility {
        RawVisibility {
            visible,
            child: None,
        }
    }

    fn child<K>(mut self, child: impl IntoWidget<K>) -> RawVisibility {
        self.child = Some(child.into_widget());
        self
    }
}

impl RenderObjectWidget for RawVisibility {
    type RenderObject = RenderVisibility;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderVisibility::new(app, self.visible, None).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderVisibility>,
    ) {
        render_object.set_visible(app, self.visible);
    }
}

impl SingleChildRenderObjectWidget for RawVisibility {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// The render object of [`RawVisibility`]: a proxy box that skips painting its child while it
/// is not [`visible`](Self::visible).
///
/// Dart's `_RenderVisibility`.
struct RenderVisibility {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    visible: bool,
}

impl RenderVisibility {
    fn new(
        app: &mut App,
        visible: bool,
        child: Option<AnyRenderBox>,
    ) -> RenderHandle<RenderVisibility> {
        let this = RenderHandle::new_box(
            app,
            RenderVisibility {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                visible,
            },
        );
        this.set_child(app, child);
        this
    }

    /// Whether the child is painted.
    fn visible(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).visible
    }

    /// Sets [`visible`](Self::visible).
    fn set_visible(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if value == self.visible(app) {
            return;
        }
        self.get_mut(app).visible = value;
        self.mark_needs_paint(app);
    }
}

impl RenderObjectWithChildMixin for RenderVisibility {
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

impl RenderProxyBoxMixin for RenderVisibility {}

impl RenderObject for RenderVisibility {
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
        if !self.visible(app) {
            return;
        }
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

impl RenderBox for RenderVisibility {
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

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use inset_embedder::{Size, TextDirection};
    use inset_foundation::{App, AppCell, Handle};
    use inset_rendering::{ErasedLayer, RenderConstrainedBox, RenderIgnorePointer, RenderOffstage};

    use super::*;
    use crate::framework::{State, StateData, StatefulWidget};
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Directionality, IndexedStack, RepaintBoundary};

    /// What a [`Probe`] saw the last time it built.
    #[derive(Debug, Default)]
    struct Report {
        builds: Cell<u32>,
        live: Cell<i32>,
        visible: Cell<bool>,
        ticking: Cell<bool>,
        focusable: Cell<bool>,
    }

    /// A stateful leaf that reports what [`Visibility::of`], [`TickerMode`] and the enclosing
    /// [`ExcludeFocus`] say where it builds, and lays out at 10 by 10.
    #[derive(Debug)]
    struct Probe {
        report: Rc<Report>,
    }

    impl Probe {
        fn new(report: &Rc<Report>) -> Probe {
            Probe {
                report: Rc::clone(report),
            }
        }
    }

    impl StatefulWidget for Probe {
        type State = ProbeState;

        fn create_state(&self) -> ProbeState {
            ProbeState {
                state: StateData::new(),
            }
        }
    }

    struct ProbeState {
        state: StateData<Probe>,
    }

    impl ProbeState {
        fn report(self: Handle<Self>, app: &App) -> Rc<Report> {
            Rc::clone(&self.widget(app).report)
        }
    }

    impl State for ProbeState {
        type Widget = Probe;
        crate::state_accessors!();

        fn init_state(self: Handle<Self>, app: &mut App) {
            let report = self.report(app);
            report.live.set(report.live.get() + 1);
        }

        fn dispose(self: Handle<Self>, app: &mut App) {
            let report = self.report(app);
            report.live.set(report.live.get() - 1);
        }

        fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
            let report = self.report(app);
            report.builds.set(report.builds.get() + 1);
            report.visible.set(Visibility::of(app, context));
            report
                .ticking
                .set(TickerMode::values_of(app, context).enabled);
            report.focusable.set(
                context
                    .find_ancestor_widget_of_exact_type::<ExcludeFocus>(app)
                    .is_none_or(|exclude_focus| !exclude_focus.excluding),
            );
            SizedBox::new().width(10.0).height(10.0).into_widget()
        }
    }

    /// The render object under the test root, typed.
    fn root_child<T: RenderObject>(harness: &Harness, app: &App) -> RenderHandle<T> {
        harness
            .render_root(app)
            .child(app)
            .expect("a child")
            .as_object()
            .downcast::<T>(app)
            .expect("the widget's render object")
    }

    #[test]
    fn a_hidden_visibility_replaces_the_child_and_discards_its_state() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let report = Rc::new(Report::default());
        let harness = Harness::mount(&mut app, Visibility::new(Probe::new(&report)).into_widget());
        harness.pump(&mut app);
        assert_eq!(report.live.get(), 1);
        assert_eq!(
            root_child::<RenderConstrainedBox>(&harness, &app).size(&app),
            Size::new(10.0, 10.0),
            "a visible child is the subtree, without the ExcludeFocus"
        );

        harness.set_child(
            &mut app,
            Visibility::new(Probe::new(&report))
                .visible(false)
                .into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(report.live.get(), 0, "the child subtree was disposed");
        assert_eq!(
            root_child::<RenderConstrainedBox>(&harness, &app).size(&app),
            Size::ZERO,
            "the replacement is a zero-sized box"
        );
    }

    #[test]
    fn a_hidden_visibility_takes_the_replacement_it_is_given() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            Visibility::new(SizedBox::new().width(10.0).height(10.0))
                .replacement(SizedBox::new().width(4.0).height(4.0))
                .visible(false)
                .into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(
            root_child::<RenderConstrainedBox>(&harness, &app).size(&app),
            Size::new(4.0, 4.0)
        );
    }

    #[test]
    fn maintain_state_hides_the_child_offstage_and_mutes_its_tickers() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let report = Rc::new(Report::default());
        let visibility = |visible: bool, report: &Rc<Report>| {
            Visibility::new(Probe::new(report))
                .visible(visible)
                .maintain_state(true)
                .into_widget()
        };
        let harness = Harness::mount(&mut app, visibility(true, &report));
        harness.pump(&mut app);
        let offstage = root_child::<RenderOffstage>(&harness, &app);
        assert!(!offstage.offstage(&app));
        assert!(report.ticking.get());
        assert!(report.focusable.get());

        harness.set_child(&mut app, visibility(false, &report));
        harness.pump(&mut app);
        assert_eq!(report.live.get(), 1, "the state survives the hiding");
        assert!(offstage.offstage(&app), "an Offstage hides the child");
        assert_eq!(offstage.size(&app), Size::ZERO, "it takes up no room");
        assert!(!report.ticking.get(), "a TickerMode mutes the child");
        assert!(
            !report.focusable.get(),
            "an ExcludeFocus excludes the child"
        );
    }

    #[test]
    fn maintain_animation_leaves_out_the_ticker_mode() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let report = Rc::new(Report::default());
        let harness = Harness::mount(
            &mut app,
            Visibility::new(Probe::new(&report))
                .visible(false)
                .maintain_state(true)
                .maintain_animation(true)
                .into_widget(),
        );
        harness.pump(&mut app);
        assert!(root_child::<RenderOffstage>(&harness, &app).offstage(&app));
        assert!(report.ticking.get(), "no TickerMode is in the tree");
    }

    #[test]
    fn maintain_focusability_keeps_the_hidden_child_focusable() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let report = Rc::new(Report::default());
        let harness = Harness::mount(
            &mut app,
            Visibility::new(Probe::new(&report))
                .visible(false)
                .maintain_state(true)
                .maintain_focusability(true)
                .into_widget(),
        );
        harness.pump(&mut app);
        assert!(report.focusable.get());
    }

    #[test]
    fn maintain_size_keeps_the_layout_and_stops_the_paint() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let report = Rc::new(Report::default());
        let harness = Harness::mount(
            &mut app,
            Visibility::maintain(Probe::new(&report))
                .visible(false)
                .into_widget(),
        );
        harness.pump(&mut app);

        let visibility = root_child::<RenderVisibility>(&harness, &app);
        assert!(!visibility.visible(&app), "the child is not painted");
        assert_eq!(
            visibility.size(&app),
            Size::new(10.0, 10.0),
            "the space the child would have taken is kept"
        );
        assert_eq!(report.live.get(), 1);
        assert!(report.ticking.get(), "no TickerMode is in the tree");
        assert!(
            report.focusable.get(),
            "Visibility.maintain keeps the focus"
        );
        let ignore_pointer = visibility
            .child(&app)
            .expect("an IgnorePointer")
            .as_object()
            .downcast::<RenderIgnorePointer>(&app)
            .expect("an IgnorePointer");
        assert!(
            !ignore_pointer.ignoring(&app),
            "Visibility.maintain keeps the interactivity"
        );

        harness.set_child(
            &mut app,
            Visibility::maintain(Probe::new(&report)).into_widget(),
        );
        harness.pump(&mut app);
        assert!(visibility.visible(&app));
    }

    /// A repaint boundary's layer is detached while nothing paints it (`layers_test.dart`:
    /// `non-painted layers are detached`).
    #[test]
    fn a_hidden_child_whose_size_is_maintained_is_not_painted() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let tree = |visible: bool| {
            Visibility::maintain(
                RepaintBoundary::new().child(SizedBox::new().width(10.0).height(10.0)),
            )
            .visible(visible)
            .into_widget()
        };
        let harness = Harness::mount(&mut app, tree(true));
        harness.pump(&mut app);
        let boundary = root_child::<RenderVisibility>(&harness, &app)
            .child(&app)
            .expect("an IgnorePointer")
            .as_object()
            .downcast::<RenderIgnorePointer>(&app)
            .expect("an IgnorePointer")
            .child(&app)
            .expect("a RepaintBoundary")
            .as_object();
        assert!(
            boundary
                .debug_layer(&app)
                .expect("painted")
                .as_layer()
                .attached(&app),
            "a visible child paints"
        );

        harness.set_child(&mut app, tree(false));
        harness.pump(&mut app);
        assert!(
            !boundary
                .debug_layer(&app)
                .expect("a layer")
                .as_layer()
                .attached(&app),
            "the hidden child is not painted"
        );
    }

    #[test]
    fn maintain_size_alone_hides_the_child_from_pointers() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            Visibility::new(SizedBox::new().width(10.0).height(10.0))
                .visible(false)
                .maintain_state(true)
                .maintain_animation(true)
                .maintain_size(true)
                .into_widget(),
        );
        harness.pump(&mut app);

        let visibility = root_child::<RenderVisibility>(&harness, &app);
        assert!(!visibility.visible(&app));
        assert_eq!(visibility.size(&app), Size::new(10.0, 10.0));
        let ignore_pointer = visibility
            .child(&app)
            .expect("an IgnorePointer")
            .as_object()
            .downcast::<RenderIgnorePointer>(&app)
            .expect("an IgnorePointer");
        assert!(ignore_pointer.ignoring(&app));
    }

    #[test]
    fn maintain_interactivity_lets_pointers_through_a_hidden_child() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(
            &mut app,
            Visibility::new(SizedBox::new().width(10.0).height(10.0))
                .visible(false)
                .maintain_state(true)
                .maintain_animation(true)
                .maintain_size(true)
                .maintain_interactivity(true)
                .into_widget(),
        );
        harness.pump(&mut app);
        let ignore_pointer = root_child::<RenderVisibility>(&harness, &app)
            .child(&app)
            .expect("an IgnorePointer")
            .as_object()
            .downcast::<RenderIgnorePointer>(&app)
            .expect("an IgnorePointer");
        assert!(!ignore_pointer.ignoring(&app));
    }

    #[test]
    fn visibility_of_is_true_without_an_ancestor_visibility() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let report = Rc::new(Report::default());
        let harness = Harness::mount(&mut app, Probe::new(&report).into_widget());
        harness.pump(&mut app);
        assert!(report.visible.get());
    }

    #[test]
    fn visibility_of_reports_every_ancestor_scope() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let report = Rc::new(Report::default());
        let nested = |outer: bool, inner: bool, report: &Rc<Report>| {
            Visibility::new(
                Visibility::new(Probe::new(report))
                    .visible(inner)
                    .maintain_state(true),
            )
            .visible(outer)
            .maintain_state(true)
            .into_widget()
        };
        let harness = Harness::mount(&mut app, nested(true, true, &report));
        harness.pump(&mut app);
        assert!(report.visible.get());

        harness.set_child(&mut app, nested(true, false, &report));
        harness.pump(&mut app);
        assert!(!report.visible.get(), "the nearest scope hides it");

        harness.set_child(&mut app, nested(false, true, &report));
        harness.pump(&mut app);
        assert!(!report.visible.get(), "an outer scope hides it too");
    }

    #[test]
    fn visibility_of_rebuilds_its_dependents() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let report = Rc::new(Report::default());
        let visibility = |visible: bool, report: &Rc<Report>| {
            Visibility::new(Probe::new(report))
                .visible(visible)
                .maintain_state(true)
                .maintain_animation(true)
                .into_widget()
        };
        let harness = Harness::mount(&mut app, visibility(true, &report));
        harness.pump(&mut app);
        let builds = report.builds.get();

        harness.set_child(&mut app, visibility(false, &report));
        harness.pump(&mut app);
        assert!(report.builds.get() > builds, "the scope notified it");
        assert!(!report.visible.get());
    }

    #[test]
    fn visibility_of_reports_an_indexed_stack_scope() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let report = Rc::new(Report::default());
        let harness = Harness::mount(
            &mut app,
            Directionality::new(
                TextDirection::Ltr,
                IndexedStack::new().index(Some(1)).children([
                    Probe::new(&report).into_widget(),
                    SizedBox::new().width(10.0).height(10.0).into_widget(),
                ]),
            )
            .into_widget(),
        );
        harness.pump(&mut app);
        assert!(
            !report.visible.get(),
            "the scope IndexedStack builds is the one Visibility.of reads"
        );
    }
}
