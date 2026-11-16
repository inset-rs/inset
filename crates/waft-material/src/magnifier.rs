//! Flutter counterpart: `material/magnifier.dart`.

use std::rc::Rc;
use std::time::Duration;

use waft_embedder::{BlurStyle, Clip, Color, Offset, Radius, Size, TargetPlatform, clamp_double};
use waft_foundation::{
    App, Handle, Listenable, Listener, Ref, Timer, ValueListenable, ValueNotifier,
};
use waft_painting::{BorderRadius, BorderSide, BoxShadow, RoundedRectangleBorder};
use waft_widgets::{
    AnimatedPositioned, BuildContext, ColoredBox, IntoWidget, KeyRef, MagnifierBuilder,
    MagnifierController, MagnifierDecoration, MagnifierInfo, MediaQuery, RawMagnifier, State,
    StateData, StatefulWidget, StatelessWidget, TextMagnifierConfiguration, WidgetRef,
};

use waft_cupertino::CupertinoTextMagnifier;

/// A [`Magnifier`] positioned by rules dictated by the native Android magnifier.
///
/// The positioning rules are based on [`magnifier_info`](Self::magnifier_info), as follows:
///
/// - The loupe tracks the gesture's _x_ coordinate, clamping to the beginning
///   and end of the currently editing line.
///
/// - The focal point never contains anything out of the bounds of the text
///   field or other widget being magnified (the [`MagnifierInfo::field_bounds`]).
///
/// - The focal point always remains aligned with the _y_ coordinate of the touch.
///
/// - The loupe always remains on the screen.
///
/// - When the line targeted by the touch's _y_ coordinate changes, the position
///   is animated over [`JUMP_BETWEEN_LINES_ANIMATION_DURATION`](Self::JUMP_BETWEEN_LINES_ANIMATION_DURATION).
///
/// This behavior was based on the Android 12 source code, where possible, and
/// on eyeballing a Pixel 6 running Android 12 otherwise.
#[derive(Debug)]
pub struct TextMagnifier {
    pub key: Option<KeyRef>,
    /// The current status of the user's touch.
    ///
    /// As the value of the [`magnifier_info`](Self::magnifier_info) changes, the position of the
    /// loupe is adjusted automatically, according to the rules described in the
    /// [`TextMagnifier`] class description.
    pub magnifier_info: Ref<ValueNotifier<MagnifierInfo>>,
}

impl TextMagnifier {
    /// Creates a [`TextMagnifier`].
    ///
    /// The `magnifier_info` must be provided, and must be updated with new values
    /// as the user's touch changes.
    pub fn new(magnifier_info: Ref<ValueNotifier<MagnifierInfo>>) -> TextMagnifier {
        TextMagnifier {
            key: None,
            magnifier_info,
        }
    }

    /// Dart `TextMagnifier(key:)`.
    pub fn key(mut self, key: KeyRef) -> TextMagnifier {
        self.key = Some(key);
        self
    }

    /// Dart's `adaptiveMagnifierConfiguration`: a [`TextMagnifierConfiguration`] that returns a
    /// [`CupertinoTextMagnifier`] on iOS, [`TextMagnifier`] on Android, and `None` on all other
    /// platforms, and shows the editing handles only on iOS.
    ///
    /// Every call returns a configuration with the same builder, so two of them are the same
    /// configuration to a widget that compares them.
    pub fn adaptive_magnifier_configuration(app: &App) -> TextMagnifierConfiguration {
        thread_local! {
            static MAGNIFIER_BUILDER: MagnifierBuilder = Rc::new(
                |app: &mut App,
                 _context: BuildContext,
                 controller: Ref<MagnifierController>,
                 magnifier_info: Ref<ValueNotifier<MagnifierInfo>>| {
                    match app.platform().target_platform() {
                        TargetPlatform::IOS => Some(
                            CupertinoTextMagnifier::new(controller, magnifier_info).into_widget(),
                        ),
                        TargetPlatform::Android => {
                            Some(TextMagnifier::new(magnifier_info).into_widget())
                        }
                        TargetPlatform::Fuchsia
                        | TargetPlatform::Linux
                        | TargetPlatform::MacOS
                        | TargetPlatform::Windows => None,
                    }
                },
            );
        }
        TextMagnifierConfiguration::new()
            .should_display_handles_in_magnifier(
                app.platform().target_platform() == TargetPlatform::IOS,
            )
            .with_magnifier_builder(MAGNIFIER_BUILDER.with(MagnifierBuilder::clone))
    }

    /// The duration that the position is animated if [`TextMagnifier`] just switched
    /// between lines.
    pub const JUMP_BETWEEN_LINES_ANIMATION_DURATION: Duration = Duration::from_millis(70);
}

impl StatefulWidget for TextMagnifier {
    type State = TextMagnifierState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> TextMagnifierState {
        TextMagnifierState {
            state: StateData::new(),
            magnifier_position: None,
            position_should_be_animated_timer: None,
            extra_focal_point_offset: Offset::ZERO,
        }
    }
}

/// Dart's `_TextMagnifierState`.
pub struct TextMagnifierState {
    state: StateData<TextMagnifier>,
    // Should _only_ be null on construction. This is because of the animation logic.
    //
    // Animations are added when `last_build_y != current_build_y`. This condition
    // is true on the initial render, which would mean that the initial
    // build would be animated - this is undesired. Thus, this is null for the
    // first frame and the condition becomes `magnifier_position != null && last_build_y != this_build_y`.
    magnifier_position: Option<Offset>,
    // A timer that unsets itself after an animation duration.
    // If the timer exists, then the magnifier animates its position -
    // if this timer does not exist, the magnifier tracks the gesture (with respect
    // to the positioning rules) directly.
    position_should_be_animated_timer: Option<Timer>,
    extra_focal_point_offset: Offset,
}

impl TextMagnifierState {
    fn position_should_be_animated(self: Handle<Self>, app: &App) -> bool {
        app.get(self).position_should_be_animated_timer.is_some()
    }

    fn determine_magnifier_position_and_focal_point(self: Handle<Self>, app: &mut App) {
        let magnifier_info = self.widget(app).magnifier_info.clone();
        let selection_info = *magnifier_info.value(app);
        let context = self.context(app);
        let screen_rect = Offset::ZERO & MediaQuery::size_of(app, context);

        // Since by default we draw at the top left corner, this offset
        // shifts the magnifier so we draw at the center, and then also includes
        // the "above touch point" shift.
        let basic_magnifier_offset = Offset::new(
            Magnifier::K_DEFAULT_MAGNIFIER_SIZE.width() / 2.0,
            Magnifier::K_DEFAULT_MAGNIFIER_SIZE.height()
                + Magnifier::K_STANDARD_VERTICAL_FOCAL_POINT_SHIFT,
        );

        // Since the magnifier should not go past the edges of the line,
        // but must track the gesture otherwise, constrain the X of the magnifier
        // to always stay between line start and end.
        let magnifier_x = clamp_double(
            selection_info.global_gesture_position.dx(),
            selection_info.current_line_boundaries.left,
            selection_info.current_line_boundaries.right,
        );

        // Place the magnifier at the previously calculated X, and the Y should be
        // exactly at the center of the handle.
        let unadjusted_magnifier_rect =
            (Offset::new(magnifier_x, selection_info.caret_rect.center().dy())
                - basic_magnifier_offset)
                & Magnifier::K_DEFAULT_MAGNIFIER_SIZE;

        // Shift the magnifier so that, if we are ever out of the screen, we become in bounds.
        // This probably won't have much of an effect on the X, since it is already bound
        // to the currentLineBoundaries, but will shift vertically if the magnifier is out of bounds.
        let screen_bounds_adjusted_magnifier_rect =
            MagnifierController::shift_within_bounds(unadjusted_magnifier_rect, screen_rect);

        // Done with the magnifier position!
        let final_magnifier_position = screen_bounds_adjusted_magnifier_rect.top_left();

        // The insets, from either edge, that the focal point should not point
        // past lest the magnifier displays something out of bounds.
        let horizontal_max_focal_point_edge_insets =
            (Magnifier::K_DEFAULT_MAGNIFIER_SIZE.width() / 2.0) / Magnifier::MAGNIFICATION;

        // Adjust the focal point horizontally such that none of the magnifier
        // ever points to anything out of bounds.
        // If the text field is so narrow that we must show out of bounds,
        // then settle for pointing to the center all the time.
        let new_global_focal_point_x =
            if selection_info.field_bounds.width() < horizontal_max_focal_point_edge_insets * 2.0 {
                selection_info.field_bounds.center().dx()
            } else {
                // Otherwise, we can clamp the focal point to always point in bounds.
                clamp_double(
                    screen_bounds_adjusted_magnifier_rect.center().dx(),
                    selection_info.field_bounds.left + horizontal_max_focal_point_edge_insets,
                    selection_info.field_bounds.right - horizontal_max_focal_point_edge_insets,
                )
            };

        // Since the previous value is now a global offset (i.e. `new_global_focal_point`
        // is now a global offset), we must subtract the magnifier's global offset
        // to obtain the relative shift in the focal point.
        let new_relative_focal_point_x =
            new_global_focal_point_x - screen_bounds_adjusted_magnifier_rect.center().dx();

        // The Y component means that if we are pressed up against the top of the screen,
        // then we should adjust the focal point such that it now points to how far we moved
        // the magnifier. screen_bounds_adjusted_magnifier_rect.top == unadjusted_magnifier_rect.top for most cases,
        // but when pressed up against the top of the screen, we adjust the focal point by
        // the amount that we shifted from our "natural" position.
        let focal_point_adjustment_for_screen_bounds_adjustment = Offset::new(
            new_relative_focal_point_x,
            unadjusted_magnifier_rect.top - screen_bounds_adjusted_magnifier_rect.top,
        );

        let mut position_should_be_animated = app.get(self).position_should_be_animated_timer;

        if let Some(magnifier_position) = app.get(self).magnifier_position
            && final_magnifier_position.dy() != magnifier_position.dy()
        {
            if let Some(timer) = position_should_be_animated
                && timer.is_active(app)
            {
                timer.cancel(app);
            }

            // Create a timer that deletes itself when the timer is complete.
            // This is `mounted` safe, since the timer is canceled in `dispose`.
            position_should_be_animated = Some(Timer::new(
                app,
                TextMagnifier::JUMP_BETWEEN_LINES_ANIMATION_DURATION,
                Listener::new(move |app| {
                    self.set_state(app, |state| {
                        state.position_should_be_animated_timer = None;
                    });
                }),
            ));
        }

        self.set_state(app, |state| {
            state.magnifier_position = Some(final_magnifier_position);
            state.position_should_be_animated_timer = position_should_be_animated;
            state.extra_focal_point_offset = focal_point_adjustment_for_screen_bounds_adjustment;
        });
    }
}

impl State for TextMagnifierState {
    type Widget = TextMagnifier;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let magnifier_info = self.widget(app).magnifier_info.clone();
        magnifier_info.add_listener(
            app,
            Listener::handle_method(self, Self::determine_magnifier_position_and_focal_point),
        );
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let magnifier_info = self.widget(app).magnifier_info.clone();
        magnifier_info.remove_listener(
            app,
            &Listener::handle_method(self, Self::determine_magnifier_position_and_focal_point),
        );
        if let Some(timer) = app.get(self).position_should_be_animated_timer {
            timer.cancel(app);
        }
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.determine_magnifier_position_and_focal_point(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &TextMagnifier) {
        let magnifier_info = self.widget(app).magnifier_info.clone();
        if old_widget.magnifier_info != magnifier_info {
            old_widget.magnifier_info.remove_listener(
                app,
                &Listener::handle_method(self, Self::determine_magnifier_position_and_focal_point),
            );
            magnifier_info.add_listener(
                app,
                Listener::handle_method(self, Self::determine_magnifier_position_and_focal_point),
            );
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let magnifier_position = app.get(self).magnifier_position;
        debug_assert!(
            magnifier_position.is_some(),
            "Magnifier position should only be null before the first build."
        );
        let magnifier_position = magnifier_position.expect("set in did_change_dependencies");
        let extra_focal_point_offset = app.get(self).extra_focal_point_offset;
        let duration = if self.position_should_be_animated(app) {
            TextMagnifier::JUMP_BETWEEN_LINES_ANIMATION_DURATION
        } else {
            Duration::ZERO
        };

        AnimatedPositioned::new(
            Magnifier::new().additional_focal_point_offset(extra_focal_point_offset),
            // Material magnifier typically does not animate, unless we jump between lines,
            // in which case we animate between lines.
            duration,
        )
        .top(magnifier_position.dy())
        .left(magnifier_position.dx())
        .into_widget()
    }
}

/// A Material-styled magnifying glass.
///
/// This widget focuses on mimicking the _style_ of the magnifier on material.
/// For a widget that is focused on mimicking the _behavior_ of a material
/// magnifier, see [`TextMagnifier`], which uses [`Magnifier`].
///
/// The styles implemented in this widget were based on the Android 12 source
/// code, where possible, and on eyeballing a Pixel 6 running Android 12
/// otherwise.
#[derive(Debug)]
pub struct Magnifier {
    pub key: Option<KeyRef>,
    /// Any additional offset the focal point requires to "point"
    /// to the correct place.
    ///
    /// This value is added to [`K_STANDARD_VERTICAL_FOCAL_POINT_SHIFT`](Self::K_STANDARD_VERTICAL_FOCAL_POINT_SHIFT)
    /// to obtain the actual offset.
    ///
    /// This is useful for instances where the magnifier is not pointing to
    /// something directly below it.
    pub additional_focal_point_offset: Offset,
    /// The border radius for this magnifier.
    ///
    /// The magnifier's shape is a [`RoundedRectangleBorder`] with this radius.
    pub border_radius: BorderRadius,
    /// The color to tint the image in this [`Magnifier`].
    ///
    /// On native Android, there is a almost transparent gray tint to the
    /// magnifier, in order to better distinguish the contents of the lens from
    /// the background.
    pub film_color: Color,
    /// A list of shadows cast by the [`Magnifier`].
    ///
    /// If the shadows use a [`BlurStyle`] that paints inside the shape, or if they
    /// are offset, then a [`clip_behavior`](Self::clip_behavior) that enables clipping (such as the
    /// default [`Clip::HardEdge`]) is recommended, otherwise the shadow will occlude
    /// the magnifier (the shadow is drawn above the magnifier so as to not be
    /// included in the magnified image).
    ///
    /// By default, the shadows are offset vertically by two logical pixels, so
    /// clipping is recommended.
    ///
    /// A shadow that uses [`BlurStyle::Outer`] and is not offset does not need
    /// clipping; in that case, consider setting [`clip_behavior`](Self::clip_behavior) to [`Clip::None`].
    pub shadows: Vec<BoxShadow>,
    /// Whether and how to clip the [`shadows`](Self::shadows) that render inside the loupe.
    ///
    /// Defaults to [`Clip::HardEdge`].
    ///
    /// A value of [`Clip::None`] can be used if the shadow will not paint where the
    /// magnified image appears, or if doing so is intentional (e.g. to blur the
    /// edges of the magnified image).
    ///
    /// See the discussion at [`shadows`](Self::shadows).
    pub clip_behavior: Clip,
    /// The [`Size`] of this [`Magnifier`].
    ///
    /// The [`shadows`](Self::shadows) are drawn outside of the [`size`](Self::size).
    pub size: Size,
}

impl Magnifier {
    /// Creates a [`RawMagnifier`] in the Material style.
    pub fn new() -> Magnifier {
        Magnifier::default()
    }

    /// Dart `Magnifier(key:)`.
    pub fn key(mut self, key: KeyRef) -> Magnifier {
        self.key = Some(key);
        self
    }

    /// Dart `Magnifier(additionalFocalPointOffset:)`.
    pub fn additional_focal_point_offset(
        mut self,
        additional_focal_point_offset: Offset,
    ) -> Magnifier {
        self.additional_focal_point_offset = additional_focal_point_offset;
        self
    }

    /// Dart `Magnifier(borderRadius:)`.
    pub fn border_radius(mut self, border_radius: BorderRadius) -> Magnifier {
        self.border_radius = border_radius;
        self
    }

    /// Dart `Magnifier(filmColor:)`.
    pub fn film_color(mut self, film_color: Color) -> Magnifier {
        self.film_color = film_color;
        self
    }

    /// Dart `Magnifier(shadows:)`.
    pub fn shadows(mut self, shadows: Vec<BoxShadow>) -> Magnifier {
        self.shadows = shadows;
        self
    }

    /// Dart `Magnifier(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> Magnifier {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `Magnifier(size:)`.
    pub fn size(mut self, size: Size) -> Magnifier {
        self.size = size;
        self
    }

    /// The default size of this [`Magnifier`].
    ///
    /// The size of the magnifier may be modified through the constructor;
    /// [`K_DEFAULT_MAGNIFIER_SIZE`](Self::K_DEFAULT_MAGNIFIER_SIZE) is extracted from the default
    /// parameter of [`Magnifier`]'s constructor so that positioners may depend on it.
    pub const K_DEFAULT_MAGNIFIER_SIZE: Size = Size::new(77.37, 37.9);

    /// The vertical distance that the magnifier should be above the focal point.
    ///
    /// The [`K_STANDARD_VERTICAL_FOCAL_POINT_SHIFT`](Self::K_STANDARD_VERTICAL_FOCAL_POINT_SHIFT)
    /// value is a constant so that positioning of this [`Magnifier`] can be done with a guaranteed
    /// size, as opposed to an estimate.
    pub const K_STANDARD_VERTICAL_FOCAL_POINT_SHIFT: f64 = 22.0;

    const BORDER_RADIUS: f64 = 40.0;
    const MAGNIFICATION: f64 = 1.25;
}

impl Default for Magnifier {
    fn default() -> Magnifier {
        Magnifier {
            key: None,
            additional_focal_point_offset: Offset::ZERO,
            border_radius: BorderRadius::all(Radius::circular(Magnifier::BORDER_RADIUS)),
            film_color: Color::from_argb(8, 158, 158, 158),
            shadows: vec![BoxShadow::new(
                Color::from_argb(25, 0, 0, 0),
                Offset::new(0.0, 2.0),
                1.5,
                0.75,
                BlurStyle::Normal,
            )],
            clip_behavior: Clip::HardEdge,
            size: Magnifier::K_DEFAULT_MAGNIFIER_SIZE,
        }
    }
}

impl StatelessWidget for Magnifier {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        RawMagnifier::new(self.size)
            .decoration(
                MagnifierDecoration::new()
                    .shape(RoundedRectangleBorder::new(
                        BorderSide::NONE,
                        self.border_radius.into(),
                    ))
                    .shadows(self.shadows.clone()),
            )
            .clip_behavior(self.clip_behavior)
            .magnification_scale(Magnifier::MAGNIFICATION)
            .focal_point_offset(
                self.additional_focal_point_offset
                    + Offset::new(
                        0.0,
                        Magnifier::K_STANDARD_VERTICAL_FOCAL_POINT_SHIFT
                            + Magnifier::K_DEFAULT_MAGNIFIER_SIZE.height() / 2.0,
                    ),
            )
            .child(
                // This couldn't be part of the decoration (even if the
                // MagnifierDecoration supported specifying a color) because the
                // decoration's shadows are offset and therefore we set a clipBehavior
                // that clips the inner part of the decoration to avoid occluding the
                // magnified image with the shadow.
                ColoredBox::new(self.film_color),
            )
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::{Rect, TargetPlatform};
    use waft_widgets::{
        GlobalKey, ImplicitlyAnimatedWidget, Positioned, SizedBox, downcast_widget,
    };

    use waft_widgets::testing::{WidgetTester, find, test_widgets};

    use super::*;
    use crate::app::MaterialApp;
    use crate::theme_data::ThemeData;

    const REASONABLE_TEXT_FIELD: Rect = Rect::from_ltrb(50.0, 100.0, 200.0, 100.0);
    const BASIC_OFFSET: Offset = Offset::new(
        Magnifier::K_DEFAULT_MAGNIFIER_SIZE.width() / 2.0,
        Magnifier::K_STANDARD_VERTICAL_FOCAL_POINT_SHIFT
            + Magnifier::K_DEFAULT_MAGNIFIER_SIZE.height(),
    );

    fn info(
        line: Rect,
        field: Rect,
        caret: Rect,
        global_gesture_position: Offset,
    ) -> MagnifierInfo {
        MagnifierInfo::new(global_gesture_position, caret, field, line)
    }

    /// An info whose line, field and caret are all `rect`, the gesture at `position`.
    fn info_over(rect: Rect, position: Offset) -> MagnifierInfo {
        info(rect, rect, rect, position)
    }

    /// Dart's `MaterialApp(home: Placeholder())`, whose home is a sized box under `key`.
    fn pump_app(tester: &mut WidgetTester, key: &GlobalKey) {
        let root = {
            let app = tester.cell.borrow();
            MaterialApp::new()
                .theme(ThemeData::new(&app))
                .home(SizedBox::new().key(key.clone().into()))
                .into_widget()
        };
        tester.pump_widget(root);
    }

    fn app_context(tester: &mut WidgetTester) -> BuildContext {
        let home = GlobalKey::new();
        pump_app(tester, &home);
        home.current_context(&tester.cell.borrow_mut())
            .expect("mounted")
    }

    /// Dart's `showMagnifier`: shows a [`TextMagnifier`] over `magnifier_info` and settles.
    fn show_magnifier(
        tester: &mut WidgetTester,
        context: BuildContext,
        magnifier_info: MagnifierInfo,
    ) -> (Ref<MagnifierController>, Ref<ValueNotifier<MagnifierInfo>>) {
        let (controller, notifier) = {
            let mut app = tester.cell.borrow_mut();
            let notifier = app.create(ValueNotifier::new(magnifier_info));
            let controller = MagnifierController::new(&mut app);
            let shown = notifier.clone();
            controller.show(
                &mut app,
                context,
                Rc::new(move |_app: &mut App, _context: BuildContext| {
                    TextMagnifier::new(shown.clone()).into_widget()
                }),
                None,
            );
            (controller, notifier)
        };
        tester.pump_and_settle();
        (controller, notifier)
    }

    fn magnifier_position(tester: &WidgetTester) -> Offset {
        let positioned = find::by_type::<Positioned>().evaluate(tester)[0];
        tester.widget::<Positioned, _>(positioned, |positioned| {
            Offset::new(
                positioned.left.unwrap_or(0.0),
                positioned.top.unwrap_or(0.0),
            )
        })
    }

    fn magnifier_additional_focal_point(tester: &WidgetTester) -> Offset {
        let magnifier = find::by_type::<Magnifier>().evaluate(tester)[0];
        tester.widget::<Magnifier, _>(magnifier, |magnifier| {
            magnifier.additional_focal_point_offset
        })
    }

    fn is_animated(tester: &WidgetTester) -> bool {
        let positioned = find::by_type::<AnimatedPositioned>().evaluate(tester)[0];
        tester.widget::<AnimatedPositioned, _>(positioned, |positioned| {
            !positioned
                .implicitly_animated_widget_data()
                .duration
                .is_zero()
        })
    }

    /// What the configuration's builder builds on `platform`.
    fn built_by_adaptive_configuration(platform: TargetPlatform) -> Option<WidgetRef> {
        let mut tester = WidgetTester::on(platform);
        let context = app_context(&mut tester);
        let mut app = tester.cell.borrow_mut();
        let notifier = app.create(ValueNotifier::new(MagnifierInfo::EMPTY));
        let controller = MagnifierController::new(&mut app);
        let builder = TextMagnifier::adaptive_magnifier_configuration(&app).magnifier_builder();
        builder(&mut app, context, controller, notifier)
    }

    mod adaptive_magnifier_controller_builder {
        use super::*;

        #[test]
        fn should_return_a_text_editing_magnifier_on_android() {
            test_widgets(|_tester| {
                let built = built_by_adaptive_configuration(TargetPlatform::Android)
                    .expect("a magnifier on Android");
                assert!(downcast_widget::<TextMagnifier>(&*built).is_some());
            });
        }

        #[test]
        fn should_return_a_cupertino_magnifier_on_ios() {
            test_widgets(|_tester| {
                let built = built_by_adaptive_configuration(TargetPlatform::IOS)
                    .expect("a magnifier on iOS");
                assert!(downcast_widget::<CupertinoTextMagnifier>(&*built).is_some());
            });
        }

        #[test]
        fn should_return_null_on_all_platforms_not_android_ios() {
            test_widgets(|_tester| {
                for platform in [
                    TargetPlatform::Fuchsia,
                    TargetPlatform::Linux,
                    TargetPlatform::MacOS,
                    TargetPlatform::Windows,
                ] {
                    assert!(
                        built_by_adaptive_configuration(platform).is_none(),
                        "{platform:?}"
                    );
                }
            });
        }

        #[test]
        fn shows_the_editing_handles_only_on_ios() {
            test_widgets(|_tester| {
                for platform in [
                    TargetPlatform::Android,
                    TargetPlatform::Fuchsia,
                    TargetPlatform::IOS,
                    TargetPlatform::Linux,
                    TargetPlatform::MacOS,
                    TargetPlatform::Windows,
                ] {
                    let tester = WidgetTester::on(platform);
                    let configuration =
                        TextMagnifier::adaptive_magnifier_configuration(&tester.cell.borrow());
                    assert_eq!(
                        configuration.should_display_handles_in_magnifier,
                        platform == TargetPlatform::IOS,
                        "{platform:?}"
                    );
                }
            });
        }

        #[test]
        fn keeps_one_builder_identity_across_calls() {
            test_widgets(|tester| {
                let app = tester.cell.borrow();
                let first =
                    TextMagnifier::adaptive_magnifier_configuration(&app).magnifier_builder();
                let second =
                    TextMagnifier::adaptive_magnifier_configuration(&app).magnifier_builder();
                assert!(Rc::ptr_eq(&first, &second));
            });
        }
    }

    mod position {
        use super::*;

        #[test]
        fn should_never_move_outside_the_right_bounds_of_the_editing_line() {
            test_widgets(|tester| {
                const GESTURE_OUTSIDE_LINE: f64 = 100.0;
                let context = app_context(tester);

                let (controller, _) = show_magnifier(
                    tester,
                    context,
                    info(
                        REASONABLE_TEXT_FIELD,
                        // Inflate these two to make sure we're bounding on the
                        // current line boundaries, not anything else.
                        REASONABLE_TEXT_FIELD.inflate(GESTURE_OUTSIDE_LINE),
                        REASONABLE_TEXT_FIELD.inflate(GESTURE_OUTSIDE_LINE),
                        // The tap position is far out of the right side of the app.
                        Offset::new(REASONABLE_TEXT_FIELD.right + GESTURE_OUTSIDE_LINE, 0.0),
                    ),
                );

                // Should be less than the right edge, since we have padding.
                assert!(magnifier_position(tester).dx() <= REASONABLE_TEXT_FIELD.right);
                controller.remove_from_overlay(&mut tester.cell.borrow_mut());
            });
        }

        #[test]
        fn should_never_move_outside_the_left_bounds_of_the_editing_line() {
            test_widgets(|tester| {
                const GESTURE_OUTSIDE_LINE: f64 = 100.0;
                let context = app_context(tester);

                let (controller, _) = show_magnifier(
                    tester,
                    context,
                    info(
                        REASONABLE_TEXT_FIELD,
                        REASONABLE_TEXT_FIELD.inflate(GESTURE_OUTSIDE_LINE),
                        REASONABLE_TEXT_FIELD.inflate(GESTURE_OUTSIDE_LINE),
                        // The tap position is far out of the left side of the app.
                        Offset::new(REASONABLE_TEXT_FIELD.left - GESTURE_OUTSIDE_LINE, 0.0),
                    ),
                );

                assert!(
                    magnifier_position(tester).dx() + BASIC_OFFSET.dx()
                        >= REASONABLE_TEXT_FIELD.left
                );
                controller.remove_from_overlay(&mut tester.cell.borrow_mut());
            });
        }

        #[test]
        fn should_position_vertically_at_the_center_of_the_line() {
            test_widgets(|tester| {
                let context = app_context(tester);

                let (controller, _) = show_magnifier(
                    tester,
                    context,
                    info_over(REASONABLE_TEXT_FIELD, REASONABLE_TEXT_FIELD.center()),
                );

                assert_eq!(
                    magnifier_position(tester).dy(),
                    REASONABLE_TEXT_FIELD.center().dy() - BASIC_OFFSET.dy()
                );
                controller.remove_from_overlay(&mut tester.cell.borrow_mut());
            });
        }

        #[test]
        fn should_reposition_vertically_if_mashed_against_the_ceiling() {
            test_widgets(|tester| {
                let top_of_screen_text_field =
                    Rect::from_points(Offset::ZERO, Offset::new(200.0, 0.0));
                let context = app_context(tester);

                let (controller, _) = show_magnifier(
                    tester,
                    context,
                    info_over(
                        top_of_screen_text_field,
                        top_of_screen_text_field.top_center(),
                    ),
                );

                assert!(magnifier_position(tester).dy() >= 0.0);
                controller.remove_from_overlay(&mut tester.cell.borrow_mut());
            });
        }
    }

    mod focal_point {
        use super::*;

        #[test]
        fn should_shift_focal_point_so_that_the_lens_sees_nothing_out_of_bounds() {
            test_widgets(|tester| {
                let context = app_context(tester);

                let (controller, _) = show_magnifier(
                    tester,
                    context,
                    // Gesture on the far right of the magnifier.
                    info_over(REASONABLE_TEXT_FIELD, REASONABLE_TEXT_FIELD.top_left()),
                );

                assert!(magnifier_additional_focal_point(tester).dx() < REASONABLE_TEXT_FIELD.left);
                controller.remove_from_overlay(&mut tester.cell.borrow_mut());
            });
        }

        #[test]
        fn focal_point_should_shift_if_mashed_against_the_top_to_always_point_to_text() {
            test_widgets(|tester| {
                let top_of_screen_text_field =
                    Rect::from_points(Offset::ZERO, Offset::new(200.0, 0.0));
                let context = app_context(tester);

                let (controller, _) = show_magnifier(
                    tester,
                    context,
                    info_over(
                        top_of_screen_text_field,
                        top_of_screen_text_field.top_center(),
                    ),
                );

                assert!(magnifier_additional_focal_point(tester).dy() < 0.0);
                controller.remove_from_overlay(&mut tester.cell.borrow_mut());
            });
        }
    }

    mod animation_state {
        use super::*;

        const VERTICAL_SHIFT: Offset = Offset::new(0.0, 200.0);

        /// The info of a gesture that went down a line from `REASONABLE_TEXT_FIELD`.
        fn info_one_line_down() -> MagnifierInfo {
            info(
                REASONABLE_TEXT_FIELD.shift(VERTICAL_SHIFT),
                Rect::from_points(
                    REASONABLE_TEXT_FIELD.top_left(),
                    REASONABLE_TEXT_FIELD.bottom_right() + VERTICAL_SHIFT,
                ),
                REASONABLE_TEXT_FIELD.shift(VERTICAL_SHIFT),
                REASONABLE_TEXT_FIELD.center() + VERTICAL_SHIFT,
            )
        }

        #[test]
        fn should_not_be_animated_on_the_initial_state() {
            test_widgets(|tester| {
                let context = app_context(tester);

                let (controller, _) = show_magnifier(
                    tester,
                    context,
                    info_over(REASONABLE_TEXT_FIELD, REASONABLE_TEXT_FIELD.center()),
                );

                assert!(!is_animated(tester));
                controller.remove_from_overlay(&mut tester.cell.borrow_mut());
            });
        }

        #[test]
        fn should_not_be_animated_on_horizontal_shifts() {
            test_widgets(|tester| {
                let context = app_context(tester);
                let (controller, notifier) = show_magnifier(
                    tester,
                    context,
                    info_over(REASONABLE_TEXT_FIELD, REASONABLE_TEXT_FIELD.center()),
                );

                // New position has a horizontal shift.
                notifier.set_value(
                    &mut tester.cell.borrow_mut(),
                    info_over(
                        REASONABLE_TEXT_FIELD,
                        REASONABLE_TEXT_FIELD.center() + Offset::new(200.0, 0.0),
                    ),
                );
                tester.pump_and_settle();

                assert!(!is_animated(tester));
                controller.remove_from_overlay(&mut tester.cell.borrow_mut());
            });
        }

        #[test]
        fn should_be_animated_on_vertical_shifts() {
            test_widgets(|tester| {
                let context = app_context(tester);
                let (controller, notifier) = show_magnifier(
                    tester,
                    context,
                    info_over(REASONABLE_TEXT_FIELD, REASONABLE_TEXT_FIELD.center()),
                );

                // New position has a vertical shift.
                notifier.set_value(&mut tester.cell.borrow_mut(), info_one_line_down());

                tester.pump();
                assert!(is_animated(tester));
                controller.remove_from_overlay(&mut tester.cell.borrow_mut());
            });
        }

        #[test]
        fn should_stop_being_animated_when_timer_is_up() {
            test_widgets(|tester| {
                let context = app_context(tester);
                let (controller, notifier) = show_magnifier(
                    tester,
                    context,
                    info_over(REASONABLE_TEXT_FIELD, REASONABLE_TEXT_FIELD.center()),
                );

                // New position has a vertical shift.
                notifier.set_value(&mut tester.cell.borrow_mut(), info_one_line_down());

                tester.pump();
                assert!(is_animated(tester));
                tester.pump_by(
                    TextMagnifier::JUMP_BETWEEN_LINES_ANIMATION_DURATION + Duration::from_secs(2),
                );
                assert!(!is_animated(tester));
                controller.remove_from_overlay(&mut tester.cell.borrow_mut());
            });
        }
    }

    mod material_magnifier {
        use waft_widgets::RawMagnifier;

        use super::*;

        #[test]
        fn builds_a_raw_magnifier_in_the_material_style_by_default() {
            test_widgets(|tester| {
                let _context = app_context(tester);
                let root = {
                    let app = tester.cell.borrow();
                    MaterialApp::new()
                        .theme(ThemeData::new(&app))
                        .home(Magnifier::new())
                        .into_widget()
                };
                tester.pump_widget(root);

                let raw = find::by_type::<RawMagnifier>().evaluate(tester)[0];
                tester.widget::<RawMagnifier, _>(raw, |raw| {
                    assert_eq!(raw.size, Magnifier::K_DEFAULT_MAGNIFIER_SIZE);
                    assert_eq!(raw.size, Size::new(77.37, 37.9));
                    assert_eq!(raw.clip_behavior, Clip::HardEdge);
                    assert_eq!(raw.magnification_scale, 1.25);
                    assert_eq!(raw.focal_point_offset, Offset::new(0.0, 22.0 + 37.9 / 2.0));
                    assert_eq!(
                        raw.decoration.shadows,
                        Some(vec![BoxShadow::new(
                            Color::from_argb(25, 0, 0, 0),
                            Offset::new(0.0, 2.0),
                            1.5,
                            0.75,
                            BlurStyle::Normal,
                        )])
                    );
                    let expected_shape = RoundedRectangleBorder::new(
                        BorderSide::NONE,
                        BorderRadius::all(Radius::circular(40.0)).into(),
                    );
                    assert!(raw.decoration.shape.eq_shape(&expected_shape));
                });
            });
        }

        #[test]
        fn adds_the_additional_focal_point_offset_to_the_standard_shift() {
            test_widgets(|tester| {
                let _context = app_context(tester);
                let root = {
                    let app = tester.cell.borrow();
                    MaterialApp::new()
                        .theme(ThemeData::new(&app))
                        .home(
                            Magnifier::new().additional_focal_point_offset(Offset::new(5.0, -3.0)),
                        )
                        .into_widget()
                };
                tester.pump_widget(root);

                let raw = find::by_type::<RawMagnifier>().evaluate(tester)[0];
                let focal_point_offset =
                    tester.widget::<RawMagnifier, _>(raw, |raw| raw.focal_point_offset);
                assert_eq!(
                    focal_point_offset,
                    Offset::new(5.0, 22.0 + 37.9 / 2.0 - 3.0)
                );
            });
        }

        #[test]
        fn sizes_the_lens_to_zero_area_when_given_a_zero_size() {
            test_widgets(|tester| {
                let _context = app_context(tester);
                let root = {
                    let app = tester.cell.borrow();
                    MaterialApp::new()
                        .theme(ThemeData::new(&app))
                        .home(Magnifier::new().size(Size::ZERO))
                        .into_widget()
                };
                tester.pump_widget(root);

                let raw = find::by_type::<RawMagnifier>().evaluate(tester)[0];
                assert_eq!(
                    tester.widget::<RawMagnifier, _>(raw, |raw| raw.size),
                    Size::ZERO
                );
            });
        }
    }
}
