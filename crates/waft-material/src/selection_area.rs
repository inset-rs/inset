//! Flutter counterpart: `material/selection_area.dart`.
//!
//! A [`SelectionArea`] is a [`SelectableRegion`] with this platform's selection controls and a
//! Material context menu over them, so a subtree of texts, cards and lists can be selected and
//! copied.

use std::rc::Rc;

use waft_embedder::TargetPlatform;
use waft_foundation::{App, Handle, ValueChanged};
use waft_widgets::{
    BuildContext, FocusNodeRef, GlobalKey, IntoWidget, KeyRef, SelectableRegion,
    SelectableRegionContextMenuBuilder, SelectableRegionState, State, StateData, StatefulWidget,
    TextMagnifierConfiguration, TextSelectionControls, WidgetRef,
};

use waft_cupertino::{
    cupertino_desktop_text_selection_handle_controls, cupertino_text_selection_handle_controls,
};
use waft_rendering::SelectedContent;

use crate::adaptive_text_selection_toolbar::AdaptiveTextSelectionToolbar;
use crate::debug::debug_check_has_material_localizations;
use crate::desktop_text_selection::desktop_text_selection_handle_controls;
use crate::magnifier::TextMagnifier;
use crate::text_selection::material_text_selection_handle_controls;
use crate::theme::Theme;

/// A widget that introduces an area for user selections with adaptive selection controls.
///
/// It is a [`SelectableRegion`] whose controls and context menu follow the ambient
/// [`Theme`]'s platform.
pub struct SelectionArea {
    /// See [`Widget::key`](waft_widgets::Widget::key).
    pub key: Option<KeyRef>,
    /// The configuration for the magnifier in the selection region.
    pub magnifier_configuration: Option<TextMagnifierConfiguration>,
    /// The focus node of the region the area builds.
    pub focus_node: Option<FocusNodeRef>,
    /// The delegate that builds the selection handles and toolbar; one of the platform's own
    /// when `None`.
    pub selection_controls: Option<Rc<dyn TextSelectionControls>>,
    /// Builds the context menu; `None` shows none.
    pub context_menu_builder: Option<SelectableRegionContextMenuBuilder>,
    /// Called when the selected content changes.
    pub on_selection_changed: Option<ValueChanged<Option<SelectedContent>>>,
    /// The child widget this selection area applies to.
    pub child: WidgetRef,
}

impl SelectionArea {
    /// Creates a selection area over `child`.
    pub fn new<K>(child: impl IntoWidget<K>) -> SelectionArea {
        SelectionArea {
            key: None,
            magnifier_configuration: None,
            focus_node: None,
            selection_controls: None,
            context_menu_builder: Some(Rc::new(Self::default_context_menu_builder)),
            on_selection_changed: None,
            child: child.into_widget(),
        }
    }

    /// Dart `SelectionArea(key:)`.
    pub fn key(mut self, key: KeyRef) -> SelectionArea {
        self.key = Some(key);
        self
    }

    /// Dart `SelectionArea(focusNode:)`.
    pub fn focus_node(mut self, focus_node: FocusNodeRef) -> SelectionArea {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `SelectionArea(selectionControls:)`.
    pub fn selection_controls(
        mut self,
        selection_controls: Rc<dyn TextSelectionControls>,
    ) -> SelectionArea {
        self.selection_controls = Some(selection_controls);
        self
    }

    /// Dart `SelectionArea(contextMenuBuilder:)`.
    pub fn context_menu_builder(
        mut self,
        context_menu_builder: Option<SelectableRegionContextMenuBuilder>,
    ) -> SelectionArea {
        self.context_menu_builder = context_menu_builder;
        self
    }

    /// Dart `SelectionArea(magnifierConfiguration:)`.
    pub fn magnifier_configuration(
        mut self,
        magnifier_configuration: TextMagnifierConfiguration,
    ) -> SelectionArea {
        self.magnifier_configuration = Some(magnifier_configuration);
        self
    }

    /// Dart `SelectionArea(onSelectionChanged:)`.
    pub fn on_selection_changed(
        mut self,
        on_selection_changed: ValueChanged<Option<SelectedContent>>,
    ) -> SelectionArea {
        self.on_selection_changed = Some(on_selection_changed);
        self
    }

    /// The area's default context menu, as `AdaptiveTextSelectionToolbar.selectableRegion`.
    fn default_context_menu_builder(
        app: &mut App,
        _context: BuildContext,
        state: Handle<SelectableRegionState>,
    ) -> WidgetRef {
        AdaptiveTextSelectionToolbar::selectable_region(app, state).into_widget()
    }
}

impl std::fmt::Debug for SelectionArea {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectionArea").finish_non_exhaustive()
    }
}

impl StatefulWidget for SelectionArea {
    type State = SelectionAreaState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> SelectionAreaState {
        SelectionAreaState {
            state: StateData::new(),
            selectable_region_key: GlobalKey::new(),
        }
    }
}

/// State for a [`SelectionArea`].
pub struct SelectionAreaState {
    state: StateData<SelectionArea>,
    /// The key of the [`SelectableRegion`] this area builds.
    selectable_region_key: GlobalKey,
}

impl SelectionAreaState {
    /// The state of the [`SelectableRegion`] this area wraps.
    pub fn selectable_region(self: Handle<Self>, app: &App) -> Handle<SelectableRegionState> {
        app.get(self)
            .selectable_region_key
            .current_state::<SelectableRegionState>(app)
            .expect("the area has built its region")
    }
}

impl State for SelectionAreaState {
    type Widget = SelectionArea;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(
            debug_check_has_material_localizations(app, context),
            "A SelectionArea needs a MaterialLocalizations ancestor, as a MaterialApp provides."
        );
        let (
            selection_controls,
            context_menu_builder,
            magnifier_configuration,
            focus_node,
            on_selection_changed,
            child,
        ) = {
            let widget = self.widget(app);
            (
                widget.selection_controls.clone(),
                widget.context_menu_builder.clone(),
                widget.magnifier_configuration.clone(),
                widget.focus_node.clone(),
                widget.on_selection_changed.clone(),
                widget.child.clone(),
            )
        };
        let controls =
            selection_controls.unwrap_or_else(|| match Theme::of(app, context).platform {
                TargetPlatform::Android | TargetPlatform::Fuchsia => {
                    material_text_selection_handle_controls()
                }
                TargetPlatform::Linux | TargetPlatform::Windows => {
                    desktop_text_selection_handle_controls()
                }
                TargetPlatform::IOS => cupertino_text_selection_handle_controls(),
                TargetPlatform::MacOS => cupertino_desktop_text_selection_handle_controls(),
            });
        let region_key = app.get(self).selectable_region_key.clone();
        let mut region = SelectableRegion::new(controls, child)
            .key(KeyRef::Global(region_key))
            .magnifier_configuration(
                magnifier_configuration
                    .unwrap_or_else(|| TextMagnifier::adaptive_magnifier_configuration(app)),
            );
        if let Some(context_menu_builder) = context_menu_builder {
            region = region.context_menu_builder(context_menu_builder);
        }
        if let Some(focus_node) = focus_node {
            region = region.focus_node(focus_node);
        }
        if let Some(on_selection_changed) = on_selection_changed {
            region = region.on_selection_changed(on_selection_changed);
        }
        region.into_widget()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    //! `selection_area_test.dart`'s cases.

    use waft_embedder::{Offset, Rect, TargetPlatform, TextPosition};
    use waft_rendering::{RenderBox, RenderHandle, RenderParagraph};

    use waft_widgets::Text;
    use waft_widgets::testing::{
        TargetPlatformVariant, TestPointer, WidgetTester, find, test_widgets, test_widgets_variant,
    };

    use super::*;
    use crate::app::MaterialApp;
    use crate::text_selection::material_text_selection_handle_controls;
    use waft_cupertino::{
        cupertino_desktop_text_selection_handle_controls, cupertino_text_selection_handle_controls,
    };
    use waft_gestures::K_PRIMARY_BUTTON;

    /// The global position of the caret at `offset` in `paragraph`, as flutter_test's
    /// `textOffsetToPosition` places it.
    fn text_offset_to_position(
        tester: &mut WidgetTester,
        paragraph: RenderHandle<RenderParagraph>,
        offset: i32,
    ) -> Offset {
        tester.update(|app| {
            let caret = Rect::from_ltwh(0.0, 0.0, 2.0, 20.0);
            let local = paragraph.get_offset_for_caret(app, TextPosition::new(offset), caret);
            paragraph.as_box().local_to_global(app, local, None)
        })
    }

    /// The paragraph showing `text`.
    fn paragraph_showing(tester: &WidgetTester, text: &str) -> RenderHandle<RenderParagraph> {
        let paragraphs = tester.all::<RenderParagraph>();
        tester.update(|app| {
            *paragraphs
                .iter()
                .find(|paragraph| paragraph.text(app).to_plain_text(true, true) == text)
                .unwrap_or_else(|| panic!("a paragraph showing {text}"))
        })
    }

    // selection_area_test.dart 'SelectionArea uses correct selection controls'.
    #[test]
    fn selection_area_uses_correct_selection_controls() {
        test_widgets_variant(TargetPlatformVariant::all(), |tester| {
            tester.pump_widget(MaterialApp::new().home(SelectionArea::new(Text::new("abc"))));
            let region = tester.widget(
                tester.element(find::by_type::<SelectableRegion>()),
                |region: &SelectableRegion| region.selection_controls.clone(),
            );
            let expected: Rc<dyn TextSelectionControls> = match tester.target_platform() {
                TargetPlatform::Android | TargetPlatform::Fuchsia => {
                    material_text_selection_handle_controls()
                }
                TargetPlatform::IOS => cupertino_text_selection_handle_controls(),
                TargetPlatform::Linux | TargetPlatform::Windows => {
                    desktop_text_selection_handle_controls()
                }
                TargetPlatform::MacOS => cupertino_desktop_text_selection_handle_controls(),
            };
            assert!(Rc::ptr_eq(&region, &expected));
        });
    }

    // selection_area_test.dart 'onSelectionChange is called when the selection changes'.
    #[test]
    fn on_selection_change_is_called_when_the_selection_changes() {
        test_widgets(|tester| {
            let content: Rc<std::cell::RefCell<Option<SelectedContent>>> =
                Rc::new(std::cell::RefCell::new(None));
            let log = Rc::clone(&content);
            tester.pump_widget(MaterialApp::new().home(
                SelectionArea::new(Text::new("How are you")).on_selection_changed(Rc::new(
                    move |_app, selected_content| *log.borrow_mut() = selected_content,
                )),
            ));
            let plain_text = || content.borrow().as_ref().map(|c| c.plain_text.clone());
            let paragraph = paragraph_showing(tester, "How are you");
            let mut gesture = TestPointer::mouse(tester.next_pointer(), K_PRIMARY_BUTTON);
            let at = text_offset_to_position(tester, paragraph, 4);
            tester.down(&mut gesture, at);
            assert!(content.borrow().is_none());
            tester.pump();

            let at = text_offset_to_position(tester, paragraph, 7);
            tester.move_to(&mut gesture, at, true);
            tester.up(&gesture);
            tester.pump();
            assert_eq!(plain_text(), Some("are".to_string()));

            // Backwards selection.
            let at = text_offset_to_position(tester, paragraph, 3);
            tester.down(&mut gesture, at);
            tester.pump();
            tester.up(&gesture);
            tester.pump_and_settle_by(waft_gestures::K_DOUBLE_TAP_TIMEOUT);
            assert_eq!(plain_text(), Some(String::new()));

            let at = text_offset_to_position(tester, paragraph, 3);
            tester.down(&mut gesture, at);
            tester.pump();
            let at = text_offset_to_position(tester, paragraph, 0);
            tester.move_to(&mut gesture, at, true);
            tester.up(&gesture);
            tester.pump();
            assert_eq!(plain_text(), Some("How".to_string()));
            tester.remove_pointer(&gesture);
        });
    }

    // selection_area_test.dart 'SelectionArea does not crash at zero area'.
    #[test]
    fn selection_area_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            tester.pump_widget(MaterialApp::new().home(waft_widgets::Center::new().child(
                waft_widgets::SizedBox::shrink().child(SelectionArea::new(Text::new("X"))),
            )));
            assert_eq!(
                tester.get_size(find::by_type::<SelectionArea>()),
                waft_embedder::Size::ZERO
            );
        });
    }
}
