//! macOS's compositor: Core Animation's, which iOS shares (`apple/composition`), in the layer of
//! winit's `NSView`.

use std::sync::Arc;

use inset_embedder::Compositor;
use objc2::rc::Retained;
use objc2_quartz_core::CALayer;
use winit::window::Window;

use super::{AppKit, ns_view};
use crate::os::NativeComposition;
use crate::os::apple::composition::{
    CoreAnimationCompositor, CoreAnimationWindowSurfaces, WindowLayer,
};

impl NativeComposition for AppKit {
    type WindowSurfaces = CoreAnimationWindowSurfaces<AppKit>;

    fn compositor() -> Option<Arc<dyn Compositor>> {
        Some(Arc::new(CoreAnimationCompositor))
    }
}

impl WindowLayer for AppKit {
    fn window_layer(window: &dyn Window) -> Option<Retained<CALayer>> {
        ns_view(window)?.layer()
    }
}
