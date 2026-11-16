//! The harness's own rules: its defaults, which frames it runs, when time passes, and what it
//! checks when a case ends.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Mutex;
use std::time::Duration;

use waft_embedder::{Size, TargetPlatform, TextDirection};
use waft_foundation::{Listener, Timer};
use waft_scheduler::{FrameTarget, PersistentFrameCallback, SchedulerBinding};

use super::{
    TargetPlatformVariant, WidgetTester, expect, find, finds_nothing, finds_one_widget,
    test_widgets, test_widgets_variant,
};
use crate::IntoWidget;
use crate::framework::GlobalKey;
use crate::widgets::basic::{Center, Directionality, SizedBox};
use crate::widgets::text::Text;

/// A timer that counts how often it fired.
fn counting_timer(tester: &WidgetTester, after: Duration) -> Rc<Cell<u32>> {
    let fired = Rc::new(Cell::new(0));
    let count = Rc::clone(&fired);
    tester.update(|app| {
        Timer::new(
            app,
            after,
            Listener::new(move |_| count.set(count.get() + 1)),
        )
    });
    fired
}

/// Counts the frames from here on, each of the tester's view alone.
fn counting_frames(tester: &WidgetTester) -> Rc<Cell<u32>> {
    let frames = Rc::new(Cell::new(0));
    let count = Rc::clone(&frames);
    let view = waft_embedder::View::id(&**tester.view());
    tester.update(|app| {
        SchedulerBinding::add_persistent_frame_callback(
            app,
            PersistentFrameCallback::new(move |_, views, _| {
                assert_eq!(views, [view]);
                count.set(count.get() + 1);
            }),
        )
    });
    frames
}

#[test]
fn the_view_is_800_by_600_logical_pixels_at_3_on_android() {
    test_widgets(|tester| {
        let key = GlobalKey::new();
        tester.pump_widget(SizedBox::expand().key(key.clone().into()));
        assert_eq!(tester.get_size(&key), Size::new(800.0, 600.0));
        assert_eq!(tester.device_pixel_ratio(), 3.0);
        assert_eq!(
            waft_embedder::View::metrics(&**tester.view()).physical_size,
            [2400.0, 1800.0]
        );
        assert_eq!(tester.target_platform(), TargetPlatform::Android);
    });
}

#[test]
fn text_is_set_in_flutter_test_whose_glyphs_are_squares_of_the_font_size() {
    test_widgets(|tester| {
        let key = GlobalKey::new();
        tester.pump_widget(Directionality::new(
            TextDirection::Ltr,
            Center::new().child(Text::new("Hi").key(key.clone().into())),
        ));
        assert_eq!(tester.get_size(&key), Size::new(28.0, 14.0));
    });
}

// flutter_test's `_runTestBody` pumps `Container(key: UniqueKey(), child: _preTestMessage)`
// before the case, so the case's first `pumpWidget` replaces a tree.
#[test]
fn a_case_starts_on_flutter_tests_pre_test_tree() {
    test_widgets(|tester| {
        expect(tester, &find::text("Test starting..."), finds_one_widget());
        assert_eq!(tester.view().presented(), 1);
        tester.pump_widget(SizedBox::new());
        expect(tester, &find::text("Test starting..."), finds_nothing());
    });
}

#[test]
fn pump_widget_runs_one_frame() {
    test_widgets(|tester| {
        let frames = counting_frames(tester);
        let presented = tester.view().presented();
        tester.pump_widget(SizedBox::new());
        assert_eq!(frames.get(), 1);
        assert_eq!(tester.view().presented(), presented + 1);
    });
}

#[test]
fn a_pump_frames_only_a_view_that_asked() {
    test_widgets(|tester| {
        let frames = counting_frames(tester);
        tester.pump_widget(SizedBox::new());
        tester.pump();
        tester.pump_by(Duration::from_millis(100));
        assert_eq!(frames.get(), 1, "no view asked");

        let view = waft_embedder::View::id(&**tester.view());
        tester.update(|app| SchedulerBinding::schedule_frame(app, FrameTarget::View(view)));
        tester.pump();
        assert_eq!(frames.get(), 2, "the view asked");
    });
}

#[test]
fn pump_fires_no_timer_and_pump_by_fires_those_due() {
    test_widgets(|tester| {
        let now = counting_timer(tester, Duration::ZERO);
        let later = counting_timer(tester, Duration::from_millis(100));

        tester.pump();
        assert_eq!((now.get(), later.get()), (0, 0));

        tester.pump_by(Duration::ZERO);
        assert_eq!((now.get(), later.get()), (1, 0));

        tester.pump_by(Duration::from_millis(99));
        assert_eq!(later.get(), 0);
        tester.pump_by(Duration::from_millis(1));
        assert_eq!(later.get(), 1);
    });
}

#[test]
#[should_panic(expected = "A Timer is still pending even after the widget tree was disposed.")]
fn a_timer_still_pending_at_the_end_fails_the_case() {
    test_widgets(|tester| {
        tester.pump_widget(SizedBox::new());
        counting_timer(tester, Duration::from_secs(1));
    });
}

#[test]
fn a_variant_runs_the_case_once_per_platform_in_order() {
    let ran = Mutex::new(Vec::new());
    test_widgets_variant(TargetPlatformVariant::mobile(), |tester| {
        ran.lock().unwrap().push(tester.target_platform());
    });
    assert_eq!(
        *ran.lock().unwrap(),
        [
            TargetPlatform::Android,
            TargetPlatform::IOS,
            TargetPlatform::Fuchsia
        ]
    );
}

/// flutter_test leaves the tree alone after a test that failed; a case that caught the panic of
/// a build left the tree half built, so its tester ends without disposing it.
#[test]
fn a_case_that_caught_a_build_panic_ends_without_tearing_down() {
    test_widgets(|tester| {
        let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tester.pump_widget(crate::widgets::basic::Builder::new(|_, _| {
                panic!("the build fails")
            }));
        }));
        assert!(built.is_err());
    });
}

#[test]
fn each_case_has_its_own_app_and_flags() {
    test_widgets(|tester| {
        tester.update(|app| crate::EditableText::set_debug_deterministic_cursor(app, true));
    });
    test_widgets(|tester| {
        assert!(!tester.update(|app| crate::EditableText::debug_deterministic_cursor(app)));
    });
}

/// Two `Text`s and an offstage one, the second keyed, in a column.
fn three_texts(tester: &mut WidgetTester, key: &GlobalKey) {
    use crate::widgets::basic::{Column, Offstage};
    tester.pump_widget(Directionality::new(
        TextDirection::Ltr,
        Column::new().children([
            Text::new("a").into_widget(),
            SizedBox::new()
                .key(key.clone().into())
                .child(Text::new("b"))
                .into_widget(),
            Offstage::new().child(Text::new("hidden")).into_widget(),
        ]),
    ));
}

#[test]
fn a_finder_skips_what_is_offstage_unless_asked() {
    use super::{expect, find, finds_nothing, finds_one_widget};
    test_widgets(|tester| {
        let key = GlobalKey::new();
        three_texts(tester, &key);
        expect(tester, &find::text("a"), finds_one_widget());
        expect(tester, &find::text("hidden"), finds_nothing());
        expect(
            tester,
            &find::text("hidden").skip_offstage(false),
            finds_one_widget(),
        );
        assert_eq!(find::by_type::<Text>().evaluate(tester).len(), 2);
    });
}

#[test]
fn first_last_at_descendant_and_ancestor_follow_tree_order() {
    use super::{expect, find, finds_nothing, finds_one_widget};
    test_widgets(|tester| {
        let key = GlobalKey::new();
        three_texts(tester, &key);
        let texts = find::by_type::<Text>();
        assert_eq!(
            tester
                .widget(texts.clone().first(), |text: &Text| text.data.clone())
                .as_deref(),
            Some("a")
        );
        assert_eq!(
            tester
                .widget(texts.clone().last(), |text: &Text| text.data.clone())
                .as_deref(),
            Some("b")
        );
        expect(tester, &texts.clone().at(1), finds_one_widget());
        expect(tester, &texts.at(2), finds_nothing());
        let under_key = find::descendant(find::by_global_key(&key), find::by_type::<Text>());
        assert_eq!(
            tester
                .widget(&under_key, |text: &Text| text.data.clone())
                .as_deref(),
            Some("b")
        );
        expect(
            tester,
            &find::ancestor(find::text("b"), find::by_type::<SizedBox>()),
            finds_one_widget(),
        );
        expect(
            tester,
            &find::widget_with_text::<SizedBox>("b"),
            finds_one_widget(),
        );
        expect(
            tester,
            &find::widget_with_text::<SizedBox>("a"),
            finds_nothing(),
        );
    });
}

#[test]
#[should_panic(
    expected = "Expected: exactly one matching candidate\n  Actual: Found 2 widgets \
                           with type \"Text\""
)]
fn a_finder_matcher_fails_with_darts_description() {
    use super::{expect, find, finds_one_widget};
    test_widgets(|tester| {
        let key = GlobalKey::new();
        three_texts(tester, &key);
        expect(tester, &find::by_type::<Text>(), finds_one_widget());
    });
}

#[test]
fn tap_finds_its_one_target_and_uses_the_next_pointer() {
    use super::find;
    use crate::widgets::gesture_detector::GestureDetector;
    test_widgets(|tester| {
        let taps = Rc::new(Cell::new(0));
        let count = Rc::clone(&taps);
        tester.pump_widget(
            Center::new().child(
                GestureDetector::new()
                    .behavior(waft_rendering::HitTestBehavior::Opaque)
                    .on_tap(Listener::new(move |_| count.set(count.get() + 1)))
                    .child(SizedBox::new().width(100.0).height(100.0)),
            ),
        );
        tester.tap(find::by_type::<GestureDetector>());
        tester.tap(find::by_type::<GestureDetector>());
        assert_eq!(taps.get(), 2);
        assert_eq!(tester.next_pointer(), 3);
    });
}

#[test]
#[should_panic(
    expected = "ambiguously found multiple matching widgets. The \"tap()\" method needs a \
                single target."
)]
fn tap_needs_a_single_target() {
    use super::find;
    test_widgets(|tester| {
        let key = GlobalKey::new();
        three_texts(tester, &key);
        tester.tap(find::by_type::<Text>());
    });
}

#[test]
fn pump_frames_pumps_until_the_duration_has_passed() {
    test_widgets(|tester| {
        let frames = counting_frames(tester);
        // Each pump frames only if one was asked; a ticking animation asks each frame.
        tester.pump_frames(SizedBox::new(), Duration::from_millis(50));
        assert_eq!(frames.get(), 1);
        assert_eq!(
            tester.cell.borrow().clock(),
            Duration::from_micros(16_683 * 3)
        );
    });
}

#[test]
fn a_view_metric_set_mid_case_reaches_the_framework_and_reset_puts_it_back() {
    use std::cell::RefCell;

    use waft_embedder::ViewPadding;

    use crate::widgets::basic::Builder;
    use crate::widgets::media_query::{MediaQuery, MediaQueryData};

    test_widgets(|tester| {
        let seen: Rc<RefCell<Option<MediaQueryData>>> = Rc::default();
        let read = Rc::clone(&seen);
        tester.pump_widget(Builder::new(move |app, context| {
            *read.borrow_mut() = Some(MediaQuery::of(app, context));
            SizedBox::new().into_widget()
        }));
        let data = || seen.borrow().clone().expect("built");
        assert_eq!(data().size, Size::new(800.0, 600.0));

        tester.view().set_physical_size(Size::new(1200.0, 2400.0));
        tester.view().set_view_insets(ViewPadding {
            bottom: 300.0,
            ..ViewPadding::ZERO
        });
        tester.pump();
        assert_eq!(data().size, Size::new(400.0, 800.0));
        assert_eq!(data().view_insets.bottom, 100.0);

        tester.view().set_device_pixel_ratio(1.0);
        tester.pump();
        assert_eq!(data().size, Size::new(1200.0, 2400.0));
        assert_eq!(tester.device_pixel_ratio(), 1.0);

        tester.view().reset();
        tester.pump();
        assert_eq!(data().size, Size::new(800.0, 600.0));
        assert_eq!(data().view_insets.bottom, 0.0);
    });
}
