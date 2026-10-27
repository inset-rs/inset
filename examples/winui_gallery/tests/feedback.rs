//! GPU checks for feedback controls, retained content and adaptive notification layout.
#![feature(arbitrary_self_types)]
mod common;
use common::GalleryFixtureExt;

use common::Fixture;
use inset_widgets::downcast_widget;
use inset_winui::{InfoBar, ProgressBar};
use winui_gallery::Feature;

#[test]
fn progress_states_and_themes_render() {
    let mut fixture = Fixture::for_feature([1080, 900], Feature::ProgressBar);
    fixture.capture("progress_bar_light");
    fixture.tap("Paused");
    fixture.pump();
    fixture.pump();
    fixture.capture("progress_bar_paused");
    fixture.tap("Error");
    fixture.pump();
    fixture.capture("progress_bar_error");

    let elements = fixture.elements();
    let app = fixture.cell.borrow();
    let bars: Vec<_> = elements
        .into_iter()
        .filter_map(|element| {
            let widget = element.widget(&app);
            downcast_widget::<ProgressBar>(&**widget).cloned()
        })
        .collect();
    assert_eq!(
        bars.iter()
            .filter(|bar| bar.show_error && bar.show_paused)
            .count(),
        2
    );
    drop(app);

    fixture.tap("Dark theme");
    fixture.capture("progress_bar_dark");
}

#[test]
fn expander_retains_content_and_reports_state_changes() {
    let mut fixture = Fixture::for_feature([1080, 1050], Feature::Expander);
    fixture.tap("Download details");
    fixture.pump();
    fixture.find("Expanding event");
    fixture.tap("Retry download");
    fixture.find("Retries: 1");
    fixture.capture("expander_light");
    fixture.tap("Download details");
    fixture.pump();
    fixture.find("Collapsed event");
    fixture.tap("Download details");
    fixture.pump();
    fixture.find("Retries: 1");
    fixture.tap("Storage details");
    fixture.pump();
    fixture.tap("Storage details");
    fixture.pump();
    fixture.tap("Dark theme");
    fixture.capture("expander_dark");
}

#[test]
fn info_bar_programmatic_close_can_be_canceled() {
    let mut fixture = Fixture::for_feature([1080, 1200], Feature::InfoBar);
    fixture.capture("info_bar_light");
    fixture.tap("Cancel closing");
    fixture.tap("Close notification");
    fixture.find("Opened");
    fixture.find("Download ready");
    fixture.tap("Cancel closing");
    fixture.tap("Close notification");
    fixture.find("Closed: Programmatic");
    let elements = fixture.elements();
    let app = fixture.cell.borrow();
    assert!(elements.into_iter().any(|element| {
        let widget = element.widget(&app);
        downcast_widget::<InfoBar>(&**widget)
            .is_some_and(|bar| bar.title == "Download ready" && !bar.is_open)
    }));
    drop(app);

    fixture.tap("Open notification");
    fixture.find("Download ready");
    fixture.tap("Dark theme");
    fixture.capture("info_bar_dark");
}

#[test]
fn info_bar_narrow_layout_renders() {
    let mut fixture = Fixture::for_feature([420, 1200], Feature::InfoBar);
    fixture.capture("info_bar_narrow");
    fixture.ensure_visible("Resume sync");
    fixture.capture("info_bar_action_narrow");
    assert!(
        fixture
            .find("Review your connection before continuing.")
            .dy()
            > fixture.find("Sync paused").dy()
    );
    fixture.tap("Resume sync");
    fixture.ensure_visible("Resume requested");
    fixture.find("Resume requested");
}

use inset_foundation::{App, Handle, Listener};
use inset_painting::Alignment;
use inset_rendering::MainAxisSize;
use inset_widgets::*;
use inset_winui::{AccentPalette, ExpandDirection, Expander};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

/// A fixture owner lets tests change properties between exact animation frames.
#[derive(Debug)]
struct LifecyclePage {
    /// Exposes the owner's state to the test.
    probe: Rc<Cell<Option<Handle<LifecycleState>>>>,

    /// Records events in delivery order.
    events: Rc<RefCell<Vec<String>>>,

    /// Selects the Expander example when present, otherwise the InfoBar.
    direction: Option<ExpandDirection>,
}

/// Controlled values supplied to the control under test.
struct LifecycleState {
    /// Native state identity.
    state: StateData<LifecyclePage>,

    /// Expanded or open value, depending on the example.
    open: bool,

    /// Whether Closing cancels the InfoBar close.
    cancel: bool,
}

impl StatefulWidget for LifecyclePage {
    type State = LifecycleState;

    fn create_state(&self) -> Self::State {
        LifecycleState {
            state: StateData::new(),
            open: true,
            cancel: false,
        }
    }
}

impl State for LifecycleState {
    type Widget = LifecyclePage;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        self.widget(app).probe.set(Some(self));
        let open = app.get(self).open;
        let events = self.widget(app).events.clone();
        let control = if let Some(direction) = self.widget(app).direction {
            Expander::new(open, move |app, expanded| {
                self.set_state(app, |state| state.open = expanded);
            })
            .expand_direction(direction)
            .header(Text::new("Header"))
            .content(
                SizedBox::new()
                    .height(80.0)
                    .child(Text::new("Retained content")),
            )
            .collapsed(Listener::new(move |_| {
                events.borrow_mut().push("Collapsed".to_owned())
            }))
            .into_widget()
        } else {
            let clicks = events.clone();
            let closing = events.clone();
            let closed = events.clone();
            InfoBar::new(open, move |app, open| {
                self.set_state(app, |state| state.open = open);
            })
            .title("Test notification")
            .close_button_style(Rc::new(|_, _, _, _| Text::new("Close").into_widget()))
            .close_button_click(Listener::new(move |_| {
                clicks.borrow_mut().push("Click".to_owned())
            }))
            .closing(move |app, args| {
                args.cancel = app.get(self).cancel;
                closing
                    .borrow_mut()
                    .push(format!("Closing {:?}", args.reason));
            })
            .closed(move |_, args| {
                closed
                    .borrow_mut()
                    .push(format!("Closed {:?}", args.reason))
            })
            .opened(Listener::new(move |_| {
                events.borrow_mut().push("Opened".to_owned())
            }))
            .into_widget()
        };

        Align::new()
            .alignment(Alignment::TOP_LEFT.into())
            .child(
                SizedBox::new().width(320.0).child(
                    Column::new()
                        .main_axis_size(MainAxisSize::Min)
                        .children([control, Text::new("After control").into_widget()]),
                ),
            )
            .into_widget()
    }
}

/// Pumps one frame without concealing the intermediate animation state.
fn frame(fixture: &mut Fixture, milliseconds: u64) {
    fixture.frame_after(Duration::from_millis(milliseconds));
}

type LifecycleFixture = (Fixture, Handle<LifecycleState>, Rc<RefCell<Vec<String>>>);

/// Mounts a source lifecycle example inside the native app and overlay.
fn lifecycle_fixture(direction: Option<ExpandDirection>) -> LifecycleFixture {
    let probe = Rc::new(Cell::new(None));
    let owner = probe.clone();
    let events = Rc::new(RefCell::new(Vec::new()));
    let recorded = events.clone();
    let mut fixture = Fixture::with_root([400, 400], move |app| {
        winui_gallery::install_fonts(app);
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| {
                LifecyclePage {
                    probe: owner.clone(),
                    events: recorded.clone(),
                    direction,
                }
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
    fixture.pump();
    (fixture, probe.get().unwrap(), events)
}

#[test]
fn collapse_reserves_content_height_until_the_source_visibility_key() {
    for (direction, hide_at) in [(ExpandDirection::Down, 167), (ExpandDirection::Up, 200)] {
        let (mut fixture, owner, events) = lifecycle_fixture(Some(direction));
        let expanded_y = fixture.find("After control").dy();
        owner.set_state(&mut fixture.cell.borrow_mut(), |state| state.open = false);
        frame(&mut fixture, 1);
        assert_eq!(&*events.borrow(), &["Collapsed"]);
        assert_eq!(fixture.find("After control").dy(), expanded_y);
        frame(&mut fixture, 1);
        frame(&mut fixture, hide_at - 2);
        assert_eq!(fixture.find("After control").dy(), expanded_y);
        frame(&mut fixture, 1);
        assert!(fixture.find("After control").dy() < expanded_y - 80.0);

        owner.set_state(&mut fixture.cell.borrow_mut(), |state| state.open = true);
        frame(&mut fixture, 1);
        assert_eq!(
            fixture.find("After control").dy(),
            expanded_y,
            "expanding reserves full height immediately"
        );
    }
}

#[test]
fn close_button_event_order_and_cancellation_match_infobar() {
    let (mut fixture, owner, events) = lifecycle_fixture(None);
    owner.set_state(&mut fixture.cell.borrow_mut(), |state| state.cancel = true);
    fixture.pump();
    fixture.tap("Close");
    assert_eq!(
        &*events.borrow(),
        &["Click", "Closing CloseButton", "Opened"]
    );
    assert!(fixture.cell.borrow().get(owner).open);

    events.borrow_mut().clear();
    owner.set_state(&mut fixture.cell.borrow_mut(), |state| state.cancel = false);
    fixture.pump();
    fixture.tap("Close");
    assert_eq!(
        &*events.borrow(),
        &["Click", "Closing CloseButton", "Closed CloseButton"]
    );
    assert!(!fixture.cell.borrow().get(owner).open);
}

#[test]
fn determinate_progress_clamps_to_the_range_and_handles_a_zero_span() {
    for (minimum, maximum, value, expected_width) in [
        (0.0, 100.0, -10.0, 0),
        (20.0, 80.0, 50.0, 100),
        (0.0, 100.0, 200.0, 200),
        (20.0, 20.0, 20.0, 0),
    ] {
        let fixture = Fixture::with_root([240, 80], move |app| {
            run_app(
                app,
                WidgetsApp::new(AccentPalette::default().base)
                    .debug_show_checked_mode_banner(false)
                    .builder(move |_, _, _| {
                        Align::new()
                            .alignment(Alignment::TOP_LEFT.into())
                            .child(
                                SizedBox::new().width(200.0).child(
                                    ProgressBar::new()
                                        .minimum(minimum)
                                        .maximum(maximum)
                                        .value(value)
                                        .corner_radius(0.0)
                                        .foreground(inset_embedder::Color::from_argb(
                                            255, 255, 0, 0,
                                        ))
                                        .background(inset_embedder::Color::from_argb(
                                            255, 0, 0, 255,
                                        )),
                                ),
                            )
                            .into_widget()
                    })
                    .into_widget(),
            );
        });
        let pixels = fixture.view.pixels.borrow();
        let red_pixels = pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|pixel| **pixel == [255, 0, 0, 255])
            .count();
        // The default indicator grid is three pixels high.
        assert_eq!(red_pixels, expected_width * 3);
    }
}

#[test]
fn info_bar_status_glyphs_keep_their_natural_line_height() {
    let fixture = Fixture::for_feature([1080, 1200], Feature::InfoBar);
    let all = fixture.elements();
    let mut app = fixture.cell.borrow_mut();
    let mut elements: Vec<_> = all
        .into_iter()
        .filter(|element| downcast_widget::<InfoBar>(element.widget(&app).as_ref()).is_some())
        .collect();
    let mut index = 0;
    while index < elements.len() {
        elements.extend(elements[index].children(&app));
        index += 1;
    }
    let mut checked = 0;
    for element in elements {
        let Some(object) = element.render_object(&app) else {
            continue;
        };
        let Some(paragraph) = object.downcast::<inset_rendering::RenderParagraph>(&app) else {
            continue;
        };
        let text = paragraph.text(&app).to_plain_text(true, true);
        if ![
            inset_winui::FluentSymbol::Info,
            inset_winui::FluentSymbol::CheckmarkCircle,
            inset_winui::FluentSymbol::ErrorCircle,
            inset_winui::FluentSymbol::DismissCircle,
        ]
        .iter()
        .any(|symbol| text == symbol.glyph().to_string())
        {
            continue;
        }
        let render_box = object.as_box().unwrap();
        let actual = render_box.size(&app);
        let natural = render_box.get_dry_layout(
            &mut app,
            inset_rendering::BoxConstraints::new().max_width(actual.width()),
        );
        assert!(
            actual.height() >= natural.height(),
            "status glyph line is constrained: {actual:?}, natural: {natural:?}"
        );
        checked += 1;
    }
    assert!(checked >= 4);
}

/// Like WinUI's style (`HorizontalAlignment` Left), an expander given more width than its header
/// needs keeps its own: a narrow header leaves it at the style's `MinWidth`.
#[test]
fn an_expander_given_spare_width_keeps_its_own() {
    let r = inset_winui::ThemeResources::new(
        inset_winui::Theme::Light,
        inset_winui::AccentPalette::default(),
    );
    let size = common::size_under_center([300, 300], |_| {
        Expander::new(false, |_, _| {})
            .header(SizedBox::new().width(10.0).height(10.0))
            .into_widget()
    });
    assert_eq!(
        size,
        inset_embedder::Size::new(r.flyout_theme_min_width, r.expander_min_height)
    );
}

/// An InfoBar's content measures at its own width, so content narrower than the message leaves a
/// bar given spare width as wide as it is without content.
#[test]
fn an_info_bar_given_spare_width_keeps_its_own() {
    let bar = || {
        InfoBar::new(true, |_, _| {})
            .title("Title")
            .message("Message")
    };
    let without_content = common::size_under_center([300, 300], move |_| bar().into_widget());
    let with_content = common::size_under_center([300, 300], move |_| {
        bar()
            .content(SizedBox::new().width(10.0).height(10.0))
            .into_widget()
    });
    assert_eq!(with_content.width(), without_content.width());
}
