//! A `TableView` rebuilt over the application's own selection and source: the selected row
//! survives when the source is the same, and a replaced source clears it without dirtying a
//! listener above the table while the table builds.

use std::cell::Cell;
use std::rc::Rc;

use waft_embedder::Color;
use waft_foundation::Ref;
use waft_widgets::testing::{WidgetTester, test_widgets};
use waft_widgets::*;
use waft_winui::*;

/// The source's items.
fn people() -> Vec<&'static str> {
    vec!["Mina", "Asha", "Diego", "Lea"]
}

/// The application's table, kept across mounts.
struct Table {
    source: Ref<TableViewSource<&'static str>>,
    column: Ref<TableViewColumn<&'static str>>,
    selection: Ref<TableViewSelection>,
}

impl Table {
    fn new(tester: &WidgetTester) -> Table {
        tester.update(|app| {
            let source = TableViewSource::from(app, people());
            let column = TableViewColumn::text(app, "Name", |name: &&'static str| name.to_string());
            Table {
                source,
                column,
                selection: TableViewSelection::new(app),
            }
        })
    }

    /// The window, with a `ListenableBuilder` above the table counting its builds.
    fn root(&self, listener_builds: &Rc<Cell<usize>>) -> WidgetRef {
        let source = self.source.clone();
        let column = self.column.clone();
        let selection = self.selection.clone();
        let builds = Rc::clone(listener_builds);
        WidgetsApp::new(AccentPalette::default().base)
            .debug_show_checked_mode_banner(false)
            .builder(move |_, _, _| {
                let table = TableView::new(source.clone(), vec![column.clone()])
                    .selection(selection.clone());
                let builds = Rc::clone(&builds);
                ThemeScope::new(
                    Theme::Light,
                    ColoredBox::new(Color::from_argb(255, 255, 255, 255)).child(
                        ListenableBuilder::new(Rc::new(selection.clone()), move |_, _, child| {
                            builds.set(builds.get() + 1);
                            child.unwrap().clone()
                        })
                        .child(table)
                        .into_widget(),
                    ),
                )
                .into_widget()
            })
            .into_widget()
    }
}

#[test]
fn a_table_rebuilt_over_the_same_source_keeps_the_selected_row() {
    test_widgets(|tester| {
        let table = Table::new(tester);
        let listener_builds = Rc::new(Cell::new(0));

        tester.pump_widget(table.root(&listener_builds));
        tester.update(|app| table.selection.select(app, 1));
        assert_eq!(
            table.selection.selected_index(&tester.cell.borrow()),
            Some(1)
        );

        // The tab goes away and comes back: a new table element over the same selection and
        // source. The selection must not clear, and its listener must not be dirtied during the
        // new table's build.
        tester.pump_widget(SizedBox::shrink());
        tester.pump_widget(table.root(&listener_builds));
        tester.pump_and_settle();

        assert_eq!(
            table.selection.selected_index(&tester.cell.borrow()),
            Some(1)
        );
        let builds = listener_builds.get();
        // Once when first mounted, once for the new tree: no extra rebuild from the selection.
        assert!(
            builds <= 3,
            "the selection rebuilt its listener above the table {builds} times"
        );
    });
}

#[test]
fn a_table_rebuilt_over_a_new_source_clears_the_selection_after_the_frame() {
    test_widgets(|tester| {
        let table = Table::new(tester);
        let listener_builds = Rc::new(Cell::new(0));
        tester.pump_widget(table.root(&listener_builds));
        tester.update(|app| table.selection.select(app, 1));

        // A different source replaces the table's.
        let other_source = tester.update(|app| TableViewSource::from(app, people()));
        let other = Table {
            source: other_source,
            column: table.column.clone(),
            selection: table.selection.clone(),
        };
        tester.pump_widget(SizedBox::shrink());
        tester.pump_widget(other.root(&listener_builds));

        // The clear happened while the new table built; the listener hears of it only after the
        // frame, so it is gone from the row by then and the build above never panicked.
        assert_eq!(table.selection.selected_index(&tester.cell.borrow()), None);
        let before = listener_builds.get();
        tester.pump();
        assert!(
            listener_builds.get() > before,
            "the selection notified no listener above the table"
        );
    });
}
