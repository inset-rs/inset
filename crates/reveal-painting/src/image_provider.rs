//! Flutter counterpart: `painting/image_provider.dart`, and the load surface of
//! `services/asset_bundle.dart`.
//!
//! A provider names an image and knows how to start it decoding; the cache gives everyone asking
//! for the same one the same decode; and [`ImagePlayback`] drives what the host answers with into
//! the stream a widget listens to, one frame at a time for an animation.

use std::fmt::{self, Debug};
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

use std::time::Duration;

use reveal_embedder::{
    ImageCodec, ImageDecodeError, ImageFrame, ImageRepetition, Locale, Size, TargetPlatform,
};
use reveal_foundation::{App, Handle, Listener, Timer};

use crate::basic_types::TextDirection;
use crate::image_cache::{ImageCache, ImageCacheKey};
use crate::image_stream::{ImageInfo, ImageStream, ImageStreamCompleter};

/// Bytes on their way from a bundle. Dart's `Future<ByteData>`.
pub type AssetFuture = Pin<Box<dyn Future<Output = Result<Vec<u8>, String>>>>;

/// Bytes for a named asset. Flutter `AssetBundle`.
///
/// Answering takes time — a bundle may be reading a file or the network — so it answers with a
/// future, as Dart's does. Flutter's also carries string and structured helpers, its own caching
/// and a network fallback; those are deferred.
pub trait AssetBundle: Debug {
    /// Reads a binary asset. Dart throws when the asset is missing; here that is [`Err`].
    fn load(&self, key: &str) -> AssetFuture;
}

/// Configuration information passed to an image loader.
///
/// All the arguments are optional. Configuration information is merely
/// advisory and best-effort.
pub struct ImageConfiguration {
    /// The preferred [`AssetBundle`] to use if the loader needs one and does
    /// not have one already selected.
    pub bundle: Option<Rc<dyn AssetBundle>>,
    /// The device pixel ratio where the image will be shown.
    pub device_pixel_ratio: Option<f64>,
    /// The language and region for which to select the image.
    pub locale: Option<Locale>,
    /// The reading direction of the language for which to select the image.
    pub text_direction: Option<TextDirection>,
    /// The size at which the image will be rendered.
    pub size: Option<Size>,
    /// The [`TargetPlatform`] for which assets should be used.
    pub platform: Option<TargetPlatform>,
}

impl ImageConfiguration {
    /// An image configuration that provides no additional information.
    ///
    /// Useful when resolving an image without any context.
    pub const EMPTY: ImageConfiguration = ImageConfiguration {
        bundle: None,
        device_pixel_ratio: None,
        locale: None,
        text_direction: None,
        size: None,
        platform: None,
    };

    /// Creates an object holding the configuration information for an image
    /// loader.
    pub fn new(
        bundle: Option<Rc<dyn AssetBundle>>,
        device_pixel_ratio: Option<f64>,
        locale: Option<Locale>,
        text_direction: Option<TextDirection>,
        size: Option<Size>,
        platform: Option<TargetPlatform>,
    ) -> ImageConfiguration {
        ImageConfiguration {
            bundle,
            device_pixel_ratio,
            locale,
            text_direction,
            size,
            platform,
        }
    }

    /// Creates an object holding the configuration information for an image
    /// loader.
    ///
    /// All the arguments are optional. Configuration information is merely
    /// advisory and best-effort.
    pub fn copy_with(
        &self,
        bundle: Option<Rc<dyn AssetBundle>>,
        device_pixel_ratio: Option<f64>,
        locale: Option<Locale>,
        text_direction: Option<TextDirection>,
        size: Option<Size>,
        platform: Option<TargetPlatform>,
    ) -> ImageConfiguration {
        ImageConfiguration {
            bundle: bundle.or_else(|| self.bundle.clone()),
            device_pixel_ratio: device_pixel_ratio.or(self.device_pixel_ratio),
            locale: locale.or_else(|| self.locale.clone()),
            text_direction: text_direction.or(self.text_direction),
            size: size.or(self.size),
            platform: platform.or(self.platform),
        }
    }
}

impl Default for ImageConfiguration {
    fn default() -> ImageConfiguration {
        ImageConfiguration::EMPTY
    }
}

impl Clone for ImageConfiguration {
    fn clone(&self) -> ImageConfiguration {
        ImageConfiguration {
            bundle: self.bundle.clone(),
            device_pixel_ratio: self.device_pixel_ratio,
            locale: self.locale.clone(),
            text_direction: self.text_direction,
            size: self.size,
            platform: self.platform,
        }
    }
}

impl PartialEq for ImageConfiguration {
    fn eq(&self, other: &ImageConfiguration) -> bool {
        bundles_eq(self.bundle.as_ref(), other.bundle.as_ref())
            && self.device_pixel_ratio == other.device_pixel_ratio
            && self.locale == other.locale
            && self.text_direction == other.text_direction
            && self.size == other.size
            && self.platform == other.platform
    }
}

fn bundles_eq(a: Option<&Rc<dyn AssetBundle>>, b: Option<&Rc<dyn AssetBundle>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

impl Debug for ImageConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fields = Vec::new();
        if self.bundle.is_some() {
            fields.push(format!("bundle: {:?}", self.bundle));
        }
        if let Some(dpr) = self.device_pixel_ratio {
            fields.push(format!("devicePixelRatio: {dpr:.1}"));
        }
        if let Some(locale) = &self.locale {
            fields.push(format!("locale: {locale}"));
        }
        if let Some(text_direction) = self.text_direction {
            fields.push(format!("textDirection: {text_direction:?}"));
        }
        if let Some(size) = self.size {
            fields.push(format!("size: {size:?}"));
        }
        if let Some(platform) = self.platform {
            fields.push(format!("platform: {platform:?}"));
        }
        write!(f, "ImageConfiguration({})", fields.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reveal_embedder::TargetPlatform;

    use crate::basic_types::TextDirection;

    #[derive(Debug)]
    struct MemoryBundle {
        bytes: Vec<u8>,
    }

    impl AssetBundle for MemoryBundle {
        fn load(&self, _key: &str) -> AssetFuture {
            Box::pin(std::future::ready(Ok(self.bytes.clone())))
        }
    }

    #[test]
    fn memory_keys_preserve_shared_and_static_identity_without_comparing_contents() {
        let configuration = ImageConfiguration::EMPTY;
        let bytes: Arc<[u8]> = Arc::from(&b"same"[..]);
        let weak = Arc::downgrade(&bytes);
        let key = MemoryImage::new(bytes.clone()).key(&configuration);
        assert_eq!(key, MemoryImage::new(bytes.clone()).key(&configuration));
        assert_ne!(
            key,
            MemoryImage::new(Arc::from(&b"same"[..])).key(&configuration)
        );
        drop(bytes);
        assert!(weak.upgrade().is_some(), "the key owns the allocation");
        drop(key);
        assert!(weak.upgrade().is_none());
        static BYTES: [u8; 4] = *b"same";
        let whole = MemoryImage::from_static(&BYTES).key(&configuration);
        assert_eq!(whole, MemoryImage::from_static(&BYTES).key(&configuration));
        assert_ne!(
            whole,
            MemoryImage::from_static(&BYTES[..2]).key(&configuration)
        );
    }

    #[test]
    fn scales_use_numeric_equality_and_keys_preserve_reflexive_identity() {
        let configuration = ImageConfiguration::EMPTY;
        static BYTES: [u8; 1] = [42];
        let key = |scale| {
            MemoryImage::from_static(&BYTES)
                .scale(scale)
                .key(&configuration)
        };
        let hash = |key: &ImageCacheKey| {
            let mut hasher = std::hash::DefaultHasher::new();
            key.hash(&mut hasher);
            hasher.finish()
        };
        assert_eq!(key(0.0), key(-0.0));
        assert_eq!(hash(&key(0.0)), hash(&key(-0.0)));
        assert_ne!(key(1.0), key(2.0));
        assert_ne!(key(f64::NAN), key(f64::NAN));
        let nan = key(f64::NAN);
        assert_eq!(nan, nan.clone());
    }

    #[test]
    fn asset_keys_include_bundle_identity_and_loading_uses_the_resolved_name() {
        #[derive(Debug)]
        struct RecordingBundle(Rc<std::cell::RefCell<Vec<String>>>);
        impl AssetBundle for RecordingBundle {
            fn load(&self, key: &str) -> AssetFuture {
                self.0.borrow_mut().push(key.to_owned());
                Box::pin(std::future::ready(Ok(vec![1])))
            }
        }
        let requests = Rc::new(std::cell::RefCell::new(Vec::new()));
        let bundle: Rc<dyn AssetBundle> = Rc::new(RecordingBundle(requests.clone()));
        let configuration =
            ImageConfiguration::new(Some(bundle.clone()), None, None, None, None, None);
        let provider = AssetImage::new("original");
        let key = provider.key(&configuration);
        assert_eq!(key, provider.key(&configuration));
        let another = ImageConfiguration::new(
            Some(Rc::new(RecordingBundle(requests.clone()))),
            None,
            None,
            None,
            None,
            None,
        );
        assert_ne!(key, provider.key(&another));
        let cell = reveal_foundation::AppCell::new();
        let mut app = cell.borrow_mut();
        let completer = AssetImage::new("changed").load_image(&mut app, &key);
        assert_eq!(&*requests.borrow(), &["original"]);
        completer.maybe_dispose(&mut app);
        drop(app);
        cell.checkpoint(); // The late asset load must not recreate playback after disposal.
    }

    #[test]
    fn empty_and_copy_with() {
        let empty = ImageConfiguration::EMPTY;
        assert_eq!(empty, ImageConfiguration::default());
        let sized = empty.copy_with(
            None,
            Some(2.0),
            Some(Locale::new("ar")),
            Some(TextDirection::Rtl),
            Some(Size::new(10.0, 20.0)),
            Some(TargetPlatform::IOS),
        );
        assert_eq!(sized.device_pixel_ratio, Some(2.0));
        assert_eq!(sized.locale, Some(Locale::new("ar")));
        assert_eq!(sized.text_direction, Some(TextDirection::Rtl));
        assert_eq!(sized.size, Some(Size::new(10.0, 20.0)));
        assert_eq!(sized.platform, Some(TargetPlatform::IOS));
        assert!(sized.bundle.is_none());
    }

    #[test]
    fn bundle_identity() {
        let a: Rc<dyn AssetBundle> = Rc::new(MemoryBundle {
            bytes: b"one".to_vec(),
        });
        let b: Rc<dyn AssetBundle> = Rc::new(MemoryBundle {
            bytes: b"one".to_vec(),
        });
        let left = ImageConfiguration::new(Some(a.clone()), None, None, None, None, None);
        let same = ImageConfiguration::new(Some(a), None, None, None, None, None);
        let other = ImageConfiguration::new(Some(b), None, None, None, None, None);
        assert_eq!(left, same);
        assert_ne!(left, other);
        let mut loading = left.bundle.as_ref().unwrap().load("k");
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        let std::task::Poll::Ready(Ok(bytes)) = loading.as_mut().poll(&mut context) else {
            panic!("a bundle holding its bytes answers at once");
        };
        assert_eq!(bytes, b"one");
    }
}

/// Something that can produce an image, and knows what identifies it. Flutter `ImageProvider`.
///
/// A provider is asked twice: once for the key that names its image in the cache, and once — only
/// if nothing has it already — to start the decode. Keeping those apart is what lets several
/// widgets showing the same picture share a single decode, and what lets a rebuilt widget paint
/// an image that is already in memory without waiting a frame.
pub trait ImageProvider: Debug + 'static {
    /// What names this image in the cache. Flutter `obtainKey`.
    ///
    /// Two requests for the same image must produce the same key, and two different images must
    /// never share one. Anything that changes the decoded pixels belongs in it, the scale
    /// included.
    fn key(&self, configuration: &ImageConfiguration) -> ImageCacheKey;

    /// Starts decoding, answering with the completer that will carry the frame. Flutter
    /// `loadImage`.
    ///
    /// Called only when the cache has neither the image nor a decode of it already running.
    fn load_image(&self, app: &mut App, key: &ImageCacheKey) -> Handle<ImageStreamCompleter>;

    /// The stream for this image, sharing whatever the cache already has. Flutter `resolve`.
    fn resolve(&self, app: &mut App, configuration: &ImageConfiguration) -> Handle<ImageStream> {
        let stream = ImageStream::new(app);
        let key = self.key(configuration);
        let cache = ImageCache::instance(app);
        let completer = cache.put_if_absent(app, key.clone(), |app| self.load_image(app, &key));
        stream.set_completer(app, completer);
        stream
    }
}

/// The erased provider a widget holds.
pub type ImageProviderRef = Rc<dyn ImageProvider>;

/// Immutable encoded bytes whose identity survives widget rebuilds.
///
/// Static data is borrowed without allocation. Shared data keeps its allocation alive; cloning
/// either variant preserves identity. Conversion for the host occurs only on a cache miss.
#[derive(Clone, Debug)]
pub enum ImageBytes {
    /// Bytes embedded in the application, identified by address and length.
    Static(&'static [u8]),
    /// An owned buffer, identified by its shared allocation.
    Shared(Arc<[u8]>),
}

impl ImageBytes {
    fn for_codec(&self) -> Arc<[u8]> {
        match self {
            Self::Static(bytes) => Arc::from(*bytes),
            Self::Shared(bytes) => bytes.clone(),
        }
    }
}

impl PartialEq for ImageBytes {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Static(a), Self::Static(b)) => {
                a.len() == b.len() && std::ptr::eq(a.as_ptr(), b.as_ptr())
            }
            (Self::Shared(a), Self::Shared(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl Eq for ImageBytes {}

impl Hash for ImageBytes {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Static(bytes) => {
                bytes.as_ptr().hash(state);
                bytes.len().hash(state);
            }
            Self::Shared(bytes) => Arc::as_ptr(bytes).cast::<u8>().hash(state),
        }
    }
}

/// Decodes an image from an immutable buffer. Flutter `MemoryImage`.
#[derive(Clone, Debug)]
pub struct MemoryImage {
    /// The source whose identity is used for cache lookup.
    pub bytes: ImageBytes,
    /// Linear pixels per logical point in the decoded image.
    pub scale: f64,
}

impl MemoryImage {
    /// Creates a provider retaining the shared allocation; clone the Arc to reuse a decode.
    pub fn new(bytes: Arc<[u8]>) -> Self {
        Self {
            bytes: ImageBytes::Shared(bytes),
            scale: 1.0,
        }
    }

    /// Creates a provider for static data without copying it to establish a cache key.
    pub fn from_static(bytes: &'static [u8]) -> Self {
        Self {
            bytes: ImageBytes::Static(bytes),
            scale: 1.0,
        }
    }

    /// Dart `MemoryImage(scale:)`.
    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }
}

/// The retained source and scale identify one decoded memory image.
#[derive(Clone, Debug, PartialEq)]
struct MemoryImageKey {
    bytes: ImageBytes,
    scale: f64,
}

/// Dart's numeric equality treats positive and negative zero as equal.
fn hash_scale<H: Hasher>(scale: f64, state: &mut H) {
    (if scale == 0.0 { 0 } else { scale.to_bits() }).hash(state);
}

impl Hash for MemoryImageKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bytes.hash(state);
        hash_scale(self.scale, state);
    }
}

impl ImageProvider for MemoryImage {
    fn key(&self, _configuration: &ImageConfiguration) -> ImageCacheKey {
        ImageCacheKey::new(MemoryImageKey {
            bytes: self.bytes.clone(),
            scale: self.scale,
        })
    }

    fn load_image(&self, app: &mut App, key: &ImageCacheKey) -> Handle<ImageStreamCompleter> {
        let key = key
            .downcast_ref::<MemoryImageKey>()
            .expect("MemoryImage must load its resolved key");
        decode_into_completer(app, key.bytes.for_codec(), key.scale)
    }
}

/// Decodes an image the application bundles. Flutter `ExactAssetImage`.
///
/// Dart's `AssetImage` also picks among the resolution variants a manifest declares — the two-
/// and three-times copies of the same picture. Without a manifest this names one asset exactly,
/// which is Dart's `ExactAssetImage`; see `PORTING.md`.
#[derive(Clone, Debug)]
pub struct AssetImage {
    /// The asset's name in the bundle.
    pub name: String,
    /// Linear pixels per logical point in the decoded image.
    pub scale: f64,
}

impl AssetImage {
    /// Creates a provider for a bundled asset at the natural scale.
    pub fn new(name: impl Into<String>) -> AssetImage {
        AssetImage {
            name: name.into(),
            scale: 1.0,
        }
    }

    /// Dart `ExactAssetImage(scale:)`.
    pub fn scale(mut self, scale: f64) -> AssetImage {
        self.scale = scale;
        self
    }
}

/// What tells one `AssetImage` from another. Dart's `AssetBundleImageKey`.
#[derive(Clone, Debug)]
struct AssetImageKey {
    bundle: Option<Rc<dyn AssetBundle>>,
    name: String,
    scale: f64,
}

impl PartialEq for AssetImageKey {
    fn eq(&self, other: &Self) -> bool {
        bundles_eq(self.bundle.as_ref(), other.bundle.as_ref())
            && self.name == other.name
            && self.scale == other.scale
    }
}

impl Hash for AssetImageKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bundle
            .as_ref()
            .map(|bundle| Rc::as_ptr(bundle).cast::<()>())
            .hash(state);
        self.name.hash(state);
        hash_scale(self.scale, state);
    }
}

impl ImageProvider for AssetImage {
    fn key(&self, configuration: &ImageConfiguration) -> ImageCacheKey {
        ImageCacheKey::new(AssetImageKey {
            bundle: configuration.bundle.clone(),
            name: self.name.clone(),
            scale: self.scale,
        })
    }

    fn load_image(&self, app: &mut App, key: &ImageCacheKey) -> Handle<ImageStreamCompleter> {
        let key = key
            .downcast_ref::<AssetImageKey>()
            .expect("AssetImage must load its resolved key");
        let completer = ImageStreamCompleter::new(app);
        let Some(bundle) = key.bundle.clone() else {
            completer.report_error(
                app,
                ImageDecodeError::Damaged("no asset bundle in the image configuration".to_owned()),
            );
            return completer;
        };
        let loading = bundle.load(&key.name);
        let scale = key.scale;
        let retained = app.retain(completer);
        // Not held: dropping the task does not cancel it, and the completer is what carries the
        // answer onwards.
        let _loading = app.spawn(async move |async_app| {
            let bytes = loading.await;
            async_app.update(move |app| {
                let completer = retained.get();
                if !app.is_disposed(completer) {
                    match bytes {
                        Ok(bytes) => decode_into(app, completer, Arc::from(bytes), scale),
                        Err(reason) => {
                            completer.report_error(app, ImageDecodeError::Damaged(reason))
                        }
                    }
                }
            });
        });
        completer
    }
}

/// Asks the host to decode `bytes` and drives the answer into a fresh completer.
fn decode_into_completer(
    app: &mut App,
    bytes: Arc<[u8]>,
    scale: f64,
) -> Handle<ImageStreamCompleter> {
    let completer = ImageStreamCompleter::new(app);
    decode_into(app, completer, bytes, scale);
    completer
}

/// Opens `bytes` through the host and drives its frames into `completer`.
///
/// For a still image that is one frame. For an animation it is one after another, each shown for
/// as long as the file asks; [`ImagePlayback`] is what keeps that going, and what stops it while
/// nothing is watching.
fn decode_into(
    app: &mut App,
    completer: Handle<ImageStreamCompleter>,
    bytes: Arc<[u8]>,
    scale: f64,
) {
    ImagePlayback::start(app, completer, bytes, scale);
}

/// How long a frame stays up when its file gives no delay.
///
/// Files routinely record a delay of zero, meaning "as fast as possible", which in practice every
/// browser reads as a tenth of a second. Following that keeps such an animation watchable instead
/// of spending the whole frame budget decoding.
const DEFAULT_FRAME_DURATION: Duration = Duration::from_millis(100);

/// Drives one image: the codec, its first frame, and the rest if it is an animation.
///
/// Dart's `MultiFrameImageStreamCompleter` is a completer that decodes on its own; here the
/// completer only carries frames and this drives them, so one completer serves both kinds of
/// image. The loop is Dart's: decode one frame, show it, wait out its time, decode the next,
/// and stop whenever nothing is listening.
struct ImagePlayback {
    completer: Handle<ImageStreamCompleter>,
    scale: f64,
    /// The host's decoder, once it has answered. Dropping it is what releases the decoder.
    codec: Option<Box<dyn ImageCodec>>,
    /// How many frames have been shown, which is what decides when a repeat count runs out.
    frames_emitted: u32,
    /// The frame decoded and waiting for its turn on screen. Dart's `_nextFrame`.
    ///
    /// It is decoded while the frame before it is still up, so a frame's time on screen is its
    /// own and does not also cover the work of finding the one after it.
    next_frame: Option<ImageFrame>,
    /// Whether the frame on screen has had its time.
    ///
    /// True before anything has been shown, so the first frame goes up as soon as it arrives.
    current_frame_done: bool,
    /// Whether anyone is listening. Nothing is asked for while nobody is.
    watched: bool,
    /// Counts down the current frame's time on screen.
    timer: Option<Timer>,
}

impl ImagePlayback {
    fn start(app: &mut App, completer: Handle<ImageStreamCompleter>, bytes: Arc<[u8]>, scale: f64) {
        let playback = app.create(ImagePlayback {
            completer,
            scale,
            codec: None,
            frames_emitted: 0,
            next_frame: None,
            current_frame_done: true,
            watched: false,
            timer: None,
        });
        completer.set_on_watched(
            app,
            Rc::new(move |app: &mut App, watched| playback.set_watched(app, watched)),
        );
        completer.set_on_disposed(
            app,
            Listener::handle_method(playback, ImagePlayback::dispose),
        );
        // Asked for now rather than inside the task: a spawned body first runs at the next
        // checkpoint, and waiting until then would cost a frame before the host had even heard
        // of the image.
        let opening = app.platform().open_image_codec(bytes);
        // The task is not held: dropping it does not cancel it, and nothing waits on its result
        // — the completer is what carries the answer onwards.
        let retained = app.retain(playback);
        let _opening = app.spawn(async move |async_app| {
            let opened = opening.await;
            async_app.update(move |app| {
                let playback = retained.get();
                if !app.is_disposed(playback) {
                    playback.codec_opened(app, opened);
                }
            });
        });
    }

    /// Releases composed playback resources; in-flight tasks hold only arena retention.
    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(timer) = app.get_mut(self).timer.take() {
            timer.cancel(app);
        }
        app.get_mut(self).codec = None;
        app.get_mut(self).next_frame = None;
        app.destroy(self);
    }

    /// The host answered the open. Dart's `_handleCodecReady`.
    fn codec_opened(
        self: Handle<Self>,
        app: &mut App,
        opened: Result<Box<dyn ImageCodec>, ImageDecodeError>,
    ) {
        let completer = app.get(self).completer;
        match opened {
            Err(error) => completer.report_error(app, error),
            Ok(codec) => {
                app.get_mut(self).codec = Some(codec);
                if app.get(self).watched {
                    self.decode_next(app);
                }
            }
        }
    }

    /// Asks the codec for its next frame, unless one is already on its way.
    ///
    /// The codec moves into the task for as long as the decode takes and comes back with the
    /// frame: a frame borrows the codec it came from, so nothing else can hold it meanwhile. Its
    /// absence here is also what says a decode is already running.
    fn decode_next(self: Handle<Self>, app: &mut App) {
        let Some(mut codec) = app.get_mut(self).codec.take() else {
            return;
        };
        let retained = app.retain(self);
        let _decoding = app.spawn(async move |async_app| {
            let frame = codec.next_frame().await;
            async_app.update(move |app| {
                let playback = retained.get();
                if !app.is_disposed(playback) {
                    playback.frame_decoded(app, codec, frame);
                }
            });
        });
    }

    /// Puts the decoded frame aside for its turn. Dart's `_decodeNextFrameAndSchedule`.
    fn frame_decoded(
        self: Handle<Self>,
        app: &mut App,
        codec: Box<dyn ImageCodec>,
        frame: Result<ImageFrame, ImageDecodeError>,
    ) {
        app.get_mut(self).codec = Some(codec);
        match frame {
            // Dart also keeps the codec here, so a later listener can try again; dropping it
            // would make a transient failure permanent.
            Err(error) => app.get(self).completer.report_error(app, error),
            Ok(frame) => {
                app.get_mut(self).next_frame = Some(frame);
                self.show_when_due(app);
            }
        }
    }

    /// Shows the waiting frame once the one on screen has had its time. Dart's `_handleAppFrame`.
    fn show_when_due(self: Handle<Self>, app: &mut App) {
        if !app.get(self).watched || !app.get(self).current_frame_done {
            return;
        }
        let Some(frame) = app.get_mut(self).next_frame.take() else {
            return;
        };
        self.show(app, frame);
    }

    /// Puts a frame on screen, then starts both its time and the decode of the one after it.
    fn show(self: Handle<Self>, app: &mut App, frame: ImageFrame) {
        let _retained = app.retain(self);
        let scale = app.get(self).scale;
        let completer = app.get(self).completer;
        completer.set_image(app, ImageInfo::new(frame.image).scale(scale));
        if app.is_disposed(self) {
            return;
        }
        app.get_mut(self).frames_emitted += 1;
        if !self.more_to_play(app) {
            // Releasing the codec is what tells the host the image is finished with.
            app.get_mut(self).codec = None;
            return;
        }
        app.get_mut(self).current_frame_done = false;
        let wait = if frame.duration.is_zero() {
            DEFAULT_FRAME_DURATION
        } else {
            frame.duration
        };
        let timer = Timer::new(
            app,
            wait,
            Listener::handle_method(self, ImagePlayback::frame_time_up),
        );
        app.get_mut(self).timer = Some(timer);
        self.decode_next(app);
    }

    /// Whether the animation has passes left to play.
    fn more_to_play(self: Handle<Self>, app: &mut App) -> bool {
        let Some(codec) = app.get(self).codec.as_ref() else {
            return false;
        };
        let frame_count = codec.frame_count().max(1);
        if frame_count <= 1 {
            return false;
        }
        let passes_done = app.get(self).frames_emitted / frame_count;
        match codec.repetition() {
            ImageRepetition::Forever => true,
            ImageRepetition::Once => passes_done < 1,
            ImageRepetition::Times(times) => passes_done <= times,
        }
    }

    /// The frame on screen has had its time; the next one goes up as soon as it is decoded.
    fn frame_time_up(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).timer = None;
        app.get_mut(self).current_frame_done = true;
        self.show_when_due(app);
    }

    /// Starts or stops the animation as listeners come and go.
    ///
    /// An image scrolled out of view has no listeners, and decoding frames nobody sees would cost
    /// the whole animation's work for nothing.
    fn set_watched(self: Handle<Self>, app: &mut App, watched: bool) {
        let _retained = app.retain(self);
        if app.get(self).watched == watched {
            return;
        }
        app.get_mut(self).watched = watched;
        if watched {
            self.show_when_due(app);
            if app.is_disposed(self) {
                return;
            }
            if app.get(self).next_frame.is_none() {
                self.decode_next(app);
            }
            return;
        }
        // A frame's time runs while nothing is watching, as it does in Dart, where the clock the
        // frames are timed against is the display's rather than this animation's own.
        if let Some(timer) = app.get_mut(self).timer.take() {
            timer.cancel(app);
        }
        app.get_mut(self).current_frame_done = true;
    }
}
