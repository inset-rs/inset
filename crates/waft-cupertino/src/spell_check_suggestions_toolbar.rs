//! Flutter counterpart: `cupertino/spell_check_suggestions_toolbar.dart`.

use waft_embedder::{TextAffinity, TextRange, TextSelection};
use waft_foundation::{App, Handle, Listener};
use waft_scheduler::{FrameCallback, SchedulerBinding};
use waft_services::{SelectionChangedCause, TextInputClient, TextSelectionDelegate};
use waft_widgets::{
    BuildContext, ContextMenuButtonItem, EditableTextState, IntoWidget, KeyRef, SizedBox, State,
    StatelessWidget, TextSelectionToolbarAnchors, WidgetRef,
};

use crate::debug::debug_check_has_cupertino_localizations;
use crate::localizations::CupertinoLocalizations;
use crate::text_selection_toolbar::CupertinoTextSelectionToolbar;
use crate::text_selection_toolbar_button::CupertinoTextSelectionToolbarButton;

/// iOS only shows 3 spell check suggestions in the toolbar.
const K_MAX_SUGGESTIONS: usize = 3;

/// The default spell check suggestions toolbar for iOS.
///
/// Tries to position itself below the [`anchors`](Self::anchors), but if it doesn't fit, then
/// it readjusts to fit above bottom view insets.
///
/// See also:
///  * `SpellCheckSuggestionsToolbar`, which is similar but for both the Material and Cupertino
///    libraries.
#[derive(Debug)]
pub struct CupertinoSpellCheckSuggestionsToolbar {
    pub key: Option<KeyRef>,
    /// The location on which to anchor the menu.
    pub anchors: TextSelectionToolbarAnchors,
    /// The [`ContextMenuButtonItem`]s that will be turned into the correct button widgets and
    /// displayed in the spell check suggestions toolbar.
    ///
    /// Must not contain more than three items.
    ///
    /// See also:
    ///
    ///  * `AdaptiveTextSelectionToolbar.buttonItems`, the list of [`ContextMenuButtonItem`]s
    ///    that are used to build the buttons of the text selection toolbar.
    ///  * `SpellCheckSuggestionsToolbar.buttonItems`, the list of [`ContextMenuButtonItem`]s
    ///    used to build the Material style spell check suggestions toolbar.
    pub button_items: Vec<ContextMenuButtonItem>,
}

impl CupertinoSpellCheckSuggestionsToolbar {
    /// Constructs a [`CupertinoSpellCheckSuggestionsToolbar`].
    ///
    /// `button_items` must not contain more than three items.
    pub fn new(
        anchors: TextSelectionToolbarAnchors,
        button_items: Vec<ContextMenuButtonItem>,
    ) -> CupertinoSpellCheckSuggestionsToolbar {
        debug_assert!(button_items.len() <= K_MAX_SUGGESTIONS);
        CupertinoSpellCheckSuggestionsToolbar {
            key: None,
            anchors,
            button_items,
        }
    }

    /// Constructs a [`CupertinoSpellCheckSuggestionsToolbar`] with the default children for an
    /// `EditableText`.
    ///
    /// See also:
    ///  * `SpellCheckSuggestionsToolbar.editableText`, which is similar but builds an
    ///    Android-style toolbar.
    pub fn editable_text(
        app: &mut App,
        editable_text_state: Handle<EditableTextState>,
    ) -> CupertinoSpellCheckSuggestionsToolbar {
        CupertinoSpellCheckSuggestionsToolbar {
            key: None,
            button_items: Self::build_button_items(app, editable_text_state).unwrap_or_default(),
            anchors: editable_text_state.context_menu_anchors(app),
        }
    }

    /// Dart `CupertinoSpellCheckSuggestionsToolbar(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoSpellCheckSuggestionsToolbar {
        self.key = Some(key);
        self
    }

    /// Builds the button items for the toolbar based on the available spell check
    /// suggestions.
    pub fn build_button_items(
        app: &mut App,
        editable_text_state: Handle<EditableTextState>,
    ) -> Option<Vec<ContextMenuButtonItem>> {
        // Determine if composing region is misspelled.
        let span_at_cursor_index = editable_text_state.find_suggestion_span_at_cursor_index(
            app,
            TextInputClient::current_text_editing_value(editable_text_state, app)
                .expect("an EditableTextState always has a value")
                .selection
                .base_offset,
        )?;

        if span_at_cursor_index.suggestions.is_empty() {
            let context = editable_text_state.context(app);
            debug_assert!(debug_check_has_cupertino_localizations(app, context));
            let localizations = <dyn CupertinoLocalizations>::of(app, context);
            return Some(vec![
                ContextMenuButtonItem::new(None)
                    .label(localizations.no_spell_check_replacements_label()),
            ]);
        }

        let mut button_items = Vec::new();

        // Build suggestion buttons.
        let range = span_at_cursor_index.range;
        for suggestion in span_at_cursor_index
            .suggestions
            .into_iter()
            .take(K_MAX_SUGGESTIONS)
        {
            let label = suggestion.clone();
            button_items.push(
                ContextMenuButtonItem::new(Some(Listener::new(move |app| {
                    if !editable_text_state.mounted(app) {
                        return;
                    }
                    Self::replace_text(app, editable_text_state, &suggestion, range);
                })))
                .label(label),
            );
        }
        Some(button_items)
    }

    fn replace_text(
        app: &mut App,
        editable_text_state: Handle<EditableTextState>,
        text: &str,
        replacement_range: TextRange,
    ) {
        // Replacement cannot be performed if the text is read only or obscured.
        debug_assert!({
            let widget = editable_text_state.widget(app);
            !widget.read_only && !widget.obscure_text
        });

        let new_value = editable_text_state
            .text_editing_value(app)
            .replaced(replacement_range, text)
            .copy_with()
            .selection(TextSelection::collapsed(
                replacement_range.start + text.encode_utf16().count() as i32,
                TextAffinity::Downstream,
            ));
        editable_text_state.user_update_text_editing_value(
            app,
            new_value,
            SelectionChangedCause::Toolbar,
        );

        // Schedule a call to bringIntoView() after renderEditable updates.
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _duration| {
                if editable_text_state.mounted(app) {
                    let extent = editable_text_state
                        .text_editing_value(app)
                        .selection
                        .extent();
                    editable_text_state.bring_into_view(app, extent);
                }
            }),
            None,
        );
        editable_text_state.hide_toolbar(app, true);
    }

    /// Builds the toolbar buttons based on the [`button_items`](Self::button_items).
    fn build_toolbar_buttons(&self, _app: &mut App, _context: BuildContext) -> Vec<WidgetRef> {
        self.button_items
            .iter()
            .map(|button_item| {
                CupertinoTextSelectionToolbarButton::button_item(button_item.clone()).into_widget()
            })
            .collect()
    }
}

impl StatelessWidget for CupertinoSpellCheckSuggestionsToolbar {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        if self.button_items.is_empty() {
            return SizedBox::shrink().into_widget();
        }

        let children = self.build_toolbar_buttons(app, context);
        CupertinoTextSelectionToolbar::new(
            self.anchors.primary_anchor,
            match self.anchors.secondary_anchor {
                None => self.anchors.primary_anchor,
                Some(secondary_anchor) => secondary_anchor,
            },
            children,
        )
        .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::{Offset, Size};
    use waft_widgets::{Center, GlobalKey};

    use waft_widgets::testing::test_widgets;

    use super::*;
    use crate::app::CupertinoApp;

    fn button_items(suggestions: &[&str]) -> Vec<ContextMenuButtonItem> {
        suggestions
            .iter()
            .map(|suggestion| {
                ContextMenuButtonItem::new(Some(Listener::new(|_app| {}))).label(*suggestion)
            })
            .collect()
    }

    /// Dart's `more than three suggestions throws an error`, its first half: three build.
    #[test]
    fn three_suggestions_build() {
        test_widgets(|tester| {
            tester.pump_widget(
                CupertinoApp::new()
                    .home(
                        Center::new().child(CupertinoSpellCheckSuggestionsToolbar::new(
                            TextSelectionToolbarAnchors::new(Offset::ZERO),
                            button_items(&["hello", "yellow", "yell"]),
                        )),
                    )
                    .into_widget(),
            );
        });
    }

    /// Dart's `more than three suggestions throws an error`, its second half.
    #[test]
    #[should_panic(expected = "button_items.len() <= K_MAX_SUGGESTIONS")]
    fn more_than_three_suggestions_panic() {
        let _ = CupertinoSpellCheckSuggestionsToolbar::new(
            TextSelectionToolbarAnchors::new(Offset::ZERO),
            button_items(&["hello", "yellow", "yell", "yeller"]),
        );
    }

    #[test]
    fn the_toolbar_does_not_panic_at_zero_area() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            tester.pump_widget(
                CupertinoApp::new()
                    .home(
                        Center::new().child(
                            SizedBox::shrink().child(
                                CupertinoSpellCheckSuggestionsToolbar::new(
                                    TextSelectionToolbarAnchors::new(Offset::new(1.0, 1.0)),
                                    Vec::new(),
                                )
                                .key(key.clone().into()),
                            ),
                        ),
                    )
                    .into_widget(),
            );
            assert_eq!(tester.get_size(&key), Size::ZERO);
        });
    }
}
