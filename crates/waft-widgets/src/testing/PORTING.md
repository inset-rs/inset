# waft-widgets/src/testing
Flutter home: packages/flutter_test/lib/src
Ported against: ed2132410ee94b5a590cb7f67cee7a6ea9101a60

## Identical
- gestures.rs → test_pointer.dart, controller.dart (`tap`, `tapAt`, `fling`, `flingFrom` with `initialOffset`, `drag`, `dragFrom`, `longPress`, `longPressAt`, `moveBy`, `sendKeyEvent`, `nextPointer`)
- matchers.rs → matchers.dart (`findsNothing`, `findsOneWidget`, `findsWidgets`, `findsNWidgets`, `findsAtLeastNWidgets`)

## tester.rs → widget_tester.dart, binding.dart (`AutomatedTestWidgetsFlutterBinding`)

- Change: `WidgetTester::pump()` and `pump_by(duration)` frame only the views that asked for a frame since the last pump, each frame of them at once.
  Reason: platform — a host frames the views that asked (`Platform::request_frame` names its view), where flutter_test's binding has one `hasScheduledFrame` for the app.
  Affect: a pump after a change to one view's tree runs no frame of the others.

- Change: dropping a tester after a case that did not panic disposes the tree and checks that no timer is pending; flutter_test's checks that debug variables are unset have no counterpart.
  Reason: language — each case has its own `App`, which holds its debug flags, clock and singletons, where flutter_test's isolate-global ones are reset and checked between tests.
  Affect: a case that leaves a flag set or a singleton changed cannot affect the next, and is not reported for it.

- Change: a view metric a case sets (`tester.view().set_physical_size(..)` and the rest) reaches the framework at once, through a listener the tester installs on its `TestView`, as the shell's `view_metrics_changed` does: `RendererBinding::handle_metrics_changed`, then the checkpoint.
  Reason: platform — `TestFlutterView`'s setters call the dispatcher's `onMetricsChanged`; here the tester stands in for the shell, which hands a view's changes to the framework.
  Affect: a case sets a metric while it does not hold the app, and needs no `addTearDown(tester.view.reset)`, since each case has its own view.

- Change: a tester whose pump panicked (a case that catches the panic of a build, a layout or a paint) skips the end-of-test disposal and timer check when dropped.
  Reason: language — a panic unwinds out of the frame and leaves the tree half built, where Dart's framework reports the error through `FlutterError.reportError`, finishes the frame, and the case reads it with `takeException`.
  Affect: such a case is not checked for a timer left pending; the cases that expect a build to fail pump nothing after it.

## finder.rs → finders.dart, tree_traversal.dart

- Change: `find::text_containing` and `find::by_tooltip` take a string, matched by containment and by equality; Dart's take a `Pattern`, a `RegExp` among them.
  Reason: language — Rust has no `Pattern` type a string and a regular expression share, and the framework has no regular expressions.
  Affect: a case that finds by a regular expression matches its text with `find::by_widget_predicate`.

- Change: `find::descendant_or_self` and `find::ancestor_or_self` are Dart's `matchRoot: true`, and `Finder::skip_offstage(false)` is Dart's `skipOffstage: false`.
  Reason: language — Rust has no named optional arguments.
  Affect: none beyond the spelling.

- Change: a tester method that acts on one widget (`tap`, `widget`, `get_rect`, …) takes a `FindsOne` or `RenderTarget`: a `Finder`, which must find exactly one, or an element, a global key or a render box.
  Reason: language — Dart's `FinderBase<Element>` parameter is one type, and Rust has no overload by type for the element and render-box forms the older helpers pass.
  Affect: an older helper's element or render box still works where a finder would.

## entry.rs → widget_tester.dart (`testWidgets`, `TargetPlatformVariant`)

- Change: `test_widgets_variant(variant, body)` runs `body` once per platform inside one Rust test, stopping at the first platform that fails and naming it on stderr.
  Reason: language — Rust's test harness registers each test at compile time, so one function cannot add a test per variant value as `testWidgets(variant:)` does.
  Affect: a failure on one platform hides whether the later ones pass, and the test list shows one test, not one per platform.

## Deferred

- tester.rs: the end-of-test check for an exception the case did not take (`takeException`). Trigger: `FlutterError.reportError`.
- finder.rs: `find.bySemanticsLabel` and the semantics finders. Trigger: the semantics tree.
- finder.rs: `find.image`, `find.widgetWithImage`, `find.bySubtype`, `find.byElementType`, `find.backButton`, `find.closeButton` and `Finder.hitTestable`. Trigger: a case that needs one.
- finders.rs: the older eager helpers (`elements`, `ancestors`, `is_onstage`, `find_onstage_by_key`, `descendants_of_type`, `ancestor_of_type`, `center_of`, `global_rect`, `render_box_of`) and the cases that hold an `AnyElement` to hand them, which Dart writes as a finder (`find.descendant(of:)`, `find.ancestor(of:)`, `tester.renderObject(finder)`, `tester.getRect(finder)`). No `find::by_element` stands in: flutter_test has none, and a tester method already takes an element. Trigger: the next edit of each case's file.
