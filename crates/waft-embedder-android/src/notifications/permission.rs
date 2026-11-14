//! The user's permission to notify. From Android 13, for an app that targets it, posting is the
//! runtime permission `POST_NOTIFICATIONS`, asked for with the system's question
//! (`runtime_permissions`). Before that there is no question: the user turns the app's
//! notifications off in its settings, which `areNotificationsEnabled` tells, and that is the whole
//! state.
//!
//! The system says whether the permission is granted, and whether the user refused it once and
//! may be asked again (`shouldShowRequestPermissionRationale`), but not whether they were never
//! asked or refused it for good: both answer `NotAsked`. Asking then shows the question, or for one
//! refused for good answers at once that it is denied.
//!
//! Only the app's `request` asks. A post never does, as `notify` does not: it answers `Denied`
//! unless the permission is granted, where the system would show nothing.
//!
//! `NotificationPermission` names no view, so the question shows over the activity in front
//! (`views.rs`).

use std::rc::Rc;

use jni::objects::{JObject, JValue};
use jni::{Env, jni_sig, jni_str};
use waft_embedder::{NotificationError, NotificationPermission, PermissionFuture, PermissionState};

use crate::activity::{Activity, device_api_level};
use crate::application::Application;
use crate::log::warn_to_log;
use crate::runtime_permissions::{self, Asked, answers_are_heard};
use crate::views::Views;

/// `Manifest.permission.POST_NOTIFICATIONS`.
const POST_NOTIFICATIONS: &str = "android.permission.POST_NOTIFICATIONS";
/// Android 13, from which posting is a permission the user is asked for.
const ASKED_FROM: i32 = 33;

/// The user's permission to notify, as the system keeps it for the app.
#[derive(Clone)]
pub(super) struct Permission {
    application: Application,
    /// The views, whose activity in front the question is asked over, and gets its answer.
    views: Rc<Views>,
    /// Whether the system asks the user: Android 13 and later, for an app that targets it.
    asks: bool,
}

impl Permission {
    /// The permission, asked for over the activity in front, once the answer method of Waft's
    /// activity is registered; `None` where the app's activities, as `first` tells, are not
    /// Waft's, which hand no answer on.
    pub(super) fn new(
        application: Application,
        views: Rc<Views>,
        first: &Activity,
    ) -> Option<Permission> {
        if !(answers_are_heard(&application) && first.is_wafts()) {
            return None;
        }
        let asks = application.with(|env, context| {
            Ok(device_api_level() >= ASKED_FROM && target_api_level(env, context)? >= ASKED_FROM)
        });
        match asks {
            Ok(asks) => Some(Permission {
                application,
                views,
                asks,
            }),
            Err(_) => {
                application.clear_exception();
                None
            }
        }
    }

    /// Whether the app may notify now, without asking.
    pub(super) fn allowed(&self) -> Result<(), NotificationError> {
        match self.current() {
            PermissionState::Granted => Ok(()),
            PermissionState::Denied | PermissionState::NotAsked => Err(NotificationError::Denied),
        }
    }

    /// The permission as the system tells it now.
    fn current(&self) -> PermissionState {
        let current = self.where_held().and_then(|held| match held {
            Some(held) => Ok(held),
            None => self.where_not_held(),
        });
        current.unwrap_or_else(|error| {
            warn_to_log!("waft: the permission to notify could not be read: {error}");
            PermissionState::Denied
        })
    }

    /// The state where the app holds the permission, or the system asks for none: whether the
    /// user has the app's notifications on. `None` where the app does not hold it. The
    /// application answers.
    fn where_held(&self) -> jni::errors::Result<Option<PermissionState>> {
        self.application.with(|env, context| {
            let held =
                !self.asks || runtime_permissions::is_granted(env, context, POST_NOTIFICATIONS)?;
            if !held {
                return Ok(None);
            }
            Ok(Some(granted_if(notifications_enabled(env, context)?)))
        })
    }

    /// The state where the app does not hold the permission: refused, where the user refused it
    /// once, and otherwise not asked for. Only an activity answers.
    fn where_not_held(&self) -> jni::errors::Result<PermissionState> {
        self.in_front()?.with(|env, activity| {
            Ok(
                if runtime_permissions::refused_once(env, activity, POST_NOTIFICATIONS)? {
                    PermissionState::Denied
                } else {
                    PermissionState::NotAsked
                },
            )
        })
    }

    /// The activity in front, which the question is asked over; an error while none is.
    fn in_front(&self) -> jni::errors::Result<Activity> {
        self.views
            .activity_in_front()
            .ok_or(jni::errors::Error::NullPtr(
                "an activity in sight, while none is",
            ))
    }

    /// Shows the system's question, and answers the user's answer: `NotAsked` where the question
    /// went unanswered, and the state as it is where it could not be asked.
    fn ask(&self) -> PermissionFuture {
        let asking = self
            .in_front()
            .and_then(|in_front| runtime_permissions::ask(&in_front, POST_NOTIFICATIONS));
        match asking {
            Ok(answered) => Box::pin(async move {
                match answered.await {
                    Asked::Granted => PermissionState::Granted,
                    Asked::Refused => PermissionState::Denied,
                    Asked::Interrupted => PermissionState::NotAsked,
                }
            }),
            Err(error) => {
                warn_to_log!("waft: the permission to notify could not be asked for: {error}");
                self.application.clear_exception();
                Box::pin(std::future::ready(self.current()))
            }
        }
    }
}

impl NotificationPermission for Permission {
    fn state(&self) -> PermissionFuture {
        Box::pin(std::future::ready(self.current()))
    }

    fn request(&self) -> PermissionFuture {
        match self.current() {
            PermissionState::NotAsked => self.ask(),
            answered => Box::pin(std::future::ready(answered)),
        }
    }
}

fn granted_if(enabled: bool) -> PermissionState {
    if enabled {
        PermissionState::Granted
    } else {
        PermissionState::Denied
    }
}

fn notifications_enabled(env: &mut Env<'_>, context: &JObject<'_>) -> jni::errors::Result<bool> {
    let service = env.new_string("notification")?;
    let manager = env
        .call_method(
            context,
            jni_str!("getSystemService"),
            jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
            &[JValue::Object(&service)],
        )?
        .l()?;
    env.call_method(
        &manager,
        jni_str!("areNotificationsEnabled"),
        jni_sig!("()Z"),
        &[],
    )?
    .z()
}

/// The release the app's package targets, which decides whether the system asks for it.
fn target_api_level(env: &mut Env<'_>, context: &JObject<'_>) -> jni::errors::Result<i32> {
    let info = env
        .call_method(
            context,
            jni_str!("getApplicationInfo"),
            jni_sig!("()Landroid/content/pm/ApplicationInfo;"),
            &[],
        )?
        .l()?;
    env.get_field(&info, jni_str!("targetSdkVersion"), jni_sig!("I"))?
        .i()
}
