//! XAML `ProgressRing` (`controls/dev/ProgressRing/ProgressRing.xaml` and `ProgressRing.cpp`).
//! The template is nothing but an `AnimatedVisualPlayer`, so the ring itself is transcribed from
//! the two generated shape graphs the player is given: `AnimatedVisuals/ProgressRingDeterminate.cpp`
//! and `AnimatedVisuals/ProgressRingIndeterminate.cpp`.

use crate::{RangeBase, ThemeResources};
use inset_animation::{Cubic, Curve};
use inset_embedder::valo::{Cap, Point};
use inset_embedder::{Canvas, Color, Paint, PaintStyle, PathBuilder, Size, Stroke};
use inset_foundation::{App, Handle};
use inset_rendering::{BoxConstraints, CustomPainter};
use inset_scheduler::{FrameCallback, Ticker};
use inset_widgets::*;
use std::{f64::consts::TAU, time::Duration};

/// `c_durationTicks` of both generated visuals: twenty million ticks of 100 ns, so two seconds.
pub const PROGRESS_RING_DURATION: Duration = Duration::from_secs(2);

/// `ProgressRing.xaml`'s `Width` and `Height` setters.
pub const PROGRESS_RING_SIZE: f64 = 32.0;

/// `ProgressRing.xaml`'s `MinWidth` and `MinHeight` setters.
pub const PROGRESS_RING_MIN_SIZE: f64 = 16.0;

/// `ProgressRing.xaml`'s `Maximum` setter.
pub const PROGRESS_RING_DEFAULT_MAXIMUM: f64 = 100.0;

/// `ProgressRingDeterminate`'s ellipse of radius 8 under a scale of 1.77, as a fraction of the
/// 32-unit visual it was generated at.
const DETERMINATE_RADIUS: f64 = 8.0 * 1.77 / 32.0;

/// `ProgressRingDeterminate`'s stroke of 1.5 under the same scale.
const DETERMINATE_STROKE: f64 = 1.5 * 1.77 / 32.0;

/// `ProgressRingIndeterminate`'s ellipse of radius 7 under a scale of 5, as a fraction of the
/// 80-unit visual it was generated at.
const INDETERMINATE_RADIUS: f64 = 7.0 * 5.0 / 80.0;

/// `ProgressRingIndeterminate`'s stroke of 1.5 under the same scale.
const INDETERMINATE_STROKE: f64 = 1.5 * 5.0 / 80.0;

/// `CubicBezierEasingFunction_0`, shared by every eased segment of both visuals.
fn lottie_ease() -> Cubic {
    Cubic::new(0.167, 0.167, 0.833, 0.833)
}

/// How a generated animation eases the segment that ends at one of its key frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Easing {
    /// `CubicBezierEasingFunction_0`.
    Eased,
    /// A `StepEasingFunction`, which keeps the previous value until the key frame it lands on.
    Held,
}

/// One `InsertKeyFrame` call: where it lands, the value it reaches and how it is approached.
type KeyFrame = (f64, f64, Easing);

/// Reads a generated `ScalarKeyFrameAnimation` at one point of its progress. Before the first
/// key frame and after the last one the animation holds that key frame's value.
fn key_frames(progress: f64, frames: &[KeyFrame]) -> f64 {
    let (first, last) = (frames[0], frames[frames.len() - 1]);
    if progress <= first.0 {
        return first.1;
    }
    if progress >= last.0 {
        return last.1;
    }
    let index = frames.iter().position(|frame| frame.0 > progress).unwrap();
    let (from_progress, from_value, _) = frames[index - 1];
    let (to_progress, to_value, easing) = frames[index];
    match easing {
        Easing::Held => from_value,
        Easing::Eased => {
            let span = (progress - from_progress) / (to_progress - from_progress);
            from_value + (to_value - from_value) * lottie_ease().transform(span)
        }
    }
}

/// `TrimEndScalarAnimation_0_to_1` of `ProgressRingDeterminate`.
fn determinate_trim_end(progress: f64) -> f64 {
    key_frames(
        progress,
        &[
            (0.0, 0.0001, Easing::Held),
            (0.00833333377, 0.0001, Easing::Held),
            (0.25, 0.25, Easing::Eased),
            (0.5, 0.5, Easing::Eased),
            (0.75, 0.75, Easing::Eased),
            (0.983333349, 0.96666666, Easing::Eased),
            (0.991666675, 1.0, Easing::Eased),
        ],
    )
}

/// `ShapeVisibilityAnimation` of `ProgressRingDeterminate`: the arc's container is scaled to
/// nothing until the animation has begun.
fn determinate_arc_is_shown(progress: f64) -> bool {
    progress >= 0.00833333377
}

/// `RotationAngleInDegreesScalarAnimation_0_to_900` of `ProgressRingIndeterminate`, in turns.
fn indeterminate_rotation(progress: f64) -> f64 {
    key_frames(
        progress,
        &[
            (0.0, 0.0, Easing::Held),
            (0.5, 450.0, Easing::Eased),
            (1.0, 900.0, Easing::Eased),
        ],
    ) / 360.0
}

/// `TrimEndScalarAnimation_0_to_0p5` of `ProgressRingIndeterminate`: the arc that grows through
/// the first half of the loop.
fn indeterminate_growing_arc(progress: f64) -> (f64, f64) {
    let end = key_frames(
        progress,
        &[(0.0, 0.0001, Easing::Held), (0.5, 0.5, Easing::Eased)],
    );
    (0.0, end)
}

/// `TrimStartScalarAnimation_0_to_0p5` of `ProgressRingIndeterminate`: the arc whose start
/// catches up with its fixed end through the second half of the loop.
fn indeterminate_shrinking_arc(progress: f64) -> (f64, f64) {
    let start = key_frames(
        progress,
        &[
            (0.0, 0.0, Easing::Held),
            (0.5, 0.0, Easing::Held),
            (1.0, 0.5, Easing::Eased),
        ],
    );
    (start, 0.5)
}

/// XAML `ProgressRing`: a ring that either fills to a value or turns while work continues.
#[derive(Clone, Debug)]
pub struct ProgressRing {
    /// `Minimum` of the range the value is read against.
    pub minimum: f64,

    /// `Maximum`; the Fluent style sets it to 100.
    pub maximum: f64,

    /// `Value`, coerced into the range.
    pub value: f64,

    /// `IsActive`: whether the ring is shown at all.
    pub is_active: bool,

    /// `IsIndeterminate`: whether the ring turns instead of showing the value.
    pub is_indeterminate: bool,

    /// `Foreground`, in place of `ProgressRingForegroundThemeBrush`.
    pub foreground: Option<Color>,

    /// `Background`, the track behind the arc, in place of `ProgressRingBackgroundThemeBrush`.
    pub background: Option<Color>,

    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl Default for ProgressRing {
    fn default() -> Self {
        Self {
            minimum: 0.0,
            maximum: PROGRESS_RING_DEFAULT_MAXIMUM,
            value: 0.0,
            // `ProgressRing.idl` defaults both to true.
            is_active: true,
            is_indeterminate: true,
            foreground: None,
            background: None,
            key: None,
        }
    }
}

impl ProgressRing {
    /// Creates an active, indeterminate ring over the style's range.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the XAML `Minimum`.
    pub fn minimum(mut self, value: f64) -> Self {
        self.minimum = value;
        self
    }

    /// Sets the XAML `Maximum`.
    pub fn maximum(mut self, value: f64) -> Self {
        self.maximum = value;
        self
    }

    /// Sets the XAML `Value`.
    pub fn value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }

    /// Sets the XAML `IsActive`.
    pub fn is_active(mut self, value: bool) -> Self {
        self.is_active = value;
        self
    }

    /// Sets the XAML `IsIndeterminate`.
    pub fn is_indeterminate(mut self, value: bool) -> Self {
        self.is_indeterminate = value;
        self
    }

    /// Sets the XAML `Foreground`.
    pub fn foreground(mut self, value: Color) -> Self {
        self.foreground = Some(value);
        self
    }

    /// Sets the XAML `Background`.
    pub fn background(mut self, value: Color) -> Self {
        self.background = Some(value);
        self
    }

    /// Sets the identity retained across rebuilds.
    pub fn key(mut self, value: KeyRef) -> Self {
        self.key = Some(value);
        self
    }

    /// `UpdateLottieProgress`: where in the determinate animation the current value sits.
    fn progress(&self) -> f64 {
        let range = RangeBase::new(self.minimum, self.maximum, self.value).coerced();
        let span = range.maximum - range.minimum;
        if span == 0.0 {
            0.0
        } else {
            (range.value - range.minimum) / span
        }
    }
}

/// Holds the clock the player runs on, and the segment a value change is playing through.
pub struct ProgressRingState {
    /// Framework widget state.
    state: StateData<ProgressRing>,

    /// Native TickerMode support.
    single_ticker_provider: SingleTickerProviderStateMixinData,

    /// The clock both generated visuals are driven by.
    ticker: Option<Handle<Ticker>>,

    /// Time since the current segment started playing.
    elapsed: Duration,

    /// `m_oldValue` as a progress: where `PlayAsync` started the current segment.
    from: f64,

    /// Where the current segment ends.
    to: f64,
}

impl StatefulWidget for ProgressRing {
    type State = ProgressRingState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ProgressRingState {
        let progress = self.progress();
        ProgressRingState {
            state: StateData::new(),
            single_ticker_provider: Default::default(),
            ticker: None,
            elapsed: Duration::ZERO,
            from: progress,
            to: progress,
        }
    }
}

impl SingleTickerProviderStateMixin for ProgressRingState {
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

impl ProgressRingState {
    /// Where the visual the player holds currently sits, between 0 and 1.
    fn progress(self: Handle<Self>, app: &App) -> f64 {
        let state = app.get(self);
        let seconds = state.elapsed.as_secs_f64() / PROGRESS_RING_DURATION.as_secs_f64();
        if self.widget(app).is_indeterminate {
            // PlayAsync(0, 1, true) loops the whole visual.
            seconds.fract()
        } else {
            // PlayAsync(fromProgress, toProgress, false) plays that segment at normal speed.
            (state.from + seconds).min(state.to)
        }
    }

    /// `ProgressRing::UpdateStates`: the player runs while the ring is active, and stops
    /// otherwise.
    fn update_states(self: Handle<Self>, app: &mut App) {
        let ticker = app.get(self).ticker.unwrap();
        // AnimatedVisualPlayer replaces the previous playback before starting a segment.
        ticker.stop(app, false);
        app.get_mut(self).elapsed = Duration::ZERO;

        let widget = self.widget(app);
        if !widget.is_active {
            // player.Stop() returns the visual to its first frame.
            return;
        }
        if widget.is_indeterminate || app.get(self).from < app.get(self).to {
            ticker.start(app);
        }
    }

    /// `ProgressRing::UpdateLottieProgress`: a rising value plays forward from the old one, and
    /// a falling value jumps.
    fn update_progress(self: Handle<Self>, app: &mut App, old_progress: f64) {
        let target = self.widget(app).progress();
        let state = app.get_mut(self);
        state.elapsed = Duration::ZERO;
        if old_progress < target {
            state.from = old_progress;
        } else {
            state.from = target;
        }
        state.to = target;
        self.update_states(app);
    }

    /// Stops a finished determinate segment; the indeterminate visual keeps looping.
    fn tick(self: Handle<Self>, app: &mut App, elapsed: Duration) {
        self.set_state(app, |state| state.elapsed = elapsed);
        if !self.widget(app).is_indeterminate && self.progress(app) >= app.get(self).to {
            app.get(self).ticker.unwrap().stop(app, false);
        }
    }
}

impl State for ProgressRingState {
    type Widget = ProgressRing;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let ticker = SingleTickerProviderStateMixin::create_ticker(
            self,
            app,
            FrameCallback::new(move |app, elapsed| self.tick(app, elapsed)),
        );
        app.get_mut(self).ticker = Some(ticker);
        self.update_states(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &ProgressRing) {
        let widget = self.widget(app).clone();
        let switched_kind = old.is_indeterminate != widget.is_indeterminate;
        if switched_kind || (!widget.is_indeterminate && old.progress() != widget.progress()) {
            self.update_progress(app, old.progress());
        } else if old.is_active != widget.is_active {
            // Reactivation reloads the visual and seeks to the last requested value.
            if !widget.is_indeterminate {
                app.get_mut(self).from = app.get(self).to;
            }
            self.update_states(app);
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get(self).ticker.unwrap().dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let widget = self.widget(app).clone();
        let painter = ProgressRingVisual {
            progress: self.progress(app),
            indeterminate: widget.is_indeterminate,
            foreground: widget
                .foreground
                .unwrap_or(resources.progress_ring_foreground_theme_brush),
            background: widget
                .background
                .unwrap_or(resources.progress_ring_background_theme_brush),
        };
        // The Inactive state sets LayoutRoot.Opacity to 0.
        let opacity = if widget.is_active { 1.0 } else { 0.0 };
        ConstrainedBox::new(BoxConstraints {
            min_width: PROGRESS_RING_MIN_SIZE,
            max_width: f64::INFINITY,
            min_height: PROGRESS_RING_MIN_SIZE,
            max_height: f64::INFINITY,
        })
        .child(
            Opacity::new(opacity).child(
                SizedBox::new()
                    .width(PROGRESS_RING_SIZE)
                    .height(PROGRESS_RING_SIZE)
                    .child(IgnorePointer::new().child(CustomPaint::new().painter(painter))),
            ),
        )
        .into_widget()
    }
}

/// The shapes of `ProgressRingDeterminate` and `ProgressRingIndeterminate` at one point of
/// their progress.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ProgressRingVisual {
    progress: f64,
    indeterminate: bool,
    foreground: Color,
    background: Color,
}

impl ProgressRingVisual {
    /// Strokes one trimmed part of the ellipse, as a sprite shape with round caps does.
    fn arc(&self, canvas: &mut Canvas, size: Size, trim: (f64, f64), rotation: f64, color: Color) {
        if color.a == 0.0 || trim.1 <= trim.0 {
            return;
        }
        let (radius, width) = self.geometry(size);
        let mut path = PathBuilder::new();
        // Composition trims an ellipse from its start point, which is the top of the ring.
        let start = (rotation + trim.0 - 0.25) * TAU;
        path.arc(
            center(size),
            radius as f32,
            start as f32,
            ((trim.1 - trim.0) * TAU) as f32,
        );
        let mut paint = Paint::from_color(color.into());
        paint.style = PaintStyle::Stroke(Stroke {
            cap: Cap::Round,
            ..Stroke::new(width as f32)
        });
        canvas.draw_path(&path.build(), Default::default(), &paint);
    }

    /// The ring's radius and stroke width in the space it was given.
    fn geometry(&self, size: Size) -> (f64, f64) {
        let side = size.width().min(size.height());
        if self.indeterminate {
            (INDETERMINATE_RADIUS * side, INDETERMINATE_STROKE * side)
        } else {
            (DETERMINATE_RADIUS * side, DETERMINATE_STROKE * side)
        }
    }
}

/// The centre both generated visuals place their ellipse at.
fn center(size: Size) -> Point {
    Point::new((size.width() / 2.0) as f32, (size.height() / 2.0) as f32)
}

impl CustomPainter for ProgressRingVisual {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        if size.is_empty() {
            return;
        }

        // AnimatedVisualPlayer uses Stretch.Fill, including the stroke and round caps.
        canvas.save();
        canvas.scale((size.width() / 32.0) as f32, (size.height() / 32.0) as f32);
        let size = Size::new(32.0, 32.0);

        // SpriteShape_0 of both visuals: the untrimmed track, in the Background colour.
        self.arc(canvas, size, (0.0, 1.0), 0.0, self.background);
        if self.indeterminate {
            let rotation = indeterminate_rotation(self.progress);
            // The two arcs cross-fade at the halfway step of their opacity animations.
            if self.progress < 0.5 {
                let trim = indeterminate_growing_arc(self.progress);
                self.arc(canvas, size, trim, rotation, self.foreground);
            } else {
                let trim = indeterminate_shrinking_arc(self.progress);
                self.arc(canvas, size, trim, rotation, self.foreground);
            }
        } else if determinate_arc_is_shown(self.progress) {
            let trim = (0.0, determinate_trim_end(self.progress));
            self.arc(canvas, size, trim, 0.0, self.foreground);
        }
        canvas.restore();
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        old_delegate.as_any().downcast_ref::<Self>() != Some(self)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
