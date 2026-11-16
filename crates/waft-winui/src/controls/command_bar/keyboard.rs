//! `CommandBar`'s keyboard and focus (`CommandBar_Partial.cpp`: `OnKeyDown`, `OnOverflowContentKeyDown`, `ProcessTabStopOverride`, `SetFocusedElementInOverflow`, `RestoreFocusToExpandButton`; `AppBar_Partial.cpp`: `SetFocusOnAppBar`, `RestoreSavedFocus`).
//!
//! Left and Right move along the bar's commands and its `MoreButton` without wrapping; Up and Down on the `MoreButton` open the overflow or move into it, and move through it, wrapping back to the button; Escape closes; Tab keeps the focus inside an open bar that is not sticky, passing from the bar's last element into the overflow, which it leaves at once (`TabNavigation="Once"`). The bar's elements are, in order, the content's focusable descendants, the commands still in the bar and the `MoreButton`; a later editor must keep that list the order the bar draws them in.

use super::{CommandBarState, OverflowInitialFocusItem};
use crate::FocusState;
use waft_embedder::TextDirection;
use waft_foundation::{App, Handle, OptionRetained};
use waft_services::{HardwareKeyboard, KeyEvent, LogicalKeyboardKey};
use waft_widgets::*;

impl CommandBarState {
    /// Focuses `node` for `state`, as XAML's `Focus(FocusState)`.
    pub(super) fn focus(app: &mut App, node: AnyFocusNode, state: FocusState) {
        FocusState::name(app, node, state);
        node.request_focus(app, None);
    }

    fn focused(app: &mut App) -> Option<AnyFocusNode> {
        primary_focus(app)
    }

    fn more_node(self: Handle<Self>, app: &App) -> AnyFocusNode {
        app.get(self)
            .more_focus
            .handle()
            .expect("made in `init_state`")
    }

    /// The commands still in the bar that can take the focus, and the `MoreButton` when shown.
    fn command_focusables(self: Handle<Self>, app: &mut App) -> Vec<AnyFocusNode> {
        let widget = self.widget(app).clone();
        let dynamic_primary = app.get(self).overflow.borrow().dynamic_primary.clone();
        let mut nodes: Vec<AnyFocusNode> = dynamic_primary
            .into_iter()
            .filter(|index| widget.primary_commands[*index].is_focusable())
            .filter_map(|index| {
                app.get(self)
                    .primary_focus
                    .get(index)
                    .map(|node| node.handle())
            })
            .filter(|node| node.context(app).is_some())
            .collect();
        if app.get(self).metrics.more_button_shown.get() {
            nodes.push(self.more_node(app));
        }
        nodes
    }

    /// The content's focusable descendants.
    fn content_focusables(self: Handle<Self>, app: &mut App) -> Vec<AnyFocusNode> {
        let group = app
            .get(self)
            .content_focus
            .handle()
            .expect("made in `init_state`");
        group
            .traversal_descendants(app)
            .into_iter()
            .filter(|node| node.can_request_focus(app))
            .collect()
    }

    /// `GetFirstFocusableElement` and `GetLastFocusableElement` of the bar run over these.
    fn bar_focusables(self: Handle<Self>, app: &mut App) -> Vec<AnyFocusNode> {
        let mut nodes = self.content_focusables(app);
        nodes.extend(self.command_focusables(app));
        nodes
    }

    /// The overflow's items that can take the focus, in menu order.
    pub(super) fn overflow_focusables(self: Handle<Self>, app: &mut App) -> Vec<AnyFocusNode> {
        let widget = self.widget(app).clone();
        let entries = app.get(self).overflow.borrow().dynamic_secondary.clone();
        entries
            .into_iter()
            .filter(|entry| {
                Self::entry_element(&widget, *entry).is_some_and(|element| element.is_focusable())
            })
            .filter_map(|entry| {
                app.get(self)
                    .overflow_focus
                    .get(&entry)
                    .map(|node| node.handle())
            })
            .filter(|node| node.context(app).is_some())
            .collect()
    }

    fn overflow_has_focus(self: Handle<Self>, app: &mut App) -> bool {
        let focused = Self::focused(app);
        focused.is_some_and(|focused| {
            app.get(self)
                .overflow_focus
                .values()
                .any(|node| node.handle() == focused)
        })
    }

    /// `ContainsElement`: the focus is in the bar or its overflow.
    fn has_focus(self: Handle<Self>, app: &mut App) -> bool {
        let Some(focused) = Self::focused(app) else {
            return false;
        };
        self.bar_focusables(app).contains(&focused)
            || app
                .get(self)
                .primary_focus
                .iter()
                .any(|node| node.handle() == focused)
            || self.overflow_has_focus(app)
    }

    /// `SetFocusOnAppBar`: unless the focus is already in the bar, it is saved and the bar's first element takes it.
    pub(super) fn set_focus_on_app_bar(self: Handle<Self>, app: &mut App) {
        if self.has_focus(app) {
            return;
        }
        let saved = Self::focused(app).map(|node| (node, FocusState::current(app)));
        let state = saved.map(|(_, state)| state).unwrap_or(FocusState::Pointer);
        app.get_mut(self).saved_focus = saved;
        if let Some(first) = self.bar_focusables(app).first().copied() {
            Self::focus(app, first, state);
        }
    }

    /// `RestoreSavedFocus`: the focus goes back where opening took it from, or, with nothing saved, from the overflow to the `MoreButton` (`RestoreSavedFocusImpl`).
    pub(super) fn restore_saved_focus(self: Handle<Self>, app: &mut App) {
        match app.get_mut(self).saved_focus.take() {
            Some((node, state)) if app.contains(node.id()) && node.context(app).is_some() => {
                Self::focus(app, node, state);
            }
            Some(_) => {}
            None => self.restore_focus_to_expand_button(app),
        }
    }

    /// `RestoreFocusToExpandButton`: focus in the overflow moves to the `MoreButton`, as it arrived.
    fn restore_focus_to_expand_button(self: Handle<Self>, app: &mut App) {
        if self.overflow_has_focus(app) {
            let state = FocusState::current(app);
            let more = self.more_node(app);
            Self::focus(app, more, state);
        }
    }

    /// `SetFocusedElementInOverflow`. Answers whether the focus moved.
    pub(super) fn set_focused_element_in_overflow(
        self: Handle<Self>,
        app: &mut App,
        first: bool,
    ) -> bool {
        let items = self.overflow_focusables(app);
        let target = if first { items.first() } else { items.last() };
        match target.copied() {
            Some(node) => {
                Self::focus(app, node, FocusState::Keyboard);
                true
            }
            None => false,
        }
    }

    /// `CommandBar::OnKeyDown` and `AppBar::OnKeyDown`, for a key the focused element left unhandled.
    pub(super) fn bar_key(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> KeyEventResult {
        if matches!(event, KeyEvent::Up(_)) {
            return KeyEventResult::Ignored;
        }
        let keyboard = HardwareKeyboard::instance(app);
        let alt = keyboard.is_alt_pressed(app);
        let handled = match event.logical_key() {
            key @ (LogicalKeyboardKey::ARROW_LEFT | LogicalKeyboardKey::ARROW_RIGHT) => {
                // `OnLeftRightKeyDown`: with Alt the key is the app's.
                if alt {
                    false
                } else {
                    let rtl = Directionality::maybe_of(app, self.context(app))
                        == Some(TextDirection::Rtl);
                    let right = (key == LogicalKeyboardKey::ARROW_RIGHT) != rtl;
                    self.shift_focus_horizontally(app, right);
                    true
                }
            }
            key @ (LogicalKeyboardKey::ARROW_UP | LogicalKeyboardKey::ARROW_DOWN) => {
                !alt && self.on_up_down_key_down(app, key == LogicalKeyboardKey::ARROW_UP)
            }
            LogicalKeyboardKey::ESCAPE => self.try_dismiss_inline_app_bar(app),
            LogicalKeyboardKey::TAB => {
                let shift = keyboard.is_shift_pressed(app);
                self.process_tab_stop_override(app, shift)
            }
            _ => false,
        };
        if handled {
            KeyEventResult::Handled
        } else {
            KeyEventResult::Ignored
        }
    }

    /// `ShiftFocusHorizontally`: not from the content, and not past either end.
    fn shift_focus_horizontally(self: Handle<Self>, app: &mut App, right: bool) {
        let Some(focused) = Self::focused(app) else {
            return;
        };
        if self.content_focusables(app).contains(&focused) {
            return;
        }
        let nodes = self.command_focusables(app);
        let Some(index) = nodes.iter().position(|node| *node == focused) else {
            return;
        };
        let next = if right {
            nodes.get(index + 1)
        } else {
            index.checked_sub(1).and_then(|index| nodes.get(index))
        };
        if let Some(next) = next.copied() {
            Self::focus(app, next, FocusState::Keyboard);
        }
    }

    /// `OnUpDownKeyDown` from the `MoreButton`: into the open overflow at its far end, or open it with that end to take the focus.
    fn on_up_down_key_down(self: Handle<Self>, app: &mut App, up: bool) -> bool {
        let more = self.more_node(app);
        if !more.has_primary_focus(app) {
            return false;
        }
        if self.widget(app).is_open {
            // Keyboard navigation may wrap, so either end is reachable.
            self.set_focused_element_in_overflow(app, !up)
        } else {
            app.get_mut(self).overflow_initial_focus_item = if up {
                OverflowInitialFocusItem::LastItem
            } else {
                OverflowInitialFocusItem::FirstItem
            };
            self.request_open(app, true);
            true
        }
    }

    /// `ProcessTabStopOverride` and `ProcessCandidateTabStopOverride` for a Tab within the open bar: from its last element into the overflow, and, unless it is sticky, round its ends.
    fn process_tab_stop_override(self: Handle<Self>, app: &mut App, backward: bool) -> bool {
        let widget = self.widget(app).clone();
        if !widget.is_open {
            return false;
        }
        let Some(focused) = Self::focused(app) else {
            return false;
        };
        let bar = self.bar_focusables(app);
        let (Some(first), Some(last)) = (bar.first().copied(), bar.last().copied()) else {
            return false;
        };
        let overflow_first = self.overflow_focusables(app).first().copied();
        let target = if !backward && focused == last {
            overflow_first.or((!widget.is_sticky).then_some(first))
        } else if backward && focused == first && !widget.is_sticky {
            // The wrap lands on the bar's last element, which the overflow takes over.
            overflow_first.or(Some(last))
        } else {
            None
        };
        match target {
            Some(target) => {
                Self::focus(app, target, FocusState::Keyboard);
                true
            }
            None => false,
        }
    }

    /// `OnOverflowContentKeyDown`.
    pub(super) fn overflow_key(
        self: Handle<Self>,
        app: &mut App,
        event: &KeyEvent,
    ) -> KeyEventResult {
        if matches!(event, KeyEvent::Up(_)) {
            return KeyEventResult::Ignored;
        }
        match event.logical_key() {
            LogicalKeyboardKey::ESCAPE | LogicalKeyboardKey::GAME_BUTTON_B => {
                // `TryDismissCommandBarOverflow`: a sticky bar stays open; the focus returns to the `MoreButton` either way.
                if !self.widget(app).is_sticky {
                    self.request_open(app, false);
                }
                self.restore_focus_to_expand_button(app);
            }
            // Handled so the focus stays in the menu.
            LogicalKeyboardKey::ARROW_LEFT | LogicalKeyboardKey::ARROW_RIGHT => {}
            key @ (LogicalKeyboardKey::ARROW_UP | LogicalKeyboardKey::ARROW_DOWN) => {
                self.shift_focus_vertically_in_overflow(app, key == LogicalKeyboardKey::ARROW_DOWN);
            }
            LogicalKeyboardKey::TAB => {
                let shift = HardwareKeyboard::instance(app).is_shift_pressed(app);
                self.handle_tab_key_pressed_in_overflow(app, shift);
            }
            _ => return KeyEventResult::Ignored,
        }
        KeyEventResult::Handled
    }

    /// `ShiftFocusVerticallyInOverflow` with wrapping: past either end the focus goes to the `MoreButton`.
    fn shift_focus_vertically_in_overflow(self: Handle<Self>, app: &mut App, down: bool) {
        let Some(focused) = Self::focused(app) else {
            return;
        };
        let items = self.overflow_focusables(app);
        let Some(index) = items.iter().position(|node| *node == focused) else {
            return;
        };
        let next = if down {
            items.get(index + 1)
        } else {
            index.checked_sub(1).and_then(|index| items.get(index))
        };
        let target = next.copied().unwrap_or_else(|| self.more_node(app));
        Self::focus(app, target, FocusState::Keyboard);
    }

    /// `HandleTabKeyPressedInOverflow` with `TabNavigation="Once"`: the focus leaves the menu at once, backward to the bar's last element, forward to its first, or, for a sticky bar, on past the bar.
    fn handle_tab_key_pressed_in_overflow(self: Handle<Self>, app: &mut App, backward: bool) {
        let bar = self.bar_focusables(app);
        let sticky = self.widget(app).is_sticky;
        let candidate = if backward || sticky {
            bar.last()
        } else {
            bar.first()
        };
        if let Some(candidate) = candidate.copied() {
            Self::focus(app, candidate, FocusState::Keyboard);
            if sticky && !backward {
                candidate.next_focus(app);
            }
        }
    }
}
