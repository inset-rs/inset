//! Runs a small widget tree in two windows, each an app of its own on Windows and Linux: a padded,
//! centered, decorated card naming its window and holding a `CupertinoButton` that fades while
//! pressed and counts its presses on stdout. The card is a mouse region that shows the
//! pointing-hand cursor while hovered.
//!
//! Each window carries an identity of its own, `rs.waft.window.<name>`, apart from the app's: on
//! Windows its AppUserModelID, with what a pin of it starts, this executable with its name; on
//! Linux its app id, which a desktop file `rs.waft.window.<name>.desktop` gives a name and an icon.
//! The taskbar or the dock then shows each window as an app of its own, and pins it as one.
//!
//! The command line names the windows to open, `alpha` and `beta` by default. The app runs once: a
//! later launch, such as a pin clicked while it runs, hands its names to it, which brings each
//! named window forward, with the focus the launch was given, or opens it.
//!
//! ```text
//! cargo run -p window
//! cargo run -p window -- beta
//! ```
#![windows_subsystem = "windows"]
#![feature(arbitrary_self_types)]

use std::cell::Cell;
use std::ffi::OsString;
use std::rc::Rc;

use waft_cupertino::{CupertinoButton, CupertinoTheme, CupertinoThemeData};
use waft_embedder::{
    Brightness, Color, LinuxWindowConfig, TextDirection, ViewFocusDirection, ViewFocusState,
    ViewRef, WindowConfig, WindowsRelaunch, WindowsWindowConfig,
};
use waft_embedder_winit::{LinuxSettings, WindowsSettings, WinitEmbedder, WinitPlatform};
use waft_foundation::{App, Listener};
use waft_painting::{BoxDecoration, EdgeInsetsGeometry, TextStyle};
use waft_rendering::MainAxisSize;
use waft_services::SystemMouseCursors;
use waft_shell::Shell;
use waft_single_instance::{FirstInstance, Launch, SingleInstance, single_instance};
use waft_widgets::{
    Center, Column, DecoratedBox, Directionality, IntoWidget, MouseRegion, MultiViewApp, Padding,
    SizedBox, Text, WidgetRef, run_widget,
};

const BACKGROUND: Color = Color::from_argb(255, 245, 245, 240);
const CARD: Color = Color::from_argb(255, 51, 51, 64);
const LABEL: Color = Color::from_argb(255, 245, 245, 240);

/// The app's own identity, which every window without one of its own shows under.
const APP_ID: &str = waft_macros::linux_app_id!();
/// The windows a launch that names none opens.
const DEFAULT_WINDOWS: [&str; 2] = ["alpha", "beta"];

fn main() {
    WinitEmbedder::attach_to_parent_console();
    let first = match single_instance(APP_ID) {
        Ok(SingleInstance::First(first)) => Some(first),
        // The first instance has this launch's names.
        Ok(SingleInstance::Second) => return,
        Err(_) => None,
    };
    let names = window_names(std::env::args_os().skip(1));
    WinitEmbedder::default()
        .linux(LinuxSettings {
            app_id: Some(APP_ID.to_owned()),
        })
        .windows(WindowsSettings {
            app_user_model_id: Some(APP_ID.to_owned()),
        })
        .run(move |platform| {
            Shell::new(platform, move |app| {
                for name in &names {
                    open_window(app, name, None);
                }
                if let Some(first) = first {
                    hear_later_launches(app, first);
                }
                let windows = MultiViewApp::new(|_app, view| Some(home(&name_of(view))));
                run_widget(app, windows.into_widget());
            })
        });
}

/// The windows a command line names; the default ones where it names none.
fn window_names(arguments: impl IntoIterator<Item = OsString>) -> Vec<String> {
    let names: Vec<String> = arguments
        .into_iter()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect();
    if names.is_empty() {
        DEFAULT_WINDOWS.map(str::to_owned).to_vec()
    } else {
        names
    }
}

/// The identity of the window `name`, an app of its own to the taskbar and the dock.
fn identity_of(name: &str) -> String {
    format!("{APP_ID}.{name}")
}

/// The window's name, which its view carries as its initial data.
fn name_of(view: &ViewRef) -> String {
    view.initial_data().unwrap_or_default().to_owned()
}

/// Asks for the window `name`, as an app of its own, taking the focus with `activation_token`,
/// the one a later launch was given on Linux.
fn open_window(app: &mut App, name: &str, activation_token: Option<String>) {
    let config = WindowConfig {
        title: format!("Waft — {name}"),
        size: [480.0, 320.0],
        windows: WindowsWindowConfig {
            app_user_model_id: Some(identity_of(name)),
            relaunch: Some(WindowsRelaunch {
                command: relaunch_command(name),
                display_name: format!("Waft {name}"),
                icon: None,
            }),
        },
        linux: LinuxWindowConfig {
            app_id: Some(identity_of(name)),
            activation_token,
        },
        ..WindowConfig::default()
    };
    if let Err(error) = app.platform().request_view(config, Some(name.to_owned())) {
        println!("window: {name} was refused: {error}");
    }
}

/// This executable, quoted, with the window's name: what a pin of the window starts.
fn relaunch_command(name: &str) -> String {
    let executable = std::env::current_exe().unwrap_or_default();
    format!("\"{}\" {name}", executable.display())
}

/// Brings forward or opens the windows each later launch names, as they come.
fn hear_later_launches(app: &mut App, first: FirstInstance) {
    let mut launches = first.launches();
    app.spawn(async move |cx| {
        loop {
            let launch = launches.next_launch().await;
            cx.update(|app| show_launched(app, launch));
        }
    });
}

/// Brings forward each window `launch` names, with the focus the launch was given, or opens it.
fn show_launched(app: &mut App, launch: Launch) {
    for name in window_names(launch.arguments) {
        let open = app
            .platform()
            .views()
            .into_iter()
            .find(|view| name_of(view) == name);
        match open {
            Some(view) => bring_forward(app, &view, launch.activation_token.as_deref()),
            None => open_window(app, &name, launch.activation_token.clone()),
        }
    }
}

/// Brings `view`'s window forward: on Linux with the launch's activation token, which the desktop
/// asks for; elsewhere by asking for the focus, which on Windows the launch let this process take.
fn bring_forward(app: &mut App, view: &ViewRef, activation_token: Option<&str>) {
    let Some(window) = view.window() else {
        return;
    };
    window.show();
    let platform = app.platform();
    let linux = platform
        .downcast_ref::<WinitPlatform>()
        .and_then(WinitPlatform::linux);
    match (linux, activation_token) {
        (Some(linux), Some(token)) => linux.activate(&window, token),
        _ => view
            .host()
            .request_focus_change(ViewFocusState::Focused, ViewFocusDirection::Undefined),
    }
}

/// What `WidgetsApp` would supply: the ambient reading direction the text needs.
fn home(name: &str) -> WidgetRef {
    Directionality::new(TextDirection::Ltr, background(name)).into_widget()
}

fn background(name: &str) -> WidgetRef {
    DecoratedBox::new(BoxDecoration::new().color(BACKGROUND))
        .child(Padding::new(EdgeInsetsGeometry::all(24.0)).child(Center::new().child(card(name))))
        .into_widget()
}

fn card(name: &str) -> WidgetRef {
    MouseRegion::new()
        .cursor(SystemMouseCursors::CLICK.into())
        .child(
            SizedBox::new().width(240.0).height(140.0).child(
                DecoratedBox::new(BoxDecoration::new().color(CARD)).child(
                    Center::new().child(
                        Column::new()
                            .main_axis_size(MainAxisSize::Min)
                            .spacing(12.0)
                            .children([label(name), button()]),
                    ),
                ),
            ),
        )
        .into_widget()
}

/// A filled iOS button under a dark Cupertino theme, counting its presses.
fn button() -> WidgetRef {
    let presses = Rc::new(Cell::new(0));
    let on_pressed = Listener::new(move |_app| {
        presses.set(presses.get() + 1);
        println!("pressed {} time(s)", presses.get());
    });
    CupertinoTheme::new(
        CupertinoThemeData::new().with_brightness(Brightness::Dark),
        CupertinoButton::filled(label("Press me"), Some(on_pressed)),
    )
    .into_widget()
}

fn label(text: &str) -> WidgetRef {
    Text::new(text.to_owned())
        .style(TextStyle::new().font_size(20.0).color(LABEL))
        .into_widget()
}
