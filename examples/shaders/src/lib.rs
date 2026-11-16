//! Custom shaders, three fragment programs in one window: one paints the background, sampling a
//! picture drawn into an image; one filters what lies behind a panel, through `BackdropFilter`;
//! and one filters a child, through `ImageFiltered`. Each program is WGSL in `src/shaders`, built
//! into the binary with `include_str!` (`programs.rs`).
//!
//! The crate is a library as well as a binary so `tests/` can draw the real app.
#![feature(arbitrary_self_types)]

pub mod programs;

use std::any::Any;

use waft_embedder::{Canvas, Color, Paint, Rect, Size, TextDirection, WindowConfig, valo};
use waft_foundation::App;
use waft_rendering::CustomPainter;
use waft_widgets::{
    BackdropFilter, BorderRadius, BorderRadiusGeometry, ClipRRect, CustomPaint, Directionality,
    ImageFiltered, IntoWidget, Positioned, SizedBox, Stack, WidgetRef, run_app_with,
};

use crate::programs::Rings;

/// The window's size, in logical pixels.
pub const WINDOW_SIZE: [f64; 2] = [640.0, 420.0];

/// The panel whose backdrop is filtered, in the window's coordinates.
pub const PANEL: Rect = Rect::from_ltrb(40.0, 40.0, 280.0, 280.0);

/// The child filtered in two colours, in the window's coordinates.
pub const BADGE: Rect = Rect::from_ltrb(340.0, 220.0, 600.0, 380.0);

/// The side of the pixelated panel's cells, in physical pixels.
pub const CELL: f64 = 16.0;

/// The two colours the badge is filtered into.
pub const DARK: Color = Color::new(0xFF1B2A4A);
pub const LIGHT: Color = Color::new(0xFFF6C85F);

/// Every host's entry point; `src/main.rs` calls the generated `main`.
#[waft::main]
fn main(app: &mut waft::App) {
    let window = WindowConfig {
        title: "Waft — shaders".to_owned(),
        size: WINDOW_SIZE,
        ..WindowConfig::default()
    };
    run_app_with(app, shaders(), window);
}

/// The app: the painted background, the pixelated panel over it, and the two-coloured badge.
pub fn shaders() -> WidgetRef {
    let panel = ClipRRect::new()
        .border_radius(BorderRadiusGeometry::from(BorderRadius::circular(24.0)))
        .child(BackdropFilter::filter(programs::pixelate(CELL)).child(SizedBox::expand()));
    let badge = ImageFiltered::new(programs::duotone(DARK, LIGHT))
        .child(CustomPaint::new().painter(StripesPainter));
    Directionality::new(
        TextDirection::Ltr,
        Stack::new().children([
            Positioned::fill(CustomPaint::new().painter(RingsPainter)).into_widget(),
            Positioned::from_rect(PANEL, panel).into_widget(),
            Positioned::from_rect(BADGE, badge).into_widget(),
        ]),
    )
    .into_widget()
}

/// Paints the whole box with the rings program over a checkerboard the platform drew into an
/// image; paints nothing on a platform that draws no pictures into images.
struct RingsPainter;

impl CustomPainter for RingsPainter {
    fn paint(&self, app: &mut App, canvas: &mut Canvas, size: Size) {
        let Some(pattern) = app
            .platform()
            .picture_to_image_sync(&checkerboard(), 64, 64)
        else {
            return;
        };
        Rings::new().paint(canvas, size, &pattern);
    }

    fn should_repaint(&self, _app: &App, _old_delegate: &dyn CustomPainter) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A 64-pixel checkerboard of eight squares a side, as a picture.
fn checkerboard() -> waft_embedder::Picture {
    let mut canvas = Canvas::new();
    for row in 0..8 {
        for column in 0..8 {
            let color = if (row + column) % 2 == 0 {
                Color::new(0xFFE76F51)
            } else {
                Color::new(0xFF264653)
            };
            canvas.draw_rect(
                valo::Rect::new(column as f32 * 8.0, row as f32 * 8.0, 8.0, 8.0),
                &Paint::from_color(color.into()),
            );
        }
    }
    canvas.build()
}

/// Diagonal stripes of grey, from black to white, for the duotone to colour.
struct StripesPainter;

impl CustomPainter for StripesPainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        const STRIPES: usize = 8;
        let width = size.width() / STRIPES as f64;
        for stripe in 0..STRIPES {
            let grey = (stripe * 255 / (STRIPES - 1)) as i32;
            let rect = Rect::from_ltwh(stripe as f64 * width, 0.0, width, size.height());
            canvas.draw_rect(
                rect,
                &Paint::from_color(Color::from_argb(255, grey, grey, grey).into()),
            );
        }
    }

    fn should_repaint(&self, _app: &App, _old_delegate: &dyn CustomPainter) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
