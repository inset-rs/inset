//! Slider's source value converter, keyboard tooltip, and opt-out behavior.
#![feature(arbitrary_self_types)]
use inset_foundation::{App, Handle};
use inset_rendering::RenderParagraph;
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey, PhysicalKeyboardKey,
};
use inset_widgets::*;
use inset_winui::*;
use inset_winui_test_support::Fixture;
use std::{cell::Cell, rc::Rc};
#[derive(Debug)]
struct Page(Rc<Cell<Option<Handle<PageState>>>>);
struct PageState {
    state: StateData<Page>,
    value: f64,
    enabled: bool,
    custom: bool,
}
impl StatefulWidget for Page {
    type State = PageState;
    fn create_state(&self) -> Self::State {
        PageState {
            state: StateData::new(),
            value: 0.57,
            enabled: true,
            custom: false,
        }
    }
}
impl State for PageState {
    type Widget = Page;
    inset_widgets::state_accessors!();
    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        self.widget(app).0.set(Some(self));
        let mut slider = Slider::new(app.get(self).value, move |app, value| {
            self.set_state(app, |s| s.value = value)
        })
        .maximum(1.0)
        .step_frequency(0.1)
        .is_thumb_tool_tip_enabled(app.get(self).enabled);
        if app.get(self).custom {
            slider =
                slider.thumb_tool_tip_value_converter(|value| format!("{:.0}%", value * 100.0));
        }
        ThemeScope::new(
            Theme::Light,
            Stack::new().children([Positioned::new(SizedBox::new().width(300.0).child(slider))
                .left(50.0)
                .top(100.0)
                .into_widget()]),
        )
        .into_widget()
    }
}
fn has_text(f: &Fixture, label: &str) -> bool {
    let elements = f.elements();
    let app = f.cell.borrow();
    elements.into_iter().any(|e| {
        e.render_object(&app)
            .and_then(|r| r.downcast::<RenderParagraph>(&app))
            .is_some_and(|r| r.text(&app).to_plain_text(true, true) == label)
    })
}
fn key(f: &mut Fixture, logical: LogicalKeyboardKey, physical: PhysicalKeyboardKey) {
    for e in [
        KeyEvent::Down(KeyDownEvent::new(physical, logical, f.at)),
        KeyEvent::Up(KeyUpEvent::new(physical, logical, f.at)),
    ] {
        let mut app = f.cell.borrow_mut();
        HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &e);
        drop(app);
        f.cell.checkpoint();
    }
    for _ in 0..3 {
        f.pump();
    }
}
#[test]
fn keyboard_focus_formats_current_value_and_converter_and_disable_rebuild() {
    let slot = Rc::new(Cell::new(None));
    let state = slot.clone();
    let mut f = Fixture::with_root([420, 240], move |app| {
        winui_gallery::install_fonts(app);
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| Page(state.clone()).into_widget()),
            false,
            true,
            false,
        );
        run_app(
            app,
            WidgetsApp::new(AccentPalette::default().base)
                .debug_show_checked_mode_banner(false)
                .builder(move |_, _, _| {
                    FocusScope::new(Overlay::new().initial_entries([entry]))
                        .autofocus(true)
                        .into_widget()
                })
                .into_widget(),
        );
    });
    assert!(!has_text(&f, "0.6"));
    key(&mut f, LogicalKeyboardKey::TAB, PhysicalKeyboardKey::TAB);
    assert!(has_text(&f, "0.6"));
    let text = f.find("0.6");
    assert!(text.dy() < 100.0, "tooltip must be above thumb: {text:?}");
    slot.get()
        .unwrap()
        .set_state(&mut f.cell.borrow_mut(), |s| s.custom = true);
    for _ in 0..3 {
        f.pump();
    }
    assert!(has_text(&f, "57%"));
    f.capture("slider-tooltip-keyboard-converter");
    slot.get()
        .unwrap()
        .set_state(&mut f.cell.borrow_mut(), |s| s.enabled = false);
    for _ in 0..3 {
        f.pump();
    }
    f.cell.elapse(std::time::Duration::from_millis(200));
    f.pump();
    assert!(!has_text(&f, "57%"));
}
