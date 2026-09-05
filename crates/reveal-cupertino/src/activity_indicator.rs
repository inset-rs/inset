//! Flutter counterpart: `cupertino/activity_indicator.dart`.

use std::any::Any;
use std::f64::consts::PI;
use std::rc::Rc;
use std::time::Duration;

use reveal_animation::{Animation, AnimationBehavior, AnimationController, AnyAnimation};
use reveal_embedder::{Canvas, Color, Offset, Paint, RRect, Radius, Size, clamp_double};
use reveal_foundation::{App, Handle, Listenable};
use reveal_painting::{AnyColor, BorderRadius, draw_rrect};
use reveal_rendering::{BoxConstraints, CustomPainter};
use reveal_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use reveal_widgets::{
    BuildContext, ConstrainedBox, CustomPaint, IntoWidget, KeyRef, SingleTickerProviderStateMixin,
    SingleTickerProviderStateMixinData, SizedBox, State, StateData, StatefulWidget,
    StatelessWidget, WidgetRef,
};

use crate::colors::{CupertinoColors, CupertinoDynamicColor};

const K_DEFAULT_INDICATOR_RADIUS: f64 = 10.0;

// Extracted from iOS 13.2 Beta.
const K_ACTIVE_TICK_COLOR_DYNAMIC: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0xFF3C3C44), Color::new(0xFFEBEBF5));
const K_ACTIVE_TICK_COLOR: AnyColor = K_ACTIVE_TICK_COLOR_DYNAMIC.to_any();

/// An iOS-style activity indicator that spins clockwise.
///
/// See also:
///
///  * [`CupertinoLinearActivityIndicator`], which displays progress along a line.
///  * <https://developer.apple.com/design/human-interface-guidelines/progress-indicators/>
#[derive(Debug)]
pub struct CupertinoActivityIndicator {
    pub key: Option<KeyRef>,
    /// Color of the activity indicator.
    ///
    /// Defaults to color extracted from native iOS.
    pub color: Option<AnyColor>,
    /// Whether the activity indicator is running its animation.
    ///
    /// Defaults to true.
    pub animating: bool,
    /// Radius of the spinner widget.
    ///
    /// Defaults to 10 pixels. Must be positive.
    pub radius: f64,
    /// Determines the percentage of spinner ticks that will be shown. Typical usage would
    /// display all ticks, however, this allows for more fine-grained control such as
    /// during pull-to-refresh when the drag-down action shows one tick at a time as
    /// the user continues to drag down.
    ///
    /// Defaults to one. Must be between zero and one, inclusive.
    pub progress: f64,
}

impl CupertinoActivityIndicator {
    /// Creates an iOS-style activity indicator that spins clockwise.
    pub fn new() -> CupertinoActivityIndicator {
        CupertinoActivityIndicator::default()
    }

    /// Creates a non-animated iOS-style activity indicator that displays
    /// a partial count of ticks based on the value of [`progress`](Self::progress).
    ///
    /// When provided, the value of [`progress`](Self::progress) must be between 0.0 (zero
    /// ticks will be shown) and 1.0 (all ticks will be shown) inclusive. Defaults to 1.0.
    pub fn partially_revealed() -> CupertinoActivityIndicator {
        CupertinoActivityIndicator {
            animating: false,
            ..CupertinoActivityIndicator::default()
        }
    }

    /// Dart `CupertinoActivityIndicator(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoActivityIndicator {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoActivityIndicator(color:)`.
    pub fn color(mut self, color: impl Into<AnyColor>) -> CupertinoActivityIndicator {
        self.color = Some(color.into());
        self
    }

    /// Dart `CupertinoActivityIndicator(animating:)`.
    pub fn animating(mut self, animating: bool) -> CupertinoActivityIndicator {
        self.animating = animating;
        self
    }

    /// Dart `CupertinoActivityIndicator(radius:)`.
    pub fn radius(mut self, radius: f64) -> CupertinoActivityIndicator {
        debug_assert!(radius > 0.0);
        self.radius = radius;
        self
    }

    /// Dart `CupertinoActivityIndicator.partiallyRevealed(progress:)`.
    pub fn progress(mut self, progress: f64) -> CupertinoActivityIndicator {
        debug_assert!(progress >= 0.0);
        debug_assert!(progress <= 1.0);
        self.progress = progress;
        self
    }
}

impl Default for CupertinoActivityIndicator {
    fn default() -> CupertinoActivityIndicator {
        CupertinoActivityIndicator {
            key: None,
            color: None,
            animating: true,
            radius: K_DEFAULT_INDICATOR_RADIUS,
            progress: 1.0,
        }
    }
}

impl StatefulWidget for CupertinoActivityIndicator {
    type State = CupertinoActivityIndicatorState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoActivityIndicatorState {
        CupertinoActivityIndicatorState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::default(),
            controller: None,
        }
    }
}

/// Dart's `_CupertinoActivityIndicatorState`.
pub struct CupertinoActivityIndicatorState {
    state: StateData<CupertinoActivityIndicator>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    controller: Option<Handle<AnimationController>>,
}

impl CupertinoActivityIndicatorState {
    fn controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self).controller.expect("created in init_state")
    }
}

impl SingleTickerProviderStateMixin for CupertinoActivityIndicatorState {
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
impl TickerProviderObject for CupertinoActivityIndicatorState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for CupertinoActivityIndicatorState {
    type Widget = CupertinoActivityIndicator;
    reveal_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = AnimationController::create(
            app,
            None,
            Some(Duration::from_secs(1)),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        app.get_mut(self).controller = Some(controller);

        if self.widget(app).animating {
            controller.repeat(app, None, None, false, None, None);
        }
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &CupertinoActivityIndicator,
    ) {
        if self.widget(app).animating != old_widget.animating {
            let controller = self.controller(app);
            if self.widget(app).animating {
                controller.repeat(app, None, None, false, None, None);
            } else {
                controller.stop(app, true);
            }
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        controller.dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
        app.get_mut(self).controller = None;
        app.destroy(controller);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app);
        let (color, radius, progress) = (widget.color.clone(), widget.radius, widget.progress);
        let active_color = match color {
            Some(color) => color.color(),
            None => CupertinoDynamicColor::resolve(&K_ACTIVE_TICK_COLOR, app, context).color(),
        };
        let position = self.controller(app).as_animation();
        SizedBox::square(Some(radius * 2.0))
            .child(
                CustomPaint::new().painter(CupertinoActivityIndicatorPainter::new(
                    position,
                    active_color,
                    radius,
                    progress,
                )),
            )
            .into_widget()
    }
}

const K_TWO_PI: f64 = PI * 2.0;

/// Alpha values extracted from the native component (for both dark and light mode) to
/// draw the spinning ticks.
const K_ALPHA_VALUES: [i32; 8] = [47, 47, 47, 47, 72, 97, 122, 147];

/// The alpha value that is used to draw the partially revealed ticks.
const PARTIALLY_REVEALED_ALPHA: i32 = 147;

/// Dart's `_CupertinoActivityIndicatorPainter`.
struct CupertinoActivityIndicatorPainter {
    position: AnyAnimation<f64>,
    active_color: Color,
    #[expect(
        dead_code,
        reason = "Dart's field; only the constructor reads it, to size the tick"
    )]
    radius: f64,
    progress: f64,
    // Use a RRect instead of RSuperellipse since this shape is really small
    // and should make little visual difference.
    tick_fundamental_shape: RRect,
    /// [`position`](Self::position) again, as the type Dart's `repaint:` argument takes.
    repaint: Rc<dyn Listenable>,
}

impl CupertinoActivityIndicatorPainter {
    fn new(
        position: AnyAnimation<f64>,
        active_color: Color,
        radius: f64,
        progress: f64,
    ) -> CupertinoActivityIndicatorPainter {
        CupertinoActivityIndicatorPainter {
            repaint: Rc::new(position) as Rc<dyn Listenable>,
            position,
            active_color,
            radius,
            progress,
            tick_fundamental_shape: RRect::from_ltrbxy(
                -radius / K_DEFAULT_INDICATOR_RADIUS,
                -radius / 3.0,
                radius / K_DEFAULT_INDICATOR_RADIUS,
                -radius,
                radius / K_DEFAULT_INDICATOR_RADIUS,
                radius / K_DEFAULT_INDICATOR_RADIUS,
            ),
        }
    }
}

impl CustomPainter for CupertinoActivityIndicatorPainter {
    fn repaint(&self) -> Option<&Rc<dyn Listenable>> {
        Some(&self.repaint)
    }

    fn paint(&self, app: &mut App, canvas: &mut Canvas, size: Size) {
        let mut paint = Paint::default();
        let tick_count = K_ALPHA_VALUES.len() as i64;

        canvas.save();
        canvas.translate((size.width() / 2.0) as f32, (size.height() / 2.0) as f32);

        let active_tick = (tick_count as f64 * self.position.value(app)).floor() as i64;

        let mut i: i64 = 0;
        while (i as f64) < tick_count as f64 * self.progress {
            let t = (i - active_tick).rem_euclid(tick_count);
            paint.color = self
                .active_color
                .with_alpha(if self.progress < 1.0 {
                    PARTIALLY_REVEALED_ALPHA
                } else {
                    K_ALPHA_VALUES[t as usize]
                })
                .into();
            draw_rrect(canvas, self.tick_fundamental_shape, &paint);
            canvas.rotate((K_TWO_PI / tick_count as f64) as f32);
            i += 1;
        }

        canvas.restore();
    }

    fn should_repaint(&self, _app: &App, old_painter: &dyn CustomPainter) -> bool {
        let old_painter = old_painter
            .as_any()
            .downcast_ref::<CupertinoActivityIndicatorPainter>()
            .expect("Dart's covariant parameter");
        old_painter.position != self.position
            || old_painter.active_color != self.active_color
            || old_painter.progress != self.progress
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An iOS-style linear activity indicator.
///
/// The [`CupertinoLinearActivityIndicator`] is a linear progress bar that
/// displays a colored bar to indicate the progress of an ongoing task.
///
/// See also:
///
///  * [`CupertinoActivityIndicator`], which is an iOS-style activity indicator that spins
///    clockwise.
///  * <https://developer.apple.com/design/human-interface-guidelines/progress-indicators/>
#[derive(Debug)]
pub struct CupertinoLinearActivityIndicator {
    pub key: Option<KeyRef>,
    /// The current progress of the linear activity indicator.
    ///
    /// This value must be between 0.0 and 1.0. A value of 0.0 means no progress
    /// and 1.0 means that progress is complete.
    pub progress: f64,
    /// The height of the line used to draw the linear activity indicator.
    ///
    /// Defaults to 4.5 units. Must be positive.
    pub height: f64,
    /// The color of the progress bar.
    ///
    /// This color represents the portion of the bar that indicates progress.
    ///
    /// Defaults to [`CupertinoColors::ACTIVE_BLUE`] if no color is specified.
    pub color: Option<AnyColor>,
}

impl CupertinoLinearActivityIndicator {
    /// Creates a linear iOS-style activity indicator.
    pub fn new(progress: f64) -> CupertinoLinearActivityIndicator {
        debug_assert!((0.0..=1.0).contains(&progress));
        CupertinoLinearActivityIndicator {
            key: None,
            progress,
            height: 4.5,
            color: None,
        }
    }

    /// Dart `CupertinoLinearActivityIndicator(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoLinearActivityIndicator {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoLinearActivityIndicator(height:)`.
    pub fn height(mut self, height: f64) -> CupertinoLinearActivityIndicator {
        debug_assert!(height > 0.0);
        self.height = height;
        self
    }

    /// Dart `CupertinoLinearActivityIndicator(color:)`.
    pub fn color(mut self, color: impl Into<AnyColor>) -> CupertinoLinearActivityIndicator {
        self.color = Some(color.into());
        self
    }
}

impl StatelessWidget for CupertinoLinearActivityIndicator {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        ConstrainedBox::new(
            BoxConstraints::new()
                .min_height(self.height)
                .min_width(f64::INFINITY),
        )
        .child(
            CustomPaint::new().painter(CupertinoLinearActivityIndicatorPainter::new(
                self.progress,
                self.color.clone(),
            )),
        )
        .into_widget()
    }
}

/// Dart's `_CupertinoLinearActivityIndicator`; the public widget owns that name here.
struct CupertinoLinearActivityIndicatorPainter {
    progress: f64,
    color: Option<AnyColor>,
    /// The background paint used to draw the full width of the progress bar.
    ///
    /// This paint object is created once and reused to fill the background
    /// with a system fill color.
    background_paint: Paint,
    /// The paint used to draw the progress portion of the progress bar.
    ///
    /// This paint object is created once and reused to fill the progress area.
    progress_paint: Paint,
}

impl CupertinoLinearActivityIndicatorPainter {
    fn new(progress: f64, color: Option<AnyColor>) -> CupertinoLinearActivityIndicatorPainter {
        let progress_color = color
            .clone()
            .unwrap_or(CupertinoColors::ACTIVE_BLUE)
            .color();
        CupertinoLinearActivityIndicatorPainter {
            progress,
            color,
            background_paint: Paint::from_color(CupertinoColors::SYSTEM_FILL.color().into()),
            progress_paint: Paint::from_color(progress_color.into()),
        }
    }
}

impl CustomPainter for CupertinoLinearActivityIndicatorPainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        // Draw the background of the progress bar.
        draw_rrect(
            canvas,
            BorderRadius::all(Radius::circular(size.height() / 2.0)).to_rrect(Offset::ZERO & size),
            &self.background_paint,
        );

        // Draw the progress portion of the bar.
        if self.progress > 0.0 {
            draw_rrect(
                canvas,
                BorderRadius::all(Radius::circular(size.height() / 2.0)).to_rrect(
                    Offset::ZERO
                        & Size::new(
                            clamp_double(self.progress, 0.0, 1.0) * size.width(),
                            size.height(),
                        ),
                ),
                &self.progress_paint,
            );
        }
    }

    fn should_repaint(&self, _app: &App, old_painter: &dyn CustomPainter) -> bool {
        let old_painter = old_painter
            .as_any()
            .downcast_ref::<CupertinoLinearActivityIndicatorPainter>()
            .expect("Dart's covariant parameter");
        old_painter.progress != self.progress || old_painter.color != self.color
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use reveal_embedder::valo::Op;
    use reveal_painting::AlignmentGeometry;
    use reveal_rendering::{
        AnyRenderObject, RenderBox, RenderCustomPaint, RenderHandle, RenderObject,
    };
    use reveal_widgets::{Align, GlobalKey};

    use super::*;
    use crate::test_support::{app, build, pump};

    /// The first descendant render object of type `T`, from `node` down.
    fn find<T: RenderObject>(app: &App, node: AnyRenderObject) -> Option<RenderHandle<T>> {
        if let Some(found) = node.downcast::<T>(app) {
            return Some(found);
        }
        let mut found = None;
        node.visit_children(app, &mut |child| {
            if found.is_none() {
                found = find::<T>(app, child);
            }
        });
        found
    }

    /// The painter the `CustomPaint` under `key` holds, and the size it paints into.
    fn painter_of(app: &mut App, key: &GlobalKey) -> (Rc<dyn CustomPainter>, Size) {
        let root = key
            .current_context(app)
            .expect("the indicator mounted")
            .find_render_object(app)
            .expect("the indicator has been laid out");
        let custom = find::<RenderCustomPaint>(app, root).expect("a CustomPaint under it");
        (custom.painter(app).expect("a painter"), custom.size(app))
    }

    /// Records the painter and reads back one number from every fill, in paint order.
    fn drawn<T>(
        app: &mut App,
        painter: &dyn CustomPainter,
        size: Size,
        of: impl Fn(&reveal_embedder::Paint, reveal_embedder::valo::Rect) -> T,
    ) -> Vec<T> {
        let mut canvas = Canvas::new();
        painter.paint(app, &mut canvas, size);
        canvas
            .build()
            .ops()
            .iter()
            .filter_map(|op| match op {
                Op::DrawPath { paint, bounds, .. } => Some(of(paint, *bounds)),
                _ => None,
            })
            .collect()
    }

    fn drawn_alphas(app: &mut App, painter: &dyn CustomPainter, size: Size) -> Vec<i32> {
        drawn(app, painter, size, |paint, _| {
            (paint.color.a * 255.0).round() as i32
        })
    }

    fn key_of(key: &GlobalKey) -> KeyRef {
        Rc::new(key.clone())
    }

    /// Mounts `child` where it may take its own size; the view's root is tight.
    fn loosely<K>(app: &mut App, child: impl IntoWidget<K>) {
        build(
            app,
            Align::new()
                .alignment(AlignmentGeometry::TOP_LEFT)
                .child(child)
                .into_widget(),
        );
    }

    #[test]
    fn a_spinning_indicator_paints_its_eight_ticks_and_rotates_the_alpha_table() {
        let mut app = app();
        let global_key = GlobalKey::new();
        loosely(
            &mut app,
            CupertinoActivityIndicator::new().key(key_of(&global_key)),
        );

        let (painter, size) = painter_of(&mut app, &global_key);
        assert_eq!(size, Size::new(20.0, 20.0), "the radius doubled");
        assert_eq!(
            drawn_alphas(&mut app, &*painter, size),
            K_ALPHA_VALUES.to_vec()
        );

        pump(&mut app, Duration::from_millis(500));
        let state = global_key
            .current_state::<CupertinoActivityIndicatorState>(&mut app)
            .expect("the indicator mounted");
        assert_eq!(
            state.controller(&app).value(&app),
            0.5,
            "half of the one-second period"
        );
        assert_eq!(
            drawn_alphas(&mut app, &*painter, size),
            vec![72, 97, 122, 147, 47, 47, 47, 47],
            "the same painter, its table rotated by four ticks"
        );
    }

    #[test]
    fn an_indicator_that_is_not_animating_never_advances() {
        let mut app = app();
        let global_key = GlobalKey::new();
        loosely(
            &mut app,
            CupertinoActivityIndicator::new()
                .animating(false)
                .key(key_of(&global_key)),
        );
        let state = global_key
            .current_state::<CupertinoActivityIndicatorState>(&mut app)
            .expect("the indicator mounted");
        let controller = state.controller(&app);
        assert!(!controller.is_animating(&mut app));

        let (painter, size) = painter_of(&mut app, &global_key);
        pump(&mut app, Duration::from_millis(500));
        assert_eq!(controller.value(&app), 0.0);
        assert_eq!(
            drawn_alphas(&mut app, &*painter, size),
            K_ALPHA_VALUES.to_vec()
        );
    }

    #[test]
    fn a_partially_revealed_indicator_draws_a_fraction_of_its_ticks() {
        let mut app = app();
        let global_key = GlobalKey::new();
        let indicator = CupertinoActivityIndicator::partially_revealed().progress(0.5);
        assert!(!indicator.animating);
        loosely(&mut app, indicator.key(key_of(&global_key)));

        let (painter, size) = painter_of(&mut app, &global_key);
        assert_eq!(
            drawn_alphas(&mut app, &*painter, size),
            vec![PARTIALLY_REVEALED_ALPHA; 4]
        );
    }

    #[test]
    fn the_linear_indicator_fills_its_track_up_to_the_progress() {
        let mut app = app();
        let global_key = GlobalKey::new();
        loosely(
            &mut app,
            SizedBox::new()
                .width(200.0)
                .height(10.0)
                .child(CupertinoLinearActivityIndicator::new(0.25).key(key_of(&global_key))),
        );

        let (painter, size) = painter_of(&mut app, &global_key);
        assert_eq!(size, Size::new(200.0, 10.0));
        assert_eq!(
            drawn(&mut app, &*painter, size, |_, bounds| bounds.width),
            vec![200.0, 50.0],
            "the track, then a quarter of it"
        );
    }
}
