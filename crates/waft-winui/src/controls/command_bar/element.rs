//! `ICommandBarElement`, `ICommandBarLabeledElement` and `ICommandBarOverflowElement` (`CommandBar.idl`): what a `CommandBar` sets on the `AppBarButton`, `AppBarToggleButton` and `AppBarSeparator` it hosts, and the visual state each one picks from it (`AppBarButtonHelpers::ChangeCommonVisualStates`).
//!
//! An element is a value the application builds; the bar clones it into the primary row and, when it moves there, into the overflow, each copy with its own [`ElementPresentation`]. A later editor must keep the presentation and the [`ElementHost`] all the bar gives an element: an element reads nothing of the bar any other way.

use super::{AppBarButton, AppBarSeparator, AppBarToggleButton};
use crate::InputDevice;
use waft_foundation::Listener;
use waft_widgets::FocusNodeRef;

/// XAML `CommandBarDefaultLabelPosition`: where the bar's buttons show their labels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CommandBarDefaultLabelPosition {
    /// Under the icon, shown while the bar is open.
    #[default]
    Bottom,
    /// To the right of the icon, always shown.
    Right,
    /// Not shown.
    Collapsed,
}

/// XAML `CommandBarLabelPosition`: whether a button follows the bar's label position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CommandBarLabelPosition {
    /// The bar's `DefaultLabelPosition`.
    #[default]
    Default,
    /// No label, whatever the bar's position.
    Collapsed,
}

/// An item of `PrimaryCommands` or `SecondaryCommands`: XAML's `ICommandBarElement`.
#[derive(Clone, Debug)]
pub enum CommandBarElement {
    Button(AppBarButton),
    ToggleButton(AppBarToggleButton),
    Separator(AppBarSeparator),
}

impl From<AppBarButton> for CommandBarElement {
    fn from(button: AppBarButton) -> Self {
        CommandBarElement::Button(button)
    }
}

impl From<AppBarToggleButton> for CommandBarElement {
    fn from(button: AppBarToggleButton) -> Self {
        CommandBarElement::ToggleButton(button)
    }
}

impl From<AppBarSeparator> for CommandBarElement {
    fn from(separator: AppBarSeparator) -> Self {
        CommandBarElement::Separator(separator)
    }
}

impl CommandBarElement {
    /// `ICommandBarElement::DynamicOverflowOrder`.
    pub fn dynamic_overflow_order(&self) -> i32 {
        match self {
            CommandBarElement::Button(button) => button.dynamic_overflow_order,
            CommandBarElement::ToggleButton(button) => button.dynamic_overflow_order,
            CommandBarElement::Separator(separator) => separator.dynamic_overflow_order,
        }
    }

    pub(crate) fn is_separator(&self) -> bool {
        matches!(self, CommandBarElement::Separator(_))
    }

    /// The `Label`, which a separator has none of.
    pub(crate) fn label(&self) -> Option<&str> {
        match self {
            CommandBarElement::Button(button) => Some(&button.label),
            CommandBarElement::ToggleButton(button) => Some(&button.label),
            CommandBarElement::Separator(_) => None,
        }
    }

    pub(crate) fn has_icon(&self) -> bool {
        match self {
            CommandBarElement::Button(button) => button.icon.is_some(),
            CommandBarElement::ToggleButton(button) => button.icon.is_some(),
            CommandBarElement::Separator(_) => false,
        }
    }

    pub(crate) fn keyboard_accelerator_text(&self) -> &str {
        match self {
            CommandBarElement::Button(button) => &button.keyboard_accelerator_text_override,
            CommandBarElement::ToggleButton(button) => &button.keyboard_accelerator_text_override,
            CommandBarElement::Separator(_) => "",
        }
    }

    pub(crate) fn is_focusable(&self) -> bool {
        match self {
            CommandBarElement::Button(button) => button.is_enabled,
            CommandBarElement::ToggleButton(button) => button.is_enabled,
            CommandBarElement::Separator(_) => false,
        }
    }

    /// `ICommandBarLabeledElement::GetHasBottomLabel` and `GetHasRightLabel`: a labelled button whose effective label position is `position`.
    pub(crate) fn has_label_at(
        &self,
        default: CommandBarDefaultLabelPosition,
        position: CommandBarDefaultLabelPosition,
    ) -> bool {
        let (label, label_position) = match self {
            CommandBarElement::Button(button) => (&button.label, button.label_position),
            CommandBarElement::ToggleButton(button) => (&button.label, button.label_position),
            CommandBarElement::Separator(_) => return false,
        };
        effective_label_position(label_position, default) == position && !label.is_empty()
    }

    /// The element as the bar hosts it, with `presentation` and the bar's `host` hooks.
    pub(crate) fn hosted(
        &self,
        presentation: ElementPresentation,
        host: ElementHost,
    ) -> CommandBarElement {
        match self.clone() {
            CommandBarElement::Button(mut button) => {
                button.presentation = presentation;
                button.host = Some(host);
                CommandBarElement::Button(button)
            }
            CommandBarElement::ToggleButton(mut button) => {
                button.presentation = presentation;
                button.host = Some(host);
                CommandBarElement::ToggleButton(button)
            }
            CommandBarElement::Separator(mut separator) => {
                separator.presentation = presentation;
                CommandBarElement::Separator(separator)
            }
        }
    }

    /// What the bar compares between builds to tell whether its commands changed: each element's kind, label and `DynamicOverflowOrder`.
    pub(crate) fn signature(&self) -> (u8, Option<String>, i32) {
        let kind = match self {
            CommandBarElement::Button(_) => 0,
            CommandBarElement::ToggleButton(_) => 1,
            CommandBarElement::Separator(_) => 2,
        };
        (
            kind,
            self.label().map(str::to_string),
            self.dynamic_overflow_order(),
        )
    }
}

/// `GetEffectiveLabelPosition`: a `Collapsed` button has no label; any other follows the bar.
pub(crate) fn effective_label_position(
    label_position: CommandBarLabelPosition,
    default: CommandBarDefaultLabelPosition,
) -> CommandBarDefaultLabelPosition {
    match label_position {
        CommandBarLabelPosition::Collapsed => CommandBarDefaultLabelPosition::Collapsed,
        CommandBarLabelPosition::Default => default,
    }
}

/// What a hosting bar sets on an element: `IsCompact`, `UseOverflowStyle`, `SetDefaultLabelPosition`, `SetOverflowStyleParams`, `SetInputMode` and the `KeyboardAcceleratorTextMinWidth` template setting. An element outside a bar has the default: full size, labels at the bottom.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ElementPresentation {
    /// `IsCompact`: the bar is closed, so a bottom label is hidden.
    pub(crate) is_compact: bool,
    /// `UseOverflowStyle`: the element is in the overflow menu.
    pub(crate) use_overflow_style: bool,
    /// The bar's `DefaultLabelPosition`.
    pub(crate) default_label_position: CommandBarDefaultLabelPosition,
    /// `m_isWithIcons`: an overflow item has an icon.
    pub(crate) with_icons: bool,
    /// `m_isWithToggleButtons`: an overflow item is a toggle button.
    pub(crate) with_toggle_buttons: bool,
    /// `m_isWithKeyboardAcceleratorText`: an overflow item shows accelerator text.
    pub(crate) with_keyboard_accelerator_text: bool,
    /// `m_inputDeviceTypeUsedToOpenOverflow`.
    pub(crate) input_mode: InputDevice,
    /// `TemplateSettings.KeyboardAcceleratorTextMinWidth`: the widest accelerator text in the overflow.
    pub(crate) keyboard_accelerator_text_min_width: f64,
    /// The bar's `AppBarButtonHasFlyoutChevronVisibility`, `Collapsed`, overrides the button's own.
    pub(crate) in_command_bar: bool,
}

/// `ApplicationViewStates`, from `ChangeCommonVisualStates` for an element outside the overflow and from `ChangeVisualState` for one in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ApplicationViewState {
    FullSize,
    Compact,
    LabelOnRight,
    LabelCollapsed,
    Overflow,
    OverflowWithToggleButtons,
    OverflowWithMenuIcons,
    OverflowWithToggleButtonsAndMenuIcons,
}

impl ApplicationViewState {
    /// The state of a button with `label_position`, which `AppBarToggleButton` reaches only without toggle buttons, as it reports none.
    pub(crate) fn of(
        presentation: &ElementPresentation,
        label_position: CommandBarLabelPosition,
    ) -> ApplicationViewState {
        if presentation.use_overflow_style {
            return match (presentation.with_toggle_buttons, presentation.with_icons) {
                (true, true) => ApplicationViewState::OverflowWithToggleButtonsAndMenuIcons,
                (true, false) => ApplicationViewState::OverflowWithToggleButtons,
                (false, true) => ApplicationViewState::OverflowWithMenuIcons,
                (false, false) => ApplicationViewState::Overflow,
            };
        }
        match effective_label_position(label_position, presentation.default_label_position) {
            CommandBarDefaultLabelPosition::Right => ApplicationViewState::LabelOnRight,
            CommandBarDefaultLabelPosition::Collapsed => ApplicationViewState::LabelCollapsed,
            CommandBarDefaultLabelPosition::Bottom if presentation.is_compact => {
                ApplicationViewState::Compact
            }
            CommandBarDefaultLabelPosition::Bottom => ApplicationViewState::FullSize,
        }
    }

    pub(crate) fn is_overflow(self) -> bool {
        matches!(
            self,
            ApplicationViewState::Overflow
                | ApplicationViewState::OverflowWithToggleButtons
                | ApplicationViewState::OverflowWithMenuIcons
                | ApplicationViewState::OverflowWithToggleButtonsAndMenuIcons
        )
    }
}

/// What a bar hands an element it hosts besides its presentation.
#[derive(Clone, Debug)]
pub(crate) struct ElementHost {
    /// `CommandBar::OnCommandExecutionStatic`: the click closes the bar, before the element's own `Click`.
    pub(crate) executed: Listener,
    /// The node the bar moves the keyboard focus with.
    pub(crate) focus_node: FocusNodeRef,
}
