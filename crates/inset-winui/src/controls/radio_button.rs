//! XAML `RadioButton` with the `DefaultRadioButtonStyle` of `RadioButton_themeresources.xaml`.
//!
//! Template: `RootGrid` (the control's `Background`, `BorderBrush`, `BorderThickness`, `CornerRadius`) with columns 20 and `*`; in column 0 a 32-high grid aligned `Top` holding `OuterEllipse` (the unchecked ring), `CheckOuterEllipse` (the checked ring, shown in the `Checked` state), `CheckGlyph` (the dot, shown when checked; its `Width` / `Height` animate per common state) and `PressedCheckGlyph` (a 4 pt dot that grows to 10 while pressed); in column 1 the `ContentPresenter` with `Margin="{TemplateBinding Padding}"` (8,6,0,0), aligned `Left` / `Top`.
//!
//! Behaviour (`RadioButton_Partial.cpp`, `ToggleButton_Partial.cpp`): `ToggleButton::OnClick` calls `OnToggle`, which `RadioButton::OnToggle` overrides to set `IsChecked` to `true` only, so a checked radio button does not uncheck on click; `RadioButton::UpdateRadioButtonGroup` unchecks the other members of the group when one becomes checked.

use crate::{
    Brush, ColumnDefinition, CommonState, CommonStates, ControlStates, FocusVisual, Grid, GridCell,
    GridLength, ThemeResources, control_text_style, fast_out_slow_in,
};
use inset_embedder::{Canvas, Color, FontWeight, Offset, Size};
use inset_foundation::{App, Listener};
use inset_painting::{AlignmentGeometry, EdgeInsetsGeometry};
use inset_rendering::{BoxConstraints, CustomPainter};
use inset_widgets::*;
use std::{fmt, time::Duration};

/// `OuterEllipse` and `CheckOuterEllipse`: `Width="20" Height="20"`.
pub const RING_SIZE: f64 = 20.0;
/// The grid the ellipses centre in: `Height="32"`.
pub const GLYPH_AREA_HEIGHT: f64 = 32.0;
/// The first column of `RootGrid`: `Width="20"`.
pub const GLYPH_COLUMN_WIDTH: f64 = 20.0;
/// `PressedCheckGlyph`: `Width="4" Height="4"` until the `Pressed` storyboard grows it.
pub const PRESSED_GLYPH_SIZE: f64 = 4.0;
/// The size the `Pressed` storyboard animates `PressedCheckGlyph` to.
pub const PRESSED_GLYPH_PRESSED_SIZE: f64 = 10.0;
/// The size the `Disabled` storyboard animates `CheckGlyph` to.
pub const CHECK_GLYPH_DISABLED_SIZE: f64 = 14.0;
/// The default `StrokeThickness` of a XAML `Shape`, which `CheckGlyph` keeps.
pub const CHECK_GLYPH_STROKE_THICKNESS: f64 = 1.0;
/// The style's `Padding`, the `ContentPresenter`'s margin: left, top, right, bottom.
pub const RADIO_BUTTON_PADDING: [f64; 4] = [8.0, 6.0, 0.0, 0.0];
/// The style's `MinWidth`.
pub const RADIO_BUTTON_MIN_WIDTH: f64 = 120.0;
/// The style's `FocusVisualMargin`.
pub const RADIO_BUTTON_FOCUS_VISUAL_MARGIN: [f64; 4] = [-7.0, -3.0, -7.0, -3.0];

/// The size `CheckGlyph` has in each common state: `RadioButtonCheckGlyphSize` (12) at rest, the `PointerOver` storyboard's 14, the `Pressed` storyboard's 10 and the `Disabled` storyboard's 14.
pub fn check_glyph_size(r: &ThemeResources, state: CommonState) -> f64 {
    match state {
        CommonState::Normal => r.radio_button_check_glyph_size,
        CommonState::PointerOver => r.radio_button_check_glyph_pointer_over_size,
        CommonState::Pressed => r.radio_button_check_glyph_pressed_over_size,
        CommonState::Disabled => CHECK_GLYPH_DISABLED_SIZE,
    }
}

/// How long `CheckGlyph` takes to reach its size for `state`: the `KeyTime` of the state's `SplineDoubleKeyFrame` (`ControlNormalAnimationDuration` for `PointerOver` and `Pressed`, `ControlFastAnimationDuration` for `Disabled`). `Normal` declares no size animation, so the glyph returns to its base size at once, as a stopped storyboard releases the property.
pub fn check_glyph_duration(r: &ThemeResources, state: CommonState) -> Duration {
    match state {
        CommonState::Normal => Duration::ZERO,
        CommonState::PointerOver | CommonState::Pressed => r.control_normal_animation_duration,
        CommonState::Disabled => r.control_fast_animation_duration,
    }
}

/// XAML `RadioButton`: `IsChecked` with `Checked`, `Content`, `IsEnabled`. The group is the caller's: the `checked` handler fires when an unchecked button is activated, and the caller checks it and unchecks the others.
#[derive(Clone)]
pub struct RadioButton {
    pub is_checked: bool,
    /// XAML `Checked`: raised when activation checks the button.
    pub checked: Listener,
    pub content: Option<WidgetRef>,
    pub is_enabled: bool,
    pub key: Option<KeyRef>,
}

impl RadioButton {
    pub fn new(is_checked: bool, checked: impl Fn(&mut App) + 'static) -> RadioButton {
        RadioButton {
            is_checked,
            checked: Listener::new(checked),
            content: None,
            is_enabled: true,
            key: None,
        }
    }

    /// XAML `Content`.
    pub fn content<K>(mut self, content: impl IntoWidget<K>) -> RadioButton {
        self.content = Some(content.into_widget());
        self
    }

    /// XAML `IsEnabled`.
    pub fn is_enabled(mut self, enabled: bool) -> RadioButton {
        self.is_enabled = enabled;
        self
    }

    pub fn key(mut self, key: KeyRef) -> RadioButton {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for RadioButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RadioButton")
            .field("is_checked", &self.is_checked)
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for RadioButton {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.clone();
        // `RadioButton::OnToggle`: `put_IsChecked(TRUE)`; a value that does not change raises nothing.
        let checked = self.checked.clone();
        let is_checked = self.is_checked;
        let click = Listener::new(move |app| {
            if !is_checked {
                checked.call(app);
            }
        });
        // `RadioButton::Initialize`: `SetAcceptsReturn(false)`.
        CommonStates::new(click, move |app, context, states| {
            template(app, context, &widget, states)
        })
        .is_enabled(self.is_enabled)
        .accepts_return(false)
        .into_widget()
    }
}

/// The `{ThemeResource …}` values the `CommonStates` storyboards name for one state.
struct RadioBrushes {
    foreground: Color,
    background: Color,
    border: Color,
    outer_ellipse_stroke: Color,
    outer_ellipse_fill: Color,
    check_outer_ellipse_stroke: Color,
    check_outer_ellipse_fill: Color,
    check_glyph_fill: Color,
    /// `RadioButtonCheckGlyphStroke*`: `CircleElevationBorderBrush`.
    check_glyph_stroke: [(f64, Color); 2],
}

fn brushes(r: &ThemeResources, state: CommonState) -> RadioBrushes {
    match state {
        CommonState::Normal => RadioBrushes {
            foreground: r.radio_button_foreground,
            background: r.radio_button_background,
            border: r.radio_button_border_brush,
            outer_ellipse_stroke: r.radio_button_outer_ellipse_stroke,
            outer_ellipse_fill: r.radio_button_outer_ellipse_fill,
            check_outer_ellipse_stroke: r.radio_button_outer_ellipse_checked_stroke,
            check_outer_ellipse_fill: r.radio_button_outer_ellipse_checked_fill,
            check_glyph_fill: r.radio_button_check_glyph_fill,
            check_glyph_stroke: r.radio_button_check_glyph_stroke,
        },
        CommonState::PointerOver => RadioBrushes {
            foreground: r.radio_button_foreground_pointer_over,
            background: r.radio_button_background_pointer_over,
            border: r.radio_button_border_brush_pointer_over,
            outer_ellipse_stroke: r.radio_button_outer_ellipse_stroke_pointer_over,
            outer_ellipse_fill: r.radio_button_outer_ellipse_fill_pointer_over,
            check_outer_ellipse_stroke: r.radio_button_outer_ellipse_checked_stroke_pointer_over,
            check_outer_ellipse_fill: r.radio_button_outer_ellipse_checked_fill_pointer_over,
            check_glyph_fill: r.radio_button_check_glyph_fill_pointer_over,
            check_glyph_stroke: r.radio_button_check_glyph_stroke_pointer_over,
        },
        CommonState::Pressed => RadioBrushes {
            foreground: r.radio_button_foreground_pressed,
            background: r.radio_button_background_pressed,
            border: r.radio_button_border_brush_pressed,
            outer_ellipse_stroke: r.radio_button_outer_ellipse_stroke_pressed,
            outer_ellipse_fill: r.radio_button_outer_ellipse_fill_pressed,
            check_outer_ellipse_stroke: r.radio_button_outer_ellipse_checked_stroke_pressed,
            check_outer_ellipse_fill: r.radio_button_outer_ellipse_checked_fill_pressed,
            check_glyph_fill: r.radio_button_check_glyph_fill_pressed,
            check_glyph_stroke: r.radio_button_check_glyph_stroke_pressed,
        },
        CommonState::Disabled => RadioBrushes {
            foreground: r.radio_button_foreground_disabled,
            background: r.radio_button_background_disabled,
            border: r.radio_button_border_brush_disabled,
            outer_ellipse_stroke: r.radio_button_outer_ellipse_stroke_disabled,
            outer_ellipse_fill: r.radio_button_outer_ellipse_fill_disabled,
            check_outer_ellipse_stroke: r.radio_button_outer_ellipse_checked_stroke_disabled,
            check_outer_ellipse_fill: r.radio_button_outer_ellipse_checked_fill_disabled,
            check_glyph_fill: r.radio_button_check_glyph_fill_disabled,
            check_glyph_stroke: r.radio_button_check_glyph_stroke_disabled,
        },
    }
}

/// The `ControlTemplate` of `DefaultRadioButtonStyle` for one set of states.
fn template(
    app: &mut App,
    context: BuildContext,
    widget: &RadioButton,
    states: ControlStates,
) -> WidgetRef {
    let resources = ThemeResources::of(app, context);
    let r = &resources;
    let brushes = brushes(r, states.common);
    let glyphs = glyph_area(r, &brushes, widget.is_checked, states.common);
    // `ContentPresenter`: `Margin="{TemplateBinding Padding}"`, `HorizontalContentAlignment="Left"`, `VerticalContentAlignment="Top"`, `TextWrapping="Wrap"`.
    let text = control_text_style(
        &resources,
        r.control_content_theme_font_size,
        FontWeight::W400,
        brushes.foreground,
    );
    let [left, top, right, bottom] = RADIO_BUTTON_PADDING;
    let mut presenter = Align::new()
        .alignment(AlignmentGeometry::TOP_LEFT)
        .width_factor(1.0)
        .height_factor(1.0);
    if let Some(content) = &widget.content {
        presenter = presenter.child(
            Padding::new(EdgeInsetsGeometry::from_ltrb(left, top, right, bottom))
                .child(DefaultTextStyle::new(text, content.clone())),
        );
    }
    // `RootGrid`: the control's `BorderThickness` is the `Control` default, 0.
    let root = Grid::new()
        .background(Brush::Solid(brushes.background))
        .border_brush(Brush::Solid(brushes.border))
        .border_thickness(0.0)
        .corner_radius(r.control_corner_radius[0])
        .column_definitions([
            ColumnDefinition::new(GridLength::pixel(GLYPH_COLUMN_WIDTH)),
            ColumnDefinition::new(GridLength::STAR),
        ])
        .children([
            GridCell::new(glyphs).into_widget(),
            GridCell::new(presenter).column(1).into_widget(),
        ]);
    let body =
        ConstrainedBox::new(BoxConstraints::new().min_width(RADIO_BUTTON_MIN_WIDTH)).child(root);
    FocusVisual::new(body, resources.theme)
        .visible(states.focused)
        .margin(RADIO_BUTTON_FOCUS_VISUAL_MARGIN)
        .corner_radius(r.control_corner_radius[0])
        .into_widget()
}

/// The 32-high grid aligned `Top` in column 0, with the two rings and the two dots centred in it.
fn glyph_area(
    r: &ThemeResources,
    brushes: &RadioBrushes,
    is_checked: bool,
    state: CommonState,
) -> WidgetRef {
    // `OuterEllipse`: `Opacity` 0 in the `Checked` state; `CheckOuterEllipse`: `Opacity="0"`, 1 in `Checked`.
    let ring = if is_checked {
        Ellipse::new(
            Brush::Solid(brushes.check_outer_ellipse_fill),
            Brush::Solid(brushes.check_outer_ellipse_stroke),
        )
    } else {
        Ellipse::new(
            Brush::Solid(brushes.outer_ellipse_fill),
            Brush::Solid(brushes.outer_ellipse_stroke),
        )
    };
    let ring = SizedBox::new()
        .width(RING_SIZE)
        .height(RING_SIZE)
        .child(ring.stroke_thickness(r.radio_button_border_theme_thickness))
        .into_widget();
    let mut layers = vec![Center::new().child(ring).into_widget()];
    // `CheckGlyph`: `Opacity="0"`, 1 in `Checked`. Its `Stroke` is the `CommonStates` storyboards' `RadioButtonCheckGlyphStroke*` (`CircleElevationBorderBrush`) until the `Checked` storyboard, applied after them, sets `RadioButtonCheckGlyphStrokeChecked` (`AccentControlElevationBorderBrush`).
    let stroke = if is_checked {
        Brush::ControlElevation(r.radio_button_check_glyph_stroke_checked)
    } else {
        Brush::CircleElevation(brushes.check_glyph_stroke)
    };
    let glyph = Ellipse::new(Brush::Solid(brushes.check_glyph_fill), stroke)
        .stroke_thickness(CHECK_GLYPH_STROKE_THICKNESS);
    let size = check_glyph_size(r, state);
    let glyph = AnimatedContainer::new(check_glyph_duration(r, state))
        .curve(fast_out_slow_in(r))
        .width(size)
        .height(size)
        .child(glyph);
    layers.push(
        Center::new()
            .child(Opacity::new(if is_checked { 1.0 } else { 0.0 }).child(glyph))
            .into_widget(),
    );
    // `PressedCheckGlyph`: `Opacity="0"`, 1 in `Pressed`, where it grows from 4 to 10 over `ControlFastAnimationDuration`; a `Border` with `CornerRadius="6"` and no `BorderThickness`, so a filled circle.
    let pressed = state == CommonState::Pressed;
    let pressed_glyph = Ellipse::new(
        Brush::Solid(brushes.check_glyph_fill),
        Brush::Solid(Color::from_argb(0, 0, 0, 0)),
    )
    .stroke_thickness(0.0);
    let size = if pressed {
        PRESSED_GLYPH_PRESSED_SIZE
    } else {
        PRESSED_GLYPH_SIZE
    };
    let duration = if pressed {
        r.control_fast_animation_duration
    } else {
        Duration::ZERO
    };
    let pressed_glyph = AnimatedContainer::new(duration)
        .curve(fast_out_slow_in(r))
        .width(size)
        .height(size)
        .child(pressed_glyph);
    layers.push(
        Center::new()
            .child(Opacity::new(if pressed { 1.0 } else { 0.0 }).child(pressed_glyph))
            .into_widget(),
    );
    let area = SizedBox::new()
        .height(GLYPH_AREA_HEIGHT)
        .child(Stack::new().children(layers))
        .into_widget();
    Align::new()
        .alignment(AlignmentGeometry::TOP_CENTER)
        .width_factor(1.0)
        .height_factor(1.0)
        .child(area)
        .into_widget()
}

/// XAML `Ellipse`: `Fill` and `Stroke` at `StrokeThickness`, the stroke centred on the geometry inset by half the thickness so it stays inside the bounds, as a `Shape` lays out.
#[derive(Clone, Debug, PartialEq)]
pub struct Ellipse {
    pub fill: Brush,
    pub stroke: Brush,
    pub stroke_thickness: f64,
}

impl Ellipse {
    pub fn new(fill: Brush, stroke: Brush) -> Ellipse {
        Ellipse {
            fill,
            stroke,
            stroke_thickness: 1.0,
        }
    }

    /// XAML `StrokeThickness`.
    pub fn stroke_thickness(mut self, thickness: f64) -> Ellipse {
        self.stroke_thickness = thickness;
        self
    }
}

impl StatelessWidget for Ellipse {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        CustomPaint::new().painter(self.clone()).into_widget()
    }
}

impl CustomPainter for Ellipse {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        let bounds = Offset::ZERO & size;
        let geometry = bounds.deflate(self.stroke_thickness / 2.0);
        if matches!(self.fill, Brush::Acrylic(_)) || self.fill.representative_color().a > 0.0 {
            self.fill.paint_oval(canvas, geometry, bounds);
        }
        if self.stroke_thickness > 0.0
            && (matches!(self.stroke, Brush::Acrylic(_))
                || self.stroke.representative_color().a > 0.0)
        {
            self.stroke
                .paint_oval_stroke(canvas, geometry, bounds, self.stroke_thickness);
        }
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        old_delegate.as_any().downcast_ref::<Self>() != Some(self)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
