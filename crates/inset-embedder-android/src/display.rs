//! The screen the activity is on, as Android's `Display` describes it: the one screen the host
//! reports, as Flutter's Android embedder describes its one display to the engine
//! (`FlutterJNI.updateDisplayMetrics`, `setRefreshRateFPS`).
//!
//! `NativeActivity` hands out no `Display`, so it is read through JNI, from the activity's window
//! manager, on the app's thread. The whole screen and what the system's bars leave of it come
//! from the window manager's largest window, as Flutter's do, from API 30, which has it; before,
//! the screen is the display's real size and the bars are not known.

use android_activity::AndroidApp;
use inset_embedder::{Rect, Screen, ScreenId};
use jni::objects::{JObject, JString, JValue};
use jni::{Env, jni_sig, jni_str};

use crate::activity::{device_api_level, with_activity};

/// The first release whose window manager reports the largest window and its insets.
const MAXIMUM_WINDOW_METRICS: i32 = 30;

/// The screen the activity is on, at `scale`, the configuration's density, which every view's
/// pixel ratio is; `None`, said in the log, where the system would not say.
pub(crate) fn read_screen(app: &AndroidApp, scale: f64) -> Option<Screen> {
    match with_activity(app, |env, activity| screen_of(env, activity, scale)) {
        Ok(screen) => Some(screen),
        Err(error) => {
            crate::log::warn_to_log!("the system would not describe its display: {error:?}");
            None
        }
    }
}

fn screen_of(env: &mut Env<'_>, activity: &JObject<'_>, scale: f64) -> jni::errors::Result<Screen> {
    let manager = window_manager(env, activity)?;
    let display = default_display(env, &manager)?;
    let (frame, visible_frame) = if device_api_level() >= MAXIMUM_WINDOW_METRICS {
        largest_window(env, &manager)?
    } else {
        let size = real_size(env, &display)?;
        (size, size)
    };
    Ok(Screen {
        id: display_id(env, &display)?,
        name: display_name(env, &display)?,
        frame: in_points(frame, scale),
        visible_frame: in_points(visible_frame, scale),
        scale,
        refresh_rate: refresh_rate(env, &display)?,
    })
}

fn window_manager<'local>(
    env: &mut Env<'local>,
    activity: &JObject<'_>,
) -> jni::errors::Result<JObject<'local>> {
    env.call_method(
        activity,
        jni_str!("getWindowManager"),
        jni_sig!("()Landroid/view/WindowManager;"),
        &[],
    )?
    .l()
}

/// The display the window manager puts the activity's windows on. `getDefaultDisplay` is
/// deprecated in favour of the activity's own `getDisplay`, which arrived in API 30; this library
/// runs on 24.
fn default_display<'local>(
    env: &mut Env<'local>,
    manager: &JObject<'_>,
) -> jni::errors::Result<JObject<'local>> {
    env.call_method(
        manager,
        jni_str!("getDefaultDisplay"),
        jni_sig!("()Landroid/view/Display;"),
        &[],
    )?
    .l()
}

/// The display's own number: 0 for a phone's built-in screen, `Display.DEFAULT_DISPLAY`.
fn display_id(env: &mut Env<'_>, display: &JObject<'_>) -> jni::errors::Result<ScreenId> {
    let id = env
        .call_method(display, jni_str!("getDisplayId"), jni_sig!("()I"), &[])?
        .i()?;
    Ok(ScreenId(u64::try_from(id).unwrap_or_default()))
}

/// What the system calls the display, "Built-in Screen"; empty where it has no name.
fn display_name(env: &mut Env<'_>, display: &JObject<'_>) -> jni::errors::Result<String> {
    let name = env
        .call_method(
            display,
            jni_str!("getName"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?
        .l()?;
    if name.is_null() {
        return Ok(String::new());
    }
    env.cast_local::<JString>(name)?.try_to_string(env)
}

/// The rate the display runs at now, in hertz; `None` where it reports none.
fn refresh_rate(env: &mut Env<'_>, display: &JObject<'_>) -> jni::errors::Result<Option<f64>> {
    let rate = env
        .call_method(display, jni_str!("getRefreshRate"), jni_sig!("()F"), &[])?
        .f()?;
    Ok((rate > 0.0).then_some(f64::from(rate)))
}

/// The largest window the window manager would give the app, which is the whole screen, and what
/// the system's bars leave of it, shown or not: `getMaximumWindowMetrics`, in pixels.
fn largest_window(env: &mut Env<'_>, manager: &JObject<'_>) -> jni::errors::Result<(Rect, Rect)> {
    let metrics = env
        .call_method(
            manager,
            jni_str!("getMaximumWindowMetrics"),
            jni_sig!("()Landroid/view/WindowMetrics;"),
            &[],
        )?
        .l()?;
    let bounds = env
        .call_method(
            &metrics,
            jni_str!("getBounds"),
            jni_sig!("()Landroid/graphics/Rect;"),
            &[],
        )?
        .l()?;
    let insets = env
        .call_method(
            &metrics,
            jni_str!("getWindowInsets"),
            jni_sig!("()Landroid/view/WindowInsets;"),
            &[],
        )?
        .l()?;
    let types = env.find_class(jni_str!("android/view/WindowInsets$Type"))?;
    let system_bars = env
        .call_static_method(&types, jni_str!("systemBars"), jni_sig!("()I"), &[])?
        .i()?;
    let bars = env
        .call_method(
            &insets,
            jni_str!("getInsetsIgnoringVisibility"),
            jni_sig!("(I)Landroid/graphics/Insets;"),
            &[JValue::Int(system_bars)],
        )?
        .l()?;
    let [left, top, right, bottom] = sides(env, &bounds)?;
    let [bar_left, bar_top, bar_right, bar_bottom] = sides(env, &bars)?;
    let frame = Rect::from_ltrb(left, top, right, bottom);
    let visible_frame = Rect::from_ltrb(
        left + bar_left,
        top + bar_top,
        right - bar_right,
        bottom - bar_bottom,
    );
    Ok((frame, visible_frame))
}

/// The display's whole size, in pixels: `getRealMetrics`.
fn real_size(env: &mut Env<'_>, display: &JObject<'_>) -> jni::errors::Result<Rect> {
    let metrics = env.new_object(
        jni_str!("android/util/DisplayMetrics"),
        jni_sig!("()V"),
        &[],
    )?;
    env.call_method(
        display,
        jni_str!("getRealMetrics"),
        jni_sig!("(Landroid/util/DisplayMetrics;)V"),
        &[JValue::Object(&metrics)],
    )?;
    let width = env
        .get_field(&metrics, jni_str!("widthPixels"), jni_sig!("I"))?
        .i()?;
    let height = env
        .get_field(&metrics, jni_str!("heightPixels"), jni_sig!("I"))?
        .i()?;
    Ok(Rect::from_ltwh(
        0.0,
        0.0,
        f64::from(width),
        f64::from(height),
    ))
}

/// The `left`, `top`, `right` and `bottom` fields of a `Rect` or an `Insets`.
fn sides(env: &mut Env<'_>, object: &JObject<'_>) -> jni::errors::Result<[f64; 4]> {
    let mut side = |name: &'static jni::strings::JNIStr| -> jni::errors::Result<f64> {
        Ok(f64::from(env.get_field(object, name, jni_sig!("I"))?.i()?))
    };
    Ok([
        side(jni_str!("left"))?,
        side(jni_str!("top"))?,
        side(jni_str!("right"))?,
        side(jni_str!("bottom"))?,
    ])
}

/// `pixels` in points at `scale`.
fn in_points(pixels: Rect, scale: f64) -> Rect {
    Rect::from_ltrb(
        pixels.left / scale,
        pixels.top / scale,
        pixels.right / scale,
        pixels.bottom / scale,
    )
}
