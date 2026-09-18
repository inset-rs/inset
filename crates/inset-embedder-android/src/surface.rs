//! The window's valo surface: rendering a picture into a drawable and presenting it.
//!
//! The system takes the window away when the activity goes to the background and gives a
//! new one back on return, so the surface is made and dropped over the host's life while
//! the view outlives both. A draw with no surface is refused like any other, and the view
//! decides what to do about it.

// Naming Android's display to wgpu is the one `unsafe` here: see `Target`.
#![allow(unsafe_code)]

use inset_embedder::Picture;
use ndk::native_window::NativeWindow;
use raw_window_handle::{
    AndroidDisplayHandle, DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle,
    RawDisplayHandle, WindowHandle,
};

pub(crate) struct Surface {
    /// `None` between the window being taken away and the next one arriving.
    pub(crate) surface: Option<valo::Surface>,
    pub(crate) context: valo::Context,
}

impl Surface {
    /// Renders `picture` into the surface's next drawable and presents it, cleared to
    /// `clear`. `Err` is the surface refusing a drawable.
    pub(crate) fn draw(
        &mut self,
        picture: &Picture,
        clear: valo::Color,
    ) -> Result<(), valo::Refused> {
        let Some(surface) = &mut self.surface else {
            return Err(valo::Refused::Lost);
        };
        let frame = surface.acquire()?;
        self.context.render(picture, &frame.target(Some(clear)));
        self.context.present(frame);
        Ok(())
    }
}

/// The window as wgpu takes it. wgpu asks a target for both the window and the display;
/// [`NativeWindow`] answers for the window alone, and Android's display names nothing.
pub(crate) struct Target(pub(crate) NativeWindow);

impl HasWindowHandle for Target {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.0.window_handle()
    }
}

impl HasDisplayHandle for Target {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // SAFETY: Android's display handle names nothing — it carries no pointer and no
        // lifetime — so the borrow is valid for as long as this target is.
        let handle = unsafe {
            DisplayHandle::borrow_raw(RawDisplayHandle::Android(AndroidDisplayHandle::new()))
        };
        Ok(handle)
    }
}
