//! `ToggleSwitch` dragged: the knob follows the pointer and the switch toggles when it is released past half the track, and does not when it returns.
#![feature(arbitrary_self_types)]
mod common;
use common::Fixture;
use common::GalleryFixtureExt;
use inset_embedder::{Offset, PointerChange, PointerData, PointerDataPacket, PointerDeviceKind};
use inset_gestures::GestureBinding;
use inset_rendering::RenderParagraph;
use inset_winui::{
    AccentPalette, CONTENT_GAP, KNOB_AREA, KNOB_TRANSLATION_RANGE, TRACK_HEIGHT, TRACK_WIDTH,
    Theme, ThemeResources,
};
use std::time::Duration;

/// A label's left edge, centre line and height.
fn label_bounds(fixture: &Fixture, text: &str) -> (Offset, f64) {
    let elements = fixture.elements();
    let app = fixture.cell.borrow();
    for element in elements {
        if let Some(object) = element.render_object(&app)
            && let Some(paragraph) = object.downcast::<RenderParagraph>(&app)
            && paragraph.text(&app).to_plain_text(true, true) == text
        {
            let object = object.as_box().unwrap();
            let size = object.size(&app);
            let left = object.local_to_global(&app, Offset::new(0.0, size.height() / 2.0), None);
            return (left, size.height());
        }
    }
    panic!("missing label {text}")
}

/// The knob's centre on a track whose left end is `track_left` and whose centre line is `y`: at the end the state names.
fn knob_at(track_left: f64, y: f64, is_on: bool) -> Offset {
    let translation = if is_on { KNOB_TRANSLATION_RANGE } else { 0.0 };
    Offset::new(track_left + translation + KNOB_AREA / 2.0, y)
}

/// The knob of the switch whose content label is `content`: the track ends `CONTENT_GAP` before the label, on its centre line (`VerticalContentAlignment` is `Center`).
fn knob_center(fixture: &Fixture, content: &str, is_on: bool) -> Offset {
    let (label, _) = label_bounds(fixture, content);
    knob_at(label.dx() - CONTENT_GAP - TRACK_WIDTH, label.dy(), is_on)
}

/// The knob of the switch whose header is `header`: the track starts under the header's left edge, `ToggleSwitchTopHeaderMargin` and `ToggleSwitchPreContentMargin` below it.
fn knob_under_header(r: &ThemeResources, fixture: &Fixture, header: &str, is_on: bool) -> Offset {
    let (label, height) = label_bounds(fixture, header);
    let y = label.dy()
        + height / 2.0
        + r.toggle_switch_top_header_margin[3]
        + r.toggle_switch_pre_content_margin
        + TRACK_HEIGHT / 2.0;
    knob_at(label.dx(), y, is_on)
}

/// A pointer move `by` from `from`, with the delta the platform reports and the drag recogniser measures.
fn send_move(fixture: &mut Fixture, from: Offset, by: f64) -> Offset {
    let to = Offset::new(from.dx() + by, from.dy());
    fixture.at += Duration::from_millis(20);
    let mut app = fixture.cell.borrow_mut();
    GestureBinding::instance(&mut app).handle_pointer_data_packet(
        &mut app,
        PointerDataPacket::new(vec![PointerData {
            change: PointerChange::Move,
            kind: PointerDeviceKind::Touch,
            time_stamp: fixture.at,
            pointer_identifier: fixture.touch_pointer,
            physical_x: to.dx(),
            physical_y: to.dy(),
            physical_delta_x: by,
            ..Default::default()
        }]),
    );
    drop(app);
    fixture.cell.checkpoint();
    to
}

/// A touch drag: down at `from`, one move per horizontal `step`, then up where it ended.
fn drag(fixture: &mut Fixture, from: Offset, steps: &[f64]) {
    fixture.send(PointerChange::Down, from);
    let mut at = from;
    for &step in steps {
        at = send_move(fixture, at, step);
    }
    fixture.at += Duration::from_millis(20);
    fixture.send(PointerChange::Up, at);
    fixture.pump();
}

#[test]
fn toggle_switch_drags_toggle_past_half_the_track() {
    let r = ThemeResources::new(Theme::Light, AccentPalette::default());
    let mut fixture = Fixture::for_feature([900, 2000], winui_gallery::Feature::ToggleSwitch);
    fixture.ensure_visible("Airplane mode off");
    fixture.find("Clicked 0 times · wifi true · airplane false");

    // Airplane mode is off. Dragged 30 pt right (past the touch slop, then on), the knob is
    // released past half the 20 pt range: the switch toggles on.
    let knob = knob_center(&fixture, "Airplane mode off", false);
    drag(&mut fixture, knob, &[8.0, 8.0, 8.0, 6.0]);
    fixture.find("Clicked 0 times · wifi true · airplane true");
    fixture.find("Airplane mode on");

    // A short drag: past the slop towards off, then back to 16 pt from the off end. Released
    // above half, the knob returns and nothing toggles.
    let knob = knob_center(&fixture, "Airplane mode on", true);
    drag(&mut fixture, knob, &[-20.0, -2.0, 18.0]);
    fixture.find("Clicked 0 times · wifi true · airplane true");
    fixture.find("Airplane mode on");

    // A tap on the thumb still toggles (`Tapped`, with no drag holding the pointer).
    fixture.tap("Airplane mode on");
    fixture.find("Clicked 0 times · wifi true · airplane false");

    // Wi-Fi is on; the knob dragged 30 pt left, held mid-drag for a frame, then released
    // below half: it toggles off.
    let knob = knob_under_header(&r, &fixture, "Wi-Fi", true);
    fixture.send(PointerChange::Down, knob);
    let mut at = knob;
    for step in [-8.0, -8.0, -8.0] {
        at = send_move(&mut fixture, at, step);
    }
    fixture.pump();
    fixture.capture("toggle_switch_dragging");
    at = send_move(&mut fixture, at, -6.0);
    fixture.at += Duration::from_millis(20);
    fixture.send(PointerChange::Up, at);
    fixture.pump();
    fixture.find("Clicked 0 times · wifi false · airplane false");
    fixture.capture("toggle_switch_light");

    fixture.tap("Dark theme");
    fixture.find("Light theme");
    fixture.capture("toggle_switch_dark");
}
