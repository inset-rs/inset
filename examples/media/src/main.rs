//! A video beside a camera's preview, each on a native surface the system draws into: the movie
//! given, looped, and the first camera the platform lists, opened at once, which asks the user for
//! the cameras the first time, then started with its preview in its largest format up to 1920 by
//! 1080. The cameras the platform lists are printed, and again each time one is plugged in or out;
//! the camera's formats, the one it runs in and its state as it changes too. A click on the video
//! pauses or plays it.
//!
//! ```text
//! cargo waft run -p media -- <a movie's path or link>
//! ```
//!
//! Run through `cargo waft`, the app is a bundle that says why it uses the camera
//! (`usage_descriptions` in its `Cargo.toml`), without which macOS ends an app that opens one.

use std::path::PathBuf;
use std::rc::Rc;

use waft_embedder::{
    Camera, CameraDevice, CameraError, CameraFormat, CameraOutputs, Color, PickedFile,
    PlayerActivity, TextDirection, WindowConfig,
};
use waft_embedder_winit::WinitEmbedder;
use waft_foundation::{App, AsyncApp, Entity, Handle, Listenable, Listener};
use waft_painting::{BoxDecoration, TextStyle};
use waft_shell::Shell;
use waft_widgets::{
    BuildContext, CameraPreview, CameraSession, Center, DecoratedBox, Directionality, Expanded,
    GestureDetector, IntoWidget, Player, PlayerView, Row, StatelessWidget, Text, WidgetRef,
    available_cameras, available_cameras_changed, open_camera, run_app_with,
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
    print_cameras(app);
    let camera = app.new_entity(|_cx| CameraPane::Opening);
    open_first_camera(app, &camera);
    let window = WindowConfig {
        title: "Waft — media".to_owned(),
        size: [960.0, 400.0],
        ..WindowConfig::default()
    };
    run_app_with(app, home(player, camera), window);
}

/// Prints the cameras the platform lists now, and again each time one is plugged in or out.
fn print_cameras(app: &mut App) {
    let _printing = app.spawn(async move |cx| {
        loop {
            // The next change is asked for before the cameras are listed, so none goes unheard.
            let (changed, listing) =
                cx.update(|app| (available_cameras_changed(app), available_cameras(app)));
            print_camera_list(&listing.await);
            let Some(changed) = changed else {
                return;
            };
            changed.await;
        }
    });
}

fn print_camera_list(cameras: &[Camera]) {
    println!("cameras: {}", cameras.len());
    for camera in cameras {
        println!("  {}, {:?}", camera.name, camera.facing);
    }
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
    let listing = available_cameras(app);
    let pane = pane.clone();
    let _opening = app.spawn(async move |cx| {
        let next = match listing.await.into_iter().next() {
            Some(camera) => run_camera(cx, &camera).await,
            None => CameraPane::NoCamera,
        };
        cx.update(|app| show(app, &pane, next));
    });
}

/// Opens `camera` and starts it with its preview: the pane that shows it running, or what kept it
/// from running.
async fn run_camera(cx: &AsyncApp, camera: &Camera) -> CameraPane {
    let Some(opening) = cx.update(|app| open_camera(app, camera)) else {
        return CameraPane::NoCamera;
    };
    let started = match opening.await {
        Ok(device) => start_preview(cx, camera, &*device).await,
        Err(error) => Err(error),
    };
    match started {
        Ok(session) => CameraPane::Running(session),
        Err(error) => CameraPane::Failed(error),
    }
}

fn show(app: &mut App, pane: &Entity<CameraPane>, next: CameraPane) {
    pane.update(app, |pane, cx| {
        *pane = next;
        cx.notify();
    });
}

/// Starts `device` in its preferred format with its preview, printing its formats, the one it runs
/// in and its state as it changes.
async fn start_preview(
    cx: &AsyncApp,
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
    let starting = cx.update(|app| {
        let mut outputs = CameraOutputs::new();
        if let Some(compositor) = app.platform().compositor() {
            outputs = outputs.preview(compositor.make_layer(false));
        }
        CameraSession::start(app, device, format, outputs)
    });
    let session = starting.await?;
    cx.update(|app| {
        println!("  picked {}", described(format));
        println!(
            "  runs in {}",
            described(CameraSession::format(session, app))
        );
        print_states(app, session);
    });
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
            if matches!(
                Player::state(player, app).activity,
                PlayerActivity::Paused(_)
            ) {
                Player::play(player, app);
            } else {
                Player::pause(player, app);
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
