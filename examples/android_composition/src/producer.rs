//! The producer: a thread of its own that waits for a refresh of its view's screen on the
//! platform's vsync waiter, draws the next frame into a buffer the compositor has released, and
//! presents it, as a video decoder or a program in a launcher would. Its buffers are allocated and
//! imported on the renderer's device before it starts, since the platform is the loop's; a refresh
//! at which none is free is skipped and counted. It draws each frame with valo on the GPU, and
//! presents once that work is done: `present` carries no fence.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, Weak};
use std::thread;
use std::time::{Duration, Instant};

use waft_embedder::image_codecs::PixelFormat;
use waft_embedder::valo;
use waft_embedder::{
    CompositionLayer, HardwareBuffer, HardwareBufferUsage, PlatformRef, ScreenId, VsyncObserver,
    VsyncWaiter,
};

/// How many buffers the producer draws into in turn.
const BUFFERS: usize = 3;

/// How long to wait for a refresh before counting it as missed: a little more than one refresh
/// at 60 Hz.
const PATIENCE: Duration = Duration::from_millis(20);

/// What a buffer the producer presents is allocated for: copied into by the GPU, and shown by the
/// display as an overlay.
const PRESENTED: HardwareBufferUsage = HardwareBufferUsage {
    gpu_sampled: true,
    gpu_color_output: true,
    composer_overlay: true,
    cpu_read: false,
};

/// The screen the layer's view is on, which the app keeps current and the producer paces by.
#[derive(Default)]
pub(crate) struct ViewScreen(Mutex<Option<ScreenId>>);

impl ViewScreen {
    pub(crate) fn set(&self, screen: Option<ScreenId>) {
        *self.0.lock().expect("held briefly") = screen;
    }

    fn get(&self) -> Option<ScreenId> {
        *self.0.lock().expect("held briefly")
    }
}

/// What the producer draws on, and what it paces by.
pub(crate) struct Pacing {
    pub(crate) layer: Arc<dyn CompositionLayer>,
    pub(crate) waiter: Arc<dyn VsyncWaiter>,
    pub(crate) screen: Arc<ViewScreen>,
}

/// What draws into a buffer through its texture, on the renderer's device.
pub(crate) struct Painter {
    pub(crate) device: wgpu::Device,
    context: valo::Context,
}

impl Painter {
    pub(crate) fn new(device: wgpu::Device, queue: wgpu::Queue) -> Painter {
        Painter {
            context: valo::Context::new(device.clone(), queue),
            device,
        }
    }

    /// Renders `picture` into `texture`, and waits for the GPU: `present` carries no fence.
    pub(crate) fn paint(&mut self, texture: &wgpu::Texture, picture: &valo::DisplayList) {
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let size = texture.size();
        self.context.render(
            picture,
            &valo::RenderTarget {
                view: &view,
                texture,
                format: texture.format(),
                size: [size.width, size.height],
                clear: Some(valo::Color::TRANSPARENT),
            },
        );
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
    }
}

/// What the producer counted, read by the screen.
#[derive(Default)]
pub(crate) struct Counts {
    presented: AtomicU64,
    released: AtomicU64,
    vsyncs: AtomicU64,
    /// Refreshes at which every buffer was still the compositor's.
    skipped: AtomicU64,
}

impl Counts {
    pub(crate) fn presented(&self) -> u64 {
        self.presented.load(Ordering::Relaxed)
    }

    pub(crate) fn released(&self) -> u64 {
        self.released.load(Ordering::Relaxed)
    }

    fn line(&self) -> String {
        format!(
            "{} vsyncs, {} presented, {} released, {} skipped with no buffer free",
            self.vsyncs.load(Ordering::Relaxed),
            self.presented(),
            self.released(),
            self.skipped.load(Ordering::Relaxed)
        )
    }
}

/// A buffer of the layer's and its texture on the renderer's device.
pub(crate) type Buffer = (Arc<dyn HardwareBuffer>, wgpu::Texture);

/// The producer's buffers, `size` pixels large, allocated and imported on `painter`'s device.
pub(crate) fn buffers(platform: &PlatformRef, painter: &Painter, size: [u32; 2]) -> Vec<Buffer> {
    let allocator = platform
        .hardware_buffer_allocator()
        .expect("the Android host allocates hardware buffers");
    (0..BUFFERS)
        .map(|_| {
            let buffer = allocator
                .allocate(size[0], size[1], PixelFormat::Rgba8, PRESENTED)
                .expect("a buffer of the layer's size");
            let texture = platform
                .import_texture(&*buffer, &painter.device)
                .expect("the renderer's device imports hardware buffers");
            (buffer, texture)
        })
        .collect()
}

pub(crate) fn start(
    pacing: Pacing,
    painter: Painter,
    buffers: Vec<Buffer>,
    size: [u32; 2],
    counts: Arc<Counts>,
) {
    thread::Builder::new()
        .name("producer".into())
        .spawn(move || Producer::new(pacing, painter, buffers, size, counts).run())
        .expect("a thread for the producer");
}

struct Producer {
    pacing: Pacing,
    painter: Painter,
    buffers: Vec<Buffer>,
    size: [u32; 2],
    /// Where a buffer's release hands its index back, and where the producer takes the next one
    /// to draw into.
    hand_back: Sender<usize>,
    free_buffers: Receiver<usize>,
    vsync: Arc<VsyncSignal>,
    vsyncs: Receiver<Instant>,
    counts: Arc<Counts>,
    frame: u32,
}

impl Producer {
    fn new(
        pacing: Pacing,
        painter: Painter,
        buffers: Vec<Buffer>,
        size: [u32; 2],
        counts: Arc<Counts>,
    ) -> Producer {
        let (hand_back, free_buffers) = mpsc::channel();
        for index in 0..buffers.len() {
            let _ = hand_back.send(index);
        }
        let (vsync, vsyncs) = VsyncSignal::new();
        Producer {
            pacing,
            painter,
            buffers,
            size,
            hand_back,
            free_buffers,
            vsync,
            vsyncs,
            counts,
            frame: 0,
        }
    }

    fn run(mut self) {
        let mut last_report = Instant::now();
        loop {
            if self.wait_for_vsync() {
                self.draw_next_frame();
            }
            if last_report.elapsed() >= Duration::from_secs(1) {
                crate::log::info(&self.counts.line());
                last_report = Instant::now();
            }
        }
    }

    /// Asks for the next refresh of the screen the view is on and waits for it; a view on no
    /// screen has none to wait for.
    fn wait_for_vsync(&self) -> bool {
        let Some(screen) = self.pacing.screen.get() else {
            thread::sleep(PATIENCE);
            return false;
        };
        let observer = Arc::downgrade(&self.vsync) as Weak<dyn VsyncObserver>;
        self.pacing.waiter.next_vsync(screen, observer);
        match self.vsyncs.recv_timeout(PATIENCE) {
            Ok(_) => {
                self.counts.vsyncs.fetch_add(1, Ordering::Relaxed);
                true
            }
            Err(RecvTimeoutError::Timeout) => false,
            Err(RecvTimeoutError::Disconnected) => unreachable!("the producer holds the signal"),
        }
    }

    /// Draws the next frame into a buffer the compositor has released and presents it; skips the
    /// refresh when none is.
    fn draw_next_frame(&mut self) {
        let Ok(index) = self.free_buffers.try_recv() else {
            self.counts.skipped.fetch_add(1, Ordering::Relaxed);
            return;
        };
        let (buffer, texture) = &self.buffers[index];
        let frame = stripe(self.size, self.frame);
        self.painter.paint(texture, &frame);
        let hand_back = self.hand_back.clone();
        let counts = Arc::clone(&self.counts);
        self.pacing.layer.present(
            Arc::clone(buffer),
            Vec::new(),
            Box::new(move || {
                counts.released.fetch_add(1, Ordering::Relaxed);
                let _ = hand_back.send(index);
            }),
        );
        self.counts.presented.fetch_add(1, Ordering::Relaxed);
        self.frame += 1;
    }
}

/// One frame: dark blue, a red border, and a green stripe that moves right by 8 pixels a frame.
fn stripe([width, height]: [u32; 2], frame: u32) -> valo::DisplayList {
    const BORDER: f32 = 8.0;
    const STRIPE: f32 = 40.0;
    let [width, height] = [width as f32, height as f32];
    let stripe_left = (frame * 8) as f32 % width;
    let mut picture = valo::DisplayListBuilder::new();
    let fill = |rgb: [f32; 3]| valo::Paint::from_color(valo::Color::rgb(rgb[0], rgb[1], rgb[2]));
    picture.draw_rect(
        valo::Rect::new(0.0, 0.0, width, height),
        &fill([0.9, 0.12, 0.12]),
    );
    let inside = valo::Rect::new(BORDER, BORDER, width - 2.0 * BORDER, height - 2.0 * BORDER);
    picture.draw_rect(inside, &fill([0.08, 0.16, 0.47]));
    picture.draw_rect(
        valo::Rect::new(stripe_left, BORDER, STRIPE, height - 2.0 * BORDER),
        &fill([0.16, 0.78, 0.24]),
    );
    picture.build()
}

/// A `side`-pixel square of four quarters: red, green, blue and yellow, from the top-left.
pub(crate) fn quarters(side: u32) -> valo::DisplayList {
    let half = side as f32 / 2.0;
    let mut picture = valo::DisplayListBuilder::new();
    let quarters = [
        (0.0, 0.0, valo::Color::rgb(1.0, 0.0, 0.0)),
        (half, 0.0, valo::Color::rgb(0.0, 1.0, 0.0)),
        (0.0, half, valo::Color::rgb(0.0, 0.0, 1.0)),
        (half, half, valo::Color::rgb(1.0, 1.0, 0.0)),
    ];
    for (left, top, colour) in quarters {
        picture.draw_rect(
            valo::Rect::new(left, top, half, half),
            &valo::Paint::from_color(colour),
        );
    }
    picture.build()
}

/// What the waiter calls at a refresh the producer asked for: one message to the producer.
struct VsyncSignal {
    vsyncs: Sender<Instant>,
}

impl VsyncSignal {
    fn new() -> (Arc<VsyncSignal>, Receiver<Instant>) {
        let (vsyncs, received) = mpsc::channel();
        (Arc::new(VsyncSignal { vsyncs }), received)
    }
}

impl VsyncObserver for VsyncSignal {
    fn vsync(&self, at: Instant, _display_time: Instant) {
        let _ = self.vsyncs.send(at);
    }
}
