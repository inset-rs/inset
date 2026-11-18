//! Flutter counterpart: material_ui's `badge.dart`.

use waft_embedder::{Clip, Color, Offset, Size, TextBaseline};
use waft_foundation::App;
use waft_painting::{
    AlignmentGeometry, BorderSide, EdgeInsetsGeometry, ShapeDecoration, StadiumBorder,
    TextDirection, TextStyle,
};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BoxConstraints, BoxHitTestResult, ChildLayoutHelper,
    ChildLayouter, PaintingContext, RenderAligningShiftedBox, RenderAligningShiftedBoxData,
    RenderBox, RenderBoxData, RenderHandle, RenderObject, RenderObjectData,
    RenderObjectWithChildData, RenderObjectWithChildMixin, RenderProxyBoxMixin, RenderShiftedBox,
};
use waft_widgets::{
    BuildContext, Container, DefaultTextStyle, Directionality, IntoWidget, KeyRef, Positioned,
    RenderObjectWidget, SingleChildRenderObjectWidget, SizedBox, Stack, StatelessWidget, Text,
    WidgetRef,
};

use crate::badge_theme::BadgeTheme;
use crate::badge_theme::BadgeThemeDataGetters;
use crate::generated::badge_defaults_m3::BadgeDefaultsM3;

/// A Material Design "badge".
///
/// A badge's [`label`](Self::label) conveys a small amount of information about its
/// [`child`](Self::child), like a count or status. If the label is null then this is
/// a "small" badge that's displayed as a [`small_size`](Self::small_size) diameter filled
/// circle. Otherwise this is a [`StadiumBorder`] shaped "large" badge
/// with height [`large_size`](Self::large_size).
///
/// Badges are typically used to decorate the icon within a
/// `BottomNavigationBarItem` or a `NavigationRailDestination`
/// or a button's icon, as in `TextButton::icon`. The badge's default
/// configuration is intended to work well with a default sized (24)
/// `Icon`.
#[derive(Debug)]
pub struct Badge {
    pub key: Option<KeyRef>,

    /// The badge's fill color.
    ///
    /// Defaults to the [`BadgeTheme`]'s background color, or
    /// `ColorScheme::error` if the theme value is null.
    pub background_color: Option<Color>,

    /// The color of the badge's [`label`](Self::label) text.
    ///
    /// This color overrides the color of the label's [`text_style`](Self::text_style).
    ///
    /// Defaults to the [`BadgeTheme`]'s foreground color, or
    /// `ColorScheme::on_error` if the theme value is null.
    pub text_color: Option<Color>,

    /// The diameter of the badge if [`label`](Self::label) is null.
    ///
    /// Defaults to the [`BadgeTheme`]'s small size, or 6 if the theme value
    /// is null.
    pub small_size: Option<f64>,

    /// The badge's height if [`label`](Self::label) is non-null.
    ///
    /// Defaults to the [`BadgeTheme`]'s large size, or 16 if the theme value
    /// is null. If the default value is overridden then it may be useful to
    /// also override [`padding`](Self::padding) and [`alignment`](Self::alignment).
    pub large_size: Option<f64>,

    /// The `DefaultTextStyle` for the badge's label.
    ///
    /// The text style's color is overwritten by the [`text_color`](Self::text_color).
    ///
    /// This value is only used if [`label`](Self::label) is non-null.
    ///
    /// Defaults to the [`BadgeTheme`]'s text style, or the overall theme's
    /// `TextTheme::label_small` if the badge theme's value is null. If
    /// the default text style is overridden then it may be useful to
    /// also override [`large_size`](Self::large_size), [`padding`](Self::padding), and
    /// [`alignment`](Self::alignment).
    pub text_style: Option<TextStyle>,

    /// The padding added to the badge's label.
    ///
    /// This value is only used if [`label`](Self::label) is non-null.
    ///
    /// Defaults to the [`BadgeTheme`]'s padding, or 4 pixels on the
    /// left and right if the theme's value is null.
    pub padding: Option<EdgeInsetsGeometry>,

    /// Combined with [`offset`](Self::offset) to determine the location of the
    /// [`label`](Self::label) relative to the [`child`](Self::child).
    ///
    /// The alignment positions the label in the same way a child of an
    /// `Align` widget is positioned, except that, the alignment is
    /// resolved as if the label was a [`large_size`](Self::large_size) square and
    /// [`offset`](Self::offset) is added to the result.
    ///
    /// This value is only used if [`label`](Self::label) is non-null.
    ///
    /// Defaults to the [`BadgeTheme`]'s alignment, or
    /// `AlignmentDirectional::TOP_END` if the theme's value is null.
    pub alignment: Option<AlignmentGeometry>,

    /// Combined with [`alignment`](Self::alignment) to determine the location of the
    /// [`label`](Self::label) relative to the [`child`](Self::child).
    ///
    /// This value is only used if [`label`](Self::label) is non-null.
    ///
    /// Defaults to the [`BadgeTheme`]'s offset, or
    /// if the theme's value is null then `Offset(4, -4)` for
    /// [`TextDirection::Ltr`] or `Offset(-4, -4)` for [`TextDirection::Rtl`].
    pub offset: Option<Offset>,

    /// The badge's content, typically a `Text` widget that contains 1 to 4
    /// characters.
    ///
    /// If the label is null then this is a "small" badge that's
    /// displayed as a [`small_size`](Self::small_size) diameter filled circle. Otherwise
    /// this is a [`StadiumBorder`] shaped "large" badge with height
    /// [`large_size`](Self::large_size).
    ///
    /// A [`Badge::count`] has no label of its own: read it through
    /// [`effective_label`](Self::effective_label).
    pub label: Option<WidgetRef>,

    /// If false, the badge's [`label`](Self::label) is not included.
    ///
    /// This flag is true by default. It's intended to make it convenient
    /// to create a badge that's only shown under certain conditions.
    pub is_label_visible: bool,

    /// The widget that the badge is stacked on top of.
    ///
    /// Typically this is an default sized `Icon` that's part of a
    /// `BottomNavigationBarItem` or a `NavigationRailDestination`.
    pub child: Option<WidgetRef>,

    count: Option<i64>,
    max_count: i64,
}

impl Badge {
    /// Create a Badge that stacks [`label`](Self::label) on top of [`child`](Self::child).
    ///
    /// If [`label`](Self::label) is null then just a filled circle is displayed. Otherwise
    /// the [`label`](Self::label) is displayed within a [`StadiumBorder`] shaped area.
    pub fn new() -> Badge {
        Badge {
            key: None,
            background_color: None,
            text_color: None,
            small_size: None,
            large_size: None,
            text_style: None,
            padding: None,
            alignment: None,
            offset: None,
            label: None,
            is_label_visible: true,
            child: None,
            count: None,
            max_count: 999,
        }
    }

    /// Convenience constructor for creating a badge with a numeric label based on `count`.
    ///
    /// The label is a `Text` widget that shows:
    /// - the `count` value if it is less than or equal to [`max_count`](Self::max_count),
    /// - otherwise, shows '`max_count`+'.
    ///
    /// For example, if `count` is 1000 and [`max_count`](Self::max_count) is 99, the label
    /// will display '99+'.
    ///
    /// The `count` must be non-negative.
    pub fn count(count: i64) -> Badge {
        debug_assert!(count >= 0, "count must be non-negative");
        Badge {
            count: Some(count),
            ..Badge::new()
        }
    }

    /// Dart `Badge.count(maxCount:)`: the largest count the label shows, 999 unless given. It
    /// only has a say in a [`Badge::count`].
    ///
    /// The `max_count` must be positive.
    pub fn max_count(mut self, max_count: i64) -> Badge {
        debug_assert!(max_count > 0, "maxCount must be positive");
        self.max_count = max_count;
        self
    }

    /// Dart `Badge(key:)`.
    pub fn key(mut self, key: KeyRef) -> Badge {
        self.key = Some(key);
        self
    }

    /// Dart `Badge(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> Badge {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `Badge(textColor:)`.
    pub fn text_color(mut self, text_color: Color) -> Badge {
        self.text_color = Some(text_color);
        self
    }

    /// Dart `Badge(smallSize:)`.
    pub fn small_size(mut self, small_size: f64) -> Badge {
        self.small_size = Some(small_size);
        self
    }

    /// Dart `Badge(largeSize:)`.
    pub fn large_size(mut self, large_size: f64) -> Badge {
        self.large_size = Some(large_size);
        self
    }

    /// Dart `Badge(textStyle:)`.
    pub fn text_style(mut self, text_style: TextStyle) -> Badge {
        self.text_style = Some(text_style);
        self
    }

    /// Dart `Badge(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> Badge {
        let padding = padding.into();
        self.padding = Some(padding);
        self
    }

    /// Dart `Badge(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> Badge {
        self.alignment = Some(alignment);
        self
    }

    /// Dart `Badge(offset:)`.
    pub fn offset(mut self, offset: Offset) -> Badge {
        self.offset = Some(offset);
        self
    }

    /// Dart `Badge(label:)`.
    pub fn label<K>(mut self, label: impl IntoWidget<K>) -> Badge {
        self.label = Some(label.into_widget());
        self
    }

    /// Dart `Badge(isLabelVisible:)`.
    pub fn is_label_visible(mut self, is_label_visible: bool) -> Badge {
        self.is_label_visible = is_label_visible;
        self
    }

    /// Dart `Badge(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Badge {
        self.child = Some(child.into_widget());
        self
    }

    /// Dart's `label` field, as `Badge.count` fills it: a `Text` of the count, or of
    /// `maxCount+` once the count is over it.
    pub fn effective_label(&self) -> Option<WidgetRef> {
        match self.count {
            Some(count) if count > self.max_count => {
                Some(Text::new(format!("{}+", self.max_count)).into_widget())
            }
            Some(count) => Some(Text::new(count.to_string()).into_widget()),
            None => self.label.clone(),
        }
    }
}

impl Default for Badge {
    fn default() -> Badge {
        Badge::new()
    }
}

impl StatelessWidget for Badge {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        if !self.is_label_visible {
            return self
                .child
                .clone()
                .unwrap_or_else(|| SizedBox::new().into_widget());
        }

        let badge_theme = BadgeTheme::of(app, context);
        let defaults = BadgeDefaultsM3::new(context);
        let effective_decoration = ShapeDecoration::new(StadiumBorder::new(BorderSide::NONE))
            .color(
                self.background_color
                    .or(badge_theme.background_color)
                    .or_else(|| defaults.background_color(app))
                    .expect("the defaults give a background color"),
            );
        let effective_width_offset;
        let badge;
        let label = self.effective_label();
        let has_label = label.is_some();
        if let Some(label) = label {
            let min_size = self
                .large_size
                .or(badge_theme.large_size)
                .or_else(|| defaults.large_size(app))
                .expect("the defaults give a large size");
            effective_width_offset = min_size;
            let text_style = self
                .text_style
                .clone()
                .or(badge_theme.text_style.clone())
                .or_else(|| defaults.text_style(app))
                .expect("the defaults give a text style");
            let mut container = Container::new()
                .clip_behavior(Clip::AntiAlias)
                .decoration(effective_decoration)
                .padding(
                    self.padding
                        .or(badge_theme.padding)
                        .or_else(|| defaults.padding(app))
                        .expect("the defaults give a padding"),
                )
                .alignment(AlignmentGeometry::CENTER);
            container = container.child(label);
            badge = DefaultTextStyle::new(
                text_style.copy_with().color(
                    self.text_color
                        .or(badge_theme.text_color)
                        .or_else(|| defaults.text_color(app))
                        .expect("the defaults give a text color"),
                ),
                IntrinsicHorizontalStadium {
                    min_size,
                    child: Some(container.into_widget()),
                },
            )
            .into_widget();
        } else {
            let effective_small_size = self
                .small_size
                .or(badge_theme.small_size)
                .or_else(|| defaults.small_size(app))
                .expect("the defaults give a small size");
            effective_width_offset = effective_small_size;
            badge = Container::new()
                .width(effective_small_size)
                .height(effective_small_size)
                .clip_behavior(Clip::AntiAlias)
                .decoration(effective_decoration)
                .into_widget();
        }

        let Some(child) = self.child.clone() else {
            return badge;
        };

        let effective_alignment = self
            .alignment
            .or(badge_theme.alignment)
            .or_else(|| defaults.alignment(app))
            .expect("the defaults give an alignment");
        let text_direction = Directionality::of(app, context);
        let default_offset = if text_direction == TextDirection::Ltr {
            Offset::new(4.0, -4.0)
        } else {
            Offset::new(-4.0, -4.0)
        };
        // Adds a offset const Offset(0, 8) to avoiding breaking customers after
        // the offset calculation changes.
        // See https://github.com/flutter/flutter/pull/146853.
        let effective_offset =
            self.offset.or(badge_theme.offset).unwrap_or(default_offset) + Offset::new(0.0, 8.0);

        Stack::new()
            .clip_behavior(Clip::None)
            .children([
                child,
                Positioned::fill(BadgeLayout {
                    alignment: effective_alignment,
                    offset: if has_label {
                        effective_offset
                    } else {
                        Offset::ZERO
                    },
                    width_offset: effective_width_offset,
                    has_label,
                    child: Some(badge),
                })
                .into_widget(),
            ])
            .into_widget()
    }
}

/// Dart's `_Badge`, without its `textDirection`, which Dart passes and never reads.
#[derive(Debug)]
struct BadgeLayout {
    alignment: AlignmentGeometry,
    offset: Offset,
    width_offset: f64,
    has_label: bool,
    // the badge
    child: Option<WidgetRef>,
}

impl RenderObjectWidget for BadgeLayout {
    type RenderObject = RenderBadge;

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::maybe_of(app, context);
        RenderBadge::new(
            app,
            self.alignment,
            self.width_offset,
            self.has_label,
            self.offset,
            text_direction,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderBadge>,
    ) {
        let text_direction = Directionality::maybe_of(app, context);
        render_object.set_alignment(app, self.alignment);
        render_object.set_offset(app, self.offset);
        render_object.set_width_offset(app, self.width_offset);
        render_object.set_has_label(app, self.has_label);
        render_object.set_text_direction(app, text_direction);
    }
}

impl SingleChildRenderObjectWidget for BadgeLayout {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// Dart's `_RenderBadge`.
struct RenderBadge {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    aligning: RenderAligningShiftedBoxData,
    offset: Offset,
    has_label: bool,
    width_offset: f64,
}

impl RenderBadge {
    fn new(
        app: &mut App,
        alignment: AlignmentGeometry,
        width_offset: f64,
        has_label: bool,
        offset: Offset,
        text_direction: Option<TextDirection>,
    ) -> RenderHandle<RenderBadge> {
        RenderHandle::new_box(
            app,
            RenderBadge {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                aligning: RenderAligningShiftedBoxData::new(alignment, text_direction),
                offset,
                has_label,
                width_offset,
            },
        )
    }

    fn set_offset(self: RenderHandle<Self>, app: &mut App, value: Offset) {
        if self.get(app).offset == value {
            return;
        }
        self.get_mut(app).offset = value;
        self.mark_needs_layout(app);
    }

    fn set_has_label(self: RenderHandle<Self>, app: &mut App, value: bool) {
        if self.get(app).has_label == value {
            return;
        }
        self.get_mut(app).has_label = value;
        self.mark_needs_layout(app);
    }

    fn set_width_offset(self: RenderHandle<Self>, app: &mut App, value: f64) {
        if self.get(app).width_offset == value {
            return;
        }
        self.get_mut(app).width_offset = value;
        self.mark_needs_layout(app);
    }
}

impl RenderObjectWithChildMixin for RenderBadge {
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

impl RenderShiftedBox for RenderBadge {}

impl RenderAligningShiftedBox for RenderBadge {
    fn aligning_data(self: RenderHandle<Self>, app: &App) -> &RenderAligningShiftedBoxData {
        &self.get(app).aligning
    }

    fn aligning_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderAligningShiftedBoxData {
        &mut self.get_mut(app).aligning
    }
}

impl RenderObject for RenderBadge {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        debug_assert!(constraints.has_bounded_width());
        debug_assert!(constraints.has_bounded_height());
        self.set_size(app, constraints.biggest());
        let size = self.size(app);

        let child = self.child(app).expect("a badge has its child");
        child.layout(app, BoxConstraints::new(), true);
        let badge_size = child.size(app).height();
        let resolved_alignment = self.alignment(app).resolve(self.text_direction(app));
        let (offset, width_offset, has_label) = {
            let this = self.get(app);
            (this.offset, this.width_offset, this.has_label)
        };
        let mut badge_location = offset
            + resolved_alignment
                .along_offset(Offset::new(size.width() - width_offset, size.height()));
        if has_label {
            // Adjust for label height.
            badge_location = badge_location - Offset::new(0.0, badge_size / 2.0);
        }
        child
            .as_object()
            .parent_data_of_mut::<waft_rendering::BoxParentData>(app)
            .offset = badge_location;
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

impl RenderBox for RenderBadge {
    waft_rendering::render_box_accessors!();

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        RenderShiftedBox::hit_test_children(self, app, result, position)
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderShiftedBox::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderShiftedBox::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderShiftedBox::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderShiftedBox::compute_max_intrinsic_height(self, app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderShiftedBox::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        _app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        // Mirrors performLayout: size is the tightest allowed (biggest) under bounded constraints.
        // Callers (e.g., Stack) pass in tight constraints for Positioned.fill; otherwise, this
        // is still consistent with performLayout which asserts bounded constraints.
        constraints.biggest()
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app)?;

        // Child is laid out with unconstrained BoxConstraints in performLayout.
        let child_constraints = BoxConstraints::new();
        let child_baseline = child.get_dry_baseline(app, child_constraints, baseline)?;

        // Mirror the paint offset logic from performLayout using dry sizes only.
        let my_size = self.get_dry_layout(app, constraints);
        let resolved_alignment = self.alignment(app).resolve(self.text_direction(app));
        let child_size = child.get_dry_layout(app, child_constraints);

        let (offset, width_offset, has_label) = {
            let this = self.get(app);
            (this.offset, this.width_offset, this.has_label)
        };
        let mut badge_location = offset
            + resolved_alignment.along_offset(Offset::new(
                my_size.width() - width_offset,
                my_size.height(),
            ));
        if has_label {
            // Subtract half of the badge height when we have a label (as in performLayout).
            badge_location = badge_location - Offset::new(0.0, child_size.height() / 2.0);
        }

        Some(child_baseline + badge_location.dy())
    }
}

/// Dart's `_IntrinsicHorizontalStadium`.
#[derive(Debug)]
struct IntrinsicHorizontalStadium {
    min_size: f64,
    child: Option<WidgetRef>,
}

impl RenderObjectWidget for IntrinsicHorizontalStadium {
    type RenderObject = RenderIntrinsicHorizontalStadium;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderIntrinsicHorizontalStadium::new(app, self.min_size).as_object()
    }
}

impl SingleChildRenderObjectWidget for IntrinsicHorizontalStadium {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// Dart's `_RenderIntrinsicHorizontalStadium`.
struct RenderIntrinsicHorizontalStadium {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    min_size: f64,
}

impl RenderIntrinsicHorizontalStadium {
    fn new(app: &mut App, min_size: f64) -> RenderHandle<RenderIntrinsicHorizontalStadium> {
        RenderHandle::new_box(
            app,
            RenderIntrinsicHorizontalStadium {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                min_size,
            },
        )
    }

    fn child_constraints(
        self: RenderHandle<Self>,
        app: &mut App,
        child: AnyRenderBox,
        constraints: BoxConstraints,
    ) -> BoxConstraints {
        let min_size = self.get(app).min_size;
        let child_height = min_size.max(child.get_max_intrinsic_height(app, constraints.max_width));
        let child_width = child.get_max_intrinsic_width(app, constraints.max_height);
        constraints.tighten(Some(child_width.max(child_height)), Some(child_height))
    }

    fn compute_size(
        self: RenderHandle<Self>,
        app: &mut App,
        layout_child: ChildLayouter,
        constraints: BoxConstraints,
    ) -> Size {
        let child = self.child(app).expect("the badge label has its child");
        let child_constraints = self.child_constraints(app, child, constraints);
        let child_size = layout_child(app, child, child_constraints);
        if child_size.height() > child_size.width() {
            return Size::new(child_size.height(), child_size.height());
        }
        child_size
    }
}

impl RenderObjectWithChildMixin for RenderIntrinsicHorizontalStadium {
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

impl RenderProxyBoxMixin for RenderIntrinsicHorizontalStadium {}

impl RenderObject for RenderIntrinsicHorizontalStadium {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let size = self.compute_size(app, ChildLayoutHelper::layout_child, constraints);
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

impl RenderBox for RenderIntrinsicHorizontalStadium {
    waft_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        RenderProxyBoxMixin::setup_parent_data(self, app, child);
    }

    fn apply_paint_transform(
        self: RenderHandle<Self>,
        app: &App,
        child: AnyRenderObject,
        transform: &mut waft_embedder::Matrix4,
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
        self.get_max_intrinsic_width(app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        self.get_max_intrinsic_height(app, f64::INFINITY).max(
            RenderProxyBoxMixin::compute_max_intrinsic_width(self, app, height),
        )
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        self.get_max_intrinsic_height(app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let min_size = self.get(app).min_size;
        min_size.max(RenderProxyBoxMixin::compute_max_intrinsic_height(
            self, app, width,
        ))
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        RenderProxyBoxMixin::compute_distance_to_actual_baseline(self, app, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        self.compute_size(app, ChildLayoutHelper::dry_layout_child, constraints)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child = self.child(app).expect("the badge label has its child");
        let child_constraints = self.child_constraints(app, child, constraints);
        child.get_dry_baseline(app, child_constraints, baseline)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    //! Ported from `test/badge_test.dart`. flutter_test lays text out in FlutterTest, whose glyphs
    //! are squares of the font size; a case that reads a label's style reads the style of the
    //! `RenderParagraph`, as Dart does.

    use std::cell::RefCell;
    use std::rc::Rc;

    use waft_embedder::{Offset, RRect, Radius, Rect, Size};
    use waft_painting::Alignment;
    use waft_widgets::{Align, Builder, Center, Container, Icon};

    use waft_widgets::testing::{WidgetTester, find, test_widgets};

    use super::*;
    use crate::bottom_navigation_bar::tests::paragraph_style;
    use crate::colors::Colors;
    use crate::icons::Icons;
    use crate::test_support::{PaintInspection, path_like};
    use crate::theme::Theme;
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::{m3, pump_app};

    /// What a case reads from the build of its widgets.
    #[derive(Clone, Default)]
    pub(crate) struct Seen {
        theme: Rc<RefCell<Option<Rc<ThemeData>>>>,
    }

    impl Seen {
        fn theme(&self) -> Rc<ThemeData> {
            self.theme.borrow().clone().expect("the probe was built")
        }
    }

    /// Dart's `Builder(builder: (context) { theme = Theme.of(context); return badge; })`.
    fn badge_reading_theme(seen: &Seen, badge: Badge) -> WidgetRef {
        let theme = Rc::clone(&seen.theme);
        let badge = badge.into_widget();
        Builder::new(move |app, context| {
            *theme.borrow_mut() = Some(Theme::of(app, context));
            badge.clone()
        })
        .into_widget()
    }

    pub(crate) fn add_icon() -> WidgetRef {
        Icon::new(Some(Icons::add())).into_widget()
    }

    fn top_left_aligned<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Align::new()
            .alignment(AlignmentGeometry::TOP_LEFT)
            .child(child)
            .into_widget()
    }

    pub(crate) fn badge_rect(tester: &WidgetTester) -> Rect {
        let badge = find::by_type::<Badge>().evaluate(tester)[0];
        tester.get_rect(badge)
    }

    pub(crate) fn badge_paints_rrect(
        tester: &WidgetTester,
        rrect: RRect,
        color: Option<Color>,
    ) -> bool {
        let badge = find::by_type::<Badge>().evaluate(tester)[0];
        waft_rendering::testing::paints(
            &tester.painted_ops(tester.render_object(badge)),
            &[waft_rendering::testing::PaintCall::rrect_at(rrect, color)],
        )
        .is_ok()
    }

    fn stadium(left: f64, top: f64, right: f64, bottom: f64, radius: f64) -> RRect {
        RRect::from_ltrbr(left, top, right, bottom, Radius::circular(radius))
    }

    #[test]
    fn large_badge_defaults() {
        test_widgets(|tester| {
            let seen = Seen::default();
            pump_app(tester, m3, |_app| {
                top_left_aligned(badge_reading_theme(
                    &seen,
                    Badge::new().label(Text::new("0")).child(add_icon()),
                ))
            });
            let theme = seen.theme();

            assert_eq!(
                paragraph_style(tester, "0"),
                theme
                    .text_theme
                    .label_small
                    .clone()
                    .expect("a label small style")
                    .copy_with()
                    .color(theme.color_scheme.on_error())
            );

            // default badge alignment = AlignmentDirection.topEnd
            // default offset for LTR = Offset(4, -4)
            // default padding = EdgeInsets.symmetric(horizontal: 4)
            // default largeSize = 16
            // '0'.width = 11.5 (a 11 square and its letter spacing of 0.5)
            // icon.width = 24

            // default Icon size
            assert_eq!(badge_rect(tester).size(), Size::new(24.0, 24.0));
            assert_eq!(badge_rect(tester).top_left(), Offset::ZERO);

            assert_eq!(
                tester.get_rect(find::text("0")).top_left(),
                Offset::new(16.0, -4.0)
            );

            assert!(badge_paints_rrect(
                tester,
                stadium(12.0, -4.0, 31.5, 12.0, 8.0),
                Some(theme.color_scheme.error()),
            ));
        });
    }

    #[test]
    fn large_badge_defaults_with_rtl() {
        test_widgets(|tester| {
            let seen = Seen::default();
            pump_app(tester, m3, |_app| {
                Directionality::new(
                    TextDirection::Rtl,
                    top_left_aligned(badge_reading_theme(
                        &seen,
                        Badge::new().label(Text::new("0")).child(add_icon()),
                    )),
                )
                .into_widget()
            });
            let theme = seen.theme();

            assert_eq!(
                paragraph_style(tester, "0"),
                theme
                    .text_theme
                    .label_small
                    .clone()
                    .expect("a label small style")
                    .copy_with()
                    .color(theme.color_scheme.on_error())
            );

            assert_eq!(badge_rect(tester).size(), Size::new(24.0, 24.0));
            assert_eq!(badge_rect(tester).top_left(), Offset::ZERO);

            assert_eq!(
                tester.get_rect(find::text("0")).top_left(),
                Offset::new(0.0, -4.0)
            );

            assert!(badge_paints_rrect(
                tester,
                stadium(-4.0, -4.0, 15.5, 12.0, 8.0),
                Some(theme.color_scheme.error()),
            ));
        });
    }

    // Essentially the same as 'Large Badge defaults'.
    #[test]
    fn badge_count() {
        test_widgets(|tester| {
            let seen = Seen::default();
            let build_frame = |count: i64, seen: &Seen| {
                top_left_aligned(badge_reading_theme(
                    seen,
                    Badge::count(count).child(add_icon()),
                ))
            };
            pump_app(tester, m3, |_app| build_frame(0, &seen));
            let theme = seen.theme();

            // default badge alignment = AlignmentDirectional(12, -4)
            // default padding = EdgeInsets.symmetric(horizontal: 4)
            // default largeSize = 16
            // icon.width = 24

            // default Icon size
            assert_eq!(badge_rect(tester).size(), Size::new(24.0, 24.0));
            assert_eq!(badge_rect(tester).top_left(), Offset::ZERO);

            // The font's '0' is narrower than the badge is tall, where FlutterTest
            // gives it 11.5: the stadium is as wide as it is tall, and centers the text.
            let label = find::text("0").evaluate(tester);
            assert_eq!(label.len(), 1);
            let text_rect = tester.get_rect(label[0]);
            let badge_width = (text_rect.width() + 8.0).max(16.0);
            // x = alignment.start + the space around the text
            waft_widgets::testing::assert_more_or_less_equals(
                text_rect.left,
                12.0 + (badge_width - text_rect.width()) / 2.0,
            );
            waft_widgets::testing::assert_more_or_less_equals_within(text_rect.top, -4.0, 0.05);

            // L = alignment.start
            // T = alignment.top
            // R = L + '0'.width + padding.width
            // B = T + largeSize, R = largeSize/2
            assert!(badge_paints_rrect(
                tester,
                stadium(12.0, -4.0, 12.0 + badge_width, 12.0, 8.0),
                Some(theme.color_scheme.error()),
            ));

            pump_app(tester, m3, |_app| build_frame(1000, &seen));
            assert_eq!(find::text("999+").evaluate(tester).len(), 1);
        });
    }

    #[test]
    fn small_badge_defaults() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                top_left_aligned(Badge::new().child(add_icon()))
            });
            let colors = ThemeData::new(&tester.cell.borrow()).color_scheme;

            // default badge location is end=0, top=0
            // default padding = EdgeInsets.symmetric(horizontal: 4)
            // default smallSize = 6
            // icon.width = 24

            // default Icon size
            assert_eq!(badge_rect(tester).size(), Size::new(24.0, 24.0));
            assert_eq!(badge_rect(tester).top_left(), Offset::ZERO);

            // L = icon.size.width - smallSize
            // T = 0
            // R = icon.size.width
            // B = smallSize
            assert!(badge_paints_rrect(
                tester,
                stadium(18.0, 0.0, 24.0, 6.0, 3.0),
                Some(colors.error()),
            ));
        });
    }

    #[test]
    fn small_badge_rtl_defaults() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                Directionality::new(
                    TextDirection::Rtl,
                    top_left_aligned(Badge::new().child(add_icon())),
                )
                .into_widget()
            });
            let colors = ThemeData::new(&tester.cell.borrow()).color_scheme;

            // default badge location is end=0, top=0
            // default smallSize = 6
            // icon.width = 24

            // default Icon size
            assert_eq!(badge_rect(tester).size(), Size::new(24.0, 24.0));
            assert_eq!(badge_rect(tester).top_left(), Offset::ZERO);

            // L = 0
            // T = 0
            // R = smallSize
            // B = smallSize
            assert!(badge_paints_rrect(
                tester,
                stadium(0.0, 0.0, 6.0, 6.0, 3.0),
                Some(colors.error()),
            ));
        });
    }

    #[test]
    fn large_badge_text_style_and_colors() {
        test_widgets(|tester| {
            let green = Color::new(0xff00ff00);
            let black = Color::new(0xff000000);
            pump_app(tester, m3, |_app| {
                top_left_aligned(
                    Badge::new()
                        .text_color(green)
                        .background_color(black)
                        .text_style(TextStyle::new().font_size(10.0))
                        .label(Text::new("0"))
                        .child(add_icon()),
                )
            });

            let text_style = paragraph_style(tester, "0");
            assert_eq!(text_style.font_size, Some(10.0));
            assert_eq!(
                text_style.color.as_ref().map(|color| color.color()),
                Some(green)
            );
            let badge = find::by_type::<Badge>().evaluate(tester)[0];
            let drawn = tester.painted_calls(tester.render_object(badge));
            assert!(drawn.iter().any(|call| {
                path_like(call).is_some_and(|(color, _)| color.to_argb32() == 0xff000000)
            }));
        });
    }

    #[test]
    fn is_label_visible() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                top_left_aligned(
                    Badge::new()
                        .label(Text::new("0"))
                        .is_label_visible(false)
                        .child(add_icon()),
                )
            });

            assert!(find::text("0").evaluate(tester).is_empty());
            assert_eq!(find::by_type::<Icon>().evaluate(tester).len(), 1);

            // default Icon size
            assert_eq!(badge_rect(tester).size(), Size::new(24.0, 24.0));
            assert_eq!(badge_rect(tester).top_left(), Offset::ZERO);
            let badge = find::by_type::<Badge>().evaluate(tester)[0];
            let drawn = tester.painted_calls(tester.render_object(badge));
            assert!(!drawn.iter().any(|drawn| path_like(drawn).is_some()));
        });
    }

    /// The eight alignments of [`Alignment`] a badge `width` wide takes on a 200 square, as
    /// `(alignment, left, top)` of the badge's box, whose top is its alignment's, the box being
    /// the same height as the badge's label sits at.
    fn aligned_corners(width: f64) -> [(Alignment, f64, f64); 8] {
        [
            (Alignment::TOP_LEFT, 0.0, 0.0),
            (Alignment::TOP_CENTER, 100.0 - width / 2.0, 0.0),
            (Alignment::TOP_RIGHT, 200.0 - width, 0.0),
            (Alignment::CENTER_LEFT, 0.0, 100.0),
            (Alignment::CENTER_RIGHT, 200.0 - width, 100.0),
            (Alignment::BOTTOM_LEFT, 0.0, 200.0),
            (Alignment::BOTTOM_CENTER, 100.0 - width / 2.0, 200.0),
            (Alignment::BOTTOM_RIGHT, 200.0 - width, 200.0),
        ]
    }

    #[test]
    fn large_badge_alignment() {
        test_widgets(|tester| {
            let build_frame = |alignment: Alignment, offset: Offset| {
                top_left_aligned(
                    Badge::new()
                        // Default largeSize = 16, badge with label is "large".
                        .label(
                            Container::new()
                                .width(8.0)
                                .height(8.0)
                                .color(Colors::BLUE.primary()),
                        )
                        .alignment(alignment.into())
                        .offset(offset)
                        .child(
                            Container::new()
                                .color(Color::new(0xFF00FF00))
                                .width(200.0)
                                .height(200.0),
                        ),
                )
            };

            for offset in [Offset::ZERO, Offset::new(5.0, 10.0)] {
                for (alignment, left, top) in aligned_corners(16.0) {
                    pump_app(tester, m3, |_app| build_frame(alignment, offset));
                    assert!(
                        badge_paints_rrect(
                            tester,
                            stadium(left, top, left + 16.0, top + 16.0, 8.0).shift(offset),
                            None,
                        ),
                        "{alignment:?} with {offset:?}"
                    );
                }
            }
        });
    }

    #[test]
    fn small_badge_alignment() {
        test_widgets(|tester| {
            let build_frame = |alignment: Alignment, offset: Offset| {
                top_left_aligned(
                    Badge::new()
                        // Default smallSize = 6, badge without label is "small".
                        .alignment(alignment.into())
                        // Not used for smallSize badges.
                        .offset(offset)
                        .child(
                            Container::new()
                                .color(Color::new(0xFF00FF00))
                                .width(200.0)
                                .height(200.0),
                        ),
                )
            };

            for offset in [Offset::ZERO, Offset::new(5.0, 10.0)] {
                for (alignment, left, top) in aligned_corners(6.0) {
                    pump_app(tester, m3, |_app| build_frame(alignment, offset));
                    assert!(
                        badge_paints_rrect(
                            tester,
                            stadium(left, top, left + 6.0, top + 6.0, 3.0),
                            None,
                        ),
                        "{alignment:?} with {offset:?}"
                    );
                }
            }
        });
    }

    #[test]
    fn badge_larger_than_large_size() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                top_left_aligned(
                    Badge::new()
                        // LargeSize = 16, make content of badge bigger than the default.
                        .label(
                            Container::new()
                                .width(30.0)
                                .height(30.0)
                                .color(Colors::BLUE.primary()),
                        )
                        .alignment(Alignment::TOP_LEFT.into())
                        .offset(Offset::ZERO)
                        .child(
                            Container::new()
                                .color(Color::new(0xFF00FF00))
                                .width(200.0)
                                .height(200.0),
                        ),
                )
            });

            // Badge should scale with content
            assert!(badge_paints_rrect(
                tester,
                stadium(0.0, -7.0, 30.0 + 8.0, 23.0, 15.0),
                None,
            ));
        });
    }

    #[test]
    fn badge_renders_at_zero_area() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                Center::new()
                    .child(SizedBox::shrink().child(Badge::new().label(Text::new("X"))))
                    .into_widget()
            });
            let label = find::text("X").evaluate(tester);
            assert_eq!(label.len(), 1);
            let rect = tester.get_rect(label[0]);
            assert_eq!(rect.size(), Size::ZERO);
        });
    }

    #[test]
    fn badge_count_max_count_limits_displayed_value() {
        test_widgets(|tester| {
            let mut shows = |count: i64, max_count: Option<i64>, text: &str| {
                let badge = match max_count {
                    Some(max_count) => Badge::count(count).max_count(max_count),
                    None => Badge::count(count),
                };
                pump_app(tester, m3, |_app| top_left_aligned(badge.child(add_icon())));
                assert_eq!(find::text(text).evaluate(tester).len(), 1, "{text}");
            };

            shows(5, Some(99), "5");
            shows(99, Some(99), "99");
            shows(100, Some(99), "99+");
            shows(999, None, "999");
            shows(1000, None, "999+");
            // Test default maxCount (999)
            shows(1001, None, "999+");
        });
    }

    #[test]
    #[should_panic(expected = "count must be non-negative")]
    fn badge_count_asserts_on_negative_count() {
        let _ = Badge::count(-1).max_count(999).child(add_icon());
    }

    #[test]
    #[should_panic(expected = "maxCount must be positive")]
    fn badge_count_asserts_on_non_positive_max_count() {
        let _ = Badge::count(5).max_count(0).child(add_icon());
    }

    #[test]
    fn badge_count_displays_0_when_count_is_zero() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                top_left_aligned(Badge::count(0).max_count(5).child(add_icon()))
            });
            assert_eq!(find::text("0").evaluate(tester).len(), 1);
        });
    }
}
