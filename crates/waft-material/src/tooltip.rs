//! Flutter counterpart: material_ui's `tooltip.dart`.

use std::fmt;
use std::rc::Rc;
use std::time::Duration;

use waft_embedder::{Brightness, Radius, TargetPlatform, TextAlign};
use waft_foundation::{App, Handle, Ref};
use waft_painting::{
    BorderRadius, BoxDecoration, DecorationRef, EdgeInsets, EdgeInsetsGeometry, InlineSpanRef,
    TextSpan, TextStyle, position_dependent_box,
};
use waft_rendering::BoxConstraints;
use waft_scheduler::{Ticker, TickerCallback, TickerProviderObject};
use waft_services::{MouseCursor, MouseCursorRef};
use waft_widgets::{
    BuildContext, Center, ConstrainedBox, Container, DefaultTextStyle, FadeTransition, GlobalKey,
    IntoWidget, KeyRef, MouseRegion, RawTooltip, RawTooltipState, SingleTickerProviderStateMixin,
    SingleTickerProviderStateMixinData, SizedBox, State, StateData, StatefulWidget,
    StatelessWidget, Text, TooltipPositionContext, TooltipPositionDelegate, TooltipTriggerMode,
    TooltipTriggeredCallback, WidgetRef,
};

use crate::colors::Colors;
use crate::theme::Theme;
use crate::tooltip_theme::{TooltipTheme, TooltipThemeData};
use crate::tooltip_visibility::TooltipVisibility;

/// A Material Design tooltip.
///
/// Tooltips provide text labels which help explain the function of a button or
/// other user interface action. Wrap the button in a [`Tooltip`] widget and provide
/// a message which will be shown when the widget is long pressed.
///
/// Many widgets, such as `IconButton`, `FloatingActionButton`, and
/// `PopupMenuButton` have a `tooltip` property that, when non-null, causes the
/// widget to include a [`Tooltip`] in its build.
///
/// Tooltips improve the accessibility of visual widgets by proving a textual
/// representation of the widget, which, for example, can be vocalized by a
/// screen reader.
///
/// This tooltip will default to showing above its child instead of below when its ambient
/// [`TooltipThemeData::prefer_below`] is false. Setting that piece of theme data is
/// recommended to avoid having a finger or cursor hide the tooltip. For other ways to set
/// that piece of theme data see:
///
/// * `Theme::data`, `ThemeData::tooltip_theme`
/// * [`TooltipTheme::data`]
///
/// or it can be set directly on each tooltip with [`Tooltip::prefer_below`].
///
/// A tooltip can be shown manually with [`TooltipTriggerMode::Manual`] by calling the
/// [`TooltipState::ensure_tooltip_visible`] function.
///
/// See also:
///
///  * <https://material.io/design/components/tooltips.html>
///  * [`TooltipTheme`] or `ThemeData::tooltip_theme`
///  * [`TooltipVisibility`]
pub struct Tooltip {
    pub key: Option<KeyRef>,

    /// The text to display in the tooltip.
    ///
    /// Only one of [`message`](Self::message) and [`rich_message`](Self::rich_message) may be
    /// non-null.
    pub message: Option<String>,

    /// The rich text to display in the tooltip.
    ///
    /// Only one of [`message`](Self::message) and [`rich_message`](Self::rich_message) may be
    /// non-null.
    pub rich_message: Option<InlineSpanRef>,

    /// The minimum height of the [`Tooltip`]'s message.
    #[deprecated(
        note = "Use Tooltip.constraints instead. This feature was deprecated after v3.30.0-0.1.pre."
    )]
    pub height: Option<f64>,

    /// Constrains the size of the [`Tooltip`]'s message.
    ///
    /// If null, then the [`TooltipThemeData::constraints`] of the ambient
    /// `ThemeData::tooltip_theme` will be used. If that is also null, then a default value
    /// will be picked based on the current platform. For desktop platforms, the default value
    /// is `BoxConstraints(minHeight: 24.0)`, while for mobile platforms the default value is
    /// `BoxConstraints(minHeight: 32.0)`.
    pub constraints: Option<BoxConstraints>,

    /// The amount of space by which to inset the [`Tooltip`]'s message.
    ///
    /// On mobile, defaults to 16.0 logical pixels horizontally and 4.0 vertically.
    /// On desktop, defaults to 8.0 logical pixels horizontally and 4.0 vertically.
    pub padding: Option<EdgeInsetsGeometry>,

    /// The empty space that surrounds the tooltip.
    ///
    /// Defines the tooltip's outer `Container::margin`. By default, a
    /// long tooltip will span the width of its window. If long enough,
    /// a tooltip might also span the window's height. This property allows
    /// one to define how much space the tooltip must be inset from the edges
    /// of their display window.
    ///
    /// If this property is null, then [`TooltipThemeData::margin`] is used.
    /// If [`TooltipThemeData::margin`] is also null, the default margin is
    /// 0.0 logical pixels on all sides.
    ///
    /// See also:
    ///
    ///  * [`constraints`](Self::constraints), which allow setting an explicit size for the
    ///    tooltip.
    pub margin: Option<EdgeInsetsGeometry>,

    /// The vertical gap between the widget and the displayed tooltip.
    ///
    /// When [`prefer_below`](Self::prefer_below) is set to true and tooltips have sufficient
    /// space to display themselves, this property defines how much vertical space
    /// tooltips will position themselves under their corresponding widgets.
    /// Otherwise, tooltips will position themselves above their corresponding
    /// widgets with the given offset.
    pub vertical_offset: Option<f64>,

    /// Whether the tooltip defaults to being displayed below the widget.
    ///
    /// If there is insufficient space to display the tooltip in
    /// the preferred direction, the tooltip will be displayed in the opposite
    /// direction.
    ///
    /// If this property is null, then [`TooltipThemeData::prefer_below`] is used.
    /// If that is also null, the default value is true.
    ///
    /// Applying [`TooltipThemeData::prefer_below`]: `false` for the entire app
    /// is recommended to avoid having a finger or cursor hide a tooltip.
    pub prefer_below: Option<bool>,

    /// Whether the tooltip's [`message`](Self::message) or
    /// [`rich_message`](Self::rich_message) should be excluded from the semantics tree.
    ///
    /// Defaults to false. A tooltip will add a `Semantics` label that is set to
    /// [`Tooltip::message`] if non-null, or the plain text value of
    /// [`Tooltip::rich_message`] otherwise. Set this property to true if the app is
    /// going to provide its own custom semantics label.
    pub exclude_from_semantics: Option<bool>,

    /// The widget below this widget in the tree.
    pub child: Option<WidgetRef>,

    /// Specifies the tooltip's shape and background color.
    ///
    /// The tooltip shape defaults to a rounded rectangle with a border radius of
    /// 4.0. Tooltips will also default to an opacity of 90% and with the color
    /// `Colors::GREY[700]` if `ThemeData::brightness` is [`Brightness::Light`], and
    /// `Colors::WHITE` if it is [`Brightness::Dark`].
    pub decoration: Option<DecorationRef>,

    /// The style to use for the message of the tooltip.
    ///
    /// If null, the message's [`TextStyle`] will be determined based on
    /// `ThemeData`. If `ThemeData::brightness` is set to [`Brightness::Dark`],
    /// `TextTheme::body_medium` of `ThemeData::text_theme` will be used with
    /// `Colors::WHITE`. Otherwise, if `ThemeData::brightness` is set to
    /// [`Brightness::Light`], `TextTheme::body_medium` of `ThemeData::text_theme` will be
    /// used with `Colors::BLACK`.
    pub text_style: Option<TextStyle>,

    /// How the message of the tooltip is aligned horizontally.
    ///
    /// If this property is null, then [`TooltipThemeData::text_align`] is used.
    /// If [`TooltipThemeData::text_align`] is also null, the default value is
    /// [`TextAlign::Start`].
    pub text_align: Option<TextAlign>,

    /// The length of time that a pointer must hover over a tooltip's widget
    /// before the tooltip will be shown.
    ///
    /// Defaults to 0 milliseconds (the tooltip is shown immediately upon hover).
    pub wait_duration: Option<Duration>,

    /// The length of time that the tooltip will be shown after a long press is
    /// released (if trigger_mode is [`TooltipTriggerMode::LongPress`]) or a tap is
    /// released (if trigger_mode is [`TooltipTriggerMode::Tap`]).
    ///
    /// This property does not affect mouse pointer devices.
    ///
    /// Defaults to 1.5 seconds (the tooltip is shown 1.5 seconds after a tap or
    /// long press is released).
    ///
    /// See also:
    ///
    ///  * [`exit_duration`](Self::exit_duration), which allows configuring the time until a
    ///    pointer disappears when hovering.
    pub show_duration: Option<Duration>,

    /// The length of time that a pointer must have stopped hovering over a
    /// tooltip's widget before the tooltip will be hidden.
    ///
    /// Defaults to 100 milliseconds.
    ///
    /// See also:
    ///
    ///  * [`show_duration`](Self::show_duration), which allows configuring the length of time
    ///    that a tooltip will be visible after touch events are released.
    pub exit_duration: Option<Duration>,

    /// Whether the tooltip can be dismissed by tap.
    ///
    /// The default value is true.
    pub enable_tap_to_dismiss: bool,

    /// The [`TooltipTriggerMode`] that will show the tooltip.
    ///
    /// This property does not affect mouse devices. Setting
    /// [`trigger_mode`](Self::trigger_mode) to [`TooltipTriggerMode::Manual`] will not prevent
    /// the tooltip from showing when the mouse cursor hovers over it.
    ///
    /// If this property is null, then [`TooltipThemeData::trigger_mode`] is used.
    /// If [`TooltipThemeData::trigger_mode`] is also null, the default mode is
    /// [`TooltipTriggerMode::LongPress`].
    pub trigger_mode: Option<TooltipTriggerMode>,

    /// Whether the tooltip should provide acoustic and/or haptic feedback.
    ///
    /// For example, on Android a tap will produce a clicking sound and a
    /// long-press will produce a short vibration, when feedback is enabled.
    ///
    /// When null, the default value is true.
    ///
    /// See also:
    ///
    ///  * `Feedback`, for providing platform-specific feedback to certain
    ///    actions.
    pub enable_feedback: Option<bool>,

    /// Called when the [`Tooltip`] is triggered programmatically, on tap, or on
    /// long press.
    ///
    /// The tooltip is triggered after a tap when [`trigger_mode`](Self::trigger_mode) is
    /// [`TooltipTriggerMode::Tap`] or after a long press when
    /// [`trigger_mode`](Self::trigger_mode) is [`TooltipTriggerMode::LongPress`].
    ///
    /// Hovering over the tooltip with a mouse pointer does not trigger this
    /// callback.
    pub on_triggered: Option<TooltipTriggeredCallback>,

    /// The cursor for a mouse pointer when it enters or is hovering over the
    /// widget.
    ///
    /// If this property is null, `MouseCursor::defer` will be used.
    pub mouse_cursor: Option<MouseCursorRef>,

    /// Whether this tooltip should be invisible to hit testing.
    ///
    /// If no value is passed, pointer events are ignored unless the tooltip has a
    /// [`rich_message`](Self::rich_message) instead of a [`message`](Self::message).
    ///
    /// See also:
    ///
    /// * `IgnorePointer`, for more information about how pointer events are
    ///   handled or ignored.
    pub ignore_pointer: Option<bool>,

    /// A custom position delegate function for computing where the tooltip should
    /// be positioned.
    ///
    /// If provided, this function will be called with a [`TooltipPositionContext`]
    /// containing all the necessary information for positioning the tooltip. The
    /// function should return an [`Offset`](waft_embedder::Offset) indicating where to place the tooltip
    /// in the closest `Overlay`.
    ///
    /// If null, defaults to positioning the tooltip center-aligned and below the
    /// target using [`position_dependent_box`].
    pub position_delegate: Option<TooltipPositionDelegate>,
}

#[allow(deprecated)]
impl Default for Tooltip {
    fn default() -> Tooltip {
        Tooltip {
            key: None,
            message: None,
            rich_message: None,
            height: None,
            constraints: None,
            padding: None,
            margin: None,
            vertical_offset: None,
            prefer_below: None,
            exclude_from_semantics: None,
            child: None,
            decoration: None,
            text_style: None,
            text_align: None,
            wait_duration: None,
            show_duration: None,
            exit_duration: None,
            enable_tap_to_dismiss: true,
            trigger_mode: None,
            enable_feedback: None,
            on_triggered: None,
            mouse_cursor: None,
            ignore_pointer: None,
            position_delegate: None,
        }
    }
}

#[allow(deprecated)]
impl Tooltip {
    /// Creates a tooltip.
    ///
    /// By default, tooltips should adhere to the
    /// [Material specification](https://material.io/design/components/tooltips.html#spec).
    /// If the optional constructor parameters are not defined, the values
    /// provided by [`TooltipTheme::of`] will be used if a [`TooltipTheme`] is present
    /// or specified in `ThemeData`.
    ///
    /// All parameters that are defined in the constructor will
    /// override the default values _and_ the values in [`TooltipTheme::of`].
    ///
    /// Exactly one of [`message`](Self::message) and [`rich_message`](Self::rich_message) must
    /// be given; the tooltip's state checks it when it builds.
    pub fn new() -> Tooltip {
        Tooltip::default()
    }

    /// Dart `Tooltip(key:)`.
    pub fn key(mut self, key: KeyRef) -> Tooltip {
        self.key = Some(key);
        self
    }

    /// Dart `Tooltip(message:)`.
    pub fn message(mut self, message: impl Into<String>) -> Tooltip {
        debug_assert!(
            self.rich_message.is_none(),
            "Either `message` or `richMessage` must be specified"
        );
        self.message = Some(message.into());
        self
    }

    /// Dart `Tooltip(richMessage:)`.
    pub fn rich_message(mut self, rich_message: InlineSpanRef) -> Tooltip {
        debug_assert!(
            self.message.is_none(),
            "Either `message` or `richMessage` must be specified"
        );
        self.rich_message = Some(rich_message);
        self
    }

    /// Dart `Tooltip(height:)`.
    #[deprecated(
        note = "Use Tooltip.constraints instead. This feature was deprecated after v3.30.0-0.1.pre."
    )]
    pub fn height(mut self, height: f64) -> Tooltip {
        debug_assert!(
            self.constraints.is_none(),
            "Only one of `height` and `constraints` may be specified."
        );
        self.height = Some(height);
        self
    }

    /// Dart `Tooltip(constraints:)`.
    pub fn constraints(mut self, constraints: BoxConstraints) -> Tooltip {
        debug_assert!(
            self.height.is_none(),
            "Only one of `height` and `constraints` may be specified."
        );
        self.constraints = Some(constraints);
        self
    }

    /// Dart `Tooltip(padding:)`.
    pub fn padding(mut self, padding: EdgeInsetsGeometry) -> Tooltip {
        self.padding = Some(padding);
        self
    }

    /// Dart `Tooltip(margin:)`.
    pub fn margin(mut self, margin: EdgeInsetsGeometry) -> Tooltip {
        self.margin = Some(margin);
        self
    }

    /// Dart `Tooltip(verticalOffset:)`.
    pub fn vertical_offset(mut self, vertical_offset: f64) -> Tooltip {
        self.vertical_offset = Some(vertical_offset);
        self
    }

    /// Dart `Tooltip(preferBelow:)`.
    pub fn prefer_below(mut self, prefer_below: bool) -> Tooltip {
        self.prefer_below = Some(prefer_below);
        self
    }

    /// Dart `Tooltip(excludeFromSemantics:)`.
    pub fn exclude_from_semantics(mut self, exclude_from_semantics: bool) -> Tooltip {
        self.exclude_from_semantics = Some(exclude_from_semantics);
        self
    }

    /// Dart `Tooltip(decoration:)`.
    pub fn decoration(mut self, decoration: impl Into<DecorationRef>) -> Tooltip {
        self.decoration = Some(decoration.into());
        self
    }

    /// Dart `Tooltip(textStyle:)`.
    pub fn text_style(mut self, text_style: TextStyle) -> Tooltip {
        self.text_style = Some(text_style);
        self
    }

    /// Dart `Tooltip(textAlign:)`.
    pub fn text_align(mut self, text_align: TextAlign) -> Tooltip {
        self.text_align = Some(text_align);
        self
    }

    /// Dart `Tooltip(waitDuration:)`.
    pub fn wait_duration(mut self, wait_duration: Duration) -> Tooltip {
        self.wait_duration = Some(wait_duration);
        self
    }

    /// Dart `Tooltip(showDuration:)`.
    pub fn show_duration(mut self, show_duration: Duration) -> Tooltip {
        self.show_duration = Some(show_duration);
        self
    }

    /// Dart `Tooltip(exitDuration:)`.
    pub fn exit_duration(mut self, exit_duration: Duration) -> Tooltip {
        self.exit_duration = Some(exit_duration);
        self
    }

    /// Dart `Tooltip(enableTapToDismiss:)`.
    pub fn enable_tap_to_dismiss(mut self, enable_tap_to_dismiss: bool) -> Tooltip {
        self.enable_tap_to_dismiss = enable_tap_to_dismiss;
        self
    }

    /// Dart `Tooltip(triggerMode:)`.
    pub fn trigger_mode(mut self, trigger_mode: TooltipTriggerMode) -> Tooltip {
        self.trigger_mode = Some(trigger_mode);
        self
    }

    /// Dart `Tooltip(enableFeedback:)`.
    pub fn enable_feedback(mut self, enable_feedback: bool) -> Tooltip {
        self.enable_feedback = Some(enable_feedback);
        self
    }

    /// Dart `Tooltip(onTriggered:)`.
    pub fn on_triggered(mut self, on_triggered: TooltipTriggeredCallback) -> Tooltip {
        self.on_triggered = Some(on_triggered);
        self
    }

    /// Dart `Tooltip(mouseCursor:)`.
    pub fn mouse_cursor(mut self, mouse_cursor: MouseCursorRef) -> Tooltip {
        self.mouse_cursor = Some(mouse_cursor);
        self
    }

    /// Dart `Tooltip(ignorePointer:)`.
    pub fn ignore_pointer(mut self, ignore_pointer: bool) -> Tooltip {
        self.ignore_pointer = Some(ignore_pointer);
        self
    }

    /// Dart `Tooltip(positionDelegate:)`.
    pub fn position_delegate(mut self, position_delegate: TooltipPositionDelegate) -> Tooltip {
        self.position_delegate = Some(position_delegate);
        self
    }

    /// Dart `Tooltip(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> Tooltip {
        self.child = Some(child.into_widget());
        self
    }

    /// Dismiss all of the tooltips that are currently shown on the screen,
    /// including those with mouse cursors currently hovering over them.
    ///
    /// This method returns true if it successfully dismisses at least one tooltip
    /// and returns false if there is no tooltip currently displayed.
    pub fn dismiss_all_tool_tips(app: &mut App) -> bool {
        RawTooltip::dismiss_all_tool_tips(app)
    }
}

#[allow(deprecated)]
impl fmt::Debug for Tooltip {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tooltip")
            .field("key", &self.key)
            .field("message", &self.message)
            .field(
                "richMessage",
                &self
                    .rich_message
                    .as_ref()
                    .map(|rich_message| rich_message.to_plain_text(true, true)),
            )
            .field("height", &self.height)
            .field("constraints", &self.constraints)
            .field("padding", &self.padding)
            .field("margin", &self.margin)
            .field("vertical offset", &self.vertical_offset)
            .field("position", &self.prefer_below)
            .field("semantics", &self.exclude_from_semantics)
            .field("wait duration", &self.wait_duration)
            .field("show duration", &self.show_duration)
            .field("exit duration", &self.exit_duration)
            .field("triggerMode", &self.trigger_mode)
            .field("enableFeedback", &self.enable_feedback)
            .field("textAlign", &self.text_align)
            .field("positionDelegate", &self.position_delegate.is_some())
            .finish_non_exhaustive()
    }
}

impl StatefulWidget for Tooltip {
    type State = TooltipState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> TooltipState {
        TooltipState {
            state: StateData::new(),
            single_ticker_provider: SingleTickerProviderStateMixinData::new(),
            tooltip_key: GlobalKey::new(),
            visible: true,
            tooltip_theme: TooltipThemeData::new(),
        }
    }
}

/// Contains the state for a [`Tooltip`].
///
/// This class can be used to programmatically show the Tooltip, see the
/// [`ensure_tooltip_visible`](Self::ensure_tooltip_visible) method.
pub struct TooltipState {
    state: StateData<Tooltip>,
    single_ticker_provider: SingleTickerProviderStateMixinData,

    /// Dart's `GlobalKey<RawTooltipState>`.
    tooltip_key: GlobalKey,

    // From InheritedWidgets; Dart's `late` fields, set by `did_change_dependencies` before the
    // first build.
    visible: bool,
    tooltip_theme: TooltipThemeData,
}

impl TooltipState {
    const DEFAULT_VERTICAL_OFFSET: f64 = 24.0;
    const DEFAULT_PREFER_BELOW: bool = true;
    const DEFAULT_MARGIN: EdgeInsetsGeometry = EdgeInsetsGeometry::ZERO;
    const DEFAULT_SHOW_DURATION: Duration = Duration::from_millis(1500);
    const DEFAULT_EXIT_DURATION: Duration = Duration::from_millis(100);
    const DEFAULT_WAIT_DURATION: Duration = Duration::ZERO;
    const DEFAULT_EXCLUDE_FROM_SEMANTICS: bool = false;
    const DEFAULT_TRIGGER_MODE: TooltipTriggerMode = TooltipTriggerMode::LongPress;
    const DEFAULT_ENABLE_FEEDBACK: bool = true;
    const DEFAULT_TEXT_ALIGN: TextAlign = TextAlign::Start;

    /// The plain text message for this tooltip.
    ///
    /// This value will either come from [`Tooltip::message`] or
    /// [`Tooltip::rich_message`].
    fn tooltip_message(self: Handle<Self>, app: &App) -> String {
        let widget = self.widget(app);
        match &widget.message {
            Some(message) => message.clone(),
            None => widget
                .rich_message
                .as_ref()
                .expect("a tooltip has a message or a rich message")
                .to_plain_text(true, true),
        }
    }

    /// Shows the tooltip if it is not already visible.
    ///
    /// After made visible by this method, the tooltip does not automatically
    /// dismiss after `wait_duration` until the user dismisses/re-triggers it, or
    /// [`Tooltip::dismiss_all_tool_tips`] is called.
    ///
    /// Returns `false` when the tooltip shouldn't be shown or when the tooltip
    /// was already visible.
    pub fn ensure_tooltip_visible(self: Handle<Self>, app: &mut App) -> bool {
        let tooltip_key = app.get(self).tooltip_key.clone();
        tooltip_key
            .current_state::<RawTooltipState>(app)
            .is_some_and(|raw_tooltip| raw_tooltip.ensure_tooltip_visible(app))
    }

    // https://material.io/components/tooltips#specs
    fn get_default_tooltip_height(self: Handle<Self>, app: &mut App, context: BuildContext) -> f64 {
        match Theme::of(app, context).platform {
            TargetPlatform::MacOS | TargetPlatform::Linux | TargetPlatform::Windows => 24.0,
            TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::IOS => 32.0,
        }
    }

    fn get_default_padding(self: Handle<Self>, app: &mut App, context: BuildContext) -> EdgeInsets {
        match Theme::of(app, context).platform {
            TargetPlatform::MacOS | TargetPlatform::Linux | TargetPlatform::Windows => {
                EdgeInsets::symmetric(4.0, 8.0)
            }
            TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::IOS => {
                EdgeInsets::symmetric(4.0, 16.0)
            }
        }
    }

    fn get_default_font_size(platform: TargetPlatform) -> f64 {
        match platform {
            TargetPlatform::MacOS | TargetPlatform::Linux | TargetPlatform::Windows => 12.0,
            TargetPlatform::Android | TargetPlatform::Fuchsia | TargetPlatform::IOS => 14.0,
        }
    }

    /// Dart's `_getDefaultPositionDelegate`, a tear-off that reads `widget` and
    /// `_tooltipTheme` when it is called; here a closure holding what it reads, built anew on
    /// every build, the only point either can change.
    fn get_default_position_delegate(self: Handle<Self>, app: &App) -> TooltipPositionDelegate {
        let widget = self.widget(app);
        let tooltip_theme = &app.get(self).tooltip_theme;
        let effective_vertical_offset = widget
            .vertical_offset
            .or(tooltip_theme.vertical_offset)
            .unwrap_or(Self::DEFAULT_VERTICAL_OFFSET);
        let effective_prefer_below = widget
            .prefer_below
            .or(tooltip_theme.prefer_below)
            .unwrap_or(Self::DEFAULT_PREFER_BELOW);
        let position_delegate = widget.position_delegate.clone();
        Rc::new(move |context: TooltipPositionContext| {
            let resolved_context = TooltipPositionContext::new(
                context.target,
                context.target_size,
                context.tooltip_size,
                effective_vertical_offset,
            )
            .overlay_size(context.overlay_size)
            .prefer_below(effective_prefer_below);
            match &position_delegate {
                Some(position_delegate) => position_delegate(resolved_context),
                None => position_dependent_box(
                    context.overlay_size,
                    context.tooltip_size,
                    context.target,
                    effective_prefer_below,
                    effective_vertical_offset,
                    10.0,
                ),
            }
        })
    }
}

impl SingleTickerProviderStateMixin for TooltipState {
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

/// Dart's `TooltipState with SingleTickerProviderStateMixin`.
impl TickerProviderObject for TooltipState {
    fn create_ticker(self: Handle<Self>, app: &mut App, on_tick: TickerCallback) -> Ref<Ticker> {
        SingleTickerProviderStateMixin::create_ticker(self, app, on_tick)
    }
}

impl State for TooltipState {
    type Widget = Tooltip;
    waft_widgets::state_accessors!();

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let visible = TooltipVisibility::of(app, context);
        let tooltip_theme = TooltipTheme::of(app, context);
        let this = app.get_mut(self);
        this.visible = visible;
        this.tooltip_theme = tooltip_theme;
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    #[allow(deprecated)]
    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        // If no message is provided, there is no need to create a tooltip overlay
        // to show an empty container. In this case, just return the wrapped child
        // as is, or SizedBox.shrink if a child is not provided.
        if self.tooltip_message(app).is_empty() {
            return self
                .widget(app)
                .child
                .clone()
                .unwrap_or_else(|| SizedBox::shrink().into_widget());
        }
        let theme = Theme::of(app, context);
        let body_medium = theme
            .text_theme
            .body_medium
            .as_ref()
            .expect("a theme's text theme has a body medium style");
        let font_size = Self::get_default_font_size(theme.platform);
        let (default_text_style, default_decoration) = match theme.brightness() {
            Brightness::Dark => (
                body_medium
                    .copy_with()
                    .color(Colors::BLACK)
                    .font_size(font_size),
                BoxDecoration::new()
                    .color(Colors::WHITE.with_opacity(0.9))
                    .border_radius(BorderRadius::all(Radius::circular(4.0))),
            ),
            Brightness::Light => (
                body_medium
                    .copy_with()
                    .color(Colors::WHITE)
                    .font_size(font_size),
                BoxDecoration::new()
                    .color(
                        Colors::GREY
                            .get(&700)
                            .expect("grey has a 700 shade")
                            .with_opacity(0.9),
                    )
                    .border_radius(BorderRadius::all(Radius::circular(4.0))),
            ),
        };
        let default_tooltip_height = self.get_default_tooltip_height(app, context);
        let default_padding = self.get_default_padding(app, context);
        let widget = self.widget(app);
        let tooltip_theme = &app.get(self).tooltip_theme;
        let default_constraints = BoxConstraints::new().min_height(
            widget
                .height
                .or(tooltip_theme.height)
                .unwrap_or(default_tooltip_height),
        );

        let tooltip_box = TooltipBox {
            constraints: widget
                .constraints
                .or(tooltip_theme.constraints)
                .unwrap_or(default_constraints),
            text_style: widget
                .text_style
                .clone()
                .or_else(|| tooltip_theme.text_style.clone())
                .unwrap_or(default_text_style),
            text_align: widget
                .text_align
                .or(tooltip_theme.text_align)
                .unwrap_or(Self::DEFAULT_TEXT_ALIGN),
            decoration: Some(
                widget
                    .decoration
                    .as_ref()
                    .or(tooltip_theme.decoration.as_ref())
                    .cloned()
                    .unwrap_or_else(|| DecorationRef::new(default_decoration)),
            ),
            padding: Some(
                widget
                    .padding
                    .or(tooltip_theme.padding)
                    .unwrap_or(EdgeInsetsGeometry::Insets(default_padding)),
            ),
            margin: Some(
                widget
                    .margin
                    .or(tooltip_theme.margin)
                    .unwrap_or(Self::DEFAULT_MARGIN),
            ),
            rich_message: widget.rich_message.clone().unwrap_or_else(|| {
                let mut span = TextSpan::new();
                span.text = widget.message.clone();
                Rc::new(span)
            }),
        }
        .into_widget();

        let mut effective_child = MouseRegion::new()
            .cursor(
                widget
                    .mouse_cursor
                    .clone()
                    .unwrap_or_else(<dyn MouseCursor>::defer),
            )
            .child(
                widget
                    .child
                    .clone()
                    .unwrap_or_else(|| SizedBox::shrink().into_widget()),
            )
            .into_widget();

        let exclude_from_semantics = widget
            .exclude_from_semantics
            .or(tooltip_theme.exclude_from_semantics)
            .unwrap_or(Self::DEFAULT_EXCLUDE_FROM_SEMANTICS);

        if app.get(self).visible {
            let widget = self.widget(app);
            let tooltip_theme = &app.get(self).tooltip_theme;
            let mut raw_tooltip = RawTooltip::new(
                if exclude_from_semantics {
                    None
                } else {
                    Some(self.tooltip_message(app))
                },
                move |_app, _context, animation| {
                    FadeTransition::new(animation)
                        .child(tooltip_box.clone())
                        .into_widget()
                },
                effective_child,
            )
            .key(app.get(self).tooltip_key.clone().into())
            .touch_delay(
                widget
                    .show_duration
                    .or(tooltip_theme.show_duration)
                    .unwrap_or(Self::DEFAULT_SHOW_DURATION),
            )
            .trigger_mode(
                widget
                    .trigger_mode
                    .or(tooltip_theme.trigger_mode)
                    .unwrap_or(Self::DEFAULT_TRIGGER_MODE),
            )
            .enable_feedback(
                widget
                    .enable_feedback
                    .or(tooltip_theme.enable_feedback)
                    .unwrap_or(Self::DEFAULT_ENABLE_FEEDBACK),
            )
            .hover_delay(
                widget
                    .wait_duration
                    .or(tooltip_theme.wait_duration)
                    .unwrap_or(Self::DEFAULT_WAIT_DURATION),
            )
            .enable_tap_to_dismiss(widget.enable_tap_to_dismiss);
            if let Some(on_triggered) = widget.on_triggered.clone() {
                raw_tooltip = raw_tooltip.on_triggered(on_triggered);
            }
            effective_child = raw_tooltip
                .dismiss_delay(
                    widget
                        .exit_duration
                        .or(tooltip_theme.exit_duration)
                        .unwrap_or(Self::DEFAULT_EXIT_DURATION),
                )
                .position_delegate(self.get_default_position_delegate(app))
                .ignore_pointer(
                    widget
                        .ignore_pointer
                        .or(tooltip_theme.ignore_pointer)
                        .unwrap_or(widget.message.is_some()),
                )
                .into_widget();
        }

        effective_child
    }
}

#[derive(Debug)]
struct TooltipBox {
    constraints: BoxConstraints,
    text_style: TextStyle,
    text_align: TextAlign,
    decoration: Option<DecorationRef>,
    padding: Option<EdgeInsetsGeometry>,
    margin: Option<EdgeInsetsGeometry>,
    rich_message: InlineSpanRef,
}

impl StatelessWidget for TooltipBox {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        let mut container = Container::new();
        container.decoration = self.decoration.clone();
        container.padding = self.padding;
        container.margin = self.margin;
        ConstrainedBox::new(self.constraints)
            .child(
                DefaultTextStyle::new(
                    self.text_style.clone(),
                    container.child(
                        Center::new().width_factor(1.0).height_factor(1.0).child(
                            Text::rich(Rc::clone(&self.rich_message))
                                .style(self.text_style.clone())
                                .text_align(self.text_align),
                        ),
                    ),
                )
                .text_align(self.text_align),
            )
            .into_widget()
    }
}

#[cfg(test)]
#[allow(deprecated)]
pub(crate) mod tests {
    use std::cell::Cell;

    use waft_embedder::{
        Color, FontWeight, Offset, PointerChange, RRect, Rect, Size, TextDecoration,
        TextDecorationStyle,
    };
    use waft_foundation::Listener;
    use waft_gestures::{GestureRecognizerLeaf, TapGestureRecognizer};
    use waft_painting::{AnyColor, BorderSide, ShapeDecoration, StadiumBorder, TextScaler};
    use waft_rendering::testing::{PaintCall, assert_paints};
    use waft_rendering::{AnyRenderBox, RenderParagraph, RendererBinding};
    use waft_services::SystemMouseCursors;
    use waft_widgets::{
        AnyElement, IgnorePointer, MediaQuery, MediaQueryData, Overlay, OverlayEntry, Padding,
        Positioned, RichText, Stack, WidgetSpan,
    };

    use waft_widgets::testing::{
        TargetPlatformVariant, TestPointer, WidgetTester, expect, find, finds_nothing,
        finds_one_widget, test_widgets, test_widgets_variant,
    };

    use super::*;
    use crate::test_support::{PaintInspection, material_app};
    use crate::theme_data::ThemeData;

    pub(crate) const TOOLTIP_TEXT: &str = "TIP";

    /// `tester.pumpWidget(MaterialApp(theme: theme(app), home: home(app)))`.
    pub(crate) fn pump_app(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
        home: impl FnOnce(&mut App) -> WidgetRef,
    ) {
        let root = {
            let mut app = tester.cell.borrow_mut();
            let theme = theme(&app);
            material_app(theme, home(&mut app))
        };
        tester.pump_widget(root);
    }

    pub(crate) fn m3(app: &App) -> ThemeData {
        ThemeData::new(app)
    }

    pub(crate) fn m2(app: &App) -> ThemeData {
        ThemeData::builder().use_material3(false).build(app)
    }

    /// Dart's `Overlay(initialEntries: [OverlayEntry(builder: (_) => child)])`.
    pub(crate) fn overlay(app: &mut App, child: WidgetRef) -> WidgetRef {
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_app, _context| child.clone()),
            false,
            false,
            false,
        );
        Overlay::new().initial_entries([entry]).into_widget()
    }

    /// Dart's `Stack(children: [Positioned(left:, top:, child:)])`.
    pub(crate) fn positioned_at(left: f64, top: f64, child: Tooltip) -> WidgetRef {
        Stack::new()
            .children([Positioned::new(child).left(left).top(top).into_widget()])
            .into_widget()
    }

    /// Dart's `tooltipKey.currentState?.ensureTooltipVisible()`.
    pub(crate) fn ensure_tooltip_visible(tester: &WidgetTester, key: &GlobalKey) -> bool {
        let mut app = tester.cell.borrow_mut();
        key.current_state::<TooltipState>(&app)
            .is_some_and(|state| state.ensure_tooltip_visible(&mut app))
    }

    /// Dart's `find.text(tooltipText)`, found once.
    pub(crate) fn tooltip_text(tester: &WidgetTester) -> AnyElement {
        let found = find::text(TOOLTIP_TEXT).evaluate(tester);
        assert_eq!(found.len(), 1, "one tooltip text");
        found[0]
    }

    /// Dart's `tester.renderObject(_findTooltipContainer(tooltipText))`.
    pub(crate) fn tooltip_container(tester: &WidgetTester) -> AnyRenderBox {
        let container = tester.ancestor_of_type::<Container>(tooltip_text(tester));
        tester.render_box_of(container)
    }

    /// The render box of the tooltip's text: Dart's `tester.renderObject(find.text(..))`.
    pub(crate) fn text_box(tester: &WidgetTester) -> AnyRenderBox {
        tester.render_box_of(tooltip_text(tester))
    }

    /// Dart's `tester.widget<Text>(find.text(tooltipText)).style!`.
    pub(crate) fn text_style(tester: &WidgetTester) -> TextStyle {
        tester.widget(tooltip_text(tester), |text: &Text| {
            text.style
                .clone()
                .expect("the tooltip gives its text a style")
        })
    }

    pub(crate) fn rect(tester: &WidgetTester, render: AnyRenderBox) -> Rect {
        tester.get_rect(render)
    }

    pub(crate) fn size(tester: &WidgetTester, render: AnyRenderBox) -> Size {
        render.size(&tester.cell.borrow())
    }

    /// The `Container` the tooltip builds around its text.
    fn container_widget<R>(tester: &WidgetTester, read: impl FnOnce(&Container) -> R) -> R {
        let container = tester.ancestor_of_type::<Container>(tooltip_text(tester));
        tester.widget(container, read)
    }

    /// The size of the tooltip's paragraph: Dart's `tester.getSize(find.text(tooltipText))`.
    pub(crate) fn paragraph_size(tester: &WidgetTester) -> Size {
        size(tester, text_box(tester))
    }

    pub(crate) fn found(tester: &WidgetTester, text: &str) -> bool {
        !find::text(text).evaluate(tester).is_empty()
    }

    pub(crate) fn center(tester: &WidgetTester, element: AnyElement) -> Offset {
        rect(tester, tester.render_box_of(element)).center()
    }

    pub(crate) fn tooltip_center(tester: &WidgetTester) -> Offset {
        center(tester, find::by_type::<Tooltip>().evaluate(tester)[0])
    }

    pub(crate) fn ensure_visible_and_fade_in(tester: &mut WidgetTester, key: &GlobalKey) {
        ensure_tooltip_visible(tester, key);
        tester.pump_by(Duration::from_secs(2)); // faded in, show timer started (and at 0.0)
    }

    /// The tooltip of the positioning cases: `height`, `padding`, `verticalOffset` and
    /// `preferBelow` given.
    fn positioned_tooltip(
        key: &GlobalKey,
        height: f64,
        padding: EdgeInsetsGeometry,
        vertical_offset: f64,
        prefer_below: bool,
    ) -> Tooltip {
        Tooltip::new()
            .key(key.clone().into())
            .message(TOOLTIP_TEXT)
            .height(height)
            .padding(padding)
            .vertical_offset(vertical_offset)
            .prefer_below(prefer_below)
            .child(SizedBox::shrink())
    }

    pub(crate) fn green_box() -> Container {
        Container::new()
            .width(100.0)
            .height(100.0)
            .color(Colors::GREEN.get(&500).expect("a 500 shade"))
    }

    #[test]
    fn does_tooltip_end_up_in_the_right_place_center() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let tooltip = positioned_tooltip(&key, 20.0, EdgeInsetsGeometry::all(5.0), 20.0, false);
            pump_app(tester, m3, |app| {
                overlay(app, positioned_at(300.0, 0.0, tooltip))
            });
            ensure_visible_and_fade_in(tester, &key);

            let tip_in_global = rect(tester, tooltip_container(tester)).top_center();
            assert_eq!(tip_in_global.dx(), 300.0);
            assert_eq!(tip_in_global.dy(), 20.0);
        });
    }

    #[test]
    fn does_tooltip_end_up_in_the_right_place_center_with_padding_outside_overlay() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let tooltip = positioned_tooltip(&key, 20.0, EdgeInsetsGeometry::all(5.0), 20.0, false);
            pump_app(tester, m3, |app| {
                let overlay = overlay(app, positioned_at(300.0, 0.0, tooltip));
                Padding::new(EdgeInsetsGeometry::all(20.0))
                    .child(overlay)
                    .into_widget()
            });
            ensure_visible_and_fade_in(tester, &key);

            let tip_in_global = rect(tester, tooltip_container(tester)).top_center();
            assert_eq!(tip_in_global.dx(), 320.0);
            assert_eq!(tip_in_global.dy(), 40.0);
        });
    }

    fn top_left_case(tester: &mut WidgetTester, theme: fn(&App) -> ThemeData) -> Rect {
        let key = GlobalKey::new();
        let tooltip = positioned_tooltip(&key, 20.0, EdgeInsetsGeometry::all(5.0), 20.0, false);
        pump_app(tester, theme, |app| {
            overlay(app, positioned_at(0.0, 0.0, tooltip))
        });
        ensure_visible_and_fade_in(tester, &key);
        rect(tester, tooltip_container(tester))
    }

    #[test]
    fn material2_does_tooltip_end_up_in_the_right_place_top_left() {
        test_widgets(|tester| {
            let tip = top_left_case(tester, m2);
            assert_eq!(tip.height(), 24.0); // 14.0 height + 5.0 padding * 2 (top, bottom)
            assert_eq!(tip.top_left(), Offset::new(10.0, 20.0));
        });
    }

    #[test]
    fn material3_does_tooltip_end_up_in_the_right_place_top_left() {
        test_widgets(|tester| {
            let tip = top_left_case(tester, m3);
            assert_eq!(tip.height(), 30.0); // 20.0 height + 5.0 padding * 2 (top, bottom)
            assert_eq!(tip.top_left(), Offset::new(10.0, 20.0));
        });
    }

    fn center_case(tester: &mut WidgetTester, top: f64, height: f64, prefer_below: bool) -> Rect {
        let key = GlobalKey::new();
        let tooltip =
            positioned_tooltip(&key, height, EdgeInsetsGeometry::ZERO, 100.0, prefer_below);
        pump_app(tester, m3, |app| {
            overlay(app, positioned_at(400.0, top, tooltip))
        });
        ensure_visible_and_fade_in(tester, &key);
        rect(tester, tooltip_container(tester))
    }

    #[test]
    fn does_tooltip_end_up_in_the_right_place_center_prefer_above_fits() {
        test_widgets(|tester| {
            let tip = center_case(tester, 300.0, 100.0, false);
            assert_eq!(tip.height(), 100.0);
            assert_eq!(tip.top, 100.0);
            assert_eq!(tip.bottom, 200.0);
        });
    }

    #[test]
    fn does_tooltip_end_up_in_the_right_place_center_prefer_above_does_not_fit() {
        test_widgets(|tester| {
            let tip = center_case(tester, 299.0, 190.0, false);
            assert_eq!(tip.height(), 190.0);
            assert_eq!(tip.top, 399.0);
            assert_eq!(tip.bottom, 589.0);
        });
    }

    #[test]
    fn does_tooltip_end_up_in_the_right_place_center_prefer_below_fits() {
        test_widgets(|tester| {
            let tip = center_case(tester, 300.0, 190.0, true);
            assert_eq!(tip.height(), 190.0);
            assert_eq!(tip.top, 400.0);
            assert_eq!(tip.bottom, 590.0);
        });
    }

    fn right_edge_case(tester: &mut WidgetTester, theme: fn(&App) -> ThemeData, left: f64) -> Rect {
        let key = GlobalKey::new();
        let tooltip = positioned_tooltip(&key, 10.0, EdgeInsetsGeometry::ZERO, 10.0, true);
        pump_app(tester, theme, |app| {
            overlay(app, positioned_at(left, 300.0, tooltip))
        });
        ensure_visible_and_fade_in(tester, &key);
        rect(tester, tooltip_container(tester))
    }

    #[test]
    fn material2_does_tooltip_end_up_in_the_right_place_way_off_to_the_right() {
        test_widgets(|tester| {
            let tip = right_edge_case(tester, m2, 1600.0);
            assert_eq!(tip.height(), 14.0);
            assert_eq!(tip.top, 310.0);
            assert_eq!(tip.right, 790.0);
            assert_eq!(tip.bottom, 324.0);
        });
    }

    #[test]
    fn material3_does_tooltip_end_up_in_the_right_place_way_off_to_the_right() {
        test_widgets(|tester| {
            let tip = right_edge_case(tester, m3, 1600.0);
            assert_eq!(tip.height(), 20.0);
            assert_eq!(tip.top, 310.0);
            assert_eq!(tip.right, 790.0);
            assert_eq!(tip.bottom, 330.0);
        });
    }

    #[test]
    fn material2_does_tooltip_end_up_in_the_right_place_near_the_edge() {
        test_widgets(|tester| {
            let tip = right_edge_case(tester, m2, 780.0);
            assert_eq!(tip.height(), 14.0);
            assert_eq!(tip.top, 310.0);
            assert_eq!(tip.right, 790.0);
            assert_eq!(tip.bottom, 324.0);
        });
    }

    #[test]
    fn material3_does_tooltip_end_up_in_the_right_place_near_the_edge() {
        test_widgets(|tester| {
            let tip = right_edge_case(tester, m3, 780.0);
            assert_eq!(tip.height(), 20.0);
            assert_eq!(tip.top, 310.0);
            assert_eq!(tip.right, 790.0);
            assert_eq!(tip.bottom, 330.0);
        });
    }

    #[test]
    fn custom_tooltip_margin() {
        test_widgets(|tester| {
            const CUSTOM_MARGIN_VALUE: f64 = 10.0;
            let key = GlobalKey::new();
            let tooltip = Tooltip::new()
                .key(key.clone().into())
                .message(TOOLTIP_TEXT)
                .padding(EdgeInsetsGeometry::ZERO)
                .margin(EdgeInsetsGeometry::all(CUSTOM_MARGIN_VALUE))
                .child(SizedBox::shrink())
                .into_widget();
            pump_app(tester, m3, |app| overlay(app, tooltip));
            ensure_visible_and_fade_in(tester, &key);

            let tip = rect(tester, tooltip_container(tester));
            let content = rect(tester, text_box(tester));
            assert_eq!(content.left, tip.left + CUSTOM_MARGIN_VALUE);
            assert_eq!(content.top, tip.top + CUSTOM_MARGIN_VALUE);
            assert_eq!(content.right, tip.right - CUSTOM_MARGIN_VALUE);
            assert_eq!(content.bottom, tip.bottom - CUSTOM_MARGIN_VALUE);
        });
    }

    fn default_text_style_case(
        tester: &mut WidgetTester,
        theme: impl FnOnce(&App) -> ThemeData,
    ) -> TextStyle {
        let key = GlobalKey::new();
        let tooltip = Tooltip::new()
            .key(key.clone().into())
            .message(TOOLTIP_TEXT)
            .child(green_box())
            .into_widget();
        pump_app(tester, theme, |_app| tooltip);
        ensure_visible_and_fade_in(tester, &key);
        text_style(tester)
    }

    #[test]
    fn material2_default_tooltip_message_text_style_light() {
        test_widgets(|tester| {
            let text_style = default_text_style_case(tester, m2);
            assert_eq!(text_style.color, Some(AnyColor::new(Colors::WHITE)));
            assert_eq!(text_style.font_family.as_deref(), Some("Roboto"));
            assert_eq!(text_style.decoration, Some(TextDecoration::NONE));
            assert_eq!(
                text_style.debug_label.as_deref(),
                Some(
                    "((englishLike bodyMedium 2014).merge(blackMountainView bodyMedium)).copyWith"
                )
            );
        });
    }

    #[test]
    fn material3_default_tooltip_message_text_style_light() {
        test_widgets(|tester| {
            let text_style = default_text_style_case(tester, m3);
            assert_eq!(text_style.color, Some(AnyColor::new(Colors::WHITE)));
            assert_eq!(text_style.font_family.as_deref(), Some("Roboto"));
            assert_eq!(text_style.decoration, Some(TextDecoration::NONE));
            assert_eq!(
                text_style.debug_label.as_deref(),
                Some(
                    "((englishLike bodyMedium 2021).merge((blackMountainView bodyMedium).apply)).copyWith"
                )
            );
        });
    }

    #[test]
    fn material2_default_tooltip_message_text_style_dark() {
        test_widgets(|tester| {
            let text_style = default_text_style_case(tester, |app| {
                ThemeData::builder()
                    .use_material3(false)
                    .brightness(Brightness::Dark)
                    .build(app)
            });
            assert_eq!(text_style.color, Some(AnyColor::new(Colors::BLACK)));
            assert_eq!(text_style.font_family.as_deref(), Some("Roboto"));
            assert_eq!(text_style.decoration, Some(TextDecoration::NONE));
            assert_eq!(
                text_style.debug_label.as_deref(),
                Some(
                    "((englishLike bodyMedium 2014).merge(whiteMountainView bodyMedium)).copyWith"
                )
            );
        });
    }

    #[test]
    fn material3_default_tooltip_message_text_style_dark() {
        test_widgets(|tester| {
            let text_style = default_text_style_case(tester, |app| {
                ThemeData::builder().brightness(Brightness::Dark).build(app)
            });
            assert_eq!(text_style.color, Some(AnyColor::new(Colors::BLACK)));
            assert_eq!(text_style.font_family.as_deref(), Some("Roboto"));
            assert_eq!(text_style.decoration, Some(TextDecoration::NONE));
            assert_eq!(
                text_style.debug_label.as_deref(),
                Some(
                    "((englishLike bodyMedium 2021).merge((whiteMountainView bodyMedium).apply)).copyWith"
                )
            );
        });
    }

    #[test]
    fn custom_tooltip_message_text_style() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let tooltip = Tooltip::new()
                .key(key.clone().into())
                .text_style(
                    TextStyle::new()
                        .color(Colors::ORANGE.primary())
                        .decoration(TextDecoration::UNDERLINE),
                )
                .message(TOOLTIP_TEXT)
                .child(green_box())
                .into_widget();
            pump_app(tester, m3, |_app| tooltip);
            ensure_visible_and_fade_in(tester, &key);

            let text_style = text_style(tester);
            assert_eq!(
                text_style.color,
                Some(AnyColor::new(Colors::ORANGE.primary()))
            );
            assert_eq!(text_style.font_family, None);
            assert_eq!(text_style.decoration, Some(TextDecoration::UNDERLINE));
        });
    }

    #[test]
    fn custom_tooltip_message_text_align() {
        test_widgets(|tester| {
            let mut pump_tooltip_with_text_align = |text_align: Option<TextAlign>| {
                let key = GlobalKey::new();
                let mut tooltip = Tooltip::new()
                    .key(key.clone().into())
                    .message(TOOLTIP_TEXT)
                    .child(green_box());
                if let Some(text_align) = text_align {
                    tooltip = tooltip.text_align(text_align);
                }
                let tooltip = tooltip.into_widget();
                pump_app(tester, m3, |_app| tooltip);
                ensure_visible_and_fade_in(tester, &key);
                tester.widget(tooltip_text(tester), |text: &Text| text.text_align)
            };

            // Default value should be TextAlign.start
            assert_eq!(pump_tooltip_with_text_align(None), Some(TextAlign::Start));
            assert_eq!(
                pump_tooltip_with_text_align(Some(TextAlign::Center)),
                Some(TextAlign::Center)
            );
            assert_eq!(
                pump_tooltip_with_text_align(Some(TextAlign::End)),
                Some(TextAlign::End)
            );
        });
    }

    #[test]
    fn tooltip_overlay_wrapped_with_a_non_fallback_default_text_style_widget() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let tooltip = Tooltip::new()
                .key(key.clone().into())
                .message(TOOLTIP_TEXT)
                .child(green_box())
                .into_widget();
            pump_app(tester, m3, |_app| tooltip);
            ensure_visible_and_fade_in(tester, &key);

            let default_text_style =
                tester.ancestor_of_type::<DefaultTextStyle>(tooltip_text(tester));
            let text_style = tester.widget(default_text_style, |widget: &DefaultTextStyle| {
                widget.style.clone()
            });
            // The default fallback text style results in a text with a
            // double underline of Color(0xffffff00).
            assert_ne!(text_style.decoration, Some(TextDecoration::UNDERLINE));
            assert_ne!(
                text_style.decoration_color,
                Some(AnyColor::new(Color::new(0xffffff00)))
            );
            assert_ne!(
                text_style.decoration_style,
                Some(TextDecorationStyle::Double)
            );
        });
    }

    /// The tooltip of the size, shape and colour cases, made visible in an overlay.
    fn default_look_case(
        tester: &mut WidgetTester,
        theme: fn(&App) -> ThemeData,
        decoration: Option<ShapeDecoration>,
    ) {
        let key = GlobalKey::new();
        let mut tooltip = Tooltip::new()
            .key(key.clone().into())
            .message(TOOLTIP_TEXT)
            .child(SizedBox::shrink());
        if let Some(decoration) = decoration {
            tooltip = tooltip.decoration(decoration);
        }
        let tooltip = tooltip.into_widget();
        pump_app(tester, theme, |app| overlay(app, tooltip));
        ensure_visible_and_fade_in(tester, &key);
    }

    fn default_decoration() -> BoxDecoration {
        BoxDecoration::new()
            .color(Color::new(0xe6616161))
            .border_radius(BorderRadius::all(Radius::circular(4.0)))
    }

    fn assert_default_look(tester: &WidgetTester, padding: EdgeInsets) {
        let tip = tooltip_container(tester);
        assert_paints(
            &tester.painted_ops(tip.as_object()),
            &[PaintCall::rrect_at(
                RRect::from_rect_and_radius(
                    Rect::from_ltwh(
                        0.0,
                        0.0,
                        size(tester, tip).width(),
                        size(tester, tip).height(),
                    ),
                    Radius::circular(4.0),
                ),
                Some(Color::new(0xe6616161)),
            )],
        );
        container_widget(tester, |container| {
            assert!(
                container
                    .decoration
                    .as_deref()
                    .is_some_and(|decoration| decoration.eq_decoration(&default_decoration()))
            );
            assert_eq!(container.padding, Some(EdgeInsetsGeometry::Insets(padding)));
        });
    }

    #[test]
    fn material2_does_tooltip_end_up_with_the_right_default_size_shape_and_color() {
        test_widgets(|tester| {
            default_look_case(tester, m2, None);
            let tip = size(tester, tooltip_container(tester));
            assert_eq!(tip.height(), 32.0);
            assert_eq!(tip.width(), 74.0);
            assert_default_look(tester, EdgeInsets::symmetric(4.0, 16.0));
        });
    }

    #[test]
    fn material3_does_tooltip_end_up_with_the_right_default_size_shape_and_color() {
        test_widgets(|tester| {
            default_look_case(tester, m3, None);
            let tip = size(tester, tooltip_container(tester));
            assert_eq!(tip.height(), 32.0);
            assert_eq!(tip.width(), 74.75);
            assert_default_look(tester, EdgeInsets::symmetric(4.0, 16.0));
        });
    }

    const DESKTOP: [TargetPlatform; 3] = [
        TargetPlatform::MacOS,
        TargetPlatform::Linux,
        TargetPlatform::Windows,
    ];

    fn desktop_case(tester: &mut WidgetTester, theme: fn(&App) -> ThemeData) {
        let key = GlobalKey::new();
        let tooltip = Tooltip::new()
            .key(key.clone().into())
            .message(TOOLTIP_TEXT)
            .child(SizedBox::shrink())
            .into_widget();
        pump_app(tester, theme, |_app| tooltip);
        ensure_visible_and_fade_in(tester, &key);
    }

    #[test]
    fn material2_tooltip_default_size_shape_and_color_test_for_desktop() {
        test_widgets(|_| {
            for platform in DESKTOP {
                test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                    desktop_case(tester, m2);
                    let paragraph = tester.first::<RenderParagraph>();
                    let text_size = paragraph.text_size(&tester.cell.borrow());
                    assert_eq!(text_size.height(), 12.0);
                    let tip = size(tester, tooltip_container(tester));
                    assert_eq!(tip.height(), 24.0);
                    assert_default_look(tester, EdgeInsets::symmetric(4.0, 8.0));
                });
            }
        });
    }

    #[test]
    fn material3_tooltip_default_size_shape_and_color_test_for_desktop() {
        test_widgets(|_| {
            for platform in DESKTOP {
                test_widgets_variant(TargetPlatformVariant::only(platform), |tester| {
                    desktop_case(tester, m3);
                    let paragraph = tester.first::<RenderParagraph>();
                    let text_size = paragraph.text_size(&tester.cell.borrow());
                    assert_eq!(text_size.height(), 17.0);
                    let tip = size(tester, tooltip_container(tester));
                    assert_eq!(tip.height(), 25.0);
                    assert_default_look(tester, EdgeInsets::symmetric(4.0, 8.0));
                });
            }
        });
    }

    pub(crate) fn custom_decoration() -> ShapeDecoration {
        ShapeDecoration::new(StadiumBorder::new(BorderSide::NONE)).color(Color::new(0x80800000))
    }

    #[test]
    fn material2_can_tooltip_decoration_be_customized() {
        test_widgets(|tester| {
            default_look_case(tester, m2, Some(custom_decoration()));
            let tip = tooltip_container(tester);
            assert_eq!(size(tester, tip).height(), 32.0);
            assert_eq!(size(tester, tip).width(), 74.0);
            assert_paints(
                &tester.painted_ops(tip.as_object()),
                &[PaintCall::rrect(Some(Color::new(0x80800000)))],
            );
        });
    }

    #[test]
    fn material3_can_tooltip_decoration_be_customized() {
        test_widgets(|tester| {
            default_look_case(tester, m3, Some(custom_decoration()));
            let tip = tooltip_container(tester);
            assert_eq!(size(tester, tip).height(), 32.0);
            assert_eq!(size(tester, tip).width(), 74.75);
            assert_paints(
                &tester.painted_ops(tip.as_object()),
                &[PaintCall::rrect(Some(Color::new(0x80800000)))],
            );
        });
    }

    /// Dart's text scaling cases, whose `Navigator` and `MaterialPageRoute` only put the
    /// tooltip under the `MediaQuery`: (paragraph, tip) sizes at no scaling and at 4x, pumped
    /// into one tester.
    fn text_scale_case(
        tester: &mut WidgetTester,
        theme: fn(&App) -> ThemeData,
    ) -> [(Size, f64); 2] {
        [TextScaler::NO_SCALING, TextScaler::linear(4.0)].map(|text_scaler| {
            pump_app(tester, theme, |_app| {
                let data = MediaQueryData::new().text_scaler(text_scaler);
                MediaQuery::new(
                    data,
                    Center::new().child(Tooltip::new().message(TOOLTIP_TEXT).child(green_box())),
                )
                .into_widget()
            });
            let at = tooltip_center(tester);
            tester.long_press_at(at, 1);
            assert!(found(tester, TOOLTIP_TEXT));
            (
                paragraph_size(tester),
                size(tester, tooltip_container(tester)).height(),
            )
        })
    }

    #[test]
    fn material2_tooltip_text_scales_with_text_scaler() {
        test_widgets(|tester| {
            let [(text, tip), (scaled_text, scaled_tip)] = text_scale_case(tester, m2);
            assert_eq!(text, Size::new(42.0, 14.0));
            assert_eq!(tip, 32.0);
            assert_eq!(scaled_text, Size::new(168.0, 56.0));
            assert_eq!(scaled_tip, 64.0);
        });
    }

    #[test]
    fn material3_tooltip_text_scales_with_text_scale_factor() {
        test_widgets(|tester| {
            let [(text, tip), (scaled_text, scaled_tip)] = text_scale_case(tester, m3);
            assert_eq!(text.width(), 42.75);
            assert_eq!(text.height(), 20.0);
            assert_eq!(tip, 32.0);
            assert_eq!(scaled_text.width(), 168.75);
            assert_eq!(scaled_text.height(), 80.0);
            assert_eq!(scaled_tip, 88.0);
        });
    }

    #[test]
    fn tooltip_text_displays_with_rich_message() {
        test_widgets(|tester| {
            const TEXT_SPAN1_TEXT: &str = "I am a rich tooltip message. ";
            const TEXT_SPAN2_TEXT: &str = "I am another span of a rich tooltip message";
            let key = GlobalKey::new();
            let rich_message = TextSpan::new()
                .text(TEXT_SPAN1_TEXT)
                .children(vec![TextSpan::new().text(TEXT_SPAN2_TEXT).into_span()])
                .into_span();
            let tooltip = Tooltip::new()
                .key(key.clone().into())
                .rich_message(rich_message)
                .child(green_box())
                .into_widget();
            pump_app(tester, m3, |_app| tooltip);
            ensure_visible_and_fade_in(tester, &key);

            let rich_texts = find::by_type::<RichText>().evaluate(tester);
            assert_eq!(rich_texts.len(), 1);
            let plain_text = tester.widget(rich_texts[0], |rich_text: &RichText| {
                rich_text.text.to_plain_text(true, true)
            });
            assert_eq!(plain_text, format!("{TEXT_SPAN1_TEXT}{TEXT_SPAN2_TEXT}"));
        });
    }

    #[test]
    fn tooltip_does_not_crash_at_zero_area() {
        test_widgets(|tester| {
            tester.view().set_physical_size(Size::ZERO);
            let key = GlobalKey::new();
            let tooltip = Tooltip::new().key(key.clone().into()).message("X");
            pump_app(tester, m3, |_app| {
                Center::new().child(tooltip).into_widget()
            });
            let tooltip_size = |tester: &WidgetTester| {
                size(
                    tester,
                    tester.render_box_of(find::by_type::<Tooltip>().evaluate(tester)[0]),
                )
            };
            assert_eq!(tooltip_size(tester), Size::ZERO);
            ensure_tooltip_visible(tester, &key);
            tester.pump_and_settle();
            assert_eq!(tooltip_size(tester), Size::ZERO);
            expect(tester, &find::text("X"), finds_one_widget());
        });
    }

    #[test]
    #[should_panic(expected = "Either `message` or `richMessage` must be specified")]
    fn tooltip_throws_assertion_error_when_both_message_and_rich_message_are_specified() {
        let _ = Tooltip::new()
            .message("I am a tooltip message.")
            .rich_message(
                TextSpan::new()
                    .text("I am a rich tooltip.")
                    .children(vec![
                        TextSpan::new()
                            .text("I am another span of a rich tooltip.")
                            .into_span(),
                    ])
                    .into_span(),
            )
            .child(green_box());
    }

    #[test]
    fn tooltip_should_not_be_shown_with_empty_message_with_child() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                Tooltip::new()
                    .message("")
                    .child(Text::new(TOOLTIP_TEXT))
                    .into_widget()
            });
            expect(tester, &find::text(TOOLTIP_TEXT), finds_one_widget());
            expect(tester, &find::by_type::<Tooltip>(), finds_one_widget());

            let at = center(tester, tooltip_text(tester));
            tester.long_press_at(at, 1);
            expect(tester, &find::by_type::<Tooltip>(), finds_one_widget());
            expect(tester, &find::by_type::<RawTooltip>(), finds_nothing());
        });
    }

    #[test]
    fn tooltip_should_not_be_shown_with_empty_message_without_child() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| Tooltip::new().message("").into_widget());
            expect(tester, &find::by_type::<Tooltip>(), finds_one_widget());
            expect(tester, &find::by_type::<SizedBox>(), finds_one_widget());
            expect(tester, &find::by_type::<RawTooltip>(), finds_nothing());
        });
    }

    /// Dart's `_testGestureTap`.
    pub(crate) fn test_gesture_tap(tester: &mut WidgetTester, at: Offset) {
        tester.tap_at(at, 1);
        tester.pump_by(Duration::from_millis(10));
    }

    #[test]
    fn tooltip_should_not_ignore_users_tap_on_rich_message() {
        test_widgets(|tester| {
            let is_tapped = Rc::new(Cell::new(false));
            let recognizer = {
                let mut app = tester.cell.borrow_mut();
                let recognizer = TapGestureRecognizer::new(&mut app);
                let tapped = Rc::clone(&is_tapped);
                recognizer.set_on_tap(&mut app, Some(Listener::new(move |_app| tapped.set(true))));
                recognizer
            };
            let rich_message = TextSpan::new()
                .text(TOOLTIP_TEXT)
                .recognizer(recognizer.erase())
                .into_span();
            pump_app(tester, m3, |_app| {
                Tooltip::new()
                    .rich_message(rich_message)
                    .show_duration(Duration::from_secs(5))
                    .trigger_mode(TooltipTriggerMode::Tap)
                    .child(SizedBox::new().width(24.0).height(24.0))
                    .into_widget()
            });

            assert!(!found(tester, TOOLTIP_TEXT));

            let at = tooltip_center(tester);
            test_gesture_tap(tester, at);
            expect(tester, &find::text(TOOLTIP_TEXT), finds_one_widget());

            let at = center(tester, tooltip_text(tester));
            test_gesture_tap(tester, at);
            assert!(is_tapped.get());
            recognizer
                .as_recognizer()
                .dispose(&mut tester.cell.borrow_mut());
        });
    }

    #[test]
    fn tooltip_does_not_rebuild_for_fade_in_fade_out_animation() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                Center::new()
                    .child(
                        SizedBox::square(Some(10.0)).child(
                            Tooltip::new()
                                .message(TOOLTIP_TEXT)
                                .wait_duration(Duration::from_secs(1))
                                .trigger_mode(TooltipTriggerMode::LongPress)
                                .child(SizedBox::expand()),
                        ),
                    )
                    .into_widget()
            });
            let element = find::by_type::<Tooltip>().evaluate(tester)[0];
            let state = element
                .state_handle::<TooltipState>(&tester.cell.borrow())
                .expect("a tooltip state");
            let dirty = |tester: &WidgetTester| element.dirty(&tester.cell.borrow());
            // The Tooltip widget itself is almost stateless thus doesn't need
            // rebuilding.
            assert!(!dirty(tester));

            assert!(state.ensure_tooltip_visible(&mut tester.cell.borrow_mut()));
            assert!(!dirty(tester));
            tester.pump_by(Duration::from_secs(1));
            assert!(!dirty(tester));

            assert!(Tooltip::dismiss_all_tool_tips(
                &mut tester.cell.borrow_mut()
            ));
            assert!(!dirty(tester));
            tester.pump_by(Duration::from_secs(1));
            assert!(!dirty(tester));
        });
    }

    fn active_cursor(tester: &WidgetTester) -> MouseCursorRef {
        let mut app = tester.cell.borrow_mut();
        let tracker = RendererBinding::instance(&mut app).mouse_tracker(&app);
        tracker
            .debug_device_active_cursor(&app, 1)
            .expect("the mouse is over the view")
    }

    #[test]
    fn tooltip_mouse_cursor_behavior() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                Center::new()
                    .child(
                        Tooltip::new()
                            .message(TOOLTIP_TEXT)
                            .mouse_cursor(SystemMouseCursors::GRAB.into())
                            .child(SizedBox::square(Some(50.0))),
                    )
                    .into_widget()
            });

            let mut gesture = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::new(10.0, 10.0));
            tester.pump();
            assert!(*active_cursor(tester) == *MouseCursorRef::from(SystemMouseCursors::BASIC));

            let chip = tooltip_center(tester);
            tester.move_to(&mut gesture, chip, false);
            tester.pump();
            assert!(*active_cursor(tester) == *MouseCursorRef::from(SystemMouseCursors::GRAB));
        });
    }

    /// The hover cases on whether the overlay ignores the pointer: `ignoring` of the last
    /// `IgnorePointer` once the mouse hovers the child. Dart's child is an `ElevatedButton`
    /// in a `Scaffold`, only a hover target here.
    fn overlay_ignores_pointer(
        tester: &mut WidgetTester,
        tooltip: Tooltip,
        tooltip_message: &str,
    ) -> bool {
        let tooltip = tooltip.child(Text::new("Hover me")).into_widget();
        pump_app(tester, m3, |_app| {
            Center::new().child(tooltip).into_widget()
        });

        let button = find::text("Hover me").evaluate(tester);
        assert_eq!(button.len(), 1);

        let mut gesture = TestPointer::mouse(1, 0);
        tester.add_pointer(&mut gesture, Offset::ZERO);
        let at = center(tester, button[0]);
        tester.move_to(&mut gesture, at, false);
        tester.pump_and_settle();

        expect(tester, &find::text(tooltip_message), finds_one_widget());

        let ignore_pointers = find::by_type::<IgnorePointer>().evaluate(tester);
        let ignoring = tester.widget(
            *ignore_pointers.last().expect("an IgnorePointer"),
            |ignore_pointer: &IgnorePointer| ignore_pointer.ignoring,
        );
        tester.send(&gesture, PointerChange::Remove);
        ignoring
    }

    fn rich_tooltip_message() -> InlineSpanRef {
        TextSpan::new()
            .children(vec![
                TextSpan::new()
                    .text("Rich ")
                    .style(TextStyle::new().font_weight(FontWeight::BOLD))
                    .into_span(),
                TextSpan::new().text("Tooltip").into_span(),
            ])
            .into_span()
    }

    #[test]
    fn tooltip_overlay_ignores_pointer_by_default_when_passing_simple_message() {
        test_widgets(|tester| {
            const TOOLTIP_MESSAGE: &str = "Tooltip message";
            assert!(overlay_ignores_pointer(
                tester,
                Tooltip::new().message(TOOLTIP_MESSAGE),
                TOOLTIP_MESSAGE
            ));
        });
    }

    #[test]
    fn tooltip_overlay_with_simple_message_does_not_ignore_pointer_when_passing_ignore_pointer_false()
     {
        test_widgets(|tester| {
            const TOOLTIP_MESSAGE: &str = "Tooltip message";
            assert!(!overlay_ignores_pointer(
                tester,
                Tooltip::new()
                    .ignore_pointer(false)
                    .message(TOOLTIP_MESSAGE),
                TOOLTIP_MESSAGE
            ));
        });
    }

    #[test]
    fn tooltip_overlay_does_not_ignore_pointer_by_default_when_passing_rich_message() {
        test_widgets(|tester| {
            assert!(!overlay_ignores_pointer(
                tester,
                Tooltip::new().rich_message(rich_tooltip_message()),
                "Rich Tooltip"
            ));
        });
    }

    #[test]
    fn tooltip_overlay_with_rich_message_ignores_pointer_when_passing_ignore_pointer_true() {
        test_widgets(|tester| {
            assert!(overlay_ignores_pointer(
                tester,
                Tooltip::new()
                    .ignore_pointer(true)
                    .rich_message(rich_tooltip_message()),
                "Rich Tooltip"
            ));
        });
    }

    #[test]
    fn tooltip_should_pass_its_default_text_style_down_to_widget_spans() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let tooltip = Tooltip::new()
                .key(key.clone().into())
                .rich_message(Rc::new(WidgetSpan::new(Text::new(TOOLTIP_TEXT))))
                .child(green_box())
                .into_widget();
            pump_app(tester, m3, |_app| tooltip);
            ensure_visible_and_fade_in(tester, &key);

            let default_text_style =
                tester.ancestor_of_type::<DefaultTextStyle>(tooltip_text(tester));
            let style = tester.widget(default_text_style, |widget: &DefaultTextStyle| {
                widget.style.clone()
            });
            assert_eq!(style.color, Some(AnyColor::new(Colors::WHITE)));
            assert_eq!(style.font_family.as_deref(), Some("Roboto"));
            assert_eq!(style.decoration, Some(TextDecoration::NONE));
            assert_eq!(
                style.debug_label.as_deref(),
                Some(
                    "((englishLike bodyMedium 2021).merge((blackMountainView bodyMedium).apply)).copyWith"
                )
            );
        });
    }

    #[test]
    fn tooltip_should_apply_provided_text_style_to_rich_messages() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let expected_text_style = TextStyle::new().color(Colors::ORANGE.primary());
            let tooltip = Tooltip::new()
                .key(key.clone().into())
                .rich_message(TextSpan::new().text(TOOLTIP_TEXT).into_span())
                .text_style(expected_text_style.clone())
                .child(green_box())
                .into_widget();
            pump_app(tester, m3, |_app| tooltip);
            ensure_visible_and_fade_in(tester, &key);

            let text_style = text_style(tester);
            let default_text_style =
                tester.ancestor_of_type::<DefaultTextStyle>(tooltip_text(tester));
            let default_text_style = tester
                .widget(default_text_style, |widget: &DefaultTextStyle| {
                    widget.style.clone()
                });
            // Dart's `same`: a style here is a value, so equality stands for identity.
            assert_eq!(text_style, expected_text_style);
            assert_eq!(default_text_style, expected_text_style);
        });
    }

    #[test]
    fn tooltip_respects_and_prefers_the_given_constraints_over_theme_constraints() {
        test_widgets(|tester| {
            let key = GlobalKey::new();
            let theme_constraints = BoxConstraints::tight_for(Some(300.0), Some(150.0));
            let tooltip_constraints = BoxConstraints::tight_for(Some(500.0), Some(250.0));
            let tooltip = Tooltip::new()
                .key(key.clone().into())
                .message(TOOLTIP_TEXT)
                .constraints(tooltip_constraints)
                .padding(EdgeInsetsGeometry::ZERO)
                .child(waft_widgets::ColoredBox::new(Colors::GREEN.primary()))
                .into_widget();
            pump_app(
                tester,
                |app| {
                    ThemeData::builder()
                        .tooltip_theme(TooltipThemeData::new().constraints(theme_constraints))
                        .build(app)
                },
                |_app| tooltip,
            );
            ensure_visible_and_fade_in(tester, &key);

            let text_ancestor = tester.ancestors(tooltip_text(tester))[0];
            assert_eq!(
                text_ancestor.size(&tester.cell.borrow()),
                Some(tooltip_constraints.biggest())
            );
        });
    }

    #[test]
    fn custom_tooltip_positioning_position_delegate_parameter() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                let position_delegate: TooltipPositionDelegate =
                    Rc::new(|context: TooltipPositionContext| {
                        // Align on top right of box with bottom left of tooltip.
                        Offset::new(
                            context.target.dx() + (context.target_size.width() / 2.0),
                            context.target.dy()
                                - (context.target_size.height() / 2.0)
                                - context.tooltip_size.height(),
                        )
                    });
                Center::new()
                    .child(
                        Tooltip::new()
                            .message(TOOLTIP_TEXT)
                            .position_delegate(position_delegate)
                            .child(SizedBox::new().width(50.0).height(50.0)),
                    )
                    .into_widget()
            });

            let at = tooltip_center(tester);
            tester.long_press_at(at, 1);
            tester.pump_by(Duration::from_secs(1));

            expect(tester, &find::text(TOOLTIP_TEXT), finds_one_widget());

            let target_center = tooltip_center(tester);
            let tooltip_position = rect(tester, tooltip_container(tester)).top_left();

            // The tooltip should be positioned at target + (25, -25-32).
            assert!((tooltip_position.dx() - (target_center.dx() + 25.0)).abs() <= 5.0);
            assert!((tooltip_position.dy() - (target_center.dy() - 25.0 - 32.0)).abs() <= 5.0);
        });
    }

    #[test]
    fn tooltip_should_disappear_even_if_mouse_is_hovering_over_the_tooltip_overlay() {
        test_widgets(|tester| {
            pump_app(tester, m3, |_app| {
                Center::new()
                    .child(
                        Tooltip::new()
                            .message(TOOLTIP_TEXT)
                            .child(Text::new("Hover me")),
                    )
                    .into_widget()
            });

            // Hover over the target widget.
            let mut gesture = TestPointer::mouse(1, 0);
            tester.add_pointer(&mut gesture, Offset::ZERO);

            let target = find::text("Hover me").evaluate(tester)[0];
            let at = center(tester, target);
            tester.move_to(&mut gesture, at, false);
            tester.pump();

            // Wait for the tooltip to actually appear.
            tester.pump_by(Duration::from_millis(100));
            expect(tester, &find::text(TOOLTIP_TEXT), finds_one_widget());

            // Move the mouse to the tooltip overlay.
            let at = center(tester, tooltip_text(tester));
            tester.move_to(&mut gesture, at, false);
            tester.pump_and_settle();

            // Verify the tooltip overlay is no longer displayed.
            assert!(!found(tester, TOOLTIP_TEXT));
            tester.send(&gesture, PointerChange::Remove);
        });
    }
}
