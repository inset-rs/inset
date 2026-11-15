//! How a view's picture reaches the screen: drawn with valo into a `CAMetalLayer` of the view's own,
//! the content of a composition layer the host's compositor made (`waft-embedder-apple`'s
//! `CoreAnimationViewLayer`), with the composition layers each scene places beneath it, as macOS's
//! views draw, and as the native-surfaces decision has every view on Core Animation draw. The
//! composition layer fills the layer of the view's `UIView`, placed again as that view is laid out,
//! in the transaction that lays it out.
//!
//! The surface is made at the first draw that has a size, and goes with the scene: a view whose
//! scene disconnected holds no drawable, and the next scene's first draw makes one again.

use std::sync::Arc;

use objc2_quartz_core::CALayer;
use waft_embedder::{CompositionLayer, Compositor, PreparedPrograms, Scene, ViewId};
use waft_embedder_apple::composition::{
    CoreAnimationCompositor, CoreAnimationViewLayer, place_filling,
};

use crate::gpu::Gpu;

/// What a frame starts from.
const CLEAR: valo::Color = valo::Color::WHITE;

pub(crate) struct ViewPicture {
    /// The composition layer the view draws into, which its `UIView`'s layer places.
    layer: Arc<dyn CompositionLayer>,
    /// Its content: the `CAMetalLayer` drawn into, with the scenes' composition layers beneath.
    drawing: CoreAnimationViewLayer,
    gpu: Gpu,
    context: valo::Context,
    /// The fragment programs the context has prepared for the surface's format.
    prepared_programs: PreparedPrograms,
    /// Made at the first draw with a size; `None` while the view is in no scene.
    surface: Option<valo::Surface>,
    /// The drawing's pixels.
    size: [u32; 2],
    /// Opaque until a scene places a composition layer, which shows through the room made for it.
    alpha: valo::SurfaceAlpha,
}

impl ViewPicture {
    /// The picture of a view, drawn on `gpu`'s device, in no scene yet.
    pub(crate) fn new(gpu: &Gpu) -> ViewPicture {
        let layer = CoreAnimationCompositor.make_layer(false);
        let drawing =
            CoreAnimationViewLayer::on(&layer).expect("a layer this host's compositor made");
        drawing.show_background_color(CLEAR);
        let mut context = valo::Context::new(gpu.device.clone(), gpu.queue.clone());
        context.set_hide_missing_glyphs(true);
        ViewPicture {
            layer,
            drawing,
            gpu: gpu.clone(),
            context,
            prepared_programs: PreparedPrograms::new(),
            surface: None,
            size: [0, 0],
            alpha: valo::SurfaceAlpha::Opaque,
        }
    }

    /// Places the picture filling `parent`, the layer of the view's `UIView`, at `scale` pixels to
    /// its point, `size` pixels large: the drawing follows. Main thread, inside the layout's
    /// transaction.
    pub(crate) fn fill(&mut self, parent: &CALayer, scale: f64, size: [u32; 2]) {
        place_filling(parent, &self.layer, scale);
        self.drawing.set_drawing_scale(scale);
        if self.size != size {
            self.size = size;
            if let Some(surface) = &mut self.surface {
                surface.resize(size);
            }
        }
    }

    /// The view's scene disconnected: what the picture was drawn into goes with it.
    pub(crate) fn drop_surface(&mut self) {
        self.surface = None;
    }

    /// Draws `scene` of the view numbered `view`, its composition layers placed beneath it; with
    /// no frame reader here, a video drawn from frames shows none. `Err` is the picture refused.
    pub(crate) fn draw(&mut self, scene: &Scene, view: ViewId) -> Result<(), valo::Refused> {
        let slices = waft_embedder::slices(scene, |_reader| None);
        self.drawing
            .place_scene_layers(slices.composition_layers(), view);
        self.keep_alpha_once_layers_are_placed(slices.composition_layers().next().is_some());
        self.make_surface_if_missing();
        let Some(surface) = self.surface.as_mut() else {
            return Err(valo::Refused::Lost);
        };
        let format = surface.format();
        let context = &mut self.context;
        self.prepared_programs
            .prepare_new(|effect| context.prepare_runtime_effect(effect, format));
        let frame = surface.acquire()?;
        self.context
            .render(&slices.picture_with_room(), &frame.target(Some(CLEAR)));
        self.context.present(frame);
        Ok(())
    }

    /// Makes the surface if the view has none and has a size.
    fn make_surface_if_missing(&mut self) {
        if self.surface.is_none() && self.size.iter().all(|side| *side > 0) {
            let made = self.drawing.make_surface_on(
                &self.gpu.instance,
                &self.gpu.adapter,
                &self.gpu.device,
                self.size,
                self.alpha,
            );
            match made {
                Ok(surface) => self.surface = Some(surface),
                Err(reason) => eprintln!("waft: no surface on a view's layer: {reason}"),
            }
        }
    }

    /// The first scene that places a composition layer makes the drawing keep its alpha from then
    /// on, as the winit host's views do (`rasterizer.rs`).
    fn keep_alpha_once_layers_are_placed(&mut self, places_layers: bool) {
        if !places_layers || self.alpha == valo::SurfaceAlpha::Transparent {
            return;
        }
        self.alpha = valo::SurfaceAlpha::Transparent;
        if let Some(surface) = &mut self.surface {
            surface.set_alpha(valo::SurfaceAlpha::Transparent);
        }
    }
}
