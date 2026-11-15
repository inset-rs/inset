//! The field as UIKit's keyboards edit it, each edit's value as the framework would hear it:
//! Flutter's `FlutterTextInputPluginTest.mm` cases for typing, marked text, deleting and the return
//! key. UIKit's objects need the main thread, which the test harness does not give, so this is a
//! program of its own: `xcrun simctl spawn <device> <test binary>`.

#[cfg(target_os = "ios")]
fn main() {
    use objc2::MainThreadMarker;
    use objc2_foundation::NSRange;
    use waft_embedder::{
        TextAffinity, TextEditingValue, TextInputAction, TextInputConfiguration, TextRange,
        TextSelection,
    };
    use waft_embedder_ios::test_support::{Field, Heard, heard};

    fn value(text: &str, base: i32, extent: i32, composing: TextRange) -> Heard {
        Heard::Edited(
            TextEditingValue::new()
                .text(text)
                .selection(TextSelection::new(base, extent).affinity(TextAffinity::Downstream))
                .composing(composing),
        )
    }

    let main_thread =
        MainThreadMarker::new().expect("a test program's main runs on the main thread");

    // Typing goes in at the caret, and the framework hears each key.
    let field = Field::new(main_thread);
    field.insert_text("h");
    field.insert_text("i");
    assert_eq!(
        heard(),
        [
            value("h", 1, 1, TextRange::EMPTY),
            value("hi", 2, 2, TextRange::EMPTY)
        ]
    );

    // Marked text replaces what was marked, and typing commits it.
    let field = Field::new(main_thread);
    field.set_marked_text("ni", NSRange::new(2, 0));
    field.set_marked_text("你", NSRange::new(1, 0));
    field.insert_text("你");
    assert_eq!(
        heard(),
        [
            value("ni", 2, 2, TextRange::new(0, 2)),
            value("你", 1, 1, TextRange::new(0, 1)),
            value("你", 1, 1, TextRange::EMPTY),
        ]
    );

    // Deleting takes the unit before the caret, and the whole of an emoji.
    let field = Field::new(main_thread);
    field.set_state(
        &TextEditingValue::new()
            .text("a👍")
            .selection(TextSelection::collapsed(3, TextAffinity::Downstream)),
    );
    field.delete_backward();
    field.delete_backward();
    assert_eq!(
        heard(),
        [
            value("a", 1, 1, TextRange::EMPTY),
            value("", 0, 0, TextRange::EMPTY)
        ]
    );

    // The framework's own value is not heard back.
    let field = Field::new(main_thread);
    field.set_state(
        &TextEditingValue::new()
            .text("hello")
            .selection(TextSelection::new(1, 4)),
    );
    assert_eq!(heard(), []);
    assert_eq!(field.selected_text(), "ell");

    // Return in a single-line field is its action alone.
    let field = Field::new(main_thread);
    field.configure(
        &TextInputConfiguration::builder()
            .input_action(TextInputAction::Send)
            .build(),
    );
    assert!(!field.takes_new_line(), "no new line goes in");
    assert_eq!(heard(), [Heard::Action(TextInputAction::Send)]);

    println!("a field hands the framework each edit: ok");
}

#[cfg(not(target_os = "ios"))]
fn main() {}
