//! iOS's compositor: Core Animation's, which macOS shares (`apple/composition`), in the layer of
//! winit's `UIView`.

// The window's raw handle needs `unsafe`; this module writes it for it.
#![allow(unsafe_code)]

use std::sync::Arc;

use inset_embedder::Compositor;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_quartz_core::CALayer;
use objc2_ui_kit::UIView;
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use super::Ios;
use crate::os::NativeComposition;
use crate::os::apple::composition::{
    CoreAnimationCompositor, CoreAnimationWindowSurfaces, WindowLayer,
};

impl NativeComposition for Ios {
    type WindowSurfaces = CoreAnimationWindowSurfaces<Ios>;

    fn compositor() -> Option<Arc<dyn Compositor>> {
        Some(Arc::new(CoreAnimationCompositor))
    }
}

impl WindowLayer for Ios {
    fn window_layer(window: &Window) -> Option<Retained<CALayer>> {
        MainThreadMarker::new()?;
        let RawWindowHandle::UiKit(uikit) = window.window_handle().ok()?.as_raw() else {
            return None;
        };
        // SAFETY: winit's view of a live window, on the main thread, checked above.
        let view = unsafe { Retained::retain(uikit.ui_view.cast::<UIView>().as_ptr()) }?;
        Some(view.layer())
    }
}
