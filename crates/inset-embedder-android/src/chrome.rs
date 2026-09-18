//! The system's bars over the app: their colour, and the colour of their icons.
//!
//! The status bar and the navigation bar are drawn by the system, not by the app, so the
//! system is the one that has to be told what is behind them. An app that says nothing
//! keeps whatever the device's theme chose, which over a light page is often white icons
//! on white. Flutter's engine answers the same ask in
//! `PlatformPlugin.setSystemChromeSystemUIOverlayStyle`, and the split below is its: a
//! window from API 30 carries an insets controller that owns the bars' appearance, and
//! before that the appearance is flags on the decor view.
//!
//! Everything here is posted to the Java main thread, which is where a window and its
//! decor view may be touched, so each call returns at once and the framework never waits.

// The VM and the activity reach this crate as raw pointers, which JNI takes back as its
// own handles; that is the `unsafe` here.
#![allow(unsafe_code)]

use android_activity::AndroidApp;
use inset_embedder::{
    ApplicationSwitcherDescription, Brightness, SystemChrome, SystemUiOverlayStyle,
};
use jni::objects::{JObject, JValue};
use jni::{Env, jni_sig, jni_str};

/// `View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR`, API 23: a light status bar, which is to say
/// dark icons on it.
const LIGHT_STATUS_BAR_FLAG: i32 = 0x2000;
/// `View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR`, API 26.
const LIGHT_NAVIGATION_BAR_FLAG: i32 = 0x10;
/// `WindowInsetsController.APPEARANCE_LIGHT_STATUS_BARS`, API 30.
const LIGHT_STATUS_BARS_APPEARANCE: i32 = 0x8;
/// `WindowInsetsController.APPEARANCE_LIGHT_NAVIGATION_BARS`, API 30.
const LIGHT_NAVIGATION_BARS_APPEARANCE: i32 = 0x10;

/// The first release whose window owns its bars' appearance through an insets controller.
const INSETS_CONTROLLER: i32 = 30;
/// The first release whose window takes a colour for the line above the navigation bar.
const NAVIGATION_BAR_DIVIDER: i32 = 28;
/// The first release that scrims a transparent bar for contrast, and lets an app say not to.
const BAR_CONTRAST: i32 = 29;
/// The first release whose task description takes an icon by resource id rather than a
/// bitmap.
const TASK_ICON_BY_ID: i32 = 28;

/// The system bars around the activity's window.
pub(crate) struct AndroidChrome {
    app: AndroidApp,
}

impl AndroidChrome {
    pub(crate) fn new(app: AndroidApp) -> AndroidChrome {
        AndroidChrome { app }
    }
}

impl SystemChrome for AndroidChrome {
    fn set_overlay_style(&self, style: &SystemUiOverlayStyle) {
        let style = *style;
        on_the_activity(
            &self.app,
            "a style for the system bars",
            move |env, activity| {
                let window = window_of(env, activity)?;
                paint_the_bars(env, &window, &style)?;
                light_or_dark_icons(env, &window, &style)
            },
        );
    }

    fn set_application_switcher_description(&self, description: &ApplicationSwitcherDescription) {
        let description = description.clone();
        on_the_activity(
            &self.app,
            "a description for the switcher",
            move |env, activity| describe_the_task(env, activity, &description),
        );
    }
}

/// The colours the bars themselves take. A colour only shows on a window that draws the
/// bars' backgrounds, which is the platform's own rule, and the one Flutter's engine
/// leaves in place rather than forcing the flag on.
fn paint_the_bars(
    env: &mut Env<'_>,
    window: &JObject<'_>,
    style: &SystemUiOverlayStyle,
) -> jni::errors::Result<()> {
    let api = device_api_level();
    if let Some(color) = style.status_bar_color {
        set_color(
            env,
            window,
            jni_str!("setStatusBarColor"),
            color.to_argb32(),
        )?;
    }
    if let Some(color) = style.system_navigation_bar_color {
        set_color(
            env,
            window,
            jni_str!("setNavigationBarColor"),
            color.to_argb32(),
        )?;
    }
    if let Some(color) = style.system_navigation_bar_divider_color
        && api >= NAVIGATION_BAR_DIVIDER
    {
        set_color(
            env,
            window,
            jni_str!("setNavigationBarDividerColor"),
            color.to_argb32(),
        )?;
    }
    if api < BAR_CONTRAST {
        return Ok(());
    }
    if let Some(enforced) = style.system_status_bar_contrast_enforced {
        set_flag(
            env,
            window,
            jni_str!("setStatusBarContrastEnforced"),
            enforced,
        )?;
    }
    if let Some(enforced) = style.system_navigation_bar_contrast_enforced {
        set_flag(
            env,
            window,
            jni_str!("setNavigationBarContrastEnforced"),
            enforced,
        )?;
    }
    Ok(())
}

/// Which way round the bars' icons are drawn. `status_bar_brightness` is iOS's field and
/// is not read here, as it is not read by Flutter's Android engine.
fn light_or_dark_icons(
    env: &mut Env<'_>,
    window: &JObject<'_>,
    style: &SystemUiOverlayStyle,
) -> jni::errors::Result<()> {
    if device_api_level() >= INSETS_CONTROLLER {
        return ask_the_insets_controller(env, window, style);
    }
    set_the_decor_flags(env, window, style)
}

/// A window from API 30 on: its insets controller holds the bars' appearance, and a mask
/// says which bits of it this call is about.
fn ask_the_insets_controller(
    env: &mut Env<'_>,
    window: &JObject<'_>,
    style: &SystemUiOverlayStyle,
) -> jni::errors::Result<()> {
    let (appearance, mask) = appearance_of(style);
    if mask == 0 {
        return Ok(());
    }
    let controller = env
        .call_method(
            window,
            jni_str!("getInsetsController"),
            jni_sig!("()Landroid/view/WindowInsetsController;"),
            &[],
        )?
        .l()?;
    if controller.is_null() {
        return Ok(());
    }
    env.call_method(
        &controller,
        jni_str!("setSystemBarsAppearance"),
        jni_sig!("(II)V"),
        &[JValue::Int(appearance), JValue::Int(mask)],
    )?;
    Ok(())
}

/// The bits to set and the bits this call speaks for. A brightness the style leaves out is
/// left as the window has it.
fn appearance_of(style: &SystemUiOverlayStyle) -> (i32, i32) {
    let mut appearance = 0;
    let mut mask = 0;
    if let Some(brightness) = style.status_bar_icon_brightness {
        mask |= LIGHT_STATUS_BARS_APPEARANCE;
        if brightness == Brightness::Dark {
            appearance |= LIGHT_STATUS_BARS_APPEARANCE;
        }
    }
    if let Some(brightness) = style.system_navigation_bar_icon_brightness {
        mask |= LIGHT_NAVIGATION_BARS_APPEARANCE;
        if brightness == Brightness::Dark {
            appearance |= LIGHT_NAVIGATION_BARS_APPEARANCE;
        }
    }
    (appearance, mask)
}

/// A window before API 30: the appearance is flags on the decor view, read back and
/// written whole, so a bit the style says nothing about keeps its value.
fn set_the_decor_flags(
    env: &mut Env<'_>,
    window: &JObject<'_>,
    style: &SystemUiOverlayStyle,
) -> jni::errors::Result<()> {
    let decor = env
        .call_method(
            window,
            jni_str!("getDecorView"),
            jni_sig!("()Landroid/view/View;"),
            &[],
        )?
        .l()?;
    let mut flags = env
        .call_method(
            &decor,
            jni_str!("getSystemUiVisibility"),
            jni_sig!("()I"),
            &[],
        )?
        .i()?;
    if let Some(brightness) = style.status_bar_icon_brightness {
        flags = with_bit(flags, LIGHT_STATUS_BAR_FLAG, brightness == Brightness::Dark);
    }
    if let Some(brightness) = style.system_navigation_bar_icon_brightness {
        flags = with_bit(
            flags,
            LIGHT_NAVIGATION_BAR_FLAG,
            brightness == Brightness::Dark,
        );
    }
    env.call_method(
        &decor,
        jni_str!("setSystemUiVisibility"),
        jni_sig!("(I)V"),
        &[JValue::Int(flags)],
    )?;
    Ok(())
}

/// How the app appears in the switcher: `ActivityManager.TaskDescription`, whose icon this
/// leaves to the manifest. The older constructor takes a bitmap where the newer takes a
/// resource id, and both are happy with nothing.
fn describe_the_task(
    env: &mut Env<'_>,
    activity: &JObject<'_>,
    description: &ApplicationSwitcherDescription,
) -> jni::errors::Result<()> {
    let label = match &description.label {
        Some(label) => JObject::from(env.new_string(label)?),
        None => JObject::null(),
    };
    let color = description.primary_color.unwrap_or(0) as i32;
    let class = env.find_class(jni_str!("android/app/ActivityManager$TaskDescription"))?;
    let task = if device_api_level() >= TASK_ICON_BY_ID {
        env.new_object(
            &class,
            jni_sig!("(Ljava/lang/String;II)V"),
            &[JValue::Object(&label), JValue::Int(0), JValue::Int(color)],
        )?
    } else {
        let icon = JObject::null();
        env.new_object(
            &class,
            jni_sig!("(Ljava/lang/String;Landroid/graphics/Bitmap;I)V"),
            &[
                JValue::Object(&label),
                JValue::Object(&icon),
                JValue::Int(color),
            ],
        )?
    };
    env.call_method(
        activity,
        jni_str!("setTaskDescription"),
        jni_sig!("(Landroid/app/ActivityManager$TaskDescription;)V"),
        &[JValue::Object(&task)],
    )?;
    Ok(())
}

fn with_bit(flags: i32, bit: i32, set: bool) -> i32 {
    if set { flags | bit } else { flags & !bit }
}

fn set_color(
    env: &mut Env<'_>,
    window: &JObject<'_>,
    method: &'static jni::strings::JNIStr,
    color: u32,
) -> jni::errors::Result<()> {
    env.call_method(
        window,
        method,
        jni_sig!("(I)V"),
        &[JValue::Int(color as i32)],
    )?;
    Ok(())
}

fn set_flag(
    env: &mut Env<'_>,
    window: &JObject<'_>,
    method: &'static jni::strings::JNIStr,
    value: bool,
) -> jni::errors::Result<()> {
    env.call_method(window, method, jni_sig!("(Z)V"), &[JValue::Bool(value)])?;
    Ok(())
}

fn window_of<'local>(
    env: &mut Env<'local>,
    activity: &JObject<'_>,
) -> jni::errors::Result<JObject<'local>> {
    env.call_method(
        activity,
        jni_str!("getWindow"),
        jni_sig!("()Landroid/view/Window;"),
        &[],
    )?
    .l()
}

/// Runs `work` on the Java main thread with the activity in hand, and says in the log
/// whatever it turned out the system would not take.
fn on_the_activity(
    app: &AndroidApp,
    what: &'static str,
    work: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> jni::errors::Result<()> + Send + 'static,
) {
    let app = app.clone();
    let posted = app.clone();
    posted.run_on_java_main_thread(Box::new(move || {
        if let Err(error) = with_activity(&app, work) {
            crate::log::warn_to_log!("the system would not take {what}: {error:?}");
        }
    }));
}

fn with_activity(
    app: &AndroidApp,
    work: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> jni::errors::Result<()>,
) -> jni::errors::Result<()> {
    // SAFETY: this process's own VM, as android-activity hands it out.
    let vm = unsafe { jni::JavaVM::from_raw(app.vm_as_ptr().cast()) };
    vm.attach_current_thread(|env| {
        // SAFETY: an unowned global reference to the activity, borrowed for this frame.
        let activity = unsafe { JObject::from_raw(env, app.activity_as_ptr().cast()) };
        work(env, &activity)
    })
}

/// The release this app is running on, which decides how the bars are asked.
fn device_api_level() -> i32 {
    // SAFETY: a plain query of the running system, part of the platform since API 24.
    unsafe { ndk_sys::android_get_device_api_level() }
}
