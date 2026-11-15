//! Flutter counterpart: `widgets/animated_cross_fade.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    AnimationBehavior, AnimationController, AnimationStatusListener, AnyAnimation, Curve,
    CurveTween, Curves, Tween,
};
use waft_embedder::Clip;
use waft_foundation::{App, Handle, Listener, ValueKey};
use waft_painting::AlignmentGeometry;
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::animated_size::AnimatedSize;
use crate::widgets::basic::{ClipRect, ExcludeSemantics, IgnorePointer, Positioned, Stack};
use crate::widgets::focus_scope::ExcludeFocus;
use crate::widgets::ticker_provider::{
    TickerMode, TickerProviderStateMixin, TickerProviderStateMixinData,
};
use crate::widgets::transitions::FadeTransition;

/// Specifies which of two children to show. See [`AnimatedCrossFade`].
///
/// The child that is shown will fade in, while the other will fade out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CrossFadeState {
    /// Show the first child ([`AnimatedCrossFade::first_child`]) and hide the second
    /// ([`AnimatedCrossFade::second_child`]).
    ShowFirst,

    /// Show the second child ([`AnimatedCrossFade::second_child`]) and hide the first
    /// ([`AnimatedCrossFade::first_child`]).
    ShowSecond,
}

/// Signature for the [`AnimatedCrossFade::layout_builder`] callback.
///
/// The `top_child` is the child fading in, which is normally drawn on top. The
/// `bottom_child` is the child fading out, normally drawn on the bottom.
///
/// For good performance, the returned widget tree should contain both the
/// `top_child` and the `bottom_child`; the depth of the tree, and the types of
/// the widgets in the tree, from the returned widget to each of the children
/// should be the same; and where there is a widget with multiple children, the
/// top child and the bottom child should be keyed using the provided
/// `top_child_key` and `bottom_child_key` keys respectively.
///
/// See [`AnimatedCrossFade::default_layout_builder`] for the default.
pub type AnimatedCrossFadeBuilder =
    Rc<dyn Fn(&mut App, &WidgetRef, &KeyRef, &WidgetRef, &KeyRef) -> WidgetRef>;

/// A widget that cross-fades between two given children and animates itself
/// between their sizes.
///
/// The animation is controlled through the [`cross_fade_state`](Self::cross_fade_state)
/// parameter. [`first_curve`](Self::first_curve) and [`second_curve`](Self::second_curve)
/// represent the opacity curves of the two children. The
/// [`first_curve`](Self::first_curve) is inverted, i.e. it fades out when providing a
/// growing curve like `Curves::linear`. The [`size_curve`](Self::size_curve) is the curve
/// used to animate between the size of the fading-out child and the size of the fading-in
/// child.
///
/// This widget is intended to be used to fade a pair of widgets with the same
/// width. In the case where the two children have different heights, the
/// animation crops overflowing children during the animation by aligning their
/// top edge, which means that the bottom will be clipped.
///
/// The animation is automatically triggered when an existing
/// [`AnimatedCrossFade`] is rebuilt with a different value for the
/// [`cross_fade_state`](Self::cross_fade_state) property.
///
/// See also:
///
///  * `AnimatedOpacity`, which fades between nothing and a single child.
///  * `AnimatedSwitcher`, which switches out a child for a new one with a
///    customizable transition, supporting multiple cross-fades at once.
///  * [`AnimatedSize`], the lower-level widget which [`AnimatedCrossFade`] uses to
///    automatically change size.
pub struct AnimatedCrossFade {
    pub key: Option<KeyRef>,

    /// The child that is visible when [`cross_fade_state`](Self::cross_fade_state) is
    /// [`CrossFadeState::ShowFirst`]. It fades out when transitioning
    /// [`cross_fade_state`](Self::cross_fade_state) from [`CrossFadeState::ShowFirst`] to
    /// [`CrossFadeState::ShowSecond`] and vice versa.
    pub first_child: WidgetRef,

    /// The child that is visible when [`cross_fade_state`](Self::cross_fade_state) is
    /// [`CrossFadeState::ShowSecond`]. It fades in when transitioning
    /// [`cross_fade_state`](Self::cross_fade_state) from [`CrossFadeState::ShowFirst`] to
    /// [`CrossFadeState::ShowSecond`] and vice versa.
    pub second_child: WidgetRef,

    /// The child that will be shown when the animation has completed.
    pub cross_fade_state: CrossFadeState,

    /// The duration of the whole orchestrated animation.
    pub duration: Duration,

    /// The duration of the whole orchestrated animation when running in reverse.
    ///
    /// If not supplied, this defaults to [`duration`](Self::duration).
    pub reverse_duration: Option<Duration>,

    /// The fade curve of the first child.
    ///
    /// Defaults to `Curves::linear`.
    pub first_curve: Rc<dyn Curve>,

    /// The fade curve of the second child.
    ///
    /// Defaults to `Curves::linear`.
    pub second_curve: Rc<dyn Curve>,

    /// The curve of the animation between the two children's sizes.
    ///
    /// Defaults to `Curves::linear`.
    pub size_curve: Rc<dyn Curve>,

    /// How the children should be aligned while the size is animating.
    ///
    /// Defaults to `Alignment::TOP_CENTER`.
    ///
    /// See also:
    ///
    ///  * `Alignment`, a class with convenient constants typically used to
    ///    specify an [`AlignmentGeometry`].
    ///  * `AlignmentDirectional`, like `Alignment` for specifying alignments
    ///    relative to text direction.
    pub alignment: AlignmentGeometry,

    /// A builder that positions the [`first_child`](Self::first_child) and
    /// [`second_child`](Self::second_child) widgets.
    ///
    /// The widget returned by this method is wrapped in an [`AnimatedSize`].
    ///
    /// By default, this uses [`AnimatedCrossFade::default_layout_builder`], which uses
    /// a [`Stack`] and aligns the `bottom_child` to the top of the stack while
    /// providing the `top_child` as the non-positioned child to fill the provided
    /// constraints. This works well when the [`AnimatedCrossFade`] is in a position
    /// to change size and when the children are not flexible. However, if the
    /// children are less fussy about their sizes (for example a
    /// `CircularProgressIndicator` inside a `Center`), or if the
    /// [`AnimatedCrossFade`] is being forced to a particular size, then it can
    /// result in the widgets jumping about when the cross-fade state is changed.
    pub layout_builder: AnimatedCrossFadeBuilder,

    /// When true, this is equivalent to wrapping the bottom widget with an [`ExcludeFocus`]
    /// widget while it is at the bottom of the cross-fade stack.
    ///
    /// Defaults to true. When it is false, the bottom widget in the cross-fade stack
    /// can remain in focus until the top widget requests focus. This is useful for
    /// animating between different `TextField`s so the keyboard remains open during the
    /// cross-fade animation.
    pub exclude_bottom_focus: bool,

    /// Controls whether the content is clipped during the cross-fade transition.
    ///
    /// During the size transition between [`first_child`](Self::first_child) and
    /// [`second_child`](Self::second_child), the content is clipped by default to prevent it
    /// from overflowing the bounding box of the animating widget.
    ///
    /// Set this to [`Clip::None`] to disable clipping entirely. This can be
    /// useful for widgets with shadows or visual effects that extend beyond
    /// their bounds.
    ///
    /// Defaults to [`Clip::HardEdge`].
    pub clip_behavior: Clip,

    /// Called every time an animation completes.
    ///
    /// This can be useful to trigger additional actions (e.g. another animation)
    /// at the end of the current animation.
    pub on_end: Option<Listener>,
}

impl Debug for AnimatedCrossFade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnimatedCrossFade")
            .field("key", &self.key)
            .field("cross_fade_state", &self.cross_fade_state)
            .field("alignment", &self.alignment)
            .field("duration", &self.duration)
            .field("reverse_duration", &self.reverse_duration)
            .finish_non_exhaustive()
    }
}

impl AnimatedCrossFade {
    /// Creates a cross-fade animation widget.
    ///
    /// The [`duration`](Self::duration) of the animation is the same for all components (fade
    /// in, fade out, and size), and you can pass `Interval`s instead of `Curve`s in order to
    /// have finer control, e.g., creating an overlap between the fades.
    pub fn new<F, S>(
        first_child: impl IntoWidget<F>,
        second_child: impl IntoWidget<S>,
        cross_fade_state: CrossFadeState,
        duration: Duration,
    ) -> AnimatedCrossFade {
        AnimatedCrossFade {
            key: None,
            first_child: first_child.into_widget(),
            second_child: second_child.into_widget(),
            cross_fade_state,
            duration,
            reverse_duration: None,
            first_curve: Curves::linear(),
            second_curve: Curves::linear(),
            size_curve: Curves::linear(),
            alignment: AlignmentGeometry::TOP_CENTER,
            layout_builder: Rc::new(AnimatedCrossFade::default_layout_builder),
            exclude_bottom_focus: true,
            clip_behavior: Clip::HardEdge,
            on_end: None,
        }
    }

    /// Dart `AnimatedCrossFade(key:)`.
    pub fn key(mut self, key: KeyRef) -> AnimatedCrossFade {
        self.key = Some(key);
        self
    }

    /// Dart `AnimatedCrossFade(reverseDuration:)`.
    pub fn reverse_duration(mut self, reverse_duration: Duration) -> AnimatedCrossFade {
        self.reverse_duration = Some(reverse_duration);
        self
    }

    /// Dart `AnimatedCrossFade(firstCurve:)`.
    pub fn first_curve(mut self, first_curve: Rc<dyn Curve>) -> AnimatedCrossFade {
        self.first_curve = first_curve;
        self
    }

    /// Dart `AnimatedCrossFade(secondCurve:)`.
    pub fn second_curve(mut self, second_curve: Rc<dyn Curve>) -> AnimatedCrossFade {
        self.second_curve = second_curve;
        self
    }

    /// Dart `AnimatedCrossFade(sizeCurve:)`.
    pub fn size_curve(mut self, size_curve: Rc<dyn Curve>) -> AnimatedCrossFade {
        self.size_curve = size_curve;
        self
    }

    /// Dart `AnimatedCrossFade(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> AnimatedCrossFade {
        self.alignment = alignment;
        self
    }

    /// Dart `AnimatedCrossFade(layoutBuilder:)`.
    pub fn layout_builder(
        mut self,
        layout_builder: impl Fn(&mut App, &WidgetRef, &KeyRef, &WidgetRef, &KeyRef) -> WidgetRef
        + 'static,
    ) -> AnimatedCrossFade {
        self.layout_builder = Rc::new(layout_builder);
        self
    }

    /// Dart `AnimatedCrossFade(excludeBottomFocus:)`.
    pub fn exclude_bottom_focus(mut self, exclude_bottom_focus: bool) -> AnimatedCrossFade {
        self.exclude_bottom_focus = exclude_bottom_focus;
        self
    }

    /// Dart `AnimatedCrossFade(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> AnimatedCrossFade {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `AnimatedCrossFade(onEnd:)`.
    pub fn on_end(mut self, on_end: Listener) -> AnimatedCrossFade {
        self.on_end = Some(on_end);
        self
    }

    /// The default layout algorithm used by [`AnimatedCrossFade`].
    ///
    /// The top child is placed in a stack that sizes itself to match the top
    /// child. The bottom child is positioned at the top of the same stack, sized
    /// to fit its width but without forcing the height. The stack is then
    /// clipped.
    ///
    /// This is the default value for [`layout_builder`](Self::layout_builder). It implements
    /// [`AnimatedCrossFadeBuilder`].
    pub fn default_layout_builder(
        _app: &mut App,
        top_child: &WidgetRef,
        top_child_key: &KeyRef,
        bottom_child: &WidgetRef,
        bottom_child_key: &KeyRef,
    ) -> WidgetRef {
        Stack::new()
            .clip_behavior(Clip::None)
            .children([
                Positioned::new(bottom_child.clone())
                    .key(bottom_child_key.clone())
                    .left(0.0)
                    .top(0.0)
                    .right(0.0)
                    .into_widget(),
                Positioned::new(top_child.clone())
                    .key(top_child_key.clone())
                    .into_widget(),
            ])
            .into_widget()
    }
}

impl StatefulWidget for AnimatedCrossFade {
    type State = AnimatedCrossFadeState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> AnimatedCrossFadeState {
        AnimatedCrossFadeState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            controller: None,
            first_animation: None,
            second_animation: None,
        }
    }
}

/// Dart's `_AnimatedCrossFadeState`; its `late` fields are set by `init_state`.
pub struct AnimatedCrossFadeState {
    state: StateData<AnimatedCrossFade>,
    ticker_provider: TickerProviderStateMixinData,
    controller: Option<Handle<AnimationController>>,
    first_animation: Option<AnyAnimation<f64>>,
    second_animation: Option<AnyAnimation<f64>>,
}

impl Debug for AnimatedCrossFadeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnimatedCrossFadeState")
            .finish_non_exhaustive()
    }
}

impl TickerProviderStateMixin for AnimatedCrossFadeState {
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

impl TickerProviderObject for AnimatedCrossFadeState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl AnimatedCrossFadeState {
    fn controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self).controller.expect("created in init_state")
    }

    fn init_animation(
        self: Handle<Self>,
        app: &mut App,
        curve: Rc<dyn Curve>,
        inverted: bool,
    ) -> AnyAnimation<f64> {
        let curve_tween = CurveTween::new(app, curve);
        let mut result = self.controller(app).drive(app, curve_tween);
        if inverted {
            let tween = Tween::new(app, Some(1.0), Some(0.0));
            result = result.drive(app, tween);
        }
        result
    }
}

impl State for AnimatedCrossFadeState {
    type Widget = AnimatedCrossFade;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let duration = widget.duration;
        let reverse_duration = widget.reverse_duration;
        let cross_fade_state = widget.cross_fade_state;
        let first_curve = Rc::clone(&widget.first_curve);
        let second_curve = Rc::clone(&widget.second_curve);
        let controller = AnimationController::create(
            app,
            None,
            Some(duration),
            reverse_duration,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        app.get_mut(self).controller = Some(controller);
        if cross_fade_state == CrossFadeState::ShowSecond {
            controller.set_value(app, 1.0);
        }
        let first_animation = self.init_animation(app, first_curve, true);
        let second_animation = self.init_animation(app, second_curve, false);
        let state = app.get_mut(self);
        state.first_animation = Some(first_animation);
        state.second_animation = Some(second_animation);
        controller.view().add_status_listener(
            app,
            AnimationStatusListener::new(move |status, app| {
                self.set_state(app, |_state| {
                    // Trigger a rebuild because it depends on _isTransitioning, which
                    // changes its value together with animation status.
                });
                if (status.is_completed() || status.is_dismissed())
                    && let Some(on_end) = self.widget(app).on_end.clone()
                {
                    on_end.call(app);
                }
            }),
        );
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.controller(app).dispose(app);
        TickerProviderStateMixin::dispose(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &AnimatedCrossFade) {
        let widget = self.widget(app);
        let duration = widget.duration;
        let reverse_duration = widget.reverse_duration;
        let first_curve = Rc::clone(&widget.first_curve);
        let second_curve = Rc::clone(&widget.second_curve);
        let cross_fade_state = widget.cross_fade_state;
        let controller = self.controller(app);
        if duration != old_widget.duration {
            app.get_mut(controller).duration = Some(duration);
        }
        if reverse_duration != old_widget.reverse_duration {
            app.get_mut(controller).reverse_duration = reverse_duration;
        }
        if !Rc::ptr_eq(&first_curve, &old_widget.first_curve) {
            let first_animation = self.init_animation(app, first_curve, true);
            app.get_mut(self).first_animation = Some(first_animation);
        }
        if !Rc::ptr_eq(&second_curve, &old_widget.second_curve) {
            let second_animation = self.init_animation(app, second_curve, false);
            app.get_mut(self).second_animation = Some(second_animation);
        }
        if cross_fade_state != old_widget.cross_fade_state {
            match cross_fade_state {
                CrossFadeState::ShowFirst => {
                    controller.reverse(app, None);
                }
                CrossFadeState::ShowSecond => {
                    controller.forward(app, None);
                }
            }
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let first_child_key: KeyRef = ValueKey::new(CrossFadeState::ShowFirst).into();
        let second_child_key: KeyRef = ValueKey::new(CrossFadeState::ShowSecond).into();
        let widget = self.widget(app);
        let first_child = widget.first_child.clone();
        let second_child = widget.second_child.clone();
        let exclude_bottom_focus = widget.exclude_bottom_focus;
        let clip_behavior = widget.clip_behavior;
        let alignment = widget.alignment;
        let duration = widget.duration;
        let reverse_duration = widget.reverse_duration;
        let size_curve = Rc::clone(&widget.size_curve);
        let layout_builder = Rc::clone(&widget.layout_builder);
        let first_animation = app
            .get(self)
            .first_animation
            .expect("created in init_state");
        let second_animation = app
            .get(self)
            .second_animation
            .expect("created in init_state");
        let controller = self.controller(app);
        let top_key;
        let top_child;
        let top_animation;
        let bottom_key;
        let bottom_child;
        let bottom_animation;
        if controller.status(app).is_forward_or_completed() {
            top_key = second_child_key;
            top_child = second_child;
            top_animation = second_animation;
            bottom_key = first_child_key;
            bottom_child = first_child;
            bottom_animation = first_animation;
        } else {
            top_key = first_child_key;
            top_child = first_child;
            top_animation = first_animation;
            bottom_key = second_child_key;
            bottom_child = second_child;
            bottom_animation = second_animation;
        }

        let bottom_child = TickerMode::new(
            controller.is_animating(app),
            IgnorePointer::new().child(
                // Always exclude the semantics of the widget that's fading out.
                ExcludeSemantics::new().child(
                    ExcludeFocus::new(FadeTransition::new(bottom_animation).child(bottom_child))
                        .excluding(exclude_bottom_focus),
                ),
            ),
        )
        .key(bottom_key.clone())
        .into_widget();
        let top_child = TickerMode::new(
            true, // Top widget always has its animations enabled.
            IgnorePointer::new().ignoring(false).child(
                ExcludeSemantics::new()
                    // Always publish semantics for the widget that's fading in.
                    .excluding(false)
                    .child(
                        ExcludeFocus::new(FadeTransition::new(top_animation).child(top_child))
                            .excluding(false),
                    ),
            ),
        )
        .key(top_key.clone())
        .into_widget();
        let layout = layout_builder(app, &top_child, &top_key, &bottom_child, &bottom_key);
        let mut animated_size = AnimatedSize::new(duration)
            .alignment(alignment)
            .curve(size_curve)
            .clip_behavior(clip_behavior)
            .child(layout);
        if let Some(reverse_duration) = reverse_duration {
            animated_size = animated_size.reverse_duration(reverse_duration);
        }
        ClipRect::new()
            .clip_behavior(clip_behavior)
            .child(animated_size)
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use waft_embedder::{Offset, Size, TextDirection};
    use waft_foundation::UniqueKey;
    use waft_scheduler::FrameCallback;

    use super::*;
    use crate::binding::WidgetsBinding;
    use crate::framework::AnyElement;
    use crate::test_harness::{
        VIEW_HEIGHT, VIEW_WIDTH, binding_cell, binding_mount, binding_pump, platform_view,
    };
    use crate::view::View;
    use crate::widgets::actions::FocusableActionDetector;
    use crate::widgets::basic::{Builder, Center, Directionality, Semantics, SizedBox};
    use crate::widgets::container::Container;
    use crate::widgets::focus_scope::Focus;
    use crate::widgets::gesture_detector::GestureDetector;
    use crate::widgets::text::Text;
    use crate::widgets::ticker_provider::{
        SingleTickerProviderStateMixin, SingleTickerProviderStateMixinData,
    };

    fn at(milliseconds: u64) -> Duration {
        Duration::from_millis(milliseconds)
    }

    /// Dart's `tester.pumpWidget(widget)` after the first one: the frame at `time` that builds
    /// it.
    fn pump_widget(app: &mut App, widget: WidgetRef, time: Duration) {
        let view = platform_view(app);
        let root = View::new(view, widget).into_widget();
        WidgetsBinding::instance(app).attach_root_widget(app, root);
        binding_pump(app, time);
    }

    fn elements(app: &mut App) -> Vec<AnyElement> {
        fn visit(app: &App, element: AnyElement, found: &mut Vec<AnyElement>) {
            found.push(element);
            for child in element.children(app) {
                visit(app, child, found);
            }
        }
        let mut found = Vec::new();
        let root = WidgetsBinding::instance(app)
            .root_element(app)
            .expect("a mounted tree");
        visit(app, root, &mut found);
        found
    }

    /// `find.byType(T)`: the tree's widgets that are a `T`, in tree order.
    fn find_by_type<T: 'static>(app: &mut App) -> Vec<WidgetRef> {
        elements(app)
            .into_iter()
            .map(|element| element.widget(app).clone())
            .filter(|widget| widget.as_any().is::<T>())
            .collect()
    }

    /// The render box of the first element whose widget `matches`.
    fn render_box_where(
        app: &mut App,
        matches: impl Fn(&WidgetRef) -> bool,
    ) -> waft_rendering::AnyRenderBox {
        elements(app)
            .into_iter()
            .find(|element| matches(element.widget(app)))
            .and_then(|element| element.find_render_object(app))
            .and_then(|render_object| render_object.as_box())
            .expect("a mounted render box")
    }

    fn cross_fade_size(app: &mut App) -> Size {
        let render_box = render_box_where(app, |widget| widget.as_any().is::<AnimatedCrossFade>());
        render_box.size(app)
    }

    fn global_offset_of(app: &mut App, key: &KeyRef) -> Offset {
        let render_box = render_box_where(app, |widget| widget.key() == Some(key));
        render_box.local_to_global(app, Offset::ZERO, None)
    }

    fn sized(width: f64, height: f64) -> WidgetRef {
        SizedBox::new().width(width).height(height).into_widget()
    }

    fn keyed(key: &KeyRef, width: f64, height: f64) -> WidgetRef {
        SizedBox::new()
            .key(key.clone())
            .width(width)
            .height(height)
            .into_widget()
    }

    fn centered(text_direction: TextDirection, cross_fade: AnimatedCrossFade) -> WidgetRef {
        Directionality::new(text_direction, Center::new().child(cross_fade)).into_widget()
    }

    fn simple_cross_fade(state: CrossFadeState) -> AnimatedCrossFade {
        AnimatedCrossFade::new(sized(100.0, 100.0), sized(200.0, 200.0), state, at(200))
    }

    #[test]
    fn animates_between_the_sizes_of_its_children() {
        let cell = binding_cell();
        binding_mount(
            &cell,
            centered(
                TextDirection::Ltr,
                simple_cross_fade(CrossFadeState::ShowFirst),
            ),
        );
        let mut app = cell.borrow_mut();
        assert_eq!(find_by_type::<FadeTransition>(&mut app).len(), 2);
        assert_eq!(cross_fade_size(&mut app), Size::new(100.0, 100.0));

        pump_widget(
            &mut app,
            centered(
                TextDirection::Ltr,
                simple_cross_fade(CrossFadeState::ShowSecond),
            ),
            at(0),
        );
        binding_pump(&mut app, at(100));

        assert_eq!(find_by_type::<FadeTransition>(&mut app).len(), 2);
        assert_eq!(cross_fade_size(&mut app), Size::new(150.0, 150.0));
    }

    #[test]
    fn show_second_starts_at_the_size_of_the_second_child() {
        let cell = binding_cell();
        binding_mount(
            &cell,
            centered(
                TextDirection::Ltr,
                simple_cross_fade(CrossFadeState::ShowSecond),
            ),
        );
        let mut app = cell.borrow_mut();
        assert_eq!(find_by_type::<FadeTransition>(&mut app).len(), 2);
        assert_eq!(cross_fade_size(&mut app), Size::new(200.0, 200.0));
    }

    /// The first and second children keep their places in the stack while the size animates:
    /// `alignment` and the text direction decide where both sit.
    fn assert_aligned_halfway(
        text_direction: TextDirection,
        alignment: AlignmentGeometry,
        expected: Offset,
    ) {
        let cell = binding_cell();
        let (first_key, second_key): (KeyRef, KeyRef) =
            (UniqueKey::new().into(), UniqueKey::new().into());
        let tree = |state: CrossFadeState| {
            centered(
                text_direction,
                AnimatedCrossFade::new(
                    keyed(&first_key, 100.0, 100.0),
                    keyed(&second_key, 200.0, 200.0),
                    state,
                    at(200),
                )
                .alignment(alignment),
            )
        };
        binding_mount(&cell, tree(CrossFadeState::ShowFirst));
        let mut app = cell.borrow_mut();
        pump_widget(&mut app, tree(CrossFadeState::ShowSecond), at(0));
        binding_pump(&mut app, at(100));

        assert_eq!(global_offset_of(&mut app, &first_key), expected);
        assert_eq!(global_offset_of(&mut app, &second_key), expected);
    }

    /// Dart's expected offsets are measured from the centre of its 800 x 600 view, and the
    /// harness's view is [`VIEW_WIDTH`] x [`VIEW_HEIGHT`].
    fn from_centre(dx: f64) -> Offset {
        Offset::new(VIEW_WIDTH / 2.0 + dx, VIEW_HEIGHT / 2.0 - 125.0)
    }

    #[test]
    fn alignment_visual() {
        assert_aligned_halfway(
            TextDirection::Ltr,
            AlignmentGeometry::BOTTOM_RIGHT,
            from_centre(-125.0),
        );
    }

    #[test]
    fn alignment_ltr() {
        assert_aligned_halfway(
            TextDirection::Ltr,
            AlignmentGeometry::BOTTOM_END,
            from_centre(-125.0),
        );
    }

    #[test]
    fn alignment_rtl() {
        assert_aligned_halfway(
            TextDirection::Rtl,
            AlignmentGeometry::BOTTOM_END,
            from_centre(-75.0),
        );
    }

    /// Dart's `_TickerWatchingWidget`: a state with a started ticker of its own.
    #[derive(Debug)]
    struct TickerWatching {
        ticker: Rc<Cell<Option<Handle<Ticker>>>>,
        inits: Rc<Cell<u32>>,
    }

    impl StatefulWidget for TickerWatching {
        type State = TickerWatchingState;

        fn create_state(&self) -> TickerWatchingState {
            TickerWatchingState {
                state: StateData::new(),
                single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            }
        }
    }

    struct TickerWatchingState {
        state: StateData<TickerWatching>,
        single_ticker_provider: SingleTickerProviderStateMixinData,
    }

    impl SingleTickerProviderStateMixin for TickerWatchingState {
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

    impl TickerProviderObject for TickerWatchingState {
        fn create_ticker(
            self: Handle<Self>,
            app: &mut App,
            on_tick: TickerCallback,
        ) -> Handle<Ticker> {
            SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
        }
    }

    impl State for TickerWatchingState {
        type Widget = TickerWatching;
        crate::state_accessors!();

        fn init_state(self: Handle<Self>, app: &mut App) {
            let ticker = SingleTickerProviderStateMixin::create_ticker(
                self,
                app,
                FrameCallback::new(|_app, _elapsed| {}),
            );
            ticker.start(app);
            let widget = self.widget(app);
            widget.ticker.set(Some(ticker));
            widget.inits.set(widget.inits.get() + 1);
        }

        fn activate(self: Handle<Self>, app: &mut App) {
            SingleTickerProviderStateMixin::activate(self, app);
        }

        fn dispose(self: Handle<Self>, app: &mut App) {
            let ticker = self
                .widget(app)
                .ticker
                .get()
                .expect("started in init_state");
            ticker.dispose(app);
            SingleTickerProviderStateMixin::dispose(self, app);
        }

        fn build(self: Handle<Self>, _app: &mut App, _context: BuildContext) -> WidgetRef {
            Container::new().into_widget()
        }
    }

    struct Watcher {
        ticker: Rc<Cell<Option<Handle<Ticker>>>>,
        inits: Rc<Cell<u32>>,
    }

    impl Watcher {
        fn new() -> Watcher {
            Watcher {
                ticker: Rc::default(),
                inits: Rc::default(),
            }
        }

        fn cross_fade(&self, towards_second: bool) -> WidgetRef {
            let first_child = TickerWatching {
                ticker: Rc::clone(&self.ticker),
                inits: Rc::clone(&self.inits),
            };
            Directionality::new(
                TextDirection::Ltr,
                AnimatedCrossFade::new(
                    first_child,
                    Container::new(),
                    if towards_second {
                        CrossFadeState::ShowSecond
                    } else {
                        CrossFadeState::ShowFirst
                    },
                    at(50),
                ),
            )
            .into_widget()
        }

        fn ticker(&self) -> Handle<Ticker> {
            self.ticker.get().expect("the watching state started it")
        }
    }

    #[test]
    fn preserves_widget_state() {
        let cell = binding_cell();
        let watcher = Watcher::new();
        binding_mount(&cell, watcher.cross_fade(false));
        let mut app = cell.borrow_mut();
        assert_eq!(watcher.inits.get(), 1);

        pump_widget(&mut app, watcher.cross_fade(true), at(0));
        for time in [25, 50, 75] {
            binding_pump(&mut app, at(time));
            assert_eq!(watcher.inits.get(), 1);
        }
    }

    #[test]
    fn switches_off_ticker_mode_on_the_faded_out_widget() {
        let cell = binding_cell();
        let watcher = Watcher::new();
        binding_mount(&cell, watcher.cross_fade(false));
        let mut app = cell.borrow_mut();
        assert!(!watcher.ticker().muted(&app));

        pump_widget(&mut app, watcher.cross_fade(true), at(0));
        for time in [25, 50] {
            binding_pump(&mut app, at(time));
            // Animations are kept alive in the middle of cross-fade
            assert!(!watcher.ticker().muted(&app));
        }

        // In the final state animations should be off on the widget that's faded out.
        binding_pump(&mut app, at(75));
        assert!(watcher.ticker().muted(&app));
    }

    #[test]
    fn layout_builder_decides_which_children_are_in_the_tree() {
        let cell = binding_cell();
        let first = UniqueKey::new();
        let second = UniqueKey::new();
        let (first, second): (KeyRef, KeyRef) = (first.into(), second.into());
        let tree = |state: CrossFadeState, only_top: bool| {
            let mut cross_fade = AnimatedCrossFade::new(
                keyed(&first, 10.0, 10.0),
                keyed(&second, 10.0, 10.0),
                state,
                at(50),
            );
            if only_top {
                cross_fade =
                    cross_fade.layout_builder(|_app, top_child, _, _, _| top_child.clone());
            }
            Directionality::new(TextDirection::Ltr, cross_fade).into_widget()
        };
        let count = |app: &mut App, key: &KeyRef| {
            elements(app)
                .into_iter()
                .filter(|element| element.widget(app).key() == Some(key))
                .count()
        };
        binding_mount(&cell, tree(CrossFadeState::ShowFirst, false));
        let mut app = cell.borrow_mut();
        assert_eq!(count(&mut app, &first), 1);
        assert_eq!(count(&mut app, &second), 1);

        pump_widget(&mut app, tree(CrossFadeState::ShowFirst, true), at(0));
        assert_eq!(count(&mut app, &first), 1);
        assert_eq!(count(&mut app, &second), 0);

        pump_widget(&mut app, tree(CrossFadeState::ShowSecond, true), at(0));
        assert_eq!(count(&mut app, &second), 1);
        assert_eq!(count(&mut app, &first), 0);
    }

    /// The `Focus` nodes of the two children, as `Focus.of(tester.element(find.text(..)))` finds
    /// them.
    fn focus_tree(
        exclude_bottom_focus: Option<bool>,
        nodes: &Rc<[Cell<Option<crate::widgets::focus_manager::AnyFocusNode>>; 2]>,
    ) -> WidgetRef {
        let probe = |index: usize| {
            let nodes = Rc::clone(nodes);
            Focus::new(Builder::new(move |app, context| {
                nodes[index].set(Some(Focus::of(app, context, true, false)));
                SizedBox::shrink().into_widget()
            }))
            .into_widget()
        };
        let mut cross_fade =
            AnimatedCrossFade::new(probe(0), probe(1), CrossFadeState::ShowFirst, at(50));
        if let Some(exclude_bottom_focus) = exclude_bottom_focus {
            cross_fade = cross_fade.exclude_bottom_focus(exclude_bottom_focus);
        }
        Directionality::new(TextDirection::Ltr, cross_fade).into_widget()
    }

    fn assert_hidden_focus(exclude_bottom_focus: Option<bool>, hidden_gets_focus: bool) {
        let cell = binding_cell();
        let nodes = Rc::new([Cell::new(None), Cell::new(None)]);
        binding_mount(&cell, focus_tree(exclude_bottom_focus, &nodes));
        let mut app = cell.borrow_mut();

        let visible = nodes[0].get().expect("built");
        visible.request_focus(&mut app, None);
        binding_pump(&mut app, at(16));
        app.drain_microtasks();
        assert!(visible.has_primary_focus(&app));

        let hidden = nodes[1].get().expect("built");
        hidden.request_focus(&mut app, None);
        binding_pump(&mut app, at(32));
        app.drain_microtasks();
        assert_eq!(hidden.has_primary_focus(&app), hidden_gets_focus);
    }

    #[test]
    fn the_bottom_child_cannot_take_focus() {
        assert_hidden_focus(None, false);
    }

    #[test]
    fn the_bottom_child_can_have_focus_when_not_excluded() {
        assert_hidden_focus(Some(false), true);
    }

    #[test]
    fn does_not_crash_at_zero_area() {
        let cell = binding_cell();
        binding_mount(
            &cell,
            Directionality::new(
                TextDirection::Ltr,
                Center::new().child(SizedBox::shrink().child(AnimatedCrossFade::new(
                    sized(10.0, 10.0),
                    sized(20.0, 20.0),
                    CrossFadeState::ShowFirst,
                    Duration::from_secs(1),
                ))),
            )
            .into_widget(),
        );
        assert_eq!(cross_fade_size(&mut cell.borrow_mut()), Size::ZERO);
    }

    /// An `on_end` that counts its calls.
    fn counting_on_end() -> (Rc<Cell<u32>>, Listener) {
        let calls = Rc::new(Cell::new(0));
        let counted = Rc::clone(&calls);
        (
            calls,
            Listener::new(move |_app| counted.set(counted.get() + 1)),
        )
    }

    fn with_on_end(state: CrossFadeState, on_end: &Listener) -> WidgetRef {
        Directionality::new(
            TextDirection::Ltr,
            simple_cross_fade(state).on_end(on_end.clone()),
        )
        .into_widget()
    }

    #[test]
    fn on_end_is_called_when_the_animation_completes_forward() {
        let cell = binding_cell();
        let (calls, on_end) = counting_on_end();
        binding_mount(&cell, with_on_end(CrossFadeState::ShowFirst, &on_end));
        let mut app = cell.borrow_mut();
        assert_eq!(calls.get(), 0);

        pump_widget(
            &mut app,
            with_on_end(CrossFadeState::ShowSecond, &on_end),
            at(0),
        );
        binding_pump(&mut app, at(100));
        assert_eq!(calls.get(), 0);

        binding_pump(&mut app, at(200));
        binding_pump(&mut app, at(300));
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn on_end_is_called_when_the_animation_completes_reverse() {
        let cell = binding_cell();
        let (calls, on_end) = counting_on_end();
        binding_mount(&cell, with_on_end(CrossFadeState::ShowSecond, &on_end));
        let mut app = cell.borrow_mut();
        assert_eq!(calls.get(), 0);

        pump_widget(
            &mut app,
            with_on_end(CrossFadeState::ShowFirst, &on_end),
            at(0),
        );
        binding_pump(&mut app, at(100));
        assert_eq!(calls.get(), 0);

        binding_pump(&mut app, at(200));
        binding_pump(&mut app, at(300));
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn on_end_is_called_for_each_animation_cycle() {
        let cell = binding_cell();
        let (calls, on_end) = counting_on_end();
        binding_mount(&cell, with_on_end(CrossFadeState::ShowFirst, &on_end));
        let mut app = cell.borrow_mut();
        assert_eq!(calls.get(), 0);

        let mut time = 0;
        for (state, expected_calls) in [
            (CrossFadeState::ShowSecond, 1),
            (CrossFadeState::ShowFirst, 2),
            (CrossFadeState::ShowSecond, 3),
        ] {
            pump_widget(&mut app, with_on_end(state, &on_end), at(time));
            for _ in 0..3 {
                time += 100;
                binding_pump(&mut app, at(time));
            }
            assert_eq!(calls.get(), expected_calls);
        }
    }

    fn assert_clip_behavior(cross_fade: AnimatedCrossFade, expected: Clip) {
        let cell = binding_cell();
        binding_mount(&cell, centered(TextDirection::Ltr, cross_fade));
        let mut app = cell.borrow_mut();
        let clip_rect = find_by_type::<ClipRect>(&mut app);
        let clip_rect = clip_rect[0]
            .as_any()
            .downcast_ref::<ClipRect>()
            .expect("a ClipRect");
        assert_eq!(clip_rect.clip_behavior, expected);
        let animated_size = find_by_type::<AnimatedSize>(&mut app);
        let animated_size = animated_size[0]
            .as_any()
            .downcast_ref::<AnimatedSize>()
            .expect("an AnimatedSize");
        assert_eq!(animated_size.clip_behavior, expected);
    }

    #[test]
    fn clip_behavior_defaults_to_hard_edge() {
        assert_clip_behavior(simple_cross_fade(CrossFadeState::ShowFirst), Clip::HardEdge);
    }

    #[test]
    fn clip_behavior_is_forwarded_to_clip_rect_and_animated_size() {
        assert_clip_behavior(
            simple_cross_fade(CrossFadeState::ShowFirst).clip_behavior(Clip::None),
            Clip::None,
        );
    }

    #[test]
    fn the_alignment_default_is_top_center() {
        assert_eq!(
            simple_cross_fade(CrossFadeState::ShowFirst).alignment,
            AlignmentGeometry::TOP_CENTER
        );
    }

    /// flutter_test's `startGesture(position)` then `up()`: a touch down and up at `position`.
    fn touch_at(app: &mut App, position: Offset) {
        use waft_gestures::{GestureBinding, PointerDownEvent, PointerEvent, PointerUpEvent};
        GestureBinding::instance(app).handle_pointer_event(
            app,
            PointerEvent::Down(PointerDownEvent {
                pointer: 1,
                position,
                kind: waft_embedder::PointerDeviceKind::Touch,
                ..PointerDownEvent::default()
            }),
        );
        app.drain_microtasks();
        GestureBinding::instance(app).handle_pointer_event(
            app,
            PointerEvent::Up(PointerUpEvent {
                pointer: 1,
                position,
                kind: waft_embedder::PointerDeviceKind::Touch,
                ..PointerUpEvent::default()
            }),
        );
        app.drain_microtasks();
    }

    fn install_fonts(app: &mut App) {
        let binding = waft_painting::PaintingBinding::instance(app);
        if !binding.has_fonts(app) {
            binding.install_fonts(app, |fonts| {
                fonts.add_source(valo_system_fonts::SystemFonts::load());
            });
        }
    }

    /// button_tester.dart's `TestButton`, without a focus node, so its `onFocus` does nothing.
    fn test_button(on_pressed: Listener, child: WidgetRef) -> WidgetRef {
        let on_tap = on_pressed.clone();
        Semantics::new()
            .label("button")
            .button(true)
            .enabled(true)
            .on_tap(Rc::new(move |app: &mut App| on_tap.call(app)))
            .on_focus(Rc::new(|_app: &mut App| {}))
            .focusable(true)
            .child(FocusableActionDetector::new(
                GestureDetector::new().on_tap(on_pressed).child(child),
            ))
            .into_widget()
    }

    #[test]
    fn second_child_do_not_receive_touch_events() {
        let number_of_touch_event_noticed = Rc::new(Cell::new(0));
        let on_pressed = Listener::new({
            let noticed = Rc::clone(&number_of_touch_event_noticed);
            move |_app| noticed.set(noticed.get() + 1)
        });
        let build_animated_frame = |cross_fade_state: CrossFadeState| -> WidgetRef {
            SizedBox::new()
                .width(300.0)
                .height(600.0)
                .child(Directionality::new(
                    TextDirection::Ltr,
                    AnimatedCrossFade::new(
                        Text::new("AAA"),
                        SizedBox::new()
                            .width(f64::INFINITY)
                            .height(600.0)
                            .child(test_button(
                                on_pressed.clone(),
                                Text::new("BBB").into_widget(),
                            )),
                        cross_fade_state,
                        at(50),
                    ),
                ))
                .into_widget()
        };

        // flutter_test's 800x600 view: the touch at (150, 300) is below `binding_cell`'s.
        let platform = waft_test::TestPlatform::new()
            .on(waft_embedder::TargetPlatform::MacOS)
            .with_view(Rc::new(waft_test::TestView::new(800.0, 600.0)));
        let cell = waft_foundation::AppCell::with_platform(Rc::new(platform));
        install_fonts(&mut cell.borrow_mut());
        binding_mount(&cell, build_animated_frame(CrossFadeState::ShowSecond));
        let mut app = cell.borrow_mut();

        touch_at(&mut app, Offset::new(150.0, 300.0));
        assert_eq!(number_of_touch_event_noticed.get(), 1);

        pump_widget(
            &mut app,
            build_animated_frame(CrossFadeState::ShowFirst),
            at(0),
        );
        touch_at(&mut app, Offset::new(150.0, 300.0));
        touch_at(&mut app, Offset::new(150.0, 300.0));

        assert_eq!(number_of_touch_event_noticed.get(), 1);
    }
}
