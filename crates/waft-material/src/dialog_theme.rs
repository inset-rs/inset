//! Flutter counterpart: material_ui's `dialog_theme.dart`.

use waft_embedder::{Clip, Color, lerp_double};
use waft_foundation::App;
use waft_painting::{AlignmentGeometry, EdgeInsets, EdgeInsetsGeometry, ShapeBorderRef, TextStyle};
use waft_rendering::BoxConstraints;
use waft_widgets::{
    BuildContext, InheritedTheme, InheritedWidget, IntoWidget, KeyRef, SizedBox, WidgetRef,
};

use crate::theme::Theme;

/// Defines a theme for `Dialog` widgets.
///
/// Descendant widgets obtain the current [`DialogThemeData`] object using
/// [`DialogTheme::of`]. Instances of [`DialogThemeData`] can be customized with
/// [`DialogThemeData::copy_with`].
///
/// [`title_text_style`](Self::title_text_style) and
/// [`content_text_style`](Self::content_text_style) are used in `AlertDialog`s and
/// `SimpleDialog`s.
///
/// Dart's `DialogTheme` constructor takes either the properties or a [`data`](Self::data);
/// here the properties are the `with_*` setters and [`with_data`](Self::with_data) the other,
/// and each setter that breaks Dart's constructor assert (a `data` together with any property)
/// fails in a debug build. The getters are methods, as Dart's, hence the `with_` prefix.
///
/// See also:
///
///  * `Dialog`, a dialog that can be customized using this [`DialogTheme`].
///  * `AlertDialog`, a dialog that can be customized using this [`DialogTheme`].
///  * `SimpleDialog`, a dialog that can be customized using this [`DialogTheme`].
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug)]
pub struct DialogTheme {
    pub key: Option<KeyRef>,
    data: Option<DialogThemeData>,
    background_color: Option<Color>,
    elevation: Option<f64>,
    shadow_color: Option<Color>,
    surface_tint_color: Option<Color>,
    shape: Option<ShapeBorderRef>,
    alignment: Option<AlignmentGeometry>,
    title_text_style: Option<TextStyle>,
    content_text_style: Option<TextStyle>,
    actions_padding: Option<EdgeInsetsGeometry>,
    icon_color: Option<Color>,
    barrier_color: Option<Color>,
    inset_padding: Option<EdgeInsets>,
    clip_behavior: Option<Clip>,
    child: WidgetRef,
}

impl DialogTheme {
    /// Creates a dialog theme that can be used for `ThemeData::dialog_theme`.
    pub fn new() -> DialogTheme {
        DialogTheme {
            key: None,
            data: None,
            background_color: None,
            elevation: None,
            shadow_color: None,
            surface_tint_color: None,
            shape: None,
            alignment: None,
            title_text_style: None,
            content_text_style: None,
            actions_padding: None,
            icon_color: None,
            barrier_color: None,
            inset_padding: None,
            clip_behavior: None,
            child: SizedBox::new().into_widget(),
        }
    }

    /// Dart `DialogTheme(key:)`.
    pub fn key(mut self, key: KeyRef) -> DialogTheme {
        self.key = Some(key);
        self
    }

    /// Dart `DialogTheme(child:)`; the default is an empty `SizedBox`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> DialogTheme {
        self.child = child.into_widget();
        self
    }

    /// Dart `DialogTheme(data:)`.
    pub fn with_data(mut self, data: DialogThemeData) -> DialogTheme {
        debug_assert!(self.has_no_property());
        self.data = Some(data);
        self
    }

    /// Dart `DialogTheme(backgroundColor:)`.
    pub fn with_background_color(mut self, background_color: Color) -> DialogTheme {
        debug_assert!(self.data.is_none());
        self.background_color = Some(background_color);
        self
    }

    /// Dart `DialogTheme(elevation:)`.
    pub fn with_elevation(mut self, elevation: f64) -> DialogTheme {
        debug_assert!(self.data.is_none());
        self.elevation = Some(elevation);
        self
    }

    /// Dart `DialogTheme(shadowColor:)`.
    pub fn with_shadow_color(mut self, shadow_color: Color) -> DialogTheme {
        debug_assert!(self.data.is_none());
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `DialogTheme(surfaceTintColor:)`.
    pub fn with_surface_tint_color(mut self, surface_tint_color: Color) -> DialogTheme {
        debug_assert!(self.data.is_none());
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `DialogTheme(shape:)`.
    pub fn with_shape(mut self, shape: impl Into<ShapeBorderRef>) -> DialogTheme {
        debug_assert!(self.data.is_none());
        self.shape = Some(shape.into());
        self
    }

    /// Dart `DialogTheme(alignment:)`.
    pub fn with_alignment(mut self, alignment: AlignmentGeometry) -> DialogTheme {
        debug_assert!(self.data.is_none());
        self.alignment = Some(alignment);
        self
    }

    /// Dart `DialogTheme(iconColor:)`.
    pub fn with_icon_color(mut self, icon_color: Color) -> DialogTheme {
        debug_assert!(self.data.is_none());
        self.icon_color = Some(icon_color);
        self
    }

    /// Dart `DialogTheme(titleTextStyle:)`.
    pub fn with_title_text_style(mut self, title_text_style: TextStyle) -> DialogTheme {
        debug_assert!(self.data.is_none());
        self.title_text_style = Some(title_text_style);
        self
    }

    /// Dart `DialogTheme(contentTextStyle:)`.
    pub fn with_content_text_style(mut self, content_text_style: TextStyle) -> DialogTheme {
        debug_assert!(self.data.is_none());
        self.content_text_style = Some(content_text_style);
        self
    }

    /// Dart `DialogTheme(actionsPadding:)`.
    pub fn with_actions_padding(mut self, actions_padding: EdgeInsetsGeometry) -> DialogTheme {
        debug_assert!(self.data.is_none());
        self.actions_padding = Some(actions_padding);
        self
    }

    /// Dart `DialogTheme(barrierColor:)`.
    pub fn with_barrier_color(mut self, barrier_color: Color) -> DialogTheme {
        debug_assert!(self.data.is_none());
        self.barrier_color = Some(barrier_color);
        self
    }

    /// Dart `DialogTheme(insetPadding:)`.
    pub fn with_inset_padding(mut self, inset_padding: EdgeInsets) -> DialogTheme {
        debug_assert!(self.data.is_none());
        self.inset_padding = Some(inset_padding);
        self
    }

    /// Dart `DialogTheme(clipBehavior:)`.
    pub fn with_clip_behavior(mut self, clip_behavior: Clip) -> DialogTheme {
        debug_assert!(self.data.is_none());
        self.clip_behavior = Some(clip_behavior);
        self
    }

    /// Overrides the default value for `Dialog::background_color`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`DialogThemeData::background_color`] property in [`data`](Self::data)
    /// instead.
    pub fn background_color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.background_color,
            None => self.background_color,
        }
    }

    /// Overrides the default value for `Dialog::elevation`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`DialogThemeData::elevation`] property in [`data`](Self::data) instead.
    pub fn elevation(&self) -> Option<f64> {
        match &self.data {
            Some(data) => data.elevation,
            None => self.elevation,
        }
    }

    /// Overrides the default value for `Dialog::shadow_color`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`DialogThemeData::shadow_color`] property in [`data`](Self::data) instead.
    pub fn shadow_color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.shadow_color,
            None => self.shadow_color,
        }
    }

    /// Overrides the default value for `Dialog::surface_tint_color`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`DialogThemeData::surface_tint_color`] property in [`data`](Self::data)
    /// instead.
    pub fn surface_tint_color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.surface_tint_color,
            None => self.surface_tint_color,
        }
    }

    /// Overrides the default value for `Dialog::shape`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`DialogThemeData::shape`] property in [`data`](Self::data) instead.
    pub fn shape(&self) -> Option<&ShapeBorderRef> {
        match &self.data {
            Some(data) => data.shape.as_ref(),
            None => self.shape.as_ref(),
        }
    }

    /// Overrides the default value for `Dialog::alignment`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`DialogThemeData::alignment`] property in [`data`](Self::data) instead.
    pub fn alignment(&self) -> Option<AlignmentGeometry> {
        match &self.data {
            Some(data) => data.alignment,
            None => self.alignment,
        }
    }

    /// Overrides the default value for `DefaultTextStyle` for `SimpleDialog::title` and
    /// `AlertDialog::title`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`DialogThemeData::title_text_style`] property in [`data`](Self::data)
    /// instead.
    pub fn title_text_style(&self) -> Option<&TextStyle> {
        match &self.data {
            Some(data) => data.title_text_style.as_ref(),
            None => self.title_text_style.as_ref(),
        }
    }

    /// Overrides the default value for `DefaultTextStyle` for `SimpleDialog::children` and
    /// `AlertDialog::content`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`DialogThemeData::content_text_style`] property in [`data`](Self::data)
    /// instead.
    pub fn content_text_style(&self) -> Option<&TextStyle> {
        match &self.data {
            Some(data) => data.content_text_style.as_ref(),
            None => self.content_text_style.as_ref(),
        }
    }

    /// Overrides the default value for `AlertDialog::actions_padding`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`DialogThemeData::actions_padding`] property in [`data`](Self::data)
    /// instead.
    pub fn actions_padding(&self) -> Option<EdgeInsetsGeometry> {
        match &self.data {
            Some(data) => data.actions_padding,
            None => self.actions_padding,
        }
    }

    /// Used to configure the `IconTheme` for the `AlertDialog::icon` widget.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`DialogThemeData::icon_color`] property in [`data`](Self::data) instead.
    pub fn icon_color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.icon_color,
            None => self.icon_color,
        }
    }

    /// Overrides the default value for `barrier_color` in `ShowDialog`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`DialogThemeData::barrier_color`] property in [`data`](Self::data)
    /// instead.
    pub fn barrier_color(&self) -> Option<Color> {
        match &self.data {
            Some(data) => data.barrier_color,
            None => self.barrier_color,
        }
    }

    /// Overrides the default value for `Dialog::inset_padding`.
    pub fn inset_padding(&self) -> Option<EdgeInsets> {
        match &self.data {
            Some(data) => data.inset_padding,
            None => self.inset_padding,
        }
    }

    /// Overrides the default value of `Dialog::clip_behavior`.
    ///
    /// This property is obsolete and will be deprecated in a future release:
    /// please use the [`DialogThemeData::clip_behavior`] property in [`data`](Self::data)
    /// instead.
    pub fn clip_behavior(&self) -> Option<Clip> {
        match &self.data {
            Some(data) => data.clip_behavior,
            None => self.clip_behavior,
        }
    }

    /// The properties used for all descendant `Dialog` widgets.
    pub fn data(&self) -> DialogThemeData {
        self.data.clone().unwrap_or_else(|| DialogThemeData {
            background_color: self.background_color,
            elevation: self.elevation,
            shadow_color: self.shadow_color,
            surface_tint_color: self.surface_tint_color,
            shape: self.shape.clone(),
            alignment: self.alignment,
            icon_color: self.icon_color,
            title_text_style: self.title_text_style.clone(),
            content_text_style: self.content_text_style.clone(),
            actions_padding: self.actions_padding,
            barrier_color: self.barrier_color,
            inset_padding: self.inset_padding,
            clip_behavior: self.clip_behavior,
            constraints: None,
        })
    }

    /// Retrieves the [`DialogThemeData`] from the closest ancestor [`DialogTheme`].
    ///
    /// If there is no enclosing [`DialogTheme`] widget, then
    /// `Theme::of(context).dialog_theme` is used.
    pub fn of(app: &mut App, context: BuildContext) -> DialogThemeData {
        match context.depend_on_inherited_widget_of_exact_type::<DialogTheme>(app) {
            Some(dialog_theme) => dialog_theme.data(),
            None => Theme::of(app, context).dialog_theme.clone(),
        }
    }

    /// Creates a copy of this object but with the given fields replaced with the
    /// new values.
    ///
    /// This method is obsolete and will be deprecated in a future release:
    /// please use the [`DialogThemeData::copy_with`] instead.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`theme.copy_with().with_elevation(2.0)`). As in Dart, the copy carries neither the key
    /// nor the child, and holds the properties, never a [`data`](Self::data).
    pub fn copy_with(&self) -> DialogTheme {
        DialogTheme {
            key: None,
            data: None,
            background_color: self.background_color(),
            elevation: self.elevation(),
            shadow_color: self.shadow_color(),
            surface_tint_color: self.surface_tint_color(),
            shape: self.shape().cloned(),
            alignment: self.alignment(),
            title_text_style: self.title_text_style().cloned(),
            content_text_style: self.content_text_style().cloned(),
            actions_padding: self.actions_padding(),
            icon_color: self.icon_color(),
            barrier_color: self.barrier_color(),
            inset_padding: self.inset_padding(),
            clip_behavior: self.clip_behavior(),
            child: SizedBox::new().into_widget(),
        }
    }

    /// Linearly interpolate between two dialog themes.
    ///
    /// {@macro dart.ui.shadow.lerp}
    ///
    /// This method is obsolete and will be deprecated in a future release:
    /// please use the [`DialogThemeData::lerp`] instead.
    pub fn lerp(a: Option<&DialogTheme>, b: Option<&DialogTheme>, t: f64) -> DialogTheme {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        DialogTheme {
            key: None,
            data: None,
            background_color: Color::lerp(
                a.and_then(|a| a.background_color()),
                b.and_then(|b| b.background_color()),
                t,
            ),
            elevation: lerp_double(
                a.and_then(|a| a.elevation()),
                b.and_then(|b| b.elevation()),
                t,
            ),
            shadow_color: Color::lerp(
                a.and_then(|a| a.shadow_color()),
                b.and_then(|b| b.shadow_color()),
                t,
            ),
            surface_tint_color: Color::lerp(
                a.and_then(|a| a.surface_tint_color()),
                b.and_then(|b| b.surface_tint_color()),
                t,
            ),
            shape: ShapeBorderRef::lerp(a.and_then(|a| a.shape()), b.and_then(|b| b.shape()), t),
            alignment: AlignmentGeometry::lerp(
                a.and_then(|a| a.alignment()),
                b.and_then(|b| b.alignment()),
                t,
            ),
            icon_color: Color::lerp(
                a.and_then(|a| a.icon_color()),
                b.and_then(|b| b.icon_color()),
                t,
            ),
            title_text_style: TextStyle::lerp(
                a.and_then(|a| a.title_text_style()),
                b.and_then(|b| b.title_text_style()),
                t,
            ),
            content_text_style: TextStyle::lerp(
                a.and_then(|a| a.content_text_style()),
                b.and_then(|b| b.content_text_style()),
                t,
            ),
            actions_padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.actions_padding()),
                b.and_then(|b| b.actions_padding()),
                t,
            ),
            barrier_color: Color::lerp(
                a.and_then(|a| a.barrier_color()),
                b.and_then(|b| b.barrier_color()),
                t,
            ),
            inset_padding: EdgeInsets::lerp(
                a.and_then(|a| a.inset_padding()),
                b.and_then(|b| b.inset_padding()),
                t,
            ),
            clip_behavior: if t < 0.5 {
                a.and_then(|a| a.clip_behavior())
            } else {
                b.and_then(|b| b.clip_behavior())
            },
            child: SizedBox::new().into_widget(),
        }
    }

    /// Dart's constructor assert: a `data` takes the place of every property.
    fn has_no_property(&self) -> bool {
        self.background_color.is_none()
            && self.elevation.is_none()
            && self.shadow_color.is_none()
            && self.surface_tint_color.is_none()
            && self.shape.is_none()
            && self.alignment.is_none()
            && self.icon_color.is_none()
            && self.title_text_style.is_none()
            && self.content_text_style.is_none()
            && self.actions_padding.is_none()
            && self.barrier_color.is_none()
            && self.inset_padding.is_none()
            && self.clip_behavior.is_none()
    }
}

impl Default for DialogTheme {
    fn default() -> DialogTheme {
        DialogTheme::new()
    }
}

impl InheritedTheme for DialogTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        DialogTheme::new()
            .with_data(self.data())
            .child(child)
            .into_widget()
    }
}

impl InheritedWidget for DialogTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &DialogTheme) -> bool {
        self.data() != old_widget.data()
    }

    fn as_inherited_theme(&self) -> Option<&dyn InheritedTheme> {
        Some(self)
    }
}

/// Defines default property values for descendant `Dialog` widgets.
///
/// Descendant widgets obtain the current [`DialogThemeData`] object using
/// [`DialogTheme::of`]. Instances of [`DialogThemeData`] can be
/// customized with [`DialogThemeData::copy_with`].
///
/// Typically a [`DialogThemeData`] is specified as part of the overall `Theme`
/// with `ThemeData::dialog_theme`.
///
/// All [`DialogThemeData`] properties are `null` by default. When null, the `Dialog`
/// will use the values from `ThemeData` if they exist, otherwise it will
/// provide its own defaults. See the individual `Dialog` properties for details.
///
/// See also:
///
///  * `Dialog`, a dialog that can be customized using this [`DialogTheme`].
///  * `AlertDialog`, a dialog that can be customized using this [`DialogTheme`].
///  * `SimpleDialog`, a dialog that can be customized using this [`DialogTheme`].
///  * `ThemeData`, which describes the overall theme information for the
///    application.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DialogThemeData {
    /// Overrides the default value for `Dialog::background_color`.
    pub background_color: Option<Color>,

    /// Overrides the default value for `Dialog::elevation`.
    pub elevation: Option<f64>,

    /// Overrides the default value for `Dialog::shadow_color`.
    pub shadow_color: Option<Color>,

    /// Overrides the default value for `Dialog::surface_tint_color`.
    pub surface_tint_color: Option<Color>,

    /// Overrides the default value for `Dialog::shape`.
    pub shape: Option<ShapeBorderRef>,

    /// Overrides the default value for `Dialog::alignment`.
    pub alignment: Option<AlignmentGeometry>,

    /// Overrides the default value for `DefaultTextStyle` for `SimpleDialog::title` and
    /// `AlertDialog::title`.
    pub title_text_style: Option<TextStyle>,

    /// Overrides the default value for `DefaultTextStyle` for `SimpleDialog::children` and
    /// `AlertDialog::content`.
    pub content_text_style: Option<TextStyle>,

    /// Overrides the default value for `AlertDialog::actions_padding`.
    pub actions_padding: Option<EdgeInsetsGeometry>,

    /// Used to configure the `IconTheme` for the `AlertDialog::icon` widget.
    pub icon_color: Option<Color>,

    /// Overrides the default value for `barrier_color` in `ShowDialog`.
    pub barrier_color: Option<Color>,

    /// Overrides the default value for `Dialog::inset_padding`.
    pub inset_padding: Option<EdgeInsets>,

    /// Overrides the default value of `Dialog::clip_behavior`.
    pub clip_behavior: Option<Clip>,

    /// Constrains the size of the `Dialog`.
    ///
    /// If null, the bottom sheet's size will be unconstrained.
    pub constraints: Option<BoxConstraints>,
}

crate::theme_data_getters::theme_data_getters! {
    /// Dart's getters of `DialogThemeData`: the theme data answers with its fields, and a defaults
    /// subclass (Dart's `_...Defaults...(context)`) computes the ones it overrides when they
    /// are read.
    pub trait DialogThemeDataGetters for DialogThemeData, fields: dialog_theme_data {
        background_color: Option<Color>,
        elevation: Option<f64>,
        shadow_color: Option<Color>,
        surface_tint_color: Option<Color>,
        shape: Option<ShapeBorderRef>,
        alignment: Option<AlignmentGeometry>,
        title_text_style: Option<TextStyle>,
        content_text_style: Option<TextStyle>,
        actions_padding: Option<EdgeInsetsGeometry>,
        icon_color: Option<Color>,
        barrier_color: Option<Color>,
        inset_padding: Option<EdgeInsets>,
        clip_behavior: Option<Clip>,
        constraints: Option<BoxConstraints>,
    }
}

impl DialogThemeData {
    /// Creates a dialog theme that can be used for `ThemeData::dialog_theme`.
    pub fn new() -> DialogThemeData {
        DialogThemeData::default()
    }

    /// Dart `DialogThemeData(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> DialogThemeData {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `DialogThemeData(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> DialogThemeData {
        self.elevation = Some(elevation);
        self
    }

    /// Dart `DialogThemeData(shadowColor:)`.
    pub fn shadow_color(mut self, shadow_color: Color) -> DialogThemeData {
        self.shadow_color = Some(shadow_color);
        self
    }

    /// Dart `DialogThemeData(surfaceTintColor:)`.
    pub fn surface_tint_color(mut self, surface_tint_color: Color) -> DialogThemeData {
        self.surface_tint_color = Some(surface_tint_color);
        self
    }

    /// Dart `DialogThemeData(shape:)`.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> DialogThemeData {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `DialogThemeData(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> DialogThemeData {
        self.alignment = Some(alignment);
        self
    }

    /// Dart `DialogThemeData(iconColor:)`.
    pub fn icon_color(mut self, icon_color: Color) -> DialogThemeData {
        self.icon_color = Some(icon_color);
        self
    }

    /// Dart `DialogThemeData(titleTextStyle:)`.
    pub fn title_text_style(mut self, title_text_style: TextStyle) -> DialogThemeData {
        self.title_text_style = Some(title_text_style);
        self
    }

    /// Dart `DialogThemeData(contentTextStyle:)`.
    pub fn content_text_style(mut self, content_text_style: TextStyle) -> DialogThemeData {
        self.content_text_style = Some(content_text_style);
        self
    }

    /// Dart `DialogThemeData(actionsPadding:)`.
    pub fn actions_padding(mut self, actions_padding: EdgeInsetsGeometry) -> DialogThemeData {
        self.actions_padding = Some(actions_padding);
        self
    }

    /// Dart `DialogThemeData(barrierColor:)`.
    pub fn barrier_color(mut self, barrier_color: Color) -> DialogThemeData {
        self.barrier_color = Some(barrier_color);
        self
    }

    /// Dart `DialogThemeData(insetPadding:)`.
    pub fn inset_padding(mut self, inset_padding: EdgeInsets) -> DialogThemeData {
        self.inset_padding = Some(inset_padding);
        self
    }

    /// Dart `DialogThemeData(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> DialogThemeData {
        self.clip_behavior = Some(clip_behavior);
        self
    }

    /// Dart `DialogThemeData(constraints:)`.
    pub fn constraints(mut self, constraints: BoxConstraints) -> DialogThemeData {
        self.constraints = Some(constraints);
        self
    }

    /// Creates a copy of this object but with the given fields replaced with the
    /// new values.
    ///
    /// Dart's named arguments are the fluent setters on the copy
    /// (`data.copy_with().elevation(2.0)`).
    pub fn copy_with(&self) -> DialogThemeData {
        self.clone()
    }

    /// Linearly interpolate between two dialog themes.
    ///
    /// {@macro dart.ui.shadow.lerp}
    pub fn lerp(
        a: Option<&DialogThemeData>,
        b: Option<&DialogThemeData>,
        t: f64,
    ) -> DialogThemeData {
        if let (Some(a), Some(b)) = (a, b)
            && std::ptr::eq(a, b)
        {
            return a.clone();
        }
        DialogThemeData {
            background_color: Color::lerp(
                a.and_then(|a| a.background_color),
                b.and_then(|b| b.background_color),
                t,
            ),
            elevation: lerp_double(a.and_then(|a| a.elevation), b.and_then(|b| b.elevation), t),
            shadow_color: Color::lerp(
                a.and_then(|a| a.shadow_color),
                b.and_then(|b| b.shadow_color),
                t,
            ),
            surface_tint_color: Color::lerp(
                a.and_then(|a| a.surface_tint_color),
                b.and_then(|b| b.surface_tint_color),
                t,
            ),
            shape: ShapeBorderRef::lerp(
                a.and_then(|a| a.shape.as_ref()),
                b.and_then(|b| b.shape.as_ref()),
                t,
            ),
            alignment: AlignmentGeometry::lerp(
                a.and_then(|a| a.alignment),
                b.and_then(|b| b.alignment),
                t,
            ),
            icon_color: Color::lerp(
                a.and_then(|a| a.icon_color),
                b.and_then(|b| b.icon_color),
                t,
            ),
            title_text_style: TextStyle::lerp(
                a.and_then(|a| a.title_text_style.as_ref()),
                b.and_then(|b| b.title_text_style.as_ref()),
                t,
            ),
            content_text_style: TextStyle::lerp(
                a.and_then(|a| a.content_text_style.as_ref()),
                b.and_then(|b| b.content_text_style.as_ref()),
                t,
            ),
            actions_padding: EdgeInsetsGeometry::lerp(
                a.and_then(|a| a.actions_padding),
                b.and_then(|b| b.actions_padding),
                t,
            ),
            barrier_color: Color::lerp(
                a.and_then(|a| a.barrier_color),
                b.and_then(|b| b.barrier_color),
                t,
            ),
            inset_padding: EdgeInsets::lerp(
                a.and_then(|a| a.inset_padding),
                b.and_then(|b| b.inset_padding),
                t,
            ),
            clip_behavior: if t < 0.5 {
                a.and_then(|a| a.clip_behavior)
            } else {
                b.and_then(|b| b.clip_behavior)
            },
            constraints: BoxConstraints::lerp(
                a.and_then(|a| a.constraints),
                b.and_then(|b| b.constraints),
                t,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Ported from `test/dialog_theme_test.dart`; the `debugFillProperties` cases wait for
    //! diagnostics, `hashCode` for `Hash`, and the goldens are not ported.

    use std::rc::Rc;
    use waft_painting::ShapeBorder;

    use waft_embedder::{Offset, Radius, Rect};
    use waft_painting::{
        Alignment, BeveledRectangleBorder, BorderRadiusGeometry, BorderSide, RoundedRectangleBorder,
    };
    use waft_widgets::{
        Builder, Center, Container, GlobalKey, Icon, IconThemeData, Padding, Text, WidgetBuilder,
    };

    use waft_widgets::testing::{WidgetTester, find, test_widgets};

    use super::*;
    use crate::colors::Colors;
    use crate::dialog::tests::{
        dialog_material, last_barrier_color, material_from_dialog, open, placeholder, rect_of_key,
        text_style_in_dialog,
    };
    use crate::dialog::{AlertDialog, Dialog, ShowDialog, SimpleDialog};
    use crate::elevated_button::ElevatedButton;
    use crate::icons::Icons;
    use crate::material::Material;
    use crate::test_support::material_app;
    use crate::text_theme::TextTheme;
    use crate::theme_data::ThemeData;

    /// Dart's `_appWithDialog(tester, dialog, theme:, dialogTheme:)`.
    fn app_with_dialog<K>(
        tester: &mut WidgetTester,
        dialog: impl IntoWidget<K>,
        theme: impl FnOnce(&App) -> ThemeData,
        dialog_theme: Option<DialogThemeData>,
    ) {
        let dialog = dialog.into_widget();
        let root = {
            let app = tester.cell.borrow();
            let theme = theme(&app);
            let mut dialog_builder = Builder::new(move |_app, context| {
                let dialog = dialog.clone();
                Center::new()
                    .child(ElevatedButton::new(
                        Some(waft_foundation::Listener::new(move |app| {
                            let dialog = dialog.clone();
                            let builder: WidgetBuilder =
                                Rc::new(move |_app, _context| dialog.clone());
                            drop(ShowDialog::new(builder).show(app, context));
                        })),
                        Some(Text::new("X")),
                    ))
                    .into_widget()
            })
            .into_widget();
            if let Some(dialog_theme) = dialog_theme {
                dialog_builder = DialogTheme::new()
                    .with_data(dialog_theme)
                    .child(dialog_builder)
                    .into_widget();
            }
            material_app(theme, Material::new().child(dialog_builder).into_widget())
        };
        tester.pump_widget(root);
    }

    fn m3(app: &App) -> ThemeData {
        ThemeData::new(app)
    }

    fn with_dialog_theme(dialog_theme: DialogThemeData) -> impl FnOnce(&App) -> ThemeData {
        move |app| ThemeData::builder().dialog_theme(dialog_theme).build(app)
    }

    fn m2_with_dialog_theme(dialog_theme: DialogThemeData) -> impl FnOnce(&App) -> ThemeData {
        move |app| {
            ThemeData::builder()
                .use_material3(false)
                .dialog_theme(dialog_theme)
                .build(app)
        }
    }

    /// Dart's `_getMaterialAlertDialog(tester)`.
    fn material_alert_dialog<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
        let dialog = find::by_type::<AlertDialog>().evaluate(tester)[0];
        let material = tester.descendants_of_type::<Material>(dialog)[0];
        tester.widget::<Material, R>(material, read)
    }

    /// Dart's `_getIconRenderObject(tester, icon).text.style?.color`.
    fn icon_color(tester: &WidgetTester, icon: &waft_widgets::IconData) -> Option<Color> {
        tester
            .glyph_style(tester.element(find::by_icon(icon).first()))
            .color
            .map(|color| color.color())
    }

    /// Dart's `_getTextRenderObject(tester, text).text.style?.color`.
    fn text_color(tester: &WidgetTester, text: &str) -> Option<Color> {
        text_style_in_dialog(tester, text)
            .and_then(|style| style.color)
            .map(|color| color.color())
    }

    fn dialog_bottom_left(tester: &WidgetTester) -> Offset {
        tester.get_rect(dialog_material(tester)).bottom_left()
    }

    #[test]
    fn dialog_theme_data_copy_with_equality_basics() {
        assert_eq!(DialogThemeData::new(), DialogThemeData::new().copy_with());
    }

    #[test]
    fn dialog_theme_data_lerp_special_cases() {
        assert_eq!(
            DialogThemeData::lerp(None, None, 0.0),
            DialogThemeData::new()
        );
        let theme = DialogThemeData::new();
        assert_eq!(
            DialogThemeData::lerp(Some(&theme), Some(&theme), 0.5),
            theme
        );
    }

    #[test]
    fn dialog_theme_data_defaults() {
        let dialog_theme_data = DialogThemeData::new();

        assert_eq!(dialog_theme_data.background_color, None);
        assert_eq!(dialog_theme_data.elevation, None);
        assert_eq!(dialog_theme_data.shadow_color, None);
        assert_eq!(dialog_theme_data.surface_tint_color, None);
        assert!(dialog_theme_data.shape.is_none());
        assert_eq!(dialog_theme_data.alignment, None);
        assert_eq!(dialog_theme_data.icon_color, None);
        assert_eq!(dialog_theme_data.title_text_style, None);
        assert_eq!(dialog_theme_data.content_text_style, None);
        assert_eq!(dialog_theme_data.actions_padding, None);
        assert_eq!(dialog_theme_data.barrier_color, None);
        assert_eq!(dialog_theme_data.inset_padding, None);
        assert_eq!(dialog_theme_data.clip_behavior, None);
        assert_eq!(dialog_theme_data.constraints, None);

        let dialog_theme = DialogTheme::new()
            .with_data(DialogThemeData::new())
            .child(waft_widgets::SizedBox::new());
        assert_eq!(dialog_theme.background_color(), None);
        assert_eq!(dialog_theme.elevation(), None);
        assert_eq!(dialog_theme.shadow_color(), None);
        assert_eq!(dialog_theme.surface_tint_color(), None);
        assert!(dialog_theme.shape().is_none());
        assert_eq!(dialog_theme.alignment(), None);
        assert_eq!(dialog_theme.icon_color(), None);
        assert_eq!(dialog_theme.title_text_style(), None);
        assert_eq!(dialog_theme.content_text_style(), None);
        assert_eq!(dialog_theme.actions_padding(), None);
        assert_eq!(dialog_theme.barrier_color(), None);
        assert_eq!(dialog_theme.inset_padding(), None);
        assert_eq!(dialog_theme.clip_behavior(), None);
        assert_eq!(dialog_theme_data.constraints, None);
    }

    /// The theme both "Local DialogThemeData" cases give the `DialogTheme`.
    fn local_dialog_theme() -> DialogThemeData {
        DialogThemeData::new()
            .background_color(Color::new(0xff123456))
            .elevation(8.0)
            .shadow_color(Color::new(0xff000001))
            .surface_tint_color(Color::new(0xff000002))
            .shape(BeveledRectangleBorder::new(
                BorderSide::NONE,
                BorderRadiusGeometry::all(Radius::circular(20.5)),
            ))
            .alignment(AlignmentGeometry::BOTTOM_LEFT)
            .icon_color(Color::new(0xff654321))
            .title_text_style(TextStyle::new().color(Color::new(0xffffffff)))
            .content_text_style(TextStyle::new().color(Color::new(0xff000000)))
            .actions_padding(EdgeInsetsGeometry::all(8.0))
            .barrier_color(Color::new(0xff000005))
            .inset_padding(EdgeInsets::all(30.0))
            .clip_behavior(Clip::AntiAlias)
    }

    fn icon_title_content_dialog() -> AlertDialog {
        AlertDialog::new()
            .title(Text::new("Title"))
            .content(Text::new("Content"))
            .icon(Icon::new(Some(Icons::search())))
            .actions([Icon::new(Some(Icons::cancel())).into_widget()])
    }

    /// The expectations both "Local DialogThemeData" cases share: the local theme wins.
    fn expect_local_theme(tester: &WidgetTester) {
        let theme = local_dialog_theme();
        material_alert_dialog(tester, |material| {
            assert_eq!(material.color, theme.background_color);
            assert_eq!(material.elevation, 8.0);
            assert_eq!(material.shadow_color, theme.shadow_color);
            assert_eq!(material.surface_tint_color, theme.surface_tint_color);
            assert_eq!(material.shape, theme.shape);
            assert_eq!(material.clip_behavior, Clip::AntiAlias);
        });
        let bottom_left = dialog_bottom_left(tester);
        assert_eq!(bottom_left.dx(), 30.0); // 30 is the padding value.
        assert_eq!(bottom_left.dy(), 570.0); // 600 - 30
        assert_eq!(icon_color(tester, &Icons::search()), theme.icon_color);
        assert_eq!(text_color(tester, "Title"), Some(Color::new(0xffffffff)));
        assert_eq!(text_color(tester, "Content"), Some(Color::new(0xff000000)));
        assert_eq!(last_barrier_color(tester), theme.barrier_color);

        let cancel = tester.element(find::by_icon(&Icons::cancel()).first());
        let padding = tester.ancestor_of_type::<Padding>(cancel);
        assert_eq!(
            tester.widget::<Padding, _>(padding, |padding| padding.padding),
            EdgeInsetsGeometry::all(8.0)
        );
    }

    #[test]
    fn local_dialog_theme_data_overrides_dialog_defaults() {
        test_widgets(|tester| {
            app_with_dialog(
                tester,
                icon_title_content_dialog(),
                m3,
                Some(local_dialog_theme()),
            );
            open(tester);

            expect_local_theme(tester);
        });
    }

    #[test]
    fn local_dialog_theme_data_overrides_global_dialog_theme() {
        test_widgets(|tester| {
            let global_dialog_theme = DialogThemeData::new()
                .background_color(Color::new(0xff654321))
                .elevation(7.0)
                .shadow_color(Color::new(0xff200001))
                .surface_tint_color(Color::new(0xff222002))
                .shape(BeveledRectangleBorder::new(
                    BorderSide::NONE,
                    BorderRadiusGeometry::all(Radius::circular(25.5)),
                ))
                .alignment(AlignmentGeometry::CENTER_RIGHT)
                .icon_color(Color::new(0xff666666))
                .title_text_style(TextStyle::new().color(Color::new(0xff000000)))
                .content_text_style(TextStyle::new().color(Color::new(0xffdddddd)))
                .actions_padding(EdgeInsetsGeometry::all(18.0))
                .barrier_color(Color::new(0xff111115))
                .inset_padding(EdgeInsets::all(35.0))
                .clip_behavior(Clip::HardEdge);
            app_with_dialog(
                tester,
                icon_title_content_dialog(),
                with_dialog_theme(global_dialog_theme),
                Some(local_dialog_theme()),
            );
            open(tester);

            expect_local_theme(tester);
        });
    }

    fn title_dialog() -> AlertDialog {
        AlertDialog::new().title(Text::new("Title")).actions([])
    }

    #[test]
    fn dialog_background_color() {
        test_widgets(|tester| {
            let custom_color = Colors::PINK.primary();
            app_with_dialog(
                tester,
                title_dialog(),
                with_dialog_theme(DialogThemeData::new().background_color(custom_color)),
                None,
            );
            open(tester);

            assert_eq!(
                material_alert_dialog(tester, |material| material.color),
                Some(custom_color)
            );
        });
    }

    #[test]
    fn custom_dialog_elevation() {
        test_widgets(|tester| {
            let custom_elevation = 12.0;
            let shadow_color = Color::new(0xFF000001);
            let surface_tint_color = Color::new(0xFF000002);
            app_with_dialog(
                tester,
                title_dialog(),
                with_dialog_theme(
                    DialogThemeData::new()
                        .elevation(custom_elevation)
                        .shadow_color(shadow_color)
                        .surface_tint_color(surface_tint_color),
                ),
                None,
            );
            open(tester);

            material_alert_dialog(tester, |material| {
                assert_eq!(material.elevation, custom_elevation);
                assert_eq!(material.shadow_color, Some(shadow_color));
                assert_eq!(material.surface_tint_color, Some(surface_tint_color));
            });
        });
    }

    #[test]
    fn custom_dialog_shape() {
        test_widgets(|tester| {
            let custom_border = RoundedRectangleBorder::new(
                BorderSide::NONE,
                BorderRadiusGeometry::all(Radius::circular(16.0)),
            );
            app_with_dialog(
                tester,
                title_dialog(),
                with_dialog_theme(DialogThemeData::new().shape(custom_border)),
                None,
            );
            open(tester);

            material_alert_dialog(tester, |material| {
                assert_eq!(
                    material.shape.as_deref(),
                    Some(&custom_border as &dyn ShapeBorder)
                );
            });
        });
    }

    #[test]
    fn custom_dialog_alignment() {
        test_widgets(|tester| {
            app_with_dialog(
                tester,
                title_dialog(),
                with_dialog_theme(DialogThemeData::new().alignment(AlignmentGeometry::BOTTOM_LEFT)),
                None,
            );
            open(tester);

            let bottom_left = dialog_bottom_left(tester);
            assert_eq!(bottom_left.dx(), 40.0);
            assert_eq!(bottom_left.dy(), 576.0);
        });
    }

    fn alignment_takes_priority(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        dy: f64,
    ) {
        app_with_dialog(
            tester,
            title_dialog().alignment(AlignmentGeometry::Alignment(Alignment::TOP_RIGHT)),
            theme,
            None,
        );
        open(tester);

        let bottom_left = dialog_bottom_left(tester);
        assert_eq!(bottom_left.dx(), 480.0);
        assert_eq!(bottom_left.dy(), dy);
    }

    #[test]
    fn material3_dialog_alignment_takes_priority_over_theme() {
        test_widgets(|tester| {
            alignment_takes_priority(
                tester,
                with_dialog_theme(DialogThemeData::new().alignment(AlignmentGeometry::BOTTOM_LEFT)),
                124.0,
            );
        });
    }

    #[test]
    fn material2_dialog_alignment_takes_priority_over_theme() {
        test_widgets(|tester| {
            alignment_takes_priority(
                tester,
                m2_with_dialog_theme(
                    DialogThemeData::new().alignment(AlignmentGeometry::BOTTOM_LEFT),
                ),
                104.0,
            );
        });
    }

    fn icon_dialog() -> AlertDialog {
        AlertDialog::new()
            .icon(Icon::new(Some(Icons::ac_unit())))
            .actions([])
    }

    #[test]
    fn custom_icon_color_constructor_param_highest_preference() {
        test_widgets(|tester| {
            let (icon_color_, dialog_theme_color, icon_theme_color) = (
                Colors::PINK.primary(),
                Colors::GREEN.primary(),
                Colors::YELLOW.primary(),
            );
            app_with_dialog(
                tester,
                icon_dialog().icon_color(icon_color_),
                move |app| {
                    ThemeData::builder()
                        .icon_theme(IconThemeData::new().color(icon_theme_color))
                        .dialog_theme(DialogThemeData::new().icon_color(dialog_theme_color))
                        .build(app)
                },
                None,
            );
            open(tester);

            assert_eq!(icon_color(tester, &Icons::ac_unit()), Some(icon_color_));
        });
    }

    #[test]
    fn custom_icon_color_dialog_theme_preference_over_theme() {
        test_widgets(|tester| {
            let (dialog_theme_color, icon_theme_color) =
                (Colors::GREEN.primary(), Colors::YELLOW.primary());
            app_with_dialog(
                tester,
                icon_dialog(),
                move |app| {
                    ThemeData::builder()
                        .icon_theme(IconThemeData::new().color(icon_theme_color))
                        .dialog_theme(DialogThemeData::new().icon_color(dialog_theme_color))
                        .build(app)
                },
                None,
            );
            open(tester);

            assert_eq!(
                icon_color(tester, &Icons::ac_unit()),
                Some(dialog_theme_color)
            );
        });
    }

    #[test]
    fn material3_custom_icon_color_theme_lowest_preference() {
        test_widgets(|tester| {
            let secondary = m3(&tester.cell.borrow()).color_scheme.secondary;
            app_with_dialog(tester, icon_dialog(), m3, None);
            open(tester);

            assert_eq!(icon_color(tester, &Icons::ac_unit()), Some(secondary));
        });
    }

    #[test]
    fn material2_custom_icon_color_theme_lowest_preference() {
        test_widgets(|tester| {
            let icon_theme_color = Colors::YELLOW.primary();
            app_with_dialog(
                tester,
                icon_dialog(),
                move |app| {
                    ThemeData::builder()
                        .use_material3(false)
                        .icon_theme(IconThemeData::new().color(icon_theme_color))
                        .build(app)
                },
                None,
            );
            open(tester);

            assert_eq!(
                icon_color(tester, &Icons::ac_unit()),
                Some(icon_theme_color)
            );
        });
    }

    fn pink() -> TextStyle {
        TextStyle::new().color(Colors::PINK.primary())
    }

    /// The text style cases: `dialog` shows `text`, and its style is `expected` whole, or
    /// only its color where `color_only`.
    fn expect_text_style<K>(
        tester: &mut WidgetTester,
        dialog: impl IntoWidget<K>,
        theme: impl FnOnce(&App) -> ThemeData,
        text: &str,
        color_only: bool,
    ) {
        app_with_dialog(tester, dialog, theme, None);
        open(tester);

        let style = text_style_in_dialog(tester, text);
        if color_only {
            assert_eq!(style.and_then(|style| style.color), pink().color);
        } else {
            assert_eq!(style, Some(pink()));
        }
    }

    #[test]
    fn custom_title_text_style_constructor_param() {
        test_widgets(|tester| {
            expect_text_style(
                tester,
                AlertDialog::new()
                    .title(Text::new("Title"))
                    .title_text_style(pink())
                    .actions([]),
                m3,
                "Title",
                false,
            );
        });
    }

    #[test]
    fn custom_title_text_style_dialog_theme() {
        test_widgets(|tester| {
            expect_text_style(
                tester,
                title_dialog(),
                with_dialog_theme(DialogThemeData::new().title_text_style(pink())),
                "Title",
                false,
            );
        });
    }

    #[test]
    fn material3_custom_title_text_style_theme() {
        test_widgets(|tester| {
            expect_text_style(
                tester,
                AlertDialog::new().title(Text::new("Title")),
                |app| {
                    ThemeData::builder()
                        .text_theme(TextTheme::new().headline_small(pink()))
                        .build(app)
                },
                "Title",
                true,
            );
        });
    }

    #[test]
    fn material2_custom_title_text_style_theme() {
        test_widgets(|tester| {
            expect_text_style(
                tester,
                AlertDialog::new().title(Text::new("Title")),
                |app| {
                    ThemeData::builder()
                        .use_material3(false)
                        .text_theme(TextTheme::new().title_large(pink()))
                        .build(app)
                },
                "Title",
                true,
            );
        });
    }

    #[test]
    fn simple_dialog_custom_title_text_style_constructor_param() {
        test_widgets(|tester| {
            expect_text_style(
                tester,
                SimpleDialog::new()
                    .title(Text::new("Title"))
                    .title_text_style(pink()),
                m3,
                "Title",
                false,
            );
        });
    }

    #[test]
    fn simple_dialog_custom_title_text_style_dialog_theme() {
        test_widgets(|tester| {
            expect_text_style(
                tester,
                SimpleDialog::new().title(Text::new("Title")),
                with_dialog_theme(DialogThemeData::new().title_text_style(pink())),
                "Title",
                false,
            );
        });
    }

    #[test]
    fn simple_dialog_custom_title_text_style_theme() {
        test_widgets(|tester| {
            expect_text_style(
                tester,
                SimpleDialog::new().title(Text::new("Title")),
                |app| {
                    ThemeData::builder()
                        .text_theme(TextTheme::new().title_large(pink()))
                        .build(app)
                },
                "Title",
                true,
            );
        });
    }

    #[test]
    fn custom_content_text_style_constructor_param() {
        test_widgets(|tester| {
            expect_text_style(
                tester,
                AlertDialog::new()
                    .content(Text::new("Content"))
                    .content_text_style(pink())
                    .actions([]),
                m3,
                "Content",
                false,
            );
        });
    }

    #[test]
    fn custom_content_text_style_dialog_theme() {
        test_widgets(|tester| {
            expect_text_style(
                tester,
                AlertDialog::new().content(Text::new("Content")).actions([]),
                with_dialog_theme(DialogThemeData::new().content_text_style(pink())),
                "Content",
                false,
            );
        });
    }

    #[test]
    fn material3_custom_content_text_style_theme() {
        test_widgets(|tester| {
            expect_text_style(
                tester,
                AlertDialog::new().content(Text::new("Content")),
                |app| {
                    ThemeData::builder()
                        .text_theme(TextTheme::new().body_medium(pink()))
                        .build(app)
                },
                "Content",
                true,
            );
        });
    }

    #[test]
    fn material2_custom_content_text_style_theme() {
        test_widgets(|tester| {
            expect_text_style(
                tester,
                AlertDialog::new().content(Text::new("Content")),
                |app| {
                    ThemeData::builder()
                        .use_material3(false)
                        .text_theme(TextTheme::new().title_medium(pink()))
                        .build(app)
                },
                "Content",
                true,
            );
        });
    }

    #[test]
    fn custom_barrier_color_theme() {
        test_widgets(|tester| {
            let barrier_color = Colors::BLUE.primary();
            app_with_dialog(
                tester,
                SimpleDialog::new(),
                with_dialog_theme(DialogThemeData::new().barrier_color(barrier_color)),
                None,
            );
            open(tester);

            assert_eq!(last_barrier_color(tester), Some(barrier_color));
        });
    }

    #[test]
    fn dialog_theme_inset_padding_updates_dialog_inset_padding() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            // The default testing screen (800, 600)
            let screen_rect = Rect::from_ltrb(0.0, 0.0, 800.0, 600.0);
            let inset_padding = EdgeInsets::from_ltrb(10.0, 15.0, 20.0, 25.0);
            app_with_dialog(
                tester,
                Dialog::new().child(placeholder(&key)),
                with_dialog_theme(DialogThemeData::new().inset_padding(inset_padding)),
                None,
            );
            tester.tap(find::text("X"));
            tester.pump_by(std::time::Duration::ZERO);

            assert_eq!(
                rect_of_key(tester, &key),
                Rect::from_ltrb(
                    screen_rect.left + inset_padding.left,
                    screen_rect.top + inset_padding.top,
                    screen_rect.right - inset_padding.right,
                    screen_rect.bottom - inset_padding.bottom,
                )
            );
        });
    }

    fn clip_of_dialog(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        dialog: Dialog,
    ) -> Clip {
        app_with_dialog(tester, dialog, theme, None);
        open(tester);
        material_from_dialog(tester, |material| material.clip_behavior)
    }

    #[test]
    fn dialog_theme_clip_behavior_updates_the_dialogs_clip_behavior() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            assert_eq!(
                clip_of_dialog(
                    tester,
                    with_dialog_theme(DialogThemeData::new().clip_behavior(Clip::HardEdge)),
                    Dialog::new().child(placeholder(&key)),
                ),
                Clip::HardEdge
            );
        });
    }

    #[test]
    fn dialog_clip_behavior_takes_priority_over_theme() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            assert_eq!(
                clip_of_dialog(
                    tester,
                    with_dialog_theme(DialogThemeData::new().clip_behavior(Clip::HardEdge)),
                    Dialog::new()
                        .clip_behavior(Clip::AntiAlias)
                        .child(placeholder(&key)),
                ),
                Clip::AntiAlias
            );
        });
    }

    #[test]
    fn material2_dialog_clip_behavior_takes_priority_over_theme() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            assert_eq!(
                clip_of_dialog(
                    tester,
                    m2_with_dialog_theme(DialogThemeData::new().clip_behavior(Clip::HardEdge)),
                    Dialog::new()
                        .clip_behavior(Clip::AntiAlias)
                        .child(placeholder(&key)),
                ),
                Clip::AntiAlias
            );
        });
    }

    #[test]
    fn dialog_theme_data_constraints_is_respected_if_dialog_constraints_is_null() {
        test_widgets(|tester| {
            let theme_constraints = BoxConstraints::new().max_width(500.0).max_height(500.0);
            let key = GlobalKey::new();
            app_with_dialog(
                tester,
                Dialog::new().child(
                    waft_widgets::SizedBox::expand().child(
                        Container::new()
                            .key(key.clone().into())
                            .color(Colors::AMBER.primary()),
                    ),
                ),
                with_dialog_theme(
                    DialogThemeData::new()
                        .alignment(AlignmentGeometry::CENTER)
                        .constraints(theme_constraints),
                ),
                None,
            );
            open(tester);

            // Verify that the dialog respects the constraints from the theme
            let render_box = tester.render_object(&key).as_box().expect("a render box");
            let constraints = render_box.constraints(&tester.cell.borrow());
            assert_eq!(constraints.max_width, theme_constraints.max_width);
            assert_eq!(constraints.max_height, theme_constraints.max_height);
        });
    }

    #[test]
    fn the_legacy_properties_make_the_data_and_a_data_answers_them() {
        let theme = DialogTheme::new()
            .with_elevation(3.0)
            .with_barrier_color(Colors::RED.primary())
            .with_inset_padding(EdgeInsets::all(4.0));
        assert_eq!(theme.elevation(), Some(3.0));
        assert_eq!(
            theme.data(),
            DialogThemeData::new()
                .elevation(3.0)
                .barrier_color(Colors::RED.primary())
                .inset_padding(EdgeInsets::all(4.0))
        );
        assert_eq!(theme.copy_with().with_elevation(5.0).elevation(), Some(5.0));

        let from_data = DialogTheme::new().with_data(DialogThemeData::new().elevation(9.0));
        assert_eq!(from_data.elevation(), Some(9.0));
        assert_eq!(
            from_data.copy_with().data(),
            DialogThemeData::new().elevation(9.0)
        );
    }

    #[test]
    fn dialog_theme_data_lerps_each_field() {
        let a = DialogThemeData::new()
            .background_color(Colors::BLACK)
            .elevation(2.0)
            .inset_padding(EdgeInsets::all(10.0))
            .constraints(BoxConstraints::new().max_width(100.0))
            .clip_behavior(Clip::HardEdge);
        let b = DialogThemeData::new()
            .background_color(Colors::WHITE)
            .elevation(6.0)
            .inset_padding(EdgeInsets::all(30.0))
            .constraints(BoxConstraints::new().max_width(300.0))
            .clip_behavior(Clip::AntiAlias);

        let halfway = DialogThemeData::lerp(Some(&a), Some(&b), 0.5);

        assert_eq!(
            halfway.background_color,
            Color::lerp(Some(Colors::BLACK), Some(Colors::WHITE), 0.5)
        );
        assert_eq!(halfway.elevation, Some(4.0));
        assert_eq!(halfway.inset_padding, Some(EdgeInsets::all(20.0)));
        assert_eq!(
            halfway.constraints,
            BoxConstraints::lerp(a.constraints, b.constraints, 0.5)
        );
        assert_eq!(halfway.clip_behavior, Some(Clip::AntiAlias));
        assert_eq!(
            DialogThemeData::lerp(Some(&a), Some(&b), 0.4).clip_behavior,
            Some(Clip::HardEdge)
        );
    }

    #[test]
    fn update_should_notify_only_when_the_data_differs() {
        let theme = |data: DialogThemeData| DialogTheme::new().with_data(data);
        let old = theme(DialogThemeData::new().elevation(1.0));

        assert!(!theme(DialogThemeData::new().elevation(1.0)).update_should_notify(&old));
        assert!(theme(DialogThemeData::new().elevation(2.0)).update_should_notify(&old));
    }

    #[test]
    fn theme_data_holds_the_dialog_theme() {
        test_widgets(|tester| {
            let app = tester.cell.borrow();
            let theme = ThemeData::new(&app);
            assert_eq!(theme.dialog_theme, DialogThemeData::new());

            let raised = ThemeData::builder()
                .dialog_theme(DialogThemeData::new().elevation(8.0))
                .build(&app);
            assert_eq!(raised.dialog_theme.elevation, Some(8.0));
            assert_ne!(theme, raised);
            assert_eq!(
                theme
                    .copy_with()
                    .dialog_theme(raised.dialog_theme.clone())
                    .build(),
                raised
            );

            let flat = ThemeData::builder()
                .dialog_theme(DialogThemeData::new().elevation(0.0))
                .build(&app);
            assert_eq!(
                ThemeData::lerp(&flat, &raised, 0.5).dialog_theme.elevation,
                Some(4.0)
            );
        });
    }
}
