//! A view's task, Android's window: the document task a view the app asks for starts in, what a
//! view reads back from the intent its activity started with, how Recents describes the task, and
//! the task's end.
//!
//! A requested view starts Inset's document activity, `rs.inset.InsetDocumentActivity`, in a task
//! of its own (`FLAG_ACTIVITY_NEW_DOCUMENT`, with `FLAG_ACTIVITY_MULTIPLE_TASK`, so that every
//! request makes a new one). The intent's data is the view's initial data under the scheme
//! `inset-view`, and an extra numbers the request. Android keeps that intent as the task's: a task
//! it brings back by itself, from Recents after the app's process was gone, starts again with the
//! same data, as a view the platform gave that carries it; and an intent with the same data and no
//! `FLAG_ACTIVITY_MULTIPLE_TASK`, a pinned shortcut's, finds the task again (the manifest's
//! `documentLaunchMode="intoExisting"`).
//!
//! The invariant: a view's initial data is its intent's data and nothing else, so whatever starts
//! the document activity with that data opens or finds that view.
//!
//! What Recents shows of a task is its `TaskDescription`: the app's own description, Flutter's
//! application switcher description, under what the view's window was given of its own, which
//! wins ([`TaskDescriptions`]). A window's icon is no resource of the package, so below API 37 it
//! goes through the deprecated constructor that takes a `Bitmap`, and from 37 through the
//! builder's `setIcon(Icon)`.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use android_activity::AndroidApp;
use inset_embedder::{
    ApplicationSwitcherDescription, IconImage, RequestId, ViewError, ViewId, ViewRequest,
};
use jni::objects::{JObject, JValue};
use jni::sys::{jint, jlong};
use jni::{Env, jni_sig, jni_str};

use crate::activity::{
    Activities, clear_exception, device_api_level, with_activity, with_application,
};
use crate::bitmap::fitted_bitmap;
use crate::file_dialogs::string;
use crate::log::warn_to_log;

/// Inset's activity for a view of its own.
const DOCUMENT_ACTIVITY: &str = "rs.inset.InsetDocumentActivity";
/// The scheme of a document intent's data, whose scheme-specific part is the view's initial data.
const SCHEME: &str = "inset-view";
/// The extra of a document intent numbering the request it answers.
const EXTRA_REQUEST: &str = "rs.inset.view.REQUEST";
/// `Intent.FLAG_ACTIVITY_NEW_TASK`, which starting an activity from the application needs.
const NEW_TASK: jint = 0x1000_0000;
/// `Intent.FLAG_ACTIVITY_NEW_DOCUMENT`.
const NEW_DOCUMENT: jint = 0x0008_0000;
/// `Intent.FLAG_ACTIVITY_MULTIPLE_TASK`.
const MULTIPLE_TASK: jint = 0x0800_0000;
/// The first release whose task description takes an `Icon`, through its builder.
const TASK_ICON: i32 = 37;

/// What an activity was started with, for the view it is.
#[derive(Debug, Default)]
pub(crate) struct Launch {
    /// The request the intent numbers, from this run of the app or an earlier one.
    request: Option<RequestId>,
    pub(crate) initial_data: Option<String>,
}

/// What the activity `app` was started with: nothing for an intent that is not a document's.
pub(crate) fn launch_of(app: &AndroidApp) -> Launch {
    with_activity(app, |env, activity| {
        let intent = env
            .call_method(
                activity,
                jni_str!("getIntent"),
                jni_sig!("()Landroid/content/Intent;"),
                &[],
            )?
            .l()?;
        if intent.is_null() {
            return Ok(Launch::default());
        }
        Ok(Launch {
            request: request_of(env, &intent)?,
            initial_data: initial_data_of(env, &intent)?,
        })
    })
    .unwrap_or_else(|error| {
        clear_exception();
        warn_to_log!("inset: what the activity was started with could not be read: {error}");
        Launch::default()
    })
}

fn request_of(env: &mut Env<'_>, intent: &JObject<'_>) -> jni::errors::Result<Option<RequestId>> {
    let key = env.new_string(EXTRA_REQUEST)?;
    let number = env
        .call_method(
            intent,
            jni_str!("getLongExtra"),
            jni_sig!("(Ljava/lang/String;J)J"),
            &[JValue::Object(&key), JValue::Long(-1)],
        )?
        .j()?;
    Ok(u64::try_from(number).ok().map(RequestId))
}

/// The scheme-specific part of an `inset-view` intent's data.
fn initial_data_of(env: &mut Env<'_>, intent: &JObject<'_>) -> jni::errors::Result<Option<String>> {
    let data = env
        .call_method(
            intent,
            jni_str!("getData"),
            jni_sig!("()Landroid/net/Uri;"),
            &[],
        )?
        .l()?;
    if data.is_null() {
        return Ok(None);
    }
    let scheme = env
        .call_method(
            &data,
            jni_str!("getScheme"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?
        .l()?;
    if string(env, scheme)?.as_deref() != Some(SCHEME) {
        return Ok(None);
    }
    let part = env
        .call_method(
            &data,
            jni_str!("getSchemeSpecificPart"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?
        .l()?;
    string(env, part)
}

/// The intent that starts Inset's document activity with `initial_data`, which finds the task
/// that has that data, or starts one.
pub(crate) fn document_intent<'local>(
    env: &mut Env<'local>,
    context: &JObject<'_>,
    initial_data: Option<&str>,
) -> jni::errors::Result<JObject<'local>> {
    let intent = env.new_object(jni_str!("android/content/Intent"), jni_sig!("()V"), &[])?;
    let activity = env.new_string(DOCUMENT_ACTIVITY)?;
    env.call_method(
        &intent,
        jni_str!("setClassName"),
        jni_sig!("(Landroid/content/Context;Ljava/lang/String;)Landroid/content/Intent;"),
        &[JValue::Object(context), JValue::Object(&activity)],
    )?;
    if let Some(initial_data) = initial_data {
        let scheme = env.new_string(SCHEME)?;
        let part = env.new_string(initial_data)?;
        let data = env
            .call_static_method(
                jni_str!("android/net/Uri"),
                jni_str!("fromParts"),
                jni_sig!(
                    "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Landroid/net/Uri;"
                ),
                &[
                    JValue::Object(&scheme),
                    JValue::Object(&part),
                    JValue::Object(&JObject::null()),
                ],
            )?
            .l()?;
        env.call_method(
            &intent,
            jni_str!("setData"),
            jni_sig!("(Landroid/net/Uri;)Landroid/content/Intent;"),
            &[JValue::Object(&data)],
        )?;
    }
    Ok(intent)
}

/// The views the app asked for that have not arrived, each with the title its window was asked
/// for with.
pub(crate) struct ViewRequests {
    /// The number the next request is given.
    next: Cell<u64>,
    waiting: RefCell<BTreeMap<RequestId, String>>,
}

impl ViewRequests {
    /// Requests numbered from the time the app started, in nanoseconds since the epoch.
    pub(crate) fn new() -> ViewRequests {
        let started = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos() as u64);
        ViewRequests {
            next: Cell::new(started),
            waiting: RefCell::default(),
        }
    }

    /// Starts a document task for `request`, from the application; answers the request's number.
    pub(crate) fn request(&self, request: ViewRequest) -> Result<RequestId, ViewError> {
        let id = RequestId(self.next.get());
        self.next.set(id.0 + 1);
        let started = with_application(|env, application| {
            let intent = document_intent(env, application, request.initial_data.as_deref())?;
            let key = env.new_string(EXTRA_REQUEST)?;
            env.call_method(
                &intent,
                jni_str!("putExtra"),
                jni_sig!("(Ljava/lang/String;J)Landroid/content/Intent;"),
                &[JValue::Object(&key), JValue::Long(id.0 as jlong)],
            )?;
            env.call_method(
                &intent,
                jni_str!("addFlags"),
                jni_sig!("(I)Landroid/content/Intent;"),
                &[JValue::Int(NEW_TASK | NEW_DOCUMENT | MULTIPLE_TASK)],
            )?;
            env.call_method(
                application,
                jni_str!("startActivity"),
                jni_sig!("(Landroid/content/Intent;)V"),
                &[JValue::Object(&intent)],
            )
            .map(drop)
        });
        if let Err(error) = started {
            clear_exception();
            return Err(ViewError::Failed(format!("no task started: {error}")));
        }
        self.waiting.borrow_mut().insert(id, request.config.title);
        Ok(id)
    }

    /// The request of this run a view whose activity started with `launch` answers, and the
    /// title its window was asked for with; `None` for a view the platform gave.
    pub(crate) fn answered_by(&self, launch: &Launch) -> Option<(RequestId, String)> {
        let id = launch.request?;
        let title = self.waiting.borrow_mut().remove(&id)?;
        Some((id, title))
    }
}

/// How Recents describes each view's task.
#[derive(Clone)]
pub(crate) struct TaskDescriptions {
    activities: Activities,
    described: Rc<RefCell<Described>>,
}

#[derive(Default)]
struct Described {
    /// The app's own description, which every task shows where its window was given nothing.
    app: ApplicationSwitcherDescription,
    /// What each view's window was given of its own.
    windows: BTreeMap<ViewId, OwnDescription>,
}

/// What a window was given of its own.
#[derive(Clone, Default)]
struct OwnDescription {
    title: Option<String>,
    icon: Option<IconImage>,
}

/// One task's description, as it is handed to the Java main thread.
struct Description {
    label: Option<String>,
    icon: Option<IconImage>,
    /// Opaque, as Android requires a task's colour to be.
    color: Option<u32>,
}

impl TaskDescriptions {
    pub(crate) fn new(activities: Activities) -> TaskDescriptions {
        TaskDescriptions {
            activities,
            described: Rc::default(),
        }
    }

    /// `view`'s task, from now on, described as the app is until its window is given more.
    pub(crate) fn add(&self, view: ViewId) {
        self.described
            .borrow_mut()
            .windows
            .insert(view, OwnDescription::default());
        if self.described.borrow().app != ApplicationSwitcherDescription::default() {
            self.describe(view);
        }
    }

    /// `view`'s task, gone.
    pub(crate) fn forget(&self, view: ViewId) {
        self.described.borrow_mut().windows.remove(&view);
    }

    /// The app's own description, Flutter's application switcher description: every task's,
    /// under what its window was given of its own.
    pub(crate) fn describe_app(&self, description: &ApplicationSwitcherDescription) {
        self.described.borrow_mut().app = description.clone();
        let views: Vec<ViewId> = self.described.borrow().windows.keys().copied().collect();
        for view in views {
            self.describe(view);
        }
    }

    /// `view`'s window's own title, which its card in Recents shows.
    pub(crate) fn set_title(&self, view: ViewId, title: &str) {
        self.change(view, |own| own.title = Some(title.to_owned()));
    }

    /// `view`'s window's own icon, which its card in Recents shows; `None` gives the task the
    /// app's icon again.
    pub(crate) fn set_icon(&self, view: ViewId, icon: Option<IconImage>) {
        self.change(view, |own| own.icon = icon);
    }

    fn change(&self, view: ViewId, change: impl FnOnce(&mut OwnDescription)) {
        let changed = match self.described.borrow_mut().windows.get_mut(&view) {
            Some(own) => {
                change(own);
                true
            }
            None => false,
        };
        if changed {
            self.describe(view);
        }
    }

    /// Hands `view`'s task its description, on the Java main thread.
    fn describe(&self, view: ViewId) {
        let Some(activity) = self.activities.of(view) else {
            return;
        };
        let description = {
            let described = self.described.borrow();
            let own = described.windows.get(&view).cloned().unwrap_or_default();
            Description {
                label: own.title.or_else(|| described.app.label.clone()),
                icon: own.icon,
                color: described
                    .app
                    .primary_color
                    .filter(|color| color >> 24 == 0xFF),
            }
        };
        let posted = activity.clone();
        posted.run_on_java_main_thread(Box::new(move || {
            let described = with_activity(&activity, |env, activity| {
                let task = task_description(env, activity, &description)?;
                env.call_method(
                    activity,
                    jni_str!("setTaskDescription"),
                    jni_sig!("(Landroid/app/ActivityManager$TaskDescription;)V"),
                    &[JValue::Object(&task)],
                )
                .map(drop)
            });
            if let Err(error) = described {
                clear_exception();
                warn_to_log!("inset: the task would not take its description: {error:?}");
            }
        }));
    }
}

/// `description` as Android's `TaskDescription`: through the builder, which takes an `Icon`, from
/// API 37; through the constructor that takes a `Bitmap` before.
fn task_description<'local>(
    env: &mut Env<'local>,
    activity: &JObject<'_>,
    description: &Description,
) -> jni::errors::Result<JObject<'local>> {
    let label = match &description.label {
        Some(label) => JObject::from(env.new_string(label)?),
        None => JObject::null(),
    };
    let icon = match &description.icon {
        Some(icon) => {
            let largest = launcher_icon_size(env, activity)?;
            fitted_bitmap(env, icon, largest)?
        }
        None => JObject::null(),
    };
    let color = description.color.unwrap_or(0) as jint;
    if device_api_level() < TASK_ICON {
        return env.new_object(
            jni_str!("android/app/ActivityManager$TaskDescription"),
            jni_sig!("(Ljava/lang/String;Landroid/graphics/Bitmap;I)V"),
            &[
                JValue::Object(&label),
                JValue::Object(&icon),
                JValue::Int(color),
            ],
        );
    }
    let builder = env.new_object(
        jni_str!("android/app/ActivityManager$TaskDescription$Builder"),
        jni_sig!("()V"),
        &[],
    )?;
    env.call_method(
        &builder,
        jni_str!("setLabel"),
        jni_sig!("(Ljava/lang/String;)Landroid/app/ActivityManager$TaskDescription$Builder;"),
        &[JValue::Object(&label)],
    )?;
    env.call_method(
        &builder,
        jni_str!("setPrimaryColor"),
        jni_sig!("(I)Landroid/app/ActivityManager$TaskDescription$Builder;"),
        &[JValue::Int(color)],
    )?;
    if !icon.is_null() {
        let icon = env
            .call_static_method(
                jni_str!("android/graphics/drawable/Icon"),
                jni_str!("createWithBitmap"),
                jni_sig!("(Landroid/graphics/Bitmap;)Landroid/graphics/drawable/Icon;"),
                &[JValue::Object(&icon)],
            )?
            .l()?;
        env.call_method(
            &builder,
            jni_str!("setIcon"),
            jni_sig!(
                "(Landroid/graphics/drawable/Icon;)Landroid/app/ActivityManager$TaskDescription$Builder;"
            ),
            &[JValue::Object(&icon)],
        )?;
    }
    env.call_method(
        &builder,
        jni_str!("build"),
        jni_sig!("()Landroid/app/ActivityManager$TaskDescription;"),
        &[],
    )?
    .l()
}

/// The side of the launcher's large icons, the most Recents shows of a task's icon.
fn launcher_icon_size(env: &mut Env<'_>, activity: &JObject<'_>) -> jni::errors::Result<i32> {
    let service = env.new_string("activity")?;
    let manager = env
        .call_method(
            activity,
            jni_str!("getSystemService"),
            jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
            &[JValue::Object(&service)],
        )?
        .l()?;
    env.call_method(
        &manager,
        jni_str!("getLauncherLargeIconSize"),
        jni_sig!("()I"),
        &[],
    )?
    .i()
}

/// Finishes the activity `app`, as Android does for one that does not handle back itself; its
/// task stays in Recents.
pub(crate) fn finish(app: &AndroidApp) {
    on_the_main_thread(app, "finish", |env, activity| {
        env.call_method(activity, jni_str!("finish"), jni_sig!("()V"), &[])
            .map(drop)
    });
}

/// Ends `app`'s task, its card in Recents with it.
pub(crate) fn finish_and_remove(app: &AndroidApp) {
    on_the_main_thread(app, "end its task", |env, activity| {
        env.call_method(
            activity,
            jni_str!("finishAndRemoveTask"),
            jni_sig!("()V"),
            &[],
        )
        .map(drop)
    });
}

/// Runs `work` with the activity `app` on the Java main thread, where an activity is finished, and
/// says in the log what the activity would not do.
fn on_the_main_thread(
    app: &AndroidApp,
    what: &'static str,
    work: impl FnOnce(&mut Env<'_>, &JObject<'_>) -> jni::errors::Result<()> + Send + 'static,
) {
    let activity = app.clone();
    app.run_on_java_main_thread(Box::new(move || {
        if let Err(error) = with_activity(&activity, work) {
            clear_exception();
            warn_to_log!("inset: the activity would not {what}: {error:?}");
        }
    }));
}
