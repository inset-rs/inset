//! The system's menus through Inset's widgets: a `PlatformMenuBar` with the application menu's
//! provided About, Hide and Quit, a File menu whose New has ⌘N, a View menu whose checkbox
//! follows the app's state, and the provided Edit items, which act on the text field; a
//! `PlatformTrayIcon` whose menu shows the window or quits, opened on each system's own trigger,
//! and whose click, where the system reports one, brings the window forward; and the system's
//! own context menu on a right click. The window shows the last choice.
//!
//! Each place is the system's own or none: where the platform answers `None` for one, as Windows
//! and Linux have no menu bar and Linux no context menu of the system's, the window says so in
//! its place, and an app would draw its own there.
//!
//! ```text
//! cargo run -p menus
//! ```
#![feature(arbitrary_self_types)]

use std::rc::Rc;

use inset_cupertino::{CupertinoApp, CupertinoTextField};
use inset_embedder::{
    AppExitType, CheckState, Color, IconImage, MenuEntry, MenuItem, MenuItemCheck, Offset,
    ViewFocusDirection, ViewFocusState, ViewId, WindowConfig,
};
use inset_embedder_winit::WinitEmbedder;
use inset_foundation::{App, Entity, VoidCallback};
use inset_gestures::TapUpDetails;
use inset_painting::{BoxDecoration, EdgeInsetsGeometry, TextStyle};
use inset_services::{LogicalKeyboardKey, ServicesBinding};
use inset_shell::Shell;
use inset_widgets::{
    BuildContext, Column, DecoratedBox, GestureDetector, IntoWidget, Padding,
    PlainPlatformMenuItem, PlatformMenu, PlatformMenuBar, PlatformMenuItemGroup,
    PlatformMenuItemRef, PlatformProvidedMenuItem, PlatformProvidedMenuItemType, PlatformTrayIcon,
    SingleActivator, SizedBox, StatelessWidget, Text, View, WidgetRef, run_app_with,
};

const BACKGROUND: Color = Color::from_argb(255, 245, 245, 240);
const INK: Color = Color::from_argb(255, 51, 51, 64);
const COLOURS: [&str; 3] = ["Red", "Green", "Blue"];

/// What the menus have done: the last item chosen, and the View menu's checkbox.
#[derive(Default)]
struct Choices {
    last: String,
    show_hints: bool,
}

fn main() {
    WinitEmbedder.run(|platform| {
        Shell::new(platform, |app| {
            let choices = app.new_entity(|_cx| Choices {
                last: "none yet".to_owned(),
                show_hints: true,
            });
            let window = WindowConfig {
                title: "Inset — menus".to_owned(),
                size: [520.0, 360.0],
                ..WindowConfig::default()
            };
            let home = CupertinoApp::new().title("Menus").home(Menus { choices });
            run_app_with(app, home.into_widget(), window);
        })
    });
}

#[derive(Debug)]
struct Menus {
    choices: Entity<Choices>,
}

impl StatelessWidget for Menus {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let platform = app.platform();
        let view = View::of(app, context).id();
        let choices = self.choices.read(app);
        let mut lines = vec![line(format!("Last choice: {}", choices.last))];
        let show_hints = choices.show_hints;
        if platform.menu_bar().is_none() {
            lines.push(line(
                "This system has no menu bar of its own: an app draws its own.",
            ));
        }
        if platform.tray_icons().is_none() {
            lines.push(line("This system shows no tray icon."));
        }
        if platform.popup_menus().is_none() {
            lines.push(line("This system has no context menu of its own."));
        } else if show_hints {
            lines.push(context_area(&self.choices, view));
        }
        if show_hints {
            lines.push(line("Try Edit's items, and ⌘C and ⌘V, in the field:"));
        }
        lines.push(
            CupertinoTextField::new()
                .placeholder("Some text")
                .into_widget(),
        );
        let content = DecoratedBox::new(BoxDecoration::new().color(BACKGROUND)).child(
            Padding::new(EdgeInsetsGeometry::all(24.0))
                .child(Column::new().spacing(12.0).children(lines)),
        );
        let tray_icon = PlatformTrayIcon::new(dot())
            .tooltip("Inset menus")
            .menus(vec![
                item("Show Window", Rc::new(bring_window_forward)),
                item("Quit", Rc::new(quit)),
            ])
            .on_activated(Rc::new(bring_window_forward))
            .child(content);
        PlatformMenuBar::new(menu_bar(&self.choices, show_hints))
            .child(tray_icon)
            .into_widget()
    }
}

/// The application menu, File, View with its checkbox, and the system's Edit.
fn menu_bar(choices: &Entity<Choices>, show_hints: bool) -> Vec<PlatformMenuItemRef> {
    let provided = |kind| -> PlatformMenuItemRef { Rc::new(PlatformProvidedMenuItem::new(kind)) };
    let new = PlainPlatformMenuItem::new("New")
        .shortcut(Rc::new(
            SingleActivator::new(LogicalKeyboardKey::KEY_N).meta(true),
        ))
        .on_selected(choose(choices, "File › New"));
    let toggled = choices.clone();
    let hints = PlainPlatformMenuItem::new("Show Hints")
        .check(MenuItemCheck::Checkbox(if show_hints {
            CheckState::On
        } else {
            CheckState::Off
        }))
        .on_selected(Rc::new(move |app| {
            toggled.update(app, |choices, cx| {
                choices.show_hints = !choices.show_hints;
                choices.last = "View › Show Hints".to_owned();
                cx.notify();
            });
        }));
    use PlatformProvidedMenuItemType as Provided;
    vec![
        Rc::new(PlatformMenu::new(
            "Menus",
            vec![
                provided(Provided::About),
                provided(Provided::Hide),
                provided(Provided::Quit),
            ],
        )),
        Rc::new(PlatformMenu::new("File", vec![Rc::new(new)])),
        Rc::new(PlatformMenu::new("View", vec![Rc::new(hints)])),
        Rc::new(PlatformMenu::new(
            "Edit",
            vec![
                Rc::new(PlatformMenuItemGroup::new(vec![
                    provided(Provided::Undo),
                    provided(Provided::Redo),
                ])),
                provided(Provided::Cut),
                provided(Provided::Copy),
                provided(Provided::Paste),
                provided(Provided::SelectAll),
            ],
        )),
    ]
}

/// An area whose right click opens the system's context menu of colours.
fn context_area(choices: &Entity<Choices>, view: ViewId) -> WidgetRef {
    let choices = choices.clone();
    GestureDetector::new()
        .on_secondary_tap_up(Rc::new(move |app, details: TapUpDetails| {
            open_context_menu(app, &choices, view, details.global_position)
        }))
        .child(
            SizedBox::new()
                .height(28.0)
                .child(line("Right-click here for a context menu.")),
        )
        .into_widget()
}

fn open_context_menu(app: &mut App, choices: &Entity<Choices>, view: ViewId, at: Offset) {
    let platform = app.platform();
    let Some(popup_menus) = platform.popup_menus() else {
        return;
    };
    let entries: Vec<MenuEntry> = (1..)
        .zip(COLOURS)
        .map(|(id, label)| MenuEntry::Item {
            id,
            item: MenuItem::new(label),
        })
        .collect();
    let shown = popup_menus.show(view, at, &entries);
    let choices = choices.clone();
    app.spawn(async move |cx| {
        if let Some(id) = shown.await {
            let colour = COLOURS[id as usize - 1];
            cx.update(|app| choose(&choices, colour)(app));
        }
    });
}

fn item(label: &str, on_selected: VoidCallback) -> PlatformMenuItemRef {
    Rc::new(PlainPlatformMenuItem::new(label).on_selected(on_selected))
}

/// Notes `what` as the last choice.
fn choose(choices: &Entity<Choices>, what: &'static str) -> VoidCallback {
    let choices = choices.clone();
    Rc::new(move |app| {
        choices.update(app, |choices, cx| {
            choices.last = what.to_owned();
            cx.notify();
        });
    })
}

/// Shows the app's window and gives it the focus, which brings it in front.
fn bring_window_forward(app: &mut App) {
    let platform = app.platform();
    for view in platform.views() {
        if let Some(window) = view.window() {
            window.show();
        }
        platform.request_view_focus_change(
            view.id(),
            ViewFocusState::Focused,
            ViewFocusDirection::Undefined,
        );
    }
}

/// Asks to quit, as the app menu's Quit does: the app's exit listeners hear it first.
fn quit(app: &mut App) {
    ServicesBinding::instance(app).exit_application(app, AppExitType::Cancelable, 0);
}

/// A filled circle drawn by its alpha alone, as macOS draws a template image in its menu bar.
fn dot() -> IconImage {
    const SIZE: u32 = 18;
    let centre = f64::from(SIZE) / 2.0;
    let rgba: Vec<u8> = (0..SIZE * SIZE)
        .flat_map(|pixel| {
            let (x, y) = (f64::from(pixel % SIZE) + 0.5, f64::from(pixel / SIZE) + 0.5);
            let inside = (x - centre).hypot(y - centre) <= 6.0;
            [0, 0, 0, if inside { 255 } else { 0 }]
        })
        .collect();
    IconImage {
        width: SIZE,
        height: SIZE,
        rgba: rgba.into(),
        template: true,
    }
}

fn line(text: impl Into<String>) -> WidgetRef {
    Text::new(text.into())
        .style(TextStyle::new().font_size(15.0).color(INK))
        .into_widget()
}
