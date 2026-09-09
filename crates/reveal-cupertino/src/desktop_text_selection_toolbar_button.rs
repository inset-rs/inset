//! Flutter counterpart: `cupertino/desktop_text_selection_toolbar_button.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;

use reveal_embedder::{Color, FontWeight, Radius, Size};
use reveal_foundation::{App, Handle, Listener};
use reveal_painting::{
    AlignmentGeometry, BorderRadius, EdgeInsets, EdgeInsetsGeometry, TextOverflow, TextStyle,
};
use reveal_widgets::{
    BuildContext, ContextMenuButtonItem, IntoWidget, KeyRef, MouseRegion, SizedBox, State,
    StateData, StatefulWidget, Text, WidgetRef,
};

use crate::button::CupertinoButton;
use crate::colors::CupertinoDynamicColor;
use crate::text_selection_toolbar_button::CupertinoTextSelectionToolbarButton;
use crate::theme::CupertinoTheme;

fn k_toolbar_button_font_style() -> TextStyle {
    TextStyle::new()
        .inherit(false)
        .font_size(14.0)
        .letter_spacing(-0.15)
        .font_weight(FontWeight::W400)
}

const K_TOOLBAR_BUTTON_PADDING: EdgeInsets = EdgeInsets::from_ltrb(8.0, 2.0, 8.0, 5.0);

/// A button in the style of the Mac context menu buttons.
pub struct CupertinoDesktopTextSelectionToolbarButton {
    pub key: Option<KeyRef>,
    /// Called when this button is pressed.
    pub on_pressed: Option<Listener>,
    /// The child of this button.
    pub child: Option<WidgetRef>,
    /// The buttonItem used to generate the button when using
    /// [`button_item`](Self::button_item).
    pub button_item: Option<ContextMenuButtonItem>,
    /// The text used in the button's label when using [`text`](Self::text).
    pub text: Option<String>,
}

impl Debug for CupertinoDesktopTextSelectionToolbarButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoDesktopTextSelectionToolbarButton")
            .finish_non_exhaustive()
    }
}

impl CupertinoDesktopTextSelectionToolbarButton {
    /// Creates an instance of [`CupertinoDesktopTextSelectionToolbarButton`].
    pub fn new<K>(
        on_pressed: Option<Listener>,
        child: impl IntoWidget<K>,
    ) -> CupertinoDesktopTextSelectionToolbarButton {
        CupertinoDesktopTextSelectionToolbarButton {
            key: None,
            on_pressed,
            child: Some(child.into_widget()),
            button_item: None,
            text: None,
        }
    }

    /// Dart `CupertinoDesktopTextSelectionToolbarButton(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoDesktopTextSelectionToolbarButton {
        self.key = Some(key);
        self
    }

    /// Create an instance of [`CupertinoDesktopTextSelectionToolbarButton`] whose child is
    /// a [`Text`] widget styled like the default Mac context menu button.
    pub fn text(
        on_pressed: Option<Listener>,
        text: impl Into<String>,
    ) -> CupertinoDesktopTextSelectionToolbarButton {
        CupertinoDesktopTextSelectionToolbarButton {
            key: None,
            on_pressed,
            child: None,
            button_item: None,
            text: Some(text.into()),
        }
    }

    /// Create an instance of [`CupertinoDesktopTextSelectionToolbarButton`] from the given
    /// [`ContextMenuButtonItem`].
    pub fn button_item(
        button_item: ContextMenuButtonItem,
    ) -> CupertinoDesktopTextSelectionToolbarButton {
        let on_pressed = button_item.on_pressed.clone();
        CupertinoDesktopTextSelectionToolbarButton {
            key: None,
            on_pressed,
            child: None,
            button_item: Some(button_item),
            text: None,
        }
    }
}

impl StatefulWidget for CupertinoDesktopTextSelectionToolbarButton {
    type State = CupertinoDesktopTextSelectionToolbarButtonState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoDesktopTextSelectionToolbarButtonState {
        CupertinoDesktopTextSelectionToolbarButtonState {
            state: StateData::new(),
            is_hovered: false,
        }
    }
}

/// Dart's `_CupertinoDesktopTextSelectionToolbarButtonState`.
pub struct CupertinoDesktopTextSelectionToolbarButtonState {
    state: StateData<CupertinoDesktopTextSelectionToolbarButton>,
    is_hovered: bool,
}

impl Debug for CupertinoDesktopTextSelectionToolbarButtonState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoDesktopTextSelectionToolbarButtonState")
            .finish_non_exhaustive()
    }
}

impl CupertinoDesktopTextSelectionToolbarButtonState {
    fn on_enter(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |state| state.is_hovered = true);
    }

    fn on_exit(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |state| state.is_hovered = false);
    }
}

impl State for CupertinoDesktopTextSelectionToolbarButtonState {
    type Widget = CupertinoDesktopTextSelectionToolbarButton;
    reveal_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let is_hovered = app.get(self).is_hovered;
        let on_pressed = self.widget(app).on_pressed.clone();
        let child = if let Some(child) = self.widget(app).child.clone() {
            child
        } else {
            let text = self.widget(app).text.clone();
            let button_item = self.widget(app).button_item.clone();
            let label = text.unwrap_or_else(|| {
                CupertinoTextSelectionToolbarButton::get_button_label(
                    app,
                    context,
                    button_item.as_ref().expect("text or buttonItem"),
                )
            });
            let color = if is_hovered {
                CupertinoTheme::of(app, context).primary_contrasting_color()
            } else {
                CupertinoDynamicColor::with_brightness(
                    Color::new(0xFF000000),
                    Color::new(0xFFFFFFFF),
                )
                .resolve_from(app, context)
                .into()
            };
            Text::new(label)
                .overflow(TextOverflow::Ellipsis)
                .style(k_toolbar_button_font_style().color(color))
                .into_widget()
        };

        let mut button = CupertinoButton::new(child, on_pressed)
            .alignment(AlignmentGeometry::CENTER_LEFT)
            .border_radius(BorderRadius::all(Radius::circular(4.0)))
            .minimum_size(Size::ZERO)
            .padding(EdgeInsetsGeometry::Insets(K_TOOLBAR_BUTTON_PADDING))
            .pressed_opacity(Some(0.7));
        if is_hovered {
            button = button.color(CupertinoTheme::of(app, context).primary_color());
        }

        let this = self;
        SizedBox::new()
            .width(f64::INFINITY)
            .child(
                MouseRegion::new()
                    .on_enter(Rc::new(move |app, _event| this.on_enter(app)))
                    .on_exit(Rc::new(move |app, _event| this.on_exit(app)))
                    .child(button),
            )
            .into_widget()
    }
}
