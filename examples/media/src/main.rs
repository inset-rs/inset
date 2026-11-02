//! A video beside a camera's preview, each on a native surface the system draws into: the movie
//! given, looped, and the first camera the platform lists, opened at once, which asks the user for
//! the cameras the first time, then started with its preview in its largest format up to 1920 by
//! 1080. The camera's formats, the one it runs in and its state as it changes are printed. A click
//! on the video pauses or plays it.
//!
//! ```text
//! cargo inset run -p media -- <a movie's path or link>
//! ```
//!
//! Run through `cargo inset`, the app is a bundle that says why it uses the camera
//! (`usage_descriptions` in its `Cargo.toml`), without which macOS ends an app that opens one.

use std::path::PathBuf;
use std::rc::Rc;

use inset_embedder::{
    Camera, CameraDevice, CameraError, CameraFormat, CameraOutputs, Color, PickedFile,
    TextDirection, WindowConfig,
};
use inset_embedder_winit::WinitEmbedder;
use inset_foundation::{App, Entity, Handle, Listenable, Listener};
use inset_painting::{BoxDecoration, TextStyle};
use inset_shell::Shell;
use inset_widgets::{
    BuildContext, CameraPreview, CameraSession, Center, DecoratedBox, Directionality, Expanded,
    GestureDetector, IntoWidget, Player, PlayerView, Row, StatelessWidget, Text, WidgetRef,
    available_cameras, open_camera, run_app_with,
};

const BACKGROUND: Color = Color::from_argb(255, 20, 20, 24);
const LABEL: Color = Color::from_argb(255, 200, 200, 200);

/// The largest picture the camera is started at.
const LARGEST: (f64, f64) = (1920.0, 1080.0);

fn main() {
    let url = std::env::args().nth(1).map(url_of);
    WinitEmbedder.run(move |platform| Shell::new(platform, move |app| start(app, url)));
}

/// A link as it is, and anything else as a file's path, by the URL a pick of it would have.
fn url_of(argument: String) -> String {
    if argument.contains("://") {
        argument
    } else {
        PickedFile::Path(PathBuf::from(argument)).url()
    }
}

fn start(app: &mut App, url: Option<String>) {
    let player = url.and_then(|url| Player::new(app, &url));
    if let Some(player) = player {
        Player::set_looping(player, app, true);
        Player::play(player, app);
    }
    let camera = app.new_entity(|_cx| CameraPane::Opening);
    open_first_camera(app, &camera);
    let window = WindowConfig {
        title: "Inset — media".to_owned(),
        size: [960.0, 400.0],
        ..WindowConfig::default()
    };
    run_app_with(app, home(player, camera), window);
}

/// Where the camera's side of the window stands.
#[derive(Debug)]
enum CameraPane {
    Opening,
    NoCamera,
    Running(Handle<CameraSession>),
    Failed(CameraError),
}

/// Opens the first camera the platform lists, and once the user lets the app use it, starts it
/// with its preview.
fn open_first_camera(app: &mut App, pane: &Entity<CameraPane>) {
    let first = available_cameras(app).into_iter().next();
    let Some(camera) = first else {
        show(app, pane, CameraPane::NoCamera);
        return;
    };
    let Some(opening) = open_camera(app, &camera) else {
        show(app, pane, CameraPane::NoCamera);
        return;
    };
    let pane = pane.clone();
    let _opening = app.spawn(async move |cx| {
        let opened = opening.await;
        cx.update(|app| {
            let started = opened.and_then(|device| start_preview(app, &camera, &*device));
            let next = match started {
                Ok(session) => CameraPane::Running(session),
                Err(error) => CameraPane::Failed(error),
            };
            show(app, &pane, next);
        });
    });
}

fn show(app: &mut App, pane: &Entity<CameraPane>, next: CameraPane) {
    pane.update(app, |pane, cx| {
        *pane = next;
        cx.notify();
    });
}

/// Starts `device` in its preferred format with its preview, printing its formats, the one it runs
/// in and its state as it changes.
fn start_preview(
    app: &mut App,
    camera: &Camera,
    device: &dyn CameraDevice,
) -> Result<Handle<CameraSession>, CameraError> {
    let formats = device.formats();
    println!("camera: {}", camera.name);
    for format in &formats {
        println!("  lists {}", described(format));
    }
    let format = preferred(&formats)
        .ok_or_else(|| CameraError::Failed("the camera lists no format".to_owned()))?;
    let mut outputs = CameraOutputs::new();
    if let Some(compositor) = app.platform().compositor() {
        outputs = outputs.preview(compositor.make_layer(false));
    }
    let session = CameraSession::start(app, device, format, outputs)?;
    println!("  picked {}", described(format));
    println!(
        "  runs in {}",
        described(CameraSession::format(session, app))
    );
    print_states(app, session);
    Ok(session)
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

fn described(format: &CameraFormat) -> String {
    let photos: Vec<String> = format
        .photo_sizes
        .iter()
        .map(|size| format!("{}×{}", size.width(), size.height()))
        .collect();
    format!(
        "{}×{} at {}–{} fps, photos at {}",
        format.size.width(),
        format.size.height(),
        format.min_frame_rate,
        format.max_frame_rate,
        photos.join(", ")
    )
}

/// Prints `session`'s state now and each time it changes.
fn print_states(app: &mut App, session: Handle<CameraSession>) {
    println!("  state: {:?}", CameraSession::state(session, app));
    let listenable: Rc<dyn Listenable> = Rc::new(session);
    listenable.add_listener(
        app,
        Listener::new(move |app| println!("  state: {:?}", CameraSession::state(session, app))),
    );
}

fn home(player: Option<Handle<Player>>, camera: Entity<CameraPane>) -> WidgetRef {
    let video = match player {
        Some(player) => clickable(player),
        None => label("No movie: pass a path or a link, on a platform with players"),
    };
    Directionality::new(
        TextDirection::Ltr,
        DecoratedBox::new(BoxDecoration::new().color(BACKGROUND)).child(Row::new().children([
            Expanded::new(Center::new().child(video)).into_widget(),
            Expanded::new(Center::new().child(CameraSide { camera })).into_widget(),
        ])),
    )
    .into_widget()
}

/// The camera's preview once it runs, or what keeps it from running.
#[derive(Debug)]
struct CameraSide {
    camera: Entity<CameraPane>,
}

impl StatelessWidget for CameraSide {
    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        match self.camera.read(app) {
            CameraPane::Opening => label("Opening the camera…"),
            CameraPane::NoCamera => label("No camera"),
            CameraPane::Failed(error) => label(&format!("No camera: {error}")),
            CameraPane::Running(session) => CameraPreview::new(*session).into_widget(),
        }
    }
}

/// `player`'s video, which a click pauses or plays.
fn clickable(player: Handle<Player>) -> WidgetRef {
    GestureDetector::new()
        .on_tap(Listener::new(move |app| {
            if Player::state(player, app).playing {
                Player::pause(player, app);
            } else {
                Player::play(player, app);
            }
        }))
        .child(PlayerView::new(player))
        .into_widget()
}

fn label(text: &str) -> WidgetRef {
    Text::new(text)
        .style(TextStyle::new().color(LABEL))
        .into_widget()
}
