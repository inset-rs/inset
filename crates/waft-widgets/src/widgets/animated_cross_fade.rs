//! Flutter counterpart: `widgets/animated_cross_fade.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    AnimationBehavior, AnimationController, AnimationRef, AnimationStatusListener, Curve,
    CurveTween, Curves, Drive, Tween,
};
use waft_embedder::Clip;
use waft_foundation::{App, Handle, Listener, OptionRetained, ValueKey};
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
use waft_foundation::Ref;

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
    controller: Option<Ref<AnimationController>>,
    first_animation: Option<AnimationRef<f64>>,
    second_animation: Option<AnimationRef<f64>>,
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
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl AnimatedCrossFadeState {
    fn controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .controller
            .handle()
            .expect("created in init_state")
    }

    fn init_animation(
        self: Handle<Self>,
        app: &mut App,
        curve: Rc<dyn Curve>,
        inverted: bool,
    ) -> AnimationRef<f64> {
        let curve_tween = CurveTween::new(app, curve);
        let controller = app
            .get(self)
            .controller
            .clone()
            .expect("created in init_state");
        let mut result = controller.drive(app, curve_tween);
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
        app.get_mut(self).controller = Some(controller.clone());
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
            .clone()
            .expect("created in init_state");
        let second_animation = app
            .get(self)
            .second_animation
            .clone()
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
    use crate::testing::{
        WidgetTester, expect, find, finds_n_widgets, finds_nothing, finds_one_widget, test_widgets,
    };
    use crate::widgets::actions::FocusableActionDetector;
    use crate::widgets::basic::{Builder, Center, Directionality, Semantics, SizedBox};
    use crate::widgets::container::Container;
    use crate::widgets::focus_scope::Focus;
    use crate::widgets::gesture_detector::GestureDetector;
    use crate::widgets::text::Text;
    use crate::widgets::ticker_provider::{
        SingleTickerProviderStateMixin, SingleTickerProviderStateMixinData,
    };

    fn ms(milliseconds: u64) -> Duration {
        Duration::from_millis(milliseconds)
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
        AnimatedCrossFade::new(sized(100.0, 100.0), sized(200.0, 200.0), state, ms(200))
    }

    #[test]
    fn animates_between_the_sizes_of_its_children() {
        test_widgets(|tester| {
            tester.pump_widget(centered(
                TextDirection::Ltr,
                simple_cross_fade(CrossFadeState::ShowFirst),
            ));
            expect(
                tester,
                &find::by_type::<FadeTransition>(),
                finds_n_widgets(2),
            );
            assert_eq!(
                tester.get_size(find::by_type::<AnimatedCrossFade>()),
                Size::new(100.0, 100.0)
            );

            tester.pump_widget(centered(
                TextDirection::Ltr,
                simple_cross_fade(CrossFadeState::ShowSecond),
            ));
            tester.pump_by(ms(100));

            expect(
                tester,
                &find::by_type::<FadeTransition>(),
                finds_n_widgets(2),
            );
            assert_eq!(
                tester.get_size(find::by_type::<AnimatedCrossFade>()),
                Size::new(150.0, 150.0)
            );
        });
    }

    #[test]
    fn show_second_starts_at_the_size_of_the_second_child() {
        test_widgets(|tester| {
            tester.pump_widget(centered(
                TextDirection::Ltr,
                simple_cross_fade(CrossFadeState::ShowSecond),
            ));
            expect(
                tester,
                &find::by_type::<FadeTransition>(),
                finds_n_widgets(2),
            );
            assert_eq!(
                tester.get_size(find::by_type::<AnimatedCrossFade>()),
                Size::new(200.0, 200.0)
            );
        });
    }

    /// The first and second children keep their places in the stack while the size animates:
    /// `alignment` and the text direction decide where both sit.
    fn assert_aligned_halfway(
        text_direction: TextDirection,
        alignment: AlignmentGeometry,
        expected: Offset,
    ) {
        test_widgets(|tester| {
            let (first_key, second_key): (KeyRef, KeyRef) =
                (UniqueKey::new().into(), UniqueKey::new().into());
            let tree = |state: CrossFadeState| {
                centered(
                    text_direction,
                    AnimatedCrossFade::new(
                        keyed(&first_key, 100.0, 100.0),
                        keyed(&second_key, 200.0, 200.0),
                        state,
                        ms(200),
                    )
                    .alignment(alignment),
                )
            };
            tester.pump_widget(tree(CrossFadeState::ShowFirst));
            tester.pump_widget(tree(CrossFadeState::ShowSecond));
            tester.pump_by(ms(100));

            assert_eq!(
                tester.get_top_left(find::by_key(first_key.clone())),
                expected
            );
            assert_eq!(
                tester.get_top_left(find::by_key(second_key.clone())),
                expected
            );
        });
    }

    #[test]
    fn alignment_visual() {
        assert_aligned_halfway(
            TextDirection::Ltr,
            AlignmentGeometry::BOTTOM_RIGHT,
            Offset::new(275.0, 175.0),
        );
    }

    #[test]
    fn alignment_ltr() {
        assert_aligned_halfway(
            TextDirection::Ltr,
            AlignmentGeometry::BOTTOM_END,
            Offset::new(275.0, 175.0),
        );
    }

    #[test]
    fn alignment_rtl() {
        assert_aligned_halfway(
            TextDirection::Rtl,
            AlignmentGeometry::BOTTOM_END,
            Offset::new(325.0, 175.0),
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
        ) -> Ref<Ticker> {
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
            widget.ticker.set(Some(ticker.handle()));
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
                    ms(50),
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
        test_widgets(|tester| {
            let watcher = Watcher::new();
            tester.pump_widget(watcher.cross_fade(false));
            assert_eq!(watcher.inits.get(), 1);

            tester.pump_widget(watcher.cross_fade(true));
            for _ in 0..3 {
                tester.pump_by(ms(25));
                assert_eq!(watcher.inits.get(), 1);
            }
        });
    }

    #[test]
    fn switches_off_ticker_mode_on_the_faded_out_widget() {
        test_widgets(|tester| {
            let watcher = Watcher::new();
            tester.pump_widget(watcher.cross_fade(false));
            assert!(!watcher.ticker().muted(&tester.cell.borrow()));

            tester.pump_widget(watcher.cross_fade(true));
            for _ in 0..2 {
                tester.pump_by(ms(25));
                // Animations are kept alive in the middle of cross-fade
                assert!(!watcher.ticker().muted(&tester.cell.borrow()));
            }

            // In the final state animations should be off on the widget that's faded out.
            tester.pump_by(ms(25));
            assert!(watcher.ticker().muted(&tester.cell.borrow()));
        });
    }

    #[test]
    fn layout_builder_decides_which_children_are_in_the_tree() {
        test_widgets(|tester| {
            let (first, second): (KeyRef, KeyRef) =
                (UniqueKey::new().into(), UniqueKey::new().into());
            let tree = |state: CrossFadeState, only_top: bool| {
                let mut cross_fade = AnimatedCrossFade::new(
                    keyed(&first, 10.0, 10.0),
                    keyed(&second, 10.0, 10.0),
                    state,
                    ms(50),
                );
                if only_top {
                    cross_fade =
                        cross_fade.layout_builder(|_app, top_child, _, _, _| top_child.clone());
                }
                Directionality::new(TextDirection::Ltr, cross_fade).into_widget()
            };
            tester.pump_widget(tree(CrossFadeState::ShowFirst, false));
            expect(tester, &find::by_key(first.clone()), finds_one_widget());
            expect(tester, &find::by_key(second.clone()), finds_one_widget());

            tester.pump_widget(tree(CrossFadeState::ShowFirst, true));
            expect(tester, &find::by_key(first.clone()), finds_one_widget());
            expect(tester, &find::by_key(second.clone()), finds_nothing());

            tester.pump_widget(tree(CrossFadeState::ShowSecond, true));
            expect(tester, &find::by_key(second.clone()), finds_one_widget());
            expect(tester, &find::by_key(first.clone()), finds_nothing());
        });
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
            AnimatedCrossFade::new(probe(0), probe(1), CrossFadeState::ShowFirst, ms(50));
        if let Some(exclude_bottom_focus) = exclude_bottom_focus {
            cross_fade = cross_fade.exclude_bottom_focus(exclude_bottom_focus);
        }
        Directionality::new(TextDirection::Ltr, cross_fade).into_widget()
    }

    fn assert_hidden_focus(exclude_bottom_focus: Option<bool>, hidden_gets_focus: bool) {
        test_widgets(|tester| {
            let nodes = Rc::new([Cell::new(None), Cell::new(None)]);
            tester.pump_widget(focus_tree(exclude_bottom_focus, &nodes));

            let visible = nodes[0].get().expect("built");
            tester.update(|app| visible.request_focus(app, None));
            tester.pump();
            assert!(visible.has_primary_focus(&tester.cell.borrow()));

            let hidden = nodes[1].get().expect("built");
            tester.update(|app| hidden.request_focus(app, None));
            tester.pump();
            assert_eq!(
                hidden.has_primary_focus(&tester.cell.borrow()),
                hidden_gets_focus
            );
        });
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
        test_widgets(|tester| {
            tester.pump_widget(Directionality::new(
                TextDirection::Ltr,
                Center::new().child(SizedBox::shrink().child(AnimatedCrossFade::new(
                    sized(10.0, 10.0),
                    sized(20.0, 20.0),
                    CrossFadeState::ShowFirst,
                    Duration::from_secs(1),
                ))),
            ));
            assert_eq!(
                tester.get_size(find::by_type::<AnimatedCrossFade>()),
                Size::ZERO
            );
        });
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

    /// Dart's `onEnd` cases: from `from`, a pump to `to`, no call 100ms in, one once settled.
    fn assert_on_end_once(from: CrossFadeState, to: CrossFadeState) {
        test_widgets(|tester| {
            let (calls, on_end) = counting_on_end();
            tester.pump_widget(with_on_end(from, &on_end));
            assert_eq!(calls.get(), 0);

            tester.pump_widget(with_on_end(to, &on_end));
            tester.pump_by(ms(100));
            assert_eq!(calls.get(), 0);

            tester.pump_and_settle();
            assert_eq!(calls.get(), 1);
        });
    }

    #[test]
    fn on_end_is_called_when_the_animation_completes_forward() {
        assert_on_end_once(CrossFadeState::ShowFirst, CrossFadeState::ShowSecond);
    }

    #[test]
    fn on_end_is_called_when_the_animation_completes_reverse() {
        assert_on_end_once(CrossFadeState::ShowSecond, CrossFadeState::ShowFirst);
    }

    #[test]
    fn on_end_is_called_for_each_animation_cycle() {
        test_widgets(|tester| {
            let (calls, on_end) = counting_on_end();
            tester.pump_widget(with_on_end(CrossFadeState::ShowFirst, &on_end));
            assert_eq!(calls.get(), 0);

            for (state, expected_calls) in [
                (CrossFadeState::ShowSecond, 1),
                (CrossFadeState::ShowFirst, 2),
                (CrossFadeState::ShowSecond, 3),
            ] {
                tester.pump_widget(with_on_end(state, &on_end));
                tester.pump_and_settle();
                assert_eq!(calls.get(), expected_calls);
            }
        });
    }

    fn assert_clip_behavior(cross_fade: fn() -> AnimatedCrossFade, expected: Clip) {
        test_widgets(|tester| {
            tester.pump_widget(centered(TextDirection::Ltr, cross_fade()));
            assert_eq!(
                tester.widget(find::by_type::<ClipRect>(), |clip_rect: &ClipRect| {
                    clip_rect.clip_behavior
                }),
                expected
            );
            assert_eq!(
                tester.widget(
                    find::by_type::<AnimatedSize>(),
                    |animated_size: &AnimatedSize| { animated_size.clip_behavior }
                ),
                expected
            );
        });
    }

    #[test]
    fn clip_behavior_defaults_to_hard_edge() {
        assert_clip_behavior(
            || simple_cross_fade(CrossFadeState::ShowFirst),
            Clip::HardEdge,
        );
    }

    #[test]
    fn clip_behavior_is_forwarded_to_clip_rect_and_animated_size() {
        assert_clip_behavior(
            || simple_cross_fade(CrossFadeState::ShowFirst).clip_behavior(Clip::None),
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
        test_widgets(|tester| {
            let number_of_touch_event_noticed = Rc::new(Cell::new(0));
            let on_pressed = Listener::new({
                let noticed = Rc::clone(&number_of_touch_event_noticed);
                move |_app| noticed.set(noticed.get() + 1)
            });
            let build_animated_frame =
                |cross_fade_state: CrossFadeState| -> WidgetRef {
                    SizedBox::new()
                        .width(300.0)
                        .height(600.0)
                        .child(Directionality::new(
                            TextDirection::Ltr,
                            AnimatedCrossFade::new(
                                Text::new("AAA"),
                                SizedBox::new().width(f64::INFINITY).height(600.0).child(
                                    test_button(on_pressed.clone(), Text::new("BBB").into_widget()),
                                ),
                                cross_fade_state,
                                ms(50),
                            ),
                        ))
                        .into_widget()
                };
            let touch_second_button = |tester: &mut WidgetTester| {
                tester.tap_at(Offset::new(150.0, 300.0), 1);
            };

            tester.pump_widget(build_animated_frame(CrossFadeState::ShowSecond));

            touch_second_button(tester);
            assert_eq!(number_of_touch_event_noticed.get(), 1);

            tester.pump_widget(build_animated_frame(CrossFadeState::ShowFirst));
            touch_second_button(tester);
            touch_second_button(tester);

            assert_eq!(number_of_touch_event_noticed.get(), 1);
        });
    }
}
