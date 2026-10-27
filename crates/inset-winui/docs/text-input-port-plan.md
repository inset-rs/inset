# TextBox and PasswordBox source mapping

Status: TextBox, PasswordBox and their gallery pages are implemented with the approved native Flutter multiline wrapping and editing-event behavior. Verification against the completed Inset editor is complete: automated editing and IME tests passed, and the user confirmed keyboard input in the macOS gallery.

The resource generator now resolves references through the selected sibling Fluent dictionaries before consulting generic.xaml, so PasswordBox uses TextBox's actual brushes, including its elevation gradients, instead of the older fallback colors.

## Source ownership

| Concern | WinUI source | Inset integration |
| --- | --- | --- |
| Template geometry, common states, helper buttons | `controls/dev/CommonStyles/TextBox_themeresources.xaml` and `PasswordBox_themeresources.xaml` | Transcribe the named template parts using Grid, ControlBorder, text and native editing widgets; resolve resources through the generator. |
| TextBox clear button | `core/native/text/Controls/TextBox.cpp::CanInvokeDeleteButton` and `dxaml/lib/TextBox_Partial.cpp::OnDeleteButtonSizeChanged` | Preserve focus, empty/read-only checks, single-line/no-wrap eligibility, available-space checks and the source's square helper-button sizing. |
| Password reveal | `core/native/text/Controls/PasswordBox.cpp::RevealPassword`, `OnContentChanged`, `OnGotFocus`, `UpdateVisualState` and `CanInvokeRevealButton` | Preserve Peek/Hidden/Visible modes, press-to-peek, empty-to-nonempty eligibility, focus reset and concealment after editing; use EditableText's obscuring support. |
| Selection gestures and keyboard editing | Windows RichEdit and TextBoxBase | Use Inset's TextSelectionGestureDetectorBuilder and EditableText, with the editing-engine substitution recorded in [controls/PORTING.md](../src/controls/PORTING.md#text_controlrs--textboxbase). |
| Context and selection menus | `TextControlCommandBarContextFlyout`, `TextControlCommandBarSelectionFlyout` | The desktop TextCommandBarFlyout command order, availability and overflow-item template use EditableText’s native context-menu overlay and in-window acrylic. |

## Accepted editing-engine differences

### Multiline editing without wrapping

[WinUI TextBoxBase](../../microsoft-ui-xaml/dxaml/xcp/core/native/text/Controls/TextBoxBase.cpp) derives multiline mode from `AcceptsReturn || TextWrapping == Wrap`; the RichEdit word-wrap flag is set independently. Thus `AcceptsReturn=true, TextWrapping=NoWrap` accepts newlines and preserves long lines without wrapping.

[Inset RenderEditable](../../reveal-rs/crates/inset-rendering/src/editable.rs), following Flutter's `rendering/editable.dart::_adjustConstraints`, constrains multiline text to the viewport width and scrolls it vertically. Single-line text gets unlimited layout width and horizontal scrolling. There is no independent wrapping property or two-axis editor offset in this mechanism.

The approved integration uses native multiline wrapping and records the WinUI behavior difference. Preserving the unwrapped multiline case would need an editing design beyond a direct Flutter EditableText transcription.

### Cancelable edits and selection

[WinUI TextBox::UpdateTextProperty](../../microsoft-ui-xaml/dxaml/xcp/core/native/text/Controls/TextBox.cpp) raises BeforeTextChanging before updating the public Text property, skips that event during IME composition, and restores text/selection and clears RichEdit undo history when canceled. TextChanging and TextChanged are not raised for the canceled change. TextBox::OnSelectionChanged also supports canceling selection before updating the public selection properties.

[Inset EditableText::format_and_set_value](../../reveal-rs/crates/inset-widgets/src/widgets/editable_text.rs) runs formatters for text changes and composition commits, then updates its controller and sends change callbacks. Formatter callbacks cannot cover selection-only changes or direct TextEditingController assignments. A normal controller listener is notified after the value changes. Flutter's public UndoHistoryController exposes undo/redo, but not a clear-history operation.

The WinUI cancellation events described above are not exposed by this port. Applications use input formatters to validate user edits and controller listeners to observe assignments, following Inset’s Flutter-based editor.

## Validation

Input tests cover clear/reveal behavior, IME composition, native selection and undo, clipboard commands, read-only/disabled states and focus. Light/dark gallery captures verify the template layout. Valo’s coincident-gradient-endpoint regression checks that the focused accent stays on the bottom edge rather than coloring the entire border.

Initial batch checks: `cargo test --workspace --no-fail-fast` passed all 137 tests; workspace clippy completed with only the existing valo-harness manifest warning; resource-generator tests passed all 9 cases and regeneration matched the checked-in file. Valo passed 27 renderer unit tests, 53 golden tests and the new gradient-endpoint GPU regression. The native input check was initially deferred until the base editor was ready.

Follow-up verification on 2026-09-11: all six text-control integration tests and all five winit IME tests passed against the completed editor. The keyboard test now verifies deletion directly from hardware key events, matching winit’s `handles_text_editing_keys=false` setting; it no longer manually invokes a macOS deletion command. In the macOS gallery, automation verified typing and Backspace, and the user confirmed that keyboard input works, including ⌘A to select all text. Clippy reported only the existing valo-harness manifest warning.
