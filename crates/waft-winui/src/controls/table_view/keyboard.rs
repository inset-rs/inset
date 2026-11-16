//! `TableView`'s keyboard navigation (`controls/dev/TableView/TableView_Keyboard.cpp`): Up, Down, Home, End, Page Up and Page Down move the focus through the rows and the selection follows it, Space selects the focused row, and Escape ends a column resize.
//!
//! A row that is not built is brought into view by an offset estimated from the nearest built row and focused once built; the table's registry of built rows by index is what this reads, and a later editor must keep it exact.

use super::*;

// MARK: Keyboard navigation (`TableView_Keyboard.cpp`)

impl<T: 'static> TableViewState<T> {
    pub(super) fn register_row(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
        node: Handle<FocusNode>,
    ) {
        if app.contains(self) {
            app.get_mut(self).rows.insert(index, node);
        }
    }

    pub(super) fn unregister_row(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
        node: Handle<FocusNode>,
    ) {
        if app.contains(self) && app.get(self).rows.get(&index) == Some(&node) {
            app.get_mut(self).rows.remove(&index);
        }
    }

    /// `GetFocusedRowIndex`.
    pub(super) fn focused_row_index(self: Handle<Self>, app: &App) -> Option<usize> {
        app.get(self)
            .rows
            .iter()
            .find(|(_, node)| app.contains(**node) && node.has_primary_focus(app))
            .map(|(index, _)| *index)
    }

    /// `GetEstimatedRowsPerPage`: the viewport's height in rows of the first built row's height, 10 before any is.
    pub(super) fn estimated_rows_per_page(self: Handle<Self>, app: &mut App) -> usize {
        let viewport = self.vertical(app).metrics(app).viewport_length;
        let first = app.get(self).rows.keys().min().copied();
        let row_height = first
            .and_then(|index| self.row_rect(app, index))
            .map(|rect| rect.height())
            .filter(|height| *height > 0.0);
        match row_height {
            Some(height) if viewport > 0.0 => ((viewport / height) as usize).max(1),
            _ => 10,
        }
    }

    /// `OnKeyDownForNavigation`: Space selects the focused row; Up, Down, Home, End, Page Up and Page Down move the focus and the selection follows it, unless Control is down; Escape ends a column resize.
    pub(super) fn key(self: Handle<Self>, app: &mut App, event: &KeyEvent) -> KeyEventResult {
        if matches!(event, KeyEvent::Up(_)) {
            return KeyEventResult::Ignored;
        }
        let key = event.logical_key();
        if key == LogicalKeyboardKey::ESCAPE {
            return if self.cancel_column_resize_drag(app) {
                KeyEventResult::Handled
            } else {
                KeyEventResult::Ignored
            };
        }
        let count = self.row_count(app);
        if count == 0 {
            return KeyEventResult::Ignored;
        }
        let focused = self.focused_row_index(app);
        if key == LogicalKeyboardKey::SPACE {
            if self.can_select_rows(app)
                && let Some(index) = focused
            {
                self.select_row_from_interaction(app, index);
                return KeyEventResult::Handled;
            }
            return KeyEventResult::Ignored;
        }
        let up_or_down =
            key == LogicalKeyboardKey::ARROW_UP || key == LogicalKeyboardKey::ARROW_DOWN;
        let absolute = matches!(
            key,
            LogicalKeyboardKey::HOME
                | LogicalKeyboardKey::END
                | LogicalKeyboardKey::PAGE_UP
                | LogicalKeyboardKey::PAGE_DOWN
        );
        if !up_or_down && !absolute {
            return KeyEventResult::Ignored;
        }
        let control = HardwareKeyboard::instance(app).is_control_pressed(app);
        let mut current = focused;
        if current.is_none() && up_or_down {
            // Focus was not on a row: relative keys resume from the selected row.
            current = self
                .selection_handle(app)
                .and_then(|selection| selection.selected_index(app))
                .filter(|index| *index < count);
        }
        let Some(current) = current else {
            let initial = match key {
                LogicalKeyboardKey::END => count - 1,
                LogicalKeyboardKey::PAGE_DOWN => self
                    .estimated_rows_per_page(app)
                    .saturating_sub(1)
                    .min(count - 1),
                _ => 0,
            };
            self.focus_row(app, initial, !control);
            return KeyEventResult::Handled;
        };
        let page = if matches!(
            key,
            LogicalKeyboardKey::PAGE_UP | LogicalKeyboardKey::PAGE_DOWN
        ) {
            self.estimated_rows_per_page(app)
        } else {
            0
        };
        let target = match key {
            LogicalKeyboardKey::ARROW_UP => current.saturating_sub(1),
            LogicalKeyboardKey::ARROW_DOWN => (current + 1).min(count - 1),
            LogicalKeyboardKey::HOME => 0,
            LogicalKeyboardKey::END => count - 1,
            LogicalKeyboardKey::PAGE_UP => current.saturating_sub(page),
            _ => (current + page).min(count - 1),
        };
        if target != current {
            self.focus_row(app, target, !control);
            KeyEventResult::Handled
        } else if absolute {
            // A boundary no-op is consumed so the body does not scroll.
            KeyEventResult::Handled
        } else {
            KeyEventResult::Ignored
        }
    }

    /// `FocusRow`: the row is brought into view and takes the focus once built, a few frames at most, unless a later request came; `select` then selects it.
    pub(super) fn focus_row(self: Handle<Self>, app: &mut App, index: usize, select: bool) {
        let epoch = {
            let state = app.get_mut(self);
            state.focus_epoch += 1;
            state.focus_epoch
        };
        self.focus_when_built(app, index, select, epoch, 0);
    }

    pub(super) fn focus_when_built(
        self: Handle<Self>,
        app: &mut App,
        index: usize,
        select: bool,
        epoch: u64,
        attempt: u32,
    ) {
        if app.get(self).focus_epoch != epoch {
            return;
        }
        if let Some(node) = app.get(self).rows.get(&index).copied() {
            FocusState::name(app, node.as_node(), FocusState::Keyboard);
            node.request_focus(app, None);
            self.bring_row_into_view(app, index);
            if select {
                self.select_row_from_interaction(app, index);
            }
            return;
        }
        if attempt < 3 {
            self.bring_row_into_view(app, index);
            self.after_frame(app, move |app| {
                self.focus_when_built(app, index, select, epoch, attempt + 1)
            });
        }
    }

    /// `StartBringIntoView` on the body: as little scrolling as shows the row; a row not built is placed by the height of the nearest built one.
    pub(super) fn bring_row_into_view(self: Handle<Self>, app: &mut App, index: usize) {
        let Some(viewport) = self.body_rect(app) else {
            return;
        };
        let (top, height) = match self.row_rect(app, index) {
            Some(rect) => (rect.top - viewport.top, rect.height()),
            None => {
                let Some(anchor) = app
                    .get(self)
                    .rows
                    .keys()
                    .copied()
                    .min_by_key(|built| built.abs_diff(index))
                else {
                    return;
                };
                let Some(rect) = self.row_rect(app, anchor) else {
                    return;
                };
                let offset = (index as f64 - anchor as f64) * rect.height();
                (rect.top - viewport.top + offset, rect.height())
            }
        };
        let view = viewport.height();
        let delta = if top < 0.0 {
            top
        } else if top + height > view {
            (top + height - view).min(top)
        } else {
            return;
        };
        self.vertical(app).scroll_by(app, delta, true);
    }

    pub(super) fn row_rect(self: Handle<Self>, app: &mut App, index: usize) -> Option<Rect> {
        let node = app.get(self).rows.get(&index).copied()?;
        let context = node.as_node().context(app)?;
        global_rect(app, context)
    }

    pub(super) fn body_rect(self: Handle<Self>, app: &mut App) -> Option<Rect> {
        let context = app.get(self).body_key.current_context(app)?;
        global_rect(app, context)
    }
}

fn global_rect(app: &mut App, context: BuildContext) -> Option<Rect> {
    let render = context.find_render_object(app)?.as_box()?;
    let origin = render.local_to_global(app, Offset::ZERO, None);
    Some(origin & render.size(app))
}
