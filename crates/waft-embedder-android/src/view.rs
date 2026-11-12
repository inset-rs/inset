//! The host's [`View`]: an activity's window as the framework sees it.
//!
//! Each activity has one window, which is one view, and the view outlives the windows the system
//! hands the activity: the framework keeps its tree and its last picture while the activity is in
//! the background, and the picture is presented again on the window that comes back.
//!
//! What is one window's is kept here, by view, and nowhere for the app: the window and its size,
//! what the system covers of it, the rectangle its content was laid out in, its picture, its text
//! input, and the activity it is of. A later editor must keep it so: the loop reaches a view
//! through the registry (`views.rs`) by the id an event carries, and what it finds there must be
//! that window's and no other's.
//!
//! While the view has no window it holds no buffers: what its picture was drawn into goes with the
//! window (`picture.rs`), and the scene it kept is drawn again into new ones when a window arrives.

use std::cell::{Cell, RefCell};
use std::sync::Arc;

use ndk::native_window::NativeWindow;
use waft_embedder::{
    Scene, ScreenId, SystemMouseCursorKind, TextInputHost, View, ViewConstraints,
    ViewFocusDirection, ViewFocusState, ViewHost, ViewId, ViewMetrics, ViewPadding,
};

use crate::activity::Activity;
use crate::composition::SurfaceControlCompositor;
use crate::gpu::Gpu;
use crate::picture::ViewPicture;
use crate::text_input::AndroidTextInput;
use crate::window_insets::WindowInsets;

pub(crate) struct AndroidView {
    id: ViewId,
    /// The activity whose window the view is.
    activity: Activity,
    /// The window, while the system gives the activity one.
    window: RefCell<Option<NativeWindow>>,
    /// What the system covers of the window, as its insets listener last told
    /// (`window_insets.rs`): none before it told any, and never any under an activity that is not
    /// Waft's.
    window_insets: Cell<Option<WindowInsets>>,
    /// The rectangle the activity laid its content out in, left, top, right and bottom, as the
    /// system last told: what is known of the system's bars until the insets are told.
    content_rect: Cell<[i32; 4]>,
    /// How the picture reaches the screen (`picture.rs`).
    picture: RefCell<ViewPicture>,
    metrics: Cell<ViewMetrics>,
    /// What a frame starts from.
    clear: valo::Color,
    /// The last picture the framework handed over, kept to present again on the window the
    /// system gives back, and to tell a picture equal to it from one worth drawing.
    latest: RefCell<Option<Arc<Scene>>>,
    /// Whether the latest picture is on the screen: not while there is no window, and not
    /// when the screen refused it.
    on_screen: Cell<bool>,
    /// The keyboards' way in, where the activity is Waft's (`text_input.rs`).
    text_input: Option<AndroidTextInput>,
}

impl AndroidView {
    /// The view numbered `id` that `activity`'s window is, with no window yet: it draws nothing
    /// until the system gives one. Its picture is a composition layer's content on `compositor`,
    /// where there is one, and its text is typed through `text_input`, where there is one.
    pub(crate) fn new(
        id: ViewId,
        activity: Activity,
        gpu: &Gpu,
        compositor: Option<&SurfaceControlCompositor>,
        text_input: Option<AndroidTextInput>,
    ) -> AndroidView {
        let mut context = valo::Context::new(gpu.device.clone(), gpu.queue.clone());
        context.set_hide_missing_glyphs(true);
        AndroidView {
            id,
            activity,
            window: RefCell::new(None),
            window_insets: Cell::new(None),
            content_rect: Cell::new([0; 4]),
            picture: RefCell::new(ViewPicture::new(id, gpu, compositor, context)),
            metrics: Cell::new(ViewMetrics::default()),
            clear: valo::Color::WHITE,
            latest: RefCell::new(None),
            on_screen: Cell::new(false),
            text_input,
        }
    }

    /// The activity whose window the view is.
    pub(crate) fn activity(&self) -> &Activity {
        &self.activity
    }

    /// Whether the system gives the view a window now: a frame is drawn for no other.
    pub(crate) fn has_window(&self) -> bool {
        self.window.borrow().is_some()
    }

    /// The system gave the activity `window`: the picture reaches it, owed whatever the framework
    /// last drew.
    pub(crate) fn window_arrived(&self, gpu: &Gpu, window: NativeWindow) {
        let arrived = self
            .picture
            .borrow_mut()
            .window_arrived(gpu, &window, size_of(&window));
        if let Err(error) = arrived {
            crate::log::warn_to_log!("no picture on the window given: {error}");
        }
        *self.window.borrow_mut() = Some(window);
        self.on_screen.set(false);
    }

    /// The system took the window away: the view keeps the scene it showed and owes it, and
    /// nothing it was drawn into.
    pub(crate) fn window_gone(&self) {
        self.picture.borrow_mut().window_gone();
        self.window.borrow_mut().take();
        self.on_screen.set(false);
    }

    /// The window changed size: the picture follows it, and whatever it showed is gone.
    pub(crate) fn resized(&self) {
        let Some(size) = self.window.borrow().as_ref().map(size_of) else {
            return;
        };
        self.picture.borrow_mut().resized(size);
        self.on_screen.set(false);
    }

    /// Android applied `insets` to the window.
    pub(crate) fn set_window_insets(&self, insets: WindowInsets) {
        self.window_insets.set(Some(insets));
    }

    /// The activity laid its content out in `rect`, its left, top, right and bottom.
    pub(crate) fn set_content_rect(&self, rect: [i32; 4]) {
        self.content_rect.set(rect);
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

    /// Takes the metrics the system now reports of the window, at `density` pixels per logical
    /// pixel on `screen`, and answers whether they changed.
    pub(crate) fn refresh_metrics(&self, density: f64, screen: Option<ScreenId>) -> bool {
        let metrics = self.geometry(density, screen).metrics();
        if self.metrics.get() == metrics {
            return false;
        }
        self.metrics.set(metrics);
        true
    }

    /// What the system says about the window now; no size while there is no window.
    fn geometry(&self, density: f64, screen: Option<ScreenId>) -> Geometry {
        let size = self.window.borrow().as_ref().map_or([0, 0], size_of);
        Geometry {
            size,
            density,
            insets: self.insets_of(size),
            screen,
        }
    }

    /// What the system covers of the window of `size`: what its insets listener told, and until
    /// it tells, or where it never does, what the content rect leaves.
    fn insets_of(&self, size: [u32; 2]) -> WindowInsets {
        self.window_insets
            .get()
            .unwrap_or_else(|| WindowInsets::around_content(size, self.content_rect.get()))
    }
}

/// The window's pixels, as the system reports them now.
fn size_of(window: &NativeWindow) -> [u32; 2] {
    [window.width() as u32, window.height() as u32]
}

impl View for AndroidView {
    fn id(&self) -> ViewId {
        self.id
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
struct Geometry {
    /// The window's pixels.
    size: [u32; 2],
    /// Pixels per logical pixel, from the configuration's density.
    density: f64,
    /// What the system covers of the window.
    insets: WindowInsets,
    /// The screen the window is on, once the system has described it.
    screen: Option<ScreenId>,
}

impl Geometry {
    /// The view's metrics: the window's whole surface, with Flutter's three paddings of it and the
    /// system's gesture zones.
    fn metrics(self) -> ViewMetrics {
        let [width, height] = self.size.map(f64::from);
        let WindowInsets {
            view_padding,
            view_insets,
            system_gesture_insets,
        } = self.insets;
        ViewMetrics {
            physical_size: [width, height],
            physical_constraints: ViewConstraints::tight(width, height),
            device_pixel_ratio: self.density,
            padding: clear_of(view_padding, view_insets),
            view_padding,
            view_insets,
            system_gesture_insets,
            screen: self.screen,
        }
    }
}

/// `padding` as dart:ui makes it of the other two (`_buildViewConfiguration`, `hooks.dart`): what
/// of the system's bars the keyboard does not already cover.
fn clear_of(view_padding: ViewPadding, view_insets: ViewPadding) -> ViewPadding {
    ViewPadding {
        left: (view_padding.left - view_insets.left).max(0.0),
        top: (view_padding.top - view_insets.top).max(0.0),
        right: (view_padding.right - view_insets.right).max(0.0),
        bottom: (view_padding.bottom - view_insets.bottom).max(0.0),
    }
}
