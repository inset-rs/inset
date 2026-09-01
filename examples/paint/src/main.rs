//! Records a display list each frame and presents it on the implicit view.

use std::f32::consts::TAU;
use std::time::Duration;

use reveal_embedder::ViewMetrics;
use reveal_embedder_winit::{ImplicitViewConfig, WinitEmbedder};
use reveal_geometry::{Color, ColorSpace, RRect, Radius, Rect};
use reveal_painting::{Canvas, Paint, Picture, draw_rrect};
use reveal_scheduler::{FrameCallback, SchedulerBinding, Shell};

fn main() {
    WinitEmbedder {
        implicit_view: Some(ImplicitViewConfig {
            title: "reveal — paint".to_owned(),
            logical_size: [900.0, 600.0],
        }),
    }
    .run(|platform| {
        Shell::new(platform, |app| {
            SchedulerBinding::add_persistent_frame_callback(
                app,
                FrameCallback::new(|app, elapsed| {
                    let Some(view) = app.platform().implicit_view() else {
                        return;
                    };
                    let picture = record(view.metrics(), elapsed);
                    view.present(&picture);
                    SchedulerBinding::schedule_frame(app);
                }),
            );
            SchedulerBinding::schedule_frame(app);
        })
    });
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
