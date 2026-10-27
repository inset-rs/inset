//! XAML `CheckBox` with the `DefaultCheckBoxStyle` of `CheckBox_themeresources.xaml`.
//!
//! Template, top to bottom: `RootGrid` (`Background`, `BorderBrush`, `BorderThickness`, `CornerRadius`; columns `Auto` and `*`); in column 0 a 20 × 20 grid centred vertically holding `NormalRectangle` (the box: `CheckBoxCheckBackgroundFill*` / `Stroke*`, `CheckBoxBorderThickness`, radius `CornerRadius`) and `CheckGlyph` (an `AnimatedIcon` playing `AnimatedAcceptVisualSource`, `Margin="0,1,0,-1"`, `Margin="0"` in the indeterminate states; its `FallbackIconSource` is the `DownLevelCheckGlyph` FontIcon); in column 1 the `ContentPresenter` with `Margin="{TemplateBinding Padding}"`, `HorizontalContentAlignment="Left"`, `VerticalContentAlignment="Top"`. The `CombinedStates` group (`UncheckedNormal` … `IndeterminateDisabled`) swaps the six brushes discretely; there is no `BrushTransition`.
//!
//! The glyph is not a font glyph on a Windows 11 host: `AnimatedIcon` renders the composition program in `controls/dev/AnimatedIcon/AnimatedVisuals/AnimatedAcceptVisualSource.cpp` (LottieGen output), which this file transcribes: a 48 × 48 canvas, the check as a polyline stroked 4 wide with round caps and joins under the layer transforms, drawn on by `TrimEnd` and erased by `TrimStart`, and the indeterminate bar as a line. `AnimatedIcon::ArrangeOverride` scales the canvas uniformly into the icon's bounds and centres it.

use crate::{
    BackgroundSizing, Brush, ColumnDefinition, CommonState, CommonStates, ControlBorder,
    ControlStates, FocusVisual, Grid, GridCell, GridLength, ThemeResources, control_text_style,
};
use inset_animation::{
    Animation, AnimationBehavior, AnimationController, AnimationStatusListener, Cubic, Curve,
};
use inset_embedder::valo::{Cap, FillRule, Join, Point};
use inset_embedder::{
    Canvas, Color, FontWeight, Offset, Paint, PaintStyle, PathBuilder, Size, Stroke,
};
use inset_foundation::{App, Handle, Listener};
use inset_painting::{AlignmentGeometry, EdgeInsetsGeometry};
use inset_rendering::{BoxConstraints, CustomPainter};
use inset_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use inset_services::LogicalKeyboardKey;
use inset_widgets::*;
use std::{any::TypeId, collections::HashMap, fmt, rc::Rc, time::Duration};

/// `Control.BorderThickness`: the style sets none, so `RootGrid` draws no border.
pub const CHECK_BOX_ROOT_BORDER_THICKNESS: f64 = 0.0;
/// `CheckGlyph`'s `Margin="0,1,0,-1"`: the glyph sits one pixel below the box's centre in the checked and unchecked states.
pub const CHECK_GLYPH_OFFSET: Offset = Offset::new(0.0, 1.0);

/// `AnimatedAcceptVisualSource::Size()`.
pub const ACCEPT_VISUAL_SIZE: f64 = 48.0;
/// `AnimatedAcceptVisualSource::Duration()`: `c_durationTicks` of 100 ns, 160 frames at 60 fps.
pub const ACCEPT_VISUAL_DURATION: Duration = Duration::from_nanos(26_666_666 * 100);
/// The `NormalOffToNormalOn` markers; `PointerOverOffToPointerOverOn` and `PressedOffToNormalOn` span the same length, with the same `TrimEnd` keyframes.
pub const NORMAL_OFF_TO_NORMAL_ON: (f64, f64) = (0.0940625, 0.2128125);
/// The `NormalOnToNormalOff` markers; `PointerOverOnToPointerOverOff` and `PressedOnToNormalOff` span the same length, with the same `TrimStart` keyframes.
pub const NORMAL_ON_TO_NORMAL_OFF: (f64, f64) = (0.0, 0.0253125);
/// `CubicBezierEasingFunction_0`, easing every `TrimEnd` keyframe.
pub const TRIM_END_EASING: [f64; 4] = [0.55, 0.0, 0.0, 1.0];
/// `CubicBezierEasingFunction_1`, easing every `TrimStart` keyframe.
pub const TRIM_START_EASING: [f64; 4] = [0.167, 0.167, 0.833, 0.833];
/// `CompositionSpriteShape::StrokeThickness` of every shape in the program.
pub const ACCEPT_STROKE_THICKNESS: f32 = 4.0;
/// `Geometry_1`: the check, in shape space.
pub const CHECK_POLYLINE: [(f32, f32); 3] = [(-15.172, 0.016), (-5.0, 10.188), (15.337, -10.337)];
/// `Geometry_0`: the indeterminate bar, in shape space.
pub const INDETERMINATE_POLYLINE: [(f32, f32); 2] = [(-11.75, -0.125), (11.875, -0.125)];
/// The sprite shapes' `TransformMatrix`: scale 0.7 then offset (24, 23) for the check layers and (24, 24) for the indeterminate layer.
pub const SPRITE_SCALE: f32 = 0.7;
pub const CHECK_SPRITE_OFFSET: (f32, f32) = (24.0, 23.0);
pub const INDETERMINATE_SPRITE_OFFSET: (f32, f32) = (24.0, 24.0);
/// `Null 230`: every layer's container scales 1.05 about the canvas centre (offset (−24, −24), scale 1.05, offset (24, 24)).
pub const LAYER_SCALE: f32 = 1.05;

/// How long `AnimatedIcon::PlaySegment` plays a segment: the visual's duration times the marker span.
pub fn segment_duration(markers: (f64, f64)) -> Duration {
    ACCEPT_VISUAL_DURATION.mul_f64(markers.1 - markers.0)
}

/// XAML `ToggleButton.IsChecked`, whose `IReference<bool>` is null for indeterminate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CheckState {
    #[default]
    Unchecked,
    Checked,
    Indeterminate,
}

impl CheckState {
    pub fn from_is_checked(is_checked: Option<bool>) -> CheckState {
        match is_checked {
            Some(true) => CheckState::Checked,
            Some(false) => CheckState::Unchecked,
            None => CheckState::Indeterminate,
        }
    }
}

/// `ToggleButton::OnToggleImpl`: indeterminate → unchecked; checked → indeterminate when three-state, else unchecked; unchecked → checked.
pub fn toggle_is_checked(is_checked: Option<bool>, is_three_state: bool) -> Option<bool> {
    match is_checked {
        None => Some(false),
        Some(true) if is_three_state => None,
        Some(true) => Some(false),
        Some(false) => Some(true),
    }
}

/// The brushes one `CombinedStates` storyboard names.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckBoxBrushes {
    /// `ContentPresenter.Foreground`.
    pub foreground: Color,
    /// `RootGrid.Background`.
    pub background: Color,
    /// `RootGrid.BorderBrush`.
    pub border: Color,
    /// `NormalRectangle.Stroke`.
    pub box_stroke: Color,
    /// `NormalRectangle.Fill`.
    pub box_fill: Color,
    /// `CheckGlyph.Foreground`.
    pub glyph: Color,
}

/// The `{ThemeResource …}` values the `CombinedStates` storyboard for (`check`, `common`) names.
pub fn check_box_brushes(
    r: &ThemeResources,
    check: CheckState,
    common: CommonState,
) -> CheckBoxBrushes {
    match (check, common) {
        // `UncheckedNormal`: the template's own values.
        (CheckState::Unchecked, CommonState::Normal) => CheckBoxBrushes {
            foreground: r.check_box_foreground_unchecked,
            background: r.check_box_background_unchecked,
            border: r.check_box_border_brush_unchecked,
            box_stroke: r.check_box_check_background_stroke_unchecked,
            box_fill: r.check_box_check_background_fill_unchecked,
            glyph: r.check_box_check_glyph_foreground_unchecked,
        },
        (CheckState::Unchecked, CommonState::PointerOver) => CheckBoxBrushes {
            foreground: r.check_box_foreground_unchecked_pointer_over,
            background: r.check_box_background_unchecked_pointer_over,
            border: r.check_box_border_brush_unchecked_pointer_over,
            box_stroke: r.check_box_check_background_stroke_unchecked_pointer_over,
            box_fill: r.check_box_check_background_fill_unchecked_pointer_over,
            glyph: r.check_box_check_glyph_foreground_unchecked_pointer_over,
        },
        (CheckState::Unchecked, CommonState::Pressed) => CheckBoxBrushes {
            foreground: r.check_box_foreground_unchecked_pressed,
            background: r.check_box_background_unchecked_pressed,
            border: r.check_box_border_brush_unchecked_pressed,
            box_stroke: r.check_box_check_background_stroke_unchecked_pressed,
            box_fill: r.check_box_check_background_fill_unchecked_pressed,
            glyph: r.check_box_check_glyph_foreground_unchecked_pressed,
        },
        (CheckState::Unchecked, CommonState::Disabled) => CheckBoxBrushes {
            foreground: r.check_box_foreground_unchecked_disabled,
            background: r.check_box_background_unchecked_disabled,
            border: r.check_box_border_brush_unchecked_disabled,
            box_stroke: r.check_box_check_background_stroke_unchecked_disabled,
            box_fill: r.check_box_check_background_fill_unchecked_disabled,
            glyph: r.check_box_check_glyph_foreground_unchecked_disabled,
        },
        (CheckState::Checked, CommonState::Normal) => CheckBoxBrushes {
            foreground: r.check_box_foreground_checked,
            background: r.check_box_background_checked,
            border: r.check_box_border_brush_checked,
            box_stroke: r.check_box_check_background_stroke_checked,
            box_fill: r.check_box_check_background_fill_checked,
            glyph: r.check_box_check_glyph_foreground_checked,
        },
        (CheckState::Checked, CommonState::PointerOver) => CheckBoxBrushes {
            foreground: r.check_box_foreground_checked_pointer_over,
            background: r.check_box_background_checked_pointer_over,
            border: r.check_box_border_brush_checked_pointer_over,
            box_stroke: r.check_box_check_background_stroke_checked_pointer_over,
            box_fill: r.check_box_check_background_fill_checked_pointer_over,
            glyph: r.check_box_check_glyph_foreground_checked_pointer_over,
        },
        (CheckState::Checked, CommonState::Pressed) => CheckBoxBrushes {
            foreground: r.check_box_foreground_checked_pressed,
            background: r.check_box_background_checked_pressed,
            border: r.check_box_border_brush_checked_pressed,
            box_stroke: r.check_box_check_background_stroke_checked_pressed,
            box_fill: r.check_box_check_background_fill_checked_pressed,
            glyph: r.check_box_check_glyph_foreground_checked_pressed,
        },
        (CheckState::Checked, CommonState::Disabled) => CheckBoxBrushes {
            foreground: r.check_box_foreground_checked_disabled,
            background: r.check_box_background_checked_disabled,
            border: r.check_box_border_brush_checked_disabled,
            box_stroke: r.check_box_check_background_stroke_checked_disabled,
            box_fill: r.check_box_check_background_fill_checked_disabled,
            glyph: r.check_box_check_glyph_foreground_checked_disabled,
        },
        (CheckState::Indeterminate, CommonState::Normal) => CheckBoxBrushes {
            foreground: r.check_box_foreground_indeterminate,
            background: r.check_box_background_indeterminate,
            border: r.check_box_border_brush_indeterminate,
            box_stroke: r.check_box_check_background_stroke_indeterminate,
            box_fill: r.check_box_check_background_fill_indeterminate,
            glyph: r.check_box_check_glyph_foreground_indeterminate,
        },
        (CheckState::Indeterminate, CommonState::PointerOver) => CheckBoxBrushes {
            foreground: r.check_box_foreground_indeterminate_pointer_over,
            background: r.check_box_background_indeterminate_pointer_over,
            border: r.check_box_border_brush_indeterminate_pointer_over,
            box_stroke: r.check_box_check_background_stroke_indeterminate_pointer_over,
            box_fill: r.check_box_check_background_fill_indeterminate_pointer_over,
            glyph: r.check_box_check_glyph_foreground_indeterminate_pointer_over,
        },
        (CheckState::Indeterminate, CommonState::Pressed) => CheckBoxBrushes {
            foreground: r.check_box_foreground_indeterminate_pressed,
            background: r.check_box_background_indeterminate_pressed,
            border: r.check_box_border_brush_indeterminate_pressed,
            box_stroke: r.check_box_check_background_stroke_indeterminate_pressed,
            box_fill: r.check_box_check_background_fill_indeterminate_pressed,
            glyph: r.check_box_check_glyph_foreground_indeterminate_pressed,
        },
        (CheckState::Indeterminate, CommonState::Disabled) => CheckBoxBrushes {
            foreground: r.check_box_foreground_indeterminate_disabled,
            background: r.check_box_background_indeterminate_disabled,
            border: r.check_box_border_brush_indeterminate_disabled,
            box_stroke: r.check_box_check_background_stroke_indeterminate_disabled,
            box_fill: r.check_box_check_background_fill_indeterminate_disabled,
            glyph: r.check_box_check_glyph_foreground_indeterminate_disabled,
        },
    }
}

/// The handler for XAML `Checked`, `Unchecked` and `Indeterminate`, given the `IsChecked` the control wants.
pub type CheckBoxCheckedHandler = Rc<dyn Fn(&mut App, Option<bool>)>;

/// XAML `CheckBox`: `IsChecked` (`None` is indeterminate) with `IsThreeState`, `Content`, and the `Checked` / `Unchecked` / `Indeterminate` events as one handler.
#[derive(Clone)]
pub struct CheckBox {
    pub is_checked: Option<bool>,
    /// XAML `Checked`, `Unchecked` and `Indeterminate`, with the value the control wants.
    pub checked: CheckBoxCheckedHandler,
    pub content: Option<WidgetRef>,
    pub is_three_state: bool,
    pub is_enabled: bool,
    pub key: Option<KeyRef>,
}

impl CheckBox {
    pub fn new(
        is_checked: Option<bool>,
        checked: impl Fn(&mut App, Option<bool>) + 'static,
    ) -> CheckBox {
        CheckBox {
            is_checked,
            checked: Rc::new(checked),
            content: None,
            is_three_state: false,
            is_enabled: true,
            key: None,
        }
    }

    /// XAML `Content`.
    pub fn content<K>(mut self, content: impl IntoWidget<K>) -> CheckBox {
        self.content = Some(content.into_widget());
        self
    }

    /// XAML `IsThreeState`.
    pub fn is_three_state(mut self, three_state: bool) -> CheckBox {
        self.is_three_state = three_state;
        self
    }

    /// XAML `IsEnabled`.
    pub fn is_enabled(mut self, enabled: bool) -> CheckBox {
        self.is_enabled = enabled;
        self
    }

    pub fn key(mut self, key: KeyRef) -> CheckBox {
        self.key = Some(key);
        self
    }

    fn set_is_checked(&self, value: Option<bool>) -> Listener {
        let checked = self.checked.clone();
        Listener::new(move |app| checked(app, value))
    }
}

impl fmt::Debug for CheckBox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CheckBox")
            .field("is_checked", &self.is_checked)
            .field("is_three_state", &self.is_three_state)
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

impl StatelessWidget for CheckBox {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.clone();
        // `ToggleButton::OnClick` → `OnToggle`.
        let click = self.set_is_checked(toggle_is_checked(self.is_checked, self.is_three_state));
        // `CheckBox::Initialize`: `SetAcceptsReturn(false)`.
        let states = CommonStates::new(click, move |app, context, states| {
            template(app, context, &widget, states)
        })
        .is_enabled(self.is_enabled)
        .accepts_return(false);
        CheckBoxKeys {
            check: self.set_is_checked(Some(true)),
            uncheck: self.set_is_checked(Some(false)),
            is_three_state: self.is_three_state,
            is_enabled: self.is_enabled,
            child: states.into_widget(),
        }
        .into_widget()
    }
}

/// The `ControlTemplate` for one set of states.
fn template(
    app: &mut App,
    context: BuildContext,
    widget: &CheckBox,
    states: ControlStates,
) -> WidgetRef {
    let resources = ThemeResources::of(app, context);
    let check = CheckState::from_is_checked(widget.is_checked);
    let brushes = check_box_brushes(&resources, check, states.common);
    // `RootGrid`.
    let root = Grid::new()
        .column_definitions([
            ColumnDefinition::new(GridLength::AUTO),
            ColumnDefinition::new(GridLength::STAR),
        ])
        .background(Brush::Solid(brushes.background))
        .border_brush(Brush::Solid(brushes.border))
        .border_thickness(CHECK_BOX_ROOT_BORDER_THICKNESS)
        .corner_radius(resources.control_corner_radius[0])
        .children([
            GridCell::new(check_area(&resources, &brushes, check)).into_widget(),
            GridCell::new(content_presenter(&resources, widget, &brushes))
                .column(1)
                .into_widget(),
        ]);
    // The style's `MinWidth` and `MinHeight`.
    let body = ConstrainedBox::new(
        BoxConstraints::new()
            .min_width(resources.check_box_min_width)
            .min_height(resources.check_box_height),
    )
    .child(root);
    FocusVisual::new(body, resources.theme)
        .visible(states.focused)
        .margin(resources.check_box_focus_visual_margin)
        .corner_radius(resources.control_corner_radius[0])
        .into_widget()
}

/// The 20 × 20 grid in column 0 (`VerticalAlignment="Center"`) with `NormalRectangle` and `CheckGlyph` in its one cell.
fn check_area(r: &ThemeResources, brushes: &CheckBoxBrushes, check: CheckState) -> WidgetRef {
    // `NormalRectangle`: a `Rectangle` fills under its stroke.
    let rectangle = ControlBorder::new(
        Brush::Solid(brushes.box_fill),
        Brush::Solid(brushes.box_stroke),
    )
    .border_thickness(r.check_box_border_thickness)
    .corner_radius(r.control_corner_radius[0])
    .background_sizing(BackgroundSizing::OuterBorderEdge);
    let glyph = CheckGlyph {
        check,
        color: brushes.glyph,
    };
    // `CheckGlyph.Margin`: `0,1,0,-1`, and `0` from the indeterminate states' setters.
    let glyph = match check {
        CheckState::Indeterminate => glyph.into_widget(),
        _ => Transform::translate(CHECK_GLYPH_OFFSET)
            .child(glyph)
            .into_widget(),
    };
    let grid = Grid::new().children([
        GridCell::new(rectangle).into_widget(),
        GridCell::new(glyph).into_widget(),
    ]);
    Center::new()
        .width_factor(1.0)
        .height_factor(1.0)
        .child(
            SizedBox::new()
                .width(r.check_box_size)
                .height(r.check_box_size)
                .child(grid),
        )
        .into_widget()
}

/// `ContentPresenter` in column 1: `Margin` is the style's `Padding`, aligned left and top.
fn content_presenter(
    r: &ThemeResources,
    widget: &CheckBox,
    brushes: &CheckBoxBrushes,
) -> WidgetRef {
    let [left, top, right, bottom] = r.check_box_padding;
    let text = control_text_style(
        r,
        r.control_content_theme_font_size,
        FontWeight::W400,
        brushes.foreground,
    );
    let presenter = Align::new()
        .alignment(AlignmentGeometry::TOP_LEFT)
        .width_factor(1.0)
        .height_factor(1.0);
    let presenter = match &widget.content {
        Some(content) => presenter.child(DefaultTextStyle::new(text, content.clone())),
        None => presenter,
    };
    Padding::new(EdgeInsetsGeometry::from_ltrb(left, top, right, bottom))
        .child(presenter)
        .into_widget()
}

/// `CheckBox::OnKeyDownInternal`: `VirtualKey_Add` checks and `VirtualKey_Subtract` unchecks a two-state box.
#[derive(Clone)]
struct CheckBoxKeys {
    check: Listener,
    uncheck: Listener,
    is_three_state: bool,
    is_enabled: bool,
    child: WidgetRef,
}

impl fmt::Debug for CheckBoxKeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CheckBoxKeys")
            .field("is_three_state", &self.is_three_state)
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
struct CheckIntent;
impl Intent for CheckIntent {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug)]
struct UncheckIntent;
impl Intent for UncheckIntent {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct CheckBoxKeysState {
    state: StateData<CheckBoxKeys>,
    actions: HashMap<TypeId, AnyAction>,
}

impl StatefulWidget for CheckBoxKeys {
    type State = CheckBoxKeysState;
    fn create_state(&self) -> CheckBoxKeysState {
        CheckBoxKeysState {
            state: StateData::new(),
            actions: HashMap::new(),
        }
    }
}

impl CheckBoxKeysState {
    fn shortcuts(widget: &CheckBoxKeys) -> ShortcutMap {
        let bind =
            |key: LogicalKeyboardKey, intent: IntentRef| -> (ShortcutActivatorRef, IntentRef) {
                (Rc::new(SingleActivator::new(key)), intent)
            };
        let mut map = Vec::new();
        if !widget.is_three_state && widget.is_enabled {
            map.push(bind(LogicalKeyboardKey::NUMPAD_ADD, Rc::new(CheckIntent)));
            map.push(bind(
                LogicalKeyboardKey::NUMPAD_SUBTRACT,
                Rc::new(UncheckIntent),
            ));
        }
        map
    }
}

impl State for CheckBoxKeysState {
    type Widget = CheckBoxKeys;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let check = CallbackAction::<CheckIntent>::new(
            app,
            Rc::new(move |app, _| {
                self.widget(app).check.clone().call(app);
                None
            }),
        );
        let uncheck = CallbackAction::<UncheckIntent>::new(
            app,
            Rc::new(move |app, _| {
                self.widget(app).uncheck.clone().call(app);
                None
            }),
        );
        let actions = &mut app.get_mut(self).actions;
        actions.insert(TypeId::of::<CheckIntent>(), check.as_action());
        actions.insert(TypeId::of::<UncheckIntent>(), uncheck.as_action());
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        for action in std::mem::take(&mut app.get_mut(self).actions).into_values() {
            app.destroy(action.id());
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let actions = app.get(self).actions.clone();
        Shortcuts::new(
            Self::shortcuts(&widget),
            Actions::new(actions, widget.child),
        )
        .into_widget()
    }
}

/// `CheckGlyph`: the `AnimatedIcon` showing `AnimatedAcceptVisualSource` at the state's marker, playing the transition segment when the check state changes.
#[derive(Clone, Copy, Debug, PartialEq)]
struct CheckGlyph {
    check: CheckState,
    color: Color,
}

/// Which trim the playing segment animates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Segment {
    /// At the marker: the state's still frame.
    Still,
    /// `NormalOffToNormalOn`: `TrimEnd` 0 → 1.
    DrawOn,
    /// `NormalOnToNormalOff`: `TrimStart` 0 → 1.
    Erase,
}

struct CheckGlyphState {
    state: StateData<CheckGlyph>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    controller: Option<Handle<AnimationController>>,
    segment: Segment,
}

impl StatefulWidget for CheckGlyph {
    type State = CheckGlyphState;
    fn create_state(&self) -> CheckGlyphState {
        CheckGlyphState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::default(),
            controller: None,
            segment: Segment::Still,
        }
    }
}

impl SingleTickerProviderStateMixin for CheckGlyphState {
    fn single_ticker_provider_data(
        self: Handle<Self>,
        app: &App,
    ) -> &SingleTickerProviderStateMixinData {
        &app.get(self).single_ticker_provider
    }
    fn single_ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut SingleTickerProviderStateMixinData {
        &mut app.get_mut(self).single_ticker_provider
    }
}

impl TickerProviderObject for CheckGlyphState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl CheckGlyphState {
    fn controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self).controller.expect("created in init_state")
    }

    /// `AnimatedIcon::PlaySegment` for the transition `from` → `to`; a transition into or out of the indeterminate state has a single-position marker and jumps.
    fn play(self: Handle<Self>, app: &mut App, from: CheckState, to: CheckState) {
        let (segment, markers, easing) = match (from, to) {
            (CheckState::Unchecked, CheckState::Checked) => {
                (Segment::DrawOn, NORMAL_OFF_TO_NORMAL_ON, TRIM_END_EASING)
            }
            (CheckState::Checked, CheckState::Unchecked) => {
                (Segment::Erase, NORMAL_ON_TO_NORMAL_OFF, TRIM_START_EASING)
            }
            _ => {
                app.get_mut(self).segment = Segment::Still;
                self.controller(app).set_value(app, 1.0);
                return;
            }
        };
        app.get_mut(self).segment = segment;
        let controller = self.controller(app);
        controller.set_value(app, 0.0);
        let [x1, y1, x2, y2] = easing;
        let curve: Rc<dyn Curve> = Rc::new(Cubic::new(x1, y1, x2, y2));
        controller.animate_to(app, 1.0, Some(segment_duration(markers)), curve);
    }

    /// The glyph and its trim at this frame.
    fn frame(self: Handle<Self>, app: &App) -> Option<(Glyph, (f32, f32))> {
        let value = self.controller(app).value(app) as f32;
        let check = self.widget(app).check;
        match (app.get(self).segment, check) {
            (Segment::DrawOn, _) => Some((Glyph::Check, (0.0, value))),
            (Segment::Erase, _) => Some((Glyph::Check, (value, 1.0))),
            (Segment::Still, CheckState::Checked) => Some((Glyph::Check, (0.0, 1.0))),
            (Segment::Still, CheckState::Indeterminate) => Some((Glyph::Indeterminate, (0.0, 1.0))),
            (Segment::Still, CheckState::Unchecked) => None,
        }
    }
}

impl State for CheckGlyphState {
    type Widget = CheckGlyph;
    inset_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = AnimationController::create(
            app,
            Some(1.0),
            Some(ACCEPT_VISUAL_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        controller.add_listener(app, Listener::new(move |app| self.set_state(app, |_| {})));
        controller.add_status_listener(
            app,
            AnimationStatusListener::new(move |status, app| {
                if status.is_completed() {
                    self.set_state(app, |state| state.segment = Segment::Still);
                }
            }),
        );
        app.get_mut(self).controller = Some(controller);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &CheckGlyph) {
        let check = self.widget(app).check;
        if check != old_widget.check {
            self.play(app, old_widget.check, check);
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.controller(app).dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let painter = AcceptVisualPainter {
            frame: self.frame(app),
            color: self.widget(app).color,
        };
        CustomPaint::new().painter(painter).into_widget()
    }
}

/// The two geometries of `AnimatedAcceptVisualSource`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Glyph {
    /// `Geometry_1`.
    Check,
    /// `Geometry_0`.
    Indeterminate,
}

impl Glyph {
    fn polyline(self) -> &'static [(f32, f32)] {
        match self {
            Glyph::Check => &CHECK_POLYLINE,
            Glyph::Indeterminate => &INDETERMINATE_POLYLINE,
        }
    }

    fn sprite_offset(self) -> (f32, f32) {
        match self {
            Glyph::Check => CHECK_SPRITE_OFFSET,
            Glyph::Indeterminate => INDETERMINATE_SPRITE_OFFSET,
        }
    }
}

/// Paints one frame of `AnimatedAcceptVisualSource` scaled into its bounds as `AnimatedIcon::ArrangeOverride` does: uniformly, centred.
#[derive(Clone, Debug, PartialEq)]
struct AcceptVisualPainter {
    frame: Option<(Glyph, (f32, f32))>,
    color: Color,
}

impl CustomPainter for AcceptVisualPainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        let Some((glyph, trim)) = self.frame else {
            return;
        };
        let Some(path) = trimmed_polyline(glyph.polyline(), trim) else {
            return;
        };
        let scale = (size.width().min(size.height()) / ACCEPT_VISUAL_SIZE) as f32;
        let canvas_size = ACCEPT_VISUAL_SIZE as f32 * scale;
        let (dx, dy) = glyph.sprite_offset();
        canvas.save();
        // `AnimatedIcon`: the root visual's offset and scale.
        canvas.translate(
            (size.width() as f32 - canvas_size) / 2.0,
            (size.height() as f32 - canvas_size) / 2.0,
        );
        canvas.scale(scale, scale);
        // `Null 230`: scale 1.05 about the canvas centre.
        canvas.translate(
            ACCEPT_VISUAL_SIZE as f32 / 2.0,
            ACCEPT_VISUAL_SIZE as f32 / 2.0,
        );
        canvas.scale(LAYER_SCALE, LAYER_SCALE);
        canvas.translate(
            -ACCEPT_VISUAL_SIZE as f32 / 2.0,
            -ACCEPT_VISUAL_SIZE as f32 / 2.0,
        );
        // The sprite shape's `TransformMatrix`.
        canvas.translate(dx, dy);
        canvas.scale(SPRITE_SCALE, SPRITE_SCALE);
        let mut stroke = Stroke::new(ACCEPT_STROKE_THICKNESS);
        stroke.cap = Cap::Round;
        stroke.join = Join::Round;
        let mut paint = Paint::from_color(self.color.into());
        paint.style = PaintStyle::Stroke(stroke);
        canvas.draw_path(&path, FillRule::NonZero, &paint);
        canvas.restore();
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        old_delegate.as_any().downcast_ref::<Self>() != Some(self)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// `CompositionPathGeometry.TrimStart` / `TrimEnd`: the part of `points` between the two fractions of its length; `None` when nothing is left.
fn trimmed_polyline(
    points: &[(f32, f32)],
    (start, end): (f32, f32),
) -> Option<std::sync::Arc<inset_embedder::Path>> {
    let lengths: Vec<f32> = points
        .windows(2)
        .map(|pair| {
            let (ax, ay) = pair[0];
            let (bx, by) = pair[1];
            ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt()
        })
        .collect();
    let total: f32 = lengths.iter().sum();
    let (from, to) = (start.clamp(0.0, 1.0) * total, end.clamp(0.0, 1.0) * total);
    if to <= from {
        return None;
    }
    let point_at = |distance: f32| -> Point {
        let mut travelled = 0.0;
        let mut index = 0;
        while index + 1 < lengths.len() && distance > travelled + lengths[index] {
            travelled += lengths[index];
            index += 1;
        }
        let t = ((distance - travelled) / lengths[index]).clamp(0.0, 1.0);
        let (ax, ay) = points[index];
        let (bx, by) = points[index + 1];
        Point::new(ax + (bx - ax) * t, ay + (by - ay) * t)
    };
    let mut path = PathBuilder::new();
    path.move_to(point_at(from));
    let mut travelled = 0.0;
    for (&(x, y), length) in points[1..].iter().zip(&lengths) {
        travelled += *length;
        if travelled > from && travelled < to {
            path.line_to(Point::new(x, y));
        }
    }
    path.line_to(point_at(to));
    Some(path.build())
}
