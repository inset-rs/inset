//! WinUI's absent-brush hit area and rounded child clipping.
#![feature(arbitrary_self_types)]

use inset_embedder::{Color, Offset, PointerChange};
use inset_foundation::Listener;
use inset_painting::Alignment;
use inset_rendering::HitTestBehavior;
use inset_widgets::*;
use inset_winui::*;
use inset_winui_test_support::Fixture;
use std::{cell::Cell, rc::Rc};

fn fixture(background: Option<Brush>, rounded_child: bool) -> (Fixture, Rc<Cell<usize>>) {
    let clicks = Rc::new(Cell::new(0));
    let shared = clicks.clone();
    let f = Fixture::with_root([120, 120], move |app| {
        let bottom_clicks = shared.clone();
        let top = if rounded_child {
            ControlBorder::new_optional(None, None)
                .border_thickness(0.0)
                .corner_radius(30.0)
                .child(ColoredBox::new(Color::from_argb(255, 0, 255, 0)))
                .into_widget()
        } else {
            let mut top = Grid::new()
                .border_brush(Brush::Solid(Color::from_argb(255, 255, 0, 0)))
                .border_thickness(8.0);
            top.background = background;
            top.into_widget()
        };
        run_app(
            app,
            WidgetsApp::new(Color::from_argb(255, 0, 0, 0))
                .debug_show_checked_mode_banner(false)
                .builder(move |_, _, _| {
                    Align::new()
                        .alignment(Alignment::TOP_LEFT.into())
                        .child(
                            SizedBox::new().width(100.0).height(100.0).child(
                                Stack::new().children([
                                    Positioned::fill(
                                        GestureDetector::new()
                                            .behavior(HitTestBehavior::Opaque)
                                            .on_tap(Listener::new({
                                                let clicks = bottom_clicks.clone();
                                                move |_| clicks.set(clicks.get() + 1)
                                            }))
                                            .child(ColoredBox::new(Color::from_argb(
                                                255, 0, 0, 255,
                                            ))),
                                    )
                                    .into_widget(),
                                    Positioned::fill(top.clone()).into_widget(),
                                ]),
                            ),
                        )
                        .into_widget()
                })
                .into_widget(),
        );
    });
    (f, clicks)
}

fn tap(f: &mut Fixture, point: Offset) {
    f.send(PointerChange::Down, point);
    f.send(PointerChange::Up, point);
    f.pump();
}

#[test]
fn absent_background_passes_interior_hits_but_keeps_border_hits() {
    let (mut f, clicks) = fixture(None, false);
    tap(&mut f, Offset::new(50.0, 50.0));
    assert_eq!(clicks.get(), 1);
    tap(&mut f, Offset::new(4.0, 50.0));
    assert_eq!(clicks.get(), 1);
}

#[test]
fn transparent_background_still_hits_its_interior() {
    let (mut f, clicks) = fixture(Some(Brush::Solid(Color::new(0))), false);
    tap(&mut f, Offset::new(50.0, 50.0));
    assert_eq!(clicks.get(), 0);
}

#[test]
fn rounded_corners_clip_child_paint_and_hit_testing() {
    let (mut f, clicks) = fixture(None, true);
    tap(&mut f, Offset::new(1.0, 1.0));
    assert_eq!(clicks.get(), 1);
    let pixels = f.view.pixels.borrow();
    assert_eq!(&pixels[(120 + 1) * 4..(120 + 1) * 4 + 4], &[0, 0, 255, 255]);
    assert_eq!(
        &pixels[(50 * 120 + 50) * 4..(50 * 120 + 50) * 4 + 4],
        &[0, 255, 0, 255]
    );
}
