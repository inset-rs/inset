//! Flutter counterpart: material_ui's `toggle_buttons.dart`.

#![expect(deprecated, reason = "Dart calls the deprecated `withOpacity`")]

use std::any::Any;
use std::f64::consts::PI;
use std::fmt::{self, Debug};
use std::rc::Rc;

use waft_embedder::{
    Color, FillRule, Offset, PathBuilder, RRect, Radius, Rect, Size, TextBaseline,
};
use waft_foundation::{App, Listener, ValueChanged};
use waft_painting::{
    Alignment, AnyColor, Axis, BorderRadius, BorderSide, ClipContext, EdgeInsetsDirectional,
    EdgeInsetsGeometry, OutlinedBorderRef, RoundedRectangleBorder, TextStyle, VerticalDirection,
};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BaselineOffset, BoxConstraints, BoxHitTestResult, BoxParentData,
    ChildLayoutHelper, ChildLayouter, CrossAxisAlignment, MainAxisSize, PaintingContext, RenderBox,
    RenderBoxData, RenderHandle, RenderObject, RenderObjectData, RenderObjectWithChildData,
    RenderObjectWithChildMixin, RenderShiftedBox,
};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, Center, ClipRRect, Column, Directionality, FocusNodeRef, IntoWidget,
    IntrinsicHeight, IntrinsicWidth, KeyRef, MergeSemantics, RenderObjectWidget, Row, Semantics,
    SingleChildRenderObjectWidget, StatelessWidget, WidgetRef, WidgetState, WidgetStateColor,
    WidgetStateProperty, WidgetStatePropertyAll, WidgetStatePropertyRef, WidgetStates,
};

use crate::button_style::ButtonStyle;
use crate::color_scheme::ColorScheme;
use crate::constants::K_MIN_INTERACTIVE_DIMENSION;
use crate::constants::K_THEME_CHANGE_DURATION;
use crate::ink_ripple::InkRipple;
use crate::text_button::TextButton;
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};
use crate::toggle_buttons_theme::ToggleButtonsTheme;
use crate::toggle_buttons_theme::ToggleButtonsThemeData;

/// A set of toggle buttons.
///
/// The list of [`children`](Self::children) are laid out along [`direction`](Self::direction).
/// The state of each button is controlled by [`is_selected`](Self::is_selected), which is a
/// list of bools that determine if a button is in an unselected or selected state. They are
/// both correlated by their index in the list. The length of [`is_selected`](Self::is_selected)
/// has to match the length of the [`children`](Self::children) list.
///
/// ## Updating the button states
///
/// When the button is tapped, a [`on_pressed`](Self::on_pressed) callback with the index of
/// the pressed button is called. The app must update the selection state in response, as the
/// buttons keep none.
///
/// ## Customizing toggle buttons
///
/// Each toggle's behavior can be configured by the [`on_pressed`](Self::on_pressed) callback
/// and [`is_selected`](Self::is_selected) list, which determines if the button is enabled or
/// selected, and the colors and border of the buttons are set by the other fields, falling
/// back to `ToggleButtonsTheme`, then to values from the overall `Theme`.
///
/// See also:
///
///  * `SegmentedButton`, a Material 3 replacement for this widget.
///  * `IconButton`, to create buttons that contain icons.
///  * `TextButton`, a Material Design button without a shadow.
///  * `ElevatedButton`, a filled button whose material elevates when pressed.
///  * `OutlinedButton`, a button with an outlined border and no fill color.
///  * <https://material.io/design/components/buttons.html>
pub struct ToggleButtons {
    /// See [`Widget::key`](waft_widgets::Widget::key).
    pub key: Option<KeyRef>,

    /// The toggle button widgets.
    ///
    /// These are typically `Icon` or `Text` widgets. The boolean selection
    /// state of each widget is defined by the corresponding
    /// [`is_selected`](Self::is_selected) list item.
    ///
    /// The length of this list has to match the length of the
    /// [`is_selected`](Self::is_selected) list.
    pub children: Vec<WidgetRef>,

    /// The corresponding selection state of each toggle button.
    ///
    /// Each value in this list represents the selection state of the [`children`](Self::children)
    /// widget at the same index.
    ///
    /// The length of this list has to match the length of the [`children`](Self::children) list.
    pub is_selected: Vec<bool>,

    /// The callback that is called when a button is tapped.
    ///
    /// The index parameter of the callback is the index of the button that is tapped or
    /// otherwise activated.
    ///
    /// When the callback is null, all toggle buttons will be disabled.
    pub on_pressed: Option<ValueChanged<usize>>,

    /// {@macro flutter.material.RawMaterialButton.mouseCursor}
    ///
    /// If this property is null, `WidgetStateMouseCursor::adaptive_clickable` will be used.
    pub mouse_cursor: Option<MouseCursorRef>,

    /// Configures the minimum size of the area within which the buttons may be pressed.
    ///
    /// If the [`tap_target_size`](Self::tap_target_size) is larger than [`constraints`](Self::constraints),
    /// the button will include a pad of the remaining size so that the tap target is
    /// at least as big as the [`tap_target_size`](Self::tap_target_size).
    ///
    /// Defaults to `ThemeData::material_tap_target_size`.
    pub tap_target_size: Option<MaterialTapTargetSize>,

    /// The text style of [`children`](Self::children).
    ///
    /// `TextStyle::color` will be ignored and substituted by [`color`](Self::color),
    /// [`selected_color`](Self::selected_color) or [`disabled_color`](Self::disabled_color)
    /// depending on whether the buttons are active, selected, or disabled.
    ///
    /// If this property is null, then `ToggleButtonsThemeData::text_style` is used.
    /// If that is also null, then `TextTheme::body_medium` is used.
    pub text_style: Option<TextStyle>,

    /// Defines the button's size.
    ///
    /// Typically used to constrain the button's minimum size.
    ///
    /// If this property is null, then `ToggleButtonsThemeData::constraints` is used. If that
    /// is also null, then a minimum size of 48 by 48 is used.
    pub constraints: Option<BoxConstraints>,

    /// The color for descendant `Text` and `Icon` widgets if the button is enabled and not
    /// selected.
    ///
    /// If [`on_pressed`](Self::on_pressed) is not null, this color will be used for
    /// unselected children. If it is null, then `ToggleButtonsThemeData::color` is used.
    /// If that is also null, then `ColorScheme::on_surface` with an opacity of 0.87 is used.
    pub color: Option<Color>,

    /// The color for descendant `Text` and `Icon` widgets if the button is selected.
    ///
    /// If null, then `ToggleButtonsThemeData::selected_color` is used. If that is also null,
    /// then `ColorScheme::primary` is used.
    pub selected_color: Option<Color>,

    /// The color for descendant `Text` and `Icon` widgets if the button is disabled.
    ///
    /// If null, then `ToggleButtonsThemeData::disabled_color` is used. If that is also null,
    /// then `ColorScheme::on_surface` with an opacity of 0.38 is used.
    pub disabled_color: Option<Color>,

    /// The fill color for selected toggle buttons.
    ///
    /// If this is null, then `ToggleButtonsThemeData::fill_color` is used. If that is also
    /// null, then `ColorScheme::primary` with an opacity of 0.12 is used.
    ///
    /// It can be a `WidgetStateColor`, which then resolves in the `WidgetState::Selected` and
    /// `WidgetState::Disabled` states.
    pub fill_color: Option<AnyColor>,

    /// The color to use for filling the button when the button has input focus.
    ///
    /// If null, `ToggleButtonsThemeData::focus_color` is used, else `ColorScheme::primary` or
    /// `ColorScheme::on_surface` with an opacity of 0.12.
    pub focus_color: Option<Color>,

    /// The highlight color for the button's `InkWell`.
    ///
    /// If null, `ToggleButtonsThemeData::highlight_color` is used, else
    /// `ColorScheme::on_surface` with an opacity of 0.16 for an unselected button.
    pub highlight_color: Option<Color>,

    /// The splash color for the button's `InkWell`.
    ///
    /// If null, `ToggleButtonsThemeData::splash_color` is used, else `ColorScheme::primary`
    /// or `ColorScheme::on_surface` with an opacity of 0.16.
    pub splash_color: Option<Color>,

    /// The color to use for filling the button when the button has a pointer hovering over it.
    ///
    /// If null, `ToggleButtonsThemeData::hover_color` is used, else `ColorScheme::primary` or
    /// `ColorScheme::on_surface` with an opacity of 0.04.
    pub hover_color: Option<Color>,

    /// The list of [`FocusNodeRef`]s, corresponding to each toggle button.
    ///
    /// Focus is used to determine which widget should be affected by keyboard events. The
    /// focus tree keeps track of which widget is currently focused on by the user.
    ///
    /// If not null, the length of this list has to match the length of the
    /// [`children`](Self::children) list.
    pub focus_nodes: Option<Vec<FocusNodeRef>>,

    /// Whether or not to render a border around each toggle button.
    ///
    /// When true, a border with [`border_width`](Self::border_width), [`border_color`](Self::border_color),
    /// [`selected_border_color`](Self::selected_border_color) and
    /// [`disabled_border_color`](Self::disabled_border_color) is drawn.
    ///
    /// Defaults to true.
    pub render_border: bool,

    /// The border color to display when the toggle button is enabled and not selected.
    ///
    /// If null, `ToggleButtonsThemeData::border_color` is used, else `ColorScheme::on_surface`
    /// with an opacity of 0.12.
    pub border_color: Option<Color>,

    /// The border color to display when the toggle button is selected.
    ///
    /// If null, `ToggleButtonsThemeData::selected_border_color` is used, else
    /// `ColorScheme::on_surface` with an opacity of 0.12.
    pub selected_border_color: Option<Color>,

    /// The border color to display when the toggle button is disabled.
    ///
    /// If null, `ToggleButtonsThemeData::disabled_border_color` is used, else
    /// `ColorScheme::on_surface` with an opacity of 0.12.
    pub disabled_border_color: Option<Color>,

    /// The width of the border surrounding each toggle button.
    ///
    /// This applies to both the greater surrounding border, as well as the
    /// borders dividing each toggle button.
    ///
    /// To render a hairline border (one physical pixel), set [`border_width`](Self::border_width)
    /// to 0.0. If null, `ToggleButtonsThemeData::border_width` is used, else 1.0.
    pub border_width: Option<f64>,

    /// The radii of the border's corners.
    ///
    /// If this property is null, then `ToggleButtonsThemeData::border_radius` is used.
    /// If that is also null, then [`BorderRadius::ZERO`] is used.
    pub border_radius: Option<BorderRadius>,

    /// The direction along which the buttons are rendered.
    ///
    /// Defaults to [`Axis::Horizontal`].
    pub direction: Axis,

    /// If [`direction`](Self::direction) is [`Axis::Vertical`], this property defines whether
    /// the buttons are laid out starting from the top (down) or the bottom (up).
    ///
    /// Defaults to [`VerticalDirection::Down`].
    pub vertical_direction: VerticalDirection,
}

impl ToggleButtons {
    const DEFAULT_BORDER_WIDTH: f64 = 1.0;

    /// Creates a set of toggle buttons.
    ///
    /// It displays its widgets provided in a list of [`children`](Self::children) along
    /// [`direction`](Self::direction). The state of each button is controlled by
    /// [`is_selected`](Self::is_selected), which is a list of bools that determine if a button
    /// is in an unselected or selected state. They are both correlated by their index in the
    /// list. The length of [`is_selected`](Self::is_selected) has to match the length of the
    /// [`children`](Self::children) list.
    pub fn new(children: Vec<WidgetRef>, is_selected: Vec<bool>) -> ToggleButtons {
        ToggleButtons {
            key: None,
            children,
            is_selected,
            on_pressed: None,
            mouse_cursor: None,
            tap_target_size: None,
            text_style: None,
            constraints: None,
            color: None,
            selected_color: None,
            disabled_color: None,
            fill_color: None,
            focus_color: None,
            highlight_color: None,
            splash_color: None,
            hover_color: None,
            focus_nodes: None,
            render_border: true,
            border_color: None,
            selected_border_color: None,
            disabled_border_color: None,
            border_width: None,
            border_radius: None,
            direction: Axis::Horizontal,
            vertical_direction: VerticalDirection::Down,
        }
    }

    /// Dart `ToggleButtons(key:)`.
    pub fn key(mut self, key: KeyRef) -> ToggleButtons {
        self.key = Some(key);
        self
    }

    /// Dart `ToggleButtons(onPressed:)`.
    pub fn on_pressed(mut self, on_pressed: impl Fn(&mut App, usize) + 'static) -> ToggleButtons {
        self.on_pressed = Some(Rc::new(on_pressed));
        self
    }

    /// Dart `ToggleButtons(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: impl Into<MouseCursorRef>) -> ToggleButtons {
        self.mouse_cursor = Some(mouse_cursor.into());
        self
    }

    /// Dart `ToggleButtons(tapTargetSize:)`.
    pub fn tap_target_size(mut self, tap_target_size: MaterialTapTargetSize) -> ToggleButtons {
        self.tap_target_size = Some(tap_target_size);
        self
    }

    /// Dart `ToggleButtons(textStyle:)`.
    pub fn text_style(mut self, text_style: TextStyle) -> ToggleButtons {
        self.text_style = Some(text_style);
        self
    }

    /// Dart `ToggleButtons(constraints:)`.
    pub fn constraints(mut self, constraints: BoxConstraints) -> ToggleButtons {
        self.constraints = Some(constraints);
        self
    }

    /// Dart `ToggleButtons(color:)`.
    pub fn color(mut self, color: Color) -> ToggleButtons {
        self.color = Some(color);
        self
    }

    /// Dart `ToggleButtons(selectedColor:)`.
    pub fn selected_color(mut self, selected_color: Color) -> ToggleButtons {
        self.selected_color = Some(selected_color);
        self
    }

    /// Dart `ToggleButtons(disabledColor:)`.
    pub fn disabled_color(mut self, disabled_color: Color) -> ToggleButtons {
        self.disabled_color = Some(disabled_color);
        self
    }

    /// Dart `ToggleButtons(fillColor:)`.
    pub fn fill_color(mut self, fill_color: impl Into<AnyColor>) -> ToggleButtons {
        self.fill_color = Some(fill_color.into());
        self
    }

    /// Dart `ToggleButtons(focusColor:)`.
    pub fn focus_color(mut self, focus_color: Color) -> ToggleButtons {
        self.focus_color = Some(focus_color);
        self
    }

    /// Dart `ToggleButtons(highlightColor:)`.
    pub fn highlight_color(mut self, highlight_color: Color) -> ToggleButtons {
        self.highlight_color = Some(highlight_color);
        self
    }

    /// Dart `ToggleButtons(hoverColor:)`.
    pub fn hover_color(mut self, hover_color: Color) -> ToggleButtons {
        self.hover_color = Some(hover_color);
        self
    }

    /// Dart `ToggleButtons(splashColor:)`.
    pub fn splash_color(mut self, splash_color: Color) -> ToggleButtons {
        self.splash_color = Some(splash_color);
        self
    }

    /// Dart `ToggleButtons(focusNodes:)`.
    pub fn focus_nodes(mut self, focus_nodes: Vec<FocusNodeRef>) -> ToggleButtons {
        self.focus_nodes = Some(focus_nodes);
        self
    }

    /// Dart `ToggleButtons(renderBorder:)`.
    pub fn render_border(mut self, render_border: bool) -> ToggleButtons {
        self.render_border = render_border;
        self
    }

    /// Dart `ToggleButtons(borderColor:)`.
    pub fn border_color(mut self, border_color: Color) -> ToggleButtons {
        self.border_color = Some(border_color);
        self
    }

    /// Dart `ToggleButtons(selectedBorderColor:)`.
    pub fn selected_border_color(mut self, selected_border_color: Color) -> ToggleButtons {
        self.selected_border_color = Some(selected_border_color);
        self
    }

    /// Dart `ToggleButtons(disabledBorderColor:)`.
    pub fn disabled_border_color(mut self, disabled_border_color: Color) -> ToggleButtons {
        self.disabled_border_color = Some(disabled_border_color);
        self
    }

    /// Dart `ToggleButtons(borderRadius:)`.
    pub fn border_radius(mut self, border_radius: BorderRadius) -> ToggleButtons {
        self.border_radius = Some(border_radius);
        self
    }

    /// Dart `ToggleButtons(borderWidth:)`.
    pub fn border_width(mut self, border_width: f64) -> ToggleButtons {
        self.border_width = Some(border_width);
        self
    }

    /// Dart `ToggleButtons(direction:)`.
    pub fn direction(mut self, direction: Axis) -> ToggleButtons {
        self.direction = direction;
        self
    }

    /// Dart `ToggleButtons(verticalDirection:)`.
    pub fn vertical_direction(mut self, vertical_direction: VerticalDirection) -> ToggleButtons {
        self.vertical_direction = vertical_direction;
        self
    }

    // Determines if this is the first child that is being laid out
    // by the render object, _not_ the order of the children in its list.
    fn is_first_button(
        &self,
        index: usize,
        length: usize,
        text_direction: waft_painting::TextDirection,
    ) -> bool {
        match self.direction {
            Axis::Horizontal => match text_direction {
                waft_painting::TextDirection::Rtl => index == length - 1,
                waft_painting::TextDirection::Ltr => index == 0,
            },
            Axis::Vertical => match self.vertical_direction {
                VerticalDirection::Up => index == length - 1,
                VerticalDirection::Down => index == 0,
            },
        }
    }

    // Determines if this is the last child that is being laid out
    // by the render object, _not_ the order of the children in its list.
    fn is_last_button(
        &self,
        index: usize,
        length: usize,
        text_direction: waft_painting::TextDirection,
    ) -> bool {
        match self.direction {
            Axis::Horizontal => match text_direction {
                waft_painting::TextDirection::Rtl => index == 0,
                waft_painting::TextDirection::Ltr => index == length - 1,
            },
            Axis::Vertical => match self.vertical_direction {
                VerticalDirection::Up => index == 0,
                VerticalDirection::Down => index == length - 1,
            },
        }
    }

    fn get_edge_border_radius(
        &self,
        index: usize,
        length: usize,
        text_direction: waft_painting::TextDirection,
        toggle_buttons_theme: &ToggleButtonsThemeData,
    ) -> BorderRadius {
        let resulting_border_radius = self
            .border_radius
            .or(toggle_buttons_theme.border_radius)
            .unwrap_or(BorderRadius::ZERO);

        if length == 1 {
            return resulting_border_radius;
        } else if self.direction == Axis::Horizontal {
            if self.is_first_button(index, length, text_direction) {
                return BorderRadius::only(
                    resulting_border_radius.top_left,
                    Radius::ZERO,
                    resulting_border_radius.bottom_left,
                    Radius::ZERO,
                );
            } else if self.is_last_button(index, length, text_direction) {
                return BorderRadius::only(
                    Radius::ZERO,
                    resulting_border_radius.top_right,
                    Radius::ZERO,
                    resulting_border_radius.bottom_right,
                );
            }
        } else if self.is_first_button(index, length, text_direction) {
            return BorderRadius::only(
                resulting_border_radius.top_left,
                resulting_border_radius.top_right,
                Radius::ZERO,
                Radius::ZERO,
            );
        } else if self.is_last_button(index, length, text_direction) {
            return BorderRadius::only(
                Radius::ZERO,
                Radius::ZERO,
                resulting_border_radius.bottom_left,
                resulting_border_radius.bottom_right,
            );
        }

        BorderRadius::ZERO
    }

    fn get_clip_border_radius(
        &self,
        index: usize,
        length: usize,
        text_direction: waft_painting::TextDirection,
        toggle_buttons_theme: &ToggleButtonsThemeData,
    ) -> BorderRadius {
        let resulting_border_radius = self
            .border_radius
            .or(toggle_buttons_theme.border_radius)
            .unwrap_or(BorderRadius::ZERO);
        let resulting_border_width = self
            .border_width
            .or(toggle_buttons_theme.border_width)
            .unwrap_or(Self::DEFAULT_BORDER_WIDTH);
        let half = Radius::circular(resulting_border_width / 2.0);

        if length == 1 {
            return BorderRadius::only(
                resulting_border_radius.top_left - half,
                resulting_border_radius.top_right - half,
                resulting_border_radius.bottom_left - half,
                resulting_border_radius.bottom_right - half,
            );
        } else if self.direction == Axis::Horizontal {
            if self.is_first_button(index, length, text_direction) {
                return BorderRadius::only(
                    resulting_border_radius.top_left - half,
                    Radius::ZERO,
                    resulting_border_radius.bottom_left - half,
                    Radius::ZERO,
                );
            } else if self.is_last_button(index, length, text_direction) {
                return BorderRadius::only(
                    Radius::ZERO,
                    resulting_border_radius.top_right - half,
                    Radius::ZERO,
                    resulting_border_radius.bottom_right - half,
                );
            }
        } else if self.is_first_button(index, length, text_direction) {
            return BorderRadius::only(
                resulting_border_radius.top_left - half,
                resulting_border_radius.top_right - half,
                Radius::ZERO,
                Radius::ZERO,
            );
        } else if self.is_last_button(index, length, text_direction) {
            return BorderRadius::only(
                Radius::ZERO,
                Radius::ZERO,
                resulting_border_radius.bottom_left - half,
                resulting_border_radius.bottom_right - half,
            );
        }
        BorderRadius::ZERO
    }

    /// The border side of a button in the state the button is in: Dart's three
    /// `_get*BorderSide` methods differ only in whether `selected` is true.
    fn border_side_in_state(
        &self,
        selected: bool,
        theme: &crate::theme_data::ThemeData,
        toggle_buttons_theme: &ToggleButtonsThemeData,
    ) -> BorderSide {
        let resulting_border_width = self
            .border_width
            .or(toggle_buttons_theme.border_width)
            .unwrap_or(Self::DEFAULT_BORDER_WIDTH);
        let default_color = theme.color_scheme.on_surface().with_opacity(0.12);
        if self.on_pressed.is_some() && selected {
            BorderSide {
                color: self
                    .selected_border_color
                    .or(toggle_buttons_theme.selected_border_color)
                    .unwrap_or(default_color),
                width: resulting_border_width,
                ..BorderSide::default()
            }
        } else if self.on_pressed.is_some() && !selected {
            BorderSide {
                color: self
                    .border_color
                    .or(toggle_buttons_theme.border_color)
                    .unwrap_or(default_color),
                width: resulting_border_width,
                ..BorderSide::default()
            }
        } else {
            BorderSide {
                color: self
                    .disabled_border_color
                    .or(toggle_buttons_theme.disabled_border_color)
                    .unwrap_or(default_color),
                width: resulting_border_width,
                ..BorderSide::default()
            }
        }
    }

    fn get_leading_border_side(
        &self,
        index: usize,
        theme: &crate::theme_data::ThemeData,
        toggle_buttons_theme: &ToggleButtonsThemeData,
    ) -> BorderSide {
        if !self.render_border {
            return BorderSide::NONE;
        }

        self.border_side_in_state(
            self.is_selected[index] || (index != 0 && self.is_selected[index - 1]),
            theme,
            toggle_buttons_theme,
        )
    }

    fn get_border_side(
        &self,
        index: usize,
        theme: &crate::theme_data::ThemeData,
        toggle_buttons_theme: &ToggleButtonsThemeData,
    ) -> BorderSide {
        if !self.render_border {
            return BorderSide::NONE;
        }

        self.border_side_in_state(self.is_selected[index], theme, toggle_buttons_theme)
    }

    fn get_trailing_border_side(
        &self,
        index: usize,
        theme: &crate::theme_data::ThemeData,
        toggle_buttons_theme: &ToggleButtonsThemeData,
    ) -> BorderSide {
        if !self.render_border {
            return BorderSide::NONE;
        }

        if index != self.children.len() - 1 {
            return BorderSide::NONE;
        }

        self.border_side_in_state(self.is_selected[index], theme, toggle_buttons_theme)
    }
}

impl Debug for ToggleButtons {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToggleButtons")
            .field("is_selected", &self.is_selected)
            .field("disabled", &self.on_pressed.is_none())
            .field("direction", &self.direction)
            .field("vertical_direction", &self.vertical_direction)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for ToggleButtons {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(
            self.children.len() == self.is_selected.len(),
            "children.length == isSelected.length: {} children, {} isSelected",
            self.children.len(),
            self.is_selected.len()
        );
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(
            self.focus_nodes
                .as_ref()
                .is_none_or(|focus_nodes| focus_nodes.len() == self.children.len()),
            "focusNodes.length must match children.length.\nThere are {} focus nodes, while \
             there are {} children.",
            self.focus_nodes.as_ref().map_or(0, Vec::len),
            self.children.len()
        );
        let theme = Theme::of(app, context);
        let toggle_buttons_theme = ToggleButtonsTheme::of(app, context);
        let text_direction = Directionality::of(app, context);
        let length = self.children.len();

        let buttons: Vec<WidgetRef> = (0..length)
            .map(|index| {
                let edge_border_radius = self.get_edge_border_radius(
                    index,
                    length,
                    text_direction,
                    &toggle_buttons_theme,
                );
                let clip_border_radius = self.get_clip_border_radius(
                    index,
                    length,
                    text_direction,
                    &toggle_buttons_theme,
                );

                let leading_border_side =
                    self.get_leading_border_side(index, &theme, &toggle_buttons_theme);
                let border_side = self.get_border_side(index, &theme, &toggle_buttons_theme);
                let trailing_border_side =
                    self.get_trailing_border_side(index, &theme, &toggle_buttons_theme);

                let mut states = WidgetStates::new();
                if self.is_selected[index] && self.on_pressed.is_some() {
                    states.insert(WidgetState::Selected);
                }
                if self.on_pressed.is_none() {
                    states.insert(WidgetState::Disabled);
                }
                let effective_fill_color = ResolveFillColor::new(
                    self.fill_color
                        .clone()
                        .or_else(|| toggle_buttons_theme.fill_color.clone()),
                )
                .resolve(&states)
                .unwrap_or_else(|| DefaultFillColor::new(&theme.color_scheme).resolve(&states));
                let current_color = if self.on_pressed.is_some() && self.is_selected[index] {
                    self.selected_color
                        .or(toggle_buttons_theme.selected_color)
                        .unwrap_or(theme.color_scheme.primary())
                } else if self.on_pressed.is_some() && !self.is_selected[index] {
                    self.color
                        .or(toggle_buttons_theme.color)
                        .unwrap_or_else(|| theme.color_scheme.on_surface().with_opacity(0.87))
                } else {
                    self.disabled_color
                        .or(toggle_buttons_theme.disabled_color)
                        .unwrap_or_else(|| theme.color_scheme.on_surface().with_opacity(0.38))
                };
                let current_text_style = self
                    .text_style
                    .clone()
                    .or_else(|| toggle_buttons_theme.text_style.clone())
                    .or_else(|| theme.text_theme.body_medium.clone())
                    .expect("the text theme has a bodyMedium");
                let current_constraints = self.constraints.or(toggle_buttons_theme.constraints);
                let minimum_size = current_constraints
                    .map(|constraints| constraints.smallest())
                    .unwrap_or(Size::new(
                        K_MIN_INTERACTIVE_DIMENSION,
                        K_MIN_INTERACTIVE_DIMENSION,
                    ));
                let maximum_size = current_constraints.map(|constraints| constraints.biggest());
                let min_padding_size = match self
                    .tap_target_size
                    .unwrap_or(theme.material_tap_target_size)
                {
                    MaterialTapTargetSize::Padded => {
                        let min_padding_size = match self.direction {
                            Axis::Horizontal => Size::new(0.0, K_MIN_INTERACTIVE_DIMENSION),
                            Axis::Vertical => Size::new(K_MIN_INTERACTIVE_DIMENSION, 0.0),
                        };
                        debug_assert!(min_padding_size.width() >= 0.0);
                        debug_assert!(min_padding_size.height() >= 0.0);
                        min_padding_size
                    }
                    MaterialTapTargetSize::ShrinkWrap => Size::ZERO,
                };

                let on_pressed = self.on_pressed.clone().map(|on_pressed| {
                    Listener::new(move |app| {
                        on_pressed(app, index);
                    })
                });
                let mut text_button =
                    TextButton::new(on_pressed, Some(self.children[index].clone())).style(
                        ButtonStyle::new()
                            .background_color(all(effective_fill_color))
                            .foreground_color(all(current_color))
                            .icon_size(all(24.0))
                            .icon_color(all(current_color))
                            .overlay_color(Rc::new(ToggleButtonDefaultOverlay {
                                selected: self.on_pressed.is_some() && self.is_selected[index],
                                unselected: self.on_pressed.is_some() && !self.is_selected[index],
                                color_scheme: Some(theme.color_scheme.clone()),
                                disabled_color: self
                                    .disabled_color
                                    .or(toggle_buttons_theme.disabled_color),
                                focus_color: self.focus_color.or(toggle_buttons_theme.focus_color),
                                highlight_color: self
                                    .highlight_color
                                    .or(toggle_buttons_theme.highlight_color),
                                hover_color: self.hover_color.or(toggle_buttons_theme.hover_color),
                                splash_color: self
                                    .splash_color
                                    .or(toggle_buttons_theme.splash_color),
                            }))
                            .elevation(all(0.0))
                            .text_style(all(current_text_style.copy_with().color(current_color)))
                            .padding(all(EdgeInsetsGeometry::ZERO))
                            .minimum_size(all(minimum_size))
                            .maximum_size(Rc::new(WidgetStatePropertyAll::new(maximum_size)))
                            .shape(all(OutlinedBorderRef::new(
                                RoundedRectangleBorder::default(),
                            )))
                            .mouse_cursor(Rc::new(WidgetStatePropertyAll::new(
                                self.mouse_cursor.clone(),
                            )))
                            .visual_density(VisualDensity::STANDARD)
                            .tap_target_size(MaterialTapTargetSize::ShrinkWrap)
                            .animation_duration(K_THEME_CHANGE_DURATION)
                            .enable_feedback(true)
                            .alignment(waft_painting::AlignmentGeometry::CENTER)
                            .splash_factory(InkRipple::splash_factory()),
                    );
                if let Some(focus_nodes) = &self.focus_nodes {
                    text_button = text_button.focus_node(focus_nodes[index].clone());
                }

                let mut button = SelectToggleButton {
                    child: ClipRRect::new()
                        .border_radius(clip_border_radius.into())
                        .child(text_button)
                        .into_widget(),
                    leading_border_side,
                    border_side,
                    trailing_border_side,
                    border_radius: edge_border_radius,
                    is_first_button: index == 0,
                    is_last_button: index == length - 1,
                    direction: self.direction,
                    vertical_direction: self.vertical_direction,
                }
                .into_widget();

                if current_constraints.is_some() {
                    button = Center::new().child(button).into_widget();
                }

                MergeSemantics::new()
                    .child(
                        Semantics::new()
                            .container(true)
                            .checked(self.is_selected[index])
                            .enabled(self.on_pressed.is_some())
                            .child(InputPadding {
                                child: Some(button),
                                min_size: min_padding_size,
                                direction: self.direction,
                            }),
                    )
                    .into_widget()
            })
            .collect();

        if self.direction == Axis::Vertical {
            return IntrinsicWidth::new()
                .child(
                    Column::new()
                        .main_axis_size(MainAxisSize::Min)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .vertical_direction(self.vertical_direction)
                        .children(buttons),
                )
                .into_widget();
        }

        IntrinsicHeight::new()
            .child(
                Row::new()
                    .main_axis_size(MainAxisSize::Min)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .children(buttons),
            )
            .into_widget()
    }
}

/// Dart's `MaterialStatePropertyAll<Color?>(value)` and its `<double>` and `<Size?>` kin.
fn all<T: Clone + Debug + PartialEq + 'static>(value: T) -> WidgetStatePropertyRef<Option<T>> {
    Rc::new(WidgetStatePropertyAll::new(Some(value)))
}

/// Dart's `_ResolveFillColor`.
struct ResolveFillColor {
    primary: Option<AnyColor>,
}

impl ResolveFillColor {
    fn new(primary: Option<AnyColor>) -> ResolveFillColor {
        ResolveFillColor { primary }
    }

    fn resolve(&self, states: &WidgetStates) -> Option<Color> {
        let primary = self.primary.as_ref()?;
        if primary.extension::<WidgetStateColor>().is_some() {
            return Some(
                <dyn WidgetStateProperty<Color>>::resolve_as(&Some(primary.clone()), states)?
                    .color(),
            );
        }
        if states.contains(&WidgetState::Selected) {
            Some(primary.color())
        } else {
            None
        }
    }
}

/// Dart's `_DefaultFillColor`.
struct DefaultFillColor<'a> {
    color_scheme: &'a ColorScheme,
}

impl<'a> DefaultFillColor<'a> {
    fn new(color_scheme: &'a ColorScheme) -> DefaultFillColor<'a> {
        DefaultFillColor { color_scheme }
    }

    fn resolve(&self, states: &WidgetStates) -> Color {
        if states.contains(&WidgetState::Selected) {
            return self.color_scheme.primary().with_opacity(0.12);
        }
        self.color_scheme.surface().with_opacity(0.0)
    }
}

/// Dart's `_ToggleButtonDefaultOverlay`.
struct ToggleButtonDefaultOverlay {
    selected: bool,
    unselected: bool,
    color_scheme: Option<ColorScheme>,
    focus_color: Option<Color>,
    highlight_color: Option<Color>,
    hover_color: Option<Color>,
    splash_color: Option<Color>,
    #[expect(dead_code, reason = "Dart's class takes it and never reads it")]
    disabled_color: Option<Color>,
}

impl ToggleButtonDefaultOverlay {
    fn primary(&self, opacity: f64) -> Option<Color> {
        self.color_scheme
            .as_ref()
            .map(|color_scheme| color_scheme.primary().with_opacity(opacity))
    }

    fn on_surface(&self, opacity: f64) -> Option<Color> {
        self.color_scheme
            .as_ref()
            .map(|color_scheme| color_scheme.on_surface().with_opacity(opacity))
    }
}

impl WidgetStateProperty<Option<Color>> for ToggleButtonDefaultOverlay {
    fn resolve(&self, states: &WidgetStates) -> Option<Color> {
        if self.selected {
            if states.contains(&WidgetState::Pressed) {
                return self.splash_color.or_else(|| self.primary(0.16));
            }
            if states.contains(&WidgetState::Hovered) {
                return self.hover_color.or_else(|| self.primary(0.04));
            }
            if states.contains(&WidgetState::Focused) {
                return self.focus_color.or_else(|| self.primary(0.12));
            }
        } else if self.unselected {
            if states.contains(&WidgetState::Pressed) {
                return self
                    .splash_color
                    .or(self.highlight_color)
                    .or_else(|| self.on_surface(0.16));
            }
            if states.contains(&WidgetState::Hovered) {
                return self.hover_color.or_else(|| self.on_surface(0.04));
            }
            if states.contains(&WidgetState::Focused) {
                return self.focus_color.or_else(|| self.on_surface(0.12));
            }
        }
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Debug for ToggleButtonDefaultOverlay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{\n  selected:\n    hovered: {:?}, otherwise: {:?},\n    focused: {:?}, otherwise: {:?},\n    pressed: {:?}, otherwise: {:?},\n  unselected:\n    hovered: {:?}, otherwise: {:?},\n    focused: {:?}, otherwise: {:?},\n    pressed: {:?}, otherwise: {:?},\n  otherwise: null,\n}}",
            self.hover_color,
            self.primary(0.04),
            self.focus_color,
            self.primary(0.12),
            self.splash_color,
            self.primary(0.16),
            self.hover_color,
            self.on_surface(0.04),
            self.focus_color,
            self.on_surface(0.12),
            self.splash_color,
            self.on_surface(0.16),
        )
    }
}

/// Dart's `_SelectToggleButton`.
#[derive(Debug)]
struct SelectToggleButton {
    /// The width and color of the button's leading side border.
    leading_border_side: BorderSide,

    /// The width and color of the side borders.
    ///
    /// If [`direction`](Self::direction) is [`Axis::Horizontal`], this corresponds to the width
    /// and color of the button's top and bottom side borders.
    ///
    /// If [`direction`](Self::direction) is [`Axis::Vertical`], this corresponds to the width
    /// and color of the button's left and right side borders.
    border_side: BorderSide,

    /// The width and color of the button's trailing side border.
    trailing_border_side: BorderSide,

    /// The border radii of each corner of the button.
    border_radius: BorderRadius,

    /// Whether or not this toggle button is the first button in the list.
    is_first_button: bool,

    /// Whether or not this toggle button is the last button in the list.
    is_last_button: bool,

    /// The direction along which the buttons are rendered.
    direction: Axis,

    /// If [`direction`](Self::direction) is [`Axis::Vertical`], this property defines whether
    /// or not this button in its list of buttons is laid out starting from top to bottom or
    /// from bottom to top.
    vertical_direction: VerticalDirection,

    child: WidgetRef,
}

impl RenderObjectWidget for SelectToggleButton {
    type RenderObject = SelectToggleButtonRenderObject;

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::of(app, context);
        SelectToggleButtonRenderObject::new(
            app,
            self.leading_border_side,
            self.border_side,
            self.trailing_border_side,
            self.border_radius,
            self.is_first_button,
            self.is_last_button,
            self.direction,
            self.vertical_direction,
            text_direction,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<SelectToggleButtonRenderObject>,
    ) {
        render_object.set_leading_border_side(app, self.leading_border_side);
        render_object.set_border_side(app, self.border_side);
        render_object.set_trailing_border_side(app, self.trailing_border_side);
        render_object.set_border_radius(app, self.border_radius);
        render_object.set_is_first_button(app, self.is_first_button);
        render_object.set_is_last_button(app, self.is_last_button);
        render_object.set_direction(app, self.direction);
        render_object.set_vertical_direction(app, self.vertical_direction);
        let text_direction = Directionality::of(app, context);
        render_object.set_text_direction(app, text_direction);
    }
}

impl SingleChildRenderObjectWidget for SelectToggleButton {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

/// Dart's `_SelectToggleButtonRenderObject`.
pub(crate) struct SelectToggleButtonRenderObject {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    direction: Axis,
    vertical_direction: VerticalDirection,
    leading_border_side: BorderSide,
    border_side: BorderSide,
    trailing_border_side: BorderSide,
    border_radius: BorderRadius,
    is_first_button: bool,
    is_last_button: bool,
    text_direction: waft_painting::TextDirection,
}

/// Dart's `Path.addArc(oval, startAngle, sweepAngle)` on the path being built.
///
/// A degenerate oval adds nothing.
fn add_arc(path: &mut PathBuilder, oval: Rect, start_angle: f64, sweep_angle: f64) {
    if oval.width() == 0.0 && oval.height() == 0.0 {
        return;
    }
    let center = oval.center();
    path.ellipse(
        center,
        [(oval.width() / 2.0) as f32, (oval.height() / 2.0) as f32],
        0.0,
        start_angle as f32,
        sweep_angle as f32,
    );
}

impl SelectToggleButtonRenderObject {
    #[expect(
        clippy::too_many_arguments,
        reason = "Dart's constructor takes them positionally"
    )]
    fn new(
        app: &mut App,
        leading_border_side: BorderSide,
        border_side: BorderSide,
        trailing_border_side: BorderSide,
        border_radius: BorderRadius,
        is_first_button: bool,
        is_last_button: bool,
        direction: Axis,
        vertical_direction: VerticalDirection,
        text_direction: waft_painting::TextDirection,
    ) -> RenderHandle<SelectToggleButtonRenderObject> {
        RenderHandle::new_box(
            app,
            SelectToggleButtonRenderObject {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                direction,
                vertical_direction,
                leading_border_side,
                border_side,
                trailing_border_side,
                border_radius,
                is_first_button,
                is_last_button,
                text_direction,
            },
        )
    }

    fn direction(self: RenderHandle<Self>, app: &App) -> Axis {
        self.get(app).direction
    }

    fn set_direction(self: RenderHandle<Self>, app: &mut App, value: Axis) {
        if self.direction(app) == value {
            return;
        }
        self.get_mut(app).direction = value;
        self.mark_needs_layout(app);
    }

    fn vertical_direction(self: RenderHandle<Self>, app: &App) -> VerticalDirection {
        self.get(app).vertical_direction
    }

    fn set_vertical_direction(self: RenderHandle<Self>, app: &mut App, value: VerticalDirection) {
        if self.vertical_direction(app) == value {
            return;
        }
        self.get_mut(app).vertical_direction = value;
        self.mark_needs_layout(app);
    }

    // The width and color of the button's leading side border.
    fn leading_border_side(self: RenderHandle<Self>, app: &App) -> BorderSide {
        self.get(app).leading_border_side
    }

    fn set_leading_border_side(self: RenderHandle<Self>, app: &mut App, value: BorderSide) {
        if self.leading_border_side(app) == value {
            return;
        }
        self.get_mut(app).leading_border_side = value;
        self.mark_needs_layout(app);
    }

    // The width and color of the button's top and bottom side borders.
    fn border_side(self: RenderHandle<Self>, app: &App) -> BorderSide {
        self.get(app).border_side
    }

    fn set_border_side(self: RenderHandle<Self>, app: &mut App, value: BorderSide) {
        if self.border_side(app) == value {
            return;
        }
        self.get_mut(app).border_side = value;
        self.mark_needs_layout(app);
    }

    // The width and color of the button's trailing side border.
    fn trailing_border_side(self: RenderHandle<Self>, app: &App) -> BorderSide {
        self.get(app).trailing_border_side
    }

    fn set_trailing_border_side(self: RenderHandle<Self>, app: &mut App, value: BorderSide) {
        if self.trailing_border_side(app) == value {
            return;
        }
        self.get_mut(app).trailing_border_side = value;
        self.mark_needs_layout(app);
    }

    // The border radii of each corner of the button.
    fn border_radius(self: RenderHandle<Self>, app: &App) -> BorderRadius {
        self.get(app).border_radius
    }

    fn set_border_radius(self: RenderHandle<Self>, app: &mut App, value: BorderRadius) {
        if self.border_radius(app) == value {
            return;
        }
        self.get_mut(app).border_radius = value;
        self.mark_needs_layout(app);
    }

    // Whether or not this toggle button is the first button in the list.
    fn is_first_button(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).is_first_button
    }

    fn set_is_first_button(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.is_first_button(app) == value {
            return;
        }
        self.get_mut(app).is_first_button = value;
        self.mark_needs_layout(app);
    }

    // Whether or not this toggle button is the last button in the list.
    fn is_last_button(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).is_last_button
    }

    fn set_is_last_button(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.is_last_button(app) == value {
            return;
        }
        self.get_mut(app).is_last_button = value;
        self.mark_needs_layout(app);
    }

    // The direction in which text flows for this application.
    fn text_direction(self: RenderHandle<Self>, app: &App) -> waft_painting::TextDirection {
        self.get(app).text_direction
    }

    fn set_text_direction(
        self: RenderHandle<Self>,
        app: &mut App,
        value: waft_painting::TextDirection,
    ) {
        if self.text_direction(app) == value {
            return;
        }
        self.get_mut(app).text_direction = value;
        self.mark_needs_layout(app);
    }

    fn max_height(app: &mut App, child: Option<AnyRenderBox>, width: f64) -> f64 {
        child.map_or(0.0, |child| child.get_max_intrinsic_height(app, width))
    }

    fn min_height(app: &mut App, child: Option<AnyRenderBox>, width: f64) -> f64 {
        child.map_or(0.0, |child| child.get_min_intrinsic_height(app, width))
    }

    fn min_width(app: &mut App, child: Option<AnyRenderBox>, height: f64) -> f64 {
        child.map_or(0.0, |child| child.get_min_intrinsic_width(app, height))
    }

    fn max_width(app: &mut App, child: Option<AnyRenderBox>, height: f64) -> f64 {
        child.map_or(0.0, |child| child.get_max_intrinsic_width(app, height))
    }

    fn child_padding(self: RenderHandle<Self>, app: &App) -> EdgeInsetsDirectional {
        debug_assert!(self.child(app).is_some());
        let (leading, border, trailing) = (
            self.leading_border_side(app),
            self.border_side(app),
            self.trailing_border_side(app),
        );
        // It does not matter what [textDirection] or [verticalDirection] is,
        // since deflating the size constraints horizontally/vertically
        // and the returned size accounts for the width of both sides.
        match self.direction(app) {
            Axis::Horizontal => EdgeInsetsDirectional::only(
                leading.width,
                border.width,
                trailing.width,
                border.width,
            ),
            Axis::Vertical => EdgeInsetsDirectional::only(
                border.width,
                leading.width,
                border.width,
                trailing.width,
            ),
        }
    }

    fn compute_size(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        layout_child: ChildLayouter,
    ) -> Size {
        let Some(child) = self.child(app) else {
            let horizontal_size = Size::new(
                self.leading_border_side(app).width + self.trailing_border_side(app).width,
                self.border_side(app).width * 2.0,
            );
            return match self.direction(app) {
                Axis::Horizontal => constraints.constrain(horizontal_size),
                Axis::Vertical => constraints.constrain(horizontal_size.flipped()),
            };
        };

        let child_padding = EdgeInsetsGeometry::from(self.child_padding(app));
        let inner_constraints = constraints.deflate(child_padding);
        let child_size = layout_child(app, child, inner_constraints);
        constraints.constrain(child_padding.inflate_size(child_size))
    }
}

impl RenderObjectWithChildMixin for SelectToggleButtonRenderObject {
    type ChildType = AnyRenderBox;

    fn child_data(self: RenderHandle<Self>, app: &App) -> &RenderObjectWithChildData<AnyRenderBox> {
        &self.get(app).child
    }

    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
        &mut self.get_mut(app).child
    }
}

impl RenderShiftedBox for SelectToggleButtonRenderObject {}

impl RenderObject for SelectToggleButtonRenderObject {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let size = self.compute_size(app, constraints, ChildLayoutHelper::layout_child);
        self.set_size(app, size);
        let Some(child) = self.child(app) else {
            return;
        };
        let (leading, border, trailing) = (
            self.leading_border_side(app),
            self.border_side(app),
            self.trailing_border_side(app),
        );
        let offset = if self.direction(app) == Axis::Horizontal {
            match self.text_direction(app) {
                waft_painting::TextDirection::Ltr => Offset::new(leading.width, border.width),
                waft_painting::TextDirection::Rtl => Offset::new(trailing.width, border.width),
            }
        } else {
            match self.vertical_direction(app) {
                VerticalDirection::Down => Offset::new(border.width, leading.width),
                VerticalDirection::Up => Offset::new(border.width, trailing.width),
            }
        };
        child.parent_data_of_mut::<BoxParentData>(app).offset = offset;
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderShiftedBox::paint(self, app, context, offset);
        let size = self.size(app);
        let (leading_border_side, border_side, trailing_border_side) = (
            self.leading_border_side(app),
            self.border_side(app),
            self.trailing_border_side(app),
        );
        let (border_radius, is_first_button, is_last_button) = (
            self.border_radius(app),
            self.is_first_button(app),
            self.is_last_button(app),
        );
        let (direction, vertical_direction, text_direction) = (
            self.direction(app),
            self.vertical_direction(app),
            self.text_direction(app),
        );
        let bottom_right = size.bottom_right(offset);
        let outer = Rect::from_ltrb(
            offset.dx(),
            offset.dy(),
            bottom_right.dx(),
            bottom_right.dy(),
        );
        let center = outer.deflate(border_side.width / 2.0);
        const SWEEP_ANGLE: f64 = PI / 2.0;
        let corner = |radius: Radius| {
            if radius.x * radius.y != 0.0 {
                radius
            } else {
                Radius::ZERO
            }
        };
        let rrect = RRect::from_rect_and_corners(
            center,
            corner(border_radius.top_left),
            corner(border_radius.top_right),
            corner(border_radius.bottom_right),
            corner(border_radius.bottom_left),
        )
        .scale_radii();

        let tl_corner = Rect::from_ltwh(
            rrect.left,
            rrect.top,
            rrect.tl_radius_x * 2.0,
            rrect.tl_radius_y * 2.0,
        );
        let bl_corner = Rect::from_ltwh(
            rrect.left,
            rrect.bottom - (rrect.bl_radius_y * 2.0),
            rrect.bl_radius_x * 2.0,
            rrect.bl_radius_y * 2.0,
        );
        let tr_corner = Rect::from_ltwh(
            rrect.right - (rrect.tr_radius_x * 2.0),
            rrect.top,
            rrect.tr_radius_x * 2.0,
            rrect.tr_radius_y * 2.0,
        );
        let br_corner = Rect::from_ltwh(
            rrect.right - (rrect.br_radius_x * 2.0),
            rrect.bottom - (rrect.br_radius_y * 2.0),
            rrect.br_radius_x * 2.0,
            rrect.br_radius_y * 2.0,
        );

        let leading_paint = leading_border_side.to_paint();
        let draw =
            |context: &mut PaintingContext, path: PathBuilder, paint: &waft_embedder::Paint| {
                context
                    .canvas()
                    .draw_path(&path.build(), FillRule::NonZero, paint);
            };
        // Only one button.
        if is_first_button && is_last_button {
            let mut leading_path = PathBuilder::new();
            let start_x = if rrect.br_radius_x == 0.0 {
                outer.right
            } else {
                rrect.right - rrect.br_radius_x
            };
            leading_path
                .move_to(Offset::new(start_x, rrect.bottom))
                .line_to(Offset::new(rrect.left + rrect.bl_radius_x, rrect.bottom));
            add_arc(&mut leading_path, bl_corner, PI / 2.0, SWEEP_ANGLE);
            leading_path.line_to(Offset::new(rrect.left, rrect.top + rrect.tl_radius_y));
            add_arc(&mut leading_path, tl_corner, PI, SWEEP_ANGLE);
            leading_path.line_to(Offset::new(rrect.right - rrect.tr_radius_x, rrect.top));
            add_arc(&mut leading_path, tr_corner, PI * 3.0 / 2.0, SWEEP_ANGLE);
            leading_path.line_to(Offset::new(rrect.right, rrect.bottom - rrect.br_radius_y));
            add_arc(&mut leading_path, br_corner, 0.0, SWEEP_ANGLE);
            draw(context, leading_path, &leading_paint);
            return;
        }

        if direction == Axis::Horizontal {
            match text_direction {
                waft_painting::TextDirection::Ltr => {
                    if is_last_button {
                        let mut left_path = PathBuilder::new();
                        left_path
                            .move_to(Offset::new(
                                rrect.left,
                                rrect.bottom + leading_border_side.width / 2.0,
                            ))
                            .line_to(Offset::new(
                                rrect.left,
                                rrect.top - leading_border_side.width / 2.0,
                            ));
                        draw(context, left_path, &leading_paint);

                        let ending_paint = trailing_border_side.to_paint();
                        let mut ending_path = PathBuilder::new();
                        ending_path
                            .move_to(Offset::new(rrect.left + border_side.width / 2.0, rrect.top))
                            .line_to(Offset::new(rrect.right - rrect.tr_radius_x, rrect.top));
                        add_arc(&mut ending_path, tr_corner, PI * 3.0 / 2.0, SWEEP_ANGLE);
                        ending_path
                            .line_to(Offset::new(rrect.right, rrect.bottom - rrect.br_radius_y));
                        add_arc(&mut ending_path, br_corner, 0.0, SWEEP_ANGLE);
                        ending_path.line_to(Offset::new(
                            rrect.left + border_side.width / 2.0,
                            rrect.bottom,
                        ));
                        draw(context, ending_path, &ending_paint);
                    } else if is_first_button {
                        let mut leading_path = PathBuilder::new();
                        leading_path
                            .move_to(Offset::new(outer.right, rrect.bottom))
                            .line_to(Offset::new(rrect.left + rrect.bl_radius_x, rrect.bottom));
                        add_arc(&mut leading_path, bl_corner, PI / 2.0, SWEEP_ANGLE);
                        leading_path
                            .line_to(Offset::new(rrect.left, rrect.top + rrect.tl_radius_y));
                        add_arc(&mut leading_path, tl_corner, PI, SWEEP_ANGLE);
                        leading_path.line_to(Offset::new(outer.right, rrect.top));
                        draw(context, leading_path, &leading_paint);
                    } else {
                        let mut leading_path = PathBuilder::new();
                        leading_path
                            .move_to(Offset::new(
                                rrect.left,
                                rrect.bottom + leading_border_side.width / 2.0,
                            ))
                            .line_to(Offset::new(
                                rrect.left,
                                rrect.top - leading_border_side.width / 2.0,
                            ));
                        draw(context, leading_path, &leading_paint);

                        let horizontal_paint = border_side.to_paint();
                        let mut horizontal_paths = PathBuilder::new();
                        horizontal_paths
                            .move_to(Offset::new(rrect.left + border_side.width / 2.0, rrect.top))
                            .line_to(Offset::new(outer.right - rrect.tr_radius_x, rrect.top))
                            .move_to(Offset::new(
                                rrect.left + border_side.width / 2.0 + rrect.tl_radius_x,
                                rrect.bottom,
                            ))
                            .line_to(Offset::new(outer.right - rrect.tr_radius_x, rrect.bottom));
                        draw(context, horizontal_paths, &horizontal_paint);
                    }
                }
                waft_painting::TextDirection::Rtl => {
                    if is_last_button {
                        let mut leading_path = PathBuilder::new();
                        leading_path
                            .move_to(Offset::new(
                                rrect.right,
                                rrect.bottom + leading_border_side.width / 2.0,
                            ))
                            .line_to(Offset::new(
                                rrect.right,
                                rrect.top - leading_border_side.width / 2.0,
                            ));
                        draw(context, leading_path, &leading_paint);

                        let ending_paint = trailing_border_side.to_paint();
                        let mut ending_path = PathBuilder::new();
                        ending_path
                            .move_to(Offset::new(
                                rrect.right - border_side.width / 2.0,
                                rrect.top,
                            ))
                            .line_to(Offset::new(rrect.left + rrect.tl_radius_x, rrect.top));
                        add_arc(&mut ending_path, tl_corner, PI * 3.0 / 2.0, -SWEEP_ANGLE);
                        ending_path
                            .line_to(Offset::new(rrect.left, rrect.bottom - rrect.bl_radius_y));
                        add_arc(&mut ending_path, bl_corner, PI, -SWEEP_ANGLE);
                        ending_path.line_to(Offset::new(
                            rrect.right - border_side.width / 2.0,
                            rrect.bottom,
                        ));
                        draw(context, ending_path, &ending_paint);
                    } else if is_first_button {
                        let mut leading_path = PathBuilder::new();
                        leading_path
                            .move_to(Offset::new(outer.left, rrect.bottom))
                            .line_to(Offset::new(rrect.right - rrect.br_radius_x, rrect.bottom));
                        add_arc(&mut leading_path, br_corner, PI / 2.0, -SWEEP_ANGLE);
                        leading_path
                            .line_to(Offset::new(rrect.right, rrect.top + rrect.tr_radius_y));
                        add_arc(&mut leading_path, tr_corner, 0.0, -SWEEP_ANGLE);
                        leading_path.line_to(Offset::new(outer.left, rrect.top));
                        draw(context, leading_path, &leading_paint);
                    } else {
                        let mut leading_path = PathBuilder::new();
                        leading_path
                            .move_to(Offset::new(
                                rrect.right,
                                rrect.bottom + leading_border_side.width / 2.0,
                            ))
                            .line_to(Offset::new(
                                rrect.right,
                                rrect.top - leading_border_side.width / 2.0,
                            ));
                        draw(context, leading_path, &leading_paint);

                        let horizontal_paint = border_side.to_paint();
                        let mut horizontal_paths = PathBuilder::new();
                        horizontal_paths
                            .move_to(Offset::new(
                                rrect.right - border_side.width / 2.0,
                                rrect.top,
                            ))
                            .line_to(Offset::new(outer.left - rrect.tl_radius_x, rrect.top))
                            .move_to(Offset::new(
                                rrect.right - border_side.width / 2.0 + rrect.tr_radius_x,
                                rrect.bottom,
                            ))
                            .line_to(Offset::new(outer.left - rrect.tl_radius_x, rrect.bottom));
                        draw(context, horizontal_paths, &horizontal_paint);
                    }
                }
            }
        } else {
            match vertical_direction {
                VerticalDirection::Down => {
                    if is_last_button {
                        let mut top_path = PathBuilder::new();
                        top_path
                            .move_to(Offset::new(
                                outer.left,
                                outer.top + leading_border_side.width / 2.0,
                            ))
                            .line_to(Offset::new(
                                outer.right,
                                outer.top + leading_border_side.width / 2.0,
                            ));
                        draw(context, top_path, &leading_paint);

                        let ending_paint = trailing_border_side.to_paint();
                        let mut ending_path = PathBuilder::new();
                        ending_path
                            .move_to(Offset::new(
                                rrect.left,
                                rrect.top + leading_border_side.width / 2.0,
                            ))
                            .line_to(Offset::new(rrect.left, rrect.bottom - rrect.bl_radius_y));
                        add_arc(&mut ending_path, bl_corner, PI * 3.0, -SWEEP_ANGLE);
                        ending_path
                            .line_to(Offset::new(rrect.right - rrect.bl_radius_x, rrect.bottom));
                        add_arc(&mut ending_path, br_corner, PI / 2.0, -SWEEP_ANGLE);
                        ending_path.line_to(Offset::new(
                            rrect.right,
                            rrect.top + leading_border_side.width / 2.0,
                        ));
                        draw(context, ending_path, &ending_paint);
                    } else if is_first_button {
                        let mut leading_path = PathBuilder::new();
                        leading_path
                            .move_to(Offset::new(rrect.left, outer.bottom))
                            .line_to(Offset::new(rrect.left, rrect.top + rrect.tl_radius_x));
                        add_arc(&mut leading_path, tl_corner, PI, SWEEP_ANGLE);
                        leading_path
                            .line_to(Offset::new(rrect.right - rrect.tr_radius_x, rrect.top));
                        add_arc(&mut leading_path, tr_corner, PI * 3.0 / 2.0, SWEEP_ANGLE);
                        leading_path.line_to(Offset::new(rrect.right, outer.bottom));
                        draw(context, leading_path, &leading_paint);
                    } else {
                        let mut top_path = PathBuilder::new();
                        top_path
                            .move_to(Offset::new(
                                outer.left,
                                outer.top + leading_border_side.width / 2.0,
                            ))
                            .line_to(Offset::new(
                                outer.right,
                                outer.top + leading_border_side.width / 2.0,
                            ));
                        draw(context, top_path, &leading_paint);

                        let paint = border_side.to_paint();
                        let mut paths = PathBuilder::new(); // Left and right borders.
                        paths
                            .move_to(Offset::new(
                                rrect.left,
                                outer.top + leading_border_side.width,
                            ))
                            .line_to(Offset::new(rrect.left, outer.bottom))
                            .move_to(Offset::new(
                                rrect.right,
                                outer.top + leading_border_side.width,
                            ))
                            .line_to(Offset::new(rrect.right, outer.bottom));
                        draw(context, paths, &paint);
                    }
                }
                VerticalDirection::Up => {
                    if is_last_button {
                        let mut bottom_path = PathBuilder::new();
                        bottom_path
                            .move_to(Offset::new(
                                outer.left,
                                outer.bottom - leading_border_side.width / 2.0,
                            ))
                            .line_to(Offset::new(
                                outer.right,
                                outer.bottom - leading_border_side.width / 2.0,
                            ));
                        draw(context, bottom_path, &leading_paint);

                        let ending_paint = trailing_border_side.to_paint();
                        let mut ending_path = PathBuilder::new();
                        ending_path
                            .move_to(Offset::new(
                                rrect.left,
                                rrect.bottom - leading_border_side.width / 2.0,
                            ))
                            .line_to(Offset::new(rrect.left, rrect.top + rrect.tl_radius_y));
                        add_arc(&mut ending_path, tl_corner, PI, SWEEP_ANGLE);
                        ending_path
                            .line_to(Offset::new(rrect.right - rrect.tr_radius_x, rrect.top));
                        add_arc(&mut ending_path, tr_corner, PI * 3.0 / 2.0, SWEEP_ANGLE);
                        ending_path.line_to(Offset::new(
                            rrect.right,
                            rrect.bottom - leading_border_side.width / 2.0,
                        ));
                        draw(context, ending_path, &ending_paint);
                    } else if is_first_button {
                        let mut leading_path = PathBuilder::new();
                        leading_path
                            .move_to(Offset::new(rrect.left, outer.top))
                            .line_to(Offset::new(rrect.left, rrect.bottom - rrect.bl_radius_y));
                        add_arc(&mut leading_path, bl_corner, PI, -SWEEP_ANGLE);
                        leading_path
                            .line_to(Offset::new(rrect.right - rrect.br_radius_x, rrect.bottom));
                        add_arc(&mut leading_path, br_corner, PI / 2.0, -SWEEP_ANGLE);
                        leading_path.line_to(Offset::new(rrect.right, outer.top));
                        draw(context, leading_path, &leading_paint);
                    } else {
                        let mut bottom_path = PathBuilder::new();
                        bottom_path
                            .move_to(Offset::new(
                                outer.left,
                                outer.bottom - leading_border_side.width / 2.0,
                            ))
                            .line_to(Offset::new(
                                outer.right,
                                outer.bottom - leading_border_side.width / 2.0,
                            ));
                        draw(context, bottom_path, &leading_paint);

                        let paint = border_side.to_paint();
                        let mut paths = PathBuilder::new(); // Left and right borders.
                        paths
                            .move_to(Offset::new(rrect.left, outer.top))
                            .line_to(Offset::new(
                                rrect.left,
                                outer.bottom - leading_border_side.width,
                            ))
                            .move_to(Offset::new(rrect.right, outer.top))
                            .line_to(Offset::new(
                                rrect.right,
                                outer.bottom - leading_border_side.width,
                            ));
                        draw(context, paths, &paint);
                    }
                }
            }
        }
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child.as_object());
        }
    }
}

impl RenderBox for SelectToggleButtonRenderObject {
    waft_rendering::render_box_accessors!();

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderShiftedBox::hit_test_children(self, app, result, position)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        // The baseline of this widget is the baseline of its child
        let child_offset = BaselineOffset(
            self.child(app)
                .and_then(|child| child.get_distance_to_actual_baseline(app, baseline)),
        );
        match self.direction(app) {
            Axis::Horizontal => child_offset + self.border_side(app).width,
            Axis::Vertical => {
                child_offset
                    + match self.vertical_direction(app) {
                        VerticalDirection::Down => self.leading_border_side(app).width,
                        VerticalDirection::Up => self.trailing_border_side(app).width,
                    }
            }
        }
        .offset()
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let child = self.child(app);
        let child_height = Self::max_height(app, child, width);
        let (leading, border, trailing) = (
            self.leading_border_side(app),
            self.border_side(app),
            self.trailing_border_side(app),
        );
        if self.direction(app) == Axis::Horizontal {
            border.width * 2.0 + child_height
        } else {
            leading.width + child_height + trailing.width
        }
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let child = self.child(app);
        let (leading, border, trailing) = (
            self.leading_border_side(app),
            self.border_side(app),
            self.trailing_border_side(app),
        );
        if self.direction(app) == Axis::Horizontal {
            border.width * 2.0 + Self::min_height(app, child, width)
        } else {
            leading.width + Self::max_height(app, child, width) + trailing.width
        }
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let child = self.child(app);
        let (leading, border, trailing) = (
            self.leading_border_side(app),
            self.border_side(app),
            self.trailing_border_side(app),
        );
        if self.direction(app) == Axis::Horizontal {
            leading.width + Self::max_width(app, child, height) + trailing.width
        } else {
            border.width * 2.0 + Self::max_width(app, child, height)
        }
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let child = self.child(app);
        let (leading, border, trailing) = (
            self.leading_border_side(app),
            self.border_side(app),
            self.trailing_border_side(app),
        );
        if self.direction(app) == Axis::Horizontal {
            leading.width + Self::min_width(app, child, height) + trailing.width
        } else {
            border.width * 2.0 + Self::min_width(app, child, height)
        }
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        self.compute_size(app, constraints, ChildLayoutHelper::dry_layout_child)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let child_padding = EdgeInsetsGeometry::from(self.child_padding(app));
        let child_baseline =
            child.get_dry_baseline(app, constraints.deflate(child_padding), baseline)?;
        Some(
            child_baseline
                + match self.direction(app) {
                    Axis::Horizontal => self.border_side(app).width,
                    Axis::Vertical => match self.vertical_direction(app) {
                        VerticalDirection::Down => self.leading_border_side(app).width,
                        VerticalDirection::Up => self.trailing_border_side(app).width,
                    },
                },
        )
    }
}

/// A widget to pad the area around a [`ToggleButtons`]'s inner `TextButton`.
///
/// Redirect taps that occur in the padded area around the child to the center
/// of the child. This increases the size of the button and the button's
/// "tap target", but not its material or its ink splashes.
///
/// Dart's `_InputPadding`.
#[derive(Debug)]
struct InputPadding {
    child: Option<WidgetRef>,
    min_size: Size,
    direction: Axis,
}

impl RenderObjectWidget for InputPadding {
    type RenderObject = RenderInputPadding;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderInputPadding::new(app, self.min_size, self.direction).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderInputPadding>,
    ) {
        render_object.set_min_size(app, self.min_size);
        render_object.set_direction(app, self.direction);
    }
}

impl SingleChildRenderObjectWidget for InputPadding {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// Dart's `_RenderInputPadding`.
struct RenderInputPadding {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    min_size: Size,
    direction: Axis,
}

impl RenderInputPadding {
    fn new(app: &mut App, min_size: Size, direction: Axis) -> RenderHandle<RenderInputPadding> {
        RenderHandle::new_box(
            app,
            RenderInputPadding {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                min_size,
                direction,
            },
        )
    }

    fn min_size(self: RenderHandle<Self>, app: &App) -> Size {
        self.get(app).min_size
    }

    fn set_min_size(self: RenderHandle<Self>, app: &mut App, value: Size) {
        if self.min_size(app) == value {
            return;
        }
        self.get_mut(app).min_size = value;
        self.mark_needs_layout(app);
    }

    fn direction(self: RenderHandle<Self>, app: &App) -> Axis {
        self.get(app).direction
    }

    fn set_direction(self: RenderHandle<Self>, app: &mut App, value: Axis) {
        if self.direction(app) == value {
            return;
        }
        self.get_mut(app).direction = value;
        self.mark_needs_layout(app);
    }

    fn compute_size(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        layout_child: ChildLayouter,
    ) -> Size {
        if let Some(child) = self.child(app) {
            let min_size = self.min_size(app);
            let child_size = layout_child(app, child, constraints);
            let width = child_size.width().max(min_size.width());
            let height = child_size.height().max(min_size.height());
            return constraints.constrain(Size::new(width, height));
        }
        Size::ZERO
    }
}

impl RenderObjectWithChildMixin for RenderInputPadding {
    type ChildType = AnyRenderBox;

    fn child_data(self: RenderHandle<Self>, app: &App) -> &RenderObjectWithChildData<AnyRenderBox> {
        &self.get(app).child
    }

    fn child_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderObjectWithChildData<AnyRenderBox> {
        &mut self.get_mut(app).child
    }
}

impl RenderShiftedBox for RenderInputPadding {}

impl RenderObject for RenderInputPadding {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let size = self.compute_size(app, constraints, ChildLayoutHelper::layout_child);
        self.set_size(app, size);
        if let Some(child) = self.child(app) {
            let child_size = child.size(app);
            child.parent_data_of_mut::<BoxParentData>(app).offset =
                Alignment::CENTER.along_offset(size - child_size);
        }
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderShiftedBox::paint(self, app, context, offset);
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        if let Some(child) = self.child(app) {
            visitor(child.as_object());
        }
    }
}

impl RenderBox for RenderInputPadding {
    waft_rendering::render_box_accessors!();

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        if let Some(child) = self.child(app) {
            return child
                .get_min_intrinsic_width(app, height)
                .max(self.min_size(app).width());
        }
        0.0
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        if let Some(child) = self.child(app) {
            return child
                .get_min_intrinsic_height(app, width)
                .max(self.min_size(app).height());
        }
        0.0
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        if let Some(child) = self.child(app) {
            return child
                .get_max_intrinsic_width(app, height)
                .max(self.min_size(app).width());
        }
        0.0
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        if let Some(child) = self.child(app) {
            return child
                .get_max_intrinsic_height(app, width)
                .max(self.min_size(app).height());
        }
        0.0
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        self.compute_size(app, constraints, ChildLayoutHelper::dry_layout_child)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;
        let result = child.get_dry_baseline(app, constraints, baseline)?;
        // Calculate the size and child offset using the same logic as performLayout
        let dry_size = self.get_dry_layout(app, constraints);
        let child_size = child.get_dry_layout(app, constraints);
        let child_offset = Alignment::CENTER.along_offset(dry_size - child_size);
        Some(result + child_offset.dy())
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderShiftedBox::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderShiftedBox::hit_test_children(self, app, result, position)
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        // The super.hitTest() method also checks hitTestChildren(). We don't
        // want that in this case because we've padded around the children per
        // tapTargetSize.
        if !self.size(app).contains(position) {
            return false;
        }

        let child = self.child(app).expect("a hit on the padding needs a child");
        let child_size = child.size(app);
        // Only adjust one axis to ensure the correct button is tapped.
        let center = match self.direction(app) {
            Axis::Horizontal => Offset::new(position.dx(), child_size.height() / 2.0),
            Axis::Vertical => Offset::new(child_size.width() / 2.0, position.dy()),
        };
        result.add_with_raw_transform(
            Some(waft_painting::force_to_point(center)),
            center,
            |result, position| {
                debug_assert!(
                    (position.dx() - center.dx()).abs() <= center.dx().abs() * f32::EPSILON as f64
                        && (position.dy() - center.dy()).abs()
                            <= center.dy().abs() * f32::EPSILON as f64,
                    "the forced transform maps every hit to the center"
                );
                child.hit_test(app, result, center)
            },
        )
    }
}

#[cfg(test)]
pub(crate) mod tests {
    //! Ported from `test/toggle_buttons_test.dart`; the goldens and the semantics-tree cases are
    //! not ported.

    use waft_painting::TextDirection;
    use waft_widgets::{Container, DefaultTextStyle, Icon, IconTheme, Text, WidgetStateColor};

    use waft_widgets::testing::{TestPointer, WidgetTester, find, test_widgets};

    use super::*;
    use crate::colors::Colors;
    use crate::icons::Icons;
    use crate::material::{Material, MaterialType};
    use crate::test_support::{PaintInspection, path_like};
    use crate::theme_data::ThemeData;
    use waft_rendering::testing::RecordedCall;
    use waft_services::SystemMouseCursors;
    use waft_widgets::Directionality as DirectionalityWidget;
    use waft_widgets::FocusNodeLeaf;
    use waft_widgets::SizedBox;

    const DEFAULT_BORDER_WIDTH: f64 = 1.0;

    /// Dart's `boilerplate`.
    fn boilerplate<K>(theme: ThemeData, child: impl IntoWidget<K>) -> WidgetRef {
        let child = child.into_widget();
        Theme::new(
            theme,
            DirectionalityWidget::new(
                TextDirection::Ltr,
                Center::new().child(Material::new().child(child)),
            ),
        )
        .into_widget()
    }

    pub(crate) fn default_theme(tester: &WidgetTester) -> ThemeData {
        ThemeData::new(&tester.cell.borrow())
    }

    fn themed(tester: &WidgetTester, tap_target_size: MaterialTapTargetSize) -> ThemeData {
        ThemeData::builder()
            .material_tap_target_size(tap_target_size)
            .build(&tester.cell.borrow())
    }

    pub(crate) fn texts(values: &[&str]) -> Vec<WidgetRef> {
        values
            .iter()
            .map(|value| Text::new(*value).into_widget())
            .collect()
    }

    pub(crate) fn text_button_of(text: &str) -> waft_widgets::testing::Finder {
        find::widget_with_text::<TextButton>(text)
    }

    /// `tester.widget<DefaultTextStyle>(find.descendant(of: find.widgetWithText(TextButton,
    /// text), matching: find.byType(DefaultTextStyle))).style`.
    pub(crate) fn button_text_style(tester: &WidgetTester, text: &str) -> TextStyle {
        tester.widget(
            find::descendant(text_button_of(text), find::by_type::<DefaultTextStyle>()),
            |style: &DefaultTextStyle| style.style.clone(),
        )
    }

    pub(crate) fn style_color(style: &TextStyle) -> Option<Color> {
        style.color.as_deref().copied()
    }

    /// The `IconTheme` of `icon` inside its `TextButton`: the last under it.
    pub(crate) fn icon_theme_color(
        tester: &WidgetTester,
        icon: &waft_widgets::IconData,
    ) -> Option<Color> {
        let theme = find::descendant(
            find::widget_with_icon::<TextButton>(icon),
            find::by_type::<IconTheme>(),
        )
        .last();
        tester.widget(theme, |theme: &IconTheme| {
            theme.data.color.as_deref().copied()
        })
    }

    pub(crate) fn button_rect(tester: &WidgetTester, index: usize) -> Rect {
        let element = find::by_type::<TextButton>().evaluate(tester)[index];
        tester.get_rect(element)
    }

    // 'Initial toggle state is reflected'
    #[test]
    fn the_initial_toggle_state_is_reflected() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            tester.pump_widget(boilerplate(
                theme.clone(),
                ToggleButtons::new(texts(&["First child", "Second child"]), vec![false, true])
                    .on_pressed(|_app, _index| {}),
            ));

            assert_eq!(
                style_color(&button_text_style(tester, "First child")),
                Some(theme.color_scheme.on_surface().with_opacity(0.87))
            );
            assert_eq!(
                style_color(&button_text_style(tester, "Second child")),
                Some(theme.color_scheme.primary())
            );
        });
    }

    // 'onPressed is triggered on button tap'
    #[test]
    fn on_pressed_is_triggered_on_button_tap() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            let is_selected = Rc::new(std::cell::RefCell::new(vec![false, true]));
            let root = {
                let theme = theme.clone();
                let is_selected = Rc::clone(&is_selected);
                waft_widgets::StatefulBuilder::new(move |_app, _context, set_state| {
                    let toggled = Rc::clone(&is_selected);
                    boilerplate(
                        theme.clone(),
                        ToggleButtons::new(
                            texts(&["First child", "Second child"]),
                            is_selected.borrow().clone(),
                        )
                        .on_pressed(move |app, index| {
                            let selected = toggled.borrow()[index];
                            toggled.borrow_mut()[index] = !selected;
                            set_state(app, &mut |_| {});
                        }),
                    )
                })
                .into_widget()
            };
            tester.pump_widget(root);

            assert_eq!(*is_selected.borrow(), [false, true]);
            assert_eq!(
                style_color(&button_text_style(tester, "First child")),
                Some(theme.color_scheme.on_surface().with_opacity(0.87))
            );
            assert_eq!(
                style_color(&button_text_style(tester, "Second child")),
                Some(theme.color_scheme.primary())
            );

            let center = text_center(tester, "Second child");
            tester.tap_at(center, 1);
            tester.pump_and_settle();

            assert_eq!(*is_selected.borrow(), [false, false]);
            assert_eq!(
                style_color(&button_text_style(tester, "First child")),
                Some(theme.color_scheme.on_surface().with_opacity(0.87))
            );
            assert_eq!(
                style_color(&button_text_style(tester, "Second child")),
                Some(theme.color_scheme.on_surface().with_opacity(0.87))
            );
        });
    }

    pub(crate) fn text_center(tester: &WidgetTester, text: &str) -> Offset {
        let element = find::text(text).evaluate(tester)[0];
        tester.get_rect(element).center()
    }

    // 'onPressed that is null disables buttons'
    #[test]
    fn a_null_on_pressed_disables_the_buttons() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            tester.pump_widget(boilerplate(
                theme.clone(),
                ToggleButtons::new(texts(&["First child", "Second child"]), vec![false, true]),
            ));

            let disabled = Some(theme.color_scheme.on_surface().with_opacity(0.38));
            assert_eq!(
                style_color(&button_text_style(tester, "First child")),
                disabled
            );
            assert_eq!(
                style_color(&button_text_style(tester, "Second child")),
                disabled
            );

            let center = text_center(tester, "Second child");
            tester.tap_at(center, 1);
            tester.pump_and_settle();

            // Nothing should change
            assert_eq!(
                style_color(&button_text_style(tester, "First child")),
                disabled
            );
            assert_eq!(
                style_color(&button_text_style(tester, "Second child")),
                disabled
            );
        });
    }

    // 'children and isSelected properties have to be the same length'
    #[test]
    fn children_and_is_selected_have_to_be_the_same_length() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                tester.pump_widget(boilerplate(
                    theme,
                    ToggleButtons::new(texts(&["First child", "Second child"]), vec![false]),
                ));
            }));
            let panic = outcome.expect_err("a length mismatch asserts");
            let message = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| {
                    panic
                        .downcast_ref::<&str>()
                        .map(|message| (*message).to_string())
                })
                .unwrap_or_default();
            assert!(message.contains("children.length"), "{message}");
            assert!(message.contains("isSelected.length"), "{message}");
        });
    }

    // 'Default text style is applied'
    #[test]
    fn the_default_text_style_is_applied() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            tester.pump_widget(boilerplate(
                theme.clone(),
                ToggleButtons::new(texts(&["First child", "Second child"]), vec![false, true])
                    .on_pressed(|_app, _index| {}),
            ));

            let body_medium = theme.text_theme.body_medium.clone().expect("a bodyMedium");
            for text in ["First child", "Second child"] {
                let text_style = button_text_style(tester, text);
                assert_eq!(text_style.font_family, body_medium.font_family);
                assert_eq!(text_style.decoration, body_medium.decoration);
            }
        });
    }

    // 'Custom text style except color is applied'
    #[test]
    fn a_custom_text_style_is_applied_except_its_color() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            tester.pump_widget(boilerplate(
                theme,
                ToggleButtons::new(texts(&["First child", "Second child"]), vec![false, true])
                    .on_pressed(|_app, _index| {})
                    .text_style(
                        TextStyle::new()
                            .text_baseline(waft_embedder::TextBaseline::Ideographic)
                            .font_size(20.0)
                            .color(Colors::ORANGE.primary()),
                    ),
            ));

            for text in ["First child", "Second child"] {
                let text_style = button_text_style(tester, text);
                assert_eq!(
                    text_style.text_baseline,
                    Some(waft_embedder::TextBaseline::Ideographic)
                );
                assert_eq!(text_style.font_size, Some(20.0));
                assert_ne!(style_color(&text_style), Some(Colors::ORANGE.primary()));
            }
        });
    }

    pub(crate) fn three_icons() -> Vec<WidgetRef> {
        [Icons::check(), Icons::access_alarm(), Icons::cake()]
            .into_iter()
            .map(|icon| Icon::new(Some(icon)).into_widget())
            .collect()
    }

    // 'Default BoxConstraints'
    #[test]
    fn the_default_box_constraints_make_buttons_48_by_48() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            tester.pump_widget(boilerplate(
                theme,
                ToggleButtons::new(three_icons(), vec![false; 3]).on_pressed(|_app, _index| {}),
            ));

            for index in 0..3 {
                let rect = button_rect(tester, index);
                assert_eq!(rect.width(), 48.0);
                assert_eq!(rect.height(), 48.0);
            }
        });
    }

    // 'Custom BoxConstraints'
    #[test]
    fn custom_box_constraints_size_the_buttons() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            // Test for minimum constraints
            tester.pump_widget(boilerplate(
                theme.clone(),
                ToggleButtons::new(three_icons(), vec![false; 3])
                    .constraints(BoxConstraints::new().min_width(50.0).min_height(60.0))
                    .on_pressed(|_app, _index| {}),
            ));
            for index in 0..3 {
                let rect = button_rect(tester, index);
                assert_eq!(rect.width(), 50.0);
                assert_eq!(rect.height(), 60.0);
            }

            // Test for maximum constraints
            tester.pump_widget(boilerplate(
                theme,
                ToggleButtons::new(three_icons(), vec![false; 3])
                    .tap_target_size(MaterialTapTargetSize::ShrinkWrap)
                    .constraints(BoxConstraints::new().max_width(20.0).max_height(10.0))
                    .on_pressed(|_app, _index| {}),
            ));
            for index in 0..3 {
                let rect = button_rect(tester, index);
                assert_eq!(rect.width(), 20.0);
                assert_eq!(rect.height(), 10.0);
            }
        });
    }

    pub(crate) fn row_with_check() -> Vec<WidgetRef> {
        vec![
            Row::new()
                .children([
                    Text::new("First child").into_widget(),
                    Icon::new(Some(Icons::check())).into_widget(),
                ])
                .into_widget(),
        ]
    }

    // 'Default text/icon colors for enabled, selected and disabled states'
    #[test]
    fn the_default_text_and_icon_colors_follow_the_state() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            let check = Icons::check();

            tester.pump_widget(boilerplate(
                theme.clone(),
                ToggleButtons::new(row_with_check(), vec![false]).on_pressed(|_app, _index| {}),
            ));
            // Default enabled color
            let enabled = Some(theme.color_scheme.on_surface().with_opacity(0.87));
            assert_eq!(
                style_color(&button_text_style(tester, "First child")),
                enabled
            );
            assert_eq!(icon_theme_color(tester, &check), enabled);

            tester.pump_widget(boilerplate(
                theme.clone(),
                ToggleButtons::new(row_with_check(), vec![true]).on_pressed(|_app, _index| {}),
            ));
            tester.pump_and_settle();
            // Default selected color
            let selected = Some(theme.color_scheme.primary());
            assert_eq!(
                style_color(&button_text_style(tester, "First child")),
                selected
            );
            assert_eq!(icon_theme_color(tester, &check), selected);

            tester.pump_widget(boilerplate(
                theme.clone(),
                ToggleButtons::new(row_with_check(), vec![true]),
            ));
            tester.pump_and_settle();
            // Default disabled color
            let disabled = Some(theme.color_scheme.on_surface().with_opacity(0.38));
            assert_eq!(
                style_color(&button_text_style(tester, "First child")),
                disabled
            );
            assert_eq!(icon_theme_color(tester, &check), disabled);
        });
    }

    // 'Custom text/icon colors for enabled, selected and disabled states'
    #[test]
    fn custom_text_and_icon_colors_follow_the_state() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            let check = Icons::check();
            let enabled_color = Colors::LIME.primary();
            let selected_color = Colors::GREEN.primary();
            let disabled_color = Colors::YELLOW.primary();

            // Tests are ineffective if the custom colors are the same as the theme's
            assert_ne!(theme.color_scheme.on_surface(), enabled_color);
            assert_ne!(theme.color_scheme.primary(), selected_color);
            assert_ne!(
                theme.color_scheme.on_surface().with_opacity(0.38),
                disabled_color
            );

            tester.pump_widget(boilerplate(
                theme.clone(),
                ToggleButtons::new(row_with_check(), vec![false])
                    .color(enabled_color)
                    .on_pressed(|_app, _index| {}),
            ));
            // Custom enabled color
            assert_eq!(
                style_color(&button_text_style(tester, "First child")),
                Some(enabled_color)
            );
            assert_eq!(icon_theme_color(tester, &check), Some(enabled_color));

            tester.pump_widget(boilerplate(
                theme.clone(),
                ToggleButtons::new(row_with_check(), vec![true])
                    .selected_color(selected_color)
                    .on_pressed(|_app, _index| {}),
            ));
            tester.pump_and_settle();
            // Custom selected color
            assert_eq!(
                style_color(&button_text_style(tester, "First child")),
                Some(selected_color)
            );
            assert_eq!(icon_theme_color(tester, &check), Some(selected_color));

            tester.pump_widget(boilerplate(
                theme,
                ToggleButtons::new(row_with_check(), vec![true]).disabled_color(disabled_color),
            ));
            tester.pump_and_settle();
            // Custom disabled color
            assert_eq!(
                style_color(&button_text_style(tester, "First child")),
                Some(disabled_color)
            );
            assert_eq!(icon_theme_color(tester, &check), Some(disabled_color));
        });
    }

    /// `tester.widget<Material>(find.descendant(of: find.byType(TextButton), matching:
    /// find.byType(Material)))`, or the one around `text`.
    pub(crate) fn button_material<R>(
        tester: &WidgetTester,
        text: Option<&str>,
        read: impl FnOnce(&Material) -> R,
    ) -> R {
        let button = match text {
            Some(text) => text_button_of(text),
            None => find::by_type::<TextButton>().first(),
        };
        let material = find::descendant(button, find::by_type::<Material>()).first();
        tester.widget(material, read)
    }

    pub(crate) fn single_button(selected: bool, enabled: bool) -> ToggleButtons {
        let buttons = ToggleButtons::new(
            vec![
                Row::new()
                    .children([Text::new("First child").into_widget()])
                    .into_widget(),
            ],
            vec![selected],
        );
        if enabled {
            buttons.on_pressed(|_app, _index| {})
        } else {
            buttons
        }
    }

    // 'Default button fillColor - unselected'
    #[test]
    fn the_default_fill_color_of_an_unselected_button() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            tester.pump_widget(boilerplate(theme.clone(), single_button(false, true)));

            let (color, material_type) =
                button_material(tester, None, |material| (material.color, material.r#type));
            assert_eq!(color, Some(theme.color_scheme.surface().with_opacity(0.0)));
            assert_eq!(material_type, MaterialType::Button);
        });
    }

    // 'Default button fillColor - selected'
    #[test]
    fn the_default_fill_color_of_a_selected_button() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            tester.pump_widget(boilerplate(theme.clone(), single_button(true, true)));

            let (color, material_type) =
                button_material(tester, None, |material| (material.color, material.r#type));
            assert_eq!(color, Some(theme.color_scheme.primary().with_opacity(0.12)));
            assert_eq!(material_type, MaterialType::Button);
        });
    }

    // 'Default button fillColor - disabled'
    #[test]
    fn the_default_fill_color_of_a_disabled_button() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            tester.pump_widget(boilerplate(theme.clone(), single_button(true, false)));

            let (color, material_type) =
                button_material(tester, None, |material| (material.color, material.r#type));
            assert_eq!(color, Some(theme.color_scheme.surface().with_opacity(0.0)));
            assert_eq!(material_type, MaterialType::Button);
        });
    }

    // 'Custom button fillColor'
    #[test]
    fn a_custom_fill_color_fills_a_selected_button() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            let custom_fill_color = Colors::GREEN.primary();
            tester.pump_widget(boilerplate(
                theme,
                single_button(true, true).fill_color(custom_fill_color),
            ));

            let (color, material_type) =
                button_material(tester, None, |material| (material.color, material.r#type));
            assert_eq!(color, Some(custom_fill_color));
            assert_eq!(material_type, MaterialType::Button);
        });
    }

    pub(crate) fn two_texts() -> Vec<WidgetRef> {
        texts(&["First child", "Second child"])
    }

    pub(crate) fn material_color(tester: &WidgetTester, text: &str) -> Option<Color> {
        button_material(tester, Some(text), |material| material.color)
    }

    // 'Custom button fillColor - Non WidgetState'
    #[test]
    fn a_plain_fill_color_fills_only_selected_enabled_buttons() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            let selected_fill_color = Colors::YELLOW.primary();
            let unselected = Some(theme.color_scheme.surface().with_opacity(0.0));

            tester.pump_widget(boilerplate(
                theme.clone(),
                ToggleButtons::new(two_texts(), vec![false, true])
                    .fill_color(selected_fill_color)
                    .on_pressed(|_app, _index| {}),
            ));
            tester.pump_and_settle();
            assert_eq!(material_color(tester, "First child"), unselected);
            assert_eq!(
                material_color(tester, "Second child"),
                Some(selected_fill_color)
            );

            tester.pump_widget(boilerplate(
                theme,
                ToggleButtons::new(two_texts(), vec![false, true]).fill_color(selected_fill_color),
            ));
            tester.pump_and_settle();
            assert_eq!(material_color(tester, "First child"), unselected);
            assert_eq!(material_color(tester, "Second child"), unselected);
        });
    }

    // 'Custom button fillColor - WidgetState'
    #[test]
    fn a_widget_state_fill_color_resolves_in_the_button_states() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            let selected_fill_color = Colors::ORANGE.primary();
            let default_fill_color = Colors::BLUE.primary();
            let get_fill_color = move |states: &WidgetStates| {
                if states.contains(&WidgetState::Selected) {
                    return selected_fill_color;
                }
                default_fill_color
            };

            tester.pump_widget(boilerplate(
                theme.clone(),
                ToggleButtons::new(two_texts(), vec![false, true])
                    .fill_color(WidgetStateColor::resolve_with(get_fill_color))
                    .on_pressed(|_app, _index| {}),
            ));
            tester.pump_and_settle();
            assert_eq!(
                material_color(tester, "First child"),
                Some(default_fill_color)
            );
            assert_eq!(
                material_color(tester, "Second child"),
                Some(selected_fill_color)
            );

            // disabled
            tester.pump_widget(boilerplate(
                theme,
                ToggleButtons::new(two_texts(), vec![false, true])
                    .fill_color(WidgetStateColor::resolve_with(get_fill_color)),
            ));
            tester.pump_and_settle();
            assert_eq!(
                material_color(tester, "First child"),
                Some(default_fill_color)
            );
            assert_eq!(
                material_color(tester, "Second child"),
                Some(default_fill_color)
            );
        });
    }

    /// What `paints` records for the first `_RenderInkFeatures`: its own paint and its
    /// descendants' (the button's `Material` is one of them).
    fn first_ink_features_painted(tester: &WidgetTester) -> Vec<RecordedCall> {
        let render = tester.first::<crate::material::RenderInkFeatures>();
        tester.painted_calls(render.as_object())
    }

    pub(crate) fn ink_rect_colors(tester: &WidgetTester) -> Vec<u32> {
        first_ink_features_painted(tester)
            .into_iter()
            .filter_map(|drawn| match drawn {
                RecordedCall::Rect { color, .. } => Some(color.value()),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn ink_circle_colors(tester: &WidgetTester) -> Vec<u32> {
        first_ink_features_painted(tester)
            .into_iter()
            .filter_map(|drawn| path_like(&drawn).map(|(color, _)| color.value()))
            .collect()
    }

    pub(crate) fn mouse() -> TestPointer {
        TestPointer::mouse(1, 0)
    }

    /// Dart's InkWell colors cases: hover, then press, then focus a one-button toggle that
    /// `build` makes around the focus node it is given.
    pub(crate) fn check_ink_colors(
        tester: &mut WidgetTester,
        build: impl Fn(waft_foundation::Ref<waft_widgets::FocusNode>) -> WidgetRef,
        expected: [Color; 3],
        press_first: bool,
    ) {
        let focus_node = waft_widgets::FocusNode::new(&mut tester.cell.borrow_mut());
        tester.pump_widget(build(focus_node.clone()));
        let center = text_center(tester, "First child");
        let [hover, splash, focus] = expected;
        let argb = |color: Color| color.value();

        let mut hover_gesture = mouse();
        let mut touch_gesture = TestPointer::touch(2);
        if press_first {
            // splashColor
            tester.down(&mut touch_gesture, center);
            tester.pump_and_settle();
            assert!(ink_circle_colors(tester).contains(&argb(splash)));
            tester.up(&touch_gesture);
            tester.pump_and_settle();
        }

        // hoverColor
        tester.add_pointer(&mut hover_gesture, center);
        tester.move_to(&mut hover_gesture, center, false);
        tester.pump_and_settle();
        assert!(ink_rect_colors(tester).contains(&argb(hover)));
        tester.move_to(&mut hover_gesture, Offset::ZERO, false);

        if !press_first {
            // splashColor
            tester.down(&mut touch_gesture, center); // The button is on hovered and pressed
            tester.pump_and_settle();
            assert!(ink_circle_colors(tester).contains(&argb(splash)));
            tester.up(&touch_gesture);
            tester.pump_and_settle();
            tester.move_to(&mut hover_gesture, Offset::new(0.0, 50.0), false);
            tester.pump_and_settle();
        }

        // focusColor
        tester.set_highlight_strategy(waft_widgets::FocusHighlightStrategy::AlwaysTraditional);
        focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
        tester.pump_and_settle();
        assert!(ink_rect_colors(tester).contains(&argb(focus)));
    }

    // 'Default InkWell colors - unselected'
    #[test]
    fn the_default_ink_colors_of_an_unselected_button() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            let on_surface = theme.color_scheme.on_surface();
            check_ink_colors(
                tester,
                |focus_node| {
                    boilerplate(
                        theme.clone(),
                        ToggleButtons::new(texts(&["First child"]), vec![false])
                            .on_pressed(|_app, _index| {})
                            .focus_nodes(vec![focus_node.erase()]),
                    )
                },
                [
                    on_surface.with_opacity(0.04),
                    on_surface.with_opacity(0.16),
                    on_surface.with_opacity(0.12),
                ],
                false,
            );
        });
    }

    // 'Default InkWell colors - selected'
    #[test]
    fn the_default_ink_colors_of_a_selected_button() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            let primary = theme.color_scheme.primary();
            check_ink_colors(
                tester,
                |focus_node| {
                    boilerplate(
                        theme.clone(),
                        ToggleButtons::new(texts(&["First child"]), vec![true])
                            .on_pressed(|_app, _index| {})
                            .focus_nodes(vec![focus_node.erase()]),
                    )
                },
                [
                    primary.with_opacity(0.04),
                    primary.with_opacity(0.16),
                    primary.with_opacity(0.12),
                ],
                false,
            );
        });
    }

    // 'Custom InkWell colors'
    #[test]
    fn custom_ink_colors_are_used() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            let splash_color = Color::new(0xff4caf50);
            let highlight_color = Color::new(0xffcddc39);
            let hover_color = Color::new(0xffffeb3b);
            let focus_color = Color::new(0xffffff00);
            check_ink_colors(
                tester,
                |focus_node| {
                    boilerplate(
                        theme.clone(),
                        ToggleButtons::new(texts(&["First child"]), vec![true])
                            .splash_color(splash_color)
                            .highlight_color(highlight_color)
                            .hover_color(hover_color)
                            .focus_color(focus_color)
                            .on_pressed(|_app, _index| {})
                            .focus_nodes(vec![focus_node.erase()]),
                    )
                },
                [hover_color, splash_color, focus_color],
                true,
            );
        });
    }

    /// Dart's `paints..path()..path(style: stroke, color:, strokeWidth:)...`: a path drawn
    /// first (the physical model, which Dart's recording canvas sees even at alpha 0, and
    /// which the display list drops when the button's fill has no effect, so `physical_model`
    /// is false for a button that fills at alpha 0), then each of `expected` in order, stroked.
    pub(crate) fn paints_physical_model_then_strokes(
        tester: &WidgetTester,
        render: AnyRenderObject,
        physical_model: bool,
        expected: &[(Color, f64)],
    ) -> bool {
        use waft_rendering::testing::{PaintCall, PaintedStyle, paints};
        // Dart's `path(style: stroke, color:, strokeWidth:)`.
        let stroked = |(color, width): &(Color, f64)| PaintCall::Path {
            color: Some(*color),
            style: Some(PaintedStyle::Stroke(Some(*width))),
            includes: Vec::new(),
            excludes: Vec::new(),
        };
        let mut pattern = Vec::new();
        if physical_model {
            pattern.push(PaintCall::path(None));
        }
        pattern.extend(expected.iter().map(stroked));
        paints(&tester.painted_ops(render), &pattern).is_ok()
    }

    pub(crate) fn select_toggle_button_objects(
        tester: &WidgetTester,
    ) -> Vec<RenderHandle<SelectToggleButtonRenderObject>> {
        tester.all::<SelectToggleButtonRenderObject>()
    }

    // 'Default border width and border colors for enabled, selected and disabled states'
    #[test]
    fn the_default_border_width_and_colors_follow_the_state() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            let border = (
                theme.color_scheme.on_surface().with_opacity(0.12),
                DEFAULT_BORDER_WIDTH,
            );
            for (selected, enabled) in [(false, true), (true, true), (false, false)] {
                tester.pump_widget(boilerplate(theme.clone(), single_button(selected, enabled)));
                let render = select_toggle_button_objects(tester)[0].as_object();
                assert!(paints_physical_model_then_strokes(
                    tester,
                    render,
                    selected && enabled,
                    &[border]
                ));
            }
        });
    }

    // 'Custom border width and border colors for enabled, selected and disabled states'
    #[test]
    fn custom_border_width_and_colors_follow_the_state() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            let border_color = Color::new(0xff4caf50);
            let selected_border_color = Color::new(0xffcddc39);
            let disabled_border_color = Color::new(0xffffeb3b);
            const CUSTOM_WIDTH: f64 = 2.0;

            tester.pump_widget(boilerplate(
                theme.clone(),
                single_button(false, true)
                    .border_color(border_color)
                    .border_width(CUSTOM_WIDTH),
            ));
            let render = select_toggle_button_objects(tester)[0].as_object();
            assert!(paints_physical_model_then_strokes(
                tester,
                render,
                false,
                &[(border_color, CUSTOM_WIDTH)]
            ));

            tester.pump_widget(boilerplate(
                theme.clone(),
                single_button(true, true)
                    .selected_border_color(selected_border_color)
                    .border_width(CUSTOM_WIDTH),
            ));
            let render = select_toggle_button_objects(tester)[0].as_object();
            assert!(paints_physical_model_then_strokes(
                tester,
                render,
                true,
                &[(selected_border_color, CUSTOM_WIDTH)]
            ));

            tester.pump_widget(boilerplate(
                theme,
                single_button(false, false)
                    .disabled_border_color(disabled_border_color)
                    .border_width(CUSTOM_WIDTH),
            ));
            let render = select_toggle_button_objects(tester)[0].as_object();
            assert!(paints_physical_model_then_strokes(
                tester,
                render,
                false,
                &[(disabled_border_color, CUSTOM_WIDTH)]
            ));
        });
    }

    fn sized_boxes(sizes: &[(Option<f64>, Option<f64>)]) -> Vec<WidgetRef> {
        sizes
            .iter()
            .map(|(width, height)| {
                Container::new()
                    .constraints(BoxConstraints::tight_for(*width, *height))
                    .into_widget()
            })
            .collect()
    }

    fn select_toggle_button_rects(tester: &WidgetTester) -> Vec<Rect> {
        find::by_type::<SelectToggleButton>()
            .evaluate(tester)
            .into_iter()
            .map(|element| tester.get_rect(element))
            .collect()
    }

    // 'Height of segmented control is determined by tallest widget'
    #[test]
    fn the_height_of_the_buttons_is_that_of_the_tallest_child() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            let children = sized_boxes(&[
                (None, Some(100.0)),
                (None, Some(400.0)), // tallest widget
                (None, Some(200.0)),
            ]);
            tester.pump_widget(boilerplate(
                theme,
                ToggleButtons::new(children, vec![false, true, false]),
            ));

            for rect in select_toggle_button_rects(tester) {
                assert_eq!(rect.height(), 400.0 + 2.0 * DEFAULT_BORDER_WIDTH);
            }
        });
    }

    // 'Sizes of toggle buttons rebuilds with the correct dimensions'
    #[test]
    fn the_sizes_of_the_buttons_follow_a_rebuild() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            let children = sized_boxes(&[(Some(100.0), Some(100.0)); 3]);
            tester.pump_widget(boilerplate(
                theme.clone(),
                ToggleButtons::new(children, vec![false, true, false]),
            ));

            let rects = select_toggle_button_rects(tester);
            for (i, rect) in rects.iter().enumerate() {
                assert_eq!(rect.height(), 100.0 + 2.0 * DEFAULT_BORDER_WIDTH);

                // Only the last button paints both leading and trailing borders.
                // Other buttons only paint the leading border.
                if i == rects.len() - 1 {
                    assert_eq!(rect.width(), 100.0 + 2.0 * DEFAULT_BORDER_WIDTH);
                } else {
                    assert_eq!(rect.width(), 100.0 + DEFAULT_BORDER_WIDTH);
                }
            }

            let children_rebuilt = sized_boxes(&[(Some(200.0), Some(200.0)); 3]);

            // Update border width and widget sized to verify layout updates correctly
            const CUSTOM_BORDER_WIDTH: f64 = 5.0;
            tester.pump_widget(boilerplate(
                theme,
                ToggleButtons::new(children_rebuilt, vec![false, true, false])
                    .border_width(CUSTOM_BORDER_WIDTH),
            ));

            // Only the last button paints both leading and trailing borders.
            // Other buttons only paint the leading border.
            let rects = select_toggle_button_rects(tester);
            for (i, rect) in rects.iter().enumerate() {
                assert_eq!(rect.height(), 200.0 + 2.0 * CUSTOM_BORDER_WIDTH);
                if i == rects.len() - 1 {
                    assert_eq!(rect.width(), 200.0 + 2.0 * CUSTOM_BORDER_WIDTH);
                } else {
                    assert_eq!(rect.width(), 200.0 + CUSTOM_BORDER_WIDTH);
                }
            }
        });
    }

    fn bottom_left_dy(tester: &WidgetTester, text: &str) -> f64 {
        let element = find::text(text).evaluate(tester)[0];
        tester.get_rect(element).bottom
    }

    fn baseline_row() -> Row {
        let style = |font_size: f64| {
            TextStyle::new()
                .font_family("FlutterTest")
                .font_size(font_size)
        };
        Row::new()
            .cross_axis_alignment(CrossAxisAlignment::Baseline)
            .text_baseline(TextBaseline::Alphabetic)
            .children([
                ToggleButtons::new(
                    vec![
                        Text::new("First child").style(style(8.0)).into_widget(),
                        Text::new("Second child").style(style(8.0)).into_widget(),
                    ],
                    vec![false, true],
                )
                .border_width(5.0)
                .into_widget(),
                crate::elevated_button::ElevatedButton::new(
                    None,
                    Some(Text::new("Elevated Button")),
                )
                .style(
                    crate::elevated_button::ElevatedButton::style_from()
                        .text_style(style(20.0))
                        .build(),
                )
                .into_widget(),
                Text::new("Text").style(style(28.0)).into_widget(),
            ])
    }

    // 'Material2 - ToggleButtons text baseline alignment'
    #[test]
    fn material2_toggle_buttons_align_their_text_baseline() {
        test_widgets(|tester| {
            let theme = ThemeData::builder()
                .use_material3(false)
                .build(&tester.cell.borrow());
            // The font size must be a multiple of 4 until
            // https://github.com/flutter/flutter/issues/122066 is resolved.
            tester.pump_widget(boilerplate(theme, baseline_row()));

            // The test font extends 0.25 * fontSize below the baseline.
            // So the three row elements line up like this:
            //
            //  ToggleButton  MaterialButton  Text
            //  ------------------------------------   baseline
            //  2.0           5.0             7.0      space below the baseline = 0.25 * fontSize
            //  ------------------------------------   widget text dy values
            let first_toggle_button_dy = bottom_left_dy(tester, "First child");
            let second_toggle_button_dy = bottom_left_dy(tester, "Second child");
            let elevated_button_dy = bottom_left_dy(tester, "Elevated Button");
            let text_dy = bottom_left_dy(tester, "Text");

            assert_eq!(first_toggle_button_dy, second_toggle_button_dy);
            assert_eq!(first_toggle_button_dy, elevated_button_dy - 3.0);
            assert_eq!(first_toggle_button_dy, text_dy - 5.0);
        });
    }

    // 'Material3 - ToggleButtons text baseline alignment'
    #[test]
    fn material3_toggle_buttons_align_their_text_baseline() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            // The point size of the fonts must be a multiple of 4 until
            // https://github.com/flutter/flutter/issues/122066 is resolved.
            tester.pump_widget(boilerplate(theme, baseline_row()));

            let first_toggle_button_dy = bottom_left_dy(tester, "First child");
            let second_toggle_button_dy = bottom_left_dy(tester, "Second child");
            let elevated_button_dy = bottom_left_dy(tester, "Elevated Button");
            let text_dy = bottom_left_dy(tester, "Text");

            assert_eq!(first_toggle_button_dy, second_toggle_button_dy);
            waft_widgets::testing::assert_more_or_less_equals_within(
                first_toggle_button_dy,
                elevated_button_dy - 1.7,
                0.1,
            );
            waft_widgets::testing::assert_more_or_less_equals_within(
                first_toggle_button_dy,
                text_dy - 9.7,
                0.1,
            );
        });
    }

    fn top_right_dx(tester: &WidgetTester, text: &str) -> f64 {
        let element = find::text(text).evaluate(tester)[0];
        tester.get_rect(element).right
    }

    // 'Directionality test'
    #[test]
    fn the_buttons_follow_the_text_direction() {
        test_widgets(|tester| {
            for (direction, first_is_left) in
                [(TextDirection::Ltr, true), (TextDirection::Rtl, false)]
            {
                tester.pump_widget(
                    Material::new()
                        .child(DirectionalityWidget::new(
                            direction,
                            Center::new().child(
                                ToggleButtons::new(
                                    texts(&["First child", "Second child"]),
                                    vec![false, true],
                                )
                                .on_pressed(|_app, _index| {}),
                            ),
                        ))
                        .into_widget(),
                );

                let first_is_before_second =
                    top_right_dx(tester, "First child") < top_right_dx(tester, "Second child");
                assert_eq!(first_is_before_second, first_is_left);
            }
        });
    }

    fn three_texts() -> Vec<WidgetRef> {
        texts(&["First child", "Second child", "Third child"])
    }

    // 'Properly draws borders based on state'
    #[test]
    fn borders_are_drawn_based_on_the_state_of_each_button() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            tester.pump_widget(boilerplate(
                theme.clone(),
                ToggleButtons::new(three_texts(), vec![false, true, false])
                    .on_pressed(|_app, _index| {}),
            ));
            let border = (
                theme.color_scheme.on_surface().with_opacity(0.12),
                DEFAULT_BORDER_WIDTH,
            );
            let objects = select_toggle_button_objects(tester);

            // The first button paints the leading, top and bottom sides with a path
            assert!(paints_physical_model_then_strokes(
                tester,
                objects[0].as_object(),
                false,
                &[border]
            ));
            // The middle buttons paint a leading side path first, followed by a
            // top and bottom side path
            assert!(paints_physical_model_then_strokes(
                tester,
                objects[1].as_object(),
                true,
                &[border, border]
            ));
            // The last button paints a leading side path first, followed by
            // a trailing, top and bottom side path
            assert!(paints_physical_model_then_strokes(
                tester,
                objects[2].as_object(),
                false,
                &[border, border]
            ));
        });
    }

    // 'Properly draws borders based on state when direction is vertical and verticalDirection
    // is down.'
    #[test]
    fn vertical_borders_are_drawn_based_on_the_state_of_each_button() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            tester.pump_widget(boilerplate(
                theme.clone(),
                ToggleButtons::new(three_texts(), vec![false, true, false])
                    .direction(Axis::Vertical)
                    .on_pressed(|_app, _index| {}),
            ));

            // The children should be laid out along vertical and the first child at top.
            // The item height is icon height + default border width (48.0 + 1.0) pixels.
            assert_eq!(
                text_center(tester, "First child"),
                Offset::new(400.0, 251.0)
            );
            assert_eq!(
                text_center(tester, "Second child"),
                Offset::new(400.0, 300.0)
            );
            assert_eq!(
                text_center(tester, "Third child"),
                Offset::new(400.0, 349.0)
            );

            let border = (
                theme.color_scheme.on_surface().with_opacity(0.12),
                DEFAULT_BORDER_WIDTH,
            );
            let objects = select_toggle_button_objects(tester);
            // The first button paints the left, top and right sides with a path.
            assert!(paints_physical_model_then_strokes(
                tester,
                objects[0].as_object(),
                false,
                &[border]
            ));
            // The middle buttons paint a top side path first, followed by a
            // left and right side path.
            assert!(paints_physical_model_then_strokes(
                tester,
                objects[1].as_object(),
                true,
                &[border, border]
            ));
            // The last button paints a top side path first, followed by
            // a left, bottom and right side path
            assert!(paints_physical_model_then_strokes(
                tester,
                objects[2].as_object(),
                false,
                &[border, border]
            ));
        });
    }

    // 'VerticalDirection test when direction is vertical.'
    #[test]
    fn the_vertical_direction_orders_the_buttons_of_a_vertical_toggle() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            tester.pump_widget(boilerplate(
                theme,
                ToggleButtons::new(three_texts(), vec![false, true, false])
                    .direction(Axis::Vertical)
                    .vertical_direction(VerticalDirection::Up)
                    .on_pressed(|_app, _index| {}),
            ));

            // The children should be laid out along vertical and the last child at top.
            assert_eq!(
                text_center(tester, "Third child"),
                Offset::new(400.0, 251.0)
            );
            assert_eq!(
                text_center(tester, "Second child"),
                Offset::new(400.0, 300.0)
            );
            assert_eq!(
                text_center(tester, "First child"),
                Offset::new(400.0, 349.0)
            );
        });
    }

    fn toggle_buttons_size(tester: &WidgetTester) -> Size {
        let element = find::by_type::<ToggleButtons>().evaluate(tester)[0];
        tester.get_size(element)
    }

    fn tap_target_buttons(constraints: BoxConstraints, labels: [&str; 3]) -> ToggleButtons {
        ToggleButtons::new(texts(&labels), vec![false, true, false])
            .constraints(constraints)
            .on_pressed(|_app, _index| {})
    }

    fn min_32() -> BoxConstraints {
        BoxConstraints::new().min_width(32.0).min_height(32.0)
    }

    // 'Material2 - Tap target size is configurable by ThemeData.materialTapTargetSize'
    #[test]
    fn material2_tap_target_size_is_configurable_by_the_theme() {
        test_widgets(|tester| {
            // Dart's `boilerplate` takes `theme ?? ThemeData(materialTapTargetSize:)`, so the
            // Material 2 theme given here makes the argument moot: both sizes stay padded.
            for expected in [Size::new(228.0, 48.0), Size::new(228.0, 48.0)] {
                let theme = ThemeData::builder()
                    .use_material3(false)
                    .build(&tester.cell.borrow());
                tester.pump_widget(boilerplate(
                    theme,
                    tap_target_buttons(min_32(), ["First", "Second", "Third"]),
                ));
                assert_eq!(toggle_buttons_size(tester), expected);
            }
        });
    }

    // 'Material3 - Tap target size is configurable by ThemeData.materialTapTargetSize'
    #[test]
    fn material3_tap_target_size_is_configurable_by_the_theme() {
        test_widgets(|tester| {
            for (tap_target_size, expected) in [
                (MaterialTapTargetSize::Padded, Size::new(232.0, 48.0)),
                (MaterialTapTargetSize::ShrinkWrap, Size::new(232.0, 34.0)),
            ] {
                let theme = themed(tester, tap_target_size);
                tester.pump_widget(boilerplate(
                    theme,
                    tap_target_buttons(min_32(), ["First", "Second", "Third"]),
                ));
                assert_eq!(toggle_buttons_size(tester), expected);
            }
        });
    }

    // 'Material2 - Tap target size is configurable'
    #[test]
    fn material2_tap_target_size_is_configurable() {
        test_widgets(|tester| {
            for (tap_target_size, expected) in [
                (MaterialTapTargetSize::Padded, Size::new(228.0, 48.0)),
                (MaterialTapTargetSize::ShrinkWrap, Size::new(228.0, 34.0)),
            ] {
                let theme = ThemeData::builder()
                    .use_material3(false)
                    .build(&tester.cell.borrow());
                tester.pump_widget(boilerplate(
                    theme,
                    tap_target_buttons(min_32(), ["First", "Second", "Third"])
                        .tap_target_size(tap_target_size),
                ));
                assert_eq!(toggle_buttons_size(tester), expected);
            }
        });
    }

    // 'Material3 - Tap target size is configurable'
    #[test]
    fn material3_tap_target_size_is_configurable() {
        test_widgets(|tester| {
            for (tap_target_size, expected) in [
                (MaterialTapTargetSize::Padded, Size::new(232.0, 48.0)),
                (MaterialTapTargetSize::ShrinkWrap, Size::new(232.0, 34.0)),
            ] {
                let theme = default_theme(tester);
                tester.pump_widget(boilerplate(
                    theme,
                    tap_target_buttons(min_32(), ["First", "Second", "Third"])
                        .tap_target_size(tap_target_size),
                ));
                assert_eq!(toggle_buttons_size(tester), expected);
            }
        });
    }

    // 'Tap target size is configurable for vertical axis'
    #[test]
    fn tap_target_size_is_configurable_for_the_vertical_axis() {
        test_widgets(|tester| {
            for (tap_target_size, expected) in [
                (MaterialTapTargetSize::Padded, Size::new(48.0, 100.0)),
                (MaterialTapTargetSize::ShrinkWrap, Size::new(34.0, 100.0)),
            ] {
                let theme = default_theme(tester);
                tester.pump_widget(boilerplate(
                    theme,
                    tap_target_buttons(min_32(), ["1", "2", "3"])
                        .tap_target_size(tap_target_size)
                        .direction(Axis::Vertical),
                ));
                assert_eq!(toggle_buttons_size(tester), expected);
            }
        });
    }

    // 'Material2 - Border radius paint test when there is only one button' and the Material 3
    // twin; their golden files are not compared.
    #[test]
    fn a_lone_button_with_a_border_radius_is_centered_and_strokes_its_border() {
        test_widgets(|tester| {
            for use_material3 in [false, true] {
                let theme = ThemeData::builder()
                    .use_material3(use_material3)
                    .build(&tester.cell.borrow());
                tester.pump_widget(boilerplate(
                    theme.clone(),
                    waft_widgets::RepaintBoundary::new().child(
                        single_button(true, true).border_radius(BorderRadius::circular(7.0)),
                    ),
                ));

                // The only button should be laid out at the center of the screen.
                assert_eq!(
                    text_center(tester, "First child"),
                    Offset::new(400.0, 300.0)
                );

                let border = (
                    theme.color_scheme.on_surface().with_opacity(0.12),
                    DEFAULT_BORDER_WIDTH,
                );
                let render = select_toggle_button_objects(tester)[0].as_object();
                assert!(paints_physical_model_then_strokes(
                    tester,
                    render,
                    true,
                    &[border]
                ));
            }
        });
    }

    // 'ToggleButtons changes mouse cursor when the button is hovered'
    #[test]
    fn the_mouse_cursor_follows_the_toggle_buttons_configuration() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            let forbidden_region = |buttons: ToggleButtons| {
                waft_widgets::MouseRegion::new()
                    .cursor(SystemMouseCursors::FORBIDDEN.into())
                    .child(buttons)
            };
            tester.pump_widget(boilerplate(
                theme.clone(),
                forbidden_region(
                    ToggleButtons::new(two_texts(), vec![false, true])
                        .mouse_cursor(SystemMouseCursors::TEXT)
                        .on_pressed(|_app, _index| {}),
                ),
            ));

            let mut gesture = TestPointer::mouse(1, 0);
            let center = text_center(tester, "First child");
            tester.add_pointer(&mut gesture, center);
            tester.pump();
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::TEXT.into());

            // Test default cursor
            tester.pump_widget(boilerplate(
                theme.clone(),
                forbidden_region(
                    ToggleButtons::new(two_texts(), vec![false, true])
                        .on_pressed(|_app, _index| {}),
                ),
            ));
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::BASIC.into());

            // Test default cursor when disabled
            tester.pump_widget(boilerplate(
                theme,
                forbidden_region(ToggleButtons::new(two_texts(), vec![false, true])),
            ));
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::BASIC.into());
        });
    }

    // 'ToggleButtons focus, hover, and highlight elevations are 0'
    #[test]
    fn focus_hover_and_highlight_elevations_are_zero() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            let focus_nodes: Vec<_> = (0..2)
                .map(|_| waft_widgets::FocusNode::new(&mut tester.cell.borrow_mut()))
                .collect();
            tester.pump_widget(boilerplate(
                theme,
                ToggleButtons::new(texts(&["one", "two"]), vec![true, false])
                    .on_pressed(|_app, _index| {})
                    .focus_nodes(focus_nodes.iter().map(|node| node.erase()).collect()),
            ));

            // `find.widgetWithText(Material, text).first`: the outermost Material above the text.
            let toggle_button_elevation = |tester: &WidgetTester, text: &str| {
                let text_element = find::text(text).evaluate(tester)[0];
                let material = find::by_type::<Material>()
                    .evaluate(tester)
                    .into_iter()
                    .find(|material| tester.ancestors(text_element).contains(material))
                    .expect("a Material above the text");
                tester.widget(material, |material: &Material| material.elevation)
            };

            // Default toggle button elevation
            assert_eq!(toggle_button_elevation(tester, "one"), 0.0); // highlighted
            assert_eq!(toggle_button_elevation(tester, "two"), 0.0); // not highlighted

            // Hovered button elevation
            let mut hover_gesture = TestPointer::mouse(1, 0);
            let one = text_center(tester, "one");
            tester.add_pointer(&mut hover_gesture, one);
            tester.move_to(&mut hover_gesture, one, false);
            tester.pump_and_settle();
            assert_eq!(toggle_button_elevation(tester, "one"), 0.0);
            let two = text_center(tester, "two");
            tester.move_to(&mut hover_gesture, two, false);
            tester.pump_and_settle();
            assert_eq!(toggle_button_elevation(tester, "two"), 0.0);

            // Focused button elevation
            let has_focus =
                |tester: &WidgetTester, node: &waft_foundation::Ref<waft_widgets::FocusNode>| {
                    node.has_focus(&mut tester.cell.borrow_mut())
                };
            focus_nodes[0].request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert!(has_focus(tester, &focus_nodes[0]));
            assert!(!has_focus(tester, &focus_nodes[1]));
            assert_eq!(toggle_button_elevation(tester, "one"), 0.0);
            focus_nodes[1].request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            assert!(!has_focus(tester, &focus_nodes[0]));
            assert!(has_focus(tester, &focus_nodes[1]));
            assert_eq!(toggle_button_elevation(tester, "two"), 0.0);
        });
    }

    // 'Toggle buttons height matches MaterialTapTargetSize.padded height'
    #[test]
    fn the_buttons_height_matches_the_padded_tap_target_height() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            tester.pump_widget(boilerplate(
                theme,
                ToggleButtons::new(three_icons(), vec![false; 3]).on_pressed(|_app, _index| {}),
            ));

            for index in 0..3 {
                assert_eq!(button_rect(tester, index).height(), 48.0);
            }
        });
    }

    // 'Toggle buttons constraints size does not affect minimum input padding', without its
    // semantics-tree expectation (its flags, actions and rects are not ported).
    #[test]
    fn constraints_size_sets_the_height_of_the_buttons() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            tester.pump_widget(boilerplate(
                theme,
                ToggleButtons::new(three_icons(), vec![false; 3])
                    .on_pressed(|_app, _index| {})
                    .constraints(BoxConstraints::tight_for(Some(86.0), Some(32.0))),
            ));

            // Button's height is constrained to `32.0`.
            for index in 0..3 {
                assert_eq!(button_rect(tester, index).height(), 32.0);
            }
        });
    }

    // 'ToggleButtons does not crash at zero area'
    #[test]
    fn toggle_buttons_do_not_crash_at_zero_area() {
        test_widgets(|tester| {
            let theme = default_theme(tester);
            tester.pump_widget(boilerplate(
                theme,
                Center::new()
                    .child(SizedBox::shrink().child(ToggleButtons::new(texts(&["X"]), vec![true]))),
            ));

            assert_eq!(toggle_buttons_size(tester), Size::ZERO);
        });
    }
}
