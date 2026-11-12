//! The window's own surface, beneath the view's picture: one buffer of the window's size, filled
//! with the view's clear colour. Android sizes a window by the buffer its own surface holds: the
//! frame and the touchable region of the window's input are the bounds of that buffer, and a
//! surface that holds none gets an empty frame and no touches, however much its children show
//! (SurfaceFlinger's `getInputBounds` and `fillInputFrameInfo`, in
//! `services/surfaceflinger/FrontEnd/LayerSnapshotBuilder.cpp`; `RequestedLayerState::getBufferSize`
//! is `INVALID_RECT` with no buffer). A `NativeActivity` takes its window's surface from the view
//! system (`Window.takeSurface`), so nothing else draws it: the app owns the window's content, and
//! this is it. It shows only where the view's picture does not yet cover the window, before the
//! first frame and while a new size has not been drawn.
//!
//! The invariant: the buffer is the window's size, drawn again whenever that size changes, or the
//! window takes touches on the old size's bounds.

use ndk::hardware_buffer_format::HardwareBufferFormat;
use ndk::native_window::NativeWindow;

/// What the window's surface is filled as: opaque, as `NativeActivity`'s window is.
const FORMAT: HardwareBufferFormat = HardwareBufferFormat::R8G8B8X8_UNORM;

/// The window's own surface, and the size of the buffer it holds.
pub(super) struct WindowSurface {
    window: NativeWindow,
    /// The size of the buffer last posted, in pixels.
    filled: Option<[u32; 2]>,
}

impl WindowSurface {
    /// `window`'s own surface, holding nothing yet.
    pub(super) fn new(window: &NativeWindow) -> WindowSurface {
        WindowSurface {
            window: window.clone(),
            filled: None,
        }
    }

    /// Posts a buffer of `clear` to the surface where the one it holds is not `size` pixels large.
    pub(super) fn fill(&mut self, size: [u32; 2], clear: valo::Color) {
        if self.filled == Some(size) {
            return;
        }
        match post_filled(&self.window, clear) {
            Ok(posted) => self.filled = Some(posted),
            Err(error) => crate::log::warn_to_log!("the window's surface took no buffer: {error}"),
        }
    }
}

/// Posts a buffer of the window's size filled with `clear`, and answers its size.
fn post_filled(window: &NativeWindow, clear: valo::Color) -> std::io::Result<[u32; 2]> {
    window.set_buffers_geometry(0, 0, Some(FORMAT))?;
    let mut buffer = window.lock(None)?;
    let size = [buffer.width() as u32, buffer.height() as u32];
    let pixel = opaque_pixel(clear);
    let lines = buffer
        .lines()
        .ok_or_else(|| std::io::Error::other("the window's buffer has no known pixel size"))?;
    for line in lines {
        for (byte, value) in line.iter_mut().zip(pixel.iter().cycle()) {
            byte.write(*value);
        }
    }
    Ok(size)
}

/// `color` as the bytes of one `R8G8B8X8` pixel.
fn opaque_pixel(color: valo::Color) -> [u8; 4] {
    let byte = |component: f32| (component.clamp(0.0, 1.0) * 255.0).round() as u8;
    [byte(color.r), byte(color.g), byte(color.b), u8::MAX]
}
