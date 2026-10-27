//! DropDownButton.xaml: Button interaction with a content column and dropdown chevron.

use crate::*;
use inset_embedder::FontWeight;
use inset_foundation::{App, Handle, Listener};
use inset_painting::EdgeInsetsGeometry;
use inset_widgets::*;
use std::fmt;

/// A button that displays a chevron and opens its associated flyout after Click.
#[derive(Clone)]
pub struct DropDownButton {
    /// Content in the first template column.
    pub content: WidgetRef,

    /// Associated persistent flyout, owned and disposed by the caller.
    pub flyout: Option<Handle<Flyout>>,

    /// Called before opening the associated flyout.
    pub click: Option<Listener>,

    /// Whether pointer and keyboard input can activate the button.
    pub is_enabled: bool,

    /// Whether keyboard traversal stops at the button.
    pub is_tab_stop: bool,

    /// Optional caller-owned focus node.
    pub focus_node: Option<AnyFocusNode>,

    /// Widget identity across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl fmt::Debug for DropDownButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DropDownButton")
            .field("flyout", &self.flyout)
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

impl DropDownButton {
    /// Creates a button with the default DropDownButton template.
    pub fn new<K>(content: impl IntoWidget<K>) -> Self {
        Self {
            content: content.into_widget(),
            flyout: None,
            click: None,
            is_enabled: true,
            is_tab_stop: true,
            focus_node: None,
            key: None,
        }
    }

    /// Creates a button with a text label.
    pub fn text(text: impl Into<String>) -> Self {
        Self::new(Text::new(text))
    }

    /// Associates a reusable flyout with the button.
    pub fn flyout(mut self, flyout: Handle<Flyout>) -> Self {
        self.flyout = Some(flyout);
        self
    }

    /// Sets the callback raised before opening the flyout.
    pub fn click(mut self, click: Listener) -> Self {
        self.click = Some(click);
        self
    }

    /// Enables or disables activation.
    pub fn is_enabled(mut self, value: bool) -> Self {
        self.is_enabled = value;
        self
    }

    /// Includes or excludes this button from sequential keyboard traversal.
    pub fn is_tab_stop(mut self, value: bool) -> Self {
        self.is_tab_stop = value;
        self
    }

    /// Supplies a caller-owned focus node.
    pub fn focus_node(mut self, node: AnyFocusNode) -> Self {
        self.focus_node = Some(node);
        self
    }

    /// Sets widget identity.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }
}

impl StatelessWidget for DropDownButton {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _: &mut App, _: BuildContext) -> WidgetRef {
        let widget = self.clone();
        FlyoutTarget::new(Builder::new(move |_, context| {
            let flyout = widget.flyout;
            let click = widget.click.clone();
            let mut button = Button::new(
                widget.content.clone(),
                Listener::new(move |app| {
                    if let Some(click) = &click {
                        click.call(app);
                    }
                    if let Some(flyout) = flyout {
                        flyout.show_at(app, context);
                    }
                }),
            )
            .is_enabled(widget.is_enabled)
            .is_tab_stop(widget.is_tab_stop)
            .template(template);
            if let Some(focus) = widget.focus_node {
                button = button.focus_node(focus);
            }
            button.into_widget()
        }))
        .into_widget()
    }
}

/// RootGrid, ContentPresenter and ChevronIcon in DefaultDropDownButtonStyle.
fn template(
    app: &mut App,
    context: BuildContext,
    states: ControlStates,
    content: WidgetRef,
) -> WidgetRef {
    let theme = ThemeResources::of(app, context);
    let resources = &theme;
    let brushes = ButtonStyle::Default.brushes(&theme, states.common);
    let chevron = match states.common {
        CommonState::Normal => resources.drop_down_button_foreground_secondary,
        CommonState::PointerOver => resources.drop_down_button_foreground_secondary_pointer_over,
        CommonState::Pressed => resources.drop_down_button_foreground_secondary_pressed,
        CommonState::Disabled => resources.button_foreground_disabled,
    };
    let content = Grid::new()
        .column_definitions([
            ColumnDefinition::new(GridLength::STAR),
            ColumnDefinition::new(GridLength::AUTO),
        ])
        .children([
            GridCell::new(Center::new().width_factor(1.0).height_factor(1.0).child(
                DefaultTextStyle::new(
                    control_text_style(
                        &theme,
                        theme.control_content_theme_font_size,
                        FontWeight::NORMAL,
                        brushes.foreground,
                    ),
                    content,
                ),
            ))
            .into_widget(),
            GridCell::new(
                Padding::new(EdgeInsetsGeometry::from_ltrb(8.0, 0.0, 0.0, 0.0)).child(
                    SizedBox::new().width(12.0).height(12.0).child(
                        Center::new().child(
                            FontIcon::symbol(FluentSymbol::ChevronDown)
                                .font_size(8.0)
                                .foreground(chevron),
                        ),
                    ),
                ),
            )
            .column(1)
            .into_widget(),
        ])
        .into_widget();
    let (corner_radius, padding) = (theme.control_corner_radius, theme.button_padding);
    let root = ColorTransition::new(
        brushes.background,
        theme.control_faster_animation_duration,
        move |_, background| {
            ControlBorder::new(Brush::Solid(background), brushes.border)
                .border_thickness(BUTTON_BORDER_THICKNESS)
                .corner_radius_corners(corner_radius)
                .padding(padding)
                .child(content.clone())
                .into_widget()
        },
    );
    FocusVisual::new(root, theme.theme)
        .visible(states.focused)
        .margin(BUTTON_FOCUS_VISUAL_MARGIN)
        .corner_radius(corner_radius[0])
        .into_widget()
}
