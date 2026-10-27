//! XAML `Button` with the `DefaultButtonStyle`, `AccentButtonStyle` and `SubtleButtonStyle` of `Button_themeresources.xaml`.
//!
//! The template is one `ContentPresenter` (`x:Name="ContentPresenter"`) carrying `Background`, `BorderBrush`, `BorderThickness`, `CornerRadius`, `Padding` and `Foreground`, with a `BrushTransition` of 83 ms on `Background`; the `CommonStates` storyboards swap those brushes per state.

use crate::{
    BackgroundSizing, Brush, ColorTransition, CommonState, CommonStates, ControlBorder,
    ControlStates, FocusVisual, ThemeResources, control_text_style,
};
use inset_embedder::{Color, FontWeight};
use inset_foundation::{App, Listener};
use inset_widgets::*;
use std::{fmt, rc::Rc};

/// A source control template receiving the button state and its content.
pub type ButtonTemplate = Rc<dyn Fn(&mut App, BuildContext, ControlStates, WidgetRef) -> WidgetRef>;

/// XAML `ButtonBorderThemeThickness`.
pub const BUTTON_BORDER_THICKNESS: f64 = 1.0;
/// The `FocusVisualMargin` the button styles set.
pub const BUTTON_FOCUS_VISUAL_MARGIN: [f64; 4] = [-3.0; 4];

/// Which `Button` style the control uses; XAML `Style="{StaticResource AccentButtonStyle}"` and its siblings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonStyle {
    /// `DefaultButtonStyle`.
    #[default]
    Default,
    /// `AccentButtonStyle`: the accent fill, `BackgroundSizing="OuterBorderEdge"`.
    Accent,
    /// `SubtleButtonStyle`: no fill or border until the pointer is over it.
    Subtle,
}

/// The brushes a button style resolves to in one state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonBrushes {
    pub background: Color,
    pub border: Brush,
    pub foreground: Color,
}

impl ButtonStyle {
    /// The `{ThemeResource …}` values the style's `CommonStates` storyboards name for `state`.
    pub fn brushes(self, resources: &ThemeResources, state: CommonState) -> ButtonBrushes {
        let r = resources;
        match (self, state) {
            (ButtonStyle::Default, CommonState::Normal) => ButtonBrushes {
                background: r.button_background,
                border: Brush::ControlElevation(r.button_border_brush),
                foreground: r.button_foreground,
            },
            (ButtonStyle::Default, CommonState::PointerOver) => ButtonBrushes {
                background: r.button_background_pointer_over,
                border: Brush::ControlElevation(r.button_border_brush_pointer_over),
                foreground: r.button_foreground_pointer_over,
            },
            (ButtonStyle::Default, CommonState::Pressed) => ButtonBrushes {
                background: r.button_background_pressed,
                border: Brush::Solid(r.button_border_brush_pressed),
                foreground: r.button_foreground_pressed,
            },
            (ButtonStyle::Default, CommonState::Disabled) => ButtonBrushes {
                background: r.button_background_disabled,
                border: Brush::Solid(r.button_border_brush_disabled),
                foreground: r.button_foreground_disabled,
            },
            (ButtonStyle::Accent, CommonState::Normal) => ButtonBrushes {
                background: r.accent_button_background,
                border: Brush::ControlElevation(r.accent_button_border_brush),
                foreground: r.accent_button_foreground,
            },
            (ButtonStyle::Accent, CommonState::PointerOver) => ButtonBrushes {
                background: r.accent_button_background_pointer_over,
                border: Brush::ControlElevation(r.accent_button_border_brush_pointer_over),
                foreground: r.accent_button_foreground_pointer_over,
            },
            (ButtonStyle::Accent, CommonState::Pressed) => ButtonBrushes {
                background: r.accent_button_background_pressed,
                border: Brush::Solid(r.accent_button_border_brush_pressed),
                foreground: r.accent_button_foreground_pressed,
            },
            (ButtonStyle::Accent, CommonState::Disabled) => ButtonBrushes {
                background: r.accent_button_background_disabled,
                border: Brush::Solid(r.accent_button_border_brush_disabled),
                foreground: r.accent_button_foreground_disabled,
            },
            (ButtonStyle::Subtle, CommonState::Normal) => ButtonBrushes {
                background: r.subtle_button_background,
                border: Brush::Solid(r.subtle_button_border_brush),
                foreground: r.subtle_button_foreground,
            },
            (ButtonStyle::Subtle, CommonState::PointerOver) => ButtonBrushes {
                background: r.subtle_button_background_pointer_over,
                border: Brush::Solid(r.subtle_button_border_brush_pointer_over),
                foreground: r.subtle_button_foreground_pointer_over,
            },
            (ButtonStyle::Subtle, CommonState::Pressed) => ButtonBrushes {
                background: r.subtle_button_background_pressed,
                border: Brush::Solid(r.subtle_button_border_brush_pressed),
                foreground: r.subtle_button_foreground_pressed,
            },
            (ButtonStyle::Subtle, CommonState::Disabled) => ButtonBrushes {
                background: r.subtle_button_background_disabled,
                border: Brush::Solid(r.subtle_button_border_brush_disabled),
                foreground: r.subtle_button_foreground_disabled,
            },
        }
    }

    /// XAML `BackgroundSizing` of the style.
    pub fn background_sizing(self) -> BackgroundSizing {
        match self {
            ButtonStyle::Accent => BackgroundSizing::OuterBorderEdge,
            ButtonStyle::Default | ButtonStyle::Subtle => BackgroundSizing::InnerBorderEdge,
        }
    }
}

/// XAML `Button`: `Content` shown in the style's bezel, `Click` raised on activation.
#[derive(Clone)]
pub struct Button {
    /// Content presented inside the selected control template.
    pub content: WidgetRef,
    /// Raised when the button activates.
    pub click: Listener,
    /// The standard source style used when no template override is supplied.
    pub style: ButtonStyle,
    /// `Padding` between the bezel and the content, in left, top, right, bottom order; the theme's
    /// `ButtonPadding` when `None`, as the style sets it.
    pub padding: Option<[f64; 4]>,
    /// Whether pointer and keyboard input can activate this button.
    pub is_enabled: bool,
    /// Whether sequential keyboard navigation stops here.
    pub is_tab_stop: bool,
    /// Optional caller-owned focus node used by composite controls.
    pub focus_node: Option<AnyFocusNode>,
    /// An alternate source template, such as TabView's close button chrome.
    pub template: Option<ButtonTemplate>,
    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl Button {
    /// Creates a button with content and a Click callback.
    pub fn new<K>(content: impl IntoWidget<K>, click: Listener) -> Button {
        Button {
            content: content.into_widget(),
            click,
            style: ButtonStyle::Default,
            padding: None,
            is_enabled: true,
            is_tab_stop: true,
            focus_node: None,
            template: None,
            key: None,
        }
    }

    /// A text `Content`, as `<Button Content="…"/>`.
    pub fn text(text: impl Into<String>, click: Listener) -> Button {
        Button::new(Text::new(text), click)
    }

    /// Selects one of the standard source styles.
    pub fn style(mut self, style: ButtonStyle) -> Button {
        self.style = style;
        self
    }

    /// XAML `Padding`.
    pub fn padding(mut self, padding: [f64; 4]) -> Button {
        self.padding = Some(padding);
        self
    }

    /// XAML `IsEnabled`.
    pub fn is_enabled(mut self, enabled: bool) -> Button {
        self.is_enabled = enabled;
        self
    }

    /// Sets the source `IsTabStop` property.
    pub fn is_tab_stop(mut self, value: bool) -> Self {
        self.is_tab_stop = value;
        self
    }

    /// Uses a focus node whose lifetime is owned by the enclosing control.
    pub fn focus_node(mut self, node: AnyFocusNode) -> Self {
        self.focus_node = Some(node);
        self
    }

    /// Replaces presentation while retaining ButtonBase input and activation.
    pub fn template(
        mut self,
        builder: impl Fn(&mut App, BuildContext, ControlStates, WidgetRef) -> WidgetRef + 'static,
    ) -> Self {
        self.template = Some(Rc::new(builder));
        self
    }

    /// Sets the identity retained across parent rebuilds.
    pub fn key(mut self, key: KeyRef) -> Button {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for Button {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Button")
            .field("style", &self.style)
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for Button {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let (content, style, padding) = (self.content.clone(), self.style, self.padding);
        let custom_template = self.template.clone();
        let mut states = CommonStates::new(self.click.clone(), move |app, context, states| {
            if let Some(builder) = &custom_template {
                builder(app, context, states, content.clone())
            } else {
                template(app, context, style, padding, content.clone(), states)
            }
        })
        .is_enabled(self.is_enabled)
        .is_tab_stop(self.is_tab_stop);
        if let Some(node) = self.focus_node {
            states = states.focus_node(node);
        }
        states.into_widget()
    }
}

/// The `ControlTemplate` of the button styles for one set of states.
fn template(
    app: &mut App,
    context: BuildContext,
    style: ButtonStyle,
    padding: Option<[f64; 4]>,
    content: WidgetRef,
    states: ControlStates,
) -> WidgetRef {
    let r = ThemeResources::of(app, context);
    let brushes = style.brushes(&r, states.common);
    let padding = padding.unwrap_or(r.button_padding);
    let corner_radius = r.control_corner_radius[0];
    let text = control_text_style(
        &r,
        r.control_content_theme_font_size,
        FontWeight::W400,
        brushes.foreground,
    );
    // `ContentPresenter`: `HorizontalContentAlignment` and `VerticalContentAlignment` are Center.
    let label = Center::new()
        .width_factor(1.0)
        .height_factor(1.0)
        .child(DefaultTextStyle::new(text, content))
        .into_widget();
    let presenter = ColorTransition::new(
        brushes.background,
        r.control_faster_animation_duration,
        move |_app, background| {
            ControlBorder::new(Brush::Solid(background), brushes.border)
                .border_thickness(BUTTON_BORDER_THICKNESS)
                .corner_radius(corner_radius)
                .background_sizing(style.background_sizing())
                .padding(padding)
                .child(label.clone())
                .into_widget()
        },
    );
    FocusVisual::new(presenter, r.theme)
        .visible(states.focused)
        .margin(BUTTON_FOCUS_VISUAL_MARGIN)
        .corner_radius(corner_radius)
        .into_widget()
}
