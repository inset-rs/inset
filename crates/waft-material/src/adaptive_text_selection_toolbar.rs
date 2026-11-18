//! Flutter counterpart: material_ui's `adaptive_text_selection_toolbar.dart`.
//!
//! `AdaptiveTextSelectionToolbar.selectable` waits on `SelectableText`.

use std::fmt::{self, Debug};

use waft_cupertino::{
    CupertinoDesktopTextSelectionToolbar, CupertinoDesktopTextSelectionToolbarButton,
    CupertinoTextSelectionToolbar, CupertinoTextSelectionToolbarButton,
};
use waft_embedder::TargetPlatform;
use waft_foundation::{App, Handle, Listener};
use waft_painting::AlignmentGeometry;
use waft_widgets::{
    BuildContext, ClipboardStatus, ContextMenuButtonItem, ContextMenuButtonType, EditableText,
    EditableTextState, IntoWidget, KeyRef, SelectableRegionState, SizedBox, StatelessWidget, Text,
    TextSelectionToolbarAnchors, WidgetRef,
};

use crate::debug::debug_check_has_material_localizations;
use crate::desktop_text_selection_toolbar::DesktopTextSelectionToolbar;
use crate::desktop_text_selection_toolbar_button::DesktopTextSelectionToolbarButton;
use crate::material_localizations::MaterialLocalizations;
use crate::text_selection_toolbar::TextSelectionToolbar;
use crate::text_selection_toolbar_text_button::TextSelectionToolbarTextButton;
use crate::theme::Theme;

/// The default context menu for text selection for the current platform.
///
/// Typically, this widget would be passed to `contextMenuBuilder` in a
/// supported parent widget, such as:
///
/// * `EditableText.contextMenuBuilder`
/// * `TextField.contextMenuBuilder`
/// * `CupertinoTextField.contextMenuBuilder`
/// * `SelectionArea.contextMenuBuilder`
/// * `SelectableText.contextMenuBuilder`
///
/// See also:
///
/// * [`EditableText::get_editable_button_items`], which returns the default
///   [`ContextMenuButtonItem`]s for [`EditableText`] on the platform.
/// * [`AdaptiveTextSelectionToolbar::get_adaptive_buttons`], which builds the button
///   Widgets for the current platform given [`ContextMenuButtonItem`]s.
/// * `CupertinoAdaptiveTextSelectionToolbar`, which does the same thing as this
///   widget but only for Cupertino context menus.
/// * [`TextSelectionToolbar`], the default toolbar for Android.
/// * [`DesktopTextSelectionToolbar`], the default toolbar for desktop platforms
///   other than MacOS.
/// * [`CupertinoTextSelectionToolbar`], the default toolbar for iOS.
/// * [`CupertinoDesktopTextSelectionToolbar`], the default toolbar for MacOS.
pub struct AdaptiveTextSelectionToolbar {
    pub key: Option<KeyRef>,
    /// The [`ContextMenuButtonItem`]s that will be turned into the correct button
    /// widgets for the current platform.
    pub button_items: Option<Vec<ContextMenuButtonItem>>,
    /// The children of the toolbar, typically buttons.
    pub children: Option<Vec<WidgetRef>>,
    /// The location on which to anchor the menu.
    pub anchors: TextSelectionToolbarAnchors,
}

impl Debug for AdaptiveTextSelectionToolbar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AdaptiveTextSelectionToolbar")
            .field("button_items", &self.button_items)
            .finish_non_exhaustive()
    }
}

impl AdaptiveTextSelectionToolbar {
    /// Create an instance of [`AdaptiveTextSelectionToolbar`] with the
    /// given [`children`](Self::children).
    ///
    /// See also:
    ///
    /// * [`AdaptiveTextSelectionToolbar::button_items`], which takes a list of
    ///   [`ContextMenuButtonItem`]s instead of [`children`](Self::children) widgets.
    /// * [`AdaptiveTextSelectionToolbar::editable`], which builds the default
    ///   children for an editable field.
    /// * [`AdaptiveTextSelectionToolbar::editable_text`], which builds the default
    ///   children for an [`EditableText`].
    pub fn new(
        children: Option<Vec<WidgetRef>>,
        anchors: TextSelectionToolbarAnchors,
    ) -> AdaptiveTextSelectionToolbar {
        AdaptiveTextSelectionToolbar {
            key: None,
            button_items: None,
            children,
            anchors,
        }
    }

    /// Dart `AdaptiveTextSelectionToolbar(key:)`.
    pub fn key(mut self, key: KeyRef) -> AdaptiveTextSelectionToolbar {
        self.key = Some(key);
        self
    }

    /// Create an instance of [`AdaptiveTextSelectionToolbar`] whose children will
    /// be built from the given [`button_items`](Self::button_items).
    ///
    /// See also:
    ///
    /// * [`AdaptiveTextSelectionToolbar::new`], which takes the children directly as
    ///   a list of widgets.
    /// * [`AdaptiveTextSelectionToolbar::editable`], which builds the default
    ///   children for an editable field.
    /// * [`AdaptiveTextSelectionToolbar::editable_text`], which builds the default
    ///   children for an [`EditableText`].
    pub fn button_items(
        button_items: Vec<ContextMenuButtonItem>,
        anchors: TextSelectionToolbarAnchors,
    ) -> AdaptiveTextSelectionToolbar {
        AdaptiveTextSelectionToolbar {
            key: None,
            button_items: Some(button_items),
            children: None,
            anchors,
        }
    }

    /// Create an instance of [`AdaptiveTextSelectionToolbar`] with the default
    /// children for an editable field.
    ///
    /// If an on* callback parameter is null, then its corresponding button will
    /// not be built.
    ///
    /// These callbacks are called when their corresponding button is activated
    /// and only then. For example, `on_paste` is called when the user taps the
    /// "Paste" button in the context menu and not when the user pastes with the
    /// keyboard.
    ///
    /// See also:
    ///
    /// * [`AdaptiveTextSelectionToolbar::new`], which takes the children directly as
    ///   a list of widgets.
    /// * [`AdaptiveTextSelectionToolbar::editable_text`], which builds the default
    ///   children for an [`EditableText`].
    /// * [`AdaptiveTextSelectionToolbar::button_items`], which takes a list of
    ///   [`ContextMenuButtonItem`]s instead of [`children`](Self::children) widgets.
    #[allow(clippy::too_many_arguments)]
    pub fn editable(
        app: &App,
        clipboard_status: ClipboardStatus,
        on_copy: Option<Listener>,
        on_cut: Option<Listener>,
        on_paste: Option<Listener>,
        on_select_all: Option<Listener>,
        on_look_up: Option<Listener>,
        on_search_web: Option<Listener>,
        on_share: Option<Listener>,
        on_live_text_input: Option<Listener>,
        anchors: TextSelectionToolbarAnchors,
    ) -> AdaptiveTextSelectionToolbar {
        AdaptiveTextSelectionToolbar {
            key: None,
            children: None,
            button_items: Some(EditableText::get_editable_button_items(
                Some(clipboard_status),
                on_copy,
                on_cut,
                on_paste,
                on_select_all,
                on_look_up,
                on_search_web,
                on_share,
                on_live_text_input,
                app.platform().target_platform(),
            )),
            anchors,
        }
    }

    /// Create an instance of [`AdaptiveTextSelectionToolbar`] with the default
    /// children for an [`EditableText`].
    ///
    /// See also:
    ///
    /// * [`AdaptiveTextSelectionToolbar::new`], which takes the children directly as
    ///   a list of widgets.
    /// * [`AdaptiveTextSelectionToolbar::editable`], which builds the default
    ///   children for an editable field.
    /// * [`AdaptiveTextSelectionToolbar::button_items`], which takes a list of
    ///   [`ContextMenuButtonItem`]s instead of [`children`](Self::children) widgets.
    pub fn editable_text(
        app: &mut App,
        editable_text_state: Handle<EditableTextState>,
    ) -> AdaptiveTextSelectionToolbar {
        AdaptiveTextSelectionToolbar {
            key: None,
            children: None,
            button_items: Some(editable_text_state.context_menu_button_items(app)),
            anchors: editable_text_state.context_menu_anchors(app),
        }
    }

    /// Dart `AdaptiveTextSelectionToolbar.selectableRegion`.
    pub fn selectable_region(
        app: &mut App,
        selectable_region_state: Handle<SelectableRegionState>,
    ) -> AdaptiveTextSelectionToolbar {
        AdaptiveTextSelectionToolbar {
            key: None,
            children: None,
            button_items: Some(selectable_region_state.context_menu_button_items(app)),
            anchors: selectable_region_state.context_menu_anchors(app),
        }
    }

    /// Returns the default button label String for the button of the given
    /// [`ContextMenuButtonType`] on any platform.
    pub fn get_button_label(
        app: &mut App,
        context: BuildContext,
        button_item: &ContextMenuButtonItem,
    ) -> String {
        if let Some(label) = &button_item.label {
            return label.clone();
        }

        match Theme::of(app, context).platform {
            TargetPlatform::IOS | TargetPlatform::MacOS => {
                CupertinoTextSelectionToolbarButton::get_button_label(app, context, button_item)
            }
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => {
                debug_assert!(debug_check_has_material_localizations(app, context));
                let localizations = <dyn MaterialLocalizations>::of(app, context);
                match button_item.r#type {
                    ContextMenuButtonType::Cut => localizations.cut_button_label(),
                    ContextMenuButtonType::Copy => localizations.copy_button_label(),
                    ContextMenuButtonType::Paste => localizations.paste_button_label(),
                    ContextMenuButtonType::SelectAll => localizations.select_all_button_label(),
                    ContextMenuButtonType::Delete => {
                        localizations.delete_button_tooltip().to_uppercase()
                    }
                    ContextMenuButtonType::LookUp => localizations.look_up_button_label(),
                    ContextMenuButtonType::SearchWeb => localizations.search_web_button_label(),
                    ContextMenuButtonType::Share => localizations.share_button_label(),
                    ContextMenuButtonType::LiveTextInput => localizations.scan_text_button_label(),
                    ContextMenuButtonType::Custom => String::new(),
                }
            }
        }
    }

    /// Returns a List of Widgets generated by turning [`button_items`](Self::button_items) into
    /// the default context menu buttons for the current platform.
    ///
    /// This is useful when building a text selection toolbar with the default
    /// button appearance for the given platform, but where the toolbar and/or the
    /// button actions and labels may be custom.
    ///
    /// See also:
    ///
    /// * `CupertinoAdaptiveTextSelectionToolbar::get_adaptive_buttons`, which is the
    ///   Cupertino equivalent of this class and builds only the Cupertino
    ///   buttons.
    pub fn get_adaptive_buttons(
        app: &mut App,
        context: BuildContext,
        button_items: Vec<ContextMenuButtonItem>,
    ) -> Vec<WidgetRef> {
        match Theme::of(app, context).platform {
            TargetPlatform::IOS => button_items
                .into_iter()
                .map(|button_item| {
                    CupertinoTextSelectionToolbarButton::button_item(button_item).into_widget()
                })
                .collect(),
            TargetPlatform::Fuchsia | TargetPlatform::Android => {
                let mut buttons: Vec<WidgetRef> = Vec::new();
                let total = button_items.len();
                for (i, button_item) in button_items.iter().enumerate() {
                    let mut button = TextSelectionToolbarTextButton::new(
                        Text::new(Self::get_button_label(app, context, button_item)),
                        TextSelectionToolbarTextButton::get_padding(i, total),
                    )
                    .alignment(AlignmentGeometry::CENTER_START);
                    if let Some(on_pressed) = button_item.on_pressed.clone() {
                        button = button.on_pressed(on_pressed);
                    }
                    buttons.push(button.into_widget());
                }
                buttons
            }
            TargetPlatform::Linux | TargetPlatform::Windows => button_items
                .iter()
                .map(|button_item| {
                    let text = Self::get_button_label(app, context, button_item);
                    DesktopTextSelectionToolbarButton::text(
                        app,
                        context,
                        button_item.on_pressed.clone(),
                        text,
                    )
                    .into_widget()
                })
                .collect(),
            TargetPlatform::MacOS => button_items
                .iter()
                .map(|button_item| {
                    CupertinoDesktopTextSelectionToolbarButton::text(
                        button_item.on_pressed.clone(),
                        Self::get_button_label(app, context, button_item),
                    )
                    .into_widget()
                })
                .collect(),
        }
    }
}

impl StatelessWidget for AdaptiveTextSelectionToolbar {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        // If there aren't any buttons to build, build an empty toolbar.
        let empty = match (&self.children, &self.button_items) {
            (Some(children), _) => children.is_empty(),
            (None, Some(button_items)) => button_items.is_empty(),
            (None, None) => true,
        };
        if empty {
            return SizedBox::shrink().into_widget();
        }

        let result_children = match &self.children {
            Some(children) => children.clone(),
            None => Self::get_adaptive_buttons(
                app,
                context,
                self.button_items
                    .clone()
                    .expect("children or buttonItems is given"),
            ),
        };

        let anchor_below = self
            .anchors
            .secondary_anchor
            .unwrap_or(self.anchors.primary_anchor);
        match Theme::of(app, context).platform {
            TargetPlatform::IOS => CupertinoTextSelectionToolbar::new(
                self.anchors.primary_anchor,
                anchor_below,
                result_children,
            )
            .into_widget(),
            TargetPlatform::Android => TextSelectionToolbar::new(
                self.anchors.primary_anchor,
                anchor_below,
                result_children,
            )
            .into_widget(),
            TargetPlatform::Fuchsia | TargetPlatform::Linux | TargetPlatform::Windows => {
                DesktopTextSelectionToolbar::new(self.anchors.primary_anchor, result_children)
                    .into_widget()
            }
            TargetPlatform::MacOS => CupertinoDesktopTextSelectionToolbar::new(
                self.anchors.primary_anchor,
                result_children,
            )
            .into_widget(),
        }
    }
}

#[cfg(test)]
mod tests {
    use waft_embedder::{Offset, Size};
    use waft_widgets::{
        AnyElement, Builder, Center, Container, CustomPaint, GlobalKey, KeyRef, ListView,
        StandardComponentType,
    };

    use waft_widgets::testing::{
        TargetPlatformVariant, WidgetTester, expect, find, finds_n_widgets, finds_one_widget,
        test_widgets, test_widgets_variant,
    };

    use super::*;
    use crate::scaffold::Scaffold;
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    /// Flutter's `TargetPlatformVariant.all()`.
    const ALL_PLATFORMS: [TargetPlatform; 6] = [
        TargetPlatform::Android,
        TargetPlatform::Fuchsia,
        TargetPlatform::IOS,
        TargetPlatform::Linux,
        TargetPlatform::MacOS,
        TargetPlatform::Windows,
    ];

    /// A tester on `platform` with flutter_test's fonts, so a `Text` measures as in Flutter's
    /// tests.
    /// `MaterialApp(home: Scaffold(body: Center(child: child)))` under the default theme.
    fn pump_centered_in_scaffold(tester: &mut WidgetTester, child: WidgetRef) {
        let root = {
            let app = tester.cell.borrow();
            material_app(
                ThemeData::new(&app),
                Scaffold::new()
                    .body(Center::new().child(child))
                    .into_widget(),
            )
        };
        tester.pump_widget(root);
    }

    fn pressed() -> Option<Listener> {
        Some(Listener::new(|_app| {}))
    }

    #[test]
    fn builds_the_right_toolbar_on_each_platform_including_web_and_shows_button_items() {
        for platform in ALL_PLATFORMS {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                const BUTTON_TEXT: &str = "Click me";

                pump_centered_in_scaffold(
                    tester,
                    AdaptiveTextSelectionToolbar::button_items(
                        vec![ContextMenuButtonItem::new(pressed()).label(BUTTON_TEXT)],
                        TextSelectionToolbarAnchors::new(Offset::ZERO),
                    )
                    .into_widget(),
                );

                expect(tester, &find::text(BUTTON_TEXT), finds_one_widget());

                let counts = (
                    find::by_type::<TextSelectionToolbar>()
                        .evaluate(tester)
                        .len(),
                    find::by_type::<CupertinoTextSelectionToolbar>()
                        .evaluate(tester)
                        .len(),
                    find::by_type::<DesktopTextSelectionToolbar>()
                        .evaluate(tester)
                        .len(),
                    find::by_type::<CupertinoDesktopTextSelectionToolbar>()
                        .evaluate(tester)
                        .len(),
                );
                let expected = match platform {
                    TargetPlatform::Android => (1, 0, 0, 0),
                    TargetPlatform::IOS => (0, 1, 0, 0),
                    TargetPlatform::MacOS => (0, 0, 0, 1),
                    TargetPlatform::Fuchsia | TargetPlatform::Linux | TargetPlatform::Windows => {
                        (0, 0, 1, 0)
                    }
                };
                assert_eq!(counts, expected, "{platform:?}");
            });
        }
    }

    #[test]
    fn can_build_children_directly_as_well() {
        test_widgets(|tester| {
            let key = GlobalKey::new();

            pump_centered_in_scaffold(
                tester,
                AdaptiveTextSelectionToolbar::new(
                    Some(vec![Container::new().key(key.clone().into()).into_widget()]),
                    TextSelectionToolbarAnchors::new(Offset::ZERO),
                )
                .into_widget(),
            );

            expect(tester, &find::by_key(key.clone()), finds_one_widget());
        });
    }

    /// Dart's `findLiveTextButton()` on the desktop platforms.
    fn find_live_text_button(tester: &WidgetTester, platform: TargetPlatform) -> usize {
        match platform {
            TargetPlatform::MacOS => find::descendant(
                find::by_type::<CupertinoDesktopTextSelectionToolbarButton>(),
                find::text(""),
            )
            .evaluate(tester)
            .len(),
            // The icon is a painted `CustomPaint` in a `SizedBox` on iOS, where `Text`s are not used.
            TargetPlatform::IOS => cupertino_button_paints(tester, Size::ZERO).len(),
            _ => {
                find::text("").evaluate(tester).len()
                    + find::text("Scan text").evaluate(tester).len()
            }
        }
    }

    #[test]
    fn can_build_for_editable_text_from_raw_parameters() {
        for platform in ALL_PLATFORMS {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let key = GlobalKey::new();
                let toolbar = {
                    let app = tester.cell.borrow();
                    AdaptiveTextSelectionToolbar::editable(
                        &app,
                        ClipboardStatus::Pasteable,
                        pressed(),
                        pressed(),
                        pressed(),
                        pressed(),
                        pressed(),
                        pressed(),
                        pressed(),
                        pressed(),
                        TextSelectionToolbarAnchors::new(Offset::ZERO),
                    )
                    .key(key.clone().into())
                    .into_widget()
                };
                pump_centered_in_scaffold(tester, toolbar);

                expect(tester, &find::by_key(key.clone()), finds_one_widget());

                let one = |tester: &WidgetTester, text: &str| {
                    expect(tester, &find::text(text), finds_one_widget());
                };
                match platform {
                    TargetPlatform::Fuchsia => {
                        expect(
                            tester,
                            &find::by_type::<TextSelectionToolbarTextButton>(),
                            finds_n_widgets(8),
                        );
                        one(tester, "Cut");
                        one(tester, "Copy");
                        one(tester, "Paste");
                        one(tester, "Select all");
                        one(tester, "Look Up");
                        one(tester, "Search Web");
                        one(tester, "Share");
                    }
                    TargetPlatform::Linux | TargetPlatform::Windows => {
                        expect(
                            tester,
                            &find::by_type::<DesktopTextSelectionToolbarButton>(),
                            finds_n_widgets(8),
                        );
                        one(tester, "Cut");
                        one(tester, "Copy");
                        one(tester, "Paste");
                        one(tester, "Select all");
                        one(tester, "Look Up");
                        one(tester, "Search Web");
                        one(tester, "Share");
                        assert_eq!(find_live_text_button(tester, platform), 1);
                    }
                    TargetPlatform::MacOS => {
                        expect(
                            tester,
                            &find::by_type::<CupertinoDesktopTextSelectionToolbarButton>(),
                            finds_n_widgets(8),
                        );
                        one(tester, "Cut");
                        one(tester, "Copy");
                        one(tester, "Paste");
                        one(tester, "Select All");
                        one(tester, "Look Up");
                        one(tester, "Search Web");
                        one(tester, "Share...");
                        assert_eq!(find_live_text_button(tester, platform), 1);
                    }
                    TargetPlatform::Android => {
                        expect(
                            tester,
                            &find::by_type::<TextSelectionToolbarTextButton>(),
                            finds_n_widgets(6),
                        );
                        one(tester, "Cut");
                        one(tester, "Copy");
                        one(tester, "Paste");
                        one(tester, "Share");
                        one(tester, "Select all");
                        one(tester, "Look Up");
                        // Material overflow buttons are not TextSelectionToolbarTextButton.
                        expect(
                            tester,
                            &find::by_key(StandardComponentType::MoreButton.key()),
                            finds_one_widget(),
                        );

                        tap_overflow_button(tester, &StandardComponentType::MoreButton.key());

                        expect(
                            tester,
                            &find::by_type::<TextSelectionToolbarTextButton>(),
                            finds_n_widgets(2),
                        );
                        one(tester, "Search Web");
                        assert_eq!(find_live_text_button(tester, platform), 1);
                        expect(
                            tester,
                            &find::by_key(StandardComponentType::BackButton.key()),
                            finds_one_widget(),
                        );
                    }
                    TargetPlatform::IOS => {
                        expect(
                            tester,
                            &find::by_type::<CupertinoTextSelectionToolbarButton>(),
                            finds_n_widgets(6),
                        );
                        one(tester, "Cut");
                        one(tester, "Copy");
                        one(tester, "Paste");
                        one(tester, "Select All");
                        one(tester, "Look Up");
                        assert_eq!(cupertino_chevron_buttons(tester).len(), 1);

                        tap_cupertino_chevron_button(tester);

                        expect(
                            tester,
                            &find::by_type::<CupertinoTextSelectionToolbarButton>(),
                            finds_n_widgets(4),
                        );
                        assert_eq!(cupertino_chevron_buttons(tester).len(), 1);
                        one(tester, "Search Web");
                        one(tester, "Share...");
                        assert_eq!(find_live_text_button(tester, platform), 1);
                    }
                }
            });
        }
    }

    /// Dart's `tester.tapAt(tester.getCenter(find.byKey(key)))` and `pumpAndSettle`
    /// (`tapMaterialOverflowNextButton`).
    fn tap_overflow_button(tester: &mut WidgetTester, key: &KeyRef) {
        let button = tester.element(find::by_key(key.clone()));
        let center = tester.get_rect(button).center();
        tester.tap_at(center, 1);
        tester.pump_and_settle();
    }

    /// The `CustomPaint`s inside the `CupertinoTextSelectionToolbarButton`s whose size is `size`.
    fn cupertino_button_paints(tester: &WidgetTester, size: Size) -> Vec<AnyElement> {
        find::by_type::<CupertinoTextSelectionToolbarButton>()
            .evaluate(tester)
            .into_iter()
            .flat_map(|button| tester.descendants_of_type::<CustomPaint>(button))
            .filter(|paint| tester.widget(*paint, |paint: &CustomPaint| paint.size == size))
            .collect()
    }

    /// The `CupertinoTextSelectionToolbarButton`s that hold a chevron
    /// (`findCupertinoOverflowNextButton` and `findCupertinoOverflowBackButton`): the chevron
    /// painters are private to waft-cupertino, so the `CustomPaint` of the chevron's size (10)
    /// stands for them. A page shows one of the two.
    fn cupertino_chevron_buttons(tester: &WidgetTester) -> Vec<AnyElement> {
        find::by_type::<CupertinoTextSelectionToolbarButton>()
            .evaluate(tester)
            .into_iter()
            .filter(|button| {
                tester
                    .descendants_of_type::<CustomPaint>(*button)
                    .into_iter()
                    .any(|paint| {
                        tester.widget(paint, |paint: &CustomPaint| {
                            paint.size == Size::square(10.0)
                        })
                    })
            })
            .collect()
    }

    /// Dart's `tapCupertinoOverflowNextButton`.
    fn tap_cupertino_chevron_button(tester: &mut WidgetTester) {
        let button = cupertino_chevron_buttons(tester)[0];
        let center = tester.get_rect(button).center();
        tester.tap_at(center, 1);
        tester.pump_and_settle();
    }

    #[test]
    fn builds_empty_toolbar_when_children_and_button_items_are_null() {
        test_widgets(|_tester| {
            for platform in ALL_PLATFORMS {
                let mut tester = WidgetTester::on(platform);
                let root = {
                    let app = tester.cell.borrow();
                    material_app(
                        ThemeData::new(&app),
                        Center::new()
                            .child(AdaptiveTextSelectionToolbar::new(
                                None,
                                TextSelectionToolbarAnchors::new(Offset::ZERO),
                            ))
                            .into_widget(),
                    )
                };
                tester.pump_widget(root);

                let toolbars = find::by_type::<AdaptiveTextSelectionToolbar>().evaluate(&tester);
                let render = tester.render_box_of(toolbars[0]);
                assert_eq!(
                    render.size(&tester.cell.borrow()),
                    Size::ZERO,
                    "{platform:?}"
                );
            }
        });
    }

    #[test]
    fn get_adaptive_buttons_builds_the_correct_button_widgets_per_platform() {
        for platform in ALL_PLATFORMS {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                const BUTTON_TEXT: &str = "Click me";

                pump_centered_in_scaffold(
                    tester,
                    Builder::new(|app, context| {
                        let button_items =
                            vec![ContextMenuButtonItem::new(pressed()).label(BUTTON_TEXT)];
                        ListView::new()
                            .children(AdaptiveTextSelectionToolbar::get_adaptive_buttons(
                                app,
                                context,
                                button_items,
                            ))
                            .into_widget()
                    })
                    .into_widget(),
                );

                expect(tester, &find::text(BUTTON_TEXT), finds_one_widget());

                let counts = (
                    find::by_type::<TextSelectionToolbarTextButton>()
                        .evaluate(tester)
                        .len(),
                    find::by_type::<CupertinoTextSelectionToolbarButton>()
                        .evaluate(tester)
                        .len(),
                    find::by_type::<DesktopTextSelectionToolbarButton>()
                        .evaluate(tester)
                        .len(),
                    find::by_type::<CupertinoDesktopTextSelectionToolbarButton>()
                        .evaluate(tester)
                        .len(),
                );
                let expected = match platform {
                    TargetPlatform::Fuchsia | TargetPlatform::Android => (1, 0, 0, 0),
                    TargetPlatform::IOS => (0, 1, 0, 0),
                    TargetPlatform::MacOS => (0, 0, 0, 1),
                    TargetPlatform::Linux | TargetPlatform::Windows => (0, 0, 1, 0),
                };
                assert_eq!(counts, expected, "{platform:?}");
            });
        }
    }
}
