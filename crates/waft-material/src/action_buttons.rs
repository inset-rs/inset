//! Flutter counterpart: material_ui's `action_buttons.dart`.

use waft_embedder::{Color, TargetPlatform};
use waft_foundation::{App, K_IS_WEB, Listener};
use waft_widgets::{
    BuildContext, Icon, IconData, IntoWidget, KeyRef, Navigator, Semantics, StandardComponentType,
    StatelessWidget, WidgetBuilder, WidgetRef,
};

use crate::action_icons_theme::{ActionIconTheme, ActionIconThemeData};
use crate::button_style::ButtonStyle;
use crate::debug::debug_check_has_material_localizations;
use crate::icon_button::IconButton;
use crate::icons::Icons;
use crate::material_localizations::MaterialLocalizations;
use crate::scaffold::{Scaffold, scaffold_drawer_expanded_of, scaffold_end_drawer_expanded_of};
use crate::theme::Theme;

/// What `_ActionButton` reads of its subclass: the fields Dart's constructor passes to
/// `IconButton` through `super`.
struct ActionButtonFields<'a> {
    color: Option<Color>,
    style: &'a Option<ButtonStyle>,
    on_pressed: &'a Option<Listener>,
    icon: &'a WidgetRef,
    standard_component: Option<StandardComponentType>,
}

/// Dart's abstract `_ActionButton extends IconButton`: a Material Design icon button whose
/// subclasses name the tooltip and the default behaviour of a press.
///
/// A subclass is a widget that builds through [`ActionButton::build`]; the base class's
/// `build` never returns the subclass itself but a plain `IconButton`.
trait ActionButton: Sized {
    fn fields(&self) -> ActionButtonFields<'_>;

    /// This returns the appropriate tooltip text for this action button.
    fn get_tooltip(app: &mut App, context: BuildContext) -> String;

    /// This is the default function that is called when `on_pressed` is set
    /// to null.
    fn on_pressed_callback(app: &mut App, context: BuildContext);

    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material_localizations(app, context));
        let fields = self.fields();
        let on_pressed = fields.on_pressed.clone();
        let mut button = IconButton::new(
            Some(Listener::new(move |app| match &on_pressed {
                Some(on_pressed) => on_pressed.call(app),
                None => Self::on_pressed_callback(app, context),
            })),
            fields.icon.clone(),
        )
        .tooltip(Self::get_tooltip(app, context));
        if let Some(standard_component) = fields.standard_component {
            button = button.key(standard_component.key());
        }
        if let Some(style) = fields.style {
            button = button.style(style.clone());
        }
        if let Some(color) = fields.color {
            button = button.color(color);
        }
        button.into_widget()
    }
}

/// Dart's `_ActionIcon`.
#[derive(Debug)]
struct ActionIcon {
    icon_builder_callback: fn(&ActionIconThemeData) -> &Option<WidgetBuilder>,
    get_icon: fn(&mut App, BuildContext) -> IconData,
    get_android_semantics_label: fn(&dyn MaterialLocalizations) -> String,
}

impl StatelessWidget for ActionIcon {
    fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
        let action_icon_theme = ActionIconTheme::of(app, context);
        let icon_builder = action_icon_theme
            .as_ref()
            .and_then(|theme| (self.icon_builder_callback)(theme).clone());
        if let Some(icon_builder) = icon_builder {
            return icon_builder(app, context);
        }

        let data = (self.get_icon)(app, context);
        // This can't use the platform from Theme because it is the Android OS that
        // expects the duplicated tooltip and label.
        let semantics_label = match app.platform().target_platform() {
            TargetPlatform::Android => Some((self.get_android_semantics_label)(
                &*<dyn MaterialLocalizations>::of(app, context),
            )),
            TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows
            | TargetPlatform::IOS
            | TargetPlatform::MacOS => None,
        };

        let mut icon = Icon::new(Some(data));
        if let Some(semantics_label) = semantics_label {
            icon = icon.semantic_label(semantics_label);
        }
        icon.into_widget()
    }
}

/// Defines one of Dart's `_ActionButton` subclasses: a widget with `IconButton`'s `key`,
/// `color`, `style` and `onPressed`, and the icon its constructor passes to `super`.
macro_rules! action_button_widget {
    (
        $(#[$meta:meta])*
        $name:ident, icon: $icon:ident, standard_component: $standard_component:expr,
        tooltip: $tooltip:ident, on_pressed: |$app:ident, $context:ident| $on_pressed:expr
    ) => {
        $(#[$meta])*
        #[derive(Debug)]
        pub struct $name {
            pub key: Option<KeyRef>,
            pub color: Option<Color>,
            pub style: Option<ButtonStyle>,
            pub on_pressed: Option<Listener>,
            icon: WidgetRef,
            standard_component: Option<StandardComponentType>,
        }

        impl $name {
            #[doc = concat!("Dart `", stringify!($name), "()`.")]
            pub fn new() -> $name {
                $name {
                    key: None,
                    color: None,
                    style: None,
                    on_pressed: None,
                    icon: $icon::new().into_widget(),
                    standard_component: $standard_component,
                }
            }

            #[doc = concat!("Dart `", stringify!($name), "(key:)`.")]
            pub fn key(mut self, key: KeyRef) -> $name {
                self.key = Some(key);
                self
            }

            #[doc = concat!("Dart `", stringify!($name), "(color:)`.")]
            pub fn color(mut self, color: Color) -> $name {
                self.color = Some(color);
                self
            }

            #[doc = concat!("Dart `", stringify!($name), "(style:)`.")]
            pub fn style(mut self, style: ButtonStyle) -> $name {
                self.style = Some(style);
                self
            }

            #[doc = concat!("Dart `", stringify!($name), "(onPressed:)`.")]
            pub fn on_pressed(mut self, on_pressed: Listener) -> $name {
                self.on_pressed = Some(on_pressed);
                self
            }
        }

        impl Default for $name {
            fn default() -> $name {
                $name::new()
            }
        }

        impl ActionButton for $name {
            fn fields(&self) -> ActionButtonFields<'_> {
                ActionButtonFields {
                    color: self.color,
                    style: &self.style,
                    on_pressed: &self.on_pressed,
                    icon: &self.icon,
                    standard_component: self.standard_component,
                }
            }

            fn get_tooltip(app: &mut App, context: BuildContext) -> String {
                <dyn MaterialLocalizations>::of(app, context).$tooltip()
            }

            fn on_pressed_callback($app: &mut App, $context: BuildContext) {
                $on_pressed
            }
        }

        impl StatelessWidget for $name {
            fn key(&self) -> Option<&KeyRef> {
                self.key.as_ref()
            }

            fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
                ActionButton::build(self, app, context)
            }
        }
    };
}

/// Dart's `Semantics(expanded:)` around `DrawerButtonIcon`'s and `EndDrawerButtonIcon`'s
/// `_ActionIcon`, from the scaffold read `expanded_of`.
fn with_expanded_semantics(
    app: &mut App,
    context: BuildContext,
    expanded_of: fn(&mut App, BuildContext) -> Option<bool>,
    action_icon: ActionIcon,
) -> WidgetRef {
    let mut semantics = Semantics::new();
    if let Some(expanded) = expanded_of(app, context) {
        semantics = semantics.expanded(expanded);
    }
    semantics.child(action_icon).into_widget()
}

/// Defines one of Dart's `*ButtonIcon` widgets, each a `_ActionIcon` whose builder comes from
/// the action icon theme, and whose Android semantics label is the localized `$label`. The
/// drawer icons name `expanded:`, the scaffold read Dart's `Semantics(expanded:)` around them
/// takes.
macro_rules! action_icon_widget {
    (@wrap $app:ident, $context:ident, $action_icon:ident) => {
        $action_icon.into_widget()
    };
    (@wrap $app:ident, $context:ident, $action_icon:ident, $expanded:path) => {
        with_expanded_semantics($app, $context, $expanded, $action_icon)
    };
    (
        $(#[$meta:meta])*
        $name:ident, builder: $builder:ident, get_icon: |$app:ident, $context:ident| $get_icon:expr,
        android_semantics_label: $label:ident $(, expanded: $expanded:path)?
    ) => {
        $(#[$meta])*
        #[derive(Debug, Default)]
        pub struct $name {
            pub key: Option<KeyRef>,
        }

        impl $name {
            #[doc = concat!("Dart `", stringify!($name), "()`.")]
            pub fn new() -> $name {
                $name::default()
            }

            #[doc = concat!("Dart `", stringify!($name), "(key:)`.")]
            pub fn key(mut self, key: KeyRef) -> $name {
                self.key = Some(key);
                self
            }
        }

        impl StatelessWidget for $name {
            fn key(&self) -> Option<&KeyRef> {
                self.key.as_ref()
            }

            fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
                let action_icon = ActionIcon {
                    icon_builder_callback: |action_icon_theme| &action_icon_theme.$builder,
                    get_icon: |$app, $context| $get_icon,
                    get_android_semantics_label: |material_localization| {
                        material_localization.$label()
                    },
                };
                action_icon_widget!(@wrap _app, _context, action_icon $(, $expanded)?)
            }
        }
    };
}

action_icon_widget!(
    /// A "back" icon that's appropriate for the current [`TargetPlatform`].
    ///
    /// The current platform is determined by querying for the ambient `Theme`.
    ///
    /// See also:
    ///
    ///  * [`BackButton`], an `IconButton` with a [`BackButtonIcon`] that calls
    ///    `Navigator::maybe_pop` to return to the previous route.
    ///  * `IconButton`, which is a more general widget for creating buttons
    ///    with icons.
    ///  * `Icon`, a Material Design icon.
    ///  * `ThemeData::platform`, which specifies the current platform.
    BackButtonIcon,
    builder: back_button_icon_builder,
    get_icon: |app, context| {
        if K_IS_WEB {
            // Always use 'Icons.arrow_back' as a back_button icon in web.
            return Icons::arrow_back();
        }
        match Theme::of(app, context).platform {
            TargetPlatform::Android
            | TargetPlatform::Fuchsia
            | TargetPlatform::Linux
            | TargetPlatform::Windows => Icons::arrow_back(),
            TargetPlatform::IOS | TargetPlatform::MacOS => Icons::arrow_back_ios_new_rounded(),
        }
    },
    android_semantics_label: back_button_tooltip
);

action_button_widget!(
    /// A Material Design back icon button.
    ///
    /// A [`BackButton`] is an `IconButton` with a "back" icon appropriate for the
    /// current [`TargetPlatform`]. When pressed, the back button calls
    /// `Navigator::maybe_pop` to return to the previous route unless a custom
    /// [`on_pressed`](Self::on_pressed) callback is provided.
    ///
    /// The [`on_pressed`](Self::on_pressed) callback can, for instance, be used to pop the
    /// platform's navigation stack via `SystemNavigator` instead of Flutter's `Navigator` in
    /// add-to-app situations.
    ///
    /// In Material Design 3, both [`style`](Self::style)'s `ButtonStyle::icon_color` and
    /// [`color`](Self::color) are used to override the default icon color of [`BackButton`]. If
    /// both exist, the `ButtonStyle::icon_color` will override [`color`](Self::color) for states
    /// where `ButtonStyle::foreground_color` resolves to non-null.
    ///
    /// When deciding to display a [`BackButton`], consider using
    /// `ModalRoute::can_pop_of(context)` to check whether the current route can be
    /// popped. If that value is false (e.g., because the current route is the
    /// initial route), the [`BackButton`] will not have any effect when pressed,
    /// which could frustrate the user.
    ///
    /// Requires one of its ancestors to be a `Material` widget.
    ///
    /// See also:
    ///
    ///  * `AppBar`, which automatically uses a [`BackButton`] in its
    ///    `AppBar::leading` slot when the `Scaffold` has no `Drawer` and the
    ///    current `Route` is not the `Navigator`'s first route.
    ///  * [`BackButtonIcon`], which is useful if you need to create a back button
    ///    that responds differently to being pressed.
    ///  * `IconButton`, which is a more general widget for creating buttons with
    ///    icons.
    ///  * [`CloseButton`], an alternative which may be more appropriate for leaf
    ///    node pages in the navigation tree.
    BackButton,
    icon: BackButtonIcon,
    standard_component: Some(StandardComponentType::BackButton),
    tooltip: back_button_tooltip,
    on_pressed: |app, context| {
        Navigator::maybe_pop(app, context, None);
    }
);

action_icon_widget!(
    /// A "close" icon that's appropriate for the current [`TargetPlatform`].
    ///
    /// The current platform is determined by querying for the ambient `Theme`.
    ///
    /// See also:
    ///
    ///  * [`CloseButton`], an `IconButton` with a [`CloseButtonIcon`] that calls
    ///    `Navigator::maybe_pop` to return to the previous route.
    ///  * `IconButton`, which is a more general widget for creating buttons
    ///    with icons.
    ///  * `Icon`, a Material Design icon.
    ///  * `ThemeData::platform`, which specifies the current platform.
    CloseButtonIcon,
    builder: close_button_icon_builder,
    get_icon: |_app, _context| Icons::close(),
    android_semantics_label: close_button_tooltip
);

action_button_widget!(
    /// A Material Design close icon button.
    ///
    /// A [`CloseButton`] is an `IconButton` with a "close" icon. When pressed, the
    /// close button calls `Navigator::maybe_pop` to return to the previous route.
    ///
    /// The [`on_pressed`](Self::on_pressed) callback can, for instance, be used to pop the
    /// platform's navigation stack via `SystemNavigator` instead of Flutter's `Navigator` in
    /// add-to-app situations.
    ///
    /// In Material Design 3, both [`style`](Self::style)'s `ButtonStyle::icon_color` and
    /// [`color`](Self::color) are used to override the default icon color of [`CloseButton`]. If
    /// both exist, the `ButtonStyle::icon_color` will override [`color`](Self::color) for states
    /// where `ButtonStyle::foreground_color` resolves to non-null.
    ///
    /// Use a [`CloseButton`] instead of a [`BackButton`] on fullscreen dialogs or
    /// pages that may solicit additional actions to close.
    ///
    /// See also:
    ///
    ///  * `AppBar`, which automatically uses a [`CloseButton`] in its
    ///    `AppBar::leading` slot when appropriate.
    ///  * [`BackButton`], which is more appropriate for middle nodes in the
    ///    navigation tree or where pages can be popped instantaneously with
    ///    no user data consequence.
    ///  * `IconButton`, to create other Material Design icon buttons.
    CloseButton,
    icon: CloseButtonIcon,
    standard_component: Some(StandardComponentType::CloseButton),
    tooltip: close_button_tooltip,
    on_pressed: |app, context| {
        Navigator::maybe_pop(app, context, None);
    }
);

action_icon_widget!(
    /// A "drawer" icon that's appropriate for the current [`TargetPlatform`].
    ///
    /// The current platform is determined by querying for the ambient `Theme`.
    ///
    /// See also:
    ///
    ///  * [`DrawerButton`], an `IconButton` with a [`DrawerButtonIcon`] that calls
    ///    `ScaffoldState::open_drawer` to open the `Scaffold::drawer`.
    ///  * [`EndDrawerButton`], an `IconButton` with an [`EndDrawerButtonIcon`] that
    ///    calls `ScaffoldState::open_end_drawer` to open the `Scaffold::end_drawer`.
    ///  * `IconButton`, which is a more general widget for creating buttons
    ///    with icons.
    ///  * `Icon`, a Material Design icon.
    ///  * `ThemeData::platform`, which specifies the current platform.
    DrawerButtonIcon,
    builder: drawer_button_icon_builder,
    get_icon: |_app, _context| Icons::menu(),
    android_semantics_label: open_app_drawer_tooltip,
    expanded: scaffold_drawer_expanded_of
);

action_button_widget!(
    /// A Material Design drawer icon button.
    ///
    /// A [`DrawerButton`] is an `IconButton` with a "drawer" icon. When pressed, the
    /// close button calls `ScaffoldState::open_drawer` to the `Scaffold::drawer`.
    ///
    /// The default behaviour on press can be overridden with
    /// [`on_pressed`](Self::on_pressed).
    ///
    /// See also:
    ///
    ///  * [`EndDrawerButton`], an `IconButton` with an [`EndDrawerButtonIcon`] that
    ///    calls `ScaffoldState::open_end_drawer` to open the `Scaffold::end_drawer`.
    ///  * `IconButton`, which is a more general widget for creating buttons
    ///    with icons.
    ///  * `Icon`, a Material Design icon.
    ///  * `ThemeData::platform`, which specifies the current platform.
    DrawerButton,
    icon: DrawerButtonIcon,
    standard_component: Some(StandardComponentType::DrawerButton),
    tooltip: open_app_drawer_tooltip,
    on_pressed: |app, context| {
        Scaffold::of(app, context).open_drawer(app);
    }
);

action_icon_widget!(
    /// A "end drawer" icon that's appropriate for the current [`TargetPlatform`].
    ///
    /// The current platform is determined by querying for the ambient `Theme`.
    ///
    /// See also:
    ///
    ///  * [`DrawerButton`], an `IconButton` with a [`DrawerButtonIcon`] that calls
    ///    `ScaffoldState::open_drawer` to open the `Scaffold::drawer`.
    ///  * [`EndDrawerButton`], an `IconButton` with an [`EndDrawerButtonIcon`] that
    ///    calls `ScaffoldState::open_end_drawer` to open the `Scaffold::end_drawer`
    ///  * `IconButton`, which is a more general widget for creating buttons
    ///    with icons.
    ///  * `Icon`, a Material Design icon.
    ///  * `ThemeData::platform`, which specifies the current platform.
    EndDrawerButtonIcon,
    builder: end_drawer_button_icon_builder,
    get_icon: |_app, _context| Icons::menu(),
    android_semantics_label: open_app_drawer_tooltip,
    expanded: scaffold_end_drawer_expanded_of
);

action_button_widget!(
    /// A Material Design end drawer icon button.
    ///
    /// A [`EndDrawerButton`] is an `IconButton` with a "drawer" icon. When pressed, the
    /// end drawer button calls `ScaffoldState::open_end_drawer` to open the
    /// `Scaffold::end_drawer`.
    ///
    /// The default behaviour on press can be overridden with
    /// [`on_pressed`](Self::on_pressed).
    ///
    /// See also:
    ///
    ///  * [`DrawerButton`], an `IconButton` with a [`DrawerButtonIcon`] that calls
    ///    `ScaffoldState::open_drawer` to open a drawer.
    ///  * `IconButton`, which is a more general widget for creating buttons
    ///    with icons.
    ///  * `Icon`, a Material Design icon.
    ///  * `ThemeData::platform`, which specifies the current platform.
    EndDrawerButton,
    icon: EndDrawerButtonIcon,
    standard_component: None,
    tooltip: open_app_drawer_tooltip,
    on_pressed: |app, context| {
        Scaffold::of(app, context).open_end_drawer(app);
    }
);

#[cfg(test)]
mod tests {
    //! The cases of `back_button_test.dart` and `drawer_button_test.dart` that pin behaviour; the
    //! semantics and the semantics-tree cases wait on accessibility.

    use std::cell::Cell;
    use std::rc::Rc;

    use waft_embedder::{Offset, Size, TargetPlatform};
    use waft_foundation::Handle;
    use waft_services::{MouseCursorRef, SystemMouseCursors};
    use waft_widgets::{
        AnyElement, Builder, Center, Column, GlobalKey, PageRouteBuilder, Route, RoutePageBuilder,
        SizedBox, Text, WidgetStatePropertyAll,
    };

    use super::*;
    use crate::colors::Colors;
    use crate::drawer::Drawer;
    use crate::material::Material;
    use crate::scaffold::{Scaffold, ScaffoldState};
    use crate::test_support::{WidgetTester, material_app, with_main_thread_stack};
    use crate::theme::Theme;
    use crate::theme_data::ThemeData;

    fn tester() -> WidgetTester {
        let tester = WidgetTester::on(TargetPlatform::Android);
        tester.install_fonts();
        tester
    }

    fn pump_app(tester: &mut WidgetTester, home: WidgetRef) {
        let root = {
            let app = tester.cell.borrow();
            material_app(ThemeData::new(&app), home)
        };
        tester.pump_widget(root);
    }

    fn app_scaffold(tester: &mut WidgetTester, scaffold: Scaffold) {
        pump_app(tester, scaffold.into_widget());
    }

    fn material<K>(child: impl IntoWidget<K>) -> WidgetRef {
        Material::new().child(child).into_widget()
    }

    fn tap(tester: &mut WidgetTester, element: AnyElement) {
        let render = tester.render_box_of(element);
        let center = tester.global_rect(render).center();
        tester.tap(center, 1);
    }

    fn only<W: 'static>(tester: &WidgetTester) -> AnyElement {
        let found = tester.find_by_type::<W>();
        assert_eq!(found.len(), 1);
        found[0]
    }

    fn counter(count: &Rc<Cell<usize>>) -> Listener {
        let count = Rc::clone(count);
        Listener::new(move |_app| count.set(count.get() + 1))
    }

    fn red_icon_color() -> ButtonStyle {
        ButtonStyle::new().icon_color(Rc::new(WidgetStatePropertyAll::new(Some(
            Colors::RED.primary(),
        ))))
    }

    /// The colour of the glyph the button's `Icon` paints: Dart's `tester.firstWidget<RichText>(
    /// find.descendant(of: find.byType(Button), matching: find.byType(RichText))).text.style!.color`.
    fn glyph_color(tester: &WidgetTester) -> Option<Color> {
        let icon = tester.find_by_type::<Icon>()[0];
        tester
            .glyph_style(icon)
            .color
            .as_ref()
            .map(|color| color.color())
    }

    /// The context of the home page, to push a route from.
    #[derive(Clone, Default)]
    struct Home(Rc<Cell<Option<BuildContext>>>);

    impl Home {
        fn page(&self) -> WidgetRef {
            let slot = Rc::clone(&self.0);
            Builder::new(move |_app, context| {
                slot.set(Some(context));
                material(Text::new("Home"))
            })
            .into_widget()
        }

        /// Dart's `navigator.pushNamed('/next')` of a route that builds `next`.
        fn push(&self, tester: &WidgetTester, next: impl Fn() -> WidgetRef + 'static) {
            let mut app = tester.cell.borrow_mut();
            let page_builder: RoutePageBuilder = Rc::new(move |_app, _context, _, _| next());
            let route = PageRouteBuilder::new(&mut app, page_builder);
            let context = self.0.get().expect("the home page built");
            drop(Navigator::push(&mut app, context, Route::as_route(route)));
        }
    }

    /// The tester, on the page `next` builds above the home page.
    fn on_next_page(next: impl Fn() -> WidgetRef + 'static) -> (WidgetTester, Home) {
        let mut tester = tester();
        let home = Home::default();
        pump_app(&mut tester, home.page());
        home.push(&tester, next);
        tester.pump_and_settle();
        (tester, home)
    }

    fn on_home(tester: &WidgetTester) -> bool {
        !tester.find_onstage_text("Home").is_empty()
    }

    // 'BackButton control test'
    #[test]
    fn a_back_button_pops_the_route() {
        with_main_thread_stack(|| {
            let (mut tester, _home) =
                on_next_page(|| material(Center::new().child(BackButton::new())));

            let button = only::<BackButton>(&tester);
            tap(&mut tester, button);
            tester.pump_and_settle();

            assert_eq!(tester.find_onstage_text("Home").len(), 1);
        });
    }

    // 'BackButton onPressed overrides default pop behavior'
    #[test]
    fn a_back_buttons_on_pressed_overrides_the_pop() {
        with_main_thread_stack(|| {
            let calls = Rc::new(Cell::new(0));
            let (mut tester, _home) = on_next_page({
                let calls = Rc::clone(&calls);
                move || material(Center::new().child(BackButton::new().on_pressed(counter(&calls))))
            });

            assert!(!on_home(&tester)); // Start off on the second page.
            assert_eq!(calls.get(), 0);
            let button = only::<BackButton>(&tester);
            tap(&mut tester, button);
            tester.pump_and_settle();

            // We're still on the second page.
            assert!(!on_home(&tester));
            // But the custom callback is called.
            assert_eq!(calls.get(), 1);
        });
    }

    // 'CloseButton onPressed overrides default pop behavior'
    #[test]
    fn a_close_buttons_on_pressed_overrides_the_pop() {
        with_main_thread_stack(|| {
            let calls = Rc::new(Cell::new(0));
            let (mut tester, _home) = on_next_page({
                let calls = Rc::clone(&calls);
                move || {
                    material(Center::new().child(CloseButton::new().on_pressed(counter(&calls))))
                }
            });

            assert!(!on_home(&tester));
            assert_eq!(calls.get(), 0);
            let button = only::<CloseButton>(&tester);
            tap(&mut tester, button);
            tester.pump_and_settle();

            assert!(!on_home(&tester));
            assert_eq!(calls.get(), 1);
        });
    }

    // The default pop of 'BackButton control test', for the close button.
    #[test]
    fn a_close_button_pops_the_route() {
        with_main_thread_stack(|| {
            let (mut tester, _home) =
                on_next_page(|| material(Center::new().child(CloseButton::new())));

            let button = only::<CloseButton>(&tester);
            tap(&mut tester, button);
            tester.pump_and_settle();

            assert_eq!(tester.find_onstage_text("Home").len(), 1);
        });
    }

    /// The icons of the `Icon`s the tester found, in tree order.
    fn icons(tester: &WidgetTester) -> Vec<Option<IconData>> {
        tester
            .find_by_type::<Icon>()
            .into_iter()
            .map(|element| tester.widget(element, |icon: &Icon| icon.icon.clone()))
            .collect()
    }

    /// Dart's `Theme(data: ThemeData(platform: platform), child: icon)` for the five platforms,
    /// in Dart's order: Android, iOS, Linux, macOS, Windows.
    fn icon_on_each_platform(tester: &mut WidgetTester, icon: impl Fn() -> WidgetRef) {
        let platforms = [
            TargetPlatform::Android,
            TargetPlatform::IOS,
            TargetPlatform::Linux,
            TargetPlatform::MacOS,
            TargetPlatform::Windows,
        ];
        let children: Vec<WidgetRef> = {
            let app = tester.cell.borrow();
            platforms
                .into_iter()
                .map(|platform| {
                    Theme::new(ThemeData::builder().platform(platform).build(&app), icon())
                        .into_widget()
                })
                .collect()
        };
        pump_app(tester, Column::new().children(children).into_widget());
    }

    // 'BackButton icon'
    #[test]
    fn a_back_button_icon_is_the_ios_arrow_on_ios_and_macos() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            icon_on_each_platform(&mut tester, || BackButtonIcon::new().into_widget());

            let [android, ios, linux, macos, windows] = &icons(&tester)[..] else {
                panic!("one icon for each platform");
            };
            assert_eq!(ios == android, K_IS_WEB);
            assert!(linux == android);
            assert_eq!(macos == android, K_IS_WEB);
            assert!(macos == ios);
            assert!(windows == android);
        });
    }

    // 'DrawerButton icon'
    #[test]
    fn a_drawer_button_icon_is_the_same_on_every_platform() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            icon_on_each_platform(&mut tester, || DrawerButtonIcon::new().into_widget());

            let icons = icons(&tester);
            assert!(icons.iter().all(|icon| *icon == icons[0]));
            assert_eq!(icons[0], Some(Icons::menu()));
        });
    }

    // The platform choice of `CloseButtonIcon` and `EndDrawerButtonIcon`.
    #[test]
    fn a_close_icon_and_an_end_drawer_icon_are_the_same_on_every_platform() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            icon_on_each_platform(&mut tester, || CloseButtonIcon::new().into_widget());
            let close_icons = icons(&tester);
            assert!(close_icons.iter().all(|icon| *icon == Some(Icons::close())));

            icon_on_each_platform(&mut tester, || EndDrawerButtonIcon::new().into_widget());
            let end_drawer_icons = icons(&tester);
            assert!(
                end_drawer_icons
                    .iter()
                    .all(|icon| *icon == Some(Icons::menu()))
            );
        });
    }

    // The tooltip each button gives its `IconButton`, which Dart's semantics cases read as
    // 'Back', 'Close' and 'Open navigation menu'.
    #[test]
    fn each_action_button_gives_its_icon_button_the_localized_tooltip() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let buttons: Vec<WidgetRef> = vec![
                BackButton::new().into_widget(),
                CloseButton::new().into_widget(),
                DrawerButton::new().into_widget(),
                EndDrawerButton::new().into_widget(),
            ];
            pump_app(&mut tester, material(Column::new().children(buttons)));

            let tooltips: Vec<Option<String>> = tester
                .find_by_type::<IconButton>()
                .into_iter()
                .map(|element| tester.widget(element, |button: &IconButton| button.tooltip.clone()))
                .collect();
            assert_eq!(
                tooltips,
                vec![
                    Some("Back".to_string()),
                    Some("Close".to_string()),
                    Some("Open navigation menu".to_string()),
                    Some("Open navigation menu".to_string()),
                ]
            );
        });
    }

    // The keys that mark a standard component, which each button gives its `IconButton`.
    #[test]
    fn action_buttons_mark_their_icon_button_as_a_standard_component() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let buttons: Vec<WidgetRef> = vec![
                BackButton::new().into_widget(),
                CloseButton::new().into_widget(),
                DrawerButton::new().into_widget(),
                EndDrawerButton::new().into_widget(),
            ];
            pump_app(&mut tester, material(Column::new().children(buttons)));

            let keys: Vec<Option<KeyRef>> = tester
                .find_by_type::<IconButton>()
                .into_iter()
                .map(|element| tester.widget(element, |button: &IconButton| button.key.clone()))
                .collect();
            assert_eq!(
                keys,
                vec![
                    Some(StandardComponentType::BackButton.key()),
                    Some(StandardComponentType::CloseButton.key()),
                    Some(StandardComponentType::DrawerButton.key()),
                    None,
                ]
            );
        });
    }

    /// Dart's `MaterialApp(home: Material(child: button))` with the glyph colour read back.
    fn glyph_color_of<K>(button: impl IntoWidget<K>) -> Option<Color> {
        let mut tester = tester();
        pump_app(&mut tester, material(button));
        glyph_color(&tester)
    }

    // 'BackButton color'
    #[test]
    fn a_back_buttons_color_colors_its_icon() {
        with_main_thread_stack(|| {
            assert_eq!(
                glyph_color_of(BackButton::new().color(Colors::RED.primary())),
                Some(Colors::RED.primary())
            );
        });
    }

    // 'BackButton color with ButtonStyle'
    #[test]
    fn a_back_buttons_style_icon_color_colors_its_icon() {
        with_main_thread_stack(|| {
            assert_eq!(
                glyph_color_of(BackButton::new().style(red_icon_color())),
                Some(Colors::RED.primary())
            );
        });
    }

    // 'BackButton.style.iconColor parameter overrides BackButton.color'
    #[test]
    fn a_back_buttons_style_icon_color_overrides_its_color() {
        with_main_thread_stack(|| {
            assert_eq!(
                glyph_color_of(
                    BackButton::new()
                        .color(Colors::GREEN.primary())
                        .style(red_icon_color())
                ),
                Some(Colors::RED.primary())
            );
        });
    }

    // 'CloseButton color'
    #[test]
    fn a_close_buttons_color_colors_its_icon() {
        with_main_thread_stack(|| {
            assert_eq!(
                glyph_color_of(CloseButton::new().color(Colors::RED.primary())),
                Some(Colors::RED.primary())
            );
        });
    }

    // 'CloseButton color with ButtonStyle'
    #[test]
    fn a_close_buttons_style_icon_color_colors_its_icon() {
        with_main_thread_stack(|| {
            assert_eq!(
                glyph_color_of(CloseButton::new().style(red_icon_color())),
                Some(Colors::RED.primary())
            );
        });
    }

    // 'CloseButton.style.iconColor parameter overrides CloseButton.color'
    #[test]
    fn a_close_buttons_style_icon_color_overrides_its_color() {
        with_main_thread_stack(|| {
            assert_eq!(
                glyph_color_of(
                    CloseButton::new()
                        .color(Colors::GREEN.primary())
                        .style(red_icon_color())
                ),
                Some(Colors::RED.primary())
            );
        });
    }

    // 'DrawerButton color'
    #[test]
    fn a_drawer_buttons_color_colors_its_icon() {
        with_main_thread_stack(|| {
            assert_eq!(
                glyph_color_of(DrawerButton::new().color(Colors::RED.primary())),
                Some(Colors::RED.primary())
            );
        });
    }

    // 'DrawerButton color with ButtonStyle'
    #[test]
    fn a_drawer_buttons_style_icon_color_colors_its_icon() {
        with_main_thread_stack(|| {
            assert_eq!(
                glyph_color_of(DrawerButton::new().style(red_icon_color())),
                Some(Colors::RED.primary())
            );
        });
    }

    // 'EndDrawerButton color'
    #[test]
    fn an_end_drawer_buttons_color_colors_its_icon() {
        with_main_thread_stack(|| {
            assert_eq!(
                glyph_color_of(EndDrawerButton::new().color(Colors::RED.primary())),
                Some(Colors::RED.primary())
            );
        });
    }

    // 'EndDrawerButton color with ButtonStyle'
    #[test]
    fn an_end_drawer_buttons_style_icon_color_colors_its_icon() {
        with_main_thread_stack(|| {
            assert_eq!(
                glyph_color_of(EndDrawerButton::new().style(red_icon_color())),
                Some(Colors::RED.primary())
            );
        });
    }

    // 'DrawerButton control test'
    #[test]
    fn a_drawer_button_opens_the_drawer() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            app_scaffold(
                &mut tester,
                Scaffold::new()
                    .body(DrawerButton::new())
                    .drawer(Drawer::new()),
            );
            tester.pump_and_settle();

            assert!(tester.find_by_type::<Drawer>().is_empty());

            let button = only::<DrawerButton>(&tester);
            tap(&mut tester, button);
            tester.pump_and_settle();

            assert_eq!(tester.find_by_type::<Drawer>().len(), 1);
        });
    }

    // 'EndDrawerButton control test'
    #[test]
    fn an_end_drawer_button_opens_the_end_drawer() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            app_scaffold(
                &mut tester,
                Scaffold::new()
                    .body(EndDrawerButton::new())
                    .end_drawer(Drawer::new()),
            );
            tester.pump_and_settle();

            assert!(tester.find_by_type::<Drawer>().is_empty());

            let button = only::<EndDrawerButton>(&tester);
            tap(&mut tester, button);
            tester.pump_and_settle();

            assert_eq!(tester.find_by_type::<Drawer>().len(), 1);
        });
    }

    // 'DrawerButton onPressed overrides default end drawer open behaviour'
    #[test]
    fn a_drawer_buttons_on_pressed_overrides_opening_the_drawer() {
        with_main_thread_stack(|| {
            let calls = Rc::new(Cell::new(0));
            let mut tester = tester();
            app_scaffold(
                &mut tester,
                Scaffold::new()
                    .body(Center::new().child(DrawerButton::new().on_pressed(counter(&calls))))
                    .drawer(Drawer::new()),
            );
            tester.pump_and_settle();

            assert!(tester.find_by_type::<Drawer>().is_empty());
            assert_eq!(calls.get(), 0);
            let button = only::<DrawerButton>(&tester);
            tap(&mut tester, button);
            tester.pump_and_settle();

            // Drawer is still closed
            assert!(tester.find_by_type::<Drawer>().is_empty());
            // The custom callback is called.
            assert_eq!(calls.get(), 1);
        });
    }

    // 'EndDrawerButton onPressed overrides default end drawer open behaviour'
    #[test]
    fn an_end_drawer_buttons_on_pressed_overrides_opening_the_end_drawer() {
        with_main_thread_stack(|| {
            let calls = Rc::new(Cell::new(0));
            let mut tester = tester();
            app_scaffold(
                &mut tester,
                Scaffold::new()
                    .body(Center::new().child(EndDrawerButton::new().on_pressed(counter(&calls))))
                    .end_drawer(Drawer::new()),
            );
            tester.pump_and_settle();

            assert!(tester.find_by_type::<Drawer>().is_empty());
            assert_eq!(calls.get(), 0);
            let button = only::<EndDrawerButton>(&tester);
            tap(&mut tester, button);
            tester.pump_and_settle();

            assert!(tester.find_by_type::<Drawer>().is_empty());
            assert_eq!(calls.get(), 1);
        });
    }

    /// A scaffold with both drawers and both drawer buttons in its body.
    fn scaffold_with_both_drawers(
        tester: &mut WidgetTester,
        buttons: Vec<WidgetRef>,
    ) -> Handle<ScaffoldState> {
        let key = GlobalKey::new();
        app_scaffold(
            tester,
            Scaffold::new()
                .key(key.clone().into())
                .body(Center::new().child(Column::new().children(buttons)))
                .drawer(Drawer::new())
                .end_drawer(Drawer::new()),
        );
        tester.pump_and_settle();
        key.current_state::<ScaffoldState>(&tester.cell.borrow_mut())
            .expect("a mounted scaffold")
    }

    // The `openDrawer` each drawer button calls, against the scaffold's state.
    #[test]
    fn a_drawer_button_opens_only_the_drawer() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let state = scaffold_with_both_drawers(
                &mut tester,
                vec![
                    EndDrawerButton::new().into_widget(),
                    DrawerButton::new().into_widget(),
                ],
            );

            let button = only::<DrawerButton>(&tester);
            tap(&mut tester, button);
            tester.pump_and_settle();

            assert!(state.is_drawer_open(&tester.cell.borrow()));
            assert!(!state.is_end_drawer_open(&tester.cell.borrow()));
        });
    }

    // The `openEndDrawer` each drawer button calls, against the scaffold's state.
    #[test]
    fn an_end_drawer_button_opens_only_the_end_drawer() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let state = scaffold_with_both_drawers(
                &mut tester,
                vec![
                    DrawerButton::new().into_widget(),
                    EndDrawerButton::new().into_widget(),
                ],
            );

            let button = only::<EndDrawerButton>(&tester);
            tap(&mut tester, button);
            tester.pump_and_settle();

            assert!(!state.is_drawer_open(&tester.cell.borrow()));
            assert!(state.is_end_drawer_open(&tester.cell.borrow()));
        });
    }

    /// Dart's `MaterialApp(home: Center(child: SizedBox.shrink(child: Scaffold(body: button))))`.
    fn size_at_zero_area<W: 'static, K>(button: impl IntoWidget<K>) -> Size {
        let mut tester = tester();
        pump_app(
            &mut tester,
            Center::new()
                .child(SizedBox::shrink().child(Scaffold::new().body(button)))
                .into_widget(),
        );
        let element = only::<W>(&tester);
        let render = tester.render_box_of(element);
        render.size(&tester.cell.borrow())
    }

    // 'EndDrawerButton does not crash at zero area'
    #[test]
    fn an_end_drawer_button_has_no_size_at_zero_area() {
        with_main_thread_stack(|| {
            assert_eq!(
                size_at_zero_area::<EndDrawerButton, _>(EndDrawerButton::new()),
                Size::ZERO
            );
        });
    }

    // 'CloseButton does not crash at zero area'
    #[test]
    fn a_close_button_has_no_size_at_zero_area() {
        with_main_thread_stack(|| {
            assert_eq!(
                size_at_zero_area::<CloseButton, _>(CloseButton::new()),
                Size::ZERO
            );
        });
    }

    // 'DrawerButton does not crash at zero area'
    #[test]
    fn a_drawer_button_has_no_size_at_zero_area() {
        with_main_thread_stack(|| {
            assert_eq!(
                size_at_zero_area::<DrawerButton, _>(DrawerButton::new()),
                Size::ZERO
            );
        });
    }

    // 'BackButton renders at zero area'
    #[test]
    fn a_back_button_icon_is_empty_at_zero_area() {
        with_main_thread_stack(|| {
            let _ = size_at_zero_area::<BackButton, _>(BackButton::new());
            let mut tester = tester();
            pump_app(
                &mut tester,
                Center::new()
                    .child(SizedBox::shrink().child(Scaffold::new().body(BackButton::new())))
                    .into_widget(),
            );
            let icon = only::<BackButtonIcon>(&tester);
            let render = tester.render_box_of(icon);
            let size = render.size(&tester.cell.borrow());
            assert!(size.is_empty());
        });
    }

    /// The cursor under a mouse the tester moved onto the centre of the button.
    fn cursor_over<W: 'static, K>(button: impl IntoWidget<K>) -> MouseCursorRef {
        let mut tester = tester();
        pump_app(&mut tester, material(button));
        let mut mouse = WidgetTester::mouse(1, 0);
        tester.add_pointer(&mut mouse, Offset::new(1000.0, 1000.0));
        let render = tester.render_box_of(only::<W>(&tester));
        let center = tester.global_rect(render).center();
        tester.move_to(&mut mouse, center, false);
        tester.pump(std::time::Duration::ZERO);
        tester.active_cursor(1)
    }

    fn default_cursor() -> MouseCursorRef {
        if K_IS_WEB {
            SystemMouseCursors::CLICK.into()
        } else {
            SystemMouseCursors::BASIC.into()
        }
    }

    fn cell_cursor_style() -> ButtonStyle {
        let cell: MouseCursorRef = SystemMouseCursors::CELL.into();
        ButtonStyle::new().mouse_cursor(Rc::new(WidgetStatePropertyAll::new(Some(cell))))
    }

    // 'BackButton has expected default mouse cursor on hover'
    #[test]
    fn a_back_button_has_the_default_mouse_cursor_on_hover() {
        with_main_thread_stack(|| {
            assert!(*cursor_over::<BackButton, _>(BackButton::new()) == *default_cursor());
        });
    }

    // 'CloseButton has expected default mouse cursor on hover'
    #[test]
    fn a_close_button_has_the_default_mouse_cursor_on_hover() {
        with_main_thread_stack(|| {
            assert!(*cursor_over::<CloseButton, _>(CloseButton::new()) == *default_cursor());
        });
    }

    // 'DrawerButton has expected default mouse cursor on hover'
    #[test]
    fn a_drawer_button_has_the_default_mouse_cursor_on_hover() {
        with_main_thread_stack(|| {
            assert!(*cursor_over::<DrawerButton, _>(DrawerButton::new()) == *default_cursor());
        });
    }

    // 'EndDrawerButton has expected default mouse cursor on hover'
    #[test]
    fn an_end_drawer_button_has_the_default_mouse_cursor_on_hover() {
        with_main_thread_stack(|| {
            assert!(
                *cursor_over::<EndDrawerButton, _>(EndDrawerButton::new()) == *default_cursor()
            );
        });
    }

    // 'BackButton has expected mouse cursor when explicitly configured'
    #[test]
    fn a_back_button_uses_the_mouse_cursor_its_style_gives() {
        with_main_thread_stack(|| {
            let cell: MouseCursorRef = SystemMouseCursors::CELL.into();
            assert!(
                *cursor_over::<BackButton, _>(BackButton::new().style(cell_cursor_style()))
                    == *cell
            );
        });
    }

    // 'CloseButton has expected mouse cursor when explicitly configured'
    #[test]
    fn a_close_button_uses_the_mouse_cursor_its_style_gives() {
        with_main_thread_stack(|| {
            let cell: MouseCursorRef = SystemMouseCursors::CELL.into();
            assert!(
                *cursor_over::<CloseButton, _>(CloseButton::new().style(cell_cursor_style()))
                    == *cell
            );
        });
    }

    // 'DrawerButton has expected mouse cursor when explicitly configured'
    #[test]
    fn a_drawer_button_uses_the_mouse_cursor_its_style_gives() {
        with_main_thread_stack(|| {
            let cell: MouseCursorRef = SystemMouseCursors::CELL.into();
            assert!(
                *cursor_over::<DrawerButton, _>(DrawerButton::new().style(cell_cursor_style()))
                    == *cell
            );
        });
    }

    // 'EndDrawerButton has expected mouse cursor when explicitly configured'
    #[test]
    fn an_end_drawer_button_uses_the_mouse_cursor_its_style_gives() {
        with_main_thread_stack(|| {
            let cell: MouseCursorRef = SystemMouseCursors::CELL.into();
            assert!(
                *cursor_over::<EndDrawerButton, _>(
                    EndDrawerButton::new().style(cell_cursor_style())
                ) == *cell
            );
        });
    }
}
