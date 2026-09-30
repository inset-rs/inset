//! What macOS asks of an application as a whole, through the winit host: the files it is
//! asked to open, the Dock icon clicked, and quitting. The window lists them, and each is
//! printed as it comes.
//!
//! A quit asks the app first: Cmd-Q, Quit in the app menu, or Quit in the Dock icon's menu.
//! The app answers only once it has saved, which takes three seconds here, and then agrees, so a
//! single Cmd-Q quits it. A second Cmd-Q during the save quits at once, as it does a Flutter app
//! on macOS. Only a Quit asks, and only macOS has one: closing the window ends the app without
//! asking, on every system.
//!
//! ```text
//! cargo run -p app-events
//! ```
//!
//! macOS opens files only with an app bundle it knows, and matches files to it by the types the
//! bundle declares, which `[package.metadata.inset.file_types]` in this crate's Cargo.toml gives:
//!
//! ```text
//! cargo inset build macos -p app-events
//! open "target/inset/macos/release/App events.app"
//! open -a "App events" note.insetnote     # at launch or while it runs
//! ```
//!
//! Click the Dock icon for a reopen, with the window on screen or minimized: each is printed
//! with whether a window was visible and where the pointer was.
#![feature(arbitrary_self_types)]

use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use inset_embedder::{AppExitResponse, Color, TextDirection};
use inset_embedder_winit::{ImplicitViewConfig, Reopen, WinitEmbedder, WinitPlatform};
use inset_foundation::{App, Completer, Entity, Listener, Task, Timer};
use inset_painting::{BoxDecoration, EdgeInsetsGeometry, TextStyle};
use inset_shell::Shell;
use inset_widgets::{
    AppLifecycleListener, BuildContext, Column, DecoratedBox, Directionality, IntoWidget, Padding,
    StatelessWidget, Text, WidgetRef, run_app,
};

const BACKGROUND: Color = Color::from_argb(255, 245, 245, 240);
const INK: Color = Color::from_argb(255, 51, 51, 64);

/// How long the app takes to save before it can quit.
const SAVE_TIME: Duration = Duration::from_secs(3);

fn main() {
    WinitEmbedder::default()
        .implicit_view(Some(ImplicitViewConfig {
            title: "App events".to_owned(),
            logical_size: [560.0, 360.0],
        }))
        .run(|platform| {
            Shell::new(platform, |app| {
                let requests = app.new_entity(|_cx| Requests::default());
                listen(app, &requests);
                answer_quits(app, &requests);
                run_app(app, home(requests));
            })
        });
}

/// What the system has asked of the app so far.
#[derive(Debug, Default)]
struct Requests {
    files: Vec<PathBuf>,
    reopens: u32,
    save: Save,
}

/// Where the save a quit waits for stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Save {
    #[default]
    NotStarted,
    Saving,
    Saved,
}

/// Sets the host's handlers, which the host calls from its event loop; each posts its request
/// to the app.
fn listen(app: &mut App, requests: &Entity<Requests>) {
    let platform = app.platform();
    let Some(host) = platform.downcast_ref::<WinitPlatform>() else {
        return;
    };
    let cx = app.to_async();
    host.set_open_files_requested(Some(Rc::new({
        let cx = cx.clone();
        let requests = requests.clone();
        move |files: Vec<PathBuf>| {
            println!("open files: {files:?}");
            let requests = requests.clone();
            cx.post(move |app| {
                requests.update(app, |requests, cx| {
                    requests.files.extend(files);
                    cx.notify();
                });
            });
        }
    })));
    host.set_reopen_requested(Some(Rc::new({
        let requests = requests.clone();
        move |reopen: Reopen| {
            println!("reopen: {reopen:?}");
            let requests = requests.clone();
            cx.post(move |app| {
                requests.update(app, |requests, cx| {
                    requests.reopens += 1;
                    cx.notify();
                });
            });
        }
    })));
}

/// Answers each quit with an `AppLifecycleListener`: yes, once the app has saved.
fn answer_quits(app: &mut App, requests: &Entity<Requests>) {
    let requests = requests.clone();
    let saved = Completer::new();
    AppLifecycleListener::new(app)
        .on_exit_requested(app, Rc::new(move |app| answer_quit(app, &requests, &saved)));
}

/// Answers yes once `saved` completes: the first quit starts the save, and a quit after it
/// has finished is answered at once.
fn answer_quit(
    app: &mut App,
    requests: &Entity<Requests>,
    saved: &Completer<()>,
) -> Task<AppExitResponse> {
    let save = requests.read(app).save;
    println!("quit asked while the save is {save:?}");
    if save == Save::NotStarted {
        start_saving(app, requests, saved.clone());
    }
    let saved = saved.future();
    app.spawn(async move |_cx| {
        saved.await;
        println!("saved; quitting");
        AppExitResponse::Exit
    })
}

/// Saves, which a timer stands in for, and completes `saved` once done.
fn start_saving(app: &mut App, requests: &Entity<Requests>, saved: Completer<()>) {
    set_save(app, requests, Save::Saving);
    let requests = requests.clone();
    Timer::new(
        app,
        SAVE_TIME,
        Listener::new(move |app| {
            set_save(app, &requests, Save::Saved);
            saved.complete(app, ());
        }),
    );
}

fn set_save(app: &mut App, requests: &Entity<Requests>, save: Save) {
    requests.update(app, |requests, cx| {
        requests.save = save;
        cx.notify();
    });
}

fn home(requests: Entity<Requests>) -> WidgetRef {
    Directionality::new(
        TextDirection::Ltr,
        DecoratedBox::new(BoxDecoration::new().color(BACKGROUND))
            .child(Padding::new(EdgeInsetsGeometry::all(24.0)).child(RequestList { requests })),
    )
    .into_widget()
}

#[derive(Debug)]
struct RequestList {
    requests: Entity<Requests>,
}

impl StatelessWidget for RequestList {
    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let requests = self.requests.read(app);
        let mut lines = vec![
            line(save_line(requests.save).to_owned()),
            line(format!("Reopened {} time(s)", requests.reopens)),
        ];
        if requests.files.is_empty() {
            lines.push(line("No file opened yet".to_owned()));
        }
        lines.extend(
            requests
                .files
                .iter()
                .map(|file| line(format!("Opened {}", file.display()))),
        );
        Column::new().spacing(8.0).children(lines).into_widget()
    }
}

fn save_line(save: Save) -> &'static str {
    match save {
        Save::NotStarted => "Cmd-Q: the app saves first, then quits",
        Save::Saving => "Quitting once saved: saving for three seconds",
        Save::Saved => "Saved: quitting",
    }
}

fn line(text: String) -> WidgetRef {
    Text::new(text)
        .style(TextStyle::new().font_size(15.0).color(INK))
        .into_widget()
}
