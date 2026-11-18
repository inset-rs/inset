//! The host capability the framework holds: [`Platform`] and what it hands out.
//!
//! No call here calls the client: a frame asked for is the registry's to hold until the display
//! link's tick draws it (`views.rs`, `host.rs`), a wake the main queue's (`wakes.rs`), and the rest
//! acts at once. What one view asks of the system is that view's host's (`view.rs`); a system sheet
//! the app opens, a document picker or the Open In menu, is presented from its parent view's
//! controller, or the controller of a view in sight (`Views::presenter`).
//!
//! What macOS shares comes from `waft-embedder-apple`: Core Animation's compositor and `IOSurface`
//! buffers, AVFoundation's players, media assets and cameras, Image I/O's codecs, Media
//! Accessibility's caption settings, and the notification center.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use objc2_foundation::{NSLocale, NSString};
use waft_embedder::image_codecs::ImageCodecs;
use waft_embedder::{
    Brightness, Cameras, CaptionSettings, Clipboard, Compositor, Dispatcher, FileDialogs, Files,
    FontSource, HardwareBuffer, HardwareBufferAllocator, HttpClient, ImageCodec, ImageCodecFuture,
    ImageDecodeError, ImageFrame, ImageFrameFuture, ImageRepetition, Locale, MediaAssets,
    Notifications, Offset, Opener, Platform, Players, RequestId, Screen, Screens, SignedDuration,
    SnapshotDelegate, SystemFontSource, TargetPlatform, VideoFrame, VideoFrameReaders, ViewError,
    ViewId, ViewRef, VsyncWaiter, WindowConfig,
};
use waft_embedder_apple::composition::CoreAnimationCompositor;
use waft_embedder_apple::hardware_buffers::SurfaceBufferAllocator;

use crate::display_link::DisplayLink;
use crate::file_dialogs::DocumentPickers;
use crate::gpu::Gpu;
use crate::opener::UiKitOpener;
use crate::pasteboard::Pasteboard;
use crate::view::SCREEN;
use crate::views::Views;
use crate::vsync_waiter::{DisplayLinkWaiter, Waits};
use crate::wakes::{IosDispatcher, Wakes};

/// The long-lived host capability held by the application.
///
/// Beyond [`Platform`], it hands out the renderer's [`image_context`](IosPlatform::image_context),
/// for an image from a texture the app makes itself. An app reaches it with
/// `platform.downcast_ref::<IosPlatform>()`.
pub struct IosPlatform {
    /// The app's views, and the frames they asked for (`views.rs`).
    views: Rc<Views>,
    /// The display link a frame asked for starts.
    display_link: Rc<DisplayLink>,
    /// When the framework asked to be woken, shared with whatever thread asked.
    wakes: Arc<Wakes>,
    /// The waits on the display's refreshes not yet served (`vsync_waiter.rs`).
    vsync_waits: Arc<Waits>,
    origin: Instant,
    brightness: Cell<Brightness>,
    locales: RefCell<Vec<Locale>>,
    compositor: Arc<CoreAnimationCompositor>,
    hardware_buffer_allocator: Arc<SurfaceBufferAllocator>,
    /// The decoders of this build, decoding into `images`; none where they could not start.
    image_loader: Option<valo_codec::ImageLoader>,
    /// The renderer's image store, on the device every view draws with.
    images: valo::ImageContext,
    /// What draws the app's pictures into images there.
    snapshots: SnapshotDelegate,
    clipboard: Pasteboard,
    file_dialogs: DocumentPickers,
    /// `NSURLSession`, which the system's proxy and certificates apply to.
    http: waft_embedder_apple::http::AppleHttpClient,
    /// `std`'s file system on the host's threads.
    files: waft_embedder_apple::files::AppleFiles,
    opener: UiKitOpener,
    notifications: Option<Rc<dyn Notifications>>,
    players: Option<Rc<dyn Players>>,
    media_assets: Option<Rc<dyn MediaAssets>>,
    cameras: Option<Rc<dyn Cameras>>,
    video_frame_readers: Option<Rc<dyn VideoFrameReaders>>,
    image_codecs: Option<Rc<dyn ImageCodecs>>,
    caption_settings: Option<Rc<dyn CaptionSettings>>,
}

impl IosPlatform {
    /// The platform of the app whose views are `views`, drawn with `gpu`, its frames paced by
    /// `display_link`.
    pub(crate) fn new(views: Rc<Views>, gpu: &Gpu, display_link: Rc<DisplayLink>) -> IosPlatform {
        let images = valo::ImageContext::new(gpu.device.clone(), gpu.queue.clone());
        let wakes = Arc::new(Wakes::default());
        let dispatcher: Arc<dyn Dispatcher> = Arc::new(IosDispatcher(Arc::clone(&wakes)));
        IosPlatform {
            http: waft_embedder_apple::http::AppleHttpClient::new(),
            files: waft_embedder_apple::files::AppleFiles::new(dispatcher),
            file_dialogs: DocumentPickers::new(Rc::clone(&views)),
            opener: UiKitOpener::new(Rc::clone(&views)),
            views,
            display_link,
            wakes,
            vsync_waits: Arc::default(),
            origin: Instant::now(),
            brightness: Cell::new(Brightness::Light),
            locales: RefCell::new(preferred_locales()),
            compositor: Arc::new(CoreAnimationCompositor),
            hardware_buffer_allocator: Arc::new(SurfaceBufferAllocator),
            image_loader: image_loader(images.clone()),
            snapshots: SnapshotDelegate::new(images.clone()),
            images,
            clipboard: Pasteboard::general(),
            notifications: waft_embedder_apple::notifications::notifications(),
            players: waft_embedder_apple::media::players(),
            media_assets: waft_embedder_apple::media::media_assets(),
            cameras: waft_embedder_apple::media::cameras(),
            video_frame_readers: waft_embedder_apple::media::video_frame_readers(),
            image_codecs: waft_embedder_apple::image_codecs::image_codecs(),
            caption_settings: waft_embedder_apple::caption_appearance::caption_settings(),
        }
    }

    /// The renderer's image store, on the device every view draws with: an image made through it
    /// is one `RawImage` shows.
    pub fn image_context(&self) -> valo::ImageContext {
        self.images.clone()
    }

    pub(crate) fn elapsed(&self) -> Duration {
        self.elapsed_at(Instant::now())
    }

    /// The platform's clock at `instant`, since its origin.
    pub(crate) fn elapsed_at(&self, instant: Instant) -> Duration {
        instant.saturating_duration_since(self.origin)
    }

    pub(crate) fn wakes(&self) -> &Wakes {
        &self.wakes
    }

    pub(crate) fn vsync_waits(&self) -> &Waits {
        &self.vsync_waits
    }

    /// The user's appearance is `brightness` from now on; answers whether it changed.
    pub(crate) fn set_brightness(&self, brightness: Brightness) -> bool {
        self.brightness.replace(brightness) != brightness
    }

    /// Reads the user's languages again; answers whether they differ from what the app was told.
    pub(crate) fn read_locales(&self) -> bool {
        let locales = preferred_locales();
        if *self.locales.borrow() == locales {
            return false;
        }
        *self.locales.borrow_mut() = locales;
        true
    }

    /// Whether another app changed the pasteboard since the host last looked.
    pub(crate) fn clipboard_has_changed(&self) -> bool {
        self.clipboard.has_changed()
    }
}

/// The user's languages, in the order preferred, each as Foundation reads its tag, listed once.
fn preferred_locales() -> Vec<Locale> {
    let mut locales = Vec::new();
    for tag in NSLocale::preferredLanguages().iter() {
        let locale = locale_of(&tag);
        if !locales.contains(&locale) {
            locales.push(locale);
        }
    }
    locales
}

/// The tag `tag` as `NSLocale` reads it: its language, and its script and region where it names them.
fn locale_of(tag: &NSString) -> Locale {
    let read = NSLocale::localeWithLocaleIdentifier(tag);
    let mut locale = Locale::new(read.languageCode().to_string());
    if let Some(script) = read.scriptCode() {
        locale = locale.script_code(script.to_string());
    }
    // `regionCode`, which replaces it, is iOS 16's, and the app runs on earlier ones.
    #[expect(
        deprecated,
        reason = "the app runs on iOS before 16, which lacks regionCode"
    )]
    let region = read.countryCode();
    if let Some(region) = region {
        locale = locale.country_code(region.to_string());
    }
    locale
}

/// The decoders this build carries, Image I/O's and the software one's, on a worker where it has
/// one.
fn image_loader(images: valo::ImageContext) -> Option<valo_codec::ImageLoader> {
    let mut decoders = waft_embedder_apple::image_io_decoders();
    #[cfg(feature = "image-software")]
    decoders.push(Box::new(valo_codec_software::SoftwareDecoder));
    #[cfg(feature = "image-worker")]
    {
        valo_codec::ImageLoader::with_worker(images, decoders).ok()
    }
    #[cfg(not(feature = "image-worker"))]
    {
        Some(valo_codec::ImageLoader::new(images, decoders))
    }
}

impl Platform for IosPlatform {
    fn target_platform(&self) -> TargetPlatform {
        TargetPlatform::IOS
    }

    /// Held by the registry until the display link's tick finds the view in sight (`views.rs`);
    /// the link runs for one that is news.
    fn request_frame(&self, view: ViewId) {
        if self.views.request_frame(view) {
            self.display_link.set_paused(false);
        }
    }

    fn now(&self) -> Instant {
        Instant::now()
    }

    fn dispatcher(&self) -> Arc<dyn Dispatcher> {
        Arc::new(IosDispatcher(Arc::clone(&self.wakes)))
    }

    /// Every view, in the order they came: the implicit view first.
    fn views(&self) -> Vec<ViewRef> {
        let views = self.views.all();
        views.into_iter().map(|view| view as ViewRef).collect()
    }

    fn view(&self, id: ViewId) -> Option<ViewRef> {
        self.views.get(id).map(|view| view as ViewRef)
    }

    /// None yet: a view is a scene's, and a scene the app asks for, an iPad's second window, is
    /// not asked for here.
    fn request_view(
        &self,
        _config: WindowConfig,
        _initial_data: Option<String>,
    ) -> Result<RequestId, ViewError> {
        Err(ViewError::Unsupported)
    }

    fn platform_brightness(&self) -> Brightness {
        self.brightness.get()
    }

    /// The user's languages, as the app was last told them.
    fn locales(&self) -> Vec<Locale> {
        self.locales.borrow().clone()
    }

    /// The C library's zone, asked on every call, as on macOS (`waft-embedder-apple`'s
    /// `local_time`).
    fn time_zone_offset(&self, seconds_since_epoch: i64) -> SignedDuration {
        waft_embedder_apple::local_time::time_zone_offset(seconds_since_epoch)
    }

    fn time_zone_name(&self, seconds_since_epoch: i64) -> String {
        waft_embedder_apple::local_time::time_zone_name(seconds_since_epoch)
    }

    /// The system's fonts, as its own configuration lists them.
    fn font_source(&self) -> Option<Box<dyn FontSource>> {
        Some(Box::new(SystemFontSource::new(
            valo_system_fonts::fontmgr::platform(),
        )))
    }

    /// The engine's default families on iOS (`txt/platform_mac.mm`).
    fn default_font_families(&self) -> Vec<String> {
        waft_embedder::default_font_families(TargetPlatform::IOS)
    }

    fn open_image_codec(&self, bytes: Arc<[u8]>) -> ImageCodecFuture {
        let loader = self.image_loader.clone();
        Box::pin(async move {
            if bytes.is_empty() {
                return Err(ImageDecodeError::Empty);
            }
            let loader = loader.ok_or(ImageDecodeError::NoDecoder)?;
            let codec = loader
                .open(
                    bytes,
                    valo_codec::DecodeOptions {
                        // Pictures are routinely drawn smaller than they decode, and mip levels are
                        // the difference between a smooth downscale and a shimmering one.
                        mipmaps: true,
                        ..Default::default()
                    },
                )
                .await
                .map_err(as_decode_error)?;
            Ok(Box::new(IosImageCodec { codec }) as Box<dyn ImageCodec>)
        })
    }

    fn import_pixels(&self, pixels: valo::PixelBuffer) -> Option<waft_embedder::Image> {
        self.images.upload_pixels(pixels, false).ok()
    }

    /// The picture drawn on the device every view draws with.
    fn picture_to_image_sync(
        &self,
        picture: &waft_embedder::Picture,
        width: u32,
        height: u32,
    ) -> Option<waft_embedder::Image> {
        Some(
            self.snapshots
                .make_raster_snapshot_sync(picture, width, height),
        )
    }

    /// The frame's buffer, imported as any hardware buffer is.
    fn import_video_frame(&self, frame: &dyn VideoFrame) -> Option<waft_embedder::Image> {
        self.import_hardware_buffer(&*frame.hardware_buffer())
    }

    /// The buffer's texture on the device every view draws with, imported by valo.
    fn import_hardware_buffer(&self, buffer: &dyn HardwareBuffer) -> Option<waft_embedder::Image> {
        let texture =
            waft_embedder_apple::gpu_interop::import_texture(buffer, self.images.device())?;
        self.images.import_texture(texture, false).ok()
    }

    /// Metal's texture over the buffer's `IOSurface` (`waft-embedder-apple`'s `gpu_interop`).
    fn import_texture(
        &self,
        buffer: &dyn HardwareBuffer,
        device: &wgpu::Device,
    ) -> Option<wgpu::Texture> {
        waft_embedder_apple::gpu_interop::import_texture(buffer, device)
    }

    fn hardware_buffer_allocator(&self) -> Option<Arc<dyn HardwareBufferAllocator>> {
        Some(Arc::clone(&self.hardware_buffer_allocator) as Arc<dyn HardwareBufferAllocator>)
    }

    /// Core Animation's, beneath each view's drawing (`picture.rs`).
    fn compositor(&self) -> Option<Arc<dyn Compositor>> {
        Some(Arc::clone(&self.compositor) as Arc<dyn Compositor>)
    }

    /// The display link's ticks, which the frames are drawn at (`vsync_waiter.rs`).
    fn vsync_waiter(&self) -> Option<Arc<dyn VsyncWaiter>> {
        Some(Arc::new(DisplayLinkWaiter(Arc::clone(&self.vsync_waits))))
    }

    fn screens(&self) -> Option<&dyn Screens> {
        Some(self)
    }

    /// The general pasteboard (`pasteboard/`).
    fn clipboard(&self) -> Option<&dyn Clipboard> {
        Some(&self.clipboard)
    }

    /// UIKit's document pickers (`file_dialogs.rs`).
    fn file_dialogs(&self) -> Option<&dyn FileDialogs> {
        Some(&self.file_dialogs)
    }

    /// `NSURLSession`, which uses the system's proxy and trusts the system's certificates.
    fn http(&self) -> Option<&dyn HttpClient> {
        Some(&self.http)
    }

    /// `std`'s file system, its blocking work on the host's threads.
    fn files(&self) -> Option<&dyn Files> {
        Some(&self.files)
    }

    /// UIKit's opening of links, and the Open In menu for files (`opener.rs`).
    fn opener(&self) -> Option<&dyn Opener> {
        Some(&self.opener)
    }

    fn notifications(&self) -> Option<&dyn Notifications> {
        self.notifications.as_deref()
    }

    fn players(&self) -> Option<&dyn Players> {
        self.players.as_deref()
    }

    fn media_assets(&self) -> Option<&dyn MediaAssets> {
        self.media_assets.as_deref()
    }

    fn cameras(&self) -> Option<&dyn Cameras> {
        self.cameras.as_deref()
    }

    fn video_frame_readers(&self) -> Option<&dyn VideoFrameReaders> {
        self.video_frame_readers.as_deref()
    }

    fn image_codecs(&self) -> Option<&dyn ImageCodecs> {
        self.image_codecs.as_deref()
    }

    fn caption_settings(&self) -> Option<&dyn CaptionSettings> {
        self.caption_settings.as_deref()
    }
}

/// An iPhone's one screen, which a finger points at only while it touches it.
impl Screens for IosPlatform {
    fn screens(&self) -> Vec<Screen> {
        let Some(main_thread) = objc2::MainThreadMarker::new() else {
            return Vec::new();
        };
        #[allow(deprecated)]
        let screen = objc2_ui_kit::UIScreen::mainScreen(main_thread);
        let bounds = screen.bounds();
        let frame = waft_embedder::Rect::from_ltwh(
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.width,
            bounds.size.height,
        );
        vec![Screen {
            id: SCREEN,
            name: String::new(),
            frame,
            visible_frame: frame,
            scale: screen.scale(),
            refresh_rate: Some(screen.maximumFramesPerSecond() as f64),
        }]
    }

    fn pointer_location(&self) -> Option<Offset> {
        None
    }
}

/// Adapts valo's drawable frames to the framework's codec contract.
struct IosImageCodec {
    codec: valo_codec::Codec,
}

impl ImageCodec for IosImageCodec {
    fn frame_count(&self) -> u32 {
        self.codec.info().frame_count
    }

    fn repetition(&self) -> ImageRepetition {
        match self.codec.info().repetition {
            valo_codec::Repetition::Once => ImageRepetition::Once,
            valo_codec::Repetition::Times(times) => ImageRepetition::Times(times),
            valo_codec::Repetition::Forever => ImageRepetition::Forever,
        }
    }

    fn next_frame(&mut self) -> ImageFrameFuture<'_> {
        let frame = self.codec.next_frame();
        Box::pin(async move {
            let frame = frame.await.map_err(as_decode_error)?;
            Ok(ImageFrame {
                image: frame.image,
                duration: frame.duration,
            })
        })
    }
}

fn as_decode_error(error: valo_codec::DecodeError) -> ImageDecodeError {
    match error {
        valo_codec::DecodeError::NoDecoder => ImageDecodeError::NoDecoder,
        valo_codec::DecodeError::Unsupported(_) => ImageDecodeError::UnknownFormat,
        valo_codec::DecodeError::InvalidData(reason) => ImageDecodeError::Damaged(reason),
        other => ImageDecodeError::Failed(other.to_string()),
    }
}
