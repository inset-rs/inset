//! Async wgpu + valo surface from an HTML canvas.
//!
//! The canvas swapchain is `RENDER_ATTACHMENT` only. Advanced blends and
//! backdrop filters copy the target, so the frame is drawn into a backing
//! texture that has `COPY_SRC`, then blitted 1:1 onto the swapchain — the same
//! path Valo's `PersistentCanvas` uses on WebGL.

use valo::{
    BlendMode, Color, DisplayList, DisplayListBuilder, Filter, MipmapMode, Paint, Rect, Sampling,
    TileMode,
};
use web_sys::HtmlCanvasElement;

pub struct Gpu {
    pub surface: valo::Surface,
    pub context: valo::Context,
    backing: Option<valo::Image>,
    backing_size: [u32; 2],
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
                label: Some("inset.web"),
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
            backing: None,
            backing_size: [0, 0],
        }
    }

    pub fn resize(&mut self, size: [u32; 2]) {
        self.surface.resize(size);
        self.backing = None;
    }

    pub fn present(&mut self, picture: &DisplayList) {
        let size = self.surface.size();
        let format = self.surface.format();
        self.ensure_backing(size, format);
        let backing = self.backing.as_ref().expect("backing").clone();
        self.context.render(
            picture,
            &valo::RenderTarget {
                view: backing.view(),
                texture: backing.texture(),
                format,
                size,
                clear: Some(Color::WHITE),
            },
        );
        let Some(frame) = self.surface.acquire() else {
            return;
        };
        let mut builder = DisplayListBuilder::new();
        let source = Rect::new(0.0, 0.0, backing.width(), backing.height());
        let destination = Rect::new(0.0, 0.0, size[0] as f32, size[1] as f32);
        builder.draw_image_rect(
            &backing,
            source,
            destination,
            exact_sampling(),
            &copy_paint(),
        );
        self.context
            .render(&builder.build(), &frame.target(Some(Color::WHITE)));
        self.context.present(frame);
    }

    fn ensure_backing(&mut self, size: [u32; 2], format: wgpu::TextureFormat) {
        if self.backing.is_some() && self.backing_size == size {
            return;
        }
        let texture = self
            .context
            .device()
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("inset.web.backing"),
                size: wgpu::Extent3d {
                    width: size[0].max(1),
                    height: size[1].max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
        self.backing = Some(
            self.context
                .import_image(texture, [size[0].max(1), size[1].max(1)]),
        );
        self.backing_size = size;
    }
}

fn exact_sampling() -> Sampling {
    Sampling {
        filter: Filter::Nearest,
        mipmap: MipmapMode::None,
        tile_x: TileMode::Clamp,
        tile_y: TileMode::Clamp,
    }
}

fn copy_paint() -> Paint {
    Paint {
        color: Color::WHITE,
        blend_mode: BlendMode::Src,
        ..Paint::default()
    }
}
