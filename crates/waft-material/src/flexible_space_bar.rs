//! Flutter counterpart: material_ui's `flexible_space_bar.dart`.

use waft_animation::{Curve, Curves, Interval, Tween};
use waft_embedder::{
    ImageFilter, Matrix4, Offset, Size, TargetPlatform, TextBaseline, TextDirection,
};
use waft_foundation::{App, Handle};
use waft_painting::{Alignment, EdgeInsetsGeometry};
use waft_rendering::{
    AnyOffsetLayer, AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult,
    ContainerLayer, OffsetLayerMixin, OpacityLayer, PaintingContext, RenderBox, RenderBoxData,
    RenderHandle, RenderObject, RenderObjectData, RenderObjectWithChildData,
    RenderObjectWithChildMixin, RenderProxyBoxMixin,
};
use waft_widgets::{
    Align, BackdropFilter, BuildContext, ClipRect, ColoredBox, DefaultTextStyle, Directionality,
    InheritedWidget, IntoWidget, KeyRef, LayoutBuilder, Opacity, Padding, Positioned,
    RenderObjectWidget, Semantics, SingleChildRenderObjectWidget, SizedBox, Stack, State,
    StateData, StatefulWidget, Transform, WidgetRef,
};

use crate::colors::Colors;
use crate::constants::K_TOOLBAR_HEIGHT;
use crate::theme::Theme;

/// The collapsing effect while the space bar collapses from its full size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollapseMode {
    /// The background widget will scroll in a parallax fashion.
    Parallax,

    /// The background widget pin in place until it reaches the min extent.
    Pin,

    /// The background widget will act as normal with no collapsing effect.
    None,
}

/// The stretching effect while the space bar stretches beyond its full size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StretchMode {
    /// The background widget will expand to fill the extra space.
    ZoomBackground,

    /// The background will blur using a [`ImageFilter::blur`] effect.
    BlurBackground,

    /// The title will fade away as the user over-scrolls.
    FadeTitle,
}

/// The part of a Material Design `AppBar` that expands, collapses, and
/// stretches.
///
/// Most commonly used in the `SliverAppBar::flexible_space` field, a flexible
/// space bar expands and contracts as the app scrolls so that the `AppBar`
/// reaches from the top of the app to the top of the scrolling contents of the
/// app. When using `SliverAppBar::flexible_space`, the `SliverAppBar::expanded_height`
/// must be large enough to accommodate the `SliverAppBar::flexible_space` widget.
///
/// Furthermore is included functionality for stretch behavior. When
/// `SliverAppBar::stretch` is true, and your `ScrollPhysics` allow for
/// overscroll, this space will stretch with the overscroll.
///
/// The widget that sizes the `AppBar` must wrap it in the widget returned by
/// [`FlexibleSpaceBar::create_settings`], to convey sizing information down to the
/// [`FlexibleSpaceBar`].
///
/// See also:
///
///  * `SliverAppBar`, which implements the expanding and contracting.
///  * `AppBar`, which is used by `SliverAppBar`.
///  * <https://material.io/design/components/app-bars-top.html#behavior>
#[derive(Debug)]
pub struct FlexibleSpaceBar {
    pub key: Option<KeyRef>,

    /// The primary contents of the flexible space bar when expanded.
    ///
    /// Typically a `Text` widget.
    pub title: Option<WidgetRef>,

    /// Shown behind the [`title`](Self::title) when expanded.
    ///
    /// Typically an `Image` widget with `Image::fit` set to `BoxFit::Cover`.
    pub background: Option<WidgetRef>,

    /// Whether the title should be centered.
    ///
    /// If the length of the title is greater than the available space, set
    /// this property to false. This aligns the title to the start of the
    /// flexible space bar and applies [`title_padding`](Self::title_padding) to the title.
    ///
    /// By default this property is true if the current target platform
    /// is [`TargetPlatform::IOS`] or [`TargetPlatform::MacOS`], false otherwise.
    pub center_title: Option<bool>,

    /// Collapse effect while scrolling.
    ///
    /// Defaults to [`CollapseMode::Parallax`].
    pub collapse_mode: CollapseMode,

    /// Stretch effect while over-scrolling.
    ///
    /// Defaults to include [`StretchMode::ZoomBackground`].
    pub stretch_modes: Vec<StretchMode>,

    /// Defines how far the [`title`](Self::title) is inset from either the widget's
    /// bottom-left or its center.
    ///
    /// Typically this property is used to adjust how far the title is
    /// inset from the bottom-left and it is specified along with
    /// [`center_title`](Self::center_title) false.
    ///
    /// If [`center_title`](Self::center_title) is true, then the title is centered within the
    /// flexible space bar with a bottom padding of 16.0 pixels.
    ///
    /// If [`center_title`](Self::center_title) is false and
    /// [`FlexibleSpaceBarSettings::has_leading`] is true,
    /// then the title is aligned to the start of the flexible space bar with the
    /// [`title_padding`](Self::title_padding) applied. If [`title_padding`](Self::title_padding)
    /// is null, then defaults to start padding of 72.0 pixels and bottom padding of 16.0 pixels.
    pub title_padding: Option<EdgeInsetsGeometry>,

    /// Defines how much the title is scaled when the FlexibleSpaceBar is expanded
    /// due to the user scrolling downwards. The title is scaled uniformly on the
    /// x and y axes while maintaining its bottom-left position (bottom-center if
    /// [`center_title`](Self::center_title) is true).
    ///
    /// Defaults to 1.5 and must be greater than 1.
    pub expanded_title_scale: f64,
}

impl FlexibleSpaceBar {
    /// Creates a flexible space bar; Dart's optional named arguments are the setters.
    ///
    /// Most commonly used in the `AppBar::flexible_space` field.
    pub fn new() -> FlexibleSpaceBar {
        FlexibleSpaceBar {
            key: None,
            title: None,
            background: None,
            center_title: None,
            collapse_mode: CollapseMode::Parallax,
            stretch_modes: vec![StretchMode::ZoomBackground],
            title_padding: None,
            expanded_title_scale: 1.5,
        }
    }

    /// Dart `FlexibleSpaceBar(key:)`.
    pub fn key(mut self, key: KeyRef) -> FlexibleSpaceBar {
        self.key = Some(key);
        self
    }

    /// Dart `FlexibleSpaceBar(title:)`.
    pub fn title<K>(mut self, title: impl IntoWidget<K>) -> FlexibleSpaceBar {
        self.title = Some(title.into_widget());
        self
    }

    /// Dart `FlexibleSpaceBar(background:)`.
    pub fn background<K>(mut self, background: impl IntoWidget<K>) -> FlexibleSpaceBar {
        self.background = Some(background.into_widget());
        self
    }

    /// Dart `FlexibleSpaceBar(centerTitle:)`.
    pub fn center_title(mut self, center_title: bool) -> FlexibleSpaceBar {
        self.center_title = Some(center_title);
        self
    }

    /// Dart `FlexibleSpaceBar(titlePadding:)`.
    pub fn title_padding(mut self, title_padding: EdgeInsetsGeometry) -> FlexibleSpaceBar {
        self.title_padding = Some(title_padding);
        self
    }

    /// Dart `FlexibleSpaceBar(collapseMode:)`.
    pub fn collapse_mode(mut self, collapse_mode: CollapseMode) -> FlexibleSpaceBar {
        self.collapse_mode = collapse_mode;
        self
    }

    /// Dart `FlexibleSpaceBar(stretchModes:)`.
    pub fn stretch_modes(mut self, stretch_modes: Vec<StretchMode>) -> FlexibleSpaceBar {
        self.stretch_modes = stretch_modes;
        self
    }

    /// Dart `FlexibleSpaceBar(expandedTitleScale:)`.
    pub fn expanded_title_scale(mut self, expanded_title_scale: f64) -> FlexibleSpaceBar {
        debug_assert!(expanded_title_scale >= 1.0);
        self.expanded_title_scale = expanded_title_scale;
        self
    }

    /// Wraps a widget that contains an `AppBar` to convey sizing information down
    /// to the [`FlexibleSpaceBar`].
    ///
    /// Used by `Scaffold` and `SliverAppBar`.
    ///
    /// `toolbar_opacity` affects how transparent the text within the toolbar
    /// appears. `min_extent` sets the minimum height of the resulting
    /// [`FlexibleSpaceBar`] when fully collapsed. `max_extent` sets the maximum
    /// height of the resulting [`FlexibleSpaceBar`] when fully expanded.
    /// `current_extent` sets the scale of the [`FlexibleSpaceBar::background`] and
    /// [`FlexibleSpaceBar::title`] widgets of [`FlexibleSpaceBar`] upon
    /// initialization. `is_scrolled_under` is true if the [`FlexibleSpaceBar`]
    /// overlaps the app's primary scrollable, false if it does not, and null
    /// if the caller has not determined as much.
    /// See also:
    ///
    ///  * [`FlexibleSpaceBarSettings`] which creates a settings object that can be
    ///    used to specify these settings to a [`FlexibleSpaceBar`].
    ///
    /// Dart's optional named arguments are the setters of the builder; `build` fills
    /// `min_extent` and `max_extent` from `current_extent` and `toolbar_opacity` with 1.0.
    pub fn create_settings<K>(
        current_extent: f64,
        child: impl IntoWidget<K>,
    ) -> FlexibleSpaceBarSettingsBuilder {
        FlexibleSpaceBarSettingsBuilder {
            toolbar_opacity: None,
            min_extent: None,
            max_extent: None,
            is_scrolled_under: None,
            has_leading: None,
            current_extent,
            child: child.into_widget(),
        }
    }
}

impl Default for FlexibleSpaceBar {
    fn default() -> FlexibleSpaceBar {
        FlexibleSpaceBar::new()
    }
}

/// The optional arguments of [`FlexibleSpaceBar::create_settings`].
pub struct FlexibleSpaceBarSettingsBuilder {
    toolbar_opacity: Option<f64>,
    min_extent: Option<f64>,
    max_extent: Option<f64>,
    is_scrolled_under: Option<bool>,
    has_leading: Option<bool>,
    current_extent: f64,
    child: WidgetRef,
}

impl FlexibleSpaceBarSettingsBuilder {
    /// Dart `createSettings(toolbarOpacity:)`.
    pub fn toolbar_opacity(mut self, toolbar_opacity: f64) -> FlexibleSpaceBarSettingsBuilder {
        self.toolbar_opacity = Some(toolbar_opacity);
        self
    }

    /// Dart `createSettings(minExtent:)`.
    pub fn min_extent(mut self, min_extent: f64) -> FlexibleSpaceBarSettingsBuilder {
        self.min_extent = Some(min_extent);
        self
    }

    /// Dart `createSettings(maxExtent:)`.
    pub fn max_extent(mut self, max_extent: f64) -> FlexibleSpaceBarSettingsBuilder {
        self.max_extent = Some(max_extent);
        self
    }

    /// Dart `createSettings(isScrolledUnder:)`.
    pub fn is_scrolled_under(mut self, is_scrolled_under: bool) -> FlexibleSpaceBarSettingsBuilder {
        self.is_scrolled_under = Some(is_scrolled_under);
        self
    }

    /// Dart `createSettings(hasLeading:)`.
    pub fn has_leading(mut self, has_leading: bool) -> FlexibleSpaceBarSettingsBuilder {
        self.has_leading = Some(has_leading);
        self
    }

    /// The settings widget Dart's `createSettings` returns.
    pub fn build(self) -> FlexibleSpaceBarSettings {
        let mut settings = FlexibleSpaceBarSettings::new(
            self.toolbar_opacity.unwrap_or(1.0),
            self.min_extent.unwrap_or(self.current_extent),
            self.max_extent.unwrap_or(self.current_extent),
            self.current_extent,
            self.child,
        );
        settings.is_scrolled_under = self.is_scrolled_under;
        settings.has_leading = self.has_leading;
        settings
    }
}

impl StatefulWidget for FlexibleSpaceBar {
    type State = FlexibleSpaceBarState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> FlexibleSpaceBarState {
        FlexibleSpaceBarState {
            state: StateData::new(),
        }
    }
}

/// Dart's `_FlexibleSpaceBarState`.
pub struct FlexibleSpaceBarState {
    state: StateData<FlexibleSpaceBar>,
}

impl FlexibleSpaceBarState {
    fn get_effective_center_title(self: Handle<Self>, app: &App, platform: TargetPlatform) -> bool {
        self.widget(app).center_title.unwrap_or(match platform {
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => false,
            TargetPlatform::IOS | TargetPlatform::MacOS => true,
        })
    }

    fn get_title_alignment(
        self: Handle<Self>,
        app: &mut App,
        effective_center_title: bool,
    ) -> Alignment {
        if effective_center_title {
            return Alignment::BOTTOM_CENTER;
        }
        let context = self.context(app);
        match Directionality::of(app, context) {
            TextDirection::Rtl => Alignment::BOTTOM_RIGHT,
            TextDirection::Ltr => Alignment::BOTTOM_LEFT,
        }
    }

    fn get_collapse_padding(
        self: Handle<Self>,
        app: &mut App,
        t: f64,
        settings: &FlexibleSpaceBarSettings,
    ) -> f64 {
        match self.widget(app).collapse_mode {
            CollapseMode::Pin => -(settings.max_extent - settings.current_extent),
            CollapseMode::None => 0.0,
            CollapseMode::Parallax => {
                let delta_extent = settings.max_extent - settings.min_extent;
                -transform_tween(app, 0.0, delta_extent / 4.0, t)
            }
        }
    }
}

/// Dart's `Tween<double>(begin: begin, end: end).transform(t)`.
fn transform_tween(app: &mut App, begin: f64, end: f64, t: f64) -> f64 {
    let tween = Tween::new(app, Some(begin), Some(end));
    waft_animation::Animatable::transform(&tween, app, t)
}

impl State for FlexibleSpaceBarState {
    type Widget = FlexibleSpaceBar;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, _app: &mut App, _context: BuildContext) -> WidgetRef {
        LayoutBuilder::new(move |app, context, constraints| {
            self.build_with_constraints(app, context, constraints.constraints())
        })
        .into_widget()
    }
}

impl FlexibleSpaceBarState {
    /// The builder of the `LayoutBuilder` Dart's `build` returns.
    fn build_with_constraints(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        constraints: BoxConstraints,
    ) -> WidgetRef {
        let settings = context
            .depend_on_inherited_widget_of_exact_type::<FlexibleSpaceBarSettings>(app)
            .expect("a FlexibleSpaceBarSettings above the FlexibleSpaceBar")
            .clone();

        let mut children: Vec<WidgetRef> = Vec::new();

        let delta_extent = settings.max_extent - settings.min_extent;

        // 0.0 -> Expanded
        // 1.0 -> Collapsed to toolbar
        let t = waft_embedder::clamp_double(
            1.0 - (settings.current_extent - settings.min_extent) / delta_extent,
            0.0,
            1.0,
        );

        // background
        if let Some(background) = self.widget(app).background.clone() {
            let fade_start = f64::max(0.0, 1.0 - K_TOOLBAR_HEIGHT / delta_extent);
            const FADE_END: f64 = 1.0;
            debug_assert!(fade_start <= FADE_END);
            // If the min and max extent are the same, the app bar cannot collapse
            // and the content should be visible, so opacity = 1.
            let opacity = if settings.max_extent == settings.min_extent {
                1.0
            } else {
                1.0 - Interval::new(fade_start, FADE_END, Curves::linear()).transform(t)
            };
            let mut height = settings.max_extent;

            // StretchMode.zoomBackground
            let stretch_modes = self.widget(app).stretch_modes.clone();
            if stretch_modes.contains(&StretchMode::ZoomBackground)
                && constraints.max_height > height
            {
                height = constraints.max_height;
            }
            let top_padding = self.get_collapse_padding(app, t, &settings);
            children.push(
                Positioned::new(FlexibleSpaceHeaderOpacity::new(opacity, background))
                    .top(top_padding)
                    .left(0.0)
                    .right(0.0)
                    .height(height)
                    .into_widget(),
            );

            // StretchMode.blurBackground
            if stretch_modes.contains(&StretchMode::BlurBackground)
                && constraints.max_height > settings.max_extent
            {
                let blur_amount = (constraints.max_height - settings.max_extent) / 10.0;
                children.push(
                    Positioned::fill(
                        BackdropFilter::filter(ImageFilter::blur(blur_amount, blur_amount))
                            .child(ColoredBox::new(Colors::TRANSPARENT)),
                    )
                    .into_widget(),
                );
            }
        }

        // title
        if let Some(title) = self.widget(app).title.clone() {
            let theme = Theme::of(app, context);

            let mut title = match theme.platform {
                TargetPlatform::IOS | TargetPlatform::MacOS => title,
                TargetPlatform::Android
                | TargetPlatform::Fuchsia
                | TargetPlatform::Linux
                | TargetPlatform::Windows => Semantics::new()
                    .names_route(true)
                    .child(title)
                    .into_widget(),
            };

            // StretchMode.fadeTitle
            if self
                .widget(app)
                .stretch_modes
                .contains(&StretchMode::FadeTitle)
                && constraints.max_height > settings.max_extent
            {
                let stretch_opacity = 1.0
                    - waft_embedder::clamp_double(
                        (constraints.max_height - settings.max_extent) / 100.0,
                        0.0,
                        1.0,
                    );
                title = Opacity::new(stretch_opacity).child(title).into_widget();
            }

            let opacity = settings.toolbar_opacity;
            if opacity > 0.0 {
                let mut title_style = if theme.use_material3 {
                    theme.text_theme.title_large.clone()
                } else {
                    theme.primary_text_theme.title_large.clone()
                }
                .expect("the text theme has a title large style");
                #[allow(deprecated)]
                let title_color = title_style
                    .color
                    .as_ref()
                    .expect("the title large style has a color")
                    .with_opacity(opacity);
                title_style = title_style.copy_with().color(title_color);
                let effective_center_title = self.get_effective_center_title(app, theme.platform);
                let leading_padding = if settings.has_leading.unwrap_or(true) {
                    72.0
                } else {
                    0.0
                };
                let padding =
                    self.widget(app)
                        .title_padding
                        .unwrap_or(EdgeInsetsGeometry::directional(
                            if effective_center_title {
                                0.0
                            } else {
                                leading_padding
                            },
                            0.0,
                            0.0,
                            16.0,
                        ));
                let expanded_title_scale = self.widget(app).expanded_title_scale;
                let scale_value = transform_tween(app, expanded_title_scale, 1.0, t);
                let scale_transform = Matrix4::scale(scale_value as f32, scale_value as f32);
                let title_alignment = self.get_title_alignment(app, effective_center_title);
                children.push(
                    Padding::new(padding)
                        .child(
                            Transform::new(scale_transform)
                                .alignment(title_alignment.into())
                                .child(
                                    Align::new().alignment(title_alignment.into()).child(
                                        DefaultTextStyle::new(
                                            title_style,
                                            LayoutBuilder::new(
                                                move |_app, _context, constraints| {
                                                    SizedBox::new()
                                                        .width(constraints.max_width / scale_value)
                                                        .child(
                                                            Align::new()
                                                                .alignment(title_alignment.into())
                                                                .child(title.clone()),
                                                        )
                                                        .into_widget()
                                                },
                                            )
                                            .into_widget(),
                                        ),
                                    ),
                                ),
                        )
                        .into_widget(),
                );
            }
        }

        ClipRect::new()
            .child(Stack::new().children(children))
            .into_widget()
    }
}

/// Provides sizing and opacity information to a [`FlexibleSpaceBar`].
///
/// See also:
///
///  * [`FlexibleSpaceBar`] which creates a flexible space bar.
#[derive(Clone, Debug)]
pub struct FlexibleSpaceBarSettings {
    pub key: Option<KeyRef>,

    /// Affects how transparent the text within the toolbar appears.
    pub toolbar_opacity: f64,

    /// Minimum height of the resulting [`FlexibleSpaceBar`] when fully collapsed.
    pub min_extent: f64,

    /// Maximum height of the resulting [`FlexibleSpaceBar`] when fully expanded.
    pub max_extent: f64,

    /// If the [`FlexibleSpaceBar::title`] or the [`FlexibleSpaceBar::background`] is
    /// not null, then this value is used to calculate the relative scale of
    /// these elements upon initialization.
    pub current_extent: f64,

    /// True if the FlexibleSpaceBar overlaps the primary scrollable's contents.
    ///
    /// This value is used by the `AppBar` to resolve
    /// `AppBar::background_color` against `WidgetState::ScrolledUnder`,
    /// i.e. to enable apps to specify different colors when content
    /// has been scrolled up and behind the app bar.
    ///
    /// Null if the caller hasn't determined if the FlexibleSpaceBar
    /// overlaps the primary scrollable's contents.
    pub is_scrolled_under: Option<bool>,

    /// True if the FlexibleSpaceBar has a leading widget.
    ///
    /// This value is used by the [`FlexibleSpaceBar`] to determine
    /// if there should be a gap between the leading widget and
    /// the title.
    ///
    /// Null if the caller hasn't determined if the FlexibleSpaceBar
    /// has a leading widget.
    pub has_leading: Option<bool>,

    pub child: WidgetRef,
}

impl FlexibleSpaceBarSettings {
    /// Creates a Flexible Space Bar Settings widget.
    ///
    /// Used by `Scaffold` and `SliverAppBar`. `child` must have a
    /// [`FlexibleSpaceBar`] widget in its tree for the settings to take affect.
    pub fn new<K>(
        toolbar_opacity: f64,
        min_extent: f64,
        max_extent: f64,
        current_extent: f64,
        child: impl IntoWidget<K>,
    ) -> FlexibleSpaceBarSettings {
        debug_assert!(min_extent >= 0.0);
        debug_assert!(max_extent >= 0.0);
        debug_assert!(current_extent >= 0.0);
        debug_assert!(toolbar_opacity >= 0.0);
        debug_assert!(min_extent <= max_extent);
        debug_assert!(min_extent <= current_extent);
        debug_assert!(current_extent <= max_extent);
        FlexibleSpaceBarSettings {
            key: None,
            toolbar_opacity,
            min_extent,
            max_extent,
            current_extent,
            is_scrolled_under: None,
            has_leading: None,
            child: child.into_widget(),
        }
    }

    /// Dart `FlexibleSpaceBarSettings(key:)`.
    pub fn key(mut self, key: KeyRef) -> FlexibleSpaceBarSettings {
        self.key = Some(key);
        self
    }

    /// Dart `FlexibleSpaceBarSettings(isScrolledUnder:)`.
    pub fn is_scrolled_under(mut self, is_scrolled_under: bool) -> FlexibleSpaceBarSettings {
        self.is_scrolled_under = Some(is_scrolled_under);
        self
    }

    /// Dart `FlexibleSpaceBarSettings(hasLeading:)`.
    pub fn has_leading(mut self, has_leading: bool) -> FlexibleSpaceBarSettings {
        self.has_leading = Some(has_leading);
        self
    }
}

impl InheritedWidget for FlexibleSpaceBarSettings {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &FlexibleSpaceBarSettings) -> bool {
        self.toolbar_opacity != old_widget.toolbar_opacity
            || self.min_extent != old_widget.min_extent
            || self.max_extent != old_widget.max_extent
            || self.current_extent != old_widget.current_extent
            || self.is_scrolled_under != old_widget.is_scrolled_under
            || self.has_leading != old_widget.has_leading
    }
}

// We need the child widget to repaint, however both the opacity
// and potentially `widget.background` can be constant which won't
// lead to repainting.
// see: https://github.com/flutter/flutter/issues/127836
/// Dart's `_FlexibleSpaceHeaderOpacity`.
#[derive(Debug)]
pub(crate) struct FlexibleSpaceHeaderOpacity {
    opacity: f64,
    child: WidgetRef,
}

impl FlexibleSpaceHeaderOpacity {
    fn new<K>(opacity: f64, child: impl IntoWidget<K>) -> FlexibleSpaceHeaderOpacity {
        FlexibleSpaceHeaderOpacity {
            opacity,
            child: child.into_widget(),
        }
    }
}

impl RenderObjectWidget for FlexibleSpaceHeaderOpacity {
    type RenderObject = RenderFlexibleSpaceHeaderOpacity;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderFlexibleSpaceHeaderOpacity::new(app, self.opacity).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderFlexibleSpaceHeaderOpacity>,
    ) {
        render_object.set_opacity(app, self.opacity);
    }
}

impl SingleChildRenderObjectWidget for FlexibleSpaceHeaderOpacity {
    fn child(&self) -> Option<&WidgetRef> {
        Some(&self.child)
    }
}

/// Dart's `_RenderFlexibleSpaceHeaderOpacity`: a `RenderOpacity` that is not a repaint
/// boundary and keeps its own `OpacityLayer` in `layer`.
pub(crate) struct RenderFlexibleSpaceHeaderOpacity {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    alpha: i32,
    opacity: f64,
}

impl RenderFlexibleSpaceHeaderOpacity {
    fn new(app: &mut App, opacity: f64) -> RenderHandle<Self> {
        debug_assert!((0.0..=1.0).contains(&opacity));
        RenderHandle::new_box(
            app,
            RenderFlexibleSpaceHeaderOpacity {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                alpha: waft_embedder::Color::get_alpha_from_opacity(opacity),
                opacity,
            },
        )
    }

    /// Sets the fraction to scale the child's alpha value.
    fn set_opacity(self: RenderHandle<Self>, app: &mut App, value: f64) {
        debug_assert!((0.0..=1.0).contains(&value));
        if self.get(app).opacity == value {
            return;
        }
        let did_need_compositing = self.always_needs_compositing(app);
        let this = self.get_mut(app);
        this.opacity = value;
        this.alpha = waft_embedder::Color::get_alpha_from_opacity(value);
        if did_need_compositing != self.always_needs_compositing(app) {
            self.mark_needs_compositing_bits_update(app);
        }
        self.mark_needs_composited_layer_update(app);
    }
}

impl RenderObjectWithChildMixin for RenderFlexibleSpaceHeaderOpacity {
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

impl RenderProxyBoxMixin for RenderFlexibleSpaceHeaderOpacity {}

impl RenderObject for RenderFlexibleSpaceHeaderOpacity {
    waft_rendering::render_object_accessors!();

    fn always_needs_compositing(self: RenderHandle<Self>, app: &App) -> bool {
        self.child(app).is_some() && self.get(app).alpha > 0
    }

    fn is_repaint_boundary(self: RenderHandle<Self>, _app: &App) -> bool {
        false
    }

    fn update_composited_layer(
        self: RenderHandle<Self>,
        app: &mut App,
        old_layer: Option<AnyOffsetLayer>,
    ) -> AnyOffsetLayer {
        let layer = match old_layer {
            Some(old) => app
                .handle::<OpacityLayer>(old.id())
                .expect("RenderOpacity reuses an OpacityLayer"),
            None => OpacityLayer::new(app),
        };
        layer.set_alpha(app, self.get(app).alpha);
        layer.as_offset_layer()
    }

    fn paints_child(self: RenderHandle<Self>, app: &App, child: AnyRenderObject) -> bool {
        debug_assert!(child.parent(app) == Some(self.as_object()));
        self.get(app).alpha > 0
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if self.child(app).is_none() {
            return;
        }
        let opacity = self.get(app).opacity;
        if (opacity * 255.0).round() <= 0.0 {
            self.as_object().set_layer(app, None);
            return;
        }
        debug_assert!(self.as_object().needs_compositing(app));
        let old_layer = self.as_object().layer_as::<OpacityLayer>(app);
        let layer = context.push_opacity(
            app,
            offset,
            (opacity * 255.0).round() as i32,
            |app, context, offset| {
                // `RenderOpacity.paint`, which `super.paint` reaches.
                if self.child(app).is_none() || self.get(app).alpha == 0 {
                    return;
                }
                RenderProxyBoxMixin::paint(self, app, context, offset);
            },
            old_layer,
        );
        self.as_object()
            .set_layer(app, Some(layer.as_container_layer()));
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

impl RenderBox for RenderFlexibleSpaceHeaderOpacity {
    waft_rendering::render_box_accessors!();

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

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderProxyBoxMixin::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderProxyBoxMixin::compute_dry_layout(self, app, constraints)
    }
}

#[cfg(test)]
mod tests {
    //! Cases of `test/flexible_space_bar_test.dart`, `flexible_space_bar_collapse_mode_test.dart`
    //! and `flexible_space_bar_stretch_mode_test.dart`. Where a Dart case drags or stretches a
    //! `SliverAppBar` in a scroll view, or measures FlexibleSpaceBar-in-`AppBar` text, the
    //! settings the app bar would convey at that point are given by hand with
    //! [`FlexibleSpaceBar::create_settings`], in a box of the height the app bar would be.
    //! Each case's name says which Dart case it follows.

    use std::cell::RefCell;
    use std::rc::Rc;

    use waft_embedder::Rect;
    use waft_painting::{AlignmentGeometry, TextStyle};
    use waft_rendering::OpacityLayer;
    use waft_widgets::{Builder, Center, Container, GlobalKey, Text, TextOverflow};

    use waft_widgets::testing::{
        TargetPlatformVariant, WidgetTester, test_widgets, test_widgets_variant,
    };

    use super::*;
    use crate::app_bar::{AppBar, SliverAppBar};
    use crate::material::Material;
    use crate::scaffold::Scaffold;
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m3, pump_app};

    /// What `FlexibleSpaceBar.createSettings` is given by an app bar.
    #[derive(Clone, Copy)]
    struct Extents {
        current: f64,
        min: f64,
        max: f64,
    }

    impl Extents {
        /// An app bar that cannot collapse: `AppBar` conveys one extent for all three.
        fn fixed(extent: f64) -> Extents {
            Extents {
                current: extent,
                min: extent,
                max: extent,
            }
        }
    }

    /// A box as wide as the test view and `height` high at the top left, holding `bar` under
    /// settings of `extents`.
    fn bar_in_box(extents: Extents, height: f64, bar: FlexibleSpaceBar) -> WidgetRef {
        let settings = FlexibleSpaceBar::create_settings(extents.current, bar)
            .min_extent(extents.min)
            .max_extent(extents.max)
            .build();
        Align::new()
            .alignment(AlignmentGeometry::TOP_LEFT)
            .child(SizedBox::new().width(800.0).height(height).child(settings))
            .into_widget()
    }

    fn pump_bar(
        tester: &mut WidgetTester,
        platform: TargetPlatform,
        extents: Extents,
        height: f64,
        bar: FlexibleSpaceBar,
    ) {
        pump_app(
            tester,
            move |app| ThemeData::builder().platform(platform).build(app),
            move |_app| bar_in_box(extents, height, bar),
        );
    }

    /// A `Text::new("X")` that tests can find.
    fn keyed_title(key: &GlobalKey) -> SizedBox {
        SizedBox::new()
            .key(key.clone().into())
            .child(Text::new("X"))
    }

    /// The global rectangle of `key`'s widget.
    fn rect_of_key(tester: &WidgetTester, key: &GlobalKey) -> Rect {
        tester.global_rect(tester.render_object(key).as_box().expect("a render box"))
    }

    /// Dart's `getTitleBottomLeft`: the title's left, and the distance from its bottom to the
    /// bottom of the space bar.
    fn title_bottom_left(tester: &WidgetTester, key: &GlobalKey) -> Offset {
        let title = rect_of_key(tester, key);
        let bar = *tester
            .find_by_type::<FlexibleSpaceBar>()
            .first()
            .expect("a FlexibleSpaceBar");
        let bar = tester.global_rect(tester.render_box_of(bar));
        Offset::new(title.left, bar.bottom - title.bottom)
    }

    fn title_bottom_left_case(
        tester: &mut WidgetTester,
        platform: TargetPlatform,
        center_title: Option<bool>,
        title_padding: Option<EdgeInsetsGeometry>,
    ) -> Offset {
        let key = GlobalKey::new();
        let mut bar = FlexibleSpaceBar::new().title(keyed_title(&key));
        if let Some(center_title) = center_title {
            bar = bar.center_title(center_title);
        }
        if let Some(title_padding) = title_padding {
            bar = bar.title_padding(title_padding);
        }
        pump_bar(tester, platform, Extents::fixed(56.0), 56.0, bar);
        title_bottom_left(tester, &key)
    }

    /// Dart's `pumpWidget(Container(key: UniqueKey()))`: clears the widget tree to avoid
    /// animating between platforms.
    fn clear_tree(tester: &mut WidgetTester) {
        tester.pump_widget(
            Container::new()
                .key(waft_foundation::UniqueKey::new().into())
                .into_widget(),
        );
    }

    #[test]
    fn flexible_space_bar_centers_title_on_ios() {
        test_widgets(|tester| {
            let pump = |tester: &mut WidgetTester, platform: TargetPlatform| {
                pump_app(
                    tester,
                    move |app| ThemeData::builder().platform(platform).build(app),
                    |_app| {
                        Scaffold::new()
                            .app_bar(waft_widgets::PreferredSizeWidgetRef::new(
                                AppBar::new().flexible_space(
                                    FlexibleSpaceBar::new().title(waft_widgets::Text::new("X")),
                                ),
                            ))
                            .into_widget()
                    },
                );
            };
            let title = |tester: &WidgetTester| {
                tester.global_rect(tester.render_box_of(tester.find_text("X")[0]))
            };
            pump(tester, TargetPlatform::Android);
            let rect = title(tester);
            assert!(rect.center().dx() < 400.0 - rect.width() / 2.0);

            for platform in [TargetPlatform::IOS, TargetPlatform::MacOS] {
                // Clear the widget tree to avoid animating between platforms.
                tester.pump_widget(
                    Container::new()
                        .key(waft_foundation::UniqueKey::new().into())
                        .into_widget(),
                );

                pump(tester, platform);
                let rect = title(tester);
                assert!(rect.center().dx() > 400.0 - rect.width() / 2.0);
                assert!(rect.center().dx() < 400.0 + rect.width() / 2.0);
            }
        });
    }

    #[test]
    fn material3_flexible_space_bar_title_padding_defaults() {
        test_widgets(|tester| {
            let left = Offset::new(72.0, 16.0);
            let center = Offset::new(389.0, 16.0);
            let cases = [
                (TargetPlatform::Android, None, left),
                (TargetPlatform::Android, Some(true), center),
                (TargetPlatform::IOS, None, center),
                (TargetPlatform::IOS, Some(false), left),
                (TargetPlatform::MacOS, None, center),
                (TargetPlatform::MacOS, Some(false), left),
            ];
            let mut previous = None;
            for (platform, center_title, expected) in cases {
                if previous.is_some() && previous != Some(platform) {
                    clear_tree(tester);
                }
                previous = Some(platform);
                assert_eq!(
                    title_bottom_left_case(tester, platform, center_title, None),
                    expected,
                    "{platform:?} with centerTitle {center_title:?}"
                );
            }
        });
    }

    #[test]
    fn material3_flexible_space_bar_title_padding_override() {
        test_widgets(|tester| {
            let left = Offset::ZERO;
            let center = Offset::new(389.0, 0.0);
            let cases = [
                (TargetPlatform::Android, None, left),
                (TargetPlatform::Android, Some(true), center),
                (TargetPlatform::IOS, None, center),
                (TargetPlatform::IOS, Some(false), left),
                (TargetPlatform::MacOS, None, center),
                (TargetPlatform::MacOS, Some(false), left),
                (TargetPlatform::Windows, None, left),
                (TargetPlatform::Windows, Some(true), center),
                (TargetPlatform::Linux, None, left),
                (TargetPlatform::Linux, Some(true), center),
            ];
            let mut previous = None;
            for (platform, center_title, expected) in cases {
                if previous.is_some() && previous != Some(platform) {
                    clear_tree(tester);
                }
                previous = Some(platform);
                assert_eq!(
                    title_bottom_left_case(
                        tester,
                        platform,
                        center_title,
                        Some(EdgeInsetsGeometry::ZERO)
                    ),
                    expected,
                    "{platform:?} with centerTitle {center_title:?}"
                );
            }
        });
    }

    /// Dart's `TestDelegate`: a header whose extents are the settings' and that builds them.
    #[derive(Debug)]
    struct TestDelegate {
        settings: FlexibleSpaceBarSettings,
    }

    impl waft_widgets::SliverPersistentHeaderDelegate for TestDelegate {
        fn build(
            &self,
            _app: &mut App,
            _context: BuildContext,
            _shrink_offset: f64,
            _overlaps_content: bool,
        ) -> WidgetRef {
            self.settings.clone().into_widget()
        }

        fn min_extent(&self) -> f64 {
            self.settings.min_extent
        }

        fn max_extent(&self) -> f64 {
            self.settings.max_extent
        }

        fn should_rebuild(
            &self,
            _old_delegate: &dyn waft_widgets::SliverPersistentHeaderDelegate,
        ) -> bool {
            false
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    // This is a regression test for https://github.com/flutter/flutter/issues/132030.
    #[test]
    fn flexible_space_bar_settings_has_leading_provides_a_gap_between_leading_and_title() {
        test_widgets(|tester| {
            let custom_settings = FlexibleSpaceBar::create_settings(
                200.0,
                AppBar::new()
                    .leading(waft_widgets::Icon::new(Some(crate::icons::Icons::menu())))
                    .flexible_space(
                        FlexibleSpaceBar::new()
                            .title(waft_widgets::Text::new("title ".repeat(10)))
                            // Set centerTitle to false to create a gap between the leading widget
                            // and the long title.
                            .center_title(false),
                    ),
            )
            .has_leading(true)
            .build();

            pump_app(tester, m3, move |_app| {
                Scaffold::new()
                    .body(
                        waft_widgets::CustomScrollView::new().slivers([
                            waft_widgets::SliverPersistentHeader::new(Rc::new(TestDelegate {
                                settings: custom_settings,
                            }))
                            .floating(true)
                            .pinned(true)
                            .into_widget(),
                            waft_widgets::SliverToBoxAdapter::new()
                                .child(
                                    Container::new()
                                        .height(1200.0)
                                        .color(crate::colors::Colors::ORANGE.shade400()),
                                )
                                .into_widget(),
                        ]),
                    )
                    .into_widget()
            });

            tester.pump_and_settle();
            let text = tester.find_by_type::<waft_widgets::Text>()[0];
            let left = tester.global_rect(tester.render_box_of(text)).left;
            assert!((left - 72.0).abs() < 0.01, "{left}");
        });
    }

    #[test]
    fn flexible_space_bar_settings_without_a_leading_widget_has_no_gap() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let settings = FlexibleSpaceBar::create_settings(
                200.0,
                FlexibleSpaceBar::new()
                    .title(keyed_title(&key))
                    .center_title(false),
            )
            .has_leading(false)
            .build();
            pump_app(tester, m3, move |_app| settings.into_widget());
            assert_eq!(rect_of_key(tester, &key).left, 0.0);
        });
    }

    /// Dart's `Material3 - FlexibleSpaceBarSettings provides settings to a FlexibleSpaceBar`:
    /// the title's scale at half way between the extents, and its text colour at the toolbar
    /// opacity.
    #[test]
    fn material3_flexible_space_bar_settings_provides_settings_to_a_flexible_space_bar() {
        test_widgets(|tester| {
            let style = Rc::new(RefCell::new(None));
            let title = Builder::new({
                let style = Rc::clone(&style);
                move |app, context| {
                    *style.borrow_mut() = Some(DefaultTextStyle::of(app, context).style.clone());
                    SizedBox::shrink().into_widget()
                }
            });
            let settings = FlexibleSpaceBar::create_settings(
                200.0,
                FlexibleSpaceBar::new()
                    .title(title)
                    .background(SizedBox::shrink())
                    .collapse_mode(CollapseMode::Pin),
            )
            .min_extent(100.0)
            .max_extent(300.0)
            .toolbar_opacity(0.5)
            .build();
            pump_app(tester, m3, move |_app| {
                Align::new()
                    .alignment(AlignmentGeometry::TOP_LEFT)
                    .child(SizedBox::new().width(800.0).height(300.0).child(settings))
                    .into_widget()
            });

            // The current (200) is half way between the min (100) and max (300) and the
            // lerp values used to calculate the scale are 1 and 1.5, so we check for 1.25.
            let transform = tester.find_by_type::<Transform>();
            let transform = *transform.first().expect("the title's Transform");
            let scale = tester.widget(transform, |transform: &Transform| transform.transform);
            assert_eq!(scale, Matrix4::scale(1.25, 1.25));

            // The space bar rect always starts fully expanded.
            let clip_rect = *tester
                .find_by_type::<ClipRect>()
                .last()
                .expect("the space bar's ClipRect");
            let clip_size = tester.render_box_of(clip_rect).size(&tester.cell.borrow());
            assert_eq!(clip_size.height(), 300.0);

            let style = style.borrow().clone().expect("the title was built");
            #[allow(deprecated)]
            let alpha = style.color.expect("a title colour").alpha();
            assert_eq!(alpha, 128); // Which is alpha of .5
        });
    }

    /// Dart's `tester.getRect(finder)`: `element`'s box with both corners mapped to the view,
    /// so a scale above it scales the rectangle.
    fn transformed_rect(tester: &WidgetTester, element: waft_widgets::AnyElement) -> Rect {
        let render = tester.render_box_of(element);
        let app = tester.cell.borrow();
        let size = render.size(&app);
        let top_left = render.local_to_global(&app, Offset::ZERO, None);
        let bottom_right =
            render.local_to_global(&app, Offset::new(size.width(), size.height()), None);
        Rect::from_ltrb(
            top_left.dx(),
            top_left.dy(),
            bottom_right.dx(),
            bottom_right.dy(),
        )
    }

    const TITLE_FONT_SIZE: f64 = 20.0;
    const EXPANDED_HEIGHT: f64 = 300.0;

    /// The title of the width-constraint cases: Dart's `Text('X' * count, maxLines: 1,
    /// overflow: TextOverflow.ellipsis, style: style)`, which the box's constraints cut to
    /// one line as wide as they allow.
    fn constrained_title(count: usize, style: TextStyle) -> WidgetRef {
        Text::new("X".repeat(count))
            .max_lines(1)
            .overflow(TextOverflow::Ellipsis)
            .style(style)
            .into_widget()
    }

    /// The `SliverAppBar(expandedHeight: 300, pinned: true, stretch: true, flexibleSpace:
    /// FlexibleSpaceBar(titlePadding: EdgeInsets.zero, title: title, centerTitle: false))` of
    /// the width-constraint cases.
    fn constrained_title_app_bar(
        title: WidgetRef,
        expanded_title_scale: Option<f64>,
    ) -> SliverAppBar {
        let mut bar = FlexibleSpaceBar::new()
            .title_padding(EdgeInsetsGeometry::ZERO)
            .title(title)
            .center_title(false);
        if let Some(expanded_title_scale) = expanded_title_scale {
            bar = bar.expanded_title_scale(expanded_title_scale);
        }
        SliverAppBar::new()
            .expanded_height(EXPANDED_HEIGHT)
            .pinned(true)
            .stretch(true)
            .flexible_space(bar)
    }

    /// Dart's 'FlexibleSpaceBar sets width constraints for the title' under `theme`, its title
    /// styled `style`: the global rectangle of the title.
    fn width_constrained_title_rect(
        tester: &mut WidgetTester,
        theme: fn(&App) -> ThemeData,
        style: TextStyle,
    ) -> Rect {
        let width = Rc::new(std::cell::Cell::new(0.0));
        let seen_width = Rc::clone(&width);
        pump_app(tester, theme, move |_app| {
            Scaffold::new()
                .body(Builder::new(move |app, context| {
                    seen_width.set(waft_widgets::MediaQuery::size_of(app, context).width());
                    waft_widgets::CustomScrollView::new()
                        .slivers([constrained_title_app_bar(
                            constrained_title(2000, style.clone()),
                            None,
                        )
                        .into_widget()])
                        .into_widget()
                }))
                .into_widget()
        });
        let text_width = width.get();
        assert_eq!(text_width, 800.0);
        let title = tester.find_by_type::<Text>()[0];
        transformed_rect(tester, title)
    }

    fn assert_rect_more_or_less_equals(actual: Rect, expected: Rect) {
        const EPSILON: f64 = 0.0001;
        assert!(
            (actual.left - expected.left).abs() <= EPSILON
                && (actual.top - expected.top).abs() <= EPSILON
                && (actual.right - expected.right).abs() <= EPSILON
                && (actual.bottom - expected.bottom).abs() <= EPSILON,
            "{actual:?} is not {expected:?}"
        );
    }

    #[test]
    fn material3_flexible_space_bar_sets_width_constraints_for_the_title() {
        test_widgets(|tester| {
            let rect = width_constrained_title_rect(
                tester,
                m3,
                TextStyle::new().font_size(TITLE_FONT_SIZE).height(1.0),
            );
            // The title is scaled and transformed to be 1.5 times bigger, when the
            // FlexibleSpaceBar is fully expanded, thus we expect the width to be
            // 1.5 times smaller than the full width. The height of the text is the same
            // as the font size, with 10 dps bottom margin.
            assert_rect_more_or_less_equals(
                rect,
                Rect::from_ltrb(
                    0.0,
                    EXPANDED_HEIGHT - TITLE_FONT_SIZE - 10.0,
                    800.0,
                    EXPANDED_HEIGHT,
                ),
            );
        });
    }

    // This is a regression test for https://github.com/flutter/flutter/issues/14227
    #[test]
    fn material2_flexible_space_bar_sets_width_constraints_for_the_title() {
        test_widgets(|tester| {
            let rect = width_constrained_title_rect(
                tester,
                crate::tooltip::tests::m2,
                TextStyle::new().font_size(TITLE_FONT_SIZE),
            );
            assert_rect_more_or_less_equals(
                rect,
                Rect::from_ltrb(
                    0.0,
                    EXPANDED_HEIGHT - TITLE_FONT_SIZE - 10.0,
                    800.0,
                    EXPANDED_HEIGHT,
                ),
            );
        });
    }

    /// Dart's 'FlexibleSpaceBar sets constraints for the title - override expandedTitleScale'
    /// under `theme`, its title styled `style`.
    fn expanded_title_scale_case(
        tester: &mut WidgetTester,
        theme: fn(&App) -> ThemeData,
        style: TextStyle,
    ) {
        const EXPANDED_TITLE_SCALE: f64 = 3.0;
        pump_app(tester, theme, move |_app| {
            Scaffold::new()
                .body(
                    waft_widgets::CustomScrollView::new().slivers([
                        constrained_title_app_bar(
                            constrained_title(41, style.clone()),
                            Some(EXPANDED_TITLE_SCALE),
                        )
                        .into_widget(),
                        waft_widgets::SliverList::builder(|_app, _context, index| {
                            Some(
                                SizedBox::new()
                                    .height(200.0)
                                    .child(
                                        Center::new().child(waft_widgets::Text::new(format!(
                                            "Item {index}"
                                        ))),
                                    )
                                    .into_widget(),
                            )
                        })
                        .item_count(3)
                        .into_widget(),
                    ]),
                )
                .into_widget()
        });
        let center_of_text = |tester: &WidgetTester, text: &str| {
            let element = tester.find_text(text)[0];
            tester.global_rect(tester.render_box_of(element)).center()
        };

        // We drag up to fully collapse the space bar.
        let start = center_of_text(tester, "Item 0");
        tester.drag_from(start, Offset::new(0.0, -600.0), 1);
        tester.pump_and_settle();

        let title = |tester: &WidgetTester| tester.find_by_type::<Text>()[0];
        let collapsed_width = transformed_rect(tester, title(tester)).width();

        // We drag down to fully expand the space bar.
        let start = center_of_text(tester, "Item 2");
        tester.drag_from(start, Offset::new(0.0, 600.0), 2);
        tester.pump_and_settle();

        // The title is shifted by this margin to maintain the position of the
        // bottom edge.
        let bottom_margin = TITLE_FONT_SIZE * (EXPANDED_TITLE_SCALE - 1.0);

        let text_width = collapsed_width;
        // The title is scaled and transformed to be 3 times bigger, when the
        // FlexibleSpaceBar is fully expanded, thus we expect the width to be
        // 3 times smaller than the full width. The height of the text is the same
        // as the font size, with 40 dps bottom margin to maintain its bottom position.
        assert_rect_more_or_less_equals(
            transformed_rect(tester, title(tester)),
            Rect::from_ltrb(
                0.0,
                EXPANDED_HEIGHT - TITLE_FONT_SIZE - bottom_margin,
                text_width,
                EXPANDED_HEIGHT,
            ),
        );
    }

    #[test]
    fn material3_flexible_space_bar_sets_constraints_for_the_title_override_expanded_title_scale() {
        test_widgets(|tester| {
            expanded_title_scale_case(
                tester,
                m3,
                TextStyle::new().font_size(TITLE_FONT_SIZE).height(1.0),
            );
        });
    }

    #[test]
    fn material2_flexible_space_bar_sets_constraints_for_the_title_override_expanded_title_scale() {
        test_widgets(|tester| {
            expanded_title_scale_case(
                tester,
                crate::tooltip::tests::m2,
                TextStyle::new().font_size(TITLE_FONT_SIZE),
            );
        });
    }

    /// Dart's `FlexibleSpaceBar.background is visible when using height other than
    /// kToolbarHeight`: an app bar that cannot collapse shows its background in full.
    #[test]
    fn flexible_space_bar_background_is_visible_when_using_height_other_than_k_toolbar_height() {
        test_widgets(|tester| {
            pump_bar(
                tester,
                TargetPlatform::Android,
                Extents::fixed(300.0),
                300.0,
                FlexibleSpaceBar::new()
                    .title(Text::new("Title"))
                    .background(Text::new("Background"))
                    .collapse_mode(CollapseMode::Pin),
            );
            let opacity = *tester
                .find_by_type::<FlexibleSpaceHeaderOpacity>()
                .first()
                .expect("a FlexibleSpaceHeaderOpacity");
            assert_eq!(
                tester.widget(opacity, |widget: &FlexibleSpaceHeaderOpacity| widget
                    .opacity),
                1.0
            );
        });
    }

    // This is a regression test for https://github.com/flutter/flutter/issues/138608.
    #[test]
    fn flexible_space_bar_centers_title_with_a_leading_widget() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                Material::new()
                    .child(
                        waft_widgets::CustomScrollView::new().slivers([SliverAppBar::new()
                            .leading(waft_widgets::Icon::new(Some(crate::icons::Icons::menu())))
                            .flexible_space(
                                FlexibleSpaceBar::new()
                                    .center_title(true)
                                    .title(waft_widgets::Text::new("X")),
                            )
                            .into_widget()]),
                    )
                    .into_widget()
            });

            let app_bar = tester.find_by_type::<AppBar>()[0];
            let app_bar_center = tester.global_rect(tester.render_box_of(app_bar)).center();
            let title = tester.find_text("X")[0];
            let title_center = tester.global_rect(tester.render_box_of(title)).center();
            assert_eq!(app_bar_center.dx(), title_center.dx());
        });
    }

    // This is a regression test for https://github.com/flutter/flutter/issues/138296.
    #[test]
    fn material3_default_title_color() {
        test_widgets(|tester| {
            let theme = ThemeData::new(&tester.cell.borrow());
            pump_app(tester, m3, |_app| {
                Material::new()
                    .child(
                        waft_widgets::CustomScrollView::new().slivers([SliverAppBar::new()
                            .flexible_space(
                                FlexibleSpaceBar::new().title(waft_widgets::Text::new("Title")),
                            )
                            .into_widget()]),
                    )
                    .into_widget()
            });

            let title = tester.find_text("Title")[0];
            let style = DefaultTextStyle::of(&mut tester.cell.borrow_mut(), title).style;
            assert_eq!(
                style.color,
                theme.text_theme.title_large.expect("a style").color
            );
        });
    }

    #[test]
    fn flexible_space_bar_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                Center::new()
                    .child(SizedBox::shrink().child(Scaffold::new().app_bar(
                        waft_widgets::PreferredSizeWidgetRef::new(AppBar::new().flexible_space(
                            FlexibleSpaceBar::new().title(waft_widgets::Text::new("X")),
                        )),
                    )))
                    .into_widget()
            });
            let bar = *tester
                .find_by_type::<FlexibleSpaceBar>()
                .first()
                .expect("a FlexibleSpaceBar");
            assert_eq!(
                tester.render_box_of(bar).size(&tester.cell.borrow()),
                Size::ZERO
            );
        });
    }

    /// The top of the background at `current`, in a space bar that collapses from 250 to 56 and
    /// is as high as `current`: the position the Dart collapse mode cases read after dragging
    /// the scroll view by `250 - current`.
    fn background_top(tester: &mut WidgetTester, collapse_mode: CollapseMode, current: f64) -> f64 {
        let key = GlobalKey::new();
        pump_bar(
            tester,
            TargetPlatform::Android,
            Extents {
                current,
                min: 56.0,
                max: 250.0,
            },
            current,
            FlexibleSpaceBar::new()
                .background(Container::new().key(key.clone().into()))
                .collapse_mode(collapse_mode),
        );
        rect_of_key(tester, &key).top
    }

    #[test]
    fn flexible_space_bar_collapse_mode_none() {
        test_widgets(|tester| {
            assert_eq!(background_top(tester, CollapseMode::None, 250.0), 0.0);
            assert_eq!(background_top(tester, CollapseMode::None, 150.0), 0.0);
        });
    }

    #[test]
    fn flexible_space_bar_collapse_mode_pin() {
        test_widgets(|tester| {
            assert_eq!(background_top(tester, CollapseMode::Pin, 250.0), 0.0);
            assert_eq!(background_top(tester, CollapseMode::Pin, 150.0), -100.0);
        });
    }

    #[test]
    fn flexible_space_bar_collapse_mode_parallax() {
        test_widgets(|tester| {
            assert_eq!(background_top(tester, CollapseMode::Parallax, 250.0), 0.0);
            let top = background_top(tester, CollapseMode::Parallax, 150.0);
            assert!(top < 10.0);
            assert!(top > -50.0);
        });
    }

    /// The background's size when the space bar is `height` high and its settings say it
    /// expands to 250: more than that is the overscroll a stretching `SliverAppBar` gives it.
    fn background_height(
        tester: &mut WidgetTester,
        stretch_modes: Vec<StretchMode>,
        height: f64,
    ) -> f64 {
        let key = GlobalKey::new();
        pump_bar(
            tester,
            TargetPlatform::Android,
            Extents {
                current: 250.0,
                min: 56.0,
                max: 250.0,
            },
            height,
            FlexibleSpaceBar::new()
                .background(Container::new().key(key.clone().into()))
                .stretch_modes(stretch_modes),
        );
        rect_of_key(tester, &key).height()
    }

    #[test]
    fn flexible_space_bar_stretch_mode_default_zoom_background() {
        test_widgets(|tester| {
            let before = background_height(tester, vec![StretchMode::ZoomBackground], 250.0);
            let after = background_height(tester, vec![StretchMode::ZoomBackground], 350.0);
            assert!(before < after);
        });
    }

    #[test]
    fn flexible_space_bar_stretch_mode_blur_background_does_not_zoom_the_background() {
        test_widgets(|tester| {
            let before = background_height(tester, vec![StretchMode::BlurBackground], 250.0);
            let after = background_height(tester, vec![StretchMode::BlurBackground], 350.0);
            assert_eq!(before, after);
        });
    }

    /// Dart's `FlexibleSpaceBar stretch mode blurBackground` is a golden case; this checks what
    /// the golden shows the cause of: the backdrop filter is there only while stretched.
    #[test]
    fn flexible_space_bar_stretch_mode_blur_background_blurs_only_when_stretched() {
        test_widgets(|tester| {
            for (height, filters) in [(250.0, 0), (350.0, 1)] {
                pump_bar(
                    tester,
                    TargetPlatform::Android,
                    Extents {
                        current: 250.0,
                        min: 56.0,
                        max: 250.0,
                    },
                    height,
                    FlexibleSpaceBar::new()
                        .stretch_modes(vec![StretchMode::BlurBackground])
                        .background(Container::new()),
                );
                assert_eq!(
                    tester.find_by_type::<waft_widgets::BackdropFilter>().len(),
                    filters
                );
            }
        });
    }

    /// The opacity of the `Opacity` around the title when the space bar is `height` high.
    fn fade_title_opacity(tester: &mut WidgetTester, height: f64) -> f64 {
        let key = GlobalKey::new();
        pump_bar(
            tester,
            TargetPlatform::Android,
            Extents {
                current: 250.0,
                min: 56.0,
                max: 250.0,
            },
            height,
            FlexibleSpaceBar::new()
                .stretch_modes(vec![StretchMode::FadeTitle])
                .title(keyed_title(&key)),
        );
        let title = *tester
            .find_by_type::<SizedBox>()
            .iter()
            .find(|element| {
                tester.ancestors(**element).iter().any(|ancestor| {
                    let app = tester.cell.borrow();
                    waft_widgets::downcast_widget::<Opacity>(&**ancestor.widget(&app)).is_some()
                })
            })
            .expect("a title under an Opacity");
        let opacity = tester.ancestor_of_type::<Opacity>(title);
        tester.widget(opacity, |opacity: &Opacity| opacity.opacity)
    }

    #[test]
    fn flexible_space_bar_stretch_mode_fade_title() {
        test_widgets(|tester| {
            assert_eq!(fade_title_opacity(tester, 260.0).round(), 1.0);
            assert_eq!(fade_title_opacity(tester, 350.0), 0.0);
        });
    }

    /// Dart's `RenderRebuildTracker`: a painter that counts its paints.
    struct PaintCounter {
        count: Rc<std::cell::Cell<usize>>,
    }

    impl waft_rendering::CustomPainter for PaintCounter {
        fn paint(&self, _app: &mut App, _canvas: &mut waft_embedder::Canvas, _size: Size) {
            self.count.set(self.count.get() + 1);
        }

        fn should_repaint(
            &self,
            _app: &App,
            _old_delegate: &dyn waft_rendering::CustomPainter,
        ) -> bool {
            false
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    /// Pumps on `tester` a space bar with `background`, at `current` of an app bar that
    /// collapses from 200 to 56 and is as high as `current`, where Dart's cases drag the
    /// `SliverAppBar` to; answers the alpha of the `OpacityLayer` around the background.
    fn pump_collapsing<K>(
        tester: &mut WidgetTester,
        current: f64,
        background: impl IntoWidget<K>,
    ) -> Option<i32> {
        pump_bar(
            tester,
            TargetPlatform::Android,
            Extents {
                current,
                min: 56.0,
                max: 200.0,
            },
            current,
            FlexibleSpaceBar::new().background(background),
        );
        let render = tester.first::<RenderFlexibleSpaceHeaderOpacity>();
        let app = tester.cell.borrow();
        render
            .as_object()
            .layer_as::<OpacityLayer>(&app)
            .map(|layer| layer.alpha(&app).expect("an alpha"))
    }

    /// Dart's `FlexibleSpaceBar rebuilds when scrolling.`, with the space bar's extents given
    /// where the scrolled `SliverAppBar` would have them: expanded, collapsing, collapsed.
    #[test]
    fn flexible_space_bar_rebuilds_when_scrolling() {
        test_widgets(|tester| {
            let count = Rc::new(std::cell::Cell::new(0));
            let background = || {
                waft_widgets::AspectRatio::new(1.7).child(waft_widgets::CustomPaint::new().painter(
                    PaintCounter {
                        count: Rc::clone(&count),
                    },
                ))
            };

            assert_eq!(pump_collapsing(tester, 200.0, background()), Some(255));
            assert_eq!(count.get(), 1);

            let alpha = pump_collapsing(tester, 80.0, background()).expect("a layer while fading");
            assert!(alpha < 255);

            assert_eq!(pump_collapsing(tester, 56.0, background()), None);
            assert!(count.get() > 1);
            assert!(
                tester
                    .all::<RenderFlexibleSpaceHeaderOpacity>()
                    .iter()
                    .all(|render| {
                        let app = tester.cell.borrow();
                        render.as_object().layer_as::<OpacityLayer>(&app).is_none()
                    })
            );
        });
    }

    /// Dart's `_FlexibleSpaceHeaderOpacity with near zero opacity avoids compositing`, with the
    /// space bar's extent given where the dragged `SliverAppBar` would have it.
    #[test]
    fn flexible_space_header_opacity_with_near_zero_opacity_avoids_compositing() {
        for platform in [
            TargetPlatform::Android,
            TargetPlatform::IOS,
            TargetPlatform::Fuchsia,
        ] {
            test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                let pump = |tester: &mut WidgetTester, current: f64| {
                    pump_bar(
                        tester,
                        platform,
                        Extents {
                            current,
                            min: 56.0,
                            max: 200.0,
                        },
                        current,
                        FlexibleSpaceBar::new().background(SizedBox::new()),
                    );
                };
                pump(tester, 200.0);
                // Ensure collapsed height - current extent is near zero for the
                // FlexibleSpaceBar to avoid compositing.
                pump(tester, 56.087_878_920_261_29);
                tester.pump_and_settle();
                let render = tester.first::<RenderFlexibleSpaceHeaderOpacity>();
                let app = tester.cell.borrow();
                assert!(
                    render.as_object().layer_as::<OpacityLayer>(&app).is_none(),
                    "{platform:?}"
                );
            });
        }
    }
}
