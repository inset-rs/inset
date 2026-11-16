//! Flutter counterpart: material_ui's `list_tile_theme.dart`.

use waft_embedder::{Color, lerp_double};
use waft_foundation::App;
use waft_painting::{AnyColor, EdgeInsetsGeometry, ShapeBorderRef, TextStyle};
use waft_services::MouseCursorRef;
use waft_widgets::{
    BuildContext, Builder, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, WidgetRef,
    WidgetStatePropertyRef,
};

use crate::list_tile::{ListTileControlAffinity, ListTileStyle, ListTileTitleAlignment};
use crate::theme::Theme;
use crate::theme_data::VisualDensity;

/// Used with [`ListTileTheme`] to define default property values for
/// descendant `ListTile` widgets, as well as classes that build
/// `ListTile`s, like `CheckboxListTile`, `RadioListTile`, and
/// `SwitchListTile`.
///
/// Descendant widgets obtain the current [`ListTileThemeData`] object
/// using [`ListTileTheme::of`]. Instances of [`ListTileThemeData`] can be
/// customized with [`ListTileThemeData::copy_with`].
///
/// A [`ListTileThemeData`] is often specified as part of the
/// overall `Theme` with `ThemeData::list_tile_theme`.
///
/// All [`ListTileThemeData`] properties are `None` by default.
/// When a theme property is `None`, the `ListTile` will provide its own
/// default based on the overall `Theme`'s text theme and
/// color scheme. See the individual `ListTile` properties for details.
///
/// The `Drawer` widget specifies a list tile theme for its children that
/// defines [`style`](Self::style) to be [`ListTileStyle::Drawer`].
///
/// See also:
///
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug, Default)]
pub struct ListTileThemeData {
    /// Overrides the default value of `ListTile::dense`.
    pub dense: Option<bool>,

    /// Overrides the default value of `ListTile::shape`.
    pub shape: Option<ShapeBorderRef>,

    /// Overrides the default value of `ListTile::style`.
    pub style: Option<ListTileStyle>,

    /// Overrides the default value of `ListTile::selected_color`.
    pub selected_color: Option<Color>,

    /// Overrides the default value of `ListTile::icon_color`.
    pub icon_color: Option<AnyColor>,

    /// Overrides the default value of `ListTile::text_color`.
    pub text_color: Option<AnyColor>,

    /// Overrides the default value of `ListTile::title_text_style`.
    pub title_text_style: Option<TextStyle>,

    /// Overrides the default value of `ListTile::subtitle_text_style`.
    pub subtitle_text_style: Option<TextStyle>,

    /// Overrides the default value of `ListTile::leading_and_trailing_text_style`.
    pub leading_and_trailing_text_style: Option<TextStyle>,

    /// Overrides the default value of `ListTile::content_padding`.
    pub content_padding: Option<EdgeInsetsGeometry>,

    /// Overrides the default value of `ListTile::tile_color`.
    pub tile_color: Option<Color>,

    /// Overrides the default value of `ListTile::selected_tile_color`.
    pub selected_tile_color: Option<Color>,

    /// Overrides the default value of `ListTile::horizontal_title_gap`.
    pub horizontal_title_gap: Option<f64>,

    /// Overrides the default value of `ListTile::min_vertical_padding`.
    pub min_vertical_padding: Option<f64>,

    /// Overrides the default value of `ListTile::min_leading_width`.
    pub min_leading_width: Option<f64>,

    /// Overrides the default value of `ListTile::min_tile_height`.
    pub min_tile_height: Option<f64>,

    /// Overrides the default value of `ListTile::enable_feedback`.
    pub enable_feedback: Option<bool>,

    /// If specified, overrides the default value of `ListTile::mouse_cursor`.
    pub mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,

    /// If specified, overrides the default value of `ListTile::visual_density`.
    pub visual_density: Option<VisualDensity>,

    /// If specified, overrides the default value of `ListTile::title_alignment`.
    pub title_alignment: Option<ListTileTitleAlignment>,

    /// If specified, overrides the default value of `CheckboxListTile::control_affinity`
    /// or `ExpansionTile::control_affinity` or `SwitchListTile::control_affinity` or
    /// `RadioListTile::control_affinity`.
    pub control_affinity: Option<ListTileControlAffinity>,

    /// If specified, overrides the default value of `ListTile::is_three_line`
    /// or `CheckboxListTile::is_three_line` or `RadioListTile::is_three_line` or
    /// `SwitchListTile::is_three_line`.
    pub is_three_line: Option<bool>,
}

crate::theme_data_getters::theme_data_getters! {
    /// Dart's getters of `ListTileThemeData`: the theme data answers with its fields, and a defaults
    /// subclass (Dart's `_...Defaults...(context)`) computes the ones it overrides when they
    /// are read.
    pub trait ListTileThemeDataGetters for ListTileThemeData, fields: list_tile_theme_data {
        dense: Option<bool>,
        shape: Option<ShapeBorderRef>,
        style: Option<ListTileStyle>,
        selected_color: Option<Color>,
        icon_color: Option<AnyColor>,
        text_color: Option<AnyColor>,
        title_text_style: Option<TextStyle>,
        subtitle_text_style: Option<TextStyle>,
        leading_and_trailing_text_style: Option<TextStyle>,
        content_padding: Option<EdgeInsetsGeometry>,
        tile_color: Option<Color>,
        selected_tile_color: Option<Color>,
        horizontal_title_gap: Option<f64>,
        min_vertical_padding: Option<f64>,
        min_leading_width: Option<f64>,
        min_tile_height: Option<f64>,
        enable_feedback: Option<bool>,
        mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,
        visual_density: Option<VisualDensity>,
        title_alignment: Option<ListTileTitleAlignment>,
        control_affinity: Option<ListTileControlAffinity>,
        is_three_line: Option<bool>,
    }
}

/// The setters of the arguments Dart's `ListTileThemeData` constructor and
/// `ListTileTheme.merge` share, each writing its field of `$data`.
macro_rules! list_tile_theme_data_setters {
    ($owner:ident, $dart:literal, |$this:ident| $data:expr, $($field:ident: $ty:ty => $name:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `", $dart, "(", $name, ":)`.")]
            pub fn $field(mut self, $field: impl Into<$ty>) -> $owner {
                let $this = &mut self;
                $data.$field = Some($field.into());
                self
            }
        )*
    };
}

/// Every setter `ListTileThemeData` and `ListTileTheme::merge` share.
macro_rules! every_list_tile_theme_data_setter {
    ($owner:ident, $dart:literal, |$this:ident| $data:expr) => {
        list_tile_theme_data_setters!(
            $owner,
            $dart,
            |$this| $data,
            dense: bool => "dense",
            shape: ShapeBorderRef => "shape",
            style: ListTileStyle => "style",
            selected_color: Color => "selectedColor",
            icon_color: AnyColor => "iconColor",
            text_color: AnyColor => "textColor",
            title_text_style: TextStyle => "titleTextStyle",
            subtitle_text_style: TextStyle => "subtitleTextStyle",
            leading_and_trailing_text_style: TextStyle => "leadingAndTrailingTextStyle",
            content_padding: EdgeInsetsGeometry => "contentPadding",
            tile_color: Color => "tileColor",
            selected_tile_color: Color => "selectedTileColor",
            horizontal_title_gap: f64 => "horizontalTitleGap",
            min_vertical_padding: f64 => "minVerticalPadding",
            min_leading_width: f64 => "minLeadingWidth",
            min_tile_height: f64 => "minTileHeight",
            enable_feedback: bool => "enableFeedback",
            mouse_cursor: WidgetStatePropertyRef<Option<MouseCursorRef>> => "mouseCursor",
            visual_density: VisualDensity => "visualDensity",
            title_alignment: ListTileTitleAlignment => "titleAlignment",
            control_affinity: ListTileControlAffinity => "controlAffinity",
            is_three_line: bool => "isThreeLine",
        );
    };
}

impl ListTileThemeData {
    /// Creates a [`ListTileThemeData`]; Dart's optional named arguments are the fluent
    /// setters.
    pub fn new() -> ListTileThemeData {
        ListTileThemeData::default()
    }

    every_list_tile_theme_data_setter!(ListTileThemeData, "ListTileThemeData", |this| this);

    /// Creates a copy of this object with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().dense(true)`).
    pub fn copy_with(&self) -> ListTileThemeData {
        self.clone()
    }

    /// Linearly interpolate between ListTileThemeData objects.
    pub fn lerp(
        a: Option<&ListTileThemeData>,
        b: Option<&ListTileThemeData>,
        t: f64,
    ) -> Option<ListTileThemeData> {
        match (a, b) {
            (None, None) => return None,
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return Some(a.clone()),
            _ => {}
        }
        Some(ListTileThemeData {
            dense: pick(a, b, t, |theme| theme.dense),
            shape: ShapeBorderRef::lerp(
                a.and_then(|a| a.shape.as_ref()),
                b.and_then(|b| b.shape.as_ref()),
                t,
            ),
            style: pick(a, b, t, |theme| theme.style),
            selected_color: Color::lerp(
                a.and_then(|a| a.selected_color),
                b.and_then(|b| b.selected_color),
                t,
            ),
            icon_color: AnyColor::lerp(
                a.and_then(|a| a.icon_color.as_ref()),
                b.and_then(|b| b.icon_color.as_ref()),
                t,
            )
            .map(AnyColor::new),
            text_color: AnyColor::lerp(
                a.and_then(|a| a.text_color.as_ref()),
                b.and_then(|b| b.text_color.as_ref()),
                t,
            )
            .map(AnyColor::new),
            title_text_style: TextStyle::lerp(
                a.and_then(|a| a.title_text_style.as_ref()),
                b.and_then(|b| b.title_text_style.as_ref()),
                t,
            ),
            subtitle_text_style: TextStyle::lerp(
                a.and_then(|a| a.subtitle_text_style.as_ref()),
                b.and_then(|b| b.subtitle_text_style.as_ref()),
                t,
            ),
            leading_and_trailing_text_style: TextStyle::lerp(
                a.and_then(|a| a.leading_and_trailing_text_style.as_ref()),
                b.and_then(|b| b.leading_and_trailing_text_style.as_ref()),
                t,
            ),
            content_padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.content_padding),
                b.and_then(|b| b.content_padding),
                t,
            ),
            tile_color: Color::lerp(
                a.and_then(|a| a.tile_color),
                b.and_then(|b| b.tile_color),
                t,
            ),
            selected_tile_color: Color::lerp(
                a.and_then(|a| a.selected_tile_color),
                b.and_then(|b| b.selected_tile_color),
                t,
            ),
            horizontal_title_gap: lerp_double(
                a.and_then(|a| a.horizontal_title_gap),
                b.and_then(|b| b.horizontal_title_gap),
                t,
            ),
            min_vertical_padding: lerp_double(
                a.and_then(|a| a.min_vertical_padding),
                b.and_then(|b| b.min_vertical_padding),
                t,
            ),
            min_leading_width: lerp_double(
                a.and_then(|a| a.min_leading_width),
                b.and_then(|b| b.min_leading_width),
                t,
            ),
            min_tile_height: lerp_double(
                a.and_then(|a| a.min_tile_height),
                b.and_then(|b| b.min_tile_height),
                t,
            ),
            enable_feedback: pick(a, b, t, |theme| theme.enable_feedback),
            mouse_cursor: pick(a, b, t, |theme| theme.mouse_cursor.clone()),
            visual_density: pick(a, b, t, |theme| theme.visual_density),
            title_alignment: pick(a, b, t, |theme| theme.title_alignment),
            control_affinity: pick(a, b, t, |theme| theme.control_affinity),
            is_three_line: pick(a, b, t, |theme| theme.is_three_line),
        })
    }
}

/// Dart's `t < 0.5 ? a?.field : b?.field`.
fn pick<T>(
    a: Option<&ListTileThemeData>,
    b: Option<&ListTileThemeData>,
    t: f64,
    field: impl Fn(&ListTileThemeData) -> Option<T>,
) -> Option<T> {
    if t < 0.5 {
        a.and_then(&field)
    } else {
        b.and_then(&field)
    }
}

impl PartialEq for ListTileThemeData {
    fn eq(&self, other: &ListTileThemeData) -> bool {
        if std::ptr::eq(self, other) {
            return true;
        }
        other.dense == self.dense
            && other.shape == self.shape
            && other.style == self.style
            && other.selected_color == self.selected_color
            && other.icon_color == self.icon_color
            && other.title_text_style == self.title_text_style
            && other.subtitle_text_style == self.subtitle_text_style
            && other.leading_and_trailing_text_style == self.leading_and_trailing_text_style
            && other.text_color == self.text_color
            && other.content_padding == self.content_padding
            && other.tile_color == self.tile_color
            && other.selected_tile_color == self.selected_tile_color
            && other.horizontal_title_gap == self.horizontal_title_gap
            && other.min_vertical_padding == self.min_vertical_padding
            && other.min_leading_width == self.min_leading_width
            && other.min_tile_height == self.min_tile_height
            && other.enable_feedback == self.enable_feedback
            && other.mouse_cursor == self.mouse_cursor
            && other.visual_density == self.visual_density
            && other.title_alignment == self.title_alignment
            && other.control_affinity == self.control_affinity
            && other.is_three_line == self.is_three_line
    }
}

/// An inherited widget that defines color and style parameters for `ListTile`s
/// in this widget's subtree.
///
/// Values specified here are used for `ListTile` properties that are not given
/// an explicit non-null value.
///
/// The `Drawer` widget specifies a tile theme for its children which sets
/// `style` to [`ListTileStyle::Drawer`].
///
/// The fields other than [`data`](Self::data) are Dart's obsolete constructor arguments,
/// read through the `effective_*` methods that stand for Dart's getters of the same names.
#[derive(Debug)]
pub struct ListTileTheme {
    pub key: Option<KeyRef>,
    /// Dart's `_data`: the configuration given to the constructor.
    pub data: Option<ListTileThemeData>,
    pub dense: Option<bool>,
    pub shape: Option<ShapeBorderRef>,
    pub style: Option<ListTileStyle>,
    pub selected_color: Option<Color>,
    pub icon_color: Option<AnyColor>,
    pub text_color: Option<AnyColor>,
    pub content_padding: Option<EdgeInsetsGeometry>,
    pub tile_color: Option<Color>,
    pub selected_tile_color: Option<Color>,
    pub enable_feedback: Option<bool>,
    pub mouse_cursor: Option<WidgetStatePropertyRef<Option<MouseCursorRef>>>,
    pub horizontal_title_gap: Option<f64>,
    pub min_vertical_padding: Option<f64>,
    pub min_leading_width: Option<f64>,
    pub control_affinity: Option<ListTileControlAffinity>,
    pub child: WidgetRef,
}

/// The setters of `ListTileTheme`'s obsolete constructor arguments, each of which Dart's
/// constructor asserts is not given with `data`.
macro_rules! obsolete_list_tile_theme_setters {
    ($($field:ident: $ty:ty => $name:literal),* $(,)?) => {
        $(
            #[doc = concat!("Dart `ListTileTheme(", $name, ":)`.")]
            pub fn $field(mut self, $field: impl Into<$ty>) -> ListTileTheme {
                debug_assert!(self.data.is_none());
                self.$field = Some($field.into());
                self
            }
        )*
    };
}

impl ListTileTheme {
    /// Creates a list tile theme that defines the color and style parameters for
    /// descendant `ListTile`s.
    ///
    /// Only the [`data`](Self::data) parameter should be used. The other parameters are
    /// redundant (are now obsolete) and will be deprecated in a future update.
    pub fn new<K>(child: impl IntoWidget<K>) -> ListTileTheme {
        ListTileTheme {
            key: None,
            data: None,
            dense: None,
            shape: None,
            style: None,
            selected_color: None,
            icon_color: None,
            text_color: None,
            content_padding: None,
            tile_color: None,
            selected_tile_color: None,
            enable_feedback: None,
            mouse_cursor: None,
            horizontal_title_gap: None,
            min_vertical_padding: None,
            min_leading_width: None,
            control_affinity: None,
            child: child.into_widget(),
        }
    }

    /// Dart `ListTileTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> ListTileTheme {
        self.key = Some(key);
        self
    }

    /// Dart `ListTileTheme(data:)`.
    pub fn data(mut self, data: ListTileThemeData) -> ListTileTheme {
        debug_assert!(
            self.shape.is_none()
                && self.selected_color.is_none()
                && self.icon_color.is_none()
                && self.text_color.is_none()
                && self.content_padding.is_none()
                && self.tile_color.is_none()
                && self.selected_tile_color.is_none()
                && self.enable_feedback.is_none()
                && self.mouse_cursor.is_none()
                && self.horizontal_title_gap.is_none()
                && self.min_vertical_padding.is_none()
                && self.min_leading_width.is_none()
                && self.control_affinity.is_none()
        );
        self.data = Some(data);
        self
    }

    /// Dart `ListTileTheme(dense:)`.
    pub fn dense(mut self, dense: bool) -> ListTileTheme {
        self.dense = Some(dense);
        self
    }

    /// Dart `ListTileTheme(style:)`.
    pub fn style(mut self, style: ListTileStyle) -> ListTileTheme {
        self.style = Some(style);
        self
    }

    obsolete_list_tile_theme_setters!(
        shape: ShapeBorderRef => "shape",
        selected_color: Color => "selectedColor",
        icon_color: AnyColor => "iconColor",
        text_color: AnyColor => "textColor",
        content_padding: EdgeInsetsGeometry => "contentPadding",
        tile_color: Color => "tileColor",
        selected_tile_color: Color => "selectedTileColor",
        enable_feedback: bool => "enableFeedback",
        mouse_cursor: WidgetStatePropertyRef<Option<MouseCursorRef>> => "mouseCursor",
        horizontal_title_gap: f64 => "horizontalTitleGap",
        min_vertical_padding: f64 => "minVerticalPadding",
        min_leading_width: f64 => "minLeadingWidth",
        control_affinity: ListTileControlAffinity => "controlAffinity",
    );

    /// The configuration of this theme: Dart's `data` getter.
    pub fn effective_data(&self) -> ListTileThemeData {
        match &self.data {
            Some(data) => data.clone(),
            None => ListTileThemeData {
                dense: self.dense,
                shape: self.shape.clone(),
                style: self.style,
                selected_color: self.selected_color,
                icon_color: self.icon_color.clone(),
                text_color: self.text_color.clone(),
                content_padding: self.content_padding,
                tile_color: self.tile_color,
                selected_tile_color: self.selected_tile_color,
                enable_feedback: self.enable_feedback,
                mouse_cursor: self.mouse_cursor.clone(),
                horizontal_title_gap: self.horizontal_title_gap,
                min_vertical_padding: self.min_vertical_padding,
                min_leading_width: self.min_leading_width,
                control_affinity: self.control_affinity,
                ..ListTileThemeData::default()
            },
        }
    }

    /// Overrides the default value of `ListTile::dense`.
    ///
    /// This property is obsolete: please use the [`data`](Self::data)
    /// [`ListTileThemeData::dense`] property instead.
    pub fn effective_dense(&self) -> Option<bool> {
        match &self.data {
            Some(data) => data.dense,
            None => self.dense,
        }
    }

    /// Overrides the default value of `ListTile::shape`.
    ///
    /// This property is obsolete: please use the [`data`](Self::data)
    /// [`ListTileThemeData::shape`] property instead.
    pub fn effective_shape(&self) -> Option<ShapeBorderRef> {
        match &self.data {
            Some(data) => data.shape.clone(),
            None => self.shape.clone(),
        }
    }

    /// Overrides the default value of `ListTile::style`.
    ///
    /// This property is obsolete: please use the [`data`](Self::data)
    /// [`ListTileThemeData::style`] property instead.
    pub fn effective_style(&self) -> Option<ListTileStyle> {
        match &self.data {
            Some(data) => data.style,
            None => self.style,
        }
    }

    /// Overrides the default value of `ListTile::selected_color`.
    ///
    /// This property is obsolete: please use the [`data`](Self::data)
    /// [`ListTileThemeData::selected_color`] property instead.
    pub fn effective_selected_color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.selected_color,
            None => self.selected_color,
        }
    }

    /// Overrides the default value of `ListTile::icon_color`.
    ///
    /// This property is obsolete: please use the [`data`](Self::data)
    /// [`ListTileThemeData::icon_color`] property instead.
    pub fn effective_icon_color(&self) -> Option<AnyColor> {
        match &self.data {
            Some(data) => data.icon_color.clone(),
            None => self.icon_color.clone(),
        }
    }

    /// Overrides the default value of `ListTile::text_color`.
    ///
    /// This property is obsolete: please use the [`data`](Self::data)
    /// [`ListTileThemeData::text_color`] property instead.
    pub fn effective_text_color(&self) -> Option<AnyColor> {
        match &self.data {
            Some(data) => data.text_color.clone(),
            None => self.text_color.clone(),
        }
    }

    /// Overrides the default value of `ListTile::content_padding`.
    ///
    /// This property is obsolete: please use the [`data`](Self::data)
    /// [`ListTileThemeData::content_padding`] property instead.
    pub fn effective_content_padding(&self) -> Option<EdgeInsetsGeometry> {
        match &self.data {
            Some(data) => data.content_padding,
            None => self.content_padding,
        }
    }

    /// Overrides the default value of `ListTile::tile_color`.
    ///
    /// This property is obsolete: please use the [`data`](Self::data)
    /// [`ListTileThemeData::tile_color`] property instead.
    pub fn effective_tile_color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.tile_color,
            None => self.tile_color,
        }
    }

    /// Overrides the default value of `ListTile::selected_tile_color`.
    ///
    /// This property is obsolete: please use the [`data`](Self::data)
    /// [`ListTileThemeData::selected_tile_color`] property instead.
    pub fn effective_selected_tile_color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.selected_tile_color,
            None => self.selected_tile_color,
        }
    }

    /// Overrides the default value of `ListTile::horizontal_title_gap`.
    ///
    /// This property is obsolete: please use the [`data`](Self::data)
    /// [`ListTileThemeData::horizontal_title_gap`] property instead.
    pub fn effective_horizontal_title_gap(&self) -> Option<f64> {
        match &self.data {
            Some(data) => data.horizontal_title_gap,
            None => self.horizontal_title_gap,
        }
    }

    /// Overrides the default value of `ListTile::min_vertical_padding`.
    ///
    /// This property is obsolete: please use the [`data`](Self::data)
    /// [`ListTileThemeData::min_vertical_padding`] property instead.
    pub fn effective_min_vertical_padding(&self) -> Option<f64> {
        match &self.data {
            Some(data) => data.min_vertical_padding,
            None => self.min_vertical_padding,
        }
    }

    /// Overrides the default value of `ListTile::min_leading_width`.
    ///
    /// This property is obsolete: please use the [`data`](Self::data)
    /// [`ListTileThemeData::min_leading_width`] property instead.
    pub fn effective_min_leading_width(&self) -> Option<f64> {
        match &self.data {
            Some(data) => data.min_leading_width,
            None => self.min_leading_width,
        }
    }

    /// Overrides the default value of `ListTile::enable_feedback`.
    ///
    /// This property is obsolete: please use the [`data`](Self::data)
    /// [`ListTileThemeData::enable_feedback`] property instead.
    pub fn effective_enable_feedback(&self) -> Option<bool> {
        match &self.data {
            Some(data) => data.enable_feedback,
            None => self.enable_feedback,
        }
    }

    /// Overrides the default value of `CheckboxListTile::control_affinity`
    /// or `ExpansionTile::control_affinity` or `SwitchListTile::control_affinity` or
    /// `RadioListTile::control_affinity`.
    ///
    /// This property is obsolete: please use the
    /// [`ListTileThemeData::control_affinity`] property instead.
    pub fn effective_control_affinity(&self) -> Option<ListTileControlAffinity> {
        match &self.data {
            Some(data) => data.control_affinity,
            None => self.control_affinity,
        }
    }

    /// The [`data`](Self::effective_data) property of the closest instance of this class that
    /// encloses the given context.
    ///
    /// If there is no enclosing [`ListTileTheme`] widget, then
    /// `ThemeData::list_tile_theme` is used (see `Theme::of`).
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// let theme = ListTileTheme::of(app, context);
    /// ```
    pub fn of(app: &mut App, context: BuildContext) -> ListTileThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<ListTileTheme>(app) {
            Some(result) => result.effective_data(),
            None => Theme::of(app, context).list_tile_theme.clone(),
        }
    }

    /// Creates a list tile theme that controls the color and style parameters for
    /// `ListTile`s, and merges in the current list tile theme, if any.
    ///
    /// Dart's optional named arguments are the setters of the [`ListTileThemeMerge`] this
    /// returns.
    pub fn merge<K>(child: impl IntoWidget<K>) -> ListTileThemeMerge {
        ListTileThemeMerge {
            key: None,
            given: ListTileThemeData::new(),
            child: child.into_widget(),
        }
    }
}

/// The arguments of Dart's `ListTileTheme.merge`, which becomes the widget it returns.
pub struct ListTileThemeMerge {
    key: Option<KeyRef>,
    /// The arguments given, each `None` where the parent's value is kept.
    given: ListTileThemeData,
    child: WidgetRef,
}

impl ListTileThemeMerge {
    /// Dart `ListTileTheme.merge(key:)`.
    pub fn key(mut self, key: KeyRef) -> ListTileThemeMerge {
        self.key = Some(key);
        self
    }

    every_list_tile_theme_data_setter!(ListTileThemeMerge, "ListTileTheme.merge", |this| this
        .given);
}

/// The kind tag of [`IntoWidget`] for a [`ListTileThemeMerge`].
pub struct ListTileThemeMergeKind;

impl IntoWidget<ListTileThemeMergeKind> for ListTileThemeMerge {
    fn into_widget(self) -> WidgetRef {
        Builder::new(move |app, context| {
            let parent = ListTileTheme::of(app, context);
            let given = &self.given;
            let theme = ListTileTheme::new(self.child.clone()).data(ListTileThemeData {
                dense: given.dense.or(parent.dense),
                shape: given.shape.clone().or(parent.shape),
                style: given.style.or(parent.style),
                selected_color: given.selected_color.or(parent.selected_color),
                icon_color: given.icon_color.clone().or(parent.icon_color),
                text_color: given.text_color.clone().or(parent.text_color),
                title_text_style: given.title_text_style.clone().or(parent.title_text_style),
                subtitle_text_style: given
                    .subtitle_text_style
                    .clone()
                    .or(parent.subtitle_text_style),
                leading_and_trailing_text_style: given
                    .leading_and_trailing_text_style
                    .clone()
                    .or(parent.leading_and_trailing_text_style),
                content_padding: given.content_padding.or(parent.content_padding),
                tile_color: given.tile_color.or(parent.tile_color),
                selected_tile_color: given.selected_tile_color.or(parent.selected_tile_color),
                enable_feedback: given.enable_feedback.or(parent.enable_feedback),
                horizontal_title_gap: given.horizontal_title_gap.or(parent.horizontal_title_gap),
                min_vertical_padding: given.min_vertical_padding.or(parent.min_vertical_padding),
                min_leading_width: given.min_leading_width.or(parent.min_leading_width),
                min_tile_height: given.min_tile_height.or(parent.min_tile_height),
                title_alignment: given.title_alignment.or(parent.title_alignment),
                mouse_cursor: given.mouse_cursor.clone().or(parent.mouse_cursor),
                visual_density: given.visual_density.or(parent.visual_density),
                control_affinity: given.control_affinity.or(parent.control_affinity),
                is_three_line: given.is_three_line.or(parent.is_three_line),
            });
            match &self.key {
                Some(key) => theme.key(key.clone()),
                None => theme,
            }
            .into_widget()
        })
        .into_widget()
    }
}

impl InheritedTheme for ListTileTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        ListTileTheme::new(child)
            .data(ListTileThemeData {
                dense: self.effective_dense(),
                shape: self.effective_shape(),
                style: self.effective_style(),
                selected_color: self.effective_selected_color(),
                icon_color: self.effective_icon_color(),
                text_color: self.effective_text_color(),
                content_padding: self.effective_content_padding(),
                tile_color: self.effective_tile_color(),
                selected_tile_color: self.effective_selected_tile_color(),
                enable_feedback: self.effective_enable_feedback(),
                horizontal_title_gap: self.effective_horizontal_title_gap(),
                min_vertical_padding: self.effective_min_vertical_padding(),
                min_leading_width: self.effective_min_leading_width(),
                is_three_line: self.data.as_ref().and_then(|data| data.is_three_line),
                ..ListTileThemeData::default()
            })
            .into_widget()
    }
}

impl InheritedWidget for ListTileTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &ListTileTheme) -> bool {
        self.effective_data() != old_widget.effective_data()
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::time::Duration;

    use waft_embedder::{FontStyle, Offset, Radius, Rect, Size};
    use waft_foundation::Listener;
    use waft_painting::{BorderRadius, EdgeInsets, RoundedRectangleBorder, StadiumBorder};
    use waft_services::SystemMouseCursors;
    use waft_widgets::{
        Center, Container, GlobalKey, ListView, SizedBox, StatefulBuilder, Text, WidgetState,
        WidgetStateColor, WidgetStateMouseCursor, WidgetStateProperty,
    };

    use waft_widgets::testing::{TestPointer, WidgetTester, test_widgets};

    use super::*;
    use crate::colors::Colors;
    use crate::drawer::Drawer;
    use crate::ink_well::InkWell;
    use crate::list_tile::ListTile;
    use crate::list_tile::tests::{
        THIRTEEN_LINES, avatar, ink_tile_color, key_rect, m2, m3, placeholder, pump_app, sized,
        style_color, test_icon, test_text, text_rect, tile_rect,
    };
    use crate::material::Material;
    use crate::scaffold::{Scaffold, ScaffoldState};
    use crate::test_support::material_app;
    use crate::theme_data::ThemeData;

    /// Dart's `WidgetStateMouseCursor.clickable` as a `WidgetStateProperty<MouseCursor?>`.
    fn clickable() -> WidgetStatePropertyRef<Option<MouseCursorRef>> {
        <dyn WidgetStateProperty<Option<MouseCursorRef>>>::resolve_with(|states| {
            Some(WidgetStateMouseCursor::clickable().resolve(states))
        })
    }

    /// Dart's `WidgetStateMouseCursor.textable` as a `WidgetStateProperty<MouseCursor?>`.
    fn textable() -> WidgetStatePropertyRef<Option<MouseCursorRef>> {
        <dyn WidgetStateProperty<Option<MouseCursorRef>>>::resolve_with(|states| {
            Some(WidgetStateMouseCursor::textable().resolve(states))
        })
    }

    #[test]
    fn list_tile_theme_data_copy_with_equality_basics() {
        assert_eq!(
            ListTileThemeData::new(),
            ListTileThemeData::new().copy_with()
        );
    }

    #[test]
    fn list_tile_theme_data_lerp_special_cases() {
        assert_eq!(ListTileThemeData::lerp(None, None, 0.0), None);
        let data = ListTileThemeData::new();
        // Dart's `identical`: the value comes back as it was.
        assert_eq!(
            ListTileThemeData::lerp(Some(&data), Some(&data), 0.5),
            Some(data.clone())
        );
    }

    #[test]
    fn list_tile_theme_data_defaults() {
        let theme_data = ListTileThemeData::new();
        assert_eq!(theme_data.dense, None);
        assert!(theme_data.shape.is_none());
        assert_eq!(theme_data.style, None);
        assert_eq!(theme_data.selected_color, None);
        assert_eq!(theme_data.icon_color, None);
        assert_eq!(theme_data.text_color, None);
        assert_eq!(theme_data.title_text_style, None);
        assert_eq!(theme_data.subtitle_text_style, None);
        assert_eq!(theme_data.leading_and_trailing_text_style, None);
        assert_eq!(theme_data.content_padding, None);
        assert_eq!(theme_data.tile_color, None);
        assert_eq!(theme_data.selected_tile_color, None);
        assert_eq!(theme_data.horizontal_title_gap, None);
        assert_eq!(theme_data.min_vertical_padding, None);
        assert_eq!(theme_data.min_leading_width, None);
        assert_eq!(theme_data.min_tile_height, None);
        assert_eq!(theme_data.enable_feedback, None);
        assert!(theme_data.mouse_cursor.is_none());
        assert_eq!(theme_data.visual_density, None);
        assert_eq!(theme_data.title_alignment, None);
        assert_eq!(theme_data.is_three_line, None);
    }

    /// The theme a `ListTileTheme` gives its subtree, read in a builder under it.
    fn theme_under(
        tester: &mut WidgetTester,
        theme: impl FnOnce(WidgetRef) -> ListTileTheme,
    ) -> ListTileThemeData {
        let seen = Rc::new(RefCell::new(None));
        pump_app(tester, m3, |_app| {
            let seen = Rc::clone(&seen);
            let probe = Builder::new(move |app, context| {
                *seen.borrow_mut() = Some(ListTileTheme::of(app, context));
                SizedBox::new().into_widget()
            });
            Material::new()
                .child(theme(Center::new().child(probe).into_widget()))
                .into_widget()
        });
        seen.borrow_mut().take().expect("built")
    }

    #[test]
    fn list_tile_theme_backwards_compatibility_constructor() {
        test_widgets(|tester| {
            let mouse_cursor = clickable();
            let theme = theme_under(tester, |child| {
                ListTileTheme::new(child)
                    .dense(true)
                    .shape(StadiumBorder::default())
                    .style(ListTileStyle::Drawer)
                    .selected_color(Color::new(0x00000001))
                    .icon_color(Color::new(0x00000002))
                    .text_color(Color::new(0x00000003))
                    .content_padding(EdgeInsets::all(100.0))
                    .tile_color(Color::new(0x00000004))
                    .selected_tile_color(Color::new(0x00000005))
                    .horizontal_title_gap(200.0)
                    .min_vertical_padding(300.0)
                    .min_leading_width(400.0)
                    .enable_feedback(true)
                    .mouse_cursor(Rc::clone(&mouse_cursor))
            });

            assert_eq!(theme.dense, Some(true));
            assert_eq!(
                theme.shape,
                Some(ShapeBorderRef::new(StadiumBorder::default()))
            );
            assert_eq!(theme.style, Some(ListTileStyle::Drawer));
            assert_eq!(theme.selected_color, Some(Color::new(0x00000001)));
            assert_eq!(
                theme.icon_color,
                Some(AnyColor::new(Color::new(0x00000002)))
            );
            assert_eq!(
                theme.text_color,
                Some(AnyColor::new(Color::new(0x00000003)))
            );
            assert_eq!(theme.content_padding, Some(EdgeInsets::all(100.0).into()));
            assert_eq!(theme.tile_color, Some(Color::new(0x00000004)));
            assert_eq!(theme.selected_tile_color, Some(Color::new(0x00000005)));
            assert_eq!(theme.horizontal_title_gap, Some(200.0));
            assert_eq!(theme.min_vertical_padding, Some(300.0));
            assert_eq!(theme.min_leading_width, Some(400.0));
            assert_eq!(theme.enable_feedback, Some(true));
            assert!(Rc::ptr_eq(
                theme.mouse_cursor.as_ref().expect("a mouse cursor"),
                &mouse_cursor
            ));
        });
    }

    /// The tester of "ListTileTheme", and the colours its `TestIcon`s and `TestText`s and the
    /// theme its builder saw in the last frame.
    struct ThemedCase<'a> {
        tester: &'a mut WidgetTester,
        leading: Rc<Cell<Option<Color>>>,
        trailing: Rc<Cell<Option<Color>>>,
        title: Rc<RefCell<Option<TextStyle>>>,
        subtitle: Rc<RefCell<Option<TextStyle>>>,
        theme: Rc<RefCell<Option<Rc<ThemeData>>>>,
        leading_key: GlobalKey,
        trailing_key: GlobalKey,
        list_tile_key: GlobalKey,
    }

    #[derive(Default)]
    struct ThemedArgs {
        enabled: Option<bool>,
        dense: bool,
        selected: bool,
        shape: Option<ShapeBorderRef>,
        selected_color: Option<Color>,
        icon_color: Option<Color>,
        text_color: Option<Color>,
    }

    impl<'a> ThemedCase<'a> {
        fn new(tester: &'a mut WidgetTester) -> ThemedCase<'a> {
            ThemedCase {
                tester,
                leading: Rc::new(Cell::new(None)),
                trailing: Rc::new(Cell::new(None)),
                title: Rc::new(RefCell::new(None)),
                subtitle: Rc::new(RefCell::new(None)),
                theme: Rc::new(RefCell::new(None)),
                leading_key: GlobalKey::new(),
                trailing_key: GlobalKey::new(),
                list_tile_key: GlobalKey::new(),
            }
        }

        /// Dart's `tester.pumpWidget(buildFrame(...))`.
        fn pump_frame(&mut self, args: ThemedArgs) {
            let mut data = ListTileThemeData::new()
                .dense(args.dense)
                .min_vertical_padding(25.0)
                .mouse_cursor(
                    <dyn WidgetStateProperty<Option<MouseCursorRef>>>::resolve_with(|states| {
                        if states.contains(&WidgetState::Disabled) {
                            return Some(SystemMouseCursors::FORBIDDEN.into());
                        }

                        Some(SystemMouseCursors::CLICK.into())
                    }),
                )
                .visual_density(VisualDensity::COMPACT)
                .title_alignment(ListTileTitleAlignment::Bottom);
            data.shape = args.shape;
            data.selected_color = args.selected_color;
            data.icon_color = args.icon_color.map(AnyColor::new);
            data.text_color = args.text_color.map(AnyColor::new);
            let (leading, trailing, title, subtitle, theme) = (
                Rc::clone(&self.leading),
                Rc::clone(&self.trailing),
                Rc::clone(&self.title),
                Rc::clone(&self.subtitle),
                Rc::clone(&self.theme),
            );
            let (leading_key, trailing_key, list_tile_key) = (
                self.leading_key.clone(),
                self.trailing_key.clone(),
                self.list_tile_key.clone(),
            );
            let home = Material::new()
                .child(
                    Center::new().child(
                        ListTileTheme::new(Builder::new(move |app, context| {
                            *theme.borrow_mut() = Some(Theme::of(app, context));
                            ListTile::new()
                                .key(list_tile_key.clone().into())
                                .enabled(args.enabled.unwrap_or(true))
                                .selected(args.selected)
                                .leading(
                                    SizedBox::new()
                                        .key(leading_key.clone().into())
                                        .child(test_icon(&leading)),
                                )
                                .trailing(
                                    SizedBox::new()
                                        .key(trailing_key.clone().into())
                                        .child(test_icon(&trailing)),
                                )
                                .title(test_text("title", &title))
                                .subtitle(test_text("subtitle", &subtitle))
                                .into_widget()
                        }))
                        .data(data),
                    ),
                )
                .into_widget();
            let theme = m2(&self.tester.cell.borrow());
            self.tester.pump_widget(material_app(theme, home));
        }

        fn colors(&self) -> [Option<Color>; 4] {
            [
                self.leading.get(),
                self.trailing.get(),
                style_color(&self.title),
                style_color(&self.subtitle),
            ]
        }

        fn theme(&self) -> Rc<ThemeData> {
            self.theme.borrow().clone().expect("built")
        }
    }

    /// "ListTileTheme". `TestIcon` sits in a keyed box: Dart keys the icon itself.
    #[test]
    fn list_tile_theme() {
        test_widgets(|tester| {
            const GREEN: Color = Color::new(0xFF00FF00);
            const RED: Color = Color::new(0xFFFF0000);
            let rounded_shape = RoundedRectangleBorder {
                border_radius: BorderRadius::all(Radius::circular(4.0)).into(),
                ..RoundedRectangleBorder::default()
            };
            let mut case = ThemedCase::new(tester);

            // A selected ListTile's leading, trailing, and text get the primary color by default
            case.pump_frame(ThemedArgs {
                selected: true,
                ..ThemedArgs::default()
            });
            case.tester.pump_by(Duration::from_millis(300)); // DefaultTextStyle changes animate
            assert_eq!(case.colors(), [Some(case.theme().primary_color); 4]);

            // A selected ListTile's leading, trailing, and text get the ListTileTheme's selectedColor
            case.pump_frame(ThemedArgs {
                selected: true,
                selected_color: Some(GREEN),
                ..ThemedArgs::default()
            });
            case.tester.pump_by(Duration::from_millis(300)); // DefaultTextStyle changes animate
            assert_eq!(case.colors(), [Some(GREEN); 4]);

            // An unselected ListTile's leading and trailing get the ListTileTheme's iconColor
            // An unselected ListTile's title texts get the ListTileTheme's textColor
            case.pump_frame(ThemedArgs {
                icon_color: Some(RED),
                text_color: Some(GREEN),
                ..ThemedArgs::default()
            });
            case.tester.pump_by(Duration::from_millis(300)); // DefaultTextStyle changes animate
            assert_eq!(
                case.colors(),
                [Some(RED), Some(RED), Some(GREEN), Some(GREEN)]
            );

            // If the item is disabled it's rendered with the theme's disabled color.
            case.pump_frame(ThemedArgs {
                enabled: Some(false),
                ..ThemedArgs::default()
            });
            case.tester.pump_by(Duration::from_millis(300)); // DefaultTextStyle changes animate
            assert_eq!(case.colors(), [Some(case.theme().disabled_color); 4]);

            // If the item is disabled it's rendered with the theme's disabled color.
            // Even if it's selected.
            case.pump_frame(ThemedArgs {
                enabled: Some(false),
                selected: true,
                ..ThemedArgs::default()
            });
            case.tester.pump_by(Duration::from_millis(300)); // DefaultTextStyle changes animate
            assert_eq!(case.colors(), [Some(case.theme().disabled_color); 4]);

            // A selected ListTile's InkWell gets the ListTileTheme's shape
            case.pump_frame(ThemedArgs {
                selected: true,
                shape: Some(ShapeBorderRef::new(rounded_shape)),
                ..ThemedArgs::default()
            });
            let ink_well = case.tester.find_by_type::<InkWell>()[0];
            let custom_border = case.tester.widget(ink_well, |ink_well: &InkWell| {
                ink_well.ink_response.custom_border.clone()
            });
            assert_eq!(custom_border, Some(ShapeBorderRef::new(rounded_shape)));

            // Cursor updates when hovering disabled ListTile
            case.pump_frame(ThemedArgs {
                enabled: Some(false),
                ..ThemedArgs::default()
            });
            let list_tile = text_rect(case.tester, "title").center();
            let mut gesture = TestPointer::mouse(1, 0);
            case.tester.add_pointer(&mut gesture, Offset::ZERO);
            case.tester.move_to(&mut gesture, list_tile, false);
            case.tester.pump_and_settle();
            let forbidden: MouseCursorRef = SystemMouseCursors::FORBIDDEN.into();
            assert!(*case.tester.active_cursor(1) == *forbidden);

            // VisualDensity is respected
            assert_eq!(
                key_rect(case.tester, &case.list_tile_key).size(),
                Size::new(800.0, 80.0)
            );

            // titleAlignment is respected.
            let title_offset = text_rect(case.tester, "title").top_left();
            let leading_offset = key_rect(case.tester, &case.leading_key).top_left();
            let trailing_offset = key_rect(case.tester, &case.trailing_key).top_right();
            assert_eq!(leading_offset.dy() - title_offset.dy(), 6.0);
            assert_eq!(trailing_offset.dy() - title_offset.dy(), 6.0);
        });
    }

    #[test]
    fn list_tile_theme_colors_are_applied_to_leading_and_trailing_text_widgets() {
        test_widgets(|tester| {
            let selected_color = Colors::ORANGE.primary();
            let default_color = Colors::BLACK;
            let mut colors = |enabled: bool, selected: bool| {
                let leading = Rc::new(RefCell::new(None));
                let trailing = Rc::new(RefCell::new(None));
                let title = Rc::new(RefCell::new(None));
                let theme = Rc::new(RefCell::new(None));
                pump_app(tester, m3, |_app| {
                    let (leading, trailing, title, theme) = (
                        Rc::clone(&leading),
                        Rc::clone(&trailing),
                        Rc::clone(&title),
                        Rc::clone(&theme),
                    );
                    Material::new()
                        .child(
                            Center::new().child(
                                ListTileTheme::new(Builder::new(move |app, context| {
                                    *theme.borrow_mut() = Some(Theme::of(app, context));
                                    ListTile::new()
                                        .enabled(enabled)
                                        .selected(selected)
                                        .leading(test_text("leading", &leading))
                                        .title(test_text("title", &title))
                                        .trailing(test_text("trailing", &trailing))
                                        .into_widget()
                                }))
                                .data(
                                    ListTileThemeData::new()
                                        .selected_color(selected_color)
                                        .text_color(default_color),
                                ),
                            ),
                        )
                        .into_widget()
                });
                // Wait for text color to animate.
                tester.pump_and_settle();
                let theme: Rc<ThemeData> = theme.borrow_mut().take().expect("built");
                (style_color(&leading), style_color(&trailing), theme)
            };

            let (leading, trailing, _) = colors(true, false);
            // Enabled color should use ListTileTheme.textColor.
            assert_eq!(leading, Some(default_color));
            assert_eq!(trailing, Some(default_color));

            let (leading, trailing, _) = colors(true, true);
            // Selected color should use ListTileTheme.selectedColor.
            assert_eq!(leading, Some(selected_color));
            assert_eq!(trailing, Some(selected_color));

            let (leading, trailing, theme) = colors(false, false);
            // Disabled color should be ThemeData.disabledColor.
            assert_eq!(leading, Some(theme.disabled_color));
            assert_eq!(trailing, Some(theme.disabled_color));
        });
    }

    fn italic(font_size: f64, color: u32) -> TextStyle {
        TextStyle::new()
            .font_size(font_size)
            .color(Color::new(color))
            .font_style(FontStyle::Italic)
    }

    /// The default text styles of the four slots under `theme`, for `tile`.
    fn slot_styles(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        tile: ListTile,
    ) -> [TextStyle; 4] {
        let seen: [Rc<RefCell<Option<TextStyle>>>; 4] =
            std::array::from_fn(|_| Rc::new(RefCell::new(None)));
        pump_app(tester, theme, |_app| {
            Material::new()
                .child(
                    Center::new().child(
                        tile.leading(test_text("leading", &seen[0]))
                            .title(test_text("title", &seen[1]))
                            .subtitle(test_text("subtitle", &seen[2]))
                            .trailing(test_text("trailing", &seen[3])),
                    ),
                )
                .into_widget()
        });
        seen.map(|seen| seen.borrow_mut().take().expect("built"))
    }

    fn assert_slot_styles(styles: &[TextStyle; 4], expected: [&TextStyle; 4]) {
        for (style, expected) in styles.iter().zip(expected) {
            assert_eq!(style.font_size, expected.font_size);
            assert_eq!(style.color, expected.color);
            assert_eq!(style.font_style, expected.font_style);
        }
    }

    /// The text-style cases' four styles, Dart's `titleTextStyle`, `subtitleTextStyle` and
    /// `leadingAndTrailingTextStyle`.
    fn case_styles() -> (TextStyle, TextStyle, TextStyle) {
        (
            italic(23.0, 0xffff0000),
            italic(20.0, 0xff00ff00),
            italic(18.0, 0xff0000ff),
        )
    }

    fn respects_theme_text_styles(tester: &mut WidgetTester, use_material3: bool) {
        let (title, subtitle, leading_and_trailing) = case_styles();
        let theme_data = ListTileThemeData::new()
            .title_text_style(title.clone())
            .subtitle_text_style(subtitle.clone())
            .leading_and_trailing_text_style(leading_and_trailing.clone());
        let styles = slot_styles(
            tester,
            move |app| {
                ThemeData::builder()
                    .use_material3(use_material3)
                    .list_tile_theme(theme_data)
                    .build(app)
            },
            ListTile::new(),
        );
        assert_slot_styles(
            &styles,
            [
                &leading_and_trailing,
                &title,
                &subtitle,
                &leading_and_trailing,
            ],
        );
    }

    #[test]
    fn material3_list_tile_respects_list_tile_theme_text_styles() {
        test_widgets(|tester| respects_theme_text_styles(tester, true));
    }

    #[test]
    fn material2_list_tile_respects_list_tile_theme_text_styles() {
        test_widgets(|tester| respects_theme_text_styles(tester, false));
    }

    fn theme_text_styles_are_overridden(tester: &mut WidgetTester, use_material3: bool) {
        let (title, subtitle, leading_and_trailing) = case_styles();
        let styles = slot_styles(
            tester,
            move |app| {
                ThemeData::builder()
                    .use_material3(use_material3)
                    .list_tile_theme(
                        ListTileThemeData::new()
                            .title_text_style(TextStyle::new().font_size(20.0))
                            .subtitle_text_style(TextStyle::new().font_size(17.5))
                            .leading_and_trailing_text_style(TextStyle::new().font_size(15.0)),
                    )
                    .build(app)
            },
            ListTile::new()
                .title_text_style(title.clone())
                .subtitle_text_style(subtitle.clone())
                .leading_and_trailing_text_style(leading_and_trailing.clone()),
        );
        assert_slot_styles(
            &styles,
            [
                &leading_and_trailing,
                &title,
                &subtitle,
                &leading_and_trailing,
            ],
        );
    }

    #[test]
    fn material3_list_tile_text_styles_are_overridden_by_list_tile_properties() {
        test_widgets(|tester| theme_text_styles_are_overridden(tester, true));
    }

    #[test]
    fn material2_list_tile_text_styles_are_overridden_by_list_tile_properties() {
        test_widgets(|tester| theme_text_styles_are_overridden(tester, false));
    }

    /// The colour of the first rect the material's ink features draw.
    fn tile_colors_before_and_after_tap(
        tester: &mut WidgetTester,
        theme_data: ListTileThemeData,
        tile: impl Fn() -> ListTile + 'static,
    ) -> (Option<Color>, Option<Color>) {
        let is_selected = Rc::new(Cell::new(false));
        pump_app(tester, m3, move |_app| {
            Material::new()
                .child(
                    ListTileTheme::new(Center::new().child(StatefulBuilder::new(
                        move |_app, _context, set_state| {
                            let toggled = Rc::clone(&is_selected);
                            tile()
                                .selected(is_selected.get())
                                .on_tap(Listener::new(move |app| {
                                    let toggled = Rc::clone(&toggled);
                                    set_state(app, &mut |_app| toggled.set(!toggled.get()));
                                }))
                                .title(Text::new("Title"))
                                .into_widget()
                        },
                    )))
                    .data(theme_data),
                )
                .into_widget()
        });
        let before = ink_tile_color(tester);
        // Tap on tile to change isSelected.
        let center = tile_rect(tester, 0).center();
        tester.tap(center, 1);
        tester.pump_and_settle();
        (before, ink_tile_color(tester))
    }

    /// "ListTile respects ListTileTheme's tileColor & selectedTileColor": the colours are
    /// read from the tile's `Ink` decoration, whose rect Dart's `paints` matches.
    #[test]
    fn list_tile_respects_list_tile_theme_tile_color_and_selected_tile_color() {
        test_widgets(|tester| {
            let theme = ListTileThemeData::new()
                .tile_color(Colors::GREEN.shade500())
                .selected_tile_color(Colors::RED.shade500());
            assert_eq!(
                tile_colors_before_and_after_tap(tester, theme, ListTile::new),
                (Some(Colors::GREEN.shade500()), Some(Colors::RED.shade500()))
            );
        });
    }

    /// "ListTileTheme's tileColor & selectedTileColor are overridden by ListTile properties".
    #[test]
    fn list_tile_theme_tile_colors_are_overridden_by_list_tile_properties() {
        test_widgets(|tester| {
            let tile_color = Colors::GREEN.shade500();
            let selected_tile_color = Colors::RED.shade500();
            let theme = ListTileThemeData::new()
                .selected_tile_color(Colors::GREEN.primary())
                .tile_color(Colors::RED.primary());
            assert_eq!(
                tile_colors_before_and_after_tap(tester, theme, move || {
                    ListTile::new()
                        .tile_color(tile_color)
                        .selected_tile_color(selected_tile_color)
                }),
                (Some(tile_color), Some(selected_tile_color))
            );
        });
    }

    fn state_color() -> WidgetStateColor {
        WidgetStateColor::resolve_with(|states| {
            if states.contains(&WidgetState::Disabled) {
                return Colors::RED.primary();
            }
            if states.contains(&WidgetState::Selected) {
                return Colors::GREEN.primary();
            }
            Colors::BLUE.primary()
        })
    }

    /// "ListTile uses ListTileTheme shape in a drawer".
    #[test]
    fn list_tile_uses_list_tile_theme_shape_in_a_drawer() {
        test_widgets(|tester| {
            // This is a regression test for https://github.com/flutter/flutter/issues/106303

            let scaffold_key = GlobalKey::new();
            let shape_border = RoundedRectangleBorder {
                border_radius: BorderRadius::all(Radius::circular(20.0)).into(),
                ..RoundedRectangleBorder::default()
            };

            pump_app(
                tester,
                |app| {
                    ThemeData::builder()
                        .list_tile_theme(ListTileThemeData::new().shape(shape_border))
                        .build(app)
                },
                |_app| {
                    Scaffold::new()
                        .key(scaffold_key.clone().into())
                        .drawer(Drawer::new().child(ListTile::new()))
                        .body(Container::new())
                        .into_widget()
                },
            );
            tester.pump_and_settle();

            let scaffold_state = scaffold_key
                .current_state::<ScaffoldState>(&tester.cell.borrow_mut())
                .expect("a mounted scaffold");
            scaffold_state.open_drawer(&mut tester.cell.borrow_mut());
            // Start drawer animation.
            tester.pump();

            let list_tile = tester.find_by_type::<ListTile>()[0];
            let ink_well = tester
                .find_by_type::<InkWell>()
                .into_iter()
                .find(|ink_well| tester.ancestors(*ink_well).contains(&list_tile))
                .expect("the tile's InkWell");
            let ink_well_border = tester.widget(ink_well, |ink_well: &InkWell| {
                ink_well.ink_response.custom_border.clone()
            });
            // Test shape.
            assert_eq!(ink_well_border, Some(ShapeBorderRef::new(shape_border)));
        });
    }

    #[test]
    fn list_tile_respects_widget_state_color_list_tile_theme_text_color() {
        test_widgets(|tester| {
            let mut title_color = |enabled: bool, selected: bool| {
                pump_app(
                    tester,
                    |app| {
                        ThemeData::builder()
                            .list_tile_theme(ListTileThemeData::new().text_color(state_color()))
                            .build(app)
                    },
                    |_app| {
                        Material::new()
                            .child(
                                Center::new().child(
                                    ListTile::new()
                                        .enabled(enabled)
                                        .selected(selected)
                                        .title(Text::new("title"))
                                        .subtitle(Text::new("subtitle")),
                                ),
                            )
                            .into_widget()
                    },
                );
                tester.pump_and_settle();
                tester.text_color("title")
            };
            // Test disabled state.
            assert_eq!(title_color(false, false), Colors::RED.primary());
            // Test enabled state.
            assert_eq!(title_color(true, false), Colors::BLUE.primary());
            // Test selected state.
            assert_eq!(title_color(true, true), Colors::GREEN.primary());
        });
    }

    #[test]
    fn list_tile_respects_widget_state_color_list_tile_theme_icon_color() {
        test_widgets(|tester| {
            let mut icon_color = |enabled: bool, selected: bool| {
                let seen = Rc::new(Cell::new(None));
                pump_app(
                    tester,
                    |app| {
                        ThemeData::builder()
                            .list_tile_theme(ListTileThemeData::new().icon_color(state_color()))
                            .build(app)
                    },
                    |_app| {
                        Material::new()
                            .child(
                                Center::new().child(
                                    ListTile::new()
                                        .enabled(enabled)
                                        .selected(selected)
                                        .leading(test_icon(&seen)),
                                ),
                            )
                            .into_widget()
                    },
                );
                tester.pump_and_settle();
                seen.get()
            };
            // Test disabled state.
            assert_eq!(icon_color(false, false), Some(Colors::RED.primary()));
            // Test enabled state.
            assert_eq!(icon_color(true, false), Some(Colors::BLUE.primary()));
            // Test selected state.
            assert_eq!(icon_color(true, true), Some(Colors::GREEN.primary()));
        });
    }

    #[test]
    fn list_tile_theme_data_copy_with_overrides_all_properties() {
        // This is a regression test for https://github.com/flutter/flutter/issues/119734
        let original = ListTileThemeData::new()
            .dense(true)
            .shape(StadiumBorder::default())
            .style(ListTileStyle::Drawer)
            .selected_color(Color::new(0x00000001))
            .icon_color(Color::new(0x00000002))
            .text_color(Color::new(0x00000003))
            .title_text_style(TextStyle::new().color(Color::new(0x00000004)))
            .subtitle_text_style(TextStyle::new().color(Color::new(0x00000005)))
            .leading_and_trailing_text_style(TextStyle::new().color(Color::new(0x00000006)))
            .content_padding(EdgeInsets::all(100.0))
            .tile_color(Color::new(0x00000007))
            .selected_tile_color(Color::new(0x00000008))
            .horizontal_title_gap(200.0)
            .min_vertical_padding(300.0)
            .min_leading_width(400.0)
            .min_tile_height(30.0)
            .enable_feedback(true)
            .title_alignment(ListTileTitleAlignment::Bottom)
            .is_three_line(true);

        let copy = original
            .copy_with()
            .dense(false)
            .shape(RoundedRectangleBorder::default())
            .style(ListTileStyle::List)
            .selected_color(Color::new(0x00000009))
            .icon_color(Color::new(0x0000000A))
            .text_color(Color::new(0x0000000B))
            .title_text_style(TextStyle::new().color(Color::new(0x0000000C)))
            .subtitle_text_style(TextStyle::new().color(Color::new(0x0000000D)))
            .leading_and_trailing_text_style(TextStyle::new().color(Color::new(0x0000000E)))
            .content_padding(EdgeInsets::all(500.0))
            .tile_color(Color::new(0x0000000F))
            .selected_tile_color(Color::new(0x00000010))
            .horizontal_title_gap(600.0)
            .min_vertical_padding(700.0)
            .min_leading_width(800.0)
            .min_tile_height(80.0)
            .enable_feedback(false)
            .title_alignment(ListTileTitleAlignment::Top)
            .is_three_line(false);

        assert_eq!(copy.dense, Some(false));
        assert_eq!(
            copy.shape,
            Some(ShapeBorderRef::new(RoundedRectangleBorder::default()))
        );
        assert_eq!(copy.style, Some(ListTileStyle::List));
        assert_eq!(copy.selected_color, Some(Color::new(0x00000009)));
        assert_eq!(copy.icon_color, Some(AnyColor::new(Color::new(0x0000000A))));
        assert_eq!(copy.text_color, Some(AnyColor::new(Color::new(0x0000000B))));
        assert_eq!(
            copy.title_text_style,
            Some(TextStyle::new().color(Color::new(0x0000000C)))
        );
        assert_eq!(
            copy.subtitle_text_style,
            Some(TextStyle::new().color(Color::new(0x0000000D)))
        );
        assert_eq!(
            copy.leading_and_trailing_text_style,
            Some(TextStyle::new().color(Color::new(0x0000000E)))
        );
        assert_eq!(copy.content_padding, Some(EdgeInsets::all(500.0).into()));
        assert_eq!(copy.tile_color, Some(Color::new(0x0000000F)));
        assert_eq!(copy.selected_tile_color, Some(Color::new(0x00000010)));
        assert_eq!(copy.horizontal_title_gap, Some(600.0));
        assert_eq!(copy.min_vertical_padding, Some(700.0));
        assert_eq!(copy.min_leading_width, Some(800.0));
        assert_eq!(copy.min_tile_height, Some(80.0));
        assert_eq!(copy.enable_feedback, Some(false));
        assert_eq!(copy.title_alignment, Some(ListTileTitleAlignment::Top));
        assert_eq!(copy.is_three_line, Some(false));
    }

    #[test]
    fn list_tile_theme_title_alignment_is_overridden_by_list_tile_title_alignment() {
        test_widgets(|tester| {
            let leading_key = GlobalKey::new();
            let trailing_key = GlobalKey::new();
            pump_app(
                tester,
                |app| {
                    ThemeData::builder()
                        .list_tile_theme(
                            ListTileThemeData::new()
                                .title_alignment(ListTileTitleAlignment::Center),
                        )
                        .build(app)
                },
                |_app| {
                    Material::new()
                        .child(
                            Center::new().child(
                                ListTile::new()
                                    .title_alignment(ListTileTitleAlignment::Top)
                                    .leading(sized(&leading_key, 24.0, 24.0))
                                    .title(Text::new("\nHeadline Text\n"))
                                    .subtitle(Text::new("\nSupporting Text\n"))
                                    .trailing(sized(&trailing_key, 24.0, 24.0)),
                            ),
                        )
                        .into_widget()
                },
            );
            let tile_offset = tile_rect(tester, 0).top_left();
            let leading_offset = key_rect(tester, &leading_key).top_left();
            let trailing_offset = key_rect(tester, &trailing_key).top_right();
            assert_eq!(leading_offset.dy() - tile_offset.dy(), 8.0);
            assert_eq!(trailing_offset.dy() - tile_offset.dy(), 8.0);
        });
    }

    #[test]
    fn list_tile_theme_merge_supports_all_properties() {
        test_widgets(|tester| {
            let parent_mouse_cursor = textable();
            let mouse_cursor = clickable();
            let seen = Rc::new(RefCell::new(None));
            pump_app(
                tester,
                {
                    let parent_mouse_cursor = Rc::clone(&parent_mouse_cursor);
                    move |app| {
                        ThemeData::builder()
                            .list_tile_theme(
                                ListTileThemeData::new()
                                    .dense(true)
                                    .shape(StadiumBorder::default())
                                    .style(ListTileStyle::Drawer)
                                    .selected_color(Color::new(0x00000001))
                                    .icon_color(Color::new(0x00000002))
                                    .text_color(Color::new(0x00000003))
                                    .title_text_style(
                                        TextStyle::new().color(Color::new(0x00000004)),
                                    )
                                    .subtitle_text_style(
                                        TextStyle::new().color(Color::new(0x00000005)),
                                    )
                                    .leading_and_trailing_text_style(
                                        TextStyle::new().color(Color::new(0x00000006)),
                                    )
                                    .content_padding(EdgeInsets::all(100.0))
                                    .tile_color(Color::new(0x00000007))
                                    .selected_tile_color(Color::new(0x00000008))
                                    .horizontal_title_gap(200.0)
                                    .min_vertical_padding(300.0)
                                    .min_leading_width(400.0)
                                    .min_tile_height(30.0)
                                    .enable_feedback(true)
                                    .title_alignment(ListTileTitleAlignment::Bottom)
                                    .mouse_cursor(parent_mouse_cursor)
                                    .visual_density(VisualDensity::COMFORTABLE)
                                    .is_three_line(true),
                            )
                            .build(app)
                    }
                },
                {
                    let mouse_cursor = Rc::clone(&mouse_cursor);
                    let seen = Rc::clone(&seen);
                    move |_app| {
                        let probe = Builder::new(move |app, context| {
                            *seen.borrow_mut() = Some(ListTileTheme::of(app, context));
                            ListTile::new().into_widget()
                        });
                        Material::new()
                            .child(
                                Center::new().child(
                                    ListTileTheme::merge(probe)
                                        .dense(false)
                                        .shape(RoundedRectangleBorder::default())
                                        .style(ListTileStyle::List)
                                        .selected_color(Color::new(0x00000009))
                                        .icon_color(Color::new(0x0000000A))
                                        .text_color(Color::new(0x0000000B))
                                        .title_text_style(
                                            TextStyle::new().color(Color::new(0x0000000C)),
                                        )
                                        .subtitle_text_style(
                                            TextStyle::new().color(Color::new(0x0000000D)),
                                        )
                                        .leading_and_trailing_text_style(
                                            TextStyle::new().color(Color::new(0x0000000E)),
                                        )
                                        .content_padding(EdgeInsets::all(500.0))
                                        .tile_color(Color::new(0x0000000F))
                                        .selected_tile_color(Color::new(0x00000010))
                                        .horizontal_title_gap(600.0)
                                        .min_vertical_padding(700.0)
                                        .min_leading_width(800.0)
                                        .min_tile_height(80.0)
                                        .enable_feedback(false)
                                        .title_alignment(ListTileTitleAlignment::Top)
                                        .mouse_cursor(mouse_cursor)
                                        .visual_density(VisualDensity::COMPACT)
                                        .is_three_line(false),
                                ),
                            )
                            .into_widget()
                    }
                },
            );
            let _ = tester;
            let theme: ListTileThemeData = seen.borrow_mut().take().expect("built");
            assert_eq!(theme.dense, Some(false));
            assert_eq!(
                theme.shape,
                Some(ShapeBorderRef::new(RoundedRectangleBorder::default()))
            );
            assert_eq!(theme.style, Some(ListTileStyle::List));
            assert_eq!(theme.selected_color, Some(Color::new(0x00000009)));
            assert_eq!(
                theme.icon_color,
                Some(AnyColor::new(Color::new(0x0000000A)))
            );
            assert_eq!(
                theme.text_color,
                Some(AnyColor::new(Color::new(0x0000000B)))
            );
            assert_eq!(
                theme.title_text_style,
                Some(TextStyle::new().color(Color::new(0x0000000C)))
            );
            assert_eq!(
                theme.subtitle_text_style,
                Some(TextStyle::new().color(Color::new(0x0000000D)))
            );
            assert_eq!(
                theme.leading_and_trailing_text_style,
                Some(TextStyle::new().color(Color::new(0x0000000E)))
            );
            assert_eq!(theme.content_padding, Some(EdgeInsets::all(500.0).into()));
            assert_eq!(theme.tile_color, Some(Color::new(0x0000000F)));
            assert_eq!(theme.selected_tile_color, Some(Color::new(0x00000010)));
            assert_eq!(theme.horizontal_title_gap, Some(600.0));
            assert_eq!(theme.min_vertical_padding, Some(700.0));
            assert_eq!(theme.min_leading_width, Some(800.0));
            assert_eq!(theme.min_tile_height, Some(80.0));
            assert_eq!(theme.enable_feedback, Some(false));
            assert_eq!(theme.title_alignment, Some(ListTileTitleAlignment::Top));
            assert!(Rc::ptr_eq(
                theme.mouse_cursor.as_ref().expect("a mouse cursor"),
                &mouse_cursor
            ));
            assert_eq!(theme.visual_density, Some(VisualDensity::COMPACT));
            assert_eq!(theme.is_three_line, Some(false));
        });
    }

    const HEIGHT: f64 = 300.0;
    const AVATAR_TOP: f64 = 130.0;
    const PLACEHOLDER_TOP: f64 = 138.0;

    fn two_line() -> [Rect; 6] {
        [
            Rect::from_ltwh(0.0, 0.0, 800.0, HEIGHT),
            Rect::from_ltwh(16.0, AVATAR_TOP, 40.0, 40.0),
            Rect::from_ltwh(800.0 - 24.0 - 24.0, PLACEHOLDER_TOP, 24.0, 24.0),
            Rect::from_ltwh(0.0, HEIGHT, 800.0, 72.0),
            Rect::from_ltwh(16.0, HEIGHT + 16.0, 40.0, 40.0),
            Rect::from_ltwh(800.0 - 24.0 - 24.0, HEIGHT + 24.0, 24.0, 24.0),
        ]
    }

    fn three_line() -> [Rect; 6] {
        [
            Rect::from_ltwh(0.0, 0.0, 800.0, HEIGHT),
            Rect::from_ltwh(16.0, 8.0, 40.0, 40.0),
            Rect::from_ltwh(800.0 - 24.0 - 24.0, 8.0, 24.0, 24.0),
            Rect::from_ltwh(0.0, HEIGHT, 800.0, 88.0),
            Rect::from_ltwh(16.0, HEIGHT + 8.0, 40.0, 40.0),
            Rect::from_ltwh(800.0 - 24.0 - 24.0, HEIGHT + 8.0, 24.0, 24.0),
        ]
    }

    /// The rects of the two tiles of the `isThreeLine` cases, with their avatars and
    /// placeholders.
    fn rects(
        tester: &mut WidgetTester,
        theme_data_is_three_line: Option<bool>,
        theme_is_three_line: Option<bool>,
    ) -> [Rect; 6] {
        let keys: [GlobalKey; 4] = std::array::from_fn(|_| GlobalKey::new());
        crate::list_tile::tests::pump_unique_app(
            tester,
            |app| {
                let mut theme = ThemeData::builder();
                if let Some(is_three_line) = theme_data_is_three_line {
                    theme = theme
                        .list_tile_theme(ListTileThemeData::new().is_three_line(is_three_line));
                }
                theme.build(app)
            },
            |_app| {
                let tile = |avatar_key: &GlobalKey, placeholder_key: &GlobalKey, subtitle: &str| {
                    ListTile::new()
                        .leading(avatar(avatar_key))
                        .trailing(placeholder(placeholder_key, 24.0, 24.0))
                        .title(Text::new("A"))
                        .subtitle(Text::new(subtitle))
                        .into_widget()
                };
                let list = ListView::new().children([
                    tile(&keys[0], &keys[1], THIRTEEN_LINES),
                    tile(&keys[2], &keys[3], "A"),
                ]);
                let child = match theme_is_three_line {
                    Some(is_three_line) => ListTileTheme::new(list)
                        .data(ListTileThemeData::new().is_three_line(is_three_line))
                        .into_widget(),
                    None => list.into_widget(),
                };
                Material::new().child(child).into_widget()
            },
        );
        [
            tile_rect(tester, 0),
            key_rect(tester, &keys[0]),
            key_rect(tester, &keys[1]),
            tile_rect(tester, 1),
            key_rect(tester, &keys[2]),
            key_rect(tester, &keys[3]),
        ]
    }

    // Regression test for https://github.com/flutter/flutter/issues/165453
    #[test]
    fn list_tile_theme_data_is_three_line() {
        test_widgets(|tester| {
            assert_eq!(rects(tester, None, None), two_line());
            assert_eq!(rects(tester, Some(false), None), two_line());
            assert_eq!(rects(tester, Some(true), None), three_line());
        });
    }

    // Regression test for https://github.com/flutter/flutter/issues/165453
    #[test]
    fn list_tile_theme_is_three_line() {
        test_widgets(|tester| {
            assert_eq!(rects(tester, Some(true), Some(false)), two_line());
            // THREE-LINE
            assert_eq!(rects(tester, None, Some(true)), three_line());
        });
    }
}
