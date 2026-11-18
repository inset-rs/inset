//! Flutter counterpart: material_ui's `text_selection.dart`.

use std::any::Any;
use std::f64::consts::PI;
use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::{Canvas, Color, FillRule, Offset, Paint, PathBuilder, Rect, Size};
use waft_foundation::{App, Handle, Listenable, Listener, Ref, Task};
use waft_painting::AlignmentGeometry;
use waft_rendering::{CustomPainter, HitTestBehavior, TextSelectionPoint};
use waft_services::AnyTextSelectionDelegate;
use waft_widgets::{
    BuildContext, ClipboardStatus, ClipboardStatusNotifier, CustomPaint, GestureDetector,
    IntoWidget, KeyRef, SizedBox, State, StateData, StatefulWidget, Text, TextSelectionControls,
    TextSelectionHandleControls, TextSelectionHandleType, Transform, WidgetRef,
};

use crate::debug::debug_check_has_material_localizations;
use crate::material_localizations::MaterialLocalizations;
use crate::text_selection_theme::TextSelectionTheme;
use crate::text_selection_toolbar::TextSelectionToolbar;
use crate::text_selection_toolbar_text_button::TextSelectionToolbarTextButton;
use crate::theme::Theme;

const K_HANDLE_SIZE: f64 = 22.0;

// Padding between the toolbar and the anchor.
const K_TOOLBAR_CONTENT_DISTANCE_BELOW: f64 = K_HANDLE_SIZE - 2.0;
const K_TOOLBAR_CONTENT_DISTANCE: f64 = 8.0;

/// Android Material styled text selection handle controls.
///
/// Specifically does not manage the toolbar, which is left to
/// `EditableText.contextMenuBuilder`.
#[deprecated(note = "Use `MaterialTextSelectionControls`. \
            This feature was deprecated after v3.3.0-0.5.pre.")]
#[derive(Debug, Default)]
pub struct MaterialTextSelectionHandleControls;

/// Android Material styled text selection controls.
///
/// [`material_text_selection_controls`] has a suitable instance of this class.
#[derive(Debug, Default)]
pub struct MaterialTextSelectionControls;

impl MaterialTextSelectionControls {
    /// Creates Android Material styled text selection controls.
    pub const fn new() -> MaterialTextSelectionControls {
        MaterialTextSelectionControls
    }
}

impl TextSelectionControls for MaterialTextSelectionControls {
    /// Returns the size of the Material handle.
    fn get_handle_size(&self, _text_line_height: f64) -> Size {
        Size::new(K_HANDLE_SIZE, K_HANDLE_SIZE)
    }

    /// Builder for material-style copy/paste text selection toolbar.
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
        _last_secondary_tap_down_position: Option<Offset>,
    ) -> WidgetRef {
        let handle_cut = self.can_cut(app, delegate).then(|| {
            Listener::new(move |app| {
                MaterialTextSelectionControls.handle_cut(app, delegate);
            })
        });
        let handle_copy = self.can_copy(app, delegate).then(|| {
            Listener::new(move |app| {
                MaterialTextSelectionControls.handle_copy(app, delegate);
            })
        });
        let handle_paste = self.can_paste(app, delegate).then(|| {
            Listener::new(move |app| {
                MaterialTextSelectionControls.handle_paste(app, delegate);
            })
        });
        let handle_select_all = self.can_select_all(app, delegate).then(|| {
            Listener::new(move |app| {
                MaterialTextSelectionControls.handle_select_all(app, delegate);
            })
        });
        TextSelectionControlsToolbar {
            key: None,
            global_editable_region,
            text_line_height,
            selection_midpoint,
            endpoints: endpoints.to_vec(),
            clipboard_status,
            handle_cut,
            handle_copy,
            handle_paste,
            handle_select_all,
        }
        .into_widget()
    }

    /// Builder for material-style text selection handles.
    fn build_handle(
        &self,
        app: &mut App,
        context: BuildContext,
        handle_type: TextSelectionHandleType,
        _text_line_height: f64,
        on_tap: Option<Listener>,
    ) -> WidgetRef {
        let theme = Theme::of(app, context);
        let handle_color = TextSelectionTheme::of(app, context)
            .selection_handle_color
            .unwrap_or(theme.color_scheme.primary);
        let mut gesture_detector = GestureDetector::new().behavior(HitTestBehavior::Translucent);
        if let Some(on_tap) = on_tap {
            gesture_detector = gesture_detector.on_tap(on_tap);
        }
        let handle = SizedBox::square(Some(K_HANDLE_SIZE))
            .child(
                CustomPaint::new()
                    .painter(TextSelectionHandlePainter {
                        color: handle_color,
                    })
                    .child(gesture_detector),
            )
            .into_widget();

        // [handle] is a circle, with a rectangle in the top left quadrant of that
        // circle (an onion pointing to 10:30). We rotate [handle] to point
        // straight up or up-right depending on the handle type.
        match handle_type {
            // points up-right
            TextSelectionHandleType::Left => {
                Transform::rotate(PI / 2.0).child(handle).into_widget()
            }
            // points up-left
            TextSelectionHandleType::Right => handle,
            // points up
            TextSelectionHandleType::Collapsed => {
                Transform::rotate(PI / 4.0).child(handle).into_widget()
            }
        }
    }

    /// Gets anchor for material-style text selection handles.
    ///
    /// See `TextSelectionControls::get_handle_anchor`.
    fn get_handle_anchor(
        &self,
        handle_type: TextSelectionHandleType,
        _text_line_height: f64,
    ) -> Offset {
        match handle_type {
            TextSelectionHandleType::Collapsed => Offset::new(K_HANDLE_SIZE / 2.0, -4.0),
            TextSelectionHandleType::Left => Offset::new(K_HANDLE_SIZE, 0.0),
            TextSelectionHandleType::Right => Offset::ZERO,
        }
    }

    #[allow(deprecated)]
    fn can_select_all(&self, app: &App, delegate: AnyTextSelectionDelegate) -> bool {
        // Android allows SelectAll when selection is not collapsed, unless
        // everything has already been selected.
        let value = delegate.text_editing_value(app);
        delegate.select_all_enabled(app)
            && !value.text.is_empty()
            && !(value.selection.start() == 0
                && value.selection.end() == value.text.encode_utf16().count() as i32)
    }
}

#[allow(deprecated)]
impl TextSelectionControls for MaterialTextSelectionHandleControls {
    fn as_text_selection_handle_controls(&self) -> Option<&dyn TextSelectionHandleControls> {
        Some(self)
    }

    fn get_handle_size(&self, text_line_height: f64) -> Size {
        MaterialTextSelectionControls.get_handle_size(text_line_height)
    }

    #[allow(clippy::too_many_arguments)]
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
        MaterialTextSelectionControls.build_handle(
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
        MaterialTextSelectionControls.get_handle_anchor(handle_type, text_line_height)
    }

    fn can_cut(&self, app: &App, delegate: AnyTextSelectionDelegate) -> bool {
        TextSelectionHandleControls::can_cut(self, app, delegate)
    }

    fn can_copy(&self, app: &App, delegate: AnyTextSelectionDelegate) -> bool {
        TextSelectionHandleControls::can_copy(self, app, delegate)
    }

    fn can_paste(&self, app: &App, delegate: AnyTextSelectionDelegate) -> bool {
        TextSelectionHandleControls::can_paste(self, app, delegate)
    }

    fn can_select_all(&self, app: &App, delegate: AnyTextSelectionDelegate) -> bool {
        TextSelectionHandleControls::can_select_all(self, app, delegate)
    }

    fn handle_cut(&self, app: &mut App, delegate: AnyTextSelectionDelegate) {
        TextSelectionHandleControls::handle_cut(self, app, delegate, None);
    }

    fn handle_copy(&self, app: &mut App, delegate: AnyTextSelectionDelegate) {
        TextSelectionHandleControls::handle_copy(self, app, delegate, None);
    }

    fn handle_paste(&self, app: &mut App, delegate: AnyTextSelectionDelegate) -> Task<()> {
        TextSelectionHandleControls::handle_paste(self, app, delegate)
    }

    fn handle_select_all(&self, app: &mut App, delegate: AnyTextSelectionDelegate) {
        TextSelectionHandleControls::handle_select_all(self, app, delegate);
    }
}

#[allow(deprecated)]
impl TextSelectionHandleControls for MaterialTextSelectionHandleControls {}

// The label and callback for the available default text selection menu buttons.
struct TextSelectionToolbarItemData {
    label: String,
    on_pressed: Listener,
}

// The highest level toolbar widget, built directly by buildToolbar.
struct TextSelectionControlsToolbar {
    key: Option<KeyRef>,
    clipboard_status: Option<Ref<ClipboardStatusNotifier>>,
    endpoints: Vec<TextSelectionPoint>,
    global_editable_region: Rect,
    handle_cut: Option<Listener>,
    handle_copy: Option<Listener>,
    handle_paste: Option<Listener>,
    handle_select_all: Option<Listener>,
    selection_midpoint: Offset,
    text_line_height: f64,
}

impl Debug for TextSelectionControlsToolbar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_TextSelectionControlsToolbar")
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for TextSelectionControlsToolbar {
    type State = TextSelectionControlsToolbarState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> TextSelectionControlsToolbarState {
        TextSelectionControlsToolbarState {
            state: StateData::new(),
        }
    }
}

struct TextSelectionControlsToolbarState {
    state: StateData<TextSelectionControlsToolbar>,
}

impl TextSelectionControlsToolbarState {
    fn on_changed_clipboard_status(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_state| {
            // Inform the widget that the value of clipboardStatus has changed.
        });
    }
}

impl State for TextSelectionControlsToolbarState {
    type Widget = TextSelectionControlsToolbar;
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
        old_widget: &TextSelectionControlsToolbar,
    ) {
        if self.widget(app).clipboard_status != old_widget.clipboard_status {
            if let Some(clipboard_status) = self.widget(app).clipboard_status.clone() {
                clipboard_status.add_listener(
                    app,
                    Listener::handle_method(self, Self::on_changed_clipboard_status),
                );
            }
            if let Some(clipboard_status) = old_widget.clipboard_status.clone() {
                clipboard_status.remove_listener(
                    app,
                    &Listener::handle_method(self, Self::on_changed_clipboard_status),
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
        let widget = self.widget(app);
        let handle_cut = widget.handle_cut.clone();
        let handle_copy = widget.handle_copy.clone();
        let handle_paste = widget.handle_paste.clone();
        let handle_select_all = widget.handle_select_all.clone();
        let clipboard_status = widget.clipboard_status.clone();
        let endpoints = widget.endpoints.clone();
        let text_line_height = widget.text_line_height;
        let global_editable_region = widget.global_editable_region;
        let selection_midpoint = widget.selection_midpoint;

        // If there are no buttons to be shown, don't render anything.
        if handle_cut.is_none()
            && handle_copy.is_none()
            && handle_paste.is_none()
            && handle_select_all.is_none()
        {
            return SizedBox::shrink().into_widget();
        }
        // If the paste button is desired, don't render anything until the state of
        // the clipboard is known, since it's used to determine if paste is shown.
        if handle_paste.is_some()
            && clipboard_status
                .as_ref()
                .is_some_and(|status| status.value(app) == ClipboardStatus::Unknown)
        {
            return SizedBox::shrink().into_widget();
        }

        // Calculate the positioning of the menu. It is placed above the selection
        // if there is enough room, or otherwise below.
        let start_text_selection_point = &endpoints[0];
        let end_text_selection_point = if endpoints.len() > 1 {
            &endpoints[1]
        } else {
            &endpoints[0]
        };
        let top_amount_in_editable_region =
            start_text_selection_point.point.dy() - text_line_height;
        let anchor_top = top_amount_in_editable_region.max(0.0) + global_editable_region.top
            - K_TOOLBAR_CONTENT_DISTANCE;

        let anchor_above = Offset::new(
            global_editable_region.left + selection_midpoint.dx(),
            anchor_top,
        );
        let anchor_below = Offset::new(
            global_editable_region.left + selection_midpoint.dx(),
            global_editable_region.top
                + end_text_selection_point.point.dy()
                + K_TOOLBAR_CONTENT_DISTANCE_BELOW,
        );

        // Determine which buttons will appear so that the order and total number is
        // known. A button's position in the menu can slightly affect its
        // appearance.
        debug_assert!(debug_check_has_material_localizations(app, context));
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let mut item_datas: Vec<TextSelectionToolbarItemData> = Vec::new();
        if let Some(handle_cut) = handle_cut {
            item_datas.push(TextSelectionToolbarItemData {
                label: localizations.cut_button_label(),
                on_pressed: handle_cut,
            });
        }
        if let Some(handle_copy) = handle_copy {
            item_datas.push(TextSelectionToolbarItemData {
                label: localizations.copy_button_label(),
                on_pressed: handle_copy,
            });
        }
        if let Some(handle_paste) = handle_paste
            && clipboard_status
                .as_ref()
                .is_some_and(|status| status.value(app) == ClipboardStatus::Pasteable)
        {
            item_datas.push(TextSelectionToolbarItemData {
                label: localizations.paste_button_label(),
                on_pressed: handle_paste,
            });
        }
        if let Some(handle_select_all) = handle_select_all {
            item_datas.push(TextSelectionToolbarItemData {
                label: localizations.select_all_button_label(),
                on_pressed: handle_select_all,
            });
        }

        // If there is no option available, build an empty widget.
        if item_datas.is_empty() {
            return SizedBox::shrink().into_widget();
        }

        let total = item_datas.len();
        TextSelectionToolbar::new(
            anchor_above,
            anchor_below,
            item_datas
                .into_iter()
                .enumerate()
                .map(|(index, item_data)| {
                    TextSelectionToolbarTextButton::new(
                        Text::new(item_data.label),
                        TextSelectionToolbarTextButton::get_padding(index, total),
                    )
                    .alignment(AlignmentGeometry::CENTER_START)
                    .on_pressed(item_data.on_pressed)
                    .into_widget()
                }),
        )
        .into_widget()
    }
}

/// Draws a single text selection handle which points up and to the left.
struct TextSelectionHandlePainter {
    color: Color,
}

impl CustomPainter for TextSelectionHandlePainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        let paint = Paint {
            color: self.color.into(),
            ..Paint::default()
        };
        let radius = size.width() / 2.0;
        let circle = Rect::from_circle(Offset::new(radius, radius), radius);
        let point = Rect::from_ltwh(0.0, 0.0, radius, radius);
        let mut path = PathBuilder::new();
        path.rrect_radii_elliptical(circle, [[radius as f32, radius as f32]; 4]);
        path.rect(point.into());
        canvas.draw_path(&path.build(), FillRule::NonZero, &paint);
    }

    fn should_repaint(&self, _app: &App, old_painter: &dyn CustomPainter) -> bool {
        old_painter
            .as_any()
            .downcast_ref::<TextSelectionHandlePainter>()
            .is_none_or(|old| self.color != old.color)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

// TODO(justinmc): Deprecate this after TextSelectionControls.buildToolbar is
// deleted, when users should migrate back to materialTextSelectionControls.
// See https://github.com/flutter/flutter/pull/124262
/// Text selection handle controls that follow the Material Design specification.
///
/// Dart's global: every call hands out the same controls, so an `EditableText` given them
/// again sees them unchanged.
#[allow(deprecated)]
pub fn material_text_selection_handle_controls() -> Rc<dyn TextSelectionControls> {
    thread_local! {
        static CONTROLS: Rc<dyn TextSelectionControls> = Rc::new(MaterialTextSelectionHandleControls);
    }
    CONTROLS.with(Rc::clone)
}

/// Text selection controls that follow the Material Design specification.
///
/// Dart's global: every call hands out the same controls, so an `EditableText` given them
/// again sees them unchanged.
pub fn material_text_selection_controls() -> Rc<dyn TextSelectionControls> {
    thread_local! {
        static CONTROLS: Rc<dyn TextSelectionControls> = Rc::new(MaterialTextSelectionControls);
    }
    CONTROLS.with(Rc::clone)
}

#[cfg(test)]
mod tests {
    use waft_embedder::{TextAffinity, TextSelection};
    use waft_painting::{EdgeInsets, TextStyle};
    use waft_services::TextSelectionDelegate;
    use waft_widgets::{
        Builder, Container, EditableText, EditableTextState, FittedBox, FocusNode, GlobalKey,
        Padding, TextEditingController, TextEditingControllerLeaf,
    };

    use waft_widgets::testing::{WidgetTester, test_widgets};

    use super::*;
    use crate::colors::Colors;
    use crate::test_support::material_app;
    use crate::text_selection_theme::TextSelectionThemeData;
    use crate::theme_data::ThemeData;

    /// Dart's `createEditableText(key:, text:, selection:)`.
    fn create_editable_text(
        tester: &WidgetTester,
        key: &GlobalKey,
        text: Option<&str>,
        selection: Option<TextSelection>,
    ) -> WidgetRef {
        let mut app = tester.cell.borrow_mut();
        let controller = TextEditingController::new(&mut app);
        let controller = match text {
            Some(text) => controller.text(&mut app, text),
            None => controller,
        };
        controller.set_selection(
            &mut app,
            selection.unwrap_or(TextSelection::collapsed(-1, TextAffinity::Downstream)),
        );
        let focus_node = FocusNode::new(&mut app);
        material_app(
            ThemeData::new(&app),
            EditableText::new(
                controller.erase(),
                focus_node.erase(),
                TextStyle::new(),
                Colors::BLACK,
                Colors::BLACK,
            )
            .key(key.clone().into())
            .into_widget(),
        )
    }

    /// `materialTextSelectionControls.canSelectAll(key.currentState!)`.
    #[allow(deprecated)]
    fn can_select_all(tester: &WidgetTester, key: &GlobalKey) -> bool {
        let app = tester.cell.borrow();
        let state = key
            .current_state::<EditableTextState>(&app)
            .expect("the EditableText is mounted");
        material_text_selection_controls().can_select_all(&app, state.as_text_selection_delegate())
    }

    #[test]
    fn can_select_all_should_return_false_when_there_is_no_text() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let root = create_editable_text(tester, &key, None, None);
            tester.pump_widget(root);
            assert!(!can_select_all(tester, &key));
        });
    }

    #[test]
    fn can_select_all_should_return_true_when_there_is_text_and_collapsed_selection() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let root = create_editable_text(tester, &key, Some("123"), None);
            tester.pump_widget(root);
            assert!(can_select_all(tester, &key));
        });
    }

    #[test]
    fn can_select_all_should_return_true_when_there_is_text_and_partial_uncollapsed_selection() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let root =
                create_editable_text(tester, &key, Some("123"), Some(TextSelection::new(1, 2)));
            tester.pump_widget(root);
            assert!(can_select_all(tester, &key));
        });
    }

    #[test]
    fn can_select_all_should_return_false_when_there_is_text_and_full_selection() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let root =
                create_editable_text(tester, &key, Some("123"), Some(TextSelection::new(0, 3)));
            tester.pump_widget(root);
            assert!(!can_select_all(tester, &key));
        });
    }

    #[test]
    fn material_handles_works_with_3_positional_parameters() {
        test_widgets(|tester| {
            let theme = ThemeData::builder()
                .text_selection_theme(
                    TextSelectionThemeData::new().selection_handle_color(Color::new(0x550000AA)),
                )
                .build(&tester.cell.borrow());
            tester.pump_widget(
                Theme::new(
                    theme,
                    Builder::new(|app, context| {
                        Container::new()
                            .color(Colors::WHITE)
                            .height(800.0)
                            .width(800.0)
                            .child(Padding::new(EdgeInsets::symmetric(0.0, 250.0)).child(
                                FittedBox::new().child(
                                    material_text_selection_controls().build_handle(
                                        app,
                                        context,
                                        TextSelectionHandleType::Right,
                                        10.0,
                                        None,
                                    ),
                                ),
                            ))
                            .into_widget()
                    }),
                )
                .into_widget(),
            );

            // No expect here as this should simply compile / not throw any
            // exceptions while building.
        });
    }

    /// The matrix of the `Transform` that `build_handle` wraps the handle of `handle_type` in,
    /// if any.
    fn handle_rotation(
        tester: &mut WidgetTester,
        handle_type: TextSelectionHandleType,
    ) -> Option<waft_embedder::Matrix4> {
        let theme = ThemeData::new(&tester.cell.borrow());
        let built = Rc::new(std::cell::RefCell::new(None));
        let root = {
            let built = Rc::clone(&built);
            Theme::new(
                theme,
                Builder::new(move |app, context| {
                    let handle = material_text_selection_controls().build_handle(
                        app,
                        context,
                        handle_type,
                        10.0,
                        None,
                    );
                    *built.borrow_mut() = Some(Rc::clone(&handle));
                    handle
                }),
            )
            .into_widget()
        };
        tester.pump_widget(root);
        let handle = built.borrow_mut().take().expect("the handle built");
        waft_widgets::downcast_widget::<Transform>(&*handle).map(|transform| transform.transform)
    }

    #[test]
    fn material_handles_point_up_left_and_turn_by_type() {
        test_widgets(|tester| {
            assert_eq!(
                handle_rotation(tester, TextSelectionHandleType::Left),
                Some(Transform::rotate(PI / 2.0).transform)
            );
            assert_eq!(
                handle_rotation(tester, TextSelectionHandleType::Right),
                None
            );
            assert_eq!(
                handle_rotation(tester, TextSelectionHandleType::Collapsed),
                Some(Transform::rotate(PI / 4.0).transform)
            );
        });
    }

    #[test]
    fn material_handles_anchor_and_size_by_type() {
        let controls = MaterialTextSelectionControls::new();
        assert_eq!(controls.get_handle_size(10.0), Size::new(22.0, 22.0));
        assert_eq!(
            controls.get_handle_anchor(TextSelectionHandleType::Collapsed, 10.0),
            Offset::new(11.0, -4.0)
        );
        assert_eq!(
            controls.get_handle_anchor(TextSelectionHandleType::Left, 10.0),
            Offset::new(22.0, 0.0)
        );
        assert_eq!(
            controls.get_handle_anchor(TextSelectionHandleType::Right, 10.0),
            Offset::ZERO
        );
    }

    #[test]
    fn the_handle_painter_fills_a_circle_with_its_top_left_quadrant() {
        let cell = waft_foundation::AppCell::new();
        let mut app = cell.borrow_mut();
        let painter = TextSelectionHandlePainter {
            color: Color::new(0x550000AA),
        };
        let mut canvas = Canvas::new();
        painter.paint(&mut app, &mut canvas, Size::new(22.0, 22.0));
        let picture = canvas.build();
        let ops = waft_embedder::flattened_ops(&picture);
        assert_eq!(ops.len(), 1, "one path");
        let waft_embedder::valo::Op::DrawPath { path, paint, .. } = &ops[0] else {
            panic!("the handle is a path, not {:?}", ops[0]);
        };
        assert_eq!(paint.color, Color::new(0x550000AA).into());
        let bounds = path.bounds();
        assert_eq!(
            (bounds.x, bounds.y, bounds.width, bounds.height),
            (0.0, 0.0, 22.0, 22.0)
        );
        // The square fills the corner the circle leaves open.
        assert!(path.contains((1.0, 1.0).into(), FillRule::NonZero));
        assert!(!path.contains((21.0, 1.0).into(), FillRule::NonZero));
        // Where the circle and the square overlap is filled once, with no hole: what Dart's
        // 'draws transparent handle correctly' golden guards.
        assert!(path.contains((8.0, 8.0).into(), FillRule::NonZero));
        assert!(path.contains((15.0, 15.0).into(), FillRule::NonZero));
    }
}
