//! WinUI ProgressBar.xaml and ProgressBar.cpp: range sizing and CommonStates storyboards.

use crate::*;
use inset_animation::{Cubic, Curve};
use inset_embedder::{Color, Rect, Size};
use inset_foundation::{App, Handle};
use inset_painting::EdgeInsetsGeometry;
use inset_rendering::CustomClipper;
use inset_scheduler::{FrameCallback, Ticker};
use inset_widgets::*;
use std::{any::Any, time::Duration};

/// A horizontal progress indicator with the Fluent determinate, paused, error and indeterminate states.
#[derive(Clone, Debug)]
pub struct ProgressBar {
    /// Lower bound of the progress range.
    pub minimum: f64,

    /// Upper bound of the range; the Fluent style sets it to 100.
    pub maximum: f64,

    /// Progress value, coerced to the range.
    pub value: f64,

    /// Whether the two moving indicators replace the determinate indicator.
    pub is_indeterminate: bool,

    /// Whether progress is paused; error takes precedence.
    pub show_paused: bool,

    /// Whether the error visual state is active.
    pub show_error: bool,

    /// Whether the control is visible and its indeterminate animation runs.
    pub is_visible: bool,

    /// Height of the indicator grid, as bound by the source template; the theme's `ProgressBarMinHeight` when `None`.
    pub min_height: Option<f64>,

    /// Padding in left, top, right, bottom order.
    pub padding: [f64; 4],

    /// Border widths in left, top, right, bottom order; the theme's `ProgressBarBorderThemeThickness` when `None`.
    pub border_thickness: Option<[f64; 4]>,

    /// Radius of the indicator rectangles; the theme's `ProgressBarCornerRadius` when `None`.
    pub corner_radius: Option<f64>,

    /// Optional normal indicator color.
    pub foreground: Option<Color>,

    /// Optional track color.
    pub background: Option<Color>,

    /// Optional outer border color.
    pub border_brush: Option<Color>,

    /// Widget identity across rebuilds.
    pub key: Option<KeyRef>,
}

impl ProgressBar {
    /// Creates the source default ProgressBar.
    pub fn new() -> Self {
        Self {
            minimum: 0.0,
            maximum: 100.0,
            value: 0.0,
            is_indeterminate: false,
            show_paused: false,
            show_error: false,
            is_visible: true,
            min_height: None,
            padding: [0.0; 4],
            border_thickness: None,
            corner_radius: None,
            foreground: None,
            background: None,
            border_brush: None,
            key: None,
        }
    }

    /// Sets Minimum.
    pub fn minimum(mut self, value: f64) -> Self {
        self.minimum = value;
        self
    }

    /// Sets Maximum.
    pub fn maximum(mut self, value: f64) -> Self {
        self.maximum = value;
        self
    }

    /// Sets Value.
    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }

    /// Sets IsIndeterminate.
    pub fn is_indeterminate(mut self, value: bool) -> Self {
        self.is_indeterminate = value;
        self
    }

    /// Sets ShowPaused.
    pub fn show_paused(mut self, value: bool) -> Self {
        self.show_paused = value;
        self
    }

    /// Sets ShowError.
    pub fn show_error(mut self, value: bool) -> Self {
        self.show_error = value;
        self
    }

    /// Sets IsVisible.
    pub fn is_visible(mut self, value: bool) -> Self {
        self.is_visible = value;
        self
    }

    /// Sets MinHeight.
    pub fn min_height(mut self, value: f64) -> Self {
        self.min_height = Some(value);
        self
    }

    /// Sets Padding.
    pub fn padding(mut self, value: [f64; 4]) -> Self {
        self.padding = value;
        self
    }

    /// Sets BorderThickness.
    pub fn border_thickness(mut self, value: [f64; 4]) -> Self {
        self.border_thickness = Some(value);
        self
    }

    /// Sets CornerRadius.
    pub fn corner_radius(mut self, value: f64) -> Self {
        self.corner_radius = Some(value);
        self
    }

    /// Sets Foreground.
    pub fn foreground(mut self, value: Color) -> Self {
        self.foreground = Some(value);
        self
    }

    /// Sets Background.
    pub fn background(mut self, value: Color) -> Self {
        self.background = Some(value);
        self
    }

    /// Sets BorderBrush.
    pub fn border_brush(mut self, value: Color) -> Self {
        self.border_brush = Some(value);
        self
    }

    /// Sets Key.
    pub fn key(mut self, value: KeyRef) -> Self {
        self.key = Some(value);
        self
    }

    /// Chooses CommonStates using ProgressBar::UpdateStates precedence.
    fn visual_state(&self) -> ProgressState {
        match (
            self.is_indeterminate && self.is_visible,
            self.show_error,
            self.show_paused,
        ) {
            (true, true, _) => ProgressState::IndeterminateError,
            (true, false, true) => ProgressState::IndeterminatePaused,
            (true, false, false) => ProgressState::Indeterminate,
            (false, true, _) => ProgressState::Error,
            (false, false, true) => ProgressState::Paused,
            _ => ProgressState::Determinate,
        }
    }

    /// Resolves the target fill of the active state.
    fn indicator_color(&self, resources: &ThemeResources) -> Color {
        if self.show_error {
            resources.progress_bar_error_foreground_color
        } else if self.show_paused {
            resources.progress_bar_paused_foreground_color
        } else {
            self.foreground.unwrap_or(resources.progress_bar_foreground)
        }
    }
}

impl Default for ProgressBar {
    fn default() -> Self {
        Self::new()
    }
}

/// Stable states of ProgressBar.xaml's CommonStates group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProgressState {
    Determinate,
    Updating,
    UpdatingError,
    Paused,
    Error,
    Indeterminate,
    IndeterminatePaused,
    IndeterminateError,
}

impl ProgressState {
    /// Whether the second indicator is filling the track after a pause or error.
    fn suspended(self) -> bool {
        matches!(self, Self::IndeterminatePaused | Self::IndeterminateError)
    }
}

/// A sampled source storyboard, with translation expressed as a multiple of LayoutRoot width.
#[derive(Clone, Copy, Debug)]
struct ProgressVisuals {
    /// Opacity of the determinate track.
    track_opacity: f64,

    /// Translation of the track.
    track_translation: f64,

    /// Translation of the first 40-percent indicator, absent when collapsed.
    first: Option<f64>,

    /// Translation of the second indicator, absent when collapsed.
    second: Option<f64>,
}

/// Evaluates the explicit spline keys; Windows theme animations are documented separately.
fn progress_visuals(
    state: ProgressState,
    from: ProgressState,
    elapsed: f64,
    initial_second: f64,
) -> ProgressVisuals {
    let mut result = ProgressVisuals {
        track_opacity: 1.0,
        track_translation: 0.0,
        first: None,
        second: None,
    };

    if state == ProgressState::Indeterminate {
        if from.suspended() && elapsed < 0.5 {
            let curve = Cubic::new(1.0, 0.0, 1.0, 1.0);
            result.second = Some(
                initial_second
                    + (1.2 - initial_second) * curve.transform((elapsed / 0.333).min(1.0)),
            );
            result.track_opacity = 0.0;
            result.track_translation = 1.2 * curve.transform((elapsed / 0.5).min(1.0));
            return result;
        }

        let time = (elapsed - if from.suspended() { 0.5 } else { 0.0 }).max(0.0) % 2.0;
        let curve = Cubic::new(0.4, 0.0, 0.6, 1.0);
        result.track_opacity = 0.0;
        result.first = Some(-0.4 + 1.6 * curve.transform((time / 1.5).min(1.0)));
        result.second =
            Some(-0.9 + 1.896 * curve.transform(((time - 0.75) / 1.25).clamp(0.0, 1.0)));
    } else if state.suspended() {
        result.track_opacity = 0.0;
        // The two keys at 167 ms first reach the end and then jump to the start.
        result.second = Some(if elapsed < 0.167 {
            initial_second
                + (0.996 - initial_second)
                    * Cubic::new(1.0, 1.0, 0.0, 1.0).transform(elapsed / 0.167)
        } else {
            -0.9 + 0.9
                * Cubic::new(0.0, 0.0, 0.0, 1.0).transform(((elapsed - 0.167) / 0.583).min(1.0))
        });
        result.track_translation =
            -0.9 + 0.9 * Cubic::new(0.0, 0.0, 0.0, 1.0).transform((elapsed / 0.75).min(1.0));
    }

    result
}

/// Owns the source storyboard clock and the color captured when a state changes.
pub struct ProgressBarState {
    /// Framework widget state.
    state: StateData<ProgressBar>,

    /// Native TickerMode support.
    single_ticker_provider: SingleTickerProviderStateMixinData,

    /// Clock shared by the source tracks.
    ticker: Option<Handle<Ticker>>,

    /// State whose visual values were captured at transition start.
    from: ProgressState,

    /// Elapsed time in the active storyboard.
    elapsed: Duration,

    /// Second indicator position when a transition interrupted it.
    initial_second: f64,

    /// Initial fill for the source's 167 ms color animation.
    from_color: Option<Color>,

    /// Last LayoutRoot size whose SizeChanged callback updated the source state.
    layout_size: Option<Size>,
}

impl StatefulWidget for ProgressBar {
    type State = ProgressBarState;

    /// Returns the identity used to retain this control across owner rebuilds.
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    /// Initializes template state from the first widget description.
    fn create_state(&self) -> ProgressBarState {
        ProgressBarState {
            state: StateData::new(),
            single_ticker_provider: Default::default(),
            ticker: None,
            from: self.visual_state(),
            elapsed: Duration::ZERO,
            initial_second: 0.0,
            from_color: None,
            layout_size: None,
        }
    }
}

impl SingleTickerProviderStateMixin for ProgressBarState {
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

impl ProgressBarState {
    /// SetProgressBarIndicatorWidth visits Updating before returning to the target state.
    fn update_indicator_width(self: Handle<Self>, app: &mut App) {
        let resources = ThemeResources::of(app, self.context(app));
        let widget = self.widget(app);
        let from = if widget.show_error {
            ProgressState::UpdatingError
        } else {
            ProgressState::Updating
        };
        let color = if widget.show_error && !widget.is_indeterminate {
            resources.progress_bar_error_foreground_color
        } else {
            widget
                .foreground
                .unwrap_or(resources.progress_bar_foreground)
        };
        self.start_state(app, from, color, 0.0);
    }

    /// Starts the selected source storyboard after leaving the previous visual state.
    fn start_state(
        self: Handle<Self>,
        app: &mut App,
        from: ProgressState,
        color: Color,
        second: f64,
    ) {
        let ticker = app.get(self).ticker.unwrap();
        ticker.stop(app, false);
        let state = app.get_mut(self);
        state.from = from;
        state.from_color = Some(color);
        state.initial_second = second;
        state.elapsed = Duration::ZERO;

        if self.widget(app).is_visible {
            ticker.start(app);
        }
    }

    /// Advances finite tracks to their final key and keeps indeterminate tracks repeating.
    fn tick(self: Handle<Self>, app: &mut App, elapsed: Duration) {
        self.set_state(app, |state| state.elapsed = elapsed);
        let target = self.widget(app).visual_state();
        if target != ProgressState::Indeterminate && elapsed >= Duration::from_millis(750) {
            app.get(self).ticker.unwrap().stop(app, false);
        }
    }

    /// Current animated fill, including an interrupted color transition.
    fn color(
        self: Handle<Self>,
        app: &App,
        widget: &ProgressBar,
        resources: &ThemeResources,
    ) -> Color {
        let target = widget.indicator_color(resources);
        let state = app.get(self);
        Color::lerp(
            state.from_color,
            Some(target),
            (state.elapsed.as_secs_f64() / 0.167).min(1.0),
        )
        .unwrap_or(target)
    }
}

impl State for ProgressBarState {
    type Widget = ProgressBar;
    inset_widgets::state_accessors!();

    /// Creates the native clock for the source storyboards.
    fn init_state(self: Handle<Self>, app: &mut App) {
        let ticker = SingleTickerProviderStateMixin::create_ticker(
            self,
            app,
            FrameCallback::new(move |app, elapsed| self.tick(app, elapsed)),
        );
        app.get_mut(self).ticker = Some(ticker);
        if self.widget(app).is_indeterminate && self.widget(app).is_visible {
            ticker.start(app);
        }
    }

    /// Applies source property-change behavior when the owner supplies new values.
    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &ProgressBar) {
        let widget = self.widget(app);
        let width_component_changed = old.value != widget.value
            || old.minimum != widget.minimum
            || old.maximum != widget.maximum
            || old.padding != widget.padding
            || old.is_indeterminate != widget.is_indeterminate
            || old.show_paused != widget.show_paused
            || old.show_error != widget.show_error;
        if width_component_changed {
            self.update_indicator_width(app);
        } else if old.visual_state() != widget.visual_state() {
            let resources = ThemeResources::of(app, self.context(app));
            let color = self.color(app, old, &resources);
            let previous = progress_visuals(
                old.visual_state(),
                app.get(self).from,
                app.get(self).elapsed.as_secs_f64(),
                app.get(self).initial_second,
            );
            self.start_state(
                app,
                old.visual_state(),
                color,
                previous.second.unwrap_or(0.0),
            );
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    /// Releases the storyboard clock before disposing its ticker provider.
    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get(self).ticker.unwrap().dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    /// Builds the named template parts from the current visual state.
    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let resources = ThemeResources::of(app, context);
        let state = widget.visual_state();
        let scale = MediaQuery::device_pixel_ratio_of(app, context);
        let border = widget
            .border_thickness
            .unwrap_or(resources.progress_bar_border_theme_thickness)
            .map(|side| (side * scale).round() / scale);
        let range = RangeBase::new(widget.minimum, widget.maximum, widget.value).coerced();

        let body = LayoutBuilder::new(move |app, _, constraints| {
            let width = if constraints.max_width.is_finite() {
                constraints.max_width
            } else {
                constraints.min_width
            };
            let inset = widget.padding[0] + widget.padding[2] + border[0] + border[2];
            let available = (width - inset).max(0.0);
            let height = widget
                .min_height
                .unwrap_or(resources.progress_bar_min_height);
            let root_size = constraints.constrain(Size::new(
                width,
                height + widget.padding[1] + widget.padding[3] + border[1] + border[3],
            ));
            if app.get(self).layout_size != Some(root_size) {
                app.get_mut(self).layout_size = Some(root_size);
                // OnSizeChanged calls SetProgressBarIndicatorWidth before updating the template settings.
                self.update_indicator_width(app);
            }
            let foreground = self.color(app, &widget, &resources);
            let visuals = progress_visuals(
                state,
                app.get(self).from,
                app.get(self).elapsed.as_secs_f64(),
                app.get(self).initial_second,
            );
            let mut parts = Vec::new();

            // ProgressBarTrack.
            if visuals.track_opacity > 0.0 {
                parts.push(
                    Positioned::new(progress_rectangle(
                        available,
                        resources.progress_bar_track_height,
                        resources.progress_bar_track_corner_radius[0],
                        widget
                            .background
                            .unwrap_or(resources.progress_bar_background),
                    ))
                    .left(visuals.track_translation * width)
                    .top((height - resources.progress_bar_track_height) / 2.0)
                    .into_widget(),
                );
            }

            // DeterminateProgressBarIndicator.
            if !widget.is_indeterminate {
                parts.push(
                    Positioned::new(progress_rectangle(
                        if (range.maximum - range.minimum).abs() > f64::EPSILON {
                            available * range.fraction_of(range.value)
                        } else {
                            0.0
                        },
                        height,
                        widget
                            .corner_radius
                            .unwrap_or(resources.progress_bar_corner_radius[0]),
                        foreground,
                    ))
                    .left(0.0)
                    .top(0.0)
                    .into_widget(),
                );
            }

            // IndeterminateProgressBarIndicator and IndeterminateProgressBarIndicator2.
            if let Some(position) = visuals.first {
                parts.push(
                    Positioned::new(progress_rectangle(
                        available * 0.4,
                        height,
                        widget
                            .corner_radius
                            .unwrap_or(resources.progress_bar_corner_radius[0]),
                        foreground,
                    ))
                    .left(position * width)
                    .top(0.0)
                    .into_widget(),
                );
            }
            if let Some(position) = visuals.second {
                parts.push(
                    Positioned::new(progress_rectangle(
                        available * if state.suspended() { 1.0 } else { 0.6 },
                        height,
                        widget
                            .corner_radius
                            .unwrap_or(resources.progress_bar_corner_radius[0]),
                        foreground,
                    ))
                    .left(position * width)
                    .top(0.0)
                    .into_widget(),
                );
            }

            SizedBox::new()
                .width(width)
                .child(
                    ControlBorder::new(
                        Brush::Solid(Color::from_argb(0, 0, 0, 0)),
                        Brush::Solid(
                            widget
                                .border_brush
                                .unwrap_or(resources.progress_bar_border_brush),
                        ),
                    )
                    .border_thickness_ltrb(border)
                    .corner_radius(
                        widget
                            .corner_radius
                            .unwrap_or(resources.progress_bar_corner_radius[0]),
                    )
                    .child(
                        Padding::new(EdgeInsetsGeometry::from_ltrb(
                            widget.padding[0],
                            widget.padding[1],
                            widget.padding[2],
                            widget.padding[3],
                        ))
                        .child(
                            ClipRect::new()
                                .clipper(ProgressClipper(Rect::from_ltwh(
                                    widget.padding[0],
                                    widget.padding[1],
                                    (width - widget.padding[0] - widget.padding[2]).max(0.0),
                                    (root_size.height() - widget.padding[1] - widget.padding[3])
                                        .max(0.0),
                                )))
                                .child(
                                    Align::new().height_factor(1.0).child(
                                        SizedBox::new().height(height).child(
                                            Stack::new()
                                                .clip_behavior(inset_embedder::Clip::None)
                                                .children(parts),
                                        ),
                                    ),
                                ),
                        ),
                    ),
                )
                .into_widget()
        });

        Offstage::new()
            .child(body.into_widget())
            .offstage(!self.widget(app).is_visible)
            .into_widget()
    }
}

/// A source Rectangle with equal horizontal and vertical radii.
fn progress_rectangle(width: f64, height: f64, radius: f64, color: Color) -> WidgetRef {
    SizedBox::new()
        .width(width)
        .height(height)
        .child(
            ControlBorder::new(
                Brush::Solid(color),
                Brush::Solid(Color::from_argb(0, 0, 0, 0)),
            )
            .border_thickness(0.0)
            .corner_radius(radius),
        )
        .into_widget()
}

/// TemplateSettings.ClipRect is expressed in LayoutRoot coordinates, including its padding offset.
#[derive(Debug)]
struct ProgressClipper(Rect);

impl CustomClipper<Rect> for ProgressClipper {
    fn get_clip(&self, _: Size) -> Rect {
        self.0
    }

    fn should_reclip(&self, old: &dyn CustomClipper<Rect>) -> bool {
        old.as_any()
            .downcast_ref::<Self>()
            .is_none_or(|old| old.0 != self.0)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_precedes_pause_and_hidden_indeterminate_uses_determinate_states() {
        let bar = ProgressBar::new()
            .is_indeterminate(true)
            .show_paused(true)
            .show_error(true);
        assert_eq!(bar.visual_state(), ProgressState::IndeterminateError);
        assert_eq!(bar.is_visible(false).visual_state(), ProgressState::Error);
    }

    #[test]
    fn indeterminate_segments_follow_the_staggered_two_second_loop() {
        let sample = |time| {
            progress_visuals(
                ProgressState::Indeterminate,
                ProgressState::Determinate,
                time,
                0.0,
            )
        };
        assert_eq!(sample(0.0).first, Some(-0.4));
        assert_eq!(sample(0.75).second, Some(-0.9));
        assert!((sample(1.5).first.unwrap() - 1.2).abs() < 1e-12);
        assert_eq!(sample(2.0).first, Some(-0.4));
        assert_eq!(sample(2.0).second, Some(-0.9));
    }

    #[test]
    fn suspension_fills_the_track_and_resume_finishes_before_the_loop() {
        let pause = progress_visuals(
            ProgressState::IndeterminatePaused,
            ProgressState::Indeterminate,
            0.75,
            0.2,
        );
        assert_eq!(pause.first, None);
        assert_eq!(pause.second, Some(0.0));
        let resume = |time| {
            progress_visuals(
                ProgressState::Indeterminate,
                ProgressState::IndeterminatePaused,
                time,
                0.0,
            )
        };
        assert_eq!(resume(0.333).first, None);
        assert!((resume(0.333).second.unwrap() - 1.2).abs() < 1e-12);
        assert_eq!(resume(0.5).first, Some(-0.4));
    }
}
