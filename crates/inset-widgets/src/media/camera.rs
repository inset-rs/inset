//! A camera running for an app, as the app holds it, and the view that shows its preview.

use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use inset_embedder::{
    Camera, CameraDevice, CameraError, CameraFormat, CameraOpenFuture, CameraOutputs,
    CaptureSession, CaptureState, CompositionLayer, MediaChangeFuture, PhotoFuture,
    RecordingFuture,
};
use inset_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle};

use super::at_size;
use crate::framework::{BuildContext, IntoWidget, KeyRef, StatelessWidget, WidgetRef};
use crate::widgets::basic::SizedBox;
use crate::widgets::composition_layer_view::CompositionLayerView;

/// The cameras attached now, in the system's order; none where the platform has no cameras:
/// Flutter's `availableCameras`. Listing asks the user nothing.
pub fn available_cameras(app: &App) -> Vec<Camera> {
    app.platform()
        .cameras()
        .map(|cameras| cameras.cameras())
        .unwrap_or_default()
}

/// Opens `camera`, asking the user for the cameras first where the system asks and the user has
/// not answered; `None` where the platform has no cameras. The camera opened lists the formats it
/// runs in, and a [`CameraSession`] starts it in one.
pub fn open_camera(app: &App, camera: &Camera) -> Option<CameraOpenFuture> {
    Some(app.platform().cameras()?.open(camera))
}

/// A camera running for the app in one format, feeding the outputs it started with, each the
/// system's own: the host's session, whose listeners hear its state change.
pub struct CameraSession {
    change_notifier: ChangeNotifierData,
    /// The host's session, until [`dispose`](CameraSession::dispose).
    host: Option<Rc<dyn CaptureSession>>,
    /// The format the camera runs in for the session.
    format: CameraFormat,
    /// The host's state when it last changed.
    state: CaptureState,
    /// The layer the preview shows on, among the outputs it started with, which a
    /// [`CameraPreview`] places.
    preview_layer: Option<Arc<dyn CompositionLayer>>,
}

impl CameraSession {
    /// Starts `camera` in `format`, one of its formats, feeding `outputs`, which the system
    /// configures together; where the camera already runs for another session in a format of its
    /// own, the session joins that one, which [`format`](CameraSession::format) says. It fails
    /// for what can happen at any time: the camera in use, unplugged, or not allowed; and for a
    /// format the camera does not list.
    pub fn start(
        app: &mut App,
        camera: &dyn CameraDevice,
        format: &CameraFormat,
        outputs: CameraOutputs,
    ) -> Result<Handle<CameraSession>, CameraError> {
        let preview_layer = outputs.preview.clone();
        let host: Rc<dyn CaptureSession> = Rc::from(camera.start(format, outputs)?);
        let changed = host.changed();
        let session = app.create(CameraSession {
            change_notifier: ChangeNotifierData::new(),
            format: host.format(),
            state: host.state(),
            host: Some(host),
            preview_layer,
        });
        session.follow(app, changed);
        Ok(session)
    }

    /// The format the camera runs in: the one picked, unless the session joined another.
    pub fn format(self: Handle<Self>, app: &App) -> &CameraFormat {
        &app.get(self).format
    }

    /// The session's state when it last changed.
    pub fn state(self: Handle<Self>, app: &App) -> &CaptureState {
        &app.get(self).state
    }

    /// The layer the preview shows on, which a [`CameraPreview`] places; `None` where the session
    /// started without one, or once disposed.
    pub fn preview_layer(self: Handle<Self>, app: &App) -> Option<&Arc<dyn CompositionLayer>> {
        app.get(self).preview_layer.as_ref()
    }

    /// Takes a photo, captured, processed and encoded by the system, while the session runs with
    /// photos among its outputs.
    pub fn take_photo(self: Handle<Self>, app: &App) -> PhotoFuture {
        self.host(app).take_photo()
    }

    /// Records to a new file at `path`, encoded and written by the system, while the session runs
    /// with movies among its outputs, until [`stop_recording`](CameraSession::stop_recording).
    pub fn start_recording(self: Handle<Self>, app: &App, path: PathBuf) -> RecordingFuture {
        self.host(app).start_recording(path)
    }

    /// Ends the recording, whose future then answers.
    pub fn stop_recording(self: Handle<Self>, app: &App) {
        self.host(app).stop_recording();
    }

    /// Stops the camera for the app and lets go of the host's session: its preview leaves the
    /// layer it was on, and its listeners hear nothing more.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        let session = app.get_mut(self);
        session.host = None;
        session.preview_layer = None;
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
            .field("format", &self.format)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

/// A camera session's preview, laid out at the size of the format the camera runs in, in logical
/// pixels, or as large as the constraints let it be at that aspect. It places the layer the
/// session started with for its preview, which the system draws into; a session started without
/// one shows nothing there.
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

    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let size = self.session.format(app).size;
        let preview = match self.session.preview_layer(app) {
            Some(layer) => CompositionLayerView::new(Arc::clone(layer)).into_widget(),
            None => SizedBox::expand().into_widget(),
        };
        at_size(size, preview)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use inset_embedder::{
        CameraFacing, CameraId, CameraInterruption, MediaError, MovieOptions, Photo, Size,
        TargetPlatform, VideoFrameReaders,
    };
    use inset_foundation::{AppCell, ListenableObject, Listener};
    use inset_test::{MockCameras, MockCompositor, MockVideoFrameReaders, TestPlatform, TestView};

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

    fn format(width: f64, height: f64) -> CameraFormat {
        CameraFormat {
            size: Size::new(width, height),
            min_frame_rate: 15.0,
            max_frame_rate: 30.0,
            photo_sizes: vec![Size::new(width, height)],
        }
    }

    fn cameras() -> MockCameras {
        MockCameras::new(vec![front_camera()])
            .with_formats(vec![format(1920.0, 1080.0), format(640.0, 480.0)])
    }

    fn app_with(cameras: &MockCameras, compositor: &MockCompositor) -> Rc<AppCell> {
        app_on(cameras, compositor, &Rc::new(TestView::new(400.0, 800.0)))
    }

    fn app_on(
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

    /// Runs `future` to its end, which the mocks answer without waiting.
    fn now<T>(future: impl std::future::Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(output) => output,
            std::task::Poll::Pending => panic!("the mock answers at once"),
        }
    }

    fn opened(cell: &AppCell) -> Box<dyn CameraDevice> {
        let opening =
            open_camera(&cell.borrow(), &front_camera()).expect("a platform with cameras");
        now(opening).expect("the user lets the app use the cameras")
    }

    #[test]
    fn the_cameras_are_the_platforms_and_none_where_it_has_none() {
        let cameras = cameras();
        let cell = app_with(&cameras, &MockCompositor::new());
        assert_eq!(available_cameras(&cell.borrow()), vec![front_camera()]);

        let without = AppCell::with_platform(Rc::new(TestPlatform::new()));
        assert!(available_cameras(&without.borrow()).is_empty());
        assert!(open_camera(&without.borrow(), &front_camera()).is_none());
    }

    #[test]
    fn a_camera_the_user_does_not_allow_does_not_open() {
        let cell = app_with(&cameras().denying(), &MockCompositor::new());
        let opening = open_camera(&cell.borrow(), &front_camera()).unwrap();
        assert!(matches!(now(opening), Err(CameraError::NotAllowed)));
    }

    #[test]
    fn a_camera_starts_in_a_format_it_lists_with_its_outputs_given_whole() {
        let cameras = cameras();
        let cell = app_with(&cameras, &MockCompositor::new());
        let camera = opened(&cell);
        let formats = camera.formats();
        assert_eq!(formats, vec![format(1920.0, 1080.0), format(640.0, 480.0)]);

        let reader = MockVideoFrameReaders::new().make_reader();
        let outputs = CameraOutputs::new()
            .frames(Arc::clone(&reader))
            .photos(Size::new(640.0, 480.0))
            .movies(MovieOptions { sound: true });
        let session =
            CameraSession::start(&mut cell.borrow_mut(), &*camera, &formats[1], outputs).unwrap();
        let host = &cameras.sessions()[0];
        assert_eq!(host.camera(), front_camera());
        assert_eq!(*session.format(&cell.borrow()), format(640.0, 480.0));
        assert!(Arc::ptr_eq(&host.frame_reader().unwrap(), &reader));
        assert_eq!(host.photo_size(), Some(Size::new(640.0, 480.0)));
        assert_eq!(host.movie_options(), Some(MovieOptions { sound: true }));
        assert_eq!(*session.state(&cell.borrow()), CaptureState::Running);

        let unlisted = format(320.0, 240.0);
        assert!(matches!(
            CameraSession::start(
                &mut cell.borrow_mut(),
                &*camera,
                &unlisted,
                CameraOutputs::new()
            ),
            Err(CameraError::Failed(_))
        ));

        session.dispose(&mut cell.borrow_mut());
        assert!(host.is_dropped());
    }

    #[test]
    fn a_session_on_a_camera_another_runs_joins_its_format_and_says_so() {
        let cameras = cameras();
        let cell = app_with(&cameras, &MockCompositor::new());
        let camera = opened(&cell);
        let formats = camera.formats();
        let first = CameraSession::start(
            &mut cell.borrow_mut(),
            &*camera,
            &formats[0],
            CameraOutputs::new(),
        )
        .unwrap();
        let joined = CameraSession::start(
            &mut cell.borrow_mut(),
            &*camera,
            &formats[1],
            CameraOutputs::new(),
        )
        .unwrap();
        assert_eq!(cameras.sessions()[1].picked(), format(640.0, 480.0));
        assert_eq!(*joined.format(&cell.borrow()), format(1920.0, 1080.0));

        first.dispose(&mut cell.borrow_mut());
        joined.dispose(&mut cell.borrow_mut());
        let alone = CameraSession::start(
            &mut cell.borrow_mut(),
            &*camera,
            &formats[1],
            CameraOutputs::new(),
        )
        .unwrap();
        assert_eq!(
            *alone.format(&cell.borrow()),
            format(640.0, 480.0),
            "once no session runs the camera, it runs in the format picked"
        );
    }

    #[test]
    fn a_sessions_listeners_hear_it_interrupted_resume_and_end() {
        let cameras = cameras();
        let cell = app_with(&cameras, &MockCompositor::new());
        let camera = opened(&cell);
        let outputs = CameraOutputs::new().photos(Size::new(1920.0, 1080.0));
        let session = CameraSession::start(
            &mut cell.borrow_mut(),
            &*camera,
            &camera.formats()[0],
            outputs,
        )
        .unwrap();
        let heard = Rc::new(std::cell::Cell::new(0));
        let hearing = Rc::clone(&heard);
        session.add_listener(
            &mut cell.borrow_mut(),
            Listener::new(move |_| hearing.set(hearing.get() + 1)),
        );
        let host = &cameras.sessions()[0];
        let photo = Photo {
            bytes: vec![0xff, 0xd8],
            mime_type: "image/jpeg".to_owned(),
        };
        host.answering_photo(photo.clone());
        assert_eq!(now(session.take_photo(&cell.borrow())), Ok(photo));
        assert!(
            matches!(
                now(session.start_recording(&cell.borrow(), PathBuf::from("/tmp/clip.mov"))),
                Err(MediaError::Failed(_))
            ),
            "movies were not among its outputs"
        );

        let interrupted = CaptureState::Interrupted(CameraInterruption::InUseByAnotherApp);
        host.set_state(interrupted.clone());
        cell.checkpoint();
        assert_eq!(*session.state(&cell.borrow()), interrupted);
        assert_eq!(
            now(session.take_photo(&cell.borrow())),
            Err(MediaError::NotRunning)
        );

        host.set_state(CaptureState::Running);
        cell.checkpoint();
        host.set_state(CaptureState::Ended(CameraError::Unplugged));
        cell.checkpoint();
        assert_eq!(
            *session.state(&cell.borrow()),
            CaptureState::Ended(CameraError::Unplugged)
        );
        assert_eq!(heard.get(), 3);
    }

    #[test]
    fn a_session_records_while_it_runs_with_movies_among_its_outputs() {
        let cameras = cameras();
        let cell = app_with(&cameras, &MockCompositor::new());
        let camera = opened(&cell);
        let outputs = CameraOutputs::new().movies(MovieOptions::default());
        let session = CameraSession::start(
            &mut cell.borrow_mut(),
            &*camera,
            &camera.formats()[0],
            outputs,
        )
        .unwrap();
        let mut recording = session.start_recording(&cell.borrow(), PathBuf::from("/tmp/clip.mov"));
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(recording.as_mut().poll(&mut context).is_pending());
        session.stop_recording(&cell.borrow());
        assert_eq!(now(recording), Ok(()));
        assert_eq!(
            cameras.sessions()[0].recordings(),
            vec![PathBuf::from("/tmp/clip.mov")]
        );
    }

    #[test]
    fn a_camera_preview_places_the_sessions_preview_layer_at_its_formats_size() {
        let cameras = cameras();
        let compositor = MockCompositor::new();
        let view = Rc::new(TestView::new(400.0, 800.0));
        let cell = app_on(&cameras, &compositor, &view);
        let camera = opened(&cell);
        let layer = cell
            .borrow()
            .platform()
            .compositor()
            .expect("a platform with a compositor")
            .make_layer(false);
        let outputs = CameraOutputs::new().preview(Arc::clone(&layer));
        let session = CameraSession::start(
            &mut cell.borrow_mut(),
            &*camera,
            &camera.formats()[0],
            outputs,
        )
        .unwrap();
        assert!(Arc::ptr_eq(
            &cameras.sessions()[0].preview_layer().unwrap(),
            &layer
        ));
        run_app(
            &mut cell.borrow_mut(),
            crate::Center::new()
                .child(CameraPreview::new(session))
                .into_widget(),
        );
        cell.elapse(Duration::ZERO);
        pump_every_view(&mut cell.borrow_mut(), Duration::ZERO);
        let scene = view.last_scene().unwrap();
        let slices = inset_embedder::slices(&scene, |_| None);
        let (placed, placement) = slices.composition_layers().next().unwrap();
        assert!(Arc::ptr_eq(placed, &layer));
        assert_eq!(
            placement.bounds().size(),
            Size::new(400.0, 225.0),
            "as wide as the view lets it be, at the format's aspect"
        );
    }
}
