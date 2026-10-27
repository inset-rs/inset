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

fn hold(fixture: &mut Fixture, millis: u64) {
    for _ in 0..millis / 20 {
        fixture.cell.elapse(std::time::Duration::from_millis(20));
    }
    fixture.pump();
}

#[test]
fn keyboard_focus_space_repeats_and_enter_clicks_once() {
    let mut fixture = Fixture::for_feature([900, 2000], winui_gallery::Feature::RepeatButton);
    fixture.tap("Hold me");
    fixture.focus("Hold me");
    fixture.find("RepeatButton: 1 clicks");
    send(
        &mut fixture,
        LogicalKeyboardKey::SPACE,
        PhysicalKeyboardKey::SPACE,
        0,
    );
    fixture.find("RepeatButton: 2 clicks");
    send(
        &mut fixture,
        LogicalKeyboardKey::SPACE,
        PhysicalKeyboardKey::SPACE,
        1,
    );
    fixture.find("RepeatButton: 2 clicks");
    hold(&mut fixture, 700);
    fixture.find("RepeatButton: 9 clicks");
    send(
        &mut fixture,
        LogicalKeyboardKey::SPACE,
        PhysicalKeyboardKey::SPACE,
        2,
    );
    hold(&mut fixture, 600);
    fixture.find("RepeatButton: 9 clicks");
    send(
        &mut fixture,
        LogicalKeyboardKey::ENTER,
        PhysicalKeyboardKey::ENTER,
        0,
    );
    send(
        &mut fixture,
        LogicalKeyboardKey::ENTER,
        PhysicalKeyboardKey::ENTER,
        1,
    );
    hold(&mut fixture, 700);
    fixture.find("RepeatButton: 10 clicks");
    send(
        &mut fixture,
        LogicalKeyboardKey::ENTER,
        PhysicalKeyboardKey::ENTER,
        2,
    );
    send(
        &mut fixture,
        LogicalKeyboardKey::GAME_BUTTON_A,
        PhysicalKeyboardKey::GAME_BUTTON_A,
        0,
    );
    send(
        &mut fixture,
        LogicalKeyboardKey::GAME_BUTTON_A,
        PhysicalKeyboardKey::GAME_BUTTON_A,
        1,
    );
    hold(&mut fixture, 700);
    fixture.find("RepeatButton: 11 clicks");
    send(
        &mut fixture,
        LogicalKeyboardKey::GAME_BUTTON_A,
        PhysicalKeyboardKey::GAME_BUTTON_A,
        2,
    );
    fixture.find("RepeatButton: 11 clicks");
}

#[test]
fn focus_loss_and_another_key_cancel_keyboard_repetition() {
    let mut fixture = Fixture::for_feature([900, 2000], winui_gallery::Feature::RepeatButton);
    fixture.tap("Hold me");
    fixture.focus("Hold me");
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
    hold(&mut fixture, 700);
    fixture.find("RepeatButton: 2 clicks");
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
    send(
        &mut fixture,
        LogicalKeyboardKey::SPACE,
        PhysicalKeyboardKey::SPACE,
        0,
    );
    fixture.focus("Dark theme");
    hold(&mut fixture, 700);
    fixture.find("RepeatButton: 3 clicks");
    send(
        &mut fixture,
        LogicalKeyboardKey::SPACE,
        PhysicalKeyboardKey::SPACE,
        2,
    );
    fixture.find("RepeatButton: 3 clicks");
}
