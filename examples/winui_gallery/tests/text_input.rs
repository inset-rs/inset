//! Source text-control policies around the native editor, including focus and IME updates.
#![feature(arbitrary_self_types)]
mod common;
use common::GalleryFixtureExt;

use common::Fixture;
use inset_embedder::{Offset, PointerChange, TextAffinity, TextRange, TextSelection};
use inset_foundation::Handle;
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey as Key,
    PhysicalKeyboardKey as Physical, TextEditingValue, TextInput,
};
use inset_widgets::*;
use inset_winui::*;

/// Finds a public control's native controller by its placeholder.
fn controller(f: &Fixture, placeholder: &str) -> AnyTextEditingController {
    let elements = f.elements();
    let app = f.cell.borrow();
    for element in elements {
        let widget = element.widget(&app);
        if let Some(text) = downcast_widget::<TextBox>(widget.as_ref())
            && text.placeholder_text == placeholder
        {
            return text.controller;
        }

        if let Some(password) = downcast_widget::<PasswordBox>(widget.as_ref())
            && password.placeholder_text == placeholder
        {
            return password.controller;
        }
    }
    panic!("missing control {placeholder}")
}

/// Finds the mounted editor and a point inside its native render object.
fn editor(f: &Fixture, controller: AnyTextEditingController) -> (Handle<FocusNode>, Offset, bool) {
    let elements = f.elements();
    let app = f.cell.borrow();
    for element in elements {
        if let Some(editable) = downcast_widget::<EditableText>(element.widget(&app).as_ref())
            && editable.controller == controller
        {
            let render = element.find_render_object(&app).unwrap().as_box().unwrap();
            return (
                editable.focus_node,
                render.local_to_global(&app, Offset::new(8.0, 8.0), None),
                editable.obscure_text,
            );
        }
    }
    panic!("missing editor")
}

/// Sends a mouse click through native gesture recognition.
fn click(f: &mut Fixture, point: Offset) {
    f.send_mouse(PointerChange::Hover, point, 0);
    f.send_mouse(PointerChange::Down, point, 1);
    f.send_mouse(PointerChange::Up, point, 0);
    f.pump();
}

/// Delivers the host's text/selection/composition value through TextInput.
fn input(f: &mut Fixture, text: &str, composing: TextRange) {
    let value = TextEditingValue::new()
        .text(text)
        .selection(TextSelection::collapsed(
            text.encode_utf16().count() as i32,
            TextAffinity::Downstream,
        ))
        .composing(composing);
    let mut app = f.cell.borrow_mut();
    TextInput::instance(&mut app).update_editing_value(&mut app, value);
    drop(app);
    f.cell.checkpoint();
    f.pump();
}

/// A hardware down or up, including modifiers used by source reveal gestures.
fn key(f: &mut Fixture, down: bool, key: Key, physical: Physical) {
    let event = if down {
        KeyEvent::Down(KeyDownEvent::new(physical, key, f.at))
    } else {
        KeyEvent::Up(KeyUpEvent::new(physical, key, f.at))
    };
    let mut app = f.cell.borrow_mut();
    HardwareKeyboard::instance(&mut app).handle_key_event(&mut app, &event);
    drop(app);
    f.cell.checkpoint();
    f.pump();
}

/// Locates a helper's icon, excluding the gallery navigation's symbols.
fn helper(f: &Fixture, symbol: FluentSymbol) -> Option<Offset> {
    let elements = f.elements();
    let app = f.cell.borrow();
    elements.into_iter().find_map(|element| {
        let widget = element.widget(&app);
        let icon = downcast_widget::<FontIcon>(widget.as_ref())?;
        if icon.glyph != symbol.glyph().to_string() {
            return None;
        }

        let render = element.find_render_object(&app)?.as_box()?;
        let size = render.size(&app);
        Some(render.local_to_global(
            &app,
            Offset::new(size.width() / 2.0, size.height() / 2.0),
            None,
        ))
    })
}

#[test]
fn text_box_mouse_focus_clear_and_native_edit_notifications() {
    let mut f = Fixture::for_feature([1080, 1100], winui_gallery::Feature::TextBox);
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    let c = controller(&f, "Enter your name");
    let (focus, point, _) = editor(&f, c);
    click(&mut f, point);
    assert!(focus.has_focus(&mut f.cell.borrow_mut()));
    input(&mut f, "Hello", TextRange::EMPTY);
    f.find("User edits: 1");
    let clear = helper(&f, FluentSymbol::Dismiss).expect("focused nonempty TextBox shows clear");
    click(&mut f, clear);
    assert_eq!(c.text_value(&f.cell.borrow()), "");
    assert!(focus.has_focus(&mut f.cell.borrow_mut()));
    f.find("User edits: 2");
    c.set_text(&mut f.cell.borrow_mut(), "From controller");
    f.cell.checkpoint();
    f.pump();
    f.find("User edits: 2");
    f.capture("text_box_light");
    f.tap("Dark theme");
    f.capture("text_box_dark");
}

#[test]
fn text_box_composition_multiline_and_read_only_use_native_editing() {
    let mut f = Fixture::for_feature([1080, 1100], winui_gallery::Feature::TextBox);
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    let c = controller(&f, "Write a few lines…");
    let (focus, point, _) = editor(&f, c);
    click(&mut f, point);
    assert!(focus.has_focus(&mut f.cell.borrow_mut()));
    input(&mut f, "你好", TextRange::new(0, 2));
    assert_eq!(c.value(&f.cell.borrow()).composing, TextRange::new(0, 2));
    input(&mut f, "你好\nsecond line", TextRange::EMPTY);
    assert_eq!(c.text_value(&f.cell.borrow()), "你好\nsecond line");
    assert!(helper(&f, FluentSymbol::Dismiss).is_none());
    let name = controller(&f, "Enter your name");
    f.tap("Read-only");
    let (_, point, _) = editor(&f, name);
    click(&mut f, point);
    input(&mut f, "Rejected", TextRange::EMPTY);
    assert_eq!(name.text_value(&f.cell.borrow()), "");
}

#[test]
fn password_peek_modes_reset_and_length_limit() {
    let mut f = Fixture::for_feature([1080, 1100], winui_gallery::Feature::PasswordBox);
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    let c = controller(&f, "Enter a password");
    let (focus, point, obscured) = editor(&f, c);
    assert!(obscured);
    click(&mut f, point);
    input(&mut f, "sample", TextRange::EMPTY);
    let eye = helper(&f, FluentSymbol::Eye).expect("new password enables Peek");
    f.send_mouse(PointerChange::Down, eye, 1);
    f.pump();
    assert!(!editor(&f, c).2);
    assert!(focus.has_focus(&mut f.cell.borrow_mut()));
    f.send_mouse(PointerChange::Up, eye, 0);
    f.pump();
    assert!(editor(&f, c).2);
    f.capture("password_box_light");
    f.tap("Reveal mode: Peek");
    assert!(helper(&f, FluentSymbol::Eye).is_none());
    f.tap("Reveal mode: Hidden");
    assert!(!editor(&f, c).2);
    f.tap("Reveal mode: Visible");
    assert!(editor(&f, c).2);
    // Returning to the field starts a new focus session; entry from empty enables Peek.
    click(&mut f, point);
    input(&mut f, "", TextRange::EMPTY);
    input(&mut f, "sample", TextRange::EMPTY);
    key(&mut f, true, Key::ALT_LEFT, Physical::ALT_LEFT);
    key(&mut f, true, Key::F8, Physical::F8);
    assert!(!editor(&f, c).2);
    key(&mut f, false, Key::F8, Physical::F8);
    key(&mut f, false, Key::ALT_LEFT, Physical::ALT_LEFT);
    assert!(editor(&f, c).2);
    let limited = controller(&f, "Up to 12 characters");
    let (_, point, _) = editor(&f, limited);
    click(&mut f, point);
    input(&mut f, "abcdefghijklmn", TextRange::EMPTY);
    assert_eq!(limited.text_value(&f.cell.borrow()), "abcdefghijkl");
    let (_, point, _) = editor(&f, c);
    click(&mut f, point);
    assert!(
        helper(&f, FluentSymbol::Eye).is_none(),
        "returning to existing password hides the eye"
    );
    f.tap("Dark theme");
    f.capture("password_box_dark");
}

/// Opens the native editing context menu through a secondary pointer gesture.
fn context_menu(f: &mut Fixture, point: Offset) {
    f.send_mouse(PointerChange::Hover, point, 0);
    f.send_mouse(PointerChange::Down, point, 2);
    f.send_mouse(PointerChange::Up, point, 0);
    f.pump();
}

#[test]
fn text_commands_copy_paste_undo_and_password_copy_policy() {
    let mut f = Fixture::for_feature([1080, 1100], winui_gallery::Feature::TextBox);
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    let c = controller(&f, "Enter your name");
    let (focus, point, _) = editor(&f, c);
    click(&mut f, point);
    input(&mut f, "copy this", TextRange::EMPTY);
    c.set_selection(&mut f.cell.borrow_mut(), TextSelection::new(0, 4));
    f.cell.checkpoint();
    f.pump();
    context_menu(&mut f, point);
    f.find("Cut");
    f.find("Copy");
    f.find("Select all");
    f.capture("text_box_context_menu_light");
    let copy = f.find("Copy");
    click(&mut f, copy);
    assert_eq!(f.clipboard_text().as_deref(), Some("copy"));
    assert!(focus.has_focus(&mut f.cell.borrow_mut()));
    f.tap("Dark theme");
    context_menu(&mut f, point);
    f.capture("text_box_context_menu_dark");
    f.find("Paste");
    let paste = f.find("Paste");
    click(&mut f, paste);
    assert!(focus.has_focus(&mut f.cell.borrow_mut()));
    // A committed edit is added to the native undo history after its throttle window.
    let before = c.text_value(&f.cell.borrow()).to_owned();
    f.cell.elapse(f.at + std::time::Duration::from_secs(1));
    input(&mut f, "changed", TextRange::EMPTY);
    f.cell.elapse(f.at + std::time::Duration::from_secs(2));
    context_menu(&mut f, point);
    let undo = f.find("Undo");
    click(&mut f, undo);
    assert_eq!(c.text_value(&f.cell.borrow()), before);
    // Visible passwords retain PasswordBox's prohibition on clipboard copying.
    f.navigate(winui_gallery::Feature::PasswordBox);
    let password = controller(&f, "Enter a password");
    let (_, point, _) = editor(&f, password);
    click(&mut f, point);
    input(&mut f, "sample", TextRange::EMPTY);
    f.tap("Reveal mode: Peek");
    f.tap("Reveal mode: Hidden");
    password.set_selection(&mut f.cell.borrow_mut(), TextSelection::new(0, 6));
    f.cell.checkpoint();
    f.pump();
    context_menu(&mut f, point);
    f.find("Paste");
    f.find("Select all");
    let widgets = f.elements();
    let app = f.cell.borrow();
    assert!(!widgets.into_iter().any(|element| {
        element
            .render_object(&app)
            .and_then(|render| render.downcast::<inset_rendering::RenderParagraph>(&app))
            .is_some_and(|paragraph| paragraph.text(&app).to_plain_text(true, true) == "Copy")
    }));
}

#[test]
fn disabling_a_focused_editor_and_hiding_a_revealed_page_clear_focus() {
    let mut f = Fixture::for_feature([1080, 1100], winui_gallery::Feature::PasswordBox);
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    let c = controller(&f, "Enter a password");
    let (focus, point, _) = editor(&f, c);
    click(&mut f, point);
    input(&mut f, "sample", TextRange::EMPTY);
    key(&mut f, true, Key::ALT_LEFT, Physical::ALT_LEFT);
    key(&mut f, true, Key::F8, Physical::F8);
    assert!(!editor(&f, c).2);
    f.tap("Enabled");
    assert!(editor(&f, c).2);
    assert!(!focus.has_focus(&mut f.cell.borrow_mut()));
    key(&mut f, false, Key::F8, Physical::F8);
    key(&mut f, false, Key::ALT_LEFT, Physical::ALT_LEFT);
    f.tap("Enabled");
    click(&mut f, point);
    assert!(focus.has_focus(&mut f.cell.borrow_mut()));
    f.navigate(winui_gallery::Feature::TextBox);
    assert!(!focus.has_focus(&mut f.cell.borrow_mut()));
    f.navigate(winui_gallery::Feature::PasswordBox);
    assert_eq!(c.text_value(&f.cell.borrow()), "sample");
    assert!(editor(&f, c).2);
}

#[test]
fn native_keyboard_selection_deletion_and_password_copy_policy() {
    let mut f = Fixture::for_feature([1080, 1100], winui_gallery::Feature::TextBox);
    f.send_mouse(PointerChange::Add, Offset::ZERO, 0);
    let c = controller(&f, "Enter your name");
    let (_, point, _) = editor(&f, c);
    click(&mut f, point);
    input(&mut f, "keyboard selection", TextRange::EMPTY);
    key(&mut f, true, Key::META_LEFT, Physical::META_LEFT);
    key(&mut f, true, Key::KEY_A, Physical::KEY_A);
    key(&mut f, false, Key::KEY_A, Physical::KEY_A);
    key(&mut f, false, Key::META_LEFT, Physical::META_LEFT);
    assert_eq!(c.selection(&f.cell.borrow()), TextSelection::new(0, 18));
    key(&mut f, true, Key::BACKSPACE, Physical::BACKSPACE);
    key(&mut f, false, Key::BACKSPACE, Physical::BACKSPACE);
    assert_eq!(c.text_value(&f.cell.borrow()), "");
    f.navigate(winui_gallery::Feature::PasswordBox);
    let c = controller(&f, "Enter a password");
    let (_, point, _) = editor(&f, c);
    click(&mut f, point);
    input(&mut f, "sample", TextRange::EMPTY);
    f.tap("Reveal mode: Peek");
    f.tap("Reveal mode: Hidden");
    click(&mut f, point);
    c.set_selection(&mut f.cell.borrow_mut(), TextSelection::new(0, 6));
    f.cell.checkpoint();
    for (logical, physical) in [(Key::KEY_C, Physical::KEY_C), (Key::KEY_X, Physical::KEY_X)] {
        key(&mut f, true, Key::META_LEFT, Physical::META_LEFT);
        key(&mut f, true, logical, physical);
        key(&mut f, false, logical, physical);
        key(&mut f, false, Key::META_LEFT, Physical::META_LEFT);
        assert!(f.clipboard_text().is_none());
        assert_eq!(c.text_value(&f.cell.borrow()), "sample");
    }
}

/// Like WinUI's `DeleteButtonStyle` (`GlyphElement` is `VerticalAlignment` Center), a focused box
/// showing its delete button, given more height than it needs, keeps the height it needs.
#[test]
fn a_box_showing_its_delete_button_given_spare_height_keeps_its_own() {
    let r = ThemeResources::new(Theme::Light, AccentPalette::default());
    let size = common::size_under_center([300, 300], |app| {
        let controller = TextEditingController::new(app).text(app, "Text");
        TextBox::new(controller.as_controller())
            .autofocus(true)
            .into_widget()
    });
    assert_eq!(size.height(), r.text_control_theme_min_height);
}
