//! Fingers and pens on a view as dart:ui pointer reports, as Flutter's `FlutterViewController`
//! makes its pointer data (`dispatchTouches:pointerDataChangeOverride:event:`): one device per
//! touch, the touch itself, its position in the view's physical pixels, its force and its radius;
//! a touch that ends or is cancelled leaves, as a finger leaves the
//! screen. The iPad's pointer and its scrolling are not reported.

use std::time::Duration;

use objc2_foundation::NSSet;
use objc2_ui_kit::{UITouch, UITouchPhase, UITouchType, UIView};
use waft_embedder::{Contact, PointerDeviceKind, PointerReport, ReportedChange, ViewId};

/// Flutter's `kPrimaryButton`, which a finger or a pen touching the screen holds.
const PRIMARY_BUTTON: i64 = 1;

/// What `touches`, on `ui_view`, the view numbered `view`'s, did, each in its own phase.
pub(crate) fn reports_of(
    touches: &NSSet<UITouch>,
    ui_view: &UIView,
    view: ViewId,
) -> Vec<PointerReport> {
    let scale = ui_view
        .window()
        .map_or(1.0, |window| window.screen().scale());
    let mut reports = Vec::new();
    for touch in touches.iter() {
        let report = |change| report_of(&touch, ui_view, view, scale, change);
        match touch.phase() {
            UITouchPhase::Began => reports.push(report(ReportedChange::Down {
                buttons: PRIMARY_BUTTON,
            })),
            UITouchPhase::Moved | UITouchPhase::Stationary => {
                reports.push(report(ReportedChange::Move {
                    buttons: PRIMARY_BUTTON,
                }))
            }
            UITouchPhase::Ended => {
                reports.push(report(ReportedChange::Up { buttons: 0 }));
                reports.push(report(ReportedChange::Remove));
            }
            UITouchPhase::Cancelled => {
                reports.push(report(ReportedChange::Cancel { buttons: 0 }));
                reports.push(report(ReportedChange::Remove));
            }
            _ => {}
        }
    }
    reports
}

fn report_of(
    touch: &UITouch,
    ui_view: &UIView,
    view: ViewId,
    scale: f64,
    change: ReportedChange,
) -> PointerReport {
    let location = touch.locationInView(Some(ui_view));
    PointerReport {
        view_id: view,
        time_stamp: Duration::from_secs_f64(touch.timestamp().max(0.0)),
        // Flutter's device is the touch's address, which UIKit keeps for one touch's life.
        device: std::ptr::from_ref(touch) as i64,
        kind: kind_of(touch),
        position: [location.x * scale, location.y * scale],
        change,
        contact: contact_of(touch),
    }
}

fn kind_of(touch: &UITouch) -> PointerDeviceKind {
    if touch.r#type() == UITouchType::Pencil {
        PointerDeviceKind::Stylus
    } else {
        PointerDeviceKind::Touch
    }
}

/// The touch's force against the most the screen measures, and its radius in points, as Flutter
/// reads them, a screen that measures no force answering none.
fn contact_of(touch: &UITouch) -> Contact {
    let radius = touch.majorRadius();
    let tolerance = touch.majorRadiusTolerance();
    Contact {
        pressure: touch.force(),
        pressure_min: 0.0,
        pressure_max: touch.maximumPossibleForce(),
        radius_major: radius,
        radius_min: radius - tolerance,
        radius_max: radius + tolerance,
        ..Contact::default()
    }
}
