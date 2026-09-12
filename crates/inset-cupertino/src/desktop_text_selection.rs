//! Flutter counterpart: `cupertino/desktop_text_selection.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;

use inset_embedder::{Offset, Rect, Size, clamp_double};
use inset_foundation::{App, Handle, Listenable, Listener};
use inset_rendering::TextSelectionPoint;
use inset_services::{AnyTextSelectionDelegate, SelectionChangedCause};
use inset_widgets::{
    BuildContext, ClipboardStatus, ClipboardStatusNotifier, IntoWidget, KeyRef, MediaQuery,
    SizedBox, State, StateData, StatefulWidget, TextSelectionControls, TextSelectionHandleControls,
    TextSelectionHandleType, WidgetRef,
};

use crate::desktop_text_selection_toolbar::CupertinoDesktopTextSelectionToolbar;
use crate::desktop_text_selection_toolbar_button::CupertinoDesktopTextSelectionToolbarButton;
use crate::localizations::CupertinoLocalizations;

/// MacOS Cupertino styled text selection handle controls.
///
/// Specifically does not manage the toolbar, which is left to `EditableText.contextMenuBuilder`.
#[derive(Debug)]
struct CupertinoDesktopTextSelectionHandleControls;

/// Desktop Cupertino styled text selection controls.
///
/// [`cupertino_desktop_text_selection_controls`] has a suitable instance of this class.
#[derive(Debug)]
pub struct CupertinoDesktopTextSelectionControls;

impl CupertinoDesktopTextSelectionControls {
    /// Creates desktop Cupertino styled text selection controls.
    pub const fn new() -> CupertinoDesktopTextSelectionControls {
        CupertinoDesktopTextSelectionControls
    }
}

impl Default for CupertinoDesktopTextSelectionControls {
    fn default() -> CupertinoDesktopTextSelectionControls {
        CupertinoDesktopTextSelectionControls
    }
}

impl TextSelectionControls for CupertinoDesktopTextSelectionControls {
    fn get_handle_size(&self, _text_line_height: f64) -> Size {
        Size::ZERO
    }

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
        clipboard_status: Option<Handle<ClipboardStatusNotifier>>,
        last_secondary_tap_down_position: Option<Offset>,
    ) -> WidgetRef {
        let handle_cut = self.can_cut(app, delegate).then(|| {
            Listener::new(move |app| {
                CupertinoDesktopTextSelectionControls.handle_cut(app, delegate);
            })
        });
        let handle_copy = self.can_copy(app, delegate).then(|| {
            Listener::new(move |app| {
                CupertinoDesktopTextSelectionControls.handle_copy(app, delegate);
            })
        });
        let handle_paste = self.can_paste(app, delegate).then(|| {
            Listener::new(move |app| {
                CupertinoDesktopTextSelectionControls.handle_paste(app, delegate);
            })
        });
        let handle_select_all = self.can_select_all(app, delegate).then(|| {
            Listener::new(move |app| {
                CupertinoDesktopTextSelectionControls.handle_select_all(app, delegate);
            })
        });
        CupertinoDesktopTextSelectionControlsToolbar {
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

    fn get_handle_anchor(
        &self,
        _handle_type: TextSelectionHandleType,
        _text_line_height: f64,
    ) -> Offset {
        Offset::ZERO
    }

    #[allow(deprecated)]
    fn handle_select_all(&self, app: &mut App, delegate: AnyTextSelectionDelegate) {
        delegate.select_all(app, SelectionChangedCause::Toolbar);
        delegate.hide_toolbar(app, true);
    }
}

impl TextSelectionControls for CupertinoDesktopTextSelectionHandleControls {
    fn get_handle_size(&self, text_line_height: f64) -> Size {
        CupertinoDesktopTextSelectionControls.get_handle_size(text_line_height)
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
        clipboard_status: Option<Handle<ClipboardStatusNotifier>>,
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
        CupertinoDesktopTextSelectionControls.build_handle(
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
        CupertinoDesktopTextSelectionControls.get_handle_anchor(handle_type, text_line_height)
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
    fn handle_paste(&self, app: &mut App, delegate: AnyTextSelectionDelegate) {
        TextSelectionHandleControls::handle_paste(self, app, delegate);
    }

    #[allow(deprecated)]
    fn handle_select_all(&self, app: &mut App, delegate: AnyTextSelectionDelegate) {
        TextSelectionHandleControls::handle_select_all(self, app, delegate);
    }
}

impl TextSelectionHandleControls for CupertinoDesktopTextSelectionHandleControls {}

/// Text selection handle controls that follow MacOS design conventions.
pub fn cupertino_desktop_text_selection_handle_controls() -> Rc<dyn TextSelectionControls> {
    Rc::new(CupertinoDesktopTextSelectionHandleControls)
}

/// Text selection controls that follow MacOS design conventions.
pub fn cupertino_desktop_text_selection_controls() -> Rc<dyn TextSelectionControls> {
    Rc::new(CupertinoDesktopTextSelectionControls)
}

#[allow(dead_code)]
struct CupertinoDesktopTextSelectionControlsToolbar {
    key: Option<KeyRef>,
    clipboard_status: Option<Handle<ClipboardStatusNotifier>>,
    endpoints: Vec<TextSelectionPoint>,
    global_editable_region: Rect,
    handle_copy: Option<Listener>,
    handle_cut: Option<Listener>,
    handle_paste: Option<Listener>,
    handle_select_all: Option<Listener>,
    selection_midpoint: Offset,
    last_secondary_tap_down_position: Option<Offset>,
    text_line_height: f64,
}

impl Debug for CupertinoDesktopTextSelectionControlsToolbar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoDesktopTextSelectionControlsToolbar")
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CupertinoDesktopTextSelectionControlsToolbar {
    type State = CupertinoDesktopTextSelectionControlsToolbarState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoDesktopTextSelectionControlsToolbarState {
        CupertinoDesktopTextSelectionControlsToolbarState {
            state: StateData::new(),
        }
    }
}

struct CupertinoDesktopTextSelectionControlsToolbarState {
    state: StateData<CupertinoDesktopTextSelectionControlsToolbar>,
}

impl CupertinoDesktopTextSelectionControlsToolbarState {
    fn on_changed_clipboard_status(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_state| {});
    }
}

impl State for CupertinoDesktopTextSelectionControlsToolbarState {
    type Widget = CupertinoDesktopTextSelectionControlsToolbar;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        if let Some(clipboard_status) = self.widget(app).clipboard_status {
            clipboard_status.add_listener(
                app,
                Listener::handle_method(self, Self::on_changed_clipboard_status),
            );
        }
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &CupertinoDesktopTextSelectionControlsToolbar,
    ) {
        if old_widget.clipboard_status != self.widget(app).clipboard_status {
            if let Some(clipboard_status) = old_widget.clipboard_status {
                clipboard_status.remove_listener(
                    app,
                    &Listener::handle_method(self, Self::on_changed_clipboard_status),
                );
            }
            if let Some(clipboard_status) = self.widget(app).clipboard_status {
                clipboard_status.add_listener(
                    app,
                    Listener::handle_method(self, Self::on_changed_clipboard_status),
                );
            }
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(clipboard_status) = self.widget(app).clipboard_status {
            clipboard_status.remove_listener(
                app,
                &Listener::handle_method(self, Self::on_changed_clipboard_status),
            );
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let handle_paste = self.widget(app).handle_paste.clone();
        let clipboard_status = self.widget(app).clipboard_status;
        if handle_paste.is_some()
            && clipboard_status.is_some_and(|status| status.value(app) == ClipboardStatus::Unknown)
        {
            return SizedBox::shrink().into_widget();
        }

        let media_query_padding = MediaQuery::padding_of(app, context);
        let selection_midpoint = self.widget(app).selection_midpoint;
        let global_editable_region = self.widget(app).global_editable_region;
        let midpoint_anchor = Offset::new(
            clamp_double(
                selection_midpoint.dx() - global_editable_region.left,
                media_query_padding.left,
                MediaQuery::width_of(app, context) - media_query_padding.right,
            ),
            selection_midpoint.dy() - global_editable_region.top,
        );

        let localizations = <dyn CupertinoLocalizations>::of(app, context);
        let pixel = 1.0 / MediaQuery::device_pixel_ratio_of(app, context);
        let mut items: Vec<WidgetRef> = Vec::new();

        let mut add_toolbar_button = |text: String, on_pressed: Listener| {
            if !items.is_empty() {
                items.push(SizedBox::new().width(pixel).into_widget());
            }
            items.push(
                CupertinoDesktopTextSelectionToolbarButton::text(Some(on_pressed), text)
                    .into_widget(),
            );
        };

        if let Some(handle_cut) = self.widget(app).handle_cut.clone() {
            add_toolbar_button(localizations.cut_button_label(), handle_cut);
        }
        if let Some(handle_copy) = self.widget(app).handle_copy.clone() {
            add_toolbar_button(localizations.copy_button_label(), handle_copy);
        }
        if let Some(handle_paste) = self.widget(app).handle_paste.clone()
            && clipboard_status
                .is_some_and(|status| status.value(app) == ClipboardStatus::Pasteable)
        {
            add_toolbar_button(localizations.paste_button_label(), handle_paste);
        }
        if let Some(handle_select_all) = self.widget(app).handle_select_all.clone() {
            add_toolbar_button(localizations.select_all_button_label(), handle_select_all);
        }

        if items.is_empty() {
            return SizedBox::shrink().into_widget();
        }

        let anchor = self
            .widget(app)
            .last_secondary_tap_down_position
            .unwrap_or(midpoint_anchor);
        CupertinoDesktopTextSelectionToolbar::new(anchor, items).into_widget()
    }
}
