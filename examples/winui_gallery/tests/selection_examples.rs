//! Gallery property choices update independent examples without changing their neighbors.
#![feature(arbitrary_self_types)]
mod common;
use common::GalleryFixtureExt;

use common::Fixture;
use winui_gallery::Feature;

#[test]
fn selection_examples_keep_independent_values() {
    let mut f = Fixture::for_feature([1080, 900], Feature::CheckBox);
    f.ensure_visible("Include optional files");
    f.tap("Include optional files");
    f.find("Configurable value: checked");
    f.find("CheckBox: unchecked · checked · indeterminate");
    f.capture("gallery_checkbox_properties");
    f.navigate(Feature::RadioButton);
    f.ensure_visible("Express delivery");
    f.tap("Express delivery");
    f.find("Delivery: Express");
    f.find("RadioButton: Option 2");
    f.capture("gallery_radio_properties");
    f.navigate(Feature::ToggleSwitch);
    f.ensure_visible("Notifications");
    f.tap("Notifications");
    f.find("Notification preference: On");
    f.find("Clicked 0 times · wifi true · airplane false");
    f.capture("gallery_switch_properties");
}

#[test]
fn narrow_slider_properties_and_presets_remain_accessible() {
    let mut f = Fixture::for_feature([640, 900], Feature::Slider);
    f.ensure_visible("Set gain to minimum");
    f.tap("Set gain to minimum");
    f.find("Gain: -50 dB");
    f.ensure_visible("Set gain to maximum");
    f.tap("Set gain to maximum");
    f.find("Gain: +50 dB");
    f.ensure_visible("Reset gain");
    f.tap("Reset gain");
    f.find("Gain: +0 dB");
    f.find("Slider: 42 · stepped 50 · vertical 30");
    f.capture("gallery_slider_properties_narrow");
}
