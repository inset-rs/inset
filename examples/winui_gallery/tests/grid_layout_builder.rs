//! Grid measures layout-time children before allocating its remaining star row.
#![feature(arbitrary_self_types)]

use inset_embedder::{Offset, Size};
use inset_widgets::*;
use inset_winui::{Grid, GridCell, GridLength, RowDefinition};
use inset_winui_test_support::Fixture;
use std::{cell::RefCell, rc::Rc};

/// WinUI MeasureCell calls child.Measure; a native LayoutBuilder needs actual layout.
#[test]
fn grid_measures_layout_builder_and_constrains_remaining_star_row() {
    let auto_key = Rc::new(GlobalKey::new());
    let star_key = Rc::new(GlobalKey::new());
    let seen = Rc::new(RefCell::new(Vec::new()));
    let auto = auto_key.clone();
    let star = star_key.clone();
    let constraints_seen = seen.clone();
    let fixture = Fixture::with_root([300, 240], move |app| {
        let grid = Grid::new()
            .row_definitions([
                RowDefinition::new(GridLength::AUTO),
                RowDefinition::new(GridLength::STAR),
                RowDefinition::new(GridLength::pixel(40.0)),
            ])
            .row_spacing(10.0)
            .children([
                GridCell::new(
                    LayoutBuilder::new(move |_, _, constraints| {
                        SizedBox::new()
                            .key(auto.clone())
                            .width(constraints.max_width)
                            .height(30.0)
                            .into_widget()
                    })
                    .into_widget(),
                )
                .into_widget(),
                GridCell::new(
                    LayoutBuilder::new(move |_, _, constraints| {
                        constraints_seen.borrow_mut().push(constraints);
                        SizedBox::expand().key(star.clone()).into_widget()
                    })
                    .into_widget(),
                )
                .row(1)
                .into_widget(),
                GridCell::new(SizedBox::shrink()).row(2).into_widget(),
            ]);
        run_app(app, grid.into_widget());
    });
    let mut app = fixture.cell.borrow_mut();
    let auto = auto_key.current_context(&mut app).unwrap();
    let star = star_key.current_context(&mut app).unwrap();
    let auto = auto.find_render_object(&app).unwrap().as_box().unwrap();
    let star = star.find_render_object(&app).unwrap().as_box().unwrap();
    assert_eq!(auto.size(&app), Size::new(300.0, 30.0));
    assert_eq!(star.size(&app), Size::new(300.0, 150.0));
    assert_eq!(
        star.local_to_global(&app, Offset::ZERO, None),
        Offset::new(0.0, 40.0)
    );
    assert!(!seen.borrow().is_empty());
    assert!(
        seen.borrow()
            .iter()
            .all(|constraints| constraints.max_height == 150.0)
    );
}
