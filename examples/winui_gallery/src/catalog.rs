//! One navigation entry per feature, following the Cupertino gallery's explicit catalog.

use inset_winui::FluentSymbol;

/// A gallery destination with its own retained examples and scroll position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    /// Application menu headers and keyboard navigation.
    MenuBar,
    BreadcrumbBar,
    /// Independent primary and flyout actions.
    SplitButton,
    /// A checked primary action beside a flyout button.
    ToggleSplitButton,
    /// Commands, checks, radio choices and nested menus.
    MenuFlyout,
    /// Reusable anchored content and dismissal policies.
    Flyout,
    /// Modal questions with command buttons and a result.
    ContentDialog,
    /// Buttons that open an associated flyout.
    DropDownButton,
    /// Determinate and animated progress states.
    ProgressBar,
    /// A ring that fills to a value or turns while work lasts.
    ProgressRing,
    /// Expandable content in either direction.
    Expander,
    /// Inline notifications and cancelable closing.
    InfoBar,
    /// Counts, icons and dots that mark status.
    InfoBadge,
    /// Standard, accent and disabled buttons.
    Button,
    /// Plain and multiline text entry.
    TextBox,
    /// Concealed entry and password reveal modes.
    PasswordBox,
    /// Numbers, expressions and spin buttons.
    NumberBox,
    /// Choosing one item from a drop-down list, or typing it.
    ComboBox,
    /// Switch content, headers and dragging.
    ToggleSwitch,
    /// Pixel, auto and star layout.
    Grid,
    /// Two-state and three-state check boxes.
    CheckBox,
    /// Mutually exclusive choices.
    RadioButton,
    /// Buttons that retain a checked state.
    ToggleButton,
    /// Repeated activation while held.
    RepeatButton,
    /// Link presentation and activation.
    HyperlinkButton,
    /// Continuous and stepped values.
    Slider,
    /// A color from a spectrum, sliders, channels or hex.
    ColorPicker,
    /// Inline and overlay panes.
    SplitView,
    /// Backdrop blur and tint with sharp foreground content.
    Acrylic,
    /// Pointer and keyboard help.
    ToolTip,
    /// Tab selection, closing and dragging.
    TabView,
    /// The available text styles and foreground hierarchy.
    Typography,
    /// Fluent symbol sizes, colors and mirroring.
    Icons,
    /// Adaptive panes, hierarchy and top navigation.
    NavigationView,
    /// Nested nodes to expand, select and reorder.
    TreeView,
    /// A few views of one page, one at a time.
    SelectorBar,
}

impl Feature {
    /// Display order in the navigation pane.
    pub const ALL: [Self; 36] = [
        Self::Button,
        Self::DropDownButton,
        Self::SplitButton,
        Self::ToggleSplitButton,
        Self::MenuFlyout,
        Self::MenuBar,
        Self::BreadcrumbBar,
        Self::Flyout,
        Self::ContentDialog,
        Self::TextBox,
        Self::PasswordBox,
        Self::NumberBox,
        Self::ComboBox,
        Self::ToggleSwitch,
        Self::Grid,
        Self::CheckBox,
        Self::RadioButton,
        Self::ToggleButton,
        Self::RepeatButton,
        Self::HyperlinkButton,
        Self::Slider,
        Self::ColorPicker,
        Self::ProgressBar,
        Self::ProgressRing,
        Self::Expander,
        Self::InfoBar,
        Self::InfoBadge,
        Self::SplitView,
        Self::Acrylic,
        Self::ToolTip,
        Self::TabView,
        Self::NavigationView,
        Self::TreeView,
        Self::SelectorBar,
        Self::Typography,
        Self::Icons,
    ];

    /// The navigation label and stable destination identity.
    pub fn title(self) -> &'static str {
        match self {
            Self::SplitButton => "SplitButton",
            Self::ToggleSplitButton => "ToggleSplitButton",
            Self::MenuFlyout => "MenuFlyout",
            Self::MenuBar => "MenuBar",
            Self::BreadcrumbBar => "BreadcrumbBar",
            Self::Flyout => "Flyout",
            Self::ContentDialog => "ContentDialog",
            Self::DropDownButton => "DropDownButton",
            Self::Button => "Button",
            Self::TextBox => "TextBox",
            Self::PasswordBox => "PasswordBox",
            Self::NumberBox => "NumberBox",
            Self::ComboBox => "ComboBox",
            Self::ToggleSwitch => "ToggleSwitch",
            Self::Grid => "Grid",
            Self::CheckBox => "CheckBox",
            Self::RadioButton => "RadioButton",
            Self::ToggleButton => "ToggleButton",
            Self::RepeatButton => "RepeatButton",
            Self::HyperlinkButton => "HyperlinkButton",
            Self::Slider => "Slider",
            Self::ColorPicker => "ColorPicker",
            Self::ProgressBar => "ProgressBar",
            Self::ProgressRing => "ProgressRing",
            Self::Expander => "Expander",
            Self::InfoBar => "InfoBar",
            Self::InfoBadge => "InfoBadge",
            Self::SplitView => "SplitView",
            Self::Acrylic => "Acrylic",
            Self::ToolTip => "ToolTip",
            Self::TabView => "TabView",
            Self::NavigationView => "NavigationView",
            Self::TreeView => "TreeView",
            Self::SelectorBar => "SelectorBar",
            Self::Typography => "Typography",
            Self::Icons => "Icons",
        }
    }

    /// A distinct bundled Fluent symbol keeps each destination recognizable in compact mode.
    pub fn symbol(self) -> FluentSymbol {
        match self {
            Self::SplitButton => FluentSymbol::Add,
            Self::ToggleSplitButton => FluentSymbol::Checkmark,
            Self::MenuFlyout => FluentSymbol::More,
            Self::MenuBar => FluentSymbol::MenuList,
            Self::BreadcrumbBar => FluentSymbol::ChevronLeft,
            Self::Flyout => FluentSymbol::ChevronRight,
            Self::ContentDialog => FluentSymbol::Warning,
            Self::DropDownButton => FluentSymbol::ChevronDown,
            Self::Button => FluentSymbol::CursorClick,
            Self::TextBox => FluentSymbol::TextField,
            Self::PasswordBox => FluentSymbol::LockClosed,
            Self::NumberBox => FluentSymbol::ChevronUpDown,
            Self::ComboBox => FluentSymbol::CheckmarkCircle,
            Self::ToggleSwitch => FluentSymbol::ToggleLeft,
            Self::Grid => FluentSymbol::Grid,
            Self::CheckBox => FluentSymbol::CheckboxChecked,
            Self::RadioButton => FluentSymbol::RadioButton,
            Self::ToggleButton => FluentSymbol::CheckboxIndeterminate,
            Self::RepeatButton => FluentSymbol::ArrowRepeatAll,
            Self::HyperlinkButton => FluentSymbol::Link,
            Self::Slider => FluentSymbol::Options,
            Self::ColorPicker => FluentSymbol::Color,
            Self::ProgressBar => FluentSymbol::Timer,
            Self::ProgressRing => FluentSymbol::SpinnerIos,
            Self::Expander => FluentSymbol::ChevronDownUp,
            Self::InfoBar => FluentSymbol::Info,
            Self::InfoBadge => FluentSymbol::Circle,
            Self::SplitView => FluentSymbol::PanelLeft,
            Self::Acrylic => FluentSymbol::Layer,
            Self::ToolTip => FluentSymbol::TooltipQuote,
            Self::TabView => FluentSymbol::Tab,
            Self::NavigationView => FluentSymbol::Navigation,
            Self::TreeView => FluentSymbol::TextBulletListTree,
            Self::SelectorBar => FluentSymbol::Tabs,
            Self::Typography => FluentSymbol::TextFont,
            Self::Icons => FluentSymbol::Apps,
        }
    }

    /// Brief guidance above the examples.
    pub fn description(self) -> &'static str {
        match self {
            Self::SplitButton => "Combine a primary action with a menu of related choices.",
            Self::ToggleSplitButton => "Toggle an action while keeping its menu independent.",
            Self::MenuFlyout => "Choose commands and settings from a hierarchical menu.",
            Self::BreadcrumbBar => {
                "Navigate a path and keep earlier destinations in the overflow menu."
            }
            Self::MenuBar => {
                "Organize application commands into menus with pointer and keyboard navigation."
            }
            Self::Flyout => "Show reusable content beside its opening control.",
            Self::ContentDialog => "Ask a modal question with up to three command buttons.",
            Self::DropDownButton => "Open a flyout from a button with a dropdown chevron.",
            Self::Button => "Activate an action using the pointer or keyboard.",
            Self::TextBox => "Enter and edit text, select words, and explore multiline input.",
            Self::PasswordBox => {
                "Enter a password and compare Peek, Hidden and Visible reveal modes."
            }
            Self::NumberBox => {
                "Enter a number or an expression, and step it with keys, the wheel or spin buttons."
            }
            Self::ComboBox => {
                "Choose one item from a drop-down list with the pointer or the keyboard, or type it."
            }
            Self::ToggleSwitch => "Switch settings on and off, or drag the thumb.",
            Self::Grid => "Arrange content with fixed, content-sized and proportional tracks.",
            Self::CheckBox => "Choose independent options, including an indeterminate state.",
            Self::RadioButton => "Choose one option from a group.",
            Self::ToggleButton => "Keep an action checked until it is toggled again.",
            Self::RepeatButton => "Hold the button to repeat its action.",
            Self::HyperlinkButton => "Present an action as a text link.",
            Self::Slider => "Adjust a value continuously or in steps.",
            Self::ColorPicker => {
                "Choose a color on a spectrum, with sliders, or by its channels and hex value."
            }
            Self::ProgressBar => {
                "Show completion or ongoing work, including paused and error states."
            }
            Self::ProgressRing => "Show progress as a ring, filled to a value or turning.",
            Self::Expander => "Reveal related content above or below a header.",
            Self::InfoBar => "Show inline feedback with severity, actions and cancelable closing.",
            Self::InfoBadge => "Mark status with a count, an icon or a dot.",
            Self::SplitView => "Explore inline, compact and overlay pane layouts.",
            Self::Acrylic => {
                "Blur and tint the scene behind a surface while keeping its content sharp."
            }
            Self::ToolTip => "Hover or focus a control to reveal its help text.",
            Self::TabView => "Add, close, reorder and move tabs between strips.",
            Self::NavigationView => "Explore pane modes, nested destinations and top overflow.",
            Self::TreeView => {
                "Expand, select and reorder nested items, one at a time or with check boxes."
            }
            Self::SelectorBar => "Switch between a few views of the same page.",
            Self::Typography => "Choose text styles and build a clear hierarchy.",
            Self::Icons => "Use Fluent symbols in navigation, actions and status indicators.",
        }
    }
}
