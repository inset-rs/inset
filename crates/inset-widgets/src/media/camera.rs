//! A camera session, as an app holds it, and the view that shows its preview.

use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

use inset_embedder::{
    Camera, CameraState, CaptureSession, FrameReader, MediaChangeFuture, MovieOptions, MovieOutput,
    PhotoOutput, Size, Surface,
};
use inset_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle};

use super::{PictureSource, SurfacePicture};
use crate::framework::{BuildContext, IntoWidget, KeyRef, StatelessWidget, WidgetRef};

/// The cameras attached now, in the system's order; none where the platform has no cameras:
/// Flutter's `availableCameras`.
pub fn available_cameras(app: &App) -> Vec<Camera> {
    app.platform()
        .cameras()
        .map(|cameras| cameras.cameras())
        .unwrap_or_default()
}

/// A session on one camera, stopped until [`start`](CameraSession::start), with outputs each
/// the system's own: the host's session, whose listeners hear its state change. It asks the user
/// for the camera at its first start, where the system asks.
pub struct CameraSession {
    change_notifier: ChangeNotifierData,
    /// The host's session, until [`dispose`](CameraSession::dispose).
    host: Option<Rc<dyn CaptureSession>>,
    /// The host's state when it last changed.
    state: CameraState,
}

impl CameraSession {
    /// A session on `camera`; `None` where the platform has no cameras.
    pub fn new(app: &mut App, camera: &Camera) -> Option<Handle<CameraSession>> {
        let platform = app.platform();
        let host: Rc<dyn CaptureSession> = Rc::from(platform.cameras()?.open_session(camera));
        let changed = host.changed();
        let session = app.create(CameraSession {
            change_notifier: ChangeNotifierData::new(),
            state: host.state(),
            host: Some(host),
        });
        session.follow(app, changed);
        Some(session)
    }

    /// The session's state when it last changed.
    pub fn state(self: Handle<Self>, app: &App) -> &CameraState {
        &app.get(self).state
    }

    /// Starts the camera, asking the user for it first where the system asks.
    pub fn start(self: Handle<Self>, app: &App) {
        self.host(app).start();
    }

    pub fn stop(self: Handle<Self>, app: &App) {
        self.host(app).stop();
    }

    /// Shows the preview on `surface` from now on, and on no surface it showed on before: what
    /// [`CameraPreview`] does with a surface of its own.
    pub fn show_on(self: Handle<Self>, app: &App, surface: Arc<dyn Surface>) {
        self.host(app).show_on(surface);
    }

    /// The camera's frames, the newest each time.
    pub fn frame_output(self: Handle<Self>, app: &App) -> Arc<dyn FrameReader> {
        self.host(app).frame_output()
    }

    /// Photos, each captured, processed and encoded by the system.
    pub fn photo_output(self: Handle<Self>, app: &App) -> Box<dyn PhotoOutput> {
        self.host(app).photo_output()
    }

    /// Recordings, each encoded and written to a file by the system.
    pub fn movie_output(
        self: Handle<Self>,
        app: &App,
        options: MovieOptions,
    ) -> Box<dyn MovieOutput> {
        self.host(app).movie_output(options)
    }

    /// Stops the camera and lets go of the host's session: its preview leaves the surface it
    /// showed on, and its listeners hear nothing more.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        let session = app.get_mut(self);
        session.host = None;
        session.change_notifier.dispose();
    }

    /// The host's session.
    ///
    /// # Panics
    ///
    /// Once disposed.
    fn host(self: Handle<Self>, app: &App) -> &dyn CaptureSession {
        app.get(self)
            .host
            .as_deref()
            .expect("a CameraSession used after dispose")
    }

    /// Takes the host's state at each of its changes, from the one `changed` resolves at, until
    /// the session is disposed.
    fn follow(self: Handle<Self>, app: &mut App, changed: MediaChangeFuture) {
        let _following = app.spawn(async move |cx| {
            let mut changed = changed;
            loop {
                changed.await;
                match cx.update(|app| self.take_host_state(app)) {
                    Some(next) => changed = next,
                    None => return,
                }
            }
        });
    }

    /// Takes the host's state, telling the listeners if it changed; answers the future of the
    /// host's next change, asked before the state is read so that no change goes unheard, or
    /// `None` once the session is disposed.
    fn take_host_state(self: Handle<Self>, app: &mut App) -> Option<MediaChangeFuture> {
        let host = app.get(self).host.clone()?;
        let next = host.changed();
        let state = host.state();
        if state != app.get(self).state {
            app.get_mut(self).state = state;
            self.notify_listeners(app);
        }
        Some(next)
    }
}

impl ChangeNotifier for CameraSession {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl fmt::Debug for CameraSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CameraSession")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl PictureSource for CameraSession {
    fn show_on(handle: Handle<Self>, app: &mut App, surface: Arc<dyn Surface>) {
        handle.show_on(app, surface);
    }

    fn picture_size(handle: Handle<Self>, app: &App) -> Option<Size> {
        handle.state(app).size
    }
}

/// A camera session's preview, laid out at the camera's picture size in logical pixels, or as
/// large as the constraints let it be at that aspect; no room until the session has the camera.
/// It shows the session on a surface of its own, which the system draws the preview into.
#[derive(Clone)]
pub struct CameraPreview {
    pub key: Option<KeyRef>,
    pub session: Handle<CameraSession>,
}

impl CameraPreview {
    pub fn new(session: Handle<CameraSession>) -> CameraPreview {
        CameraPreview { key: None, session }
    }

    pub fn key(mut self, key: KeyRef) -> CameraPreview {
        self.key = Some(key);
        self
    }
}

impl fmt::Debug for CameraPreview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CameraPreview").finish_non_exhaustive()
    }
}

impl StatelessWidget for CameraPreview {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        SurfacePicture {
            source: self.session,
        }
        .into_widget()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use inset_embedder::{CameraFacing, CameraId, CameraStatus, MediaError, Photo, TargetPlatform};
    use inset_foundation::AppCell;
    use inset_test::{MockCameras, MockCompositor, SessionCall, TestPlatform, TestView};

    use super::*;
    use crate::binding::run_app;
    use crate::test_harness::pump_every_view;

    fn front_camera() -> Camera {
        Camera {
            id: CameraId("front".to_owned()),
            name: "Front camera".to_owned(),
            facing: CameraFacing::Front,
        }
    }

    fn app_with(
        cameras: &MockCameras,
        compositor: &MockCompositor,
        view: &Rc<TestView>,
    ) -> Rc<AppCell> {
        let platform = TestPlatform::new()
            .on(TargetPlatform::IOS)
            .with_view(Rc::clone(view) as inset_embedder::ViewRef)
            .with_cameras(Rc::new(cameras.clone()))
            .with_compositor(Arc::new(compositor.clone()));
        AppCell::with_platform(Rc::new(platform))
    }

    fn running(size: Size) -> CameraState {
        CameraState {
            status: CameraStatus::Running,
            size: Some(size),
        }
    }

    /// Runs `future` to its end, which the mocks answer without waiting.
    fn now<T>(future: impl std::future::Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(output) => output,
            std::task::Poll::Pending => panic!("the mock answers at once"),
        }
    }

    #[test]
    fn the_cameras_are_the_platforms_and_none_where_it_has_none() {
        let cameras = MockCameras::new(vec![front_camera()]);
        let cell = app_with(
            &cameras,
            &MockCompositor::new(),
            &Rc::new(TestView::new(400.0, 800.0)),
        );
        assert_eq!(available_cameras(&cell.borrow()), vec![front_camera()]);

        let without = AppCell::with_platform(Rc::new(TestPlatform::new()));
        assert!(available_cameras(&without.borrow()).is_empty());
        assert!(CameraSession::new(&mut without.borrow_mut(), &front_camera()).is_none());
    }

    #[test]
    fn a_session_starts_and_its_outputs_are_the_systems() {
        let cameras = MockCameras::new(vec![front_camera()]);
        let cell = app_with(
            &cameras,
            &MockCompositor::new(),
            &Rc::new(TestView::new(400.0, 800.0)),
        );
        let session = CameraSession::new(&mut cell.borrow_mut(), &front_camera()).unwrap();
        let photos = session.photo_output(&cell.borrow());
        let movies = session.movie_output(&cell.borrow(), MovieOptions { sound: true });
        let _frames = session.frame_output(&cell.borrow());
        session.start(&cell.borrow());
        let host = &cameras.sessions()[0];
        assert_eq!(host.camera(), front_camera());
        assert_eq!(
            host.calls(),
            vec![
                SessionCall::PhotoOutput,
                SessionCall::MovieOutput(MovieOptions { sound: true }),
                SessionCall::FrameOutput,
                SessionCall::Start
            ]
        );
        assert_eq!(now(photos.capture()), Err(MediaError::NotRunning));

        host.set_state(running(Size::new(1920.0, 1080.0)));
        cell.checkpoint();
        assert_eq!(session.state(&cell.borrow()).status, CameraStatus::Running);
        let photo = Photo {
            bytes: vec![0xff, 0xd8],
            mime_type: "image/jpeg".to_owned(),
        };
        host.answering_photo(photo.clone());
        assert_eq!(now(photos.capture()), Ok(photo));

        let mut recording = movies.start(PathBuf::from("/tmp/clip.mov"));
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(recording.as_mut().poll(&mut context).is_pending());
        movies.stop();
        assert_eq!(now(recording), Ok(()));
        assert_eq!(host.recordings(), vec![PathBuf::from("/tmp/clip.mov")]);

        session.dispose(&mut cell.borrow_mut());
        assert!(host.is_dropped());
    }

    #[test]
    fn a_camera_preview_shows_its_session_on_a_surface_placed_at_the_cameras_size() {
        let cameras = MockCameras::new(vec![front_camera()]);
        let compositor = MockCompositor::new();
        let view = Rc::new(TestView::new(400.0, 800.0));
        let cell = app_with(&cameras, &compositor, &view);
        let session = CameraSession::new(&mut cell.borrow_mut(), &front_camera()).unwrap();
        run_app(
            &mut cell.borrow_mut(),
            crate::Center::new()
                .child(CameraPreview::new(session))
                .into_widget(),
        );
        cell.elapse(Duration::ZERO);
        pump_every_view(&mut cell.borrow_mut(), Duration::ZERO);
        let host = &cameras.sessions()[0];
        let opened = compositor.opened();
        assert!(Arc::ptr_eq(&host.shown_on().unwrap(), &opened[0].surface));

        host.set_state(running(Size::new(1920.0, 1080.0)));
        cell.checkpoint();
        pump_every_view(&mut cell.borrow_mut(), Duration::from_millis(16));
        let scene = view.last_scene().unwrap();
        assert_eq!(
            scene.surfaces[0].rect.size(),
            Size::new(400.0, 225.0),
            "as wide as the view lets it be, at the camera's aspect"
        );
    }
}
