//! The window around a view on Android: its activity's task, the card in Recents. Every activity
//! is a view in a task of its own, the launcher's and each document activity a request started
//! (`view_requests.rs`), so every view has one.
//!
//! What a window is on a phone, call by call:
//!
//! - the title and the icon are the card's, the task's `TaskDescription` (`WaftTask.java`): the
//!   window's own title where it has one, the app's description in the switcher otherwise
//!   (`SystemChrome::set_application_switcher_description`), and the app's primary colour;
//!   Recents draws the icon and reads the label out to accessibility, but draws no label;
//! - closing finishes the activity and removes its task (`finishAndRemoveTask`), and the view goes
//!   when Android destroys the activity finishing;
//! - minimizing sends the task to the background (`moveTaskToBack`), and restoring it brings it
//!   forward (`ActivityManager.AppTask.moveToFront`), as focusing the view does;
//! - minimized is having no window from the system, as in the background, and the host reports
//!   each change (`EmbedderClient::window_changed`); a phone tells the app of no window covering
//!   another, so none is occluded, and none is maximized;
//! - the system sizes and places every window, so the frame is the view's size at the origin, and
//!   moving, sizing, showing, hiding, maximizing and dragging do nothing; there are no size limits.
//!
//! The invariant a later editor will break: what the task shows is the window's, kept here, and
//! told again to each activity attached to the view ([`TaskWindow::attach`]), since an activity
//! Android makes in place of a destroyed one starts with the manifest's.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

use jni::objects::{JIntArray, JObject, JValue};
use jni::strings::JNIStr;
use jni::{Env, jni_sig, jni_str};
use raw_window_handle::HasWindowHandle;
use waft_embedder::{
    HostWindow, IconImage, RawWindowHandle, Rect, View, ViewRef, WindowConfig, WindowSizeLimits,
};

use crate::activity::{Activity, AttachedActivity};
use crate::application::{Application, package_class};
use crate::chrome::AppDescription;
use crate::log::warn_to_log;
use crate::view::AndroidView;

/// Waft's helpers for a task (`WaftTask.java`).
const WAFT_TASK: &str = "rs.waft.WaftTask";

/// A view's task, as the app manages it.
pub(crate) struct TaskWindow {
    /// The view the task shows, which holds this.
    view: Weak<AndroidView>,
    /// The view's activity, while Android has one for it.
    activity: AttachedActivity,
    application: Application,
    /// What every card starts from: the app's own description in the switcher.
    app_description: AppDescription,
    /// The task's number, as the first activity attached to the view answered it.
    task_id: Cell<Option<i32>>,
    title: RefCell<String>,
    icon: RefCell<Option<IconImage>>,
}

impl TaskWindow {
    /// The task of `view`, whose activity, attached later, is `activity`, titled and iconed as
    /// `config` says where a request asked for it.
    pub(crate) fn new(
        view: Weak<AndroidView>,
        activity: AttachedActivity,
        application: Application,
        app_description: AppDescription,
        config: Option<&WindowConfig>,
    ) -> TaskWindow {
        TaskWindow {
            view,
            activity,
            application,
            app_description,
            task_id: Cell::new(None),
            title: RefCell::new(
                config
                    .map(|config| config.title.clone())
                    .unwrap_or_default(),
            ),
            icon: RefCell::new(config.and_then(|config| config.icon.clone())),
        }
    }

    /// Android made `activity` for the view: the task is the one it is in, and its card shows
    /// what the window has.
    pub(crate) fn attach(&self, activity: &Activity) {
        match activity.task_id() {
            Ok(id) => self.task_id.set(Some(id)),
            Err(error) => {
                activity.clear_exception();
                warn_to_log!("waft: the view's task is not known: {error}");
            }
        }
        self.describe();
    }

    /// The task's number, once an activity was attached to the view.
    pub(crate) fn task_id(&self) -> Option<i32> {
        self.task_id.get()
    }

    /// Tells the task's card what the window and the app have now.
    pub(crate) fn describe(&self) {
        let app = self.app_description.get();
        let title = self.title.borrow().clone();
        let label = Some(title)
            .filter(|title| !title.is_empty())
            .or_else(|| app.as_ref().and_then(|app| app.label.clone()));
        let color = app.and_then(|app| app.primary_color).unwrap_or(0) as i32;
        let icon = self.icon.borrow().clone();
        self.activity
            .on_main_thread("a description of the task", move |env, activity| {
                describe_the_task(env, activity, label.as_deref(), icon.as_ref(), color)
            });
    }

    /// Brings the task forward: where Android allows it, which is while the app has a window in
    /// sight.
    pub(crate) fn move_to_front(&self) {
        self.on_the_task(jni_str!("moveToFront"), "bringing the task forward");
    }

    /// Calls `WaftTask`'s `method` for the task, from the application: `what` it does, for the
    /// log.
    fn on_the_task(&self, method: &'static JNIStr, what: &str) {
        let Some(id) = self.task_id.get() else {
            return;
        };
        let called = self.application.with(|env, context| {
            let class = package_class(env, context, WAFT_TASK)?;
            env.call_static_method(
                &class,
                method,
                jni_sig!("(Landroid/content/Context;I)Z"),
                &[JValue::Object(context), JValue::Int(id)],
            )?
            .z()
        });
        match called {
            Ok(true) => {}
            Ok(false) => warn_to_log!("waft: {what}: the app has no task {id}"),
            Err(error) => {
                self.application.clear_exception();
                warn_to_log!("waft: {what} failed: {error}");
            }
        }
    }

    fn view_of(&self) -> Option<Rc<AndroidView>> {
        self.view.upgrade()
    }
}

/// The numbers of the tasks Android keeps of the app, in Recents or running; none where it would
/// not say.
pub(crate) fn task_ids(application: &Application) -> Option<Vec<i32>> {
    let ids = application.with(|env, context| {
        let class = package_class(env, context, WAFT_TASK)?;
        let ids = env
            .call_static_method(
                &class,
                jni_str!("taskIds"),
                jni_sig!("(Landroid/content/Context;)[I"),
                &[JValue::Object(context)],
            )?
            .l()?;
        let ids = env.cast_local::<JIntArray>(ids)?;
        let mut read = vec![0; ids.len(env)?];
        ids.get_region(env, 0, &mut read)?;
        Ok(read)
    });
    ids.inspect_err(|error| {
        application.clear_exception();
        warn_to_log!("waft: the app's tasks are not known: {error}");
    })
    .ok()
}

/// `WaftTask.describe` for `activity`'s task.
fn describe_the_task(
    env: &mut Env<'_>,
    activity: &JObject<'_>,
    label: Option<&str>,
    icon: Option<&IconImage>,
    color: i32,
) -> jni::errors::Result<()> {
    let class = package_class(env, activity, WAFT_TASK)?;
    let label = match label {
        Some(label) => JObject::from(env.new_string(label)?),
        None => JObject::null(),
    };
    let pixels = match icon {
        Some(icon) => JObject::from(crate::icon_pixels::int_array(env, icon)?),
        None => JObject::null(),
    };
    let [width, height] = icon.map_or([0, 0], |icon| [icon.width, icon.height]);
    env.call_static_method(
        &class,
        jni_str!("describe"),
        jni_sig!("(Landroid/app/Activity;Ljava/lang/String;[IIII)V"),
        &[
            JValue::Object(activity),
            JValue::Object(&label),
            JValue::Object(&pixels),
            JValue::Int(width as i32),
            JValue::Int(height as i32),
            JValue::Int(color),
        ],
    )?;
    Ok(())
}

impl HostWindow for TaskWindow {
    fn view(&self) -> ViewRef {
        self.view_of()
            .expect("a task is asked for its view while the view is") as ViewRef
    }

    /// The view's logical size at the origin: the system places every window on a phone.
    fn frame(&self) -> Rect {
        let Some(view) = self.view_of() else {
            return Rect::from_ltwh(0.0, 0.0, 0.0, 0.0);
        };
        let metrics = view.metrics();
        let [width, height] = metrics.physical_size;
        let ratio = metrics.device_pixel_ratio;
        Rect::from_ltwh(0.0, 0.0, width / ratio, height / ratio)
    }

    fn set_frame(&self, _frame: Rect, _animate: Option<Duration>) {}

    fn set_title(&self, title: &str) {
        *self.title.borrow_mut() = title.to_owned();
        self.describe();
    }

    fn set_icon(&self, icon: Option<&IconImage>) {
        *self.icon.borrow_mut() = icon.cloned();
        self.describe();
    }

    fn show(&self) {}

    fn hide(&self) {}

    /// Whether the system gives the view no window now: the task is in the background, or its
    /// activity is being made again.
    fn is_minimized(&self) -> bool {
        self.view_of().is_some_and(|view| !view.has_window())
    }

    fn is_occluded(&self) -> bool {
        false
    }

    /// Finishes the activity and removes its task; with no activity, while Android makes one
    /// again, the task is removed through the activity manager, and the view goes at the host's
    /// next look at the app's tasks.
    fn close(&self) {
        if self.activity.get().is_none() {
            self.on_the_task(jni_str!("remove"), "removing the task");
            if let Some(view) = self.view_of() {
                view.task_was_removed();
            }
            return;
        }
        self.activity
            .on_main_thread("closing the task", |env, activity| {
                env.call_method(
                    activity,
                    jni_str!("finishAndRemoveTask"),
                    jni_sig!("()V"),
                    &[],
                )
                .map(drop)
            });
    }

    fn size_limits(&self) -> Option<&dyn WindowSizeLimits> {
        None
    }

    fn drag(&self) {}

    fn is_maximized(&self) -> bool {
        false
    }

    fn set_maximized(&self, _maximized: bool) {}

    /// Sends the task to the background, its card staying in Recents, or brings it forward.
    fn set_minimized(&self, minimized: bool) {
        if !minimized {
            self.move_to_front();
            return;
        }
        self.activity
            .on_main_thread("sending the task to the background", |env, activity| {
                env.call_method(
                    activity,
                    jni_str!("moveTaskToBack"),
                    jni_sig!("(Z)Z"),
                    &[JValue::Bool(true)],
                )
                .map(drop)
            });
    }

    /// The `ANativeWindow`, while the system gives the view one.
    fn native_handle(&self) -> Option<RawWindowHandle> {
        let view = self.view_of()?;
        let window = view.native_window()?;
        let handle = window.window_handle().ok()?.as_raw();
        Some(handle)
    }
}
