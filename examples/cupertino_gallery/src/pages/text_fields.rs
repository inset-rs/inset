//! `CupertinoTextField`: the rounded field, a placeholder, a disabled field, and a password.

use reveal_cupertino::{CupertinoTextField, OverlayVisibilityMode};
use reveal_foundation::App;
use reveal_widgets::{BuildContext, WidgetRef};

use crate::support::{screen, scrolling_body, section_with_footer, showcase_leading};

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    screen(scrolling_body(vec![
        section_with_footer(
            "Type here",
            "The first field autofocuses so the caret is already there.",
            vec![showcase_leading(
                CupertinoTextField::new()
                    .placeholder("Name")
                    .autofocus(true)
                    .clear_button_mode(OverlayVisibilityMode::Editing),
            )],
        ),
        section_with_footer(
            "Password",
            "obscure_text replaces each glyph with a bullet.",
            vec![showcase_leading(
                CupertinoTextField::new()
                    .placeholder("Password")
                    .obscure_text(true),
            )],
        ),
        section_with_footer(
            "Borderless",
            "CupertinoTextField::borderless has no rounded box.",
            vec![showcase_leading(
                CupertinoTextField::borderless().placeholder("Search"),
            )],
        ),
        section_with_footer(
            "Disabled",
            "enabled(false) greys the default decoration and ignores taps.",
            vec![showcase_leading(
                CupertinoTextField::new()
                    .placeholder("Cannot edit")
                    .enabled(false),
            )],
        ),
    ]))
}
