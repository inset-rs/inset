//! macOS's compositor: Core Animation's, which iOS shares (`apple/composition`), in the layer of
//! winit's `NSView`; and what a window's code does for its view there, as an app does for a view it
//! adds on a layer of its own: it makes the composition layer the view draws into
//! (`apple/composition/view_layer.rs`), places it in the layer of winit's view, filling it, and
//! keeps a hold on it ([`ViewLayerFilling`]) that places it again whenever that layer's bounds or
//! scale change.
//!
//! The hold follows the layer of winit's view itself, by key-value observing of its `bounds` and
//! `contentsScale`, as wgpu's `CAMetalLayer` followed it (raw-window-metal's `ObserverLayer`), and
//! as Flutter's view follows its own frame and backing scale (`setFrameSize:`,
//! `viewDidChangeBackingProperties` in `FlutterView.mm`), rather than winit's resize event, which
//! winit delivers later in the turn when it comes inside one of its calls, as when the app resizes
//! its window from a frame. So the composition layer changes in the transaction the view's layer
//! changes in.

use std::ffi::c_void;
use std::sync::Arc;

use inset_embedder::{CompositionLayer, Compositor};
use objc2::rc::{Retained, Weak};
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol};
use objc2::{AllocAnyThread, DefinedClass, define_class, msg_send};
use objc2_foundation::{
    NSDictionary, NSKeyValueChangeKey, NSKeyValueObservingOptions,
    NSObjectNSKeyValueObserverRegistration, NSString,
};
use objc2_quartz_core::CALayer;
use winit::window::Window;

use super::{AppKit, abort_on_panic, ns_view};
use crate::os::apple::composition::{
    CoreAnimationCompositor, CoreAnimationViewLayer, WindowLayer, place_filling,
};
use crate::os::capabilities::NativeComposition;

impl NativeComposition for AppKit {
    type ViewLayer = CoreAnimationViewLayer;
    type WindowHold = ViewLayerFilling;

    fn compositor() -> Option<Arc<dyn Compositor>> {
        Some(Arc::new(CoreAnimationCompositor))
    }

    /// A composition layer made by the host's compositor, made with buffers that carry alpha: it
    /// is never presented a buffer, and its empty buffer layer lies above the composition layers
    /// the view's scenes place. How the view's drawing blends is the surface's alpha
    /// (`rasterizer.rs`).
    fn view_layer_in(window: &Arc<dyn Window>) -> (CoreAnimationViewLayer, ViewLayerFilling) {
        let layer = CoreAnimationCompositor.make_layer(false);
        let filling = ViewLayerFilling::new(&**window, Arc::clone(&layer));
        let view_layer =
            CoreAnimationViewLayer::on(&layer).expect("a layer this host's compositor made");
        (view_layer, filling)
    }
}

impl WindowLayer for AppKit {
    fn window_layer(window: &dyn Window) -> Option<Retained<CALayer>> {
        ns_view(window)?.layer()
    }
}

/// The key paths of the layer of winit's view whose changes place the view's layer again.
const FOLLOWED_KEY_PATHS: [&str; 2] = ["bounds", "contentsScale"];

/// A window's hold on the composition layer its view draws into: the layer placed in the layer of
/// winit's view, filling it, and placed again whenever that layer's bounds or scale change, until
/// the hold is dropped as the window closes.
pub(crate) struct ViewLayerFilling {
    /// `None` for a view without a layer, where nothing is placed.
    observer: Option<Retained<FillingObserver>>,
}

impl ViewLayerFilling {
    /// Places `layer` in the layer of `window`'s view, filling it, and keeps it filling it.
    fn new(window: &dyn Window, layer: Arc<dyn CompositionLayer>) -> ViewLayerFilling {
        let Some(parent) = AppKit::window_layer(window) else {
            eprintln!("inset: a window's view has no layer; the view drawn in it is not shown");
            return ViewLayerFilling { observer: None };
        };
        ViewLayerFilling::in_layer(&parent, layer)
    }

    /// Places `layer` in `parent`, filling it, and keeps it filling it as `parent`'s bounds and
    /// scale change.
    fn in_layer(parent: &CALayer, layer: Arc<dyn CompositionLayer>) -> ViewLayerFilling {
        let observer = FillingObserver::alloc().set_ivars(Filling {
            parent: Weak::from(parent),
            layer,
        });
        // SAFETY: plain NSObject initialisation of an allocated instance.
        let observer: Retained<FillingObserver> = unsafe { msg_send![super(observer), init] };
        for key_path in FOLLOWED_KEY_PATHS {
            // SAFETY: the observer is removed in `Drop`, before it is let go. `Initial` places the
            // layer at once.
            unsafe {
                parent.addObserver_forKeyPath_options_context(
                    &observer,
                    &NSString::from_str(key_path),
                    NSKeyValueObservingOptions::Initial,
                    std::ptr::null_mut(),
                );
            }
        }
        ViewLayerFilling {
            observer: Some(observer),
        }
    }
}

impl Drop for ViewLayerFilling {
    /// Stops following the layer of winit's view, if it is still there: it may go before the hold.
    fn drop(&mut self) {
        let Some(observer) = &self.observer else {
            return;
        };
        let Some(parent) = observer.ivars().parent.load() else {
            return;
        };
        for key_path in FOLLOWED_KEY_PATHS {
            // SAFETY: added in `in_layer` for this observer and key path.
            unsafe { parent.removeObserver_forKeyPath(observer, &NSString::from_str(key_path)) };
        }
    }
}

/// What a [`FillingObserver`] places, and where.
struct Filling {
    /// The layer of winit's view.
    parent: Weak<CALayer>,
    /// The composition layer the view draws into.
    layer: Arc<dyn CompositionLayer>,
}

impl Filling {
    /// Places the layer again, filling the layer of winit's view at its bounds and scale now,
    /// inside the transaction that changed them.
    fn place(&self) {
        if let Some(parent) = self.parent.load() {
            place_filling(&parent, &self.layer, parent.contentsScale());
        }
    }
}

define_class!(
    /// Places a window's view's layer again whenever the layer of winit's view it observes changes
    /// its bounds or scale.
    //
    // SAFETY: NSObject has no subclassing requirements, and `FillingObserver` has no `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "InsetViewLayerFillingObserver"]
    #[ivars = Filling]
    struct FillingObserver;

    unsafe impl NSObjectProtocol for FillingObserver {}

    // SAFETY: the method has the signature `NSKeyValueObserving` declares.
    impl FillingObserver {
        #[unsafe(method(observeValueForKeyPath:ofObject:change:context:))]
        fn observe_value(
            &self,
            _key_path: Option<&NSString>,
            _object: Option<&AnyObject>,
            _change: Option<&NSDictionary<NSKeyValueChangeKey, AnyObject>>,
            _context: *mut c_void,
        ) {
            abort_on_panic(|| self.ivars().place());
        }
    }
);

#[cfg(test)]
mod tests {
    use objc2_foundation::{NSPoint, NSRect, NSSize};

    use super::*;

    /// The view's layer fills the layer of winit's view from the moment the hold is made, and
    /// follows its bounds and its scale as they change, before anything else hears of them.
    #[test]
    fn a_held_view_layer_follows_the_bounds_and_scale_of_the_windows_view() {
        let parent = CALayer::new();
        parent.setGeometryFlipped(true);
        parent.setContentsScale(2.0);
        parent.setBounds(rect(640.0, 480.0));
        let layer = CoreAnimationCompositor.make_layer(false);
        let view_layer = CoreAnimationViewLayer::on(&layer).unwrap();
        let filling = ViewLayerFilling::in_layer(&parent, Arc::clone(&layer));
        let placed = only_sublayer(&parent);
        assert_eq!(placed.frame(), rect(640.0, 480.0));

        parent.setBounds(rect(800.5, 600.0));
        assert_eq!(placed.frame(), rect(800.5, 600.0));

        parent.setContentsScale(1.0);
        assert_eq!(
            placed.frame(),
            rect(800.5, 600.0),
            "a scale of its own keeps it filling the same points"
        );

        drop(filling);
        parent.setBounds(rect(320.0, 240.0));
        assert_eq!(
            placed.frame(),
            rect(800.5, 600.0),
            "a dropped hold places nothing"
        );
        drop(view_layer);
    }

    fn rect(width: f64, height: f64) -> NSRect {
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height))
    }

    fn only_sublayer(parent: &CALayer) -> Retained<CALayer> {
        let sublayers = unsafe { parent.sublayers() }.expect("a sublayer");
        assert_eq!(sublayers.count(), 1);
        sublayers.objectAtIndex(0)
    }
}
