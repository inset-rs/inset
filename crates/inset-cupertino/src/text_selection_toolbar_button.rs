//! Flutter counterpart: `cupertino/text_selection_toolbar_button.dart`.

use std::any::Any;
use std::f64::consts::PI;
use std::fmt::{self, Debug};
use std::rc::Rc;

use inset_embedder::{
    Canvas, Color, FillRule, FontWeight, Offset, Paint, PaintStyle, PathBuilder, Size, Stroke,
};
use inset_foundation::{App, Handle, Listener};
use inset_gestures::{GestureTapCancelCallback, GestureTapDownCallback, GestureTapUpCallback};
use inset_painting::{AnyColor, EdgeInsets, EdgeInsetsGeometry, TextOverflow, TextStyle};
use inset_rendering::CustomPainter;
use inset_widgets::{
    BuildContext, ContextMenuButtonItem, ContextMenuButtonType, CustomPaint, GestureDetector,
    IntoWidget, KeyRef, SizedBox, State, StateData, StatefulWidget, Text, WidgetRef,
};

use crate::button::CupertinoButton;
use crate::colors::{CupertinoColors, CupertinoDynamicColor};
use crate::debug::debug_check_has_cupertino_localizations;
use crate::localizations::CupertinoLocalizations;

fn k_toolbar_button_font_style() -> TextStyle {
    TextStyle::new()
        .inherit(false)
        .font_size(15.0)
        .letter_spacing(-0.15)
        .font_weight(FontWeight::W400)
}

const K_TOOLBAR_TEXT_COLOR: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0xFF000000), Color::new(0xFFFFFFFF));

const K_TOOLBAR_PRESSED_COLOR: CupertinoDynamicColor =
    CupertinoDynamicColor::with_brightness(Color::new(0x10000000), Color::new(0x10FFFFFF));

const K_TOOLBAR_BUTTON_PADDING: EdgeInsets = EdgeInsets::from_ltrb(16.0, 18.0, 16.0, 18.0);

/// A button in the style of the iOS text selection toolbar buttons.
pub struct CupertinoTextSelectionToolbarButton {
    pub key: Option<KeyRef>,
    /// Called when this button is pressed.
    pub on_pressed: Option<Listener>,
    /// The child of this button.
    ///
    /// Usually a [`Text`] or an `Icon`.
    pub child: Option<WidgetRef>,
    /// The buttonItem used to generate the button when using
    /// [`button_item`](Self::button_item).
    pub button_item: Option<ContextMenuButtonItem>,
    /// The text used in the button's label when using [`text`](Self::text).
    pub text: Option<String>,
}

impl Debug for CupertinoTextSelectionToolbarButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoTextSelectionToolbarButton")
            .finish_non_exhaustive()
    }
}

impl CupertinoTextSelectionToolbarButton {
    /// Create an instance of [`CupertinoTextSelectionToolbarButton`].
    pub fn new<K>(
        on_pressed: Option<Listener>,
        child: impl IntoWidget<K>,
    ) -> CupertinoTextSelectionToolbarButton {
        CupertinoTextSelectionToolbarButton {
            key: None,
            on_pressed,
            child: Some(child.into_widget()),
            button_item: None,
            text: None,
        }
    }

    /// Dart `CupertinoTextSelectionToolbarButton(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoTextSelectionToolbarButton {
        self.key = Some(key);
        self
    }

    /// Create an instance of [`CupertinoTextSelectionToolbarButton`] whose child is a
    /// [`Text`] widget styled like the default iOS text selection toolbar button.
    pub fn text(
        on_pressed: Option<Listener>,
        text: impl Into<String>,
    ) -> CupertinoTextSelectionToolbarButton {
        CupertinoTextSelectionToolbarButton {
            key: None,
            on_pressed,
            child: None,
            button_item: None,
            text: Some(text.into()),
        }
    }

    /// Create an instance of [`CupertinoTextSelectionToolbarButton`] from the given
    /// [`ContextMenuButtonItem`].
    pub fn button_item(button_item: ContextMenuButtonItem) -> CupertinoTextSelectionToolbarButton {
        let on_pressed = button_item.on_pressed.clone();
        CupertinoTextSelectionToolbarButton {
            key: None,
            on_pressed,
            child: None,
            button_item: Some(button_item),
            text: None,
        }
    }

    /// Returns the default button label String for the button of the given
    /// [`ContextMenuButtonItem`]'s [`ContextMenuButtonType`].
    pub fn get_button_label(
        app: &mut App,
        context: BuildContext,
        button_item: &ContextMenuButtonItem,
    ) -> String {
        if let Some(label) = &button_item.label {
            return label.clone();
        }
        debug_assert!(debug_check_has_cupertino_localizations(app, context));
        let localizations = <dyn CupertinoLocalizations>::of(app, context);
        match button_item.r#type {
            ContextMenuButtonType::Cut => localizations.cut_button_label(),
            ContextMenuButtonType::Copy => localizations.copy_button_label(),
            ContextMenuButtonType::Paste => localizations.paste_button_label(),
            ContextMenuButtonType::SelectAll => localizations.select_all_button_label(),
            ContextMenuButtonType::LookUp => localizations.look_up_button_label(),
            ContextMenuButtonType::SearchWeb => localizations.search_web_button_label(),
            ContextMenuButtonType::Share => localizations.share_button_label(),
            ContextMenuButtonType::LiveTextInput
            | ContextMenuButtonType::Delete
            | ContextMenuButtonType::Custom => String::new(),
        }
    }
}

impl StatefulWidget for CupertinoTextSelectionToolbarButton {
    type State = CupertinoTextSelectionToolbarButtonState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoTextSelectionToolbarButtonState {
        CupertinoTextSelectionToolbarButtonState {
            state: StateData::new(),
            is_pressed: false,
        }
    }
}

/// Dart's `_CupertinoTextSelectionToolbarButtonState`.
pub struct CupertinoTextSelectionToolbarButtonState {
    state: StateData<CupertinoTextSelectionToolbarButton>,
    is_pressed: bool,
}

impl Debug for CupertinoTextSelectionToolbarButtonState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoTextSelectionToolbarButtonState")
            .finish_non_exhaustive()
    }
}

impl CupertinoTextSelectionToolbarButtonState {
    fn on_tap_down(self: Handle<Self>, app: &mut App, _details: inset_gestures::TapDownDetails) {
        self.set_state(app, |state| state.is_pressed = true);
    }

    fn on_tap_up(self: Handle<Self>, app: &mut App, _details: inset_gestures::TapUpDetails) {
        self.set_state(app, |state| state.is_pressed = false);
        if let Some(on_pressed) = self.widget(app).on_pressed.clone() {
            on_pressed.call(app);
        }
    }

    fn on_tap_cancel(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |state| state.is_pressed = false);
    }

    fn get_content_widget(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        if let Some(child) = self.widget(app).child.clone() {
            return child;
        }
        let on_pressed = self.widget(app).on_pressed.clone();
        let text = self.widget(app).text.clone();
        let button_item = self.widget(app).button_item.clone();
        let label = text.unwrap_or_else(|| {
            CupertinoTextSelectionToolbarButton::get_button_label(
                app,
                context,
                button_item.as_ref().expect("text or buttonItem"),
            )
        });
        let text_color: AnyColor = if on_pressed.is_some() {
            K_TOOLBAR_TEXT_COLOR.resolve_from(app, context).into()
        } else {
            CupertinoColors::INACTIVE_GRAY.clone()
        };
        let text_widget = Text::new(label)
            .overflow(TextOverflow::Ellipsis)
            .style(k_toolbar_button_font_style().color(text_color))
            .into_widget();
        match button_item.as_ref().map(|item| item.r#type) {
            Some(ContextMenuButtonType::LiveTextInput) => SizedBox::square(Some(13.0))
                .child(
                    CustomPaint::new().painter(LiveTextIconPainter {
                        color: K_TOOLBAR_TEXT_COLOR
                            .resolve_from(app, context)
                            .effective_color(),
                    }),
                )
                .into_widget(),
            _ => text_widget,
        }
    }
}

impl State for CupertinoTextSelectionToolbarButtonState {
    type Widget = CupertinoTextSelectionToolbarButton;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let content = self.get_content_widget(app, context);
        let is_pressed = app.get(self).is_pressed;
        let on_pressed = self.widget(app).on_pressed.clone();
        let color: AnyColor = if is_pressed {
            K_TOOLBAR_PRESSED_COLOR.resolve_from(app, context).into()
        } else {
            CupertinoColors::TRANSPARENT.clone()
        };
        let child = CupertinoButton::new(content, on_pressed.clone())
            .color(color)
            .disabled_color(CupertinoColors::TRANSPARENT)
            .padding(EdgeInsetsGeometry::Insets(K_TOOLBAR_BUTTON_PADDING))
            .pressed_opacity(Some(1.0));
        if on_pressed.is_some() {
            let this = self;
            GestureDetector::new()
                .on_tap_down({
                    let cb: GestureTapDownCallback = Rc::new(move |app, details| {
                        this.on_tap_down(app, details);
                    });
                    cb
                })
                .on_tap_up({
                    let cb: GestureTapUpCallback = Rc::new(move |app, details| {
                        this.on_tap_up(app, details);
                    });
                    cb
                })
                .on_tap_cancel({
                    let cb: GestureTapCancelCallback =
                        Listener::handle_method(this, Self::on_tap_cancel);
                    cb
                })
                .child(child)
                .into_widget()
        } else {
            child.into_widget()
        }
    }
}

struct LiveTextIconPainter {
    color: Color,
}

impl CustomPainter for LiveTextIconPainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        let painter = Paint {
            style: PaintStyle::Stroke(Stroke::new(1.0)),
            color: self.color.into(),
            ..Paint::default()
        };
        canvas.save();
        canvas.translate((size.width() / 2.0) as f32, (size.height() / 2.0) as f32);

        let origin = Offset::new(-size.width() / 2.0, -size.height() / 2.0);
        let mut path = PathBuilder::new();
        path.move_to(Offset::new(origin.dx(), origin.dy() + 3.5));
        path.line_to(Offset::new(origin.dx(), origin.dy() + 1.0));
        path.arc_to(
            Offset::new(origin.dx(), origin.dy()),
            Offset::new(origin.dx() + 1.0, origin.dy()),
            1.0,
        );
        path.line_to(Offset::new(origin.dx() + 3.5, origin.dy()));
        let corner = path.build();

        for _ in 0..4 {
            canvas.draw_path(&corner, FillRule::NonZero, &painter);
            canvas.rotate((PI / 2.0) as f32);
        }

        let mut lines = PathBuilder::new();
        lines.move_to(Offset::new(-3.0, -3.0));
        lines.line_to(Offset::new(3.0, -3.0));
        lines.move_to(Offset::new(-3.0, 0.0));
        lines.line_to(Offset::new(3.0, 0.0));
        lines.move_to(Offset::new(-3.0, 3.0));
        lines.line_to(Offset::new(1.0, 3.0));
        canvas.draw_path(&lines.build(), FillRule::NonZero, &painter);

        canvas.restore();
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        old_delegate
            .as_any()
            .downcast_ref::<LiveTextIconPainter>()
            .is_none_or(|old| old.color != self.color)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
