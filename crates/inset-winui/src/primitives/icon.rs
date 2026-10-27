//! The XAML `IconElement` family from `core/elements/icon.cpp`: `FontIcon`, a glyph centred in
//! the icon's bounds, and `PathIcon`, a geometry filled with the icon's foreground. Each takes
//! the enclosing control's text foreground when it carries none of its own. The bundled open
//! Fluent System Icons font stands in for the Windows `Segoe Fluent Icons` family.

use std::sync::Arc;

use inset_embedder::{
    Canvas, Color, FillRule, FontStyle, FontWeight, Paint, Path, Size, TextAlign, TextDirection,
};
use inset_foundation::App;
use inset_painting::{PaintingBinding, TextScaler, TextStyle};
use inset_rendering::CustomPainter;
use inset_widgets::*;

/// `g_ClientCoreFontSize` of `icon.cpp`: the glyph size an icon uses when its `FontSize` is left
/// at its default, rather than the enclosing control's text size.
pub const ICON_FONT_SIZE: f64 = 20.0;

/// The colour an icon falls back to when neither it nor its surroundings name a foreground,
/// matching what a `TextBlock` in the same position would draw.
const DEFAULT_ICON_FOREGROUND: Color = Color::from_argb(255, 0, 0, 0);

/// The family under which the bundled open icon font is registered.
pub const FLUENT_ICON_FONT_FAMILY: &str = "FluentSystemIcons-Regular";

/// Registers the bundled font before the first icon is laid out.
pub fn install_icon_font(app: &mut App) {
    PaintingBinding::instance(app).register_font(
        app,
        FLUENT_ICON_FONT_FAMILY,
        include_bytes!("../../assets/fluent-system-icons/FluentSystemIcons-Regular.ttf").to_vec(),
    );
}

/// Built-in control symbols, mapped to the open Fluent System Icons codepoints.
/// These names intentionally do not promise Segoe's private-use glyph mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FluentSymbol {
    /// A list of menu commands.
    MenuList,
    /// A menu check mark.
    Checkmark,
    /// General information.
    Info,
    /// Successful completion.
    CheckmarkCircle,
    /// An error status.
    DismissCircle,
    /// A warning status.
    Warning,
    /// An exclamation mark inside a circle.
    ErrorCircle,
    /// Progress or elapsed time.
    Timer,
    /// Collapsible sections.
    ChevronDownUp,

    /// Activate an action.
    CursorClick,
    /// Switch a setting.
    ToggleLeft,
    /// A checked choice.
    CheckboxChecked,
    /// A mixed selection.
    CheckboxIndeterminate,
    /// Choose one option.
    RadioButton,
    /// Arrange rows and columns.
    Grid,
    /// Adjust a value or configuration.
    Options,
    /// A side pane with content.
    PanelLeft,
    /// Switch between tabbed pages.
    Tab,
    /// Switch between a few views.
    Tabs,
    /// Contextual help.
    TooltipQuote,
    /// Layered surfaces and materials.
    Layer,
    /// Navigate to a link.
    Link,
    /// Repeat an action.
    ArrowRepeatAll,
    /// Text and typography.
    TextFont,
    /// A collection of symbols or features.
    Apps,
    /// Switch to a dark theme.
    WeatherMoon,
    /// Switch to a light theme.
    WeatherSunny,
    /// Add a tab or item.
    Add,
    /// Close a tab or dismiss a surface.
    Dismiss,
    /// Scroll toward the leading edge in left-to-right layout.
    ChevronLeft,
    /// Scroll toward the trailing edge in left-to-right layout.
    ChevronRight,
    /// Expand a child list or open a menu.
    ChevronDown,
    /// Upward disclosure or scroll affordance.
    ChevronUp,
    /// A value that steps up or down, as NumberBox's popup indicator.
    ChevronUpDown,
    /// Toggle the navigation pane.
    Navigation,
    /// Opens the navigation search presenter.
    Search,
    /// Navigate back.
    Back,
    /// Open settings.
    Settings,
    /// Open the overflow menu.
    More,
    /// Temporarily reveal concealed text.
    Eye,
    /// Editable text input.
    TextField,
    /// Password or protected input.
    LockClosed,
    /// A status dot or badge.
    Circle,
    /// Colour choice: a palette.
    Color,
    /// A hierarchy of items.
    TextBulletListTree,
    /// Work whose completion is shown as a ring.
    SpinnerIos,
}

impl FluentSymbol {
    /// The glyph from the bundled revision's `codepoints.json`.
    pub fn glyph(self) -> char {
        char::from_u32(match self {
            Self::MenuList => 60642,
            Self::Checkmark => 62099,
            Self::Info => 62626,
            Self::CheckmarkCircle => 62103,
            Self::DismissCircle => 62316,
            Self::Warning => 63592,
            Self::ErrorCircle => 62448,
            Self::Timer => 60808,
            Self::ChevronDownUp => 983404,
            Self::CursorClick => 58437,
            Self::ToggleLeft => 60816,
            Self::CheckboxChecked => 62093,
            Self::CheckboxIndeterminate => 58110,
            Self::RadioButton => 63044,
            Self::Grid => 62562,
            Self::Options => 62855,
            Self::PanelLeft => 59568,
            Self::Tab => 63300,
            Self::Tabs => 60527,
            Self::TooltipQuote => 63419,
            Self::Layer => 62667,
            Self::Link => 62692,
            Self::ArrowRepeatAll => 61809,
            Self::TextFont => 63460,
            Self::Apps => 61747,
            Self::WeatherMoon => 63613,
            Self::WeatherSunny => 63649,
            Self::Add => 61704,
            Self::Dismiss => 62312,
            Self::ChevronLeft => 62121,
            Self::ChevronRight => 62127,
            Self::ChevronDown => 62114,
            Self::ChevronUp => 62134,
            Self::ChevronUpDown => 58158,
            Self::Search => 63119,
            Self::Navigation => 62816,
            Self::Back => 61787,
            Self::Settings => 63145,
            Self::More => 59428,
            Self::Eye => 58866,
            Self::TextField => 63455,
            Self::LockClosed => 59279,
            Self::Circle => 62140,
            Self::Color => 62197,
            Self::TextBulletListTree => 63406,
            Self::SpinnerIos => 63241,
        })
        .unwrap()
    }
}

/// XAML `FontIcon`: a glyph centred in the icon's bounds, drawn in `FontFamily` at `FontSize`
/// and optionally reflected in right-to-left layout. The glyph shapes use the documented open
/// font substitute.
#[derive(Clone, Debug)]
pub struct FontIcon {
    /// `Glyph`: the character or characters shown.
    pub glyph: String,
    /// `FontFamily`: the family the glyph is taken from; the bundled icon font by default.
    pub font_family: String,
    /// `FontSize`: `g_ClientCoreFontSize`, 20, unless the caller sets it.
    pub font_size: f64,
    /// `FontWeight`.
    pub font_weight: FontWeight,
    /// `FontStyle`.
    pub font_style: FontStyle,
    /// `MirroredWhenRightToLeft`: whether the glyph is reflected horizontally in right-to-left
    /// layout.
    pub mirrored_when_right_to_left: bool,
    /// `IconElement.Foreground`, or the enclosing control's text foreground.
    pub foreground: Option<Color>,
    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl FontIcon {
    /// Creates an icon for the glyph, in the bundled icon font at the source default size.
    pub fn new(glyph: impl Into<String>) -> Self {
        Self {
            glyph: glyph.into(),
            font_family: FLUENT_ICON_FONT_FAMILY.to_string(),
            font_size: ICON_FONT_SIZE,
            font_weight: FontWeight::NORMAL,
            font_style: FontStyle::Normal,
            mirrored_when_right_to_left: false,
            foreground: None,
            key: None,
        }
    }

    /// Creates an icon for one of the kit's built-in control symbols.
    pub fn symbol(symbol: FluentSymbol) -> Self {
        Self::new(symbol.glyph().to_string())
    }

    /// Sets the XAML `FontFamily` the glyph is taken from.
    pub fn font_family(mut self, value: impl Into<String>) -> Self {
        self.font_family = value.into();
        self
    }

    /// Sets the XAML `FontSize`.
    pub fn font_size(mut self, value: f64) -> Self {
        self.font_size = value;
        self
    }

    /// Sets the XAML `FontWeight`.
    pub fn font_weight(mut self, value: FontWeight) -> Self {
        self.font_weight = value;
        self
    }

    /// Sets the XAML `FontStyle`.
    pub fn font_style(mut self, value: FontStyle) -> Self {
        self.font_style = value;
        self
    }

    /// Sets the XAML `MirroredWhenRightToLeft`.
    pub fn mirrored_when_right_to_left(mut self, value: bool) -> Self {
        self.mirrored_when_right_to_left = value;
        self
    }

    /// Sets the XAML `Foreground` instead of inheriting the label colour.
    pub fn foreground(mut self, value: Color) -> Self {
        self.foreground = Some(value);
        self
    }

    /// Sets the identity retained across rebuilds.
    pub fn key(mut self, value: KeyRef) -> Self {
        self.key = Some(value);
        self
    }
}

impl StatelessWidget for FontIcon {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let mut style = TextStyle::new()
            .font_family(self.font_family.clone())
            .font_size(self.font_size)
            .font_weight(self.font_weight)
            .font_style(self.font_style);
        if let Some(color) = self.foreground {
            style = style.color(color);
        }
        // FontIcon's child TextBlock is centred and disables text scale factors.
        let glyph = Text::new(self.glyph.clone())
            .style(style)
            .text_align(TextAlign::Center)
            .text_scaler(TextScaler::NO_SCALING);
        let glyph = Center::new()
            .width_factor(1.0)
            .height_factor(1.0)
            .child(glyph);
        if self.mirrored_when_right_to_left
            && Directionality::of(app, context) == TextDirection::Rtl
        {
            Transform::scale(None, Some(-1.0), Some(1.0))
                .child(glyph)
                .into_widget()
        } else {
            glyph.into_widget()
        }
    }
}

/// XAML `PathIcon`: a geometry filled with the icon's foreground. As `Shape` lays out with
/// `Stretch.None`, the icon asks for the space between the origin and the geometry's far edges,
/// and the geometry keeps its own coordinates.
#[derive(Clone, Debug)]
pub struct PathIcon {
    /// `Data`: the geometry to fill.
    pub data: Arc<Path>,
    /// `Geometry.FillRule`: which regions of a self-crossing geometry are filled;
    /// `XcpFillModeAlternate`, the even-odd rule, is the source default.
    pub fill_rule: FillRule,
    /// `IconElement.Foreground`, or the enclosing control's text foreground.
    pub foreground: Option<Color>,
    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl PathIcon {
    /// Creates an icon filling the geometry.
    pub fn new(data: impl Into<Arc<Path>>) -> Self {
        Self {
            data: data.into(),
            fill_rule: FillRule::EvenOdd,
            foreground: None,
            key: None,
        }
    }

    /// Sets the XAML `Geometry.FillRule`.
    pub fn fill_rule(mut self, value: FillRule) -> Self {
        self.fill_rule = value;
        self
    }

    /// Sets the XAML `Foreground` instead of inheriting the label colour.
    pub fn foreground(mut self, value: Color) -> Self {
        self.foreground = Some(value);
        self
    }

    /// Sets the identity retained across rebuilds.
    pub fn key(mut self, value: KeyRef) -> Self {
        self.key = Some(value);
        self
    }
}

impl StatelessWidget for PathIcon {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let fill = self.foreground.unwrap_or_else(|| {
            DefaultTextStyle::of(app, context)
                .style
                .color
                .map_or(DEFAULT_ICON_FOREGROUND, |color| color.color())
        });
        // CShape::MeasureOverride with Stretch.None asks for the geometry's far edges.
        let bounds = self.data.tight_bounds();
        let size = Size::new(
            f64::from(bounds.x + bounds.width).max(0.0),
            f64::from(bounds.y + bounds.height).max(0.0),
        );
        CustomPaint::new()
            .size(size)
            .painter(PathIconPainter {
                data: self.data.clone(),
                fill_rule: self.fill_rule,
                fill,
            })
            .into_widget()
    }
}

/// The `Path` inside `PathIcon`'s template, filled with the resolved foreground.
#[derive(Clone, Debug)]
struct PathIconPainter {
    data: Arc<Path>,
    fill_rule: FillRule,
    fill: Color,
}

impl CustomPainter for PathIconPainter {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, _size: Size) {
        canvas.draw_path(
            &self.data,
            self.fill_rule,
            &Paint::from_color(self.fill.into()),
        );
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        match old_delegate.as_any().downcast_ref::<Self>() {
            Some(old) => {
                old.fill != self.fill
                    || old.fill_rule != self.fill_rule
                    || !Arc::ptr_eq(&old.data, &self.data)
                        && !old.data.elements().eq(self.data.elements())
            }
            None => true,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
