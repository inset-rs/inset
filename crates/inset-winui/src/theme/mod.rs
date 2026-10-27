//! WinUI's theme resources: the XAML `ResourceDictionary.ThemeDictionaries` (light, dark) and the OS accent palette, reachable from any widget through [`ThemeResources::of`] the way a template reaches `{ThemeResource X}`.

mod generated;
mod typography;
pub use generated::*;
pub use typography::*;

use std::fmt;
use std::rc::Rc;

use inset_embedder::Color;
use inset_foundation::App;
use inset_widgets::{
    BuildContext, InheritedWidget, IntoWidget, MediaQuery, StatelessWidget, WidgetRef,
};

/// The source AcrylicBrush tint, luminosity, and fallback recipe.
/// `Brush::Acrylic` paints the in-window material using these values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AcrylicBrushResources {
    /// The colour blended over the blurred backdrop.
    pub tint_color: Color,
    /// The opacity applied to the tint colour.
    pub tint_opacity: f64,
    /// An explicit luminosity-layer opacity, or the source's automatic calculation.
    pub tint_luminosity_opacity: Option<f64>,
    /// The source colour used when acrylic composition is unavailable.
    pub fallback_color: Color,
}

/// XAML `ElementTheme` / the theme dictionary a lookup resolves in. High contrast is deferred: its values are the OS's system colours.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

/// The `SystemAccentColor*` resources the OS supplies to every XAML app: the accent and its three lighter and three darker shades. Light theme controls use `dark1`, dark theme controls `light2`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccentPalette {
    pub base: Color,
    pub light1: Color,
    pub light2: Color,
    pub light3: Color,
    pub dark1: Color,
    pub dark2: Color,
    pub dark3: Color,
}

impl AccentPalette {
    /// Windows 11's default blue. `dark1` (light-theme controls, #0067C0) and `light2` (dark-theme controls, #4CC2FF) are the published defaults; the other shades are the ramp Windows shows for that blue and should be verified against a Windows machine.
    pub const WINDOWS_BLUE: AccentPalette = AccentPalette {
        base: Color::from_argb(255, 0, 120, 212),
        light1: Color::from_argb(255, 0, 145, 248),
        light2: Color::from_argb(255, 76, 194, 255),
        light3: Color::from_argb(255, 153, 235, 255),
        dark1: Color::from_argb(255, 0, 103, 192),
        dark2: Color::from_argb(255, 0, 62, 146),
        dark3: Color::from_argb(255, 0, 26, 104),
    };
}

impl Default for AccentPalette {
    fn default() -> Self {
        Self::WINDOWS_BLUE
    }
}

impl ThemeResources {
    /// The nearest [`ThemeScope`]'s resources, or WinUI's (light or dark from the media query, the default accent) when none encloses `context`.
    pub fn of(app: &mut App, context: BuildContext) -> Rc<ThemeResources> {
        if let Some(scope) =
            context.depend_on_inherited_widget_of_exact_type::<InheritedResources>(app)
        {
            return scope.resources.clone();
        }
        let theme = match MediaQuery::maybe_platform_brightness_of(app, context) {
            Some(inset_embedder::Brightness::Dark) => Theme::Dark,
            _ => Theme::Light,
        };
        ThemeResources::default_for(theme)
    }

    /// WinUI's resources for `theme` in the default accent, made once per thread.
    fn default_for(theme: Theme) -> Rc<ThemeResources> {
        thread_local! {
            static LIGHT: Rc<ThemeResources> = Rc::new(ThemeResources::new(Theme::Light, AccentPalette::default()));
            static DARK: Rc<ThemeResources> = Rc::new(ThemeResources::new(Theme::Dark, AccentPalette::default()));
        }
        match theme {
            Theme::Light => LIGHT.with(Rc::clone),
            Theme::Dark => DARK.with(Rc::clone),
        }
    }
}

/// Gives a subtree its theme resources, as XAML's `RequestedTheme` and resource dictionaries do: [`ThemeScope::new`] a theme and accent with WinUI's values, [`ThemeScope::overriding`] the enclosing scope's, and [`ThemeScope::with_resources`] resolved ones. Its `any`, `light` and `dark` then change them, as a dictionary's entries and its `ThemeDictionaries` do: `any` in both themes, then the one for the theme shown.
pub struct ThemeScope {
    base: Base,
    any: Option<Override>,
    light: Option<Override>,
    dark: Option<Override>,
    child: WidgetRef,
}

/// What a [`ThemeScope`] starts from, before its own values.
#[allow(clippy::large_enum_variant, reason = "a scope is made once per build")]
enum Base {
    /// WinUI's values for the theme in the palette; XAML `RequestedTheme`, with the accent the OS would supply.
    Theme(Theme, AccentPalette),
    /// The enclosing scope's resources; XAML `<Control.Resources>`.
    Enclosing,
    /// Resources resolved elsewhere, such as a popup's target's.
    Resources(Rc<ThemeResources>),
}

/// A change to some of a scope's resources.
type Override = Rc<dyn Fn(&mut ThemeResources)>;

impl ThemeScope {
    /// `new` shows `child` in `theme`, with WinUI's values in the default accent. It does not keep an enclosing scope's values: XAML's `RequestedTheme` would.
    pub fn new<K>(theme: Theme, child: impl IntoWidget<K>) -> ThemeScope {
        ThemeScope::from_base(Base::Theme(theme, AccentPalette::default()), child)
    }

    /// `overriding` shows `child` with the enclosing scope's resources, changed by its own.
    pub fn overriding<K>(child: impl IntoWidget<K>) -> ThemeScope {
        ThemeScope::from_base(Base::Enclosing, child)
    }

    /// `with_resources` shows `child` with `resources`. A popup's content takes its target's this way, since the overlay it shows in is outside the target's scope.
    pub fn with_resources<K>(
        resources: Rc<ThemeResources>,
        child: impl IntoWidget<K>,
    ) -> ThemeScope {
        ThemeScope::from_base(Base::Resources(resources), child)
    }

    fn from_base<K>(base: Base, child: impl IntoWidget<K>) -> ThemeScope {
        ThemeScope {
            base,
            any: None,
            light: None,
            dark: None,
            child: child.into_widget(),
        }
    }

    /// `accent` sets the palette of a scope made with [`ThemeScope::new`]; the others take theirs from their base. Panics otherwise.
    pub fn accent(mut self, accent: AccentPalette) -> ThemeScope {
        match &mut self.base {
            Base::Theme(_, palette) => *palette = accent,
            _ => panic!("`accent` sets the palette of a scope made with `ThemeScope::new`"),
        }
        self
    }

    /// `any` changes the resources in both themes.
    pub fn any(mut self, change: impl Fn(&mut ThemeResources) + 'static) -> ThemeScope {
        self.any = Some(Rc::new(change));
        self
    }

    /// `light` changes the resources in the light theme, after `any`.
    pub fn light(mut self, change: impl Fn(&mut ThemeResources) + 'static) -> ThemeScope {
        self.light = Some(Rc::new(change));
        self
    }

    /// `dark` changes the resources in the dark theme, after `any`.
    pub fn dark(mut self, change: impl Fn(&mut ThemeResources) + 'static) -> ThemeScope {
        self.dark = Some(Rc::new(change));
        self
    }

    /// `base` is the resources the scope starts from.
    fn base(&self, app: &mut App, context: BuildContext) -> Rc<ThemeResources> {
        match &self.base {
            Base::Theme(theme, accent) if *accent == AccentPalette::default() => {
                ThemeResources::default_for(*theme)
            }
            Base::Theme(theme, accent) => Rc::new(ThemeResources::new(*theme, *accent)),
            Base::Enclosing => ThemeResources::of(app, context),
            Base::Resources(resources) => resources.clone(),
        }
    }

    /// `changed` is `resources` with the scope's own values: `any`, then the one for their theme.
    fn changed(&self, resources: Rc<ThemeResources>) -> Rc<ThemeResources> {
        let for_theme = match resources.theme {
            Theme::Light => &self.light,
            Theme::Dark => &self.dark,
        };
        let changes: Vec<&Override> = self.any.iter().chain(for_theme).collect();
        if changes.is_empty() {
            return resources;
        }
        let mut resources = Rc::unwrap_or_clone(resources);
        for change in changes {
            change(&mut resources);
        }
        Rc::new(resources)
    }
}

impl fmt::Debug for ThemeScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ThemeScope").finish_non_exhaustive()
    }
}

impl StatelessWidget for ThemeScope {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = self.changed(self.base(app, context));
        InheritedResources {
            resources,
            child: self.child.clone(),
        }
        .into_widget()
    }
}

/// The resources a [`ThemeScope`] resolved, for [`ThemeResources::of`] below it.
#[derive(Debug)]
struct InheritedResources {
    resources: Rc<ThemeResources>,
    child: WidgetRef,
}

impl InheritedWidget for InheritedResources {
    fn child(&self) -> &WidgetRef {
        &self.child
    }
    fn update_should_notify(&self, old: &Self) -> bool {
        !Rc::ptr_eq(&self.resources, &old.resources) && self.resources != old.resources
    }
}
