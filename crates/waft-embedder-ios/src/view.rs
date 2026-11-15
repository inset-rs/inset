//! The host's [`View`]: one scene's window as the framework sees it.
//!
//! A view outlives its scene's connection: the framework keeps its tree and its last picture while
//! UIKit has its scene disconnected, and the picture is presented again in the window of the scene
//! that connects again by the same session. What is one window's is kept here, by view, and nowhere
//! for the app: the scene, its window, view controller and `UIView` while it is connected, the
//! state UIKit says it is in, the keyboard's inset over it, its metrics and its picture
//! (`picture.rs`). The host reaches a view through the registry (`views.rs`) by the id an event
//! carries, and what it finds there must be that window's and no other's.

// The trait collection's getters are marked `unsafe` for their threading; this module calls them
// on the main thread, and writes the `unsafe` for it.
#![allow(unsafe_code)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use objc2::rc::Retained;
use objc2::{MainThreadOnly, Message};
use objc2_core_foundation::CGRect;
use objc2_ui_kit::{
    UICoordinateSpace, UIUserInterfaceStyle, UIViewController, UIWindow, UIWindowScene,
};
use waft_embedder::{
    Brightness, Scene, ScreenId, SystemMouseCursorKind, TextInputHost, View, ViewFocusDirection,
    ViewFocusState, ViewHost, ViewId, ViewMetrics, ViewPadding,
};

use crate::gpu::Gpu;
use crate::keyboard_inset::{KeyboardChange, ScreenRect};
use crate::lifecycle::SceneState;
use crate::metrics::Geometry;
use crate::picture::ViewPicture;
use crate::text_input::IosTextInput;
use crate::ui_view::WaftView;
use crate::view_controller::WaftViewController;

/// The one screen an iPhone's scenes are on.
pub(crate) const SCREEN: ScreenId = ScreenId(0);

pub(crate) struct IosView {
    id: ViewId,
    /// The persistent identifier of the session whose scene shows the view, once one has.
    session: RefCell<Option<String>>,
    /// The scene's window, while the scene is connected.
    window: RefCell<Option<SceneWindow>>,
    /// What UIKit says the scene is in; in the background until it says otherwise.
    state: Cell<SceneState>,
    /// How many points of the view's bottom the docked keyboard covers.
    keyboard_inset: Cell<f64>,
    metrics: Cell<ViewMetrics>,
    picture: RefCell<ViewPicture>,
    /// The last picture the framework handed over, kept to present again on the scene that
    /// connects again, and to tell a picture equal to it from one worth drawing.
    latest: RefCell<Option<Arc<Scene>>>,
    /// Whether the latest picture is on the screen: not while the scene is disconnected or in the
    /// background, and not when the surface refused it.
    on_screen: Cell<bool>,
    /// The keyboards' way into the view's text fields (`text_input/`).
    text_input: IosTextInput,
}

/// A connected scene's window and what shows the view in it.
struct SceneWindow {
    scene: Retained<UIWindowScene>,
    window: Retained<UIWindow>,
    controller: Retained<WaftViewController>,
    view: Retained<WaftView>,
}

impl IosView {
    /// The view numbered `id`, drawing on `gpu`'s device, in no scene yet: it has no size until a
    /// scene shows it, as Flutter's implicit view has empty metrics until its window reports them.
    pub(crate) fn new(id: ViewId, gpu: &Gpu) -> Rc<IosView> {
        Rc::new(IosView {
            id,
            session: RefCell::new(None),
            window: RefCell::new(None),
            state: Cell::new(SceneState::Background),
            keyboard_inset: Cell::new(0.0),
            metrics: Cell::new(ViewMetrics::default()),
            picture: RefCell::new(ViewPicture::new(gpu)),
            latest: RefCell::new(None),
            on_screen: Cell::new(false),
            text_input: IosTextInput::new(id),
        })
    }

    /// The session whose scene showed the view, once one has.
    pub(crate) fn session(&self) -> Option<String> {
        self.session.borrow().clone()
    }

    /// The view's session was closed for good: the next scene that connects shows it.
    pub(crate) fn forget_session(&self) {
        *self.session.borrow_mut() = None;
    }

    /// `scene`, connected to `session`, shows the view from now on, in a window of its own whose
    /// root view controller shows the view's `UIView`, which takes the hardware keyboard's presses.
    pub(crate) fn attach(&self, scene: &UIWindowScene, session: String) {
        let main_thread = scene.mtm();
        let view = WaftView::new(main_thread, self.id);
        let controller = WaftViewController::new(main_thread, self.id, &view);
        let window = UIWindow::initWithWindowScene(main_thread.alloc(), scene);
        window.setRootViewController(Some(&controller));
        *self.session.borrow_mut() = Some(session);
        *self.window.borrow_mut() = Some(SceneWindow {
            scene: scene.retain(),
            window: window.clone(),
            controller,
            view: view.clone(),
        });
        self.state.set(SceneState::Background);
        self.text_input.attach(&view);
        window.makeKeyAndVisible();
        view.becomeFirstResponder();
        self.laid_out();
    }

    /// The scene disconnected: its window goes, and what the picture was drawn into with it, and
    /// the view keeps its last picture, owed to the scene that connects again.
    pub(crate) fn detach(&self) {
        self.text_input.detach();
        if let Some(gone) = self.window.borrow_mut().take() {
            gone.window.setHidden(true);
        }
        self.picture.borrow_mut().drop_surface();
        self.state.set(SceneState::Background);
        self.on_screen.set(false);
    }

    /// The scene is in `state` from now on; one sent to the background no longer shows the picture
    /// it had, which iOS may have dropped, and is owed it.
    pub(crate) fn set_state(&self, state: SceneState) {
        if state == SceneState::Background {
            self.on_screen.set(false);
        }
        self.state.set(state);
    }

    /// Whether the view is drawn: its scene connected and in the foreground.
    pub(crate) fn is_in_sight(&self) -> bool {
        self.window.borrow().is_some() && self.state.get() != SceneState::Background
    }

    /// Whether the view's scene is active, taking events.
    pub(crate) fn is_focused(&self) -> bool {
        self.window.borrow().is_some() && self.state.get() == SceneState::Active
    }

    /// Whether the view has pixels to draw.
    pub(crate) fn has_area(&self) -> bool {
        let [width, height] = self.metrics.get().physical_size;
        width > 0.0 && height > 0.0
    }

    /// The view controller of the view's window, while its scene is connected.
    pub(crate) fn view_controller(&self) -> Option<Retained<UIViewController>> {
        let window = self.window.borrow();
        Some(Retained::into_super(window.as_ref()?.controller.clone()))
    }

    /// The appearance the user chose for the view's scene, while it is connected.
    pub(crate) fn brightness(&self) -> Option<Brightness> {
        let window = self.window.borrow();
        let traits = window.as_ref()?.scene.traitCollection();
        // SAFETY: read on the main thread, which the trait collection is the scene's on.
        let style = unsafe { traits.userInterfaceStyle() };
        Some(if style == UIUserInterfaceStyle::Dark {
            Brightness::Dark
        } else {
            Brightness::Light
        })
    }

    /// The view was laid out: the picture fills its `UIView` at its size and scale.
    pub(crate) fn laid_out(&self) {
        let Some(geometry) = self.geometry() else {
            return;
        };
        let window = self.window.borrow();
        let Some(window) = window.as_ref() else {
            return;
        };
        let size = geometry
            .size
            .map(|side| (side * geometry.scale).round() as u32);
        self.picture
            .borrow_mut()
            .fill(&window.view.layer(), geometry.scale, size);
        self.on_screen.set(false);
    }

    /// Takes the metrics UIKit now reports of the view, and answers whether they changed.
    pub(crate) fn refresh_metrics(&self) -> bool {
        let metrics = self
            .geometry()
            .map_or_else(ViewMetrics::default, Geometry::metrics);
        if self.metrics.get() == metrics {
            return false;
        }
        self.metrics.set(metrics);
        true
    }

    /// What UIKit says of the view now; nothing while its scene is disconnected.
    fn geometry(&self) -> Option<Geometry> {
        let window = self.window.borrow();
        let window = window.as_ref()?;
        let bounds = window.view.bounds();
        let safe_area = window.view.safeAreaInsets();
        Some(Geometry {
            size: [bounds.size.width, bounds.size.height],
            scale: window.scene.screen().scale(),
            safe_area: ViewPadding {
                left: safe_area.left,
                top: safe_area.top,
                right: safe_area.right,
                bottom: safe_area.bottom,
            },
            keyboard_inset: self.keyboard_inset.get(),
            screen: Some(SCREEN),
        })
    }

    /// The keyboard moves as `change` says: the view takes what it covers of the view from now on,
    /// and answers whether that changed.
    pub(crate) fn follow_keyboard(&self, change: KeyboardChange) -> bool {
        let covered = {
            let window = self.window.borrow();
            let Some(window) = window.as_ref() else {
                return false;
            };
            let screen = window.scene.screen();
            let space = screen.coordinateSpace();
            let in_screen = window
                .view
                .convertRect_toCoordinateSpace(window.view.bounds(), &space);
            change.inset_of(screen_rect(in_screen), screen_rect(screen.bounds()))
        };
        match covered {
            Some(covered) if covered != self.keyboard_inset.get() => {
                self.keyboard_inset.set(covered);
                true
            }
            _ => false,
        }
    }

    /// Presents the latest picture again if it is not on the screen: an owed frame, paid without
    /// the framework drawing another.
    pub(crate) fn represent(&self) {
        if self.on_screen.get() {
            return;
        }
        let latest = self.latest.borrow().clone();
        if let Some(latest) = latest {
            self.draw(&latest);
        }
    }

    /// Draws `scene` while the view is in sight; out of sight it stays owed.
    fn draw(&self, scene: &Scene) {
        if !self.is_in_sight() {
            return;
        }
        let drawn = self.picture.borrow_mut().draw(scene, self.id);
        self.on_screen.set(drawn.is_ok());
    }
}

/// `rect` as the keyboard's reckoning takes it.
fn screen_rect(rect: CGRect) -> ScreenRect {
    ScreenRect {
        x: rect.origin.x,
        y: rect.origin.y,
        width: rect.size.width,
        height: rect.size.height,
    }
}

impl View for IosView {
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

/// The view is its own host, which takes text through the system's keyboards. iOS shows no cursor
/// a view picks, and a phone has no focus to move to a view; Back is no key on iOS.
impl ViewHost for IosView {
    fn text_input(&self) -> Option<&dyn TextInputHost> {
        Some(&self.text_input)
    }

    fn activate_system_cursor(&self, _device: i64, _kind: SystemMouseCursorKind) {}

    fn request_focus_change(&self, _state: ViewFocusState, _direction: ViewFocusDirection) {}
}
