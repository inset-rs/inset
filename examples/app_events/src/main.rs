//! What the system asks of an application as a whole, through the winit embedder: the files and
//! links it is asked to open, the Dock icon clicked, and quitting; and what is dragged onto its
//! window. The window lists them, and each is printed as it comes.
//!
//! A quit asks the app first: Cmd-Q, Quit in the app menu, or Quit in the Dock icon's menu on
//! macOS, and closing the window on every system, since closing `run_app`'s only window asks to
//! quit. The app answers only once it has saved, which takes three seconds here, and then agrees,
//! so a single Cmd-Q or close quits it. A second Cmd-Q during the save reaches the app too, which
//! answers it once the same save is done.
//!
//! ```text
//! cargo run -p app-events
//! ```
//!
//! macOS opens files only with an app bundle it knows, and matches files to it by the types the
//! bundle declares, which `[package.metadata.inset.file_associations]` in this crate's Cargo.toml
//! gives:
//!
//! ```text
//! cargo inset build macos -p app-events
//! open "target/inset/macos/release/App events.app"
//! open -a "App events" note.insetnote     # at launch or while it runs
//! ```
//!
//! Windows and Linux start the app with what it opens on its command line, which is the app's
//! own: this one reads each argument as a file, and hands them on as an open request of its own,
//! which reaches the same handler as the system's:
//!
//! ```text
//! cargo run -p app-events -- note.insetnote
//! ```
//!
//! Click the Dock icon for a reopen, with the window on screen or minimized: each is printed
//! with whether a window was visible and where the pointer was.
//!
//! Drag anything onto the window, files, text, a link or an image from another application: each
//! report of the drag is printed with the types its items are offered in, and on the drop each
//! item is read in its first type, which the window lists.
#![feature(arbitrary_self_types)]

use std::rc::Rc;
use std::time::Duration;

use inset_embedder::{
    AppExitResponse, Color, DataTransferItem, DragData, OpenRequest, PickedFile, TextDirection,
    WindowConfig,
};
use inset_embedder_winit::{Reopen, WinitEmbedder, WinitPlatform};
use inset_foundation::{App, Completer, Entity, Listener, Task, Timer};
use inset_painting::{BoxDecoration, EdgeInsetsGeometry, TextStyle};
use inset_shell::Shell;
use inset_widgets::{
    AppLifecycleListener, BuildContext, Column, DecoratedBox, Directionality, DropListener,
    IntoWidget, Padding, StatelessWidget, Text, WidgetRef, run_app_with,
};

const BACKGROUND: Color = Color::from_argb(255, 245, 245, 240);
const INK: Color = Color::from_argb(255, 51, 51, 64);

/// How long the app takes to save before it can quit.
const SAVE_TIME: Duration = Duration::from_secs(3);

fn main() {
    WinitEmbedder.run(|platform| {
        Shell::new(platform, |app| {
            let requests = app.new_entity(|_cx| Requests::default());
            listen(app, &requests);
            open_command_line(app);
            answer_quits(app, &requests);
            let window = WindowConfig {
                title: "App events".to_owned(),
                size: [560.0, 360.0],
                ..WindowConfig::default()
            };
            run_app_with(app, home(requests), window);
        })
    });
}

/// What the system has asked of the app so far.
#[derive(Debug, Default)]
struct Requests {
    /// The files and links the system asked the app to open, as the window lists them.
    opened: Vec<String>,
    /// What was dropped on the window, each item in its first type, as the window lists it.
    dropped: Vec<String>,
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

/// Sets the handlers for what the system asks of the app, which the embedder calls from its event
/// loop; each posts its request to the app.
fn listen(app: &mut App, requests: &Entity<Requests>) {
    listen_for_opens(app, requests);
    listen_for_reopens(app, requests);
    listen_for_drags(app, requests);
}

/// The files and links the system asks the app to open, at launch and on macOS while it runs, and
/// those its command line names. A request that came before the handler was set reaches it as it
/// is set.
fn listen_for_opens(app: &mut App, requests: &Entity<Requests>) {
    let requests = requests.clone();
    app.set_open_requested_handler(Some(Rc::new(move |app: &mut App, request: OpenRequest| {
        println!("open: {request:?}");
        requests.update(app, |requests, cx| {
            requests.opened.extend(request.files.iter().map(described));
            requests.opened.extend(request.links);
            cx.notify();
        });
    })));
}

/// The files the app's command line names, as an open request of the app's own: this app takes
/// every argument for a file, in the folder it was started in.
fn open_command_line(app: &mut App) {
    let current_dir = std::env::current_dir().unwrap_or_default();
    let files = std::env::args_os()
        .skip(1)
        .map(|argument| PickedFile::Path(current_dir.join(argument)))
        .collect();
    let request = OpenRequest {
        files,
        links: Vec::new(),
    };
    if !request.is_empty() {
        app.open_requested(request);
    }
}

/// A file as the window lists it: its path, or the name the system gives it.
fn described(file: &PickedFile) -> String {
    match file {
        PickedFile::Path(path) => path.display().to_string(),
        PickedFile::Handle(handle) => handle.name().to_owned(),
        PickedFile::Stream(stream) => stream.name().to_owned(),
    }
}

/// The system asking the app to show itself again, which only macOS does.
fn listen_for_reopens(app: &mut App, requests: &Entity<Requests>) {
    let platform = app.platform();
    let Some(macos) = platform
        .downcast_ref::<WinitPlatform>()
        .and_then(WinitPlatform::macos)
    else {
        return;
    };
    let cx = app.to_async();
    let requests = requests.clone();
    macos.set_reopen_handler(Some(Rc::new(move |reopen: Reopen| {
        println!("reopen: {reopen:?}");
        let requests = requests.clone();
        cx.post(move |app| {
            requests.update(app, |requests, cx| {
                requests.reopens += 1;
                cx.notify();
            });
        });
    })));
}

/// What is dragged onto the window: each report printed, and each dropped item read.
fn listen_for_drags(app: &mut App, requests: &Entity<Requests>) {
    let requests = requests.clone();
    DropListener::new(app)
        .on_change(
            app,
            Rc::new(|_app, data: &DragData| println!("drag: {data:?}")),
        )
        .on_dropped(
            app,
            Rc::new(move |app, items| read_dropped(app, &requests, items)),
        );
}

/// Reads each of `items` in its first type, starting every read while the drop is reported, and
/// lists what each gives once it has.
fn read_dropped(app: &mut App, requests: &Entity<Requests>, items: &[Rc<dyn DataTransferItem>]) {
    for item in items {
        let Some(mime) = item.types().first().cloned() else {
            continue;
        };
        let read = item.read(&mime);
        let requests = requests.clone();
        // Dropping the task lets it run, as a Dart future does.
        let _listed = app.spawn(async move |cx| {
            let dropped = match read.await {
                Ok(content) => format!("{mime}: {}", preview(&content)),
                Err(error) => format!("{mime}: {error}"),
            };
            println!("dropped {dropped}");
            cx.update(|app| {
                requests.update(app, |requests, cx| {
                    requests.dropped.push(dropped);
                    cx.notify();
                });
            });
        });
    }
}

/// The start of `content`, as text where it is text, or else its size.
fn preview(content: &[u8]) -> String {
    match std::str::from_utf8(content) {
        Ok(text) => text
            .lines()
            .next()
            .unwrap_or_default()
            .chars()
            .take(80)
            .collect(),
        Err(_) => format!("{} bytes", content.len()),
    }
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
        if requests.opened.is_empty() {
            lines.push(line("Nothing opened yet".to_owned()));
        }
        lines.extend(
            requests
                .opened
                .iter()
                .map(|opened| line(format!("Opened {opened}"))),
        );
        lines.extend(
            requests
                .dropped
                .iter()
                .map(|dropped| line(format!("Dropped {dropped}"))),
        );
        Column::new().spacing(8.0).children(lines).into_widget()
    }
}

fn save_line(save: Save) -> &'static str {
    match save {
        Save::NotStarted => "Cmd-Q or closing the window: the app saves first, then quits",
        Save::Saving => "Quitting once saved: saving for three seconds",
        Save::Saved => "Saved: quitting",
    }
}

fn line(text: String) -> WidgetRef {
    Text::new(text)
        .style(TextStyle::new().font_size(15.0).color(INK))
        .into_widget()
}
