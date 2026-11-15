//! Flutter counterpart: material_ui's `spell_check_suggestions_toolbar.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_cupertino::CupertinoTextSelectionToolbar;
use waft_embedder::{Offset, TextRange};
use waft_foundation::{App, Handle, Listener};
use waft_painting::{
    AlignmentGeometry, Border, BorderSide, BorderStyle, BoxDecoration, EdgeInsets,
    EdgeInsetsGeometry, TextStyle,
};
use waft_scheduler::{FrameCallback, SchedulerBinding};
use waft_services::{SelectionChangedCause, TextInputClient, TextSelectionDelegate};
use waft_widgets::{
    AnimatedSize, BuildContext, Column, ContextMenuButtonItem, ContextMenuButtonType,
    CrossAxisAlignment, CustomSingleChildLayout, DecoratedBox, EditableTextState, IntoWidget,
    KeyRef, MainAxisSize, MediaQuery, Padding, SizedBox, State, StatelessWidget, Text,
    TextSelectionToolbarAnchors, WidgetRef,
};

use crate::adaptive_text_selection_toolbar::AdaptiveTextSelectionToolbar;
use crate::colors::Colors;
use crate::material::{Material, MaterialType};
use crate::spell_check_suggestions_toolbar_layout_delegate::SpellCheckSuggestionsToolbarLayoutDelegate;
use crate::text_selection_toolbar_text_button::TextSelectionToolbarTextButton;

// The default height of the SpellCheckSuggestionsToolbar, which
// assumes there are the maximum number of spell check suggestions available, 3.
// Size eyeballed on Pixel 4 emulator running Android API 31.
const K_DEFAULT_TOOLBAR_HEIGHT: f64 = 193.0;

/// The maximum number of suggestions in the toolbar is 3, plus a delete button.
const K_MAX_SUGGESTIONS: usize = 3;

/// The default spell check suggestions toolbar for Android.
///
/// Tries to position itself below the [`anchor`](Self::anchor), but if it doesn't fit, then it
/// readjusts to fit above bottom view insets.
///
/// See also:
///
///  * `CupertinoSpellCheckSuggestionsToolbar`, which is similar but builds an
///    iOS-style spell check toolbar.
pub struct SpellCheckSuggestionsToolbar {
    pub key: Option<KeyRef>,
    /// The focal point below which the toolbar attempts to position itself.
    pub anchor: Offset,
    /// The [`ContextMenuButtonItem`]s that will be turned into the correct button
    /// widgets and displayed in the spell check suggestions toolbar.
    ///
    /// Must not contain more than four items, typically three suggestions and a
    /// delete button.
    ///
    /// See also:
    ///
    ///  * [`AdaptiveTextSelectionToolbar::button_items`], the list of
    ///    [`ContextMenuButtonItem`]s that are used to build the buttons of the
    ///    text selection toolbar.
    ///  * `CupertinoSpellCheckSuggestionsToolbar::button_items`, the list of
    ///    [`ContextMenuButtonItem`]s used to build the Cupertino style spell check
    ///    suggestions toolbar.
    pub button_items: Vec<ContextMenuButtonItem>,
}

impl Debug for SpellCheckSuggestionsToolbar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpellCheckSuggestionsToolbar")
            .field("anchor", &self.anchor)
            .field("button_items", &self.button_items)
            .finish_non_exhaustive()
    }
}

impl SpellCheckSuggestionsToolbar {
    /// Constructs a [`SpellCheckSuggestionsToolbar`].
    ///
    /// `button_items` must not contain more than four items, generally three
    /// suggestions and one delete button.
    pub fn new(
        anchor: Offset,
        button_items: Vec<ContextMenuButtonItem>,
    ) -> SpellCheckSuggestionsToolbar {
        debug_assert!(button_items.len() <= K_MAX_SUGGESTIONS + 1);
        SpellCheckSuggestionsToolbar {
            key: None,
            anchor,
            button_items,
        }
    }

    /// Dart `SpellCheckSuggestionsToolbar(key:)`.
    pub fn key(mut self, key: KeyRef) -> SpellCheckSuggestionsToolbar {
        self.key = Some(key);
        self
    }

    /// Constructs a [`SpellCheckSuggestionsToolbar`] with the default children for
    /// an `EditableText`.
    ///
    /// See also:
    ///  * `CupertinoSpellCheckSuggestionsToolbar::editable_text`, which is similar
    ///    but builds an iOS-style toolbar.
    pub fn editable_text(
        app: &mut App,
        editable_text_state: Handle<EditableTextState>,
    ) -> SpellCheckSuggestionsToolbar {
        SpellCheckSuggestionsToolbar {
            key: None,
            button_items: Self::build_button_items(app, editable_text_state).unwrap_or_default(),
            anchor: Self::get_toolbar_anchor(editable_text_state.context_menu_anchors(app)),
        }
    }

    /// Builds the button items for the toolbar based on the available
    /// spell check suggestions.
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

        // Build delete button.
        let delete_button = ContextMenuButtonItem::new(Some(Listener::new(move |app| {
            if !editable_text_state.mounted(app) {
                return;
            }
            let composing = TextInputClient::current_text_editing_value(editable_text_state, app)
                .expect("an EditableTextState always has a value")
                .composing;
            Self::replace_text(app, editable_text_state, "", composing);
        })))
        .r#type(ContextMenuButtonType::Delete);
        button_items.push(delete_button);

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
            .replaced(replacement_range, text);
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

    /// Determines the Offset that the toolbar will be anchored to.
    pub fn get_toolbar_anchor(anchors: TextSelectionToolbarAnchors) -> Offset {
        // Since this will be positioned below the anchor point, use the secondary
        // anchor by default.
        match anchors.secondary_anchor {
            None => anchors.primary_anchor,
            Some(secondary_anchor) => secondary_anchor,
        }
    }

    /// Builds the toolbar buttons based on the [`button_items`](Self::button_items).
    fn build_toolbar_buttons(&self, app: &mut App, context: BuildContext) -> Vec<WidgetRef> {
        self.button_items
            .iter()
            .map(|button_item| {
                let mut label = Text::new(AdaptiveTextSelectionToolbar::get_button_label(
                    app,
                    context,
                    button_item,
                ));
                if button_item.r#type == ContextMenuButtonType::Delete {
                    label = label.style(TextStyle::new().color(Colors::BLUE.primary()));
                }
                let mut button = TextSelectionToolbarTextButton::new(
                    label,
                    EdgeInsets::from_ltrb(20.0, 0.0, 0.0, 0.0),
                )
                .alignment(AlignmentGeometry::CENTER_LEFT);
                if let Some(on_pressed) = button_item.on_pressed.clone() {
                    button = button.on_pressed(on_pressed);
                }

                if button_item.r#type != ContextMenuButtonType::Delete {
                    return button.into_widget();
                }
                DecoratedBox::new(BoxDecoration::new().border(Border {
                    top: BorderSide::new(
                        Colors::GREY.primary(),
                        1.0,
                        BorderStyle::Solid,
                        BorderSide::STROKE_ALIGN_INSIDE,
                    ),
                    ..Border::default()
                }))
                .child(button)
                .into_widget()
            })
            .collect()
    }
}

impl StatelessWidget for SpellCheckSuggestionsToolbar {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        if self.button_items.is_empty() {
            return SizedBox::shrink().into_widget();
        }

        // Adjust toolbar height if needed.
        let spell_check_suggestions_toolbar_height =
            K_DEFAULT_TOOLBAR_HEIGHT - (48.0 * (4.0 - self.button_items.len() as f64));
        // Incorporate the padding distance between the content and toolbar.
        let media_query_data = MediaQuery::of(app, context);
        let soft_keyboard_view_insets_bottom = media_query_data.view_insets.bottom;
        let padding_above =
            media_query_data.padding.top + CupertinoTextSelectionToolbar::K_TOOLBAR_SCREEN_PADDING;
        // Makes up for the Padding.
        let local_adjustment = Offset::new(
            CupertinoTextSelectionToolbar::K_TOOLBAR_SCREEN_PADDING,
            padding_above,
        );

        let children = self.build_toolbar_buttons(app, context);
        Padding::new(EdgeInsetsGeometry::from_ltrb(
            CupertinoTextSelectionToolbar::K_TOOLBAR_SCREEN_PADDING,
            padding_above,
            CupertinoTextSelectionToolbar::K_TOOLBAR_SCREEN_PADDING,
            CupertinoTextSelectionToolbar::K_TOOLBAR_SCREEN_PADDING
                + soft_keyboard_view_insets_bottom,
        ))
        .child(
            CustomSingleChildLayout::new(Rc::new(SpellCheckSuggestionsToolbarLayoutDelegate::new(
                self.anchor - local_adjustment,
            )))
            .child(
                // This duration was eyeballed on a Pixel 2 emulator running Android
                // API 28 for the Material TextSelectionToolbar.
                AnimatedSize::new(Duration::from_millis(140)).child(
                    SpellCheckSuggestionsToolbarContainer {
                        height: spell_check_suggestions_toolbar_height,
                        children,
                    },
                ),
            ),
        )
        .into_widget()
    }
}

/// The Material-styled toolbar outline for the spell check suggestions
/// toolbar.
#[derive(Debug)]
struct SpellCheckSuggestionsToolbarContainer {
    height: f64,
    children: Vec<WidgetRef>,
}

impl StatelessWidget for SpellCheckSuggestionsToolbarContainer {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        Material::new()
            // This elevation was eyeballed on a Pixel 4 emulator running Android
            // API 31 for the SpellCheckSuggestionsToolbar.
            .elevation(2.0)
            .r#type(MaterialType::Card)
            .child(
                SizedBox::new()
                    // This width was eyeballed on a Pixel 4 emulator running Android
                    // API 31 for the SpellCheckSuggestionsToolbar.
                    .width(165.0)
                    .height(self.height)
                    .child(
                        Column::new()
                            .main_axis_size(MainAxisSize::Min)
                            .cross_axis_alignment(CrossAxisAlignment::Stretch)
                            .children(self.children.clone()),
                    ),
            )
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::{Size, TargetPlatform};
    use waft_widgets::Center;

    use super::*;
    use crate::scaffold::Scaffold;
    use crate::test_support::{WidgetTester, material_app, with_main_thread_stack};
    use crate::theme_data::ThemeData;

    // Vertical position at which to anchor the toolbar for testing.
    const K_ANCHOR: f64 = 200.0;
    // Amount for toolbar to overlap bottom padding for testing.
    const K_TEST_TOOLBAR_OVERLAP: f64 = 10.0;

    fn tester() -> WidgetTester {
        let tester = WidgetTester::on(TargetPlatform::Android);
        tester.install_platform_fonts();
        tester
    }

    /// Builds test button items for each of the suggestions provided.
    fn build_suggestion_buttons(suggestions: &[&str]) -> Vec<ContextMenuButtonItem> {
        let mut items: Vec<ContextMenuButtonItem> = suggestions
            .iter()
            .map(|suggestion| {
                ContextMenuButtonItem::new(Some(Listener::new(|_app| {}))).label(*suggestion)
            })
            .collect();
        items.push(
            ContextMenuButtonItem::new(Some(Listener::new(|_app| {})))
                .r#type(ContextMenuButtonType::Delete)
                .label("DELETE"),
        );
        items
    }

    /// `MaterialApp(home: Scaffold(body: SpellCheckSuggestionsToolbar(anchor:, buttonItems:)))`.
    fn pump_toolbar(tester: &mut WidgetTester, anchor: Offset, suggestions: &[&str]) {
        let root = {
            let app = tester.cell.borrow();
            material_app(
                ThemeData::new(&app),
                Scaffold::new()
                    .body(SpellCheckSuggestionsToolbar::new(
                        anchor,
                        build_suggestion_buttons(suggestions),
                    ))
                    .into_widget(),
            )
        };
        tester.pump_widget(root);
    }

    /// Dart's `tester.getTopLeft(findSpellCheckSuggestionsToolbar()).dy`.
    fn toolbar_y(tester: &WidgetTester) -> f64 {
        let containers = tester.find_by_type::<SpellCheckSuggestionsToolbarContainer>();
        assert_eq!(containers.len(), 1);
        tester.global_rect(tester.render_box_of(containers[0])).top
    }

    #[test]
    fn positions_toolbar_below_anchor_when_it_fits_above_bottom_view_padding() {
        with_main_thread_stack(|| {
            // We expect the toolbar to be positioned right below the anchor with padding
            // accounted for.
            let mut tester = tester();
            pump_toolbar(
                &mut tester,
                Offset::new(0.0, K_ANCHOR),
                &["hello", "yellow", "yell"],
            );
            assert_eq!(toolbar_y(&tester), K_ANCHOR);
        });
    }

    #[test]
    fn re_positions_toolbar_higher_below_anchor_when_it_does_not_fit_above_bottom_view_padding() {
        with_main_thread_stack(|| {
            // We expect the toolbar to be positioned _kTestToolbarOverlap pixels above the
            // anchor.
            let expected_toolbar_y = K_ANCHOR - K_TEST_TOOLBAR_OVERLAP;
            let mut tester = tester();
            pump_toolbar(
                &mut tester,
                Offset::new(0.0, K_ANCHOR - K_TEST_TOOLBAR_OVERLAP),
                &["hello", "yellow", "yell"],
            );
            assert_eq!(toolbar_y(&tester), expected_toolbar_y);
        });
    }

    /// Dart's `more than three suggestions throws an error`, its first half: three build.
    #[test]
    fn three_suggestions_build() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            pump_toolbar(
                &mut tester,
                Offset::new(0.0, K_ANCHOR - K_TEST_TOOLBAR_OVERLAP),
                &["hello", "yellow", "yell"],
            );
        });
    }

    /// Dart's `more than three suggestions throws an error`, its second half.
    #[test]
    #[should_panic(expected = "button_items.len() <= K_MAX_SUGGESTIONS + 1")]
    fn more_than_three_suggestions_panic() {
        let _ = SpellCheckSuggestionsToolbar::new(
            Offset::new(0.0, K_ANCHOR - K_TEST_TOOLBAR_OVERLAP),
            build_suggestion_buttons(&["hello", "yellow", "yell", "yeller"]),
        );
    }

    #[test]
    fn spell_check_suggestions_toolbar_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let root = {
                let app = tester.cell.borrow();
                material_app(
                    ThemeData::new(&app),
                    Center::new()
                        .child(SizedBox::shrink().child(SpellCheckSuggestionsToolbar::new(
                            Offset::new(1.0, 1.0),
                            build_suggestion_buttons(&["X", "Y"]),
                        )))
                        .into_widget(),
                )
            };
            tester.pump_widget(root);
            let toolbars = tester.find_by_type::<SpellCheckSuggestionsToolbar>();
            let render = tester.render_box_of(toolbars[0]);
            assert_eq!(render.size(&tester.cell.borrow()), Size::ZERO);
        });
    }

    #[test]
    fn get_toolbar_anchor_prefers_the_secondary_anchor() {
        let primary = Offset::new(1.0, 2.0);
        let secondary = Offset::new(3.0, 4.0);
        assert_eq!(
            SpellCheckSuggestionsToolbar::get_toolbar_anchor(TextSelectionToolbarAnchors::new(
                primary
            )),
            primary
        );
        assert_eq!(
            SpellCheckSuggestionsToolbar::get_toolbar_anchor(
                TextSelectionToolbarAnchors::new(primary).secondary_anchor(secondary)
            ),
            secondary
        );
    }
}
