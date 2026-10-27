//! The Fluent MenuFlyout item templates, including their shared placeholder columns.

use super::item::{MenuFlyoutItem, MenuFlyoutItemKind};
use crate::*;
use inset_embedder::{Color, FontWeight};
use inset_foundation::App;
use inset_painting::{Alignment, TextOverflow};
use inset_widgets::*;

/// MenuFlyoutPresenter's collection-dependent template settings.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct MenuTemplateSettings {
    /// At least one sibling reserves the check column.
    pub has_checks: bool,

    /// At least one sibling reserves the icon column.
    pub has_icons: bool,

    /// Every command reserves space when one sibling displays a shortcut.
    pub has_accelerators: bool,

    /// Largest shortcut-label width among siblings.
    pub accelerator_width: f64,

    /// Mouse, pen and keyboard openings use narrow padding.
    pub narrow: bool,
}

/// CommonStates plus the SubMenuOpened visual state.
#[derive(Clone, Copy, Debug)]
pub(super) struct MenuVisualState {
    /// Pointer and enabled state.
    pub common: CommonState,

    /// Native keyboard-focus highlighting.
    pub focused: bool,

    /// Whether this item's cascading menu is open.
    pub submenu_open: bool,
}

/// The source template's current brushes.
struct MenuBrushes {
    /// LayoutRoot fill.
    background: Color,

    /// TextBlock and IconContent foreground.
    foreground: Color,

    /// SubItemChevron foreground.
    chevron: Color,

    /// KeyboardAcceleratorTextBlock foreground.
    accelerator: Color,

    /// CheckGlyph foreground, whose source states differ from the label.
    check: Color,
}

/// Resolves the named setters in the three source item templates.
fn brushes(r: &ThemeResources, item: &MenuFlyoutItem, state: MenuVisualState) -> MenuBrushes {
    let index = match state.common {
        CommonState::Normal => 0,
        CommonState::PointerOver => 1,
        CommonState::Pressed => 2,
        CommonState::Disabled => 3,
    };
    let submenu = matches!(item.kind, MenuFlyoutItemKind::SubItem { .. });
    let radio = matches!(item.kind, MenuFlyoutItemKind::Radio { .. });
    let foreground = if submenu {
        [
            r.menu_flyout_sub_item_foreground,
            r.menu_flyout_sub_item_foreground_pointer_over,
            r.menu_flyout_sub_item_foreground_pressed,
            r.menu_flyout_sub_item_foreground_disabled,
        ][index]
    } else {
        [
            r.menu_flyout_item_foreground,
            r.menu_flyout_item_foreground_pointer_over,
            r.menu_flyout_item_foreground_pressed,
            r.menu_flyout_item_foreground_disabled,
        ][index]
    };
    let background = if submenu {
        [
            r.menu_flyout_sub_item_background,
            r.menu_flyout_sub_item_background_pointer_over,
            r.menu_flyout_sub_item_background_pressed,
            r.menu_flyout_sub_item_background_disabled,
        ][index]
    } else if radio {
        [
            r.menu_flyout_item_background,
            r.menu_flyout_sub_item_background_pointer_over,
            r.menu_flyout_sub_item_background_pressed,
            r.menu_flyout_item_background,
        ][index]
    } else {
        [
            r.menu_flyout_item_background,
            r.menu_flyout_item_background_pointer_over,
            r.menu_flyout_item_background_pressed,
            r.menu_flyout_item_background_disabled,
        ][index]
    };
    let chevron = [
        r.menu_flyout_sub_item_chevron,
        r.menu_flyout_sub_item_chevron_pointer_over,
        r.menu_flyout_sub_item_chevron_pressed,
        r.menu_flyout_sub_item_chevron_disabled,
    ][index];
    let checked_variant = matches!(
        item.kind,
        MenuFlyoutItemKind::Toggle { .. } | MenuFlyoutItemKind::Radio { .. }
    );
    let accelerator = if checked_variant {
        [
            r.toggle_menu_flyout_item_keyboard_accelerator_text_foreground,
            r.toggle_menu_flyout_item_keyboard_accelerator_text_foreground_pointer_over,
            r.toggle_menu_flyout_item_keyboard_accelerator_text_foreground_pressed,
            r.toggle_menu_flyout_item_keyboard_accelerator_text_foreground_disabled,
        ][index]
    } else {
        [
            r.menu_flyout_item_keyboard_accelerator_text_foreground,
            r.menu_flyout_item_keyboard_accelerator_text_foreground_pointer_over,
            r.menu_flyout_item_keyboard_accelerator_text_foreground_pressed,
            r.menu_flyout_item_keyboard_accelerator_text_foreground_disabled,
        ][index]
    };
    if submenu && state.submenu_open && state.common != CommonState::Disabled {
        MenuBrushes {
            background: r.menu_flyout_sub_item_background_sub_menu_opened,
            foreground: r.menu_flyout_sub_item_foreground_sub_menu_opened,
            chevron: r.menu_flyout_sub_item_chevron_sub_menu_opened,
            accelerator,
            check: r.menu_flyout_sub_item_chevron,
        }
    } else {
        MenuBrushes {
            background,
            foreground,
            chevron,
            accelerator,
            check: if radio && state.common != CommonState::Normal {
                foreground
            } else {
                r.menu_flyout_sub_item_chevron
            },
        }
    }
}

/// Builds the separator rectangle with the source signed margins.
pub(super) fn separator(app: &mut App, context: BuildContext) -> WidgetRef {
    let resources = ThemeResources::of(app, context);
    Margin::new(
        resources.menu_flyout_separator_theme_padding,
        SizedBox::new()
            .height(resources.menu_flyout_separator_height)
            .child(ColoredBox::new(resources.menu_flyout_separator_background)),
    )
    .into_widget()
}

/// LayoutRoot, optional CheckGlyph, IconRoot, TextBlock and accelerator/chevron columns.
pub(super) fn item_template(
    app: &mut App,
    context: BuildContext,
    item: &MenuFlyoutItem,
    settings: MenuTemplateSettings,
    state: MenuVisualState,
) -> WidgetRef {
    let theme = ThemeResources::of(app, context);
    let colors = brushes(&theme, item, state);
    let checked = match item.kind {
        MenuFlyoutItemKind::Toggle { is_checked, .. }
        | MenuFlyoutItemKind::Radio { is_checked, .. } => Some(is_checked),
        _ => None,
    };
    let submenu = matches!(item.kind, MenuFlyoutItemKind::SubItem { .. });
    let first_column = usize::from(checked.is_some());
    let mut columns = Vec::new();
    if checked.is_some() {
        columns.push(ColumnDefinition::new(GridLength::AUTO));
    }
    columns.extend([
        ColumnDefinition::new(GridLength::STAR),
        ColumnDefinition::new(GridLength::AUTO),
    ]);
    let text_margin = if checked.is_some() {
        if settings.has_icons {
            theme.menu_flyout_item_placeholder_theme_thickness
        } else {
            [0.0; 4]
        }
    } else if settings.has_checks && settings.has_icons {
        theme.menu_flyout_item_double_placeholder_theme_thickness
    } else if settings.has_checks || settings.has_icons {
        theme.menu_flyout_item_placeholder_theme_thickness
    } else {
        [0.0; 4]
    };
    let mut children = Vec::new();
    if let Some(checked) = checked {
        children.push(
            GridCell::new(Margin::new(
                [0.0, 0.0, 16.0, 0.0],
                Opacity::new(if checked { 1.0 } else { 0.0 }).child(
                    FontIcon::symbol(if matches!(item.kind, MenuFlyoutItemKind::Radio { .. }) {
                        FluentSymbol::RadioButton
                    } else {
                        FluentSymbol::Checkmark
                    })
                    .font_size(12.0)
                    .foreground(colors.check),
                ),
            ))
            .into_widget(),
        );
    }
    if settings.has_icons {
        let icon_margin = if checked.is_none() && settings.has_checks {
            theme.menu_flyout_item_placeholder_theme_thickness
        } else {
            [0.0; 4]
        };
        children.push(
            GridCell::new(Margin::new(
                icon_margin,
                Align::new()
                    .alignment(Alignment::CENTER_LEFT.into())
                    .width_factor(1.0)
                    .height_factor(1.0)
                    .child(
                        SizedBox::new().width(16.0).height(16.0).child(
                            FittedBox::new().child(
                                item.icon
                                    .clone()
                                    .unwrap_or_else(|| SizedBox::shrink().into_widget()),
                            ),
                        ),
                    ),
            ))
            .column(first_column)
            .into_widget(),
        );
    }
    children.push(
        GridCell::new(Margin::new(
            text_margin,
            Align::new()
                .alignment(Alignment::CENTER_LEFT.into())
                .width_factor(1.0)
                .height_factor(1.0)
                .child(
                    Text::new(item.text.clone())
                        .soft_wrap(false)
                        .overflow(TextOverflow::Clip),
                ),
        ))
        .column(first_column)
        .into_widget(),
    );
    if submenu {
        children.push(
            GridCell::new(Margin::new(
                theme.menu_flyout_item_chevron_margin,
                FontIcon::symbol(FluentSymbol::ChevronRight)
                    .font_size(12.0)
                    .foreground(colors.chevron)
                    .mirrored_when_right_to_left(true),
            ))
            .column(first_column + 1)
            .into_widget(),
        );
    } else if settings.has_accelerators {
        children.push(
            GridCell::new(Margin::new(
                [24.0, if checked.is_some() { 0.0 } else { 4.0 }, 0.0, 0.0],
                ConstrainedBox::new(
                    inset_rendering::BoxConstraints::new().min_width(settings.accelerator_width),
                )
                .child(
                    Align::new()
                        .alignment(Alignment::CENTER_RIGHT.into())
                        .width_factor(1.0)
                        .height_factor(1.0)
                        .child(
                            Text::new(item.keyboard_accelerator_text_override.clone())
                                .style(
                                    TextBlockStyle::Caption.text_style(&theme, colors.accelerator),
                                )
                                .soft_wrap(false),
                        ),
                ),
            ))
            .column(first_column + 1)
            .into_widget(),
        );
    }
    let layout = DefaultTextStyle::new(
        control_text_style(
            &theme,
            theme.control_content_theme_font_size,
            FontWeight::NORMAL,
            colors.foreground,
        ),
        Grid::new().column_definitions(columns).children(children),
    );
    Margin::new(
        theme.menu_flyout_item_margin,
        FocusVisual::new(
            ControlBorder::new(Brush::Solid(colors.background), Brush::Solid(Color::new(0)))
                .border_thickness_ltrb(theme.menu_flyout_item_border_thickness)
                .corner_radius_corners(theme.control_corner_radius)
                .padding(if settings.narrow {
                    theme.menu_flyout_item_theme_padding_narrow
                } else {
                    theme.menu_flyout_item_theme_padding
                })
                .child(layout),
            theme.theme,
        )
        .visible(state.focused)
        .corner_radius(theme.control_corner_radius[0]),
    )
    .into_widget()
}
