//! The window's valo surface: rendering a picture into a drawable and presenting it, where the
//! view's picture reaches the screen through a swapchain on the window (`picture.rs`).
//!
//! The system takes the window away when the activity goes to the background and gives a
//! new one back on return, so the surface is made and dropped over the host's life while
//! the view outlives both. A draw with no surface is refused like any other, and the view
//! decides what to do about it.

// Naming Android's display to wgpu is the one `unsafe` here: see `Target`.
#![allow(unsafe_code)]

use ndk::native_window::NativeWindow;
use raw_window_handle::{
    AndroidDisplayHandle, DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle,
    RawDisplayHandle, WindowHandle,
};
use waft_embedder::Picture;

use crate::gpu::Gpu;

pub(crate) struct Surface {
    /// `None` between the window being taken away and the next one arriving.
    surface: Option<valo::Surface>,
    context: valo::Context,
}

impl Surface {
    /// A surface with no window yet, which renders with `context`.
    pub(crate) fn new(context: valo::Context) -> Surface {
        Surface {
            surface: None,
            context,
        }
    }

    /// The system gave the activity `window`, `size` pixels large: a surface on it, on `gpu`'s
    /// device. `Err` where wgpu makes none.
    pub(crate) fn window_arrived(
        &mut self,
        gpu: &Gpu,
        window: &NativeWindow,
        size: [u32; 2],
    ) -> Result<(), String> {
        let made = valo::Surface::new_with_options(
            &gpu.instance,
            &gpu.adapter,
            &gpu.device,
            Target(window.clone()),
            size,
            valo::SurfaceOptions::default(),
        )
        .map_err(|error| error.to_string())?;
        self.surface = Some(made);
        Ok(())
    }

    /// The system took the window away, and the surface with it.
    pub(crate) fn window_gone(&mut self) {
        self.surface = None;
    }

    /// The window changed size: the surface follows it.
    pub(crate) fn resized(&mut self, size: [u32; 2]) {
        if let Some(surface) = &mut self.surface {
            surface.resize(size);
        }
    }

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
