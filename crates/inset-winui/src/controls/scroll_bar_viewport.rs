//! Source ScrollBar chrome around a native viewport, shared by navigation and flyouts.

use crate::*;
use inset_embedder::{Color, Radius, TextDirection};
use inset_foundation::{App, Handle, Listener, Timer};
use inset_painting::{Axis, EdgeInsetsGeometry, TextStyle};
use inset_widgets::*;
use std::{rc::Rc, time::Duration};

/// Native single-axis viewport with the source horizontal or vertical scrollbar template.
#[derive(Clone, Debug)]
pub(crate) struct ScrollBarViewport {
    /// Caller-owned scroll state, also used by the native scrollbar drag recognizer.
    controller: Handle<ScrollViewportController>,
    /// Content measured by the native single-child scrolling viewport, or a scroller of its own.
    child: WidgetRef,
    /// Scroll direction; navigation uses vertical and flyouts may use either axis.
    axis: Axis,
    /// Whether `child` is a native scroller attached to `controller`, such as a lazy ListView,
    /// rather than content to put in a viewport.
    child_scrolls: bool,
}

impl ScrollBarViewport {
    /// Wraps content without changing controller ownership or native focus scrolling.
    pub(crate) fn new<K>(
        controller: Handle<ScrollViewportController>,
        child: impl IntoWidget<K>,
    ) -> Self {
        Self {
            controller,
            child: child.into_widget(),
            axis: Axis::Vertical,
            child_scrolls: false,
        }
    }

    /// Puts the scrollbar on `scroller`, a native scroller attached to `controller` with
    /// whatever lies between them, such as a lazy ListView inside a fixed margin.
    pub(crate) fn scroller<K>(
        controller: Handle<ScrollViewportController>,
        scroller: impl IntoWidget<K>,
    ) -> Self {
        Self {
            child_scrolls: true,
            ..Self::new(controller, scroller)
        }
    }

    /// Selects the scrollbar's source horizontal or vertical template.
    pub(crate) fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
}

/// Native hover lifetime and delayed source expansion state.
pub(crate) struct ScrollBarViewportState {
    /// Widget attachment.
    state: StateData<ScrollBarViewport>,
    /// The pointer is inside the scrolling area.
    hovered: bool,
    /// The source Expanded visual state.
    expanded: bool,
    /// Delayed expand/contract callback, canceled on another crossing.
    timer: Option<Timer>,
}

impl StatefulWidget for ScrollBarViewport {
    type State = ScrollBarViewportState;
    fn create_state(&self) -> Self::State {
        ScrollBarViewportState {
            state: StateData::new(),
            hovered: false,
            expanded: false,
            timer: None,
        }
    }
}

impl ScrollBarViewportState {
    /// Applies the source delayed bar-hover state, leaving scrolling to native widgets.
    fn hover_bar(self: Handle<Self>, app: &mut App, enter: bool) {
        let r = ThemeResources::of(app, self.context(app));
        if let Some(timer) = app.get_mut(self).timer.take() {
            timer.cancel(app);
        }
        let timer = Timer::new(
            app,
            if enter {
                r.scroll_bar_expand_begin_time
            } else {
                r.scroll_bar_contract_begin_time
            },
            Listener::new(move |app| {
                if app.contains(self) && self.mounted(app) {
                    self.set_state(app, |s| {
                        s.expanded = enter;
                        s.timer = None;
                    });
                }
            }),
        );
        app.get_mut(self).timer = Some(timer);
    }

    /// ScrollContentPresenter followed by VerticalScrollBar's track and small-change buttons.
    fn template(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let r = ThemeResources::of(app, context);
        let vertical = widget.axis == Axis::Vertical;
        let rtl = Directionality::of(app, context) == TextDirection::Rtl;
        let metrics = widget.controller.metrics(app);
        let overflow = metrics.scrollable_length > 0.0;
        let expanded = app.get(self).expanded && overflow;
        let viewport = if widget.child_scrolls {
            ScrollMetricsObserver::new(widget.controller, widget.child).into_widget()
        } else {
            ScrollViewport::new(widget.axis, widget.controller, widget.child).into_widget()
        };
        let mut parts = vec![viewport];
        if overflow {
            let mut chrome = Vec::new();
            if expanded {
                chrome.push(
                    Positioned::fill(
                        ControlBorder::new(
                            Brush::Acrylic(r.scroll_bar_track_fill),
                            Brush::Solid(Color::new(0)),
                        )
                        .border_thickness(0.0),
                    )
                    .into_widget(),
                );
                for increase in [false, true] {
                    let controller = widget.controller;
                    let button = RepeatButton::new(
                        FontIcon::symbol(if vertical {
                            if increase {
                                FluentSymbol::ChevronDown
                            } else {
                                FluentSymbol::ChevronUp
                            }
                        } else if increase != rtl {
                            FluentSymbol::ChevronRight
                        } else {
                            FluentSymbol::ChevronLeft
                        })
                        .font_size(r.scroll_bar_button_arrow_icon_font_size),
                        Listener::new(move |app| {
                            // ScrollViewerLineDelta in ScrollViewer_Partial.h.
                            controller.scroll_by(app, if increase { 16.0 } else { -16.0 }, true);
                        }),
                    )
                    .is_tab_stop(false)
                    .is_enabled(if increase {
                        metrics.offset < metrics.scrollable_length
                    } else {
                        metrics.offset > 0.0
                    })
                    .interval(Duration::from_millis(50))
                    .template(move |app, context, states, content| {
                        arrow_template(app, context, states, content, increase, vertical)
                    });
                    let button = Positioned::new(button);
                    let button = if vertical {
                        let button = button.left(0.0).right(0.0).height(r.scroll_bar_size);
                        if increase {
                            button.bottom(0.0)
                        } else {
                            button.top(0.0)
                        }
                    } else {
                        let button = button.top(0.0).bottom(0.0).width(r.scroll_bar_size);
                        if increase != rtl {
                            button.right(0.0)
                        } else {
                            button.left(0.0)
                        }
                    };
                    chrome.push(button.into_widget());
                }
            }
            let bar = Positioned::new(
                MouseRegion::new()
                    .opaque(false)
                    .on_enter(Rc::new(move |app, _| self.hover_bar(app, true)))
                    .on_exit(Rc::new(move |app, _| self.hover_bar(app, false)))
                    .child(Stack::new().children(chrome)),
            );
            parts.push(
                if vertical {
                    bar.right(0.0).top(0.0).bottom(0.0).width(r.scroll_bar_size)
                } else {
                    bar.left(0.0)
                        .right(0.0)
                        .bottom(0.0)
                        .height(r.scroll_bar_size)
                }
                .into_widget(),
            );
        }
        // ThumbVisual has a six-pixel transparent stroke; RawScrollbar paints only its visible fill.
        let thickness = if expanded {
            r.scroll_bar_size
        } else {
            if vertical {
                r.scroll_bar_vertical_thumb_min_width
            } else {
                r.scroll_bar_horizontal_thumb_min_height
            }
        } - r.scroll_bar_thumb_stroke_thickness;
        RawScrollbar::new(Stack::new().children(parts))
            .controller(widget.controller.native_controller(app))
            .interactive(true)
            .thumb_visibility(app.get(self).hovered && overflow)
            .thumb_color(r.scroll_bar_thumb_fill)
            .thickness(thickness)
            .radius(Radius::circular(r.scroll_bar_corner_radius[0]))
            .min_thumb_length(
                (if vertical {
                    r.scroll_bar_vertical_thumb_min_height
                } else {
                    r.scroll_bar_horizontal_thumb_min_width
                }) - r.scroll_bar_thumb_stroke_thickness,
            )
            .main_axis_margin(r.scroll_bar_size + r.scroll_bar_thumb_stroke_thickness / 2.0)
            .cross_axis_margin((r.scroll_bar_size - thickness) / 2.0)
            .padding(EdgeInsetsGeometry::ZERO)
            .scrollbar_orientation(if vertical {
                ScrollbarOrientation::Right
            } else {
                ScrollbarOrientation::Bottom
            })
            .fade_duration(r.scroll_bar_opacity_change_duration)
            .time_to_fade(r.scroll_bar_contract_delay)
            .into_widget()
    }
}

impl State for ScrollBarViewportState {
    type Widget = ScrollBarViewport;
    inset_widgets::state_accessors!();
    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(timer) = app.get_mut(self).timer.take() {
            timer.cancel(app);
        }
    }
    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        let controller = self.widget(app).controller;
        MouseRegion::new()
            .opaque(false)
            .on_enter(Rc::new(move |app, _| {
                self.set_state(app, |s| s.hovered = true)
            }))
            .on_exit(Rc::new(move |app, _| {
                self.set_state(app, |s| s.hovered = false)
            }))
            .child(ListenableBuilder::new(
                Rc::new(controller),
                move |app, context, _| self.template(app, context),
            ))
            .into_widget()
    }
}

/// Vertical increment/decrement template CommonStates, without taking keyboard traversal focus.
fn arrow_template(
    app: &mut App,
    context: BuildContext,
    states: ControlStates,
    content: WidgetRef,
    increase: bool,
    vertical: bool,
) -> WidgetRef {
    let r = ThemeResources::of(app, context);
    let (background, foreground) = match states.common {
        CommonState::Normal => (
            r.scroll_bar_button_background,
            r.scroll_bar_button_arrow_foreground,
        ),
        CommonState::PointerOver => (
            r.scroll_bar_button_background_pointer_over,
            r.scroll_bar_button_arrow_foreground_pointer_over,
        ),
        CommonState::Pressed => (
            r.scroll_bar_button_background_pressed,
            r.scroll_bar_button_arrow_foreground_pressed,
        ),
        CommonState::Disabled => (
            r.scroll_bar_button_background_disabled,
            r.scroll_bar_button_arrow_foreground_disabled,
        ),
    };
    ControlBorder::new(Brush::Solid(background), Brush::Solid(Color::new(0)))
        .border_thickness(0.0)
        .padding(if vertical {
            if increase {
                r.scroll_bar_vertical_increase_margin
            } else {
                r.scroll_bar_vertical_decrease_margin
            }
        } else {
            if increase {
                r.scroll_bar_horizontal_increase_margin
            } else {
                r.scroll_bar_horizontal_decrease_margin
            }
        })
        .child(Center::new().child(DefaultTextStyle::new(
            TextStyle::new().color(foreground),
            content,
        )))
        .into_widget()
}
