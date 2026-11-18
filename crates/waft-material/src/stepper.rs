//! Flutter counterpart: material_ui's `stepper.dart`.

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use waft_embedder::{
    Brightness, Canvas, Clip, Color, FillRule, Offset, Paint, PathBuilder, Size, TextDirection,
    valo,
};
use waft_foundation::{App, Handle, Listener, ValueChanged};
use waft_painting::{
    Alignment, BorderRadiusGeometry, BorderSide, BoxBorderRef, BoxDecoration, BoxShadow, BoxShape,
    EdgeInsets, EdgeInsetsDirectional, EdgeInsetsGeometry, GradientRef, OutlinedBorderRef,
    RoundedRectangleBorder, TextStyle,
};
use waft_rendering::CustomPainter;
use waft_widgets::{
    Align, AnimatedContainer, AnimatedCrossFade, AnimatedDefaultTextStyle, AnimatedSize,
    BuildContext, Center, ClipRect, ColoredBox, Column, CrossAxisAlignment, CustomPaint, Expanded,
    GlobalKey, Icon, Interval, IntoWidget, KeyRef, ListView, MainAxisAlignment, MainAxisSize,
    Padding, PositionedDirectional, Row, ScrollControllerRef, ScrollPositionAlignmentPolicy,
    Scrollable, SizedBox, Stack, State, StateData, StatefulWidget, Text, Visibility, WidgetRef,
    WidgetState, WidgetStateProperty, WidgetStatePropertyAll, WidgetStatePropertyRef,
};

use crate::button_style::ButtonStyle;
use crate::colors::Colors;
use crate::debug::{debug_check_has_material, debug_check_has_material_localizations};
use crate::icons::Icons;
use crate::ink_well::{InkResponse, InkWell};
use crate::material::Material;
use crate::material_localizations::MaterialLocalizations;
use crate::text_button::TextButton;
use crate::theme::{K_THEME_ANIMATION_DURATION, Theme};

fn k_step_style() -> TextStyle {
    TextStyle::new().font_size(12.0).color(Colors::WHITE)
}
fn k_error_light() -> Color {
    Colors::RED.primary()
}
fn k_error_dark() -> Color {
    Colors::RED.shade400()
}
const K_CIRCLE_ACTIVE_LIGHT: Color = Colors::WHITE;
const K_CIRCLE_ACTIVE_DARK: Color = Colors::BLACK87;
const K_DISABLED_LIGHT: Color = Colors::BLACK38;
const K_DISABLED_DARK: Color = Colors::WHITE38;
const K_STEP_SIZE: f64 = 24.0;
/// `sqrt(3.0) / 2.0`.
const K_TRIANGLE_SQRT: f64 = 0.866_025;
const K_TRIANGLE_HEIGHT: f64 = K_STEP_SIZE * K_TRIANGLE_SQRT;
const K_MAX_STEP_SIZE: f64 = 80.0;
const K_DEFAULT_VERTICAL_CONTENT_PADDING: EdgeInsetsDirectional =
    EdgeInsetsDirectional::only(60.0, 0.0, 24.0, 24.0);
const K_DEFAULT_HORIZONTAL_CONTENT_PADDING: EdgeInsets = EdgeInsets::all(24.0);
const K_DEFAULT_HEADER_PADDING: EdgeInsetsGeometry = EdgeInsetsGeometry::symmetric(0.0, 24.0);

/// Dart's `<WidgetState>{if (isActive) WidgetState.selected else WidgetState.disabled}`.
fn selected_or_disabled(is_active: bool) -> waft_widgets::WidgetStates {
    let state = if is_active {
        WidgetState::Selected
    } else {
        WidgetState::Disabled
    };
    [state].into_iter().collect()
}

/// The state of a [`Step`], which is used to control the style of the circle and text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepState {
    /// A step that displays its index in its circle.
    Indexed,
    /// A step that displays a pencil icon in its circle.
    Editing,
    /// A step that displays a tick icon in its circle.
    Complete,
    /// A step that is disabled and does not react to taps.
    Disabled,
    /// A step that is currently having an error.
    Error,
}

/// Defines the [`Stepper`]'s main axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepperType {
    /// A vertical layout of the steps with their content in between the titles.
    Vertical,
    /// A horizontal layout of the steps with their content below the titles.
    Horizontal,
}

/// All the information necessary to build a [`Stepper`]'s forward and backward controls for a
/// given step.
///
/// Used by [`Stepper::controls_builder`].
#[derive(Clone, Debug)]
pub struct ControlsDetails {
    /// Index that is active for the surrounding [`Stepper`].
    pub current_step: usize,
    /// Index of the step for which these controls are being built.
    pub step_index: usize,
    /// The callback called when the 'continue' button is tapped.
    pub on_step_continue: Option<Listener>,
    /// The callback called when the 'cancel' button is tapped.
    pub on_step_cancel: Option<Listener>,
}

impl ControlsDetails {
    /// Whether the indicated step is also the current active step.
    pub fn is_active(&self) -> bool {
        self.current_step == self.step_index
    }
}

/// A builder that creates the controls from their [`ControlsDetails`].
pub type ControlsWidgetBuilder = Rc<dyn Fn(&mut App, BuildContext, ControlsDetails) -> WidgetRef>;

/// A builder that creates the icon for the step at the index, given its state.
pub type StepIconBuilder = Rc<dyn Fn(usize, StepState) -> Option<WidgetRef>>;

/// A material step used in [`Stepper`]. The step can have a title and subtitle, an icon within
/// its circle, some content and a state that governs its styling.
#[derive(Clone)]
pub struct Step {
    /// The title of the step that typically describes it.
    pub title: WidgetRef,
    /// The subtitle of the step that appears below the title.
    pub subtitle: Option<WidgetRef>,
    /// The content of the step that appears below the title and subtitle.
    pub content: WidgetRef,
    /// The state of the step which determines the styling of its components and whether steps
    /// are interactive.
    pub state: StepState,
    /// Whether or not the step is active. The flag only influences styling.
    pub is_active: bool,
    /// Only for [`StepperType::Horizontal`], an optional widget that appears under the title.
    pub label: Option<WidgetRef>,
    /// Optional overrides for the step's default visual configuration.
    pub step_style: Option<StepStyle>,
}

impl Step {
    /// Creates a step for a [`Stepper`]; Dart's optional named arguments are the setters.
    pub fn new<T, C>(title: impl IntoWidget<T>, content: impl IntoWidget<C>) -> Step {
        Step {
            title: title.into_widget(),
            subtitle: None,
            content: content.into_widget(),
            state: StepState::Indexed,
            is_active: false,
            label: None,
            step_style: None,
        }
    }

    /// Dart `Step(subtitle:)`.
    pub fn subtitle<K>(mut self, subtitle: impl IntoWidget<K>) -> Step {
        self.subtitle = Some(subtitle.into_widget());
        self
    }

    /// Dart `Step(state:)`.
    pub fn state(mut self, state: StepState) -> Step {
        self.state = state;
        self
    }

    /// Dart `Step(isActive:)`.
    pub fn is_active(mut self, is_active: bool) -> Step {
        self.is_active = is_active;
        self
    }

    /// Dart `Step(label:)`.
    pub fn label<K>(mut self, label: impl IntoWidget<K>) -> Step {
        self.label = Some(label.into_widget());
        self
    }

    /// Dart `Step(stepStyle:)`.
    pub fn step_style(mut self, step_style: StepStyle) -> Step {
        self.step_style = Some(step_style);
        self
    }
}

impl fmt::Debug for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Step")
            .field("state", &self.state)
            .field("is_active", &self.is_active)
            .finish_non_exhaustive()
    }
}

/// A material stepper widget that displays progress through a sequence of steps.
///
/// Steppers are particularly useful in the case of forms where one step requires the
/// completion of another one, or where multiple steps need to be completed in order to submit
/// the whole form.
///
/// The widget is a flexible wrapper. A parent class should pass [`current_step`](Self::current_step)
/// to this widget based on some logic triggered by the three callbacks that it provides.
#[derive(Clone)]
pub struct Stepper {
    pub key: Option<KeyRef>,
    /// The steps of the stepper whose titles, subtitles and icons always get shown.
    ///
    /// The length of [`steps`](Self::steps) must not change.
    pub steps: Vec<Step>,
    /// How the stepper's scroll view should respond to user input.
    pub physics: Option<waft_widgets::ScrollPhysicsRef>,
    /// An object that can be used to control the position to which the scroll view is scrolled.
    pub controller: Option<ScrollControllerRef>,
    /// The type of stepper that determines the layout.
    pub r#type: StepperType,
    /// The index into [`steps`](Self::steps) of the current step whose content is displayed.
    pub current_step: usize,
    /// The callback called when a step is tapped, with its index passed as an argument.
    pub on_step_tapped: Option<ValueChanged<usize>>,
    /// The callback called when the 'continue' button is tapped.
    pub on_step_continue: Option<Listener>,
    /// The callback called when the 'cancel' button is tapped.
    pub on_step_cancel: Option<Listener>,
    /// The callback for creating custom controls.
    pub controls_builder: Option<ControlsWidgetBuilder>,
    /// The elevation of this stepper's material when [`r#type`](Self::r#type) is
    /// [`StepperType::Horizontal`].
    pub elevation: Option<f64>,
    /// Custom margin on a vertical stepper.
    pub margin: Option<EdgeInsetsGeometry>,
    /// Customize the connected lines' colors.
    ///
    /// Resolves in the following states: [`WidgetState::Selected`] and
    /// [`WidgetState::Disabled`].
    pub connector_color: Option<WidgetStatePropertyRef<Color>>,
    /// The thickness of the connecting lines.
    pub connector_thickness: Option<f64>,
    /// Callback for creating custom icons for the steps.
    pub step_icon_builder: Option<StepIconBuilder>,
    /// Overrides the default step icon size height.
    pub step_icon_height: Option<f64>,
    /// Overrides the default step icon size width.
    pub step_icon_width: Option<f64>,
    /// Overrides the default step icon margin.
    pub step_icon_margin: Option<EdgeInsets>,
    /// The [`Step::content`] will be clipped to this [`Clip`] type.
    pub clip_behavior: Clip,
    /// The padding around the header row in both [`StepperType::Vertical`] and
    /// [`StepperType::Horizontal`] steppers.
    pub header_padding: Option<EdgeInsetsGeometry>,
    /// The padding around the content area in both stepper types.
    pub content_padding: Option<EdgeInsetsGeometry>,
}

impl Stepper {
    /// Creates a stepper from a list of steps; Dart's optional named arguments are the setters.
    pub fn new(steps: Vec<Step>) -> Stepper {
        debug_assert!(!steps.is_empty(), "current_step must name one of the steps");
        Stepper {
            key: None,
            steps,
            physics: None,
            controller: None,
            r#type: StepperType::Vertical,
            current_step: 0,
            on_step_tapped: None,
            on_step_continue: None,
            on_step_cancel: None,
            controls_builder: None,
            elevation: None,
            margin: None,
            connector_color: None,
            connector_thickness: None,
            step_icon_builder: None,
            step_icon_height: None,
            step_icon_width: None,
            step_icon_margin: None,
            clip_behavior: Clip::None,
            header_padding: None,
            content_padding: None,
        }
    }

    /// Dart `Stepper(key:)`.
    pub fn key(mut self, key: KeyRef) -> Stepper {
        self.key = Some(key);
        self
    }

    /// Dart `Stepper(physics:)`.
    pub fn physics(mut self, physics: waft_widgets::ScrollPhysicsRef) -> Stepper {
        self.physics = Some(physics);
        self
    }

    /// Dart `Stepper(controller:)`.
    pub fn controller(mut self, controller: ScrollControllerRef) -> Stepper {
        self.controller = Some(controller);
        self
    }

    /// Dart `Stepper(type:)`.
    pub fn r#type(mut self, r#type: StepperType) -> Stepper {
        self.r#type = r#type;
        self
    }

    /// Dart `Stepper(currentStep:)`.
    pub fn current_step(mut self, current_step: usize) -> Stepper {
        debug_assert!(
            current_step < self.steps.len(),
            "current_step must name one of the steps"
        );
        self.current_step = current_step;
        self
    }

    /// Dart `Stepper(onStepTapped:)`.
    pub fn on_step_tapped(mut self, on_step_tapped: ValueChanged<usize>) -> Stepper {
        self.on_step_tapped = Some(on_step_tapped);
        self
    }

    /// Dart `Stepper(onStepContinue:)`.
    pub fn on_step_continue(mut self, on_step_continue: Listener) -> Stepper {
        self.on_step_continue = Some(on_step_continue);
        self
    }

    /// Dart `Stepper(onStepCancel:)`.
    pub fn on_step_cancel(mut self, on_step_cancel: Listener) -> Stepper {
        self.on_step_cancel = Some(on_step_cancel);
        self
    }

    /// Dart `Stepper(controlsBuilder:)`.
    pub fn controls_builder(mut self, controls_builder: ControlsWidgetBuilder) -> Stepper {
        self.controls_builder = Some(controls_builder);
        self
    }

    /// Dart `Stepper(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> Stepper {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `Stepper(margin:)`.
    pub fn margin(mut self, margin: impl Into<EdgeInsetsGeometry>) -> Stepper {
        self.margin = Some(margin.into());
        self
    }

    /// Dart `Stepper(connectorColor:)`.
    pub fn connector_color(mut self, connector_color: WidgetStatePropertyRef<Color>) -> Stepper {
        self.connector_color = Some(connector_color);
        self
    }

    /// Dart `Stepper(connectorThickness:)`.
    pub fn connector_thickness(mut self, connector_thickness: f64) -> Stepper {
        self.connector_thickness = Some(connector_thickness);
        self
    }

    /// Dart `Stepper(stepIconBuilder:)`.
    pub fn step_icon_builder(mut self, step_icon_builder: StepIconBuilder) -> Stepper {
        self.step_icon_builder = Some(step_icon_builder);
        self
    }

    /// Dart `Stepper(stepIconHeight:)`.
    pub fn step_icon_height(mut self, step_icon_height: f64) -> Stepper {
        debug_assert!(
            (K_STEP_SIZE..=K_MAX_STEP_SIZE).contains(&step_icon_height),
            "step_icon_height must be greater than {K_STEP_SIZE} and less or equal to \
             {K_MAX_STEP_SIZE}"
        );
        self.step_icon_height = Some(step_icon_height);
        self
    }

    /// Dart `Stepper(stepIconWidth:)`.
    pub fn step_icon_width(mut self, step_icon_width: f64) -> Stepper {
        debug_assert!(
            (K_STEP_SIZE..=K_MAX_STEP_SIZE).contains(&step_icon_width),
            "step_icon_width must be greater than {K_STEP_SIZE} and less or equal to \
             {K_MAX_STEP_SIZE}"
        );
        self.step_icon_width = Some(step_icon_width);
        self
    }

    /// Dart `Stepper(stepIconMargin:)`.
    pub fn step_icon_margin(mut self, step_icon_margin: EdgeInsets) -> Stepper {
        self.step_icon_margin = Some(step_icon_margin);
        self
    }

    /// Dart `Stepper(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> Stepper {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart `Stepper(headerPadding:)`.
    pub fn header_padding(mut self, header_padding: impl Into<EdgeInsetsGeometry>) -> Stepper {
        self.header_padding = Some(header_padding.into());
        self
    }

    /// Dart `Stepper(contentPadding:)`.
    pub fn content_padding(mut self, content_padding: impl Into<EdgeInsetsGeometry>) -> Stepper {
        self.content_padding = Some(content_padding.into());
        self
    }
}

impl fmt::Debug for Stepper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Stepper")
            .field("current_step", &self.current_step)
            .field("type", &self.r#type)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for Stepper {
    type State = StepperState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> StepperState {
        StepperState {
            state: StateData::new(),
            keys: Vec::new(),
            old_states: HashMap::new(),
        }
    }
}

/// Dart's `_StepperState`.
pub struct StepperState {
    state: StateData<Stepper>,
    keys: Vec<GlobalKey>,
    old_states: HashMap<usize, StepState>,
}

impl StepperState {
    /// The padding around the header row.
    fn effective_header_padding(self: Handle<Self>, stepper: &Stepper) -> EdgeInsetsGeometry {
        stepper.header_padding.unwrap_or(K_DEFAULT_HEADER_PADDING)
    }

    /// How tall the horizontal header is, as a multiple of [`Stepper::step_icon_height`].
    fn height_factor(self: Handle<Self>, stepper: &Stepper) -> f64 {
        if self.is_label(stepper) && stepper.step_icon_height.is_some() {
            2.5
        } else {
            2.0
        }
    }

    fn is_first(self: Handle<Self>, index: usize) -> bool {
        index == 0
    }

    fn is_last(self: Handle<Self>, stepper: &Stepper, index: usize) -> bool {
        stepper.steps.len() - 1 == index
    }

    fn is_current(self: Handle<Self>, stepper: &Stepper, index: usize) -> bool {
        stepper.current_step == index
    }

    fn is_dark(self: Handle<Self>, app: &mut App, context: BuildContext) -> bool {
        Theme::brightness_of(app, context) == Brightness::Dark
    }

    fn is_label(self: Handle<Self>, stepper: &Stepper) -> bool {
        stepper.steps.iter().any(|step| step.label.is_some())
    }

    fn step_style(self: Handle<Self>, stepper: &Stepper, index: usize) -> Option<StepStyle> {
        stepper.steps[index].step_style.clone()
    }

    fn connector_color(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        is_active: bool,
    ) -> Color {
        let color_scheme = Theme::of(app, context).color_scheme.clone();
        let states = selected_or_disabled(is_active);
        let resolved_connector_color = self
            .widget(app)
            .connector_color
            .as_ref()
            .map(|connector_color| connector_color.resolve(&states));
        resolved_connector_color.unwrap_or_else(|| {
            if is_active {
                color_scheme.primary()
            } else {
                Colors::GREY.shade400()
            }
        })
    }

    fn build_line(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        stepper: &Stepper,
        visible: bool,
        is_active: bool,
    ) -> WidgetRef {
        let color = self.connector_color(app, context, is_active);
        let width = if visible {
            stepper.connector_thickness.unwrap_or(1.0)
        } else {
            0.0
        };
        ColoredBox::new(color)
            .child(SizedBox::new().width(width).height(16.0))
            .into_widget()
    }

    fn build_circle_child(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        index: usize,
        old_state: bool,
    ) -> WidgetRef {
        let stepper = self.widget(app).clone();
        let state = if old_state {
            app.get(self).old_states[&index]
        } else {
            stepper.steps[index].state
        };
        if let Some(icon) = stepper
            .step_icon_builder
            .as_ref()
            .and_then(|builder| builder(index, state))
        {
            return icon;
        }
        let is_dark_active = self.is_dark(app, context) && stepper.steps[index].is_active;
        let icon_color = if is_dark_active {
            K_CIRCLE_ACTIVE_DARK
        } else {
            K_CIRCLE_ACTIVE_LIGHT
        };
        let text_style = self
            .step_style(&stepper, index)
            .and_then(|step_style| step_style.index_style.clone())
            .unwrap_or_else(|| {
                if is_dark_active {
                    k_step_style().copy_with().color(Colors::BLACK87)
                } else {
                    k_step_style()
                }
            });

        match state {
            StepState::Indexed | StepState::Disabled => Text::new(format!("{}", index + 1))
                .style(text_style)
                .into_widget(),
            StepState::Editing => Icon::new(Some(Icons::edit()))
                .color(icon_color)
                .size(18.0)
                .into_widget(),
            StepState::Complete => Icon::new(Some(Icons::check()))
                .color(icon_color)
                .size(18.0)
                .into_widget(),
            StepState::Error => Center::new()
                .child(Text::new("!").style(k_step_style()))
                .into_widget(),
        }
    }

    #[expect(
        deprecated,
        reason = "Dart's `_circleColor` reads the deprecated `ColorScheme.background` and \
                  `Color.withOpacity`"
    )]
    fn circle_color(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        stepper: &Stepper,
        index: usize,
    ) -> Color {
        let is_active = stepper.steps[index].is_active;
        let color_scheme = Theme::of(app, context).color_scheme.clone();
        let states = selected_or_disabled(is_active);
        if let Some(resolved_connector_color) = stepper
            .connector_color
            .as_ref()
            .map(|connector_color| connector_color.resolve(&states))
        {
            return resolved_connector_color;
        }
        if !self.is_dark(app, context) {
            if is_active {
                color_scheme.primary()
            } else {
                color_scheme.on_surface().with_opacity(0.38)
            }
        } else if is_active {
            color_scheme.secondary()
        } else {
            color_scheme.background()
        }
    }

    fn build_circle(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        index: usize,
        old_state: bool,
    ) -> WidgetRef {
        let stepper = self.widget(app).clone();
        let step_style = self.step_style(&stepper, index);
        let color = step_style
            .as_ref()
            .and_then(|step_style| step_style.color)
            .unwrap_or_else(|| self.circle_color(app, context, &stepper, index));
        let mut decoration = BoxDecoration::new().color(color).shape(BoxShape::Circle);
        if let Some(border) = step_style
            .as_ref()
            .and_then(|step_style| step_style.border.clone())
        {
            decoration = decoration.border(border);
        }
        if let Some(box_shadow) = step_style
            .as_ref()
            .and_then(|step_style| step_style.box_shadow)
        {
            decoration = decoration.box_shadow(vec![box_shadow]);
        }
        if let Some(gradient) = step_style
            .as_ref()
            .and_then(|step_style| step_style.gradient.clone())
        {
            decoration = decoration.gradient(gradient);
        }
        let circle_child = self.build_circle_child(
            app,
            context,
            index,
            old_state && stepper.steps[index].state == StepState::Error,
        );
        Padding::new(
            stepper
                .step_icon_margin
                .unwrap_or(EdgeInsets::symmetric(8.0, 0.0)),
        )
        .child(
            SizedBox::new()
                .width(stepper.step_icon_width.unwrap_or(K_STEP_SIZE))
                .height(stepper.step_icon_height.unwrap_or(K_STEP_SIZE))
                .child(
                    AnimatedContainer::new(K_THEME_ANIMATION_DURATION)
                        .curve(waft_animation::Curves::fast_out_slow_in())
                        .decoration(decoration)
                        .child(Center::new().child(circle_child)),
                ),
        )
        .into_widget()
    }

    fn build_triangle(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        index: usize,
        old_state: bool,
    ) -> WidgetRef {
        let stepper = self.widget(app).clone();
        let color = self
            .step_style(&stepper, index)
            .and_then(|step_style| step_style.error_color)
            .unwrap_or_else(|| {
                if self.is_dark(app, context) {
                    k_error_dark()
                } else {
                    k_error_light()
                }
            });
        let triangle_height = stepper
            .step_icon_height
            .map(|height| height * K_TRIANGLE_SQRT)
            .unwrap_or(K_TRIANGLE_HEIGHT);
        let circle_child = self.build_circle_child(
            app,
            context,
            index,
            old_state && stepper.steps[index].state != StepState::Error,
        );
        Padding::new(
            stepper
                .step_icon_margin
                .unwrap_or(EdgeInsets::symmetric(8.0, 0.0)),
        )
        .child(
            SizedBox::new()
                .width(stepper.step_icon_width.unwrap_or(K_STEP_SIZE))
                .height(stepper.step_icon_height.unwrap_or(K_STEP_SIZE))
                .child(
                    Center::new().child(
                        SizedBox::new()
                            .width(stepper.step_icon_width.unwrap_or(K_STEP_SIZE))
                            .height(triangle_height)
                            .child(
                                CustomPaint::new().painter(TrianglePainter { color }).child(
                                    Align::new()
                                        // 0.8 looks better than the geometrical 0.33.
                                        .alignment(Alignment::new(0.0, 0.8).into())
                                        .child(circle_child),
                                ),
                            ),
                    ),
                ),
        )
        .into_widget()
    }

    fn build_icon(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        index: usize,
    ) -> WidgetRef {
        let stepper = self.widget(app).clone();
        let state = stepper.steps[index].state;
        let old_state = app.get(self).old_states[&index];
        if state != old_state {
            AnimatedCrossFade::new(
                self.build_circle(app, context, index, true),
                self.build_triangle(app, context, index, true),
                if state == StepState::Error {
                    waft_widgets::CrossFadeState::ShowSecond
                } else {
                    waft_widgets::CrossFadeState::ShowFirst
                },
                K_THEME_ANIMATION_DURATION,
            )
            .first_curve(Rc::new(Interval::new(
                0.0,
                0.6,
                waft_animation::Curves::fast_out_slow_in(),
            )))
            .second_curve(Rc::new(Interval::new(
                0.4,
                1.0,
                waft_animation::Curves::fast_out_slow_in(),
            )))
            .size_curve(waft_animation::Curves::fast_out_slow_in())
            .into_widget()
        } else if state != StepState::Error {
            self.build_circle(app, context, index, false)
        } else {
            self.build_triangle(app, context, index, false)
        }
    }

    fn title_style(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        index: usize,
    ) -> TextStyle {
        let stepper = self.widget(app).clone();
        let text_theme = Theme::of(app, context).text_theme.clone();
        let body_large = text_theme
            .body_large
            .clone()
            .expect("the text theme has a body large");
        match stepper.steps[index].state {
            StepState::Indexed | StepState::Editing | StepState::Complete => body_large,
            StepState::Disabled => body_large.copy_with().color(if self.is_dark(app, context) {
                K_DISABLED_DARK
            } else {
                K_DISABLED_LIGHT
            }),
            StepState::Error => body_large.copy_with().color(if self.is_dark(app, context) {
                k_error_dark()
            } else {
                k_error_light()
            }),
        }
    }

    fn subtitle_style(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        index: usize,
    ) -> TextStyle {
        let stepper = self.widget(app).clone();
        let text_theme = Theme::of(app, context).text_theme.clone();
        let body_small = text_theme
            .body_small
            .clone()
            .expect("the text theme has a body small");
        match stepper.steps[index].state {
            StepState::Indexed | StepState::Editing | StepState::Complete => body_small,
            StepState::Disabled => body_small.copy_with().color(if self.is_dark(app, context) {
                K_DISABLED_DARK
            } else {
                K_DISABLED_LIGHT
            }),
            StepState::Error => body_small.copy_with().color(if self.is_dark(app, context) {
                k_error_dark()
            } else {
                k_error_light()
            }),
        }
    }

    fn label_style(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        index: usize,
    ) -> TextStyle {
        self.title_style(app, context, index)
    }

    fn build_header_text(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        index: usize,
    ) -> WidgetRef {
        let stepper = self.widget(app).clone();
        let title_style = self.title_style(app, context, index);
        let mut children = vec![
            AnimatedDefaultTextStyle::new(
                stepper.steps[index].title.clone(),
                title_style,
                K_THEME_ANIMATION_DURATION,
            )
            .curve(waft_animation::Curves::fast_out_slow_in())
            .into_widget(),
        ];
        if let Some(subtitle) = stepper.steps[index].subtitle.clone() {
            let style = self.subtitle_style(app, context, index);
            children.push(
                Padding::new(EdgeInsets::only(0.0, 2.0, 0.0, 0.0))
                    .child(
                        AnimatedDefaultTextStyle::new(subtitle, style, K_THEME_ANIMATION_DURATION)
                            .curve(waft_animation::Curves::fast_out_slow_in()),
                    )
                    .into_widget(),
            );
        }
        Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .main_axis_size(MainAxisSize::Min)
            .children(children)
            .into_widget()
    }

    fn build_label_text(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        index: usize,
    ) -> WidgetRef {
        let stepper = self.widget(app).clone();
        match stepper.steps[index].label.clone() {
            Some(label) => {
                let style = self.label_style(app, context, index);
                AnimatedDefaultTextStyle::new(label, style, K_THEME_ANIMATION_DURATION)
                    .into_widget()
            }
            None => SizedBox::shrink().into_widget(),
        }
    }

    fn build_vertical_header(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        index: usize,
    ) -> WidgetRef {
        let stepper = self.widget(app).clone();
        let is_active = stepper.steps[index].is_active;
        let is_previous_active = index > 0 && stepper.steps[index - 1].is_active;
        let padding = self.effective_header_padding(&stepper);
        let first_line = self.build_line(
            app,
            context,
            &stepper,
            !self.is_first(index),
            is_previous_active,
        );
        let icon = self.build_icon(app, context, index);
        let last_line = self.build_line(
            app,
            context,
            &stepper,
            !self.is_last(&stepper, index),
            is_active,
        );
        let header_text = self.build_header_text(app, context, index);
        Padding::new(padding)
            .child(Row::new().children(vec![
                    Column::new()
                        .children(vec![first_line, icon, last_line])
                        .into_widget(),
                    Expanded::new(
                        Padding::new(EdgeInsetsDirectional::only(12.0, 0.0, 0.0, 0.0))
                            .child(header_text),
                    )
                    .into_widget(),
                ]))
            .into_widget()
    }

    fn build_vertical_controls(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        step_index: usize,
    ) -> WidgetRef {
        let stepper = self.widget(app).clone();
        if let Some(controls_builder) = stepper.controls_builder.clone() {
            return controls_builder(
                app,
                context,
                ControlsDetails {
                    current_step: stepper.current_step,
                    on_step_continue: stepper.on_step_continue.clone(),
                    on_step_cancel: stepper.on_step_cancel.clone(),
                    step_index,
                },
            );
        }

        let is_dark = self.is_dark(app, context);
        let cancel_color = match Theme::brightness_of(app, context) {
            Brightness::Light => Colors::BLACK54,
            Brightness::Dark => Colors::WHITE70,
        };

        let theme_data = Theme::of(app, context);
        let color_scheme = theme_data.color_scheme.clone();
        let localizations = <dyn MaterialLocalizations>::of(app, context);

        let button_shape = OutlinedBorderRef::new(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::all(waft_embedder::Radius::circular(2.0)),
        ));
        let button_padding = EdgeInsetsGeometry::symmetric(0.0, 16.0);

        let foreground_color_scheme = color_scheme.clone();
        let continue_foreground_color: WidgetStatePropertyRef<Option<Color>> =
            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                if states.contains(&WidgetState::Disabled) {
                    None
                } else if is_dark {
                    Some(foreground_color_scheme.on_surface())
                } else {
                    Some(foreground_color_scheme.on_primary())
                }
            });
        let continue_background_color: WidgetStatePropertyRef<Option<Color>> =
            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                if is_dark || states.contains(&WidgetState::Disabled) {
                    None
                } else {
                    Some(color_scheme.primary())
                }
            });
        let continue_style = ButtonStyle::new()
            .foreground_color(continue_foreground_color)
            .background_color(continue_background_color)
            .padding(Rc::new(WidgetStatePropertyAll::new(Some(button_padding))))
            .shape(Rc::new(WidgetStatePropertyAll::new(Some(
                button_shape.clone(),
            ))));

        let continue_label = if theme_data.use_material3 {
            localizations.continue_button_label()
        } else {
            localizations.continue_button_label().to_uppercase()
        };
        let cancel_label = if theme_data.use_material3 {
            localizations.cancel_button_label()
        } else {
            localizations.cancel_button_label().to_uppercase()
        };
        let cancel_style = TextButton::style_from()
            .foreground_color(cancel_color)
            .padding(button_padding)
            .shape(button_shape)
            .build();

        let continue_button = TextButton::new(
            stepper.on_step_continue.clone(),
            Some(Text::new(continue_label)),
        )
        .style(continue_style);
        let cancel_button = TextButton::new(
            stepper.on_step_cancel.clone(),
            Some(Text::new(cancel_label)),
        )
        .style(cancel_style);

        Padding::new(EdgeInsets::only(0.0, 16.0, 0.0, 0.0))
            .child(SizedBox::new().height(48.0).child(Row::new().children(vec![
                        continue_button.into_widget(),
                        Padding::new(EdgeInsetsDirectional::only(8.0, 0.0, 0.0, 0.0))
                            .child(cancel_button)
                            .into_widget(),
                    ])))
            .into_widget()
    }

    fn build_vertical_body(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        index: usize,
    ) -> WidgetRef {
        let stepper = self.widget(app).clone();
        let step_icon_margin = stepper.step_icon_margin;
        let margin_left =
            step_icon_margin.map(|margin| margin.resolve(Some(TextDirection::Ltr)).left);
        let margin_right =
            step_icon_margin.map(|margin| margin.resolve(Some(TextDirection::Ltr)).right);
        let additional_margin_left = margin_left.map(|margin_left| margin_left / 2.0);
        let additional_margin_right = margin_right.map(|margin_right| margin_right / 2.0);
        // Adjust vertical content padding to align with step icon when stepIconMargin is set.
        let effective_vertical_content_padding = stepper
            .content_padding
            .unwrap_or(K_DEFAULT_VERTICAL_CONTENT_PADDING.into())
            .add(EdgeInsetsDirectional::only(margin_left.unwrap_or(0.0), 0.0, 0.0, 0.0).into());

        let connector_width = if !self.is_last(&stepper, index) {
            stepper.connector_thickness.unwrap_or(1.0)
        } else {
            0.0
        };
        let connector_color = self.connector_color(app, context, stepper.steps[index].is_active);
        let line = PositionedDirectional::new(
            Center::new().child(
                SizedBox::new()
                    .width(connector_width)
                    .height(f64::INFINITY)
                    .child(ColoredBox::new(connector_color)),
            ),
        )
        // When use margin affects the left or right side of the child, we need to add half of the
        // margin to the start or end of the child respectively to get the correct positioning.
        .start(
            24.0 + additional_margin_left.unwrap_or(0.0) + additional_margin_right.unwrap_or(0.0),
        )
        .top(0.0)
        .bottom(0.0)
        .width(stepper.step_icon_width.unwrap_or(K_STEP_SIZE));

        let content = stepper.steps[index].content.clone();
        let controls = self.build_vertical_controls(app, context, index);
        let clip_behavior = stepper.clip_behavior;
        let is_current = self.is_current(&stepper, index);
        let body = AnimatedCrossFade::new(
            SizedBox::new().width(f64::INFINITY).height(0.0),
            Padding::new(effective_vertical_content_padding).child(Column::new().children(vec![
                    ClipRect::new()
                        .clip_behavior(clip_behavior)
                        .child(content)
                        .into_widget(),
                    controls,
                ])),
            if is_current {
                waft_widgets::CrossFadeState::ShowSecond
            } else {
                waft_widgets::CrossFadeState::ShowFirst
            },
            K_THEME_ANIMATION_DURATION,
        )
        .first_curve(Rc::new(Interval::new(
            0.0,
            0.6,
            waft_animation::Curves::fast_out_slow_in(),
        )))
        .second_curve(Rc::new(Interval::new(
            0.4,
            1.0,
            waft_animation::Curves::fast_out_slow_in(),
        )))
        .size_curve(waft_animation::Curves::fast_out_slow_in())
        .into_widget();

        Stack::new()
            .children(vec![line.into_widget(), body])
            .into_widget()
    }

    fn build_vertical(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let stepper = self.widget(app).clone();
        let keys = app.get(self).keys.clone();
        let children: Vec<WidgetRef> = (0..stepper.steps.len())
            .map(|index| {
                let key = keys[index].clone();
                let is_disabled = stepper.steps[index].state == StepState::Disabled;
                let on_step_tapped = stepper.on_step_tapped.clone();
                let tap_key = key.clone();
                let mut ink_well = InkWell::new().can_request_focus(!is_disabled);
                if !is_disabled {
                    ink_well = ink_well.on_tap(Listener::new(move |app: &mut App| {
                        // In the vertical case we need to scroll to the newly tapped step.
                        let current_context = tap_key
                            .current_context(app)
                            .expect("the tapped step is mounted");
                        drop(Scrollable::ensure_visible(
                            app,
                            current_context,
                            0.0,
                            K_THEME_ANIMATION_DURATION,
                            waft_animation::Curves::fast_out_slow_in(),
                            ScrollPositionAlignmentPolicy::Explicit,
                        ));
                        if let Some(on_step_tapped) = on_step_tapped.as_ref() {
                            on_step_tapped(app, index);
                        }
                    }));
                }
                let header = self.build_vertical_header(app, context, index);
                let body = self.build_vertical_body(app, context, index);
                Column::new()
                    .key(KeyRef::Global(key))
                    .children(vec![ink_well.child(header).into_widget(), body])
                    .into_widget()
            })
            .collect();

        let mut list_view = ListView::new().shrink_wrap(true).children(children);
        if let Some(controller) = stepper.controller.clone() {
            list_view = list_view.controller(controller);
        }
        if let Some(physics) = stepper.physics.clone() {
            list_view = list_view.physics(physics);
        }
        list_view.into_widget()
    }

    fn build_horizontal(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let stepper = self.widget(app).clone();
        let is_label = self.is_label(&stepper);

        let mut children: Vec<WidgetRef> = Vec::new();
        for index in 0..stepper.steps.len() {
            let is_disabled = stepper.steps[index].state == StepState::Disabled;
            let on_step_tapped = stepper.on_step_tapped.clone();
            let mut ink_response = InkResponse::new().can_request_focus(!is_disabled);
            if !is_disabled {
                ink_response = ink_response.on_tap(Listener::new(move |app: &mut App| {
                    if let Some(on_step_tapped) = on_step_tapped.as_ref() {
                        on_step_tapped(app, index);
                    }
                }));
            }

            let has_label = stepper.steps[index].label.is_some();
            let icon = self.build_icon(app, context, index);
            let label = self.build_label_text(app, context, index);
            let header_text = self.build_header_text(app, context, index);
            let margin: EdgeInsetsGeometry = stepper.step_icon_margin.map_or_else(
                || EdgeInsetsDirectional::only(12.0, 0.0, 0.0, 0.0).into(),
                |margin| margin.into(),
            );

            let mut column_children = Vec::new();
            if has_label {
                column_children.push(SizedBox::new().height(24.0).into_widget());
            }
            column_children.push(Center::new().child(icon).into_widget());
            if has_label {
                column_children.push(SizedBox::new().height(24.0).child(label).into_widget());
            }

            children.push(
                ink_response
                    .child(Row::new().children(vec![
                            SizedBox::new()
                                .height(if is_label { 104.0 } else { 72.0 })
                                .child(
                                    Column::new()
                                        .main_axis_alignment(MainAxisAlignment::Center)
                                        .children(column_children),
                                )
                                .into_widget(),
                            Padding::new(margin).child(header_text).into_widget(),
                        ]))
                    .into_widget(),
            );

            if !self.is_last(&stepper, index) {
                let connector_height = stepper.steps[index]
                    .step_style
                    .as_ref()
                    .and_then(|step_style| step_style.connector_thickness)
                    .or(stepper.connector_thickness)
                    .unwrap_or(1.0);
                let connector_color = stepper.steps[index]
                    .step_style
                    .as_ref()
                    .and_then(|step_style| step_style.connector_color)
                    .unwrap_or_else(|| {
                        self.connector_color(app, context, stepper.steps[index].is_active)
                    });
                let connector_margin = stepper.step_icon_margin.map_or_else(
                    || EdgeInsetsGeometry::symmetric(0.0, 8.0),
                    |margin| margin.into(),
                );
                children.push(
                    Expanded::new(
                        Padding::new(connector_margin).child(
                            SizedBox::new()
                                .height(connector_height)
                                .child(ColoredBox::new(connector_color)),
                        ),
                    )
                    .into_widget(),
                );
            }
        }

        let mut step_panels: Vec<WidgetRef> = Vec::new();
        for index in 0..stepper.steps.len() {
            step_panels.push(
                Visibility::new(
                    ClipRect::new()
                        .clip_behavior(stepper.clip_behavior)
                        .child(stepper.steps[index].content.clone()),
                )
                .maintain_state(true)
                .visible(index == stepper.current_step)
                .into_widget(),
            );
        }

        let header_height = stepper
            .step_icon_height
            .map(|height| height * self.height_factor(&stepper));
        let mut header_row = SizedBox::new().child(Row::new().children(children));
        if let Some(header_height) = header_height {
            header_row = header_row.height(header_height);
        }
        let header = Material::new()
            .elevation(stepper.elevation.unwrap_or(2.0))
            .child(Padding::new(self.effective_header_padding(&stepper)).child(header_row));

        let content = AnimatedSize::new(K_THEME_ANIMATION_DURATION)
            .curve(waft_animation::Curves::fast_out_slow_in())
            .child(
                Column::new()
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .children(step_panels),
            );
        let controls = self.build_vertical_controls(app, context, stepper.current_step);
        // Effective horizontal content padding (custom or default).
        let horizontal_content_padding = stepper
            .content_padding
            .unwrap_or(K_DEFAULT_HORIZONTAL_CONTENT_PADDING.into());
        let mut list_view = ListView::new()
            .padding(horizontal_content_padding)
            .children(vec![content.into_widget(), controls]);
        if let Some(controller) = stepper.controller.clone() {
            list_view = list_view.controller(controller);
        }
        if let Some(physics) = stepper.physics.clone() {
            list_view = list_view.physics(physics);
        }

        Column::new()
            .children(vec![
                header.into_widget(),
                Expanded::new(list_view).into_widget(),
            ])
            .into_widget()
    }
}

impl fmt::Debug for StepperState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StepperState").finish_non_exhaustive()
    }
}

impl State for StepperState {
    type Widget = Stepper;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let stepper = self.widget(app).clone();
        let keys = (0..stepper.steps.len()).map(|_| GlobalKey::new()).collect();
        let mut old_states = HashMap::new();
        for (index, step) in stepper.steps.iter().enumerate() {
            old_states.insert(index, step.state);
        }
        let state = app.get_mut(self);
        state.keys = keys;
        state.old_states = old_states;
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &Stepper) {
        assert_eq!(
            self.widget(app).steps.len(),
            old_widget.steps.len(),
            "the length of steps must not change"
        );
        let mut old_states = HashMap::new();
        for (index, step) in old_widget.steps.iter().enumerate() {
            old_states.insert(index, step.state);
        }
        app.get_mut(self).old_states = old_states;
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let stepper = self.widget(app).clone();
        debug_assert!(
            stepper.step_icon_height.is_none()
                || stepper.step_icon_width.is_none()
                || stepper.step_icon_height == stepper.step_icon_width,
            "if either step_icon_height or step_icon_width is specified, both must be specified \
             and the values must be equal"
        );
        debug_assert!(debug_check_has_material(app, context));
        debug_assert!(debug_check_has_material_localizations(app, context));
        debug_assert!(
            context
                .find_ancestor_widget_of_exact_type::<Stepper>(app)
                .is_none(),
            "Steppers must not be nested.\n\
             The material specification advises that one should avoid embedding steppers within \
             steppers. \
             https://material.io/archive/guidelines/components/steppers.html#steppers-usage"
        );
        match stepper.r#type {
            StepperType::Vertical => self.build_vertical(app, context),
            StepperType::Horizontal => self.build_horizontal(app, context),
        }
    }
}

/// Paints a triangle whose base is the bottom of the bounding rectangle and its top vertex the
/// middle of its top.
struct TrianglePainter {
    color: Color,
}

impl CustomPainter for TrianglePainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        let base = size.width();
        let half_base = size.width() / 2.0;
        let height = size.height();
        let mut path = PathBuilder::new();
        path.move_to(valo::Point::new(0.0, height as f32));
        path.line_to(valo::Point::new(base as f32, height as f32));
        path.line_to(valo::Point::new(half_base as f32, 0.0));
        path.close();
        canvas.draw_path(
            &path.build(),
            FillRule::NonZero,
            &Paint::from_color(self.color.into()),
        );
    }

    // Hitting the rectangle is fine enough.
    fn hit_test(&self, _app: &App, _position: Offset) -> Option<bool> {
        Some(true)
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        old_delegate
            .as_any()
            .downcast_ref::<TrianglePainter>()
            .is_none_or(|old_delegate| old_delegate.color != self.color)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Used to override the default visual properties of [`Step`] widgets within a [`Stepper`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StepStyle {
    /// Overrides the default color of the circle in the step.
    pub color: Option<Color>,
    /// Overrides the default color of the error indicator in the step.
    pub error_color: Option<Color>,
    /// Overrides the default color of the connector line between two steps.
    ///
    /// This property only applies when [`Stepper::r#type`] is [`StepperType::Horizontal`].
    pub connector_color: Option<Color>,
    /// Overrides the default thickness of the connector line between two steps.
    ///
    /// This property only applies when [`Stepper::r#type`] is [`StepperType::Horizontal`].
    pub connector_thickness: Option<f64>,
    /// Adds a border around the step.
    ///
    /// Will be applied to the circle in the step.
    pub border: Option<BoxBorderRef>,
    /// Adds a shadow around the step.
    pub box_shadow: Option<BoxShadow>,
    /// Adds a gradient around the step.
    ///
    /// If [`gradient`](Self::gradient) is specified, [`color`](Self::color) will be ignored.
    pub gradient: Option<GradientRef>,
    /// Overrides the default style of the index in the step.
    pub index_style: Option<TextStyle>,
}

impl StepStyle {
    /// Constructs a set of step properties; Dart's optional named arguments are the setters,
    /// which carry `with_` so that they do not collide with the fields.
    pub fn new() -> StepStyle {
        StepStyle::default()
    }

    /// Dart `StepStyle(color:)`.
    pub fn with_color(mut self, color: Color) -> StepStyle {
        self.color = Some(color);
        self
    }

    /// Dart `StepStyle(errorColor:)`.
    pub fn with_error_color(mut self, error_color: Color) -> StepStyle {
        self.error_color = Some(error_color);
        self
    }

    /// Dart `StepStyle(connectorColor:)`.
    pub fn with_connector_color(mut self, connector_color: Color) -> StepStyle {
        self.connector_color = Some(connector_color);
        self
    }

    /// Dart `StepStyle(connectorThickness:)`.
    pub fn with_connector_thickness(mut self, connector_thickness: f64) -> StepStyle {
        self.connector_thickness = Some(connector_thickness);
        self
    }

    /// Dart `StepStyle(border:)`.
    pub fn with_border(mut self, border: impl Into<BoxBorderRef>) -> StepStyle {
        self.border = Some(border.into());
        self
    }

    /// Dart `StepStyle(boxShadow:)`.
    pub fn with_box_shadow(mut self, box_shadow: BoxShadow) -> StepStyle {
        self.box_shadow = Some(box_shadow);
        self
    }

    /// Dart `StepStyle(gradient:)`.
    pub fn with_gradient(mut self, gradient: impl Into<GradientRef>) -> StepStyle {
        self.gradient = Some(gradient.into());
        self
    }

    /// Dart `StepStyle(indexStyle:)`.
    pub fn with_index_style(mut self, index_style: TextStyle) -> StepStyle {
        self.index_style = Some(index_style);
        self
    }

    /// Returns a copy of this style with the given fields replaced with the new values.
    pub fn copy_with(&self) -> StepStyle {
        self.clone()
    }

    /// Returns a copy of this style where the fields `step_style` sets replace the
    /// corresponding fields of this style.
    pub fn merge(&self, step_style: Option<&StepStyle>) -> StepStyle {
        let Some(step_style) = step_style else {
            return self.clone();
        };
        StepStyle {
            color: step_style.color.or(self.color),
            error_color: step_style.error_color.or(self.error_color),
            connector_color: step_style.connector_color.or(self.connector_color),
            connector_thickness: step_style.connector_thickness.or(self.connector_thickness),
            border: step_style.border.clone().or_else(|| self.border.clone()),
            box_shadow: step_style.box_shadow.or(self.box_shadow),
            gradient: step_style
                .gradient
                .clone()
                .or_else(|| self.gradient.clone()),
            index_style: step_style
                .index_style
                .clone()
                .or_else(|| self.index_style.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::time::Duration;

    use waft_embedder::{Rect, Size};
    use waft_foundation::ValueKey;
    use waft_painting::{Border, BorderStyle, LinearGradient, ShapeBorderRef};
    use waft_rendering::testing::{PaintCall, paints};
    use waft_rendering::{RenderFlex, RenderFlexMixin};
    use waft_widgets::testing::{
        WidgetTester, assert_more_or_less_equals, assert_more_or_less_equals_within, find,
        test_widgets,
    };
    use waft_widgets::{
        AnyElement, BoxScrollView, Center, Container, Focus, GestureDetector, IntoWidget, ListView,
        NeverScrollableScrollPhysics, ScrollController, ScrollControllerLeaf, ScrollView,
        ScrollableState, SizedBox, StatefulBuilder, Text, downcast_widget,
    };

    use super::*;
    use crate::material::Material;
    use crate::scaffold::Scaffold;
    use crate::test_support::{PaintInspection, material_app};
    use crate::theme_data::ThemeData;

    /// Dart's `Step(title: Text(title), content: SizedBox(width: width, height: height))`.
    fn sized_step(title: &'static str, width: f64, height: f64) -> Step {
        Step::new(
            Text::new(title),
            SizedBox::new().width(width).height(height),
        )
    }

    /// Dart's `Step(title: Text(title), content: Text(content))`.
    fn text_step(title: &'static str, content: &'static str) -> Step {
        Step::new(Text::new(title), Text::new(content))
    }

    fn light_theme(tester: &WidgetTester) -> ThemeData {
        ThemeData::builder().build(&tester.cell.borrow())
    }

    fn material2_theme(tester: &WidgetTester) -> ThemeData {
        ThemeData::builder()
            .use_material3(false)
            .build(&tester.cell.borrow())
    }

    /// Dart's `pumpWidget(MaterialApp(home: Material(child: stepper)))`.
    fn pump(tester: &mut WidgetTester, stepper: Stepper) {
        let theme = light_theme(tester);
        tester.pump_widget(material_app(
            theme,
            Material::new().child(stepper).into_widget(),
        ));
    }

    /// Dart's `pumpWidget(MaterialApp(home: Center(child: Material(child: stepper))))`.
    fn pump_centered(tester: &mut WidgetTester, stepper: Stepper) {
        let theme = light_theme(tester);
        tester.pump_widget(material_app(
            theme,
            Center::new()
                .child(Material::new().child(stepper))
                .into_widget(),
        ));
    }

    /// Dart's `pumpWidget(MaterialApp(home: Scaffold(body: Center(child: child))))`.
    fn pump_in_scaffold(tester: &mut WidgetTester, theme: ThemeData, child: WidgetRef) {
        tester.pump_widget(material_app(
            theme,
            Scaffold::new()
                .body(Center::new().child(child))
                .into_widget(),
        ));
    }

    fn count(tester: &WidgetTester, finder: waft_widgets::testing::Finder) -> usize {
        finder.evaluate(tester).len()
    }

    fn stepper_height(tester: &WidgetTester) -> f64 {
        tester.get_size(find::by_type::<Stepper>()).height()
    }

    /// Dart's `(tester.widget<AnimatedContainer>(find.widgetWithText(AnimatedContainer, text))
    /// .decoration as BoxDecoration?)?.color`.
    fn circle_color(tester: &WidgetTester, text: &str) -> Option<Color> {
        tester.widget(
            find::widget_with_text::<AnimatedContainer>(text),
            |container: &AnimatedContainer| {
                container.decoration.as_ref().and_then(|decoration| {
                    decoration
                        .as_any()
                        .downcast_ref::<BoxDecoration>()
                        .and_then(|decoration| decoration.color.as_ref())
                        .map(|color| color.color())
                })
            },
        )
    }

    /// Dart's `tester.widget<ColoredBox>(finder).color`.
    fn colored_box_color(tester: &WidgetTester, element: AnyElement) -> Color {
        tester.widget(element, |colored_box: &ColoredBox| {
            colored_box.color.color()
        })
    }

    /// The `ListView` a stepper builds: Dart's `tester.widget<ListView>(find.descendant(of:
    /// find.byType(Stepper), matching: find.byType(ListView)))`.
    fn stepper_list_view<R>(tester: &WidgetTester, read: impl FnOnce(&ListView) -> R) -> R {
        tester.widget(
            find::descendant(find::by_type::<Stepper>(), find::by_type::<ListView>()),
            read,
        )
    }

    /// The paddings above `text`, nearest first: Dart's `tester.widgetList<Padding>(
    /// find.ancestor(of: find.text(text), matching: find.byType(Padding)))`.
    fn paddings_above(tester: &WidgetTester, text: &str) -> Vec<EdgeInsets> {
        find::ancestor(find::text(text), find::by_type::<Padding>())
            .evaluate(tester)
            .into_iter()
            .map(|element| {
                tester.widget(element, |padding: &Padding| {
                    padding.padding.resolve(Some(TextDirection::Ltr))
                })
            })
            .collect()
    }

    #[test]
    fn material3_has_sentence_case_labels() {
        test_widgets(|tester| {
            pump(
                tester,
                Stepper::new(vec![
                    sized_step("Step 1", 100.0, 100.0),
                    sized_step("Step 2", 100.0, 100.0),
                ])
                .on_step_tapped(Rc::new(|_app, _index| {})),
            );
            assert!(count(tester, find::text("Continue")) > 0);
            assert!(count(tester, find::text("Cancel")) > 0);
        });
    }

    #[test]
    fn stepper_tap_callback_test() {
        test_widgets(|tester| {
            let index = Rc::new(Cell::new(0));
            let tapped = Rc::clone(&index);
            pump(
                tester,
                Stepper::new(vec![
                    sized_step("Step 1", 100.0, 100.0),
                    sized_step("Step 2", 100.0, 100.0),
                ])
                .on_step_tapped(Rc::new(move |_app, i| tapped.set(i))),
            );
            tester.tap(find::text("Step 2"));
            assert_eq!(index.get(), 1);
        });
    }

    #[test]
    fn stepper_expansion_test() {
        test_widgets(|tester| {
            let steps = || {
                vec![
                    sized_step("Step 1", 100.0, 100.0),
                    sized_step("Step 2", 200.0, 200.0),
                ]
            };
            pump_centered(tester, Stepper::new(steps()));
            assert_eq!(stepper_height(tester), 332.0);

            pump_centered(tester, Stepper::new(steps()).current_step(1));
            tester.pump_by(Duration::from_millis(100));
            assert!(stepper_height(tester) > 332.0);
            tester.pump_by(Duration::from_millis(100));
            assert_eq!(stepper_height(tester), 432.0);
        });
    }

    #[test]
    fn stepper_horizontal_size_test() {
        test_widgets(|tester| {
            pump_centered(
                tester,
                Stepper::new(vec![sized_step("Step 1", 100.0, 100.0)])
                    .r#type(StepperType::Horizontal),
            );
            assert_eq!(stepper_height(tester), 600.0);
        });
    }

    #[test]
    fn stepper_visibility_test() {
        test_widgets(|tester| {
            let steps = || vec![text_step("Step 1", "A"), text_step("Step 2", "B")];
            pump(
                tester,
                Stepper::new(steps()).r#type(StepperType::Horizontal),
            );
            assert_eq!(count(tester, find::text("A")), 1);
            assert_eq!(count(tester, find::text("B")), 0);

            pump(
                tester,
                Stepper::new(steps())
                    .current_step(1)
                    .r#type(StepperType::Horizontal),
            );
            assert_eq!(count(tester, find::text("A")), 0);
            assert_eq!(count(tester, find::text("B")), 1);
        });
    }

    /// Dart's `Stepper button test`: taps the continue and cancel buttons labelled `continue_label`
    /// and `cancel_label` under `theme`.
    fn stepper_button_test(
        tester: &mut WidgetTester,
        theme: ThemeData,
        continue_label: &str,
        cancel_label: &str,
    ) {
        let continue_pressed = Rc::new(Cell::new(false));
        let cancel_pressed = Rc::new(Cell::new(false));
        let (continued, cancelled) = (Rc::clone(&continue_pressed), Rc::clone(&cancel_pressed));
        tester.pump_widget(material_app(
            theme,
            Material::new()
                .child(
                    Stepper::new(vec![
                        sized_step("Step 1", 100.0, 100.0),
                        sized_step("Step 2", 200.0, 200.0),
                    ])
                    .r#type(StepperType::Horizontal)
                    .on_step_continue(Listener::new(move |_app| continued.set(true)))
                    .on_step_cancel(Listener::new(move |_app| cancelled.set(true))),
                )
                .into_widget(),
        ));

        tester.tap(find::text(continue_label));
        tester.tap(find::text(cancel_label));

        assert!(continue_pressed.get());
        assert!(cancel_pressed.get());
    }

    #[test]
    fn material2_stepper_button_test() {
        test_widgets(|tester| {
            let theme = material2_theme(tester);
            stepper_button_test(tester, theme, "CONTINUE", "CANCEL");
        });
    }

    #[test]
    fn material3_stepper_button_test() {
        test_widgets(|tester| {
            let theme = light_theme(tester);
            stepper_button_test(tester, theme, "Continue", "Cancel");
        });
    }

    #[test]
    fn stepper_disabled_step_test() {
        test_widgets(|tester| {
            let index = Rc::new(Cell::new(0));
            let tapped = Rc::clone(&index);
            pump(
                tester,
                Stepper::new(vec![
                    sized_step("Step 1", 100.0, 100.0),
                    sized_step("Step 2", 100.0, 100.0).state(StepState::Disabled),
                ])
                .on_step_tapped(Rc::new(move |_app, i| tapped.set(i))),
            );
            tester.tap(find::text("Step 2"));
            assert_eq!(index.get(), 0);
        });
    }

    #[test]
    fn stepper_scroll_test() {
        test_widgets(|tester| {
            let steps = || {
                vec![
                    sized_step("Step 1", 100.0, 300.0),
                    sized_step("Step 2", 100.0, 300.0),
                    sized_step("Step 3", 100.0, 100.0),
                ]
            };
            pump(tester, Stepper::new(steps()));

            let scrollable_state = tester
                .element(find::by_type::<Scrollable>().first())
                .state_handle::<ScrollableState>(&tester.cell.borrow())
                .expect("a scrollable state");
            let pixels = |tester: &WidgetTester| {
                let app = tester.cell.borrow();
                scrollable_state.position(&app).pixels(&app)
            };
            assert_eq!(pixels(tester), 0.0);

            tester.tap(find::text("Step 3"));
            pump(tester, Stepper::new(steps()).current_step(2));

            tester.pump_by(Duration::from_millis(100));
            assert!(pixels(tester) > 0.0);
        });
    }

    #[test]
    fn stepper_index_test() {
        test_widgets(|tester| {
            pump_centered(
                tester,
                Stepper::new(vec![
                    sized_step("A", 100.0, 100.0).state(StepState::Complete),
                    sized_step("B", 100.0, 100.0),
                ]),
            );
            assert_eq!(count(tester, find::text("1")), 0);
            assert_eq!(count(tester, find::text("2")), 1);
        });
    }

    /// Dart's custom `controlsBuilder`: a continue and a cancel button labelled by `labels`,
    /// pressing the callbacks `details` carries, or nothing when `labels` gives no labels.
    fn custom_controls(
        labels: impl Fn(&ControlsDetails) -> Option<(String, String)> + 'static,
    ) -> ControlsWidgetBuilder {
        Rc::new(move |_app, _context, details| {
            let Some((continue_label, cancel_label)) = labels(&details) else {
                return Container::new().into_widget();
            };
            Container::new()
                .margin(EdgeInsets::only(0.0, 16.0, 0.0, 0.0))
                .child(SizedBox::new().height(48.0).child(Row::new().children(vec![
                            TextButton::new(
                                details.on_step_continue.clone(),
                                Some(Text::new(continue_label)),
                            )
                            .into_widget(),
                            Container::new()
                                .margin(EdgeInsetsDirectional::only(8.0, 0.0, 0.0, 0.0))
                                .child(TextButton::new(
                                    details.on_step_cancel.clone(),
                                    Some(Text::new(cancel_label)),
                                ))
                                .into_widget(),
                        ])))
                .into_widget()
        })
    }

    #[test]
    fn stepper_custom_controls_test() {
        test_widgets(|tester| {
            let continue_pressed = Rc::new(Cell::new(false));
            let canceled_pressed = Rc::new(Cell::new(false));
            let (set_continue, set_canceled) =
                (Rc::clone(&continue_pressed), Rc::clone(&canceled_pressed));
            pump_centered(
                tester,
                Stepper::new(vec![
                    sized_step("A", 100.0, 100.0).state(StepState::Complete),
                    sized_step("B", 100.0, 100.0),
                ])
                .controls_builder(custom_controls(|_details| {
                    Some(("Let us continue!".to_owned(), "Cancel This!".to_owned()))
                }))
                .on_step_cancel(Listener::new(move |_app| set_canceled.set(true)))
                .on_step_continue(Listener::new(move |_app| set_continue.set(true))),
            );

            // 2 because stepper creates a set of controls for each step
            assert_eq!(count(tester, find::text("Let us continue!")), 2);
            assert_eq!(count(tester, find::text("Cancel This!")), 2);

            tester.tap(find::text("Cancel This!").first());
            tester.pump_and_settle();
            tester.tap(find::text("Let us continue!").first());
            tester.pump_and_settle();

            assert!(canceled_pressed.get());
            assert!(continue_pressed.get());
        });
    }

    #[test]
    fn stepper_custom_indexed_controls_test() {
        test_widgets(|tester| {
            let current_step = Rc::new(Cell::new(0_i32));
            let builder = {
                let current_step = Rc::clone(&current_step);
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let (cancel_step, continue_step) =
                        (Rc::clone(&current_step), Rc::clone(&current_step));
                    let (cancel_set_state, continue_set_state) = (set_state.clone(), set_state);
                    Stepper::new(vec![
                        sized_step("A", 100.0, 100.0).state(StepState::Complete),
                        sized_step("C", 100.0, 100.0),
                    ])
                    .current_step(current_step.get() as usize)
                    // For the purposes of testing, only render something for the active step.
                    .controls_builder(custom_controls(|details| {
                        details.is_active().then(|| {
                            let step_index = details.step_index as i32;
                            (
                                format!("Continue to {}", step_index + 1),
                                format!("Return to {}", step_index - 1),
                            )
                        })
                    }))
                    .on_step_cancel(Listener::new(move |app| {
                        cancel_set_state(app, &mut |_app| cancel_step.set(cancel_step.get() - 1));
                    }))
                    .on_step_continue(Listener::new(move |app| {
                        continue_set_state(app, &mut |_app| {
                            continue_step.set(continue_step.get() + 1);
                        });
                    }))
                    .into_widget()
                })
            };
            let theme = light_theme(tester);
            tester.pump_widget(material_app(
                theme,
                Center::new()
                    .child(Material::new().child(builder))
                    .into_widget(),
            ));

            // Never mind that there is no Step -1 or Step 2 -- actual build method
            // implementations would make those checks.
            assert_eq!(count(tester, find::text("Return to -1")), 1);
            assert_eq!(count(tester, find::text("Continue to 1")), 1);
            assert_eq!(count(tester, find::text("Return to 0")), 0);
            assert_eq!(count(tester, find::text("Continue to 2")), 0);

            tester.tap(find::text("Continue to 1").first());
            tester.pump_and_settle();

            assert_eq!(count(tester, find::text("Return to -1")), 0);
            assert_eq!(count(tester, find::text("Continue to 1")), 0);
            assert_eq!(count(tester, find::text("Return to 0")), 1);
            assert_eq!(count(tester, find::text("Continue to 2")), 1);
        });
    }

    #[test]
    fn stepper_error_test() {
        test_widgets(|tester| {
            pump_centered(
                tester,
                Stepper::new(vec![sized_step("A", 100.0, 100.0).state(StepState::Error)]),
            );
            assert_eq!(count(tester, find::text("!")), 1);
        });
    }

    #[test]
    #[should_panic(
        expected = "Steppers must not be nested.\nThe material specification advises \
                               that one should avoid embedding steppers within steppers. \
                               https://material.io/archive/guidelines/components/steppers.html#steppers-usage"
    )]
    fn nested_stepper_error_test() {
        test_widgets(|tester| {
            pump(
                tester,
                Stepper::new(vec![
                    Step::new(
                        Text::new("Step 2"),
                        Stepper::new(vec![
                            text_step("Nested step 1", "A"),
                            text_step("Nested step 2", "A"),
                        ]),
                    ),
                    text_step("Step 1", "A"),
                ])
                .r#type(StepperType::Horizontal),
            );
        });
    }

    // https://github.com/flutter/flutter/issues/16920
    #[test]
    fn stepper_icons_size_test() {
        test_widgets(|tester| {
            pump(
                tester,
                Stepper::new(vec![
                    sized_step("A", 100.0, 100.0).state(StepState::Editing),
                    sized_step("B", 100.0, 100.0).state(StepState::Complete),
                ]),
            );
            assert_eq!(
                tester.get_size(find::by_icon(&Icons::edit())),
                Size::square(18.0)
            );
            assert_eq!(
                tester.get_size(find::by_icon(&Icons::check())),
                Size::square(18.0)
            );
        });
    }

    #[test]
    fn stepper_physics_scroll_error_test() {
        test_widgets(|tester| {
            let steps = (1..=10)
                .map(|i| {
                    Step::new(
                        Text::new(format!("Step {i}")),
                        Text::new(format!("Text {i}")),
                    )
                })
                .collect();
            let theme = light_theme(tester);
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(ListView::new().children(vec![
                        Stepper::new(steps).into_widget(),
                        Text::new("Text After Stepper").into_widget(),
                    ]))
                    .into_widget(),
            ));

            tester.fling(find::by_type::<Stepper>(), Offset::new(0.0, -100.0), 1000.0);
            tester.pump_and_settle();

            assert_eq!(count(tester, find::text("Text After Stepper")), 0);
        });
    }

    /// Dart's `Stepper can't be focused when disabled.`: asks the disabled step's focus node for
    /// focus and checks it did not take it.
    fn disabled_step_takes_focus(tester: &mut WidgetTester, r#type: StepperType) -> bool {
        pump(
            tester,
            Stepper::new(vec![
                text_step("Step 0", "Text 0").state(StepState::Disabled),
            ])
            .r#type(r#type),
        );
        tester.pump();

        let element = tester.element(find::text("Step 0"));
        let disabled_node = Focus::of(&mut tester.cell.borrow_mut(), element, true, false);
        disabled_node.request_focus(&mut tester.cell.borrow_mut(), None);
        tester.pump();
        disabled_node.has_primary_focus(&tester.cell.borrow())
    }

    #[test]
    fn vertical_stepper_cant_be_focused_when_disabled() {
        test_widgets(|tester| {
            assert!(!disabled_step_takes_focus(tester, StepperType::Vertical));
        });
    }

    #[test]
    fn horizontal_stepper_cant_be_focused_when_disabled() {
        test_widgets(|tester| {
            assert!(!disabled_step_takes_focus(tester, StepperType::Horizontal));
        });
    }

    /// Dart's `expect(tester.takeException(), isNull)` after a layout that may overflow: Waft
    /// reports no overflow exception, so the case reads each `RenderFlex`'s overflow.
    fn assert_no_flex_overflows(tester: &WidgetTester) {
        let flexes = tester.all::<RenderFlex>();
        let app = tester.cell.borrow();
        for flex in flexes {
            assert!(!flex.has_overflow(&app));
        }
    }

    const LONG_TEXT: &str = "A long long long long long long long long long long long long text";

    #[test]
    fn stepper_header_title_should_not_overflow() {
        test_widgets(|tester| {
            let theme = light_theme(tester);
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(ListView::new().children(vec![
                            Stepper::new(vec![Step::new(
                                Text::new(LONG_TEXT),
                                Text::new("Text content"),
                            )])
                            .into_widget(),
                        ]))
                    .into_widget(),
            ));
            assert_no_flex_overflows(tester);
        });
    }

    #[test]
    fn stepper_header_subtitle_should_not_overflow() {
        test_widgets(|tester| {
            let theme = light_theme(tester);
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(ListView::new().children(vec![
                            Stepper::new(vec![
                                text_step("Regular title", "Text content")
                                    .subtitle(Text::new(LONG_TEXT)),
                            ])
                            .into_widget(),
                        ]))
                    .into_widget(),
            ));
            assert_no_flex_overflows(tester);
        });
    }

    /// What a test reads of the `Material` under the `TextButton` labelled `label`: its color,
    /// its text style's color and its shape.
    struct ButtonMaterial {
        color: Option<Color>,
        text_color: Option<Color>,
        shape: Option<RoundedRectangleBorder>,
    }

    /// Dart's `buttonMaterial(label)`.
    fn button_material(tester: &WidgetTester, label: &str) -> ButtonMaterial {
        tester.widget(
            find::descendant(
                find::widget_with_text::<TextButton>(label),
                find::by_type::<Material>(),
            ),
            |material: &Material| ButtonMaterial {
                color: material.color,
                text_color: material
                    .text_style
                    .as_ref()
                    .and_then(|style| style.color.as_ref())
                    .map(|color| color.color()),
                shape: material.shape.as_ref().and_then(|shape: &ShapeBorderRef| {
                    shape
                        .as_any()
                        .downcast_ref::<RoundedRectangleBorder>()
                        .copied()
                }),
            },
        )
    }

    fn button_shape() -> RoundedRectangleBorder {
        RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::all(waft_embedder::Radius::circular(2.0)),
        )
    }

    /// Dart's `buildFrame(theme)` of the button style cases: a horizontal stepper of one step,
    /// with both buttons enabled when `enabled`.
    fn pump_button_frame(tester: &mut WidgetTester, theme: ThemeData, enabled: bool) {
        let mut stepper =
            Stepper::new(vec![sized_step("step1", 100.0, 100.0)]).r#type(StepperType::Horizontal);
        if enabled {
            stepper = stepper
                .on_step_cancel(Listener::new(|_app| {}))
                .on_step_continue(Listener::new(|_app| {}));
        }
        tester.pump_widget(material_app(
            theme,
            Material::new().child(stepper).into_widget(),
        ));
    }

    fn assert_rect_more_or_less_equals(actual: Rect, expected: Rect) {
        assert_more_or_less_equals_within(actual.left, expected.left, 0.001);
        assert_more_or_less_equals_within(actual.top, expected.top, 0.001);
        assert_more_or_less_equals_within(actual.right, expected.right, 0.001);
        assert_more_or_less_equals_within(actual.bottom, expected.bottom, 0.001);
    }

    #[test]
    fn material2_stepper_enabled_button_styles() {
        test_widgets(|tester| {
            let theme_light = material2_theme(tester);
            pump_button_frame(tester, theme_light, true);

            const CONTINUE_STR: &str = "CONTINUE";
            const CANCEL_STR: &str = "CANCEL";
            let continue_button_rect = Rect::from_ltrb(24.0, 212.0, 168.0, 260.0);
            let cancel_button_rect = Rect::from_ltrb(176.0, 212.0, 292.0, 260.0);
            let continue_material = button_material(tester, CONTINUE_STR);
            assert_eq!(continue_material.color, Some(Color::new(0xff2196f3)));
            assert_eq!(continue_material.text_color, Some(Color::new(0xffffffff)));
            assert_eq!(continue_material.shape, Some(button_shape()));
            assert_eq!(
                tester.get_rect(find::widget_with_text::<TextButton>(CONTINUE_STR)),
                continue_button_rect
            );

            let cancel_material = button_material(tester, CANCEL_STR);
            assert_eq!(cancel_material.color, Some(Color::new(0)));
            assert_eq!(cancel_material.text_color, Some(Color::new(0x8a000000)));
            assert_eq!(cancel_material.shape, Some(button_shape()));
            assert_eq!(
                tester.get_rect(find::widget_with_text::<TextButton>(CANCEL_STR)),
                cancel_button_rect
            );

            let theme_dark = ThemeData::dark(&tester.cell.borrow(), Some(false));
            pump_button_frame(tester, theme_dark, true);
            // Complete the theme animation.
            tester.pump_and_settle();

            let continue_material = button_material(tester, CONTINUE_STR);
            assert_eq!(continue_material.color, Some(Color::new(0)));
            assert_eq!(continue_material.text_color, Some(Color::new(0xffffffff)));
            assert_eq!(continue_material.shape, Some(button_shape()));
            assert_eq!(
                tester.get_rect(find::widget_with_text::<TextButton>(CONTINUE_STR)),
                continue_button_rect
            );

            let cancel_material = button_material(tester, CANCEL_STR);
            assert_eq!(cancel_material.color, Some(Color::new(0)));
            assert_eq!(cancel_material.text_color, Some(Color::new(0xb3ffffff)));
            assert_eq!(cancel_material.shape, Some(button_shape()));
            assert_eq!(
                tester.get_rect(find::widget_with_text::<TextButton>(CANCEL_STR)),
                cancel_button_rect
            );
        });
    }

    #[test]
    fn material3_stepper_enabled_button_styles() {
        test_widgets(|tester| {
            let theme_light = light_theme(tester);
            let light_primary = theme_light.color_scheme.primary();
            pump_button_frame(tester, theme_light, true);

            const CONTINUE_STR: &str = "Continue";
            const CANCEL_STR: &str = "Cancel";
            let continue_button_rect = Rect::from_ltrb(24.0, 212.0, 168.8, 260.0);
            let cancel_button_rect = Rect::from_ltrb(176.8, 212.0, 293.4, 260.0);
            let continue_material = button_material(tester, CONTINUE_STR);
            assert_eq!(continue_material.color, Some(light_primary));
            assert_eq!(continue_material.text_color, Some(Color::new(0xffffffff)));
            assert_eq!(continue_material.shape, Some(button_shape()));
            assert_rect_more_or_less_equals(
                tester.get_rect(find::widget_with_text::<TextButton>(CONTINUE_STR)),
                continue_button_rect,
            );

            let cancel_material = button_material(tester, CANCEL_STR);
            assert_eq!(cancel_material.color, Some(Color::new(0)));
            assert_eq!(cancel_material.text_color, Some(Color::new(0x8a000000)));
            assert_eq!(cancel_material.shape, Some(button_shape()));
            assert_rect_more_or_less_equals(
                tester.get_rect(find::widget_with_text::<TextButton>(CANCEL_STR)),
                cancel_button_rect,
            );

            let theme_dark = ThemeData::dark(&tester.cell.borrow(), None);
            let dark_on_surface = theme_dark.color_scheme.on_surface();
            pump_button_frame(tester, theme_dark, true);
            // Complete the theme animation.
            tester.pump_and_settle();

            let continue_material = button_material(tester, CONTINUE_STR);
            assert_eq!(continue_material.color, Some(Color::new(0)));
            assert_eq!(continue_material.text_color, Some(dark_on_surface));
            assert_eq!(continue_material.shape, Some(button_shape()));
            assert_rect_more_or_less_equals(
                tester.get_rect(find::widget_with_text::<TextButton>(CONTINUE_STR)),
                continue_button_rect,
            );

            let cancel_material = button_material(tester, CANCEL_STR);
            assert_eq!(cancel_material.color, Some(Color::new(0)));
            assert_eq!(cancel_material.text_color, Some(Color::new(0xb3ffffff)));
            assert_eq!(cancel_material.shape, Some(button_shape()));
            assert_rect_more_or_less_equals(
                tester.get_rect(find::widget_with_text::<TextButton>(CANCEL_STR)),
                cancel_button_rect,
            );
        });
    }

    /// Dart's `Stepper disabled button styles`: both buttons are transparent with
    /// `disabled_text` text, under `theme`.
    fn assert_disabled_button_styles(
        tester: &mut WidgetTester,
        theme: ThemeData,
        continue_label: &str,
        cancel_label: &str,
        disabled_text: Color,
    ) {
        pump_button_frame(tester, theme, false);
        // Complete the theme animation.
        tester.pump_and_settle();
        for label in [continue_label, cancel_label] {
            let material = button_material(tester, label);
            assert_eq!(material.color, Some(Color::new(0)), "{label}");
            assert_eq!(material.text_color, Some(disabled_text), "{label}");
        }
    }

    #[test]
    fn material2_stepper_disabled_button_styles() {
        test_widgets(|tester| {
            let theme_light = material2_theme(tester);
            assert_disabled_button_styles(
                tester,
                theme_light,
                "CONTINUE",
                "CANCEL",
                Color::new(0x61000000),
            );

            let theme_dark = ThemeData::dark(&tester.cell.borrow(), Some(false));
            assert_disabled_button_styles(
                tester,
                theme_dark,
                "CONTINUE",
                "CANCEL",
                Color::new(0x61ffffff),
            );
        });
    }

    #[test]
    #[expect(
        deprecated,
        reason = "Dart's case compares against `Color.withOpacity`"
    )]
    fn material3_stepper_disabled_button_styles() {
        test_widgets(|tester| {
            let theme_light = light_theme(tester);
            let colors_light = theme_light.color_scheme.clone();
            assert_disabled_button_styles(
                tester,
                theme_light,
                "Continue",
                "Cancel",
                colors_light.on_surface().with_opacity(0.38),
            );

            let theme_dark = ThemeData::dark(&tester.cell.borrow(), None);
            let colors_dark = theme_dark.color_scheme.clone();
            assert_disabled_button_styles(
                tester,
                theme_dark,
                "Continue",
                "Cancel",
                colors_dark.on_surface().with_opacity(0.38),
            );
        });
    }

    #[test]
    fn vertical_and_horizontal_stepper_physics_test() {
        test_widgets(|tester| {
            let physics: waft_widgets::ScrollPhysicsRef =
                Rc::new(NeverScrollableScrollPhysics::new());

            for r#type in [StepperType::Vertical, StepperType::Horizontal] {
                pump(
                    tester,
                    Stepper::new(vec![sized_step("Step 1", 100.0, 100.0)])
                        .physics(physics.clone())
                        .r#type(r#type),
                );

                let list_view_physics = stepper_list_view(tester, |list_view| {
                    list_view.scroll_view_data().physics.clone()
                })
                .expect("the list view has physics");
                assert!(Rc::ptr_eq(&list_view_physics, &physics), "{type:?}");
            }
        });
    }

    #[test]
    fn scroll_controller_is_passed_to_the_stepper_listview() {
        test_widgets(|tester| {
            let controller = ScrollController::default(&mut tester.cell.borrow_mut());
            for r#type in [StepperType::Vertical, StepperType::Horizontal] {
                pump(
                    tester,
                    Stepper::new(vec![sized_step("Step 1", 100.0, 100.0)])
                        .controller(controller.erase())
                        .r#type(r#type),
                );

                let list_view_controller = stepper_list_view(tester, |list_view| {
                    list_view.scroll_view_data().controller.clone()
                })
                .expect("the list view has a controller");
                assert_eq!(
                    *list_view_controller,
                    controller.as_controller(),
                    "{type:?}"
                );
            }
            controller.dispose(&mut tester.cell.borrow_mut());
        });
    }

    // Regression test for https://github.com/flutter/flutter/pull/77732
    #[test]
    #[expect(
        deprecated,
        reason = "Dart's case compares against `Color.withOpacity` and `ColorScheme.background`"
    )]
    fn stepper_horizontal_size_test_circle_colors() {
        test_widgets(|tester| {
            let build_frame =
                |tester: &mut WidgetTester, is_active: bool, brightness: Brightness| {
                    let theme = match brightness {
                        Brightness::Dark => ThemeData::dark(&tester.cell.borrow(), None),
                        Brightness::Light => light_theme(tester),
                    };
                    pump_in_scaffold(
                        tester,
                        theme,
                        Stepper::new(vec![text_step("step", "content").is_active(is_active)])
                            .r#type(StepperType::Horizontal)
                            .into_widget(),
                    );
                };

            // Light theme
            let light = light_theme(tester).color_scheme.clone();
            build_frame(tester, true, Brightness::Light);
            assert_eq!(circle_color(tester, "1"), Some(light.primary()));
            build_frame(tester, false, Brightness::Light);
            tester.pump_and_settle();
            assert_eq!(
                circle_color(tester, "1"),
                Some(light.on_surface().with_opacity(0.38))
            );

            // Dark theme
            let dark = ThemeData::dark(&tester.cell.borrow(), None)
                .color_scheme
                .clone();
            build_frame(tester, true, Brightness::Dark);
            tester.pump_and_settle();
            assert_eq!(circle_color(tester, "1"), Some(dark.secondary()));
            build_frame(tester, false, Brightness::Dark);
            tester.pump_and_settle();
            assert_eq!(circle_color(tester, "1"), Some(dark.background()));
        });
    }

    /// Dart's `tester.firstWidget<Material>(find.descendant(of: find.byType(Stepper),
    /// matching: find.byType(Material))).elevation` for a 200 by 75 horizontal stepper.
    fn horizontal_stepper_elevation(tester: &mut WidgetTester, stepper: Stepper) -> f64 {
        let theme = light_theme(tester);
        tester.pump_widget(material_app(
            theme,
            Material::new()
                .child(
                    SizedBox::new()
                        .width(200.0)
                        .height(75.0)
                        .child(stepper.r#type(StepperType::Horizontal)),
                )
                .into_widget(),
        ));
        tester.widget(
            find::descendant(find::by_type::<Stepper>(), find::by_type::<Material>()).first(),
            |material: &Material| material.elevation,
        )
    }

    #[test]
    fn stepper_custom_elevation() {
        test_widgets(|tester| {
            const ELEVATION: f64 = 4.0;
            let stepper =
                Stepper::new(vec![text_step("Regular title", "Text content")]).elevation(ELEVATION);
            assert_eq!(horizontal_stepper_elevation(tester, stepper), ELEVATION);
        });
    }

    #[test]
    fn stepper_with_default_elevation() {
        test_widgets(|tester| {
            let stepper = Stepper::new(vec![text_step("Regular title", "Text content")]);
            assert_eq!(horizontal_stepper_elevation(tester, stepper), 2.0);
        });
    }

    /// Dart's `_TappableColorWidget`: a 50 by 50 box keyed `tap-me` that turns from
    /// `untapped_color` to `tapped_color` when tapped.
    struct TappableColorWidget {
        key: KeyRef,
        tapped_color: Color,
        untapped_color: Color,
    }

    impl fmt::Debug for TappableColorWidget {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("TappableColorWidget")
                .finish_non_exhaustive()
        }
    }

    struct TappableColorWidgetState {
        state: StateData<TappableColorWidget>,
        color: Option<Color>,
    }

    impl fmt::Debug for TappableColorWidgetState {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("TappableColorWidgetState")
                .finish_non_exhaustive()
        }
    }

    impl StatefulWidget for TappableColorWidget {
        type State = TappableColorWidgetState;

        fn key(&self) -> Option<&KeyRef> {
            Some(&self.key)
        }

        fn create_state(&self) -> TappableColorWidgetState {
            TappableColorWidgetState {
                state: StateData::new(),
                color: None,
            }
        }
    }

    impl State for TappableColorWidgetState {
        type Widget = TappableColorWidget;
        waft_widgets::state_accessors!();

        fn init_state(self: Handle<Self>, app: &mut App) {
            app.get_mut(self).color = Some(self.widget(app).untapped_color);
        }

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            let color = app.get(self).color.expect("set in init_state");
            GestureDetector::new()
                .on_tap(Listener::new(move |app| {
                    let tapped_color = self.widget(app).tapped_color;
                    self.set_state(app, |state| state.color = Some(tapped_color));
                }))
                .child(
                    Container::new()
                        .key(ValueKey::new("tap-me".to_owned()).into())
                        .height(50.0)
                        .width(50.0)
                        .color(color),
                )
                .into_widget()
        }
    }

    #[test]
    fn stepper_horizontal_preserves_state() {
        test_widgets(|tester| {
            let untapped_color = Colors::BLUE.primary();
            let tapped_color = Colors::RED.primary();
            let index = Rc::new(Cell::new(0_usize));
            let builder = {
                let index = Rc::clone(&index);
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let tapped = Rc::clone(&index);
                    Stepper::new(vec![
                        Step::new(
                            Text::new("Step 1"),
                            TappableColorWidget {
                                key: ValueKey::new("tappable-color".to_owned()).into(),
                                tapped_color,
                                untapped_color,
                            },
                        ),
                        text_step("Step 2", "Step 2 Content"),
                    ])
                    .on_step_tapped(Rc::new(move |app, i| {
                        set_state(app, &mut |_app| tapped.set(i));
                    }))
                    .current_step(index.get())
                    .r#type(StepperType::Horizontal)
                    .into_widget()
                })
            };
            let theme = light_theme(tester);
            pump_in_scaffold(tester, theme, builder.into_widget());

            // Set up a getter to examine the MacGuffin's color
            let get_color = |tester: &WidgetTester| {
                let colored_box = find::descendant(
                    find::by_key(ValueKey::new("tappable-color".to_owned())),
                    find::by_type::<ColoredBox>(),
                )
                .evaluate(tester)[0];
                colored_box_color(tester, colored_box)
            };

            // We are on step 1
            assert_eq!(count(tester, find::text("Step 2 Content")), 0);
            assert_eq!(get_color(tester), untapped_color);

            tester.tap(find::by_key(ValueKey::new("tap-me".to_owned())));
            tester.pump_and_settle();
            assert_eq!(get_color(tester), tapped_color);

            // Now flip to step 2
            tester.tap(find::text("Step 2"));
            tester.pump_and_settle();

            // Confirm that we did in fact flip to step 2
            assert_eq!(count(tester, find::text("Step 2 Content")), 1);

            // Now go back to step 1
            tester.tap(find::text("Step 1"));
            tester.pump_and_settle();

            // Confirm that we flipped back to step 1
            assert_eq!(count(tester, find::text("Step 2 Content")), 0);

            // The color should still be `tappedColor`
            assert_eq!(get_color(tester), tapped_color);
        });
    }

    #[test]
    fn stepper_with_alternative_label() {
        test_widgets(|tester| {
            let index = Rc::new(Cell::new(0_usize));
            let styles: Rc<RefCell<Option<(TextStyle, TextStyle, TextStyle)>>> =
                Rc::new(RefCell::new(None));
            let builder = {
                let index = Rc::clone(&index);
                let styles = Rc::clone(&styles);
                StatefulBuilder::new(move |app, context, set_state| {
                    let text_theme = Theme::of(app, context).text_theme.clone();
                    let body_large = text_theme.body_large.clone().expect("a body large style");
                    let body_medium = text_theme.body_medium.clone().expect("a body medium style");
                    let body_small = text_theme.body_small.clone().expect("a body small style");
                    *styles.borrow_mut() =
                        Some((body_large.clone(), body_medium.clone(), body_small.clone()));
                    let tapped = Rc::clone(&index);
                    Stepper::new(vec![
                        text_step("Title 1", "Content 1")
                            .label(Text::new("Label 1").style(body_small)),
                        text_step("Title 2", "Content 2")
                            .label(Text::new("Label 2").style(body_large)),
                        text_step("Title 3", "Content 3")
                            .label(Text::new("Label 3").style(body_medium)),
                    ])
                    .r#type(StepperType::Horizontal)
                    .current_step(index.get())
                    .on_step_tapped(Rc::new(move |app, i| {
                        set_state(app, &mut |_app| tapped.set(i));
                    }))
                    .into_widget()
                })
            };
            let theme = light_theme(tester);
            tester.pump_widget(material_app(
                theme,
                Material::new().child(builder).into_widget(),
            ));
            let (body_large_style, body_medium_style, body_small_style) =
                styles.borrow().clone().expect("the builder ran");
            let label_style = |tester: &WidgetTester, label: &str| {
                tester.widget(find::text(label), |text: &Text| text.style.clone())
            };

            // Check Styles of Label Text Widgets before tapping steps
            assert_eq!(
                Some(body_small_style.clone()),
                label_style(tester, "Label 1")
            );
            assert_eq!(
                Some(body_medium_style.clone()),
                label_style(tester, "Label 3")
            );

            // Tap to Step1 Label then, `index` become 0
            tester.tap(find::text("Label 1"));
            assert_eq!(index.get(), 0);

            // Check Styles of Selected Label Text Widgets and Another Label Text Widget
            assert_eq!(
                Some(body_small_style.clone()),
                label_style(tester, &format!("Label {}", index.get() + 1))
            );
            assert_eq!(
                Some(body_large_style.clone()),
                label_style(tester, &format!("Label {}", index.get() + 2))
            );

            // Tap to Step2 Label then, `index` become 1
            tester.tap(find::text("Label 2"));
            assert_eq!(index.get(), 1);

            // Check Styles of Selected Label Text Widgets and Another Label Text Widget
            assert_eq!(
                Some(body_large_style),
                label_style(tester, &format!("Label {}", index.get() + 1))
            );
            assert_eq!(
                Some(body_medium_style),
                label_style(tester, &format!("Label {}", index.get() + 2))
            );
        });
    }

    #[test]
    fn stepper_connector_style() {
        test_widgets(|tester| {
            let selected_color = Colors::BLACK;
            let disabled_color = Colors::WHITE;
            let index = Rc::new(Cell::new(0_i32));
            let builder = {
                let index = Rc::clone(&index);
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let tapped = Rc::clone(&index);
                    Stepper::new(vec![
                        text_step("step1", "step1 content").is_active(index.get() >= 0),
                        text_step("step2", "step2 content").is_active(index.get() >= 1),
                    ])
                    .r#type(StepperType::Horizontal)
                    .connector_color(<dyn WidgetStateProperty<Color>>::resolve_with(
                        move |states| {
                            if states.contains(&WidgetState::Selected) {
                                selected_color
                            } else {
                                disabled_color
                            }
                        },
                    ))
                    .on_step_tapped(Rc::new(move |app, i| {
                        set_state(app, &mut |_app| tapped.set(i as i32));
                    }))
                    .current_step(index.get() as usize)
                    .into_widget()
                })
            };
            let theme = light_theme(tester);
            pump_in_scaffold(tester, theme, builder.into_widget());

            let line_color = |tester: &WidgetTester| {
                let line = tester.element(find::descendant(
                    find::by_type::<Stepper>(),
                    find::by_type::<ColoredBox>(),
                ));
                colored_box_color(tester, line)
            };

            // Step 1
            // check if I'm in step 1
            assert_eq!(count(tester, find::text("step1 content")), 1);
            assert_eq!(count(tester, find::text("step2 content")), 0);

            assert_eq!(circle_color(tester, "1"), Some(selected_color));
            assert_eq!(circle_color(tester, "2"), Some(disabled_color));
            // in two steps case there will be single line
            assert_eq!(line_color(tester), selected_color);

            // now hitting step two
            tester.tap(find::text("step2"));
            tester.pump_and_settle();

            // check if I'm in step 1
            assert_eq!(count(tester, find::text("step1 content")), 0);
            assert_eq!(count(tester, find::text("step2 content")), 1);

            assert_eq!(circle_color(tester, "1"), Some(selected_color));
            assert_eq!(circle_color(tester, "2"), Some(selected_color));

            assert_eq!(line_color(tester), selected_color);
        });
    }

    #[test]
    fn stepper_step_icon_builder_test() {
        test_widgets(|tester| {
            let logo_key: KeyRef = ValueKey::new("logo".to_owned()).into();
            let builder: StepIconBuilder = {
                let logo_key = logo_key.clone();
                Rc::new(move |_index, state| {
                    // Dart returns a `FlutterLogo(size: 18)`; any widget of that size will do.
                    (state == StepState::Complete).then(|| {
                        SizedBox::square(Some(18.0))
                            .key(logo_key.clone())
                            .into_widget()
                    })
                })
            };
            pump(
                tester,
                Stepper::new(vec![
                    sized_step("A", 100.0, 100.0).state(StepState::Complete),
                    sized_step("B", 100.0, 100.0).state(StepState::Editing),
                    sized_step("C", 100.0, 100.0).state(StepState::Error),
                ])
                .step_icon_builder(builder),
            );

            // Finds the overridden widget for StepState.complete
            assert_eq!(count(tester, find::by_key(logo_key)), 1);

            // StepState.editing and StepState.error should have a default icon
            assert_eq!(count(tester, find::by_icon(&Icons::edit())), 1);
            assert_eq!(count(tester, find::text("!")), 1);
        });
    }

    #[test]
    fn step_style_test() {
        test_widgets(|tester| {
            let border = Border::all(Colors::BLACK, 1.0, BorderStyle::Solid, -1.0);
            let gradient = GradientRef::new(LinearGradient::new(vec![
                Colors::RED.primary(),
                Colors::BLUE.primary(),
            ]));
            let step_style = StepStyle::new()
                .with_color(Colors::WHITE)
                .with_error_color(Colors::ORANGE.primary())
                .with_connector_color(Colors::RED.primary())
                .with_connector_thickness(2.0)
                .with_border(border)
                .with_gradient(gradient.clone())
                .with_index_style(TextStyle::new().color(Colors::BLACK));

            pump(
                tester,
                Stepper::new(vec![
                    text_step("Regular title", "Text content").step_style(step_style.clone()),
                ]),
            );

            let style = tester.widget(find::by_type::<Stepper>(), |stepper: &Stepper| {
                stepper.steps[0].step_style.clone()
            });
            assert_eq!(style.as_ref(), Some(&step_style));

            // copyWith
            let new_style = step_style
                .copy_with()
                .with_color(Colors::BLACK)
                .with_error_color(Colors::RED.primary())
                .with_connector_color(Colors::BLUE.primary())
                .with_connector_thickness(3.0)
                .with_border(border)
                .with_gradient(gradient)
                .with_index_style(TextStyle::new().color(Colors::BLACK));

            assert_eq!(new_style.color, Some(Colors::BLACK));
            assert_eq!(new_style.error_color, Some(Colors::RED.primary()));
            assert_eq!(new_style.connector_color, Some(Colors::BLUE.primary()));
            assert_eq!(new_style.connector_thickness, Some(3.0));
            assert_eq!(new_style.border, step_style.border);
            assert_eq!(new_style.gradient, step_style.gradient);
            assert_eq!(new_style.index_style, step_style.index_style);

            // merge
            let merged_style = step_style.merge(Some(&new_style));

            assert_eq!(merged_style.color, Some(Colors::BLACK));
            assert_eq!(merged_style.error_color, Some(Colors::RED.primary()));
            assert_eq!(merged_style.connector_color, Some(Colors::BLUE.primary()));
            assert_eq!(merged_style.connector_thickness, Some(3.0));
            assert_eq!(merged_style.border, step_style.border);
            assert_eq!(merged_style.gradient, step_style.gradient);
            assert_eq!(merged_style.index_style, step_style.index_style);
        });
    }

    #[test]
    fn a_step_style_color_fills_the_circle() {
        test_widgets(|tester| {
            let color = Color::new(0xFF123456);
            pump(
                tester,
                Stepper::new(vec![
                    text_step("A", "A").step_style(StepStyle::new().with_color(color)),
                ]),
            );
            assert_eq!(circle_color(tester, "1"), Some(color));
        });
    }

    // This is a regression test for https://github.com/flutter/flutter/issues/144376.
    #[test]
    fn vertical_stepper_does_not_draw_connector_on_the_last_step() {
        test_widgets(|tester| {
            let theme = light_theme(tester);
            pump_in_scaffold(
                tester,
                theme,
                Stepper::new(vec![
                    text_step("step1", "step1 content"),
                    text_step("step2", "step2 content"),
                ])
                .current_step(1)
                .into_widget(),
            );

            let last_connector_width = tester.widget(
                find::descendant(
                    find::by_type::<PositionedDirectional>(),
                    find::by_type::<Center>(),
                )
                .last(),
                |center: &Center| {
                    let child = center.child.as_ref().expect("the connector's box");
                    downcast_widget::<SizedBox>(&**child)
                        .expect("the connector is a SizedBox")
                        .width
                },
            );

            assert_eq!(last_connector_width, Some(0.0));
        });
    }

    /// Dart's `getContentClipRect().clipBehavior`: the `ClipRect` nearest above `step1 content`.
    fn content_clip_behavior(tester: &WidgetTester) -> Clip {
        tester.widget(
            find::ancestor(find::text("step1 content"), find::by_type::<ClipRect>()).first(),
            |clip_rect: &ClipRect| clip_rect.clip_behavior,
        )
    }

    /// Dart's `buildStepper` of the clip behavior cases.
    fn pump_two_step_stepper(tester: &mut WidgetTester, stepper: Stepper) {
        let theme = light_theme(tester);
        pump_in_scaffold(tester, theme, stepper.into_widget());
    }

    fn two_steps() -> Vec<Step> {
        vec![
            text_step("step1", "step1 content"),
            text_step("step2", "step2 content"),
        ]
    }

    // This is a regression test for https://github.com/flutter/flutter/issues/66007.
    #[test]
    fn default_stepper_clip_behavior() {
        test_widgets(|tester| {
            for r#type in [StepperType::Vertical, StepperType::Horizontal] {
                pump_two_step_stepper(tester, Stepper::new(two_steps()).r#type(r#type));
                assert_eq!(content_clip_behavior(tester), Clip::None, "{type:?}");
            }
        });
    }

    // This is a regression test for https://github.com/flutter/flutter/issues/66007.
    #[test]
    fn stepper_steps_can_be_clipped() {
        test_widgets(|tester| {
            for r#type in [StepperType::Vertical, StepperType::Horizontal] {
                pump_two_step_stepper(
                    tester,
                    Stepper::new(two_steps())
                        .clip_behavior(Clip::HardEdge)
                        .r#type(r#type),
                );
                assert_eq!(content_clip_behavior(tester), Clip::HardEdge, "{type:?}");
            }
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/160156.
    #[test]
    fn vertical_stepper_border_displays_correctly() {
        test_widgets(|tester| {
            let index = Rc::new(Cell::new(0_usize));
            let connector_color = Color::new(0xff00ffff);
            let builder = {
                let index = Rc::clone(&index);
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let tapped = Rc::clone(&index);
                    Stepper::new(two_steps())
                        .current_step(index.get())
                        .connector_color(Rc::new(WidgetStatePropertyAll::new(connector_color)))
                        .on_step_tapped(Rc::new(move |app, value| {
                            set_state(app, &mut |_app| tapped.set(value));
                        }))
                        .into_widget()
                })
            };
            let theme = light_theme(tester);
            pump_in_scaffold(tester, theme, builder.into_widget());

            let find_connector = || {
                find::descendant(
                    find::by_type::<Stepper>(),
                    find::descendant(
                        find::by_type::<PositionedDirectional>(),
                        find::by_element_predicate("the connector", move |app, element| {
                            let is_connector_box =
                                downcast_widget::<ColoredBox>(&**element.widget(app)).is_some_and(
                                    |colored_box| colored_box.color.color() == connector_color,
                                );
                            let size = element.size(app);
                            is_connector_box
                                && size
                                    .is_some_and(|size| size.width() == 1.0 && size.height() > 0.0)
                        }),
                    ),
                )
            };

            let verify_connector = |tester: &WidgetTester| {
                assert_eq!(count(tester, find_connector()), 1);
                let render = tester
                    .render_box_of(tester.element(find_connector()))
                    .as_object();
                let ops = tester.painted_ops_deep(render);
                assert!(paints(&ops, &[PaintCall::rect(Some(connector_color))]).is_ok());
            };

            verify_connector(tester);

            tester.tap(find::text("step2"));

            const CHECK_COUNT: u32 = 5;
            let duration = K_THEME_ANIMATION_DURATION / (CHECK_COUNT + 1);

            for _ in 0..CHECK_COUNT {
                tester.pump_by(duration);
                verify_connector(tester);
            }
        });
    }

    #[test]
    fn vertical_stepper_active_step_has_fully_colored_connector_line() {
        test_widgets(|tester| {
            let active_color = Color::new(0xFF2196F3);
            let inactive_color = Color::new(0xFF9E9E9E);
            let theme = light_theme(tester);
            pump_in_scaffold(
                tester,
                theme,
                Stepper::new(vec![
                    text_step("step1", "step1 content").is_active(true),
                    text_step("step2", "step2 content"),
                ])
                .controls_builder(Rc::new(|_app, _context, _details| {
                    SizedBox::shrink().into_widget()
                }))
                .connector_thickness(3.0)
                .connector_color(<dyn WidgetStateProperty<Color>>::resolve_with(
                    move |states| {
                        if states.contains(&WidgetState::Selected) {
                            active_color
                        } else {
                            inactive_color
                        }
                    },
                ))
                .into_widget(),
            );

            let connector_lines =
                find::by_widget_predicate("a 3-wide connector line", |_app, widget| {
                    downcast_widget::<ColoredBox>(widget).is_some_and(|colored_box| {
                        colored_box
                            .child
                            .as_ref()
                            .and_then(|child| downcast_widget::<SizedBox>(&**child))
                            .is_some_and(|sized_box| sized_box.width == Some(3.0))
                    })
                });

            let lines = connector_lines.evaluate(tester);
            assert!(!lines.is_empty());

            let colors: Vec<Color> = lines
                .into_iter()
                .map(|line| colored_box_color(tester, line))
                .collect();
            // Both top and bottom box should be colored.
            assert_eq!(
                colors
                    .iter()
                    .filter(|color| **color == active_color)
                    .count(),
                2
            );
            assert_eq!(colors.first(), Some(&active_color));
            assert_eq!(colors.last(), Some(&active_color));
        });
    }

    #[test]
    fn stepper_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            for r#type in [StepperType::Vertical, StepperType::Horizontal] {
                let theme = light_theme(tester);
                pump_in_scaffold(
                    tester,
                    theme,
                    SizedBox::shrink()
                        .child(
                            Stepper::new(vec![text_step("X", "X"), text_step("Y", "Y")])
                                .r#type(r#type),
                        )
                        .into_widget(),
                );
                assert_eq!(tester.get_size(find::by_type::<Stepper>()), Size::ZERO);
            }
        });
    }

    /// Dart's `Stepper custom headerPadding …`: how far the title moves left and down when the
    /// header padding becomes `custom_padding`.
    fn title_shift(
        tester: &mut WidgetTester,
        r#type: StepperType,
        custom_padding: EdgeInsets,
    ) -> (Rect, Rect) {
        let step = || vec![text_step("Step 1", "Content 1")];
        pump(tester, Stepper::new(step()).r#type(r#type));
        let default_title_rect = tester.get_rect(find::text("Step 1"));

        pump(
            tester,
            Stepper::new(step())
                .r#type(r#type)
                .header_padding(custom_padding),
        );
        let custom_title_rect = tester.get_rect(find::text("Step 1"));
        (default_title_rect, custom_title_rect)
    }

    #[test]
    fn stepper_custom_header_padding_for_vertical_stepper() {
        test_widgets(|tester| {
            // Custom padding: horizontal 12.0 (reduces left by 12.0), vertical 8.0 (increases top
            // by 8.0).
            let (default_title_rect, custom_title_rect) = title_shift(
                tester,
                StepperType::Vertical,
                EdgeInsets::symmetric(8.0, 12.0),
            );

            assert!(custom_title_rect.left < default_title_rect.left);
            assert_more_or_less_equals(default_title_rect.left - custom_title_rect.left, 12.0);
            assert_eq!(default_title_rect.top, 24.0);
            assert!(custom_title_rect.top > default_title_rect.top);
            assert_eq!(custom_title_rect.top, 32.0);
            assert_more_or_less_equals(custom_title_rect.top - default_title_rect.top, 8.0);
        });
    }

    #[test]
    fn stepper_custom_header_padding_for_horizontal_stepper() {
        test_widgets(|tester| {
            // Custom padding: horizontal 16.0 (reduces left by 8.0 from default 24.0), vertical 8.0
            // (increases top by 8.0).
            let (default_title_rect, custom_title_rect) = title_shift(
                tester,
                StepperType::Horizontal,
                EdgeInsets::symmetric(8.0, 16.0),
            );

            assert!(custom_title_rect.left < default_title_rect.left);
            assert_more_or_less_equals(default_title_rect.left - custom_title_rect.left, 8.0);
            assert_eq!(default_title_rect.top, 24.0);
            assert!(custom_title_rect.top > default_title_rect.top);
            assert_eq!(custom_title_rect.top, 32.0);
            assert_more_or_less_equals(custom_title_rect.top - default_title_rect.top, 8.0);
        });
    }

    #[test]
    fn stepper_custom_content_padding_for_horizontal_stepper() {
        test_widgets(|tester| {
            let custom_padding: EdgeInsetsGeometry = EdgeInsets::all(16.0).into();
            pump(
                tester,
                Stepper::new(vec![text_step("Step 1", "Content 1")])
                    .r#type(StepperType::Horizontal)
                    .content_padding(custom_padding),
            );

            // The content is in a ListView, verify its padding
            let padding =
                stepper_list_view(tester, |list_view| list_view.box_scroll_view_data().padding);
            assert_eq!(padding, Some(custom_padding));
        });
    }

    #[test]
    fn stepper_custom_content_padding_for_vertical_stepper() {
        test_widgets(|tester| {
            pump(
                tester,
                Stepper::new(vec![text_step("Step 1", "Content 1")])
                    .content_padding(EdgeInsetsDirectional::only(48.0, 0.0, 16.0, 16.0)),
            );

            // Find the Padding widget that wraps the step content and verify the effective
            // padding.
            let expected = EdgeInsets::only(48.0, 0.0, 16.0, 16.0);
            assert!(paddings_above(tester, "Content 1").contains(&expected));
        });
    }

    #[test]
    fn stepper_default_content_padding_for_horizontal_stepper() {
        test_widgets(|tester| {
            pump(
                tester,
                Stepper::new(vec![text_step("Step 1", "Content 1")])
                    .r#type(StepperType::Horizontal),
            );

            // Verify the default padding is applied to ListView
            let padding =
                stepper_list_view(tester, |list_view| list_view.box_scroll_view_data().padding);
            assert_eq!(padding, Some(EdgeInsets::all(24.0).into()));
        });
    }

    /// The padding nearest above `Content 1`, resolved left to right: Dart's
    /// `tester.widget<Padding>(contentPadding.first).padding.resolve(TextDirection.ltr)`.
    fn content_padding_of(tester: &mut WidgetTester, stepper: Stepper) -> EdgeInsets {
        pump(tester, stepper);
        paddings_above(tester, "Content 1")[0]
    }

    #[test]
    fn stepper_vertical_content_padding_includes_step_icon_margin_start_value() {
        test_widgets(|tester| {
            let padding = content_padding_of(
                tester,
                Stepper::new(vec![text_step("Step 1", "Content 1")])
                    .step_icon_margin(EdgeInsets::only(16.0, 0.0, 8.0, 0.0))
                    .content_padding(EdgeInsetsDirectional::only(40.0, 0.0, 20.0, 16.0)),
            );

            // The effective padding should be the content padding plus the icon margin's left:
            // start: 40.0 + 16.0 = 56.0
            assert_eq!(padding, EdgeInsets::only(56.0, 0.0, 20.0, 16.0));
        });
    }

    #[test]
    fn stepper_vertical_default_content_padding_includes_step_icon_margin_start_value() {
        test_widgets(|tester| {
            let padding = content_padding_of(
                tester,
                Stepper::new(vec![text_step("Step 1", "Content 1")])
                    .step_icon_margin(EdgeInsets::only(10.0, 0.0, 0.0, 0.0)),
            );

            // Default padding is start: 60.0, end: 24.0, bottom: 24.0
            // Plus stepIconMargin.left: 10.0
            // Expected start: 60.0 + 10.0 = 70.0
            assert_eq!(padding, EdgeInsets::only(70.0, 0.0, 24.0, 24.0));
        });
    }

    #[test]
    fn stepper_vertical_content_padding_without_step_icon_margin_uses_default() {
        test_widgets(|tester| {
            let padding =
                content_padding_of(tester, Stepper::new(vec![text_step("Step 1", "Content 1")]));

            // Default padding without stepIconMargin
            assert_eq!(padding, EdgeInsets::only(60.0, 0.0, 24.0, 24.0));
        });
    }
}
