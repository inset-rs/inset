# waft-rendering/src/testing
Flutter home: packages/flutter_test/lib/src
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## paints.rs → mock_canvas.dart (`paints`, `paintsExactlyCountTimes`), recording_canvas.dart

- Change: `paints` reads valo's finished display list, not a log of canvas calls: `drawCircle`, `drawRRect`, `drawOval` and `drawPath` are one op told apart by the path's shape, `drawLine` is an open path of two points, `drawArc` is a path that an `arc` step reads by where it runs, and a draw with no effect is not there.
  Reason: platform — valo's `DisplayListBuilder` records shapes, not calls, and drops a draw with no effect (`PaintResult`), as Flutter's engine does; Waft keeps no separate call log.
  Affect: a step for a draw with no visible effect (alpha 0, zero size) cannot match, so such a case is skipped; a rounded rect drawn with no corners matches as a `rect` and one whose corners make an oval as a `circle`, as `draw_rrect` records them.

- Change: a failed pattern is a `PaintMismatch`: the step, the call it stopped at and every call recorded, as `RecordedCall`s; `assert_paints` panics with it.
  Reason: language — a Rust test asserts on a value, with no matcher to describe a mismatch the way `expect` does.
  Affect: a failure lists the whole recording with each call's arguments.

## pictures.rs → recording_canvas.dart (`TestRecordingPaintingContext`)

- Change: `pictures_deep` and `painted_ops_deep` collect a render object's pictures and those of every layer it pushes, depth first, each in its own layer's coordinates, where `TestRecordingPaintingContext` paints a repaint-boundary child inline, translated into its parent's.
  Reason: platform — a child that is a repaint boundary records into a layer of its own, which already holds its picture.
  Affect: a pattern that runs into a repaint boundary's calls compares their positions in the child layer's coordinates.
