//! A video beside a camera's preview, each on a native surface the system draws into: the movie
//! given, looped, and the first camera the platform lists, started at once, which asks the user for
//! the camera the first time. A click on the video pauses or plays it.
//!
//! ```text
//! cargo inset run -p media -- <a movie's path or link>
//! ```
//!
//! Run through `cargo inset`, the app is a bundle that says why it uses the camera
//! (`usage_descriptions` in its `Cargo.toml`), without which macOS ends an app that opens one.

use std::path::PathBuf;

use inset_embedder::{Color, PickedFile, TextDirection, WindowConfig};
use inset_embedder_winit::WinitEmbedder;
use inset_foundation::{App, Handle, Listener};
use inset_painting::{BoxDecoration, TextStyle};
use inset_shell::Shell;
use inset_widgets::{
    CameraPreview, CameraSession, Center, DecoratedBox, Directionality, Expanded, GestureDetector,
    IntoWidget, Player, Row, Text, VideoView, WidgetRef, available_cameras, run_app_with,
};

const BACKGROUND: Color = Color::from_argb(255, 20, 20, 24);
const LABEL: Color = Color::from_argb(255, 200, 200, 200);

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
    let session = available_cameras(app)
        .first()
        .and_then(|camera| CameraSession::new(app, camera));
    if let Some(session) = session {
        CameraSession::start(session, app);
    }
    let window = WindowConfig {
        title: "Inset — media".to_owned(),
        size: [960.0, 400.0],
        ..WindowConfig::default()
    };
    run_app_with(app, home(player, session), window);
}

fn home(player: Option<Handle<Player>>, session: Option<Handle<CameraSession>>) -> WidgetRef {
    let video = match player {
        Some(player) => clickable(player),
        None => label("No movie: pass a path or a link, on a platform with players"),
    };
    let preview = match session {
        Some(session) => CameraPreview::new(session).into_widget(),
        None => label("No camera"),
    };
    Directionality::new(
        TextDirection::Ltr,
        DecoratedBox::new(BoxDecoration::new().color(BACKGROUND)).child(Row::new().children([
            Expanded::new(Center::new().child(video)).into_widget(),
            Expanded::new(Center::new().child(preview)).into_widget(),
        ])),
    )
    .into_widget()
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
        .child(VideoView::new(player))
        .into_widget()
}

fn label(text: &str) -> WidgetRef {
    Text::new(text)
        .style(TextStyle::new().color(LABEL))
        .into_widget()
}
