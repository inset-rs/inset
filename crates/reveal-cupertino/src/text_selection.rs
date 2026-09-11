//! Flutter counterpart: `cupertino/text_selection.dart`.

use std::any::Any;
use std::f64::consts::PI;
use std::fmt::{self, Debug};
use std::rc::Rc;

use reveal_embedder::{
    Canvas, Color, FillRule, Matrix4, Offset, Paint, PathBuilder, Rect, Size, clamp_double,
};
use reveal_foundation::{App, Handle, Listenable, Listener};
use reveal_rendering::{CustomPainter, TextSelectionPoint};
use reveal_services::AnyTextSelectionDelegate;
use reveal_widgets::{
    BuildContext, ClipboardStatus, ClipboardStatusNotifier, CustomPaint, IntoWidget, KeyRef,
    MediaQuery, SizedBox, State, StateData, StatefulWidget, TextSelectionControls,
    TextSelectionHandleControls, TextSelectionHandleType, Transform, WidgetRef,
};

use crate::localizations::CupertinoLocalizations;
use crate::text_selection_toolbar::CupertinoTextSelectionToolbar;
use crate::text_selection_toolbar_button::CupertinoTextSelectionToolbarButton;
use crate::theme::CupertinoTheme;

const K_SELECTION_HANDLE_OVERLAP: f64 = 1.5;
const K_SELECTION_HANDLE_RADIUS: f64 = 6.0;
const K_ARROW_SCREEN_PADDING: f64 = 26.0;

struct CupertinoTextSelectionHandlePainter {
    color: Color,
}

impl CustomPainter for CupertinoTextSelectionHandlePainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        const HALF_STROKE_WIDTH: f64 = 1.0;
        let paint = Paint {
            color: self.color.into(),
            ..Paint::default()
        };
        let circle = Rect::from_circle(
            Offset::new(K_SELECTION_HANDLE_RADIUS, K_SELECTION_HANDLE_RADIUS),
            K_SELECTION_HANDLE_RADIUS,
        );
        let line = Rect::from_points(
            Offset::new(
                K_SELECTION_HANDLE_RADIUS - HALF_STROKE_WIDTH,
                2.0 * K_SELECTION_HANDLE_RADIUS - K_SELECTION_HANDLE_OVERLAP,
            ),
            Offset::new(K_SELECTION_HANDLE_RADIUS + HALF_STROKE_WIDTH, size.height()),
        );
        let radius = K_SELECTION_HANDLE_RADIUS as f32;
        let mut path = PathBuilder::new();
        path.rrect_radii_elliptical(circle, [[radius, radius]; 4]);
        path.rect(line.into());
        canvas.draw_path(&path.build(), FillRule::NonZero, &paint);
    }

    fn should_repaint(&self, _app: &App, old_painter: &dyn CustomPainter) -> bool {
        old_painter
            .as_any()
            .downcast_ref::<CupertinoTextSelectionHandlePainter>()
            .is_none_or(|old| old.color != self.color)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// iOS Cupertino styled text selection handle controls.
///
/// Specifically does not manage the toolbar, which is left to `EditableText.contextMenuBuilder`.
#[derive(Debug)]
struct CupertinoTextSelectionHandleControls;

/// iOS Cupertino styled text selection controls.
///
/// [`cupertino_text_selection_controls`] has a suitable instance of this class.
#[derive(Debug)]
pub struct CupertinoTextSelectionControls;

impl CupertinoTextSelectionControls {
    /// Creates iOS Cupertino styled text selection controls.
    pub const fn new() -> CupertinoTextSelectionControls {
        CupertinoTextSelectionControls
    }
}

impl Default for CupertinoTextSelectionControls {
    fn default() -> CupertinoTextSelectionControls {
        CupertinoTextSelectionControls
    }
}

impl TextSelectionControls for CupertinoTextSelectionControls {
    fn get_handle_size(&self, text_line_height: f64) -> Size {
        Size::new(
            K_SELECTION_HANDLE_RADIUS * 2.0,
            text_line_height + K_SELECTION_HANDLE_RADIUS * 2.0 - K_SELECTION_HANDLE_OVERLAP,
        )
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
        _last_secondary_tap_down_position: Option<Offset>,
    ) -> WidgetRef {
        let handle_cut = self.can_cut(app, delegate).then(|| {
            Listener::new(move |app| {
                CupertinoTextSelectionControls.handle_cut(app, delegate);
            })
        });
        let handle_copy = self.can_copy(app, delegate).then(|| {
            Listener::new(move |app| {
                CupertinoTextSelectionControls.handle_copy(app, delegate);
            })
        });
        let handle_paste = self.can_paste(app, delegate).then(|| {
            Listener::new(move |app| {
                CupertinoTextSelectionControls.handle_paste(app, delegate);
            })
        });
        let handle_select_all = self.can_select_all(app, delegate).then(|| {
            Listener::new(move |app| {
                CupertinoTextSelectionControls.handle_select_all(app, delegate);
            })
        });
        CupertinoTextSelectionControlsToolbar {
            key: None,
            clipboard_status,
            endpoints: endpoints.to_vec(),
            global_editable_region,
            handle_cut,
            handle_copy,
            handle_paste,
            handle_select_all,
            selection_midpoint,
            text_line_height,
        }
        .into_widget()
    }

    fn build_handle(
        &self,
        app: &mut App,
        context: BuildContext,
        handle_type: TextSelectionHandleType,
        text_line_height: f64,
        _on_tap: Option<Listener>,
    ) -> WidgetRef {
        let custom_paint = CustomPaint::new()
            .painter(CupertinoTextSelectionHandlePainter {
                color: CupertinoTheme::of(app, context)
                    .selection_handle_color()
                    .color(),
            })
            .into_widget();
        match handle_type {
            TextSelectionHandleType::Left => {
                let desired_size = self.get_handle_size(text_line_height);
                SizedBox::from_size(Some(desired_size))
                    .child(custom_paint)
                    .into_widget()
            }
            TextSelectionHandleType::Right => {
                let desired_size = self.get_handle_size(text_line_height);
                let handle = SizedBox::from_size(Some(desired_size))
                    .child(custom_paint)
                    .into_widget();
                let half_width = desired_size.width() as f32 / 2.0;
                let half_height = desired_size.height() as f32 / 2.0;
                Transform::new(
                    Matrix4::translation(half_width, half_height).then(
                        &Matrix4::rotation(PI as f32)
                            .then(&Matrix4::translation(-half_width, -half_height)),
                    ),
                )
                .child(handle)
                .into_widget()
            }
            TextSelectionHandleType::Collapsed => {
                SizedBox::from_size(Some(self.get_handle_size(text_line_height))).into_widget()
            }
        }
    }

    fn get_handle_anchor(
        &self,
        handle_type: TextSelectionHandleType,
        text_line_height: f64,
    ) -> Offset {
        let handle_size = self.get_handle_size(text_line_height);
        match handle_type {
            TextSelectionHandleType::Left => {
                Offset::new(handle_size.width() / 2.0, handle_size.height())
            }
            TextSelectionHandleType::Right => Offset::new(
                handle_size.width() / 2.0,
                handle_size.height() - 2.0 * K_SELECTION_HANDLE_RADIUS + K_SELECTION_HANDLE_OVERLAP,
            ),
            TextSelectionHandleType::Collapsed => Offset::new(
                handle_size.width() / 2.0,
                text_line_height + (handle_size.height() - text_line_height) / 2.0,
            ),
        }
    }
}

impl TextSelectionControls for CupertinoTextSelectionHandleControls {
    fn get_handle_size(&self, text_line_height: f64) -> Size {
        CupertinoTextSelectionControls.get_handle_size(text_line_height)
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
        CupertinoTextSelectionControls.build_handle(
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
        CupertinoTextSelectionControls.get_handle_anchor(handle_type, text_line_height)
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

impl TextSelectionHandleControls for CupertinoTextSelectionHandleControls {}

/// Text selection handle controls that follow iOS design conventions.
pub fn cupertino_text_selection_handle_controls() -> Rc<dyn TextSelectionControls> {
    Rc::new(CupertinoTextSelectionHandleControls)
}

/// Text selection controls that follow iOS design conventions.
pub fn cupertino_text_selection_controls() -> Rc<dyn TextSelectionControls> {
    Rc::new(CupertinoTextSelectionControls)
}

struct CupertinoTextSelectionControlsToolbar {
    key: Option<KeyRef>,
    clipboard_status: Option<Handle<ClipboardStatusNotifier>>,
    endpoints: Vec<TextSelectionPoint>,
    global_editable_region: Rect,
    handle_copy: Option<Listener>,
    handle_cut: Option<Listener>,
    handle_paste: Option<Listener>,
    handle_select_all: Option<Listener>,
    selection_midpoint: Offset,
    text_line_height: f64,
}

impl Debug for CupertinoTextSelectionControlsToolbar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_CupertinoTextSelectionControlsToolbar")
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CupertinoTextSelectionControlsToolbar {
    type State = CupertinoTextSelectionControlsToolbarState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoTextSelectionControlsToolbarState {
        CupertinoTextSelectionControlsToolbarState {
            state: StateData::new(),
        }
    }
}

struct CupertinoTextSelectionControlsToolbarState {
    state: StateData<CupertinoTextSelectionControlsToolbar>,
}

impl CupertinoTextSelectionControlsToolbarState {
    fn on_changed_clipboard_status(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_state| {});
    }
}

impl State for CupertinoTextSelectionControlsToolbarState {
    type Widget = CupertinoTextSelectionControlsToolbar;
    reveal_widgets::state_accessors!();

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
        old_widget: &CupertinoTextSelectionControlsToolbar,
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
        let endpoints = self.widget(app).endpoints.clone();
        let text_line_height = self.widget(app).text_line_height;
        let anchor_x = clamp_double(
            selection_midpoint.dx() + global_editable_region.left,
            K_ARROW_SCREEN_PADDING + media_query_padding.left,
            MediaQuery::width_of(app, context) - media_query_padding.right - K_ARROW_SCREEN_PADDING,
        );
        let top_amount_in_editable_region = endpoints[0].point.dy() - text_line_height;
        let anchor_top = top_amount_in_editable_region.max(0.0) + global_editable_region.top;
        let anchor_above = Offset::new(anchor_x, anchor_top);
        let anchor_below = Offset::new(
            anchor_x,
            endpoints.last().expect("endpoints").point.dy() + global_editable_region.top,
        );

        let localizations = <dyn CupertinoLocalizations>::of(app, context);
        let pixel = 1.0 / MediaQuery::device_pixel_ratio_of(app, context);
        let mut items: Vec<WidgetRef> = Vec::new();
        let mut add_toolbar_button = |text: String, on_pressed: Listener| {
            if !items.is_empty() {
                items.push(SizedBox::new().width(pixel).into_widget());
            }
            items.push(
                CupertinoTextSelectionToolbarButton::text(Some(on_pressed), text).into_widget(),
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
        CupertinoTextSelectionToolbar::new(anchor_above, anchor_below, items).into_widget()
    }
}
