//! The window, and the view the app adds on a layer it places in it: the window's content takes the
//! clicks where the layer is and forwards them to the added view, which counts them. Both arrive as
//! every view does and the app's one builder builds each from its data: the window, which carries
//! none, gets the window's content, and the added view, which carries [`COUNTER`], the counter.

use std::cell::Cell;
use std::hash::{BuildHasher, RandomState};
use std::rc::Rc;
use std::sync::Arc;

use waft_embedder::{
    Color, CompositionLayer, Contact, Matrix4, Mutator, Placement, PointerReport, Rect,
    ReportedChange, SystemMouseCursorKind, TextDirection, TextInputHost, ViewConstraints,
    ViewFocusDirection, ViewFocusState, ViewHost, ViewMetrics, ViewRef, WindowConfig, WindowRef,
};
use waft_embedder_winit::{LinuxSettings, WinitEmbedder, WinitPlatform};
use waft_foundation::{App, Handle, Listener, ValueListenable, ValueNotifier};
use waft_gestures::PointerEvent;
use waft_painting::{BoxDecoration, TextStyle};
use waft_rendering::HitTestBehavior;
use waft_scheduler::{PersistentFrameCallback, SchedulerBinding};
use waft_shell::Shell;
use waft_widgets::{
    BuildContext, Center, DecoratedBox, Directionality, GestureDetector, IntoWidget, MultiViewApp,
    Positioned, SizedBox, Stack, State, StateData, StatefulWidget, Text, ValueListenableBuilder,
    View, WidgetRef, WindowScope, run_widget,
};

use crate::window_layer;

/// The room around the added view in the window, and the added view's height, in logical pixels:
/// it is as wide as the window less the room on either side.
const MARGIN: f64 = 32.0;
const ADDED_HEIGHT: f64 = 160.0;
const WINDOW_BACKGROUND: Color = Color::from_argb(255, 236, 236, 230);
const ADDED_BACKGROUND: Color = Color::from_argb(255, 40, 80, 140);
const ADDED_TEXT: Color = Color::from_argb(255, 255, 255, 255);
const CAPTION: Color = Color::from_argb(255, 60, 60, 60);
/// What the app counts on of the macOS host, which embeds views on layers.
const EMBEDS_VIEWS: &str = "the macOS host has a view embedder";
/// The initial data of the view the app adds on its layer, which says it is the counter.
const COUNTER: &str = "counter";

/// Runs the window and the view the app adds in it; closing the window quits.
pub(crate) fn run() {
    WinitEmbedder::default()
        .linux(LinuxSettings {
            app_id: Some(waft_macros::linux_app_id!().to_owned()),
        })
        .run(|platform| {
            Shell::new(platform, |app| {
                let config = WindowConfig {
                    title: "Waft — a view on a layer".to_owned(),
                    size: [480.0, 300.0],
                    ..WindowConfig::default()
                };
                app.platform()
                    .request_view(config, None)
                    .expect("the macOS host makes windows");
                let clicks = app.create(ValueNotifier::new(0));
                let windows =
                    MultiViewApp::new(move |_app, view| match view.initial_data().as_deref() {
                        None => Some(Home.into_widget()),
                        Some(COUNTER) => Some(click_counter(clicks)),
                        Some(_) => None,
                    })
                    .on_close_requested(Rc::new(quit));
                run_widget(app, windows.into_widget());
            })
        });
}

/// The window's content, which adds the counter's view on a layer it places in its window, under
/// its own drawing, and forwards the presses over it there.
#[derive(Debug)]
struct Home;

struct HomeState {
    state: StateData<Home>,
    /// What forwards the window's presses to the counter's view, from the content's first build
    /// in its window on, once the view is asked for.
    forwarding: Option<Rc<Forwarding>>,
}

impl StatefulWidget for Home {
    type State = HomeState;

    fn create_state(&self) -> HomeState {
        HomeState {
            state: StateData::new(),
            forwarding: None,
        }
    }
}

impl State for HomeState {
    type Widget = Home;
    waft_widgets::state_accessors!();

    /// Asks for the counter's view, once, on a layer placed in the window the content is in.
    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        if app.get(self).forwarding.is_some() {
            return;
        }
        let context = self.context(app);
        let window_view = View::of(app, context);
        let window = WindowScope::of(app, context);
        add_counter_in(app, window_view, &window);
        app.get_mut(self).forwarding = Some(Rc::new(Forwarding::new()));
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let forwarding = app
            .get(self)
            .forwarding
            .clone()
            .expect("asked for as the content came");
        window_content(forwarding)
    }
}

/// Asks for the counter's view on a layer placed in `window`, `window_view`'s; it arrives as every
/// view does.
fn add_counter_in(app: &mut App, window_view: ViewRef, window: &WindowRef) {
    let layer = app
        .platform()
        .compositor()
        .expect("the macOS host has a compositor")
        .make_layer(true);
    let metrics = added_metrics(&window_view.metrics());
    place_in_window(app, window, &layer, &metrics);
    let host = Rc::new(ThroughWindow(Rc::clone(&window_view)));
    let platform = app.platform();
    let embedder = platform.view_embedder().expect(EMBEDS_VIEWS);
    embedder
        .add_view(&layer, metrics, host, Some(COUNTER.to_owned()))
        .expect("the macOS host adds a view on a layer of its compositor's");
    follow_window(app, window_view, layer, metrics);
}

/// The counter's view, once it has arrived.
fn counter_view(app: &App) -> Option<ViewRef> {
    app.platform()
        .views()
        .into_iter()
        .find(|view| view.initial_data().as_deref() == Some(COUNTER))
}

fn quit(app: &mut App, _view: &ViewRef) {
    if let Some(app_exit) = app.platform().app_exit() {
        app_exit.quit(0);
    }
}

/// The added view's metrics in a window of `window` metrics: as wide as the window less the room
/// on either side, at the window's scale and on its screen.
fn added_metrics(window: &ViewMetrics) -> ViewMetrics {
    let scale = window.device_pixel_ratio;
    let width = (window.physical_size[0] - 2.0 * MARGIN * scale).max(0.0);
    let height = ADDED_HEIGHT * scale;
    ViewMetrics {
        physical_size: [width, height],
        physical_constraints: ViewConstraints::tight(width, height),
        device_pixel_ratio: scale,
        screen: window.screen,
        ..ViewMetrics::default()
    }
}

/// Places `layer` in `window`, above the window's own drawing, `MARGIN` from its top-left corner,
/// at the size of the added view's `metrics`: in its logical pixels, under a transform by its
/// scale, as a view's scene places a layer.
fn place_in_window(
    app: &App,
    window: &WindowRef,
    layer: &Arc<dyn CompositionLayer>,
    metrics: &ViewMetrics,
) {
    let Some(parent) = window_layer::of(window) else {
        return;
    };
    let scale = metrics.device_pixel_ratio;
    let [width, height] = metrics.physical_size.map(|side| side / scale);
    let placement = Placement {
        rect: Rect::from_ltwh(MARGIN, MARGIN, width, height),
        mutators: vec![Mutator::Transform(Matrix4::scale(
            scale as f32,
            scale as f32,
        ))],
    };
    app.platform()
        .downcast_ref::<WinitPlatform>()
        .and_then(WinitPlatform::macos)
        .expect("the macOS host")
        .place_layer_in(&parent, layer, &placement, scale);
}

/// At each frame after the window resizes or moves to another screen, places the counter's layer
/// again and gives its view the metrics it now has; `placed_for` are the metrics the layer was
/// placed for last.
fn follow_window(
    app: &mut App,
    window_view: ViewRef,
    layer: Arc<dyn CompositionLayer>,
    placed_for: ViewMetrics,
) {
    let placed_for = Cell::new(placed_for);
    SchedulerBinding::add_persistent_frame_callback(
        app,
        PersistentFrameCallback::new(move |app, _views, _elapsed| {
            let metrics = added_metrics(&window_view.metrics());
            if placed_for.replace(metrics) != metrics
                && let Some(window) = window_view.window()
            {
                place_in_window(app, &window, &layer, &metrics);
            }
            if let Some(view) = counter_view(app)
                && view.metrics() != metrics
            {
                let platform = app.platform();
                let embedder = platform.view_embedder().expect(EMBEDS_VIEWS);
                embedder.set_view_metrics(view.id(), metrics);
            }
        }),
    );
}

/// What the added view asks of the system, as the app answers it: its cursor is the window's,
/// since the pointer over the view is over the window; it edits no text; and the app keeps the
/// focus where it is.
struct ThroughWindow(ViewRef);

impl ViewHost for ThroughWindow {
    fn text_input(&self) -> Option<&dyn TextInputHost> {
        None
    }

    fn activate_system_cursor(&self, device: i64, kind: SystemMouseCursorKind) {
        self.0.host().activate_system_cursor(device, kind);
    }

    fn request_focus_change(&self, _state: ViewFocusState, _direction: ViewFocusDirection) {}
}

/// Forwards to the added view what the window's pointers do over it, as a host reports them: in
/// the view's physical pixels, each device under a number of its own, random for each run, which
/// none of the host's own devices takes.
struct Forwarding {
    device_numbers: RandomState,
}

impl Forwarding {
    fn new() -> Forwarding {
        Forwarding {
            device_numbers: RandomState::new(),
        }
    }

    /// Forwards `event`, a press, move, release or cancel the window took over the counter's
    /// view, once the view has arrived.
    fn forward(&self, app: &mut App, event: PointerEvent) {
        let Some(view) = counter_view(app) else {
            return;
        };
        let Some(change) = reported_change(&event) else {
            return;
        };
        let scale = view.metrics().device_pixel_ratio;
        let at = event.local_position();
        let report = PointerReport {
            view_id: view.id(),
            time_stamp: event.time_stamp(),
            device: self.device_numbers.hash_one(event.device()) as i64,
            kind: event.kind(),
            position: [at.dx() * scale, at.dy() * scale],
            change,
            contact: Contact::default(),
        };
        let platform = app.platform();
        let embedder = platform.view_embedder().expect(EMBEDS_VIEWS);
        embedder.send_pointer_reports(vec![report]);
    }
}

/// The change a host reports for `event`, a press, move, release or cancel.
fn reported_change(event: &PointerEvent) -> Option<ReportedChange> {
    let buttons = event.buttons();
    match event {
        PointerEvent::Down(_) => Some(ReportedChange::Down { buttons }),
        PointerEvent::Move(_) => Some(ReportedChange::Move { buttons }),
        PointerEvent::Up(_) => Some(ReportedChange::Up { buttons }),
        PointerEvent::Cancel(_) => Some(ReportedChange::Cancel { buttons }),
        _ => None,
    }
}

/// The window's content: a caption under the place of the added view, and over that place what
/// takes the pointer's presses, moves and releases there and forwards them.
fn window_content(forwarding: Rc<Forwarding>) -> WidgetRef {
    let over_added = Positioned::new(forwarded_presses(forwarding))
        .left(MARGIN)
        .top(MARGIN)
        .right(MARGIN)
        .height(ADDED_HEIGHT);
    let caption = Text::new("The view above is the app's own, on a layer it placed here")
        .style(TextStyle::new().font_size(14.0).color(CAPTION));
    let under_added = Positioned::new(caption)
        .left(MARGIN)
        .right(MARGIN)
        .top(MARGIN + ADDED_HEIGHT + 12.0);
    Directionality::new(
        TextDirection::Ltr,
        DecoratedBox::new(BoxDecoration::new().color(WINDOW_BACKGROUND))
            .child(Stack::new().children([over_added.into_widget(), under_added.into_widget()])),
    )
    .into_widget()
}

fn forwarded_presses(forwarding: Rc<Forwarding>) -> WidgetRef {
    let (down, moved, up) = (
        Rc::clone(&forwarding),
        Rc::clone(&forwarding),
        Rc::clone(&forwarding),
    );
    waft_widgets::Listener::new()
        .behavior(HitTestBehavior::Opaque)
        .on_pointer_down(Rc::new(move |app, event| {
            down.forward(app, PointerEvent::Down(event));
        }))
        .on_pointer_move(Rc::new(move |app, event| {
            moved.forward(app, PointerEvent::Move(event));
        }))
        .on_pointer_up(Rc::new(move |app, event| {
            up.forward(app, PointerEvent::Up(event));
        }))
        .on_pointer_cancel(Rc::new(move |app, event| {
            forwarding.forward(app, PointerEvent::Cancel(event));
        }))
        .child(SizedBox::new())
        .into_widget()
}

/// What the counter's view shows: how many clicks it took, which the window forwards.
fn click_counter(clicks: Handle<ValueNotifier<u32>>) -> WidgetRef {
    let count: Rc<dyn ValueListenable<u32>> = Rc::new(clicks);
    let label = ValueListenableBuilder::new(count, |_app, _context, count, _child| {
        Text::new(format!("A view the app added: {count} clicks forwarded"))
            .style(TextStyle::new().font_size(17.0).color(ADDED_TEXT))
            .into_widget()
    });
    let counting = GestureDetector::new()
        .on_tap(Listener::new(move |app| {
            let count = *clicks.value(app);
            clicks.set_value(app, count + 1);
        }))
        .child(
            DecoratedBox::new(BoxDecoration::new().color(ADDED_BACKGROUND))
                .child(Center::new().child(label)),
        );
    Directionality::new(TextDirection::Ltr, counting).into_widget()
}
