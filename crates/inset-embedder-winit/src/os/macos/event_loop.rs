//! What winit would set up of AppKit's application as it builds its event loop, and the embedder
//! leaves to others: the menu bar is the system's standard one, not winit's (`menus`), and whether
//! the app comes forward at launch is macOS's to decide.

use winit::event_loop::EventLoopBuilder;
use winit::platform::macos::EventLoopBuilderExtMacOS;

use super::{AppKit, menus};
use crate::main_loop::MainLoopEvent;
use crate::os::EventLoopSetup;

impl EventLoopSetup for AppKit {
    fn configure_event_loop(builder: &mut EventLoopBuilder<MainLoopEvent>) {
        menus::leave_winit_menu_out(builder);
        leave_activation_to_macos(builder);
    }
}

/// The app comes forward at launch only as macOS brings it: what is launched as an app, from the
/// Dock, the Finder, a notification's click or `open`, comes forward, and what is launched in the
/// background, as another app starts it to post a notification or at login, stays behind. winit
/// would otherwise activate the app over whatever the user is in. A bare executable run from a
/// terminal opens behind the terminal.
fn leave_activation_to_macos(builder: &mut EventLoopBuilder<MainLoopEvent>) {
    builder.with_activate_ignoring_other_apps(false);
}
