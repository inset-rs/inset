//! The screen: the layer, moved to another place every second through a clip that cuts its left
//! edge off, the framework's label above its bottom edge, the producer's counts, and a buffer drawn
//! once and shown as an image in the corner.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use waft_embedder::image_codecs::PixelFormat;
use waft_embedder::{
    Color, CompositionLayer, HardwareBufferUsage, Image, PlatformRef, TextDirection,
};
use waft_embedder_android::AndroidPlatform;
use waft_foundation::{App, Handle, Listener, Timer, ValueListenable, ValueNotifier};
use waft_painting::{BoxDecoration, EdgeInsetsGeometry, ImageInfo, TextStyle};
use waft_scheduler::{PersistentFrameCallback, SchedulerBinding};
use waft_widgets::{
    CompositionLayerView, DecoratedBox, Directionality, IntoWidget, Padding, Positioned, RawImage,
    SizedBox, Stack, StackFit, Text, ValueListenableBuilder, WidgetRef, run_app,
};

use crate::producer::{self, Counts, Pacing, Painter, ViewScreen};

/// The layer's size in logical pixels.
const LAYER_SIZE: [f64; 2] = [300.0, 200.0];
/// How much of the layer's left edge its clip cuts off, in logical pixels.
const CUT: f64 = 60.0;
/// Where the clip lies in turn, a second at each, in logical pixels from the top-left.
const PLACES: [[f64; 2]; 3] = [[40.0, 240.0], [90.0, 470.0], [20.0, 700.0]];
/// The side of the buffer drawn once and shown as an image, in pixels.
const IMAGE_SIDE: u32 = 96;
const BACKGROUND: Color = Color::from_argb(255, 245, 245, 240);
const LABEL_BACKGROUND: Color = Color::from_argb(160, 0, 0, 0);
const LABEL: Color = Color::from_argb(255, 255, 255, 255);
const COUNTS: Color = Color::from_argb(255, 30, 30, 30);

/// Shows the screen, with the layer fed by the producer once the view has its size; without a
/// compositor, says so.
pub(crate) fn run(app: &mut App) {
    let platform = app.platform();
    let (Some(compositor), Some(mut painter)) = (platform.compositor(), painter_of(&platform))
    else {
        crate::log::info("no compositor here: Waft's device imports no hardware buffers");
        run_app(app, message("No compositor on this device"));
        return;
    };
    let layer = compositor.make_layer(true);
    let image = image_drawn_once(&platform, &mut painter);
    let place = app.create(ValueNotifier::new(0usize));
    move_every_second(app, place);
    let counts = Arc::new(Counts::default());
    start_producer_once_sized(app, Arc::clone(&layer), painter, Arc::clone(&counts));
    run_app(app, home(layer, place, image, counts));
}

/// What draws on the renderer's device, the one device here that imports hardware buffers.
fn painter_of(platform: &PlatformRef) -> Option<Painter> {
    let images = platform
        .downcast_ref::<AndroidPlatform>()?
        .image_context()?;
    Some(Painter::new(
        images.device().clone(),
        images.queue().clone(),
    ))
}

/// A buffer drawn once, quarters of four colours, imported on the renderer's device as an image.
fn image_drawn_once(platform: &PlatformRef, painter: &mut Painter) -> Option<Image> {
    let usage = HardwareBufferUsage {
        gpu_sampled: true,
        gpu_color_output: true,
        ..HardwareBufferUsage::default()
    };
    let buffer = platform
        .hardware_buffer_allocator()?
        .allocate(IMAGE_SIDE, IMAGE_SIDE, PixelFormat::Rgba8, usage)
        .ok()?;
    let texture = platform.import_texture(&*buffer, &painter.device)?;
    painter.paint(&texture, &producer::quarters(IMAGE_SIDE));
    let image = platform.import_hardware_buffer(&*buffer);
    crate::log::info(&format!(
        "a buffer drawn once, shown as an image: {}",
        if image.is_some() {
            "imported"
        } else {
            "not imported"
        }
    ));
    image
}

/// Moves the layer to the next place every second.
fn move_every_second(app: &mut App, place: Handle<ValueNotifier<usize>>) {
    Timer::new(
        app,
        Duration::from_secs(1),
        Listener::new(move |app| {
            let next = (*place.value(app) + 1) % PLACES.len();
            place.set_value(app, next);
            move_every_second(app, place);
        }),
    );
}

/// Starts the producer at the first frame in which the view has a size: only then does it have
/// the display's pixel ratio the producer draws at. Each frame tells it the screen the view is
/// on, whose refreshes it paces by.
fn start_producer_once_sized(
    app: &mut App,
    layer: Arc<dyn CompositionLayer>,
    painter: Painter,
    counts: Arc<Counts>,
) {
    let platform = app.platform();
    let view = Rc::clone(&platform.views()[0]);
    let waiter = platform
        .vsync_waiter()
        .expect("the Android host has a vsync waiter");
    let screen = Arc::new(ViewScreen::default());
    let waiting = Cell::new(Some((layer, painter, waiter, Arc::clone(&screen), counts)));
    SchedulerBinding::add_persistent_frame_callback(
        app,
        PersistentFrameCallback::new(move |app, _views, _elapsed| {
            let metrics = view.metrics();
            screen.set(metrics.screen);
            if metrics.physical_size == [0.0, 0.0] {
                return;
            }
            let Some((layer, painter, waiter, screen, counts)) = waiting.take() else {
                return;
            };
            let size = LAYER_SIZE.map(|side| (side * metrics.device_pixel_ratio).round() as u32);
            let platform = app.platform();
            let buffers = producer::buffers(&platform, &painter, size);
            let pacing = Pacing {
                layer,
                waiter,
                screen,
            };
            producer::start(pacing, painter, buffers, size, counts);
        }),
    );
}

fn home(
    layer: Arc<dyn CompositionLayer>,
    place: Handle<ValueNotifier<usize>>,
    image: Option<Image>,
    counts: Arc<Counts>,
) -> WidgetRef {
    let listenable: Rc<dyn ValueListenable<usize>> = Rc::new(place);
    let screen = ValueListenableBuilder::new(listenable, move |_app, _context, place, _child| {
        let [left, top] = PLACES[*place];
        Stack::new()
            .fit(StackFit::Expand)
            .children([
                DecoratedBox::new(BoxDecoration::new().color(BACKGROUND)).into_widget(),
                Positioned::new(counted(&counts, *place))
                    .left(16.0)
                    .top(80.0)
                    .into_widget(),
                Positioned::new(RawImage::new(image.clone().map(ImageInfo::new)))
                    .right(16.0)
                    .top(80.0)
                    .into_widget(),
                Positioned::new(clipped_layer(Arc::clone(&layer)))
                    .left(left)
                    .top(top)
                    .into_widget(),
            ])
            .into_widget()
    });
    Directionality::new(TextDirection::Ltr, screen).into_widget()
}

/// The layer with its label, its left edge cut off by the clip of the stack it lies in.
fn clipped_layer(layer: Arc<dyn CompositionLayer>) -> WidgetRef {
    SizedBox::new()
        .width(LAYER_SIZE[0] - CUT)
        .height(LAYER_SIZE[1])
        .child(
            Stack::new().children([Positioned::new(framed_layer(layer))
                .left(-CUT)
                .top(0.0)
                .width(LAYER_SIZE[0])
                .height(LAYER_SIZE[1])
                .into_widget()]),
        )
        .into_widget()
}

/// The layer at its size, with the label over its bottom edge.
fn framed_layer(layer: Arc<dyn CompositionLayer>) -> WidgetRef {
    Stack::new()
        .fit(StackFit::Expand)
        .children([
            CompositionLayerView::new(layer).into_widget(),
            Positioned::new(label("Waft paints this label above the layer"))
                .left(0.0)
                .right(0.0)
                .bottom(0.0)
                .into_widget(),
        ])
        .into_widget()
}

/// What the producer counted, as the screen was last built.
fn counted(counts: &Counts, place: usize) -> WidgetRef {
    Text::new(format!(
        "place {place}\n{} presented\n{} released",
        counts.presented(),
        counts.released()
    ))
    .style(TextStyle::new().font_size(15.0).color(COUNTS))
    .into_widget()
}

fn label(text: &str) -> WidgetRef {
    DecoratedBox::new(BoxDecoration::new().color(LABEL_BACKGROUND))
        .child(
            Padding::new(EdgeInsetsGeometry::all(8.0)).child(
                Text::new(text.to_owned()).style(TextStyle::new().font_size(15.0).color(LABEL)),
            ),
        )
        .into_widget()
}

fn message(text: &str) -> WidgetRef {
    Directionality::new(TextDirection::Ltr, label(text)).into_widget()
}
