//! The trackpad's gestures as Flutter's pan-zoom pointer, as Flutter's macOS embedder makes them
//! (`FlutterViewController.mm`, `dispatchGestureEvent:` and the pan-zoom parts of
//! `dispatchMouseEvent:phase:`): fingers scrolling, pinching and rotating are one gesture on one
//! device, which the first of them to begin starts and the last of them to end ends, since the
//! framework takes one `PanZoomStart` at a time. Its pan, scale and rotation are accumulated from
//! each. The system's momentum after the fingers lift is not one of them: the framework makes its
//! own fling, so the momentum only marks when a touch on the trackpad stops that fling, with the
//! scroll-inertia cancel Flutter's `touchesBeganWithEvent:` sends.
//!
//! The engine's part is done here too, as the mouse's is (`pointer.rs`): the device is added
//! before its first gesture and never removed, each gesture is a press of its own, each update
//! carries its pan's delta since the one before, and a gesture that starts or ends away from
//! where the device was last reported is preceded by a hover or an update there
//! (`PointerDataPacketConverter`).

use std::f64::consts::PI;
use std::time::{Duration, Instant};

use inset_embedder::{
    PointerChange, PointerData, PointerDataPacket, PointerDeviceKind, PointerSignalKind,
};
use winit::event::{MouseScrollDelta, TouchPhase};

use crate::pointer::{PointerIds, PointerStamp};

/// Flutter's macOS embedder's `kPointerPanZoomDeviceId`: the mouse is device 0.
const PAN_ZOOM_DEVICE: i64 = 1;

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
    /// Whether the framework has been told of the device, which the engine adds before its first
    /// gesture.
    added: bool,
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
    /// The pan the framework was last told, which the next update's delta is from: the engine's
    /// `PointerState.pan_x` and `pan_y`.
    reported_pan: [f64; 2],
    /// Where the framework was last told the device is, in physical pixels: the engine's
    /// `PointerState.physical_x` and `physical_y`.
    reported_at: [f64; 2],
    /// Flutter's `pointerIdentifier` of the gesture under way.
    pointer_id: i64,
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
        ids: &mut PointerIds,
    ) -> Option<PointerDataPacket> {
        let [dx, dy] = pan_of(delta, scale);
        self.gesture(Gesture::Pan, phase, at, stamp, ids, |trackpad| {
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
        ids: &mut PointerIds,
    ) -> Option<PointerDataPacket> {
        let magnification = if magnification.is_nan() {
            0.0
        } else {
            magnification
        };
        self.gesture(Gesture::Scale, phase, at, stamp, ids, |trackpad| {
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
        ids: &mut PointerIds,
    ) -> Option<PointerDataPacket> {
        self.gesture(Gesture::Rotate, phase, at, stamp, ids, |trackpad| {
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
        ids: &mut PointerIds,
    ) -> Option<PointerDataPacket> {
        let moved_at = self.momentum_moved_at?;
        if touched_at.saturating_duration_since(moved_at) >= INERTIA_CANCEL_WINDOW {
            return None;
        }
        self.momentum_moved_at = None;
        let mut data: Vec<PanZoomDatum> = self.located_at(at).into_iter().collect();
        data.push(self.inertia_cancel(at));
        packet(data, stamp, ids)
    }

    /// One event of `gesture`: `dispatchGestureEvent:`'s start, update or end, an update first
    /// applying `change`.
    fn gesture(
        &mut self,
        gesture: Gesture,
        phase: TouchPhase,
        at: [f64; 2],
        stamp: PointerStamp,
        ids: &mut PointerIds,
        change: impl FnOnce(&mut Trackpad),
    ) -> Option<PointerDataPacket> {
        match phase {
            TouchPhase::Started => self.start(gesture, at, stamp, ids),
            TouchPhase::Moved => {
                change(self);
                self.update(at, stamp, ids)
            }
            TouchPhase::Ended | TouchPhase::Cancelled => self.end(gesture, at, stamp, ids),
        }
    }

    /// `gesture` began: the pan-zoom starts, as a press of its own, unless another gesture
    /// already started it, after the device is located at the mouse. A scroll starting ends what
    /// the system's momentum marked, as Flutter's does.
    fn start(
        &mut self,
        gesture: Gesture,
        at: [f64; 2],
        stamp: PointerStamp,
        ids: &mut PointerIds,
    ) -> Option<PointerDataPacket> {
        if gesture == Gesture::Pan {
            self.momentum_moved_at = None;
        }
        self.set_under_way(gesture, true);
        if self.started {
            return None;
        }
        self.started = true;
        self.pointer_id = ids.next_pointer();
        self.reported_pan = [0.0, 0.0];
        let mut data: Vec<PanZoomDatum> = self.located_at(at).into_iter().collect();
        data.push(self.datum(PointerChange::PanZoomStart, at));
        packet(data, stamp, ids)
    }

    /// The engine's part before the device acts at `at`: its add before its first act, and a hover
    /// there before a later one away from where it was last reported.
    fn located_at(&mut self, at: [f64; 2]) -> Option<PanZoomDatum> {
        if !std::mem::replace(&mut self.added, true) {
            self.reported_at = at;
            return Some(self.datum(PointerChange::Add, at));
        }
        (self.reported_at != at).then(|| self.moved_datum(PointerChange::Hover, at).synthesized())
    }

    /// The pan-zoom changed: the framework hears its pan, scale and rotation so far. An update
    /// before any start is dropped, as the engine takes none.
    fn update(
        &mut self,
        at: [f64; 2],
        stamp: PointerStamp,
        ids: &mut PointerIds,
    ) -> Option<PointerDataPacket> {
        if !self.started {
            return None;
        }
        let datum = self.moved_datum(PointerChange::PanZoomUpdate, at);
        self.reported_pan = self.pan;
        packet(vec![datum], stamp, ids)
    }

    /// `gesture` ended: the pan-zoom ends once no gesture is under way, after an update to where
    /// the mouse is if it moved, and its state is reset. An end with nothing started is dropped, as
    /// Flutter drops a cancel AppKit sends out of turn.
    fn end(
        &mut self,
        gesture: Gesture,
        at: [f64; 2],
        stamp: PointerStamp,
        ids: &mut PointerIds,
    ) -> Option<PointerDataPacket> {
        self.set_under_way(gesture, false);
        if !self.started || self.panning || self.scaling || self.rotating {
            return None;
        }
        let mut data = Vec::new();
        if self.reported_at != at {
            data.push(
                self.moved_datum(PointerChange::PanZoomUpdate, at)
                    .synthesized(),
            );
        }
        data.push(self.datum(PointerChange::PanZoomEnd, at));
        self.reset_gesture();
        packet(data, stamp, ids)
    }

    fn set_under_way(&mut self, gesture: Gesture, under_way: bool) {
        match gesture {
            Gesture::Pan => self.panning = under_way,
            Gesture::Scale => self.scaling = under_way,
            Gesture::Rotate => self.rotating = under_way,
        }
    }

    /// Flutter's `GestureReset`; the engine keeps where the device is, and the momentum is no
    /// gesture's.
    fn reset_gesture(&mut self) {
        *self = Trackpad {
            added: self.added,
            reported_at: self.reported_at,
            momentum_moved_at: self.momentum_moved_at,
            ..Trackpad::default()
        };
    }

    /// The datum for `change` at `at`, carrying how far the device came since it was last
    /// reported, which is where it is reported from now on: the engine's `UpdateDeltaAndState`.
    fn moved_datum(&mut self, change: PointerChange, at: [f64; 2]) -> PanZoomDatum {
        let mut datum = self.datum(change, at);
        datum.delta = [at[0] - self.reported_at[0], at[1] - self.reported_at[1]];
        self.reported_at = at;
        datum
    }

    /// The pan-zoom pointer's datum for `change` at `at`, in no view yet: an update carries the
    /// pan, its delta since the last one reported, the scale normalized to grow from zero to
    /// infinity, and the rotation; the device's add is the engine's own.
    fn datum(&self, change: PointerChange, at: [f64; 2]) -> PanZoomDatum {
        let mut datum = PanZoomDatum {
            change,
            signal_kind: None,
            at,
            delta: [0.0, 0.0],
            synthesized: change == PointerChange::Add,
            pointer_identifier: self.pointer_id,
            pan: [0.0, 0.0],
            pan_delta: [0.0, 0.0],
            scale: 0.0,
            rotation: 0.0,
        };
        match change {
            PointerChange::Add | PointerChange::Hover => datum.pointer_identifier = 0,
            PointerChange::PanZoomUpdate => {
                datum.pan = self.pan;
                datum.pan_delta = [
                    self.pan[0] - self.reported_pan[0],
                    self.pan[1] - self.reported_pan[1],
                ];
                datum.scale = 2.0_f64.powf(self.scale);
                datum.rotation = self.rotation;
            }
            _ => {}
        }
        datum
    }

    /// The scroll-inertia cancel at `at`: a signal of no press, its change the cancel Flutter's
    /// macOS embedder leaves the event's phase at.
    fn inertia_cancel(&self, at: [f64; 2]) -> PanZoomDatum {
        PanZoomDatum {
            signal_kind: Some(PointerSignalKind::ScrollInertiaCancel),
            pointer_identifier: 0,
            ..self.datum(PointerChange::Cancel, at)
        }
    }
}

/// One change of the pan-zoom pointer, before it is in a view.
struct PanZoomDatum {
    change: PointerChange,
    signal_kind: Option<PointerSignalKind>,
    at: [f64; 2],
    delta: [f64; 2],
    /// Whether the engine makes it, rather than the system's event.
    synthesized: bool,
    pointer_identifier: i64,
    pan: [f64; 2],
    pan_delta: [f64; 2],
    scale: f64,
    rotation: f64,
}

impl PanZoomDatum {
    /// The same change, made by the engine.
    fn synthesized(self) -> PanZoomDatum {
        PanZoomDatum {
            synthesized: true,
            ..self
        }
    }
}

/// The packet of `data` in the view `stamp` names; `None` for a window the embedder no longer has.
fn packet(
    data: Vec<PanZoomDatum>,
    stamp: PointerStamp,
    ids: &mut PointerIds,
) -> Option<PointerDataPacket> {
    let view_id = stamp.view?;
    let data = data
        .into_iter()
        .map(|datum| PointerData {
            view_id,
            embedder_id: ids.next_embedder(),
            time_stamp: stamp.time_stamp,
            change: datum.change,
            signal_kind: datum.signal_kind,
            kind: PointerDeviceKind::Trackpad,
            synthesized: datum.synthesized,
            device: PAN_ZOOM_DEVICE,
            pointer_identifier: datum.pointer_identifier,
            physical_x: datum.at[0],
            physical_y: datum.at[1],
            physical_delta_x: datum.delta[0],
            physical_delta_y: datum.delta[1],
            pan_x: datum.pan[0],
            pan_y: datum.pan[1],
            pan_delta_x: datum.pan_delta[0],
            pan_delta_y: datum.pan_delta[1],
            scale: datum.scale,
            rotation: datum.rotation,
            ..PointerData::default()
        })
        .collect();
    Some(PointerDataPacket::new(data))
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

    fn data(packet: Option<PointerDataPacket>) -> Vec<PointerData> {
        packet.map(|packet| packet.data).unwrap_or_default()
    }

    fn changes(packet: Option<PointerDataPacket>) -> Vec<PointerChange> {
        data(packet).iter().map(|datum| datum.change).collect()
    }

    struct Fixture {
        trackpad: Trackpad,
        ids: PointerIds,
    }

    impl Fixture {
        fn new() -> Fixture {
            Fixture {
                trackpad: Trackpad::default(),
                ids: PointerIds::default(),
            }
        }

        fn scroll(&mut self, phase: TouchPhase, dx: f64, dy: f64) -> Option<PointerDataPacket> {
            self.trackpad
                .scrolled(phase, pixels(dx, dy), 2.0, AT, stamp(), &mut self.ids)
        }

        fn pinch(&mut self, phase: TouchPhase, magnification: f64) -> Option<PointerDataPacket> {
            self.trackpad
                .pinched(phase, magnification, AT, stamp(), &mut self.ids)
        }

        fn rotate(&mut self, phase: TouchPhase, degrees: f32) -> Option<PointerDataPacket> {
            self.trackpad
                .rotated(phase, degrees, AT, stamp(), &mut self.ids)
        }
    }

    #[test]
    fn a_finger_scroll_is_a_pan_zoom_from_its_start_to_its_end() {
        let mut fixture = Fixture::new();
        let started = data(fixture.scroll(TouchPhase::Started, 0.0, 0.0));
        assert_eq!(
            started.iter().map(|datum| datum.change).collect::<Vec<_>>(),
            [PointerChange::Add, PointerChange::PanZoomStart],
            "the device is added before its first gesture"
        );
        assert!(
            started
                .iter()
                .all(|datum| datum.kind == PointerDeviceKind::Trackpad)
        );
        assert!(started.iter().all(|datum| datum.device == PAN_ZOOM_DEVICE));
        assert!(started[0].synthesized, "the add is the engine's");
        assert_eq!(started[1].physical_x, AT[0]);
        assert_eq!(
            changes(fixture.scroll(TouchPhase::Moved, 0.0, -10.0)),
            [PointerChange::PanZoomUpdate]
        );
        assert_eq!(
            changes(fixture.scroll(TouchPhase::Ended, 0.0, 0.0)),
            [PointerChange::PanZoomEnd]
        );
        assert_eq!(
            changes(fixture.scroll(TouchPhase::Started, 0.0, 0.0)),
            [PointerChange::PanZoomStart],
            "added once"
        );
    }

    #[test]
    fn an_update_carries_the_pan_so_far_and_its_delta_since_the_last() {
        let mut fixture = Fixture::new();
        fixture.scroll(TouchPhase::Started, 0.0, -99.0);
        let first = data(fixture.scroll(TouchPhase::Moved, 2.0, -10.0));
        assert_eq!([first[0].pan_x, first[0].pan_y], [2.0, -10.0]);
        assert_eq!([first[0].pan_delta_x, first[0].pan_delta_y], [2.0, -10.0]);
        let second = data(fixture.scroll(TouchPhase::Moved, 0.0, -5.0));
        assert_eq!([second[0].pan_x, second[0].pan_y], [2.0, -15.0]);
        assert_eq!([second[0].pan_delta_x, second[0].pan_delta_y], [0.0, -5.0]);
        assert_eq!(second[0].scale, 1.0, "no pinch is a scale of one");
        assert_eq!(second[0].rotation, 0.0);
    }

    #[test]
    fn a_line_scroll_pans_by_its_lines_at_the_view_scale() {
        assert_eq!(
            pan_of(MouseScrollDelta::LineDelta(1.0, -2.0), 2.0),
            [2.0, -4.0]
        );
    }

    #[test]
    fn each_gesture_is_a_press_of_its_own() {
        let mut fixture = Fixture::new();
        let first = data(fixture.scroll(TouchPhase::Started, 0.0, 0.0));
        let update = data(fixture.scroll(TouchPhase::Moved, 0.0, -1.0));
        fixture.scroll(TouchPhase::Ended, 0.0, 0.0);
        let second = data(fixture.scroll(TouchPhase::Started, 0.0, 0.0));
        assert_eq!(first[0].pointer_identifier, 0, "an add is no press");
        assert_eq!(first[1].pointer_identifier, 1);
        assert_eq!(update[0].pointer_identifier, 1);
        assert_eq!(second[0].pointer_identifier, 2);
    }

    #[test]
    fn a_pinch_and_a_rotation_join_the_pan_in_one_start_and_one_end() {
        let mut fixture = Fixture::new();
        assert_eq!(
            changes(fixture.scroll(TouchPhase::Started, 0.0, 0.0)),
            [PointerChange::Add, PointerChange::PanZoomStart]
        );
        assert!(
            fixture.pinch(TouchPhase::Started, 0.0).is_none(),
            "one start for every gesture"
        );
        assert!(fixture.rotate(TouchPhase::Started, 0.0).is_none());
        let pinched = data(fixture.pinch(TouchPhase::Moved, 1.0));
        assert_eq!(
            pinched[0].scale, 2.0,
            "two to the power of the magnification"
        );
        let rotated = data(fixture.rotate(TouchPhase::Moved, 90.0));
        assert!(
            (rotated[0].rotation + PI / 2.0).abs() < 1e-9,
            "clockwise, in radians"
        );
        assert_eq!(rotated[0].scale, 2.0, "the scale so far");
        assert!(fixture.scroll(TouchPhase::Ended, 0.0, 0.0).is_none());
        assert!(fixture.pinch(TouchPhase::Ended, 0.0).is_none());
        assert_eq!(
            changes(fixture.rotate(TouchPhase::Ended, 0.0)),
            [PointerChange::PanZoomEnd],
            "the last gesture to end ends the pan-zoom"
        );
        fixture.pinch(TouchPhase::Started, 0.0);
        let fresh = data(fixture.pinch(TouchPhase::Moved, 0.0));
        assert_eq!(fresh[0].scale, 1.0, "a new pan-zoom starts from no change");
        assert_eq!(fresh[0].rotation, 0.0);
        assert_eq!(fresh[0].pan_y, 0.0);
    }

    #[test]
    fn a_gesture_away_from_where_the_device_was_hovers_there_first() {
        let mut fixture = Fixture::new();
        fixture.scroll(TouchPhase::Started, 0.0, 0.0);
        fixture.scroll(TouchPhase::Ended, 0.0, 0.0);
        let elsewhere = [30.0, 20.0];
        let started = data(fixture.trackpad.scrolled(
            TouchPhase::Started,
            pixels(0.0, 0.0),
            2.0,
            elsewhere,
            stamp(),
            &mut fixture.ids,
        ));
        assert_eq!(
            started.iter().map(|datum| datum.change).collect::<Vec<_>>(),
            [PointerChange::Hover, PointerChange::PanZoomStart]
        );
        assert!(started[0].synthesized);
        assert_eq!(started[0].physical_delta_x, 20.0);
        let ended = data(fixture.scroll(TouchPhase::Ended, 0.0, 0.0));
        assert_eq!(
            ended.iter().map(|datum| datum.change).collect::<Vec<_>>(),
            [PointerChange::PanZoomUpdate, PointerChange::PanZoomEnd],
            "an end away from the last update updates there first"
        );
        assert!(ended[0].synthesized);
        assert_eq!(ended[0].physical_delta_x, -20.0);
    }

    #[test]
    fn a_second_start_of_one_gesture_starts_nothing() {
        let mut fixture = Fixture::new();
        fixture.scroll(TouchPhase::Started, 0.0, 0.0);
        assert!(
            fixture.scroll(TouchPhase::Started, 0.0, 0.0).is_none(),
            "AppKit's may-begin and began are both a start"
        );
    }

    #[test]
    fn an_update_or_an_end_with_nothing_started_is_dropped() {
        let mut fixture = Fixture::new();
        assert!(fixture.scroll(TouchPhase::Moved, 0.0, -1.0).is_none());
        assert!(fixture.scroll(TouchPhase::Cancelled, 0.0, 0.0).is_none());
        assert!(fixture.pinch(TouchPhase::Ended, 0.0).is_none());
    }

    #[test]
    fn a_magnification_that_is_not_a_number_changes_nothing() {
        let mut fixture = Fixture::new();
        fixture.pinch(TouchPhase::Started, 0.0);
        let pinched = data(fixture.pinch(TouchPhase::Moved, f64::NAN));
        assert_eq!(pinched[0].scale, 1.0);
    }

    impl Fixture {
        fn momentum(&mut self, phase: TouchPhase, now: Instant) {
            self.trackpad.momentum(phase, now);
        }

        fn touch(&mut self, touched_at: Instant, at: [f64; 2]) -> Option<PointerDataPacket> {
            self.trackpad
                .touched(touched_at, at, stamp(), &mut self.ids)
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

    fn is_inertia_cancel(datum: &PointerData) -> bool {
        datum.signal_kind == Some(PointerSignalKind::ScrollInertiaCancel)
            && datum.kind == PointerDeviceKind::Trackpad
            && datum.device == PAN_ZOOM_DEVICE
    }

    /// Flutter's `testTrackpadGesturesAreSentToFramework`, from the system's momentum on.
    #[test]
    fn a_touch_soon_after_the_momentum_moved_cancels_the_fling_once() {
        let mut fixture = Fixture::new();
        let moved_at = Instant::now();
        fixture.fling(moved_at);
        assert!(
            fixture
                .touch(moved_at + Duration::from_millis(150), AT)
                .is_none(),
            "too long after the momentum"
        );
        let cancel = data(fixture.touch(moved_at + Duration::from_millis(5), AT));
        assert_eq!(cancel.len(), 1);
        assert!(is_inertia_cancel(&cancel[0]));
        assert_eq!([cancel[0].physical_x, cancel[0].physical_y], AT);
        assert!(
            fixture
                .touch(moved_at + Duration::from_millis(6), AT)
                .is_none(),
            "no further cancel"
        );
        fixture.momentum(TouchPhase::Ended, moved_at);
        assert_eq!(
            changes(fixture.scroll(TouchPhase::Started, 0.0, 0.0)),
            [PointerChange::PanZoomStart],
            "may-begin is a pan-zoom's start"
        );
    }

    #[test]
    fn a_touch_before_the_momentum_moves_cancels_nothing() {
        let mut fixture = Fixture::new();
        let started_at = Instant::now();
        fixture.scroll(TouchPhase::Started, 0.0, 0.0);
        fixture.scroll(TouchPhase::Ended, 0.0, 0.0);
        fixture.momentum(TouchPhase::Started, started_at);
        assert!(fixture.touch(started_at, AT).is_none());
    }

    #[test]
    fn a_scroll_starting_forgets_the_momentum() {
        let mut fixture = Fixture::new();
        let moved_at = Instant::now();
        fixture.fling(moved_at);
        fixture.scroll(TouchPhase::Started, 0.0, 0.0);
        assert!(fixture.touch(moved_at, AT).is_none());
    }

    #[test]
    fn a_pinch_ending_keeps_the_momentum() {
        let mut fixture = Fixture::new();
        let moved_at = Instant::now();
        fixture.fling(moved_at);
        fixture.pinch(TouchPhase::Started, 0.0);
        fixture.pinch(TouchPhase::Ended, 0.0);
        assert!(fixture.touch(moved_at, AT).is_some());
    }

    #[test]
    fn a_cancel_away_from_where_the_device_was_hovers_there_first() {
        let mut fixture = Fixture::new();
        let moved_at = Instant::now();
        fixture.fling(moved_at);
        let elsewhere = [30.0, 20.0];
        let cancel = data(fixture.touch(moved_at, elsewhere));
        assert_eq!(cancel[0].change, PointerChange::Hover);
        assert!(cancel[0].synthesized);
        assert_eq!(cancel[0].physical_delta_x, 20.0);
        assert!(is_inertia_cancel(&cancel[1]));
    }

    #[test]
    fn a_cancel_before_any_gesture_adds_the_device_first() {
        let mut fixture = Fixture::new();
        let moved_at = Instant::now();
        fixture.momentum(TouchPhase::Moved, moved_at);
        let cancel = data(fixture.touch(moved_at, AT));
        assert_eq!(cancel[0].change, PointerChange::Add);
        assert!(is_inertia_cancel(&cancel[1]));
    }
}
