//! The host's [`View`]: the activity's window as the framework sees it.
//!
//! One activity has one window, so the host has one view, and it outlives the windows the
//! system hands out: the framework keeps its tree and its last picture while the app is in
//! the background, and the picture is presented again on the window that comes back.

use std::cell::{Cell, RefCell};
use std::sync::Arc;

use ndk::native_window::NativeWindow;
use waft_embedder::{
    Scene, ScreenId, SystemMouseCursorKind, TextInputHost, View, ViewConstraints,
    ViewFocusDirection, ViewFocusState, ViewHost, ViewId, ViewMetrics, ViewPadding,
};

use crate::composition::SurfaceControlCompositor;
use crate::gpu::Gpu;
use crate::picture::ViewPicture;
use crate::text_input::AndroidTextInput;

/// The one view the activity has.
pub(crate) const VIEW: ViewId = ViewId(0);

pub(crate) struct AndroidView {
    /// How the picture reaches the screen (`picture.rs`).
    picture: RefCell<ViewPicture>,
    /// The renderer's image store, on the device the picture is drawn with.
    images: valo::ImageContext,
    metrics: Cell<ViewMetrics>,
    /// What a frame starts from.
    clear: valo::Color,
    /// The last picture the framework handed over, kept to present again on the window the
    /// system gives back, and to tell a picture equal to it from one worth drawing.
    latest: RefCell<Option<Arc<Scene>>>,
    /// Whether the latest picture is on the screen: not while there is no window, and not
    /// when the screen refused it.
    on_screen: Cell<bool>,
    /// The system's bars, told from the keyboard in the one padding the activity reports.
    bars: Cell<Bars>,
    /// The keyboards' way in, where the activity is Waft's (`text_input.rs`).
    text_input: Option<AndroidTextInput>,
}

impl AndroidView {
    /// A view with no window yet: it draws nothing until the system gives one. Its picture is a
    /// composition layer's content on `compositor`, where there is one, and its text is typed
    /// through `text_input`, where there is one.
    pub(crate) fn new(
        gpu: &Gpu,
        compositor: Option<&SurfaceControlCompositor>,
        text_input: Option<AndroidTextInput>,
    ) -> AndroidView {
        let mut context = valo::Context::new(gpu.device.clone(), gpu.queue.clone());
        context.set_hide_missing_glyphs(true);
        let images = context.image_context();
        AndroidView {
            picture: RefCell::new(ViewPicture::new(gpu, compositor, context)),
            images,
            metrics: Cell::new(ViewMetrics::default()),
            clear: valo::Color::WHITE,
            latest: RefCell::new(None),
            on_screen: Cell::new(false),
            bars: Cell::new(Bars::default()),
            text_input,
        }
    }

    /// The renderer's image store, for the images decoded on this device.
    pub(crate) fn image_context(&self) -> valo::ImageContext {
        self.images.clone()
    }

    /// The system gave the activity a window: the picture reaches it, owed whatever the framework
    /// last drew.
    pub(crate) fn window_arrived(&self, gpu: &Gpu, window: &NativeWindow, size: [u32; 2]) {
        let arrived = self.picture.borrow_mut().window_arrived(gpu, window, size);
        if let Err(error) = arrived {
            crate::log::warn_to_log!("no picture on the window given: {error}");
        }
        self.on_screen.set(false);
    }

    /// The system took the window away: the view keeps its picture and owes it.
    pub(crate) fn window_gone(&self) {
        self.picture.borrow_mut().window_gone();
        self.on_screen.set(false);
    }

    /// The window changed size: the picture follows it, and whatever it showed is gone.
    pub(crate) fn resized(&self, size: [u32; 2]) {
        self.picture.borrow_mut().resized(size);
        self.on_screen.set(false);
    }

    /// Presents the latest picture again if it is not on the screen: an owed frame, paid
    /// without the framework drawing another.
    pub(crate) fn represent(&self) {
        if self.on_screen.get() {
            return;
        }
        let latest = self.latest.borrow().clone();
        if let Some(latest) = latest {
            self.draw(&latest);
        }
    }

    /// Draws `scene`, its composition layers beneath it where the picture is composited.
    fn draw(&self, scene: &Scene) {
        let pixel_ratio = self.metrics.get().device_pixel_ratio;
        let drawn = self
            .picture
            .borrow_mut()
            .draw(scene, self.clear, pixel_ratio);
        self.on_screen.set(drawn.is_ok());
    }

    /// Takes the metrics the system now reports and answers whether they changed. The
    /// activity's content rect moves when the bars or the keyboard do, and the glue reports
    /// no event for it, so the host reads it again after every turn.
    pub(crate) fn refresh_metrics(&self, geometry: Geometry) -> bool {
        let metrics = self.read_metrics(geometry);
        if self.metrics.get() == metrics {
            return false;
        }
        self.metrics.set(metrics);
        true
    }

    fn read_metrics(&self, geometry: Geometry) -> ViewMetrics {
        let [width, height] = geometry.size.map(f64::from);
        let content = geometry.content_padding([width, height]);
        let bars = self.bars.get().update([width, height], content);
        self.bars.set(bars);
        let (padding, view_padding, view_insets) = split_padding(content, bars.padding);
        ViewMetrics {
            physical_size: [width, height],
            physical_constraints: ViewConstraints::tight(width, height),
            device_pixel_ratio: geometry.density,
            padding,
            view_padding,
            view_insets,
            screen: geometry.screen,
        }
    }
}

impl View for AndroidView {
    fn id(&self) -> ViewId {
        VIEW
    }

    fn metrics(&self) -> ViewMetrics {
        self.metrics.get()
    }

    fn present(&self, scene: Arc<Scene>) {
        let unchanged = self.on_screen.get()
            && self
                .latest
                .borrow()
                .as_ref()
                .is_some_and(|latest| **latest == *scene);
        if unchanged {
            return;
        }
        *self.latest.borrow_mut() = Some(Arc::clone(&scene));
        self.draw(&scene);
    }

    fn host(&self) -> &dyn ViewHost {
        self
    }
}

/// The view is its own host, which takes text through the system's keyboards; this embedder has
/// no cursor and no focus to move yet.
impl ViewHost for AndroidView {
    fn text_input(&self) -> Option<&dyn TextInputHost> {
        self.text_input
            .as_ref()
            .map(|text_input| text_input as &dyn TextInputHost)
    }

    fn activate_system_cursor(&self, _device: i64, _kind: SystemMouseCursorKind) {}

    fn request_focus_change(&self, _state: ViewFocusState, _direction: ViewFocusDirection) {}
}

/// What the system says about the window this turn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Geometry {
    /// The window's pixels.
    pub(crate) size: [u32; 2],
    /// Pixels per logical pixel, from the configuration's density.
    pub(crate) density: f64,
    /// The rectangle the activity's content was laid out in, within the window.
    pub(crate) content: [i32; 4],
    /// The screen the window is on, once the system has described it.
    pub(crate) screen: Option<ScreenId>,
}

impl Geometry {
    /// What the content rect leaves on each side. Empty before the activity's first
    /// layout, which is no padding at all.
    fn content_padding(self, [width, height]: [f64; 2]) -> ViewPadding {
        let [left, top, right, bottom] = self.content;
        if right <= left || bottom <= top {
            return ViewPadding::ZERO;
        }
        ViewPadding {
            left: f64::from(left).max(0.0),
            top: f64::from(top).max(0.0),
            right: (width - f64::from(right)).max(0.0),
            bottom: (height - f64::from(bottom)).max(0.0),
        }
    }
}

/// The system's bars, as the least the content has been padded at the bottom at one window
/// size: the bars do not move while the window keeps its size, and the one thing that pads
/// the content more is the keyboard the window was resized for. A new size, a rotation,
/// starts over.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Bars {
    size: [f64; 2],
    padding: ViewPadding,
}

impl Default for Bars {
    fn default() -> Bars {
        Bars {
            size: [0.0, 0.0],
            padding: ViewPadding::ZERO,
        }
    }
}

impl Bars {
    fn update(self, size: [f64; 2], content: ViewPadding) -> Bars {
        // A keyboard only ever pads the bottom, so anything else changing is a new safe
        // area and not a keyboard: a new window size, or a different top, left or right.
        // Without that, a reading taken before the window is laid out, which is all zeros,
        // would be kept as the bars and the real safe area arriving later would be read as
        // a keyboard for as long as the size held.
        let same_bars = size == self.size
            && content.top == self.padding.top
            && content.left == self.padding.left
            && content.right == self.padding.right;
        if same_bars && content.bottom > self.padding.bottom {
            self
        } else {
            Bars {
                size,
                padding: content,
            }
        }
    }
}

/// Flutter's three paddings from the one the activity reports: `bars` is the system's own,
/// `viewPadding`; what the content lost at the bottom beyond them is the keyboard,
/// `viewInsets`; and `padding` is the bars less what the keyboard covers, since
/// `MediaQuery.padding` excludes the insets.
fn split_padding(
    content: ViewPadding,
    bars: ViewPadding,
) -> (ViewPadding, ViewPadding, ViewPadding) {
    let keyboard = (content.bottom - bars.bottom).max(0.0);
    let padding = ViewPadding {
        bottom: (bars.bottom - keyboard).max(0.0),
        ..bars
    };
    let view_insets = ViewPadding {
        bottom: keyboard,
        ..ViewPadding::ZERO
    };
    (padding, bars, view_insets)
}
