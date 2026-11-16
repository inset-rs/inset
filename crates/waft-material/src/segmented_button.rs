//! Flutter counterpart: material_ui's `segmented_button.dart`.

use std::any::{Any, TypeId};
use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_embedder::{
    Color, Offset, PathBuilder, RRect, Radius, Rect, Size, TextBaseline, clamp_double,
};
use waft_foundation::{App, Handle, Listener, Ref, ValueChanged};
use waft_painting::{
    AlignmentGeometry, Axis, BorderSide, ClipContext, EdgeInsets, EdgeInsetsDirectional,
    EdgeInsetsGeometry, OutlinedBorderRef, RoundedRectangleBorder, TextDirection, TextStyle,
    draw_line,
};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BaselineOffset, BoxConstraints, BoxHitTestResult, BoxParentData,
    ContainerBoxParentData, ContainerParentData, ContainerParentDataMixin,
    ContainerRenderObjectData, ContainerRenderObjectMixin, MainAxisSize, PaintingContext,
    ParentData, PipelineOwner, RenderBox, RenderBoxContainerDefaultsMixin, RenderBoxData,
    RenderHandle, RenderObject, RenderObjectData,
};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, Directionality, Flexible, IntoWidget, KeyRef, MediaQuery, MergeSemantics,
    MultiChildRenderObjectWidget, Padding, RenderObjectWidget, Row, Semantics, SizedBox, State,
    StateData, StatefulWidget, WidgetRef, WidgetState, WidgetStateProperty, WidgetStatePropertyAll,
    WidgetStatePropertyRef, WidgetStates, WidgetStatesController,
};

use crate::button_style::ButtonStyle;
use crate::button_style_button::{ButtonStyleButton, IconAlignment};
use crate::constants::K_MIN_INTERACTIVE_DIMENSION;
use crate::generated::segmented_button_defaults_m3::SegmentedButtonDefaultsM3;
use crate::ink_well::InteractiveInkFeatureFactoryRef;
use crate::material::{Material, MaterialType};
use crate::segmented_button_theme::{SegmentedButtonTheme, SegmentedButtonThemeDataGetters};
use crate::text_button::TextButton;
use crate::text_button_theme::{TextButtonTheme, TextButtonThemeData};
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, VisualDensity};
use crate::tooltip::Tooltip;

/// Data describing a segment of a [`SegmentedButton`].
///
/// Two segments are equal when their fields are: their widgets by identity. Where Dart compares
/// two segments by identity, a [`SegmentedButton`] matches the segments of a rebuild by
/// [`value`](Self::value), which must be unique.
#[derive(Clone, Debug)]
pub struct ButtonSegment<T> {
    /// Value used to identify the segment.
    ///
    /// This value must be unique across all segments in a [`SegmentedButton`].
    pub value: T,

    /// Optional icon displayed in the segment.
    pub icon: Option<WidgetRef>,

    /// Optional label displayed in the segment.
    pub label: Option<WidgetRef>,

    /// Optional tooltip for the segment
    pub tooltip: Option<String>,

    /// Determines if the segment is available for selection.
    pub enabled: bool,
}

impl<T> ButtonSegment<T> {
    /// Construct a [`ButtonSegment`].
    ///
    /// One of [`icon`](Self::icon) or [`label`](Self::label) must be non-null, which
    /// [`SegmentedButton`] asserts when it is built into a widget.
    pub fn new(value: T) -> ButtonSegment<T> {
        ButtonSegment {
            value,
            icon: None,
            label: None,
            tooltip: None,
            enabled: true,
        }
    }

    /// Dart `ButtonSegment(icon:)`.
    pub fn icon<K>(mut self, icon: impl IntoWidget<K>) -> ButtonSegment<T> {
        self.icon = Some(icon.into_widget());
        self
    }

    /// Dart `ButtonSegment(label:)`.
    pub fn label<K>(mut self, label: impl IntoWidget<K>) -> ButtonSegment<T> {
        self.label = Some(label.into_widget());
        self
    }

    /// Dart `ButtonSegment(tooltip:)`.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> ButtonSegment<T> {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Dart `ButtonSegment(enabled:)`.
    pub fn enabled(mut self, enabled: bool) -> ButtonSegment<T> {
        self.enabled = enabled;
        self
    }
}

impl<T: PartialEq> PartialEq for ButtonSegment<T> {
    fn eq(&self, other: &ButtonSegment<T>) -> bool {
        self.value == other.value
            && self.enabled == other.enabled
            && self.tooltip == other.tooltip
            && same_widget(&self.icon, &other.icon)
            && same_widget(&self.label, &other.label)
    }
}

fn same_widget(a: &Option<WidgetRef>, b: &Option<WidgetRef>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}

/// A Material button that allows the user to select from limited set of options.
///
/// Segmented buttons are used to help people select options, switch views, or
/// sort elements. They are typically used in cases where there are only 2-5
/// options.
///
/// The options are represented by segments described with [`ButtonSegment`]
/// entries in the [`segments`](Self::segments) field. Each segment has a
/// [`ButtonSegment::value`] that is used to indicate which segments are selected.
///
/// The [`selected`](Self::selected) field is a set of selected [`ButtonSegment::value`]s,
/// each once. This should be updated by the app in response to
/// [`on_selection_changed`](Self::on_selection_changed) updates.
///
/// By default, only a single segment can be selected (for mutually exclusive
/// choices). This can be relaxed with the
/// [`multi_selection_enabled`](Self::multi_selection_enabled) field.
///
/// Like [`ButtonStyleButton`]s, the [`SegmentedButton`]'s visuals can be
/// configured with a [`ButtonStyle`] [`style`](Self::style) field. However, unlike other
/// buttons, some of the style parameters are applied to the entire segmented
/// button, and others are used for each of the segments.
///
/// By default, a checkmark icon is used to show selected items. To configure
/// this behavior, you can use the [`show_selected_icon`](Self::show_selected_icon) and
/// [`selected_icon`](Self::selected_icon) fields.
///
/// Individual segments can be enabled or disabled with their
/// [`ButtonSegment::enabled`] flag. If the [`on_selection_changed`](Self::on_selection_changed)
/// field is null, then the entire segmented button will be disabled, regardless of the
/// individual segment settings.
///
/// See also:
///
///   * Material Design spec: <https://m3.material.io/components/segmented-buttons/overview>
///   * [`ButtonStyle`], which can be used in the [`style`](Self::style) field to configure
///     the appearance of the button and its segments.
///   * `ToggleButtons`, a similar widget that was built for Material 2.
///     [`SegmentedButton`] should be considered as a replacement for
///     `ToggleButtons`.
///   * `Radio`, an alternative way to present the user with a mutually exclusive set of options.
///   * `FilterChip`, `ChoiceChip`, which can be used when you need to show more than five options.
pub struct SegmentedButton<T> {
    /// See [`Widget::key`](waft_widgets::Widget::key).
    pub key: Option<KeyRef>,

    /// Descriptions of the segments in the button.
    ///
    /// This a required parameter and must contain at least one segment,
    /// but it is recommended to contain two to five segments. If you need only
    /// a single segment, consider using a `Checkbox` or `Radio` widget instead.
    /// If you need more than five options, consider using `FilterChip` or
    /// `ChoiceChip` widgets.
    pub segments: Vec<ButtonSegment<T>>,

    /// The orientation of the button's [`segments`](Self::segments).
    ///
    /// If this is [`Axis::Vertical`], the segments will be aligned vertically
    /// and the first item in [`segments`](Self::segments) will be on the top.
    ///
    /// Defaults to [`Axis::Horizontal`].
    pub direction: Axis,

    /// The set of [`ButtonSegment::value`]s that indicate which [`segments`](Self::segments) are
    /// selected, in insertion order, each once.
    ///
    /// As the [`SegmentedButton`] does not maintain the state of the selection,
    /// you will need to update this in response to
    /// [`on_selection_changed`](Self::on_selection_changed) calls.
    ///
    /// This is a required parameter.
    pub selected: Vec<T>,

    /// The function that is called when the selection changes.
    ///
    /// The callback's parameter indicates which of the segments are selected.
    ///
    /// When the callback is null, the entire [`SegmentedButton`] is disabled,
    /// and will not respond to input.
    ///
    /// The default is null.
    pub on_selection_changed: Option<ValueChanged<Vec<T>>>,

    /// Determines if multiple segments can be selected at one time.
    ///
    /// If true, more than one segment can be selected. When selecting a
    /// segment, the other selected segments will stay selected. Selecting an
    /// already selected segment will unselect it.
    ///
    /// If false, only one segment may be selected at a time. When a segment
    /// is selected, any previously selected segment will be unselected.
    ///
    /// The default is false, so only a single segment may be selected at one
    /// time.
    pub multi_selection_enabled: bool,

    /// Determines if having no selected segments is allowed.
    ///
    /// If true, then it is acceptable for none of the segments to be selected.
    /// This means that [`selected`](Self::selected) can be empty. If the user taps on a
    /// selected segment, it will be removed from the selection set passed into
    /// [`on_selection_changed`](Self::on_selection_changed).
    ///
    /// If false (the default), there must be at least one segment selected. If
    /// the user taps on the only selected segment it will not be deselected, and
    /// [`on_selection_changed`](Self::on_selection_changed) will not be called.
    pub empty_selection_allowed: bool,

    /// Determines the segmented button's size and padding based on
    /// [`expanded_insets`](Self::expanded_insets).
    ///
    /// If null (default), the button adopts its intrinsic content size. When specified,
    /// the button expands to fill its parent's space, with the [`EdgeInsets`]
    /// defining the padding.
    pub expanded_insets: Option<EdgeInsets>,

    /// Customizes this button's appearance.
    ///
    /// The following style properties apply to the entire segmented button:
    ///
    ///   * [`ButtonStyle::shadow_color`]
    ///   * [`ButtonStyle::elevation`]
    ///   * [`ButtonStyle::side`] - which is used for both the outer shape and
    ///     dividers between segments.
    ///   * [`ButtonStyle::shape`]
    ///
    /// The following style properties are applied to each of the individual
    /// button segments. For properties that are a `WidgetStateProperty`,
    /// they will be resolved with the current state of the segment:
    ///
    ///   * [`ButtonStyle::text_style`]
    ///   * [`ButtonStyle::background_color`]
    ///   * [`ButtonStyle::foreground_color`]
    ///   * [`ButtonStyle::overlay_color`]
    ///   * [`ButtonStyle::surface_tint_color`]
    ///   * [`ButtonStyle::elevation`]
    ///   * [`ButtonStyle::padding`]
    ///   * [`ButtonStyle::icon_color`]
    ///   * [`ButtonStyle::icon_size`]
    ///   * [`ButtonStyle::mouse_cursor`]
    ///   * [`ButtonStyle::visual_density`]
    ///   * [`ButtonStyle::tap_target_size`]
    ///   * [`ButtonStyle::animation_duration`]
    ///   * [`ButtonStyle::enable_feedback`]
    ///   * [`ButtonStyle::alignment`]
    ///   * [`ButtonStyle::splash_factory`]
    ///
    /// If [`ButtonStyle::side`] is provided, `WidgetStateProperty::resolve` is used
    /// for the following `WidgetState`s:
    ///
    ///  * `WidgetState::Focused`.
    ///  * `WidgetState::Hovered`.
    ///  * `WidgetState::Disabled`.
    ///  * `WidgetState::Selected`.
    pub style: Option<ButtonStyle>,

    /// Determines if the [`selected_icon`](Self::selected_icon) (usually an icon using
    /// `Icons::check`) is displayed on the selected segments.
    ///
    /// If true, the [`selected_icon`](Self::selected_icon) will be displayed at the start of
    /// the segment. If both the [`ButtonSegment::label`] and [`ButtonSegment::icon`] are
    /// provided, then the icon will be replaced with the [`selected_icon`](Self::selected_icon).
    /// If only the icon or the label is present then the
    /// [`selected_icon`](Self::selected_icon) will be shown at the start of the segment.
    ///
    /// If false, then the [`selected_icon`](Self::selected_icon) is not used and will not be
    /// displayed on selected segments.
    ///
    /// The default is true, meaning the [`selected_icon`](Self::selected_icon) will be shown on
    /// selected segments.
    pub show_selected_icon: bool,

    /// An icon that is used to indicate a segment is selected.
    ///
    /// If [`show_selected_icon`](Self::show_selected_icon) is true then for selected segments
    /// this icon will be shown before the [`ButtonSegment::label`], replacing the
    /// [`ButtonSegment::icon`] if it is specified.
    ///
    /// Defaults to an `Icon` with `Icons::check`.
    pub selected_icon: Option<WidgetRef>,
}

impl<T: Clone + PartialEq + 'static> SegmentedButton<T> {
    /// Creates a [`SegmentedButton`].
    ///
    /// [`segments`](Self::segments) must contain at least one segment, but it is recommended
    /// to have two to five segments. If you need only single segment, consider using a
    /// `Checkbox` or `Radio` widget instead. If you need more than five options, consider
    /// using `FilterChip` or `ChoiceChip` widgets.
    ///
    /// If [`on_selection_changed`](Self::on_selection_changed) is null, then the entire
    /// segmented button will be disabled.
    ///
    /// By default [`selected`](Self::selected) must only contain one entry. However, if
    /// [`multi_selection_enabled`](Self::multi_selection_enabled) is true, then
    /// [`selected`](Self::selected) can contain multiple entries. If
    /// [`empty_selection_allowed`](Self::empty_selection_allowed) is true, then
    /// [`selected`](Self::selected) can be empty.
    pub fn new(segments: Vec<ButtonSegment<T>>, selected: Vec<T>) -> SegmentedButton<T> {
        SegmentedButton {
            key: None,
            segments,
            direction: Axis::Horizontal,
            selected,
            on_selection_changed: None,
            multi_selection_enabled: false,
            empty_selection_allowed: false,
            expanded_insets: None,
            style: None,
            show_selected_icon: true,
            selected_icon: None,
        }
    }

    /// Dart `SegmentedButton(key:)`.
    pub fn key(mut self, key: KeyRef) -> SegmentedButton<T> {
        self.key = Some(key);
        self
    }

    /// Dart `SegmentedButton(direction:)`.
    pub fn direction(mut self, direction: Axis) -> SegmentedButton<T> {
        self.direction = direction;
        self
    }

    /// Dart `SegmentedButton(onSelectionChanged:)`.
    pub fn on_selection_changed(
        mut self,
        on_selection_changed: impl Fn(&mut App, Vec<T>) + 'static,
    ) -> SegmentedButton<T> {
        self.on_selection_changed = Some(Rc::new(on_selection_changed));
        self
    }

    /// Dart `SegmentedButton(multiSelectionEnabled:)`.
    pub fn multi_selection_enabled(mut self, multi_selection_enabled: bool) -> SegmentedButton<T> {
        self.multi_selection_enabled = multi_selection_enabled;
        self
    }

    /// Dart `SegmentedButton(emptySelectionAllowed:)`.
    pub fn empty_selection_allowed(mut self, empty_selection_allowed: bool) -> SegmentedButton<T> {
        self.empty_selection_allowed = empty_selection_allowed;
        self
    }

    /// Dart `SegmentedButton(expandedInsets:)`.
    pub fn expanded_insets(mut self, expanded_insets: EdgeInsets) -> SegmentedButton<T> {
        self.expanded_insets = Some(expanded_insets);
        self
    }

    /// Dart `SegmentedButton(style:)`.
    pub fn style(mut self, style: ButtonStyle) -> SegmentedButton<T> {
        self.style = Some(style);
        self
    }

    /// Dart `SegmentedButton(showSelectedIcon:)`.
    pub fn show_selected_icon(mut self, show_selected_icon: bool) -> SegmentedButton<T> {
        self.show_selected_icon = show_selected_icon;
        self
    }

    /// Dart `SegmentedButton(selectedIcon:)`.
    pub fn selected_icon<K>(mut self, selected_icon: impl IntoWidget<K>) -> SegmentedButton<T> {
        self.selected_icon = Some(selected_icon.into_widget());
        self
    }
}

impl<T> SegmentedButton<T> {
    /// A static convenience method that constructs a segmented button
    /// [`ButtonStyle`] given simple values.
    ///
    /// The `foreground_color`, `selected_foreground_color`, and `disabled_foreground_color`
    /// colors are used to create a `WidgetStateProperty` [`ButtonStyle::foreground_color`],
    /// and a derived [`ButtonStyle::overlay_color`] if `overlay_color` isn't specified.
    ///
    /// If `overlay_color` is specified and its value is `Colors::TRANSPARENT`
    /// then the pressed/focused/hovered highlights are effectively defeated.
    /// Otherwise a `WidgetStateProperty` with the same opacities as the
    /// default is created.
    ///
    /// The `background_color`, `selected_background_color` and `disabled_background_color`
    /// colors are used to create a `WidgetStateProperty` [`ButtonStyle::background_color`].
    ///
    /// Similarly, the `enabled_mouse_cursor` and `disabled_mouse_cursor`
    /// parameters are used to construct [`ButtonStyle::mouse_cursor`].
    ///
    /// The `icon_color`, `disabled_icon_color` are used to construct
    /// [`ButtonStyle::icon_color`] and `icon_size` is used to construct
    /// [`ButtonStyle::icon_size`].
    ///
    /// All of the other parameters are either used directly or used to
    /// create a `WidgetStateProperty` with a single value for all
    /// states.
    ///
    /// All parameters default to null. By default this method returns
    /// a [`ButtonStyle`] that doesn't override anything.
    ///
    /// For example, to override the default text and icon colors for a
    /// [`SegmentedButton`], as well as its overlay color, with all of the
    /// standard opacity adjustments for the pressed, focused, and
    /// hovered states, one could write:
    ///
    /// ```text
    /// SegmentedButton::new(segments, selected).style(
    ///     SegmentedButton::<i32>::style_from()
    ///         .foreground_color(Colors::BLACK)
    ///         .selected_foreground_color(Colors::WHITE)
    ///         .background_color(Colors::AMBER.primary())
    ///         .selected_background_color(Colors::RED.primary())
    ///         .build(),
    /// )
    /// ```
    pub fn style_from() -> SegmentedButtonStyleFromBuilder {
        SegmentedButtonStyleFromBuilder::default()
    }
}

/// Dart's `SegmentedButton.styleFrom(..)` arguments; [`build`](Self::build) makes the
/// [`ButtonStyle`].
#[derive(Clone, Default)]
pub struct SegmentedButtonStyleFromBuilder {
    foreground_color: Option<Color>,
    background_color: Option<Color>,
    selected_foreground_color: Option<Color>,
    selected_background_color: Option<Color>,
    disabled_foreground_color: Option<Color>,
    disabled_background_color: Option<Color>,
    shadow_color: Option<Color>,
    surface_tint_color: Option<Color>,
    icon_color: Option<Color>,
    icon_size: Option<f64>,
    disabled_icon_color: Option<Color>,
    overlay_color: Option<Color>,
    elevation: Option<f64>,
    text_style: Option<TextStyle>,
    padding: Option<EdgeInsetsGeometry>,
    minimum_size: Option<Size>,
    fixed_size: Option<Size>,
    maximum_size: Option<Size>,
    side: Option<BorderSide>,
    shape: Option<OutlinedBorderRef>,
    enabled_mouse_cursor: Option<MouseCursorRef>,
    disabled_mouse_cursor: Option<MouseCursorRef>,
    visual_density: Option<VisualDensity>,
    tap_target_size: Option<MaterialTapTargetSize>,
    animation_duration: Option<Duration>,
    enable_feedback: Option<bool>,
    alignment: Option<AlignmentGeometry>,
    splash_factory: Option<InteractiveInkFeatureFactoryRef>,
}

/// The fluent setters of Dart's named `styleFrom` arguments.
macro_rules! style_from_setters {
    ($($field:ident: $ty:ty => $dart:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `SegmentedButton.styleFrom(", $dart, ":)`.")]
            pub fn $field(mut self, $field: $ty) -> SegmentedButtonStyleFromBuilder {
                self.$field = Some($field);
                self
            }
        )*
    };
}

impl SegmentedButtonStyleFromBuilder {
    style_from_setters!(
        foreground_color: Color => "foregroundColor",
        background_color: Color => "backgroundColor",
        selected_foreground_color: Color => "selectedForegroundColor",
        selected_background_color: Color => "selectedBackgroundColor",
        disabled_foreground_color: Color => "disabledForegroundColor",
        disabled_background_color: Color => "disabledBackgroundColor",
        shadow_color: Color => "shadowColor",
        surface_tint_color: Color => "surfaceTintColor",
        icon_color: Color => "iconColor",
        icon_size: f64 => "iconSize",
        disabled_icon_color: Color => "disabledIconColor",
        overlay_color: Color => "overlayColor",
        elevation: f64 => "elevation",
        text_style: TextStyle => "textStyle",
        padding: EdgeInsetsGeometry => "padding",
        minimum_size: Size => "minimumSize",
        fixed_size: Size => "fixedSize",
        maximum_size: Size => "maximumSize",
        side: BorderSide => "side",
        enabled_mouse_cursor: MouseCursorRef => "enabledMouseCursor",
        disabled_mouse_cursor: MouseCursorRef => "disabledMouseCursor",
        visual_density: VisualDensity => "visualDensity",
        tap_target_size: MaterialTapTargetSize => "tapTargetSize",
        animation_duration: Duration => "animationDuration",
        enable_feedback: bool => "enableFeedback",
        alignment: AlignmentGeometry => "alignment",
        splash_factory: InteractiveInkFeatureFactoryRef => "splashFactory",
    );

    /// Dart `SegmentedButton.styleFrom(shape:)`.
    pub fn shape(mut self, shape: impl Into<OutlinedBorderRef>) -> SegmentedButtonStyleFromBuilder {
        self.shape = Some(shape.into());
        self
    }

    /// The [`ButtonStyle`] made from these values.
    pub fn build(self) -> ButtonStyle {
        let overlay_color_prop: Option<WidgetStatePropertyRef<Option<Color>>> =
            if self.foreground_color.is_none()
                && self.selected_foreground_color.is_none()
                && self.overlay_color.is_none()
            {
                None
            } else {
                match self.overlay_color {
                    Some(overlay_color) if overlay_color.to_argb32() == 0 => Some(Rc::new(
                        WidgetStatePropertyAll::new(Some(crate::colors::Colors::TRANSPARENT)),
                    )),
                    _ => Some(SegmentedButtonDefaultsM3::resolve_state_color(
                        self.foreground_color,
                        self.selected_foreground_color,
                        self.overlay_color,
                    )),
                }
            };
        let mut text_button_style = TextButton::style_from();
        macro_rules! forward {
            ($($field:ident),* $(,)?) => {
                $(
                    if let Some($field) = self.$field.clone() {
                        text_button_style = text_button_style.$field($field);
                    }
                )*
            };
        }
        forward!(
            text_style,
            shadow_color,
            surface_tint_color,
            icon_color,
            icon_size,
            disabled_icon_color,
            elevation,
            padding,
            minimum_size,
            fixed_size,
            maximum_size,
            side,
            shape,
            enabled_mouse_cursor,
            disabled_mouse_cursor,
            visual_density,
            tap_target_size,
            animation_duration,
            enable_feedback,
            alignment,
            splash_factory,
        );
        let mut style = text_button_style.build();
        if let Some(foreground_color) = default_color(
            self.foreground_color,
            self.disabled_foreground_color,
            self.selected_foreground_color,
        ) {
            style = style.foreground_color(foreground_color);
        }
        if let Some(background_color) = default_color(
            self.background_color,
            self.disabled_background_color,
            self.selected_background_color,
        ) {
            style = style.background_color(background_color);
        }
        if let Some(overlay_color) = overlay_color_prop {
            style = style.overlay_color(overlay_color);
        }
        style
    }
}

/// Dart's `SegmentedButton._defaultColor`.
fn default_color(
    enabled: Option<Color>,
    disabled: Option<Color>,
    selected: Option<Color>,
) -> Option<WidgetStatePropertyRef<Option<Color>>> {
    selected.or(enabled).or(disabled)?;
    Some(<dyn WidgetStateProperty<Option<Color>>>::from_map(vec![
        (WidgetState::Disabled.into(), disabled),
        (WidgetState::Selected.into(), selected),
        (WidgetState::any(), enabled),
    ]))
}

impl<T> Clone for SegmentedButton<T>
where
    T: Clone,
{
    fn clone(&self) -> SegmentedButton<T> {
        SegmentedButton {
            key: self.key.clone(),
            segments: self.segments.clone(),
            direction: self.direction,
            selected: self.selected.clone(),
            on_selection_changed: self.on_selection_changed.clone(),
            multi_selection_enabled: self.multi_selection_enabled,
            empty_selection_allowed: self.empty_selection_allowed,
            expanded_insets: self.expanded_insets,
            style: self.style.clone(),
            show_selected_icon: self.show_selected_icon,
            selected_icon: self.selected_icon.clone(),
        }
    }
}

impl<T: Debug> Debug for SegmentedButton<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SegmentedButton")
            .field("segments", &self.segments)
            .field("selected", &self.selected)
            .field("direction", &self.direction)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + Debug + 'static> StatefulWidget for SegmentedButton<T> {
    type State = SegmentedButtonState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn debug_check_constructor(&self) {
        debug_assert!(
            self.segments
                .iter()
                .all(|segment| segment.icon.is_some() || segment.label.is_some())
        );
        debug_assert!(!self.segments.is_empty());
        debug_assert!(!self.selected.is_empty() || self.empty_selection_allowed);
        debug_assert!(self.selected.len() < 2 || self.multi_selection_enabled);
    }

    fn create_state(&self) -> SegmentedButtonState<T> {
        SegmentedButtonState {
            state: StateData::new(),
            hovering: false,
            focused: false,
            states_controllers: Vec::new(),
        }
    }
}

/// State for [`SegmentedButton`].
///
/// Dart marks the class `@visibleForTesting`.
pub struct SegmentedButtonState<T: Clone + PartialEq + Debug + 'static> {
    state: StateData<SegmentedButton<T>>,
    hovering: bool,
    focused: bool,

    /// Controllers for the [`ButtonSegment`]s, in the order they were first built, each
    /// keyed by its segment.
    ///
    /// Dart's `@visibleForTesting` map.
    pub states_controllers: Vec<(ButtonSegment<T>, Ref<WidgetStatesController>)>,
}

impl<T: Clone + PartialEq + Debug + 'static> Debug for SegmentedButtonState<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SegmentedButtonState")
            .field("hovering", &self.hovering)
            .field("focused", &self.focused)
            .finish_non_exhaustive()
    }
}

/// What the three layers of Dart's `build` read a style property from.
struct SegmentedButtonStyles {
    widget: Option<ButtonStyle>,
    theme: Option<ButtonStyle>,
    defaults: SegmentedButtonDefaultsM3,
    states: WidgetStates,
}

impl SegmentedButtonStyles {
    /// Dart's `effectiveValue`: the widget's, else the theme's, else the defaults' property,
    /// each read only when the one before is null.
    fn effective_value<P>(
        &self,
        app: &mut App,
        get_property: impl Fn(Option<&ButtonStyle>) -> Option<P>,
    ) -> Option<P> {
        get_property(self.widget.as_ref())
            .or_else(|| get_property(self.theme.as_ref()))
            .or_else(|| {
                let defaults_style = self.defaults.style(app);
                get_property(defaults_style.as_ref())
            })
    }

    /// Dart's `resolve`: the property of the layer that has one, resolved in `states`, or in
    /// the state's own states.
    fn resolve<P: 'static>(
        &self,
        app: &mut App,
        get_property: impl Fn(&ButtonStyle) -> Option<&WidgetStatePropertyRef<Option<P>>>,
        states: Option<&WidgetStates>,
    ) -> Option<P> {
        let states = states.unwrap_or(&self.states);
        self.effective_value(app, |style| {
            get_property(style?).and_then(|property| property.resolve(states))
        })
    }
}

/// Dart's `setEquals`.
fn set_equals<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    a.len() == b.len() && a.iter().all(|item| b.contains(item))
}

impl<T: Clone + PartialEq + Debug + 'static> SegmentedButtonState<T> {
    fn enabled(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).on_selection_changed.is_some()
    }

    fn selected(self: Handle<Self>, app: &App) -> bool {
        !self.widget(app).selected.is_empty()
    }

    fn states(self: Handle<Self>, app: &App) -> WidgetStates {
        let mut states = WidgetStates::new();
        if !self.enabled(app) {
            states.insert(WidgetState::Disabled);
        }
        if app.get(self).hovering {
            states.insert(WidgetState::Hovered);
        }
        if app.get(self).focused {
            states.insert(WidgetState::Focused);
        }
        if self.selected(app) {
            states.insert(WidgetState::Selected);
        }
        states
    }

    fn handle_on_pressed(self: Handle<Self>, app: &mut App, segment_value: T) {
        if !self.enabled(app) {
            return;
        }
        let widget = self.widget(app).clone();
        let only_selected_segment =
            widget.selected.len() == 1 && widget.selected.contains(&segment_value);
        let valid_change = widget.empty_selection_allowed || !only_selected_segment;
        if valid_change {
            let toggle = widget.multi_selection_enabled
                || (widget.empty_selection_allowed && only_selected_segment);
            let updated_selection: Vec<T> = if toggle {
                if widget.selected.contains(&segment_value) {
                    widget
                        .selected
                        .iter()
                        .filter(|value| **value != segment_value)
                        .cloned()
                        .collect()
                } else {
                    let mut union = widget.selected.clone();
                    union.push(segment_value);
                    union
                }
            } else {
                vec![segment_value]
            };
            if !set_equals(&updated_selection, &widget.selected)
                && let Some(on_selection_changed) = &widget.on_selection_changed
            {
                on_selection_changed(app, updated_selection);
            }
        }
    }

    /// Dart's `statesControllers.putIfAbsent(segment, () => MaterialStatesController())`.
    fn states_controller_for(
        self: Handle<Self>,
        app: &mut App,
        segment: &ButtonSegment<T>,
    ) -> Ref<WidgetStatesController> {
        let existing = app
            .get(self)
            .states_controllers
            .iter()
            .find(|(key, _)| key.value == segment.value)
            .map(|(_, controller)| controller.clone());
        if let Some(controller) = existing {
            return controller;
        }
        let controller = WidgetStatesController::new(app, None);
        app.get_mut(self)
            .states_controllers
            .push((segment.clone(), controller.clone()));
        controller
    }

    fn button_for(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        segment: &ButtonSegment<T>,
        segment_style: &ButtonStyle,
        selected_icon: &Option<WidgetRef>,
    ) -> WidgetRef {
        let widget = self.widget(app).clone();
        let label = segment
            .label
            .clone()
            .or_else(|| segment.icon.clone())
            .unwrap_or_else(|| SizedBox::shrink().into_widget());
        let segment_selected = widget.selected.contains(&segment.value);
        let icon = if segment_selected && widget.show_selected_icon {
            selected_icon.clone()
        } else if segment.label.is_some() {
            segment.icon.clone()
        } else {
            None
        };
        let controller = self.states_controller_for(app, segment);
        controller.update(app, WidgetState::Selected, segment_selected);

        let mut content = label.clone();
        let mut effective_segment_style = segment_style.clone();
        if let Some(icon) = icon {
            // This logic is needed to get the exact same rendering as using TextButton.icon.
            // It is duplicated from _TextButtonWithIcon and _TextButtonWithIconChild.
            // TODO(bleroux): remove once https://github.com/flutter/flutter/issues/173944 is fixed.
            let use_material3 = Theme::of(app, context).use_material3;
            let default_font_size = segment_style
                .text_style
                .as_ref()
                .and_then(|text_style| text_style.resolve(&WidgetStates::new()))
                .and_then(|text_style| text_style.font_size)
                .unwrap_or(14.0);
            let effective_text_scale =
                MediaQuery::text_scaler_of(app, context).scale(default_font_size) / 14.0;
            let scaled_padding = <dyn ButtonStyleButton>::scaled_padding(
                if use_material3 {
                    EdgeInsetsDirectional::from_steb(12.0, 8.0, 16.0, 8.0).into()
                } else {
                    EdgeInsets::all(8.0).into()
                },
                EdgeInsets::symmetric(4.0, 0.0).into(),
                EdgeInsets::symmetric(4.0, 0.0).into(),
                effective_text_scale,
            );
            effective_segment_style = segment_style
                .copy_with()
                .padding(Rc::new(WidgetStatePropertyAll::new(Some(scaled_padding))));
            let scale = clamp_double(effective_text_scale, 1.0, 2.0) - 1.0;
            let text_button_theme = TextButtonTheme::of(app, context);
            let effective_icon_alignment = text_button_theme
                .style
                .as_ref()
                .and_then(|style| style.icon_alignment)
                .or(segment_style.icon_alignment)
                .unwrap_or(IconAlignment::Start);
            content = Row::new()
                .main_axis_size(MainAxisSize::Min)
                .spacing(waft_embedder::lerp_double(Some(8.0), Some(4.0), scale).expect("a value"))
                .children(if effective_icon_alignment == IconAlignment::Start {
                    vec![icon, Flexible::new(label).into_widget()]
                } else {
                    vec![Flexible::new(label).into_widget(), icon]
                })
                .into_widget();
        }

        let enabled = self.enabled(app);
        let on_pressed = (enabled && segment.enabled).then(|| {
            let value = segment.value.clone();
            Listener::new(move |app| self.handle_on_pressed(app, value.clone()))
        });
        let button = TextButton::new(on_pressed, Some(content))
            .style(effective_segment_style)
            .states_controller(controller)
            .on_hover(Rc::new(move |app, hovering| {
                self.set_state(app, |state| state.hovering = hovering);
            }))
            .on_focus_change(Rc::new(move |app, focused| {
                self.set_state(app, |state| state.focused = focused);
            }))
            .into_widget();

        let button_with_tooltip = match &segment.tooltip {
            Some(tooltip) => Tooltip::new()
                .message(tooltip.clone())
                .child(button)
                .into_widget(),
            None => button,
        };

        let mut semantics = Semantics::new().selected(segment_selected);
        if !widget.multi_selection_enabled {
            semantics = semantics.in_mutually_exclusive_group(true);
        }
        MergeSemantics::new()
            .child(semantics.child(button_with_tooltip))
            .into_widget()
    }
}

impl<T: Clone + PartialEq + Debug + 'static> State for SegmentedButtonState<T> {
    type Widget = SegmentedButton<T>;
    waft_widgets::state_accessors!();

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &SegmentedButton<T>) {
        if !std::ptr::eq(old_widget, self.widget(app)) {
            let segments = self.widget(app).segments.clone();
            let controllers = std::mem::take(&mut app.get_mut(self).states_controllers);
            let mut kept = Vec::new();
            for (segment, controller) in controllers {
                if segments.iter().any(|other| other.value == segment.value) {
                    kept.push((segment, controller));
                } else {
                    controller.dispose(app);
                }
            }
            app.get_mut(self).states_controllers = kept;
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let theme = SegmentedButtonTheme::of(app, context);
        let defaults = SegmentedButtonDefaultsM3::new(context);
        let text_direction = Directionality::of(app, context);
        let disabled_state: WidgetStates = [WidgetState::Disabled].into_iter().collect();
        let widget = self.widget(app).clone();
        let styles = SegmentedButtonStyles {
            widget: widget.style.clone(),
            theme: theme.style.clone(),
            defaults,
            states: self.states(app),
        };

        let segment_style = segment_style_for(widget.style.as_ref());
        let defaults_style = styles.defaults.style(app);
        let segment_theme_style = segment_style_for(theme.style.as_ref())
            .merge(Some(&segment_style_for(defaults_style.as_ref())));
        let selected_icon = if widget.show_selected_icon {
            widget
                .selected_icon
                .clone()
                .or_else(|| theme.selected_icon.clone())
                .or_else(|| styles.defaults.selected_icon(app))
        } else {
            None
        };

        let effective_border = styles
            .resolve(app, |style| style.shape.as_ref(), None)
            .unwrap_or_else(|| OutlinedBorderRef::new(RoundedRectangleBorder::default()));
        let resolved_disabled_border = styles
            .resolve(app, |style| style.shape.as_ref(), Some(&disabled_state))
            .unwrap_or_else(|| OutlinedBorderRef::new(RoundedRectangleBorder::default()));
        let effective_side = styles
            .resolve(app, |style| style.side.as_ref(), None)
            .unwrap_or(BorderSide::NONE);
        let disabled_side = styles
            .resolve(app, |style| style.side.as_ref(), Some(&disabled_state))
            .unwrap_or(BorderSide::NONE);

        let enabled_border = effective_border.copy_with(Some(effective_side));
        let disabled_border = resolved_disabled_border.copy_with(Some(disabled_side));
        let resolved_visual_density = match segment_style
            .visual_density
            .or(segment_theme_style.visual_density)
        {
            Some(visual_density) => visual_density,
            None => Theme::of(app, context).visual_density,
        };
        let resolved_padding = styles
            .resolve(app, |style| style.padding.as_ref(), None)
            .unwrap_or(EdgeInsetsGeometry::ZERO);
        let resolved_tap_target_size = match segment_style
            .tap_target_size
            .or(segment_theme_style.tap_target_size)
        {
            Some(tap_target_size) => tap_target_size,
            None => Theme::of(app, context).material_tap_target_size,
        };
        let font_size = styles
            .resolve(app, |style| style.text_style.as_ref(), None)
            .and_then(|text_style| text_style.font_size)
            .unwrap_or(20.0);

        let buttons: Vec<WidgetRef> = widget
            .segments
            .iter()
            .map(|segment| self.button_for(app, context, segment, &segment_style, &selected_icon))
            .collect();

        let density_adjustment = resolved_visual_density.base_size_adjustment();
        const TEXT_BUTTON_MIN_HEIGHT: f64 = 40.0;

        let adjust_button_min_height = TEXT_BUTTON_MIN_HEIGHT + density_adjustment.dy();
        let effective_vertical_padding =
            resolved_padding.vertical() + density_adjustment.dy() * 2.0;
        let effected_button_height =
            (font_size + effective_vertical_padding).max(adjust_button_min_height);
        let tap_target_vertical_padding = match resolved_tap_target_size {
            MaterialTapTargetSize::ShrinkWrap => 0.0,
            MaterialTapTargetSize::Padded => {
                (K_MIN_INTERACTIVE_DIMENSION + density_adjustment.dy() - effected_button_height)
                    .max(0.0)
            }
        };

        let enabled = self.enabled(app);
        let mut material = Material::new()
            .r#type(MaterialType::Transparency)
            .elevation(
                styles
                    .resolve(app, |style| style.elevation.as_ref(), None)
                    .expect("the default style has an elevation"),
            );
        if let Some(shadow_color) = styles.resolve(app, |style| style.shadow_color.as_ref(), None) {
            material = material.shadow_color(shadow_color);
        }
        if let Some(surface_tint_color) =
            styles.resolve(app, |style| style.surface_tint_color.as_ref(), None)
        {
            material = material.surface_tint_color(surface_tint_color);
        }
        material
            .child(TextButtonTheme::new(
                TextButtonThemeData::new().style(segment_theme_style),
                Padding::new(
                    widget
                        .expanded_insets
                        .map_or(EdgeInsetsGeometry::ZERO, Into::into),
                )
                .child(SegmentedButtonRenderWidget {
                    segments: widget.segments.clone(),
                    enabled_border: if enabled {
                        enabled_border
                    } else {
                        disabled_border.clone()
                    },
                    disabled_border,
                    direction: widget.direction,
                    text_direction,
                    tap_target_vertical_padding,
                    is_expanded: widget.expanded_insets.is_some(),
                    children: buttons,
                }),
            ))
            .into_widget()
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let controllers = std::mem::take(&mut app.get_mut(self).states_controllers);
        for (_, controller) in controllers {
            controller.dispose(app);
        }
    }
}

/// Dart's `segmentStyleFor`.
fn segment_style_for(style: Option<&ButtonStyle>) -> ButtonStyle {
    ButtonStyle {
        text_style: style.and_then(|style| style.text_style.clone()),
        background_color: style.and_then(|style| style.background_color.clone()),
        foreground_color: style.and_then(|style| style.foreground_color.clone()),
        overlay_color: style.and_then(|style| style.overlay_color.clone()),
        surface_tint_color: style.and_then(|style| style.surface_tint_color.clone()),
        elevation: style.and_then(|style| style.elevation.clone()),
        padding: style.and_then(|style| style.padding.clone()),
        icon_color: style.and_then(|style| style.icon_color.clone()),
        icon_size: style.and_then(|style| style.icon_size.clone()),
        shape: Some(Rc::new(WidgetStatePropertyAll::new(Some(
            OutlinedBorderRef::new(RoundedRectangleBorder::default()),
        )))),
        mouse_cursor: style.and_then(|style| style.mouse_cursor.clone()),
        visual_density: style.and_then(|style| style.visual_density),
        tap_target_size: style.and_then(|style| style.tap_target_size),
        animation_duration: style.and_then(|style| style.animation_duration),
        enable_feedback: style.and_then(|style| style.enable_feedback),
        alignment: style.and_then(|style| style.alignment),
        splash_factory: style.and_then(|style| style.splash_factory.clone()),
        ..ButtonStyle::new()
    }
}

/// Dart's `_SegmentedButtonRenderWidget`.
struct SegmentedButtonRenderWidget<T> {
    segments: Vec<ButtonSegment<T>>,
    enabled_border: OutlinedBorderRef,
    disabled_border: OutlinedBorderRef,
    direction: Axis,
    text_direction: TextDirection,
    tap_target_vertical_padding: f64,
    is_expanded: bool,
    children: Vec<WidgetRef>,
}

impl<T: Debug> Debug for SegmentedButtonRenderWidget<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SegmentedButtonRenderWidget")
            .field("segments", &self.segments)
            .field("direction", &self.direction)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + PartialEq + Debug + 'static> RenderObjectWidget for SegmentedButtonRenderWidget<T> {
    type RenderObject = RenderSegmentedButton<T>;

    fn debug_check_constructor(&self) {
        debug_assert_eq!(self.children.len(), self.segments.len());
    }

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderSegmentedButton::new(
            app,
            self.segments.clone(),
            self.enabled_border.clone(),
            self.disabled_border.clone(),
            self.text_direction,
            self.tap_target_vertical_padding,
            self.is_expanded,
            self.direction,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderSegmentedButton<T>>,
    ) {
        render_object.set_segments(app, self.segments.clone());
        render_object.set_enabled_border(app, self.enabled_border.clone());
        render_object.set_disabled_border(app, self.disabled_border.clone());
        render_object.set_direction(app, self.direction);
        render_object.set_text_direction(app, self.text_direction);
    }
}

impl<T: Clone + PartialEq + Debug + 'static> MultiChildRenderObjectWidget
    for SegmentedButtonRenderWidget<T>
{
    fn children(&self) -> &[WidgetRef] {
        &self.children
    }
}

/// Dart's `_SegmentedButtonContainerBoxParentData`.
#[derive(Debug, Default)]
struct SegmentedButtonContainerBoxParentData {
    box_parent_data: BoxParentData,
    container_parent_data: ContainerParentData<AnyRenderBox>,
    surrounding_rect: Option<RRect>,
}

impl SegmentedButtonContainerBoxParentData {
    const fn new() -> SegmentedButtonContainerBoxParentData {
        SegmentedButtonContainerBoxParentData {
            box_parent_data: BoxParentData::new(),
            container_parent_data: ContainerParentData::new(),
            surrounding_rect: None,
        }
    }
}

impl ParentData for SegmentedButtonContainerBoxParentData {
    fn detach(&mut self) {
        ContainerParentDataMixin::detach(self);
    }

    fn provide(&self, id: TypeId) -> Option<&dyn Any> {
        if id == TypeId::of::<SegmentedButtonContainerBoxParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide(id)
    }

    fn provide_mut(&mut self, id: TypeId) -> Option<&mut dyn Any> {
        if id == TypeId::of::<SegmentedButtonContainerBoxParentData>() {
            return Some(self);
        }
        self.box_parent_data.provide_mut(id)
    }
}

impl ContainerParentDataMixin for SegmentedButtonContainerBoxParentData {
    type ChildType = AnyRenderBox;

    fn container_parent_data(&self) -> &ContainerParentData<AnyRenderBox> {
        &self.container_parent_data
    }

    fn container_parent_data_mut(&mut self) -> &mut ContainerParentData<AnyRenderBox> {
        &mut self.container_parent_data
    }
}

impl ContainerBoxParentData for SegmentedButtonContainerBoxParentData {
    fn box_parent_data(&self) -> &BoxParentData {
        &self.box_parent_data
    }

    fn box_parent_data_mut(&mut self) -> &mut BoxParentData {
        &mut self.box_parent_data
    }
}

impl fmt::Display for SegmentedButtonContainerBoxParentData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.box_parent_data, f)
    }
}

/// Dart's `_NextChild`.
type NextChild<T> =
    fn(RenderHandle<RenderSegmentedButton<T>>, &App, AnyRenderBox) -> Option<AnyRenderBox>;

/// Dart's `_RenderSegmentedButton`.
struct RenderSegmentedButton<T> {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    container: ContainerRenderObjectData<AnyRenderBox>,
    segments: Vec<ButtonSegment<T>>,
    enabled_border: OutlinedBorderRef,
    disabled_border: OutlinedBorderRef,
    text_direction: TextDirection,
    direction: Axis,
    tap_target_vertical_padding: f64,
    is_expanded: bool,
}

impl<T: Clone + PartialEq + Debug + 'static> RenderSegmentedButton<T> {
    #[expect(
        clippy::too_many_arguments,
        reason = "Dart's constructor takes them as named arguments"
    )]
    fn new(
        app: &mut App,
        segments: Vec<ButtonSegment<T>>,
        enabled_border: OutlinedBorderRef,
        disabled_border: OutlinedBorderRef,
        text_direction: TextDirection,
        tap_target_vertical_padding: f64,
        is_expanded: bool,
        direction: Axis,
    ) -> RenderHandle<RenderSegmentedButton<T>> {
        RenderHandle::new_box(
            app,
            RenderSegmentedButton {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                container: ContainerRenderObjectData::new(),
                segments,
                enabled_border,
                disabled_border,
                text_direction,
                direction,
                tap_target_vertical_padding,
                is_expanded,
            },
        )
    }

    fn set_segments(self: RenderHandle<Self>, app: &mut App, value: Vec<ButtonSegment<T>>) {
        if self.get(app).segments == value {
            return;
        }
        self.get_mut(app).segments = value;
        self.mark_needs_layout(app);
    }

    fn set_enabled_border(self: RenderHandle<Self>, app: &mut App, value: OutlinedBorderRef) {
        if self.get(app).enabled_border == value {
            return;
        }
        self.get_mut(app).enabled_border = value;
        self.mark_needs_layout(app);
    }

    fn set_disabled_border(self: RenderHandle<Self>, app: &mut App, value: OutlinedBorderRef) {
        if self.get(app).disabled_border == value {
            return;
        }
        self.get_mut(app).disabled_border = value;
        self.mark_needs_layout(app);
    }

    fn text_direction(self: RenderHandle<Self>, app: &App) -> TextDirection {
        self.get(app).text_direction
    }

    fn set_text_direction(self: RenderHandle<Self>, app: &mut App, value: TextDirection) {
        if value == self.get(app).text_direction {
            return;
        }
        self.get_mut(app).text_direction = value;
        self.mark_needs_layout(app);
    }

    fn direction(self: RenderHandle<Self>, app: &App) -> Axis {
        self.get(app).direction
    }

    fn set_direction(self: RenderHandle<Self>, app: &mut App, value: Axis) {
        if value == self.get(app).direction {
            return;
        }
        self.get_mut(app).direction = value;
        self.mark_needs_layout(app);
    }

    fn is_expanded(self: RenderHandle<Self>, app: &App) -> bool {
        self.get(app).is_expanded
    }

    /// Dart's `computeMinIntrinsicWidth`, `computeMaxIntrinsicWidth`,
    /// `computeMinIntrinsicHeight` and `computeMaxIntrinsicHeight` bodies, which differ only in
    /// the child intrinsic they read.
    fn max_child_intrinsic(
        self: RenderHandle<Self>,
        app: &mut App,
        extent: f64,
        child_intrinsic: fn(AnyRenderBox, &mut App, f64) -> f64,
    ) -> f64 {
        let mut child = self.first_child(app);
        let mut max_extent: f64 = 0.0;
        while let Some(current) = child {
            let next_sibling = current
                .as_object()
                .parent_data_of::<SegmentedButtonContainerBoxParentData>(app)
                .next_sibling();
            let child_extent = child_intrinsic(current, app, extent);
            max_extent = max_extent.max(child_extent);
            child = next_sibling;
        }
        max_extent
    }

    fn layout_rects(
        self: RenderHandle<Self>,
        app: &mut App,
        next_child: NextChild<T>,
        left_child: Option<AnyRenderBox>,
        _right_child: Option<AnyRenderBox>,
    ) {
        let mut child = left_child;
        let mut start = 0.0;
        let direction = self.direction(app);
        while let Some(current) = child {
            let child_size = current.size(app);
            let (offset, r_child_rect);
            if direction == Axis::Vertical {
                offset = Offset::new(0.0, start);
                let child_rect =
                    Rect::from_ltwh(0.0, offset.dy(), child_size.width(), child_size.height());
                r_child_rect = RRect::from_rect_and_corners(
                    child_rect,
                    Radius::ZERO,
                    Radius::ZERO,
                    Radius::ZERO,
                    Radius::ZERO,
                );
                start += child_size.height();
            } else {
                offset = Offset::new(start, 0.0);
                let child_rect =
                    Rect::from_ltwh(start, 0.0, child_size.width(), child_size.height());
                r_child_rect = RRect::from_rect_and_corners(
                    child_rect,
                    Radius::ZERO,
                    Radius::ZERO,
                    Radius::ZERO,
                    Radius::ZERO,
                );
                start += child_size.width();
            }
            let child_parent_data = current
                .as_object()
                .parent_data_of_mut::<SegmentedButtonContainerBoxParentData>(app);
            child_parent_data.set_offset(offset);
            child_parent_data.surrounding_rect = Some(r_child_rect);
            child = next_child(self, app, current);
        }
    }

    fn calculate_child_size(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        if self.direction(app) == Axis::Horizontal {
            self.calculate_horizontal_child_size(app, constraints)
        } else {
            self.calculate_vertical_child_size(app, constraints)
        }
    }

    fn calculate_horizontal_child_size(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let mut max_height: f64 = 0.0;
        let mut child = self.first_child(app);
        let child_count = self.child_count(app) as f64;
        let mut child_width;
        if self.is_expanded(app) {
            child_width = constraints.max_width / child_count;
        } else {
            child_width = constraints.min_width / child_count;
            while let Some(current) = child {
                child_width = child_width.max(current.get_max_intrinsic_width(app, f64::INFINITY));
                child = self.child_after(app, current);
            }
            child_width = child_width.min(constraints.max_width / child_count);
        }
        child = self.first_child(app);
        while let Some(current) = child {
            let box_height = current.get_max_intrinsic_height(app, child_width);
            max_height = max_height.max(box_height);
            child = self.child_after(app, current);
        }
        Size::new(child_width, max_height)
    }

    fn calculate_vertical_child_size(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let mut max_width: f64 = 0.0;
        let mut child = self.first_child(app);
        let child_count = self.child_count(app) as f64;
        let mut child_height;
        if self.is_expanded(app) {
            child_height = constraints.max_height / child_count;
        } else {
            child_height = constraints.min_height / child_count;
            while let Some(current) = child {
                child_height =
                    child_height.max(current.get_max_intrinsic_height(app, f64::INFINITY));
                child = self.child_after(app, current);
            }
            child_height = child_height.min(constraints.max_height / child_count);
        }
        child = self.first_child(app);
        while let Some(current) = child {
            let box_width = current.get_max_intrinsic_width(app, max_width);
            max_width = max_width.max(box_width);
            child = self.child_after(app, current);
        }

        let mut child_size = Size::new(max_width, child_height);

        // When the parent provides a tight width constraint in the vertical
        // direction, use that width for layout so the visual size and the
        // interactive area match. This preserves the intrinsic (shrink-wrap)
        // sizing behavior when the width is unconstrained.
        if constraints.has_tight_width() && child_size.width() < constraints.max_width {
            child_size = Size::new(constraints.max_width, child_size.height());
        }

        child_size
    }

    fn compute_overall_size_from_child_size(
        self: RenderHandle<Self>,
        app: &App,
        child_size: Size,
    ) -> Size {
        let constraints = self.constraints(app);
        let child_count = self.child_count(app) as f64;
        if self.direction(app) == Axis::Vertical {
            return constraints.constrain(Size::new(
                child_size.width(),
                child_size.height() * child_count,
            ));
        }
        constraints.constrain(Size::new(
            child_size.width() * child_count,
            child_size.height(),
        ))
    }
}

impl<T: Clone + PartialEq + Debug + 'static> ContainerRenderObjectMixin
    for RenderSegmentedButton<T>
{
    type ChildType = AnyRenderBox;
    type ParentDataType = SegmentedButtonContainerBoxParentData;

    fn container_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &ContainerRenderObjectData<AnyRenderBox> {
        &self.get(app).container
    }

    fn container_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut ContainerRenderObjectData<AnyRenderBox> {
        &mut self.get_mut(app).container
    }
}

impl<T: Clone + PartialEq + Debug + 'static> RenderBoxContainerDefaultsMixin
    for RenderSegmentedButton<T>
{
}

impl<T: Clone + PartialEq + Debug + 'static> RenderObject for RenderSegmentedButton<T> {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let child_size = self.calculate_child_size(app, constraints);

        let child_constraints =
            BoxConstraints::tight_for(Some(child_size.width()), Some(child_size.height()));

        let mut child = self.first_child(app);
        while let Some(current) = child {
            current.layout(app, child_constraints, true);
            child = self.child_after(app, current);
        }

        match self.text_direction(app) {
            TextDirection::Rtl => {
                let (last_child, first_child) = (self.last_child(app), self.first_child(app));
                self.layout_rects(
                    app,
                    |this, app, child| this.child_before(app, child),
                    last_child,
                    first_child,
                );
            }
            TextDirection::Ltr => {
                let (first_child, last_child) = (self.first_child(app), self.last_child(app));
                self.layout_rects(
                    app,
                    |this, app, child| this.child_after(app, child),
                    first_child,
                    last_child,
                );
            }
        }

        let size = self.compute_overall_size_from_child_size(app, child_size);
        self.set_size(app, size);
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let size = self.size(app);
        let text_direction = self.text_direction(app);
        let tap_target_vertical_padding = self.get(app).tap_target_vertical_padding;
        let enabled_border = self.get(app).enabled_border.clone();
        let disabled_border = self.get(app).disabled_border.clone();
        let direction = self.direction(app);
        let segments = self.get(app).segments.clone();
        let border_rect = Rect::from_ltwh(
            offset.dx(),
            offset.dy() + tap_target_vertical_padding / 2.0,
            size.width(),
            size.height() - tap_target_vertical_padding,
        );
        let border_clip_path = enabled_border.get_inner_path(border_rect, Some(text_direction));
        let (first_child, last_child) = (self.first_child(app), self.last_child(app));
        let mut child = first_child;
        let mut previous_child: Option<AnyRenderBox> = None;
        let mut index = 0;
        let mut enabled_clip_path: Option<PathBuilder> = None;
        let mut disabled_clip_path: Option<PathBuilder> = None;

        while let Some(current) = child {
            let child_parent_data = current
                .as_object()
                .parent_data_of::<SegmentedButtonContainerBoxParentData>(app);
            let child_offset = child_parent_data.offset();
            let child_rect = child_parent_data
                .surrounding_rect
                .expect("laid out")
                .outer_rect()
                .shift(offset);

            let canvas = context.canvas();
            canvas.save();
            canvas.clip_path(
                border_clip_path.as_valo(),
                border_clip_path.fill_type,
                waft_embedder::ClipOp::Intersect,
                true,
            );
            context.paint_child(app, current.as_object(), child_offset + offset);
            context.canvas().restore();

            // Compute a clip rect for the outer border of the child.
            let (segment_left, segment_right, divider_pos);
            let border_outset = enabled_border
                .side()
                .stroke_outset()
                .max(disabled_border.side().stroke_outset());
            match text_direction {
                TextDirection::Rtl => {
                    segment_left = if Some(current) == last_child {
                        border_rect.left - border_outset
                    } else {
                        child_rect.left
                    };
                    segment_right = if Some(current) == first_child {
                        border_rect.right + border_outset
                    } else {
                        child_rect.right
                    };
                    divider_pos = segment_right;
                }
                TextDirection::Ltr => {
                    segment_left = if Some(current) == first_child {
                        border_rect.left - border_outset
                    } else {
                        child_rect.left
                    };
                    segment_right = if Some(current) == last_child {
                        border_rect.right + border_outset
                    } else {
                        child_rect.right
                    };
                    divider_pos = segment_left;
                }
            }
            let segment_clip_rect = Rect::from_ltrb(
                segment_left,
                border_rect.top - border_outset,
                segment_right,
                border_rect.bottom + border_outset,
            );

            // Add the clip rect to the appropriate border clip path
            if segments[index].enabled {
                enabled_clip_path
                    .get_or_insert_with(PathBuilder::new)
                    .rect(segment_clip_rect.into());
            } else {
                disabled_clip_path
                    .get_or_insert_with(PathBuilder::new)
                    .rect(segment_clip_rect.into());
            }

            // Paint the divider between this segment and the previous one.
            if previous_child.is_some() {
                let divider = if segments[index - 1].enabled || segments[index].enabled {
                    enabled_border.side().copy_with(None, None, None, Some(0.0))
                } else {
                    disabled_border
                        .side()
                        .copy_with(None, None, None, Some(0.0))
                };
                if direction == Axis::Horizontal {
                    let top = Offset::new(divider_pos, border_rect.top);
                    let bottom = Offset::new(divider_pos, border_rect.bottom);
                    draw_line(context.canvas(), top, bottom, &divider.to_paint());
                } else if direction == Axis::Vertical {
                    let start = Offset::new(border_rect.left, child_rect.top);
                    let end = Offset::new(border_rect.right, child_rect.top);
                    let canvas = context.canvas();
                    canvas.save();
                    canvas.clip_path(
                        border_clip_path.as_valo(),
                        border_clip_path.fill_type,
                        waft_embedder::ClipOp::Intersect,
                        true,
                    );
                    draw_line(canvas, start, end, &divider.to_paint());
                    canvas.restore();
                }
            }

            previous_child = Some(current);
            child = self.child_after(app, current);
            index += 1;
        }

        // Paint the outer border for both disabled and enabled clip rect if needed.
        let canvas = context.canvas();
        match (enabled_clip_path, disabled_clip_path) {
            (_, None) => {
                // Just paint the enabled border with no clip.
                enabled_border.paint(canvas, border_rect, Some(text_direction));
            }
            (None, Some(_)) => {
                // Just paint the disabled border with no.
                disabled_border.paint(canvas, border_rect, Some(text_direction));
            }
            (Some(enabled_clip_path), Some(disabled_clip_path)) => {
                // Paint both of them clipped appropriately for the children segments.
                let enabled_clip_path: waft_embedder::Path = enabled_clip_path.build().into();
                let disabled_clip_path: waft_embedder::Path = disabled_clip_path.build().into();
                canvas.save();
                canvas.clip_path(
                    enabled_clip_path.as_valo(),
                    enabled_clip_path.fill_type,
                    waft_embedder::ClipOp::Intersect,
                    true,
                );
                enabled_border.paint(canvas, border_rect, Some(text_direction));
                canvas.restore();
                canvas.save();
                canvas.clip_path(
                    disabled_clip_path.as_valo(),
                    disabled_clip_path.fill_type,
                    waft_embedder::ClipOp::Intersect,
                    true,
                );
                disabled_border.paint(canvas, border_rect, Some(text_direction));
                canvas.restore();
            }
        }
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        ContainerRenderObjectMixin::visit_children(self, app, visitor)
    }

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        ContainerRenderObjectMixin::did_attach(self, app, owner)
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::did_detach(self, app)
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        ContainerRenderObjectMixin::redepth_children(self, app)
    }
}

impl<T: Clone + PartialEq + Debug + 'static> RenderBox for RenderSegmentedButton<T> {
    waft_rendering::render_box_accessors!();

    fn setup_parent_data(self: RenderHandle<Self>, app: &mut App, child: AnyRenderObject) {
        if !child.parent_data_is::<SegmentedButtonContainerBoxParentData>(app) {
            child.set_parent_data(app, SegmentedButtonContainerBoxParentData::new());
        }
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let min_width = self.max_child_intrinsic(app, height, |child, app, height| {
            child.get_min_intrinsic_width(app, height)
        });
        min_width * self.child_count(app) as f64
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let max_width = self.max_child_intrinsic(app, height, |child, app, height| {
            child.get_max_intrinsic_width(app, height)
        });
        max_width * self.child_count(app) as f64
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        self.max_child_intrinsic(app, width, |child, app, width| {
            child.get_min_intrinsic_height(app, width)
        })
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        self.max_child_intrinsic(app, width, |child, app, width| {
            child.get_max_intrinsic_height(app, width)
        })
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        self.default_compute_distance_to_highest_actual_baseline(app, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        let child_size = self.calculate_child_size(app, constraints);
        self.compute_overall_size_from_child_size(app, child_size)
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let child_size = self.calculate_child_size(app, constraints);
        let child_constraints = BoxConstraints::tight(child_size);

        let mut baseline_offset = BaselineOffset::NO_BASELINE;
        let mut child = self.first_child(app);
        while let Some(current) = child {
            baseline_offset = baseline_offset.min_of(BaselineOffset(current.get_dry_baseline(
                app,
                child_constraints,
                baseline,
            )));
            child = self.child_after(app, current);
        }
        baseline_offset.offset()
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let mut child = self.last_child(app);
        while let Some(current) = child {
            let child_parent_data = current
                .as_object()
                .parent_data_of::<SegmentedButtonContainerBoxParentData>(app);
            let offset = child_parent_data.offset();
            let surrounding_rect = child_parent_data.surrounding_rect.expect("laid out");
            let previous_sibling = child_parent_data.previous_sibling();
            if surrounding_rect.contains(position) {
                return result.add_with_paint_offset(
                    Some(offset),
                    position,
                    |result, local_offset| {
                        debug_assert_eq!(local_offset, position - offset);
                        current.hit_test(app, result, local_offset)
                    },
                );
            }
            child = previous_sibling;
        }
        false
    }
}

#[cfg(test)]
#[allow(deprecated, reason = "Dart's tests call `withOpacity`")]
pub(crate) mod tests {
    //! Ported from `test/segmented_button_test.dart`; the goldens, the semantics-tree cases and
    //! the `paints` cases that read an ordered call sequence are not ported.

    use std::cell::RefCell;

    use waft_services::{LogicalKeyboardKey, PhysicalKeyboardKey, SystemMouseCursors};
    use waft_widgets::{AnyElement, Center, Icon, IconData, Row, Text};

    use waft_widgets::testing::{TestPointer, WidgetTester, test_widgets};

    use super::*;
    use crate::icons::Icons;
    use crate::scaffold::Scaffold;
    use crate::test_support::{Drawn, PaintInspection, material_app};
    use crate::theme_data::ThemeData;
    use crate::tooltip::tests::m3;

    pub(crate) fn label(text: &str) -> Text {
        Text::new(text)
    }

    pub(crate) fn numbered(values: &[i32]) -> Vec<ButtonSegment<i32>> {
        values
            .iter()
            .map(|value| ButtonSegment::new(*value).label(label(&value.to_string())))
            .collect()
    }

    /// `MaterialApp(theme: theme, home: Scaffold(body: Center(child: button)))`.
    pub(crate) fn pump_button<T: Clone + PartialEq + Debug + 'static>(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        button: SegmentedButton<T>,
    ) {
        let root = {
            let app = tester.cell.borrow();
            material_app(
                theme(&app),
                Scaffold::new()
                    .body(Center::new().child(button))
                    .into_widget(),
            )
        };
        tester.pump_widget(root);
    }

    fn state_of(tester: &WidgetTester) -> Handle<SegmentedButtonState<i32>> {
        let element = tester.find_by_type::<SegmentedButton<i32>>()[0];
        element
            .state_handle::<SegmentedButtonState<i32>>(&tester.cell.borrow())
            .expect("a segmented button state")
    }

    fn tap_text(tester: &mut WidgetTester, text: &str) {
        let element = tester.find_text(text)[0];
        let center = tester.get_rect(tester.render_box_of(element)).center();
        tester.tap(center, 1);
    }

    // 'SegmentedButton releases state controllers for deleted segments'
    #[test]
    fn a_segmented_button_releases_the_state_controllers_of_deleted_segments() {
        test_widgets(|tester| {
            let key = waft_foundation::UniqueKey::new();
            pump_button(
                tester,
                m3,
                SegmentedButton::new(numbered(&[1, 2]), vec![2]).key(key.into()),
            );
            pump_button(
                tester,
                m3,
                SegmentedButton::new(numbered(&[2, 3]), vec![2]).key(key.into()),
            );

            let state = state_of(tester);
            let app = tester.cell.borrow();
            let controllers = &app.get(state).states_controllers;
            assert_eq!(controllers.len(), 2);
            assert_eq!(controllers.first().expect("a controller").0.value, 2);
            assert_eq!(controllers.last().expect("a controller").0.value, 3);
        });
    }

    // 'SegmentedButton is built with Material of type MaterialType.transparency'
    #[test]
    fn a_segmented_button_is_built_with_a_transparent_material() {
        test_widgets(|tester| {
            pump_button(
                tester,
                m3,
                SegmentedButton::new(
                    vec![
                        ButtonSegment::new(1).label(label("1")),
                        ButtonSegment::new(2).label(label("2")),
                        ButtonSegment::new(3).label(label("3")).enabled(false),
                    ],
                    vec![2],
                )
                .on_selection_changed(|_app, _selected| {}),
            );

            // Expect SegmentedButton to be built with type MaterialType.transparency.
            let text = tester.find_text("1")[0];
            let parent = tester.ancestor_of_type::<Material>(text);
            let parent_material = tester.ancestor_of_type::<Material>(parent);
            let material_type =
                tester.widget::<Material, _>(parent_material, |material| material.r#type);
            assert_eq!(material_type, MaterialType::Transparency);
        });
    }

    /// The state a pumped selection and a recording callback give.
    struct Recording {
        callback_count: Rc<RefCell<u32>>,
        selection: Rc<RefCell<Vec<i32>>>,
    }

    impl Recording {
        fn new(selection: Vec<i32>) -> Recording {
            Recording {
                callback_count: Rc::new(RefCell::new(0)),
                selection: Rc::new(RefCell::new(selection)),
            }
        }

        fn callback_count(&self) -> u32 {
            *self.callback_count.borrow()
        }

        fn selection(&self) -> Vec<i32> {
            self.selection.borrow().clone()
        }

        fn on_selection_changed(&self) -> impl Fn(&mut App, Vec<i32>) + 'static {
            let (callback_count, selection) =
                (Rc::clone(&self.callback_count), Rc::clone(&self.selection));
            move |_app, selected| {
                *selection.borrow_mut() = selected;
                *callback_count.borrow_mut() += 1;
            }
        }
    }

    fn sorted(mut values: Vec<i32>) -> Vec<i32> {
        values.sort();
        values
    }

    // 'SegmentedButton supports exclusive choice by default'
    #[test]
    fn a_segmented_button_supports_exclusive_choice_by_default() {
        test_widgets(|tester| {
            let recording = Recording::new(vec![2]);
            let frame_with_selection = |tester: &mut WidgetTester, selected: i32| {
                pump_button(
                    tester,
                    m3,
                    SegmentedButton::new(numbered(&[1, 2, 3]), vec![selected])
                        .on_selection_changed(recording.on_selection_changed()),
                );
            };

            frame_with_selection(tester, 2);
            assert_eq!(recording.selection(), [2]);
            assert_eq!(recording.callback_count(), 0);

            // Tap on segment 1.
            tap_text(tester, "1");
            tester.pump_and_settle();
            assert_eq!(recording.callback_count(), 1);
            assert_eq!(recording.selection(), [1]);

            // Update the selection in the widget
            frame_with_selection(tester, 1);

            // Tap on segment 1 again should do nothing.
            tap_text(tester, "1");
            tester.pump_and_settle();
            assert_eq!(recording.callback_count(), 1);
            assert_eq!(recording.selection(), [1]);

            // Tap on segment 3.
            tap_text(tester, "3");
            tester.pump_and_settle();
            assert_eq!(recording.callback_count(), 2);
            assert_eq!(recording.selection(), [3]);
        });
    }

    // 'SegmentedButton supports multiple selected segments'
    #[test]
    fn a_segmented_button_supports_multiple_selected_segments() {
        test_widgets(|tester| {
            let recording = Recording::new(vec![1]);
            let frame_with_selection = |tester: &mut WidgetTester, selected: Vec<i32>| {
                pump_button(
                    tester,
                    m3,
                    SegmentedButton::new(numbered(&[1, 2, 3]), selected)
                        .multi_selection_enabled(true)
                        .on_selection_changed(recording.on_selection_changed()),
                );
            };

            frame_with_selection(tester, recording.selection());
            assert_eq!(recording.selection(), [1]);
            assert_eq!(recording.callback_count(), 0);

            // Tap on segment 2.
            tap_text(tester, "2");
            tester.pump_and_settle();
            assert_eq!(recording.callback_count(), 1);
            assert_eq!(sorted(recording.selection()), [1, 2]);

            // Update the selection in the widget
            frame_with_selection(tester, vec![1, 2]);
            tester.pump_and_settle();

            // Tap on segment 1 again should remove it from selection.
            tap_text(tester, "1");
            tester.pump_and_settle();
            assert_eq!(recording.callback_count(), 2);
            assert_eq!(recording.selection(), [2]);

            // Update the selection in the widget
            frame_with_selection(tester, vec![2]);
            tester.pump_and_settle();

            // Tap on segment 3.
            tap_text(tester, "3");
            tester.pump_and_settle();
            assert_eq!(recording.callback_count(), 3);
            assert_eq!(sorted(recording.selection()), [2, 3]);
        });
    }

    // 'SegmentedButton allows for empty selection'
    #[test]
    fn a_segmented_button_allows_for_empty_selection() {
        test_widgets(|tester| {
            let recording = Recording::new(vec![1]);
            let frame_with_selection = |tester: &mut WidgetTester, selected: Vec<i32>| {
                pump_button(
                    tester,
                    m3,
                    SegmentedButton::new(numbered(&[1, 2, 3]), selected)
                        .empty_selection_allowed(true)
                        .on_selection_changed(recording.on_selection_changed()),
                );
            };

            frame_with_selection(tester, vec![1]);
            assert_eq!(recording.selection(), [1]);
            assert_eq!(recording.callback_count(), 0);

            // Tap on segment 1 should deselect it and make the selection empty.
            tap_text(tester, "1");
            tester.pump_and_settle();
            assert_eq!(recording.callback_count(), 1);
            assert_eq!(recording.selection(), Vec::<i32>::new());

            // Update the selection in the widget
            frame_with_selection(tester, vec![]);

            // Tap on segment 2 should select it.
            tap_text(tester, "2");
            tester.pump_and_settle();
            assert_eq!(recording.callback_count(), 2);
            assert_eq!(recording.selection(), [2]);

            // Update the selection in the widget
            frame_with_selection(tester, vec![2]);

            // Tap on segment 3.
            tap_text(tester, "3");
            tester.pump_and_settle();
            assert_eq!(recording.callback_count(), 3);
            assert_eq!(recording.selection(), [3]);
        });
    }

    /// `find.descendant(of: find.widgetWithText(Row, text), matching: find.byIcon(icon))`.
    fn icons_in_rows_with_text(tester: &WidgetTester, text: &str, icon: &IconData) -> usize {
        let text_element = tester.find_text(text)[0];
        tester
            .find_by_type::<Row>()
            .into_iter()
            .filter(|row| tester.ancestors(text_element).contains(row))
            .map(|row| icons_under(tester, row, icon))
            .sum()
    }

    /// `find.descendant(of: find.widgetWithIcon(Row, icon1), matching: find.byIcon(icon2))`.
    fn icons_in_rows_with_icon(tester: &WidgetTester, icon1: &IconData, icon2: &IconData) -> usize {
        tester
            .find_by_type::<Row>()
            .into_iter()
            .filter(|row| icons_under(tester, *row, icon1) > 0)
            .map(|row| icons_under(tester, row, icon2))
            .sum()
    }

    fn icons_under(tester: &WidgetTester, of: AnyElement, icon: &IconData) -> usize {
        tester
            .descendants_of_type::<Icon>(of)
            .into_iter()
            .filter(|element| {
                tester.widget(*element, |widget: &Icon| widget.icon.as_ref() == Some(icon))
            })
            .count()
    }

    fn count_icons(tester: &WidgetTester, icon: &IconData) -> usize {
        tester
            .find_by_type::<Icon>()
            .into_iter()
            .filter(|element| {
                tester.widget(*element, |widget: &Icon| widget.icon.as_ref() == Some(icon))
            })
            .count()
    }

    // 'SegmentedButton shows checkboxes for selected segments'
    #[test]
    fn a_segmented_button_shows_checkboxes_for_selected_segments() {
        test_widgets(|tester| {
            let check = Icons::check();
            for selected in [1, 2, 2] {
                pump_button(
                    tester,
                    m3,
                    SegmentedButton::new(numbered(&[1, 2, 3]), vec![selected])
                        .on_selection_changed(|_app, _selected| {}),
                );
                assert_eq!(
                    icons_in_rows_with_text(tester, &selected.to_string(), &check),
                    1
                );
                assert_eq!(count_icons(tester, &check), 1);
            }
        });
    }

    // 'SegmentedButton shows selected checkboxes in place of icon if it has a label as well'
    #[test]
    fn a_segmented_button_shows_selected_checkboxes_in_place_of_the_icon_of_a_labelled_segment() {
        test_widgets(|tester| {
            let icons = [Icons::add(), Icons::add_a_photo(), Icons::add_alarm()];
            let check = Icons::check();
            for selected in 1..=3 {
                let segments = (1..=3)
                    .map(|value| {
                        ButtonSegment::new(value)
                            .icon(Icon::new(Some(icons[value as usize - 1].clone())))
                            .label(label(&value.to_string()))
                    })
                    .collect();
                pump_button(
                    tester,
                    m3,
                    SegmentedButton::new(segments, vec![selected])
                        .on_selection_changed(|_app, _selected| {}),
                );
                for value in 1..=3 {
                    let text = value.to_string();
                    let shown = if value == selected {
                        &check
                    } else {
                        &icons[value as usize - 1]
                    };
                    assert_eq!(icons_in_rows_with_text(tester, &text, shown), 1);
                    if value == selected {
                        assert_eq!(count_icons(tester, &icons[value as usize - 1]), 0);
                    }
                }
            }
        });
    }

    // 'SegmentedButton shows selected checkboxes next to icon if there is no label'
    #[test]
    fn a_segmented_button_shows_selected_checkboxes_next_to_the_icon_of_an_unlabelled_segment() {
        test_widgets(|tester| {
            let icons = [Icons::add(), Icons::add_a_photo(), Icons::add_alarm()];
            let check = Icons::check();
            for selected in 1..=3 {
                let segments = (1..=3)
                    .map(|value| {
                        ButtonSegment::new(value)
                            .icon(Icon::new(Some(icons[value as usize - 1].clone())))
                    })
                    .collect();
                pump_button(
                    tester,
                    m3,
                    SegmentedButton::new(segments, vec![selected])
                        .on_selection_changed(|_app, _selected| {}),
                );
                for value in 1..=3 {
                    let expected = usize::from(value == selected);
                    assert_eq!(
                        icons_in_rows_with_icon(tester, &icons[value as usize - 1], &check),
                        expected
                    );
                }
            }
        });
    }

    fn tooltip_messages(tester: &WidgetTester) -> Vec<String> {
        tester
            .find_by_type::<Tooltip>()
            .into_iter()
            .filter_map(|element| {
                tester.widget(element, |tooltip: &Tooltip| tooltip.message.clone())
            })
            .collect()
    }

    fn three_segments_with_tooltips(tooltips: bool) -> Vec<ButtonSegment<i32>> {
        let mut segments = numbered(&[1, 2, 3]);
        segments[2].enabled = false;
        if tooltips {
            segments[1].tooltip = Some("t2".to_string());
            segments[2].tooltip = Some("t3".to_string());
        }
        segments
    }

    // 'SegmentedButton has no tooltips by default'
    #[test]
    fn a_segmented_button_has_no_tooltips_by_default() {
        test_widgets(|tester| {
            pump_button(
                tester,
                m3,
                SegmentedButton::new(three_segments_with_tooltips(false), vec![2])
                    .on_selection_changed(|_app, _selected| {}),
            );

            assert!(tester.find_by_type::<Tooltip>().is_empty());
        });
    }

    // 'SegmentedButton has correct tooltips'
    #[test]
    fn a_segmented_button_has_the_tooltips_of_its_segments() {
        test_widgets(|tester| {
            pump_button(
                tester,
                m3,
                SegmentedButton::new(three_segments_with_tooltips(true), vec![2])
                    .on_selection_changed(|_app, _selected| {}),
            );

            assert_eq!(tester.find_by_type::<Tooltip>().len(), 2);
            assert_eq!(tooltip_messages(tester), ["t2", "t3"]);
        });
    }

    // 'Disabled SegmentedButton has correct states when rebuilding'
    #[test]
    fn a_disabled_segmented_button_keeps_its_states_when_rebuilding() {
        test_widgets(|tester| {
            // Dart's `const SegmentedButton`: one widget the rebuild hands on again.
            let button = SegmentedButton::new(numbered(&[0]), vec![0]).into_widget();
            let root = {
                let app = tester.cell.borrow();
                material_app(
                    m3(&app),
                    Scaffold::new()
                        .body(Center::new().child(waft_widgets::StatefulBuilder::new(
                            move |_app, _context, set_state| {
                                waft_widgets::Column::new()
                                    .children([
                                        button.clone(),
                                        crate::elevated_button::ElevatedButton::new(
                                            Some(Listener::new(move |app| {
                                                set_state(app, &mut |_| {})
                                            })),
                                            Some(label("Trigger rebuild")),
                                        )
                                        .into_widget(),
                                    ])
                                    .into_widget()
                            },
                        )))
                        .into_widget(),
                )
            };
            tester.pump_widget(root);
            let states: WidgetStates = [WidgetState::Selected, WidgetState::Disabled]
                .into_iter()
                .collect();
            let first_states = |tester: &WidgetTester| {
                let state = state_of(tester);
                let app = tester.cell.borrow();
                let controller = app
                    .get(state)
                    .states_controllers
                    .first()
                    .expect("a controller")
                    .1
                    .clone();
                app.get(&controller).value().clone()
            };
            // Check the initial states.
            assert_eq!(first_states(tester), states);
            // Trigger a rebuild.
            tap_text(tester, "Trigger rebuild");
            tester.pump_and_settle();
            // Check the states after the rebuild.
            assert_eq!(first_states(tester), states);
        });
    }

    fn segmented_button_width(tester: &WidgetTester) -> f64 {
        let element = tester.find_by_type::<SegmentedButton<i32>>()[0];
        let app = tester.cell.borrow();
        tester.render_box_of(element).size(&app).width()
    }

    fn scaffold_width(tester: &WidgetTester) -> f64 {
        let element = tester.find_by_type::<Scaffold>()[0];
        let app = tester.cell.borrow();
        tester.render_box_of(element).size(&app).width()
    }

    // 'SegmentedButton expands to fill the available width when expandedInsets is not null'
    #[test]
    fn a_segmented_button_with_expanded_insets_expands_to_the_available_width() {
        test_widgets(|tester| {
            let segments = vec![
                ButtonSegment::new(1).label(label("Segment 1")),
                ButtonSegment::new(2).label(label("Segment 2")),
            ];
            pump_button(
                tester,
                m3,
                SegmentedButton::new(segments, vec![1]).expanded_insets(EdgeInsets::ZERO),
            );

            // The width of the SegmentedButton must be equal to the width of the parent widget.
            assert_eq!(segmented_button_width(tester), scaffold_width(tester));
        });
    }

    // 'SegmentedButton does not expand when expandedInsets is null'
    #[test]
    fn a_segmented_button_without_expanded_insets_does_not_expand() {
        test_widgets(|tester| {
            let segments = vec![
                ButtonSegment::new(1).label(label("Segment 1")),
                ButtonSegment::new(2).label(label("Segment 2")),
            ];
            pump_button(tester, m3, SegmentedButton::new(segments, vec![1]));

            // The width of the SegmentedButton must be less than the width of the parent widget.
            assert!(segmented_button_width(tester) < scaffold_width(tester));
        });
    }

    // 'SegmentedButton vertical aligned children'
    #[test]
    fn a_vertical_segmented_button_stacks_its_segments() {
        test_widgets(|tester| {
            pump_button(
                tester,
                m3,
                SegmentedButton::new(numbered(&[0, 1, 2, 3]), vec![-1])
                    .direction(Axis::Vertical)
                    .on_selection_changed(|_app, _selected| {}),
            );

            let mut previous_child_rect: Option<Rect> = None;
            for i in 0..=3 {
                let text = tester.find_text(&i.to_string())[0];
                let button = tester.ancestor_of_type::<TextButton>(text);
                let current_child_rect = tester.get_rect(tester.render_box_of(button));
                if let Some(previous) = previous_child_rect {
                    assert_eq!(current_child_rect.left, previous.left);
                    assert_eq!(current_child_rect.right, previous.right);
                    assert_eq!(current_child_rect.top, previous.top + previous.height());
                }
                previous_child_rect = Some(current_child_rect);
            }
        });
    }

    // 'SegmentedButton does not crash at zero area'
    #[test]
    fn a_segmented_button_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            let root = {
                let app = tester.cell.borrow();
                material_app(
                    m3(&app),
                    Center::new()
                        .child(SizedBox::shrink().child(SegmentedButton::new(
                            vec![ButtonSegment::new("X").label(label("X"))],
                            vec!["X"],
                        )))
                        .into_widget(),
                )
            };
            tester.pump_widget(root);

            let element = tester.find_by_type::<SegmentedButton<&str>>()[0];
            let size = tester.render_box_of(element).size(&tester.cell.borrow());
            assert_eq!(size, Size::ZERO);
        });
    }

    // 'SegmentedButton should expand to fill the full width'
    #[test]
    fn a_vertical_segmented_button_expands_to_fill_the_full_width() {
        test_widgets(|tester| {
            const SCREEN_WIDTH: f64 = 800.0;
            let segments = ["All", "Top free", "Top paid"]
                .iter()
                .map(|value| ButtonSegment::new(*value).label(label(value)))
                .collect();
            let root = {
                let app = tester.cell.borrow();
                material_app(
                    m3(&app),
                    Scaffold::new()
                        .body(
                            Center::new().child(
                                SizedBox::new().width(f64::INFINITY).child(
                                    SegmentedButton::new(segments, vec!["All"])
                                        .direction(Axis::Vertical),
                                ),
                            ),
                        )
                        .into_widget(),
                )
            };
            tester.pump_widget(root);
            tester.pump_and_settle();

            let button = tester.find_by_type::<SegmentedButton<&str>>()[0];
            assert_eq!(
                tester
                    .render_box_of(button)
                    .size(&tester.cell.borrow())
                    .width(),
                SCREEN_WIDTH
            );
            for material in tester.descendants_of_type::<Material>(button) {
                assert_eq!(
                    tester
                        .render_box_of(material)
                        .size(&tester.cell.borrow())
                        .width(),
                    SCREEN_WIDTH
                );
            }
        });
    }

    // 'SegmentedButton has expected default mouse cursor on hover'
    #[test]
    fn a_segmented_button_has_the_basic_mouse_cursor_on_hover() {
        test_widgets(|tester| {
            pump_button(
                tester,
                m3,
                SegmentedButton::new(numbered(&[0, 1]), vec![0])
                    .on_selection_changed(|_app, _selected| {}),
            );

            let mut gesture = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::new(10.0, 10.0));
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::BASIC.into());

            let center = text_center(tester, "0");
            tester.move_to(&mut gesture, center, false);
            tester.pump();
            assert_eq!(tester.active_cursor(1), SystemMouseCursors::BASIC.into());
        });
    }

    pub(crate) fn text_center(tester: &WidgetTester, text: &str) -> Offset {
        let element = tester.find_text(text)[0];
        tester.get_rect(tester.render_box_of(element)).center()
    }

    // 'SegmentedButton has expected mouse cursor when explicitly configured'
    #[test]
    fn a_segmented_button_has_the_mouse_cursor_it_is_configured_with() {
        test_widgets(|tester| {
            pump_button(
                tester,
                m3,
                SegmentedButton::new(numbered(&[0, 1]), vec![0])
                    .style(
                        ButtonStyle::new().mouse_cursor(Rc::new(WidgetStatePropertyAll::new(
                            Some(MouseCursorRef::from(SystemMouseCursors::GRAB)),
                        ))),
                    )
                    .on_selection_changed(|_app, _selected| {}),
            );

            let mut gesture = TestPointer::mouse(1, 0);
            let element = tester.find_by_type::<SegmentedButton<i32>>()[0];
            let center = tester.get_rect(tester.render_box_of(element)).center();
            tester.add_pointer(&mut gesture, center);

            assert_eq!(tester.active_cursor(1), SystemMouseCursors::GRAB.into());
        });
    }

    pub(crate) fn argb(color: Color) -> u32 {
        color.to_argb32()
    }

    /// The colors of the rects the ink features of every material draw: Dart's
    /// `getOverlayColor`, which reads the ink render object the hovered segment's ink is on.
    pub(crate) fn ink_rect_colors(tester: &WidgetTester) -> Vec<u32> {
        tester
            .all::<crate::material::RenderInkFeatures>()
            .into_iter()
            .flat_map(|render| {
                let mut canvas = waft_embedder::Canvas::new();
                render.paint_ink_features(&mut tester.cell.borrow_mut(), &mut canvas, Offset::ZERO);
                Drawn::draws(&canvas.build())
            })
            .filter_map(|drawn| match drawn {
                Drawn::Rect(color, _) => Some(color),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn mouse_at(tester: &mut WidgetTester, at: Offset) -> TestPointer {
        let mut gesture = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
        tester.add_pointer(&mut gesture, at);
        tester.move_to(&mut gesture, at, false);
        gesture
    }

    fn enabled_states() -> WidgetStates {
        WidgetStates::new()
    }

    fn disabled_states() -> WidgetStates {
        [WidgetState::Disabled].into_iter().collect()
    }

    fn selected_states() -> WidgetStates {
        [WidgetState::Selected].into_iter().collect()
    }

    fn button_style_of(tester: &WidgetTester, index: usize) -> ButtonStyle {
        let button = tester.find_by_type::<TextButton>()[index];
        tester.widget(button, |button: &TextButton| {
            button
                .button_style_button_data()
                .style
                .clone()
                .expect("a style")
        })
    }

    // 'SegmentedButton default overlayColor and foregroundColor resolve pressed state'
    #[test]
    fn a_segmented_button_resolves_its_default_overlay_and_foreground_colors_when_pressed() {
        test_widgets(|tester| {
            let theme = ThemeData::new(&tester.cell.borrow());
            pump_button(
                tester,
                m3,
                SegmentedButton::new(numbered(&[1, 2]), vec![1])
                    .on_selection_changed(|_app, _selected| {}),
            );
            let on_surface = theme.color_scheme.on_surface;
            let button = *tester
                .find_by_type::<TextButton>()
                .last()
                .expect("a button");
            let material = tester.descendants_of_type::<Material>(button)[0];
            let text_color = |tester: &WidgetTester| {
                tester.widget(material, |material: &Material| {
                    material
                        .text_style
                        .as_ref()
                        .and_then(|style| style.color.as_deref().copied())
                })
            };

            // Hovered.
            let center = text_center(tester, "2");
            let mut gesture = mouse_at(tester, center);
            tester.pump_and_settle();
            assert!(ink_rect_colors(tester).contains(&argb(on_surface.with_opacity(0.08))));
            assert_eq!(text_color(tester), Some(on_surface));

            // Highlighted (pressed).
            tester.down(&mut gesture, center);
            tester.pump_and_settle();
            let colors = ink_rect_colors(tester);
            assert!(colors.contains(&argb(on_surface.with_opacity(0.1))));
            assert_eq!(text_color(tester), Some(on_surface));
        });
    }

    // 'SegmentedButton.styleFrom is applied to the SegmentedButton'
    #[test]
    fn a_segmented_button_applies_the_style_from_style_from() {
        test_widgets(|tester| {
            let foreground_color = Color::new(0xfffffff0);
            let background_color = Color::new(0xfffffff1);
            let selected_background_color = Color::new(0xfffffff2);
            let selected_foreground_color = Color::new(0xfffffff3);
            let disabled_background_color = Color::new(0xfffffff4);
            let disabled_foreground_color = Color::new(0xfffffff5);
            let enabled_mouse_cursor: MouseCursorRef = SystemMouseCursors::TEXT.into();
            let disabled_mouse_cursor: MouseCursorRef = SystemMouseCursors::GRAB.into();

            let style_from_style = SegmentedButton::<i32>::style_from()
                .foreground_color(foreground_color)
                .background_color(background_color)
                .selected_foreground_color(selected_foreground_color)
                .selected_background_color(selected_background_color)
                .disabled_foreground_color(disabled_foreground_color)
                .disabled_background_color(disabled_background_color)
                .shadow_color(Color::new(0xfffffff6))
                .surface_tint_color(Color::new(0xfffffff7))
                .elevation(1.0)
                .text_style(TextStyle::new().color(Color::new(0xfffffff8)))
                .padding(EdgeInsets::all(2.0).into())
                .side(BorderSide {
                    color: Color::new(0xfffffff9),
                    ..BorderSide::default()
                })
                .shape(RoundedRectangleBorder {
                    border_radius: waft_painting::BorderRadius::all(Radius::circular(3.0)).into(),
                    ..RoundedRectangleBorder::default()
                })
                .enabled_mouse_cursor(enabled_mouse_cursor.clone())
                .disabled_mouse_cursor(disabled_mouse_cursor.clone())
                .visual_density(VisualDensity::COMPACT)
                .tap_target_size(MaterialTapTargetSize::ShrinkWrap)
                .animation_duration(Duration::from_millis(100))
                .enable_feedback(true)
                .alignment(AlignmentGeometry::CENTER)
                .splash_factory(crate::no_splash::NoSplash::splash_factory())
                .build();

            let mut segments = numbered(&[1, 2, 3]);
            segments[2].enabled = false;
            pump_button(
                tester,
                m3,
                SegmentedButton::new(segments, vec![2])
                    .style(style_from_style.clone())
                    .on_selection_changed(|_app, _selected| {})
                    .selected_icon(Icon::new(Some(Icons::alarm()))),
            );

            // Test provided button style is applied to the enabled button segment.
            let button_style = button_style_of(tester, 0);
            let resolve_color = |property: &Option<WidgetStatePropertyRef<Option<Color>>>,
                                 states: &WidgetStates| {
                property
                    .as_ref()
                    .and_then(|property| property.resolve(states))
            };
            assert_eq!(
                resolve_color(&button_style.foreground_color, &enabled_states()),
                Some(foreground_color)
            );
            assert_eq!(
                resolve_color(&button_style.background_color, &enabled_states()),
                Some(background_color)
            );
            assert_eq!(button_style.overlay_color, style_from_style.overlay_color);
            assert_eq!(
                button_style.surface_tint_color,
                style_from_style.surface_tint_color
            );
            assert_eq!(button_style.elevation, style_from_style.elevation);
            assert_eq!(button_style.text_style, style_from_style.text_style);
            assert_eq!(button_style.padding, style_from_style.padding);
            assert_eq!(
                button_style
                    .mouse_cursor
                    .as_ref()
                    .and_then(|property| property.resolve(&enabled_states())),
                Some(enabled_mouse_cursor.clone())
            );
            assert_eq!(button_style.visual_density, style_from_style.visual_density);
            assert_eq!(
                button_style.tap_target_size,
                style_from_style.tap_target_size
            );
            assert_eq!(
                button_style.animation_duration,
                style_from_style.animation_duration
            );
            assert_eq!(
                button_style.enable_feedback,
                style_from_style.enable_feedback
            );
            assert_eq!(button_style.alignment, style_from_style.alignment);
            assert_eq!(button_style.splash_factory, style_from_style.splash_factory);

            // Test provided button style is applied selected button segment.
            let button_style = button_style_of(tester, 1);
            assert_eq!(
                resolve_color(&button_style.foreground_color, &selected_states()),
                Some(selected_foreground_color)
            );
            assert_eq!(
                resolve_color(&button_style.background_color, &selected_states()),
                Some(selected_background_color)
            );
            assert_eq!(
                button_style
                    .mouse_cursor
                    .as_ref()
                    .and_then(|property| property.resolve(&enabled_states())),
                Some(enabled_mouse_cursor)
            );

            // Test provided button style is applied disabled button segment.
            let button_style = button_style_of(tester, 2);
            assert_eq!(
                resolve_color(&button_style.foreground_color, &disabled_states()),
                Some(disabled_foreground_color)
            );
            assert_eq!(
                resolve_color(&button_style.background_color, &disabled_states()),
                Some(disabled_background_color)
            );
            assert_eq!(
                button_style
                    .mouse_cursor
                    .as_ref()
                    .and_then(|property| property.resolve(&disabled_states())),
                Some(disabled_mouse_cursor)
            );

            // Test provided button style is applied to the segmented button material.
            let segmented_button = tester.find_by_type::<SegmentedButton<i32>>()[0];
            let material = tester.descendants_of_type::<Material>(segmented_button)[0];
            let (elevation, shadow_color, surface_tint_color) =
                tester.widget(material, |material: &Material| {
                    (
                        material.elevation,
                        material.shadow_color,
                        material.surface_tint_color,
                    )
                });
            let resolve = |property: &Option<WidgetStatePropertyRef<Option<f64>>>| {
                property
                    .as_ref()
                    .and_then(|property| property.resolve(&enabled_states()))
            };
            assert_eq!(Some(elevation), resolve(&style_from_style.elevation));
            assert_eq!(
                shadow_color,
                resolve_color(&style_from_style.shadow_color, &enabled_states())
            );
            assert_eq!(
                surface_tint_color,
                resolve_color(&style_from_style.surface_tint_color, &enabled_states())
            );

            // Test provided button style border is applied to the segmented button border.
            let side_color: waft_embedder::valo::Color = style_from_style
                .side
                .as_ref()
                .and_then(|property| property.resolve(&enabled_states()))
                .expect("styleFrom gives a side")
                .color
                .into();
            let render = tester.render_box_of(segmented_button).as_object();
            assert!(tester.painted_ops(render).iter().any(|op| matches!(
                op,
                waft_embedder::valo::Op::DrawPath { path, paint, .. }
                    if path.elements().count() == 2 && paint.color == side_color
            )));

            // Test foreground color is applied to the overlay color.
            let center = text_center(tester, "1");
            let mut gesture = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.add_pointer(&mut gesture, center);
            tester.down(&mut gesture, center);
            tester.pump_and_settle();
            assert!(ink_rect_colors(tester).contains(&argb(foreground_color.with_opacity(0.08))));
        });
    }

    fn overlay_style(color: Color) -> ButtonStyle {
        crate::icon_button::IconButton::style_from()
            .overlay_color(color)
            .build()
    }

    // 'SegmentedButton.styleFrom overlayColor overrides default overlay color'
    #[test]
    fn a_style_from_overlay_color_overrides_the_default_overlay_color() {
        test_widgets(|tester| {
            let overlay_color = Color::new(0xffff0000);
            pump_button(
                tester,
                m3,
                SegmentedButton::new(
                    vec![
                        ButtonSegment::new(0).label(label("Option 1")),
                        ButtonSegment::new(1).label(label("Option 2")),
                    ],
                    vec![1],
                )
                .style(overlay_style(overlay_color))
                .on_selection_changed(|_app, _selected| {}),
            );

            // Hovered selected segment,
            let center = text_center(tester, "Option 1");
            let mut gesture = mouse_at(tester, center);
            tester.pump_and_settle();
            assert!(ink_rect_colors(tester).contains(&argb(overlay_color.with_opacity(0.08))));

            // Hovered unselected segment,
            let center = text_center(tester, "Option 2");
            tester.move_to(&mut gesture, center, false);
            tester.pump_and_settle();
            assert!(ink_rect_colors(tester).contains(&argb(overlay_color.with_opacity(0.08))));

            // Highlighted unselected segment (pressed).
            let center = text_center(tester, "Option 1");
            tester.down(&mut gesture, center);
            tester.pump_and_settle();
            let colors = ink_rect_colors(tester);
            assert!(colors.contains(&argb(overlay_color.with_opacity(0.08))));
            assert!(colors.contains(&argb(overlay_color.with_opacity(0.1))));
            // Remove pressed and hovered states,
            tester.up(&gesture);
            tester.pump_and_settle();
            tester.move_to(&mut gesture, Offset::new(0.0, 50.0), false);
            tester.pump_and_settle();

            // Highlighted selected segment (pressed)
            let center = text_center(tester, "Option 2");
            tester.down(&mut gesture, center);
            tester.pump_and_settle();
            let colors = ink_rect_colors(tester);
            assert!(colors.contains(&argb(overlay_color.with_opacity(0.08))));
            assert!(colors.contains(&argb(overlay_color.with_opacity(0.1))));
            // Remove pressed and hovered states,
            tester.up(&gesture);
            tester.pump_and_settle();
            tester.move_to(&mut gesture, Offset::new(0.0, 50.0), false);
            tester.pump_and_settle();

            // Focused unselected segment.
            tester.send_key_event(PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
            tester.pump_and_settle();
            assert!(ink_rect_colors(tester).contains(&argb(overlay_color.with_opacity(0.1))));

            // Focused selected segment.
            tester.send_key_event(PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
            tester.pump_and_settle();
            assert!(ink_rect_colors(tester).contains(&argb(overlay_color.with_opacity(0.1))));
        });
    }

    // 'SegmentedButton.styleFrom can customize the button icon'
    #[test]
    fn a_style_from_icon_color_size_and_disabled_icon_color_customize_the_button_icon() {
        test_widgets(|tester| {
            let icon_color = Color::new(0xFFF000FF);
            let icon_size = 32.0;
            let disabled_icon_color = Color::new(0xFFFFF000);
            let build_button = |enabled: bool| {
                let segments = vec![
                    ButtonSegment::new(0)
                        .label(label("Add"))
                        .icon(Icon::new(Some(Icons::add()))),
                    ButtonSegment::new(1)
                        .label(label("Subtract"))
                        .icon(Icon::new(Some(Icons::remove()))),
                ];
                let button = SegmentedButton::new(segments, vec![0])
                    .style(
                        SegmentedButton::<i32>::style_from()
                            .icon_color(icon_color)
                            .icon_size(icon_size)
                            .disabled_icon_color(disabled_icon_color)
                            .build(),
                    )
                    .show_selected_icon(false);
                if enabled {
                    button.on_selection_changed(|_app, _selected| {})
                } else {
                    button
                }
            };

            // Test enabled button.
            pump_button(tester, m3, build_button(true));
            let icon = tester.find_by_icon(&Icons::add());
            assert_eq!(
                tester.render_box_of(icon).size(&tester.cell.borrow()),
                Size::new(icon_size, icon_size)
            );
            assert_eq!(
                tester.glyph_style(icon).color.as_deref().copied(),
                Some(icon_color)
            );

            // Test disabled button.
            pump_button(tester, m3, build_button(false));
            tester.pump_and_settle();
            let icon = tester.find_by_icon(&Icons::add());
            assert_eq!(
                tester.glyph_style(icon).color.as_deref().copied(),
                Some(disabled_icon_color)
            );
        });
    }

    /// The colours of the shapes the segmented button paints as paths, in paint order: Dart's
    /// `drawRRect` calls, which Valo records as paths. The divider lines, also paths, are left
    /// out: a line is a path of two elements.
    fn rrect_colors(tester: &WidgetTester) -> Vec<waft_embedder::valo::Color> {
        use waft_embedder::valo::Op;
        let element = tester.find_by_type::<SegmentedButton<i32>>()[0];
        let render = tester.render_box_of(element).as_object();
        tester
            .painted_ops(render)
            .iter()
            .filter_map(|op| match op {
                Op::DrawPath { path, paint, .. } if path.elements().count() > 2 => {
                    Some(paint.color)
                }
                _ => None,
            })
            .collect()
    }

    /// Dart's `paints..rrect(color:)` on the segmented button.
    fn paints_rrect_of_color(tester: &WidgetTester, color: Color) -> bool {
        rrect_colors(tester).contains(&color.into())
    }

    fn border_side_style(
        entries: Vec<(waft_widgets::WidgetStatesConstraintRef, Option<BorderSide>)>,
    ) -> ButtonStyle {
        ButtonStyle::new().side(<dyn WidgetStateProperty<Option<BorderSide>>>::from_map(
            entries,
        ))
    }

    fn side_of(color: u32) -> Option<BorderSide> {
        Some(BorderSide {
            color: Color::new(color),
            ..BorderSide::default()
        })
    }

    fn pump_sided(
        tester: &mut WidgetTester,
        enabled: bool,
        style: ButtonStyle,
        selected: Vec<i32>,
    ) {
        let segments = vec![
            ButtonSegment::new(0)
                .label(label("Add"))
                .icon(Icon::new(Some(Icons::add()))),
            ButtonSegment::new(1)
                .label(label("Subtract"))
                .icon(Icon::new(Some(Icons::remove()))),
            ButtonSegment::new(2)
                .label(label("Multiply"))
                .icon(Icon::new(Some(Icons::multiple_stop()))),
        ];
        let mut button = SegmentedButton::new(segments, selected)
            .style(style)
            .show_selected_icon(false)
            .empty_selection_allowed(true);
        if enabled {
            button = button.on_selection_changed(|_app, _selected| {});
        }
        pump_button(tester, m3, button);
    }

    // 'SegmentedButton border sides respect states'
    #[test]
    fn a_segmented_button_border_sides_respect_states() {
        test_widgets(|tester| {
            let disabled_color = 0xFF999999;
            let hovered_color = 0xFF0000FF;
            let focused_color = 0xFF00FF00;
            let selected_color = 0xFF001234;
            let hovered_selected_color = 0xFF32CD32;
            let focused_selected_color = 0xFF0000CD;
            let enabled_color = 0xFFFF0000;

            pump_sided(
                tester,
                true,
                border_side_style(vec![
                    (WidgetState::Hovered.into(), side_of(hovered_color)),
                    (WidgetState::Focused.into(), side_of(focused_color)),
                    (WidgetState::any(), side_of(enabled_color)),
                ]),
                vec![],
            );

            assert!(paints_rrect_of_color(tester, Color::new(enabled_color)));

            // Hovered.
            let center = text_center(tester, "Add");
            let mut gesture = mouse_at(tester, center);
            tester.pump_and_settle();

            assert!(paints_rrect_of_color(tester, Color::new(hovered_color)));

            tester.send(&gesture, waft_embedder::PointerChange::Remove);
            tester.pump_and_settle();

            // Focused.
            tester.send_key_event(PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
            tester.pump_and_settle();

            assert!(paints_rrect_of_color(tester, Color::new(focused_color)));

            pump_sided(
                tester,
                true,
                border_side_style(vec![
                    (
                        WidgetState::Hovered & WidgetState::Selected,
                        side_of(hovered_selected_color),
                    ),
                    (
                        WidgetState::Focused & WidgetState::Selected,
                        side_of(focused_selected_color),
                    ),
                    (WidgetState::Hovered.into(), side_of(hovered_color)),
                    (WidgetState::Focused.into(), side_of(focused_color)),
                    (WidgetState::any(), side_of(enabled_color)),
                ]),
                vec![1],
            );
            tester.pump_and_settle();

            // Hovered.
            let center = text_center(tester, "Add");
            gesture = mouse_at(tester, center);
            tester.pump_and_settle();

            assert!(paints_rrect_of_color(
                tester,
                Color::new(hovered_selected_color)
            ));

            tester.send(&gesture, waft_embedder::PointerChange::Remove);
            tester.pump_and_settle();

            // Focused.
            tester.send_key_event(PhysicalKeyboardKey::TAB, LogicalKeyboardKey::TAB);
            tester.pump_and_settle();

            assert!(paints_rrect_of_color(
                tester,
                Color::new(focused_selected_color)
            ));

            pump_sided(
                tester,
                false,
                border_side_style(vec![
                    (WidgetState::Disabled.into(), side_of(disabled_color)),
                    (WidgetState::any(), side_of(enabled_color)),
                ]),
                vec![],
            );
            tester.pump_and_settle();

            assert!(paints_rrect_of_color(tester, Color::new(disabled_color)));

            pump_sided(
                tester,
                true,
                border_side_style(vec![
                    (WidgetState::Selected.into(), side_of(selected_color)),
                    (WidgetState::any(), side_of(enabled_color)),
                ]),
                vec![1],
            );
            tester.pump_and_settle();

            assert!(paints_rrect_of_color(tester, Color::new(selected_color)));
        });
    }

    // 'SegmentedButton border sides respect disabled state'
    #[test]
    fn a_segmented_button_border_sides_respect_the_disabled_state_of_a_segment() {
        test_widgets(|tester| {
            let disabled_color = 0xFF999999;
            let enabled_color = 0xFFFF0000;
            pump_button(
                tester,
                m3,
                SegmentedButton::new(
                    vec![
                        ButtonSegment::new(0).label(label("0")),
                        ButtonSegment::new(1).label(label("1")).enabled(false),
                    ],
                    vec![0],
                )
                .style(border_side_style(vec![
                    (WidgetState::Disabled.into(), side_of(disabled_color)),
                    (WidgetState::any(), side_of(enabled_color)),
                ]))
                .on_selection_changed(|_app, _selected| {}),
            );
            tester.pump_and_settle();

            // First segment has an enabled border, the second a disabled one, in that order.
            let colors = rrect_colors(tester);
            let enabled = colors
                .iter()
                .position(|color| *color == Color::new(enabled_color).into())
                .expect("an enabled border");
            assert!(
                colors[enabled + 1..].contains(&Color::new(disabled_color).into()),
                "{colors:?}"
            );
        });
    }

    // 'Min button hit target height is 48.0 and min (painted) button height is 40 by default
    // with standard density and MaterialTapTargetSize.padded'
    #[test]
    fn a_segmented_button_is_48_high_to_hit_and_40_high_painted_by_default() {
        test_widgets(|tester| {
            use waft_embedder::valo::{Op, PaintStyle};
            let theme = ThemeData::new(&tester.cell.borrow());
            let icons = [
                Icons::calendar_view_day(),
                Icons::calendar_view_week(),
                Icons::calendar_view_month(),
                Icons::calendar_today(),
            ];
            let segments = ["Day", "Week", "Month", "Year"]
                .iter()
                .enumerate()
                .map(|(value, text)| {
                    ButtonSegment::new(value as i32)
                        .label(label(text))
                        .icon(Icon::new(Some(icons[value].clone())))
                })
                .collect();
            pump_button(
                tester,
                m3,
                SegmentedButton::new(segments, vec![0]).on_selection_changed(|_app, _value| {}),
            );

            assert_eq!(theme.visual_density, VisualDensity::STANDARD);
            assert_eq!(
                theme.material_tap_target_size,
                MaterialTapTargetSize::Padded
            );

            let button = tester.find_by_type::<SegmentedButton<i32>>()[0];
            assert_eq!(
                tester
                    .render_box_of(button)
                    .size(&tester.cell.borrow())
                    .height(),
                48.0
            );
            // Button border height is button.bottom(43.5) - button.top(4.5) + stoke width(1) = 40.
            let mut canvas = waft_embedder::Canvas::new();
            waft_painting::draw_rrect(
                &mut canvas,
                RRect::from_rect_and_corners(
                    Rect::from_ltrb(0.5, 4.5, 497.5, 43.5),
                    Radius::circular(19.5),
                    Radius::circular(19.5),
                    Radius::circular(19.5),
                    Radius::circular(19.5),
                ),
                &waft_embedder::Paint::default(),
            );
            let expected = canvas.build();
            let Some(Op::DrawPath {
                path: expected_path,
                ..
            }) = expected.ops().first()
            else {
                panic!("draw_rrect records a path");
            };
            let render = tester.render_box_of(button).as_object();
            assert!(tester.painted_ops(render).iter().any(|op| matches!(
                op,
                Op::DrawPath { path, paint, .. }
                    if path == expected_path
                        && matches!(paint.style, PaintStyle::Stroke(ref stroke) if stroke.width == 1.0)
            )));
        });
    }

    // 'Focused segment does not lose focus when its selection state changes'
    #[test]
    fn a_focused_segment_keeps_its_focus_when_its_selection_state_changes() {
        test_widgets(|tester| {
            let recording = Recording::new(vec![1]);
            let frame_with_selection = |tester: &mut WidgetTester, selected: Vec<i32>| {
                pump_button(
                    tester,
                    m3,
                    SegmentedButton::new(numbered(&[1, 2]), selected)
                        .multi_selection_enabled(true)
                        .on_selection_changed(recording.on_selection_changed()),
                );
            };

            frame_with_selection(tester, recording.selection());
            assert_eq!(recording.selection(), [1]);
            assert_eq!(recording.callback_count(), 0);

            // Select segment 2.
            frame_with_selection(tester, vec![1, 2]);
            tester.pump_and_settle();

            let segment_2_has_focus = |tester: &WidgetTester, request: bool| {
                let element = tester.find_text("2")[0];
                let mut app = tester.cell.borrow_mut();
                let node = waft_widgets::Focus::of(&mut app, element, false, true);
                if request {
                    node.request_focus(&mut app, None);
                }
                node.has_focus(&mut app)
            };

            // Set focus on segment 2.
            segment_2_has_focus(tester, true);
            tester.pump_and_settle();
            assert!(segment_2_has_focus(tester, false));

            // Unselect segment 2.
            frame_with_selection(tester, vec![1]);
            tester.pump_and_settle();

            // The button should still be focused.
            assert!(segment_2_has_focus(tester, false));
        });
    }

    // 'SegmentedButton dividers matches border rect size'
    #[test]
    fn a_segmented_button_divider_matches_the_border_rect_size() {
        test_widgets(|tester| {
            use waft_embedder::valo::Op;
            pump_button(
                tester,
                m3,
                SegmentedButton::new(
                    vec![
                        ButtonSegment::new(0).label(label("Option 1")),
                        ButtonSegment::new(1).label(label("Option 2")),
                    ],
                    vec![0],
                )
                .on_selection_changed(|_app, _selected| {}),
            );

            const TAP_TARGET_SIZE: f64 = 48.0;
            // Dart's x, 166.8000030517578, is 166.8 at f32; Waft's text measures one f32 step
            // narrower (166.79998779296875), so x is compared within f32 rounding.
            let within_f32 = |actual: f64, expected: f64| {
                (actual - expected).abs() <= f64::from(f32::EPSILON) * expected.abs().max(1.0)
            };
            let (p1, p2) = (
                Offset::new(166.8000030517578, 4.0),
                Offset::new(166.8000030517578, TAP_TARGET_SIZE - 4.0),
            );
            let element = tester.find_by_type::<SegmentedButton<i32>>()[0];
            let render = tester.render_box_of(element).as_object();
            let lines: Vec<(Offset, Offset)> = tester
                .painted_ops(render)
                .iter()
                .filter_map(|op| match op {
                    Op::DrawPath { path, paint, .. }
                        if matches!(paint.style, waft_embedder::valo::PaintStyle::Stroke(_))
                            && path.elements().count() == 2 =>
                    {
                        let bounds = path.bounds();
                        Some((
                            Offset::new(f64::from(bounds.x), f64::from(bounds.y)),
                            Offset::new(
                                f64::from(bounds.x + bounds.width),
                                f64::from(bounds.y + bounds.height),
                            ),
                        ))
                    }
                    _ => None,
                })
                .collect();
            assert!(
                lines.iter().any(|(a, b)| {
                    within_f32(a.dx(), p1.dx())
                        && a.dy() == p1.dy()
                        && within_f32(b.dx(), p2.dx())
                        && b.dy() == p2.dy()
                }),
                "lines painted: {lines:?}"
            );
        });
    }
}
