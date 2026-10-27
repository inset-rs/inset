//! `ToggleButton`, `RepeatButton` and `HyperlinkButton` in the gallery: toggling in two and three states, a held repeat against the App clock, a hyperlink activation, and both themes.
#![feature(arbitrary_self_types)]
mod common;
use common::Fixture;
use common::GalleryFixtureExt;
use inset_embedder::{Offset, PointerChange};
use std::time::Duration;

/// Lets `duration` pass on the App clock in 20 ms steps (the timers fire from `AppCell::elapse`, not from frames), pumping frames between steps so what the timers changed is built and painted.
fn hold(fixture: &mut Fixture, duration: Duration) {
    let step = Duration::from_millis(20);
    let mut elapsed = Duration::ZERO;
    while elapsed < duration {
        fixture.cell.elapse(step);
        elapsed += step;
        fixture.pump();
    }
}

#[test]
fn button_family_toggles_repeats_navigates_and_switches_theme() {
    let mut fixture = Fixture::for_feature([900, 2000], winui_gallery::Feature::ToggleButton);
    fixture.find("ToggleButton: unchecked · three-state unchecked");

    // `OnToggleImpl`: unchecked → checked → unchecked in two-state mode.
    fixture.tap("Toggle me");
    fixture.find("ToggleButton: checked · three-state unchecked");
    fixture.tap("Toggle me");
    fixture.find("ToggleButton: unchecked · three-state unchecked");

    // With `IsThreeState`: unchecked → checked → indeterminate → unchecked.
    fixture.tap("Three-state");
    fixture.find("ToggleButton: unchecked · three-state checked");
    fixture.tap("Three-state");
    fixture.find("ToggleButton: unchecked · three-state indeterminate");
    fixture.tap("Three-state");
    fixture.find("ToggleButton: unchecked · three-state unchecked");

    // A disabled toggle button ignores the pointer.
    fixture.tap("Disabled checked");
    fixture.find("ToggleButton: unchecked · three-state unchecked");

    fixture.navigate(winui_gallery::Feature::RepeatButton);

    // `ClickMode_Press`: one click on the press, then one at `Delay` (500 ms) and one every `Interval` (33 ms) while held: 700 ms gives repeats at 500, 533, …, 698.
    let hold_me = fixture.find("Hold me");
    fixture.send(PointerChange::Down, hold_me);
    fixture.pump();
    fixture.find("RepeatButton: 1 clicks");
    hold(&mut fixture, Duration::from_millis(700));
    fixture.send(PointerChange::Up, hold_me);
    fixture.pump();
    fixture.find("RepeatButton: 8 clicks");

    // Released: no more clicks however long the clock runs.
    hold(&mut fixture, Duration::from_millis(500));
    fixture.find("RepeatButton: 8 clicks");

    // A disabled repeat button does nothing on the press.
    let disabled = fixture.find("Disabled repeat");
    fixture.send(PointerChange::Down, disabled);
    hold(&mut fixture, Duration::from_millis(600));
    fixture.send(PointerChange::Up, disabled);
    fixture.pump();
    fixture.find("RepeatButton: 8 clicks");

    fixture.navigate(winui_gallery::Feature::HyperlinkButton);
    fixture.tap("Learn more");
    fixture.find("HyperlinkButton: 1 clicks");
    fixture.tap("Disabled link");
    fixture.find("HyperlinkButton: 1 clicks");

    fixture.navigate(winui_gallery::Feature::ToggleButton);

    // Leave the two-state button checked so the captures show the `Checked` row.
    fixture.tap("Toggle me");
    fixture.find("ToggleButton: checked · three-state unchecked");

    fixture.capture("button_family_light");
    fixture.tap("Dark theme");
    fixture.find("Light theme");
    fixture.capture("button_family_dark");
}

/// The same hold with a mouse: the press starts the repeat, the hover before it does not.
#[test]
fn repeat_button_repeats_under_a_mouse() {
    let mut fixture = Fixture::for_feature([900, 2000], winui_gallery::Feature::RepeatButton);
    let hold_me = fixture.find("Hold me");
    fixture.send_mouse(PointerChange::Add, hold_me - Offset::new(50.0, 50.0), 0);
    fixture.send_mouse(PointerChange::Hover, hold_me, 0);
    fixture.pump();
    fixture.find("RepeatButton: 0 clicks");
    fixture.send_mouse(PointerChange::Down, hold_me, 1);
    fixture.pump();
    fixture.find("RepeatButton: 1 clicks");
    hold(&mut fixture, Duration::from_millis(700));
    fixture.send_mouse(PointerChange::Up, hold_me, 0);
    fixture.pump();
    fixture.find("RepeatButton: 8 clicks");
}

/// The width of a button showing a 16 px square with `padding`, or the style's own.
fn button_width(padding: Option<[f64; 4]>) -> f64 {
    use common::mount;
    use inset_foundation::{App, Listener};
    use inset_widgets::*;
    use inset_winui::*;
    use std::rc::Rc;

    let key = Rc::new(GlobalKey::new());
    let button_key = key.clone();
    let fixture = Fixture::with_root(
        [200, 200],
        mount(move |app: &mut App| {
            winui_gallery::install_fonts(app);
            let square = SizedBox::new().width(16.0).height(16.0);
            let mut button = Button::new(square, Listener::new(|_| {})).key(button_key);
            if let Some(padding) = padding {
                button = button.padding(padding);
            }
            WidgetsApp::new(AccentPalette::default().base)
                .debug_show_checked_mode_banner(false)
                .builder(move |_, _, _| Center::new().child(button.clone()).into_widget())
                .into_widget()
        }),
    );
    let mut app = fixture.cell.borrow_mut();
    let context = key.current_context(&mut app).unwrap();
    let object = context.find_render_object(&app).unwrap().as_box().unwrap();
    object.size(&app).width()
}

#[test]
fn a_buttons_padding_replaces_the_styles() {
    let wide = button_width(None);
    let narrow = button_width(Some([4.0; 4]));
    assert_eq!(wide - narrow, (11.0 + 11.0) - (4.0 + 4.0));
}
