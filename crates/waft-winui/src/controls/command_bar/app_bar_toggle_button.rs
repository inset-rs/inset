//! XAML `AppBarToggleButton` with the `DefaultAppBarToggleButtonStyle` of `AppBarToggleButton_themeresources.xaml`, and `AppBarToggleButton_Partial.cpp` with `AppBarButtonHelpers.h`.
//!
//! The template is `Root` holding `AppBarToggleButtonInnerBorder` under `ContentRoot`, a grid of two columns and two rows: `OverflowCheckGlyph`, `ContentViewbox`, `TextLabel`, `OverflowTextLabel` and `KeyboardAcceleratorTextLabel`. `ApplicationViewStates` is the button's (see [`ApplicationViewState`]) without the toggle-button variants. `CommonStates` holds the unchecked and `Checked*` states `ToggleButton::ChangeVisualState` picks and the `Overflow*` states `AppBarToggleButton::ChangeVisualState` then picks for an enabled button in the overflow; it has no `Indeterminate*` states, so an indeterminate button keeps the look of the state it was last in, and a later editor must keep that memory in the widget's state.

use super::app_bar_button::{
    content_root_min_height, content_viewbox, inner_border, inner_border_margin,
    keyboard_accelerator_text_label, overflow_text_label, root, with_tool_tip,
};
use super::element::{
    ApplicationViewState, CommandBarLabelPosition, ElementHost, ElementPresentation,
};
use crate::{
    BackgroundSizing, Brush, ColumnDefinition, CommonState, CommonStates, ControlStates,
    FluentSymbol, FontIcon, Grid, GridCell, GridLength, InputDevice, Margin, RowDefinition,
    ThemeResources, control_text_style, next_is_checked,
};
use std::{fmt, rc::Rc};
use waft_embedder::{Color, FontWeight, TextAlign};
use waft_foundation::{App, Handle, Listener};
use waft_painting::AlignmentGeometry;
use waft_rendering::BoxConstraints;
use waft_widgets::*;

/// Receives the `IsChecked` value a toggle asks for: XAML `Checked`, `Unchecked` and `Indeterminate`.
pub type AppBarToggleButtonCheckedHandler = Rc<dyn Fn(&mut App, Option<bool>)>;

/// XAML `AppBarToggleButton`: a command that keeps a checked state, for a `CommandBar`.
#[derive(Clone)]
pub struct AppBarToggleButton {
    /// `Label`.
    pub label: String,
    /// `IsChecked`; `None` is indeterminate.
    pub is_checked: Option<bool>,
    /// Asked for the next `IsChecked` on each click; the button shows the value it is given.
    pub checked: AppBarToggleButtonCheckedHandler,
    /// `Icon`.
    pub icon: Option<WidgetRef>,
    /// `LabelPosition`.
    pub label_position: CommandBarLabelPosition,
    /// `IsThreeState`: a click on a checked button makes it indeterminate.
    pub is_three_state: bool,
    /// `IsEnabled`.
    pub is_enabled: bool,
    /// `DynamicOverflowOrder`.
    pub dynamic_overflow_order: i32,
    /// `KeyboardAcceleratorTextOverride`.
    pub keyboard_accelerator_text_override: String,
    pub(crate) presentation: ElementPresentation,
    pub(crate) host: Option<ElementHost>,
    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl AppBarToggleButton {
    /// Creates a toggle button with a `Label`, its `IsChecked` and the callback asked for the next value.
    pub fn new(
        label: impl Into<String>,
        is_checked: Option<bool>,
        checked: impl Fn(&mut App, Option<bool>) + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            is_checked,
            checked: Rc::new(checked),
            icon: None,
            label_position: CommandBarLabelPosition::Default,
            is_three_state: false,
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

    /// Sets the XAML `IsThreeState`.
    pub fn is_three_state(mut self, value: bool) -> Self {
        self.is_three_state = value;
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
}

impl fmt::Debug for AppBarToggleButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppBarToggleButton")
            .field("label", &self.label)
            .field("is_checked", &self.is_checked)
            .field("is_three_state", &self.is_three_state)
            .field("is_enabled", &self.is_enabled)
            .field("dynamic_overflow_order", &self.dynamic_overflow_order)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for AppBarToggleButton {
    type State = AppBarToggleButtonState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> AppBarToggleButtonState {
        AppBarToggleButtonState {
            state: StateData::new(),
            last_state: ToggleCommonState::Normal,
        }
    }
}

/// The `CommonStates` state the template is in, kept for an indeterminate button.
pub struct AppBarToggleButtonState {
    state: StateData<AppBarToggleButton>,
    last_state: ToggleCommonState,
}

impl State for AppBarToggleButtonState {
    type Widget = AppBarToggleButton;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let next = next_is_checked(widget.is_checked, widget.is_three_state);
        let (checked, host) = (widget.checked.clone(), widget.host.clone());
        // `AppBarToggleButton::OnClick`: the bar closes, then `ToggleButton::OnClick` toggles.
        let on_click = Listener::new(move |app| {
            if let Some(host) = &host {
                host.executed.call(app);
            }
            checked(app, next);
        });
        let template_widget = widget.clone();
        let mut states = CommonStates::new(on_click, move |app, context, states| {
            let state = ToggleCommonState::of(
                &template_widget.presentation,
                template_widget.is_checked,
                states,
                app.get(self).last_state,
            );
            app.get_mut(self).last_state = state.base;
            template(app, context, &template_widget, state.shown, states)
        })
        .is_enabled(widget.is_enabled);
        if let Some(host) = &widget.host {
            states = states.focus_node(host.focus_node.clone());
        }
        with_tool_tip(
            states.into_widget(),
            &widget.label,
            &widget.keyboard_accelerator_text_override,
            &widget.presentation,
        )
    }
}

/// The template's `CommonStates`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToggleCommonState {
    Normal,
    PointerOver,
    Pressed,
    Disabled,
    Checked,
    CheckedPointerOver,
    CheckedPressed,
    CheckedDisabled,
    OverflowNormal,
    OverflowPointerOver,
    OverflowPressed,
    OverflowChecked,
    OverflowCheckedPointerOver,
    OverflowCheckedPressed,
}

/// The state `ToggleButton::ChangeVisualState` leaves (`base`) and the one shown after `AppBarToggleButton::ChangeVisualState` (`shown`).
struct ResolvedToggleState {
    base: ToggleCommonState,
    shown: ToggleCommonState,
}

impl ToggleCommonState {
    fn of(
        presentation: &ElementPresentation,
        is_checked: Option<bool>,
        states: ControlStates,
        last: ToggleCommonState,
    ) -> ResolvedToggleState {
        use ToggleCommonState::*;
        let base = match (is_checked, states.common) {
            // The template has no `Indeterminate*` states: the group stays where it was.
            (None, _) => last,
            (Some(true), CommonState::Disabled) => CheckedDisabled,
            (Some(true), CommonState::Pressed) => CheckedPressed,
            (Some(true), CommonState::PointerOver) => CheckedPointerOver,
            (Some(true), CommonState::Normal) => Checked,
            (Some(false), CommonState::Disabled) => Disabled,
            (Some(false), CommonState::Pressed) => Pressed,
            (Some(false), CommonState::PointerOver) => PointerOver,
            (Some(false), CommonState::Normal) => Normal,
        };
        let shown = if !presentation.use_overflow_style || states.common == CommonState::Disabled {
            base
        } else {
            match (is_checked == Some(true), states.common) {
                (true, CommonState::Pressed) => OverflowCheckedPressed,
                (true, CommonState::PointerOver) => OverflowCheckedPointerOver,
                (true, _) => OverflowChecked,
                (false, CommonState::Pressed) => OverflowPressed,
                (false, CommonState::PointerOver) => OverflowPointerOver,
                (false, _) => OverflowNormal,
            }
        };
        ResolvedToggleState { base, shown }
    }
}

/// The values one `CommonStates` state sets, the style's where it sets none.
#[derive(Clone, Copy, Debug)]
struct ToggleBrushes {
    background: Color,
    border: Brush,
    background_sizing: BackgroundSizing,
    /// `Content.Foreground`.
    content: Color,
    /// `TextLabel.Foreground`.
    label: Color,
    /// `OverflowCheckGlyph.Foreground`.
    glyph: Color,
    /// `OverflowTextLabel.Foreground`.
    overflow_label: Color,
    /// `KeyboardAcceleratorTextLabel.Foreground`.
    accelerator: Color,
    /// `OverflowCheckGlyph.Opacity`.
    glyph_opacity: f64,
}

fn brushes(r: &ThemeResources, state: ToggleCommonState) -> ToggleBrushes {
    use ToggleCommonState::*;
    let style = ToggleBrushes {
        background: r.app_bar_toggle_button_background,
        border: Brush::Solid(r.app_bar_toggle_button_border_brush),
        background_sizing: BackgroundSizing::InnerBorderEdge,
        content: r.app_bar_toggle_button_foreground,
        label: r.app_bar_toggle_button_foreground,
        glyph: r.app_bar_toggle_button_check_glyph_foreground,
        overflow_label: r.app_bar_toggle_button_foreground,
        accelerator: r.app_bar_toggle_button_keyboard_accelerator_text_foreground,
        glyph_opacity: 0.0,
    };
    let checked = |background, border, content, accelerator| ToggleBrushes {
        background,
        border,
        background_sizing: BackgroundSizing::OuterBorderEdge,
        content,
        label: content,
        accelerator,
        glyph_opacity: 1.0,
        ..style
    };
    match state {
        Normal | OverflowNormal => style,
        PointerOver => ToggleBrushes {
            background: r.app_bar_toggle_button_background_pointer_over,
            border: Brush::Solid(r.app_bar_toggle_button_border_brush_pointer_over),
            content: r.app_bar_toggle_button_foreground_pointer_over,
            label: r.app_bar_toggle_button_foreground_pointer_over,
            glyph: r.app_bar_toggle_button_overflow_label_foreground_pointer_over,
            overflow_label: r.app_bar_toggle_button_overflow_label_foreground_pointer_over,
            accelerator: r.app_bar_toggle_button_keyboard_accelerator_text_foreground_pointer_over,
            ..style
        },
        Pressed => ToggleBrushes {
            background: r.app_bar_toggle_button_background_pressed,
            border: Brush::Solid(r.app_bar_toggle_button_border_brush_pressed),
            content: r.app_bar_toggle_button_foreground_pressed,
            label: r.app_bar_toggle_button_foreground_pressed,
            glyph: r.app_bar_toggle_button_overflow_label_foreground_pressed,
            overflow_label: r.app_bar_toggle_button_overflow_label_foreground_pressed,
            accelerator: r.app_bar_toggle_button_keyboard_accelerator_text_foreground_pressed,
            ..style
        },
        Disabled => ToggleBrushes {
            background: r.app_bar_toggle_button_background_disabled,
            border: Brush::Solid(r.app_bar_toggle_button_border_brush_disabled),
            content: r.app_bar_toggle_button_foreground_disabled,
            label: r.app_bar_toggle_button_foreground_disabled,
            glyph: r.app_bar_toggle_button_overflow_label_foreground_disabled,
            overflow_label: r.app_bar_toggle_button_overflow_label_foreground_disabled,
            accelerator: r.app_bar_toggle_button_keyboard_accelerator_text_foreground_disabled,
            ..style
        },
        Checked => checked(
            r.app_bar_toggle_button_background_checked,
            Brush::ControlElevation(r.app_bar_toggle_button_border_brush_checked),
            r.app_bar_toggle_button_foreground_checked,
            r.app_bar_toggle_button_keyboard_accelerator_text_foreground_checked,
        ),
        CheckedPointerOver => ToggleBrushes {
            glyph: r.app_bar_toggle_button_overflow_label_foreground_checked_pointer_over,
            overflow_label: r.app_bar_toggle_button_overflow_label_foreground_checked_pointer_over,
            ..checked(
                r.app_bar_toggle_button_background_checked_pointer_over,
                Brush::ControlElevation(r.app_bar_toggle_button_border_brush_checked_pointer_over),
                r.app_bar_toggle_button_foreground_checked_pointer_over,
                r.app_bar_toggle_button_keyboard_accelerator_text_foreground_checked_pointer_over,
            )
        },
        CheckedPressed => ToggleBrushes {
            glyph: r.app_bar_toggle_button_overflow_label_foreground_checked_pressed,
            overflow_label: r.app_bar_toggle_button_overflow_label_foreground_checked_pressed,
            ..checked(
                r.app_bar_toggle_button_background_checked_pressed,
                Brush::Solid(r.app_bar_toggle_button_border_brush_checked_pressed),
                r.app_bar_toggle_button_foreground_checked_pressed,
                r.app_bar_toggle_button_keyboard_accelerator_text_foreground_checked_pressed,
            )
        },
        CheckedDisabled => ToggleBrushes {
            glyph: r.app_bar_toggle_button_overflow_label_foreground_checked_disabled,
            overflow_label: r.app_bar_toggle_button_overflow_label_foreground_checked_disabled,
            ..checked(
                r.app_bar_toggle_button_background_checked_disabled,
                Brush::Solid(r.app_bar_toggle_button_border_brush_checked_disabled),
                r.app_bar_toggle_button_foreground_checked_disabled,
                r.app_bar_toggle_button_keyboard_accelerator_text_foreground_checked_disabled,
            )
        },
        OverflowPointerOver => ToggleBrushes {
            background: r.app_bar_toggle_button_background_high_light_overlay_pointer_over,
            border: Brush::Solid(r.app_bar_toggle_button_border_brush_pointer_over),
            content: r.app_bar_toggle_button_foreground_pointer_over,
            label: r.app_bar_toggle_button_foreground_pointer_over,
            glyph: r.app_bar_toggle_button_check_glyph_foreground_pointer_over,
            overflow_label: r.app_bar_toggle_button_overflow_label_foreground_pointer_over,
            accelerator: r.app_bar_toggle_button_keyboard_accelerator_text_foreground_pointer_over,
            ..style
        },
        OverflowPressed => ToggleBrushes {
            background: r.app_bar_toggle_button_background_high_light_overlay_pressed,
            border: Brush::Solid(r.app_bar_toggle_button_border_brush_pressed),
            content: r.app_bar_toggle_button_foreground_pressed,
            label: r.app_bar_toggle_button_foreground_pressed,
            glyph: r.app_bar_toggle_button_check_glyph_foreground_pressed,
            overflow_label: r.app_bar_toggle_button_overflow_label_foreground_pressed,
            accelerator: r.app_bar_toggle_button_keyboard_accelerator_text_foreground_pressed,
            ..style
        },
        OverflowChecked => ToggleBrushes {
            border: Brush::ControlElevation(r.app_bar_toggle_button_border_brush_checked),
            content: r.app_bar_toggle_button_foreground,
            label: r.app_bar_toggle_button_foreground,
            glyph: r.app_bar_toggle_button_foreground,
            glyph_opacity: 1.0,
            ..style
        },
        OverflowCheckedPointerOver => ToggleBrushes {
            background: r.app_bar_toggle_button_background_high_light_overlay_checked_pointer_over,
            content: r.app_bar_toggle_button_overflow_label_foreground_checked_pointer_over,
            label: r.app_bar_toggle_button_foreground_checked_pointer_over,
            glyph: r.app_bar_toggle_button_overflow_label_foreground_checked_pointer_over,
            overflow_label: r.app_bar_toggle_button_overflow_label_foreground_checked_pointer_over,
            accelerator: r.app_bar_toggle_button_keyboard_accelerator_text_foreground_pointer_over,
            glyph_opacity: 1.0,
            ..style
        },
        OverflowCheckedPressed => ToggleBrushes {
            background: r.app_bar_toggle_button_background_high_light_overlay_checked_pressed,
            border: Brush::Solid(r.app_bar_toggle_button_border_brush_checked_pressed),
            content: r.app_bar_toggle_button_overflow_label_foreground_checked_pressed,
            label: r.app_bar_toggle_button_foreground_checked_pressed,
            glyph: r.app_bar_toggle_button_overflow_label_foreground_checked_pressed,
            overflow_label: r.app_bar_toggle_button_overflow_label_foreground_checked_pressed,
            accelerator: r.app_bar_toggle_button_keyboard_accelerator_text_foreground_pressed,
            glyph_opacity: 1.0,
            ..style
        },
    }
}

/// The `ControlTemplate` for one view state and `CommonStates` state.
fn template(
    app: &mut App,
    context: BuildContext,
    widget: &AppBarToggleButton,
    state: ToggleCommonState,
    states: ControlStates,
) -> WidgetRef {
    let r = ThemeResources::of(app, context);
    // `SetOverflowStyleParams` tells a toggle button only of icons and accelerator text.
    let presentation = ElementPresentation {
        with_toggle_buttons: false,
        ..widget.presentation
    };
    let view = ApplicationViewState::of(&presentation, widget.label_position);
    let brushes = brushes(&r, state);
    let touch = presentation.input_mode == InputDevice::Touch;
    let label_style =
        |size: f64, color: Color| control_text_style(&r, size, FontWeight::NORMAL, color);
    let mut cells = Vec::new();

    // OverflowCheckGlyph
    if view.is_overflow() {
        let margin = if touch {
            r.app_bar_toggle_button_overflow_check_touch_margin
        } else {
            r.app_bar_toggle_button_overflow_check_margin
        };
        cells.push(
            GridCell::new(Margin::new(
                margin,
                Align::new()
                    .alignment(AlignmentGeometry::CENTER_LEFT)
                    .height_factor(1.0)
                    .width_factor(1.0)
                    .child(
                        SizedBox::new().width(14.0).height(14.0).child(
                            Opacity::new(brushes.glyph_opacity).child(
                                FontIcon::symbol(FluentSymbol::Checkmark)
                                    .font_size(16.0)
                                    .foreground(brushes.glyph),
                            ),
                        ),
                    ),
            ))
            .into_widget(),
        );
    }
    // ContentViewbox > Content
    if view != ApplicationViewState::Overflow {
        let icon = widget.icon.clone().map(|icon| {
            DefaultTextStyle::new(
                label_style(r.control_content_theme_font_size, brushes.content),
                icon,
            )
            .into_widget()
        });
        cells.push(GridCell::new(content_viewbox(&r, view, icon, true)).into_widget());
    }
    // TextLabel
    match view {
        ApplicationViewState::FullSize => cells.push(
            GridCell::new(Margin::new(
                r.app_bar_toggle_button_text_label_margin,
                Text::new(widget.label.clone())
                    .style(label_style(12.0, brushes.label))
                    .text_align(TextAlign::Center),
            ))
            .row(1)
            .into_widget(),
        ),
        ApplicationViewState::LabelOnRight => cells.push(
            GridCell::new(Margin::new(
                r.app_bar_toggle_button_text_label_on_right_margin,
                Text::new(widget.label.clone())
                    .style(label_style(12.0, brushes.label))
                    .text_align(TextAlign::Left),
            ))
            .column(1)
            .into_widget(),
        ),
        _ => {}
    }
    // OverflowTextLabel
    if view.is_overflow() {
        let margin = if view == ApplicationViewState::OverflowWithMenuIcons {
            [76.0, 0.0, 12.0, 0.0]
        } else {
            [38.0, 0.0, 12.0, 0.0]
        };
        let padding = if touch {
            r.app_bar_toggle_button_overflow_text_touch_margin
        } else {
            r.app_bar_toggle_button_overflow_text_label_padding
        };
        cells.push(
            GridCell::new(overflow_text_label(
                &widget.label,
                label_style(r.control_content_theme_font_size, brushes.overflow_label),
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

    let content_root =
        ConstrainedBox::new(BoxConstraints::new().min_height(content_root_min_height(&r, view)))
            .child(
                Grid::new()
                    .column_definitions([
                        ColumnDefinition::new(GridLength::STAR),
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
        brushes.border,
        r.app_bar_toggle_button_border_theme_thickness[0],
        brushes.background_sizing,
        states.focused,
    );
    root(
        view,
        Margin::new(inner_border_margin(&r, view), inner_border),
        content_root.into_widget(),
    )
}
