//! The harness's own rules: its defaults, which frames it runs, when time passes, and what it
//! checks when a case ends.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Mutex;
use std::time::Duration;

use waft_embedder::{Size, TargetPlatform, TextDirection};
use waft_foundation::{Listener, Timer};
use waft_scheduler::{FrameTarget, PersistentFrameCallback, SchedulerBinding};

use super::{TargetPlatformVariant, WidgetTester, test_widgets, test_widgets_variant};
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
        assert_eq!(tester.size_of(&key), Size::new(800.0, 600.0));
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
        assert_eq!(tester.size_of(&key), Size::new(28.0, 14.0));
    });
}

#[test]
fn pump_widget_runs_one_frame() {
    test_widgets(|tester| {
        let frames = counting_frames(tester);
        tester.pump_widget(SizedBox::new());
        assert_eq!(frames.get(), 1);
        assert_eq!(tester.view().presented(), 1);
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
            TargetPlatform::Fuchsia,
            TargetPlatform::IOS
        ]
    );
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
