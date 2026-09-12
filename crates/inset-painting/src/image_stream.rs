//! Flutter counterpart: `painting/image_stream.dart`.
//!
//! An image arrives later than the widget that wants it, so a widget holds a stream and is
//! called back when a frame is ready. The stream is the listening half and the completer is
//! the resolving half, exactly as in Dart; a provider hands out the completer and the cache
//! keeps it, so several widgets showing the same image share one decode.
//!
//! Cache retention and listener lifetime follow Flutter; animation playback is composed with
//! the completer and is disposed through its cleanup hook.

use std::fmt::{self, Debug};
use std::rc::Rc;

use inset_embedder::{Image, ImageDecodeError};
use inset_foundation::{App, Handle, Listener, RetainedHandle};

/// A decoded image and how to read it. Flutter `ImageInfo`.
///
/// The scale is what turns pixels into logical points: an image drawn from a two-times asset
/// carries a scale of two and paints at half its pixel size, which is what makes the same asset
/// look right on displays of different densities.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageInfo {
    /// The decoded image itself.
    pub image: Image,
    /// Linear pixels per logical point; one means the image draws at its pixel size.
    pub scale: f64,
    /// A name for error messages, such as the asset the image came from.
    pub debug_label: Option<String>,
}

impl ImageInfo {
    /// Creates an [`ImageInfo`] at the natural scale.
    pub fn new(image: Image) -> ImageInfo {
        ImageInfo {
            image,
            scale: 1.0,
            debug_label: None,
        }
    }

    /// Dart `ImageInfo(scale:)`.
    pub fn scale(mut self, scale: f64) -> ImageInfo {
        self.scale = scale;
        self
    }

    /// Dart `ImageInfo(debugLabel:)`.
    pub fn debug_label(mut self, label: impl Into<String>) -> ImageInfo {
        self.debug_label = Some(label.into());
        self
    }

    /// How much memory the decoded image occupies, which is what the cache budgets by.
    pub fn size_bytes(&self) -> usize {
        let [width, height] = self.image.size();
        width as usize * height as usize * 4
    }

    /// The image's size in logical points, which is its pixel size divided by its scale.
    pub fn size(&self) -> inset_embedder::Size {
        let [width, height] = self.image.size();
        inset_embedder::Size::new(
            f64::from(width) / self.scale,
            f64::from(height) / self.scale,
        )
    }
}

/// Called when a frame is ready. `synchronous` is true when the image was already there and the
/// call happened inside `add_listener` rather than later.
pub type ImageListener = Rc<ImageCallback>;

type ImageCallback = dyn Fn(&mut App, &ImageInfo, bool);

/// Called when an image will never arrive.
pub type ImageErrorListener = Rc<ImageErrorCallback>;

type ImageErrorCallback = dyn Fn(&mut App, &ImageDecodeError);

/// What a caller wants to hear about an image. Flutter `ImageStreamListener`.
///
/// Without an error listener a failure is silent here, where Dart's would reach
/// `FlutterError.onError`; a caller that wants to show something in place of a broken image
/// supplies one.
#[derive(Clone)]
pub struct ImageStreamListener {
    /// Called for each frame, including one that had already arrived.
    pub on_image: ImageListener,
    /// Called instead when the image failed.
    pub on_error: Option<ImageErrorListener>,
}

/// Callback identities without owning them, for a subscription that removes itself.
pub(crate) struct WeakImageStreamListener {
    on_image: std::rc::Weak<ImageCallback>,
    on_error: Option<std::rc::Weak<ImageErrorCallback>>,
}

impl WeakImageStreamListener {
    pub(crate) fn upgrade(&self) -> Option<ImageStreamListener> {
        Some(ImageStreamListener {
            on_image: self.on_image.upgrade()?,
            on_error: match &self.on_error {
                Some(callback) => Some(callback.upgrade()?),
                None => None,
            },
        })
    }
}

impl ImageStreamListener {
    pub(crate) fn downgrade(&self) -> WeakImageStreamListener {
        WeakImageStreamListener {
            on_image: Rc::downgrade(&self.on_image),
            on_error: self.on_error.as_ref().map(Rc::downgrade),
        }
    }

    /// Creates a listener that only wants the image.
    pub fn new(on_image: ImageListener) -> ImageStreamListener {
        ImageStreamListener {
            on_image,
            on_error: None,
        }
    }

    /// Dart `ImageStreamListener(onError:)`.
    pub fn on_error(mut self, on_error: ImageErrorListener) -> ImageStreamListener {
        self.on_error = Some(on_error);
        self
    }

    /// Whether both halves are the same callbacks as `other`'s, which is how a listener is
    /// found again to remove it.
    fn is(&self, other: &ImageStreamListener) -> bool {
        Rc::ptr_eq(&self.on_image, &other.on_image)
            && match (&self.on_error, &other.on_error) {
                (None, None) => true,
                (Some(left), Some(right)) => Rc::ptr_eq(left, right),
                _ => false,
            }
    }
}

impl Debug for ImageStreamListener {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ImageStreamListener")
            .field("on_error", &self.on_error.is_some())
            .finish()
    }
}

/// Keeps a completer usable without listening to animation frames.
///
/// Call `dispose` exactly once to release logical retention. Arena retention separately keeps
/// the receiver allocated while disposal invokes callbacks, as a Dart reference would.
#[must_use = "dispose the keep-alive handle when its owner releases the completer"]
pub struct ImageStreamCompleterHandle {
    completer: RetainedHandle<Handle<ImageStreamCompleter>>,
}

impl ImageStreamCompleterHandle {
    /// Releases this keep-alive and disposes the completer if it has no listeners or other holds.
    pub fn dispose(self, app: &mut App) {
        let completer = self.completer.get();
        debug_assert!(!app.is_disposed(completer));
        debug_assert!(app.get(completer).keep_alive_handles > 0);
        app.get_mut(completer).keep_alive_handles -= 1;
        completer.maybe_dispose(app);
        app.release(self.completer);
    }
}

/// The resolving half of an image: holds the frame once it exists and tells the listeners.
///
/// Flutter `ImageStreamCompleter`. A listener added after the image has arrived is called
/// straight away, which is what lets a widget built later show a cached image in its first
/// frame instead of flashing empty.
#[derive(Default)]
pub struct ImageStreamCompleter {
    listeners: Vec<ImageStreamListener>,
    current_image: Option<ImageInfo>,
    current_error: Option<ImageDecodeError>,
    on_watched: Option<WatchedCallback>,
    keep_alive_handles: usize,
    on_last_listener_removed: Vec<Listener>,
    on_disposed: Option<Listener>,
}

/// Told whether anyone is listening now, when that answer changes.
pub type WatchedCallback = Rc<dyn Fn(&mut App, bool)>;

impl ImageStreamCompleter {
    /// Creates a completer with no image yet.
    pub fn new(app: &mut App) -> Handle<ImageStreamCompleter> {
        app.create(ImageStreamCompleter::default())
    }

    /// Holds this completer without subscribing to its frames.
    pub fn keep_alive(self: Handle<Self>, app: &mut App) -> ImageStreamCompleterHandle {
        debug_assert!(!app.is_disposed(self));
        app.get_mut(self).keep_alive_handles += 1;
        ImageStreamCompleterHandle {
            completer: app.retain(self),
        }
    }

    /// Registers cleanup when the last frame listener leaves. Callbacks are cleared afterward.
    pub fn add_on_last_listener_removed_callback(
        self: Handle<Self>,
        app: &mut App,
        callback: Listener,
    ) {
        debug_assert!(!app.is_disposed(self));
        app.get_mut(self).on_last_listener_removed.push(callback);
    }

    /// Unregisters a previously installed last-listener callback.
    pub fn remove_on_last_listener_removed_callback(
        self: Handle<Self>,
        app: &mut App,
        callback: &Listener,
    ) {
        let callbacks = &mut app.get_mut(self).on_last_listener_removed;
        if let Some(index) = callbacks.iter().position(|held| held == callback) {
            callbacks.remove(index);
        }
    }

    /// Installs the composed playback's resource cleanup.
    pub(crate) fn set_on_disposed(self: Handle<Self>, app: &mut App, callback: Listener) {
        debug_assert!(app.get(self).on_disposed.is_none());
        app.get_mut(self).on_disposed = Some(callback);
    }

    /// Releases resources once neither listeners nor keep-alive handles need this completer.
    pub fn maybe_dispose(self: Handle<Self>, app: &mut App) {
        if app.is_disposed(self)
            || !app.get(self).listeners.is_empty()
            || app.get(self).keep_alive_handles != 0
        {
            return;
        }
        let retained = app.retain(self);
        let data = app.get_mut(self);
        data.current_image = None;
        data.current_error = None;
        data.on_watched = None;
        data.on_last_listener_removed.clear();
        let cleanup = data.on_disposed.take();
        app.destroy(self);
        if let Some(cleanup) = cleanup {
            cleanup.call(app);
        }
        app.release(retained);
    }

    /// The frame this completer holds, if one has arrived.
    pub fn current_image(self: Handle<Self>, app: &App) -> Option<ImageInfo> {
        app.get(self).current_image.clone()
    }

    /// Why this image will never arrive, if it failed.
    pub fn current_error(self: Handle<Self>, app: &App) -> Option<ImageDecodeError> {
        app.get(self).current_error.clone()
    }

    /// How many listeners are waiting, which is what tells the cache whether anyone still wants
    /// this image.
    pub fn listener_count(self: Handle<Self>, app: &App) -> usize {
        app.get(self).listeners.len()
    }

    /// Watches whether anyone is listening, so work nobody is waiting for can be stopped.
    ///
    /// Called with true when the first listener arrives and false when the last one leaves. An
    /// animation uses this to stop between frames while it is off screen and pick up again when
    /// it comes back; Dart's completer overrides its listener methods to the same end.
    pub fn set_on_watched(self: Handle<Self>, app: &mut App, on_watched: WatchedCallback) {
        let watched = !app.get(self).listeners.is_empty();
        app.get_mut(self).on_watched = Some(Rc::clone(&on_watched));
        if watched {
            on_watched(app, true);
        }
    }

    /// Adds a listener, calling it at once if the image or the failure is already here.
    pub fn add_listener(self: Handle<Self>, app: &mut App, listener: ImageStreamListener) {
        debug_assert!(!app.is_disposed(self));
        let _retained = app.retain(self);
        let was_watched = !app.get(self).listeners.is_empty();
        app.get_mut(self).listeners.push(listener.clone());
        if !was_watched {
            self.report_watched(app, true);
        }
        if let Some(image) = app.get(self).current_image.clone() {
            (listener.on_image)(app, &image, true);
            return;
        }
        if let Some(error) = app.get(self).current_error.clone()
            && let Some(on_error) = &listener.on_error
        {
            on_error(app, &error);
        }
    }

    /// Removes a listener added with [`add_listener`](Self::add_listener).
    pub fn remove_listener(self: Handle<Self>, app: &mut App, listener: &ImageStreamListener) {
        debug_assert!(!app.is_disposed(self));
        let _retained = app.retain(self);
        let listeners = &mut app.get_mut(self).listeners;
        if let Some(index) = listeners.iter().position(|held| held.is(listener)) {
            listeners.remove(index);
        }
        if listeners.is_empty() {
            for callback in app.get(self).on_last_listener_removed.clone() {
                callback.call(app);
            }
            app.get_mut(self).on_last_listener_removed.clear();
            self.report_watched(app, false);
            self.maybe_dispose(app);
        }
    }

    fn report_watched(self: Handle<Self>, app: &mut App, watched: bool) {
        let Some(on_watched) = app.get(self).on_watched.clone() else {
            return;
        };
        on_watched(app, watched);
    }

    /// Hands a frame to every listener and keeps it for listeners still to come.
    pub fn set_image(self: Handle<Self>, app: &mut App, image: ImageInfo) {
        debug_assert!(!app.is_disposed(self));
        let _retained = app.retain(self);
        app.get_mut(self).current_image = Some(image.clone());
        app.get_mut(self).current_error = None;
        // A copy, so a listener that adds or removes one while being called does not disturb
        // this pass — the same rule the change notifiers follow.
        for listener in app.get(self).listeners.clone() {
            (listener.on_image)(app, &image, false);
        }
    }

    /// Tells every listener that the image failed, and keeps the failure for later listeners.
    pub fn report_error(self: Handle<Self>, app: &mut App, error: ImageDecodeError) {
        debug_assert!(!app.is_disposed(self));
        let _retained = app.retain(self);
        app.get_mut(self).current_error = Some(error.clone());
        for listener in app.get(self).listeners.clone() {
            if let Some(on_error) = &listener.on_error {
                on_error(app, &error);
            }
        }
    }
}

/// The listening half of an image. Flutter `ImageStream`.
///
/// A stream exists before its completer does, because a provider may have to look in the cache
/// or start a decode first. Listeners added in the meantime are held here and handed over when
/// [`set_completer`](Self::set_completer) arrives.
#[derive(Default)]
pub struct ImageStream {
    completer: Option<RetainedHandle<Handle<ImageStreamCompleter>>>,
    listeners: Vec<ImageStreamListener>,
}

impl ImageStream {
    /// Creates a stream with no completer yet.
    pub fn new(app: &mut App) -> Handle<ImageStream> {
        app.create(ImageStream::default())
    }

    /// The completer, once one has been assigned.
    ///
    /// Two streams for the same image share a completer, which is how the cache makes one decode
    /// serve every widget showing it. Comparing these is how a widget tells whether the image it
    /// resolved to has actually changed.
    pub fn completer(self: Handle<Self>, app: &App) -> Option<Handle<ImageStreamCompleter>> {
        app.get(self).completer.as_ref().map(RetainedHandle::get)
    }

    /// Releases the stream and unregisters its listeners. The resolving consumer owns this call.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        let _retained = app.retain(self);
        let listeners = std::mem::take(&mut app.get_mut(self).listeners);
        if let Some(completer) = app.get_mut(self).completer.take() {
            for listener in listeners {
                completer.remove_listener(app, &listener);
            }
            completer.maybe_dispose(app);
            app.release(completer);
        }
        app.destroy(self);
    }

    /// Assigns the completer, handing it every listener added while there was none.
    ///
    /// # Panics
    ///
    /// Panics if a completer was already assigned, as Dart's assert does; a stream resolves once.
    pub fn set_completer(
        self: Handle<Self>,
        app: &mut App,
        completer: Handle<ImageStreamCompleter>,
    ) {
        assert!(
            app.get(self).completer.is_none(),
            "an ImageStream takes its completer once"
        );
        let _retained = app.retain(self);
        let retained_completer = app.retain(completer);
        app.get_mut(self).completer = Some(retained_completer);
        for listener in app.get(self).listeners.clone() {
            completer.add_listener(app, listener);
        }
    }

    /// Adds a listener now, or holds it until the completer arrives.
    pub fn add_listener(self: Handle<Self>, app: &mut App, listener: ImageStreamListener) {
        app.get_mut(self).listeners.push(listener.clone());
        if let Some(completer) = self.completer(app) {
            completer.add_listener(app, listener);
        }
    }

    /// Removes a listener, whether it is waiting here or already on the completer.
    pub fn remove_listener(self: Handle<Self>, app: &mut App, listener: &ImageStreamListener) {
        let listeners = &mut app.get_mut(self).listeners;
        if let Some(index) = listeners.iter().position(|held| held.is(listener)) {
            listeners.remove(index);
            if let Some(completer) = self.completer(app) {
                completer.remove_listener(app, listener);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use inset_embedder::test_support::solid_image;
    use inset_foundation::AppCell;

    use super::*;

    /// A listener that records what it was told, so a test can assert on the order.
    #[derive(Default)]
    struct Record {
        images: RefCell<Vec<(f64, bool)>>,
        errors: RefCell<Vec<ImageDecodeError>>,
    }

    fn listener(record: &Rc<Record>) -> ImageStreamListener {
        let images = Rc::clone(record);
        let errors = Rc::clone(record);
        ImageStreamListener::new(Rc::new(move |_app: &mut App, info: &ImageInfo, sync| {
            images.images.borrow_mut().push((info.scale, sync));
        }))
        .on_error(Rc::new(move |_app: &mut App, error: &ImageDecodeError| {
            errors.errors.borrow_mut().push(error.clone());
        }))
    }

    fn an_image() -> ImageInfo {
        ImageInfo::new(solid_image([4, 2], [255, 0, 0, 255])).scale(2.0)
    }

    #[test]
    fn an_image_reaches_the_listeners_waiting_for_it() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let completer = ImageStreamCompleter::new(&mut app);
        let record = Rc::new(Record::default());
        completer.add_listener(&mut app, listener(&record));

        completer.set_image(&mut app, an_image());

        assert_eq!(record.images.borrow().as_slice(), &[(2.0, false)]);
    }

    #[test]
    fn a_listener_added_after_the_image_hears_about_it_at_once() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let completer = ImageStreamCompleter::new(&mut app);
        completer.set_image(&mut app, an_image());

        let record = Rc::new(Record::default());
        completer.add_listener(&mut app, listener(&record));

        // Reported as synchronous, which is what lets a widget paint in the frame it was built.
        assert_eq!(record.images.borrow().as_slice(), &[(2.0, true)]);
    }

    #[test]
    fn a_removed_listener_hears_nothing_further() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let completer = ImageStreamCompleter::new(&mut app);
        let keep_alive = completer.keep_alive(&mut app);
        let record = Rc::new(Record::default());
        let listener = listener(&record);
        completer.add_listener(&mut app, listener.clone());
        completer.remove_listener(&mut app, &listener);

        completer.set_image(&mut app, an_image());

        assert!(record.images.borrow().is_empty());
        assert_eq!(completer.listener_count(&app), 0);
        keep_alive.dispose(&mut app);
    }

    #[test]
    fn a_failure_reaches_the_error_listeners_and_the_late_ones() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let completer = ImageStreamCompleter::new(&mut app);
        let waiting = Rc::new(Record::default());
        completer.add_listener(&mut app, listener(&waiting));

        completer.report_error(&mut app, ImageDecodeError::UnknownFormat);

        let late = Rc::new(Record::default());
        completer.add_listener(&mut app, listener(&late));

        assert_eq!(
            waiting.errors.borrow().as_slice(),
            &[ImageDecodeError::UnknownFormat]
        );
        assert_eq!(
            late.errors.borrow().as_slice(),
            &[ImageDecodeError::UnknownFormat],
            "a listener added after the failure still learns of it"
        );
    }

    #[test]
    fn listeners_added_before_a_completer_are_handed_over_with_it() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let stream = ImageStream::new(&mut app);
        let record = Rc::new(Record::default());
        stream.add_listener(&mut app, listener(&record));
        assert!(stream.completer(&app).is_none());

        let completer = ImageStreamCompleter::new(&mut app);
        completer.set_image(&mut app, an_image());
        stream.set_completer(&mut app, completer);

        assert_eq!(record.images.borrow().as_slice(), &[(2.0, true)]);
        assert_eq!(completer.listener_count(&app), 1);
    }

    #[test]
    fn a_listener_removed_before_the_completer_never_reaches_it() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let stream = ImageStream::new(&mut app);
        let record = Rc::new(Record::default());
        let listener = listener(&record);
        stream.add_listener(&mut app, listener.clone());
        stream.remove_listener(&mut app, &listener);

        let completer = ImageStreamCompleter::new(&mut app);
        stream.set_completer(&mut app, completer);
        completer.set_image(&mut app, an_image());

        assert!(record.images.borrow().is_empty());
    }

    #[test]
    fn an_image_reports_its_logical_size_and_what_it_costs_to_hold() {
        let info = ImageInfo::new(solid_image([4, 2], [0, 0, 255, 255])).scale(2.0);
        assert_eq!(info.size(), inset_embedder::Size::new(2.0, 1.0));
        assert_eq!(info.size_bytes(), 4 * 2 * 4);
    }
    #[test]
    fn a_listener_can_dispose_its_stream_during_frame_delivery() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let stream = ImageStream::new(&mut app);
        let completer = ImageStreamCompleter::new(&mut app);
        stream.set_completer(&mut app, completer);
        stream.add_listener(
            &mut app,
            ImageStreamListener::new(Rc::new(move |app, _, _| {
                stream.dispose(app);
            })),
        );
        completer.set_image(&mut app, an_image());
        assert!(app.is_disposed(completer));
        assert!(completer.current_image(&app).is_none());
        drop(app);
        cell.checkpoint();
        let app = cell.borrow();
        assert!(!app.contains(stream));
        assert!(!app.contains(completer));
    }
}
