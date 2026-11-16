# waft-rendering/src/testing
Flutter home: packages/flutter_test/lib/src
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## paints.rs → mock_canvas.dart (`paints`, `paintsExactlyCountTimes`), recording_canvas.dart

- Change: `paints` reads valo's finished display list, not a log of canvas calls: `drawCircle`, `drawRRect`, `drawOval` and `drawPath` are one op told apart by the path's shape, `drawLine` is an open path of two points, `drawArc` is a path that an `arc` step reads by where it runs, and a draw with no effect is not there.
  Reason: platform — valo's `DisplayListBuilder` records shapes, not calls, and drops a draw with no effect (`PaintResult`), as Flutter's engine does; Waft keeps no separate call log.
  Affect: a step for a draw with no visible effect (alpha 0, zero size) cannot match, so such a case is skipped. A `drawRect` or `drawRRect` with a mask filter is valo's blurred rect, read as that method's call.

- Change: a step takes an op that valo records alike for its method and another only when the op has the step's arguments, and otherwise skips it as the other method's call: a `rect` step a rect (a `drawRRect` without corners records one), an `rrect` step a rect, an oval or a path (`drawRRect` records those for corners that are none, that make an oval, or that are not circular), and a `path` step a line (a `drawPath` of one straight run records one). A `circle`, `line` or `path` step takes its own record as its method's call, as Dart's does.
  Reason: platform — valo keeps the shape, not the call, so the method of such an op is not known.
  Affect: where Dart's `rect`, `rrect` or `path` step fails at another method's call with other arguments, Waft's skips it and goes on, so it fails only when no later op of the shape fits.

- Change: a failed pattern is a `PaintMismatch`: the step, the call it stopped at and every call recorded, as `RecordedCall`s; `assert_paints` panics with it.
  Reason: language — a Rust test asserts on a value, with no matcher to describe a mismatch the way `expect` does.
  Affect: a failure lists the whole recording with each call's arguments.

## pictures.rs → recording_canvas.dart (`TestRecordingPaintingContext`)

- Change: `pictures_deep` and `painted_ops_deep` collect a render object's pictures and those of every layer it pushes, depth first, each in its own layer's coordinates, where `TestRecordingPaintingContext` paints a repaint-boundary child inline, translated into its parent's.
  Reason: platform — a child that is a repaint boundary records into a layer of its own, which already holds its picture.
  Affect: a pattern that runs into a repaint boundary's calls compares their positions in the child layer's coordinates.

## rendering_tester.rs → flutter/test/rendering/rendering_tester.dart

- Change: `layout(app, box)` and `pump_frame(app, phase)` lay the box out under a repaint boundary of the app's, tight to the 800x600 test view, with its own pipeline owner, where Dart's `TestRenderingFlutterBinding` lays it out under its `RenderView`; `EnginePhase` stops at `Paint`.
  Reason: platform — a `RenderView` stands for a host's view, which a render-object test has none of, and compositing and semantics need the scene and the semantics tree.
  Affect: a case pumps through `Paint` at most, and reads the root with `test_render_root(app)`.

- Change: `painted_ops` and `pictures` bring the render tree's compositing bits up to date before they paint, where Dart's `paints` paints after a layout-only frame as it is.
  Reason: platform — they paint with a real `PaintingContext`, which needs each child's compositing bits current, where Dart's `TestRecordingPaintingContext` paints children directly.
  Affect: reading paint runs the frame's compositing-bits phase early; a case pumps the phase Dart's does.

## more_or_less.rs → matchers.dart (`moreOrLessEquals`, `offsetMoreOrLessEquals`, `rectMoreOrLessEquals`, `matrixMoreOrLessEquals`)

- Change: `assert_f32_equals` compares two values within `f32` rounding relative to the expected one, for a value that went through valo, where Dart's `expect(actual, expected)` compares exactly.
  Reason: platform — valo's matrices and text layout keep `f32`, where Dart's keep `f64`.
  Affect: a ported exact comparison of a global position, a transformed rect or a text width passes within one `f32` step.

- Change: `MoreOrLessEquals for Matrix4` compares `f32` entries, as Waft's `Matrix4` keeps them.
  Reason: platform — valo's matrix is `f32`.
  Affect: a case comparing matrices passes an `epsilon` above `f32` rounding, not Dart's default 1e-10.
