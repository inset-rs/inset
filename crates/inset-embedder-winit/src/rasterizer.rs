//! Drawing a view's scenes on its window's surface, as Flutter's `Rasterizer` draws its layer
//! trees: the last scene the framework handed over, whether the surface shows it or is owed it,
//! and what to do when the surface refuses one.
//!
//! On macOS wgpu asks AppKit whether the window is visible, its occlusion state, before
//! it asks the layer for a drawable, and refuses with `Occluded` while it is not (wgpu
//! issue 8309). A window just ordered on screen counts as occluded until AppKit reports
//! otherwise, which it does through `NSWindowDidChangeOcclusionState`, winit's
//! `Occluded(false)`: that event is when a refused picture is presented, as wgpu's own
//! guidance says. The layer's other refusals are a drawable timeout, which the next
//! vsync clears, and a lost surface. Flutter's macOS embedder never meets any of this,
//! as it composites IOSurfaces through layers Core Animation retains; gpui drops a refused
//! frame and lets its display link draw the next. This embedder keeps the framework's last
//! scene instead, since the framework draws only when asked, and draws it again when the
//! system says the window can take it, as Flutter's `DrawLastLayerTree` does.
//!
//! The surface is the system's to give and take, winit's `Resumed` and `Suspended`: a
//! rasterizer starts without one, and one taken away is made again on the same window when the
//! system gives it back. Without a surface nothing is drawn: every draw is refused as lost, and
//! the last scene is owed until a surface is given.

use std::cell::RefCell;
use std::sync::Arc;

use inset_embedder::{Picture, Scene};
use winit::window::Window;

use crate::gpu::Gpu;
use crate::os::{FramePresentation, ThisOs, WindowGeometry};

/// At how many vsyncs a scene is drawn again after the surface timed out on it.
const TIMEOUT_RETRIES: u8 = 8;

/// One window's surface and what the framework last drew on it.
pub(crate) struct Rasterizer {
    /// The device every window draws with, which the surface is made on.
    gpu: Gpu,
    /// `None` until the system gives the window a surface, and again once it takes it away:
    /// every draw is refused as lost meanwhile.
    surface: Option<valo::Surface>,
    context: valo::Context,
    /// What a frame starts from: white, or nothing for a window that shows what is behind it.
    clear: valo::Color,
    /// Whether the compositor honours the content's alpha: from the start for a window that
    /// sees through, and from the first scene that places a native surface for any other.
    /// Kept for each surface the system gives.
    content_alpha: valo::SurfaceAlpha,
    last_scene: LastScene,
}

impl Rasterizer {
    /// A rasterizer on `gpu`'s device, with no surface until the system gives one. A window
    /// that sees through gets surfaces whose alpha the compositor honours, cleared to nothing,
    /// so what the app leaves unpainted shows what is behind the window.
    pub(crate) fn new(gpu: &Gpu, sees_through: bool) -> Rasterizer {
        let content_alpha = if sees_through {
            valo::SurfaceAlpha::Transparent
        } else {
            valo::SurfaceAlpha::Opaque
        };
        let mut context = valo::Context::new(gpu.device.clone(), gpu.queue.clone());
        context.set_hide_missing_glyphs(true);
        let clear = if sees_through {
            valo::Color::TRANSPARENT
        } else {
            valo::Color::WHITE
        };
        Rasterizer {
            gpu: gpu.clone(),
            surface: None,
            context,
            clear,
            content_alpha,
            last_scene: LastScene::default(),
        }
    }

    /// The renderer's image context, for the images decoded on this device.
    pub(crate) fn image_context(&self) -> valo::ImageContext {
        self.context.image_context()
    }

    /// Whether the surface shows `scene` already, so drawing it would change nothing.
    pub(crate) fn shows(&self, scene: &Scene) -> bool {
        self.last_scene.shows(scene)
    }

    /// Draws `scene`, and keeps it as the last one, to draw again while the surface does not
    /// show it.
    pub(crate) fn draw(&mut self, scene: &Arc<Scene>) {
        self.last_scene.hand_over(scene);
        self.keep_alpha_once_surfaces_are_placed(scene);
        self.draw_picture(&scene.picture);
    }

    /// Draws the last scene again if the surface does not show it: an owed frame, paid
    /// without the framework.
    pub(crate) fn draw_last_scene(&mut self) {
        if let Some(owed) = self.last_scene.owed() {
            self.draw_picture(&owed.picture);
        }
    }

    /// Whether the next vsync should draw the owed scene again: the surface timed out, and
    /// not too many vsyncs ago. An occluded window waits for the system's word instead, and a
    /// view without a surface for the system to give one.
    pub(crate) fn retries_at_vsync(&self) -> bool {
        self.last_scene.retries_at_vsync()
    }

    /// Draws the owed scene again, at a vsync.
    pub(crate) fn retry_owed_scene(&mut self) {
        self.last_scene.count_retry();
        self.draw_last_scene();
    }

    /// The surface was reconfigured to `size`: whatever it showed is gone, and whatever it
    /// refused is forgotten with it. Without a surface there is nothing to reconfigure.
    pub(crate) fn resized(&mut self, size: [u32; 2]) {
        let Some(surface) = &mut self.surface else {
            return;
        };
        surface.resize(size);
        self.last_scene.surface_reset();
    }

    /// The system gives `window` a surface: one is made on it, at the size the window has now,
    /// owed the last scene.
    pub(crate) fn give_surface(&mut self, window: &Arc<Window>) -> Result<(), String> {
        self.surface = Some(make_surface(&self.gpu, window, self.content_alpha)?);
        self.last_scene.surface_reset();
        Ok(())
    }

    /// The system took the window's surface away: the last scene is kept and owed until a
    /// surface is given again.
    pub(crate) fn take_surface(&mut self) {
        self.surface = None;
        self.last_scene.surface_taken();
    }

    /// The window closed: its surface goes, which is the rasterizer's hold on the window, and
    /// the last scene with it, which may hold native surfaces a producer would reuse.
    pub(crate) fn window_closed(&mut self) {
        self.surface = None;
        self.last_scene = LastScene::default();
    }

    /// Whether the surface presents through the window's own transaction, so that a frame
    /// and the geometry it was laid out for are committed together.
    fn set_presents_in_transaction(&mut self, wanted: bool) {
        if let Some(surface) = &mut self.surface {
            ThisOs::present_in_transaction(surface, wanted);
        }
    }

    /// Renders and presents `picture`, or records why the surface refused it, so that the
    /// picture is presented when the refusal ends.
    fn draw_picture(&mut self, picture: &Picture) {
        let presented = self.render_and_present(picture);
        self.last_scene.presented(presented);
    }

    /// Renders `picture` into the surface's next drawable and presents it. `Err` is the
    /// surface refusing a drawable.
    fn render_and_present(&mut self, picture: &Picture) -> Result<(), valo::Refused> {
        let Some(surface) = &mut self.surface else {
            return Err(valo::Refused::Lost);
        };
        let surface_frame = surface.acquire()?;
        self.context
            .render(picture, &surface_frame.target(Some(self.clear)));
        self.context.present(surface_frame);
        Ok(())
    }

    /// The first scene that places a native surface in the window makes its content keep its
    /// alpha from then on, so the surfaces beneath show through the rectangles the framework
    /// leaves clear. The window itself stays opaque, and a window that never shows a surface
    /// keeps an opaque content layer the compositor need not blend. It does not switch back
    /// when the surfaces go: a launcher's tabs come and go, and each switch reconfigures the
    /// surface.
    fn keep_alpha_once_surfaces_are_placed(&mut self, scene: &Scene) {
        if scene.surfaces.is_empty() || self.content_alpha == valo::SurfaceAlpha::Transparent {
            return;
        }
        self.content_alpha = valo::SurfaceAlpha::Transparent;
        if let Some(surface) = &mut self.surface {
            surface.set_alpha(valo::SurfaceAlpha::Transparent);
        }
    }
}

/// Presents through the window's own transaction while it lives: for the frame drawn inside a
/// resize, so that frame and the geometry it was laid out for are committed together. Off in
/// the ordinary run of frames, since it makes every present wait to be scheduled; gpui does the
/// same for its synchronous draws.
pub(crate) struct PresentingInTransaction<'a>(&'a RefCell<Rasterizer>);

impl<'a> PresentingInTransaction<'a> {
    pub(crate) fn begin(rasterizer: &'a RefCell<Rasterizer>) -> PresentingInTransaction<'a> {
        rasterizer.borrow_mut().set_presents_in_transaction(true);
        PresentingInTransaction(rasterizer)
    }
}

impl Drop for PresentingInTransaction<'_> {
    fn drop(&mut self) {
        self.0.borrow_mut().set_presents_in_transaction(false);
    }
}

/// The last scene the framework handed over, Flutter's `last_layer_tree_`, and whether the
/// surface shows it: kept to draw again when the window can take it, and to tell a scene equal
/// to it from one worth drawing.
#[derive(Default)]
struct LastScene {
    scene: Option<Arc<Scene>>,
    /// Whether the surface shows the scene: not while the window is hidden or occluded, when
    /// the surface gives no texture, nor after the surface was resized.
    on_screen: bool,
    /// Why the surface last refused the scene, while it still does not show it: what brings it
    /// there is the system's word for an occluded window and the next vsync after a timeout.
    refused: Option<valo::Refused>,
    /// Vsyncs tried at since a timeout; a surface that keeps timing out is left to the
    /// system's next redraw request rather than asked at every vsync for ever.
    retries: u8,
}

impl LastScene {
    /// Whether the surface shows `scene`: the last scene equals it and is on screen.
    fn shows(&self, scene: &Scene) -> bool {
        self.on_screen && self.scene.as_deref().is_some_and(|last| last == scene)
    }

    /// Takes `scene` as the last one, which the surface shows once it is presented.
    fn hand_over(&mut self, scene: &Arc<Scene>) {
        self.scene = Some(Arc::clone(scene));
        self.on_screen = false;
    }

    /// The last scene was presented, or the surface refused it.
    fn presented(&mut self, result: Result<(), valo::Refused>) {
        match result {
            Ok(()) => {
                self.on_screen = true;
                self.refused = None;
                self.retries = 0;
            }
            Err(refused) => {
                self.on_screen = false;
                self.refused = Some(refused);
            }
        }
    }

    /// The last scene, if one was handed over and the surface does not show it.
    fn owed(&self) -> Option<Arc<Scene>> {
        if self.on_screen {
            return None;
        }
        self.scene.clone()
    }

    fn retries_at_vsync(&self) -> bool {
        self.owed().is_some()
            && self.refused == Some(valo::Refused::Timeout)
            && self.retries < TIMEOUT_RETRIES
    }

    fn count_retry(&mut self) {
        self.retries = self.retries.saturating_add(1);
    }

    /// The surface shows nothing now, and has refused nothing.
    fn surface_reset(&mut self) {
        self.on_screen = false;
        self.refused = None;
        self.retries = 0;
    }

    fn surface_taken(&mut self) {
        self.surface_reset();
        self.refused = Some(valo::Refused::Lost);
    }
}

/// A surface on `gpu`'s device for `window`, as it is presented to, with the compositor
/// honouring its alpha or not.
fn make_surface(
    gpu: &Gpu,
    window: &Arc<Window>,
    alpha: valo::SurfaceAlpha,
) -> Result<valo::Surface, String> {
    valo::Surface::new_with_options(
        &gpu.instance,
        &gpu.adapter,
        &gpu.device,
        Arc::clone(window),
        ThisOs::surface_size(window),
        valo::SurfaceOptions::default().with_alpha(alpha),
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use inset_embedder::{Canvas, Offset, Paint, Rect, SceneBuilder};

    use super::*;

    fn empty_scene() -> Arc<Scene> {
        Arc::new(SceneBuilder::new().build())
    }

    fn a_square_scene() -> Arc<Scene> {
        let mut canvas = Canvas::new();
        canvas.draw_rect(Rect::from_ltwh(0.0, 0.0, 10.0, 10.0), &Paint::default());
        let mut builder = SceneBuilder::new();
        builder.add_picture(Offset::ZERO, &Arc::new(canvas.build()), false, false);
        Arc::new(builder.build())
    }

    #[test]
    fn a_scene_equal_to_the_one_on_screen_is_not_drawn_again() {
        let mut last = LastScene::default();
        assert!(!last.shows(&empty_scene()));
        last.hand_over(&empty_scene());
        last.presented(Ok(()));
        assert!(last.shows(&empty_scene()));
        assert!(!last.shows(&a_square_scene()));
    }

    #[test]
    fn an_equal_scene_is_drawn_while_the_surface_does_not_show_the_last() {
        let mut last = LastScene::default();
        last.hand_over(&empty_scene());
        last.presented(Err(valo::Refused::Occluded));
        assert!(!last.shows(&empty_scene()));
    }

    #[test]
    fn a_refused_scene_is_owed_until_it_is_drawn() {
        let mut last = LastScene::default();
        assert!(last.owed().is_none(), "nothing handed over");
        last.hand_over(&empty_scene());
        last.presented(Err(valo::Refused::Occluded));
        assert!(last.owed().is_some());
        last.presented(Ok(()));
        assert!(last.owed().is_none());
    }

    #[test]
    fn only_a_timeout_is_retried_at_vsyncs_and_only_so_often() {
        let mut last = LastScene::default();
        last.hand_over(&empty_scene());
        last.presented(Err(valo::Refused::Occluded));
        assert!(
            !last.retries_at_vsync(),
            "an occluded window waits for AppKit"
        );
        last.presented(Err(valo::Refused::Timeout));
        for _ in 0..TIMEOUT_RETRIES {
            assert!(last.retries_at_vsync());
            last.count_retry();
            last.presented(Err(valo::Refused::Timeout));
        }
        assert!(!last.retries_at_vsync(), "left to the system's next redraw");
    }

    #[test]
    fn a_resized_surface_owes_the_last_scene_and_forgets_its_refusal() {
        let mut last = LastScene::default();
        last.hand_over(&empty_scene());
        last.presented(Err(valo::Refused::Timeout));
        last.count_retry();
        last.surface_reset();
        assert!(last.owed().is_some());
        assert!(!last.retries_at_vsync(), "nothing refused since");
        last.presented(Err(valo::Refused::Timeout));
        assert!(last.retries_at_vsync(), "the count starts again");
    }

    #[test]
    fn a_taken_surface_draws_nothing_until_one_is_given_again() {
        let mut last = LastScene::default();
        last.hand_over(&empty_scene());
        last.presented(Ok(()));

        last.surface_taken();
        assert!(last.owed().is_some(), "what the surface showed is owed");
        assert!(
            !last.retries_at_vsync(),
            "a view without a surface waits for one"
        );
        assert!(
            !last.shows(&empty_scene()),
            "an equal scene is drawn while nothing shows"
        );
        last.hand_over(&empty_scene());
        last.presented(Err(valo::Refused::Lost));
        assert!(last.owed().is_some(), "without a surface nothing is drawn");

        last.surface_reset();
        assert!(last.owed().is_some(), "a given surface shows nothing yet");
        last.presented(Ok(()));
        assert!(last.owed().is_none(), "and draws again");
    }

    #[test]
    fn a_view_that_never_had_a_surface_owes_what_it_was_handed() {
        let mut last = LastScene::default();
        last.hand_over(&empty_scene());
        last.presented(Err(valo::Refused::Lost));
        assert!(last.owed().is_some());
        assert!(!last.retries_at_vsync(), "it waits for its first surface");
    }
}
