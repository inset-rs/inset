//! Public Flyout placement, closing policy and retained presenter integration.
#![feature(arbitrary_self_types)]
mod common;
use common::GalleryFixtureExt;

use common::Fixture;
use inset_foundation::{Handle, Listener};
use inset_painting::Alignment;
use inset_widgets::*;
use inset_winui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Default)]
struct Probe {
    flyout: Rc<Cell<Option<Handle<Flyout>>>>,
    target: Rc<Cell<Option<BuildContext>>>,
    events: Rc<RefCell<Vec<&'static str>>>,
    outside: Rc<Cell<usize>>,
    cancel: Rc<Cell<bool>>,
}

fn fixture(mode: FlyoutShowMode) -> (Fixture, Probe) {
    themed_fixture(mode, Theme::Light)
}

fn themed_fixture(mode: FlyoutShowMode, theme: Theme) -> (Fixture, Probe) {
    let probe = Probe::default();
    let shared = probe.clone();
    let fixture = Fixture::with_root([500, 400], move |app| {
        winui_gallery::install_fonts(app);
        let flyout = Flyout::new(app, Button::text("Popup action", Listener::new(|_| {})))
            .show_mode(app, mode);
        shared.flyout.set(Some(flyout));
        let events = shared.events.clone();
        app.get_mut(flyout).opening =
            Some(Listener::new(move |_| events.borrow_mut().push("opening")));
        let events = shared.events.clone();
        app.get_mut(flyout).opened =
            Some(Listener::new(move |_| events.borrow_mut().push("opened")));
        let events = shared.events.clone();
        let cancel = shared.cancel.clone();
        app.get_mut(flyout).closing = Some(Rc::new(move |_, args| {
            events.borrow_mut().push("closing");
            args.cancel = cancel.get();
        }));
        let events = shared.events.clone();
        app.get_mut(flyout).closed =
            Some(Listener::new(move |_| events.borrow_mut().push("closed")));
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| {
                let outside = shared.outside.clone();
                let target = shared.target.clone();
                ThemeScope::new(
                    theme,
                    Stack::new().children([
                        Align::new()
                            .alignment(Alignment::CENTER.into())
                            .child(FlyoutTarget::new(Builder::new(move |_, context| {
                                target.set(Some(context));
                                DropDownButton::text("Open flyout")
                                    .flyout(flyout)
                                    .into_widget()
                            })))
                            .into_widget(),
                        Align::new()
                            .alignment(Alignment::BOTTOM_LEFT.into())
                            .child(Button::text(
                                "Outside action",
                                Listener::new(move |_| outside.set(outside.get() + 1)),
                            ))
                            .into_widget(),
                    ]),
                )
                .into_widget()
            }),
            false,
            true,
            false,
        );
        run_app(
            app,
            WidgetsApp::new(AccentPalette::default().base)
                .debug_show_checked_mode_banner(false)
                .builder(move |_, _, _| Overlay::new().initial_entries([entry]).into_widget())
                .into_widget(),
        );
    });
    (fixture, probe)
}

#[test]
fn standard_dismissal_consumes_outside_tap_and_orders_events() {
    let (mut f, probe) = fixture(FlyoutShowMode::Standard);
    f.tap("Open flyout");
    f.find("Popup action");
    assert_eq!(&*probe.events.borrow(), &["opening", "opened"]);
    f.capture("flyout_standard");
    f.tap("Outside action");
    assert_eq!(probe.outside.get(), 0);
    assert!(!probe.flyout.get().unwrap().is_open(&f.cell.borrow()));
    assert_eq!(
        &*probe.events.borrow(),
        &["opening", "opened", "closing", "closed"]
    );
}

#[test]
fn transient_dismissal_allows_outside_action() {
    let (mut f, probe) = fixture(FlyoutShowMode::Transient);
    f.tap("Open flyout");
    f.tap("Outside action");
    assert_eq!(probe.outside.get(), 1);
    assert!(!probe.flyout.get().unwrap().is_open(&f.cell.borrow()));
}

#[test]
fn closing_handler_can_keep_presenter_open() {
    let (mut f, probe) = fixture(FlyoutShowMode::Standard);
    probe.cancel.set(true);
    f.tap("Open flyout");
    f.tap("Outside action");
    assert!(probe.flyout.get().unwrap().is_open(&f.cell.borrow()));
    assert_eq!(&*probe.events.borrow(), &["opening", "opened", "closing"]);
}

#[test]
fn nested_presenter_is_inside_parent_but_parent_is_outside_child() {
    let (mut f, probe) = fixture(FlyoutShowMode::Standard);
    let parent = probe.flyout.get().unwrap();
    let child = {
        let mut app = f.cell.borrow_mut();
        let child = Flyout::new(
            &mut app,
            Button::text("Nested action", Listener::new(|_| {})),
        );
        parent.content(
            &mut app,
            FlyoutTarget::new(Builder::new(move |_, context| {
                Button::text(
                    "Open nested",
                    Listener::new(move |app| child.show_at(app, context)),
                )
                .into_widget()
            })),
        );
        child
    };
    f.tap("Open flyout");
    f.tap("Open nested");
    f.tap("Nested action");
    assert!(parent.is_open(&f.cell.borrow()));
    assert!(child.is_open(&f.cell.borrow()));
    f.tap("Open nested");
    assert!(parent.is_open(&f.cell.borrow()));
    assert!(!child.is_open(&f.cell.borrow()));
    f.tap("Outside action");
    assert!(!parent.is_open(&f.cell.borrow()));
}

#[test]
fn opening_another_flyout_waits_for_closed_event() {
    let (mut f, probe) = fixture(FlyoutShowMode::Standard);
    f.tap("Open flyout");
    let first = probe.flyout.get().unwrap();
    let second = {
        let mut app = f.cell.borrow_mut();
        let second = Flyout::new(
            &mut app,
            Button::text("Second popup", Listener::new(|_| {})),
        );
        let events = probe.events.clone();
        app.get_mut(second).opening = Some(Listener::new(move |_| {
            events.borrow_mut().push("second opening")
        }));
        second.show_at(&mut app, probe.target.get().unwrap());
        second
    };
    assert!(!first.is_open(&f.cell.borrow()));
    assert!(!second.is_open(&f.cell.borrow()));
    f.pump();
    assert!(second.is_open(&f.cell.borrow()));
    f.find("Second popup");
    assert_eq!(
        &*probe.events.borrow(),
        &["opening", "opened", "closing", "closed", "second opening"]
    );
}

#[test]
fn closed_callback_can_replace_a_staged_request() {
    let (mut f, probe) = fixture(FlyoutShowMode::Standard);
    f.tap("Open flyout");
    let first = probe.flyout.get().unwrap();
    let (second, third) = {
        let mut app = f.cell.borrow_mut();
        let second = Flyout::new(&mut app, Text::new("Superseded"));
        let third = Flyout::new(&mut app, Text::new("Latest request"));
        let target = probe.target.get().unwrap();
        app.get_mut(first).closed = Some(Listener::new(move |app| third.show_at(app, target)));
        second.show_at(&mut app, target);
        (second, third)
    };
    f.pump();
    assert!(!second.is_open(&f.cell.borrow()));
    assert!(third.is_open(&f.cell.borrow()));
    f.find("Latest request");
}

#[test]
fn canceling_close_keeps_the_staged_request_until_a_later_close() {
    let (mut f, probe) = fixture(FlyoutShowMode::Standard);
    f.tap("Open flyout");
    probe.cancel.set(true);
    let first = probe.flyout.get().unwrap();
    let second = {
        let mut app = f.cell.borrow_mut();
        let second = Flyout::new(&mut app, Text::new("Waiting request"));
        second.show_at(&mut app, probe.target.get().unwrap());
        second
    };
    f.pump();
    assert!(first.is_open(&f.cell.borrow()));
    assert!(!second.is_open(&f.cell.borrow()));
    probe.cancel.set(false);
    first.hide(&mut f.cell.borrow_mut());
    f.pump();
    assert!(second.is_open(&f.cell.borrow()));
    f.find("Waiting request");
}

#[test]
fn gallery_pages_open_retained_content() {
    for feature in [
        winui_gallery::Feature::Flyout,
        winui_gallery::Feature::DropDownButton,
    ] {
        let mut f = Fixture::for_feature([1100, 850], feature);
        f.tap("Open here");
        let mut label_count = 0;
        for element in f.elements() {
            let app = f.cell.borrow();
            if let Some(object) = element.render_object(&app)
                && let Some(paragraph) = object.downcast::<inset_rendering::RenderParagraph>(&app)
                && paragraph
                    .text(&app)
                    .to_plain_text(true, true)
                    .starts_with("This content")
            {
                assert!(
                    object.as_box().unwrap().size(&app).width() > 200.0,
                    "plain flyout text must receive the control's default font"
                );
                label_count += 1;
            }
        }
        assert!(label_count > 0);
        f.tap("Count: 0");
        f.find("Count: 1");
        f.capture(if feature == winui_gallery::Feature::Flyout {
            "gallery_flyout"
        } else {
            "gallery_dropdown"
        });
    }
}

#[test]
fn standard_outside_dismissal_closes_only_the_topmost_flyout() {
    let (mut f, probe) = fixture(FlyoutShowMode::Standard);
    let parent = probe.flyout.get().unwrap();
    let child = {
        let mut app = f.cell.borrow_mut();
        let child = Flyout::new(&mut app, Text::new("Nested content"));
        parent.content(
            &mut app,
            FlyoutTarget::new(Builder::new(move |_, context| {
                Button::text(
                    "Open child",
                    Listener::new(move |app| child.show_at(app, context)),
                )
                .into_widget()
            })),
        );
        child
    };
    f.tap("Open flyout");
    f.tap("Open child");
    f.tap("Outside action");
    assert!(!child.is_open(&f.cell.borrow()));
    assert!(parent.is_open(&f.cell.borrow()));
    assert_eq!(probe.outside.get(), 0);
    f.tap("Outside action");
    assert!(!parent.is_open(&f.cell.borrow()));
}

#[test]
fn popup_shadow_leaves_transparent_content_clear_and_follows_theme() {
    let mut samples = Vec::new();
    for theme in [Theme::Light, Theme::Dark] {
        let (mut f, probe) = themed_fixture(FlyoutShowMode::Standard, theme);
        let flyout = probe.flyout.get().unwrap();
        flyout.flyout_presenter_style(
            &mut f.cell.borrow_mut(),
            Rc::new(|_, _, _| {
                SizedBox::new()
                    .width(180.0)
                    .height(80.0)
                    .child(Center::new().child(Text::new("Shadow probe")))
                    .into_widget()
            }),
        );
        f.tap("Open flyout");
        let center = f.find("Shadow probe");
        let pixels = f.view.pixels.borrow();
        let pixel = |x: f64, y: f64| {
            let index = (y as usize * f.view.size[0] as usize + x as usize) * 4;
            pixels[index]
        };
        // The transparent popup must not receive its own shadow beneath the text.
        assert_eq!(pixel(center.dx(), center.dy() + 20.0), 255);
        let outside = pixel(center.dx(), center.dy() + 45.0);
        assert!(outside < 255, "the shadow must extend below the popup");
        samples.push(outside);
        drop(pixels);
        f.capture(if theme == Theme::Light {
            "flyout_shadow_light"
        } else {
            "flyout_shadow_dark"
        });
    }
    assert!(
        samples[1] < samples[0],
        "dark theme uses the stronger source shadow"
    );
}
