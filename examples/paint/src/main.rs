//! Records a display list each frame and presents it on its window's view. Below the widgets,
//! it asks the host for its window, draws once the view has arrived, and quits when the
//! window's close button is pressed.
// Proving a shared display list `Send` walks its nesting past the default limit.
#![recursion_limit = "256"]
#![feature(arbitrary_self_types)]

use std::f32::consts::TAU;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use inset_embedder::{
    Color, ColorSpace, RRect, Radius, Rect, RequestId, Scene, ViewMetrics, ViewRef, ViewRequest,
    WindowConfig,
};
use inset_embedder_winit::WinitEmbedder;
use inset_foundation::App;
use inset_painting::{Canvas, Paint, Picture, draw_rrect};
use inset_scheduler::{FrameCallback, SchedulerBinding};
use inset_shell::Shell;

fn main() {
    WinitEmbedder.run(|platform| Shell::new(platform, open_window));
}

/// Asks the host for the window, and draws into its view at every frame once it has arrived.
fn open_window(app: &mut App) {
    let request_id = app
        .platform()
        .request_view(ViewRequest {
            config: WindowConfig {
                title: "Inset — paint".to_owned(),
                size: [900.0, 600.0],
                ..WindowConfig::default()
            },
            initial_data: None,
        })
        .expect("a desktop host makes windows");
    app.platform_callbacks_mut().on_view_close_requested = Some(Rc::new(|app, _view| quit(app)));
    SchedulerBinding::add_persistent_frame_callback(
        app,
        FrameCallback::new(move |app, elapsed| {
            if let Some(view) = view_for(app, request_id) {
                present(&view, elapsed);
            }
            SchedulerBinding::schedule_frame(app);
        }),
    );
    SchedulerBinding::schedule_frame(app);
}

/// The view that arrived for `request_id`, once it has.
fn view_for(app: &App, request_id: RequestId) -> Option<ViewRef> {
    app.platform()
        .views()
        .into_iter()
        .find(|view| view.request_id() == Some(request_id))
}

fn present(view: &ViewRef, elapsed: Duration) {
    let picture = record(view.metrics(), elapsed);
    view.present(Arc::new(Scene {
        picture: Arc::new(picture),
        surfaces: Vec::new(),
    }));
}

/// The only window's close button quits.
fn quit(app: &mut App) {
    if let Some(app_exit) = app.platform().app_exit() {
        app_exit.quit(0);
    }
}

fn record(metrics: ViewMetrics, elapsed: Duration) -> Picture {
    let scale = metrics.device_pixel_ratio as f32;
    let logical_width = metrics.physical_size[0] / metrics.device_pixel_ratio;
    let logical_height = metrics.physical_size[1] / metrics.device_pixel_ratio;
    let time = elapsed.as_secs_f32();

    let mut canvas = Canvas::new();
    canvas.scale(scale, scale);

    canvas.draw_rect(
        Rect::from_ltwh(0.0, 0.0, logical_width, logical_height),
        &Paint::from_color(Color::from_argb(255, 245, 245, 240).into()),
    );
    draw_rrect(
        &mut canvas,
        RRect::from_rect_and_radius(
            Rect::from_ltwh(24.0, 24.0, 100.0, 12.0),
            Radius::circular(6.0),
        ),
        &Paint::from_color(Color::from_argb(255, 51, 51, 64).into()),
    );

    const CARDS: usize = 10;
    for index in 0..CARDS {
        let angle = time * 0.8 + index as f32 * TAU / CARDS as f32;
        canvas.save();
        canvas.translate((logical_width * 0.5) as f32, (logical_height * 0.5) as f32);
        canvas.rotate(angle);
        draw_rrect(
            &mut canvas,
            RRect::from_rect_and_radius(
                Rect::from_ltwh(90.0, -22.0, 120.0, 44.0),
                Radius::circular(10.0),
            ),
            &Paint::from_color(card_color(index).into()),
        );
        canvas.restore();
    }
    canvas.build()
}

fn card_color(index: usize) -> Color {
    let hue = index as f64 / 10.0;
    Color::from(0.9, 0.3 + 0.6 * hue, 0.5, 0.8 - 0.5 * hue, ColorSpace::Srgb)
}
