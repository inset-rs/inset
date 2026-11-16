# waft-widgets/src/testing
Flutter home: packages/flutter_test/lib/src
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical
- gestures.rs → test_pointer.dart, controller.dart (`tap`, `flingFrom`, `dragFrom`, `longPress`, `sendKeyEvent`)

## tester.rs → widget_tester.dart, binding.dart (`AutomatedTestWidgetsFlutterBinding`)

- Change: `WidgetTester::pump()` and `pump_by(duration)` frame only the views that asked for a frame since the last pump, each frame of them at once.
  Reason: platform — a host frames the views that asked (`Platform::request_frame` names its view), where flutter_test's binding has one `hasScheduledFrame` for the app.
  Affect: a pump after a change to one view's tree runs no frame of the others.

- Change: dropping a tester after a case that did not panic disposes the tree and checks that no timer is pending; flutter_test's checks that debug variables are unset have no counterpart.
  Reason: language — each case has its own `App`, which holds its debug flags, clock and singletons, where flutter_test's isolate-global ones are reset and checked between tests.
  Affect: a case that leaves a flag set or a singleton changed cannot affect the next, and is not reported for it.

- Change: a tester whose pump panicked (a case that catches the panic of a build, a layout or a paint) skips the end-of-test disposal and timer check when dropped.
  Reason: language — a panic unwinds out of the frame and leaves the tree half built, where Dart's framework reports the error through `FlutterError.reportError`, finishes the frame, and the case reads it with `takeException`.
  Affect: such a case is not checked for a timer left pending; the cases that expect a build to fail pump nothing after it.

## entry.rs → widget_tester.dart (`testWidgets`, `TargetPlatformVariant`)

- Change: `test_widgets_variant(variant, body)` runs `body` once per platform inside one Rust test, stopping at the first platform that fails and naming it on stderr.
  Reason: language — Rust's test harness registers each test at compile time, so one function cannot add a test per variant value as `testWidgets(variant:)` does.
  Affect: a failure on one platform hides whether the later ones pass, and the test list shows one test, not one per platform.

## Deferred

- tester.rs: the end-of-test check for an exception the case did not take (`takeException`). Trigger: `FlutterError.reportError`.
- finders.rs: the lazy `Finder` with `skipOffstage`; the finders are eager `Vec`s, where `find_text` and `find_by_type` keep offstage elements and the `find_onstage_*` ones skip them. Trigger: the harness upgrade's step 5.
- finders.rs: `Element.debugVisitOnstageChildren`, for which `is_onstage` reads a `ToolbarItemsParentData` that does not paint as offstage only under a parent of more than one child. Trigger: the harness upgrade's step 5.
- tester.rs, gestures.rs: the helpers a case needs that the one tester lacks, each still a local copy where it is used: `tap_text` (21 waft-material files), `move_by` (waft-material's app_bar.rs and scrollbar.rs, waft-widgets' dismissible.rs, page_view.rs and overscroll_indicator.rs, waft-cupertino's refresh.rs), `fling_with_initial_offset` (dismissible.rs), `pump_frames` (overscroll_indicator.rs), `resize_view` (waft-material's date_picker.rs), and waft-material's `only_onstage_text`, `find_onstage_by_icon`, `text_field_controller_text`, `remove_pointer` and `sharing` (date_picker.rs, date_picker_theme.rs, time_picker.rs). Trigger: harness branches C and D, which add them to the tester.
