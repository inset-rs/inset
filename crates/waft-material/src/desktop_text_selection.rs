//! Flutter counterpart: material_ui's `desktop_text_selection.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::{Offset, Rect, Size, clamp_double};
use waft_foundation::{App, Handle, Listenable, Listener, Ref, Task};
use waft_rendering::TextSelectionPoint;
use waft_services::{AnyTextSelectionDelegate, SelectionChangedCause};
use waft_widgets::{
    BuildContext, ClipboardStatus, ClipboardStatusNotifier, IntoWidget, KeyRef, MediaQuery,
    SizedBox, State, StateData, StatefulWidget, TextSelectionControls, TextSelectionHandleControls,
    TextSelectionHandleType, WidgetRef, debug_check_has_media_query,
};

use crate::debug::debug_check_has_material_localizations;
use crate::desktop_text_selection_toolbar::DesktopTextSelectionToolbar;
use crate::desktop_text_selection_toolbar_button::DesktopTextSelectionToolbarButton;
use crate::material_localizations::MaterialLocalizations;

/// Desktop Material styled text selection handle controls.
///
/// Specifically does not manage the toolbar, which is left to
/// `EditableText.contextMenuBuilder`.
#[derive(Debug)]
struct DesktopTextSelectionHandleControls;

/// Desktop Material styled text selection controls.
///
/// [`desktop_text_selection_controls`] has a suitable instance of this class.
#[derive(Debug, Default)]
pub struct DesktopTextSelectionControls;

impl DesktopTextSelectionControls {
    /// Creates desktop Material styled text selection controls.
    pub const fn new() -> DesktopTextSelectionControls {
        DesktopTextSelectionControls
    }
}

impl TextSelectionControls for DesktopTextSelectionControls {
    /// Desktop has no text selection handles.
    fn get_handle_size(&self, _text_line_height: f64) -> Size {
        Size::ZERO
    }

    /// Builder for the Material-style desktop copy/paste text selection toolbar.
    #[allow(deprecated, clippy::too_many_arguments)]
    fn build_toolbar(
        &self,
        app: &mut App,
        _context: BuildContext,
        global_editable_region: Rect,
        text_line_height: f64,
        selection_midpoint: Offset,
        endpoints: &[TextSelectionPoint],
        delegate: AnyTextSelectionDelegate,
        clipboard_status: Option<Ref<ClipboardStatusNotifier>>,
        last_secondary_tap_down_position: Option<Offset>,
    ) -> WidgetRef {
        let handle_cut = self.can_cut(app, delegate).then(|| {
            Listener::new(move |app| {
                DesktopTextSelectionControls.handle_cut(app, delegate);
            })
        });
        let handle_copy = self.can_copy(app, delegate).then(|| {
            Listener::new(move |app| {
                DesktopTextSelectionControls.handle_copy(app, delegate);
            })
        });
        let handle_paste = self.can_paste(app, delegate).then(|| {
            Listener::new(move |app| {
                DesktopTextSelectionControls.handle_paste(app, delegate);
            })
        });
        let handle_select_all = self.can_select_all(app, delegate).then(|| {
            Listener::new(move |app| {
                DesktopTextSelectionControls.handle_select_all(app, delegate);
            })
        });
        DesktopTextSelectionControlsToolbar {
            key: None,
            clipboard_status,
            endpoints: endpoints.to_vec(),
            global_editable_region,
            handle_cut,
            handle_copy,
            handle_paste,
            handle_select_all,
            selection_midpoint,
            last_secondary_tap_down_position,
            text_line_height,
        }
        .into_widget()
    }

    /// Builds the text selection handles, but desktop has none.
    fn build_handle(
        &self,
        _app: &mut App,
        _context: BuildContext,
        _handle_type: TextSelectionHandleType,
        _text_line_height: f64,
        _on_tap: Option<Listener>,
    ) -> WidgetRef {
        SizedBox::shrink().into_widget()
    }

    /// Gets the position for the text selection handles, but desktop has none.
    fn get_handle_anchor(
        &self,
        _handle_type: TextSelectionHandleType,
        _text_line_height: f64,
    ) -> Offset {
        Offset::ZERO
    }

    #[allow(deprecated)]
    fn can_select_all(&self, app: &App, delegate: AnyTextSelectionDelegate) -> bool {
        // Allow SelectAll when selection is not collapsed, unless everything has
        // already been selected. Same behavior as Android.
        let value = delegate.text_editing_value(app);
        delegate.select_all_enabled(app)
            && !value.text.is_empty()
            && !(value.selection.start() == 0
                && value.selection.end() == value.text.encode_utf16().count() as i32)
    }

    #[allow(deprecated)]
    fn handle_select_all(&self, app: &mut App, delegate: AnyTextSelectionDelegate) {
        // Dart's `super.handleSelectAll(delegate)`.
        delegate.select_all(app, SelectionChangedCause::Toolbar);
        delegate.hide_toolbar(app, true);
    }
}

impl TextSelectionControls for DesktopTextSelectionHandleControls {
    fn as_text_selection_handle_controls(&self) -> Option<&dyn TextSelectionHandleControls> {
        Some(self)
    }

    fn get_handle_size(&self, text_line_height: f64) -> Size {
        DesktopTextSelectionControls.get_handle_size(text_line_height)
    }

    #[allow(deprecated, clippy::too_many_arguments)]
    fn build_toolbar(
        &self,
        app: &mut App,
        context: BuildContext,
        global_editable_region: Rect,
        text_line_height: f64,
        selection_midpoint: Offset,
        endpoints: &[TextSelectionPoint],
        delegate: AnyTextSelectionDelegate,
        clipboard_status: Option<Ref<ClipboardStatusNotifier>>,
        last_secondary_tap_down_position: Option<Offset>,
    ) -> WidgetRef {
        TextSelectionHandleControls::build_toolbar(
            self,
            app,
            context,
            global_editable_region,
            text_line_height,
            selection_midpoint,
            endpoints,
            delegate,
            clipboard_status,
            last_secondary_tap_down_position,
        )
    }

    fn build_handle(
        &self,
        app: &mut App,
        context: BuildContext,
        handle_type: TextSelectionHandleType,
        text_line_height: f64,
        on_tap: Option<Listener>,
    ) -> WidgetRef {
        DesktopTextSelectionControls.build_handle(
            app,
            context,
            handle_type,
            text_line_height,
            on_tap,
        )
    }

    fn get_handle_anchor(
        &self,
        handle_type: TextSelectionHandleType,
        text_line_height: f64,
    ) -> Offset {
        DesktopTextSelectionControls.get_handle_anchor(handle_type, text_line_height)
    }

    #[allow(deprecated)]
    fn can_cut(&self, app: &App, delegate: AnyTextSelectionDelegate) -> bool {
        TextSelectionHandleControls::can_cut(self, app, delegate)
    }

    #[allow(deprecated)]
    fn can_copy(&self, app: &App, delegate: AnyTextSelectionDelegate) -> bool {
        TextSelectionHandleControls::can_copy(self, app, delegate)
    }

    #[allow(deprecated)]
    fn can_paste(&self, app: &App, delegate: AnyTextSelectionDelegate) -> bool {
        TextSelectionHandleControls::can_paste(self, app, delegate)
    }

    #[allow(deprecated)]
    fn can_select_all(&self, app: &App, delegate: AnyTextSelectionDelegate) -> bool {
        TextSelectionHandleControls::can_select_all(self, app, delegate)
    }

    #[allow(deprecated)]
    fn handle_cut(&self, app: &mut App, delegate: AnyTextSelectionDelegate) {
        TextSelectionHandleControls::handle_cut(self, app, delegate, None);
    }

    #[allow(deprecated)]
    fn handle_copy(&self, app: &mut App, delegate: AnyTextSelectionDelegate) {
        TextSelectionHandleControls::handle_copy(self, app, delegate, None);
    }

    #[allow(deprecated)]
    fn handle_paste(&self, app: &mut App, delegate: AnyTextSelectionDelegate) -> Task<()> {
        TextSelectionHandleControls::handle_paste(self, app, delegate)
    }

    #[allow(deprecated)]
    fn handle_select_all(&self, app: &mut App, delegate: AnyTextSelectionDelegate) {
        TextSelectionHandleControls::handle_select_all(self, app, delegate);
    }
}

impl TextSelectionHandleControls for DesktopTextSelectionHandleControls {}

// TODO(justinmc): Deprecate this after TextSelectionControls.buildToolbar is
// deleted, when users should migrate back to desktopTextSelectionControls.
// See https://github.com/flutter/flutter/pull/124262
/// Desktop text selection handle controls that loosely follow Material design
/// conventions.
///
/// Dart's global: every call hands out the same controls, so an `EditableText` given them
/// again sees them unchanged.
pub fn desktop_text_selection_handle_controls() -> Rc<dyn TextSelectionControls> {
    thread_local! {
        static CONTROLS: Rc<dyn TextSelectionControls> = Rc::new(DesktopTextSelectionHandleControls);
    }
    CONTROLS.with(Rc::clone)
}

/// Desktop text selection controls that loosely follow Material design
/// conventions.
///
/// Dart's global: every call hands out the same controls, so an `EditableText` given them
/// again sees them unchanged.
pub fn desktop_text_selection_controls() -> Rc<dyn TextSelectionControls> {
    thread_local! {
        static CONTROLS: Rc<dyn TextSelectionControls> = Rc::new(DesktopTextSelectionControls);
    }
    CONTROLS.with(Rc::clone)
}

// Generates the child that's passed into DesktopTextSelectionToolbar.
struct DesktopTextSelectionControlsToolbar {
    key: Option<KeyRef>,
    clipboard_status: Option<Ref<ClipboardStatusNotifier>>,
    #[allow(dead_code)]
    endpoints: Vec<TextSelectionPoint>,
    global_editable_region: Rect,
    handle_copy: Option<Listener>,
    handle_cut: Option<Listener>,
    handle_paste: Option<Listener>,
    handle_select_all: Option<Listener>,
    last_secondary_tap_down_position: Option<Offset>,
    selection_midpoint: Offset,
    #[allow(dead_code)]
    text_line_height: f64,
}

impl Debug for DesktopTextSelectionControlsToolbar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_DesktopTextSelectionControlsToolbar")
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for DesktopTextSelectionControlsToolbar {
    type State = DesktopTextSelectionControlsToolbarState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> DesktopTextSelectionControlsToolbarState {
        DesktopTextSelectionControlsToolbarState {
            state: StateData::new(),
        }
    }
}

struct DesktopTextSelectionControlsToolbarState {
    state: StateData<DesktopTextSelectionControlsToolbar>,
}

impl DesktopTextSelectionControlsToolbarState {
    fn on_changed_clipboard_status(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_state| {
            // Inform the widget that the value of clipboardStatus has changed.
        });
    }
}

impl State for DesktopTextSelectionControlsToolbarState {
    type Widget = DesktopTextSelectionControlsToolbar;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        if let Some(clipboard_status) = self.widget(app).clipboard_status.clone() {
            clipboard_status.add_listener(
                app,
                Listener::handle_method(self, Self::on_changed_clipboard_status),
            );
        }
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &DesktopTextSelectionControlsToolbar,
    ) {
        if old_widget.clipboard_status != self.widget(app).clipboard_status {
            if let Some(clipboard_status) = old_widget.clipboard_status.clone() {
                clipboard_status.remove_listener(
                    app,
                    &Listener::handle_method(self, Self::on_changed_clipboard_status),
                );
            }
            if let Some(clipboard_status) = self.widget(app).clipboard_status.clone() {
                clipboard_status.add_listener(
                    app,
                    Listener::handle_method(self, Self::on_changed_clipboard_status),
                );
            }
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(clipboard_status) = self.widget(app).clipboard_status.clone() {
            clipboard_status.remove_listener(
                app,
                &Listener::handle_method(self, Self::on_changed_clipboard_status),
            );
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material_localizations(app, context));
        debug_assert!(debug_check_has_media_query(app, context));

        let widget = self.widget(app);
        let handle_cut = widget.handle_cut.clone();
        let handle_copy = widget.handle_copy.clone();
        let handle_paste = widget.handle_paste.clone();
        let handle_select_all = widget.handle_select_all.clone();
        let clipboard_status = widget.clipboard_status.clone();
        let selection_midpoint = widget.selection_midpoint;
        let global_editable_region = widget.global_editable_region;
        let last_secondary_tap_down_position = widget.last_secondary_tap_down_position;

        // Don't render the menu until the state of the clipboard is known.
        if handle_paste.is_some()
            && clipboard_status
                .as_ref()
                .is_some_and(|status| status.value(app) == ClipboardStatus::Unknown)
        {
            return SizedBox::shrink().into_widget();
        }

        let media_query_padding = MediaQuery::padding_of(app, context);
        let midpoint_anchor = Offset::new(
            clamp_double(
                selection_midpoint.dx() - global_editable_region.left,
                media_query_padding.left,
                MediaQuery::width_of(app, context) - media_query_padding.right,
            ),
            selection_midpoint.dy() - global_editable_region.top,
        );

        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let mut items: Vec<WidgetRef> = Vec::new();

        let mut add_toolbar_button = |app: &mut App, text: String, on_pressed: Listener| {
            items.push(
                DesktopTextSelectionToolbarButton::text(app, context, Some(on_pressed), text)
                    .into_widget(),
            );
        };

        if let Some(handle_cut) = handle_cut {
            add_toolbar_button(app, localizations.cut_button_label(), handle_cut);
        }
        if let Some(handle_copy) = handle_copy {
            add_toolbar_button(app, localizations.copy_button_label(), handle_copy);
        }
        if let Some(handle_paste) = handle_paste
            && clipboard_status
                .as_ref()
                .is_some_and(|status| status.value(app) == ClipboardStatus::Pasteable)
        {
            add_toolbar_button(app, localizations.paste_button_label(), handle_paste);
        }
        if let Some(handle_select_all) = handle_select_all {
            add_toolbar_button(
                app,
                localizations.select_all_button_label(),
                handle_select_all,
            );
        }

        // If there is no option available, build an empty widget.
        if items.is_empty() {
            return SizedBox::shrink().into_widget();
        }

        DesktopTextSelectionToolbar::new(
            last_secondary_tap_down_position.unwrap_or(midpoint_anchor),
            items,
        )
        .into_widget()
    }
}
