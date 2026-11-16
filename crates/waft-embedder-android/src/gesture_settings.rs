//! The gesture settings Android gives a view: its touch slop, `ViewConfiguration`'s scaled touch
//! slop for the activity, which Flutter's `FlutterView` sends as the viewport metrics'
//! `physicalTouchSlop` (`sendViewportMetricsToFlutter`). Android gives no double-tap slop, so the
//! framework's own applies.
//!
//! The slop follows the configuration's density, so it is read through JNI as the view's activity
//! attaches and at each change of configuration, not with every metrics.

use waft_embedder::GestureSettings;

#[cfg(target_os = "android")]
use crate::activity::Activity;

/// The gesture settings Android gives `activity`'s window; the framework's own, said in the log,
/// where the system would not say.
#[cfg(target_os = "android")]
pub(crate) fn read_gesture_settings(activity: &Activity) -> GestureSettings {
    match activity.with(scaled_touch_slop) {
        Ok(slop) => gesture_settings_of(slop),
        Err(error) => {
            crate::log::warn_to_log!("the system would not say its touch slop: {error:?}");
            GestureSettings::default()
        }
    }
}

/// `ViewConfiguration.get(context).getScaledTouchSlop()`: the touch slop in pixels.
#[cfg(target_os = "android")]
fn scaled_touch_slop(
    env: &mut jni::Env<'_>,
    context: &jni::objects::JObject<'_>,
) -> jni::errors::Result<i32> {
    use jni::objects::JValue;
    use jni::{jni_sig, jni_str};

    let configurations = env.find_class(jni_str!("android/view/ViewConfiguration"))?;
    let configuration = env
        .call_static_method(
            &configurations,
            jni_str!("get"),
            jni_sig!("(Landroid/content/Context;)Landroid/view/ViewConfiguration;"),
            &[JValue::Object(context)],
        )?
        .l()?;
    env.call_method(
        &configuration,
        jni_str!("getScaledTouchSlop"),
        jni_sig!("()I"),
        &[],
    )?
    .i()
}

/// The settings of a view whose scaled touch slop is `scaled_touch_slop` pixels, as Flutter's
/// framework reads `physicalTouchSlop`: a negative one, Flutter's unset `-1`, leaves the
/// framework's own.
fn gesture_settings_of(scaled_touch_slop: i32) -> GestureSettings {
    GestureSettings {
        physical_touch_slop: (scaled_touch_slop >= 0).then(|| f64::from(scaled_touch_slop)),
        physical_double_tap_slop: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scaled_touch_slop_is_the_physical_touch_slop_in_pixels() {
        // A Pixel 2 XL's 8 dp at its 3.5 pixels per dp.
        let settings = gesture_settings_of(28);
        assert_eq!(settings.physical_touch_slop, Some(28.0));
        assert_eq!(settings.physical_double_tap_slop, None);
    }

    #[test]
    fn an_unset_slop_leaves_the_frameworks_own() {
        assert_eq!(gesture_settings_of(-1), GestureSettings::default());
    }
}
