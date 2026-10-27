//! `RadioButton`: activation checks an unchecked button and only then raises `Checked`; a checked or disabled one ignores the tap; both themes are captured.
#![feature(arbitrary_self_types)]
mod common;
use common::Fixture;
use common::GalleryFixtureExt;
use inset_embedder::Size;
use inset_widgets::*;
use inset_winui::*;

#[test]
fn radio_button_checks_once_per_group_and_switches_theme() {
    let mut fixture = Fixture::for_feature([900, 2000], winui_gallery::Feature::RadioButton);
    fixture.find("RadioButton: Option 2");
    fixture.find("Checked 0 times");
    fixture.tap("Option 1");
    fixture.find("RadioButton: Option 1");
    fixture.find("Checked 1 times");
    fixture.tap("Option 1");
    fixture.find("RadioButton: Option 1");
    fixture.find("Checked 1 times");
    fixture.tap("Option 3");
    fixture.find("RadioButton: Option 3");
    fixture.find("Checked 2 times");
    fixture.tap("Disabled");
    fixture.find("RadioButton: Option 3");
    fixture.find("Checked 2 times");
    fixture.capture("radio_button_light");
    fixture.tap("Dark theme");
    fixture.find("Light theme");
    fixture.capture("radio_button_dark");
}

/// Like WinUI's style (`HorizontalAlignment` Left, `VerticalAlignment` Center), a button given
/// more room than it needs keeps its `MinWidth` and the height of its glyph area.
#[test]
fn a_button_given_spare_room_keeps_its_own_size() {
    let size = common::size_under_center([300, 300], |_| {
        RadioButton::new(false, |_| {})
            .content(Text::new("Option"))
            .into_widget()
    });
    assert_eq!(size, Size::new(RADIO_BUTTON_MIN_WIDTH, GLYPH_AREA_HEIGHT));
}
