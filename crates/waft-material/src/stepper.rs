//! Flutter counterpart: material_ui's `stepper.dart`.

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use waft_embedder::{
    Brightness, Canvas, Clip, Color, FillRule, Paint, PathBuilder, Size, TextDirection, valo,
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
    WidgetState, WidgetStateProperty, WidgetStatePropertyRef,
};

use crate::button_style::ButtonStyle;
use crate::colors::Colors;
use crate::constants::K_THEME_CHANGE_DURATION;
use crate::debug::{debug_check_has_material, debug_check_has_material_localizations};
use crate::icons::Icons;
use crate::ink_well::{InkResponse, InkWell};
use crate::material::Material;
use crate::material_localizations::MaterialLocalizations;
use crate::text_button::TextButton;
use crate::theme::Theme;

fn k_step_style() -> TextStyle {
    TextStyle::new().font_size(12.0).color(Colors::WHITE)
}
const K_STEP_SIZE: f64 = 24.0;
/// `sqrt(3.0) / 2.0`.
const K_TRIANGLE_SQRT: f64 = 0.866_025;
const K_TRIANGLE_HEIGHT: f64 = K_STEP_SIZE * K_TRIANGLE_SQRT;
const K_MAX_STEP_SIZE: f64 = 80.0;

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
        assert!(!steps.is_empty(), "a stepper has at least one step");
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
        assert!(
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

impl Clone for Stepper {
    fn clone(&self) -> Stepper {
        Stepper {
            key: self.key.clone(),
            steps: self.steps.clone(),
            physics: self.physics.clone(),
            controller: self.controller.clone(),
            r#type: self.r#type,
            current_step: self.current_step,
            on_step_tapped: self.on_step_tapped.clone(),
            on_step_continue: self.on_step_continue.clone(),
            on_step_cancel: self.on_step_cancel.clone(),
            controls_builder: self.controls_builder.clone(),
            elevation: self.elevation,
            margin: self.margin,
            connector_color: self.connector_color.clone(),
            connector_thickness: self.connector_thickness,
            step_icon_builder: self.step_icon_builder.clone(),
            step_icon_height: self.step_icon_height,
            step_icon_width: self.step_icon_width,
            step_icon_margin: self.step_icon_margin,
            clip_behavior: self.clip_behavior,
            header_padding: self.header_padding,
            content_padding: self.content_padding,
        }
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
    /// The padding around each step's icon.
    fn step_icon_margin(self: Handle<Self>, stepper: &Stepper) -> EdgeInsets {
        stepper
            .step_icon_margin
            .unwrap_or_else(|| EdgeInsets::symmetric(8.0, 0.0))
    }

    /// The padding around the header row.
    fn effective_header_padding(self: Handle<Self>, stepper: &Stepper) -> EdgeInsetsGeometry {
        stepper
            .header_padding
            .unwrap_or_else(|| EdgeInsetsGeometry::symmetric(0.0, 24.0))
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
        let states: waft_widgets::WidgetStates = if is_active {
            [WidgetState::Selected].into_iter().collect()
        } else {
            [WidgetState::Disabled].into_iter().collect()
        };
        self.widget(app)
            .connector_color
            .as_ref()
            .map(|connector_color| connector_color.resolve(&states))
            .unwrap_or_else(|| {
                if is_active {
                    color_scheme.primary
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
            *app.get(self)
                .old_states
                .get(&index)
                .unwrap_or(&stepper.steps[index].state)
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
            Colors::BLACK87
        } else {
            Colors::WHITE
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
        reason = "Dart's `_circleColor` reads the deprecated `ColorScheme.background`"
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
        let states: waft_widgets::WidgetStates = if is_active {
            [WidgetState::Selected].into_iter().collect()
        } else {
            [WidgetState::Disabled].into_iter().collect()
        };
        if let Some(connector_color) = stepper
            .connector_color
            .as_ref()
            .map(|connector_color| connector_color.resolve(&states))
        {
            return connector_color;
        }
        if !self.is_dark(app, context) {
            if is_active {
                color_scheme.primary
            } else {
                color_scheme
                    .on_surface
                    .with_values(Some(0.38), None, None, None, None)
            }
        } else if is_active {
            color_scheme.secondary
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
        Padding::new(self.step_icon_margin(&stepper).into())
            .child(
                SizedBox::new()
                    .width(stepper.step_icon_width.unwrap_or(K_STEP_SIZE))
                    .height(stepper.step_icon_height.unwrap_or(K_STEP_SIZE))
                    .child(
                        AnimatedContainer::new(K_THEME_CHANGE_DURATION)
                            .curve(waft_animation::Curves::fast_out_slow_in())
                            .decoration(decoration)
                            .child(
                                Center::new()
                                    .child(self.build_circle_child(app, context, index, old_state)),
                            ),
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
                    Colors::RED.shade400()
                } else {
                    Colors::RED.primary()
                }
            });
        let width = stepper.step_icon_width.unwrap_or(K_STEP_SIZE);
        let height = stepper
            .step_icon_height
            .map(|height| height * K_TRIANGLE_SQRT)
            .unwrap_or(K_TRIANGLE_HEIGHT);
        Padding::new(self.step_icon_margin(&stepper).into())
            .child(
                SizedBox::new()
                    .width(stepper.step_icon_width.unwrap_or(K_STEP_SIZE))
                    .height(stepper.step_icon_height.unwrap_or(K_STEP_SIZE))
                    .child(
                        Center::new().child(
                            SizedBox::new().width(width).height(height).child(
                                CustomPaint::new().painter(TrianglePainter { color }).child(
                                    Align::new()
                                        // 0.8 looks better than the geometrical 0.33.
                                        .alignment(Alignment::new(0.0, 0.8).into())
                                        .child(
                                            self.build_circle_child(app, context, index, old_state),
                                        ),
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
        let old_state = *app.get(self).old_states.get(&index).unwrap_or(&state);
        if state != old_state {
            AnimatedCrossFade::new(
                self.build_circle(app, context, index, true),
                self.build_triangle(app, context, index, true),
                if state == StepState::Error {
                    waft_widgets::CrossFadeState::ShowSecond
                } else {
                    waft_widgets::CrossFadeState::ShowFirst
                },
                K_THEME_CHANGE_DURATION,
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
                Colors::WHITE38
            } else {
                Colors::BLACK38
            }),
            StepState::Error => body_large.copy_with().color(if self.is_dark(app, context) {
                Colors::RED.shade400()
            } else {
                Colors::RED.primary()
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
                Colors::WHITE38
            } else {
                Colors::BLACK38
            }),
            StepState::Error => body_small.copy_with().color(if self.is_dark(app, context) {
                Colors::RED.shade400()
            } else {
                Colors::RED.primary()
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
                K_THEME_CHANGE_DURATION,
            )
            .curve(waft_animation::Curves::fast_out_slow_in())
            .into_widget(),
        ];
        if let Some(subtitle) = stepper.steps[index].subtitle.clone() {
            let style = self.subtitle_style(app, context, index);
            children.push(
                Padding::new(EdgeInsets::only(0.0, 2.0, 0.0, 0.0).into())
                    .child(
                        AnimatedDefaultTextStyle::new(subtitle, style, K_THEME_CHANGE_DURATION)
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
                AnimatedDefaultTextStyle::new(label, style, K_THEME_CHANGE_DURATION).into_widget()
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
                        Padding::new(EdgeInsetsDirectional::only(12.0, 0.0, 0.0, 0.0).into())
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
        let cancel_color = if is_dark {
            Colors::WHITE70
        } else {
            Colors::BLACK54
        };
        let theme_data = Theme::of(app, context);
        let color_scheme = theme_data.color_scheme.clone();
        let localizations = <dyn MaterialLocalizations>::of(app, context);

        let button_shape = OutlinedBorderRef::new(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::all(waft_embedder::Radius::circular(2.0)),
        ));
        let button_padding = EdgeInsetsGeometry::symmetric(0.0, 16.0);

        let continue_foreground_color: WidgetStatePropertyRef<Option<Color>> =
            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                if states.contains(&WidgetState::Disabled) {
                    None
                } else if is_dark {
                    Some(color_scheme.on_surface)
                } else {
                    Some(color_scheme.on_primary)
                }
            });
        let continue_background_color: WidgetStatePropertyRef<Option<Color>> = {
            let color_scheme = Theme::of(app, context).color_scheme.clone();
            <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                if is_dark || states.contains(&WidgetState::Disabled) {
                    None
                } else {
                    Some(color_scheme.primary)
                }
            })
        };
        let padding_property: WidgetStatePropertyRef<Option<EdgeInsetsGeometry>> =
            <dyn WidgetStateProperty<Option<EdgeInsetsGeometry>>>::resolve_with(move |_| {
                Some(button_padding)
            });
        let shape_property: WidgetStatePropertyRef<Option<OutlinedBorderRef>> = {
            let button_shape = button_shape.clone();
            <dyn WidgetStateProperty<Option<OutlinedBorderRef>>>::resolve_with(move |_| {
                Some(button_shape.clone())
            })
        };
        let continue_style = ButtonStyle::new()
            .foreground_color(continue_foreground_color)
            .background_color(continue_background_color)
            .padding(padding_property)
            .shape(shape_property);

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

        let mut continue_button = TextButton::new(
            stepper.on_step_continue.clone(),
            Some(Text::new(continue_label)),
        );
        continue_button = continue_button.style(continue_style);
        let mut cancel_button = TextButton::new(
            stepper.on_step_cancel.clone(),
            Some(Text::new(cancel_label)),
        );
        cancel_button = cancel_button.style(cancel_style);

        Padding::new(EdgeInsets::only(0.0, 16.0, 0.0, 0.0).into())
            .child(SizedBox::new().height(48.0).child(Row::new().children(vec![
                        continue_button.into_widget(),
                        Padding::new(EdgeInsetsDirectional::only(8.0, 0.0, 0.0, 0.0).into())
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
        let additional_margin_left = margin_left.unwrap_or(0.0) / 2.0;
        let additional_margin_right = margin_right.unwrap_or(0.0) / 2.0;
        // Adjust vertical content padding to align with step icon when stepIconMargin is set.
        let effective_vertical_content_padding = stepper
            .content_padding
            .unwrap_or_else(|| EdgeInsetsDirectional::only(60.0, 0.0, 24.0, 24.0).into())
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
        .start(24.0 + additional_margin_left + additional_margin_right)
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
            K_THEME_CHANGE_DURATION,
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
                        if let Some(current_context) = tap_key.current_context(app) {
                            drop(Scrollable::ensure_visible(
                                app,
                                current_context,
                                0.0,
                                K_THEME_CHANGE_DURATION,
                                waft_animation::Curves::fast_out_slow_in(),
                                ScrollPositionAlignmentPolicy::Explicit,
                            ));
                        }
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
        let is_last = stepper.steps.len() - 1;

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
            let margin = stepper.step_icon_margin.map_or_else(
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

            if index != is_last {
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

        let content = AnimatedSize::new(K_THEME_CHANGE_DURATION)
            .curve(waft_animation::Curves::fast_out_slow_in())
            .child(
                Column::new()
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .children(step_panels),
            );
        let controls = self.build_vertical_controls(app, context, stepper.current_step);
        let horizontal_content_padding = stepper
            .content_padding
            .unwrap_or_else(|| EdgeInsets::all(24.0).into());
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
        assert!(
            !cfg!(debug_assertions)
                || context
                    .find_ancestor_widget_of_exact_type::<Stepper>(app)
                    .is_none(),
            "Steppers must not be nested.\n\
             The material specification advises that one should avoid embedding steppers within \
             steppers.\n\
             https://material.io/archive/guidelines/components/steppers.html#steppers-usage"
        );
        match self.widget(app).r#type {
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
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use waft_embedder::Size;
    use waft_widgets::testing::{WidgetTester, find, test_widgets};
    use waft_widgets::{Center, IntoWidget, SizedBox, Text};

    use super::*;
    use crate::material::Material;
    use crate::scaffold::Scaffold;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    fn step(title: &'static str, content: &'static str) -> Step {
        Step::new(Text::new(title), Text::new(content))
    }

    /// Dart's `pumpWidget(MaterialApp(home: Material(child: stepper)))`.
    fn pump(tester: &mut WidgetTester, stepper: Stepper) {
        let theme = ThemeData::builder().build(&tester.cell.borrow());
        tester.pump_widget(material_app(
            theme,
            Material::new().child(stepper).into_widget(),
        ));
    }

    #[test]
    fn the_material3_labels_are_sentence_case() {
        test_widgets(|tester| {
            pump(
                tester,
                Stepper::new(vec![step("Step 1", "A"), step("Step 2", "B")]),
            );
            assert!(!find::text("Continue").evaluate(tester).is_empty());
            assert!(!find::text("Cancel").evaluate(tester).is_empty());
        });
    }

    #[test]
    fn tapping_a_step_reports_its_index() {
        test_widgets(|tester| {
            let tapped = Rc::new(Cell::new(usize::MAX));
            let seen = Rc::clone(&tapped);
            pump(
                tester,
                Stepper::new(vec![step("Step 1", "A"), step("Step 2", "B")])
                    .on_step_tapped(Rc::new(move |_app, index| seen.set(index))),
            );
            tester.tap(find::text("Step 2"));
            assert_eq!(tapped.get(), 1);
        });
    }

    #[test]
    fn a_disabled_step_does_not_report_a_tap() {
        test_widgets(|tester| {
            let tapped = Rc::new(Cell::new(false));
            let seen = Rc::clone(&tapped);
            pump(
                tester,
                Stepper::new(vec![
                    step("Step 1", "A"),
                    step("Step 2", "B").state(StepState::Disabled),
                ])
                .on_step_tapped(Rc::new(move |_app, _index| seen.set(true))),
            );
            tester.tap(find::text("Step 2"));
            assert!(!tapped.get());
        });
    }

    #[test]
    fn the_horizontal_stepper_shows_only_the_current_step_s_content() {
        test_widgets(|tester| {
            let steps = || vec![step("Step 1", "A"), step("Step 2", "B")];
            pump(
                tester,
                Stepper::new(steps()).r#type(StepperType::Horizontal),
            );
            assert_eq!(find::text("A").evaluate(tester).len(), 1);
            assert!(find::text("B").evaluate(tester).is_empty());

            pump(
                tester,
                Stepper::new(steps())
                    .r#type(StepperType::Horizontal)
                    .current_step(1),
            );
            tester.pump_and_settle();
            assert!(find::text("A").evaluate(tester).is_empty());
            assert_eq!(find::text("B").evaluate(tester).len(), 1);
        });
    }

    #[test]
    fn a_completed_step_shows_a_check_and_an_editing_step_a_pencil() {
        test_widgets(|tester| {
            pump(
                tester,
                Stepper::new(vec![
                    step("A", "A").state(StepState::Complete),
                    step("B", "B").state(StepState::Editing),
                ]),
            );
            assert_eq!(find::by_icon(&Icons::check()).evaluate(tester).len(), 1);
            assert_eq!(find::by_icon(&Icons::edit()).evaluate(tester).len(), 1);
            assert_eq!(
                tester.get_size(find::by_icon(&Icons::check())),
                Size::square(18.0)
            );
            assert_eq!(
                tester.get_size(find::by_icon(&Icons::edit())),
                Size::square(18.0)
            );
        });
    }

    #[test]
    fn an_error_step_shows_the_error_mark() {
        test_widgets(|tester| {
            pump(
                tester,
                Stepper::new(vec![step("A", "A").state(StepState::Error)]),
            );
            assert_eq!(find::text("!").evaluate(tester).len(), 1);
        });
    }

    #[test]
    fn a_completed_step_does_not_show_its_index() {
        test_widgets(|tester| {
            pump(
                tester,
                Stepper::new(vec![
                    step("A", "A").state(StepState::Complete),
                    step("B", "B"),
                ]),
            );
            assert!(find::text("1").evaluate(tester).is_empty());
            assert_eq!(find::text("2").evaluate(tester).len(), 1);
        });
    }

    #[test]
    fn the_custom_controls_replace_the_default_buttons() {
        test_widgets(|tester| {
            let continued = Rc::new(Cell::new(false));
            let cancelled = Rc::new(Cell::new(false));
            let (continued_seen, cancelled_seen) = (Rc::clone(&continued), Rc::clone(&cancelled));
            let controls: ControlsWidgetBuilder = Rc::new(move |_app, _context, details| {
                let continued_seen = Rc::clone(&continued_seen);
                let cancelled_seen = Rc::clone(&cancelled_seen);
                let continue_button = TextButton::new(
                    details
                        .on_step_continue
                        .map(|_| Listener::new(move |_app| continued_seen.set(true))),
                    Some(Text::new("Let us continue!")),
                );
                let cancel_button = TextButton::new(
                    details
                        .on_step_cancel
                        .map(|_| Listener::new(move |_app| cancelled_seen.set(true))),
                    Some(Text::new("Cancel This!")),
                );
                Row::new()
                    .children(vec![
                        continue_button.into_widget(),
                        cancel_button.into_widget(),
                    ])
                    .into_widget()
            });
            pump(
                tester,
                Stepper::new(vec![step("A", "A")])
                    .controls_builder(controls)
                    .on_step_continue(Listener::new(|_app| {}))
                    .on_step_cancel(Listener::new(|_app| {})),
            );
            assert!(find::text("Continue").evaluate(tester).is_empty());
            tester.tap(find::text("Let us continue!"));
            assert!(continued.get());
            tester.tap(find::text("Cancel This!"));
            assert!(cancelled.get());
        });
    }

    #[test]
    fn a_step_style_color_fills_the_circle() {
        test_widgets(|tester| {
            let color = Color::new(0xFF123456);
            pump(
                tester,
                Stepper::new(vec![
                    step("A", "A").step_style(StepStyle::new().with_color(color)),
                ]),
            );
            let found = find::by_type::<AnimatedContainer>()
                .evaluate(tester)
                .into_iter()
                .filter_map(|element| {
                    tester.widget(element, |container: &AnimatedContainer| {
                        container.decoration.as_ref().and_then(|decoration| {
                            decoration
                                .as_any()
                                .downcast_ref::<BoxDecoration>()
                                .and_then(|decoration| decoration.color.as_ref())
                                .map(|color| color.color())
                        })
                    })
                })
                .collect::<Vec<_>>();
            assert!(
                found.contains(&color),
                "the circle is filled with the step's color"
            );
        });
    }

    #[test]
    fn the_step_icon_builder_replaces_the_circle() {
        test_widgets(|tester| {
            let builder: StepIconBuilder =
                Rc::new(|index, _state| Some(Text::new(format!("icon {index}")).into_widget()));
            pump(
                tester,
                Stepper::new(vec![step("A", "A")]).step_icon_builder(builder),
            );
            assert_eq!(find::text("icon 0").evaluate(tester).len(), 1);
            assert!(find::text("1").evaluate(tester).is_empty());
        });
    }

    #[test]
    #[should_panic(expected = "Steppers must not be nested")]
    fn a_stepper_may_not_be_nested() {
        test_widgets(|tester| {
            pump(
                tester,
                Stepper::new(vec![
                    Step::new(
                        Text::new("Step 1"),
                        Stepper::new(vec![step("Nested step 1", "A"), step("Nested step 2", "B")]),
                    ),
                    step("Step 2", "B"),
                ])
                .r#type(StepperType::Horizontal),
            );
        });
    }

    #[test]
    fn the_stepper_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            for r#type in [StepperType::Vertical, StepperType::Horizontal] {
                let theme = ThemeData::builder().build(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Scaffold::new()
                        .body(Center::new().child(SizedBox::shrink().child(
                            Stepper::new(vec![step("X", "X"), step("Y", "Y")]).r#type(r#type),
                        )))
                        .into_widget(),
                ));
                assert_eq!(tester.get_size(find::by_type::<Stepper>()), Size::ZERO);
            }
        });
    }
}
