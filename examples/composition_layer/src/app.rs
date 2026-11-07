//! The window: a layer in the middle, the framework's label above its bottom edge, both shown
//! as the command line says.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use waft_embedder::{
    Color, CompositionLayer, PlatformRef, TextDirection, ViewId, ViewRef, WindowConfig,
};
use waft_embedder_winit::WinitEmbedder;
use waft_foundation::{App, Handle, Listenable, Listener};
use waft_painting::{BorderRadiusGeometry, BoxDecoration, EdgeInsetsGeometry, TextStyle};
use waft_scheduler::{PersistentFrameCallback, SchedulerBinding};
use waft_shell::Shell;
use waft_widgets::{
    Center, ClipRRect, CompositionLayerView, DecoratedBox, Directionality, IntoWidget, Opacity,
    Padding, Positioned, SizedBox, Stack, StackFit, Text, Transform, ViewRequestState, WidgetRef,
    Window, WindowController, run_widget,
};

use crate::producer::{self, Pacing, ViewScreen};

/// The layer's size in logical pixels.
const LAYER_SIZE: [f64; 2] = [400.0, 300.0];
const BACKGROUND: Color = Color::from_argb(255, 245, 245, 240);
const LABEL_BACKGROUND: Color = Color::from_argb(160, 0, 0, 0);
const LABEL: Color = Color::from_argb(255, 255, 255, 255);

/// How the layer and its label are shown, together.
#[derive(Clone, Copy)]
pub(crate) struct Presentation {
    /// What they are scaled by, about their middle.
    pub(crate) scale: f64,
    pub(crate) opacity: f64,
    /// How far they are turned about their middle, clockwise.
    pub(crate) turn_degrees: f64,
    /// The radius their corners are cut round by.
    pub(crate) corner_radius: f64,
}

/// Runs the window, the layer and its label shown as `presentation` says.
pub(crate) fn run(exit_after: Option<Duration>, presentation: Presentation) {
    WinitEmbedder.run(move |platform| {
        Shell::new(platform, move |app| {
            open_window(app, exit_after, presentation)
        })
    });
}

/// Asks for the window, and once its view has arrived makes the layer and shows both;
/// closing the window quits.
fn open_window(app: &mut App, exit_after: Option<Duration>, presentation: Presentation) {
    let config = WindowConfig {
        title: "Waft — a composition layer".to_owned(),
        size: [640.0, 480.0],
        ..WindowConfig::default()
    };
    let controller = WindowController::new(app, config);
    controller.on_close_requested(app, Rc::new(quit));
    let shown = Cell::new(false);
    let listenable: Rc<dyn Listenable> = Rc::new(controller);
    listenable.add_listener(
        app,
        Listener::new(move |app| {
            if let ViewRequestState::Arrived(view) = controller.request(app).state(app)
                && !shown.replace(true)
            {
                let view = Rc::clone(view);
                show(app, controller, view, exit_after, presentation);
            }
        }),
    );
}

fn show(
    app: &mut App,
    controller: Handle<WindowController>,
    view: ViewRef,
    exit_after: Option<Duration>,
    presentation: Presentation,
) {
    let layer = make_layer(&app.platform());
    start_producer_once_sized(app, view, Arc::clone(&layer), exit_after);
    run_widget(
        app,
        Window::new(controller, home(layer, presentation)).into_widget(),
    );
}

fn quit(app: &mut App, _view: ViewId) {
    if let Some(app_exit) = app.platform().app_exit() {
        app_exit.quit(0);
    }
}

/// Starts the producer at the first frame in which `view` has a size: only then does the view
/// have the display's pixel ratio the producer draws at. Each frame tells it the screen the view
/// is on, whose refreshes it paces by.
fn start_producer_once_sized(
    app: &mut App,
    view: ViewRef,
    layer: Arc<dyn CompositionLayer>,
    exit_after: Option<Duration>,
) {
    let waiter = app
        .platform()
        .vsync_waiter()
        .expect("the macOS host has a vsync waiter");
    let screen = Arc::new(ViewScreen::default());
    let waiting = Cell::new(Some(Pacing {
        layer,
        waiter,
        screen: Arc::clone(&screen),
    }));
    SchedulerBinding::add_persistent_frame_callback(
        app,
        PersistentFrameCallback::new(move |_app, _views, _elapsed| {
            let metrics = view.metrics();
            screen.set(metrics.screen);
            if metrics.physical_size == [0.0, 0.0] {
                return;
            }
            if let Some(pacing) = waiting.take() {
                producer::start(pacing, pixel_size(metrics.device_pixel_ratio), exit_after);
            }
        }),
    );
}

/// A composition layer from the host's compositor, which the window's frames place.
fn make_layer(platform: &PlatformRef) -> Arc<dyn CompositionLayer> {
    let compositor = platform
        .compositor()
        .expect("the macOS host has a compositor");
    compositor.make_layer(true)
}

/// The layer's size in the display's pixels, which is what the producer draws.
fn pixel_size(device_pixel_ratio: f64) -> [usize; 2] {
    LAYER_SIZE.map(|side| (side * device_pixel_ratio).round() as usize)
}

fn home(layer: Arc<dyn CompositionLayer>, presentation: Presentation) -> WidgetRef {
    let rounded = ClipRRect::new()
        .border_radius(BorderRadiusGeometry::circular(presentation.corner_radius))
        .child(framed_layer(layer));
    let faded = Opacity::new(presentation.opacity).child(rounded);
    let turned = Transform::rotate(presentation.turn_degrees.to_radians()).child(faded);
    let scaled = Transform::scale(Some(presentation.scale), None, None).child(turned);
    Directionality::new(
        TextDirection::Ltr,
        DecoratedBox::new(BoxDecoration::new().color(BACKGROUND))
            .child(Center::new().child(scaled)),
    )
    .into_widget()
}

/// The layer at its size, with the label over its bottom edge.
fn framed_layer(layer: Arc<dyn CompositionLayer>) -> WidgetRef {
    SizedBox::new()
        .width(LAYER_SIZE[0])
        .height(LAYER_SIZE[1])
        .child(
            Stack::new().fit(StackFit::Expand).children([
                CompositionLayerView::new(layer).into_widget(),
                Positioned::new(label())
                    .left(0.0)
                    .right(0.0)
                    .bottom(0.0)
                    .into_widget(),
            ]),
        )
        .into_widget()
}

fn label() -> WidgetRef {
    DecoratedBox::new(BoxDecoration::new().color(LABEL_BACKGROUND))
        .child(
            Padding::new(EdgeInsetsGeometry::all(8.0)).child(
                Text::new("Waft paints this label above the layer")
                    .style(TextStyle::new().font_size(15.0).color(LABEL)),
            ),
        )
        .into_widget()
}
