//! Async wgpu + valo surface from an HTML canvas.
//!
//! Each frame draws straight onto the canvas's texture, cleared first: valo
//! copies a target out only when a frame keeps its pixels (`clear: None`), and
//! an advanced blend or a backdrop filter copies nothing.

use valo::{Color, DisplayList};
use waft_embedder::PreparedPrograms;
use web_sys::HtmlCanvasElement;

pub struct Gpu {
    pub surface: valo::Surface,
    pub context: valo::Context,
    /// The fragment programs the context has prepared for the canvas's format.
    prepared_programs: PreparedPrograms,
}

impl Gpu {
    pub async fn attach(canvas: HtmlCanvasElement, size: [u32; 2]) -> Gpu {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let wgpu_surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .expect("create wgpu canvas surface");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&wgpu_surface),
                ..Default::default()
            })
            .await
            .expect("request wgpu adapter");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("waft.web"),
                ..Default::default()
            })
            .await
            .expect("request wgpu device");
        let surface = valo::Surface::from_wgpu_surface(wgpu_surface, &adapter, &device, size);
        let mut context = valo::Context::new(device, queue);
        context.set_hide_missing_glyphs(true);
        Gpu {
            surface,
            context,
            prepared_programs: PreparedPrograms::new(),
        }
    }

    /// Configures the surface for `size` where it has another: configuring the canvas's context
    /// replaces what it shows, and its frame presents nothing when nothing changed.
    pub fn resize(&mut self, size: [u32; 2]) {
        if self.surface.size() == size {
            return;
        }
        self.surface.resize(size);
    }

    /// Draws `picture` on the canvas, the fragment programs made since the last draw prepared
    /// first.
    pub fn present(&mut self, picture: &DisplayList) {
        let format = self.surface.format();
        self.prepared_programs
            .prepare_new(|effect| self.context.prepare_runtime_effect(effect, format));
        let Ok(frame) = self.surface.acquire() else {
            return;
        };
        self.context
            .render(picture, &frame.target(Some(Color::WHITE)));
        self.context.present(frame);
    }
}
