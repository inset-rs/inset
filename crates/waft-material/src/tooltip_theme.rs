//! Flutter counterpart: material_ui's `tooltip_theme.dart`.

use std::time::Duration;

use waft_embedder::{TextAlign, lerp_double};
use waft_foundation::App;
use waft_painting::{DecorationRef, EdgeInsetsGeometry, TextStyle};
use waft_rendering::BoxConstraints;
use waft_widgets::{
    BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, TooltipTriggerMode,
    WidgetRef,
};

use crate::theme::Theme;

/// Defines default property values for descendant `Tooltip` widgets.
///
/// Descendant widgets obtain the current [`TooltipThemeData`] object using
/// [`TooltipTheme::of`]. Instances of [`TooltipThemeData`] can be customized
/// with [`TooltipThemeData::copy_with`].
///
/// Typically a [`TooltipThemeData`] is specified as part of the overall `Theme`
/// with `ThemeData::tooltip_theme`.
///
/// See also:
///
///  * [`TooltipTheme`], a widget which overrides the tooltip theme for a subtree.
///  * `ThemeData::tooltip_theme`, which specifies a tooltip theme as part of
///    an overall theme.
///  * `MaterialApp::theme`, which specifies a theme for the whole application.
#[derive(Clone, Debug, Default)]
pub struct TooltipThemeData {
    /// The minimum height of the `Tooltip`'s message.
    #[deprecated(
        note = "Use TooltipThemeData.constraints instead. This feature was deprecated after v3.30.0-0.1.pre."
    )]
    pub height: Option<f64>,

    /// Constrains the size of the `Tooltip`'s message.
    pub constraints: Option<BoxConstraints>,

    /// If provided, the amount of space by which to inset the `Tooltip`'s message.
    pub padding: Option<EdgeInsetsGeometry>,

    /// If provided, the amount of empty space to surround the `Tooltip`.
    pub margin: Option<EdgeInsetsGeometry>,

    /// The vertical gap between the widget and the displayed tooltip.
    ///
    /// When [`prefer_below`](Self::prefer_below) is set to true and tooltips have sufficient
    /// space to display themselves, this property defines how much vertical space
    /// tooltips will position themselves under their corresponding widgets.
    /// Otherwise, tooltips will position themselves above their corresponding
    /// widgets with the given offset.
    pub vertical_offset: Option<f64>,

    /// Whether the tooltip is displayed below its widget by default.
    ///
    /// If there is insufficient space to display the tooltip in the preferred
    /// direction, the tooltip will be displayed in the opposite direction.
    ///
    /// Applying `false` for the entire app is recommended
    /// to avoid having a finger or cursor hide a tooltip.
    pub prefer_below: Option<bool>,

    /// Whether the `Tooltip::message` should be excluded from the semantics
    /// tree.
    ///
    /// By default, `Tooltip`s will add a `Semantics` label that is set to
    /// `Tooltip::message`. Set this property to true if the app is going to
    /// provide its own custom semantics label.
    pub exclude_from_semantics: Option<bool>,

    /// The `Tooltip`'s shape and background color.
    pub decoration: Option<DecorationRef>,

    /// The style to use for the message of `Tooltip`s.
    pub text_style: Option<TextStyle>,

    /// The [`TextAlign`] to use for the message of `Tooltip`s.
    pub text_align: Option<TextAlign>,

    /// The length of time that a pointer must hover over a tooltip's widget
    /// before the tooltip will be shown.
    pub wait_duration: Option<Duration>,

    /// The length of time that the tooltip will be shown once it has appeared.
    pub show_duration: Option<Duration>,

    /// The length of time that a pointer must have stopped hovering over a
    /// tooltip's widget before the tooltip will be hidden.
    pub exit_duration: Option<Duration>,

    /// The [`TooltipTriggerMode`] that will show the tooltip.
    pub trigger_mode: Option<TooltipTriggerMode>,

    /// Whether the tooltip should provide acoustic and/or haptic feedback.
    ///
    /// For example, on Android a tap will produce a clicking sound and a
    /// long-press will produce a short vibration, when feedback is enabled.
    ///
    /// This value is used if `Tooltip::enable_feedback` is null.
    /// If this value is null, the default is true.
    ///
    /// See also:
    ///
    ///   * `Feedback`, for providing platform-specific feedback to certain actions.
    pub enable_feedback: Option<bool>,

    /// Whether the tooltip overlay should be invisible to hit testing.
    ///
    /// If `false`, moving the mouse pointer onto the tooltip overlay keeps the
    /// tooltip visible (satisfying WCAG 1.4.13 Hoverable).
    ///
    /// This value is used if `Tooltip::ignore_pointer` is null.
    pub ignore_pointer: Option<bool>,
}

#[allow(deprecated)]
impl TooltipThemeData {
    /// Creates the set of properties used to configure `Tooltip`s; Dart's optional named
    /// arguments are the fluent setters.
    pub fn new() -> TooltipThemeData {
        TooltipThemeData::default()
    }

    /// Dart `TooltipThemeData(height:)`.
    #[deprecated(
        note = "Use TooltipThemeData.constraints instead. This feature was deprecated after v3.30.0-0.1.pre."
    )]
    pub fn height(mut self, height: f64) -> TooltipThemeData {
        debug_assert!(
            self.constraints.is_none(),
            "Only one of `height` and `constraints` may be specified."
        );
        self.height = Some(height);
        self
    }

    /// Dart `TooltipThemeData(constraints:)`.
    pub fn constraints(mut self, constraints: BoxConstraints) -> TooltipThemeData {
        debug_assert!(
            self.height.is_none(),
            "Only one of `height` and `constraints` may be specified."
        );
        self.constraints = Some(constraints);
        self
    }

    /// Dart `TooltipThemeData(padding:)`.
    pub fn padding(mut self, padding: EdgeInsetsGeometry) -> TooltipThemeData {
        self.padding = Some(padding);
        self
    }

    /// Dart `TooltipThemeData(margin:)`.
    pub fn margin(mut self, margin: EdgeInsetsGeometry) -> TooltipThemeData {
        self.margin = Some(margin);
        self
    }

    /// Dart `TooltipThemeData(verticalOffset:)`.
    pub fn vertical_offset(mut self, vertical_offset: f64) -> TooltipThemeData {
        self.vertical_offset = Some(vertical_offset);
        self
    }

    /// Dart `TooltipThemeData(preferBelow:)`.
    pub fn prefer_below(mut self, prefer_below: bool) -> TooltipThemeData {
        self.prefer_below = Some(prefer_below);
        self
    }

    /// Dart `TooltipThemeData(excludeFromSemantics:)`.
    pub fn exclude_from_semantics(mut self, exclude_from_semantics: bool) -> TooltipThemeData {
        self.exclude_from_semantics = Some(exclude_from_semantics);
        self
    }

    /// Dart `TooltipThemeData(decoration:)`.
    pub fn decoration(mut self, decoration: impl Into<DecorationRef>) -> TooltipThemeData {
        self.decoration = Some(decoration.into());
        self
    }

    /// Dart `TooltipThemeData(textStyle:)`.
    pub fn text_style(mut self, text_style: TextStyle) -> TooltipThemeData {
        self.text_style = Some(text_style);
        self
    }

    /// Dart `TooltipThemeData(textAlign:)`.
    pub fn text_align(mut self, text_align: TextAlign) -> TooltipThemeData {
        self.text_align = Some(text_align);
        self
    }

    /// Dart `TooltipThemeData(waitDuration:)`.
    pub fn wait_duration(mut self, wait_duration: Duration) -> TooltipThemeData {
        self.wait_duration = Some(wait_duration);
        self
    }

    /// Dart `TooltipThemeData(showDuration:)`.
    pub fn show_duration(mut self, show_duration: Duration) -> TooltipThemeData {
        self.show_duration = Some(show_duration);
        self
    }

    /// Dart `TooltipThemeData(exitDuration:)`.
    pub fn exit_duration(mut self, exit_duration: Duration) -> TooltipThemeData {
        self.exit_duration = Some(exit_duration);
        self
    }

    /// Dart `TooltipThemeData(triggerMode:)`.
    pub fn trigger_mode(mut self, trigger_mode: TooltipTriggerMode) -> TooltipThemeData {
        self.trigger_mode = Some(trigger_mode);
        self
    }

    /// Dart `TooltipThemeData(enableFeedback:)`.
    pub fn enable_feedback(mut self, enable_feedback: bool) -> TooltipThemeData {
        self.enable_feedback = Some(enable_feedback);
        self
    }

    /// Dart `TooltipThemeData(ignorePointer:)`.
    pub fn ignore_pointer(mut self, ignore_pointer: bool) -> TooltipThemeData {
        self.ignore_pointer = Some(ignore_pointer);
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().prefer_below(false)`).
    pub fn copy_with(&self) -> TooltipThemeData {
        self.clone()
    }

    /// Linearly interpolate between two tooltip themes.
    ///
    /// If both arguments are null, then null is returned.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(
        a: Option<&TooltipThemeData>,
        b: Option<&TooltipThemeData>,
        t: f64,
    ) -> Option<TooltipThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(TooltipThemeData {
            height: lerp_double(a.and_then(|a| a.height), b.and_then(|b| b.height), t),
            constraints: BoxConstraints::lerp(
                a.and_then(|a| a.constraints),
                b.and_then(|b| b.constraints),
                t,
            ),
            padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.padding),
                b.and_then(|b| b.padding),
                t,
            ),
            margin: EdgeInsetsGeometry::lerp(a.and_then(|a| a.margin), b.and_then(|b| b.margin), t),
            vertical_offset: lerp_double(
                a.and_then(|a| a.vertical_offset),
                b.and_then(|b| b.vertical_offset),
                t,
            ),
            prefer_below: pick(a, b, t, |theme| theme.prefer_below),
            exclude_from_semantics: pick(a, b, t, |theme| theme.exclude_from_semantics),
            decoration: DecorationRef::lerp(
                a.and_then(|a| a.decoration.as_ref()),
                b.and_then(|b| b.decoration.as_ref()),
                t,
            ),
            text_style: TextStyle::lerp(
                a.and_then(|a| a.text_style.as_ref()),
                b.and_then(|b| b.text_style.as_ref()),
                t,
            ),
            text_align: pick(a, b, t, |theme| theme.text_align),
            wait_duration: pick(a, b, t, |theme| theme.wait_duration),
            show_duration: pick(a, b, t, |theme| theme.show_duration),
            exit_duration: pick(a, b, t, |theme| theme.exit_duration),
            trigger_mode: pick(a, b, t, |theme| theme.trigger_mode),
            enable_feedback: pick(a, b, t, |theme| theme.enable_feedback),
            ignore_pointer: pick(a, b, t, |theme| theme.ignore_pointer),
        })
    }
}

/// Dart's `t < 0.5 ? a?.field : b?.field`.
fn pick<T>(
    a: Option<&TooltipThemeData>,
    b: Option<&TooltipThemeData>,
    t: f64,
    field: impl Fn(&TooltipThemeData) -> Option<T>,
) -> Option<T> {
    if t < 0.5 {
        a.and_then(&field)
    } else {
        b.and_then(&field)
    }
}

#[allow(deprecated)]
impl PartialEq for TooltipThemeData {
    fn eq(&self, other: &TooltipThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.height == self.height
            && other.constraints == self.constraints
            && other.padding == self.padding
            && other.margin == self.margin
            && other.vertical_offset == self.vertical_offset
            && other.prefer_below == self.prefer_below
            && other.exclude_from_semantics == self.exclude_from_semantics
            && other.decoration == self.decoration
            && other.text_style == self.text_style
            && other.text_align == self.text_align
            && other.wait_duration == self.wait_duration
            && other.show_duration == self.show_duration
            && other.exit_duration == self.exit_duration
            && other.trigger_mode == self.trigger_mode
            && other.enable_feedback == self.enable_feedback
            && other.ignore_pointer == self.ignore_pointer
    }
}

/// Applies a tooltip theme to descendant `Tooltip` widgets.
///
/// A tooltip theme describes the values to use for `Tooltip` properties
/// that are not given an explicit non-null value.
///
/// Descendant widgets obtain the ambient tooltip theme, a [`TooltipThemeData`],
/// using [`TooltipTheme::of`].
///
/// Here is an example of a tooltip theme that applies a blue foreground
/// with non-rounded corners.
///
/// ```text
/// TooltipTheme::new(
///     TooltipThemeData::new().decoration(
///         BoxDecoration::new()
///             .color(Colors::BLUE.primary().with_values(Some(0.9), None, None, None, None))
///             .border_radius(BorderRadius::ZERO),
///     ),
///     Tooltip::message("Example tooltip").child(
///         IconButton::new(Icon::new(Icons::TOUCH_APP), on_pressed).icon_size(36.0),
///     ),
/// )
/// ```
///
/// See also:
///
///  * [`TooltipThemeData`], which describes the actual configuration of a
///    tooltip theme.
///  * `TooltipVisibility`, which can be used to visually disable descendant `Tooltip`s.
#[derive(Debug)]
pub struct TooltipTheme {
    pub key: Option<KeyRef>,
    /// The properties for descendant `Tooltip` widgets.
    pub data: TooltipThemeData,
    pub child: WidgetRef,
}

impl TooltipTheme {
    /// Creates a tooltip theme that controls the configurations for
    /// `Tooltip`.
    pub fn new<K>(data: TooltipThemeData, child: impl IntoWidget<K>) -> TooltipTheme {
        TooltipTheme {
            key: None,
            data,
            child: child.into_widget(),
        }
    }

    /// Dart `TooltipTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> TooltipTheme {
        self.key = Some(key);
        self
    }

    /// Retrieves the [`TooltipThemeData`] from the closest ancestor [`TooltipTheme`].
    ///
    /// The result comes from the closest [`TooltipTheme`] ancestor if any,
    /// and otherwise from `Theme::of` and `ThemeData::tooltip_theme`.
    ///
    /// When a widget uses this method, it is automatically rebuilt if the
    /// tooltip theme later changes, so that the changes can be applied.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = TooltipTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> TooltipThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<TooltipTheme>(app) {
            Some(tooltip_theme) => tooltip_theme.data.clone(),
            None => Theme::of(app, context).tooltip_theme.clone(),
        }
    }
}

impl InheritedTheme for TooltipTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        TooltipTheme::new(self.data.clone(), child).into_widget()
    }
}

impl InheritedWidget for TooltipTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &TooltipTheme) -> bool {
        self.data != old_widget.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use waft_embedder::{Color, Offset, Rect, TextDecoration};
    use waft_gestures::K_LONG_PRESS_TIMEOUT;
    use waft_painting::AnyColor;
    use waft_rendering::AnyRenderBox;
    use waft_rendering::testing::{PaintCall, assert_paints};
    use waft_widgets::{Center, GlobalKey, Padding, RawTooltip, SizedBox, Text};

    use waft_widgets::testing::{
        TestPointer, WidgetTester, expect, find, finds_one_widget, test_widgets,
    };

    use super::*;
    use crate::colors::Colors;
    use crate::test_support::PaintInspection;
    use crate::theme_data::ThemeData;
    use crate::tooltip::Tooltip;
    use crate::tooltip::tests::{
        TOOLTIP_TEXT, custom_decoration, ensure_visible_and_fade_in, found, green_box, m2, m3,
        overlay, paragraph_size, positioned_at, pump_app, rect, size, text_box, text_style,
        tooltip_center, tooltip_text,
    };

    const CUSTOM_PADDING_VALUE: f64 = 10.0;

    #[test]
    fn tooltip_theme_data_copy_with_equality_basics() {
        assert_eq!(TooltipThemeData::new(), TooltipThemeData::new().copy_with());
    }

    #[test]
    fn tooltip_theme_data_lerp_special_cases() {
        assert_eq!(TooltipThemeData::lerp(None, None, 0.0), None);
        let data = TooltipThemeData::new();
        // Dart's `identical`: the value comes back as it was.
        assert_eq!(
            TooltipThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data.clone())
        );

        let a = TooltipThemeData::new()
            .wait_duration(Duration::from_millis(100))
            .show_duration(Duration::from_millis(200))
            .exit_duration(Duration::from_millis(300))
            .trigger_mode(TooltipTriggerMode::Tap)
            .enable_feedback(true)
            .ignore_pointer(true);
        let b = TooltipThemeData::new()
            .wait_duration(Duration::from_millis(400))
            .show_duration(Duration::from_millis(500))
            .exit_duration(Duration::from_millis(600))
            .trigger_mode(TooltipTriggerMode::LongPress)
            .enable_feedback(false)
            .ignore_pointer(false);
        let lerped_early = TooltipThemeData::lerp(Some(&a), Some(&b), 0.25).expect("both given");
        assert_eq!(lerped_early.wait_duration, a.wait_duration);
        assert_eq!(lerped_early.show_duration, a.show_duration);
        assert_eq!(lerped_early.exit_duration, a.exit_duration);
        assert_eq!(lerped_early.trigger_mode, a.trigger_mode);
        assert_eq!(lerped_early.enable_feedback, a.enable_feedback);
        assert_eq!(lerped_early.ignore_pointer, a.ignore_pointer);

        let lerped_late = TooltipThemeData::lerp(Some(&a), Some(&b), 0.75).expect("both given");
        assert_eq!(lerped_late.wait_duration, b.wait_duration);
        assert_eq!(lerped_late.show_duration, b.show_duration);
        assert_eq!(lerped_late.exit_duration, b.exit_duration);
        assert_eq!(lerped_late.trigger_mode, b.trigger_mode);
        assert_eq!(lerped_late.enable_feedback, b.enable_feedback);
        assert_eq!(lerped_late.ignore_pointer, b.ignore_pointer);
    }

    #[test]
    fn tooltip_theme_data_defaults() {
        let theme = TooltipThemeData::new();
        assert_eq!(theme.height, None);
        assert_eq!(theme.constraints, None);
        assert_eq!(theme.padding, None);
        assert_eq!(theme.vertical_offset, None);
        assert_eq!(theme.prefer_below, None);
        assert_eq!(theme.exclude_from_semantics, None);
        assert!(theme.decoration.is_none());
        assert_eq!(theme.text_style, None);
        assert_eq!(theme.text_align, None);
        assert_eq!(theme.wait_duration, None);
        assert_eq!(theme.show_duration, None);
        assert_eq!(theme.exit_duration, None);
        assert_eq!(theme.trigger_mode, None);
        assert_eq!(theme.enable_feedback, None);
        assert_eq!(theme.ignore_pointer, None);
    }

    /// Where a case puts its tooltip theme: in the app's `ThemeData`, or in a `TooltipTheme`
    /// above the tree under test.
    #[derive(Clone, Copy)]
    enum Source {
        ThemeData,
        TooltipTheme,
    }

    /// `MaterialApp(theme: base(app) with tooltip_theme, home: home)`, or
    /// `MaterialApp(theme: base(app), home: TooltipTheme(data: tooltip_theme, child: home))`.
    fn pump_themed(
        tester: &mut WidgetTester,
        source: Source,
        base: fn(&App) -> ThemeData,
        tooltip_theme: TooltipThemeData,
        home: impl FnOnce(&mut App) -> WidgetRef,
    ) {
        match source {
            Source::ThemeData => pump_app(
                tester,
                |app| base(app).copy_with().tooltip_theme(tooltip_theme).build(),
                home,
            ),
            Source::TooltipTheme => pump_app(tester, base, |app| {
                TooltipTheme::new(tooltip_theme, home(app)).into_widget()
            }),
        }
    }

    /// The render box `n` parents above the tooltip's paragraph: Dart's
    /// `tester.renderObject(find.text(tooltipText)).parent!...`.
    fn text_parent(tester: &WidgetTester, n: usize) -> AnyRenderBox {
        let mut render = text_box(tester).as_object();
        let app = tester.cell.borrow();
        for _ in 0..n {
            render = render.parent(&app).expect("a parent");
        }
        render.as_box().expect("a render box")
    }

    fn vertical_offset_case(
        tester: &mut WidgetTester,
        source: Source,
        top: f64,
        prefer_below: bool,
    ) -> Rect {
        let key = GlobalKey::new();
        let tooltip = Tooltip::new()
            .key(key.clone().into())
            .message(TOOLTIP_TEXT)
            .child(SizedBox::shrink());
        let height = if top == 300.0 && !prefer_below {
            100.0
        } else {
            190.0
        };
        pump_themed(
            tester,
            source,
            m3,
            TooltipThemeData::new()
                .height(height)
                .padding(EdgeInsetsGeometry::ZERO)
                .vertical_offset(100.0)
                .prefer_below(prefer_below),
            |app| overlay(app, positioned_at(400.0, top, tooltip)),
        );
        ensure_visible_and_fade_in(tester, &key);
        rect(tester, text_parent(tester, 1))
    }

    fn assert_prefer_above_fits(tip: Rect) {
        assert_eq!(tip.height(), 100.0);
        assert_eq!(tip.top, 100.0);
        assert_eq!(tip.bottom, 200.0);
    }

    fn assert_prefer_above_does_not_fit(tip: Rect) {
        assert_eq!(tip.height(), 190.0);
        assert_eq!(tip.top, 399.0);
        assert_eq!(tip.bottom, 589.0);
    }

    fn assert_prefer_below_fits(tip: Rect) {
        assert_eq!(tip.height(), 190.0);
        assert_eq!(tip.top, 400.0);
        assert_eq!(tip.bottom, 590.0);
    }

    #[test]
    fn tooltip_vertical_offset_prefer_below_center_prefer_above_fits_theme_data_tooltip_theme() {
        test_widgets(|tester| {
            assert_prefer_above_fits(vertical_offset_case(
                tester,
                Source::ThemeData,
                300.0,
                false,
            ));
        });
    }

    #[test]
    fn tooltip_vertical_offset_prefer_below_center_prefer_above_fits_tooltip_theme() {
        test_widgets(|tester| {
            assert_prefer_above_fits(vertical_offset_case(
                tester,
                Source::TooltipTheme,
                300.0,
                false,
            ));
        });
    }

    #[test]
    fn tooltip_vertical_offset_prefer_below_center_prefer_above_does_not_fit_theme_data_tooltip_theme()
     {
        test_widgets(|tester| {
            assert_prefer_above_does_not_fit(vertical_offset_case(
                tester,
                Source::ThemeData,
                299.0,
                false,
            ));
        });
    }

    #[test]
    fn tooltip_vertical_offset_prefer_below_center_prefer_above_does_not_fit_tooltip_theme() {
        test_widgets(|tester| {
            assert_prefer_above_does_not_fit(vertical_offset_case(
                tester,
                Source::TooltipTheme,
                299.0,
                false,
            ));
        });
    }

    #[test]
    fn tooltip_vertical_offset_prefer_below_center_prefer_below_fits_theme_data_tooltip_theme() {
        test_widgets(|tester| {
            assert_prefer_below_fits(vertical_offset_case(tester, Source::ThemeData, 300.0, true));
        });
    }

    #[test]
    fn tooltip_vertical_offset_prefer_below_center_prefer_below_fits_tooltip_theme() {
        test_widgets(|tester| {
            assert_prefer_below_fits(vertical_offset_case(
                tester,
                Source::TooltipTheme,
                300.0,
                true,
            ));
        });
    }

    /// The margin cases, whose theme wraps the tooltip inside the overlay entry: a `Theme`
    /// with a fresh `ThemeData`, or a `TooltipTheme`.
    fn margin_case(tester: &mut WidgetTester, source: Source) {
        let key = GlobalKey::new();
        let tooltip = Tooltip::new()
            .key(key.clone().into())
            .message(TOOLTIP_TEXT)
            .child(SizedBox::shrink());
        let tooltip_theme = TooltipThemeData::new()
            .padding(EdgeInsetsGeometry::ZERO)
            .margin(EdgeInsetsGeometry::all(CUSTOM_PADDING_VALUE));
        pump_app(tester, m3, |app| {
            let themed = match source {
                Source::ThemeData => {
                    let data = ThemeData::builder().tooltip_theme(tooltip_theme).build(app);
                    crate::theme::Theme::new(data, tooltip).into_widget()
                }
                Source::TooltipTheme => TooltipTheme::new(tooltip_theme, tooltip).into_widget(),
            };
            overlay(app, themed)
        });
        ensure_visible_and_fade_in(tester, &key);

        let tip = rect(tester, text_parent(tester, 5));
        let tooltip_content = rect(tester, text_box(tester));
        assert_eq!(tooltip_content.left, tip.left + CUSTOM_PADDING_VALUE);
        assert_eq!(tooltip_content.top, tip.top + CUSTOM_PADDING_VALUE);
        assert_eq!(tooltip_content.right, tip.right - CUSTOM_PADDING_VALUE);
        assert_eq!(tooltip_content.bottom, tip.bottom - CUSTOM_PADDING_VALUE);
    }

    #[test]
    fn tooltip_margin_theme_data() {
        test_widgets(|tester| margin_case(tester, Source::ThemeData));
    }

    #[test]
    fn tooltip_margin_tooltip_theme() {
        test_widgets(|tester| margin_case(tester, Source::TooltipTheme));
    }

    fn orange_underline() -> TextStyle {
        TextStyle::new()
            .color(Colors::ORANGE.primary())
            .decoration(TextDecoration::UNDERLINE)
    }

    fn assert_orange_underline(text_style: &TextStyle) {
        assert_eq!(
            text_style.color,
            Some(AnyColor::new(Colors::ORANGE.primary()))
        );
        assert_eq!(text_style.font_family, None);
        assert_eq!(text_style.decoration, Some(TextDecoration::UNDERLINE));
    }

    #[test]
    fn tooltip_message_text_style_theme_data_tooltip_theme() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let tooltip = Tooltip::new()
                .key(key.clone().into())
                .message(TOOLTIP_TEXT)
                .child(green_box())
                .into_widget();
            pump_themed(
                tester,
                Source::ThemeData,
                m3,
                TooltipThemeData::new().text_style(orange_underline()),
                |_app| tooltip,
            );
            ensure_visible_and_fade_in(tester, &key);
            assert_orange_underline(&text_style(tester));
        });
    }

    #[test]
    fn tooltip_message_text_style_tooltip_theme() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let tooltip = Tooltip::new()
                .text_style(orange_underline())
                .key(key.clone().into())
                .message(TOOLTIP_TEXT)
                .child(green_box())
                .into_widget();
            pump_themed(
                tester,
                Source::TooltipTheme,
                m3,
                TooltipThemeData::new(),
                |_app| tooltip,
            );
            ensure_visible_and_fade_in(tester, &key);
            assert_orange_underline(&text_style(tester));
        });
    }

    #[test]
    fn tooltip_message_text_align_tooltip_theme() {
        test_widgets(|tester| {
            // A tester each: Waft's `WidgetsApp` keeps the `home` its first route was made
            // with, where Dart's page reads `widget.home` when it rebuilds.
            let mut pump_tooltip_with_text_align = |text_align: Option<TextAlign>| {
                let key = GlobalKey::new();
                let tooltip = Tooltip::new()
                    .key(key.clone().into())
                    .message(TOOLTIP_TEXT)
                    .child(green_box())
                    .into_widget();
                let mut data = TooltipThemeData::new();
                if let Some(text_align) = text_align {
                    data = data.text_align(text_align);
                }
                pump_themed(tester, Source::TooltipTheme, m3, data, |_app| tooltip);
                ensure_visible_and_fade_in(tester, &key);
                tester.widget(tooltip_text(tester), |text: &Text| text.text_align)
            };

            // Default value should be TextAlign.start
            assert_eq!(pump_tooltip_with_text_align(None), Some(TextAlign::Start));
            assert_eq!(
                pump_tooltip_with_text_align(Some(TextAlign::Center)),
                Some(TextAlign::Center)
            );
            assert_eq!(
                pump_tooltip_with_text_align(Some(TextAlign::End)),
                Some(TextAlign::End)
            );
        });
    }

    fn decoration_case(tester: &mut WidgetTester, source: Source, base: fn(&App) -> ThemeData) {
        let key = GlobalKey::new();
        let tooltip = Tooltip::new()
            .key(key.clone().into())
            .message(TOOLTIP_TEXT)
            .child(SizedBox::shrink())
            .into_widget();
        pump_themed(
            tester,
            source,
            base,
            TooltipThemeData::new().decoration(custom_decoration()),
            |app| overlay(app, tooltip),
        );
        ensure_visible_and_fade_in(tester, &key);

        let tip = text_parent(tester, 4);
        assert_eq!(size(tester, tip).height(), 32.0);
        // Flutter's 74.0 (Material 2) and 74.75 (Material 3): FlutterTest's `TIP` + 16.0 padding * 2.
        assert_eq!(
            size(tester, tip).width(),
            paragraph_size(tester).width() + 32.0
        );
        assert_paints(
            &tester.painted_ops(tip.as_object()),
            &[PaintCall::rrect(Some(Color::new(0x80800000)))],
        );
    }

    #[test]
    fn material2_tooltip_decoration_theme_data_tooltip_theme() {
        test_widgets(|tester| decoration_case(tester, Source::ThemeData, m2));
    }

    #[test]
    fn material3_tooltip_decoration_theme_data_tooltip_theme() {
        test_widgets(|tester| decoration_case(tester, Source::ThemeData, m3));
    }

    #[test]
    fn material2_tooltip_decoration_tooltip_theme() {
        test_widgets(|tester| decoration_case(tester, Source::TooltipTheme, m2));
    }

    #[test]
    fn material3_tooltip_decoration_tooltip_theme() {
        test_widgets(|tester| decoration_case(tester, Source::TooltipTheme, m3));
    }

    fn height_and_padding_case(tester: &mut WidgetTester, source: Source) {
        const CUSTOM_TOOLTIP_HEIGHT: f64 = 100.0;
        const CUSTOM_PADDING_VAL: f64 = 20.0;
        let key = GlobalKey::new();
        let tooltip = Tooltip::new()
            .key(key.clone().into())
            .message(TOOLTIP_TEXT)
            .into_widget();
        pump_themed(
            tester,
            source,
            m3,
            TooltipThemeData::new()
                .height(CUSTOM_TOOLTIP_HEIGHT)
                .padding(EdgeInsetsGeometry::all(CUSTOM_PADDING_VAL)),
            |app| overlay(app, tooltip),
        );
        let state = key
            .current_state::<crate::tooltip::TooltipState>(&tester.cell.borrow_mut())
            .expect("mounted");
        state.ensure_tooltip_visible(&mut tester.cell.borrow_mut());
        tester.pump_and_settle();

        // Select [Tooltip.padding] instead of [Tooltip.margin].
        let padding = tester.ancestor_of_type::<Padding>(tooltip_text(tester));
        let tip = size(tester, tester.render_box_of(padding));
        let center = tester.ancestor_of_type::<Center>(tooltip_text(tester));
        let content = size(tester, tester.render_box_of(center));

        assert_eq!(tip.height(), CUSTOM_TOOLTIP_HEIGHT);
        assert_eq!(
            content.height(),
            CUSTOM_TOOLTIP_HEIGHT - 2.0 * CUSTOM_PADDING_VAL
        );
        assert_eq!(content.width(), tip.width() - 2.0 * CUSTOM_PADDING_VAL);
    }

    #[test]
    fn tooltip_height_and_padding_theme_data_tooltip_theme() {
        test_widgets(|tester| height_and_padding_case(tester, Source::ThemeData));
    }

    #[test]
    fn tooltip_height_and_padding_tooltip_theme() {
        test_widgets(|tester| height_and_padding_case(tester, Source::TooltipTheme));
    }

    /// Dart's `Theme(data: ThemeData(tooltipTheme:), child:)` around the home, or a
    /// `TooltipTheme`: the duration and trigger cases.
    fn pump_home_themed(tester: &mut WidgetTester, source: Source, data: TooltipThemeData) {
        let home = Center::new()
            .child(
                Tooltip::new()
                    .message(TOOLTIP_TEXT)
                    .child(SizedBox::new().width(100.0).height(100.0)),
            )
            .into_widget();
        pump_app(tester, m3, |app| match source {
            Source::ThemeData => {
                let theme = ThemeData::builder().tooltip_theme(data).build(app);
                crate::theme::Theme::new(theme, home).into_widget()
            }
            Source::TooltipTheme => TooltipTheme::new(data, home).into_widget(),
        });
    }

    /// Dart's mouse `TestGesture` added at the origin and wiggled before the tree is pumped.
    fn mouse_at_origin(tester: &mut WidgetTester) -> TestPointer {
        let mut gesture = TestPointer::mouse(1, 0);
        tester.add_pointer(&mut gesture, Offset::ZERO);
        tester.move_to(&mut gesture, Offset::new(1.0, 1.0), false);
        tester.pump();
        tester.move_to(&mut gesture, Offset::ZERO, false);
        gesture
    }

    fn wait_duration_case(tester: &mut WidgetTester, source: Source) {
        const CUSTOM_WAIT_DURATION: Duration = Duration::from_millis(500);
        let mut gesture = mouse_at_origin(tester);
        pump_home_themed(
            tester,
            source,
            TooltipThemeData::new().wait_duration(CUSTOM_WAIT_DURATION),
        );

        tester.move_to(&mut gesture, Offset::ZERO, false);
        tester.pump();
        let at = tooltip_center(tester);
        tester.move_to(&mut gesture, at, false);
        tester.pump();
        tester.pump_by(Duration::from_millis(250));
        assert!(!found(tester, TOOLTIP_TEXT)); // Should not appear yet
        tester.pump_by(Duration::from_millis(250));
        expect(tester, &find::text(TOOLTIP_TEXT), finds_one_widget()); // Should appear after customWaitDuration

        tester.move_to(&mut gesture, Offset::ZERO, false);
        tester.pump();

        // Wait for it to disappear.
        tester.pump_by(Duration::from_millis(100)); // Should disappear after default exitDuration
        assert!(!found(tester, TOOLTIP_TEXT));
    }

    #[test]
    fn tooltip_wait_duration_theme_data_tooltip_theme() {
        test_widgets(|tester| wait_duration_case(tester, Source::ThemeData));
    }

    #[test]
    fn tooltip_wait_duration_tooltip_theme() {
        test_widgets(|tester| wait_duration_case(tester, Source::TooltipTheme));
    }

    fn show_duration_case(tester: &mut WidgetTester, source: Source) {
        const CUSTOM_SHOW_DURATION: Duration = Duration::from_millis(3000);
        pump_home_themed(
            tester,
            source,
            TooltipThemeData::new().show_duration(CUSTOM_SHOW_DURATION),
        );

        let at = tooltip_center(tester);
        let mut gesture = TestPointer::touch(1);
        tester.down(&mut gesture, at);
        tester.pump();
        tester.pump_by(K_LONG_PRESS_TIMEOUT);
        tester.up(&gesture);
        expect(tester, &find::text(TOOLTIP_TEXT), finds_one_widget());
        tester.pump();
        tester.pump_by(Duration::from_millis(2000)); // Tooltip should remain
        expect(tester, &find::text(TOOLTIP_TEXT), finds_one_widget());
        tester.pump_by(Duration::from_millis(1000));
        tester.pump_and_settle(); // Tooltip should fade out after
        assert!(!found(tester, TOOLTIP_TEXT));
    }

    #[test]
    fn tooltip_show_duration_theme_data_tooltip_theme() {
        test_widgets(|tester| show_duration_case(tester, Source::ThemeData));
    }

    #[test]
    fn tooltip_show_duration_tooltip_theme() {
        test_widgets(|tester| show_duration_case(tester, Source::TooltipTheme));
    }

    fn exit_duration_case(tester: &mut WidgetTester, source: Source) {
        const CUSTOM_EXIT_DURATION: Duration = Duration::from_millis(500);
        let mut gesture = mouse_at_origin(tester);
        pump_home_themed(
            tester,
            source,
            TooltipThemeData::new().exit_duration(CUSTOM_EXIT_DURATION),
        );

        let at = tooltip_center(tester);
        tester.move_to(&mut gesture, at, false);
        tester.pump();
        expect(tester, &find::text(TOOLTIP_TEXT), finds_one_widget());

        tester.move_to(&mut gesture, Offset::ZERO, false);
        tester.pump_by(Duration::from_millis(200));
        tester.pump_and_settle();
        expect(tester, &find::text(TOOLTIP_TEXT), finds_one_widget());

        // Wait for it to disappear.
        tester.pump_by(CUSTOM_EXIT_DURATION);
        tester.pump_and_settle();
        assert!(!found(tester, TOOLTIP_TEXT));
    }

    #[test]
    fn tooltip_exit_duration_theme_data_tooltip_theme() {
        test_widgets(|tester| exit_duration_case(tester, Source::ThemeData));
    }

    #[test]
    fn tooltip_exit_duration_tooltip_theme() {
        test_widgets(|tester| exit_duration_case(tester, Source::TooltipTheme));
    }

    fn trigger_mode_case(tester: &mut WidgetTester, source: Source) {
        pump_home_themed(
            tester,
            source,
            TooltipThemeData::new().trigger_mode(TooltipTriggerMode::Tap),
        );

        let at = tooltip_center(tester);
        let mut gesture = TestPointer::touch(1);
        tester.down(&mut gesture, at);
        tester.up(&gesture);
        tester.pump();
        // Tooltip should show immediately after tap
        expect(tester, &find::text(TOOLTIP_TEXT), finds_one_widget());
    }

    #[test]
    fn tooltip_trigger_mode_theme_data_trigger_mode() {
        test_widgets(|tester| trigger_mode_case(tester, Source::ThemeData));
    }

    #[test]
    fn tooltip_trigger_mode_tooltip_theme() {
        test_widgets(|tester| trigger_mode_case(tester, Source::TooltipTheme));
    }

    #[test]
    fn tooltip_respects_constraints_from_the_ambient_theme() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let theme_constraints = BoxConstraints::tight_for(Some(300.0), Some(150.0));
            let tooltip = Tooltip::new()
                .key(key.clone().into())
                .message(TOOLTIP_TEXT)
                .padding(EdgeInsetsGeometry::ZERO)
                .child(waft_widgets::ColoredBox::new(Colors::GREEN.primary()))
                .into_widget();
            pump_themed(
                tester,
                Source::ThemeData,
                m3,
                TooltipThemeData::new().constraints(theme_constraints),
                |_app| tooltip,
            );
            ensure_visible_and_fade_in(tester, &key);

            let text_ancestor = tester.ancestors(tooltip_text(tester))[0];
            assert_eq!(
                text_ancestor.size(&tester.cell.borrow()),
                Some(theme_constraints.biggest())
            );
        });
    }

    #[test]
    fn tooltip_respects_ignore_pointer_from_the_ambient_theme() {
        test_widgets(|tester| {
            let tooltip = Tooltip::new()
                .message(TOOLTIP_TEXT)
                .child(SizedBox::new().width(100.0).height(100.0))
                .into_widget();
            pump_themed(
                tester,
                Source::ThemeData,
                m3,
                TooltipThemeData::new().ignore_pointer(false),
                |_app| tooltip,
            );

            let raw_tooltips = find::by_type::<RawTooltip>().evaluate(tester);
            assert_eq!(raw_tooltips.len(), 1);
            let ignore_pointer = tester.widget(raw_tooltips[0], |raw_tooltip: &RawTooltip| {
                raw_tooltip.ignore_pointer
            });
            assert!(!ignore_pointer);
        });
    }
}
