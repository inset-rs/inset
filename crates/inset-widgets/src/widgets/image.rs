//! Flutter counterpart: `widgets/image.dart`.
//!
//! [`Image`] resolves a provider and shows what comes back; [`RawImage`] draws an image already
//! in hand. Everything a provider needs — the cache, the decode — is below in painting, so this
//! layer is only about listening for the frame and rebuilding when it arrives.
//!
//! `precacheImage`, the frame-by-frame and loading builders, and the semantics wrapper are not
//! here; see `PORTING.md`.

use std::rc::Rc;

use inset_embedder::{ImageDecodeError, Size, TextDirection};
use inset_foundation::{App, Handle};
use inset_painting::{
    Alignment, BoxFit, ImageConfiguration, ImageInfo, ImageProviderRef, ImageRepeat, ImageStream,
    ImageStreamListener, MemoryImage,
};
use inset_rendering::{AnyRenderObject, RenderBox, RenderHandle, RenderImage};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, LeafRenderObjectWidget, RenderObjectWidget, State, StateData,
    StatefulWidget, WidgetRef,
};
use crate::widgets::basic::Directionality;
use crate::widgets::localizations::Localizations;
use crate::widgets::media_query::MediaQuery;

/// Creates an [`ImageConfiguration`] based on the given [`BuildContext`] (and optionally
/// size).
///
/// This is the object that must be passed to `BoxPainter.paint` and to
/// `ImageProvider.resolve`.
///
/// If this is not called from a build method, then it should be reevaluated each time the
/// dependencies change, e.g. by calling it from `State::did_change_dependencies`, so that
/// any changes in the environment cause the image to be re-resolved.
///
/// The asset bundle waits with `DefaultAssetBundle`.
pub fn create_local_image_configuration(
    app: &mut App,
    context: BuildContext,
    size: Option<Size>,
) -> ImageConfiguration {
    ImageConfiguration {
        bundle: None,
        device_pixel_ratio: Some(
            MediaQuery::maybe_device_pixel_ratio_of(app, context).unwrap_or(1.0),
        ),
        locale: Localizations::maybe_locale_of(app, context),
        text_direction: Directionality::maybe_of(app, context),
        size,
        platform: Some(app.platform().target_platform()),
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::cell::RefCell;
    use std::rc::Rc;

    use inset_embedder::TextDirection;

    use super::*;
    use crate::framework::{IntoWidget, WidgetRef};
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Builder, SizedBox};
    use crate::widgets::media_query::MediaQueryData;

    type Seen = Rc<RefCell<Option<ImageConfiguration>>>;

    /// A leaf that records the configuration its context yields.
    fn probe(seen: &Seen, size: Option<Size>) -> WidgetRef {
        let seen = Rc::clone(seen);
        Builder::new(move |app, context| {
            *seen.borrow_mut() = Some(create_local_image_configuration(app, context, size));
            SizedBox::shrink().into_widget()
        })
        .into_widget()
    }

    #[test]
    fn the_configuration_reads_the_direction_pixel_ratio_size_and_platform() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let seen: Seen = Rc::default();
        let tree = Directionality::new(
            TextDirection::Rtl,
            MediaQuery::new(
                MediaQueryData::new().device_pixel_ratio(3.0),
                probe(&seen, Some(Size::new(1.0, 2.0))),
            )
            .into_widget(),
        )
        .into_widget();
        let harness = Harness::mount(&mut app, tree);
        harness.pump(&mut app);

        let configuration = seen.borrow_mut().take().expect("the builder ran");
        assert_eq!(configuration.text_direction, Some(TextDirection::Rtl));
        assert_eq!(configuration.device_pixel_ratio, Some(3.0));
        assert_eq!(configuration.size, Some(Size::new(1.0, 2.0)));
        assert_eq!(
            configuration.platform,
            Some(app.platform().target_platform())
        );
        assert!(configuration.bundle.is_none());
    }

    #[test]
    fn without_ancestors_the_configuration_falls_back() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let seen: Seen = Rc::default();
        let harness = Harness::mount(&mut app, probe(&seen, None));
        harness.pump(&mut app);

        let configuration = seen.borrow_mut().take().expect("the builder ran");
        assert_eq!(configuration.text_direction, None);
        assert_eq!(configuration.device_pixel_ratio, Some(1.0));
        assert_eq!(configuration.size, None);
    }
}

/// Draws an image that is already decoded. Flutter `RawImage`.
///
/// Most callers want [`Image`], which finds the image first. This is the widget underneath it,
/// and what to use when the image is already in hand.
#[derive(Clone, Debug)]
pub struct RawImage {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,
    /// The image to draw, or nothing while it is still on its way.
    pub image: Option<ImageInfo>,
    /// The width to take, or `None` to follow the image.
    pub width: Option<f64>,
    /// The height to take, or `None` to follow the image.
    pub height: Option<f64>,
    /// How opaque to draw it, from zero to one.
    pub opacity: f64,
    /// How to fit it into the space it is given.
    pub fit: Option<BoxFit>,
    /// Where it sits when it does not fill that space.
    pub alignment: Alignment,
    /// What fills the space it does not cover.
    pub repeat: ImageRepeat,
    /// Whether to mirror it in a right-to-left layout.
    pub match_text_direction: bool,
}

impl RawImage {
    /// Creates a [`RawImage`] showing `image`, or nothing when it is `None`.
    pub fn new(image: Option<ImageInfo>) -> RawImage {
        RawImage {
            key: None,
            image,
            width: None,
            height: None,
            opacity: 1.0,
            fit: None,
            alignment: Alignment::CENTER,
            repeat: ImageRepeat::NoRepeat,
            match_text_direction: false,
        }
    }

    /// Dart `RawImage(key:)`.
    pub fn key(mut self, key: KeyRef) -> RawImage {
        self.key = Some(key);
        self
    }

    /// Dart `RawImage(width:)`.
    pub fn width(mut self, width: f64) -> RawImage {
        self.width = Some(width);
        self
    }

    /// Dart `RawImage(height:)`.
    pub fn height(mut self, height: f64) -> RawImage {
        self.height = Some(height);
        self
    }

    /// Dart `RawImage(opacity:)`.
    pub fn opacity(mut self, opacity: f64) -> RawImage {
        self.opacity = opacity;
        self
    }

    /// Dart `RawImage(fit:)`.
    pub fn fit(mut self, fit: BoxFit) -> RawImage {
        self.fit = Some(fit);
        self
    }

    /// Dart `RawImage(alignment:)`.
    pub fn alignment(mut self, alignment: Alignment) -> RawImage {
        self.alignment = alignment;
        self
    }

    /// Dart `RawImage(repeat:)`.
    pub fn repeat(mut self, repeat: ImageRepeat) -> RawImage {
        self.repeat = repeat;
        self
    }

    /// Dart `RawImage(matchTextDirection:)`.
    pub fn match_text_direction(mut self, match_text_direction: bool) -> RawImage {
        self.match_text_direction = match_text_direction;
        self
    }

    /// The reading direction the render object needs, which is only read when mirroring.
    fn resolved_text_direction(
        &self,
        app: &mut App,
        context: BuildContext,
    ) -> Option<TextDirection> {
        self.match_text_direction
            .then(|| Directionality::maybe_of(app, context))
            .flatten()
    }
}

impl RenderObjectWidget for RawImage {
    type RenderObject = RenderImage;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_render_object(&self, app: &mut App, context: BuildContext) -> AnyRenderObject {
        let render = RenderImage::new(app);
        self.update_render_object(app, context, render);
        render.as_object()
    }

    fn update_render_object(
        &self,
        app: &mut App,
        context: BuildContext,
        render_object: RenderHandle<RenderImage>,
    ) {
        let text_direction = self.resolved_text_direction(app, context);
        render_object.set_image(app, self.image.clone());
        render_object.set_width(app, self.width);
        render_object.set_height(app, self.height);
        render_object.set_opacity(app, self.opacity);
        render_object.set_fit(app, self.fit);
        render_object.set_alignment(app, self.alignment);
        render_object.set_repeat(app, self.repeat);
        render_object.set_match_text_direction(app, self.match_text_direction);
        render_object.set_text_direction(app, text_direction);
    }
}

impl LeafRenderObjectWidget for RawImage {}

/// Shows an image from a provider. Flutter `Image`.
///
/// The image arrives later than the widget, so nothing is drawn on the first frame unless the
/// cache already had it — in which case it is there immediately, which is what stops a scrolling
/// list from flickering.
#[derive(Clone, Debug)]
pub struct Image {
    /// See [`Widget::key`](crate::Widget::key).
    pub key: Option<KeyRef>,
    /// Where the image comes from.
    pub image: ImageProviderRef,
    /// The width to take, or `None` to follow the image.
    pub width: Option<f64>,
    /// The height to take, or `None` to follow the image.
    pub height: Option<f64>,
    /// How opaque to draw it, from zero to one.
    pub opacity: f64,
    /// How to fit it into the space it is given.
    pub fit: Option<BoxFit>,
    /// Where it sits when it does not fill that space.
    pub alignment: Alignment,
    /// What fills the space it does not cover.
    pub repeat: ImageRepeat,
    /// Whether to mirror it in a right-to-left layout.
    pub match_text_direction: bool,
}

impl Image {
    /// Creates an [`Image`] from any provider.
    pub fn new(image: ImageProviderRef) -> Image {
        Image {
            key: None,
            image,
            width: None,
            height: None,
            opacity: 1.0,
            fit: None,
            alignment: Alignment::CENTER,
            repeat: ImageRepeat::NoRepeat,
            match_text_direction: false,
        }
    }

    /// Creates an [`Image`] from encoded bytes already in memory. Flutter `Image.memory`.
    pub fn memory(bytes: std::sync::Arc<[u8]>) -> Image {
        Image::new(Rc::new(MemoryImage::new(bytes)) as ImageProviderRef)
    }

    /// Creates an image from static bytes, preserving their identity across rebuilds.
    pub fn memory_static(bytes: &'static [u8]) -> Image {
        Image::new(Rc::new(MemoryImage::from_static(bytes)) as ImageProviderRef)
    }

    /// Dart `Image(key:)`.
    pub fn key(mut self, key: KeyRef) -> Image {
        self.key = Some(key);
        self
    }

    /// Dart `Image(width:)`.
    pub fn width(mut self, width: f64) -> Image {
        self.width = Some(width);
        self
    }

    /// Dart `Image(height:)`.
    pub fn height(mut self, height: f64) -> Image {
        self.height = Some(height);
        self
    }

    /// Dart `Image(opacity:)`.
    pub fn opacity(mut self, opacity: f64) -> Image {
        self.opacity = opacity;
        self
    }

    /// Dart `Image(fit:)`.
    pub fn fit(mut self, fit: BoxFit) -> Image {
        self.fit = Some(fit);
        self
    }

    /// Dart `Image(alignment:)`.
    pub fn alignment(mut self, alignment: Alignment) -> Image {
        self.alignment = alignment;
        self
    }

    /// Dart `Image(repeat:)`.
    pub fn repeat(mut self, repeat: ImageRepeat) -> Image {
        self.repeat = repeat;
        self
    }

    /// Dart `Image(matchTextDirection:)`.
    pub fn match_text_direction(mut self, match_text_direction: bool) -> Image {
        self.match_text_direction = match_text_direction;
        self
    }
}

impl StatefulWidget for Image {
    type State = ImageState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ImageState {
        ImageState {
            state: StateData::new(),
            stream: None,
            listener: None,
            resolved: None,
            error: None,
        }
    }
}

/// Listens for [`Image`]'s frame and rebuilds when it arrives. Flutter `_ImageState`.
pub struct ImageState {
    state: StateData<Image>,
    stream: Option<Handle<ImageStream>>,
    listener: Option<ImageStreamListener>,
    resolved: Option<ImageInfo>,
    error: Option<ImageDecodeError>,
}

impl ImageState {
    /// Why the image will never arrive, if it failed.
    ///
    /// Dart hands a failure to `FlutterError.onError` and, if given, an error builder. Neither
    /// is here, so this is how a caller finds out.
    pub fn error(self: Handle<Self>, app: &App) -> Option<ImageDecodeError> {
        app.get(self).error.clone()
    }

    /// Resolves the provider and starts listening, dropping any earlier stream.
    ///
    /// Resolving may answer within the call when the cache already holds the image, which is why
    /// the listener records into the state directly rather than assuming a later frame.
    fn resolve(self: Handle<Self>, app: &mut App, context: BuildContext) {
        let configuration = create_local_image_configuration(app, context, None);
        let provider = Rc::clone(&self.widget(app).image);
        let stream = provider.resolve(app, &configuration);
        if let Some(previous) = app.get(self).stream
            && previous.completer(app) == stream.completer(app)
        {
            stream.dispose(app);
            return;
        }
        self.stop_listening(app);
        if let Some(previous) = app.get_mut(self).stream.take() {
            previous.dispose(app);
        }
        app.get_mut(self).stream = Some(stream);

        let on_image = self;
        let on_error = self;
        let listener = ImageStreamListener::new(Rc::new(
            move |app: &mut App, info: &ImageInfo, _synchronous| {
                let info = info.clone();
                on_image.set_state(app, move |state| {
                    state.resolved = Some(info);
                    state.error = None;
                });
            },
        ))
        .on_error(Rc::new(move |app: &mut App, error: &ImageDecodeError| {
            let error = error.clone();
            on_error.set_state(app, move |state| {
                state.error = Some(error);
            });
        }));
        app.get_mut(self).listener = Some(listener.clone());
        stream.add_listener(app, listener);
    }

    fn stop_listening(self: Handle<Self>, app: &mut App) {
        let (Some(stream), Some(listener)) =
            (app.get(self).stream, app.get_mut(self).listener.take())
        else {
            return;
        };
        stream.remove_listener(app, &listener);
    }
}

impl State for ImageState {
    type Widget = Image;
    crate::state_accessors!();

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        self.resolve(app, context);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &Image) {
        if !Rc::ptr_eq(&self.widget(app).image, &old_widget.image) {
            let context = self.context(app);
            self.resolve(app, context);
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.stop_listening(app);
        if let Some(stream) = app.get_mut(self).stream.take() {
            stream.dispose(app);
        }
        app.get_mut(self).resolved = None;
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let widget = self.widget(app).clone();
        let mut raw = RawImage::new(app.get(self).resolved.clone())
            .opacity(widget.opacity)
            .alignment(widget.alignment)
            .repeat(widget.repeat)
            .match_text_direction(widget.match_text_direction);
        if let Some(width) = widget.width {
            raw = raw.width(width);
        }
        if let Some(height) = widget.height {
            raw = raw.height(height);
        }
        if let Some(fit) = widget.fit {
            raw = raw.fit(fit);
        }
        raw.into_widget()
    }
}

#[cfg(test)]
mod image_widget_tests {
    use inset_painting::ImageProvider;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    use inset_embedder::test_support::solid_image;
    use inset_embedder::{
        ImageCodec, ImageCodecFuture, ImageFrame, ImageFrameFuture, ImageRepetition, Platform,
        PlatformRef, TargetPlatform, ViewId, ViewRef,
    };
    use inset_foundation::AppCell;
    use inset_rendering::RenderImage;

    use super::*;
    use crate::framework::IntoWidget;
    use crate::test_harness::Harness;

    /// A codec over pictures the test wrote out in advance.
    struct TestCodec {
        colours: Vec<[u8; 4]>,
        repetition: ImageRepetition,
        frame_duration: Duration,
        next_index: usize,
        /// How many frames have been asked for, which is what says when they were asked for.
        asked: Rc<Cell<u32>>,
        dropped: Rc<Cell<u32>>,
        frame_gate: Option<inset_foundation::CompleterFuture<()>>,
    }

    impl Drop for TestCodec {
        fn drop(&mut self) {
            self.dropped.set(self.dropped.get() + 1);
        }
    }

    impl ImageCodec for TestCodec {
        fn frame_count(&self) -> u32 {
            self.colours.len() as u32
        }

        fn repetition(&self) -> ImageRepetition {
            self.repetition
        }

        fn next_frame(&mut self) -> ImageFrameFuture<'_> {
            self.asked.set(self.asked.get() + 1);
            let colour = self.colours[self.next_index];
            self.next_index = (self.next_index + 1) % self.colours.len();
            let gate = self.frame_gate.clone();
            let duration = self.frame_duration;
            Box::pin(async move {
                if let Some(gate) = gate {
                    gate.await;
                }
                Ok(ImageFrame {
                    image: solid_image([30, 10], colour),
                    duration,
                })
            })
        }
    }

    /// A host that answers with a codec at once, and counts how often it was asked.
    struct TestHost {
        opens: RefCell<u32>,
        colours: Vec<[u8; 4]>,
        repetition: ImageRepetition,
        frame_duration: Duration,
        asked: Rc<Cell<u32>>,
        dropped: Rc<Cell<u32>>,
        open_gate: Option<inset_foundation::CompleterFuture<()>>,
        frame_gate: Option<inset_foundation::CompleterFuture<()>>,
    }

    impl TestHost {
        fn still() -> TestHost {
            TestHost {
                opens: RefCell::new(0),
                colours: vec![[0, 128, 255, 255]],
                repetition: ImageRepetition::Once,
                frame_duration: Duration::ZERO,
                asked: Rc::new(Cell::new(0)),
                dropped: Rc::new(Cell::new(0)),
                open_gate: None,
                frame_gate: None,
            }
        }

        fn animation() -> TestHost {
            TestHost {
                opens: RefCell::new(0),
                colours: vec![[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]],
                repetition: ImageRepetition::Forever,
                frame_duration: Duration::from_millis(40),
                asked: Rc::new(Cell::new(0)),
                dropped: Rc::new(Cell::new(0)),
                open_gate: None,
                frame_gate: None,
            }
        }
    }

    impl Platform for TestHost {
        fn target_platform(&self) -> TargetPlatform {
            TargetPlatform::MacOS
        }

        fn open_image_codec(&self, _bytes: std::sync::Arc<[u8]>) -> ImageCodecFuture {
            *self.opens.borrow_mut() += 1;
            let codec = Box::new(TestCodec {
                colours: self.colours.clone(),
                repetition: self.repetition,
                frame_duration: self.frame_duration,
                next_index: 0,
                asked: Rc::clone(&self.asked),
                dropped: self.dropped.clone(),
                frame_gate: self.frame_gate.clone(),
            }) as Box<dyn ImageCodec>;
            let gate = self.open_gate.clone();
            Box::pin(async move {
                if let Some(gate) = gate {
                    gate.await;
                }
                Ok(codec)
            })
        }

        fn request_frame(&self) {}

        fn now(&self) -> Instant {
            Instant::now()
        }

        fn wake_at(&self, _deadline: Instant) {}

        fn views(&self) -> Vec<ViewRef> {
            Vec::new()
        }

        fn view(&self, _id: ViewId) -> Option<ViewRef> {
            None
        }

        fn implicit_view(&self) -> Option<ViewRef> {
            None
        }
    }

    /// A host with no decoder at all, which is the trait's default.
    struct SilentHost;

    impl Platform for SilentHost {
        fn target_platform(&self) -> TargetPlatform {
            TargetPlatform::MacOS
        }

        fn request_frame(&self) {}

        fn now(&self) -> Instant {
            Instant::now()
        }

        fn wake_at(&self, _deadline: Instant) {}

        fn views(&self) -> Vec<ViewRef> {
            Vec::new()
        }

        fn view(&self, _id: ViewId) -> Option<ViewRef> {
            None
        }

        fn implicit_view(&self) -> Option<ViewRef> {
            None
        }
    }

    /// The one image render object in the mounted tree.
    fn mounted_image(app: &App, harness: &Harness) -> RenderHandle<RenderImage> {
        let mut found = None;
        let mut stack = vec![harness.render_root(app).as_object()];
        while let Some(object) = stack.pop() {
            if let Some(image) = object.downcast::<RenderImage>(app) {
                found = Some(image);
            }
            object.visit_children(app, &mut |child| stack.push(child));
        }
        found.expect("the tree has an image")
    }

    fn mount_image(cell: &Rc<AppCell>, widget: WidgetRef) -> Harness {
        let mut app = cell.borrow_mut();
        let harness = Harness::mount(&mut app, widget);
        harness.pump(&mut app);
        harness
    }

    #[test]
    fn an_image_appears_once_the_host_answers_and_takes_the_size_it_decoded() {
        let host = Rc::new(TestHost::still());
        let cell = AppCell::with_platform(Rc::clone(&host) as PlatformRef);
        let harness = mount_image(&cell, Image::memory_static(&b"a picture"[..]).into_widget());
        {
            let app = cell.borrow();
            let image = mounted_image(&app, &harness);
            assert!(image.image(&app).is_none(), "nothing yet");
            assert_eq!(image.size(&app), Size::new(0.0, 0.0));
        }

        // The open and the first frame both resolve at the checkpoint, as every awaited answer
        // does.
        cell.checkpoint();
        let mut app = cell.borrow_mut();
        harness.pump(&mut app);

        let image = mounted_image(&app, &harness);
        assert!(image.image(&app).is_some(), "the frame reached the widget");
        assert_eq!(
            image.size(&app),
            Size::new(30.0, 10.0),
            "the box takes the size the host decoded"
        );
    }

    #[test]
    fn a_host_with_no_decoder_leaves_the_box_empty() {
        let cell = AppCell::with_platform(Rc::new(SilentHost) as PlatformRef);
        let harness = mount_image(&cell, Image::memory_static(&b"a picture"[..]).into_widget());
        cell.checkpoint();

        let mut app = cell.borrow_mut();
        harness.pump(&mut app);
        let image = mounted_image(&app, &harness);
        assert!(image.image(&app).is_none());
        assert_eq!(image.size(&app), Size::new(0.0, 0.0));
    }

    #[test]
    fn two_widgets_showing_the_same_bytes_share_one_decode() {
        let host = Rc::new(TestHost::still());
        let cell = AppCell::with_platform(Rc::clone(&host) as PlatformRef);
        let bytes: std::sync::Arc<[u8]> = std::sync::Arc::from(&b"one picture"[..]);
        let provider: ImageProviderRef = Rc::new(MemoryImage::new(bytes));
        mount_image(
            &cell,
            crate::widgets::basic::Column::new()
                .children([
                    Image::new(Rc::clone(&provider)).into_widget(),
                    Image::new(provider).into_widget(),
                ])
                .into_widget(),
        );

        assert_eq!(
            *host.opens.borrow(),
            1,
            "the cache shared the first decode with the second widget"
        );
    }

    #[test]
    fn an_animation_shows_a_new_frame_when_the_last_ones_time_is_up() {
        let host = Rc::new(TestHost::animation());
        let cell = AppCell::with_platform(Rc::clone(&host) as PlatformRef);
        let harness = mount_image(
            &cell,
            Image::memory_static(&b"an animation"[..]).into_widget(),
        );
        cell.checkpoint();

        let first = {
            let mut app = cell.borrow_mut();
            harness.pump(&mut app);
            mounted_image(&app, &harness)
                .image(&app)
                .expect("the first frame")
                .image
        };

        // Nothing changes until the frame's own time has passed.
        cell.elapse(Duration::from_millis(10));
        {
            let app = cell.borrow();
            let showing = mounted_image(&app, &harness).image(&app).expect("a frame");
            assert_eq!(showing.image, first, "still the first frame");
        }

        cell.elapse(Duration::from_millis(50));
        let mut app = cell.borrow_mut();
        harness.pump(&mut app);
        let showing = mounted_image(&app, &harness).image(&app).expect("a frame");
        assert_ne!(showing.image, first, "the animation moved on");
    }

    #[test]
    fn the_frame_after_the_one_on_screen_is_asked_for_while_it_is_still_up() {
        let host = Rc::new(TestHost::animation());
        let cell = AppCell::with_platform(Rc::clone(&host) as PlatformRef);
        let harness = mount_image(
            &cell,
            Image::memory_static(&b"an animation"[..]).into_widget(),
        );
        cell.checkpoint();
        {
            let mut app = cell.borrow_mut();
            harness.pump(&mut app);
        }
        cell.checkpoint();
        // The one on screen and the one being got ready: a frame's time on screen is its own,
        // rather than also covering the decode of the frame after it.
        assert_eq!(
            host.asked.get(),
            2,
            "asked before any of the frame's time passed"
        );
    }

    #[test]
    fn an_animation_nobody_is_watching_stops_between_frames() {
        let host = Rc::new(TestHost::animation());
        let cell = AppCell::with_platform(Rc::clone(&host) as PlatformRef);
        let bytes: std::sync::Arc<[u8]> = std::sync::Arc::from(&b"an animation"[..]);
        let provider: ImageProviderRef = Rc::new(MemoryImage::new(bytes));

        let stream = {
            let mut app = cell.borrow_mut();
            let configuration = ImageConfiguration::EMPTY;
            provider.resolve(&mut app, &configuration)
        };
        cell.checkpoint();

        // A listener of our own, so the playback has someone to run for.
        let seen: Rc<RefCell<u32>> = Rc::default();
        let counting = Rc::clone(&seen);
        let listener = ImageStreamListener::new(Rc::new(move |_app: &mut App, _info, _| {
            *counting.borrow_mut() += 1;
        }));
        {
            let mut app = cell.borrow_mut();
            stream.add_listener(&mut app, listener.clone());
        }
        cell.elapse(Duration::from_millis(200));
        let while_watched = *seen.borrow();
        assert!(while_watched >= 2, "frames arrived while watched");

        {
            let mut app = cell.borrow_mut();
            stream.remove_listener(&mut app, &listener);
        }
        cell.elapse(Duration::from_millis(500));
        assert_eq!(
            *seen.borrow(),
            while_watched,
            "nothing decoded once nobody was listening"
        );
    }
    fn finish_frame(app: &mut App) {
        inset_scheduler::SchedulerBinding::handle_begin_frame(app, Some(Duration::ZERO));
        app.drain_microtasks();
        inset_scheduler::SchedulerBinding::handle_draw_frame(app);
        app.drain_microtasks();
    }

    #[test]
    fn late_open_and_frame_results_release_their_codec_after_stream_disposal() {
        for delay_open in [true, false] {
            let gate = inset_foundation::Completer::new();
            let mut host = TestHost::animation();
            if delay_open {
                host.open_gate = Some(gate.future());
            } else {
                host.frame_gate = Some(gate.future());
            }
            let host = Rc::new(host);
            let cell = AppCell::with_platform(host.clone() as PlatformRef);
            let provider = MemoryImage::from_static(b"late");
            let stream = {
                let mut app = cell.borrow_mut();
                provider.resolve(&mut app, &ImageConfiguration::EMPTY)
            };
            cell.checkpoint();
            let completer;
            {
                let mut app = cell.borrow_mut();
                completer = stream.completer(&app).unwrap();
                let cache = inset_painting::ImageCache::instance(&mut app);
                cache.clear(&mut app);
                stream.dispose(&mut app);
                finish_frame(&mut app);
            }
            cell.checkpoint();
            assert!(!cell.borrow().contains(stream));
            assert!(!cell.borrow().contains(completer));
            assert_eq!(
                host.dropped.get(),
                0,
                "the decode operation still owns its codec"
            );
            gate.complete(&mut cell.borrow_mut(), ());
            cell.checkpoint();
            assert_eq!(
                host.dropped.get(),
                1,
                "the late result is dropped without reviving playback"
            );
        }
    }

    #[test]
    fn cached_animation_disposes_its_codec_after_the_final_cache_hold_leaves() {
        let host = Rc::new(TestHost::animation());
        let cell = AppCell::with_platform(host.clone() as PlatformRef);
        let stream = {
            let mut app = cell.borrow_mut();
            MemoryImage::from_static(b"animation").resolve(&mut app, &ImageConfiguration::EMPTY)
        };
        cell.checkpoint();
        let completer;
        {
            let mut app = cell.borrow_mut();
            completer = stream.completer(&app).unwrap();
            finish_frame(&mut app);
            stream.dispose(&mut app);
            assert_eq!(
                host.dropped.get(),
                0,
                "LRU retention keeps the paused codec reusable"
            );
            let cache = inset_painting::ImageCache::instance(&mut app);
            cache.clear(&mut app);
            finish_frame(&mut app);
        }
        cell.checkpoint();
        assert_eq!(host.dropped.get(), 1);
        assert!(!cell.borrow().contains(completer));
        let asked = host.asked.get();
        cell.elapse(Duration::from_secs(1));
        assert_eq!(
            host.asked.get(),
            asked,
            "no timer accesses the disposed playback"
        );
    }

    #[test]
    fn rebuilding_static_images_reuses_the_decode_and_disposes_replaced_streams() {
        let host = Rc::new(TestHost::animation());
        let cell = AppCell::with_platform(host.clone() as PlatformRef);
        static DATA: [u8; 4] = *b"same";
        let harness = mount_image(&cell, Image::memory_static(&DATA).into_widget());
        cell.checkpoint();
        for _ in 0..8 {
            let mut app = cell.borrow_mut();
            harness.set_child(&mut app, Image::memory_static(&DATA).into_widget());
            harness.pump(&mut app);
            finish_frame(&mut app);
            drop(app);
            cell.checkpoint();
        }
        assert_eq!(*host.opens.borrow(), 1);
        {
            let mut app = cell.borrow_mut();
            harness.set_child(
                &mut app,
                crate::widgets::basic::SizedBox::shrink().into_widget(),
            );
            harness.pump(&mut app);
            let cache = inset_painting::ImageCache::instance(&mut app);
            assert_eq!(cache.live_image_count(&app), 0);
            cache.clear(&mut app);
            finish_frame(&mut app);
        }
        cell.checkpoint();
        assert_eq!(host.dropped.get(), 1);
    }
}
