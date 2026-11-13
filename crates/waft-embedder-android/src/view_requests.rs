//! Asking for a view (`Platform::request_view`): Waft's document activity,
//! `rs.waft.WaftDocumentActivity`, started from the application in a task of its own, a card in
//! Recents, as a desktop makes a window for each request; the activity arrives later as the view,
//! carrying the request's number and the view's initial data.
//!
//! The activity's intent is the request (`WaftDocumentActivity.java`): its data is the view's
//! initial data, and its extras number the request and the process that made it
//! (`this_process.rs`). Android keeps that intent as the task's, so a card it brings back by itself
//! after the process died starts the activity with it again. An activity is told apart so, at its
//! creation:
//!
//! - one Android makes in place of one it destroyed in this process carries that one's saved token
//!   (`saved_view.rs`) and is the same view again, never an arrival here;
//! - one whose intent names this process and a request still waiting is that request's view;
//! - any other is a view the platform gave: a restored card's, its intent another process's, which
//!   carries the initial data and no request, or the launcher's.
//!
//! A request is answered once: the invariant a later editor will break is that the request leaves
//! [`ViewRequests`] as its view arrives, so no second activity with the same intent is told as the
//! same request.
//!
//! Android starts no activity for an app with nothing in sight, and says nothing of the start it
//! blocked (`BAL_BLOCK`), so a request made while no view of the app has a window fails at once,
//! rather than waiting for a view that never comes.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use jni::objects::{JObject, JValue};
use jni::{Env, jni_sig, jni_str};
use waft_embedder::{RequestId, ViewError, WindowConfig};

use crate::activity::{Activity, WAFT_DOCUMENT_ACTIVITY};
use crate::application::{Application, package_class};
use crate::file_dialogs::string;
use crate::log::warn_to_log;
use crate::loop_waker::wake_app_loop;
use crate::this_process::this_process;

/// What a request's intent says of no request.
const NO_REQUEST: i64 = -1;

/// The requests for views whose activities have not arrived, and those that failed.
pub(crate) struct ViewRequests {
    application: Application,
    /// Whether the package has Waft's document activity, without which a request is refused.
    documents: bool,
    /// The number the next request takes.
    next: Cell<u64>,
    /// The window each request still waiting asked for.
    waiting: RefCell<BTreeMap<RequestId, WindowConfig>>,
    /// The requests that failed, for the loop to tell the client.
    failed: RefCell<Vec<(RequestId, ViewError)>>,
}

/// What an activity Android made for a view new to the app arrived with.
#[derive(Default)]
pub(crate) struct Arrival {
    /// The request it answers, with the window it asked for; none for a view the platform gave.
    pub(crate) request: Option<(RequestId, WindowConfig)>,
    pub(crate) initial_data: Option<String>,
}

impl ViewRequests {
    /// The requests of the app `application` is, through its document activity where its package
    /// has one.
    pub(crate) fn new(application: Application) -> ViewRequests {
        let documents = application
            .with(|env, context| package_class(env, context, WAFT_DOCUMENT_ACTIVITY).map(drop))
            .is_ok();
        if !documents {
            application.clear_exception();
        }
        ViewRequests {
            application,
            documents,
            next: Cell::new(0),
            waiting: RefCell::default(),
            failed: RefCell::default(),
        }
    }

    /// Starts a task of its own for a view in a window configured as `config`, carrying
    /// `initial_data`, while something of the app is `in_sight`. Refused at once in a package
    /// without the document activity; a start Android refused, or one while nothing is in sight,
    /// fails through [`take_failed`](Self::take_failed).
    pub(crate) fn request(
        &self,
        config: WindowConfig,
        initial_data: Option<String>,
        in_sight: bool,
    ) -> Result<RequestId, ViewError> {
        if !self.documents {
            return Err(ViewError::Unsupported);
        }
        let id = RequestId(self.next.get());
        self.next.set(id.0 + 1);
        if !in_sight {
            self.fail(
                id,
                "nothing of the app is in sight, and Android starts no activity for an app in \
                 the background"
                    .to_owned(),
            );
            return Ok(id);
        }
        match self.start(id, initial_data.as_deref()) {
            Ok(()) => {
                self.waiting.borrow_mut().insert(id, config);
            }
            Err(reason) => self.fail(id, reason),
        }
        Ok(id)
    }

    /// What `activity`, made for a view new to the app, arrived with: the request it answers,
    /// which waits no more, and its initial data, where it is the document activity.
    pub(crate) fn arrival(&self, activity: &Activity) -> Arrival {
        if !activity.is_document() {
            return Arrival::default();
        }
        let launch = activity.with(|env, activity| {
            let data = env
                .call_method(
                    activity,
                    jni_str!("initialData"),
                    jni_sig!("()Ljava/lang/String;"),
                    &[],
                )?
                .l()?;
            let request = env
                .call_method(
                    activity,
                    jni_str!("request"),
                    jni_sig!("(J)J"),
                    &[JValue::Long(this_process() as i64)],
                )?
                .j()?;
            Ok((string(env, data)?, request))
        });
        let (initial_data, request) = launch.unwrap_or_else(|error| {
            activity.clear_exception();
            warn_to_log!("waft: what the view's activity was started with is not known: {error}");
            (None, NO_REQUEST)
        });
        let request = u64::try_from(request).ok().map(RequestId).and_then(|id| {
            let config = self.waiting.borrow_mut().remove(&id)?;
            Some((id, config))
        });
        Arrival {
            request,
            initial_data,
        }
    }

    /// The requests that failed since the last call, in the order they failed.
    pub(crate) fn take_failed(&self) -> Vec<(RequestId, ViewError)> {
        std::mem::take(&mut *self.failed.borrow_mut())
    }

    /// Starts the document activity for request `id`, carrying `initial_data`; the reason Android
    /// refused it otherwise.
    fn start(&self, id: RequestId, initial_data: Option<&str>) -> Result<(), String> {
        let started = self.application.with(|env, context| {
            let started = start_document(env, context, id, initial_data);
            Ok(started.map_err(|error| exception_text(env, &error)))
        });
        started.unwrap_or_else(|error| {
            self.application.clear_exception();
            Err(error.to_string())
        })
    }

    /// Request `id` failed for `reason`, which the loop tells the client on its next pass.
    fn fail(&self, id: RequestId, reason: String) {
        warn_to_log!("waft: the view asked for got no task: {reason}");
        self.failed
            .borrow_mut()
            .push((id, ViewError::Failed(reason)));
        wake_app_loop();
    }
}

/// `WaftDocumentActivity.start`, from `context`, for request `id` of this process.
fn start_document(
    env: &mut Env<'_>,
    context: &JObject<'_>,
    id: RequestId,
    initial_data: Option<&str>,
) -> jni::errors::Result<()> {
    let class = package_class(env, context, WAFT_DOCUMENT_ACTIVITY)?;
    let initial_data = match initial_data {
        Some(data) => JObject::from(env.new_string(data)?),
        None => JObject::null(),
    };
    env.call_static_method(
        &class,
        jni_str!("start"),
        jni_sig!("(Landroid/content/Context;Ljava/lang/String;JJ)V"),
        &[
            JValue::Object(context),
            JValue::Object(&initial_data),
            JValue::Long(this_process() as i64),
            JValue::Long(id.0 as i64),
        ],
    )?;
    Ok(())
}

/// What a failed call into Java tells: the exception it left, which is cleared, or the call's
/// error.
fn exception_text(env: &mut Env<'_>, error: &jni::errors::Error) -> String {
    let Some(exception) = env.exception_occurred() else {
        return error.to_string();
    };
    env.exception_clear();
    let described = env
        .call_method(
            &exception,
            jni_str!("toString"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )
        .and_then(|text| text.l())
        .and_then(|text| string(env, text));
    env.exception_clear();
    match described {
        Ok(Some(text)) => text,
        _ => error.to_string(),
    }
}
