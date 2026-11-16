//! XAML `AppBarButton` with the `DefaultAppBarButtonStyle` of `AppBarButton_themeresources.xaml`, and `AppBarButton_Partial.cpp` with `AppBarButtonHelpers.h`.
//!
//! The template is `Root` holding `AppBarButtonInnerBorder`, the focus target with an 83 ms `BrushTransition` on its background, under `ContentRoot`, a grid of three columns and two rows: `ContentViewbox` (the `Icon` scaled to `AppBarButtonContentHeight`), `TextLabel` below it, `OverflowTextLabel`, `KeyboardAcceleratorTextLabel` and `SubItemChevronPanel`. `ApplicationViewStates` moves and hides those parts by label position and overflow (see [`ApplicationViewState`]); `CommonStates` swaps the brushes, with its `Overflow*` states only for an overflow menu that has icons.

use super::element::{
    ApplicationViewState, CommandBarLabelPosition, ElementHost, ElementPresentation,
};
use crate::{
    Brush, ColorTransition, ColumnDefinition, CommonState, CommonStates, ControlBorder,
    ControlStates, FluentSymbol, FocusVisual, FontIcon, Grid, GridCell, GridLength, InputDevice,
    Margin, RowDefinition, TextBlockStyle, ThemeResources, ToolTip, ToolTipService,
    control_text_style,
};
use crate::{Flyout, FlyoutPlacementMode, FlyoutShowOptions, FlyoutTarget};
use std::{fmt, rc::Rc};
use waft_embedder::{Color, FontWeight, TextAlign};
use waft_foundation::{App, Listener, Ref};
use waft_painting::{AlignmentGeometry, EdgeInsetsGeometry, TextStyle};
use waft_rendering::BoxConstraints;
use waft_widgets::*;

/// The `FocusVisualMargin` the style sets.
pub(crate) const APP_BAR_BUTTON_FOCUS_VISUAL_MARGIN: [f64; 4] = [-3.0; 4];
/// The style's `Width`, which `LabelOnRight` and the overflow style clear.
pub(crate) const APP_BAR_BUTTON_WIDTH: f64 = 68.0;

/// XAML `AppBarButton`: a command with an icon and a label, for a `CommandBar`.
#[derive(Clone)]
pub struct AppBarButton {
    /// `Label`: the text under or beside the icon, and the item's text in the overflow.
    pub label: String,
    /// `Click`.
    pub click: Listener,
    /// `Icon`: the glyph or image shown, scaled to the button's icon height.
    pub icon: Option<WidgetRef>,
    /// `LabelPosition`.
    pub label_position: CommandBarLabelPosition,
    /// `Flyout`: opened on click; the bar stays open.
    pub flyout: Option<Ref<Flyout>>,
    /// `IsEnabled`.
    pub is_enabled: bool,
    /// `DynamicOverflowOrder`: when the button moves to the overflow, lowest first; 0 moves with the rightmost commands.
    pub dynamic_overflow_order: i32,
    /// `KeyboardAcceleratorTextOverride`: shown beside the label in the overflow, and in the tooltip in the bar.
    pub keyboard_accelerator_text_override: String,
    pub(crate) presentation: ElementPresentation,
    pub(crate) host: Option<ElementHost>,
    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl AppBarButton {
    /// Creates a button with a `Label` and a `Click` callback.
    pub fn new(label: impl Into<String>, click: Listener) -> Self {
        Self {
            label: label.into(),
            click,
            icon: None,
            label_position: CommandBarLabelPosition::Default,
            flyout: None,
            is_enabled: true,
            dynamic_overflow_order: 0,
            keyboard_accelerator_text_override: String::new(),
            presentation: ElementPresentation::default(),
            host: None,
            key: None,
        }
    }

    /// Sets the XAML `Icon`.
    pub fn icon<K>(mut self, icon: impl IntoWidget<K>) -> Self {
        self.icon = Some(icon.into_widget());
        self
    }

    /// Sets the XAML `LabelPosition`.
    pub fn label_position(mut self, position: CommandBarLabelPosition) -> Self {
        self.label_position = position;
        self
    }

    /// Sets the XAML `Flyout`.
    pub fn flyout(mut self, flyout: Ref<Flyout>) -> Self {
        self.flyout = Some(flyout);
        self
    }

    /// Sets the XAML `IsEnabled`.
    pub fn is_enabled(mut self, enabled: bool) -> Self {
        self.is_enabled = enabled;
        self
    }

    /// Sets the XAML `DynamicOverflowOrder`.
    pub fn dynamic_overflow_order(mut self, order: i32) -> Self {
        self.dynamic_overflow_order = order;
        self
    }

    /// Sets the XAML `KeyboardAcceleratorTextOverride`.
    pub fn keyboard_accelerator_text_override(mut self, text: impl Into<String>) -> Self {
        self.keyboard_accelerator_text_override = text.into();
        self
    }

    /// Sets the identity retained across rebuilds.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }

    /// `AppBarButton::OnClick`, then `Button::OnClick`'s `OpenAssociatedFlyout`: a button without a flyout closes its bar, `Click` is raised, and a flyout opens below the button or, in the overflow, beside it (`OpenSubMenu`).
    fn on_click(&self, target: BuildContext) -> Listener {
        let (click, flyout) = (self.click.clone(), self.flyout.clone());
        let host = self.host.clone();
        let in_overflow = self.presentation.use_overflow_style;
        Listener::new(move |app| {
            if flyout.is_none()
                && let Some(host) = &host
            {
                host.executed.call(app);
            }
            click.call(app);
            if let Some(flyout) = &flyout {
                if in_overflow {
                    let options = FlyoutShowOptions {
                        placement: Some(FlyoutPlacementMode::RightEdgeAlignedTop),
                        show_mode: app.get(flyout).show_mode,
                        ..Default::default()
                    };
                    flyout.show_at_with_options(app, target, options);
                } else {
                    flyout.show_at(app, target);
                }
            }
        })
    }
}

impl fmt::Debug for AppBarButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppBarButton")
            .field("label", &self.label)
            .field("label_position", &self.label_position)
            .field("is_enabled", &self.is_enabled)
            .field("dynamic_overflow_order", &self.dynamic_overflow_order)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for AppBarButton {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.clone();
        FlyoutTarget::new(Builder::new(move |_app, target| {
            let template_widget = widget.clone();
            let mut states =
                CommonStates::new(widget.on_click(target), move |app, context, states| {
                    let sub_menu_open = template_widget
                        .flyout
                        .as_ref()
                        .is_some_and(|flyout| flyout.is_open(app));
                    template(app, context, &template_widget, states, sub_menu_open)
                })
                .is_enabled(widget.is_enabled);
            if let Some(host) = &widget.host {
                states = states.focus_node(host.focus_node.clone());
            }
            let button = match widget.flyout.clone() {
                // `OverflowSubMenuOpened` follows the flyout's opening and closing.
                Some(flyout) if widget.presentation.use_overflow_style => {
                    let states = states.clone();
                    ListenableBuilder::new(Rc::new(flyout), move |_, _, _| {
                        states.clone().into_widget()
                    })
                    .into_widget()
                }
                _ => states.into_widget(),
            };
            with_tool_tip(
                button,
                &widget.label,
                &widget.keyboard_accelerator_text_override,
                &widget.presentation,
            )
        }))
        .into_widget()
    }
}

/// `AppBarButtonHelpers::UpdateToolTip`: in the bar, a button with accelerator text gets the tooltip "Label (Accelerator)" (`KEYBOARD_ACCELERATOR_TEXT_TOOLTIP`).
pub(crate) fn with_tool_tip(
    button: WidgetRef,
    label: &str,
    accelerator: &str,
    presentation: &ElementPresentation,
) -> WidgetRef {
    if presentation.use_overflow_style || accelerator.is_empty() {
        return button;
    }
    ToolTipService::new(button, ToolTip::text(format!("{label} ({accelerator})"))).into_widget()
}

/// `CommonStates` of the template, the base `ButtonBase` states and the `Overflow*` states `ChangeVisualState` adds for an overflow menu with icons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppBarButtonCommonState {
    Normal,
    PointerOver,
    Pressed,
    Disabled,
    OverflowNormal,
    OverflowPointerOver,
    OverflowPressed,
    OverflowSubMenuOpened,
}

impl AppBarButtonCommonState {
    /// `ChangeVisualState`: the base state, then, in an overflow with icons, the overflow state for an enabled button.
    fn of(presentation: &ElementPresentation, states: ControlStates, sub_menu_open: bool) -> Self {
        let base = match states.common {
            CommonState::Normal => Self::Normal,
            CommonState::PointerOver => Self::PointerOver,
            CommonState::Pressed => Self::Pressed,
            CommonState::Disabled => Self::Disabled,
        };
        if !(presentation.use_overflow_style && presentation.with_icons) {
            return base;
        }
        if sub_menu_open {
            Self::OverflowSubMenuOpened
        } else {
            match base {
                Self::Pressed => Self::OverflowPressed,
                Self::PointerOver => Self::OverflowPointerOver,
                Self::Disabled => Self::Disabled,
                _ => Self::OverflowNormal,
            }
        }
    }
}

/// The brushes one `CommonStates` state sets.
#[derive(Clone, Copy, Debug)]
struct AppBarButtonBrushes {
    background: Color,
    border: Color,
    foreground: Color,
    chevron: Color,
    accelerator: Color,
}

fn brushes(r: &ThemeResources, state: AppBarButtonCommonState) -> AppBarButtonBrushes {
    use AppBarButtonCommonState::*;
    let set = |background, border, foreground, chevron, accelerator| AppBarButtonBrushes {
        background,
        border,
        foreground,
        chevron,
        accelerator,
    };
    match state {
        // `Normal` and `OverflowNormal` set no brush: the style's own.
        Normal | OverflowNormal => set(
            r.app_bar_button_background,
            r.app_bar_button_border_brush,
            r.app_bar_button_foreground,
            r.app_bar_button_sub_item_chevron_foreground,
            r.app_bar_button_keyboard_accelerator_text_foreground,
        ),
        PointerOver | OverflowPointerOver => set(
            r.app_bar_button_background_pointer_over,
            r.app_bar_button_border_brush_pointer_over,
            r.app_bar_button_foreground_pointer_over,
            r.app_bar_button_sub_item_chevron_foreground_pointer_over,
            r.app_bar_button_keyboard_accelerator_text_foreground_pointer_over,
        ),
        Pressed | OverflowPressed => set(
            r.app_bar_button_background_pressed,
            r.app_bar_button_border_brush_pressed,
            r.app_bar_button_foreground_pressed,
            r.app_bar_button_sub_item_chevron_foreground_pressed,
            r.app_bar_button_keyboard_accelerator_text_foreground_pressed,
        ),
        Disabled => set(
            r.app_bar_button_background_disabled,
            r.app_bar_button_border_brush_disabled,
            r.app_bar_button_foreground_disabled,
            r.app_bar_button_sub_item_chevron_foreground_disabled,
            r.app_bar_button_keyboard_accelerator_text_foreground_disabled,
        ),
        OverflowSubMenuOpened => set(
            r.app_bar_button_background_sub_menu_opened,
            r.app_bar_button_border_brush_sub_menu_opened,
            r.app_bar_button_foreground_sub_menu_opened,
            r.app_bar_button_sub_item_chevron_foreground_sub_menu_opened,
            r.app_bar_button_keyboard_accelerator_text_foreground_sub_menu_opened,
        ),
    }
}

/// The `ControlTemplate` for one view state and set of common states.
fn template(
    app: &mut App,
    context: BuildContext,
    widget: &AppBarButton,
    states: ControlStates,
    sub_menu_open: bool,
) -> WidgetRef {
    let r = ThemeResources::of(app, context);
    let presentation = widget.presentation;
    let view = ApplicationViewState::of(&presentation, widget.label_position);
    let common = AppBarButtonCommonState::of(&presentation, states, sub_menu_open);
    let brushes = brushes(&r, common);
    let overflow_common = matches!(
        common,
        AppBarButtonCommonState::OverflowNormal
            | AppBarButtonCommonState::OverflowPointerOver
            | AppBarButtonCommonState::OverflowPressed
            | AppBarButtonCommonState::OverflowSubMenuOpened
    );
    let touch = presentation.input_mode == InputDevice::Touch;
    let label_style =
        |size: f64, color: Color| control_text_style(&r, size, FontWeight::NORMAL, color);
    let mut cells = Vec::new();

    // ContentViewbox > Content
    if !matches!(
        view,
        ApplicationViewState::Overflow | ApplicationViewState::OverflowWithToggleButtons
    ) {
        let icon = widget.icon.clone().map(|icon| {
            DefaultTextStyle::new(
                label_style(r.control_content_theme_font_size, brushes.foreground),
                icon,
            )
            .into_widget()
        });
        cells.push(GridCell::new(content_viewbox(&r, view, icon, false)).into_widget());
    }
    // TextLabel
    match view {
        ApplicationViewState::FullSize => cells.push(
            GridCell::new(Margin::new(
                r.app_bar_button_text_label_margin,
                Text::new(widget.label.clone())
                    .style(label_style(
                        r.app_bar_button_label_font_size,
                        brushes.foreground,
                    ))
                    .text_align(TextAlign::Center),
            ))
            .row(1)
            .into_widget(),
        ),
        ApplicationViewState::LabelOnRight => cells.push(
            GridCell::new(Margin::new(
                r.app_bar_button_text_label_on_right_margin,
                Text::new(widget.label.clone())
                    .style(label_style(
                        r.app_bar_button_label_font_size,
                        brushes.foreground,
                    ))
                    .text_align(TextAlign::Left),
            ))
            .column(1)
            .into_widget(),
        ),
        _ => {}
    }
    // OverflowTextLabel
    if view.is_overflow() {
        let margin = match view {
            ApplicationViewState::Overflow => [12.0, 0.0, 12.0, 0.0],
            ApplicationViewState::OverflowWithToggleButtonsAndMenuIcons => [76.0, 0.0, 12.0, 0.0],
            _ => [38.0, 0.0, 12.0, 0.0],
        };
        let padding = if touch {
            r.app_bar_button_overflow_text_touch_margin
        } else {
            r.app_bar_button_overflow_text_label_padding
        };
        cells.push(
            GridCell::new(overflow_text_label(
                &widget.label,
                label_style(r.control_content_theme_font_size, brushes.foreground),
                margin,
                padding,
            ))
            .into_widget(),
        );
    }
    // KeyboardAcceleratorTextLabel
    if let Some(cell) = keyboard_accelerator_text_label(
        &r,
        &presentation,
        &widget.keyboard_accelerator_text_override,
        brushes.accelerator,
    ) {
        cells.push(GridCell::new(cell).column(1).into_widget());
    }
    // SubItemChevronPanel: `HasFlyout` sets `AppBarButtonHasFlyoutChevronVisibility`, which a bar's resources collapse.
    if widget.flyout.is_some() && !presentation.in_command_bar {
        let chevron = if overflow_common {
            // OverflowSubItemChevron
            Margin::new(
                r.app_bar_button_secondary_sub_item_chevron_margin,
                Align::new()
                    .alignment(AlignmentGeometry::CENTER)
                    .height_factor(1.0)
                    .child(
                        FontIcon::symbol(FluentSymbol::ChevronRight)
                            .font_size(r.app_bar_button_secondary_sub_item_chevron_font_size)
                            .mirrored_when_right_to_left(true)
                            .foreground(r.app_bar_button_sub_item_chevron_foreground),
                    ),
            )
        } else {
            // SubItemChevron
            Margin::new(
                if view == ApplicationViewState::LabelOnRight {
                    r.app_bar_button_sub_item_chevron_label_on_right_margin
                } else {
                    r.app_bar_button_sub_item_chevron_margin
                },
                Align::new()
                    .alignment(AlignmentGeometry::TOP_CENTER)
                    .height_factor(1.0)
                    .child(
                        FontIcon::symbol(FluentSymbol::ChevronRight)
                            .font_size(r.app_bar_button_sub_item_chevron_font_size)
                            .mirrored_when_right_to_left(true)
                            .foreground(brushes.chevron),
                    ),
            )
        };
        cells.push(GridCell::new(chevron).column(2).into_widget());
    }

    let content_root =
        ConstrainedBox::new(BoxConstraints::new().min_height(content_root_min_height(&r, view)))
            .child(
                Grid::new()
                    .column_definitions([
                        ColumnDefinition::new(GridLength::STAR),
                        ColumnDefinition::new(GridLength::AUTO),
                        ColumnDefinition::new(GridLength::AUTO),
                    ])
                    .row_definitions([
                        RowDefinition::new(GridLength::AUTO),
                        RowDefinition::new(GridLength::AUTO),
                    ])
                    .children(cells),
            );
    let inner_border = inner_border(
        &r,
        Brush::Solid(brushes.background),
        Brush::Solid(brushes.border),
        0.0,
        crate::BackgroundSizing::InnerBorderEdge,
        states.focused,
    );
    root(
        view,
        Margin::new(inner_border_margin(&r, view), inner_border),
        content_root.into_widget(),
    )
}

/// `AppBarButtonInnerBorder`'s `Margin` in each view state, shared by `AppBarToggleButtonInnerBorder`.
pub(crate) fn inner_border_margin(r: &ThemeResources, view: ApplicationViewState) -> [f64; 4] {
    match view {
        ApplicationViewState::Compact => r.app_bar_button_inner_border_compact_margin,
        view if view.is_overflow() => r.app_bar_button_inner_border_overflow_margin,
        _ => r.app_bar_button_inner_border_margin,
    }
}

/// `ContentRoot.MinHeight` in each view state.
pub(crate) fn content_root_min_height(r: &ThemeResources, view: ApplicationViewState) -> f64 {
    match view {
        ApplicationViewState::LabelOnRight | ApplicationViewState::LabelCollapsed => {
            r.app_bar_theme_compact_height
        }
        view if view.is_overflow() => 0.0,
        _ => r.app_bar_theme_min_height,
    }
}

/// The inner border, the template's focus target: `Background` with its 83 ms `BrushTransition`, `BorderBrush`, `CornerRadius` and the system focus visual at `FocusVisualMargin`.
pub(crate) fn inner_border(
    r: &ThemeResources,
    background: Brush,
    border: Brush,
    border_thickness: f64,
    background_sizing: crate::BackgroundSizing,
    focused: bool,
) -> WidgetRef {
    let corner_radius = r.control_corner_radius;
    let background_color = background.representative_color();
    let presenter = ColorTransition::new(
        background_color,
        r.control_faster_animation_duration,
        move |_app, background| {
            ControlBorder::new(Brush::Solid(background), border)
                .border_thickness(border_thickness)
                .corner_radius_corners(corner_radius)
                .background_sizing(background_sizing)
                .into_widget()
        },
    );
    FocusVisual::new(presenter, r.theme)
        .visible(focused)
        .margin(APP_BAR_BUTTON_FOCUS_VISUAL_MARGIN)
        .corner_radius(corner_radius[0])
        .into_widget()
}

/// `Root`: the inner border under `ContentRoot`, at the style's `Width` of 68 except in `LabelOnRight` and the overflow, where the width is the content's or the menu's.
pub(crate) fn root(
    view: ApplicationViewState,
    inner_border: Margin,
    content_root: WidgetRef,
) -> WidgetRef {
    let root = Grid::new().children([inner_border.into_widget(), content_root]);
    if view == ApplicationViewState::LabelOnRight || view.is_overflow() {
        root.into_widget()
    } else {
        SizedBox::new()
            .width(APP_BAR_BUTTON_WIDTH)
            .child(root)
            .into_widget()
    }
}

/// `ContentViewbox`: the icon scaled to `AppBarButtonContentHeight`, across the column, or a 16 pixel square at the start of an overflow item with icons (`max` the toggle button's `MaxWidth` and `MaxHeight` rather than the button's `Width` and `Height`).
pub(crate) fn content_viewbox(
    r: &ThemeResources,
    view: ApplicationViewState,
    icon: Option<WidgetRef>,
    toggle: bool,
) -> WidgetRef {
    let scaled = |icon: Option<WidgetRef>| match icon {
        Some(icon) => FittedBox::new().child(icon).into_widget(),
        None => SizedBox::shrink().into_widget(),
    };
    match view {
        ApplicationViewState::OverflowWithMenuIcons
        | ApplicationViewState::OverflowWithToggleButtonsAndMenuIcons => {
            let margin = if view == ApplicationViewState::OverflowWithMenuIcons && !toggle {
                [12.0, 0.0, 12.0, 0.0]
            } else {
                [38.0, 0.0, 12.0, 0.0]
            };
            let square = if toggle {
                ConstrainedBox::new(BoxConstraints::new().max_width(16.0).max_height(16.0))
                    .child(scaled(icon))
                    .into_widget()
            } else {
                SizedBox::new()
                    .width(16.0)
                    .height(16.0)
                    .child(scaled(icon))
                    .into_widget()
            };
            Margin::new(
                margin,
                Align::new()
                    .alignment(AlignmentGeometry::CENTER_LEFT)
                    .height_factor(1.0)
                    .child(square),
            )
            .into_widget()
        }
        _ => {
            let margin = if view == ApplicationViewState::LabelOnRight {
                r.app_bar_button_content_viewbox_margin
            } else {
                r.app_bar_button_content_viewbox_collapsed_margin
            };
            Margin::new(
                margin,
                SizedBox::new()
                    .height(r.app_bar_button_content_height)
                    .child(scaled(icon)),
            )
            .into_widget()
        }
    }
}

/// `OverflowTextLabel`: the label on one clipped line, centred down the item, inside its margin and padding.
pub(crate) fn overflow_text_label(
    label: &str,
    style: TextStyle,
    margin: [f64; 4],
    padding: [f64; 4],
) -> WidgetRef {
    let [left, top, right, bottom] = padding;
    Margin::new(
        margin,
        Align::new()
            .alignment(AlignmentGeometry::CENTER_LEFT)
            .height_factor(1.0)
            .child(
                Padding::new(EdgeInsetsGeometry::from_ltrb(left, top, right, bottom)).child(
                    Text::new(label.to_string())
                        .style(style)
                        .max_lines(1)
                        .soft_wrap(false)
                        .text_align(TextAlign::Left),
                ),
            ),
    )
    .into_widget()
}

/// `KeyboardAcceleratorTextLabel`, shown in `KeyboardAcceleratorTextVisible`: an overflow item of a menu with accelerator text, at least `KeyboardAcceleratorTextMinWidth` wide, at the end of the item.
pub(crate) fn keyboard_accelerator_text_label(
    r: &ThemeResources,
    presentation: &ElementPresentation,
    text: &str,
    color: Color,
) -> Option<WidgetRef> {
    if !(presentation.with_keyboard_accelerator_text && presentation.use_overflow_style) {
        return None;
    }
    Some(
        Margin::new(
            [24.0, 0.0, 12.0, 0.0],
            Align::new()
                .alignment(AlignmentGeometry::CENTER_RIGHT)
                .height_factor(1.0)
                .width_factor(1.0)
                .child(
                    ConstrainedBox::new(
                        BoxConstraints::new()
                            .min_width(presentation.keyboard_accelerator_text_min_width),
                    )
                    .child(
                        Text::new(text.to_string())
                            .style(TextBlockStyle::Caption.text_style(r, color))
                            .max_lines(1)
                            .soft_wrap(false),
                    ),
                ),
        )
        .into_widget(),
    )
}
