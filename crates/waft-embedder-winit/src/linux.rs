//! What only Linux offers an app, beyond `Platform`: bringing a window forward with the activation
//! token a launch was given, which a later launch passed on to the app's first instance
//! (`waft-single-instance`'s `Launch::activation_token`). A desktop lets a window take the focus
//! only for the launch it meant it for, so without the token the window may stay behind the one in
//! front. Flutter's framework has no API for it, so it is this embedder's own, and an app reaches it
//! through [`WinitPlatform::linux`](crate::WinitPlatform::linux).

use std::rc::Rc;

use waft_embedder::WindowRef;

use crate::os::System;
use crate::os::capabilities::WindowChrome;
use crate::state::EmbedderState;

/// What only Linux offers an app. There on Linux alone, so a call here always does what it says.
pub struct Linux {
    state: Rc<EmbedderState>,
}

impl Linux {
    pub(crate) fn new(state: Rc<EmbedderState>) -> Linux {
        Linux { state }
    }

    /// Brings `window` forward with `token`, an XDG activation token (`XDG_ACTIVATION_TOKEN`) a
    /// launch was given, as the desktop meant for that launch's window. A token the compositor
    /// does not know, or no longer honours, is ignored; so is every token on X11 and under a
    /// compositor without XDG activation. A window about to open takes its token as
    /// `LinuxWindowConfig::activation_token` instead.
    pub fn activate(&self, window: &WindowRef, token: &str) {
        if let Some(open) = self.state.windows().by_view(window.view().id()) {
            System::activate_with_token(&*open.window, token);
        }
    }
}
