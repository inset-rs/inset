//! XAML `InfoBadge` (`controls/dev/InfoBadge/InfoBadge_themeresources.xaml` and
//! `InfoBadge.cpp`): a small status badge showing a number, an icon, or a bare dot, filled with
//! its severity colour and rounded to a stadium by half its own height.

use crate::{Brush, FontIcon, Margin, ThemeResources, control_text_style};
use inset_embedder::{Canvas, Color, FontWeight, Matrix4, Offset, RRect, Radius, Size};
use inset_foundation::App;
use inset_painting::EdgeInsets;
use inset_rendering::*;
use inset_widgets::*;

/// The named `InfoBadge` styles of `InfoBadge_themeresources.xaml`, which differ in the
/// background their severity calls for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InfoBadgeStyle {
    /// `DefaultInfoBadgeStyle`: `InfoBadgeBackground`, the accent fill.
    #[default]
    Default,
    /// `AttentionDotInfoBadgeStyle` and its value and icon variants.
    Attention,
    /// `InformationalDotInfoBadgeStyle` and its value and icon variants.
    Informational,
    /// `SuccessDotInfoBadgeStyle` and its value and icon variants.
    Success,
    /// `CautionDotInfoBadgeStyle` and its value and icon variants.
    Caution,
    /// `CriticalDotInfoBadgeStyle` and its value and icon variants.
    Critical,
}

impl InfoBadgeStyle {
    /// The `Background` setter of the style, resolved in this theme.
    pub fn background(self, resources: &ThemeResources) -> Color {
        let common = &resources;
        match self {
            Self::Default => resources.info_badge_background,
            Self::Attention => common.system_fill_color_attention_brush,
            Self::Informational => common.system_fill_color_solid_neutral_brush,
            Self::Success => common.system_fill_color_success_brush,
            Self::Caution => common.system_fill_color_caution_brush,
            Self::Critical => common.system_fill_color_critical_brush,
        }
    }
}

/// The `DisplayKindStates` group of the template, as `OnDisplayKindPropertiesChanged` chooses it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InfoBadgeDisplayKind {
    /// `Dot`: neither a value nor an icon, leaving the filled shape alone.
    Dot,
    /// `Value`: the number, which takes precedence over an icon.
    Value,
    /// `Icon`: an icon that is not a glyph, inset on all four sides.
    Icon,
    /// `FontIcon`: a glyph, which needs no inset above it.
    FontIcon,
}

impl InfoBadgeDisplayKind {
    /// The margin the state's setter gives the visible element.
    fn margin(self, r: &ThemeResources) -> [f64; 4] {
        match self {
            Self::Dot => [0.0; 4],
            Self::Value => r.value_info_badge_text_margin,
            Self::Icon => r.icon_info_badge_icon_margin,
            Self::FontIcon => r.icon_info_badge_font_icon_margin,
        }
    }
}

/// XAML `InfoBadge`: a badge showing a count, an icon or a dot.
#[derive(Clone, Debug)]
pub struct InfoBadge {
    /// `Value`: the number shown, or -1 for none. Values below -1 are out of bounds.
    pub value: i32,

    /// `IconSource`: the icon shown when no value is set.
    pub icon_source: Option<WidgetRef>,

    /// The named style whose severity supplies the background.
    pub style: InfoBadgeStyle,

    /// `Background`, in place of the style's.
    pub background: Option<Brush>,

    /// Inherited number and icon color, in place of `InfoBadgeForeground`.
    pub foreground: Option<Color>,

    /// `CornerRadius` when the application sets one; otherwise half the arranged height, as
    /// `OnSizeChanged` computes it.
    pub corner_radius: Option<[f64; 4]>,

    /// `Padding` inside the filled shape; the theme's `InfoBadgePadding` when `None`.
    pub padding: Option<[f64; 4]>,

    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl Default for InfoBadge {
    fn default() -> Self {
        Self {
            // `MUX_DEFAULT_VALUE("-1")` of InfoBadge.idl.
            value: -1,
            icon_source: None,
            style: InfoBadgeStyle::Default,
            background: None,
            foreground: None,
            corner_radius: None,
            padding: None,
            key: None,
        }
    }
}

impl InfoBadge {
    /// Creates a badge showing neither a value nor an icon: the source's dot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the XAML `Value`; `OnPropertyChanged` rejects anything below -1.
    pub fn value(mut self, value: i32) -> Self {
        assert!(value >= -1, "Value must be equal to or greater than -1");
        self.value = value;
        self
    }

    /// Sets the XAML `IconSource`.
    pub fn icon_source<K>(mut self, icon: impl IntoWidget<K>) -> Self {
        self.icon_source = Some(icon.into_widget());
        self
    }

    /// Sets which named style supplies the background.
    pub fn style(mut self, style: InfoBadgeStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets the XAML `Background`.
    pub fn background(mut self, background: Brush) -> Self {
        self.background = Some(background);
        self
    }

    /// Sets the XAML `Foreground`.
    pub fn foreground(mut self, foreground: Color) -> Self {
        self.foreground = Some(foreground);
        self
    }

    /// Sets the XAML `CornerRadius`, which the source keeps instead of halving the height.
    pub fn corner_radius(mut self, radii: [f64; 4]) -> Self {
        self.corner_radius = Some(radii);
        self
    }

    /// Sets the XAML `Padding`.
    pub fn padding(mut self, padding: [f64; 4]) -> Self {
        self.padding = Some(padding);
        self
    }

    /// Sets the identity retained across rebuilds.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }

    /// `InfoBadge::OnDisplayKindPropertiesChanged`: the value wins over an icon, and a glyph
    /// icon takes the tighter margin.
    pub fn display_kind(&self) -> InfoBadgeDisplayKind {
        if self.value >= 0 {
            InfoBadgeDisplayKind::Value
        } else if let Some(icon) = &self.icon_source {
            if downcast_widget::<FontIcon>(icon.as_ref()).is_some() {
                InfoBadgeDisplayKind::FontIcon
            } else {
                InfoBadgeDisplayKind::Icon
            }
        } else {
            InfoBadgeDisplayKind::Dot
        }
    }
}

impl StatelessWidget for InfoBadge {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let foreground = self
            .foreground
            .unwrap_or_else(|| resources.info_badge_foreground);
        let kind = self.display_kind();
        // ValueTextBlock and IconPresenter, of which one state at a time is visible.
        let content = match kind {
            InfoBadgeDisplayKind::Dot => SizedBox::new().into_widget(),
            InfoBadgeDisplayKind::Value => Margin::new(
                kind.margin(&resources),
                // HorizontalAlignment and VerticalAlignment are Center, so the text keeps its
                // own size until the badge is squared up around it.
                Center::new().width_factor(1.0).height_factor(1.0).child(
                    Text::new(self.value.to_string()).style(control_text_style(
                        &resources,
                        resources.info_badge_value_font_size,
                        FontWeight::NORMAL,
                        foreground,
                    )),
                ),
            )
            .into_widget(),
            InfoBadgeDisplayKind::Icon | InfoBadgeDisplayKind::FontIcon => Margin::new(
                kind.margin(&resources),
                // IconPresenter is a Viewbox: it keeps the icon's own size where there is room
                // and scales it down to whatever height the badge is left with.
                Center::new()
                    .width_factor(1.0)
                    .height_factor(1.0)
                    .child(FittedBox::new().child(self.icon_source.clone().unwrap())),
            )
            .into_widget(),
        };
        let content = DefaultTextStyle::new(
            DefaultTextStyle::of(app, context)
                .style
                .clone()
                .color(foreground),
            content,
        );

        // RootGrid: the filled, rounded background around the padded content.
        let [left, top, right, bottom] = self.padding.unwrap_or(resources.info_badge_padding);
        let root = CustomPaint::new()
            .painter(BadgeFill {
                background: self
                    .background
                    .unwrap_or_else(|| Brush::Solid(self.style.background(&resources))),
                corner_radius: self.corner_radius,
            })
            .child(
                Padding::new(EdgeInsets::from_ltrb(left, top, right, bottom).into()).child(content),
            );
        ConstrainedBox::new(BoxConstraints {
            min_width: resources.info_badge_min_width,
            max_width: f64::INFINITY,
            min_height: resources.info_badge_min_height,
            max_height: resources.info_badge_max_height,
        })
        .child(SquareWhenTallerThanWide::new(root))
        .into_widget()
    }
}

/// RootGrid's fill: `Background` inside `TemplateSettings.InfoBadgeCornerRadius`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct BadgeFill {
    background: Brush,
    corner_radius: Option<[f64; 4]>,
}

impl CustomPainter for BadgeFill {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        // InfoBadge::OnSizeChanged halves the arranged height unless CornerRadius is set.
        let radii = self.corner_radius.unwrap_or([size.height() / 2.0; 4]);
        let [top_left, top_right, bottom_right, bottom_left] = radii.map(Radius::circular);
        let bounds = Offset::ZERO & size;
        let shape =
            RRect::from_rect_and_corners(bounds, top_left, top_right, bottom_right, bottom_left)
                .scale_radii();
        self.background.paint_rrect(canvas, shape, bounds);
    }

    fn should_repaint(&self, _app: &App, old_delegate: &dyn CustomPainter) -> bool {
        old_delegate.as_any().downcast_ref::<Self>() != Some(self)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// `InfoBadge::MeasureOverride`: a badge never becomes taller than it is wide.
#[derive(Debug)]
struct SquareWhenTallerThanWide {
    key: Option<KeyRef>,
    child: WidgetRef,
}

impl SquareWhenTallerThanWide {
    fn new<K>(child: impl IntoWidget<K>) -> Self {
        Self {
            key: None,
            child: child.into_widget(),
        }
    }
}

impl RenderObjectWidget for SquareWhenTallerThanWide {
    type RenderObject = RenderSquareWhenTallerThanWide;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderHandle::new_box(
            app,
            RenderSquareWhenTallerThanWide {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
            },
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        _app: &mut App,
        _context: BuildContext,
        _render: RenderHandle<RenderSquareWhenTallerThanWide>,
    ) {
    }
}

impl SingleChildRenderObjectWidget for SquareWhenTallerThanWide {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

/// Measures the template once, squares the result up, then arranges it at that size.
struct RenderSquareWhenTallerThanWide {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
}

/// `MeasureOverride`'s rule: a badge narrower than it is tall becomes square.
fn squared(desired: Size) -> Size {
    if desired.width() < desired.height() {
        Size::new(desired.height(), desired.height())
    } else {
        desired
    }
}

impl RenderObjectWithChildMixin for RenderSquareWhenTallerThanWide {
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

impl RenderProxyBoxMixin for RenderSquareWhenTallerThanWide {}

impl RenderObject for RenderSquareWhenTallerThanWide {
    inset_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let Some(child) = self.child(app) else {
            self.set_size(app, constraints.constrain(Size::ZERO));
            return;
        };
        child.layout(app, constraints, true);
        let size = constraints.constrain(squared(child.size(app)));
        if size != child.size(app) {
            child.layout(app, BoxConstraints::tight(size), true);
        }
        self.set_size(app, size);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        RenderProxyBoxMixin::paint(self, app, context, offset);
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

impl RenderBox for RenderSquareWhenTallerThanWide {
    inset_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut Matrix4,
    ) {
        RenderProxyBoxMixin::apply_paint_transform(self, app, child, transform);
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderProxyBoxMixin::hit_test_children(self, app, result, position)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        match self.child(app) {
            Some(child) => constraints.constrain(squared(child.get_dry_layout(app, constraints))),
            None => constraints.constrain(Size::ZERO),
        }
    }
}
