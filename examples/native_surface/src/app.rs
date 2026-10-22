//! The window: a surface in the middle, the framework's label above its bottom edge.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use inset_embedder::{Color, PlatformRef, Surface, TextDirection, ViewId, ViewRef, WindowConfig};
use inset_embedder_winit::WinitEmbedder;
use inset_foundation::{App, Handle, Listenable, Listener};
use inset_painting::{BoxDecoration, EdgeInsetsGeometry, TextStyle};
use inset_scheduler::{PersistentFrameCallback, SchedulerBinding};
use inset_shell::Shell;
use inset_widgets::{
    Center, DecoratedBox, Directionality, IntoWidget, Padding, Positioned, SizedBox, Stack,
    StackFit, SurfaceView, Text, Transform, WidgetRef, Window, WindowController, run_widget,
};

use crate::producer;

/// The surface's size in logical pixels.
const SURFACE_SIZE: [f64; 2] = [400.0, 300.0];
const BACKGROUND: Color = Color::from_argb(255, 245, 245, 240);
const LABEL_BACKGROUND: Color = Color::from_argb(160, 0, 0, 0);
const LABEL: Color = Color::from_argb(255, 255, 255, 255);

/// Runs the window, the surface and its label scaled by `scale` about their middle.
pub(crate) fn run(exit_after: Option<Duration>, scale: f64) {
    WinitEmbedder
        .run(move |platform| Shell::new(platform, move |app| open_window(app, exit_after, scale)));
}

/// Asks for the window, and once its view has arrived opens the surface on it and shows both;
/// closing the window quits.
fn open_window(app: &mut App, exit_after: Option<Duration>, scale: f64) {
    let config = WindowConfig {
        title: "Inset — a native surface".to_owned(),
        size: [640.0, 480.0],
        ..WindowConfig::default()
    };
    let controller = WindowController::new(app, config);
    WindowController::on_close_requested(controller, app, Rc::new(quit));
    let shown = Cell::new(false);
    let listenable: Rc<dyn Listenable> = Rc::new(controller);
    listenable.add_listener(
        app,
        Listener::new(move |app| {
            if let Some(view) = WindowController::view(controller, app)
                && !shown.replace(true)
            {
                show(app, controller, view, exit_after, scale);
            }
        }),
    );
}

fn show(
    app: &mut App,
    controller: Handle<WindowController>,
    view: ViewRef,
    exit_after: Option<Duration>,
    scale: f64,
) {
    let surface = open_surface(&app.platform(), view.id());
    start_producer_once_sized(app, view, Arc::clone(&surface), exit_after);
    run_widget(
        app,
        Window::new(controller, home(surface, scale)).into_widget(),
    );
}

fn quit(app: &mut App, _view: ViewId) {
    if let Some(app_exit) = app.platform().app_exit() {
        app_exit.quit(0);
    }
}

/// Starts the producer at the first frame in which `view` has a size: only then does the view
/// have the display's pixel ratio the producer draws at.
fn start_producer_once_sized(
    app: &mut App,
    view: ViewRef,
    surface: Arc<dyn Surface>,
    exit_after: Option<Duration>,
) {
    let waiting = Cell::new(Some(surface));
    SchedulerBinding::add_persistent_frame_callback(
        app,
        PersistentFrameCallback::new(move |_app, _views, _elapsed| {
            let metrics = view.metrics();
            if metrics.physical_size == [0.0, 0.0] {
                return;
            }
            if let Some(surface) = waiting.take() {
                producer::start(surface, pixel_size(metrics.device_pixel_ratio), exit_after);
            }
        }),
    );
}

/// A surface of `view`'s, from the host's compositor.
fn open_surface(platform: &PlatformRef, view: ViewId) -> Arc<dyn Surface> {
    let compositor = platform
        .compositor()
        .expect("the macOS host has a compositor");
    compositor.open_surface(view, true)
}

/// The surface's size in the display's pixels, which is what the producer draws.
fn pixel_size(device_pixel_ratio: f64) -> [usize; 2] {
    SURFACE_SIZE.map(|side| (side * device_pixel_ratio).round() as usize)
}

fn home(surface: Arc<dyn Surface>, scale: f64) -> WidgetRef {
    let scaled = Transform::scale(Some(scale), None, None).child(framed_surface(surface));
    Directionality::new(
        TextDirection::Ltr,
        DecoratedBox::new(BoxDecoration::new().color(BACKGROUND))
            .child(Center::new().child(scaled)),
    )
    .into_widget()
}

/// The surface at its size, with the label over its bottom edge.
fn framed_surface(surface: Arc<dyn Surface>) -> WidgetRef {
    SizedBox::new()
        .width(SURFACE_SIZE[0])
        .height(SURFACE_SIZE[1])
        .child(
            Stack::new().fit(StackFit::Expand).children([
                SurfaceView::new(surface).into_widget(),
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
                Text::new("Inset paints this label above the surface")
                    .style(TextStyle::new().font_size(15.0).color(LABEL)),
            ),
        )
        .into_widget()
}
