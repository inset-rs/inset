//! ContentDialog's command buttons, keys, cancelable closing, one open dialog per window and the
//! result delivered after the close transition.
#![feature(arbitrary_self_types)]
mod common;
use common::GalleryFixtureExt;

use common::Fixture;
use inset_foundation::{App, CompleterFuture, Handle};
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey, PhysicalKeyboardKey,
};
use inset_widgets::*;
use inset_winui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use winui_gallery::Feature;

/// A window with one page whose context shows the dialogs, and what they reported.
#[derive(Clone, Default)]
struct Probe {
    first: Rc<Cell<Option<Handle<ContentDialog>>>>,
    second: Rc<Cell<Option<Handle<ContentDialog>>>>,
    page: Rc<Cell<Option<BuildContext>>>,
    events: Rc<RefCell<Vec<String>>>,
}

fn fixture(theme: Theme) -> (Fixture, Probe) {
    let probe = Probe::default();
    let shared = probe.clone();
    let fixture = Fixture::with_root([640, 480], move |app| {
        winui_gallery::install_fonts(app);
        let first = reporting_dialog(app, "First", &shared.events);
        let second = reporting_dialog(app, "Second", &shared.events);
        shared.first.set(Some(first));
        shared.second.set(Some(second));
        let page = shared.page.clone();
        run_app(
            app,
            WidgetsApp::new(AccentPalette::default().base)
                .debug_show_checked_mode_banner(false)
                .page_route_builder(|app, settings, builder| {
                    let route = PageRouteBuilder::new(
                        app,
                        Rc::new(move |app, context, _, _| builder(app, context)),
                    )
                    .settings(app, RouteSettingsRef::Settings(settings.clone()));
                    PageRoute::as_page_route(route)
                })
                .home(ThemeScope::new(
                    theme,
                    Builder::new(move |_, context| {
                        page.set(Some(context));
                        ColoredBox::new(
                            ThemeResources::new(theme, AccentPalette::default())
                                .solid_background_fill_color_base,
                        )
                        .child(SizedBox::expand())
                        .into_widget()
                    }),
                ))
                .into_widget(),
        );
    });
    (fixture, probe)
}

/// A dialog titled `name` with all three buttons, recording its Closing and Closed events.
fn reporting_dialog(
    app: &mut App,
    name: &str,
    events: &Rc<RefCell<Vec<String>>>,
) -> Handle<ContentDialog> {
    let dialog = ContentDialog::new(app)
        .title(app, Text::new(name.to_owned()))
        .content(app, Text::new("Both answers keep the draft."))
        .primary_button_text(app, "Keep")
        .secondary_button_text(app, "Send")
        .close_button_text(app, "Cancel")
        .default_button(app, ContentDialogButton::Primary);
    let closing = events.clone();
    app.get_mut(dialog).closing = Some(Rc::new(move |_, args| {
        closing
            .borrow_mut()
            .push(format!("closing {:?}", args.result()));
    }));
    let closed = events.clone();
    app.get_mut(dialog).closed = Some(Rc::new(move |_, args| {
        closed
            .borrow_mut()
            .push(format!("closed {:?}", args.result));
    }));
    dialog
}

fn show(
    f: &mut Fixture,
    dialog: Handle<ContentDialog>,
    probe: &Probe,
) -> Result<CompleterFuture<ContentDialogResult>, ContentDialogError> {
    let context = probe.page.get().unwrap();
    let shown = dialog.show_async(&mut f.cell.borrow_mut(), context);
    f.cell.checkpoint();
    settle(f);
    shown
}

fn key(f: &mut Fixture, logical: LogicalKeyboardKey, physical: PhysicalKeyboardKey) {
    for event in [
        KeyEvent::Down(KeyDownEvent::new(physical, logical, f.at)),
        KeyEvent::Up(KeyUpEvent::new(physical, logical, f.at)),
    ] {
        let mut app = f.cell.borrow_mut();
        HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &event);
        drop(app);
        f.cell.checkpoint();
    }
    settle(f);
}

/// Runs frames until every transition has played.
fn settle(f: &mut Fixture) {
    for _ in 0..3 {
        f.pump();
    }
}

/// Taps the label reading `text` and lets the transitions it starts play.
fn tap(f: &mut Fixture, text: &str) {
    f.tap(text);
    settle(f);
}

/// Whether a label reading `text` is on screen.
fn shows(f: &Fixture, text: &str) -> bool {
    let elements = f.elements();
    let app = f.cell.borrow();
    elements.into_iter().any(|element| {
        element
            .render_object(&app)
            .and_then(|object| object.downcast::<inset_rendering::RenderParagraph>(&app))
            .is_some_and(|paragraph| paragraph.text(&app).to_plain_text(true, true) == text)
    })
}

#[test]
fn a_command_button_closes_with_its_result_after_closing_and_closed() {
    let mut f = Fixture::for_feature([900, 700], Feature::ContentDialog);
    tap(&mut f, "Show dialog");
    f.find("Save your work?");
    f.capture("content_dialog_light");
    tap(&mut f, "Don't save");
    assert!(!shows(&f, "Save your work?"));
    f.find("Closed: Secondary");
}

#[test]
fn escape_presses_the_close_button_and_enter_the_default_button() {
    let mut f = Fixture::for_feature([900, 700], Feature::ContentDialog);
    tap(&mut f, "Show dialog");
    key(
        &mut f,
        LogicalKeyboardKey::ESCAPE,
        PhysicalKeyboardKey::ESCAPE,
    );
    f.find("Closed: None");
    tap(&mut f, "Show dialog");
    key(
        &mut f,
        LogicalKeyboardKey::ENTER,
        PhysicalKeyboardKey::ENTER,
    );
    f.find("Closed: Primary");
}

#[test]
fn the_content_takes_the_initial_focus_and_closing_can_keep_the_dialog_open() {
    let mut f = Fixture::for_feature([900, 700], Feature::ContentDialog);
    tap(&mut f, "Create a collection");
    {
        let mut app = f.cell.borrow_mut();
        let focused = inset_widgets::primary_focus(&mut app).expect("a focused element");
        let context = focused.context(&app).expect("a mounted focus node");
        assert!(
            context
                .find_ancestor_widget_of_exact_type::<EditableText>(&app)
                .is_some()
                || inset_widgets::downcast_widget::<EditableText>(context.widget(&app).as_ref())
                    .is_some(),
            "the name field takes the initial focus"
        );
    }
    tap(&mut f, "Create");
    f.find("New collection");
    f.find("A collection needs a name");
    tap(&mut f, "Cancel");
    f.find("Closed: None");
}

#[test]
fn only_one_dialog_opens_in_a_window_until_the_first_is_closing() {
    let (mut f, probe) = fixture(Theme::Light);
    let (first, second) = (probe.first.get().unwrap(), probe.second.get().unwrap());
    let result = show(&mut f, first, &probe).expect("the first dialog opens");
    assert_eq!(
        show(&mut f, first, &probe).err(),
        Some(ContentDialogError::MultipleOpen)
    );
    assert_eq!(
        show(&mut f, second, &probe).err(),
        Some(ContentDialogError::MultipleOpen)
    );
    tap(&mut f, "Send");
    assert_eq!(result.peek(), Some(ContentDialogResult::Secondary));
    assert_eq!(
        &*probe.events.borrow(),
        &["closing Secondary", "closed Secondary"]
    );
    show(&mut f, second, &probe).expect("the second dialog opens once the first has closed");
    f.find("Second");
}

#[test]
fn hide_closes_with_none_and_the_result_follows_the_transition() {
    let (mut f, probe) = fixture(Theme::Dark);
    let dialog = probe.first.get().unwrap();
    let result = show(&mut f, dialog, &probe).unwrap();
    f.capture("content_dialog_dark");
    dialog.hide(&mut f.cell.borrow_mut());
    f.cell.checkpoint();
    assert_eq!(result.peek(), None, "the close transition has not played");
    settle(&mut f);
    assert_eq!(result.peek(), Some(ContentDialogResult::None));
    assert_eq!(&*probe.events.borrow(), &["closing None", "closed None"]);
}

#[test]
fn disposing_a_shown_dialog_closes_it_without_events() {
    let (mut f, probe) = fixture(Theme::Light);
    let dialog = probe.first.get().unwrap();
    let result = show(&mut f, dialog, &probe).unwrap();
    dialog.dispose(&mut f.cell.borrow_mut());
    f.cell.checkpoint();
    settle(&mut f);
    assert_eq!(result.peek(), Some(ContentDialogResult::None));
    assert!(probe.events.borrow().is_empty());
    assert!(!f.cell.borrow().contains(dialog));
    assert!(!shows(&f, "First"));
}

#[test]
fn the_default_button_takes_the_initial_focus_without_focusable_content() {
    let (mut f, probe) = fixture(Theme::Light);
    show(&mut f, probe.first.get().unwrap(), &probe).unwrap();
    key(
        &mut f,
        LogicalKeyboardKey::SPACE,
        PhysicalKeyboardKey::SPACE,
    );
    assert_eq!(
        &*probe.events.borrow(),
        &["closing Primary", "closed Primary"]
    );
}

#[test]
fn a_dialog_hidden_in_the_frame_it_was_shown_still_closes() {
    let (mut f, probe) = fixture(Theme::Light);
    let dialog = probe.first.get().unwrap();
    let result = {
        let mut app = f.cell.borrow_mut();
        let shown = dialog.show_async(&mut app, probe.page.get().unwrap());
        dialog.hide(&mut app);
        shown.unwrap()
    };
    f.cell.checkpoint();
    settle(&mut f);
    assert_eq!(result.peek(), Some(ContentDialogResult::None));
    assert_eq!(&*probe.events.borrow(), &["closed None"]);
    show(&mut f, dialog, &probe).expect("the dialog opens again");
}

#[test]
fn a_deferred_click_closes_with_its_button_once_the_deferral_completes() {
    let (mut f, probe) = fixture(Theme::Light);
    let dialog = probe.first.get().unwrap();
    let pending = Rc::new(RefCell::new(None));
    let taken = pending.clone();
    f.cell.borrow_mut().get_mut(dialog).primary_button_click = Some(Rc::new(move |_, args| {
        *taken.borrow_mut() = Some(args.get_deferral().expect("a deferral during the click"));
    }));
    let result = show(&mut f, dialog, &probe).unwrap();
    tap(&mut f, "Keep");
    assert!(shows(&f, "First"), "the click waits for its deferral");
    assert!(probe.events.borrow().is_empty());
    let deferral = pending.borrow_mut().take().unwrap();
    deferral.complete(&mut f.cell.borrow_mut()).unwrap();
    f.cell.checkpoint();
    settle(&mut f);
    assert_eq!(result.peek(), Some(ContentDialogResult::Primary));
    assert_eq!(
        &*probe.events.borrow(),
        &["closing Primary", "closed Primary"]
    );
    assert_eq!(
        deferral.complete(&mut f.cell.borrow_mut()),
        Err(DeferralCompleted)
    );
}

#[test]
fn closing_waits_for_its_deferral_which_can_still_keep_the_dialog_open() {
    let (mut f, probe) = fixture(Theme::Light);
    let dialog = probe.first.get().unwrap();
    let pending = Rc::new(RefCell::new(None));
    let taken = pending.clone();
    f.cell.borrow_mut().get_mut(dialog).closing = Some(Rc::new(move |_, args| {
        let deferral = args.get_deferral().expect("a deferral during Closing");
        *taken.borrow_mut() = Some((Rc::clone(args), deferral));
    }));
    let result = show(&mut f, dialog, &probe).unwrap();
    dialog.hide(&mut f.cell.borrow_mut());
    f.cell.checkpoint();
    settle(&mut f);
    assert!(shows(&f, "First"), "the close waits for its deferral");
    let (args, deferral) = pending.borrow_mut().take().unwrap();
    args.set_cancel(true);
    deferral.complete(&mut f.cell.borrow_mut()).unwrap();
    f.cell.checkpoint();
    settle(&mut f);
    assert!(shows(&f, "First"), "the canceled close keeps it open");
    f.cell.borrow_mut().get_mut(dialog).closing = None;
    dialog.hide(&mut f.cell.borrow_mut());
    f.cell.checkpoint();
    settle(&mut f);
    assert_eq!(result.peek(), Some(ContentDialogResult::None));
}

#[test]
fn the_result_arrives_as_the_167_ms_close_transition_ends() {
    let (mut f, probe) = fixture(Theme::Light);
    let dialog = probe.first.get().unwrap();
    let result = show(&mut f, dialog, &probe).unwrap();
    dialog.hide(&mut f.cell.borrow_mut());
    f.cell.checkpoint();
    f.frame_after(std::time::Duration::from_millis(16));
    f.frame_after(std::time::Duration::from_millis(150));
    assert_eq!(result.peek(), None, "150 ms into the close transition");
    f.frame_after(std::time::Duration::from_millis(20));
    assert_eq!(result.peek(), Some(ContentDialogResult::None));
}

#[test]
fn arrow_keys_move_between_the_command_buttons() {
    let (mut f, probe) = fixture(Theme::Light);
    show(&mut f, probe.first.get().unwrap(), &probe).unwrap();
    key(
        &mut f,
        LogicalKeyboardKey::ARROW_RIGHT,
        PhysicalKeyboardKey::ARROW_RIGHT,
    );
    key(
        &mut f,
        LogicalKeyboardKey::SPACE,
        PhysicalKeyboardKey::SPACE,
    );
    assert_eq!(
        &*probe.events.borrow(),
        &["closing Secondary", "closed Secondary"]
    );
}
