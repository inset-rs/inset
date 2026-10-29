//! What winit would set up of AppKit's application as it builds its event loop, and the embedder
//! sets up otherwise: the menu bar is the system's standard one, not winit's (`menus`).

use winit::event_loop::EventLoopBuilder;

use super::{AppKit, menus};
use crate::main_loop::MainLoopEvent;
use crate::os::EventLoopSetup;

impl EventLoopSetup for AppKit {
    fn configure_event_loop(builder: &mut EventLoopBuilder<MainLoopEvent>) {
        menus::leave_winit_menu_out(builder);
    }
}
