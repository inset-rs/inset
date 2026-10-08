//! One window's surfaces as the main thread holds them: placing a scene's surfaces in the
//! window's layer tree, beneath its content, and having their vsyncs follow the window.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use inset_embedder::{SurfacePlacement, ViewId};
use objc2::MainThreadMarker;
use winit::window::Window;

use super::super::ns_view;
use super::super::visibility::VisibilityNotices;
use super::window_vsync::PlacedSurfaces;
use super::{CoreAnimationSurface, in_transaction};
use crate::os::WindowSurfaces;

/// One window's surfaces, as the main thread holds them.
pub(crate) struct CoreAnimationWindowSurfaces {
    window: Arc<Window>,
    placed: Arc<PlacedSurfaces>,
    /// AppKit's notices that the window's visibility may have changed; `None`
    /// where AppKit gives none.
    _visibility_notices: Option<VisibilityNotices>,
}

impl WindowSurfaces for CoreAnimationWindowSurfaces {
    fn new(window: Arc<Window>) -> Self {
        let placed = Arc::new(PlacedSurfaces::new());
        let _visibility_notices =
            VisibilityNotices::observe(&window, follow_window_on_notice(&placed, &window));
        CoreAnimationWindowSurfaces {
            window,
            placed,
            _visibility_notices,
        }
    }

    /// Places `view`'s surfaces in the scene beneath the window's content, in paint order, hides
    /// the ones the scene no longer places, and has their vsyncs follow the window.
    fn place(&self, placements: &[SurfacePlacement], view: ViewId) {
        let surfaces = host_surfaces_in(placements, view);
        if surfaces.is_empty() && self.placed.is_empty() {
            return;
        }
        let Some(parent) = ns_view(&self.window).and_then(|view| view.layer()) else {
            eprintln!("inset: a window's view has no layer; its surfaces are not shown");
            return;
        };
        let window_scale = self.window.scale_factor();
        in_transaction(|| {
            for (index, (surface, placement)) in (0u32..).zip(&surfaces) {
                surface.place(&parent, index, placement, window_scale);
            }
        });
        self.replace_placed(surfaces.into_iter().map(|(surface, _)| surface).collect());
        self.follow_window();
    }

    fn follow_window(&self) {
        if let Some(main_thread) = MainThreadMarker::new() {
            self.placed.follow_window(&self.window, main_thread);
        }
    }
}

/// What a notice that `window`'s visibility may have changed does: the window's
/// surfaces follow it.
fn follow_window_on_notice(placed: &Arc<PlacedSurfaces>, window: &Arc<Window>) -> Box<dyn Fn()> {
    let placed = Arc::downgrade(placed);
    let window = Arc::downgrade(window);
    Box::new(move || {
        if let (Some(placed), Some(window), Some(main_thread)) =
            (placed.upgrade(), window.upgrade(), MainThreadMarker::new())
        {
            placed.follow_window(&window, main_thread);
        }
    })
}

impl CoreAnimationWindowSurfaces {
    /// Makes `surfaces` the window's, and hides the ones that no longer are.
    fn replace_placed(&self, surfaces: Vec<Arc<CoreAnimationSurface>>) {
        for surface in &surfaces {
            surface.attach_to_window(Arc::downgrade(&self.placed));
        }
        let previous = self.placed.replace_placed(surfaces.clone());
        in_transaction(|| {
            for gone in previous.iter().filter(|old| !contains(&surfaces, old)) {
                gone.hide();
            }
        });
    }
}

/// This embedder's surfaces a scene of `view` places, with their placements, in paint order.
fn host_surfaces_in(
    placements: &[SurfacePlacement],
    view: ViewId,
) -> Vec<(Arc<CoreAnimationSurface>, &SurfacePlacement)> {
    placements
        .iter()
        .filter_map(|placement| {
            let surface =
                CoreAnimationSurface::downcast_from(&placement.surface).or_else(|| {
                    report_foreign_surface();
                    None
                })?;
            surface
                .may_be_placed_in(view)
                .then_some((surface, placement))
        })
        .collect()
}

/// Reports, once, a placed surface this embedder did not open.
fn report_foreign_surface() {
    static REPORTED: AtomicBool = AtomicBool::new(false);
    if !REPORTED.swap(true, Ordering::Relaxed) {
        eprintln!(
            "inset: a scene placed a surface this host's compositor did not open; it is not shown"
        );
    }
}

fn contains(surfaces: &[Arc<CoreAnimationSurface>], surface: &Arc<CoreAnimationSurface>) -> bool {
    surfaces.iter().any(|placed| Arc::ptr_eq(placed, surface))
}
