//! The app's shortcuts, through the framework's `ShortcutManager` (`WaftShortcuts.java`), reached
//! as [`AndroidPlatform::shortcuts`](crate::AndroidPlatform::shortcuts): what only Android has, an
//! icon of the app's own on the home screen, pinned, and the entries of the app icon's menu,
//! dynamic, both by one id, so a dynamic shortcut the user drags onto the home screen is the same
//! shortcut pinned. Android lets no app remove a pinned shortcut: the app updates it, or disables
//! it, greyed out with a message, and the user removes it.
//!
//! A shortcut opens a view in a task of its own, its intent the document activity's with the view's
//! initial data (`WaftDocumentActivity.shortcutIntent`), so Android finds the task that has that
//! data, or starts one whose view the platform gives the app; or it brings Waft's activity forward
//! with an action of the app's, a tap the app hears through its handler.
//!
//! The calls Android marks for a worker thread, which may take long, run on a thread of their own
//! and answer a future; the others are made where they are called. The invariant a later editor
//! will break: a tap is handed to the app's handler only from the app's loop, outside every call
//! into the app, and a tap that comes before the app set a handler, as the one that started the
//! app, waits for the first handler.

use std::cell::RefCell;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use jni::objects::{JClass, JObject, JObjectArray, JValue};
use jni::sys::jint;
use jni::{Env, jni_sig, jni_str};
use waft_embedder::IconImage;

use crate::answer::answer;
use crate::application::{Application, package_class};
use crate::file_dialogs::string;
use crate::log::warn_to_log;
use crate::loop_waker::wake_app_loop;
use crate::view_requests::exception_text;

/// Waft's helpers for the app's shortcuts (`WaftShortcuts.java`).
const WAFT_SHORTCUTS: &str = "rs.waft.WaftShortcuts";
/// `ShortcutManager.FLAG_MATCH_DYNAMIC`.
const MATCH_DYNAMIC: jint = 2;
/// `ShortcutManager.FLAG_MATCH_PINNED`.
const MATCH_PINNED: jint = 4;

/// A shortcut, as Android's `ShortcutInfo`.
#[derive(Clone, Debug, PartialEq)]
pub struct ShortcutInfo {
    /// The app's own name for it, by which it is updated, enabled and disabled.
    pub id: String,
    pub short_label: String,
    /// What a launcher shows where it has room; the short label where none.
    pub long_label: Option<String>,
    /// What a tap on it says while it is disabled; the system's own words where none.
    pub disabled_message: Option<String>,
    pub icon: Option<ShortcutIcon>,
    pub target: ShortcutTarget,
    /// Its place on the app icon's menu, 0 first; a dynamic shortcut's only.
    pub rank: Option<u32>,
}

/// A shortcut's icon, which the host scales down to the most the system keeps.
#[derive(Clone, Debug, PartialEq)]
pub enum ShortcutIcon {
    /// `Icon.createWithAdaptiveBitmap`: a full-bleed layer 108 units a side, of which the
    /// launcher shows the middle 72, cut to its own shape, and the rest only as the icon moves.
    Adaptive(IconImage),
    /// `Icon.createWithBitmap`: shown as it is, which a launcher may set on a plate of its own.
    Bitmap(IconImage),
}

/// What a tap on a shortcut opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShortcutTarget {
    /// The view carrying `initial_data`, in its task: the task that has it comes forward, or a new
    /// task's view arrives as one the platform gave, carrying it.
    View { initial_data: String },
    /// Waft's activity, in the app's own task, which comes forward, and the app hears `action`
    /// through [`Shortcuts::set_activation_handler`].
    App { action: String },
}

/// A shortcut the system keeps of the app's, read back; Android hands no icon back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublishedShortcut {
    pub id: String,
    pub short_label: Option<String>,
    pub long_label: Option<String>,
    /// What a tap opens; none where the intent is not one Waft wrote.
    pub target: Option<ShortcutTarget>,
    pub pinned: bool,
    pub dynamic: bool,
    pub enabled: bool,
    /// Why it is disabled; none while it is not.
    pub disabled_reason: Option<DisabledReason>,
}

/// Why a shortcut is disabled (`ShortcutInfo.DISABLED_REASON_*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisabledReason {
    /// The app disabled it.
    ByApp,
    /// An update of the app took away what it opened.
    AppChanged,
    /// It was restored to a device whose app is older than the one it was backed up from.
    VersionLower,
    /// It was restored for an app that allows no backup.
    BackupNotSupported,
    /// It was restored for an app signed by another key.
    SignatureMismatch,
    OtherRestoreIssue,
    Unknown,
}

/// Which of the app's shortcuts to read back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShortcutMatch {
    pub pinned: bool,
    pub dynamic: bool,
}

/// Why a call on the shortcuts did not do what it asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShortcutError {
    /// The launcher pins nothing on request.
    Unsupported,
    /// Android limits how often an app in the background changes its shortcuts; the limit goes
    /// once the app is in front.
    RateLimited,
    Failed(String),
}

impl fmt::Display for ShortcutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShortcutError::Unsupported => f.write_str("the launcher pins no shortcuts on request"),
            ShortcutError::RateLimited => {
                f.write_str("Android limits how often the app changes its shortcuts now")
            }
            ShortcutError::Failed(reason) => write!(f, "the shortcuts did not change: {reason}"),
        }
    }
}

impl std::error::Error for ShortcutError {}

/// What a call on the shortcuts that runs off the app's thread answers.
pub type ShortcutFuture<T> = Pin<Box<dyn Future<Output = Result<T, ShortcutError>>>>;

/// A tap on a shortcut whose target is [`ShortcutTarget::App`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShortcutActivation {
    /// The shortcut's id.
    pub id: String,
    /// The app's action it carries.
    pub action: String,
}

/// What hears the taps on the app's shortcuts that open its own activity, on the app's thread and
/// outside every call into the app, so it may post to the app.
pub type ShortcutActivationHandler = Rc<dyn Fn(ShortcutActivation)>;

/// The app's shortcuts: a handle on Android's, which every clone shares.
#[derive(Clone)]
pub struct Shortcuts {
    shared: Rc<Shared>,
}

struct Shared {
    application: Application,
    handler: RefCell<Option<ShortcutActivationHandler>>,
    /// The taps that came while no handler was set, in the order they came.
    waiting: RefCell<Vec<ShortcutActivation>>,
}

impl Shortcuts {
    pub(crate) fn new(application: Application) -> Shortcuts {
        Shortcuts {
            shared: Rc::new(Shared {
                application,
                handler: RefCell::new(None),
                waiting: RefCell::new(Vec::new()),
            }),
        }
    }

    /// Whether the default launcher pins a shortcut on request; it changes with the launcher.
    pub fn is_request_pin_supported(&self) -> bool {
        self.ask("whether the launcher pins", false, |env, class, context| {
            env.call_static_method(
                class,
                jni_str!("pins"),
                jni_sig!("(Landroid/content/Context;)Z"),
                &[JValue::Object(context)],
            )?
            .z()
        })
    }

    /// The side, in pixels, of the visible part of a shortcut's icon, `getIconMaxWidth`.
    pub fn icon_max_size(&self) -> u32 {
        let size = self.ask("the icons' size", 0, |env, class, context| {
            env.call_static_method(
                class,
                jni_str!("iconMaxSize"),
                jni_sig!("(Landroid/content/Context;)I"),
                &[JValue::Object(context)],
            )?
            .i()
        });
        u32::try_from(size).unwrap_or(0)
    }

    /// How many dynamic shortcuts the app icon's menu keeps, `getMaxShortcutCountPerActivity`.
    pub fn max_per_activity(&self) -> u32 {
        let count = self.ask("the most dynamic shortcuts", 0, |env, class, context| {
            env.call_static_method(
                class,
                jni_str!("maxPerActivity"),
                jni_sig!("(Landroid/content/Context;)I"),
                &[JValue::Object(context)],
            )?
            .i()
        });
        u32::try_from(count).unwrap_or(0)
    }

    /// Whether Android limits how often the app changes its shortcuts now.
    pub fn is_rate_limiting_active(&self) -> bool {
        self.ask(
            "whether the app is limited",
            false,
            |env, class, context| {
                env.call_static_method(
                    class,
                    jni_str!("isRateLimitingActive"),
                    jni_sig!("(Landroid/content/Context;)Z"),
                    &[JValue::Object(context)],
                )?
                .z()
            },
        )
    }

    /// Asks the launcher to pin `shortcut`, which asks the user: answered once the launcher has
    /// the request, never with the user's answer, which Android tells no app. It fails where
    /// nothing of the app is in front, and where a disabled shortcut has the same id.
    pub fn request_pin(&self, shortcut: &ShortcutInfo) -> ShortcutFuture<()> {
        let shortcut = shortcut.clone();
        self.off_the_thread("waft-shortcut-pin", move |env, class, context| {
            let info = info_object(env, class, context, &shortcut)?;
            let asked = env
                .call_static_method(
                    class,
                    jni_str!("requestPin"),
                    jni_sig!("(Landroid/content/Context;Landroid/content/pm/ShortcutInfo;)Z"),
                    &[JValue::Object(context), JValue::Object(&info)],
                )?
                .z()?;
            Ok(if asked {
                Ok(())
            } else {
                Err(ShortcutError::Unsupported)
            })
        })
    }

    /// Changes `shortcuts`, pinned or dynamic, by their ids, to what each says now.
    pub fn update(&self, shortcuts: &[ShortcutInfo]) -> ShortcutFuture<()> {
        self.change_many("waft-shortcut-update", jni_str!("update"), shortcuts)
    }

    /// Makes `shortcuts` the app icon's dynamic shortcuts, in place of those it had.
    pub fn set_dynamic(&self, shortcuts: &[ShortcutInfo]) -> ShortcutFuture<()> {
        self.change_many("waft-shortcut-set", jni_str!("setDynamic"), shortcuts)
    }

    /// Adds `shortcuts` to the app icon's dynamic shortcuts, or updates those of the same ids;
    /// fails where they would be more than the menu keeps.
    pub fn add_dynamic(&self, shortcuts: &[ShortcutInfo]) -> ShortcutFuture<()> {
        self.change_many("waft-shortcut-add", jni_str!("addDynamic"), shortcuts)
    }

    /// Puts `shortcut` among the app icon's dynamic shortcuts at its rank, or updates the one of
    /// its id, pinned or dynamic; where the menu is full, the one of the lowest rank leaves.
    pub fn push_dynamic(&self, shortcut: &ShortcutInfo) {
        self.tell("a dynamic shortcut pushed", |env, class, context| {
            let info = info_object(env, class, context, shortcut)?;
            env.call_static_method(
                class,
                jni_str!("pushDynamic"),
                jni_sig!("(Landroid/content/Context;Landroid/content/pm/ShortcutInfo;)V"),
                &[JValue::Object(context), JValue::Object(&info)],
            )
            .map(drop)
        });
    }

    /// Takes the dynamic shortcuts of `ids` off the app icon's menu; a pinned one stays.
    pub fn remove_dynamic(&self, ids: &[&str]) {
        self.with_ids("dynamic shortcuts removed", jni_str!("removeDynamic"), ids);
    }

    /// Takes every dynamic shortcut off the app icon's menu.
    pub fn remove_all_dynamic(&self) {
        self.tell("every dynamic shortcut removed", |env, class, context| {
            env.call_static_method(
                class,
                jni_str!("removeAllDynamic"),
                jni_sig!("(Landroid/content/Context;)V"),
                &[JValue::Object(context)],
            )
            .map(drop)
        });
    }

    /// Tells the launcher the user chose the shortcut `id`, or did in the app what it does, which
    /// a launcher ranks shortcuts by.
    pub fn report_used(&self, id: &str) {
        self.tell("a shortcut's use reported", |env, class, context| {
            let id = env.new_string(id)?;
            env.call_static_method(
                class,
                jni_str!("reportUsed"),
                jni_sig!("(Landroid/content/Context;Ljava/lang/String;)V"),
                &[JValue::Object(context), JValue::Object(&id)],
            )
            .map(drop)
        });
    }

    /// Enables the shortcuts of `ids` the app disabled.
    pub fn enable(&self, ids: &[&str]) {
        self.with_ids("shortcuts enabled", jni_str!("enable"), ids);
    }

    /// Disables the shortcuts of `ids`, greyed out on the home screen, where a tap says
    /// `message`, or the system's own words; a dynamic one leaves the app icon's menu.
    pub fn disable(&self, ids: &[&str], message: Option<&str>) {
        if ids.is_empty() {
            return;
        }
        self.tell("shortcuts disabled", |env, class, context| {
            let ids = string_array(env, ids)?;
            let message = match message {
                Some(message) => JObject::from(env.new_string(message)?),
                None => JObject::null(),
            };
            env.call_static_method(
                class,
                jni_str!("disable"),
                jni_sig!("(Landroid/content/Context;[Ljava/lang/String;Ljava/lang/String;)V"),
                &[
                    JValue::Object(context),
                    JValue::Object(&ids),
                    JValue::Object(&message),
                ],
            )
            .map(drop)
        });
    }

    /// The app's shortcuts `which` matches, as the system keeps them.
    pub fn shortcuts(&self, which: ShortcutMatch) -> ShortcutFuture<Vec<PublishedShortcut>> {
        let flags = if which.pinned { MATCH_PINNED } else { 0 }
            | if which.dynamic { MATCH_DYNAMIC } else { 0 };
        self.off_the_thread("waft-shortcut-read", move |env, class, context| {
            let found = env
                .call_static_method(
                    class,
                    jni_str!("shortcuts"),
                    jni_sig!("(Landroid/content/Context;I)[Landroid/content/pm/ShortcutInfo;"),
                    &[JValue::Object(context), JValue::Int(flags)],
                )?
                .l()?;
            let found = env.cast_local::<JObjectArray>(found)?;
            let mut published = Vec::new();
            for index in 0..found.len(env)? {
                let info = found.get_element(env, index)?;
                published.push(published_of(env, class, &info)?);
            }
            Ok(Ok(published))
        })
    }

    /// From now on `handler` hears each tap on a shortcut whose target is
    /// [`ShortcutTarget::App`], from the app's loop; the taps that came before, as the one that
    /// started the app, come to it first. None drops the taps from now on.
    pub fn set_activation_handler(&self, handler: Option<ShortcutActivationHandler>) {
        *self.shared.handler.borrow_mut() = handler;
        wake_app_loop();
    }

    /// Hands `taps`, and those waiting, to the handler, where one is set; they wait otherwise.
    /// Called from the app's loop.
    pub(crate) fn hand_on(&self, taps: Vec<ShortcutActivation>) {
        self.shared.waiting.borrow_mut().extend(taps);
        let Some(handler) = self.shared.handler.borrow().clone() else {
            return;
        };
        let waiting = std::mem::take(&mut *self.shared.waiting.borrow_mut());
        for tap in waiting {
            handler(tap);
        }
    }

    /// Calls `method` with `shortcuts` on a thread of its own, which answers whether Android
    /// took the change.
    fn change_many(
        &self,
        thread: &str,
        method: &'static jni::strings::JNIStr,
        shortcuts: &[ShortcutInfo],
    ) -> ShortcutFuture<()> {
        let shortcuts = shortcuts.to_vec();
        self.off_the_thread(thread, move |env, class, context| {
            let infos = info_array(env, class, context, &shortcuts)?;
            let taken = env
                .call_static_method(
                    class,
                    method,
                    jni_sig!("(Landroid/content/Context;[Landroid/content/pm/ShortcutInfo;)Z"),
                    &[JValue::Object(context), JValue::Object(&infos)],
                )?
                .z()?;
            Ok(if taken {
                Ok(())
            } else {
                Err(ShortcutError::RateLimited)
            })
        })
    }

    /// Calls `method` with `ids`, here.
    fn with_ids(&self, what: &str, method: &'static jni::strings::JNIStr, ids: &[&str]) {
        if ids.is_empty() {
            return;
        }
        self.tell(what, |env, class, context| {
            let ids = string_array(env, ids)?;
            env.call_static_method(
                class,
                method,
                jni_sig!("(Landroid/content/Context;[Ljava/lang/String;)V"),
                &[JValue::Object(context), JValue::Object(&ids)],
            )
            .map(drop)
        });
    }

    /// What `work` answers here, with `WaftShortcuts` and the application; `if_failed`, said in
    /// the log, where it fails.
    fn ask<T>(
        &self,
        what: &str,
        if_failed: T,
        work: impl FnOnce(&mut Env<'_>, &JClass<'_>, &JObject<'_>) -> jni::errors::Result<T>,
    ) -> T {
        let application = &self.shared.application;
        let asked = application.with(|env, context| {
            let done = package_class(env, context, WAFT_SHORTCUTS)
                .and_then(|class| work(env, &class, context));
            Ok(done.map_err(|error| exception_text(env, &error)))
        });
        match asked {
            Ok(Ok(answer)) => answer,
            Ok(Err(reason)) => {
                warn_to_log!("waft: {what}: {reason}");
                if_failed
            }
            Err(error) => {
                application.clear_exception();
                warn_to_log!("waft: {what}: {error}");
                if_failed
            }
        }
    }

    /// Does `work` here, saying in the log where it fails.
    fn tell(
        &self,
        what: &str,
        work: impl FnOnce(&mut Env<'_>, &JClass<'_>, &JObject<'_>) -> jni::errors::Result<()>,
    ) {
        self.ask(what, (), work);
    }

    /// Runs `work` on a thread named `thread`, attached to the app's VM, and answers what it
    /// answers; a call into Java that fails is [`ShortcutError::Failed`] with its exception.
    fn off_the_thread<T: Send + 'static>(
        &self,
        thread: &str,
        work: impl FnOnce(
            &mut Env<'_>,
            &JClass<'_>,
            &JObject<'_>,
        ) -> jni::errors::Result<Result<T, ShortcutError>>
        + Send
        + 'static,
    ) -> ShortcutFuture<T> {
        // Answered on the thread, or, dropped with it where no thread could be made, answered so.
        let (given, answered) = answer(Err(ShortcutError::Failed(
            "the call was not handed to Android".to_owned(),
        )));
        let application = self.shared.application.clone();
        let started = std::thread::Builder::new()
            .name(thread.to_owned())
            .spawn(move || {
                let done = application.with(|env, context| {
                    let done = package_class(env, context, WAFT_SHORTCUTS)
                        .and_then(|class| work(env, &class, context));
                    Ok(done.unwrap_or_else(|error| {
                        Err(ShortcutError::Failed(exception_text(env, &error)))
                    }))
                });
                given.give(done.unwrap_or_else(|error| {
                    application.clear_exception();
                    Err(ShortcutError::Failed(error.to_string()))
                }));
            });
        if let Err(error) = started {
            warn_to_log!("waft: no thread for the shortcuts: {error}");
        }
        Box::pin(answered)
    }
}

/// The tap `intent` is, on a shortcut whose target is the app's; none for any other intent.
pub(crate) fn tap_of(
    env: &mut Env<'_>,
    context: &JObject<'_>,
    intent: &JObject<'_>,
) -> jni::errors::Result<Option<ShortcutActivation>> {
    let class = package_class(env, context, WAFT_SHORTCUTS)?;
    let tap = env
        .call_static_method(
            &class,
            jni_str!("tapOf"),
            jni_sig!("(Landroid/content/Intent;)[Ljava/lang/String;"),
            &[JValue::Object(intent)],
        )?
        .l()?;
    if tap.is_null() {
        return Ok(None);
    }
    let tap = env.cast_local::<JObjectArray>(tap)?;
    let id = tap.get_element(env, 0)?;
    let action = tap.get_element(env, 1)?;
    let (Some(id), Some(action)) = (string(env, id)?, string(env, action)?) else {
        return Ok(None);
    };
    Ok(Some(ShortcutActivation { id, action }))
}

/// `shortcut` as Android's `ShortcutInfo`.
fn info_object<'local>(
    env: &mut Env<'local>,
    class: &JClass<'_>,
    context: &JObject<'_>,
    shortcut: &ShortcutInfo,
) -> jni::errors::Result<JObject<'local>> {
    let id = env.new_string(&shortcut.id)?;
    let short_label = env.new_string(&shortcut.short_label)?;
    let long_label = optional_string(env, shortcut.long_label.as_deref())?;
    let disabled_message = optional_string(env, shortcut.disabled_message.as_deref())?;
    let (image, adaptive) = match &shortcut.icon {
        Some(ShortcutIcon::Adaptive(image)) => (Some(image), true),
        Some(ShortcutIcon::Bitmap(image)) => (Some(image), false),
        None => (None, false),
    };
    let pixels = match image {
        Some(image) => JObject::from(crate::icon_pixels::int_array(env, image)?),
        None => JObject::null(),
    };
    let [width, height] = image.map_or([0, 0], |image| [image.width, image.height]);
    let (opens_view, data) = match &shortcut.target {
        ShortcutTarget::View { initial_data } => (true, initial_data),
        ShortcutTarget::App { action } => (false, action),
    };
    let data = env.new_string(data)?;
    let rank = shortcut
        .rank
        .map_or(-1, |rank| jint::try_from(rank).unwrap_or(jint::MAX));
    env.call_static_method(
        class,
        jni_str!("info"),
        jni_sig!(
            "(Landroid/content/Context;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;[IIIZZLjava/lang/String;I)Landroid/content/pm/ShortcutInfo;"
        ),
        &[
            JValue::Object(context),
            JValue::Object(&id),
            JValue::Object(&short_label),
            JValue::Object(&long_label),
            JValue::Object(&disabled_message),
            JValue::Object(&pixels),
            JValue::Int(width as jint),
            JValue::Int(height as jint),
            JValue::Bool(adaptive),
            JValue::Bool(opens_view),
            JValue::Object(&data),
            JValue::Int(rank),
        ],
    )?
    .l()
}

/// `shortcuts` as a Java array of Android's `ShortcutInfo`s.
fn info_array<'local>(
    env: &mut Env<'local>,
    class: &JClass<'_>,
    context: &JObject<'_>,
    shortcuts: &[ShortcutInfo],
) -> jni::errors::Result<JObject<'local>> {
    let info_class = env.find_class(jni_str!("android/content/pm/ShortcutInfo"))?;
    let array = env.new_object_array(shortcuts.len() as i32, &info_class, JObject::null())?;
    for (index, shortcut) in shortcuts.iter().enumerate() {
        let info = info_object(env, class, context, shortcut)?;
        array.set_element(env, index, &info)?;
    }
    Ok(array.into())
}

/// The shortcut `info` is, as the system keeps it.
fn published_of(
    env: &mut Env<'_>,
    class: &JClass<'_>,
    info: &JObject<'_>,
) -> jni::errors::Result<PublishedShortcut> {
    let id = env
        .call_method(
            info,
            jni_str!("getId"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?
        .l()?;
    let id = string(env, id)?.unwrap_or_default();
    let short_label = info_string(env, class, info, jni_str!("shortLabel"))?;
    let long_label = info_string(env, class, info, jni_str!("longLabel"))?;
    let target = match info_string(env, class, info, jni_str!("viewData"))? {
        Some(initial_data) => Some(ShortcutTarget::View { initial_data }),
        None => info_string(env, class, info, jni_str!("tapAction"))?
            .map(|action| ShortcutTarget::App { action }),
    };
    let flag = |env: &mut Env<'_>, name: &'static jni::strings::JNIStr| {
        env.call_method(info, name, jni_sig!("()Z"), &[])?.z()
    };
    let pinned = flag(env, jni_str!("isPinned"))?;
    let dynamic = flag(env, jni_str!("isDynamic"))?;
    let enabled = flag(env, jni_str!("isEnabled"))?;
    let reason = env
        .call_method(info, jni_str!("getDisabledReason"), jni_sig!("()I"), &[])?
        .i()?;
    Ok(PublishedShortcut {
        id,
        short_label,
        long_label,
        target,
        pinned,
        dynamic,
        enabled,
        disabled_reason: disabled_reason_of(reason),
    })
}

/// What `WaftShortcuts.method(info)` answers, as text.
fn info_string(
    env: &mut Env<'_>,
    class: &JClass<'_>,
    info: &JObject<'_>,
    method: &'static jni::strings::JNIStr,
) -> jni::errors::Result<Option<String>> {
    let text = env
        .call_static_method(
            class,
            method,
            jni_sig!("(Landroid/content/pm/ShortcutInfo;)Ljava/lang/String;"),
            &[JValue::Object(info)],
        )?
        .l()?;
    string(env, text)
}

/// `ShortcutInfo.getDisabledReason`'s answer; none for `DISABLED_REASON_NOT_DISABLED`.
fn disabled_reason_of(reason: jint) -> Option<DisabledReason> {
    Some(match reason {
        0 => return None,
        1 => DisabledReason::ByApp,
        2 => DisabledReason::AppChanged,
        100 => DisabledReason::VersionLower,
        101 => DisabledReason::BackupNotSupported,
        102 => DisabledReason::SignatureMismatch,
        103 => DisabledReason::OtherRestoreIssue,
        _ => DisabledReason::Unknown,
    })
}

fn optional_string<'local>(
    env: &mut Env<'local>,
    text: Option<&str>,
) -> jni::errors::Result<JObject<'local>> {
    Ok(match text {
        Some(text) => JObject::from(env.new_string(text)?),
        None => JObject::null(),
    })
}

/// `texts` as a Java `String[]`.
fn string_array<'local>(
    env: &mut Env<'local>,
    texts: &[&str],
) -> jni::errors::Result<JObject<'local>> {
    let string_class = env.find_class(jni_str!("java/lang/String"))?;
    let array = env.new_object_array(texts.len() as i32, &string_class, JObject::null())?;
    for (index, text) in texts.iter().enumerate() {
        let text = env.new_string(text)?;
        array.set_element(env, index, &text)?;
    }
    Ok(array.into())
}
