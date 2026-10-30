//! The trackpad's gestures as Flutter's pan-zoom pointer, as Flutter's macOS embedder makes them
//! (`FlutterViewController.mm`, `dispatchGestureEvent:` and the pan-zoom parts of
//! `dispatchMouseEvent:phase:`): fingers scrolling, pinching and rotating are one gesture on one
//! device, which the first of them to begin starts and the last of them to end ends, since the
//! framework takes one `PanZoomStart` at a time. Its pan, scale and rotation are accumulated from
//! each. The system's momentum after the fingers lift is not one of them: the framework makes its
//! own fling, so the momentum only marks when a touch on the trackpad stops that fling, with the
//! scroll-inertia cancel Flutter's `touchesBeganWithEvent:` sends.
//!
//! The device is reported where the mouse is. What the engine adds, the device's add, the hover
//! to where a gesture or a cancel happens, each gesture's press and its pan's deltas, the
//! client's converter adds (`PointerDataPacketConverter`).

use std::f64::consts::PI;
use std::time::{Duration, Instant};

use inset_embedder::{Contact, PointerDeviceKind, PointerReport, ReportedChange};
use winit::event::{MouseScrollDelta, TouchPhase};

use crate::pointer::PointerStamp;

/// Flutter's macOS embedder's `kPointerPanZoomDeviceId`: the mouse is device 0.
pub(crate) const PAN_ZOOM_DEVICE: i64 = 1;

/// How soon after the system's momentum last moved a touch on the trackpad stops the framework's
/// fling: Flutter's `kTrackpadTouchInertiaCancelWindowMs`.
const INERTIA_CANCEL_WINDOW: Duration = Duration::from_millis(50);

/// One of the gestures the pan-zoom is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Gesture {
    Pan,
    Scale,
    Rotate,
}

/// The trackpad, as the pan-zoom pointer Flutter's `MouseState` keeps its gestures for.
#[derive(Default)]
pub(crate) struct Trackpad {
    /// Flutter's `flutter_state_is_pan_zoom_started`.
    started: bool,
    /// Which gestures are under way, between their start and their end: Flutter's
    /// `pan_gesture_phase`, `scale_gesture_phase` and `rotate_gesture_phase`.
    panning: bool,
    scaling: bool,
    rotating: bool,
    /// The accumulated pan, in physical pixels: Flutter's `delta_x` and `delta_y`.
    pan: [f64; 2],
    /// The accumulated magnification, reported as two to its power: Flutter's `scale`.
    scale: f64,
    /// The accumulated rotation, in radians, clockwise: Flutter's `rotation`.
    rotation: f64,
    /// When the system's momentum last moved: Flutter's `last_scroll_momentum_changed_time`,
    /// kept until a touch cancels the fling or a scroll starts.
    momentum_moved_at: Option<Instant>,
}

impl Trackpad {
    /// Fingers scrolled on the trackpad by `delta`, in a view `scale` physical pixels to the
    /// logical one, with the mouse at `at`.
    pub(crate) fn scrolled(
        &mut self,
        phase: TouchPhase,
        delta: MouseScrollDelta,
        scale: f64,
        at: [f64; 2],
        stamp: PointerStamp,
    ) -> Option<PointerReport> {
        let [dx, dy] = pan_of(delta, scale);
        self.gesture(Gesture::Pan, phase, at, stamp, |trackpad| {
            trackpad.pan[0] += dx;
            trackpad.pan[1] += dy;
        })
    }

    /// Fingers pinched on the trackpad by `magnification`, with the mouse at `at`. winit may
    /// report a magnification that is not a number, which changes nothing.
    pub(crate) fn pinched(
        &mut self,
        phase: TouchPhase,
        magnification: f64,
        at: [f64; 2],
        stamp: PointerStamp,
    ) -> Option<PointerReport> {
        let magnification = if magnification.is_nan() {
            0.0
        } else {
            magnification
        };
        self.gesture(Gesture::Scale, phase, at, stamp, |trackpad| {
            trackpad.scale += magnification;
        })
    }

    /// Fingers rotated on the trackpad by `degrees`, counterclockwise, with the mouse at `at`.
    pub(crate) fn rotated(
        &mut self,
        phase: TouchPhase,
        degrees: f32,
        at: [f64; 2],
        stamp: PointerStamp,
    ) -> Option<PointerReport> {
        self.gesture(Gesture::Rotate, phase, at, stamp, |trackpad| {
            trackpad.rotation += f64::from(degrees) * (-PI / 180.0);
        })
    }

    /// The system's momentum after the fingers lifted, in `phase`, at `now`. The framework does not
    /// hear it, since it makes its own fling; a move marks when a touch stops that fling. Its start
    /// does not, as Flutter waits for the first change: macOS can report a touch between the two
    /// in low power mode.
    pub(crate) fn momentum(&mut self, phase: TouchPhase, now: Instant) {
        if phase == TouchPhase::Moved {
            self.momentum_moved_at = Some(now);
        }
    }

    /// Fingers came down on the trackpad at `touched_at`, with the mouse at `at`: Flutter's
    /// `touchesBeganWithEvent:`. Within `INERTIA_CANCEL_WINDOW` of the system's momentum moving,
    /// the framework hears a scroll-inertia cancel, which stops its fling, once.
    pub(crate) fn touched(
        &mut self,
        touched_at: Instant,
        at: [f64; 2],
        stamp: PointerStamp,
    ) -> Option<PointerReport> {
        let moved_at = self.momentum_moved_at?;
        if touched_at.saturating_duration_since(moved_at) >= INERTIA_CANCEL_WINDOW {
            return None;
        }
        self.momentum_moved_at = None;
        report(ReportedChange::ScrollInertiaCancel, at, stamp)
    }

    /// One event of `gesture`: `dispatchGestureEvent:`'s start, update or end, an update first
    /// applying `change`.
    fn gesture(
        &mut self,
        gesture: Gesture,
        phase: TouchPhase,
        at: [f64; 2],
        stamp: PointerStamp,
        change: impl FnOnce(&mut Trackpad),
    ) -> Option<PointerReport> {
        let reported = match phase {
            TouchPhase::Started => self.start(gesture),
            TouchPhase::Moved => {
                change(self);
                self.update()
            }
            TouchPhase::Ended | TouchPhase::Cancelled => self.end(gesture),
        }?;
        report(reported, at, stamp)
    }

    /// `gesture` began: the pan-zoom starts, unless another gesture already started it. A scroll
    /// starting ends what the system's momentum marked, as Flutter's does.
    fn start(&mut self, gesture: Gesture) -> Option<ReportedChange> {
        if gesture == Gesture::Pan {
            self.momentum_moved_at = None;
        }
        self.set_under_way(gesture, true);
        if std::mem::replace(&mut self.started, true) {
            return None;
        }
        Some(ReportedChange::PanZoomStart)
    }

    /// The pan-zoom changed: the framework hears its pan, scale and rotation so far, the scale
    /// normalized to grow from zero to infinity. An update before any start is dropped, as the
    /// engine takes none.
    fn update(&self) -> Option<ReportedChange> {
        self.started.then(|| ReportedChange::PanZoomUpdate {
            pan: self.pan,
            scale: 2.0_f64.powf(self.scale),
            rotation: self.rotation,
        })
    }

    /// `gesture` ended: the pan-zoom ends once no gesture is under way, and its state is reset.
    /// An end with nothing started is dropped, as Flutter drops a cancel AppKit sends out of turn.
    fn end(&mut self, gesture: Gesture) -> Option<ReportedChange> {
        self.set_under_way(gesture, false);
        if !self.started || self.panning || self.scaling || self.rotating {
            return None;
        }
        self.reset_gesture();
        Some(ReportedChange::PanZoomEnd)
    }

    fn set_under_way(&mut self, gesture: Gesture, under_way: bool) {
        match gesture {
            Gesture::Pan => self.panning = under_way,
            Gesture::Scale => self.scaling = under_way,
            Gesture::Rotate => self.rotating = under_way,
        }
    }

    /// Flutter's `GestureReset`; the momentum is no gesture's.
    fn reset_gesture(&mut self) {
        *self = Trackpad {
            momentum_moved_at: self.momentum_moved_at,
            ..Trackpad::default()
        };
    }
}

/// The pan-zoom pointer's report of `change` at `at`, in the view `stamp` names; `None` for a
/// window the embedder no longer has.
fn report(change: ReportedChange, at: [f64; 2], stamp: PointerStamp) -> Option<PointerReport> {
    Some(PointerReport {
        view_id: stamp.view?,
        time_stamp: stamp.time_stamp,
        device: PAN_ZOOM_DEVICE,
        kind: PointerDeviceKind::Trackpad,
        position: at,
        change,
        contact: Contact::default(),
    })
}

/// The pan a scroll of the fingers adds, in physical pixels, in the direction the fingers moved:
/// Flutter's `scrollingDeltaX * contentsScale`, which is winit's pixel delta, and its line delta
/// scaled to the view.
fn pan_of(delta: MouseScrollDelta, scale: f64) -> [f64; 2] {
    match delta {
        MouseScrollDelta::PixelDelta(physical) => [physical.x, physical.y],
        MouseScrollDelta::LineDelta(x, y) => [f64::from(x) * scale, f64::from(y) * scale],
        // A kind of delta winit adds later pans nothing until it is ported.
        _ => [0.0, 0.0],
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use inset_embedder::ViewId;
    use winit::dpi::PhysicalPosition;

    use super::*;

    const AT: [f64; 2] = [10.0, 20.0];

    fn stamp() -> PointerStamp {
        PointerStamp {
            view: Some(ViewId(1)),
            time_stamp: Duration::ZERO,
        }
    }

    fn pixels(dx: f64, dy: f64) -> MouseScrollDelta {
        MouseScrollDelta::PixelDelta(PhysicalPosition::new(dx, dy))
    }

    fn change(report: Option<PointerReport>) -> Option<ReportedChange> {
        report.map(|report| report.change)
    }

    /// The pan, scale and rotation of an update.
    fn totals(report: Option<PointerReport>) -> ([f64; 2], f64, f64) {
        match change(report) {
            Some(ReportedChange::PanZoomUpdate {
                pan,
                scale,
                rotation,
            }) => (pan, scale, rotation),
            other => panic!("not an update: {other:?}"),
        }
    }

    #[derive(Default)]
    struct Fixture {
        trackpad: Trackpad,
    }

    impl Fixture {
        fn scroll(&mut self, phase: TouchPhase, dx: f64, dy: f64) -> Option<PointerReport> {
            self.trackpad
                .scrolled(phase, pixels(dx, dy), 2.0, AT, stamp())
        }

        fn pinch(&mut self, phase: TouchPhase, magnification: f64) -> Option<PointerReport> {
            self.trackpad.pinched(phase, magnification, AT, stamp())
        }

        fn rotate(&mut self, phase: TouchPhase, degrees: f32) -> Option<PointerReport> {
            self.trackpad.rotated(phase, degrees, AT, stamp())
        }
    }

    #[test]
    fn a_finger_scroll_is_a_pan_zoom_from_its_start_to_its_end_where_the_mouse_is() {
        let mut fixture = Fixture::default();
        let started = fixture.scroll(TouchPhase::Started, 0.0, 0.0).unwrap();
        assert_eq!(started.change, ReportedChange::PanZoomStart);
        assert_eq!(started.kind, PointerDeviceKind::Trackpad);
        assert_eq!(started.device, PAN_ZOOM_DEVICE);
        assert_eq!(started.position, AT);
        assert!(matches!(
            change(fixture.scroll(TouchPhase::Moved, 0.0, -10.0)),
            Some(ReportedChange::PanZoomUpdate { .. })
        ));
        assert_eq!(
            change(fixture.scroll(TouchPhase::Ended, 0.0, 0.0)),
            Some(ReportedChange::PanZoomEnd)
        );
    }

    #[test]
    fn an_update_carries_the_pan_so_far() {
        let mut fixture = Fixture::default();
        fixture.scroll(TouchPhase::Started, 0.0, -99.0);
        assert_eq!(
            totals(fixture.scroll(TouchPhase::Moved, 2.0, -10.0)),
            ([2.0, -10.0], 1.0, 0.0),
            "no pinch is a scale of one"
        );
        assert_eq!(
            totals(fixture.scroll(TouchPhase::Moved, 0.0, -5.0)).0,
            [2.0, -15.0]
        );
    }

    #[test]
    fn a_line_scroll_pans_by_its_lines_at_the_view_scale() {
        assert_eq!(
            pan_of(MouseScrollDelta::LineDelta(1.0, -2.0), 2.0),
            [2.0, -4.0]
        );
    }

    #[test]
    fn a_pinch_and_a_rotation_join_the_pan_in_one_start_and_one_end() {
        let mut fixture = Fixture::default();
        assert_eq!(
            change(fixture.scroll(TouchPhase::Started, 0.0, 0.0)),
            Some(ReportedChange::PanZoomStart)
        );
        assert!(
            fixture.pinch(TouchPhase::Started, 0.0).is_none(),
            "one start for every gesture"
        );
        assert!(fixture.rotate(TouchPhase::Started, 0.0).is_none());
        assert_eq!(
            totals(fixture.pinch(TouchPhase::Moved, 1.0)).1,
            2.0,
            "two to the power of the magnification"
        );
        let (_, scale, rotation) = totals(fixture.rotate(TouchPhase::Moved, 90.0));
        assert!((rotation + PI / 2.0).abs() < 1e-9, "clockwise, in radians");
        assert_eq!(scale, 2.0, "the scale so far");
        assert!(fixture.scroll(TouchPhase::Ended, 0.0, 0.0).is_none());
        assert!(fixture.pinch(TouchPhase::Ended, 0.0).is_none());
        assert_eq!(
            change(fixture.rotate(TouchPhase::Ended, 0.0)),
            Some(ReportedChange::PanZoomEnd),
            "the last gesture to end ends the pan-zoom"
        );
        fixture.pinch(TouchPhase::Started, 0.0);
        assert_eq!(
            totals(fixture.pinch(TouchPhase::Moved, 0.0)),
            ([0.0, 0.0], 1.0, 0.0),
            "a new pan-zoom starts from no change"
        );
    }

    #[test]
    fn a_second_start_of_one_gesture_starts_nothing() {
        let mut fixture = Fixture::default();
        fixture.scroll(TouchPhase::Started, 0.0, 0.0);
        assert!(
            fixture.scroll(TouchPhase::Started, 0.0, 0.0).is_none(),
            "AppKit's may-begin and began are both a start"
        );
    }

    #[test]
    fn an_update_or_an_end_with_nothing_started_is_dropped() {
        let mut fixture = Fixture::default();
        assert!(fixture.scroll(TouchPhase::Moved, 0.0, -1.0).is_none());
        assert!(fixture.scroll(TouchPhase::Cancelled, 0.0, 0.0).is_none());
        assert!(fixture.pinch(TouchPhase::Ended, 0.0).is_none());
    }

    #[test]
    fn a_magnification_that_is_not_a_number_changes_nothing() {
        let mut fixture = Fixture::default();
        fixture.pinch(TouchPhase::Started, 0.0);
        assert_eq!(totals(fixture.pinch(TouchPhase::Moved, f64::NAN)).1, 1.0);
    }
    impl Fixture {
        fn momentum(&mut self, phase: TouchPhase, now: Instant) {
            self.trackpad.momentum(phase, now);
        }

        fn touch(&mut self, touched_at: Instant, at: [f64; 2]) -> Option<PointerReport> {
            self.trackpad.touched(touched_at, at, stamp())
        }

        /// A fling of the fingers: a scroll from its start to its end, then the system's momentum
        /// starting and moving at `moved_at`.
        fn fling(&mut self, moved_at: Instant) {
            self.scroll(TouchPhase::Started, 0.0, 0.0);
            self.scroll(TouchPhase::Moved, 16.0, 32.0);
            self.scroll(TouchPhase::Ended, 0.0, 0.0);
            self.momentum(TouchPhase::Started, moved_at);
            self.momentum(TouchPhase::Moved, moved_at);
        }
    }

    /// Flutter's `testTrackpadGesturesAreSentToFramework`, from the system's momentum on.
    #[test]
    fn a_touch_soon_after_the_momentum_moved_cancels_the_fling_once() {
        let mut fixture = Fixture::default();
        let moved_at = Instant::now();
        fixture.fling(moved_at);
        assert!(
            fixture
                .touch(moved_at + Duration::from_millis(150), AT)
                .is_none(),
            "too long after the momentum"
        );
        let cancel = fixture
            .touch(moved_at + Duration::from_millis(5), AT)
            .unwrap();
        assert_eq!(cancel.change, ReportedChange::ScrollInertiaCancel);
        assert_eq!(cancel.kind, PointerDeviceKind::Trackpad);
        assert_eq!(cancel.device, PAN_ZOOM_DEVICE);
        assert_eq!(cancel.position, AT);
        assert!(
            fixture
                .touch(moved_at + Duration::from_millis(6), AT)
                .is_none(),
            "no further cancel"
        );
        fixture.momentum(TouchPhase::Ended, moved_at);
        assert_eq!(
            change(fixture.scroll(TouchPhase::Started, 0.0, 0.0)),
            Some(ReportedChange::PanZoomStart),
            "may-begin is a pan-zoom's start"
        );
    }

    #[test]
    fn a_touch_before_the_momentum_moves_cancels_nothing() {
        let mut fixture = Fixture::default();
        let started_at = Instant::now();
        fixture.scroll(TouchPhase::Started, 0.0, 0.0);
        fixture.scroll(TouchPhase::Ended, 0.0, 0.0);
        fixture.momentum(TouchPhase::Started, started_at);
        assert!(fixture.touch(started_at, AT).is_none());
    }

    #[test]
    fn a_scroll_starting_forgets_the_momentum() {
        let mut fixture = Fixture::default();
        let moved_at = Instant::now();
        fixture.fling(moved_at);
        fixture.scroll(TouchPhase::Started, 0.0, 0.0);
        assert!(fixture.touch(moved_at, AT).is_none());
    }

    #[test]
    fn a_pinch_ending_keeps_the_momentum() {
        let mut fixture = Fixture::default();
        let moved_at = Instant::now();
        fixture.fling(moved_at);
        fixture.pinch(TouchPhase::Started, 0.0);
        fixture.pinch(TouchPhase::Ended, 0.0);
        assert!(fixture.touch(moved_at, AT).is_some());
    }
}
