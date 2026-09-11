//! Flutter counterpart: `cupertino/button.dart`.
//!
//! `Semantics(button: true)` waits with accessibility.

use std::any::TypeId;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::time::Duration;

use reveal_animation::{
    AnimationBehavior, AnimationController, AnyAnimation, CurveTween, Curves, Tween,
};
use reveal_embedder::{Brightness, Size, TargetPlatform};
use reveal_foundation::{App, Handle, K_IS_WEB, Listener, ValueChanged};
use reveal_gestures::{
    LongPressGestureRecognizer, TapDownDetails, TapGestureRecognizer, TapMoveDetails, TapUpDetails,
};
use reveal_painting::{
    AlignmentGeometry, AnyColor, BorderRadius, BorderSide, BorderStyle, EdgeInsetsGeometry,
    HSLColor, RoundedSuperellipseBorder, ShapeDecoration,
};
use reveal_rendering::{BoxConstraints, HitTestBehavior};
use reveal_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use reveal_services::{MouseCursor, MouseCursorRef, SystemMouseCursors};
use reveal_widgets::{
    Action, ActivateIntent, Align, AnyAction, AnyFocusNode, BuildContext, CallbackAction,
    ConstrainedBox, DecoratedBox, DefaultTextStyle, FadeTransition, FocusableActionDetector,
    GestureRecognizerFactories, GestureRecognizerFactory, GestureRecognizerFactoryWithHandlers,
    IconTheme, IntoWidget, KeyRef, MediaQuery, MouseRegion, Padding, RawGestureDetector,
    SingleTickerProviderStateMixin, SingleTickerProviderStateMixinData, State, StateData,
    StatefulWidget, WidgetRef, WidgetState, WidgetStateProperty, WidgetStatePropertyRef,
    WidgetStates,
};

use crate::colors::{CupertinoColors, CupertinoDynamicColor};
use crate::constants::{
    K_CUPERTINO_BUTTON_DEFAULT_ICON_SIZE, K_CUPERTINO_BUTTON_TAP_MOVE_SLOP,
    K_CUPERTINO_BUTTON_TINTED_OPACITY_DARK, K_CUPERTINO_BUTTON_TINTED_OPACITY_LIGHT,
    K_CUPERTINO_FOCUS_COLOR_BRIGHTNESS, K_CUPERTINO_FOCUS_COLOR_OPACITY,
    K_CUPERTINO_FOCUS_COLOR_SATURATION, K_MIN_INTERACTIVE_DIMENSION_CUPERTINO,
    k_cupertino_button_min_size, k_cupertino_button_padding, k_cupertino_button_size_border_radius,
};
use crate::theme::CupertinoTheme;

/// The size of a [`CupertinoButton`].
///
/// Based on the iOS (17) [Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines/buttons#iOS-iPadOS).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CupertinoButtonSize {
    /// Displays a smaller button with round sides and smaller text (uses
    /// `CupertinoTextThemeData.actionSmallTextStyle`).
    Small,
    /// Displays a standard-sized button with round sides and regular-sized text.
    Medium,
    /// Displays a large button with a bigger [`CupertinoButtonSize::Small`] corner radius and
    /// regular-sized text.
    Large,
}

/// The style of a [`CupertinoButton`] that changes the style of the button's background.
///
/// Based on the iOS Human Interface Guidelines
/// (https://developer.apple.com/design/human-interface-guidelines/buttons#iOS-iPadOS).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CupertinoButtonStyle {
    /// No background or border, primary foreground color.
    Plain,
    /// Translucent background, primary foreground color.
    Tinted,
    /// Solid background, contrasting foreground color.
    Filled,
}

/// An iOS-style button.
///
/// Takes in a text or an icon that fades out and in on touch. May optionally have a
/// background.
///
/// The [`padding`](Self::padding) defaults to 16.0 pixels. When using a [`CupertinoButton`]
/// within a fixed height parent, like a `CupertinoNavigationBar`, a smaller, or even
/// `EdgeInsets.zero`, should be used to prevent clipping larger [`child`](Self::child)
/// widgets.
///
/// Preserves any parent `IconThemeData` but overwrites its `IconThemeData.color` with the
/// `CupertinoThemeData.primaryColor` (or `CupertinoThemeData.primaryContrastingColor` if the
/// button is disabled).
///
/// Dart's three constructors are [`new`](Self::new), [`tinted`](Self::tinted), and
/// [`filled`](Self::filled); their named arguments are the fluent setters.
///
/// See also:
///
///  * <https://developer.apple.com/design/human-interface-guidelines/buttons/>
#[derive(Clone)]
pub struct CupertinoButton {
    pub key: Option<KeyRef>,
    /// The widget below this widget in the tree.
    ///
    /// Typically a `Text` widget.
    pub child: WidgetRef,
    /// The size of the button.
    ///
    /// Defaults to [`CupertinoButtonSize::Large`].
    pub size_style: CupertinoButtonSize,
    /// The amount of space to surround the child inside the bounds of the button.
    ///
    /// Defaults to 16.0 pixels.
    pub padding: Option<EdgeInsetsGeometry>,
    /// The color of the button's background.
    ///
    /// Defaults to null which produces a button with no background or border.
    ///
    /// Defaults to the `CupertinoTheme`'s `primaryColor` when the [`filled`](Self::filled)
    /// constructor is used.
    pub color: Option<AnyColor>,
    /// The color of the button's text and icons.
    ///
    /// Defaults to the `CupertinoTheme`'s `primaryColor` when the [`filled`](Self::filled)
    /// constructor is used.
    pub foreground_color: Option<AnyColor>,
    /// The color of the button's background when the button is disabled.
    ///
    /// Ignored if the [`CupertinoButton`] doesn't also have a [`color`](Self::color).
    ///
    /// Defaults to [`CupertinoColors::QUATERNARY_SYSTEM_FILL`] when [`color`](Self::color) is
    /// specified.
    pub disabled_color: AnyColor,
    /// The minimum size of the button.
    ///
    /// Defaults to a button with a height and a width of
    /// [`K_MIN_INTERACTIVE_DIMENSION_CUPERTINO`], which the iOS Human Interface Guidelines
    /// recommends as the minimum tappable area.
    pub minimum_size: Option<Size>,
    /// The opacity that the button will fade to when it is pressed. The button will have an
    /// opacity of 1.0 when it is not pressed.
    ///
    /// This defaults to 0.4. If null, opacity will not change on pressed if using your own
    /// custom effects is desired.
    pub pressed_opacity: Option<f64>,
    /// The radius of the button's corners when it has a background color.
    ///
    /// Defaults to `kCupertinoButtonSizeBorderRadius`, based on
    /// [`size_style`](Self::size_style).
    pub border_radius: Option<BorderRadius>,
    /// The alignment of the button's [`child`](Self::child).
    ///
    /// Typically buttons are sized to be just big enough to contain the child and its
    /// [`padding`](Self::padding). If the button's size is constrained to a fixed size, for
    /// example by enclosing it with a `SizedBox`, this property defines how the child is
    /// aligned within the available space.
    ///
    /// Always defaults to `Alignment.center`.
    pub alignment: AlignmentGeometry,
    /// The color to use for the focus highlight for keyboard interactions.
    ///
    /// Defaults to a slightly transparent [`color`](Self::color). If [`color`](Self::color)
    /// is null, defaults to a slightly transparent [`CupertinoColors::ACTIVE_BLUE`]. Slightly
    /// transparent in this context means the color is used with an opacity of 0.80, a
    /// brightness of 0.69 and a saturation of 0.835.
    pub focus_color: Option<AnyColor>,
    /// The cursor for a mouse pointer when it enters or is hovering over the widget.
    ///
    /// If [`mouse_cursor`](Self::mouse_cursor) is a `WidgetStateMouseCursor`,
    /// `WidgetStateProperty.resolve` is used for the following `WidgetState`:
    ///  * `WidgetState.disabled`.
    ///  * `WidgetState.pressed`.
    ///  * `WidgetState.focused`.
    ///
    /// If null, then `MouseCursor.defer` is used when the button is disabled. When the button
    /// is enabled, `SystemMouseCursors.click` is used on Web and `MouseCursor.defer` is used
    /// on other platforms.
    pub mouse_cursor: Option<MouseCursorRef>,
    /// An optional focus node to use as the focus node for this widget.
    ///
    /// If one is not supplied, then one will be automatically allocated, owned, and managed
    /// by this widget. The widget will be focusable even if a `focus_node` is not supplied.
    /// If supplied, the given `focus_node` will be _hosted_ by this widget, but not owned.
    /// See `FocusNode` for more information on what being hosted and/or owned implies.
    pub focus_node: Option<AnyFocusNode>,
    /// Handler called when the focus changes.
    ///
    /// Called with true if this widget's node gains focus, and false if it loses focus.
    pub on_focus_change: Option<ValueChanged<bool>>,
    /// True if this widget will be selected as the initial focus when no other node in its
    /// scope is currently focused.
    ///
    /// Ideally, there is only one widget with autofocus set in each `FocusScope`. If there is
    /// more than one widget with autofocus set, then the first one added to the tree will get
    /// focus.
    ///
    /// Defaults to false.
    pub autofocus: bool,
    /// The callback that is called when the button is long-pressed.
    ///
    /// If [`on_pressed`](Self::on_pressed) and [`on_long_press`](Self::on_long_press)
    /// callbacks are null, then the button will be disabled.
    pub on_long_press: Option<Listener>,
    /// The callback that is called when the button is tapped or otherwise activated.
    ///
    /// If [`on_pressed`](Self::on_pressed) and [`on_long_press`](Self::on_long_press)
    /// callbacks are null, then the button will be disabled.
    pub on_pressed: Option<Listener>,
    style: CupertinoButtonStyle,
}

impl CupertinoButton {
    /// Creates an iOS-style button.
    pub fn new(child: WidgetRef, on_pressed: Option<Listener>) -> CupertinoButton {
        CupertinoButton::with_style(
            child,
            on_pressed,
            CupertinoButtonStyle::Plain,
            CupertinoColors::QUATERNARY_SYSTEM_FILL,
        )
    }

    /// Creates an iOS-style button with a tinted background.
    ///
    /// The background color is derived from the `CupertinoTheme`'s `primaryColor`. The
    /// foreground color is the `CupertinoTheme`'s `primaryColor`.
    ///
    /// To specify a custom background color, use the [`color`](Self::color) argument of the
    /// default constructor.
    pub fn tinted(child: WidgetRef, on_pressed: Option<Listener>) -> CupertinoButton {
        CupertinoButton::with_style(
            child,
            on_pressed,
            CupertinoButtonStyle::Tinted,
            CupertinoColors::TERTIARY_SYSTEM_FILL,
        )
    }

    /// Creates an iOS-style button with a filled background.
    ///
    /// The background color is derived from the `CupertinoTheme`'s `primaryColor`. The
    /// foreground color is the `CupertinoTheme`'s `primaryContrastingColor`.
    ///
    /// To specify a custom background color, use the [`color`](Self::color) argument of the
    /// default constructor.
    pub fn filled(child: WidgetRef, on_pressed: Option<Listener>) -> CupertinoButton {
        CupertinoButton::with_style(
            child,
            on_pressed,
            CupertinoButtonStyle::Filled,
            CupertinoColors::TERTIARY_SYSTEM_FILL,
        )
    }

    fn with_style(
        child: WidgetRef,
        on_pressed: Option<Listener>,
        style: CupertinoButtonStyle,
        disabled_color: AnyColor,
    ) -> CupertinoButton {
        CupertinoButton {
            key: None,
            child,
            size_style: CupertinoButtonSize::Large,
            padding: None,
            color: None,
            foreground_color: None,
            disabled_color,
            minimum_size: None,
            pressed_opacity: Some(0.4),
            border_radius: None,
            alignment: AlignmentGeometry::CENTER,
            focus_color: None,
            focus_node: None,
            on_focus_change: None,
            autofocus: false,
            mouse_cursor: None,
            on_long_press: None,
            on_pressed,
            style,
        }
    }

    /// Dart `CupertinoButton(key:)`.
    pub fn key(mut self, key: KeyRef) -> CupertinoButton {
        self.key = Some(key);
        self
    }

    /// Dart `CupertinoButton(sizeStyle:)`.
    pub fn size_style(mut self, size_style: CupertinoButtonSize) -> CupertinoButton {
        self.size_style = size_style;
        self
    }

    /// Dart `CupertinoButton(padding:)`.
    pub fn padding(mut self, padding: EdgeInsetsGeometry) -> CupertinoButton {
        self.padding = Some(padding);
        self
    }

    /// Dart `CupertinoButton(color:)`.
    pub fn color(mut self, color: impl Into<AnyColor>) -> CupertinoButton {
        self.color = Some(color.into());
        self
    }

    /// Dart `CupertinoButton(foregroundColor:)`.
    pub fn foreground_color(mut self, foreground_color: impl Into<AnyColor>) -> CupertinoButton {
        self.foreground_color = Some(foreground_color.into());
        self
    }

    /// Dart `CupertinoButton(disabledColor:)`.
    pub fn disabled_color(mut self, disabled_color: impl Into<AnyColor>) -> CupertinoButton {
        self.disabled_color = disabled_color.into();
        self
    }

    /// Dart `CupertinoButton(minimumSize:)`.
    pub fn minimum_size(mut self, minimum_size: Size) -> CupertinoButton {
        self.minimum_size = Some(minimum_size);
        self
    }

    /// Dart `CupertinoButton(pressedOpacity:)`; `None` keeps the opacity on press.
    pub fn pressed_opacity(mut self, pressed_opacity: Option<f64>) -> CupertinoButton {
        debug_assert!(pressed_opacity.is_none_or(|opacity| (0.0..=1.0).contains(&opacity)));
        self.pressed_opacity = pressed_opacity;
        self
    }

    /// Dart `CupertinoButton(borderRadius:)`.
    pub fn border_radius(mut self, border_radius: BorderRadius) -> CupertinoButton {
        self.border_radius = Some(border_radius);
        self
    }

    /// Dart `CupertinoButton(alignment:)`.
    pub fn alignment(mut self, alignment: AlignmentGeometry) -> CupertinoButton {
        self.alignment = alignment;
        self
    }

    /// Dart `CupertinoButton(focusColor:)`.
    pub fn focus_color(mut self, focus_color: impl Into<AnyColor>) -> CupertinoButton {
        self.focus_color = Some(focus_color.into());
        self
    }

    /// Dart `CupertinoButton(focusNode:)`.
    pub fn focus_node(mut self, focus_node: AnyFocusNode) -> CupertinoButton {
        self.focus_node = Some(focus_node);
        self
    }

    /// Dart `CupertinoButton(onFocusChange:)`.
    pub fn on_focus_change(
        mut self,
        on_focus_change: impl Fn(&mut App, bool) + 'static,
    ) -> CupertinoButton {
        self.on_focus_change = Some(Rc::new(on_focus_change));
        self
    }

    /// Dart `CupertinoButton(autofocus:)`.
    pub fn autofocus(mut self, autofocus: bool) -> CupertinoButton {
        self.autofocus = autofocus;
        self
    }

    /// Dart `CupertinoButton(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> CupertinoButton {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `CupertinoButton(onLongPress:)`.
    pub fn on_long_press(mut self, on_long_press: Listener) -> CupertinoButton {
        self.on_long_press = Some(on_long_press);
        self
    }

    /// Whether the button is enabled or disabled. Buttons are disabled by default. To enable
    /// a button, set [`on_pressed`](Self::on_pressed) or [`on_long_press`](Self::on_long_press)
    /// to a non-null value.
    pub fn enabled(&self) -> bool {
        self.on_pressed.is_some() || self.on_long_press.is_some()
    }

    /// The distance a button needs to be moved after being pressed for its opacity to change.
    ///
    /// The opacity changes when the position moved is this distance away from the button.
    pub fn tap_move_slop(app: &App) -> f64 {
        match app.platform().target_platform() {
            TargetPlatform::IOS | TargetPlatform::Android | TargetPlatform::Fuchsia => {
                K_CUPERTINO_BUTTON_TAP_MOVE_SLOP
            }
            TargetPlatform::MacOS | TargetPlatform::Linux | TargetPlatform::Windows => 0.0,
        }
    }
}

impl fmt::Debug for CupertinoButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CupertinoButton")
            .field("enabled", &self.enabled())
            .field("size_style", &self.size_style)
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for CupertinoButton {
    type State = CupertinoButtonState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CupertinoButtonState {
        CupertinoButtonState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::default(),
            opacity_tween: None,
            curve_tween: None,
            animation_controller: None,
            opacity_animation: None,
            is_focused: false,
            action_map: HashMap::new(),
            button_held_down: false,
            tap_in_progress: false,
        }
    }
}

// Eyeballed values. Feel free to tweak.
const K_FADE_OUT_DURATION: Duration = Duration::from_millis(120);
const K_FADE_IN_DURATION: Duration = Duration::from_millis(180);

/// Dart's `_CupertinoButtonState`.
pub struct CupertinoButtonState {
    state: StateData<CupertinoButton>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    /// Dart's `final Tween<double> _opacityTween = Tween<double>(begin: 1.0)`, created in
    /// `init_state` because a state is created without the `App`.
    opacity_tween: Option<Handle<Tween<f64>>>,
    curve_tween: Option<Handle<CurveTween>>,
    animation_controller: Option<Handle<AnimationController>>,
    opacity_animation: Option<AnyAnimation<f64>>,
    is_focused: bool,
    /// Dart's `late final _actionMap`, filled in `init_state` (an action is an arena object).
    action_map: HashMap<TypeId, AnyAction>,
    button_held_down: bool,
    tap_in_progress: bool,
}

impl CupertinoButtonState {
    /// Dart's `static final _defaultCursor`.
    fn default_cursor() -> WidgetStatePropertyRef<MouseCursorRef> {
        <dyn WidgetStateProperty<MouseCursorRef>>::resolve_with(|states: &WidgetStates| {
            if !states.contains(&WidgetState::Disabled) && K_IS_WEB {
                SystemMouseCursors::CLICK.into()
            } else {
                <dyn MouseCursor>::defer()
            }
        })
    }

    fn animation_controller(self: Handle<Self>, app: &App) -> Handle<AnimationController> {
        app.get(self)
            .animation_controller
            .expect("created in init_state")
    }

    fn set_tween(self: Handle<Self>, app: &mut App) {
        let end = self.widget(app).pressed_opacity.unwrap_or(1.0);
        let opacity_tween = app.get(self).opacity_tween.expect("created in init_state");
        opacity_tween.set_end(app, Some(end));
    }

    fn handle_tap_down(self: Handle<Self>, app: &mut App, _event: TapDownDetails) {
        self.set_state(app, |state| {
            state.tap_in_progress = true;
        });
        if !app.get(self).button_held_down {
            app.get_mut(self).button_held_down = true;
            self.animate(app);
        }
    }

    fn handle_tap_up(self: Handle<Self>, app: &mut App, event: TapUpDetails) {
        self.set_state(app, |state| {
            state.tap_in_progress = false;
        });
        if app.get(self).button_held_down {
            app.get_mut(self).button_held_down = false;
            self.animate(app);
        }
        let render_object = self.render_box(app);
        let local_position = render_object.global_to_local(app, event.global_position, None);
        if render_object
            .as_object()
            .paint_bounds(app)
            .inflate(CupertinoButton::tap_move_slop(app))
            .contains(local_position)
        {
            self.handle_tap(app);
        }
    }

    fn handle_tap_cancel(self: Handle<Self>, app: &mut App) {
        self.set_state(app, |state| {
            state.tap_in_progress = false;
        });
        if app.get(self).button_held_down {
            app.get_mut(self).button_held_down = false;
            self.animate(app);
        }
    }

    fn handle_tap_move(self: Handle<Self>, app: &mut App, event: TapMoveDetails) {
        let render_object = self.render_box(app);
        let local_position = render_object.global_to_local(app, event.global_position, None);
        let button_should_held_down = render_object
            .as_object()
            .paint_bounds(app)
            .inflate(CupertinoButton::tap_move_slop(app))
            .contains(local_position);
        let state = app.get_mut(self);
        if state.tap_in_progress && button_should_held_down != state.button_held_down {
            state.button_held_down = button_should_held_down;
            self.animate(app);
        }
    }

    /// Dart's `context.findRenderObject()! as RenderBox`.
    fn render_box(self: Handle<Self>, app: &App) -> reveal_rendering::AnyRenderBox {
        self.context(app)
            .find_render_object(app)
            .and_then(|render_object| render_object.as_box())
            .expect("the button has been laid out")
    }

    fn on_show_focus_highlight(self: Handle<Self>, app: &mut App, show_highlight: bool) {
        self.set_state(app, |state| state.is_focused = show_highlight);
    }

    fn handle_tap(self: Handle<Self>, app: &mut App) {
        if let Some(on_pressed) = self.widget(app).on_pressed.clone() {
            on_pressed.call(app);
            // `sendSemanticsEvent(TapSemanticEvent())` waits with accessibility.
        }
    }

    fn animate(self: Handle<Self>, app: &mut App) {
        let animation_controller = self.animation_controller(app);
        if animation_controller.is_animating(app) {
            return;
        }
        let was_held_down = app.get(self).button_held_down;
        let ticker = if was_held_down {
            animation_controller.animate_to(
                app,
                1.0,
                Some(K_FADE_OUT_DURATION),
                Curves::ease_in_out_cubic_emphasized(),
            )
        } else {
            animation_controller.animate_to(
                app,
                0.0,
                Some(K_FADE_IN_DURATION),
                Curves::ease_out_cubic(),
            )
        };
        ticker.when_complete(
            app,
            Listener::new(move |app| {
                if self.mounted(app) && was_held_down != app.get(self).button_held_down {
                    self.animate(app);
                }
            }),
        );
    }
}

impl SingleTickerProviderStateMixin for CupertinoButtonState {
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
impl TickerProviderObject for CupertinoButtonState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Handle<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for CupertinoButtonState {
    type Widget = CupertinoButton;
    reveal_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).is_focused = false;
        let activate = CallbackAction::<ActivateIntent>::new(
            app,
            Rc::new(move |app: &mut App, _intent: &ActivateIntent| {
                self.handle_tap(app);
                None
            }),
        );
        app.get_mut(self)
            .action_map
            .insert(TypeId::of::<ActivateIntent>(), activate.as_action());
        let animation_controller = AnimationController::create(
            app,
            Some(0.0),
            Some(Duration::from_millis(200)),
            None,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            self,
        );
        let opacity_tween = Tween::new(app, Some(1.0), None);
        let curve_tween = CurveTween::new(app, Curves::decelerate());
        let opacity_animation = animation_controller
            .drive(app, curve_tween)
            .drive(app, opacity_tween);
        let state = app.get_mut(self);
        state.animation_controller = Some(animation_controller);
        state.opacity_tween = Some(opacity_tween);
        state.curve_tween = Some(curve_tween);
        state.opacity_animation = Some(opacity_animation);
        self.set_tween(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &CupertinoButton) {
        self.set_tween(app);
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        for action in std::mem::take(&mut app.get_mut(self).action_map).into_values() {
            app.destroy(action.id());
        }
        let animation_controller = self.animation_controller(app);
        animation_controller.dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
        let state = app.get_mut(self);
        let opacity_tween = state.opacity_tween.take();
        let curve_tween = state.curve_tween.take();
        state.animation_controller = None;
        state.opacity_animation = None;
        app.destroy(animation_controller);
        if let Some(opacity_tween) = opacity_tween {
            app.destroy(opacity_tween);
        }
        if let Some(curve_tween) = curve_tween {
            app.destroy(curve_tween);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let enabled = widget.enabled();
        let minimum_size = widget.minimum_size;
        let theme_data = CupertinoTheme::of(app, context);
        let primary_color = theme_data.primary_color();
        let unresolved_background_color = match &widget.color {
            None => (widget.style != CupertinoButtonStyle::Plain).then(|| primary_color.clone()),
            Some(color) => CupertinoDynamicColor::maybe_resolve(Some(color), app, context),
        };
        let background_color = unresolved_background_color.map(|background_color| {
            // Dart's deprecated `withOpacity` is `with_values(alpha:)`, as its note says.
            let opacity = if widget.style == CupertinoButtonStyle::Tinted {
                if CupertinoTheme::brightness_of(app, context) == Brightness::Light {
                    K_CUPERTINO_BUTTON_TINTED_OPACITY_LIGHT
                } else {
                    K_CUPERTINO_BUTTON_TINTED_OPACITY_DARK
                }
            } else {
                widget.color.as_ref().map_or(1.0, |color| color.a)
            };
            AnyColor::new(background_color.with_values(Some(opacity), None, None, None, None))
        });
        let effective_foreground_color = match &widget.foreground_color {
            Some(foreground_color) => foreground_color.clone(),
            None => match (widget.style, enabled) {
                (CupertinoButtonStyle::Filled, _) => theme_data.primary_contrasting_color(),
                (_, true) => primary_color.clone(),
                (_, false) => {
                    CupertinoDynamicColor::resolve(&CupertinoColors::TERTIARY_LABEL, app, context)
                }
            },
        };
        let effective_focus_outline_color = match &widget.focus_color {
            Some(focus_color) => focus_color.color(),
            None => HSLColor::from_color(
                background_color
                    .clone()
                    .unwrap_or(CupertinoColors::ACTIVE_BLUE)
                    .with_values(
                        Some(K_CUPERTINO_FOCUS_COLOR_OPACITY),
                        None,
                        None,
                        None,
                        None,
                    ),
            )
            .with_lightness(K_CUPERTINO_FOCUS_COLOR_BRIGHTNESS)
            .with_saturation(K_CUPERTINO_FOCUS_COLOR_SATURATION)
            .to_color(),
        };
        let text_style = if widget.size_style == CupertinoButtonSize::Small {
            theme_data.text_theme().action_small_text_style()
        } else {
            theme_data.text_theme().action_text_style()
        }
        .copy_with()
        .color(effective_foreground_color.clone());
        let icon_theme = IconTheme::of(app, context)
            .copy_with()
            .color(effective_foreground_color)
            .size(
                text_style
                    .font_size
                    .map_or(K_CUPERTINO_BUTTON_DEFAULT_ICON_SIZE, |font_size| {
                        font_size * 1.2
                    }),
            );
        let gesture_settings = MediaQuery::maybe_gesture_settings_of(app, context);
        let is_focused = app.get(self).is_focused;
        let mut states = WidgetStates::new();
        if !enabled {
            states.insert(WidgetState::Disabled);
        }
        if app.get(self).tap_in_progress {
            states.insert(WidgetState::Pressed);
        }
        if is_focused {
            states.insert(WidgetState::Focused);
        }
        let effective_mouse_cursor: MouseCursorRef =
            <dyn WidgetStateProperty<MouseCursorRef>>::resolve_as(&widget.mouse_cursor, &states)
                .unwrap_or_else(|| CupertinoButtonState::default_cursor().resolve(&states));
        let side = if enabled && is_focused {
            BorderSide::new(
                effective_focus_outline_color,
                3.5,
                BorderStyle::Solid,
                BorderSide::STROKE_ALIGN_OUTSIDE,
            )
        } else {
            BorderSide::NONE
        };
        let shape = RoundedSuperellipseBorder::new(
            side,
            Some(
                widget
                    .border_radius
                    .unwrap_or_else(|| k_cupertino_button_size_border_radius(widget.size_style))
                    .into(),
            ),
        );
        let decoration_color = match background_color {
            Some(_) if !enabled => Some(CupertinoDynamicColor::resolve(
                &widget.disabled_color,
                app,
                context,
            )),
            background_color => background_color,
        };
        let shape_decoration = match decoration_color {
            Some(color) => ShapeDecoration::new(shape).color(color),
            None => ShapeDecoration::new(shape),
        };

        let mut gestures: GestureRecognizerFactories = Vec::new();
        gestures.push((
            TypeId::of::<TapGestureRecognizer>(),
            GestureRecognizerFactoryWithHandlers::<TapGestureRecognizer>::new(
                |app| TapGestureRecognizer::new(app).post_accept_slop_tolerance(app, None),
                move |app, instance: Handle<TapGestureRecognizer>| {
                    instance.set_on_tap_down(
                        app,
                        enabled.then(|| {
                            Rc::new(move |app: &mut App, details| {
                                self.handle_tap_down(app, details)
                            }) as _
                        }),
                    );
                    instance.set_on_tap_up(
                        app,
                        enabled.then(|| {
                            Rc::new(move |app: &mut App, details| self.handle_tap_up(app, details))
                                as _
                        }),
                    );
                    instance.set_on_tap_cancel(
                        app,
                        enabled.then(|| Listener::new(move |app| self.handle_tap_cancel(app))),
                    );
                    instance.set_on_tap_move(
                        app,
                        enabled.then(|| {
                            Rc::new(move |app: &mut App, details| {
                                self.handle_tap_move(app, details)
                            }) as _
                        }),
                    );
                    instance.set_gesture_settings(app, gesture_settings);
                },
            )
            .into_factory(),
        ));
        if let Some(on_long_press) = widget.on_long_press.clone() {
            gestures.push((
                TypeId::of::<LongPressGestureRecognizer>(),
                GestureRecognizerFactoryWithHandlers::<LongPressGestureRecognizer>::new(
                    LongPressGestureRecognizer::new,
                    move |app, instance: Handle<LongPressGestureRecognizer>| {
                        instance.set_on_long_press(app, Some(on_long_press.clone()));
                        instance.set_gesture_settings(app, gesture_settings);
                    },
                )
                .into_factory(),
            ));
        }

        let min_size = k_cupertino_button_min_size(widget.size_style);
        let constraints = BoxConstraints::new()
            .min_width(minimum_size.map_or(min_size, |size| size.width()))
            .min_height(minimum_size.map_or(min_size, |size| size.height()));
        let _ = K_MIN_INTERACTIVE_DIMENSION_CUPERTINO; // Dart's last `??` arm; the lookup is total.
        let padding = widget
            .padding
            .unwrap_or_else(|| k_cupertino_button_padding(widget.size_style));
        let opacity = app
            .get(self)
            .opacity_animation
            .expect("created in init_state");
        let mut detector = FocusableActionDetector::new(
            RawGestureDetector::new()
                .behavior(HitTestBehavior::Opaque)
                .gestures(gestures)
                .child(
                    ConstrainedBox::new(constraints).child(
                        FadeTransition::new(opacity).child(
                            DecoratedBox::new(shape_decoration).child(
                                Padding::new(padding).child(
                                    Align::new()
                                        .alignment(widget.alignment)
                                        .width_factor(1.0)
                                        .height_factor(1.0)
                                        .child(DefaultTextStyle::new(
                                            text_style,
                                            IconTheme::new(icon_theme, widget.child.clone())
                                                .into_widget(),
                                        )),
                                ),
                            ),
                        ),
                    ),
                ),
        )
        .actions(app.get(self).action_map.clone())
        .autofocus(widget.autofocus)
        .on_show_focus_highlight(move |app, show_highlight| {
            self.on_show_focus_highlight(app, show_highlight)
        })
        .enabled(enabled);
        if let Some(focus_node) = widget.focus_node {
            detector = detector.focus_node(focus_node);
        }
        if let Some(on_focus_change) = widget.on_focus_change.clone() {
            detector = detector.on_focus_change(move |app, focused| on_focus_change(app, focused));
        }
        // `Semantics(button: true)` waits with accessibility.
        MouseRegion::new()
            .cursor(effective_mouse_cursor)
            .child(detector)
            .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::Duration;

    use reveal_embedder::{PointerChange, PointerData, PointerDataPacket, PointerDeviceKind};
    use reveal_gestures::GestureBinding;
    use reveal_widgets::{
        Actions, Builder, Directionality, FocusNode, FocusNodeLeaf, GlobalKey, SizedBox,
    };

    use super::*;
    use crate::test_support::{build, pump};

    /// The test view is 2x: logical coordinates double into the packet.
    fn send(app: &mut App, change: PointerChange, x: f64, y: f64, at: Duration) {
        GestureBinding::instance(app).handle_pointer_data_packet(
            app,
            PointerDataPacket::new(vec![PointerData {
                change,
                kind: PointerDeviceKind::Touch,
                time_stamp: at,
                pointer_identifier: 1,
                physical_x: x * 2.0,
                physical_y: y * 2.0,
                ..PointerData::default()
            }]),
        );
        app.drain_microtasks();
    }

    struct Mounted {
        cell: Rc<AppCell>,
        state: Handle<CupertinoButtonState>,
        presses: Rc<Cell<u32>>,
    }

    /// Mounts a 40x40 button filling the view, with `on_pressed` counting.
    fn mount(configure: impl FnOnce(CupertinoButton) -> CupertinoButton) -> Mounted {
        let cell = crate::test_support::test_cell();
        let presses = Rc::new(Cell::new(0));
        let on_pressed = Listener::new({
            let presses = Rc::clone(&presses);
            move |_app| presses.set(presses.get() + 1)
        });
        let global_key = GlobalKey::new();
        let key: KeyRef = Rc::new(global_key.clone());
        let button = configure(CupertinoButton::new(
            SizedBox::square(Some(40.0)).into_widget(),
            Some(on_pressed),
        ))
        .key(key);
        build(&cell, button.into_widget());
        let mut app = cell.borrow_mut();
        let state = global_key
            .current_state::<CupertinoButtonState>(&mut app)
            .expect("the button mounted");
        drop(app);
        Mounted {
            cell,
            state,
            presses,
        }
    }

    #[test]
    fn an_activate_intent_presses_the_button_and_autofocus_takes_the_supplied_node() {
        let cell = crate::test_support::test_cell();
        let mut app = cell.borrow_mut();
        let node = FocusNode::new(&mut app).as_node();
        let presses = Rc::new(Cell::new(0));
        let on_pressed = Listener::new({
            let presses = Rc::clone(&presses);
            move |_app| presses.set(presses.get() + 1)
        });
        let inner_context: Rc<Cell<Option<BuildContext>>> = Rc::default();
        let child = Builder::new({
            let inner_context = Rc::clone(&inner_context);
            move |_app, context| {
                inner_context.set(Some(context));
                SizedBox::square(Some(40.0)).into_widget()
            }
        });
        drop(app);
        build(
            &cell,
            CupertinoButton::new(child.into_widget(), Some(on_pressed))
                .focus_node(node)
                .autofocus(true)
                .into_widget(),
        );
        let mut app = cell.borrow_mut();
        pump(&mut app, Duration::ZERO);
        assert!(
            node.has_primary_focus(&app),
            "autofocus focused the supplied node"
        );

        let context = inner_context.get().expect("the child built");
        Actions::invoke(&mut app, context, &ActivateIntent);
        assert_eq!(presses.get(), 1, "ActivateIntent runs onPressed");
    }

    fn opacity(mounted: &Mounted) -> f64 {
        mounted
            .cell
            .borrow()
            .get(mounted.state)
            .opacity_animation
            .expect("created in init_state")
            .value(&mounted.cell.borrow())
    }

    #[test]
    fn a_press_fades_the_button_and_a_release_fires_on_pressed() {
        let mounted = mount(|button| button);
        assert_eq!(opacity(&mounted), 1.0);

        send(
            &mut mounted.cell.borrow_mut(),
            PointerChange::Down,
            20.0,
            20.0,
            Duration::ZERO,
        );
        assert!(mounted.cell.borrow().get(mounted.state).tap_in_progress);
        assert!(mounted.cell.borrow().get(mounted.state).button_held_down);
        pump(&mut mounted.cell.borrow_mut(), Duration::from_millis(16));
        pump(&mut mounted.cell.borrow_mut(), Duration::from_millis(200));
        assert!((opacity(&mounted) - 0.4).abs() < 1e-9);
        assert_eq!(mounted.presses.get(), 0);

        send(
            &mut mounted.cell.borrow_mut(),
            PointerChange::Up,
            20.0,
            20.0,
            Duration::from_millis(210),
        );
        assert_eq!(mounted.presses.get(), 1);
        assert!(!mounted.cell.borrow().get(mounted.state).tap_in_progress);
        assert!(!mounted.cell.borrow().get(mounted.state).button_held_down);
        pump(&mut mounted.cell.borrow_mut(), Duration::from_millis(220));
        pump(&mut mounted.cell.borrow_mut(), Duration::from_millis(500));
        assert_eq!(opacity(&mounted), 1.0);
    }

    #[test]
    fn moving_past_the_slop_releases_the_button_without_a_press() {
        let mounted = mount(|button| button);
        send(
            &mut mounted.cell.borrow_mut(),
            PointerChange::Down,
            20.0,
            20.0,
            Duration::ZERO,
        );
        assert!(mounted.cell.borrow().get(mounted.state).button_held_down);

        send(
            &mut mounted.cell.borrow_mut(),
            PointerChange::Move,
            2000.0,
            2000.0,
            Duration::from_millis(50),
        );
        assert!(mounted.cell.borrow().get(mounted.state).tap_in_progress);
        assert!(!mounted.cell.borrow().get(mounted.state).button_held_down);

        send(
            &mut mounted.cell.borrow_mut(),
            PointerChange::Up,
            2000.0,
            2000.0,
            Duration::from_millis(60),
        );
        assert_eq!(mounted.presses.get(), 0);
        assert!(!mounted.cell.borrow().get(mounted.state).tap_in_progress);
    }

    #[test]
    fn a_disabled_button_ignores_the_pointer() {
        let cell = crate::test_support::test_cell();
        let global_key = GlobalKey::new();
        let key: KeyRef = Rc::new(global_key.clone());
        let button =
            CupertinoButton::new(SizedBox::square(Some(40.0)).into_widget(), None).key(key);
        assert!(!button.enabled());
        build(&cell, button.into_widget());
        let mut app = cell.borrow_mut();
        let state = global_key
            .current_state::<CupertinoButtonState>(&mut app)
            .expect("the button mounted");

        send(&mut app, PointerChange::Down, 20.0, 20.0, Duration::ZERO);
        send(
            &mut app,
            PointerChange::Up,
            20.0,
            20.0,
            Duration::from_millis(10),
        );
        assert!(!app.get(state).tap_in_progress);
        assert!(!app.get(state).button_held_down);
    }

    #[test]
    fn a_long_press_alone_enables_the_button_and_the_slop_follows_the_platform() {
        let cell = crate::test_support::test_cell();
        let app = cell.borrow();
        let long_pressed = Rc::new(Cell::new(0));
        let button = CupertinoButton::new(SizedBox::square(Some(40.0)).into_widget(), None)
            .on_long_press(Listener::new({
                let long_pressed = Rc::clone(&long_pressed);
                move |_app| long_pressed.set(long_pressed.get() + 1)
            }));
        assert!(button.enabled());
        let expected = match app.platform().target_platform() {
            TargetPlatform::IOS | TargetPlatform::Android | TargetPlatform::Fuchsia => {
                K_CUPERTINO_BUTTON_TAP_MOVE_SLOP
            }
            _ => 0.0,
        };
        assert_eq!(CupertinoButton::tap_move_slop(&app), expected);
    }

    /// A mouse hovers in, presses, and releases, as a desktop click arrives from the shell.
    fn click(app: &mut App, x: f64, y: f64) {
        for (change, buttons, at) in [
            (PointerChange::Add, 0, 0),
            (PointerChange::Hover, 0, 10),
            (PointerChange::Down, 1, 20),
            (PointerChange::Up, 0, 30),
        ] {
            GestureBinding::instance(app).handle_pointer_data_packet(
                app,
                PointerDataPacket::new(vec![PointerData {
                    change,
                    kind: PointerDeviceKind::Mouse,
                    time_stamp: Duration::from_millis(at),
                    device: 1,
                    pointer_identifier: 1,
                    buttons,
                    physical_x: x * 2.0,
                    physical_y: y * 2.0,
                    ..PointerData::default()
                }]),
            );
            app.drain_microtasks();
        }
    }

    #[test]
    fn a_mouse_click_presses_and_releases_the_button() {
        let mounted = mount(|button| button);
        click(&mut mounted.cell.borrow_mut(), 20.0, 20.0);
        assert_eq!(mounted.presses.get(), 1);
        assert!(!mounted.cell.borrow().get(mounted.state).tap_in_progress);
        // The fade-out runs to its end, then the fade-in it owes runs.
        for at in [16, 150, 300, 600] {
            pump(&mut mounted.cell.borrow_mut(), Duration::from_millis(at));
        }
        assert_eq!(opacity(&mounted), 1.0);
    }

    #[test]
    fn a_mouse_click_on_a_filled_text_button_in_a_dark_theme() {
        let cell = crate::test_support::test_cell();
        let mut app = cell.borrow_mut();
        let binding = reveal_painting::PaintingBinding::instance(&mut app);
        if !binding.has_fonts(&app) {
            binding.install_fonts(&mut app, |fonts| {
                fonts.add_source(valo_system_fonts::SystemFonts::load());
            });
        }
        let presses = Rc::new(Cell::new(0));
        let on_pressed = Listener::new({
            let presses = Rc::clone(&presses);
            move |_app| presses.set(presses.get() + 1)
        });
        let button = CupertinoButton::filled(
            reveal_widgets::Text::new("Press me").into_widget(),
            Some(on_pressed),
        );
        // The window example's shape: proxies (a decorated box, a sized box) above the
        // button, which `globalToLocal` walks through.
        let tree = Directionality::new(
            reveal_embedder::TextDirection::Ltr,
            DecoratedBox::new(reveal_painting::BoxDecoration::new()).child(
                Padding::new(EdgeInsetsGeometry::all(24.0)).child(
                    reveal_widgets::Center::new().child(
                        SizedBox::new()
                            .width(240.0)
                            .height(140.0)
                            .child(CupertinoTheme::new(
                                crate::theme::CupertinoThemeData::new()
                                    .with_brightness(Brightness::Dark),
                                reveal_widgets::Center::new().child(button),
                            )),
                    ),
                ),
            ),
        );
        drop(app);
        build(&cell, tree.into_widget());
        let mut app = cell.borrow_mut();

        click(&mut app, 200.0, 150.0);
        assert_eq!(presses.get(), 1);
        for at in [16, 150, 300, 600] {
            pump(&mut app, Duration::from_millis(at));
        }
    }

    struct Counter {
        count: i32,
    }

    struct Doubler {
        count: i32,
    }

    #[test]
    fn a_button_that_updates_an_entity_rebuilds_and_an_observer_follows() {
        let cell = crate::test_support::test_cell();
        let mut app = cell.borrow_mut();
        let counter = app.new_entity(|_cx| Counter { count: 0 });
        let doubler = app.new_entity(|cx| {
            cx.observe(&counter, |doubler: &mut Doubler, counter, cx| {
                doubler.count = counter.read(cx).count * 2;
            })
            .detach();
            Doubler { count: 0 }
        });
        let shown = Rc::new(Cell::new((0, 0)));
        let tree = Builder::new({
            let counter = counter.clone();
            let doubler = doubler.clone();
            let shown = Rc::clone(&shown);
            move |app, _context| {
                let n = counter.read(app).count;
                let d = doubler.read(app).count;
                shown.set((n, d));
                CupertinoButton::new(
                    SizedBox::square(Some(40.0)).into_widget(),
                    Some(Listener::new({
                        let counter = counter.clone();
                        move |app| {
                            counter.update(app, |counter, cx| {
                                counter.count += 1;
                                cx.notify();
                            });
                        }
                    })),
                )
                .into_widget()
            }
        });
        drop(app);
        build(&cell, tree.into_widget());
        assert_eq!(shown.get(), (0, 0));

        click(&mut cell.borrow_mut(), 20.0, 20.0);
        pump(&mut cell.borrow_mut(), Duration::from_millis(16));
        assert_eq!(shown.get(), (1, 2));
        assert_eq!(doubler.read(&cell.borrow()).count, 2);
    }
}
