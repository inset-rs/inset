//! Flutter counterpart: material_ui's `input_decorator.dart`, with its part
//! `generated/input_decorator_defaults_m3.g.dart` in
//! `crate::generated::input_decorator_defaults_m3`.
//!
//! `InputDecorator` builds a `_Decorator` whose `RenderDecoration` lays out the eleven slots;
//! the `_BorderContainer` paints the border with the gap the render object writes into the
//! state's `InputBorderGap` during layout. Both defaults (`_InputDecoratorDefaultsM2`/`M3`)
//! are values filled from the context's theme when the decorator builds.

use std::any::Any;
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animatable, Animation, AnimationBehavior, AnimationController, AnimationRef,
    AnimationWithParent, ColorTween, Curve, CurvedAnimation, Curves, Drive, Tween,
};
use waft_embedder::{
    Brightness, Canvas, Color, Matrix4, Offset, Paint, PaintStyle, Size, TextBaseline, lerp_double,
};
use waft_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, Listenable, Listener, MergingListenable,
    OptionRetained, Ref,
};
use waft_painting::{
    AnyBorderSide, AnyColor, AnyTextStyle, BorderSide, EdgeInsets, EdgeInsetsDirectional,
    EdgeInsetsGeometry, ShapeBorderRef, TextAlign, TextAlignVertical, TextDirection, TextOverflow,
    TextStyle,
};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, BoxParentData,
    ChildLayoutHelper, ChildLayouter, ContainerLayer, CustomPainter, PaintingContext,
    PipelineOwner, RenderBox, RenderBoxData, RenderHandle, RenderObject, RenderObjectData,
    TransformLayer,
};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_services::SystemMouseCursors;
use waft_widgets::{
    AnimatedDefaultTextStyle, AnimatedOpacity, AnimatedSwitcher, AnimatedSwitcherTransitionBuilder,
    BuildContext, Builder, Center, ConstrainedBox, CustomPaint, DefaultTextStyle, Directionality,
    FadeTransition, FractionalTranslation, IconTheme, IconThemeData, IgnorePointer, InheritedTheme,
    InheritedWidget, IntoWidget, KeyRef, MatrixTransition, MediaQuery, MouseRegion, Padding,
    RenderObjectWidget, SingleTickerProviderStateMixin, SingleTickerProviderStateMixinData,
    SizedBox, SlottedContainerRenderObjectData, SlottedContainerRenderObjectMixin,
    SlottedMultiChildRenderObjectWidget, Stack, State, StateData, StatefulWidget, StatelessWidget,
    Text, TickerProviderStateMixin, TickerProviderStateMixinData, WidgetRef, WidgetState,
    WidgetStateColor, WidgetStateProperty, WidgetStatePropertyAll, WidgetStatePropertyRef,
    WidgetStateTextStyle, WidgetStates,
};

use crate::button_style::ButtonStyle;
use crate::colors::Colors;
use crate::constants::K_MIN_INTERACTIVE_DIMENSION;
use crate::generated::input_decorator_defaults_m3::InputDecoratorDefaultsM3;
use crate::icon_button_theme::{IconButtonTheme, IconButtonThemeData};
use crate::input_border::{
    InputBorder, InputBorderRef, OutlineInputBorder, UnderlineInputBorder, input_border_of,
};
use crate::theme::Theme;
use crate::theme_data::{ThemeData, VisualDensity};
use crate::theme_data_getters::DefaultsContext;

// The duration value extracted from:
// https://github.com/material-components/material-components-android/blob/master/lib/java/com/google/android/material/textfield/TextInputLayout.java
const K_TRANSITION_DURATION: Duration = Duration::from_millis(167);
fn k_transition_curve() -> Rc<dyn Curve> {
    Curves::fast_out_slow_in()
}
const K_FINAL_LABEL_SCALE: f64 = 0.75;

// From the M3 spec, horizontal padding is 12 pixels for the prefix icon and
// 16 pixels for the input content.
// InputDecorator default padding is set to 12 pixels because 16 pixels will move
// the prefix icon too far.
// An extra padding should be added for the input content to comply with the 16 pixels padding.
const K_INPUT_EXTRA_PADDING: f64 = 4.0;

// Padding between the character counter and helper/error text to prevent overlap.
// Based on Material 3 specification for text fields.
const K_SUPPORTING_TEXT_COUNTER_PADDING: f64 = 16.0;

/// Dart's `_SupportingTextSize` record.
#[derive(Clone, Copy, Debug)]
struct SupportingTextSize {
    ascent: f64,
    bottom_height: f64,
    supporting_text_height: f64,
}

/// Dart's `_ChildBaselineGetter`.
type ChildBaselineGetter = fn(&mut App, AnyRenderBox, BoxConstraints) -> f64;

// The default duration for hint fade in/out transitions.
//
// Animating hint is not mentioned in the Material specification.
// The animation is kept for backward compatibility and a short duration
// is used to mitigate the UX impact.
const K_HINT_FADE_TRANSITION_DURATION: Duration = Duration::from_millis(20);

// Defines the gap in the InputDecorator's outline border where the
// floating label will appear.
struct InputBorderGap {
    change_notifier: ChangeNotifierData,
    start: Option<f64>,
    extent: f64,
}

impl InputBorderGap {
    fn new(app: &mut App) -> Ref<InputBorderGap> {
        app.create(InputBorderGap {
            change_notifier: ChangeNotifierData::default(),
            start: None,
            extent: 0.0,
        })
    }

    fn start(self: Handle<Self>, app: &App) -> Option<f64> {
        app.get(self).start
    }

    fn set_start(self: Handle<Self>, app: &mut App, value: Option<f64>) {
        if value != app.get(self).start {
            app.get_mut(self).start = value;
            self.notify_listeners(app);
        }
    }

    fn extent(self: Handle<Self>, app: &App) -> f64 {
        app.get(self).extent
    }

    fn set_extent(self: Handle<Self>, app: &mut App, value: f64) {
        if value != app.get(self).extent {
            app.get_mut(self).extent = value;
            self.notify_listeners(app);
        }
    }
}

impl ChangeNotifier for InputBorderGap {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl Debug for InputBorderGap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("_InputBorderGap")
    }
}

// Used to interpolate between two InputBorders.
//
// A value rather than an arena object, as material.rs's `ShapeBorderTween`; the painter holds
// it in an `Rc`, whose identity is Dart's `Tween ==`.
#[derive(Clone, Debug)]
struct InputBorderTween {
    begin: InputBorderRef,
    end: InputBorderRef,
}

impl InputBorderTween {
    fn new(begin: InputBorderRef, end: InputBorderRef) -> InputBorderTween {
        InputBorderTween { begin, end }
    }

    /// Dart's `ShapeBorder.lerp(begin, end, t)! as InputBorder`.
    fn lerp(&self, t: f64) -> InputBorderRef {
        let begin = ShapeBorderRef::from(self.begin.clone());
        let end = ShapeBorderRef::from(self.end.clone());
        let lerped = ShapeBorderRef::lerp(Some(&begin), Some(&end), t)
            .expect("lerping two borders gives a border");
        input_border_of(&*lerped).unwrap_or_else(|| {
            if t < 0.5 {
                self.begin.clone()
            } else {
                self.end.clone()
            }
        })
    }
}

/// Dart's `Tween.transform`, which hands back the endpoints themselves at 0 and 1.
impl Animatable<InputBorderRef> for InputBorderTween {
    fn transform(&self, _app: &App, t: f64) -> InputBorderRef {
        if t == 0.0 {
            return self.begin.clone();
        }
        if t == 1.0 {
            return self.end.clone();
        }
        self.lerp(t)
    }
}

// Passes the _InputBorderGap parameters along to an InputBorder's paint method.
struct InputBorderPainter {
    repaint: Rc<dyn Listenable>,
    border_animation: AnimationRef<f64>,
    border: Rc<InputBorderTween>,
    gap_animation: AnimationRef<f64>,
    gap: Ref<InputBorderGap>,
    text_direction: TextDirection,
    fill_color: Color,
    hover_color_tween: Ref<ColorTween>,
    hover_animation: AnimationRef<f64>,
}

impl InputBorderPainter {
    fn blended_color(&self, app: &App) -> Color {
        Color::alpha_blend(
            self.hover_color_tween
                .evaluate(app, self.hover_animation.handle())
                .expect("both colours are given"),
            self.fill_color,
        )
    }
}

impl CustomPainter for InputBorderPainter {
    fn repaint(&self) -> Option<&Rc<dyn Listenable>> {
        Some(&self.repaint)
    }

    fn paint(&self, app: &mut App, canvas: &mut Canvas, size: Size) {
        let border_value = self.border.evaluate(app, self.border_animation.handle());
        let canvas_rect = Offset::ZERO & size;
        let blended_fill_color = self.blended_color(app);
        // Dart's `alpha`, deprecated there too.
        #[allow(deprecated)]
        let alpha = blended_fill_color.alpha();
        if alpha > 0 {
            let paint = Paint {
                color: blended_fill_color.into(),
                style: PaintStyle::Fill,
                ..Paint::default()
            };
            if border_value.prefer_paint_interior() {
                border_value.paint_interior(canvas, canvas_rect, &paint, Some(self.text_direction));
            } else {
                let path = border_value.get_outer_path(canvas_rect, Some(self.text_direction));
                canvas.draw_path(path.as_valo(), path.fill_type, &paint);
            }
        }

        InputBorder::paint(
            &*border_value,
            canvas,
            canvas_rect,
            self.gap.start(app),
            self.gap.extent(app),
            self.gap_animation.value(app),
            Some(self.text_direction),
        );
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        let Some(old_painter) = old_delegate.as_any().downcast_ref::<InputBorderPainter>() else {
            return true;
        };
        self.border_animation != old_painter.border_animation
            || self.hover_animation != old_painter.hover_animation
            || self.gap_animation != old_painter.gap_animation
            || !Rc::ptr_eq(&self.border, &old_painter.border)
            || self.gap != old_painter.gap
            || self.text_direction != old_painter.text_direction
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Debug for InputBorderPainter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("_InputBorderPainter")
    }
}

// An analog of AnimatedContainer, which can animate its shaped border, for
// _InputBorder. This specialized animated container is needed because the
// _InputBorderGap, which is computed at layout time, is required by the
// _InputBorder's paint method.
#[derive(Debug)]
struct BorderContainer {
    border: InputBorderRef,
    gap: Ref<InputBorderGap>,
    gap_animation: AnimationRef<f64>,
    fill_color: Color,
    hover_color: Color,
    is_hovering: bool,
}

impl StatefulWidget for BorderContainer {
    type State = BorderContainerState;

    fn key(&self) -> Option<&KeyRef> {
        None
    }

    fn create_state(&self) -> BorderContainerState {
        BorderContainerState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            controller: None,
            hover_color_controller: None,
            border_animation: None,
            border: None,
            hover_animation: None,
            hover_color_tween: None,
        }
    }
}

/// Dart's `_BorderContainerState`. Its `late` fields are set in `init_state`, because an
/// arena object needs the [`App`].
struct BorderContainerState {
    state: StateData<BorderContainer>,
    ticker_provider: TickerProviderStateMixinData,
    controller: Option<Ref<AnimationController>>,
    hover_color_controller: Option<Ref<AnimationController>>,
    border_animation: Option<Ref<CurvedAnimation>>,
    border: Option<Rc<InputBorderTween>>,
    hover_animation: Option<Ref<CurvedAnimation>>,
    hover_color_tween: Option<Ref<ColorTween>>,
}

impl BorderContainerState {
    const K_HOVER_DURATION: Duration = Duration::from_millis(15);

    fn controller(self: Handle<Self>, app: &App) -> Ref<AnimationController> {
        app.get(self)
            .controller
            .clone()
            .expect("created in init_state")
    }

    fn hover_color_controller(self: Handle<Self>, app: &App) -> Ref<AnimationController> {
        app.get(self)
            .hover_color_controller
            .clone()
            .expect("created in init_state")
    }

    fn border_animation(self: Handle<Self>, app: &App) -> Ref<CurvedAnimation> {
        app.get(self)
            .border_animation
            .clone()
            .expect("created in init_state")
    }

    fn hover_animation(self: Handle<Self>, app: &App) -> Ref<CurvedAnimation> {
        app.get(self)
            .hover_animation
            .clone()
            .expect("created in init_state")
    }

    fn hover_color_tween(self: Handle<Self>, app: &App) -> Ref<ColorTween> {
        app.get(self)
            .hover_color_tween
            .clone()
            .expect("created in init_state")
    }

    /// Dart's `ColorTween(begin: hoverColor.withAlpha(0), end: hoverColor)`: animate between
    /// transparent [`BorderContainer::hover_color`] and [`BorderContainer::hover_color`].
    fn new_hover_color_tween(app: &mut App, hover_color: Color) -> Ref<ColorTween> {
        ColorTween::new(app, Some(hover_color.with_alpha(0)), Some(hover_color))
    }
}

impl TickerProviderStateMixin for BorderContainerState {
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

/// Dart's `vsync: this`.
impl TickerProviderObject for BorderContainerState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for BorderContainerState {
    type Widget = BorderContainer;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let is_hovering = widget.is_hovering;
        let border = widget.border.clone();
        let hover_color = widget.hover_color;
        let hover_color_controller = AnimationController::new(
            app,
            Some(if is_hovering { 1.0 } else { 0.0 }),
            Some(BorderContainerState::K_HOVER_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let controller = AnimationController::new(
            app,
            None,
            Some(K_TRANSITION_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let border_animation = CurvedAnimation::new(
            app,
            controller.erase(),
            k_transition_curve(),
            Some(Rc::new(k_transition_curve().flipped())),
        );
        let border_tween = Rc::new(InputBorderTween::new(border.clone(), border));
        let hover_animation =
            CurvedAnimation::new(app, hover_color_controller.erase(), Curves::linear(), None);

        // Animate between transparent [widget.hoverColor] and [widget.hoverColor].
        let hover_color_tween = BorderContainerState::new_hover_color_tween(app, hover_color);
        let state = app.get_mut(self);
        state.hover_color_controller = Some(hover_color_controller);
        state.controller = Some(controller);
        state.border_animation = Some(border_animation);
        state.border = Some(border_tween);
        state.hover_animation = Some(hover_animation);
        state.hover_color_tween = Some(hover_color_tween);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        let hover_color_controller = self.hover_color_controller(app);
        let border_animation = self.border_animation(app);
        let hover_animation = self.hover_animation(app);
        controller.dispose(app);
        hover_color_controller.dispose(app);
        border_animation.dispose(app);
        hover_animation.dispose(app);
        TickerProviderStateMixin::dispose(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &BorderContainer) {
        let widget = self.widget(app);
        let border = widget.border.clone();
        let hover_color = widget.hover_color;
        let is_hovering = widget.is_hovering;
        if *border != *old_widget.border {
            app.get_mut(self).border = Some(Rc::new(InputBorderTween::new(
                old_widget.border.clone(),
                border,
            )));
            let controller = self.controller(app);
            controller.set_value(app, 0.0);
            controller.forward(app, None);
        }
        if hover_color != old_widget.hover_color {
            // Animate between transparent [widget.hoverColor] and [widget.hoverColor].
            let hover_color_tween = BorderContainerState::new_hover_color_tween(app, hover_color);
            app.get_mut(self).hover_color_tween = Some(hover_color_tween);
        }
        if is_hovering != old_widget.is_hovering {
            let hover_color_controller = self.hover_color_controller(app);
            if is_hovering {
                hover_color_controller.forward(app, None);
            } else {
                hover_color_controller.reverse(app, None);
            }
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let border_animation: AnimationRef<f64> = self.border_animation(app).erase();
        let hover_color_controller = self.hover_color_controller(app);
        let widget = self.widget(app);
        let gap = widget.gap.clone();
        let gap_animation = widget.gap_animation.clone();
        let fill_color = widget.fill_color;
        let border = app.get(self).border.clone().expect("created in init_state");
        let hover_color_tween = self.hover_color_tween(app);
        let hover_animation: AnimationRef<f64> = self.hover_animation(app).erase();
        let text_direction = Directionality::of(app, context);
        let repaint: Rc<dyn Listenable> = Rc::new(MergingListenable::new(vec![
            Some(Rc::new(border_animation.clone()) as Rc<dyn Listenable>),
            Some(Rc::new(gap.clone()) as Rc<dyn Listenable>),
            Some(Rc::new(hover_color_controller) as Rc<dyn Listenable>),
        ]));
        CustomPaint::new()
            .foreground_painter(InputBorderPainter {
                repaint,
                border_animation,
                border,
                gap_animation,
                gap,
                text_direction,
                fill_color,
                hover_color_tween,
                hover_animation,
            })
            .into_widget()
    }
}

impl Debug for BorderContainerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_BorderContainerState")
            .field("controller", &self.controller)
            .finish_non_exhaustive()
    }
}

// Display the helper and error text. When the error text appears
// it fades and the helper text fades out. The error text also
// slides upwards a little when it first appears.
#[derive(Debug)]
struct HelperError {
    text_align: Option<TextAlign>,
    helper: Option<WidgetRef>,
    helper_text: Option<String>,
    helper_style: Option<TextStyle>,
    helper_max_lines: Option<i32>,
    error: Option<WidgetRef>,
    error_text: Option<String>,
    error_style: Option<TextStyle>,
    error_max_lines: Option<i32>,
}

impl StatefulWidget for HelperError {
    type State = HelperErrorState;

    fn key(&self) -> Option<&KeyRef> {
        None
    }

    fn create_state(&self) -> HelperErrorState {
        HelperErrorState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::default(),
            controller: None,
            helper: None,
            error: None,
        }
    }
}

/// Dart's `_HelperErrorState`.
struct HelperErrorState {
    state: StateData<HelperError>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    /// Created in `init_state`, because an arena object needs the [`App`].
    controller: Option<Ref<AnimationController>>,
    helper: Option<WidgetRef>,
    error: Option<WidgetRef>,
}

impl HelperErrorState {
    fn controller(self: Handle<Self>, app: &App) -> Ref<AnimationController> {
        app.get(self)
            .controller
            .clone()
            .expect("created in init_state")
    }

    /// Dart's `Tween<double>(begin: 1.0, end: 0.0).animate(_controller)`.
    fn fade_out(self: Handle<Self>, app: &mut App) -> AnimationRef<f64> {
        let controller = self.controller(app);
        let tween = Tween::new(app, Some(1.0), Some(0.0));
        controller.drive(app, tween)
    }

    fn has_helper(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        widget.helper_text.is_some() || widget.helper.is_some()
    }

    fn has_error(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        widget.error_text.is_some() || widget.error.is_some()
    }

    fn handle_change(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_state| {
            // The _controller's value has changed.
        });
    }

    fn build_helper(self: Handle<Self>, app: &mut App) -> WidgetRef {
        debug_assert!(self.widget(app).helper.is_some() || self.widget(app).helper_text.is_some());
        let fade_out = self.fade_out(app);
        let widget = self.widget(app);
        let child = match &widget.helper {
            Some(helper) => helper.clone(),
            None => {
                let mut text = Text::new(
                    widget
                        .helper_text
                        .clone()
                        .expect("a helper or its text is given"),
                )
                .overflow(TextOverflow::Ellipsis);
                if let Some(style) = &widget.helper_style {
                    text = text.style(style.clone());
                }
                if let Some(text_align) = widget.text_align {
                    text = text.text_align(text_align);
                }
                if let Some(max_lines) = widget.helper_max_lines {
                    text = text.max_lines(max_lines);
                }
                text.into_widget()
            }
        };
        FadeTransition::new(fade_out).child(child).into_widget()
    }

    fn build_error(self: Handle<Self>, app: &mut App) -> WidgetRef {
        debug_assert!(self.widget(app).error.is_some() || self.widget(app).error_text.is_some());
        let controller = self.controller(app);
        let widget = self.widget(app);
        let captured_error_text = widget.error_text.clone();
        let mut captured_error = widget.error.clone();
        if let (Some(error), Some(error_style)) = (&captured_error, &widget.error_style) {
            captured_error =
                Some(DefaultTextStyle::new(error_style.clone(), error.clone()).into_widget());
        }
        let this = self;
        Builder::new(move |app, context| {
            // `Semantics(liveRegion:)` waits with accessibility; the read keeps the dependency
            // on `MediaQuery` Dart registers here.
            MediaQuery::supports_announce_of(app, context);
            let child = match &captured_error {
                Some(error) => error.clone(),
                None => {
                    let widget = this.widget(app);
                    let error_style = widget.error_style.clone();
                    let text_align = widget.text_align;
                    let error_max_lines = widget.error_max_lines;
                    let mut text = Text::new(
                        captured_error_text
                            .clone()
                            .expect("an error or its text is given"),
                    )
                    .overflow(TextOverflow::Ellipsis);
                    if let Some(style) = &error_style {
                        text = text.style(style.clone());
                    }
                    if let Some(text_align) = text_align {
                        text = text.text_align(text_align);
                    }
                    if let Some(max_lines) = error_max_lines {
                        text = text.max_lines(max_lines);
                    }
                    text.into_widget()
                }
            };
            // Dart's `Tween<Offset>(begin: Offset(0.0, -0.25), end: Offset.zero)
            // .evaluate(_controller.view)`.
            let begin = Offset::new(0.0, -0.25);
            let translation = begin + (Offset::ZERO - begin) * controller.value(app);
            FadeTransition::new(controller.erase())
                .child(FractionalTranslation::new(translation).child(child))
                .into_widget()
        })
        .into_widget()
    }
}

impl SingleTickerProviderStateMixin for HelperErrorState {
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
impl TickerProviderObject for HelperErrorState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for HelperErrorState {
    type Widget = HelperError;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = AnimationController::new(
            app,
            None,
            Some(K_TRANSITION_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        app.get_mut(self).controller = Some(controller.clone());
        if self.has_error(app) {
            let error = self.build_error(app);
            app.get_mut(self).error = Some(error);
            controller.set_value(app, 1.0);
        } else if self.has_helper(app) {
            let helper = self.build_helper(app);
            app.get_mut(self).helper = Some(helper);
        }
        controller.add_listener(app, Listener::handle_method(self, Self::handle_change));
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        controller.dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &HelperError) {
        let widget = self.widget(app);
        let new_error = widget.error.is_some();
        let new_error_text = widget.error_text.is_some();
        let new_helper = widget.helper.is_some();
        let new_helper_text = widget.helper_text.is_some();
        let old_error = old.error.is_some();
        let old_error_text = old.error_text.is_some();
        let old_helper = old.helper.is_some();
        let old_helper_text = old.helper_text.is_some();

        let error_state_changed = new_error != old_error;
        let error_text_state_changed = new_error_text != old_error_text;
        let helper_state_changed = new_helper != old_helper;
        let helper_text_state_changed = !new_error_text && new_helper_text != old_helper_text;

        if error_state_changed
            || error_text_state_changed
            || helper_state_changed
            || helper_text_state_changed
        {
            let controller = self.controller(app);
            if new_error || new_error_text {
                let error = self.build_error(app);
                app.get_mut(self).error = Some(error);
                controller.forward(app, None);
            } else if new_helper || new_helper_text {
                let helper = self.build_helper(app);
                app.get_mut(self).helper = Some(helper);
                controller.reverse(app, None);
            } else {
                controller.reverse(app, None);
            }
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        // If the height of this widget and the counter are zero ("empty") at
        // layout time, no space is allocated for the supportingText.
        let empty = || SizedBox::shrink().into_widget();

        let controller = self.controller(app);
        if controller.is_dismissed(app) {
            app.get_mut(self).error = None;
            if self.has_helper(app) {
                let helper = self.build_helper(app);
                app.get_mut(self).helper = Some(helper.clone());
                return helper;
            } else {
                app.get_mut(self).helper = None;
                return empty();
            }
        }

        if controller.is_completed(app) {
            app.get_mut(self).helper = None;
            if self.has_error(app) {
                let error = self.build_error(app);
                app.get_mut(self).error = Some(error.clone());
                return error;
            } else {
                app.get_mut(self).error = None;
                return empty();
            }
        }

        if app.get(self).helper.is_none() && self.has_error(app) {
            return self.build_error(app);
        }

        if app.get(self).error.is_none() && self.has_helper(app) {
            return self.build_helper(app);
        }

        if self.has_error(app) {
            let fade_out = self.fade_out(app);
            let mut helper_transition = FadeTransition::new(fade_out);
            if let Some(helper) = app.get(self).helper.clone() {
                helper_transition = helper_transition.child(helper);
            }
            let error = self.build_error(app);
            return Stack::new()
                .children([helper_transition.into_widget(), error])
                .into_widget();
        }

        if self.has_helper(app) {
            let helper = self.build_helper(app);
            let mut error_transition = FadeTransition::new(controller.erase());
            if let Some(error) = app.get(self).error.clone() {
                error_transition = error_transition.child(error);
            }
            return Stack::new()
                .children([helper, error_transition.into_widget()])
                .into_widget();
        }

        empty()
    }
}

impl Debug for HelperErrorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_HelperErrorState")
            .field("controller", &self.controller)
            .finish_non_exhaustive()
    }
}

/// Defines **how** the floating label should behave.
///
/// See also:
///
///  * `InputDecoration::floating_label_behavior` which defines the behavior for
///    `InputDecoration::label` or `InputDecoration::label_text`.
///  * [`FloatingLabelAlignment`] which defines **where** the floating label
///    should displayed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FloatingLabelBehavior {
    /// The label will always be positioned within the content, or hidden.
    Never,

    /// The label will float when the input is focused, or has content.
    Auto,

    /// The label will always float above the content.
    Always,
}

/// Defines **where** the floating label should be displayed within an
/// `InputDecorator`.
///
/// See also:
///
///  * `InputDecoration::floating_label_alignment` which defines the alignment for
///    `InputDecoration::label` or `InputDecoration::label_text`.
///  * [`FloatingLabelBehavior`] which defines **how** the floating label should
///    behave.
#[derive(Clone, Copy, PartialEq)]
pub struct FloatingLabelAlignment {
    // -1 denotes start, 0 denotes center, and 1 denotes end.
    x: f64,
}

impl FloatingLabelAlignment {
    const fn private(x: f64) -> FloatingLabelAlignment {
        assert!(x >= -1.0 && x <= 1.0);
        FloatingLabelAlignment { x }
    }

    /// Align the floating label on the leading edge of the `InputDecorator`.
    ///
    /// For left-to-right text ([`TextDirection::Ltr`]), this is the left edge.
    ///
    /// For right-to-left text ([`TextDirection::Rtl`]), this is the right edge.
    pub const START: FloatingLabelAlignment = FloatingLabelAlignment::private(-1.0);

    /// Aligns the floating label to the center of an `InputDecorator`.
    pub const CENTER: FloatingLabelAlignment = FloatingLabelAlignment::private(0.0);

    fn stringify(x: f64) -> String {
        if x == -1.0 {
            "FloatingLabelAlignment.start".to_owned()
        } else if x == 0.0 {
            "FloatingLabelAlignment.center".to_owned()
        } else {
            format!("FloatingLabelAlignment(x: {x:.1})")
        }
    }
}

impl Debug for FloatingLabelAlignment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&FloatingLabelAlignment::stringify(self.x))
    }
}

// Identifies the children of a _RenderDecorationElement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum DecorationSlot {
    Icon,
    Input,
    Label,
    Hint,
    Prefix,
    Suffix,
    PrefixIcon,
    SuffixIcon,
    HelperError,
    Counter,
    Container,
}

impl DecorationSlot {
    /// Dart's `_DecorationSlot.values`.
    const VALUES: [DecorationSlot; 11] = [
        DecorationSlot::Icon,
        DecorationSlot::Input,
        DecorationSlot::Label,
        DecorationSlot::Hint,
        DecorationSlot::Prefix,
        DecorationSlot::Suffix,
        DecorationSlot::PrefixIcon,
        DecorationSlot::SuffixIcon,
        DecorationSlot::HelperError,
        DecorationSlot::Counter,
        DecorationSlot::Container,
    ];
}

/// Dart's `identical` on two optional widgets.
fn same_widget(a: &Option<WidgetRef>, b: &Option<WidgetRef>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

// An analog of InputDecoration for the _Decorator widget.
#[derive(Clone, Debug)]
struct Decoration {
    content_padding: EdgeInsetsDirectional,
    is_collapsed: bool,
    floating_label_height: f64,
    floating_label_progress: f64,
    floating_label_alignment: FloatingLabelAlignment,
    border: InputBorderRef,
    border_gap: Ref<InputBorderGap>,
    align_label_with_hint: bool,
    is_dense: Option<bool>,
    is_empty: bool,
    visual_density: VisualDensity,
    input_gap: f64,
    maintain_hint_size: bool,
    maintain_label_size: bool,
    icon: Option<WidgetRef>,
    input: Option<WidgetRef>,
    label: Option<WidgetRef>,
    hint: Option<WidgetRef>,
    prefix: Option<WidgetRef>,
    suffix: Option<WidgetRef>,
    prefix_icon: Option<WidgetRef>,
    suffix_icon: Option<WidgetRef>,
    helper_error: Option<WidgetRef>,
    counter: Option<WidgetRef>,
    container: Option<WidgetRef>,
    supporting_text_padding: Option<EdgeInsetsDirectional>,
}

impl PartialEq for Decoration {
    // Dart's `==` compares the widgets by their `==`, which is identity.
    fn eq(&self, other: &Decoration) -> bool {
        other.content_padding == self.content_padding
            && other.is_collapsed == self.is_collapsed
            && other.floating_label_height == self.floating_label_height
            && other.floating_label_progress == self.floating_label_progress
            && other.floating_label_alignment == self.floating_label_alignment
            && *other.border == *self.border
            && other.border_gap == self.border_gap
            && other.align_label_with_hint == self.align_label_with_hint
            && other.is_dense == self.is_dense
            && other.is_empty == self.is_empty
            && other.visual_density == self.visual_density
            && other.input_gap == self.input_gap
            && other.maintain_hint_size == self.maintain_hint_size
            && other.maintain_label_size == self.maintain_label_size
            && same_widget(&other.icon, &self.icon)
            && same_widget(&other.input, &self.input)
            && same_widget(&other.label, &self.label)
            && same_widget(&other.hint, &self.hint)
            && same_widget(&other.prefix, &self.prefix)
            && same_widget(&other.suffix, &self.suffix)
            && same_widget(&other.prefix_icon, &self.prefix_icon)
            && same_widget(&other.suffix_icon, &self.suffix_icon)
            && same_widget(&other.helper_error, &self.helper_error)
            && same_widget(&other.counter, &self.counter)
            && same_widget(&other.container, &self.container)
            && other.supporting_text_padding == self.supporting_text_padding
    }
}

// A container for the layout values computed by _RenderDecoration._layout.
// These values are used by _RenderDecoration.performLayout to position
// all of the renderer children of a _RenderDecoration.
#[derive(Clone, Copy, Debug)]
struct RenderDecorationLayout {
    input_constraints: BoxConstraints,
    baseline: f64,
    container_height: f64,
    supporting_text_size: Option<SupportingTextSize>,
    size: Size,
}

// The workhorse: layout and paint a _Decorator widget's _Decoration.
struct RenderDecoration {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    slotted: SlottedContainerRenderObjectData<DecorationSlot, AnyRenderBox>,
    decoration: Decoration,
    text_direction: TextDirection,
    text_baseline: TextBaseline,
    text_align_vertical: Option<TextAlignVertical>,
    is_focused: bool,
    expands: bool,
    material3: bool,
    // Records where the label was painted.
    label_transform: Option<Matrix4>,
}

/// The getter and the setter of one of `_RenderDecoration`'s properties, the setter marking
/// the decoration as needing layout when the value changes. Dart declares both for each
/// property; `textBaseline`'s getter and `material3`'s setter have no caller there either.
macro_rules! render_decoration_property {
    ($($field:ident, $setter:ident: $ty:ty;)*) => {
        $(
            #[allow(dead_code)]
            fn $field(self: RenderHandle<Self>, app: &App) -> $ty {
                self.get(app).$field
            }

            #[allow(dead_code)]
            fn $setter(self: RenderHandle<Self>, app: &mut App, value: $ty) {
                if self.get(app).$field == value {
                    return;
                }
                self.get_mut(app).$field = value;
                self.mark_needs_layout(app);
            }
        )*
    };
}

impl RenderDecoration {
    #[allow(clippy::too_many_arguments)]
    fn new(
        app: &mut App,
        decoration: Decoration,
        text_direction: TextDirection,
        text_baseline: TextBaseline,
        is_focused: bool,
        expands: bool,
        material3: bool,
        text_align_vertical: Option<TextAlignVertical>,
    ) -> RenderHandle<RenderDecoration> {
        RenderHandle::new_box(
            app,
            RenderDecoration {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                slotted: SlottedContainerRenderObjectData::new(),
                decoration,
                text_direction,
                text_baseline,
                text_align_vertical,
                is_focused,
                expands,
                material3,
                label_transform: None,
            },
        )
    }

    // TODO(bleroux): consider defining this value as a Material token and making it
    // configurable by InputDecorationThemeData.
    fn supporting_text_gap(self: RenderHandle<Self>, app: &App) -> f64 {
        if self.material3(app) { 4.0 } else { 8.0 }
    }

    fn prefix_to_input_gap(self: RenderHandle<Self>, app: &App) -> f64 {
        if self.material3(app) { 4.0 } else { 0.0 }
    }

    fn input_to_suffix_gap(self: RenderHandle<Self>, app: &App) -> f64 {
        if self.material3(app) { 4.0 } else { 0.0 }
    }

    fn icon(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderBox> {
        self.child_for_slot(app, &DecorationSlot::Icon)
    }

    fn input(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderBox> {
        self.child_for_slot(app, &DecorationSlot::Input)
    }

    fn label(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderBox> {
        self.child_for_slot(app, &DecorationSlot::Label)
    }

    fn hint(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderBox> {
        self.child_for_slot(app, &DecorationSlot::Hint)
    }

    fn prefix(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderBox> {
        self.child_for_slot(app, &DecorationSlot::Prefix)
    }

    fn suffix(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderBox> {
        self.child_for_slot(app, &DecorationSlot::Suffix)
    }

    fn prefix_icon(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderBox> {
        self.child_for_slot(app, &DecorationSlot::PrefixIcon)
    }

    fn suffix_icon(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderBox> {
        self.child_for_slot(app, &DecorationSlot::SuffixIcon)
    }

    fn helper_error(self: RenderHandle<Self>, app: &App) -> AnyRenderBox {
        self.child_for_slot(app, &DecorationSlot::HelperError)
            .expect("an input decorator always has a helper and error child")
    }

    fn counter(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderBox> {
        self.child_for_slot(app, &DecorationSlot::Counter)
    }

    /// The decoration's "container", the area [`InputDecorator::container_of`] answers.
    fn container(self: RenderHandle<Self>, app: &App) -> Option<AnyRenderBox> {
        self.child_for_slot(app, &DecorationSlot::Container)
    }

    fn decoration(self: RenderHandle<Self>, app: &App) -> &Decoration {
        &self.get(app).decoration
    }

    fn set_decoration(self: RenderHandle<Self>, app: &mut App, value: Decoration) {
        if self.get(app).decoration == value {
            return;
        }
        self.get_mut(app).decoration = value;
        self.mark_needs_layout(app);
    }

    render_decoration_property! {
        text_direction, set_text_direction: TextDirection;
        text_baseline, set_text_baseline: TextBaseline;
        expands, set_expands: bool;
        material3, set_material3: bool;
    }

    fn default_text_align_vertical(self: RenderHandle<Self>, app: &App) -> TextAlignVertical {
        if self.is_outline_aligned(app) {
            TextAlignVertical::CENTER
        } else {
            TextAlignVertical::TOP
        }
    }

    fn text_align_vertical(self: RenderHandle<Self>, app: &App) -> TextAlignVertical {
        self.get(app)
            .text_align_vertical
            .unwrap_or_else(|| self.default_text_align_vertical(app))
    }

    fn set_text_align_vertical(
        self: RenderHandle<Self>,
        app: &mut App,
        value: Option<TextAlignVertical>,
    ) {
        if self.get(app).text_align_vertical.map(|value| value.y) == value.map(|value| value.y) {
            return;
        }
        // No need to relayout if the effective value is still the same.
        if self.text_align_vertical(app).y
            == value.map_or_else(|| self.default_text_align_vertical(app).y, |value| value.y)
        {
            self.get_mut(app).text_align_vertical = value;
            return;
        }
        self.get_mut(app).text_align_vertical = value;
        self.mark_needs_layout(app);
    }

    fn set_is_focused(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).is_focused == value {
            return;
        }
        self.get_mut(app).is_focused = value;
    }

    // Indicates that the decoration should be aligned to accommodate an outline
    // border.
    fn is_outline_aligned(self: RenderHandle<Self>, app: &App) -> bool {
        let decoration = self.decoration(app);
        !decoration.is_collapsed && decoration.border.is_outline()
    }

    fn density_offset(self: RenderHandle<Self>, app: &App) -> Offset {
        self.decoration(app).visual_density.base_size_adjustment()
    }

    fn min_width(app: &mut App, r#box: Option<AnyRenderBox>, height: f64) -> f64 {
        r#box.map_or(0.0, |r#box| r#box.get_min_intrinsic_width(app, height))
    }

    fn max_width(app: &mut App, r#box: Option<AnyRenderBox>, height: f64) -> f64 {
        r#box.map_or(0.0, |r#box| r#box.get_max_intrinsic_width(app, height))
    }

    fn min_height(app: &mut App, r#box: Option<AnyRenderBox>, width: f64) -> f64 {
        r#box.map_or(0.0, |r#box| r#box.get_min_intrinsic_height(app, width))
    }

    fn box_size(app: &App, r#box: Option<AnyRenderBox>) -> Size {
        r#box.map_or(Size::ZERO, |r#box| r#box.size(app))
    }

    fn get_baseline(app: &mut App, r#box: AnyRenderBox, box_constraints: BoxConstraints) -> f64 {
        ChildLayoutHelper::get_baseline(app, r#box, box_constraints, TextBaseline::Alphabetic)
            .unwrap_or_else(|| r#box.size(app).height())
    }

    fn get_dry_baseline(
        app: &mut App,
        r#box: AnyRenderBox,
        box_constraints: BoxConstraints,
    ) -> f64 {
        ChildLayoutHelper::get_dry_baseline(app, r#box, box_constraints, TextBaseline::Alphabetic)
            .unwrap_or_else(|| {
                ChildLayoutHelper::dry_layout_child(app, r#box, box_constraints).height()
            })
    }

    fn box_parent_data(app: &App, r#box: AnyRenderBox) -> &BoxParentData {
        r#box.box_parent_data(app)
    }

    fn set_offset(app: &mut App, r#box: AnyRenderBox, offset: Offset) {
        r#box.parent_data_of_mut::<BoxParentData>(app).offset = offset;
    }

    fn content_padding(self: RenderHandle<Self>, app: &App) -> EdgeInsetsDirectional {
        self.decoration(app).content_padding
    }

    fn supporting_text_padding(
        self: RenderHandle<Self>,
        app: &App,
    ) -> Option<EdgeInsetsDirectional> {
        self.decoration(app).supporting_text_padding
    }

    fn compute_supporting_text_sizes(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        layout_child: ChildLayouter,
        get_baseline: ChildBaselineGetter,
    ) -> Option<SupportingTextSize> {
        let counter = self.counter(app);
        let (counter_size, counter_ascent) = match counter {
            Some(r#box) => (
                layout_child(app, r#box, constraints),
                get_baseline(app, r#box, constraints),
            ),
            None => (Size::ZERO, 0.0),
        };

        // Only add padding when counter is present (maxLength is used).
        let counter_padding = if counter.is_some() {
            K_SUPPORTING_TEXT_COUNTER_PADDING
        } else {
            0.0
        };
        let helper_error_constraints = constraints.deflate(EdgeInsetsGeometry::only(
            counter_size.width() + counter_padding,
            0.0,
            0.0,
            0.0,
        ));
        let helper_error = self.helper_error(app);
        let helper_error_height =
            layout_child(app, helper_error, helper_error_constraints).height();

        if helper_error_height == 0.0 && counter_size.height() == 0.0 {
            return None;
        }

        // TODO(LongCatIsLooong): the bottomHeight expression doesn't make much sense.
        // Use the real descent and make sure the supportingText line box is tall enough for both children.
        // See https://github.com/flutter/flutter/issues/13715

        // The vertical padding around the row containing the helper, error, and counter widgets.
        // Defaults to `supportingTextGap` at the top and 0.0 at the bottom when `supportingTextPadding` is null.
        let supporting_text_padding = self.supporting_text_padding(app);
        let top_padding = supporting_text_padding
            .map_or_else(|| self.supporting_text_gap(app), |padding| padding.top);
        let bottom_padding = supporting_text_padding.map_or(0.0, |padding| padding.bottom);
        let ascent = counter_ascent.max(get_baseline(app, helper_error, helper_error_constraints))
            + top_padding;
        let bottom_height =
            counter_ascent.max(helper_error_height) + (top_padding + bottom_padding);
        let supporting_text_height =
            counter_size.height().max(helper_error_height) + (top_padding + bottom_padding);
        Some(SupportingTextSize {
            ascent,
            bottom_height,
            supporting_text_height,
        })
    }

    // Returns a value used by performLayout to position all of the renderers.
    // This method applies layout to all of the renderers except the container.
    // For convenience, the container is laid out in performLayout().
    fn layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        layout_child: ChildLayouter,
        get_baseline: ChildBaselineGetter,
    ) -> RenderDecorationLayout {
        debug_assert!(
            constraints.max_width < f64::INFINITY,
            "An InputDecorator, which is typically created by a TextField, cannot \
             have an unbounded width.\n\
             This happens when the parent widget does not provide a finite width \
             constraint. For example, if the InputDecorator is contained by a Row, \
             then its width must be constrained. An Expanded widget or a SizedBox \
             can be used to constrain the width of the InputDecorator or the \
             TextField that contains it."
        );

        let box_constraints = constraints.loosen();
        let decoration = self.decoration(app).clone();
        let content_padding = self.content_padding(app);
        let supporting_text_padding = decoration.supporting_text_padding;

        // Layout all the widgets used by InputDecorator
        let icon = self.icon(app);
        let icon_width = match icon {
            None => 0.0,
            Some(icon) => layout_child(app, icon, box_constraints).width(),
        };
        let container_constraints =
            box_constraints.deflate(EdgeInsetsGeometry::only(icon_width, 0.0, 0.0, 0.0));
        let content_constraints = container_constraints.deflate(EdgeInsetsGeometry::directional(
            content_padding.start + decoration.input_gap,
            0.0,
            content_padding.end + decoration.input_gap,
            0.0,
        ));
        let supporting_text_constraints =
            container_constraints.deflate(EdgeInsetsGeometry::directional(
                supporting_text_padding.map_or(content_padding.start, |padding| padding.start)
                    + decoration.input_gap,
                0.0,
                supporting_text_padding.map_or(content_padding.end, |padding| padding.end)
                    + decoration.input_gap,
                0.0,
            ));

        // The helper or error text can occupy the full width less the space
        // occupied by the icon and counter.
        let supporting_text_size = self.compute_supporting_text_sizes(
            app,
            supporting_text_constraints,
            layout_child,
            get_baseline,
        );

        let prefix_icon = self.prefix_icon(app);
        let suffix_icon = self.suffix_icon(app);
        let prefix_icon_size = match prefix_icon {
            None => Size::ZERO,
            Some(prefix_icon) => layout_child(app, prefix_icon, container_constraints),
        };
        let suffix_icon_size = match suffix_icon {
            None => Size::ZERO,
            Some(suffix_icon) => layout_child(app, suffix_icon, container_constraints),
        };
        let prefix = self.prefix(app);
        let suffix = self.suffix(app);
        let prefix_size = match prefix {
            None => Size::ZERO,
            Some(prefix) => layout_child(app, prefix, content_constraints),
        };
        let suffix_size = match suffix {
            None => Size::ZERO,
            Some(suffix) => layout_child(app, suffix, content_constraints),
        };

        let accessory_horizontal_insets = EdgeInsetsDirectional::only(
            icon_width
                + prefix_size.width()
                + if prefix_icon.is_none() {
                    content_padding.start + decoration.input_gap
                } else {
                    prefix_icon_size.width() + self.prefix_to_input_gap(app)
                },
            0.0,
            suffix_size.width()
                + if suffix_icon.is_none() {
                    content_padding.end + decoration.input_gap
                } else {
                    suffix_icon_size.width() + self.input_to_suffix_gap(app)
                },
            0.0,
        );

        let input_width = (constraints.max_width
            - (accessory_horizontal_insets.start + accessory_horizontal_insets.end))
            .max(0.0);
        let label = self.label(app);
        let top_height = if let Some(label) = label {
            let suffix_icon_space = if decoration.border.is_outline() {
                lerp_double(
                    Some(suffix_icon_size.width()),
                    Some(content_padding.end),
                    decoration.floating_label_progress,
                )
                .expect("both ends are given")
            } else {
                suffix_icon_size.width()
            };
            let label_width = (constraints.max_width
                - (decoration.input_gap * 2.0
                    + icon_width
                    + if prefix_icon.is_none() {
                        content_padding.start
                    } else {
                        prefix_icon_size.width()
                    }
                    + if suffix_icon.is_none() {
                        content_padding.end
                    } else {
                        suffix_icon_space
                    }))
            .max(0.0);

            // Increase the available width for the label when it is scaled down.
            let inverted_label_scale = lerp_double(
                Some(1.00),
                Some(1.0 / K_FINAL_LABEL_SCALE),
                decoration.floating_label_progress,
            )
            .expect("both ends are given");
            let label_constraints = BoxConstraints {
                max_width: label_width * inverted_label_scale,
                ..box_constraints
            };
            layout_child(app, label, label_constraints);

            let label_height = decoration.floating_label_height;
            if decoration.border.is_outline() {
                (label_height - get_baseline(app, label, label_constraints)).max(0.0)
            } else {
                label_height
            }
        } else {
            0.0
        };

        // The height of the input needs to accommodate label above and counter and
        // helperError below, when they exist.
        let bottom_height = supporting_text_size.map_or(0.0, |size| size.bottom_height);
        let density_offset = self.density_offset(app);
        let input_constraints = box_constraints
            .deflate(EdgeInsetsGeometry::only(
                0.0,
                (content_padding.top + content_padding.bottom)
                    + top_height
                    + bottom_height
                    + density_offset.dy(),
                0.0,
                0.0,
            ))
            .tighten(Some(input_width), None);

        let input = self.input(app);
        let hint = self.hint(app);
        let input_size = match input {
            None => Size::ZERO,
            Some(input) => layout_child(app, input, input_constraints),
        };
        let hint_size = match hint {
            None => Size::ZERO,
            Some(hint) => layout_child(app, hint, box_constraints.tighten(Some(input_width), None)),
        };
        let input_baseline = match input {
            None => 0.0,
            Some(input) => get_baseline(app, input, input_constraints),
        };
        let hint_baseline = match hint {
            None => 0.0,
            Some(hint) => get_baseline(app, hint, box_constraints.tighten(Some(input_width), None)),
        };

        // The field can be occupied by a hint or by the input itself.
        let input_height = (if decoration.is_empty || decoration.maintain_hint_size {
            hint_size.height()
        } else {
            0.0
        })
        .max(input_size.height());
        let input_internal_baseline = input_baseline.max(hint_baseline);

        let prefix_baseline = match prefix {
            None => 0.0,
            Some(prefix) => get_baseline(app, prefix, content_constraints),
        };
        let suffix_baseline = match suffix {
            None => 0.0,
            Some(suffix) => get_baseline(app, suffix, content_constraints),
        };

        // Calculate the amount that prefix/suffix affects height above and below
        // the input.
        let fix_height = prefix_baseline.max(suffix_baseline);
        let fix_above_input = (fix_height - input_internal_baseline).max(0.0);
        let fix_below_baseline =
            (prefix_size.height() - prefix_baseline).max(suffix_size.height() - suffix_baseline);
        // TODO(justinmc): fixBelowInput should have no effect when there is no
        // prefix/suffix below the input.
        // https://github.com/flutter/flutter/issues/66050
        let fix_below_input =
            (fix_below_baseline - (input_height - input_internal_baseline)).max(0.0);

        // Calculate the height of the input text container.
        let fix_icon_height = prefix_icon_size.height().max(suffix_icon_size.height());
        let content_height = fix_icon_height.max(
            top_height
                + content_padding.top
                + fix_above_input
                + input_height
                + fix_below_input
                + content_padding.bottom
                + density_offset.dy(),
        );
        let expands = self.expands(app);
        let min_container_height = if decoration.is_dense.expect("applied from the theme")
            || decoration.is_collapsed
            || expands
        {
            input_height
        } else {
            K_MIN_INTERACTIVE_DIMENSION
        };
        let max_container_height = (box_constraints.max_height - bottom_height).max(0.0);
        let container_height = if expands {
            max_container_height
        } else {
            content_height
                .max(min_container_height)
                .min(max_container_height)
        };

        // Ensure the text is vertically centered in cases where the content is
        // shorter than kMinInteractiveDimension.
        let interactive_adjustment = if min_container_height > content_height {
            (min_container_height - content_height) / 2.0
        } else {
            0.0
        };

        // Try to consider the prefix/suffix as part of the text when aligning it.
        // If the prefix/suffix overflows however, allow it to extend outside of the
        // input and align the remaining part of the text and prefix/suffix.
        let overflow = (content_height - max_container_height).max(0.0);
        // Map textAlignVertical from -1:1 to 0:1 so that it can be used to scale
        // the baseline from its minimum to maximum values.
        let text_align_vertical = self.text_align_vertical(app);
        let text_align_vertical_factor = (text_align_vertical.y + 1.0) / 2.0;
        // Adjust to try to fit top overflow inside the input on an inverse scale of
        // textAlignVertical, so that top aligned text adjusts the most and bottom
        // aligned text doesn't adjust at all.
        let baseline_adjustment = fix_above_input - overflow * (1.0 - text_align_vertical_factor);

        // The baselines that will be used to draw the actual input text content.
        let top_input_baseline = content_padding.top
            + top_height
            + input_internal_baseline
            + baseline_adjustment
            + interactive_adjustment
            + density_offset.dy() / 2.0;
        let max_content_height = container_height
            - (content_padding.top + content_padding.bottom)
            - top_height
            - density_offset.dy();
        let alignable_height = fix_above_input + input_height + fix_below_input;
        let max_vertical_offset = max_content_height - alignable_height;

        let baseline = if self.is_outline_aligned(app) {
            // The three main alignments for the baseline when an outline is present are
            //
            //  * top (-1.0): topmost point considering padding.
            //  * center (0.0): the absolute center of the input ignoring padding but
            //      accommodating the border and floating label.
            //  * bottom (1.0): bottommost point considering padding.
            //
            // That means that if the padding is uneven, center is not the exact
            // midpoint of top and bottom. To account for this, the above center and
            // below center alignments are interpolated independently.
            let outline_center_baseline = input_internal_baseline
                + baseline_adjustment / 2.0
                + (container_height - input_height) / 2.0;
            let outline_top_baseline = top_input_baseline;
            let outline_bottom_baseline = top_input_baseline + max_vertical_offset;
            RenderDecoration::interpolate_three(
                outline_top_baseline,
                outline_center_baseline,
                outline_bottom_baseline,
                text_align_vertical,
            )
        } else {
            let text_align_vertical_offset = max_vertical_offset * text_align_vertical_factor;
            top_input_baseline + text_align_vertical_offset
        };

        RenderDecorationLayout {
            input_constraints,
            container_height,
            baseline,
            supporting_text_size,
            size: Size::new(
                constraints.max_width,
                container_height
                    + supporting_text_size.map_or(0.0, |size| size.supporting_text_height),
            ),
        }
    }

    // Interpolate between three stops using textAlignVertical. This is used to
    // calculate the outline baseline, which ignores padding when the alignment is
    // middle. When the alignment is less than zero, it interpolates between the
    // centered text box's top and the top of the content padding. When the
    // alignment is greater than zero, it interpolates between the centered box's
    // top and the position that would align the bottom of the box with the bottom
    // padding.
    fn interpolate_three(
        begin: f64,
        middle: f64,
        end: f64,
        text_align_vertical: TextAlignVertical,
    ) -> f64 {
        // It's possible for begin, middle, and end to not be in order because of
        // excessive padding. Those cases are handled by using middle.
        let basis = if text_align_vertical.y <= 0.0 {
            (middle - begin).max(0.0)
        } else {
            (end - middle).max(0.0)
        };
        middle + basis * text_align_vertical.y
    }

    fn line_height(app: &mut App, width: f64, boxes: &[Option<AnyRenderBox>]) -> f64 {
        let mut height: f64 = 0.0;
        for r#box in boxes {
            let Some(r#box) = *r#box else {
                continue;
            };
            height = RenderDecoration::min_height(app, Some(r#box), width).max(height);
        }
        height
        // TODO(hansmuller): this should compute the overall line height for the
        // boxes when they've been baseline-aligned.
        // See https://github.com/flutter/flutter/issues/13715
    }

    fn label_offset(app: &App, label: AnyRenderBox) -> Offset {
        RenderDecoration::box_parent_data(app, label).offset
    }

    fn paint_label(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let label = self
            .label(app)
            .expect("the label is painted only when there is one");
        context.paint_child(app, label.as_object(), offset);
    }
}

impl SlottedContainerRenderObjectMixin<DecorationSlot> for RenderDecoration {
    type ChildType = AnyRenderBox;

    fn slotted_container_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &SlottedContainerRenderObjectData<DecorationSlot, AnyRenderBox> {
        &self.get(app).slotted
    }

    fn slotted_container_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut SlottedContainerRenderObjectData<DecorationSlot, AnyRenderBox> {
        &mut self.get_mut(app).slotted
    }

    // The returned list is ordered for hit testing.
    fn children(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderBox> {
        let helper_error = self.child_for_slot(app, &DecorationSlot::HelperError);
        [
            self.icon(app),
            self.input(app),
            self.prefix_icon(app),
            self.suffix_icon(app),
            self.prefix(app),
            self.suffix(app),
            self.label(app),
            self.hint(app),
            helper_error,
            self.counter(app),
            self.container(app),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

impl RenderObject for RenderDecoration {
    waft_rendering::render_object_accessors!();

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        SlottedContainerRenderObjectMixin::did_attach(self, app, owner);
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        SlottedContainerRenderObjectMixin::did_detach(self, app);
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        SlottedContainerRenderObjectMixin::redepth_children(self, app);
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        SlottedContainerRenderObjectMixin::visit_children(self, app, visitor);
    }

    fn sized_by_parent(self: RenderHandle<Self>, _app: &App) -> bool {
        false
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        self.get_mut(app).label_transform = None;
        let layout = self.layout(
            app,
            constraints,
            ChildLayoutHelper::layout_child,
            RenderDecoration::get_baseline,
        );
        let size = constraints.constrain(layout.size);
        self.set_size(app, size);
        debug_assert!(size.width() == constraints.constrain_width(layout.size.width()));
        debug_assert!(size.height() == constraints.constrain_height(layout.size.height()));

        let overall_width = layout.size.width();
        let text_direction = self.text_direction(app);
        let decoration = self.decoration(app).clone();
        let content_padding = self.content_padding(app);
        let supporting_text_padding = decoration.supporting_text_padding;
        let icon = self.icon(app);

        let container = self.container(app);
        if let Some(container) = container {
            let container_constraints = BoxConstraints::tight_for(
                Some(overall_width - RenderDecoration::box_size(app, icon).width()),
                Some(layout.container_height),
            );
            container.layout(app, container_constraints, true);
            let x = match text_direction {
                TextDirection::Rtl => 0.0,
                TextDirection::Ltr => RenderDecoration::box_size(app, icon).width(),
            };
            RenderDecoration::set_offset(app, container, Offset::new(x, 0.0));
        }

        let height = layout.container_height;
        let center_layout = |app: &mut App, r#box: AnyRenderBox, x: f64| -> f64 {
            let box_size = r#box.size(app);
            RenderDecoration::set_offset(
                app,
                r#box,
                Offset::new(x, (height - box_size.height()) / 2.0),
            );
            box_size.width()
        };

        if let Some(icon) = icon {
            let x = match text_direction {
                TextDirection::Rtl => overall_width - icon.size(app).width(),
                TextDirection::Ltr => 0.0,
            };
            center_layout(app, icon, x);
        }

        let supporting_text_baseline =
            layout.supporting_text_size.map_or(0.0, |size| size.ascent) + layout.container_height;
        let counter = self.counter(app);
        let helper_error = self.helper_error(app);
        let helper_error_baseline = helper_error
            .get_distance_to_baseline(app, TextBaseline::Alphabetic, false)
            .expect("a baseline that falls back to the height");
        let counter_baseline = counter.map_or(0.0, |counter| {
            counter
                .get_distance_to_baseline(app, TextBaseline::Alphabetic, false)
                .expect("a baseline that falls back to the height")
        });

        let icon_width = RenderDecoration::box_size(app, icon).width();
        let mut start;
        let mut end;
        let start_supporting;
        let end_supporting;
        match text_direction {
            TextDirection::Ltr => {
                start = content_padding.start + icon_width;
                end = overall_width - content_padding.end;
                start_supporting = supporting_text_padding
                    .map_or(content_padding.start, |padding| padding.start)
                    + icon_width;
                end_supporting = overall_width
                    - supporting_text_padding.map_or(content_padding.end, |padding| padding.end);
                RenderDecoration::set_offset(
                    app,
                    helper_error,
                    Offset::new(
                        start_supporting + decoration.input_gap,
                        supporting_text_baseline - helper_error_baseline,
                    ),
                );
                if let Some(counter) = counter {
                    let counter_width = counter.size(app).width();
                    RenderDecoration::set_offset(
                        app,
                        counter,
                        Offset::new(
                            end_supporting - counter_width - decoration.input_gap,
                            supporting_text_baseline - counter_baseline,
                        ),
                    );
                }
            }
            TextDirection::Rtl => {
                start = overall_width - content_padding.start - icon_width;
                end = content_padding.end;
                start_supporting = overall_width
                    - supporting_text_padding
                        .map_or(content_padding.start, |padding| padding.start)
                    - icon_width;
                end_supporting =
                    supporting_text_padding.map_or(content_padding.end, |padding| padding.end);
                let helper_error_width = helper_error.size(app).width();
                RenderDecoration::set_offset(
                    app,
                    helper_error,
                    Offset::new(
                        start_supporting - helper_error_width - decoration.input_gap,
                        supporting_text_baseline - helper_error_baseline,
                    ),
                );
                if let Some(counter) = counter {
                    RenderDecoration::set_offset(
                        app,
                        counter,
                        Offset::new(
                            end_supporting + decoration.input_gap,
                            supporting_text_baseline - counter_baseline,
                        ),
                    );
                }
            }
        }

        let baseline = layout.baseline;
        let baseline_layout = |app: &mut App, r#box: AnyRenderBox, x: f64| -> f64 {
            let distance = r#box
                .get_distance_to_baseline(app, TextBaseline::Alphabetic, false)
                .expect("a baseline that falls back to the height");
            RenderDecoration::set_offset(app, r#box, Offset::new(x, baseline - distance));
            r#box.size(app).width()
        };

        let prefix_icon = self.prefix_icon(app);
        let suffix_icon = self.suffix_icon(app);
        let label = self.label(app);
        let prefix = self.prefix(app);
        let suffix = self.suffix(app);
        let input = self.input(app);
        let hint = self.hint(app);
        let prefix_to_input_gap = self.prefix_to_input_gap(app);
        let input_to_suffix_gap = self.input_to_suffix_gap(app);
        match text_direction {
            TextDirection::Rtl => {
                if let Some(prefix_icon) = prefix_icon {
                    start += content_padding.start;
                    let x = start - prefix_icon.size(app).width();
                    start -= center_layout(app, prefix_icon, x);
                    start -= prefix_to_input_gap;
                } else {
                    start -= decoration.input_gap;
                }
                if let Some(label) = label {
                    let x = start - label.size(app).width();
                    if decoration.align_label_with_hint {
                        baseline_layout(app, label, x);
                    } else {
                        center_layout(app, label, x);
                    }
                }
                if let Some(prefix) = prefix {
                    let x = start - prefix.size(app).width();
                    start -= baseline_layout(app, prefix, x);
                }
                if let Some(input) = input {
                    let x = start - input.size(app).width();
                    baseline_layout(app, input, x);
                }
                if let Some(hint) = hint {
                    let x = start - hint.size(app).width();
                    baseline_layout(app, hint, x);
                }
                if let Some(suffix_icon) = suffix_icon {
                    end -= content_padding.end;
                    end += center_layout(app, suffix_icon, end);
                    end += input_to_suffix_gap;
                } else {
                    end += decoration.input_gap;
                }
                if let Some(suffix) = suffix {
                    baseline_layout(app, suffix, end);
                }
            }
            TextDirection::Ltr => {
                if let Some(prefix_icon) = prefix_icon {
                    start -= content_padding.start;
                    start += center_layout(app, prefix_icon, start);
                    start += prefix_to_input_gap;
                } else {
                    start += decoration.input_gap;
                }
                if let Some(label) = label {
                    if decoration.align_label_with_hint {
                        baseline_layout(app, label, start);
                    } else {
                        center_layout(app, label, start);
                    }
                }
                if let Some(prefix) = prefix {
                    start += baseline_layout(app, prefix, start);
                }
                if let Some(input) = input {
                    baseline_layout(app, input, start);
                }
                if let Some(hint) = hint {
                    baseline_layout(app, hint, start);
                }
                if let Some(suffix_icon) = suffix_icon {
                    end += content_padding.end;
                    let x = end - suffix_icon.size(app).width();
                    end -= center_layout(app, suffix_icon, x);
                    end -= input_to_suffix_gap;
                } else {
                    end -= decoration.input_gap;
                }
                if let Some(suffix) = suffix {
                    let x = end - suffix.size(app).width();
                    baseline_layout(app, suffix, x);
                }
            }
        }

        let border_gap = decoration.border_gap;
        if let Some(label) = label {
            let label_x = RenderDecoration::label_offset(app, label).dx();
            // +1 shifts the range of x from (-1.0, 1.0) to (0.0, 2.0).
            let float_align = decoration.floating_label_alignment.x + 1.0;
            let label_width = RenderDecoration::box_size(app, Some(label)).width();
            let float_width = label_width * K_FINAL_LABEL_SCALE;
            let container_width = RenderDecoration::box_size(app, container).width();
            let prefix_icon_width = RenderDecoration::box_size(app, prefix_icon).width();
            let material3 = self.material3(app);
            // When floating label is centered, its x is relative to
            // _BorderContainer's x and is independent of label's x.
            match text_direction {
                TextDirection::Rtl => {
                    let mut offset_to_prefix_icon = 0.0;
                    if prefix_icon.is_some() && !decoration.align_label_with_hint {
                        offset_to_prefix_icon = if material3 {
                            prefix_icon_width - content_padding.end
                        } else {
                            0.0
                        };
                    }
                    border_gap.set_start(
                        app,
                        lerp_double(
                            Some(label_x + label_width + offset_to_prefix_icon),
                            Some(container_width / 2.0 + float_width / 2.0),
                            float_align,
                        ),
                    );
                }
                TextDirection::Ltr => {
                    // The value of _InputBorderGap.start is relative to the origin of the
                    // _BorderContainer which is inset by the icon's width. Although, when
                    // floating label is centered, it's already relative to _BorderContainer.
                    let mut offset_to_prefix_icon = 0.0;
                    if prefix_icon.is_some() && !decoration.align_label_with_hint {
                        offset_to_prefix_icon = if material3 {
                            -prefix_icon_width + content_padding.start
                        } else {
                            0.0
                        };
                    }
                    border_gap.set_start(
                        app,
                        lerp_double(
                            Some(label_x - icon_width + offset_to_prefix_icon),
                            Some(container_width / 2.0 - float_width / 2.0),
                            float_align,
                        ),
                    );
                }
            }
            border_gap.set_extent(app, label.size(app).width() * K_FINAL_LABEL_SCALE);
        } else {
            border_gap.set_start(app, None);
            border_gap.set_extent(app, 0.0);
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let do_paint =
            |app: &mut App, context: &mut PaintingContext, child: Option<AnyRenderBox>| {
                if let Some(child) = child {
                    let child_offset = RenderDecoration::box_parent_data(app, child).offset;
                    context.paint_child(app, child.as_object(), child_offset + offset);
                }
            };

        let container = self.container(app);
        do_paint(app, context, container);

        if let Some(label) = self.label(app) {
            let decoration = self.decoration(app).clone();
            let label_offset = RenderDecoration::label_offset(app, label);
            let label_height = RenderDecoration::box_size(app, Some(label)).height();
            let label_width = RenderDecoration::box_size(app, Some(label)).width();
            // +1 shifts the range of x from (-1.0, 1.0) to (0.0, 2.0).
            let float_align = decoration.floating_label_alignment.x + 1.0;
            let float_width = label_width * K_FINAL_LABEL_SCALE;
            let border_side = decoration.border.border_side();
            let t = decoration.floating_label_progress;
            // The center of the outline border label ends up a little below the
            // center of the top border line.
            let is_outline_border = decoration.border.is_outline();
            // Temporary opt-in fix for https://github.com/flutter/flutter/issues/54028
            // Center the scaled label relative to the border.
            let outlined_floating_y =
                (-label_height * K_FINAL_LABEL_SCALE) / 2.0 - border_side.stroke_offset() / 2.0;
            let floating_y = if is_outline_border {
                outlined_floating_y
            } else {
                decoration.content_padding.top + self.density_offset(app).dy() / 2.0
            };
            let scale =
                lerp_double(Some(1.0), Some(K_FINAL_LABEL_SCALE), t).expect("both ends are given");
            let container = container.expect("an input decorator always has a container");
            let centered_float_x = RenderDecoration::box_parent_data(app, container)
                .offset
                .dx()
                + RenderDecoration::box_size(app, Some(container)).width() / 2.0
                - float_width / 2.0;
            let prefix_icon = self.prefix_icon(app);
            let prefix_icon_width = RenderDecoration::box_size(app, prefix_icon).width();
            let material3 = self.material3(app);
            let start_x;
            let mut float_start_x;
            match self.text_direction(app) {
                // origin is on the right
                TextDirection::Rtl => {
                    start_x = label_offset.dx() + label_width * (1.0 - scale);
                    float_start_x = start_x;
                    if prefix_icon.is_some()
                        && !decoration.align_label_with_hint
                        && is_outline_border
                    {
                        float_start_x += if material3 {
                            prefix_icon_width - decoration.content_padding.end
                        } else {
                            0.0
                        };
                    }
                }
                // origin on the left
                TextDirection::Ltr => {
                    start_x = label_offset.dx();
                    float_start_x = start_x;
                    if prefix_icon.is_some()
                        && !decoration.align_label_with_hint
                        && is_outline_border
                    {
                        float_start_x += if material3 {
                            -prefix_icon_width + decoration.content_padding.start
                        } else {
                            0.0
                        };
                    }
                }
            }
            let float_end_x = lerp_double(Some(float_start_x), Some(centered_float_x), float_align)
                .expect("both ends are given");
            let dx = lerp_double(Some(start_x), Some(float_end_x), t).expect("both ends are given");
            let dy = lerp_double(Some(0.0), Some(floating_y - label_offset.dy()), t)
                .expect("both ends are given");
            // Dart's `Matrix4.identity()..translateByDouble(dx, labelOffset.dy + dy, 0, 1)
            // ..scaleByDouble(scale, scale, scale, 1)`, column-major.
            let (dx, dy, scale) = (dx as f32, (label_offset.dy() + dy) as f32, scale as f32);
            let label_transform = Matrix4::from_flutter_array(&[
                scale, 0.0, 0.0, 0.0, //
                0.0, scale, 0.0, 0.0, //
                0.0, 0.0, scale, 0.0, //
                dx, dy, 0.0, 1.0,
            ]);
            self.get_mut(app).label_transform = Some(label_transform);
            let old_layer = self.as_object().layer_as::<TransformLayer>(app);
            let needs_compositing = self.as_object().needs_compositing(app);
            let layer = context.push_transform(
                app,
                needs_compositing,
                offset,
                label_transform,
                |app, context, offset| self.paint_label(app, context, offset),
                old_layer,
            );
            self.as_object()
                .set_layer(app, layer.map(|layer| layer.as_container_layer()));
        } else {
            self.as_object().set_layer(app, None);
        }

        do_paint(app, context, self.icon(app));
        do_paint(app, context, self.prefix(app));
        do_paint(app, context, self.suffix(app));
        do_paint(app, context, self.prefix_icon(app));
        do_paint(app, context, self.suffix_icon(app));
        if self.decoration(app).is_empty {
            do_paint(app, context, self.hint(app));
        }
        do_paint(app, context, self.input(app));
        do_paint(app, context, Some(self.helper_error(app)));
        do_paint(app, context, self.counter(app));
    }
}

impl RenderBox for RenderDecoration {
    waft_rendering::render_box_accessors!();

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let decoration = self.decoration(app).clone();
        let input = self.input(app);
        let input_width = if decoration.is_empty || decoration.maintain_hint_size {
            let input_width = RenderDecoration::min_width(app, input, height);
            let hint = self.hint(app);
            input_width.max(RenderDecoration::min_width(app, hint, height))
        } else {
            RenderDecoration::min_width(app, input, height)
        };
        let content_width = if decoration.maintain_label_size {
            let label = self.label(app);
            input_width.max(RenderDecoration::min_width(app, label, height))
        } else {
            input_width
        };
        let prefix_icon = self.prefix_icon(app);
        let suffix_icon = self.suffix_icon(app);
        let content_padding = self.content_padding(app);
        let leading_gap = if prefix_icon.is_some() {
            self.prefix_to_input_gap(app)
        } else {
            content_padding.start + decoration.input_gap
        };
        let trailing_gap = if suffix_icon.is_some() {
            self.input_to_suffix_gap(app)
        } else {
            content_padding.end + decoration.input_gap
        };
        let icon = self.icon(app);
        let prefix = self.prefix(app);
        let suffix = self.suffix(app);
        RenderDecoration::min_width(app, icon, height)
            + leading_gap
            + RenderDecoration::min_width(app, prefix_icon, height)
            + RenderDecoration::min_width(app, prefix, height)
            + content_width
            + RenderDecoration::min_width(app, suffix, height)
            + RenderDecoration::min_width(app, suffix_icon, height)
            + trailing_gap
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let decoration = self.decoration(app).clone();
        let input = self.input(app);
        let input_width = if decoration.is_empty || decoration.maintain_hint_size {
            let input_width = RenderDecoration::max_width(app, input, height);
            let hint = self.hint(app);
            input_width.max(RenderDecoration::max_width(app, hint, height))
        } else {
            RenderDecoration::max_width(app, input, height)
        };
        let content_width = if decoration.maintain_label_size {
            let label = self.label(app);
            input_width.max(RenderDecoration::max_width(app, label, height))
        } else {
            input_width
        };
        let prefix_icon = self.prefix_icon(app);
        let suffix_icon = self.suffix_icon(app);
        let content_padding = self.content_padding(app);
        let leading_gap = if prefix_icon.is_some() {
            self.prefix_to_input_gap(app)
        } else {
            content_padding.start + decoration.input_gap
        };
        let trailing_gap = if suffix_icon.is_some() {
            self.input_to_suffix_gap(app)
        } else {
            content_padding.end + decoration.input_gap
        };
        let icon = self.icon(app);
        let prefix = self.prefix(app);
        let suffix = self.suffix(app);
        RenderDecoration::max_width(app, icon, height)
            + leading_gap
            + RenderDecoration::max_width(app, prefix_icon, height)
            + RenderDecoration::max_width(app, prefix, height)
            + content_width
            + RenderDecoration::max_width(app, suffix, height)
            + RenderDecoration::max_width(app, suffix_icon, height)
            + trailing_gap
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let decoration = self.decoration(app).clone();
        let content_padding = self.content_padding(app);
        let supporting_text_padding = decoration.supporting_text_padding;
        let icon = self.icon(app);
        let icon_height = RenderDecoration::min_height(app, icon, width);
        let icon_width = RenderDecoration::min_width(app, icon, icon_height);

        let width = (width - icon_width).max(0.0);
        let supporting_text_width = (width
            - supporting_text_padding
                .map_or(content_padding.start + content_padding.end, |padding| {
                    padding.start + padding.end
                })
            - decoration.input_gap * 2.0)
            .max(0.0);

        let prefix_icon = self.prefix_icon(app);
        let prefix_icon_height = RenderDecoration::min_height(app, prefix_icon, width);
        let prefix_icon_width = RenderDecoration::min_width(app, prefix_icon, prefix_icon_height);

        let suffix_icon = self.suffix_icon(app);
        let suffix_icon_height = RenderDecoration::min_height(app, suffix_icon, width);
        let suffix_icon_width = RenderDecoration::min_width(app, suffix_icon, suffix_icon_height);

        let width =
            (width - (content_padding.start + content_padding.end) - decoration.input_gap * 2.0)
                .max(0.0);

        // TODO(LongCatIsLooong): use _computeSupportingTextSizes for supportingText intrinsic sizes.
        // See https://github.com/flutter/flutter/issues/13715.
        let counter = self.counter(app);
        let counter_height = RenderDecoration::min_height(app, counter, supporting_text_width);
        let counter_width = RenderDecoration::min_width(app, counter, counter_height);

        // Only add padding when counter is present (maxLength is used).
        let counter_padding = if counter.is_some() {
            K_SUPPORTING_TEXT_COUNTER_PADDING
        } else {
            0.0
        };
        let helper_error_available_width =
            (supporting_text_width - counter_width - counter_padding).max(0.0);
        let helper_error = self.helper_error(app);
        let helper_error_height =
            RenderDecoration::min_height(app, Some(helper_error), helper_error_available_width);
        let mut supporting_text_height = counter_height.max(helper_error_height);
        if supporting_text_height > 0.0 {
            let top_padding = supporting_text_padding
                .map_or_else(|| self.supporting_text_gap(app), |padding| padding.top);
            let bottom_padding = supporting_text_padding.map_or(0.0, |padding| padding.bottom);
            supporting_text_height += top_padding + bottom_padding;
        }

        let prefix = self.prefix(app);
        let prefix_height = RenderDecoration::min_height(app, prefix, width);
        let prefix_width = RenderDecoration::min_width(app, prefix, prefix_height);

        let suffix = self.suffix(app);
        let suffix_height = RenderDecoration::min_height(app, suffix, width);
        let suffix_width = RenderDecoration::min_width(app, suffix, suffix_height);

        let available_input_width =
            (width - prefix_width - suffix_width - prefix_icon_width - suffix_icon_width).max(0.0);
        let input = self.input(app);
        let hint = if decoration.is_empty {
            self.hint(app)
        } else {
            None
        };
        let input_height =
            RenderDecoration::line_height(app, available_input_width, &[input, hint]);
        let input_max_height = [input_height, prefix_height, suffix_height]
            .into_iter()
            .fold(f64::NEG_INFINITY, f64::max);

        let label = self.label(app);
        let content_height = content_padding.top
            + if label.is_none() {
                0.0
            } else {
                decoration.floating_label_height
            }
            + input_max_height
            + content_padding.bottom
            + self.density_offset(app).dy();
        let container_height = [
            icon_height,
            content_height,
            prefix_icon_height,
            suffix_icon_height,
        ]
        .into_iter()
        .fold(f64::NEG_INFINITY, f64::max);
        // Keep in sync with the minimum container height computed in _layout.
        let min_container_height = if decoration.is_dense.expect("applied from the theme")
            || decoration.is_collapsed
            || self.expands(app)
        {
            input_height
        } else {
            K_MIN_INTERACTIVE_DIMENSION
        };

        container_height.max(min_container_height) + supporting_text_height
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        self.as_box().get_min_intrinsic_height(app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let Some(input) = self.input(app) else {
            return Some(0.0);
        };
        let offset_dy = RenderDecoration::box_parent_data(app, input).offset.dy();
        let distance = input
            .get_distance_to_actual_baseline(app, baseline)
            .unwrap_or_else(|| input.size(app).height());
        Some(offset_dy + distance)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let Some(input) = self.input(app) else {
            return Some(0.0);
        };
        let layout = self.layout(
            app,
            constraints,
            ChildLayoutHelper::dry_layout_child,
            RenderDecoration::get_dry_baseline,
        );
        let baseline_offset = match baseline {
            TextBaseline::Alphabetic => 0.0,
            TextBaseline::Ideographic => {
                let ideographic = input
                    .get_dry_baseline(app, layout.input_constraints, TextBaseline::Ideographic)
                    .unwrap_or_else(|| {
                        input.get_dry_layout(app, layout.input_constraints).height()
                    });
                let alphabetic = input
                    .get_dry_baseline(app, layout.input_constraints, TextBaseline::Alphabetic)
                    .unwrap_or_else(|| {
                        input.get_dry_layout(app, layout.input_constraints).height()
                    });
                ideographic - alphabetic
            }
        };
        Some(baseline_offset + layout.baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let layout = self.layout(
            app,
            constraints,
            ChildLayoutHelper::dry_layout_child,
            RenderDecoration::get_dry_baseline,
        );
        constraints.constrain(layout.size)
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        if let Some(label) = self.label(app)
            && child == label.as_object()
            && let Some(label_transform) = self.get(app).label_transform
        {
            let label_offset = RenderDecoration::label_offset(app, label);
            *transform = transform.then(&label_transform).then(&Matrix4::translation(
                -label_offset.dx() as f32,
                -label_offset.dy() as f32,
            ));
        }
        // Dart's `super.applyPaintTransform`: `RenderBox`'s, which translates by the child's
        // offset.
        let offset = child.parent_data_of::<BoxParentData>(app).offset;
        *transform = transform.then(&Matrix4::translation(
            offset.dx() as f32,
            offset.dy() as f32,
        ));
    }

    fn hit_test_self(self: RenderHandle<Self>, _app: &App, _position: Offset) -> bool {
        true
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let label = self.label(app);
        for child in SlottedContainerRenderObjectMixin::children(self, app) {
            // The label must be handled specially since we've transformed it.
            if Some(child) == label && self.get(app).label_transform.is_some() {
                let mut transform = Matrix4::IDENTITY;
                RenderBox::apply_paint_transform(self, app, child.as_object(), &mut transform);
                if result.add_with_paint_transform(
                    Some(transform),
                    position,
                    |result, transformed| child.hit_test(app, result, transformed),
                ) {
                    return true;
                }
                continue;
            }
            let offset = RenderDecoration::box_parent_data(app, child).offset;
            let is_hit =
                result.add_with_paint_offset(Some(offset), position, |result, transformed| {
                    debug_assert!(transformed == position - offset);
                    child.hit_test(app, result, transformed)
                });
            if is_hit {
                return true;
            }
        }
        false
    }
}

impl Debug for RenderDecoration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_RenderDecoration")
            .field("decoration", &self.decoration)
            .finish_non_exhaustive()
    }
}

/// Dart's `_Decorator`.
#[derive(Debug)]
struct Decorator {
    decoration: Decoration,
    text_direction: TextDirection,
    text_baseline: TextBaseline,
    text_align_vertical: Option<TextAlignVertical>,
    is_focused: bool,
    expands: bool,
}

impl SlottedMultiChildRenderObjectWidget for Decorator {
    type Slot = DecorationSlot;

    fn slots(&self) -> Vec<DecorationSlot> {
        DecorationSlot::VALUES.to_vec()
    }

    fn child_for_slot(&self, slot: &DecorationSlot) -> Option<WidgetRef> {
        let decoration = &self.decoration;
        match slot {
            DecorationSlot::Icon => decoration.icon.clone(),
            DecorationSlot::Input => decoration.input.clone(),
            DecorationSlot::Label => decoration.label.clone(),
            DecorationSlot::Hint => decoration.hint.clone(),
            DecorationSlot::Prefix => decoration.prefix.clone(),
            DecorationSlot::Suffix => decoration.suffix.clone(),
            DecorationSlot::PrefixIcon => decoration.prefix_icon.clone(),
            DecorationSlot::SuffixIcon => decoration.suffix_icon.clone(),
            DecorationSlot::HelperError => decoration.helper_error.clone(),
            DecorationSlot::Counter => decoration.counter.clone(),
            DecorationSlot::Container => decoration.container.clone(),
        }
    }
}

impl RenderObjectWidget for Decorator {
    type RenderObject = RenderDecoration;

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let material3 = Theme::of(app, context).use_material3;
        RenderDecoration::new(
            app,
            self.decoration.clone(),
            self.text_direction,
            self.text_baseline,
            self.is_focused,
            self.expands,
            material3,
            self.text_align_vertical,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderDecoration>,
    ) {
        render_object.set_decoration(app, self.decoration.clone());
        render_object.set_expands(app, self.expands);
        render_object.set_is_focused(app, self.is_focused);
        render_object.set_text_align_vertical(app, self.text_align_vertical);
        render_object.set_text_baseline(app, self.text_baseline);
        render_object.set_text_direction(app, self.text_direction);
    }
}

/// Dart's `_AffixText`.
#[derive(Debug)]
struct AffixText {
    label_is_floating: bool,
    text: Option<String>,
    style: Option<TextStyle>,
    child: Option<WidgetRef>,
}

impl StatelessWidget for AffixText {
    fn key(&self) -> Option<&KeyRef> {
        None
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        // `Semantics(sortKey:, tagForChildren:)` around the child waits with accessibility.
        let child = self.child.clone().or_else(|| {
            self.text.as_ref().map(|text| {
                let mut text = Text::new(text.clone());
                if let Some(style) = &self.style {
                    text = text.style(style.clone());
                }
                text.into_widget()
            })
        });
        let mut opacity = AnimatedOpacity::new(
            if self.label_is_floating { 1.0 } else { 0.0 },
            K_TRANSITION_DURATION,
        )
        .curve(k_transition_curve());
        if let Some(child) = child {
            opacity = opacity.child(child);
        }
        let mut merge = DefaultTextStyle::merge(
            IgnorePointer::new()
                .ignoring(!self.label_is_floating)
                .child(opacity),
        );
        if let Some(style) = &self.style {
            merge = merge.style(style.clone());
        }
        merge.into_widget()
    }
}

/// Defines the appearance of a Material Design text field.
///
/// [`InputDecorator`] displays the visual elements of a Material Design text
/// field around its input [`child`](Self::child). The visual elements themselves are defined
/// by an [`InputDecoration`] object and their layout and appearance depend
/// on the `base_style`, `text_align`, `is_focused`, and `is_empty` parameters.
///
/// `TextField` uses this widget to decorate its `EditableText` child.
///
/// [`InputDecorator`] can be used to create widgets that look and behave like a
/// `TextField` but support other kinds of input.
///
/// Requires one of its ancestors to be a `Material` widget. The [`child`](Self::child) widget,
/// as well as the decorative widgets specified in [`decoration`](Self::decoration), must have
/// non-negative baselines.
///
/// See also:
///
///  * `TextField`, which uses an [`InputDecorator`] to display a border,
///    labels, and icons, around its `EditableText` child.
///  * `Decoration` and `DecoratedBox`, for drawing arbitrary decorations
///    around other widgets.
#[derive(Clone)]
pub struct InputDecorator {
    pub key: Option<KeyRef>,

    /// The text and styles to use when decorating the child.
    ///
    /// Null [`InputDecoration`] properties are initialized with the corresponding
    /// values from the ambient [`InputDecorationThemeData`].
    pub decoration: InputDecoration,

    /// The style on which to base the label, hint, counter, and error styles
    /// if the [`decoration`](Self::decoration) does not provide explicit styles.
    ///
    /// If null, [`base_style`](Self::base_style) defaults to the `title_medium` style from the
    /// current `Theme`, see `ThemeData::text_theme`.
    ///
    /// The `TextStyle::text_baseline` of the [`base_style`](Self::base_style) is used to
    /// determine the baseline used for text alignment.
    pub base_style: Option<TextStyle>,

    /// How the text in the decoration should be aligned horizontally.
    pub text_align: Option<TextAlign>,

    /// How the text should be aligned vertically.
    ///
    /// Determines the alignment of the baseline within the available space of
    /// the input (typically a TextField). For example, `TextAlignVertical::TOP` will
    /// place the baseline such that the text, and any attached decoration like
    /// prefix and suffix, is as close to the top of the input as possible without
    /// overflowing. The heights of the prefix and suffix are similarly included
    /// for other alignment values. If the height is greater than the height
    /// available, then the prefix and suffix will be allowed to overflow first
    /// before the text scrolls.
    pub text_align_vertical: Option<TextAlignVertical>,

    /// Whether the input field has focus.
    ///
    /// Determines the position of the label text and the color and weight of the
    /// border.
    ///
    /// Defaults to false.
    ///
    /// See also:
    ///
    ///  * [`InputDecoration::hover_color`], which is also blended into the focus
    ///    color and fill color when the [`is_hovering`](Self::is_hovering) is true to produce
    ///    the final color.
    pub is_focused: bool,

    /// Whether the input field is being hovered over by a mouse pointer.
    ///
    /// Determines the container fill color, which is a blend of
    /// [`InputDecoration::hover_color`] with [`InputDecoration::fill_color`] when
    /// true, and [`InputDecoration::fill_color`] when not.
    ///
    /// Defaults to false.
    pub is_hovering: bool,

    /// If true, the height of the input field will be as large as possible.
    ///
    /// If wrapped in a widget that constrains its child's height, like Expanded
    /// or SizedBox, the input field will only be affected if [`expands`](Self::expands) is
    /// set to true.
    ///
    /// See `TextField::min_lines` and `TextField::max_lines` for related ways to
    /// affect the height of an input. When [`expands`](Self::expands) is true, both must be
    /// null in order to avoid ambiguity in determining the height.
    ///
    /// Defaults to false.
    pub expands: bool,

    /// Whether the input field is empty.
    ///
    /// Determines the position of the label text and whether to display the hint
    /// text.
    ///
    /// Defaults to false.
    pub is_empty: bool,

    /// The widget below this widget in the tree.
    ///
    /// Typically an `EditableText`, `DropdownButton`, or `InkWell`.
    pub child: Option<WidgetRef>,
}

impl InputDecorator {
    /// Creates a widget that displays a border, labels, and icons,
    /// for a `TextField`.
    pub fn new(decoration: InputDecoration) -> InputDecorator {
        InputDecorator {
            key: None,
            decoration,
            base_style: None,
            text_align: None,
            text_align_vertical: None,
            is_focused: false,
            is_hovering: false,
            expands: false,
            is_empty: false,
            child: None,
        }
    }

    /// Dart `InputDecorator(key:)`.
    pub fn key(mut self, key: KeyRef) -> InputDecorator {
        self.key = Some(key);
        self
    }

    /// Dart `InputDecorator(baseStyle:)`.
    pub fn base_style(mut self, base_style: TextStyle) -> InputDecorator {
        self.base_style = Some(base_style);
        self
    }

    /// Dart `InputDecorator(textAlign:)`.
    pub fn text_align(mut self, text_align: TextAlign) -> InputDecorator {
        self.text_align = Some(text_align);
        self
    }

    /// Dart `InputDecorator(textAlignVertical:)`.
    pub fn text_align_vertical(mut self, text_align_vertical: TextAlignVertical) -> InputDecorator {
        self.text_align_vertical = Some(text_align_vertical);
        self
    }

    /// Dart `InputDecorator(isFocused:)`.
    pub fn is_focused(mut self, is_focused: bool) -> InputDecorator {
        self.is_focused = is_focused;
        self
    }

    /// Dart `InputDecorator(isHovering:)`.
    pub fn is_hovering(mut self, is_hovering: bool) -> InputDecorator {
        self.is_hovering = is_hovering;
        self
    }

    /// Dart `InputDecorator(expands:)`.
    pub fn expands(mut self, expands: bool) -> InputDecorator {
        self.expands = expands;
        self
    }

    /// Dart `InputDecorator(isEmpty:)`.
    pub fn is_empty(mut self, is_empty: bool) -> InputDecorator {
        self.is_empty = is_empty;
        self
    }

    /// Dart `InputDecorator(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> InputDecorator {
        self.child = Some(child.into_widget());
        self
    }

    /// Whether the label needs to get out of the way of the input, either by
    /// floating or disappearing.
    ///
    /// Will withdraw when not empty or when focused while enabled.
    fn label_should_withdraw(&self) -> bool {
        !self.is_empty || (self.is_focused && self.decoration.enabled)
    }

    /// The RenderBox that defines this decorator's "container". That's the
    /// area which is filled if [`InputDecoration::filled`] is true. It's the area
    /// adjacent to [`InputDecoration::icon`] and above the widgets that contain
    /// [`InputDecoration::helper_text`], [`InputDecoration::error_text`], and
    /// [`InputDecoration::counter_text`].
    ///
    /// `TextField` renders ink splashes within the container.
    pub fn container_of(app: &App, context: BuildContext) -> Option<AnyRenderBox> {
        context
            .find_ancestor_render_object_of_type::<RenderDecoration>(app)
            .and_then(|result| result.container(app))
    }
}

impl StatefulWidget for InputDecorator {
    type State = InputDecoratorState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        self.decoration.debug_check_arguments();
    }

    fn create_state(&self) -> InputDecoratorState {
        InputDecoratorState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            floating_label_controller: None,
            floating_label_animation: None,
            shaking_label_controller: None,
            border_gap: None,
            effective_decoration: None,
            curved_animation: None,
        }
    }
}

impl Debug for InputDecorator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("InputDecorator");
        debug.field("decoration", &self.decoration);
        if let Some(base_style) = &self.base_style {
            debug.field("baseStyle", base_style);
        }
        debug.field("isFocused", &self.is_focused);
        if self.expands {
            debug.field("expands", &self.expands);
        }
        debug.field("isEmpty", &self.is_empty);
        debug.finish()
    }
}

/// Dart's `_InputDecoratorState`. Its `late final` fields and the `_InputBorderGap` its
/// field initializer makes are created in `init_state`, because an arena object needs the
/// [`App`].
pub struct InputDecoratorState {
    state: StateData<InputDecorator>,
    ticker_provider: TickerProviderStateMixinData,
    floating_label_controller: Option<Ref<AnimationController>>,
    floating_label_animation: Option<Ref<CurvedAnimation>>,
    shaking_label_controller: Option<Ref<AnimationController>>,
    border_gap: Option<Ref<InputBorderGap>>,
    effective_decoration: Option<InputDecoration>,
    curved_animation: Option<Ref<CurvedAnimation>>,
}

impl InputDecoratorState {
    fn floating_label_controller(self: Handle<Self>, app: &App) -> Ref<AnimationController> {
        app.get(self)
            .floating_label_controller
            .clone()
            .expect("created in init_state")
    }

    fn floating_label_animation(self: Handle<Self>, app: &App) -> Ref<CurvedAnimation> {
        app.get(self)
            .floating_label_animation
            .clone()
            .expect("created in init_state")
    }

    fn shaking_label_controller(self: Handle<Self>, app: &App) -> Ref<AnimationController> {
        app.get(self)
            .shaking_label_controller
            .clone()
            .expect("created in init_state")
    }

    fn border_gap(self: Handle<Self>, app: &App) -> Ref<InputBorderGap> {
        app.get(self)
            .border_gap
            .clone()
            .expect("created in init_state")
    }

    fn handle_change(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_state| {
            // The _floatingLabelController's value has changed.
        });
    }

    /// Dart's `decoration` getter: the widget's decoration with the ambient theme's
    /// defaults applied, worked out once until the widget or the dependencies change.
    fn decoration(self: Handle<Self>, app: &mut App) -> &InputDecoration {
        if app.get(self).effective_decoration.is_none() {
            let context = self.context(app);
            let theme = InputDecorationTheme::of(app, context);
            let effective = self.widget(app).decoration.apply_defaults(theme);
            app.get_mut(self).effective_decoration = Some(effective);
        }
        app.get(self)
            .effective_decoration
            .as_ref()
            .expect("worked out above")
    }

    fn text_align(self: Handle<Self>, app: &App) -> Option<TextAlign> {
        self.widget(app).text_align
    }

    fn is_focused(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).is_focused
    }

    fn has_error(self: Handle<Self>, app: &mut App) -> bool {
        let decoration = self.decoration(app);
        decoration.error_text.is_some() || decoration.error.is_some()
    }

    fn is_hovering(self: Handle<Self>, app: &mut App) -> bool {
        let enabled = self.decoration(app).enabled;
        self.widget(app).is_hovering && enabled
    }

    fn is_empty(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).is_empty
    }

    fn floating_label_enabled(self: Handle<Self>, app: &mut App) -> bool {
        self.decoration(app).floating_label_behavior != Some(FloatingLabelBehavior::Never)
    }

    fn label_should_withdraw(self: Handle<Self>, app: &mut App) -> bool {
        let always =
            self.decoration(app).floating_label_behavior == Some(FloatingLabelBehavior::Always);
        self.widget(app).label_should_withdraw() || always
    }

    fn get_default_m2_border_color(
        self: Handle<Self>,
        app: &mut App,
        theme_data: &ThemeData,
    ) -> Color {
        let is_focused = self.is_focused(app);
        let has_error = self.has_error(app);
        let is_hovering = self.is_hovering(app);
        let decoration = self.decoration(app);
        if !decoration.enabled && !is_focused {
            return if decoration.filled.unwrap_or(false)
                && !decoration
                    .border
                    .as_ref()
                    .is_some_and(|border| border.is_outline())
            {
                Colors::TRANSPARENT
            } else {
                theme_data.disabled_color
            };
        }
        if has_error {
            return theme_data.color_scheme.error;
        }
        if is_focused {
            return theme_data.color_scheme.primary;
        }
        if decoration.filled.expect("applied from the theme") {
            return theme_data.hint_color;
        }
        // Dart's `withOpacity`, deprecated there too: its rounding to an 8-bit alpha is part of
        // the behaviour being ported.
        #[allow(deprecated)]
        let enabled_color = theme_data.color_scheme.on_surface.with_opacity(0.38);
        if is_hovering {
            let hover_color = decoration.hover_color.unwrap_or(theme_data.hover_color);
            #[allow(deprecated)]
            return Color::alpha_blend(hover_color.with_opacity(0.12), enabled_color);
        }
        enabled_color
    }

    fn get_fill_color(
        self: Handle<Self>,
        app: &mut App,
        _theme_data: &ThemeData,
        defaults: &dyn InputDecorationThemeDataGetters,
    ) -> Color {
        let widget_state = self.widget_state(app);
        let decoration = self.decoration(app);
        if decoration.filled != Some(true) {
            // filled == null same as filled == false
            return Colors::TRANSPARENT;
        }
        if let Some(fill_color) = &decoration.fill_color {
            return *<dyn WidgetStateProperty<Color>>::resolve_as(fill_color, &widget_state);
        }
        *<dyn WidgetStateProperty<Color>>::resolve_as(
            defaults
                .fill_color(app)
                .as_ref()
                .expect("the defaults give one"),
            &widget_state,
        )
    }

    fn get_hover_color(self: Handle<Self>, app: &mut App, theme_data: &ThemeData) -> Color {
        let decoration = self.decoration(app);
        if decoration.filled.is_none() || !decoration.filled.unwrap_or(false) || !decoration.enabled
        {
            return Colors::TRANSPARENT;
        }
        decoration.hover_color.unwrap_or(theme_data.hover_color)
    }

    fn get_icon_color(
        self: Handle<Self>,
        app: &mut App,
        _theme_data: &ThemeData,
        defaults: &dyn InputDecorationThemeDataGetters,
    ) -> Color {
        let widget_state = self.widget_state(app);
        let decoration = self.decoration(app);
        <dyn WidgetStateProperty<Color>>::resolve_as(&decoration.icon_color, &widget_state)
            .map(|color| *color)
            .unwrap_or_else(|| {
                *<dyn WidgetStateProperty<Color>>::resolve_as(
                    defaults
                        .icon_color(app)
                        .as_ref()
                        .expect("the defaults give one"),
                    &widget_state,
                )
            })
    }

    fn get_prefix_icon_color(
        self: Handle<Self>,
        app: &mut App,
        icon_button_theme: &IconButtonThemeData,
        defaults: &dyn InputDecorationThemeDataGetters,
    ) -> Color {
        let widget_state = self.widget_state(app);
        let decoration = self.decoration(app);
        <dyn WidgetStateProperty<Color>>::resolve_as(&decoration.prefix_icon_color, &widget_state)
            .map(|color| *color)
            .or_else(|| {
                icon_button_theme
                    .style
                    .as_ref()
                    .and_then(|style| style.foreground_color.as_ref())
                    .and_then(|foreground_color| foreground_color.resolve(&widget_state))
            })
            .unwrap_or_else(|| {
                *<dyn WidgetStateProperty<Color>>::resolve_as(
                    defaults
                        .prefix_icon_color(app)
                        .as_ref()
                        .expect("the defaults give one"),
                    &widget_state,
                )
            })
    }

    fn get_suffix_icon_color(
        self: Handle<Self>,
        app: &mut App,
        icon_button_theme: &IconButtonThemeData,
        defaults: &dyn InputDecorationThemeDataGetters,
    ) -> Color {
        let widget_state = self.widget_state(app);
        let decoration = self.decoration(app);
        <dyn WidgetStateProperty<Color>>::resolve_as(&decoration.suffix_icon_color, &widget_state)
            .map(|color| *color)
            .or_else(|| {
                icon_button_theme
                    .style
                    .as_ref()
                    .and_then(|style| style.foreground_color.as_ref())
                    .and_then(|foreground_color| foreground_color.resolve(&widget_state))
            })
            .unwrap_or_else(|| {
                *<dyn WidgetStateProperty<Color>>::resolve_as(
                    defaults
                        .suffix_icon_color(app)
                        .as_ref()
                        .expect("the defaults give one"),
                    &widget_state,
                )
            })
    }

    // True if the label will be shown and the hint will not.
    // If we're not focused, there's no value, labelText was provided, and
    // floatingLabelBehavior isn't set to always, then the label appears where the
    // hint would.
    fn has_inline_label(self: Handle<Self>, app: &mut App) -> bool {
        let label_should_withdraw = self.label_should_withdraw(app);
        let decoration = self.decoration(app);
        !label_should_withdraw && (decoration.label_text.is_some() || decoration.label.is_some())
    }

    // If the label is a floating placeholder, it's always shown.
    fn should_show_label(self: Handle<Self>, app: &mut App) -> bool {
        self.has_inline_label(app) || self.floating_label_enabled(app)
    }

    // The base style for the inline label when they're displayed "inline",
    // i.e. when they appear in place of the empty text field.
    fn get_inline_label_style(
        self: Handle<Self>,
        app: &mut App,
        theme_data: &ThemeData,
        defaults: &dyn InputDecorationThemeDataGetters,
    ) -> TextStyle {
        let widget_state = self.widget_state(app);
        let default_style = <dyn WidgetStateProperty<TextStyle>>::resolve_as(
            defaults
                .label_style(app)
                .as_ref()
                .expect("the defaults give one"),
            &widget_state,
        );

        let style = <dyn WidgetStateProperty<TextStyle>>::resolve_as(
            &self.decoration(app).label_style,
            &widget_state,
        );

        theme_data
            .text_theme
            .title_medium
            .as_ref()
            .expect("the theme's text theme gives titleMedium")
            .merge(self.widget(app).base_style.as_ref())
            .merge(Some(&default_style))
            .merge(style.as_deref())
            .copy_with()
            .height(1.0)
    }

    // The base style for the inline hint when they're displayed "inline",
    // i.e. when they appear in place of the empty text field.
    fn get_inline_hint_style(
        self: Handle<Self>,
        app: &mut App,
        theme_data: &ThemeData,
        defaults: &dyn InputDecorationThemeDataGetters,
    ) -> TextStyle {
        let widget_state = self.widget_state(app);
        let default_style = <dyn WidgetStateProperty<TextStyle>>::resolve_as(
            defaults
                .hint_style(app)
                .as_ref()
                .expect("the defaults give one"),
            &widget_state,
        );

        let style = <dyn WidgetStateProperty<TextStyle>>::resolve_as(
            &self.decoration(app).hint_style,
            &widget_state,
        );

        (if theme_data.use_material3 {
            theme_data.text_theme.body_large.as_ref()
        } else {
            theme_data.text_theme.title_medium.as_ref()
        })
        .expect("the theme's text theme gives the style")
        .merge(self.widget(app).base_style.as_ref())
        .merge(Some(&default_style))
        .merge(style.as_deref())
    }

    fn get_floating_label_style(
        self: Handle<Self>,
        app: &mut App,
        theme_data: &ThemeData,
        defaults: &dyn InputDecorationThemeDataGetters,
    ) -> TextStyle {
        let widget_state = self.widget_state(app);
        let has_error = self.has_error(app);
        let mut default_text_style: TextStyle = <dyn WidgetStateProperty<TextStyle>>::resolve_as(
            defaults
                .floating_label_style(app)
                .as_ref()
                .expect("the defaults give one"),
            &widget_state,
        )
        .value()
        .clone();
        let decoration = self.decoration(app);
        if has_error
            && let Some(error_color) = decoration
                .error_style
                .as_ref()
                .and_then(|error_style| error_style.color.clone())
        {
            default_text_style = default_text_style.copy_with().color(error_color);
        }
        default_text_style = default_text_style.merge(
            decoration
                .floating_label_style
                .as_deref()
                .or(decoration.label_style.as_deref()),
        );

        let style = <dyn WidgetStateProperty<TextStyle>>::resolve_as(
            &decoration.floating_label_style,
            &widget_state,
        );

        theme_data
            .text_theme
            .title_medium
            .as_ref()
            .expect("the theme's text theme gives titleMedium")
            .merge(self.widget(app).base_style.as_ref())
            .merge(Some(&default_text_style))
            .merge(style.as_deref())
            .copy_with()
            .height(1.0)
    }

    fn get_helper_style(
        self: Handle<Self>,
        app: &mut App,
        _theme_data: &ThemeData,
        defaults: &dyn InputDecorationThemeDataGetters,
    ) -> TextStyle {
        let widget_state = self.widget_state(app);
        let style = <dyn WidgetStateProperty<TextStyle>>::resolve_as(
            &self.decoration(app).helper_style,
            &widget_state,
        );
        <dyn WidgetStateProperty<TextStyle>>::resolve_as(
            defaults
                .helper_style(app)
                .as_ref()
                .expect("the defaults give one"),
            &widget_state,
        )
        .merge(style.as_deref())
    }

    fn get_error_style(
        self: Handle<Self>,
        app: &mut App,
        _theme_data: &ThemeData,
        defaults: &dyn InputDecorationThemeDataGetters,
    ) -> TextStyle {
        let widget_state = self.widget_state(app);
        let error_style = self.decoration(app).error_style.clone();
        <dyn WidgetStateProperty<TextStyle>>::resolve_as(
            defaults
                .error_style(app)
                .as_ref()
                .expect("the defaults give one"),
            &widget_state,
        )
        .merge(error_style.as_deref())
    }

    fn widget_state(self: Handle<Self>, app: &mut App) -> WidgetStates {
        let enabled = self.decoration(app).enabled;
        let is_focused = self.is_focused(app);
        let is_hovering = self.is_hovering(app);
        let has_error = self.has_error(app);
        let mut states = WidgetStates::new();
        if !enabled {
            states.insert(WidgetState::Disabled);
        }
        if is_focused {
            states.insert(WidgetState::Focused);
        }
        if is_hovering {
            states.insert(WidgetState::Hovered);
        }
        if has_error {
            states.insert(WidgetState::Error);
        }
        states
    }

    fn get_default_border(
        self: Handle<Self>,
        app: &mut App,
        theme_data: &ThemeData,
        defaults: &dyn InputDecorationThemeDataGetters,
    ) -> InputBorderRef {
        let widget_state = self.widget_state(app);
        let decoration = self.decoration(app).clone();
        let border: InputBorderRef = <dyn WidgetStateProperty<InputBorderRef>>::resolve_as(
            &decoration.border,
            &widget_state,
        )
        .unwrap_or_else(|| InputBorderRef::new(UnderlineInputBorder::new()));

        if decoration
            .border
            .as_ref()
            .is_some_and(|border| border.as_widget_state_property().is_some())
        {
            return border;
        }

        if border.border_side() == BorderSide::NONE {
            return border;
        }

        if theme_data.use_material3 {
            if decoration.filled.expect("applied from the theme") {
                let context = self.context(app);
                let decoration_theme = InputDecorationTheme::of(app, context);
                let active_indicator_border = decoration_theme
                    .active_indicator_border
                    .clone()
                    .or_else(|| defaults.active_indicator_border(app));
                InputBorder::copy_with(
                    &*border,
                    <dyn WidgetStateProperty<Option<BorderSide>>>::resolve_as(
                        &active_indicator_border,
                        &widget_state,
                    )
                    .map(|side| *side),
                )
            } else {
                InputBorder::copy_with(
                    &*border,
                    <dyn WidgetStateProperty<Option<BorderSide>>>::resolve_as(
                        &defaults.outline_border(app),
                        &widget_state,
                    )
                    .map(|side| *side),
                )
            }
        } else {
            let is_focused = self.is_focused(app);
            let color = self.get_default_m2_border_color(app, theme_data);
            let width = if decoration.is_collapsed.expect("applied from the theme")
                || decoration
                    .border
                    .as_ref()
                    .is_some_and(|border| **border == *InputBorderRef::none())
                || !decoration.enabled
            {
                0.0
            } else if is_focused {
                2.0
            } else {
                1.0
            };
            InputBorder::copy_with(
                &*border,
                Some(BorderSide {
                    color,
                    width,
                    ..BorderSide::default()
                }),
            )
        }
    }

    fn build_transition(
        self: Handle<Self>,
        app: &mut App,
        child: &WidgetRef,
        animation: AnimationRef<f64>,
    ) -> WidgetRef {
        let current = app.get(self).curved_animation.handle();
        if current.is_none_or(|curved| curved.parent(app) != animation) {
            if let Some(curved) = current {
                curved.dispose(app);
            }
            let curved = CurvedAnimation::new(app, animation, k_transition_curve(), None);
            app.get_mut(self).curved_animation = Some(curved);
        }

        let curved = app.get(self).curved_animation.clone().expect("made above");
        FadeTransition::new(curved.erase())
            .child(child.clone())
            .into_widget()
    }

    fn top_start_layout(
        _app: &mut App,
        current_child: Option<&WidgetRef>,
        previous_children: &[WidgetRef],
    ) -> WidgetRef {
        Stack::new()
            .children(
                previous_children
                    .iter()
                    .cloned()
                    .chain(current_child.cloned()),
            )
            .into_widget()
    }
}

impl TickerProviderStateMixin for InputDecoratorState {
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

/// Dart's `vsync: this`.
impl TickerProviderObject for InputDecoratorState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for InputDecoratorState {
    type Widget = InputDecorator;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let border_gap = InputBorderGap::new(app);
        app.get_mut(self).border_gap = Some(border_gap);

        let floating_label_controller = AnimationController::new(
            app,
            None,
            Some(K_TRANSITION_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        floating_label_controller
            .add_listener(app, Listener::handle_method(self, Self::handle_change));
        let floating_label_animation = CurvedAnimation::new(
            app,
            floating_label_controller.erase(),
            k_transition_curve(),
            Some(Rc::new(k_transition_curve().flipped())),
        );

        let shaking_label_controller = AnimationController::new(
            app,
            None,
            Some(K_TRANSITION_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let state = app.get_mut(self);
        state.floating_label_controller = Some(floating_label_controller);
        state.floating_label_animation = Some(floating_label_animation);
        state.shaking_label_controller = Some(shaking_label_controller);
    }

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).effective_decoration = None;

        let label_is_initially_floating = self.decoration(app).floating_label_behavior
            != Some(FloatingLabelBehavior::Never)
            && self.label_should_withdraw(app);
        let floating_label_controller = self.floating_label_controller(app);
        floating_label_controller.set_value(
            app,
            if label_is_initially_floating {
                1.0
            } else {
                0.0
            },
        );
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let floating_label_controller = self.floating_label_controller(app);
        let floating_label_animation = self.floating_label_animation(app);
        let shaking_label_controller = self.shaking_label_controller(app);
        let border_gap = self.border_gap(app);
        let curved_animation = app.get(self).curved_animation.handle();
        floating_label_controller.dispose(app);
        floating_label_animation.dispose(app);
        shaking_label_controller.dispose(app);
        ChangeNotifierData::dispose(app, &border_gap);
        if let Some(curved_animation) = curved_animation {
            curved_animation.dispose(app);
        }
        TickerProviderStateMixin::dispose(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &InputDecorator) {
        if self.widget(app).decoration != old.decoration {
            app.get_mut(self).effective_decoration = None;
        }

        let widget = self.widget(app);
        let float_behavior_changed =
            widget.decoration.floating_label_behavior != old.decoration.floating_label_behavior;

        if widget.label_should_withdraw() != old.label_should_withdraw() || float_behavior_changed {
            let floating_label_controller = self.floating_label_controller(app);
            if self.floating_label_enabled(app) && self.label_should_withdraw(app) {
                floating_label_controller.forward(app, None);
            } else {
                floating_label_controller.reverse(app, None);
            }
        }

        let error_text = self.decoration(app).error_text.clone();
        let old_error_text = &old.decoration.error_text;

        let floating_label_controller = self.floating_label_controller(app);
        if floating_label_controller.is_completed(app)
            && error_text.is_some()
            && error_text != *old_error_text
        {
            let shaking_label_controller = self.shaking_label_controller(app);
            shaking_label_controller.set_value(app, 0.0);
            shaking_label_controller.forward(app, None);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme_data = Theme::of(app, context);
        let visual_density = self
            .decoration(app)
            .visual_density
            .unwrap_or(theme_data.visual_density);
        let use_material3 = Theme::of(app, context).use_material3;
        let defaults: Box<dyn InputDecorationThemeDataGetters> = if use_material3 {
            Box::new(InputDecoratorDefaultsM3::new(context))
        } else {
            Box::new(InputDecoratorDefaultsM2::new(context))
        };
        let icon_button_theme = IconButtonTheme::of(app, context);

        let label_style = self.get_inline_label_style(app, &theme_data, &*defaults);
        let text_baseline = label_style
            .text_baseline
            .expect("the label style gives a text baseline");

        let hint_style = self.get_inline_hint_style(app, &theme_data, &*defaults);
        let is_empty = self.is_empty(app);
        let has_inline_label = self.has_inline_label(app);
        let text_align = self.text_align(app);
        let decoration = self.decoration(app).clone();
        let hint_text = decoration.hint_text.clone();
        let maintain_hint_size = decoration.maintain_hint_size;
        let mut hint: Option<WidgetRef> = None;
        if decoration.hint.is_some() || hint_text.is_some() {
            let hint_widget = decoration.hint.clone().unwrap_or_else(|| {
                let mut text = Text::new(hint_text.clone().expect("a hint or its text is given"))
                    .style(hint_style.clone());
                if let Some(text_direction) = decoration.hint_text_direction {
                    text = text.text_direction(text_direction);
                }
                let overflow = hint_style
                    .overflow
                    .or(if decoration.hint_max_lines.is_none() {
                        None
                    } else {
                        Some(TextOverflow::Ellipsis)
                    });
                if let Some(overflow) = overflow {
                    text = text.overflow(overflow);
                }
                if let Some(text_align) = text_align {
                    text = text.text_align(text_align);
                }
                if let Some(max_lines) = decoration.hint_max_lines {
                    text = text.max_lines(max_lines);
                }
                text.into_widget()
            });
            let show_hint = is_empty && !has_inline_label;
            let hint_fade_duration = decoration
                .hint_fade_duration
                .unwrap_or(K_HINT_FADE_TRANSITION_DURATION);
            hint = Some(if maintain_hint_size {
                AnimatedOpacity::new(if show_hint { 1.0 } else { 0.0 }, hint_fade_duration)
                    .curve(k_transition_curve())
                    .child(hint_widget)
                    .into_widget()
            } else {
                AnimatedSwitcher::new(hint_fade_duration)
                    .transition_builder(AnimatedSwitcherTransitionBuilder::handle_method(
                        self,
                        Self::build_transition,
                    ))
                    .layout_builder(Self::top_start_layout)
                    .child(if show_hint {
                        hint_widget
                    } else {
                        SizedBox::shrink().into_widget()
                    })
                    .into_widget()
            });
        }

        let is_focused = self.is_focused(app);
        let has_error = self.has_error(app);
        let mut border: Option<InputBorderRef> = if !decoration.enabled {
            if has_error {
                decoration.error_border.clone()
            } else {
                decoration.disabled_border.clone()
            }
        } else if is_focused {
            if has_error {
                decoration.focused_error_border.clone()
            } else {
                decoration.focused_border.clone()
            }
        } else if has_error {
            decoration.error_border.clone()
        } else {
            decoration.enabled_border.clone()
        };
        let border = border
            .take()
            .unwrap_or_else(|| self.get_default_border(app, &theme_data, &*defaults));

        let fill_color = self.get_fill_color(app, &theme_data, &*defaults);
        let hover_color = self.get_hover_color(app, &theme_data);
        let is_hovering = self.is_hovering(app);
        let container = BorderContainer {
            border: border.clone(),
            gap: self.border_gap(app),
            gap_animation: self.floating_label_animation(app).erase(),
            fill_color,
            hover_color,
            is_hovering,
        }
        .into_widget();

        let label_should_withdraw = self.label_should_withdraw(app);
        let mut label: Option<WidgetRef> = None;
        if decoration.label_text.is_some() || decoration.label.is_some() {
            let should_show_label = self.should_show_label(app);
            let style = if label_should_withdraw {
                self.get_floating_label_style(app, &theme_data, &*defaults)
            } else {
                label_style.clone()
            };
            let label_child = decoration.label.clone().unwrap_or_else(|| {
                let mut text = Text::new(
                    decoration
                        .label_text
                        .clone()
                        .expect("a label or its text is given"),
                )
                .overflow(TextOverflow::Ellipsis);
                if let Some(text_align) = text_align {
                    text = text.text_align(text_align);
                }
                if let Some(text_direction) = decoration.label_text_direction {
                    text = text.text_direction(text_direction);
                }
                text.into_widget()
            });
            let shaking_label_controller = self.shaking_label_controller(app);
            label = Some(
                MatrixTransition::new(shaking_label_controller.erase(), |value: f64| {
                    let shake_offset = if value <= 0.25 {
                        -value
                    } else if value < 0.75 {
                        value - 0.5
                    } else {
                        (1.0 - value) * 4.0
                    };
                    // Shakes the floating label to the left and right
                    // when the errorText first appears.
                    Matrix4::translation((shake_offset * 4.0) as f32, 0.0)
                })
                .child(
                    AnimatedOpacity::new(
                        if should_show_label { 1.0 } else { 0.0 },
                        K_TRANSITION_DURATION,
                    )
                    .curve(k_transition_curve())
                    .child(
                        AnimatedDefaultTextStyle::new(label_child, style, K_TRANSITION_DURATION)
                            .curve(k_transition_curve()),
                    ),
                )
                .into_widget(),
            );
        }

        let has_prefix = decoration.prefix.is_some() || decoration.prefix_text.is_some();
        let has_suffix = decoration.suffix.is_some() || decoration.suffix_text.is_some();

        // `Semantics(sortKey:)` around the input and the sort keys of the affixes, which Dart
        // sets when at least two of the three are visible, wait with accessibility.
        let input = self.widget(app).child.clone();

        let widget_state = self.widget_state(app);
        let prefix = has_prefix.then(|| {
            AffixText {
                label_is_floating: label_should_withdraw,
                text: decoration.prefix_text.clone(),
                style: Some(
                    <dyn WidgetStateProperty<TextStyle>>::resolve_as(
                        &decoration.prefix_style,
                        &widget_state,
                    )
                    .map(|style| style.value().clone())
                    .unwrap_or_else(|| hint_style.clone()),
                ),
                child: decoration.prefix.clone(),
            }
            .into_widget()
        });

        let suffix = has_suffix.then(|| {
            AffixText {
                label_is_floating: label_should_withdraw,
                text: decoration.suffix_text.clone(),
                style: Some(
                    <dyn WidgetStateProperty<TextStyle>>::resolve_as(
                        &decoration.suffix_style,
                        &widget_state,
                    )
                    .map(|style| style.value().clone())
                    .unwrap_or_else(|| hint_style.clone()),
                ),
                child: decoration.suffix.clone(),
            }
            .into_widget()
        });

        let decoration_is_dense = decoration.is_dense.unwrap_or(false);
        let icon_size = if decoration_is_dense { 18.0 } else { 24.0 };

        let icon = match &decoration.icon {
            None => None,
            Some(decoration_icon) => {
                let icon_color = self.get_icon_color(app, &theme_data, &*defaults);
                Some(
                    MouseRegion::new()
                        .cursor(SystemMouseCursors::BASIC.into())
                        .child(
                            Padding::new(EdgeInsetsGeometry::directional(0.0, 0.0, 16.0, 0.0))
                                .child(IconTheme::merge(
                                    None,
                                    IconThemeData::new().color(icon_color).size(icon_size),
                                    decoration_icon.clone(),
                                )),
                        )
                        .into_widget(),
                )
            }
        };

        let prefix_icon = match &decoration.prefix_icon {
            None => None,
            Some(decoration_prefix_icon) => {
                let prefix_icon_color =
                    self.get_prefix_icon_color(app, &icon_button_theme, &*defaults);
                Some(Self::build_affix_icon(
                    decoration_prefix_icon.clone(),
                    decoration.prefix_icon_constraints,
                    visual_density,
                    prefix_icon_color,
                    icon_size,
                    &icon_button_theme,
                ))
            }
        };

        let suffix_icon = match &decoration.suffix_icon {
            None => None,
            Some(decoration_suffix_icon) => {
                let suffix_icon_color =
                    self.get_suffix_icon_color(app, &icon_button_theme, &*defaults);
                Some(Self::build_affix_icon(
                    decoration_suffix_icon.clone(),
                    decoration.suffix_icon_constraints,
                    visual_density,
                    suffix_icon_color,
                    icon_size,
                    &icon_button_theme,
                ))
            }
        };

        let helper_style = self.get_helper_style(app, &theme_data, &*defaults);
        let error_style = self.get_error_style(app, &theme_data, &*defaults);
        let helper_error = HelperError {
            text_align,
            helper: decoration.helper.clone(),
            helper_text: decoration.helper_text.clone(),
            helper_style: Some(helper_style.clone()),
            helper_max_lines: decoration.helper_max_lines,
            error: decoration.error.clone(),
            error_text: decoration.error_text.clone(),
            error_style: Some(error_style),
            error_max_lines: decoration.error_max_lines,
        }
        .into_widget();

        let mut counter: Option<WidgetRef> = None;
        if decoration.counter.is_some() {
            counter = decoration.counter.clone();
        } else if let Some(counter_text) = decoration
            .counter_text
            .as_ref()
            .filter(|counter_text| !counter_text.is_empty())
        {
            // `Semantics(container:, liveRegion:)` and the counter's `semanticsLabel`
            // (`semantic_counter_text`) wait with accessibility.
            let counter_style = <dyn WidgetStateProperty<TextStyle>>::resolve_as(
                &decoration.counter_style,
                &widget_state,
            );
            counter = Some(
                Text::new(counter_text.clone())
                    .style(helper_style.merge(counter_style.as_deref()))
                    .overflow(TextOverflow::Ellipsis)
                    .into_widget(),
            );
        }

        // The _Decoration widget and _RenderDecoration assume that contentPadding
        // has been resolved to EdgeInsets.
        let text_direction = Directionality::of(app, context);
        let flip_horizontal = match text_direction {
            TextDirection::Ltr => false,
            TextDirection::Rtl => true,
        };
        let resolved_padding = decoration
            .content_padding
            .map(|padding| padding.resolve(Some(text_direction)));
        let decoration_content_padding = resolved_padding.map(|resolved_padding| {
            EdgeInsetsDirectional::from_steb(
                if flip_horizontal {
                    resolved_padding.right
                } else {
                    resolved_padding.left
                },
                resolved_padding.top,
                if flip_horizontal {
                    resolved_padding.left
                } else {
                    resolved_padding.right
                },
                resolved_padding.bottom,
            )
        });

        let resolved_supporting_text_padding = decoration
            .supporting_text_padding
            .map(|padding| padding.resolve(Some(text_direction)));
        let decoration_supporting_text_padding =
            resolved_supporting_text_padding.map(|resolved_supporting_text_padding| {
                EdgeInsetsDirectional::from_steb(
                    if flip_horizontal {
                        resolved_supporting_text_padding.right
                    } else {
                        resolved_supporting_text_padding.left
                    },
                    resolved_supporting_text_padding.top,
                    if flip_horizontal {
                        resolved_supporting_text_padding.left
                    } else {
                        resolved_supporting_text_padding.right
                    },
                    resolved_supporting_text_padding.bottom,
                )
            });

        let content_padding: EdgeInsetsDirectional;
        let floating_label_height: f64;

        if decoration.is_collapsed.expect("applied from the theme") {
            floating_label_height = 0.0;
            content_padding = decoration_content_padding.unwrap_or(EdgeInsetsDirectional::ZERO);
        } else if !border.is_outline() {
            // 4.0: the vertical gap between the inline elements and the floating label.
            floating_label_height = MediaQuery::text_scaler_of(app, context).scale(
                4.0 + 0.75
                    * label_style
                        .font_size
                        .expect("the label style gives a font size"),
            );
            if decoration.filled.unwrap_or(false) {
                content_padding = decoration_content_padding.unwrap_or(if use_material3 {
                    if decoration_is_dense {
                        EdgeInsetsDirectional::from_steb(12.0, 4.0, 12.0, 4.0)
                    } else {
                        EdgeInsetsDirectional::from_steb(12.0, 8.0, 12.0, 8.0)
                    }
                } else if decoration_is_dense {
                    EdgeInsetsDirectional::from_steb(12.0, 8.0, 12.0, 8.0)
                } else {
                    EdgeInsetsDirectional::from_steb(12.0, 12.0, 12.0, 12.0)
                });
            } else {
                // No left or right padding for underline borders that aren't filled
                // is a small concession to backwards compatibility. This eliminates
                // the most noticeable layout change introduced by #13734.
                content_padding = decoration_content_padding.unwrap_or(if use_material3 {
                    if decoration_is_dense {
                        EdgeInsetsDirectional::from_steb(0.0, 4.0, 0.0, 4.0)
                    } else {
                        EdgeInsetsDirectional::from_steb(0.0, 8.0, 0.0, 8.0)
                    }
                } else if decoration_is_dense {
                    EdgeInsetsDirectional::from_steb(0.0, 8.0, 0.0, 8.0)
                } else {
                    EdgeInsetsDirectional::from_steb(0.0, 12.0, 0.0, 12.0)
                });
            }
        } else {
            floating_label_height = 0.0;
            content_padding = decoration_content_padding.unwrap_or(if use_material3 {
                if decoration_is_dense {
                    EdgeInsetsDirectional::from_steb(12.0, 16.0, 12.0, 8.0)
                } else {
                    EdgeInsetsDirectional::from_steb(12.0, 20.0, 12.0, 12.0)
                }
            } else if decoration_is_dense {
                EdgeInsetsDirectional::from_steb(12.0, 20.0, 12.0, 12.0)
            } else {
                EdgeInsetsDirectional::from_steb(12.0, 24.0, 12.0, 16.0)
            });
        }

        let mut input_gap = 0.0;
        if use_material3 {
            if let Some(outline) = border.as_any().downcast_ref::<OutlineInputBorder>() {
                input_gap = outline.gap_padding;
            } else {
                input_gap = if border.is_outline() || decoration.filled.unwrap_or(false) {
                    K_INPUT_EXTRA_PADDING
                } else {
                    0.0
                };
            }
        }

        let floating_label_progress = self.floating_label_animation(app).value(app);
        let border_gap = self.border_gap(app);
        let decorator = Decorator {
            decoration: Decoration {
                content_padding,
                is_collapsed: decoration.is_collapsed.expect("applied from the theme"),
                input_gap,
                floating_label_height,
                floating_label_alignment: decoration
                    .floating_label_alignment
                    .expect("applied from the theme"),
                floating_label_progress,
                border,
                border_gap,
                align_label_with_hint: decoration.align_label_with_hint.unwrap_or(false),
                is_dense: decoration.is_dense,
                is_empty,
                visual_density,
                maintain_hint_size,
                maintain_label_size: decoration.maintain_label_size,
                icon,
                input,
                label,
                hint,
                prefix,
                suffix,
                prefix_icon,
                suffix_icon,
                helper_error: Some(helper_error),
                counter,
                container: Some(container),
                supporting_text_padding: decoration_supporting_text_padding,
            },
            text_direction,
            text_baseline,
            text_align_vertical: self.widget(app).text_align_vertical,
            is_focused,
            expands: self.widget(app).expands,
        };

        // `Semantics(hint: decoration.errorText)` around the decorator waits with
        // accessibility.
        let result = decorator.into_widget();

        if let Some(constraints) = decoration.constraints {
            return ConstrainedBox::new(constraints).child(result).into_widget();
        }
        result
    }
}

impl InputDecoratorState {
    /// The `prefixIcon` and `suffixIcon` widgets of Dart's `build`, which differ only in their
    /// icon, constraints and colour.
    fn build_affix_icon(
        affix_icon: WidgetRef,
        constraints: Option<BoxConstraints>,
        visual_density: VisualDensity,
        color: Color,
        icon_size: f64,
        icon_button_theme: &IconButtonThemeData,
    ) -> WidgetRef {
        // `Semantics(tagForChildren:)` around the icon waits with accessibility.
        Center::new()
            .width_factor(1.0)
            .height_factor(1.0)
            .child(
                MouseRegion::new()
                    .cursor(SystemMouseCursors::BASIC.into())
                    .child(
                        ConstrainedBox::new(constraints.unwrap_or_else(|| {
                            visual_density.effective_constraints(BoxConstraints {
                                min_width: K_MIN_INTERACTIVE_DIMENSION,
                                min_height: K_MIN_INTERACTIVE_DIMENSION,
                                ..BoxConstraints::new()
                            })
                        }))
                        .child(IconTheme::merge(
                            None,
                            IconThemeData::new().color(color).size(icon_size),
                            IconButtonTheme::new(
                                IconButtonThemeData::new().style(
                                    ButtonStyle::new()
                                        .foreground_color(Rc::new(WidgetStatePropertyAll::new(
                                            Some(color),
                                        ))
                                            as WidgetStatePropertyRef<Option<Color>>)
                                        .icon_size(Rc::new(WidgetStatePropertyAll::new(Some(
                                            icon_size,
                                        )))
                                            as WidgetStatePropertyRef<Option<f64>>)
                                        .merge(icon_button_theme.style.as_ref()),
                                ),
                                affix_icon,
                            ),
                        )),
                    ),
            )
            .into_widget()
    }
}

impl Debug for InputDecoratorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_InputDecoratorState")
            .field("floating_label_controller", &self.floating_label_controller)
            .finish_non_exhaustive()
    }
}

/// The border, labels, icons, and styles used to decorate a Material
/// Design text field.
///
/// The `TextField` and `InputDecorator` classes use `InputDecoration` objects
/// to describe their decoration. (In fact, this class is merely the
/// configuration of an `InputDecorator`, which does all the heavy lifting.)
///
/// See also:
///
///  * `TextField`, which is a text input widget that uses an
///    `InputDecoration`.
///  * `InputDecorator`, which is a widget that draws an `InputDecoration`
///    around an input child widget.
///  * `Decoration` and `DecoratedBox`, for drawing borders and backgrounds
///    around a child widget.
///
/// Dart's two constructors are [`new`](Self::new) and [`collapsed`](Self::collapsed), and
/// their named arguments the fluent setters, which every decoration has. Dart's constructor
/// asserts that relate two arguments run in `debug_check_arguments`, which `InputDecorator`
/// calls when it is turned into a `WidgetRef` (`into_widget`).
#[derive(Clone)]
pub struct InputDecoration {
    /// An icon to show before the input field and outside of the decoration's
    /// container.
    ///
    /// The size and color of the icon is configured automatically using an
    /// `IconTheme` and therefore does not need to be explicitly given in the
    /// icon widget.
    ///
    /// The trailing edge of the icon is padded by 16dps.
    ///
    /// The decoration's container is the area which is filled if `filled` is
    /// true and bordered per the `border`. It's the area adjacent to
    /// `icon` and above the widgets that contain `helper_text`,
    /// `error_text`, and `counter_text`.
    ///
    /// See `Icon`, `ImageIcon`.
    pub icon: Option<WidgetRef>,

    /// The color of the `icon`.
    ///
    /// If `icon_color` is a `WidgetStateColor`, then the effective
    /// color can depend on the `WidgetState::Focused` state, i.e.
    /// if the `TextField` is focused or not.
    pub icon_color: Option<AnyColor>,

    /// Optional widget that describes the input field.
    ///
    /// When the input field is empty and unfocused, the label is displayed on
    /// top of the input field (i.e., at the same location on the screen where
    /// text may be entered in the input field). When the input field receives
    /// focus (or if the field is non-empty), depending on `floating_label_alignment`,
    /// the label moves above, either vertically adjacent to, or to the center of
    /// the input field.
    ///
    /// This can be used, for example, to add multiple `TextStyle`'s to a label that would
    /// otherwise be specified using `label_text`, which only takes one `TextStyle`.
    ///
    /// Only one of `label` and `label_text` can be specified.
    pub label: Option<WidgetRef>,

    /// Optional text that describes the input field.
    ///
    /// When the input field is empty and unfocused, the label is displayed on
    /// top of the input field (i.e., at the same location on the screen where
    /// text may be entered in the input field). When the input field receives
    /// focus (or if the field is non-empty), depending on `floating_label_alignment`,
    /// the label moves above, either vertically adjacent to, or to the center of
    /// the input field.
    ///
    /// If a more elaborate label is required, consider using `label` instead.
    /// Only one of `label` and `label_text` can be specified.
    pub label_text: Option<String>,

    /// The style to use for `InputDecoration::label_text` when the label is on top
    /// of the input field.
    ///
    /// If `label_style` is a `WidgetStateTextStyle`, then the effective
    /// text style can depend on the `WidgetState::Focused` state, i.e.
    /// if the `TextField` is focused or not.
    ///
    /// When the `InputDecoration::label_text` is above (i.e., vertically adjacent to)
    /// the input field, the text uses the `floating_label_style` instead.
    ///
    /// If null, defaults to a value derived from the base `TextStyle` for the
    /// input field and the current `Theme`.
    ///
    /// Specifying this style will override the default behavior
    /// of `InputDecoration` that changes the color of the label to the
    /// `InputDecoration::error_style` color or `ColorScheme::error`.
    pub label_style: Option<AnyTextStyle>,

    /// The style to use for `InputDecoration::label_text` when the label is
    /// above (i.e., vertically adjacent to) the input field.
    ///
    /// When the `InputDecoration::label_text` is on top of the input field, the
    /// text uses the `label_style` instead.
    ///
    /// If `floating_label_style` is a `WidgetStateTextStyle`, then the effective
    /// text style can depend on the `WidgetState::Focused` state, i.e.
    /// if the `TextField` is focused or not.
    ///
    /// If null, defaults to `label_style`.
    ///
    /// Specifying this style will override the default behavior
    /// of `InputDecoration` that changes the color of the label to the
    /// `InputDecoration::error_style` color or `ColorScheme::error`.
    ///
    /// When the input field receives focus, the font size of `InputDecoration::label` is
    /// scaled down by 75%.
    pub floating_label_style: Option<AnyTextStyle>,

    /// The direction to use for the `label_text`.
    ///
    /// If null, defaults to a value derived from `Directionality` for the
    /// input field and the current context.
    ///
    /// This is ignored if `label` is provided instead of `label_text`.
    /// If you need to set the direction for a custom `label` widget,
    /// wrap that widget in a `Directionality` widget or use a `Text`
    /// widget with its own `Text::text_direction` property.
    pub label_text_direction: Option<TextDirection>,

    /// Optional widget that appears below the `InputDecorator::child`.
    ///
    /// If non-null, the `helper` is displayed below the `InputDecorator::child`, in
    /// the same location as `error`. If a non-null `error` or `error_text` value is
    /// specified then the `helper` is not shown.
    ///
    /// Only one of `helper` and `helper_text` can be specified.
    pub helper: Option<WidgetRef>,

    /// Text that provides context about the `InputDecorator::child`'s value, such
    /// as how the value will be used.
    ///
    /// If non-null, the text is displayed below the `InputDecorator::child`, in
    /// the same location as `error_text`. If a non-null `error_text` value is
    /// specified then the helper text is not shown.
    ///
    /// If a more elaborate helper text is required, consider using `helper` instead.
    ///
    /// Only one of `helper` and `helper_text` can be specified.
    pub helper_text: Option<String>,

    /// The style to use for the `helper_text`.
    ///
    /// If `helper_style` is a `WidgetStateTextStyle`, then the effective
    /// text style can depend on the `WidgetState::Focused` state, i.e.
    /// if the `TextField` is focused or not.
    pub helper_style: Option<AnyTextStyle>,

    /// The maximum number of lines the `helper_text` can occupy.
    ///
    /// Defaults to null, which means that soft line breaks in `helper_text` are
    /// truncated with an ellipse while hard line breaks are respected.
    /// For example, a `helper_text` that overflows the width of the field will be
    /// truncated with an ellipse. However, a `helper_text` with explicit linebreak
    /// characters (\n) will display on multiple lines.
    ///
    /// To cause a long `helper_text` to wrap, either set `helper_max_lines` or use
    /// `helper` which offers more flexibility. For instance, it can be set to a
    /// `Text` widget with a specific overflow value.
    ///
    /// This value is passed along to the `Text::max_lines` attribute
    /// of the `Text` widget used to display the helper.
    ///
    /// See also:
    ///
    ///  * `error_max_lines`, the equivalent but for the `error_text`.
    pub helper_max_lines: Option<i32>,

    /// Text that suggests what sort of input the field accepts.
    ///
    /// Displayed on top of the `InputDecorator::child` (i.e., at the same location
    /// on the screen where text may be entered in the `InputDecorator::child`),
    /// when `InputDecorator::is_empty` is true and either (a) `label_text` is null
    /// or (b) the input has the focus.
    pub hint_text: Option<String>,

    /// The widget to use in place of the `hint_text`.
    ///
    /// Either `hint_text` or `hint` can be specified, but not both.
    pub hint: Option<WidgetRef>,

    /// The style to use for the `hint_text`.
    ///
    /// If `hint_style` is a `WidgetStateTextStyle`, then the effective
    /// text style can depend on the `WidgetState::Focused` state, i.e.
    /// if the `TextField` is focused or not.
    ///
    /// Also used for the `label_text` when the `label_text` is displayed on
    /// top of the input field (i.e., at the same location on the screen where
    /// text may be entered in the `InputDecorator::child`).
    ///
    /// If null, defaults to a value derived from the base `TextStyle` for the
    /// input field and the current `Theme`.
    pub hint_style: Option<AnyTextStyle>,

    /// The direction to use for the `hint_text`.
    ///
    /// If null, defaults to a value derived from `Directionality` for the
    /// input field and the current context.
    ///
    /// This is ignored if `hint` is provided instead of `hint_text`.
    /// If you need to set the direction for a custom `hint` widget,
    /// wrap that widget in a `Directionality` widget or use a `Text`
    /// widget with its own `Text::text_direction` property.
    pub hint_text_direction: Option<TextDirection>,

    /// The maximum number of lines the `hint_text` can occupy.
    ///
    /// Defaults to the value of `TextField::max_lines` attribute.
    ///
    /// This value is passed along to the `Text::max_lines` attribute
    /// of the `Text` widget used to display the hint text. `TextOverflow::Ellipsis` is
    /// used to handle the overflow when it is limited to single line.
    pub hint_max_lines: Option<i32>,

    /// The duration of the `hint_text` fade in and fade out animations.
    ///
    /// If null, defaults to `InputDecorationThemeData::hint_fade_duration`.
    /// If `InputDecorationThemeData::hint_fade_duration` is null defaults to 20ms.
    pub hint_fade_duration: Option<Duration>,

    /// Whether the input field's height should always be greater than or equal to
    /// the height of the `hint_text`, even if the `hint_text` is not visible.
    ///
    /// The `InputDecorator` widget ignores `hint_text` during layout when
    /// it's not visible, if this flag is set to false.
    ///
    /// Defaults to true.
    #[deprecated(
        note = "Use maintainHintSize instead. This will maintain both hint height and hint width. This feature was deprecated after v3.28.0-2.0.pre."
    )]
    pub maintain_hint_height: bool,

    /// Whether the input field's size should always be greater than or equal to
    /// the size of the `hint` or `hint_text`, even if the `hint` or `hint_text` are not visible.
    ///
    /// The `InputDecorator` widget ignores `hint` and `hint_text` during layout when
    /// they are not visible, if this flag is set to false.
    ///
    /// Defaults to true.
    pub maintain_hint_size: bool,

    /// Whether the input field's size should always be greater than or equal to
    /// the size of the `label` or `label_text`, even if the `label` or `label_text` are not visible.
    ///
    /// The `InputDecorator` widget ignores `label` and `label_text` during layout when
    /// this flag is set to false.
    ///
    /// Defaults to false for compatibility reason.
    pub maintain_label_size: bool,

    /// Optional widget that appears below the `InputDecorator::child` and the border.
    ///
    /// If non-null, the border's color animates to red and the `helper_text` is not shown.
    ///
    /// Only one of `error` and `error_text` can be specified.
    pub error: Option<WidgetRef>,

    /// Text that appears below the `InputDecorator::child` and the border.
    ///
    /// If non-null, the border's color animates to red and the `helper_text` is
    /// not shown.
    ///
    /// In a `TextFormField`, this is overridden by the value returned from
    /// `TextFormField::validator`, if that is not null.
    ///
    /// If a more elaborate error is required, consider using `error` instead.
    ///
    /// Only one of `error` and `error_text` can be specified.
    pub error_text: Option<String>,

    /// The style to use for the `InputDecoration::error_text`.
    ///
    /// If null, defaults of a value derived from the base `TextStyle` for the
    /// input field and the current `Theme`.
    ///
    /// By default the color of style will be used by the label of
    /// `InputDecoration` if `InputDecoration::error_text` is not null. See
    /// `InputDecoration::label_style` or `InputDecoration::floating_label_style` for
    /// an example of how to replicate this behavior when specifying those
    /// styles.
    pub error_style: Option<AnyTextStyle>,

    /// The maximum number of lines the `error_text` can occupy.
    ///
    /// Defaults to null, which means that soft line breaks in `error_text` are
    /// truncated with an ellipse while hard line breaks are respected.
    /// For example, an `error_text` that overflows the width of the field will be
    /// truncated with an ellipse. However, an `error_text` with explicit linebreak
    /// characters (\n) will display on multiple lines.
    ///
    /// To cause a long `error_text` to wrap, either set `error_max_lines` or use
    /// `error` which offers more flexibility. For instance, it can be set to a
    /// `Text` widget with a specific overflow value.
    ///
    /// This value is passed along to the `Text::max_lines` attribute
    /// of the `Text` widget used to display the error.
    ///
    /// See also:
    ///
    ///  * `helper_max_lines`, the equivalent but for the `helper_text`.
    pub error_max_lines: Option<i32>,

    /// Defines **how** the floating label should behave.
    ///
    /// When `FloatingLabelBehavior::Auto` the label will float to the top only when
    /// the field is focused or has some text content, otherwise it will appear
    /// in the field in place of the content.
    ///
    /// When `FloatingLabelBehavior::Always` the label will always float at the top
    /// of the field above the content.
    ///
    /// When `FloatingLabelBehavior::Never` the label will always appear in an empty
    /// field in place of the content.
    ///
    /// If null, `InputDecorationThemeData::floating_label_behavior` will be used.
    ///
    /// See also:
    ///
    ///  * `floating_label_alignment` which defines **where** the floating label
    ///    should be displayed.
    pub floating_label_behavior: Option<FloatingLabelBehavior>,

    /// Defines **where** the floating label should be displayed.
    ///
    /// `FloatingLabelAlignment::START` aligns the floating label to the leftmost
    /// (when `TextDirection::Ltr`) or rightmost (when `TextDirection::Rtl`),
    /// possible position, which is vertically adjacent to the label, on top of
    /// the field.
    ///
    /// `FloatingLabelAlignment::CENTER` aligns the floating label to the center on
    /// top of the field.
    ///
    /// If null, `InputDecorationThemeData::floating_label_alignment` will be used.
    ///
    /// See also:
    ///
    ///  * `floating_label_behavior` which defines **how** the floating label should
    ///    behave.
    pub floating_label_alignment: Option<FloatingLabelAlignment>,

    /// Whether the `InputDecorator::child` is part of a dense form (i.e., uses less vertical
    /// space).
    ///
    /// Defaults to false.
    pub is_dense: Option<bool>,

    /// The padding for the input decoration's container.
    ///
    /// The decoration's container is the area which is filled if `filled` is true
    /// and bordered per the `border`. It's the area adjacent to `icon` and above
    /// the widgets that contain `helper_text`, `error_text`, and `counter_text`.
    ///
    /// By default the `content_padding` reflects `is_dense` and the type of the
    /// `border`.
    ///
    /// If `is_collapsed` is true then `content_padding` is `EdgeInsets::ZERO`.
    ///
    /// ### Material 3 default content padding
    ///
    /// If `isOutline` property of `border` is false and if `filled` is true then
    /// `content_padding` is `EdgeInsets.fromLTRB(12, 4, 12, 4)` when `is_dense`
    /// is true and `EdgeInsets.fromLTRB(12, 8, 12, 8)` when `is_dense` is false.
    ///
    /// If `isOutline` property of `border` is false and if `filled` is false then
    /// `content_padding` is `EdgeInsets.fromLTRB(0, 4, 0, 4)` when `is_dense` is
    /// true and `EdgeInsets.fromLTRB(0, 8, 0, 8)` when `is_dense` is false.
    ///
    /// If `isOutline` property of `border` is true then `content_padding` is
    /// `EdgeInsets.fromLTRB(12, 16, 12, 8)` when `is_dense` is true
    /// and `EdgeInsets.fromLTRB(12, 20, 12, 12)` when `is_dense` is false.
    ///
    /// ### Material 2 default content padding
    ///
    /// If `isOutline` property of `border` is false and if `filled` is true then
    /// `content_padding` is `EdgeInsets.fromLTRB(12, 8, 12, 8)` when `is_dense`
    /// is true and `EdgeInsets.fromLTRB(12, 12, 12, 12)` when `is_dense` is false.
    ///
    /// If `isOutline` property of `border` is false and if `filled` is false then
    /// `content_padding` is `EdgeInsets.fromLTRB(0, 8, 0, 8)` when `is_dense` is
    /// true and `EdgeInsets.fromLTRB(0, 12, 0, 12)` when `is_dense` is false.
    ///
    /// If `isOutline` property of `border` is true then `content_padding` is
    /// `EdgeInsets.fromLTRB(12, 20, 12, 12)` when `is_dense` is true
    /// and `EdgeInsets.fromLTRB(12, 24, 12, 16)` when `is_dense` is false.
    pub content_padding: Option<EdgeInsetsGeometry>,

    /// Whether the decoration is the same size as the input field.
    ///
    /// A collapsed decoration cannot have `label_text`, `error_text`, `counter`,
    /// `icon`, prefixes, and suffixes.
    ///
    /// To create a collapsed input decoration, use `InputDecoration::collapsed`.
    pub is_collapsed: Option<bool>,

    /// An icon that appears before the `prefix` or `prefix_text` and before
    /// the editable part of the text field, within the decoration's container.
    ///
    /// The size and color of the prefix icon is configured automatically using an
    /// `IconTheme` and therefore does not need to be explicitly given in the
    /// icon widget.
    ///
    /// The prefix icon is constrained with a minimum size of 48px by 48px, but
    /// can be expanded beyond that. Anything larger than 24px will require
    /// additional padding to ensure it matches the Material Design spec of 12px
    /// padding between the left edge of the input and leading edge of the prefix
    /// icon. The following snippet shows how to pad the leading edge of the
    /// prefix icon:
    ///
    /// ```text
    /// prefixIcon: Padding(
    ///   padding: const EdgeInsetsDirectional.only(start: 12.0),
    ///   child: _myIcon, // _myIcon is a 48px-wide widget.
    /// )
    /// ```
    ///
    /// The decoration's container is the area which is filled if `filled` is true
    /// and bordered per the `border`. It's the area adjacent to `icon` and above
    /// the widgets that contain `helper_text`, `error_text`, and `counter_text`.
    ///
    /// The prefix icon alignment can be changed using `Align` with a fixed `widthFactor` and
    /// `heightFactor`.
    ///
    /// See also:
    ///
    ///  * `Icon` and `ImageIcon`, which are typically used to show icons.
    ///  * `prefix` and `prefix_text`, which are other ways to show content
    ///    before the text field (but after the icon).
    ///  * `suffix_icon`, which is the same but on the trailing edge.
    ///  * `Align` A widget that aligns its child within itself and optionally
    ///    sizes itself based on the child's size.
    pub prefix_icon: Option<WidgetRef>,

    /// The constraints for the prefix icon.
    ///
    /// This can be used to modify the `BoxConstraints` surrounding `prefix_icon`.
    ///
    /// This property is particularly useful for getting the decoration's height
    /// less than the minimum tappable height (which is 48px when the visual
    /// density is set to `VisualDensity::standard`). This can be achieved by
    /// setting `is_dense` to true and setting the constraints' minimum height
    /// and width to a value lower than the minimum tappable size.
    pub prefix_icon_constraints: Option<BoxConstraints>,

    /// Optional widget to place on the line before the input.
    ///
    /// This can be used, for example, to add some padding to text that would
    /// otherwise be specified using `prefix_text`, or to add a custom widget in
    /// front of the input. The widget's baseline is lined up with the input
    /// baseline.
    ///
    /// Only one of `prefix` and `prefix_text` can be specified.
    ///
    /// The `prefix` appears after the `prefix_icon`, if both are specified.
    ///
    /// See also:
    ///
    ///  * `suffix`, the equivalent but on the trailing edge.
    pub prefix: Option<WidgetRef>,

    /// Optional text prefix to place on the line before the input.
    ///
    /// Uses the `prefix_style`. Uses `hint_style` if `prefix_style` isn't specified.
    /// The prefix text is not returned as part of the user's input.
    ///
    /// If a more elaborate prefix is required, consider using `prefix` instead.
    /// Only one of `prefix` and `prefix_text` can be specified.
    ///
    /// The `prefix_text` appears after the `prefix_icon`, if both are specified.
    ///
    /// See also:
    ///
    ///  * `suffix_text`, the equivalent but on the trailing edge.
    pub prefix_text: Option<String>,

    /// The style to use for the `prefix_text`.
    ///
    /// If `prefix_style` is a `WidgetStateTextStyle`, then the effective
    /// text style can depend on the `WidgetState::Focused` state, i.e.
    /// if the `TextField` is focused or not.
    ///
    /// If null, defaults to the `hint_style`.
    ///
    /// See also:
    ///
    ///  * `suffix_style`, the equivalent but on the trailing edge.
    pub prefix_style: Option<AnyTextStyle>,

    /// Optional color of the prefixIcon
    ///
    /// Defaults to `icon_color`
    ///
    /// If `prefix_icon_color` is a `WidgetStateColor`, then the effective
    /// color can depend on the `WidgetState::Focused` state, i.e.
    /// if the `TextField` is focused or not.
    pub prefix_icon_color: Option<AnyColor>,

    /// An icon that appears after the editable part of the text field and
    /// after the `suffix` or `suffix_text`, within the decoration's container.
    ///
    /// The size and color of the suffix icon is configured automatically using an
    /// `IconTheme` and therefore does not need to be explicitly given in the
    /// icon widget.
    ///
    /// The suffix icon is constrained with a minimum size of 48px by 48px, but
    /// can be expanded beyond that. Anything larger than 24px will require
    /// additional padding to ensure it matches the Material Design spec of 12px
    /// padding between the right edge of the input and trailing edge of the
    /// prefix icon. The following snippet shows how to pad the trailing edge of
    /// the suffix icon:
    ///
    /// ```text
    /// suffixIcon: Padding(
    ///   padding: const EdgeInsetsDirectional.only(end: 12.0),
    ///   child: _myIcon, // myIcon is a 48px-wide widget.
    /// )
    /// ```
    ///
    /// The decoration's container is the area which is filled if `filled` is
    /// true and bordered per the `border`. It's the area adjacent to
    /// `icon` and above the widgets that contain `helper_text`,
    /// `error_text`, and `counter_text`.
    ///
    /// The suffix icon alignment can be changed using `Align` with a fixed `widthFactor` and
    /// `heightFactor`.
    ///
    /// See also:
    ///
    ///  * `Icon` and `ImageIcon`, which are typically used to show icons.
    ///  * `suffix` and `suffix_text`, which are other ways to show content
    ///    after the text field (but before the icon).
    ///  * `prefix_icon`, which is the same but on the leading edge.
    ///  * `Align` A widget that aligns its child within itself and optionally
    ///    sizes itself based on the child's size.
    pub suffix_icon: Option<WidgetRef>,

    /// Optional widget to place on the line after the input.
    ///
    /// This can be used, for example, to add some padding to the text that would
    /// otherwise be specified using `suffix_text`, or to add a custom widget after
    /// the input. The widget's baseline is lined up with the input baseline.
    ///
    /// Only one of `suffix` and `suffix_text` can be specified.
    ///
    /// The `suffix` appears before the `suffix_icon`, if both are specified.
    ///
    /// See also:
    ///
    ///  * `prefix`, the equivalent but on the leading edge.
    pub suffix: Option<WidgetRef>,

    /// Optional text suffix to place on the line after the input.
    ///
    /// Uses the `suffix_style`. Uses `hint_style` if `suffix_style` isn't specified.
    /// The suffix text is not returned as part of the user's input.
    ///
    /// If a more elaborate suffix is required, consider using `suffix` instead.
    /// Only one of `suffix` and `suffix_text` can be specified.
    ///
    /// The `suffix_text` appears before the `suffix_icon`, if both are specified.
    ///
    /// See also:
    ///
    ///  * `prefix_text`, the equivalent but on the leading edge.
    pub suffix_text: Option<String>,

    /// The style to use for the `suffix_text`.
    ///
    /// If `suffix_style` is a `WidgetStateTextStyle`, then the effective text
    /// style can depend on the `WidgetState::Focused` state, i.e. if the
    /// `TextField` is focused or not.
    ///
    /// If null, defaults to the `hint_style`.
    ///
    /// See also:
    ///
    ///  * `prefix_style`, the equivalent but on the leading edge.
    pub suffix_style: Option<AnyTextStyle>,

    /// Optional color of the `suffix_icon`.
    ///
    /// Defaults to `icon_color`
    ///
    /// If `suffix_icon_color` is a `WidgetStateColor`, then the effective
    /// color can depend on the `WidgetState::Focused` state, i.e.
    /// if the `TextField` is focused or not.
    pub suffix_icon_color: Option<AnyColor>,

    /// The constraints for the suffix icon.
    ///
    /// This can be used to modify the `BoxConstraints` surrounding `suffix_icon`.
    ///
    /// This property is particularly useful for getting the decoration's height
    /// less than the minimum tappable height (which is 48px when the visual
    /// density is set to `VisualDensity::standard`). This can be achieved by
    /// setting `is_dense` to true and setting the constraints' minimum height
    /// and width to a value lower than the minimum tappable size.
    ///
    /// If null, a `BoxConstraints` with a minimum width and height of 48px is
    /// used.
    pub suffix_icon_constraints: Option<BoxConstraints>,

    /// Optional text to place below the line as a character count.
    ///
    /// Rendered using `counter_style`. Uses `helper_style` if `counter_style` is
    /// null.
    ///
    /// The semantic label can be replaced by providing a `semantic_counter_text`.
    ///
    /// If null or an empty string and `counter` isn't specified, then nothing
    /// will appear in the counter's location.
    pub counter_text: Option<String>,

    /// Optional custom counter widget to go in the place otherwise occupied by
    /// `counter_text`. If this property is non null, then `counter_text` is
    /// ignored.
    pub counter: Option<WidgetRef>,

    /// The style to use for the `counter_text`.
    ///
    /// If `counter_style` is a `WidgetStateTextStyle`, then the effective
    /// text style can depend on the `WidgetState::Focused` state, i.e.
    /// if the `TextField` is focused or not.
    ///
    /// If null, defaults to the `helper_style`.
    pub counter_style: Option<AnyTextStyle>,

    /// If true the decoration's container is filled with `fill_color`.
    ///
    /// When `InputDecorator::is_hovering` is true, the `hover_color` is also blended
    /// into the final fill color.
    ///
    /// Typically this field set to true if `border` is an `UnderlineInputBorder`.
    ///
    /// The decoration's container is the area which is filled if `filled` is true
    /// and bordered per the `border`. It's the area adjacent to `icon` and above
    /// the widgets that contain `helper_text`, `error_text`, and `counter_text`.
    ///
    /// This property is false by default.
    pub filled: Option<bool>,

    /// The base fill color of the decoration's container color.
    ///
    /// When `InputDecorator::is_hovering` is true, the `hover_color` is also blended
    /// into the final fill color.
    ///
    /// By default the `fill_color` is based on the current
    /// `InputDecorationThemeData::fill_color`.
    ///
    /// The decoration's container is the area which is filled if `filled` is true
    /// and bordered per the `border`. It's the area adjacent to `icon` and above
    /// the widgets that contain `helper_text`, `error_text`, and `counter_text`.
    pub fill_color: Option<AnyColor>,

    /// The fill color of the decoration's container when it has the input focus.
    ///
    /// By default the `focus_color` is based on the current
    /// `InputDecorationThemeData::focus_color`.
    ///
    /// This `focus_color` is ignored by `TextField` and `TextFormField` because
    /// they don't respond to focus changes by changing their decorator's
    /// container color, they respond by changing their border to the
    /// `focused_border`, which you can change the color of.
    ///
    /// The decoration's container is the area which is filled if `filled` is true
    /// and bordered per the `border`. It's the area adjacent to `icon` and above
    /// the widgets that contain `helper_text`, `error_text`, and `counter_text`.
    pub focus_color: Option<Color>,

    /// The color of the highlight for the decoration shown if the container
    /// is being hovered over by a mouse.
    ///
    /// If `filled` is true, the `hover_color` is blended with `fill_color` and
    /// fills the decoration's container.
    ///
    /// If `filled` is false, and `InputDecorator::is_focused` is false, the color
    /// is blended over the `enabled_border`'s color.
    ///
    /// By default the `hover_color` is based on the current `Theme`.
    ///
    /// The decoration's container is the area which is filled if `filled` is true
    /// and bordered per the `border`. It's the area adjacent to `icon` and above
    /// the widgets that contain `helper_text`, `error_text`, and `counter_text`.
    pub hover_color: Option<Color>,

    /// The border to display when the `InputDecorator` does not have the focus and
    /// is showing an error.
    ///
    /// See also:
    ///
    ///  * `InputDecorator::is_focused`, which is true if the `InputDecorator`'s child
    ///    has the focus.
    ///  * `InputDecoration::error_text`, the error shown by the `InputDecorator`, if non-null.
    ///  * `border`, for a description of where the `InputDecorator` border appears.
    ///  * `UnderlineInputBorder`, an `InputDecorator` border which draws a horizontal
    ///    line at the bottom of the input decorator's container.
    ///  * `OutlineInputBorder`, an `InputDecorator` border which draws a
    ///    rounded rectangle around the input decorator's container.
    ///  * `InputBorderRef::none()`, which doesn't draw a border.
    ///  * `focused_border`, displayed when `InputDecorator::is_focused` is true
    ///    and `InputDecoration::error_text` is null.
    ///  * `focused_error_border`, displayed when `InputDecorator::is_focused` is true
    ///    and `InputDecoration::error_text` is non-null.
    ///  * `disabled_border`, displayed when `InputDecoration::enabled` is false
    ///    and `InputDecoration::error_text` is null.
    ///  * `enabled_border`, displayed when `InputDecoration::enabled` is true
    ///    and `InputDecoration::error_text` is null.
    pub error_border: Option<InputBorderRef>,

    /// The border to display when the `InputDecorator` has the focus and is not
    /// showing an error.
    ///
    /// See also:
    ///
    ///  * `InputDecorator::is_focused`, which is true if the `InputDecorator`'s child
    ///    has the focus.
    ///  * `InputDecoration::error_text`, the error shown by the `InputDecorator`, if non-null.
    ///  * `border`, for a description of where the `InputDecorator` border appears.
    ///  * `UnderlineInputBorder`, an `InputDecorator` border which draws a horizontal
    ///    line at the bottom of the input decorator's container.
    ///  * `OutlineInputBorder`, an `InputDecorator` border which draws a
    ///    rounded rectangle around the input decorator's container.
    ///  * `InputBorderRef::none()`, which doesn't draw a border.
    ///  * `error_border`, displayed when `InputDecorator::is_focused` is false
    ///    and `InputDecoration::error_text` is non-null.
    ///  * `focused_error_border`, displayed when `InputDecorator::is_focused` is true
    ///    and `InputDecoration::error_text` is non-null.
    ///  * `disabled_border`, displayed when `InputDecoration::enabled` is false
    ///    and `InputDecoration::error_text` is null.
    ///  * `enabled_border`, displayed when `InputDecoration::enabled` is true
    ///    and `InputDecoration::error_text` is null.
    pub focused_border: Option<InputBorderRef>,

    /// The border to display when the `InputDecorator` has the focus and is
    /// showing an error.
    ///
    /// See also:
    ///
    ///  * `InputDecorator::is_focused`, which is true if the `InputDecorator`'s child
    ///    has the focus.
    ///  * `InputDecoration::error_text`, the error shown by the `InputDecorator`, if non-null.
    ///  * `border`, for a description of where the `InputDecorator` border appears.
    ///  * `UnderlineInputBorder`, an `InputDecorator` border which draws a horizontal
    ///    line at the bottom of the input decorator's container.
    ///  * `OutlineInputBorder`, an `InputDecorator` border which draws a
    ///    rounded rectangle around the input decorator's container.
    ///  * `InputBorderRef::none()`, which doesn't draw a border.
    ///  * `error_border`, displayed when `InputDecorator::is_focused` is false
    ///    and `InputDecoration::error_text` is non-null.
    ///  * `focused_border`, displayed when `InputDecorator::is_focused` is true
    ///    and `InputDecoration::error_text` is null.
    ///  * `disabled_border`, displayed when `InputDecoration::enabled` is false
    ///    and `InputDecoration::error_text` is null.
    ///  * `enabled_border`, displayed when `InputDecoration::enabled` is true
    ///    and `InputDecoration::error_text` is null.
    pub focused_error_border: Option<InputBorderRef>,

    /// The border to display when the `InputDecorator` is disabled and is not
    /// showing an error.
    ///
    /// See also:
    ///
    ///  * `InputDecoration::enabled`, which is false if the `InputDecorator` is disabled.
    ///  * `InputDecoration::error_text`, the error shown by the `InputDecorator`, if non-null.
    ///  * `border`, for a description of where the `InputDecorator` border appears.
    ///  * `UnderlineInputBorder`, an `InputDecorator` border which draws a horizontal
    ///    line at the bottom of the input decorator's container.
    ///  * `OutlineInputBorder`, an `InputDecorator` border which draws a
    ///    rounded rectangle around the input decorator's container.
    ///  * `InputBorderRef::none()`, which doesn't draw a border.
    ///  * `error_border`, displayed when `InputDecorator::is_focused` is false
    ///    and `InputDecoration::error_text` is non-null.
    ///  * `focused_border`, displayed when `InputDecorator::is_focused` is true
    ///    and `InputDecoration::error_text` is null.
    ///  * `focused_error_border`, displayed when `InputDecorator::is_focused` is true
    ///    and `InputDecoration::error_text` is non-null.
    ///  * `enabled_border`, displayed when `InputDecoration::enabled` is true
    ///    and `InputDecoration::error_text` is null.
    pub disabled_border: Option<InputBorderRef>,

    /// The border to display when the `InputDecorator` is enabled and is not
    /// showing an error.
    ///
    /// See also:
    ///
    ///  * `InputDecoration::enabled`, which is false if the `InputDecorator` is disabled.
    ///  * `InputDecoration::error_text`, the error shown by the `InputDecorator`, if non-null.
    ///  * `border`, for a description of where the `InputDecorator` border appears.
    ///  * `UnderlineInputBorder`, an `InputDecorator` border which draws a horizontal
    ///    line at the bottom of the input decorator's container.
    ///  * `OutlineInputBorder`, an `InputDecorator` border which draws a
    ///    rounded rectangle around the input decorator's container.
    ///  * `InputBorderRef::none()`, which doesn't draw a border.
    ///  * `error_border`, displayed when `InputDecorator::is_focused` is false
    ///    and `InputDecoration::error_text` is non-null.
    ///  * `focused_border`, displayed when `InputDecorator::is_focused` is true
    ///    and `InputDecoration::error_text` is null.
    ///  * `focused_error_border`, displayed when `InputDecorator::is_focused` is true
    ///    and `InputDecoration::error_text` is non-null.
    ///  * `disabled_border`, displayed when `InputDecoration::enabled` is false
    ///    and `InputDecoration::error_text` is null.
    pub enabled_border: Option<InputBorderRef>,

    /// The shape of the border to draw around the decoration's container.
    ///
    /// If `border` is a `WidgetStateInputBorder` then the effective border is resolved
    /// in the following states:
    ///  * `WidgetState::Disabled`.
    ///  * `WidgetState::Error`.
    ///  * `WidgetState::Focused`.
    ///  * `WidgetState::Hovered`.
    ///
    /// If `border` derives from `InputBorder` the border's `InputBorder::border_side`,
    /// i.e. the border's color and width, will be overridden to reflect the input
    /// decorator's state. Only the border's shape is used. If custom  `BorderSide`
    /// values are desired for a given state, all five borders – `error_border`,
    /// `focused_border`, `enabled_border`, `disabled_border`, `focused_error_border` – must be set.
    ///
    /// The decoration's container is the area which is filled if `filled` is
    /// true and bordered per the `border`. It's the area adjacent to
    /// `InputDecoration::icon` and above the widgets that contain
    /// `InputDecoration::helper_text`, `InputDecoration::error_text`, and
    /// `InputDecoration::counter_text`.
    ///
    /// The border's bounds, i.e. the value of `border.getOuterPath()`, define
    /// the area to be filled.
    ///
    /// This property is only used when the appropriate one of `error_border`,
    /// `focused_border`, `focused_error_border`, `disabled_border`, or `enabled_border`
    /// is not specified. This border's `InputBorder::border_side` property is
    /// configured by the InputDecorator, depending on the values of
    /// `InputDecoration::error_text`, `InputDecoration::enabled`,
    /// `InputDecorator::is_focused` and the current `Theme`.
    ///
    /// Typically one of `UnderlineInputBorder` or `OutlineInputBorder`.
    /// If null, InputDecorator's default is `const UnderlineInputBorder()`.
    ///
    /// See also:
    ///
    ///  * `InputBorderRef::none()`, which doesn't draw a border.
    ///  * `UnderlineInputBorder`, which draws a horizontal line at the
    ///    bottom of the input decorator's container.
    ///  * `OutlineInputBorder`, an `InputDecorator` border which draws a
    ///    rounded rectangle around the input decorator's container.
    pub border: Option<InputBorderRef>,

    /// If false the opacity of the visual elements is reduced, including `helper_text`,`error_text`, and `counter_text`.
    ///
    /// This property is true by default.
    pub enabled: bool,

    /// A semantic label for the `counter_text`.
    ///
    /// Defaults to null.
    ///
    /// If provided, this replaces the semantic label of the `counter_text`.
    pub semantic_counter_text: Option<String>,

    /// Typically set to true when the `InputDecorator` contains a multiline
    /// `TextField` (`TextField::max_lines` is null or > 1) to override the default
    /// behavior of aligning the label with the center of the `TextField`.
    ///
    /// Defaults to false.
    pub align_label_with_hint: Option<bool>,

    /// Defines minimum and maximum sizes for the `InputDecorator`.
    ///
    /// Typically the decorator will fill the horizontal space it is given. For
    /// larger screens, it may be useful to have the maximum width clamped to
    /// a given value so it doesn't fill the whole screen. This property
    /// allows you to control how big the decorator will be in its available
    /// space.
    ///
    /// If null, then the ambient `InputDecorationThemeData::constraints` will be used.
    /// If that is null then the decorator will fill the available width with
    /// a default height based on text size.
    pub constraints: Option<BoxConstraints>,

    /// Defines how compact the decoration's layout will be.
    ///
    /// The vertical aspect of the default or user-specified `content_padding` is adjusted
    /// automatically based on `visual_density`.
    ///
    /// When the visual density is `VisualDensity::compact`, the vertical aspect of
    /// `content_padding` is reduced by 8 pixels.
    ///
    /// When the visual density is `VisualDensity::comfortable`, the vertical aspect of
    /// `content_padding` is reduced by 4 pixels.
    ///
    /// When the visual density is `VisualDensity::standard` vertical aspect of
    /// `content_padding` is not changed.
    ///
    /// If null, then the ambient `InputDecorationThemeData::visual_density` will be used.
    /// If that is null then `ThemeData::visual_density` will be used.
    ///
    /// See also:
    ///
    ///  * `ThemeData::visual_density`, which specifies the `visual_density` for all widgets
    ///    within a `Theme`.
    ///  * `InputDecorationThemeData::visual_density`, which can override this setting for a
    ///    given decorator.
    pub visual_density: Option<VisualDensity>,

    /// The padding around the supporting text row, which contains the helper
    /// text, the error text, and the counter.
    ///
    /// The start and end values set the horizontal position of the row, the
    /// same way `InputDecoration::content_padding` does for the input. The top
    /// value is the gap between the bottom of the decoration's container and
    /// the row, and the bottom value is the space below the row.
    ///
    /// If null, the row uses these defaults:
    ///
    ///  * start and end: the start and end values of
    ///    `InputDecoration::content_padding`.
    ///  * top: 4.0 for Material 3, 8.0 for Material 2.
    ///  * bottom: 0.0.
    ///
    /// If non-null, every side of this value is used as is, including sides
    /// that are zero. For example, `EdgeInsetsDirectional.only(start: 24.0)`
    /// sets the top gap to zero, so the supporting text sits directly against
    /// the container. To change the horizontal padding while keeping the
    /// default Material 3 gap, set the top value explicitly:
    ///
    /// ```text
    /// const InputDecoration(
    ///   helperText: 'Helper text',
    ///   supportingTextPadding: EdgeInsetsDirectional.fromSTEB(24.0, 4.0, 24.0, 0.0),
    /// )
    /// ```
    ///
    /// If both this and `InputDecorationThemeData::supporting_text_padding` are
    /// non-null, this value replaces the theme value entirely. The two are not
    /// merged side by side.
    pub supporting_text_padding: Option<EdgeInsetsGeometry>,
}

#[allow(deprecated)]
impl InputDecoration {
    /// Creates a bundle of the border, labels, icons, and styles used to
    /// decorate a Material Design text field.
    ///
    /// Unless specified by the ambient `InputDecorationThemeData`, `InputDecorator`
    /// defaults `is_dense` to false and `filled` to false. The default border is
    /// an instance of `UnderlineInputBorder`. If `border` is `InputBorderRef::none()`
    /// then no border is drawn.
    ///
    /// Only one of `prefix` and `prefix_text` can be specified.
    ///
    /// Similarly, only one of `suffix` and `suffix_text` can be specified.
    pub fn new() -> InputDecoration {
        InputDecoration {
            icon: None,
            icon_color: None,
            label: None,
            label_text: None,
            label_style: None,
            floating_label_style: None,
            label_text_direction: None,
            helper: None,
            helper_text: None,
            helper_style: None,
            helper_max_lines: None,
            hint_text: None,
            hint: None,
            hint_style: None,
            hint_text_direction: None,
            hint_max_lines: None,
            hint_fade_duration: None,
            maintain_hint_height: true,
            maintain_hint_size: true,
            maintain_label_size: false,
            error: None,
            error_text: None,
            error_style: None,
            error_max_lines: None,
            floating_label_behavior: None,
            floating_label_alignment: None,
            is_dense: None,
            content_padding: None,
            is_collapsed: None,
            prefix_icon: None,
            prefix_icon_constraints: None,
            prefix: None,
            prefix_text: None,
            prefix_style: None,
            prefix_icon_color: None,
            suffix_icon: None,
            suffix: None,
            suffix_text: None,
            suffix_style: None,
            suffix_icon_color: None,
            suffix_icon_constraints: None,
            counter_text: None,
            counter: None,
            counter_style: None,
            filled: None,
            fill_color: None,
            focus_color: None,
            hover_color: None,
            error_border: None,
            focused_border: None,
            focused_error_border: None,
            disabled_border: None,
            enabled_border: None,
            border: None,
            enabled: true,
            semantic_counter_text: None,
            align_label_with_hint: None,
            constraints: None,
            visual_density: None,
            supporting_text_padding: None,
        }
    }

    /// Defines an `InputDecorator` that is the same size as the input field.
    ///
    /// This type of input decoration does not include a border by default.
    ///
    /// A collapsed decoration cannot have `label_text`, `error_text`, `counter`,
    /// `icon`, prefixes, and suffixes.
    ///
    /// Sets the `is_collapsed` property to true.
    /// Sets the `content_padding` property to `EdgeInsets::ZERO`.
    pub fn collapsed(hint_text: Option<String>) -> InputDecoration {
        InputDecoration {
            icon: None,
            icon_color: None,
            label: None,
            label_text: None,
            label_style: None,
            floating_label_style: None,
            label_text_direction: None,
            helper: None,
            helper_text: None,
            helper_style: None,
            helper_max_lines: None,
            hint_text,
            hint: None,
            hint_style: None,
            hint_text_direction: None,
            hint_max_lines: None,
            hint_fade_duration: None,
            maintain_hint_height: true,
            maintain_hint_size: true,
            maintain_label_size: false,
            error: None,
            error_text: None,
            error_style: None,
            error_max_lines: None,
            floating_label_behavior: None,
            floating_label_alignment: None,
            is_dense: Some(false),
            content_padding: Some(EdgeInsets::ZERO.into()),
            is_collapsed: Some(true),
            prefix_icon: None,
            prefix_icon_constraints: None,
            prefix: None,
            prefix_text: None,
            prefix_style: None,
            prefix_icon_color: None,
            suffix_icon: None,
            suffix: None,
            suffix_text: None,
            suffix_style: None,
            suffix_icon_color: None,
            suffix_icon_constraints: None,
            counter_text: None,
            counter: None,
            counter_style: None,
            filled: Some(false),
            fill_color: None,
            focus_color: None,
            hover_color: None,
            error_border: None,
            focused_border: None,
            focused_error_border: None,
            disabled_border: None,
            enabled_border: None,
            border: Some(InputBorderRef::none()),
            enabled: true,
            semantic_counter_text: None,
            align_label_with_hint: Some(false),
            constraints: None,
            visual_density: None,
            supporting_text_padding: None,
        }
    }

    /// Dart `InputDecoration(icon:)`.
    pub fn icon<K>(mut self, icon: impl IntoWidget<K>) -> InputDecoration {
        self.icon = Some(icon.into_widget());
        self
    }

    /// Dart `InputDecoration(iconColor:)`.
    pub fn icon_color(mut self, icon_color: impl Into<AnyColor>) -> InputDecoration {
        self.icon_color = Some(icon_color.into());
        self
    }

    /// Dart `InputDecoration(label:)`.
    pub fn label<K>(mut self, label: impl IntoWidget<K>) -> InputDecoration {
        self.label = Some(label.into_widget());
        self
    }

    /// Dart `InputDecoration(labelText:)`.
    pub fn label_text(mut self, label_text: impl Into<String>) -> InputDecoration {
        self.label_text = Some(label_text.into());
        self
    }

    /// Dart `InputDecoration(labelStyle:)`.
    pub fn label_style(mut self, label_style: impl Into<AnyTextStyle>) -> InputDecoration {
        self.label_style = Some(label_style.into());
        self
    }

    /// Dart `InputDecoration(floatingLabelStyle:)`.
    pub fn floating_label_style(
        mut self,
        floating_label_style: impl Into<AnyTextStyle>,
    ) -> InputDecoration {
        self.floating_label_style = Some(floating_label_style.into());
        self
    }

    /// Dart `InputDecoration(labelTextDirection:)`.
    pub fn label_text_direction(mut self, label_text_direction: TextDirection) -> InputDecoration {
        self.label_text_direction = Some(label_text_direction);
        self
    }

    /// Dart `InputDecoration(helper:)`.
    pub fn helper<K>(mut self, helper: impl IntoWidget<K>) -> InputDecoration {
        self.helper = Some(helper.into_widget());
        self
    }

    /// Dart `InputDecoration(helperText:)`.
    pub fn helper_text(mut self, helper_text: impl Into<String>) -> InputDecoration {
        self.helper_text = Some(helper_text.into());
        self
    }

    /// Dart `InputDecoration(helperStyle:)`.
    pub fn helper_style(mut self, helper_style: impl Into<AnyTextStyle>) -> InputDecoration {
        self.helper_style = Some(helper_style.into());
        self
    }

    /// Dart `InputDecoration(helperMaxLines:)`.
    pub fn helper_max_lines(mut self, helper_max_lines: i32) -> InputDecoration {
        self.helper_max_lines = Some(helper_max_lines);
        self
    }

    /// Dart `InputDecoration(hintText:)`.
    pub fn hint_text(mut self, hint_text: impl Into<String>) -> InputDecoration {
        self.hint_text = Some(hint_text.into());
        self
    }

    /// Dart `InputDecoration(hint:)`.
    pub fn hint<K>(mut self, hint: impl IntoWidget<K>) -> InputDecoration {
        self.hint = Some(hint.into_widget());
        self
    }

    /// Dart `InputDecoration(hintStyle:)`.
    pub fn hint_style(mut self, hint_style: impl Into<AnyTextStyle>) -> InputDecoration {
        self.hint_style = Some(hint_style.into());
        self
    }

    /// Dart `InputDecoration(hintTextDirection:)`.
    pub fn hint_text_direction(mut self, hint_text_direction: TextDirection) -> InputDecoration {
        self.hint_text_direction = Some(hint_text_direction);
        self
    }

    /// Dart `InputDecoration(hintMaxLines:)`.
    pub fn hint_max_lines(mut self, hint_max_lines: i32) -> InputDecoration {
        self.hint_max_lines = Some(hint_max_lines);
        self
    }

    /// Dart `InputDecoration(hintFadeDuration:)`.
    pub fn hint_fade_duration(mut self, hint_fade_duration: Duration) -> InputDecoration {
        self.hint_fade_duration = Some(hint_fade_duration);
        self
    }

    /// Dart `InputDecoration(maintainHintHeight:)`.
    #[deprecated(
        note = "Use maintainHintSize instead. This will maintain both hint height and hint width. This feature was deprecated after v3.28.0-2.0.pre."
    )]
    pub fn maintain_hint_height(mut self, maintain_hint_height: bool) -> InputDecoration {
        self.maintain_hint_height = maintain_hint_height;
        self
    }

    /// Dart `InputDecoration(maintainHintSize:)`.
    pub fn maintain_hint_size(mut self, maintain_hint_size: bool) -> InputDecoration {
        self.maintain_hint_size = maintain_hint_size;
        self
    }

    /// Dart `InputDecoration(maintainLabelSize:)`.
    pub fn maintain_label_size(mut self, maintain_label_size: bool) -> InputDecoration {
        self.maintain_label_size = maintain_label_size;
        self
    }

    /// Dart `InputDecoration(error:)`.
    pub fn error<K>(mut self, error: impl IntoWidget<K>) -> InputDecoration {
        self.error = Some(error.into_widget());
        self
    }

    /// Dart `InputDecoration(errorText:)`.
    pub fn error_text(mut self, error_text: impl Into<String>) -> InputDecoration {
        self.error_text = Some(error_text.into());
        self
    }

    /// Dart `InputDecoration(errorStyle:)`.
    pub fn error_style(mut self, error_style: impl Into<AnyTextStyle>) -> InputDecoration {
        self.error_style = Some(error_style.into());
        self
    }

    /// Dart `InputDecoration(errorMaxLines:)`.
    pub fn error_max_lines(mut self, error_max_lines: i32) -> InputDecoration {
        self.error_max_lines = Some(error_max_lines);
        self
    }

    /// Dart `InputDecoration(floatingLabelBehavior:)`.
    pub fn floating_label_behavior(
        mut self,
        floating_label_behavior: FloatingLabelBehavior,
    ) -> InputDecoration {
        self.floating_label_behavior = Some(floating_label_behavior);
        self
    }

    /// Dart `InputDecoration(floatingLabelAlignment:)`.
    pub fn floating_label_alignment(
        mut self,
        floating_label_alignment: FloatingLabelAlignment,
    ) -> InputDecoration {
        self.floating_label_alignment = Some(floating_label_alignment);
        self
    }

    /// Dart `InputDecoration(isCollapsed:)`.
    pub fn is_collapsed(mut self, is_collapsed: bool) -> InputDecoration {
        self.is_collapsed = Some(is_collapsed);
        self
    }

    /// Dart `InputDecoration(isDense:)`.
    pub fn is_dense(mut self, is_dense: bool) -> InputDecoration {
        self.is_dense = Some(is_dense);
        self
    }

    /// Dart `InputDecoration(contentPadding:)`.
    pub fn content_padding(mut self, content_padding: EdgeInsetsGeometry) -> InputDecoration {
        self.content_padding = Some(content_padding);
        self
    }

    /// Dart `InputDecoration(prefixIcon:)`.
    pub fn prefix_icon<K>(mut self, prefix_icon: impl IntoWidget<K>) -> InputDecoration {
        self.prefix_icon = Some(prefix_icon.into_widget());
        self
    }

    /// Dart `InputDecoration(prefixIconConstraints:)`.
    pub fn prefix_icon_constraints(
        mut self,
        prefix_icon_constraints: BoxConstraints,
    ) -> InputDecoration {
        self.prefix_icon_constraints = Some(prefix_icon_constraints);
        self
    }

    /// Dart `InputDecoration(prefix:)`.
    pub fn prefix<K>(mut self, prefix: impl IntoWidget<K>) -> InputDecoration {
        self.prefix = Some(prefix.into_widget());
        self
    }

    /// Dart `InputDecoration(prefixText:)`.
    pub fn prefix_text(mut self, prefix_text: impl Into<String>) -> InputDecoration {
        self.prefix_text = Some(prefix_text.into());
        self
    }

    /// Dart `InputDecoration(prefixStyle:)`.
    pub fn prefix_style(mut self, prefix_style: impl Into<AnyTextStyle>) -> InputDecoration {
        self.prefix_style = Some(prefix_style.into());
        self
    }

    /// Dart `InputDecoration(prefixIconColor:)`.
    pub fn prefix_icon_color(mut self, prefix_icon_color: impl Into<AnyColor>) -> InputDecoration {
        self.prefix_icon_color = Some(prefix_icon_color.into());
        self
    }

    /// Dart `InputDecoration(suffixIcon:)`.
    pub fn suffix_icon<K>(mut self, suffix_icon: impl IntoWidget<K>) -> InputDecoration {
        self.suffix_icon = Some(suffix_icon.into_widget());
        self
    }

    /// Dart `InputDecoration(suffix:)`.
    pub fn suffix<K>(mut self, suffix: impl IntoWidget<K>) -> InputDecoration {
        self.suffix = Some(suffix.into_widget());
        self
    }

    /// Dart `InputDecoration(suffixText:)`.
    pub fn suffix_text(mut self, suffix_text: impl Into<String>) -> InputDecoration {
        self.suffix_text = Some(suffix_text.into());
        self
    }

    /// Dart `InputDecoration(suffixStyle:)`.
    pub fn suffix_style(mut self, suffix_style: impl Into<AnyTextStyle>) -> InputDecoration {
        self.suffix_style = Some(suffix_style.into());
        self
    }

    /// Dart `InputDecoration(suffixIconColor:)`.
    pub fn suffix_icon_color(mut self, suffix_icon_color: impl Into<AnyColor>) -> InputDecoration {
        self.suffix_icon_color = Some(suffix_icon_color.into());
        self
    }

    /// Dart `InputDecoration(suffixIconConstraints:)`.
    pub fn suffix_icon_constraints(
        mut self,
        suffix_icon_constraints: BoxConstraints,
    ) -> InputDecoration {
        self.suffix_icon_constraints = Some(suffix_icon_constraints);
        self
    }

    /// Dart `InputDecoration(counter:)`.
    pub fn counter<K>(mut self, counter: impl IntoWidget<K>) -> InputDecoration {
        self.counter = Some(counter.into_widget());
        self
    }

    /// Dart `InputDecoration(counterText:)`.
    pub fn counter_text(mut self, counter_text: impl Into<String>) -> InputDecoration {
        self.counter_text = Some(counter_text.into());
        self
    }

    /// Dart `InputDecoration(counterStyle:)`.
    pub fn counter_style(mut self, counter_style: impl Into<AnyTextStyle>) -> InputDecoration {
        self.counter_style = Some(counter_style.into());
        self
    }

    /// Dart `InputDecoration(filled:)`.
    pub fn filled(mut self, filled: bool) -> InputDecoration {
        self.filled = Some(filled);
        self
    }

    /// Dart `InputDecoration(fillColor:)`.
    pub fn fill_color(mut self, fill_color: impl Into<AnyColor>) -> InputDecoration {
        self.fill_color = Some(fill_color.into());
        self
    }

    /// Dart `InputDecoration(focusColor:)`.
    pub fn focus_color(mut self, focus_color: Color) -> InputDecoration {
        self.focus_color = Some(focus_color);
        self
    }

    /// Dart `InputDecoration(hoverColor:)`.
    pub fn hover_color(mut self, hover_color: Color) -> InputDecoration {
        self.hover_color = Some(hover_color);
        self
    }

    /// Dart `InputDecoration(errorBorder:)`.
    pub fn error_border(mut self, error_border: impl Into<InputBorderRef>) -> InputDecoration {
        self.error_border = Some(error_border.into());
        self
    }

    /// Dart `InputDecoration(focusedBorder:)`.
    pub fn focused_border(mut self, focused_border: impl Into<InputBorderRef>) -> InputDecoration {
        self.focused_border = Some(focused_border.into());
        self
    }

    /// Dart `InputDecoration(focusedErrorBorder:)`.
    pub fn focused_error_border(
        mut self,
        focused_error_border: impl Into<InputBorderRef>,
    ) -> InputDecoration {
        self.focused_error_border = Some(focused_error_border.into());
        self
    }

    /// Dart `InputDecoration(disabledBorder:)`.
    pub fn disabled_border(
        mut self,
        disabled_border: impl Into<InputBorderRef>,
    ) -> InputDecoration {
        self.disabled_border = Some(disabled_border.into());
        self
    }

    /// Dart `InputDecoration(enabledBorder:)`.
    pub fn enabled_border(mut self, enabled_border: impl Into<InputBorderRef>) -> InputDecoration {
        self.enabled_border = Some(enabled_border.into());
        self
    }

    /// Dart `InputDecoration(border:)`.
    pub fn border(mut self, border: impl Into<InputBorderRef>) -> InputDecoration {
        self.border = Some(border.into());
        self
    }

    /// Dart `InputDecoration(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> InputDecoration {
        self.enabled = enabled;
        self
    }

    /// Dart `InputDecoration(semanticCounterText:)`.
    pub fn semantic_counter_text(
        mut self,
        semantic_counter_text: impl Into<String>,
    ) -> InputDecoration {
        self.semantic_counter_text = Some(semantic_counter_text.into());
        self
    }

    /// Dart `InputDecoration(alignLabelWithHint:)`.
    pub fn align_label_with_hint(mut self, align_label_with_hint: bool) -> InputDecoration {
        self.align_label_with_hint = Some(align_label_with_hint);
        self
    }

    /// Dart `InputDecoration(constraints:)`.
    pub fn constraints(mut self, constraints: BoxConstraints) -> InputDecoration {
        self.constraints = Some(constraints);
        self
    }

    /// Dart `InputDecoration(visualDensity:)`.
    pub fn visual_density(mut self, visual_density: VisualDensity) -> InputDecoration {
        self.visual_density = Some(visual_density);
        self
    }

    /// Dart `InputDecoration(supportingTextPadding:)`.
    pub fn supporting_text_padding(
        mut self,
        supporting_text_padding: EdgeInsetsGeometry,
    ) -> InputDecoration {
        self.supporting_text_padding = Some(supporting_text_padding);
        self
    }

    /// Dart's constructor asserts on pairs of arguments, which a fluent chain only completes
    /// once it is handed over: `InputDecorator` calls this when it is turned into a
    /// `WidgetRef` (`into_widget`).
    pub(crate) fn debug_check_arguments(&self) {
        debug_assert!(
            !(self.label.is_some() && self.label_text.is_some()),
            "Declaring both label and labelText is not supported."
        );
        debug_assert!(
            self.hint.is_none() || self.hint_text.is_none(),
            "Declaring both hint and hintText is not supported."
        );
        debug_assert!(
            !(self.helper.is_some() && self.helper_text.is_some()),
            "Declaring both helper and helperText is not supported."
        );
        debug_assert!(
            !(self.prefix.is_some() && self.prefix_text.is_some()),
            "Declaring both prefix and prefixText is not supported."
        );
        debug_assert!(
            !(self.suffix.is_some() && self.suffix_text.is_some()),
            "Declaring both suffix and suffixText is not supported."
        );
        debug_assert!(
            !(self.error.is_some() && self.error_text.is_some()),
            "Declaring both error and errorText is not supported."
        );
    }

    /// Creates a copy of this input decoration with the given fields replaced
    /// by the new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`decoration.copy_with().label_text("Name")`).
    pub fn copy_with(&self) -> InputDecoration {
        self.clone()
    }

    /// Used by widgets like `TextField` and `InputDecorator` to create a new
    /// `InputDecoration` with default values taken from the `input_decoration_theme`.
    ///
    /// Only null valued properties from this `InputDecoration` are replaced
    /// by the corresponding values from `input_decoration_theme`.
    ///
    /// Dart's parameter is an `Object` that must be an `InputDecorationTheme` or an
    /// `InputDecorationThemeData`; here either converts into the data.
    pub fn apply_defaults(
        &self,
        input_decoration_theme: impl Into<InputDecorationThemeData>,
    ) -> InputDecoration {
        let theme: InputDecorationThemeData = input_decoration_theme.into();
        let mut decoration = self.copy_with();
        decoration.label_style = self
            .label_style
            .clone()
            .or_else(|| theme.label_style.clone());
        decoration.floating_label_style = self
            .floating_label_style
            .clone()
            .or_else(|| theme.floating_label_style.clone());
        decoration.helper_style = self
            .helper_style
            .clone()
            .or_else(|| theme.helper_style.clone());
        decoration.helper_max_lines = self.helper_max_lines.or(theme.helper_max_lines);
        decoration.hint_style = self.hint_style.clone().or_else(|| theme.hint_style.clone());
        decoration.hint_fade_duration = self.hint_fade_duration.or(theme.hint_fade_duration);
        decoration.hint_max_lines = self.hint_max_lines.or(theme.hint_max_lines);
        decoration.error_style = self
            .error_style
            .clone()
            .or_else(|| theme.error_style.clone());
        decoration.error_max_lines = self.error_max_lines.or(theme.error_max_lines);
        decoration.floating_label_behavior = self
            .floating_label_behavior
            .or(Some(theme.floating_label_behavior));
        decoration.floating_label_alignment = self
            .floating_label_alignment
            .or(Some(theme.floating_label_alignment));
        decoration.is_dense = self.is_dense.or(Some(theme.is_dense));
        decoration.content_padding = self.content_padding.or(theme.content_padding);
        decoration.is_collapsed = self.is_collapsed.or(Some(theme.is_collapsed));
        decoration.icon_color = self.icon_color.clone().or_else(|| theme.icon_color.clone());
        decoration.prefix_style = self
            .prefix_style
            .clone()
            .or_else(|| theme.prefix_style.clone());
        decoration.prefix_icon_color = self
            .prefix_icon_color
            .clone()
            .or_else(|| theme.prefix_icon_color.clone());
        decoration.prefix_icon_constraints = self
            .prefix_icon_constraints
            .or(theme.prefix_icon_constraints);
        decoration.suffix_style = self
            .suffix_style
            .clone()
            .or_else(|| theme.suffix_style.clone());
        decoration.suffix_icon_color = self
            .suffix_icon_color
            .clone()
            .or_else(|| theme.suffix_icon_color.clone());
        decoration.suffix_icon_constraints = self
            .suffix_icon_constraints
            .or(theme.suffix_icon_constraints);
        decoration.counter_style = self
            .counter_style
            .clone()
            .or_else(|| theme.counter_style.clone());
        decoration.filled = self.filled.or(Some(theme.filled));
        decoration.fill_color = self.fill_color.clone().or_else(|| theme.fill_color.clone());
        decoration.focus_color = self.focus_color.or(theme.focus_color);
        decoration.hover_color = self.hover_color.or(theme.hover_color);
        decoration.error_border = self
            .error_border
            .clone()
            .or_else(|| theme.error_border.clone());
        decoration.focused_border = self
            .focused_border
            .clone()
            .or_else(|| theme.focused_border.clone());
        decoration.focused_error_border = self
            .focused_error_border
            .clone()
            .or_else(|| theme.focused_error_border.clone());
        decoration.disabled_border = self
            .disabled_border
            .clone()
            .or_else(|| theme.disabled_border.clone());
        decoration.enabled_border = self
            .enabled_border
            .clone()
            .or_else(|| theme.enabled_border.clone());
        decoration.border = self.border.clone().or_else(|| theme.border.clone());
        decoration.align_label_with_hint = self
            .align_label_with_hint
            .or(Some(theme.align_label_with_hint));
        decoration.constraints = self.constraints.or(theme.constraints);
        decoration.visual_density = self.visual_density.or(theme.visual_density);
        decoration.supporting_text_padding = self
            .supporting_text_padding
            .or(theme.supporting_text_padding);
        decoration
    }
}

impl Default for InputDecoration {
    fn default() -> InputDecoration {
        InputDecoration::new()
    }
}

#[allow(deprecated)]
impl PartialEq for InputDecoration {
    // Dart compares the widgets by their `==`, which is identity.
    fn eq(&self, other: &InputDecoration) -> bool {
        same_widget(&other.icon, &self.icon)
            && other.icon_color == self.icon_color
            && same_widget(&other.label, &self.label)
            && other.label_text == self.label_text
            && other.label_style == self.label_style
            && other.floating_label_style == self.floating_label_style
            && other.label_text_direction == self.label_text_direction
            && same_widget(&other.helper, &self.helper)
            && other.helper_text == self.helper_text
            && other.helper_style == self.helper_style
            && other.helper_max_lines == self.helper_max_lines
            && other.hint_text == self.hint_text
            && same_widget(&other.hint, &self.hint)
            && other.hint_style == self.hint_style
            && other.hint_text_direction == self.hint_text_direction
            && other.hint_max_lines == self.hint_max_lines
            && other.hint_fade_duration == self.hint_fade_duration
            && other.maintain_hint_height == self.maintain_hint_height
            && other.maintain_hint_size == self.maintain_hint_size
            && other.maintain_label_size == self.maintain_label_size
            && same_widget(&other.error, &self.error)
            && other.error_text == self.error_text
            && other.error_style == self.error_style
            && other.error_max_lines == self.error_max_lines
            && other.floating_label_behavior == self.floating_label_behavior
            && other.floating_label_alignment == self.floating_label_alignment
            && other.is_dense == self.is_dense
            && other.content_padding == self.content_padding
            && other.is_collapsed == self.is_collapsed
            && same_widget(&other.prefix_icon, &self.prefix_icon)
            && other.prefix_icon_color == self.prefix_icon_color
            && same_widget(&other.prefix, &self.prefix)
            && other.prefix_text == self.prefix_text
            && other.prefix_style == self.prefix_style
            && other.prefix_icon_constraints == self.prefix_icon_constraints
            && same_widget(&other.suffix_icon, &self.suffix_icon)
            && other.suffix_icon_color == self.suffix_icon_color
            && same_widget(&other.suffix, &self.suffix)
            && other.suffix_text == self.suffix_text
            && other.suffix_style == self.suffix_style
            && other.suffix_icon_constraints == self.suffix_icon_constraints
            && same_widget(&other.counter, &self.counter)
            && other.counter_text == self.counter_text
            && other.counter_style == self.counter_style
            && other.filled == self.filled
            && other.fill_color == self.fill_color
            && other.focus_color == self.focus_color
            && other.hover_color == self.hover_color
            && other.error_border == self.error_border
            && other.focused_border == self.focused_border
            && other.focused_error_border == self.focused_error_border
            && other.disabled_border == self.disabled_border
            && other.enabled_border == self.enabled_border
            && other.border == self.border
            && other.enabled == self.enabled
            && other.semantic_counter_text == self.semantic_counter_text
            && other.align_label_with_hint == self.align_label_with_hint
            && other.constraints == self.constraints
            && other.visual_density == self.visual_density
            && other.supporting_text_padding == self.supporting_text_padding
    }
}

/// Dart's `toString`.
#[allow(deprecated)]
impl Debug for InputDecoration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut description: Vec<String> = Vec::new();
        if let Some(icon) = &self.icon {
            description.push(format!("icon: {:?}", icon));
        }
        if let Some(icon_color) = &self.icon_color {
            description.push(format!("iconColor: {:?}", icon_color));
        }
        if let Some(label) = &self.label {
            description.push(format!("label: {:?}", label));
        }
        if let Some(label_text) = &self.label_text {
            description.push(format!("labelText: \"{}\"", label_text));
        }
        if let Some(floating_label_style) = &self.floating_label_style {
            description.push(format!(
                "floatingLabelStyle: \"{:?}\"",
                floating_label_style
            ));
        }
        if let Some(helper) = &self.helper {
            description.push(format!("helper: \"{:?}\"", helper));
        }
        if let Some(helper_text) = &self.helper_text {
            description.push(format!("helperText: \"{}\"", helper_text));
        }
        if let Some(helper_max_lines) = &self.helper_max_lines {
            description.push(format!("helperMaxLines: \"{:?}\"", helper_max_lines));
        }
        if let Some(hint_text) = &self.hint_text {
            description.push(format!("hintText: \"{}\"", hint_text));
        }
        if let Some(hint) = &self.hint {
            description.push(format!("hint: {:?}", hint));
        }
        if let Some(hint_max_lines) = &self.hint_max_lines {
            description.push(format!("hintMaxLines: \"{:?}\"", hint_max_lines));
        }
        if let Some(hint_fade_duration) = &self.hint_fade_duration {
            description.push(format!("hintFadeDuration: \"{:?}\"", hint_fade_duration));
        }
        if !self.maintain_hint_height {
            description.push("maintainHintHeight: false".to_owned());
        }
        if !self.maintain_hint_size {
            description.push("maintainHintSize: false".to_owned());
        }
        if self.maintain_label_size {
            description.push("maintainLabelSize: true".to_owned());
        }
        if let Some(error) = &self.error {
            description.push(format!("error: \"{:?}\"", error));
        }
        if let Some(error_text) = &self.error_text {
            description.push(format!("errorText: \"{}\"", error_text));
        }
        if let Some(error_style) = &self.error_style {
            description.push(format!("errorStyle: \"{:?}\"", error_style));
        }
        if let Some(error_max_lines) = &self.error_max_lines {
            description.push(format!("errorMaxLines: \"{:?}\"", error_max_lines));
        }
        if let Some(floating_label_behavior) = &self.floating_label_behavior {
            description.push(format!(
                "floatingLabelBehavior: {:?}",
                floating_label_behavior
            ));
        }
        if let Some(floating_label_alignment) = &self.floating_label_alignment {
            description.push(format!(
                "floatingLabelAlignment: {:?}",
                floating_label_alignment
            ));
        }
        if self.is_dense.unwrap_or(false) {
            description.push(format!("isDense: {:?}", self.is_dense.expect("checked")));
        }
        if let Some(content_padding) = &self.content_padding {
            description.push(format!("contentPadding: {:?}", content_padding));
        }
        if self.is_collapsed.unwrap_or(false) {
            description.push(format!(
                "isCollapsed: {:?}",
                self.is_collapsed.expect("checked")
            ));
        }
        if let Some(prefix_icon) = &self.prefix_icon {
            description.push(format!("prefixIcon: {:?}", prefix_icon));
        }
        if let Some(prefix_icon_color) = &self.prefix_icon_color {
            description.push(format!("prefixIconColor: {:?}", prefix_icon_color));
        }
        if let Some(prefix) = &self.prefix {
            description.push(format!("prefix: {:?}", prefix));
        }
        if let Some(prefix_text) = &self.prefix_text {
            description.push(format!("prefixText: {}", prefix_text));
        }
        if let Some(prefix_style) = &self.prefix_style {
            description.push(format!("prefixStyle: {:?}", prefix_style));
        }
        if let Some(prefix_icon_constraints) = &self.prefix_icon_constraints {
            description.push(format!(
                "prefixIconConstraints: {:?}",
                prefix_icon_constraints
            ));
        }
        if let Some(suffix_icon) = &self.suffix_icon {
            description.push(format!("suffixIcon: {:?}", suffix_icon));
        }
        if let Some(suffix_icon_color) = &self.suffix_icon_color {
            description.push(format!("suffixIconColor: {:?}", suffix_icon_color));
        }
        if let Some(suffix) = &self.suffix {
            description.push(format!("suffix: {:?}", suffix));
        }
        if let Some(suffix_text) = &self.suffix_text {
            description.push(format!("suffixText: {}", suffix_text));
        }
        if let Some(suffix_style) = &self.suffix_style {
            description.push(format!("suffixStyle: {:?}", suffix_style));
        }
        if let Some(suffix_icon_constraints) = &self.suffix_icon_constraints {
            description.push(format!(
                "suffixIconConstraints: {:?}",
                suffix_icon_constraints
            ));
        }
        if let Some(counter) = &self.counter {
            description.push(format!("counter: {:?}", counter));
        }
        if let Some(counter_text) = &self.counter_text {
            description.push(format!("counterText: {}", counter_text));
        }
        if let Some(counter_style) = &self.counter_style {
            description.push(format!("counterStyle: {:?}", counter_style));
        }
        if self.filled.unwrap_or(false) {
            description.push("filled: true".to_owned());
        }
        if let Some(fill_color) = &self.fill_color {
            description.push(format!("fillColor: {:?}", fill_color));
        }
        if let Some(focus_color) = &self.focus_color {
            description.push(format!("focusColor: {:?}", focus_color));
        }
        if let Some(hover_color) = &self.hover_color {
            description.push(format!("hoverColor: {:?}", hover_color));
        }
        if let Some(error_border) = &self.error_border {
            description.push(format!("errorBorder: {:?}", error_border));
        }
        if let Some(focused_border) = &self.focused_border {
            description.push(format!("focusedBorder: {:?}", focused_border));
        }
        if let Some(focused_error_border) = &self.focused_error_border {
            description.push(format!("focusedErrorBorder: {:?}", focused_error_border));
        }
        if let Some(disabled_border) = &self.disabled_border {
            description.push(format!("disabledBorder: {:?}", disabled_border));
        }
        if let Some(enabled_border) = &self.enabled_border {
            description.push(format!("enabledBorder: {:?}", enabled_border));
        }
        if let Some(border) = &self.border {
            description.push(format!("border: {:?}", border));
        }
        if !self.enabled {
            description.push("enabled: false".to_owned());
        }
        if let Some(semantic_counter_text) = &self.semantic_counter_text {
            description.push(format!("semanticCounterText: {}", semantic_counter_text));
        }
        if let Some(align_label_with_hint) = &self.align_label_with_hint {
            description.push(format!("alignLabelWithHint: {:?}", align_label_with_hint));
        }
        if let Some(constraints) = &self.constraints {
            description.push(format!("constraints: {:?}", constraints));
        }
        if let Some(visual_density) = &self.visual_density {
            description.push(format!("visualDensity: {:?}", visual_density));
        }
        if let Some(supporting_text_padding) = &self.supporting_text_padding {
            description.push(format!(
                "supportingTextPadding: {:?}",
                supporting_text_padding
            ));
        }
        write!(f, "InputDecoration({})", description.join(", "))
    }
}

/// Defines the default appearance of `InputDecorator`s.
///
/// Descendant widgets obtain the current theme's `InputDecorationThemeData` using
/// `InputDecorationTheme::of`. When a widget uses `InputDecorationTheme::of`, it is
/// automatically rebuilt if the theme later changes.
///
/// See also:
///
///  * `InputDecorationThemeData`, which describes the actual configuration of an
///    input decoration theme.
///  * `ThemeData::input_decoration_theme`, which specifies an input decoration theme as
///    part of the overall Material theme.
///
/// Dart's constructor takes either the properties or a [`data`](Self::data); here the
/// properties are the `with_*` setters and [`with_data`](Self::with_data) the other, and each
/// setter that breaks Dart's constructor assert (a `data` together with any property) fails in
/// a debug build. The getters are methods, as Dart's, hence the `with_` prefix.
#[derive(Clone, Debug)]
pub struct InputDecorationTheme {
    pub key: Option<KeyRef>,
    data: Option<InputDecorationThemeData>,
    label_style: Option<AnyTextStyle>,
    floating_label_style: Option<AnyTextStyle>,
    helper_style: Option<AnyTextStyle>,
    helper_max_lines: Option<i32>,
    hint_style: Option<AnyTextStyle>,
    hint_fade_duration: Option<Duration>,
    hint_max_lines: Option<i32>,
    error_style: Option<AnyTextStyle>,
    error_max_lines: Option<i32>,
    floating_label_behavior: Option<FloatingLabelBehavior>,
    floating_label_alignment: Option<FloatingLabelAlignment>,
    is_dense: Option<bool>,
    content_padding: Option<EdgeInsetsGeometry>,
    is_collapsed: Option<bool>,
    icon_color: Option<AnyColor>,
    prefix_style: Option<AnyTextStyle>,
    prefix_icon_color: Option<AnyColor>,
    prefix_icon_constraints: Option<BoxConstraints>,
    suffix_style: Option<AnyTextStyle>,
    suffix_icon_color: Option<AnyColor>,
    suffix_icon_constraints: Option<BoxConstraints>,
    counter_style: Option<AnyTextStyle>,
    filled: Option<bool>,
    fill_color: Option<AnyColor>,
    active_indicator_border: Option<AnyBorderSide>,
    outline_border: Option<AnyBorderSide>,
    focus_color: Option<Color>,
    hover_color: Option<Color>,
    error_border: Option<InputBorderRef>,
    focused_border: Option<InputBorderRef>,
    focused_error_border: Option<InputBorderRef>,
    disabled_border: Option<InputBorderRef>,
    enabled_border: Option<InputBorderRef>,
    border: Option<InputBorderRef>,
    align_label_with_hint: Option<bool>,
    constraints: Option<BoxConstraints>,
    visual_density: Option<VisualDensity>,
    child: WidgetRef,
}

impl InputDecorationTheme {
    /// Creates a `InputDecorationTheme` that controls visual parameters for
    /// descendant `InputDecorator`s.
    pub fn new() -> InputDecorationTheme {
        InputDecorationTheme {
            key: None,
            data: None,
            label_style: None,
            floating_label_style: None,
            helper_style: None,
            helper_max_lines: None,
            hint_style: None,
            hint_fade_duration: None,
            hint_max_lines: None,
            error_style: None,
            error_max_lines: None,
            floating_label_behavior: None,
            floating_label_alignment: None,
            is_dense: None,
            content_padding: None,
            is_collapsed: None,
            icon_color: None,
            prefix_style: None,
            prefix_icon_color: None,
            prefix_icon_constraints: None,
            suffix_style: None,
            suffix_icon_color: None,
            suffix_icon_constraints: None,
            counter_style: None,
            filled: None,
            fill_color: None,
            active_indicator_border: None,
            outline_border: None,
            focus_color: None,
            hover_color: None,
            error_border: None,
            focused_border: None,
            focused_error_border: None,
            disabled_border: None,
            enabled_border: None,
            border: None,
            align_label_with_hint: None,
            constraints: None,
            visual_density: None,
            child: SizedBox::shrink().into_widget(),
        }
    }

    /// Dart `InputDecorationTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> InputDecorationTheme {
        self.key = Some(key);
        self
    }

    /// Dart `InputDecorationTheme(child:)`; the default is `SizedBox::shrink()`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> InputDecorationTheme {
        self.child = child.into_widget();
        self
    }

    /// Dart `InputDecorationTheme(data:)`.
    pub fn with_data(mut self, data: InputDecorationThemeData) -> InputDecorationTheme {
        debug_assert!(self.has_no_property());
        self.data = Some(data);
        self
    }

    /// Dart `InputDecorationTheme(labelStyle:)`.
    pub fn with_label_style(
        mut self,
        label_style: impl Into<AnyTextStyle>,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.label_style = Some(label_style.into());
        self
    }

    /// Dart `InputDecorationTheme(floatingLabelStyle:)`.
    pub fn with_floating_label_style(
        mut self,
        floating_label_style: impl Into<AnyTextStyle>,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.floating_label_style = Some(floating_label_style.into());
        self
    }

    /// Dart `InputDecorationTheme(helperStyle:)`.
    pub fn with_helper_style(
        mut self,
        helper_style: impl Into<AnyTextStyle>,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.helper_style = Some(helper_style.into());
        self
    }

    /// Dart `InputDecorationTheme(helperMaxLines:)`.
    pub fn with_helper_max_lines(mut self, helper_max_lines: i32) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.helper_max_lines = Some(helper_max_lines);
        self
    }

    /// Dart `InputDecorationTheme(hintStyle:)`.
    pub fn with_hint_style(mut self, hint_style: impl Into<AnyTextStyle>) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.hint_style = Some(hint_style.into());
        self
    }

    /// Dart `InputDecorationTheme(hintFadeDuration:)`.
    pub fn with_hint_fade_duration(mut self, hint_fade_duration: Duration) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.hint_fade_duration = Some(hint_fade_duration);
        self
    }

    /// Dart `InputDecorationTheme(hintMaxLines:)`.
    pub fn with_hint_max_lines(mut self, hint_max_lines: i32) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.hint_max_lines = Some(hint_max_lines);
        self
    }

    /// Dart `InputDecorationTheme(errorStyle:)`.
    pub fn with_error_style(
        mut self,
        error_style: impl Into<AnyTextStyle>,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.error_style = Some(error_style.into());
        self
    }

    /// Dart `InputDecorationTheme(errorMaxLines:)`.
    pub fn with_error_max_lines(mut self, error_max_lines: i32) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.error_max_lines = Some(error_max_lines);
        self
    }

    /// Dart `InputDecorationTheme(floatingLabelBehavior:)`.
    pub fn with_floating_label_behavior(
        mut self,
        floating_label_behavior: FloatingLabelBehavior,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.floating_label_behavior = Some(floating_label_behavior);
        self
    }

    /// Dart `InputDecorationTheme(floatingLabelAlignment:)`.
    pub fn with_floating_label_alignment(
        mut self,
        floating_label_alignment: FloatingLabelAlignment,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.floating_label_alignment = Some(floating_label_alignment);
        self
    }

    /// Dart `InputDecorationTheme(isDense:)`.
    pub fn with_is_dense(mut self, is_dense: bool) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.is_dense = Some(is_dense);
        self
    }

    /// Dart `InputDecorationTheme(contentPadding:)`.
    pub fn with_content_padding(
        mut self,
        content_padding: EdgeInsetsGeometry,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.content_padding = Some(content_padding);
        self
    }

    /// Dart `InputDecorationTheme(isCollapsed:)`.
    pub fn with_is_collapsed(mut self, is_collapsed: bool) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.is_collapsed = Some(is_collapsed);
        self
    }

    /// Dart `InputDecorationTheme(iconColor:)`.
    pub fn with_icon_color(mut self, icon_color: impl Into<AnyColor>) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.icon_color = Some(icon_color.into());
        self
    }

    /// Dart `InputDecorationTheme(prefixStyle:)`.
    pub fn with_prefix_style(
        mut self,
        prefix_style: impl Into<AnyTextStyle>,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.prefix_style = Some(prefix_style.into());
        self
    }

    /// Dart `InputDecorationTheme(prefixIconColor:)`.
    pub fn with_prefix_icon_color(
        mut self,
        prefix_icon_color: impl Into<AnyColor>,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.prefix_icon_color = Some(prefix_icon_color.into());
        self
    }

    /// Dart `InputDecorationTheme(prefixIconConstraints:)`.
    pub fn with_prefix_icon_constraints(
        mut self,
        prefix_icon_constraints: BoxConstraints,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.prefix_icon_constraints = Some(prefix_icon_constraints);
        self
    }

    /// Dart `InputDecorationTheme(suffixStyle:)`.
    pub fn with_suffix_style(
        mut self,
        suffix_style: impl Into<AnyTextStyle>,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.suffix_style = Some(suffix_style.into());
        self
    }

    /// Dart `InputDecorationTheme(suffixIconColor:)`.
    pub fn with_suffix_icon_color(
        mut self,
        suffix_icon_color: impl Into<AnyColor>,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.suffix_icon_color = Some(suffix_icon_color.into());
        self
    }

    /// Dart `InputDecorationTheme(suffixIconConstraints:)`.
    pub fn with_suffix_icon_constraints(
        mut self,
        suffix_icon_constraints: BoxConstraints,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.suffix_icon_constraints = Some(suffix_icon_constraints);
        self
    }

    /// Dart `InputDecorationTheme(counterStyle:)`.
    pub fn with_counter_style(
        mut self,
        counter_style: impl Into<AnyTextStyle>,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.counter_style = Some(counter_style.into());
        self
    }

    /// Dart `InputDecorationTheme(filled:)`.
    pub fn with_filled(mut self, filled: bool) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.filled = Some(filled);
        self
    }

    /// Dart `InputDecorationTheme(fillColor:)`.
    pub fn with_fill_color(mut self, fill_color: impl Into<AnyColor>) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.fill_color = Some(fill_color.into());
        self
    }

    /// Dart `InputDecorationTheme(activeIndicatorBorder:)`.
    pub fn with_active_indicator_border(
        mut self,
        active_indicator_border: impl Into<AnyBorderSide>,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.active_indicator_border = Some(active_indicator_border.into());
        self
    }

    /// Dart `InputDecorationTheme(outlineBorder:)`.
    pub fn with_outline_border(
        mut self,
        outline_border: impl Into<AnyBorderSide>,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.outline_border = Some(outline_border.into());
        self
    }

    /// Dart `InputDecorationTheme(focusColor:)`.
    pub fn with_focus_color(mut self, focus_color: Color) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.focus_color = Some(focus_color);
        self
    }

    /// Dart `InputDecorationTheme(hoverColor:)`.
    pub fn with_hover_color(mut self, hover_color: Color) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.hover_color = Some(hover_color);
        self
    }

    /// Dart `InputDecorationTheme(errorBorder:)`.
    pub fn with_error_border(
        mut self,
        error_border: impl Into<InputBorderRef>,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.error_border = Some(error_border.into());
        self
    }

    /// Dart `InputDecorationTheme(focusedBorder:)`.
    pub fn with_focused_border(
        mut self,
        focused_border: impl Into<InputBorderRef>,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.focused_border = Some(focused_border.into());
        self
    }

    /// Dart `InputDecorationTheme(focusedErrorBorder:)`.
    pub fn with_focused_error_border(
        mut self,
        focused_error_border: impl Into<InputBorderRef>,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.focused_error_border = Some(focused_error_border.into());
        self
    }

    /// Dart `InputDecorationTheme(disabledBorder:)`.
    pub fn with_disabled_border(
        mut self,
        disabled_border: impl Into<InputBorderRef>,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.disabled_border = Some(disabled_border.into());
        self
    }

    /// Dart `InputDecorationTheme(enabledBorder:)`.
    pub fn with_enabled_border(
        mut self,
        enabled_border: impl Into<InputBorderRef>,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.enabled_border = Some(enabled_border.into());
        self
    }

    /// Dart `InputDecorationTheme(border:)`.
    pub fn with_border(mut self, border: impl Into<InputBorderRef>) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.border = Some(border.into());
        self
    }

    /// Dart `InputDecorationTheme(alignLabelWithHint:)`.
    pub fn with_align_label_with_hint(
        mut self,
        align_label_with_hint: bool,
    ) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.align_label_with_hint = Some(align_label_with_hint);
        self
    }

    /// Dart `InputDecorationTheme(constraints:)`.
    pub fn with_constraints(mut self, constraints: BoxConstraints) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.constraints = Some(constraints);
        self
    }

    /// Dart `InputDecorationTheme(visualDensity:)`.
    pub fn with_visual_density(mut self, visual_density: VisualDensity) -> InputDecorationTheme {
        debug_assert!(self.data.is_none());
        self.visual_density = Some(visual_density);
        self
    }

    /// Overrides the default value for `InputDecoration::label_style`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::label_style` property in `data` instead.
    pub fn label_style(&self) -> Option<AnyTextStyle> {
        match &self.data {
            Some(data) => data.label_style.clone(),
            None => self.label_style.clone(),
        }
    }

    /// Overrides the default value for `InputDecoration::floating_label_style`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::floating_label_style` property in `data` instead.
    pub fn floating_label_style(&self) -> Option<AnyTextStyle> {
        match &self.data {
            Some(data) => data.floating_label_style.clone(),
            None => self.floating_label_style.clone(),
        }
    }

    /// Overrides the default value for `InputDecoration::floating_label_style`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::helper_style` property in `data` instead.
    pub fn helper_style(&self) -> Option<AnyTextStyle> {
        match &self.data {
            Some(data) => data.helper_style.clone(),
            None => self.helper_style.clone(),
        }
    }

    /// Overrides the default value for `InputDecoration::helper_max_lines`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::helper_max_lines` property in `data` instead.
    pub fn helper_max_lines(&self) -> Option<i32> {
        match &self.data {
            Some(data) => data.helper_max_lines,
            None => self.helper_max_lines,
        }
    }

    /// Overrides the default value for `InputDecoration::hint_style`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::hint_style` property in `data` instead.
    pub fn hint_style(&self) -> Option<AnyTextStyle> {
        match &self.data {
            Some(data) => data.hint_style.clone(),
            None => self.hint_style.clone(),
        }
    }

    /// Overrides the default value for `InputDecoration::hint_fade_duration`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::hint_fade_duration` property in `data` instead.
    pub fn hint_fade_duration(&self) -> Option<Duration> {
        match &self.data {
            Some(data) => data.hint_fade_duration,
            None => self.hint_fade_duration,
        }
    }

    /// Overrides the default value for `InputDecoration::hint_max_lines`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::hint_max_lines` property in `data` instead.
    pub fn hint_max_lines(&self) -> Option<i32> {
        match &self.data {
            Some(data) => data.hint_max_lines,
            None => self.hint_max_lines,
        }
    }

    /// Overrides the default value for `InputDecoration::error_style`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::error_style` property in `data` instead.
    pub fn error_style(&self) -> Option<AnyTextStyle> {
        match &self.data {
            Some(data) => data.error_style.clone(),
            None => self.error_style.clone(),
        }
    }

    /// Overrides the default value for `InputDecoration::error_max_lines`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::error_max_lines` property in `data` instead.
    pub fn error_max_lines(&self) -> Option<i32> {
        match &self.data {
            Some(data) => data.error_max_lines,
            None => self.error_max_lines,
        }
    }

    /// Overrides the default value for `InputDecoration::floating_label_behavior`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::floating_label_behavior` property in `data` instead.
    pub fn floating_label_behavior(&self) -> FloatingLabelBehavior {
        match &self.data {
            Some(data) => data.floating_label_behavior,
            None => self
                .floating_label_behavior
                .unwrap_or(FloatingLabelBehavior::Auto),
        }
    }

    /// Overrides the default value for `InputDecoration::floating_label_alignment`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::floating_label_alignment` property in `data` instead.
    pub fn floating_label_alignment(&self) -> FloatingLabelAlignment {
        match &self.data {
            Some(data) => data.floating_label_alignment,
            None => self
                .floating_label_alignment
                .unwrap_or(FloatingLabelAlignment::START),
        }
    }

    /// Overrides the default value for `InputDecoration::is_dense`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::is_dense` property in `data` instead.
    pub fn is_dense(&self) -> bool {
        match &self.data {
            Some(data) => data.is_dense,
            None => self.is_dense.unwrap_or(false),
        }
    }

    /// Overrides the default value for `InputDecoration::content_padding`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::content_padding` property in `data` instead.
    pub fn content_padding(&self) -> Option<EdgeInsetsGeometry> {
        match &self.data {
            Some(data) => data.content_padding,
            None => self.content_padding,
        }
    }

    /// Overrides the default value for `InputDecoration::is_collapsed`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::is_collapsed` property in `data` instead.
    pub fn is_collapsed(&self) -> bool {
        match &self.data {
            Some(data) => data.is_collapsed,
            None => self.is_collapsed.unwrap_or(false),
        }
    }

    /// Overrides the default value for `InputDecoration::icon_color`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::icon_color` property in `data` instead.
    pub fn icon_color(&self) -> Option<AnyColor> {
        match &self.data {
            Some(data) => data.icon_color.clone(),
            None => self.icon_color.clone(),
        }
    }

    /// Overrides the default value for `InputDecoration::prefix_style`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::prefix_style` property in `data` instead.
    pub fn prefix_style(&self) -> Option<AnyTextStyle> {
        match &self.data {
            Some(data) => data.prefix_style.clone(),
            None => self.prefix_style.clone(),
        }
    }

    /// Overrides the default value for `InputDecoration::prefix_icon_color`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::prefix_icon_color` property in `data` instead.
    pub fn prefix_icon_color(&self) -> Option<AnyColor> {
        match &self.data {
            Some(data) => data.prefix_icon_color.clone(),
            None => self.prefix_icon_color.clone(),
        }
    }

    /// Overrides the default value for `InputDecoration::prefix_icon_constraints`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::prefix_icon_constraints` property in `data` instead.
    pub fn prefix_icon_constraints(&self) -> Option<BoxConstraints> {
        match &self.data {
            Some(data) => data.prefix_icon_constraints,
            None => self.prefix_icon_constraints,
        }
    }

    /// Overrides the default value for `InputDecoration::suffix_style`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::suffix_style` property in `data` instead.
    pub fn suffix_style(&self) -> Option<AnyTextStyle> {
        match &self.data {
            Some(data) => data.suffix_style.clone(),
            None => self.suffix_style.clone(),
        }
    }

    /// Overrides the default value for `InputDecoration::suffix_icon_color`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::suffix_icon_color` property in `data` instead.
    pub fn suffix_icon_color(&self) -> Option<AnyColor> {
        match &self.data {
            Some(data) => data.suffix_icon_color.clone(),
            None => self.suffix_icon_color.clone(),
        }
    }

    /// Overrides the default value for `InputDecoration::suffix_icon_constraints`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::suffix_icon_constraints` property in `data` instead.
    pub fn suffix_icon_constraints(&self) -> Option<BoxConstraints> {
        match &self.data {
            Some(data) => data.suffix_icon_constraints,
            None => self.suffix_icon_constraints,
        }
    }

    /// Overrides the default value for `InputDecoration::counter_style`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::counter_style` property in `data` instead.
    pub fn counter_style(&self) -> Option<AnyTextStyle> {
        match &self.data {
            Some(data) => data.counter_style.clone(),
            None => self.counter_style.clone(),
        }
    }

    /// Overrides the default value for `InputDecoration::filled`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::filled` property in `data` instead.
    pub fn filled(&self) -> bool {
        match &self.data {
            Some(data) => data.filled,
            None => self.filled.unwrap_or(false),
        }
    }

    /// Overrides the default value for `InputDecoration::fill_color`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::fill_color` property in `data` instead.
    pub fn fill_color(&self) -> Option<AnyColor> {
        match &self.data {
            Some(data) => data.fill_color.clone(),
            None => self.fill_color.clone(),
        }
    }

    /// The borderSide of the UnderlineInputBorder with `color` and `weight`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::active_indicator_border` property in `data` instead.
    pub fn active_indicator_border(&self) -> Option<AnyBorderSide> {
        match &self.data {
            Some(data) => data.active_indicator_border.clone(),
            None => self.active_indicator_border.clone(),
        }
    }

    /// The borderSide of the OutlineInputBorder with `color` and `weight`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::outline_border` property in `data` instead.
    pub fn outline_border(&self) -> Option<AnyBorderSide> {
        match &self.data {
            Some(data) => data.outline_border.clone(),
            None => self.outline_border.clone(),
        }
    }

    /// Overrides the default value for `InputDecoration::focus_color`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::focus_color` property in `data` instead.
    pub fn focus_color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.focus_color,
            None => self.focus_color,
        }
    }

    /// Overrides the default value for `InputDecoration::hover_color`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::hover_color` property in `data` instead.
    pub fn hover_color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.hover_color,
            None => self.hover_color,
        }
    }

    /// Overrides the default value for `InputDecoration::error_border`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::error_border` property in `data` instead.
    pub fn error_border(&self) -> Option<&InputBorderRef> {
        match &self.data {
            Some(data) => data.error_border.as_ref(),
            None => self.error_border.as_ref(),
        }
    }

    /// Overrides the default value for `InputDecoration::focused_border`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::focused_border` property in `data` instead.
    pub fn focused_border(&self) -> Option<&InputBorderRef> {
        match &self.data {
            Some(data) => data.focused_border.as_ref(),
            None => self.focused_border.as_ref(),
        }
    }

    /// Overrides the default value for `InputDecoration::focused_error_border`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::focused_error_border` property in `data` instead.
    pub fn focused_error_border(&self) -> Option<&InputBorderRef> {
        match &self.data {
            Some(data) => data.focused_error_border.as_ref(),
            None => self.focused_error_border.as_ref(),
        }
    }

    /// Overrides the default value for `InputDecoration::disabled_border`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::disabled_border` property in `data` instead.
    pub fn disabled_border(&self) -> Option<&InputBorderRef> {
        match &self.data {
            Some(data) => data.disabled_border.as_ref(),
            None => self.disabled_border.as_ref(),
        }
    }

    /// Overrides the default value for `InputDecoration::enabled_border`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::enabled_border` property in `data` instead.
    pub fn enabled_border(&self) -> Option<&InputBorderRef> {
        match &self.data {
            Some(data) => data.enabled_border.as_ref(),
            None => self.enabled_border.as_ref(),
        }
    }

    /// Overrides the default value for `InputDecoration::border`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::border` property in `data` instead.
    pub fn border(&self) -> Option<&InputBorderRef> {
        match &self.data {
            Some(data) => data.border.as_ref(),
            None => self.border.as_ref(),
        }
    }

    /// Overrides the default value for `InputDecoration::align_label_with_hint`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::align_label_with_hint` property in `data` instead.
    pub fn align_label_with_hint(&self) -> bool {
        match &self.data {
            Some(data) => data.align_label_with_hint,
            None => self.align_label_with_hint.unwrap_or(false),
        }
    }

    /// Overrides the default value for `InputDecoration::constraints`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::constraints` property in `data` instead.
    pub fn constraints(&self) -> Option<BoxConstraints> {
        match &self.data {
            Some(data) => data.constraints,
            None => self.constraints,
        }
    }

    /// Overrides the default value for `InputDecoration::visual_density`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::visual_density` property in `data` instead.
    pub fn visual_density(&self) -> Option<VisualDensity> {
        match &self.data {
            Some(data) => data.visual_density,
            None => self.visual_density,
        }
    }

    /// The properties used for all descendant `TabBar` widgets.
    pub fn data(&self) -> InputDecorationThemeData {
        self.data
            .clone()
            .unwrap_or_else(|| InputDecorationThemeData {
                label_style: self.label_style.clone(),
                floating_label_style: self.floating_label_style.clone(),
                helper_style: self.helper_style.clone(),
                helper_max_lines: self.helper_max_lines,
                hint_style: self.hint_style.clone(),
                hint_fade_duration: self.hint_fade_duration,
                hint_max_lines: self.hint_max_lines,
                error_style: self.error_style.clone(),
                error_max_lines: self.error_max_lines,
                floating_label_behavior: self
                    .floating_label_behavior
                    .unwrap_or(FloatingLabelBehavior::Auto),
                floating_label_alignment: self
                    .floating_label_alignment
                    .unwrap_or(FloatingLabelAlignment::START),
                is_dense: self.is_dense.unwrap_or(false),
                content_padding: self.content_padding,
                is_collapsed: self.is_collapsed.unwrap_or(false),
                icon_color: self.icon_color.clone(),
                prefix_style: self.prefix_style.clone(),
                prefix_icon_color: self.prefix_icon_color.clone(),
                prefix_icon_constraints: self.prefix_icon_constraints,
                suffix_style: self.suffix_style.clone(),
                suffix_icon_color: self.suffix_icon_color.clone(),
                suffix_icon_constraints: self.suffix_icon_constraints,
                counter_style: self.counter_style.clone(),
                filled: self.filled.unwrap_or(false),
                fill_color: self.fill_color.clone(),
                active_indicator_border: self.active_indicator_border.clone(),
                outline_border: self.outline_border.clone(),
                focus_color: self.focus_color,
                hover_color: self.hover_color,
                error_border: self.error_border.clone(),
                focused_border: self.focused_border.clone(),
                focused_error_border: self.focused_error_border.clone(),
                disabled_border: self.disabled_border.clone(),
                enabled_border: self.enabled_border.clone(),
                border: self.border.clone(),
                align_label_with_hint: self.align_label_with_hint.unwrap_or(false),
                constraints: self.constraints,
                visual_density: self.visual_density,
                supporting_text_padding: None,
            })
    }

    /// Returns the closest `InputDecorationThemeData` instance given the build context.
    ///
    /// If there is no enclosing `InputDecorationTheme` widget, then
    /// `ThemeData::input_decoration_theme` is used.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// InputDecorationThemeData theme = InputDecorationTheme.of(context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> InputDecorationThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<InputDecorationTheme>(app) {
            Some(input_decoration_theme) => input_decoration_theme.data(),
            None => Theme::of(app, context).input_decoration_theme.clone(),
        }
    }

    /// Creates a copy of this object but with the given fields replaced with the
    /// new values.
    ///
    /// This method is obsolete and will be deprecated in a future release:
    /// please use the `InputDecorationThemeData::copy_with` instead.
    ///
    /// Dart's named arguments are the `with_*` setters on the copy
    /// (`theme.copy_with().with_is_dense(true)`). As in Dart, the copy carries neither the key
    /// nor the child, and holds the properties, never a [`data`](Self::data).
    pub fn copy_with(&self) -> InputDecorationTheme {
        InputDecorationTheme {
            key: None,
            data: None,
            label_style: self.label_style(),
            floating_label_style: self.floating_label_style(),
            helper_style: self.helper_style(),
            helper_max_lines: self.helper_max_lines(),
            hint_style: self.hint_style(),
            hint_fade_duration: self.hint_fade_duration(),
            hint_max_lines: self.hint_max_lines(),
            error_style: self.error_style(),
            error_max_lines: self.error_max_lines(),
            floating_label_behavior: Some(self.floating_label_behavior()),
            floating_label_alignment: Some(self.floating_label_alignment()),
            is_dense: Some(self.is_dense()),
            content_padding: self.content_padding(),
            is_collapsed: Some(self.is_collapsed()),
            icon_color: self.icon_color(),
            prefix_style: self.prefix_style(),
            prefix_icon_color: self.prefix_icon_color(),
            prefix_icon_constraints: self.prefix_icon_constraints(),
            suffix_style: self.suffix_style(),
            suffix_icon_color: self.suffix_icon_color(),
            suffix_icon_constraints: self.suffix_icon_constraints(),
            counter_style: self.counter_style(),
            filled: Some(self.filled()),
            fill_color: self.fill_color(),
            active_indicator_border: self.active_indicator_border(),
            outline_border: self.outline_border(),
            focus_color: self.focus_color(),
            hover_color: self.hover_color(),
            error_border: self.error_border().cloned(),
            focused_border: self.focused_border().cloned(),
            focused_error_border: self.focused_error_border().cloned(),
            disabled_border: self.disabled_border().cloned(),
            enabled_border: self.enabled_border().cloned(),
            border: self.border().cloned(),
            align_label_with_hint: Some(self.align_label_with_hint()),
            constraints: self.constraints(),
            visual_density: self.visual_density(),
            child: SizedBox::shrink().into_widget(),
        }
    }

    /// Returns a copy of this InputDecorationTheme where the non-null fields in
    /// the given InputDecorationTheme override the corresponding nullable fields
    /// in this InputDecorationTheme.
    ///
    /// The non-nullable fields of InputDecorationTheme, such as `floating_label_behavior`,
    /// `is_dense`, `is_collapsed`, `filled`, and `align_label_with_hint` cannot be overridden.
    ///
    /// In other words, the fields of the provided `InputDecorationTheme` are used to
    /// fill in the unspecified and nullable fields of this InputDecorationTheme.
    pub fn merge(&self, other: Option<&InputDecorationTheme>) -> InputDecorationTheme {
        let Some(other) = other else {
            return self.clone();
        };
        let mut merged = self.copy_with();
        merged.label_style = self.label_style().or_else(|| other.label_style());
        merged.floating_label_style = self
            .floating_label_style()
            .or_else(|| other.floating_label_style());
        merged.helper_style = self.helper_style().or_else(|| other.helper_style());
        merged.helper_max_lines = self.helper_max_lines().or_else(|| other.helper_max_lines());
        merged.hint_style = self.hint_style().or_else(|| other.hint_style());
        merged.hint_fade_duration = self
            .hint_fade_duration()
            .or_else(|| other.hint_fade_duration());
        merged.hint_max_lines = self.hint_max_lines().or_else(|| other.hint_max_lines());
        merged.error_style = self.error_style().or_else(|| other.error_style());
        merged.error_max_lines = self.error_max_lines().or_else(|| other.error_max_lines());
        merged.content_padding = self.content_padding().or_else(|| other.content_padding());
        merged.icon_color = self.icon_color().or_else(|| other.icon_color());
        merged.prefix_style = self.prefix_style().or_else(|| other.prefix_style());
        merged.prefix_icon_color = self
            .prefix_icon_color()
            .or_else(|| other.prefix_icon_color());
        merged.prefix_icon_constraints = self
            .prefix_icon_constraints()
            .or_else(|| other.prefix_icon_constraints());
        merged.suffix_style = self.suffix_style().or_else(|| other.suffix_style());
        merged.suffix_icon_color = self
            .suffix_icon_color()
            .or_else(|| other.suffix_icon_color());
        merged.suffix_icon_constraints = self
            .suffix_icon_constraints()
            .or_else(|| other.suffix_icon_constraints());
        merged.counter_style = self.counter_style().or_else(|| other.counter_style());
        merged.fill_color = self.fill_color().or_else(|| other.fill_color());
        merged.active_indicator_border = self
            .active_indicator_border()
            .or_else(|| other.active_indicator_border());
        merged.outline_border = self.outline_border().or_else(|| other.outline_border());
        merged.focus_color = self.focus_color().or_else(|| other.focus_color());
        merged.hover_color = self.hover_color().or_else(|| other.hover_color());
        merged.error_border = self.error_border().or(other.error_border()).cloned();
        merged.focused_border = self.focused_border().or(other.focused_border()).cloned();
        merged.focused_error_border = self
            .focused_error_border()
            .or(other.focused_error_border())
            .cloned();
        merged.disabled_border = self.disabled_border().or(other.disabled_border()).cloned();
        merged.enabled_border = self.enabled_border().or(other.enabled_border()).cloned();
        merged.border = self.border().or(other.border()).cloned();
        merged.constraints = self.constraints().or_else(|| other.constraints());
        merged.visual_density = self.visual_density().or_else(|| other.visual_density());
        merged
    }

    /// Dart's constructor assert: a `data` takes the place of every property.
    fn has_no_property(&self) -> bool {
        self.label_style.is_none()
            && self.floating_label_style.is_none()
            && self.helper_style.is_none()
            && self.helper_max_lines.is_none()
            && self.hint_style.is_none()
            && self.hint_fade_duration.is_none()
            && self.hint_max_lines.is_none()
            && self.error_style.is_none()
            && self.error_max_lines.is_none()
            && self.floating_label_behavior.is_none()
            && self.floating_label_alignment.is_none()
            && self.is_dense.is_none()
            && self.content_padding.is_none()
            && self.is_collapsed.is_none()
            && self.icon_color.is_none()
            && self.prefix_style.is_none()
            && self.prefix_icon_color.is_none()
            && self.prefix_icon_constraints.is_none()
            && self.suffix_style.is_none()
            && self.suffix_icon_color.is_none()
            && self.suffix_icon_constraints.is_none()
            && self.counter_style.is_none()
            && self.filled.is_none()
            && self.fill_color.is_none()
            && self.active_indicator_border.is_none()
            && self.outline_border.is_none()
            && self.focus_color.is_none()
            && self.hover_color.is_none()
            && self.error_border.is_none()
            && self.focused_border.is_none()
            && self.focused_error_border.is_none()
            && self.disabled_border.is_none()
            && self.enabled_border.is_none()
            && self.border.is_none()
            && self.align_label_with_hint.is_none()
            && self.constraints.is_none()
            && self.visual_density.is_none()
    }
}

impl Default for InputDecorationTheme {
    fn default() -> InputDecorationTheme {
        InputDecorationTheme::new()
    }
}

impl From<InputDecorationTheme> for InputDecorationThemeData {
    /// Dart's `inputDecorationTheme.data`, where an API takes either type.
    fn from(theme: InputDecorationTheme) -> InputDecorationThemeData {
        theme.data()
    }
}

impl InheritedTheme for InputDecorationTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        InputDecorationTheme::new()
            .with_data(self.data())
            .child(child)
            .into_widget()
    }
}

impl InheritedWidget for InputDecorationTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &InputDecorationTheme) -> bool {
        self.data() != old_widget.data()
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

/// Defines the default appearance of `InputDecorator`s.
///
/// Descendant widgets obtain the current theme's `InputDecorationThemeData` using
/// `InputDecorationTheme::of`. When a widget uses `InputDecorationTheme::of`, it is
/// automatically rebuilt if the theme later changes.
///
/// The `InputDecoration::apply_defaults` method is used to combine an input
/// decoration theme with an `InputDecoration` object.
///
/// See also:
///
///  * `ThemeData::input_decoration_theme`, which specifies an input decoration theme as
///    part of the overall Material theme.
#[derive(Clone, PartialEq)]
pub struct InputDecorationThemeData {
    /// The style to use for `InputDecoration::label_text` when the label is on top
    /// of the input field.
    ///
    /// If `label_style` is a `WidgetStateTextStyle`, then the effective
    /// text style can depend on the `WidgetState::Focused` state, i.e.
    /// if the `TextField` is focused or not.
    ///
    /// When the `InputDecoration::label_text` is above (i.e., vertically adjacent to)
    /// the input field, the text uses the `floating_label_style` instead.
    ///
    /// If null, defaults to a value derived from the base `TextStyle` for the
    /// input field and the current `Theme`.
    ///
    /// Specifying this style will override the default behavior
    /// of `InputDecoration` that changes the color of the label to the
    /// `InputDecoration::error_style` color or `ColorScheme::error`.
    pub label_style: Option<AnyTextStyle>,

    /// The style to use for `InputDecoration::label_text` when the label is
    /// above (i.e., vertically adjacent to) the input field.
    ///
    /// When the `InputDecoration::label_text` is on top of the input field, the
    /// text uses the `label_style` instead.
    ///
    /// If `floating_label_style` is a `WidgetStateTextStyle`, then the effective
    /// text style can depend on the `WidgetState::Focused` state, i.e.
    /// if the `TextField` is focused or not.
    ///
    /// If null, defaults to `label_style`.
    ///
    /// Specifying this style will override the default behavior
    /// of `InputDecoration` that changes the color of the label to the
    /// `InputDecoration::error_style` color or `ColorScheme::error`.
    ///
    /// When the input field receives focus, the font size of `InputDecoration::label` is
    /// scaled down by 75%.
    pub floating_label_style: Option<AnyTextStyle>,

    /// The style to use for `InputDecoration::helper_text`.
    ///
    /// If `helper_style` is a `WidgetStateTextStyle`, then the effective
    /// text style can depend on the `WidgetState::Focused` state, i.e.
    /// if the `TextField` is focused or not.
    pub helper_style: Option<AnyTextStyle>,

    /// The maximum number of lines the `InputDecoration::helper_text` can occupy.
    ///
    /// Defaults to null, which means that the `InputDecoration::helper_text` will
    /// be limited to a single line with `TextOverflow::Ellipsis`.
    ///
    /// This value is passed along to the `Text::max_lines` attribute
    /// of the `Text` widget used to display the helper.
    ///
    /// See also:
    ///
    ///  * `error_max_lines`, the equivalent but for the `InputDecoration::error_text`.
    pub helper_max_lines: Option<i32>,

    /// The style to use for the `InputDecoration::hint_text`.
    ///
    /// If `hint_style` is a `WidgetStateTextStyle`, then the effective
    /// text style can depend on the `WidgetState::Focused` state, i.e.
    /// if the `TextField` is focused or not.
    ///
    /// Also used for the `InputDecoration::label_text` when the
    /// `InputDecoration::label_text` is displayed on top of the input field (i.e.,
    /// at the same location on the screen where text may be entered in the input
    /// field).
    ///
    /// If null, defaults to a value derived from the base `TextStyle` for the
    /// input field and the current `Theme`.
    pub hint_style: Option<AnyTextStyle>,

    /// The duration of the `InputDecoration::hint_text` fade in and fade out animations.
    pub hint_fade_duration: Option<Duration>,

    /// The maximum number of lines the `InputDecoration::hint_text` can occupy.
    ///
    /// Defaults to null, which means that the `InputDecoration::hint_text` will
    /// be limited to a single line with `TextOverflow::Ellipsis`.
    ///
    /// This value is passed along to the `Text::max_lines` attribute
    /// of the `Text` widget used to display the hint text.
    pub hint_max_lines: Option<i32>,

    /// The style to use for the `InputDecoration::error_text`.
    ///
    /// If null, defaults of a value derived from the base `TextStyle` for the
    /// input field and the current `Theme`.
    ///
    /// By default the color of style will be used by the label of
    /// `InputDecoration` if `InputDecoration::error_text` is not null. See
    /// `InputDecoration::label_style` or `InputDecoration::floating_label_style` for
    /// an example of how to replicate this behavior when specifying those
    /// styles.
    pub error_style: Option<AnyTextStyle>,

    /// The maximum number of lines the `InputDecoration::error_text` can occupy.
    ///
    /// Defaults to null, which means that the `InputDecoration::error_text` will be
    /// limited to a single line with `TextOverflow::Ellipsis`.
    ///
    /// This value is passed along to the `Text::max_lines` attribute
    /// of the `Text` widget used to display the error.
    ///
    /// See also:
    ///
    ///  * `helper_max_lines`, the equivalent but for the `InputDecoration::helper_text`.
    pub error_max_lines: Option<i32>,

    /// Defines **how** the floating label should behave.
    ///
    /// When `FloatingLabelBehavior::Auto` the label will float to the top only when
    /// the field is focused or has some text content, otherwise it will appear
    /// in the field in place of the content.
    ///
    /// When `FloatingLabelBehavior::Always` the label will always float at the top
    /// of the field above the content.
    ///
    /// When `FloatingLabelBehavior::Never` the label will always appear in an empty
    /// field in place of the content.
    ///
    /// Defaults to `FloatingLabelBehavior::Auto`.
    pub floating_label_behavior: FloatingLabelBehavior,

    /// Defines **where** the floating label should be displayed.
    ///
    /// `FloatingLabelAlignment::START` aligns the floating label to the leftmost
    /// (when `TextDirection::Ltr`) or rightmost (when `TextDirection::Rtl`),
    /// possible position, which is vertically adjacent to the label, on top of
    /// the field.
    ///
    /// `FloatingLabelAlignment::CENTER` aligns the floating label to the center on
    /// top of the field.
    ///
    /// Defaults to `FloatingLabelAlignment::START`.
    pub floating_label_alignment: FloatingLabelAlignment,

    /// Whether the input decorator's child is part of a dense form (i.e., uses
    /// less vertical space).
    ///
    /// Defaults to false.
    pub is_dense: bool,

    /// The padding for the input decoration's container.
    ///
    /// The decoration's container is the area which is filled if
    /// `InputDecoration::filled` is true and bordered per the `border`.
    /// It's the area adjacent to `InputDecoration::icon` and above the
    /// `InputDecoration::icon` and above the widgets that contain
    /// `InputDecoration::helper_text`, `InputDecoration::error_text`, and
    /// `InputDecoration::counter_text`.
    ///
    /// By default the `content_padding` reflects `visual_density`, `is_dense` and
    /// the type of the `border`. If `is_collapsed` is true then `content_padding`
    /// is `EdgeInsets::ZERO`.
    pub content_padding: Option<EdgeInsetsGeometry>,

    /// Whether the decoration is the same size as the input field.
    ///
    /// A collapsed decoration cannot have `InputDecoration::label_text`,
    /// `InputDecoration::error_text`, or an `InputDecoration::icon`.
    pub is_collapsed: bool,

    /// The Color to use for the `InputDecoration::icon`.
    ///
    /// If `icon_color` is a `WidgetStateColor`, then the effective
    /// color can depend on the `WidgetState::Focused` state, i.e.
    /// if the `TextField` is focused or not.
    ///
    /// If null, defaults to the `ColorScheme::primary`.
    pub icon_color: Option<AnyColor>,

    /// The style to use for the `InputDecoration::prefix_text`.
    ///
    /// If `prefix_style` is a `WidgetStateTextStyle`, then the effective
    /// text style can depend on the `WidgetState::Focused` state, i.e.
    /// if the `TextField` is focused or not.
    ///
    /// If null, defaults to the `hint_style`.
    pub prefix_style: Option<AnyTextStyle>,

    /// The Color to use for the `InputDecoration::prefix_icon`.
    ///
    /// If `prefix_icon_color` is a `WidgetStateColor`, then the effective
    /// color can depend on the `WidgetState::Focused` state, i.e.
    /// if the `TextField` is focused or not.
    ///
    /// If null, defaults to the `ColorScheme::primary`.
    pub prefix_icon_color: Option<AnyColor>,

    /// The constraints to use for `InputDecoration::prefix_icon_constraints`.
    ///
    /// This can be used to modify the `BoxConstraints` surrounding
    /// `InputDecoration::prefix_icon`.
    ///
    /// This property is particularly useful for getting the decoration's height
    /// less than the minimum tappable height (which is 48px when the visual
    /// density is set to `VisualDensity::standard`). This can be achieved by
    /// setting `is_dense` to true and setting the constraints' minimum height
    /// and width to a value lower than the minimum tappable size.
    ///
    /// If null, `BoxConstraints` with a minimum width and height of 48px is
    /// used.
    pub prefix_icon_constraints: Option<BoxConstraints>,

    /// The style to use for the `InputDecoration::suffix_text`.
    ///
    /// If `suffix_style` is a `WidgetStateTextStyle`, then the effective
    /// color can depend on the `WidgetState::Focused` state, i.e.
    /// if the `TextField` is focused or not.
    ///
    /// If null, defaults to the `hint_style`.
    pub suffix_style: Option<AnyTextStyle>,

    /// The Color to use for the `InputDecoration::suffix_icon`.
    ///
    /// If `suffix_icon_color` is a `WidgetStateColor`, then the effective
    /// color can depend on the `WidgetState::Focused` state, i.e.
    /// if the `TextField` is focused or not.
    ///
    /// If null, defaults to the `ColorScheme::primary`.
    pub suffix_icon_color: Option<AnyColor>,

    /// The constraints to use for `InputDecoration::suffix_icon_constraints`.
    ///
    /// This can be used to modify the `BoxConstraints` surrounding
    /// `InputDecoration::suffix_icon`.
    ///
    /// This property is particularly useful for getting the decoration's height
    /// less than the minimum tappable height (which is 48px when the visual
    /// density is set to `VisualDensity::standard`). This can be achieved by
    /// setting `is_dense` to true and setting the constraints' minimum height
    /// and width to a value lower than the minimum tappable size.
    ///
    /// If null, `BoxConstraints` with a minimum width and height of 48px is
    /// used.
    pub suffix_icon_constraints: Option<BoxConstraints>,

    /// The style to use for the `InputDecoration::counter_text`.
    ///
    /// If `counter_style` is a `WidgetStateTextStyle`, then the effective
    /// text style can depend on the `WidgetState::Focused` state, i.e.
    /// if the `TextField` is focused or not.
    ///
    /// If null, defaults to the `helper_style`.
    pub counter_style: Option<AnyTextStyle>,

    /// If true the decoration's container is filled with `fill_color`.
    ///
    /// Typically this field set to true if `border` is an
    /// `UnderlineInputBorder`.
    ///
    /// The decoration's container is the area, defined by the border's
    /// `InputBorder::get_outer_path`, which is filled if `filled` is
    /// true and bordered per the `border`.
    ///
    /// This property is false by default.
    pub filled: bool,

    /// The color to fill the decoration's container with, if `filled` is true.
    ///
    /// By default the fillColor is based on the current `Theme`.
    ///
    /// The decoration's container is the area, defined by the border's
    /// `InputBorder::get_outer_path`, which is filled if `filled` is
    /// true and bordered per the `border`.
    pub fill_color: Option<AnyColor>,

    /// The borderSide of the OutlineInputBorder with `color` and `weight`.
    pub outline_border: Option<AnyBorderSide>,

    /// The borderSide of the UnderlineInputBorder with `color` and `weight`.
    pub active_indicator_border: Option<AnyBorderSide>,

    /// The color to blend with the decoration's `fill_color` with, if `filled` is
    /// true and the container has the input focus.
    ///
    /// By default the `focus_color` is based on the current `Theme`.
    ///
    /// The decoration's container is the area, defined by the border's
    /// `InputBorder::get_outer_path`, which is filled if `filled` is
    /// true and bordered per the `border`.
    pub focus_color: Option<Color>,

    /// The color to blend with the decoration's `fill_color` with, if the
    /// decoration is being hovered over by a mouse pointer.
    ///
    /// By default the `hover_color` is based on the current `Theme`.
    ///
    /// The decoration's container is the area, defined by the border's
    /// `InputBorder::get_outer_path`, which is filled if `filled` is
    /// true and bordered per the `border`.
    ///
    /// The container will be filled when hovered over even if `filled` is false.
    pub hover_color: Option<Color>,

    /// The border to display when the `InputDecorator` does not have the focus and
    /// is showing an error.
    ///
    /// See also:
    ///
    ///  * `InputDecorator::is_focused`, which is true if the `InputDecorator`'s child
    ///    has the focus.
    ///  * `InputDecoration::error_text`, the error shown by the `InputDecorator`, if non-null.
    ///  * `border`, for a description of where the `InputDecorator` border appears.
    ///  * `UnderlineInputBorder`, an `InputDecorator` border which draws a horizontal
    ///    line at the bottom of the input decorator's container.
    ///  * `OutlineInputBorder`, an `InputDecorator` border which draws a
    ///    rounded rectangle around the input decorator's container.
    ///  * `InputBorderRef::none()`, which doesn't draw a border.
    ///  * `focused_border`, displayed when `InputDecorator::is_focused` is true
    ///    and `InputDecoration::error_text` is null.
    ///  * `focused_error_border`, displayed when `InputDecorator::is_focused` is true
    ///    and `InputDecoration::error_text` is non-null.
    ///  * `disabled_border`, displayed when `InputDecoration::enabled` is false
    ///    and `InputDecoration::error_text` is null.
    ///  * `enabled_border`, displayed when `InputDecoration::enabled` is true
    ///    and `InputDecoration::error_text` is null.
    pub error_border: Option<InputBorderRef>,

    /// The border to display when the `InputDecorator` has the focus and is not
    /// showing an error.
    ///
    /// See also:
    ///
    ///  * `InputDecorator::is_focused`, which is true if the `InputDecorator`'s child
    ///    has the focus.
    ///  * `InputDecoration::error_text`, the error shown by the `InputDecorator`, if non-null.
    ///  * `border`, for a description of where the `InputDecorator` border appears.
    ///  * `UnderlineInputBorder`, an `InputDecorator` border which draws a horizontal
    ///    line at the bottom of the input decorator's container.
    ///  * `OutlineInputBorder`, an `InputDecorator` border which draws a
    ///    rounded rectangle around the input decorator's container.
    ///  * `InputBorderRef::none()`, which doesn't draw a border.
    ///  * `error_border`, displayed when `InputDecorator::is_focused` is false
    ///    and `InputDecoration::error_text` is non-null.
    ///  * `focused_error_border`, displayed when `InputDecorator::is_focused` is true
    ///    and `InputDecoration::error_text` is non-null.
    ///  * `disabled_border`, displayed when `InputDecoration::enabled` is false
    ///    and `InputDecoration::error_text` is null.
    ///  * `enabled_border`, displayed when `InputDecoration::enabled` is true
    ///    and `InputDecoration::error_text` is null.
    pub focused_border: Option<InputBorderRef>,

    /// The border to display when the `InputDecorator` has the focus and is
    /// showing an error.
    ///
    /// See also:
    ///
    ///  * `InputDecorator::is_focused`, which is true if the `InputDecorator`'s child
    ///    has the focus.
    ///  * `InputDecoration::error_text`, the error shown by the `InputDecorator`, if non-null.
    ///  * `border`, for a description of where the `InputDecorator` border appears.
    ///  * `UnderlineInputBorder`, an `InputDecorator` border which draws a horizontal
    ///    line at the bottom of the input decorator's container.
    ///  * `OutlineInputBorder`, an `InputDecorator` border which draws a
    ///    rounded rectangle around the input decorator's container.
    ///  * `InputBorderRef::none()`, which doesn't draw a border.
    ///  * `error_border`, displayed when `InputDecorator::is_focused` is false
    ///    and `InputDecoration::error_text` is non-null.
    ///  * `focused_border`, displayed when `InputDecorator::is_focused` is true
    ///    and `InputDecoration::error_text` is null.
    ///  * `disabled_border`, displayed when `InputDecoration::enabled` is false
    ///    and `InputDecoration::error_text` is null.
    ///  * `enabled_border`, displayed when `InputDecoration::enabled` is true
    ///    and `InputDecoration::error_text` is null.
    pub focused_error_border: Option<InputBorderRef>,

    /// The border to display when the `InputDecorator` is disabled and is not
    /// showing an error.
    ///
    /// See also:
    ///
    ///  * `InputDecoration::enabled`, which is false if the `InputDecorator` is disabled.
    ///  * `InputDecoration::error_text`, the error shown by the `InputDecorator`, if non-null.
    ///  * `border`, for a description of where the `InputDecorator` border appears.
    ///  * `UnderlineInputBorder`, an `InputDecorator` border which draws a horizontal
    ///    line at the bottom of the input decorator's container.
    ///  * `OutlineInputBorder`, an `InputDecorator` border which draws a
    ///    rounded rectangle around the input decorator's container.
    ///  * `InputBorderRef::none()`, which doesn't draw a border.
    ///  * `error_border`, displayed when `InputDecorator::is_focused` is false
    ///    and `InputDecoration::error_text` is non-null.
    ///  * `focused_border`, displayed when `InputDecorator::is_focused` is true
    ///    and `InputDecoration::error_text` is null.
    ///  * `focused_error_border`, displayed when `InputDecorator::is_focused` is true
    ///    and `InputDecoration::error_text` is non-null.
    ///  * `enabled_border`, displayed when `InputDecoration::enabled` is true
    ///    and `InputDecoration::error_text` is null.
    pub disabled_border: Option<InputBorderRef>,

    /// The border to display when the `InputDecorator` is enabled and is not
    /// showing an error.
    ///
    /// See also:
    ///
    ///  * `InputDecoration::enabled`, which is false if the `InputDecorator` is disabled.
    ///  * `InputDecoration::error_text`, the error shown by the `InputDecorator`, if non-null.
    ///  * `border`, for a description of where the `InputDecorator` border appears.
    ///  * `UnderlineInputBorder`, an `InputDecorator` border which draws a horizontal
    ///    line at the bottom of the input decorator's container.
    ///  * `OutlineInputBorder`, an `InputDecorator` border which draws a
    ///    rounded rectangle around the input decorator's container.
    ///  * `InputBorderRef::none()`, which doesn't draw a border.
    ///  * `error_border`, displayed when `InputDecorator::is_focused` is false
    ///    and `InputDecoration::error_text` is non-null.
    ///  * `focused_border`, displayed when `InputDecorator::is_focused` is true
    ///    and `InputDecoration::error_text` is null.
    ///  * `focused_error_border`, displayed when `InputDecorator::is_focused` is true
    ///    and `InputDecoration::error_text` is non-null.
    ///  * `disabled_border`, displayed when `InputDecoration::enabled` is false
    ///    and `InputDecoration::error_text` is null.
    pub enabled_border: Option<InputBorderRef>,

    /// The shape of the border to draw around the decoration's container.
    ///
    /// If `border` is a `WidgetStateInputBorder`, then the effective border can
    /// depend on the `WidgetState::Focused` state, i.e. if the `TextField` is
    /// focused or not.
    ///
    /// The decoration's container is the area which is filled if `filled` is
    /// true and bordered per the `border`. It's the area adjacent to
    /// `InputDecoration::icon` and above the widgets that contain
    /// `InputDecoration::helper_text`, `InputDecoration::error_text`, and
    /// `InputDecoration::counter_text`.
    ///
    /// The border's bounds, i.e. the value of `border.getOuterPath()`, define
    /// the area to be filled.
    ///
    /// This property is only used when the appropriate one of `error_border`,
    /// `focused_border`, `focused_error_border`, `disabled_border`, or `enabled_border`
    /// is not specified. This border's `InputBorder::border_side` property is
    /// configured by the InputDecorator, depending on the values of
    /// `InputDecoration::error_text`, `InputDecoration::enabled`,
    /// `InputDecorator::is_focused` and the current `Theme`.
    ///
    /// Typically one of `UnderlineInputBorder` or `OutlineInputBorder`.
    /// If null, InputDecorator's default is `const UnderlineInputBorder()`.
    ///
    /// See also:
    ///
    ///  * `InputBorderRef::none()`, which doesn't draw a border.
    ///  * `UnderlineInputBorder`, which draws a horizontal line at the
    ///    bottom of the input decorator's container.
    ///  * `OutlineInputBorder`, an `InputDecorator` border which draws a
    ///    rounded rectangle around the input decorator's container.
    pub border: Option<InputBorderRef>,

    /// Typically set to true when the `InputDecorator` contains a multiline
    /// `TextField` (`TextField::max_lines` is null or > 1) to override the default
    /// behavior of aligning the label with the center of the `TextField`.
    pub align_label_with_hint: bool,

    /// Defines minimum and maximum sizes for the `InputDecorator`.
    ///
    /// Typically the decorator will fill the horizontal space it is given. For
    /// larger screens, it may be useful to have the maximum width clamped to
    /// a given value so it doesn't fill the whole screen. This property
    /// allows you to control how big the decorator will be in its available
    /// space.
    ///
    /// If null, then the decorator will fill the available width with
    /// a default height based on text size.
    ///
    /// See also:
    ///
    ///  * `InputDecoration::constraints`, which can override this setting for a
    ///    given decorator.
    pub constraints: Option<BoxConstraints>,

    /// Defines how compact the decoration's layout will be.
    ///
    /// The vertical aspect of the default or user-specified `content_padding` is adjusted
    /// automatically based on `visual_density`.
    ///
    /// When the visual density is `VisualDensity::compact`, the vertical aspect of
    /// `content_padding` is reduced by 8 pixels.
    ///
    /// When the visual density is `VisualDensity::comfortable`, the vertical aspect of
    /// `content_padding` is reduced by 4 pixels.
    ///
    /// When the visual density is `VisualDensity::standard` vertical aspect of
    /// `content_padding` is not changed.
    ///
    /// If null, defaults to `ThemeData::visual_density`.
    ///
    /// See also:
    ///
    ///  * `ThemeData::visual_density`, which specifies the `visual_density` for all widgets
    ///    within a `Theme`.
    ///  * `InputDecoration::visual_density`, which can override this setting for a
    ///    given decorator.
    pub visual_density: Option<VisualDensity>,

    /// The padding around the supporting text row, which contains the helper
    /// text, the error text, and the counter.
    ///
    /// The start and end values set the horizontal position of the row, the
    /// same way `InputDecoration::content_padding` does for the input. The top
    /// value is the gap between the bottom of the decoration's container and
    /// the row, and the bottom value is the space below the row.
    ///
    /// If null, the row uses these defaults:
    ///
    ///  * start and end: the start and end values of
    ///    `InputDecoration::content_padding`.
    ///  * top: 4.0 for Material 3, 8.0 for Material 2.
    ///  * bottom: 0.0.
    ///
    /// If non-null, every side of this value is used as is, including sides
    /// that are zero. For example, `EdgeInsetsDirectional.only(start: 24.0)`
    /// sets the top gap to zero, so the supporting text sits directly against
    /// the container. To change the horizontal padding while keeping the
    /// default Material 3 gap, set the top value explicitly:
    ///
    /// ```text
    /// const InputDecoration(
    ///   helperText: 'Helper text',
    ///   supportingTextPadding: EdgeInsetsDirectional.fromSTEB(24.0, 4.0, 24.0, 0.0),
    /// )
    /// ```
    pub supporting_text_padding: Option<EdgeInsetsGeometry>,
}

crate::theme_data_getters::theme_data_getters! {
    /// Dart's getters of `InputDecorationThemeData`: the theme data answers with its fields, and a defaults
    /// subclass (Dart's `_...Defaults...(context)`) computes the ones it overrides when they
    /// are read.
    pub trait InputDecorationThemeDataGetters for InputDecorationThemeData, fields: input_decoration_theme_data {
        label_style: Option<AnyTextStyle>,
        floating_label_style: Option<AnyTextStyle>,
        helper_style: Option<AnyTextStyle>,
        helper_max_lines: Option<i32>,
        hint_style: Option<AnyTextStyle>,
        hint_fade_duration: Option<Duration>,
        hint_max_lines: Option<i32>,
        error_style: Option<AnyTextStyle>,
        error_max_lines: Option<i32>,
        floating_label_behavior: FloatingLabelBehavior,
        floating_label_alignment: FloatingLabelAlignment,
        is_dense: bool,
        content_padding: Option<EdgeInsetsGeometry>,
        is_collapsed: bool,
        icon_color: Option<AnyColor>,
        prefix_style: Option<AnyTextStyle>,
        prefix_icon_color: Option<AnyColor>,
        prefix_icon_constraints: Option<BoxConstraints>,
        suffix_style: Option<AnyTextStyle>,
        suffix_icon_color: Option<AnyColor>,
        suffix_icon_constraints: Option<BoxConstraints>,
        counter_style: Option<AnyTextStyle>,
        filled: bool,
        fill_color: Option<AnyColor>,
        outline_border: Option<AnyBorderSide>,
        active_indicator_border: Option<AnyBorderSide>,
        focus_color: Option<Color>,
        hover_color: Option<Color>,
        error_border: Option<InputBorderRef>,
        focused_border: Option<InputBorderRef>,
        focused_error_border: Option<InputBorderRef>,
        disabled_border: Option<InputBorderRef>,
        enabled_border: Option<InputBorderRef>,
        border: Option<InputBorderRef>,
        align_label_with_hint: bool,
        constraints: Option<BoxConstraints>,
        visual_density: Option<VisualDensity>,
        supporting_text_padding: Option<EdgeInsetsGeometry>,
    }
}

impl InputDecorationThemeData {
    /// Creates a `InputDecorationThemeData` that can be used to override default
    /// properties in a `InputDecorationTheme` widget.
    pub fn new() -> InputDecorationThemeData {
        InputDecorationThemeData {
            label_style: None,
            floating_label_style: None,
            helper_style: None,
            helper_max_lines: None,
            hint_style: None,
            hint_fade_duration: None,
            hint_max_lines: None,
            error_style: None,
            error_max_lines: None,
            floating_label_behavior: FloatingLabelBehavior::Auto,
            floating_label_alignment: FloatingLabelAlignment::START,
            is_dense: false,
            content_padding: None,
            is_collapsed: false,
            icon_color: None,
            prefix_style: None,
            prefix_icon_color: None,
            prefix_icon_constraints: None,
            suffix_style: None,
            suffix_icon_color: None,
            suffix_icon_constraints: None,
            counter_style: None,
            filled: false,
            fill_color: None,
            outline_border: None,
            active_indicator_border: None,
            focus_color: None,
            hover_color: None,
            error_border: None,
            focused_border: None,
            focused_error_border: None,
            disabled_border: None,
            enabled_border: None,
            border: None,
            align_label_with_hint: false,
            constraints: None,
            visual_density: None,
            supporting_text_padding: None,
        }
    }

    /// Dart `InputDecorationThemeData(labelStyle:)`.
    pub fn label_style(mut self, label_style: impl Into<AnyTextStyle>) -> InputDecorationThemeData {
        self.label_style = Some(label_style.into());
        self
    }

    /// Dart `InputDecorationThemeData(floatingLabelStyle:)`.
    pub fn floating_label_style(
        mut self,
        floating_label_style: impl Into<AnyTextStyle>,
    ) -> InputDecorationThemeData {
        self.floating_label_style = Some(floating_label_style.into());
        self
    }

    /// Dart `InputDecorationThemeData(helperStyle:)`.
    pub fn helper_style(
        mut self,
        helper_style: impl Into<AnyTextStyle>,
    ) -> InputDecorationThemeData {
        self.helper_style = Some(helper_style.into());
        self
    }

    /// Dart `InputDecorationThemeData(helperMaxLines:)`.
    pub fn helper_max_lines(mut self, helper_max_lines: i32) -> InputDecorationThemeData {
        self.helper_max_lines = Some(helper_max_lines);
        self
    }

    /// Dart `InputDecorationThemeData(hintStyle:)`.
    pub fn hint_style(mut self, hint_style: impl Into<AnyTextStyle>) -> InputDecorationThemeData {
        self.hint_style = Some(hint_style.into());
        self
    }

    /// Dart `InputDecorationThemeData(hintFadeDuration:)`.
    pub fn hint_fade_duration(mut self, hint_fade_duration: Duration) -> InputDecorationThemeData {
        self.hint_fade_duration = Some(hint_fade_duration);
        self
    }

    /// Dart `InputDecorationThemeData(hintMaxLines:)`.
    pub fn hint_max_lines(mut self, hint_max_lines: i32) -> InputDecorationThemeData {
        self.hint_max_lines = Some(hint_max_lines);
        self
    }

    /// Dart `InputDecorationThemeData(errorStyle:)`.
    pub fn error_style(mut self, error_style: impl Into<AnyTextStyle>) -> InputDecorationThemeData {
        self.error_style = Some(error_style.into());
        self
    }

    /// Dart `InputDecorationThemeData(errorMaxLines:)`.
    pub fn error_max_lines(mut self, error_max_lines: i32) -> InputDecorationThemeData {
        self.error_max_lines = Some(error_max_lines);
        self
    }

    /// Dart `InputDecorationThemeData(floatingLabelBehavior:)`.
    pub fn floating_label_behavior(
        mut self,
        floating_label_behavior: FloatingLabelBehavior,
    ) -> InputDecorationThemeData {
        self.floating_label_behavior = floating_label_behavior;
        self
    }

    /// Dart `InputDecorationThemeData(floatingLabelAlignment:)`.
    pub fn floating_label_alignment(
        mut self,
        floating_label_alignment: FloatingLabelAlignment,
    ) -> InputDecorationThemeData {
        self.floating_label_alignment = floating_label_alignment;
        self
    }

    /// Dart `InputDecorationThemeData(isDense:)`.
    pub fn is_dense(mut self, is_dense: bool) -> InputDecorationThemeData {
        self.is_dense = is_dense;
        self
    }

    /// Dart `InputDecorationThemeData(contentPadding:)`.
    pub fn content_padding(
        mut self,
        content_padding: EdgeInsetsGeometry,
    ) -> InputDecorationThemeData {
        self.content_padding = Some(content_padding);
        self
    }

    /// Dart `InputDecorationThemeData(isCollapsed:)`.
    pub fn is_collapsed(mut self, is_collapsed: bool) -> InputDecorationThemeData {
        self.is_collapsed = is_collapsed;
        self
    }

    /// Dart `InputDecorationThemeData(iconColor:)`.
    pub fn icon_color(mut self, icon_color: impl Into<AnyColor>) -> InputDecorationThemeData {
        self.icon_color = Some(icon_color.into());
        self
    }

    /// Dart `InputDecorationThemeData(prefixStyle:)`.
    pub fn prefix_style(
        mut self,
        prefix_style: impl Into<AnyTextStyle>,
    ) -> InputDecorationThemeData {
        self.prefix_style = Some(prefix_style.into());
        self
    }

    /// Dart `InputDecorationThemeData(prefixIconColor:)`.
    pub fn prefix_icon_color(
        mut self,
        prefix_icon_color: impl Into<AnyColor>,
    ) -> InputDecorationThemeData {
        self.prefix_icon_color = Some(prefix_icon_color.into());
        self
    }

    /// Dart `InputDecorationThemeData(prefixIconConstraints:)`.
    pub fn prefix_icon_constraints(
        mut self,
        prefix_icon_constraints: BoxConstraints,
    ) -> InputDecorationThemeData {
        self.prefix_icon_constraints = Some(prefix_icon_constraints);
        self
    }

    /// Dart `InputDecorationThemeData(suffixStyle:)`.
    pub fn suffix_style(
        mut self,
        suffix_style: impl Into<AnyTextStyle>,
    ) -> InputDecorationThemeData {
        self.suffix_style = Some(suffix_style.into());
        self
    }

    /// Dart `InputDecorationThemeData(suffixIconColor:)`.
    pub fn suffix_icon_color(
        mut self,
        suffix_icon_color: impl Into<AnyColor>,
    ) -> InputDecorationThemeData {
        self.suffix_icon_color = Some(suffix_icon_color.into());
        self
    }

    /// Dart `InputDecorationThemeData(suffixIconConstraints:)`.
    pub fn suffix_icon_constraints(
        mut self,
        suffix_icon_constraints: BoxConstraints,
    ) -> InputDecorationThemeData {
        self.suffix_icon_constraints = Some(suffix_icon_constraints);
        self
    }

    /// Dart `InputDecorationThemeData(counterStyle:)`.
    pub fn counter_style(
        mut self,
        counter_style: impl Into<AnyTextStyle>,
    ) -> InputDecorationThemeData {
        self.counter_style = Some(counter_style.into());
        self
    }

    /// Dart `InputDecorationThemeData(filled:)`.
    pub fn filled(mut self, filled: bool) -> InputDecorationThemeData {
        self.filled = filled;
        self
    }

    /// Dart `InputDecorationThemeData(fillColor:)`.
    pub fn fill_color(mut self, fill_color: impl Into<AnyColor>) -> InputDecorationThemeData {
        self.fill_color = Some(fill_color.into());
        self
    }

    /// Dart `InputDecorationThemeData(activeIndicatorBorder:)`.
    pub fn active_indicator_border(
        mut self,
        active_indicator_border: impl Into<AnyBorderSide>,
    ) -> InputDecorationThemeData {
        self.active_indicator_border = Some(active_indicator_border.into());
        self
    }

    /// Dart `InputDecorationThemeData(outlineBorder:)`.
    pub fn outline_border(
        mut self,
        outline_border: impl Into<AnyBorderSide>,
    ) -> InputDecorationThemeData {
        self.outline_border = Some(outline_border.into());
        self
    }

    /// Dart `InputDecorationThemeData(focusColor:)`.
    pub fn focus_color(mut self, focus_color: Color) -> InputDecorationThemeData {
        self.focus_color = Some(focus_color);
        self
    }

    /// Dart `InputDecorationThemeData(hoverColor:)`.
    pub fn hover_color(mut self, hover_color: Color) -> InputDecorationThemeData {
        self.hover_color = Some(hover_color);
        self
    }

    /// Dart `InputDecorationThemeData(errorBorder:)`.
    pub fn error_border(
        mut self,
        error_border: impl Into<InputBorderRef>,
    ) -> InputDecorationThemeData {
        self.error_border = Some(error_border.into());
        self
    }

    /// Dart `InputDecorationThemeData(focusedBorder:)`.
    pub fn focused_border(
        mut self,
        focused_border: impl Into<InputBorderRef>,
    ) -> InputDecorationThemeData {
        self.focused_border = Some(focused_border.into());
        self
    }

    /// Dart `InputDecorationThemeData(focusedErrorBorder:)`.
    pub fn focused_error_border(
        mut self,
        focused_error_border: impl Into<InputBorderRef>,
    ) -> InputDecorationThemeData {
        self.focused_error_border = Some(focused_error_border.into());
        self
    }

    /// Dart `InputDecorationThemeData(disabledBorder:)`.
    pub fn disabled_border(
        mut self,
        disabled_border: impl Into<InputBorderRef>,
    ) -> InputDecorationThemeData {
        self.disabled_border = Some(disabled_border.into());
        self
    }

    /// Dart `InputDecorationThemeData(enabledBorder:)`.
    pub fn enabled_border(
        mut self,
        enabled_border: impl Into<InputBorderRef>,
    ) -> InputDecorationThemeData {
        self.enabled_border = Some(enabled_border.into());
        self
    }

    /// Dart `InputDecorationThemeData(border:)`.
    pub fn border(mut self, border: impl Into<InputBorderRef>) -> InputDecorationThemeData {
        self.border = Some(border.into());
        self
    }

    /// Dart `InputDecorationThemeData(alignLabelWithHint:)`.
    pub fn align_label_with_hint(
        mut self,
        align_label_with_hint: bool,
    ) -> InputDecorationThemeData {
        self.align_label_with_hint = align_label_with_hint;
        self
    }

    /// Dart `InputDecorationThemeData(constraints:)`.
    pub fn constraints(mut self, constraints: BoxConstraints) -> InputDecorationThemeData {
        self.constraints = Some(constraints);
        self
    }

    /// Dart `InputDecorationThemeData(visualDensity:)`.
    pub fn visual_density(mut self, visual_density: VisualDensity) -> InputDecorationThemeData {
        self.visual_density = Some(visual_density);
        self
    }

    /// Dart `InputDecorationThemeData(supportingTextPadding:)`.
    pub fn supporting_text_padding(
        mut self,
        supporting_text_padding: EdgeInsetsGeometry,
    ) -> InputDecorationThemeData {
        self.supporting_text_padding = Some(supporting_text_padding);
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`theme.copy_with().fill_color(color)`).
    pub fn copy_with(&self) -> InputDecorationThemeData {
        self.clone()
    }

    /// Returns a copy of this InputDecorationThemeData where the non-null fields in
    /// the given InputDecorationThemeData override the corresponding nullable fields
    /// in this InputDecorationThemeData.
    ///
    /// The non-nullable fields of InputDecorationThemeData, such as
    /// `floating_label_behavior`, `is_dense`, `is_collapsed`, `filled`, and
    /// `align_label_with_hint` cannot be overridden.
    ///
    /// In other words, the fields of the provided `InputDecorationThemeData`
    /// are used to fill in the unspecified and nullable fields of this
    /// InputDecorationThemeData.
    pub fn merge(&self, other: Option<&InputDecorationThemeData>) -> InputDecorationThemeData {
        let Some(other) = other else {
            return self.clone();
        };
        let mut merged = self.copy_with();
        merged.label_style = self
            .label_style
            .clone()
            .or_else(|| other.label_style.clone());
        merged.floating_label_style = self
            .floating_label_style
            .clone()
            .or_else(|| other.floating_label_style.clone());
        merged.helper_style = self
            .helper_style
            .clone()
            .or_else(|| other.helper_style.clone());
        merged.helper_max_lines = self.helper_max_lines.or(other.helper_max_lines);
        merged.hint_style = self.hint_style.clone().or_else(|| other.hint_style.clone());
        merged.hint_fade_duration = self.hint_fade_duration.or(other.hint_fade_duration);
        merged.hint_max_lines = self.hint_max_lines.or(other.hint_max_lines);
        merged.error_style = self
            .error_style
            .clone()
            .or_else(|| other.error_style.clone());
        merged.error_max_lines = self.error_max_lines.or(other.error_max_lines);
        merged.content_padding = self.content_padding.or(other.content_padding);
        merged.icon_color = self.icon_color.clone().or_else(|| other.icon_color.clone());
        merged.prefix_style = self
            .prefix_style
            .clone()
            .or_else(|| other.prefix_style.clone());
        merged.prefix_icon_color = self
            .prefix_icon_color
            .clone()
            .or_else(|| other.prefix_icon_color.clone());
        merged.prefix_icon_constraints = self
            .prefix_icon_constraints
            .or(other.prefix_icon_constraints);
        merged.suffix_style = self
            .suffix_style
            .clone()
            .or_else(|| other.suffix_style.clone());
        merged.suffix_icon_color = self
            .suffix_icon_color
            .clone()
            .or_else(|| other.suffix_icon_color.clone());
        merged.suffix_icon_constraints = self
            .suffix_icon_constraints
            .or(other.suffix_icon_constraints);
        merged.counter_style = self
            .counter_style
            .clone()
            .or_else(|| other.counter_style.clone());
        merged.fill_color = self.fill_color.clone().or_else(|| other.fill_color.clone());
        merged.active_indicator_border = self
            .active_indicator_border
            .clone()
            .or_else(|| other.active_indicator_border.clone());
        merged.outline_border = self
            .outline_border
            .clone()
            .or_else(|| other.outline_border.clone());
        merged.focus_color = self.focus_color.or(other.focus_color);
        merged.hover_color = self.hover_color.or(other.hover_color);
        merged.error_border = self
            .error_border
            .clone()
            .or_else(|| other.error_border.clone());
        merged.focused_border = self
            .focused_border
            .clone()
            .or_else(|| other.focused_border.clone());
        merged.focused_error_border = self
            .focused_error_border
            .clone()
            .or_else(|| other.focused_error_border.clone());
        merged.disabled_border = self
            .disabled_border
            .clone()
            .or_else(|| other.disabled_border.clone());
        merged.enabled_border = self
            .enabled_border
            .clone()
            .or_else(|| other.enabled_border.clone());
        merged.border = self.border.clone().or_else(|| other.border.clone());
        merged.constraints = self.constraints.or(other.constraints);
        merged.visual_density = self.visual_density.or(other.visual_density);
        merged.supporting_text_padding = self
            .supporting_text_padding
            .or(other.supporting_text_padding);
        merged
    }
}

impl Default for InputDecorationThemeData {
    fn default() -> InputDecorationThemeData {
        InputDecorationThemeData::new()
    }
}

/// Dart's `toString` through `debugFillProperties`: the properties that differ from
/// `InputDecorationThemeData()`'s.
impl Debug for InputDecorationThemeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let default_theme = InputDecorationThemeData::new();
        let mut properties: Vec<String> = Vec::new();
        if let Some(label_style) = &self.label_style {
            properties.push(format!("labelStyle: {:?}", label_style));
        }
        if let Some(floating_label_style) = &self.floating_label_style {
            properties.push(format!("floatingLabelStyle: {:?}", floating_label_style));
        }
        if let Some(helper_style) = &self.helper_style {
            properties.push(format!("helperStyle: {:?}", helper_style));
        }
        if let Some(helper_max_lines) = &self.helper_max_lines {
            properties.push(format!("helperMaxLines: {:?}", helper_max_lines));
        }
        if let Some(hint_style) = &self.hint_style {
            properties.push(format!("hintStyle: {:?}", hint_style));
        }
        if let Some(hint_fade_duration) = &self.hint_fade_duration {
            properties.push(format!("hintFadeDuration: {:?}", hint_fade_duration));
        }
        if let Some(hint_max_lines) = &self.hint_max_lines {
            properties.push(format!("hintMaxLines: {:?}", hint_max_lines));
        }
        if let Some(error_style) = &self.error_style {
            properties.push(format!("errorStyle: {:?}", error_style));
        }
        if let Some(error_max_lines) = &self.error_max_lines {
            properties.push(format!("errorMaxLines: {:?}", error_max_lines));
        }
        if self.floating_label_behavior != default_theme.floating_label_behavior {
            properties.push(format!(
                "floatingLabelBehavior: {:?}",
                self.floating_label_behavior
            ));
        }
        if self.floating_label_alignment != default_theme.floating_label_alignment {
            properties.push(format!(
                "floatingLabelAlignment: {:?}",
                self.floating_label_alignment
            ));
        }
        if self.is_dense != default_theme.is_dense {
            properties.push(format!("isDense: {:?}", self.is_dense));
        }
        if let Some(content_padding) = &self.content_padding {
            properties.push(format!("contentPadding: {:?}", content_padding));
        }
        if let Some(supporting_text_padding) = &self.supporting_text_padding {
            properties.push(format!(
                "supportingTextPadding: {:?}",
                supporting_text_padding
            ));
        }
        if self.is_collapsed != default_theme.is_collapsed {
            properties.push(format!("isCollapsed: {:?}", self.is_collapsed));
        }
        if let Some(icon_color) = &self.icon_color {
            properties.push(format!("iconColor: {:?}", icon_color));
        }
        if let Some(prefix_icon_color) = &self.prefix_icon_color {
            properties.push(format!("prefixIconColor: {:?}", prefix_icon_color));
        }
        if let Some(prefix_icon_constraints) = &self.prefix_icon_constraints {
            properties.push(format!(
                "prefixIconConstraints: {:?}",
                prefix_icon_constraints
            ));
        }
        if let Some(prefix_style) = &self.prefix_style {
            properties.push(format!("prefixStyle: {:?}", prefix_style));
        }
        if let Some(suffix_icon_color) = &self.suffix_icon_color {
            properties.push(format!("suffixIconColor: {:?}", suffix_icon_color));
        }
        if let Some(suffix_icon_constraints) = &self.suffix_icon_constraints {
            properties.push(format!(
                "suffixIconConstraints: {:?}",
                suffix_icon_constraints
            ));
        }
        if let Some(suffix_style) = &self.suffix_style {
            properties.push(format!("suffixStyle: {:?}", suffix_style));
        }
        if let Some(counter_style) = &self.counter_style {
            properties.push(format!("counterStyle: {:?}", counter_style));
        }
        if self.filled != default_theme.filled {
            properties.push(format!("filled: {:?}", self.filled));
        }
        if let Some(fill_color) = &self.fill_color {
            properties.push(format!("fillColor: {:?}", fill_color));
        }
        if let Some(active_indicator_border) = &self.active_indicator_border {
            properties.push(format!(
                "activeIndicatorBorder: {:?}",
                active_indicator_border
            ));
        }
        if let Some(outline_border) = &self.outline_border {
            properties.push(format!("outlineBorder: {:?}", outline_border));
        }
        if let Some(focus_color) = &self.focus_color {
            properties.push(format!("focusColor: {:?}", focus_color));
        }
        if let Some(hover_color) = &self.hover_color {
            properties.push(format!("hoverColor: {:?}", hover_color));
        }
        if let Some(error_border) = &self.error_border {
            properties.push(format!("errorBorder: {:?}", error_border));
        }
        if let Some(focused_border) = &self.focused_border {
            properties.push(format!("focusedBorder: {:?}", focused_border));
        }
        if let Some(focused_error_border) = &self.focused_error_border {
            properties.push(format!("focusedErrorBorder: {:?}", focused_error_border));
        }
        if let Some(disabled_border) = &self.disabled_border {
            properties.push(format!("disabledBorder: {:?}", disabled_border));
        }
        if let Some(enabled_border) = &self.enabled_border {
            properties.push(format!("enabledBorder: {:?}", enabled_border));
        }
        if let Some(border) = &self.border {
            properties.push(format!("border: {:?}", border));
        }
        if self.align_label_with_hint != default_theme.align_label_with_hint {
            properties.push(format!(
                "alignLabelWithHint: {:?}",
                self.align_label_with_hint
            ));
        }
        if let Some(constraints) = &self.constraints {
            properties.push(format!("constraints: {:?}", constraints));
        }
        if let Some(visual_density) = &self.visual_density {
            properties.push(format!("visualDensity: {:?}", visual_density));
        }
        write!(f, "InputDecorationThemeData({})", properties.join(", "))
    }
}

/// Dart's `_InputDecoratorDefaultsM2`.
struct InputDecoratorDefaultsM2 {
    /// Dart's `super(..)` arguments.
    input_decoration_theme_data: InputDecorationThemeData,
    context: DefaultsContext,
}

impl InputDecoratorDefaultsM2 {
    fn new(context: BuildContext) -> InputDecoratorDefaultsM2 {
        InputDecoratorDefaultsM2 {
            input_decoration_theme_data: InputDecorationThemeData::new(),
            context: DefaultsContext::new(context),
        }
    }
}

impl InputDecorationThemeDataGetters for InputDecoratorDefaultsM2 {
    fn input_decoration_theme_data(&self) -> &InputDecorationThemeData {
        &self.input_decoration_theme_data
    }

    fn hint_style(&self, app: &mut App) -> Option<AnyTextStyle> {
        let theme = self.context.theme(app);
        let disabled_color = theme.disabled_color;
        let hint_color = theme.hint_color;
        let hint_style = WidgetStateTextStyle::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Disabled) {
                return TextStyle::new().color(disabled_color);
            }
            TextStyle::new().color(hint_color)
        });
        Some(hint_style.into())
    }

    fn label_style(&self, app: &mut App) -> Option<AnyTextStyle> {
        let theme = self.context.theme(app);
        let disabled_color = theme.disabled_color;
        let hint_color = theme.hint_color;
        let label_style = WidgetStateTextStyle::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Disabled) {
                return TextStyle::new().color(disabled_color);
            }
            TextStyle::new().color(hint_color)
        });
        Some(label_style.into())
    }

    fn floating_label_style(&self, app: &mut App) -> Option<AnyTextStyle> {
        let theme = self.context.theme(app);
        let disabled_color = theme.disabled_color;
        let hint_color = theme.hint_color;
        let error = theme.color_scheme.error;
        let primary = theme.color_scheme.primary;
        let floating_label_style =
            WidgetStateTextStyle::resolve_with(move |states: &WidgetStates| {
                if states.contains(&WidgetState::Disabled) {
                    return TextStyle::new().color(disabled_color);
                }
                if states.contains(&WidgetState::Error) {
                    return TextStyle::new().color(error);
                }
                if states.contains(&WidgetState::Focused) {
                    return TextStyle::new().color(primary);
                }
                TextStyle::new().color(hint_color)
            });
        Some(floating_label_style.into())
    }

    fn helper_style(&self, app: &mut App) -> Option<AnyTextStyle> {
        let theme = self.context.theme(app);
        let hint_color = theme.hint_color;
        let body_small = theme.text_theme.body_small.clone();
        let helper_body_small = body_small.clone();
        let helper_style = WidgetStateTextStyle::resolve_with(move |states: &WidgetStates| {
            let body_small = helper_body_small
                .clone()
                .expect("ThemeData::text_theme has a body_small");
            if states.contains(&WidgetState::Disabled) {
                return body_small.copy_with().color(Colors::TRANSPARENT);
            }

            body_small.copy_with().color(hint_color)
        });
        Some(helper_style.into())
    }

    fn error_style(&self, app: &mut App) -> Option<AnyTextStyle> {
        let theme = self.context.theme(app);
        let error = theme.color_scheme.error;
        let body_small = theme.text_theme.body_small.clone();
        let error_style = WidgetStateTextStyle::resolve_with(move |states: &WidgetStates| {
            let body_small = body_small
                .clone()
                .expect("ThemeData::text_theme has a body_small");
            if states.contains(&WidgetState::Disabled) {
                return body_small.copy_with().color(Colors::TRANSPARENT);
            }
            body_small.copy_with().color(error)
        });
        Some(error_style.into())
    }

    fn fill_color(&self, app: &mut App) -> Option<AnyColor> {
        let brightness = Theme::brightness_of(app, self.context.context());
        let fill_color = WidgetStateColor::resolve_with(move |states: &WidgetStates| {
            match (brightness, states.contains(&WidgetState::Disabled)) {
                (Brightness::Dark, true) => Color::new(0x0DFFFFFF), //  5% white
                (Brightness::Dark, false) => Color::new(0x1AFFFFFF), // 10% white
                (Brightness::Light, true) => Color::new(0x05000000), //  2% black
                (Brightness::Light, false) => Color::new(0x0A000000), //  4% black
            }
        });
        Some(fill_color.into())
    }

    fn icon_color(&self, app: &mut App) -> Option<AnyColor> {
        let theme = self.context.theme(app);
        let brightness = Theme::brightness_of(app, self.context.context());
        let disabled_color = theme.disabled_color;
        let primary = theme.color_scheme.primary;
        let icon_color = WidgetStateColor::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Disabled) && !states.contains(&WidgetState::Focused) {
                return disabled_color;
            }
            if states.contains(&WidgetState::Focused) {
                return primary;
            }
            match brightness {
                Brightness::Dark => Colors::WHITE70,
                Brightness::Light => Colors::BLACK45,
            }
        });
        Some(icon_color.into())
    }

    fn prefix_icon_color(&self, app: &mut App) -> Option<AnyColor> {
        let theme = self.context.theme(app);
        let brightness = Theme::brightness_of(app, self.context.context());
        let disabled_color = theme.disabled_color;
        let primary = theme.color_scheme.primary;
        let prefix_icon_color = WidgetStateColor::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Disabled) && !states.contains(&WidgetState::Focused) {
                return disabled_color;
            }
            if states.contains(&WidgetState::Focused) {
                return primary;
            }
            match brightness {
                Brightness::Dark => Colors::WHITE70,
                Brightness::Light => Colors::BLACK45,
            }
        });
        Some(prefix_icon_color.into())
    }

    fn suffix_icon_color(&self, app: &mut App) -> Option<AnyColor> {
        let theme = self.context.theme(app);
        let brightness = Theme::brightness_of(app, self.context.context());
        let disabled_color = theme.disabled_color;
        let error = theme.color_scheme.error;
        let primary = theme.color_scheme.primary;
        let suffix_icon_color = WidgetStateColor::resolve_with(move |states: &WidgetStates| {
            if states.contains(&WidgetState::Disabled) && !states.contains(&WidgetState::Focused) {
                return disabled_color;
            }
            if states.contains(&WidgetState::Error) {
                return error;
            }
            if states.contains(&WidgetState::Focused) {
                return primary;
            }
            match brightness {
                Brightness::Dark => Colors::WHITE70,
                Brightness::Light => Colors::BLACK45,
            }
        });
        Some(suffix_icon_color.into())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    pub(crate) mod common {
        // The helpers of input_decorator_test.dart.
        //
        use waft_widgets::testing::WidgetTester;

        use std::time::Duration;

        use waft_embedder::{Color, Offset, Rect};
        use waft_painting::{
            Alignment, AlignmentGeometry, BorderRadius, BorderSide, TextAlignVertical,
            TextDirection, TextStyle,
        };
        use waft_rendering::AnyRenderBox;
        use waft_widgets::{
            Align, AnyElement, Builder, CustomPaint, Directionality, IntoWidget, IntrinsicWidth,
            MatrixTransition, WidgetRef, downcast_widget,
        };

        use super::super::*;
        use crate::app::MaterialApp;
        use crate::icon_button_theme::IconButtonThemeData;
        use crate::input_border::{InputBorderRef, OutlineInputBorder, UnderlineInputBorder};
        use crate::material::Material;
        use crate::theme::Theme;
        use crate::theme_data::{ThemeData, VisualDensity};

        pub(super) const K_TRANSITION_DURATION: Duration = Duration::from_millis(167);

        pub(super) const HINT_TEXT: &str = "hint";
        pub(super) const INPUT_TEXT: &str = "text";
        pub(super) const LABEL_TEXT: &str = "label";
        pub(super) const ERROR_TEXT: &str = "error";
        pub(super) const HELPER_TEXT: &str = "helper";
        pub(super) const COUNTER_TEXT: &str = "counter";

        pub(super) const TWO_LINES: &str = "line1\nline2";
        pub(super) const THREE_LINES: &str = "line1\nline2\nline3";

        /// Dart's `ThemeData()`.
        pub(super) fn m3(app: &App) -> ThemeData {
            ThemeData::builder().build(app)
        }

        /// Dart's `ThemeData(useMaterial3: false)`.
        pub(super) fn m2(app: &App) -> ThemeData {
            ThemeData::builder().use_material3(false).build(app)
        }

        /// Dart's input child: `Text(inputText, style: TextStyle(fontSize: 16.0, fontWeight:
        /// FontWeight.w400, letterSpacing: 0.5, height: 1.50))`, a text style compliant with M3
        /// specification (which is bodyLarge for text fields).
        pub(super) fn input_text() -> WidgetRef {
            Text::new(INPUT_TEXT)
                .style(
                    TextStyle::new()
                        .font_size(16.0)
                        .font_weight(waft_painting::FontWeight::W400)
                        .letter_spacing(0.5)
                        .height(1.50),
                )
                .into_widget()
        }

        /// Dart's `buildInputDecorator`, its named arguments the setters.
        pub(super) struct BuildInputDecorator {
            decoration: InputDecoration,
            theme: Option<fn(&App) -> ThemeData>,
            input_decoration_theme: Option<InputDecorationThemeData>,
            local_input_decoration_theme: Option<InputDecorationThemeData>,
            icon_button_theme: Option<IconButtonThemeData>,
            text_direction: TextDirection,
            expands: bool,
            is_empty: bool,
            is_focused: bool,
            is_hovering: bool,
            use_intrinsic_width: bool,
            base_style: Option<TextStyle>,
            text_align_vertical: Option<TextAlignVertical>,
            visual_density: Option<VisualDensity>,
            child: Option<WidgetRef>,
        }

        // Every argument of Dart's helper is a setter, named after it, whether or not a ported
        // case passes it.
        #[allow(dead_code, clippy::wrong_self_convention)]
        impl BuildInputDecorator {
            pub(super) fn new(decoration: InputDecoration) -> BuildInputDecorator {
                BuildInputDecorator {
                    decoration,
                    theme: None,
                    input_decoration_theme: None,
                    local_input_decoration_theme: None,
                    icon_button_theme: None,
                    text_direction: TextDirection::Ltr,
                    expands: false,
                    is_empty: false,
                    is_focused: false,
                    is_hovering: false,
                    use_intrinsic_width: false,
                    base_style: None,
                    text_align_vertical: None,
                    visual_density: None,
                    child: None,
                }
            }

            pub(super) fn theme(mut self, theme: fn(&App) -> ThemeData) -> BuildInputDecorator {
                self.theme = Some(theme);
                self
            }

            pub(super) fn input_decoration_theme(
                mut self,
                theme: InputDecorationThemeData,
            ) -> BuildInputDecorator {
                self.input_decoration_theme = Some(theme);
                self
            }

            pub(super) fn local_input_decoration_theme(
                mut self,
                theme: InputDecorationThemeData,
            ) -> BuildInputDecorator {
                self.local_input_decoration_theme = Some(theme);
                self
            }

            pub(super) fn icon_button_theme(
                mut self,
                theme: IconButtonThemeData,
            ) -> BuildInputDecorator {
                self.icon_button_theme = Some(theme);
                self
            }

            pub(super) fn text_direction(
                mut self,
                text_direction: TextDirection,
            ) -> BuildInputDecorator {
                self.text_direction = text_direction;
                self
            }

            pub(super) fn expands(mut self, expands: bool) -> BuildInputDecorator {
                self.expands = expands;
                self
            }

            pub(super) fn is_empty(mut self, is_empty: bool) -> BuildInputDecorator {
                self.is_empty = is_empty;
                self
            }

            pub(super) fn is_focused(mut self, is_focused: bool) -> BuildInputDecorator {
                self.is_focused = is_focused;
                self
            }

            pub(super) fn is_hovering(mut self, is_hovering: bool) -> BuildInputDecorator {
                self.is_hovering = is_hovering;
                self
            }

            pub(super) fn use_intrinsic_width(
                mut self,
                use_intrinsic_width: bool,
            ) -> BuildInputDecorator {
                self.use_intrinsic_width = use_intrinsic_width;
                self
            }

            pub(super) fn base_style(mut self, base_style: TextStyle) -> BuildInputDecorator {
                self.base_style = Some(base_style);
                self
            }

            pub(super) fn text_align_vertical(
                mut self,
                text_align_vertical: TextAlignVertical,
            ) -> BuildInputDecorator {
                self.text_align_vertical = Some(text_align_vertical);
                self
            }

            pub(super) fn visual_density(
                mut self,
                visual_density: VisualDensity,
            ) -> BuildInputDecorator {
                self.visual_density = Some(visual_density);
                self
            }

            pub(super) fn child<K>(mut self, child: impl IntoWidget<K>) -> BuildInputDecorator {
                self.child = Some(child.into_widget());
                self
            }

            pub(super) fn build(self) -> WidgetRef {
                let mut decorator = InputDecorator::new(self.decoration)
                    .expands(self.expands)
                    .is_empty(self.is_empty)
                    .is_focused(self.is_focused)
                    .is_hovering(self.is_hovering)
                    .child(self.child.unwrap_or_else(input_text));
                if let Some(base_style) = self.base_style {
                    decorator = decorator.base_style(base_style);
                }
                if let Some(text_align_vertical) = self.text_align_vertical {
                    decorator = decorator.text_align_vertical(text_align_vertical);
                }
                let mut widget = decorator.into_widget();

                if self.use_intrinsic_width {
                    widget = IntrinsicWidth::new().child(widget).into_widget();
                }

                if let Some(local) = self.local_input_decoration_theme {
                    widget = InputDecorationTheme::new()
                        .with_data(local)
                        .child(widget)
                        .into_widget();
                }

                let theme = self.theme;
                let input_decoration_theme = self.input_decoration_theme;
                let icon_button_theme = self.icon_button_theme;
                let visual_density = self.visual_density;
                let text_direction = self.text_direction;
                MaterialApp::new()
                    .home(Material::new().child(Builder::new(move |app, context| {
                        let base = match theme {
                            Some(theme) => Rc::new(theme(app)),
                            None => Theme::of(app, context),
                        };
                        let mut copy = base.copy_with();
                        if let Some(input_decoration_theme) = &input_decoration_theme {
                            copy = copy.input_decoration_theme(input_decoration_theme.clone());
                        }
                        if let Some(icon_button_theme) = &icon_button_theme {
                            copy = copy.icon_button_theme(icon_button_theme.clone());
                        }
                        if let Some(visual_density) = visual_density {
                            copy = copy.visual_density(visual_density);
                        }
                        Theme::new(
                            copy.build(),
                            Align::new()
                                .alignment(AlignmentGeometry::Alignment(Alignment::TOP_LEFT))
                                .child(Directionality::new(text_direction, widget.clone())),
                        )
                        .into_widget()
                    })))
                    .into_widget()
            }
        }

        /// Dart's `tester.getRect(find.byWidget(..))` of an element.
        pub(super) fn rect_of(tester: &WidgetTester, element: AnyElement) -> Rect {
            tester.get_rect(tester.render_box_of(element))
        }

        /// Dart's `tester.getRect(find.text(text))`.
        pub(super) fn text_rect(tester: &WidgetTester, text: &str) -> Rect {
            let found = tester.find_text(text);
            assert_eq!(found.len(), 1, "one {text:?}");
            rect_of(tester, found[0])
        }

        /// Dart's `tester.getRect(find.byType(InputDecorator))`.
        pub(super) fn get_decorator_rect(tester: &WidgetTester) -> Rect {
            rect_of(tester, tester.find_by_type::<InputDecorator>()[0])
        }

        /// Dart's `findBorderPainter()`: the `CustomPaint` the `_BorderContainer` builds, if any.
        pub(super) fn find_border_painter(tester: &WidgetTester) -> Option<AnyElement> {
            let container = tester
                .find_by_type::<BorderContainer>()
                .into_iter()
                .next()?;
            tester
                .descendants_of_type::<CustomPaint>(container)
                .into_iter()
                .next()
        }

        /// Reads the `_InputBorderPainter` of the border painter.
        pub(super) fn with_border_painter<R>(
            tester: &WidgetTester,
            read: impl FnOnce(&App, &InputBorderPainter) -> R,
        ) -> R {
            let element = find_border_painter(tester).expect("a border painter");
            let app = tester.cell.borrow();
            let custom_paint =
                downcast_widget::<CustomPaint>(&**element.widget(&app)).expect("a CustomPaint");
            let painter = custom_paint
                .foreground_painter
                .as_ref()
                .expect("a foreground painter");
            let painter = painter
                .as_any()
                .downcast_ref::<InputBorderPainter>()
                .expect("an _InputBorderPainter");
            read(&app, painter)
        }

        /// Dart's `getBorder(tester)`: the border the painter paints now.
        pub(super) fn get_border(tester: &WidgetTester) -> Option<InputBorderRef> {
            find_border_painter(tester)?;
            Some(with_border_painter(tester, |app, painter| {
                painter
                    .border
                    .evaluate(app, painter.border_animation.handle())
            }))
        }

        /// Dart's `getBorderSide(tester)`.
        pub(super) fn get_border_side(tester: &WidgetTester) -> BorderSide {
            get_border(tester).expect("a border").border_side()
        }

        /// Dart's `getBorderRadius(tester)`.
        pub(super) fn get_border_radius(tester: &WidgetTester) -> Option<BorderRadius> {
            let border = get_border(tester).expect("a border");
            if let Some(underline) = border.as_any().downcast_ref::<UnderlineInputBorder>() {
                return Some(underline.border_radius);
            }
            border
                .as_any()
                .downcast_ref::<OutlineInputBorder>()
                .map(|outline| outline.border_radius)
        }

        /// Dart's `getBorderWeight(tester)`.
        pub(super) fn get_border_weight(tester: &WidgetTester) -> f64 {
            get_border_side(tester).width
        }

        /// Dart's `getBorderColor(tester)`.
        pub(super) fn get_border_color(tester: &WidgetTester) -> Color {
            get_border_side(tester).color
        }

        /// The border each `_BorderContainer`'s `_InputBorderPainter` paints now and its
        /// `blendedColor`, in tree order: what Dart's tests read through the painter.
        pub(crate) fn borders_and_container_colors(
            tester: &WidgetTester,
        ) -> Vec<(InputBorderRef, Color)> {
            tester
                .find_by_type::<BorderContainer>()
                .into_iter()
                .filter_map(|container| {
                    tester
                        .descendants_of_type::<CustomPaint>(container)
                        .into_iter()
                        .next()
                })
                .map(|element| {
                    let app = tester.cell.borrow();
                    let custom_paint = downcast_widget::<CustomPaint>(&**element.widget(&app))
                        .expect("a CustomPaint");
                    let painter = custom_paint
                        .foreground_painter
                        .as_ref()
                        .and_then(|painter| painter.as_any().downcast_ref::<InputBorderPainter>())
                        .expect("an _InputBorderPainter");
                    (
                        painter
                            .border
                            .evaluate(&app, painter.border_animation.handle()),
                        painter.blended_color(&app),
                    )
                })
                .collect()
        }

        /// Dart's `getContainerColor(tester)`: the painter's `blendedColor`.
        pub(super) fn get_container_color(tester: &WidgetTester) -> Color {
            with_border_painter(tester, |app, painter| painter.blended_color(app))
        }

        /// Dart's `getBorderBottom(tester)` and `getContainerRect(tester).height`: the height of
        /// `InputDecorator.containerOf` the border painter.
        pub(super) fn get_border_bottom(tester: &WidgetTester) -> f64 {
            get_container_rect(tester).height()
        }

        /// Dart's `getContainerRect(tester)`: the container's `paintBounds`.
        pub(super) fn get_container_rect(tester: &WidgetTester) -> Rect {
            let element = find_border_painter(tester).expect("a border painter");
            let app = tester.cell.borrow();
            let container: AnyRenderBox =
                InputDecorator::container_of(&app, element).expect("a decorator container");
            Offset::ZERO & container.size(&app)
        }

        /// Dart's `findLabel()`: the label's text, a descendant of a `MatrixTransition` (the
        /// decorator's; the page route's transitions have their own).
        pub(super) fn find_label(tester: &WidgetTester) -> AnyElement {
            for transition in tester.find_by_type::<waft_widgets::Animated<MatrixTransition>>() {
                if let Some(text) = tester
                    .descendants_of_type::<waft_widgets::Text>(transition)
                    .into_iter()
                    .next()
                {
                    return text;
                }
            }
            panic!("no label")
        }

        /// Dart's `getLabelRect(tester)`.
        pub(super) fn get_label_rect(tester: &WidgetTester) -> Rect {
            rect_of(tester, find_label(tester))
        }

        /// Dart's `getLabelStyle(tester)`: the style of the `AnimatedDefaultTextStyle` above the label.
        pub(super) fn get_label_style(tester: &WidgetTester) -> TextStyle {
            let label = find_label(tester);
            let ancestor = tester
                .ancestors(label)
                .into_iter()
                .find(|element| {
                    let app = tester.cell.borrow();
                    downcast_widget::<waft_widgets::AnimatedDefaultTextStyle>(
                        &**element.widget(&app),
                    )
                    .is_some()
                })
                .expect("an AnimatedDefaultTextStyle above the label");
            tester.widget::<waft_widgets::AnimatedDefaultTextStyle, _>(ancestor, |style| {
                style.style.clone()
            })
        }
    }
    // The cases on the decoration and theme values.
    mod data {
        use waft_widgets::testing::test_widgets;

        use waft_embedder::{Color, Radius};
        use waft_painting::{
            AnyColor, AnyTextStyle, BorderRadius, BorderSide, EdgeInsets, EdgeInsetsDirectional,
            EdgeInsetsGeometry, TextStyle,
        };
        use waft_rendering::BoxConstraints;

        use super::super::*;
        use crate::colors::Colors;
        use crate::input_border::{InputBorderRef, OutlineInputBorder, UnderlineInputBorder};
        use crate::theme_data::ThemeData;

        fn constraints(
            min_width: f64,
            max_width: f64,
            min_height: f64,
            max_height: f64,
        ) -> BoxConstraints {
            BoxConstraints::new()
                .min_width(min_width)
                .max_width(max_width)
                .min_height(min_height)
                .max_height(max_height)
        }

        fn side(color: u32) -> BorderSide {
            BorderSide {
                color: Color::new(color),
                ..BorderSide::default()
            }
        }

        fn border(border: impl Into<InputBorderRef>) -> Option<InputBorderRef> {
            Some(border.into())
        }

        fn style(color: u32) -> Option<AnyTextStyle> {
            Some(TextStyle::new().color(Color::new(color)).into())
        }

        fn color(color: u32) -> Option<AnyColor> {
            Some(Color::new(color).into())
        }

        // input_decorator_test.dart 'InputDecorationThemeData.isCollapsed is applied'.
        #[test]
        fn a_theme_is_collapsed_is_applied() {
            let decoration = InputDecoration::new()
                .hint_text("Hello, Flutter!")
                .apply_defaults(InputDecorationThemeData::new().is_collapsed(true));

            assert_eq!(decoration.is_collapsed, Some(true));
        }

        // input_decorator_test.dart 'InputDecorationThemeData.isCollapsed defaults to false'.
        #[test]
        fn a_theme_is_collapsed_defaults_to_false() {
            let decoration = InputDecoration::new()
                .hint_text("Hello, Flutter!")
                .apply_defaults(InputDecorationThemeData::new());

            assert_eq!(decoration.is_collapsed, Some(false));
        }

        // input_decorator_test.dart 'InputDecorationThemeData.isCollapsed can be overridden'.
        #[test]
        fn a_theme_is_collapsed_can_be_overridden() {
            let decoration = InputDecoration::new()
                .is_collapsed(true)
                .hint_text("Hello, Flutter!")
                .apply_defaults(InputDecorationThemeData::new());

            assert_eq!(decoration.is_collapsed, Some(true));
        }

        // input_decorator_test.dart 'FloatingLabelAlignment.toString()'.
        #[test]
        fn floating_label_alignment_prints_its_name() {
            assert_eq!(
                format!("{:?}", FloatingLabelAlignment::START),
                "FloatingLabelAlignment.start"
            );
            assert_eq!(
                format!("{:?}", FloatingLabelAlignment::CENTER),
                "FloatingLabelAlignment.center"
            );
        }

        // input_decorator_test.dart 'InputDecoration.applyDefaults initializes empty field'.
        #[test]
        fn apply_defaults_initializes_empty_fields() {
            let theme_style = style(0xFF00FFFF);
            let theme_color = Color::new(0xFF00FF00);
            let theme_input_border = OutlineInputBorder::new().border_side(side(0xFF0000FF));

            let decoration = InputDecoration::new().apply_defaults(
                InputDecorationThemeData::new()
                    .label_style(theme_style.clone().unwrap())
                    .floating_label_style(theme_style.clone().unwrap())
                    .helper_style(theme_style.clone().unwrap())
                    .helper_max_lines(2)
                    .hint_style(theme_style.clone().unwrap())
                    .error_style(theme_style.clone().unwrap())
                    .error_max_lines(2)
                    .floating_label_behavior(FloatingLabelBehavior::Never)
                    .floating_label_alignment(FloatingLabelAlignment::CENTER)
                    .is_dense(true)
                    .content_padding(EdgeInsetsGeometry::all(1.0))
                    .icon_color(theme_color)
                    .prefix_style(theme_style.clone().unwrap())
                    .prefix_icon_color(theme_color)
                    .prefix_icon_constraints(constraints(10.0, 10.0, 30.0, 30.0))
                    .suffix_style(theme_style.clone().unwrap())
                    .suffix_icon_color(theme_color)
                    .suffix_icon_constraints(constraints(20.0, 20.0, 40.0, 40.0))
                    .counter_style(theme_style.clone().unwrap())
                    .filled(true)
                    .fill_color(theme_color)
                    .focus_color(theme_color)
                    .hover_color(theme_color)
                    .error_border(theme_input_border)
                    .focused_border(theme_input_border)
                    .focused_error_border(theme_input_border)
                    .disabled_border(theme_input_border)
                    .enabled_border(theme_input_border)
                    .border(InputBorderRef::none())
                    .align_label_with_hint(true)
                    .constraints(constraints(10.0, 20.0, 30.0, 40.0)),
            );

            assert_eq!(decoration.label_style, theme_style);
            assert_eq!(decoration.floating_label_style, theme_style);
            assert_eq!(decoration.helper_style, theme_style);
            assert_eq!(decoration.helper_max_lines, Some(2));
            assert_eq!(decoration.hint_style, theme_style);
            assert_eq!(decoration.error_style, theme_style);
            assert_eq!(decoration.error_max_lines, Some(2));
            assert_eq!(
                decoration.floating_label_behavior,
                Some(FloatingLabelBehavior::Never)
            );
            assert_eq!(
                decoration.floating_label_alignment,
                Some(FloatingLabelAlignment::CENTER)
            );
            assert_eq!(decoration.is_dense, Some(true));
            assert_eq!(
                decoration.content_padding,
                Some(EdgeInsetsGeometry::all(1.0))
            );
            assert_eq!(decoration.icon_color, Some(theme_color.into()));
            assert_eq!(decoration.prefix_style, theme_style);
            assert_eq!(decoration.prefix_icon_color, Some(theme_color.into()));
            assert_eq!(
                decoration.prefix_icon_constraints,
                Some(constraints(10.0, 10.0, 30.0, 30.0))
            );
            assert_eq!(decoration.suffix_style, theme_style);
            assert_eq!(decoration.suffix_icon_color, Some(theme_color.into()));
            assert_eq!(
                decoration.suffix_icon_constraints,
                Some(constraints(20.0, 20.0, 40.0, 40.0))
            );
            assert_eq!(decoration.counter_style, theme_style);
            assert_eq!(decoration.filled, Some(true));
            assert_eq!(decoration.fill_color, Some(theme_color.into()));
            assert_eq!(decoration.focus_color, Some(theme_color));
            assert_eq!(decoration.hover_color, Some(theme_color));
            assert_eq!(decoration.error_border, border(theme_input_border));
            assert_eq!(decoration.focused_border, border(theme_input_border));
            assert_eq!(decoration.focused_error_border, border(theme_input_border));
            assert_eq!(decoration.disabled_border, border(theme_input_border));
            assert_eq!(decoration.enabled_border, border(theme_input_border));
            assert_eq!(decoration.border, Some(InputBorderRef::none()));
            assert_eq!(decoration.align_label_with_hint, Some(true));
            assert_eq!(
                decoration.constraints,
                Some(constraints(10.0, 20.0, 30.0, 40.0))
            );
        }

        // input_decorator_test.dart 'InputDecoration.applyDefaults does not override non-null fields'.
        #[test]
        fn apply_defaults_does_not_override_non_null_fields() {
            let theme_style = style(0xFF00FFFF);
            let theme_color = Color::new(0xFF00FF00);
            let theme_input_border = OutlineInputBorder::new().border_side(side(0xFF0000FF));
            let decoration_style = style(0xFFFFFF00);
            let decoration_color = Color::new(0xFF0000FF);
            let decoration_input_border = OutlineInputBorder::new().border_side(side(0xFFFF00FF));
            let decoration_constraints = constraints(40.0, 50.0, 60.0, 70.0);

            let decoration = InputDecoration::new()
                .label_style(decoration_style.clone().unwrap())
                .floating_label_style(decoration_style.clone().unwrap())
                .helper_style(decoration_style.clone().unwrap())
                .helper_max_lines(3)
                .hint_style(decoration_style.clone().unwrap())
                .error_style(decoration_style.clone().unwrap())
                .error_max_lines(3)
                .floating_label_behavior(FloatingLabelBehavior::Always)
                .floating_label_alignment(FloatingLabelAlignment::START)
                .is_dense(false)
                .content_padding(EdgeInsetsGeometry::all(4.0))
                .icon_color(decoration_color)
                .prefix_style(decoration_style.clone().unwrap())
                .prefix_icon_color(decoration_color)
                .prefix_icon_constraints(decoration_constraints)
                .suffix_style(decoration_style.clone().unwrap())
                .suffix_icon_color(decoration_color)
                .suffix_icon_constraints(decoration_constraints)
                .counter_style(decoration_style.clone().unwrap())
                .filled(false)
                .fill_color(decoration_color)
                .focus_color(decoration_color)
                .hover_color(decoration_color)
                .error_border(decoration_input_border)
                .focused_border(decoration_input_border)
                .focused_error_border(decoration_input_border)
                .disabled_border(decoration_input_border)
                .enabled_border(decoration_input_border)
                .border(OutlineInputBorder::new())
                .align_label_with_hint(false)
                .constraints(decoration_constraints)
                .apply_defaults(
                    InputDecorationThemeData::new()
                        .label_style(theme_style.clone().unwrap())
                        .floating_label_style(theme_style.clone().unwrap())
                        .helper_style(theme_style.clone().unwrap())
                        .helper_max_lines(2)
                        .hint_style(theme_style.clone().unwrap())
                        .error_style(theme_style.clone().unwrap())
                        .error_max_lines(2)
                        .floating_label_behavior(FloatingLabelBehavior::Never)
                        .floating_label_alignment(FloatingLabelAlignment::CENTER)
                        .is_dense(true)
                        .content_padding(EdgeInsetsGeometry::all(1.0))
                        .icon_color(theme_color)
                        .prefix_style(theme_style.clone().unwrap())
                        .prefix_icon_color(theme_color)
                        .suffix_style(theme_style.clone().unwrap())
                        .suffix_icon_color(theme_color)
                        .counter_style(theme_style.clone().unwrap())
                        .filled(true)
                        .fill_color(theme_color)
                        .focus_color(theme_color)
                        .hover_color(theme_color)
                        .error_border(theme_input_border)
                        .focused_border(theme_input_border)
                        .focused_error_border(theme_input_border)
                        .disabled_border(theme_input_border)
                        .enabled_border(theme_input_border)
                        .border(InputBorderRef::none())
                        .align_label_with_hint(true)
                        .constraints(constraints(10.0, 20.0, 30.0, 40.0)),
                );

            assert_eq!(decoration.label_style, decoration_style);
            assert_eq!(decoration.floating_label_style, decoration_style);
            assert_eq!(decoration.helper_style, decoration_style);
            assert_eq!(decoration.helper_max_lines, Some(3));
            assert_eq!(decoration.hint_style, decoration_style);
            assert_eq!(decoration.error_style, decoration_style);
            assert_eq!(decoration.error_max_lines, Some(3));
            assert_eq!(
                decoration.floating_label_behavior,
                Some(FloatingLabelBehavior::Always)
            );
            assert_eq!(
                decoration.floating_label_alignment,
                Some(FloatingLabelAlignment::START)
            );
            assert_eq!(decoration.is_dense, Some(false));
            assert_eq!(
                decoration.content_padding,
                Some(EdgeInsetsGeometry::all(4.0))
            );
            assert_eq!(decoration.icon_color, Some(decoration_color.into()));
            assert_eq!(decoration.prefix_style, decoration_style);
            assert_eq!(decoration.prefix_icon_color, Some(decoration_color.into()));
            assert_eq!(
                decoration.prefix_icon_constraints,
                Some(decoration_constraints)
            );
            assert_eq!(decoration.suffix_style, decoration_style);
            assert_eq!(decoration.suffix_icon_color, Some(decoration_color.into()));
            assert_eq!(
                decoration.suffix_icon_constraints,
                Some(decoration_constraints)
            );
            assert_eq!(decoration.counter_style, decoration_style);
            assert_eq!(decoration.filled, Some(false));
            assert_eq!(decoration.fill_color, Some(decoration_color.into()));
            assert_eq!(decoration.focus_color, Some(decoration_color));
            assert_eq!(decoration.hover_color, Some(decoration_color));
            assert_eq!(decoration.error_border, border(decoration_input_border));
            assert_eq!(decoration.focused_border, border(decoration_input_border));
            assert_eq!(
                decoration.focused_error_border,
                border(decoration_input_border)
            );
            assert_eq!(decoration.disabled_border, border(decoration_input_border));
            assert_eq!(decoration.enabled_border, border(decoration_input_border));
            assert_eq!(decoration.border, border(OutlineInputBorder::new()));
            assert_eq!(decoration.align_label_with_hint, Some(false));
            assert_eq!(decoration.constraints, Some(decoration_constraints));
        }

        // input_decorator_test.dart 'InputDecoration.applyDefaults accepts only a InputDecorationTheme
        // or a InputDecorationThemeData': the two accepted types (the third, an `Object`, does not
        // compile here).
        #[test]
        fn apply_defaults_accepts_a_theme_or_its_data() {
            let decoration = InputDecoration::new();
            assert_eq!(
                decoration.apply_defaults(InputDecorationTheme::new()),
                decoration.apply_defaults(InputDecorationThemeData::new())
            );
        }

        // input_decorator_test.dart 'InputDecorationThemeData.toString()', without the `labelStyle`
        // spot check: a `TextStyle` prints its fields in Rust's `Debug` form, not Dart's
        // `TextStyle(inherit: true, height: 1.0x)`.
        #[test]
        fn theme_data_prints_its_non_default_properties() {
            assert!(
                format!(
                    "{:?}",
                    InputDecorationThemeData::new()
                        .content_padding(EdgeInsetsDirectional::only(5.0, 0.0, 0.0, 0.0).into())
                )
                .contains("contentPadding: EdgeInsetsDirectional(5.0, 0.0, 0.0, 0.0)")
            );

            assert!(
                format!(
                    "{:?}",
                    InputDecorationThemeData::new()
                        .content_padding(EdgeInsets::only(5.0, 0.0, 0.0, 0.0).into())
                )
                .contains("contentPadding: EdgeInsets(5.0, 0.0, 0.0, 0.0)")
            );

            let debug_string = format!(
                "{:?}",
                InputDecorationThemeData::new()
                    .label_style(TextStyle::new().height(1.0))
                    .helper_style(TextStyle::new().height(2.0))
                    .helper_max_lines(5)
                    .hint_style(TextStyle::new().height(3.0))
                    .error_style(TextStyle::new().height(4.0))
                    .error_max_lines(5)
                    .is_dense(true)
                    .content_padding(EdgeInsets::only(0.0, 0.0, 6.0, 0.0).into())
                    .is_collapsed(true)
                    .prefix_style(TextStyle::new().height(7.0))
                    .suffix_style(TextStyle::new().height(8.0))
                    .counter_style(TextStyle::new().height(9.0))
                    .filled(true)
                    .fill_color(Color::new(0x00000010))
                    .focus_color(Color::new(0x00000020))
                    .error_border(UnderlineInputBorder::new())
                    .focused_border(OutlineInputBorder::new())
                    .focused_error_border(UnderlineInputBorder::new())
                    .disabled_border(OutlineInputBorder::new())
                    .enabled_border(UnderlineInputBorder::new())
                    .border(OutlineInputBorder::new())
            );

            assert!(debug_string.contains("isDense: true"));
            assert!(debug_string.contains(&format!("fillColor: {:?}", Color::new(0x00000010))));
            assert!(debug_string.contains(&format!("focusColor: {:?}", Color::new(0x00000020))));
            assert!(debug_string.contains("errorBorder: UnderlineInputBorder()"));
            assert!(debug_string.contains("focusedBorder: OutlineInputBorder()"));
        }

        // input_decorator_test.dart 'InputDecorationThemeData.copyWith keeps original iconColor'.
        #[test]
        fn theme_data_copy_with_keeps_the_original_icon_color() {
            let original = InputDecorationThemeData::new().icon_color(Color::new(0xDEADBEEF));
            assert_eq!(original.icon_color, color(0xDEADBEEF));
            assert_ne!(original.fill_color, color(0xDEADCAFE));
            let copy1 = original.copy_with().fill_color(Color::new(0xDEADCAFE));
            assert_eq!(copy1.icon_color, color(0xDEADBEEF));
            assert_eq!(copy1.fill_color, color(0xDEADCAFE));
            let copy2 = original.copy_with().icon_color(Color::new(0xDEADCAFE));
            assert_eq!(copy2.icon_color, color(0xDEADCAFE));
            assert_ne!(copy2.fill_color, color(0xDEADCAFE));
        }

        // input_decorator_test.dart 'InputDecorationThemeData copyWith, ==, hashCode basics', without
        // the `hashCode` half (deferred).
        #[test]
        fn theme_data_equals_its_copy() {
            assert_eq!(
                InputDecorationThemeData::new(),
                InputDecorationThemeData::new().copy_with()
            );
        }

        // input_decorator_test.dart 'InputDecorationThemeData copyWith correctly copies and replaces
        // values'.
        #[test]
        fn theme_data_copy_with_copies_and_replaces_values() {
            let original = InputDecorationThemeData::new()
                .focus_color(Colors::ORANGE.primary())
                .fill_color(Colors::GREEN.primary());
            let copy = original
                .copy_with()
                .focus_color(Colors::YELLOW.primary())
                .fill_color(Colors::BLUE.primary());

            assert_eq!(original.focus_color, Some(Colors::ORANGE.primary()));
            assert_eq!(original.fill_color, Some(Colors::GREEN.primary().into()));
            assert_eq!(copy.focus_color, Some(Colors::YELLOW.primary()));
            assert_eq!(copy.fill_color, Some(Colors::BLUE.primary().into()));
        }

        // input_decorator_test.dart 'InputDecorationThemeData merge'.
        #[test]
        fn theme_data_merge_fills_only_the_nullable_fields() {
            test_widgets(|tester| {
                let app = tester.cell.borrow();
                let override_theme = InputDecorationThemeData::new()
                    .label_style(TextStyle::new().color(Color::new(0x000000f0)))
                    .floating_label_style(TextStyle::new().color(Color::new(0x000000f1)))
                    .helper_style(TextStyle::new().color(Color::new(0x000000f2)))
                    .helper_max_lines(1)
                    .hint_style(TextStyle::new().color(Color::new(0x000000f3)))
                    .error_style(TextStyle::new().color(Color::new(0x000000f4)))
                    .error_max_lines(1)
                    .floating_label_behavior(FloatingLabelBehavior::Never)
                    .floating_label_alignment(FloatingLabelAlignment::CENTER)
                    .is_dense(true)
                    .content_padding(EdgeInsetsGeometry::all(1.0))
                    .is_collapsed(true)
                    .icon_color(Color::new(0x000000f5))
                    .prefix_style(TextStyle::new().color(Color::new(0x000000f6)))
                    .prefix_icon_color(Color::new(0x000000f7))
                    .suffix_style(TextStyle::new().color(Color::new(0x000000f8)))
                    .suffix_icon_color(Color::new(0x000000f9))
                    .counter_style(TextStyle::new().color(Color::new(0x00000f10)))
                    .filled(true)
                    .fill_color(Color::new(0x00000f11))
                    .active_indicator_border(side(0x00000f12))
                    .outline_border(side(0x00000f13))
                    .focus_color(Color::new(0x00000f14))
                    .hover_color(Color::new(0x00000f15))
                    .error_border(
                        OutlineInputBorder::new()
                            .border_radius(BorderRadius::all(Radius::circular(2.0))),
                    )
                    .focused_border(OutlineInputBorder::new().border_side(side(0x00000f16)))
                    .focused_error_border(OutlineInputBorder::new().border_side(side(0x00000f17)))
                    .disabled_border(OutlineInputBorder::new().border_side(side(0x00000f18)))
                    .enabled_border(OutlineInputBorder::new().border_side(side(0x00000f19)))
                    .border(OutlineInputBorder::new().border_side(side(0x00000f20)))
                    .align_label_with_hint(true)
                    .constraints(BoxConstraints::new().min_height(1.0).min_width(1.0));

                let input_decoration_theme = ThemeData::new(&app).input_decoration_theme;
                let merged = input_decoration_theme.merge(Some(&override_theme));

                assert_eq!(merged.label_style, override_theme.label_style);
                assert_eq!(
                    merged.floating_label_style,
                    override_theme.floating_label_style
                );
                assert_eq!(merged.helper_style, override_theme.helper_style);
                assert_eq!(merged.helper_max_lines, override_theme.helper_max_lines);
                assert_eq!(merged.hint_style, override_theme.hint_style);
                assert_eq!(merged.error_style, override_theme.error_style);
                assert_eq!(merged.error_max_lines, override_theme.error_max_lines);
                assert_ne!(
                    merged.floating_label_behavior,
                    override_theme.floating_label_behavior
                );
                assert_ne!(
                    merged.floating_label_alignment,
                    override_theme.floating_label_alignment
                );
                assert_ne!(merged.is_dense, override_theme.is_dense);
                assert_eq!(merged.content_padding, override_theme.content_padding);
                assert_ne!(merged.is_collapsed, override_theme.is_collapsed);
                assert_eq!(merged.icon_color, override_theme.icon_color);
                assert_eq!(merged.prefix_style, override_theme.prefix_style);
                assert_eq!(merged.prefix_icon_color, override_theme.prefix_icon_color);
                assert_eq!(merged.suffix_style, override_theme.suffix_style);
                assert_eq!(merged.suffix_icon_color, override_theme.suffix_icon_color);
                assert_eq!(merged.counter_style, override_theme.counter_style);
                assert_ne!(merged.filled, override_theme.filled);
                assert_eq!(merged.fill_color, override_theme.fill_color);
                assert_eq!(
                    merged.active_indicator_border,
                    override_theme.active_indicator_border
                );
                assert_eq!(merged.outline_border, override_theme.outline_border);
                assert_eq!(merged.focus_color, override_theme.focus_color);
                assert_eq!(merged.hover_color, override_theme.hover_color);
                assert_eq!(merged.error_border, override_theme.error_border);
                assert_eq!(merged.focused_border, override_theme.focused_border);
                assert_eq!(
                    merged.focused_error_border,
                    override_theme.focused_error_border
                );
                assert_eq!(merged.disabled_border, override_theme.disabled_border);
                assert_eq!(merged.enabled_border, override_theme.enabled_border);
                assert_eq!(merged.border, override_theme.border);
                assert_ne!(
                    merged.align_label_with_hint,
                    override_theme.align_label_with_hint
                );
                assert_eq!(merged.constraints, override_theme.constraints);
            });
        }

        // The data `InputDecorationTheme` builds from its properties takes Dart's defaults, and a
        // theme given a `data` answers it: Dart's getters and `data`.
        #[test]
        fn a_theme_widget_answers_its_properties_or_its_data() {
            let theme = InputDecorationTheme::new().with_is_dense(true);
            let data = theme.data();
            assert!(data.is_dense);
            assert_eq!(data.floating_label_behavior, FloatingLabelBehavior::Auto);
            assert_eq!(data.floating_label_alignment, FloatingLabelAlignment::START);
            assert!(!data.filled);

            let given = InputDecorationThemeData::new().hint_max_lines(3);
            let theme = InputDecorationTheme::new().with_data(given.clone());
            assert_eq!(theme.hint_max_lines(), Some(3));
            assert_eq!(theme.data(), given);
        }

        // The six cross-field asserts of Dart's `InputDecoration` constructor.
        #[test]
        #[should_panic(expected = "Declaring both hint and hintText is not supported.")]
        fn a_decoration_with_both_hint_and_hint_text_fails_its_check() {
            InputDecoration::new()
                .hint_text("hint")
                .hint(Text::new("hint"))
                .debug_check_arguments();
        }

        // InputDecoration `==` compares its widgets by identity, as Dart's widget `==` does.
        #[test]
        fn decorations_compare_widgets_by_identity_and_values_by_value() {
            let icon: WidgetRef = SizedBox::shrink().into_widget();
            let a = InputDecoration::new()
                .icon(icon.clone())
                .label_text("label");
            assert_eq!(a, a.copy_with());
            assert_eq!(a, InputDecoration::new().icon(icon).label_text("label"));
            assert_ne!(
                a,
                InputDecoration::new()
                    .icon(SizedBox::shrink())
                    .label_text("label")
            );
        }

        // Dart's `toString` lists the set fields with its quoting.
        #[test]
        fn a_decoration_prints_its_set_fields() {
            assert_eq!(
                format!(
                    "{:?}",
                    InputDecoration::new()
                        .label_text("label")
                        .helper_max_lines(2)
                        .enabled(false)
                ),
                "InputDecoration(labelText: \"label\", helperMaxLines: \"2\", enabled: false)"
            );
            assert_eq!(
                format!("{:?}", InputDecoration::collapsed(Some("hint".to_owned()))),
                "InputDecoration(hintText: \"hint\", contentPadding: EdgeInsets.zero, \
             isCollapsed: true, border: _NoInputBorder(), alignLabelWithHint: false)"
            );
        }
    }

    // input_decorator_test.dart's 'Material3 - InputDecoration container' group and the input
    // border cases after it.
    #[allow(deprecated)]
    mod container_and_borders {
        use std::sync::Arc;
        use waft_widgets::testing::{
            TargetPlatformVariant, WidgetTester, test_widgets, test_widgets_variant,
        };

        use waft_embedder::valo::Op;
        use waft_embedder::{
            Canvas, Color, FillRule, Offset, PaintStyle, RRect, Radius, Rect, TargetPlatform,
        };
        use waft_painting::{
            Alignment, AlignmentGeometry, BorderRadius, BorderSide, EdgeInsetsGeometry,
            ShapeBorder, TextDirection, draw_drrect, draw_rrect,
        };
        use waft_widgets::{AnyElement, Container, Icon, IntoWidget, RepaintBoundary, WidgetRef};

        use super::super::*;
        use super::common::K_TRANSITION_DURATION;
        use super::common::*;
        use crate::app::MaterialApp;
        use crate::colors::Colors;
        use crate::icons::Icons;
        use crate::input_border::{InputBorderRef, OutlineInputBorder, UnderlineInputBorder};
        use crate::scaffold::Scaffold;
        use crate::test_support::PaintInspection;
        use crate::theme::Theme;
        use crate::theme_data::{ThemeData, VisualDensity};

        // Default container height for InputDecorator (filled or outlined) is 56dp on mobile
        // whether the label is floating or not.
        // This value is taken from https://m3.material.io/components/text-fields/specs.
        const CONTAINER_HEIGHT: f64 = 56.0;

        // On desktop, visual density is used to reduce the container height.
        // Desktop default density is [VisualDensity.compact] which corresponds to a density value of -2.
        // As a rule of thumb, a change of 1 or -1 in density corresponds to 4 logical pixels.
        // See https://m3.material.io/foundations/layout/understanding-layout/spacing#a5674a8b-5f38-4a58-8202-5838b082390d.
        const DESKTOP_CONTAINER_HEIGHT: f64 = CONTAINER_HEIGHT - 2.0 * 4.0; // 48.0

        /// Dart's `TargetPlatformVariant.all()`.
        const ALL_PLATFORMS: [TargetPlatform; 6] = [
            TargetPlatform::Android,
            TargetPlatform::Fuchsia,
            TargetPlatform::IOS,
            TargetPlatform::Linux,
            TargetPlatform::MacOS,
            TargetPlatform::Windows,
        ];

        /// Dart's `TargetPlatformVariant.desktop()`.
        const DESKTOP_PLATFORMS: [TargetPlatform; 3] = [
            TargetPlatform::Linux,
            TargetPlatform::MacOS,
            TargetPlatform::Windows,
        ];

        fn pump(tester: &mut WidgetTester, decorator: BuildInputDecorator) {
            tester.pump_widget(decorator.build());
        }

        /// Dart's `Theme.of(tester.element(findDecorator()))`.
        fn theme_of(tester: &WidgetTester) -> Rc<ThemeData> {
            let element = tester.find_by_type::<InputDecorator>()[0];
            Theme::of(&mut tester.cell.borrow_mut(), element)
        }

        /// The decoration of the filled cases: `filled`, `labelText` and `helperText` (or
        /// `errorText` when `error`).
        fn filled(error: bool) -> InputDecoration {
            let decoration = InputDecoration::new().filled(true).label_text(LABEL_TEXT);
            if error {
                decoration.error_text(ERROR_TEXT)
            } else {
                decoration.helper_text(HELPER_TEXT)
            }
        }

        /// The decoration of the outlined cases: `OutlineInputBorder()`, `labelText` and
        /// `helperText` (or `errorText` when `error`).
        fn outlined(error: bool) -> InputDecoration {
            let decoration = InputDecoration::new()
                .border(OutlineInputBorder::new())
                .label_text(LABEL_TEXT);
            if error {
                decoration.error_text(ERROR_TEXT)
            } else {
                decoration.helper_text(HELPER_TEXT)
            }
        }

        /// The ops the render object of `element` and its subtree record.
        fn ops_of(tester: &WidgetTester, element: AnyElement) -> Vec<Op> {
            tester.painted_ops(tester.render_box_of(element).as_object())
        }

        /// Dart's `expect(findBorderPainter(), paints…)`: what the border painter records.
        fn border_painter_ops(tester: &WidgetTester) -> Vec<Op> {
            ops_of(
                tester,
                find_border_painter(tester).expect("a border painter"),
            )
        }

        /// Dart's `expect(tester.renderObject(find.byType(InputDecorator)), paints…)`.
        fn decorator_ops(tester: &WidgetTester) -> Vec<Op> {
            ops_of(tester, tester.find_by_type::<InputDecorator>()[0])
        }

        /// The path `draw` records first.
        fn path_drawn_by(draw: impl FnOnce(&mut Canvas)) -> Arc<waft_embedder::valo::Path> {
            let mut canvas = Canvas::new();
            draw(&mut canvas);
            let list = canvas.build();
            match list
                .ops()
                .iter()
                .find(|op| matches!(op, Op::DrawPath { .. }))
            {
                Some(Op::DrawPath { path, .. }) => Arc::clone(path),
                _ => panic!("the draw records a path"),
            }
        }

        /// A paint style the matchers name: `PaintingStyle.fill`, or `PaintingStyle.stroke`
        /// with an optional `strokeWidth`.
        #[derive(Clone, Copy)]
        enum Style {
            Fill,
            Stroke(Option<f64>),
        }

        fn style_matches(style: Style, paint: &waft_embedder::Paint) -> bool {
            match (style, &paint.style) {
                (Style::Fill, PaintStyle::Fill) => true,
                (Style::Stroke(None), PaintStyle::Stroke(_)) => true,
                (Style::Stroke(Some(width)), PaintStyle::Stroke(stroke)) => {
                    f64::from(stroke.width) == width
                }
                _ => false,
            }
        }

        fn color_matches(color: Option<Color>, paint: &waft_embedder::Paint) -> bool {
            let color: Option<waft_embedder::valo::Color> = color.map(Into::into);
            color.is_none_or(|color| paint.color == color)
        }

        /// One call of Dart's `paints` matcher.
        enum Call {
            Save,
            Restore,
            /// `..rrect(style:, color:, rrect:)`.
            RRect(Style, Option<Color>, Option<RRect>),
            /// `..drrect(style:, outer:, inner:)`.
            DRRect(Option<Style>, RRect, RRect),
            /// `..path(style:, includes:, excludes:)`.
            Path(Option<Style>, Vec<Offset>, Vec<Offset>),
            /// `..line(p1:, p2:, strokeWidth:)`.
            Line(Offset, Offset, f64),
        }

        fn call_matches(call: &Call, op: &Op) -> bool {
            match (call, op) {
                (Call::Save, Op::Save) => true,
                (Call::Restore, Op::Restore) => true,
                (Call::RRect(style, color, rrect), Op::DrawPath { path, paint, .. }) => {
                    let shape_matches = match rrect {
                        Some(rrect) => {
                            **path
                                == *path_drawn_by(|canvas| {
                                    draw_rrect(canvas, *rrect, &waft_embedder::Paint::default())
                                })
                        }
                        None => path.as_round_rect().is_some(),
                    };
                    shape_matches && style_matches(*style, paint) && color_matches(*color, paint)
                }
                (
                    Call::DRRect(style, outer, inner),
                    Op::DrawPath {
                        path,
                        fill_rule,
                        paint,
                        ..
                    },
                ) => {
                    let ring = path_drawn_by(|canvas| {
                        draw_drrect(canvas, *outer, *inner, &waft_embedder::Paint::default())
                    });
                    **path == *ring
                        && *fill_rule == FillRule::EvenOdd
                        && style.is_none_or(|style| style_matches(style, paint))
                }
                (Call::Path(style, includes, excludes), Op::DrawPath { path, paint, .. }) => {
                    style.is_none_or(|style| style_matches(style, paint))
                        && includes
                            .iter()
                            .all(|point| path.contains((*point).into(), FillRule::NonZero))
                        && excludes
                            .iter()
                            .all(|point| !path.contains((*point).into(), FillRule::NonZero))
                }
                (Call::Line(p1, p2, width), Op::DrawPath { path, paint, .. }) => {
                    let mut line = waft_embedder::PathBuilder::new();
                    line.move_to(*p1);
                    line.line_to(*p2);
                    **path == *line.build() && style_matches(Style::Stroke(Some(*width)), paint)
                }
                _ => false,
            }
        }

        /// Dart's `paints` matcher: `ops` makes the `calls` in order, other ops between them.
        fn paints(ops: &[Op], calls: &[Call]) -> bool {
            let mut ops = ops.iter();
            calls
                .iter()
                .all(|call| ops.by_ref().any(|op| call_matches(call, op)))
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is enabled - container has correct height and shape'.
        #[test]
        fn a_filled_enabled_container_has_the_m3_height_and_an_underline_with_rounded_top() {
            test_widgets(|tester| {
                check_filled_shape(tester, BuildInputDecorator::new(filled(false)));
            });
        }

        fn check_filled_shape(tester: &mut WidgetTester, decorator: BuildInputDecorator) {
            pump(tester, decorator);
            assert_eq!(get_container_rect(tester).height(), CONTAINER_HEIGHT);
            assert!(
                get_border(tester)
                    .expect("a border")
                    .as_any()
                    .is::<UnderlineInputBorder>()
            );
            assert_eq!(
                get_border_radius(tester),
                Some(BorderRadius::only(
                    Radius::circular(4.0),
                    Radius::circular(4.0),
                    Radius::ZERO,
                    Radius::ZERO,
                ))
            );
        }

        fn check_outlined_shape(tester: &mut WidgetTester, decorator: BuildInputDecorator) {
            pump(tester, decorator);
            assert_eq!(get_container_rect(tester).height(), CONTAINER_HEIGHT);
            assert!(
                get_border(tester)
                    .expect("a border")
                    .as_any()
                    .is::<OutlineInputBorder>()
            );
            assert_eq!(
                get_border_radius(tester),
                Some(BorderRadius::all(Radius::circular(4.0)))
            );
        }

        /// Dart's `expect(findBorderPainter(), paints..rrect(style: fill, color: color))`.
        fn check_filled_color(
            tester: &mut WidgetTester,
            decorator: BuildInputDecorator,
            color: fn(&ThemeData) -> Color,
        ) {
            pump(tester, decorator);
            let color = color(&theme_of(tester));
            assert!(paints(
                &border_painter_ops(tester),
                &[Call::RRect(Style::Fill, Some(color), None)]
            ));
        }

        /// Dart's `expect(findBorderPainter(), paints..path(style: PaintingStyle.stroke))`.
        fn check_outlined_painted(tester: &mut WidgetTester, decorator: BuildInputDecorator) {
            pump(tester, decorator);
            assert!(paints(
                &border_painter_ops(tester),
                &[Call::Path(
                    Some(Style::Stroke(None)),
                    Vec::new(),
                    Vec::new()
                )]
            ));
        }

        fn check_border(
            tester: &mut WidgetTester,
            decorator: BuildInputDecorator,
            color: fn(&ThemeData) -> Color,
            width: f64,
        ) {
            pump(tester, decorator);
            let color = color(&theme_of(tester));
            assert_eq!(get_border_color(tester), color);
            assert_eq!(get_border_weight(tester), width);
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is enabled - container has correct color'.
        #[test]
        fn a_filled_enabled_container_is_filled_with_surface_container_highest() {
            test_widgets(|tester| {
                check_filled_color(tester, BuildInputDecorator::new(filled(false)), |theme| {
                    theme.color_scheme.surface_container_highest()
                });
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is enabled - active indicator has correct weight and color'.
        #[test]
        fn a_filled_enabled_active_indicator_is_on_surface_variant_and_1_wide() {
            test_widgets(|tester| {
                check_border(
                    tester,
                    BuildInputDecorator::new(filled(false)),
                    |theme| theme.color_scheme.on_surface_variant(),
                    1.0,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is disabled - container has correct height and shape'.
        #[test]
        fn a_filled_disabled_container_has_the_m3_height_and_an_underline_with_rounded_top() {
            test_widgets(|tester| {
                check_filled_shape(
                    tester,
                    BuildInputDecorator::new(filled(false).enabled(false)),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is disabled - container has correct color'.
        #[test]
        fn a_filled_disabled_container_is_filled_with_4_percent_on_surface() {
            test_widgets(|tester| {
                check_filled_color(
                    tester,
                    BuildInputDecorator::new(filled(false).enabled(false)),
                    |theme| theme.color_scheme.on_surface.with_opacity(0.04),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is disabled - active indicator has correct weight and color'.
        #[test]
        fn a_filled_disabled_active_indicator_is_38_percent_on_surface_and_1_wide() {
            test_widgets(|tester| {
                check_border(
                    tester,
                    BuildInputDecorator::new(filled(false).enabled(false)),
                    |theme| theme.color_scheme.on_surface.with_opacity(0.38),
                    1.0,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is hovered - container has correct height and shape'.
        #[test]
        fn a_filled_hovered_container_has_the_m3_height_and_an_underline_with_rounded_top() {
            test_widgets(|tester| {
                check_filled_shape(
                    tester,
                    BuildInputDecorator::new(filled(false)).is_hovering(true),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is hovered - container has correct color'.
        #[test]
        fn a_filled_hovered_container_blends_the_hover_color_over_surface_container_highest() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(filled(false)).is_hovering(true),
                );
                let theme = theme_of(tester);
                assert_eq!(theme.hover_color, Colors::BLACK.with_opacity(0.04));
                assert!(paints(
                    &border_painter_ops(tester),
                    &[Call::RRect(
                        Style::Fill,
                        Some(Color::alpha_blend(
                            theme.hover_color,
                            theme.color_scheme.surface_container_highest()
                        )),
                        None
                    )]
                ));
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is hovered - active indicator has correct weight and color'.
        #[test]
        fn a_filled_hovered_active_indicator_is_on_surface_and_1_wide() {
            test_widgets(|tester| {
                check_border(
                    tester,
                    BuildInputDecorator::new(filled(false)).is_hovering(true),
                    |theme| theme.color_scheme.on_surface,
                    1.0,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is focused - container has correct height and shape'.
        #[test]
        fn a_filled_focused_container_has_the_m3_height_and_an_underline_with_rounded_top() {
            test_widgets(|tester| {
                check_filled_shape(
                    tester,
                    BuildInputDecorator::new(filled(false)).is_focused(true),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is focused - container has correct color'.
        #[test]
        fn a_filled_focused_container_is_filled_with_surface_container_highest() {
            test_widgets(|tester| {
                check_filled_color(
                    tester,
                    BuildInputDecorator::new(filled(false)).is_focused(true),
                    |theme| theme.color_scheme.surface_container_highest(),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is focused - container has correct color when focused and hovered'.
        #[test]
        fn a_filled_focused_and_hovered_container_blends_the_hover_color_over_the_fill() {
            test_widgets(|tester| {
                check_filled_color(
                    tester,
                    BuildInputDecorator::new(filled(false))
                        .is_focused(true)
                        .is_hovering(true),
                    |theme| {
                        let focus_color = theme.color_scheme.surface_container_highest();
                        let hover_color = theme.hover_color;
                        Color::alpha_blend(hover_color, focus_color)
                    },
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is focused - active indicator has correct weight and color'.
        #[test]
        fn a_filled_focused_active_indicator_is_primary_and_2_wide() {
            test_widgets(|tester| {
                check_border(
                    tester,
                    BuildInputDecorator::new(filled(false)).is_focused(true),
                    |theme| theme.color_scheme.primary,
                    2.0,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is focused - active indicator has correct weight and color when focused and
        // hovered'.
        #[test]
        fn a_filled_focused_and_hovered_active_indicator_is_primary_and_2_wide() {
            test_widgets(|tester| {
                check_border(
                    tester,
                    BuildInputDecorator::new(filled(false))
                        .is_focused(true)
                        .is_hovering(true),
                    |theme| theme.color_scheme.primary,
                    2.0,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is in error - container has correct height and shape'.
        #[test]
        fn a_filled_error_container_has_the_m3_height_and_an_underline_with_rounded_top() {
            test_widgets(|tester| {
                check_filled_shape(tester, BuildInputDecorator::new(filled(true)));
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is in error - container has correct color'.
        #[test]
        fn a_filled_error_container_is_filled_with_surface_container_highest() {
            test_widgets(|tester| {
                check_filled_color(tester, BuildInputDecorator::new(filled(true)), |theme| {
                    theme.color_scheme.surface_container_highest()
                });
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is in error - active indicator has correct weight and color'.
        #[test]
        fn a_filled_error_active_indicator_is_error_and_1_wide() {
            test_widgets(|tester| {
                check_border(
                    tester,
                    BuildInputDecorator::new(filled(true)),
                    |theme| theme.color_scheme.error,
                    1.0,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is in error - active indicator has correct weight and color when focused'.
        #[test]
        fn a_filled_focused_error_active_indicator_is_error_and_2_wide() {
            test_widgets(|tester| {
                check_border(
                    tester,
                    BuildInputDecorator::new(filled(true)).is_focused(true),
                    |theme| theme.color_scheme.error,
                    2.0,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is in error - active indicator has correct weight and color when hovered'.
        #[test]
        fn a_filled_hovered_error_active_indicator_is_on_error_container_and_1_wide() {
            test_widgets(|tester| {
                check_border(
                    tester,
                    BuildInputDecorator::new(filled(true)).is_hovering(true),
                    |theme| theme.color_scheme.on_error_container(),
                    1.0,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - when field is in error - active indicator has correct weight and color when focused and
        // hovered'.
        #[test]
        fn a_filled_focused_and_hovered_error_active_indicator_is_error_and_2_wide() {
            test_widgets(|tester| {
                check_border(
                    tester,
                    BuildInputDecorator::new(filled(true))
                        .is_focused(true)
                        .is_hovering(true),
                    |theme| theme.color_scheme.error,
                    2.0,
                );
            });
        }

        /// Pumps `decoration` on each of `platforms` and checks the container's height.
        fn check_height_on(
            platforms: &[TargetPlatform],
            decorator: impl Fn() -> BuildInputDecorator + Sync,
            height: f64,
        ) {
            for platform in platforms {
                test_widgets_variant(TargetPlatformVariant::only(*platform), |tester| {
                    tester.pump_widget(decorator().build());
                    assert_eq!(
                        get_container_rect(tester).height(),
                        height,
                        "on {platform:?}"
                    );
                });
            }
        }

        fn compact_decoration_theme(app: &App) -> ThemeData {
            ThemeData::builder()
                .input_decoration_theme(
                    InputDecorationThemeData::new().visual_density(VisualDensity::COMPACT),
                )
                .build(app)
        }

        fn compact_theme(app: &App) -> ThemeData {
            ThemeData::builder()
                .visual_density(VisualDensity::COMPACT)
                .build(app)
        }

        fn standard_decoration_theme(app: &App) -> ThemeData {
            ThemeData::builder()
                .input_decoration_theme(
                    InputDecorationThemeData::new().visual_density(VisualDensity::STANDARD),
                )
                .build(app)
        }

        fn standard_theme(app: &App) -> ThemeData {
            ThemeData::builder()
                .visual_density(VisualDensity::STANDARD)
                .build(app)
        }

        /// `ThemeData(visualDensity: compact, inputDecorationTheme: (visualDensity: standard))`.
        fn compact_theme_with_standard_decoration_theme(app: &App) -> ThemeData {
            ThemeData::builder()
                .visual_density(VisualDensity::COMPACT)
                .input_decoration_theme(
                    InputDecorationThemeData::new().visual_density(VisualDensity::STANDARD),
                )
                .build(app)
        }

        /// `ThemeData(visualDensity: compact, inputDecorationTheme: (visualDensity: comfortable))`.
        fn compact_theme_with_comfortable_decoration_theme(app: &App) -> ThemeData {
            ThemeData::builder()
                .visual_density(VisualDensity::COMPACT)
                .input_decoration_theme(
                    InputDecorationThemeData::new().visual_density(VisualDensity::COMFORTABLE),
                )
                .build(app)
        }

        type ThemeFn = fn(&App) -> ThemeData;

        /// The three density cases of a container kind: at the decoration, its theme, the theme.
        fn check_density_levels(
            decoration: fn() -> InputDecoration,
            density: VisualDensity,
            height: f64,
        ) {
            let (decoration_theme, theme): (ThemeFn, ThemeFn) = if density == VisualDensity::COMPACT
            {
                (compact_decoration_theme, compact_theme)
            } else {
                (standard_decoration_theme, standard_theme)
            };
            // Visual density configured at the decoration level.
            check_height_on(
                &ALL_PLATFORMS,
                || BuildInputDecorator::new(decoration().visual_density(density)),
                height,
            );
            // Visual density configured at the input decoration theme level.
            check_height_on(
                &ALL_PLATFORMS,
                || BuildInputDecorator::new(decoration()).theme(decoration_theme),
                height,
            );
            // Visual density configured at the theme level.
            check_height_on(
                &ALL_PLATFORMS,
                || BuildInputDecorator::new(decoration()).theme(theme),
                height,
            );
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - default container height is 48dp on desktop'.
        #[test]
        fn a_filled_container_is_48_high_on_desktop() {
            check_height_on(
                &DESKTOP_PLATFORMS,
                || BuildInputDecorator::new(filled(false)),
                DESKTOP_CONTAINER_HEIGHT,
            );
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - default container height is 48dp on all platforms when visual density is
        // VisualDensity.compact'.
        #[test]
        fn a_filled_container_is_48_high_on_every_platform_at_compact_density() {
            check_density_levels(|| filled(false), VisualDensity::COMPACT, 48.0);
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - default container height is 56dp on all platforms when visual density if
        // VisualDensity.standard'.
        #[test]
        fn a_filled_container_is_56_high_on_every_platform_at_standard_density() {
            check_density_levels(|| filled(false), VisualDensity::STANDARD, 56.0);
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - Visual density defined at the decoration level takes precedence'.
        #[test]
        fn a_filled_container_takes_the_decoration_density_over_the_themes() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        filled(false).visual_density(VisualDensity::COMFORTABLE),
                    )
                    .theme(compact_theme_with_standard_decoration_theme),
                );
                assert_eq!(get_container_rect(tester).height(), 52.0);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - Visual density defined at the input decoration theme level takes precedence'.
        #[test]
        fn a_filled_container_takes_the_decoration_theme_density_over_the_themes() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(filled(false))
                        .theme(compact_theme_with_comfortable_decoration_theme),
                );
                assert_eq!(get_container_rect(tester).height(), 52.0);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for filled text field
        // - Ambient activeIndicatorBorder is used'.
        #[test]
        fn a_filled_container_uses_the_ambient_active_indicator_border() {
            test_widgets(|tester| {
                let active_indicator_border = BorderSide {
                    color: Colors::AMBER.primary(),
                    width: 2.0,
                    ..BorderSide::default()
                };
                pump(
                    tester,
                    BuildInputDecorator::new(InputDecoration::new().label_text(LABEL_TEXT))
                        .input_decoration_theme(
                            InputDecorationThemeData::new()
                                .filled(true)
                                .active_indicator_border(active_indicator_border),
                        ),
                );
                assert_eq!(get_border_color(tester), active_indicator_border.color);
                assert_eq!(get_border_weight(tester), active_indicator_border.width);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - when field is enabled - container has correct height and shape'.
        #[test]
        fn an_outlined_enabled_container_has_the_m3_height_and_a_rounded_outline() {
            test_widgets(|tester| {
                check_outlined_shape(tester, BuildInputDecorator::new(outlined(false)));
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - when field is enabled - container is painted correctly'.
        #[test]
        fn an_outlined_enabled_container_is_stroked_and_not_filled() {
            test_widgets(|tester| {
                check_outlined_painted(tester, BuildInputDecorator::new(outlined(false)));
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - when field is enabled - outline has correct weight and color'.
        #[test]
        fn an_outlined_enabled_outline_is_outline_and_1_wide() {
            test_widgets(|tester| {
                check_border(
                    tester,
                    BuildInputDecorator::new(outlined(false)),
                    |theme| theme.color_scheme.outline(),
                    1.0,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - when field is disabled - container has correct height and shape'.
        #[test]
        fn an_outlined_disabled_container_has_the_m3_height_and_a_rounded_outline() {
            test_widgets(|tester| {
                check_outlined_shape(
                    tester,
                    BuildInputDecorator::new(outlined(false).enabled(false)),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - when field is disabled - container is painted correctly'.
        #[test]
        fn an_outlined_disabled_container_is_stroked_and_not_filled() {
            test_widgets(|tester| {
                check_outlined_painted(
                    tester,
                    BuildInputDecorator::new(outlined(false).enabled(false)),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - when field is disabled - outline has correct weight and color'.
        #[test]
        fn an_outlined_disabled_outline_is_12_percent_on_surface_and_1_wide() {
            test_widgets(|tester| {
                check_border(
                    tester,
                    BuildInputDecorator::new(outlined(false).enabled(false)),
                    |theme| theme.color_scheme.on_surface.with_opacity(0.12),
                    1.0,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - when field is hovered - container has correct height and shape'.
        #[test]
        fn an_outlined_hovered_container_has_the_m3_height_and_a_rounded_outline() {
            test_widgets(|tester| {
                check_outlined_shape(
                    tester,
                    BuildInputDecorator::new(outlined(false)).is_hovering(true),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - when field is hovered - container is painted correctly'.
        #[test]
        fn an_outlined_hovered_container_is_stroked_and_not_filled() {
            test_widgets(|tester| {
                check_outlined_painted(
                    tester,
                    BuildInputDecorator::new(outlined(false)).is_hovering(true),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - when field is hovered - outline has correct weight and color'.
        #[test]
        fn an_outlined_hovered_outline_is_on_surface_and_1_wide() {
            test_widgets(|tester| {
                check_border(
                    tester,
                    BuildInputDecorator::new(outlined(false)).is_hovering(true),
                    |theme| theme.color_scheme.on_surface,
                    1.0,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - when field is focused - container has correct height and shape'.
        #[test]
        fn an_outlined_focused_container_has_the_m3_height_and_a_rounded_outline() {
            test_widgets(|tester| {
                check_outlined_shape(
                    tester,
                    BuildInputDecorator::new(outlined(false)).is_focused(true),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - when field is focused - container is painted correctly'.
        #[test]
        fn an_outlined_focused_container_is_stroked_and_not_filled() {
            test_widgets(|tester| {
                check_outlined_painted(
                    tester,
                    BuildInputDecorator::new(outlined(false)).is_focused(true),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - when field is focused - outline has correct weight and color'.
        #[test]
        fn an_outlined_focused_outline_is_primary_and_2_wide() {
            test_widgets(|tester| {
                check_border(
                    tester,
                    BuildInputDecorator::new(outlined(false)).is_focused(true),
                    |theme| theme.color_scheme.primary,
                    2.0,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - when field is focused - outline has correct weight and color when focused and
        // hovered'.
        #[test]
        fn an_outlined_focused_and_hovered_outline_is_primary_and_2_wide() {
            test_widgets(|tester| {
                check_border(
                    tester,
                    BuildInputDecorator::new(outlined(false))
                        .is_focused(true)
                        .is_hovering(true),
                    |theme| theme.color_scheme.primary,
                    2.0,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - when field is in error - container has correct height and shape'.
        #[test]
        fn an_outlined_error_container_has_the_m3_height_and_a_rounded_outline() {
            test_widgets(|tester| {
                check_outlined_shape(tester, BuildInputDecorator::new(outlined(true)));
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - when field is in error - container is painted correctly'.
        #[test]
        fn an_outlined_error_container_is_stroked_and_not_filled() {
            test_widgets(|tester| {
                check_outlined_painted(tester, BuildInputDecorator::new(outlined(true)));
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - when field is in error - outline has correct weight and color'.
        #[test]
        fn an_outlined_error_outline_is_error_and_1_wide() {
            test_widgets(|tester| {
                check_border(
                    tester,
                    BuildInputDecorator::new(outlined(true)),
                    |theme| theme.color_scheme.error,
                    1.0,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - when field is in error - outline has correct weight and color when focused'.
        #[test]
        fn an_outlined_focused_error_outline_is_error_and_2_wide() {
            test_widgets(|tester| {
                check_border(
                    tester,
                    BuildInputDecorator::new(outlined(true)).is_focused(true),
                    |theme| theme.color_scheme.error,
                    2.0,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - when field is in error - outline has correct weight and color when hovered'.
        #[test]
        fn an_outlined_hovered_error_outline_is_on_error_container_and_1_wide() {
            test_widgets(|tester| {
                check_border(
                    tester,
                    BuildInputDecorator::new(outlined(true)).is_hovering(true),
                    |theme| theme.color_scheme.on_error_container(),
                    1.0,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - when field is in error - outline has correct weight and color when focused and
        // hovered'.
        #[test]
        fn an_outlined_focused_and_hovered_error_outline_is_error_and_2_wide() {
            test_widgets(|tester| {
                check_border(
                    tester,
                    BuildInputDecorator::new(outlined(true))
                        .is_focused(true)
                        .is_hovering(true),
                    |theme| theme.color_scheme.error,
                    2.0,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - default container height is 48dp on desktop'.
        #[test]
        fn an_outlined_container_is_48_high_on_desktop() {
            check_height_on(
                &DESKTOP_PLATFORMS,
                || BuildInputDecorator::new(outlined(false)),
                DESKTOP_CONTAINER_HEIGHT,
            );
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - default container height is 48dp on all platforms when visual density is
        // VisualDensity.compact'.
        #[test]
        fn an_outlined_container_is_48_high_on_every_platform_at_compact_density() {
            check_density_levels(|| outlined(false), VisualDensity::COMPACT, 48.0);
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - default container height is 56dp on all platforms when visual density if
        // VisualDensity.standard'.
        #[test]
        fn an_outlined_container_is_56_high_on_every_platform_at_standard_density() {
            check_density_levels(|| outlined(false), VisualDensity::STANDARD, 56.0);
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - Visual density defined at the decoration level takes precedence'.
        #[test]
        fn an_outlined_container_takes_the_decoration_density_over_the_themes() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        outlined(false).visual_density(VisualDensity::COMFORTABLE),
                    )
                    .theme(compact_theme_with_standard_decoration_theme),
                );
                assert_eq!(get_container_rect(tester).height(), 52.0);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - for outlined text
        // field - Visual density defined at the input decoration theme level takes precedence'.
        #[test]
        fn an_outlined_container_takes_the_decoration_theme_density_over_the_themes() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(outlined(false))
                        .theme(compact_theme_with_comfortable_decoration_theme),
                );
                assert_eq!(get_container_rect(tester).height(), 52.0);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - InputDecorator with
        // no input border'.
        #[test]
        fn a_decorator_with_no_input_border_has_a_zero_border_weight() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(InputDecoration::new().border(InputBorderRef::none()))
                        .is_empty(true),
                );
                assert_eq!(get_border_weight(tester), 0.0);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - OutlineInputBorder
        // async lerp': the enabled border appears and disappears while the decorator animates
        // between them, without failing. Dart's tap awaits a completer between the two changes;
        // here each change is a pumped widget.
        #[test]
        fn an_enabled_border_that_comes_and_goes_mid_animation_lerps_without_failing() {
            test_widgets(|tester| {
                let decorator = |wait_is_over: bool| {
                    let mut decoration = InputDecoration::new().label_text("Test");
                    if wait_is_over {
                        decoration = decoration.enabled_border(
                            OutlineInputBorder::new().border_side(BorderSide {
                                color: Colors::BLUE.primary(),
                                ..BorderSide::default()
                            }),
                        );
                    }
                    MaterialApp::new()
                        .home(InputDecorator::new(decoration))
                        .into_widget()
                };
                tester.pump_widget(decorator(false));
                tester.pump_widget(decorator(true));
                tester.pump_by(K_TRANSITION_DURATION);
                tester.pump_widget(decorator(false));
                tester.pump_by(K_TRANSITION_DURATION);
                assert!(get_border(tester).is_some());
            });
        }

        fn blue_side() -> BorderSide {
            BorderSide {
                color: Colors::BLUE.primary(),
                ..BorderSide::default()
            }
        }

        fn red_side() -> BorderSide {
            BorderSide {
                color: Colors::RED.primary(),
                ..BorderSide::default()
            }
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - InputBorder
        // equality'.
        #[test]
        fn input_borders_are_equal_when_their_sides_radii_and_gap_paddings_are() {
            // OutlineInputBorder's equality is defined by the borderRadius, borderSide, & gapPadding.
            let outline_input_border = OutlineInputBorder::new()
                .border_radius(BorderRadius::all(Radius::circular(9.0)))
                .border_side(blue_side())
                .gap_padding(32.0);
            assert!(
                outline_input_border.eq_shape(
                    &OutlineInputBorder::new()
                        .border_side(blue_side())
                        .border_radius(BorderRadius::all(Radius::circular(9.0)))
                        .gap_padding(32.0)
                )
            );
            assert!(!outline_input_border.eq_shape(&OutlineInputBorder::new()));
            assert!(
                !outline_input_border.eq_shape(
                    &OutlineInputBorder::new()
                        .border_side(red_side())
                        .border_radius(BorderRadius::all(Radius::circular(9.0)))
                        .gap_padding(32.0)
                )
            );
            assert!(
                !outline_input_border.eq_shape(
                    &OutlineInputBorder::new()
                        .border_side(blue_side())
                        .border_radius(BorderRadius::all(Radius::circular(10.0)))
                        .gap_padding(32.0)
                )
            );
            assert!(
                !outline_input_border.eq_shape(
                    &OutlineInputBorder::new()
                        .border_side(blue_side())
                        .border_radius(BorderRadius::all(Radius::circular(9.0)))
                        .gap_padding(33.0)
                )
            );

            // UnderlineInputBorder's equality is defined by the borderSide and borderRadius.
            let top = |radius: f64| {
                BorderRadius::only(
                    Radius::circular(radius),
                    Radius::circular(radius),
                    Radius::ZERO,
                    Radius::ZERO,
                )
            };
            let underline_input_border = UnderlineInputBorder::new()
                .border_side(blue_side())
                .border_radius(top(5.0));
            assert!(
                underline_input_border.eq_shape(
                    &UnderlineInputBorder::new()
                        .border_side(blue_side())
                        .border_radius(top(5.0))
                )
            );
            assert!(!underline_input_border.eq_shape(&UnderlineInputBorder::new()));
            assert!(
                !underline_input_border.eq_shape(
                    &UnderlineInputBorder::new()
                        .border_side(red_side())
                        .border_radius(top(5.0))
                )
            );
            assert!(
                !underline_input_border.eq_shape(
                    &UnderlineInputBorder::new()
                        .border_side(blue_side())
                        .border_radius(top(6.0))
                )
            );
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - OutlineInputBorder
        // borders scale down to fit when large values are passed in'.
        #[test]
        fn an_outline_with_radii_larger_than_the_decorator_fills_its_rrect() {
            test_widgets(|tester| {
                const LARGER_BORDER_RADIUS: f64 = 200.0;
                const SMALLER_BORDER_RADIUS: f64 = 100.0;
                const INPUT_DECORATOR_HEIGHT: f64 = 56.0;
                const INPUT_DECORATOR_WIDTH: f64 = 800.0;
                let border_radius = BorderRadius::only(
                    // Intentionally large values that are larger than the InputDecorator.
                    Radius::circular(SMALLER_BORDER_RADIUS),
                    Radius::circular(LARGER_BORDER_RADIUS),
                    Radius::circular(LARGER_BORDER_RADIUS),
                    Radius::circular(SMALLER_BORDER_RADIUS),
                );
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .filled(true)
                            .fill_color(Color::new(0xFF00FF00))
                            .label_text("label text")
                            .border(OutlineInputBorder::new().border_radius(border_radius)),
                    ),
                );
                assert!(paints(
                    &border_painter_ops(tester),
                    &[
                        Call::Save,
                        Call::RRect(
                            Style::Fill,
                            Some(Color::new(0xFF00FF00)),
                            Some(border_radius.to_rrect(Rect::from_ltwh(
                                0.0,
                                0.0,
                                INPUT_DECORATOR_WIDTH,
                                INPUT_DECORATOR_HEIGHT
                            )))
                        ),
                        Call::Restore,
                    ]
                ));
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - rounded
        // OutlineInputBorder with zero padding just wraps the label'.
        #[test]
        fn a_rounded_outline_with_zero_gap_padding_fills_its_rrect() {
            test_widgets(|tester| {
                const BORDER_RADIUS: f64 = 30.0;
                const LABEL: &str = "label text";
                const INPUT_DECORATOR_HEIGHT: f64 = 56.0;
                const INPUT_DECORATOR_WIDTH: f64 = 800.0;
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .filled(true)
                            .fill_color(Color::new(0xFF00FF00))
                            .label_text(LABEL)
                            .border(
                                OutlineInputBorder::new()
                                    .border_radius(BorderRadius::all(Radius::circular(
                                        BORDER_RADIUS,
                                    )))
                                    .gap_padding(0.0),
                            ),
                    ),
                );
                assert_eq!(tester.find_text(LABEL).len(), 1);
                assert!(paints(
                    &border_painter_ops(tester),
                    &[
                        Call::Save,
                        Call::RRect(
                            Style::Fill,
                            Some(Color::new(0xFF00FF00)),
                            Some(BorderRadius::all(Radius::circular(BORDER_RADIUS)).to_rrect(
                                Rect::from_ltwh(
                                    0.0,
                                    0.0,
                                    INPUT_DECORATOR_WIDTH,
                                    INPUT_DECORATOR_HEIGHT
                                )
                            ))
                        ),
                        Call::Restore,
                    ]
                ));
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - OutlineInputBorder
        // with BorderRadius.zero should draw a rectangular border'.
        #[test]
        fn an_outline_with_a_zero_radius_draws_a_rectangle() {
            test_widgets(|tester| {
                const LABEL: &str = "Flutter";
                const INPUT_DECORATOR_HEIGHT: f64 = 56.0;
                const INPUT_DECORATOR_WIDTH: f64 = 800.0;
                const BORDER_WIDTH: f64 = 4.0;
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .filled(false)
                            .label_text(LABEL)
                            .focused_border(
                                OutlineInputBorder::new()
                                    .border_radius(BorderRadius::ZERO)
                                    .border_side(BorderSide {
                                        width: BORDER_WIDTH,
                                        color: Colors::RED.primary(),
                                        ..BorderSide::default()
                                    }),
                            ),
                    )
                    .is_focused(true),
                );
                assert_eq!(tester.find_text(LABEL).len(), 1);
                assert!(paints(
                    &border_painter_ops(tester),
                    &[
                        Call::Save,
                        Call::Path(
                            None,
                            vec![
                                // Corner points in the middle of the border line should be in the path.
                                // The path is not filled and borderWidth is 4.0 so Offset(2.0, 2.0) is in the path and Offset(1.0, 1.0) is not.
                                // See Skia SkPath::contains method.

                                // Top-left.
                                Offset::new(BORDER_WIDTH / 2.0, BORDER_WIDTH / 2.0),
                                // Top-right.
                                Offset::new(
                                    INPUT_DECORATOR_WIDTH - 1.0 - BORDER_WIDTH / 2.0,
                                    BORDER_WIDTH / 2.0
                                ),
                                // Bottom-left.
                                Offset::new(
                                    BORDER_WIDTH / 2.0,
                                    INPUT_DECORATOR_HEIGHT - 1.0 - BORDER_WIDTH / 2.0
                                ),
                                // Bottom-right.
                                Offset::new(
                                    INPUT_DECORATOR_WIDTH - 1.0 - BORDER_WIDTH / 2.0,
                                    INPUT_DECORATOR_HEIGHT - 1.0 - BORDER_WIDTH / 2.0
                                ),
                            ],
                            vec![
                                // The path is not filled and borderWidth is 4.0 so the path should not contains the corner points.
                                // See Skia SkPath::contains method.

                                // Top-left.
                                Offset::ZERO,
                                // Top-right.
                                Offset::new(INPUT_DECORATOR_WIDTH - 1.0, 0.0),
                                // Bottom-left.
                                Offset::new(0.0, INPUT_DECORATOR_HEIGHT - 1.0),
                                // Bottom-right.
                                Offset::new(
                                    INPUT_DECORATOR_WIDTH - 1.0,
                                    INPUT_DECORATOR_HEIGHT - 1.0
                                ),
                            ],
                        ),
                        Call::Restore,
                    ]
                ));
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - InputDecorator
        // OutlineInputBorder fillColor is clipped by border'.
        #[test]
        fn an_outline_fill_is_its_rounded_rect_under_the_stroked_outline() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .filled(true)
                            .fill_color(Color::new(0xFF00FF00))
                            .border(
                                OutlineInputBorder::new()
                                    .border_radius(BorderRadius::all(Radius::circular(12.0))),
                            ),
                    ),
                );
                assert!(paints(
                    &decorator_ops(tester),
                    &[
                        // The border's outer path, a rounded rectangle.
                        Call::RRect(
                            Style::Fill,
                            Some(Color::new(0xFF00FF00)),
                            Some(
                                BorderRadius::all(Radius::circular(12.0))
                                    .to_rrect(Rect::from_ltwh(0.0, 0.0, 800.0, 56.0))
                            )
                        ),
                        // Border outline. The rrect is the -center- of the 1.0 stroked outline.
                        Call::RRect(
                            Style::Stroke(Some(1.0)),
                            None,
                            Some(RRect::from_ltrbr(
                                0.5,
                                0.5,
                                799.5,
                                55.5,
                                Radius::circular(11.5)
                            ))
                        ),
                    ]
                ));
            });
        }

        const STROKE_BORDER_WIDTH: f64 = 4.0;
        const STROKE_BORDER_RADIUS: f64 = 12.0;
        const STROKE_DECORATOR_WIDTH: f64 = 800.0;
        const STROKE_DECORATOR_HEIGHT: f64 = 56.0;

        /// Dart's `testStrokeAlign` of the 'OutlineInputBorder strokeAlign' group.
        fn check_stroke_align(tester: &mut WidgetTester, stroke_align: f64, expected_rrect: RRect) {
            pump(
                tester,
                BuildInputDecorator::new(
                    InputDecoration::new()
                        .filled(true)
                        .fill_color(Color::new(0xFF00FF00))
                        .enabled_border(
                            OutlineInputBorder::new()
                                .border_radius(BorderRadius::all(Radius::circular(
                                    STROKE_BORDER_RADIUS,
                                )))
                                .border_side(BorderSide {
                                    width: STROKE_BORDER_WIDTH,
                                    stroke_align,
                                    ..BorderSide::default()
                                }),
                        ),
                ),
            );
            assert!(paints(
                &decorator_ops(tester),
                &[
                    // Background fill of the InputDecorator.
                    Call::RRect(
                        Style::Fill,
                        Some(Color::new(0xFF00FF00)),
                        Some(RRect::from_ltrbr(
                            0.0,
                            0.0,
                            STROKE_DECORATOR_WIDTH,
                            STROKE_DECORATOR_HEIGHT,
                            Radius::circular(STROKE_BORDER_RADIUS),
                        ))
                    ),
                    // The stroked OutlineInputBorder.
                    Call::RRect(
                        Style::Stroke(Some(STROKE_BORDER_WIDTH)),
                        None,
                        Some(expected_rrect)
                    ),
                ]
            ));
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - OutlineInputBorder
        // strokeAlign - strokeAlignOutside should draw border outside bounds'.
        #[test]
        fn an_outline_aligned_outside_strokes_outside_the_decorator() {
            test_widgets(|tester| {
                check_stroke_align(
                    tester,
                    BorderSide::STROKE_ALIGN_OUTSIDE,
                    RRect::from_ltrbr(
                        -STROKE_BORDER_WIDTH / 2.0,
                        -STROKE_BORDER_WIDTH / 2.0,
                        STROKE_DECORATOR_WIDTH + STROKE_BORDER_WIDTH / 2.0,
                        STROKE_DECORATOR_HEIGHT + STROKE_BORDER_WIDTH / 2.0,
                        Radius::circular(STROKE_BORDER_RADIUS + STROKE_BORDER_WIDTH / 2.0),
                    ),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - OutlineInputBorder
        // strokeAlign - strokeAlignCenter should draw border between bounds'.
        #[test]
        fn an_outline_aligned_center_strokes_on_the_decorator_edge() {
            test_widgets(|tester| {
                check_stroke_align(
                    tester,
                    BorderSide::STROKE_ALIGN_CENTER,
                    RRect::from_ltrbr(
                        0.0,
                        0.0,
                        STROKE_DECORATOR_WIDTH,
                        STROKE_DECORATOR_HEIGHT,
                        Radius::circular(STROKE_BORDER_RADIUS),
                    ),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - OutlineInputBorder
        // strokeAlign - strokeAlignInside should draw border inside bounds'.
        #[test]
        fn an_outline_aligned_inside_strokes_inside_the_decorator() {
            test_widgets(|tester| {
                check_stroke_align(
                    tester,
                    BorderSide::STROKE_ALIGN_INSIDE,
                    RRect::from_ltrbr(
                        STROKE_BORDER_WIDTH / 2.0,
                        STROKE_BORDER_WIDTH / 2.0,
                        STROKE_DECORATOR_WIDTH - STROKE_BORDER_WIDTH / 2.0,
                        STROKE_DECORATOR_HEIGHT - STROKE_BORDER_WIDTH / 2.0,
                        Radius::circular(STROKE_BORDER_RADIUS - STROKE_BORDER_WIDTH / 2.0),
                    ),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - InputDecorator
        // UnderlineInputBorder fillColor is clipped by border'.
        #[test]
        fn an_underline_with_rounded_bottom_corners_paints_its_bottom_as_a_ring() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .filled(true)
                            .fill_color(Color::new(0xFF00FF00))
                            .border(
                                UnderlineInputBorder::new().border_radius(BorderRadius::only(
                                    Radius::ZERO,
                                    Radius::ZERO,
                                    Radius::circular(12.0),
                                    Radius::circular(12.0),
                                )),
                            ),
                    ),
                );
                // Fill is the border's outer path, a rounded rectangle.
                assert!(paints(
                    &decorator_ops(tester),
                    &[Call::DRRect(
                        Some(Style::Fill),
                        RRect::from_ltrb_and_corners(
                            0.0,
                            0.0,
                            800.0,
                            48.0,
                            Radius::ZERO,
                            Radius::ZERO,
                            Radius::elliptical(12.0, 12.0),
                            Radius::elliptical(12.0, 12.0),
                        ),
                        RRect::from_ltrb_and_corners(
                            0.0,
                            0.0,
                            800.0,
                            47.0,
                            Radius::ZERO,
                            Radius::ZERO,
                            Radius::elliptical(12.0, 11.0),
                            Radius::elliptical(12.0, 11.0),
                        ),
                    )]
                ));
            });
        }

        /// Dart's `(Canvas canvas) => border.paint(canvas, canvasRect)` as a `paints` subject.
        fn ops_of_border(border: &dyn ShapeBorder, rect: Rect) -> Vec<Op> {
            let mut canvas = Canvas::new();
            border.paint(&mut canvas, rect, None);
            canvas.build().ops().to_vec()
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - UnderlineInputBorder
        // clips top border to prevent anti-aliasing glitches'.
        #[test]
        fn an_underline_paints_only_its_bottom_side_with_bottom_radii_clamped_to_half_its_height() {
            let canvas_rect = Rect::from_ltwh(0.0, 0.0, 100.0, 100.0);
            let border = UnderlineInputBorder::new()
                .border_radius(BorderRadius::all(Radius::circular(12.0)));
            assert!(paints(
                &ops_of_border(&border, canvas_rect),
                &[Call::DRRect(
                    None,
                    RRect::from_ltrb_and_corners(
                        0.0,
                        0.0,
                        100.0,
                        100.0,
                        Radius::ZERO,
                        Radius::ZERO,
                        Radius::elliptical(12.0, 12.0),
                        Radius::elliptical(12.0, 12.0),
                    ),
                    RRect::from_ltrb_and_corners(
                        0.0,
                        0.0,
                        100.0,
                        99.0,
                        Radius::ZERO,
                        Radius::ZERO,
                        Radius::elliptical(12.0, 11.0),
                        Radius::elliptical(12.0, 11.0),
                    ),
                )]
            ));

            let border2 = UnderlineInputBorder::new()
                .border_radius(BorderRadius::all(Radius::circular(60.0)));
            assert!(
                paints(
                    &ops_of_border(&border2, canvas_rect),
                    &[Call::DRRect(
                        None,
                        RRect::from_ltrb_and_corners(
                            0.0,
                            0.0,
                            100.0,
                            100.0,
                            Radius::ZERO,
                            Radius::ZERO,
                            Radius::elliptical(50.0, 50.0),
                            Radius::elliptical(50.0, 50.0),
                        ),
                        RRect::from_ltrb_and_corners(
                            0.0,
                            0.0,
                            100.0,
                            99.0,
                            Radius::ZERO,
                            Radius::ZERO,
                            Radius::elliptical(50.0, 49.0),
                            Radius::elliptical(50.0, 49.0),
                        ),
                    )]
                ),
                "clamp is expected"
            );
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - UnderlineInputBorder
        // draws bottom border inside container bounds'.
        #[test]
        fn an_underline_draws_its_line_inside_the_bottom_of_the_rect() {
            let canvas_rect = Rect::from_ltwh(0.0, 0.0, 100.0, 100.0);
            const BORDER_WIDTH: f64 = 2.0;
            let border = UnderlineInputBorder::new().border_side(BorderSide {
                width: BORDER_WIDTH,
                ..BorderSide::default()
            });
            assert!(paints(
                &ops_of_border(&border, canvas_rect),
                &[Call::Line(
                    Offset::new(0.0, canvas_rect.height() - BORDER_WIDTH / 2.0),
                    Offset::new(100.0, canvas_rect.height() - BORDER_WIDTH / 2.0),
                    BORDER_WIDTH,
                )]
            ));
        }

        /// The `buildFrame` of the label-gap cases: a `RepaintBoundary` around `decorator` in a
        /// padded, centred `Container` of a `Scaffold`'s body.
        fn gap_frame(text_direction: TextDirection, decorator: InputDecorator) -> WidgetRef {
            MaterialApp::new()
                .home(
                    Scaffold::new().body(
                        Container::new()
                            .padding(EdgeInsetsGeometry::all(16.0))
                            .alignment(AlignmentGeometry::Alignment(Alignment::CENTER))
                            .child(waft_widgets::Directionality::new(
                                text_direction,
                                RepaintBoundary::new().child(decorator),
                            )),
                    ),
                )
                .into_widget()
        }

        /// `point` in the border container's coordinates: Dart's `borderBox.globalToLocal(point)`.
        fn border_local(tester: &WidgetTester, point: Offset) -> Offset {
            let element = find_border_painter(tester).expect("a border painter");
            let app = tester.cell.borrow();
            let border_box =
                InputDecorator::container_of(&app, element).expect("a decorator container");
            border_box.global_to_local(&app, point, None)
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - OutlineBorder starts
        // at the right position when border radius is taller than horizontal content padding'.
        #[test]
        fn an_outline_with_a_tall_radius_starts_its_gap_at_the_label() {
            test_widgets(|tester| {
                let decorator = || {
                    InputDecorator::new(
                        InputDecoration::new().label_text(LABEL_TEXT).border(
                            OutlineInputBorder::new()
                                .border_radius(BorderRadius::all(Radius::circular(30.0)))
                                .gap_padding(0.0),
                        ),
                    )
                    .is_focused(true)
                    .is_empty(true)
                };
                tester.pump_widget(gap_frame(TextDirection::Ltr, decorator()));
                // Convert label bottom left offset to border path coordinate system.
                let label_bottom_left_local_to_border =
                    border_local(tester, get_label_rect(tester).bottom_left());
                assert!(paints(
                    &border_painter_ops(tester),
                    &[
                        Call::Save,
                        Call::Path(
                            None,
                            // The points just before the label bottom left corner should be part of the border.
                            vec![
                                label_bottom_left_local_to_border + Offset::new(-1.0, 0.0),
                                label_bottom_left_local_to_border + Offset::new(-1.0, -1.0),
                            ],
                            // The label bottom left corner should not be part of the border.
                            vec![label_bottom_left_local_to_border],
                        ),
                        Call::Restore,
                    ]
                ));

                tester.pump_widget(gap_frame(TextDirection::Rtl, decorator()));
                // Convert label bottom right offset to border path coordinate system.
                let label_bottom_right_local_to_border =
                    border_local(tester, get_label_rect(tester).bottom_right());
                assert!(paints(
                    &border_painter_ops(tester),
                    &[
                        Call::Save,
                        Call::Path(
                            None,
                            // The points just after the label bottom right corner should be part of the border.
                            vec![
                                label_bottom_right_local_to_border + Offset::new(1.0, 0.0),
                                label_bottom_right_local_to_border + Offset::new(1.0, -1.0),
                            ],
                            // The label bottom right corner should not be part of the border.
                            vec![label_bottom_right_local_to_border],
                        ),
                        Call::Restore,
                    ]
                ));
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - OutlineBorder does
        // not overlap with the label at the default radius'.
        #[test]
        fn an_outline_at_the_default_radius_leaves_a_gap_exactly_under_the_label() {
            test_widgets(|tester| {
                let decorator = || {
                    InputDecorator::new(
                        InputDecoration::new()
                            .label_text(LABEL_TEXT)
                            .border(OutlineInputBorder::new().gap_padding(0.0)),
                    )
                    .is_focused(true)
                };
                for text_direction in [TextDirection::Ltr, TextDirection::Rtl] {
                    tester.pump_widget(gap_frame(text_direction, decorator()));
                    let label_rect = get_label_rect(tester);
                    let center_left = border_local(tester, label_rect.center_left());
                    let center_right = border_local(tester, label_rect.center_right());
                    assert!(
                        paints(
                            &border_painter_ops(tester),
                            &[
                                Call::Save,
                                Call::Path(
                                    None,
                                    // The points of the label edge should be part of the border.
                                    vec![center_left, center_right],
                                    // The points inside the label should not be part of the border.
                                    vec![
                                        center_left + Offset::new(1.0, 0.0),
                                        center_right + Offset::new(-1.0, 0.0),
                                    ],
                                ),
                                Call::Restore,
                            ]
                        ),
                        "{text_direction:?}"
                    );
                }
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration container - OutlineBorder does
        // not draw over label when input decorator is focused and has an icon'.
        #[test]
        fn an_outline_with_an_icon_leaves_its_gap_under_the_label() {
            test_widgets(|tester| {
                let decorator = || {
                    InputDecorator::new(
                        InputDecoration::new()
                            .icon(Icon::new(Some(Icons::insert_link())))
                            .label_text(LABEL_TEXT)
                            .border(OutlineInputBorder::new()),
                    )
                    .is_focused(true)
                    .is_empty(true)
                };
                for text_direction in [TextDirection::Ltr, TextDirection::Rtl] {
                    tester.pump_widget(gap_frame(text_direction, decorator()));
                    let label_rect = get_label_rect(tester);
                    let center_left = border_local(tester, label_rect.center_left());
                    let center_right = border_local(tester, label_rect.center_right());
                    assert!(
                        paints(
                            &border_painter_ops(tester),
                            &[
                                Call::Save,
                                Call::Path(None, Vec::new(), vec![center_left, center_right]),
                                Call::Restore,
                            ]
                        ),
                        "{text_direction:?}"
                    );
                }
            });
        }
    }

    // input_decorator_test.dart's label and hint cases.
    mod label_and_hint {
        #![allow(deprecated)]
        use waft_widgets::testing::{WidgetTester, test_widgets};

        use std::time::Duration;

        use waft_embedder::{Color, Offset, Rect, Size};
        use waft_painting::{AnyColor, BorderSide, TextDirection, TextOverflow, TextStyle};
        use waft_widgets::{
            AnyElement, Center, FadeTransition, Icon, IntoWidget, RichText, SizedBox, Text,
            WidgetRef, downcast_widget,
        };

        use super::super::*;
        use super::common::K_TRANSITION_DURATION;
        use super::common::*;
        use crate::colors::Colors;
        use crate::icons::Icons;
        use crate::input_border::OutlineInputBorder;
        use crate::theme::Theme;
        use crate::theme_data::{ThemeData, VisualDensity};

        fn ms(milliseconds: u64) -> Duration {
            Duration::from_millis(milliseconds)
        }

        /// Dart's `Theme.of(tester.element(findDecorator()))`.
        fn decorator_theme(tester: &WidgetTester) -> Rc<ThemeData> {
            let element = tester.find_by_type::<InputDecorator>()[0];
            let mut app = tester.cell.borrow_mut();
            Theme::of(&mut app, element)
        }

        fn color_of(style: &TextStyle) -> Option<Color> {
            style.color.as_ref().map(AnyColor::color)
        }

        /// Dart's `getInputRect(tester)`.
        fn get_input_rect(tester: &WidgetTester) -> Rect {
            text_rect(tester, INPUT_TEXT)
        }

        /// Dart's `getHintRect(tester)`.
        fn get_hint_rect(tester: &WidgetTester) -> Rect {
            text_rect(tester, HINT_TEXT)
        }

        /// The hint's `Text`.
        fn find_hint(tester: &WidgetTester) -> AnyElement {
            tester
                .find_text(HINT_TEXT)
                .into_iter()
                .next()
                .expect("a hint")
        }

        /// Dart's `getHintOpacity(tester)`: the opacity of the nearest `FadeTransition` above the
        /// hint.
        fn get_hint_opacity(tester: &WidgetTester) -> f64 {
            let hint = find_hint(tester);
            let fade = tester
                .ancestors(hint)
                .into_iter()
                .find(|element| {
                    let app = tester.cell.borrow();
                    downcast_widget::<FadeTransition>(&**element.widget(&app)).is_some()
                })
                .expect("a FadeTransition above the hint");
            let opacity = tester.widget(fade, |fade: &FadeTransition| fade.opacity.clone());
            let app = tester.cell.borrow();
            opacity.value(&app)
        }

        /// Dart's `getHintStyle(tester)`: the style of the `RichText` the hint's `Text` builds.
        fn get_hint_style(tester: &WidgetTester) -> TextStyle {
            let hint = tester.find_text(HINT_TEXT)[0];
            let rich_text = tester.descendants_of_type::<RichText>(hint)[0];
            tester.widget(rich_text, |rich_text: &RichText| {
                rich_text.text.style().cloned().expect("a style")
            })
        }

        fn pump(tester: &mut WidgetTester, decorator: BuildInputDecorator) {
            tester.pump_widget(decorator.build());
        }

        // ----- Material3 - InputDecoration label -----

        /// The label style expected of a decorator showing its label at rest: `bodyLarge` in
        /// `color` with its line height forced to 1.0.
        fn expected_label_style(theme: &ThemeData, color: Color) -> TextStyle {
            theme
                .text_theme
                .body_large
                .clone()
                .expect("bodyLarge")
                .copy_with()
                .color(color)
                .height(1.0)
        }

        fn filled_label() -> InputDecoration {
            InputDecoration::new().filled(true).label_text(LABEL_TEXT)
        }

        fn outlined_label() -> InputDecoration {
            InputDecoration::new()
                .border(OutlineInputBorder::new())
                .label_text(LABEL_TEXT)
                .helper_text(HELPER_TEXT)
        }

        fn check_label_color(
            tester: &mut WidgetTester,
            decorator: BuildInputDecorator,
            expected: fn(&ThemeData) -> Color,
        ) {
            pump(tester, decorator);
            let theme = decorator_theme(tester);
            assert_eq!(color_of(&get_label_style(tester)), Some(expected(&theme)));
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for filled text field -
        // label and input horizontal positions are M3 compliant in LTR'.
        #[test]
        fn filled_label_and_input_start_at_the_padding_and_the_input_gap_in_ltr() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new().filled(true).label_text(LABEL_TEXT),
                    ),
                );
                assert_eq!(get_decorator_rect(tester).size(), Size::new(800.0, 56.0));
                let label_and_input_start = 12.0 + 4.0; // Content left padding + default input gap.
                assert_eq!(get_label_rect(tester).left, label_and_input_start);
                assert_eq!(get_input_rect(tester).left, label_and_input_start);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for filled text field -
        // label and input horizontal positions are M3 compliant in RTL'.
        #[test]
        fn filled_label_and_input_start_at_the_padding_and_the_input_gap_in_rtl() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new().filled(true).label_text(LABEL_TEXT),
                    )
                    .text_direction(TextDirection::Rtl),
                );
                assert_eq!(get_decorator_rect(tester).size(), Size::new(800.0, 56.0));
                let label_and_input_start = 12.0 + 4.0; // Content left padding + default input gap.
                assert_eq!(get_label_rect(tester).right, 800.0 - label_and_input_start);
                assert_eq!(get_input_rect(tester).right, 800.0 - label_and_input_start);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for filled text field -
        // when field is enabled - label text has correct style'.
        #[test]
        fn filled_enabled_label_text_has_body_large_in_on_surface_variant() {
            test_widgets(|tester| {
                pump(tester, BuildInputDecorator::new(filled_label()));
                let theme = decorator_theme(tester);
                // Current input decorator implementation forces line height to 1.0,
                // this is not compliant with M3 spec.
                let expected =
                    expected_label_style(&theme, theme.color_scheme.on_surface_variant());
                assert_eq!(get_label_style(tester), expected);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for filled text field -
        // when field is disabled - label text has correct color'.
        #[test]
        fn filled_disabled_label_text_is_on_surface_at_38_percent() {
            test_widgets(|tester| {
                check_label_color(
                    tester,
                    BuildInputDecorator::new(filled_label().enabled(false)),
                    |theme| theme.color_scheme.on_surface.with_opacity(0.38),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for filled text field -
        // when field is hovered - label text has correct color'.
        #[test]
        fn filled_hovered_label_text_is_on_surface_variant() {
            test_widgets(|tester| {
                check_label_color(
                    tester,
                    BuildInputDecorator::new(filled_label()).is_hovering(true),
                    |theme| theme.color_scheme.on_surface_variant(),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for filled text field -
        // when field is focused - label text has correct color'.
        #[test]
        fn filled_focused_label_text_is_primary() {
            test_widgets(|tester| {
                check_label_color(
                    tester,
                    BuildInputDecorator::new(filled_label()).is_focused(true),
                    |theme| theme.color_scheme.primary,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for filled text field -
        // when field is focused - label text has correct color when focused and hovered'.
        #[test]
        fn filled_focused_and_hovered_label_text_is_primary() {
            test_widgets(|tester| {
                check_label_color(
                    tester,
                    BuildInputDecorator::new(filled_label())
                        .is_focused(true)
                        .is_hovering(true),
                    |theme| theme.color_scheme.primary,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for filled text field -
        // when field is in error - label text has correct color'.
        #[test]
        fn filled_error_label_text_is_error() {
            test_widgets(|tester| {
                check_label_color(
                    tester,
                    BuildInputDecorator::new(filled_label().error_text(ERROR_TEXT)),
                    |theme| theme.color_scheme.error,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for filled text field -
        // when field is in error - label text has correct color when focused'.
        #[test]
        fn filled_focused_error_label_text_is_error() {
            test_widgets(|tester| {
                check_label_color(
                    tester,
                    BuildInputDecorator::new(filled_label().error_text(ERROR_TEXT))
                        .is_focused(true),
                    |theme| theme.color_scheme.error,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for filled text field -
        // when field is in error - label text has correct style when hovered'.
        #[test]
        fn filled_hovered_error_label_text_is_on_error_container() {
            test_widgets(|tester| {
                check_label_color(
                    tester,
                    BuildInputDecorator::new(filled_label().error_text(ERROR_TEXT))
                        .is_hovering(true),
                    |theme| theme.color_scheme.on_error_container(),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for filled text field -
        // when field is in error - label text has correct style when focused and hovered'.
        #[test]
        fn filled_focused_and_hovered_error_label_text_is_error() {
            test_widgets(|tester| {
                check_label_color(
                    tester,
                    BuildInputDecorator::new(filled_label().error_text(ERROR_TEXT))
                        .is_focused(true)
                        .is_hovering(true),
                    |theme| theme.color_scheme.error,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for outlined text field -
        // label and input horizontal positions are M3 compliant in LTR'.
        #[test]
        fn outlined_label_and_input_start_at_the_padding_and_the_input_gap_in_ltr() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .border(OutlineInputBorder::new())
                            .label_text(LABEL_TEXT),
                    ),
                );
                assert_eq!(get_decorator_rect(tester).size(), Size::new(800.0, 56.0));
                let label_and_input_start = 12.0 + 4.0; // Content left padding + default input gap.
                assert_eq!(get_label_rect(tester).left, label_and_input_start);
                assert_eq!(get_input_rect(tester).left, label_and_input_start);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for outlined text field -
        // label and input horizontal positions are M3 compliant in RTL'.
        #[test]
        fn outlined_label_and_input_start_at_the_padding_and_the_input_gap_in_rtl() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .border(OutlineInputBorder::new())
                            .label_text(LABEL_TEXT),
                    )
                    .text_direction(TextDirection::Rtl),
                );
                assert_eq!(get_decorator_rect(tester).size(), Size::new(800.0, 56.0));
                let label_and_input_start = 12.0 + 4.0; // Content left padding + default input gap.
                assert_eq!(get_label_rect(tester).right, 800.0 - label_and_input_start);
                assert_eq!(get_input_rect(tester).right, 800.0 - label_and_input_start);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for outlined text field -
        // label and input horizontal positions can be adjusted in LTR'.
        #[test]
        fn outlined_label_and_input_start_at_a_custom_gap_padding_in_ltr() {
            test_widgets(|tester| {
                let custom_gap = 6.0;
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .border(OutlineInputBorder::new().gap_padding(custom_gap))
                            .label_text(LABEL_TEXT),
                    ),
                );
                assert_eq!(get_decorator_rect(tester).size(), Size::new(800.0, 56.0));
                let label_and_input_start = 12.0 + custom_gap; // Content left padding + input gap.
                assert_eq!(get_label_rect(tester).left, label_and_input_start);
                assert_eq!(get_input_rect(tester).left, label_and_input_start);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for outlined text field -
        // label and input horizontal positions can be adjusted in RTL'.
        #[test]
        fn outlined_label_and_input_start_at_a_custom_gap_padding_in_rtl() {
            test_widgets(|tester| {
                let custom_gap = 6.0;
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .border(OutlineInputBorder::new().gap_padding(custom_gap))
                            .label_text(LABEL_TEXT),
                    )
                    .text_direction(TextDirection::Rtl),
                );
                assert_eq!(get_decorator_rect(tester).size(), Size::new(800.0, 56.0));
                let label_and_input_start = 12.0 + custom_gap; // Content left padding + input gap.
                assert_eq!(get_label_rect(tester).right, 800.0 - label_and_input_start);
                assert_eq!(get_input_rect(tester).right, 800.0 - label_and_input_start);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for outlined text field -
        // when field is enabled - label text has correct style'.
        #[test]
        fn outlined_enabled_label_text_has_body_large_in_on_surface_variant() {
            test_widgets(|tester| {
                pump(tester, BuildInputDecorator::new(outlined_label()));
                let theme = decorator_theme(tester);
                // Current input decorator implementation forces line height to 1.0,
                // this is not compliant with M3 spec.
                let expected =
                    expected_label_style(&theme, theme.color_scheme.on_surface_variant());
                assert_eq!(get_label_style(tester), expected);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for outlined text field -
        // when field is disabled - label text has correct color'.
        #[test]
        fn outlined_disabled_label_text_is_on_surface_at_38_percent() {
            test_widgets(|tester| {
                check_label_color(
                    tester,
                    BuildInputDecorator::new(outlined_label().enabled(false)),
                    |theme| theme.color_scheme.on_surface.with_opacity(0.38),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for outlined text field -
        // when field is hovered - label text has correct color'.
        #[test]
        fn outlined_hovered_label_text_is_on_surface_variant() {
            test_widgets(|tester| {
                check_label_color(
                    tester,
                    BuildInputDecorator::new(outlined_label()).is_hovering(true),
                    |theme| theme.color_scheme.on_surface_variant(),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for outlined text field -
        // when field is focused - label text has correct color'.
        #[test]
        fn outlined_focused_label_text_is_primary() {
            test_widgets(|tester| {
                check_label_color(
                    tester,
                    BuildInputDecorator::new(outlined_label()).is_focused(true),
                    |theme| theme.color_scheme.primary,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for outlined text field -
        // when field is focused - label text has correct color when focused and hovered'.
        #[test]
        fn outlined_focused_and_hovered_label_text_is_primary() {
            test_widgets(|tester| {
                check_label_color(
                    tester,
                    BuildInputDecorator::new(outlined_label())
                        .is_focused(true)
                        .is_hovering(true),
                    |theme| theme.color_scheme.primary,
                );
            });
        }

        fn outlined_error_label() -> InputDecoration {
            InputDecoration::new()
                .border(OutlineInputBorder::new())
                .label_text(LABEL_TEXT)
                .error_text(ERROR_TEXT)
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for outlined text field -
        // when field is in error - label text has correct color'.
        #[test]
        fn outlined_error_label_text_is_error() {
            test_widgets(|tester| {
                check_label_color(
                    tester,
                    BuildInputDecorator::new(outlined_error_label()),
                    |theme| theme.color_scheme.error,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for outlined text field -
        // when field is in error - label text has correct color when focused'.
        #[test]
        fn outlined_focused_error_label_text_is_error() {
            test_widgets(|tester| {
                check_label_color(
                    tester,
                    BuildInputDecorator::new(outlined_error_label()).is_focused(true),
                    |theme| theme.color_scheme.error,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for outlined text field -
        // when field is in error - label text has correct color when hovered'.
        #[test]
        fn outlined_hovered_error_label_text_is_on_error_container() {
            test_widgets(|tester| {
                check_label_color(
                    tester,
                    BuildInputDecorator::new(outlined_error_label()).is_hovering(true),
                    |theme| theme.color_scheme.on_error_container(),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - for outlined text field -
        // when field is in error - label text has correct color when focused and hovered'.
        #[test]
        fn outlined_focused_and_hovered_error_label_text_is_error() {
            test_widgets(|tester| {
                check_label_color(
                    tester,
                    BuildInputDecorator::new(outlined_error_label())
                        .is_focused(true)
                        .is_hovering(true),
                    |theme| theme.color_scheme.error,
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - Label and input for
        // non-filled and non-outlined text field have no horizontal padding in LTR'.
        #[test]
        fn underlined_label_and_input_have_no_horizontal_padding_in_ltr() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(InputDecoration::new().label_text(LABEL_TEXT)),
                );
                assert_eq!(get_decorator_rect(tester).size(), Size::new(800.0, 56.0));
                assert_eq!(get_label_rect(tester).left, 0.0);
                assert_eq!(get_input_rect(tester).left, 0.0);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - Label and input for
        // non-filled and non-outlined text field have no horizontal padding in RTL'.
        #[test]
        fn underlined_label_and_input_have_no_horizontal_padding_in_rtl() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(InputDecoration::new().label_text(LABEL_TEXT))
                        .text_direction(TextDirection::Rtl),
                );
                assert_eq!(get_decorator_rect(tester).size(), Size::new(800.0, 56.0));
                assert_eq!(get_label_rect(tester).right, 800.0);
                assert_eq!(get_input_rect(tester).right, 800.0);
            });
        }

        fn style(color: Color) -> TextStyle {
            TextStyle::new().color(color).font_size(16.0)
        }

        fn check_label_color_and_size(tester: &WidgetTester, expected: &TextStyle) {
            let label_style = get_label_style(tester);
            assert_eq!(color_of(&label_style), color_of(expected));
            assert_eq!(label_style.font_size, expected.font_size);
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - floatingLabelStyle
        // overrides default style'.
        #[test]
        fn floating_label_style_overrides_the_default_style() {
            test_widgets(|tester| {
                let floating_label_style = style(Colors::INDIGO.primary());
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .label_text(LABEL_TEXT)
                            .floating_label_style(floating_label_style.clone()),
                    )
                    .is_empty(true)
                    // Label appears floating above input field.
                    .is_focused(true),
                );
                check_label_color_and_size(tester, &floating_label_style);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - floatingLabelStyle
        // defaults to labelStyle'.
        #[test]
        fn floating_label_style_defaults_to_label_style() {
            test_widgets(|tester| {
                let label_style = style(Colors::AMBER.primary());
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .label_text(LABEL_TEXT)
                            .label_style(label_style.clone()),
                    )
                    .is_empty(true)
                    // Label appears floating above input field.
                    .is_focused(true),
                );
                check_label_color_and_size(tester, &label_style);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - floatingLabelStyle takes
        // precedence over labelStyle'.
        #[test]
        fn floating_label_style_takes_precedence_over_label_style() {
            test_widgets(|tester| {
                let label_style = style(Colors::AMBER.primary());
                let floating_label_style = style(Colors::INDIGO.primary());
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .label_text(LABEL_TEXT)
                            .label_style(label_style)
                            .floating_label_style(floating_label_style.clone()),
                    )
                    .is_empty(true)
                    // Label appears floating above input field.
                    .is_focused(true),
                );
                check_label_color_and_size(tester, &floating_label_style);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label -
        // InputDecorationThemeData.labelStyle overrides default style'.
        #[test]
        fn the_theme_label_style_overrides_the_default_style() {
            test_widgets(|tester| {
                let label_style = style(Colors::AMBER.primary());
                pump(
                    tester,
                    // Label appears inline, on top of the input field.
                    BuildInputDecorator::new(InputDecoration::new().label_text(LABEL_TEXT))
                        .is_empty(true)
                        .input_decoration_theme(
                            InputDecorationThemeData::new().label_style(label_style.clone()),
                        ),
                );
                assert_eq!(color_of(&get_label_style(tester)), color_of(&label_style));
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label -
        // InputDecorationThemeData.floatingLabelStyle overrides default style'.
        #[test]
        fn the_theme_floating_label_style_overrides_the_default_style() {
            test_widgets(|tester| {
                let floating_label_style = style(Colors::INDIGO.primary());
                pump(
                    tester,
                    BuildInputDecorator::new(InputDecoration::new().label_text(LABEL_TEXT))
                        .is_empty(true)
                        // Label appears floating above input field.
                        .is_focused(true)
                        .input_decoration_theme(
                            InputDecorationThemeData::new()
                                .floating_label_style(floating_label_style.clone()),
                        ),
                );
                assert_eq!(
                    color_of(&get_label_style(tester)),
                    color_of(&floating_label_style)
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label - floatingLabelStyle is
        // always used when FloatingLabelBehavior.always'.
        #[test]
        fn floating_label_style_is_always_used_when_the_label_always_floats() {
            test_widgets(|tester| {
                let label_style = style(Colors::AMBER.primary());
                let floating_label_style = style(Colors::INDIGO.primary());
                let decoration = InputDecoration::new()
                    .label_text(LABEL_TEXT)
                    .label_style(label_style)
                    .floating_label_style(floating_label_style.clone())
                    .floating_label_behavior(FloatingLabelBehavior::Always);

                pump(
                    tester,
                    BuildInputDecorator::new(decoration.clone()).is_empty(true),
                );
                check_label_color_and_size(tester, &floating_label_style);

                // Focus the input decorator.
                tester.pump_widget(
                    BuildInputDecorator::new(decoration)
                        .is_empty(true)
                        .is_focused(true)
                        .build(),
                );
                check_label_color_and_size(tester, &floating_label_style);
            });
        }

        // ----- Material3 - InputDecoration labelText layout / label layout -----
        //
        // Dart runs the same layout cases with `labelText` and with the `customLabel` widget; both
        // are `label_text` here, so each pair is one case below.

        // input_decorator_test.dart 'Material3 - InputDecoration labelText layout - The label
        // appears above input' and 'Material3 - InputDecoration label layout - The label appears
        // above input'.
        #[test]
        fn the_label_appears_above_the_input() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(InputDecoration::new().label_text(LABEL_TEXT)),
                );

                // Overall height for this InputDecorator is 56dp on mobile:
                //    8 - top padding
                //   12 - floating label (font size = 16 * 0.75, line height is forced to 1.0)
                //    4 - gap between label and input (this is not part of the M3 spec)
                //   24 - input text (font size = 16, line height = 1.5)
                //    8 - bottom padding
                assert_eq!(get_decorator_rect(tester).size(), Size::new(800.0, 56.0));
                assert_eq!(get_label_rect(tester).top, 8.0);
                assert_eq!(get_label_rect(tester).bottom, 20.0);
                assert_eq!(get_input_rect(tester).top, 24.0);
                assert_eq!(get_input_rect(tester).bottom, 48.0);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration labelText layout - The label
        // appears within the input when there is no text content' and its label layout twin.
        #[test]
        fn the_label_appears_within_the_input_when_there_is_no_text_content() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(InputDecoration::new().label_text(LABEL_TEXT))
                        .is_empty(true),
                );
                assert_eq!(get_decorator_rect(tester).size(), Size::new(800.0, 56.0));
                // Label line height is forced to 1.0 and font size is 16.0,
                // the label should be vertically centered (20 pixels above and below).
                assert_eq!(get_label_rect(tester).top, 20.0);
                assert_eq!(get_label_rect(tester).bottom, 36.0);
            });
        }

        fn large_label_rect(tester: &mut WidgetTester, decoration: InputDecoration) -> Rect {
            pump(tester, BuildInputDecorator::new(decoration).is_empty(true));
            let large = tester
                .find_by_type::<SizedBox>()
                .into_iter()
                .find(|element| {
                    tester.widget(*element, |sized: &SizedBox| sized.width == Some(1000.0))
                })
                .expect("the large label");
            rect_of(tester, large)
        }

        // input_decorator_test.dart 'Material3 - InputDecoration labelText layout - When the label
        // appears within the input its padding is correct'.
        #[test]
        fn a_label_within_the_input_fills_the_space_between_the_paddings() {
            test_widgets(|tester| {
                // Define a label larger than the available decorator, the label will fill
                // all the available space (decorator width minus padding and affixes).
                let large_label = || SizedBox::new().width(1000.0).height(16.0).into_widget();

                // For filled and/or outlined decoration, the horizontal padding is 16.
                let horizontal_padding = 16.0;
                let rect = large_label_rect(
                    tester,
                    InputDecoration::new().filled(true).label(large_label()),
                );
                assert_eq!(rect.left, horizontal_padding);
                assert_eq!(rect.right, 800.0 - horizontal_padding);

                // Outlined decorator.
                let rect = large_label_rect(
                    tester,
                    InputDecoration::new()
                        .border(OutlineInputBorder::new())
                        .label(large_label()),
                );
                assert_eq!(rect.left, horizontal_padding);
                assert_eq!(rect.right, 800.0 - horizontal_padding);

                // Rebuild with affixes.
                let rect = large_label_rect(
                    tester,
                    InputDecoration::new()
                        .filled(true)
                        .label(large_label())
                        .suffix_icon(Icon::new(Some(Icons::align_horizontal_left_sharp())))
                        .prefix_icon(Icon::new(Some(Icons::align_horizontal_right_sharp()))),
                );

                // When suffixIcon and/or prefixIcon are set, the corresponding horizontal
                // padding is 52 (48 for the icon + 4 input gap based on M3 spec).
                let affixes_horizontal_padding = 52.0;
                assert_eq!(rect.left, affixes_horizontal_padding);
                assert_eq!(rect.right, 800.0 - affixes_horizontal_padding);

                // Outlined decorator.
                let rect = large_label_rect(
                    tester,
                    InputDecoration::new()
                        .border(OutlineInputBorder::new())
                        .label(large_label())
                        .suffix_icon(Icon::new(Some(Icons::align_horizontal_left_sharp())))
                        .prefix_icon(Icon::new(Some(Icons::align_horizontal_right_sharp()))),
                );
                assert_eq!(rect.left, affixes_horizontal_padding);
                assert_eq!(rect.right, 800.0 - affixes_horizontal_padding);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration labelText layout - The label
        // appears above the input when there is no content and floatingLabelBehavior is always'
        // and its label layout twin.
        #[test]
        fn an_always_floating_label_appears_above_an_empty_input() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .label_text(LABEL_TEXT)
                            .floating_label_behavior(FloatingLabelBehavior::Always),
                    )
                    .is_empty(true),
                );
                assert_eq!(get_decorator_rect(tester).size(), Size::new(800.0, 56.0));
                assert_eq!(get_label_rect(tester).top, 8.0);
                assert_eq!(get_label_rect(tester).bottom, 20.0);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration labelText layout - The label
        // appears within the input text when there is content and floatingLabelBehavior is never'
        // and its label layout twin.
        #[test]
        fn a_never_floating_label_appears_within_an_input_with_content() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .label_text(LABEL_TEXT)
                            .floating_label_behavior(FloatingLabelBehavior::Never),
                    ),
                );
                assert_eq!(get_decorator_rect(tester).size(), Size::new(800.0, 56.0));
                // Label line height is forced to 1.0 and font size is 16.0,
                // the label should be vertically centered (20 pixels above and below).
                assert_eq!(get_label_rect(tester).top, 20.0);
                assert_eq!(get_label_rect(tester).bottom, 36.0);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration labelText layout - Ambient theme
        // floatingLabelBehavior is used'.
        #[test]
        fn the_ambient_theme_floating_label_behavior_is_used() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(InputDecoration::new().label_text(LABEL_TEXT))
                        .input_decoration_theme(
                            InputDecorationThemeData::new()
                                .floating_label_behavior(FloatingLabelBehavior::Never),
                        ),
                );
                assert_eq!(get_decorator_rect(tester).size(), Size::new(800.0, 56.0));
                // Label line height is forced to 1.0 and font size is 16.0,
                // the label should be vertically centered (20 pixels above and below).
                assert_eq!(get_label_rect(tester).top, 20.0);
                assert_eq!(get_label_rect(tester).bottom, 36.0);
            });
        }

        fn assert_close_to(actual: f64, expected: f64, delta: f64) {
            assert!(
                (actual - expected).abs() <= delta,
                "{actual} is not within {delta} of {expected}"
            );
        }

        // input_decorator_test.dart 'Material3 - InputDecoration labelText layout - Floating label
        // animation duration and curve' and its label layout twin.
        #[test]
        fn the_floating_label_animates_for_167_ms_along_fast_out_slow_in() {
            test_widgets(|tester| {
                let build = |is_focused: bool| {
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .label_text(LABEL_TEXT)
                            .floating_label_behavior(FloatingLabelBehavior::Auto),
                    )
                    .is_empty(true)
                    .is_focused(is_focused)
                    .build()
                };

                tester.pump_widget(build(false));
                assert_eq!(get_label_rect(tester).top, 20.0);

                // The label animates upwards and scales down.
                // The animation duration is 167ms and the curve is fastOutSlowIn.
                tester.pump_widget(build(true));
                tester.pump_by(ms(42));
                assert_close_to(get_label_rect(tester).top, 17.09, 0.5);
                tester.pump_by(ms(42));
                assert_close_to(get_label_rect(tester).top, 10.66, 0.5);
                tester.pump_by(ms(42));
                assert_close_to(get_label_rect(tester).top, 8.47, 0.5);
                tester.pump_by(ms(41));
                assert_eq!(get_label_rect(tester).top, 8.0);

                // If the animation changes direction without first reaching the
                // AnimationStatus.completed or AnimationStatus.dismissed status,
                // the CurvedAnimation stays on the same curve in the opposite direction.
                // The pumpAndSettle is used to prevent this behavior.
                tester.pump_and_settle();

                // The label animates downwards and scales up.
                // The animation duration is 167ms and the curve is fastOutSlowIn.
                tester.pump_widget(build(false));
                tester.pump_by(ms(42));
                assert_close_to(get_label_rect(tester).top, 10.90, 0.5);
                tester.pump_by(ms(42));
                assert_close_to(get_label_rect(tester).top, 17.34, 0.5);
                tester.pump_by(ms(42));
                assert_close_to(get_label_rect(tester).top, 19.69, 0.5);
                tester.pump_by(ms(41));
                assert_eq!(get_label_rect(tester).top, 20.0);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration labelText layout - InputDecorator
        // withdraws label when not empty or focused'.
        #[test]
        fn the_label_withdraws_when_not_empty_or_focused_while_enabled() {
            test_widgets(|tester| {
                let mut pump_decorator = |focused: bool, enabled: bool, empty: bool| {
                    tester.pump_widget(
                        BuildInputDecorator::new(
                            InputDecoration::new()
                                .label_text("Label")
                                .enabled(enabled)
                                .filled(false)
                                .focused_border(OutlineInputBorder::new())
                                .disabled_border(OutlineInputBorder::new())
                                .border(OutlineInputBorder::new()),
                        )
                        .is_empty(empty)
                        .is_focused(focused)
                        .build(),
                    );
                    tester.pump_by(K_TRANSITION_DURATION);
                    let rect = text_rect(tester, "Label");
                    (rect.top_left(), rect.size())
                };

                let label_size = Size::new(82.5, 16.0);
                let floating = (Offset::new(16.0, -5.5), label_size * 0.75);
                let inline = (Offset::new(16.0, 20.0), label_size);
                assert_eq!(pump_decorator(false, true, true), inline);
                assert_eq!(pump_decorator(false, true, false), floating);
                assert_eq!(pump_decorator(true, true, true), floating);
                assert_eq!(pump_decorator(true, true, false), floating);
                assert_eq!(pump_decorator(false, false, true), inline);
                assert_eq!(pump_decorator(false, false, false), floating);
                // Focused and disabled happens with NavigationMode.directional.
                assert_eq!(pump_decorator(true, false, true), inline);
                assert_eq!(pump_decorator(true, false, false), floating);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration labelText layout - InputDecorator
        // floating label width scales when focused'. Dart reads the width from the clip the
        // label's ellipsized `Text` paints; here it is the label's laid out width.
        #[test]
        fn the_floating_label_width_scales_up_when_focused() {
            test_widgets(|tester| {
                let long_string_a = "A".repeat(200);
                let long_string_b = "B".repeat(200);
                let build = |text: &str, is_focused: bool| {
                    Center::new()
                        .child(
                            SizedBox::square(Some(100.0)).child(
                                BuildInputDecorator::new(InputDecoration::new().label_text(text))
                                    .is_empty(true)
                                    .is_focused(is_focused)
                                    .build(),
                            ),
                        )
                        .into_widget()
                };
                let label_size = |tester: &WidgetTester, text: &str| {
                    let element = tester.find_text(text)[0];
                    let render = tester.render_box_of(element);
                    let app = tester.cell.borrow();
                    render.size(&app)
                };

                tester.pump_widget(build(&long_string_a, false));
                assert_eq!(label_size(tester, &long_string_a), Size::new(100.0, 16.0));

                tester.pump_widget(build(&long_string_b, true));
                tester.pump_by(K_TRANSITION_DURATION);
                let size = label_size(tester, &long_string_b);
                // _kFinalLabelScale = 0.75
                assert!((size.width() - 100.0 / 0.75).abs() < 1e-5, "{size:?}");
                assert_eq!(size.height(), 16.0);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration labelText layout - InputDecorator
        // floating label Y coordinate'.
        #[test]
        fn the_floating_label_is_centred_on_a_wide_outline() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .label_text(LABEL_TEXT)
                            .enabled_border(OutlineInputBorder::new().border_side(BorderSide {
                                width: 4.0,
                                ..BorderSide::default()
                            }))
                            .floating_label_behavior(FloatingLabelBehavior::Always),
                    )
                    .is_empty(true),
                );
                tester.pump_by(K_TRANSITION_DURATION);

                // floatingLabelHeight = 12 (font size 16dps * 0.75 = 12)
                // labelY = -floatingLabelHeight/2 + borderWidth/2
                assert_eq!(get_label_rect(tester).top, -4.0);
            });
        }

        fn density_frames(
            tester: &mut WidgetTester,
            visual_density: VisualDensity,
            height: f64,
            inline_label: (f64, f64),
            floating_label: (f64, f64),
            content: (f64, f64),
        ) {
            let decorator = |is_empty: bool, is_focused: bool| {
                BuildInputDecorator::new(
                    InputDecoration::new()
                        .label_text(LABEL_TEXT)
                        .hint_text(HINT_TEXT),
                )
                .is_empty(is_empty)
                .is_focused(is_focused)
                .visual_density(visual_density)
                .build()
            };
            tester.pump_widget(decorator(true, false));

            assert_eq!(get_decorator_rect(tester).size(), Size::new(800.0, height));

            // The decorator is empty, label is not floating and is vertically centered.
            assert_eq!(get_label_rect(tester).top, inline_label.0);
            assert_eq!(get_label_rect(tester).bottom, inline_label.1);
            assert_eq!(get_hint_opacity(tester), 0.0);
            assert_eq!(get_border_bottom(tester), height);
            assert_eq!(get_border_weight(tester), 1.0);

            // When the decorator is focused, label moves upwards, hint is visible (opacity 1.0).
            tester.pump_widget(decorator(true, true));
            tester.pump_by(K_TRANSITION_DURATION);

            // The decorator is empty and focused, label and hint are visible.
            assert_eq!(get_decorator_rect(tester).size(), Size::new(800.0, height));
            assert_eq!(get_label_rect(tester).top, floating_label.0);
            assert_eq!(get_label_rect(tester).bottom, floating_label.1);
            assert_eq!(get_hint_rect(tester).top, content.0);
            assert_eq!(get_hint_rect(tester).bottom, content.1);
            assert_eq!(get_hint_opacity(tester), 1.0);
            assert_eq!(get_border_bottom(tester), height);
            assert_eq!(get_border_weight(tester), 2.0);

            tester.pump_widget(decorator(false, true));
            tester.pump_by(K_TRANSITION_DURATION);

            // The decorator is focused and not empty, label and input are visible.
            assert_eq!(get_decorator_rect(tester).size(), Size::new(800.0, height));
            assert_eq!(get_label_rect(tester).top, floating_label.0);
            assert_eq!(get_label_rect(tester).bottom, floating_label.1);
            assert_eq!(get_input_rect(tester).top, content.0);
            assert_eq!(get_input_rect(tester).bottom, content.1);
            assert_eq!(get_hint_opacity(tester), 0.0);
            assert_eq!(get_border_bottom(tester), height);
            assert_eq!(get_border_weight(tester), 2.0);
        }

        // input_decorator_test.dart 'Material3 - InputDecoration labelText layout - InputDecorator
        // respects reduced theme visualDensity'.
        #[test]
        fn the_decorator_respects_a_reduced_theme_visual_density() {
            test_widgets(|tester| {
                // Overall height for this InputDecorator is 48dp:
                //    4 - top padding (8 minus 4 due to reduced visual density)
                //   12 - floating label (font size = 16 * 0.75, line height is forced to 1.0)
                //    4 - gap between label and input (this is not part of the M3 spec)
                //   24 - input text (font size = 16, line height = 1.5)
                //    4 - bottom padding (8 minus 4 due to reduced visual density)
                density_frames(
                    tester,
                    VisualDensity::COMPACT,
                    48.0,
                    (16.0, 32.0),
                    (4.0, 16.0),
                    (20.0, 44.0),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration labelText layout - InputDecorator
        // respects increased theme visualDensity'.
        #[test]
        fn the_decorator_respects_an_increased_theme_visual_density() {
            test_widgets(|tester| {
                // Overall height for this InputDecorator is 64dp:
                //   12 - top padding (8 plus 4 due to increased visual density)
                //   12 - floating label (font size = 16 * 0.75, line height is forced to 1.0)
                //    4 - gap between label and input (this is not part of the M3 spec)
                //   24 - input text (font size = 16, line height = 1.5)
                //   12 - bottom padding (8 plus 4 due to increased visual density)
                density_frames(
                    tester,
                    VisualDensity::new(2.0, 2.0),
                    64.0,
                    (24.0, 40.0),
                    (12.0, 24.0),
                    (28.0, 52.0),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label layout -
        // InputDecorationThemeData.floatingLabelStyle overrides label widget styles when the widget
        // is a text widget (focused)'.
        #[test]
        fn the_theme_floating_label_style_reaches_a_text_label_widget() {
            test_widgets(|tester| {
                let floating_label_style = TextStyle::new()
                    .font_size(16.0)
                    .merge(Some(&TextStyle::new().color(Colors::INDIGO.primary())));
                pump(
                    tester,
                    BuildInputDecorator::new(InputDecoration::new().label(Text::new("label")))
                        .is_empty(true)
                        // Label appears floating above input field.
                        .is_focused(true)
                        .input_decoration_theme(
                            InputDecorationThemeData::new()
                                .floating_label_style(floating_label_style.clone()),
                        ),
                );
                // Verify that the styles were passed along.
                assert_eq!(
                    color_of(&get_label_style(tester)),
                    color_of(&floating_label_style)
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration label layout -
        // InputDecorationThemeData.labelStyle overrides label widget styles when the widget is a
        // text widget'.
        #[test]
        fn the_theme_label_style_reaches_a_text_label_widget() {
            test_widgets(|tester| {
                let label_style = TextStyle::new()
                    .font_size(16.0)
                    .merge(Some(&TextStyle::new().color(Colors::PURPLE.primary())));
                pump(
                    tester,
                    // Label appears inline, on top of the input field.
                    BuildInputDecorator::new(InputDecoration::new().label(Text::new("label")))
                        .is_empty(true)
                        .input_decoration_theme(
                            InputDecorationThemeData::new().label_style(label_style.clone()),
                        ),
                );
                // Verify that the styles were passed along.
                assert_eq!(color_of(&get_label_style(tester)), color_of(&label_style));
            });
        }

        // ----- Material3 - InputDecoration hint -----

        /// The decoration of a hint case: filled or outlined, with or without a label, the hint
        /// and label as texts.
        fn hint_decoration(outlined: bool, with_label: bool) -> InputDecoration {
            let mut decoration = InputDecoration::new().hint_text(HINT_TEXT);
            decoration = if outlined {
                decoration.border(OutlineInputBorder::new())
            } else {
                decoration.filled(true)
            };
            if with_label {
                decoration = decoration.label_text(LABEL_TEXT);
            }
            decoration
        }

        /// The decoration of a hint style case, with Dart's strings.
        fn hint_style_decoration(outlined: bool, with_label: bool) -> InputDecoration {
            let mut decoration = InputDecoration::new().hint_text(HINT_TEXT);
            decoration = if outlined {
                decoration.border(OutlineInputBorder::new())
            } else {
                decoration.filled(true)
            };
            if with_label {
                decoration = decoration.label_text(LABEL_TEXT);
            }
            decoration
        }

        /// One "hint and input align vertically" case.
        fn check_hint_alignment(
            tester: &mut WidgetTester,
            decorator: BuildInputDecorator,
            container_height: f64,
            input: (f64, f64),
            hint_opacity: f64,
        ) {
            pump(tester, decorator);
            assert_eq!(get_container_rect(tester).height(), container_height);
            assert_eq!(get_input_rect(tester).top, input.0);
            assert_eq!(get_input_rect(tester).bottom, input.1);
            assert_eq!(get_hint_rect(tester).top, get_input_rect(tester).top);
            assert_eq!(get_hint_rect(tester).bottom, get_input_rect(tester).bottom);
            assert_eq!(get_hint_opacity(tester), hint_opacity);
        }

        /// One "hint has correct style when visible" case.
        fn check_hint_style(
            tester: &mut WidgetTester,
            decorator: BuildInputDecorator,
            expected_color: fn(&ThemeData) -> Color,
        ) {
            pump(tester, decorator);
            // Hint is visible because decorator is empty.
            assert_eq!(get_hint_opacity(tester), 1.0);
            let theme = decorator_theme(tester);
            let expected_style = theme
                .text_theme
                .body_large
                .clone()
                .expect("bodyLarge")
                .copy_with()
                .color(expected_color(&theme));
            assert_eq!(get_hint_style(tester), expected_style);
        }

        fn on_surface_variant(theme: &ThemeData) -> Color {
            theme.color_scheme.on_surface_variant()
        }

        fn disabled_on_surface(theme: &ThemeData) -> Color {
            theme.color_scheme.on_surface.with_opacity(0.38)
        }

        /// Declares the "hint and input align vertically" cases of one decoration kind.
        macro_rules! hint_alignment_cases {
        ($outlined:expr, $with_label:expr, $height:expr, $input:expr,
         $($name:ident: $dart:literal, |$d:ident| $decoration:expr, |$b:ident| $builder:expr,
            $opacity:expr;)*) => {
            $(
                #[doc = concat!("input_decorator_test.dart '", $dart, "'.")]
                #[test]
                fn $name() {
                    test_widgets(|tester| {
                        let $d = hint_decoration($outlined, $with_label);
                        let $b = BuildInputDecorator::new($decoration);
                        check_hint_alignment(tester, $builder, $height, $input, $opacity);
                    });
                }
            )*
        };
    }

        // Overall height for this InputDecorator is 48dp on mobile:
        //   12 - Top padding
        //   24 - Input and hint (font size = 16, line height = 1.5)
        //   12 - Bottom padding
        hint_alignment_cases! { false, false, 48.0, (12.0, 36.0),
            filled_enabled_empty_hint_aligns_with_the_input_and_shows:
                "hint - for filled text field without label - when field is enabled - hint and input align vertically when decorator is empty",
                |d| d, |b| b.is_empty(true), 1.0;
            filled_enabled_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for filled text field without label - when field is enabled - hint and input align vertically when decorator is not empty",
                |d| d, |b| b, 0.0;
            filled_disabled_empty_hint_aligns_with_the_input_and_shows:
                "hint - for filled text field without label - when field is disabled - hint and input align vertically when decorator is empty",
                |d| d.enabled(false), |b| b.is_empty(true), 1.0;
            filled_disabled_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for filled text field without label - when field is disabled - hint and input align vertically when decorator is not empty",
                |d| d.enabled(false), |b| b, 0.0;
            filled_hovered_empty_hint_aligns_with_the_input_and_shows:
                "hint - for filled text field without label - when field is hovered - hint and input align vertically when decorator is empty",
                |d| d, |b| b.is_hovering(true).is_empty(true), 1.0;
            filled_hovered_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for filled text field without label - when field is hovered - hint and input align vertically when decorator is not empty",
                |d| d, |b| b.is_hovering(true), 0.0;
            filled_focused_empty_hint_aligns_with_the_input_and_shows:
                "hint - for filled text field without label - when field is focused - hint and input align vertically when decorator is empty",
                |d| d, |b| b.is_focused(true).is_empty(true), 1.0;
            filled_focused_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for filled text field without label - when field is focused - hint and input align vertically when decorator is not empty",
                |d| d, |b| b.is_focused(true), 0.0;
            filled_error_empty_hint_aligns_with_the_input_and_shows:
                "hint - for filled text field without label - when field is in error - hint and input align vertically when decorator is empty",
                |d| d.error_text(ERROR_TEXT), |b| b.is_empty(true), 1.0;
            filled_error_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for filled text field without label - when field is in error - hint and input align vertically when decorator is not empty",
                |d| d.error_text(ERROR_TEXT), |b| b, 0.0;
        }

        // Overall height for this InputDecorator is 56dp on mobile:
        //    8 - Top padding
        //   12 - Floating label (font size = 16 * 0.75, line height is forced to 1.0)
        //    4 - Gap between label and input (this is not part of the M3 spec)
        //   24 - Input/Hint (font size = 16, line height = 1.5)
        //    8 - Bottom padding
        hint_alignment_cases! { false, true, 56.0, (24.0, 48.0),
            filled_labelled_enabled_empty_hint_aligns_with_the_input_and_hides:
                "hint - for filled text field with label - when field is enabled - hint and input align vertically when decorator is empty",
                |d| d, |b| b.is_empty(true), 0.0;
            filled_labelled_enabled_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for filled text field with label - when field is enabled - hint and input align vertically when decorator is not empty",
                |d| d, |b| b, 0.0;
            filled_labelled_disabled_empty_hint_aligns_with_the_input_and_hides:
                "hint - for filled text field with label - when field is disabled - hint and input align vertically when decorator is empty",
                |d| d.enabled(false), |b| b.is_empty(true), 0.0;
            filled_labelled_disabled_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for filled text field with label - when field is disabled - hint and input align vertically when decorator is not empty",
                |d| d.enabled(false), |b| b, 0.0;
            filled_labelled_hovered_empty_hint_aligns_with_the_input_and_hides:
                "hint - for filled text field with label - when field is hovered - hint and input align vertically when decorator is empty",
                |d| d, |b| b.is_hovering(true).is_empty(true), 0.0;
            filled_labelled_hovered_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for filled text field with label - when field is hovered - hint and input align vertically when decorator is not empty",
                |d| d, |b| b.is_hovering(true), 0.0;
            filled_labelled_focused_empty_hint_aligns_with_the_input_and_shows:
                "hint - for filled text field with label - when field is focused - hint and input align vertically when decorator is empty",
                |d| d, |b| b.is_focused(true).is_empty(true), 1.0;
            filled_labelled_focused_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for filled text field with label - when field is focused - hint and input align vertically when decorator is not empty",
                |d| d, |b| b.is_focused(true), 0.0;
            filled_labelled_error_empty_hint_aligns_with_the_input_and_hides:
                "hint - for filled text field with label - when field is in error - hint and input align vertically when decorator is empty",
                |d| d.error_text(ERROR_TEXT), |b| b.is_empty(true), 0.0;
            filled_labelled_error_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for filled text field with label - when field is in error - hint and input align vertically when decorator is not empty",
                |d| d.error_text(ERROR_TEXT), |b| b, 0.0;
        }

        // Overall height for this InputDecorator is 56dp on mobile:
        //   16 - Top padding
        //   24 - Input and hint (font size = 16, line height = 1.5)
        //   16 - Bottom padding
        hint_alignment_cases! { true, false, 56.0, (16.0, 40.0),
            outlined_enabled_empty_hint_aligns_with_the_input_and_shows:
                "hint - for outlined text field without label - when field is enabled - hint and input align vertically when decorator is empty",
                |d| d, |b| b.is_empty(true), 1.0;
            outlined_enabled_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for outlined text field without label - when field is enabled - hint and input align vertically when decorator is not empty",
                |d| d, |b| b, 0.0;
            outlined_disabled_empty_hint_aligns_with_the_input_and_shows:
                "hint - for outlined text field without label - when field is disabled - hint and input align vertically when decorator is empty",
                |d| d.enabled(false), |b| b.is_empty(true), 1.0;
            outlined_disabled_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for outlined text field without label - when field is disabled - hint and input align vertically when decorator is not empty",
                |d| d.enabled(false), |b| b, 0.0;
            outlined_hovered_empty_hint_aligns_with_the_input_and_shows:
                "hint - for outlined text field without label - when field is hovered - hint and input align vertically when decorator is empty",
                |d| d, |b| b.is_hovering(true).is_empty(true), 1.0;
            outlined_hovered_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for outlined text field without label - when field is hovered - hint and input align vertically when decorator is not empty",
                |d| d, |b| b.is_hovering(true), 0.0;
            outlined_focused_empty_hint_aligns_with_the_input_and_shows:
                "hint - for outlined text field without label - when field is focused - hint and input align vertically when decorator is empty",
                |d| d, |b| b.is_focused(true).is_empty(true), 1.0;
            outlined_focused_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for outlined text field without label - when field is focused - hint and input align vertically when decorator is not empty",
                |d| d, |b| b.is_focused(true), 0.0;
            outlined_error_empty_hint_aligns_with_the_input_and_shows:
                "hint - for outlined text field without label - when field is in error - hint and input align vertically when decorator is empty",
                |d| d.error_text(ERROR_TEXT), |b| b.is_empty(true), 1.0;
            outlined_error_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for outlined text field without label - when field is in error - hint and input align vertically when decorator is not empty",
                |d| d.error_text(ERROR_TEXT), |b| b, 0.0;
        }

        // Overall height for this InputDecorator is 56dp on mobile:
        //   16 - Top padding
        //   24 - Input and hint (font size = 16, line height = 1.5)
        //   16 - Bottom padding
        hint_alignment_cases! { true, true, 56.0, (16.0, 40.0),
            outlined_labelled_enabled_empty_hint_aligns_with_the_input_and_hides:
                "hint - for outlined text field with label - when field is enabled - hint and input align vertically when decorator is empty",
                |d| d, |b| b.is_empty(true), 0.0;
            outlined_labelled_enabled_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for outlined text field with label - when field is enabled - hint and input align vertically when decorator is not empty",
                |d| d, |b| b, 0.0;
            outlined_labelled_disabled_empty_hint_aligns_with_the_input_and_hides:
                "hint - for outlined text field with label - when field is disabled - hint and input align vertically when decorator is empty",
                |d| d.enabled(false), |b| b.is_empty(true), 0.0;
            outlined_labelled_disabled_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for outlined text field with label - when field is disabled - hint and input align vertically when decorator is not empty",
                |d| d.enabled(false), |b| b, 0.0;
            outlined_labelled_hovered_empty_hint_aligns_with_the_input_and_hides:
                "hint - for outlined text field with label - when field is hovered - hint and input align vertically when decorator is empty",
                |d| d, |b| b.is_hovering(true).is_empty(true), 0.0;
            outlined_labelled_hovered_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for outlined text field with label - when field is hovered - hint and input align vertically when decorator is not empty",
                |d| d, |b| b.is_hovering(true), 0.0;
            outlined_labelled_focused_empty_hint_aligns_with_the_input_and_shows:
                "hint - for outlined text field with label - when field is focused - hint and input align vertically when decorator is empty",
                |d| d, |b| b.is_focused(true).is_empty(true), 1.0;
            outlined_labelled_focused_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for outlined text field with label - when field is focused - hint and input align vertically when decorator is not empty",
                |d| d, |b| b.is_focused(true), 0.0;
            outlined_labelled_error_empty_hint_aligns_with_the_input_and_hides:
                "hint - for outlined text field with label - when field is in error - hint and input align vertically when decorator is empty",
                |d| d.error_text(ERROR_TEXT), |b| b.is_empty(true), 0.0;
            outlined_labelled_error_hint_aligns_with_a_non_empty_input_and_hides:
                "hint - for outlined text field with label - when field is in error - hint and input align vertically when decorator is not empty",
                |d| d.error_text(ERROR_TEXT), |b| b, 0.0;
        }

        /// Declares the "hint has correct style when visible" cases.
        macro_rules! hint_style_cases {
        ($($name:ident: $dart:literal, $outlined:expr, $with_label:expr,
            |$d:ident| $decoration:expr, |$b:ident| $builder:expr, $color:expr;)*) => {
            $(
                #[doc = concat!("input_decorator_test.dart '", $dart, "'.")]
                #[test]
                fn $name() {
                    test_widgets(|tester| {
                        let $d = hint_style_decoration($outlined, $with_label);
                        let $b = BuildInputDecorator::new($decoration);
                        check_hint_style(tester, $builder, $color);
                    });
                }
            )*
        };
    }

        hint_style_cases! {
            filled_enabled_hint_is_body_large_in_on_surface_variant:
                "hint - for filled text field without label - when field is enabled - hint has correct style when visible",
                false, false, |d| d, |b| b.is_empty(true), on_surface_variant;
            filled_disabled_hint_is_body_large_in_on_surface_at_38_percent:
                "hint - for filled text field without label - when field is disabled - hint has correct style when visible",
                false, false, |d| d.enabled(false), |b| b.is_empty(true), disabled_on_surface;
            filled_hovered_hint_is_body_large_in_on_surface_variant:
                "hint - for filled text field without label - when field is hovered - hint has correct style when visible",
                false, false, |d| d, |b| b.is_hovering(true).is_empty(true), on_surface_variant;
            filled_focused_hint_is_body_large_in_on_surface_variant:
                "hint - for filled text field without label - when field is focused - hint has correct style when visible",
                false, false, |d| d, |b| b.is_focused(true).is_empty(true), on_surface_variant;
            filled_error_hint_is_body_large_in_on_surface_variant:
                "hint - for filled text field without label - when field is in error - hint has correct style when visible",
                false, false, |d| d.error_text(ERROR_TEXT), |b| b.is_empty(true), on_surface_variant;
            filled_labelled_focused_hint_is_body_large_in_on_surface_variant:
                "hint - for filled text field with label - when field is focused - hint has correct style when visible",
                false, true, |d| d, |b| b.is_focused(true).is_empty(true), on_surface_variant;
            outlined_enabled_hint_is_body_large_in_on_surface_variant:
                "hint - for outlined text field without label - when field is enabled - hint has correct style when visible",
                true, false, |d| d, |b| b.is_empty(true), on_surface_variant;
            outlined_disabled_hint_is_body_large_in_on_surface_at_38_percent:
                "hint - for outlined text field without label - when field is disabled - hint has correct style when visible",
                true, false, |d| d.enabled(false), |b| b.is_empty(true), disabled_on_surface;
            outlined_hovered_hint_is_body_large_in_on_surface_variant:
                "hint - for outlined text field without label - when field is hovered - hint has correct style when visible",
                true, false, |d| d, |b| b.is_hovering(true).is_empty(true), on_surface_variant;
            outlined_focused_hint_is_body_large_in_on_surface_variant:
                "hint - for outlined text field without label - when field is focused - hint has correct style when visible",
                true, false, |d| d, |b| b.is_focused(true).is_empty(true), on_surface_variant;
            outlined_error_hint_is_body_large_in_on_surface_variant:
                "hint - for outlined text field without label - when field is in error - hint has correct style when visible",
                true, false, |d| d.error_text(ERROR_TEXT), |b| b.is_empty(true), on_surface_variant;
            outlined_labelled_focused_hint_is_body_large_in_on_surface_variant:
                "hint - for outlined text field with label - when field is focused - hint has correct style when visible",
                true, true, |d| d, |b| b.is_focused(true).is_empty(true), on_surface_variant;
        }

        // ----- InputDecoration.alignLabelWithHint -----

        // input_decorator_test.dart 'InputDecoration.alignLabelWithHint - positions
        // InputDecoration.labelText vertically aligned with the hint' and 'positions
        // InputDecoration.label vertically aligned with the hint'.
        #[test]
        fn align_label_with_hint_centres_the_label_on_the_hint() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .label_text(LABEL_TEXT)
                            .align_label_with_hint(true)
                            .hint_text(HINT_TEXT),
                    )
                    .is_empty(true),
                );
                // Label and hint should be vertically aligned.
                assert_eq!(
                    get_label_rect(tester).center().dy(),
                    get_hint_rect(tester).center().dy()
                );
            });
        }

        fn ac_unit() -> WidgetRef {
            Icon::new(Some(Icons::ac_unit())).into_widget()
        }

        // input_decorator_test.dart 'InputDecoration.alignLabelWithHint - Horizontal alignment -
        // Label for outlined decoration aligns horizontally with prefixIcon by default'.
        #[test]
        fn an_outlined_label_aligns_with_the_prefix_icon_by_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .prefix_icon(ac_unit())
                            .label_text(LABEL_TEXT)
                            .border(OutlineInputBorder::new()),
                    )
                    .is_focused(true),
                );
                // Label left padding is 16.0 (12.0 right padding for a decoration with icons + 4.0
                // extra padding for the floating label)
                assert_eq!(get_label_rect(tester).left, 16.0);
                // Based on M3 spec, the expected horizontal position is 52 (12 padding, 24 icon, 16
                // gap between icon and input).
                assert_eq!(get_input_rect(tester).left, 52.0);
            });
        }

        // input_decorator_test.dart 'InputDecoration.alignLabelWithHint - Horizontal alignment -
        // Label for outlined decoration aligns horizontally with input when alignLabelWithHint is
        // true'.
        #[test]
        fn an_outlined_label_aligned_with_the_hint_aligns_with_the_input() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .prefix_icon(ac_unit())
                            .label_text(LABEL_TEXT)
                            .border(OutlineInputBorder::new())
                            .align_label_with_hint(true),
                    )
                    .is_focused(true),
                );
                assert_eq!(get_label_rect(tester).left, get_input_rect(tester).left);
            });
        }

        // input_decorator_test.dart 'InputDecoration.alignLabelWithHint - Horizontal alignment -
        // Label for filled decoration is horizontally aligned with text by default'.
        #[test]
        fn a_filled_label_aligns_with_the_input_by_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .prefix_icon(ac_unit())
                            .label_text(LABEL_TEXT)
                            .filled(true),
                    )
                    .is_focused(true),
                );
                // Label and input are horizontally aligned despite `alignLabelWithHint` being
                // false (default value).
                assert_eq!(get_label_rect(tester).left, get_input_rect(tester).left);
            });
        }

        // ----- hint opacity animation -----

        fn assert_in_exclusive_range(value: f64, low: f64, high: f64) {
            assert!(
                low < value && value < high,
                "{value} not in ({low}, {high})"
            );
        }

        /// The three 'hint opacity animation' cases: the hint fades in when the empty decorator
        /// gains the focus and out when it loses it, in steps of `step` for `steps` steps.
        fn check_hint_fade(
            tester: &mut WidgetTester,
            decoration: InputDecoration,
            theme: Option<InputDecorationThemeData>,
            step: u64,
            settle: Option<u64>,
        ) {
            let build = |is_focused: bool| {
                let mut builder = BuildInputDecorator::new(decoration.clone())
                    .is_empty(true)
                    .is_focused(is_focused);
                if let Some(theme) = &theme {
                    builder = builder.input_decoration_theme(theme.clone());
                }
                builder.build()
            };
            // Build once without focus.
            tester.pump_widget(build(false));
            // Hint is not visible (opacity 0.0).
            assert_eq!(get_hint_opacity(tester), 0.0);

            // Focus the decorator to trigger the animation.
            tester.pump_widget(build(true));
            tester.pump_by(ms(step));
            let first = get_hint_opacity(tester);
            assert_in_exclusive_range(first, 0.0, 1.0);
            tester.pump_by(ms(step));
            let second = get_hint_opacity(tester);
            assert_in_exclusive_range(second, first, 1.0);
            tester.pump_by(settle.map_or(K_TRANSITION_DURATION, ms));
            // Hint is fully visible (opacity 1.0).
            assert_eq!(get_hint_opacity(tester), 1.0);

            // Unfocus the decorator to trigger the reversed animation.
            tester.pump_widget(build(false));
            tester.pump_by(ms(step));
            let first = get_hint_opacity(tester);
            assert_in_exclusive_range(first, 0.0, 1.0);
            tester.pump_by(ms(step));
            let second = get_hint_opacity(tester);
            assert_in_exclusive_range(second, 0.0, first);
            if let Some(settle) = settle {
                tester.pump_by(ms(settle));
                assert_eq!(get_hint_opacity(tester), 0.0);
            }
        }

        // input_decorator_test.dart 'hint opacity animation - default duration'.
        #[test]
        fn the_hint_fades_in_and_out_in_20_ms_by_default() {
            test_widgets(|tester| {
                // The animation's default duration is 20ms.
                check_hint_fade(
                    tester,
                    InputDecoration::new()
                        .label_text(LABEL_TEXT)
                        .hint_text(HINT_TEXT),
                    None,
                    9,
                    None,
                );
            });
        }

        // input_decorator_test.dart 'hint opacity animation - custom duration'.
        #[test]
        fn the_hint_fades_for_the_decoration_hint_fade_duration() {
            test_widgets(|tester| {
                // The animation's duration is set to 120ms.
                check_hint_fade(
                    tester,
                    InputDecoration::new()
                        .label_text(LABEL_TEXT)
                        .hint_text(HINT_TEXT)
                        .hint_fade_duration(ms(120)),
                    None,
                    50,
                    Some(50),
                );
            });
        }

        // input_decorator_test.dart 'hint opacity animation - duration from theme'.
        #[test]
        fn the_hint_fades_for_the_theme_hint_fade_duration() {
            test_widgets(|tester| {
                // The animation's duration is set to 120ms.
                check_hint_fade(
                    tester,
                    InputDecoration::new()
                        .label_text(LABEL_TEXT)
                        .hint_text(HINT_TEXT),
                    Some(InputDecorationThemeData::new().hint_fade_duration(ms(120))),
                    50,
                    Some(50),
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration hint - InputDecorator shows hint
        // widget'.
        #[test]
        fn the_decorator_shows_a_hint_widget() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(InputDecoration::new().hint(Text::new("hint"))),
                );
                assert_eq!(tester.find_text("hint").len(), 1);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration hint - hint style overflow works'.
        #[test]
        fn the_hint_style_overflow_reaches_the_hint_text() {
            test_widgets(|tester| {
                let hint_text = "hint text".repeat(20);
                let hint_style = TextStyle::new()
                    .font_size(14.0)
                    .overflow(TextOverflow::Fade);
                let decoration = InputDecoration::new()
                    .hint_text(hint_text.clone())
                    .hint_style(hint_style);
                pump(tester, BuildInputDecorator::new(decoration.clone()));
                tester.pump_by(K_TRANSITION_DURATION);

                let hint = tester.find_text(&hint_text)[0];
                let overflow = tester.widget(hint, |text: &Text| {
                    text.style.as_ref().and_then(|style| style.overflow)
                });
                assert_eq!(
                    overflow,
                    decoration
                        .hint_style
                        .as_ref()
                        .and_then(|style| style.overflow)
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration hint - Widget height collapses
        // from hint height when maintainHintSize is false'.
        #[test]
        fn the_height_collapses_from_the_hint_height_without_maintain_hint_size() {
            test_widgets(|tester| {
                let decoration = InputDecoration::new()
                    .hint_text("hint".repeat(20))
                    .hint_max_lines(3)
                    .maintain_hint_size(false);
                pump(tester, BuildInputDecorator::new(decoration));
                assert_eq!(get_decorator_rect(tester).height(), 48.0);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration hint - Widget height stays at hint
        // height by default'.
        #[test]
        fn the_height_stays_at_the_hint_height_by_default() {
            test_widgets(|tester| {
                // Dart's hint is 'hint' * 20, which wraps onto two lines at 800: FlutterTest
                // breaks a word too long for its line, which Waft's text layout does not, so
                // the line break is explicit here.
                let hint_text = format!("{}\n{}", "hint".repeat(10), "hint".repeat(10));
                let decoration = InputDecoration::new()
                    .hint_max_lines(3)
                    .hint_text(hint_text.clone());
                pump(tester, BuildInputDecorator::new(decoration));
                let hint_height = text_rect(tester, &hint_text).height();
                let input_height = get_decorator_rect(tester).height();
                assert_eq!(input_height, hint_height + 16.0);
            });
        }

        /// Dart's `find.text(hintText)` count.
        fn hint_count(tester: &WidgetTester) -> usize {
            tester.find_text(HINT_TEXT).len()
        }

        fn fading_hint_decoration() -> InputDecoration {
            InputDecoration::new()
                .hint_text(HINT_TEXT)
                .hint_max_lines(3)
                .hint_fade_duration(ms(120))
                .maintain_hint_size(false)
        }

        // input_decorator_test.dart 'Material3 - InputDecoration hint - hintFadeDuration applies to
        // hint fade-in when maintainHintSize is false'.
        #[test]
        fn the_hint_fade_duration_applies_to_the_fade_in_without_maintain_hint_size() {
            test_widgets(|tester| {
                // Build once with empty content.
                pump(tester, BuildInputDecorator::new(fading_hint_decoration()));

                // Hint is not exist.
                assert_eq!(hint_count(tester), 0);

                // Rebuild with empty content.
                tester.pump_widget(
                    BuildInputDecorator::new(fading_hint_decoration())
                        .is_empty(true)
                        .build(),
                );

                // The hint's opacity animates from 0.0 to 1.0.
                tester.pump_by(ms(50));
                let hint_opacity_50ms = get_hint_opacity(tester);
                assert_in_exclusive_range(hint_opacity_50ms, 0.0, 1.0);
                tester.pump_by(ms(50));
                let hint_opacity_100ms = get_hint_opacity(tester);
                assert_in_exclusive_range(hint_opacity_100ms, hint_opacity_50ms, 1.0);
                tester.pump_by(ms(20));
                assert_eq!(get_hint_opacity(tester), 1.0);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration hint - hintFadeDuration applies to
        // hint fade-out when maintainHintSize is false'.
        #[test]
        fn the_hint_fade_duration_applies_to_the_fade_out_without_maintain_hint_size() {
            test_widgets(|tester| {
                // Build once with empty content.
                pump(
                    tester,
                    BuildInputDecorator::new(fading_hint_decoration()).is_empty(true),
                );

                // Hint is visible (opacity 1.0).
                assert_eq!(get_hint_opacity(tester), 1.0);

                // Rebuild with non-empty content.
                tester.pump_widget(BuildInputDecorator::new(fading_hint_decoration()).build());

                // The hint's opacity animates from 1.0 to 0.0.
                tester.pump_by(ms(50));
                let hint_opacity_50ms = get_hint_opacity(tester);
                assert_in_exclusive_range(hint_opacity_50ms, 0.0, 1.0);
                tester.pump_by(ms(50));
                let hint_opacity_100ms = get_hint_opacity(tester);
                assert_in_exclusive_range(hint_opacity_100ms, 0.0, hint_opacity_50ms);
                tester.pump_by(ms(20));
                assert_eq!(get_hint_opacity(tester), 0.0);
                tester.pump_by(ms(1));
                // The hintText replaced with SizeBox.
                assert_eq!(hint_count(tester), 0);
            });
        }

        fn hint_edge(
            tester: &mut WidgetTester,
            maintain_hint_size: bool,
            text_direction: TextDirection,
        ) -> Rect {
            pump(
                tester,
                BuildInputDecorator::new(
                    InputDecoration::new()
                        .hint_text(HINT_TEXT)
                        .maintain_hint_size(maintain_hint_size),
                )
                .is_empty(true)
                .text_direction(text_direction),
            );
            get_hint_rect(tester)
        }

        // input_decorator_test.dart 'Material3 - InputDecoration hint - Hint does not change
        // position when maintainHintSize is false - LTR'.
        #[test]
        fn the_hint_keeps_its_position_without_maintain_hint_size_in_ltr() {
            test_widgets(|tester| {
                let expected_hint_left = hint_edge(tester, true, TextDirection::Ltr).left;
                assert_eq!(
                    hint_edge(tester, false, TextDirection::Ltr).left,
                    expected_hint_left
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration hint - Hint does not change
        // position when maintainHintSize is false - RTL'.
        #[test]
        fn the_hint_keeps_its_position_without_maintain_hint_size_in_rtl() {
            test_widgets(|tester| {
                let expected_hint_right = hint_edge(tester, true, TextDirection::Rtl).right;
                assert_eq!(
                    hint_edge(tester, false, TextDirection::Rtl).right,
                    expected_hint_right
                );
            });
        }
    }

    // input_decorator_test.dart's helper/counter/error,
    // constraints, prefix/suffix, collapsed, floatingLabelAlignment and isDense groups.
    mod supporting_and_affixes {
        #![allow(deprecated)]
        use waft_widgets::testing::{
            TargetPlatformVariant, WidgetTester, test_widgets, test_widgets_variant,
        };

        use std::cell::Cell;
        use std::rc::Rc;
        use std::time::Duration;

        use waft_embedder::{Color, Rect, Size};
        use waft_foundation::Listener;
        use waft_painting::{EdgeInsetsGeometry, TextDirection, TextStyle};
        use waft_rendering::BoxConstraints;
        use waft_widgets::{
            AnimatedOpacity, Center, FadeTransition, Focus, GestureDetector, Icon, IconData,
            IntoWidget, RichText, SizedBox, Text, WidgetRef, downcast_widget,
        };

        use super::super::*;
        use super::common::K_TRANSITION_DURATION;
        use super::common::*;
        use crate::icons::Icons;
        use crate::input_border::OutlineInputBorder;
        use crate::theme::Theme;
        use crate::theme_data::ThemeData;

        // Overall height for InputDecorator (filled or outlined) is 76dp on mobile:
        //    8 - top padding
        //   12 - floating label (font size = 16 * 0.75, line height is forced to 1.0)
        //    4 - gap between label and input
        //   24 - input text (font size = 16, line height = 1.5)
        //    8 - bottom padding
        //    4 - gap above helper/error/counter
        //   16 - helper/counter (font size = 12, line height is 1.5)
        const TOP_PADDING: f64 = 8.0;
        const FLOATING_LABEL_HEIGHT: f64 = 12.0;
        const LABEL_INPUT_GAP: f64 = 4.0;
        const INPUT_HEIGHT: f64 = 24.0;
        const BOTTOM_PADDING: f64 = 8.0;
        const HELPER_GAP: f64 = 4.0;
        const HELPER_HEIGHT: f64 = 16.0;
        const CONTAINER_HEIGHT: f64 =
            TOP_PADDING + FLOATING_LABEL_HEIGHT + LABEL_INPUT_GAP + INPUT_HEIGHT + BOTTOM_PADDING; // 56.0
        const FULL_HEIGHT: f64 = CONTAINER_HEIGHT + HELPER_GAP + HELPER_HEIGHT; // 76.0
        const ERROR_HEIGHT: f64 = HELPER_HEIGHT;
        const HINT_HEIGHT: f64 = INPUT_HEIGHT;
        const HELPER_START_PADDING: f64 = 16.0;
        const COUNTER_END_PADDING: f64 = 16.0;

        /// A tester with the Material icon font, whose glyphs the icon cases measure.
        fn pump(tester: &mut WidgetTester, decorator: BuildInputDecorator) {
            tester.pump_widget(decorator.build());
        }

        /// Dart's `Theme.of(tester.element(findDecorator()))`.
        fn decorator_theme(tester: &WidgetTester) -> Rc<ThemeData> {
            let element = tester.find_by_type::<InputDecorator>()[0];
            let mut app = tester.cell.borrow_mut();
            Theme::of(&mut app, element)
        }

        /// Dart's `tester.widget<RichText>(find.descendant(of: find.text(text), matching:
        /// find.byType(RichText))).text.style!`.
        fn text_style(tester: &WidgetTester, text: &str) -> TextStyle {
            let found = tester.find_text(text);
            assert_eq!(found.len(), 1, "one {text:?}");
            let rich_text = tester.descendants_of_type::<RichText>(found[0])[0];
            tester.widget(rich_text, |rich_text: &RichText| {
                rich_text.text.style().cloned().expect("a styled text")
            })
        }

        /// Dart's `tester.getRect(find.text(text))` over a `Text`.
        fn string_rect(tester: &WidgetTester, text: &str) -> Rect {
            let found = tester.find_text(text);
            assert_eq!(found.len(), 1, "one {text:?}");
            rect_of(tester, found[0])
        }

        fn body_small_with(theme: &ThemeData, color: Color) -> TextStyle {
            theme
                .text_theme
                .body_small
                .clone()
                .expect("bodySmall")
                .copy_with()
                .color(color)
        }

        fn filled() -> InputDecoration {
            InputDecoration::new().filled(true)
        }

        fn outlined() -> InputDecoration {
            InputDecoration::new().border(OutlineInputBorder::new())
        }

        /// The decoration of the 'Helper and counter are correctly positioned' cases, with
        /// the label, helper and counter texts.
        fn measured_supporting(base: InputDecoration) -> InputDecoration {
            base.label_text(LABEL_TEXT)
                .helper_text(HELPER_TEXT)
                .counter_text(COUNTER_TEXT)
        }

        /// The decoration of the 'Helper and counter are correctly styled' cases.
        fn styled_supporting(base: InputDecoration) -> InputDecoration {
            base.label_text(LABEL_TEXT)
                .helper_text(HELPER_TEXT)
                .counter_text(COUNTER_TEXT)
        }

        fn check_helper_and_counter_positions(tester: &WidgetTester) {
            assert_eq!(get_decorator_rect(tester).height(), FULL_HEIGHT);
            assert_eq!(get_border_bottom(tester), CONTAINER_HEIGHT);
            let helper = text_rect(tester, HELPER_TEXT);
            assert_eq!(helper.top, CONTAINER_HEIGHT + HELPER_GAP);
            assert_eq!(helper.height(), HELPER_HEIGHT);
            assert_eq!(helper.left, HELPER_START_PADDING);
            let counter = text_rect(tester, COUNTER_TEXT);
            assert_eq!(counter.top, CONTAINER_HEIGHT + HELPER_GAP);
            assert_eq!(counter.height(), HELPER_HEIGHT);
            // The global transform is f32, which leaves the counter's right a rounding off 784.
            assert_close(counter.right, 800.0 - COUNTER_END_PADDING, 1e-4);
        }

        fn check_error_and_counter_positions(tester: &WidgetTester) {
            assert_eq!(get_decorator_rect(tester).height(), FULL_HEIGHT);
            assert_eq!(get_border_bottom(tester), CONTAINER_HEIGHT);
            let error = text_rect(tester, ERROR_TEXT);
            assert_eq!(error.top, CONTAINER_HEIGHT + HELPER_GAP);
            assert_eq!(error.height(), ERROR_HEIGHT);
            assert_eq!(error.left, HELPER_START_PADDING);
            let counter = text_rect(tester, COUNTER_TEXT);
            assert_eq!(counter.top, CONTAINER_HEIGHT + HELPER_GAP);
            assert_eq!(counter.height(), ERROR_HEIGHT);
            // The global transform is f32, which leaves the counter's right a rounding off 784.
            assert_close(counter.right, 800.0 - COUNTER_END_PADDING, 1e-4);
        }

        fn check_helper_and_counter_style(
            tester: &WidgetTester,
            color: impl Fn(&ThemeData) -> Color,
        ) {
            let theme = decorator_theme(tester);
            let expected_style = body_small_with(&theme, color(&theme));
            assert_eq!(text_style(tester, HELPER_TEXT), expected_style);
            assert_eq!(text_style(tester, COUNTER_TEXT), expected_style);
        }

        fn check_error_and_counter_style(tester: &WidgetTester) {
            let theme = decorator_theme(tester);
            assert_eq!(
                text_style(tester, ERROR_TEXT),
                body_small_with(&theme, theme.color_scheme.error)
            );
            assert_eq!(
                text_style(tester, COUNTER_TEXT),
                body_small_with(&theme, theme.color_scheme.on_surface_variant())
            );
        }

        fn on_surface_variant(theme: &ThemeData) -> Color {
            theme.color_scheme.on_surface_variant()
        }

        fn disabled_on_surface(theme: &ThemeData) -> Color {
            theme.color_scheme.on_surface.with_opacity(0.38)
        }

        // input_decorator_test.dart 'Material3 - InputDecoration helper/counter/error - for filled
        // text field - when field is enabled - Helper and counter are correctly positioned'.
        #[test]
        fn filled_enabled_helper_and_counter_are_correctly_positioned() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(measured_supporting(filled())),
                );
                check_helper_and_counter_positions(tester);
            });
        }

        // input_decorator_test.dart '... for filled text field - when field is enabled - Helper
        // and counter are correctly styled'.
        #[test]
        fn filled_enabled_helper_and_counter_are_correctly_styled() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(styled_supporting(filled())),
                );
                check_helper_and_counter_style(tester, on_surface_variant);
            });
        }

        // input_decorator_test.dart '... for filled text field - when field is disabled - Helper
        // and counter are correctly positioned'.
        #[test]
        fn filled_disabled_helper_and_counter_are_correctly_positioned() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(measured_supporting(filled().enabled(false))),
                );
                check_helper_and_counter_positions(tester);
            });
        }

        // input_decorator_test.dart '... for filled text field - when field is disabled - Helper
        // and counter are correctly styled'.
        #[test]
        fn filled_disabled_helper_and_counter_are_correctly_styled() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(styled_supporting(filled().enabled(false))),
                );
                check_helper_and_counter_style(tester, disabled_on_surface);
            });
        }

        // input_decorator_test.dart '... for filled text field - when field is hovered - Helper
        // and counter are correctly positioned'.
        #[test]
        fn filled_hovered_helper_and_counter_are_correctly_positioned() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(measured_supporting(filled())).is_hovering(true),
                );
                check_helper_and_counter_positions(tester);
            });
        }

        // input_decorator_test.dart '... for filled text field - when field is hovered - Helper
        // and counter are correctly styled'.
        #[test]
        fn filled_hovered_helper_and_counter_are_correctly_styled() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(styled_supporting(filled())).is_hovering(true),
                );
                check_helper_and_counter_style(tester, on_surface_variant);
            });
        }

        // input_decorator_test.dart '... for filled text field - when field is focused - Helper
        // and counter are correctly positioned'.
        #[test]
        fn filled_focused_helper_and_counter_are_correctly_positioned() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(measured_supporting(filled())).is_focused(true),
                );
                check_helper_and_counter_positions(tester);
            });
        }

        // input_decorator_test.dart '... for filled text field - when field is focused - Helper
        // and counter are correctly styled'.
        #[test]
        fn filled_focused_helper_and_counter_are_correctly_styled() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(styled_supporting(filled())).is_focused(true),
                );
                check_helper_and_counter_style(tester, on_surface_variant);
            });
        }

        fn check_error_shows_and_helper_hides(tester: &mut WidgetTester, base: InputDecoration) {
            pump(
                tester,
                BuildInputDecorator::new(styled_supporting(base).error_text(ERROR_TEXT))
                    .is_focused(true),
            );
            assert_eq!(tester.find_text(ERROR_TEXT).len(), 1);
            assert_eq!(tester.find_text(COUNTER_TEXT).len(), 1);
            assert!(tester.find_text(HELPER_TEXT).is_empty());
        }

        fn check_error_positions(tester: &mut WidgetTester, base: InputDecoration) {
            pump(
                tester,
                BuildInputDecorator::new(measured_supporting(base).error_text(ERROR_TEXT))
                    .is_focused(true),
            );
            check_error_and_counter_positions(tester);
        }

        fn check_error_style(tester: &mut WidgetTester, base: InputDecoration) {
            pump(
                tester,
                BuildInputDecorator::new(styled_supporting(base).error_text(ERROR_TEXT))
                    .is_focused(true),
            );
            check_error_and_counter_style(tester);
        }

        // input_decorator_test.dart '... for filled text field - when field is in error - Error and
        // counter are visible, helper is not visible'.
        #[test]
        fn filled_error_and_counter_are_visible_and_the_helper_is_not() {
            test_widgets(|tester| check_error_shows_and_helper_hides(tester, filled()));
        }

        // input_decorator_test.dart '... for filled text field - when field is in error - Error and
        // counter are correctly positioned'.
        #[test]
        fn filled_error_and_counter_are_correctly_positioned() {
            test_widgets(|tester| check_error_positions(tester, filled()));
        }

        // input_decorator_test.dart '... for filled text field - when field is in error - Error and
        // counter are correctly styled'.
        #[test]
        fn filled_error_and_counter_are_correctly_styled() {
            test_widgets(|tester| check_error_style(tester, filled()));
        }

        // input_decorator_test.dart '... for outlined text field - when field is enabled - Helper
        // and counter are correctly positioned'.
        #[test]
        fn outlined_enabled_helper_and_counter_are_correctly_positioned() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(measured_supporting(outlined())),
                );
                check_helper_and_counter_positions(tester);
            });
        }

        // input_decorator_test.dart '... for outlined text field - when field is enabled - Helper
        // and counter are correctly styled'.
        #[test]
        fn outlined_enabled_helper_and_counter_are_correctly_styled() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(styled_supporting(outlined())),
                );
                check_helper_and_counter_style(tester, on_surface_variant);
            });
        }

        // input_decorator_test.dart '... for outlined text field - when field is disabled - Helper
        // and counter are correctly positioned'.
        #[test]
        fn outlined_disabled_helper_and_counter_are_correctly_positioned() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(measured_supporting(outlined().enabled(false))),
                );
                check_helper_and_counter_positions(tester);
            });
        }

        // input_decorator_test.dart '... for outlined text field - when field is disabled - Helper
        // and counter are correctly styled'.
        #[test]
        fn outlined_disabled_helper_and_counter_are_correctly_styled() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(styled_supporting(outlined().enabled(false))),
                );
                check_helper_and_counter_style(tester, disabled_on_surface);
            });
        }

        // input_decorator_test.dart '... for outlined text field - when field is hovered - Helper
        // and counter are correctly positioned'.
        #[test]
        fn outlined_hovered_helper_and_counter_are_correctly_positioned() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(measured_supporting(outlined())).is_hovering(true),
                );
                check_helper_and_counter_positions(tester);
            });
        }

        // input_decorator_test.dart '... for outlined text field - when field is hovered - Helper
        // and counter are correctly styled'.
        #[test]
        fn outlined_hovered_helper_and_counter_are_correctly_styled() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(styled_supporting(outlined())).is_hovering(true),
                );
                check_helper_and_counter_style(tester, on_surface_variant);
            });
        }

        // input_decorator_test.dart '... for outlined text field - when field is focused - Helper
        // and counter are correctly positioned'.
        #[test]
        fn outlined_focused_helper_and_counter_are_correctly_positioned() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(measured_supporting(outlined())).is_focused(true),
                );
                check_helper_and_counter_positions(tester);
            });
        }

        // input_decorator_test.dart '... for outlined text field - when field is focused - Helper
        // and counter are correctly styled'.
        #[test]
        fn outlined_focused_helper_and_counter_are_correctly_styled() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(styled_supporting(outlined())).is_focused(true),
                );
                check_helper_and_counter_style(tester, on_surface_variant);
            });
        }

        // input_decorator_test.dart '... for outlined text field - when field is in error - Error
        // and counter are visible, helper is not visible'.
        #[test]
        fn outlined_error_and_counter_are_visible_and_the_helper_is_not() {
            test_widgets(|tester| check_error_shows_and_helper_hides(tester, outlined()));
        }

        // input_decorator_test.dart '... for outlined text field - when field is in error - Error
        // and counter are correctly positioned'.
        #[test]
        fn outlined_error_and_counter_are_correctly_positioned() {
            test_widgets(|tester| check_error_positions(tester, outlined()));
        }

        // input_decorator_test.dart '... for outlined text field - when field is in error - Error
        // and counter are correctly styled'.
        #[test]
        fn outlined_error_and_counter_are_correctly_styled() {
            test_widgets(|tester| check_error_style(tester, outlined()));
        }

        fn assert_close(actual: f64, expected: f64, tolerance: f64) {
            assert!(
                (actual - expected).abs() <= tolerance,
                "{actual} is not within {tolerance} of {expected}"
            );
        }

        /// The error or helper `text`, `lines` lines high, and the decorator below it.
        fn check_supporting_lines(
            tester: &mut WidgetTester,
            decoration: InputDecoration,
            text: &str,
            lines: f64,
        ) {
            pump(tester, BuildInputDecorator::new(decoration));
            assert_close(
                string_rect(tester, text).height(),
                ERROR_HEIGHT * lines,
                0.25,
            );
            assert_close(
                get_decorator_rect(tester).height(),
                CONTAINER_HEIGHT + HELPER_GAP + ERROR_HEIGHT * lines,
                0.25,
            );
        }

        // input_decorator_test.dart 'Multiline error/helper - Error height grows to accommodate
        // error text'.
        #[test]
        fn the_error_grows_to_accommodate_its_text() {
            test_widgets(|tester| {
                check_supporting_lines(
                    tester,
                    filled()
                        .label_text("label")
                        .error_text(THREE_LINES)
                        .error_max_lines(3),
                    THREE_LINES,
                    3.0,
                );
            });
        }

        // input_decorator_test.dart 'Multiline error/helper - Error height is correct when
        // errorMaxLines is restricted'.
        #[test]
        fn the_error_height_is_correct_when_error_max_lines_is_restricted() {
            test_widgets(|tester| {
                check_supporting_lines(
                    tester,
                    filled()
                        .label_text("label")
                        .error_text(THREE_LINES)
                        .error_max_lines(2),
                    THREE_LINES,
                    2.0,
                );
            });
        }

        // input_decorator_test.dart 'Multiline error/helper - Error height is correct when
        // errorMaxLines is bigger than the number of lines in errorText'.
        #[test]
        fn the_error_height_is_its_line_count_when_error_max_lines_is_bigger() {
            test_widgets(|tester| {
                check_supporting_lines(
                    tester,
                    filled()
                        .label_text("label")
                        .error_text(TWO_LINES)
                        .error_max_lines(3),
                    TWO_LINES,
                    2.0,
                );
            });
        }

        // input_decorator_test.dart 'Multiline error/helper - Error height is not limited by
        // default'.
        #[test]
        fn the_error_height_is_not_limited_by_default() {
            test_widgets(|tester| {
                check_supporting_lines(
                    tester,
                    filled().label_text("label").error_text(THREE_LINES),
                    THREE_LINES,
                    3.0,
                );
            });
        }

        // input_decorator_test.dart 'Multiline error/helper - InputDecorationThemeData.hintMaxLines
        // behaves as default value'.
        #[test]
        fn the_theme_hint_max_lines_is_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(InputDecoration::new().hint_text(THREE_LINES))
                        .input_decoration_theme(InputDecorationThemeData::new().hint_max_lines(2)),
                );
                assert_close(
                    string_rect(tester, THREE_LINES).height(),
                    HINT_HEIGHT * 2.0,
                    0.25,
                );
                assert_close(
                    get_decorator_rect(tester).height(),
                    TOP_PADDING + HINT_HEIGHT * 2.0 + BOTTOM_PADDING,
                    0.25,
                );
            });
        }

        // input_decorator_test.dart 'Multiline error/helper - InputDecoration hintMaxLines default
        // expands with hintText'.
        #[test]
        fn the_hint_expands_with_its_text_by_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(InputDecoration::new().hint_text(THREE_LINES))
                        .input_decoration_theme(InputDecorationThemeData::new()),
                );
                assert_eq!(
                    get_decorator_rect(tester).height(),
                    TOP_PADDING + HINT_HEIGHT * 3.0 + BOTTOM_PADDING
                );
            });
        }

        // input_decorator_test.dart 'Multiline error/helper - Helper height grows to accommodate
        // helper text'.
        #[test]
        fn the_helper_grows_to_accommodate_its_text() {
            test_widgets(|tester| {
                check_supporting_lines(
                    tester,
                    filled()
                        .label_text("label")
                        .helper_text(THREE_LINES)
                        .helper_max_lines(3),
                    THREE_LINES,
                    3.0,
                );
            });
        }

        // input_decorator_test.dart 'Multiline error/helper - Helper height is correct when
        // maxLines is restricted'.
        #[test]
        fn the_helper_height_is_correct_when_helper_max_lines_is_restricted() {
            test_widgets(|tester| {
                check_supporting_lines(
                    tester,
                    filled()
                        .label_text("label")
                        .helper_text(THREE_LINES)
                        .helper_max_lines(2),
                    THREE_LINES,
                    2.0,
                );
            });
        }

        // input_decorator_test.dart 'Multiline error/helper - Helper height is correct when
        // helperMaxLines is bigger than the number of lines in helperText'.
        #[test]
        fn the_helper_height_is_its_line_count_when_helper_max_lines_is_bigger() {
            test_widgets(|tester| {
                check_supporting_lines(
                    tester,
                    filled()
                        .label_text("label")
                        .helper_text(TWO_LINES)
                        .helper_max_lines(3),
                    TWO_LINES,
                    2.0,
                );
            });
        }

        // input_decorator_test.dart 'Multiline error/helper - Helper height is not limited by
        // default'.
        #[test]
        fn the_helper_height_is_not_limited_by_default() {
            test_widgets(|tester| {
                check_supporting_lines(
                    tester,
                    filled().label_text("label").helper_text(THREE_LINES),
                    THREE_LINES,
                    3.0,
                );
            });
        }

        // input_decorator_test.dart 'Helper widget - InputDecorator shows helper widget'.
        #[test]
        fn the_decorator_shows_a_helper_widget() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .helper(Text::new("helper").style(TextStyle::new().font_size(20.0))),
                    ),
                );
                assert_eq!(tester.find_text("helper").len(), 1);
            });
        }

        // input_decorator_test.dart 'Helper widget - InputDecorator throws when helper text and
        // helper widget are provided'.
        #[test]
        #[should_panic(expected = "Declaring both helper and helperText is not supported.")]
        fn a_helper_text_and_a_helper_widget_together_panic() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .helper_text("helperText")
                            .helper(Text::new("helper").style(TextStyle::new().font_size(20.0))),
                    ),
                );
            });
        }

        // input_decorator_test.dart 'Error widget - InputDecorator shows error widget'.
        #[test]
        fn the_decorator_shows_an_error_widget() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .error(Text::new("error").style(TextStyle::new().font_size(20.0))),
                    ),
                );
                assert_eq!(tester.find_text("error").len(), 1);
            });
        }

        // input_decorator_test.dart 'Error widget - InputDecorator throws when error text and error
        // widget are provided'.
        #[test]
        #[should_panic(expected = "Declaring both error and errorText is not supported.")]
        fn an_error_text_and_an_error_widget_together_panic() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .error_text("errorText")
                            .error(Text::new("error").style(TextStyle::new().font_size(20.0))),
                    ),
                );
            });
        }

        // input_decorator_test.dart 'Error widget - InputDecorator error widget text style defaults
        // to errorStyle'.
        #[test]
        fn an_error_widget_text_style_defaults_to_the_error_style() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(InputDecoration::new().error(Text::new(ERROR_TEXT))),
                );
                assert_eq!(tester.find_text(ERROR_TEXT).len(), 1);
                let theme = decorator_theme(tester);
                assert_eq!(
                    text_style(tester, ERROR_TEXT),
                    body_small_with(&theme, theme.color_scheme.error)
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration helper/counter/error -
        // InputDecorator with counter does not crash when given a 0 size'.
        #[test]
        fn a_decorator_with_a_counter_does_not_crash_at_zero_size() {
            test_widgets(|tester| {
                let decorator = BuildInputDecorator::new(
                    InputDecoration::new()
                        .content_padding(EdgeInsetsGeometry::directional(99.0, 99.0, 99.0, 99.0))
                        .prefix_icon(Focus::new(Icon::new(Some(Icons::search()))))
                        .counter(Text::new("COUNTER")),
                )
                .build();
                tester.pump_widget(
                    Center::new()
                        .child(SizedBox::new().width(0.0).height(0.0).child(decorator))
                        .into_widget(),
                );
                tester.pump_by(K_TRANSITION_DURATION);

                assert_eq!(tester.find_by_type::<InputDecorator>().len(), 1);
                let counter = tester.find_text("COUNTER")[0];
                let render = tester.render_box_of(counter);
                assert_eq!(render.size(&tester.cell.borrow()), Size::ZERO);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration constraints - No InputDecorator
        // constraints'.
        #[test]
        fn without_constraints_the_decorator_fills_the_width_at_the_default_height() {
            test_widgets(|tester| {
                pump(tester, BuildInputDecorator::new(InputDecoration::new()));
                // Should fill the screen width and be default height.
                assert_eq!(
                    get_decorator_rect(tester).size(),
                    Size::new(800.0, crate::constants::K_MIN_INTERACTIVE_DIMENSION)
                );
            });
        }

        fn theme_with_constraints(app: &App) -> ThemeData {
            ThemeData::builder()
                .input_decoration_theme(InputDecorationThemeData::new().constraints(
                    BoxConstraints {
                        max_width: 300.0,
                        max_height: 40.0,
                        ..BoxConstraints::new()
                    },
                ))
                .build(app)
        }

        // input_decorator_test.dart 'Material3 - InputDecoration constraints -
        // InputDecoratorThemeData constraints'.
        #[test]
        fn the_theme_constraints_apply() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(InputDecoration::new()).theme(theme_with_constraints),
                );
                // Theme settings should make it 300x40 pixels.
                assert_eq!(get_decorator_rect(tester).size(), Size::new(300.0, 40.0));
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration constraints - InputDecorator
        // constraints'.
        #[test]
        fn the_decoration_constraints_override_the_theme() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(InputDecoration::new().constraints(BoxConstraints {
                        max_width: 200.0,
                        max_height: 32.0,
                        ..BoxConstraints::new()
                    }))
                    .theme(theme_with_constraints),
                );
                // InputDecoration.constraints should override the theme. It should be
                // only 200x32 pixels.
                assert_eq!(get_decorator_rect(tester).size(), Size::new(200.0, 32.0));
            });
        }

        fn prefix_icon() -> IconData {
            Icons::search()
        }

        fn suffix_icon() -> IconData {
            Icons::cancel_outlined()
        }

        /// Dart's `getPrefixIconRect` / `getSuffixIconRect`: the rect of the `Icon`.
        fn icon_rect(tester: &WidgetTester, icon: &IconData) -> Rect {
            rect_of(tester, tester.find_by_icon(icon))
        }

        /// Dart's `tester.getRect(findPrefixIconInnerRichText())`.
        fn icon_inner_rect(tester: &WidgetTester, icon: &IconData) -> Rect {
            let rich_text = tester.descendants_of_type::<RichText>(tester.find_by_icon(icon))[0];
            rect_of(tester, rich_text)
        }

        /// Dart's `getPrefixIconStyle` / `getSuffixIconStyle`.
        fn icon_style(tester: &WidgetTester, icon: &IconData) -> TextStyle {
            tester.glyph_style(tester.find_by_icon(icon))
        }

        fn input_rect(tester: &WidgetTester) -> Rect {
            text_rect(tester, INPUT_TEXT)
        }

        /// The checks every 'prefixIcon is correctly positioned' case makes, in `text_direction`.
        fn check_prefix_icon_position(tester: &WidgetTester, text_direction: TextDirection) {
            let icon = prefix_icon();
            // By default, the prefix icon is rendered inside a 48x48 constrained box.
            assert_eq!(icon_rect(tester, &icon).size(), Size::new(48.0, 48.0));
            // The icon size is 24 per Material 3 spec.
            assert_eq!(icon_style(tester, &icon).font_size, Some(24.0));
            // Prefix icon is vertically centered inside the container.
            assert_eq!(
                icon_rect(tester, &icon).center().dy(),
                get_container_rect(tester).center().dy()
            );
            let inner = icon_inner_rect(tester, &icon);
            match text_direction {
                TextDirection::Ltr => {
                    // Left padding is 12 per Material 3 spec.
                    assert_eq!(inner.left, 12.0);
                    // The gap between the icon and the input should be 16 based on M3 specification.
                    assert_eq!(input_rect(tester).left - inner.right, 16.0);
                }
                TextDirection::Rtl => {
                    // Right padding is 12 per Material 3 spec.
                    assert_eq!(get_decorator_rect(tester).right - inner.right, 12.0);
                    assert_eq!(inner.left - input_rect(tester).right, 16.0);
                }
            }
        }

        /// The checks every 'suffixIcon is correctly positioned' case makes, in `text_direction`.
        fn check_suffix_icon_position(tester: &WidgetTester, text_direction: TextDirection) {
            let icon = suffix_icon();
            // By default, the suffix icon is rendered inside a 48x48 constrained box.
            assert_eq!(icon_rect(tester, &icon).size(), Size::new(48.0, 48.0));
            // The icon size is 24 per Material 3 spec.
            assert_eq!(icon_style(tester, &icon).font_size, Some(24.0));
            // Suffix icon is vertically centered inside the container.
            assert_eq!(
                icon_rect(tester, &icon).center().dy(),
                get_container_rect(tester).center().dy()
            );
            let inner = icon_inner_rect(tester, &icon);
            match text_direction {
                TextDirection::Ltr => {
                    // Right padding is 12 per Material 3 spec.
                    assert_eq!(get_decorator_rect(tester).right - inner.right, 12.0);
                    // The gap between the icon and the input should be 16 based on M3 specification.
                    assert_eq!(inner.left - input_rect(tester).right, 16.0);
                }
                TextDirection::Rtl => {
                    // Left padding is 12 per Material 3 spec.
                    assert_eq!(inner.left, 12.0);
                    assert_eq!(input_rect(tester).left - inner.right, 16.0);
                }
            }
        }

        fn prefixed(base: InputDecoration) -> InputDecoration {
            base.label_text(LABEL_TEXT)
                .prefix_icon(Icon::new(Some(prefix_icon())))
        }

        fn suffixed(base: InputDecoration) -> InputDecoration {
            base.label_text(LABEL_TEXT)
                .suffix_icon(Icon::new(Some(suffix_icon())))
        }

        fn check_icon_color(
            tester: &mut WidgetTester,
            decorator: BuildInputDecorator,
            icon: IconData,
            color: impl Fn(&ThemeData) -> Color,
        ) {
            pump(tester, decorator);
            let theme = decorator_theme(tester);
            assert_eq!(
                icon_style(tester, &icon).color.map(|color| *color),
                Some(color(&theme))
            );
        }

        fn error(theme: &ThemeData) -> Color {
            theme.color_scheme.error
        }

        fn on_error_container(theme: &ThemeData) -> Color {
            theme.color_scheme.on_error_container()
        }

        /// One positioned and one colour case of an icon for each of Dart's prefix/suffix groups.
        macro_rules! affix_icon_cases {
        ($(
            $(#[$doc:meta])*
            $position:ident: $decorate:ident, $base:expr, |$builder:ident| $configure:expr,
                $check:ident, $direction:expr;
        )*) => {
            $(
                $(#[$doc])*
                #[test]
                fn $position() {
                    test_widgets(|tester| {
                        let $builder = BuildInputDecorator::new($decorate($base));
                        pump(tester, $configure);
                        $check(tester, $direction);
                    });
                }
            )*
        };
    }

        affix_icon_cases! {
            // input_decorator_test.dart 'Material3 - InputDecoration prefix/suffix - for filled text
            // field - when field is enabled - prefixIcon is correctly positioned - LTR'.
            filled_enabled_prefix_icon_is_correctly_positioned_ltr: prefixed, filled(),
                |b| b, check_prefix_icon_position, TextDirection::Ltr;
            // '... - when field is enabled - prefixIcon is correctly positioned - RTL'.
            filled_enabled_prefix_icon_is_correctly_positioned_rtl: prefixed, filled(),
                |b| b.text_direction(TextDirection::Rtl), check_prefix_icon_position,
                TextDirection::Rtl;
            // '... - when field is enabled - suffixIcon is correctly positioned - LTR'.
            filled_enabled_suffix_icon_is_correctly_positioned_ltr: suffixed, filled(),
                |b| b, check_suffix_icon_position, TextDirection::Ltr;
            // '... - when field is enabled - suffixIcon is correctly positioned - RTL'.
            filled_enabled_suffix_icon_is_correctly_positioned_rtl: suffixed, filled(),
                |b| b.text_direction(TextDirection::Rtl), check_suffix_icon_position,
                TextDirection::Rtl;
            // '... for filled text field - when field is disabled - prefixIcon is correctly
            // positioned'.
            filled_disabled_prefix_icon_is_correctly_positioned: prefixed, filled().enabled(false),
                |b| b, check_prefix_icon_position, TextDirection::Ltr;
            // '... for filled text field - when field is disabled - suffixIcon is correctly
            // positioned'.
            filled_disabled_suffix_icon_is_correctly_positioned: suffixed, filled().enabled(false),
                |b| b, check_suffix_icon_position, TextDirection::Ltr;
            // '... for filled text field - when field is hovered - prefixIcon is correctly
            // positioned'.
            filled_hovered_prefix_icon_is_correctly_positioned: prefixed, filled(),
                |b| b.is_hovering(true), check_prefix_icon_position, TextDirection::Ltr;
            // '... for filled text field - when field is hovered - suffixIcon is correctly
            // positioned'.
            filled_hovered_suffix_icon_is_correctly_positioned: suffixed, filled(),
                |b| b.is_hovering(true), check_suffix_icon_position, TextDirection::Ltr;
            // '... for filled text field - when field is focused - prefixIcon is correctly
            // positioned'.
            filled_focused_prefix_icon_is_correctly_positioned: prefixed, filled(),
                |b| b.is_focused(true), check_prefix_icon_position, TextDirection::Ltr;
            // '... for filled text field - when field is focused - suffixIcon is correctly
            // positioned'.
            filled_focused_suffix_icon_is_correctly_positioned: suffixed, filled(),
                |b| b.is_focused(true), check_suffix_icon_position, TextDirection::Ltr;
            // '... for filled text field - when field is in error - prefixIcon is correctly
            // positioned'.
            filled_error_prefix_icon_is_correctly_positioned: prefixed, filled().error_text(ERROR_TEXT),
                |b| b, check_prefix_icon_position, TextDirection::Ltr;
            // '... for filled text field - when field is in error - suffixIcon is correctly
            // positioned'.
            filled_error_suffix_icon_is_correctly_positioned: suffixed, filled().error_text(ERROR_TEXT),
                |b| b, check_suffix_icon_position, TextDirection::Ltr;
            // '... for outlined text field - when field is enabled - prefixIcon is correctly
            // positioned'.
            outlined_enabled_prefix_icon_is_correctly_positioned: prefixed, outlined(),
                |b| b, check_prefix_icon_position, TextDirection::Ltr;
            // '... for outlined text field - when field is enabled - suffixIcon is correctly
            // positioned'.
            outlined_enabled_suffix_icon_is_correctly_positioned: suffixed, outlined(),
                |b| b, check_suffix_icon_position, TextDirection::Ltr;
            // '... for outlined text field - when field is disabled - prefixIcon is correctly
            // positioned'.
            outlined_disabled_prefix_icon_is_correctly_positioned: prefixed, outlined().enabled(false),
                |b| b, check_prefix_icon_position, TextDirection::Ltr;
            // '... for outlined text field - when field is disabled - suffixIcon is correctly
            // positioned'.
            outlined_disabled_suffix_icon_is_correctly_positioned: suffixed, outlined().enabled(false),
                |b| b, check_suffix_icon_position, TextDirection::Ltr;
            // '... for outlined text field - when field is hovered - prefixIcon is correctly
            // positioned'.
            outlined_hovered_prefix_icon_is_correctly_positioned: prefixed, outlined(),
                |b| b.is_hovering(true), check_prefix_icon_position, TextDirection::Ltr;
            // '... for outlined text field - when field is hovered - suffixIcon is correctly
            // positioned'.
            outlined_hovered_suffix_icon_is_correctly_positioned: suffixed, outlined(),
                |b| b.is_hovering(true), check_suffix_icon_position, TextDirection::Ltr;
            // '... for outlined text field - when field is focused - prefixIcon is correctly
            // positioned'.
            outlined_focused_prefix_icon_is_correctly_positioned: prefixed, outlined(),
                |b| b.is_focused(true), check_prefix_icon_position, TextDirection::Ltr;
            // '... for outlined text field - when field is focused - suffixIcon is correctly
            // positioned'.
            outlined_focused_suffix_icon_is_correctly_positioned: suffixed, outlined(),
                |b| b.is_focused(true), check_suffix_icon_position, TextDirection::Ltr;
            // '... for outlined text field - when field is in error - prefixIcon is correctly
            // positioned'.
            outlined_error_prefix_icon_is_correctly_positioned: prefixed,
                outlined().error_text(ERROR_TEXT), |b| b, check_prefix_icon_position,
                TextDirection::Ltr;
            // '... for outlined text field - when field is in error - suffixIcon is correctly
            // positioned'.
            outlined_error_suffix_icon_is_correctly_positioned: suffixed,
                outlined().error_text(ERROR_TEXT), |b| b, check_suffix_icon_position,
                TextDirection::Ltr;
        }

        /// One 'has correct color' case of Dart's prefix/suffix groups.
        macro_rules! affix_icon_color_cases {
        ($(
            $name:ident: $decorate:ident, $base:expr, |$builder:ident| $configure:expr,
                $icon:expr, $color:expr;
        )*) => {
            $(
                #[test]
                fn $name() {
                    test_widgets(|tester| {
                        let $builder = BuildInputDecorator::new($decorate($base));
                        check_icon_color(tester, $configure, $icon, $color);
                    });
                }
            )*
        };
    }

        // input_decorator_test.dart 'Material3 - InputDecoration prefix/suffix - for filled text
        // field' and 'for outlined text field', each state's 'prefixIcon has correct color',
        // 'suffixIcon has correct color' and, in error, '... has correct color when hovered'.
        affix_icon_color_cases! {
            filled_enabled_prefix_icon_has_correct_color: prefixed, filled(), |b| b,
                prefix_icon(), on_surface_variant;
            filled_enabled_suffix_icon_has_correct_color: suffixed, filled(), |b| b,
                suffix_icon(), on_surface_variant;
            filled_disabled_prefix_icon_has_correct_color: prefixed, filled().enabled(false), |b| b,
                prefix_icon(), disabled_on_surface;
            filled_disabled_suffix_icon_has_correct_color: suffixed, filled().enabled(false), |b| b,
                suffix_icon(), disabled_on_surface;
            filled_hovered_prefix_icon_has_correct_color: prefixed, filled(), |b| b.is_hovering(true),
                prefix_icon(), on_surface_variant;
            filled_hovered_suffix_icon_has_correct_color: suffixed, filled(), |b| b.is_hovering(true),
                suffix_icon(), on_surface_variant;
            filled_focused_prefix_icon_has_correct_color: prefixed, filled(), |b| b.is_focused(true),
                prefix_icon(), on_surface_variant;
            filled_focused_suffix_icon_has_correct_color: suffixed, filled(), |b| b.is_focused(true),
                suffix_icon(), on_surface_variant;
            filled_error_prefix_icon_has_correct_color: prefixed, filled().error_text(ERROR_TEXT),
                |b| b, prefix_icon(), on_surface_variant;
            filled_error_prefix_icon_has_correct_color_when_hovered: prefixed,
                filled().error_text(ERROR_TEXT), |b| b.is_hovering(true), prefix_icon(),
                on_surface_variant;
            filled_error_suffix_icon_has_correct_color: suffixed, filled().error_text(ERROR_TEXT),
                |b| b, suffix_icon(), error;
            filled_error_suffix_icon_has_correct_color_when_hovered: suffixed,
                filled().error_text(ERROR_TEXT), |b| b.is_hovering(true), suffix_icon(),
                on_error_container;
            outlined_enabled_prefix_icon_has_correct_color: prefixed, outlined(), |b| b,
                prefix_icon(), on_surface_variant;
            outlined_enabled_suffix_icon_has_correct_color: suffixed, outlined(), |b| b,
                suffix_icon(), on_surface_variant;
            outlined_disabled_prefix_icon_has_correct_color: prefixed, outlined().enabled(false),
                |b| b, prefix_icon(), disabled_on_surface;
            outlined_disabled_suffix_icon_has_correct_color: suffixed, outlined().enabled(false),
                |b| b, suffix_icon(), disabled_on_surface;
            outlined_hovered_prefix_icon_has_correct_color: prefixed, outlined(),
                |b| b.is_hovering(true), prefix_icon(), on_surface_variant;
            outlined_hovered_suffix_icon_has_correct_color: suffixed, outlined(),
                |b| b.is_hovering(true), suffix_icon(), on_surface_variant;
            outlined_focused_prefix_icon_has_correct_color: prefixed, outlined(),
                |b| b.is_focused(true), prefix_icon(), on_surface_variant;
            outlined_focused_suffix_icon_has_correct_color: suffixed, outlined(),
                |b| b.is_focused(true), suffix_icon(), on_surface_variant;
            outlined_error_prefix_icon_has_correct_color: prefixed, outlined().error_text(ERROR_TEXT),
                |b| b, prefix_icon(), on_surface_variant;
            outlined_error_prefix_icon_has_correct_color_when_hovered: prefixed,
                outlined().error_text(ERROR_TEXT), |b| b.is_hovering(true), prefix_icon(),
                on_surface_variant;
            outlined_error_suffix_icon_has_correct_color: suffixed, outlined().error_text(ERROR_TEXT),
                |b| b, suffix_icon(), error;
            outlined_error_suffix_icon_has_correct_color_when_hovered: suffixed,
                outlined().error_text(ERROR_TEXT), |b| b.is_hovering(true), suffix_icon(),
                on_error_container;
        }

        /// The colour of the nearest `IconTheme` above the `Icon` showing `icon`: Dart's
        /// `tester.widget<IconTheme>(find.widgetWithIcon(IconTheme, icon).first).data.color`.
        fn icon_theme_color(tester: &WidgetTester, icon: &IconData) -> Option<Color> {
            let icon_element = tester.find_by_icon(icon);
            let icon_theme = tester
                .ancestors(icon_element)
                .into_iter()
                .find(|element| {
                    let app = tester.cell.borrow();
                    downcast_widget::<waft_widgets::IconTheme>(&**element.widget(&app)).is_some()
                })
                .expect("an IconTheme above the icon");
            tester.widget(icon_theme, |theme: &waft_widgets::IconTheme| {
                theme.data.color.as_ref().map(|color| **color)
            })
        }

        // input_decorator_test.dart 'Material3 - InputDecoration prefix/suffix - InputDecorator
        // iconColor/prefixIconColor/suffixIconColor', on an `InputDecorator` where Dart uses the
        // `TextField` that builds one.
        #[test]
        fn the_icon_colors_reach_the_icon_themes() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .icon(Icon::new(Some(Icons::cabin())))
                            .prefix_icon(Icon::new(Some(Icons::sailing())))
                            .suffix_icon(Icon::new(Some(Icons::close())))
                            .icon_color(crate::colors::Colors::AMBER.primary())
                            .prefix_icon_color(crate::colors::Colors::GREEN.primary())
                            .suffix_icon_color(crate::colors::Colors::RED.primary())
                            .filled(true),
                    ),
                );
                assert_eq!(
                    icon_theme_color(tester, &Icons::cabin()),
                    Some(crate::colors::Colors::AMBER.primary())
                );
                assert_eq!(
                    icon_theme_color(tester, &Icons::sailing()),
                    Some(crate::colors::Colors::GREEN.primary())
                );
                assert_eq!(
                    icon_theme_color(tester, &Icons::close()),
                    Some(crate::colors::Colors::RED.primary())
                );
            });
        }

        /// The target opacity of the `AnimatedOpacity` above the only `Icon`.
        fn affix_opacity(tester: &WidgetTester) -> f64 {
            let icon = tester.find_by_type::<Icon>()[0];
            let opacity = tester
                .ancestors(icon)
                .into_iter()
                .find(|element| {
                    let app = tester.cell.borrow();
                    downcast_widget::<AnimatedOpacity>(&**element.widget(&app)).is_some()
                })
                .expect("an AnimatedOpacity above the icon");
            tester.widget(opacity, |opacity: &AnimatedOpacity| opacity.opacity)
        }

        /// Dart's 'Prefix/Suffix ignores pointer when hidden' on an `InputDecorator`: the
        /// `TextField` Dart taps to focus becomes a rebuild with `is_focused`.
        fn check_affix_ignores_pointer_when_hidden(
            tester: &mut WidgetTester,
            affix: fn(InputDecoration, WidgetRef) -> InputDecoration,
        ) {
            let tapped = Rc::new(Cell::new(false));
            let build = |is_focused: bool| {
                let tapped = Rc::clone(&tapped);
                let affix_widget = GestureDetector::new()
                    .on_tap(Listener::new(move |_app| tapped.set(true)))
                    .child(Icon::new(Some(Icons::search())))
                    .into_widget();
                BuildInputDecorator::new(affix(
                    InputDecoration::new().label_text("label"),
                    affix_widget,
                ))
                .is_empty(true)
                .is_focused(is_focused)
                .build()
            };
            tester.pump_widget(build(false));
            assert!(!tapped.get());

            // Initially the affix icon should be hidden.
            assert_eq!(affix_opacity(tester), 0.0);

            let icon_center = rect_of(tester, tester.find_by_type::<Icon>()[0]).center();
            tester.tap(icon_center, 1);
            tester.pump();

            // The affix icon should ignore pointer events when hidden.
            assert!(!tapped.get());

            // Focus the decorator to show the affix icon.
            tester.pump_widget(build(true));

            // The affix icon should be visible.
            assert_eq!(affix_opacity(tester), 1.0);

            // Tap the affix icon.
            let icon_center = rect_of(tester, tester.find_by_type::<Icon>()[0]).center();
            tester.tap(icon_center, 2);
            tester.pump();

            // The affix icon should be tapped.
            assert!(tapped.get());
        }

        // input_decorator_test.dart 'Material3 - InputDecoration prefix/suffix - Prefix ignores
        // pointer when hidden'.
        #[test]
        fn the_prefix_ignores_the_pointer_when_hidden() {
            test_widgets(|tester| {
                check_affix_ignores_pointer_when_hidden(tester, |decoration, prefix| {
                    decoration.prefix(prefix)
                })
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration prefix/suffix - Suffix ignores
        // pointer when hidden'.
        #[test]
        fn the_suffix_ignores_the_pointer_when_hidden() {
            test_widgets(|tester| {
                check_affix_ignores_pointer_when_hidden(tester, |decoration, suffix| {
                    decoration.suffix(suffix)
                })
            });
        }

        /// Dart's `getHintOpacity(tester)`: the value of the first `FadeTransition` above the hint.
        fn hint_opacity(tester: &WidgetTester) -> f64 {
            let hint = tester.find_text(HINT_TEXT)[0];
            let fade = tester
                .ancestors(hint)
                .into_iter()
                .find(|element| {
                    let app = tester.cell.borrow();
                    downcast_widget::<FadeTransition>(&**element.widget(&app)).is_some()
                })
                .expect("a FadeTransition above the hint");
            let opacity = tester.widget(fade, |fade: &FadeTransition| fade.opacity.clone());
            opacity.value(&tester.cell.borrow())
        }

        fn collapsed() -> InputDecoration {
            InputDecoration::collapsed(None).hint_text(HINT_TEXT)
        }

        // Overall height for a collapsed InputDecorator is 24dp which is the input
        // height (font size = 16, line height = 1.5).
        const COLLAPSED_INPUT_HEIGHT: f64 = 24.0;

        fn check_collapsed_height(tester: &mut WidgetTester) {
            tester.pump_widget(BuildInputDecorator::new(collapsed()).build());

            assert_eq!(
                get_decorator_rect(tester).size(),
                Size::new(800.0, COLLAPSED_INPUT_HEIGHT)
            );
            assert_eq!(input_rect(tester).height(), COLLAPSED_INPUT_HEIGHT);
            assert_eq!(input_rect(tester).top, 0.0);
            assert_eq!(hint_opacity(tester), 0.0);

            // The hint should appear.
            tester.pump_widget(
                BuildInputDecorator::new(collapsed())
                    .is_empty(true)
                    .is_focused(true)
                    .build(),
            );
            tester.pump_and_settle();

            assert_eq!(
                get_decorator_rect(tester).size(),
                Size::new(800.0, COLLAPSED_INPUT_HEIGHT)
            );
            assert_eq!(input_rect(tester).height(), COLLAPSED_INPUT_HEIGHT);
            assert_eq!(input_rect(tester).top, 0.0);
            assert_eq!(hint_opacity(tester), 1.0);
            assert_eq!(
                text_rect(tester, HINT_TEXT).height(),
                COLLAPSED_INPUT_HEIGHT
            );
            assert_eq!(text_rect(tester, HINT_TEXT).top, 0.0);
        }

        // input_decorator_test.dart 'Material3 - InputDecoration collapsed - Decoration height is
        // set to input height on mobile' (TargetPlatformVariant.mobile()).
        #[test]
        fn a_collapsed_decoration_is_the_input_height_on_mobile() {
            test_widgets_variant(TargetPlatformVariant::mobile(), check_collapsed_height);
        }

        // input_decorator_test.dart 'Material3 - InputDecoration collapsed - Decoration height is
        // set to input height on desktop' (TargetPlatformVariant.desktop()).
        #[test]
        fn a_collapsed_decoration_is_the_input_height_on_desktop() {
            test_widgets_variant(TargetPlatformVariant::desktop(), check_collapsed_height);
        }

        // input_decorator_test.dart 'Material3 - InputDecoration collapsed -
        // InputDecoration.collapsed defaults to no border'.
        #[test]
        fn a_collapsed_decoration_has_no_border() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(InputDecoration::collapsed(Some(
                        HINT_TEXT.to_owned(),
                    ))),
                );
                assert_eq!(get_border_weight(tester), 0.0);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration collapsed -
        // InputDecoration.collapsed accepts constraints'.
        #[test]
        fn a_collapsed_decoration_takes_constraints() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::collapsed(Some(HINT_TEXT.to_owned()))
                            .constraints(BoxConstraints::tight_for(Some(200.0), Some(32.0))),
                    ),
                );
                assert_eq!(get_decorator_rect(tester).size(), Size::new(200.0, 32.0));
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration collapsed -
        // InputDecoration.collapsed accepts hintMaxLines'.
        #[test]
        fn a_collapsed_decoration_takes_hint_max_lines() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::collapsed(Some(THREE_LINES.to_owned())).hint_max_lines(2),
                    ),
                );
                const HINT_LINE_HEIGHT: f64 = 24.0; // font size = 16 and font height = 1.5.
                assert_eq!(
                    get_decorator_rect(tester).size(),
                    Size::new(800.0, 2.0 * HINT_LINE_HEIGHT)
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration collapsed -
        // InputDecoration.collapsed accepts hintFadeDuration'.
        #[test]
        fn a_collapsed_decoration_takes_hint_fade_duration() {
            test_widgets(|tester| {
                let decoration = || collapsed().hint_fade_duration(Duration::from_millis(120));
                // Build once with empty content.
                tester.pump_widget(
                    BuildInputDecorator::new(decoration())
                        .is_empty(true)
                        .build(),
                );

                // Hint is visible (opacity 1.0).
                assert_eq!(hint_opacity(tester), 1.0);

                // Rebuild with non-empty content.
                tester.pump_widget(BuildInputDecorator::new(decoration()).build());

                // The hint's opacity animates from 1.0 to 0.0.
                // The animation's default duration is 20ms.
                tester.pump_by(Duration::from_millis(50));
                let hint_opacity_50ms = hint_opacity(tester);
                assert!(hint_opacity_50ms > 0.0 && hint_opacity_50ms < 1.0);
                tester.pump_by(Duration::from_millis(50));
                let hint_opacity_100ms = hint_opacity(tester);
                assert!(hint_opacity_100ms > 0.0 && hint_opacity_100ms < hint_opacity_50ms);
                tester.pump_by(Duration::from_millis(50));
                assert_eq!(hint_opacity(tester), 0.0);
            });
        }

        /// Dart's `buildInputDecoratorWithFloatingLabel`, the label a text.
        fn floating_label_decorator(
            text_direction: TextDirection,
            has_icon: bool,
            alignment: FloatingLabelAlignment,
            border_is_outline: bool,
        ) -> WidgetRef {
            let mut decoration = InputDecoration::new()
                .content_padding(EdgeInsetsGeometry::directional(40.0, 12.0, 0.0, 12.0))
                .floating_label_alignment(alignment)
                .label_text(LABEL_TEXT)
                .hint_text(HINT_TEXT)
                .filled(true);
            if has_icon {
                decoration = decoration.icon(Icon::new(Some(Icons::insert_link())));
            }
            if border_is_outline {
                decoration = decoration.border(OutlineInputBorder::new());
            }
            BuildInputDecorator::new(decoration)
                .text_direction(text_direction)
                .build()
        }

        /// Pumps the filled and then the outlined frame of a floatingLabelAlignment case and
        /// checks each with `check`.
        fn check_floating_label(
            tester: &mut WidgetTester,
            text_direction: TextDirection,
            has_icon: bool,
            alignment: FloatingLabelAlignment,
            check: impl Fn(Rect),
        ) {
            for border_is_outline in [false, true] {
                tester.pump_widget(floating_label_decorator(
                    text_direction,
                    has_icon,
                    alignment,
                    border_is_outline,
                ));
                check(get_label_rect(tester));
            }
        }

        // input_decorator_test.dart 'Material3 - InputDecoration floatingLabelAlignment - LTR with
        // icon aligned - start'.
        #[test]
        fn ltr_with_an_icon_the_floating_label_aligns_to_the_start() {
            test_widgets(|tester| {
                check_floating_label(
                    tester,
                    TextDirection::Ltr,
                    true,
                    FloatingLabelAlignment::START,
                    // icon (40) + contentPadding (40) + _kInputExtraPadding
                    |label| assert_eq!(label.left, 84.0),
                );
            });
        }

        // input_decorator_test.dart '... - LTR with icon aligned - center'.
        #[test]
        fn ltr_with_an_icon_the_floating_label_aligns_to_the_center() {
            test_widgets(|tester| {
                check_floating_label(
                    tester,
                    TextDirection::Ltr,
                    true,
                    FloatingLabelAlignment::CENTER,
                    // icon (40) + (decorator (800) - icon (40)) / 2
                    |label| assert_eq!(label.center().dx(), 420.0),
                );
            });
        }

        // input_decorator_test.dart '... - LTR without icon aligned - start'.
        #[test]
        fn ltr_without_an_icon_the_floating_label_aligns_to_the_start() {
            test_widgets(|tester| {
                check_floating_label(
                    tester,
                    TextDirection::Ltr,
                    false,
                    FloatingLabelAlignment::START,
                    // contentPadding (40) + _kInputExtraPadding
                    |label| assert_eq!(label.left, 44.0),
                );
            });
        }

        // input_decorator_test.dart '... - LTR without icon aligned - center'.
        #[test]
        fn ltr_without_an_icon_the_floating_label_aligns_to_the_center() {
            test_widgets(|tester| {
                check_floating_label(
                    tester,
                    TextDirection::Ltr,
                    false,
                    FloatingLabelAlignment::CENTER,
                    // decorator (800) / 2
                    |label| assert_eq!(label.center().dx(), 400.0),
                );
            });
        }

        // input_decorator_test.dart '... - RTL with icon aligned - start'.
        #[test]
        fn rtl_with_an_icon_the_floating_label_aligns_to_the_start() {
            test_widgets(|tester| {
                check_floating_label(
                    tester,
                    TextDirection::Rtl,
                    true,
                    FloatingLabelAlignment::START,
                    // decorator (800) - icon (40) - contentPadding (40) - _kInputExtraPadding
                    |label| assert_eq!(label.right, 716.0),
                );
            });
        }

        // input_decorator_test.dart '... - RTL with icon aligned - center'.
        #[test]
        fn rtl_with_an_icon_the_floating_label_aligns_to_the_center() {
            test_widgets(|tester| {
                check_floating_label(
                    tester,
                    TextDirection::Rtl,
                    true,
                    FloatingLabelAlignment::CENTER,
                    // (decorator (800) - icon (40)) / 2
                    |label| assert_eq!(label.center().dx(), 380.0),
                );
            });
        }

        // input_decorator_test.dart '... - RTL without icon aligned - start'.
        #[test]
        fn rtl_without_an_icon_the_floating_label_aligns_to_the_start() {
            test_widgets(|tester| {
                check_floating_label(
                    tester,
                    TextDirection::Rtl,
                    false,
                    FloatingLabelAlignment::START,
                    // decorator (800) - contentPadding (40) - _kInputExtraPadding
                    |label| assert_eq!(label.right, 756.0),
                );
            });
        }

        // input_decorator_test.dart '... - RTL without icon aligned - center'.
        #[test]
        fn rtl_without_an_icon_the_floating_label_aligns_to_the_center() {
            test_widgets(|tester| {
                check_floating_label(
                    tester,
                    TextDirection::Rtl,
                    false,
                    FloatingLabelAlignment::CENTER,
                    // decorator (800) / 2
                    |label| assert_eq!(label.center().dx(), 400.0),
                );
            });
        }

        // M3 extra horizontal padding.
        const K_INPUT_EXTRA_PADDING: f64 = 4.0;

        // input_decorator_test.dart 'Material3 - InputDecoration isDense - Dense layout for an
        // outlined decoration'.
        #[test]
        fn the_dense_layout_of_an_outlined_decoration() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(outlined().label_text(LABEL_TEXT).is_dense(true)),
                );
                let container_rect = get_container_rect(tester);
                let input_rect = input_rect(tester);

                // Content padding is EdgeInsetsDirectional.fromSTEB(12.0, 16.0, 12.0, 8.0).
                // Vertical padding affects the container height not the input vertical
                // position as the input is centered.
                assert_eq!(container_rect.height(), 48.0);
                assert_eq!(input_rect.center().dy(), container_rect.center().dy());
                assert_eq!(
                    input_rect.left,
                    container_rect.left + 12.0 + K_INPUT_EXTRA_PADDING
                );
                assert_eq!(
                    input_rect.right,
                    container_rect.right - 12.0 - K_INPUT_EXTRA_PADDING
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration isDense - Dense layout for a filled
        // and non-outlined decoration'.
        #[test]
        fn the_dense_layout_of_a_filled_decoration() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(filled().label_text(LABEL_TEXT).is_dense(true)),
                );
                let container_rect = get_container_rect(tester);
                let input_rect = input_rect(tester);
                let label_rect = get_label_rect(tester);

                // Content padding is EdgeInsetsDirectional.fromSTEB(12.0, 4.0, 12.0, 4.0).
                assert_eq!(container_rect.height(), 48.0);
                assert_eq!(label_rect.top, container_rect.top + 4.0);
                assert_eq!(input_rect.bottom, container_rect.bottom - 4.0);
                assert_eq!(
                    input_rect.left,
                    container_rect.left + 12.0 + K_INPUT_EXTRA_PADDING
                );
                assert_eq!(
                    input_rect.right,
                    container_rect.right - 12.0 - K_INPUT_EXTRA_PADDING
                );
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration isDense - Dense layout for a
        // non-filled and non-outlined decoration'.
        #[test]
        fn the_dense_layout_of_an_unfilled_underlined_decoration() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new().label_text(LABEL_TEXT).is_dense(true),
                    ),
                );
                let container_rect = get_container_rect(tester);
                let input_rect = input_rect(tester);
                let label_rect = get_label_rect(tester);

                // Content padding is EdgeInsetsDirectional.fromSTEB(0.0, 4.0, 0.0, 4.0).
                assert_eq!(container_rect.height(), 48.0);
                assert_eq!(label_rect.top, container_rect.top + 4.0);
                assert_eq!(input_rect.bottom, container_rect.bottom - 4.0);
                assert_eq!(input_rect.left, container_rect.left);
                assert_eq!(input_rect.right, container_rect.right);
            });
        }

        // input_decorator_test.dart 'Material3 - InputDecoration isDense - Ambient theme is used'.
        #[test]
        fn the_ambient_theme_is_dense() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(InputDecoration::new().label_text(LABEL_TEXT))
                        .input_decoration_theme(InputDecorationThemeData::new().is_dense(true)),
                );
                assert_eq!(get_container_rect(tester).height(), 48.0);
            });
        }

        // input_decorator_test.dart 'InputDecorator.debugDescribeChildren': the part that holds
        // without diagnostics, the decoration's eleven children, one per slot.
        #[test]
        fn a_decoration_with_every_slot_has_eleven_children() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .icon(Text::new("icon"))
                            .label_text("label")
                            .hint_text("hint")
                            .prefix_text("prefix")
                            .suffix_text("suffix")
                            .prefix_icon(Text::new("prefixIcon"))
                            .suffix_icon(Text::new("suffixIcon"))
                            .helper_text("helper")
                            .counter_text("counter"),
                    )
                    .child(Text::new("text")),
                );
                let render = tester.first::<RenderDecoration>();
                let app = tester.cell.borrow();
                let children = SlottedContainerRenderObjectMixin::children(render, &app);
                assert_eq!(children.len(), 11);
                for (index, child) in children.iter().enumerate() {
                    assert!(
                        !children[..index].contains(child),
                        "each slot has its own child"
                    );
                }
            });
        }
    }

    // input_decorator_test.dart's cases on themes, widget-state values, intrinsics, the local
    // theme's overrides, icon button themes and custom borders.
    mod themes_and_intrinsics {
        use std::time::Duration;
        use waft_painting::OutlinedBorderRef;
        use waft_widgets::testing::{TestPointer, WidgetTester, test_widgets};

        use waft_embedder::valo::Op;
        use waft_embedder::{Color, Offset, PaintStyle, Radius, Size};
        use waft_foundation::Listener;
        use waft_painting::{
            AlignmentGeometry, BorderRadius, BorderSide, EdgeInsets, EdgeInsetsDirectional,
            EdgeInsetsGeometry, RoundedRectangleBorder, ShapeBorder, TextDirection, TextStyle,
        };
        use waft_rendering::BoxConstraints;
        use waft_widgets::{
            Builder, Center, Column, ConstrainedBox, CrossAxisAlignment, Directionality, Expanded,
            FadeTransition, GlobalKey, Icon, IconData, IconTheme, IntoWidget, IntrinsicHeight,
            MainAxisSize, RichText, Row, SizedBox, Text, UnconstrainedBox, WidgetRef, WidgetState,
            WidgetStateProperty, WidgetStatePropertyAll, WidgetStatePropertyRef,
            WidgetStateTextStyle, WidgetStates,
        };

        use super::super::*;
        use super::common::K_TRANSITION_DURATION;
        use super::common::*;
        use crate::app::MaterialApp;
        use crate::button_style::ButtonStyle;
        use crate::colors::Colors;
        use crate::icon_button::IconButton;
        use crate::icon_button_theme::{IconButtonTheme, IconButtonThemeData};
        use crate::icons::Icons;
        use crate::input_border::{InputBorderRef, OutlineInputBorder};
        use crate::material::Material;
        use crate::material_state::WidgetStateInputBorder;
        use crate::scaffold::Scaffold;
        use crate::test_support::{Drawn, PaintInspection};
        use crate::theme::Theme;
        use crate::theme_data::VisualDensity;

        fn noop() -> Listener {
            Listener::new(|_app| {})
        }

        fn icon(icon: IconData) -> Icon {
            Icon::new(Some(icon))
        }

        /// The effective style of the `RichText` a `Text` showing `text` builds: Dart's
        /// `tester.widget<RichText>(find.descendant(of: find.text(text), matching:
        /// find.byType(RichText))).text.style!`.
        fn rich_text_style(tester: &WidgetTester, text: &str) -> TextStyle {
            let found = *tester
                .find_text(text)
                .first()
                .expect("a Text showing the text");
            let rich_text = *tester
                .descendants_of_type::<RichText>(found)
                .first()
                .expect("the Text's RichText");
            tester.widget(rich_text, |rich_text: &RichText| {
                rich_text.text.style().cloned().expect("a styled text")
            })
        }

        fn style_color(style: &TextStyle) -> Option<Color> {
            style.color.as_ref().map(|color| color.color())
        }

        /// Dart's `getOpacity(tester, text)`: the value of the nearest `FadeTransition` above the
        /// `Text` showing `text`.
        fn get_opacity(tester: &WidgetTester, text: &str) -> f64 {
            let found = *tester
                .find_text(text)
                .first()
                .expect("a Text showing the text");
            let fade = tester
                .ancestors(found)
                .into_iter()
                .find(|element| {
                    let app = tester.cell.borrow();
                    waft_widgets::downcast_widget::<FadeTransition>(&**element.widget(&app))
                        .is_some()
                })
                .expect("a FadeTransition above the text");
            let opacity = tester.widget(fade, |fade: &FadeTransition| fade.opacity.clone());
            opacity.value(&tester.cell.borrow())
        }

        /// The colour of the `IconTheme` nearest the `Icon` showing `icon`: Dart's
        /// `tester.widget<IconTheme>(find.widgetWithIcon(IconTheme, icon).first).data.color`.
        fn icon_theme_color(tester: &WidgetTester, icon: &IconData) -> Option<Color> {
            let found = tester.find_by_icon(icon);
            let icon_theme = tester
                .ancestors(found)
                .into_iter()
                .find(|element| {
                    let app = tester.cell.borrow();
                    waft_widgets::downcast_widget::<IconTheme>(&**element.widget(&app)).is_some()
                })
                .expect("an IconTheme above the icon");
            tester.widget(icon_theme, |theme: &IconTheme| {
                theme.data.color.as_ref().map(|color| color.color())
            })
        }

        /// Dart's `tester.getSize(find.byIcon(icon))`.
        fn icon_size(tester: &WidgetTester, icon: &IconData) -> Size {
            let found = tester.find_by_icon(icon);
            let render = tester.render_box_of(found);
            render.size(&tester.cell.borrow())
        }

        /// Dart's `getIconStyle(tester, icon)?.color`.
        fn icon_glyph_color(tester: &WidgetTester, icon: &IconData) -> Option<Color> {
            let found = tester.find_by_icon(icon);
            style_color(&tester.glyph_style(found))
        }

        /// Dart's `getBorderRadius`'s render object, the `CustomPaint` of the border: what it
        /// paints, as Dart's `paints` matcher records it.
        fn border_painted(tester: &WidgetTester) -> Vec<Drawn> {
            let element = find_border_painter(tester).expect("a border painter");
            let render = tester.render_box_of(element);
            tester.painted(render.as_object())
        }

        /// Dart's `paints..rrect(style: PaintingStyle.fill, color: color)` on the border painter.
        fn paints_fill(tester: &WidgetTester, color: Color) -> bool {
            border_painted(tester)
                .iter()
                .any(|drawn| matches!(drawn, Drawn::Path(argb, _) if *argb == color.to_argb32()))
        }

        /// The strokes the border painter paints, as (colour, width): what Dart's
        /// `paints..rrect(color:, strokeWidth:)` matches.
        fn border_strokes(tester: &WidgetTester) -> Vec<(waft_embedder::valo::Color, f32)> {
            let element = find_border_painter(tester).expect("a border painter");
            let render = tester.render_box_of(element);
            tester
                .painted_ops(render.as_object())
                .into_iter()
                .filter_map(|op| match op {
                    Op::DrawPath { paint, .. } => match paint.style {
                        PaintStyle::Stroke(stroke) => Some((paint.color, stroke.width)),
                        PaintStyle::Fill => None,
                    },
                    _ => None,
                })
                .collect()
        }

        fn outline(color: Color, width: f64) -> OutlineInputBorder {
            OutlineInputBorder::new().border_side(BorderSide {
                color,
                width,
                ..BorderSide::default()
            })
        }

        fn border_is(tester: &WidgetTester, expected: &dyn ShapeBorder) -> bool {
            get_border(tester).expect("a border").eq_shape(expected)
        }

        fn constraints(
            min_width: f64,
            max_width: f64,
            min_height: f64,
            max_height: f64,
        ) -> BoxConstraints {
            BoxConstraints {
                min_width,
                max_width,
                min_height,
                max_height,
            }
        }

        fn decorator_render(tester: &WidgetTester) -> waft_rendering::AnyRenderBox {
            tester.render_box_of(tester.find_by_type::<InputDecorator>()[0])
        }

        fn key_size(tester: &WidgetTester, key: &GlobalKey) -> Size {
            let render = tester.render_object(key).as_box().expect("a render box");
            render.size(&tester.cell.borrow())
        }

        fn pump(tester: &mut WidgetTester, decorator: BuildInputDecorator) {
            tester.pump_widget(decorator.build());
        }

        // input_decorator_test.dart 'InputDecorationThemeData.inputDecoration with WidgetState'.
        #[test]
        fn widget_state_values_go_from_the_theme_to_the_decoration_unless_it_has_its_own() {
            test_widgets(|_tester| {
                let theme_style: AnyTextStyle =
                    WidgetStateTextStyle::resolve_with(|_states: &WidgetStates| {
                        TextStyle::new().color(Colors::GREEN.primary())
                    })
                    .into_any();
                let decoration_style: AnyTextStyle =
                    WidgetStateTextStyle::resolve_with(|_states: &WidgetStates| {
                        TextStyle::new().color(Colors::BLUE.primary())
                    })
                    .into_any();
                let none = InputBorderRef::none();

                // InputDecorationThemeData arguments define InputDecoration properties.
                let decoration = InputDecoration::new().apply_defaults(
                    InputDecorationThemeData::new()
                        .label_style(theme_style.clone())
                        .helper_style(theme_style.clone())
                        .hint_style(theme_style.clone())
                        .error_style(theme_style.clone())
                        .is_dense(true)
                        .content_padding(EdgeInsets::all(1.0).into())
                        .prefix_style(theme_style.clone())
                        .suffix_style(theme_style.clone())
                        .counter_style(theme_style.clone())
                        .filled(true)
                        .fill_color(Colors::RED.primary())
                        .focus_color(Colors::BLUE.primary())
                        .border(InputBorderRef::none())
                        .align_label_with_hint(true)
                        .constraints(constraints(10.0, 20.0, 30.0, 40.0)),
                );

                assert_eq!(decoration.label_style, Some(theme_style.clone()));
                assert_eq!(decoration.helper_style, Some(theme_style.clone()));
                assert_eq!(decoration.hint_style, Some(theme_style.clone()));
                assert_eq!(decoration.error_style, Some(theme_style.clone()));
                assert_eq!(decoration.is_dense, Some(true));
                assert_eq!(
                    decoration.content_padding,
                    Some(EdgeInsets::all(1.0).into())
                );
                assert_eq!(decoration.prefix_style, Some(theme_style.clone()));
                assert_eq!(decoration.suffix_style, Some(theme_style.clone()));
                assert_eq!(decoration.counter_style, Some(theme_style.clone()));
                assert_eq!(decoration.filled, Some(true));
                assert_eq!(
                    decoration.fill_color,
                    Some(AnyColor::from(Colors::RED.primary()))
                );
                assert!(decoration.border.as_deref() == Some(&*none));
                assert_eq!(decoration.align_label_with_hint, Some(true));
                assert_eq!(
                    decoration.constraints,
                    Some(constraints(10.0, 20.0, 30.0, 40.0))
                );

                // InputDecoration (baseDecoration) defines InputDecoration properties
                let border = WidgetStateInputBorder::resolve_with(|_states: &WidgetStates| {
                    InputBorderRef::new(OutlineInputBorder::new())
                });
                let decoration = InputDecoration::new()
                    .label_style(decoration_style.clone())
                    .helper_style(decoration_style.clone())
                    .hint_style(decoration_style.clone())
                    .error_style(decoration_style.clone())
                    .is_dense(false)
                    .content_padding(EdgeInsets::all(4.0).into())
                    .prefix_style(decoration_style.clone())
                    .suffix_style(decoration_style.clone())
                    .counter_style(decoration_style.clone())
                    .filled(false)
                    .fill_color(Colors::BLUE.primary())
                    .border(border)
                    .align_label_with_hint(false)
                    .constraints(constraints(10.0, 20.0, 30.0, 40.0))
                    .apply_defaults(
                        InputDecorationThemeData::new()
                            .label_style(theme_style.clone())
                            .helper_style(theme_style.clone())
                            .helper_max_lines(5)
                            .hint_style(theme_style.clone())
                            .error_style(theme_style.clone())
                            .error_max_lines(4)
                            .is_dense(true)
                            .content_padding(EdgeInsets::all(1.0).into())
                            .prefix_style(theme_style.clone())
                            .suffix_style(theme_style.clone())
                            .counter_style(theme_style.clone())
                            .filled(true)
                            .fill_color(Colors::RED.primary())
                            .focus_color(Colors::BLUE.primary())
                            .border(InputBorderRef::none())
                            .align_label_with_hint(true)
                            .constraints(constraints(40.0, 30.0, 20.0, 10.0)),
                    );

                assert_eq!(decoration.label_style, Some(decoration_style.clone()));
                assert_eq!(decoration.helper_style, Some(decoration_style.clone()));
                assert_eq!(decoration.helper_max_lines, Some(5));
                assert_eq!(decoration.hint_style, Some(decoration_style.clone()));
                assert_eq!(decoration.error_style, Some(decoration_style.clone()));
                assert_eq!(decoration.error_max_lines, Some(4));
                assert_eq!(decoration.is_dense, Some(false));
                assert_eq!(
                    decoration.content_padding,
                    Some(EdgeInsets::all(4.0).into())
                );
                assert_eq!(decoration.prefix_style, Some(decoration_style.clone()));
                assert_eq!(decoration.suffix_style, Some(decoration_style.clone()));
                assert_eq!(decoration.counter_style, Some(decoration_style.clone()));
                assert_eq!(decoration.filled, Some(false));
                assert_eq!(
                    decoration.fill_color,
                    Some(AnyColor::from(Colors::BLUE.primary()))
                );
                assert!(
                    decoration
                        .border
                        .as_ref()
                        .expect("a border")
                        .as_any()
                        .is::<WidgetStateInputBorder>()
                );
                assert_eq!(decoration.align_label_with_hint, Some(false));
                assert_eq!(
                    decoration.constraints,
                    Some(constraints(10.0, 20.0, 30.0, 40.0))
                );
            });
        }

        // input_decorator_test.dart 'InputDecoration with WidgetStateInputBorder', on an
        // `InputDecorator` whose `isFocused`/`isHovering` stand for the `TextField`'s focus and the
        // mouse over it.
        #[test]
        fn a_widget_state_input_border_paints_the_border_of_the_state() {
            test_widgets(|tester| {
                let outline_input_border = || {
                    WidgetStateInputBorder::from_map(vec![
                        (
                            WidgetState::Focused.into(),
                            InputBorderRef::new(outline(Colors::BLUE.primary(), 4.0)),
                        ),
                        (
                            WidgetState::Hovered.into(),
                            InputBorderRef::new(outline(Colors::CYAN.primary(), 8.0)),
                        ),
                        (
                            WidgetState::any(),
                            InputBorderRef::new(OutlineInputBorder::new()),
                        ),
                    ])
                };
                let build = |is_focused: bool, is_hovering: bool| {
                    BuildInputDecorator::new(InputDecoration::new().border(outline_input_border()))
                        .is_focused(is_focused)
                        .is_hovering(is_hovering)
                        .build()
                };
                tester.pump_widget(build(false, false));
                assert!(
                    border_strokes(tester)
                        .iter()
                        .any(|(_, width)| *width == 1.0)
                );

                tester.pump_widget(build(true, false));
                tester.pump_and_settle();
                let blue: waft_embedder::valo::Color = Colors::BLUE.primary().into();
                assert!(
                    border_strokes(tester)
                        .iter()
                        .any(|(color, width)| *color == blue && *width == 4.0)
                );

                tester.pump_widget(build(false, true));
                tester.pump_and_settle();
                let cyan: waft_embedder::valo::Color = Colors::CYAN.primary().into();
                assert!(
                    border_strokes(tester)
                        .iter()
                        .any(|(color, width)| *color == cyan && *width == 8.0)
                );
            });
        }

        // input_decorator_test.dart 'InputDecorator constrained to 0x0'.
        #[test]
        fn a_decorator_constrained_to_zero_by_zero_lays_out() {
            test_widgets(|tester| {
                tester.pump_widget(
                    crate::material::Material::new()
                        .child(Directionality::new(
                            TextDirection::Ltr,
                            UnconstrainedBox::new().child(
                                ConstrainedBox::new(BoxConstraints::tight(Size::ZERO)).child(
                                    InputDecorator::new(
                                        InputDecoration::new()
                                            .label_text("XP")
                                            .border(OutlineInputBorder::new()),
                                    ),
                                ),
                            ),
                        ))
                        .into_widget(),
                );
            });
        }

        fn labeled_input_decorator(show_fix: bool, is_focused: bool) -> WidgetRef {
            const PREFIX_TEXT: &str = "The five boxing wizards jump quickly.";
            const SUFFIX_TEXT: &str = "Suffix";
            let mut decoration = InputDecoration::new()
                .icon(icon(Icons::assistant()))
                .suffix_icon(icon(Icons::threesixty()))
                .label_text("My Label");
            if show_fix {
                decoration = decoration.prefix_text(PREFIX_TEXT).suffix_text(SUFFIX_TEXT);
            }
            MaterialApp::new()
                .home(Material::new().child(Builder::new(move |app, context| {
                    Theme::new(
                        Theme::of(app, context),
                        waft_widgets::Align::new()
                            .alignment(AlignmentGeometry::Alignment(
                                waft_painting::Alignment::TOP_LEFT,
                            ))
                            .child(
                                InputDecorator::new(decoration.clone())
                                    .is_empty(true)
                                    .is_focused(is_focused)
                                    .child(input_text()),
                            ),
                    )
                    .into_widget()
                })))
                .into_widget()
        }

        fn text_size(tester: &WidgetTester, text: &str) -> Size {
            let found = *tester
                .find_text(text)
                .first()
                .expect("a Text showing the text");
            tester.render_box_of(found).size(&tester.cell.borrow())
        }

        // input_decorator_test.dart "InputDecorator label width isn't affected by prefix or
        // suffix", on an empty `InputDecorator` that the tap focuses.
        #[test]
        fn the_label_width_is_not_affected_by_the_prefix_or_suffix() {
            test_widgets(|tester| {
                const PREFIX_TEXT: &str = "The five boxing wizards jump quickly.";
                const SUFFIX_TEXT: &str = "Suffix";
                // Build with no prefix or suffix.
                tester.pump_widget(labeled_input_decorator(false, false));

                // Get the width of the label when there is no prefix/suffix.
                assert!(tester.find_text(PREFIX_TEXT).is_empty());
                assert!(tester.find_text(SUFFIX_TEXT).is_empty());
                let label_width = text_size(tester, "My Label").width();

                // Build with a prefix and suffix.
                tester.pump_widget(labeled_input_decorator(true, false));

                // The prefix and suffix exist but aren't visible. They have not affected
                // the width of the label.
                assert_eq!(tester.find_text(PREFIX_TEXT).len(), 1);
                assert_eq!(get_opacity(tester, PREFIX_TEXT), 0.0);
                assert_eq!(tester.find_text(SUFFIX_TEXT).len(), 1);
                assert_eq!(get_opacity(tester, SUFFIX_TEXT), 0.0);
                assert_eq!(text_size(tester, "My Label").width(), label_width);

                // Tap to focus.
                tester.pump_widget(labeled_input_decorator(true, true));
                tester.pump_and_settle();

                // The prefix and suffix are visible, and the label is floating and still
                // hasn't had its width affected.
                assert_eq!(text_size(tester, "My Label").width(), label_width);
                assert_eq!(get_opacity(tester, PREFIX_TEXT), 1.0);
            });
        }

        // input_decorator_test.dart 'Prefix and suffix are not visible when decorator is empty'.
        #[test]
        fn the_prefix_and_suffix_are_hidden_when_the_decorator_is_empty() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .filled(true)
                            .label_text(LABEL_TEXT)
                            .prefix_text("Prefix")
                            .suffix_text("Suffix"),
                    )
                    .is_empty(true),
                );

                // Prefix and suffix are hidden.
                assert_eq!(get_opacity(tester, "Prefix"), 0.0);
                assert_eq!(get_opacity(tester, "Suffix"), 0.0);
            });
        }

        // input_decorator_test.dart 'Prefix and suffix are visible when decorator is empty and
        // floating behavior is FloatingBehavior.always'.
        #[test]
        fn the_prefix_and_suffix_show_when_the_decorator_is_empty_and_the_label_always_floats() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .filled(true)
                            .label_text(LABEL_TEXT)
                            .prefix_text("Prefix")
                            .suffix_text("Suffix")
                            .floating_label_behavior(FloatingLabelBehavior::Always),
                    )
                    .is_empty(true),
                );

                // Prefix and suffix are visible.
                assert_eq!(get_opacity(tester, "Prefix"), 1.0);
                assert_eq!(get_opacity(tester, "Suffix"), 1.0);
            });
        }

        fn long_labeled_input_decorator(
            floating_label_behavior: FloatingLabelBehavior,
            is_focused: bool,
        ) -> WidgetRef {
            const LONG_LABEL: &str = "Flutter is Google’s UI toolkit for building beautiful, natively compiled applications for mobile, web, and desktop from a single codebase.";
            MaterialApp::new()
                .home(
                    Scaffold::new().body(
                        SizedBox::new().width(300.0).child(
                            InputDecorator::new(
                                InputDecoration::new()
                                    .border(OutlineInputBorder::new().border_side(BorderSide {
                                        color: Colors::GREEN_ACCENT.primary(),
                                        ..BorderSide::default()
                                    }))
                                    .suffix_icon(icon(Icons::arrow_drop_down()))
                                    .floating_label_behavior(floating_label_behavior)
                                    .label_text(LONG_LABEL),
                            )
                            .is_empty(true)
                            .is_focused(is_focused)
                            .child(input_text()),
                        ),
                    ),
                )
                .into_widget()
        }

        // input_decorator_test.dart 'OutlineInputBorder and InputDecorator long labels and in
        // Floating, the width should ignore the icon width', on an empty `InputDecorator` that the
        // tap focuses.
        #[test]
        fn a_long_floating_outlined_label_is_wider_than_its_inline_width() {
            test_widgets(|tester| {
                tester.pump_widget(long_labeled_input_decorator(
                    FloatingLabelBehavior::Never,
                    false,
                ));

                let label_width = get_label_rect(tester).width();

                tester.pump_widget(long_labeled_input_decorator(
                    FloatingLabelBehavior::Always,
                    false,
                ));
                tester.pump_by(K_TRANSITION_DURATION);

                let floated_label_width = get_label_rect(tester).width();

                assert!(floated_label_width > label_width);

                tester.pump_widget(long_labeled_input_decorator(
                    FloatingLabelBehavior::Auto,
                    false,
                ));
                tester.pump_by(K_TRANSITION_DURATION);

                assert_eq!(get_label_rect(tester).width(), label_width);

                // Click for Focus.
                tester.pump_widget(long_labeled_input_decorator(
                    FloatingLabelBehavior::Auto,
                    true,
                ));
                // Default animation duration is 167ms.
                tester.pump_by(Duration::from_millis(80));

                assert!(get_label_rect(tester).width() > label_width);
                assert!(get_label_rect(tester).width() <= floated_label_width);

                tester.pump_by(K_TRANSITION_DURATION);

                assert_eq!(get_label_rect(tester).width(), floated_label_width);
            });
        }

        fn density_row(
            visual_density: VisualDensity,
            unconstrained: &GlobalKey,
            constrained: &GlobalKey,
        ) -> WidgetRef {
            let unconstrained = unconstrained.clone();
            let constrained = constrained.clone();
            MaterialApp::new()
                .home(Material::new().child(Builder::new(move |app, context| {
                    Theme::new(
                        Theme::of(app, context)
                            .copy_with()
                            .visual_density(visual_density)
                            .build(),
                        Center::new().child(
                            Row::new().children([
                                SizedBox::new()
                                    .width(35.0)
                                    .child(
                                        InputDecorator::new(InputDecoration::new())
                                            .key(unconstrained.clone().into())
                                            .child(input_text()),
                                    )
                                    .into_widget(),
                                SizedBox::new()
                                    .width(35.0)
                                    // 48 is the height that this TextField would take when
                                    // laid out with no constraints.
                                    .height(48.0)
                                    .child(
                                        InputDecorator::new(InputDecoration::new())
                                            .key(constrained.clone().into())
                                            .child(input_text()),
                                    )
                                    .into_widget(),
                            ]),
                        ),
                    )
                    .into_widget()
                })))
                .into_widget()
        }

        // input_decorator_test.dart 'given enough space, constrained and unconstrained heights
        // result in the same size widget', on `InputDecorator`s around the `TextField`'s input.
        #[test]
        fn constrained_and_unconstrained_heights_give_the_same_size_given_enough_space() {
            test_widgets(|tester| {
                let key_unconstrained = GlobalKey::new();
                let key_constrained = GlobalKey::new();
                tester.pump_widget(density_row(
                    VisualDensity::STANDARD,
                    &key_unconstrained,
                    &key_constrained,
                ));
                let constrained_height = key_size(tester, &key_constrained).height();
                let unconstrained_height = key_size(tester, &key_unconstrained).height();
                assert_eq!(constrained_height, unconstrained_height);

                tester.pump_widget(density_row(
                    VisualDensity::COMPACT,
                    &key_unconstrained,
                    &key_constrained,
                ));
                let constrained_height_compact = key_size(tester, &key_constrained).height();
                let unconstrained_height_compact = key_size(tester, &key_unconstrained).height();
                assert_eq!(constrained_height_compact, unconstrained_height_compact);
            });
        }

        // input_decorator_test.dart 'A vertically constrained TextField still positions its text
        // inside of itself', on an `InputDecorator` around the `TextField`'s input.
        #[test]
        fn a_vertically_constrained_decorator_positions_its_text_inside_itself() {
            test_widgets(|tester| {
                tester.pump_widget(
                    MaterialApp::new()
                        .home(
                            Material::new().child(
                                Center::new().child(
                                    SizedBox::new().width(200.0).height(28.0).child(
                                        InputDecorator::new(InputDecoration::new()).child(
                                            Text::new("A").style(
                                                TextStyle::new().font_size(16.0).height(1.5),
                                            ),
                                        ),
                                    ),
                                ),
                            ),
                        )
                        .into_widget(),
                );

                let field = get_decorator_rect(tester);
                let text = text_rect(tester, "A");

                // The text is inside the field.
                assert!(text.height() < field.bottom - field.top);
                assert!(text.top > field.top);
                assert!(text.top < field.bottom);
            });
        }

        // input_decorator_test.dart 'Visual density is included in the intrinsic height
        // calculation', on `InputDecorator`s around the `TextField`'s input.
        #[test]
        fn the_visual_density_is_included_in_the_intrinsic_height() {
            test_widgets(|tester| {
                let key = GlobalKey::new();
                let intrinsic_height_key = GlobalKey::new();
                let (k1, k2) = (key.clone(), intrinsic_height_key.clone());
                tester.pump_widget(
                    MaterialApp::new()
                        .home(Material::new().child(Builder::new(move |app, context| {
                            Theme::new(
                                Theme::of(app, context)
                                    .copy_with()
                                    .visual_density(VisualDensity::COMPACT)
                                    .build(),
                                Center::new().child(
                                    Row::new().children([
                                        SizedBox::new()
                                            .width(35.0)
                                            .child(
                                                InputDecorator::new(InputDecoration::new())
                                                    .key(k1.clone().into())
                                                    .child(input_text()),
                                            )
                                            .into_widget(),
                                        SizedBox::new()
                                            .width(35.0)
                                            .child(
                                                IntrinsicHeight::new().child(
                                                    InputDecorator::new(InputDecoration::new())
                                                        .key(k2.clone().into())
                                                        .child(input_text()),
                                                ),
                                            )
                                            .into_widget(),
                                    ]),
                                ),
                            )
                            .into_widget()
                        })))
                        .into_widget(),
                );

                let height = key_size(tester, &key).height();
                let intrinsic_height = key_size(tester, &intrinsic_height_key).height();
                assert_eq!(intrinsic_height, height);
            });
        }

        /// Dart's `SizedBox(width:, child: IntrinsicHeight(child: Column(children:
        /// [TextField(decoration:)])))` under `Center` in a `Material` app, on an
        /// `InputDecorator` around the `TextField`'s input.
        fn pump_intrinsic_column(
            tester: &mut WidgetTester,
            width: Option<f64>,
            decoration: InputDecoration,
        ) {
            let column = IntrinsicHeight::new().child(
                Column::new().children([InputDecorator::new(decoration)
                    .child(input_text())
                    .into_widget()]),
            );
            let body = match width {
                Some(width) => SizedBox::new().width(width).child(column).into_widget(),
                None => column.into_widget(),
            };
            tester.pump_widget(
                MaterialApp::new()
                    .home(Material::new().child(Center::new().child(body)))
                    .into_widget(),
            );
        }

        // input_decorator_test.dart 'Min intrinsic height for TextField with no content padding'.
        #[test]
        fn the_min_intrinsic_height_lays_out_with_no_content_padding() {
            test_widgets(|tester| {
                pump_intrinsic_column(
                    tester,
                    None,
                    InputDecoration::new()
                        .label_text("Label Text")
                        .helper_text("Helper Text")
                        .content_padding(EdgeInsets::ZERO.into()),
                );
            });
        }

        // input_decorator_test.dart 'Min intrinsic height for TextField with prefix icon'.
        #[test]
        fn the_min_intrinsic_height_lays_out_with_a_prefix_icon() {
            test_widgets(|tester| {
                pump_intrinsic_column(
                    tester,
                    Some(100.0),
                    InputDecoration::new().prefix_icon(icon(Icons::search())),
                );
            });
        }

        // input_decorator_test.dart 'Min intrinsic height for TextField with suffix icon'.
        #[test]
        fn the_min_intrinsic_height_lays_out_with_a_suffix_icon() {
            test_widgets(|tester| {
                pump_intrinsic_column(
                    tester,
                    Some(100.0),
                    InputDecoration::new().suffix_icon(icon(Icons::search())),
                );
            });
        }

        // input_decorator_test.dart 'Min intrinsic height for TextField with prefix'.
        #[test]
        fn the_min_intrinsic_height_lays_out_with_a_prefix() {
            test_widgets(|tester| {
                pump_intrinsic_column(
                    tester,
                    Some(100.0),
                    InputDecoration::new().prefix(Text::new("prefix")),
                );
            });
        }

        // input_decorator_test.dart 'Min intrinsic height for TextField with suffix'.
        #[test]
        fn the_min_intrinsic_height_lays_out_with_a_suffix() {
            test_widgets(|tester| {
                pump_intrinsic_column(
                    tester,
                    Some(100.0),
                    InputDecoration::new().suffix(Text::new("suffix")),
                );
            });
        }

        // input_decorator_test.dart 'Min intrinsic height for TextField with icon'.
        #[test]
        fn the_min_intrinsic_height_lays_out_with_an_icon() {
            test_widgets(|tester| {
                pump_intrinsic_column(
                    tester,
                    Some(100.0),
                    InputDecoration::new().icon(icon(Icons::search())),
                );
            });
        }

        fn helper_decoration(width: f64) -> WidgetRef {
            MaterialApp::new()
                .home(
                    Scaffold::new().body(
                        SizedBox::new().width(width).child(
                            IntrinsicHeight::new().child(InputDecorator::new(
                                InputDecoration::new()
                                    .filled(true)
                                    .label_text(LABEL_TEXT)
                                    .helper_text("0123456789")
                                    .helper_max_lines(2),
                            )),
                        ),
                    ),
                )
                .into_widget()
        }

        // input_decorator_test.dart 'Min intrinsic height calculation should not impact the
        // container height'.
        #[test]
        fn the_min_intrinsic_height_does_not_change_the_container_height() {
            test_widgets(|tester| {
                // Build once with enough width for the helper to fit in one line.
                tester.pump_widget(helper_decoration(300.0));
                let default_container_height = get_container_rect(tester).height();

                // Build once with a smaller width that will force the helper text to wrap.
                tester.pump_widget(helper_decoration(150.0));

                // The container height should be the same as before.
                assert_eq!(
                    get_container_rect(tester).height(),
                    default_container_height
                );
            });
        }

        /// The four decorations of Dart's 'Min intrinsic height matches the laid out height'
        /// group, with `hintText`.
        fn intrinsic_decoration(name: &str) -> InputDecoration {
            match name {
                "isDense and isCollapsed" => InputDecoration::new()
                    .is_dense(true)
                    .is_collapsed(true)
                    .content_padding(EdgeInsets::ZERO.into())
                    .border(InputBorderRef::none())
                    .hint_text(HINT_TEXT),
                "isDense with zero content padding" => InputDecoration::new()
                    .is_dense(true)
                    .content_padding(EdgeInsets::ZERO.into())
                    .hint_text(HINT_TEXT),
                "isCollapsed" => InputDecoration::new()
                    .is_collapsed(true)
                    .hint_text(HINT_TEXT),
                "InputDecoration.collapsed" => {
                    InputDecoration::collapsed(None).hint_text(HINT_TEXT)
                }
                _ => unreachable!("one of the group's decorations"),
            }
        }

        /// One case of Dart's 'Min intrinsic height matches the laid out height' group, on an
        /// empty `InputDecorator` around the `TextField`'s input.
        fn check_min_intrinsic_height_matches(
            tester: &mut WidgetTester,
            name: &str,
            visual_density: VisualDensity,
        ) {
            let decoration = intrinsic_decoration(name);
            let theme = ThemeData::builder()
                .visual_density(visual_density)
                .build(&tester.cell.borrow());
            tester.pump_widget(crate::test_support::material_app(
                theme,
                crate::material::Material::new()
                    .child(
                        Center::new().child(
                            SizedBox::new().width(200.0).child(
                                InputDecorator::new(decoration)
                                    .is_empty(true)
                                    .child(input_text()),
                            ),
                        ),
                    )
                    .into_widget(),
            ));

            let decorator = decorator_render(tester);
            let mut app = tester.cell.borrow_mut();
            let height = decorator.size(&app).height();
            assert_eq!(decorator.get_min_intrinsic_height(&mut app, 200.0), height);
            assert_eq!(decorator.get_max_intrinsic_height(&mut app, 200.0), height);
        }

        // input_decorator_test.dart 'Min intrinsic height matches the laid out height - isDense
        // and isCollapsed with standard visual density'.
        #[test]
        fn the_min_intrinsic_height_matches_the_layout_dense_and_collapsed_at_standard_density() {
            test_widgets(|tester| {
                check_min_intrinsic_height_matches(
                    tester,
                    "isDense and isCollapsed",
                    VisualDensity::STANDARD,
                )
            });
        }

        // input_decorator_test.dart 'Min intrinsic height matches the laid out height - isDense
        // with zero content padding with standard visual density'.
        #[test]
        fn the_min_intrinsic_height_matches_the_layout_dense_without_padding_at_standard_density() {
            test_widgets(|tester| {
                check_min_intrinsic_height_matches(
                    tester,
                    "isDense with zero content padding",
                    VisualDensity::STANDARD,
                )
            });
        }

        // input_decorator_test.dart 'Min intrinsic height matches the laid out height -
        // isCollapsed with standard visual density'.
        #[test]
        fn the_min_intrinsic_height_matches_the_layout_collapsed_at_standard_density() {
            test_widgets(|tester| {
                check_min_intrinsic_height_matches(tester, "isCollapsed", VisualDensity::STANDARD)
            });
        }

        // input_decorator_test.dart 'Min intrinsic height matches the laid out height -
        // InputDecoration.collapsed with standard visual density'.
        #[test]
        fn the_min_intrinsic_height_matches_the_layout_of_a_collapsed_decoration_at_standard_density()
         {
            test_widgets(|tester| {
                check_min_intrinsic_height_matches(
                    tester,
                    "InputDecoration.collapsed",
                    VisualDensity::STANDARD,
                )
            });
        }

        // input_decorator_test.dart 'Min intrinsic height matches the laid out height - isDense
        // and isCollapsed with compact visual density'.
        #[test]
        fn the_min_intrinsic_height_matches_the_layout_dense_and_collapsed_at_compact_density() {
            test_widgets(|tester| {
                check_min_intrinsic_height_matches(
                    tester,
                    "isDense and isCollapsed",
                    VisualDensity::COMPACT,
                )
            });
        }

        // input_decorator_test.dart 'Min intrinsic height matches the laid out height - isDense
        // with zero content padding with compact visual density'.
        #[test]
        fn the_min_intrinsic_height_matches_the_layout_dense_without_padding_at_compact_density() {
            test_widgets(|tester| {
                check_min_intrinsic_height_matches(
                    tester,
                    "isDense with zero content padding",
                    VisualDensity::COMPACT,
                )
            });
        }

        // input_decorator_test.dart 'Min intrinsic height matches the laid out height -
        // isCollapsed with compact visual density'.
        #[test]
        fn the_min_intrinsic_height_matches_the_layout_collapsed_at_compact_density() {
            test_widgets(|tester| {
                check_min_intrinsic_height_matches(tester, "isCollapsed", VisualDensity::COMPACT)
            });
        }

        // input_decorator_test.dart 'Min intrinsic height matches the laid out height -
        // InputDecoration.collapsed with compact visual density'.
        #[test]
        fn the_min_intrinsic_height_matches_the_layout_of_a_collapsed_decoration_at_compact_density()
         {
            test_widgets(|tester| {
                check_min_intrinsic_height_matches(
                    tester,
                    "InputDecoration.collapsed",
                    VisualDensity::COMPACT,
                )
            });
        }

        // input_decorator_test.dart 'Min intrinsic height matches the laid out height -
        // IntrinsicHeight does not overflow with a dense collapsed TextField', on an empty
        // `InputDecorator` around the `TextField`'s input.
        #[test]
        fn an_intrinsic_height_does_not_overflow_with_a_dense_collapsed_decorator() {
            test_widgets(|tester| {
                let theme = ThemeData::builder()
                    .visual_density(VisualDensity::COMPACT)
                    .build(&tester.cell.borrow());
                tester.pump_widget(crate::test_support::material_app(
                    theme,
                    Scaffold::new()
                        .body(
                            IntrinsicHeight::new().child(
                                Row::new()
                                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                                    .children([
                                        Expanded::new(
                                            Column::new()
                                                .main_axis_size(MainAxisSize::Min)
                                                .children([
                                                    Text::new("Text").into_widget(),
                                                    InputDecorator::new(
                                                        InputDecoration::new()
                                                            .is_dense(true)
                                                            .is_collapsed(true)
                                                            .content_padding(
                                                                EdgeInsets::ZERO.into(),
                                                            )
                                                            .border(InputBorderRef::none())
                                                            .hint_text(HINT_TEXT),
                                                    )
                                                    .is_empty(true)
                                                    .child(input_text())
                                                    .into_widget(),
                                                ]),
                                        )
                                        .into_widget(),
                                        Text::new("Right text").into_widget(),
                                    ]),
                            ),
                        )
                        .into_widget(),
                ));
            });
        }

        fn intrinsic_padding() -> EdgeInsetsGeometry {
            EdgeInsetsDirectional::only(12.0, 0.0, 24.0, 0.0).into()
        }

        /// Dart's `measureText`: the size of the input text in an intrinsic-width decorator.
        fn measure_text(
            tester: &mut WidgetTester,
            decoration: InputDecoration,
            direction: TextDirection,
        ) -> Size {
            tester.pump_widget(
                BuildInputDecorator::new(decoration)
                    .use_intrinsic_width(true)
                    .text_direction(direction)
                    .build(),
            );
            let found = tester.find_text(INPUT_TEXT)[0];
            tester.render_box_of(found).size(&tester.cell.borrow())
        }

        fn without_icons() -> InputDecoration {
            InputDecoration::new().content_padding(intrinsic_padding())
        }

        fn with_prefix() -> InputDecoration {
            without_icons().prefix_icon(icon(Icons::search()))
        }

        fn with_suffix() -> InputDecoration {
            without_icons().suffix_icon(icon(Icons::search()))
        }

        fn with_affixes() -> InputDecoration {
            without_icons()
                .prefix_icon(icon(Icons::search()))
                .suffix_icon(icon(Icons::search()))
        }

        // input_decorator_test.dart 'Intrinsic width - with prefixIcon in LTR'.
        #[test]
        fn the_intrinsic_width_keeps_the_text_width_with_a_prefix_icon_ltr() {
            test_widgets(|tester| {
                assert_eq!(
                    measure_text(tester, with_prefix(), TextDirection::Ltr).width(),
                    measure_text(tester, without_icons(), TextDirection::Ltr).width()
                );
            });
        }

        // input_decorator_test.dart 'Intrinsic width - with suffixIcon in LTR'.
        #[test]
        fn the_intrinsic_width_keeps_the_text_width_with_a_suffix_icon_ltr() {
            test_widgets(|tester| {
                assert_eq!(
                    measure_text(tester, with_suffix(), TextDirection::Ltr).width(),
                    measure_text(tester, without_icons(), TextDirection::Ltr).width()
                );
            });
        }

        // input_decorator_test.dart 'Intrinsic width - with prefixIcon and suffixIcon in LTR'.
        #[test]
        fn the_intrinsic_width_keeps_the_text_width_with_both_icons_ltr() {
            test_widgets(|tester| {
                assert_eq!(
                    measure_text(tester, with_affixes(), TextDirection::Ltr).width(),
                    measure_text(tester, without_icons(), TextDirection::Ltr).width()
                );
            });
        }

        // input_decorator_test.dart 'Intrinsic width - with prefixIcon in RTL'.
        #[test]
        fn the_intrinsic_width_keeps_the_text_width_with_a_prefix_icon_rtl() {
            test_widgets(|tester| {
                assert_eq!(
                    measure_text(tester, with_prefix(), TextDirection::Rtl).width(),
                    measure_text(tester, without_icons(), TextDirection::Rtl).width()
                );
            });
        }

        // input_decorator_test.dart 'Intrinsic width - with suffixIcon in RTL'.
        #[test]
        fn the_intrinsic_width_keeps_the_text_width_with_a_suffix_icon_rtl() {
            test_widgets(|tester| {
                assert_eq!(
                    measure_text(tester, with_suffix(), TextDirection::Rtl).width(),
                    measure_text(tester, without_icons(), TextDirection::Rtl).width()
                );
            });
        }

        // input_decorator_test.dart 'Intrinsic width - with prefixIcon and suffixIcon in RTL'.
        #[test]
        fn the_intrinsic_width_keeps_the_text_width_with_both_icons_rtl() {
            test_widgets(|tester| {
                assert_eq!(
                    measure_text(tester, with_affixes(), TextDirection::Rtl).width(),
                    measure_text(tester, without_icons(), TextDirection::Rtl).width()
                );
            });
        }

        fn intrinsic_width_of(
            tester: &mut WidgetTester,
            decoration: InputDecoration,
            is_empty: bool,
            child: WidgetRef,
        ) -> f64 {
            pump(
                tester,
                BuildInputDecorator::new(decoration)
                    .use_intrinsic_width(true)
                    .is_empty(is_empty)
                    .child(child),
            );
            get_decorator_rect(tester).width()
        }

        fn sized_width(width: f64) -> WidgetRef {
            SizedBox::new().width(width).into_widget()
        }

        // input_decorator_test.dart 'Intrinsic width - depends on hint width and content width
        // when decorator is empty'.
        #[test]
        fn the_intrinsic_width_of_an_empty_decorator_is_the_wider_of_hint_and_content() {
            test_widgets(|tester| {
                let decoration_with_hint = || {
                    InputDecoration::new()
                        .content_padding(EdgeInsets::ZERO.into())
                        .hint_text("Hint")
                };
                const HINT_TEXT_WIDTH: f64 = 66.0;
                const SMALL_CONTENT_WIDTH: f64 = 20.0;
                const LARGE_CONTENT_WIDTH: f64 = 80.0;

                // Decorator width depends on the hint because the hint is larger than the content.
                assert_eq!(
                    intrinsic_width_of(
                        tester,
                        decoration_with_hint(),
                        true,
                        sized_width(SMALL_CONTENT_WIDTH)
                    ),
                    HINT_TEXT_WIDTH
                );

                // Decorator width depends on the content because the content is larger than the hint.
                assert_eq!(
                    intrinsic_width_of(
                        tester,
                        decoration_with_hint(),
                        true,
                        sized_width(LARGE_CONTENT_WIDTH)
                    ),
                    LARGE_CONTENT_WIDTH
                );
            });
        }

        // input_decorator_test.dart 'Intrinsic width - depends on content width when decorator is
        // not empty and maintainHintSize is false'.
        #[test]
        fn the_intrinsic_width_ignores_the_hint_when_not_empty_and_not_maintaining_its_size() {
            test_widgets(|tester| {
                let decoration_with_hint = InputDecoration::new()
                    .content_padding(EdgeInsets::ZERO.into())
                    .hint_text("Hint")
                    .maintain_hint_size(false);
                const CONTENT_WIDTH: f64 = 20.0;

                // The hint width is ignored even if larger than the content width.
                assert_eq!(
                    intrinsic_width_of(
                        tester,
                        decoration_with_hint,
                        false,
                        sized_width(CONTENT_WIDTH)
                    ),
                    CONTENT_WIDTH
                );
            });
        }

        // input_decorator_test.dart 'Intrinsic width - depends on hint width when decorator is not
        // empty and maintainHintSize is true'.
        #[test]
        fn the_intrinsic_width_keeps_the_hint_width_when_maintaining_its_size() {
            test_widgets(|tester| {
                let decoration_with_hint = InputDecoration::new()
                    .content_padding(EdgeInsets::ZERO.into())
                    .hint_text("Hint");
                const CONTENT_WIDTH: f64 = 20.0;
                const HINT_TEXT_WIDTH: f64 = 66.0;
                assert_eq!(
                    intrinsic_width_of(
                        tester,
                        decoration_with_hint,
                        false,
                        sized_width(CONTENT_WIDTH)
                    ),
                    HINT_TEXT_WIDTH
                );
            });
        }

        fn decoration_with_label(maintain_label_size: bool) -> InputDecoration {
            InputDecoration::new()
                .content_padding(EdgeInsets::ZERO.into())
                .label(SizedBox::new().width(30.0))
                .maintain_label_size(maintain_label_size)
        }

        // input_decorator_test.dart 'Intrinsic width - does not depend on label width when
        // decorator is empty and maintainLabelSize is false'.
        #[test]
        fn the_intrinsic_width_of_an_empty_decorator_ignores_the_label() {
            test_widgets(|tester| {
                // The label width is ignored even if larger than the content width.
                assert_eq!(
                    intrinsic_width_of(
                        tester,
                        decoration_with_label(false),
                        true,
                        SizedBox::shrink().into_widget()
                    ),
                    0.0
                );
            });
        }

        // input_decorator_test.dart 'Intrinsic width - depends on label width when decorator is
        // empty and maintainLabelSize is true'.
        #[test]
        fn the_intrinsic_width_of_an_empty_decorator_keeps_the_label_when_maintaining_its_size() {
            test_widgets(|tester| {
                assert_eq!(
                    intrinsic_width_of(
                        tester,
                        decoration_with_label(true),
                        true,
                        SizedBox::shrink().into_widget()
                    ),
                    30.0
                );
            });
        }

        // input_decorator_test.dart 'Intrinsic width - does not depend on label width when
        // decorator is not empty and maintainLabelSize is false'.
        #[test]
        fn the_intrinsic_width_ignores_the_label() {
            test_widgets(|tester| {
                // The label width is ignored even if larger than the content width.
                assert_eq!(
                    intrinsic_width_of(
                        tester,
                        decoration_with_label(false),
                        false,
                        sized_width(20.0)
                    ),
                    20.0
                );
            });
        }

        // input_decorator_test.dart 'Intrinsic width - depends on label width when decorator is
        // not empty and maintainLabelSize is true'.
        #[test]
        fn the_intrinsic_width_keeps_the_label_when_maintaining_its_size() {
            test_widgets(|tester| {
                assert_eq!(
                    intrinsic_width_of(
                        tester,
                        decoration_with_label(true),
                        false,
                        sized_width(20.0)
                    ),
                    30.0
                );
            });
        }

        // input_decorator_test.dart 'Ensure the height of labelStyle remains unchanged when
        // TextField is focused', on an empty `InputDecorator` that the focus request focuses.
        #[test]
        fn the_label_style_height_is_unchanged_when_focused() {
            test_widgets(|tester| {
                let build = |is_focused: bool| {
                    MaterialApp::new()
                        .home(
                            Material::new().child(
                                InputDecorator::new(InputDecoration::new().label_text("label"))
                                    .is_empty(true)
                                    .is_focused(is_focused)
                                    .child(input_text()),
                            ),
                        )
                        .into_widget()
                };
                tester.pump_widget(build(false));
                let before_style = get_label_style(tester);
                // Focused.
                tester.pump_widget(build(true));
                tester.pump_by(K_TRANSITION_DURATION);

                assert_eq!(get_label_style(tester).height, before_style.height);
            });
        }

        fn local(
            theme: InputDecorationThemeData,
            decoration: InputDecoration,
        ) -> BuildInputDecorator {
            BuildInputDecorator::new(decoration).local_input_decoration_theme(theme)
        }

        fn indigo() -> Color {
            Colors::INDIGO.primary()
        }

        fn indigo_style() -> TextStyle {
            TextStyle::new().color(indigo())
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default - labelStyle'.
        #[test]
        fn a_local_theme_label_style_overrides_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().label_style(indigo_style()),
                        InputDecoration::new().label_text(LABEL_TEXT),
                    ),
                );
                assert_eq!(style_color(&get_label_style(tester)), Some(indigo()));
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default -
        // floatingLabelStyle'.
        #[test]
        fn a_local_theme_floating_label_style_overrides_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().floating_label_style(indigo_style()),
                        InputDecoration::new().label_text(LABEL_TEXT),
                    )
                    .is_focused(true), // Label appears floating above input field.
                );
                assert_eq!(style_color(&get_label_style(tester)), Some(indigo()));
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default - helperStyle'.
        #[test]
        fn a_local_theme_helper_style_overrides_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().helper_style(indigo_style()),
                        InputDecoration::new()
                            .label_text(LABEL_TEXT)
                            .helper_text(HELPER_TEXT),
                    ),
                );
                assert_eq!(
                    style_color(&rich_text_style(tester, HELPER_TEXT)),
                    Some(indigo())
                );
            });
        }

        /// Dart's `tester.getRect(find.text(text))` of a `Text`.
        fn string_rect(tester: &WidgetTester, text: &str) -> waft_embedder::Rect {
            let found = *tester
                .find_text(text)
                .first()
                .expect("a Text showing the text");
            rect_of(tester, found)
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default -
        // helperMaxLines'.
        #[test]
        fn a_local_theme_helper_max_lines_overrides_the_default() {
            test_widgets(|tester| {
                const HELPER_MAX_LINES: i32 = 2;
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().helper_max_lines(HELPER_MAX_LINES),
                        InputDecoration::new()
                            .label_text(LABEL_TEXT)
                            .helper_text(THREE_LINES),
                    ),
                );

                const HELPER_GAP: f64 = 4.0;
                const HELPER_HEIGHT: f64 = 16.0;
                const CONTAINER_HEIGHT: f64 = 56.0;
                let helper_rect = string_rect(tester, THREE_LINES);
                assert!(
                    (helper_rect.height() - HELPER_HEIGHT * f64::from(HELPER_MAX_LINES)).abs()
                        <= 0.25,
                    "{helper_rect:?}"
                );
                let height = get_decorator_rect(tester).height();
                assert!(
                    (height
                        - (CONTAINER_HEIGHT
                            + HELPER_GAP
                            + HELPER_HEIGHT * f64::from(HELPER_MAX_LINES)))
                    .abs()
                        <= 0.25,
                    "{height}"
                );
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default - hintStyle'.
        #[test]
        fn a_local_theme_hint_style_overrides_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().hint_style(indigo_style()),
                        InputDecoration::new()
                            .label_text(LABEL_TEXT)
                            .hint_text(HINT_TEXT),
                    ),
                );
                assert_eq!(
                    style_color(&rich_text_style(tester, HINT_TEXT)),
                    Some(indigo())
                );
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default -
        // hintFadeDuration'.
        #[test]
        fn a_local_theme_hint_fade_duration_overrides_the_default() {
            test_widgets(|tester| {
                const HINT_FADE_DURATION: Duration = Duration::from_millis(404);
                let build = |is_empty: bool| {
                    local(
                        InputDecorationThemeData::new().hint_fade_duration(HINT_FADE_DURATION),
                        InputDecoration::new().hint_text(HINT_TEXT),
                    )
                    .is_empty(is_empty)
                    .build()
                };

                // Build once with empty content.
                tester.pump_widget(build(true));

                // Hint is visible (opacity 1.0).
                assert_eq!(get_opacity(tester, HINT_TEXT), 1.0);

                // Rebuild with non-empty content.
                tester.pump_widget(build(false));

                // The hint's opacity animates from 1.0 to 0.0.
                // The animation's default duration is 20ms.
                tester.pump_by(Duration::from_millis(200));
                let hint_opacity_200ms = get_opacity(tester, HINT_TEXT);
                assert!(hint_opacity_200ms > 0.0 && hint_opacity_200ms < 1.0);
                tester.pump_by(Duration::from_millis(200));
                let hint_opacity_400ms = get_opacity(tester, HINT_TEXT);
                assert!(hint_opacity_400ms > 0.0 && hint_opacity_400ms < hint_opacity_200ms);
                tester.pump_by(Duration::from_millis(4));
                assert_eq!(get_opacity(tester, HINT_TEXT), 0.0);
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default - hintMaxLines'.
        #[test]
        fn a_local_theme_hint_max_lines_overrides_the_default() {
            test_widgets(|tester| {
                const HINT_MAX_LINES: i32 = 2;
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().hint_max_lines(HINT_MAX_LINES),
                        InputDecoration::collapsed(Some(THREE_LINES.to_owned())),
                    ),
                );

                const HINT_LINE_HEIGHT: f64 = 24.0; // Font size = 16 and font height = 1.5.
                assert_eq!(
                    get_decorator_rect(tester).size(),
                    Size::new(800.0, f64::from(HINT_MAX_LINES) * HINT_LINE_HEIGHT)
                );
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default - errorStyle'.
        #[test]
        fn a_local_theme_error_style_overrides_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().error_style(indigo_style()),
                        InputDecoration::new()
                            .label_text(LABEL_TEXT)
                            .error_text(ERROR_TEXT),
                    )
                    .is_focused(true),
                );
                assert_eq!(
                    style_color(&rich_text_style(tester, ERROR_TEXT)),
                    Some(indigo())
                );
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default -
        // errorMaxLines'.
        #[test]
        fn a_local_theme_error_max_lines_overrides_the_default() {
            test_widgets(|tester| {
                const ERROR_MAX_LINES: i32 = 2;
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().error_max_lines(ERROR_MAX_LINES),
                        InputDecoration::new()
                            .error_text(THREE_LINES)
                            .label_text("label")
                            .filled(true),
                    ),
                );

                const HELPER_GAP: f64 = 4.0;
                const CONTAINER_HEIGHT: f64 = 56.0;
                const ERROR_HEIGHT: f64 = 16.0;

                let error_rect = string_rect(tester, THREE_LINES);
                assert!(
                    (error_rect.height() - ERROR_HEIGHT * f64::from(ERROR_MAX_LINES)).abs() <= 0.25,
                    "{error_rect:?}"
                );
                let height = get_decorator_rect(tester).height();
                assert!(
                    (height
                        - (CONTAINER_HEIGHT
                            + HELPER_GAP
                            + ERROR_HEIGHT * f64::from(ERROR_MAX_LINES)))
                    .abs()
                        <= 0.25,
                    "{height}"
                );
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default -
        // floatingLabelBehavior'.
        #[test]
        fn a_local_theme_floating_label_behavior_overrides_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new()
                            .floating_label_behavior(FloatingLabelBehavior::Never),
                        InputDecoration::new().label_text(LABEL_TEXT),
                    ),
                );

                assert_eq!(get_decorator_rect(tester).size(), Size::new(800.0, 56.0));
                // Label line height is forced to 1.0 and font size is 16.0,
                // the label should be vertically centered (20 pixels above and below).
                assert_eq!(get_label_rect(tester).top, 20.0);
                assert_eq!(get_label_rect(tester).bottom, 36.0);
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default -
        // floatingLabelAlignment', through Dart's `buildInputDecoratorWithFloatingLabel`.
        #[test]
        fn a_local_theme_floating_label_alignment_overrides_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .content_padding(
                                EdgeInsetsDirectional::only(40.0, 12.0, 0.0, 12.0).into(),
                            )
                            .icon(icon(Icons::insert_link()))
                            .label_text(LABEL_TEXT)
                            .hint_text(HINT_TEXT)
                            .filled(true),
                    )
                    .local_input_decoration_theme(
                        InputDecorationThemeData::new()
                            .floating_label_alignment(FloatingLabelAlignment::CENTER),
                    ),
                );
                // icon (40) + (decorator (800) - icon (40)) / 2
                assert_eq!(get_label_rect(tester).center().dx(), 420.0);
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default - isDense'.
        #[test]
        fn a_local_theme_is_dense_overrides_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().is_dense(true),
                        InputDecoration::new().label_text(LABEL_TEXT),
                    ),
                );
                assert_eq!(get_container_rect(tester).height(), 48.0);
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default -
        // contentPadding'.
        #[test]
        fn a_local_theme_content_padding_overrides_the_default() {
            test_widgets(|tester| {
                const START: f64 = 11.0;
                const TOP: f64 = 13.0;
                const END: f64 = 15.0;
                const BOTTOM: f64 = 17.0;

                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().content_padding(
                            EdgeInsetsDirectional::only(START, TOP, END, BOTTOM).into(),
                        ),
                        InputDecoration::new().label_text(LABEL_TEXT),
                    ),
                );

                const LABEL_HEIGHT: f64 = 16.0;
                const INPUT_HEIGHT: f64 = 24.0;
                assert_eq!(
                    get_container_rect(tester).height(),
                    TOP + LABEL_HEIGHT + INPUT_HEIGHT + BOTTOM
                );
                assert_eq!(text_rect(tester, INPUT_TEXT).left, START);
                assert_eq!(text_rect(tester, INPUT_TEXT).right, 800.0 - END);
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default - isCollapsed'.
        #[test]
        fn a_local_theme_is_collapsed_overrides_the_default() {
            test_widgets(|tester| {
                // Overall height for a collapsed InputDecorator is 24dp which is the input
                // height (font size = 16, line height = 1.5).
                const INPUT_HEIGHT: f64 = 24.0;

                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().is_collapsed(true),
                        InputDecoration::new().label_text(LABEL_TEXT),
                    )
                    .is_focused(true),
                );

                assert_eq!(
                    get_decorator_rect(tester).size(),
                    Size::new(800.0, INPUT_HEIGHT)
                );
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default - iconColor'.
        #[test]
        fn a_local_theme_icon_color_overrides_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().icon_color(indigo()),
                        InputDecoration::new().icon(icon(Icons::cabin())),
                    ),
                );
                assert_eq!(icon_theme_color(tester, &Icons::cabin()), Some(indigo()));
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default - prefixStyle'.
        #[test]
        fn a_local_theme_prefix_style_overrides_the_default() {
            test_widgets(|tester| {
                const PREFIX_LABEL: &str = "prefix";
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().prefix_style(indigo_style()),
                        InputDecoration::new().prefix(Text::new(PREFIX_LABEL)),
                    )
                    .is_focused(true),
                );
                assert_eq!(
                    style_color(&rich_text_style(tester, PREFIX_LABEL)),
                    Some(indigo())
                );
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default -
        // prefixIconColor'.
        #[test]
        fn a_local_theme_prefix_icon_color_overrides_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().prefix_icon_color(indigo()),
                        InputDecoration::new().prefix_icon(icon(Icons::cabin())),
                    ),
                );
                assert_eq!(icon_theme_color(tester, &Icons::cabin()), Some(indigo()));
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default -
        // prefixIconConstraints'.
        #[test]
        fn a_local_theme_prefix_icon_constraints_override_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().prefix_icon_constraints(BoxConstraints {
                            min_width: 60.0,
                            min_height: 60.0,
                            ..BoxConstraints::new()
                        }),
                        InputDecoration::new().prefix_icon(icon(Icons::cabin())),
                    ),
                );
                assert_eq!(icon_size(tester, &Icons::cabin()), Size::new(60.0, 60.0));
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default - suffixStyle'.
        #[test]
        fn a_local_theme_suffix_style_overrides_the_default() {
            test_widgets(|tester| {
                const SUFFIX_LABEL: &str = "suffix";
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().suffix_style(indigo_style()),
                        InputDecoration::new().suffix(Text::new(SUFFIX_LABEL)),
                    )
                    .is_focused(true),
                );
                assert_eq!(
                    style_color(&rich_text_style(tester, SUFFIX_LABEL)),
                    Some(indigo())
                );
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default -
        // suffixIconColor'.
        #[test]
        fn a_local_theme_suffix_icon_color_overrides_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().suffix_icon_color(indigo()),
                        InputDecoration::new().suffix_icon(icon(Icons::cabin())),
                    ),
                );
                assert_eq!(icon_theme_color(tester, &Icons::cabin()), Some(indigo()));
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default -
        // suffixIconConstraints'.
        #[test]
        fn a_local_theme_suffix_icon_constraints_override_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().suffix_icon_constraints(BoxConstraints {
                            min_width: 60.0,
                            min_height: 60.0,
                            ..BoxConstraints::new()
                        }),
                        InputDecoration::new().suffix_icon(icon(Icons::cabin())),
                    ),
                );
                assert_eq!(icon_size(tester, &Icons::cabin()), Size::new(60.0, 60.0));
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default -
        // counterStyle'.
        #[test]
        fn a_local_theme_counter_style_overrides_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().counter_style(indigo_style()),
                        InputDecoration::new()
                            .label_text(LABEL_TEXT)
                            .counter_text(COUNTER_TEXT),
                    ),
                );
                assert_eq!(
                    style_color(&rich_text_style(tester, COUNTER_TEXT)),
                    Some(indigo())
                );
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default - filled and
        // fillColor'.
        #[test]
        fn a_local_theme_filled_and_fill_color_override_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new()
                            .filled(true)
                            .fill_color(indigo()),
                        InputDecoration::new()
                            .label_text(LABEL_TEXT)
                            .counter_text(COUNTER_TEXT),
                    ),
                );
                assert!(
                    paints_fill(tester, indigo()),
                    "{:?}",
                    border_painted(tester)
                );
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default -
        // activeIndicatorBorder'.
        #[test]
        fn a_local_theme_active_indicator_border_overrides_the_default() {
            test_widgets(|tester| {
                let active_indicator_border = BorderSide {
                    color: Colors::AMBER.primary(),
                    width: 2.0,
                    ..BorderSide::default()
                };
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new()
                            .filled(true)
                            .active_indicator_border(active_indicator_border),
                        InputDecoration::new().label_text(LABEL_TEXT),
                    ),
                );

                assert_eq!(get_border_color(tester), active_indicator_border.color);
                assert_eq!(get_border_weight(tester), active_indicator_border.width);
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default - hoverColor'.
        #[test]
        fn a_local_theme_hover_color_overrides_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new()
                            .filled(true)
                            .hover_color(indigo()),
                        InputDecoration::new()
                            .label_text(LABEL_TEXT)
                            .helper_text(HELPER_TEXT),
                    )
                    .is_focused(true)
                    .is_hovering(true),
                );

                let theme = {
                    let decorator = tester.find_by_type::<InputDecorator>()[0];
                    let mut app = tester.cell.borrow_mut();
                    Theme::of(&mut app, decorator)
                };
                let focus_color = theme.color_scheme.surface_container_highest();
                assert!(
                    paints_fill(tester, Color::alpha_blend(indigo(), focus_color)),
                    "{:?}",
                    border_painted(tester)
                );
            });
        }

        fn settled(tester: &mut WidgetTester, decorator: BuildInputDecorator) {
            pump(tester, decorator);
            tester.pump_and_settle(); // Border changes are animated.
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default - errorBorder'.
        #[test]
        fn a_local_theme_error_border_overrides_the_default() {
            test_widgets(|tester| {
                let error_border = outline(Colors::RED.primary(), 1.5);
                settled(
                    tester,
                    local(
                        InputDecorationThemeData::new().error_border(error_border),
                        InputDecoration::new().error_text("error"),
                    ),
                );
                assert!(border_is(tester, &error_border));
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default -
        // focusedErrorBorder'.
        #[test]
        fn a_local_theme_focused_error_border_overrides_the_default() {
            test_widgets(|tester| {
                let focused_error_border = outline(Colors::RED.primary(), 1.5);
                settled(
                    tester,
                    local(
                        InputDecorationThemeData::new().focused_error_border(focused_error_border),
                        InputDecoration::new().error_text("error"),
                    )
                    .is_focused(true),
                );
                assert!(border_is(tester, &focused_error_border));
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default -
        // focusedBorder'.
        #[test]
        fn a_local_theme_focused_border_overrides_the_default() {
            test_widgets(|tester| {
                let focused_border = outline(Colors::GREEN.primary(), 1.5);
                settled(
                    tester,
                    local(
                        InputDecorationThemeData::new().focused_border(focused_border),
                        InputDecoration::new(),
                    )
                    .is_focused(true),
                );
                assert!(border_is(tester, &focused_border));
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default -
        // enabledBorder'.
        #[test]
        fn a_local_theme_enabled_border_overrides_the_default() {
            test_widgets(|tester| {
                let enabled_border = outline(Colors::TEAL.primary(), 5.0);
                settled(
                    tester,
                    local(
                        InputDecorationThemeData::new().enabled_border(enabled_border),
                        InputDecoration::new(),
                    ),
                );
                assert!(border_is(tester, &enabled_border));
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default -
        // disabledBorder'.
        #[test]
        fn a_local_theme_disabled_border_overrides_the_default() {
            test_widgets(|tester| {
                let disabled_border = outline(Colors::TEAL.primary(), 3.0);
                settled(
                    tester,
                    local(
                        InputDecorationThemeData::new().disabled_border(disabled_border),
                        InputDecoration::new().enabled(false),
                    ),
                );
                assert!(border_is(tester, &disabled_border));
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default - border'.
        #[test]
        fn a_local_theme_border_overrides_the_default() {
            test_widgets(|tester| {
                let border_radius = BorderRadius::all(Radius::circular(6.0));
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new()
                            .border(OutlineInputBorder::new().border_radius(border_radius)),
                        InputDecoration::new(),
                    ),
                );

                // The real instance of border is created based on the given border.
                // The type and the borderRadius should be the same as the given border.
                assert!(
                    get_border(tester)
                        .expect("a border")
                        .as_any()
                        .is::<OutlineInputBorder>()
                );
                assert_eq!(get_border_radius(tester), Some(border_radius));
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default -
        // alignLabelWithHint'.
        #[test]
        fn a_local_theme_align_label_with_hint_overrides_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().align_label_with_hint(true),
                        InputDecoration::new()
                            .prefix_icon(icon(Icons::ac_unit()))
                            .label_text(LABEL_TEXT)
                            .border(OutlineInputBorder::new()),
                    )
                    .is_focused(true),
                );
                assert_eq!(
                    get_label_rect(tester).left,
                    text_rect(tester, INPUT_TEXT).left
                );
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default - constraints'.
        #[test]
        fn local_theme_constraints_override_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new().constraints(BoxConstraints {
                            max_width: 300.0,
                            max_height: 40.0,
                            ..BoxConstraints::new()
                        }),
                        InputDecoration::new().label_text(LABEL_TEXT),
                    ),
                );

                // Theme settings should make it 300x40 pixels.
                assert_eq!(get_decorator_rect(tester).size(), Size::new(300.0, 40.0));
            });
        }

        // input_decorator_test.dart 'Local InputDecorationTheme overrides default -
        // visualDensity'.
        #[test]
        fn a_local_theme_visual_density_overrides_the_default() {
            test_widgets(|tester| {
                pump(
                    tester,
                    local(
                        InputDecorationThemeData::new()
                            .visual_density(VisualDensity::new(2.0, 2.0)),
                        InputDecoration::new()
                            .label_text(LABEL_TEXT)
                            .hint_text(HINT_TEXT),
                    )
                    .is_empty(true),
                );

                // Overall height for this InputDecorator is 64dp:
                //   12 - top padding (8 plus 4 due to increased visual density)
                //   12 - floating label (font size = 16 * 0.75, line height is forced to 1.0)
                //    4 - gap between label and input (this is not part of the M3 spec)
                //   24 - input text (font size = 16, line height = 1.5)
                //   12 - bottom padding (8 plus 4 due to increased visual density)
                assert_eq!(get_decorator_rect(tester).size().height(), 64.0);
            });
        }

        // input_decorator_test.dart 'Helper and counter are correctly styled'.
        #[test]
        fn the_focused_filled_helper_and_counter_are_styled_on_surface_variant() {
            test_widgets(|tester| {
                pump(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .filled(true)
                            .label_text(LABEL_TEXT)
                            .helper_text(HELPER_TEXT)
                            .counter_text(COUNTER_TEXT),
                    )
                    .is_focused(true),
                );

                let theme = {
                    let decorator = tester.find_by_type::<InputDecorator>()[0];
                    let mut app = tester.cell.borrow_mut();
                    Theme::of(&mut app, decorator)
                };
                let expected_color = theme.color_scheme.on_surface_variant();
                let expected_style = theme
                    .text_theme
                    .body_small
                    .clone()
                    .expect("bodySmall")
                    .copy_with()
                    .color(expected_color);
                assert_eq!(rich_text_style(tester, HELPER_TEXT), expected_style);
                assert_eq!(rich_text_style(tester, COUNTER_TEXT), expected_style);
            });
        }

        fn property<T: Clone + std::fmt::Debug + PartialEq + 'static>(
            value: T,
        ) -> WidgetStatePropertyRef<Option<T>> {
            Rc::new(WidgetStatePropertyAll::new(Some(value)))
        }

        /// Dart's 'Prefix IconButton inherits IconButtonTheme' and its suffix twin.
        fn check_icon_button_inherits_icon_button_theme(
            tester: &mut WidgetTester,
            prefix: bool,
            button_icon: IconData,
        ) {
            let background_color = Color::new(0xffff0000);
            let foreground_color = Color::new(0xff00ff00);
            let overlay_color = Color::new(0xff0000ff);
            let shadow_color = Color::new(0xff0ff0ff);
            const ELEVATION: f64 = 4.0;
            let shape = RoundedRectangleBorder::new(
                BorderSide::NONE,
                BorderRadius::all(Radius::circular(10.0)).into(),
            );
            let icon_button_style = ButtonStyle::new()
                .background_color(property(background_color))
                .foreground_color(property(foreground_color))
                .overlay_color(property(overlay_color))
                .shadow_color(property(shadow_color))
                .elevation(property(ELEVATION))
                .shape(property(OutlinedBorderRef::new(shape)));

            let button = IconButton::new(Some(noop()), icon(button_icon.clone()));
            let decoration = if prefix {
                InputDecoration::new().prefix_icon(button)
            } else {
                InputDecoration::new().suffix_icon(button)
            };
            tester.pump_widget(
                IconButtonTheme::new(
                    IconButtonThemeData::new().style(icon_button_style),
                    BuildInputDecorator::new(decoration).build(),
                )
                .into_widget(),
            );

            let icon_button = tester
                .ancestors(tester.find_by_icon(&button_icon))
                .into_iter()
                .find(|element| {
                    let app = tester.cell.borrow();
                    waft_widgets::downcast_widget::<IconButton>(&**element.widget(&app)).is_some()
                })
                .expect("the IconButton");
            let material = tester.descendants_of_type::<Material>(icon_button)[0];
            tester.widget(material, |material: &Material| {
                assert_eq!(material.color, Some(background_color));
                assert_eq!(material.shadow_color, Some(shadow_color));
                assert_eq!(material.elevation, ELEVATION);
                assert!(material.shape.as_ref().expect("a shape").eq_shape(&shape));
            });

            assert_eq!(
                icon_glyph_color(tester, &button_icon),
                Some(foreground_color)
            );

            let center = {
                let render = tester.render_box_of(tester.find_by_icon(&button_icon));
                let rect = tester.get_rect(render);
                rect.center()
            };
            let mut gesture = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, center, false);
            tester.pump_and_settle();
            let ink = tester.first::<crate::material::RenderInkFeatures>();
            let rects: Vec<u32> = tester
                .painted(waft_rendering::RenderBox::as_object(ink))
                .into_iter()
                .filter_map(|drawn| match drawn {
                    Drawn::Rect(color, _) => Some(color),
                    _ => None,
                })
                .collect();
            assert!(rects.contains(&overlay_color.to_argb32()), "{rects:x?}");
        }

        // input_decorator_test.dart 'Prefix IconButton inherits IconButtonTheme'.
        #[test]
        fn a_prefix_icon_button_inherits_the_icon_button_theme() {
            test_widgets(|tester| {
                check_icon_button_inherits_icon_button_theme(tester, true, Icons::person())
            });
        }

        // input_decorator_test.dart 'Suffix IconButton inherits IconButtonTheme'.
        #[test]
        fn a_suffix_icon_button_inherits_the_icon_button_theme() {
            test_widgets(|tester| {
                check_icon_button_inherits_icon_button_theme(tester, false, Icons::delete())
            });
        }

        /// Dart's 'Prefix IconButton color respects IconButtonTheme foreground color states' and
        /// its suffix twin.
        fn check_icon_button_color_respects_foreground_states(
            tester: &mut WidgetTester,
            prefix: bool,
            button_icon: IconData,
        ) {
            let icon_error_color = Color::new(0xffff0000);
            let icon_color = Color::new(0xff00ff00);
            let icon_button_style =
                ButtonStyle::new().foreground_color(
                    <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                        if states.contains(&WidgetState::Error) {
                            return Some(icon_error_color);
                        }
                        Some(icon_color)
                    }),
                );
            let build = |error_text: Option<&str>| {
                let button = IconButton::new(Some(noop()), icon(button_icon.clone()));
                let mut decoration = if prefix {
                    InputDecoration::new().prefix_icon(button)
                } else {
                    InputDecoration::new().suffix_icon(button)
                };
                if let Some(error_text) = error_text {
                    decoration = decoration.error_text(error_text);
                }
                BuildInputDecorator::new(decoration)
                    .icon_button_theme(IconButtonThemeData::new().style(icon_button_style.clone()))
                    .build()
            };

            // Test the prefix IconButton color when there is an error text.
            tester.pump_widget(build(Some("error")));

            assert_eq!(
                icon_glyph_color(tester, &button_icon),
                Some(icon_error_color)
            );

            // Test the prefix IconButton color when there is no error text.
            tester.pump_widget(build(None));
            tester.pump_and_settle();

            assert_eq!(icon_glyph_color(tester, &button_icon), Some(icon_color));
        }

        // input_decorator_test.dart 'Prefix IconButton color respects IconButtonTheme foreground
        // color states'.
        #[test]
        fn a_prefix_icon_button_colour_follows_the_icon_button_theme_foreground_states() {
            test_widgets(|tester| {
                check_icon_button_color_respects_foreground_states(tester, true, Icons::person())
            });
        }

        // input_decorator_test.dart 'Suffix IconButton color respects IconButtonTheme foreground
        // color states'.
        #[test]
        fn a_suffix_icon_button_colour_follows_the_icon_button_theme_foreground_states() {
            test_widgets(|tester| {
                check_icon_button_color_respects_foreground_states(tester, false, Icons::search())
            });
        }

        // input_decorator_test.dart 'Custom borders - InputDecoration shows custom focused border'.
        #[test]
        fn a_decoration_shows_its_custom_focused_border() {
            test_widgets(|tester| {
                let focused_border = outline(Colors::TEAL.primary(), 5.0);
                settled(
                    tester,
                    BuildInputDecorator::new(InputDecoration::new().focused_border(focused_border))
                        .is_focused(true),
                );
                assert!(border_is(tester, &focused_border));
            });
        }

        // input_decorator_test.dart 'Custom borders - InputDecoration shows custom error borders'.
        #[test]
        fn a_decoration_shows_its_custom_error_borders() {
            test_widgets(|tester| {
                let error_border = outline(Colors::RED.primary(), 1.5);
                let focused_error_border = outline(Colors::TEAL.primary(), 5.0);

                let mut check_borders =
                    |is_focused: bool,
                     enabled: bool,
                     error_text: Option<&str>,
                     error: Option<WidgetRef>,
                     expected_border: &OutlineInputBorder| {
                        let mut decoration = InputDecoration::new()
                            .enabled(enabled)
                            .error_border(error_border)
                            .focused_error_border(focused_error_border);
                        if let Some(error_text) = error_text {
                            decoration = decoration.error_text(error_text);
                        }
                        if let Some(error) = error {
                            decoration = decoration.error(error);
                        }
                        tester.pump_widget(
                            BuildInputDecorator::new(decoration)
                                .is_focused(is_focused)
                                .build(),
                        );
                        tester.pump_and_settle(); // Border changes are animated.
                        assert!(border_is(tester, expected_border));
                    };

                // Test with errorText.
                check_borders(true, true, Some("error"), None, &focused_error_border);
                check_borders(false, true, Some("error"), None, &error_border);
                check_borders(false, false, Some("error"), None, &error_border);

                // Test with error widget.
                let error = || Text::new("error").into_widget();
                check_borders(true, true, None, Some(error()), &focused_error_border);
                check_borders(false, true, None, Some(error()), &error_border);
                check_borders(false, false, None, Some(error()), &error_border);
            });
        }

        // input_decorator_test.dart 'Custom borders - InputDecoration shows custom enabled border'.
        #[test]
        fn a_decoration_shows_its_custom_enabled_border() {
            test_widgets(|tester| {
                let enabled_border = outline(Colors::TEAL.primary(), 5.0);
                settled(
                    tester,
                    BuildInputDecorator::new(InputDecoration::new().enabled_border(enabled_border)),
                );
                assert!(border_is(tester, &enabled_border));
            });
        }

        // input_decorator_test.dart 'Custom borders - InputDecoration shows custom disabled
        // border'.
        #[test]
        fn a_decoration_shows_its_custom_disabled_border() {
            test_widgets(|tester| {
                let disabled_border = outline(Colors::TEAL.primary(), 3.0);
                settled(
                    tester,
                    BuildInputDecorator::new(
                        InputDecoration::new()
                            .enabled(false)
                            .disabled_border(disabled_border),
                    ),
                );
                assert!(border_is(tester, &disabled_border));
            });
        }
    }

    // input_decorator_test.dart's 'Material2' group and the
    // cases after it.
    mod material2_and_layout {
        use waft_embedder::{Color, Offset, Rect, Size};
        use waft_painting::{
            BorderRadius, BorderSide, EdgeInsets, EdgeInsetsGeometry, TextAlignVertical,
            TextDirection, TextStyle,
        };
        use waft_rendering::BoxConstraints;
        use waft_widgets::testing::{WidgetTester, test_widgets};
        use waft_widgets::{
            AnyElement, Builder, Directionality, FadeTransition, GlobalKey, Icon, IntoWidget,
            IntrinsicWidth, Padding, SizedBox, WidgetRef, downcast_widget,
        };

        use super::super::*;
        use super::common::*;

        use crate::colors::Colors;
        use crate::icons::Icons;
        use crate::input_border::{InputBorderRef, OutlineInputBorder, UnderlineInputBorder};
        use crate::material::Material;
        use crate::test_support::{PaintInspection, material_app};
        use crate::text_theme::TextTheme;
        use crate::theme::Theme;
        use crate::theme_data::{ThemeData, VisualDensity};

        const K_MIN_INTERACTIVE_DIMENSION: f64 = 48.0;

        /// Dart's `buildInputDecoratorM2`, its named arguments the setters: a Material 2 app whose
        /// theme's text theme is `TextTheme(bodyLarge: TextStyle(fontSize: 16.0))`.
        struct M2 {
            decoration: InputDecoration,
            theme: Option<fn(&App) -> ThemeData>,
            input_decoration_theme: Option<InputDecorationThemeData>,
            text_direction: TextDirection,
            expands: bool,
            is_empty: bool,
            is_focused: bool,
            is_hovering: bool,
            use_intrinsic_width: bool,
            base_style: Option<TextStyle>,
            text_align_vertical: Option<TextAlignVertical>,
            visual_density: Option<VisualDensity>,
            child: Option<WidgetRef>,
        }

        // The setters are named after Dart's named arguments, as InputDecorator's are.
        #[allow(clippy::wrong_self_convention)]
        impl M2 {
            fn new(decoration: InputDecoration) -> M2 {
                M2 {
                    decoration,
                    theme: None,
                    input_decoration_theme: None,
                    text_direction: TextDirection::Ltr,
                    expands: false,
                    is_empty: false,
                    is_focused: false,
                    is_hovering: false,
                    use_intrinsic_width: false,
                    base_style: None,
                    text_align_vertical: None,
                    visual_density: None,
                    child: None,
                }
            }

            fn theme(mut self, theme: fn(&App) -> ThemeData) -> M2 {
                self.theme = Some(theme);
                self
            }

            fn input_decoration_theme(mut self, theme: InputDecorationThemeData) -> M2 {
                self.input_decoration_theme = Some(theme);
                self
            }

            fn text_direction(mut self, text_direction: TextDirection) -> M2 {
                self.text_direction = text_direction;
                self
            }

            fn expands(mut self, expands: bool) -> M2 {
                self.expands = expands;
                self
            }

            fn is_empty(mut self, is_empty: bool) -> M2 {
                self.is_empty = is_empty;
                self
            }

            fn is_focused(mut self, is_focused: bool) -> M2 {
                self.is_focused = is_focused;
                self
            }

            fn is_hovering(mut self, is_hovering: bool) -> M2 {
                self.is_hovering = is_hovering;
                self
            }

            fn use_intrinsic_width(mut self, use_intrinsic_width: bool) -> M2 {
                self.use_intrinsic_width = use_intrinsic_width;
                self
            }

            fn base_style(mut self, base_style: TextStyle) -> M2 {
                self.base_style = Some(base_style);
                self
            }

            fn text_align_vertical(mut self, text_align_vertical: TextAlignVertical) -> M2 {
                self.text_align_vertical = Some(text_align_vertical);
                self
            }

            fn visual_density(mut self, visual_density: VisualDensity) -> M2 {
                self.visual_density = Some(visual_density);
                self
            }

            fn child<K>(mut self, child: impl IntoWidget<K>) -> M2 {
                self.child = Some(child.into_widget());
                self
            }

            /// Dart's `tester.pumpWidget(buildInputDecoratorM2(..))`.
            fn pump(self, tester: &mut WidgetTester) {
                let widget = self.build(tester);
                tester.pump_widget(widget);
            }

            /// Dart's `buildInputDecoratorM2(..)`.
            fn build(self, tester: &WidgetTester) -> WidgetRef {
                let mut decorator = InputDecorator::new(self.decoration)
                    .expands(self.expands)
                    .is_empty(self.is_empty)
                    .is_focused(self.is_focused)
                    .is_hovering(self.is_hovering)
                    .child(self.child.unwrap_or_else(m2_input_text));
                if let Some(base_style) = self.base_style {
                    decorator = decorator.base_style(base_style);
                }
                if let Some(text_align_vertical) = self.text_align_vertical {
                    decorator = decorator.text_align_vertical(text_align_vertical);
                }
                let mut widget = decorator.into_widget();
                if self.use_intrinsic_width {
                    widget = IntrinsicWidth::new().child(widget).into_widget();
                }

                let theme = self.theme;
                let input_decoration_theme = self.input_decoration_theme;
                let visual_density = self.visual_density;
                let text_direction = self.text_direction;
                let home = Material::new().child(Builder::new(move |app, context| {
                    let base = match theme {
                        Some(theme) => Rc::new(theme(app)),
                        None => Theme::of(app, context),
                    };
                    let mut copy = base.copy_with();
                    if let Some(input_decoration_theme) = &input_decoration_theme {
                        copy = copy.input_decoration_theme(input_decoration_theme.clone());
                    }
                    if let Some(visual_density) = visual_density {
                        copy = copy.visual_density(visual_density);
                    }
                    copy = copy
                        .text_theme(TextTheme::new().body_large(TextStyle::new().font_size(16.0)));
                    Theme::new(
                        copy.build(),
                        waft_widgets::Align::new()
                            .alignment(waft_painting::AlignmentGeometry::Alignment(
                                waft_painting::Alignment::TOP_LEFT,
                            ))
                            .child(Directionality::new(text_direction, widget.clone())),
                    )
                    .into_widget()
                }));
                let app_theme = m2(&tester.cell.borrow());
                material_app(app_theme, home.into_widget())
            }
        }

        /// Dart's M2 input child: `Text('text', style: TextStyle(fontSize: 16.0))`.
        fn m2_input_text() -> WidgetRef {
            Text::new(INPUT_TEXT)
                .style(TextStyle::new().font_size(16.0))
                .into_widget()
        }

        fn top(tester: &WidgetTester, text: &str) -> f64 {
            text_rect(tester, text).top
        }

        fn bottom(tester: &WidgetTester, text: &str) -> f64 {
            text_rect(tester, text).bottom
        }

        fn decorator_size(tester: &WidgetTester) -> Size {
            get_decorator_rect(tester).size()
        }

        fn key_rect(tester: &WidgetTester, key: &GlobalKey) -> Rect {
            let render = tester.render_object(key).as_box().expect("a render box");
            tester.get_rect(render)
        }

        fn sized(key: &GlobalKey, width: f64, height: f64) -> WidgetRef {
            SizedBox::new()
                .key(key.clone().into())
                .width(width)
                .height(height)
                .into_widget()
        }

        fn icon_rect(tester: &WidgetTester, icon: &waft_widgets::IconData) -> Rect {
            rect_of(tester, tester.find_by_icon(icon))
        }

        /// Dart's `getOpacity(tester, text)`: the value of the nearest `FadeTransition` above the
        /// `Text` showing `text`.
        fn get_opacity(tester: &WidgetTester, text: &str) -> f64 {
            let found = tester.find_text(text);
            let element = found[0];
            let fade = tester
                .ancestors(element)
                .into_iter()
                .find(|ancestor: &AnyElement| {
                    let app = tester.cell.borrow();
                    downcast_widget::<FadeTransition>(&**ancestor.widget(&app)).is_some()
                })
                .expect("a FadeTransition above the text");
            let opacity = tester.widget(fade, |fade: &FadeTransition| fade.opacity.clone());
            opacity.value(&tester.cell.borrow())
        }

        fn in_exclusive_range(value: f64, low: f64, high: f64) -> bool {
            low < value && value < high
        }

        // input_decorator_test.dart 'Material2 - InputDecorator input/label text layout'.
        #[test]
        fn material2_input_and_label_text_layout() {
            test_widgets(|tester| {
                // The label appears above the input text
                M2::new(InputDecoration::new().label_text(LABEL_TEXT)).pump(tester);
                tester.pump_and_settle();

                // Overall height for this InputDecorator is 56dps:
                //   12 - top padding
                //   12 - floating label (font size 16dps * 0.75 = 12)
                //    4 - floating label / input text gap
                //   16 - input text (font size 16dps)
                //   12 - bottom padding
                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(top(tester, INPUT_TEXT), 28.0);
                assert_eq!(bottom(tester, INPUT_TEXT), 44.0);
                assert_eq!(top(tester, LABEL_TEXT), 12.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 24.0);
                assert_eq!(get_border_bottom(tester), 56.0);
                assert_eq!(get_border_weight(tester), 1.0);

                // The label appears within the input when there is no text content
                M2::new(InputDecoration::new().label_text(LABEL_TEXT))
                    .is_empty(true)
                    .pump(tester);
                tester.pump_and_settle();
                assert_eq!(top(tester, LABEL_TEXT), 20.0);

                // The label appears above the input text when there is no content and
                // floatingLabelBehavior is always
                M2::new(
                    InputDecoration::new()
                        .label_text(LABEL_TEXT)
                        .floating_label_behavior(FloatingLabelBehavior::Always),
                )
                .is_empty(true)
                .pump(tester);
                tester.pump_and_settle();
                assert_eq!(top(tester, LABEL_TEXT), 12.0);

                // The label appears within the input text when there is content and
                // floatingLabelBehavior is never
                M2::new(
                    InputDecoration::new()
                        .label_text(LABEL_TEXT)
                        .floating_label_behavior(FloatingLabelBehavior::Never),
                )
                .pump(tester);
                tester.pump_and_settle();
                assert_eq!(top(tester, LABEL_TEXT), 20.0);

                // isFocused: true increases the border's weight from 1.0 to 2.0
                // but does not change the overall height.
                M2::new(InputDecoration::new().label_text(LABEL_TEXT))
                    .is_focused(true)
                    .pump(tester);
                tester.pump_and_settle();
                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(top(tester, INPUT_TEXT), 28.0);
                assert_eq!(bottom(tester, INPUT_TEXT), 44.0);
                assert_eq!(top(tester, LABEL_TEXT), 12.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 24.0);
                assert_eq!(get_border_bottom(tester), 56.0);
                assert_eq!(get_border_weight(tester), 2.0);

                // isEmpty: true causes the label to be aligned with the input text
                M2::new(InputDecoration::new().label_text(LABEL_TEXT))
                    .is_empty(true)
                    .pump(tester);

                // The label animates downwards from it's initial position
                // above the input text. The animation's duration is 167ms.
                tester.pump_by(Duration::from_millis(50));
                let label_y_50ms = top(tester, LABEL_TEXT);
                assert!(
                    in_exclusive_range(label_y_50ms, 12.0, 20.0),
                    "{label_y_50ms}"
                );
                tester.pump_by(Duration::from_millis(50));
                let label_y_100ms = top(tester, LABEL_TEXT);
                assert!(
                    in_exclusive_range(label_y_100ms, label_y_50ms, 20.0),
                    "{label_y_100ms}"
                );
                tester.pump_and_settle();
                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(top(tester, INPUT_TEXT), 28.0);
                assert_eq!(bottom(tester, INPUT_TEXT), 44.0);
                assert_eq!(top(tester, LABEL_TEXT), 20.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 36.0);
                assert_eq!(get_border_bottom(tester), 56.0);
                assert_eq!(get_border_weight(tester), 1.0);

                // isFocused: true causes the label to move back up above the input text.
                M2::new(InputDecoration::new().label_text(LABEL_TEXT))
                    .is_empty(true)
                    .is_focused(true)
                    .pump(tester);

                // The label animates upwards from it's initial position
                // above the input text. The animation's duration is 167ms.
                tester.pump_by(Duration::from_millis(50));
                let label_y_50ms = top(tester, LABEL_TEXT);
                assert!(
                    in_exclusive_range(label_y_50ms, 12.0, 28.0),
                    "{label_y_50ms}"
                );
                tester.pump_by(Duration::from_millis(50));
                let label_y_100ms = top(tester, LABEL_TEXT);
                assert!(
                    in_exclusive_range(label_y_100ms, 12.0, label_y_50ms),
                    "{label_y_100ms}"
                );

                tester.pump_and_settle();
                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(top(tester, INPUT_TEXT), 28.0);
                assert_eq!(bottom(tester, INPUT_TEXT), 44.0);
                assert_eq!(top(tester, LABEL_TEXT), 12.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 24.0);
                assert_eq!(get_border_bottom(tester), 56.0);
                assert_eq!(get_border_weight(tester), 2.0);

                // enabled: false produces a hairline border if filled: false (the default)
                // The widget's size and layout is the same as for enabled: true.
                M2::new(InputDecoration::new().label_text(LABEL_TEXT).enabled(false))
                    .is_empty(true)
                    .pump(tester);
                tester.pump_and_settle();
                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(top(tester, INPUT_TEXT), 28.0);
                assert_eq!(bottom(tester, INPUT_TEXT), 44.0);
                assert_eq!(top(tester, LABEL_TEXT), 20.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 36.0);
                assert_eq!(get_border_weight(tester), 0.0);

                // enabled: false produces a transparent border if filled: true.
                // The widget's size and layout is the same as for enabled: true.
                M2::new(
                    InputDecoration::new()
                        .label_text(LABEL_TEXT)
                        .enabled(false)
                        .filled(true),
                )
                .is_empty(true)
                .pump(tester);
                tester.pump_and_settle();
                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(top(tester, INPUT_TEXT), 28.0);
                assert_eq!(bottom(tester, INPUT_TEXT), 44.0);
                assert_eq!(top(tester, LABEL_TEXT), 20.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 36.0);
                assert_eq!(get_border_color(tester), Colors::TRANSPARENT);

                // alignLabelWithHint: true positions the label at the text baseline,
                // aligned with the hint.
                M2::new(
                    InputDecoration::new()
                        .label_text(LABEL_TEXT)
                        .align_label_with_hint(true)
                        .hint_text(HINT_TEXT),
                )
                .is_empty(true)
                .pump(tester);
                tester.pump_and_settle();
                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(top(tester, LABEL_TEXT), top(tester, HINT_TEXT));
                assert_eq!(bottom(tester, LABEL_TEXT), bottom(tester, HINT_TEXT));
            });
        }

        fn pump_floating_label(tester: &mut WidgetTester, is_focused: bool) {
            M2::new(
                InputDecoration::new()
                    .label_text(LABEL_TEXT)
                    .floating_label_behavior(FloatingLabelBehavior::Auto),
            )
            .is_empty(true)
            .is_focused(is_focused)
            .pump(tester);
        }

        fn assert_close(value: f64, expected: f64, delta: f64) {
            assert!(
                (value - expected).abs() <= delta,
                "{value} is not within {delta} of {expected}"
            );
        }

        // input_decorator_test.dart 'Material2 - InputDecorator floating label animation duration
        // and curve'.
        #[test]
        fn material2_floating_label_animation_duration_and_curve() {
            test_widgets(|tester| {
                pump_floating_label(tester, false);
                assert_eq!(top(tester, LABEL_TEXT), 20.0);

                // The label animates upwards and scales down.
                // The animation duration is 167ms and the curve is fastOutSlowIn.
                pump_floating_label(tester, true);
                tester.pump_by(Duration::from_millis(42));
                assert_close(top(tester, LABEL_TEXT), 18.06, 0.5);
                tester.pump_by(Duration::from_millis(42));
                assert_close(top(tester, LABEL_TEXT), 13.78, 0.5);
                tester.pump_by(Duration::from_millis(42));
                assert_close(top(tester, LABEL_TEXT), 12.31, 0.5);
                tester.pump_by(Duration::from_millis(41));
                assert_eq!(top(tester, LABEL_TEXT), 12.0);

                // If the animation changes direction without first reaching the
                // AnimationStatus.completed or AnimationStatus.dismissed status,
                // the CurvedAnimation stays on the same curve in the opposite direction.
                // The pumpAndSettle is used to prevent this behavior.
                tester.pump_and_settle();

                // The label animates downwards and scales up.
                // The animation duration is 167ms and the curve is fastOutSlowIn.
                pump_floating_label(tester, false);
                tester.pump_by(Duration::from_millis(42));
                assert_close(top(tester, LABEL_TEXT), 13.94, 0.5);
                tester.pump_by(Duration::from_millis(42));
                assert_close(top(tester, LABEL_TEXT), 18.22, 0.5);
                tester.pump_by(Duration::from_millis(42));
                assert_close(top(tester, LABEL_TEXT), 19.69, 0.5);
                tester.pump_by(Duration::from_millis(41));
                assert_eq!(top(tester, LABEL_TEXT), 20.0);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator input/hint layout'.
        #[test]
        fn material2_input_and_hint_layout() {
            test_widgets(|tester| {
                // The hint aligns with the input text
                M2::new(InputDecoration::new().hint_text(HINT_TEXT))
                    .is_empty(true)
                    .pump(tester);

                assert_eq!(
                    decorator_size(tester),
                    Size::new(800.0, K_MIN_INTERACTIVE_DIMENSION)
                );
                assert_eq!(top(tester, INPUT_TEXT), 16.0);
                assert_eq!(bottom(tester, INPUT_TEXT), 32.0);
                assert_eq!(top(tester, HINT_TEXT), 16.0);
                assert_eq!(bottom(tester, HINT_TEXT), 32.0);
                assert_eq!(get_border_bottom(tester), 48.0);
                assert_eq!(get_border_weight(tester), 1.0);

                assert_eq!(
                    text_rect(tester, HINT_TEXT).width(),
                    text_rect(tester, INPUT_TEXT).width()
                );
            });
        }

        fn label_and_hint() -> InputDecoration {
            InputDecoration::new()
                .label_text(LABEL_TEXT)
                .hint_text(HINT_TEXT)
        }

        // input_decorator_test.dart 'Material2 - InputDecorator input/label/hint layout'.
        #[test]
        fn material2_input_label_and_hint_layout() {
            test_widgets(|tester| {
                // Label is visible, hint is not (opacity 0.0).
                M2::new(label_and_hint()).is_empty(true).pump(tester);

                // The label is not floating so it's vertically centered.
                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(top(tester, INPUT_TEXT), 28.0);
                assert_eq!(bottom(tester, INPUT_TEXT), 44.0);
                assert_eq!(top(tester, LABEL_TEXT), 20.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 36.0);
                assert_eq!(get_opacity(tester, HINT_TEXT), 0.0);
                assert_eq!(get_border_bottom(tester), 56.0);
                assert_eq!(get_border_weight(tester), 1.0);

                // Label moves upwards, hint is visible (opacity 1.0).
                M2::new(label_and_hint())
                    .is_empty(true)
                    .is_focused(true)
                    .pump(tester);

                // The hint's opacity animates from 0.0 to 1.0.
                // The animation's default duration is 20ms.
                tester.pump_by(Duration::from_millis(9));
                let hint_opacity_9ms = get_opacity(tester, HINT_TEXT);
                assert!(in_exclusive_range(hint_opacity_9ms, 0.0, 1.0));
                tester.pump_by(Duration::from_millis(9));
                let hint_opacity_18ms = get_opacity(tester, HINT_TEXT);
                assert!(in_exclusive_range(hint_opacity_18ms, hint_opacity_9ms, 1.0));

                tester.pump_and_settle();
                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(top(tester, INPUT_TEXT), 28.0);
                assert_eq!(bottom(tester, INPUT_TEXT), 44.0);
                assert_eq!(top(tester, LABEL_TEXT), 12.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 24.0);
                assert_eq!(top(tester, HINT_TEXT), 28.0);
                assert_eq!(bottom(tester, HINT_TEXT), 44.0);
                assert_eq!(get_opacity(tester, HINT_TEXT), 1.0);
                assert_eq!(get_border_bottom(tester), 56.0);
                assert_eq!(get_border_weight(tester), 2.0);

                M2::new(label_and_hint()).is_focused(true).pump(tester);

                // The hint's opacity animates from 1.0 to 0.0.
                // The animation's default duration is 20ms.
                tester.pump_by(Duration::from_millis(9));
                let hint_opacity_9ms = get_opacity(tester, HINT_TEXT);
                assert!(in_exclusive_range(hint_opacity_9ms, 0.0, 1.0));
                tester.pump_by(Duration::from_millis(9));
                let hint_opacity_18ms = get_opacity(tester, HINT_TEXT);
                assert!(in_exclusive_range(hint_opacity_18ms, 0.0, hint_opacity_9ms));

                tester.pump_and_settle();
                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(top(tester, INPUT_TEXT), 28.0);
                assert_eq!(bottom(tester, INPUT_TEXT), 44.0);
                assert_eq!(top(tester, LABEL_TEXT), 12.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 24.0);
                assert_eq!(top(tester, HINT_TEXT), 28.0);
                assert_eq!(bottom(tester, HINT_TEXT), 44.0);
                assert_eq!(get_opacity(tester, HINT_TEXT), 0.0);
                assert_eq!(get_border_bottom(tester), 56.0);
                assert_eq!(get_border_weight(tester), 2.0);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator input/label/hint dense layout'.
        #[test]
        fn material2_input_label_and_hint_dense_layout() {
            test_widgets(|tester| {
                // Label is visible, hint is not (opacity 0.0).
                M2::new(label_and_hint().is_dense(true))
                    .is_empty(true)
                    .pump(tester);

                // The label is not floating so it's vertically centered.
                assert_eq!(decorator_size(tester), Size::new(800.0, 48.0));
                assert_eq!(top(tester, INPUT_TEXT), 24.0);
                assert_eq!(bottom(tester, INPUT_TEXT), 40.0);
                assert_eq!(top(tester, LABEL_TEXT), 16.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 32.0);
                assert_eq!(get_opacity(tester, HINT_TEXT), 0.0);
                assert_eq!(get_border_bottom(tester), 48.0);
                assert_eq!(get_border_weight(tester), 1.0);

                // Label is visible, hint is not (opacity 0.0).
                M2::new(label_and_hint().is_dense(true))
                    .is_empty(true)
                    .is_focused(true)
                    .pump(tester);
                tester.pump_and_settle();
                assert_eq!(decorator_size(tester), Size::new(800.0, 48.0));
                assert_eq!(top(tester, INPUT_TEXT), 24.0);
                assert_eq!(bottom(tester, INPUT_TEXT), 40.0);
                assert_eq!(top(tester, LABEL_TEXT), 8.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 20.0);
                assert_eq!(get_opacity(tester, HINT_TEXT), 1.0);
                assert_eq!(get_border_bottom(tester), 48.0);
                assert_eq!(get_border_weight(tester), 2.0);
            });
        }

        /// The shared steps of the three hint animation duration cases: the hint fades in when the
        /// field gets the focus and out when it loses it, over `step` three times.
        fn check_hint_fade(tester: &mut WidgetTester, build: impl Fn(bool) -> M2, step: Duration) {
            build(false).pump(tester);

            // The hint is not visible (opacity 0.0).
            assert_eq!(get_opacity(tester, HINT_TEXT), 0.0);

            // Focus to show the hint.
            build(true).pump(tester);

            // The hint's opacity animates from 0.0 to 1.0.
            tester.pump_by(step);
            let first = get_opacity(tester, HINT_TEXT);
            assert!(in_exclusive_range(first, 0.0, 1.0), "{first}");
            tester.pump_by(step);
            let second = get_opacity(tester, HINT_TEXT);
            assert!(in_exclusive_range(second, first, 1.0), "{second}");
            tester.pump_by(step);
            assert_eq!(get_opacity(tester, HINT_TEXT), 1.0);

            // Unfocus to hide the hint.
            build(false).pump(tester);

            // The hint's opacity animates from 1.0 to 0.0.
            tester.pump_by(step);
            let first = get_opacity(tester, HINT_TEXT);
            assert!(in_exclusive_range(first, 0.0, 1.0), "{first}");
            tester.pump_by(step);
            let second = get_opacity(tester, HINT_TEXT);
            assert!(in_exclusive_range(second, 0.0, first), "{second}");
            tester.pump_by(step);
            assert_eq!(get_opacity(tester, HINT_TEXT), 0.0);
        }

        // input_decorator_test.dart 'Material2 - InputDecorator default hint animation duration'.
        #[test]
        fn material2_default_hint_animation_duration() {
            test_widgets(|tester| {
                check_hint_fade(
                    tester,
                    |is_focused| {
                        M2::new(label_and_hint())
                            .is_empty(true)
                            .is_focused(is_focused)
                    },
                    Duration::from_millis(9),
                );
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator custom hint animation duration'.
        #[test]
        fn material2_custom_hint_animation_duration() {
            test_widgets(|tester| {
                check_hint_fade(
                    tester,
                    |is_focused| {
                        M2::new(label_and_hint().hint_fade_duration(Duration::from_millis(120)))
                            .is_empty(true)
                            .is_focused(is_focused)
                    },
                    Duration::from_millis(50),
                );
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator custom hint animation duration from
        // theme'.
        #[test]
        fn material2_custom_hint_animation_duration_from_theme() {
            test_widgets(|tester| {
                check_hint_fade(
                    tester,
                    |is_focused| {
                        M2::new(label_and_hint())
                            .input_decoration_theme(
                                InputDecorationThemeData::new()
                                    .hint_fade_duration(Duration::from_millis(120)),
                            )
                            .is_empty(true)
                            .is_focused(is_focused)
                    },
                    Duration::from_millis(50),
                );
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator with no input border'.
        #[test]
        fn material2_decorator_with_no_input_border_has_a_zero_weight_border() {
            test_widgets(|tester| {
                M2::new(InputDecoration::new().border(InputBorderRef::none()))
                    .is_empty(true)
                    .pump(tester);
                assert_eq!(get_border_weight(tester), 0.0);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator error/helper/counter layout'.
        #[test]
        fn material2_error_helper_and_counter_layout() {
            test_widgets(|tester| {
                M2::new(
                    InputDecoration::new()
                        .label_text(LABEL_TEXT)
                        .helper_text(HELPER_TEXT)
                        .counter_text(COUNTER_TEXT)
                        .filled(true),
                )
                .is_empty(true)
                .pump(tester);

                // isEmpty: true, the label is not floating
                assert_eq!(decorator_size(tester), Size::new(800.0, 76.0));
                assert_eq!(top(tester, INPUT_TEXT), 28.0);
                assert_eq!(bottom(tester, INPUT_TEXT), 44.0);
                assert_eq!(top(tester, LABEL_TEXT), 20.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 36.0);
                assert_eq!(get_border_bottom(tester), 56.0);
                assert_eq!(get_border_weight(tester), 1.0);
                assert_eq!(
                    text_rect(tester, HELPER_TEXT).top_left(),
                    Offset::new(12.0, 64.0)
                );
                assert_eq!(
                    text_rect(tester, COUNTER_TEXT).top_right(),
                    Offset::new(788.0, 64.0)
                );

                // If errorText is specified then the helperText isn't shown
                M2::new(
                    InputDecoration::new()
                        .label_text(LABEL_TEXT)
                        .error_text(ERROR_TEXT)
                        .helper_text(HELPER_TEXT)
                        .counter_text(COUNTER_TEXT)
                        .filled(true),
                )
                .pump(tester);
                tester.pump_and_settle();

                // isEmpty: false, the label _is_ floating
                assert_eq!(decorator_size(tester), Size::new(800.0, 76.0));
                assert_eq!(top(tester, INPUT_TEXT), 28.0);
                assert_eq!(bottom(tester, INPUT_TEXT), 44.0);
                assert_eq!(top(tester, LABEL_TEXT), 12.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 24.0);
                assert_eq!(get_border_bottom(tester), 56.0);
                assert_eq!(get_border_weight(tester), 1.0);
                assert_eq!(
                    text_rect(tester, ERROR_TEXT).top_left(),
                    Offset::new(12.0, 64.0)
                );
                assert_eq!(
                    text_rect(tester, COUNTER_TEXT).top_right(),
                    Offset::new(788.0, 64.0)
                );
                assert!(tester.find_text(HELPER_TEXT).is_empty());

                // The layout of the error/helper/counter supportingText doesn't change for dense
                // layout.
                M2::new(
                    InputDecoration::new()
                        .is_dense(true)
                        .label_text(LABEL_TEXT)
                        .error_text(ERROR_TEXT)
                        .helper_text(HELPER_TEXT)
                        .counter_text(COUNTER_TEXT)
                        .filled(true),
                )
                .pump(tester);
                tester.pump_and_settle();

                // isEmpty: false, the label _is_ floating
                assert_eq!(decorator_size(tester), Size::new(800.0, 68.0));
                assert_eq!(top(tester, INPUT_TEXT), 24.0);
                assert_eq!(bottom(tester, INPUT_TEXT), 40.0);
                assert_eq!(top(tester, LABEL_TEXT), 8.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 20.0);
                assert_eq!(get_border_bottom(tester), 48.0);
                assert_eq!(get_border_weight(tester), 1.0);
                assert_eq!(
                    text_rect(tester, ERROR_TEXT).top_left(),
                    Offset::new(12.0, 56.0)
                );
                assert_eq!(
                    text_rect(tester, COUNTER_TEXT).top_right(),
                    Offset::new(788.0, 56.0)
                );

                M2::new(
                    InputDecoration::new()
                        .is_dense(true)
                        .label_text(LABEL_TEXT)
                        .error_text(ERROR_TEXT)
                        .helper_text(HELPER_TEXT)
                        .counter_text(COUNTER_TEXT)
                        .filled(true),
                )
                .is_empty(true)
                .pump(tester);
                tester.pump_and_settle();

                // isEmpty: false, the label is not floating
                assert_eq!(decorator_size(tester), Size::new(800.0, 68.0));
                assert_eq!(top(tester, INPUT_TEXT), 24.0);
                assert_eq!(bottom(tester, INPUT_TEXT), 40.0);
                assert_eq!(top(tester, LABEL_TEXT), 16.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 32.0);
                assert_eq!(get_border_bottom(tester), 48.0);
                assert_eq!(get_border_weight(tester), 1.0);
                assert_eq!(
                    text_rect(tester, ERROR_TEXT).top_left(),
                    Offset::new(12.0, 56.0)
                );
                assert_eq!(
                    text_rect(tester, COUNTER_TEXT).top_right(),
                    Offset::new(788.0, 56.0)
                );
            });
        }

        fn error_border() -> InputBorderRef {
            InputBorderRef::new(OutlineInputBorder::new().border_side(BorderSide {
                color: Colors::RED.primary(),
                width: 1.5,
                ..BorderSide::default()
            }))
        }

        fn focused_error_border() -> InputBorderRef {
            InputBorderRef::new(OutlineInputBorder::new().border_side(BorderSide {
                color: Colors::TEAL.primary(),
                width: 5.0,
                ..BorderSide::default()
            }))
        }

        // input_decorator_test.dart 'Material2 - InputDecoration shows error border for errorText and
        // error widget'. The `errorText` steps keep Dart's string; the `error` steps hand a widget.
        #[test]
        fn material2_error_border_shows_for_error_text_and_error_widget() {
            test_widgets(|tester| {
                let with_text = || {
                    InputDecoration::new()
                        .error_text(ERROR_TEXT)
                        .error_border(error_border())
                        .focused_error_border(focused_error_border())
                };
                let with_widget = || {
                    InputDecoration::new()
                        .error(waft_widgets::Text::new(ERROR_TEXT))
                        .error_border(error_border())
                        .focused_error_border(focused_error_border())
                };
                for decoration in [with_text as fn() -> InputDecoration, with_widget] {
                    M2::new(decoration()).is_focused(true).pump(tester);
                    tester.pump_and_settle(); // Border changes are animated.
                    assert!(*get_border(tester).unwrap() == *focused_error_border());

                    M2::new(decoration()).pump(tester);
                    tester.pump_and_settle(); // Border changes are animated.
                    assert!(*get_border(tester).unwrap() == *error_border());

                    M2::new(decoration().enabled(false)).pump(tester);
                    tester.pump_and_settle(); // Border changes are animated.
                    assert!(*get_border(tester).unwrap() == *error_border());
                }
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator prefix/suffix texts'.
        #[test]
        fn material2_prefix_and_suffix_texts() {
            test_widgets(|tester| {
                M2::new(
                    InputDecoration::new()
                        .prefix_text("p")
                        .suffix_text("s")
                        .filled(true),
                )
                .pump(tester);

                // The prefix and suffix wrap the input text and are left and right justified
                // respectively. They should have the same height as the input text (16).
                assert_eq!(
                    decorator_size(tester),
                    Size::new(800.0, K_MIN_INTERACTIVE_DIMENSION)
                );
                assert_eq!(text_rect(tester, INPUT_TEXT).height(), 16.0);
                assert_eq!(text_rect(tester, "p").height(), 16.0);
                assert_eq!(text_rect(tester, "s").height(), 16.0);
                assert_eq!(top(tester, INPUT_TEXT), 16.0);
                assert_eq!(top(tester, "p"), 16.0);
                assert_eq!(text_rect(tester, "p").left, 12.0);
                assert_eq!(top(tester, "s"), 16.0);
                assert_eq!(text_rect(tester, "s").right, 788.0);

                // layout is a row: [p text s]
                assert_eq!(text_rect(tester, "p").left, 12.0);
                assert!(text_rect(tester, "p").right <= text_rect(tester, INPUT_TEXT).left);
                assert!(text_rect(tester, INPUT_TEXT).right <= text_rect(tester, "s").left);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator icon/prefix/suffix'.
        #[test]
        fn material2_icon_prefix_and_suffix() {
            test_widgets(|tester| {
                M2::new(
                    InputDecoration::new()
                        .prefix_text("p")
                        .suffix_text("s")
                        .icon(Icon::new(Some(Icons::android())))
                        .filled(true),
                )
                .pump(tester);

                assert_eq!(
                    decorator_size(tester),
                    Size::new(800.0, K_MIN_INTERACTIVE_DIMENSION)
                );
                assert_eq!(text_rect(tester, INPUT_TEXT).height(), 16.0);
                assert_eq!(text_rect(tester, "p").height(), 16.0);
                assert_eq!(text_rect(tester, "s").height(), 16.0);
                assert_eq!(top(tester, INPUT_TEXT), 16.0);
                assert_eq!(top(tester, "p"), 16.0);
                assert_eq!(top(tester, "s"), 16.0);
                assert_eq!(text_rect(tester, "s").right, 788.0);
                let icon = icon_rect(tester, &Icons::android());
                assert_eq!(icon.height(), 24.0);

                // The 24dps high icon is centered on the 16dps high input line
                assert_eq!(icon.top, 12.0);

                // layout is a row: [icon, p text s]
                assert_eq!(icon.left, 0.0);
                assert!(icon.right <= text_rect(tester, "p").left);
                assert!(text_rect(tester, "p").right <= text_rect(tester, INPUT_TEXT).left);
                assert!(text_rect(tester, INPUT_TEXT).right <= text_rect(tester, "s").left);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator prefix/suffix widgets'.
        #[test]
        fn material2_prefix_and_suffix_widgets() {
            test_widgets(|tester| {
                let p_key = GlobalKey::new();
                let s_key = GlobalKey::new();
                M2::new(
                    InputDecoration::new()
                        .prefix(
                            Padding::new(EdgeInsetsGeometry::all(4.0))
                                .key(p_key.clone().into())
                                .child(Text::new("p")),
                        )
                        .suffix(
                            Padding::new(EdgeInsetsGeometry::all(4.0))
                                .key(s_key.clone().into())
                                .child(Text::new("s")),
                        )
                        .filled(true),
                )
                .pump(tester);

                // Overall height for this InputDecorator is 48dps because
                // the prefix and the suffix widget is surrounded with padding:
                //   12 - top padding
                //    4 - top prefix/suffix padding
                //   16 - input text (font size 16dps)
                //    4 - bottom prefix/suffix padding
                //   12 - bottom padding
                assert_eq!(decorator_size(tester), Size::new(800.0, 48.0));
                assert_eq!(text_rect(tester, INPUT_TEXT).height(), 16.0);
                assert_eq!(key_rect(tester, &p_key).height(), 24.0);
                assert_eq!(text_rect(tester, "p").height(), 16.0);
                assert_eq!(key_rect(tester, &s_key).height(), 24.0);
                assert_eq!(text_rect(tester, "s").height(), 16.0);
                assert_eq!(top(tester, INPUT_TEXT), 16.0);
                assert_eq!(key_rect(tester, &p_key).top, 12.0);
                assert_eq!(top(tester, "p"), 16.0);
                assert_eq!(key_rect(tester, &s_key).top, 12.0);
                assert_eq!(top(tester, "s"), 16.0);
                assert_eq!(key_rect(tester, &s_key).right, 788.0);
                assert_eq!(text_rect(tester, "s").right, 784.0);

                // layout is a row: [prefix text suffix]
                assert_eq!(key_rect(tester, &p_key).left, 12.0);
                assert_eq!(
                    key_rect(tester, &p_key).right,
                    text_rect(tester, INPUT_TEXT).left
                );
                assert!(text_rect(tester, INPUT_TEXT).right <= key_rect(tester, &s_key).right);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator tall prefix'.
        #[test]
        fn material2_tall_prefix() {
            test_widgets(|tester| {
                let p_key = GlobalKey::new();
                M2::new(
                    InputDecoration::new()
                        .prefix(sized(&p_key, 10.0, 100.0))
                        .filled(true),
                )
                // Set the fontSize so that everything works out to whole numbers.
                .child(
                    Text::new(INPUT_TEXT)
                        .style(TextStyle::new().font_size(20.0))
                        .into_widget(),
                )
                .pump(tester);

                assert_eq!(decorator_size(tester).width(), 800.0);
                assert_eq!(decorator_size(tester).height(), 129.0);
                assert_eq!(text_rect(tester, INPUT_TEXT).height(), 20.0);
                assert_eq!(key_rect(tester, &p_key).height(), 100.0);
                assert_eq!(top(tester, INPUT_TEXT), 97.0); // 12 + 100 - 15
                assert_eq!(key_rect(tester, &p_key).top, 12.0);

                // layout is a row: [prefix text suffix]
                assert_eq!(key_rect(tester, &p_key).left, 12.0);
                assert_eq!(
                    key_rect(tester, &p_key).right,
                    text_rect(tester, INPUT_TEXT).left
                );
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator tall prefix with border'.
        #[test]
        fn material2_tall_prefix_with_border() {
            test_widgets(|tester| {
                let p_key = GlobalKey::new();
                M2::new(
                    InputDecoration::new()
                        .border(OutlineInputBorder::new())
                        .prefix(sized(&p_key, 10.0, 100.0))
                        .filled(true),
                )
                // Set the fontSize so that everything works out to whole numbers.
                .child(
                    Text::new(INPUT_TEXT)
                        .style(TextStyle::new().font_size(20.0))
                        .into_widget(),
                )
                .pump(tester);

                // When a border is present, the input text and prefix/suffix are centered
                // within the input. Here, that will be content of height 106, centered
                // within an input of height 145. That gives 20 pixels of space on each side
                // of the content, so the prefix is positioned at 19, and the text is at
                // 20+100-15=105.
                assert_eq!(decorator_size(tester).width(), 800.0);
                assert_eq!(decorator_size(tester).height(), 145.0);
                assert_eq!(text_rect(tester, INPUT_TEXT).height(), 20.0);
                assert_eq!(key_rect(tester, &p_key).height(), 100.0);
                assert_eq!(top(tester, INPUT_TEXT), 105.0);
                assert_eq!(key_rect(tester, &p_key).top, 20.0);

                // layout is a row: [prefix text suffix]
                assert_eq!(key_rect(tester, &p_key).left, 12.0);
                assert_eq!(
                    key_rect(tester, &p_key).right,
                    text_rect(tester, INPUT_TEXT).left
                );
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator prefixIcon/suffixIcon'.
        #[test]
        fn material2_prefix_icon_and_suffix_icon() {
            test_widgets(|tester| {
                M2::new(
                    InputDecoration::new()
                        .prefix_icon(Icon::new(Some(Icons::pages())))
                        .suffix_icon(Icon::new(Some(Icons::satellite())))
                        .filled(true),
                )
                .pump(tester);

                // Overall height for this InputDecorator is 48dps because the prefix icon's minimum
                // size is 48x48 and the rest of the elements only require 40dps.
                assert_eq!(decorator_size(tester), Size::new(800.0, 48.0));
                assert_eq!(text_rect(tester, INPUT_TEXT).height(), 16.0);
                let pages = icon_rect(tester, &Icons::pages());
                let satellite = icon_rect(tester, &Icons::satellite());
                assert_eq!(pages.height(), 48.0);
                assert_eq!(satellite.height(), 48.0);
                assert_eq!(top(tester, INPUT_TEXT), 12.0);
                assert_eq!(pages.top, 0.0);
                assert_eq!(satellite.top, 0.0);
                assert_eq!(satellite.right, 800.0);

                // layout is a row: [icon text icon]
                assert_eq!(pages.left, 0.0);
                assert!(pages.right <= text_rect(tester, INPUT_TEXT).left);
                assert!(text_rect(tester, INPUT_TEXT).right <= satellite.left);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator
        // prefixIconConstraints/suffixIconConstraints'.
        #[test]
        fn material2_prefix_icon_and_suffix_icon_constraints() {
            test_widgets(|tester| {
                M2::new(
                    InputDecoration::new()
                        .prefix_icon(Icon::new(Some(Icons::pages())))
                        .prefix_icon_constraints(BoxConstraints {
                            min_width: 32.0,
                            min_height: 32.0,
                            ..BoxConstraints::new()
                        })
                        .suffix_icon(Icon::new(Some(Icons::satellite())))
                        .suffix_icon_constraints(BoxConstraints {
                            min_width: 25.0,
                            min_height: 25.0,
                            ..BoxConstraints::new()
                        })
                        .is_dense(true), // has to be true to go below 48px height
                )
                .pump(tester);

                // Overall height for this InputDecorator is 32px because the prefix icon
                // is now a custom value
                assert_eq!(decorator_size(tester), Size::new(800.0, 32.0));
                assert_eq!(text_rect(tester, INPUT_TEXT).height(), 16.0);
                let pages = icon_rect(tester, &Icons::pages());
                let satellite = icon_rect(tester, &Icons::satellite());
                assert_eq!(pages.height(), 32.0);
                assert_eq!(satellite.height(), 25.0);

                // (InputDecorator height - Text widget height) / 2
                assert_eq!(top(tester, INPUT_TEXT), (32.0 - 16.0) / 2.0);
                // prefixIcon should take up the entire height of InputDecorator
                assert_eq!(pages.top, 0.0);
                // (InputDecorator height - suffixIcon height) / 2
                assert_eq!(satellite.top, (32.0 - 25.0) / 2.0);
                assert_eq!(satellite.right, 800.0);
            });
        }

        // input_decorator_test.dart 'Material2 - prefix/suffix icons are centered when smaller than
        // 48 by 48'.
        #[test]
        fn material2_prefix_and_suffix_icons_are_centered_when_smaller_than_48_by_48() {
            test_widgets(|tester| {
                let prefix_key = GlobalKey::new();
                M2::new(
                    InputDecoration::new()
                        .prefix_icon(Padding::new(EdgeInsetsGeometry::all(16.0)).child(sized(
                            &prefix_key,
                            8.0,
                            8.0,
                        )))
                        .filled(true),
                )
                .pump(tester);

                assert_eq!(decorator_size(tester), Size::new(800.0, 48.0));
                assert_eq!(key_rect(tester, &prefix_key).height(), 16.0);
                assert_eq!(key_rect(tester, &prefix_key).top, 16.0);
            });
        }

        /// The shared steps of the two visual density cases: Dart's expected positions for the
        /// unfocused and the focused field, with `base` the decorator height.
        struct DensityFrame {
            height: f64,
            input_top: f64,
            label_top: f64,
            floating_label_top: f64,
        }

        fn check_visual_density(
            tester: &mut WidgetTester,
            density: VisualDensity,
            frame: DensityFrame,
        ) {
            let build = |is_empty: bool, is_focused: bool| {
                M2::new(label_and_hint())
                    .is_empty(is_empty)
                    .is_focused(is_focused)
                    .visual_density(density)
            };
            // Label is visible, hint is not (opacity 0.0).
            build(true, false).pump(tester);

            // The label is not floating so it's vertically centered.
            assert_eq!(decorator_size(tester), Size::new(800.0, frame.height));
            assert_eq!(top(tester, INPUT_TEXT), frame.input_top);
            assert_eq!(bottom(tester, INPUT_TEXT), frame.input_top + 16.0);
            assert_eq!(top(tester, LABEL_TEXT), frame.label_top);
            assert_eq!(bottom(tester, LABEL_TEXT), frame.label_top + 16.0);
            assert_eq!(get_opacity(tester, HINT_TEXT), 0.0);
            assert_eq!(get_border_bottom(tester), frame.height);
            assert_eq!(get_border_weight(tester), 1.0);

            // Label moves upwards, hint is visible (opacity 1.0).
            build(true, true).pump(tester);

            // The hint's opacity animates from 0.0 to 1.0.
            // The animation's default duration is 20ms.
            tester.pump_by(Duration::from_millis(9));
            let hint_opacity_9ms = get_opacity(tester, HINT_TEXT);
            assert!(in_exclusive_range(hint_opacity_9ms, 0.0, 1.0));
            tester.pump_by(Duration::from_millis(9));
            let hint_opacity_18ms = get_opacity(tester, HINT_TEXT);
            assert!(in_exclusive_range(hint_opacity_18ms, hint_opacity_9ms, 1.0));

            tester.pump_and_settle();
            let check_focused = |tester: &WidgetTester, hint_opacity: f64| {
                assert_eq!(decorator_size(tester), Size::new(800.0, frame.height));
                assert_eq!(top(tester, INPUT_TEXT), frame.input_top);
                assert_eq!(bottom(tester, INPUT_TEXT), frame.input_top + 16.0);
                assert_eq!(top(tester, LABEL_TEXT), frame.floating_label_top);
                assert_eq!(bottom(tester, LABEL_TEXT), frame.floating_label_top + 12.0);
                assert_eq!(top(tester, HINT_TEXT), frame.input_top);
                assert_eq!(bottom(tester, HINT_TEXT), frame.input_top + 16.0);
                assert_eq!(get_opacity(tester, HINT_TEXT), hint_opacity);
                assert_eq!(get_border_bottom(tester), frame.height);
                assert_eq!(get_border_weight(tester), 2.0);
            };
            check_focused(tester, 1.0);

            build(false, true).pump(tester);

            // The hint's opacity animates from 1.0 to 0.0.
            // The animation's default duration is 20ms.
            tester.pump_by(Duration::from_millis(9));
            let hint_opacity_9ms = get_opacity(tester, HINT_TEXT);
            assert!(in_exclusive_range(hint_opacity_9ms, 0.0, 1.0));
            tester.pump_by(Duration::from_millis(9));
            let hint_opacity_18ms = get_opacity(tester, HINT_TEXT);
            assert!(in_exclusive_range(hint_opacity_18ms, 0.0, hint_opacity_9ms));

            tester.pump_and_settle();
            check_focused(tester, 0.0);
        }

        // input_decorator_test.dart 'Material2 - InputDecorator respects reduced theme visualDensity'.
        #[test]
        fn material2_decorator_respects_reduced_theme_visual_density() {
            test_widgets(|tester| {
                check_visual_density(
                    tester,
                    VisualDensity::COMPACT,
                    DensityFrame {
                        height: 48.0,
                        input_top: 24.0,
                        label_top: 16.0,
                        floating_label_top: 8.0,
                    },
                );
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator respects increased theme
        // visualDensity'.
        #[test]
        fn material2_decorator_respects_increased_theme_visual_density() {
            test_widgets(|tester| {
                check_visual_density(
                    tester,
                    VisualDensity::new(2.0, 2.0),
                    DensityFrame {
                        height: 64.0,
                        input_top: 32.0,
                        label_top: 24.0,
                        floating_label_top: 16.0,
                    },
                );
            });
        }

        // input_decorator_test.dart 'Material2 - prefix/suffix icons increase height of decoration
        // when larger than 48 by 48'.
        #[test]
        fn material2_prefix_and_suffix_icons_increase_height_when_larger_than_48_by_48() {
            test_widgets(|tester| {
                let prefix_key = GlobalKey::new();
                M2::new(
                    InputDecoration::new()
                        .prefix_icon(sized(&prefix_key, 100.0, 100.0))
                        .filled(true),
                )
                .pump(tester);

                assert_eq!(decorator_size(tester), Size::new(800.0, 100.0));
                assert_eq!(key_rect(tester, &prefix_key).height(), 100.0);
                assert_eq!(key_rect(tester, &prefix_key).top, 0.0);
            });
        }

        // input_decorator_test.dart 'Material2 - constraints - No InputDecorator constraints'.
        #[test]
        fn material2_no_constraints_fill_the_width_at_the_default_height() {
            test_widgets(|tester| {
                M2::new(InputDecoration::new()).pump(tester);

                // Should fill the screen width and be default height
                assert_eq!(decorator_size(tester), Size::new(800.0, 48.0));
            });
        }

        fn theme_with_constraints(app: &App) -> ThemeData {
            ThemeData::builder()
                .input_decoration_theme(InputDecorationThemeData::new().constraints(
                    BoxConstraints {
                        max_width: 300.0,
                        max_height: 40.0,
                        ..BoxConstraints::new()
                    },
                ))
                .build(app)
        }

        // input_decorator_test.dart 'Material2 - constraints - InputDecoratorThemeData constraints'.
        #[test]
        fn material2_theme_constraints_size_the_decorator() {
            test_widgets(|tester| {
                M2::new(InputDecoration::new())
                    .theme(theme_with_constraints)
                    .pump(tester);

                // Theme settings should make it 300x40 pixels
                assert_eq!(decorator_size(tester), Size::new(300.0, 40.0));
            });
        }

        // input_decorator_test.dart 'Material2 - constraints - InputDecorator constraints'.
        #[test]
        fn material2_decoration_constraints_override_the_theme() {
            test_widgets(|tester| {
                M2::new(InputDecoration::new().constraints(BoxConstraints {
                    max_width: 200.0,
                    max_height: 32.0,
                    ..BoxConstraints::new()
                }))
                .theme(theme_with_constraints)
                .pump(tester);

                // InputDecoration.constraints should override the theme. It should be
                // only 200x32 pixels
                assert_eq!(decorator_size(tester), Size::new(200.0, 32.0));
            });
        }

        /// Dart's `Text(text, style: TextStyle(fontFamily: 'FlutterTest', fontSize: 20.0))`, set so
        /// that everything works out to whole numbers.
        fn text_20() -> WidgetRef {
            Text::new(INPUT_TEXT)
                .style(TextStyle::new().font_size(20.0))
                .into_widget()
        }

        /// The input's top in an expanded Material 2 decorator with `decoration` aligned by
        /// `align`: the cases of 'textAlignVertical position'.
        fn expanded_input_top(
            tester: &mut WidgetTester,
            decoration: InputDecoration,
            align: TextAlignVertical,
        ) -> f64 {
            M2::new(decoration)
                .expands(true) // so we have a tall input where align can vary
                .text_align_vertical(align)
                .child(text_20())
                .pump(tester);
            top(tester, INPUT_TEXT)
        }

        // input_decorator_test.dart 'Material2 - textAlignVertical position - simple case - align
        // top (default)'.
        #[test]
        fn material2_text_align_vertical_top_puts_the_input_at_the_top() {
            test_widgets(|tester| {
                // Same as the default case above.
                assert_eq!(
                    expanded_input_top(
                        tester,
                        InputDecoration::new().filled(true),
                        TextAlignVertical::TOP
                    ),
                    12.0
                );
            });
        }

        // input_decorator_test.dart 'Material2 - textAlignVertical position - simple case - align
        // center'.
        #[test]
        fn material2_text_align_vertical_center_centers_the_input() {
            test_widgets(|tester| {
                // Below the top aligned case.
                assert_eq!(
                    expanded_input_top(
                        tester,
                        InputDecoration::new().filled(true),
                        TextAlignVertical::CENTER
                    ),
                    290.0
                );
            });
        }

        // input_decorator_test.dart 'Material2 - textAlignVertical position - simple case - align
        // bottom'.
        #[test]
        fn material2_text_align_vertical_bottom_puts_the_input_at_the_bottom() {
            test_widgets(|tester| {
                // Below the center aligned case.
                assert_eq!(
                    expanded_input_top(
                        tester,
                        InputDecoration::new().filled(true),
                        TextAlignVertical::BOTTOM
                    ),
                    568.0
                );
            });
        }

        // input_decorator_test.dart 'Material2 - textAlignVertical position - simple case - align
        // as a double'.
        #[test]
        fn material2_text_align_vertical_as_a_double_interpolates() {
            test_widgets(|tester| {
                // In between the center and bottom aligned cases.
                assert_eq!(
                    expanded_input_top(
                        tester,
                        InputDecoration::new().filled(true),
                        TextAlignVertical::new(0.75)
                    ),
                    498.5
                );
            });
        }

        // input_decorator_test.dart 'Material2 - textAlignVertical position - simple case - works
        // with density and content padding'.
        #[test]
        fn material2_text_align_vertical_works_with_density_and_content_padding() {
            test_widgets(|tester| {
                let key = GlobalKey::new();
                let container_key = GlobalKey::new();
                const TOTAL_HEIGHT: f64 = 100.0;
                const CHILD_HEIGHT: f64 = 20.0;
                let visual_density = VisualDensity::new(0.0, VisualDensity::MAXIMUM_DENSITY);
                let content_padding = EdgeInsets::only(0.0, 6.0, 0.0, 14.0);

                let decorator = M2::new(
                    InputDecoration::new()
                        .border(InputBorderRef::none())
                        .content_padding(EdgeInsetsGeometry::from(content_padding)),
                )
                .expands(true)
                .text_align_vertical(TextAlignVertical::CENTER)
                .visual_density(visual_density)
                .child(SizedBox::new().key(key.clone().into()).height(CHILD_HEIGHT))
                .build(tester);
                tester.pump_widget(
                    waft_widgets::Center::new()
                        .child(
                            SizedBox::new()
                                .key(container_key.clone().into())
                                .height(TOTAL_HEIGHT)
                                .child(decorator),
                        )
                        .into_widget(),
                );

                // Vertical components: contentPadding.vertical, densityOffset.y, child
                let density_dy = visual_density.base_size_adjustment().dy();
                let child_vertical_space_affordance =
                    TOTAL_HEIGHT - density_dy - content_padding.vertical();

                // TextAlignVertical.center is specified so `child` needs to be centered
                // in the available space.
                let child_margin = (child_vertical_space_affordance - CHILD_HEIGHT) / 2.0;
                let child_top = density_dy / 2.0 + content_padding.top + child_margin;

                assert_eq!(
                    key_rect(tester, &key).top,
                    key_rect(tester, &container_key).top + child_top
                );
            });
        }

        fn filled_outline() -> InputDecoration {
            InputDecoration::new()
                .filled(true)
                .border(OutlineInputBorder::new())
        }

        // input_decorator_test.dart 'Material2 - textAlignVertical position - outline border -
        // align top'.
        #[test]
        fn material2_outline_text_align_vertical_top_leaves_room_for_the_border() {
            test_widgets(|tester| {
                // Similar to the case without a border, but with a little extra room at
                // the top to make room for the border.
                assert_eq!(
                    expanded_input_top(tester, filled_outline(), TextAlignVertical::TOP),
                    24.0
                );
            });
        }

        // input_decorator_test.dart 'Material2 - textAlignVertical position - outline border -
        // align center (default)'.
        #[test]
        fn material2_outline_text_align_vertical_center_centers_the_input() {
            test_widgets(|tester| {
                // Below the top aligned case.
                assert_eq!(
                    expanded_input_top(tester, filled_outline(), TextAlignVertical::CENTER),
                    290.0
                );
            });
        }

        // input_decorator_test.dart 'Material2 - textAlignVertical position - outline border -
        // align bottom'.
        #[test]
        fn material2_outline_text_align_vertical_bottom_puts_the_input_at_the_bottom() {
            test_widgets(|tester| {
                // Below the center aligned case.
                assert_eq!(
                    expanded_input_top(tester, filled_outline(), TextAlignVertical::BOTTOM),
                    564.0
                );
            });
        }

        /// The input's and the prefix's tops for a tall prefix, under `align`, in an expanded
        /// decorator when `expands`.
        fn tall_prefix_tops(
            tester: &mut WidgetTester,
            outline: bool,
            expands: bool,
            align: TextAlignVertical,
        ) -> (f64, f64) {
            let p_key = GlobalKey::new();
            let mut decoration = InputDecoration::new()
                .prefix(sized(&p_key, 10.0, 100.0))
                .filled(true);
            if outline {
                decoration = decoration.border(OutlineInputBorder::new());
            }
            M2::new(decoration)
                .expands(expands)
                .text_align_vertical(align)
                .child(text_20())
                .pump(tester);
            (top(tester, INPUT_TEXT), key_rect(tester, &p_key).top)
        }

        // input_decorator_test.dart 'Material2 - textAlignVertical position - prefix - InputDecorator
        // tall prefix align top'.
        #[test]
        fn material2_tall_prefix_align_top() {
            test_widgets(|tester| {
                // Same as the default case above.
                assert_eq!(
                    tall_prefix_tops(tester, false, false, TextAlignVertical::TOP),
                    (97.0, 12.0)
                );
            });
        }

        // input_decorator_test.dart 'Material2 - textAlignVertical position - prefix - InputDecorator
        // tall prefix align center'.
        #[test]
        fn material2_tall_prefix_align_center() {
            test_widgets(|tester| {
                // Same as the default case above.
                assert_eq!(
                    tall_prefix_tops(tester, false, false, TextAlignVertical::CENTER),
                    (97.0, 12.0)
                );
            });
        }

        // input_decorator_test.dart 'Material2 - textAlignVertical position - prefix - InputDecorator
        // tall prefix align bottom'.
        #[test]
        fn material2_tall_prefix_align_bottom() {
            test_widgets(|tester| {
                // Top of the input + 100 prefix height - overlap
                assert_eq!(
                    tall_prefix_tops(tester, false, false, TextAlignVertical::BOTTOM),
                    (97.0, 12.0)
                );
            });
        }

        // input_decorator_test.dart 'Material2 - textAlignVertical position - outline border and
        // prefix - InputDecorator tall prefix align center'.
        #[test]
        fn material2_outline_tall_prefix_align_center() {
            test_widgets(|tester| {
                // In the middle of the expanded InputDecorator.
                assert_eq!(
                    tall_prefix_tops(tester, true, true, TextAlignVertical::CENTER),
                    (332.5, 247.5)
                );
            });
        }

        // input_decorator_test.dart 'Material2 - textAlignVertical position - outline border and
        // prefix - InputDecorator tall prefix with border align top'.
        #[test]
        fn material2_outline_tall_prefix_align_top() {
            test_widgets(|tester| {
                // Above the center example; the prefix is positioned at the top of the input, so
                // its value is the same as the top aligned test without a prefix.
                assert_eq!(
                    tall_prefix_tops(tester, true, true, TextAlignVertical::TOP),
                    (109.0, 24.0)
                );
            });
        }

        // input_decorator_test.dart 'Material2 - textAlignVertical position - outline border and
        // prefix - InputDecorator tall prefix with border align bottom'.
        #[test]
        fn material2_outline_tall_prefix_align_bottom() {
            test_widgets(|tester| {
                // Below the center example.
                assert_eq!(
                    tall_prefix_tops(tester, true, true, TextAlignVertical::BOTTOM),
                    (564.0, 479.0)
                );
            });
        }

        // input_decorator_test.dart 'Material2 - textAlignVertical position - outline border and
        // prefix - InputDecorator tall prefix with border align double'.
        #[test]
        fn material2_outline_tall_prefix_align_double() {
            test_widgets(|tester| {
                // Between the top and center examples.
                let (input_top, prefix_top) =
                    tall_prefix_tops(tester, true, true, TextAlignVertical::new(0.1));
                // Waft's `Matrix4` is f32, so a position read through `local_to_global` keeps f32
                // precision where Dart's is a double.
                assert_close(input_top, 355.65, 1e-4);
                assert_close(prefix_top, 270.65, 1e-4);
            });
        }

        fn filled_with_label() -> InputDecoration {
            InputDecoration::new().label_text(LABEL_TEXT).filled(true)
        }

        // input_decorator_test.dart 'Material2 - textAlignVertical position - label - align top
        // (default)'.
        #[test]
        fn material2_label_text_align_vertical_top_starts_the_input_lower() {
            test_widgets(|tester| {
                // The label causes the text to start slightly lower than it would
                // otherwise.
                assert_eq!(
                    expanded_input_top(tester, filled_with_label(), TextAlignVertical::TOP),
                    28.0
                );
            });
        }

        // input_decorator_test.dart 'Material2 - textAlignVertical position - label - align center'.
        #[test]
        fn material2_label_text_align_vertical_center_is_slightly_lower() {
            test_widgets(|tester| {
                // The label reduces the amount of space available for text, so the
                // center is slightly lower.
                assert_eq!(
                    expanded_input_top(tester, filled_with_label(), TextAlignVertical::CENTER),
                    298.0
                );
            });
        }

        // input_decorator_test.dart 'Material2 - textAlignVertical position - label - align bottom'.
        #[test]
        fn material2_label_text_align_vertical_bottom_keeps_the_bottom_line() {
            test_widgets(|tester| {
                // The label reduces the amount of space available for text, but the
                // bottom line is still in the same place.
                assert_eq!(
                    expanded_input_top(tester, filled_with_label(), TextAlignVertical::BOTTOM),
                    568.0
                );
            });
        }

        /// The decorator's size and the input's top and bottom for `decoration`, with Dart's
        /// border bottom and weight checked alongside.
        fn check_outline_alignment(
            tester: &mut WidgetTester,
            decoration: InputDecoration,
            size: Size,
            input_top: f64,
            border_weight: f64,
        ) {
            M2::new(decoration).pump(tester);
            assert_eq!(decorator_size(tester), size);
            assert_eq!(top(tester, INPUT_TEXT), input_top);
            assert_eq!(bottom(tester, INPUT_TEXT), input_top + 16.0);
            assert_eq!(get_border_bottom(tester), size.height());
            assert_eq!(get_border_weight(tester), border_weight);
        }

        // input_decorator_test.dart 'Material2 - OutlineInputBorder - default alignment - Centers
        // when border'.
        #[test]
        fn material2_outline_centers_the_input() {
            test_widgets(|tester| {
                check_outline_alignment(
                    tester,
                    InputDecoration::new().border(OutlineInputBorder::new()),
                    Size::new(800.0, 56.0),
                    20.0,
                    1.0,
                );
            });
        }

        // input_decorator_test.dart 'Material2 - OutlineInputBorder - default alignment - Centers
        // when border and label'.
        #[test]
        fn material2_outline_with_a_label_centers_the_input() {
            test_widgets(|tester| {
                check_outline_alignment(
                    tester,
                    InputDecoration::new()
                        .label_text(LABEL_TEXT)
                        .border(OutlineInputBorder::new()),
                    Size::new(800.0, 56.0),
                    20.0,
                    1.0,
                );
            });
        }

        // input_decorator_test.dart 'Material2 - OutlineInputBorder - default alignment - Centers
        // when border and contentPadding'.
        #[test]
        fn material2_outline_with_content_padding_centers_the_input() {
            test_widgets(|tester| {
                check_outline_alignment(
                    tester,
                    InputDecoration::new()
                        .border(OutlineInputBorder::new())
                        .content_padding(EdgeInsetsGeometry::from(EdgeInsets::from_ltrb(
                            12.0, 14.0, 8.0, 14.0,
                        ))),
                    Size::new(800.0, 48.0),
                    16.0,
                    1.0,
                );
            });
        }

        // input_decorator_test.dart 'Material2 - OutlineInputBorder - default alignment - Centers
        // when border and contentPadding and label'.
        #[test]
        fn material2_outline_with_content_padding_and_label_centers_the_input() {
            test_widgets(|tester| {
                check_outline_alignment(
                    tester,
                    InputDecoration::new()
                        .label_text(LABEL_TEXT)
                        .border(OutlineInputBorder::new())
                        .content_padding(EdgeInsetsGeometry::from(EdgeInsets::from_ltrb(
                            12.0, 14.0, 8.0, 14.0,
                        ))),
                    Size::new(800.0, K_MIN_INTERACTIVE_DIMENSION),
                    16.0,
                    1.0,
                );
            });
        }

        // input_decorator_test.dart 'Material2 - OutlineInputBorder - default alignment - Centers
        // when border and lopsided contentPadding and label'.
        #[test]
        fn material2_outline_with_lopsided_content_padding_and_label_centers_the_input() {
            test_widgets(|tester| {
                check_outline_alignment(
                    tester,
                    InputDecoration::new()
                        .label_text(LABEL_TEXT)
                        .border(OutlineInputBorder::new())
                        .content_padding(EdgeInsetsGeometry::from(EdgeInsets::from_ltrb(
                            12.0, 104.0, 8.0, 0.0,
                        ))),
                    Size::new(800.0, 120.0),
                    52.0,
                    1.0,
                );
            });
        }

        // input_decorator_test.dart 'Material2 - OutlineInputBorder - default alignment - Label
        // aligns horizontally with text'.
        #[test]
        fn material2_outline_label_aligns_horizontally_with_the_text() {
            test_widgets(|tester| {
                M2::new(
                    InputDecoration::new()
                        .prefix_icon(Icon::new(Some(Icons::ac_unit())))
                        .label_text(LABEL_TEXT)
                        .border(OutlineInputBorder::new()),
                )
                .is_focused(true)
                .pump(tester);

                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(text_rect(tester, LABEL_TEXT).left, 48.0);
                assert_eq!(text_rect(tester, INPUT_TEXT).left, 48.0);
                assert_eq!(get_border_weight(tester), 2.0);
            });
        }

        // input_decorator_test.dart 'Material2 - OutlineInputBorder - default alignment - Floating
        // label for filled input decoration is horizontally aligned with text'.
        #[test]
        fn material2_filled_floating_label_aligns_horizontally_with_the_text() {
            test_widgets(|tester| {
                M2::new(
                    InputDecoration::new()
                        .prefix_icon(Icon::new(Some(Icons::ac_unit())))
                        .label_text(LABEL_TEXT)
                        .filled(true),
                )
                .is_focused(true)
                .pump(tester);

                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(text_rect(tester, LABEL_TEXT).left, 48.0);
                assert_eq!(text_rect(tester, INPUT_TEXT).left, 48.0);
                assert_eq!(get_border_weight(tester), 2.0);
            });
        }

        /// The input's top and bottom in an expanded outlined decorator with `padding`, aligned by
        /// `align`: the '3 point interpolation alignment' cases.
        fn check_three_point(
            tester: &mut WidgetTester,
            align: TextAlignVertical,
            padding: EdgeInsets,
            input_top: f64,
        ) {
            M2::new(
                InputDecoration::new()
                    .border(OutlineInputBorder::new())
                    .content_padding(EdgeInsetsGeometry::from(padding)),
            )
            .expands(true)
            .text_align_vertical(align)
            .pump(tester);

            assert_eq!(decorator_size(tester), Size::new(800.0, 600.0));
            assert_eq!(top(tester, INPUT_TEXT), input_top);
            assert_eq!(bottom(tester, INPUT_TEXT), input_top + 16.0);
            assert_eq!(get_border_bottom(tester), 600.0);
            assert_eq!(get_border_weight(tester), 1.0);
        }

        // input_decorator_test.dart 'Material2 - OutlineInputBorder - 3 point interpolation
        // alignment - top align includes padding'.
        #[test]
        fn material2_three_point_top_align_includes_padding() {
            test_widgets(|tester| {
                // Aligned to the top including the 24px padding.
                check_three_point(
                    tester,
                    TextAlignVertical::TOP,
                    EdgeInsets::from_ltrb(12.0, 24.0, 8.0, 2.0),
                    24.0,
                );
            });
        }

        // input_decorator_test.dart 'Material2 - OutlineInputBorder - 3 point interpolation
        // alignment - center align ignores padding'.
        #[test]
        fn material2_three_point_center_align_ignores_padding() {
            test_widgets(|tester| {
                // Baseline is on the center of the 600px high input.
                check_three_point(
                    tester,
                    TextAlignVertical::CENTER,
                    EdgeInsets::from_ltrb(12.0, 24.0, 8.0, 2.0),
                    292.0,
                );
            });
        }

        // input_decorator_test.dart 'Material2 - OutlineInputBorder - 3 point interpolation
        // alignment - bottom align includes padding'.
        #[test]
        fn material2_three_point_bottom_align_includes_padding() {
            test_widgets(|tester| {
                // Includes bottom padding of 2px.
                check_three_point(
                    tester,
                    TextAlignVertical::BOTTOM,
                    EdgeInsets::from_ltrb(12.0, 24.0, 8.0, 2.0),
                    582.0,
                );
            });
        }

        // input_decorator_test.dart 'Material2 - OutlineInputBorder - 3 point interpolation
        // alignment - padding exceeds middle keeps top at middle'.
        #[test]
        fn material2_three_point_padding_beyond_the_middle_keeps_the_top_at_the_middle() {
            test_widgets(|tester| {
                // Same position as the center example above.
                check_three_point(
                    tester,
                    TextAlignVertical::TOP,
                    EdgeInsets::from_ltrb(12.0, 504.0, 8.0, 0.0),
                    292.0,
                );
            });
        }

        /// The decorator's size and the counter's rect for a filled decoration with a counter.
        fn counter_layout(
            tester: &mut WidgetTester,
            text_direction: TextDirection,
            is_dense: bool,
        ) -> (Size, Rect, Rect) {
            M2::new(
                InputDecoration::new()
                    .counter_text("test")
                    .filled(true)
                    .is_dense(is_dense),
            )
            .text_direction(text_direction)
            .pump(tester);
            (
                decorator_size(tester),
                get_decorator_rect(tester),
                text_rect(tester, "test"),
            )
        }

        // input_decorator_test.dart 'Material2 - counter text has correct right margin - LTR, not
        // dense'.
        #[test]
        fn material2_counter_right_margin_ltr_not_dense() {
            test_widgets(|tester| {
                // Margin for text decoration is 12 when filled
                // (dx) - 12 = (text offset)x.
                let (size, decorator, counter) = counter_layout(tester, TextDirection::Ltr, false);
                assert_eq!(size, Size::new(800.0, 68.0));
                assert_eq!(counter.right, decorator.right - 12.0);
            });
        }

        // input_decorator_test.dart 'Material2 - counter text has correct right margin - RTL, not
        // dense'.
        #[test]
        fn material2_counter_right_margin_rtl_not_dense() {
            test_widgets(|tester| {
                // Margin for text decoration is 12 when filled and top left offset is (0, 0)
                // 0 + 12 = 12.
                let (size, _, counter) = counter_layout(tester, TextDirection::Rtl, false);
                assert_eq!(size, Size::new(800.0, 68.0));
                assert_eq!(counter.left, 12.0);
            });
        }

        // input_decorator_test.dart 'Material2 - counter text has correct right margin - LTR,
        // dense'.
        #[test]
        fn material2_counter_right_margin_ltr_dense() {
            test_widgets(|tester| {
                let (size, decorator, counter) = counter_layout(tester, TextDirection::Ltr, true);
                assert_eq!(size, Size::new(800.0, 52.0));
                assert_eq!(counter.right, decorator.right - 12.0);
            });
        }

        // input_decorator_test.dart 'Material2 - counter text has correct right margin - RTL, dense'.
        #[test]
        fn material2_counter_right_margin_rtl_dense() {
            test_widgets(|tester| {
                let (size, _, counter) = counter_layout(tester, TextDirection::Rtl, true);
                assert_eq!(size, Size::new(800.0, 52.0));
                assert_eq!(counter.left, 12.0);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator error/helper/counter RTL layout'.
        #[test]
        fn material2_error_helper_and_counter_rtl_layout() {
            test_widgets(|tester| {
                M2::new(
                    InputDecoration::new()
                        .label_text(LABEL_TEXT)
                        .helper_text(HELPER_TEXT)
                        .counter_text(COUNTER_TEXT)
                        .filled(true),
                )
                .text_direction(TextDirection::Rtl)
                .pump(tester);

                assert_eq!(decorator_size(tester), Size::new(800.0, 76.0));
                assert_eq!(top(tester, INPUT_TEXT), 28.0);
                assert_eq!(bottom(tester, INPUT_TEXT), 44.0);
                assert_eq!(top(tester, LABEL_TEXT), 12.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 24.0);
                assert_eq!(get_border_bottom(tester), 56.0);
                assert_eq!(get_border_weight(tester), 1.0);
                assert_eq!(
                    text_rect(tester, COUNTER_TEXT).top_left(),
                    Offset::new(12.0, 64.0)
                );
                assert_eq!(
                    text_rect(tester, HELPER_TEXT).top_right(),
                    Offset::new(788.0, 64.0)
                );

                // If both error and helper are specified, show the error
                M2::new(
                    InputDecoration::new()
                        .label_text(LABEL_TEXT)
                        .helper_text(HELPER_TEXT)
                        .error_text(ERROR_TEXT)
                        .counter_text(COUNTER_TEXT)
                        .filled(true),
                )
                .text_direction(TextDirection::Rtl)
                .pump(tester);
                tester.pump_and_settle();
                assert_eq!(
                    text_rect(tester, COUNTER_TEXT).top_left(),
                    Offset::new(12.0, 64.0)
                );
                assert_eq!(
                    text_rect(tester, ERROR_TEXT).top_right(),
                    Offset::new(788.0, 64.0)
                );
                assert!(tester.find_text(HELPER_TEXT).is_empty());
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator prefix/suffix RTL'.
        #[test]
        fn material2_prefix_and_suffix_rtl() {
            test_widgets(|tester| {
                M2::new(
                    InputDecoration::new()
                        .prefix_text("p")
                        .suffix_text("s")
                        .filled(true),
                )
                .text_direction(TextDirection::Rtl)
                .pump(tester);

                assert_eq!(
                    decorator_size(tester),
                    Size::new(800.0, K_MIN_INTERACTIVE_DIMENSION)
                ); // 40 bumped up to minimum.
                assert_eq!(text_rect(tester, INPUT_TEXT).height(), 16.0);
                assert_eq!(text_rect(tester, "p").height(), 16.0);
                assert_eq!(text_rect(tester, "s").height(), 16.0);
                assert_eq!(top(tester, INPUT_TEXT), 16.0);
                assert_eq!(top(tester, "p"), 16.0);
                assert_eq!(top(tester, "s"), 16.0);

                // layout is a row: [s text p]
                assert_eq!(text_rect(tester, "s").left, 12.0);
                assert!(text_rect(tester, "s").right <= text_rect(tester, INPUT_TEXT).left);
                assert!(text_rect(tester, INPUT_TEXT).right <= text_rect(tester, "p").left);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator contentPadding RTL layout'.
        #[test]
        fn material2_content_padding_rtl_layout() {
            test_widgets(|tester| {
                let decoration = || {
                    InputDecoration::new()
                        .content_padding(EdgeInsetsGeometry::from(EdgeInsetsDirectional::only(
                            40.0, 12.0, 0.0, 12.0,
                        )))
                        .label_text(LABEL_TEXT)
                        .hint_text(HINT_TEXT)
                        .filled(true)
                };
                // LTR: content left edge is contentPadding.start: 40.0
                M2::new(decoration()).pump(tester);
                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(text_rect(tester, INPUT_TEXT).left, 40.0);
                assert_eq!(text_rect(tester, LABEL_TEXT).left, 40.0);
                assert_eq!(text_rect(tester, HINT_TEXT).left, 40.0);

                // RTL: content right edge is 800 - contentPadding.start: 760.0.
                M2::new(decoration())
                    .is_focused(true) // label is floating, still adjusted for contentPadding
                    .text_direction(TextDirection::Rtl)
                    .pump(tester);
                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(text_rect(tester, INPUT_TEXT).right, 760.0);
                assert_eq!(text_rect(tester, LABEL_TEXT).right, 760.0);
                assert_eq!(text_rect(tester, HINT_TEXT).right, 760.0);
            });
        }

        fn check_input_width(
            tester: &mut WidgetTester,
            decoration: InputDecoration,
            height: f64,
            left: f64,
            right: f64,
        ) {
            M2::new(decoration).pump(tester);
            assert_eq!(decorator_size(tester), Size::new(800.0, height));
            assert_eq!(text_rect(tester, INPUT_TEXT).left, left);
            assert_eq!(text_rect(tester, INPUT_TEXT).right, right);
        }

        // input_decorator_test.dart 'Material2 - inputText width - outline textField'.
        #[test]
        fn material2_outline_input_text_width() {
            test_widgets(|tester| {
                check_input_width(
                    tester,
                    InputDecoration::new().border(OutlineInputBorder::new()),
                    56.0,
                    12.0,
                    788.0,
                );
            });
        }

        // input_decorator_test.dart 'Material2 - inputText width - outline textField with prefix and
        // suffix icons'.
        #[test]
        fn material2_outline_input_text_width_with_prefix_and_suffix_icons() {
            test_widgets(|tester| {
                check_input_width(
                    tester,
                    InputDecoration::new()
                        .border(OutlineInputBorder::new())
                        .prefix_icon(Icon::new(Some(Icons::visibility())))
                        .suffix_icon(Icon::new(Some(Icons::close()))),
                    56.0,
                    48.0,
                    752.0,
                );
            });
        }

        // input_decorator_test.dart 'Material2 - inputText width - filled textField'.
        #[test]
        fn material2_filled_input_text_width() {
            test_widgets(|tester| {
                check_input_width(
                    tester,
                    InputDecoration::new().filled(true),
                    48.0,
                    12.0,
                    788.0,
                );
            });
        }

        // input_decorator_test.dart 'Material2 - inputText width - filled textField with prefix and
        // suffix icons'.
        #[test]
        fn material2_filled_input_text_width_with_prefix_and_suffix_icons() {
            test_widgets(|tester| {
                check_input_width(
                    tester,
                    InputDecoration::new()
                        .filled(true)
                        .prefix_icon(Icon::new(Some(Icons::visibility())))
                        .suffix_icon(Icon::new(Some(Icons::close()))),
                    48.0,
                    48.0,
                    752.0,
                );
            });
        }

        /// The label's rect in Dart's `buildInputDecoratorWithFloatingLabel` of the Material 2
        /// 'floatingLabelAlignment' group.
        fn floating_label_rect(
            tester: &mut WidgetTester,
            text_direction: TextDirection,
            has_icon: bool,
            alignment: FloatingLabelAlignment,
            border_is_outline: bool,
        ) -> Rect {
            let mut decoration = InputDecoration::new()
                .content_padding(EdgeInsetsGeometry::from(EdgeInsetsDirectional::only(
                    40.0, 12.0, 0.0, 12.0,
                )))
                .floating_label_alignment(alignment)
                .label_text(LABEL_TEXT)
                .hint_text(HINT_TEXT)
                .filled(true);
            if has_icon {
                decoration = decoration.icon(Icon::new(Some(Icons::insert_link())));
            }
            if border_is_outline {
                decoration = decoration.border(OutlineInputBorder::new());
            }
            M2::new(decoration)
                .text_direction(text_direction)
                .pump(tester);
            get_label_rect(tester)
        }

        // input_decorator_test.dart 'Material2 - floatingLabelAlignment - LTR with icon aligned -
        // start'.
        #[test]
        fn material2_floating_label_starts_after_the_icon_ltr() {
            test_widgets(|tester| {
                for outline in [false, true] {
                    // icon (40) + contentPadding (40)
                    let rect = floating_label_rect(
                        tester,
                        TextDirection::Ltr,
                        true,
                        FloatingLabelAlignment::START,
                        outline,
                    );
                    assert_eq!(rect.left, 80.0);
                }
            });
        }

        // input_decorator_test.dart 'Material2 - floatingLabelAlignment - LTR with icon aligned -
        // center'.
        #[test]
        fn material2_floating_label_centers_beside_the_icon_ltr() {
            test_widgets(|tester| {
                for outline in [false, true] {
                    // icon (40) + (decorator (800) - icon (40)) / 2
                    let rect = floating_label_rect(
                        tester,
                        TextDirection::Ltr,
                        true,
                        FloatingLabelAlignment::CENTER,
                        outline,
                    );
                    assert_eq!(rect.center().dx(), 420.0);
                }
            });
        }

        // input_decorator_test.dart 'Material2 - floatingLabelAlignment - LTR without icon aligned -
        // start'.
        #[test]
        fn material2_floating_label_starts_at_the_padding_ltr() {
            test_widgets(|tester| {
                for outline in [false, true] {
                    // contentPadding (40)
                    let rect = floating_label_rect(
                        tester,
                        TextDirection::Ltr,
                        false,
                        FloatingLabelAlignment::START,
                        outline,
                    );
                    assert_eq!(rect.left, 40.0);
                }
            });
        }

        // input_decorator_test.dart 'Material2 - floatingLabelAlignment - LTR without icon aligned -
        // center'.
        #[test]
        fn material2_floating_label_centers_in_the_decorator_ltr() {
            test_widgets(|tester| {
                for outline in [false, true] {
                    // decorator (800) / 2
                    let rect = floating_label_rect(
                        tester,
                        TextDirection::Ltr,
                        false,
                        FloatingLabelAlignment::CENTER,
                        outline,
                    );
                    assert_eq!(rect.center().dx(), 400.0);
                }
            });
        }

        // input_decorator_test.dart 'Material2 - floatingLabelAlignment - RTL with icon aligned -
        // start'.
        #[test]
        fn material2_floating_label_starts_after_the_icon_rtl() {
            test_widgets(|tester| {
                for outline in [false, true] {
                    // decorator (800) - icon (40) - contentPadding (40)
                    let rect = floating_label_rect(
                        tester,
                        TextDirection::Rtl,
                        true,
                        FloatingLabelAlignment::START,
                        outline,
                    );
                    assert_eq!(rect.right, 720.0);
                }
            });
        }

        // input_decorator_test.dart 'Material2 - floatingLabelAlignment - RTL with icon aligned -
        // center'.
        #[test]
        fn material2_floating_label_centers_beside_the_icon_rtl() {
            test_widgets(|tester| {
                for outline in [false, true] {
                    // (decorator (800) / icon (40)) / 2
                    let rect = floating_label_rect(
                        tester,
                        TextDirection::Rtl,
                        true,
                        FloatingLabelAlignment::CENTER,
                        outline,
                    );
                    assert_eq!(rect.center().dx(), 380.0);
                }
            });
        }

        // input_decorator_test.dart 'Material2 - floatingLabelAlignment - RTL without icon aligned -
        // start'.
        #[test]
        fn material2_floating_label_starts_at_the_padding_rtl() {
            test_widgets(|tester| {
                for outline in [false, true] {
                    // decorator (800) - contentPadding (40)
                    let rect = floating_label_rect(
                        tester,
                        TextDirection::Rtl,
                        false,
                        FloatingLabelAlignment::START,
                        outline,
                    );
                    assert_eq!(rect.right, 760.0);
                }
            });
        }

        // input_decorator_test.dart 'Material2 - floatingLabelAlignment - RTL without icon aligned -
        // center'.
        #[test]
        fn material2_floating_label_centers_in_the_decorator_rtl() {
            test_widgets(|tester| {
                for outline in [false, true] {
                    // decorator (800) / 2
                    let rect = floating_label_rect(
                        tester,
                        TextDirection::Rtl,
                        false,
                        FloatingLabelAlignment::CENTER,
                        outline,
                    );
                    assert_eq!(rect.center().dx(), 400.0);
                }
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator prefix/suffix dense layout'.
        #[test]
        fn material2_prefix_and_suffix_dense_layout() {
            test_widgets(|tester| {
                M2::new(
                    InputDecoration::new()
                        .is_dense(true)
                        .prefix_text("p")
                        .suffix_text("s")
                        .filled(true),
                )
                .is_focused(true)
                .pump(tester);

                // The only difference from normal layout for this case is that the
                // padding above and below the prefix, input text, suffix, is 8 instead of 12.
                assert_eq!(decorator_size(tester), Size::new(800.0, 32.0));
                assert_eq!(text_rect(tester, INPUT_TEXT).height(), 16.0);
                assert_eq!(text_rect(tester, "p").height(), 16.0);
                assert_eq!(text_rect(tester, "s").height(), 16.0);
                assert_eq!(top(tester, INPUT_TEXT), 8.0);
                assert_eq!(top(tester, "p"), 8.0);
                assert_eq!(text_rect(tester, "p").left, 12.0);
                assert_eq!(top(tester, "s"), 8.0);
                assert_eq!(text_rect(tester, "s").right, 788.0);

                // layout is a row: [p text s]
                assert_eq!(text_rect(tester, "p").left, 12.0);
                assert!(text_rect(tester, "p").right <= text_rect(tester, INPUT_TEXT).left);
                assert!(text_rect(tester, INPUT_TEXT).right <= text_rect(tester, "s").left);

                assert_eq!(get_border_bottom(tester), 32.0);
                assert_eq!(get_border_weight(tester), 2.0);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator with empty InputDecoration'.
        #[test]
        fn material2_empty_decoration_is_bumped_up_to_the_minimum_height() {
            test_widgets(|tester| {
                M2::new(InputDecoration::new()).pump(tester);

                assert_eq!(
                    decorator_size(tester),
                    Size::new(800.0, K_MIN_INTERACTIVE_DIMENSION)
                ); // 40 bumped up to minimum.
                assert_eq!(text_rect(tester, INPUT_TEXT).height(), 16.0);
                assert_eq!(top(tester, INPUT_TEXT), 16.0);
                assert_eq!(get_border_bottom(tester), K_MIN_INTERACTIVE_DIMENSION); // 40 bumped up to minimum.
                assert_eq!(get_border_weight(tester), 1.0);
            });
        }

        // input_decorator_test.dart 'Material2 - contentPadding smaller than
        // kMinInteractiveDimension'.
        #[test]
        fn material2_content_padding_smaller_than_the_minimum_interactive_dimension() {
            test_widgets(|tester| {
                const VERTICAL_PADDING: f64 = 1.0;
                M2::new(
                    InputDecoration::new()
                        .hint_text(HINT_TEXT)
                        .content_padding(EdgeInsetsGeometry::from(EdgeInsets::symmetric(
                            VERTICAL_PADDING,
                            0.0,
                        )))
                        .is_dense(true),
                )
                .pump(tester);

                // The overall height is 18dps. This is shorter than
                // kMinInteractiveDimension, but because isDense is true, the minimum is
                // ignored.
                assert_eq!(decorator_size(tester), Size::new(800.0, 18.0));
                assert_eq!(text_rect(tester, INPUT_TEXT).height(), 16.0);
                assert_eq!(top(tester, INPUT_TEXT), 1.0);
                assert_eq!(get_opacity(tester, HINT_TEXT), 0.0);
                assert_eq!(get_border_weight(tester), 1.0);

                // InputDecoration.collapsed does not support contentPadding
                M2::new(InputDecoration::collapsed(None).hint_text(HINT_TEXT)).pump(tester);

                // The overall height is 16dps. This is shorter than
                // kMinInteractiveDimension, but because isCollapsed is true, the minimum is
                // ignored. There is no padding at all, because isCollapsed doesn't support
                // contentPadding.
                assert_eq!(decorator_size(tester), Size::new(800.0, 16.0));
                assert_eq!(text_rect(tester, INPUT_TEXT).height(), 16.0);
                assert_eq!(top(tester, INPUT_TEXT), 0.0);
                assert_eq!(get_opacity(tester, HINT_TEXT), 0.0);
                assert_eq!(get_border_weight(tester), 1.0);

                M2::new(InputDecoration::new().hint_text(HINT_TEXT).content_padding(
                    EdgeInsetsGeometry::from(EdgeInsets::symmetric(VERTICAL_PADDING, 0.0)),
                ))
                .pump(tester);

                // The requested overall height is 18dps, however the minimum height is
                // kMinInteractiveDimension because neither isDense or isCollapsed are true.
                assert_eq!(
                    decorator_size(tester),
                    Size::new(800.0, K_MIN_INTERACTIVE_DIMENSION)
                );
                assert_eq!(text_rect(tester, INPUT_TEXT).height(), 16.0);
                assert_eq!(top(tester, INPUT_TEXT), 16.0);
                assert_eq!(get_opacity(tester, HINT_TEXT), 0.0);
                assert_eq!(get_border_weight(tester), 0.0);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator.collapsed'.
        #[test]
        fn material2_collapsed_decorator_has_no_minimum_height() {
            test_widgets(|tester| {
                M2::new(InputDecoration::collapsed(None).hint_text(HINT_TEXT)).pump(tester);

                // Overall height for this InputDecorator is 16dps. There is no minimum
                // height when InputDecoration.collapsed is used.
                assert_eq!(decorator_size(tester), Size::new(800.0, 16.0));
                assert_eq!(text_rect(tester, INPUT_TEXT).height(), 16.0);
                assert_eq!(top(tester, INPUT_TEXT), 0.0);
                assert_eq!(get_opacity(tester, HINT_TEXT), 0.0);
                assert_eq!(get_border_weight(tester), 0.0);

                // The hint should appear
                M2::new(InputDecoration::collapsed(None).hint_text(HINT_TEXT))
                    .is_empty(true)
                    .is_focused(true)
                    .pump(tester);
                tester.pump_and_settle();

                assert_eq!(decorator_size(tester), Size::new(800.0, 16.0));
                assert_eq!(text_rect(tester, INPUT_TEXT).height(), 16.0);
                assert_eq!(top(tester, INPUT_TEXT), 0.0);
                assert_eq!(text_rect(tester, HINT_TEXT).height(), 16.0);
                assert_eq!(top(tester, HINT_TEXT), 0.0);
                assert_eq!(get_border_weight(tester), 0.0);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator with baseStyle'.
        #[test]
        fn material2_decorator_with_base_style() {
            test_widgets(|tester| {
                // Setting the baseStyle of the InputDecoration and the style of the input
                // text child to a smaller font reduces the InputDecoration's vertical size.
                let style = TextStyle::new().font_size(10.0);
                M2::new(
                    InputDecoration::new()
                        .hint_text(HINT_TEXT)
                        .label_text(LABEL_TEXT),
                )
                .is_empty(true)
                .base_style(style.clone())
                .child(Text::new(INPUT_TEXT).style(style))
                .pump(tester);

                assert_eq!(
                    decorator_size(tester),
                    Size::new(800.0, K_MIN_INTERACTIVE_DIMENSION)
                ); // 45.5 bumped up to minimum.
                assert_eq!(text_rect(tester, HINT_TEXT).height(), 10.0);
                assert_eq!(text_rect(tester, LABEL_TEXT).height(), 10.0);
                assert_eq!(text_rect(tester, INPUT_TEXT).height(), 10.0);
                assert_eq!(top(tester, HINT_TEXT), 24.75);
                assert_eq!(top(tester, LABEL_TEXT), 19.0);
                assert_eq!(top(tester, INPUT_TEXT), 24.75);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator with empty style overrides'.
        #[test]
        fn material2_empty_style_overrides_lay_out_as_no_overrides() {
            test_widgets(|tester| {
                // Same as not specifying any style overrides
                M2::new(
                    InputDecoration::new()
                        .label_text(LABEL_TEXT)
                        .hint_text(HINT_TEXT)
                        .helper_text(HELPER_TEXT)
                        .counter_text(COUNTER_TEXT)
                        .label_style(TextStyle::new())
                        .hint_style(TextStyle::new())
                        .error_style(TextStyle::new())
                        .helper_style(TextStyle::new())
                        .filled(true),
                )
                .pump(tester);

                // Label is floating because isEmpty is false.
                assert_eq!(decorator_size(tester), Size::new(800.0, 76.0));
                assert_eq!(top(tester, INPUT_TEXT), 28.0);
                assert_eq!(bottom(tester, INPUT_TEXT), 44.0);
                assert_eq!(top(tester, LABEL_TEXT), 12.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 24.0);
                assert_eq!(get_border_bottom(tester), 56.0);
                assert_eq!(get_border_weight(tester), 1.0);
                assert_eq!(
                    text_rect(tester, HELPER_TEXT).top_left(),
                    Offset::new(12.0, 64.0)
                );
                assert_eq!(
                    text_rect(tester, COUNTER_TEXT).top_right(),
                    Offset::new(788.0, 64.0)
                );
            });
        }

        fn borderless_outline_never_floating() -> InputDecoration {
            InputDecoration::new()
                .border(OutlineInputBorder::new().border_side(BorderSide::NONE))
                .floating_label_behavior(FloatingLabelBehavior::Never)
                .label_text(LABEL_TEXT)
        }

        // input_decorator_test.dart 'Material2 - InputDecoration outline shape with no border and no
        // floating placeholder'.
        #[test]
        fn material2_outline_shape_with_no_border_and_no_floating_placeholder() {
            test_widgets(|tester| {
                M2::new(borderless_outline_never_floating())
                    .is_empty(true)
                    .pump(tester);

                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(top(tester, LABEL_TEXT), 20.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 36.0);
                assert_eq!(get_border_bottom(tester), 56.0);
                assert_eq!(get_border_weight(tester), 0.0);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecoration outline shape with no border and no
        // floating placeholder not empty'.
        #[test]
        fn material2_outline_shape_with_no_border_and_no_floating_placeholder_not_empty() {
            test_widgets(|tester| {
                M2::new(borderless_outline_never_floating()).pump(tester);

                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(top(tester, LABEL_TEXT), 20.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 36.0);
                assert_eq!(get_border_bottom(tester), 56.0);
                assert_eq!(get_border_weight(tester), 0.0);

                // The label should not be seen.
                assert_eq!(get_opacity(tester, LABEL_TEXT), 0.0);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorationThemeData outline border'.
        #[test]
        fn material2_theme_outline_border_centers_the_label() {
            test_widgets(|tester| {
                M2::new(InputDecoration::new().label_text(LABEL_TEXT))
                    .is_empty(true) // label appears, vertically centered
                    .input_decoration_theme(
                        InputDecorationThemeData::new().border(OutlineInputBorder::new()),
                    )
                    .pump(tester);

                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(top(tester, LABEL_TEXT), 20.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 36.0);
                assert_eq!(get_border_bottom(tester), 56.0);
                assert_eq!(get_border_weight(tester), 1.0);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorationThemeData outline border, dense
        // layout'.
        #[test]
        fn material2_theme_outline_border_dense_layout() {
            test_widgets(|tester| {
                M2::new(label_and_hint())
                    .is_empty(true) // label appears, vertically centered
                    .input_decoration_theme(
                        InputDecorationThemeData::new()
                            .border(OutlineInputBorder::new())
                            .is_dense(true),
                    )
                    .pump(tester);

                assert_eq!(decorator_size(tester), Size::new(800.0, 48.0));
                assert_eq!(top(tester, LABEL_TEXT), 16.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 32.0);
                assert_eq!(get_border_bottom(tester), 48.0);
                assert_eq!(get_border_weight(tester), 1.0);
            });
        }

        /// The styles of 'InputDecorationThemeData style overrides' and its focused twin.
        struct OverrideStyles {
            label_style: TextStyle,
            floating_label_style: TextStyle,
            hint_style: TextStyle,
            prefix_style: TextStyle,
            suffix_style: TextStyle,
            helper_style: TextStyle,
            counter_style: TextStyle,
        }

        impl OverrideStyles {
            fn new() -> OverrideStyles {
                let default_style = TextStyle::new().font_size(16.0);
                let style12 = TextStyle::new().font_size(12.0);
                let with_color = |style: &TextStyle, color: Color| {
                    style.merge(Some(&TextStyle::new().color(color)))
                };
                OverrideStyles {
                    label_style: with_color(&default_style, Colors::RED.primary()),
                    floating_label_style: with_color(&default_style, Colors::INDIGO.primary()),
                    hint_style: with_color(&default_style, Colors::GREEN.primary()),
                    prefix_style: with_color(&default_style, Colors::BLUE.primary()),
                    suffix_style: with_color(&default_style, Colors::PURPLE.primary()),
                    helper_style: with_color(&style12, Colors::ORANGE.primary()),
                    counter_style: with_color(&style12, Colors::ORANGE.primary()),
                }
            }

            fn theme(&self, floating: bool) -> InputDecorationThemeData {
                let mut theme = InputDecorationThemeData::new()
                    .label_style(self.label_style.clone())
                    .hint_style(self.hint_style.clone())
                    .prefix_style(self.prefix_style.clone())
                    .suffix_style(self.suffix_style.clone())
                    .helper_style(self.helper_style.clone())
                    .counter_style(self.counter_style.clone());
                if floating {
                    theme = theme.floating_label_style(self.floating_label_style.clone());
                }
                // filled: false (default) - don't pad by left/right 12dps
                theme
            }
        }

        fn text_style_color(tester: &WidgetTester, text: &str) -> Option<Color> {
            let element = tester.find_text(text)[0];
            tester.widget(element, |text: &waft_widgets::Text| {
                text.style
                    .as_ref()
                    .and_then(|style| style.color.as_ref().map(|color| **color))
            })
        }

        fn color_of(style: &TextStyle) -> Option<Color> {
            style.color.as_ref().map(|color| **color)
        }

        /// Dart's 'InputDecorationThemeData style overrides' (and its focused twin): the layout
        /// with the overridden styles' metrics, then the styles passed along with
        /// Dart's strings.
        fn check_style_overrides(
            tester: &mut WidgetTester,
            focused: bool,
            label_top: f64,
            label_height: f64,
            weight: f64,
        ) {
            let styles = OverrideStyles::new();
            M2::new(
                InputDecoration::new()
                    .label_text(LABEL_TEXT)
                    .hint_text(HINT_TEXT)
                    .prefix_text("prefix")
                    .suffix_text("suffix")
                    .helper_text(HELPER_TEXT)
                    .counter_text(COUNTER_TEXT),
            )
            .is_empty(true)
            .is_focused(focused)
            .input_decoration_theme(styles.theme(focused))
            .pump(tester);

            // Overall height for this InputDecorator is 76dps.
            assert_eq!(decorator_size(tester), Size::new(800.0, 76.0));
            assert_eq!(top(tester, LABEL_TEXT), label_top);
            assert_eq!(bottom(tester, LABEL_TEXT), label_top + label_height);
            assert_eq!(get_border_bottom(tester), 56.0);
            assert_eq!(get_border_weight(tester), weight);
            assert_eq!(
                text_rect(tester, HELPER_TEXT).top_left(),
                Offset::new(0.0, 64.0)
            );
            assert_eq!(
                text_rect(tester, COUNTER_TEXT).top_right(),
                Offset::new(800.0, 64.0)
            );

            // Verify that the styles were passed along
            M2::new(
                InputDecoration::new()
                    .label_text(LABEL_TEXT)
                    .hint_text(HINT_TEXT)
                    .prefix_text("prefix")
                    .suffix_text("suffix")
                    .helper_text(HELPER_TEXT)
                    .counter_text(COUNTER_TEXT),
            )
            .is_empty(true)
            .is_focused(focused)
            .input_decoration_theme(styles.theme(focused))
            .pump(tester);
            assert_eq!(
                text_style_color(tester, HINT_TEXT),
                color_of(&styles.hint_style)
            );
            assert_eq!(
                text_style_color(tester, "prefix"),
                color_of(&styles.prefix_style)
            );
            assert_eq!(
                text_style_color(tester, "suffix"),
                color_of(&styles.suffix_style)
            );
            assert_eq!(
                text_style_color(tester, HELPER_TEXT),
                color_of(&styles.helper_style)
            );
            assert_eq!(
                text_style_color(tester, COUNTER_TEXT),
                color_of(&styles.counter_style)
            );
            let label_style = if focused {
                &styles.floating_label_style
            } else {
                &styles.label_style
            };
            assert_eq!(color_of(&get_label_style(tester)), color_of(label_style));
        }

        // input_decorator_test.dart 'Material2 - InputDecorationThemeData style overrides'.
        #[test]
        fn material2_theme_style_overrides() {
            test_widgets(|tester| check_style_overrides(tester, false, 20.0, 16.0, 1.0));
        }

        // input_decorator_test.dart 'Material2 - InputDecorationThemeData style overrides
        // (focused)'.
        #[test]
        fn material2_theme_style_overrides_focused() {
            test_widgets(|tester| check_style_overrides(tester, true, 12.0, 12.0, 2.0));
        }

        // input_decorator_test.dart 'Material2 - InputDecorator with empty border and label'.
        #[test]
        fn material2_empty_border_with_a_label() {
            test_widgets(|tester| {
                // Regression test for https://github.com/flutter/flutter/issues/14165
                M2::new(
                    InputDecoration::new()
                        .label_text(LABEL_TEXT)
                        .border(InputBorderRef::none()),
                )
                .pump(tester);

                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(get_border_weight(tester), 0.0);
                assert_eq!(top(tester, LABEL_TEXT), 12.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 24.0);
            });
        }

        fn outline_with(color: Color) -> InputBorderRef {
            InputBorderRef::new(OutlineInputBorder::new().border_side(BorderSide {
                color,
                ..BorderSide::default()
            }))
        }

        fn same_color(actual: Color, expected: Color) {
            assert!(
                crate::test_support::is_same_color_as(actual, expected),
                "{actual:?} is not the same colour as {expected:?}"
            );
        }

        // input_decorator_test.dart 'Material2 - InputDecorator draws and animates hoverColor'.
        #[test]
        fn material2_decorator_draws_and_animates_hover_color() {
            test_widgets(|tester| {
                let fill_color = Color::new(0x0A000000);
                let hover_color = Color::new(0xFF00FF00);
                let disabled_color = Color::new(0x05000000);
                let enabled_border_color = Color::new(0x61000000);

                let pump_decorator =
                    |tester: &mut WidgetTester, hovering: bool, enabled: bool, filled: bool| {
                        M2::new(
                            InputDecoration::new()
                                .enabled(enabled)
                                .filled(filled)
                                .hover_color(hover_color)
                                .disabled_border(outline_with(disabled_color))
                                .border(outline_with(enabled_border_color)),
                        )
                        .is_hovering(hovering)
                        .pump(tester);
                    };

                // Test filled text field.
                pump_decorator(tester, false, true, true);
                same_color(get_container_color(tester), fill_color);
                tester.pump_by(Duration::from_secs(10));
                same_color(get_container_color(tester), fill_color);

                pump_decorator(tester, true, true, true);
                same_color(get_container_color(tester), fill_color);
                tester.pump_by(Duration::from_millis(15));
                same_color(get_container_color(tester), hover_color);

                pump_decorator(tester, false, true, true);
                same_color(get_container_color(tester), hover_color);
                tester.pump_by(Duration::from_millis(15));
                same_color(get_container_color(tester), fill_color);

                // Test that for high refresh rate displays, the color mid-animation is somewhere
                // between the fill color and the hover color.
                pump_decorator(tester, true, true, true);
                same_color(get_container_color(tester), fill_color);
                tester.pump_by(Duration::from_millis(6));
                let hover_value =
                    with_border_painter(tester, |app, painter| painter.hover_animation.value(app));
                let mid_hover_color = Color::lerp(
                    Some(hover_color.with_alpha(0)),
                    Some(hover_color),
                    hover_value,
                )
                .expect("both colours are given");
                same_color(
                    get_container_color(tester),
                    Color::alpha_blend(mid_hover_color, fill_color),
                );

                pump_decorator(tester, false, false, true);
                same_color(get_container_color(tester), disabled_color);
                tester.pump_by(Duration::from_secs(10));
                same_color(get_container_color(tester), disabled_color);

                pump_decorator(tester, true, false, true);
                same_color(get_container_color(tester), disabled_color);
                tester.pump_by(Duration::from_secs(10));
                same_color(get_container_color(tester), disabled_color);

                // Test outline text field.
                let blended_hover_color = Color::new(0x74004400);
                pump_decorator(tester, false, true, false);
                tester.pump_and_settle();
                same_color(get_border_color(tester), enabled_border_color);
                tester.pump_by(Duration::from_secs(10));
                same_color(get_border_color(tester), enabled_border_color);

                pump_decorator(tester, true, true, false);
                same_color(get_border_color(tester), enabled_border_color);
                tester.pump_by(Duration::from_millis(167));
                same_color(get_border_color(tester), blended_hover_color);

                pump_decorator(tester, false, true, false);
                same_color(get_border_color(tester), blended_hover_color);
                tester.pump_by(Duration::from_millis(167));
                same_color(get_border_color(tester), enabled_border_color);

                pump_decorator(tester, false, false, false);
                same_color(get_border_color(tester), enabled_border_color);
                tester.pump_by(Duration::from_millis(167));
                same_color(get_border_color(tester), disabled_color);

                pump_decorator(tester, true, false, false);
                same_color(get_border_color(tester), disabled_color);
                tester.pump_by(Duration::from_secs(10));
                same_color(get_border_color(tester), disabled_color);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator draws and animates focusColor'.
        #[test]
        fn material2_decorator_draws_and_animates_focus_color() {
            test_widgets(|tester| {
                let focus_color = Color::new(0xFF0000FF);
                let disabled_color = Color::new(0x05000000);
                let enabled_border_color = Color::new(0x61000000);

                let pump_decorator = |tester: &mut WidgetTester, focused: bool, enabled: bool| {
                    M2::new(
                        InputDecoration::new()
                            .enabled(enabled)
                            .filled(false)
                            .focus_color(focus_color)
                            .focused_border(outline_with(focus_color))
                            .disabled_border(outline_with(disabled_color))
                            .border(outline_with(enabled_border_color)),
                    )
                    .is_focused(focused)
                    .pump(tester);
                };

                // Test outline text field default border.
                pump_decorator(tester, false, true);
                tester.pump_and_settle();
                assert_eq!(get_border_color(tester), enabled_border_color);
                tester.pump_by(Duration::from_secs(10));
                assert_eq!(get_border_color(tester), enabled_border_color);

                pump_decorator(tester, true, true);
                assert_eq!(get_border_color(tester), enabled_border_color);
                tester.pump_by(Duration::from_millis(167));
                assert_eq!(get_border_color(tester), focus_color);

                pump_decorator(tester, false, true);
                assert_eq!(get_border_color(tester), focus_color);
                tester.pump_by(Duration::from_millis(167));
                assert_eq!(get_border_color(tester), enabled_border_color);

                pump_decorator(tester, false, false);
                assert_eq!(get_border_color(tester), enabled_border_color);
                tester.pump_by(Duration::from_millis(167));
                assert_eq!(get_border_color(tester), disabled_color);

                pump_decorator(tester, true, false);
                assert_eq!(get_border_color(tester), disabled_color);
                tester.pump_by(Duration::from_secs(10));
                assert_eq!(get_border_color(tester), disabled_color);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator withdraws label when not empty or
        // focused'.
        #[test]
        fn material2_decorator_withdraws_the_label_when_not_empty_or_focused() {
            test_widgets(|tester| {
                let pump_decorator =
                    |tester: &mut WidgetTester, focused: bool, enabled: bool, empty: bool| {
                        M2::new(
                            InputDecoration::new()
                                .label_text("Label")
                                .enabled(enabled)
                                .filled(false)
                                .focused_border(OutlineInputBorder::new())
                                .disabled_border(OutlineInputBorder::new())
                                .border(OutlineInputBorder::new()),
                        )
                        .is_empty(empty)
                        .is_focused(focused)
                        .pump(tester);
                        tester.pump_and_settle();
                    };
                let label_size = Size::new(80.0, 16.0);
                let floating_size = Size::new(80.0 * 0.75, 16.0 * 0.75);
                let check = |tester: &WidgetTester, top_left: Offset, size: Size| {
                    let rect = get_label_rect(tester);
                    assert_eq!(rect.top_left(), top_left);
                    assert_eq!(rect.size(), size);
                };

                pump_decorator(tester, false, true, true);
                check(tester, Offset::new(12.0, 20.0), label_size);

                pump_decorator(tester, false, true, false);
                check(tester, Offset::new(12.0, -5.5), floating_size);

                pump_decorator(tester, true, true, true);
                check(tester, Offset::new(12.0, -5.5), floating_size);

                pump_decorator(tester, true, true, false);
                check(tester, Offset::new(12.0, -5.5), floating_size);

                pump_decorator(tester, false, false, true);
                check(tester, Offset::new(12.0, 20.0), label_size);

                pump_decorator(tester, false, false, false);
                check(tester, Offset::new(12.0, -5.5), floating_size);

                // Focused and disabled happens with NavigationMode.directional.
                pump_decorator(tester, true, false, true);
                check(tester, Offset::new(12.0, 20.0), label_size);

                pump_decorator(tester, true, false, false);
                check(tester, Offset::new(12.0, -5.5), floating_size);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecoration default border uses colorScheme'.
        #[test]
        #[allow(deprecated)]
        fn material2_default_border_uses_the_color_scheme() {
            test_widgets(|tester| {
                let theme = m2(&tester.cell.borrow());
                // Dart's `withOpacity`, deprecated there too.
                let enabled_color = theme.color_scheme.on_surface.with_opacity(0.38);
                let disabled_color = theme.disabled_color;
                let hover_color =
                    Color::alpha_blend(theme.hover_color.with_opacity(0.12), enabled_color);

                // Enabled
                M2::new(InputDecoration::new()).theme(m2).pump(tester);
                tester.pump_and_settle();
                assert_eq!(get_border_color(tester), enabled_color);

                // Filled
                M2::new(InputDecoration::new().filled(true))
                    .theme(m2)
                    .pump(tester);
                tester.pump_and_settle();
                assert_eq!(get_border_color(tester), theme.hint_color);

                // Hovering
                M2::new(InputDecoration::new())
                    .theme(m2)
                    .is_hovering(true)
                    .pump(tester);
                tester.pump_and_settle();
                assert_eq!(get_border_color(tester), hover_color);

                // Focused
                M2::new(InputDecoration::new())
                    .theme(m2)
                    .is_focused(true)
                    .pump(tester);
                tester.pump_and_settle();
                assert_eq!(get_border_color(tester), theme.color_scheme.primary);

                // Error
                M2::new(InputDecoration::new().error_text("Nope"))
                    .theme(m2)
                    .pump(tester);
                tester.pump_and_settle();
                assert_eq!(get_border_color(tester), theme.color_scheme.error);

                // Disabled
                M2::new(InputDecoration::new().enabled(false))
                    .theme(m2)
                    .pump(tester);
                tester.pump_and_settle();
                assert_eq!(get_border_color(tester), disabled_color);

                // Disabled, filled
                M2::new(InputDecoration::new().enabled(false).filled(true))
                    .theme(m2)
                    .pump(tester);
                tester.pump_and_settle();
                assert_eq!(get_border_color(tester), Colors::TRANSPARENT);
            });
        }

        fn outline(color: Color, width: f64) -> InputBorderRef {
            InputBorderRef::new(OutlineInputBorder::new().border_side(BorderSide {
                color,
                width,
                ..BorderSide::default()
            }))
        }

        // input_decorator_test.dart 'Material2 - InputDecoration borders'.
        #[test]
        fn material2_decoration_borders_per_state() {
            test_widgets(|tester| {
                let error_border = outline(Colors::RED.primary(), 1.5);
                let focused_border = outline(Colors::GREEN.primary(), 4.0);
                let focused_error_border = outline(Colors::TEAL.primary(), 5.0);
                let disabled_border = outline(Colors::GREY.primary(), 0.0);
                let enabled_border = outline(Colors::BLUE.primary(), 2.5);
                let decoration = || {
                    InputDecoration::new()
                        .error_border(error_border.clone())
                        .focused_border(focused_border.clone())
                        .focused_error_border(focused_error_border.clone())
                        .disabled_border(disabled_border.clone())
                        .enabled_border(enabled_border.clone())
                };
                let check = |tester: &mut WidgetTester,
                             decoration: InputDecoration,
                             focused: bool,
                             expected: &dyn InputBorder| {
                    M2::new(decoration).is_focused(focused).pump(tester);
                    tester.pump_and_settle(); // border changes are animated
                    assert!(
                        *get_border(tester).unwrap() == *expected,
                        "{:?}",
                        get_border(tester).unwrap().border_side()
                    );
                };

                check(tester, decoration(), false, &*enabled_border);
                check(tester, decoration(), true, &*focused_border);
                check(
                    tester,
                    decoration().error_text(ERROR_TEXT),
                    true,
                    &*focused_error_border,
                );
                check(
                    tester,
                    decoration().error_text(ERROR_TEXT),
                    false,
                    &*error_border,
                );
                check(
                    tester,
                    decoration().error_text(ERROR_TEXT).enabled(false),
                    false,
                    &*error_border,
                );
                check(
                    tester,
                    decoration().enabled(false),
                    false,
                    &*disabled_border,
                );
                check(tester, decoration().enabled(false), true, &*disabled_border);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator floating label Y coordinate'.
        #[test]
        fn material2_floating_label_y_coordinate() {
            test_widgets(|tester| {
                // Regression test for https://github.com/flutter/flutter/issues/54028
                M2::new(
                    InputDecoration::new()
                        .label_text(LABEL_TEXT)
                        .enabled_border(OutlineInputBorder::new().border_side(BorderSide {
                            width: 4.0,
                            ..BorderSide::default()
                        }))
                        .floating_label_behavior(FloatingLabelBehavior::Always),
                )
                .is_empty(true)
                .pump(tester);
                tester.pump_and_settle();

                // floatingLabelHeight = 12 (font size 16dps * 0.75 = 12)
                // labelY = -floatingLabelHeight/2 + borderWidth/2
                assert_eq!(top(tester, LABEL_TEXT), -4.0);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator floating label obeys
        // floatingLabelBehavior'.
        #[test]
        fn material2_floating_label_obeys_floating_label_behavior() {
            test_widgets(|tester| {
                M2::new(
                    InputDecoration::new()
                        .label_text(LABEL_TEXT)
                        .floating_label_behavior(FloatingLabelBehavior::Never),
                )
                .pump(tester);

                // Passing floating behavior never results in a dy offset of 20
                // because the label is not initially floating.
                assert_eq!(bottom(tester, LABEL_TEXT), 36.0);
                assert_eq!(top(tester, LABEL_TEXT), 20.0);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator hint is displayed when
        // floatingLabelBehavior is always'.
        #[test]
        fn material2_hint_is_displayed_when_the_label_always_floats() {
            test_widgets(|tester| {
                M2::new(
                    InputDecoration::new()
                        .floating_label_behavior(FloatingLabelBehavior::Always)
                        .hint_text(HINT_TEXT)
                        .label_text(LABEL_TEXT),
                )
                .is_empty(true)
                .pump(tester);
                tester.pump_and_settle();

                assert_eq!(get_opacity(tester, HINT_TEXT), 1.0);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorator floating label width scales when
        // focused'. Dart reads the clip of the label's ellipsized paragraph; the label's
        // width is measured instead: it is the label's constraint, which the clip is in Dart.
        #[test]
        fn material2_floating_label_width_scales_when_focused() {
            test_widgets(|tester| {
                let long_string_a: String = "A".repeat(200);
                let long_string_b: String = "B".repeat(200);
                let pump = |tester: &mut WidgetTester, text: &str, is_focused: bool| {
                    let decorator = M2::new(InputDecoration::new().label_text(text))
                        .is_empty(true)
                        .is_focused(is_focused)
                        .build(tester);
                    tester.pump_widget(
                        waft_widgets::Center::new()
                            .child(SizedBox::new().width(100.0).height(100.0).child(decorator))
                            .into_widget(),
                    );
                    tester.pump_and_settle();
                };

                pump(tester, &long_string_a, false);
                let label = tester.find_text(&long_string_a)[0];
                let size = {
                    let render = tester.render_box_of(label);
                    render.size(&tester.cell.borrow())
                };
                assert_eq!(size, Size::new(100.0, 16.0));

                pump(tester, &long_string_b, true);
                let label = tester.find_text(&long_string_b)[0];
                let size = {
                    let render = tester.render_box_of(label);
                    render.size(&tester.cell.borrow())
                };
                // _kFinalLabelScale = 0.75
                assert_close(size.width(), 100.0 / 0.75, 1e-5);
                assert_eq!(size.height(), 16.0);
            });
        }

        // input_decorator_test.dart 'Material2 - textAlignVertical can be updated'. The hint keeps
        // Dart's string: the case compares its positions only.
        #[test]
        fn material2_text_align_vertical_can_be_updated() {
            test_widgets(|tester| {
                // Regression test for https://github.com/flutter/flutter/issues/56933
                let alignment: Rc<std::cell::Cell<Option<TextAlignVertical>>> =
                    Rc::new(std::cell::Cell::new(Some(TextAlignVertical::TOP)));
                let setter: Rc<std::cell::RefCell<Option<waft_widgets::StateSetter>>> =
                    Rc::new(std::cell::RefCell::new(None));
                let home = {
                    let alignment = Rc::clone(&alignment);
                    let setter = Rc::clone(&setter);
                    waft_widgets::StatefulBuilder::new(move |_app, _context, set_state| {
                        *setter.borrow_mut() = Some(set_state);
                        let mut decorator =
                            InputDecorator::new(InputDecoration::new().hint_text(HINT_TEXT));
                        if let Some(alignment) = alignment.get() {
                            decorator = decorator.text_align_vertical(alignment);
                        }
                        decorator.into_widget()
                    })
                    .into_widget()
                };
                let theme = m2(&tester.cell.borrow());
                tester.pump_widget(material_app(theme, home));
                let hint_top =
                    |tester: &WidgetTester| rect_of(tester, tester.find_text(HINT_TEXT)[0]).top;
                let set = |tester: &mut WidgetTester, value: Option<TextAlignVertical>| {
                    let set_state = setter.borrow().clone().expect("built");
                    let alignment = Rc::clone(&alignment);
                    set_state(&mut tester.cell.borrow_mut(), &mut move |_app| {
                        alignment.set(value)
                    });
                    tester.pump();
                };

                let top_position = hint_top(tester);

                set(tester, Some(TextAlignVertical::BOTTOM));
                assert!(hint_top(tester) > top_position);

                // Setting textAlignVertical back to null works and reverts to the default.
                set(tester, None);
                assert_eq!(hint_top(tester), top_position);
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorationThemeData.floatingLabelStyle
        // overrides label widget styles when the widget is a text widget (focused)'.
        #[test]
        fn material2_theme_floating_label_style_styles_a_label_widget() {
            test_widgets(|tester| {
                let floating_label_style = TextStyle::new()
                    .font_size(16.0)
                    .merge(Some(&TextStyle::new().color(Colors::INDIGO.primary())));
                M2::new(InputDecoration::new().label_text(LABEL_TEXT))
                    .is_empty(true)
                    .is_focused(true) // Label appears floating above input field.
                    .input_decoration_theme(
                        InputDecorationThemeData::new()
                            .floating_label_style(floating_label_style.clone()),
                    )
                    .pump(tester);

                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(top(tester, LABEL_TEXT), 12.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 24.0);
                assert_eq!(get_border_bottom(tester), 56.0);
                assert_eq!(get_border_weight(tester), 2.0);

                // Verify that the styles were passed along
                assert_eq!(
                    color_of(&get_label_style(tester)),
                    color_of(&floating_label_style)
                );
            });
        }

        // input_decorator_test.dart 'Material2 - InputDecorationThemeData.labelStyle overrides label
        // widget styles when the widget is a text widget'.
        #[test]
        fn material2_theme_label_style_styles_a_label_widget() {
            test_widgets(|tester| {
                let label_style = TextStyle::new()
                    .font_size(16.0)
                    .merge(Some(&TextStyle::new().color(Colors::PURPLE.primary())));
                M2::new(InputDecoration::new().label_text(LABEL_TEXT))
                    .is_empty(true) // Label appears inline, on top of the input field.
                    .input_decoration_theme(
                        InputDecorationThemeData::new().label_style(label_style.clone()),
                    )
                    .pump(tester);

                assert_eq!(decorator_size(tester), Size::new(800.0, 56.0));
                assert_eq!(top(tester, LABEL_TEXT), 20.0);
                assert_eq!(bottom(tester, LABEL_TEXT), 36.0);
                assert_eq!(get_border_bottom(tester), 56.0);
                assert_eq!(get_border_weight(tester), 1.0);

                // Verify that the styles were passed along
                assert_eq!(color_of(&get_label_style(tester)), color_of(&label_style));
            });
        }

        // input_decorator_test.dart 'Material2 - prefixIcon in RTL with asymmetric padding'.
        #[test]
        fn material2_prefix_icon_in_rtl_with_asymmetric_padding_stays_inside() {
            test_widgets(|tester| {
                // Regression test for https://github.com/flutter/flutter/issues/129591
                M2::new(
                    InputDecoration::new()
                        .content_padding(EdgeInsetsGeometry::from(EdgeInsetsDirectional::only(
                            0.0, 0.0, 24.0, 0.0,
                        )))
                        .prefix_icon(waft_widgets::Focus::new(Icon::new(Some(Icons::search())))),
                )
                .text_direction(TextDirection::Rtl)
                .pump(tester);
                tester.pump_and_settle();

                assert_eq!(tester.find_by_type::<InputDecorator>().len(), 1);
                assert_eq!(tester.find_by_type::<Icon>().len(), 1);

                let decorator_right = get_decorator_rect(tester).right;
                let prefix_right = rect_of(tester, tester.find_by_type::<Icon>()[0]).right;

                // The prefix is inside the decorator.
                assert!(decorator_right <= prefix_right);
            });
        }

        // input_decorator_test.dart 'Material2 - intrinsic width with prefixIcon/suffixIcon'.
        #[test]
        fn material2_intrinsic_width_with_prefix_and_suffix_icons_keeps_the_text_width() {
            test_widgets(|tester| {
                // Regression test for https://github.com/flutter/flutter/issues/137937
                for direction in [TextDirection::Rtl, TextDirection::Ltr] {
                    let mut measure_text = |decoration: InputDecoration| {
                        M2::new(decoration)
                            .use_intrinsic_width(true)
                            .text_direction(direction)
                            .pump(tester);
                        tester.pump_and_settle();
                        assert_eq!(tester.find_text(INPUT_TEXT).len(), 1);
                        text_rect(tester, INPUT_TEXT).size()
                    };
                    let padding = || EdgeInsetsDirectional::only(12.0, 0.0, 24.0, 0.0);
                    let icon =
                        || waft_widgets::Focus::new(Icon::new(Some(Icons::search()))).into_widget();

                    let without_icons = measure_text(
                        InputDecoration::new().content_padding(EdgeInsetsGeometry::from(padding())),
                    );
                    let with_prefix_icon = measure_text(
                        InputDecoration::new()
                            .content_padding(EdgeInsetsGeometry::from(padding()))
                            .prefix_icon(icon()),
                    );
                    let with_suffix_icon = measure_text(
                        InputDecoration::new()
                            .content_padding(EdgeInsetsGeometry::from(padding()))
                            .suffix_icon(icon()),
                    );

                    assert_eq!(
                        with_prefix_icon.width(),
                        without_icons.width(),
                        "text width is different with prefixIcon and {direction:?}"
                    );
                    assert_eq!(
                        with_suffix_icon.width(),
                        without_icons.width(),
                        "text width is different with prefixIcon and {direction:?}"
                    );
                }
            });
        }

        // input_decorator_test.dart 'UnderlineInputBorder with BorderStyle.none should not show
        // anything'.
        #[test]
        fn an_underline_border_with_no_style_paints_no_ring() {
            test_widgets(|tester| {
                // Regression test for https://github.com/flutter/flutter/issues/143746
                tester.pump_widget(
                    BuildInputDecorator::new(
                        InputDecoration::new().enabled_border(
                            UnderlineInputBorder::new()
                                .border_side(BorderSide {
                                    style: waft_painting::BorderStyle::None,
                                    ..BorderSide::default()
                                })
                                .border_radius(BorderRadius::all(waft_embedder::Radius::circular(
                                    4.0,
                                ))),
                        ),
                    )
                    .build(),
                );
                let render = tester
                    .render_box_of(tester.find_by_type::<InputDecorator>()[0])
                    .as_object();
                // Dart's `paints..drrect()`: a ring is an even-odd path here.
                let rings = tester
                    .painted_ops(render)
                    .into_iter()
                    .filter(|op| {
                        matches!(
                            op,
                            waft_embedder::valo::Op::DrawPath {
                                fill_rule: waft_embedder::FillRule::EvenOdd,
                                ..
                            }
                        )
                    })
                    .count();
                assert_eq!(rings, 0);
            });
        }

        // input_decorator_test.dart 'InputDecorator _buildError with errorText correctly updates on
        // BuildContext updates'. The decoration changes through a new pump where Dart's
        // `ValueListenableBuilder` rebuilds; both update the decorator with a new decoration.
        #[test]
        fn the_error_text_updates_when_the_decoration_changes() {
            test_widgets(|tester| {
                const HELPER_TEXT_VALUE: &str = "helper text";
                let pump = |tester: &mut WidgetTester, value: Option<&str>| {
                    let mut decoration = InputDecoration::new().helper_text(HELPER_TEXT_VALUE);
                    if let Some(value) = value {
                        decoration = decoration.error_text(value);
                    }
                    tester.pump_widget(BuildInputDecorator::new(decoration).build());
                    tester.pump_and_settle();
                };
                pump(tester, Some("initial error"));

                assert_eq!(tester.find_onstage_text("initial error").len(), 1);
                assert!(tester.find_onstage_text(HELPER_TEXT_VALUE).is_empty());

                pump(tester, None);

                assert!(tester.find_onstage_text("initial error").is_empty());
                assert_eq!(tester.find_onstage_text(HELPER_TEXT_VALUE).len(), 1);
            });
        }

        /// Dart's `MaterialApp(theme:, home: Scaffold(body: Directionality(textDirection:,
        /// child: SizedBox(width:, child: InputDecorator(decoration:)))))` of the supporting text
        /// padding cases.
        fn pump_in_scaffold(
            tester: &mut WidgetTester,
            theme: fn(&App) -> ThemeData,
            direction: TextDirection,
            width: f64,
            decoration: InputDecoration,
        ) {
            let theme = theme(&tester.cell.borrow());
            let home = crate::scaffold::Scaffold::new().body(Directionality::new(
                direction,
                SizedBox::new()
                    .width(width)
                    .child(InputDecorator::new(decoration)),
            ));
            tester.pump_widget(material_app(theme, home.into_widget()));
            tester.pump_and_settle();
        }

        fn string_rect(tester: &WidgetTester, text: &str) -> Rect {
            if !tester.find_text(text).is_empty() {
                return text_rect(tester, text);
            }
            let found = tester.find_onstage_text(text);
            assert_eq!(found.len(), 1, "one {text:?}");
            rect_of(tester, found[0])
        }

        // input_decorator_test.dart 'helper/error text default padding is correct'. The case keeps
        // Dart's strings: it reads the edges of a text that wraps to the available width.
        #[test]
        fn helper_and_error_text_default_padding_is_correct() {
            test_widgets(|tester| {
                const DEFAULT_PADDING: f64 = 16.0; // From M3 spec.
                const LONG_TEXT: &str =
                    "This is a very long text that should wrap and fill the available width";
                const INPUT_WIDTH: f64 = 300.0;

                for direction in [TextDirection::Ltr, TextDirection::Rtl] {
                    for error in [false, true] {
                        let mut decoration = InputDecoration::new().filled(true);
                        decoration = if error {
                            decoration.error_text(LONG_TEXT)
                        } else {
                            decoration.helper_text(LONG_TEXT)
                        };
                        pump_in_scaffold(tester, m3, direction, INPUT_WIDTH, decoration);
                        let rect = string_rect(tester, LONG_TEXT);
                        assert_eq!(rect.left, DEFAULT_PADDING, "{direction:?} error: {error}");
                        assert_eq!(
                            rect.right,
                            INPUT_WIDTH - DEFAULT_PADDING,
                            "{direction:?} error: {error}"
                        );
                    }
                }
            });
        }

        // input_decorator_test.dart 'InputDecorator does not crash at zero area'.
        #[test]
        fn the_decorator_does_not_crash_at_zero_area() {
            test_widgets(|tester| {
                for decoration in [
                    InputDecoration::new(),
                    InputDecoration::new().border(OutlineInputBorder::new()),
                ] {
                    let theme = m3(&tester.cell.borrow());
                    let home = crate::scaffold::Scaffold::new().body(
                        waft_widgets::Center::new()
                            .child(SizedBox::shrink().child(InputDecorator::new(decoration))),
                    );
                    tester.pump_widget(material_app(theme, home.into_widget()));
                    assert_eq!(decorator_size(tester), Size::ZERO);
                }
            });
        }

        fn supporting_decoration(
            error_text: Option<&str>,
            helper_text: Option<&str>,
            supporting_text_padding: Option<EdgeInsetsGeometry>,
        ) -> InputDecoration {
            let mut decoration = InputDecoration::new()
                .filled(true)
                .counter_text(COUNTER_TEXT);
            if let Some(error_text) = error_text {
                decoration = decoration.error_text(error_text);
            }
            if let Some(helper_text) = helper_text {
                decoration = decoration.helper_text(HELPER_TEXT);
                assert_eq!(helper_text, HELPER_TEXT);
            }
            if let Some(padding) = supporting_text_padding {
                decoration = decoration.supporting_text_padding(padding);
            }
            decoration
        }

        /// An edge read through the page's transforms, which Waft's f32 `Matrix4` rounds where
        /// Dart's doubles do not.
        fn edge_eq(actual: f64, expected: f64) {
            assert_close(actual, expected, 1e-4);
        }

        const INPUT_GAP: f64 = 4.0; // _kInputExtraPadding in Material 3 filled field.

        // input_decorator_test.dart 'supportingTextPadding defined in InputDecoration is used for
        // supporting text'. An edge at a text's width is a fraction of FlutterTest's letter
        // spacing, which the f32 transforms of a position round.
        #[test]
        fn supporting_text_padding_of_the_decoration_places_the_supporting_text() {
            test_widgets(|tester| {
                const START: f64 = 32.0;
                const END: f64 = 24.0;
                const TOP: f64 = 16.0;
                const BOTTOM: f64 = 12.0;
                const INPUT_WIDTH: f64 = 300.0;
                let padding = Some(EdgeInsetsGeometry::directional(START, 0.0, END, 0.0));

                // Calculate default vertical bounds first to ensure new vertical padding shifts
                // correctly.
                pump_in_scaffold(
                    tester,
                    m3,
                    TextDirection::Ltr,
                    INPUT_WIDTH,
                    supporting_decoration(Some(ERROR_TEXT), None, None),
                );
                let default_error_dy = string_rect(tester, ERROR_TEXT).top;
                let default_total_height = decorator_size(tester).height();

                // LTR with custom supportingTextPadding (error).
                pump_in_scaffold(
                    tester,
                    m3,
                    TextDirection::Ltr,
                    INPUT_WIDTH,
                    supporting_decoration(
                        Some(ERROR_TEXT),
                        None,
                        Some(EdgeInsetsGeometry::directional(START, TOP, END, BOTTOM)),
                    ),
                );
                edge_eq(string_rect(tester, ERROR_TEXT).left, START + INPUT_GAP);
                edge_eq(
                    string_rect(tester, COUNTER_TEXT).right,
                    INPUT_WIDTH - END - INPUT_GAP,
                );

                // Verify vertical padding shift.
                assert!(string_rect(tester, ERROR_TEXT).top > default_error_dy);
                assert!(decorator_size(tester).height() > default_total_height);

                // RTL with custom supportingTextPadding (error).
                pump_in_scaffold(
                    tester,
                    m3,
                    TextDirection::Rtl,
                    INPUT_WIDTH,
                    supporting_decoration(Some(ERROR_TEXT), None, padding),
                );
                // In RTL, "start" is from the right, so errorText is placed on the right.
                edge_eq(
                    string_rect(tester, ERROR_TEXT).right,
                    INPUT_WIDTH - START - INPUT_GAP,
                );
                // In RTL, "end" is from the left, so counterText is placed on the left.
                edge_eq(string_rect(tester, COUNTER_TEXT).left, END + INPUT_GAP);

                // LTR with custom supportingTextPadding (helper).
                pump_in_scaffold(
                    tester,
                    m3,
                    TextDirection::Ltr,
                    INPUT_WIDTH,
                    supporting_decoration(None, Some(HELPER_TEXT), padding),
                );
                edge_eq(string_rect(tester, HELPER_TEXT).left, START + INPUT_GAP);
                edge_eq(
                    string_rect(tester, COUNTER_TEXT).right,
                    INPUT_WIDTH - END - INPUT_GAP,
                );

                // RTL with custom supportingTextPadding (helper).
                pump_in_scaffold(
                    tester,
                    m3,
                    TextDirection::Rtl,
                    INPUT_WIDTH,
                    supporting_decoration(None, Some(HELPER_TEXT), padding),
                );
                edge_eq(
                    string_rect(tester, HELPER_TEXT).right,
                    INPUT_WIDTH - START - INPUT_GAP,
                );
                edge_eq(string_rect(tester, COUNTER_TEXT).left, END + INPUT_GAP);
            });
        }

        fn theme_with_supporting_text_padding(app: &App) -> ThemeData {
            ThemeData::builder()
                .use_material3(true)
                .input_decoration_theme(
                    InputDecorationThemeData::new().supporting_text_padding(
                        EdgeInsetsGeometry::directional(40.0, 0.0, 20.0, 0.0),
                    ),
                )
                .build(app)
        }

        // input_decorator_test.dart 'supportingTextPadding defined in InputDecorationTheme is used
        // for supporting text'.
        #[test]
        fn supporting_text_padding_of_the_theme_places_the_supporting_text() {
            test_widgets(|tester| {
                const THEME_START: f64 = 40.0;
                const THEME_END: f64 = 20.0;
                const INPUT_WIDTH: f64 = 300.0;
                for (error, helper) in [(Some(ERROR_TEXT), None), (None, Some(HELPER_TEXT))] {
                    let text = error.or(helper).expect("one of them");
                    pump_in_scaffold(
                        tester,
                        theme_with_supporting_text_padding,
                        TextDirection::Ltr,
                        INPUT_WIDTH,
                        supporting_decoration(error, helper, None),
                    );
                    edge_eq(string_rect(tester, text).left, THEME_START + INPUT_GAP);
                    edge_eq(
                        string_rect(tester, COUNTER_TEXT).right,
                        INPUT_WIDTH - THEME_END - INPUT_GAP,
                    );

                    // In RTL, "start" is from the right, "end" is from the left.
                    pump_in_scaffold(
                        tester,
                        theme_with_supporting_text_padding,
                        TextDirection::Rtl,
                        INPUT_WIDTH,
                        supporting_decoration(error, helper, None),
                    );
                    edge_eq(
                        string_rect(tester, text).right,
                        INPUT_WIDTH - THEME_START - INPUT_GAP,
                    );
                    edge_eq(
                        string_rect(tester, COUNTER_TEXT).left,
                        THEME_END + INPUT_GAP,
                    );
                }
            });
        }

        // input_decorator_test.dart 'supportingTextPadding with an icon positions supporting text
        // correctly in LTR and RTL'.
        #[test]
        fn supporting_text_padding_with_an_icon_shifts_only_the_start() {
            test_widgets(|tester| {
                const INPUT_WIDTH: f64 = 300.0;
                let pump =
                    |tester: &mut WidgetTester, direction: TextDirection, with_icon: bool| {
                        let mut decoration = supporting_decoration(
                            Some(ERROR_TEXT),
                            None,
                            Some(EdgeInsetsGeometry::directional(32.0, 0.0, 24.0, 0.0)),
                        );
                        if with_icon {
                            decoration = decoration.icon(SizedBox::new().width(24.0).height(24.0));
                        }
                        pump_in_scaffold(tester, m3, direction, INPUT_WIDTH, decoration);
                    };

                // Measure baseline start/end offsets without icon in LTR.
                pump(tester, TextDirection::Ltr, false);
                let ltr_start_without_icon = string_rect(tester, ERROR_TEXT).left;
                let ltr_end_without_icon = string_rect(tester, COUNTER_TEXT).right;

                // With icon in LTR: start offset increases by _boxSize(icon).width; end is
                // unaffected.
                pump(tester, TextDirection::Ltr, true);
                let ltr_start_with_icon = string_rect(tester, ERROR_TEXT).left;
                let ltr_end_with_icon = string_rect(tester, COUNTER_TEXT).right;
                // The delta is the actual _boxSize(icon).width added by the layout.
                let icon_box_width = ltr_start_with_icon - ltr_start_without_icon;
                assert!(icon_box_width > 0.0); // Icon shifts start offset right in LTR.
                assert_eq!(ltr_end_with_icon, ltr_end_without_icon); // End (counter) is unaffected by icon.

                // Measure baseline start/end offsets without icon in RTL.
                pump(tester, TextDirection::Rtl, false);
                let rtl_start_without_icon = string_rect(tester, ERROR_TEXT).right;
                let rtl_end_without_icon = string_rect(tester, COUNTER_TEXT).left;

                // With icon in RTL: start offset decreases by _boxSize(icon).width; end is
                // unaffected.
                pump(tester, TextDirection::Rtl, true);
                let rtl_start_with_icon = string_rect(tester, ERROR_TEXT).right;
                let rtl_end_with_icon = string_rect(tester, COUNTER_TEXT).left;
                // In RTL the icon is on the right, so it reduces the start offset from the right.
                assert_eq!(rtl_start_without_icon - rtl_start_with_icon, icon_box_width);
                assert_eq!(rtl_end_with_icon, rtl_end_without_icon); // End (counter) is unaffected by icon.
            });
        }

        // input_decorator_test.dart 'supportingTextPadding in Material 2 (the 8.0 gap)'.
        #[test]
        fn supporting_text_padding_in_material2_replaces_the_8_gap() {
            test_widgets(|tester| {
                let pump = |tester: &mut WidgetTester,
                            padding: Option<EdgeInsetsGeometry>,
                            error_text: Option<&str>,
                            helper_text: Option<&str>| {
                    let mut decoration = InputDecoration::new().filled(true);
                    if let Some(error_text) = error_text {
                        decoration = decoration.error_text(error_text);
                    }
                    if let Some(helper_text) = helper_text {
                        decoration = decoration.helper_text(helper_text);
                    }
                    if let Some(padding) = padding {
                        decoration = decoration.supporting_text_padding(padding);
                    }
                    let theme = m2(&tester.cell.borrow());
                    let home = crate::scaffold::Scaffold::new().body(
                        waft_widgets::Align::new()
                            .alignment(waft_painting::AlignmentGeometry::Alignment(
                                waft_painting::Alignment::TOP_LEFT,
                            ))
                            .child(
                                SizedBox::new()
                                    .width(300.0)
                                    .child(InputDecorator::new(decoration)),
                            ),
                    );
                    tester.pump_widget(material_app(theme, home.into_widget()));
                    tester.pump_and_settle();
                };
                let gap = |tester: &WidgetTester, text: &str| {
                    string_rect(tester, text).top - get_border_bottom(tester)
                };

                // Default Material 2 behavior: gap between container bottom and helper text is 8.0.
                pump(tester, None, None, Some(HELPER_TEXT));
                assert_eq!(gap(tester, HELPER_TEXT), 8.0);

                // Default Material 2 behavior: gap between container bottom and error text is 8.0.
                pump(tester, None, Some(ERROR_TEXT), None);
                assert_eq!(gap(tester, ERROR_TEXT), 8.0);

                // Custom supportingTextPadding with explicit top overrides the 8.0 gap.
                const CUSTOM_TOP_PADDING: f64 = 16.0;
                pump(
                    tester,
                    Some(EdgeInsetsGeometry::directional(
                        0.0,
                        CUSTOM_TOP_PADDING,
                        0.0,
                        0.0,
                    )),
                    None,
                    Some(HELPER_TEXT),
                );
                assert_eq!(gap(tester, HELPER_TEXT), CUSTOM_TOP_PADDING);

                // Custom supportingTextPadding with only horizontal padding removes the 8.0 gap
                // (top defaults to 0.0).
                pump(
                    tester,
                    Some(EdgeInsetsGeometry::directional(24.0, 0.0, 0.0, 0.0)),
                    None,
                    Some(HELPER_TEXT),
                );
                assert_eq!(gap(tester, HELPER_TEXT), 0.0);
            });
        }

        // input_decorator_test.dart 'InputDecorator respects individual text directions for label and
        // hint'.
        #[test]
        fn the_label_and_hint_take_their_own_text_directions() {
            test_widgets(|tester| {
                const HINT: &str = "Hint Text";
                const LABEL: &str = "Label Text";
                let theme = m3(&tester.cell.borrow());
                let home = crate::scaffold::Scaffold::new().body(Directionality::new(
                    TextDirection::Ltr,
                    waft_widgets::Center::new().child(
                        InputDecorator::new(
                            InputDecoration::new()
                                .hint_text(HINT)
                                .label_text(LABEL)
                                .label_text_direction(TextDirection::Rtl)
                                .hint_text_direction(TextDirection::Rtl),
                        )
                        .is_empty(true)
                        .child(SizedBox::shrink()),
                    ),
                ));
                tester.pump_widget(material_app(theme, home.into_widget()));

                let direction_of = |text: &str| {
                    let element = tester.find_text(text)[0];
                    tester.widget(element, |text: &waft_widgets::Text| text.text_direction)
                };
                assert_eq!(direction_of(HINT), Some(TextDirection::Rtl));
                assert_eq!(direction_of(LABEL), Some(TextDirection::Rtl));
            });
        }

        /// Dart's 'Floating label hit test matches its painted position (textDirection)'.
        fn check_floating_label_hit_test(tester: &mut WidgetTester, text_direction: TextDirection) {
            let tap_count = Rc::new(std::cell::Cell::new(0));
            let label_key = GlobalKey::new();
            let on_tap = {
                let tap_count = Rc::clone(&tap_count);
                waft_foundation::Listener::new(move |_app| tap_count.set(tap_count.get() + 1))
            };
            tester.pump_widget(
                BuildInputDecorator::new(
                    InputDecoration::new()
                        .border(OutlineInputBorder::new())
                        .label(
                            waft_widgets::GestureDetector::new()
                                .key(label_key.clone().into())
                                .behavior(waft_rendering::HitTestBehavior::Opaque)
                                .on_tap(on_tap)
                                .child(SizedBox::new().width(100.0).height(20.0)),
                        ),
                )
                .text_direction(text_direction)
                .build(),
            );

            // The label floats (isEmpty is false), so it is painted scaled and moved up
            // onto the border, above its layout position.
            let label_rect = key_rect(tester, &label_key);
            assert_eq!(label_rect.width(), 75.0);

            // Taps at the painted label hit the label.
            tester.tap(label_rect.center(), 1);
            tester.tap(label_rect.bottom_left() + Offset::new(1.0, -1.0), 2);
            assert_eq!(tap_count.get(), 2);

            // A tap below the painted label, inside its layout position, misses the label.
            tester.tap(label_rect.bottom_center() + Offset::new(0.0, 10.0), 3);
            assert_eq!(tap_count.get(), 2);
        }

        // input_decorator_test.dart 'Floating label hit test matches its painted position
        // (TextDirection.rtl)'.
        #[test]
        fn the_floating_label_is_hit_where_it_is_painted_rtl() {
            test_widgets(|tester| check_floating_label_hit_test(tester, TextDirection::Rtl));
        }

        // input_decorator_test.dart 'Floating label hit test matches its painted position
        // (TextDirection.ltr)'.
        #[test]
        fn the_floating_label_is_hit_where_it_is_painted_ltr() {
            test_widgets(|tester| check_floating_label_hit_test(tester, TextDirection::Ltr));
        }
    }
}
