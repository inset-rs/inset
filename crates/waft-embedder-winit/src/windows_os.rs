//! What only Windows offers an app, beyond `Platform`: posting notifications as another app, one
//! the notification platform knows by its AppUserModelID, as a host whose windows are other apps'
//! posts theirs. Flutter's framework has no API for it, so it is this embedder's own, and an app
//! reaches it through [`WinitPlatform::windows`](crate::WinitPlatform::windows). The module is not
//! named `windows`, which holds the app's windows.

use std::rc::Rc;

use waft_embedder::Notifications;

use crate::main_loop::{MainLoopEvent, MainLoopProxy};
use crate::notifications::NotificationActivations;
use crate::os::System;
use crate::os::capabilities::SystemNotifications;
use crate::state::EmbedderState;
use crate::system_requests::{NotificationActivationHandler, SystemRequest};

/// What only Windows offers an app. There on Windows alone, so a call here always does what it
/// says.
pub struct Windows {
    state: Rc<EmbedderState>,
    proxy: MainLoopProxy,
}

impl Windows {
    pub(crate) fn new(state: Rc<EmbedderState>, proxy: MainLoopProxy) -> Windows {
        Windows { state, proxy }
    }

    /// The notifications of the app the notification platform knows as `app_user_model_id`,
    /// which this process posts as that app: under its name and icon, and its switch in Settings.
    /// The platform knows an app by a Start menu shortcut carrying its ID or by its registration
    /// under `HKCU\Software\Classes\AppUserModelId\<id>`, which are the app's to write; `None`
    /// where the platform will not make a notifier for the ID.
    ///
    /// The clicks on them go to `on_activation`, on the main thread and outside every framework
    /// call, never to the client: while the app runs, from the toast itself; after it quit, and
    /// for the click that started it, through the toast activator the ID's registration names,
    /// where that is the app's own class. A click that comes before this call is held until it.
    pub fn notifications_of(
        &self,
        app_user_model_id: &str,
        on_activation: NotificationActivationHandler,
    ) -> Option<Rc<dyn Notifications>> {
        let activations = NotificationActivations::new(self.proxy.clone()).of(app_user_model_id);
        let notifications = System::notifications_of(app_user_model_id, activations)?;
        let held = self
            .state
            .handlers()
            .set_notification_activation(app_user_model_id, on_activation);
        for activation in held {
            self.proxy.send(MainLoopEvent::SystemRequest(
                SystemRequest::NotificationActivated {
                    identity: Some(app_user_model_id.to_owned()),
                    activation,
                },
            ));
        }
        Some(notifications)
    }
}
