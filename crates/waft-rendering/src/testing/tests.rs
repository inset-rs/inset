//! The matcher's own rules, on ops a canvas records.

use waft_embedder::valo::Op;
use waft_embedder::valo::{PaintStyle, Stroke};
use waft_embedder::{Canvas, Color, Offset, Paint, RRect, Radius, Rect, StrokeCap};

use super::{
    CanvasMethod, PaintCall, PaintedStyle, RecordedCall, assert_paints, count_calls, draws, paints,
    paints_exactly,
};

const RED: Color = Color::new(0xFFFF0000);
const BLUE: Color = Color::new(0xFF0000FF);

fn paint(color: Color) -> Paint {
    Paint {
        color: color.into(),
        ..Paint::default()
    }
}

/// A save, a translation, a red circle, a restore, a blue rect and a blue rounded rect.
fn recorded() -> Vec<Op> {
    let mut canvas = Canvas::new();
    canvas.save();
    canvas.translate(10.0, 20.0);
    canvas.draw_circle(Offset::new(5.0, 5.0), 4.0, &paint(RED));
    canvas.restore();
    canvas.draw_rect(Rect::from_ltwh(0.0, 0.0, 30.0, 10.0), &paint(BLUE));
    waft_painting::draw_rrect(&mut canvas, rounded(), &paint(BLUE));
    canvas.build().ops().to_vec()
}

fn rounded() -> RRect {
    RRect::from_rect_and_radius(
        Rect::from_ltwh(0.0, 40.0, 30.0, 20.0),
        Radius::circular(4.0),
    )
}

#[test]
fn a_pattern_finds_each_call_after_the_last_and_skips_the_others() {
    let ops = recorded();
    assert_paints(
        &ops,
        &[
            PaintCall::translate(Some(10.0), Some(20.0)),
            PaintCall::Circle {
                x: Some(5.0),
                y: Some(5.0),
                radius: Some(4.0),
                color: Some(RED),
                style: Some(PaintedStyle::Fill),
            },
            PaintCall::rect(Some(BLUE)),
            PaintCall::rrect_at(rounded(), Some(BLUE)),
        ],
    );
    assert_paints(&ops, &[PaintCall::SaveRestore, PaintCall::rect(None)]);
    assert!(paints(&ops, &[PaintCall::rect(None), PaintCall::circle(None)]).is_err());
}

#[test]
fn a_pattern_without_steps_or_a_recording_without_calls_fails() {
    assert!(paints(&recorded(), &[]).is_err());
    assert!(paints(&[], &[]).is_err());
    assert!(paints(&[], &[PaintCall::rect(None)]).is_err());
}

#[test]
fn a_rounded_rect_matches_within_darts_tolerance_and_as_the_oval_its_corners_make() {
    let mut canvas = Canvas::new();
    let pill =
        RRect::from_rect_and_radius(Rect::from_ltwh(0.0, 0.0, 6.0, 6.0), Radius::circular(3.0));
    waft_painting::draw_rrect(&mut canvas, pill, &paint(RED));
    waft_painting::draw_rrect(&mut canvas, rounded(), &paint(BLUE));
    let ops = canvas.build().ops().to_vec();
    let near_pill = RRect::from_rect_and_radius(
        Rect::from_ltwh(0.00005, 0.0, 6.0, 6.0),
        Radius::circular(3.0),
    );
    assert_paints(
        &ops,
        &[
            PaintCall::rrect_at(near_pill, Some(RED)),
            PaintCall::rrect_at(rounded().shift(Offset::new(0.00005, 0.0)), Some(BLUE)),
        ],
    );
    let off = rounded().shift(Offset::new(0.001, 0.0));
    assert!(paints(&ops, &[PaintCall::rrect_at(off, None)]).is_err());
}

#[test]
fn a_mismatch_names_the_step_the_call_and_the_log() {
    let ops = recorded();
    let mismatch = paints(&ops, &[PaintCall::circle(None), PaintCall::rect(Some(RED))])
        .expect_err("the rect is blue");
    assert_eq!(mismatch.step, 1);
    assert_eq!(mismatch.found.as_ref().map(|(index, _)| *index), Some(4));
    assert!(matches!(
        mismatch.found,
        Some((_, RecordedCall::Rect { .. }))
    ));
    assert_eq!(mismatch.log.len(), ops.len());
    let message = mismatch.to_string();
    assert!(message.contains("step 1 (rect("), "{message}");
    assert!(message.contains("#4 Rect"), "{message}");
}

#[test]
fn something_walks_on_until_its_predicate_answers_and_fails_with_its_message() {
    let ops = recorded();
    assert_paints(
        &ops,
        &[PaintCall::something(|call| {
            Ok(matches!(call, RecordedCall::Rect { .. }))
        })],
    );
    let mismatch = paints(
        &ops,
        &[PaintCall::something(|call| match call {
            RecordedCall::Circle { center, .. } if center.dx() > 100.0 => Ok(true),
            RecordedCall::Circle { .. } => Err("the circle is on the left".to_string()),
            _ => Ok(false),
        })],
    )
    .expect_err("the circle is at x 5");
    assert_eq!(mismatch.reason, "the circle is on the left");
}

#[test]
fn paints_exactly_compares_call_for_call_and_draws_leaves_out_the_state() {
    let draws = draws(&recorded());
    assert!(
        paints_exactly(
            &draws,
            &[
                PaintCall::circle(Some(RED)),
                PaintCall::rect(Some(BLUE)),
                PaintCall::rrect(Some(BLUE)),
            ],
        )
        .is_ok()
    );
    let short = paints_exactly(&draws, &[PaintCall::circle(None)]).expect_err("two more");
    assert_eq!(short.reason, "it painted more calls");
    assert!(paints_exactly(&draws, &[PaintCall::rect(None)]).is_err());
}

#[test]
fn calls_are_counted_by_method() {
    let ops = recorded();
    assert_eq!(count_calls(&ops, CanvasMethod::DrawCircle), 1);
    assert_eq!(count_calls(&ops, CanvasMethod::DrawRRect), 1);
    assert_eq!(count_calls(&ops, CanvasMethod::Save), 1);
}

#[test]
fn a_line_an_arc_and_a_path_match_by_where_they_run() {
    let mut canvas = Canvas::new();
    let stroked = |width: f32, cap: StrokeCap| Paint {
        color: RED.into(),
        style: PaintStyle::Stroke(Stroke {
            cap,
            ..Stroke::new(width)
        }),
        ..Paint::default()
    };
    waft_painting::draw_line(
        &mut canvas,
        Offset::new(0.0, 10.0),
        Offset::new(50.0, 10.0),
        &stroked(2.0, StrokeCap::Butt),
    );
    let oval = Rect::from_ltwh(0.0, 0.0, 40.0, 40.0);
    waft_painting::draw_arc(
        &mut canvas,
        oval,
        0.0,
        std::f64::consts::FRAC_PI_2,
        false,
        &stroked(4.0, StrokeCap::Round),
    );
    waft_painting::draw_drrect(&mut canvas, rounded(), rounded().deflate(2.0), &paint(BLUE));
    let ops = canvas.build().ops().to_vec();
    assert_eq!(count_calls(&ops, CanvasMethod::DrawLine), 1);
    assert_paints(
        &ops,
        &[
            PaintCall::line(Offset::new(0.0, 10.0), Offset::new(50.0, 10.0), Some(2.0)),
            PaintCall::Arc {
                rect: Some(oval),
                color: Some(RED),
                stroke_width: Some(4.0),
                stroke_cap: Some(StrokeCap::Round),
            },
            PaintCall::drrect_between(rounded(), rounded().deflate(2.0), Some(BLUE)),
        ],
    );
    assert!(
        paints(
            &ops,
            &[PaintCall::arc(Some(Rect::from_ltwh(0.0, 0.0, 80.0, 80.0)))]
        )
        .is_err()
    );
}

#[test]
fn a_path_step_reads_the_points_its_fill_covers() {
    let mut canvas = Canvas::new();
    let mut triangle = waft_embedder::PathBuilder::new();
    triangle.move_to(Offset::new(0.0, 0.0));
    triangle.line_to(Offset::new(20.0, 0.0));
    triangle.line_to(Offset::new(0.0, 20.0));
    triangle.close();
    canvas.draw_path(
        &triangle.build(),
        waft_embedder::valo::FillRule::NonZero,
        &paint(RED),
    );
    let ops = canvas.build().ops().to_vec();
    assert_paints(
        &ops,
        &[PaintCall::path_covering(
            vec![Offset::new(2.0, 2.0)],
            vec![Offset::new(18.0, 18.0)],
        )],
    );
    assert!(
        paints(
            &ops,
            &[PaintCall::path_covering(
                vec![Offset::new(18.0, 18.0)],
                Vec::new()
            )]
        )
        .is_err()
    );
}
