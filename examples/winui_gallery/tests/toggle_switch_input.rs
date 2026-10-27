#![feature(arbitrary_self_types)]
mod common;
use common::Fixture;
use common::GalleryFixtureExt;
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyRepeatEvent, KeyUpEvent, LogicalKeyboardKey,
    PhysicalKeyboardKey,
};

fn send(fixture: &mut Fixture, key: LogicalKeyboardKey, physical: PhysicalKeyboardKey, kind: u8) {
    let event = match kind {
        0 => KeyEvent::Down(KeyDownEvent::new(physical, key, fixture.at)),
        1 => KeyEvent::Repeat(KeyRepeatEvent::new(physical, key, fixture.at)),
        _ => KeyEvent::Up(KeyUpEvent::new(physical, key, fixture.at)),
    };
    let mut app = fixture.cell.borrow_mut();
    HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &event);
    drop(app);
    fixture.cell.checkpoint();
    fixture.pump();
}

#[test]
fn keyboard_focused_switch_toggles_once_on_space_release() {
    let mut fixture = Fixture::for_feature([900, 2000], winui_gallery::Feature::ToggleSwitch);
    fixture.ensure_visible("Airplane mode off");
    fixture.tap("Airplane mode off");
    fixture.find("Clicked 0 times · wifi true · airplane true");
    fixture.focus("Airplane mode on");
    send(
        &mut fixture,
        LogicalKeyboardKey::SPACE,
        PhysicalKeyboardKey::SPACE,
        0,
    );
    send(
        &mut fixture,
        LogicalKeyboardKey::SPACE,
        PhysicalKeyboardKey::SPACE,
        1,
    );
    fixture.find("Clicked 0 times · wifi true · airplane true");
    send(
        &mut fixture,
        LogicalKeyboardKey::SPACE,
        PhysicalKeyboardKey::SPACE,
        2,
    );
    fixture.find("Clicked 0 times · wifi true · airplane false");
    for (key, physical) in [
        (LogicalKeyboardKey::ENTER, PhysicalKeyboardKey::ENTER),
        (
            LogicalKeyboardKey::ARROW_RIGHT,
            PhysicalKeyboardKey::ARROW_RIGHT,
        ),
    ] {
        send(&mut fixture, key, physical, 0);
        send(&mut fixture, key, physical, 2);
        fixture.find("Clicked 0 times · wifi true · airplane false");
    }
    // Arrow navigation may have moved focus to a different control.
    fixture.tap("Airplane mode off");
    fixture.focus("Airplane mode on");
    send(
        &mut fixture,
        LogicalKeyboardKey::GAME_BUTTON_A,
        PhysicalKeyboardKey::GAME_BUTTON_A,
        0,
    );
    fixture.find("Clicked 0 times · wifi true · airplane true");
    send(
        &mut fixture,
        LogicalKeyboardKey::GAME_BUTTON_A,
        PhysicalKeyboardKey::GAME_BUTTON_A,
        2,
    );
    fixture.find("Clicked 0 times · wifi true · airplane false");
}

#[test]
fn another_key_cancels_pending_space() {
    let mut fixture = Fixture::for_feature([900, 2000], winui_gallery::Feature::ToggleSwitch);
    fixture.ensure_visible("Airplane mode off");
    fixture.tap("Airplane mode off");
    fixture.focus("Airplane mode on");
    send(
        &mut fixture,
        LogicalKeyboardKey::SPACE,
        PhysicalKeyboardKey::SPACE,
        0,
    );
    send(
        &mut fixture,
        LogicalKeyboardKey::KEY_A,
        PhysicalKeyboardKey::KEY_A,
        0,
    );
    send(
        &mut fixture,
        LogicalKeyboardKey::KEY_A,
        PhysicalKeyboardKey::KEY_A,
        2,
    );
    send(
        &mut fixture,
        LogicalKeyboardKey::SPACE,
        PhysicalKeyboardKey::SPACE,
        2,
    );
    fixture.find("Clicked 0 times · wifi true · airplane true");
}
