//! Per-side chrome and post-frame size reporting used by SplitView.
#![feature(arbitrary_self_types)]

use inset_embedder::{Color, Size};
use inset_foundation::App;
use inset_widgets::*;
use inset_winui::{Brush, ControlBorder, SizeObserver};
use inset_winui_test_support::{Fixture, mount};
use std::{cell::RefCell, rc::Rc};

type Reports = Rc<RefCell<Vec<(Size, Size)>>>;

fn border_tree(width: f64, reports: Reports) -> WidgetRef {
    Center::new()
        .child(
            SizedBox::new().width(width).height(70.0).child(
                ControlBorder::new(
                    Brush::Solid(Color::from_argb(255, 0, 0, 255)),
                    Brush::Solid(Color::from_argb(255, 255, 0, 0)),
                )
                .border_thickness_ltrb([3.0, 5.0, 7.0, 9.0])
                .padding([11.0, 13.0, 17.0, 19.0])
                .child(SizeObserver::new(
                    ColoredBox::new(Color::from_argb(255, 0, 255, 0)).child(SizedBox::expand()),
                    Rc::new(move |_: &mut App, old, new| reports.borrow_mut().push((old, new))),
                )),
            ),
        )
        .into_widget()
}

#[test]
fn per_side_border_paints_and_insets_each_side_independently() {
    let reports = Reports::default();
    let observed = reports.clone();
    let mut fixture = Fixture::with_root([160, 120], mount(move |_| border_tree(100.0, observed)));
    assert_eq!(&*reports.borrow(), &[(Size::ZERO, Size::new(62.0, 24.0))]);
    {
        let pixels = fixture.view.pixels.borrow();
        let pixel = |x: usize, y: usize| &pixels[(y * 160 + x) * 4..][..4];
        for (x, y) in [(31, 50), (60, 27), (125, 50), (60, 90)] {
            assert_eq!(pixel(x, y), &[255, 0, 0, 255], "border at {x},{y}");
        }
        assert_eq!(pixel(38, 45), &[0, 0, 255, 255]);
        assert_eq!(pixel(45, 45), &[0, 255, 0, 255]);
        assert_eq!(pixel(29, 50), &[255, 255, 255, 255]);
    }
    fixture.capture("per-side-border");
    fixture.pump();
    assert_eq!(
        reports.borrow().len(),
        1,
        "unchanged layout is not a size change"
    );
    run_app(
        &mut fixture.cell.borrow_mut(),
        border_tree(120.0, reports.clone()),
    );
    fixture.cell.elapse(std::time::Duration::ZERO);
    fixture.pump();
    assert_eq!(
        &*reports.borrow(),
        &[
            (Size::ZERO, Size::new(62.0, 24.0)),
            (Size::new(62.0, 24.0), Size::new(82.0, 24.0)),
        ]
    );
    run_app(
        &mut fixture.cell.borrow_mut(),
        SizedBox::shrink().into_widget(),
    );
    fixture.cell.elapse(std::time::Duration::ZERO);
    fixture.pump();
    assert_eq!(
        reports.borrow().len(),
        2,
        "unmount does not report a spurious size"
    );
}

#[test]
fn filtered_corner_radii_leave_the_other_corners_square() {
    let fixture = Fixture::with_root(
        [120, 120],
        mount(|_| {
            Center::new()
                .child(
                    SizedBox::new().width(80.0).height(80.0).child(
                        ControlBorder::new(
                            Brush::Solid(Color::from_argb(255, 0, 0, 255)),
                            Brush::Solid(Color::from_argb(0, 0, 0, 0)),
                        )
                        .border_thickness(0.0)
                        .corner_radius_corners([16.0, 0.0, 8.0, 0.0]),
                    ),
                )
                .into_widget()
        }),
    );
    let pixels = fixture.view.pixels.borrow();
    let pixel = |x: usize, y: usize| &pixels[(y * 120 + x) * 4..][..4];
    assert_eq!(pixel(20, 20), &[255, 255, 255, 255]);
    assert_eq!(pixel(99, 20), &[0, 0, 255, 255]);
    assert_eq!(pixel(99, 99), &[255, 255, 255, 255]);
    assert_eq!(pixel(20, 99), &[0, 0, 255, 255]);
    fixture.capture("per-corner-border");
}
