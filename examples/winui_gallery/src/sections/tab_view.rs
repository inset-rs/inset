//! TabView selection, close, width modes, in-window transfer and owner-managed reorder.

use crate::{column, example, example_row, label, section};
use inset_foundation::{App, Handle, Listener};
use inset_widgets::*;
use inset_winui::*;
use std::rc::Rc;

/// Builds the interactive tab gallery beneath the current theme.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section(
        "TabView",
        resources,
        example(
            "Editable tabs and cross-strip transfer",
            "Compare tab widths, change close-button visibility, add pages, and move tabs between the two strips.",
            resources,
            TabViewDemo.into_widget(),
        ),
    )
}

/// Owns two strips so cross-strip drop callbacks can transfer an item.
#[derive(Debug)]
struct TabViewDemo;

/// Application state stays independent of the library's visual containers.
struct TabViewDemoState {
    /// Native state lifecycle.
    state: StateData<TabViewDemo>,

    /// Stable ids in each strip's collection order.
    items: [Vec<u32>; 2],

    /// Owner-managed selection for both strips.
    selected: [Option<usize>; 2],

    /// Monotonic identity for the next added tab.
    next_id: u32,

    /// Width behavior applied to both strips.
    mode: TabViewWidthMode,

    /// Whether the drag can leave its original strip.
    drag: bool,

    /// Whether pointer and keyboard reorder requests are accepted.
    reorder: bool,

    /// Whether the add button is shown on each strip.
    add: bool,

    /// Whether close buttons are revealed on hover.
    hover_close: bool,

    /// Most recent owner event, displayed beneath the example.
    event: String,
}

impl StatefulWidget for TabViewDemo {
    type State = TabViewDemoState;

    fn create_state(&self) -> Self::State {
        TabViewDemoState {
            state: StateData::new(),
            items: [vec![0, 1, 2, 3], vec![4]],
            selected: [Some(0), Some(0)],
            next_id: 5,
            mode: TabViewWidthMode::Equal,
            drag: true,
            reorder: true,
            event: "Ready".into(),
            add: true,
            hover_close: false,
        }
    }
}

impl State for TabViewDemoState {
    type Widget = TabViewDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let text = resources.text_fill_color_primary;
        let state = app.get(self);
        let (items, selected, mode, drag, reorder, event) = (
            state.items.clone(),
            state.selected,
            state.mode,
            state.drag,
            state.reorder,
            state.event.clone(),
        );
        let (add, hover_close) = (state.add, state.hover_close);
        let controls = example_row(
            vec![
                Button::text(
                    format!("Width: {mode:?}"),
                    Listener::new(move |app| {
                        self.set_state(app, |state| {
                            state.mode = match state.mode {
                                TabViewWidthMode::Equal => TabViewWidthMode::SizeToContent,
                                TabViewWidthMode::SizeToContent => TabViewWidthMode::Compact,
                                TabViewWidthMode::Compact => TabViewWidthMode::Equal,
                            };
                        })
                    }),
                )
                .into_widget(),
                CheckBox::new(Some(reorder), move |app, value| {
                    self.set_state(app, |state| state.reorder = value.unwrap_or(false))
                })
                .content(Text::new("Reorder tabs"))
                .into_widget(),
                CheckBox::new(Some(drag), move |app, value| {
                    self.set_state(app, |state| state.drag = value.unwrap_or(false))
                })
                .content(Text::new("Move between strips"))
                .into_widget(),
                CheckBox::new(Some(add), move |app, value| {
                    self.set_state(app, |state| state.add = value.unwrap_or(false))
                })
                .content(Text::new("Show add buttons"))
                .into_widget(),
                CheckBox::new(Some(hover_close), move |app, value| {
                    self.set_state(app, |state| state.hover_close = value.unwrap_or(false))
                })
                .content(Text::new("Close buttons on hover"))
                .into_widget(),
            ],
            12.0,
        );
        let mut children = vec![
            controls,
            label(
                "Use Ctrl+Tab to switch, Ctrl+F4 to close, and Alt+Shift+arrows to reorder. Tab 2 is disabled; Tab 3 cannot close.",
                TextBlockStyle::Caption,
                text,
            ),
        ];
        for strip in 0..2 {
            let tabs = items[strip]
                .iter()
                .map(|id| {
                    TabViewItem::text(
                        id.to_string(),
                        format!("Tab {id}"),
                        Center::new().child(label(
                            format!("Page {id}"),
                            TextBlockStyle::Body,
                            text,
                        )),
                    )
                    .icon_source(FontIcon::symbol(FluentSymbol::Settings))
                    .is_enabled(*id != 2)
                    .is_closable(*id != 3)
                })
                .collect();
            let tabs = TabView::new(tabs, selected[strip], move |app, selected| {
                self.set_state(app, |state| state.selected[strip] = selected)
            })
            .tab_width_mode(mode)
            .is_add_tab_button_visible(add)
            .close_button_overlay_mode(if hover_close {
                TabViewCloseButtonOverlayMode::OnPointerOver
            } else {
                TabViewCloseButtonOverlayMode::Auto
            })
            .can_drag_tabs(drag)
            .can_reorder_tabs(reorder)
            .tab_reorder_requested(move |app, args| {
                self.set_state(app, |state| {
                    let item = state.items[strip].remove(args.old_index);
                    state.items[strip].insert(args.new_index, item);
                    state.selected[strip] = args.selected_index;
                    state.event = format!("Reordered Tab {item} to {}", args.new_index + 1);
                })
            })
            .tab_close_requested(Rc::new(move |app, args| {
                self.set_state(app, |state| {
                    state.items[strip].remove(args.index);
                    state.event = format!("Closed Tab {}", args.item.id);
                })
            }))
            .add_tab_button_click(Listener::new(move |app| {
                self.set_state(app, |state| {
                    let id = state.next_id;
                    state.next_id += 1;
                    state.items[strip].push(id);
                    state.selected[strip] = Some(state.items[strip].len() - 1);
                    state.event = format!("Added Tab {id}");
                })
            }))
            .tab_strip_drag_over(|_, args| args.accepted = true)
            .tab_strip_drop(move |app, args| {
                self.set_state(app, |state| {
                    let id = args.item.id.parse::<u32>().unwrap();
                    let source = 1 - strip;
                    state.items[source].retain(|item| *item != id);
                    state.selected[source] = state.selected[source].and_then(|index| {
                        (!state.items[source].is_empty())
                            .then(|| index.min(state.items[source].len() - 1))
                    });
                    let insertion = args.insertion_index.min(state.items[strip].len());
                    state.items[strip].insert(insertion, id);
                    state.selected[strip] = Some(insertion);
                    state.event = format!("Moved Tab {id} to strip {}", strip + 1);
                })
            });
            children.push(SizedBox::new().height(128.0).child(tabs).into_widget());
        }
        children.push(label(event, TextBlockStyle::Caption, text));
        column(children, 12.0)
    }
}
