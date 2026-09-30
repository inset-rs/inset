//! The window: a surface in the middle, the framework's label above its bottom edge.

use std::sync::Arc;
use std::time::Duration;

use inset_embedder::{Color, PlatformRef, Surface, TextDirection};
use inset_embedder_winit::{ImplicitViewConfig, WinitEmbedder};
use inset_painting::{BoxDecoration, EdgeInsetsGeometry, TextStyle};
use inset_shell::Shell;
use inset_widgets::{
    Center, DecoratedBox, Directionality, IntoWidget, Padding, Positioned, SizedBox, Stack,
    StackFit, SurfaceView, Text, Transform, WidgetRef, run_app,
};

use crate::producer;

/// The surface's size in logical pixels.
const SURFACE_SIZE: [f64; 2] = [400.0, 300.0];
const BACKGROUND: Color = Color::from_argb(255, 245, 245, 240);
const LABEL_BACKGROUND: Color = Color::from_argb(160, 0, 0, 0);
const LABEL: Color = Color::from_argb(255, 255, 255, 255);

/// Runs the window, the surface and its label scaled by `scale` about their middle.
pub(crate) fn run(exit_after: Option<Duration>, scale: f64) {
    WinitEmbedder::default()
        .implicit_view(Some(ImplicitViewConfig {
            title: "Inset — a native surface".to_owned(),
            logical_size: [640.0, 480.0],
        }))
        .run(move |platform| {
            let surface = open_surface(&platform);
            producer::start(Arc::clone(&surface), pixel_size(&platform), exit_after);
            Shell::new(platform, move |app| run_app(app, home(surface, scale)))
        });
}

/// A surface of the implicit view's, from the host's compositor.
fn open_surface(platform: &PlatformRef) -> Arc<dyn Surface> {
    let compositor = platform
        .compositor()
        .expect("the macOS host has a compositor");
    let view = platform.implicit_view().expect("an implicit view").id();
    compositor.open_surface(view, true)
}

/// The surface's size in the display's pixels, which is what the producer draws.
fn pixel_size(platform: &PlatformRef) -> [usize; 2] {
    let ratio = platform
        .implicit_view()
        .map_or(1.0, |view| view.metrics().device_pixel_ratio);
    SURFACE_SIZE.map(|side| (side * ratio).round() as usize)
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
