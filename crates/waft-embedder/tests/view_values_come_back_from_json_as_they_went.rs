//! Each value a host gives a view and takes from it comes back from JSON as it went, as an app
//! that sends them between processes needs.

use std::fmt::Debug;
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use waft_embedder::{
    Contact, KeyData, KeyEventDeviceType, KeyEventType, PointerDeviceKind, PointerReport,
    ReportedChange, ScreenId, SystemMouseCursorKind, ViewConstraints, ViewFocusDirection,
    ViewFocusEvent, ViewFocusState, ViewId, ViewMetrics, ViewPadding,
};

fn through_json<T: Serialize + DeserializeOwned>(value: &T) -> T {
    let json = serde_json::to_string(value).expect("every value is written");
    serde_json::from_str(&json).expect("what was written is read")
}

fn assert_comes_back<T: Serialize + DeserializeOwned + PartialEq + Debug>(value: T) {
    assert_eq!(through_json(&value), value);
}

fn contact() -> Contact {
    Contact {
        pressure: 0.5,
        pressure_min: 0.0,
        pressure_max: 1.0,
        distance: 0.25,
        distance_max: 2.0,
        size: 0.125,
        radius_major: 4.0,
        radius_minor: 3.0,
        radius_min: 1.0,
        radius_max: 8.0,
        orientation: 0.75,
        tilt: 0.5,
    }
}

#[test]
fn a_pointer_report_and_its_parts_come_back() {
    assert_comes_back(ViewId(3));
    assert_comes_back(PointerDeviceKind::InvertedStylus);
    assert_comes_back(ReportedChange::PanZoomUpdate {
        pan: [3.0, -4.0],
        scale: 1.5,
        rotation: 0.25,
    });
    assert_comes_back(contact());
    assert_comes_back(PointerReport {
        view_id: ViewId(3),
        time_stamp: Duration::from_nanos(1_234_567_891),
        device: 7,
        kind: PointerDeviceKind::Stylus,
        position: [10.5, 20.25],
        change: ReportedChange::Down { buttons: 2 },
        contact: contact(),
    });
}

#[test]
fn a_key_and_its_parts_come_back() {
    assert_comes_back(KeyEventType::Repeat);
    assert_comes_back(KeyEventDeviceType::Gamepad);
    assert_comes_back(KeyData {
        time_stamp: Duration::from_micros(42),
        event_type: KeyEventType::Down,
        device_type: KeyEventDeviceType::Keyboard,
        physical: 0x0007_0004,
        logical: 0x61,
        character: Some("a".to_owned()),
        synthesized: true,
    });
}

#[test]
fn a_focus_change_and_a_cursor_come_back() {
    assert_comes_back(ViewFocusState::Focused);
    assert_comes_back(ViewFocusDirection::Backward);
    assert_comes_back(SystemMouseCursorKind::ResizeUpLeftDownRight);

    let event = ViewFocusEvent {
        view_id: ViewId(5),
        state: ViewFocusState::Unfocused,
        direction: ViewFocusDirection::Forward,
    };
    let back = through_json(&event);
    assert_eq!(back.view_id, event.view_id);
    assert_eq!(back.state, event.state);
    assert_eq!(back.direction, event.direction);
}

#[test]
fn view_metrics_and_their_parts_come_back() {
    let padding = ViewPadding {
        left: 1.0,
        top: 2.0,
        right: 3.0,
        bottom: 4.0,
    };
    assert_comes_back(padding);
    assert_comes_back(ViewConstraints::new(0.0, 800.0, 0.0, 600.0));
    assert_comes_back(ScreenId(9));
    assert_comes_back(ViewMetrics {
        physical_size: [1600.0, 1200.0],
        physical_constraints: ViewConstraints::tight(1600.0, 1200.0),
        device_pixel_ratio: 2.0,
        padding,
        view_padding: padding,
        view_insets: ViewPadding::ZERO,
        screen: Some(ScreenId(9)),
    });
}
