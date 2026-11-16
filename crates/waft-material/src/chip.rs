//! Flutter counterpart: material_ui's `chip.dart` with its `generated/chip_defaults_m3.g.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;
use std::time::Duration;

use waft_animation::{
    Animatable, AnimationBehavior, AnimationController, AnimationRef, AnimationStatus,
    AnimationStyle, AnyAnimation, ColorTween, CurvedAnimation, Curves, Interval,
};
use waft_embedder::{
    BlendMode, Brightness, Canvas, Clip, Color, FillRule, Matrix4, Offset, Paint, PaintStyle,
    PathBuilder, Rect, Size, Stroke, TextBaseline, TextDirection, clamp_double,
};
use waft_foundation::{
    App, Handle, K_IS_WEB, Listenable, Listener, MergingListenable, Ref, ValueChanged,
};
use waft_painting::{
    AnyBorderSide, AnyColor, BorderRadius, BorderSide, CircleBorder, ClipContext, EdgeInsets,
    EdgeInsetsGeometry, OutlinedBorderRef, ShapeBorderRef, ShapeDecoration, StadiumBorder,
    TextStyle, force_to_point,
};
use waft_rendering::{
    AnyRenderBox, AnyRenderObject, BaselineOffset, BoxConstraints, BoxHitTestResult, BoxParentData,
    ChildLayoutHelper, ChildLayouter, LayerHandle, OpacityLayer, PaintingContext, PipelineOwner,
    RenderBox, RenderBoxData, RenderConstrainedBoxBase, RenderConstrainedBoxData, RenderHandle,
    RenderObject, RenderObjectData, RenderObjectWithChildData, RenderObjectWithChildMixin,
    RenderProxyBoxMixin,
};
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_services::MouseCursorRef;
use waft_widgets::{
    AnimatedBuilder, AnimatedSwitcher, BuildContext, Center, DefaultTextStyle, Directionality,
    FocusNodeRef, IconTheme, IconThemeData, IntoWidget, KeyRef, MediaQuery, RenderObjectWidget,
    Semantics, SingleChildRenderObjectWidget, SlottedContainerRenderObjectData,
    SlottedContainerRenderObjectMixin, SlottedMultiChildRenderObjectWidget, State, StateData,
    StatefulWidget, StatelessWidget, TextAlign, TextOverflow, TickerProviderStateMixin,
    TickerProviderStateMixinData, WidgetRef, WidgetState, WidgetStateProperty,
    WidgetStatePropertyRef, WidgetStates, WidgetStatesController,
};

use crate::chip_theme::{ChipTheme, ChipThemeData, ChipThemeDataGetters};
use crate::colors::Colors;
use crate::constants::K_MIN_INTERACTIVE_DIMENSION;
use crate::debug::{debug_check_has_material, debug_check_has_material_localizations};
use crate::generated::chip_defaults_m3::ChipDefaultsM3;
use crate::icons::Icons;
use crate::ink_decoration::Ink;
use crate::ink_well::{InkWell, InteractiveInkFeatureFactory, InteractiveInkFeatureRef};
use crate::material::{AnyMaterialInkController, Material, RectCallback};
use crate::material_localizations::MaterialLocalizations;
use crate::theme::Theme;
use crate::theme_data::{MaterialTapTargetSize, ThemeData, VisualDensity};
use crate::tooltip::Tooltip;

// Some design constants
const K_CHIP_HEIGHT: f64 = 32.0;

const K_CHECKMARK_ALPHA: i32 = 0xde; // 87%
const K_DISABLED_ALPHA: i32 = 0x61; // 38%
const K_CHECKMARK_STROKE_WIDTH: f64 = 2.0;

const K_SELECT_DURATION: Duration = Duration::from_millis(195);
const K_CHECKMARK_DURATION: Duration = Duration::from_millis(150);
const K_CHECKMARK_REVERSE_DURATION: Duration = Duration::from_millis(50);
const K_DRAWER_DURATION: Duration = Duration::from_millis(150);
const K_REVERSE_DRAWER_DURATION: Duration = Duration::from_millis(100);
const K_DISABLE_DURATION: Duration = Duration::from_millis(75);

const K_SELECT_SCRIM_COLOR: Color = Color::new(0x60191919);

/// Dart's `_kDefaultDeleteIcon`.
fn k_default_delete_icon() -> WidgetRef {
    waft_widgets::Icon::new(Some(Icons::cancel())).into_widget()
}

/// Dart's `_ChipVariant` of `ActionChip`, `ChoiceChip` and `FilterChip`, which each declare
/// their own private copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChipVariant {
    Flat,
    Elevated,
}

/// An interface defining the base attributes for a Material Design chip.
///
/// Chips are compact elements that represent an attribute, text, entity, or
/// action.
///
/// The defaults mentioned in the documentation for each attribute are what
/// the implementing classes typically use for defaults (but this class doesn't
/// provide or enforce them).
///
/// See also:
///
///  * [`Chip`], a chip that displays information and can be deleted.
///  * `InputChip`, a chip that represents a complex piece of information, such
///    as an entity (person, place, or thing) or conversational text, in a
///    compact form.
///  * `ChoiceChip`, allows a single selection from a set of options. Choice
///    chips contain related descriptive text or categories.
///  * `FilterChip`, uses tags or descriptive words as a way to filter content.
///  * `ActionChip`, represents an action related to primary content.
///  * <https://material.io/design/components/chips.html>
pub trait ChipAttributes {
    /// The primary content of the chip.
    ///
    /// Typically a `Text` widget.
    fn label(&self) -> &WidgetRef;

    /// A widget to display prior to the chip's label.
    ///
    /// Typically a `CircleAvatar` widget.
    fn avatar(&self) -> Option<&WidgetRef>;

    /// The style to be applied to the chip's label.
    ///
    /// If this is null and `ThemeData::use_material3` is true, then
    /// `TextTheme::label_large` is used. Otherwise, `TextTheme::body_large`
    /// is used.
    ///
    /// This only has an effect on widgets that respect the `DefaultTextStyle`,
    /// such as `Text`.
    ///
    /// If `TextStyle::color` is a `WidgetStateProperty<Color>`,
    /// [`WidgetStateProperty::resolve`] is used for the following [`WidgetState`]s:
    ///
    ///  * [`WidgetState::Disabled`].
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Pressed`].
    fn label_style(&self) -> Option<&TextStyle>;

    /// The color and weight of the chip's outline.
    ///
    /// Defaults to the border side in the ambient [`ChipThemeData`]. If the theme
    /// border side resolves to null and `ThemeData::use_material3` is true, then
    /// [`BorderSide`] with a `ColorScheme::outline` color is used when the chip is
    /// enabled, and [`BorderSide`] with a `ColorScheme::on_surface` color with an
    /// opacity of 0.12 is used when the chip is disabled. Otherwise, it defaults
    /// to null.
    ///
    /// This value is combined with [`shape`](Self::shape) to create a shape decorated with an
    /// outline. To omit the outline entirely, pass [`BorderSide::NONE`] to
    /// [`side`](Self::side).
    ///
    /// If it is a `WidgetStateBorderSide`, [`WidgetStateProperty::resolve`] is
    /// used for the following [`WidgetState`]s:
    ///
    ///  * [`WidgetState::Disabled`].
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Pressed`].
    fn side(&self) -> Option<&AnyBorderSide>;

    /// The `OutlinedBorder` to draw around the chip.
    ///
    /// Defaults to the shape in the ambient [`ChipThemeData`]. If the theme
    /// shape resolves to null and `ThemeData::use_material3` is true, then
    /// `RoundedRectangleBorder` with a circular border radius of 8.0 is used.
    /// Otherwise, `StadiumBorder` is used.
    ///
    /// This shape is combined with [`side`](Self::side) to create a shape decorated with an
    /// outline. If [`side`](Self::side) is not null or side of [`shape`](Self::shape) is
    /// [`BorderSide::NONE`], side of [`shape`](Self::shape) is ignored. To omit the outline
    /// entirely, pass [`BorderSide::NONE`] to [`side`](Self::side).
    ///
    /// If it is a `WidgetStateOutlinedBorder`, [`WidgetStateProperty::resolve`]
    /// is used for the following [`WidgetState`]s:
    ///
    ///  * [`WidgetState::Disabled`].
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Pressed`].
    fn shape(&self) -> Option<&OutlinedBorderRef>;

    /// Defaults to [`Clip::None`].
    fn clip_behavior(&self) -> Clip;

    /// The focus node of the chip's `InkWell`.
    fn focus_node(&self) -> Option<&FocusNodeRef>;

    /// Whether the chip's `InkWell` should focus itself when it is built.
    fn autofocus(&self) -> bool;

    /// The color that fills the chip, in all [`WidgetState`]s.
    ///
    /// Defaults to null.
    ///
    /// Resolves in the following states:
    ///  * [`WidgetState::Selected`].
    ///  * [`WidgetState::Disabled`].
    fn color(&self) -> Option<&WidgetStatePropertyRef<Option<Color>>>;

    /// Color to be used for the unselected, enabled chip's background.
    ///
    /// The default is light grey.
    fn background_color(&self) -> Option<Color>;

    /// The padding between the contents of the chip and the outside [`shape`](Self::shape).
    ///
    /// If this is null and `ThemeData::use_material3` is true, then
    /// a padding of 8.0 logical pixels on all sides is used. Otherwise,
    /// it defaults to a padding of 4.0 logical pixels on all sides.
    fn padding(&self) -> Option<&EdgeInsetsGeometry>;

    /// Defines how compact the chip's layout will be.
    ///
    /// Chips are unaffected by horizontal density changes.
    ///
    /// See also:
    ///
    ///  * `ThemeData::visual_density`, which specifies the
    ///    [`visual_density`](Self::visual_density) for all widgets within a `Theme`.
    fn visual_density(&self) -> Option<VisualDensity>;

    /// The padding around the [`label`](Self::label) widget.
    ///
    /// By default, this is 4 logical pixels at the beginning and the end of the
    /// label, and zero on top and bottom.
    fn label_padding(&self) -> Option<&EdgeInsetsGeometry>;

    /// Configures the minimum size of the tap target.
    ///
    /// Defaults to `ThemeData::material_tap_target_size`.
    ///
    /// See also:
    ///
    ///  * [`MaterialTapTargetSize`], for a description of how this affects tap targets.
    fn material_tap_target_size(&self) -> Option<MaterialTapTargetSize>;

    /// Elevation to be applied on the chip relative to its parent.
    ///
    /// This controls the size of the shadow below the chip.
    ///
    /// Defaults to 0. The value is always non-negative.
    fn elevation(&self) -> Option<f64>;

    /// Color of the chip's shadow when the elevation is greater than 0.
    ///
    /// If this is null and `ThemeData::use_material3` is true, then
    /// `Colors::TRANSPARENT` color is used. Otherwise, it defaults to null.
    fn shadow_color(&self) -> Option<Color>;

    /// Color of the chip's surface tint overlay when its elevation is
    /// greater than 0.
    ///
    /// This is not recommended for use. Material 3 spec introduced a set of tone-based
    /// surfaces and surface containers in its `ColorScheme`, which provide more flexibility.
    /// The intention is to eventually remove surface tint color from the framework.
    ///
    /// If this is null, defaults to `Colors::TRANSPARENT`.
    fn surface_tint_color(&self) -> Option<Color>;

    /// Theme used for all icons in the chip.
    ///
    /// If this is null and `ThemeData::use_material3` is true, then [`IconThemeData`]
    /// with a `ColorScheme::primary` color and a size of 18.0 is used when
    /// the chip is enabled, and [`IconThemeData`] with a `ColorScheme::on_surface`
    /// color and a size of 18.0 is used when the chip is disabled. Otherwise,
    /// it defaults to null.
    fn icon_theme(&self) -> Option<&IconThemeData>;

    /// Optional size constraints for the avatar.
    ///
    /// When unspecified, defaults to a minimum size of chip height or label height
    /// (whichever is greater) and a padding of 8.0 pixels on all sides.
    ///
    /// The default constraints ensure that the avatar is accessible.
    /// Specifying this parameter enables creation of avatar smaller than
    /// the minimum size, but it is not recommended.
    fn avatar_box_constraints(&self) -> Option<BoxConstraints>;

    /// Used to override the default chip animations durations.
    ///
    /// If [`ChipAnimationStyle::enable_animation`] with duration or reverse duration is
    /// provided, it will be used to override the chip enable and disable animation durations.
    /// If it is null, then default duration will be 75ms.
    ///
    /// If [`ChipAnimationStyle::select_animation`] with duration or reverse duration is
    /// provided, it will be used to override the chip select and unselect animation
    /// durations. If it is null, then default duration will be 195ms.
    ///
    /// If [`ChipAnimationStyle::avatar_drawer_animation`] with duration or reverse duration
    /// is provided, it will be used to override the chip checkmark animation duration.
    /// If it is null, then default duration will be 150ms.
    ///
    /// If [`ChipAnimationStyle::delete_drawer_animation`] with duration or reverse duration
    /// is provided, it will be used to override the chip delete icon animation duration.
    /// If it is null, then default duration will be 150ms.
    fn chip_animation_style(&self) -> Option<&ChipAnimationStyle>;

    /// The cursor for a mouse pointer when it enters or is hovering over the
    /// widget.
    ///
    /// If [`mouse_cursor`](Self::mouse_cursor) is a `WidgetStateMouseCursor`,
    /// [`WidgetStateProperty::resolve`] is used for the following [`WidgetState`]s:
    ///
    ///  * [`WidgetState::Hovered`].
    ///  * [`WidgetState::Focused`].
    ///  * [`WidgetState::Disabled`].
    ///
    /// If this property is null, `WidgetStateMouseCursor::adaptive_clickable` will be used.
    fn mouse_cursor(&self) -> Option<&MouseCursorRef>;
}

/// An interface for Material Design chips that can be deleted.
///
/// The defaults mentioned in the documentation for each attribute are what
/// the implementing classes typically use for defaults (but this class doesn't
/// provide or enforce them).
///
/// See also:
///
///  * [`Chip`], a chip that displays information and can be deleted.
///  * `InputChip`, a chip that represents a complex piece of information, such
///    as an entity (person, place, or thing) or conversational text, in a
///    compact form.
///  * <https://material.io/design/components/chips.html>
pub trait DeletableChipAttributes {
    /// The icon displayed when [`on_deleted`](Self::on_deleted) is set.
    ///
    /// If [`delete_icon_color`](Self::delete_icon_color) is provided, it will be used as the
    /// color of the delete icon. If [`delete_icon_color`](Self::delete_icon_color) is null,
    /// then the icon will use the color specified in the chip `IconTheme`. If the `IconTheme`
    /// is null, then the icon will use the color specified in the `ThemeData::icon_theme`.
    ///
    /// If a size is specified in the chip `IconTheme`, then the delete icon will
    /// use that size. Otherwise, defaults to 18 pixels.
    ///
    /// Defaults to an `Icon` widget set to use `Icons::clear`.
    /// If `ThemeData::use_material3` is false, then defaults to an `Icon` widget
    /// set to use `Icons::cancel`.
    fn delete_icon(&self) -> Option<&WidgetRef>;

    /// Called when the user taps the [`delete_icon`](Self::delete_icon) to delete the chip.
    ///
    /// If null, the delete button will not appear on the chip.
    ///
    /// The chip will not automatically remove itself: this just tells the app
    /// that the user tapped the delete button. In order to delete the chip, you
    /// have to remove it from the parent's children when this is called.
    fn on_deleted(&self) -> Option<&Listener>;

    /// Used to define the delete icon's color with an `IconTheme` that
    /// contains the icon.
    ///
    /// The default is `Color(0xde000000)`
    /// (slightly transparent black) for light themes, and `Color(0xdeffffff)`
    /// (slightly transparent white) for dark themes.
    ///
    /// The delete icon appears if [`DeletableChipAttributes::on_deleted`] is
    /// non-null.
    fn delete_icon_color(&self) -> Option<&AnyColor>;

    /// The message to be used for the chip's delete button tooltip.
    ///
    /// If provided with an empty string, the tooltip of the delete button will be
    /// disabled.
    ///
    /// If null, the default `MaterialLocalizations::delete_button_tooltip` will be
    /// used.
    ///
    /// If the chip is disabled, the delete button tooltip will not be shown.
    fn delete_button_tooltip_message(&self) -> Option<&str>;

    /// Optional size constraints for the delete icon.
    ///
    /// When unspecified, defaults to a minimum size of chip height or label height
    /// (whichever is greater) and a padding of 8.0 pixels on all sides.
    ///
    /// The default constraints ensure that the delete icon is accessible.
    /// Specifying this parameter enables creation of delete icon smaller than
    /// the minimum size, but it is not recommended.
    fn delete_icon_box_constraints(&self) -> Option<BoxConstraints>;
}

/// An interface for Material Design chips that can have check marks.
///
/// The defaults mentioned in the documentation for each attribute are what
/// the implementing classes typically use for defaults (but this class doesn't
/// provide or enforce them).
///
/// See also:
///
///  * `InputChip`, a chip that represents a complex piece of information, such
///    as an entity (person, place, or thing) or conversational text, in a
///    compact form.
///  * `ChoiceChip`, allows a single selection from a set of options. Choice
///    chips contain related descriptive text or categories.
///  * `FilterChip`, uses tags or descriptive words as a way to filter content.
///  * <https://material.io/design/components/chips.html>
pub trait CheckmarkableChipAttributes {
    /// Whether or not to show a check mark when
    /// [`SelectableChipAttributes::selected`] is true.
    ///
    /// Defaults to true.
    fn show_checkmark(&self) -> Option<bool>;

    /// [`Color`] of the chip's check mark when a check mark is visible.
    ///
    /// This will override the color set by the platform's brightness setting.
    ///
    /// If null, it will defer to a color selected by the platform's brightness
    /// setting.
    fn checkmark_color(&self) -> Option<Color>;
}

/// An interface for Material Design chips that can be selected.
///
/// The defaults mentioned in the documentation for each attribute are what
/// the implementing classes typically use for defaults (but this class doesn't
/// provide or enforce them).
///
/// See also:
///
///  * `InputChip`, a chip that represents a complex piece of information, such
///    as an entity (person, place, or thing) or conversational text, in a
///    compact form.
///  * `ChoiceChip`, allows a single selection from a set of options. Choice
///    chips contain related descriptive text or categories.
///  * `FilterChip`, uses tags or descriptive words as a way to filter content.
///  * <https://material.io/design/components/chips.html>
pub trait SelectableChipAttributes {
    /// Whether or not this chip is selected.
    ///
    /// If [`on_selected`](Self::on_selected) is not null, this value will be used to
    /// determine if the select check mark will be shown or not.
    ///
    /// Defaults to false.
    fn selected(&self) -> bool;

    /// Called when the chip should change between selected and de-selected
    /// states.
    ///
    /// When the chip is tapped, then the [`on_selected`](Self::on_selected) callback, if set,
    /// will be applied to `!selected` (see [`selected`](Self::selected)).
    ///
    /// The chip passes the new value to the callback but does not actually
    /// change state until the parent widget rebuilds the chip with the new
    /// value.
    ///
    /// The callback provided to [`on_selected`](Self::on_selected) should update the state of
    /// the parent `StatefulWidget` using the `State::set_state` method, so that the
    /// parent gets rebuilt.
    ///
    /// The [`on_selected`](Self::on_selected) and [`TappableChipAttributes::on_pressed`]
    /// callbacks must not both be specified at the same time.
    fn on_selected(&self) -> Option<&ValueChanged<bool>>;

    /// Elevation to be applied on the chip relative to its parent during the
    /// press motion.
    ///
    /// This controls the size of the shadow below the chip.
    ///
    /// Defaults to 8. The value is always non-negative.
    fn press_elevation(&self) -> Option<f64>;

    /// Color to be used for the chip's background, indicating that it is
    /// selected.
    ///
    /// The chip is selected when [`selected`](Self::selected) is true.
    fn selected_color(&self) -> Option<Color>;

    /// Color of the chip's shadow when the elevation is greater than 0 and the
    /// chip is selected.
    ///
    /// If this is null and `ThemeData::use_material3` is true, then
    /// `Colors::TRANSPARENT` color is used. Otherwise, it defaults to null.
    fn selected_shadow_color(&self) -> Option<Color>;

    /// Tooltip string to be used for the body area (where the label and avatar
    /// are) of the chip.
    fn tooltip(&self) -> Option<&str>;

    /// The shape for the avatar's border: the check mark's scrim is clipped to it.
    ///
    /// Defaults to a `CircleBorder`.
    fn avatar_border(&self) -> &ShapeBorderRef;
}

/// An interface for Material Design chips that can be enabled and disabled.
///
/// The defaults mentioned in the documentation for each attribute are what
/// the implementing classes typically use for defaults (but this class doesn't
/// provide or enforce them).
///
/// See also:
///
///  * `InputChip`, a chip that represents a complex piece of information, such
///    as an entity (person, place, or thing) or conversational text, in a
///    compact form.
///  * `ChoiceChip`, allows a single selection from a set of options. Choice
///    chips contain related descriptive text or categories.
///  * `FilterChip`, uses tags or descriptive words as a way to filter content.
///  * `ActionChip`, represents an action related to primary content.
///  * <https://material.io/design/components/chips.html>
pub trait DisabledChipAttributes {
    /// Whether or not this chip is enabled for input.
    ///
    /// If this is true, but all of the user action callbacks are null (for
    /// example, [`SelectableChipAttributes::on_selected`],
    /// [`TappableChipAttributes::on_pressed`], and
    /// [`DeletableChipAttributes::on_deleted`]), then the
    /// control will still be shown as disabled.
    ///
    /// This is typically used if you want the chip to be disabled, but also show
    /// a delete button.
    ///
    /// For classes which don't have this as a constructor option, [`is_enabled`](Self::is_enabled)
    /// is true if their user action callback is set.
    ///
    /// Defaults to true.
    fn is_enabled(&self) -> bool;

    /// The color used for the chip's background when it is disabled.
    fn disabled_color(&self) -> Option<Color>;
}

/// An interface for Material Design chips that can be pressed.
///
/// The defaults mentioned in the documentation for each attribute are what
/// the implementing classes typically use for defaults (but this class doesn't
/// provide or enforce them).
///
/// See also:
///
///  * `InputChip`, a chip that represents a complex piece of information, such
///    as an entity (person, place, or thing) or conversational text, in a
///    compact form.
///  * `ActionChip`, represents an action related to primary content.
///  * <https://material.io/design/components/chips.html>
pub trait TappableChipAttributes {
    /// Called when the user taps the chip.
    ///
    /// If [`on_pressed`](Self::on_pressed) is set, then this callback will be called when the
    /// user taps on the label or avatar parts of the chip. If
    /// [`on_pressed`](Self::on_pressed) is null, then the chip will be disabled.
    fn on_pressed(&self) -> Option<&Listener>;

    /// Elevation to be applied on the chip relative to its parent during the
    /// press motion.
    ///
    /// This controls the size of the shadow below the chip.
    ///
    /// Defaults to 8. The value is always non-negative.
    fn press_elevation(&self) -> Option<f64>;

    /// Tooltip string to be used for the body area (where the label and avatar
    /// are) of the chip.
    fn tooltip(&self) -> Option<&str>;
}

/// Used to override the default chip animations durations.
///
/// See also:
///
///  * [`ChipAttributes::chip_animation_style`], which is used to override the chip animations
///    durations.
#[derive(Clone, Debug, Default)]
pub struct ChipAnimationStyle {
    /// If [`AnimationStyle::duration`] is provided, it will be used to override
    /// the [`ChipAttributes`] enable animation duration. If it is null, then
    /// [`AnimationStyle::duration`] will default to 75ms.
    ///
    /// If [`AnimationStyle::reverse_duration`] is provided, it will be used to
    /// override the chip disable animation duration. If it is null, then
    /// [`AnimationStyle::reverse_duration`] will default to 75ms.
    pub enable_animation: Option<AnimationStyle>,

    /// If [`AnimationStyle::duration`] is provided, it will be used to override
    /// the chip select animation duration. If it is null, then
    /// [`AnimationStyle::duration`] will default to 195ms.
    ///
    /// If [`AnimationStyle::reverse_duration`] is provided, it will be used to
    /// override the chip unselect animation duration. If it is null, then
    /// [`AnimationStyle::reverse_duration`] will default to 195ms.
    pub select_animation: Option<AnimationStyle>,

    /// If [`AnimationStyle::duration`] is provided, it will be used to override
    /// the chip avatar animation duration. If it is null, then
    /// [`AnimationStyle::duration`] will default to 150ms.
    ///
    /// If [`AnimationStyle::reverse_duration`] is provided, it will be used to
    /// override the chip avatar reverse animation duration. If it is null, then
    /// [`AnimationStyle::reverse_duration`] will default to 150ms.
    pub avatar_drawer_animation: Option<AnimationStyle>,

    /// If [`AnimationStyle::duration`] is provided, it will be used to override
    /// the chip delete icon animation duration. If it is null, then
    /// [`AnimationStyle::duration`] will default to 150ms.
    ///
    /// If [`AnimationStyle::reverse_duration`] is provided, it will be used to
    /// override the chip delete icon reverse animation duration. If it is null, then
    /// [`AnimationStyle::reverse_duration`] will default to 150ms.
    pub delete_drawer_animation: Option<AnimationStyle>,
}

impl ChipAnimationStyle {
    /// Creates an instance of [`ChipAnimationStyle`]; Dart's named arguments are the setters.
    pub fn new() -> ChipAnimationStyle {
        ChipAnimationStyle::default()
    }

    /// Dart `ChipAnimationStyle(enableAnimation:)`.
    pub fn enable_animation(mut self, enable_animation: AnimationStyle) -> ChipAnimationStyle {
        self.enable_animation = Some(enable_animation);
        self
    }

    /// Dart `ChipAnimationStyle(selectAnimation:)`.
    pub fn select_animation(mut self, select_animation: AnimationStyle) -> ChipAnimationStyle {
        self.select_animation = Some(select_animation);
        self
    }

    /// Dart `ChipAnimationStyle(avatarDrawerAnimation:)`.
    pub fn avatar_drawer_animation(
        mut self,
        avatar_drawer_animation: AnimationStyle,
    ) -> ChipAnimationStyle {
        self.avatar_drawer_animation = Some(avatar_drawer_animation);
        self
    }

    /// Dart `ChipAnimationStyle(deleteDrawerAnimation:)`.
    pub fn delete_drawer_animation(
        mut self,
        delete_drawer_animation: AnimationStyle,
    ) -> ChipAnimationStyle {
        self.delete_drawer_animation = Some(delete_drawer_animation);
        self
    }
}

/// The fluent setters of the fields Dart's chip widgets share, each `Dart `Chip(field:)``.
macro_rules! chip_option_setters {
    ($name:ident; $($field:ident: $ty:ty),* $(,)?) => {
        $(
            #[doc = concat!("Dart `", stringify!($name), "(", stringify!($field), ":)`.")]
            pub fn $field(mut self, $field: $ty) -> $name {
                self.$field = Some($field);
                self
            }
        )*
    };
}

/// The setters whose Dart field is not nullable.
macro_rules! chip_value_setters {
    ($name:ident; $($field:ident: $ty:ty),* $(,)?) => {
        $(
            #[doc = concat!("Dart `", stringify!($name), "(", stringify!($field), ":)`.")]
            pub fn $field(mut self, $field: $ty) -> $name {
                self.$field = $field;
                self
            }
        )*
    };
}

/// The setters of widget-typed fields, which take `impl IntoWidget`.
macro_rules! chip_widget_setters {
    ($name:ident; $($field:ident),* $(,)?) => {
        $(
            #[doc = concat!("Dart `", stringify!($name), "(", stringify!($field), ":)`.")]
            pub fn $field<K>(mut self, $field: impl IntoWidget<K>) -> $name {
                self.$field = Some($field.into_widget());
                self
            }
        )*
    };
}

/// The setters of fields a caller gives as a concrete type that converts to the field's.
macro_rules! chip_into_setters {
    ($name:ident; $($field:ident: $ty:ty),* $(,)?) => {
        $(
            #[doc = concat!("Dart `", stringify!($name), "(", stringify!($field), ":)`.")]
            pub fn $field(mut self, $field: impl Into<$ty>) -> $name {
                self.$field = Some($field.into());
                self
            }
        )*
    };
}

/// The setters of a field Dart asserts is not negative.
macro_rules! chip_non_negative_setters {
    ($name:ident; $($field:ident),* $(,)?) => {
        $(
            #[doc = concat!("Dart `", stringify!($name), "(", stringify!($field), ":)`.")]
            pub fn $field(mut self, $field: f64) -> $name {
                debug_assert!($field >= 0.0);
                self.$field = Some($field);
                self
            }
        )*
    };
}

/// `Debug` for a chip widget whose callbacks have none: the label and the state flags.
macro_rules! impl_chip_debug {
    ($name:ident) => {
        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct(stringify!($name))
                    .field("label", &self.label)
                    .finish_non_exhaustive()
            }
        }
    };
}

pub(crate) use chip_into_setters;
pub(crate) use chip_non_negative_setters;
pub(crate) use chip_option_setters;
pub(crate) use chip_value_setters;
pub(crate) use chip_widget_setters;
pub(crate) use impl_chip_debug;

/// Implements the chip attribute interfaces a widget holds as plain fields, each getter
/// answering the field of the same name.
macro_rules! impl_chip_attributes {
    ($name:ident) => {
        impl $crate::chip::ChipAttributes for $name {
            fn label(&self) -> &::waft_widgets::WidgetRef {
                &self.label
            }
            fn avatar(&self) -> Option<&::waft_widgets::WidgetRef> {
                self.avatar.as_ref()
            }
            fn label_style(&self) -> Option<&::waft_painting::TextStyle> {
                self.label_style.as_ref()
            }
            fn side(&self) -> Option<&::waft_painting::AnyBorderSide> {
                self.side.as_ref()
            }
            fn shape(&self) -> Option<&::waft_painting::OutlinedBorderRef> {
                self.shape.as_ref()
            }
            fn clip_behavior(&self) -> ::waft_embedder::Clip {
                self.clip_behavior
            }
            fn focus_node(&self) -> Option<&::waft_widgets::FocusNodeRef> {
                self.focus_node.as_ref()
            }
            fn autofocus(&self) -> bool {
                self.autofocus
            }
            fn color(
                &self,
            ) -> Option<&::waft_widgets::WidgetStatePropertyRef<Option<::waft_embedder::Color>>>
            {
                self.color.as_ref()
            }
            fn background_color(&self) -> Option<::waft_embedder::Color> {
                self.background_color
            }
            fn padding(&self) -> Option<&::waft_painting::EdgeInsetsGeometry> {
                self.padding.as_ref()
            }
            fn visual_density(&self) -> Option<$crate::theme_data::VisualDensity> {
                self.visual_density
            }
            fn label_padding(&self) -> Option<&::waft_painting::EdgeInsetsGeometry> {
                self.label_padding.as_ref()
            }
            fn material_tap_target_size(
                &self,
            ) -> Option<$crate::theme_data::MaterialTapTargetSize> {
                self.material_tap_target_size
            }
            fn elevation(&self) -> Option<f64> {
                self.elevation
            }
            fn shadow_color(&self) -> Option<::waft_embedder::Color> {
                self.shadow_color
            }
            fn surface_tint_color(&self) -> Option<::waft_embedder::Color> {
                self.surface_tint_color
            }
            fn icon_theme(&self) -> Option<&::waft_widgets::IconThemeData> {
                self.icon_theme.as_ref()
            }
            fn avatar_box_constraints(&self) -> Option<::waft_rendering::BoxConstraints> {
                self.avatar_box_constraints
            }
            fn chip_animation_style(&self) -> Option<&$crate::chip::ChipAnimationStyle> {
                self.chip_animation_style.as_ref()
            }
            fn mouse_cursor(&self) -> Option<&::waft_services::MouseCursorRef> {
                self.mouse_cursor.as_ref()
            }
        }
    };
}

/// The deletable attributes, for a widget whose `delete_icon` is optional.
macro_rules! impl_deletable_chip_attributes {
    ($name:ident, $delete_icon:expr) => {
        impl $crate::chip::DeletableChipAttributes for $name {
            fn delete_icon(&self) -> Option<&::waft_widgets::WidgetRef> {
                let get: fn(&$name) -> Option<&::waft_widgets::WidgetRef> = $delete_icon;
                get(self)
            }
            fn on_deleted(&self) -> Option<&::waft_foundation::Listener> {
                self.on_deleted.as_ref()
            }
            fn delete_icon_color(&self) -> Option<&::waft_painting::AnyColor> {
                self.delete_icon_color.as_ref()
            }
            fn delete_button_tooltip_message(&self) -> Option<&str> {
                self.delete_button_tooltip_message.as_deref()
            }
            fn delete_icon_box_constraints(&self) -> Option<::waft_rendering::BoxConstraints> {
                self.delete_icon_box_constraints
            }
        }
    };
}

/// The checkmarkable attributes.
macro_rules! impl_checkmarkable_chip_attributes {
    ($name:ident) => {
        impl $crate::chip::CheckmarkableChipAttributes for $name {
            fn show_checkmark(&self) -> Option<bool> {
                self.show_checkmark
            }
            fn checkmark_color(&self) -> Option<::waft_embedder::Color> {
                self.checkmark_color
            }
        }
    };
}

/// The selectable attributes.
macro_rules! impl_selectable_chip_attributes {
    ($name:ident) => {
        impl $crate::chip::SelectableChipAttributes for $name {
            fn selected(&self) -> bool {
                self.selected
            }
            fn on_selected(&self) -> Option<&::waft_foundation::ValueChanged<bool>> {
                self.on_selected.as_ref()
            }
            fn press_elevation(&self) -> Option<f64> {
                self.press_elevation
            }
            fn selected_color(&self) -> Option<::waft_embedder::Color> {
                self.selected_color
            }
            fn selected_shadow_color(&self) -> Option<::waft_embedder::Color> {
                self.selected_shadow_color
            }
            fn tooltip(&self) -> Option<&str> {
                self.tooltip.as_deref()
            }
            fn avatar_border(&self) -> &::waft_painting::ShapeBorderRef {
                &self.avatar_border
            }
        }
    };
}

/// The disabled attributes, for a widget whose `is_enabled` is the given expression.
macro_rules! impl_disabled_chip_attributes {
    ($name:ident, $is_enabled:expr) => {
        impl $crate::chip::DisabledChipAttributes for $name {
            fn is_enabled(&self) -> bool {
                let is_enabled: fn(&$name) -> bool = $is_enabled;
                is_enabled(self)
            }
            fn disabled_color(&self) -> Option<::waft_embedder::Color> {
                self.disabled_color
            }
        }
    };
}

/// The tappable attributes.
macro_rules! impl_tappable_chip_attributes {
    ($name:ident) => {
        impl $crate::chip::TappableChipAttributes for $name {
            fn on_pressed(&self) -> Option<&::waft_foundation::Listener> {
                self.on_pressed.as_ref()
            }
            fn press_elevation(&self) -> Option<f64> {
                self.press_elevation
            }
            fn tooltip(&self) -> Option<&str> {
                self.tooltip.as_deref()
            }
        }
    };
}

pub(crate) use impl_checkmarkable_chip_attributes;
pub(crate) use impl_chip_attributes;
pub(crate) use impl_deletable_chip_attributes;
pub(crate) use impl_disabled_chip_attributes;
pub(crate) use impl_selectable_chip_attributes;
pub(crate) use impl_tappable_chip_attributes;

/// A Material Design chip.
///
/// Chips are compact elements that represent an attribute, text, entity, or action.
///
/// Supplying a non-null [`on_deleted`](Self::on_deleted) callback will cause the chip to
/// include a button for deleting the chip.
///
/// Its ancestors must include `Material`, `MediaQuery`, `Directionality`, and
/// `MaterialLocalizations`. Typically all of these widgets are provided by `MaterialApp` and
/// `Scaffold`. The [`label`](Self::label) and [`clip_behavior`](Self::clip_behavior)
/// arguments must not be null.
///
/// See also:
///
///  * `CircleAvatar`, which shows images or initials of entities.
///  * `Wrap`, A widget that displays its children in multiple horizontal or vertical runs.
///  * <https://material.io/design/components/chips.html>
#[derive(Clone, Debug)]
pub struct Chip {
    pub key: Option<KeyRef>,
    pub avatar: Option<WidgetRef>,
    pub label: WidgetRef,
    pub label_style: Option<TextStyle>,
    pub label_padding: Option<EdgeInsetsGeometry>,
    pub side: Option<AnyBorderSide>,
    pub shape: Option<OutlinedBorderRef>,
    pub clip_behavior: Clip,
    pub focus_node: Option<FocusNodeRef>,
    pub autofocus: bool,
    pub color: Option<WidgetStatePropertyRef<Option<Color>>>,
    pub background_color: Option<Color>,
    pub padding: Option<EdgeInsetsGeometry>,
    pub visual_density: Option<VisualDensity>,
    pub delete_icon: Option<WidgetRef>,
    pub on_deleted: Option<Listener>,
    pub delete_icon_color: Option<AnyColor>,
    pub delete_button_tooltip_message: Option<String>,
    pub material_tap_target_size: Option<MaterialTapTargetSize>,
    pub elevation: Option<f64>,
    pub shadow_color: Option<Color>,
    pub surface_tint_color: Option<Color>,
    pub icon_theme: Option<IconThemeData>,
    pub avatar_box_constraints: Option<BoxConstraints>,
    pub delete_icon_box_constraints: Option<BoxConstraints>,
    pub chip_animation_style: Option<ChipAnimationStyle>,
    pub mouse_cursor: Option<MouseCursorRef>,
}

impl Chip {
    /// Creates a Material Design chip.
    ///
    /// The [`elevation`](Self::elevation) must be null or non-negative. Dart's named
    /// arguments are the setters.
    pub fn new<K>(label: impl IntoWidget<K>) -> Chip {
        Chip {
            key: None,
            avatar: None,
            label: label.into_widget(),
            label_style: None,
            label_padding: None,
            side: None,
            shape: None,
            clip_behavior: Clip::None,
            focus_node: None,
            autofocus: false,
            color: None,
            background_color: None,
            padding: None,
            visual_density: None,
            delete_icon: None,
            on_deleted: None,
            delete_icon_color: None,
            delete_button_tooltip_message: None,
            material_tap_target_size: None,
            elevation: None,
            shadow_color: None,
            surface_tint_color: None,
            icon_theme: None,
            avatar_box_constraints: None,
            delete_icon_box_constraints: None,
            chip_animation_style: None,
            mouse_cursor: None,
        }
    }

    chip_option_setters!(
        Chip;
        key: KeyRef,
        label_style: TextStyle,
        focus_node: FocusNodeRef,
        color: WidgetStatePropertyRef<Option<Color>>,
        background_color: Color,
        visual_density: VisualDensity,
        on_deleted: Listener,
        delete_button_tooltip_message: String,
        material_tap_target_size: MaterialTapTargetSize,
        shadow_color: Color,
        surface_tint_color: Color,
        icon_theme: IconThemeData,
        avatar_box_constraints: BoxConstraints,
        delete_icon_box_constraints: BoxConstraints,
        chip_animation_style: ChipAnimationStyle,
        mouse_cursor: MouseCursorRef,
    );
    chip_value_setters!(Chip; clip_behavior: Clip, autofocus: bool);
    chip_widget_setters!(Chip; avatar, delete_icon);
    chip_into_setters!(
        Chip;
        label_padding: EdgeInsetsGeometry,
        side: AnyBorderSide,
        shape: OutlinedBorderRef,
        padding: EdgeInsetsGeometry,
        delete_icon_color: AnyColor,
    );
    chip_non_negative_setters!(Chip; elevation);
}

impl StatelessWidget for Chip {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material(app, context));
        let mut raw_chip = RawChip::new(self.label.clone()).tap_enabled(false);
        raw_chip.avatar = self.avatar.clone();
        raw_chip.label_style = self.label_style.clone();
        raw_chip.label_padding = self.label_padding;
        if let Some(delete_icon) = self.delete_icon.clone() {
            raw_chip.delete_icon = delete_icon;
        }
        raw_chip.on_deleted = self.on_deleted.clone();
        raw_chip.delete_icon_color = self.delete_icon_color.clone();
        raw_chip.delete_button_tooltip_message = self.delete_button_tooltip_message.clone();
        raw_chip.side = self.side.clone();
        raw_chip.shape = self.shape.clone();
        raw_chip.clip_behavior = self.clip_behavior;
        raw_chip.focus_node = self.focus_node.clone();
        raw_chip.autofocus = self.autofocus;
        raw_chip.color = self.color.clone();
        raw_chip.background_color = self.background_color;
        raw_chip.padding = self.padding;
        raw_chip.visual_density = self.visual_density;
        raw_chip.material_tap_target_size = self.material_tap_target_size;
        raw_chip.elevation = self.elevation;
        raw_chip.shadow_color = self.shadow_color;
        raw_chip.surface_tint_color = self.surface_tint_color;
        raw_chip.icon_theme = self.icon_theme.clone();
        raw_chip.avatar_box_constraints = self.avatar_box_constraints;
        raw_chip.delete_icon_box_constraints = self.delete_icon_box_constraints;
        raw_chip.chip_animation_style = self.chip_animation_style.clone();
        raw_chip.mouse_cursor = self.mouse_cursor.clone();
        raw_chip.into_widget()
    }
}

impl_chip_attributes!(Chip);
impl_deletable_chip_attributes!(Chip, |chip| chip.delete_icon.as_ref());

/// A raw material design chip.
///
/// This serves as the basis for all of the chip widget types to aggregate.
/// It is typically not created directly, one of the other chip types
/// that are appropriate for the use case are used instead:
///
///  * [`Chip`] a simple chip that can only display information and be deleted.
///  * `InputChip` represents a complex piece of information, such as an entity
///    (person, place, or thing) or conversational text, in a compact form.
///  * `ChoiceChip` allows a single selection from a set of options.
///  * `FilterChip` a chip that uses tags or descriptive words as a way to
///    filter content.
///  * `ActionChip` represents an action related to primary content.
///
/// Raw chips are typically only used if you want to create your own custom chip
/// type.
///
/// Raw chips can be selected by setting [`on_selected`](Self::on_selected), deleted by
/// setting [`on_deleted`](Self::on_deleted), and pushed like a button with
/// [`on_pressed`](Self::on_pressed). They have a [`delete_icon`](Self::delete_icon) that can
/// be set, and customized.
///
/// Its ancestors must include `Material`, `MediaQuery`, `Directionality`, and
/// `MaterialLocalizations`.
#[derive(Clone)]
pub struct RawChip {
    pub key: Option<KeyRef>,
    /// Defaults to be used when `ThemeData::use_material3` is true, or by the chips that
    /// give their own: Dart's `ChipThemeData? defaultProperties`.
    pub default_properties: Option<Rc<dyn ChipThemeDataGetters>>,
    pub avatar: Option<WidgetRef>,
    pub label: WidgetRef,
    pub label_style: Option<TextStyle>,
    pub padding: Option<EdgeInsetsGeometry>,
    pub visual_density: Option<VisualDensity>,
    pub label_padding: Option<EdgeInsetsGeometry>,
    pub delete_icon: WidgetRef,
    pub on_deleted: Option<Listener>,
    pub delete_icon_color: Option<AnyColor>,
    pub delete_button_tooltip_message: Option<String>,
    pub on_pressed: Option<Listener>,
    pub on_selected: Option<ValueChanged<bool>>,
    pub press_elevation: Option<f64>,
    pub tap_enabled: bool,
    pub selected: bool,
    pub is_enabled: bool,
    pub disabled_color: Option<Color>,
    pub selected_color: Option<Color>,
    pub tooltip: Option<String>,
    pub side: Option<AnyBorderSide>,
    pub shape: Option<OutlinedBorderRef>,
    pub clip_behavior: Clip,
    pub focus_node: Option<FocusNodeRef>,
    pub autofocus: bool,
    pub color: Option<WidgetStatePropertyRef<Option<Color>>>,
    pub background_color: Option<Color>,
    pub material_tap_target_size: Option<MaterialTapTargetSize>,
    pub elevation: Option<f64>,
    pub shadow_color: Option<Color>,
    pub surface_tint_color: Option<Color>,
    pub icon_theme: Option<IconThemeData>,
    pub selected_shadow_color: Option<Color>,
    pub show_checkmark: Option<bool>,
    pub checkmark_color: Option<Color>,
    pub avatar_border: ShapeBorderRef,
    pub avatar_box_constraints: Option<BoxConstraints>,
    pub delete_icon_box_constraints: Option<BoxConstraints>,
    pub chip_animation_style: Option<ChipAnimationStyle>,
    pub mouse_cursor: Option<MouseCursorRef>,
}

impl_chip_debug!(RawChip);

impl RawChip {
    /// Creates a raw chip; Dart's named arguments are the setters.
    ///
    /// The [`press_elevation`](Self::press_elevation) and [`elevation`](Self::elevation)
    /// must be null or non-negative. The [`delete_icon`](Self::delete_icon) defaults to an
    /// `Icon` of `Icons::cancel`.
    pub fn new<K>(label: impl IntoWidget<K>) -> RawChip {
        RawChip {
            key: None,
            default_properties: None,
            avatar: None,
            label: label.into_widget(),
            label_style: None,
            padding: None,
            visual_density: None,
            label_padding: None,
            delete_icon: k_default_delete_icon(),
            on_deleted: None,
            delete_icon_color: None,
            delete_button_tooltip_message: None,
            on_pressed: None,
            on_selected: None,
            press_elevation: None,
            tap_enabled: true,
            selected: false,
            is_enabled: true,
            disabled_color: None,
            selected_color: None,
            tooltip: None,
            side: None,
            shape: None,
            clip_behavior: Clip::None,
            focus_node: None,
            autofocus: false,
            color: None,
            background_color: None,
            material_tap_target_size: None,
            elevation: None,
            shadow_color: None,
            surface_tint_color: None,
            icon_theme: None,
            selected_shadow_color: None,
            show_checkmark: None,
            checkmark_color: None,
            avatar_border: ShapeBorderRef::new(CircleBorder::default()),
            avatar_box_constraints: None,
            delete_icon_box_constraints: None,
            chip_animation_style: None,
            mouse_cursor: None,
        }
    }

    /// Dart `RawChip(defaultProperties:)`.
    pub fn default_properties(
        mut self,
        default_properties: Rc<dyn ChipThemeDataGetters>,
    ) -> RawChip {
        self.default_properties = Some(default_properties);
        self
    }

    chip_option_setters!(
        RawChip;
        key: KeyRef,
        label_style: TextStyle,
        visual_density: VisualDensity,
        on_deleted: Listener,
        delete_button_tooltip_message: String,
        on_pressed: Listener,
        on_selected: ValueChanged<bool>,
        disabled_color: Color,
        selected_color: Color,
        tooltip: String,
        focus_node: FocusNodeRef,
        color: WidgetStatePropertyRef<Option<Color>>,
        background_color: Color,
        material_tap_target_size: MaterialTapTargetSize,
        shadow_color: Color,
        surface_tint_color: Color,
        icon_theme: IconThemeData,
        selected_shadow_color: Color,
        show_checkmark: bool,
        checkmark_color: Color,
        avatar_box_constraints: BoxConstraints,
        delete_icon_box_constraints: BoxConstraints,
        chip_animation_style: ChipAnimationStyle,
        mouse_cursor: MouseCursorRef,
    );
    chip_value_setters!(
        RawChip;
        tap_enabled: bool,
        selected: bool,
        is_enabled: bool,
        clip_behavior: Clip,
        autofocus: bool,
        avatar_border: ShapeBorderRef,
    );
    chip_widget_setters!(RawChip; avatar);
    chip_into_setters!(
        RawChip;
        padding: EdgeInsetsGeometry,
        label_padding: EdgeInsetsGeometry,
        side: AnyBorderSide,
        shape: OutlinedBorderRef,
        delete_icon_color: AnyColor,
    );
    chip_non_negative_setters!(RawChip; press_elevation, elevation);

    /// Dart `RawChip(deleteIcon:)`, which falls back to the default delete icon when null.
    pub fn delete_icon<K>(mut self, delete_icon: impl IntoWidget<K>) -> RawChip {
        self.delete_icon = delete_icon.into_widget();
        self
    }
}

impl StatefulWidget for RawChip {
    type State = RawChipState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> RawChipState {
        RawChipState {
            state: StateData::new(),
            ticker_provider: TickerProviderStateMixinData::new(),
            select_controller: None,
            avatar_drawer_controller: None,
            delete_drawer_controller: None,
            enable_controller: None,
            checkmark_animation: None,
            avatar_drawer_animation: None,
            delete_drawer_animation: None,
            enable_animation: None,
            selection_fade: None,
            states_controller: None,
            is_tapping: false,
        }
    }
}

impl_chip_attributes!(RawChip);
impl_deletable_chip_attributes!(RawChip, |chip| Some(&chip.delete_icon));
impl_checkmarkable_chip_attributes!(RawChip);
impl_selectable_chip_attributes!(RawChip);
impl_disabled_chip_attributes!(RawChip, |chip| chip.is_enabled);
impl_tappable_chip_attributes!(RawChip);

/// Dart's `_RawChipState`.
pub struct RawChipState {
    state: StateData<RawChip>,
    ticker_provider: TickerProviderStateMixinData,

    select_controller: Option<Ref<AnimationController>>,
    avatar_drawer_controller: Option<Ref<AnimationController>>,
    delete_drawer_controller: Option<Ref<AnimationController>>,
    enable_controller: Option<Ref<AnimationController>>,
    checkmark_animation: Option<Ref<CurvedAnimation>>,
    avatar_drawer_animation: Option<Ref<CurvedAnimation>>,
    delete_drawer_animation: Option<Ref<CurvedAnimation>>,
    enable_animation: Option<Ref<CurvedAnimation>>,
    selection_fade: Option<Ref<CurvedAnimation>>,

    states_controller: Option<Ref<WidgetStatesController>>,

    is_tapping: bool,
}

/// What Dart's `late` fields hold once `initState` has run.
macro_rules! raw_chip_state_fields {
    ($($field:ident: $ty:ty),* $(,)?) => {
        $(
            fn $field(self: Handle<Self>, app: &App) -> $ty {
                app.get(self).$field.clone().expect("created in init_state")
            }
        )*
    };
}

impl RawChipState {
    const PRESSED_ANIMATION_DURATION: Duration = Duration::from_millis(75);

    raw_chip_state_fields! {
        select_controller: Ref<AnimationController>,
        avatar_drawer_controller: Ref<AnimationController>,
        delete_drawer_controller: Ref<AnimationController>,
        enable_controller: Ref<AnimationController>,
        checkmark_animation: Ref<CurvedAnimation>,
        avatar_drawer_animation: Ref<CurvedAnimation>,
        delete_drawer_animation: Ref<CurvedAnimation>,
        enable_animation: Ref<CurvedAnimation>,
        selection_fade: Ref<CurvedAnimation>,
        states_controller: Ref<WidgetStatesController>,
    }

    /// Dart's `statesController.value`.
    fn states(self: Handle<Self>, app: &App) -> WidgetStates {
        app.get(&self.states_controller(app)).value().clone()
    }

    fn has_delete_button(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).on_deleted.is_some()
    }

    fn has_avatar(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).avatar.is_some()
    }

    fn can_tap(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        widget.is_enabled
            && widget.tap_enabled
            && (widget.on_pressed.is_some() || widget.on_selected.is_some())
    }

    fn is_tapping(self: Handle<Self>, app: &App) -> bool {
        self.can_tap(app) && app.get(self).is_tapping
    }

    fn handle_states_controller_change(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |_state| {});
    }

    fn handle_tap_down(self: Handle<Self>, app: &mut App, _details: waft_gestures::TapDownDetails) {
        if !self.can_tap(app) {
            return;
        }
        self.states_controller(app)
            .update(app, WidgetState::Pressed, true);
        self.set_state(app, |state| state.is_tapping = true);
    }

    fn handle_tap_cancel(self: Handle<Self>, app: &mut App) {
        if !self.can_tap(app) {
            return;
        }
        self.states_controller(app)
            .update(app, WidgetState::Pressed, false);
        self.set_state(app, |state| state.is_tapping = false);
    }

    fn handle_tap(self: Handle<Self>, app: &mut App) {
        if !self.can_tap(app) {
            return;
        }
        self.states_controller(app)
            .update(app, WidgetState::Pressed, false);
        self.set_state(app, |state| state.is_tapping = false);
        // Only one of these can be set, so only one will be called.
        let widget = self.widget(app);
        let (on_selected, selected, on_pressed) = (
            widget.on_selected.clone(),
            widget.selected,
            widget.on_pressed.clone(),
        );
        if let Some(on_selected) = on_selected {
            on_selected(app, !selected);
        }
        if let Some(on_pressed) = on_pressed {
            on_pressed.call(app);
        }
    }

    fn get_shape(
        self: Handle<Self>,
        app: &mut App,
        _theme: &ThemeData,
        chip_theme: &ChipThemeData,
        chip_defaults: &dyn ChipThemeDataGetters,
    ) -> OutlinedBorderRef {
        let states = self.states(app);
        let widget = self.widget(app).clone();
        let resolved_side =
            <dyn WidgetStateProperty<Option<BorderSide>>>::resolve_as(&widget.side, &states)
                .or_else(|| {
                    <dyn WidgetStateProperty<Option<BorderSide>>>::resolve_as(
                        &chip_theme.side,
                        &states,
                    )
                });
        let resolved_shape = <dyn WidgetStateProperty<Option<OutlinedBorderRef>>>::resolve_as(
            &widget.shape,
            &states,
        )
        .or_else(|| {
            <dyn WidgetStateProperty<Option<OutlinedBorderRef>>>::resolve_as(
                &chip_theme.shape,
                &states,
            )
        })
        .or_else(|| {
            <dyn WidgetStateProperty<Option<OutlinedBorderRef>>>::resolve_as(
                &chip_defaults.shape(app),
                &states,
            )
        })
        // TODO(tahatesser): Remove this fallback when Material 2 is deprecated.
        .unwrap_or_else(|| OutlinedBorderRef::new(StadiumBorder::new(BorderSide::NONE)));
        // If the side is provided, shape uses the provided side.
        if let Some(resolved_side) = resolved_side {
            return resolved_shape.copy_with(Some(*resolved_side));
        }
        // If the side is not provided and the shape's side is not [BorderSide.none],
        // then the shape's side is used. Otherwise, the default side is used.
        if resolved_shape.side() != BorderSide::NONE {
            resolved_shape
        } else {
            resolved_shape.copy_with(chip_defaults.side(app).map(|side| *side))
        }
    }

    fn resolve_color(
        self: Handle<Self>,
        app: &App,
        color: Option<WidgetStatePropertyRef<Option<Color>>>,
        selected_color: Option<Color>,
        background_color: Option<Color>,
        disabled_color: Option<Color>,
        default_color: Option<WidgetStatePropertyRef<Option<Color>>>,
    ) -> Option<Color> {
        let states = self.states(app);
        IndividualOverrides {
            color,
            selected_color,
            background_color,
            disabled_color,
        }
        .resolve(&states)
        .or_else(|| default_color.and_then(|default_color| default_color.resolve(&states)))
    }

    fn get_background_color(
        self: Handle<Self>,
        app: &mut App,
        theme: &ThemeData,
        chip_theme: &ChipThemeData,
        chip_defaults: &dyn ChipThemeDataGetters,
    ) -> Option<Color> {
        let widget = self.widget(app).clone();
        let enable_controller = animation_of(&self.enable_controller(app));
        let selection_fade = animation_of(&self.selection_fade(app));
        if theme.use_material3 {
            let color = widget.color.clone().or_else(|| chip_theme.color.clone());
            let default_color = chip_defaults.color(app);
            let disabled_color = self.resolve_color(
                app,
                color.clone(),
                None,
                None,
                widget.disabled_color.or(chip_theme.disabled_color),
                default_color.clone(),
            );
            let background_color = self.resolve_color(
                app,
                color.clone(),
                None,
                widget.background_color.or(chip_theme.background_color),
                None,
                default_color.clone(),
            );
            let selected_color = self.resolve_color(
                app,
                color,
                widget.selected_color.or(chip_theme.selected_color),
                None,
                None,
                default_color,
            );
            let background_tween = ColorTween::new(app, disabled_color, background_color);
            let select_begin = background_tween.evaluate(app, enable_controller);
            let select_tween = ColorTween::new(app, select_begin, selected_color);
            select_tween.evaluate(app, selection_fade)
        } else {
            let background_begin = widget
                .disabled_color
                .or(chip_theme.disabled_color)
                .or(Some(theme.disabled_color));
            let background_end = widget
                .background_color
                .or(chip_theme.background_color)
                .or(theme.chip_theme.background_color)
                .or_else(|| chip_defaults.background_color(app));
            let background_tween = ColorTween::new(app, background_begin, background_end);
            let select_end = widget
                .selected_color
                .or(chip_theme.selected_color)
                .or(theme.chip_theme.selected_color)
                .or_else(|| chip_defaults.selected_color(app));
            let select_begin = background_tween.evaluate(app, enable_controller);
            let select_tween = ColorTween::new(app, select_begin, select_end);
            select_tween.evaluate(app, selection_fade)
        }
    }

    fn wrap_with_tooltip(
        tooltip: Option<String>,
        enabled: bool,
        child: Option<WidgetRef>,
    ) -> Option<WidgetRef> {
        match (child, tooltip) {
            (Some(child), Some(tooltip)) if enabled => {
                Some(Tooltip::new().message(tooltip).child(child).into_widget())
            }
            (child, _) => child,
        }
    }

    fn build_delete_icon(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        theme: &ThemeData,
        chip_theme: &ChipThemeData,
        chip_defaults: &dyn ChipThemeDataGetters,
    ) -> Option<WidgetRef> {
        if !self.has_delete_button(app) {
            return None;
        }
        let widget = self.widget(app).clone();
        let m3_defaults = || ChipDefaultsM3::new(context, widget.is_enabled);
        let icon_theme = match widget
            .icon_theme
            .clone()
            .or_else(|| chip_theme.icon_theme.clone())
            .or_else(|| theme.chip_theme.icon_theme.clone())
        {
            Some(icon_theme) => icon_theme,
            None => m3_defaults()
                .icon_theme(app)
                .expect("the Material 3 defaults give an icon theme"),
        };
        let effective_delete_icon_color = <dyn WidgetStateProperty<Color>>::resolve_as(
            &widget
                .delete_icon_color
                .clone()
                .or_else(|| chip_theme.delete_icon_color.clone())
                .or_else(|| theme.chip_theme.delete_icon_color.clone())
                .or_else(|| {
                    widget
                        .icon_theme
                        .as_ref()
                        .and_then(|icon_theme| icon_theme.color.clone())
                })
                .or_else(|| {
                    chip_theme
                        .icon_theme
                        .as_ref()
                        .and_then(|icon_theme| icon_theme.color.clone())
                })
                .or_else(|| chip_defaults.delete_icon_color(app)),
            &self.states(app),
        )
        .map(|color| color.color());
        let effective_icon_size = match widget
            .icon_theme
            .as_ref()
            .and_then(|icon_theme| icon_theme.size)
            .or_else(|| {
                chip_theme
                    .icon_theme
                    .as_ref()
                    .and_then(|icon_theme| icon_theme.size)
            })
            .or_else(|| {
                theme
                    .chip_theme
                    .icon_theme
                    .as_ref()
                    .and_then(|icon_theme| icon_theme.size)
            }) {
            Some(size) => size,
            None => m3_defaults()
                .icon_theme(app)
                .and_then(|icon_theme| icon_theme.size)
                .expect("the Material 3 defaults give an icon size"),
        };

        let effective_material_tap_target_size = widget
            .material_tap_target_size
            .unwrap_or(theme.material_tap_target_size);
        let semantic_size = match effective_material_tap_target_size {
            MaterialTapTargetSize::Padded => Size::square(K_MIN_INTERACTIVE_DIMENSION),
            MaterialTapTargetSize::ShrinkWrap => Size::square(K_MIN_INTERACTIVE_DIMENSION - 8.0),
        };
        let effective_visual_density = widget.visual_density.unwrap_or(theme.visual_density);
        let density_adjustment = effective_visual_density.base_size_adjustment();

        let tooltip = widget
            .delete_button_tooltip_message
            .clone()
            .unwrap_or_else(|| {
                <dyn MaterialLocalizations>::of(app, context).delete_button_tooltip()
            });
        let on_deleted = widget.on_deleted.clone();
        let is_enabled = widget.is_enabled;
        let splash_factory: Rc<dyn InteractiveInkFeatureFactory> =
            Rc::new(UnconstrainedInkSplashFactory {
                parent_factory: Theme::of(app, context).splash_factory.clone(),
            });
        let mut ink_well = InkWell::new()
            // Radius should be slightly less than the full size of the chip.
            .radius(
                (K_CHIP_HEIGHT
                    + widget
                        .padding
                        .as_ref()
                        .map(|padding| padding.vertical())
                        .unwrap_or(0.0))
                    * 0.45,
            )
            // Keeps the splash from being constrained to the icon alone.
            .splash_factory(splash_factory)
            .custom_border(ShapeBorderRef::new(CircleBorder::default()))
            .child(IconTheme::new(
                {
                    let mut data = icon_theme.copy_with().size(effective_icon_size);
                    if let Some(color) = effective_delete_icon_color {
                        data = data.color(color);
                    }
                    data
                },
                widget.delete_icon.clone(),
            ));
        if is_enabled && let Some(on_deleted) = on_deleted {
            ink_well = ink_well.on_tap(on_deleted);
        }
        Some(
            EnsureMinSemanticsSize {
                semantic_size: Size::new(
                    semantic_size.width() + density_adjustment.dx(),
                    semantic_size.height() + density_adjustment.dy(),
                ),
                child: RawChipState::wrap_with_tooltip(
                    Some(tooltip),
                    widget.is_enabled && widget.on_deleted.is_some(),
                    Some(ink_well.into_widget()),
                ),
            }
            .into_widget(),
        )
    }
}

/// Dart's `Interval(1.0 - percentage, 1.0, curve: Curves.fastOutSlowIn)`.
fn fast_out_slow_in_tail(percentage: f64) -> Rc<dyn waft_animation::Curve> {
    Rc::new(Interval::new(
        1.0 - percentage,
        1.0,
        Curves::fast_out_slow_in(),
    ))
}

/// The animation a controller or curved animation is, for a tween to evaluate.
fn animation_of<T>(animation: &Ref<T>) -> AnyAnimation<f64>
where
    AnyAnimation<f64>: From<Handle<T>>,
{
    animation.handle().into()
}

impl TickerProviderStateMixin for RawChipState {
    fn ticker_provider_data(self: Handle<Self>, app: &App) -> &TickerProviderStateMixinData {
        &app.get(self).ticker_provider
    }

    fn ticker_provider_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut TickerProviderStateMixinData {
        &mut app.get_mut(self).ticker_provider
    }
}

/// Dart's `vsync: this`.
impl TickerProviderObject for RawChipState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        TickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for RawChipState {
    type Widget = RawChip;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app).clone();
        debug_assert!(widget.on_selected.is_none() || widget.on_pressed.is_none());
        let states_controller = WidgetStatesController::new(app, None);
        states_controller.update(app, WidgetState::Disabled, !widget.is_enabled);
        states_controller.update(app, WidgetState::Selected, widget.selected);
        states_controller.add_listener(
            app,
            Listener::handle_method(self, Self::handle_states_controller_change),
        );
        let style = widget.chip_animation_style.clone();
        let (has_avatar, has_delete_button) = (self.has_avatar(app), self.has_delete_button(app));
        let selected = widget.selected;
        let is_enabled = widget.is_enabled;
        let animation_style = |pick: fn(&ChipAnimationStyle) -> &Option<AnimationStyle>| {
            style
                .as_ref()
                .and_then(|style| pick(style).clone())
                .unwrap_or_default()
        };
        let select_style = animation_style(|style| &style.select_animation);
        let select_controller = AnimationController::new(
            app,
            Some(if selected { 1.0 } else { 0.0 }),
            Some(select_style.duration.unwrap_or(K_SELECT_DURATION)),
            select_style.reverse_duration,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let selection_fade = CurvedAnimation::new(
            app,
            select_controller.erase(),
            Curves::fast_out_slow_in(),
            None,
        );
        let avatar_style = animation_style(|style| &style.avatar_drawer_animation);
        let avatar_drawer_controller = AnimationController::new(
            app,
            Some(if has_avatar || selected { 1.0 } else { 0.0 }),
            Some(avatar_style.duration.unwrap_or(K_DRAWER_DURATION)),
            avatar_style.reverse_duration,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let delete_style = animation_style(|style| &style.delete_drawer_animation);
        let delete_drawer_controller = AnimationController::new(
            app,
            Some(if has_delete_button { 1.0 } else { 0.0 }),
            Some(delete_style.duration.unwrap_or(K_DRAWER_DURATION)),
            delete_style.reverse_duration,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let enable_style = animation_style(|style| &style.enable_animation);
        let enable_controller = AnimationController::new(
            app,
            Some(if is_enabled { 1.0 } else { 0.0 }),
            Some(enable_style.duration.unwrap_or(K_DISABLE_DURATION)),
            enable_style.reverse_duration,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );

        // These will delay the start of some animations, and/or reduce their
        // length compared to the overall select animation, using Intervals.
        let checkmark_percentage =
            K_CHECKMARK_DURATION.as_millis() as f64 / K_SELECT_DURATION.as_millis() as f64;
        let checkmark_reverse_percentage =
            K_CHECKMARK_REVERSE_DURATION.as_millis() as f64 / K_SELECT_DURATION.as_millis() as f64;
        let avatar_drawer_reverse_percentage =
            K_REVERSE_DRAWER_DURATION.as_millis() as f64 / K_SELECT_DURATION.as_millis() as f64;
        let checkmark_animation = CurvedAnimation::new(
            app,
            select_controller.erase(),
            fast_out_slow_in_tail(checkmark_percentage),
            Some(fast_out_slow_in_tail(checkmark_reverse_percentage)),
        );
        let delete_drawer_animation = CurvedAnimation::new(
            app,
            delete_drawer_controller.erase(),
            Curves::fast_out_slow_in(),
            None,
        );
        let avatar_drawer_animation = CurvedAnimation::new(
            app,
            avatar_drawer_controller.erase(),
            Curves::fast_out_slow_in(),
            Some(fast_out_slow_in_tail(avatar_drawer_reverse_percentage)),
        );
        let enable_animation = CurvedAnimation::new(
            app,
            enable_controller.erase(),
            Curves::fast_out_slow_in(),
            None,
        );

        let state = app.get_mut(self);
        state.states_controller = Some(states_controller);
        state.select_controller = Some(select_controller);
        state.selection_fade = Some(selection_fade);
        state.avatar_drawer_controller = Some(avatar_drawer_controller);
        state.delete_drawer_controller = Some(delete_drawer_controller);
        state.enable_controller = Some(enable_controller);
        state.checkmark_animation = Some(checkmark_animation);
        state.delete_drawer_animation = Some(delete_drawer_animation);
        state.avatar_drawer_animation = Some(avatar_drawer_animation);
        state.enable_animation = Some(enable_animation);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.select_controller(app).dispose(app);
        self.avatar_drawer_controller(app).dispose(app);
        self.delete_drawer_controller(app).dispose(app);
        self.enable_controller(app).dispose(app);
        self.checkmark_animation(app).dispose(app);
        self.avatar_drawer_animation(app).dispose(app);
        self.delete_drawer_animation(app).dispose(app);
        self.enable_animation(app).dispose(app);
        self.selection_fade(app).dispose(app);
        self.states_controller(app).dispose(app);
        TickerProviderStateMixin::dispose(self, app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        TickerProviderStateMixin::activate(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &RawChip) {
        let widget = self.widget(app).clone();
        if old_widget.is_enabled != widget.is_enabled {
            let is_enabled = widget.is_enabled;
            self.set_state(app, |_state| {});
            self.states_controller(app)
                .update(app, WidgetState::Disabled, !is_enabled);
            if is_enabled {
                self.enable_controller(app).forward(app, None);
            } else {
                self.enable_controller(app).reverse(app, None);
            }
        }
        let widget = self.widget(app).clone();
        let avatar_changed = match (&old_widget.avatar, &widget.avatar) {
            (None, None) => false,
            (Some(old), Some(new)) => !Rc::ptr_eq(old, new),
            _ => true,
        };
        if avatar_changed || old_widget.selected != widget.selected {
            self.set_state(app, |_state| {});
            if self.has_avatar(app) || self.widget(app).selected {
                self.avatar_drawer_controller(app).forward(app, None);
            } else {
                self.avatar_drawer_controller(app).reverse(app, None);
            }
        }
        let widget = self.widget(app).clone();
        if old_widget.selected != widget.selected {
            let selected = widget.selected;
            self.set_state(app, |_state| {});
            self.states_controller(app)
                .update(app, WidgetState::Selected, selected);
            if selected {
                self.select_controller(app).forward(app, None);
            } else {
                self.select_controller(app).reverse(app, None);
            }
        }
        if old_widget.on_deleted != self.widget(app).on_deleted {
            self.set_state(app, |_state| {});
            if self.has_delete_button(app) {
                self.delete_drawer_controller(app).forward(app, None);
            } else {
                self.delete_drawer_controller(app).reverse(app, None);
            }
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material(app, context));
        debug_assert!(waft_widgets::debug_check_has_media_query(app, context));
        debug_assert!(waft_widgets::debug_check_has_directionality(
            app, context, None, None, None
        ));
        debug_assert!(debug_check_has_material_localizations(app, context));

        let widget = self.widget(app).clone();
        let theme = Theme::of(app, context);
        let chip_theme = ChipTheme::of(app, context);
        let brightness = chip_theme.brightness.unwrap_or_else(|| theme.brightness());
        let chip_defaults: Rc<dyn ChipThemeDataGetters> = match widget.default_properties.clone() {
            Some(default_properties) => default_properties,
            None => {
                if theme.use_material3 {
                    Rc::new(ChipDefaultsM3::new(context, widget.is_enabled))
                } else {
                    Rc::new(ChipThemeData::from_defaults(
                        Some(brightness),
                        None,
                        if brightness == Brightness::Dark {
                            Colors::TEAL_ACCENT.shade200()
                        } else {
                            theme.primary_color
                        },
                        theme
                            .text_theme
                            .body_large
                            .clone()
                            .expect("the text theme has a body large style"),
                    ))
                }
            }
        };
        let text_direction = Directionality::maybe_of(app, context);
        let resolved_shape = self.get_shape(app, &theme, &chip_theme, &*chip_defaults);

        let elevation = widget
            .elevation
            .or(chip_theme.elevation)
            .or_else(|| chip_defaults.elevation(app))
            .unwrap_or(0.0);
        let press_elevation = widget
            .press_elevation
            .or(chip_theme.press_elevation)
            .or_else(|| chip_defaults.press_elevation(app))
            .unwrap_or(0.0);
        let shadow_color = widget
            .shadow_color
            .or(chip_theme.shadow_color)
            .or_else(|| chip_defaults.shadow_color(app));
        let surface_tint_color = widget
            .surface_tint_color
            .or(chip_theme.surface_tint_color)
            .or_else(|| chip_defaults.surface_tint_color(app));
        let selected_shadow_color = widget
            .selected_shadow_color
            .or(chip_theme.selected_shadow_color)
            .or_else(|| chip_defaults.selected_shadow_color(app));
        let checkmark_color = widget
            .checkmark_color
            .or(chip_theme.checkmark_color)
            .or_else(|| chip_defaults.checkmark_color(app));
        let show_checkmark = widget
            .show_checkmark
            .or(chip_theme.show_checkmark)
            .or_else(|| chip_defaults.show_checkmark(app))
            .expect("the defaults give showCheckmark");
        let padding = widget
            .padding
            .or(chip_theme.padding)
            .or_else(|| chip_defaults.padding(app))
            .expect("the defaults give a padding");
        // Widget's label style is merged with this below.
        let label_style = chip_theme
            .label_style
            .clone()
            .or_else(|| chip_defaults.label_style(app))
            .expect("the defaults give a label style");
        let icon_theme = widget
            .icon_theme
            .clone()
            .or_else(|| chip_theme.icon_theme.clone())
            .or_else(|| chip_defaults.icon_theme(app));
        let avatar_box_constraints = widget
            .avatar_box_constraints
            .or(chip_theme.avatar_box_constraints);
        let delete_icon_box_constraints = widget
            .delete_icon_box_constraints
            .or(chip_theme.delete_icon_box_constraints);

        let effective_label_style = label_style.merge(widget.label_style.as_ref());
        let states = self.states(app);
        let resolved_label_color =
            <dyn WidgetStateProperty<Color>>::resolve_as(&effective_label_style.color, &states);
        let resolved_label_style = match resolved_label_color {
            Some(color) => effective_label_style.copy_with().color(color),
            None => effective_label_style.copy_with(),
        };
        let has_avatar = self.has_avatar(app);
        let avatar = match (&icon_theme, widget.avatar.clone()) {
            (Some(icon_theme), Some(avatar)) if has_avatar => Some(IconTheme::merge(
                None,
                chip_defaults
                    .icon_theme(app)
                    .expect("the defaults give an icon theme")
                    .merge(Some(icon_theme)),
                avatar,
            )),
            (_, avatar) => avatar,
        };

        let default_font_size = effective_label_style.font_size.unwrap_or(14.0);
        let effective_text_scale =
            MediaQuery::text_scaler_of(app, context).scale(default_font_size) / 14.0;
        let default_label_padding: EdgeInsetsGeometry = EdgeInsets::lerp(
            Some(EdgeInsets::symmetric(0.0, 8.0)),
            Some(EdgeInsets::symmetric(0.0, 4.0)),
            clamp_double(effective_text_scale - 1.0, 0.0, 1.0),
        )
        .expect("both insets are given")
        .into();

        let label_padding = widget
            .label_padding
            .or(chip_theme.label_padding)
            .or_else(|| chip_defaults.label_padding(app))
            .unwrap_or(default_label_padding);

        let can_tap = self.can_tap(app);
        let selected_color_shadow = if widget.selected {
            selected_shadow_color
        } else {
            shadow_color
        };
        let mut material = Material::new()
            .elevation(if self.is_tapping(app) {
                press_elevation
            } else {
                elevation
            })
            .animation_duration(RawChipState::PRESSED_ANIMATION_DURATION)
            .shape(resolved_shape.clone())
            .clip_behavior(widget.clip_behavior);
        if let Some(shadow_color) = selected_color_shadow {
            material = material.shadow_color(shadow_color);
        }
        if let Some(surface_tint_color) = surface_tint_color {
            material = material.surface_tint_color(surface_tint_color);
        }

        let mut ink_well = InkWell::new()
            .on_focus_change(Rc::new(move |app: &mut App, value: bool| {
                self.states_controller(app)
                    .update(app, WidgetState::Focused, value);
            }))
            .autofocus(widget.autofocus)
            .can_request_focus(widget.is_enabled)
            .custom_border(resolved_shape.clone());
        if let Some(focus_node) = widget.focus_node.clone() {
            ink_well = ink_well.focus_node(focus_node);
        }
        if can_tap {
            ink_well = ink_well
                .on_tap(Listener::handle_method(self, Self::handle_tap))
                .on_tap_down(Rc::new(move |app: &mut App, details| {
                    self.handle_tap_down(app, details)
                }))
                .on_tap_cancel(Listener::handle_method(self, Self::handle_tap_cancel))
                .on_hover(Rc::new(move |app: &mut App, value: bool| {
                    self.states_controller(app)
                        .update(app, WidgetState::Hovered, value);
                }));
        }
        if let Some(mouse_cursor) = widget.mouse_cursor.clone() {
            ink_well = ink_well.mouse_cursor(mouse_cursor);
        }
        if widget.color.is_some() || chip_theme.color.is_some() {
            ink_well = ink_well.hover_color(Colors::TRANSPARENT);
        }

        let ink_theme = Rc::clone(&theme);
        let ink_chip_theme = chip_theme.clone();
        let ink_defaults = Rc::clone(&chip_defaults);
        let ink_shape = resolved_shape.clone();
        let animated_ink = AnimatedBuilder::new(
            Rc::new(MergingListenable::new(vec![
                Some(Rc::new(self.select_controller(app))),
                Some(Rc::new(self.enable_controller(app))),
            ])),
            move |app, _context, child| {
                let color =
                    self.get_background_color(app, &ink_theme, &ink_chip_theme, &*ink_defaults);
                let mut decoration = ShapeDecoration::new(ink_shape.clone());
                if let Some(color) = color {
                    decoration = decoration.color(color);
                }
                Ink::new()
                    .decoration(decoration)
                    .child(child.cloned().expect("the builder's child"))
                    .into_widget()
            },
        );

        let delete_icon =
            AnimatedSwitcher::new(K_DRAWER_DURATION).switch_in_curve(Curves::fast_out_slow_in());
        let delete_icon =
            match self.build_delete_icon(app, context, &theme, &chip_theme, &*chip_defaults) {
                Some(child) => delete_icon.child(child),
                None => delete_icon,
            };
        let avatar_switcher =
            AnimatedSwitcher::new(K_DRAWER_DURATION).switch_in_curve(Curves::fast_out_slow_in());
        let avatar_switcher = match avatar {
            Some(child) => avatar_switcher.child(child),
            None => avatar_switcher,
        };

        let label = DefaultTextStyle::new(resolved_label_style, widget.label.clone())
            .overflow(TextOverflow::Fade)
            .text_align(TextAlign::Start)
            .max_lines(1)
            .soft_wrap(false);

        let chip_render = ChipRenderWidget {
            theme: ChipRenderTheme {
                label: label.into_widget(),
                avatar: avatar_switcher.into_widget(),
                delete_icon: delete_icon.into_widget(),
                brightness,
                padding: padding.resolve(text_direction),
                visual_density: widget.visual_density.unwrap_or(theme.visual_density),
                label_padding: label_padding.resolve(text_direction),
                show_avatar: has_avatar,
                show_checkmark,
                checkmark_color,
                can_tap_body: can_tap,
            },
            value: Some(widget.selected),
            is_enabled: Some(widget.is_enabled),
            checkmark_animation: self.checkmark_animation(app).erase(),
            avatar_drawer_animation: self.avatar_drawer_animation(app).erase(),
            delete_drawer_animation: self.delete_drawer_animation(app).erase(),
            enable_animation: self.enable_animation(app).erase(),
            avatar_border: Some(widget.avatar_border.clone()),
            avatar_box_constraints,
            delete_icon_box_constraints,
        };
        let tooltip_child = RawChipState::wrap_with_tooltip(
            widget.tooltip.clone(),
            widget.on_pressed.is_some() || widget.on_selected.is_some(),
            Some(chip_render.into_widget()),
        )
        .expect("the chip's body");
        let ink_well = ink_well.child(animated_ink.child(tooltip_child));
        let result: WidgetRef = material.child(ink_well).into_widget();

        let density_adjustment = widget
            .visual_density
            .unwrap_or(theme.visual_density)
            .base_size_adjustment();
        let constraints = match widget
            .material_tap_target_size
            .unwrap_or(theme.material_tap_target_size)
        {
            MaterialTapTargetSize::Padded => BoxConstraints {
                min_width: K_MIN_INTERACTIVE_DIMENSION + density_adjustment.dx(),
                min_height: K_MIN_INTERACTIVE_DIMENSION + density_adjustment.dy(),
                ..BoxConstraints::new()
            },
            MaterialTapTargetSize::ShrinkWrap => BoxConstraints::new(),
        };
        let result = ChipRedirectingHitDetectionWidget {
            constraints,
            child: Some(
                Center::new()
                    .width_factor(1.0)
                    .height_factor(1.0)
                    .child(result)
                    .into_widget(),
            ),
        }
        .into_widget();
        let mut semantics = Semantics::new()
            .button(widget.tap_enabled)
            .container(true)
            .child(result);
        // On web, aria-selected only works for certain roles: gridcell, option, row and tab.
        // https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Attributes/aria-selected
        // If the role doesn't support aria-selected, aria-current will be set instead in flutter engine.
        // But in this case, aria-checked makes more sense than aria-current for a selected chip.
        // So use checked on web instead.
        if K_IS_WEB {
            semantics = semantics.checked(widget.selected);
        } else {
            semantics = semantics.selected(widget.selected);
        }
        if widget.tap_enabled {
            semantics = semantics.enabled(can_tap);
        }
        semantics.into_widget()
    }
}

/// Dart's `ColorTween(begin: begin, end: end).transform(t)` of two colors that are not null:
/// [`Tween`](waft_animation::Tween)'s `transform` answers the ends themselves at 0.0 and
/// 1.0, and interpolates in between.
fn color_tween_transform(begin: Color, end: Color, t: f64) -> Color {
    if t == 0.0 {
        return begin;
    }
    if t == 1.0 {
        return end;
    }
    Color::lerp(Some(begin), Some(end), t).expect("both colors are given")
}

/// Dart's `_IndividualOverrides`.
#[derive(Debug)]
struct IndividualOverrides {
    color: Option<WidgetStatePropertyRef<Option<Color>>>,
    background_color: Option<Color>,
    selected_color: Option<Color>,
    disabled_color: Option<Color>,
}

impl IndividualOverrides {
    fn resolve(&self, states: &WidgetStates) -> Option<Color> {
        if let Some(color) = &self.color {
            return color.resolve(states);
        }
        if states.contains(&WidgetState::Selected) && states.contains(&WidgetState::Disabled) {
            return self.selected_color;
        }
        if states.contains(&WidgetState::Disabled) {
            return self.disabled_color;
        }
        if states.contains(&WidgetState::Selected) {
            return self.selected_color;
        }
        self.background_color
    }
}

/// Dart's `_ChipRedirectingHitDetectionWidget`.
#[derive(Debug)]
struct ChipRedirectingHitDetectionWidget {
    constraints: BoxConstraints,
    child: Option<WidgetRef>,
}

impl RenderObjectWidget for ChipRedirectingHitDetectionWidget {
    type RenderObject = RenderChipRedirectingHitDetection;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderChipRedirectingHitDetection::new(app, self.constraints).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderChipRedirectingHitDetection>,
    ) {
        render_object.set_additional_constraints(app, self.constraints);
    }
}

impl SingleChildRenderObjectWidget for ChipRedirectingHitDetectionWidget {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// Dart's `_RenderChipRedirectingHitDetection extends RenderConstrainedBox`: a
/// [`RenderConstrainedBoxBase`] leaf that overrides `hit_test`.
struct RenderChipRedirectingHitDetection {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    constrained_box: RenderConstrainedBoxData,
}

impl RenderChipRedirectingHitDetection {
    fn new(
        app: &mut App,
        additional_constraints: BoxConstraints,
    ) -> RenderHandle<RenderChipRedirectingHitDetection> {
        RenderHandle::new_box(
            app,
            RenderChipRedirectingHitDetection {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                constrained_box: RenderConstrainedBoxData::new(additional_constraints),
            },
        )
    }
}

impl RenderConstrainedBoxBase for RenderChipRedirectingHitDetection {
    fn constrained_box_data(self: RenderHandle<Self>, app: &App) -> &RenderConstrainedBoxData {
        &self.get(app).constrained_box
    }

    fn constrained_box_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut RenderConstrainedBoxData {
        &mut self.get_mut(app).constrained_box
    }
}

impl RenderObjectWithChildMixin for RenderChipRedirectingHitDetection {
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

impl RenderProxyBoxMixin for RenderChipRedirectingHitDetection {}

impl RenderObject for RenderChipRedirectingHitDetection {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderConstrainedBoxBase::perform_layout(self, app);
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

impl RenderBox for RenderChipRedirectingHitDetection {
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

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let size = self.size(app);
        if !size.contains(position) {
            return false;
        }
        // Only redirects hit detection which occurs above and below the render object.
        // In order to make this assumption true, I have removed the minimum width
        // constraints, since any reasonable chip would be at least that wide.
        let offset = Offset::new(position.dx(), size.height() / 2.0);
        let child = self.child(app).expect("the hit detection has a child");
        result.add_with_raw_transform(
            Some(force_to_point(offset)),
            position,
            |result, position| {
                debug_assert!(
                    (position.dx() - offset.dx()).abs() <= offset.dx().abs() * f32::EPSILON as f64
                        && (position.dy() - offset.dy()).abs()
                            <= offset.dy().abs() * f32::EPSILON as f64,
                    "the forced transform maps every hit to the given offset"
                );
                child.hit_test(app, result, offset)
            },
        )
    }

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderConstrainedBoxBase::compute_min_intrinsic_width(self, app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        RenderConstrainedBoxBase::compute_max_intrinsic_width(self, app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderConstrainedBoxBase::compute_min_intrinsic_height(self, app, width)
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        RenderConstrainedBoxBase::compute_max_intrinsic_height(self, app, width)
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
        RenderConstrainedBoxBase::compute_dry_baseline(self, app, constraints, baseline)
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        RenderConstrainedBoxBase::compute_dry_layout(self, app, constraints)
    }
}

/// Dart's `_ChipRenderWidget`.
struct ChipRenderWidget {
    theme: ChipRenderTheme,
    value: Option<bool>,
    is_enabled: Option<bool>,
    checkmark_animation: AnimationRef<f64>,
    avatar_drawer_animation: AnimationRef<f64>,
    delete_drawer_animation: AnimationRef<f64>,
    enable_animation: AnimationRef<f64>,
    avatar_border: Option<ShapeBorderRef>,
    avatar_box_constraints: Option<BoxConstraints>,
    delete_icon_box_constraints: Option<BoxConstraints>,
}

impl Debug for ChipRenderWidget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("_ChipRenderWidget")
            .field("value", &self.value)
            .field("is_enabled", &self.is_enabled)
            .finish_non_exhaustive()
    }
}

impl SlottedMultiChildRenderObjectWidget for ChipRenderWidget {
    type Slot = ChipSlot;

    fn slots(&self) -> Vec<ChipSlot> {
        vec![ChipSlot::Label, ChipSlot::Avatar, ChipSlot::DeleteIcon]
    }

    fn child_for_slot(&self, slot: &ChipSlot) -> Option<WidgetRef> {
        Some(match slot {
            ChipSlot::Label => self.theme.label.clone(),
            ChipSlot::Avatar => self.theme.avatar.clone(),
            ChipSlot::DeleteIcon => self.theme.delete_icon.clone(),
        })
    }
}

impl RenderObjectWidget for ChipRenderWidget {
    type RenderObject = RenderChip;

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let text_direction = Directionality::of(app, context);
        RenderChip::new(
            app,
            self.theme.clone(),
            text_direction,
            self.value,
            self.is_enabled,
            self.checkmark_animation.clone(),
            self.avatar_drawer_animation.clone(),
            self.delete_drawer_animation.clone(),
            self.enable_animation.clone(),
            self.avatar_border.clone(),
            self.avatar_box_constraints,
            self.delete_icon_box_constraints,
        )
        .as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderChip>,
    ) {
        render_object.set_theme(app, self.theme.clone());
        let text_direction = Directionality::of(app, context);
        render_object.set_text_direction(app, text_direction);
        render_object.get_mut(app).value = self.value;
        render_object.get_mut(app).is_enabled = self.is_enabled;
        render_object.set_checkmark_animation(app, self.checkmark_animation.clone());
        render_object.set_avatar_drawer_animation(app, self.avatar_drawer_animation.clone());
        render_object.set_delete_drawer_animation(app, self.delete_drawer_animation.clone());
        render_object.set_enable_animation(app, self.enable_animation.clone());
        render_object.get_mut(app).avatar_border = self.avatar_border.clone();
        render_object.set_avatar_box_constraints(app, self.avatar_box_constraints);
        render_object.set_delete_icon_box_constraints(app, self.delete_icon_box_constraints);
    }
}

/// Dart's `_ChipSlot`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ChipSlot {
    Label,
    Avatar,
    DeleteIcon,
}

/// Dart's `_ChipRenderTheme`.
#[derive(Clone)]
struct ChipRenderTheme {
    avatar: WidgetRef,
    label: WidgetRef,
    delete_icon: WidgetRef,
    brightness: Brightness,
    padding: EdgeInsets,
    visual_density: VisualDensity,
    label_padding: EdgeInsets,
    show_avatar: bool,
    show_checkmark: bool,
    checkmark_color: Option<Color>,
    can_tap_body: bool,
}

/// Dart's `==`, which compares the widgets by identity.
impl PartialEq for ChipRenderTheme {
    fn eq(&self, other: &ChipRenderTheme) -> bool {
        Rc::ptr_eq(&other.avatar, &self.avatar)
            && Rc::ptr_eq(&other.label, &self.label)
            && Rc::ptr_eq(&other.delete_icon, &self.delete_icon)
            && other.brightness == self.brightness
            && other.padding == self.padding
            && other.label_padding == self.label_padding
            && other.show_avatar == self.show_avatar
            && other.show_checkmark == self.show_checkmark
            && other.checkmark_color == self.checkmark_color
            && other.can_tap_body == self.can_tap_body
    }
}

/// Dart's `_RenderChip`.
struct RenderChip {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    slotted: SlottedContainerRenderObjectData<ChipSlot, AnyRenderBox>,
    value: Option<bool>,
    is_enabled: Option<bool>,
    delete_button_rect: Rect,
    press_rect: Rect,
    checkmark_animation: AnimationRef<f64>,
    avatar_drawer_animation: AnimationRef<f64>,
    delete_drawer_animation: AnimationRef<f64>,
    enable_animation: AnimationRef<f64>,
    avatar_border: Option<ShapeBorderRef>,
    theme: ChipRenderTheme,
    text_direction: TextDirection,
    avatar_box_constraints: Option<BoxConstraints>,
    delete_icon_box_constraints: Option<BoxConstraints>,
    avatar_opacity_layer_handler: LayerHandle<Handle<OpacityLayer>>,
    label_opacity_layer_handler: LayerHandle<Handle<OpacityLayer>>,
    delete_icon_opacity_layer_handler: LayerHandle<Handle<OpacityLayer>>,
}

/// Dart's `_ChipSizes`.
struct ChipSizes {
    size: Size,
    overall: Size,
    content: f64,
    avatar: Size,
    label_constraints: BoxConstraints,
    label: Size,
    delete_icon: Size,
    density_adjustment: Offset,
}

/// The setter of one of `_RenderChip`'s properties, marking the chip as needing layout when
/// the value changes.
macro_rules! render_chip_layout_property {
    ($($setter:ident, $field:ident: $ty:ty;)*) => {
        $(
            fn $setter(self: RenderHandle<Self>, app: &mut App, value: $ty) {
                if self.get(app).$field == value {
                    return;
                }
                self.get_mut(app).$field = value;
                self.mark_needs_layout(app);
            }
        )*
    };
}

impl RenderChip {
    #[allow(clippy::too_many_arguments)]
    fn new(
        app: &mut App,
        theme: ChipRenderTheme,
        text_direction: TextDirection,
        value: Option<bool>,
        is_enabled: Option<bool>,
        checkmark_animation: AnimationRef<f64>,
        avatar_drawer_animation: AnimationRef<f64>,
        delete_drawer_animation: AnimationRef<f64>,
        enable_animation: AnimationRef<f64>,
        avatar_border: Option<ShapeBorderRef>,
        avatar_box_constraints: Option<BoxConstraints>,
        delete_icon_box_constraints: Option<BoxConstraints>,
    ) -> RenderHandle<RenderChip> {
        RenderHandle::new_box(
            app,
            RenderChip {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                slotted: SlottedContainerRenderObjectData::new(),
                value,
                is_enabled,
                delete_button_rect: Rect::ZERO,
                press_rect: Rect::ZERO,
                checkmark_animation,
                avatar_drawer_animation,
                delete_drawer_animation,
                enable_animation,
                avatar_border,
                theme,
                text_direction,
                avatar_box_constraints,
                delete_icon_box_constraints,
                avatar_opacity_layer_handler: LayerHandle::new(),
                label_opacity_layer_handler: LayerHandle::new(),
                delete_icon_opacity_layer_handler: LayerHandle::new(),
            },
        )
    }

    fn avatar(self: RenderHandle<Self>, app: &App) -> AnyRenderBox {
        self.child_for_slot(app, &ChipSlot::Avatar)
            .expect("a chip always has an avatar slot")
    }

    fn delete_icon(self: RenderHandle<Self>, app: &App) -> AnyRenderBox {
        self.child_for_slot(app, &ChipSlot::DeleteIcon)
            .expect("a chip always has a delete icon slot")
    }

    fn label(self: RenderHandle<Self>, app: &App) -> AnyRenderBox {
        self.child_for_slot(app, &ChipSlot::Label)
            .expect("a chip always has a label slot")
    }

    fn theme(self: RenderHandle<Self>, app: &App) -> ChipRenderTheme {
        self.get(app).theme.clone()
    }

    fn set_theme(self: RenderHandle<Self>, app: &mut App, value: ChipRenderTheme) {
        if self.get(app).theme == value {
            return;
        }
        self.get_mut(app).theme = value;
        self.mark_needs_layout(app);
    }

    render_chip_layout_property! {
        set_text_direction, text_direction: TextDirection;
        set_avatar_box_constraints, avatar_box_constraints: Option<BoxConstraints>;
        set_delete_icon_box_constraints, delete_icon_box_constraints: Option<BoxConstraints>;
    }

    fn set_checkmark_animation(self: RenderHandle<Self>, app: &mut App, value: AnimationRef<f64>) {
        self.get_mut(app).checkmark_animation = value;
    }

    fn set_avatar_drawer_animation(
        self: RenderHandle<Self>,
        app: &mut App,
        value: AnimationRef<f64>,
    ) {
        self.get_mut(app).avatar_drawer_animation = value;
    }

    fn set_delete_drawer_animation(
        self: RenderHandle<Self>,
        app: &mut App,
        value: AnimationRef<f64>,
    ) {
        self.get_mut(app).delete_drawer_animation = value;
    }

    fn set_enable_animation(self: RenderHandle<Self>, app: &mut App, value: AnimationRef<f64>) {
        self.get_mut(app).enable_animation = value;
    }

    fn is_drawing_checkmark(self: RenderHandle<Self>, app: &App) -> bool {
        self.theme(app).show_checkmark && !self.get(app).checkmark_animation.is_dismissed(app)
    }

    fn delete_icon_showing(self: RenderHandle<Self>, app: &App) -> bool {
        !self.get(app).delete_drawer_animation.is_dismissed(app)
    }

    fn box_rect(app: &App, r#box: AnyRenderBox) -> Rect {
        r#box.box_parent_data(app).offset & r#box.size(app)
    }

    fn layout_avatar(
        self: RenderHandle<Self>,
        app: &mut App,
        content_size: f64,
        layout_child: ChildLayouter,
    ) -> Size {
        let avatar_constraints = self
            .get(app)
            .avatar_box_constraints
            .unwrap_or_else(|| BoxConstraints::tight_for(Some(content_size), Some(content_size)));
        let avatar = self.avatar(app);
        let avatar_box_size = layout_child(app, avatar, avatar_constraints);
        let theme = self.theme(app);
        if !theme.show_checkmark && !theme.show_avatar {
            return Size::new(0.0, content_size);
        }
        let avatar_full_width = if theme.show_avatar {
            avatar_box_size.width()
        } else {
            content_size
        };
        Size::new(
            avatar_full_width * self.get(app).avatar_drawer_animation.value(app),
            avatar_box_size.height(),
        )
    }

    fn layout_delete_icon(
        self: RenderHandle<Self>,
        app: &mut App,
        content_size: f64,
        layout_child: ChildLayouter,
    ) -> Size {
        let delete_icon_constraints = self
            .get(app)
            .delete_icon_box_constraints
            .unwrap_or_else(|| BoxConstraints::tight_for(Some(content_size), Some(content_size)));
        let delete_icon = self.delete_icon(app);
        let box_size = layout_child(app, delete_icon, delete_icon_constraints);
        if !self.delete_icon_showing(app) {
            return Size::new(0.0, content_size);
        }
        Size::new(
            self.get(app).delete_drawer_animation.value(app) * box_size.width(),
            box_size.height(),
        )
    }

    fn label_constraints_from(
        self: RenderHandle<Self>,
        app: &App,
        content_constraints: BoxConstraints,
        icon_width: f64,
        content_size: f64,
        raw_label_size: Size,
    ) -> BoxConstraints {
        let theme = self.theme(app);
        // Now that we know the label height and the width of the icons, we can
        // determine how much to shrink the width constraints for the "real" layout.
        let free_space = content_constraints.max_width
            - icon_width
            - theme.label_padding.horizontal()
            - theme.padding.horizontal();
        let max_label_width = free_space.max(0.0);
        BoxConstraints {
            min_height: raw_label_size.height(),
            max_height: content_size,
            max_width: if max_label_width.is_finite() {
                max_label_width
            } else {
                raw_label_size.width()
            },
            ..BoxConstraints::new()
        }
    }

    fn compute_sizes(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        layout_child: ChildLayouter,
    ) -> ChipSizes {
        let theme = self.theme(app);
        let content_constraints = constraints.loosen();
        // Find out the height of the label within the constraints.
        let label = self.label(app);
        let raw_label_size = label.get_dry_layout(app, content_constraints);
        let content_size = (K_CHIP_HEIGHT - theme.padding.vertical()
            + theme.label_padding.vertical())
        .max(raw_label_size.height() + theme.label_padding.vertical());
        debug_assert!(content_size >= raw_label_size.height());
        let avatar_size = self.layout_avatar(app, content_size, layout_child);
        let delete_icon_size = self.layout_delete_icon(app, content_size, layout_child);

        let label_constraints = self.label_constraints_from(
            app,
            content_constraints,
            avatar_size.width() + delete_icon_size.width(),
            content_size,
            raw_label_size,
        );

        let label_size = EdgeInsetsGeometry::from(theme.label_padding).inflate_size(layout_child(
            app,
            label,
            label_constraints,
        ));
        let density_adjustment =
            Offset::new(0.0, theme.visual_density.base_size_adjustment().dy() / 2.0);
        // This is the overall size of the content: it doesn't include
        // theme.padding, that is added in at the end.
        let overall_size = Size::new(
            avatar_size.width() + label_size.width() + delete_icon_size.width(),
            content_size,
        ) + density_adjustment;
        let padded_size = Size::new(
            overall_size.width() + theme.padding.horizontal(),
            overall_size.height() + theme.padding.vertical(),
        );

        ChipSizes {
            size: constraints.constrain(padded_size),
            overall: overall_size,
            content: content_size,
            density_adjustment,
            avatar: avatar_size,
            label_constraints,
            label: label_size,
            delete_icon: delete_icon_size,
        }
    }

    fn disabled_color(self: RenderHandle<Self>, app: &App) -> Color {
        if self.get(app).enable_animation.is_completed(app) {
            return Colors::WHITE;
        }
        let color = match self.theme(app).brightness {
            Brightness::Light => Colors::WHITE,
            Brightness::Dark => Colors::BLACK,
        };
        let enable_animation = self.get(app).enable_animation.value(app);
        color_tween_transform(color.with_alpha(K_DISABLED_ALPHA), color, enable_animation)
    }

    fn paint_check(
        self: RenderHandle<Self>,
        app: &App,
        canvas: &mut Canvas,
        origin: Offset,
        size: f64,
    ) {
        let theme = self.theme(app);
        let mut paint_color =
            theme
                .checkmark_color
                .unwrap_or(match (theme.brightness, theme.show_avatar) {
                    (Brightness::Light, true) => Colors::WHITE,
                    (Brightness::Light, false) => Colors::BLACK.with_alpha(K_CHECKMARK_ALPHA),
                    (Brightness::Dark, true) => Colors::BLACK,
                    (Brightness::Dark, false) => Colors::WHITE.with_alpha(K_CHECKMARK_ALPHA),
                });

        let checkmark_animation = self.get(app).checkmark_animation.clone();
        let reversing = checkmark_animation.status(app) == AnimationStatus::Reverse;
        if reversing {
            paint_color = color_tween_transform(
                Colors::TRANSPARENT,
                paint_color,
                checkmark_animation.value(app),
            );
        }

        let avatar_height = self.avatar(app).size(app).height();
        let paint = Paint {
            color: paint_color.into(),
            style: PaintStyle::Stroke(Stroke::new(
                (K_CHECKMARK_STROKE_WIDTH * avatar_height / 24.0) as f32,
            )),
            ..Paint::default()
        };
        let t = if reversing {
            1.0
        } else {
            checkmark_animation.value(app)
        };
        if t == 0.0 {
            // Nothing to draw.
            return;
        }
        debug_assert!(t > 0.0 && t <= 1.0);
        // As t goes from 0.0 to 1.0, animate the two check mark strokes from the
        // short side to the long side.
        let mut path = PathBuilder::new();
        let start = Offset::new(size * 0.15, size * 0.45);
        let mid = Offset::new(size * 0.4, size * 0.7);
        let end = Offset::new(size * 0.85, size * 0.25);
        if t < 0.5 {
            let stroke_t = t * 2.0;
            let draw_mid = Offset::lerp(Some(start), Some(mid), stroke_t).expect("non-null");
            path.move_to(Offset::new(
                origin.dx() + start.dx(),
                origin.dy() + start.dy(),
            ));
            path.line_to(Offset::new(
                origin.dx() + draw_mid.dx(),
                origin.dy() + draw_mid.dy(),
            ));
        } else {
            let stroke_t = (t - 0.5) * 2.0;
            let draw_end = Offset::lerp(Some(mid), Some(end), stroke_t).expect("non-null");
            path.move_to(Offset::new(
                origin.dx() + start.dx(),
                origin.dy() + start.dy(),
            ));
            path.line_to(Offset::new(origin.dx() + mid.dx(), origin.dy() + mid.dy()));
            path.line_to(Offset::new(
                origin.dx() + draw_end.dx(),
                origin.dy() + draw_end.dy(),
            ));
        }
        canvas.draw_path(&path.build(), FillRule::NonZero, &paint);
    }

    fn paint_selection_overlay(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        if !self.is_drawing_checkmark(app) {
            return;
        }
        let theme = self.theme(app);
        let avatar = self.avatar(app);
        if theme.show_avatar {
            let avatar_rect = RenderChip::box_rect(app, avatar).shift(offset);
            let checkmark_animation = self.get(app).checkmark_animation.value(app);
            let scrim = color_tween_transform(
                Colors::TRANSPARENT,
                K_SELECT_SCRIM_COLOR,
                checkmark_animation,
            );
            let darken_paint = Paint {
                color: scrim.into(),
                blend_mode: BlendMode::SrcAtop,
                ..Paint::default()
            };
            let avatar_border = self
                .get(app)
                .avatar_border
                .clone()
                .expect("a chip with an avatar has an avatar border");
            if avatar_border.prefer_paint_interior() {
                avatar_border.paint_interior(context.canvas(), avatar_rect, &darken_paint, None);
            } else {
                let path = avatar_border.get_outer_path(avatar_rect, None);
                context
                    .canvas()
                    .draw_path(path.as_valo(), path.fill_type, &darken_paint);
            }
        }
        // Need to make the check mark be a little smaller than the avatar.
        let avatar_height = avatar.size(app).height();
        let check_size = avatar_height * 0.75;
        let check_offset = avatar.box_parent_data(app).offset
            + Offset::new(avatar_height * 0.125, avatar_height * 0.125);
        self.paint_check(app, context.canvas(), offset + check_offset, check_size);
    }

    fn paint_avatar(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        let avatar = self.avatar(app);
        let paint_with_overlay =
            move |app: &mut App, context: &mut PaintingContext, offset: Offset| {
                let child_offset = avatar.box_parent_data(app).offset + offset;
                context.paint_child(app, avatar.as_object(), child_offset);
                self.paint_selection_overlay(app, context, offset);
            };

        if !self.theme(app).show_avatar && self.get(app).avatar_drawer_animation.is_dismissed(app) {
            LayerHandle::set_layer(
                app,
                |app| &mut self.get_mut(app).avatar_opacity_layer_handler,
                None,
            );
            return;
        }
        let disabled_color = self.disabled_color(app);
        let disabled_color_alpha = alpha(disabled_color);
        if self.as_object().needs_compositing(app) {
            let old_layer = self.get(app).avatar_opacity_layer_handler.layer();
            let layer = context.push_opacity(
                app,
                offset,
                disabled_color_alpha,
                paint_with_overlay,
                old_layer,
            );
            LayerHandle::set_layer(
                app,
                |app| &mut self.get_mut(app).avatar_opacity_layer_handler,
                Some(layer),
            );
        } else {
            LayerHandle::set_layer(
                app,
                |app| &mut self.get_mut(app).avatar_opacity_layer_handler,
                None,
            );
            if disabled_color_alpha != 0xff {
                let rect = RenderChip::box_rect(app, avatar)
                    .shift(offset)
                    .inflate(20.0);
                context.canvas().save_layer(
                    Some(rect.into()),
                    &Paint {
                        color: disabled_color.into(),
                        ..Paint::default()
                    },
                );
            }
            paint_with_overlay(app, context, offset);
            if disabled_color_alpha != 0xff {
                context.canvas().restore();
            }
        }
    }

    fn paint_child(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
        child: AnyRenderBox,
        is_delete_icon: bool,
    ) {
        let disabled_color_alpha = alpha(self.disabled_color(app));
        if self.get(app).enable_animation.is_completed(app) {
            let child_offset = child.box_parent_data(app).offset + offset;
            context.paint_child(app, child.as_object(), child_offset);
            return;
        }
        if self.as_object().needs_compositing(app) {
            let old_layer = self.get(app).label_opacity_layer_handler.layer();
            let layer = context.push_opacity(
                app,
                offset,
                disabled_color_alpha,
                move |app: &mut App, context: &mut PaintingContext, offset: Offset| {
                    let child_offset = child.box_parent_data(app).offset + offset;
                    context.paint_child(app, child.as_object(), child_offset);
                },
                old_layer,
            );
            LayerHandle::set_layer(
                app,
                |app| &mut self.get_mut(app).label_opacity_layer_handler,
                Some(layer),
            );
            if is_delete_icon {
                let old_layer = self.get(app).delete_icon_opacity_layer_handler.layer();
                let layer = context.push_opacity(
                    app,
                    offset,
                    disabled_color_alpha,
                    move |app: &mut App, context: &mut PaintingContext, offset: Offset| {
                        let child_offset = child.box_parent_data(app).offset + offset;
                        context.paint_child(app, child.as_object(), child_offset);
                    },
                    old_layer,
                );
                LayerHandle::set_layer(
                    app,
                    |app| &mut self.get_mut(app).delete_icon_opacity_layer_handler,
                    Some(layer),
                );
            }
        } else {
            LayerHandle::set_layer(
                app,
                |app| &mut self.get_mut(app).label_opacity_layer_handler,
                None,
            );
            LayerHandle::set_layer(
                app,
                |app| &mut self.get_mut(app).delete_icon_opacity_layer_handler,
                None,
            );
            let child_rect = RenderChip::box_rect(app, child).shift(offset);
            let disabled_color = self.disabled_color(app);
            context.canvas().save_layer(
                Some(child_rect.inflate(20.0).into()),
                &Paint {
                    color: disabled_color.into(),
                    ..Paint::default()
                },
            );
            let child_offset = child.box_parent_data(app).offset + offset;
            context.paint_child(app, child.as_object(), child_offset);
            context.canvas().restore();
        }
    }
}

/// Free functions the animations' listeners are tear-offs of: a tear-off is a distinct `fn`
/// item per behaviour.
fn mark_chip_needs_paint(this: Handle<RenderChip>, app: &mut App) {
    RenderHandle::from_handle(this).mark_needs_paint(app);
}

fn mark_chip_needs_layout(this: Handle<RenderChip>, app: &mut App) {
    RenderHandle::from_handle(this).mark_needs_layout(app);
}

impl SlottedContainerRenderObjectMixin<ChipSlot> for RenderChip {
    type ChildType = AnyRenderBox;

    fn slotted_container_data(
        self: RenderHandle<Self>,
        app: &App,
    ) -> &SlottedContainerRenderObjectData<ChipSlot, AnyRenderBox> {
        &self.get(app).slotted
    }

    fn slotted_container_data_mut(
        self: RenderHandle<Self>,
        app: &mut App,
    ) -> &mut SlottedContainerRenderObjectData<ChipSlot, AnyRenderBox> {
        &mut self.get_mut(app).slotted
    }

    // The returned list is ordered for hit testing.
    fn children(self: RenderHandle<Self>, app: &App) -> Vec<AnyRenderBox> {
        let avatar = self.child_for_slot(app, &ChipSlot::Avatar);
        let label = self.child_for_slot(app, &ChipSlot::Label);
        let delete_icon = self.child_for_slot(app, &ChipSlot::DeleteIcon);
        [avatar, label, delete_icon].into_iter().flatten().collect()
    }
}

impl RenderObject for RenderChip {
    waft_rendering::render_object_accessors!();

    fn did_attach(self: RenderHandle<Self>, app: &mut App, owner: Handle<PipelineOwner>) {
        SlottedContainerRenderObjectMixin::did_attach(self, app, owner);
        let this = self.handle();
        let (checkmark, avatar_drawer, delete_drawer, enable) = {
            let chip = self.get(app);
            (
                chip.checkmark_animation.clone(),
                chip.avatar_drawer_animation.clone(),
                chip.delete_drawer_animation.clone(),
                chip.enable_animation.clone(),
            )
        };
        checkmark.add_listener(app, Listener::handle_method(this, mark_chip_needs_paint));
        avatar_drawer.add_listener(app, Listener::handle_method(this, mark_chip_needs_layout));
        delete_drawer.add_listener(app, Listener::handle_method(this, mark_chip_needs_layout));
        enable.add_listener(app, Listener::handle_method(this, mark_chip_needs_paint));
    }

    fn did_detach(self: RenderHandle<Self>, app: &mut App) {
        let this = self.handle();
        let (checkmark, avatar_drawer, delete_drawer, enable) = {
            let chip = self.get(app);
            (
                chip.checkmark_animation.clone(),
                chip.avatar_drawer_animation.clone(),
                chip.delete_drawer_animation.clone(),
                chip.enable_animation.clone(),
            )
        };
        checkmark.remove_listener(app, &Listener::handle_method(this, mark_chip_needs_paint));
        avatar_drawer.remove_listener(app, &Listener::handle_method(this, mark_chip_needs_layout));
        delete_drawer.remove_listener(app, &Listener::handle_method(this, mark_chip_needs_layout));
        enable.remove_listener(app, &Listener::handle_method(this, mark_chip_needs_paint));
        SlottedContainerRenderObjectMixin::did_detach(self, app);
    }

    fn redepth_children(self: RenderHandle<Self>, app: &mut App) {
        SlottedContainerRenderObjectMixin::redepth_children(self, app);
    }

    fn visit_children(
        self: RenderHandle<Self>,
        app: &App,
        visitor: &mut dyn FnMut(AnyRenderObject),
    ) {
        SlottedContainerRenderObjectMixin::visit_children(self, app, visitor);
    }

    fn dispose(self: RenderHandle<Self>, app: &mut App) {
        LayerHandle::set_layer(
            app,
            |app| &mut self.get_mut(app).label_opacity_layer_handler,
            None,
        );
        LayerHandle::set_layer(
            app,
            |app| &mut self.get_mut(app).delete_icon_opacity_layer_handler,
            None,
        );
        LayerHandle::set_layer(
            app,
            |app| &mut self.get_mut(app).avatar_opacity_layer_handler,
            None,
        );
        waft_rendering::RenderObjectBase::dispose(self, app);
    }

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        let constraints = self.constraints(app);
        let sizes = self.compute_sizes(app, constraints, ChildLayoutHelper::layout_child);
        let theme = self.theme(app);
        let text_direction = self.get(app).text_direction;
        let avatar = self.avatar(app);
        let label = self.label(app);
        let delete_icon = self.delete_icon(app);

        // Now we have all of the dimensions. Place the children where they belong.

        let left = 0.0;
        let right = sizes.overall.width();

        let center_layout = |box_size: Size, x: f64| -> Offset {
            debug_assert!(sizes.content >= box_size.height());
            let x = match text_direction {
                TextDirection::Rtl => x - box_size.width(),
                TextDirection::Ltr => x,
            };
            Offset::new(
                x,
                (sizes.content - box_size.height() + sizes.density_adjustment.dy()) / 2.0,
            )
        };

        // These are the offsets to the upper left corners of the boxes (including
        // the child's padding) containing the children, for each child, but not
        // including the overall padding.
        let mut avatar_offset = Offset::ZERO;
        let mut label_offset;
        let mut delete_icon_offset = Offset::ZERO;
        let delete_icon_showing = self.delete_icon_showing(app);
        let delete_button_rect;
        let press_rect;
        match text_direction {
            TextDirection::Rtl => {
                let mut start = right;
                if theme.show_checkmark || theme.show_avatar {
                    avatar_offset = center_layout(sizes.avatar, start);
                    start -= sizes.avatar.width();
                }
                label_offset = center_layout(sizes.label, start);
                start -= sizes.label.width();
                if delete_icon_showing {
                    delete_button_rect = Rect::from_ltwh(
                        0.0,
                        0.0,
                        sizes.delete_icon.width() + theme.padding.right,
                        sizes.overall.height() + theme.padding.vertical(),
                    );
                    delete_icon_offset = center_layout(sizes.delete_icon, start);
                } else {
                    delete_button_rect = Rect::ZERO;
                }
                if theme.can_tap_body {
                    press_rect = Rect::from_ltwh(
                        delete_button_rect.width(),
                        0.0,
                        sizes.overall.width() - delete_button_rect.width()
                            + theme.padding.horizontal(),
                        sizes.overall.height() + theme.padding.vertical(),
                    );
                } else {
                    press_rect = Rect::ZERO;
                }
            }
            TextDirection::Ltr => {
                let mut start = left;
                if theme.show_checkmark || theme.show_avatar {
                    avatar_offset = center_layout(
                        sizes.avatar,
                        start - avatar.size(app).width() + sizes.avatar.width(),
                    );
                    start += sizes.avatar.width();
                }
                label_offset = center_layout(sizes.label, start);
                start += sizes.label.width();
                if theme.can_tap_body {
                    press_rect = Rect::from_ltwh(
                        0.0,
                        0.0,
                        if delete_icon_showing {
                            start + theme.padding.left
                        } else {
                            sizes.overall.width() + theme.padding.horizontal()
                        },
                        sizes.overall.height() + theme.padding.vertical(),
                    );
                } else {
                    press_rect = Rect::ZERO;
                }
                start -= delete_icon.size(app).width() - sizes.delete_icon.width();
                if delete_icon_showing {
                    delete_icon_offset = center_layout(sizes.delete_icon, start);
                    delete_button_rect = Rect::from_ltwh(
                        start + theme.padding.left,
                        0.0,
                        sizes.delete_icon.width() + theme.padding.right,
                        sizes.overall.height() + theme.padding.vertical(),
                    );
                } else {
                    delete_button_rect = Rect::ZERO;
                }
            }
        }
        // Center the label vertically.
        label_offset = label_offset
            + Offset::new(
                0.0,
                ((sizes.label.height() - theme.label_padding.vertical())
                    - label.size(app).height())
                    / 2.0,
            );
        avatar.parent_data_of_mut::<BoxParentData>(app).offset =
            theme.padding.top_left() + avatar_offset;
        label.parent_data_of_mut::<BoxParentData>(app).offset =
            theme.padding.top_left() + label_offset + theme.label_padding.top_left();
        delete_icon.parent_data_of_mut::<BoxParentData>(app).offset =
            theme.padding.top_left() + delete_icon_offset;
        self.get_mut(app).delete_button_rect = delete_button_rect;
        self.get_mut(app).press_rect = press_rect;
        let padded_size = Size::new(
            sizes.overall.width() + theme.padding.horizontal(),
            sizes.overall.height() + theme.padding.vertical(),
        );
        let size = constraints.constrain(padded_size);
        self.set_size(app, size);
        debug_assert!(
            size.height() == constraints.constrain_height(padded_size.height()),
            "Constrained height {} doesn't match expected height {}",
            size.height(),
            constraints.constrain_width(padded_size.height()),
        );
        debug_assert!(
            size.width() == constraints.constrain_width(padded_size.width()),
            "Constrained width {} doesn't match expected width {}",
            size.width(),
            constraints.constrain_width(padded_size.width()),
        );
    }

    fn paint(
        self: RenderHandle<Self>,
        app: &mut App,
        context: &mut PaintingContext,
        offset: Offset,
    ) {
        self.paint_avatar(app, context, offset);
        if self.delete_icon_showing(app) {
            let delete_icon = self.delete_icon(app);
            self.paint_child(app, context, offset, delete_icon, true);
        }
        let label = self.label(app);
        self.paint_child(app, context, offset, label, false);
    }
}

impl RenderBox for RenderChip {
    waft_rendering::render_box_accessors!();

    fn compute_min_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        // The overall padding isn't affected by missing avatar or delete icon
        // because we add the padding regardless to give extra padding for the label
        // when they're missing.
        let theme = self.theme(app);
        let overall_padding = theme.padding.horizontal() + theme.label_padding.horizontal();
        overall_padding
            + self.avatar(app).get_min_intrinsic_width(app, height)
            + self.label(app).get_min_intrinsic_width(app, height)
            + self.delete_icon(app).get_min_intrinsic_width(app, height)
    }

    fn compute_max_intrinsic_width(self: RenderHandle<Self>, app: &mut App, height: f64) -> f64 {
        let theme = self.theme(app);
        let overall_padding = theme.padding.horizontal() + theme.label_padding.horizontal();
        overall_padding
            + self.avatar(app).get_max_intrinsic_width(app, height)
            + self.label(app).get_max_intrinsic_width(app, height)
            + self.delete_icon(app).get_max_intrinsic_width(app, height)
    }

    fn compute_min_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        let theme = self.theme(app);
        K_CHIP_HEIGHT.max(
            theme.padding.vertical()
                + theme.label_padding.vertical()
                + self.label(app).get_min_intrinsic_height(app, width),
        )
    }

    fn compute_max_intrinsic_height(self: RenderHandle<Self>, app: &mut App, width: f64) -> f64 {
        self.as_box().get_min_intrinsic_height(app, width)
    }

    fn compute_distance_to_actual_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        baseline: TextBaseline,
    ) -> Option<f64> {
        // The baseline of this widget is the baseline of the label.
        let label = self.label(app);
        let offset_dy = label.box_parent_data(app).offset.dy();
        (BaselineOffset(label.get_distance_to_actual_baseline(app, baseline)) + offset_dy).offset()
    }

    fn hit_test(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        let size = self.size(app);
        if !size.contains(position) {
            return false;
        }
        let theme = self.theme(app);
        let delete_icon = self.delete_icon(app);
        let hit_is_on_delete_icon = hit_is_on_delete_icon(
            &theme.padding.into(),
            &theme.label_padding.into(),
            position,
            size,
            delete_icon.size(app),
            self.get(app).text_direction,
        );
        let hit_test_child = if hit_is_on_delete_icon {
            delete_icon
        } else {
            self.label(app)
        };

        let center = hit_test_child.size(app).center(Offset::ZERO);
        result.add_with_raw_transform(
            Some(force_to_point(center)),
            position,
            |result, position| {
                debug_assert!(
                    (position.dx() - center.dx()).abs() <= center.dx().abs() * f32::EPSILON as f64
                        && (position.dy() - center.dy()).abs()
                            <= center.dy().abs() * f32::EPSILON as f64,
                    "the forced transform maps every hit to the child's center"
                );
                hit_test_child.hit_test(app, result, center)
            },
        )
    }

    fn compute_dry_layout(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
    ) -> Size {
        self.compute_sizes(app, constraints, ChildLayoutHelper::dry_layout_child)
            .size
    }

    fn compute_dry_baseline(
        self: RenderHandle<Self>,
        app: &mut App,
        constraints: BoxConstraints,
        baseline: TextBaseline,
    ) -> Option<f64> {
        let sizes = self.compute_sizes(app, constraints, ChildLayoutHelper::dry_layout_child);
        let theme = self.theme(app);
        let label_baseline =
            BaselineOffset(self.label(app).get_dry_baseline(
                app,
                sizes.label_constraints,
                baseline,
            )) + (sizes.content - sizes.label.height() + sizes.density_adjustment.dy()) / 2.0
                + theme.padding.top
                + theme.label_padding.top;
        label_baseline.offset()
    }

    fn hit_test_self(self: RenderHandle<Self>, app: &App, position: Offset) -> bool {
        let chip = self.get(app);
        chip.delete_button_rect.contains(position) || chip.press_rect.contains(position)
    }

    fn hit_test_children(
        self: RenderHandle<Self>,
        app: &mut App,
        result: &mut BoxHitTestResult<'_>,
        position: Offset,
    ) -> bool {
        for child in SlottedContainerRenderObjectMixin::children(self, app) {
            let offset = child.box_parent_data(app).offset;
            let is_hit =
                result.add_with_paint_offset(Some(offset), position, |result, transformed| {
                    child.hit_test(app, result, transformed)
                });
            if is_hit {
                return true;
            }
        }
        false
    }
}

/// Dart's `_UnconstrainedInkSplashFactory`.
#[derive(Debug)]
struct UnconstrainedInkSplashFactory {
    parent_factory: Rc<dyn InteractiveInkFeatureFactory>,
}

impl InteractiveInkFeatureFactory for UnconstrainedInkSplashFactory {
    fn create(
        &self,
        app: &mut App,
        controller: AnyMaterialInkController,
        reference_box: AnyRenderBox,
        position: Offset,
        color: Color,
        text_direction: TextDirection,
        _contained_ink_well: bool,
        rect_callback: Option<RectCallback>,
        border_radius: Option<BorderRadius>,
        custom_border: Option<ShapeBorderRef>,
        radius: Option<f64>,
        on_removed: Option<Listener>,
    ) -> InteractiveInkFeatureRef {
        self.parent_factory.create(
            app,
            controller,
            reference_box,
            position,
            color,
            text_direction,
            // Dart's `create` call leaves `containedInkWell` out, so it is the default false.
            false,
            rect_callback,
            border_radius,
            custom_border,
            radius,
            on_removed,
        )
    }
}

/// Dart's `Color.alpha`, which chip.dart still reads although it is deprecated upstream.
#[expect(deprecated, reason = "the getter Dart's `disabledColor.alpha` reads")]
fn alpha(color: Color) -> i32 {
    color.alpha()
}

/// Dart's `_hitIsOnDeleteIcon`.
fn hit_is_on_delete_icon(
    padding: &EdgeInsetsGeometry,
    label_padding: &EdgeInsetsGeometry,
    tap_position: Offset,
    chip_size: Size,
    delete_button_size: Size,
    text_direction: TextDirection,
) -> bool {
    // The chipSize includes the padding, so we need to deflate the size and adjust the
    // tap position to account for the padding.
    let resolved_padding = padding.resolve(Some(text_direction));
    let deflated_size = EdgeInsetsGeometry::from(resolved_padding).deflate_size(chip_size);
    let adjusted_position = tap_position - Offset::new(resolved_padding.left, resolved_padding.top);
    // The delete button hit area should be at least the width of the delete
    // button and right label padding, but, if there's room, up to 24 pixels
    // from the center of the delete icon (corresponding to part of a 48x48 square
    // that Material would prefer for touch targets), but no more than approximately
    // half of the overall size of the chip when the chip is small.
    //
    // This isn't affected by materialTapTargetSize because it only applies to the
    // width of the tappable region within the chip, not outside of the chip,
    // which is handled elsewhere. Also because delete buttons aren't specified to
    // be used on touch devices, only desktop devices.

    // Max out at not quite half, so that tests that tap on the center of a small
    // chip will still hit the chip, not the delete button.
    let accessible_delete_button_width = (deflated_size.width() * 0.499).min(
        (label_padding.resolve(Some(text_direction)).right + delete_button_size.width())
            .min(24.0 + delete_button_size.width() / 2.0),
    );
    match text_direction {
        TextDirection::Ltr => {
            adjusted_position.dx() >= deflated_size.width() - accessible_delete_button_width
        }
        TextDirection::Rtl => adjusted_position.dx() <= accessible_delete_button_width,
    }
}

/// Dart's `_EnsureMinSemanticsSize`.
#[derive(Debug)]
struct EnsureMinSemanticsSize {
    semantic_size: Size,
    child: Option<WidgetRef>,
}

impl RenderObjectWidget for EnsureMinSemanticsSize {
    type RenderObject = RenderEnsureMinSemanticsSize;

    fn create_render_object(&self, app: &mut App, _context: BuildContext) -> AnyRenderObject {
        RenderEnsureMinSemanticsSize::new(app, self.semantic_size).as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        _context: BuildContext,
        render_object: RenderHandle<RenderEnsureMinSemanticsSize>,
    ) {
        render_object.set_semantic_size(app, self.semantic_size);
    }
}

impl SingleChildRenderObjectWidget for EnsureMinSemanticsSize {
    fn child(&self) -> Option<&WidgetRef> {
        self.child.as_ref()
    }
}

/// Dart's `_RenderEnsureMinSemanticsSize extends RenderProxyBox`.
struct RenderEnsureMinSemanticsSize {
    render_object: RenderObjectData,
    render_box: RenderBoxData,
    child: RenderObjectWithChildData<AnyRenderBox>,
    semantic_size: Size,
}

impl RenderEnsureMinSemanticsSize {
    fn new(app: &mut App, semantic_size: Size) -> RenderHandle<RenderEnsureMinSemanticsSize> {
        RenderHandle::new_box(
            app,
            RenderEnsureMinSemanticsSize {
                render_object: RenderObjectData::new(),
                render_box: RenderBoxData::new(),
                child: RenderObjectWithChildData::new(),
                semantic_size,
            },
        )
    }

    fn set_semantic_size(self: RenderHandle<Self>, app: &mut App, value: Size) {
        if self.get(app).semantic_size == value {
            return;
        }
        // Dart marks the semantics as needing an update, which waits with the semantics tree.
        self.get_mut(app).semantic_size = value;
    }
}

impl RenderObjectWithChildMixin for RenderEnsureMinSemanticsSize {
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

impl RenderProxyBoxMixin for RenderEnsureMinSemanticsSize {}

impl RenderObject for RenderEnsureMinSemanticsSize {
    waft_rendering::render_object_accessors!();

    fn perform_layout(self: RenderHandle<Self>, app: &mut App) {
        RenderProxyBoxMixin::perform_layout(self, app);
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

impl RenderBox for RenderEnsureMinSemanticsSize {
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
pub(crate) mod tests {
    use std::cell::{Cell, RefCell};

    use waft_embedder::{Offset, Rect, Size};
    use waft_painting::{BorderRadiusGeometry, RoundedRectangleBorder, TextScaler};
    use waft_widgets::{
        AnyElement, Column, CustomPaint, Expanded, Flexible, GlobalKey, IconData, MediaQueryData,
        Row, SizedBox, Text, Wrap,
    };

    use waft_widgets::testing::{
        WidgetTester, assert_more_or_less_equals_within, find, test_widgets,
    };

    use super::*;
    use crate::circle_avatar::CircleAvatar;
    use crate::scaffold::Scaffold;
    use crate::test_support::PaintInspection;

    use crate::tooltip::tests::m3;
    use waft_rendering::testing::{PaintCall, paints};

    /// Dart's `wrapForChip`: the basic requirements for a chip.
    pub(crate) fn wrap_for_chip(
        tester: &WidgetTester,
        child: WidgetRef,
        text_direction: TextDirection,
        text_scaler: TextScaler,
        theme: Option<ThemeData>,
    ) -> WidgetRef {
        let theme = theme.unwrap_or_else(|| m3(&tester.cell.borrow()));
        crate::test_support::material_app(
            theme,
            Directionality::new(
                text_direction,
                MediaQuery::new(
                    MediaQueryData::new().text_scaler(text_scaler),
                    Material::new().child(child).into_widget(),
                )
                .into_widget(),
            )
            .into_widget(),
        )
    }

    pub(crate) fn wrap_ltr<K>(tester: &WidgetTester, child: impl IntoWidget<K>) -> WidgetRef {
        wrap_for_chip(
            tester,
            child.into_widget(),
            TextDirection::Ltr,
            TextScaler::NO_SCALING,
            None,
        )
    }

    pub(crate) fn wrap_with_theme<K>(
        tester: &WidgetTester,
        theme: ThemeData,
        child: impl IntoWidget<K>,
    ) -> WidgetRef {
        wrap_for_chip(
            tester,
            child.into_widget(),
            TextDirection::Ltr,
            TextScaler::NO_SCALING,
            Some(theme),
        )
    }

    pub(crate) fn rect_of(tester: &WidgetTester, element: AnyElement) -> Rect {
        tester.get_rect(tester.render_box_of(element))
    }

    pub(crate) fn center_of(tester: &WidgetTester, element: AnyElement) -> Offset {
        tester.get_rect(element).center()
    }

    pub(crate) fn only<W: 'static>(tester: &WidgetTester) -> AnyElement {
        let elements = find::by_type::<W>().evaluate(tester);
        assert_eq!(elements.len(), 1, "one {}", std::any::type_name::<W>());
        elements[0]
    }

    pub(crate) fn text(tester: &WidgetTester, text: &str) -> AnyElement {
        *find::text(text)
            .evaluate(tester)
            .first()
            .unwrap_or_else(|| panic!("a Text showing {text:?}"))
    }

    /// Dart's `find.byKey(key)`.
    pub(crate) fn by_key(tester: &WidgetTester, key: &GlobalKey) -> AnyElement {
        let key: KeyRef = key.clone().into();
        *tester
            .find_onstage_by_key(&key)
            .first()
            .expect("an element with the key")
    }

    /// Dart's `getMaterial(tester)`: the `Material` under the `RawChip`.
    /// Dart's `expect(getMaterialBox(tester), paints..calls)`: what the first `CustomPaint` below
    /// the `RawChip`, and everything under it, paints.
    pub(crate) fn material_box_paints(tester: &WidgetTester, calls: &[PaintCall]) -> bool {
        let custom_paint = tester.descendants_of_type::<CustomPaint>(only::<RawChip>(tester))[0];
        let ops = tester.painted_ops_deep(tester.render_box_of(custom_paint).as_object());
        paints(&ops, calls).is_ok()
    }

    pub(crate) fn material_of_raw_chip(tester: &WidgetTester) -> AnyElement {
        let raw_chip = only::<RawChip>(tester);
        tester.descendants_of_type::<Material>(raw_chip)[0]
    }

    /// Dart's `getIconData(tester)`.
    pub(crate) fn icon_data(tester: &WidgetTester) -> IconThemeData {
        let raw_chip = only::<RawChip>(tester);
        let icon_theme = tester.descendants_of_type::<IconTheme>(raw_chip)[0];
        tester.widget(icon_theme, |icon_theme: &IconTheme| icon_theme.data.clone())
    }

    /// Dart's `getLabelStyle(tester, labelText)`.
    pub(crate) fn label_style(tester: &WidgetTester, label_text: &str) -> TextStyle {
        let label = text(tester, label_text);
        let default_text_style = tester
            .ancestors(label)
            .into_iter()
            .find(|element| {
                let app = tester.cell.borrow();
                waft_widgets::downcast_widget::<DefaultTextStyle>(&**element.widget(&app)).is_some()
            })
            .expect("a DefaultTextStyle above the label");
        tester.widget(default_text_style, |style: &DefaultTextStyle| {
            style.style.clone()
        })
    }

    /// Dart's `tester.tap(find.byTooltip(message))` target: the element of the `Tooltip`.
    pub(crate) fn by_tooltip(tester: &WidgetTester, message: &str) -> AnyElement {
        *find::by_type::<Tooltip>()
            .evaluate(tester)
            .iter()
            .find(|element| {
                tester.widget(**element, |tooltip: &Tooltip| {
                    tooltip.message.as_deref() == Some(message)
                })
            })
            .unwrap_or_else(|| panic!("a Tooltip with the message {message:?}"))
    }

    /// Dart's `testConstrainedLabel`: a chip that its parent constrains further constrains
    /// its label, with an avatar or a delete icon or neither.
    pub(crate) fn test_constrained_label(with_avatar: bool, with_delete: bool) {
        test_widgets(|tester| {
            let label_width = 100.0;
            let label_height = 50.0;
            let chip_parent_width = 75.0;
            let chip_parent_height = 25.0;
            let label_key = GlobalKey::new();

            let mut chip = Chip::new(
                SizedBox::new()
                    .key(label_key.clone().into())
                    .width(label_width)
                    .height(label_height),
            );
            if with_avatar {
                chip = chip.avatar(CircleAvatar::new().child(Text::new("A")));
            }
            if with_delete {
                chip = chip.on_deleted(Listener::new(|_app| {}));
            }
            let root = wrap_ltr(
                tester,
                Center::new().child(
                    SizedBox::new()
                        .width(chip_parent_width)
                        .height(chip_parent_height)
                        .child(chip),
                ),
            );
            tester.pump_widget(root);

            let label_size = tester.get_size(by_key(tester, &label_key));
            assert!(label_size.width() < chip_parent_width);
            assert!(label_size.height() <= chip_parent_height);

            let chip_size = tester.get_size(only::<Chip>(tester));
            assert_eq!(chip_size.width(), chip_parent_width);
            assert_eq!(chip_size.height(), chip_parent_height);
        });
    }

    #[test]
    fn a_chip_constrains_the_size_of_the_label_widget_when_it_exceeds_the_available_space() {
        test_constrained_label(false, false);
    }

    #[test]
    fn a_chip_constrains_the_label_when_it_exceeds_the_available_space_with_an_avatar() {
        test_constrained_label(true, false);
    }

    #[test]
    fn a_chip_constrains_the_label_when_it_exceeds_the_available_space_with_a_delete_icon() {
        test_constrained_label(false, true);
    }

    #[test]
    fn a_chip_constrains_the_label_when_it_exceeds_the_available_space_with_both_icons() {
        test_constrained_label(true, true);
    }

    #[test]
    fn a_chip_does_not_constrain_the_size_of_the_label_widget_if_it_does_not_exceed_the_space() {
        test_widgets(|tester| {
            let label_key = GlobalKey::new();
            let root = wrap_ltr(
                tester,
                Center::new().child(
                    SizedBox::new().width(500.0).height(500.0).child(
                        Column::new().children([Chip::new(
                            SizedBox::new()
                                .key(label_key.clone().into())
                                .width(50.0)
                                .height(30.0),
                        )
                        .into_widget()]),
                    ),
                ),
            );
            tester.pump_widget(root);

            let label_size = tester.get_size(by_key(tester, &label_key));
            assert_eq!(label_size.width(), 50.0);
            assert_eq!(label_size.height(), 30.0);
        });
    }

    #[test]
    fn material2_chip_in_row_works_ok() {
        test_widgets(|tester| {
            let style = TextStyle::new().font_size(10.0);
            let theme = ThemeData::builder()
                .use_material3(false)
                .build(&tester.cell.borrow());
            let chip = || Chip::new(Text::new("Test")).label_style(style.clone());
            let root = wrap_with_theme(tester, theme, Row::new().children([chip().into_widget()]));
            tester.pump_widget(root);
            assert_eq!(tester.get_size(only::<Text>(tester)), Size::new(40.0, 10.0));
            assert_eq!(tester.get_size(only::<Chip>(tester)), Size::new(64.0, 48.0));
            let root = wrap_ltr(
                tester,
                Row::new().children([Flexible::new(chip()).into_widget()]),
            );
            tester.pump_widget(root);
            assert_eq!(tester.get_size(only::<Text>(tester)), Size::new(40.0, 10.0));
            assert_eq!(tester.get_size(only::<Chip>(tester)), Size::new(64.0, 48.0));
            let root = wrap_ltr(
                tester,
                Row::new().children([Expanded::new(chip()).into_widget()]),
            );
            tester.pump_widget(root);
            assert_eq!(tester.get_size(only::<Text>(tester)), Size::new(40.0, 10.0));
            assert_eq!(
                tester.get_size(only::<Chip>(tester)),
                Size::new(800.0, 48.0)
            );
        });
    }

    #[test]
    fn material2_chip_responds_to_material_tap_target_size() {
        test_widgets(|tester| {
            let theme = ThemeData::builder()
                .use_material3(false)
                .build(&tester.cell.borrow());
            let root = wrap_with_theme(
                tester,
                theme,
                Column::new().children([
                    Chip::new(Text::new("X"))
                        .material_tap_target_size(MaterialTapTargetSize::Padded)
                        .into_widget(),
                    Chip::new(Text::new("X"))
                        .material_tap_target_size(MaterialTapTargetSize::ShrinkWrap)
                        .into_widget(),
                ]),
            );
            tester.pump_widget(root);
            let chips = find::by_type::<Chip>().evaluate(tester);
            assert_eq!(tester.get_size(chips[0]), Size::new(48.0, 48.0));
            assert_eq!(tester.get_size(chips[1]), Size::new(38.0, 32.0));
        });
    }

    #[test]
    fn material3_chip_responds_to_material_tap_target_size() {
        test_widgets(|tester| {
            let root = wrap_ltr(
                tester,
                Column::new().children([
                    Chip::new(Text::new("X"))
                        .material_tap_target_size(MaterialTapTargetSize::Padded)
                        .into_widget(),
                    Chip::new(Text::new("X"))
                        .material_tap_target_size(MaterialTapTargetSize::ShrinkWrap)
                        .into_widget(),
                ]),
            );
            tester.pump_widget(root);
            let chips = find::by_type::<Chip>().evaluate(tester);
            let (first, last) = (tester.get_size(chips[0]), tester.get_size(chips[1]));
            assert!((first.width() - 48.1).abs() < 0.01);
            assert_eq!(first.height(), 48.0);
            assert!((last.width() - 48.1).abs() < 0.01);
            assert_eq!(last.height(), 38.0);
        });
    }

    #[test]
    fn m3_chip_defaults() {
        test_widgets(|tester| {
            let light_theme = m3(&tester.cell.borrow());
            let text_theme: Rc<RefCell<Option<crate::text_theme::TextTheme>>> = Rc::default();
            let root = |theme: ThemeData| {
                let text_theme = Rc::clone(&text_theme);
                crate::test_support::material_app(
                    theme,
                    Scaffold::new()
                        .body(Center::new().child(waft_widgets::Builder::new(
                            move |app, context| {
                                *text_theme.borrow_mut() =
                                    Some(Theme::of(app, context).text_theme.clone());
                                Chip::new(Text::new("Chip A"))
                                    .avatar(CircleAvatar::new().child(Text::new("A")))
                                    .on_deleted(Listener::new(|_app| {}))
                                    .into_widget()
                            },
                        )))
                        .into_widget(),
                )
            };
            tester.pump_widget(root(light_theme.clone()));
            let material = material_of_raw_chip(tester);
            let (color, elevation, shape) = tester.widget(material, |material: &Material| {
                (material.color, material.elevation, material.shape.clone())
            });
            assert_eq!(color, None);
            assert_eq!(elevation, 0.0);
            assert_eq!(
                shape,
                Some(ShapeBorderRef::new(RoundedRectangleBorder::new(
                    BorderSide {
                        color: light_theme.color_scheme.outline_variant(),
                        ..BorderSide::default()
                    },
                    BorderRadiusGeometry::circular(8.0),
                )))
            );
            let icon_data = icon_data(tester);
            assert_eq!(
                icon_data.color.map(|color| color.color()),
                Some(light_theme.color_scheme.primary)
            );
            assert_eq!(icon_data.size, Some(18.0));
            let label_style = label_style(tester, "Chip A");
            assert_eq!(
                label_style.color.map(|color| color.color()),
                Some(light_theme.color_scheme.on_surface_variant())
            );
            let label_large = text_theme.borrow().clone().unwrap().label_large.unwrap();
            assert_eq!(label_style.font_size, label_large.font_size);
            assert_eq!(label_style.font_weight, label_large.font_weight);
            assert_eq!(label_style.letter_spacing, label_large.letter_spacing);

            let dark_theme = ThemeData::dark(&tester.cell.borrow(), None);
            tester.pump_widget(root(dark_theme.clone()));
            tester.pump_and_settle(); // Theme transition animation
            let material = material_of_raw_chip(tester);
            let (color, elevation) = tester.widget(material, |material: &Material| {
                (material.color, material.elevation)
            });
            assert_eq!(color, None);
            assert_eq!(elevation, 0.0);
            assert_eq!(
                icon_data_color(tester),
                Some(dark_theme.color_scheme.primary)
            );
            assert_eq!(
                label_style_color(tester, "Chip A"),
                Some(dark_theme.color_scheme.on_surface_variant())
            );
        });
    }

    pub(crate) fn icon_data_color(tester: &WidgetTester) -> Option<Color> {
        icon_data(tester).color.map(|color| color.color())
    }

    /// The color of the `ShapeDecoration` of the chip's only `Ink`, Dart's
    /// `(tester.widget<Ink>(find.byType(Ink)).decoration! as ShapeDecoration).color`.
    pub(crate) fn ink_decoration_color(tester: &WidgetTester) -> Option<Color> {
        let ink = only::<crate::ink_decoration::Ink>(tester);
        tester.widget(ink, |ink: &crate::ink_decoration::Ink| {
            ink.decoration
                .as_deref()
                .and_then(|decoration| decoration.as_any().downcast_ref::<ShapeDecoration>())
                .expect("a ShapeDecoration")
                .color
                .as_ref()
                .map(|color| color.color())
        })
    }

    /// What the chip's `Material` says that the defaults tests read.
    pub(crate) fn chip_material_of(
        tester: &WidgetTester,
        chip: AnyElement,
    ) -> (f64, Option<Color>, Option<Color>, Option<ShapeBorderRef>) {
        let material = tester.descendants_of_type::<Material>(chip)[0];
        tester.widget(material, |material: &Material| {
            (
                material.elevation,
                material.shadow_color,
                material.surface_tint_color,
                material.shape.clone(),
            )
        })
    }

    pub(crate) fn label_style_color(tester: &WidgetTester, label_text: &str) -> Option<Color> {
        label_style(tester, label_text)
            .color
            .map(|color| color.color())
    }

    #[test]
    fn a_chip_deletes_when_its_delete_button_is_tapped() {
        test_widgets(|tester| {
            let deleted: Rc<RefCell<Vec<&'static str>>> = Rc::default();
            let chip = |label: &'static str, avatar: &'static str, tooltip: &'static str| {
                let deleted = Rc::clone(&deleted);
                Chip::new(Text::new(label))
                    .avatar(CircleAvatar::new().child(Text::new(avatar)))
                    .on_deleted(Listener::new(move |_app| deleted.borrow_mut().push(avatar)))
                    .delete_button_tooltip_message(tooltip.to_owned())
                    .into_widget()
            };
            let root = wrap_ltr(
                tester,
                Column::new().children([
                    chip("Chip A", "A", "Delete chip A"),
                    chip("Chip B", "B", "Delete chip B"),
                ]),
            );
            tester.pump_widget(root);

            let delete_a = tester.get_center(by_tooltip(tester, "Delete chip A"));
            let delete_b = tester.get_center(by_tooltip(tester, "Delete chip B"));
            assert!(deleted.borrow().is_empty());
            tester.tap_at(delete_a, 1);
            assert_eq!(*deleted.borrow(), vec!["A"]);
            tester.pump_and_settle();
            tester.tap_at(delete_b, 1);
            assert_eq!(*deleted.borrow(), vec!["A", "B"]);
        });
    }

    #[test]
    fn the_delete_button_tap_target_is_the_right_proportion_of_the_chip() {
        test_widgets(|tester| {
            let delete_key = GlobalKey::new();
            let called_delete = Rc::new(Cell::new(false));
            let build_chip = |tester: &WidgetTester, label: WidgetRef, icon: IconData| {
                let called_delete = Rc::clone(&called_delete);
                wrap_ltr(
                    tester,
                    Column::new().children([Chip::new(label)
                        .delete_icon(
                            waft_widgets::Icon::new(Some(icon)).key(delete_key.clone().into()),
                        )
                        .on_deleted(Listener::new(move |_app| called_delete.set(true)))
                        .into_widget()]),
                )
            };
            let root = build_chip(
                tester,
                Text::new("Really Long Label").into_widget(),
                Icons::delete(),
            );
            tester.pump_widget(root);

            // Test correct tap target size.
            let center = tester.get_center(by_key(tester, &delete_key));
            // Half the width of the delete button + right label padding.
            tester.tap_at(center - Offset::new(18.0, 0.0), 1);
            tester.pump();
            assert!(called_delete.get());
            called_delete.set(false);

            // Test incorrect tap target size.
            tester.tap_at(center - Offset::new(19.0, 0.0), 1);
            tester.pump();
            assert!(!called_delete.get());
            called_delete.set(false);

            let root = build_chip(tester, SizedBox::new().into_widget(), Icons::cancel());
            tester.pump_widget(root);

            // Chip width is 48 with padding, 40 without padding, so halfway is at 20. Cancel
            // icon is 24x24, so since 24 > 20 the split location should be halfway across the
            // chip, which is at 12 + 8 = 20 from the right side. Since the split is just
            // slightly less than 50%, 8 from the center of the delete button should hit the
            // chip, not the delete button.
            let center = tester.get_center(by_key(tester, &delete_key));
            tester.tap_at(center - Offset::new(7.0, 0.0), 1);
            tester.pump();
            assert!(called_delete.get());
            called_delete.set(false);

            tester.tap_at(center - Offset::new(8.0, 0.0), 1);
            tester.pump();
            assert!(!called_delete.get());
        });
    }

    #[test]
    fn chip_elements_are_ordered_horizontally_for_locale() {
        test_widgets(|tester| {
            let icon_key = GlobalKey::new();
            let test = |tester: &WidgetTester, direction: TextDirection| {
                wrap_for_chip(
                    tester,
                    Chip::new(Text::new("ABC"))
                        .delete_icon(
                            waft_widgets::Icon::new(Some(Icons::delete()))
                                .key(icon_key.clone().into()),
                        )
                        .on_deleted(Listener::new(|_app| {}))
                        .into_widget(),
                    direction,
                    TextScaler::NO_SCALING,
                    None,
                )
            };
            let root = test(tester, TextDirection::Rtl);
            tester.pump_widget(root);
            tester.pump_and_settle();
            assert!(
                tester.get_center(text(tester, "ABC")).dx()
                    > tester.get_center(by_key(tester, &icon_key)).dx()
            );
            let root = test(tester, TextDirection::Ltr);
            tester.pump_widget(root);
            tester.pump_and_settle();
            assert!(
                tester.get_center(text(tester, "ABC")).dx()
                    < tester.get_center(by_key(tester, &icon_key)).dx()
            );
        });
    }

    pub(crate) fn top_left_of(tester: &WidgetTester, element: AnyElement) -> Offset {
        tester.get_rect(element).top_left()
    }

    pub(crate) fn pump_ms(tester: &mut WidgetTester, ms: u64) {
        tester.pump_by(Duration::from_millis(ms));
    }

    #[test]
    fn material3_avatar_drawer_works_as_expected_on_raw_chip() {
        test_widgets(|tester| {
            let label_key = GlobalKey::new();
            let avatar_key = GlobalKey::new();
            let push_chip = |tester: &mut WidgetTester, with_avatar: bool| {
                let mut chip = RawChip::new(Text::new("Chip").key(label_key.clone().into()))
                    .shape(StadiumBorder::new(BorderSide::NONE));
                if with_avatar {
                    chip = chip.avatar(
                        waft_widgets::Container::new()
                            .key(avatar_key.clone().into())
                            .color(Color::new(0xff000000))
                            .width(40.0)
                            .height(40.0),
                    );
                }
                let root = wrap_ltr(tester, Wrap::new().children([chip.into_widget()]));
                tester.pump_widget(root);
            };
            let width = |tester: &WidgetTester| tester.get_size(only::<RawChip>(tester)).width();
            let height = |tester: &WidgetTester| tester.get_size(only::<RawChip>(tester)).height();
            let avatar = |tester: &WidgetTester| by_key(tester, &avatar_key);
            let label = |tester: &WidgetTester| by_key(tester, &label_key);

            // No avatar
            push_chip(tester, false);
            assert_more_or_less_equals_within(width(tester), 90.4, 0.1);

            // Add an avatar
            push_chip(tester, true);
            // Avatar drawer should start out closed.
            assert_more_or_less_equals_within(width(tester), 90.4, 0.1);
            assert_eq!(height(tester), 48.0);
            assert_eq!(tester.get_size(avatar(tester)), Size::new(20.0, 20.0));
            assert_eq!(
                top_left_of(tester, avatar(tester)),
                Offset::new(-11.0, 14.0)
            );
            assert_eq!(top_left_of(tester, label(tester)), Offset::new(17.0, 14.0));

            for (chip_width, avatar_dx, label_dx) in [
                (91.3, -10.0, 17.9),
                (95.9, -5.4, 22.5),
                (102.6, 1.2, 29.2),
                (106.6, 5.2, 33.2),
            ] {
                pump_ms(tester, 20);
                assert_more_or_less_equals_within(width(tester), chip_width, 0.1);
                assert_eq!(height(tester), 48.0);
                assert_eq!(tester.get_size(avatar(tester)), Size::new(20.0, 20.0));
                assert_more_or_less_equals_within(
                    top_left_of(tester, avatar(tester)).dx(),
                    avatar_dx,
                    0.1,
                );
                assert_more_or_less_equals_within(
                    top_left_of(tester, label(tester)).dx(),
                    label_dx,
                    0.1,
                );
            }

            // Wait for being done with animation, and make sure it didn't change height.
            tester.pump_and_settle_by(Duration::from_millis(200));
            assert_more_or_less_equals_within(width(tester), 110.4, 0.1);
            assert_eq!(height(tester), 48.0);
            assert_eq!(tester.get_size(avatar(tester)), Size::new(20.0, 20.0));
            assert_eq!(top_left_of(tester, avatar(tester)), Offset::new(9.0, 14.0));
            assert_eq!(top_left_of(tester, label(tester)), Offset::new(37.0, 14.0));

            // Remove the avatar again
            push_chip(tester, false);
            // Avatar drawer should start out open.
            assert_more_or_less_equals_within(width(tester), 110.4, 0.1);
            assert_eq!(height(tester), 48.0);
            assert_eq!(top_left_of(tester, avatar(tester)), Offset::new(9.0, 14.0));
            assert_eq!(top_left_of(tester, label(tester)), Offset::new(37.0, 14.0));

            for (chip_width, avatar_dx, label_dx) in [
                (109.5, 8.1, 36.1),
                (105.4, 4.0, 32.0),
                (93.7, -7.6, 20.3),
                (90.4, -11.0, 17.0),
            ] {
                pump_ms(tester, 20);
                assert_more_or_less_equals_within(width(tester), chip_width, 0.1);
                assert_eq!(height(tester), 48.0);
                assert_eq!(tester.get_size(avatar(tester)), Size::new(20.0, 20.0));
                assert_more_or_less_equals_within(
                    top_left_of(tester, avatar(tester)).dx(),
                    avatar_dx,
                    0.1,
                );
                assert_more_or_less_equals_within(
                    top_left_of(tester, label(tester)).dx(),
                    label_dx,
                    0.1,
                );
            }

            // Wait for being done with animation, make sure it didn't change height, and make
            // sure that the avatar is no longer drawn.
            tester.pump_and_settle_by(Duration::from_millis(200));
            assert_more_or_less_equals_within(width(tester), 90.4, 0.1);
            assert_eq!(height(tester), 48.0);
            assert_eq!(top_left_of(tester, label(tester)), Offset::new(17.0, 14.0));
            let avatar_key_ref: KeyRef = avatar_key.clone().into();
            assert!(tester.find_onstage_by_key(&avatar_key_ref).is_empty());
        });
    }

    #[test]
    fn material3_delete_button_drawer_works_as_expected_on_raw_chip() {
        test_widgets(|tester| {
            let label_key = GlobalKey::new();
            let delete_key = GlobalKey::new();
            let was_deleted = Rc::new(Cell::new(false));
            let push_chip = |tester: &mut WidgetTester, deletable: bool| {
                let mut chip = RawChip::new(Text::new("Chip").key(label_key.clone().into()))
                    .delete_icon(
                        waft_widgets::Container::new()
                            .key(delete_key.clone().into())
                            .color(Colors::BLUE.primary())
                            .width(40.0)
                            .height(40.0),
                    )
                    .shape(StadiumBorder::new(BorderSide::NONE));
                if deletable {
                    let was_deleted = Rc::clone(&was_deleted);
                    chip = chip.on_deleted(Listener::new(move |_app| was_deleted.set(true)));
                }
                let root = wrap_ltr(tester, Wrap::new().children([chip.into_widget()]));
                tester.pump_widget(root);
            };
            let width = |tester: &WidgetTester| tester.get_size(only::<RawChip>(tester)).width();
            let height = |tester: &WidgetTester| tester.get_size(only::<RawChip>(tester)).height();
            let delete = |tester: &WidgetTester| by_key(tester, &delete_key);
            let label = |tester: &WidgetTester| by_key(tester, &label_key);

            // No delete button
            push_chip(tester, false);
            assert_more_or_less_equals_within(width(tester), 90.4, 0.01);
            assert_eq!(height(tester), 48.0);

            // Add a delete button
            push_chip(tester, true);
            // Delete button drawer should start out closed.
            assert_more_or_less_equals_within(width(tester), 90.4, 0.01);
            assert_eq!(height(tester), 48.0);
            assert_eq!(tester.get_size(delete(tester)), Size::new(20.0, 20.0));
            let top_left = top_left_of(tester, delete(tester));
            assert_more_or_less_equals_within(top_left.dx(), 61.4, 0.01);
            assert_more_or_less_equals_within(top_left.dy(), 14.0, 0.01);
            assert_eq!(top_left_of(tester, label(tester)), Offset::new(17.0, 14.0));

            for (chip_width, delete_dx) in
                [(91.3, 62.3), (95.9, 66.9), (102.6, 73.6), (106.6, 77.6)]
            {
                pump_ms(tester, 20);
                assert_more_or_less_equals_within(width(tester), chip_width, 0.1);
                assert_eq!(tester.get_size(delete(tester)), Size::new(20.0, 20.0));
                assert_more_or_less_equals_within(
                    top_left_of(tester, delete(tester)).dx(),
                    delete_dx,
                    0.1,
                );
            }

            // Wait for being done with animation, and make sure it didn't change height.
            tester.pump_and_settle_by(Duration::from_millis(200));
            assert_more_or_less_equals_within(width(tester), 110.4, 0.1);
            assert_eq!(height(tester), 48.0);
            assert_eq!(tester.get_size(delete(tester)), Size::new(20.0, 20.0));
            let top_left = top_left_of(tester, delete(tester));
            assert_more_or_less_equals_within(top_left.dx(), 81.4, 0.01);
            assert_more_or_less_equals_within(top_left.dy(), 14.0, 0.01);
            assert_eq!(top_left_of(tester, label(tester)), Offset::new(17.0, 14.0));

            // Test the tap work for the delete button, but not the rest of the chip.
            assert!(!was_deleted.get());
            tester.tap_at(tester.get_center(label(tester)), 1);
            assert!(!was_deleted.get());
            tester.tap_at(tester.get_center(delete(tester)), 1);
            assert!(was_deleted.get());

            // Remove the delete button again
            push_chip(tester, false);
            // Delete button drawer should start out open.
            assert_more_or_less_equals_within(width(tester), 110.4, 0.1);
            assert_eq!(height(tester), 48.0);
            let top_left = top_left_of(tester, delete(tester));
            assert_more_or_less_equals_within(top_left.dx(), 81.4, 0.01);
            assert_eq!(top_left_of(tester, label(tester)), Offset::new(17.0, 14.0));

            for (chip_width, delete_dx) in
                [(110.1, 81.1), (109.4, 80.4), (107.9, 78.9), (104.9, 75.9)]
            {
                pump_ms(tester, 20);
                assert_more_or_less_equals_within(width(tester), chip_width, 0.1);
                assert_eq!(tester.get_size(delete(tester)), Size::new(20.0, 20.0));
                assert_more_or_less_equals_within(
                    top_left_of(tester, delete(tester)).dx(),
                    delete_dx,
                    0.1,
                );
            }

            // Wait for being done with animation, make sure it didn't change height, and make
            // sure that the delete button is no longer drawn.
            tester.pump_and_settle_by(Duration::from_millis(200));
            assert_more_or_less_equals_within(width(tester), 90.4, 0.1);
            assert_eq!(height(tester), 48.0);
            assert_eq!(top_left_of(tester, label(tester)), Offset::new(17.0, 14.0));
            let delete_key_ref: KeyRef = delete_key.clone().into();
            assert!(tester.find_onstage_by_key(&delete_key_ref).is_empty());
        });
    }

    #[test]
    fn the_delete_button_takes_up_at_most_half_of_the_chip() {
        test_widgets(|tester| {
            let chip_key = GlobalKey::new();
            let chip_pressed = Rc::new(Cell::new(false));
            let delete_pressed = Rc::new(Cell::new(false));
            let root = wrap_ltr(
                tester,
                Wrap::new().children([RawChip::new(Text::new(""))
                    .key(chip_key.clone().into())
                    .on_pressed({
                        let chip_pressed = Rc::clone(&chip_pressed);
                        Listener::new(move |_app| chip_pressed.set(true))
                    })
                    .on_deleted({
                        let delete_pressed = Rc::clone(&delete_pressed);
                        Listener::new(move |_app| delete_pressed.set(true))
                    })
                    .into_widget()]),
            );
            tester.pump_widget(root);

            let center = tester.get_center(by_key(tester, &chip_key));
            tester.tap_at(center, 1);
            tester.pump();
            assert!(chip_pressed.get());
            assert!(!delete_pressed.get());
            chip_pressed.set(false);

            tester.tap_at(center + Offset::new(1.0, 0.0), 1);
            tester.pump();
            assert!(!chip_pressed.get());
            assert!(delete_pressed.get());
        });
    }

    mod layout_and_padding {
        //! chip_test.dart: control, rows, text scale, non-text labels, padding and the
        //! Material 2 drawers.
        //!
        //! Dart's `FeedbackTester` clicks (`feedback.clickSoundCount`) are not asserted in the control
        //! test: `SystemSound` is not ported (see `PORTING.md`).
        use waft_widgets::testing::{
            WidgetTester, assert_more_or_less_equals_within, find, test_widgets,
        };

        use std::cell::{Cell, RefCell};
        use std::rc::Rc;

        use waft_embedder::{Color, Offset, Size};
        use waft_painting::{BorderSide, StadiumBorder, TextScaler, TextStyle};
        use waft_widgets::{
            Column, Container, Expanded, Flexible, GlobalKey, Icon, Placeholder, Row, SizedBox,
            Text, TextDirection, Wrap,
        };

        use super::{
            by_key, by_tooltip, only, pump_ms, text, top_left_of, wrap_for_chip, wrap_ltr,
            wrap_with_theme,
        };
        use crate::chip::{Chip, RawChip};
        use crate::circle_avatar::CircleAvatar;
        use crate::colors::Colors;
        use crate::theme_data::ThemeData;
        use crate::tooltip::tests::overlay;
        use waft_foundation::Listener;
        use waft_widgets::{IntoWidget, WidgetRef};

        fn m2_theme(tester: &WidgetTester) -> ThemeData {
            ThemeData::builder()
                .use_material3(false)
                .build(&tester.cell.borrow())
        }

        fn chip_avatar(letter: &'static str) -> CircleAvatar {
            CircleAvatar::new().child(Text::new(letter))
        }

        fn sizes_of(tester: &WidgetTester, elements: &[waft_widgets::AnyElement]) -> Vec<Size> {
            elements.iter().map(|e| tester.get_size(*e)).collect()
        }

        #[test]
        fn chip_control_test() {
            test_widgets(|tester| {
                let deleted_chip_labels: Rc<RefCell<Vec<&'static str>>> = Rc::default();
                let chip = |label: &'static str, avatar: &'static str, tooltip: &'static str| {
                    let deleted = Rc::clone(&deleted_chip_labels);
                    Chip::new(Text::new(label))
                        .avatar(chip_avatar(avatar))
                        .on_deleted(Listener::new(move |_app| deleted.borrow_mut().push(avatar)))
                        .delete_button_tooltip_message(tooltip.to_owned())
                        .into_widget()
                };
                let root = wrap_ltr(
                    tester,
                    Column::new().children([
                        chip("Chip A", "A", "Delete chip A"),
                        chip("Chip B", "B", "Delete chip B"),
                    ]),
                );
                tester.pump_widget(root);

                let delete_a = by_tooltip(tester, "Delete chip A");
                let delete_b = by_tooltip(tester, "Delete chip B");

                assert!(deleted_chip_labels.borrow().is_empty());
                tester.tap_at(tester.get_center(delete_a), 1);
                assert_eq!(*deleted_chip_labels.borrow(), vec!["A"]);

                tester.pump_and_settle_by(std::time::Duration::from_secs(1));

                tester.tap_at(tester.get_center(delete_b), 1);
                assert_eq!(*deleted_chip_labels.borrow(), vec!["A", "B"]);

                tester.pump_and_settle_by(std::time::Duration::from_secs(1));
            });
        }

        #[test]
        fn material3_chip_in_row_works_ok() {
            test_widgets(|tester| {
                let style = TextStyle::new().font_size(10.0);
                let chip = || Chip::new(Text::new("Test")).label_style(style.clone());

                let root = wrap_ltr(tester, Row::new().children([chip().into_widget()]));
                tester.pump_widget(root);
                let text_size = tester.get_size(only::<Text>(tester));
                assert_more_or_less_equals_within(text_size.width(), 40.4, 0.01);
                assert_eq!(text_size.height(), 14.0);
                let chip_size = tester.get_size(only::<Chip>(tester));
                assert_more_or_less_equals_within(chip_size.width(), 74.4, 0.01);
                assert_eq!(chip_size.height(), 48.0);

                let root = wrap_ltr(
                    tester,
                    Row::new().children([Flexible::new(chip()).into_widget()]),
                );
                tester.pump_widget(root);
                let text_size = tester.get_size(only::<Text>(tester));
                assert_more_or_less_equals_within(text_size.width(), 40.4, 0.01);
                assert_eq!(text_size.height(), 14.0);
                let chip_size = tester.get_size(only::<Chip>(tester));
                assert_more_or_less_equals_within(chip_size.width(), 74.4, 0.01);
                assert_eq!(chip_size.height(), 48.0);

                let root = wrap_ltr(
                    tester,
                    Row::new().children([Expanded::new(chip()).into_widget()]),
                );
                tester.pump_widget(root);
                let text_size = tester.get_size(only::<Text>(tester));
                assert_more_or_less_equals_within(text_size.width(), 40.4, 0.01);
                assert_eq!(text_size.height(), 14.0);
                assert_eq!(
                    tester.get_size(only::<Chip>(tester)),
                    Size::new(800.0, 48.0)
                );
            });
        }

        fn two_avatar_chips(scaled_label_a: bool) -> WidgetRef {
            let label_a = if scaled_label_a {
                Text::new("Chip A").text_scaler(TextScaler::linear(3.0))
            } else {
                Text::new("Chip A")
            };
            Column::new()
                .children([
                    Chip::new(label_a).avatar(chip_avatar("A")).into_widget(),
                    Chip::new(Text::new("Chip B"))
                        .avatar(chip_avatar("B"))
                        .into_widget(),
                ])
                .into_widget()
        }

        #[test]
        fn material2_chip_responds_to_text_scale_factor() {
            test_widgets(|tester| {
                let theme = m2_theme(tester);
                let root = wrap_with_theme(tester, theme, two_avatar_chips(false));
                tester.pump_widget(root);

                let size_of_text =
                    |tester: &WidgetTester, label: &str| tester.get_size(text(tester, label));
                assert_eq!(size_of_text(tester, "Chip A"), Size::new(84.0, 14.0));
                assert_eq!(size_of_text(tester, "Chip B"), Size::new(84.0, 14.0));
                let chips = find::by_type::<Chip>().evaluate(tester);
                assert_eq!(
                    sizes_of(tester, &[chips[0], chips[1]]),
                    vec![Size::new(132.0, 48.0); 2]
                );

                let root = wrap_for_chip(
                    tester,
                    two_avatar_chips(false),
                    TextDirection::Ltr,
                    TextScaler::linear(3.0),
                    None,
                );
                tester.pump_widget(root);

                assert_eq!(size_of_text(tester, "Chip A"), Size::new(252.0, 42.0));
                assert_eq!(size_of_text(tester, "Chip B"), Size::new(252.0, 42.0));
                let chips = find::by_type::<Chip>().evaluate(tester);
                assert_eq!(
                    sizes_of(tester, &[chips[0], chips[1]]),
                    vec![Size::new(310.0, 50.0); 2]
                );

                // Check that individual text scales are taken into account.
                let root = wrap_ltr(tester, two_avatar_chips(true));
                tester.pump_widget(root);

                assert_eq!(size_of_text(tester, "Chip A"), Size::new(252.0, 42.0));
                assert_eq!(size_of_text(tester, "Chip B"), Size::new(84.0, 14.0));
                let chips = find::by_type::<Chip>().evaluate(tester);
                assert_eq!(tester.get_size(chips[0]), Size::new(318.0, 50.0));
                assert_eq!(tester.get_size(chips[1]), Size::new(132.0, 48.0));
            });
        }

        #[test]
        fn material3_chip_responds_to_text_scale_factor() {
            test_widgets(|tester| {
                let root = wrap_ltr(tester, two_avatar_chips(false));
                tester.pump_widget(root);

                let size_of_text =
                    |tester: &WidgetTester, label: &str| tester.get_size(text(tester, label));
                assert_more_or_less_equals_within(
                    size_of_text(tester, "Chip A").width(),
                    84.5,
                    0.1,
                );
                assert_eq!(size_of_text(tester, "Chip A").height(), 20.0);
                assert_more_or_less_equals_within(
                    size_of_text(tester, "Chip B").width(),
                    84.5,
                    0.1,
                );
                assert_eq!(size_of_text(tester, "Chip B").height(), 20.0);

                let root = wrap_for_chip(
                    tester,
                    two_avatar_chips(false),
                    TextDirection::Ltr,
                    TextScaler::linear(3.0),
                    None,
                );
                tester.pump_widget(root);

                assert_more_or_less_equals_within(
                    size_of_text(tester, "Chip A").width(),
                    252.6,
                    0.1,
                );
                assert_eq!(size_of_text(tester, "Chip A").height(), 60.0);
                assert_more_or_less_equals_within(
                    size_of_text(tester, "Chip B").width(),
                    252.6,
                    0.1,
                );
                assert_eq!(size_of_text(tester, "Chip B").height(), 60.0);
                let chips = find::by_type::<Chip>().evaluate(tester);
                let (first, last) = (tester.get_size(chips[0]), tester.get_size(chips[1]));
                assert_more_or_less_equals_within(first.width(), 338.6, 0.1);
                assert_eq!(first.height(), 78.0);
                assert_more_or_less_equals_within(last.width(), 338.6, 0.1);
                assert_eq!(last.height(), 78.0);

                // Check that individual text scales are taken into account.
                let root = wrap_ltr(tester, two_avatar_chips(true));
                tester.pump_widget(root);

                assert_more_or_less_equals_within(
                    size_of_text(tester, "Chip A").width(),
                    252.6,
                    0.01,
                );
                assert_eq!(size_of_text(tester, "Chip A").height(), 60.0);
                assert_more_or_less_equals_within(
                    size_of_text(tester, "Chip B").width(),
                    84.59,
                    0.01,
                );
                assert_eq!(size_of_text(tester, "Chip B").height(), 20.0);
                let chips = find::by_type::<Chip>().evaluate(tester);
                let (first, last) = (tester.get_size(chips[0]), tester.get_size(chips[1]));
                assert_more_or_less_equals_within(first.width(), 346.6, 0.01);
                assert_eq!(first.height(), 78.0);
                assert_more_or_less_equals_within(last.width(), 138.59, 0.01);
                assert_eq!(last.height(), 48.0);
            });
        }

        fn labels_with_non_text_widget(key_a: &GlobalKey, key_b: &GlobalKey) -> WidgetRef {
            Column::new()
                .children([
                    Chip::new(Text::new("Chip A").key(key_a.clone().into()))
                        .avatar(chip_avatar("A"))
                        .into_widget(),
                    Chip::new(
                        SizedBox::new()
                            .key(key_b.clone().into())
                            .width(10.0)
                            .height(10.0),
                    )
                    .avatar(chip_avatar("B"))
                    .into_widget(),
                ])
                .into_widget()
        }

        #[test]
        fn material2_labels_can_be_non_text_widgets() {
            test_widgets(|tester| {
                let (key_a, key_b) = (GlobalKey::new(), GlobalKey::new());
                let theme = m2_theme(tester);
                let root =
                    wrap_with_theme(tester, theme, labels_with_non_text_widget(&key_a, &key_b));
                tester.pump_widget(root);

                assert_eq!(
                    tester.get_size(by_key(tester, &key_a)),
                    Size::new(84.0, 14.0)
                );
                assert_eq!(
                    tester.get_size(by_key(tester, &key_b)),
                    Size::new(10.0, 10.0)
                );
                let chips = find::by_type::<Chip>().evaluate(tester);
                assert_eq!(tester.get_size(chips[0]), Size::new(132.0, 48.0));
                assert_eq!(tester.get_size(chips[1]), Size::new(58.0, 48.0));
            });
        }

        #[test]
        fn material3_labels_can_be_non_text_widgets() {
            test_widgets(|tester| {
                let (key_a, key_b) = (GlobalKey::new(), GlobalKey::new());
                let root = wrap_ltr(tester, labels_with_non_text_widget(&key_a, &key_b));
                tester.pump_widget(root);

                let label_a = tester.get_size(by_key(tester, &key_a));
                assert_more_or_less_equals_within(label_a.width(), 84.5, 0.1);
                assert_eq!(label_a.height(), 20.0);
                assert_eq!(
                    tester.get_size(by_key(tester, &key_b)),
                    Size::new(10.0, 10.0)
                );
                let chips = find::by_type::<Chip>().evaluate(tester);
                let first = tester.get_size(chips[0]);
                assert_more_or_less_equals_within(first.width(), 138.5, 0.1);
                assert_eq!(first.height(), 48.0);
                assert_eq!(tester.get_size(chips[1]), Size::new(60.0, 48.0));
            });
        }

        #[test]
        fn avatars_can_be_non_circle_avatar_widgets() {
            test_widgets(|tester| {
                let key_a = GlobalKey::new();
                let root = wrap_ltr(
                    tester,
                    Column::new().children([Chip::new(Text::new("Chip A"))
                        .avatar(
                            SizedBox::new()
                                .key(key_a.clone().into())
                                .width(20.0)
                                .height(20.0),
                        )
                        .into_widget()]),
                );
                tester.pump_widget(root);

                assert_eq!(
                    tester.get_size(by_key(tester, &key_a)),
                    Size::new(20.0, 20.0)
                );
            });
        }

        #[test]
        fn delete_icons_can_be_non_icon_widgets() {
            test_widgets(|tester| {
                let key_a = GlobalKey::new();
                let root = wrap_ltr(
                    tester,
                    Column::new().children([Chip::new(Text::new("Chip A"))
                        .delete_icon(
                            SizedBox::new()
                                .key(key_a.clone().into())
                                .width(20.0)
                                .height(20.0),
                        )
                        .on_deleted(Listener::new(|_app| {}))
                        .into_widget()]),
                );
                tester.pump_widget(root);

                assert_eq!(
                    tester.get_size(by_key(tester, &key_a)),
                    Size::new(20.0, 20.0)
                );
            });
        }

        fn pump_padding_chip(
            tester: &mut WidgetTester,
            text_direction: TextDirection,
            key_a: &GlobalKey,
            key_b: &GlobalKey,
        ) {
            let chip = Chip::new(
                SizedBox::new()
                    .key(key_b.clone().into())
                    .width(40.0)
                    .height(40.0),
            )
            .avatar(Placeholder::new().key(key_a.clone().into()))
            .on_deleted(Listener::new(|_app| {}));
            let entry_child = crate::material::Material::new()
                .child(waft_widgets::Center::new().child(chip))
                .into_widget();
            let overlay = overlay(&mut tester.cell.borrow_mut(), entry_child);
            let root = wrap_for_chip(
                tester,
                overlay,
                text_direction,
                TextScaler::NO_SCALING,
                None,
            );
            tester.pump_widget(root);
        }

        fn bottom_right_of(tester: &WidgetTester, element: waft_widgets::AnyElement) -> Offset {
            let rect = super::tests::rect_of(tester, element);
            Offset::new(rect.right, rect.bottom)
        }

        #[test]
        fn chip_padding_ltr() {
            test_widgets(|tester| {
                let (key_a, key_b) = (GlobalKey::new(), GlobalKey::new());
                pump_padding_chip(tester, TextDirection::Ltr, &key_a, &key_b);

                let avatar = by_key(tester, &key_a);
                let label = by_key(tester, &key_b);
                let icon = only::<Icon>(tester);
                assert_eq!(top_left_of(tester, avatar), Offset::new(332.0, 280.0));
                assert_eq!(bottom_right_of(tester, avatar), Offset::new(372.0, 320.0));
                assert_eq!(top_left_of(tester, label), Offset::new(380.0, 280.0));
                assert_eq!(bottom_right_of(tester, label), Offset::new(420.0, 320.0));
                assert_eq!(top_left_of(tester, icon), Offset::new(439.0, 291.0));
                assert_eq!(bottom_right_of(tester, icon), Offset::new(457.0, 309.0));
            });
        }

        #[test]
        fn chip_padding_rtl() {
            test_widgets(|tester| {
                let (key_a, key_b) = (GlobalKey::new(), GlobalKey::new());
                pump_padding_chip(tester, TextDirection::Rtl, &key_a, &key_b);

                let avatar = by_key(tester, &key_a);
                let label = by_key(tester, &key_b);
                let icon = only::<Icon>(tester);
                assert_eq!(top_left_of(tester, avatar), Offset::new(428.0, 280.0));
                assert_eq!(bottom_right_of(tester, avatar), Offset::new(468.0, 320.0));
                assert_eq!(top_left_of(tester, label), Offset::new(380.0, 280.0));
                assert_eq!(bottom_right_of(tester, label), Offset::new(420.0, 320.0));
                assert_eq!(top_left_of(tester, icon), Offset::new(343.0, 291.0));
                assert_eq!(bottom_right_of(tester, icon), Offset::new(361.0, 309.0));
            });
        }

        /// One expected frame of a drawer test: the chip width and the item's left edge, with the
        /// label's left edge when Dart asserts it.
        struct DrawerFrame {
            chip_width: f64,
            item_dx: f64,
            label_dx: Option<f64>,
        }

        fn frame(chip_width: f64, item_dx: f64, label_dx: Option<f64>) -> DrawerFrame {
            DrawerFrame {
                chip_width,
                item_dx,
                label_dx,
            }
        }

        fn expect_drawer_frames(
            tester: &mut WidgetTester,
            item_key: &GlobalKey,
            label_key: &GlobalKey,
            frames: &[DrawerFrame],
        ) {
            for frame in frames {
                pump_ms(tester, 20);
                let chip = tester.get_size(only::<RawChip>(tester));
                assert_more_or_less_equals_within(chip.width(), frame.chip_width, 0.1);
                assert_eq!(chip.height(), 48.0);
                assert_eq!(
                    tester.get_size(by_key(tester, item_key)),
                    Size::new(24.0, 24.0)
                );
                assert_more_or_less_equals_within(
                    top_left_of(tester, by_key(tester, item_key)).dx(),
                    frame.item_dx,
                    0.1,
                );
                if let Some(label_dx) = frame.label_dx {
                    assert_more_or_less_equals_within(
                        top_left_of(tester, by_key(tester, label_key)).dx(),
                        label_dx,
                        0.1,
                    );
                }
            }
        }

        #[test]
        fn material2_avatar_drawer_works_as_expected_on_raw_chip() {
            test_widgets(|tester| {
                let theme = m2_theme(tester);
                let label_key = GlobalKey::new();
                let avatar_key = GlobalKey::new();
                let push_chip = |tester: &mut WidgetTester, with_avatar: bool| {
                    let mut chip = RawChip::new(Text::new("Chip").key(label_key.clone().into()))
                        .shape(StadiumBorder::new(BorderSide::NONE));
                    if with_avatar {
                        chip = chip.avatar(
                            Container::new()
                                .key(avatar_key.clone().into())
                                .color(Color::new(0xff000000))
                                .width(40.0)
                                .height(40.0),
                        );
                    }
                    let root = wrap_with_theme(
                        tester,
                        theme.clone(),
                        Wrap::new().children([chip.into_widget()]),
                    );
                    tester.pump_widget(root);
                };
                let chip_size = |tester: &WidgetTester| tester.get_size(only::<RawChip>(tester));
                let avatar = |tester: &WidgetTester| by_key(tester, &avatar_key);
                let label = |tester: &WidgetTester| by_key(tester, &label_key);

                // No avatar
                push_chip(tester, false);
                assert_eq!(chip_size(tester), Size::new(80.0, 48.0));

                // Add an avatar
                push_chip(tester, true);
                // Avatar drawer should start out closed.
                assert_eq!(chip_size(tester), Size::new(80.0, 48.0));
                assert_eq!(tester.get_size(avatar(tester)), Size::new(24.0, 24.0));
                assert_eq!(
                    top_left_of(tester, avatar(tester)),
                    Offset::new(-20.0, 12.0)
                );
                assert_eq!(top_left_of(tester, label(tester)), Offset::new(12.0, 17.0));

                // Avatar drawer should start expanding.
                expect_drawer_frames(
                    tester,
                    &avatar_key,
                    &label_key,
                    &[
                        frame(81.2, -18.8, Some(13.2)),
                        frame(86.7, -13.3, Some(18.6)),
                        frame(94.7, -5.3, Some(26.7)),
                        frame(99.5, -0.5, Some(31.5)),
                    ],
                );

                // Wait for being done with animation, and make sure it didn't change height.
                tester.pump_and_settle_by(std::time::Duration::from_millis(200));
                assert_eq!(chip_size(tester), Size::new(104.0, 48.0));
                assert_eq!(chip_size(tester).height(), 48.0);
                assert_eq!(tester.get_size(avatar(tester)), Size::new(24.0, 24.0));
                assert_eq!(top_left_of(tester, avatar(tester)), Offset::new(4.0, 12.0));
                assert_eq!(top_left_of(tester, label(tester)), Offset::new(36.0, 17.0));

                // Remove the avatar again
                push_chip(tester, false);
                // Avatar drawer should start out open.
                assert_eq!(chip_size(tester), Size::new(104.0, 48.0));
                assert_eq!(tester.get_size(avatar(tester)), Size::new(24.0, 24.0));
                assert_eq!(top_left_of(tester, avatar(tester)), Offset::new(4.0, 12.0));
                assert_eq!(top_left_of(tester, label(tester)), Offset::new(36.0, 17.0));

                // Avatar drawer should start contracting.
                expect_drawer_frames(
                    tester,
                    &avatar_key,
                    &label_key,
                    &[
                        frame(102.9, 2.9, Some(34.9)),
                        frame(98.0, -2.0, Some(30.0)),
                        frame(84.1, -15.9, Some(16.1)),
                        frame(80.0, -20.0, Some(12.0)),
                    ],
                );

                // Wait for being done with animation, make sure it didn't change height, and make sure
                // that the avatar is no longer drawn.
                tester.pump_and_settle_by(std::time::Duration::from_millis(200));
                assert_eq!(chip_size(tester), Size::new(80.0, 48.0));
                assert_eq!(top_left_of(tester, label(tester)), Offset::new(12.0, 17.0));
                let avatar_key_ref: waft_widgets::KeyRef = avatar_key.clone().into();
                assert!(tester.find_onstage_by_key(&avatar_key_ref).is_empty());
            });
        }

        #[test]
        fn material2_delete_button_drawer_works_as_expected_on_raw_chip() {
            test_widgets(|tester| {
                let theme = m2_theme(tester);
                let label_key = GlobalKey::new();
                let delete_key = GlobalKey::new();
                let was_deleted = Rc::new(Cell::new(false));
                let push_chip = |tester: &mut WidgetTester, deletable: bool| {
                    let mut chip = RawChip::new(Text::new("Chip").key(label_key.clone().into()))
                        .delete_icon(
                            Container::new()
                                .key(delete_key.clone().into())
                                .color(Colors::BLUE.primary())
                                .width(40.0)
                                .height(40.0),
                        )
                        .shape(StadiumBorder::new(BorderSide::NONE));
                    if deletable {
                        let was_deleted = Rc::clone(&was_deleted);
                        chip = chip.on_deleted(Listener::new(move |_app| was_deleted.set(true)));
                    }
                    let root = wrap_with_theme(
                        tester,
                        theme.clone(),
                        Wrap::new().children([chip.into_widget()]),
                    );
                    tester.pump_widget(root);
                };
                let chip_size = |tester: &WidgetTester| tester.get_size(only::<RawChip>(tester));
                let delete = |tester: &WidgetTester| by_key(tester, &delete_key);
                let label = |tester: &WidgetTester| by_key(tester, &label_key);

                // No delete button
                push_chip(tester, false);
                assert_eq!(chip_size(tester), Size::new(80.0, 48.0));

                // Add a delete button
                push_chip(tester, true);
                // Delete button drawer should start out closed.
                assert_eq!(chip_size(tester), Size::new(80.0, 48.0));
                assert_eq!(tester.get_size(delete(tester)), Size::new(24.0, 24.0));
                assert_eq!(top_left_of(tester, delete(tester)), Offset::new(52.0, 12.0));
                assert_eq!(top_left_of(tester, label(tester)), Offset::new(12.0, 17.0));

                // Delete button drawer should start expanding.
                pump_ms(tester, 20);
                assert_more_or_less_equals_within(chip_size(tester).width(), 81.2, 0.1);
                assert_eq!(tester.get_size(delete(tester)), Size::new(24.0, 24.0));
                assert_more_or_less_equals_within(
                    top_left_of(tester, delete(tester)).dx(),
                    53.2,
                    0.1,
                );
                assert_eq!(top_left_of(tester, label(tester)), Offset::new(12.0, 17.0));
                for (chip_width, delete_dx) in [(86.7, 58.7), (94.7, 66.7), (99.5, 71.5)] {
                    pump_ms(tester, 20);
                    assert_more_or_less_equals_within(chip_size(tester).width(), chip_width, 0.1);
                    assert_eq!(tester.get_size(delete(tester)), Size::new(24.0, 24.0));
                    assert_more_or_less_equals_within(
                        top_left_of(tester, delete(tester)).dx(),
                        delete_dx,
                        0.1,
                    );
                }

                // Wait for being done with animation, and make sure it didn't change height.
                tester.pump_and_settle_by(std::time::Duration::from_millis(200));
                assert_eq!(chip_size(tester), Size::new(104.0, 48.0));
                assert_eq!(tester.get_size(delete(tester)), Size::new(24.0, 24.0));
                assert_eq!(top_left_of(tester, delete(tester)), Offset::new(76.0, 12.0));
                assert_eq!(top_left_of(tester, label(tester)), Offset::new(12.0, 17.0));

                // Test the tap work for the delete button, but not the rest of the chip.
                assert!(!was_deleted.get());
                tester.tap_at(tester.get_center(label(tester)), 1);
                assert!(!was_deleted.get());
                tester.tap_at(tester.get_center(delete(tester)), 1);
                assert!(was_deleted.get());

                // Remove the delete button again
                push_chip(tester, false);
                // Delete button drawer should start out open.
                assert_eq!(chip_size(tester), Size::new(104.0, 48.0));
                assert_eq!(tester.get_size(delete(tester)), Size::new(24.0, 24.0));
                assert_eq!(top_left_of(tester, delete(tester)), Offset::new(76.0, 12.0));
                assert_eq!(top_left_of(tester, label(tester)), Offset::new(12.0, 17.0));

                // Delete button drawer should start contracting.
                pump_ms(tester, 20);
                assert_more_or_less_equals_within(chip_size(tester).width(), 103.8, 0.1);
                assert_eq!(tester.get_size(delete(tester)), Size::new(24.0, 24.0));
                assert_more_or_less_equals_within(
                    top_left_of(tester, delete(tester)).dx(),
                    75.8,
                    0.1,
                );
                assert_eq!(top_left_of(tester, label(tester)), Offset::new(12.0, 17.0));
                for (chip_width, delete_dx) in [(102.9, 74.9), (101.0, 73.0), (97.5, 69.5)] {
                    pump_ms(tester, 20);
                    assert_more_or_less_equals_within(chip_size(tester).width(), chip_width, 0.1);
                    assert_eq!(tester.get_size(delete(tester)), Size::new(24.0, 24.0));
                    assert_more_or_less_equals_within(
                        top_left_of(tester, delete(tester)).dx(),
                        delete_dx,
                        0.1,
                    );
                }

                // Wait for being done with animation, make sure it didn't change height, and make sure
                // that the delete button is no longer drawn.
                tester.pump_and_settle_by(std::time::Duration::from_millis(200));
                assert_eq!(chip_size(tester), Size::new(80.0, 48.0));
                assert_eq!(top_left_of(tester, label(tester)), Offset::new(12.0, 17.0));
                let delete_key_ref: waft_widgets::KeyRef = delete_key.clone().into();
                assert!(tester.find_onstage_by_key(&delete_key_ref).is_empty());
            });
        }
    }

    mod ink_and_selection {
        //! chip_test.dart: the ink splash cases of the delete button and the
        //! label, and the selection and activation cases of `RawChip`.
        use waft_widgets::testing::{
            TestPointer, WidgetTester, assert_more_or_less_equals_within, expect, find,
            finds_nothing, finds_one_widget, test_widgets,
        };

        use std::time::Duration;

        use waft_embedder::{Offset, TextDirection};
        use waft_widgets::{
            AnyElement, Container, GlobalKey, Icon, IntoWidget, KeyRef, Text, WidgetRef, Wrap,
        };

        use crate::chip::RawChip;
        use crate::chip::tests::{by_key, only, wrap_for_chip};
        use crate::icons::Icons;
        use crate::ink_well::{InkResponse, InkWell};
        use crate::material::RenderInkFeatures;
        use crate::theme_data::ThemeData;
        use crate::tooltip::tests::m2;
        use waft_foundation::Listener;
        use waft_painting::TextScaler;
        use waft_rendering::testing::CanvasMethod;

        /// Dart's `chipWithOptionalDeleteButton`.
        struct ChipWithOptionalDeleteButton {
            delete_button_key: Option<GlobalKey>,
            label_key: Option<GlobalKey>,
            deletable: bool,
            text_direction: TextDirection,
            size: Option<f64>,
            on_pressed: bool,
            theme_data: Option<ThemeData>,
        }

        impl ChipWithOptionalDeleteButton {
            fn new(deletable: bool) -> ChipWithOptionalDeleteButton {
                ChipWithOptionalDeleteButton {
                    delete_button_key: None,
                    label_key: None,
                    deletable,
                    text_direction: TextDirection::Ltr,
                    size: None,
                    on_pressed: true,
                    theme_data: None,
                }
            }

            fn build(self, tester: &WidgetTester) -> WidgetRef {
                let mut delete_icon = Icon::new(Some(Icons::close()));
                if let Some(key) = &self.delete_button_key {
                    delete_icon = delete_icon.key(KeyRef::from(key.clone()));
                }
                if let Some(size) = self.size {
                    delete_icon = delete_icon.size(size);
                }
                let mut label = Text::new(if self.deletable {
                    "Chip with Delete Button"
                } else {
                    "Chip without Delete Button"
                });
                if let Some(key) = &self.label_key {
                    label = label.key(KeyRef::from(key.clone()));
                }
                let mut chip = RawChip::new(label).delete_icon(delete_icon);
                if self.on_pressed {
                    chip = chip.on_pressed(Listener::new(|_app| {}));
                }
                if self.deletable {
                    chip = chip.on_deleted(Listener::new(|_app| {}));
                }
                wrap_for_chip(
                    tester,
                    Wrap::new().children([chip.into_widget()]).into_widget(),
                    self.text_direction,
                    TextScaler::NO_SCALING,
                    self.theme_data,
                )
            }
        }

        /// The circles the ink features of the chip's `Material` draw, as `(center, radius)`:
        /// Dart's `drawCircle` calls that `ripplePattern` reads, which Valo records as paths that are
        /// ovals.
        fn ink_circles(tester: &WidgetTester) -> Vec<(Offset, f64)> {
            let chip_ink_features = *tester
                .all::<RenderInkFeatures>()
                .last()
                .expect("the chip's Material");
            let mut canvas = waft_embedder::Canvas::new();
            chip_ink_features.paint_ink_features(
                &mut tester.cell.borrow_mut(),
                &mut canvas,
                Offset::ZERO,
            );
            canvas
                .build()
                .ops()
                .iter()
                .filter(|op| CanvasMethod::of(op) == CanvasMethod::DrawCircle)
                .filter_map(|op| match op {
                    waft_embedder::valo::Op::DrawPath { path, .. } => path.as_oval(),
                    _ => None,
                })
                .map(|oval| {
                    let width = f64::from(oval.width);
                    let center = Offset::new(
                        f64::from(oval.x) + width / 2.0,
                        f64::from(oval.y) + f64::from(oval.height) / 2.0,
                    );
                    (center, width / 2.0)
                })
                .collect()
        }

        fn offsets_are_close(a: Offset, b: Offset) -> bool {
            (a - b).distance() < 1.0
        }

        fn radii_are_close(a: f64, b: f64) -> bool {
            (a - b).abs() < 1.0
        }

        /// Dart's `ripplePattern`: there is at least one ripple with `expected_center` and
        /// `expected_radius`.
        fn assert_ripple(tester: &WidgetTester, expected_center: Offset, expected_radius: f64) {
            let circles = ink_circles(tester);
            assert!(
                circles.iter().any(|(center, radius)| {
                    offsets_are_close(*center, expected_center)
                        && radii_are_close(*radius, expected_radius)
                }),
                "Expected a circle with center == {expected_center:?}, radius == {expected_radius}; \
         found {circles:?}"
            );
        }

        /// Dart's `uniqueRipplePattern`: there are no ripples other than the one with
        /// `expected_center` and `expected_radius`.
        fn assert_unique_ripple(
            tester: &WidgetTester,
            expected_center: Offset,
            expected_radius: f64,
        ) {
            for (center, radius) in ink_circles(tester) {
                assert!(
                    offsets_are_close(center, expected_center)
                        && radii_are_close(radius, expected_radius),
                    "Expected: center == {expected_center:?}, radius == {expected_radius}\n\
             Found: center == {center:?} radius == {radius}"
                );
            }
        }

        /// Dart's `findTooltipContainer(text).length`: the `Container`s above the text.
        fn tooltip_containers(tester: &WidgetTester, tooltip_text: &str) -> usize {
            find::text(tooltip_text)
                .evaluate(tester)
                .into_iter()
                .flat_map(|text| tester.ancestors(text))
                .filter(|ancestor| {
                    let app = tester.cell.borrow();
                    waft_widgets::downcast_widget::<Container>(&**ancestor.widget(&app)).is_some()
                })
                .count()
        }

        fn assert_no_tooltip(tester: &WidgetTester) {
            assert_eq!(tooltip_containers(tester, "Delete"), 0);
        }

        fn pump_ms(tester: &mut WidgetTester, ms: u64) {
            tester.pump_by(Duration::from_millis(ms));
        }

        fn start_gesture(tester: &mut WidgetTester, at: Offset) -> TestPointer {
            let mut gesture = TestPointer::touch(1);
            tester.down(&mut gesture, at);
            gesture
        }

        #[test]
        fn material2_chip_creates_centered_unique_ripple_when_label_is_tapped() {
            test_widgets(|tester| {
                let label_key = GlobalKey::new();
                let delete_button_key = GlobalKey::new();
                let mut chip = ChipWithOptionalDeleteButton::new(true);
                chip.theme_data = Some(m2(&tester.cell.borrow()));
                chip.label_key = Some(label_key.clone());
                chip.delete_button_key = Some(delete_button_key);
                let root = chip.build(tester);
                tester.pump_widget(root);

                // Taps at a location close to the center of the label.
                let center_of_label = tester.get_center(by_key(tester, &label_key));
                let tap_location_of_label = center_of_label + Offset::new(-10.0, -10.0);
                let gesture = start_gesture(tester, tap_location_of_label);
                tester.pump();

                // Waits for 100 ms.
                pump_ms(tester, 100);

                // There should be one unique, centered ink ripple.
                assert_ripple(tester, Offset::new(163.0, 6.0), 20.9);
                assert_unique_ripple(tester, Offset::new(163.0, 6.0), 20.9);

                // There should be no tooltip.
                assert_no_tooltip(tester);

                // Waits for 100 ms again.
                pump_ms(tester, 100);

                // The ripple should grow, with the same center.
                assert_ripple(tester, Offset::new(163.0, 6.0), 41.8);
                assert_unique_ripple(tester, Offset::new(163.0, 6.0), 41.8);

                // There should be no tooltip.
                assert_no_tooltip(tester);

                // Waits for a very long time.
                tester.pump_and_settle();

                // There should still be no tooltip.
                assert_no_tooltip(tester);

                tester.up(&gesture);
            });
        }

        /// The Material 3 half of Dart's sparkle cases. Only the `matchesGoldenFile` expectations are
        /// left out: the tooltip expectations and the gesture's timing are what is checked.
        #[test]
        fn material3_chip_creates_centered_unique_sparkle_when_label_is_tapped() {
            test_widgets(|tester| {
                let label_key = GlobalKey::new();
                let delete_button_key = GlobalKey::new();
                let mut chip = ChipWithOptionalDeleteButton::new(true);
                chip.label_key = Some(label_key.clone());
                chip.delete_button_key = Some(delete_button_key);
                let root = chip.build(tester);
                tester.pump_widget(root);

                // Taps at a location close to the center of the label.
                let center_of_label = tester.get_center(by_key(tester, &label_key));
                let gesture = start_gesture(tester, center_of_label + Offset::new(-10.0, -10.0));
                tester.pump();

                // Waits for 100 ms.
                pump_ms(tester, 100);

                // There should be no tooltip.
                assert_no_tooltip(tester);

                // Waits for 100 ms again.
                pump_ms(tester, 100);

                // There should be no tooltip.
                assert_no_tooltip(tester);

                // Waits for a very long time.
                tester.pump_and_settle();

                // There should still be no tooltip.
                assert_no_tooltip(tester);

                tester.up(&gesture);
            });
        }

        #[test]
        fn delete_button_is_focusable() {
            test_widgets(|tester| {
                let label_key = GlobalKey::new();
                let delete_button_key = GlobalKey::new();
                let mut chip = ChipWithOptionalDeleteButton::new(true);
                chip.label_key = Some(label_key.clone());
                chip.delete_button_key = Some(delete_button_key.clone());
                let root = chip.build(tester);
                tester.pump_widget(root);

                let focus_of = |tester: &WidgetTester, key: &GlobalKey| {
                    let mut app = tester.cell.borrow_mut();
                    let context = key
                        .current_context(&app)
                        .expect("the keyed widget is mounted");
                    waft_widgets::Focus::of(&mut app, context, false, true)
                };

                let delete_node = focus_of(tester, &delete_button_key);
                delete_node.request_focus(&mut tester.cell.borrow_mut(), None);
                tester.pump();

                // They shouldn't have the same focus node.
                let label_node = focus_of(tester, &label_key);
                let delete_node = focus_of(tester, &delete_button_key);
                assert_ne!(delete_node, label_node);
                assert!(delete_node.has_focus(&mut tester.cell.borrow_mut()));
                assert!(delete_node.has_primary_focus(&tester.cell.borrow()));
                // Delete button is a child widget of the Chip, so the Chip should have focus if
                // the delete button does.
                assert!(label_node.has_focus(&mut tester.cell.borrow_mut()));
                assert!(!label_node.has_primary_focus(&tester.cell.borrow()));

                label_node.request_focus(&mut tester.cell.borrow_mut(), None);
                tester.pump();

                assert!(!delete_node.has_focus(&mut tester.cell.borrow_mut()));
                assert!(!delete_node.has_primary_focus(&tester.cell.borrow()));
                assert!(label_node.has_focus(&mut tester.cell.borrow_mut()));
                assert!(label_node.has_primary_focus(&tester.cell.borrow()));
            });
        }

        /// The case of Dart's delete button ripple and sparkle tests: `on_pressed` is `doNothing` or
        /// null, `material3` picks the theme and the 18.0 icon size of the sparkle cases.
        fn delete_button_tap(material3: bool, on_pressed: bool) {
            test_widgets(|tester| {
                let label_key = GlobalKey::new();
                let delete_button_key = GlobalKey::new();
                let mut chip = ChipWithOptionalDeleteButton::new(true);
                if material3 {
                    chip.size = Some(18.0);
                } else {
                    chip.theme_data = Some(m2(&tester.cell.borrow()));
                }
                chip.label_key = Some(label_key);
                chip.on_pressed = on_pressed;
                chip.delete_button_key = Some(delete_button_key.clone());
                let root = chip.build(tester);
                tester.pump_widget(root);

                // Taps at a location close to the center of the delete icon.
                let center_of_delete_button = tester.get_center(by_key(tester, &delete_button_key));
                let gesture =
                    start_gesture(tester, center_of_delete_button + Offset::new(-10.0, -10.0));
                tester.pump();

                // Waits for 200 ms.
                pump_ms(tester, 100);
                pump_ms(tester, 100);

                // There should be one unique ink ripple.
                if !material3 {
                    assert_ripple(tester, Offset::ZERO, 1.44);
                    assert_unique_ripple(tester, Offset::ZERO, 1.44);
                }

                // There should be no tooltip.
                assert_no_tooltip(tester);

                // Waits for 200 ms again.
                pump_ms(tester, 100);
                pump_ms(tester, 100);

                // The ripple should grow, but the center should move,
                // towards the center of the delete icon.
                if !material3 {
                    assert_ripple(tester, Offset::new(2.0, 2.0), 4.32);
                    assert_unique_ripple(tester, Offset::new(2.0, 2.0), 4.32);
                }

                // There should be no tooltip.
                assert_no_tooltip(tester);

                // Waits for a very long time.
                // This is pressing and holding the delete button.
                tester.pump_and_settle();

                // There should be a tooltip.
                assert_eq!(tooltip_containers(tester, "Delete"), 1);

                tester.up(&gesture);
            });
        }

        #[test]
        fn material2_delete_button_creates_centered_unique_ripple_when_tapped() {
            delete_button_tap(false, true);
        }

        /// Dart's golden expectations of the sparkle are left out.
        #[test]
        fn material3_delete_button_creates_non_centered_unique_sparkle_when_tapped() {
            delete_button_tap(true, true);
        }

        #[test]
        fn material2_delete_button_in_a_chip_with_null_on_pressed_creates_ripple_when_tapped() {
            delete_button_tap(false, false);
        }

        /// Dart's golden expectations of the sparkle are left out.
        #[test]
        fn material3_delete_button_in_a_chip_with_null_on_pressed_creates_sparkle_when_tapped() {
            delete_button_tap(true, false);
        }

        #[test]
        fn rtl_delete_button_responds_to_tap_on_the_left_of_the_chip() {
            test_widgets(|tester| {
                // Creates an RTL chip with a delete button.
                let mut chip = ChipWithOptionalDeleteButton::new(true);
                chip.label_key = Some(GlobalKey::new());
                chip.delete_button_key = Some(GlobalKey::new());
                chip.text_direction = TextDirection::Rtl;
                let root = chip.build(tester);
                tester.pump_widget(root);

                // Taps at a location close to the center of the delete icon,
                // which is on the left side of the chip.
                let first_ink_well: AnyElement = find::by_type::<InkWell>().evaluate(tester)[0];
                let top_left_of_ink_well = tester.get_rect(first_ink_well).top_left();
                let tap_location = top_left_of_ink_well + Offset::new(8.0, 8.0);
                let gesture = start_gesture(tester, tap_location);
                tester.pump();

                tester.pump_and_settle();

                // The existence of a 'Delete' tooltip indicates the delete icon is tapped,
                // instead of the label.
                assert_eq!(tooltip_containers(tester, "Delete"), 1);

                tester.up(&gesture);
            });
        }

        fn chip_without_delete_button(material3: bool) {
            test_widgets(|tester| {
                let mut chip = ChipWithOptionalDeleteButton::new(false);
                chip.label_key = Some(GlobalKey::new());
                if !material3 {
                    chip.theme_data = Some(m2(&tester.cell.borrow()));
                }
                let root = chip.build(tester);
                tester.pump_widget(root);

                // Taps at a location close to the bottom-right corner of the chip.
                let ink_well = only::<InkWell>(tester);
                let bottom_right_of_ink_well = tester.get_rect(ink_well).bottom_right();
                let tap_location = bottom_right_of_ink_well + Offset::new(-10.0, -10.0);
                let gesture = start_gesture(tester, tap_location);
                tester.pump();

                // Waits for 100 ms.
                pump_ms(tester, 100);

                // There should be exactly one ink-creating widget.
                expect(tester, &find::by_type::<InkWell>(), finds_one_widget());
                expect(tester, &find::by_type::<InkResponse>(), finds_nothing());

                // There should be one unique, centered ink ripple.
                if !material3 {
                    assert_ripple(tester, Offset::new(378.0, 22.0), 37.9);
                    assert_unique_ripple(tester, Offset::new(378.0, 22.0), 37.9);
                }

                // There should be no tooltip.
                assert_no_tooltip(tester);

                // Waits for 100 ms again.
                pump_ms(tester, 100);

                // The ripple should grow, with the same center.
                // This indicates that the tap is not on a delete icon.
                if !material3 {
                    assert_ripple(tester, Offset::new(378.0, 22.0), 75.8);
                    assert_unique_ripple(tester, Offset::new(378.0, 22.0), 75.8);
                }

                // There should be no tooltip.
                assert_no_tooltip(tester);

                // Waits for a very long time.
                tester.pump_and_settle();

                // There should still be no tooltip.
                // This indicates that the tap is not on a delete icon.
                assert_no_tooltip(tester);

                tester.up(&gesture);
            });
        }

        #[test]
        fn material2_chip_without_delete_button_creates_correct_ripple() {
            chip_without_delete_button(false);
        }

        /// Dart's golden expectations of the sparkle are left out.
        #[test]
        fn material3_chip_without_delete_button_creates_correct_sparkle() {
            chip_without_delete_button(true);
        }

        /// The progress of the three animations of the render chip: Dart's `getSelectProgress`,
        /// `getAvatarDrawerProgress` and `getDeleteDrawerProgress`.
        fn progress(tester: &WidgetTester) -> (f64, f64, f64) {
            let render_chip = tester.first::<crate::chip::RenderChip>();
            let app = tester.cell.borrow();
            let chip = render_chip.get(&app);
            (
                chip.checkmark_animation.value(&app),
                chip.avatar_drawer_animation.value(&app),
                chip.delete_drawer_animation.value(&app),
            )
        }

        /// Dart's `moreOrLessEquals(expected, epsilon: 0.01)` or `equals(expected)`.
        #[derive(Clone, Copy)]
        enum Progress {
            Exactly(f64),
            About(f64),
        }

        fn assert_progress(tester: &WidgetTester, select: Progress, avatar: Progress, delete: f64) {
            let (actual_select, actual_avatar, actual_delete) = progress(tester);
            for (actual, expected) in [(actual_select, select), (actual_avatar, avatar)] {
                match expected {
                    Progress::Exactly(expected) => assert_eq!(actual, expected),
                    Progress::About(expected) => {
                        assert_more_or_less_equals_within(actual, expected, 0.01)
                    }
                }
            }
            assert_eq!(actual_delete, delete);
        }

        fn transient_callback_count(tester: &WidgetTester) -> usize {
            waft_scheduler::SchedulerBinding::transient_callback_count(
                &mut tester.cell.borrow_mut(),
            )
        }

        /// What `pushChip` of Dart's selection and activation cases builds.
        struct SelectableChip {
            material3: bool,
            selected: std::rc::Rc<std::cell::Cell<bool>>,
            label_key: GlobalKey,
            avatar_key: Option<GlobalKey>,
            selectable: bool,
            show_checkmark: Option<bool>,
        }

        impl SelectableChip {
            fn new(material3: bool) -> SelectableChip {
                SelectableChip {
                    material3,
                    selected: Default::default(),
                    label_key: GlobalKey::new(),
                    avatar_key: None,
                    selectable: false,
                    show_checkmark: None,
                }
            }

            fn push(&self, tester: &mut WidgetTester) {
                let selected = std::rc::Rc::clone(&self.selected);
                let label_key = self.label_key.clone();
                let avatar_key = self.avatar_key.clone();
                let selectable = self.selectable;
                let show_checkmark = self.show_checkmark;
                let builder =
                    waft_widgets::StatefulBuilder::new(move |_app, _context, set_state| {
                        let mut chip = RawChip::new(
                            Text::new("Long Chip Label").key(KeyRef::from(label_key.clone())),
                        )
                        .selected(selected.get())
                        .shape(waft_painting::StadiumBorder::new(
                            waft_painting::BorderSide::NONE,
                        ));
                        if let Some(avatar_key) = &avatar_key {
                            chip = chip.avatar(
                                waft_widgets::SizedBox::new()
                                    .width(40.0)
                                    .height(40.0)
                                    .key(KeyRef::from(avatar_key.clone())),
                            );
                        }
                        if let Some(show_checkmark) = show_checkmark {
                            chip = chip.show_checkmark(show_checkmark);
                        }
                        if selectable {
                            let selected = std::rc::Rc::clone(&selected);
                            chip = chip.on_selected(std::rc::Rc::new(move |app, value| {
                                let selected = std::rc::Rc::clone(&selected);
                                set_state(app, &mut move |_app| selected.set(value));
                            }));
                        }
                        Wrap::new().children([chip.into_widget()]).into_widget()
                    });
                let theme = (!self.material3).then(|| m2(&tester.cell.borrow()));
                let root = wrap_for_chip(
                    tester,
                    builder.into_widget(),
                    TextDirection::Ltr,
                    TextScaler::NO_SCALING,
                    theme,
                );
                tester.pump_widget(root);
            }

            /// Simulates a tap on the label.
            fn tap_label(&self, tester: &mut WidgetTester) {
                let at = tester.get_center(by_key(tester, &self.label_key));
                tester.tap_at(at, 1);
            }
        }

        /// Dart's `equals(size)`, to the f32 precision the text metrics of the platform fonts carry.
        fn assert_chip_size(tester: &WidgetTester, expected: waft_embedder::Size) {
            let actual = tester.get_size(only::<RawChip>(tester));
            assert!(
                (actual.width() - expected.width()).abs() < 1e-4
                    && (actual.height() - expected.height()).abs() < 1e-4,
                "{actual:?} is not {expected:?}"
            );
        }

        /// The transient callbacks of a tap: Material 2's ink splash is one more than Material 3's.
        fn tap_transient_callbacks(material3: bool) -> usize {
            if material3 { 1 } else { 2 }
        }

        fn selection_with_avatar(material3: bool, size: waft_embedder::Size) {
            test_widgets(|tester| {
                let mut chip = SelectableChip::new(material3);

                // With avatar, but not selectable.
                chip.avatar_key = Some(GlobalKey::new());
                chip.push(tester);
                assert_chip_size(tester, size);

                // Turn on selection.
                chip.selectable = true;
                chip.push(tester);
                tester.pump_and_settle();

                assert_eq!(transient_callback_count(tester), 0);

                // Simulate a tap on the label to select the chip.
                chip.tap_label(tester);
                assert!(chip.selected.get());
                assert_eq!(
                    transient_callback_count(tester),
                    tap_transient_callbacks(material3)
                );
                tester.pump();
                pump_ms(tester, 50);
                assert_progress(tester, Progress::About(0.002), Progress::Exactly(1.0), 0.0);
                pump_ms(tester, 50);
                assert_progress(tester, Progress::About(0.54), Progress::Exactly(1.0), 0.0);
                pump_ms(tester, 100);
                assert_progress(tester, Progress::Exactly(1.0), Progress::Exactly(1.0), 0.0);
                tester.pump_and_settle();

                // Simulate another tap on the label to deselect the chip.
                chip.tap_label(tester);
                assert!(!chip.selected.get());
                assert_eq!(
                    transient_callback_count(tester),
                    tap_transient_callbacks(material3)
                );
                tester.pump();
                pump_ms(tester, 20);
                assert_progress(tester, Progress::About(0.875), Progress::Exactly(1.0), 0.0);
                pump_ms(tester, 20);
                assert_progress(tester, Progress::About(0.13), Progress::Exactly(1.0), 0.0);
                pump_ms(tester, 100);
                assert_progress(tester, Progress::Exactly(0.0), Progress::Exactly(1.0), 0.0);
            });
        }

        #[test]
        fn material2_selection_with_avatar_works_as_expected_on_raw_chip() {
            selection_with_avatar(false, waft_embedder::Size::new(258.0, 48.0));
        }

        #[test]
        fn material3_selection_with_avatar_works_as_expected_on_raw_chip() {
            selection_with_avatar(true, waft_embedder::Size::new(265.5, 48.0));
        }

        fn selection_without_avatar(material3: bool, size: waft_embedder::Size) {
            test_widgets(|tester| {
                let mut chip = SelectableChip::new(material3);

                // Without avatar, but not selectable.
                chip.push(tester);
                assert_chip_size(tester, size);

                // Turn on selection.
                chip.selectable = true;
                chip.push(tester);
                tester.pump_and_settle();

                if material3 {
                    assert_eq!(transient_callback_count(tester), 0);
                }

                // Simulate a tap on the label to select the chip.
                chip.tap_label(tester);
                assert!(chip.selected.get());
                assert_eq!(
                    transient_callback_count(tester),
                    tap_transient_callbacks(material3)
                );
                tester.pump();
                pump_ms(tester, 50);
                assert_progress(tester, Progress::About(0.002), Progress::About(0.459), 0.0);
                pump_ms(tester, 50);
                assert_progress(tester, Progress::About(0.54), Progress::About(0.92), 0.0);
                pump_ms(tester, 100);
                assert_progress(tester, Progress::Exactly(1.0), Progress::Exactly(1.0), 0.0);

                tester.pump_and_settle();

                // Simulate another tap on the label to deselect the chip.
                chip.tap_label(tester);
                assert!(!chip.selected.get());
                assert_eq!(
                    transient_callback_count(tester),
                    tap_transient_callbacks(material3)
                );
                tester.pump();
                pump_ms(tester, 20);
                assert_progress(tester, Progress::About(0.875), Progress::About(0.96), 0.0);
                pump_ms(tester, 20);
                assert_progress(tester, Progress::About(0.13), Progress::About(0.75), 0.0);
                pump_ms(tester, 100);
                assert_progress(tester, Progress::Exactly(0.0), Progress::Exactly(0.0), 0.0);
            });
        }

        #[test]
        fn material2_selection_without_avatar_works_as_expected_on_raw_chip() {
            selection_without_avatar(false, waft_embedder::Size::new(234.0, 48.0));
        }

        #[test]
        fn material3_selection_without_avatar_works_as_expected_on_raw_chip() {
            selection_without_avatar(true, waft_embedder::Size::new(245.5, 48.0));
        }

        fn activation(material3: bool) {
            test_widgets(|tester| {
                let mut chip = SelectableChip::new(material3);
                chip.avatar_key = Some(GlobalKey::new());
                chip.selectable = true;
                chip.show_checkmark = Some(false);
                chip.push(tester);
                tester.pump_and_settle();

                if material3 {
                    assert_eq!(transient_callback_count(tester), 0);
                }

                chip.tap_label(tester);
                assert!(chip.selected.get());
                assert_eq!(
                    transient_callback_count(tester),
                    tap_transient_callbacks(material3)
                );
                tester.pump();
                pump_ms(tester, 50);
                assert_progress(tester, Progress::About(0.002), Progress::Exactly(1.0), 0.0);
                pump_ms(tester, 50);
                assert_progress(tester, Progress::About(0.54), Progress::Exactly(1.0), 0.0);
                pump_ms(tester, 100);
                assert_progress(tester, Progress::Exactly(1.0), Progress::Exactly(1.0), 0.0);
                tester.pump_and_settle();
            });
        }

        #[test]
        fn material2_activation_works_as_expected_on_raw_chip() {
            activation(false);
        }

        #[test]
        fn material3_activation_works_as_expected_on_raw_chip() {
            activation(true);
        }
    }

    mod theme_and_painting {
        //! chip_test.dart: the ThemeData chip theme, the label style merge, the tap target size, the
        //! theme colors, taps outside the chip, elevation and shadow color, hit testing, clip behavior,
        //! the avatar scrim and `InkWell`.
        use waft_widgets::testing::{
            WidgetTester, assert_more_or_less_equals_within, expect, find, finds_one_widget,
            test_widgets,
        };

        use std::cell::Cell;
        use std::rc::Rc;

        use waft_embedder::{Brightness, Clip, Color, Offset, Size, TargetPlatform};
        use waft_foundation::Listener;
        use waft_painting::{FontWeight, RoundedRectangleBorder, TextStyle};
        use waft_widgets::{
            Center, DefaultTextStyle, Icon, IntoWidget, Row, StatefulBuilder, Text,
        };

        use crate::chip::tests::{
            icon_data, label_style, material_box_paints, only, wrap_ltr, wrap_with_theme,
        };
        use crate::chip::{Chip, ChipRenderWidget, RawChip};
        use crate::chip_theme::{ChipTheme, ChipThemeData};
        use crate::circle_avatar::CircleAvatar;
        use crate::colors::Colors;
        use crate::filter_chip::FilterChip;
        use crate::icons::Icons;
        use crate::input_chip::InputChip;
        use crate::material::Material;
        use crate::theme::Theme;
        use crate::theme_data::{MaterialTapTargetSize, ThemeData};
        use waft_rendering::testing::PaintCall;

        fn theme_builder() -> crate::theme_data::ThemeDataBuilder {
            ThemeData::builder()
        }

        #[test]
        fn chip_uses_theme_data_chip_theme_if_present() {
            test_widgets(|tester| {
                let theme = theme_builder()
                    .chip_theme(ChipThemeData::new().background_color(Color::new(0xffff0000)))
                    .build(&tester.cell.borrow());
                let chip = InputChip::new(Text::new("Label")).on_pressed(Listener::new(|_| {}));
                let root = wrap_ltr(tester, Theme::new(theme.clone(), chip));
                tester.pump_widget(root);

                assert!(material_box_paints(
                    tester,
                    &[PaintCall::rrect(theme.chip_theme.background_color)]
                ));
            });
        }

        #[test]
        fn chip_merges_chip_theme_data_label_style_with_the_provided_label_style() {
            test_widgets(|tester| {
                // The font family should be preserved even if the chip overrides some label style
                // properties.
                let theme = theme_builder()
                    .font_family("MyFont")
                    .build(&tester.cell.borrow());
                let chip = Chip::new(Text::new("Label"))
                    .label_style(TextStyle::new().font_weight(FontWeight::W200));
                let root = wrap_ltr(tester, Theme::new(theme, chip));
                tester.pump_widget(root);

                let label_style = label_style(tester, "Label");
                assert!(!label_style.inherit);
                assert_eq!(label_style.font_family.as_deref(), Some("MyFont"));
                assert_eq!(label_style.font_weight, Some(FontWeight::W200));
            });
        }

        #[test]
        fn chip_theme_label_style_with_inherit_true() {
            test_widgets(|tester| {
                let theme = theme_builder()
                    .chip_theme(ChipThemeData::new().label_style(TextStyle::new().height(4.0))) // inherit: true
                    .build(&tester.cell.borrow());
                let root = wrap_ltr(tester, Theme::new(theme, Chip::new(Text::new("Label")))); // labelStyle: null
                tester.pump_widget(root);

                let label_style = label_style(tester, "Label");
                assert!(label_style.inherit); // because chipTheme.labelStyle.merge(null)
                assert_eq!(label_style.height, Some(4.0));
            });
        }

        #[test]
        fn chip_does_not_merge_inherit_false_label_style_with_the_theme_label_style() {
            test_widgets(|tester| {
                let theme = theme_builder()
                    .font_family("MyFont")
                    .build(&tester.cell.borrow());
                let chip = Chip::new(Text::new("Label")).label_style(
                    TextStyle::new()
                        .font_weight(FontWeight::W200)
                        .inherit(false),
                );
                let root = wrap_ltr(
                    tester,
                    Theme::new(
                        theme,
                        DefaultTextStyle::new(TextStyle::new().height(8.0), chip),
                    ),
                );
                tester.pump_widget(root);

                let label_style = label_style(tester, "Label");
                assert!(!label_style.inherit);
                assert_eq!(label_style.font_family, None);
                assert_eq!(label_style.height, None);
                assert_eq!(label_style.font_weight, Some(FontWeight::W200));
            });
        }

        fn pump_sized_chip(
            tester: &mut WidgetTester,
            material3: bool,
            tap_target_size: MaterialTapTargetSize,
        ) -> Size {
            let theme = theme_builder()
                .use_material3(material3)
                .material_tap_target_size(tap_target_size)
                .build(&tester.cell.borrow());
            let root = wrap_ltr(
                tester,
                Theme::new(theme, Center::new().child(RawChip::new(Text::new("test")))),
            );
            tester.pump_widget(root);
            tester.get_size(only::<RawChip>(tester))
        }

        #[test]
        fn material2_chip_size_is_configurable_by_theme_data_material_tap_target_size() {
            test_widgets(|tester| {
                assert_eq!(
                    pump_sized_chip(tester, false, MaterialTapTargetSize::Padded),
                    Size::new(80.0, 48.0)
                );
                assert_eq!(
                    pump_sized_chip(tester, false, MaterialTapTargetSize::ShrinkWrap),
                    Size::new(80.0, 32.0)
                );
            });
        }

        #[test]
        fn material3_chip_size_is_configurable_by_theme_data_material_tap_target_size() {
            test_widgets(|tester| {
                let size = pump_sized_chip(tester, true, MaterialTapTargetSize::Padded);
                assert_more_or_less_equals_within(size.width(), 90.4, 0.1);
                assert_eq!(size.height(), 48.0);

                let size = pump_sized_chip(tester, true, MaterialTapTargetSize::ShrinkWrap);
                assert_more_or_less_equals_within(size.width(), 90.4, 0.1);
                assert_eq!(size.height(), 38.0);
            });
        }

        #[test]
        fn chip_uses_the_right_theme_colors_for_the_right_components() {
            test_widgets(|tester| {
                let theme_data = theme_builder()
                    .platform(TargetPlatform::Android)
                    .primary_swatch(Colors::BLUE)
                    .build(&tester.cell.borrow());
                let default_chip_theme = ChipThemeData::from_defaults(
                    Some(theme_data.brightness()),
                    None,
                    Colors::BLUE.primary(),
                    theme_data.text_theme.body_large.clone().expect("bodyLarge"),
                );
                let value = Rc::new(Cell::new(false));
                let build_app = |chip_theme: &ChipThemeData, is_selectable: bool| {
                    let value = Rc::clone(&value);
                    let chip_theme = chip_theme.clone();
                    let child = StatefulBuilder::new({
                        let chip_theme = chip_theme.clone();
                        move |_app, _context, set_state| {
                            let current = value.get();
                            let on_selected: waft_foundation::ValueChanged<bool> = {
                                let value = Rc::clone(&value);
                                let set_state = set_state.clone();
                                Rc::new(move |app, new_value| {
                                    let value = Rc::clone(&value);
                                    set_state(app, &mut move |_app| value.set(new_value));
                                })
                            };
                            let mut chip = RawChip::new(Text::new(current.to_string()))
                                .show_checkmark(true)
                                .on_deleted(Listener::new(|_| {}))
                                .is_enabled(is_selectable)
                                .selected(is_selectable && current);
                            if let Some(shape) = chip_theme.shape.clone() {
                                chip = chip.shape(shape);
                            }
                            if is_selectable {
                                chip = chip.on_selected(on_selected);
                            }
                            chip.into_widget()
                        }
                    });
                    Theme::new(theme_data.clone(), ChipTheme::new(chip_theme, child))
                };
                let root = wrap_ltr(tester, build_app(&default_chip_theme, true));
                tester.pump_widget(root);

                let paints = |tester: &WidgetTester, color: Color| {
                    material_box_paints(tester, &[PaintCall::rrect(Some(color))])
                };
                let icon_color =
                    |tester: &WidgetTester| icon_data(tester).color.map(|color| color.color());
                let label_color =
                    |tester: &WidgetTester| crate::chip::tests::label_style_color(tester, "false");

                // Check default theme for enabled widget.
                assert!(paints(tester, default_chip_theme.background_color.unwrap()));
                assert_eq!(icon_color(tester), Some(Color::new(0xde000000)));
                assert_eq!(label_color(tester), Some(Colors::BLACK.with_alpha(0xde)));
                let center = crate::chip::tests::center_of(tester, only::<RawChip>(tester));
                tester.tap_at(center, 1);
                tester.pump_and_settle();
                assert!(paints(tester, default_chip_theme.selected_color.unwrap()));
                tester.tap_at(center, 1);
                tester.pump_and_settle();

                // Check default theme with disabled widget.
                let root = wrap_ltr(tester, build_app(&default_chip_theme, false));
                tester.pump_widget(root);
                tester.pump_and_settle();
                assert!(paints(tester, default_chip_theme.disabled_color.unwrap()));
                assert_eq!(label_color(tester), Some(Colors::BLACK.with_alpha(0xde)));

                // Apply a custom theme.
                let custom_color1 = Color::new(0xcafefeed);
                let custom_color2 = Color::new(0xdeadbeef);
                let custom_color3 = Color::new(0xbeefcafe);
                let custom_color4 = Color::new(0xaddedabe);
                let custom_theme = default_chip_theme
                    .copy_with()
                    .brightness(Brightness::Dark)
                    .background_color(custom_color1)
                    .disabled_color(custom_color2)
                    .selected_color(custom_color3)
                    .delete_icon_color(custom_color4);
                let root = wrap_ltr(tester, build_app(&custom_theme, true));
                tester.pump_widget(root);
                tester.pump_and_settle();

                // Check custom theme for enabled widget.
                assert!(paints(tester, custom_color1));
                assert_eq!(icon_color(tester), Some(custom_color4));
                assert_eq!(label_color(tester), Some(Colors::BLACK.with_alpha(0xde)));
                tester.tap_at(center, 1);
                tester.pump_and_settle();
                assert!(paints(tester, custom_color3));
                tester.tap_at(center, 1);
                tester.pump_and_settle();

                // Check custom theme with disabled widget.
                let root = wrap_ltr(tester, build_app(&custom_theme, false));
                tester.pump_widget(root);
                tester.pump_and_settle();
                assert!(paints(tester, custom_color2));
                assert_eq!(label_color(tester), Some(Colors::BLACK.with_alpha(0xde)));
            });
        }

        #[test]
        fn can_be_tapped_outside_of_chip_delete_icon() {
            test_widgets(|tester| {
                let deleted = Rc::new(Cell::new(false));
                let chip = Chip::new(Text::new("Chip A"))
                    .material_tap_target_size(MaterialTapTargetSize::Padded)
                    .shape(RoundedRectangleBorder::default())
                    .avatar(CircleAvatar::new().child(Text::new("A")))
                    .on_deleted({
                        let deleted = Rc::clone(&deleted);
                        Listener::new(move |_| deleted.set(true))
                    })
                    .delete_icon(Icon::new(Some(Icons::delete())));
                let root = wrap_ltr(tester, Row::new().children([chip.into_widget()]));
                tester.pump_widget(root);

                let top_right = tester.get_rect(only::<Chip>(tester)).top_right();
                tester.tap_at(top_right - Offset::new(2.0, -2.0), 1);
                tester.pump_and_settle();
                assert!(deleted.get());
            });
        }

        #[test]
        fn chips_can_be_tapped() {
            test_widgets(|tester| {
                let theme = crate::tooltip::tests::m3(&tester.cell.borrow());
                tester.pump_widget(crate::test_support::material_app(
                    theme,
                    Material::new()
                        .child(RawChip::new(Text::new("raw chip")))
                        .into_widget(),
                ));

                let center = crate::chip::tests::center_of(tester, only::<RawChip>(tester));
                tester.tap_at(center, 1);
            });
        }

        fn material_of_chip(tester: &WidgetTester) -> (f64, Option<Color>) {
            let material = crate::chip::tests::material_of_raw_chip(tester);
            tester.widget(material, |material: &Material| {
                (material.elevation, material.shadow_color)
            })
        }

        fn assert_elevation_and_shadow_color(tester: &mut WidgetTester, material3: bool) {
            let theme = if material3 {
                crate::tooltip::tests::m3(&tester.cell.borrow())
            } else {
                theme_builder()
                    .use_material3(false)
                    .platform(TargetPlatform::Android)
                    .primary_swatch(Colors::RED)
                    .build(&tester.cell.borrow())
            };
            let pump = |tester: &mut WidgetTester, chip: InputChip| {
                let root = if material3 {
                    wrap_with_theme(tester, theme.clone(), chip)
                } else {
                    wrap_ltr(tester, Theme::new(theme.clone(), chip))
                };
                tester.pump_widget(root);
            };

            pump(tester, InputChip::new(Text::new("Label")));
            let (elevation, shadow_color) = material_of_chip(tester);
            assert_eq!(elevation, 0.0);
            assert_eq!(
                shadow_color,
                Some(if material3 {
                    Colors::TRANSPARENT
                } else {
                    Colors::BLACK
                })
            );

            pump(
                tester,
                InputChip::new(Text::new("Label"))
                    .elevation(4.0)
                    .shadow_color(Colors::GREEN.primary())
                    .selected_shadow_color(Colors::BLUE.primary()),
            );
            tester.pump_and_settle();
            let (elevation, shadow_color) = material_of_chip(tester);
            assert_eq!(elevation, 4.0);
            assert_eq!(shadow_color, Some(Colors::GREEN.primary()));

            let mut selected = InputChip::new(Text::new("Label"))
                .shadow_color(Colors::GREEN.primary())
                .selected_shadow_color(Colors::BLUE.primary());
            selected.selected = true;
            pump(tester, selected);
            tester.pump_and_settle();
            let (_, shadow_color) = material_of_chip(tester);
            assert_eq!(shadow_color, Some(Colors::BLUE.primary()));
        }

        #[test]
        fn material2_chip_elevation_and_shadow_color_work_correctly() {
            test_widgets(|tester| assert_elevation_and_shadow_color(tester, false));
        }

        #[test]
        fn material3_chip_elevation_and_shadow_color_work_correctly() {
            test_widgets(|tester| assert_elevation_and_shadow_color(tester, true));
        }

        #[test]
        fn can_be_tapped_outside_of_chip_body() {
            test_widgets(|tester| {
                let pressed = Rc::new(Cell::new(false));
                let chip = InputChip::new(Text::new("Chip A"))
                    .material_tap_target_size(MaterialTapTargetSize::Padded)
                    .shape(RoundedRectangleBorder::default())
                    .avatar(CircleAvatar::new().child(Text::new("A")))
                    .on_pressed({
                        let pressed = Rc::clone(&pressed);
                        Listener::new(move |_| pressed.set(true))
                    });
                let root = wrap_ltr(tester, Row::new().children([chip.into_widget()]));
                tester.pump_widget(root);

                let top_center = tester.get_rect(only::<InputChip>(tester)).top_center();
                tester.tap_at(top_center, 1);
                tester.pump_and_settle();
                assert!(pressed.get());
            });
        }

        /// Dart's `find.byType(...).hitTestable()`: a hit test of the view at the center of the
        /// element reaches the element's render box.
        fn is_hit_testable(tester: &WidgetTester, element: waft_widgets::AnyElement) -> bool {
            let center = crate::chip::tests::center_of(tester, element);
            let target = tester.render_box_of(element).as_object().id();
            let mut app = tester.cell.borrow_mut();
            let view_id = app.platform().views()[0].id();
            let mut result = waft_gestures::HitTestResult::new();
            waft_gestures::GestureBinding::instance(&mut app).hit_test_in_view(
                &mut app,
                &mut result,
                center,
                view_id,
            );
            result
                .path()
                .iter()
                .any(|entry| entry.shared_target().arena_id() == Some(target))
        }

        #[test]
        fn is_hit_testable_chip() {
            test_widgets(|tester| {
                let chip = InputChip::new(Text::new("Chip A"))
                    .shape(RoundedRectangleBorder::default())
                    .avatar(CircleAvatar::new().child(Text::new("A")))
                    .on_pressed(Listener::new(|_| {}));
                let root = wrap_ltr(tester, chip);
                tester.pump_widget(root);

                assert!(is_hit_testable(tester, only::<InputChip>(tester)));
            });
        }

        fn check_chip_material_clip_behavior(tester: &WidgetTester, clip: Clip) {
            let materials = find::by_type::<Material>().evaluate(tester);
            assert_eq!(materials.len(), 2);
            let last = *materials.last().expect("a Material");
            assert_eq!(
                tester.widget(last, |material: &Material| material.clip_behavior),
                clip
            );
        }

        #[test]
        fn chip_clip_behavior_properly_passes_through_to_the_material() {
            test_widgets(|tester| {
                let root = wrap_ltr(tester, Chip::new(Text::new("label")));
                tester.pump_widget(root);
                check_chip_material_clip_behavior(tester, Clip::None);

                let root = wrap_ltr(
                    tester,
                    Chip::new(Text::new("label")).clip_behavior(Clip::AntiAlias),
                );
                tester.pump_widget(root);
                check_chip_material_clip_behavior(tester, Clip::AntiAlias);
            });
        }

        fn assert_selected_avatar_draws_scrim(
            tester: &mut WidgetTester,
            material3: bool,
            avatar_xy: f64,
            avatar_radius: f64,
        ) {
            let mut chip = FilterChip::new(Text::new("test"), None)
                .avatar(CircleAvatar::new().child(Text::new("t")));
            chip.selected = true;
            let root = if material3 {
                wrap_ltr(tester, chip)
            } else {
                let theme = theme_builder()
                    .use_material3(false)
                    .build(&tester.cell.borrow());
                wrap_with_theme(tester, theme, chip)
            };
            tester.pump_widget(root);

            let raw_chip = only::<RawChip>(tester);
            let render_widget = tester.descendants_of_type::<ChipRenderWidget>(raw_chip)[0];
            let render = tester.render_box_of(render_widget).as_object();
            let select_scrim_color = Color::new(0x60191919).to_argb32();
            let circles: Vec<(Offset, f64, u32)> = painted_ovals(tester, render);
            let center = Offset::new(avatar_xy, avatar_xy);
            let background = circles
                .iter()
                .position(|(at, radius, _)| *at == center && *radius == avatar_radius)
                .unwrap_or_else(|| panic!("the CircleAvatar background in {circles:?}"));
            assert!(
                circles[background + 1..]
                    .iter()
                    .any(|(at, radius, color)| *at == center
                        && *radius == avatar_radius
                        && *color == select_scrim_color),
                "{circles:?}"
            );
        }

        #[test]
        fn material2_selected_chip_and_avatar_draw_darkened_layer_within_avatar_circle() {
            test_widgets(|tester| assert_selected_avatar_draws_scrim(tester, false, 16.0, 12.0));
        }

        #[test]
        fn material3_selected_chip_and_avatar_draw_darkened_layer_within_avatar_circle() {
            test_widgets(|tester| assert_selected_avatar_draws_scrim(tester, true, 18.0, 10.0));
        }

        #[test]
        fn chips_should_use_ink_well_instead_of_ink_response() {
            test_widgets(|tester| {
                let theme = crate::tooltip::tests::m3(&tester.cell.borrow());
                tester.pump_widget(crate::test_support::material_app(
                    theme,
                    Material::new()
                        .child(
                            crate::action_chip::ActionChip::new(Text::new("action chip"))
                                .on_pressed(Listener::new(|_| {})),
                        )
                        .into_widget(),
                ));
                expect(
                    tester,
                    &find::by_type::<crate::ink_well::InkWell>(),
                    finds_one_widget(),
                );
            });
        }

        /// Every oval the render object and everything below it draws, in paint order, as Dart's
        /// `paints..circle(...)` records them: centre, radius and color. Nested layers and embedded
        /// display lists are walked and their offset layers' offsets added, which `WidgetTester::painted` does not do.
        fn painted_ovals(
            tester: &WidgetTester,
            render: waft_rendering::AnyRenderObject,
        ) -> Vec<(Offset, f64, u32)> {
            use waft_embedder::valo::Op;
            use waft_rendering::ContainerLayer;

            fn collect(
                app: &waft_foundation::App,
                container: waft_rendering::AnyContainerLayer,
                origin: Offset,
                ovals: &mut Vec<(Offset, f64, u32)>,
            ) {
                let mut child = container.first_child(app);
                while let Some(layer) = child {
                    match layer.as_container_layer() {
                        Some(nested) => {
                            let shift = nested
                                .as_offset_layer()
                                .map_or(Offset::ZERO, |offset_layer| offset_layer.offset(app));
                            collect(app, nested, origin + shift, ovals);
                        }
                        None => {
                            if let Some(picture) = app
                                .handle::<waft_rendering::PictureLayer>(layer.id())
                                .and_then(|picture| picture.picture(app))
                            {
                                for op in waft_embedder::flattened_ops(picture) {
                                    let Op::DrawPath { path, paint, .. } = op else {
                                        continue;
                                    };
                                    if let Some(oval) = path.as_oval() {
                                        let color: Color = paint.color.into();
                                        ovals.push((
                                            origin
                                                + Offset::new(
                                                    f64::from(oval.x + oval.width / 2.0),
                                                    f64::from(oval.y + oval.height / 2.0),
                                                ),
                                            f64::from(oval.width) / 2.0,
                                            color.to_argb32(),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    child = layer.next_sibling(app);
                }
            }

            let mut app = tester.cell.borrow_mut();
            let layer = waft_rendering::OffsetLayer::new(&mut app, Offset::ZERO);
            let container = layer.as_container_layer();
            let mut context =
                waft_rendering::PaintingContext::new(container, render.paint_bounds(&app));
            context.paint_child(&mut app, render, Offset::ZERO);
            context.stop_recording_if_needed(&mut app);
            let mut ovals = Vec::new();
            collect(&app, container, Offset::ZERO, &mut ovals);
            ovals
        }

        #[test]
        fn chip_uses_the_resolved_color_based_on_its_state_for_the_delete_icon_color() {
            test_widgets(|tester| {
                let chip_theme = ChipThemeData::new().delete_icon_color(
                    waft_widgets::WidgetStateColor::resolve_with(|states| {
                        if states.contains(&waft_widgets::WidgetState::Selected) {
                            Colors::GREEN.primary()
                        } else {
                            Colors::BLUE.primary()
                        }
                    }),
                );
                let build_app = |tester: &WidgetTester, is_selected: bool| {
                    let mut chip = InputChip::new(Text::new("Label"))
                        .selected(is_selected)
                        .on_deleted(Listener::new(|_| {}));
                    chip.on_selected = Some(Rc::new(|_app, _selected| {}));
                    wrap_ltr(tester, ChipTheme::new(chip_theme.clone(), chip))
                };

                let root = build_app(tester, false);
                tester.pump_widget(root);
                assert_eq!(
                    icon_data(tester).color.map(|color| color.color()),
                    Some(Colors::BLUE.primary())
                );

                let root = build_app(tester, true);
                tester.pump_widget(root);
                assert_eq!(
                    icon_data(tester).color.map(|color| color.color()),
                    Some(Colors::GREEN.primary())
                );
            });
        }
    }

    mod states_shape_and_density {
        //! chip_test.dart's stateful label colour, border side and shape cases, the cases where a
        //! chip defers to its theme, and the density cases.
        //!
        //! Dart's border side colours (`0x00000001` ... `0x00000007`) are fully transparent: the
        //! border-side cases, which assert those draws, and the paint half of the theme-deferral cases
        //! are deferred (`PORTING.md`), since the display list drops a draw with no effect.
        use waft_widgets::testing::{
            TestPointer, WidgetTester, assert_more_or_less_equals_within, test_widgets,
        };

        use std::rc::Rc;

        use waft_embedder::{Color, Offset, Rect, Size};
        use waft_painting::{
            BeveledRectangleBorder, BorderSide, CircleBorder, ContinuousRectangleBorder,
            OutlinedBorderRef, RoundedRectangleBorder, ShapeBorderRef, StadiumBorder, TextStyle,
        };
        use waft_widgets::{
            AnyFocusNode, Center, Column, Focus, FocusNode, FocusNodeRef, GlobalKey, Icon,
            IntoWidget, Text, WidgetRef, WidgetState, WidgetStateBorderSide, WidgetStateColor,
            WidgetStateOutlinedBorder, WidgetStates,
        };

        use crate::chip::tests::{by_key, material_of_raw_chip, only, wrap_with_theme};
        use crate::choice_chip::ChoiceChip;
        use crate::icons::Icons;
        use crate::input_chip::InputChip;
        use crate::material::Material;
        use crate::scaffold::Scaffold;
        use crate::test_support::material_app;
        use crate::theme_data::{ThemeData, VisualDensity};
        use crate::tooltip::tests::m3;

        const PRESSED_COLOR: Color = Color::new(0x00000001);
        const HOVER_COLOR: Color = Color::new(0x00000002);
        const FOCUSED_COLOR: Color = Color::new(0x00000003);
        const DEFAULT_COLOR: Color = Color::new(0x00000004);
        const SELECTED_COLOR: Color = Color::new(0x00000005);
        const DISABLED_COLOR: Color = Color::new(0x00000006);

        fn material_shape(tester: &WidgetTester) -> Option<ShapeBorderRef> {
            tester.widget(material_of_raw_chip(tester), |material: &Material| {
                material.shape.clone()
            })
        }

        fn is_shape<T: 'static>(shape: &Option<ShapeBorderRef>) -> bool {
            shape
                .as_ref()
                .is_some_and(|shape| shape.as_any().downcast_ref::<T>().is_some())
        }

        fn choice_chip(selected: bool, enabled: bool) -> ChoiceChip {
            let mut chip = ChoiceChip::new(Text::new("Chip"), selected);
            if enabled {
                chip.on_selected = Some(Rc::new(|_app, _value| {}));
            }
            chip
        }

        /// The states Dart's cases walk a `ChoiceChip` through, and the order of them: default,
        /// selected, focused, hovered, pressed, disabled.
        #[derive(Clone, Copy, Debug)]
        enum ChipState {
            Default,
            Selected,
            Focused,
            Hovered,
            Pressed,
            Disabled,
        }

        /// Dart's walk of a chip through its states, `chipWidget` pumped for the default, selected
        /// and disabled ones, a focus request, a mouse hover and a mouse press for the others. The
        /// `check` runs after each.
        fn walk_through_states(
            tester: &mut WidgetTester,
            chip_widget: impl Fn(&WidgetTester, &FocusNodeRef, bool, bool) -> WidgetRef,
            check: impl Fn(&WidgetTester, ChipState),
        ) {
            let focus_node =
                FocusNode::new(&mut tester.cell.borrow_mut()).into_erased::<AnyFocusNode>();

            // Default, not disabled.
            let root = chip_widget(tester, &focus_node, true, false);
            tester.pump_widget(root);
            check(tester, ChipState::Default);

            // Selected.
            let root = chip_widget(tester, &focus_node, true, true);
            tester.pump_widget(root);
            check(tester, ChipState::Selected);

            // Focused.
            let chip_focus_node = focus_node.handle().children(&tester.cell.borrow())[0];
            chip_focus_node.request_focus(&mut tester.cell.borrow_mut(), None);
            tester.pump_and_settle();
            check(tester, ChipState::Focused);

            // Hovered.
            let center = crate::chip::tests::center_of(tester, only::<ChoiceChip>(tester));
            let mut gesture = TestPointer::mouse(1, waft_gestures::K_PRIMARY_BUTTON);
            tester.add_pointer(&mut gesture, Offset::ZERO);
            tester.move_to(&mut gesture, center, false);
            tester.pump_and_settle();
            check(tester, ChipState::Hovered);

            // Pressed.
            tester.down(&mut gesture, center);
            tester.pump_and_settle();
            check(tester, ChipState::Pressed);

            // Disabled.
            let root = chip_widget(tester, &focus_node, false, false);
            tester.pump_widget(root);
            tester.pump_and_settle();
            check(tester, ChipState::Disabled);
        }

        fn focused_scaffold<K>(focus_node: &FocusNodeRef, chip: impl IntoWidget<K>) -> WidgetRef {
            Scaffold::new()
                .body(Focus::new(chip).focus_node(focus_node.clone()))
                .into_widget()
        }

        fn state_color(state: ChipState) -> Color {
            match state {
                ChipState::Default => DEFAULT_COLOR,
                ChipState::Selected => SELECTED_COLOR,
                ChipState::Focused => FOCUSED_COLOR,
                ChipState::Hovered => HOVER_COLOR,
                ChipState::Pressed => PRESSED_COLOR,
                ChipState::Disabled => DISABLED_COLOR,
            }
        }

        fn state_dependent_color(states: &WidgetStates) -> Color {
            if states.contains(&WidgetState::Disabled) {
                DISABLED_COLOR
            } else if states.contains(&WidgetState::Pressed) {
                PRESSED_COLOR
            } else if states.contains(&WidgetState::Hovered) {
                HOVER_COLOR
            } else if states.contains(&WidgetState::Focused) {
                FOCUSED_COLOR
            } else if states.contains(&WidgetState::Selected) {
                SELECTED_COLOR
            } else {
                DEFAULT_COLOR
            }
        }

        #[test]
        fn chip_uses_stateful_color_for_text_color_in_different_states() {
            test_widgets(|tester| {
                walk_through_states(
                    tester,
                    |tester, focus_node, enabled, selected| {
                        let mut chip = choice_chip(selected, enabled);
                        chip.label_style = Some(
                            TextStyle::new()
                                .color(WidgetStateColor::resolve_with(state_dependent_color)),
                        );
                        material_app(
                            m3(&tester.cell.borrow()),
                            focused_scaffold(focus_node, chip),
                        )
                    },
                    |tester, state| assert_eq!(tester.text_color("Chip"), state_color(state)),
                );
            });
        }

        fn shape_of_state(states: &WidgetStates) -> Option<OutlinedBorderRef> {
            if states.contains(&WidgetState::Disabled) {
                Some(OutlinedBorderRef::new(BeveledRectangleBorder::default()))
            } else if states.contains(&WidgetState::Pressed) {
                Some(OutlinedBorderRef::new(CircleBorder::default()))
            } else if states.contains(&WidgetState::Hovered) {
                Some(OutlinedBorderRef::new(ContinuousRectangleBorder::default()))
            } else if states.contains(&WidgetState::Focused) {
                Some(OutlinedBorderRef::new(RoundedRectangleBorder::default()))
            } else if states.contains(&WidgetState::Selected) {
                Some(OutlinedBorderRef::new(BeveledRectangleBorder::default()))
            } else {
                None
            }
        }

        fn stateful_shape_chip_widget(
            tester: &WidgetTester,
            focus_node: &FocusNodeRef,
            enabled: bool,
            selected: bool,
            material3: bool,
        ) -> WidgetRef {
            let mut chip = choice_chip(selected, enabled);
            chip.shape = Some(WidgetStateOutlinedBorder::resolve_with(shape_of_state).into());
            let theme = if material3 {
                m3(&tester.cell.borrow())
            } else {
                crate::tooltip::tests::m2(&tester.cell.borrow())
            };
            material_app(theme, focused_scaffold(focus_node, chip))
        }

        fn check_stateful_shape(tester: &WidgetTester, state: ChipState, default_is_stadium: bool) {
            let shape = material_shape(tester);
            match state {
                // Defers to the default shape.
                ChipState::Default if default_is_stadium => {
                    assert!(is_shape::<StadiumBorder>(&shape))
                }
                ChipState::Default => assert!(is_shape::<RoundedRectangleBorder>(&shape)),
                ChipState::Selected | ChipState::Disabled => {
                    assert!(is_shape::<BeveledRectangleBorder>(&shape));
                }
                ChipState::Focused => assert!(is_shape::<RoundedRectangleBorder>(&shape)),
                ChipState::Hovered => assert!(is_shape::<ContinuousRectangleBorder>(&shape)),
                ChipState::Pressed => assert!(is_shape::<CircleBorder>(&shape)),
            }
        }

        #[test]
        fn material2_chip_uses_stateful_shape_in_different_states() {
            test_widgets(|tester| {
                walk_through_states(
                    tester,
                    |tester, focus_node, enabled, selected| {
                        stateful_shape_chip_widget(tester, focus_node, enabled, selected, false)
                    },
                    |tester, state| check_stateful_shape(tester, state, true),
                );
            });
        }

        #[test]
        fn material3_chip_uses_stateful_shape_in_different_states() {
            test_widgets(|tester| {
                walk_through_states(
                    tester,
                    |tester, focus_node, enabled, selected| {
                        stateful_shape_chip_widget(tester, focus_node, enabled, selected, true)
                    },
                    |tester, state| check_stateful_shape(tester, state, false),
                );
            });
        }

        fn selected_only_shape(states: &WidgetStates) -> Option<OutlinedBorderRef> {
            states
                .contains(&WidgetState::Selected)
                .then(|| OutlinedBorderRef::new(RoundedRectangleBorder::default()))
        }

        fn selected_only_border_side(states: &WidgetStates) -> Option<BorderSide> {
            states.contains(&WidgetState::Selected).then(|| BorderSide {
                color: HOVER_COLOR,
                ..BorderSide::default()
            })
        }

        fn pump_chip_deferring_to_theme(
            tester: &mut WidgetTester,
            material3: bool,
            selected: bool,
        ) {
            let chip_theme = ThemeData::new(&tester.cell.borrow())
                .chip_theme
                .copy_with()
                .shape(StadiumBorder::new(BorderSide::NONE))
                .side(BorderSide {
                    color: PRESSED_COLOR,
                    ..BorderSide::default()
                });
            let theme = ThemeData::builder()
                .use_material3(material3)
                .chip_theme(chip_theme)
                .build(&tester.cell.borrow());
            let mut chip = choice_chip(selected, true);
            chip.shape = Some(WidgetStateOutlinedBorder::resolve_with(selected_only_shape).into());
            chip.side = Some(WidgetStateBorderSide::resolve_with(selected_only_border_side).into());
            tester.pump_widget(material_app(
                theme,
                Scaffold::new().body(chip).into_widget(),
            ));
        }

        fn check_chip_defers_to_theme(material3: bool) {
            test_widgets(|tester| {
                // Default, not disabled. Defer to theme.
                pump_chip_deferring_to_theme(tester, material3, false);
                assert!(is_shape::<StadiumBorder>(&material_shape(tester)));

                // Selected.
                pump_chip_deferring_to_theme(tester, material3, true);
                assert!(is_shape::<RoundedRectangleBorder>(&material_shape(tester)));
            });
        }

        #[test]
        fn material2_chip_defers_to_theme_if_shape_and_side_resolves_to_null() {
            check_chip_defers_to_theme(false);
        }

        #[test]
        fn chip_defers_to_theme_if_shape_and_side_resolves_to_null() {
            check_chip_defers_to_theme(true);
        }

        fn rect_close(actual: f64, expected: f64) {
            assert_more_or_less_equals_within(actual, expected, 0.1);
        }

        struct DensityKeys {
            chip: GlobalKey,
            text: GlobalKey,
            icon: GlobalKey,
            avatar: GlobalKey,
        }

        impl DensityKeys {
            fn new() -> DensityKeys {
                DensityKeys {
                    chip: GlobalKey::new(),
                    text: GlobalKey::new(),
                    icon: GlobalKey::new(),
                    avatar: GlobalKey::new(),
                }
            }

            /// Dart's `buildTest(visualDensity)`.
            fn pump(
                &self,
                tester: &mut WidgetTester,
                theme: ThemeData,
                visual_density: VisualDensity,
            ) {
                let chip = InputChip::new(Text::new("Test").key(self.text.clone().into()))
                    .key(self.chip.clone().into())
                    .visual_density(visual_density)
                    .on_pressed(waft_foundation::Listener::new(|_app| {}))
                    .on_deleted(waft_foundation::Listener::new(|_app| {}))
                    .delete_icon(Icon::new(Some(Icons::delete())).key(self.icon.clone().into()))
                    .avatar(Icon::new(Some(Icons::play_arrow())).key(self.avatar.clone().into()));
                let root = wrap_with_theme(
                    tester,
                    theme,
                    Center::new().child(Column::new().children([chip.into_widget()])),
                );
                tester.pump_widget(root);
            }

            fn rects(&self, tester: &WidgetTester) -> DensityRects {
                let rect = |key: &GlobalKey| tester.get_rect(by_key(tester, key));
                DensityRects {
                    chip: rect(&self.chip),
                    text: rect(&self.text),
                    icon: rect(&self.icon),
                    avatar: rect(&self.avatar),
                }
            }
        }

        struct DensityRects {
            chip: Rect,
            text: Rect,
            icon: Rect,
            avatar: Rect,
        }

        impl DensityRects {
            fn assert_icon_sizes(&self) {
                assert_eq!(self.icon.size(), Size::new(18.0, 18.0));
                assert_eq!(self.avatar.size(), Size::new(18.0, 18.0));
            }
        }

        #[test]
        fn material2_chip_responds_to_density_changes() {
            test_widgets(|tester| {
                let keys = DensityKeys::new();
                let pump = |tester: &mut WidgetTester, density: VisualDensity| {
                    let theme = crate::tooltip::tests::m2(&tester.cell.borrow());
                    keys.pump(tester, theme, density);
                };
                // (density, chip height, text top and bottom gap)
                let cases = [
                    (VisualDensity::STANDARD, 32.0 + 16.0, 17.0),
                    (VisualDensity::new(3.0, 3.0), 60.0, 23.0),
                    (VisualDensity::new(-3.0, -3.0), 36.0, 11.0),
                ];
                // The Chips only change in size vertically in response to density, so horizontal
                // changes aren't expected.
                for (density, height, text_gap) in cases {
                    pump(tester, density);
                    let rects = keys.rects(tester);
                    assert_eq!(rects.chip.size(), Size::new(128.0, height));
                    assert_eq!(rects.text.size(), Size::new(56.0, 14.0));
                    rects.assert_icon_sizes();
                    assert_eq!(rects.text.top, text_gap);
                    assert_eq!(rects.chip.bottom - rects.text.bottom, text_gap);
                    assert_eq!(rects.text.left, 372.0);
                    assert_eq!(rects.chip.right - rects.text.right, 36.0);
                }

                // Now test that horizontal and vertical are wired correctly. Negating the horizontal
                // should have no change over what's above.
                pump(tester, VisualDensity::new(3.0, -3.0));
                tester.pump_and_settle();
                let rects = keys.rects(tester);
                assert_eq!(rects.chip.size(), Size::new(128.0, 36.0));
                assert_eq!(rects.text.size(), Size::new(56.0, 14.0));
                rects.assert_icon_sizes();
                assert_eq!(rects.text.top, 11.0);
                assert_eq!(rects.chip.bottom - rects.text.bottom, 11.0);
                assert_eq!(rects.text.left, 372.0);
                assert_eq!(rects.chip.right - rects.text.right, 36.0);

                // Make sure the "Comfortable" setting is the spec'd size.
                pump(tester, VisualDensity::COMFORTABLE);
                tester.pump_and_settle();
                assert_eq!(
                    keys.rects(tester).chip.size(),
                    Size::new(128.0, 28.0 + 16.0)
                );

                // Make sure the "Compact" setting is the spec'd size.
                pump(tester, VisualDensity::COMPACT);
                tester.pump_and_settle();
                assert_eq!(
                    keys.rects(tester).chip.size(),
                    Size::new(128.0, 24.0 + 16.0)
                );
            });
        }

        #[test]
        fn material3_chip_responds_to_density_changes() {
            test_widgets(|tester| {
                let keys = DensityKeys::new();
                let pump = |tester: &mut WidgetTester, density: VisualDensity| {
                    let theme = m3(&tester.cell.borrow());
                    keys.pump(tester, theme, density);
                };
                let check = |rects: &DensityRects, height: f64, text_gap: f64| {
                    rect_close(rects.chip.size().width(), 130.4);
                    assert_eq!(rects.chip.size().height(), height);
                    rect_close(rects.text.size().width(), 56.4);
                    assert_eq!(rects.text.size().height(), 20.0);
                    rects.assert_icon_sizes();
                    assert_eq!(rects.text.top, text_gap);
                    assert_eq!(rects.chip.bottom - rects.text.bottom, text_gap);
                    rect_close(rects.text.left, 371.79);
                    assert_eq!(rects.chip.right - rects.text.right, 37.0);
                };

                // The Chips only change in size vertically in response to density, so horizontal
                // changes aren't expected.
                pump(tester, VisualDensity::STANDARD);
                check(&keys.rects(tester), 32.0 + 16.0, 14.0);

                // Try decreasing density (with higher density numbers).
                pump(tester, VisualDensity::new(3.0, 3.0));
                check(&keys.rects(tester), 60.0, 20.0);

                // Try increasing density (with lower density numbers).
                pump(tester, VisualDensity::new(-3.0, -3.0));
                check(&keys.rects(tester), 36.0, 8.0);

                // Now test that horizontal and vertical are wired correctly. Negating the horizontal
                // should have no change over what's above.
                pump(tester, VisualDensity::new(3.0, -3.0));
                tester.pump_and_settle();
                check(&keys.rects(tester), 36.0, 8.0);

                // Make sure the "Comfortable" setting is the spec'd size.
                pump(tester, VisualDensity::COMFORTABLE);
                tester.pump_and_settle();
                let chip = keys.rects(tester).chip;
                rect_close(chip.size().width(), 130.4);
                assert_eq!(chip.size().height(), 28.0 + 16.0);

                // Make sure the "Compact" setting is the spec'd size.
                pump(tester, VisualDensity::COMPACT);
                tester.pump_and_settle();
                let chip = keys.rects(tester).chip;
                rect_close(chip.size().width(), 130.4);
                assert_eq!(chip.size().height(), 24.0 + 16.0);
            });
        }
    }

    mod delete_button_and_colors {
        //! chip_test.dart (lines 4747-5561): delete-button tooltips, the Ink background, RawChip colour
        //! states, shape and side, the icon theme, layout constraints of the avatar and delete icon,
        //! and the default delete button's InkWell and overlay.
        use waft_widgets::testing::{TestPointer, WidgetTester, find, test_widgets};

        use std::cell::Cell;
        use std::rc::Rc;

        use waft_embedder::{Color, Offset, Rect, Size};
        use waft_foundation::Listener;
        use waft_painting::{
            BorderRadiusGeometry, BorderSide, CircleBorder, ContinuousRectangleBorder, EdgeInsets,
            RoundedRectangleBorder, ShapeBorderRef, ShapeDecoration, StadiumBorder,
        };
        use waft_rendering::{BoxConstraints, RenderBox};
        use waft_widgets::{
            Center, Column, Container, FocusHighlightStrategy, FocusNode, FocusNodeLeaf, GlobalKey,
            Icon, IconThemeData, IntoWidget, SizedBox, Text, WidgetState, WidgetStateProperty,
            Wrap,
        };

        use crate::chip::tests::{
            chip_material_of, icon_data, material_box_paints, material_of_raw_chip, only, text,
            wrap_ltr, wrap_with_theme,
        };
        use crate::chip::{Chip, RawChip};
        use crate::filter_chip::tests::tooltip_container_count;
        use crate::icons::Icons;
        use crate::ink_decoration::Ink;
        use crate::material::Material;
        use crate::test_support::{PaintInspection, material_app};

        use crate::theme_data::ThemeData;
        use crate::tooltip::tests::{m2, m3};
        use waft_rendering::testing::{PaintCall, assert_paints, paints};

        /// Dart's `chipWithOptionalDeleteButton`.
        fn pump_chip_with_optional_delete_button(
            tester: &mut WidgetTester,
            delete_button_key: &GlobalKey,
            deletable: bool,
            chip_tooltip: Option<&str>,
            delete_button_tooltip_message: Option<&str>,
        ) {
            let mut chip = RawChip::new(Text::new(if deletable {
                "Chip with Delete Button"
            } else {
                "Chip without Delete Button"
            }))
            .on_pressed(Listener::new(|_| {}))
            .delete_icon(Icon::new(Some(Icons::close())).key(delete_button_key.clone().into()));
            if deletable {
                chip = chip.on_deleted(Listener::new(|_| {}));
            }
            if let Some(tooltip) = chip_tooltip {
                chip = chip.tooltip(tooltip.to_owned());
            }
            if let Some(message) = delete_button_tooltip_message {
                chip = chip.delete_button_tooltip_message(message.to_owned());
            }
            let root = wrap_ltr(tester, Wrap::new().children([chip.into_widget()]));
            tester.pump_widget(root);
        }

        fn hover_over(tester: &mut WidgetTester, at: Offset) {
            let mut mouse = TestPointer::mouse(1, 0);
            tester.move_to(&mut mouse, at, false);
            tester.pump();
            tester.pump_and_settle();
        }

        fn drawn_background(tester: &WidgetTester, color: Color) -> bool {
            material_box_paints(tester, &[PaintCall::rrect(Some(color))])
        }

        #[test]
        fn chip_delete_button_tooltip_is_disabled_if_delete_button_tooltip_message_is_empty() {
            test_widgets(|tester| {
                let delete_button_key = GlobalKey::new();
                pump_chip_with_optional_delete_button(
                    tester,
                    &delete_button_key,
                    true,
                    None,
                    Some(""),
                );

                // Hover over the delete icon of the chip
                let center_of_delete_button =
                    tester.get_center(crate::chip::tests::by_key(tester, &delete_button_key));
                hover_over(tester, center_of_delete_button);

                // There should be no delete button tooltip
                assert_eq!(tooltip_container_count(tester, ""), 0);
            });
        }

        #[test]
        fn disabling_delete_button_tooltip_does_not_disable_chip_tooltip() {
            test_widgets(|tester| {
                let delete_button_key = GlobalKey::new();
                pump_chip_with_optional_delete_button(
                    tester,
                    &delete_button_key,
                    true,
                    Some("Chip Tooltip"),
                    Some(""),
                );

                let center_of_delete_button =
                    tester.get_center(crate::chip::tests::by_key(tester, &delete_button_key));
                hover_over(tester, center_of_delete_button);

                // There should be no delete button tooltip
                assert_eq!(tooltip_container_count(tester, ""), 0);
                // There should be a chip tooltip, however.
                assert_eq!(tooltip_container_count(tester, "Chip Tooltip"), 1);
            });
        }

        #[test]
        fn triggering_delete_button_tooltip_does_not_trigger_chip_tooltip() {
            test_widgets(|tester| {
                let delete_button_key = GlobalKey::new();
                pump_chip_with_optional_delete_button(
                    tester,
                    &delete_button_key,
                    true,
                    Some("Chip Tooltip"),
                    None,
                );

                let center_of_delete_button =
                    tester.get_center(crate::chip::tests::by_key(tester, &delete_button_key));
                hover_over(tester, center_of_delete_button);

                // There should not be a chip tooltip
                assert_eq!(tooltip_container_count(tester, "Chip Tooltip"), 0);
                // There should be a delete button tooltip
                assert_eq!(tooltip_container_count(tester, "Delete"), 1);
            });
        }

        #[test]
        fn intrinsic_height_implementation_meets_constraints() {
            test_widgets(|tester| {
                // Regression test for https://github.com/flutter/flutter/issues/49478.
                let root = wrap_ltr(
                    tester,
                    Chip::new(Text::new("text")).padding(EdgeInsets::symmetric(0.0, 20.0)),
                );
                tester.pump_widget(root);
            });
        }

        fn ink_decoration(tester: &WidgetTester) -> ShapeDecoration {
            let ink = tester.descendants_of_type::<Ink>(only::<RawChip>(tester))[0];
            tester.widget(ink, |ink: &Ink| {
                ink.decoration
                    .as_deref()
                    .and_then(|decoration| decoration.as_any().downcast_ref::<ShapeDecoration>())
                    .expect("a ShapeDecoration")
                    .clone()
            })
        }

        #[test]
        fn material2_chip_background_color_and_shape_are_drawn_on_ink() {
            test_widgets(|tester| {
                let background_color = Color::new(0xff00ff00);
                let shape = ContinuousRectangleBorder::default();
                let root = wrap_with_theme(
                    tester,
                    m2(&tester.cell.borrow()),
                    RawChip::new(Text::new("text"))
                        .background_color(background_color)
                        .shape(shape),
                );
                tester.pump_widget(root);

                let decoration = ink_decoration(tester);
                assert_eq!(
                    decoration.color.map(|color| color.color()),
                    Some(background_color)
                );
                assert_eq!(decoration.shape, ShapeBorderRef::new(shape));
            });
        }

        #[test]
        fn material3_chip_background_color_and_shape_are_drawn_on_ink() {
            test_widgets(|tester| {
                let background_color = Color::new(0xff00ff00);
                let shape = ContinuousRectangleBorder::default();
                let theme = m3(&tester.cell.borrow());
                let root = wrap_with_theme(
                    tester,
                    theme.clone(),
                    RawChip::new(Text::new("text"))
                        .background_color(background_color)
                        .shape(shape),
                );
                tester.pump_widget(root);

                let decoration = ink_decoration(tester);
                assert_eq!(
                    decoration.color.map(|color| color.color()),
                    Some(background_color)
                );
                assert_eq!(
                    decoration.shape,
                    ShapeBorderRef::new(shape.copy_with(
                        Some(BorderSide {
                            color: theme.color_scheme.outline_variant(),
                            ..BorderSide::default()
                        }),
                        None,
                    ))
                );
            });
        }

        #[test]
        fn chip_highlight_color_is_drawn_on_top_of_the_background_color() {
            test_widgets(|tester| {
                tester.set_highlight_strategy(FocusHighlightStrategy::AlwaysTraditional);
                let focus_node = FocusNode::new(&mut tester.cell.borrow_mut());
                let background_color = Color::new(0xff00ff00);
                let root = wrap_ltr(
                    tester,
                    RawChip::new(Text::new("text"))
                        .background_color(background_color)
                        .autofocus(true)
                        .focus_node(focus_node.erase())
                        .on_pressed(Listener::new(|_| {})),
                );
                tester.pump_widget(root);
                tester.pump_and_settle();

                assert!(focus_node.has_primary_focus(&tester.cell.borrow()));
                // Dart reads `find.byType(Material).last`, whose own fill is the physical shape's
                // `drawPath`, which `..rrect` skips. Valo records that fill as a rounded rect, so
                // this reads what the Material paints inside it: its ink features and their child.
                let ink_features = *tester
                    .all::<crate::material::RenderInkFeatures>()
                    .last()
                    .expect("the chip's Material");
                let ops = tester.painted_ops_deep(ink_features.as_object());
                assert_paints(
                    &ops,
                    &[
                        // Background color is drawn first.
                        PaintCall::rrect(Some(background_color)),
                        // Highlight color is drawn on top of the background color.
                        PaintCall::rect(Some(Color::new(0x1f000000))),
                    ],
                );
            });
        }

        #[test]
        fn raw_chip_color_resolves_material_states() {
            test_widgets(|tester| {
                let disabled_selected_color = Color::new(0xffffff00);
                let disabled_color = Color::new(0xff00ff00);
                let background_color = Color::new(0xff0000ff);
                let selected_color = Color::new(0xffff0000);
                let build_app = |tester: &mut WidgetTester, enabled: bool, selected: bool| {
                    let color =
                        <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
                            if states.contains(&WidgetState::Disabled)
                                && states.contains(&WidgetState::Selected)
                            {
                                return Some(disabled_selected_color);
                            }
                            if states.contains(&WidgetState::Disabled) {
                                return Some(disabled_color);
                            }
                            if states.contains(&WidgetState::Selected) {
                                return Some(selected_color);
                            }
                            Some(background_color)
                        });
                    let root = wrap_ltr(
                        tester,
                        RawChip::new(Text::new("RawChip"))
                            .is_enabled(enabled)
                            .selected(selected)
                            .color(color),
                    );
                    tester.pump_widget(root);
                };

                // Test enabled chip.
                build_app(tester, true, false);
                // Enabled chip should have the provided backgroundColor.
                assert!(drawn_background(tester, background_color));

                // Test disabled chip.
                build_app(tester, false, false);
                tester.pump_and_settle();
                // Disabled chip should have the provided disabledColor.
                assert!(drawn_background(tester, disabled_color));

                // Test enabled & selected chip.
                build_app(tester, true, true);
                tester.pump_and_settle();
                // Enabled & selected chip should have the provided selectedColor.
                assert!(drawn_background(tester, selected_color));

                // Test disabled & selected chip.
                build_app(tester, false, true);
                tester.pump_and_settle();
                // Disabled & selected chip should have the provided disabledSelectedColor.
                assert!(drawn_background(tester, disabled_selected_color));
            });
        }

        #[test]
        fn raw_chip_uses_provided_state_color_properties() {
            test_widgets(|tester| {
                let disabled_color = Color::new(0xff00ff00);
                let background_color = Color::new(0xff0000ff);
                let selected_color = Color::new(0xffff0000);
                let build_app = |tester: &mut WidgetTester, enabled: bool, selected: bool| {
                    let root = wrap_ltr(
                        tester,
                        RawChip::new(Text::new("RawChip"))
                            .is_enabled(enabled)
                            .selected(selected)
                            .disabled_color(disabled_color)
                            .background_color(background_color)
                            .selected_color(selected_color),
                    );
                    tester.pump_widget(root);
                };

                // Test enabled chip.
                build_app(tester, true, false);
                // Enabled chip should have the provided backgroundColor.
                assert!(drawn_background(tester, background_color));

                // Test disabled chip.
                build_app(tester, false, false);
                tester.pump_and_settle();
                // Disabled chip should have the provided disabledColor.
                assert!(drawn_background(tester, disabled_color));

                // Test enabled & selected chip.
                build_app(tester, true, true);
                tester.pump_and_settle();
                // Enabled & selected chip should have the provided selectedColor.
                assert!(drawn_background(tester, selected_color));
            });
        }

        #[test]
        fn delete_button_tap_target_area_does_not_include_label() {
            test_widgets(|tester| {
                let called_delete = Rc::new(Cell::new(false));
                let root = wrap_ltr(
                    tester,
                    Column::new().children([Chip::new(Text::new("Chip"))
                        .on_deleted({
                            let called_delete = Rc::clone(&called_delete);
                            Listener::new(move |_| called_delete.set(true))
                        })
                        .into_widget()]),
                );
                tester.pump_widget(root);

                // Tap on the delete button.
                let icon = tester.get_center(only::<Icon>(tester));
                tester.tap_at(icon, 1);
                tester.pump();
                assert!(called_delete.get());
                called_delete.set(false);

                let label = text(tester, "Chip");
                let label_center = tester.get_center(label);

                // Tap on the label.
                tester.tap_at(label_center, 1);
                tester.pump();
                assert!(!called_delete.get());

                // Tap before end of the label.
                let label_size = tester.get_size(label);
                tester.tap_at(
                    Offset::new(
                        label_center.dx() + (label_size.width() / 2.0) - 1.0,
                        label_center.dy(),
                    ),
                    1,
                );
                tester.pump();
                assert!(!called_delete.get());

                // Tap after end of the label.
                tester.tap_at(
                    Offset::new(
                        label_center.dx() + (label_size.width() / 2.0) + 0.01,
                        label_center.dy(),
                    ),
                    1,
                );
                tester.pump();
                assert!(called_delete.get());
            });
        }

        fn pump_material_app<K>(
            tester: &mut WidgetTester,
            theme: ThemeData,
            child: impl IntoWidget<K>,
        ) {
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(Center::new().child(child))
                    .into_widget(),
            ));
        }

        fn raw_chip_material_shape(tester: &WidgetTester) -> Option<ShapeBorderRef> {
            chip_material_of(tester, only::<RawChip>(tester)).3
        }

        #[test]
        fn material3_custom_shape_without_provided_side_uses_default_side() {
            // This is a regression test for https://github.com/flutter/flutter/pull/133615.
            test_widgets(|tester| {
                let theme = m3(&tester.cell.borrow());
                pump_material_app(
                    tester,
                    theme.clone(),
                    // No side provided.
                    RawChip::new(Text::new("RawChip")).shape(StadiumBorder::new(BorderSide::NONE)),
                );

                // Chip should have the default side.
                assert_eq!(
                    raw_chip_material_shape(tester),
                    Some(ShapeBorderRef::new(StadiumBorder::new(BorderSide {
                        color: theme.color_scheme.outline_variant(),
                        ..BorderSide::default()
                    })))
                );
            });
        }

        #[test]
        fn material3_raw_chip_shapes_side_is_used_when_provided() {
            test_widgets(|tester| {
                let theme = m3(&tester.cell.borrow());
                let shape_with = |color: u32| {
                    RoundedRectangleBorder::new(
                        BorderSide {
                            color: Color::new(color),
                            ..BorderSide::default()
                        },
                        BorderRadiusGeometry::circular(7.0),
                    )
                };
                let build_chip = |tester: &mut WidgetTester, side: Option<BorderSide>| {
                    let mut chip = RawChip::new(Text::new("RawChip")).shape(shape_with(0xffff00ff));
                    if let Some(side) = side {
                        chip = chip.side(side);
                    }
                    pump_material_app(tester, theme.clone(), chip);
                };

                // Test [RawChip.shape] with a side.
                build_chip(tester, None);

                // Chip should have the provided shape and the side from [RawChip.shape].
                assert_eq!(
                    raw_chip_material_shape(tester),
                    Some(ShapeBorderRef::new(shape_with(0xffff00ff)))
                );

                // Test [RawChip.shape] with a side and [RawChip.side].
                build_chip(
                    tester,
                    Some(BorderSide {
                        color: Color::new(0xfffff000),
                        ..BorderSide::default()
                    }),
                );
                tester.pump_and_settle();

                // Chip use shape from [RawChip.shape] and the side from [RawChip.side].
                // [RawChip.shape]'s side should be ignored.
                assert_eq!(
                    raw_chip_material_shape(tester),
                    Some(ShapeBorderRef::new(shape_with(0xfffff000)))
                );
            });
        }

        #[test]
        fn material3_chip_icon_theme_respects_default_icon_theme_size() {
            test_widgets(|tester| {
                let theme = m3(&tester.cell.borrow());
                let build_chip = |tester: &mut WidgetTester, icon_theme: IconThemeData| {
                    pump_material_app(
                        tester,
                        theme.clone(),
                        RawChip::new(SizedBox::new().width(100.0).height(100.0))
                            .icon_theme(icon_theme)
                            .avatar(Icon::new(Some(Icons::add())))
                            .on_selected(Rc::new(|_app, _selected| {})),
                    );
                };

                build_chip(tester, IconThemeData::new().color(Color::new(0xff332211)));

                // Icon should have the default chip iconSize.
                assert_eq!(icon_data(tester).size, Some(18.0));
                assert_eq!(
                    icon_data(tester).color.map(|color| color.color()),
                    Some(Color::new(0xff332211))
                );

                // Icon should have the provided iconSize.
                build_chip(
                    tester,
                    IconThemeData::new()
                        .color(Color::new(0xff112233))
                        .size(23.0),
                );
                tester.pump_and_settle();

                assert_eq!(icon_data(tester).size, Some(23.0));
                assert_eq!(
                    icon_data(tester).color.map(|color| color.color()),
                    Some(Color::new(0xff112233))
                );
            });
        }

        #[test]
        fn enabling_and_disabling_chip_with_tooltip_does_not_throw_an_exception() {
            // This is a regression test for https://github.com/flutter/flutter/issues/138287.
            test_widgets(|tester| {
                let is_enabled = Rc::new(Cell::new(true));
                let theme = m3(&tester.cell.borrow());
                let enabled_for_build = Rc::clone(&is_enabled);
                pump_material_app(
                    tester,
                    theme,
                    waft_widgets::StatefulBuilder::new(move |_app, _context, set_state| {
                        let enabled = enabled_for_build.get();
                        let toggle = {
                            let is_enabled = Rc::clone(&enabled_for_build);
                            let set_state = set_state.clone();
                            Listener::new(move |app| {
                                let is_enabled = Rc::clone(&is_enabled);
                                set_state(app, &mut move |_app| is_enabled.set(!is_enabled.get()));
                            })
                        };
                        let mut chip = RawChip::new(Text::new("RawChip"))
                            .tooltip("tooltip".to_owned())
                            .is_enabled(enabled);
                        if enabled {
                            chip = chip.on_pressed(Listener::new(|_| {}));
                        }
                        Column::new()
                            .main_axis_size(waft_widgets::MainAxisSize::Min)
                            .children([
                                chip.into_widget(),
                                crate::elevated_button::ElevatedButton::new(
                                    Some(toggle),
                                    Some(Text::new(if enabled {
                                        "Disable Chip"
                                    } else {
                                        "Enable Chip"
                                    })),
                                )
                                .into_widget(),
                            ])
                            .into_widget()
                    }),
                );

                // Tap the elevated button to disable the chip with a tooltip.
                let button = tester.get_center(text(tester, "Disable Chip"));
                tester.tap_at(button, 1);
                tester.pump_and_settle();
                assert!(!is_enabled.get());

                // Tap the elevated button to enable the chip with a tooltip.
                let button = tester.get_center(text(tester, "Enable Chip"));
                tester.tap_at(button, 1);
                tester.pump_and_settle();
                assert!(is_enabled.get());
            });
        }

        #[test]
        fn delete_button_tooltip_is_not_shown_on_disabled_raw_chip() {
            test_widgets(|tester| {
                let build_chip = |tester: &mut WidgetTester, enabled: bool| {
                    let root = wrap_ltr(
                        tester,
                        RawChip::new(Text::new("Label"))
                            .is_enabled(enabled)
                            .on_deleted(Listener::new(|_| {})),
                    );
                    tester.pump_widget(root);
                };

                // Test enabled chip.
                build_chip(tester, true);

                let delete_button_location = tester.get_center(only::<Icon>(tester));
                let mut gesture = TestPointer::mouse(1, 0);
                tester.move_to(&mut gesture, delete_button_location, false);
                tester.pump();

                // Delete button tooltip should be visible.
                assert_eq!(tooltip_container_count(tester, "Delete"), 1);

                // Test disabled chip.
                build_chip(tester, false);
                tester.pump();

                // Delete button tooltip should not be visible.
                assert_eq!(tooltip_container_count(tester, "Delete"), 0);
            });
        }

        const BORDER: f64 = 1.0;
        const ICON_SIZE: f64 = 18.0;
        const LABEL_PADDING: f64 = 8.0;
        const PADDING: f64 = 8.0;
        const LABEL_SIZE: Size = Size::new(100.0, 100.0);

        fn label_container() -> Container {
            Container::new()
                .width(LABEL_SIZE.width())
                .height(LABEL_SIZE.width())
                .color(Color::new(0xFFFF0000))
        }

        fn chip_material_rect(tester: &WidgetTester) -> Rect {
            tester.get_rect(material_of_raw_chip(tester))
        }

        /// The body of 'Chip avatar layout constraints can be customized' and its RawChip twin: the
        /// chip `W` is built by `build_chip` with the optional avatar box constraints.
        fn check_avatar_layout_constraints_can_be_customized<W: 'static>(
            build_chip: fn(Option<BoxConstraints>) -> waft_widgets::WidgetRef,
        ) {
            test_widgets(|tester| {
                let pump = |tester: &mut WidgetTester, constraints: Option<BoxConstraints>| {
                    let root = wrap_ltr(tester, Center::new().child(build_chip(constraints)));
                    tester.pump_widget(root);
                };
                let chip_size = |tester: &WidgetTester| tester.get_size(only::<W>(tester));

                // Test default avatar layout constraints.
                pump(tester, None);

                assert_eq!(chip_size(tester).width(), 234.0);
                assert_eq!(chip_size(tester).height(), 118.0);

                // Calculate the distance between avatar and chip edges.
                let chip_top_left = chip_material_rect(tester).top_left();
                let avatar_center =
                    tester.get_center(tester.element(find::by_icon(&Icons::favorite()).first()));
                assert_eq!(
                    chip_top_left.dx(),
                    avatar_center.dx() - (LABEL_SIZE.width() / 2.0) - PADDING - BORDER
                );
                assert_eq!(
                    chip_top_left.dy(),
                    avatar_center.dy() - (LABEL_SIZE.width() / 2.0) - PADDING - BORDER
                );

                // Calculate the distance between avatar and label.
                let label_top_left = tester.get_rect(only::<Container>(tester)).top_left();
                assert_eq!(
                    label_top_left.dx(),
                    avatar_center.dx() + (LABEL_SIZE.width() / 2.0) + LABEL_PADDING
                );

                // Test custom avatar layout constraints.
                pump(
                    tester,
                    Some(BoxConstraints::tight_for_finite(
                        f64::INFINITY,
                        f64::INFINITY,
                    )),
                );
                tester.pump();

                assert_eq!(chip_size(tester).width(), 152.0);
                assert_eq!(chip_size(tester).height(), 118.0);

                // Calculate the distance between avatar and chip edges.
                let chip_top_left = chip_material_rect(tester).top_left();
                assert_eq!(
                    chip_top_left.dx(),
                    avatar_center.dx() - (ICON_SIZE / 2.0) - PADDING - BORDER
                );
                assert_eq!(
                    chip_top_left.dy(),
                    avatar_center.dy() - (LABEL_SIZE.width() / 2.0) - PADDING - BORDER
                );

                // Calculate the distance between avatar and label.
                let label_top_left = tester.get_rect(only::<Container>(tester)).top_left();
                assert_eq!(
                    label_top_left.dx(),
                    avatar_center.dx() + (ICON_SIZE / 2.0) + LABEL_PADDING
                );
            });
        }

        /// The body of 'Chip delete icon layout constraints can be customized' and its RawChip twin.
        fn check_delete_icon_layout_constraints_can_be_customized<W: 'static>(
            build_chip: fn(Option<BoxConstraints>) -> waft_widgets::WidgetRef,
        ) {
            test_widgets(|tester| {
                let pump = |tester: &mut WidgetTester, constraints: Option<BoxConstraints>| {
                    let root = wrap_ltr(tester, Center::new().child(build_chip(constraints)));
                    tester.pump_widget(root);
                };
                let chip_size = |tester: &WidgetTester| tester.get_size(only::<W>(tester));

                // Test default delete icon layout constraints.
                pump(tester, None);

                assert_eq!(chip_size(tester).width(), 234.0);
                assert_eq!(chip_size(tester).height(), 118.0);

                // Calculate the distance between delete icon and chip edges.
                let chip_top_right = chip_material_rect(tester).top_right();
                let delete_icon_center =
                    tester.get_center(tester.element(find::by_icon(&Icons::cancel()).first()));
                assert_eq!(
                    chip_top_right.dx(),
                    delete_icon_center.dx() + (LABEL_SIZE.width() / 2.0) + PADDING + BORDER
                );
                assert_eq!(
                    chip_top_right.dy(),
                    delete_icon_center.dy() - (LABEL_SIZE.width() / 2.0) - PADDING - BORDER
                );

                // Calculate the distance between delete icon and label.
                let label_top_right = tester.get_rect(only::<Container>(tester)).top_right();
                assert_eq!(
                    label_top_right.dx(),
                    delete_icon_center.dx() - (LABEL_SIZE.width() / 2.0) - LABEL_PADDING
                );

                // Test custom avatar layout constraints.
                pump(
                    tester,
                    Some(BoxConstraints::tight_for_finite(
                        f64::INFINITY,
                        f64::INFINITY,
                    )),
                );
                tester.pump();

                assert_eq!(chip_size(tester).width(), 152.0);
                assert_eq!(chip_size(tester).height(), 118.0);

                // Calculate the distance between delete icon and chip edges.
                let chip_top_right = chip_material_rect(tester).top_right();
                assert_eq!(
                    chip_top_right.dx(),
                    delete_icon_center.dx() + (ICON_SIZE / 2.0) + PADDING + BORDER
                );
                assert_eq!(
                    chip_top_right.dy(),
                    delete_icon_center.dy() - (LABEL_SIZE.width() / 2.0) - PADDING - BORDER
                );

                // Calculate the distance between delete icon and label.
                let label_top_right = tester.get_rect(only::<Container>(tester)).top_right();
                assert_eq!(
                    label_top_right.dx(),
                    delete_icon_center.dx() - (ICON_SIZE / 2.0) - LABEL_PADDING
                );
            });
        }

        #[test]
        fn chip_avatar_layout_constraints_can_be_customized() {
            check_avatar_layout_constraints_can_be_customized::<Chip>(|constraints| {
                let mut chip =
                    Chip::new(label_container()).avatar(Icon::new(Some(Icons::favorite())));
                if let Some(constraints) = constraints {
                    chip = chip.avatar_box_constraints(constraints);
                }
                chip.into_widget()
            });
        }

        #[test]
        fn raw_chip_avatar_layout_constraints_can_be_customized() {
            check_avatar_layout_constraints_can_be_customized::<RawChip>(|constraints| {
                let mut chip =
                    RawChip::new(label_container()).avatar(Icon::new(Some(Icons::favorite())));
                if let Some(constraints) = constraints {
                    chip = chip.avatar_box_constraints(constraints);
                }
                chip.into_widget()
            });
        }

        #[test]
        fn chip_delete_icon_layout_constraints_can_be_customized() {
            check_delete_icon_layout_constraints_can_be_customized::<Chip>(|constraints| {
                let mut chip = Chip::new(label_container()).on_deleted(Listener::new(|_| {}));
                if let Some(constraints) = constraints {
                    chip = chip.delete_icon_box_constraints(constraints);
                }
                chip.into_widget()
            });
        }

        #[test]
        fn raw_chip_delete_icon_layout_constraints_can_be_customized() {
            check_delete_icon_layout_constraints_can_be_customized::<RawChip>(|constraints| {
                let mut chip = RawChip::new(label_container()).on_deleted(Listener::new(|_| {}));
                if let Some(constraints) = constraints {
                    chip = chip.delete_icon_box_constraints(constraints);
                }
                chip.into_widget()
            });
        }

        #[test]
        fn default_delete_button_ink_well_shape() {
            test_widgets(|tester| {
                let root = wrap_ltr(
                    tester,
                    Center::new().child(
                        RawChip::new(Text::new("RawChip")).on_deleted(Listener::new(|_| {})),
                    ),
                );
                tester.pump_widget(root);

                let icon = tester.element(find::by_icon(&Icons::cancel()).first());
                let delete_button_ink_well = tester
                    .ancestors(icon)
                    .into_iter()
                    .find(|ancestor| {
                        let app = tester.cell.borrow();
                        waft_widgets::downcast_widget::<crate::ink_well::InkWell>(
                            &**ancestor.widget(&app),
                        )
                        .is_some()
                    })
                    .expect("an InkWell above the delete icon");
                let custom_border = tester.widget(
                    delete_button_ink_well,
                    |ink_well: &crate::ink_well::InkWell| {
                        ink_well.ink_response.custom_border.clone()
                    },
                );
                assert_eq!(
                    custom_border,
                    Some(ShapeBorderRef::new(CircleBorder::default()))
                );
            });
        }

        #[test]
        fn default_delete_button_overlay() {
            use waft_embedder::valo::{FillRule, Op, PathBuilder, Point};

            test_widgets(|tester| {
                let theme = m3(&tester.cell.borrow());
                let root = wrap_with_theme(
                    tester,
                    theme.clone(),
                    Center::new().child(
                        RawChip::new(Text::new("RawChip")).on_deleted(Listener::new(|_| {})),
                    ),
                );
                tester.pump_widget(root);

                let ink_ops = |tester: &WidgetTester| {
                    let ink_features = tester.first::<crate::material::RenderInkFeatures>();
                    tester.painted_ops_deep(ink_features.as_object())
                };
                let paints_hover_rect =
                    |ops: &[Op]| paints(ops, &[PaintCall::rect(Some(theme.hover_color))]).is_ok();
                // Dart's `paintsExactlyCountTimes(#clipPath, n)`. Valo records a `clipRect` as an
                // axis-aligned path clip, which Dart counts as `#clipRect`, not `#clipPath`.
                let clip_paths = |ops: &[Op]| {
                    ops.iter()
                        .filter_map(|op| match op {
                            Op::ClipPath {
                                path,
                                is_axis_aligned_rect: false,
                                ..
                            } => Some(std::sync::Arc::clone(path)),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                };

                let ops = ink_ops(tester);
                assert!(!paints_hover_rect(&ops));
                assert_eq!(clip_paths(&ops).len(), 0);

                // Hover over the delete icon.
                let center_of_delete_button = tester.get_center(only::<Icon>(tester));
                let mut hover_gesture = TestPointer::mouse(1, 0);
                tester.add_pointer(&mut hover_gesture, Offset::new(10.0, 10.0));
                tester.pump();
                tester.move_to(&mut hover_gesture, center_of_delete_button, false);
                tester.pump_and_settle();

                let ops = ink_ops(tester);
                assert!(paints_hover_rect(&ops), "{ops:?}");
                let clips = clip_paths(&ops);
                assert_eq!(clips.len(), 1);

                let expected_clip_rect = Rect::from_ltrb(124.7, 10.0, 142.7, 28.0);
                let mut builder = PathBuilder::new();
                builder.rect(waft_embedder::valo::Rect::new(
                    expected_clip_rect.left as f32,
                    expected_clip_rect.top as f32,
                    expected_clip_rect.width() as f32,
                    expected_clip_rect.height() as f32,
                ));
                let expected_clip_path = builder.build();
                // Dart's `coversSameAreaAs(expected, areaToCompare: rect.inflate(48.0), sampleSize: 100)`.
                // Like Dart's matcher, the samples start at the origin of the plane, not at the area's
                // corner, and each has a second, jittered sample (here a fixed sequence in place of
                // Dart's seeded `Random(1)`).
                let area = expected_clip_rect.inflate(48.0);
                let sample_size = 100;
                let mut jitter = 0.5_f64;
                let mut next_jitter = || {
                    jitter = (jitter * 9301.0 + 0.49297) % 1.0;
                    jitter
                };
                let covers = |point: Offset| {
                    let point = Point::new(point.dx() as f32, point.dy() as f32);
                    clips[0].contains(point, FillRule::NonZero)
                        == expected_clip_path.contains(point, FillRule::NonZero)
                };
                for i in 0..sample_size {
                    for j in 0..sample_size {
                        let at = Offset::new(
                            i as f64 * (area.width() / sample_size as f64),
                            j as f64 * (area.height() / sample_size as f64),
                        );
                        assert!(covers(at), "{at:?}");
                        let noise = Offset::new(
                            area.width() / sample_size as f64 * next_jitter(),
                            area.height() / sample_size as f64 * next_jitter(),
                        );
                        assert!(covers(at + noise), "{:?}", at + noise);
                    }
                }

                // Dart's samples all lie beside the clip (x below 115), so its matcher only checks that
                // the clip misses them; the clip's bounds pin the clip itself, the delete button's circle.
                let bounds = clips[0].bounds();
                for (actual, expected) in [
                    (bounds.x, expected_clip_rect.left),
                    (bounds.y, expected_clip_rect.top),
                    (bounds.width, expected_clip_rect.width()),
                    (bounds.height, expected_clip_rect.height()),
                ] {
                    assert!((f64::from(actual) - expected).abs() < 0.01, "{bounds:?}");
                }
            });
        }
    }

    mod material2_and_misc {
        //! chip_test.dart: the 'Material 2' group, baseline, icon theme, animation
        //! styles, mouse cursors and zero area.
        use waft_widgets::testing::{
            TestPointer, WidgetTester, assert_more_or_less_equals_within, find, test_widgets,
        };

        use std::cell::Cell;
        use std::rc::Rc;
        use std::time::Duration;

        use waft_animation::AnimationStyle;
        use waft_embedder::{Brightness, Color, Size, TargetPlatform, TextBaseline};
        use waft_foundation::Listener;
        use waft_painting::{StadiumBorder, TextStyle};
        use waft_widgets::{
            Align, Builder, Center, Column, CrossAxisAlignment, Icon, MainAxisSize, Row, SizedBox,
            StatefulBuilder, Text,
        };

        use super::{icon_data, label_style, material_box_paints, only, text, wrap_ltr};
        use crate::chip::*;
        use crate::chip_theme::{ChipTheme, ChipThemeData};
        use crate::circle_avatar::CircleAvatar;
        use crate::colors::Colors;
        use crate::icons::Icons;
        use crate::material::Material;
        use crate::scaffold::Scaffold;
        use crate::test_support::material_app;

        use crate::theme::Theme;
        use crate::theme_data::ThemeData;
        use waft_rendering::testing::PaintCall;

        fn font_summary(style: &TextStyle) -> impl PartialEq + std::fmt::Debug {
            (
                style.font_family.clone(),
                style.font_size,
                style.font_style,
                style.font_weight,
                style.height,
                style.inherit,
                style.leading_distribution,
                style.letter_spacing,
                style.text_baseline,
                style.word_spacing,
            )
        }

        #[test]
        fn m2_chip_defaults() {
            test_widgets(|tester| {
                let text_theme: Rc<std::cell::RefCell<Option<crate::text_theme::TextTheme>>> =
                    Rc::default();
                let build_frame = |tester: &mut WidgetTester, brightness: Brightness| {
                    let theme = ThemeData::builder()
                        .brightness(brightness)
                        .use_material3(false)
                        .build(&tester.cell.borrow());
                    let text_theme = Rc::clone(&text_theme);
                    tester.pump_widget(material_app(
                        theme,
                        Scaffold::new()
                            .body(Center::new().child(Builder::new(move |app, context| {
                                *text_theme.borrow_mut() =
                                    Some(Theme::of(app, context).text_theme.clone());
                                Chip::new(Text::new("Chip A"))
                                    .avatar(CircleAvatar::new().child(Text::new("A")))
                                    .on_deleted(Listener::new(|_app| {}))
                                    .into_widget()
                            })))
                            .into_widget(),
                    ));
                };
                let check_common = |tester: &WidgetTester| {
                    assert_eq!(
                        tester.get_size(only::<Chip>(tester)),
                        Size::new(156.0, 48.0)
                    );
                    let material = super::tests::material_of_raw_chip(tester);
                    let (color, elevation, shape) =
                        tester.widget(material, |material: &Material| {
                            (material.color, material.elevation, material.shape.clone())
                        });
                    assert_eq!(color, None);
                    assert_eq!(elevation, 0.0);
                    assert_eq!(
                        shape,
                        Some(waft_painting::ShapeBorderRef::new(StadiumBorder::new(
                            waft_painting::BorderSide::NONE
                        )))
                    );
                    assert_eq!(icon_data(tester).size, Some(18.0));
                };
                let check_label_style = |tester: &WidgetTester, color: i32| {
                    let label_style = label_style(tester, "Chip A");
                    assert_eq!(
                        label_style
                            .color
                            .as_ref()
                            .map(|color| color.color().to_argb32()),
                        Some(color as u32)
                    );
                    let body_large = text_theme
                        .borrow()
                        .as_ref()
                        .and_then(|text_theme| text_theme.body_large.clone())
                        .expect("bodyLarge");
                    assert_eq!(font_summary(&label_style), font_summary(&body_large));
                };

                build_frame(tester, Brightness::Light);
                assert!(material_box_paints(
                    tester,
                    &[
                        PaintCall::rrect(None),
                        PaintCall::circle(Some(Color::new(0xff1976d2)))
                    ]
                ));
                check_common(tester);
                assert_eq!(
                    icon_data(tester).color.map(|color| color.color()),
                    Some(Color::new(0xdd000000))
                );
                assert_eq!(icon_data(tester).opacity, None);
                check_label_style(tester, 0xde000000u32 as i32);

                build_frame(tester, Brightness::Dark);
                tester.pump_and_settle(); // Theme transition animation
                assert!(material_box_paints(
                    tester,
                    &[PaintCall::rrect(Some(Color::new(0x1fffffff)))]
                ));
                check_common(tester);
                assert_eq!(
                    icon_data(tester)
                        .color
                        .map(|color| color.color().to_argb32()),
                    Some(0xffffffffu32)
                );
                assert_eq!(icon_data(tester).opacity, None);
                check_label_style(tester, 0xdeffffffu32 as i32);
            });
        }

        fn label_color(tester: &WidgetTester, label: &str) -> Option<Color> {
            label_style(tester, label).color.map(|color| color.color())
        }

        fn icon_color(tester: &WidgetTester) -> Option<Color> {
            icon_data(tester).color.map(|color| color.color())
        }

        #[test]
        fn m2_chip_uses_the_right_theme_colors_for_the_right_components() {
            test_widgets(|tester| {
                let theme_data = ThemeData::builder()
                    .platform(TargetPlatform::Android)
                    .primary_swatch(Colors::BLUE)
                    .use_material3(false)
                    .build(&tester.cell.borrow());
                let default_chip_theme = ChipThemeData::from_defaults(
                    Some(theme_data.brightness()),
                    None,
                    Colors::BLUE.primary(),
                    theme_data.text_theme.body_large.clone().expect("bodyLarge"),
                );
                let value = Rc::new(Cell::new(false));
                let build_app =
                    |tester: &mut WidgetTester, chip_theme: &ChipThemeData, selectable: bool| {
                        let value = Rc::clone(&value);
                        let chip_theme = chip_theme.clone();
                        let theme_data = theme_data.clone();
                        let root = wrap_ltr(
                            tester,
                            Theme::new(
                                theme_data,
                                ChipTheme::new(
                                    chip_theme.clone(),
                                    StatefulBuilder::new(move |_app, _context, set_state| {
                                        let on_selected: waft_foundation::ValueChanged<bool> = {
                                            let value = Rc::clone(&value);
                                            Rc::new(move |app, new_value| {
                                                set_state(app, &mut |_app| value.set(new_value));
                                            })
                                        };
                                        let mut chip =
                                            RawChip::new(Text::new(format!("{}", value.get())))
                                                .show_checkmark(true)
                                                .on_deleted(Listener::new(|_app| {}))
                                                .is_enabled(selectable)
                                                .selected(selectable && value.get());
                                        if let Some(shape) = chip_theme.shape.clone() {
                                            chip = chip.shape(shape);
                                        }
                                        if selectable {
                                            chip = chip.on_selected(on_selected);
                                        }
                                        chip.into_widget()
                                    }),
                                ),
                            ),
                        );
                        tester.pump_widget(root);
                    };
                let tap_chip = |tester: &mut WidgetTester| {
                    let center = tester.get_center(only::<RawChip>(tester));
                    tester.tap_at(center, 1);
                    tester.pump_and_settle();
                };
                let drawn_color = |tester: &WidgetTester, color: Option<Color>| {
                    material_box_paints(tester, &[PaintCall::rrect(color)])
                };
                let black_de = Some(Colors::BLACK.with_alpha(0xde));

                build_app(tester, &default_chip_theme, true);

                // Check default theme for enabled chip.
                assert!(drawn_color(tester, default_chip_theme.background_color));
                assert_eq!(icon_color(tester), Some(Color::new(0xde000000)));
                assert_eq!(label_color(tester, "false"), black_de);

                // Check default theme for disabled chip.
                build_app(tester, &default_chip_theme, false);
                tester.pump_and_settle();
                assert!(drawn_color(tester, default_chip_theme.disabled_color));
                assert_eq!(label_color(tester, "false"), black_de);

                // Check default theme for enabled and selected chip.
                build_app(tester, &default_chip_theme, true);
                tester.pump_and_settle();
                tap_chip(tester);
                assert!(drawn_color(tester, default_chip_theme.selected_color));

                // Check default theme for disabled and selected chip.
                build_app(tester, &default_chip_theme, false);
                tester.pump_and_settle();
                assert!(drawn_color(tester, default_chip_theme.disabled_color));
                assert_eq!(label_color(tester, "true"), black_de);

                // Enable the chip again.
                build_app(tester, &default_chip_theme, true);
                tester.pump_and_settle();
                // Tap to unselect the chip.
                tap_chip(tester);

                // Apply a custom theme.
                let custom_color1 = Color::new(0xcafefeed);
                let custom_color2 = Color::new(0xdeadbeef);
                let custom_color3 = Color::new(0xbeefcafe);
                let custom_color4 = Color::new(0xaddedabe);
                let custom_theme = default_chip_theme
                    .copy_with()
                    .brightness(Brightness::Dark)
                    .background_color(custom_color1)
                    .disabled_color(custom_color2)
                    .selected_color(custom_color3)
                    .delete_icon_color(custom_color4);
                build_app(tester, &custom_theme, true);
                tester.pump_and_settle();

                // Check custom theme for enabled chip.
                assert!(drawn_color(tester, custom_theme.background_color));
                assert_eq!(
                    icon_color(tester),
                    custom_theme.delete_icon_color.as_ref().map(AnyColor::color)
                );
                assert_eq!(label_color(tester, "false"), black_de);

                // Check custom theme with disabled widget.
                build_app(tester, &custom_theme, false);
                tester.pump_and_settle();
                assert!(drawn_color(tester, custom_theme.disabled_color));
                assert_eq!(label_color(tester, "false"), black_de);

                // Check custom theme for enabled and selected chip.
                build_app(tester, &custom_theme, true);
                tester.pump_and_settle();
                tap_chip(tester);
                assert!(drawn_color(tester, custom_theme.selected_color));

                // Check custom theme for disabled and selected chip.
                build_app(tester, &custom_theme, false);
                tester.pump_and_settle();
                assert!(drawn_color(tester, custom_theme.disabled_color));
                assert_eq!(label_color(tester, "true"), black_de);
            });
        }

        #[test]
        fn chip_baseline_location() {
            test_widgets(|tester| {
                let text_a = || Text::new("A").style(TextStyle::new().font_size(10.0).height(1.0));
                let root = wrap_ltr(
                    tester,
                    Align::new().child(
                        Row::new()
                            .cross_axis_alignment(CrossAxisAlignment::Baseline)
                            .text_baseline(TextBaseline::Alphabetic)
                            .children([
                                text_a().into_widget(),
                                RawChip::new(text_a()).into_widget(),
                            ]),
                    ),
                );
                tester.pump_widget(root);

                let texts = find::text("A").evaluate(tester);
                assert_eq!(texts.len(), 2);
                // Baseline aligning text.
                assert_eq!(
                    tester.get_rect(texts[0]).top_left().dy(),
                    tester.get_rect(texts[texts.len() - 1]).top_left().dy()
                );
            });
        }

        fn icon_count(tester: &WidgetTester, icon: &waft_widgets::IconData) -> usize {
            find::by_type::<Icon>()
                .evaluate(tester)
                .into_iter()
                .filter(|element| {
                    tester.widget(*element, |widget: &Icon| widget.icon.as_ref() == Some(icon))
                })
                .count()
        }

        /// Dart's `MaterialApp(home: Material(child: Center(child: child)))`.
        fn pump_centered<K>(tester: &mut WidgetTester, child: impl waft_widgets::IntoWidget<K>) {
            let theme = crate::tooltip::tests::m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Material::new()
                    .child(Center::new().child(child))
                    .into_widget(),
            ));
        }

        #[test]
        fn chip_theme_data_icon_theme_updates_avatar_and_delete_icons() {
            test_widgets(|tester| {
                let icon_color = Color::new(0xffff00ff);
                let icon_size = 28.0;
                pump_centered(
                    tester,
                    RawChip::new(SizedBox::new().height(100.0))
                        .icon_theme(IconThemeData::new().color(icon_color).size(icon_size))
                        .avatar(Icon::new(Some(Icons::favorite())))
                        .delete_icon(Icon::new(Some(Icons::delete())))
                        .on_deleted(Listener::new(|_app| {})),
                );

                // Test rendered icon size.
                let avatar_icon = tester.element(find::by_icon(&Icons::favorite()).first());
                let delete_icon = tester.element(find::by_icon(&Icons::delete()).first());
                assert_eq!(tester.get_size(avatar_icon).width(), icon_size);
                assert_eq!(tester.get_size(delete_icon).width(), icon_size);

                // Test rendered icon color.
                assert_eq!(
                    tester
                        .glyph_style(avatar_icon)
                        .color
                        .map(|color| color.color()),
                    Some(icon_color)
                );
                assert_eq!(
                    tester
                        .glyph_style(delete_icon)
                        .color
                        .map(|color| color.color()),
                    Some(icon_color)
                );
            });
        }

        #[test]
        fn raw_chip_delete_icon_color_overrides_icon_theme_color() {
            test_widgets(|tester| {
                let icon_color = Color::new(0xffff00ff);
                let delete_icon_color = Color::new(0xffff00ff);
                let build_chip = |tester: &mut WidgetTester,
                                  delete_icon_color: Option<Color>,
                                  icon_color: Option<Color>| {
                    let mut icon_theme = IconThemeData::new();
                    if let Some(icon_color) = icon_color {
                        icon_theme = icon_theme.color(icon_color);
                    }
                    let mut chip = RawChip::new(SizedBox::new().height(100.0))
                        .icon_theme(icon_theme)
                        .delete_icon(Icon::new(Some(Icons::delete())))
                        .on_deleted(Listener::new(|_app| {}));
                    if let Some(delete_icon_color) = delete_icon_color {
                        chip = chip.delete_icon_color(delete_icon_color);
                    }
                    pump_centered(tester, chip);
                };
                let rendered_color = |tester: &WidgetTester| {
                    tester
                        .glyph_style(tester.element(find::by_icon(&Icons::delete()).first()))
                        .color
                        .map(|color| color.color())
                };

                build_chip(tester, None, Some(icon_color));

                // Test rendered icon color.
                assert_eq!(rendered_color(tester), Some(icon_color));

                build_chip(tester, Some(delete_icon_color), Some(icon_color));

                // Test rendered icon color.
                assert_eq!(rendered_color(tester), Some(delete_icon_color));
            });
        }

        /// Dart's `Column(mainAxisSize: min, children: [chip, ElevatedButton(onPressed: toggle, child:
        /// Text(label))])` under a `StatefulBuilder`, with `state` the variable `toggle` flips.
        fn pump_chip_with_toggle(
            tester: &mut WidgetTester,
            state: &Rc<Cell<bool>>,
            chip: impl Fn(bool) -> RawChip + 'static,
            button_label: impl Fn(bool) -> String + 'static,
        ) {
            let state = Rc::clone(state);
            pump_centered(
                tester,
                StatefulBuilder::new(move |_app, _context, set_state| {
                    let toggled = Rc::clone(&state);
                    Column::new()
                        .main_axis_size(MainAxisSize::Min)
                        .children([
                            chip(state.get()).into_widget(),
                            crate::elevated_button::ElevatedButton::new(
                                Some(Listener::new(move |app| {
                                    set_state(app, &mut |_app| toggled.set(!toggled.get()));
                                })),
                                Some(Text::new(button_label(state.get()))),
                            )
                            .into_widget(),
                        ])
                        .into_widget()
                }),
            );
        }

        fn pump_ms(tester: &mut WidgetTester, ms: u64) {
            tester.pump_by(Duration::from_millis(ms));
        }

        fn animation_style(duration_ms: u64, reverse_duration_ms: u64) -> AnimationStyle {
            AnimationStyle::new(
                None,
                Some(Duration::from_millis(duration_ms)),
                None,
                Some(Duration::from_millis(reverse_duration_ms)),
            )
        }

        #[test]
        fn chip_animation_style_enable_animation_overrides_chip_enable_animation_duration() {
            test_widgets(|tester| {
                let disabled_color = Color::new(0xffff0000);
                let background_color = Color::new(0xff00ff00);
                let enabled = Rc::new(Cell::new(true));
                pump_chip_with_toggle(
                    tester,
                    &enabled,
                    move |enabled| {
                        RawChip::new(Text::new("RawChip"))
                            .chip_animation_style(
                                ChipAnimationStyle::new()
                                    .enable_animation(animation_style(300, 150)),
                            )
                            .is_enabled(enabled)
                            .disabled_color(disabled_color)
                            .background_color(background_color)
                    },
                    |enabled| format!("{} Chip", if enabled { "Disable" } else { "Enable" }),
                );
                let paints = |tester: &WidgetTester, color: Color| {
                    assert!(
                        material_box_paints(tester, &[PaintCall::rrect(Some(color))]),
                        "{color:?}"
                    );
                };

                // Test background color when the chip is enabled.
                paints(tester, background_color);

                tester.tap(find::text("Disable Chip"));
                tester.pump();
                pump_ms(tester, 75);

                paints(tester, Color::new(0x80ff0000));

                pump_ms(tester, 75);

                // Test background color when the chip is disabled.
                paints(tester, disabled_color);

                tester.tap(find::text("Enable Chip"));
                tester.pump();
                pump_ms(tester, 150);

                paints(tester, Color::new(0x8000ff00));

                pump_ms(tester, 150);

                // Test background color when the chip is enabled.
                paints(tester, background_color);
            });
        }

        #[test]
        fn chip_animation_style_select_animation_overrides_chip_selection_animation_duration() {
            test_widgets(|tester| {
                let background_color = Color::new(0xff00ff00);
                let selected_color = Color::new(0xff0000ff);
                let selected = Rc::new(Cell::new(false));
                pump_chip_with_toggle(
                    tester,
                    &selected,
                    move |selected| {
                        RawChip::new(Text::new("RawChip"))
                            .chip_animation_style(
                                ChipAnimationStyle::new()
                                    .select_animation(animation_style(600, 300)),
                            )
                            .background_color(background_color)
                            .selected_color(selected_color)
                            .selected(selected)
                            .on_selected(Rc::new(|_app, _value| {}))
                    },
                    |selected| format!("{} Chip", if selected { "Unselect" } else { "Select" }),
                );
                let paints = |tester: &WidgetTester, color: Color| {
                    assert!(
                        material_box_paints(tester, &[PaintCall::rrect(Some(color))]),
                        "{color:?}"
                    );
                };

                // Test background color when the chip is unselected.
                paints(tester, background_color);

                tester.tap(find::text("Select Chip"));
                tester.pump();
                pump_ms(tester, 300);

                paints(tester, Color::new(0xc60000ff));

                pump_ms(tester, 300);

                // Test background color when the chip is selected.
                paints(tester, selected_color);

                tester.tap(find::text("Unselect Chip"));
                tester.pump();
                pump_ms(tester, 150);

                paints(tester, Color::new(0x3900ff00));

                pump_ms(tester, 150);

                // Test background color when the chip is unselected.
                paints(tester, background_color);
            });
        }

        #[test]
        fn chip_animation_style_avatar_drawer_animation_overrides_chip_avatar_animation_duration() {
            test_widgets(|tester| {
                let checkmark_color = Color::new(0xffff0000);
                let selected = Rc::new(Cell::new(false));
                pump_chip_with_toggle(
                    tester,
                    &selected,
                    move |selected| {
                        RawChip::new(Text::new("RawChip"))
                            .chip_animation_style(
                                ChipAnimationStyle::new()
                                    .avatar_drawer_animation(animation_style(800, 400)),
                            )
                            .checkmark_color(checkmark_color)
                            .selected(selected)
                            .on_selected(Rc::new(|_app, _value| {}))
                    },
                    |selected| format!("{} Chip", if selected { "Unselect" } else { "Select" }),
                );
                let chip_width =
                    |tester: &WidgetTester| tester.get_size(only::<RawChip>(tester)).width();
                let checkmark_drawn = |tester: &WidgetTester| {
                    material_box_paints(tester, &[PaintCall::path(Some(checkmark_color))])
                };

                // Test the checkmark is not visible yet.
                assert!(!checkmark_drawn(tester));
                assert_more_or_less_equals_within(chip_width(tester), 132.6, 0.1);

                tester.tap(find::text("Select Chip"));
                tester.pump();
                pump_ms(tester, 400);

                assert!(checkmark_drawn(tester));
                assert_more_or_less_equals_within(chip_width(tester), 148.2, 0.1);

                pump_ms(tester, 400);

                // Test the checkmark is fully visible.
                assert!(checkmark_drawn(tester));
                assert_more_or_less_equals_within(chip_width(tester), 152.6, 0.1);

                tester.tap(find::text("Unselect Chip"));
                tester.pump();
                pump_ms(tester, 200);

                assert!(!checkmark_drawn(tester));
                assert_more_or_less_equals_within(chip_width(tester), 148.2, 0.1);

                pump_ms(tester, 200);

                // Test if checkmark is removed.
                assert!(!checkmark_drawn(tester));
                assert_more_or_less_equals_within(chip_width(tester), 132.6, 0.1);
            });
        }

        #[test]
        fn chip_animation_style_delete_drawer_animation_overrides_chip_delete_icon_animation_duration()
         {
            test_widgets(|tester| {
                let show_delete_icon = Rc::new(Cell::new(false));
                pump_chip_with_toggle(
                    tester,
                    &show_delete_icon,
                    move |show_delete_icon| {
                        let mut chip = RawChip::new(Text::new("RawChip")).chip_animation_style(
                            ChipAnimationStyle::new()
                                .delete_drawer_animation(animation_style(500, 250)),
                        );
                        if show_delete_icon {
                            chip = chip.on_deleted(Listener::new(|_app| {}));
                        }
                        chip
                    },
                    |show| format!("{} delete icon", if show { "Hide" } else { "Show" }),
                );
                let chip_width =
                    |tester: &WidgetTester| tester.get_size(only::<RawChip>(tester)).width();
                let cancel_icons = |tester: &WidgetTester| icon_count(tester, &Icons::cancel());

                // Test the delete icon is not visible yet.
                assert_eq!(cancel_icons(tester), 0);
                assert_more_or_less_equals_within(chip_width(tester), 132.6, 0.1);

                tester.tap(find::text("Show delete icon"));
                tester.pump();
                pump_ms(tester, 250);

                assert_eq!(cancel_icons(tester), 1);
                assert_more_or_less_equals_within(chip_width(tester), 148.2, 0.1);

                pump_ms(tester, 250);

                // Test the delete icon is fully visible.
                assert_eq!(cancel_icons(tester), 1);
                assert_more_or_less_equals_within(chip_width(tester), 152.6, 0.1);

                tester.tap(find::text("Hide delete icon"));
                tester.pump();
                pump_ms(tester, 125);

                assert_eq!(cancel_icons(tester), 1);
                assert_more_or_less_equals_within(chip_width(tester), 148.2, 0.1);

                pump_ms(tester, 125);

                // Test if delete icon is removed.
                assert_eq!(cancel_icons(tester), 0);
                assert_more_or_less_equals_within(chip_width(tester), 132.6, 0.1);
            });
        }

        #[test]
        fn chip_chip_animation_style_is_passed_to_raw_chip() {
            test_widgets(|tester| {
                let chip_animation_style = ChipAnimationStyle::new()
                    .enable_animation(AnimationStyle::NO_ANIMATION)
                    .select_animation(AnimationStyle::new(
                        None,
                        Some(crate::motion::Durations::LONG3),
                        None,
                        None,
                    ));
                let root = wrap_ltr(
                    tester,
                    Center::new().child(
                        Chip::new(Text::new("Chip"))
                            .chip_animation_style(chip_animation_style.clone()),
                    ),
                );
                tester.pump_widget(root);

                let passed = tester
                    .widget(only::<RawChip>(tester), |raw_chip: &RawChip| {
                        raw_chip.chip_animation_style.clone()
                    })
                    .expect("a chip animation style");
                // Dart compares by identity (`ChipAnimationStyle` has no `==`); the four styles carry it.
                assert_eq!(
                    passed.enable_animation,
                    chip_animation_style.enable_animation
                );
                assert_eq!(
                    passed.select_animation,
                    chip_animation_style.select_animation
                );
                assert_eq!(
                    passed.avatar_drawer_animation,
                    chip_animation_style.avatar_drawer_animation
                );
                assert_eq!(
                    passed.delete_drawer_animation,
                    chip_animation_style.delete_drawer_animation
                );
            });
        }

        // Regression test for https://github.com/flutter/flutter/issues/157622.
        #[test]
        fn chip_does_not_glitch_on_hover_when_providing_theme_data_hover_color() {
            test_widgets(|tester| {
                let theme_data_hover_color = Color::new(0xffff0000);
                let hover_color = Color::new(0xff00ff00);
                let background_color = Color::new(0xff0000ff);
                let theme = ThemeData::builder()
                    .hover_color(theme_data_hover_color)
                    .build(&tester.cell.borrow());
                let root = crate::chip::tests::wrap_with_theme(
            tester,
            theme,
            Center::new().child(
                RawChip::new(Text::new("Chip"))
                    .color(
                        <dyn waft_widgets::WidgetStateProperty<Option<Color>>>::resolve_with(
                            move |states| {
                                if states.contains(&waft_widgets::WidgetState::Hovered) {
                                    Some(hover_color)
                                } else {
                                    Some(background_color)
                                }
                            },
                        ),
                    )
                    .on_pressed(Listener::new(|_app| {})),
            ),
        );
                tester.pump_widget(root);

                assert!(material_box_paints(
                    tester,
                    &[PaintCall::rrect(Some(background_color))]
                ));

                // Hover over the chip.
                let center = tester.get_center(only::<RawChip>(tester));
                let mut mouse = TestPointer::mouse(1, 0);
                tester.add_pointer(&mut mouse, center);
                tester.move_to(&mut mouse, center, false);
                tester.pump_and_settle();

                // Dart's `paints..rrect(color: hoverColor)..rect(color: Colors.transparent)` is deferred:
                // the display list drops the transparent rect (`PORTING.md`).
                assert!(!material_box_paints(
                    tester,
                    &[
                        PaintCall::rrect(Some(hover_color)),
                        PaintCall::rect(Some(theme_data_hover_color))
                    ]
                ));
                tester.send(&mouse, waft_embedder::PointerChange::Remove);
            });
        }

        #[test]
        fn chip_mouse_cursor_behavior() {
            test_widgets(|tester| {
                let custom_cursor: waft_services::MouseCursorRef =
                    waft_services::SystemMouseCursors::GRAB.into();
                let root = wrap_ltr(
                    tester,
                    Center::new()
                        .child(Chip::new(Text::new("Chip")).mouse_cursor(custom_cursor.clone())),
                );
                tester.pump_widget(root);

                let mut mouse = TestPointer::mouse(1, 0);
                crate::action_chip::tests::hover_over_chip(tester, &mut mouse);
                assert!(*tester.active_cursor(1) == *custom_cursor);
            });
        }

        #[test]
        fn mouse_cursor_resolves_in_disabled_states() {
            test_widgets(|tester| {
                tester.set_highlight_strategy(
                    waft_widgets::FocusHighlightStrategy::AlwaysTraditional,
                );
                let cursor = waft_widgets::WidgetStateMouseCursor::from_map(vec![(
                    waft_widgets::WidgetState::Disabled.into(),
                    waft_services::SystemMouseCursors::FORBIDDEN.into(),
                )]);
                let root = wrap_ltr(
                    tester,
                    Center::new().child(Chip::new(Text::new("Chip")).mouse_cursor(cursor.into())),
                );
                tester.pump_widget(root);

                // Unfocused case.
                let mut mouse = TestPointer::mouse(1, 0);
                let chip = tester.get_center(text(tester, "Chip"));
                tester.add_pointer(&mut mouse, chip);
                tester.pump();
                tester.move_to(&mut mouse, chip, false);
                assert!(
                    *tester.active_cursor(1)
                        == *waft_services::MouseCursorRef::from(
                            waft_services::SystemMouseCursors::FORBIDDEN
                        )
                );
                tester.send(&mouse, waft_embedder::PointerChange::Remove);
            });
        }

        #[test]
        fn chip_renders_at_zero_area() {
            test_widgets(|tester| {
                let theme = crate::tooltip::tests::m3(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(
                            SizedBox::shrink()
                                .child(Scaffold::new().body(Chip::new(Text::new("X")))),
                        )
                        .into_widget(),
                ));
                assert!(tester.get_size(text(tester, "X")).is_empty());
            });
        }

        #[test]
        fn raw_chip_renders_at_zero_area() {
            test_widgets(|tester| {
                let theme = crate::tooltip::tests::m3(&tester.cell.borrow());
                tester.pump_widget(material_app(
                    theme,
                    Center::new()
                        .child(
                            SizedBox::shrink()
                                .child(Scaffold::new().body(RawChip::new(Text::new("X")))),
                        )
                        .into_widget(),
                ));
                assert!(tester.get_size(text(tester, "X")).is_empty());
            });
        }
    }
}
