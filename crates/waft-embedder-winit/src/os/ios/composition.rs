//! iOS's compositor: Core Animation's, which macOS shares (`apple/composition`), in the layer of
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

use super::Ios;
use crate::os::apple::composition::{
    CoreAnimationCompositor, CoreAnimationWindowComposition, WindowLayer,
};
use crate::os::apple::hardware_buffers::SurfaceBufferAllocator;
use crate::os::capabilities::NativeComposition;
use crate::os::window_handle_layer::{NoWindowHold, WindowHandleLayer};

impl NativeComposition for Ios {
    type ViewLayer = WindowHandleLayer<CoreAnimationWindowComposition<Ios>>;
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

impl WindowLayer for Ios {
    fn window_layer(window: &dyn Window) -> Option<Retained<CALayer>> {
        MainThreadMarker::new()?;
        let RawWindowHandle::UiKit(uikit) = window.window_handle().ok()?.as_raw() else {
            return None;
        };
        // SAFETY: winit's view of a live window, on the main thread, checked above.
        let view = unsafe { Retained::retain(uikit.ui_view.cast::<UIView>().as_ptr()) }?;
        Some(view.layer())
    }
}
