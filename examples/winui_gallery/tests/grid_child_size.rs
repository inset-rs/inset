//! Grid measures its children again when one of them changes its desired size, even a child it
//! last arranged to a cell of fixed size.
#![feature(arbitrary_self_types)]

use inset_embedder::Size;
use inset_foundation::{App, ValueNotifier};
use inset_painting::AlignmentGeometry;
use inset_widgets::*;
use inset_winui::{ColumnDefinition, Grid, GridCell, GridLength, RowDefinition};
use inset_winui_test_support::Fixture;
use std::rc::Rc;

/// The size of the render object of the widget with `key`.
fn size_of(fixture: &Fixture, key: &Rc<GlobalKey>) -> Size {
    let mut app = fixture.cell.borrow_mut();
    let context = key.current_context(&mut app).unwrap();
    let object = context.find_render_object(&app).unwrap().as_box().unwrap();
    object.size(&app)
}

/// An Auto row follows a child that a wider sibling stretches across its Auto column, when the
/// child grows and when it shrinks.
#[test]
fn an_auto_row_follows_a_stretched_child_that_changes_its_height() {
    let grid_key = Rc::new(GlobalKey::new());
    let key = grid_key.clone();
    let mut height = None;
    let slot = &mut height;
    let mut fixture = Fixture::with_root([300, 300], |app: &mut App| {
        let notifier = app.create(ValueNotifier::new(100.0));
        *slot = Some(notifier);
        let grid = Grid::new()
            .key(key)
            .column_definitions([ColumnDefinition::new(GridLength::AUTO)])
            .row_definitions([
                RowDefinition::new(GridLength::AUTO),
                RowDefinition::new(GridLength::AUTO),
            ])
            .children([
                GridCell::new(ValueListenableBuilder::new(
                    Rc::new(notifier),
                    |_, _, height: &f64, _| {
                        SizedBox::new().width(50.0).height(*height).into_widget()
                    },
                ))
                .into_widget(),
                // Wider than the first child, which its column stretches to 200.
                GridCell::new(SizedBox::new().width(200.0).height(10.0))
                    .row(1)
                    .into_widget(),
            ]);
        run_app(
            app,
            Align::new()
                .alignment(AlignmentGeometry::TOP_LEFT)
                .child(grid)
                .into_widget(),
        );
    });
    let height = height.unwrap();
    assert_eq!(size_of(&fixture, &grid_key), Size::new(200.0, 110.0));
    height.set_value(&mut fixture.cell.borrow_mut(), 50.0);
    fixture.cell.checkpoint();
    fixture.pump();
    assert_eq!(size_of(&fixture, &grid_key), Size::new(200.0, 60.0));
    height.set_value(&mut fixture.cell.borrow_mut(), 150.0);
    fixture.cell.checkpoint();
    fixture.pump();
    assert_eq!(size_of(&fixture, &grid_key), Size::new(200.0, 160.0));
}
