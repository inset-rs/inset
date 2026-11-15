//! iOS's compositor: Core Animation's, which macOS shares (`waft-embedder-apple`'s `composition`), in the layer of
//! winit's `UIView`. A view draws through its window's handle (`window_handle_layer.rs`), into the
//! `CAMetalLayer` wgpu makes in that layer, with the composition layers its scenes place beneath it.

// The window's raw handle needs `unsafe`; this module writes it for it.
#![allow(unsafe_code)]

use std::sync::Arc;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_quartz_core::CALayer;
use objc2_ui_kit::UIView;
use waft_embedder::{Compositor, HardwareBufferAllocator};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use waft_embedder_apple::composition::CoreAnimationCompositor;
use waft_embedder_apple::hardware_buffers::SurfaceBufferAllocator;

use super::Ios;
use super::window_composition::CoreAnimationWindowComposition;
use crate::os::capabilities::NativeComposition;
use crate::os::window_handle_layer::{NoWindowHold, WindowHandleLayer};

impl NativeComposition for Ios {
    type ViewLayer = WindowHandleLayer<CoreAnimationWindowComposition>;
    type WindowHold = NoWindowHold;

    fn compositor() -> Option<Arc<dyn Compositor>> {
        Some(Arc::new(CoreAnimationCompositor))
    }

    fn hardware_buffer_allocator() -> Option<Arc<dyn HardwareBufferAllocator>> {
        Some(Arc::new(SurfaceBufferAllocator))
    }

    fn view_layer_in(window: &Arc<dyn Window>) -> (Self::ViewLayer, NoWindowHold) {
        WindowHandleLayer::in_window(window)
    }
}

/// The layer of `window`'s view, which holds the layers of its content; `None` off the main thread,
/// or for a view without one.
pub(super) fn window_layer(window: &dyn Window) -> Option<Retained<CALayer>> {
    MainThreadMarker::new()?;
    let RawWindowHandle::UiKit(uikit) = window.window_handle().ok()?.as_raw() else {
        return None;
    };
    // SAFETY: winit's view of a live window, on the main thread, checked above.
    let view = unsafe { Retained::retain(uikit.ui_view.cast::<UIView>().as_ptr()) }?;
    Some(view.layer())
}
