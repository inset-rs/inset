//! Flutter counterpart: material_ui's `snack_bar.dart`.

use std::fmt;
use std::rc::Rc;
use std::time::Duration;
use waft_foundation::{App, Handle, Listenable, Listener, Ref, UniqueKey, VoidCallback};

use waft_animation::{
    Animation, AnimationBehavior, AnimationController, AnimationRef, AnimationStatus,
    AnimationStatusListener, AnyAnimation, Curve, CurvedAnimation, Curves, Interval, Threshold,
};
use waft_embedder::{Brightness, Clip, Color, Radius, TextDirection};
use waft_painting::{
    AlignmentGeometry, AnyColor, BorderRadiusGeometry, BorderSide, EdgeInsets, EdgeInsetsGeometry,
    PaintingBinding, RoundedRectangleBorder, ShapeBorderRef, TextPainter, TextSpan, TextStyle,
};
use waft_rendering::{HitTestBehavior, MainAxisAlignment};
use waft_scheduler::TickerProvider;
use waft_widgets::{
    Align, BuildContext, ClipRect, DefaultTextStyle, DismissDirection, Dismissible, Expanded,
    FadeTransition, Hero, Icon, IntoWidget, KeyRef, ListenableBuilder, MediaQuery, Padding, Row,
    SafeArea, Semantics, SizedBox, StandardComponentType, State, StateData, StatefulWidget, Text,
    WidgetRef, WidgetState, WidgetStateColor, WidgetStateProperty, WidgetStatePropertyRef, Wrap,
    debug_check_has_media_query,
};

use crate::color_scheme::ColorScheme;
use crate::colors::Colors;
use crate::generated::snackbar_defaults_m3::SnackbarDefaultsM3;
use crate::icon_button::IconButton;
use crate::icons::Icons;
use crate::material::Material;
use crate::material_localizations::MaterialLocalizations;
use crate::scaffold::ScaffoldMessenger;
use crate::snack_bar_theme::{
    SnackBarBehavior, SnackBarTheme, SnackBarThemeData, SnackBarThemeDataGetters,
};
use crate::text_button::TextButton;
use crate::text_button_theme::{TextButtonTheme, TextButtonThemeData};
use crate::theme::Theme;
use crate::theme_data::ThemeData;

const SINGLE_LINE_VERTICAL_PADDING: f64 = 14.0;
const SNACK_BAR_TRANSITION_DURATION: Duration = Duration::from_millis(250);
const SNACK_BAR_DISPLAY_DURATION: Duration = Duration::from_millis(4000);

fn snack_bar_height_curve() -> Rc<dyn Curve> {
    Curves::fast_out_slow_in()
}

fn snack_bar_m3_height_curve() -> Rc<dyn Curve> {
    Curves::ease_in_out_quart()
}

fn snack_bar_fade_in_curve() -> Rc<dyn Curve> {
    Rc::new(Interval::new(0.4, 1.0, Curves::linear()))
}

fn snack_bar_m3_fade_in_curve() -> Rc<dyn Curve> {
    Rc::new(Interval::new(0.4, 0.6, Curves::ease_in_circ()))
}

fn snack_bar_fade_out_curve() -> Rc<dyn Curve> {
    Rc::new(Interval::new(0.72, 1.0, Curves::fast_out_slow_in()))
}

/// Specify how a [`SnackBar`] was closed.
///
/// The `ScaffoldMessengerState::show_snack_bar` function returns a
/// `ScaffoldFeatureController`. The value of the controller's closed property
/// is a future that resolves to a [`SnackBarClosedReason`]. Applications that need
/// to know how a snackbar was closed can use this value.
///
/// Example:
///
/// ```text
/// ScaffoldMessenger::of(app, context)
///     .show_snack_bar(app, SnackBar::new(Text::new("He likes me. I think he likes me.")), None)
///     .closed()
///     .then(app, |app, reason: SnackBarClosedReason| {
///         // ...
///     });
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SnackBarClosedReason {
    /// The snack bar was closed after the user tapped a [`SnackBarAction`].
    Action,

    /// The snack bar was closed through a `SemanticsAction.dismiss`.
    Dismiss,

    /// The snack bar was closed by a user's swipe.
    Swipe,

    /// The snack bar was closed by the `ScaffoldFeatureController` close callback
    /// or by calling `ScaffoldMessengerState::hide_current_snack_bar` directly.
    Hide,

    /// The snack bar was closed by an call to
    /// `ScaffoldMessengerState::remove_current_snack_bar`.
    Remove,

    /// The snack bar was closed because its timer expired.
    Timeout,
}

/// A button for a [`SnackBar`], known as an "action".
///
/// Snack bar actions are always enabled. Instead of disabling a snack bar
/// action, avoid including it in the snack bar in the first place.
///
/// Snack bar actions can only be pressed once. Subsequent presses are ignored.
///
/// See also:
///
///  * [`SnackBar`]
///  * <https://material.io/design/components/snackbars.html>
#[derive(Clone)]
pub struct SnackBarAction {
    pub key: Option<KeyRef>,

    /// The button label color. If not provided, defaults to
    /// [`SnackBarThemeData::action_text_color`].
    ///
    /// If [`text_color`](Self::text_color) is a [`WidgetStateColor`], then the text color
    /// will be resolved against the set of [`WidgetState`]s that the action text
    /// is in, thus allowing for different colors for states such as pressed,
    /// hovered and others.
    pub text_color: Option<AnyColor>,

    /// The button background fill color. If not provided, defaults to
    /// [`SnackBarThemeData::action_background_color`].
    ///
    /// If [`background_color`](Self::background_color) is a [`WidgetStateColor`], then the
    /// text color will be resolved against the set of [`WidgetState`]s that the action text
    /// is in, thus allowing for different colors for the states.
    pub background_color: Option<AnyColor>,

    /// The button disabled label color. This color is shown after the
    /// [`SnackBarAction`] is dismissed.
    pub disabled_text_color: Option<Color>,

    /// The button disabled background color. This color is shown after the
    /// [`SnackBarAction`] is dismissed.
    ///
    /// If not provided, defaults to [`SnackBarThemeData::disabled_action_background_color`].
    pub disabled_background_color: Option<Color>,

    /// The button label.
    pub label: String,

    /// The callback to be called when the button is pressed.
    ///
    /// This callback will be called at most once each time this action is
    /// displayed in a [`SnackBar`].
    pub on_pressed: VoidCallback,
}

impl SnackBarAction {
    /// Creates an action for a [`SnackBar`]; Dart's optional arguments are the setters.
    pub fn new(
        label: impl Into<String>,
        on_pressed: impl Fn(&mut App) + 'static,
    ) -> SnackBarAction {
        SnackBarAction {
            key: None,
            text_color: None,
            background_color: None,
            disabled_text_color: None,
            disabled_background_color: None,
            label: label.into(),
            on_pressed: Rc::new(on_pressed),
        }
    }

    /// Dart `SnackBarAction(key:)`.
    pub fn key(mut self, key: KeyRef) -> SnackBarAction {
        self.key = Some(key);
        self
    }

    /// Dart `SnackBarAction(textColor:)`.
    pub fn text_color(mut self, text_color: impl Into<AnyColor>) -> SnackBarAction {
        self.text_color = Some(text_color.into());
        self
    }

    /// Dart `SnackBarAction(disabledTextColor:)`.
    pub fn disabled_text_color(mut self, disabled_text_color: Color) -> SnackBarAction {
        self.disabled_text_color = Some(disabled_text_color);
        self
    }

    /// Dart `SnackBarAction(backgroundColor:)`.
    pub fn background_color(mut self, background_color: impl Into<AnyColor>) -> SnackBarAction {
        let background_color = background_color.into();
        debug_assert!(
            !is_widget_state_color(&background_color) || self.disabled_background_color.is_none(),
            "disabledBackgroundColor must not be provided when background color is a \
             WidgetStateColor"
        );
        self.background_color = Some(background_color);
        self
    }

    /// Dart `SnackBarAction(disabledBackgroundColor:)`.
    pub fn disabled_background_color(mut self, disabled_background_color: Color) -> SnackBarAction {
        debug_assert!(
            !self
                .background_color
                .as_ref()
                .is_some_and(is_widget_state_color),
            "disabledBackgroundColor must not be provided when background color is a \
             WidgetStateColor"
        );
        self.disabled_background_color = Some(disabled_background_color);
        self
    }
}

impl fmt::Debug for SnackBarAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SnackBarAction")
            .field("key", &self.key)
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for SnackBarAction {
    type State = SnackBarActionState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> SnackBarActionState {
        SnackBarActionState {
            state: StateData::new(),
            have_triggered_action: false,
        }
    }
}

/// Dart's `_SnackBarActionState`.
pub struct SnackBarActionState {
    state: StateData<SnackBarAction>,
    have_triggered_action: bool,
}

impl SnackBarActionState {
    fn handle_pressed(self: Handle<Self>, app: &mut App) {
        if app.get(self).have_triggered_action {
            return;
        }
        self.set_state(app, |this| {
            this.have_triggered_action = true;
        });
        let on_pressed = Rc::clone(&self.widget(app).on_pressed);
        on_pressed(app);
        let context = self.context(app);
        ScaffoldMessenger::of(app, context)
            .hide_current_snack_bar(app, SnackBarClosedReason::Action);
    }
}

impl State for SnackBarActionState {
    type Widget = SnackBarAction;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let defaults: Box<dyn SnackBarThemeDataGetters> = if Theme::of(app, context).use_material3 {
            Box::new(SnackbarDefaultsM3::new(context))
        } else {
            Box::new(SnackbarDefaultsM2::new(app, context))
        };
        let snack_bar_theme = SnackBarTheme::of(app, context);
        let widget = self.widget(app).clone();

        let resolve_foreground_color = |app: &mut App| -> AnyColor {
            if let Some(text_color) = &widget.text_color {
                if is_widget_state_color(text_color) {
                    return text_color.clone();
                }
            } else if let Some(action_text_color) = &snack_bar_theme.action_text_color {
                if is_widget_state_color(action_text_color) {
                    return action_text_color.clone();
                }
            } else if let Some(action_text_color) = defaults.action_text_color(app)
                && is_widget_state_color(&action_text_color)
            {
                return action_text_color;
            }

            let disabled_text_color = widget
                .disabled_text_color
                .or(snack_bar_theme.disabled_action_text_color)
                .or_else(|| defaults.disabled_action_text_color(app));
            let text_color = widget
                .text_color
                .clone()
                .or_else(|| snack_bar_theme.action_text_color.clone())
                .or_else(|| defaults.action_text_color(app))
                .map(|color| color.color());
            WidgetStateColor::resolve_with(move |states| {
                if states.contains(&WidgetState::Disabled) {
                    return disabled_text_color
                        .expect("the defaults have a disabled action text color");
                }
                text_color.expect("the defaults have an action text color")
            })
            .into_any()
        };

        let resolve_background_color = || -> AnyColor {
            if let Some(background_color) = &widget.background_color
                && is_widget_state_color(background_color)
            {
                return background_color.clone();
            }
            if let Some(action_background_color) = &snack_bar_theme.action_background_color
                && is_widget_state_color(action_background_color)
            {
                return action_background_color.clone();
            }
            let disabled_background_color = widget
                .disabled_background_color
                .or(snack_bar_theme.disabled_action_background_color)
                .unwrap_or(Colors::TRANSPARENT);
            let background_color = widget
                .background_color
                .as_ref()
                .or(snack_bar_theme.action_background_color.as_ref())
                .map_or(Colors::TRANSPARENT, AnyColor::color);
            WidgetStateColor::resolve_with(move |states| {
                if states.contains(&WidgetState::Disabled) {
                    return disabled_background_color;
                }
                background_color
            })
            .into_any()
        };

        let style = TextButton::style_from()
            .overlay_color(resolve_foreground_color(app).color())
            .build()
            .copy_with();
        let style = crate::button_style::ButtonStyle {
            foreground_color: Some(widget_state_color_property(resolve_foreground_color(app))),
            background_color: Some(widget_state_color_property(resolve_background_color())),
            ..style
        };
        let label = widget.label.clone();
        let on_pressed = if app.get(self).have_triggered_action {
            None
        } else {
            Some(Listener::handle_method(self, Self::handle_pressed))
        };
        TextButton::new(on_pressed, Some(Text::new(label)))
            .style(style)
            .into_widget()
    }
}

/// Whether `color` is a [`WidgetStateColor`]: Dart's `color is WidgetStateColor`.
fn is_widget_state_color(color: &AnyColor) -> bool {
    color.extension::<WidgetStateColor>().is_some()
}

/// The [`WidgetStateColor`] `color` carries as the button-style property Dart passes it as.
fn widget_state_color_property(color: AnyColor) -> WidgetStatePropertyRef<Option<Color>> {
    <dyn WidgetStateProperty<Option<Color>>>::resolve_with(move |states| {
        Some(
            color
                .extension::<WidgetStateColor>()
                .expect("resolved as a WidgetStateColor")
                .resolve(states),
        )
    })
}

/// A lightweight message with an optional action which briefly displays at the
/// bottom of the screen.
///
/// Learn more about [`SnackBar`] on the [Flutter YouTube channel](https://www.youtube.com/watch?v=zpO6n_oZWw0).
///
/// To display a snack bar, call `ScaffoldMessenger::of(app, context).show_snack_bar(..)`,
/// passing an instance of [`SnackBar`] that describes the message.
///
/// To control how long the [`SnackBar`] remains visible, specify a
/// [`duration`](Self::duration).
///
/// A SnackBar with an action will not time out when TalkBack or VoiceOver are
/// enabled. This is controlled by `AccessibilityFeatures.accessibleNavigation`.
///
/// During page transitions, the [`SnackBar`] will smoothly animate to its
/// location on the other page. For example if the [`SnackBar::behavior`] is set to
/// [`SnackBarBehavior::Floating`] and the next page has a floating action button,
/// while the current one does not, the [`SnackBar`] will smoothly animate above
/// the floating action button. It also works in the case of a back gesture
/// transition.
///
/// See also:
///
///  * `ScaffoldMessenger::of`, to obtain the current `ScaffoldMessengerState`,
///    which manages the display and animation of snack bars.
///  * `ScaffoldMessengerState::show_snack_bar`, which displays a [`SnackBar`].
///  * `ScaffoldMessengerState::remove_current_snack_bar`, which abruptly hides the
///    currently displayed snack bar, if any, and allows the next to be displayed.
///  * [`SnackBarAction`], which is used to specify an [`action`](Self::action) button to
///    show on the snack bar.
///  * [`SnackBarThemeData`], to configure the default property values for
///    [`SnackBar`] widgets.
///  * <https://material.io/design/components/snackbars.html>
#[derive(Clone)]
pub struct SnackBar {
    pub key: Option<KeyRef>,

    /// The primary content of the snack bar.
    ///
    /// Typically a `Text` widget.
    pub content: WidgetRef,

    /// The snack bar's background color.
    ///
    /// If not specified, the ambient [`SnackBarThemeData::background_color`] is used.
    /// If that is not specified it will default to a
    /// dark variation of `ColorScheme.surface` for light themes, or
    /// `ColorScheme.onSurface` for dark themes.
    pub background_color: Option<Color>,

    /// The z-coordinate at which to place the snack bar. This controls the size
    /// of the shadow below the snack bar.
    ///
    /// Defines the card's `Material.elevation`.
    ///
    /// If this property is null, then the ambient [`SnackBarThemeData::elevation`]
    /// is used, if that is also null, the default value is 6.0.
    pub elevation: Option<f64>,

    /// Empty space to surround the snack bar.
    ///
    /// This property is only used when [`behavior`](Self::behavior) is
    /// [`SnackBarBehavior::Floating`]. It can not be used if [`width`](Self::width) is
    /// specified.
    ///
    /// If this property is null, then the ambient [`SnackBarThemeData::inset_padding`]
    /// is used. If that is also null, then the default is
    /// `EdgeInsets.fromLTRB(15.0, 5.0, 15.0, 10.0)`.
    ///
    /// If this property is not null and [`hit_test_behavior`](Self::hit_test_behavior) is
    /// null, then [`hit_test_behavior`](Self::hit_test_behavior) default is
    /// [`HitTestBehavior::DeferToChild`].
    pub margin: Option<EdgeInsetsGeometry>,

    /// The amount of padding to apply to the snack bar's content and optional
    /// action.
    ///
    /// If this property is null, the default padding values are as follows:
    ///
    /// * [`content`](Self::content)
    ///     * Top and bottom paddings are 14.
    ///     * Left padding is 24 if [`behavior`](Self::behavior) is
    ///       [`SnackBarBehavior::Fixed`], 16 if [`behavior`](Self::behavior) is
    ///       [`SnackBarBehavior::Floating`].
    ///     * Right padding is same as start padding if there is no
    ///       [`action`](Self::action), otherwise 0.
    /// * [`action`](Self::action)
    ///     * Top and bottom paddings are 14.
    ///     * Left and right paddings are half of [`content`](Self::content)'s left padding.
    ///
    /// If this property is not null, the padding is as follows:
    ///
    /// * [`content`](Self::content)
    ///     * Left, top and bottom paddings are assigned normally.
    ///     * Right padding is assigned normally if there is no [`action`](Self::action),
    ///       otherwise 0.
    /// * [`action`](Self::action)
    ///     * Left padding is replaced with half the right padding.
    ///     * Top and bottom paddings are assigned normally.
    ///     * Right padding is replaced with one and a half times the
    ///       right padding.
    pub padding: Option<EdgeInsetsGeometry>,

    /// The width of the snack bar.
    ///
    /// If width is specified, the snack bar will be centered horizontally in the
    /// available space. This property is only used when [`behavior`](Self::behavior) is
    /// [`SnackBarBehavior::Floating`]. It can not be used if [`margin`](Self::margin) is
    /// specified.
    ///
    /// If this property is null, then the ambient [`SnackBarThemeData::width`]
    /// is used. If that is null, the snack bar will take up the full device
    /// width less the margin.
    pub width: Option<f64>,

    /// The shape of the snack bar's `Material`.
    ///
    /// Defines the snack bar's `Material.shape`.
    ///
    /// If this property is null, then the ambient [`SnackBarThemeData::shape`]
    /// is used. If that's null then the shape will
    /// depend on the [`SnackBarBehavior`]. For [`SnackBarBehavior::Fixed`], no
    /// overriding shape is specified, so the [`SnackBar`] is rectangular. For
    /// [`SnackBarBehavior::Floating`], it uses a [`RoundedRectangleBorder`] with a
    /// circular corner radius of 4.0.
    pub shape: Option<ShapeBorderRef>,

    /// Defines how the snack bar area, including margin, will behave during hit testing.
    ///
    /// If this property is null, and [`margin`](Self::margin) is not null or the ambient
    /// [`SnackBarThemeData::inset_padding`] is not null, then
    /// [`HitTestBehavior::DeferToChild`] is used by default.
    ///
    /// Please refer to [`HitTestBehavior`] for a detailed explanation of every behavior.
    pub hit_test_behavior: Option<HitTestBehavior>,

    /// This defines the behavior and location of the snack bar.
    ///
    /// Defines where a [`SnackBar`] should appear within a `Scaffold` and how its
    /// location should be adjusted when the scaffold also includes a
    /// `FloatingActionButton` or a `BottomNavigationBar`
    ///
    /// If this property is null, then the ambient [`SnackBarThemeData::behavior`]
    /// is used. If that is null, then the default is [`SnackBarBehavior::Fixed`].
    ///
    /// If this value is [`SnackBarBehavior::Floating`], the length of the bar
    /// is defined by either [`width`](Self::width) or [`margin`](Self::margin).
    pub behavior: Option<SnackBarBehavior>,

    /// (optional) An action that the user can take based on the snack bar.
    ///
    /// For example, the snack bar might let the user undo the operation that
    /// prompted the snackbar. Snack bars can have at most one action.
    ///
    /// The action should not be "dismiss" or "cancel".
    pub action: Option<SnackBarAction>,

    /// (optional) The percentage threshold for action widget's width before it overflows
    /// to a new line.
    ///
    /// Must be between 0 and 1.
    /// If the width of the [`action`](Self::action) divided by the total snackbar width
    /// is greater than this percentage, the [`action`](Self::action) will appear below the
    /// [`content`](Self::content).
    ///
    /// At a value of 0, the action will always overflow to a new line.
    ///
    /// Defaults to 0.25.
    pub action_overflow_threshold: Option<f64>,

    /// (optional) Whether to include a "close" icon widget.
    ///
    /// Tapping the icon will close the snack bar.
    pub show_close_icon: Option<bool>,

    /// An optional color for the close icon, if [`show_close_icon`](Self::show_close_icon)
    /// is true.
    ///
    /// If this property is null, then the ambient [`SnackBarThemeData::close_icon_color`]
    /// is used. If that is null, then the default is inverse surface.
    ///
    /// If [`close_icon_color`](Self::close_icon_color) is a [`WidgetStateColor`], then the
    /// icon color will be resolved against the set of [`WidgetState`]s that the action text
    /// is in, thus allowing for different colors for states such as pressed,
    /// hovered and others.
    pub close_icon_color: Option<AnyColor>,

    /// The amount of time the snack bar should be displayed.
    ///
    /// Defaults to 4.0s.
    ///
    /// See also:
    ///
    ///  * `ScaffoldMessengerState::remove_current_snack_bar`, which abruptly hides the
    ///    currently displayed snack bar, if any, and allows the next to be
    ///    displayed.
    ///  * <https://material.io/design/components/snackbars.html>
    pub duration: Duration,

    /// Whether the snack bar will stay or auto-dismiss after timeout, as given; read
    /// [`effective_persist`](Self::effective_persist).
    ///
    /// If true, the snack bar remains visible even after the timeout, until the
    /// user taps the action button or the close icon.
    ///
    /// If false, the snack bar will be dismissed after the timeout.
    ///
    /// If not provided, but the snackbar action is not null, the snackbar will
    /// persist as well.
    pub persist: Option<bool>,

    /// The animation driving the entrance and exit of the snack bar.
    pub animation: Option<AnimationRef<f64>>,

    /// Called the first time that the snackbar is visible within a `Scaffold`.
    ///
    /// When multiple `Scaffold`s are registered to the same `ScaffoldMessengerState`,
    /// [`on_visible`](Self::on_visible) is called once for each scaffold.
    ///
    /// See also:
    ///
    ///  * `ScaffoldMessenger`, which manages [`SnackBar`]s for `Scaffold` descendants.
    pub on_visible: Option<VoidCallback>,

    /// The direction in which the SnackBar can be dismissed.
    ///
    /// If this property is null, then the ambient [`SnackBarThemeData::dismiss_direction`]
    /// is used. If that is null, then the default is [`DismissDirection::Down`].
    pub dismiss_direction: Option<DismissDirection>,

    /// The content will be clipped (or not) according to this option.
    ///
    /// See the enum [`Clip`] for details of all possible options and their common use cases.
    ///
    /// Defaults to [`Clip::HardEdge`].
    pub clip_behavior: Clip,
}

impl SnackBar {
    /// Creates a snack bar; Dart's optional arguments are the setters.
    ///
    /// The [`elevation`](Self::elevation) must be null or non-negative.
    pub fn new<K>(content: impl IntoWidget<K>) -> SnackBar {
        SnackBar {
            key: None,
            content: content.into_widget(),
            background_color: None,
            elevation: None,
            margin: None,
            padding: None,
            width: None,
            shape: None,
            hit_test_behavior: None,
            behavior: None,
            action: None,
            action_overflow_threshold: None,
            show_close_icon: None,
            close_icon_color: None,
            duration: SNACK_BAR_DISPLAY_DURATION,
            persist: None,
            animation: None,
            on_visible: None,
            dismiss_direction: None,
            clip_behavior: Clip::HardEdge,
        }
    }

    /// Dart `SnackBar(key:)`.
    pub fn key(mut self, key: KeyRef) -> SnackBar {
        self.key = Some(key);
        self
    }

    /// Dart `SnackBar(backgroundColor:)`.
    pub fn background_color(mut self, background_color: Color) -> SnackBar {
        self.background_color = Some(background_color);
        self
    }

    /// Dart `SnackBar(elevation:)`.
    pub fn elevation(mut self, elevation: f64) -> SnackBar {
        debug_assert!(elevation >= 0.0);
        self.elevation = Some(elevation);
        self
    }

    /// Dart `SnackBar(margin:)`.
    pub fn margin(mut self, margin: impl Into<EdgeInsetsGeometry>) -> SnackBar {
        debug_assert!(
            self.width.is_none(),
            "Width and margin can not be used together"
        );
        self.margin = Some(margin.into());
        self
    }

    /// Dart `SnackBar(padding:)`.
    pub fn padding(mut self, padding: impl Into<EdgeInsetsGeometry>) -> SnackBar {
        self.padding = Some(padding.into());
        self
    }

    /// Dart `SnackBar(width:)`.
    pub fn width(mut self, width: f64) -> SnackBar {
        debug_assert!(
            self.margin.is_none(),
            "Width and margin can not be used together"
        );
        self.width = Some(width);
        self
    }

    /// Dart `SnackBar(shape:)`.
    pub fn shape(mut self, shape: impl Into<ShapeBorderRef>) -> SnackBar {
        self.shape = Some(shape.into());
        self
    }

    /// Dart `SnackBar(hitTestBehavior:)`.
    pub fn hit_test_behavior(mut self, hit_test_behavior: HitTestBehavior) -> SnackBar {
        self.hit_test_behavior = Some(hit_test_behavior);
        self
    }

    /// Dart `SnackBar(behavior:)`.
    pub fn behavior(mut self, behavior: SnackBarBehavior) -> SnackBar {
        self.behavior = Some(behavior);
        self
    }

    /// Dart `SnackBar(action:)`.
    pub fn action(mut self, action: SnackBarAction) -> SnackBar {
        self.action = Some(action);
        self
    }

    /// Dart `SnackBar(actionOverflowThreshold:)`.
    pub fn action_overflow_threshold(mut self, action_overflow_threshold: f64) -> SnackBar {
        debug_assert!(
            (0.0..=1.0).contains(&action_overflow_threshold),
            "Action overflow threshold must be between 0 and 1 inclusive"
        );
        self.action_overflow_threshold = Some(action_overflow_threshold);
        self
    }

    /// Dart `SnackBar(showCloseIcon:)`.
    pub fn show_close_icon(mut self, show_close_icon: bool) -> SnackBar {
        self.show_close_icon = Some(show_close_icon);
        self
    }

    /// Dart `SnackBar(closeIconColor:)`.
    pub fn close_icon_color(mut self, close_icon_color: impl Into<AnyColor>) -> SnackBar {
        self.close_icon_color = Some(close_icon_color.into());
        self
    }

    /// Dart `SnackBar(duration:)`.
    pub fn duration(mut self, duration: Duration) -> SnackBar {
        self.duration = duration;
        self
    }

    /// Dart `SnackBar(persist:)`.
    pub fn persist(mut self, persist: bool) -> SnackBar {
        self.persist = Some(persist);
        self
    }

    /// Dart `SnackBar(animation:)`.
    pub fn animation(mut self, animation: AnimationRef<f64>) -> SnackBar {
        self.animation = Some(animation);
        self
    }

    /// Dart `SnackBar(onVisible:)`.
    pub fn on_visible(mut self, on_visible: impl Fn(&mut App) + 'static) -> SnackBar {
        self.on_visible = Some(Rc::new(on_visible));
        self
    }

    /// Dart `SnackBar(dismissDirection:)`.
    pub fn dismiss_direction(mut self, dismiss_direction: DismissDirection) -> SnackBar {
        self.dismiss_direction = Some(dismiss_direction);
        self
    }

    /// Dart `SnackBar(clipBehavior:)`.
    pub fn clip_behavior(mut self, clip_behavior: Clip) -> SnackBar {
        self.clip_behavior = clip_behavior;
        self
    }

    /// Dart's `persist` field, as its constructor fills it: `persist ?? action != null`.
    pub fn effective_persist(&self) -> bool {
        self.persist.unwrap_or(self.action.is_some())
    }

    // API for ScaffoldMessengerState.showSnackBar():

    /// Creates an animation controller useful for driving a snack bar's entrance and exit
    /// animation.
    pub fn create_animation_controller(
        app: &mut App,
        vsync: impl TickerProvider,
        duration: Option<Duration>,
        reverse_duration: Option<Duration>,
    ) -> Ref<AnimationController> {
        AnimationController::new(
            app,
            None,
            Some(duration.unwrap_or(SNACK_BAR_TRANSITION_DURATION)),
            reverse_duration,
            0.0,
            1.0,
            AnimationBehavior::Normal,
            vsync,
        )
    }

    /// Creates a copy of this snack bar but with the animation replaced with the given
    /// animation.
    ///
    /// If the original snack bar lacks a key, the newly created snack bar will
    /// use the given fallback key.
    pub fn with_animation(
        &self,
        new_animation: AnimationRef<f64>,
        fallback_key: Option<KeyRef>,
    ) -> SnackBar {
        SnackBar {
            key: self.key.clone().or(fallback_key),
            content: Rc::clone(&self.content),
            background_color: self.background_color,
            elevation: self.elevation,
            margin: self.margin,
            padding: self.padding,
            width: self.width,
            shape: self.shape.clone(),
            hit_test_behavior: self.hit_test_behavior,
            behavior: self.behavior,
            action: self.action.clone(),
            action_overflow_threshold: self.action_overflow_threshold,
            show_close_icon: self.show_close_icon,
            close_icon_color: self.close_icon_color.clone(),
            duration: self.duration,
            persist: Some(self.effective_persist()),
            animation: Some(new_animation),
            on_visible: self.on_visible.clone(),
            dismiss_direction: self.dismiss_direction,
            clip_behavior: self.clip_behavior,
        }
    }
}

impl fmt::Debug for SnackBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SnackBar")
            .field("key", &self.key)
            .field("content", &self.content)
            .field("behavior", &self.behavior)
            .field("action", &self.action)
            .field("duration", &self.duration)
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for SnackBar {
    type State = SnackBarState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> SnackBarState {
        SnackBarState {
            state: StateData::new(),
            was_visible: false,
            height_animation: None,
            fade_in_animation: None,
            fade_in_m3_animation: None,
            fade_out_animation: None,
            height_m3_animation: None,
            dismissible_key: UniqueKey::new().into(),
        }
    }
}

/// Dart's `_SnackBarState`.
pub struct SnackBarState {
    state: StateData<SnackBar>,
    was_visible: bool,

    height_animation: Option<Ref<CurvedAnimation>>,
    fade_in_animation: Option<Ref<CurvedAnimation>>,
    fade_in_m3_animation: Option<Ref<CurvedAnimation>>,
    fade_out_animation: Option<Ref<CurvedAnimation>>,
    height_m3_animation: Option<Ref<CurvedAnimation>>,

    dismissible_key: KeyRef,
}

impl SnackBarState {
    fn animation(self: Handle<Self>, app: &App) -> AnimationRef<f64> {
        self.widget(app)
            .animation
            .clone()
            .expect("a SnackBar is shown with an animation")
    }

    fn set_animations(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.widget(app).animation.is_some());
        let animation = self.animation(app);
        let height_animation =
            CurvedAnimation::new(app, animation.clone(), snack_bar_height_curve(), None);
        let fade_in_animation =
            CurvedAnimation::new(app, animation.clone(), snack_bar_fade_in_curve(), None);
        let fade_in_m3_animation =
            CurvedAnimation::new(app, animation.clone(), snack_bar_m3_fade_in_curve(), None);
        let fade_out_animation = CurvedAnimation::new(
            app,
            animation.clone(),
            snack_bar_fade_out_curve(),
            Some(Rc::new(Threshold::new(0.0))),
        );
        // Material 3 Animation has a height animation on entry, but a direct fade out on exit.
        let height_m3_animation = CurvedAnimation::new(
            app,
            animation,
            snack_bar_m3_height_curve(),
            Some(Rc::new(Threshold::new(0.0))),
        );
        let this = app.get_mut(self);
        this.height_animation = Some(height_animation);
        this.fade_in_animation = Some(fade_in_animation);
        this.fade_in_m3_animation = Some(fade_in_m3_animation);
        this.fade_out_animation = Some(fade_out_animation);
        this.height_m3_animation = Some(height_m3_animation);
    }

    fn dispose_animations(self: Handle<Self>, app: &mut App) {
        let this = app.get_mut(self);
        let animations = [
            this.height_animation.take(),
            this.fade_in_animation.take(),
            this.fade_in_m3_animation.take(),
            this.fade_out_animation.take(),
            this.height_m3_animation.take(),
        ];
        for animation in animations.into_iter().flatten() {
            animation.dispose(app);
        }
    }

    fn on_animation_status_changed(
        self: Handle<Self>,
        app: &mut App,
        animation_status: AnimationStatus,
    ) {
        if animation_status.is_completed() {
            if let Some(on_visible) = self.widget(app).on_visible.clone()
                && !app.get(self).was_visible
            {
                on_visible(app);
            }
            app.get_mut(self).was_visible = true;
        }
    }

    fn status_listener(self: Handle<Self>) -> AnimationStatusListener {
        AnimationStatusListener::handle_method(self, Self::on_animation_status_changed)
    }
}

impl State for SnackBarState {
    type Widget = SnackBar;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let animation = self.animation(app);
        animation.add_status_listener(app, self.status_listener());
        self.set_animations(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &SnackBar) {
        if self.widget(app).animation != old_widget.animation {
            let old_animation = old_widget
                .animation
                .clone()
                .expect("a SnackBar is shown with an animation");
            old_animation.remove_status_listener(app, &self.status_listener());
            let animation = self.animation(app);
            animation.add_status_listener(app, self.status_listener());
            self.dispose_animations(app);
            self.set_animations(app);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let animation = self.animation(app);
        animation.remove_status_listener(app, &self.status_listener());
        self.dispose_animations(app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        debug_assert!(debug_check_has_media_query(app, context));
        let accessible_navigation = MediaQuery::accessible_navigation_of(app, context);
        debug_assert!(self.widget(app).animation.is_some());
        let theme = Theme::of(app, context);
        let color_scheme = theme.color_scheme.clone();
        let snack_bar_theme = SnackBarTheme::of(app, context);
        let is_theme_dark = theme.brightness() == Brightness::Dark;
        let button_color = if is_theme_dark {
            color_scheme.primary
        } else {
            color_scheme.secondary
        };
        let defaults: Box<dyn SnackBarThemeDataGetters> = if theme.use_material3 {
            Box::new(SnackbarDefaultsM3::new(context))
        } else {
            Box::new(SnackbarDefaultsM2::new(app, context))
        };

        // SnackBar uses a theme that is the opposite brightness from
        // the surrounding theme.
        let brightness = if is_theme_dark {
            Brightness::Light
        } else {
            Brightness::Dark
        };

        // Invert the theme values for Material 2. Material 3 values are tokenized to pre-inverted
        // values.
        let effective_theme = if theme.use_material3 {
            Rc::clone(&theme)
        } else {
            Rc::new(
                theme
                    .copy_with()
                    .color_scheme(inverted_color_scheme(
                        &color_scheme,
                        button_color,
                        defaults.background_color(app),
                        brightness,
                    ))
                    .build(),
            )
        };

        let widget = self.widget(app).clone();
        let content_text_style = snack_bar_theme
            .content_text_style
            .clone()
            .or_else(|| defaults.content_text_style(app));
        let snack_bar_behavior = widget
            .behavior
            .or(snack_bar_theme.behavior)
            .or_else(|| defaults.behavior(app))
            .expect("the defaults have a behavior");
        let width = widget.width.or(snack_bar_theme.width);
        if cfg!(debug_assertions) {
            // Whether the behavior is set through the constructor or the theme,
            // assert that other properties are configured properly.
            if snack_bar_behavior != SnackBarBehavior::Floating {
                let message = |parameter: &str| {
                    let prefix = format!("{parameter} can only be used with floating behavior.");
                    if widget.behavior.is_some() {
                        format!(
                            "{prefix} SnackBarBehavior.fixed was set in the SnackBar constructor."
                        )
                    } else if snack_bar_theme.behavior.is_some() {
                        format!(
                            "{prefix} SnackBarBehavior.fixed was set by the inherited \
                             SnackBarThemeData."
                        )
                    } else {
                        format!("{prefix} SnackBarBehavior.fixed was set by default.")
                    }
                };
                assert!(widget.margin.is_none(), "{}", message("Margin"));
                assert!(width.is_none(), "{}", message("Width"));
            }
        }

        let show_close_icon = widget
            .show_close_icon
            .or(snack_bar_theme.show_close_icon)
            .or_else(|| defaults.show_close_icon(app))
            .expect("the defaults have a show close icon");

        let is_floating_snack_bar = snack_bar_behavior == SnackBarBehavior::Floating;
        let horizontal_padding = if is_floating_snack_bar { 16.0 } else { 24.0 };
        let padding = widget.padding.unwrap_or(EdgeInsetsGeometry::directional(
            horizontal_padding,
            0.0,
            if widget.action.is_some() || show_close_icon {
                0.0
            } else {
                horizontal_padding
            },
            0.0,
        ));

        let action_horizontal_margin = widget.padding.map_or(horizontal_padding, |padding| {
            padding.resolve(Some(TextDirection::Ltr)).right
        }) / 2.0;
        let icon_horizontal_margin = widget.padding.map_or(horizontal_padding, |padding| {
            padding.resolve(Some(TextDirection::Ltr)).right
        }) / 12.0;

        let icon_button = if show_close_icon {
            let close_icon_color = widget
                .close_icon_color
                .clone()
                .or(snack_bar_theme.close_icon_color.clone())
                .or_else(|| defaults.close_icon_color(app));
            let tooltip = <dyn MaterialLocalizations>::of(app, context).close_button_tooltip();
            let mut icon_button = IconButton::new(
                Some(Listener::new(move |app| {
                    ScaffoldMessenger::of(app, context)
                        .hide_current_snack_bar(app, SnackBarClosedReason::Dismiss);
                })),
                Icon::new(Some(Icons::close())),
            )
            .key(StandardComponentType::CloseButton.key())
            .icon_size(24.0)
            .tooltip(tooltip);
            if let Some(close_icon_color) = close_icon_color {
                icon_button = icon_button.color(close_icon_color.color());
            }
            Some(icon_button)
        } else {
            None
        };

        // Calculate combined width of Action, Icon, and their padding, if they are present.
        let mut action_text_span = TextSpan::new().text(
            widget
                .action
                .as_ref()
                .map(|action| action.label.clone())
                .unwrap_or_default(),
        );
        if let Some(label_large) = Theme::of(app, context).text_theme.label_large.clone() {
            action_text_span = action_text_span.style(label_large);
        }
        let mut action_text_painter = TextPainter::new();
        action_text_painter.set_text(Some(action_text_span.into_span()));
        action_text_painter.set_max_lines(Some(1));
        action_text_painter.set_text_direction(Some(TextDirection::Ltr));
        action_text_painter.set_text_scaler(MediaQuery::text_scaler_of(app, context));
        let fonts = PaintingBinding::instance(app).fonts(app);
        action_text_painter.layout(app.get_mut(fonts), 0.0, f64::INFINITY);
        let action_and_icon_width = action_text_painter.size().width()
            + if widget.action.is_some() {
                action_horizontal_margin
            } else {
                0.0
            }
            + if show_close_icon {
                icon_button
                    .as_ref()
                    .and_then(|icon_button| icon_button.icon_size)
                    .unwrap_or(0.0 + icon_horizontal_margin)
            } else {
                0.0
            };
        action_text_painter.dispose();

        let margin: EdgeInsets = widget
            .margin
            .map(|margin| margin.resolve(Some(TextDirection::Ltr)))
            .or(snack_bar_theme.inset_padding)
            .or_else(|| defaults.inset_padding(app))
            .expect("the defaults have an inset padding");

        let snack_bar_width = widget
            .width
            .unwrap_or_else(|| MediaQuery::width_of(app, context) - (margin.left + margin.right));
        let action_overflow_threshold = widget
            .action_overflow_threshold
            .or(snack_bar_theme.action_overflow_threshold)
            .or_else(|| defaults.action_overflow_threshold(app))
            .expect("the defaults have an action overflow threshold");

        let will_overflow_action =
            action_and_icon_width / snack_bar_width > action_overflow_threshold;

        let mut maybe_action_and_icon: Vec<WidgetRef> = Vec::new();
        if let Some(action) = &widget.action {
            maybe_action_and_icon.push(
                Padding::new(EdgeInsetsGeometry::symmetric(0.0, action_horizontal_margin))
                    .child(TextButtonTheme::new(
                        TextButtonThemeData::new().style(
                            TextButton::style_from()
                                .foreground_color(button_color)
                                .padding(EdgeInsetsGeometry::symmetric(0.0, horizontal_padding))
                                .build(),
                        ),
                        action.clone(),
                    ))
                    .into_widget(),
            );
        }
        if let Some(icon_button) = icon_button {
            maybe_action_and_icon.push(
                Padding::new(EdgeInsetsGeometry::symmetric(0.0, icon_horizontal_margin))
                    .child(icon_button)
                    .into_widget(),
            );
        }

        let mut row_children: Vec<WidgetRef> = vec![
            Expanded::new(
                Padding::new(if widget.padding.is_none() {
                    EdgeInsetsGeometry::symmetric(SINGLE_LINE_VERTICAL_PADDING, 0.0)
                } else {
                    EdgeInsetsGeometry::ZERO
                })
                .child(DefaultTextStyle::new(
                    content_text_style.expect("the defaults have a content text style"),
                    Rc::clone(&widget.content),
                )),
            )
            .into_widget(),
        ];
        if !will_overflow_action {
            row_children.extend(maybe_action_and_icon.iter().cloned());
        }
        if will_overflow_action && widget.padding.is_none() {
            row_children.push(SizedBox::new().width(horizontal_padding).into_widget());
        }
        let mut wrap_children: Vec<WidgetRef> =
            vec![Row::new().children(row_children).into_widget()];
        if will_overflow_action {
            wrap_children.push(
                Padding::new(EdgeInsetsGeometry::only(
                    0.0,
                    0.0,
                    0.0,
                    SINGLE_LINE_VERTICAL_PADDING,
                ))
                .child(
                    Row::new()
                        .main_axis_alignment(MainAxisAlignment::End)
                        .children(maybe_action_and_icon),
                )
                .into_widget(),
            );
        }
        let mut snack_bar: WidgetRef = Padding::new(padding)
            .child(Wrap::new().children(wrap_children))
            .into_widget();

        if !is_floating_snack_bar {
            snack_bar = SafeArea::new(snack_bar).top(false).into_widget();
        }

        let elevation = widget
            .elevation
            .or(snack_bar_theme.elevation)
            .or_else(|| defaults.elevation(app))
            .expect("the defaults have an elevation");
        let background_color = widget
            .background_color
            .or(snack_bar_theme.background_color)
            .or_else(|| defaults.background_color(app))
            .expect("the defaults have a background color");
        let shape = widget
            .shape
            .clone()
            .or(snack_bar_theme.shape.clone())
            .or_else(|| {
                if is_floating_snack_bar {
                    defaults.shape(app)
                } else {
                    None
                }
            });
        let dismiss_direction = widget
            .dismiss_direction
            .or(snack_bar_theme.dismiss_direction)
            .unwrap_or(DismissDirection::Down);

        let themed_child = if accessible_navigation || theme.use_material3 {
            snack_bar
        } else {
            let fade_out_animation = app
                .get(self)
                .fade_out_animation
                .clone()
                .expect("set by set_animations");
            FadeTransition::new(fade_out_animation.erase())
                .child(snack_bar)
                .into_widget()
        };
        let mut material = Material::new()
            .elevation(elevation)
            .color(background_color)
            .clip_behavior(widget.clip_behavior);
        if let Some(shape) = shape {
            material = material.shape(shape);
        }
        snack_bar = material
            .child(Theme::new(effective_theme, themed_child))
            .into_widget();

        if is_floating_snack_bar {
            // If width is provided, do not include horizontal margins.
            if let Some(width) = width {
                snack_bar = Padding::new(EdgeInsetsGeometry::only(
                    0.0,
                    margin.top,
                    0.0,
                    margin.bottom,
                ))
                .child(SizedBox::new().width(width).child(snack_bar))
                .into_widget();
            } else {
                snack_bar = Padding::new(EdgeInsetsGeometry::Insets(margin))
                    .child(snack_bar)
                    .into_widget();
            }
            snack_bar = SafeArea::new(snack_bar)
                .top(false)
                .bottom(false)
                .into_widget();
        }

        let dismissible_key = app.get(self).dismissible_key.clone();
        let dismissible = Dismissible::new(dismissible_key, snack_bar)
            .direction(dismiss_direction)
            .resize_duration(None)
            .behavior(widget.hit_test_behavior.unwrap_or(
                if widget.margin.is_some() || snack_bar_theme.inset_padding.is_some() {
                    HitTestBehavior::DeferToChild
                } else {
                    HitTestBehavior::Opaque
                },
            ))
            .on_dismissed(move |app, _direction| {
                ScaffoldMessenger::of(app, context)
                    .remove_current_snack_bar(app, SnackBarClosedReason::Swipe);
            });
        let snack_bar = Semantics::new()
            .container(true)
            .live_region(true)
            .on_dismiss(Rc::new(move |app: &mut App| {
                ScaffoldMessenger::of(app, context)
                    .remove_current_snack_bar(app, SnackBarClosedReason::Dismiss);
            }))
            .child(dismissible)
            .into_widget();

        let snack_bar_transition: WidgetRef = if accessible_navigation {
            snack_bar
        } else if is_floating_snack_bar && !theme.use_material3 {
            let fade_in_animation = app
                .get(self)
                .fade_in_animation
                .clone()
                .expect("set by set_animations");
            FadeTransition::new(fade_in_animation.erase())
                .child(snack_bar)
                .into_widget()
            // Is Material 3 Floating Snack Bar.
        } else if is_floating_snack_bar && theme.use_material3 {
            let this = app.get(self);
            let fade_in_m3_animation = this
                .fade_in_m3_animation
                .clone()
                .expect("set by set_animations");
            let height_m3_animation = this
                .height_m3_animation
                .clone()
                .expect("set by set_animations");
            FadeTransition::new(fade_in_m3_animation.erase())
                .child(
                    height_factor_builder(height_m3_animation, AlignmentGeometry::BOTTOM_LEFT)
                        .child(snack_bar),
                )
                .into_widget()
        } else {
            let height_animation = app
                .get(self)
                .height_animation
                .clone()
                .expect("set by set_animations");
            height_factor_builder(height_animation, AlignmentGeometry::TOP_START)
                .child(snack_bar)
                .into_widget()
        };

        Hero::new(
            Rc::new(format!("<SnackBar Hero tag - {:?}>", widget.content)),
            ClipRect::new()
                .clip_behavior(widget.clip_behavior)
                .child(snack_bar_transition),
        )
        .transition_on_user_gestures(true)
        .into_widget()
    }
}

/// Dart's `ValueListenableBuilder<double>(valueListenable: animation, builder: ...)` that
/// aligns the child at `alignment` with the animation's value as its height factor.
fn height_factor_builder(
    animation: Ref<CurvedAnimation>,
    alignment: AlignmentGeometry,
) -> ListenableBuilder {
    let listenable: Rc<dyn Listenable> = Rc::new(animation.erase::<AnyAnimation<f64>>());
    ListenableBuilder::new(listenable, move |app, _context, child| {
        let mut align = Align::new()
            .alignment(alignment)
            .height_factor(animation.value(app));
        if let Some(child) = child {
            align = align.child(Rc::clone(child));
        }
        align.into_widget()
    })
}

/// The Material 2 snack bar's color scheme: the theme's, inverted.
#[allow(deprecated)]
fn inverted_color_scheme(
    color_scheme: &ColorScheme,
    button_color: Color,
    background: Option<Color>,
    brightness: Brightness,
) -> ColorScheme {
    let mut inverted = ColorScheme::new(
        brightness,
        color_scheme.on_primary,
        color_scheme.primary,
        button_color,
        color_scheme.secondary,
        color_scheme.on_error,
        color_scheme.error,
        color_scheme.on_surface,
        color_scheme.surface,
    )
    .with_on_background(color_scheme.background());
    if let Some(background) = background {
        inverted = inverted.with_background(background);
    }
    inverted
}

/// Hand coded defaults based on Material Design 2; Dart's `_SnackbarDefaultsM2(context)`.
#[allow(deprecated)]
/// Dart's `_SnackbarDefaultsM2`.
struct SnackbarDefaultsM2 {
    /// Dart's `super(..)` arguments.
    snack_bar_theme_data: SnackBarThemeData,
    /// Dart's `_theme`, which the constructor reads.
    theme: Rc<ThemeData>,
}

impl SnackbarDefaultsM2 {
    fn new(app: &mut App, context: BuildContext) -> SnackbarDefaultsM2 {
        SnackbarDefaultsM2 {
            snack_bar_theme_data: SnackBarThemeData::new().elevation(6.0),
            theme: Theme::of(app, context),
        }
    }

    /// Dart's `_colors`.
    fn colors(&self) -> &ColorScheme {
        &self.theme.color_scheme
    }

    fn is_light(&self) -> bool {
        self.theme.brightness() == Brightness::Light
    }
}

#[allow(deprecated)]
impl SnackBarThemeDataGetters for SnackbarDefaultsM2 {
    fn snack_bar_theme_data(&self) -> &SnackBarThemeData {
        &self.snack_bar_theme_data
    }

    fn background_color(&self, _app: &mut App) -> Option<Color> {
        let colors = self.colors();
        Some(if self.is_light() {
            Color::alpha_blend(colors.on_surface.with_opacity(0.80), colors.surface)
        } else {
            colors.on_surface
        })
    }

    fn content_text_style(&self, app: &mut App) -> Option<TextStyle> {
        ThemeData::builder()
            .use_material3(self.theme.use_material3)
            .brightness(if self.is_light() {
                Brightness::Dark
            } else {
                Brightness::Light
            })
            .build(app)
            .text_theme
            .title_medium
    }

    fn behavior(&self, _app: &mut App) -> Option<SnackBarBehavior> {
        Some(SnackBarBehavior::Fixed)
    }

    fn action_text_color(&self, _app: &mut App) -> Option<AnyColor> {
        Some(self.colors().secondary.into())
    }

    fn disabled_action_text_color(&self, _app: &mut App) -> Option<Color> {
        Some(
            self.colors()
                .on_surface
                .with_opacity(if self.is_light() { 0.38 } else { 0.3 }),
        )
    }

    fn shape(&self, _app: &mut App) -> Option<ShapeBorderRef> {
        Some(ShapeBorderRef::new(RoundedRectangleBorder::new(
            BorderSide::NONE,
            BorderRadiusGeometry::all(Radius::circular(4.0)),
        )))
    }

    fn inset_padding(&self, _app: &mut App) -> Option<EdgeInsets> {
        Some(EdgeInsets::from_ltrb(15.0, 5.0, 15.0, 10.0))
    }

    fn show_close_icon(&self, _app: &mut App) -> Option<bool> {
        Some(false)
    }

    fn close_icon_color(&self, _app: &mut App) -> Option<AnyColor> {
        Some(self.colors().on_surface.into())
    }

    fn action_overflow_threshold(&self, _app: &mut App) -> Option<f64> {
        Some(0.25)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use waft_embedder::{Offset, Rect, TargetPlatform};
    use waft_widgets::{Builder, Directionality, GestureDetector, GlobalKey, SizedBox};

    use waft_widgets::testing::{WidgetTester, test_widgets};

    use super::*;
    use crate::scaffold::{Scaffold, ScaffoldFeatureController};
    use crate::test_support::material_app;

    /// Dart's `ThemeData()`.
    fn m3(app: &App) -> ThemeData {
        ThemeData::builder().build(app)
    }

    /// Dart's `ThemeData(useMaterial3: false)`.
    fn m2(app: &App) -> ThemeData {
        ThemeData::builder().use_material3(false).build(app)
    }

    /// Dart's `MaterialApp(theme: theme, home: home)`.
    fn pump_app(tester: &mut WidgetTester, theme: impl FnOnce(&App) -> ThemeData, home: WidgetRef) {
        let theme = theme(&tester.cell.borrow());
        tester.pump_widget(material_app(theme, home));
    }

    /// Dart's `Scaffold(body: Builder(builder: (context) => GestureDetector(key: tapTarget,
    /// onTap: () => show(context), behavior: HitTestBehavior.opaque,
    /// child: SizedBox(height: 100.0, width: 100.0))))`.
    fn tap_target_scaffold(
        tap_target: &GlobalKey,
        show: impl Fn(&mut App, BuildContext) + 'static,
    ) -> Scaffold {
        let tap_target = tap_target.clone();
        let show = Rc::new(show);
        Scaffold::new().body(Builder::new(move |_app, context| {
            let show = Rc::clone(&show);
            GestureDetector::new()
                .key(tap_target.clone().into())
                .on_tap(Listener::new(move |app| show(app, context)))
                .behavior(HitTestBehavior::Opaque)
                .child(SizedBox::new().height(100.0).width(100.0))
                .into_widget()
        }))
    }

    type SnackBarController = ScaffoldFeatureController<SnackBar, SnackBarClosedReason>;

    /// Dart's `ScaffoldMessenger.of(context).showSnackBar(snackBar)`.
    fn show(app: &mut App, context: BuildContext, snack_bar: SnackBar) -> Rc<SnackBarController> {
        ScaffoldMessenger::of(app, context).show_snack_bar(app, snack_bar, None)
    }

    /// Dart's `tester.tap(find.byKey(key))`.
    fn tap_key(tester: &mut WidgetTester, key: &GlobalKey) {
        let center = tester.center_of(key);
        tester.tap(center, 1);
    }

    /// Dart's `find.text(text)`, counted over `Text`s.
    fn count_text(tester: &WidgetTester, text: &str) -> usize {
        tester.find_text(text).len()
    }

    /// Dart's `tester.getRect(find.text(text))`.
    fn text_rect(tester: &WidgetTester, text: &str) -> Rect {
        let element = *tester
            .find_text(text)
            .first()
            .expect("a Text showing the text");
        tester.global_rect(tester.render_box_of(element))
    }

    /// Dart's `tester.tap(find.text(text))`.
    fn tap_text(tester: &mut WidgetTester, text: &str) {
        let center = text_rect(tester, text).center();
        tester.tap(center, 1);
    }

    /// Dart's `tester.drag(find.text(text), offset)`.
    fn drag_text(tester: &mut WidgetTester, text: &str, offset: Offset) {
        let center = text_rect(tester, text).center();
        tester.drag_from(center, offset, 1);
    }

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// Dart's `Scaffold(body: Builder(builder: (context) => GestureDetector(onTap: () =>
    /// show(context), child: const Text('X'))))`.
    fn x_scaffold(show: impl Fn(&mut App, BuildContext) + 'static) -> Scaffold {
        let show = Rc::new(show);
        Scaffold::new().body(Builder::new(move |_app, context| {
            let show = Rc::clone(&show);
            GestureDetector::new()
                .on_tap(Listener::new(move |app| show(app, context)))
                .child(Text::new("X"))
                .into_widget()
        }))
    }

    /// Dart's `tester.tap(find.text('X')); tester.pump(); tester.pump_by(750ms)`: shows the snack
    /// bar and runs its entrance to the end.
    fn tap_x_and_settle_entrance(tester: &mut WidgetTester) {
        tap_text(tester, "X");
        tester.pump(); // start animation
        tester.pump_by(ms(750));
    }

    /// Dart's `find.descendant(of: find.byType(SnackBar), matching: find.byType(Material))`,
    /// first.
    fn snack_bar_material(tester: &WidgetTester) -> waft_widgets::AnyElement {
        let snack_bar = tester.find_by_type::<SnackBar>()[0];
        tester
            .find_by_type::<Material>()
            .into_iter()
            .find(|material| tester.ancestors(*material).contains(&snack_bar))
            .expect("a SnackBar builds a Material")
    }

    /// Dart's `tester.getRect(materialFinder)` of [`snack_bar_material`].
    fn snack_bar_material_rect(tester: &WidgetTester) -> Rect {
        tester.global_rect(tester.render_box_of(snack_bar_material(tester)))
    }

    /// Dart's `tester.renderObject<RenderPhysicalModel>(find.widgetWithText(Material,
    /// text).first).color`: the colour the snack bar's material paints.
    fn snack_bar_color(tester: &WidgetTester) -> Color {
        use waft_rendering::{RenderPhysicalModel, RenderPhysicalModelBase, RenderPhysicalShape};
        let mut render = Some(tester.render_box_of(snack_bar_material(tester)).as_object());
        let app = tester.cell.borrow();
        while let Some(node) = render {
            if let Some(model) = node.downcast::<RenderPhysicalModel>(&app) {
                return model.color(&app);
            }
            if let Some(shape) = node.downcast::<RenderPhysicalShape>(&app) {
                return shape.color(&app);
            }
            let mut child = None;
            node.visit_children(&app, &mut |next| child = child.or(Some(next)));
            render = child;
        }
        panic!("a Material paints with a physical model");
    }

    // Dart: 'SnackBar control test'.
    #[test]
    fn a_snack_bar_shows_on_tap_and_leaves_after_its_duration() {
        test_widgets(|tester| {
            const HELLO_SNACK_BAR: &str = "Hello SnackBar";
            let tap_target = GlobalKey::new();
            let home = tap_target_scaffold(&tap_target, |app, context| {
                show(
                    app,
                    context,
                    SnackBar::new(Text::new(HELLO_SNACK_BAR)).duration(Duration::from_secs(2)),
                );
            });
            pump_app(tester, m3, home.into_widget());
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 0);
            tap_key(tester, &tap_target);
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 0);
            tester.pump(); // schedule animation
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 1);
            tester.pump(); // begin animation
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 1);
            tester.pump_by(ms(750)); // 0.75s // animation last frame; two second timer starts here
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 1);
            tester.pump_by(ms(750)); // 1.50s
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 1);
            tester.pump_by(ms(750)); // 2.25s
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 1);
            tester.pump_by(ms(750)); // 3.00s // timer triggers to dismiss snackbar, reverse animation is scheduled
            tester.pump(); // begin animation
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 1); // frame 0 of dismiss animation
            tester.pump_by(ms(750)); // 3.75s // last frame of animation, snackbar removed from build
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 0);
        });
    }

    // Dart: 'SnackBar twice test'.
    #[test]
    fn a_second_snack_bar_waits_for_the_first_to_leave() {
        test_widgets(|tester| {
            let snack_bar_count = Rc::new(Cell::new(0));
            let tap_target = GlobalKey::new();
            let home = tap_target_scaffold(&tap_target, {
                let snack_bar_count = Rc::clone(&snack_bar_count);
                move |app, context| {
                    snack_bar_count.set(snack_bar_count.get() + 1);
                    show(
                        app,
                        context,
                        SnackBar::new(Text::new(format!("bar{}", snack_bar_count.get())))
                            .duration(Duration::from_secs(2)),
                    );
                }
            });
            pump_app(tester, m3, home.into_widget());
            let expect = |tester: &WidgetTester, bar1: usize, bar2: usize| {
                assert_eq!(count_text(tester, "bar1"), bar1);
                assert_eq!(count_text(tester, "bar2"), bar2);
            };
            expect(tester, 0, 0);
            tap_key(tester, &tap_target); // queue bar1
            tap_key(tester, &tap_target); // queue bar2
            expect(tester, 0, 0);
            tester.pump(); // schedule animation for bar1
            expect(tester, 1, 0);
            tester.pump(); // begin animation
            expect(tester, 1, 0);
            tester.pump_by(ms(750)); // 0.75s // animation last frame; two second timer starts here
            expect(tester, 1, 0);
            tester.pump_by(ms(750)); // 1.50s
            expect(tester, 1, 0);
            tester.pump_by(ms(750)); // 2.25s
            expect(tester, 1, 0);
            tester.pump_by(ms(750)); // 3.00s // timer triggers to dismiss snackbar, reverse animation is scheduled
            tester.pump(); // begin animation
            expect(tester, 1, 0);
            tester.pump_by(ms(750)); // 3.75s // last frame of animation, snackbar removed from build, new snack bar put in its place
            expect(tester, 0, 1);
            tester.pump(); // begin animation
            expect(tester, 0, 1);
            tester.pump_by(ms(750)); // 4.50s // animation last frame; two second timer starts here
            expect(tester, 0, 1);
            tester.pump_by(ms(750)); // 5.25s
            expect(tester, 0, 1);
            tester.pump_by(ms(750)); // 6.00s
            expect(tester, 0, 1);
            tester.pump_by(ms(750)); // 6.75s // timer triggers to dismiss snackbar, reverse animation is scheduled
            tester.pump(); // begin animation
            expect(tester, 0, 1);
            tester.pump_by(ms(750)); // 7.50s // last frame of animation, snackbar removed from build, new snack bar put in its place
            expect(tester, 0, 0);
        });
    }

    // Dart: 'SnackBar cancel test'.
    #[test]
    fn closing_a_controller_hides_its_snack_bar_before_its_timeout() {
        test_widgets(|tester| {
            let snack_bar_count = Rc::new(Cell::new(0));
            let time = Rc::new(Cell::new(0));
            let last_controller: Rc<RefCell<Option<Rc<SnackBarController>>>> =
                Rc::new(RefCell::new(None));
            let tap_target = GlobalKey::new();
            let home = tap_target_scaffold(&tap_target, {
                let snack_bar_count = Rc::clone(&snack_bar_count);
                let time = Rc::clone(&time);
                let last_controller = Rc::clone(&last_controller);
                move |app, context| {
                    snack_bar_count.set(snack_bar_count.get() + 1);
                    let controller = show(
                        app,
                        context,
                        SnackBar::new(Text::new(format!("bar{}", snack_bar_count.get())))
                            .duration(Duration::from_secs(time.get())),
                    );
                    *last_controller.borrow_mut() = Some(controller);
                }
            });
            pump_app(tester, m3, home.into_widget());
            let expect = |tester: &WidgetTester, bar1: usize, bar2: usize| {
                assert_eq!(count_text(tester, "bar1"), bar1);
                assert_eq!(count_text(tester, "bar2"), bar2);
            };
            expect(tester, 0, 0);
            time.set(1000);
            tap_key(tester, &tap_target); // queue bar1
            let first_controller = last_controller.borrow().clone().expect("shown");
            time.set(2);
            tap_key(tester, &tap_target); // queue bar2
            expect(tester, 0, 0);
            tester.pump(); // schedule animation for bar1
            expect(tester, 1, 0);
            tester.pump(); // begin animation
            expect(tester, 1, 0);
            tester.pump_by(ms(750)); // 0.75s // animation last frame; two second timer starts here
            expect(tester, 1, 0);
            tester.pump_by(ms(750)); // 1.50s
            expect(tester, 1, 0);
            tester.pump_by(ms(750)); // 2.25s
            expect(tester, 1, 0);
            tester.pump_by(ms(10000)); // 12.25s
            expect(tester, 1, 0);

            (first_controller.close)(&mut tester.cell.borrow_mut()); // snackbar is manually dismissed

            tester.pump_by(ms(750)); // 13.00s // reverse animation is scheduled
            tester.pump(); // begin animation
            expect(tester, 1, 0);
            tester.pump_by(ms(750)); // 13.75s // last frame of animation, snackbar removed from build, new snack bar put in its place
            expect(tester, 0, 1);
            tester.pump(); // begin animation
            expect(tester, 0, 1);
            tester.pump_by(ms(750)); // 14.50s // animation last frame; two second timer starts here
            expect(tester, 0, 1);
            tester.pump_by(ms(750)); // 15.25s
            expect(tester, 0, 1);
            tester.pump_by(ms(750)); // 16.00s
            expect(tester, 0, 1);
            tester.pump_by(ms(750)); // 16.75s // timer triggers to dismiss snackbar, reverse animation is scheduled
            tester.pump(); // begin animation
            expect(tester, 0, 1);
            tester.pump_by(ms(750)); // 17.50s // last frame of animation, snackbar removed from build, new snack bar put in its place
            expect(tester, 0, 0);
        });
    }

    /// Dart's `_getDragGesturesOfDismissDirections`, in `DismissDirection.values` order.
    fn drag_gestures_of_dismiss_directions(
        scaffold_width: f64,
    ) -> Vec<(DismissDirection, Vec<Offset>)> {
        vec![
            (
                DismissDirection::Vertical,
                vec![
                    Offset::new(0.0, 50.0),  // drag to bottom gesture
                    Offset::new(0.0, -50.0), // drag to top gesture
                ],
            ),
            (
                DismissDirection::Horizontal,
                vec![
                    Offset::new(scaffold_width, 0.0),  // drag to right gesture
                    Offset::new(-scaffold_width, 0.0), // drag to left gesture
                ],
            ),
            (
                DismissDirection::EndToStart,
                vec![Offset::new(-scaffold_width, 0.0)], // drag to left gesture
            ),
            (
                DismissDirection::StartToEnd,
                vec![Offset::new(scaffold_width, 0.0)], // drag to right gesture
            ),
            (DismissDirection::Up, vec![Offset::new(0.0, -50.0)]), // drag to top gesture
            (DismissDirection::Down, vec![Offset::new(0.0, 50.0)]), // drag to bottom gesture
        ]
    }

    // Dart: 'SnackBar dismiss test', with `_testSnackBarDismiss`.
    #[test]
    fn a_snack_bar_is_dismissed_by_a_drag_in_each_of_its_directions() {
        test_widgets(|tester| {
            let tap_target = GlobalKey::new();
            let dismiss_direction = Rc::new(Cell::new(DismissDirection::Down));
            let width = Rc::new(Cell::new(0.0));
            let snack_bar_count = Rc::new(Cell::new(0));
            let home = Scaffold::new().body(Builder::new({
                let tap_target = tap_target.clone();
                let dismiss_direction = Rc::clone(&dismiss_direction);
                let width = Rc::clone(&width);
                let snack_bar_count = Rc::clone(&snack_bar_count);
                move |app, context| {
                    width.set(MediaQuery::size_of(app, context).width());
                    let dismiss_direction = Rc::clone(&dismiss_direction);
                    let snack_bar_count = Rc::clone(&snack_bar_count);
                    GestureDetector::new()
                        .key(tap_target.clone().into())
                        .on_tap(Listener::new(move |app| {
                            snack_bar_count.set(snack_bar_count.get() + 1);
                            show(
                                app,
                                context,
                                SnackBar::new(Text::new(format!("bar{}", snack_bar_count.get())))
                                    .duration(Duration::from_secs(2))
                                    .dismiss_direction(dismiss_direction.get()),
                            );
                        }))
                        .behavior(HitTestBehavior::Opaque)
                        .child(SizedBox::new().height(100.0).width(100.0))
                        .into_widget()
                }
            }));
            pump_app(tester, m3, home.into_widget());

            for (direction, gestures) in drag_gestures_of_dismiss_directions(width.get()) {
                dismiss_direction.set(direction);
                for drag_gesture in gestures {
                    snack_bar_count.set(0);
                    let expect = |tester: &WidgetTester, bar1: usize, bar2: usize| {
                        assert_eq!(
                            count_text(tester, "bar1"),
                            bar1,
                            "{direction:?} {drag_gesture:?}"
                        );
                        assert_eq!(
                            count_text(tester, "bar2"),
                            bar2,
                            "{direction:?} {drag_gesture:?}"
                        );
                    };
                    expect(tester, 0, 0);
                    tap_key(tester, &tap_target); // queue bar1
                    tap_key(tester, &tap_target); // queue bar2
                    expect(tester, 0, 0);
                    tester.pump(); // schedule animation for bar1
                    expect(tester, 1, 0);
                    tester.pump(); // begin animation
                    expect(tester, 1, 0);
                    tester.pump_by(ms(750)); // 0.75s // animation last frame; two second timer starts here
                    drag_text(tester, "bar1", drag_gesture);
                    tester.pump(); // bar1 dismissed, bar2 begins animating
                    expect(tester, 0, 1);
                    tester.pump_by(ms(750)); // 0.75s // animation last frame; two second timer starts here
                    drag_text(tester, "bar2", drag_gesture);
                    tester.pump(); // bar2 dismissed
                    expect(tester, 0, 0);
                }
            }
        });
    }

    /// The two dismiss-direction cases: a scaffold whose tap target shows `snack_bar`, under
    /// a theme whose snack bars dismiss from start to end; a drag by `drag` dismisses it.
    fn dismisses_by_dragging(
        tester: &mut WidgetTester,
        snack_bar: impl Fn() -> SnackBar + 'static,
        label: &str,
        drag: f64,
    ) {
        let tap_target = GlobalKey::new();
        let home = tap_target_scaffold(&tap_target, move |app, context| {
            show(app, context, snack_bar());
        });
        pump_app(
            tester,
            |app| {
                ThemeData::builder()
                    .snack_bar_theme(
                        SnackBarThemeData::new().dismiss_direction(DismissDirection::StartToEnd),
                    )
                    .build(app)
            },
            home.into_widget(),
        );
        assert_eq!(count_text(tester, label), 0);
        tap_key(tester, &tap_target);
        assert_eq!(count_text(tester, label), 0);
        tester.pump(); // schedule animation for snack bar
        assert_eq!(count_text(tester, label), 1);
        tester.pump(); // begin animation
        assert_eq!(count_text(tester, label), 1);
        tester.pump_by(ms(750));
        drag_text(tester, label, Offset::new(drag * 800.0, 0.0));
        tester.pump(); // snack bar dismissed
        assert_eq!(count_text(tester, label), 0);
    }

    // Dart: 'SnackBar dismissDirection can be customised from SnackBarThemeData'.
    #[test]
    fn the_theme_sets_the_dismiss_direction() {
        test_widgets(|tester| {
            dismisses_by_dragging(
                tester,
                || SnackBar::new(Text::new("swipe ltr")).duration(Duration::from_secs(2)),
                "swipe ltr",
                1.0,
            );
        });
    }

    // Dart: 'dismissDirection from SnackBar should be preferred over SnackBarThemeData'.
    #[test]
    fn the_snack_bar_s_dismiss_direction_wins_over_the_theme_s() {
        test_widgets(|tester| {
            dismisses_by_dragging(
                tester,
                || {
                    SnackBar::new(Text::new("swipe rtl"))
                        .duration(Duration::from_secs(2))
                        .dismiss_direction(DismissDirection::EndToStart)
                },
                "swipe rtl",
                -1.0,
            );
        });
    }

    /// Dart's `SnackBar(content: Text('I am a snack bar.'), duration: 2s,
    /// action: SnackBarAction(label: 'ACTION', onPressed: () {}))`.
    fn snack_bar_with_action() -> SnackBar {
        SnackBar::new(Text::new("I am a snack bar."))
            .duration(Duration::from_secs(2))
            .action(SnackBarAction::new("ACTION", |_app| {}))
    }

    // Dart: 'SnackBar cannot be tapped twice'.
    #[test]
    fn a_snack_bar_action_fires_only_once() {
        test_widgets(|tester| {
            let tap_count = Rc::new(Cell::new(0));
            let home = x_scaffold({
                let tap_count = Rc::clone(&tap_count);
                move |app, context| {
                    let tap_count = Rc::clone(&tap_count);
                    show(
                        app,
                        context,
                        SnackBar::new(Text::new("I am a snack bar."))
                            .duration(Duration::from_secs(2))
                            .action(SnackBarAction::new("ACTION", move |_app| {
                                tap_count.set(tap_count.get() + 1);
                            })),
                    );
                }
            });
            pump_app(tester, m3, home.into_widget());
            tap_x_and_settle_entrance(tester);

            assert_eq!(tap_count.get(), 0);
            tap_text(tester, "ACTION");
            assert_eq!(tap_count.get(), 1);
            tap_text(tester, "ACTION");
            assert_eq!(tap_count.get(), 1);
            tester.pump();
            tap_text(tester, "ACTION");
            assert_eq!(tap_count.get(), 1);
        });
    }

    /// Shows [`snack_bar_with_action`] under `theme` and returns the tester with it entered.
    fn with_action_under(tester: &mut WidgetTester, theme: impl FnOnce(&App) -> ThemeData) {
        let home = x_scaffold(|app, context| {
            show(app, context, snack_bar_with_action());
        });
        pump_app(tester, theme, home.into_widget());
        tap_x_and_settle_entrance(tester);
    }

    // Dart: 'Material2 - Light theme SnackBar has dark background'.
    #[test]
    fn a_material_2_light_theme_snack_bar_has_a_dark_background() {
        test_widgets(|tester| {
            with_action_under(tester, |app| ThemeData::light(app, Some(false)));
            // There is a somewhat complicated background color calculation based
            // off of the surface color. For the default light theme it
            // should be this value.
            assert!(crate::test_support::is_same_color_as(
                snack_bar_color(tester),
                Color::new(0xFF333333)
            ));
        });
    }

    // Dart: 'Material3 - Light theme SnackBar has dark background'.
    #[test]
    fn a_material_3_light_theme_snack_bar_has_the_inverse_surface_background() {
        test_widgets(|tester| {
            with_action_under(tester, m3);
            let inverse_surface = m3(&tester.cell.borrow()).color_scheme.inverse_surface();
            assert_eq!(snack_bar_color(tester), inverse_surface);
        });
    }

    // Dart: 'Dark theme SnackBar has light background'.
    #[test]
    fn a_dark_theme_snack_bar_has_a_light_background() {
        test_widgets(|tester| {
            with_action_under(tester, |app| ThemeData::dark(app, None));
            let on_surface = ThemeData::dark(&tester.cell.borrow(), None)
                .color_scheme
                .on_surface;
            assert_eq!(snack_bar_color(tester), on_surface);
        });
    }

    // Dart: 'Material2 - Dark theme SnackBar has primary text buttons'.
    #[test]
    fn a_material_2_dark_theme_snack_bar_has_primary_text_buttons() {
        test_widgets(|tester| {
            with_action_under(tester, |app| ThemeData::dark(app, Some(false)));
            let primary = ThemeData::dark(&tester.cell.borrow(), Some(false))
                .color_scheme
                .primary;
            assert_eq!(tester.text_color("ACTION"), primary);
        });
    }

    // Dart: 'Material3 - Dark theme SnackBar has primary text buttons'.
    #[test]
    fn a_material_3_dark_theme_snack_bar_has_inverse_primary_text_buttons() {
        test_widgets(|tester| {
            with_action_under(tester, |app| ThemeData::dark(app, None));
            let inverse_primary = ThemeData::dark(&tester.cell.borrow(), None)
                .color_scheme
                .inverse_primary();
            assert_eq!(tester.text_color("ACTION"), inverse_primary);
        });
    }

    // Dart: 'SnackBar should inherit theme data from its ancestor'.
    #[test]
    fn a_snack_bar_inherits_its_ancestor_s_theme_but_for_the_colour_scheme() {
        test_widgets(|tester| {
            let theme_before_snack_bar: Rc<RefCell<Option<Rc<ThemeData>>>> = Rc::default();
            let theme_after_snack_bar: Rc<RefCell<Option<Rc<ThemeData>>>> = Rc::default();
            let home = Scaffold::new().body(Builder::new({
                let theme_before_snack_bar = Rc::clone(&theme_before_snack_bar);
                let theme_after_snack_bar = Rc::clone(&theme_after_snack_bar);
                move |app, context| {
                    *theme_before_snack_bar.borrow_mut() = Some(Theme::of(app, context));
                    let theme_after_snack_bar = Rc::clone(&theme_after_snack_bar);
                    GestureDetector::new()
                        .on_tap(Listener::new(move |app| {
                            let theme_after_snack_bar = Rc::clone(&theme_after_snack_bar);
                            show(
                                app,
                                context,
                                SnackBar::new(Builder::new(move |app, context| {
                                    *theme_after_snack_bar.borrow_mut() =
                                        Some(Theme::of(app, context));
                                    Text::new("I am a snack bar.").into_widget()
                                }))
                                .duration(Duration::from_secs(2))
                                .action(SnackBarAction::new("ACTION", |_app| {})),
                            );
                        }))
                        .child(Text::new("X"))
                        .into_widget()
                }
            }));
            pump_app(
                tester,
                |app| {
                    ThemeData::builder()
                        .color_scheme(
                            ColorScheme::from_seed(Colors::GREEN.primary())
                                .brightness(Brightness::Dark)
                                .build(),
                        )
                        .build(app)
                },
                home.into_widget(),
            );
            tap_x_and_settle_entrance(tester);

            let theme_after_snack_bar = theme_after_snack_bar.borrow().clone().expect("built");
            let compared_theme = theme_before_snack_bar
                .borrow()
                .as_ref()
                .expect("built")
                .copy_with()
                .color_scheme(theme_after_snack_bar.color_scheme.clone())
                .build(); // Fields replaced by SnackBar.
            assert!(compared_theme == *theme_after_snack_bar);
        });
    }

    /// Shows `snack_bar` under `theme` from the 'X' scaffold and returns the tester with it
    /// entered.
    fn showing(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        snack_bar: impl Fn() -> SnackBar + 'static,
    ) {
        let home = x_scaffold(move |app, context| {
            show(app, context, snack_bar());
        });
        pump_app(tester, theme, home.into_widget());
        tap_x_and_settle_entrance(tester);
    }

    // Dart: 'Snackbar margin can be customized'.
    #[test]
    fn a_floating_snack_bar_s_margin_can_be_customized() {
        test_widgets(|tester| {
            const PADDING: f64 = 20.0;
            showing(tester, m3, || {
                SnackBar::new(Text::new("I am a snack bar."))
                    .margin(EdgeInsets::all(PADDING))
                    .behavior(SnackBarBehavior::Floating)
            });
            let material = snack_bar_material_rect(tester);
            assert_eq!(material.bottom_left().dx(), PADDING);
            assert_eq!(material.bottom_left().dy(), 600.0 - PADDING); // Device height is 600.
            assert_eq!(material.bottom_right().dx(), 800.0 - PADDING); // Device width is 800.
        });
    }

    // Dart: 'SnackbarBehavior.floating is positioned within safe area'.
    #[test]
    fn a_floating_snack_bar_is_positioned_within_the_safe_area() {
        test_widgets(|tester| {
            const VIEW_PADDING: f64 = 50.0;
            const FLOATING_SNACK_BAR_DEFAULT_BOTTOM_MARGIN: f64 = 10.0;
            let home = waft_widgets::MediaQuery::new(
                // Simulate non-safe area.
                waft_widgets::MediaQueryData::new().view_padding(EdgeInsets::only(
                    0.0,
                    0.0,
                    0.0,
                    VIEW_PADDING,
                )),
                x_scaffold(|app, context| {
                    show(
                        app,
                        context,
                        SnackBar::new(Text::new("I am a snack bar."))
                            .behavior(SnackBarBehavior::Floating),
                    );
                }),
            );
            pump_app(tester, m3, home.into_widget());
            tap_x_and_settle_entrance(tester);
            assert_eq!(
                snack_bar_material_rect(tester).bottom_left().dy(),
                // Device height is 600.
                600.0 - VIEW_PADDING - FLOATING_SNACK_BAR_DEFAULT_BOTTOM_MARGIN
            );
        });
    }

    // Dart: 'Snackbar padding can be customized'.
    #[test]
    fn a_snack_bar_s_padding_can_be_customized() {
        test_widgets(|tester| {
            const PADDING: f64 = 20.0;
            showing(tester, m3, || {
                SnackBar::new(Text::new("I am a snack bar.")).padding(EdgeInsets::all(PADDING))
            });
            let text = text_rect(tester, "I am a snack bar.");
            let material = snack_bar_material_rect(tester);
            assert_eq!(
                text.bottom_left().dx() - material.bottom_left().dx(),
                PADDING
            );
            assert_eq!(material.top_right().dx() - text.top_right().dx(), PADDING);
            assert_eq!(
                material.bottom_left().dy() - text.bottom_left().dy(),
                PADDING
            );
            assert_eq!(text.top_right().dy() - material.top_right().dy(), PADDING);
        });
    }

    /// Asserts the snack bar's material is `width` wide, centred in the 800 wide view.
    fn assert_centred_with_width(tester: &WidgetTester, width: f64) {
        let material = snack_bar_material_rect(tester);
        assert_eq!(material.bottom_left().dx(), (800.0 - width) / 2.0); // Device width is 800.
        assert_eq!(material.bottom_right().dx(), (800.0 + width) / 2.0); // Device width is 800.
    }

    // Dart: 'Snackbar width can be customized'.
    #[test]
    fn a_floating_snack_bar_s_width_can_be_customized() {
        test_widgets(|tester| {
            const WIDTH: f64 = 200.0;
            showing(tester, m3, || {
                SnackBar::new(Text::new("I am a snack bar."))
                    .width(WIDTH)
                    .behavior(SnackBarBehavior::Floating)
            });
            assert_centred_with_width(tester, WIDTH);
        });
    }

    // Dart: 'Snackbar width can be customized from ThemeData'.
    #[test]
    fn a_floating_snack_bar_s_width_can_come_from_the_theme() {
        test_widgets(|tester| {
            const WIDTH: f64 = 200.0;
            showing(
                tester,
                |app| {
                    ThemeData::builder()
                        .snack_bar_theme(
                            SnackBarThemeData::new()
                                .width(WIDTH)
                                .behavior(SnackBarBehavior::Floating),
                        )
                        .build(app)
                },
                || SnackBar::new(Text::new("Feeling snackish")),
            );
            assert_centred_with_width(tester, WIDTH);
        });
    }

    // Dart: 'Snackbar width customization takes preference of widget over theme'.
    #[test]
    fn the_snack_bar_s_width_wins_over_the_theme_s() {
        test_widgets(|tester| {
            const THEME_WIDTH: f64 = 200.0;
            const WIDGET_WIDTH: f64 = 400.0;
            showing(
                tester,
                |app| {
                    ThemeData::builder()
                        .snack_bar_theme(
                            SnackBarThemeData::new()
                                .width(THEME_WIDTH)
                                .behavior(SnackBarBehavior::Floating),
                        )
                        .build(app)
                },
                || SnackBar::new(Text::new("Feeling super snackish")).width(WIDGET_WIDTH),
            );
            assert_centred_with_width(tester, WIDGET_WIDTH);
        });
    }

    /// Dart's `tester.widget<TextButton>(find.widgetWithText(TextButton, 'ACTION')).style!`.
    fn action_button_style(tester: &WidgetTester) -> crate::button_style::ButtonStyle {
        use crate::button_style_button::ButtonStyleButton;
        let action = tester.find_text("ACTION")[0];
        let button = tester.ancestor_of_type::<TextButton>(action);
        tester.widget(button, |button: &TextButton| {
            button
                .button_style_button_data()
                .style
                .clone()
                .expect("the action's style")
        })
    }

    fn states(states: &[WidgetState]) -> waft_widgets::WidgetStates {
        states.iter().copied().collect()
    }

    // Dart: 'Material2 - Snackbar labels can be colored as MaterialColor'.
    #[test]
    fn a_material_2_snack_bar_action_label_takes_its_text_color() {
        test_widgets(|tester| {
            showing(tester, m2, || {
                SnackBar::new(Text::new("I am a snack bar."))
                    .duration(Duration::from_secs(2))
                    .action(
                        SnackBarAction::new("ACTION", |_app| {})
                            .text_color(Colors::LIGHT_BLUE.primary())
                            .disabled_text_color(Colors::RED.primary()),
                    )
            });
            assert_eq!(tester.text_color("ACTION"), Colors::LIGHT_BLUE.primary());
        });
    }

    // Dart: 'Material3 - Snackbar labels can be colored as MaterialColor'.
    #[test]
    fn a_material_3_snack_bar_action_resolves_its_text_color_in_every_state() {
        test_widgets(|tester| {
            let used_color = Colors::TEAL.primary();
            showing(tester, m3, move || {
                SnackBar::new(Text::new("I am a snack bar."))
                    .duration(Duration::from_secs(2))
                    .action(SnackBarAction::new("ACTION", |_app| {}).text_color(used_color))
            });
            let style = action_button_style(tester);
            // Same color when resolved
            assert_eq!(
                style
                    .foreground_color
                    .expect("a foreground color")
                    .resolve(&states(&[])),
                Some(used_color)
            );
        });
    }

    // Dart: 'Snackbar labels can be colored as WidgetStateColor (Material 3)'.
    #[test]
    fn a_widget_state_text_color_resolves_the_action_label_by_state() {
        test_widgets(|tester| {
            const COLOR_RED: u32 = 0xFFF44336;
            const COLOR_BLUE: u32 = 0xFF2196F3;
            showing(tester, m3, || {
                // Dart's `_TestMaterialStateColor`.
                let used_color = WidgetStateColor::resolve_with(|states| {
                    if states.contains(&WidgetState::Pressed) {
                        return Color::new(COLOR_BLUE);
                    }
                    Color::new(COLOR_RED)
                });
                SnackBar::new(Text::new("I am a snack bar."))
                    .duration(Duration::from_secs(2))
                    .action(SnackBarAction::new("ACTION", |_app| {}).text_color(used_color))
            });
            let foreground_color = action_button_style(tester)
                .foreground_color
                .expect("a foreground color");
            assert_eq!(
                foreground_color.resolve(&states(&[WidgetState::Pressed])),
                Some(Color::new(COLOR_BLUE))
            );
            assert_eq!(
                foreground_color.resolve(&states(&[])),
                Some(Color::new(COLOR_RED))
            );
        });
    }

    // Dart: 'SnackBar should push FloatingActionButton above'.
    #[test]
    fn a_snack_bar_pushes_the_floating_action_button_above_it() {
        test_widgets(|tester| {
            let home = waft_widgets::MediaQuery::new(
                waft_widgets::MediaQueryData::new()
                    .padding(EdgeInsets::only(10.0, 20.0, 30.0, 40.0)),
                x_scaffold(|app, context| {
                    show(app, context, snack_bar_with_action());
                })
                .floating_action_button(
                    crate::floating_action_button::FloatingActionButton::new(Some(Listener::new(
                        |_app| {},
                    )))
                    .child(Icon::new(Some(Icons::send()))),
                ),
            );
            pump_app(tester, m3, home.into_widget());
            let fab_rect = |tester: &WidgetTester| {
                let fab =
                    tester.find_by_type::<crate::floating_action_button::FloatingActionButton>()[0];
                tester.global_rect(tester.render_box_of(fab))
            };

            // Get the Rect of the FAB to compare after the SnackBar appears.
            let original_fab_rect = fab_rect(tester);

            tap_x_and_settle_entrance(tester); // Animation last frame.

            let fab_rect = fab_rect(tester);

            // FAB should shift upwards after SnackBar appears.
            assert!(fab_rect.center().dy() < original_fab_rect.center().dy());

            let snack_bar = tester.find_by_type::<SnackBar>()[0];
            let snack_bar_top_right = tester
                .global_rect(tester.render_box_of(snack_bar))
                .top_right();

            // FAB's surrounding padding is set to [kFloatingActionButtonMargin] in floating_action_button_location.dart by default.
            const DEFAULT_FAB_PADDING: f64 = 16.0;

            // FAB should be positioned above the SnackBar by the default padding.
            assert_eq!(
                fab_rect.bottom_right().dy(),
                snack_bar_top_right.dy() - DEFAULT_FAB_PADDING
            );
        });
    }

    // Dart: 'SnackBarClosedReason'.
    #[test]
    fn the_closed_future_tells_how_the_snack_bar_closed() {
        test_widgets(|tester| {
            let action_pressed = Rc::new(Cell::new(false));
            let closed_reason: Rc<Cell<Option<SnackBarClosedReason>>> = Rc::default();
            let body_context: Rc<Cell<Option<BuildContext>>> = Rc::default();
            let home = |with_action: bool| {
                let action_pressed = Rc::clone(&action_pressed);
                let closed_reason = Rc::clone(&closed_reason);
                let body_context = Rc::clone(&body_context);
                Scaffold::new().body(Builder::new(move |_app, context| {
                    body_context.set(Some(context));
                    let action_pressed = Rc::clone(&action_pressed);
                    let closed_reason = Rc::clone(&closed_reason);
                    GestureDetector::new()
                        .on_tap(Listener::new(move |app| {
                            let mut snack_bar =
                                SnackBar::new(Text::new("snack")).duration(Duration::from_secs(2));
                            if with_action {
                                let action_pressed = Rc::clone(&action_pressed);
                                snack_bar = snack_bar
                                    .action(SnackBarAction::new("ACTION", move |_app| {
                                        action_pressed.set(true)
                                    }));
                            }
                            let closed_reason = Rc::clone(&closed_reason);
                            show(app, context, snack_bar)
                                .closed()
                                .then(app, move |_app, reason| closed_reason.set(Some(reason)));
                        }))
                        .child(Text::new("X"))
                        .into_widget()
                }))
            };
            let messenger = |tester: &WidgetTester| {
                let mut app = tester.cell.borrow_mut();
                let context = body_context.get().expect("built");
                ScaffoldMessenger::of(&mut app, context)
            };
            pump_app(tester, m3, home(true).into_widget());

            // Pop up the snack bar and then press its action button.
            tap_x_and_settle_entrance(tester);
            assert!(!action_pressed.get());
            tap_text(tester, "ACTION");
            assert!(action_pressed.get());
            // Closed reason is only set when the animation is complete.
            tester.pump_by(ms(250));
            assert_eq!(closed_reason.get(), None);
            // Wait for animation to complete.
            tester.pump_and_settle_by(Duration::from_secs(1));
            assert_eq!(closed_reason.get(), Some(SnackBarClosedReason::Action));

            // Pop up the snack bar and then swipe downwards to dismiss it.
            tap_text(tester, "X");
            tester.pump_by(ms(750));
            tester.pump_by(ms(750));
            drag_text(tester, "snack", Offset::new(0.0, 50.0));
            tester.pump_and_settle_by(Duration::from_secs(1));
            assert_eq!(closed_reason.get(), Some(SnackBarClosedReason::Swipe));

            // Pop up the snack bar and then remove it.
            tap_text(tester, "X");
            tester.pump_by(ms(750));
            let scaffold_messenger = messenger(tester);
            scaffold_messenger.remove_current_snack_bar(
                &mut tester.cell.borrow_mut(),
                SnackBarClosedReason::Remove,
            );
            tester.pump_and_settle_by(Duration::from_secs(1));
            assert_eq!(closed_reason.get(), Some(SnackBarClosedReason::Remove));

            // Pop up the snack bar and then hide it.
            tap_text(tester, "X");
            tester.pump_by(ms(750));
            let scaffold_messenger = messenger(tester);
            scaffold_messenger
                .hide_current_snack_bar(&mut tester.cell.borrow_mut(), SnackBarClosedReason::Hide);
            tester.pump_and_settle_by(Duration::from_secs(1));
            assert_eq!(closed_reason.get(), Some(SnackBarClosedReason::Hide));

            // Remove action to test SnackBarClosedReason.timeout because Snackbar with
            // action doesn't timeout.
            pump_app(tester, m3, home(false).into_widget());

            // Pop up the snack bar and then let it time out.
            tap_text(tester, "X");
            tester.pump_by(ms(750));
            tester.pump_by(ms(750));
            tester.pump_by(ms(1500));
            tester.pump(); // begin animation
            tester.pump_and_settle_by(Duration::from_secs(1));
            assert_eq!(closed_reason.get(), Some(SnackBarClosedReason::Timeout));
        });
    }

    // Dart: 'accessible navigation behavior with action'.
    #[test]
    fn under_accessible_navigation_a_snack_bar_with_an_action_shows_at_once_and_stays() {
        test_widgets(|tester| {
            let scaffold_key = GlobalKey::new();
            let home = waft_widgets::MediaQuery::new(
                waft_widgets::MediaQueryData::new().accessible_navigation(true),
                ScaffoldMessenger::new(Builder::new({
                    let scaffold_key = scaffold_key.clone();
                    move |_app, context| {
                        Scaffold::new()
                            .key(scaffold_key.clone().into())
                            .body(
                                GestureDetector::new()
                                    .on_tap(Listener::new(move |app| {
                                        show(
                                            app,
                                            context,
                                            SnackBar::new(Text::new("snack"))
                                                .duration(Duration::from_secs(1))
                                                .action(SnackBarAction::new("ACTION", |_app| {})),
                                        );
                                    }))
                                    .child(Text::new("X")),
                            )
                            .into_widget()
                    }
                })),
            );
            pump_app(tester, m3, home.into_widget());
            tap_text(tester, "X");
            tester.pump();
            // Find action immediately
            assert_eq!(count_text(tester, "ACTION"), 1);
            // Snackbar doesn't close
            tester.pump_by(Duration::from_secs(10));
            assert_eq!(count_text(tester, "ACTION"), 1);
            tap_text(tester, "ACTION");
            tester.pump();
            // Snackbar closes immediately
            assert_eq!(count_text(tester, "ACTION"), 0);
        });
    }

    // Dart: 'SnackBar default display duration test'.
    #[test]
    fn a_snack_bar_shows_for_four_seconds_by_default() {
        test_widgets(|tester| {
            const HELLO_SNACK_BAR: &str = "Hello SnackBar";
            let tap_target = GlobalKey::new();
            let home = tap_target_scaffold(&tap_target, |app, context| {
                show(app, context, SnackBar::new(Text::new(HELLO_SNACK_BAR)));
            });
            pump_app(tester, m3, home.into_widget());
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 0);
            tap_key(tester, &tap_target);
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 0);
            tester.pump(); // schedule animation
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 1);
            tester.pump(); // begin animation
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 1);
            tester.pump_by(ms(750)); // 0.75s // animation last frame; four second timer starts here
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 1);
            tester.pump_by(ms(750)); // 1.50s
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 1);
            tester.pump_by(ms(750)); // 2.25s
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 1);
            tester.pump_by(ms(750)); // 3.00s
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 1);
            tester.pump_by(ms(750)); // 3.75s
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 1);
            tester.pump_by(ms(1000)); // 4.75s // timer triggers to dismiss snackbar, reverse animation is scheduled
            tester.pump(); // begin animation
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 1); // frame 0 of dismiss animation
            tester.pump_by(ms(750)); // 5.50s // last frame of animation, snackbar removed from build
            assert_eq!(count_text(tester, HELLO_SNACK_BAR), 0);
        });
    }

    // Dart: 'SnackBar handles updates to accessibleNavigation'.
    #[test]
    fn a_snack_bar_follows_changes_to_accessible_navigation() {
        test_widgets(|tester| {
            let boilerplate = |tester: &mut WidgetTester, accessible_navigation: bool| {
                let theme = m3(&tester.cell.borrow());
                tester.pump_widget(
                    waft_widgets::MediaQuery::new(
                        waft_widgets::MediaQueryData::new()
                            .accessible_navigation(accessible_navigation),
                        material_app(
                            theme,
                            Scaffold::new()
                                .body(Builder::new(|_app, context| {
                                    GestureDetector::new()
                                        .on_tap(Listener::new(move |app| {
                                            show(
                                                app,
                                                context,
                                                SnackBar::new(Text::new("test"))
                                                    .action(SnackBarAction::new("foo", |_app| {})),
                                            );
                                        }))
                                        .behavior(HitTestBehavior::Opaque)
                                        .child(Text::new("X"))
                                        .into_widget()
                                }))
                                .into_widget(),
                        ),
                    )
                    .into_widget(),
                );
            };

            boilerplate(tester, false);
            assert_eq!(count_text(tester, "test"), 0);
            tap_text(tester, "X");
            tester.pump(); // schedule animation
            assert_eq!(count_text(tester, "test"), 1);
            tester.pump(); // begin animation
            tester.pump_by(ms(4750)); // 4.75s
            assert_eq!(count_text(tester, "test"), 1);

            // Enabled accessible navigation
            boilerplate(tester, true);

            tester.pump_by(ms(4000)); // 8.75s
            tester.pump();
            assert_eq!(count_text(tester, "test"), 1);

            // disable accessible navigation
            boilerplate(tester, false);
            tester.pump_and_settle_by(ms(5750));
            assert_eq!(count_text(tester, "test"), 1);

            tap_text(tester, "foo");
            tester.pump_and_settle();
            assert_eq!(count_text(tester, "test"), 0);
        });
    }

    /// The `onVisible` cases: a tap shows the snack bars `show_all` shows, each counting into
    /// the returned cell when it becomes visible.
    fn count_on_visible(
        tester: &mut WidgetTester,
        labels: &'static [&'static str],
    ) -> Rc<Cell<i32>> {
        let called = Rc::new(Cell::new(0));
        let tap_target = GlobalKey::new();
        let home = tap_target_scaffold(&tap_target, {
            let called = Rc::clone(&called);
            move |app, context| {
                for label in labels {
                    let called = Rc::clone(&called);
                    show(
                        app,
                        context,
                        SnackBar::new(Text::new(*label))
                            .duration(Duration::from_secs(1))
                            .on_visible(move |_app| called.set(called.get() + 1)),
                    );
                }
            }
        });
        pump_app(tester, m3, home.into_widget());
        tap_key(tester, &tap_target);
        tester.pump(); // start animation
        tester.pump_and_settle();
        called
    }

    // Dart: 'Snackbar calls onVisible once'.
    #[test]
    fn a_snack_bar_calls_on_visible_once() {
        test_widgets(|tester| {
            let called = count_on_visible(tester, &["hello"]);
            assert_eq!(count_text(tester, "hello"), 1);
            assert_eq!(called.get(), 1);
        });
    }

    // Dart: 'Snackbar does not call onVisible when it is queued'.
    #[test]
    fn a_queued_snack_bar_does_not_call_on_visible() {
        test_widgets(|tester| {
            let called = count_on_visible(tester, &["hello", "hello 2"]);
            assert_eq!(count_text(tester, "hello"), 1);
            assert_eq!(called.get(), 1);
        });
    }

    // SnackBar position.

    /// Dart's `tester.getRect(find.byType(W))`, of the first `W`.
    fn rect_of_type<W: 'static>(tester: &WidgetTester) -> Rect {
        let element = tester.find_by_type::<W>()[0];
        tester.global_rect(tester.render_box_of(element))
    }

    /// Dart's `tester.getRect(find.byKey(key))`.
    fn rect_of_key(tester: &WidgetTester, key: &GlobalKey) -> Rect {
        tester.global_rect(tester.render_object(key).as_box().expect("a render box"))
    }

    /// Dart's `tester.state<ScaffoldMessengerState>(find.byType(ScaffoldMessenger))
    /// .showSnackBar(snackBar)`, then `tester.pumpAndSettle()`.
    fn show_from_messenger_and_settle(tester: &mut WidgetTester, snack_bar: SnackBar) {
        let messenger = tester.find_by_type::<ScaffoldMessenger>()[0];
        {
            let mut app = tester.cell.borrow_mut();
            let state = messenger
                .state_handle::<crate::scaffold::ScaffoldMessengerState>(&app)
                .expect("a ScaffoldMessengerState");
            state.show_snack_bar(&mut app, snack_bar, None);
        }
        tester.pump_and_settle(); // Have the SnackBar fully animate out.
    }

    fn fab() -> crate::floating_action_button::FloatingActionButton {
        crate::floating_action_button::FloatingActionButton::new(Some(Listener::new(|_app| {})))
    }

    fn fab_with_icon() -> crate::floating_action_button::FloatingActionButton {
        fab().child(Icon::new(Some(Icons::send())))
    }

    // Dart: '$behavior should align SnackBar with the bottom of Scaffold when Scaffold has no
    // other elements', for each behavior.
    #[test]
    fn a_snack_bar_aligns_with_the_bottom_of_an_empty_scaffold() {
        test_widgets(|tester| {
            for behavior in [SnackBarBehavior::Fixed, SnackBarBehavior::Floating] {
                pump_app(
                    tester,
                    m3,
                    Scaffold::new()
                        .body(waft_widgets::Container::new())
                        .into_widget(),
                );
                show_from_messenger_and_settle(
                    tester,
                    SnackBar::new(Text::new("SnackBar text")).behavior(behavior),
                );
                let snack_bar = rect_of_type::<SnackBar>(tester);
                let scaffold = rect_of_type::<Scaffold>(tester);
                assert_eq!(
                    snack_bar.bottom_right(),
                    scaffold.bottom_right(),
                    "{behavior:?}"
                );
                assert_eq!(
                    snack_bar.bottom_left(),
                    scaffold.bottom_left(),
                    "{behavior:?}"
                );
            }
        });
    }

    // Dart: '$behavior should align SnackBar with the top of BottomNavigationBar when
    // Scaffold has no FloatingActionButton', for each behavior.
    #[test]
    fn a_snack_bar_aligns_with_the_top_of_the_bottom_navigation_bar() {
        test_widgets(|tester| {
            for behavior in [SnackBarBehavior::Fixed, SnackBarBehavior::Floating] {
                let box_key = GlobalKey::new();
                pump_app(
                    tester,
                    m3,
                    Scaffold::new()
                        .body(waft_widgets::Container::new())
                        .bottom_navigation_bar(
                            SizedBox::new()
                                .key(box_key.clone().into())
                                .width(800.0)
                                .height(60.0),
                        )
                        .into_widget(),
                );
                show_from_messenger_and_settle(
                    tester,
                    SnackBar::new(Text::new("SnackBar text")).behavior(behavior),
                );
                let snack_bar = rect_of_type::<SnackBar>(tester);
                let bottom_navigation_bar = rect_of_key(tester, &box_key);
                assert_eq!(snack_bar.bottom_right(), bottom_navigation_bar.top_right());
                assert_eq!(snack_bar.bottom_left(), bottom_navigation_bar.top_left());
            }
        });
    }

    // Dart: 'Padding of SnackBarBehavior.fixed is not consumed by viewInsets'.
    #[test]
    fn a_fixed_snack_bar_s_padding_is_not_consumed_by_view_insets() {
        test_widgets(|tester| {
            let child = |tester: &WidgetTester| {
                material_app(
                    m3(&tester.cell.borrow()),
                    Scaffold::new()
                        .resize_to_avoid_bottom_inset(false)
                        .floating_action_button(fab_with_icon())
                        .body(Builder::new(|_app, context| {
                            GestureDetector::new()
                                .on_tap(Listener::new(move |app| {
                                    show(
                                        app,
                                        context,
                                        snack_bar_with_action().behavior(SnackBarBehavior::Fixed),
                                    );
                                }))
                                .child(Text::new("X"))
                                .into_widget()
                        }))
                        .into_widget(),
                )
            };
            let app = child(tester);
            tester.pump_widget(
                waft_widgets::MediaQuery::new(
                    waft_widgets::MediaQueryData::new()
                        .padding(EdgeInsets::only(0.0, 0.0, 0.0, 20.0)),
                    app,
                )
                .into_widget(),
            );
            tap_text(tester, "X");
            tester.pump_and_settle(); // Show snackbar
            let initial = rect_of_type::<SnackBar>(tester);
            // Consume bottom padding - as if by the keyboard opening
            let app = child(tester);
            tester.pump_widget(
                waft_widgets::MediaQuery::new(
                    waft_widgets::MediaQueryData::new()
                        .view_padding(EdgeInsets::all(20.0))
                        .view_insets(EdgeInsets::all(100.0)),
                    app,
                )
                .into_widget(),
            );
            tap_text(tester, "X");
            tester.pump_and_settle(); // Have the SnackBar fully animate out.
            let last = rect_of_type::<SnackBar>(tester);
            assert_eq!(initial.bottom_left(), last.bottom_left());
            assert_eq!(initial.bottom_right(), last.bottom_right());
        });
    }

    // Dart: 'SnackBarBehavior.fixed should align SnackBar with the bottom of Scaffold when
    // Scaffold has a FloatingActionButton'.
    #[test]
    fn a_fixed_snack_bar_aligns_with_the_bottom_of_a_scaffold_with_a_floating_action_button() {
        test_widgets(|tester| {
            pump_app(
                tester,
                m3,
                Scaffold::new()
                    .body(waft_widgets::Container::new())
                    .floating_action_button(fab())
                    .into_widget(),
            );
            show_from_messenger_and_settle(
                tester,
                SnackBar::new(Text::new("Snackbar text")).behavior(SnackBarBehavior::Fixed),
            );
            let snack_bar = rect_of_type::<SnackBar>(tester);
            let scaffold = rect_of_type::<Scaffold>(tester);
            assert_eq!(snack_bar.bottom_right(), scaffold.bottom_right());
            assert_eq!(snack_bar.bottom_left(), scaffold.bottom_left());
        });
    }

    /// The floating-above-the-button cases: a scaffold with a button at `fab_location` whose
    /// 'X' shows a floating snack bar with an action.
    fn floating_with_fab(
        tester: &mut WidgetTester,
        fab_location: Option<
            crate::floating_action_button_location::FloatingActionButtonLocationRef,
        >,
    ) {
        let mut scaffold = x_scaffold(|app, context| {
            show(
                app,
                context,
                snack_bar_with_action().behavior(SnackBarBehavior::Floating),
            );
        })
        .floating_action_button(fab_with_icon());
        if let Some(fab_location) = fab_location {
            scaffold = scaffold.floating_action_button_location(fab_location);
        }
        pump_app(tester, m3, scaffold.into_widget());
        tap_text(tester, "X");
        tester.pump_and_settle(); // Have the SnackBar fully animate out.
    }

    // Dart: 'SnackBarBehavior.floating should align SnackBar with the top of
    // FloatingActionButton when Scaffold has a FloatingActionButton'.
    #[test]
    fn a_floating_snack_bar_sits_on_top_of_the_floating_action_button() {
        test_widgets(|tester| {
            floating_with_fab(tester, None);
            // Since padding between the SnackBar and the FAB is created by the SnackBar,
            // the bottom offset of the SnackBar should be equal to the top offset of the FAB
            assert_eq!(
                rect_of_type::<SnackBar>(tester).bottom_left().dy(),
                rect_of_type::<crate::floating_action_button::FloatingActionButton>(tester)
                    .top_left()
                    .dy()
            );
        });
    }

    // Dart: 'SnackBarBehavior.floating should not align SnackBar with the top of
    // FloatingActionButton when Scaffold has a FloatingActionButton and
    // floatingActionButtonLocation is set to a top position'.
    #[test]
    fn a_floating_snack_bar_ignores_a_floating_action_button_at_a_top_location() {
        test_widgets(|tester| {
            use crate::floating_action_button_location::FloatingActionButtonLocation;
            let top_locations = [
                <dyn FloatingActionButtonLocation>::start_top(),
                <dyn FloatingActionButtonLocation>::center_top(),
                <dyn FloatingActionButtonLocation>::end_top(),
                <dyn FloatingActionButtonLocation>::mini_start_top(),
                <dyn FloatingActionButtonLocation>::mini_center_top(),
                <dyn FloatingActionButtonLocation>::mini_end_top(),
            ];
            for location in top_locations {
                floating_with_fab(tester, Some(Rc::clone(&location)));
                assert_eq!(
                    rect_of_type::<SnackBar>(tester).bottom_left().dy(),
                    600.0, // Device height is 600.
                    "{location:?}"
                );
            }
        });
    }

    /// Dart's `_CustomFloatingActionButtonLocation`.
    #[derive(Debug)]
    struct CustomFloatingActionButtonLocation;

    impl crate::floating_action_button_location::FloatingActionButtonLocation
        for CustomFloatingActionButtonLocation
    {
        fn get_offset(
            &self,
            scaffold_geometry: &crate::scaffold::ScaffoldPrelayoutGeometry,
        ) -> Offset {
            crate::floating_action_button_location::StandardFabLocation::get_offset(
                self,
                scaffold_geometry,
            )
        }
    }

    impl crate::floating_action_button_location::StandardFabLocation
        for CustomFloatingActionButtonLocation
    {
        fn get_offset_x(
            &self,
            scaffold_geometry: &crate::scaffold::ScaffoldPrelayoutGeometry,
            adjustment: f64,
        ) -> f64 {
            crate::floating_action_button_location::FabEndOffsetX::get_offset_x(
                self,
                scaffold_geometry,
                adjustment,
            )
        }

        fn get_offset_y(
            &self,
            scaffold_geometry: &crate::scaffold::ScaffoldPrelayoutGeometry,
            adjustment: f64,
        ) -> f64 {
            crate::floating_action_button_location::FabFloatOffsetY::get_offset_y(
                self,
                scaffold_geometry,
                adjustment,
            )
        }
    }

    impl crate::floating_action_button_location::FabEndOffsetX for CustomFloatingActionButtonLocation {}
    impl crate::floating_action_button_location::FabFloatOffsetY
        for CustomFloatingActionButtonLocation
    {
    }

    // Dart: 'SnackBarBehavior.floating should align SnackBar with the top of
    // FloatingActionButton when Scaffold has a FloatingActionButton and
    // floatingActionButtonLocation is not set to a top position'.
    #[test]
    fn a_floating_snack_bar_sits_on_top_of_a_floating_action_button_at_any_other_location() {
        test_widgets(|tester| {
            use crate::floating_action_button_location::FloatingActionButtonLocation;
            let non_top_locations: Vec<
                crate::floating_action_button_location::FloatingActionButtonLocationRef,
            > = vec![
                <dyn FloatingActionButtonLocation>::start_docked(),
                <dyn FloatingActionButtonLocation>::start_float(),
                <dyn FloatingActionButtonLocation>::center_docked(),
                <dyn FloatingActionButtonLocation>::center_float(),
                <dyn FloatingActionButtonLocation>::end_contained(),
                <dyn FloatingActionButtonLocation>::end_docked(),
                <dyn FloatingActionButtonLocation>::end_float(),
                <dyn FloatingActionButtonLocation>::mini_start_docked(),
                <dyn FloatingActionButtonLocation>::mini_start_float(),
                <dyn FloatingActionButtonLocation>::mini_center_docked(),
                <dyn FloatingActionButtonLocation>::mini_center_float(),
                <dyn FloatingActionButtonLocation>::mini_end_docked(),
                <dyn FloatingActionButtonLocation>::mini_end_float(),
                // Regression test related to https://github.com/flutter/flutter/pull/131303.
                Rc::new(CustomFloatingActionButtonLocation),
            ];
            for location in non_top_locations {
                floating_with_fab(tester, Some(Rc::clone(&location)));
                // Since padding between the SnackBar and the FAB is created by the SnackBar,
                // the bottom offset of the SnackBar should be equal to the top offset of the FAB
                assert_eq!(
                    rect_of_type::<SnackBar>(tester).bottom_left().dy(),
                    rect_of_type::<crate::floating_action_button::FloatingActionButton>(tester)
                        .top_left()
                        .dy(),
                    "{location:?}"
                );
            }
        });
    }

    /// A scaffold with a 60 high bottom navigation bar keyed `box_key` and a button.
    fn with_bottom_bar_and_fab(
        box_key: &GlobalKey,
        bar_height: f64,
        fab_location: Option<
            crate::floating_action_button_location::FloatingActionButtonLocationRef,
        >,
    ) -> WidgetRef {
        let mut scaffold = Scaffold::new()
            .body(waft_widgets::Container::new())
            .bottom_navigation_bar(
                SizedBox::new()
                    .key(box_key.clone().into())
                    .width(800.0)
                    .height(bar_height),
            )
            .floating_action_button(fab());
        if let Some(fab_location) = fab_location {
            scaffold = scaffold.floating_action_button_location(fab_location);
        }
        scaffold.into_widget()
    }

    // Dart: 'SnackBarBehavior.fixed should align SnackBar with the top of BottomNavigationBar
    // when Scaffold has a BottomNavigationBar and FloatingActionButton'.
    #[test]
    fn a_fixed_snack_bar_sits_on_the_bottom_navigation_bar_beside_a_floating_action_button() {
        test_widgets(|tester| {
            let box_key = GlobalKey::new();
            pump_app(tester, m3, with_bottom_bar_and_fab(&box_key, 60.0, None));
            show_from_messenger_and_settle(
                tester,
                SnackBar::new(Text::new("SnackBar text")).behavior(SnackBarBehavior::Fixed),
            );
            let snack_bar = rect_of_type::<SnackBar>(tester);
            let bottom_navigation_bar = rect_of_key(tester, &box_key);
            assert_eq!(snack_bar.bottom_right(), bottom_navigation_bar.top_right());
            assert_eq!(snack_bar.bottom_left(), bottom_navigation_bar.top_left());
        });
    }

    // Dart: 'SnackBarBehavior.floating should align SnackBar with the top of
    // FloatingActionButton when Scaffold has BottomNavigationBar and FloatingActionButton'.
    #[test]
    fn a_floating_snack_bar_sits_on_the_floating_action_button_above_a_bottom_navigation_bar() {
        test_widgets(|tester| {
            let box_key = GlobalKey::new();
            pump_app(tester, m3, with_bottom_bar_and_fab(&box_key, 60.0, None));
            show_from_messenger_and_settle(
                tester,
                SnackBar::new(Text::new("SnackBar text")).behavior(SnackBarBehavior::Floating),
            );
            assert_eq!(
                rect_of_type::<SnackBar>(tester).bottom_right().dy(),
                rect_of_type::<crate::floating_action_button::FloatingActionButton>(tester)
                    .top_right()
                    .dy()
            );
        });
    }

    // Dart: 'SnackBarBehavior.floating should align SnackBar with the top of
    // BottomNavigationBar when Scaffold has both BottomNavigationBar and FloatingActionButton
    // and BottomNavigationBar.top is higher than FloatingActionButton.top'.
    #[test]
    fn a_floating_snack_bar_sits_on_a_bottom_navigation_bar_higher_than_the_button() {
        test_widgets(|tester| {
            use crate::floating_action_button_location::FloatingActionButtonLocation;
            let box_key = GlobalKey::new();
            pump_app(
                tester,
                m3,
                with_bottom_bar_and_fab(
                    &box_key,
                    200.0,
                    Some(<dyn FloatingActionButtonLocation>::end_contained()),
                ),
            );
            show_from_messenger_and_settle(
                tester,
                SnackBar::new(Text::new("SnackBar text")).behavior(SnackBarBehavior::Floating),
            );
            let snack_bar_bottom_right = rect_of_type::<SnackBar>(tester).bottom_right();
            let fab_top_right =
                rect_of_type::<crate::floating_action_button::FloatingActionButton>(tester)
                    .top_right();
            let nav_bar_top_right = rect_of_key(tester, &box_key).top_right();

            // Test the top of the navigation bar is higher than the top of the floating action button.
            assert!(fab_top_right.dy() > nav_bar_top_right.dy());

            assert_eq!(snack_bar_bottom_right.dy(), nav_bar_top_right.dy());
        });
    }

    /// Dart's `openFloatingSnackBar`.
    fn open_floating_snack_bar(tester: &mut WidgetTester) {
        show_from_messenger_and_settle(
            tester,
            SnackBar::new(Text::new("SnackBar text")).behavior(SnackBarBehavior::Floating),
        );
    }

    /// Dart's `boilerplate(fabHeight:)`: a scaffold whose button is a container that high.
    fn with_fab_height(tester: &mut WidgetTester, fab_height: f64) {
        pump_app(
            tester,
            m3,
            Scaffold::new()
                .floating_action_button(waft_widgets::Container::new().height(fab_height))
                .into_widget(),
        );
    }

    const MEDIUM_FAB_HEIGHT: f64 = 100.0;

    /// Run once with a visible SnackBar to compute the empty space above SnackBar.
    fn space_above_snack_bar(tester: &mut WidgetTester) -> f64 {
        with_fab_height(tester, MEDIUM_FAB_HEIGHT);
        open_floating_snack_bar(tester);
        rect_of_type::<SnackBar>(tester).top_left().dy()
    }

    // Dart: 'Snackbar with SnackBarBehavior.floating will assert when offset too high by a
    // large Scaffold.floatingActionButton', the runs that do not assert.
    #[test]
    fn a_floating_snack_bar_right_at_the_top_of_the_screen_does_not_assert() {
        test_widgets(|tester| {
            let space_above_snack_bar = space_above_snack_bar(tester);
            // Run with the Snackbar fully visible right on the top of the screen.
            with_fab_height(tester, space_above_snack_bar + MEDIUM_FAB_HEIGHT);
            open_floating_snack_bar(tester);
        });
    }

    // Dart: the same case, the Snackbar fully off screen.
    #[test]
    #[should_panic(
        expected = "Floating SnackBar presented off screen.\nA SnackBar with behavior property \
                    set to SnackBarBehavior.floating is fully or partially off screen because \
                    some or all the widgets provided to Scaffold.floatingActionButton, \
                    Scaffold.persistentFooterButtons and Scaffold.bottomNavigationBar take up \
                    too much vertical space.\nConsider constraining the size of these widgets \
                    to allow room for the SnackBar to be visible."
    )]
    fn a_floating_snack_bar_pushed_fully_off_screen_by_a_large_button_asserts() {
        test_widgets(|tester| {
            let space_above_snack_bar = space_above_snack_bar(tester);
            with_fab_height(tester, space_above_snack_bar + MEDIUM_FAB_HEIGHT * 2.0);
            open_floating_snack_bar(tester);
        });
    }

    // Dart: the same case, the Snackbar partially off screen.
    #[test]
    #[should_panic(expected = "Floating SnackBar presented off screen.")]
    fn a_floating_snack_bar_pushed_partly_off_screen_by_a_large_button_asserts() {
        test_widgets(|tester| {
            let space_above_snack_bar = space_above_snack_bar(tester);
            with_fab_height(tester, space_above_snack_bar + MEDIUM_FAB_HEIGHT + 10.0);
            open_floating_snack_bar(tester);
        });
    }

    // Dart: 'Material2 - Snackbar with SnackBarBehavior.floating will assert when offset too
    // high by a large Scaffold.persistentFooterButtons'.
    #[test]
    #[should_panic(expected = "Floating SnackBar presented off screen.")]
    fn a_material_2_floating_snack_bar_pushed_off_screen_by_large_footer_buttons_asserts() {
        test_widgets(|tester| {
            pump_app(
                tester,
                m2,
                Scaffold::new()
                    .persistent_footer_buttons(vec![SizedBox::new().height(1000.0).into_widget()])
                    .into_widget(),
            );
            open_floating_snack_bar(tester);
            tester.pump_and_settle(); // Have the SnackBar fully animate out.
        });
    }

    // Dart: 'Material2 - Snackbar with SnackBarBehavior.floating will assert when offset too
    // high by a large Scaffold.bottomNavigationBar'.
    #[test]
    #[should_panic(expected = "Floating SnackBar presented off screen.")]
    fn a_material_2_floating_snack_bar_pushed_off_screen_by_a_large_bottom_bar_asserts() {
        test_widgets(|tester| {
            pump_app(
                tester,
                m2,
                Scaffold::new()
                    .bottom_navigation_bar(SizedBox::new().height(1000.0))
                    .into_widget(),
            );
            open_floating_snack_bar(tester);
            tester.pump_and_settle(); // Have the SnackBar fully animate out.
        });
    }

    // Dart: 'Material3 - Snackbar with SnackBarBehavior.floating will assert when offset too
    // high by a large Scaffold.persistentFooterButtons'.
    #[test]
    #[should_panic(expected = "Floating SnackBar presented off screen.")]
    fn a_material_3_floating_snack_bar_pushed_off_screen_by_large_footer_buttons_asserts() {
        test_widgets(|tester| {
            pump_app(
                tester,
                m3,
                Scaffold::new()
                    .persistent_footer_buttons(vec![SizedBox::new().height(1000.0).into_widget()])
                    .into_widget(),
            );
            open_floating_snack_bar(tester);
            tester.pump_and_settle(); // Have the SnackBar fully animate out.
        });
    }

    // Dart: 'Material3 - Snackbar with SnackBarBehavior.floating will assert when offset too
    // high by a large Scaffold.bottomNavigationBar'.
    #[test]
    #[should_panic(expected = "Floating SnackBar presented off screen.")]
    fn a_material_3_floating_snack_bar_pushed_off_screen_by_a_large_bottom_bar_asserts() {
        test_widgets(|tester| {
            pump_app(
                tester,
                m3,
                Scaffold::new()
                    .bottom_navigation_bar(SizedBox::new().height(1000.0))
                    .into_widget(),
            );
            open_floating_snack_bar(tester);
            tester.pump_and_settle(); // Have the SnackBar fully animate out.
        });
    }

    /// Asserts `actual` is `expected` up to the precision a global position keeps through
    /// the view's single-precision transforms, where Dart's are double precision.
    fn assert_more_or_less_equals(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-3,
            "{actual} is not more or less {expected}"
        );
    }

    /// The end-padding cases: the distance from the snack bar's right edge to its action's.
    fn action_end_padding(tester: &mut WidgetTester, behavior: SnackBarBehavior) -> f64 {
        let home = x_scaffold(move |app, context| {
            show(
                app,
                context,
                SnackBar::new(Text::new("Some content"))
                    .behavior(behavior)
                    .action(SnackBarAction::new("ACTION", |_app| {})),
            );
        });
        pump_app(tester, m3, home.into_widget());
        tap_text(tester, "X");
        tester.pump_and_settle();
        rect_of_type::<SnackBar>(tester).top_right().dx()
            - rect_of_type::<SnackBarAction>(tester).top_right().dx()
    }

    // Dart: 'SnackBar has correct end padding when it contains an action with fixed behavior'.
    #[test]
    fn a_fixed_snack_bar_s_action_sits_12_from_its_end() {
        test_widgets(|tester| {
            assert_more_or_less_equals(action_end_padding(tester, SnackBarBehavior::Fixed), 12.0);
        });
    }

    // Dart: 'SnackBar has correct end padding when it contains an action with floating
    // behavior'.
    #[test]
    fn a_floating_snack_bar_s_action_sits_23_from_its_end() {
        test_widgets(|tester| {
            assert_more_or_less_equals(
                action_end_padding(tester, SnackBarBehavior::Floating),
                8.0 + 15.0, // button margin + horizontal scaffold outside margin
            );
        });
    }

    /// The right-to-left custom width cases.
    fn centred_right_to_left(tester: &mut WidgetTester, theme: fn(&App) -> ThemeData) {
        const CUSTOM_WIDTH: f64 = 400.0;
        let home = Directionality::new(
            TextDirection::Rtl,
            x_scaffold(|app, context| {
                show(
                    app,
                    context,
                    SnackBar::new(Text::new("Feeling super snackish"))
                        .behavior(SnackBarBehavior::Floating)
                        .width(CUSTOM_WIDTH),
                );
            }),
        );
        pump_app(tester, theme, home.into_widget());
        tap_x_and_settle_entrance(tester);
        assert_centred_with_width(tester, CUSTOM_WIDTH);
    }

    // Dart: 'Material3 - Floating snackbar with custom width is centered when text direction
    // is rtl'.
    #[test]
    fn a_material_3_floating_snack_bar_of_custom_width_is_centred_right_to_left() {
        test_widgets(|tester| centred_right_to_left(tester, m3));
    }

    // Dart: 'Material2 - Floating snackbar with custom width is centered when text direction
    // is rtl'.
    #[test]
    fn a_material_2_floating_snack_bar_of_custom_width_is_centred_right_to_left() {
        test_widgets(|tester| centred_right_to_left(tester, m2));
    }

    // Dart: 'ScaffoldMessenger does not duplicate a SnackBar when presenting a
    // MaterialBanner.'.
    #[test]
    fn presenting_a_material_banner_does_not_duplicate_the_snack_bar() {
        test_widgets(|tester| {
            const SNACK_BAR_TEXT: &str = "SnackBar";
            const MATERIAL_BANNER_TEXT: &str = "MaterialBanner";
            let material_banner_tap_target = GlobalKey::new();
            let snack_bar_tap_target = GlobalKey::new();
            let home = Scaffold::new().body(Builder::new({
                let material_banner_tap_target = material_banner_tap_target.clone();
                let snack_bar_tap_target = snack_bar_tap_target.clone();
                move |_app, context| {
                    waft_widgets::Column::new()
                        .children([
                            GestureDetector::new()
                                .key(snack_bar_tap_target.clone().into())
                                .on_tap(Listener::new(move |app| {
                                    show(app, context, SnackBar::new(Text::new(SNACK_BAR_TEXT)));
                                }))
                                .behavior(HitTestBehavior::Opaque)
                                .child(SizedBox::new().height(100.0).width(100.0))
                                .into_widget(),
                            GestureDetector::new()
                                .key(material_banner_tap_target.clone().into())
                                .on_tap(Listener::new(move |app| {
                                    let banner = crate::banner::MaterialBanner::new(
                                        Text::new(MATERIAL_BANNER_TEXT),
                                        vec![
                                            TextButton::new(
                                                Some(Listener::new(move |app| {
                                                    ScaffoldMessenger::of(app, context)
                                                        .hide_current_material_banner(
                                                            app,
                                                            crate::banner::MaterialBannerClosedReason::Hide,
                                                        );
                                                })),
                                                Some(Text::new("DISMISS")),
                                            )
                                            .into_widget(),
                                        ],
                                    );
                                    ScaffoldMessenger::of(app, context)
                                        .show_material_banner(app, banner);
                                }))
                                .behavior(HitTestBehavior::Opaque)
                                .child(SizedBox::new().height(100.0).width(100.0))
                                .into_widget(),
                        ])
                        .into_widget()
                }
            }));
            pump_app(tester, m3, home.into_widget());
            tap_key(tester, &snack_bar_tap_target);
            tap_key(tester, &material_banner_tap_target);
            tester.pump_and_settle();

            assert_eq!(count_text(tester, SNACK_BAR_TEXT), 1);
            assert_eq!(count_text(tester, MATERIAL_BANNER_TEXT), 1);
        });
    }

    /// The nested scaffold cases: a scaffold with a button whose body is another scaffold.
    fn top_of_snack_bar_over_nested_scaffolds(
        tester: &mut WidgetTester,
        theme: fn(&App) -> ThemeData,
    ) -> f64 {
        pump_app(
            tester,
            theme,
            Scaffold::new()
                .body(Scaffold::new())
                .floating_action_button(fab())
                .into_widget(),
        );
        show_from_messenger_and_settle(
            tester,
            SnackBar::new(Text::new("ScaffoldMessenger"))
                .duration(Duration::from_secs(2))
                .action(SnackBarAction::new("ACTION", |_app| {}))
                .behavior(SnackBarBehavior::Floating),
        );
        assert_eq!(tester.find_by_type::<SnackBar>().len(), 1);
        // The FloatingActionButton helps us identify which Scaffold has the
        // SnackBar here. Since the outer Scaffold contains a FAB, the SnackBar
        // should be above it. If the inner Scaffold had the SnackBar, it would be
        // overlapping the FAB.
        rect_of_type::<SnackBar>(tester).top_right().dy()
    }

    // Dart: 'Material2 - ScaffoldMessenger presents SnackBars to only the root Scaffold when
    // Scaffolds are nested.', without the golden.
    #[test]
    fn a_material_2_snack_bar_shows_only_in_the_root_of_nested_scaffolds() {
        test_widgets(|tester| {
            assert_eq!(top_of_snack_bar_over_nested_scaffolds(tester, m2), 465.0);
        });
    }

    // Dart: 'Material3 - ScaffoldMessenger presents SnackBars to only the root Scaffold when
    // Scaffolds are nested.', without the golden.
    #[test]
    fn a_material_3_snack_bar_shows_only_in_the_root_of_nested_scaffolds() {
        test_widgets(|tester| {
            assert_eq!(top_of_snack_bar_over_nested_scaffolds(tester, m3), 465.0);
        });
    }

    // Dart: 'ScaffoldMessengerState clearSnackBars works as expected'.
    #[test]
    fn clear_snack_bars_hides_the_current_snack_bar_and_drops_the_queue() {
        test_widgets(|tester| {
            const SNACK_BARS: [&str; 3] = ["Hello Snackbar", "Hi Snackbar", "Bye Snackbar"];
            let snack_bar_counter = Rc::new(Cell::new(0));
            let tap_target = GlobalKey::new();
            let scaffold_messenger_key = GlobalKey::new();
            let home = ScaffoldMessenger::new(tap_target_scaffold(&tap_target, {
                let snack_bar_counter = Rc::clone(&snack_bar_counter);
                move |app, context| {
                    let index = snack_bar_counter.get();
                    snack_bar_counter.set(index + 1);
                    show(
                        app,
                        context,
                        SnackBar::new(Text::new(SNACK_BARS[index]))
                            .duration(Duration::from_secs(2)),
                    );
                }
            }))
            .key(scaffold_messenger_key.clone().into());
            pump_app(tester, m3, home.into_widget());
            let expect = |tester: &WidgetTester, counts: [usize; 3]| {
                for (text, count) in SNACK_BARS.iter().zip(counts) {
                    assert_eq!(count_text(tester, text), count, "{text}");
                }
            };
            expect(tester, [0, 0, 0]);
            tap_key(tester, &tap_target);
            tap_key(tester, &tap_target);
            tap_key(tester, &tap_target);
            expect(tester, [0, 0, 0]);
            tester.pump(); // schedule animation
            assert_eq!(count_text(tester, SNACK_BARS[0]), 1);
            {
                let mut app = tester.cell.borrow_mut();
                let messenger = scaffold_messenger_key
                    .current_state::<crate::scaffold::ScaffoldMessengerState>(&app)
                    .expect("mounted");
                messenger.clear_snack_bars(&mut app);
            }
            assert_eq!(count_text(tester, SNACK_BARS[0]), 1);
            tester.pump_by(Duration::from_secs(2));
            expect(tester, [0, 0, 0]);
        });
    }

    /// Dart's `doBuildApp`: the 'X' scaffold, with a button, showing a snack bar with an action
    /// configured by `configure`.
    fn do_build_app(tester: &mut WidgetTester, configure: impl Fn(SnackBar) -> SnackBar + 'static) {
        let home = x_scaffold(move |app, context| {
            show(app, context, configure(snack_bar_with_action()));
        })
        .floating_action_button(fab_with_icon());
        pump_app(tester, m3, home.into_widget());
    }

    /// The margin and width assertion cases: builds `configure`'s snack bar and runs its
    /// entrance, where the build asserts.
    fn shows(tester: &mut WidgetTester, configure: impl Fn(SnackBar) -> SnackBar + 'static) {
        do_build_app(tester, configure);
        tap_x_and_settle_entrance(tester);
    }

    // Dart: 'Setting SnackBarBehavior.fixed will still assert for margin'.
    #[test]
    #[should_panic(
        expected = "Margin can only be used with floating behavior. SnackBarBehavior.fixed was \
                    set in the SnackBar constructor."
    )]
    fn a_fixed_snack_bar_asserts_against_a_margin() {
        test_widgets(|tester| {
            shows(tester, |snack_bar| {
                snack_bar
                    .behavior(SnackBarBehavior::Fixed)
                    .margin(EdgeInsets::all(8.0))
            });
        });
    }

    // Dart: 'Default SnackBarBehavior will still assert for margin'.
    #[test]
    #[should_panic(
        expected = "Margin can only be used with floating behavior. SnackBarBehavior.fixed was \
                    set by default."
    )]
    fn a_snack_bar_fixed_by_default_asserts_against_a_margin() {
        test_widgets(|tester| {
            shows(tester, |snack_bar| snack_bar.margin(EdgeInsets::all(8.0)));
        });
    }

    // Dart: 'Setting SnackBarBehavior.fixed will still assert for width'.
    #[test]
    #[should_panic(
        expected = "Width can only be used with floating behavior. SnackBarBehavior.fixed was \
                    set in the SnackBar constructor."
    )]
    fn a_fixed_snack_bar_asserts_against_a_width() {
        test_widgets(|tester| {
            shows(tester, |snack_bar| {
                snack_bar.behavior(SnackBarBehavior::Fixed).width(5.0)
            });
        });
    }

    // Dart: 'Default SnackBarBehavior will still assert for width'.
    #[test]
    #[should_panic(
        expected = "Width can only be used with floating behavior. SnackBarBehavior.fixed was \
                    set by default."
    )]
    fn a_snack_bar_fixed_by_default_asserts_against_a_width() {
        test_widgets(|tester| {
            shows(tester, |snack_bar| snack_bar.width(5.0));
        });
    }

    // Dart: 'SnackBar will assert for actionOverflowThreshold outside of 0-1 range', for each
    // threshold.
    #[test]
    fn a_snack_bar_asserts_for_an_action_overflow_threshold_outside_0_to_1() {
        for overflow_threshold in [-1.0, -0.0001, 1.000001, 5.0] {
            let outcome = std::panic::catch_unwind(|| {
                test_widgets(|tester| {
                    shows(tester, move |snack_bar| {
                        snack_bar
                            .action_overflow_threshold(overflow_threshold)
                            .behavior(SnackBarBehavior::Fixed)
                    });
                });
            });
            let panic = outcome.expect_err("the threshold asserts");
            let message = panic
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| {
                    panic
                        .downcast_ref::<&str>()
                        .map(|message| message.to_string())
                })
                .expect("a message");
            assert_eq!(
                message, "Action overflow threshold must be between 0 and 1 inclusive",
                "{overflow_threshold}"
            );
        }
    }

    // Dart: 'SnackBar has tooltip for Close Button'.
    #[test]
    fn a_snack_bar_s_close_button_has_a_tooltip() {
        test_widgets(|tester| {
            pump_app(
                tester,
                m2,
                Scaffold::new()
                    .bottom_sheet(
                        SizedBox::new()
                            .width(200.0)
                            .height(50.0)
                            .child(waft_widgets::ColoredBox::new(Colors::PINK.primary())),
                    )
                    .into_widget(),
            );
            show_from_messenger_and_settle(
                tester,
                SnackBar::new(Text::new("Snackbar with close button"))
                    .duration(Duration::from_secs(365 * 24 * 60 * 60))
                    .action(SnackBarAction::new("ACTION", |_app| {}))
                    .behavior(SnackBarBehavior::Floating)
                    .show_close_icon(true),
            ); // Have the SnackBar fully animate in.
            let close_button_label = {
                let messenger = tester.find_by_type::<ScaffoldMessenger>()[0];
                let mut app = tester.cell.borrow_mut();
                let context = messenger
                    .state_handle::<crate::scaffold::ScaffoldMessengerState>(&app)
                    .expect("a ScaffoldMessengerState")
                    .context(&app);
                <dyn MaterialLocalizations>::of(&mut app, context).close_button_label()
            };
            let tooltips = tester.find_by_type::<crate::tooltip::Tooltip>();
            let with_label = tooltips
                .into_iter()
                .filter(|tooltip| {
                    tester.widget(*tooltip, |tooltip: &crate::tooltip::Tooltip| {
                        tooltip.message.as_deref() == Some(close_button_label.as_str())
                    })
                })
                .count();
            assert_eq!(with_label, 1);
        });
    }

    // Dart: 'ScaffoldMessenger will alert for snackbars that cannot be presented'.
    #[test]
    #[should_panic(
        expected = "ScaffoldMessenger.showSnackBar was called, but there are currently no \
                    descendant Scaffolds to present to."
    )]
    fn showing_a_snack_bar_with_no_scaffold_to_present_it_asserts() {
        test_widgets(|tester| {
            pump_app(tester, m3, waft_widgets::Center::new().into_widget());
            let messenger = tester.find_by_type::<ScaffoldMessenger>()[0];
            let mut app = tester.cell.borrow_mut();
            let state = messenger
                .state_handle::<crate::scaffold::ScaffoldMessengerState>(&app)
                .expect("a ScaffoldMessengerState");
            state.show_snack_bar(&mut app, SnackBar::new(Text::new("SnackBar")), None);
        });
    }

    /// Dart's `tester.widget<Material>(find.descendant(of: find.widgetWithText(TextButton,
    /// 'ACTION'), matching: find.byType(Material))).color`.
    fn action_material_color(tester: &WidgetTester) -> Option<Color> {
        let action = tester.find_text("ACTION")[0];
        let button = tester.ancestor_of_type::<TextButton>(action);
        let material = tester
            .find_by_type::<Material>()
            .into_iter()
            .find(|material| tester.ancestors(*material).contains(&button))
            .expect("a TextButton builds a Material");
        tester.widget(material, |material: &Material| material.color)
    }

    /// The action background cases: a 'Tap' scaffold showing a snack bar with `action`;
    /// returns the action's material colour before and after the action is pressed.
    fn action_background_before_and_after_press(
        tester: &mut WidgetTester,
        action: impl Fn() -> SnackBarAction + 'static,
    ) -> (Option<Color>, Option<Color>) {
        let home = Scaffold::new().body(Builder::new({
            let action = Rc::new(action);
            move |_app, context| {
                let action = Rc::clone(&action);
                GestureDetector::new()
                    .on_tap(Listener::new(move |app| {
                        show(
                            app,
                            context,
                            SnackBar::new(Text::new("I am a snack bar."))
                                .duration(Duration::from_secs(2))
                                .action(action()),
                        );
                    }))
                    .child(Text::new("Tap"))
                    .into_widget()
            }
        }));
        pump_app(tester, m3, home.into_widget());
        tap_text(tester, "Tap");
        tester.pump_and_settle();
        let before = action_material_color(tester);
        tap_text(tester, "ACTION");
        tester.pump();
        let after = action_material_color(tester);
        (before, after)
    }

    // Dart: 'SnackBarAction backgroundColor works as a Color'.
    #[test]
    fn a_snack_bar_action_s_background_color_turns_transparent_once_pressed() {
        test_widgets(|tester| {
            let background_color = Colors::BLUE.primary();
            let (before, after) = action_background_before_and_after_press(tester, move || {
                SnackBarAction::new("ACTION", |_app| {}).background_color(background_color)
            });
            assert_eq!(before, Some(background_color));
            assert_eq!(after, Some(Colors::TRANSPARENT));
        });
    }

    // Dart: 'SnackBarAction backgroundColor works as a WidgetStateColor'.
    #[test]
    fn a_snack_bar_action_s_widget_state_background_color_resolves_by_state() {
        test_widgets(|tester| {
            let (before, after) = action_background_before_and_after_press(tester, || {
                let background_color = WidgetStateColor::resolve_with(|states| {
                    if states.contains(&WidgetState::Disabled) {
                        return Colors::BLUE.primary();
                    }
                    Colors::PURPLE.primary()
                });
                SnackBarAction::new("ACTION", |_app| {}).background_color(background_color)
            });
            assert_eq!(before, Some(Colors::PURPLE.primary()));
            assert_eq!(after, Some(Colors::BLUE.primary()));
        });
    }

    // Dart: 'SnackBarAction disabledBackgroundColor works as expected'.
    #[test]
    fn a_snack_bar_action_shows_its_disabled_background_color_once_pressed() {
        test_widgets(|tester| {
            let background_color = Colors::BLUE.primary();
            let disabled_background_color = Colors::RED.primary();
            let (before, after) = action_background_before_and_after_press(tester, move || {
                SnackBarAction::new("ACTION", |_app| {})
                    .background_color(background_color)
                    .disabled_background_color(disabled_background_color)
            });
            assert_eq!(before, Some(background_color));
            assert_eq!(after, Some(disabled_background_color));
        });
    }

    // Dart: 'SnackBarAction asserts when backgroundColor is a WidgetStateColor and
    // disabledBackgroundColor is also provided'.
    #[test]
    #[should_panic(
        expected = "disabledBackgroundColor must not be provided when background color is a \
                    WidgetStateColor"
    )]
    fn a_widget_state_action_background_with_a_disabled_background_asserts() {
        test_widgets(|tester| {
            action_background_before_and_after_press(tester, || {
                let background_color = WidgetStateColor::resolve_with(|states| {
                    if states.contains(&WidgetState::Disabled) {
                        return Colors::BLUE.primary();
                    }
                    Colors::PURPLE.primary()
                });
                SnackBarAction::new("ACTION", |_app| {})
                    .background_color(background_color)
                    .disabled_background_color(Colors::RED.primary())
            });
        });
    }

    // Dart: 'SnackBar material applies SnackBar.clipBehavior'.
    #[test]
    fn a_snack_bar_s_material_takes_its_clip_behavior() {
        test_widgets(|tester| {
            pump_app(
                tester,
                m3,
                Scaffold::new()
                    .body(waft_widgets::Container::new())
                    .into_widget(),
            );
            let clip_behavior = |tester: &WidgetTester| {
                tester.widget(snack_bar_material(tester), |material: &Material| {
                    material.clip_behavior
                })
            };
            show_from_messenger_and_settle(tester, SnackBar::new(Text::new("I am a snack bar.")));
            assert_eq!(clip_behavior(tester), Clip::HardEdge);
            {
                let messenger = tester.find_by_type::<ScaffoldMessenger>()[0];
                let mut app = tester.cell.borrow_mut();
                let state = messenger
                    .state_handle::<crate::scaffold::ScaffoldMessengerState>(&app)
                    .expect("a ScaffoldMessengerState");
                state.hide_current_snack_bar(&mut app, SnackBarClosedReason::Hide); // Hide the SnackBar.
            }
            tester.pump_and_settle(); // Have the SnackBar fully animate out.
            show_from_messenger_and_settle(
                tester,
                SnackBar::new(Text::new("I am a snack bar.")).clip_behavior(Clip::AntiAlias),
            ); // Have the SnackBar fully animate in.
            assert_eq!(clip_behavior(tester), Clip::AntiAlias);
        });
    }

    /// The tap-behind cases, on a 200x200 view: two buttons at the top and the bottom of a
    /// column, the first showing `snack_bar`; returns whether a tap at the top left of the
    /// second button's label pressed it, with the snack bar still showing.
    fn taps_behind_the_snack_bar(
        theme: impl FnOnce(&App) -> ThemeData,
        snack_bar: impl Fn() -> SnackBar + 'static,
    ) -> bool {
        const BUTTON_TEXT: &str = "Show snackbar";
        const SNACKBAR_CONTENT: &str = "Snackbar";
        const BUTTON_TEXT2: &str = "Try press me";
        let completed = Rc::new(Cell::new(false));
        let mut tester = WidgetTester::with_view(
            waft_test::TestPlatform::new().on(TargetPlatform::Android),
            waft_test::TestView::with_pixel_ratio(200.0, 200.0, 1.0),
        );
        let tester = &mut tester;
        let snack_bar = Rc::new(snack_bar);
        let home = Scaffold::new().body(Builder::new({
            let completed = Rc::clone(&completed);
            move |_app, context| {
                let snack_bar = Rc::clone(&snack_bar);
                let completed = Rc::clone(&completed);
                waft_widgets::Column::new()
                    .main_axis_alignment(MainAxisAlignment::SpaceBetween)
                    .cross_axis_alignment(waft_widgets::CrossAxisAlignment::Stretch)
                    .children([
                        crate::elevated_button::ElevatedButton::new(
                            Some(Listener::new(move |app| {
                                show(app, context, snack_bar());
                            })),
                            Some(Text::new(BUTTON_TEXT)),
                        )
                        .into_widget(),
                        crate::elevated_button::ElevatedButton::new(
                            Some(Listener::new(move |_app| completed.set(true))),
                            Some(Text::new(BUTTON_TEXT2)),
                        )
                        .into_widget(),
                    ])
                    .into_widget()
            }
        }));
        pump_app(tester, theme, home.into_widget());
        tap_text(tester, BUTTON_TEXT);
        tester.pump_and_settle();
        assert_eq!(count_text(tester, SNACKBAR_CONTENT), 1);
        let top_left = text_rect(tester, BUTTON_TEXT2).top_left();
        tester.tap(top_left, 1);
        assert_eq!(count_text(tester, SNACKBAR_CONTENT), 1);
        completed.get()
    }

    // Dart: 'Tap on button behind snack bar defined by width'.
    #[test]
    fn a_button_beside_a_snack_bar_of_custom_width_can_be_tapped() {
        test_widgets(|_| {
            assert!(taps_behind_the_snack_bar(m3, || {
                SnackBar::new(Text::new("Snackbar"))
                    .behavior(SnackBarBehavior::Floating)
                    .width(100.0)
            }));
        });
    }

    // Dart: 'Tap on button behind snack bar defined by margin'.
    #[test]
    fn a_button_in_a_snack_bar_s_margin_can_be_tapped() {
        test_widgets(|_| {
            assert!(taps_behind_the_snack_bar(m3, || {
                SnackBar::new(Text::new("Snackbar"))
                    .behavior(SnackBarBehavior::Floating)
                    .margin(EdgeInsets::only(100.0, 0.0, 0.0, 0.0))
            }));
        });
    }

    // Dart: "Can't tap on button behind snack bar defined by margin and HitTestBehavior.opaque".
    #[test]
    fn a_button_in_the_margin_of_an_opaque_snack_bar_cannot_be_tapped() {
        test_widgets(|_| {
            assert!(!taps_behind_the_snack_bar(m3, || {
                SnackBar::new(Text::new("Snackbar"))
                    .hit_test_behavior(HitTestBehavior::Opaque)
                    .behavior(SnackBarBehavior::Floating)
                    .margin(EdgeInsets::only(100.0, 0.0, 0.0, 0.0))
            }));
        });
    }

    // Dart: 'Can interact with widgets behind SnackBar when insetPadding is set in
    // SnackBarThemeData'.
    #[test]
    fn a_button_in_the_theme_s_inset_padding_can_be_tapped() {
        test_widgets(|_| {
            assert!(taps_behind_the_snack_bar(
                |app| {
                    ThemeData::builder()
                        .snack_bar_theme(
                            SnackBarThemeData::new()
                                .inset_padding(EdgeInsets::only(100.0, 0.0, 0.0, 0.0)),
                        )
                        .build(app)
                },
                || SnackBar::new(Text::new("Snackbar")).behavior(SnackBarBehavior::Floating),
            ));
        });
    }

    // Dart: 'Action text button uses correct overlay color'.
    #[test]
    #[allow(deprecated)]
    fn the_action_s_overlay_is_the_inverse_primary_colour() {
        test_widgets(|tester| {
            let home = x_scaffold(|app, context| {
                show(
                    app,
                    context,
                    SnackBar::new(Text::new("I am a snack bar."))
                        .action(SnackBarAction::new("ACTION", |_app| {})),
                );
            });
            pump_app(tester, m3, home.into_widget());
            tap_text(tester, "X");
            tester.pump_and_settle();
            let theme = m3(&tester.cell.borrow());
            let overlay_color = action_button_style(tester)
                .overlay_color
                .expect("an overlay color")
                .resolve(&states(&[WidgetState::Hovered]));
            assert_eq!(
                overlay_color,
                Some(theme.color_scheme.inverse_primary().with_opacity(0.08))
            );
        });
    }

    /// The persist cases: an elevated button showing `snack_bar`.
    fn elevated_button_scaffold(snack_bar: impl Fn() -> SnackBar + 'static) -> Scaffold {
        let snack_bar = Rc::new(snack_bar);
        Scaffold::new().body(Builder::new(move |_app, context| {
            let snack_bar = Rc::clone(&snack_bar);
            crate::elevated_button::ElevatedButton::new(
                Some(Listener::new(move |app| {
                    show(app, context, snack_bar());
                })),
                Some(Text::new("Show snackbar")),
            )
            .into_widget()
        }))
    }

    // Dart: 'Setting persist to true prevents timeout'.
    #[test]
    fn a_persisting_snack_bar_does_not_time_out() {
        test_widgets(|tester| {
            const SNACKBAR_CONTENT: &str = "Snackbar";
            let home = elevated_button_scaffold(|| {
                SnackBar::new(Text::new(SNACKBAR_CONTENT))
                    .duration(Duration::from_secs(1))
                    .persist(true)
                    .show_close_icon(true)
            });
            pump_app(tester, m3, home.into_widget());
            tap_text(tester, "Show snackbar");
            tester.pump_by(ms(750));
            // The snackbar shows up before the timeout.
            assert_eq!(count_text(tester, SNACKBAR_CONTENT), 1);
            tester.pump_by(ms(1500));
            tester.pump_and_settle();
            // The snackbar is still there after the timeout.
            assert_eq!(count_text(tester, SNACKBAR_CONTENT), 1);
            let close = tester.find_by_icon(&Icons::close());
            let center = tester.global_rect(tester.render_box_of(close)).center();
            tester.tap(center, 1);
            tester.pump_and_settle();
            // The snackbar is dismissed.
            assert_eq!(count_text(tester, SNACKBAR_CONTENT), 0);
        });
    }

    // Dart: 'Setting persist to false so snackbar auto dismisses'.
    #[test]
    fn a_snack_bar_that_does_not_persist_times_out_despite_its_close_icon() {
        test_widgets(|tester| {
            const SNACKBAR_CONTENT: &str = "SnackbarContent";
            let home = elevated_button_scaffold(|| {
                SnackBar::new(Text::new(SNACKBAR_CONTENT))
                    .duration(Duration::from_secs(1))
                    .persist(false)
                    .show_close_icon(true)
            });
            pump_app(tester, m3, home.into_widget());
            let button = rect_of_type::<crate::elevated_button::ElevatedButton>(tester).center();
            tester.tap(button, 1);
            tester.pump();
            tester.pump_by(ms(750));
            // The snackbar shows up before the timeout.
            assert_eq!(count_text(tester, SNACKBAR_CONTENT), 1);
            tester.pump_by(ms(1500));
            tester.pump_and_settle();
            // The snackbar auto dismisses after the timeout.
            assert_eq!(count_text(tester, SNACKBAR_CONTENT), 0);
        });
    }

    // Dart: 'Setting persist to false overrides accessibleNavigation'.
    #[test]
    fn a_snack_bar_that_does_not_persist_times_out_under_accessible_navigation() {
        test_widgets(|tester| {
            const SNACKBAR_CONTENT: &str = "SnackbarContent";
            let theme = m3(&tester.cell.borrow());
            tester.pump_widget(
                waft_widgets::MediaQuery::new(
                    waft_widgets::MediaQueryData::new().accessible_navigation(true),
                    material_app(
                        theme,
                        elevated_button_scaffold(|| {
                            SnackBar::new(Text::new(SNACKBAR_CONTENT))
                                .duration(Duration::from_secs(1))
                                .persist(false)
                                .action(SnackBarAction::new("Action", |_app| {}))
                        })
                        .into_widget(),
                    ),
                )
                .into_widget(),
            );
            let button = rect_of_type::<crate::elevated_button::ElevatedButton>(tester).center();
            tester.tap(button, 1);
            tester.pump_and_settle();
            assert_eq!(count_text(tester, SNACKBAR_CONTENT), 1);
            tester.pump_by(Duration::from_secs(10));
            tester.pump_and_settle();
            // The snackbar auto dismisses after the timeout.
            assert_eq!(count_text(tester, SNACKBAR_CONTENT), 0);
        });
    }

    // Dart: 'SnackBarAction does not crash at zero area'.
    #[test]
    fn a_snack_bar_action_lays_out_at_zero_area() {
        test_widgets(|tester| {
            pump_app(
                tester,
                m3,
                waft_widgets::Center::new()
                    .child(SizedBox::shrink().child(SnackBarAction::new("X", |_app| {})))
                    .into_widget(),
            );
            assert_eq!(
                rect_of_type::<SnackBarAction>(tester).size(),
                waft_embedder::Size::ZERO
            );
        });
    }

    // Dart: 'SnackBar action overflow calculation respects MediaQuery textScaler'.
    #[test]
    fn the_action_overflow_measures_the_label_with_the_text_scaler() {
        test_widgets(|tester| {
            let build_snack_bar =
                |tester: &mut WidgetTester, text_scaler: waft_painting::TextScaler| {
                    let home = waft_widgets::MediaQuery::new(
                        waft_widgets::MediaQueryData::new()
                            .size(waft_embedder::Size::new(800.0, 600.0))
                            .text_scaler(text_scaler),
                        Scaffold::new().body(Builder::new(|_app, context| {
                            GestureDetector::new()
                                .on_tap(Listener::new(move |app| {
                                    show(
                                        app,
                                        context,
                                        SnackBar::new(Text::new("Message"))
                                            .action(SnackBarAction::new("Action Label", |_app| {})),
                                    );
                                }))
                                .child(Text::new("Show"))
                                .into_widget()
                        })),
                    );
                    pump_app(tester, m3, home.into_widget());
                };

            // With no scaling, the action label fits on the same row as content.
            build_snack_bar(tester, waft_painting::TextScaler::NO_SCALING);
            tap_text(tester, "Show");
            tester.pump_and_settle();
            let action_top_left1 = text_rect(tester, "Action Label").top_left();
            // Action and content are on the same line (action is not below content).
            assert!(action_top_left1.dy() <= text_rect(tester, "Message").bottom_left().dy());
            {
                let show = tester.find_text("Show")[0];
                let mut app = tester.cell.borrow_mut();
                let context = show;
                ScaffoldMessenger::of(&mut app, context).clear_snack_bars(&mut app);
            }
            tester.pump_and_settle();

            // With a large text scaler, the action button width is scaled up and overflows
            // to a separate row below the content.
            build_snack_bar(tester, waft_painting::TextScaler::linear(3.0));
            tap_text(tester, "Show");
            tester.pump_and_settle();
            let content_bottom_left2 = text_rect(tester, "Message").bottom_left();
            let action_top_left2 = text_rect(tester, "Action Label").top_left();
            // Action overflows and is positioned below the content text.
            assert!(action_top_left2.dy() >= content_bottom_left2.dy());
        });
    }

    /// The overflowing-action cases, on a 500x800 view: the snack bar's content row and the
    /// rect of its 20 high content box.
    fn overflowing_action(padding: Option<EdgeInsets>) -> (WidgetTester, Rect) {
        let content_key = GlobalKey::new();
        let mut owned = WidgetTester::with_view(
            waft_test::TestPlatform::new().on(TargetPlatform::Android),
            waft_test::TestView::with_pixel_ratio(500.0, 800.0, 1.0),
        );
        let tester = &mut owned;
        let home = Scaffold::new().body(Builder::new({
            let content_key = content_key.clone();
            move |_app, context| {
                let content_key = content_key.clone();
                GestureDetector::new()
                    .on_tap(Listener::new(move |app| {
                        let mut snack_bar = SnackBar::new(
                            SizedBox::new().key(content_key.clone().into()).height(20.0),
                        )
                        .action(SnackBarAction::new("Overflow Action", |_app| {}))
                        .action_overflow_threshold(0.1);
                        if let Some(padding) = padding {
                            snack_bar = snack_bar.padding(padding);
                        }
                        show(app, context, snack_bar);
                    }))
                    .child(Text::new("Show"))
                    .into_widget()
            }
        }));
        pump_app(tester, m3, home.into_widget());
        tap_text(tester, "Show");
        tester.pump_and_settle();
        let content = rect_of_key(tester, &content_key);
        (owned, content)
    }

    /// Dart's `tester.widget<Row>(find.descendant(of: find.byType(SnackBar), matching:
    /// find.byType(Row)).first).children`.
    fn content_row_children(tester: &WidgetTester) -> Vec<WidgetRef> {
        let snack_bar = tester.find_by_type::<SnackBar>()[0];
        let row = tester
            .find_by_type::<Row>()
            .into_iter()
            .find(|row| tester.ancestors(*row).contains(&snack_bar))
            .expect("a SnackBar lays its content out in a Row");
        tester.widget(row, |row: &Row| row.children.clone())
    }

    // Dart: 'SnackBar does not allocate 40% empty space on right when action overflows'.
    #[test]
    fn an_overflowing_action_leaves_only_the_end_padding_beside_the_content() {
        test_widgets(|_| {
            const SCREEN_WIDTH: f64 = 500.0;
            let (mut owned, content) = overflowing_action(None);
            let tester = &mut owned;
            // For screenWidth = 500, horizontalPadding = 24 on start and end, the content width is 452px (500 - 24 - 24).
            assert_eq!(content.width(), 452.0);
            assert_eq!(content.top_right().dx(), SCREEN_WIDTH - 24.0);
            // Verify the spacer SizedBox has width equal to horizontalPadding (24.0), not 40% of snackBarWidth (188.0).
            let children = content_row_children(tester);
            assert_eq!(children.len(), 2);
            assert!(waft_widgets::downcast_widget::<Expanded>(&*children[0]).is_some());
            let spacer = waft_widgets::downcast_widget::<SizedBox>(&*children[1])
                .expect("a SizedBox spacer");
            assert_eq!(spacer.width, Some(24.0));
        });
    }

    // Dart: 'SnackBar respects custom padding when action overflows'.
    #[test]
    fn an_overflowing_action_keeps_a_custom_padding_without_a_spacer() {
        test_widgets(|_| {
            const SCREEN_WIDTH: f64 = 500.0;
            let custom_padding = EdgeInsets::only(36.0, 0.0, 48.0, 0.0);
            let (mut owned, content) = overflowing_action(Some(custom_padding));
            let tester = &mut owned;
            // When a custom padding is provided, it is applied on both sides by the outer
            // Padding, so no extra spacer is inserted in the content row.
            let children = content_row_children(tester);
            assert_eq!(children.len(), 1);
            assert!(waft_widgets::downcast_widget::<Expanded>(&*children[0]).is_some());
            assert_eq!(
                content.width(),
                SCREEN_WIDTH - (custom_padding.left + custom_padding.right)
            );
            assert_eq!(content.top_left().dx(), custom_padding.left);
            assert_eq!(
                content.top_right().dx(),
                SCREEN_WIDTH - custom_padding.right
            );
        });
    }

    // From snack_bar_theme_test.dart.

    /// Dart's `_snackBarTheme(showCloseIcon:)`.
    fn snack_bar_theme(show_close_icon: Option<bool>) -> SnackBarThemeData {
        let mut theme = SnackBarThemeData::new()
            .background_color(Colors::ORANGE.primary())
            .action_text_color(Colors::GREEN.primary())
            .content_text_style(waft_painting::TextStyle::new().color(Colors::BLUE.primary()))
            .elevation(12.0)
            .shape(waft_painting::BeveledRectangleBorder::new(
                BorderSide::NONE,
                BorderRadiusGeometry::all(Radius::circular(12.0)),
            ));
        if let Some(show_close_icon) = show_close_icon {
            theme = theme.show_close_icon(show_close_icon);
        }
        theme
    }

    /// Dart's `_getSnackBarMaterial(tester)`, read by `read`.
    fn with_snack_bar_material<R>(tester: &WidgetTester, read: impl FnOnce(&Material) -> R) -> R {
        tester.widget(snack_bar_material(tester), read)
    }

    /// Dart's `_getSnackBarTextRenderObject(tester, text).text.style`.
    fn paragraph_style(tester: &WidgetTester, text: &str) -> Option<waft_painting::TextStyle> {
        let element = tester.find_text(text)[0];
        let render = tester.render_box_of(element);
        let app = tester.cell.borrow();
        let paragraph = render
            .as_object()
            .downcast::<waft_rendering::RenderParagraph>(&app)
            .expect("a Text renders a paragraph");
        paragraph.text(&app).style().cloned()
    }

    /// Dart's `_getSnackBarIconFinder(tester)`, counted.
    fn close_icons_in_snack_bar(tester: &WidgetTester) -> usize {
        let snack_bar = tester.find_by_type::<SnackBar>()[0];
        tester
            .find_by_type::<Icon>()
            .into_iter()
            .filter(|icon| {
                tester.ancestors(*icon).contains(&snack_bar)
                    && tester.widget(*icon, |icon: &Icon| icon.icon == Some(Icons::close()))
            })
            .count()
    }

    // Dart: 'Material2 - Passing no SnackBarThemeData returns defaults'.
    #[test]
    fn a_material_2_snack_bar_with_no_theme_takes_the_defaults() {
        test_widgets(|tester| {
            const TEXT: &str = "I am a snack bar.";
            let home = x_scaffold(|app, context| {
                show(
                    app,
                    context,
                    SnackBar::new(Text::new(TEXT))
                        .duration(Duration::from_secs(2))
                        .action(SnackBarAction::new("ACTION", |_app| {})),
                );
            });
            pump_app(tester, m2, home.into_widget());
            tap_text(tester, "X");
            tester.pump_and_settle();
            assert_eq!(
                paragraph_style(tester, TEXT),
                crate::typography::Typography::material2018()
                    .build()
                    .white
                    .title_medium
            );
            with_snack_bar_material(tester, |material| {
                assert!(crate::test_support::is_same_color_as(
                    material.color.expect("a colour"),
                    Color::new(0xFF333333)
                ));
                assert_eq!(material.elevation, 6.0);
                assert!(material.shape.is_none());
            });
        });
    }

    // Dart: 'Material3 - Passing no SnackBarThemeData returns defaults'.
    #[test]
    fn a_material_3_snack_bar_with_no_theme_takes_the_defaults() {
        test_widgets(|tester| {
            const TEXT: &str = "I am a snack bar.";
            let home = x_scaffold(|app, context| {
                show(
                    app,
                    context,
                    SnackBar::new(Text::new(TEXT))
                        .duration(Duration::from_secs(2))
                        .action(SnackBarAction::new("ACTION", |_app| {})),
                );
            });
            pump_app(tester, m3, home.into_widget());
            tap_text(tester, "X");
            tester.pump_and_settle();
            let theme = m3(&tester.cell.borrow());
            let typography = crate::typography::Typography::material2021().build();
            let expected = typography
                .english_like
                .body_medium
                .as_ref()
                .map(|body_medium| body_medium.merge(typography.black.body_medium.as_ref()))
                .map(|style| {
                    style
                        .copy_with()
                        .color(theme.color_scheme.on_inverse_surface())
                        .decoration_color(theme.color_scheme.on_surface)
                });
            assert_eq!(paragraph_style(tester, TEXT), expected);
            with_snack_bar_material(tester, |material| {
                assert_eq!(material.color, Some(theme.color_scheme.inverse_surface()));
                assert_eq!(material.elevation, 6.0);
                assert!(material.shape.is_none());
            });
        });
    }

    // Dart: 'SnackBar uses values from SnackBarThemeData'.
    #[test]
    fn a_snack_bar_takes_its_values_from_the_theme() {
        test_widgets(|tester| {
            const TEXT: &str = "I am a snack bar.";
            const ACTION: &str = "ACTION";
            let home = x_scaffold(|app, context| {
                show(
                    app,
                    context,
                    SnackBar::new(Text::new(TEXT))
                        .duration(Duration::from_secs(2))
                        .action(SnackBarAction::new(ACTION, |_app| {})),
                );
            });
            pump_app(
                tester,
                |app| {
                    ThemeData::builder()
                        .snack_bar_theme(snack_bar_theme(Some(true)))
                        .build(app)
                },
                home.into_widget(),
            );
            tap_text(tester, "X");
            tester.pump_and_settle();
            let theme = snack_bar_theme(Some(true));
            assert_eq!(paragraph_style(tester, TEXT), theme.content_text_style);
            with_snack_bar_material(tester, |material| {
                assert_eq!(material.color, theme.background_color);
                assert_eq!(Some(material.elevation), theme.elevation);
                assert!(
                    material
                        .shape
                        .as_deref()
                        .expect("a shape")
                        .eq_shape(theme.shape.as_deref().expect("a shape"))
                );
            });
            assert_eq!(
                Some(tester.text_color(ACTION)),
                theme.action_text_color.as_ref().map(AnyColor::color)
            );
            assert_eq!(close_icons_in_snack_bar(tester), 1);
        });
    }

    /// The priority cases: a floating 400 wide snack bar whose own values beat the theme's,
    /// under the theme `wrap` puts around the scaffold or in `theme`.
    fn widget_values_win(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        wrap: impl FnOnce(Scaffold) -> WidgetRef,
        configure: impl Fn(SnackBar) -> SnackBar + 'static,
        action: impl Fn() -> SnackBarAction + 'static,
    ) {
        const SNACK_BAR_WIDTH: f64 = 400.0;
        let background_color = Colors::PURPLE.primary();
        let text_color = Colors::PINK.primary();
        let elevation = 7.0;
        let shape = || {
            RoundedRectangleBorder::new(
                BorderSide::NONE,
                BorderRadiusGeometry::all(Radius::circular(9.0)),
            )
        };
        let home = x_scaffold(move |app, context| {
            show(
                app,
                context,
                configure(SnackBar::new(Text::new("I am a snack bar.")))
                    .duration(Duration::from_secs(2))
                    .action(action()),
            );
        });
        pump_app(tester, theme, wrap(home));
        tap_text(tester, "X");
        tester.pump_and_settle();
        with_snack_bar_material(tester, |material| {
            assert_eq!(material.color, Some(background_color));
            assert_eq!(material.elevation, elevation);
            assert!(
                material
                    .shape
                    .as_deref()
                    .expect("a shape")
                    .eq_shape(&shape())
            );
        });
        assert_eq!(tester.text_color("ACTION"), text_color);
        assert_eq!(close_icons_in_snack_bar(tester), 0);
        // Assert width.
        assert_centred_with_width(tester, SNACK_BAR_WIDTH);
    }

    /// Dart's priority-case snack bar: purple, floating, 400 wide, 7 high, 9-rounded, no close
    /// icon.
    fn prioritized(snack_bar: SnackBar) -> SnackBar {
        snack_bar
            .background_color(Colors::PURPLE.primary())
            .behavior(SnackBarBehavior::Floating)
            .width(400.0)
            .elevation(7.0)
            .shape(RoundedRectangleBorder::new(
                BorderSide::NONE,
                BorderRadiusGeometry::all(Radius::circular(9.0)),
            ))
            .show_close_icon(false)
    }

    // Dart: 'SnackBar widget properties take priority over theme'.
    #[test]
    fn a_snack_bar_s_own_values_win_over_the_theme_s() {
        test_widgets(|tester| {
            widget_values_win(
                tester,
                |app| {
                    ThemeData::builder()
                        .snack_bar_theme(snack_bar_theme(Some(true)))
                        .build(app)
                },
                |scaffold| scaffold.into_widget(),
                prioritized,
                || SnackBarAction::new("ACTION", |_app| {}).text_color(Colors::PINK.primary()),
            );
        });
    }

    // Dart: 'Local SnackBarTheme properties are used'.
    #[test]
    fn a_snack_bar_takes_its_values_from_a_local_snack_bar_theme() {
        test_widgets(|tester| {
            widget_values_win(
                tester,
                m3,
                |scaffold| {
                    SnackBarTheme::new(
                        SnackBarThemeData::new()
                            .background_color(Colors::PURPLE.primary())
                            .behavior(SnackBarBehavior::Floating)
                            .width(400.0)
                            .elevation(7.0)
                            .shape(RoundedRectangleBorder::new(
                                BorderSide::NONE,
                                BorderRadiusGeometry::all(Radius::circular(9.0)),
                            ))
                            .show_close_icon(false)
                            .action_text_color(Colors::PINK.primary()),
                        scaffold,
                    )
                    .into_widget()
                },
                |snack_bar| snack_bar,
                || SnackBarAction::new("ACTION", |_app| {}),
            );
        });
    }

    // Dart: 'SnackBar widget properties take priority over local theme'.
    #[test]
    fn a_snack_bar_s_own_values_win_over_a_local_snack_bar_theme_s() {
        test_widgets(|tester| {
            widget_values_win(
                tester,
                m3,
                |scaffold| SnackBarTheme::new(snack_bar_theme(Some(true)), scaffold).into_widget(),
                prioritized,
                || SnackBarAction::new("ACTION", |_app| {}).text_color(Colors::PINK.primary()),
            );
        });
    }

    /// The theme's action background cases, from the 'X' scaffold.
    fn themed_action_background_before_and_after_press(
        tester: &mut WidgetTester,
        theme: SnackBarThemeData,
        action: impl Fn() -> SnackBarAction + 'static,
    ) -> (Option<Color>, Option<Color>) {
        let home = x_scaffold(move |app, context| {
            show(
                app,
                context,
                SnackBar::new(Text::new("I am a snack bar.")).action(action()),
            );
        });
        pump_app(
            tester,
            |app| ThemeData::builder().snack_bar_theme(theme).build(app),
            home.into_widget(),
        );
        tap_text(tester, "X");
        tester.pump_and_settle();
        let before = action_material_color(tester);
        tap_text(tester, "ACTION");
        tester.pump();
        let after = action_material_color(tester);
        (before, after)
    }

    fn by_disabled(disabled: Color, enabled: Color) -> WidgetStateColor {
        WidgetStateColor::resolve_with(move |states| {
            if states.contains(&WidgetState::Disabled) {
                return disabled;
            }
            enabled
        })
    }

    // Dart: 'SnackBarAction uses actionBackgroundColor'.
    #[test]
    fn a_snack_bar_action_takes_the_theme_s_action_background_color() {
        test_widgets(|tester| {
            let (before, after) = themed_action_background_before_and_after_press(
                tester,
                SnackBarThemeData::new().action_background_color(by_disabled(
                    Colors::BLUE.primary(),
                    Colors::PURPLE.primary(),
                )),
                || SnackBarAction::new("ACTION", |_app| {}),
            );
            assert_eq!(before, Some(Colors::PURPLE.primary()));
            assert_eq!(after, Some(Colors::BLUE.primary()));
        });
    }

    // Dart: 'SnackBarAction backgroundColor overrides SnackBarThemeData actionBackgroundColor'.
    #[test]
    fn a_snack_bar_action_s_background_color_wins_over_the_theme_s() {
        test_widgets(|tester| {
            let (before, after) = themed_action_background_before_and_after_press(
                tester,
                SnackBarThemeData::new().action_background_color(by_disabled(
                    Colors::BLUE.primary(),
                    Colors::PURPLE.primary(),
                )),
                || {
                    SnackBarAction::new("ACTION", |_app| {}).background_color(by_disabled(
                        Colors::AMBER.primary(),
                        Colors::CYAN.primary(),
                    ))
                },
            );
            assert_eq!(before, Some(Colors::CYAN.primary()));
            assert_eq!(after, Some(Colors::AMBER.primary()));
        });
    }

    // Dart: 'SnackBarThemeData asserts when actionBackgroundColor is a WidgetStateColor and
    // disabledActionBackgroundColor is also provided'.
    #[test]
    #[should_panic(
        expected = "disabledBackgroundColor must not be provided when background color is a \
                    WidgetStateColor"
    )]
    fn a_theme_with_a_widget_state_action_background_and_a_disabled_one_asserts() {
        test_widgets(|tester| {
            themed_action_background_before_and_after_press(
                tester,
                SnackBarThemeData::new()
                    .action_background_color(by_disabled(
                        Colors::BLUE.primary(),
                        Colors::PURPLE.primary(),
                    ))
                    .disabled_action_background_color(Colors::AMBER.primary()),
                || SnackBarAction::new("ACTION", |_app| {}),
            );
        });
    }

    /// Dart's `tester.firstRenderObject(find.byType(W))` as a box, in global coordinates.
    fn first_box_rect<W: 'static>(tester: &WidgetTester) -> Rect {
        rect_of_type::<W>(tester)
    }

    /// The theme behavior cases: the 'X' scaffold with a button, under a theme of `behavior`.
    fn themed_behavior(tester: &mut WidgetTester, behavior: SnackBarBehavior) {
        let home = x_scaffold(|app, context| {
            show(app, context, snack_bar_with_action());
        })
        .floating_action_button(fab_with_icon());
        pump_app(
            tester,
            move |app| {
                ThemeData::builder()
                    .snack_bar_theme(SnackBarThemeData::new().behavior(behavior))
                    .build(app)
            },
            home.into_widget(),
        );
    }

    // Dart: 'SnackBar theme behavior is correct for floating'.
    #[test]
    fn a_theme_floating_snack_bar_sits_on_top_of_the_button() {
        test_widgets(|tester| {
            themed_behavior(tester, SnackBarBehavior::Floating);
            tap_text(tester, "X");
            tester.pump_and_settle();
            let snack_bar_bottom_center = first_box_rect::<SnackBar>(tester).bottom_center();
            let floating_action_button_top_center =
                first_box_rect::<crate::floating_action_button::FloatingActionButton>(tester)
                    .top_center();
            // Since padding and margin is handled inside snackBarBox,
            // the bottom offset of snackbar should equal with top offset of FAB
            assert!(snack_bar_bottom_center.dy() == floating_action_button_top_center.dy());
        });
    }

    // Dart: 'SnackBar theme behavior is correct for fixed'.
    #[test]
    fn a_theme_fixed_snack_bar_pushes_the_button_above_it() {
        test_widgets(|tester| {
            themed_behavior(tester, SnackBarBehavior::Fixed);
            let floating_action_button_origin_bottom_center =
                first_box_rect::<crate::floating_action_button::FloatingActionButton>(tester)
                    .bottom_center();
            tap_text(tester, "X");
            tester.pump_and_settle();
            let snack_bar_top_center = first_box_rect::<SnackBar>(tester).top_center();
            let floating_action_button_bottom_center =
                first_box_rect::<crate::floating_action_button::FloatingActionButton>(tester)
                    .bottom_center();
            assert!(
                floating_action_button_origin_bottom_center.dy()
                    > floating_action_button_bottom_center.dy()
            );
            assert!(snack_bar_top_center.dy() > floating_action_button_bottom_center.dy());
        });
    }

    /// Dart's `buildApp(themedBehavior:, margin:, width:)` of snack_bar_theme_test.dart, with
    /// the snack bar's entrance run.
    fn themed_and_shown(
        tester: &mut WidgetTester,
        behavior: SnackBarBehavior,
        configure: fn(SnackBar) -> SnackBar,
    ) {
        let home = x_scaffold(move |app, context| {
            show(app, context, configure(snack_bar_with_action()));
        })
        .floating_action_button(fab_with_icon());
        pump_app(
            tester,
            move |app| {
                ThemeData::builder()
                    .snack_bar_theme(SnackBarThemeData::new().behavior(behavior))
                    .build(app)
            },
            home.into_widget(),
        );
        tap_x_and_settle_entrance(tester);
    }

    // Dart: 'SnackBar theme behavior will assert properly for margin use', the floating run.
    #[test]
    fn a_theme_floating_snack_bar_takes_a_margin() {
        test_widgets(|tester| {
            themed_and_shown(tester, SnackBarBehavior::Floating, |snack_bar| {
                snack_bar.margin(EdgeInsets::all(8.0))
            });
        });
    }

    // Dart: the same case, the fixed run.
    #[test]
    #[should_panic(
        expected = "Margin can only be used with floating behavior. SnackBarBehavior.fixed was \
                    set by the inherited SnackBarThemeData."
    )]
    fn a_theme_fixed_snack_bar_asserts_against_a_margin() {
        test_widgets(|tester| {
            themed_and_shown(tester, SnackBarBehavior::Fixed, |snack_bar| {
                snack_bar.margin(EdgeInsets::all(8.0))
            });
        });
    }

    // Dart: 'SnackBar theme behavior will assert properly for width use', the floating run.
    #[test]
    fn a_theme_floating_snack_bar_takes_a_width() {
        test_widgets(|tester| {
            themed_and_shown(tester, SnackBarBehavior::Floating, |snack_bar| {
                snack_bar.width(5.0)
            });
        });
    }

    // Dart: the same case, the fixed run.
    #[test]
    #[should_panic(
        expected = "Width can only be used with floating behavior. SnackBarBehavior.fixed was \
                    set by the inherited SnackBarThemeData."
    )]
    fn a_theme_fixed_snack_bar_asserts_against_a_width() {
        test_widgets(|tester| {
            themed_and_shown(tester, SnackBarBehavior::Fixed, |snack_bar| {
                snack_bar.width(5.0)
            });
        });
    }

    /// Dart's collector frees the curved animations `_SnackBarState` makes once the snack bar
    /// is gone; here they are freed with the state's references to them.
    #[test]
    fn a_dismissed_snack_bar_frees_its_curved_animations() {
        test_widgets(|tester| {
            let home = x_scaffold(|app, context| {
                show(
                    app,
                    context,
                    SnackBar::new(Text::new("Snack")).duration(Duration::from_secs(1)),
                );
            });
            pump_app(tester, m3, home.into_widget());
            tap_x_and_settle_entrance(tester);
            let animations = {
                let snack_bar = tester.find_by_type::<SnackBar>()[0];
                let app = tester.cell.borrow();
                let state = app.get(
                    snack_bar
                        .state_handle::<SnackBarState>(&app)
                        .expect("a SnackBar is stateful"),
                );
                [
                    state.height_animation.clone(),
                    state.fade_in_animation.clone(),
                    state.fade_in_m3_animation.clone(),
                    state.fade_out_animation.clone(),
                    state.height_m3_animation.clone(),
                ]
                .map(|animation| animation.expect("made in init_state").handle())
            };

            tester.pump_by(ms(1500)); // the timer dismisses the snack bar
            tester.pump(); // begin the reverse animation
            tester.pump_by(ms(750)); // its last frame removes the snack bar
            assert!(tester.find_by_type::<SnackBar>().is_empty());
            tester.cell.checkpoint();
            let app = tester.cell.borrow();
            assert!(animations.iter().all(|animation| !app.contains(*animation)));
        });
    }
}
