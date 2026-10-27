//! Expander.xaml, Expander header styles and Expander.cpp state transitions.

use crate::*;
use inset_animation::{Cubic, Curve};
use inset_embedder::{FontWeight, Offset};
use inset_foundation::{App, Handle, Listener};
use inset_painting::{Alignment, EdgeInsetsGeometry};
use inset_rendering::BoxConstraints;
use inset_scheduler::{FrameCallback, SchedulerBinding, Ticker};
use inset_widgets::*;
use std::{fmt, rc::Rc, time::Duration};

/// Direction in which an Expander reveals its content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExpandDirection {
    /// The header is above its content.
    #[default]
    Down,

    /// The header is below its content.
    Up,
}

/// Requests an owner update to IsExpanded.
pub type ExpanderExpandedChangedHandler = Rc<dyn Fn(&mut App, bool)>;

/// A header toggle that reveals or hides retained content using the Fluent template.
#[derive(Clone)]
pub struct Expander {
    /// Whether the content is expanded.
    pub is_expanded: bool,

    /// Requests a new expanded value from the owner.
    pub is_expanded_changed: ExpanderExpandedChangedHandler,

    /// Content displayed by the header toggle.
    pub header: Option<WidgetRef>,

    /// Content retained while the Expander is collapsed.
    pub content: Option<WidgetRef>,

    /// Whether content is above or below the header.
    pub expand_direction: ExpandDirection,

    /// Whether the header and content accept input.
    pub is_enabled: bool,

    /// Optional content background.
    pub background: Option<Brush>,

    /// Optional content border.
    pub border_brush: Option<Brush>,

    /// Content padding in left, top, right, bottom order; the theme's `ExpanderContentPadding` when `None`.
    pub padding: Option<[f64; 4]>,

    /// Corner radius shared by the header and content; the theme's `ControlCornerRadius` when `None`.
    pub corner_radius: Option<f64>,

    /// Minimum width of the control; the theme's `FlyoutThemeMinWidth` when `None`.
    pub min_width: Option<f64>,

    /// Minimum height of both header and content borders; the theme's `ExpanderMinHeight` when `None`.
    pub min_height: Option<f64>,

    /// Called when IsExpanded changes to true, before the opening animation completes.
    pub expanding: Option<Listener>,

    /// Called when IsExpanded changes to false, rather than after the collapse animation.
    pub collapsed: Option<Listener>,

    /// Stable identity across rebuilds.
    pub key: Option<KeyRef>,
}

impl Expander {
    /// Creates an Expander whose expanded value is owned by its caller.
    pub fn new(is_expanded: bool, changed: impl Fn(&mut App, bool) + 'static) -> Self {
        Self {
            is_expanded,
            is_expanded_changed: Rc::new(changed),
            header: None,
            content: None,
            expand_direction: ExpandDirection::Down,
            is_enabled: true,
            background: None,
            border_brush: None,
            padding: None,
            corner_radius: None,
            min_width: None,
            min_height: None,
            expanding: None,
            collapsed: None,
            key: None,
        }
    }

    /// Sets Header.
    pub fn header<K>(mut self, value: impl IntoWidget<K>) -> Self {
        self.header = Some(value.into_widget());
        self
    }

    /// Sets Content.
    pub fn content<K>(mut self, value: impl IntoWidget<K>) -> Self {
        self.content = Some(value.into_widget());
        self
    }

    /// Sets ExpandDirection.
    pub fn expand_direction(mut self, value: ExpandDirection) -> Self {
        self.expand_direction = value;
        self
    }

    /// Sets IsEnabled.
    pub fn is_enabled(mut self, value: bool) -> Self {
        self.is_enabled = value;
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

    /// Sets Padding.
    pub fn padding(mut self, value: [f64; 4]) -> Self {
        self.padding = Some(value);
        self
    }

    /// Sets CornerRadius.
    pub fn corner_radius(mut self, value: f64) -> Self {
        self.corner_radius = Some(value);
        self
    }

    /// Sets MinWidth.
    pub fn min_width(mut self, value: f64) -> Self {
        self.min_width = Some(value);
        self
    }

    /// Sets MinHeight.
    pub fn min_height(mut self, value: f64) -> Self {
        self.min_height = Some(value);
        self
    }

    /// Sets Expanding.
    pub fn expanding(mut self, value: Listener) -> Self {
        self.expanding = Some(value);
        self
    }

    /// Sets Collapsed.
    pub fn collapsed(mut self, value: Listener) -> Self {
        self.collapsed = Some(value);
        self
    }

    /// Sets Key.
    pub fn key(mut self, value: KeyRef) -> Self {
        self.key = Some(value);
        self
    }
}

impl fmt::Debug for Expander {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Expander")
            .field("is_expanded", &self.is_expanded)
            .field("expand_direction", &self.expand_direction)
            .finish_non_exhaustive()
    }
}

/// Expander's animation and retained content lifecycle.
pub struct ExpanderState {
    /// Framework state identity.
    state: StateData<Expander>,

    /// Native TickerMode subscription.
    single_ticker_provider: SingleTickerProviderStateMixinData,

    /// Clock for the explicit ExpandStates storyboard.
    ticker: Option<Handle<Ticker>>,

    /// Elapsed time since the last IsExpanded change.
    elapsed: Duration,

    /// Whether the collapsed visibility key has executed.
    content_visible: bool,

    /// Direction captured by ExpandStates, independent of ExpandDirectionStates.
    motion_direction: ExpandDirection,
}

impl StatefulWidget for Expander {
    type State = ExpanderState;

    /// Returns the identity used to retain this control across owner rebuilds.
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    /// Initializes template state from the first widget description.
    fn create_state(&self) -> ExpanderState {
        ExpanderState {
            state: StateData::new(),
            single_ticker_provider: Default::default(),
            ticker: None,
            elapsed: Duration::ZERO,
            content_visible: self.is_expanded,
            motion_direction: self.expand_direction,
        }
    }
}

impl SingleTickerProviderStateMixin for ExpanderState {
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

impl ExpanderState {
    /// Duration of the last storyboard key, including CollapseDown's delayed visibility key.
    fn duration(self: Handle<Self>, app: &App) -> Duration {
        Duration::from_millis(if self.widget(app).is_expanded {
            333
        } else if app.get(self).motion_direction == ExpandDirection::Up {
            200
        } else {
            167
        })
    }

    /// Applies the source translate keys and collapses only at the visibility key.
    fn tick(self: Handle<Self>, app: &mut App, elapsed: Duration) {
        let finished = elapsed >= self.duration(app);
        let expanded = self.widget(app).is_expanded;
        self.set_state(app, |state| {
            state.elapsed = elapsed;
            if finished && !expanded {
                state.content_visible = false;
            }
        });

        if finished {
            app.get(self).ticker.unwrap().stop(app, false);
        }
    }

    /// Translation as a fraction of the actual content height, avoiding a delayed size sample.
    fn translation(self: Handle<Self>, app: &App) -> f64 {
        let time = app.get(self).elapsed.as_secs_f64();
        let sign = if app.get(self).motion_direction == ExpandDirection::Down {
            -1.0
        } else {
            1.0
        };
        if self.widget(app).is_expanded {
            sign * (1.0 - Cubic::new(0.0, 0.0, 0.0, 1.0).transform((time / 0.333).min(1.0)))
        } else {
            sign * Cubic::new(1.0, 1.0, 0.0, 1.0).transform((time / 0.167).min(1.0))
        }
    }
}

impl State for ExpanderState {
    type Widget = Expander;
    inset_widgets::state_accessors!();

    /// Creates the native clock for the source storyboards.
    fn init_state(self: Handle<Self>, app: &mut App) {
        let ticker = SingleTickerProviderStateMixin::create_ticker(
            self,
            app,
            FrameCallback::new(move |app, elapsed| self.tick(app, elapsed)),
        );
        app.get_mut(self).ticker = Some(ticker);
        if self.widget(app).is_expanded {
            ticker.start(app);
        }
    }

    /// Applies source property-change behavior when the owner supplies new values.
    fn did_update_widget(self: Handle<Self>, app: &mut App, old: &Expander) {
        let widget = self.widget(app).clone();
        if old.is_expanded == widget.is_expanded {
            return;
        }

        let ticker = app.get(self).ticker.unwrap();
        ticker.stop(app, false);
        let state = app.get_mut(self);
        state.elapsed = Duration::ZERO;
        state.content_visible = true;
        state.motion_direction = widget.expand_direction;
        ticker.start(app);

        let event = if widget.is_expanded {
            widget.expanding
        } else {
            widget.collapsed
        };
        if let Some(event) = event {
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _| {
                    if app.contains(self) && self.mounted(app) {
                        event.call(app);
                    }
                }),
                View::maybe_id_of(app, self.context(app)),
            );
        }
    }

    fn activate(self: Handle<Self>, app: &mut App) {
        SingleTickerProviderStateMixin::activate(self, app);
    }

    /// Releases the storyboard clock before disposing its ticker provider.
    fn dispose(self: Handle<Self>, app: &mut App) {
        app.get(self).ticker.unwrap().dispose(app);
        SingleTickerProviderStateMixin::dispose(self, app);
    }

    /// Builds the named template parts from the current visual state.
    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let resources = ThemeResources::of(app, context);
        let header_widget = widget.clone();
        let header_down = app.get(self).motion_direction == ExpandDirection::Down;
        let changed = widget.is_expanded_changed.clone();
        let header = ToggleButton::new(Some(widget.is_expanded), move |app, value| {
            changed(app, value == Some(true))
        })
        .content(
            widget
                .header
                .clone()
                .unwrap_or_else(|| SizedBox::shrink().into_widget()),
        )
        .is_enabled(widget.is_enabled)
        .template(move |app, context, _, content, states| {
            expander_header(
                app,
                context,
                &header_widget,
                header_down,
                content.unwrap_or_else(|| SizedBox::shrink().into_widget()),
                states,
            )
        });

        let down = widget.expand_direction == ExpandDirection::Down;
        let content = ControlBorder::new(
            widget
                .background
                .unwrap_or(Brush::Solid(resources.expander_content_background)),
            widget
                .border_brush
                .unwrap_or(Brush::Solid(resources.expander_content_border_brush)),
        )
        .border_thickness_ltrb(if down {
            resources.expander_content_down_border_thickness
        } else {
            resources.expander_content_up_border_thickness
        })
        .corner_radius_corners(if down {
            [
                0.0,
                0.0,
                widget
                    .corner_radius
                    .unwrap_or(resources.control_corner_radius[0]),
                widget
                    .corner_radius
                    .unwrap_or(resources.control_corner_radius[0]),
            ]
        } else {
            [
                widget
                    .corner_radius
                    .unwrap_or(resources.control_corner_radius[0]),
                widget
                    .corner_radius
                    .unwrap_or(resources.control_corner_radius[0]),
                0.0,
                0.0,
            ]
        })
        .padding(widget.padding.unwrap_or(resources.expander_content_padding))
        .child(
            widget
                .content
                .clone()
                .unwrap_or_else(|| SizedBox::shrink().into_widget()),
        );
        let content = ConstrainedBox::new(
            BoxConstraints::new()
                .min_height(widget.min_height.unwrap_or(resources.expander_min_height)),
        )
        .child(content);
        let visible = app.get(self).content_visible;

        // ExpanderContentClip contains the translated ExpanderContent; height is reserved immediately.
        let content = Offstage::new().offstage(!visible).child(TickerMode::new(
            visible,
            ExcludeFocus::new(ClipRect::new().child(
                FractionalTranslation::new(Offset::new(0.0, self.translation(app))).child(content),
            ))
            .excluding(!visible),
        ));
        let grid = Grid::new()
            .row_definitions([
                RowDefinition::new(if down {
                    GridLength::AUTO
                } else {
                    GridLength::STAR
                }),
                RowDefinition::new(if down {
                    GridLength::STAR
                } else {
                    GridLength::AUTO
                }),
            ])
            .children([
                GridCell::new(header)
                    .row(if down { 0 } else { 1 })
                    .into_widget(),
                GridCell::new(content)
                    .row(if down { 1 } else { 0 })
                    .into_widget(),
            ]);

        ExcludeFocus::new(
            IgnorePointer::new().ignoring(!widget.is_enabled).child(
                ConstrainedBox::new(
                    BoxConstraints::new()
                        .min_width(widget.min_width.unwrap_or(resources.flyout_theme_min_width)),
                )
                .child(grid),
            ),
        )
        .excluding(!widget.is_enabled)
        .into_widget()
    }
}

/// ExpanderHeaderDownStyle / ExpanderHeaderUpStyle common and checked visual states.
fn expander_header(
    app: &mut App,
    context: BuildContext,
    widget: &Expander,
    header_down: bool,
    content: WidgetRef,
    states: ControlStates,
) -> WidgetRef {
    let theme = ThemeResources::of(app, context);
    let r = &theme;
    let (foreground, border, chevron_fill, chevron_border, chevron_foreground) = match states.common
    {
        CommonState::PointerOver => (
            r.expander_header_foreground_pointer_over,
            r.expander_header_border_pointer_over_brush,
            r.expander_chevron_pointer_over_background,
            r.expander_chevron_border_pointer_over_brush,
            r.expander_chevron_pointer_over_foreground,
        ),
        CommonState::Pressed => (
            r.expander_header_foreground_pressed,
            r.expander_header_border_pressed_brush,
            r.expander_chevron_pressed_background,
            r.expander_chevron_border_pressed_brush,
            r.expander_chevron_pressed_foreground,
        ),
        CommonState::Disabled => (
            r.expander_header_disabled_foreground,
            r.expander_header_disabled_border_brush,
            r.expander_chevron_background,
            r.expander_header_disabled_border_brush,
            r.expander_header_disabled_foreground,
        ),
        _ => (
            r.expander_header_foreground,
            r.expander_header_border_brush,
            r.expander_chevron_background,
            r.expander_chevron_border_brush,
            r.expander_chevron_foreground,
        ),
    };
    let radii = if widget.is_expanded {
        if header_down {
            [
                widget.corner_radius.unwrap_or(r.control_corner_radius[0]),
                widget.corner_radius.unwrap_or(r.control_corner_radius[0]),
                0.0,
                0.0,
            ]
        } else {
            [
                0.0,
                0.0,
                widget.corner_radius.unwrap_or(r.control_corner_radius[0]),
                widget.corner_radius.unwrap_or(r.control_corner_radius[0]),
            ]
        }
    } else {
        [widget.corner_radius.unwrap_or(r.control_corner_radius[0]); 4]
    };
    let points_up = widget.is_expanded == (widget.expand_direction == ExpandDirection::Down);

    // Native rotation is the established AnimatedIcon substitute used by NavigationView.
    let chevron = AnimatedRotation::new(
        if points_up { 0.5 } else { 0.0 },
        r.control_normal_animation_duration,
    )
    .child(
        FontIcon::symbol(FluentSymbol::ChevronDown)
            .font_size(r.expander_chevron_glyph_size)
            .foreground(chevron_foreground),
    );
    let chevron = Padding::new(EdgeInsetsGeometry::from_ltrb(
        r.expander_chevron_margin[0],
        r.expander_chevron_margin[1],
        r.expander_chevron_margin[2],
        r.expander_chevron_margin[3],
    ))
    .child(
        SizedBox::new()
            .width(r.expander_chevron_button_size)
            .height(r.expander_chevron_button_size)
            .child(
                ControlBorder::new(Brush::Solid(chevron_fill), Brush::Solid(chevron_border))
                    .border_thickness_ltrb(r.expander_chevron_border_thickness)
                    .corner_radius(r.control_corner_radius[0])
                    .child(Center::new().child(chevron)),
            ),
    );
    let layout = Grid::new()
        .column_definitions([
            ColumnDefinition::new(GridLength::STAR),
            ColumnDefinition::new(GridLength::AUTO),
        ])
        .children([
            GridCell::new(
                Align::new()
                    .alignment(Alignment::CENTER_LEFT.into())
                    .width_factor(1.0)
                    .height_factor(1.0)
                    .child(DefaultTextStyle::new(
                        control_text_style(
                            &theme,
                            r.control_content_theme_font_size,
                            FontWeight::NORMAL,
                            foreground,
                        ),
                        content,
                    )),
            )
            .into_widget(),
            GridCell::new(Center::new().child(chevron))
                .column(1)
                .into_widget(),
        ]);
    FocusVisual::new(
        ConstrainedBox::new(
            BoxConstraints::new().min_height(widget.min_height.unwrap_or(r.expander_min_height)),
        )
        .child(
            ControlBorder::new(
                Brush::Solid(r.expander_header_background),
                Brush::Solid(border),
            )
            .border_thickness_ltrb(r.expander_header_border_thickness)
            .corner_radius_corners(radii)
            .padding(r.expander_header_padding)
            .child(layout),
        ),
        theme.theme,
    )
    .visible(states.focused)
    .into_widget()
}
