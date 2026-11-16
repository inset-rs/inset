//! XAML `CommandBar` with the `DefaultCommandBarStyle` of `CommandBar_themeresources.xaml`, and `CommandBar_Partial.cpp` over `AppBar_Partial.cpp`: a bar of primary commands, an overflow menu of secondary commands behind the `MoreButton`, and the primary commands that do not fit moved into that menu.
//!
//! The template is `LayoutRoot` (see [`layout_root`]) over `ContentRoot` (see [`content_root`]) and `OverflowPopup` (see [`overflow`]). The bar is closed or open, as its owner says, and `DisplayModeStates` animates between them; `Opening` and `Closing` are raised when the owner's value changes, `Opened` and `Closed` when the transition ends. While it is not closed the bar is drawn in the root overlay over the light-dismiss layer, as WinUI draws it through a layout transition element, and its place in the layout keeps its closed size. A later editor must keep the bar subtree under its global key in both places, so its focus and pointer state follow it in and out of the overlay.

mod app_bar_button;
mod app_bar_separator;
mod app_bar_toggle_button;
mod content_root;
mod dynamic_overflow;
mod element;
mod keyboard;
mod layout_root;
mod overflow;

pub use app_bar_button::AppBarButton;
pub use app_bar_separator::AppBarSeparator;
pub use app_bar_toggle_button::{
    AppBarToggleButton, AppBarToggleButtonCheckedHandler, AppBarToggleButtonState,
};
pub use element::{CommandBarDefaultLabelPosition, CommandBarElement, CommandBarLabelPosition};

use content_root::{ContentRoot, ContentRootLayout, MoreButtonVisibility};
use dynamic_overflow::{DynamicOverflow, OverflowEntry, PrimaryCommand};
use element::{ElementHost, ElementPresentation};
use layout_root::{AppBarLayout, AppBarLayoutRoot, AppBarMetrics, DisplayModeMotion};
use overflow::{CommandBarOverlayLayout, OverflowReveal, part};

use crate::controls::flyout::shadow::PopupShadow;
use crate::{
    Brush, CommonState, ControlBorder, ControlStates, FluentSymbol, FocusState, FocusVisual,
    FontIcon, InputDevice, LightDismissOverlayMode, Margin, ScrollBarViewport,
    ScrollViewportController, TextBlockStyle, ThemeResources, ToolTip, ToolTipService,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::time::Duration;
use waft_animation::{Cubic, Curve};
use waft_embedder::{Color, Offset, TextDirection};
use waft_foundation::{App, Handle, Listener, OptionRetained, Ref};
use waft_painting::{PaintingBinding, TextPainter, TextSpan, transform_rect};
use waft_rendering::{BoxConstraints, CrossAxisAlignment, HitTestBehavior, MainAxisSize};
use waft_scheduler::{FrameCallback, Ticker};
use waft_widgets::*;

/// XAML `AppBarClosedDisplayMode`: how much of the bar shows while it is closed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum AppBarClosedDisplayMode {
    /// The commands' icons, `AppBarThemeCompactHeight` tall.
    #[default]
    Compact,
    /// A thin strip with the `MoreButton`, `AppBarThemeMinimalHeight` tall.
    Minimal,
    /// Nothing.
    Hidden,
}

/// XAML `CommandBarOverflowButtonVisibility`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CommandBarOverflowButtonVisibility {
    /// Shown when there is something to open: overflow commands, hidden labels or a bar that is not compact.
    #[default]
    Auto,
    Visible,
    Collapsed,
}

/// XAML `CommandBarDynamicOverflowAction`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandBarDynamicOverflowAction {
    AddingToOverflow,
    RemovingFromOverflow,
}

/// XAML `DynamicOverflowItemsChangingEventArgs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DynamicOverflowItemsChangingEventArgs {
    /// `Action`: whether primary commands are moving into the overflow or out of it.
    pub action: CommandBarDynamicOverflowAction,
}

/// Receives XAML `DynamicOverflowItemsChanging`.
pub type DynamicOverflowItemsChangingHandler =
    Rc<dyn Fn(&mut App, &DynamicOverflowItemsChangingEventArgs)>;

/// Receives the `IsOpen` value the bar asks for.
pub type CommandBarOpenChangedHandler = Rc<dyn Fn(&mut App, bool)>;

/// `DisplayModeStates` as the bar moves between its closed and open states.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum CommandBarDisplayPhase {
    #[default]
    Closed,
    /// The `VisualTransition` into an open state.
    Opening,
    Open,
    /// The `VisualTransition` into the closed state.
    Closing,
}

/// XAML `CommandBar`.
#[derive(Clone)]
pub struct CommandBar {
    /// `IsOpen`.
    pub is_open: bool,
    /// Asked for a new `IsOpen` by the `MoreButton`, a command, light dismiss and the keyboard; the bar opens and closes as `is_open` follows.
    pub changed: CommandBarOpenChangedHandler,
    /// `PrimaryCommands`: the commands in the bar.
    pub primary_commands: Vec<CommandBarElement>,
    /// `SecondaryCommands`: the commands in the overflow menu.
    pub secondary_commands: Vec<CommandBarElement>,
    /// `Content`: shown at the bar's start.
    pub content: Option<WidgetRef>,
    /// `DefaultLabelPosition`.
    pub default_label_position: CommandBarDefaultLabelPosition,
    /// `OverflowButtonVisibility`.
    pub overflow_button_visibility: CommandBarOverflowButtonVisibility,
    /// `IsDynamicOverflowEnabled`: primary commands that do not fit move into the overflow.
    pub is_dynamic_overflow_enabled: bool,
    /// `ClosedDisplayMode`.
    pub closed_display_mode: AppBarClosedDisplayMode,
    /// `IsSticky`: the open bar keeps the focus where it is and ignores light dismiss.
    pub is_sticky: bool,
    /// `LightDismissOverlayMode`: whether the layer under the open bar dims the window.
    pub light_dismiss_overlay_mode: LightDismissOverlayMode,
    /// `Background`, while the bar is closed; the theme's `CommandBarBackground` when `None`.
    pub background: Option<Brush>,
    /// `Opening`.
    pub opening: Option<Listener>,
    /// `Opened`.
    pub opened: Option<Listener>,
    /// `Closing`.
    pub closing: Option<Listener>,
    /// `Closed`.
    pub closed: Option<Listener>,
    /// `DynamicOverflowItemsChanging`.
    pub dynamic_overflow_items_changing: Option<DynamicOverflowItemsChangingHandler>,
    /// Identity retained across parent rebuilds.
    pub key: Option<KeyRef>,
}

impl CommandBar {
    /// Creates a bar with its `IsOpen` and the callback asked for a new one.
    pub fn new(is_open: bool, changed: impl Fn(&mut App, bool) + 'static) -> Self {
        Self {
            is_open,
            changed: Rc::new(changed),
            primary_commands: Vec::new(),
            secondary_commands: Vec::new(),
            content: None,
            default_label_position: CommandBarDefaultLabelPosition::Bottom,
            overflow_button_visibility: CommandBarOverflowButtonVisibility::Auto,
            is_dynamic_overflow_enabled: true,
            closed_display_mode: AppBarClosedDisplayMode::Compact,
            is_sticky: false,
            light_dismiss_overlay_mode: LightDismissOverlayMode::Auto,
            background: None,
            opening: None,
            opened: None,
            closing: None,
            closed: None,
            dynamic_overflow_items_changing: None,
            key: None,
        }
    }

    /// Sets the XAML `PrimaryCommands`.
    pub fn primary_commands(mut self, commands: Vec<CommandBarElement>) -> Self {
        self.primary_commands = commands;
        self
    }

    /// Sets the XAML `SecondaryCommands`.
    pub fn secondary_commands(mut self, commands: Vec<CommandBarElement>) -> Self {
        self.secondary_commands = commands;
        self
    }

    /// Sets the XAML `Content`.
    pub fn content<K>(mut self, content: impl IntoWidget<K>) -> Self {
        self.content = Some(content.into_widget());
        self
    }

    /// Sets the XAML `DefaultLabelPosition`.
    pub fn default_label_position(mut self, position: CommandBarDefaultLabelPosition) -> Self {
        self.default_label_position = position;
        self
    }

    /// Sets the XAML `OverflowButtonVisibility`.
    pub fn overflow_button_visibility(
        mut self,
        visibility: CommandBarOverflowButtonVisibility,
    ) -> Self {
        self.overflow_button_visibility = visibility;
        self
    }

    /// Sets the XAML `IsDynamicOverflowEnabled`.
    pub fn is_dynamic_overflow_enabled(mut self, value: bool) -> Self {
        self.is_dynamic_overflow_enabled = value;
        self
    }

    /// Sets the XAML `ClosedDisplayMode`.
    pub fn closed_display_mode(mut self, mode: AppBarClosedDisplayMode) -> Self {
        self.closed_display_mode = mode;
        self
    }

    /// Sets the XAML `IsSticky`.
    pub fn is_sticky(mut self, value: bool) -> Self {
        self.is_sticky = value;
        self
    }

    /// Sets the XAML `LightDismissOverlayMode`.
    pub fn light_dismiss_overlay_mode(mut self, mode: LightDismissOverlayMode) -> Self {
        self.light_dismiss_overlay_mode = mode;
        self
    }

    /// Sets the XAML `Background`.
    pub fn background(mut self, brush: Brush) -> Self {
        self.background = Some(brush);
        self
    }

    /// Sets the XAML `Opening` handler.
    pub fn opening(mut self, listener: Listener) -> Self {
        self.opening = Some(listener);
        self
    }

    /// Sets the XAML `Opened` handler.
    pub fn opened(mut self, listener: Listener) -> Self {
        self.opened = Some(listener);
        self
    }

    /// Sets the XAML `Closing` handler.
    pub fn closing(mut self, listener: Listener) -> Self {
        self.closing = Some(listener);
        self
    }

    /// Sets the XAML `Closed` handler.
    pub fn closed(mut self, listener: Listener) -> Self {
        self.closed = Some(listener);
        self
    }

    /// Sets the XAML `DynamicOverflowItemsChanging` handler.
    pub fn dynamic_overflow_items_changing(
        mut self,
        handler: impl Fn(&mut App, &DynamicOverflowItemsChangingEventArgs) + 'static,
    ) -> Self {
        self.dynamic_overflow_items_changing = Some(Rc::new(handler));
        self
    }

    /// Sets the identity retained across rebuilds.
    pub fn key(mut self, key: KeyRef) -> Self {
        self.key = Some(key);
        self
    }

    /// What the dynamic overflow model knows of the commands.
    fn overflow_commands(&self) -> (Vec<PrimaryCommand>, usize) {
        let primary = self
            .primary_commands
            .iter()
            .map(|command| PrimaryCommand {
                is_separator: command.is_separator(),
                order: command.dynamic_overflow_order(),
            })
            .collect();
        (primary, self.secondary_commands.len())
    }

    /// What the bar compares between builds to tell whether its commands changed.
    fn signature(&self) -> CommandsSignature {
        let of = |commands: &[CommandBarElement]| {
            commands.iter().map(CommandBarElement::signature).collect()
        };
        (of(&self.primary_commands), of(&self.secondary_commands))
    }
}

type CommandsSignature = (
    Vec<(u8, Option<String>, i32)>,
    Vec<(u8, Option<String>, i32)>,
);

impl fmt::Debug for CommandBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandBar")
            .field("is_open", &self.is_open)
            .field("primary_commands", &self.primary_commands)
            .field("secondary_commands", &self.secondary_commands)
            .field("default_label_position", &self.default_label_position)
            .field("closed_display_mode", &self.closed_display_mode)
            .field("is_sticky", &self.is_sticky)
            .finish_non_exhaustive()
    }
}

/// The transitions' `ControlNormalAnimationDuration`, `ControlFastAnimationDuration` and `ControlFastOutSlowInKeySpline`, as the last build read them.
#[derive(Clone, Copy, Debug, Default)]
struct TransitionTiming {
    normal: Duration,
    fast: Duration,
    key_spline: [f64; 4],
}

/// `OverflowInitialFocusItem`: which overflow item an arrow key that opened the bar focuses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum OverflowInitialFocusItem {
    #[default]
    None,
    FirstItem,
    LastItem,
}

impl StatefulWidget for CommandBar {
    type State = CommandBarState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> CommandBarState {
        let (primary, secondary) = self.overflow_commands();
        CommandBarState {
            state: StateData::new(),
            single_ticker_provider: Default::default(),
            ticker: None,
            phase: CommandBarDisplayPhase::Closed,
            elapsed: Duration::ZERO,
            timing: TransitionTiming::default(),
            overflow: Rc::new(RefCell::new(DynamicOverflow::new(primary, secondary))),
            signature: self.signature(),
            metrics: Rc::new(AppBarMetrics::default()),
            portal: None,
            bar_key: GlobalKey::new(),
            content_focus: None,
            primary_focus: Vec::new(),
            overflow_focus: HashMap::new(),
            more_focus: None,
            overflow_scroll: None,
            opened_with_expand_button: false,
            overflow_initial_focus_item: OverflowInitialFocusItem::None,
            saved_focus: None,
            input_device_used_to_open: InputDevice::Touch,
            layout_change_queued: false,
        }
    }
}

/// The bar's view of its owner's `IsOpen`, its transition, the dynamic overflow, its overlay and its focus.
pub struct CommandBarState {
    state: StateData<CommandBar>,
    single_ticker_provider: SingleTickerProviderStateMixinData,
    /// Clock of the `DisplayModeStates` transition.
    ticker: Option<Ref<Ticker>>,
    phase: CommandBarDisplayPhase,
    /// Time since the transition began.
    elapsed: Duration,
    timing: TransitionTiming,
    overflow: Rc<RefCell<DynamicOverflow>>,
    /// The commands the model was last given.
    signature: CommandsSignature,
    metrics: Rc<AppBarMetrics>,
    portal: Option<Ref<OverlayPortalController>>,
    /// The bar subtree, in its place or in the overlay.
    bar_key: GlobalKey,
    /// Groups the `Content`'s focusable descendants.
    content_focus: Option<FocusNodeRef>,
    /// The bar's copies of the primary commands, by index.
    primary_focus: Vec<FocusNodeRef>,
    /// The overflow's copies of its entries.
    overflow_focus: HashMap<OverflowEntry, FocusNodeRef>,
    more_focus: Option<FocusNodeRef>,
    overflow_scroll: Option<Ref<ScrollViewportController>>,
    /// `m_openedWithExpandButton`.
    opened_with_expand_button: bool,
    /// `m_overflowInitialFocusItem`.
    overflow_initial_focus_item: OverflowInitialFocusItem,
    /// `m_savedFocusedElementWeakRef` and `m_savedFocusState`.
    saved_focus: Option<(AnyFocusNode, FocusState)>,
    /// `m_inputDeviceTypeUsedToOpen`.
    input_device_used_to_open: InputDevice,
    /// The rebuild after a layout that changed what the bar builds is waiting for the end of the frame.
    layout_change_queued: bool,
}

impl SingleTickerProviderStateMixin for CommandBarState {
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

// MARK: Opening and closing

impl CommandBarState {
    fn alive(self: Handle<Self>, app: &App) -> bool {
        app.contains(self) && self.mounted(app)
    }

    /// Runs `work` once the frame is over, if the bar is still mounted.
    fn after_frame(self: Handle<Self>, app: &mut App, work: impl Fn(&mut App) + 'static) {
        let view = View::maybe_id_of(app, self.context(app));
        crate::after_next_frame(
            app,
            view,
            FrameCallback::new(move |app, _| {
                if self.alive(app) {
                    work(app);
                }
            }),
        );
    }

    /// Asks the owner for `is_open`.
    pub(crate) fn request_open(self: Handle<Self>, app: &mut App, is_open: bool) {
        let changed = self.widget(app).changed.clone();
        changed(app, is_open);
    }

    fn raise(app: &mut App, listener: Option<Listener>) {
        if let Some(listener) = listener {
            listener.call(app);
        }
    }

    /// The transition's length: `ControlNormalAnimationDuration` opening, `ControlFastAnimationDuration` closing.
    fn transition_duration(self: Handle<Self>, app: &App) -> Duration {
        let timing = app.get(self).timing;
        match app.get(self).phase {
            CommandBarDisplayPhase::Closing => timing.fast,
            _ => timing.normal,
        }
    }

    /// The transition's progress through `ControlFastOutSlowInKeySpline`.
    fn motion(self: Handle<Self>, app: &mut App) -> DisplayModeMotion {
        let phase = app.get(self).phase;
        let progress = match phase {
            CommandBarDisplayPhase::Opening | CommandBarDisplayPhase::Closing => {
                let [x1, y1, x2, y2] = app.get(self).timing.key_spline;
                let duration = self.transition_duration(app).as_secs_f64();
                let t = (app.get(self).elapsed.as_secs_f64() / duration).clamp(0.0, 1.0);
                Cubic::new(x1, y1, x2, y2).transform(t)
            }
            CommandBarDisplayPhase::Open | CommandBarDisplayPhase::Closed => 1.0,
        };
        DisplayModeMotion { phase, progress }
    }

    fn start_transition(self: Handle<Self>, app: &mut App, phase: CommandBarDisplayPhase) {
        let ticker = app.get(self).ticker.clone().expect("made in `init_state`");
        ticker.stop(app, false);
        app.get_mut(self).phase = phase;
        app.get_mut(self).elapsed = Duration::ZERO;
        ticker.start(app);
    }

    /// The transition's frame; at its end `OnDisplayModesStateChanged` raises `Opened` or `Closed`.
    fn tick(self: Handle<Self>, app: &mut App, elapsed: Duration) {
        let finished = elapsed >= self.transition_duration(app);
        self.set_state(app, |state| state.elapsed = elapsed);
        if !finished {
            return;
        }
        app.get(self).ticker.handle().unwrap().stop(app, false);
        match app.get(self).phase {
            CommandBarDisplayPhase::Opening => {
                self.set_state(app, |state| state.phase = CommandBarDisplayPhase::Open);
                Self::raise(app, self.widget(app).opened.clone());
            }
            CommandBarDisplayPhase::Closing => {
                // `OnClosedImpl`: the bar compacts, the popup and the overlay go, then `Closed`.
                self.set_state(app, |state| state.phase = CommandBarDisplayPhase::Closed);
                Self::raise(app, self.widget(app).closed.clone());
            }
            _ => {}
        }
    }

    /// `OnIsOpenChanged(true)`: the transition starts now; `Opening` and the focus wait for the bar to be in the overlay.
    fn begin_opening(self: Handle<Self>, app: &mut App, animated: bool) {
        // `UpdateInputDeviceTypeUsedToOpen`.
        app.get_mut(self).input_device_used_to_open = InputDevice::current(app);
        if animated {
            self.start_transition(app, CommandBarDisplayPhase::Opening);
        } else {
            app.get_mut(self).phase = CommandBarDisplayPhase::Open;
        }
        self.after_frame(app, move |app| self.on_opening(app));
    }

    /// `AppBar::OnOpeningImpl` and `CommandBar::OnOpeningImpl`, once the overlay holds the bar.
    fn on_opening(self: Handle<Self>, app: &mut App) {
        if let Some(portal) = app.get(self).portal.clone() {
            // The overlay comes in front of what opened since the bar mounted.
            portal.show(app);
        }
        if !self.widget(app).is_sticky {
            self.set_focus_on_app_bar(app);
        }
        Self::raise(app, self.widget(app).opening.clone());
        if app.get(self).opened_with_expand_button {
            // The first overflow item, focused as the expand button was used.
            let state = FocusState::coerce_programmatic(app);
            if let Some(first) = self.overflow_focusables(app).first().copied() {
                Self::focus(app, first, state);
            }
        }
        // `OnSecondaryItemsControlLoaded`.
        let initial = std::mem::take(&mut app.get_mut(self).overflow_initial_focus_item);
        if initial != OverflowInitialFocusItem::None {
            self.set_focused_element_in_overflow(
                app,
                initial == OverflowInitialFocusItem::FirstItem,
            );
        }
    }

    /// `OnIsOpenChanged(false)`.
    fn begin_closing(self: Handle<Self>, app: &mut App) {
        self.start_transition(app, CommandBarDisplayPhase::Closing);
        self.after_frame(app, move |app| self.on_closing(app));
    }

    /// `AppBar::OnClosingImpl` and `CommandBar::OnClosingImpl`.
    fn on_closing(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).opened_with_expand_button = false;
        self.restore_saved_focus(app);
        // `CloseSubMenus`.
        let widget = self.widget(app).clone();
        for command in widget
            .primary_commands
            .iter()
            .chain(&widget.secondary_commands)
        {
            if let CommandBarElement::Button(button) = command
                && let Some(flyout) = &button.flyout
                && flyout.is_open(app)
            {
                flyout.hide(app);
            }
        }
        Self::raise(app, widget.closing);
    }

    /// `OnExpandButtonClick`.
    fn on_expand_button_click(self: Handle<Self>, app: &mut App) {
        let is_open = self.widget(app).is_open;
        if !is_open {
            app.get_mut(self).opened_with_expand_button = true;
        }
        self.request_open(app, !is_open);
    }

    /// `CommandBar::OnCommandExecutionStatic`: a command closes its bar.
    fn on_command_execution(self: Handle<Self>, app: &mut App) {
        if self.alive(app) && self.widget(app).is_open {
            self.request_open(app, false);
        }
    }

    /// `TryDismissInlineAppBar`: closes the bar unless it is sticky. Answers whether it did.
    fn try_dismiss_inline_app_bar(self: Handle<Self>, app: &mut App) -> bool {
        let widget = self.widget(app);
        if widget.is_sticky || !widget.is_open {
            return false;
        }
        self.request_open(app, false);
        true
    }

    /// `OnPointerPressed` of a closed minimal bar: a press opens it.
    fn on_bar_pointer_down(self: Handle<Self>, app: &mut App, position: Offset) {
        let widget = self.widget(app);
        if widget.is_open || widget.closed_display_mode != AppBarClosedDisplayMode::Minimal {
            return;
        }
        if app
            .get(self)
            .metrics
            .more_button
            .get()
            .is_some_and(|rect| rect.contains(position))
        {
            return;
        }
        self.request_open(app, true);
    }
}

// MARK: Dynamic overflow

impl CommandBarState {
    /// `OnPrimaryCommandsChanged` and `OnSecondaryCommandsChanged`: commands that changed start over in the bar.
    fn sync_commands(self: Handle<Self>, app: &mut App) {
        let signature = self.widget(app).signature();
        if signature == app.get(self).signature {
            return;
        }
        let (primary, secondary) = self.widget(app).overflow_commands();
        app.get(self)
            .overflow
            .borrow_mut()
            .set_commands(primary, secondary);
        app.get_mut(self).signature = signature;
    }

    /// The layout moved commands, raised `DynamicOverflowItemsChanging` or changed the heights the `MoreButton` is shown by: the events go out and the bar rebuilds after the frame.
    fn layout_changed(self: Handle<Self>, app: &mut App) {
        if !self.alive(app) || app.get(self).layout_change_queued {
            return;
        }
        app.get_mut(self).layout_change_queued = true;
        self.after_frame(app, move |app| {
            app.get_mut(self).layout_change_queued = false;
            let raised = std::mem::take(&mut app.get(self).overflow.borrow_mut().raised);
            if let Some(handler) = self.widget(app).dynamic_overflow_items_changing.clone() {
                for action in raised {
                    handler(app, &DynamicOverflowItemsChangingEventArgs { action });
                }
            }
            self.set_state(app, |_| {});
        });
    }

    /// `SetOverflowStyleParams` over the overflow's entries: whether any has an icon, is a toggle button, or has accelerator text.
    fn overflow_style_params(self: Handle<Self>, app: &App) -> (bool, bool, bool) {
        let widget = self.widget(app);
        let overflow = app.get(self).overflow.borrow();
        let mut params = (false, false, false);
        for entry in &overflow.dynamic_secondary {
            let Some(element) = Self::entry_element(widget, *entry) else {
                continue;
            };
            params.0 |= element.has_icon();
            params.1 |= matches!(element, CommandBarElement::ToggleButton(_));
            params.2 |= !element.keyboard_accelerator_text().is_empty();
        }
        params
    }

    fn entry_element(widget: &CommandBar, entry: OverflowEntry) -> Option<&CommandBarElement> {
        match entry {
            OverflowEntry::Primary(index) => widget.primary_commands.get(index),
            OverflowEntry::Secondary(index) => widget.secondary_commands.get(index),
            OverflowEntry::Separator => None,
        }
    }

    /// `UpdateTemplateSettings`' `KeyboardAcceleratorTextMinWidth`: the widest accelerator text in the overflow, in `CaptionTextBlockStyle`.
    fn keyboard_accelerator_text_min_width(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
    ) -> f64 {
        let r = ThemeResources::of(app, context);
        let texts: Vec<String> = {
            let widget = self.widget(app);
            let overflow = app.get(self).overflow.borrow();
            overflow
                .dynamic_secondary
                .iter()
                .filter_map(|entry| Self::entry_element(widget, *entry))
                .map(|element| element.keyboard_accelerator_text().to_string())
                .filter(|text| !text.is_empty())
                .collect()
        };
        let fonts = PaintingBinding::instance(app).fonts(app);
        let direction = Directionality::of(app, context);
        let scaler = MediaQuery::text_scaler_of(app, context);
        let mut widest: f64 = 0.0;
        for text in texts {
            let mut painter = TextPainter::new();
            painter.set_text(Some(Rc::new(
                TextSpan::new().text(text).style(
                    TextBlockStyle::Caption
                        .text_style(&r, r.app_bar_button_keyboard_accelerator_text_foreground),
                ),
            )));
            painter.set_text_direction(Some(direction));
            painter.set_text_scaler(scaler.clone());
            painter.layout(app.get_mut(fonts), 0.0, f64::INFINITY);
            widest = widest.max(painter.width());
            painter.dispose();
        }
        widest
    }
}

// MARK: Building

/// What one build of the bar shares among its parts.
#[derive(Clone, Copy)]
struct BarBuild {
    motion: DisplayModeMotion,
    closed: bool,
    /// `MinimalClosed`: the content and commands are transparent and take no input.
    minimal_closed: bool,
    /// `HiddenClosed` and `MinimalClosed`: the content and commands are disabled.
    inert: bool,
    /// The content's and commands' opacity in the minimal states.
    minimal_opacity: f64,
    /// The compact open states set `ContentRoot`'s open background, corner radius and `OpenBorder`.
    open_chrome: bool,
}

impl CommandBarState {
    fn bar_build(self: Handle<Self>, app: &mut App) -> BarBuild {
        let motion = self.motion(app);
        let mode = self.widget(app).closed_display_mode;
        let closed = motion.phase == CommandBarDisplayPhase::Closed;
        let minimal = mode == AppBarClosedDisplayMode::Minimal;
        let minimal_opacity = if !minimal {
            1.0
        } else {
            match motion.phase {
                CommandBarDisplayPhase::Closed => 0.0,
                CommandBarDisplayPhase::Opening => motion.progress,
                CommandBarDisplayPhase::Open => 1.0,
                CommandBarDisplayPhase::Closing => 1.0 - motion.progress,
            }
        };
        BarBuild {
            motion,
            closed,
            minimal_closed: minimal && closed,
            inert: closed && mode != AppBarClosedDisplayMode::Compact,
            minimal_opacity,
            open_chrome: mode == AppBarClosedDisplayMode::Compact
                && matches!(
                    motion.phase,
                    CommandBarDisplayPhase::Opening | CommandBarDisplayPhase::Open
                ),
        }
    }

    fn primary_node(self: Handle<Self>, app: &mut App, index: usize) -> FocusNodeRef {
        while app.get(self).primary_focus.len() <= index {
            let node: FocusNodeRef = FocusNode::new(app).into_erased();
            app.get_mut(self).primary_focus.push(node);
        }
        app.get(self).primary_focus[index].clone()
    }

    fn overflow_node(self: Handle<Self>, app: &mut App, entry: OverflowEntry) -> FocusNodeRef {
        if let Some(node) = app.get(self).overflow_focus.get(&entry) {
            return node.clone();
        }
        let node: FocusNodeRef = FocusNode::new(app).into_erased();
        app.get_mut(self).overflow_focus.insert(entry, node.clone());
        node
    }

    fn host(self: Handle<Self>, focus_node: FocusNodeRef) -> ElementHost {
        ElementHost {
            executed: Listener::new(move |app| self.on_command_execution(app)),
            focus_node,
        }
    }

    /// `UpdateTemplateSettings`' `EffectiveOverflowButtonVisibility`, as far as a build knows it.
    fn more_button_visibility(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
    ) -> MoreButtonVisibility {
        let widget = self.widget(app).clone();
        match widget.overflow_button_visibility {
            CommandBarOverflowButtonVisibility::Visible => MoreButtonVisibility {
                shown: true,
                when_overflowing: false,
            },
            CommandBarOverflowButtonVisibility::Collapsed => MoreButtonVisibility {
                shown: false,
                when_overflowing: false,
            },
            CommandBarOverflowButtonVisibility::Auto => {
                let metrics = app.get(self).metrics.clone();
                // A delta that rounds to the same physical pixel does not count.
                let scale = MediaQuery::maybe_device_pixel_ratio_of(app, context).unwrap_or(1.0);
                let delta = metrics.compact_height.get() - metrics.content_height.get();
                let has_bottom_label = {
                    let overflow = app.get(self).overflow.borrow();
                    overflow.dynamic_primary.iter().any(|index| {
                        widget.primary_commands[*index].has_label_at(
                            widget.default_label_position,
                            CommandBarDefaultLabelPosition::Bottom,
                        )
                    })
                };
                let shown = !widget.secondary_commands.is_empty()
                    || widget.closed_display_mode != AppBarClosedDisplayMode::Compact
                    || (scale > 0.0 && delta.abs() >= 0.5 / scale)
                    || has_bottom_label;
                MoreButtonVisibility {
                    shown,
                    when_overflowing: true,
                }
            }
        }
    }

    /// `HasRightLabelDynamicPrimaryCommand || HasNonLabeledDynamicPrimaryCommand`: every command is labelled, so only a right label counts.
    fn grows_with_content(self: Handle<Self>, app: &App) -> bool {
        let widget = self.widget(app);
        let overflow = app.get(self).overflow.borrow();
        overflow.dynamic_primary.iter().any(|index| {
            widget.primary_commands[*index].has_label_at(
                widget.default_label_position,
                CommandBarDefaultLabelPosition::Right,
            )
        })
    }

    /// `LayoutRoot` > `ContentRoot`, in its place or in the overlay.
    fn bar(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let widget = self.widget(app).clone();
        let build = self.bar_build(app);
        let more_button = self.more_button_visibility(app, context);
        let overflowing = !app.get(self).overflow.borrow().dynamic_secondary.is_empty();
        let more_button_shown = more_button.shown || (more_button.when_overflowing && overflowing);
        let foreground = r.command_bar_foreground;

        // ContentControl
        let content_focus = app
            .get(self)
            .content_focus
            .clone()
            .expect("made in `init_state`");
        let content = widget
            .content
            .clone()
            .unwrap_or_else(|| SizedBox::shrink().into_widget());
        let content = Margin::new(
            // The style's `Padding`.
            [4.0, 0.0, 0.0, 0.0],
            Self::inert(
                build,
                Focus::new(DefaultTextStyle::new(
                    crate::control_text_style(
                        &r,
                        r.control_content_theme_font_size,
                        waft_embedder::FontWeight::NORMAL,
                        foreground,
                    ),
                    content,
                ))
                .focus_node(content_focus)
                .can_request_focus(false)
                .skip_traversal(true)
                .into_widget(),
                false,
            ),
        );
        let mut children = vec![content.into_widget()];

        // PrimaryItemsControl
        let presentation = ElementPresentation {
            // `SetCompactMode`: compact until `Opening`, again from `Closed`.
            is_compact: build.closed,
            default_label_position: widget.default_label_position,
            in_command_bar: true,
            ..Default::default()
        };
        for (index, command) in widget.primary_commands.iter().enumerate() {
            let node = self.primary_node(app, index);
            let moved = app.get(self).overflow.borrow().is_moved(index)
                || !app
                    .get(self)
                    .overflow
                    .borrow()
                    .dynamic_primary
                    .contains(&index);
            let hosted = command.hosted(presentation, self.host(node));
            children.push(Self::inert(build, Self::element_widget(hosted), moved));
        }

        // MoreButton
        children.push(self.more_button(app, context, &r, build, more_button_shown));

        let panel = ContentRoot {
            layout: ContentRootLayout {
                dynamic_overflow_enabled: widget.is_dynamic_overflow_enabled,
                more_button,
                more_button_column_min_width: r.app_bar_more_button_column_min_width,
                min_height: r.app_bar_theme_compact_height,
                more_button_stretches: !build.closed,
                rtl: Directionality::maybe_of(app, context) == Some(TextDirection::Rtl),
            },
            overflow: app.get(self).overflow.clone(),
            metrics: app.get(self).metrics.clone(),
            overflow_changed: Rc::new(move |app| self.layout_changed(app)),
            children,
        };
        let background = if build.open_chrome {
            Brush::Acrylic(r.command_bar_background_open)
        } else {
            widget
                .background
                .unwrap_or(Brush::Solid(r.command_bar_background))
        };
        let corner_radius = if build.open_chrome {
            r.control_corner_radius
        } else {
            [0.0; 4]
        };
        let mut layers = vec![panel.into_widget()];
        if build.open_chrome {
            // OpenBorder
            layers.push(
                Positioned::fill(
                    IgnorePointer::new().child(
                        ControlBorder::new_optional(
                            None,
                            Some(Brush::Solid(r.command_bar_border_brush_open)),
                        )
                        .border_thickness_ltrb(r.command_bar_border_thickness_open)
                        .corner_radius_corners(corner_radius),
                    ),
                )
                .into_widget(),
            );
        }
        let content_root =
            ConstrainedBox::new(BoxConstraints::new().min_height(r.app_bar_theme_compact_height))
                .child(
                    ControlBorder::new_optional(Some(background), None)
                        .border_thickness(0.0)
                        .corner_radius_corners(corner_radius)
                        .child(Stack::new().children(layers)),
                );
        let content_root = waft_widgets::Listener::new()
            .behavior(HitTestBehavior::Translucent)
            .on_pointer_down(Rc::new(move |app, event| {
                self.on_bar_pointer_down(app, event.local_position())
            }))
            .child(content_root);
        let layout_root = AppBarLayoutRoot {
            layout: AppBarLayout {
                closed_display_mode: widget.closed_display_mode,
                is_open: widget.is_open,
                grows_with_content: self.grows_with_content(app),
                min_compact_height: r.app_bar_theme_compact_height,
                minimal_height: r.app_bar_theme_minimal_height,
                motion: build.motion,
            },
            metrics: app.get(self).metrics.clone(),
            child: content_root.into_widget(),
        };
        KeyedSubtree::new(layout_root.into_widget())
            .key(app.get(self).bar_key.clone().into())
            .into_widget()
    }

    /// An element as a widget.
    fn element_widget(element: CommandBarElement) -> WidgetRef {
        match element {
            CommandBarElement::Button(button) => button.into_widget(),
            CommandBarElement::ToggleButton(button) => button.into_widget(),
            CommandBarElement::Separator(separator) => separator.into_widget(),
        }
    }

    /// The minimal and hidden closed states' `IsEnabled`, `IsHitTestVisible` and `Opacity` on the content and the commands; a command moved to the overflow is out of reach too.
    fn inert(build: BarBuild, child: WidgetRef, moved: bool) -> WidgetRef {
        let child = ExcludeFocus::new(child)
            .excluding(build.inert || moved)
            .into_widget();
        let child = IgnorePointer::new()
            .ignoring(build.minimal_closed || moved)
            .child(child)
            .into_widget();
        if build.minimal_opacity < 1.0 {
            Opacity::new(build.minimal_opacity)
                .child(child)
                .into_widget()
        } else {
            child
        }
    }

    /// `MoreButton` with the `EllipsisButton` style.
    fn more_button(
        self: Handle<Self>,
        app: &mut App,
        _context: BuildContext,
        r: &ThemeResources,
        build: BarBuild,
        shown: bool,
    ) -> WidgetRef {
        let widget = self.widget(app).clone();
        let minimal = widget.closed_display_mode == AppBarClosedDisplayMode::Minimal;
        let node = app
            .get(self)
            .more_focus
            .clone()
            .expect("made in `init_state`");
        let (glyph_size, circle) = (20.0, r.app_bar_expand_button_circle_diameter);
        // EllipsisIcon: the glyph centred on a line `AppBarExpandButtonCircleDiameter` tall.
        let icon = SizedBox::new().height(circle).child(
            OverflowBox::new()
                .max_height(f64::INFINITY)
                .child(FontIcon::symbol(FluentSymbol::More).font_size(glyph_size)),
        );
        let foreground = r.command_bar_foreground;
        let button = crate::Button::new(
            icon,
            Listener::new(move |app| self.on_expand_button_click(app)),
        )
        .focus_node(node)
        // `HiddenClosed` takes the button out of tab navigation.
        .is_tab_stop(
            !(build.closed && widget.closed_display_mode == AppBarClosedDisplayMode::Hidden),
        )
        .template(move |app, context, states, content| {
            ellipsis_button_template(app, context, states, content, foreground)
        });
        let tool_tip = if widget.is_open {
            "See less"
        } else {
            "See more"
        };
        let min_height = if minimal {
            r.app_bar_theme_minimal_height
        } else {
            r.app_bar_theme_compact_height
        };
        let button = SizedBox::new()
            .width(r.app_bar_expand_button_theme_width)
            .child(
                ConstrainedBox::new(BoxConstraints::new().min_height(min_height))
                    .child(ToolTipService::new(button, ToolTip::text(tool_tip))),
            );
        ExcludeFocus::new(button).excluding(!shown).into_widget()
    }

    /// The overflow menu's items: the moved primary commands, the separator, the secondary commands.
    fn overflow_items(self: Handle<Self>, app: &mut App, context: BuildContext) -> Vec<WidgetRef> {
        let widget = self.widget(app).clone();
        let (with_icons, with_toggle_buttons, with_keyboard_accelerator_text) =
            self.overflow_style_params(app);
        let input_mode = if app.get(self).input_device_used_to_open == InputDevice::Touch {
            InputDevice::Touch
        } else {
            InputDevice::None
        };
        let presentation = ElementPresentation {
            use_overflow_style: true,
            default_label_position: widget.default_label_position,
            with_icons,
            with_toggle_buttons,
            with_keyboard_accelerator_text,
            input_mode,
            keyboard_accelerator_text_min_width: self
                .keyboard_accelerator_text_min_width(app, context),
            in_command_bar: true,
            ..Default::default()
        };
        let entries = app.get(self).overflow.borrow().dynamic_secondary.clone();
        entries
            .into_iter()
            .map(|entry| match Self::entry_element(&widget, entry).cloned() {
                Some(element) => {
                    let node = self.overflow_node(app, entry);
                    Self::element_widget(element.hosted(presentation, self.host(node)))
                }
                None => AppBarSeparator {
                    presentation,
                    ..AppBarSeparator::default()
                }
                .into_widget(),
            })
            .collect()
    }

    /// `OverflowContentRoot` > `SecondaryItemsControlShadowWrapper` > `CommandBarOverflowPresenter`.
    fn overflow_presenter(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        full_width: bool,
    ) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let items = self.overflow_items(app, context);
        let opens_up = app.get(self).metrics.opens_up.get();
        // `DisplayModeStates` of the presenter.
        let (thickness, padding) = match (full_width, opens_up) {
            (false, _) => (
                r.command_bar_overflow_presenter_border_thickness,
                r.command_bar_overflow_presenter_border_padding,
            ),
            (true, false) => (
                r.command_bar_overflow_presenter_border_down_thickness,
                r.command_bar_overflow_presenter_border_down_padding,
            ),
            (true, true) => (
                r.command_bar_overflow_presenter_border_up_thickness,
                r.command_bar_overflow_presenter_border_up_padding,
            ),
        };
        let items = Column::new()
            .main_axis_size(MainAxisSize::Min)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .children(items);
        let scroll = app
            .get(self)
            .overflow_scroll
            .clone()
            .expect("made in `init_state`");
        let presenter = ControlBorder::new(
            Brush::Acrylic(r.command_bar_overflow_presenter_background),
            Brush::Solid(r.command_bar_overflow_presenter_border_brush),
        )
        .border_thickness_ltrb(thickness)
        .corner_radius_corners(r.overlay_corner_radius)
        .padding(padding)
        .child(ScrollBarViewport::new(
            scroll,
            Margin::new(r.command_bar_overflow_presenter_margin, items),
        ));
        let shadowed = CustomPaint::new()
            .painter(PopupShadow::for_flyout(&r, r.theme, 0))
            .child(IntrinsicWidth::new().child(presenter));
        let presenter = Focus::new(shadowed)
            .can_request_focus(false)
            .skip_traversal(true)
            .on_key_event(Rc::new(move |app, _, event| self.overflow_key(app, event)));
        let motion = self.motion(app);
        OverflowReveal {
            motion,
            closed_display_mode: self.widget(app).closed_display_mode,
            minimal_height: r.app_bar_theme_minimal_height,
            metrics: app.get(self).metrics.clone(),
            child: presenter.into_widget(),
        }
        .into_widget()
    }

    /// `AppBar::SetupOverlayState`'s light-dismiss layer: hit unless the bar is sticky, dimming the window when `LightDismissOverlayMode` is on, fading in and out with the bar.
    fn light_dismiss_layer(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let widget = self.widget(app).clone();
        let motion = self.motion(app);
        let color = if widget.light_dismiss_overlay_mode == LightDismissOverlayMode::On {
            let opacity = match motion.phase {
                CommandBarDisplayPhase::Opening => motion.progress,
                CommandBarDisplayPhase::Closing => 1.0 - motion.progress,
                _ => 1.0,
            };
            let base = r.app_bar_light_dismiss_overlay_background;
            Color {
                a: base.a * opacity,
                ..base
            }
        } else {
            Color::new(0)
        };
        IgnorePointer::new()
            .ignoring(widget.is_sticky)
            .child(
                waft_widgets::Listener::new()
                    .behavior(HitTestBehavior::Opaque)
                    // `OnOverlayElementPointerPressed`.
                    .on_pointer_down(Rc::new(move |app, _| {
                        self.try_dismiss_inline_app_bar(app);
                    }))
                    .child(ColoredBox::new(color)),
            )
            .into_widget()
    }

    /// The overlay while the bar is not closed: the light-dismiss layer, the bar where its place in the layout is, and the overflow menu.
    fn overlay(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        info: OverlayChildLayoutInfo,
    ) -> WidgetRef {
        if app.get(self).phase == CommandBarDisplayPhase::Closed {
            return SizedBox::shrink().into_widget();
        }
        let r = ThemeResources::of(app, context);
        let widget = self.widget(app).clone();
        let target = transform_rect(
            &info.child_paint_transform(),
            Offset::ZERO & info.child_size(),
        );
        let overlay_size = info.overlay_size();
        let (has_primary_commands, has_overflow) = {
            let overflow = app.get(self).overflow.borrow();
            (
                !widget.primary_commands.is_empty(),
                !overflow.dynamic_secondary.is_empty(),
            )
        };
        let mut children = vec![
            LayoutId::new(part("barrier"), self.light_dismiss_layer(app, context)).into_widget(),
            LayoutId::new(part("bar"), self.bar(app, context)).into_widget(),
        ];
        if has_overflow {
            let full_width = overlay_size.width() <= r.command_bar_overflow_max_width;
            children.push(
                LayoutId::new(
                    part("overflow"),
                    self.overflow_presenter(app, context, full_width),
                )
                .into_widget(),
            );
        }
        let overflow_min_width = if app.get(self).input_device_used_to_open == InputDevice::Touch {
            r.command_bar_overflow_touch_min_width
        } else {
            r.command_bar_overflow_min_width
        };
        CustomMultiChildLayout::new(Rc::new(CommandBarOverlayLayout {
            target,
            metrics: app.get(self).metrics.clone(),
            has_bar: true,
            has_overflow,
            has_primary_commands,
            overflow_min_width,
            overflow_max_width: r.command_bar_overflow_max_width,
            rtl: Directionality::maybe_of(app, context) == Some(TextDirection::Rtl),
            closed_display_mode: widget.closed_display_mode,
            motion: self.motion(app),
            compact_height_resource: r.app_bar_theme_compact_height,
            minimal_height: r.app_bar_theme_minimal_height,
        }))
        .children(children)
        .into_widget()
    }
}

/// The `EllipsisButton` style's template: `ContentPresenter` with the `AppBarButton` brushes of its `CommonStates`, inside `AppBarEllipsisButtonInnerBorderMargin`, and the system focus visual around it.
fn ellipsis_button_template(
    app: &mut App,
    context: BuildContext,
    states: ControlStates,
    content: WidgetRef,
    foreground: Color,
) -> WidgetRef {
    let r = ThemeResources::of(app, context);
    let (background, border, foreground) = match states.common {
        CommonState::Normal => (
            r.app_bar_button_background,
            r.app_bar_button_border_brush,
            foreground,
        ),
        CommonState::PointerOver => (
            r.app_bar_button_background_pointer_over,
            r.app_bar_button_border_brush_pointer_over,
            r.app_bar_button_foreground_pointer_over,
        ),
        CommonState::Pressed => (
            r.app_bar_button_background_pressed,
            r.app_bar_button_border_brush_pressed,
            r.app_bar_button_foreground_pressed,
        ),
        CommonState::Disabled => (
            r.app_bar_button_background_disabled,
            r.app_bar_button_border_brush_disabled,
            r.app_bar_button_foreground_disabled,
        ),
    };
    let corner_radius = r.control_corner_radius;
    let presenter = ControlBorder::new(Brush::Solid(background), Brush::Solid(border))
        .border_thickness(0.0)
        .corner_radius_corners(corner_radius)
        .child(Center::new().child(DefaultTextStyle::new(
            crate::control_text_style(
                &r,
                r.control_content_theme_font_size,
                waft_embedder::FontWeight::W600,
                foreground,
            ),
            content,
        )));
    let presenter = FocusVisual::new(presenter, r.theme)
        .visible(states.focused)
        .margin([-3.0; 4])
        .corner_radius(corner_radius[0]);
    Margin::new(r.app_bar_ellipsis_button_inner_border_margin, presenter).into_widget()
}

impl State for CommandBarState {
    type Widget = CommandBar;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        InputDevice::initialize(app);
        let ticker = SingleTickerProviderStateMixin::create_ticker(
            self,
            app,
            FrameCallback::new(move |app, elapsed| self.tick(app, elapsed)),
        );
        let portal = OverlayPortalController::new(app, Some("CommandBar".into()));
        // The overlay stays shown; it is empty while the bar is closed.
        portal.show(app);
        let content_focus = FocusNode::new(app).into_erased();
        let more_focus = FocusNode::new(app).into_erased();
        let overflow_scroll = ScrollViewportController::new(app);
        *app.get(self).metrics.heights_changed.borrow_mut() =
            Some(Rc::new(move |app| self.layout_changed(app)));
        let state = app.get_mut(self);
        state.ticker = Some(ticker);
        state.portal = Some(portal);
        state.content_focus = Some(content_focus);
        state.more_focus = Some(more_focus);
        state.overflow_scroll = Some(overflow_scroll);
        if self.widget(app).is_open {
            // `OnLoaded` with `IsOpen` set: no transition runs, so no `Opened` follows.
            self.begin_opening(app, false);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &CommandBar) {
        self.sync_commands(app);
        let widget = self.widget(app).clone();
        if old.is_dynamic_overflow_enabled != widget.is_dynamic_overflow_enabled {
            app.get(self).overflow.borrow_mut().reset_dynamic_commands();
        }
        if old.is_open != widget.is_open {
            if widget.is_open {
                self.begin_opening(app, true);
            } else {
                self.begin_closing(app);
            }
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(ticker) = app.get_mut(self).ticker.take() {
            ticker.dispose(app);
        }
        let state = app.get_mut(self);
        let mut nodes: Vec<FocusNodeRef> = std::mem::take(&mut state.primary_focus);
        nodes.extend(state.overflow_focus.drain().map(|(_, node)| node));
        nodes.extend(state.content_focus.take());
        nodes.extend(state.more_focus.take());
        let scroll = state.overflow_scroll.take();
        for node in nodes {
            node.dispose(app);
        }
        if let Some(scroll) = scroll {
            scroll.dispose(app);
        }
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        app.get_mut(self).timing = TransitionTiming {
            normal: r.control_normal_animation_duration,
            fast: r.control_fast_animation_duration,
            key_spline: r.control_fast_out_slow_in_key_spline,
        };
        let in_overlay = app.get(self).phase != CommandBarDisplayPhase::Closed;
        let child = if in_overlay {
            // The bar's place keeps its closed size while the bar is in the overlay.
            let metrics = app.get(self).metrics.clone();
            LayoutBuilder::new(move |_, _, constraints| {
                let constraints = constraints.constraints();
                let width = if constraints.max_width.is_finite() {
                    constraints.max_width
                } else {
                    metrics.width.get()
                };
                SizedBox::new()
                    .width(width)
                    .height(metrics.closed_height.get())
                    .into_widget()
            })
            .into_widget()
        } else {
            self.bar(app, context)
        };
        let portal = OverlayPortal::overlay_child_layout_builder(
            app.get(self).portal.clone().expect("made in `init_state`"),
            move |app, context, info| self.overlay(app, context, info),
        )
        .overlay_location(OverlayChildLocation::RootOverlay)
        .child(child);
        Focus::new(portal)
            .can_request_focus(false)
            .skip_traversal(true)
            .on_key_event(Rc::new(move |app, _, event| self.bar_key(app, event)))
            .into_widget()
    }
}
