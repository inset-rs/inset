//! Notifications: the platform's toasts, through WinRT's `ToastNotificationManager`. A
//! notification is a toast (`toast`) tagged with its id, which a later toast of the same tag
//! replaces, shown at once or held by the platform's schedule until its time
//! (`ScheduledToastNotification`), whether the app runs or not; its image is a file the toast
//! reads (`images`). The platform shows the toasts of an app it knows (`identity`): an app it
//! does not know has no notifications here.
//!
//! A click comes through the app's toast activator where its registration names one
//! (`activator`), the click that started the app included; otherwise through the shown toast's
//! own `Activated` event while the app runs, which a toast scheduled or left in the Action Center
//! after the app quit has none of. Its arguments tell the notification and the action
//! (`arguments`).
//!
//! There is no question: the user turns an app's notifications off in Settings, which the
//! notifier answers (`ToastNotifier.Setting`).

mod activator;
mod arguments;
mod identity;
mod images;
mod toast;

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::SystemTime;

use inset_embedder::{
    Notification, NotificationError, NotificationPermission, NotificationReplies,
    NotificationScheduling, Notifications, PermissionFuture, PermissionState, PostFuture,
};
use windows::Data::Xml::Dom::XmlDocument;
use windows::Foundation::{DateTime, IReference, TypedEventHandler};
use windows::UI::Notifications::{
    NotificationSetting, ScheduledToastNotification, ToastActivatedEventArgs, ToastDismissalReason,
    ToastDismissedEventArgs, ToastNotification, ToastNotifier,
};
use windows::core::{HSTRING, IInspectable, Interface, Ref};

use self::identity::{AppIdentity, group};
use super::Windows;
use crate::notifications::NotificationActivations;
use crate::os::SystemNotifications;

impl SystemNotifications for Windows {
    /// Registers the app's toast activator, where its registration names one, before the loop
    /// runs, so the click that started the app is heard.
    fn hear_notification_activations(activations: NotificationActivations) {
        let Some(clsid) = AppIdentity::of_this_process().and_then(|app| app.activator()) else {
            return;
        };
        activator::register(clsid, activations);
    }

    fn notifications(activations: NotificationActivations) -> Option<Rc<dyn Notifications>> {
        let app = AppIdentity::of_this_process()?;
        let notifier = app.notifier().ok()?;
        Some(Rc::new(Toasts {
            app,
            notifier,
            clicks: (!activator::is_registered()).then(|| Clicks {
                activations,
                shown: Arc::default(),
            }),
        }))
    }
}

/// The app's toasts.
struct Toasts {
    app: AppIdentity,
    notifier: ToastNotifier,
    /// Where the shown toasts' own clicks go; `None` where the activator hears them.
    clicks: Option<Clicks>,
}

/// The shown toasts' own clicks, and the toasts, kept by tag so their `Activated` handlers last,
/// until the user dismisses one, the app removes it or a later toast takes its tag.
struct Clicks {
    activations: NotificationActivations,
    shown: Arc<Mutex<HashMap<HSTRING, ToastNotification>>>,
}

impl Toasts {
    /// Shows `notification` at once, in place of any toast shown or scheduled under its id.
    fn show(&self, notification: &Notification) -> Result<(), NotificationError> {
        self.allowed()?;
        let tag = HSTRING::from(toast::tag(&notification.id));
        self.unschedule(&tag).map_err(failed)?;
        let shown =
            ToastNotification::CreateToastNotification(&document(notification)?).map_err(failed)?;
        shown.SetTag(&tag).map_err(failed)?;
        shown.SetGroup(&group()).map_err(failed)?;
        if let Some(clicks) = &self.clicks {
            clicks.hear(&shown, &tag).map_err(failed)?;
        }
        self.notifier.Show(&shown).map_err(failed)?;
        if let Some(clicks) = &self.clicks {
            locked(&clicks.shown).insert(tag, shown);
        }
        Ok(())
    }

    /// Holds `notification` in the platform's schedule until `ticks`, in place of any toast
    /// scheduled under its id.
    fn add_to_schedule(
        &self,
        notification: &Notification,
        ticks: i64,
    ) -> Result<(), NotificationError> {
        self.allowed()?;
        let tag = HSTRING::from(toast::tag(&notification.id));
        self.unschedule(&tag).map_err(failed)?;
        let scheduled = ScheduledToastNotification::CreateScheduledToastNotification(
            &document(notification)?,
            DateTime {
                UniversalTime: ticks,
            },
        )
        .map_err(failed)?;
        scheduled.SetTag(&tag).map_err(failed)?;
        scheduled.SetGroup(&group()).map_err(failed)?;
        self.notifier.AddToSchedule(&scheduled).map_err(failed)
    }

    /// Takes the toasts tagged `tag` out of the platform's schedule.
    fn unschedule(&self, tag: &HSTRING) -> windows::core::Result<()> {
        let group = group();
        for scheduled in self.notifier.GetScheduledToastNotifications()? {
            if scheduled.Tag()? == *tag && scheduled.Group()? == group {
                self.notifier.RemoveFromSchedule(&scheduled)?;
            }
        }
        Ok(())
    }

    /// Whether the user lets the app notify, as Settings has it.
    fn allowed(&self) -> Result<(), NotificationError> {
        match self.notifier.Setting().map_err(failed)? {
            NotificationSetting::Enabled => Ok(()),
            _ => Err(NotificationError::Denied),
        }
    }
}

impl Notifications for Toasts {
    fn post(&self, notification: &Notification) -> PostFuture {
        Box::pin(std::future::ready(self.show(notification)))
    }

    fn remove(&self, id: &str) {
        let tag = HSTRING::from(toast::tag(id));
        let _ = self.unschedule(&tag);
        let _ = self.app.remove_shown(&tag);
        if let Some(clicks) = &self.clicks {
            locked(&clicks.shown).remove(&tag);
        }
    }

    fn permission(&self) -> Option<&dyn NotificationPermission> {
        Some(self)
    }

    fn replies(&self) -> Option<&dyn NotificationReplies> {
        Some(self)
    }

    fn scheduling(&self) -> Option<&dyn NotificationScheduling> {
        Some(self)
    }
}

/// Settings' switch for the app, which the platform never asks the user about.
impl NotificationPermission for Toasts {
    fn state(&self) -> PermissionFuture {
        let state = match self.allowed() {
            Ok(()) => PermissionState::Granted,
            Err(_) => PermissionState::Denied,
        };
        Box::pin(std::future::ready(state))
    }

    fn request(&self) -> PermissionFuture {
        self.state()
    }
}

/// A toast's text box.
impl NotificationReplies for Toasts {}

/// `ScheduledToastNotification`, which the platform shows at its time whether the app runs.
impl NotificationScheduling for Toasts {
    fn schedule(&self, notification: &Notification, at: SystemTime) -> PostFuture {
        let scheduled = match toast::delivery_ticks(at, SystemTime::now()) {
            Some(ticks) => self.add_to_schedule(notification, ticks),
            None => self.show(notification),
        };
        Box::pin(std::future::ready(scheduled))
    }
}

impl Clicks {
    /// Hears `shown`'s clicks, and forgets it, kept under `tag`, once the user dismisses it.
    fn hear(&self, shown: &ToastNotification, tag: &HSTRING) -> windows::core::Result<()> {
        let activations = self.activations.clone();
        shown.Activated(&TypedEventHandler::new(
            move |_: Ref<ToastNotification>, clicked: Ref<IInspectable>| {
                if let Some(activation) = clicked.as_ref().and_then(activation) {
                    activations.send(activation);
                }
                Ok(())
            },
        ))?;
        let kept = Arc::clone(&self.shown);
        let dismissed_tag = tag.clone();
        shown.Dismissed(&TypedEventHandler::new(
            move |toast: Ref<ToastNotification>, dismissed: Ref<ToastDismissedEventArgs>| {
                let by_the_user = dismissed
                    .as_ref()
                    .and_then(|dismissed| dismissed.Reason().ok())
                    == Some(ToastDismissalReason::UserCanceled);
                if by_the_user {
                    forget(&kept, &dismissed_tag, toast.as_ref());
                }
                Ok(())
            },
        ))?;
        Ok(())
    }
}

/// Forgets the toast kept under `tag` if it is `toast`, rather than a later one of the same tag.
fn forget(
    kept: &Mutex<HashMap<HSTRING, ToastNotification>>,
    tag: &HSTRING,
    toast: Option<&ToastNotification>,
) {
    let mut kept = locked(kept);
    if toast.is_some() && kept.get(tag) == toast {
        kept.remove(tag);
    }
}

/// The click a toast's `Activated` event tells, with what was typed into its text boxes.
fn activation(clicked: &IInspectable) -> Option<inset_embedder::NotificationActivation> {
    let clicked = clicked.cast::<ToastActivatedEventArgs>().ok()?;
    let invoked = clicked.Arguments().ok()?.to_string_lossy();
    let typed = clicked.UserInput().ok();
    arguments::activation(&invoked, |input| {
        let value = typed.as_ref()?.Lookup(&HSTRING::from(input)).ok()?;
        let text = value.cast::<IReference<HSTRING>>().ok()?.Value().ok()?;
        Some(text.to_string_lossy())
    })
}

/// The toast's XML for `notification`, as the platform reads it.
fn document(notification: &Notification) -> Result<XmlDocument, NotificationError> {
    let image_uri = notification
        .image
        .as_ref()
        .map(images::image_uri)
        .transpose()
        .map_err(|error| {
            NotificationError::Failed(format!("the image was not written: {error}"))
        })?;
    let document = XmlDocument::new().map_err(failed)?;
    document
        .LoadXml(&HSTRING::from(toast::toast_xml(
            notification,
            image_uri.as_deref(),
        )))
        .map_err(failed)?;
    Ok(document)
}

fn failed(error: windows::core::Error) -> NotificationError {
    NotificationError::Failed(error.message())
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
