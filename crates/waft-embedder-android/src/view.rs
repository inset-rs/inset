//! The host's [`View`]: an activity's window as the framework sees it.
//!
//! Each activity has one window, which is one view, and the view outlives the windows the system
//! hands the activity, and the activity itself: the framework keeps its tree and its last picture
//! while the activity is in the background, or destroyed to be made again, and the picture is
//! presented again on the window that comes back.
//!
//! What is one window's is kept here, by view, and nowhere for the app: the window and its size,
//! what the system covers of it, the rectangle its content was laid out in, its configuration, its
//! picture, its text input, the style of the bars over it, its task (`task_window.rs`), what it
//! arrived with (`view_requests.rs`), and the activity attached to it now. A later editor must keep
//! it so: the loop reaches a view
//! through the registry (`views.rs`) by the id an event carries, and what it finds there must be
//! that window's and no other's.
//!
//! While the view has no window it holds no buffers: what its picture was drawn into goes with the
//! window (`picture.rs`), and the scene it kept is drawn again into new ones when a window arrives.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use ndk::configuration::{Configuration, UiModeNight};
use ndk::native_window::NativeWindow;
use waft_embedder::{
    RequestId, Scene, ScreenId, SystemMouseCursorKind, SystemUiOverlayStyle, TextInputHost, View,
    ViewConstraints, ViewFocusDirection, ViewFocusState, ViewHost, ViewId, ViewMetrics,
    ViewPadding, WindowRef,
};

use crate::activity::{Activity, AttachedActivity};
use crate::application::Application;
use crate::chrome::AppDescription;
use crate::composition::SurfaceControlCompositor;
use crate::gpu::Gpu;
use crate::picture::ViewPicture;
use crate::task_window::TaskWindow;
use crate::text_input::AndroidTextInput;
use crate::view_requests::Arrival;
use crate::window_insets::WindowInsets;

pub(crate) struct AndroidView {
    id: ViewId,
    /// The activity whose window the view is, while Android has one for it.
    activity: AttachedActivity,
    /// The configuration of the activity's resources, as Android last told: the density among
    /// it.
    configuration: RefCell<Configuration>,
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
    /// The style the framework set last for the system's bars over the window, told again to each
    /// activity attached and at each change of configuration; none until the framework sets one,
    /// and the bars follow the system's mode till then.
    overlay_style: Cell<Option<SystemUiOverlayStyle>>,
    /// The activity's task, which is the view's window.
    task: Rc<TaskWindow>,
    /// The request the view arrived for; none for a view the platform gave.
    request: Option<RequestId>,
    initial_data: Option<String>,
    /// Whether the window has the focus, as Android last told.
    focused: Cell<bool>,
    /// Whether the app removed the task while Android had destroyed its activity, which no
    /// activity's destruction will tell.
    task_removed: Cell<bool>,
}

/// What a view is made of beside its id: what it arrived with, and what its window draws with.
pub(crate) struct ViewParts<'a> {
    pub(crate) arrival: Arrival,
    pub(crate) configuration: Configuration,
    pub(crate) application: &'a Application,
    pub(crate) app_description: AppDescription,
    pub(crate) gpu: &'a Gpu,
    pub(crate) compositor: Option<&'a SurfaceControlCompositor>,
}

impl AndroidView {
    /// The view numbered `id`, made of `parts`, whose activity is attached to it later, and whose
    /// configuration is the one it arrived with until its activity tells another; with no window
    /// yet: it draws nothing until the system gives one. Its picture is a composition layer's
    /// content on the compositor, where there is one, and its text is typed through the keyboards,
    /// under Waft's activity.
    pub(crate) fn new(id: ViewId, parts: ViewParts<'_>) -> Rc<AndroidView> {
        let ViewParts {
            arrival,
            configuration,
            application,
            app_description,
            gpu,
            compositor,
        } = parts;
        let activity = AttachedActivity::default();
        let text_input = AndroidTextInput::new(application, activity.clone());
        let mut context = valo::Context::new(gpu.device.clone(), gpu.queue.clone());
        context.set_hide_missing_glyphs(true);
        let (request, config) = arrival.request.unzip();
        Rc::new_cyclic(|view| AndroidView {
            id,
            task: Rc::new(TaskWindow::new(
                view.clone(),
                activity.clone(),
                application.clone(),
                app_description,
                config.as_ref(),
            )),
            activity,
            configuration: RefCell::new(configuration),
            window: RefCell::new(None),
            window_insets: Cell::new(None),
            content_rect: Cell::new([0; 4]),
            picture: RefCell::new(ViewPicture::new(id, gpu, compositor, context)),
            metrics: Cell::new(ViewMetrics::default()),
            clear: valo::Color::WHITE,
            latest: RefCell::new(None),
            on_screen: Cell::new(false),
            text_input,
            overlay_style: Cell::new(None),
            request,
            initial_data: arrival.initial_data,
            focused: Cell::new(false),
            task_removed: Cell::new(false),
        })
    }

    /// The view's task, which is its window.
    pub(crate) fn task(&self) -> &TaskWindow {
        &self.task
    }

    /// The window the system gives the view now.
    pub(crate) fn native_window(&self) -> Option<NativeWindow> {
        self.window.borrow().clone()
    }

    /// Whether the window has the focus, as Android last told.
    pub(crate) fn is_focused(&self) -> bool {
        self.focused.get()
    }

    /// The window gained or lost the focus.
    pub(crate) fn set_focused(&self, focused: bool) {
        self.focused.set(focused);
    }

    /// The app removed the view's task while Android had destroyed its activity: the view goes at
    /// the loop's next pass, as no activity's destruction will tell.
    pub(crate) fn task_was_removed(&self) {
        self.task_removed.set(true);
        crate::loop_waker::wake_app_loop();
    }

    /// Whether the app removed the view's task while it had no activity.
    pub(crate) fn is_task_removed(&self) -> bool {
        self.task_removed.get()
    }

    /// The activity whose window the view is, while Android has one for it.
    pub(crate) fn activity(&self) -> Option<Activity> {
        self.activity.get()
    }

    /// Android made `activity` for the view, with `configuration`: its keyboard types into the
    /// view from now on, its window's bars are styled ([`Self::set_configuration`]), and its task
    /// shows what the window has.
    pub(crate) fn attach(&self, activity: &Activity, configuration: Configuration) {
        self.activity.attach(activity);
        self.set_configuration(configuration);
        if let Some(text_input) = &self.text_input {
            text_input.types_into(self.id);
        }
        self.task.attach(activity);
    }

    /// Android destroyed `activity`: the view has none until Android makes another for it.
    pub(crate) fn detach(&self, activity: &Activity) {
        self.activity.detach(activity);
    }

    /// The activity's configuration changed to `configuration`, which may have switched the
    /// system between light and dark mode: the bars are styled again.
    pub(crate) fn set_configuration(&self, configuration: Configuration) {
        *self.configuration.borrow_mut() = configuration;
        self.style_the_bars();
    }

    /// Styles the bars over the activity's window as the framework set last, or, until it sets a
    /// style, as the system's mode has them ([`crate::chrome::style_of_the_mode`]).
    fn style_the_bars(&self) {
        let Some(activity) = self.activity() else {
            return;
        };
        let style = self.overlay_style.get().unwrap_or_else(|| {
            crate::chrome::style_of_the_mode(
                self.configuration.borrow().ui_mode_night() == UiModeNight::Yes,
            )
        });
        crate::chrome::style_the_bars(&activity, style);
    }

    /// The configuration of the activity's resources, as Android last told.
    pub(crate) fn configuration(&self) -> Configuration {
        self.configuration.borrow().clone()
    }

    /// Pixels per logical pixel, from the configuration's density.
    pub(crate) fn density(&self) -> f64 {
        self.configuration
            .borrow()
            .density()
            .map_or(1.0, |dpi| f64::from(dpi) / 160.0)
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

    fn initial_data(&self) -> Option<String> {
        self.initial_data.clone()
    }

    fn request_id(&self) -> Option<RequestId> {
        self.request
    }

    /// The activity's task, which every view on Android is in.
    fn window(&self) -> Option<WindowRef> {
        Some(Rc::clone(&self.task) as WindowRef)
    }
}

/// The view is its own host, which takes text through the system's keyboards, styles the bars
/// over its activity's window and brings its task forward for the focus; this embedder has no
/// cursor, and asks the framework about Back on the spot (`EmbedderClient::pop_route`), so it needs
/// no word in advance.
impl ViewHost for AndroidView {
    fn text_input(&self) -> Option<&dyn TextInputHost> {
        self.text_input
            .as_ref()
            .map(|text_input| text_input as &dyn TextInputHost)
    }

    fn activate_system_cursor(&self, _device: i64, _kind: SystemMouseCursorKind) {}

    /// The focus is brought to the view by bringing its task forward
    /// (`ActivityManager.AppTask.moveToFront`), which Android allows while the app has a window in
    /// sight; a phone has no focus to move away from a view but to another.
    fn request_focus_change(&self, state: ViewFocusState, _direction: ViewFocusDirection) {
        if state == ViewFocusState::Focused {
            self.task.move_to_front();
        }
    }

    fn set_overlay_style(&self, style: &SystemUiOverlayStyle) {
        self.overlay_style.set(Some(*style));
        self.style_the_bars();
    }
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
