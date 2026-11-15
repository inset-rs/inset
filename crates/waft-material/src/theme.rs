//! Flutter counterpart: material_ui's `theme.dart`.

use std::rc::Rc;
use std::time::Duration;

use waft_animation::Animatable;
use waft_cupertino::{CupertinoTheme, CupertinoThemeData, InheritedCupertinoTheme};
use waft_embedder::Brightness;
use waft_foundation::{App, Handle, Ref};
use waft_painting::AnyColor;
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_widgets::{
    AnimatedWidgetBaseState, BuildContext, DefaultSelectionStyle, IconTheme,
    ImplicitlyAnimatedWidget, ImplicitlyAnimatedWidgetData, ImplicitlyAnimatedWidgetState,
    ImplicitlyAnimatedWidgetStateData, InheritedWidget, IntoWidget, KeyRef, Localizations,
    MediaQuery, SingleTickerProviderStateMixin, SingleTickerProviderStateMixinData, State,
    StateData, StatefulWidget, StatelessWidget, TweenSlot, TweenValue, TweenVisitor, WidgetRef,
};

use crate::material_localizations::MaterialLocalizations;
use crate::theme_data::{
    CupertinoBasedMaterialThemeData, MaterialBasedCupertinoThemeData, ThemeData,
};
use crate::typography::ScriptCategory;

/// The duration over which theme changes animate by default.
pub const K_THEME_ANIMATION_DURATION: Duration = Duration::from_millis(200);

/// Applies a theme to descendant widgets.
///
/// A theme describes the colors and typographic choices of an application.
///
/// Learn more about [`Theme`] on the [Flutter YouTube channel](https://www.youtube.com/watch?v=oTvQDJOBXmM).
///
/// Descendant widgets obtain the current theme's [`ThemeData`] object using
/// [`Theme::of`]. When a widget uses [`Theme::of`], it is automatically rebuilt if
/// the theme later changes, so that the changes can be applied.
///
/// The [`Theme`] widget implies an [`IconTheme`] widget, set to the value of the
/// [`ThemeData::icon_theme`] of the [`data`](Self::data) for the [`Theme`].
///
/// To interact seamlessly with descendant Cupertino widgets, the [`Theme`] widget
/// provides a [`CupertinoTheme`] for its descendants with a [`CupertinoThemeData`] inherited
/// from the nearest ancestor [`CupertinoTheme`] or if none exists, derived from the
/// Material [`data`](Self::data) for the [`Theme`]. The values in the Material derived
/// [`CupertinoThemeData`] are overridable through [`ThemeData::cupertino_override_theme`].
/// The values from an inherited [`CupertinoThemeData`] can be overridden by wrapping the
/// desired subtree with a [`CupertinoTheme`].
///
/// See also:
///
///  * [`ThemeData`], which describes the actual configuration of a theme.
///  * [`AnimatedTheme`], which animates the [`ThemeData`] when it changes rather
///    than changing the theme all at once.
///  * `MaterialApp`, which includes an [`AnimatedTheme`] widget configured via
///    the `MaterialApp.theme` argument.
#[derive(Clone, Debug)]
pub struct Theme {
    pub key: Option<KeyRef>,
    /// Specifies the color and typography values for descendant widgets.
    ///
    /// Shared, as Dart shares the one `ThemeData` object: [`Theme::of`] hands out this
    /// value, and [`ThemeData::localize`] keys its cache on its identity.
    pub data: Rc<ThemeData>,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

/// Dart's static `_cachedFallbackTheme`, one per app.
#[derive(Default)]
struct CachedFallbackTheme {
    theme: Option<Rc<ThemeData>>,
}

impl Theme {
    /// Applies the given theme `data` to `child`.
    pub fn new<K>(data: impl Into<Rc<ThemeData>>, child: impl IntoWidget<K>) -> Theme {
        Theme {
            key: None,
            data: data.into(),
            child: child.into_widget(),
        }
    }

    /// Dart `Theme(key:)`.
    pub fn key(mut self, key: KeyRef) -> Theme {
        self.key = Some(key);
        self
    }

    /// The theme [`Theme::of`] returns when there is no [`Theme`] ancestor.
    ///
    /// The fallback is cached because constructing a [`ThemeData`] is expensive,
    /// but it is rebuilt whenever `defaultTargetPlatform` no longer matches the
    /// platform it was built for (for example after a platform override), so a theme-less
    /// subtree adapts to the current platform the same way a `MaterialApp` would.
    fn fallback_theme(app: &mut App) -> Rc<ThemeData> {
        let cache = app.singleton::<CachedFallbackTheme>();
        let platform = app.platform().target_platform();
        let stale = app
            .get(cache)
            .theme
            .as_ref()
            .is_none_or(|theme| theme.platform != platform);
        if stale {
            let theme = Rc::new(ThemeData::fallback(app, None));
            app.get_mut(cache).theme = Some(theme);
        }
        app.get(cache).theme.clone().expect("filled above")
    }

    /// The data from the closest [`Theme`] instance that encloses the given
    /// context.
    ///
    /// If the given context is enclosed in a `Localizations` widget providing
    /// `MaterialLocalizations`, the returned data is localized according to the
    /// nearest available `MaterialLocalizations`.
    ///
    /// Defaults to [`ThemeData::fallback`] if there is no [`Theme`] in the given
    /// build context.
    ///
    /// When the [`Theme`] is actually created in the same `build` function
    /// (possibly indirectly, e.g. as part of a `MaterialApp`), the `context`
    /// argument to the `build` function can't be used to find the [`Theme`] (since
    /// it's "above" the widget being returned). In such cases, a `Builder` can be used
    /// to provide a new scope with a [`BuildContext`] that is "under" the [`Theme`].
    ///
    /// See also:
    ///
    /// * `ColorScheme::of`, a convenience method that returns [`ThemeData::color_scheme`]
    ///   from the closest [`Theme`] ancestor. (equivalent to
    ///   `Theme::of(app, context).color_scheme`).
    /// * [`TextTheme::of`](crate::TextTheme::of), a convenience method that returns [`ThemeData::text_theme`]
    ///   from the closest [`Theme`] ancestor. (equivalent to
    ///   `Theme::of(app, context).text_theme`).
    /// * [`IconTheme::of`], that returns [`ThemeData::icon_theme`] from the closest
    ///   [`Theme`] or `IconThemeData::fallback` if there is no [`IconTheme`] ancestor.
    pub fn of(app: &mut App, context: BuildContext) -> Rc<ThemeData> {
        let inherited_theme = context
            .depend_on_inherited_widget_of_exact_type::<InheritedTheme>(app)
            .map(|inherited_theme| Rc::clone(&inherited_theme.theme.data));
        let localizations = Localizations::of::<dyn MaterialLocalizations>(app, context);
        let category = localizations
            .map(|localizations| localizations.script_category())
            .unwrap_or(ScriptCategory::EnglishLike);
        let inherited_cupertino_theme = context
            .depend_on_inherited_widget_of_exact_type::<InheritedCupertinoTheme>(app)
            .map(|inherited_cupertino_theme| inherited_cupertino_theme.theme.data.clone());
        let theme = match (inherited_theme, inherited_cupertino_theme) {
            (Some(theme), _) => theme,
            (None, Some(cupertino_theme)) => {
                CupertinoBasedMaterialThemeData::new(app, &cupertino_theme).material_theme
            }
            (None, None) => Theme::fallback_theme(app),
        };
        let local_text_geometry = Rc::clone(theme.typography.geometry_theme_for(category));
        ThemeData::localize(app, &theme, &local_text_geometry)
    }

    // The inherited themes in widgets library can not infer their values from
    // Theme in material library. Wraps the child with these inherited themes to
    // overrides their values directly.
    fn wraps_widget_themes(
        &self,
        app: &mut App,
        context: BuildContext,
        child: WidgetRef,
    ) -> WidgetRef {
        let selection_style = DefaultSelectionStyle::of(app, context);
        let text_selection_theme = &self.data.text_selection_theme;
        let mut default_selection_style = DefaultSelectionStyle::new(child);
        if let Some(selection_color) = text_selection_theme
            .selection_color
            .map(AnyColor::new)
            .or(selection_style.selection_color)
        {
            default_selection_style = default_selection_style.selection_color(selection_color);
        }
        if let Some(cursor_color) = text_selection_theme
            .cursor_color
            .map(AnyColor::new)
            .or(selection_style.cursor_color)
        {
            default_selection_style = default_selection_style.cursor_color(cursor_color);
        }
        IconTheme::new(self.data.icon_theme.clone(), default_selection_style).into_widget()
    }

    fn inherited_cupertino_theme_data(
        &self,
        app: &mut App,
        context: BuildContext,
    ) -> CupertinoThemeData {
        let inherited_theme = context
            .depend_on_inherited_widget_of_exact_type::<InheritedCupertinoTheme>(app)
            .map(|inherited_theme| inherited_theme.theme.data.clone());
        inherited_theme
            .unwrap_or_else(|| MaterialBasedCupertinoThemeData::new(&self.data))
            .resolve_from(app, context)
    }

    /// Retrieves the [`Brightness`] to use for descendant Material widgets, based
    /// on the value of [`ThemeData::brightness`] in the given `context`.
    ///
    /// If no `InheritedTheme` can be found in the given `context`, or its `brightness`
    /// is null, it will fall back to `MediaQueryData.platform_brightness`.
    ///
    /// See also:
    ///
    /// * [`maybe_brightness_of`](Self::maybe_brightness_of), which returns null if no valid
    ///   `InheritedTheme` or [`MediaQuery`] exists.
    /// * [`ThemeData::brightness`], the property that takes precedence over
    ///   `MediaQueryData.platform_brightness` for descendant Material widgets.
    pub fn brightness_of(app: &mut App, context: BuildContext) -> Brightness {
        let brightness = context
            .depend_on_inherited_widget_of_exact_type::<InheritedTheme>(app)
            .map(|inherited_theme| inherited_theme.theme.data.brightness());
        brightness.unwrap_or_else(|| MediaQuery::platform_brightness_of(app, context))
    }

    /// Retrieves the [`Brightness`] to use for descendant Material widgets, based
    /// on the value of [`ThemeData::brightness`] in the given `context`.
    ///
    /// If no `InheritedTheme` or [`MediaQuery`] can be found in the given `context`, it will
    /// return null.
    ///
    /// See also:
    ///
    /// * [`ThemeData::brightness`], the property that takes precedence over
    ///   `MediaQueryData.platform_brightness` for descendant Material widgets.
    /// * [`brightness_of`](Self::brightness_of), which return a default value if no valid
    ///   `InheritedTheme` or [`MediaQuery`] exists, instead of returning null.
    pub fn maybe_brightness_of(app: &mut App, context: BuildContext) -> Option<Brightness> {
        let brightness = context
            .depend_on_inherited_widget_of_exact_type::<InheritedTheme>(app)
            .map(|inherited_theme| inherited_theme.theme.data.brightness());
        brightness.or_else(|| MediaQuery::maybe_platform_brightness_of(app, context))
    }
}

impl StatelessWidget for Theme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        // If a CupertinoThemeData doesn't exist, we're using a
        // MaterialBasedCupertinoThemeData here instead of a CupertinoThemeData
        // because it defers some properties to the Material ThemeData.
        let cupertino_theme_data = self.inherited_cupertino_theme_data(app, context);
        let child = self.wraps_widget_themes(app, context, self.child.clone());
        InheritedTheme {
            key: None,
            theme: self.clone(),
            child: CupertinoTheme::new(cupertino_theme_data, child).into_widget(),
        }
        .into_widget()
    }
}

/// Dart's private `_InheritedTheme`.
#[derive(Debug)]
struct InheritedTheme {
    key: Option<KeyRef>,
    theme: Theme,
    child: WidgetRef,
}

impl waft_widgets::InheritedTheme for InheritedTheme {
    fn wrap(&self, _app: &mut App, _context: BuildContext, child: WidgetRef) -> WidgetRef {
        Theme::new(Rc::clone(&self.theme.data), child).into_widget()
    }
}

impl InheritedWidget for InheritedTheme {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &InheritedTheme) -> bool {
        self.theme.data != old_widget.theme.data
    }

    fn as_inherited_theme(&self) -> Option<&dyn waft_widgets::InheritedTheme> {
        Some(self)
    }
}

/// An interpolation between two [`ThemeData`]s.
///
/// This class specializes the interpolation of `Tween<ThemeData>` to call the
/// [`ThemeData::lerp`] method.
///
/// See `Tween` for a discussion on how to use interpolation objects.
#[derive(Clone, Debug)]
pub struct ThemeDataTween {
    /// The value this variable has at the beginning of the animation.
    pub begin: Option<Rc<ThemeData>>,

    /// The value this variable has at the end of the animation.
    pub end: Option<Rc<ThemeData>>,
}

impl ThemeDataTween {
    /// Creates a [`ThemeData`] tween.
    ///
    /// The [`begin`](Self::begin) and [`end`](Self::end) properties must be non-null before
    /// the tween is first used, but the arguments can be null if the values are going to be
    /// filled in later.
    pub fn new(begin: Option<Rc<ThemeData>>, end: Option<Rc<ThemeData>>) -> ThemeDataTween {
        ThemeDataTween { begin, end }
    }

    /// Returns the value this variable has at the given animation clock value.
    pub fn lerp(&self, t: f64) -> Rc<ThemeData> {
        Rc::new(ThemeData::lerp(
            self.begin
                .as_ref()
                .expect("Tween.begin must be set before use"),
            self.end.as_ref().expect("Tween.end must be set before use"),
            t,
        ))
    }
}

impl Animatable<Rc<ThemeData>> for ThemeDataTween {
    fn transform(&self, _app: &App, t: f64) -> Rc<ThemeData> {
        if t == 0.0 {
            return self
                .begin
                .clone()
                .expect("Tween.begin must be set before use");
        }
        if t == 1.0 {
            return self.end.clone().expect("Tween.end must be set before use");
        }
        self.lerp(t)
    }
}

impl TweenValue for ThemeData {
    fn same(&self, other: &ThemeData) -> bool {
        self == other
    }
}

impl TweenSlot for ThemeDataTween {
    type Value = Rc<ThemeData>;

    fn begin(&self, _app: &App) -> Option<Rc<ThemeData>> {
        self.begin.clone()
    }

    fn set_begin(&mut self, _app: &mut App, begin: Option<Rc<ThemeData>>) {
        self.begin = begin;
    }

    fn end(&self, _app: &App) -> Option<Rc<ThemeData>> {
        self.end.clone()
    }

    fn set_end(&mut self, _app: &mut App, end: Option<Rc<ThemeData>>) {
        self.end = end;
    }
}

/// Animated version of [`Theme`] which automatically transitions the colors,
/// etc, over a given duration whenever the given theme changes.
///
/// See also:
///
///  * [`Theme`], which [`AnimatedTheme`] uses to actually apply the interpolated
///    theme.
///  * [`ThemeData`], which describes the actual configuration of a theme.
///  * `MaterialApp`, which includes an [`AnimatedTheme`] widget configured via
///    the `MaterialApp.theme` argument.
#[derive(Debug)]
pub struct AnimatedTheme {
    implicitly_animated_widget: ImplicitlyAnimatedWidgetData,

    /// Specifies the color and typography values for descendant widgets.
    pub data: Rc<ThemeData>,

    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl AnimatedTheme {
    /// Creates an animated theme.
    ///
    /// By default, the theme transition uses a linear curve.
    pub fn new<K>(data: impl Into<Rc<ThemeData>>, child: impl IntoWidget<K>) -> AnimatedTheme {
        AnimatedTheme {
            implicitly_animated_widget: ImplicitlyAnimatedWidgetData::new(
                K_THEME_ANIMATION_DURATION,
            ),
            data: data.into(),
            child: child.into_widget(),
        }
    }

    /// Dart `AnimatedTheme(duration:)`.
    pub fn duration(mut self, duration: Duration) -> AnimatedTheme {
        self.implicitly_animated_widget.duration = duration;
        self
    }

    waft_widgets::implicitly_animated_widget_setters!(AnimatedTheme);
}

impl ImplicitlyAnimatedWidget for AnimatedTheme {
    waft_widgets::implicitly_animated_widget_accessors!();
}

impl StatefulWidget for AnimatedTheme {
    type State = AnimatedThemeState;

    fn key(&self) -> Option<&KeyRef> {
        self.implicitly_animated_widget.key.as_ref()
    }

    fn create_state(&self) -> AnimatedThemeState {
        AnimatedThemeState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData::new(),
            data: None,
        }
    }
}

/// Dart's `_AnimatedThemeState`.
pub struct AnimatedThemeState {
    state: StateData<AnimatedTheme>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    implicitly_animated_widget_state: ImplicitlyAnimatedWidgetStateData,
    data: Option<ThemeDataTween>,
}

impl SingleTickerProviderStateMixin for AnimatedThemeState {
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

impl TickerProviderObject for AnimatedThemeState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl ImplicitlyAnimatedWidgetState for AnimatedThemeState {
    waft_widgets::implicitly_animated_widget_state_accessors!();

    fn for_each_tween<V: TweenVisitor>(self: Handle<Self>, app: &mut App, visitor: &mut V) {
        let tween = app.get(self).data.clone();
        let target = Some(Rc::clone(&self.widget(app).data));
        app.get_mut(self).data = visitor.visit(app, tween, target, |_app, value| {
            ThemeDataTween::new(Some(Rc::clone(value)), None)
        });
    }
}

impl AnimatedWidgetBaseState for AnimatedThemeState {}

impl State for AnimatedThemeState {
    type Widget = AnimatedTheme;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        AnimatedWidgetBaseState::init_state(self, app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &AnimatedTheme) {
        ImplicitlyAnimatedWidgetState::did_update_widget(self, app, old_widget);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        ImplicitlyAnimatedWidgetState::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let animation = self.animation(app);
        let data = app
            .get(self)
            .data
            .clone()
            .expect("for_each_tween made the tween");
        let child = self.widget(app).child.clone();
        Theme::new(data.evaluate(app, animation), child).into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use waft_cupertino::{CupertinoColors, CupertinoThemeData};
    use waft_embedder::{Color, TargetPlatform};
    use waft_foundation::AppCell;
    use waft_painting::AnyColor;
    use waft_widgets::{Builder, IconThemeData, MediaQueryData, RawView, SizedBox, run_widget};

    use super::*;
    use crate::color_scheme::ColorScheme;
    use crate::colors::{Colors, MaterialColor};
    use crate::test_support::{
        build, probe, pump, test_cell, test_cell_on, with_main_thread_stack,
    };
    use crate::text_selection_theme::TextSelectionThemeData;
    use crate::theme_data::ThemeDataBuilder;
    use crate::typography::Typography;

    fn themed(data: ThemeData) -> impl FnOnce(&App, WidgetRef) -> WidgetRef {
        move |_app, child| Theme::new(data, child).into_widget()
    }

    /// Mounts `child` under a bare `RawView`, which, unlike `View`, adds no `MediaQuery`.
    fn build_without_media_query(cell: &AppCell, child: WidgetRef) {
        {
            let mut app = cell.borrow_mut();
            let view = app.platform().views().into_iter().next().expect("a view");
            run_widget(&mut app, RawView::new(view, child).into_widget());
        }
        cell.elapse(Duration::ZERO);
        pump(&mut cell.borrow_mut(), Duration::ZERO);
    }

    #[test]
    fn theme_data_tween_control_test() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let app = cell.borrow();
            let light = Rc::new(ThemeData::new(&app));
            let dark = Rc::new(ThemeData::dark(&app, None));
            let tween = ThemeDataTween::new(Some(Rc::clone(&light)), Some(Rc::clone(&dark)));
            assert_eq!(*tween.lerp(0.25), ThemeData::lerp(&light, &dark, 0.25));
        });
    }

    #[test]
    fn brightness_of_is_the_media_query_brightness_without_a_theme() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let (brightness, maybe_brightness) = probe(
                &cell,
                |_app, child| {
                    MediaQuery::new(
                        MediaQueryData::new().platform_brightness(Brightness::Dark),
                        child,
                    )
                    .into_widget()
                },
                |app, context| {
                    (
                        Theme::brightness_of(app, context),
                        Theme::maybe_brightness_of(app, context),
                    )
                },
            );
            assert_eq!(brightness, Brightness::Dark);
            assert_eq!(maybe_brightness, Some(Brightness::Dark));
        });
    }

    #[test]
    fn brightness_of_prefers_the_theme_brightness_over_the_media_query() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let (brightness, maybe_brightness) = probe(
                &cell,
                |app, child| {
                    let data = ThemeData::builder()
                        .brightness(Brightness::Light)
                        .build(app);
                    MediaQuery::new(
                        MediaQueryData::new().platform_brightness(Brightness::Dark),
                        Theme::new(data, child),
                    )
                    .into_widget()
                },
                |app, context| {
                    (
                        Theme::brightness_of(app, context),
                        Theme::maybe_brightness_of(app, context),
                    )
                },
            );
            assert_eq!(brightness, Brightness::Light);
            assert_eq!(maybe_brightness, Some(Brightness::Light));
        });
    }

    #[test]
    fn brightness_of_is_light_and_maybe_brightness_of_is_none_without_a_theme_or_media_query() {
        let cell = test_cell();
        let seen = Rc::new(Cell::new(None));
        let probe = Builder::new({
            let seen = Rc::clone(&seen);
            move |app, context| {
                seen.set(Some((
                    Theme::brightness_of(app, context),
                    Theme::maybe_brightness_of(app, context),
                )));
                SizedBox::shrink().into_widget()
            }
        });
        build_without_media_query(&cell, probe.into_widget());
        assert_eq!(seen.get(), Some((Brightness::Light, None)));
    }

    #[test]
    fn fallback_theme_is_the_localized_fallback() {
        for use_material3 in [false, true] {
            with_main_thread_stack(|| {
                let cell = test_cell();
                let theme = probe(
                    &cell,
                    move |app, child| {
                        let data = ThemeData::builder().use_material3(use_material3).build(app);
                        Theme::new(data, child).into_widget()
                    },
                    Theme::of,
                );
                let mut app = cell.borrow_mut();
                let fallback = Rc::new(ThemeData::fallback(&app, Some(use_material3)));
                let geometry = Rc::new(if use_material3 {
                    Typography::english_like2021()
                } else {
                    Typography::english_like2014()
                });
                assert_eq!(theme, ThemeData::localize(&mut app, &fallback, &geometry));
            });
        }
    }

    #[test]
    fn theme_data_localize_memoizes_the_result() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let mut app = cell.borrow_mut();
            let light = Rc::new(ThemeData::new(&app));
            let dark = Rc::new(ThemeData::dark(&app, None));
            let default_geometry_theme = Rc::new(Typography::english_like2014());
            let tall2014 = Rc::new(Typography::tall2014());

            // Same input, same output.
            let first = ThemeData::localize(&mut app, &light, &default_geometry_theme);
            let second = ThemeData::localize(&mut app, &light, &default_geometry_theme);
            assert!(Rc::ptr_eq(&first, &second));

            // Different text geometry, different output.
            let tall = ThemeData::localize(&mut app, &light, &tall2014);
            assert!(!Rc::ptr_eq(&first, &tall));

            // Different base theme, different output.
            let dark_localized = ThemeData::localize(&mut app, &dark, &default_geometry_theme);
            assert!(!Rc::ptr_eq(&first, &dark_localized));
        });
    }

    #[test]
    fn fallback_theme_follows_the_target_platform() {
        for platform in [
            TargetPlatform::Android,
            TargetPlatform::Fuchsia,
            TargetPlatform::IOS,
            TargetPlatform::Linux,
            TargetPlatform::MacOS,
            TargetPlatform::Windows,
        ] {
            with_main_thread_stack(move || {
                let cell = test_cell_on(platform);
                let theme = probe(&cell, |_app, child| child, Theme::of);
                assert_eq!(theme.platform, platform);
            });
        }
    }

    #[test]
    fn theme_data_with_null_typography_uses_proper_defaults() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let app = cell.borrow();
            let m2_theme = ThemeData::builder().use_material3(false).build(&app);
            assert_eq!(m2_theme.typography, Typography::material2014().build());
            let m3_theme = ThemeData::new(&app);
            assert_eq!(
                m3_theme.typography,
                Typography::material2021()
                    .color_scheme(m3_theme.color_scheme.clone())
                    .build()
            );
        });
    }

    #[test]
    fn icon_themes_are_applied_and_animated() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let seen = Rc::new(Cell::new(None));
            let child = Builder::new({
                let seen = Rc::clone(&seen);
                move |app, context| {
                    let icon_theme = IconTheme::of(app, context);
                    seen.set(Some((
                        icon_theme.color.as_ref().map(AnyColor::color),
                        icon_theme.size,
                    )));
                    SizedBox::shrink().into_widget()
                }
            })
            .into_widget();
            let animated = |app: &App, color: Color, size: f64| {
                let data = ThemeData::builder()
                    .icon_theme(IconThemeData::new().color(color).size(size))
                    .build(app);
                AnimatedTheme::new(data, child.clone()).into_widget()
            };
            let green = Colors::GREEN.primary();
            let orange = Colors::ORANGE.primary();

            let tree = animated(&cell.borrow(), green, 10.0);
            build(&cell, tree);
            assert_eq!(seen.get(), Some((Some(green), Some(10.0))));

            let tree = animated(&cell.borrow(), orange, 20.0);
            build(&cell, tree);
            // Halfway through the theme transition
            pump(&mut cell.borrow_mut(), Duration::from_millis(100));
            let (color, size) = seen.get().expect("the child built");
            assert_eq!(color, Color::lerp(Some(green), Some(orange), 0.5));
            assert_eq!(size, Some(15.0));

            // Finish the transition
            pump(&mut cell.borrow_mut(), Duration::from_millis(200));
            assert_eq!(seen.get(), Some((Some(orange), Some(20.0))));
        });
    }

    #[test]
    fn same_theme_data_reapplied_does_not_trigger_descendant_rebuilds() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let builds = Rc::new(Cell::new(0));
            let child = Builder::new({
                let builds = Rc::clone(&builds);
                move |app, context| {
                    Theme::of(app, context);
                    builds.set(builds.get() + 1);
                    SizedBox::shrink().into_widget()
                }
            })
            .into_widget();
            let build_theme = |primary_color: u32| {
                let data = ThemeData::builder()
                    .primary_color(Color::new(primary_color))
                    .build(&cell.borrow());
                Theme::new(data, child.clone()).into_widget()
            };

            build(&cell, build_theme(0xFF000000));
            assert_eq!(builds.get(), 1);

            // New instance of theme data but still the same content.
            build(&cell, build_theme(0xFF000000));
            // Still no repeated calls.
            assert_eq!(builds.get(), 1);

            // Different now.
            build(&cell, build_theme(0xFF222222));
            // Should call build again.
            assert_eq!(builds.get(), 2);
        });
    }

    #[test]
    fn text_geometry_set_in_theme_has_higher_precedence_than_that_of_localizations() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let k_magic_font_size = 4321.0;
            let custom_theme = {
                let fallback = ThemeData::fallback(&cell.borrow(), None);
                let body_medium = fallback
                    .primary_text_theme
                    .body_medium
                    .as_ref()
                    .expect("a body style")
                    .copy_with()
                    .font_size(k_magic_font_size);
                fallback
                    .copy_with()
                    .primary_text_theme(
                        fallback
                            .primary_text_theme
                            .copy_with()
                            .body_medium(body_medium),
                    )
                    .build()
            };
            let actual_font_size = probe(&cell, themed(custom_theme), |app, context| {
                Theme::of(app, context)
                    .primary_text_theme
                    .body_medium
                    .as_ref()
                    .and_then(|style| style.font_size)
            });
            assert_eq!(actual_font_size, Some(k_magic_font_size));
        });
    }

    #[test]
    fn material3_default_theme_provides_all_basic_text_style_properties() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let theme = probe(
                &cell,
                |app, child| themed(ThemeData::new(app))(app, child),
                Theme::of,
            );
            for text_theme in [&theme.text_theme, &theme.primary_text_theme] {
                let styles = [
                    &text_theme.display_large,
                    &text_theme.display_medium,
                    &text_theme.display_small,
                    &text_theme.headline_large,
                    &text_theme.headline_medium,
                    &text_theme.headline_small,
                    &text_theme.title_large,
                    &text_theme.title_medium,
                    &text_theme.body_large,
                    &text_theme.body_medium,
                    &text_theme.body_small,
                    &text_theme.label_large,
                    &text_theme.label_medium,
                ];
                for style in styles {
                    let style = style.as_ref().expect("a style");
                    assert!(!style.inherit);
                    assert!(style.color.is_some());
                    assert!(style.font_family.is_some());
                    assert!(style.font_size.is_some());
                    assert!(style.font_weight.is_some());
                    assert!(style.font_style.is_none());
                    assert!(style.letter_spacing.is_some());
                    assert!(style.word_spacing.is_none());
                    assert!(style.text_baseline.is_some());
                    assert!(style.height.is_some());
                    assert_eq!(style.decoration, Some(waft_embedder::TextDecoration::NONE));
                    assert!(style.decoration_color.is_some());
                    assert!(style.decoration_style.is_none());
                    assert!(style.debug_label.is_some());
                    assert!(style.background.is_none());
                }
            }
            assert_eq!(
                theme
                    .text_theme
                    .display_large
                    .as_ref()
                    .and_then(|style| style.debug_label.clone())
                    .as_deref(),
                Some(
                    "(englishLike displayLarge 2021).merge((blackMountainView displayLarge).apply)"
                )
            );
        });
    }

    #[test]
    fn inherited_theme_capture_reproduces_the_theme_elsewhere() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let (captured, theme) = probe(
                &cell,
                |app, child| {
                    let data = ThemeData::builder()
                        .primary_color(Colors::RED.primary())
                        .build(app);
                    Theme::new(data, child).into_widget()
                },
                |app, context| {
                    (
                        <dyn waft_widgets::InheritedTheme>::capture(app, context, None),
                        Theme::of(app, context),
                    )
                },
            );
            let recaptured = probe(&cell, |_app, child| captured.wrap(child), Theme::of);
            assert_eq!(recaptured, theme);
            assert_eq!(recaptured.primary_color, Colors::RED.primary());
        });
    }

    /// The `CupertinoThemeData` a probe under `Theme(data:)` reads with `CupertinoTheme::of`.
    fn cupertino_theme_of(
        cell: &AppCell,
        data: impl FnOnce(&App) -> ThemeData,
    ) -> CupertinoThemeData {
        probe(
            cell,
            |app, child| Theme::new(data(app), child).into_widget(),
            CupertinoTheme::of,
        )
    }

    fn any(color: Color) -> AnyColor {
        AnyColor::new(color)
    }

    #[test]
    fn material2_default_light_and_dark_cupertino_themes_have_defaults() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let light = cupertino_theme_of(&cell, |app| {
                ThemeData::builder().use_material3(false).build(app)
            });
            assert_eq!(light.brightness(), Some(Brightness::Light));
            assert_eq!(light.primary_color(), any(Colors::BLUE.primary()));
            assert_eq!(
                light.scaffold_background_color(),
                any(Colors::GREY.shade50())
            );
            assert_eq!(light.primary_contrasting_color(), any(Colors::WHITE));
            let text_style = light.text_theme().text_style();
            assert_eq!(
                text_style.font_family.as_deref(),
                Some("CupertinoSystemText")
            );
            assert_eq!(text_style.font_size, Some(17.0));

            let cell = test_cell();
            let dark = cupertino_theme_of(&cell, |app| ThemeData::dark(app, Some(false)));
            assert_eq!(dark.brightness(), Some(Brightness::Dark));
            assert_eq!(dark.primary_color(), any(Colors::BLUE.primary()));
            assert_eq!(dark.primary_contrasting_color(), any(Colors::WHITE));
            assert_eq!(
                dark.scaffold_background_color(),
                any(Colors::GREY.get(&850).expect("grey 850"))
            );
            let text_style = dark.text_theme().text_style();
            assert_eq!(
                text_style.font_family.as_deref(),
                Some("CupertinoSystemText")
            );
            assert_eq!(text_style.font_size, Some(17.0));
        });
    }

    #[test]
    fn material3_default_light_and_dark_cupertino_themes_have_defaults() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let light = cupertino_theme_of(&cell, ThemeData::new);
            assert_eq!(light.brightness(), Some(Brightness::Light));
            assert_eq!(light.primary_color(), any(Color::new(0xff6750a4)));
            // ColorScheme.background
            assert_eq!(
                light.scaffold_background_color(),
                any(Color::new(0xfffef7ff))
            );
            assert_eq!(light.primary_contrasting_color(), any(Colors::WHITE));
            let text_style = light.text_theme().text_style();
            assert_eq!(
                text_style.font_family.as_deref(),
                Some("CupertinoSystemText")
            );
            assert_eq!(text_style.font_size, Some(17.0));

            let cell = test_cell();
            let dark = cupertino_theme_of(&cell, |app| ThemeData::dark(app, None));
            assert_eq!(dark.brightness(), Some(Brightness::Dark));
            assert_eq!(dark.primary_color(), any(Color::new(0xffd0bcff)));
            assert_eq!(
                dark.primary_contrasting_color(),
                any(Color::new(0xff381e72))
            );
            assert_eq!(
                dark.scaffold_background_color(),
                any(Color::new(0xff141218))
            );
        });
    }

    #[test]
    fn material_theme_overrides_the_cupertino_brightness() {
        with_main_thread_stack(|| {
            let brightness_of = |data: fn(&App) -> ThemeData| {
                let cell = test_cell();
                probe(
                    &cell,
                    |app, child| Theme::new(data(app), child).into_widget(),
                    CupertinoTheme::brightness_of,
                )
            };
            assert_eq!(
                brightness_of(|app| ThemeData::dark(app, None)),
                Brightness::Dark
            );
            assert_eq!(brightness_of(ThemeData::new), Brightness::Light);

            // Overridable by cupertinoOverrideTheme.
            assert_eq!(
                brightness_of(|app| {
                    ThemeData::builder()
                        .brightness(Brightness::Light)
                        .cupertino_override_theme(
                            CupertinoThemeData::new()
                                .with_brightness(Brightness::Dark)
                                .no_default(),
                        )
                        .build(app)
                }),
                Brightness::Dark
            );
            assert_eq!(
                brightness_of(|app| {
                    ThemeData::builder()
                        .brightness(Brightness::Dark)
                        .cupertino_override_theme(
                            CupertinoThemeData::new()
                                .with_brightness(Brightness::Light)
                                .no_default(),
                        )
                        .build(app)
                }),
                Brightness::Light
            );
        });
    }

    #[test]
    fn cupertino_override_theme_overrides_the_material_theme_and_keeps_the_rest_cascaded() {
        for use_material3 in [false, true] {
            with_main_thread_stack(|| {
                let cell = test_cell();
                let theme = cupertino_theme_of(&cell, move |app| {
                    ThemeData::builder()
                        .cupertino_override_theme(
                            CupertinoThemeData::new()
                                .with_scaffold_background_color(
                                    CupertinoColors::LIGHT_BACKGROUND_GRAY,
                                )
                                .no_default(),
                        )
                        .use_material3(use_material3)
                        .build(app)
                });

                assert_eq!(theme.brightness(), Some(Brightness::Light));
                // We took the scaffold background override but the rest are still cascaded
                // to the material theme.
                let primary = if use_material3 {
                    Color::new(0xff6750a4)
                } else {
                    Colors::BLUE.primary()
                };
                assert_eq!(theme.primary_color(), any(primary));
                assert_eq!(theme.primary_contrasting_color(), any(Colors::WHITE));
                assert_eq!(
                    theme.scaffold_background_color(),
                    CupertinoColors::LIGHT_BACKGROUND_GRAY
                );
                let text_style = theme.text_theme().text_style();
                assert_eq!(
                    text_style.font_family.as_deref(),
                    Some("CupertinoSystemText")
                );
                assert_eq!(text_style.font_size, Some(17.0));
            });
        }
    }

    #[test]
    fn cupertino_override_theme_overrides_properties_that_are_independent_of_material() {
        for use_material3 in [false, true] {
            with_main_thread_stack(|| {
                let cell = test_cell();
                let theme = cupertino_theme_of(&cell, move |app| {
                    ThemeData::builder()
                        .cupertino_override_theme(
                            CupertinoThemeData::new()
                                // The bar colors ignore all things material except brightness.
                                .with_bar_background_color(CupertinoColors::BLACK)
                                .no_default(),
                        )
                        .use_material3(use_material3)
                        .build(app)
                });

                let primary = if use_material3 {
                    Color::new(0xff6750a4)
                } else {
                    Colors::BLUE.primary()
                };
                assert_eq!(theme.primary_color(), any(primary));
                // MaterialBasedCupertinoThemeData should also function like a normal
                // CupertinoThemeData.
                assert_eq!(theme.bar_background_color(), CupertinoColors::BLACK);
            });
        }
    }

    #[test]
    fn changing_the_material_theme_rebuilds_cupertino_dependents() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let builds = Rc::new(Cell::new(0));
            let primary = Rc::new(Cell::new(None));
            let child = Builder::new({
                let builds = Rc::clone(&builds);
                let primary = Rc::clone(&primary);
                move |app, context| {
                    builds.set(builds.get() + 1);
                    primary.set(Some(
                        CupertinoTheme::of(app, context).primary_color().color(),
                    ));
                    SizedBox::shrink().into_widget()
                }
            })
            .into_widget();
            let build_theme = |color: Color| {
                let data = ThemeData::builder()
                    .color_scheme(ColorScheme::light().with_primary(color))
                    .build(&cell.borrow());
                Theme::new(data, child.clone()).into_widget()
            };

            build(&cell, build_theme(Colors::RED.primary()));
            assert_eq!(builds.get(), 1);
            assert_eq!(primary.get(), Some(Colors::RED.primary()));

            build(&cell, build_theme(Colors::ORANGE.primary()));
            assert_eq!(builds.get(), 2);
            assert_eq!(primary.get(), Some(Colors::ORANGE.primary()));
        });
    }

    #[test]
    fn cupertino_theme_data_does_not_override_the_material_icon_theme() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let material_icon_color = Colors::BLUE.primary();
            let cupertino_icon_color = Colors::BLACK;
            let icon_color = probe(
                &cell,
                move |app, child| {
                    let data = ThemeData::builder()
                        .icon_theme(IconThemeData::new().color(material_icon_color))
                        .cupertino_override_theme(
                            CupertinoThemeData::new()
                                .with_primary_color(cupertino_icon_color)
                                .no_default(),
                        )
                        .build(app);
                    Theme::new(data, child).into_widget()
                },
                |app, context| IconTheme::of(app, context).color,
            );
            assert_eq!(icon_color, Some(any(material_icon_color)));
        });
    }

    /// Dart's `testTheme` over its `singletonThemeSubtree`: each call pumps `Theme(data:)` over
    /// the same subtree, which counts its builds and keeps the `CupertinoThemeData` it read.
    struct CupertinoThemeTester {
        cell: Rc<AppCell>,
        build_count: Rc<Cell<usize>>,
        actual_theme: Rc<RefCell<Option<CupertinoThemeData>>>,
        singleton_theme_subtree: WidgetRef,
    }

    impl CupertinoThemeTester {
        fn new() -> CupertinoThemeTester {
            let build_count = Rc::new(Cell::new(0));
            let actual_theme = Rc::new(RefCell::new(None));
            let singleton_theme_subtree = Builder::new({
                let build_count = Rc::clone(&build_count);
                let actual_theme = Rc::clone(&actual_theme);
                move |app, context| {
                    build_count.set(build_count.get() + 1);
                    *actual_theme.borrow_mut() = Some(CupertinoTheme::of(app, context));
                    SizedBox::shrink().into_widget()
                }
            })
            .into_widget();
            CupertinoThemeTester {
                cell: test_cell(),
                build_count,
                actual_theme,
                singleton_theme_subtree,
            }
        }

        fn test_theme(&self, theme: impl FnOnce(&App) -> ThemeData) -> CupertinoThemeData {
            let data = theme(&self.cell.borrow());
            build(
                &self.cell,
                Theme::new(data, self.singleton_theme_subtree.clone()).into_widget(),
            );
            let actual_theme = self.actual_theme.borrow().clone();
            actual_theme.expect("the subtree built")
        }

        fn build_count(&self) -> usize {
            self.build_count.get()
        }
    }

    /// The Material theme whose primary colour is `primary`: Dart's `primarySwatch` with
    /// `useMaterial3: false` in Material 2, `ColorScheme.light(primary:)` in Material 3.
    fn material_theme_with_primary(
        use_material3: bool,
        primary: MaterialColor,
    ) -> ThemeDataBuilder {
        if use_material3 {
            ThemeData::builder().color_scheme(ColorScheme::light().with_primary(primary.primary()))
        } else {
            ThemeData::builder()
                .use_material3(false)
                .primary_swatch(primary)
        }
    }

    #[test]
    fn changing_cupertino_theme_override_triggers_rebuilds() {
        with_main_thread_stack(|| {
            let tester = CupertinoThemeTester::new();
            let theme = tester.test_theme(|app| {
                ThemeData::builder()
                    .primary_swatch(Colors::PURPLE)
                    .cupertino_override_theme(
                        CupertinoThemeData::new()
                            .with_primary_color(CupertinoColors::ACTIVE_ORANGE)
                            .no_default(),
                    )
                    .build(app)
            });
            assert_eq!(tester.build_count(), 1);
            assert_eq!(theme.primary_color(), CupertinoColors::ACTIVE_ORANGE);

            let theme = tester.test_theme(|app| {
                ThemeData::builder()
                    .primary_swatch(Colors::PURPLE)
                    .cupertino_override_theme(
                        CupertinoThemeData::new()
                            .with_primary_color(CupertinoColors::ACTIVE_GREEN)
                            .no_default(),
                    )
                    .build(app)
            });
            assert_eq!(tester.build_count(), 2);
            assert_eq!(theme.primary_color(), CupertinoColors::ACTIVE_GREEN);
        });
    }

    #[test]
    fn cupertino_theme_override_blocks_derivative_changes() {
        with_main_thread_stack(|| {
            let tester = CupertinoThemeTester::new();
            let theme = tester.test_theme(|app| {
                ThemeData::builder()
                    .primary_swatch(Colors::PURPLE)
                    .cupertino_override_theme(
                        CupertinoThemeData::new()
                            .with_primary_color(CupertinoColors::ACTIVE_ORANGE)
                            .no_default(),
                    )
                    .build(app)
            });
            assert_eq!(tester.build_count(), 1);
            assert_eq!(theme.primary_color(), CupertinoColors::ACTIVE_ORANGE);

            // Change the upstream material primary color.
            let theme = tester.test_theme(|app| {
                ThemeData::builder()
                    .primary_swatch(Colors::BLUE)
                    .cupertino_override_theme(
                        CupertinoThemeData::new()
                            // But the primary material color is preempted by the override.
                            .with_primary_color(CupertinoColors::SYSTEM_RED)
                            .no_default(),
                    )
                    .build(app)
            });
            assert_eq!(tester.build_count(), 2);
            assert_eq!(theme.primary_color(), CupertinoColors::SYSTEM_RED);
        });
    }

    #[test]
    fn cupertino_overrides_do_not_block_derivatives_triggering_rebuilds_when_derivatives_are_not_overridden()
     {
        for use_material3 in [false, true] {
            with_main_thread_stack(|| {
                let tester = CupertinoThemeTester::new();
                let theme = tester.test_theme(|app| {
                    material_theme_with_primary(use_material3, Colors::PURPLE)
                        .cupertino_override_theme(
                            CupertinoThemeData::new()
                                .with_primary_contrasting_color(CupertinoColors::DESTRUCTIVE_RED)
                                .no_default(),
                        )
                        .build(app)
                });
                assert_eq!(tester.build_count(), 1);
                assert_eq!(
                    theme.text_theme().action_text_style().color,
                    Some(any(Colors::PURPLE.primary()))
                );
                assert_eq!(
                    theme.primary_contrasting_color(),
                    CupertinoColors::DESTRUCTIVE_RED
                );

                let theme = tester.test_theme(|app| {
                    material_theme_with_primary(use_material3, Colors::GREEN)
                        .cupertino_override_theme(
                            CupertinoThemeData::new()
                                .with_primary_contrasting_color(CupertinoColors::DESTRUCTIVE_RED)
                                .no_default(),
                        )
                        .build(app)
                });
                assert_eq!(tester.build_count(), 2);
                assert_eq!(
                    theme.text_theme().action_text_style().color,
                    Some(any(Colors::GREEN.primary()))
                );
                assert_eq!(
                    theme.primary_contrasting_color(),
                    CupertinoColors::DESTRUCTIVE_RED
                );
            });
        }
    }

    #[test]
    fn copy_with_only_copies_the_overrides_not_the_material_or_cupertino_derivatives() {
        for use_material3 in [false, true] {
            with_main_thread_stack(|| {
                let tester = CupertinoThemeTester::new();
                let original_theme = tester.test_theme(|app| {
                    material_theme_with_primary(use_material3, Colors::PURPLE)
                        .cupertino_override_theme(
                            CupertinoThemeData::new()
                                .with_primary_contrasting_color(CupertinoColors::ACTIVE_ORANGE)
                                .no_default(),
                        )
                        .build(app)
                });

                let copied_theme = original_theme
                    .copy_with()
                    .with_bar_background_color(CupertinoColors::DESTRUCTIVE_RED);

                let theme = tester.test_theme(|app| {
                    material_theme_with_primary(use_material3, Colors::BLUE)
                        .cupertino_override_theme(copied_theme.no_default())
                        .build(app)
                });

                assert_eq!(theme.primary_color(), any(Colors::BLUE.primary()));
                assert_eq!(
                    theme.primary_contrasting_color(),
                    CupertinoColors::ACTIVE_ORANGE
                );
                assert_eq!(
                    theme.bar_background_color(),
                    CupertinoColors::DESTRUCTIVE_RED
                );
            });
        }
    }

    #[test]
    fn material_themes_with_no_cupertino_overrides_can_also_be_copy_with_ed() {
        for use_material3 in [false, true] {
            with_main_thread_stack(|| {
                let tester = CupertinoThemeTester::new();
                let original_theme = tester.test_theme(|app| {
                    material_theme_with_primary(use_material3, Colors::PURPLE).build(app)
                });

                let copied_theme = original_theme
                    .copy_with()
                    .with_primary_contrasting_color(CupertinoColors::DESTRUCTIVE_RED);

                let theme = tester.test_theme(|app| {
                    material_theme_with_primary(use_material3, Colors::BLUE)
                        .cupertino_override_theme(copied_theme.no_default())
                        .build(app)
                });

                assert_eq!(theme.primary_color(), any(Colors::BLUE.primary()));
                assert_eq!(
                    theme.primary_contrasting_color(),
                    CupertinoColors::DESTRUCTIVE_RED
                );
            });
        }
    }

    #[test]
    fn material_based_cupertino_theme_data_resolves_from_the_material_theme() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let (material, resolved) = probe(
                &cell,
                |_app, child| child,
                |app, context| {
                    let material = Rc::new(ThemeData::dark(app, None));
                    let resolved =
                        MaterialBasedCupertinoThemeData::new(&material).resolve_from(app, context);
                    (material, resolved)
                },
            );
            assert_eq!(resolved.brightness(), Some(Brightness::Dark));
            assert_eq!(resolved.primary_color(), any(material.color_scheme.primary));
            assert_eq!(
                resolved.primary_contrasting_color(),
                any(material.color_scheme.on_primary)
            );
            assert_eq!(
                resolved.scaffold_background_color(),
                any(material.scaffold_background_color)
            );
            // The override theme part is what `no_default` reports: here only the text theme
            // `resolve_from` puts there.
            let no_default = resolved.no_default();
            assert!(no_default.primary_color.is_none());
            assert!(no_default.text_theme.is_some());
            // The text theme derives from the Material primary colour.
            assert_eq!(
                resolved.text_theme().action_text_style().color,
                Some(any(material.color_scheme.primary))
            );
            assert_eq!(
                resolved
                    .copy_with()
                    .with_primary_color(Colors::RED.primary())
                    .primary_color(),
                any(Colors::RED.primary())
            );
            // The iOS defaults stay dynamic: only the override theme part is resolved.
            assert_eq!(
                resolved.bar_background_color(),
                CupertinoThemeData::new().bar_background_color()
            );
        });
    }

    fn selection_colors(
        app: &mut App,
        context: BuildContext,
    ) -> (Option<AnyColor>, Option<AnyColor>) {
        let style = DefaultSelectionStyle::of(app, context);
        (style.selection_color, style.cursor_color)
    }

    #[test]
    fn theme_provides_a_default_selection_style_from_its_text_selection_theme() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let seen = probe(
                &cell,
                |app, child| {
                    let data = ThemeData::builder()
                        .text_selection_theme(
                            TextSelectionThemeData::new()
                                .selection_color(Color::new(0x88888888))
                                .cursor_color(Color::new(0xffaabbcc)),
                        )
                        .build(app);
                    Theme::new(data, child).into_widget()
                },
                selection_colors,
            );
            assert_eq!(
                seen,
                (
                    Some(any(Color::new(0x88888888))),
                    Some(any(Color::new(0xffaabbcc)))
                )
            );
        });
    }

    #[test]
    fn theme_keeps_the_ambient_selection_style_where_its_text_selection_theme_is_unset() {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let seen = probe(
                &cell,
                |app, child| {
                    let data = ThemeData::builder()
                        .text_selection_theme(
                            TextSelectionThemeData::new().cursor_color(Color::new(0xffaabbcc)),
                        )
                        .build(app);
                    DefaultSelectionStyle::new(Theme::new(data, child))
                        .selection_color(Color::new(0xffaa1111))
                        .cursor_color(Color::new(0x00cc2222))
                        .into_widget()
                },
                selection_colors,
            );
            assert_eq!(
                seen,
                (
                    Some(any(Color::new(0xffaa1111))),
                    Some(any(Color::new(0xffaabbcc)))
                )
            );
        });
    }

    #[test]
    fn material_based_cupertino_theme_data_takes_the_selection_handle_color_from_the_text_selection_theme()
     {
        with_main_thread_stack(|| {
            let cell = test_cell();
            let (from_material, overridden) = probe(
                &cell,
                |_app, child| child,
                |app, _context| {
                    let material = ThemeData::builder()
                        .text_selection_theme(
                            TextSelectionThemeData::new()
                                .selection_handle_color(Color::new(0xaabbccdd)),
                        )
                        .build(app);
                    let theme = MaterialBasedCupertinoThemeData::new(&Rc::new(material));
                    (
                        theme.selection_handle_color(),
                        theme
                            .copy_with()
                            .with_selection_handle_color(Colors::RED.primary())
                            .selection_handle_color(),
                    )
                },
            );
            assert_eq!(from_material, any(Color::new(0xaabbccdd)));
            assert_eq!(overridden, any(Colors::RED.primary()));
        });
    }

    #[test]
    fn cupertino_widgets_correctly_get_the_right_text_theme_in_dark_mode() {
        with_main_thread_stack(|| {
            let mut tester = crate::test_support::WidgetTester::on(TargetPlatform::Android);
            tester.install_fonts();
            let text_field_key = waft_widgets::GlobalKey::new();
            let theme = ThemeData::dark(&tester.cell.borrow(), None);
            tester.pump_widget(crate::test_support::material_app(
                theme,
                waft_cupertino::CupertinoTextField::new()
                    .key(text_field_key.clone().into())
                    .into_widget(),
            ));
            tester.pump_and_settle();

            let editable_text = tester.find_by_type::<waft_widgets::EditableText>();
            assert_eq!(editable_text.len(), 1);
            let style_color = tester
                .widget::<waft_widgets::EditableText, _>(editable_text[0], |editable_text| {
                    editable_text.style.color.clone()
                });

            // Default CupertinoTextStyle color is a CupertinoDynamicColor.
            let cupertino_theme_data = {
                let mut app = tester.cell.borrow_mut();
                let context = text_field_key
                    .current_context(&app)
                    .expect("the text field is mounted");
                CupertinoTheme::of(&mut app, context)
            };
            let text_style_color = cupertino_theme_data
                .text_theme()
                .text_style()
                .color
                .expect("a text colour");
            let theme_text_style_color = text_style_color
                .extension::<waft_cupertino::CupertinoDynamicColor>()
                .expect("a CupertinoDynamicColor");

            // The value of the textfield's color should resolve to the theme's dark color.
            assert_eq!(
                style_color.map(|color| color.to_argb32()),
                Some(theme_text_style_color.dark_color.to_argb32())
            );
        });
    }
}
