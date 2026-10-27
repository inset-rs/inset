//! `CheckBox`: toggling by tap, the three-state cycle, a disabled box, the check drawing on, and both themes.
#![feature(arbitrary_self_types)]
mod common;
use common::Fixture;
use common::GalleryFixtureExt;
use inset_embedder::{Color, PointerChange, Size};
use inset_foundation::{App, Handle};
use inset_painting::EdgeInsetsGeometry;
use inset_widgets::*;
use inset_winui::*;
use std::time::Duration;

/// Taps `text` and then runs only `frames` 20 ms frames, to catch an animation in flight.
fn tap_and_pump(fixture: &mut Fixture, text: &str, frames: u32) {
    let p = fixture.find(text);
    fixture.send(PointerChange::Down, p);
    fixture.at += Duration::from_millis(20);
    fixture.send(PointerChange::Up, p);
    for _ in 0..frames {
        fixture.frame_after(Duration::from_millis(20));
    }
}

#[test]
fn check_box_toggles_cycles_and_renders_in_both_themes() {
    let mut fixture = Fixture::for_feature([900, 2000], winui_gallery::Feature::CheckBox);
    fixture.find("CheckBox: unchecked · checked · indeterminate");
    fixture.capture("check_box_light");

    // `OnToggle`: unchecked → checked, checked → unchecked.
    fixture.tap("Two-state option");
    fixture.find("CheckBox: checked · checked · indeterminate");
    fixture.tap("Checked option");
    fixture.find("CheckBox: checked · unchecked · indeterminate");

    // `IsThreeState`: indeterminate → unchecked → checked → indeterminate.
    fixture.tap("Three-state option");
    fixture.find("CheckBox: checked · unchecked · unchecked");
    fixture.tap("Three-state option");
    fixture.find("CheckBox: checked · unchecked · checked");
    fixture.tap("Three-state option");
    fixture.find("CheckBox: checked · unchecked · indeterminate");

    // A disabled box ignores the tap.
    fixture.tap("Disabled option");
    fixture.find("CheckBox: checked · unchecked · indeterminate");

    // 100 ms into the 317 ms `NormalOffToNormalOn` segment: `TrimEnd` is part way along the check.
    tap_and_pump(&mut fixture, "Checked option", 5);
    fixture.find("CheckBox: checked · checked · indeterminate");
    fixture.capture("check_box_drawing");
    fixture.pump();

    fixture.tap("Dark theme");
    fixture.find("Light theme");
    fixture.capture("check_box_dark");
}

/// One `CheckBox` owning its `IsChecked`, mounted alone so the glyph animation can be caught in flight.
#[derive(Debug)]
struct Lone;
struct LoneState {
    state: StateData<Lone>,
    is_checked: Option<bool>,
}
impl StatefulWidget for Lone {
    type State = LoneState;
    fn create_state(&self) -> LoneState {
        LoneState {
            state: StateData::new(),
            is_checked: Some(false),
        }
    }
}
impl State for LoneState {
    type Widget = Lone;
    inset_widgets::state_accessors!();
    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let is_checked = app.get(self).is_checked;
        let page = ColoredBox::new(Color::from_argb(255, 255, 255, 255)).child(
            Padding::new(EdgeInsetsGeometry::all(36.0)).child(
                CheckBox::new(is_checked, move |app, value| {
                    self.set_state(app, |s| s.is_checked = value)
                })
                .content(Text::new("Lone option")),
            ),
        );
        ThemeScope::new(Theme::Light, page).into_widget()
    }
}

#[test]
fn check_box_draws_the_check_on() {
    let mut fixture = Fixture::with_root([200, 100], |app| {
        winui_gallery::install_fonts(app);
        run_app(
            app,
            WidgetsApp::new(AccentPalette::default().base)
                .builder(|_, _, _| Lone.into_widget())
                .into_widget(),
        );
    });
    fixture.capture("check_box_lone_off");
    // 100 ms into the 317 ms `NormalOffToNormalOn` segment: `TrimEnd` is part way along the check.
    tap_and_pump(&mut fixture, "Lone option", 5);
    fixture.capture("check_box_lone_drawing");
    fixture.pump();
    fixture.capture("check_box_lone_on");
    // 40 ms into the 68 ms `NormalOnToNormalOff` segment: `TrimStart` has eaten part of the check.
    tap_and_pump(&mut fixture, "Lone option", 2);
    fixture.capture("check_box_lone_erasing");
}

/// Like WinUI's style (`HorizontalAlignment` Left, `VerticalAlignment` Center), a box given more
/// room than it needs keeps its `MinWidth` and `MinHeight`.
#[test]
fn a_box_given_spare_room_keeps_its_own_size() {
    let r = ThemeResources::new(Theme::Light, AccentPalette::default());
    let size = common::size_under_center([300, 300], |_| {
        CheckBox::new(Some(true), |_, _| {})
            .content(Text::new("Check"))
            .into_widget()
    });
    assert_eq!(size, Size::new(r.check_box_min_width, r.check_box_height));
}
