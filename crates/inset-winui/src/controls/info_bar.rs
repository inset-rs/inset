//! InfoBar.xaml and InfoBar.cpp: severity, adaptive content layout and cancelable closing.

use crate::*;
use inset_embedder::Color;
use inset_foundation::{App, Handle, Listener};
use inset_painting::Alignment;
use inset_rendering::BoxConstraints;
use inset_scheduler::{FrameCallback, SchedulerBinding};
use inset_widgets::*;
use std::{fmt, rc::Rc};

/// Severity of a notification shown by InfoBar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InfoBarSeverity {
    /// General information.
    #[default]
    Informational,

    /// Successful completion.
    Success,

    /// A condition that needs attention.
    Warning,

    /// An operation that failed.
    Error,
}

/// Cause of an InfoBar close.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InfoBarCloseReason {
    /// Activation of the close button.
    CloseButton,

    /// An owner update to IsOpen.
    Programmatic,
}

/// Mutable arguments supplied before a close is applied.
#[derive(Clone, Copy, Debug)]
pub struct InfoBarClosingEventArgs {
    /// The operation that requested closure.
    pub reason: InfoBarCloseReason,

    /// Set true to keep the banner open.
    pub cancel: bool,
}

/// Arguments supplied after an uncanceled close.
#[derive(Clone, Copy, Debug)]
pub struct InfoBarClosedEventArgs {
    /// The operation that closed the banner.
    pub reason: InfoBarCloseReason,
}

/// Receives the cancelable Closing event.
pub type InfoBarClosingHandler = Rc<dyn Fn(&mut App, &mut InfoBarClosingEventArgs)>;

/// Receives the Closed event.
pub type InfoBarClosedHandler = Rc<dyn Fn(&mut App, InfoBarClosedEventArgs)>;

/// Requests an owner update to IsOpen.
pub type InfoBarOpenChangedHandler = Rc<dyn Fn(&mut App, bool)>;

/// An inline notification with a severity icon, optional action and cancelable close button.
#[derive(Clone)]
pub struct InfoBar {
    /// Whether the owner wants the banner open.
    pub is_open: bool,

    /// Applies close requests and restores true when a programmatic close is canceled.
    pub is_open_changed: InfoBarOpenChangedHandler,

    /// Heading text.
    pub title: String,

    /// Supporting message text.
    pub message: String,

    /// Severity controlling the default background and icon.
    pub severity: InfoBarSeverity,

    /// Optional custom icon scaled into the source icon box.
    pub icon_source: Option<WidgetRef>,

    /// Whether the standard or custom icon is visible.
    pub is_icon_visible: bool,

    /// Whether the close button is shown.
    pub is_closable: bool,

    /// Action content laid out after the title and message.
    pub action_button: Option<WidgetRef>,

    /// Additional content below the banner, or in its place when the banner is empty.
    pub content: Option<WidgetRef>,

    /// Optional title and message foreground.
    pub foreground: Option<Color>,

    /// Optional background drawn over the severity background.
    pub background: Option<Brush>,

    /// Optional outer border brush.
    pub border_brush: Option<Brush>,

    /// Outer border widths in left, top, right, bottom order; the theme's `InfoBarBorderThickness` when `None`.
    pub border_thickness: Option<[f64; 4]>,

    /// Outer and background corner radius; the theme's `ControlCornerRadius` when `None`.
    pub corner_radius: Option<f64>,

    /// Optional close-button template, retaining the source close behavior.
    pub close_button_style: Option<ButtonTemplate>,

    /// Runs before the close button raises Closing.
    pub close_button_click: Option<Listener>,

    /// Can cancel either a close-button request or a programmatic close.
    pub closing: Option<InfoBarClosingHandler>,

    /// Receives the reason after an uncanceled close is applied.
    pub closed: Option<InfoBarClosedHandler>,

    /// Runs when the owner opens the banner, including a canceled close restored to open.
    pub opened: Option<Listener>,

    /// Stable widget identity.
    pub key: Option<KeyRef>,
}

impl InfoBar {
    /// Creates an InfoBar whose owner applies requested IsOpen changes.
    pub fn new(is_open: bool, changed: impl Fn(&mut App, bool) + 'static) -> Self {
        Self {
            is_open,
            is_open_changed: Rc::new(changed),
            title: String::new(),
            message: String::new(),
            severity: InfoBarSeverity::Informational,
            icon_source: None,
            is_icon_visible: true,
            is_closable: true,
            action_button: None,
            content: None,
            foreground: None,
            background: None,
            border_brush: None,
            border_thickness: None,
            corner_radius: None,
            close_button_style: None,
            close_button_click: None,
            closing: None,
            closed: None,
            opened: None,
            key: None,
        }
    }

    /// Sets Title.
    pub fn title(mut self, value: impl Into<String>) -> Self {
        self.title = value.into();
        self
    }

    /// Sets Message.
    pub fn message(mut self, value: impl Into<String>) -> Self {
        self.message = value.into();
        self
    }

    /// Sets Severity.
    pub fn severity(mut self, value: InfoBarSeverity) -> Self {
        self.severity = value;
        self
    }

    /// Sets IconSource.
    pub fn icon_source<K>(mut self, value: impl IntoWidget<K>) -> Self {
        self.icon_source = Some(value.into_widget());
        self
    }

    /// Sets IsIconVisible.
    pub fn is_icon_visible(mut self, value: bool) -> Self {
        self.is_icon_visible = value;
        self
    }

    /// Sets IsClosable.
    pub fn is_closable(mut self, value: bool) -> Self {
        self.is_closable = value;
        self
    }

    /// Sets ActionButton.
    pub fn action_button<K>(mut self, value: impl IntoWidget<K>) -> Self {
        self.action_button = Some(value.into_widget());
        self
    }

    /// Sets Content.
    pub fn content<K>(mut self, value: impl IntoWidget<K>) -> Self {
        self.content = Some(value.into_widget());
        self
    }

    /// Sets Foreground.
    pub fn foreground(mut self, value: Color) -> Self {
        self.foreground = Some(value);
        self
    }

    /// Sets Background.
    pub fn background(mut self, value: Brush) -> Self {
        self.background = Some(value);
        self
    }

    /// Sets BorderBrush.
    pub fn border_brush(mut self, value: Brush) -> Self {
        self.border_brush = Some(value);
        self
    }

    /// Sets BorderThickness.
    pub fn border_thickness(mut self, value: [f64; 4]) -> Self {
        self.border_thickness = Some(value);
        self
    }

    /// Sets CornerRadius.
    pub fn corner_radius(mut self, value: f64) -> Self {
        self.corner_radius = Some(value);
        self
    }

    /// Sets CloseButtonStyle.
    pub fn close_button_style(mut self, value: ButtonTemplate) -> Self {
        self.close_button_style = Some(value);
        self
    }

    /// Sets CloseButtonClick.
    pub fn close_button_click(mut self, value: Listener) -> Self {
        self.close_button_click = Some(value);
        self
    }

    /// Sets Closing.
    pub fn closing(
        mut self,
        value: impl Fn(&mut App, &mut InfoBarClosingEventArgs) + 'static,
    ) -> Self {
        self.closing = Some(Rc::new(value));
        self
    }

    /// Sets Closed.
    pub fn closed(mut self, value: impl Fn(&mut App, InfoBarClosedEventArgs) + 'static) -> Self {
        self.closed = Some(Rc::new(value));
        self
    }

    /// Sets Opened.
    pub fn opened(mut self, value: Listener) -> Self {
        self.opened = Some(value);
        self
    }

    /// Sets Key.
    pub fn key(mut self, value: KeyRef) -> Self {
        self.key = Some(value);
        self
    }
}

impl fmt::Debug for InfoBar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InfoBar")
            .field("is_open", &self.is_open)
            .field("severity", &self.severity)
            .finish_non_exhaustive()
    }
}

/// Visibility retained while a programmatic close waits for its cancelable event.
pub struct InfoBarState {
    /// Framework state identity.
    state: StateData<InfoBar>,

    /// Visibility of ContentRoot, independent of a pending close request.
    visible: bool,

    /// A close-button request whose Closing event has already run.
    pending_close: Option<InfoBarCloseReason>,

    /// Invalidates delayed lifecycle callbacks when a newer owner update arrives.
    epoch: u64,
}

impl StatefulWidget for InfoBar {
    type State = InfoBarState;

    /// Returns the identity used to retain this control across owner rebuilds.
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    /// Initializes template state from the first widget description.
    fn create_state(&self) -> InfoBarState {
        InfoBarState {
            state: StateData::new(),
            visible: self.is_open,
            pending_close: None,
            epoch: 0,
        }
    }
}

impl InfoBarState {
    /// Raises Closing and reports whether its Cancel property was set.
    fn canceled(self: Handle<Self>, app: &mut App, reason: InfoBarCloseReason) -> bool {
        let mut args = InfoBarClosingEventArgs {
            reason,
            cancel: false,
        };
        if let Some(callback) = self.widget(app).closing.clone() {
            callback(app, &mut args);
        }
        args.cancel
    }

    /// The close button raises Click before attempting to change IsOpen.
    fn close_button(self: Handle<Self>, app: &mut App) {
        if !self.widget(app).is_open || app.get(self).pending_close.is_some() {
            return;
        }
        if let Some(callback) = self.widget(app).close_button_click.clone() {
            callback.call(app);
        }

        let reason = InfoBarCloseReason::CloseButton;
        if self.canceled(app, reason) {
            // Source cancellation restores IsOpen=true and raises Opened again.
            if let Some(callback) = self.widget(app).opened.clone() {
                callback.call(app);
            }
            return;
        }

        app.get_mut(self).pending_close = Some(reason);
        let changed = self.widget(app).is_open_changed.clone();
        changed(app, false);
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _| {
                if app.contains(self) && self.mounted(app) && self.widget(app).is_open {
                    app.get_mut(self).pending_close = None;
                }
            }),
            View::maybe_id_of(app, self.context(app)),
        );
    }

    /// Applies a close after its cancelable event and notifies the owner of cancellation.
    fn finish_close(
        self: Handle<Self>,
        app: &mut App,
        reason: InfoBarCloseReason,
        already_raised: bool,
    ) {
        if !already_raised && self.canceled(app, reason) {
            let changed = self.widget(app).is_open_changed.clone();
            changed(app, true);
            return;
        }

        self.set_state(app, |state| state.visible = false);
        if let Some(callback) = self.widget(app).closed.clone() {
            callback(app, InfoBarClosedEventArgs { reason });
        }
    }
}

impl State for InfoBarState {
    type Widget = InfoBar;
    inset_widgets::state_accessors!();

    /// Applies source property-change behavior when the owner supplies new values.
    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &InfoBar) {
        if old.is_open == self.widget(app).is_open {
            return;
        }
        app.get_mut(self).epoch += 1;
        let epoch = app.get(self).epoch;
        let open = self.widget(app).is_open;
        let pending = app.get_mut(self).pending_close.take();
        if open {
            app.get_mut(self).visible = true;
        }

        // Native owner callbacks run after the rebuilding ancestor has completed.
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _| {
                if !app.contains(self) || !self.mounted(app) || app.get(self).epoch != epoch {
                    return;
                }
                if open {
                    if let Some(callback) = self.widget(app).opened.clone() {
                        callback.call(app);
                    }
                } else {
                    self.finish_close(
                        app,
                        pending.unwrap_or(InfoBarCloseReason::Programmatic),
                        pending.is_some(),
                    );
                }
            }),
            View::maybe_id_of(app, self.context(app)),
        );
    }

    /// Builds the named template parts from the current visual state.
    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let theme = ThemeResources::of(app, context);
        let resources = &theme;
        let (background, icon_background, icon_foreground, symbol) =
            severity_brushes(resources, widget.severity);
        let none = Brush::Solid(Color::from_argb(0, 0, 0, 0));
        let banner_empty =
            widget.title.is_empty() && widget.message.is_empty() && widget.action_button.is_none();

        let title = Text::new(widget.title.clone()).style(control_text_style(
            &theme,
            resources.info_bar_title_font_size,
            resources.info_bar_title_font_weight,
            widget
                .foreground
                .unwrap_or(resources.info_bar_title_foreground),
        ));
        let message = Text::new(widget.message.clone()).style(control_text_style(
            &theme,
            resources.info_bar_message_font_size,
            resources.info_bar_message_font_weight,
            widget
                .foreground
                .unwrap_or(resources.info_bar_message_foreground),
        ));
        let action = widget
            .action_button
            .clone()
            .map(|child| {
                if downcast_widget::<HyperlinkButton>(child.as_ref()).is_some() {
                    Margin::new(resources.info_bar_hyperlink_button_margin, child).into_widget()
                } else {
                    child
                }
            })
            .unwrap_or_else(|| SizedBox::shrink().into_widget());
        let panel = InfoBarPanel::new()
            .horizontal_orientation_padding(resources.info_bar_panel_horizontal_orientation_padding)
            .vertical_orientation_padding(resources.info_bar_panel_vertical_orientation_padding)
            .parent_min_height(
                resources.info_bar_min_height
                    - resources.info_bar_panel_margin[1]
                    - resources.info_bar_panel_margin[3],
            )
            .children([
                InfoBarPanelChild::new(title)
                    .horizontal_orientation_margin(
                        resources.info_bar_title_horizontal_orientation_margin,
                    )
                    .vertical_orientation_margin(
                        resources.info_bar_title_vertical_orientation_margin,
                    )
                    .into_widget(),
                InfoBarPanelChild::new(message)
                    .horizontal_orientation_margin(
                        resources.info_bar_message_horizontal_orientation_margin,
                    )
                    .vertical_orientation_margin(
                        resources.info_bar_message_vertical_orientation_margin,
                    )
                    .into_widget(),
                InfoBarPanelChild::new(
                    Align::new()
                        .alignment(Alignment::TOP_LEFT.into())
                        .width_factor(1.0)
                        .height_factor(1.0)
                        .child(action),
                )
                .horizontal_orientation_margin(
                    resources.info_bar_action_horizontal_orientation_margin,
                )
                .vertical_orientation_margin(resources.info_bar_action_vertical_orientation_margin)
                .into_widget(),
            ]);

        let mut cells = Vec::new();
        if widget.is_icon_visible {
            let icon = widget
                .icon_source
                .clone()
                .map(|icon| {
                    SizedBox::new()
                        .width(resources.info_bar_icon_font_size)
                        .height(resources.info_bar_icon_font_size)
                        .child(FittedBox::new().child(icon))
                        .into_widget()
                })
                .unwrap_or_else(|| {
                    // The source layers two TextBlocks; FontSize does not constrain their line height.
                    Stack::new()
                        .alignment(Alignment::CENTER.into())
                        .children([
                            SizedBox::new()
                                .width(resources.info_bar_icon_font_size)
                                .height(resources.info_bar_icon_font_size)
                                .child(
                                    ControlBorder::new(Brush::Solid(icon_background), none)
                                        .border_thickness(0.0)
                                        .corner_radius(resources.info_bar_icon_font_size / 2.0),
                                )
                                .into_widget(),
                            FontIcon::symbol(symbol)
                                .font_size(resources.info_bar_icon_font_size)
                                .foreground(icon_foreground)
                                .into_widget(),
                        ])
                        .into_widget()
                });
            cells.push(
                GridCell::new(
                    Align::new()
                        .alignment(Alignment::TOP_LEFT.into())
                        .child(Margin::new(resources.info_bar_icon_margin, icon)),
                )
                .into_widget(),
            );
        }
        cells.push(
            GridCell::new(Margin::new(resources.info_bar_panel_margin, panel))
                .column(1)
                .into_widget(),
        );
        if let Some(content) = widget.content.clone() {
            cells.push(
                GridCell::new(
                    Align::new()
                        .alignment(Alignment::CENTER_LEFT.into())
                        .width_factor(1.0)
                        .height_factor(1.0)
                        .child(content),
                )
                .column(1)
                .row(if banner_empty { 0 } else { 1 })
                .into_widget(),
            );
        }
        if widget.is_closable {
            let mut button = Button::new(
                FontIcon::symbol(FluentSymbol::Dismiss)
                    .font_size(resources.info_bar_close_button_glyph_size),
                Listener::new(move |app| self.close_button(app)),
            )
            .template(close_button_template);
            if let Some(style) = widget.close_button_style.clone() {
                button.template = Some(style);
            }
            let button = ToolTipService::new(button, ToolTip::new(Text::new("Close")));
            cells.push(
                GridCell::new(
                    Align::new()
                        .alignment(Alignment::TOP_LEFT.into())
                        .child(Margin::new(
                            [5.0; 4],
                            SizedBox::new()
                                .width(resources.info_bar_close_button_size)
                                .height(resources.info_bar_close_button_size)
                                .child(button),
                        )),
                )
                .column(2)
                .into_widget(),
            );
        }

        let grid = Grid::new()
            .row_definitions([
                RowDefinition::new(GridLength::AUTO),
                RowDefinition::new(GridLength::AUTO),
            ])
            .column_definitions([
                ColumnDefinition::new(GridLength::AUTO),
                ColumnDefinition::new(GridLength::STAR),
                ColumnDefinition::new(GridLength::AUTO),
            ])
            .background(widget.background.unwrap_or(none))
            .corner_radius(
                widget
                    .corner_radius
                    .unwrap_or(theme.control_corner_radius[0]),
            )
            .padding(resources.info_bar_content_root_padding)
            .children(cells);
        let body = ControlBorder::new(
            Brush::Solid(background),
            widget
                .border_brush
                .unwrap_or(Brush::Solid(resources.info_bar_border_brush)),
        )
        .border_thickness_ltrb(
            widget
                .border_thickness
                .unwrap_or(theme.info_bar_border_thickness),
        )
        .corner_radius(
            widget
                .corner_radius
                .unwrap_or(theme.control_corner_radius[0]),
        )
        .child(
            ConstrainedBox::new(BoxConstraints::new().min_height(resources.info_bar_min_height))
                .child(grid),
        );
        let visible = app.get(self).visible;
        Offstage::new()
            .offstage(!visible)
            .child(TickerMode::new(
                visible,
                ExcludeFocus::new(body).excluding(!visible),
            ))
            .into_widget()
    }
}

/// The SeverityLevels setters and the default Informational values.
fn severity_brushes(
    r: &ThemeResources,
    severity: InfoBarSeverity,
) -> (Color, Color, Color, FluentSymbol) {
    match severity {
        InfoBarSeverity::Informational => (
            r.info_bar_informational_severity_background_brush,
            r.info_bar_informational_severity_icon_background,
            r.info_bar_informational_severity_icon_foreground,
            FluentSymbol::Info,
        ),
        InfoBarSeverity::Success => (
            r.info_bar_success_severity_background_brush,
            r.info_bar_success_severity_icon_background,
            r.info_bar_success_severity_icon_foreground,
            FluentSymbol::CheckmarkCircle,
        ),
        InfoBarSeverity::Warning => (
            r.info_bar_warning_severity_background_brush,
            r.info_bar_warning_severity_icon_background,
            r.info_bar_warning_severity_icon_foreground,
            FluentSymbol::ErrorCircle,
        ),
        InfoBarSeverity::Error => (
            r.info_bar_error_severity_background_brush,
            r.info_bar_error_severity_icon_background,
            r.info_bar_error_severity_icon_foreground,
            FluentSymbol::DismissCircle,
        ),
    }
}

/// DefaultButtonStyle with the source AppBarButton resource overrides.
fn close_button_template(
    app: &mut App,
    context: BuildContext,
    states: ControlStates,
    content: WidgetRef,
) -> WidgetRef {
    let theme = ThemeResources::of(app, context);
    let r = theme.clone();
    let (background, border, foreground) = match states.common {
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
        _ => (
            r.app_bar_button_background,
            r.app_bar_button_border_brush,
            r.app_bar_button_foreground,
        ),
    };
    let content = Center::new()
        .child(DefaultTextStyle::new(
            control_text_style(
                &theme,
                r.control_content_theme_font_size,
                inset_embedder::FontWeight::NORMAL,
                foreground,
            ),
            content,
        ))
        .into_widget();
    let body = ColorTransition::new(
        background,
        r.control_faster_animation_duration,
        move |_, color| {
            ControlBorder::new(Brush::Solid(color), Brush::Solid(border))
                .border_thickness_ltrb(r.button_border_theme_thickness)
                .corner_radius(r.control_corner_radius[0])
                .child(content.clone())
                .into_widget()
        },
    );
    FocusVisual::new(body, theme.theme)
        .visible(states.focused)
        .into_widget()
}
