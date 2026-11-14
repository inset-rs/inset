//! Cameras: the cameras Android's camera manager lists that take pictures as a camera app does
//! (`BACKWARD_COMPATIBLE`, which leaves out depth sensors), each named by which way it looks, as
//! Android gives a camera no name; listed without asking, and told changed when the list differs
//! after Android says a camera's availability changed. A camera is opened once the app holds the
//! `CAMERA` permission, asked for with the system's question over the activity in front where the
//! app does not (`runtime_permissions`); opening it asks the camera nothing: its formats are read
//! from its characteristics (`camera_formats`), and the camera itself is opened as a session
//! starts, shared by this process's sessions on it (`running_camera`).

use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use ndk_sys::{acamera_metadata_enum_acamera_lens_facing, acamera_metadata_tag};
use waft_embedder::{
    Camera, CameraDevice, CameraError, CameraFacing, CameraFormat, CameraId, CameraListFuture,
    CameraOpenFuture, CameraOutputs, CameraStartFuture, Cameras, CaptureSession, MediaChangeFuture,
    Size,
};

use super::camera_formats::StreamCharacteristics;
use super::camera_manager::CameraManager;
use super::camera_metadata::CameraMetadata;
use super::camera_queue::queue;
use super::capture_session::NdkCaptureSession;
use super::changes::Changes;
use super::orientation::SensorMount;
use super::running_camera::{CameraRun, RunningCamera};
use crate::answer::answer;
use crate::application::Application;
use crate::log::warn_to_log;
use crate::runtime_permissions::{self, Asked, answers_are_heard};
use crate::views::Views;

/// `Manifest.permission.CAMERA`.
const CAMERA: &str = "android.permission.CAMERA";
/// `ACAMERA_REQUEST_AVAILABLE_CAPABILITIES_BACKWARD_COMPATIBLE`.
const BACKWARD_COMPATIBLE: u8 = 0;

/// Android's cameras.
pub(crate) struct NdkCameras {
    manager: &'static CameraManager,
    application: Application,
    /// The views, whose activity in front the permission's question is asked over.
    views: Rc<Views>,
    listed: Arc<ListedCameras>,
}

/// The cameras listed last, and the changes of the list.
#[derive(Default)]
struct ListedCameras {
    ids: Mutex<Vec<String>>,
    changes: Changes,
    listening: OnceLock<()>,
}

impl NdkCameras {
    /// Android's cameras, the permission asked over the activity in front among `views`; `None`
    /// where the system has no camera manager.
    pub(crate) fn new(application: Application, views: Rc<Views>) -> Option<NdkCameras> {
        Some(NdkCameras {
            manager: CameraManager::get()?,
            application,
            views,
            listed: Arc::default(),
        })
    }

    /// Hears the cameras' availability change, from the first listing or wait on, for the process.
    fn listen(&self) {
        let listed = Arc::clone(&self.listed);
        let manager = self.manager;
        self.listed.listening.get_or_init(move || {
            *listed.ids.lock().unwrap_or_else(PoisonError::into_inner) = manager.camera_ids();
            manager.on_availability_change(move || listed.look_again(manager));
        });
    }

    /// Whether the app holds the camera permission.
    fn holds_permission(&self) -> bool {
        let held = self
            .application
            .with(|env, context| runtime_permissions::is_granted(env, context, CAMERA));
        held.unwrap_or_else(|error| {
            warn_to_log!("waft: the camera permission could not be read: {error}");
            self.application.clear_exception();
            false
        })
    }

    /// Asks the user for the camera permission over the activity in front, and answers whether
    /// they granted it; `None` where there is no question to ask: no Waft activity, or none in
    /// front.
    fn ask_permission(&self) -> Option<impl Future<Output = bool> + use<>> {
        if !answers_are_heard(&self.application) {
            return None;
        }
        let in_front = self.views.activity_in_front()?;
        match runtime_permissions::ask(&in_front, CAMERA) {
            Ok(answered) => Some(async move { answered.await == Asked::Granted }),
            Err(error) => {
                warn_to_log!("waft: the camera permission could not be asked for: {error}");
                self.application.clear_exception();
                None
            }
        }
    }
}

impl ListedCameras {
    /// Reads the list again, and tells a change where it differs.
    fn look_again(&self, manager: &CameraManager) {
        let ids = manager.camera_ids();
        let changed = {
            let mut listed = self.ids.lock().unwrap_or_else(PoisonError::into_inner);
            let changed = *listed != ids;
            *listed = ids;
            changed
        };
        if changed {
            self.changes.notify();
        }
    }
}

impl Cameras for NdkCameras {
    fn cameras(&self) -> CameraListFuture {
        self.listen();
        let cameras = cameras_of(self.manager);
        Box::pin(std::future::ready(cameras))
    }

    fn changed(&self) -> MediaChangeFuture {
        self.listen();
        self.listed.changes.next()
    }

    fn open(&self, camera: &Camera) -> CameraOpenFuture {
        let Some(device) =
            NdkCameraDevice::of(self.manager, &camera.id.0, self.application.clone())
        else {
            return Box::pin(std::future::ready(Err(CameraError::Unplugged)));
        };
        if self.holds_permission() {
            return Box::pin(std::future::ready(Ok(
                Box::new(device) as Box<dyn CameraDevice>
            )));
        }
        let Some(asked) = self.ask_permission() else {
            return Box::pin(std::future::ready(Err(CameraError::NotAllowed)));
        };
        Box::pin(async move {
            if asked.await {
                Ok(Box::new(device) as Box<dyn CameraDevice>)
            } else {
                Err(CameraError::NotAllowed)
            }
        })
    }
}

/// The cameras listed now that take pictures, named by which way each looks, in Android's order.
fn cameras_of(manager: &CameraManager) -> Vec<Camera> {
    let mut cameras: Vec<Camera> = Vec::new();
    for id in manager.camera_ids() {
        let Some(characteristics) = manager.characteristics(&id) else {
            continue;
        };
        if !takes_pictures(&characteristics) {
            continue;
        }
        let facing = facing_of(&characteristics);
        let before = cameras
            .iter()
            .filter(|camera| camera.facing == facing)
            .count();
        cameras.push(Camera {
            id: CameraId(id),
            name: name_of(facing, before),
            facing,
        });
    }
    cameras
}

/// Whether the camera takes pictures as a camera app does.
fn takes_pictures(characteristics: &CameraMetadata) -> bool {
    characteristics
        .u8s(acamera_metadata_tag::ACAMERA_REQUEST_AVAILABLE_CAPABILITIES)
        .contains(&BACKWARD_COMPATIBLE)
}

fn facing_of(characteristics: &CameraMetadata) -> CameraFacing {
    let facing = characteristics.u8s(acamera_metadata_tag::ACAMERA_LENS_FACING);
    match facing.first().map(|&facing| u32::from(facing)) {
        Some(facing)
            if facing == acamera_metadata_enum_acamera_lens_facing::ACAMERA_LENS_FACING_FRONT.0 =>
        {
            CameraFacing::Front
        }
        Some(facing)
            if facing == acamera_metadata_enum_acamera_lens_facing::ACAMERA_LENS_FACING_BACK.0 =>
        {
            CameraFacing::Back
        }
        _ => CameraFacing::External,
    }
}

/// The name of a camera looking `facing`, after `before` others that look so: "Back camera",
/// "Back camera 2", as Android's own camera apps call them.
fn name_of(facing: CameraFacing, before: usize) -> String {
    let way = match facing {
        CameraFacing::Front => "Front camera",
        CameraFacing::Back => "Back camera",
        CameraFacing::External => "External camera",
    };
    match before {
        0 => way.to_owned(),
        _ => format!("{way} {}", before + 1),
    }
}

/// A camera the app may use, not opened until a session starts on it.
struct NdkCameraDevice {
    manager: &'static CameraManager,
    id: String,
    streams: StreamCharacteristics,
    mount: SensorMount,
    application: Application,
}

impl NdkCameraDevice {
    /// The camera `id`, as its characteristics describe it; `None` where it is not attached.
    fn of(
        manager: &'static CameraManager,
        id: &str,
        application: Application,
    ) -> Option<NdkCameraDevice> {
        let characteristics = manager.characteristics(id)?;
        let streams = StreamCharacteristics {
            configurations: characteristics
                .i32s(acamera_metadata_tag::ACAMERA_SCALER_AVAILABLE_STREAM_CONFIGURATIONS),
            min_frame_durations: characteristics
                .i64s(acamera_metadata_tag::ACAMERA_SCALER_AVAILABLE_MIN_FRAME_DURATIONS),
            frame_rate_ranges: characteristics
                .i32s(acamera_metadata_tag::ACAMERA_CONTROL_AE_AVAILABLE_TARGET_FPS_RANGES),
        };
        let orientation = characteristics.i32s(acamera_metadata_tag::ACAMERA_SENSOR_ORIENTATION);
        let mount = SensorMount {
            orientation: orientation
                .first()
                .map_or(0, |&degrees| degrees.rem_euclid(360) as u32),
            faces_user: facing_of(&characteristics) == CameraFacing::Front,
        };
        Some(NdkCameraDevice {
            manager,
            id: id.to_owned(),
            streams,
            mount,
            application,
        })
    }

    /// Whether the camera can start in `format` now: the app may use it, it is attached, and it
    /// lists the format.
    fn startable_in(&self, format: &CameraFormat) -> Result<(), CameraError> {
        let held = self
            .application
            .with(|env, context| runtime_permissions::is_granted(env, context, CAMERA));
        if !held.unwrap_or_else(|_| {
            self.application.clear_exception();
            false
        }) {
            return Err(CameraError::NotAllowed);
        }
        if !self.manager.camera_ids().contains(&self.id) {
            return Err(CameraError::Unplugged);
        }
        if !self.formats().contains(format) {
            return Err(CameraError::Failed(
                "the camera does not list this format".to_owned(),
            ));
        }
        Ok(())
    }

    /// How the camera runs in `format` for a first session feeding `outputs`.
    fn run_for(&self, format: &CameraFormat, outputs: &CameraOutputs) -> CameraRun {
        CameraRun {
            id: self.id.clone(),
            format: format.clone(),
            target_frame_rates: self.streams.target_frame_rates(format),
            photo_size: outputs.photos.map(|size| photo_size_in(format, size)),
            mount: self.mount,
        }
    }
}

impl CameraDevice for NdkCameraDevice {
    fn formats(&self) -> Vec<CameraFormat> {
        self.streams.formats()
    }

    /// Fails for upright frames: Android's camera delivers its frames as its sensor takes them,
    /// and each reports its turn and mirror instead.
    fn start(&self, format: &CameraFormat, outputs: CameraOutputs) -> CameraStartFuture {
        if outputs.upright_frames {
            return Box::pin(std::future::ready(Err(CameraError::NotSupported(
                "Android's camera does not turn its frames; each reports its rotation and flip"
                    .to_owned(),
            ))));
        }
        if let Err(error) = self.startable_in(format) {
            return Box::pin(std::future::ready(Err(error)));
        }
        let camera = RunningCamera::share(self.run_for(format, &outputs));
        let session = NdkCaptureSession::new(Arc::clone(&camera), outputs);
        let (started, answered) = answer(Err(CameraError::Failed(
            "the cameras' thread stopped".to_owned(),
        )));
        queue(move || started.give(camera.start()));
        Box::pin(async move {
            answered.await?;
            Ok(Box::new(session) as Box<dyn CaptureSession>)
        })
    }
}

/// The photo size asked, where the format lists it; else the format's largest, as the system's own.
fn photo_size_in(format: &CameraFormat, asked: Size) -> [u32; 2] {
    let size = if format.photo_sizes.contains(&asked) {
        asked
    } else {
        format
            .photo_sizes
            .iter()
            .copied()
            .max_by(|a, b| (a.width() * a.height()).total_cmp(&(b.width() * b.height())))
            .unwrap_or(format.size)
    };
    [size.width() as u32, size.height() as u32]
}
