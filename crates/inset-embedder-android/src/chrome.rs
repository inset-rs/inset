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
//! The bars are the activity's in front, the one the user is in, and the app's description in
//! the switcher is every task's (`task`). Everything here is posted to the Java main thread, which
//! is where a window and its decor view may be touched, so each call returns at once and the
//! framework never waits.

use inset_embedder::{
    ApplicationSwitcherDescription, Brightness, SystemChrome, SystemUiOverlayStyle,
};
use jni::objects::{JObject, JValue};
use jni::{Env, jni_sig, jni_str};

use crate::activity::{Activities, device_api_level, with_activity};
use crate::task::TaskDescriptions;

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

/// The system bars around the window of the activity in front, and the app's description in
/// Recents.
pub(crate) struct AndroidChrome {
    activities: Activities,
    descriptions: TaskDescriptions,
}

impl AndroidChrome {
    pub(crate) fn new(activities: Activities, descriptions: TaskDescriptions) -> AndroidChrome {
        AndroidChrome {
            activities,
            descriptions,
        }
    }
}

impl SystemChrome for AndroidChrome {
    fn set_overlay_style(&self, style: &SystemUiOverlayStyle) {
        let style = *style;
        on_the_activity_in_front(
            &self.activities,
            "a style for the system bars",
            move |env, activity| {
                let window = window_of(env, activity)?;
                paint_the_bars(env, &window, &style)?;
                light_or_dark_icons(env, &window, &style)
            },
        );
    }

    /// Every task's description in Recents, under what each view's window was given of its own
    /// (`task.rs`).
    fn set_application_switcher_description(&self, description: &ApplicationSwitcherDescription) {
        self.descriptions.describe_app(description);
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

/// Runs `work` on the Java main thread with the activity in front in hand, and says in the log
/// whatever it turned out the system would not take; nothing while the app has no activity.
fn on_the_activity_in_front(
    activities: &Activities,
    what: &'static str,
    work: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> jni::errors::Result<()> + Send + 'static,
) {
    let Some(app) = activities.front() else {
        return;
    };
    let posted = app.clone();
    posted.run_on_java_main_thread(Box::new(move || {
        if let Err(error) = with_activity(&app, work) {
            crate::log::warn_to_log!("the system would not take {what}: {error:?}");
        }
    }));
}
