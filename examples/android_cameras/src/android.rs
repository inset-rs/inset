//! The screen: what the example found, the camera's live picture, the reader's newest frame and
//! the last photo, with the buttons that switch the camera and take a photo; each finding written
//! to the log.

use std::sync::Arc;
use std::time::Duration;

use waft_embedder::image_codecs::{PixelBuffer, PixelLayout};
use waft_embedder::{
    Camera, CameraFormat, CameraOutputs, Color, Image, Size, TextDirection, VideoFrameReader,
};
use waft_foundation::{App, AsyncApp, Entity, Handle, Listenable, Listener};
use waft_painting::{BoxDecoration, BoxFit, EdgeInsetsGeometry, ImageInfo, TextStyle};
use waft_widgets::{
    BuildContext, CameraPreview, CameraSession, Center, Column, CrossAxisAlignment, DecoratedBox,
    Directionality, Expanded, GestureDetector, IntoWidget, Padding, RawImage, Row, SizedBox,
    StatelessWidget, Text, WidgetRef, available_cameras, open_camera, run_app,
};

const BACKGROUND: Color = Color::from_argb(255, 16, 16, 20);
const LABEL: Color = Color::from_argb(255, 220, 220, 220);
const BUTTON: Color = Color::from_argb(255, 50, 90, 160);

/// The largest picture a camera is started at.
const LARGEST: (f64, f64) = (1280.0, 720.0);
/// How often the reader's newest frame is drawn.
const FRAME_SAMPLING: Duration = Duration::from_millis(500);
/// How long after a camera starts its first photo is taken.
const FIRST_PHOTO: Duration = Duration::from_secs(2);
/// The box a photo is decoded to fit, in pixels.
const PHOTO_BOX: Size = Size::new(480.0, 480.0);

/// What the screen shows.
#[derive(Default)]
struct Shown {
    cameras: Vec<Camera>,
    /// The camera shown, among `cameras`.
    index: usize,
    status: String,
    session: Option<Handle<CameraSession>>,
    reader: Option<Arc<dyn VideoFrameReader>>,
    /// The reader's newest frame, drawn, and how many it handed out.
    frame: Option<Image>,
    frames_read: u32,
    /// The last photo, decoded, and what its file says.
    photo: Option<Image>,
    photo_notes: String,
}

/// Lists the cameras, starts the first, and shows them.
pub(crate) fn run(app: &mut App) {
    let shown = app.new_entity(|_cx| Shown {
        status: "Listing the cameras…".to_owned(),
        ..Shown::default()
    });
    let listing = available_cameras(app);
    let listed = shown.clone();
    let _listing = app.spawn(async move |cx| {
        let cameras = listing.await;
        crate::log::info(&format!("cameras: {}", cameras.len()));
        for camera in &cameras {
            crate::log::info(&format!(
                "  {} ({:?}), id {}",
                camera.name, camera.facing, camera.id.0
            ));
        }
        let any = !cameras.is_empty();
        cx.update(|app| {
            change(app, &listed, |shown| {
                shown.status = if any {
                    String::new()
                } else {
                    "No camera".to_owned()
                };
                shown.cameras = cameras;
            });
            if any {
                start_camera(app, &listed, 0);
            }
        });
    });
    sample_frames(app, &shown);
    run_app(app, home(shown).into_widget());
}

/// Opens the camera `index` and starts it, in place of the one running.
fn start_camera(app: &mut App, shown: &Entity<Shown>, index: usize) {
    if let Some(session) = shown.read(app).session {
        session.dispose(app);
    }
    let Some(camera) = shown.read(app).cameras.get(index).cloned() else {
        return;
    };
    change(app, shown, |shown| {
        shown.index = index;
        shown.session = None;
        shown.reader = None;
        shown.frame = None;
        shown.frames_read = 0;
        shown.status = format!("Opening {}…", camera.name);
    });
    let starting = shown.clone();
    let _starting = app.spawn(async move |cx| {
        let status = run_camera(cx, &starting, &camera).await;
        crate::log::info(&status);
        cx.update(|app| change(app, &starting, |shown| shown.status = status));
    });
}

/// Opens `camera` and starts it with its preview, its frames and its photos; what came of it.
async fn run_camera(cx: &AsyncApp, shown: &Entity<Shown>, camera: &Camera) -> String {
    let Some(opening) = cx.update(|app| open_camera(app, camera)) else {
        return "No cameras on this platform".to_owned();
    };
    let device = match opening.await {
        Ok(device) => device,
        Err(error) => return format!("{} did not open: {error}", camera.name),
    };
    let formats = device.formats();
    crate::log::info(&format!("{} lists {} formats", camera.name, formats.len()));
    for format in &formats {
        crate::log::info(&format!("  {}", described(format)));
    }
    let Some(format) = preferred(&formats) else {
        return format!("{} lists no format", camera.name);
    };
    let refused = cx.update(|app| {
        let upright = CameraOutputs::new().upright_frames();
        CameraSession::start(app, &*device, &formats[0], upright)
    });
    if let Err(error) = refused.await {
        crate::log::info(&format!("upright frames: {error}"));
    }
    let started = cx.update(|app| {
        let platform = app.platform();
        let reader = platform
            .video_frame_readers()
            .map(|readers| readers.make_reader());
        let mut outputs = CameraOutputs::new();
        if let Some(compositor) = platform.compositor() {
            outputs = outputs.preview(compositor.make_layer(false));
        }
        if let Some(reader) = &reader {
            outputs = outputs.frames(Arc::clone(reader));
        }
        if let Some(largest) = largest(&format.photo_sizes) {
            outputs = outputs.photos(largest);
        }
        change(app, shown, |shown| shown.reader = reader);
        CameraSession::start(app, &*device, format, outputs)
    });
    let session = match started.await {
        Ok(session) => session,
        Err(error) => return format!("{} did not start: {error}", camera.name),
    };
    cx.update(|app| {
        log_states(app, session);
        change(app, shown, |shown| shown.session = Some(session));
        let shooting = shown.clone();
        let _first = app.schedule_timer(
            FIRST_PHOTO,
            Listener::new(move |app| take_photo(app, &shooting)),
        );
        format!(
            "{} runs in {}, its preview turned {}, what it captures {}",
            camera.name,
            described(CameraSession::format(session, app)),
            session.video_rotation_angle_for_horizon_level_preview(app),
            session.video_rotation_angle_for_horizon_level_capture(app),
        )
    })
}

/// Draws the reader's newest frame every [`FRAME_SAMPLING`], from now on.
fn sample_frames(app: &mut App, shown: &Entity<Shown>) {
    let sampling = shown.clone();
    let _timer = app.schedule_timer(
        FRAME_SAMPLING,
        Listener::new(move |app| {
            draw_newest_frame(app, &sampling);
            sample_frames(app, &sampling);
        }),
    );
}

/// Reads the reader's newest frame back from its memory and shows it as it should be seen, turned
/// and mirrored as the frame says, logging the first.
fn draw_newest_frame(app: &mut App, shown: &Entity<Shown>) {
    let Some(reader) = shown.read(app).reader.clone() else {
        return;
    };
    let Some(frame) = reader.frame_for(app.platform().now()) else {
        return;
    };
    let first = shown.read(app).frames_read == 0;
    change(app, shown, |shown| shown.frames_read += 1);
    let reading = frame.read_pixels();
    let showing = shown.clone();
    let _reading = app.spawn(async move |cx| {
        let pixels = match reading.await {
            Ok(pixels) => pixels,
            Err(error) => {
                crate::log::info(&format!("a frame's pixels were not read: {error}"));
                return;
            }
        };
        if first {
            crate::log::info(&format!(
                "the reader's first frame: {} by {}, shown turned {} and flipped {}, its middle \
                 pixel {:?}",
                frame.width(),
                frame.height(),
                frame.rotation(),
                frame.flip(),
                middle_pixel(&pixels),
            ));
        }
        let upright = shown_as(&pixels, frame.rotation(), frame.flip());
        cx.update(|app| {
            let image = upright.and_then(|upright| app.platform().import_pixels(upright));
            change(app, &showing, |shown| shown.frame = image);
        });
    });
}

/// `pixels` turned `rotation` degrees clockwise, then mirrored left to right where `flip`, as
/// WebCodecs shows a frame; `None` where they cannot be laid out again.
fn shown_as(pixels: &PixelBuffer, rotation: u32, flip: bool) -> Option<PixelBuffer> {
    let layout = pixels.layout();
    let [width, height] = layout.size;
    let quarter = rotation % 180 == 90;
    let [shown_width, shown_height] = if quarter {
        [height, width]
    } else {
        [width, height]
    };
    let mut bytes = vec![0; shown_width as usize * shown_height as usize * 4];
    for y in 0..shown_height {
        for x in 0..shown_width {
            let x_unflipped = if flip { shown_width - 1 - x } else { x };
            let [source_x, source_y] = match rotation % 360 {
                90 => [y, height - 1 - x_unflipped],
                180 => [width - 1 - x_unflipped, height - 1 - y],
                270 => [width - 1 - y, x_unflipped],
                _ => [x_unflipped, y],
            };
            let from = source_y as usize * layout.row_bytes as usize + source_x as usize * 4;
            let to = (y as usize * shown_width as usize + x as usize) * 4;
            bytes[to..to + 4].copy_from_slice(&pixels.bytes()[from..from + 4]);
        }
    }
    let shown_layout = PixelLayout {
        size: [shown_width, shown_height],
        row_bytes: shown_width * 4,
        ..layout
    };
    PixelBuffer::new(shown_layout, bytes).ok()
}

/// The RGBA of the pixel in the middle of `pixels`.
fn middle_pixel(pixels: &PixelBuffer) -> &[u8] {
    let layout = pixels.layout();
    let [width, height] = layout.size;
    let at = (height / 2) as usize * layout.row_bytes as usize + (width / 2) as usize * 4;
    &pixels.bytes()[at..at + 4]
}

/// Takes a photo with the camera running, and shows it decoded.
fn take_photo(app: &mut App, shown: &Entity<Shown>) {
    let Some(session) = shown.read(app).session else {
        return;
    };
    let taking = session.take_photo(app);
    let showing = shown.clone();
    let _taking = app.spawn(async move |cx| {
        let photo = match taking.await {
            Ok(photo) => photo,
            Err(error) => {
                let notes = format!("no photo: {error}");
                crate::log::info(&notes);
                cx.update(|app| change(app, &showing, |shown| shown.photo_notes = notes));
                return;
            }
        };
        let Some(opening) = cx.update(|app| {
            app.platform()
                .image_codecs()
                .map(|codecs| codecs.open(Arc::from(photo.bytes.as_slice())))
        }) else {
            return;
        };
        let source = match opening.await {
            Ok(source) => source,
            Err(error) => {
                crate::log::info(&format!("the photo did not open: {error}"));
                return;
            }
        };
        let metadata = source.metadata();
        let notes = format!(
            "photo: {} bytes of {}, {}×{} upright, stored {:?}, by {} {}",
            photo.bytes.len(),
            source.mime_type(),
            source.size().width(),
            source.size().height(),
            source.orientation(),
            metadata.camera_make.unwrap_or_default(),
            metadata.camera_model.unwrap_or_default(),
        );
        crate::log::info(&notes);
        let decoded = source.decode_pixels(0, Some(PHOTO_BOX)).await;
        cx.update(|app| {
            let image = decoded
                .ok()
                .and_then(|pixels| app.platform().import_pixels(pixels));
            change(app, &showing, |shown| {
                shown.photo = image;
                shown.photo_notes = notes;
            });
        });
    });
}

/// Logs `session`'s state now and each time it changes.
fn log_states(app: &mut App, session: Handle<CameraSession>) {
    crate::log::info(&format!("state: {:?}", session.state(app)));
    let listenable: std::rc::Rc<dyn Listenable> = std::rc::Rc::new(session);
    listenable.add_listener(
        app,
        Listener::new(move |app| crate::log::info(&format!("state: {:?}", session.state(app)))),
    );
}

/// The largest format up to [`LARGEST`], the fastest of those that size; else the first.
fn preferred(formats: &[CameraFormat]) -> Option<&CameraFormat> {
    let fits = |format: &&CameraFormat| {
        format.size.width() <= LARGEST.0 && format.size.height() <= LARGEST.1
    };
    let area = |format: &CameraFormat| format.size.width() * format.size.height();
    formats
        .iter()
        .filter(fits)
        .max_by(|a, b| {
            area(a)
                .total_cmp(&area(b))
                .then(a.max_frame_rate.total_cmp(&b.max_frame_rate))
        })
        .or(formats.first())
}

fn largest(sizes: &[Size]) -> Option<Size> {
    sizes
        .iter()
        .copied()
        .max_by(|a, b| (a.width() * a.height()).total_cmp(&(b.width() * b.height())))
}

fn described(format: &CameraFormat) -> String {
    format!(
        "{}×{} at {}–{} fps, photos up to {}",
        format.size.width(),
        format.size.height(),
        format.min_frame_rate,
        format.max_frame_rate,
        largest(&format.photo_sizes).map_or("none".to_owned(), |size| format!(
            "{}×{}",
            size.width(),
            size.height()
        ))
    )
}

fn change(app: &mut App, shown: &Entity<Shown>, change: impl FnOnce(&mut Shown)) {
    shown.update(app, |shown, cx| {
        change(shown);
        cx.notify();
    });
}

fn home(shown: Entity<Shown>) -> WidgetRef {
    Directionality::new(
        TextDirection::Ltr,
        DecoratedBox::new(BoxDecoration::new().color(BACKGROUND)).child(
            Padding::new(EdgeInsetsGeometry::from_ltrb(12.0, 40.0, 12.0, 24.0))
                .child(Screen { shown }),
        ),
    )
    .into_widget()
}

#[derive(Debug)]
struct Screen {
    shown: Entity<Shown>,
}

impl StatelessWidget for Screen {
    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let shown = self.shown.read(app);
        let listed = shown
            .cameras
            .iter()
            .enumerate()
            .map(|(index, camera)| {
                let mark = if index == shown.index { "▸ " } else { "  " };
                format!("{mark}{} ({:?})", camera.name, camera.facing)
            })
            .collect::<Vec<_>>()
            .join("\n");
        let live = match shown.session {
            Some(session) => Center::new()
                .child(CameraPreview::new(session))
                .into_widget(),
            None => SizedBox::expand().into_widget(),
        };
        let frame_notes = format!("reader: {} frames", shown.frames_read);
        let switch = self.shown.clone();
        let shoot = self.shown.clone();
        let count = shown.cameras.len().max(1);
        let next = (shown.index + 1) % count;
        Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .spacing(8.0)
            .children([
                text(&listed, 13.0),
                text(&shown.status, 13.0),
                Expanded::new(live).into_widget(),
                SizedBox::new()
                    .height(170.0)
                    .child(
                        Row::new().spacing(8.0).children([
                            Expanded::new(picture(shown.frame.as_ref(), &frame_notes))
                                .into_widget(),
                            Expanded::new(picture(shown.photo.as_ref(), &shown.photo_notes))
                                .into_widget(),
                        ]),
                    )
                    .into_widget(),
                Row::new()
                    .spacing(12.0)
                    .children([
                        button("Switch", move |app| start_camera(app, &switch, next)),
                        button("Photo", move |app| take_photo(app, &shoot)),
                    ])
                    .into_widget(),
            ])
            .into_widget()
    }
}

/// A picture over its notes, fitted in the room there is.
fn picture(image: Option<&Image>, notes: &str) -> WidgetRef {
    let image = match image {
        Some(image) => RawImage::new(Some(ImageInfo::new(image.clone())))
            .fit(BoxFit::Contain)
            .into_widget(),
        None => SizedBox::expand().into_widget(),
    };
    Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .children([Expanded::new(image).into_widget(), text(notes, 10.0)])
        .into_widget()
}

fn button(label: &str, tapped: impl Fn(&mut App) + 'static) -> WidgetRef {
    GestureDetector::new()
        .on_tap(Listener::new(tapped))
        .child(
            DecoratedBox::new(BoxDecoration::new().color(BUTTON)).child(
                Padding::new(EdgeInsetsGeometry::from_ltrb(20.0, 12.0, 20.0, 12.0))
                    .child(text(label, 16.0)),
            ),
        )
        .into_widget()
}

fn text(line: &str, size: f64) -> WidgetRef {
    Text::new(line.to_owned())
        .style(TextStyle::new().font_size(size).color(LABEL))
        .into_widget()
}
