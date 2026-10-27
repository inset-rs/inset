//! XAML `BrushTransition`: when a property's brush changes, the old colour fades to the new one over the transition's duration instead of snapping.

use inset_animation::{AnimationBehavior, AnimationController, ColorTween, Curves};
use inset_embedder::Color;
use inset_foundation::{App, Handle, Listener};
use inset_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use inset_widgets::*;
use std::{fmt, rc::Rc, time::Duration};

/// Builds the subtree for the colour currently shown.
pub type ColorBuilder = Rc<dyn Fn(&mut App, Color) -> WidgetRef>;

/// Animates from the colour last built to `color` over `duration` whenever `color` changes; XAML `BrushTransition` (linear, the transition names no easing).
#[derive(Clone)]
pub struct ColorTransition {
    pub color: Color,
    pub duration: Duration,
    pub builder: ColorBuilder,
}

impl ColorTransition {
    pub fn new(
        color: Color,
        duration: Duration,
        builder: impl Fn(&mut App, Color) -> WidgetRef + 'static,
    ) -> ColorTransition {
        ColorTransition {
            color,
            duration,
            builder: Rc::new(builder),
        }
    }
}

impl fmt::Debug for ColorTransition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ColorTransition")
            .field("color", &self.color)
            .field("duration", &self.duration)
            .finish_non_exhaustive()
    }
}

pub struct ColorTransitionState {
    state: StateData<ColorTransition>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    controller: Option<Handle<AnimationController>>,
    tween: Option<Handle<ColorTween>>,
    /// The colour on screen when the current transition started.
    from: Color,
}

impl StatefulWidget for ColorTransition {
    type State = ColorTransitionState;
    fn create_state(&self) -> ColorTransitionState {
        ColorTransitionState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::default(),
            controller: None,
            tween: None,
            from: self.color,
        }
    }
}

impl SingleTickerProviderStateMixin for ColorTransitionState {
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

impl TickerProviderObject for ColorTransitionState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl ColorTransitionState {
    fn controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self).controller.expect("created in init_state")
    }

    /// The colour at this frame of the transition.
    fn current(self: Handle<Self>, app: &App) -> Color {
        let controller = self.controller(app);
        let tween = app.get(self).tween.expect("created in init_state");
        tween
            .lerp(app, controller.value(app))
            .unwrap_or(self.widget(app).color)
    }
}

impl State for ColorTransitionState {
    type Widget = ColorTransition;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let (color, duration) = {
            let widget = self.widget(app);
            (widget.color, widget.duration)
        };
        let controller = AnimationController::create(
            app,
            Some(1.0),
            Some(duration),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        controller.add_listener(app, Listener::new(move |app| self.set_state(app, |_| {})));
        let tween = ColorTween::new(app, Some(color), Some(color));
        let state = app.get_mut(self);
        state.controller = Some(controller);
        state.tween = Some(tween);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &ColorTransition) {
        let color = self.widget(app).color;
        if color == old_widget.color {
            return;
        }
        let from = self.current(app);
        let tween = ColorTween::new(app, Some(from), Some(color));
        let controller = self.controller(app);
        let old = app.get_mut(self).tween.replace(tween);
        if let Some(old) = old {
            app.destroy(old.id());
        }
        app.get_mut(self).from = from;
        controller.set_value(app, 0.0);
        controller.animate_to(app, 1.0, None, Curves::linear());
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        controller.dispose(app);
        if let Some(tween) = app.get_mut(self).tween.take() {
            app.destroy(tween.id());
        }
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let color = self.current(app);
        let builder = self.widget(app).builder.clone();
        builder(app, color)
    }
}
