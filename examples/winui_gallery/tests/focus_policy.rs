//! Flutter click-focus behavior with the gallery's native route and window focus lifecycle.
#![feature(arbitrary_self_types)]
mod common;
use common::GalleryFixtureExt;

use common::Fixture;
use inset_embedder::{PointerChange, ViewFocusDirection, ViewFocusEvent, ViewFocusState};
use inset_rendering::RenderParagraph;
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey as Key,
    PhysicalKeyboardKey as Physical,
};
use inset_widgets::{
    AnyFocusNode, FocusableActionDetector, WidgetsBinding, downcast_widget, primary_focus,
};
use inset_winui::FocusVisual;
use winui_gallery::Feature;

/// Sends a complete hardware key press through the native shortcut handlers.
fn key(f: &mut Fixture, logical: Key, physical: Physical) {
    for event in [
        KeyEvent::Down(KeyDownEvent::new(physical, logical, f.at)),
        KeyEvent::Up(KeyUpEvent::new(physical, logical, f.at)),
    ] {
        let mut app = f.cell.borrow_mut();
        HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &event);
        drop(app);
        f.cell.checkpoint();
        f.pump();
    }
}

/// Finds the native detector owning a control's label.
fn control_focus(f: &Fixture, text: &str) -> AnyFocusNode {
    let elements = f.elements();
    let app = f.cell.borrow();
    for element in elements {
        if element
            .render_object(&app)
            .and_then(|r| r.downcast::<RenderParagraph>(&app))
            .is_some_and(|r| r.text(&app).to_plain_text(true, true) == text)
        {
            let mut ancestor = Some(element);
            while let Some(element) = ancestor {
                if let Some(detector) =
                    downcast_widget::<FocusableActionDetector>(element.widget(&app).as_ref())
                {
                    return detector.focus_node.expect("control owns a focus node");
                }
                ancestor = element.parent(&app);
            }
        }
    }
    panic!("no focusable control for {text}");
}

#[test]
fn mouse_leaves_focus_alone_and_tab_works_before_any_click() {
    let mut f = Fixture::new([1080, 780]);
    let initial =
        primary_focus(&mut f.cell.borrow_mut()).expect("native route scope has initial focus");
    key(&mut f, Key::TAB, Physical::TAB);
    assert_ne!(primary_focus(&mut f.cell.borrow_mut()), Some(initial));
    let elements = f.elements();
    assert!(elements.iter().any(|element| {
        let app = f.cell.borrow();
        downcast_widget::<FocusVisual>(element.widget(&app).as_ref())
            .is_some_and(|visual| visual.visible)
    }));
    let keyboard_focus = primary_focus(&mut f.cell.borrow_mut());
    let button = f.find("Standard");
    f.send_mouse(PointerChange::Add, button, 0);
    f.send_mouse(PointerChange::Down, button, 1);
    f.send_mouse(PointerChange::Up, button, 0);
    f.pump();
    assert_eq!(primary_focus(&mut f.cell.borrow_mut()), keyboard_focus);
    f.find("Clicked 1 times · wifi true · airplane false");
    let target = control_focus(&f, "Standard");
    // Tab passes every feature's navigation item, and a few other stops, before the page.
    for _ in 0..Feature::ALL.len() + 5 {
        if primary_focus(&mut f.cell.borrow_mut()) == Some(target) {
            break;
        }
        key(&mut f, Key::TAB, Physical::TAB);
    }
    assert_eq!(primary_focus(&mut f.cell.borrow_mut()), Some(target));
    f.capture("gallery_keyboard_focus");
    key(&mut f, Key::SPACE, Physical::SPACE);
    f.find("Clicked 2 times · wifi true · airplane false");

    for state in [ViewFocusState::Unfocused, ViewFocusState::Focused] {
        let mut app = f.cell.borrow_mut();
        WidgetsBinding::instance(&mut app).handle_view_focus_changed(
            &mut app,
            ViewFocusEvent {
                view_id: inset_embedder::ViewId(0),
                state,
                direction: ViewFocusDirection::Undefined,
            },
        );
        drop(app);
        f.cell.checkpoint();
        f.pump();
        if state == ViewFocusState::Unfocused {
            assert!(!target.has_focus(&mut f.cell.borrow_mut()));
        }
    }
    assert_eq!(primary_focus(&mut f.cell.borrow_mut()), Some(target));
}

#[test]
fn mouse_theme_action_has_no_focus_ring() {
    let mut f = Fixture::new([1080, 780]);
    let initial = primary_focus(&mut f.cell.borrow_mut());
    let action = f.find("Dark theme");
    f.send_mouse(PointerChange::Add, action, 0);
    f.send_mouse(PointerChange::Down, action, 1);
    f.send_mouse(PointerChange::Up, action, 0);
    f.pump();
    f.find("Light theme");
    assert_eq!(primary_focus(&mut f.cell.borrow_mut()), initial);
    let elements = f.elements();
    let app = f.cell.borrow();
    assert!(!elements.iter().any(|element| {
        downcast_widget::<FocusVisual>(element.widget(&app).as_ref())
            .is_some_and(|visual| visual.visible)
    }));
    drop(app);
    f.capture("gallery_mouse_theme_focus");
}

#[test]
fn pointer_interaction_does_not_focus_sliders_switches_or_repeat_buttons() {
    for feature in [
        winui_gallery::Feature::Slider,
        winui_gallery::Feature::ToggleSwitch,
        winui_gallery::Feature::RepeatButton,
    ] {
        let mut f = Fixture::for_feature([1080, 1000], feature);
        let before = primary_focus(&mut f.cell.borrow_mut());
        let elements = f.elements();
        let point = {
            let app = f.cell.borrow();
            let element = elements
                .into_iter()
                .find(|element| {
                    let widget = element.widget(&app);
                    match feature {
                        winui_gallery::Feature::Slider => {
                            downcast_widget::<inset_winui::Slider>(widget.as_ref()).is_some()
                        }
                        winui_gallery::Feature::ToggleSwitch => {
                            downcast_widget::<inset_winui::ToggleSwitch>(widget.as_ref()).is_some()
                        }
                        winui_gallery::Feature::RepeatButton => {
                            downcast_widget::<inset_winui::RepeatButton>(widget.as_ref()).is_some()
                        }
                        _ => unreachable!(),
                    }
                })
                .unwrap();
            let render = element.find_render_object(&app).unwrap().as_box().unwrap();
            let size = render.size(&app);
            render.local_to_global(
                &app,
                inset_embedder::Offset::new(size.width() / 2.0, size.height() / 2.0),
                None,
            )
        };
        f.send_mouse(PointerChange::Add, point, 0);
        f.send_mouse(PointerChange::Down, point, 1);
        f.send_mouse(PointerChange::Up, point, 0);
        f.pump();
        assert_eq!(
            primary_focus(&mut f.cell.borrow_mut()),
            before,
            "{feature:?}"
        );
    }
}
