//! XAML `HyperlinkButton` with the `DefaultHyperlinkButtonStyle` of `HyperlinkButton_themeresources.xaml`.
//!
//! The template is one `ContentPresenter` (`x:Name="ContentPresenter"`) with `BackgroundSizing="OuterBorderEdge"`, `ButtonPadding`, `HyperlinkButtonBorderThemeThickness` and a `BrushTransition` of 83 ms on `Background`; the `CommonStates` storyboards swap `Foreground`, `Background` and `BorderBrush` per state. The text carries no underline: `Hyperlink_themeresources.xaml` sets `HyperlinkUnderlineVisible` to `False`, which `HyperlinkButton::UpdateContentPresenterTextUnderline` reads (the underline returns only in high contrast). `HyperlinkButton::Initialize` sets the hand cursor.

use crate::{
    BackgroundSizing, Brush, ColorTransition, CommonState, CommonStates, ControlBorder,
    ControlStates, FocusVisual, ThemeResources, control_text_style,
};
use inset_embedder::{Color, FontWeight};
use inset_foundation::{App, Listener};
use inset_services::SystemMouseCursors;
use inset_widgets::*;
use std::{fmt, rc::Rc};

/// The `FocusVisualMargin` the style sets.
pub const HYPERLINK_BUTTON_FOCUS_VISUAL_MARGIN: [f64; 4] = [-3.0; 4];

/// The brushes the style resolves to in one state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HyperlinkButtonBrushes {
    pub background: Color,
    pub border: Brush,
    pub foreground: Color,
}

/// The `{ThemeResource …}` values the `CommonStates` storyboards name for `state`.
pub fn hyperlink_button_brushes(
    resources: &ThemeResources,
    state: CommonState,
) -> HyperlinkButtonBrushes {
    let r = resources;
    match state {
        CommonState::Normal => HyperlinkButtonBrushes {
            background: r.hyperlink_button_background,
            border: Brush::Solid(r.hyperlink_button_border_brush),
            foreground: r.hyperlink_button_foreground,
        },
        CommonState::PointerOver => HyperlinkButtonBrushes {
            background: r.hyperlink_button_background_pointer_over,
            border: Brush::Solid(r.hyperlink_button_border_brush_pointer_over),
            foreground: r.hyperlink_button_foreground_pointer_over,
        },
        CommonState::Pressed => HyperlinkButtonBrushes {
            background: r.hyperlink_button_background_pressed,
            border: Brush::Solid(r.hyperlink_button_border_brush_pressed),
            foreground: r.hyperlink_button_foreground_pressed,
        },
        CommonState::Disabled => HyperlinkButtonBrushes {
            background: r.hyperlink_button_background_disabled,
            border: Brush::Solid(r.hyperlink_button_border_brush_disabled),
            foreground: r.hyperlink_button_foreground_disabled,
        },
    }
}

/// XAML `HyperlinkButton`: `Content` in the accent foreground, `Click` raised on activation. `NavigateUri` is not offered: launching it is the Windows shell's (`Launcher::TryInvokeLauncher`); a caller opens its link in `click`.
#[derive(Clone)]
pub struct HyperlinkButton {
    pub content: WidgetRef,
    pub click: Listener,
    pub is_enabled: bool,
    pub key: Option<KeyRef>,
}

impl HyperlinkButton {
    pub fn new<K>(content: impl IntoWidget<K>, click: Listener) -> HyperlinkButton {
        HyperlinkButton {
            content: content.into_widget(),
            click,
            is_enabled: true,
            key: None,
        }
    }

    /// A text `Content`, as `<HyperlinkButton Content="…"/>`.
    pub fn text(text: impl Into<String>, click: Listener) -> HyperlinkButton {
        HyperlinkButton::new(Text::new(text), click)
    }

    /// XAML `IsEnabled`.
    pub fn is_enabled(mut self, enabled: bool) -> HyperlinkButton {
        self.is_enabled = enabled;
        self
    }

    pub fn key(mut self, key: KeyRef) -> HyperlinkButton {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for HyperlinkButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HyperlinkButton")
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for HyperlinkButton {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let content = self.content.clone();
        let states = CommonStates::new(self.click.clone(), move |app, context, states| {
            template(app, context, content.clone(), states)
        })
        .is_enabled(self.is_enabled);
        // `HyperlinkButton::Initialize`: `SetCursor(MouseCursorHand)`.
        MouseRegion::new()
            .cursor(Rc::new(SystemMouseCursors::CLICK))
            .child(states)
            .into_widget()
    }
}

/// The `ControlTemplate` of `DefaultHyperlinkButtonStyle` for one set of states.
fn template(
    app: &mut App,
    context: BuildContext,
    content: WidgetRef,
    states: ControlStates,
) -> WidgetRef {
    let resources = ThemeResources::of(app, context);
    let brushes = hyperlink_button_brushes(&resources, states.common);
    let text = control_text_style(
        &resources,
        resources.control_content_theme_font_size,
        FontWeight::W400,
        brushes.foreground,
    );
    // `ContentPresenter`: `HorizontalContentAlignment` and `VerticalContentAlignment` are Center.
    let label = Center::new()
        .width_factor(1.0)
        .height_factor(1.0)
        .child(DefaultTextStyle::new(text, content))
        .into_widget();
    let border_thickness = resources.hyperlink_button_border_theme_thickness[0];
    let (corner_radius, padding) = (resources.control_corner_radius[0], resources.button_padding);
    let presenter = ColorTransition::new(
        brushes.background,
        resources.control_faster_animation_duration,
        move |_app, background| {
            ControlBorder::new(Brush::Solid(background), brushes.border)
                .border_thickness(border_thickness)
                .corner_radius(corner_radius)
                .background_sizing(BackgroundSizing::OuterBorderEdge)
                .padding(padding)
                .child(label.clone())
                .into_widget()
        },
    );
    FocusVisual::new(presenter, resources.theme)
        .visible(states.focused)
        .margin(HYPERLINK_BUTTON_FOCUS_VISUAL_MARGIN)
        .corner_radius(resources.control_corner_radius[0])
        .into_widget()
}
