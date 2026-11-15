//! Flutter counterpart: material_ui's `expand_icon.dart`.

use std::f64::consts::PI;
use std::fmt;

use waft_animation::{
    Animatable, AnimationBehavior, AnimationController, AnimationRef, CurveTween, Curves, Drive,
    Tween,
};
use waft_embedder::{Brightness, Color};
use waft_foundation::{App, Handle, Listener, Ref, ValueChanged};
use waft_painting::EdgeInsetsGeometry;
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_widgets::{
    BuildContext, Icon, IntoWidget, KeyRef, RotationTransition, Semantics,
    SingleTickerProviderStateMixin, SingleTickerProviderStateMixinData, State, StateData,
    StatefulWidget, WidgetRef,
};

use crate::colors::Colors;
use crate::debug::{debug_check_has_material, debug_check_has_material_localizations};
use crate::icon_button::IconButton;
use crate::icons::Icons;
use crate::material_localizations::MaterialLocalizations;
use crate::theme::{K_THEME_ANIMATION_DURATION, Theme};

/// A widget representing a rotating expand/collapse button. The icon rotates
/// 180 degrees when pressed, then reverts the animation on a second press.
/// The underlying icon is `Icons::expand_more`.
///
/// The expand icon does not include a semantic label for accessibility. In
/// order to be accessible it should be combined with a label using
/// `MergeSemantics`. This is done automatically by the `ExpansionPanel` widget.
///
/// See [`IconButton`] for a more general implementation of a pressable button
/// with an icon.
///
/// See also:
///
///  * <https://material.io/design/iconography/system-icons.html>
pub struct ExpandIcon {
    pub key: Option<KeyRef>,

    /// Whether the icon is in an expanded state.
    ///
    /// Rebuilding the widget with a different [`is_expanded`](Self::is_expanded) value will
    /// trigger the animation, but will not trigger the [`on_pressed`](Self::on_pressed)
    /// callback.
    pub is_expanded: bool,

    /// The size of the icon.
    ///
    /// Defaults to 24.
    pub size: f64,

    /// The callback triggered when the icon is pressed and the state changes
    /// between expanded and collapsed. The value passed to the current state.
    ///
    /// If this is set to `None`, the button will be disabled.
    pub on_pressed: Option<ValueChanged<bool>>,

    /// The padding around the icon. The entire padded icon will react to input
    /// gestures.
    ///
    /// Defaults to a padding of 8 on all sides.
    pub padding: EdgeInsetsGeometry,

    /// The color of the icon.
    ///
    /// Defaults to `Colors::BLACK54` when the theme's
    /// `ThemeData::brightness` is [`Brightness::Light`] and to
    /// `Colors::WHITE60` when it is [`Brightness::Dark`]. This adheres to the
    /// Material Design specifications for [icons](https://material.io/design/iconography/system-icons.html#color)
    /// and for [dark theme](https://material.io/design/color/dark-theme.html#ui-application)
    pub color: Option<Color>,

    /// The color of the icon when it is disabled,
    /// i.e. if [`on_pressed`](Self::on_pressed) is `None`.
    ///
    /// Defaults to `Colors::BLACK38` when the theme's
    /// `ThemeData::brightness` is [`Brightness::Light`] and to
    /// `Colors::WHITE38` when it is [`Brightness::Dark`]. This adheres to the
    /// Material Design specifications for [icons](https://material.io/design/iconography/system-icons.html#color)
    /// and for [dark theme](https://material.io/design/color/dark-theme.html#ui-application)
    pub disabled_color: Option<Color>,

    /// The color of the icon when the icon is expanded.
    ///
    /// Defaults to `Colors::BLACK54` when the theme's
    /// `ThemeData::brightness` is [`Brightness::Light`] and to
    /// `Colors::WHITE` when it is [`Brightness::Dark`]. This adheres to the
    /// Material Design specifications for [icons](https://material.io/design/iconography/system-icons.html#color)
    /// and for [dark theme](https://material.io/design/color/dark-theme.html#ui-application)
    pub expanded_color: Option<Color>,

    /// Defines the splash color of the IconButton.
    ///
    /// If `ThemeData::use_material3` is true, this field will be ignored,
    /// as `IconButton::splash_color` will be ignored, and you should use
    /// [`highlight_color`](Self::highlight_color) instead.
    ///
    /// Defaults to `ThemeData::splash_color`.
    pub splash_color: Option<Color>,

    /// Defines the highlight color of the IconButton.
    ///
    /// Defaults to `ThemeData::highlight_color`.
    pub highlight_color: Option<Color>,
}

impl ExpandIcon {
    /// Creates an [`ExpandIcon`] with the given padding, and a callback that is
    /// triggered when the icon is pressed; Dart's optional named arguments are the setters.
    pub fn new(on_pressed: Option<ValueChanged<bool>>) -> ExpandIcon {
        ExpandIcon {
            key: None,
            is_expanded: false,
            size: 24.0,
            on_pressed,
            padding: EdgeInsetsGeometry::all(8.0),
            color: None,
            disabled_color: None,
            expanded_color: None,
            splash_color: None,
            highlight_color: None,
        }
    }

    /// Dart `ExpandIcon(key:)`.
    pub fn key(mut self, key: KeyRef) -> ExpandIcon {
        self.key = Some(key);
        self
    }

    /// Dart `ExpandIcon(isExpanded:)`.
    pub fn is_expanded(mut self, is_expanded: bool) -> ExpandIcon {
        self.is_expanded = is_expanded;
        self
    }

    /// Dart `ExpandIcon(size:)`.
    pub fn size(mut self, size: f64) -> ExpandIcon {
        self.size = size;
        self
    }

    /// Dart `ExpandIcon(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> ExpandIcon {
        self.padding = padding.into();
        self
    }

    /// Dart `ExpandIcon(color:)`.
    pub fn color(mut self, color: Color) -> ExpandIcon {
        self.color = Some(color);
        self
    }

    /// Dart `ExpandIcon(disabledColor:)`.
    pub fn disabled_color(mut self, disabled_color: Color) -> ExpandIcon {
        self.disabled_color = Some(disabled_color);
        self
    }

    /// Dart `ExpandIcon(expandedColor:)`.
    pub fn expanded_color(mut self, expanded_color: Color) -> ExpandIcon {
        self.expanded_color = Some(expanded_color);
        self
    }

    /// Dart `ExpandIcon(splashColor:)`.
    pub fn splash_color(mut self, splash_color: Color) -> ExpandIcon {
        self.splash_color = Some(splash_color);
        self
    }

    /// Dart `ExpandIcon(highlightColor:)`.
    pub fn highlight_color(mut self, highlight_color: Color) -> ExpandIcon {
        self.highlight_color = Some(highlight_color);
        self
    }
}

impl fmt::Debug for ExpandIcon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExpandIcon")
            .field("is_expanded", &self.is_expanded)
            .field("size", &self.size)
            .field("enabled", &self.on_pressed.is_some())
            .field("padding", &self.padding)
            .field("color", &self.color)
            .field("disabled_color", &self.disabled_color)
            .field("expanded_color", &self.expanded_color)
            .field("splash_color", &self.splash_color)
            .field("highlight_color", &self.highlight_color)
            .finish()
    }
}

impl StatefulWidget for ExpandIcon {
    type State = ExpandIconState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ExpandIconState {
        ExpandIconState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::default(),
            controller: None,
            icon_turns: None,
            half_turn_tween: None,
            fast_out_slow_in_tween: None,
        }
    }
}

/// Dart's `_ExpandIconState`.
pub struct ExpandIconState {
    state: StateData<ExpandIcon>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    /// Created in `init_state`, because a state is created without the [`App`].
    controller: Option<Ref<AnimationController>>,
    icon_turns: Option<AnimationRef<f64>>,
    /// Dart's `static final Animatable<double> _iconTurnTween`, a `Tween` chained to a
    /// `CurveTween`, created in `init_state` because an arena object needs the [`App`].
    half_turn_tween: Option<Ref<Tween<f64>>>,
    fast_out_slow_in_tween: Option<Ref<CurveTween>>,
}

impl ExpandIconState {
    fn controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .controller
            .as_ref()
            .expect("created in init_state")
            .handle()
    }

    fn icon_turns(self: Handle<Self>, app: &App) -> AnimationRef<f64> {
        app.get(self)
            .icon_turns
            .clone()
            .expect("created in init_state")
    }

    fn handle_pressed(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let is_expanded = widget.is_expanded;
        if let Some(on_pressed) = widget.on_pressed.clone() {
            on_pressed(app, is_expanded);
        }
    }

    /// Default icon colors and opacities for when `ThemeData::brightness` is set to
    /// [`Brightness::Light`] are based on the
    /// [Material Design system icon specifications](https://material.io/design/iconography/system-icons.html#color).
    /// Icon colors and opacities for [`Brightness::Dark`] are based on the
    /// [Material Design dark theme specifications](https://material.io/design/color/dark-theme.html#ui-application)
    fn icon_color(self: Handle<Self>, app: &mut App, context: BuildContext) -> Color {
        let widget = self.widget(app);
        if widget.is_expanded
            && let Some(expanded_color) = widget.expanded_color
        {
            return expanded_color;
        }

        if let Some(color) = widget.color {
            return color;
        }

        match Theme::brightness_of(app, context) {
            Brightness::Light => Colors::BLACK54,
            Brightness::Dark => Colors::WHITE60,
        }
    }
}

impl SingleTickerProviderStateMixin for ExpandIconState {
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

/// Dart's `vsync: this`.
impl TickerProviderObject for ExpandIconState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for ExpandIconState {
    type Widget = ExpandIcon;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let half_turn_tween = Tween::new(app, Some(0.0), Some(0.5));
        let fast_out_slow_in_tween = CurveTween::new(app, Curves::fast_out_slow_in());
        let controller = AnimationController::new(
            app,
            None,
            Some(K_THEME_ANIMATION_DURATION),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let icon_turns = controller.drive(
            app,
            half_turn_tween
                .clone()
                .chain(fast_out_slow_in_tween.clone()),
        );
        let state = app.get_mut(self);
        state.half_turn_tween = Some(half_turn_tween);
        state.fast_out_slow_in_tween = Some(fast_out_slow_in_tween);
        state.controller = Some(controller.clone());
        state.icon_turns = Some(icon_turns);
        // If the widget is initially expanded, rotate the icon without animating it.
        if self.widget(app).is_expanded {
            controller.set_value(app, PI);
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let controller = self.controller(app);
        controller.dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &ExpandIcon) {
        let is_expanded = self.widget(app).is_expanded;
        if is_expanded != old_widget.is_expanded {
            let controller = self.controller(app);
            if is_expanded {
                controller.forward(app, None);
            } else {
                controller.reverse(app, None);
            }
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_material(app, context));
        debug_assert!(debug_check_has_material_localizations(app, context));
        let localizations = <dyn MaterialLocalizations>::of(app, context);
        let on_tap_hint = if self.widget(app).is_expanded {
            localizations.expanded_icon_tap_hint()
        } else {
            localizations.collapsed_icon_tap_hint()
        };

        let icon_color = self.icon_color(app, context);
        let icon_turns = self.icon_turns(app);
        let widget = self.widget(app);
        let on_pressed = widget
            .on_pressed
            .as_ref()
            .map(|_| Listener::handle_method(self, Self::handle_pressed));
        let mut button = IconButton::new(
            on_pressed,
            RotationTransition::new(icon_turns).child(Icon::new(Some(Icons::expand_more()))),
        )
        .padding(widget.padding)
        .icon_size(widget.size)
        .color(icon_color);
        if let Some(highlight_color) = widget.highlight_color {
            button = button.highlight_color(highlight_color);
        }
        if let Some(splash_color) = widget.splash_color {
            button = button.splash_color(splash_color);
        }
        if let Some(disabled_color) = widget.disabled_color {
            button = button.disabled_color(disabled_color);
        }
        let mut semantics = Semantics::new();
        if widget.on_pressed.is_some() {
            semantics = semantics.on_tap_hint(on_tap_hint);
        }
        semantics.child(button).into_widget()
    }
}

impl fmt::Debug for ExpandIconState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExpandIconState")
            .field("controller", &self.controller)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use waft_embedder::Size;
    use waft_widgets::{Animated, Center, IconTheme, SizedBox, StatefulBuilder};

    use super::*;
    use crate::material::Material;
    use crate::scaffold::Scaffold;
    use crate::test_support::{WidgetTester, material_app, with_main_thread_stack};
    use crate::theme_data::ThemeData;

    fn tester() -> WidgetTester {
        let tester = WidgetTester::on(waft_embedder::TargetPlatform::Android);
        tester.install_fonts();
        tester
    }

    /// Dart's `ThemeData()`.
    fn m3(app: &App) -> ThemeData {
        ThemeData::builder().build(app)
    }

    /// Dart's `ThemeData(brightness: Brightness.dark)`.
    fn m3_dark(app: &App) -> ThemeData {
        ThemeData::builder().brightness(Brightness::Dark).build(app)
    }

    /// Dart's `wrap(child:, theme:)`: `MaterialApp(theme:, home: Center(child: Material(child:)))`.
    fn wrap<K>(tester: &mut WidgetTester, theme: fn(&App) -> ThemeData, child: impl IntoWidget<K>) {
        let theme = theme(&tester.cell.borrow());
        tester.pump_widget(material_app(
            theme,
            Center::new()
                .child(Material::new().child(child))
                .into_widget(),
        ));
    }

    /// Dart's `tester.firstWidget<IconTheme>(find.byType(IconTheme).last).data.color`.
    fn last_icon_theme_color(tester: &WidgetTester) -> Option<Color> {
        let last = *tester
            .find_by_type::<IconTheme>()
            .last()
            .expect("an IconTheme");
        tester.widget(last, |icon_theme: &IconTheme| {
            icon_theme.data.color.as_deref().copied()
        })
    }

    /// Dart's `tester.tap(find.byType(ExpandIcon))`.
    fn tap_expand_icon(tester: &mut WidgetTester) {
        let element = tester.find_by_type::<ExpandIcon>()[0];
        let render = tester.render_box_of(element);
        let center = tester.global_rect(render).center();
        tester.tap(center, 1);
    }

    fn toggling(expanded: &Rc<Cell<bool>>) -> ValueChanged<bool> {
        let expanded = Rc::clone(expanded);
        Rc::new(move |_app, _is_expanded| expanded.set(!expanded.get()))
    }

    /// "ExpandIcon test".
    #[test]
    fn expand_icon_test() {
        with_main_thread_stack(|| {
            let expanded = Rc::new(Cell::new(false));
            let mut tester = tester();

            // Light mode tests
            wrap(&mut tester, m3, ExpandIcon::new(Some(toggling(&expanded))));
            tester.pump_and_settle();

            assert!(!expanded.get());
            assert_eq!(last_icon_theme_color(&tester), Some(Colors::BLACK54));

            tap_expand_icon(&mut tester);
            tester.pump_and_settle();
            assert!(expanded.get());
            assert_eq!(last_icon_theme_color(&tester), Some(Colors::BLACK54));

            tap_expand_icon(&mut tester);
            tester.pump_and_settle();
            assert!(!expanded.get());
            assert_eq!(last_icon_theme_color(&tester), Some(Colors::BLACK54));

            // Dark mode tests
            wrap(
                &mut tester,
                m3_dark,
                ExpandIcon::new(Some(toggling(&expanded))),
            );
            tester.pump_and_settle();

            assert!(!expanded.get());
            assert_eq!(last_icon_theme_color(&tester), Some(Colors::WHITE60));

            tap_expand_icon(&mut tester);
            tester.pump_and_settle();
            assert!(expanded.get());
            assert_eq!(last_icon_theme_color(&tester), Some(Colors::WHITE60));

            tap_expand_icon(&mut tester);
            tester.pump_and_settle();
            assert!(!expanded.get());
            assert_eq!(last_icon_theme_color(&tester), Some(Colors::WHITE60));
        });
    }

    /// "Material2 - ExpandIcon disabled".
    #[test]
    fn material2_expand_icon_disabled() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            // Test light mode.
            wrap(
                &mut tester,
                |app| ThemeData::builder().use_material3(false).build(app),
                ExpandIcon::new(None),
            );
            tester.pump_and_settle();

            assert_eq!(last_icon_theme_color(&tester), Some(Colors::BLACK38));

            // Test dark mode.
            wrap(
                &mut tester,
                |app| {
                    ThemeData::builder()
                        .use_material3(false)
                        .brightness(Brightness::Dark)
                        .build(app)
                },
                ExpandIcon::new(None),
            );
            tester.pump_and_settle();

            assert_eq!(last_icon_theme_color(&tester), Some(Colors::WHITE38));
        });
    }

    /// "Material3 - ExpandIcon disabled".
    #[test]
    #[allow(deprecated)]
    fn material3_expand_icon_disabled() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            // Test light mode.
            wrap(&mut tester, m3, ExpandIcon::new(None));
            tester.pump_and_settle();

            let theme = m3(&tester.cell.borrow());
            assert_eq!(
                last_icon_theme_color(&tester),
                Some(theme.color_scheme.on_surface.with_opacity(0.38))
            );

            // Test dark mode.
            wrap(&mut tester, m3_dark, ExpandIcon::new(None));
            tester.pump_and_settle();

            let theme = m3_dark(&tester.cell.borrow());
            assert_eq!(
                last_icon_theme_color(&tester),
                Some(theme.color_scheme.on_surface.with_opacity(0.38))
            );
        });
    }

    /// "ExpandIcon test isExpanded does not trigger callback".
    #[test]
    fn expand_icon_test_is_expanded_does_not_trigger_callback() {
        with_main_thread_stack(|| {
            let expanded = Rc::new(Cell::new(false));
            let mut tester = tester();

            wrap(&mut tester, m3, ExpandIcon::new(Some(toggling(&expanded))));

            wrap(
                &mut tester,
                m3,
                ExpandIcon::new(Some(toggling(&expanded))).is_expanded(true),
            );

            assert!(!expanded.get());
        });
    }

    /// "ExpandIcon is rotated initially if isExpanded is true on first build".
    #[test]
    fn expand_icon_is_rotated_initially_if_is_expanded_is_true_on_first_build() {
        with_main_thread_stack(|| {
            let mut tester = tester();

            wrap(
                &mut tester,
                m3,
                ExpandIcon::new(Some(Rc::new(|_app, _is_expanded| {}))).is_expanded(true),
            );
            let rotation = tester.find_by_type::<Animated<RotationTransition>>()[0];
            let turns = tester.widget(rotation, |rotation: &Animated<RotationTransition>| {
                rotation.0.turns()
            });
            assert_eq!(turns.value(&tester.cell.borrow()), 0.5);
        });
    }

    /// "ExpandIcon default size is 24".
    #[test]
    fn expand_icon_default_size_is_24() {
        with_main_thread_stack(|| {
            let mut tester = tester();

            wrap(
                &mut tester,
                m3,
                ExpandIcon::new(Some(Rc::new(|_app, _is_expanded| {}))),
            );

            let icon = tester.find_by_type::<ExpandIcon>()[0];
            assert_eq!(tester.widget(icon, |icon: &ExpandIcon| icon.size), 24.0);
        });
    }

    /// "ExpandIcon has the correct given size".
    #[test]
    fn expand_icon_has_the_correct_given_size() {
        with_main_thread_stack(|| {
            let mut tester = tester();

            wrap(
                &mut tester,
                m3,
                ExpandIcon::new(Some(Rc::new(|_app, _is_expanded| {}))).size(36.0),
            );

            let icon = tester.find_by_type::<ExpandIcon>()[0];
            assert_eq!(tester.widget(icon, |icon: &ExpandIcon| icon.size), 36.0);

            wrap(
                &mut tester,
                m3,
                ExpandIcon::new(Some(Rc::new(|_app, _is_expanded| {}))).size(48.0),
            );

            let icon = tester.find_by_type::<ExpandIcon>()[0];
            assert_eq!(tester.widget(icon, |icon: &ExpandIcon| icon.size), 48.0);
        });
    }

    /// The `StatefulBuilder` of "ExpandIcon uses custom icon color and expanded icon color",
    /// flipping `expanded` on every press.
    fn stateful_expand_icon(
        expanded: &Rc<Cell<bool>>,
        configure: fn(ExpandIcon) -> ExpandIcon,
    ) -> StatefulBuilder {
        let expanded = Rc::clone(expanded);
        StatefulBuilder::new(move |_app, _context, set_state| {
            let on_pressed: ValueChanged<bool> = {
                let expanded = Rc::clone(&expanded);
                Rc::new(move |app, is_expanded| {
                    set_state(app, &mut |_app| expanded.set(!is_expanded));
                })
            };
            configure(ExpandIcon::new(Some(on_pressed)).is_expanded(expanded.get())).into_widget()
        })
    }

    /// "ExpandIcon uses custom icon color and expanded icon color".
    #[test]
    fn expand_icon_uses_custom_icon_color_and_expanded_icon_color() {
        with_main_thread_stack(|| {
            let expanded = Rc::new(Cell::new(false));
            let mut tester = tester();

            wrap(
                &mut tester,
                m3,
                stateful_expand_icon(&expanded, |icon| icon.color(Colors::INDIGO.primary())),
            );
            tester.pump_and_settle();
            assert_eq!(
                last_icon_theme_color(&tester),
                Some(Colors::INDIGO.primary())
            );

            tap_expand_icon(&mut tester);
            tester.pump_and_settle();
            assert_eq!(
                last_icon_theme_color(&tester),
                Some(Colors::INDIGO.primary())
            );

            expanded.set(false);

            wrap(
                &mut tester,
                m3,
                stateful_expand_icon(&expanded, |icon| {
                    icon.color(Colors::INDIGO.primary())
                        .expanded_color(Colors::TEAL.primary())
                }),
            );
            tester.pump_and_settle();
            assert_eq!(
                last_icon_theme_color(&tester),
                Some(Colors::INDIGO.primary())
            );

            tap_expand_icon(&mut tester);
            tester.pump_and_settle();
            assert_eq!(last_icon_theme_color(&tester), Some(Colors::TEAL.primary()));

            tap_expand_icon(&mut tester);
            tester.pump_and_settle();
            assert_eq!(
                last_icon_theme_color(&tester),
                Some(Colors::INDIGO.primary())
            );
        });
    }

    /// "ExpandIcon uses custom disabled icon color".
    #[test]
    fn expand_icon_uses_custom_disabled_icon_color() {
        with_main_thread_stack(|| {
            let mut tester = tester();

            wrap(
                &mut tester,
                m3,
                ExpandIcon::new(None).disabled_color(Colors::CYAN.primary()),
            );
            tester.pump_and_settle();
            assert_eq!(last_icon_theme_color(&tester), Some(Colors::CYAN.primary()));

            wrap(
                &mut tester,
                m3,
                ExpandIcon::new(None)
                    .color(Colors::INDIGO.primary())
                    .disabled_color(Colors::CYAN.primary()),
            );
            tester.pump_and_settle();
            assert_eq!(last_icon_theme_color(&tester), Some(Colors::CYAN.primary()));

            wrap(
                &mut tester,
                m3,
                ExpandIcon::new(None)
                    .is_expanded(true)
                    .disabled_color(Colors::CYAN.primary()),
            );
            wrap(
                &mut tester,
                m3,
                ExpandIcon::new(None)
                    .is_expanded(true)
                    .expanded_color(Colors::TEAL.primary())
                    .disabled_color(Colors::CYAN.primary()),
            );
            tester.pump_and_settle();
            assert_eq!(last_icon_theme_color(&tester), Some(Colors::CYAN.primary()));
        });
    }

    /// "Expand icon does not crash at zero area".
    #[test]
    fn expand_icon_does_not_crash_at_zero_area() {
        with_main_thread_stack(|| {
            let mut tester = tester();
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(material_app(
                theme,
                Scaffold::new()
                    .body(Center::new().child(
                        SizedBox::shrink().child(ExpandIcon::new(Some(Rc::new(|_app, _value| {})))),
                    ))
                    .into_widget(),
            ));
            let icon = tester.find_by_type::<ExpandIcon>()[0];
            let size = tester.render_box_of(icon).size(&tester.cell.borrow());
            assert_eq!(size, Size::ZERO);
        });
    }
}
