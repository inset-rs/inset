//! Flutter counterpart: `painting/image_cache.dart`.
//!
//! Pending requests share decoding, live entries track listener lifetime, and the LRU holds
//! completed images within its budget. Live and LRU membership can overlap.

use std::any::{Any, TypeId};
use std::cell::{Cell, OnceCell};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use indexmap::IndexMap;
use reveal_foundation::{App, Handle, Listener};
use reveal_scheduler::{FrameCallback, SchedulerBinding};

use crate::image_stream::{
    ImageStreamCompleter, ImageStreamCompleterHandle, ImageStreamListener, WeakImageStreamListener,
};

/// What identifies an image in the cache. Flutter's answer from `ImageProvider.obtainKey`.
///
/// A provider makes one out of a value of its own — an asset's name and scale, the buffer bytes
/// live in — and two keys match when their concrete types and values match.
/// Anything that changes the decoded pixels belongs in that value, the scale included.
/// Clones compare identical even for a non-reflexive value such as a NaN scale, matching Dart's
/// map identity check; distinct keys then use their values' equality and compatible hashes.
#[derive(Clone)]
pub struct ImageCacheKey(Rc<dyn KeyValue>);

impl ImageCacheKey {
    /// Creates a key out of whatever a provider uses to tell its images apart.
    ///
    /// Values must have stable, symmetric, transitive equality and equal values must hash alike.
    /// Identity makes this wrapper reflexive even when a value contains NaN.
    pub fn new<K: PartialEq + Hash + Debug + 'static>(value: K) -> ImageCacheKey {
        ImageCacheKey(Rc::new(value))
    }

    /// Reads the provider's resolved key when loading. A different key type returns `None`.
    pub fn downcast_ref<K: 'static>(&self) -> Option<&K> {
        self.0.as_any().downcast_ref()
    }
}

impl PartialEq for ImageCacheKey {
    fn eq(&self, other: &ImageCacheKey) -> bool {
        Rc::ptr_eq(&self.0, &other.0) || self.0.equals(other.0.as_ref())
    }
}

impl Eq for ImageCacheKey {}

impl Hash for ImageCacheKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash_value(state);
    }
}

impl Debug for ImageCacheKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// One provider's key value, compared and hashed without its type being known here.
///
/// Two values of different types never match, so two kinds of provider naming the same thing —
/// a file and an asset that share a path — do not compare equal.
trait KeyValue: Debug {
    fn as_any(&self) -> &dyn Any;
    fn equals(&self, other: &dyn KeyValue) -> bool;
    fn hash_value(&self, state: &mut dyn Hasher);
}

impl<K: PartialEq + Hash + Debug + 'static> KeyValue for K {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn equals(&self, other: &dyn KeyValue) -> bool {
        other.as_any().downcast_ref::<K>() == Some(self)
    }

    fn hash_value(&self, mut state: &mut dyn Hasher) {
        TypeId::of::<K>().hash(&mut state);
        self.hash(&mut state);
    }
}

/// An image held without subscribing to animation frames.
struct CachedImage {
    completer: Handle<ImageStreamCompleter>,
    size_bytes: Option<usize>,
    handle: ImageStreamCompleterHandle,
}

impl CachedImage {
    fn new(
        app: &mut App,
        completer: Handle<ImageStreamCompleter>,
        size_bytes: Option<usize>,
    ) -> Self {
        Self {
            completer,
            size_bytes,
            handle: completer.keep_alive(app),
        }
    }

    /// Flutter gives resolving consumers the rest of the frame to attach a listener.
    fn dispose(self, app: &mut App) {
        let handle = Cell::new(Some(self.handle));
        SchedulerBinding::add_post_frame_callback(
            app,
            FrameCallback::new(move |app, _| {
                if let Some(handle) = handle.take() {
                    handle.dispose(app);
                }
            }),
        );
    }
}

/// A live image leaves this index when its last listener is removed.
struct LiveImage {
    image: CachedImage,
    remove_callback: Listener,
    active: Rc<Cell<bool>>,
}

impl LiveImage {
    fn dispose(self, app: &mut App) {
        self.active.set(false);
        self.image
            .completer
            .remove_on_last_listener_removed_callback(app, &self.remove_callback);
        self.image.dispose(app);
    }
}

/// One first-frame subscription. Removing the listener also invalidates callbacks already copied
/// for dispatch, so an evicted request cannot remove a replacement with the same key.
struct PendingImage {
    completer: Handle<ImageStreamCompleter>,
    listener: ImageStreamListener,
    active: Rc<Cell<bool>>,
}

impl PendingImage {
    fn remove_listener(self, app: &mut App) {
        self.active.set(false);
        self.completer.remove_listener(app, &self.listener);
    }
}

/// Tracks pending, retained, and live image streams. Live images may also occupy the LRU.
pub struct ImageCache {
    pending_images: IndexMap<ImageCacheKey, PendingImage>,
    cache: IndexMap<ImageCacheKey, CachedImage>,
    live_images: IndexMap<ImageCacheKey, LiveImage>,
    maximum_size: usize,
    maximum_size_bytes: usize,
    current_size_bytes: usize,
}

impl Default for ImageCache {
    fn default() -> Self {
        Self {
            pending_images: IndexMap::new(),
            cache: IndexMap::new(),
            live_images: IndexMap::new(),
            maximum_size: 1000,
            maximum_size_bytes: 100 << 20,
            current_size_bytes: 0,
        }
    }
}

impl ImageCache {
    /// The application's image cache, created on first use.
    pub fn instance(app: &mut App) -> Handle<Self> {
        app.singleton()
    }

    /// Shares pending or retained work, starting `load` only when no entry tracks this key.
    pub fn put_if_absent(
        self: Handle<Self>,
        app: &mut App,
        key: ImageCacheKey,
        load: impl FnOnce(&mut App) -> Handle<ImageStreamCompleter>,
    ) -> Handle<ImageStreamCompleter> {
        if let Some(image) = app.get(self).pending_images.get(&key) {
            return image.completer;
        }
        if let Some(image) = app.get_mut(self).cache.shift_remove(&key) {
            let completer = image.completer;
            self.track_live_image(app, key.clone(), completer, image.size_bytes);
            app.get_mut(self).cache.insert(key, image);
            return completer;
        }
        if let Some(image) = app.get(self).live_images.get(&key) {
            let completer = image.image.completer;
            let size = image.image.size_bytes;
            let image = CachedImage::new(app, completer, size);
            self.touch(app, key, image);
            return completer;
        }
        let completer = load(app);
        self.track_live_image(app, key.clone(), completer, None);
        self.watch_for_completion(app, key, completer);
        completer
    }

    fn track_live_image(
        self: Handle<Self>,
        app: &mut App,
        key: ImageCacheKey,
        completer: Handle<ImageStreamCompleter>,
        size_bytes: Option<usize>,
    ) {
        if let Some(live) = app.get_mut(self).live_images.get_mut(&key) {
            if live.image.size_bytes.is_none() {
                live.image.size_bytes = size_bytes;
            }
            return;
        }
        let callback_key = key.clone();
        let active = Rc::new(Cell::new(true));
        let callback_active = active.clone();
        let remove_callback = Listener::new(move |app| {
            if !callback_active.get() {
                return;
            }
            let live = app.get_mut(self).live_images.shift_remove(&callback_key);
            if let Some(live) = live {
                live.dispose(app);
            }
        });
        completer.add_on_last_listener_removed_callback(app, remove_callback.clone());
        let image = CachedImage::new(app, completer, size_bytes);
        app.get_mut(self).live_images.insert(
            key,
            LiveImage {
                image,
                remove_callback,
                active,
            },
        );
    }

    fn watch_for_completion(
        self: Handle<Self>,
        app: &mut App,
        key: ImageCacheKey,
        completer: Handle<ImageStreamCompleter>,
    ) {
        let track_pending = app.get(self).maximum_size > 0 && app.get(self).maximum_size_bytes > 0;
        let slot = Rc::new(OnceCell::<WeakImageStreamListener>::new());
        let image_slot = slot.clone();
        let error_slot = slot.clone();
        let active = Rc::new(Cell::new(true));
        let image_active = active.clone();
        let error_active = active.clone();
        let image_key = key.clone();
        let error_key = key.clone();
        let listener = ImageStreamListener::new(Rc::new(move |app, info, _| {
            if !image_active.replace(false) {
                return;
            }
            let listener = image_slot.get().unwrap().upgrade().unwrap();
            let size = Some(info.size_bytes());
            let image = CachedImage::new(app, completer, size);
            self.track_live_image(app, image_key.clone(), completer, size);
            if track_pending {
                self.touch(app, image_key.clone(), image);
            } else {
                image.dispose(app);
            }
            app.get_mut(self).pending_images.shift_remove(&image_key);
            completer.remove_listener(app, &listener);
        }))
        .on_error(Rc::new(move |app, _| {
            if !error_active.replace(false) {
                return;
            }
            let listener = error_slot.get().unwrap().upgrade().unwrap();
            app.get_mut(self).pending_images.shift_remove(&error_key);
            let live = app.get_mut(self).live_images.shift_remove(&error_key);
            if let Some(live) = live {
                live.dispose(app);
            }
            completer.remove_listener(app, &listener);
        }));
        // Weak callback identities avoid a listener retaining itself through this slot.
        let _ = slot.set(listener.downgrade());
        if track_pending {
            app.get_mut(self).pending_images.insert(
                key,
                PendingImage {
                    completer,
                    listener: listener.clone(),
                    active,
                },
            );
        }
        completer.add_listener(app, listener);
    }

    fn touch(self: Handle<Self>, app: &mut App, key: ImageCacheKey, image: CachedImage) {
        if let Some(bytes) = image.size_bytes
            && bytes <= app.get(self).maximum_size_bytes
            && app.get(self).maximum_size > 0
        {
            app.get_mut(self).current_size_bytes += bytes;
            app.get_mut(self).cache.insert(key, image);
            self.check_cache_size(app);
        } else {
            image.dispose(app);
        }
    }

    fn check_cache_size(self: Handle<Self>, app: &mut App) {
        while app.get(self).cache.len() > app.get(self).maximum_size
            || app.get(self).current_size_bytes > app.get(self).maximum_size_bytes
        {
            let Some((_, image)) = app.get_mut(self).cache.shift_remove_index(0) else {
                break;
            };
            app.get_mut(self).current_size_bytes -= image.size_bytes.unwrap();
            image.dispose(app);
        }
    }

    /// Evicts pending/LRU retention and, by default, live tracking (for invalidating an image).
    pub fn evict(self: Handle<Self>, app: &mut App, key: &ImageCacheKey) -> bool {
        self.evict_with_include_live(app, key, true)
    }

    /// With `include_live: false`, releases cache retention while preserving reuse by listeners.
    pub fn evict_with_include_live(
        self: Handle<Self>,
        app: &mut App,
        key: &ImageCacheKey,
        include_live: bool,
    ) -> bool {
        if include_live {
            let live = app.get_mut(self).live_images.shift_remove(key);
            if let Some(live) = live {
                live.dispose(app);
            }
        }
        let pending = app.get_mut(self).pending_images.shift_remove(key);
        if let Some(pending) = pending {
            pending.remove_listener(app);
            return true;
        }
        let image = app.get_mut(self).cache.shift_remove(key);
        if let Some(image) = image {
            app.get_mut(self).current_size_bytes -= image.size_bytes.unwrap();
            image.dispose(app);
            return true;
        }
        false
    }

    /// Releases pending listeners and LRU retention; live tracking is preserved.
    pub fn clear(self: Handle<Self>, app: &mut App) {
        let cached = std::mem::take(&mut app.get_mut(self).cache);
        for image in cached.into_values() {
            image.dispose(app);
        }
        let pending = std::mem::take(&mut app.get_mut(self).pending_images);
        for image in pending.into_values() {
            image.remove_listener(app);
        }
        app.get_mut(self).current_size_bytes = 0;
    }

    /// Clears live tracking without detaching consumers' listeners.
    pub fn clear_live_images(self: Handle<Self>, app: &mut App) {
        let live = std::mem::take(&mut app.get_mut(self).live_images);
        for image in live.into_values() {
            image.dispose(app);
        }
    }

    /// Reports independent pending, LRU, and live membership.
    pub fn status_for_key(self: Handle<Self>, app: &App, key: &ImageCacheKey) -> ImageCacheStatus {
        ImageCacheStatus {
            pending: app.get(self).pending_images.contains_key(key),
            keep_alive: app.get(self).cache.contains_key(key),
            live: app.get(self).live_images.contains_key(key),
        }
    }

    /// Whether the key has pending or LRU retention; live-only entries are excluded.
    pub fn contains_key(self: Handle<Self>, app: &App, key: &ImageCacheKey) -> bool {
        let status = self.status_for_key(app, key);
        status.pending || status.keep_alive
    }

    /// The number of images retained in the LRU.
    pub fn current_size(self: Handle<Self>, app: &App) -> usize {
        app.get(self).cache.len()
    }

    /// The decoded bytes charged to LRU entries, excluding live-only images.
    pub fn current_size_bytes(self: Handle<Self>, app: &App) -> usize {
        app.get(self).current_size_bytes
    }

    /// The number of tracked first-frame requests.
    pub fn pending_image_count(self: Handle<Self>, app: &App) -> usize {
        app.get(self).pending_images.len()
    }

    /// The number of tracked first-frame requests. Use `pending_image_count` for Flutter's name.
    pub fn live_decode_count(self: Handle<Self>, app: &App) -> usize {
        self.pending_image_count(app)
    }

    /// The number of entries tracked by listener lifetime.
    pub fn live_image_count(self: Handle<Self>, app: &App) -> usize {
        app.get(self).live_images.len()
    }

    /// The configured LRU entry limit.
    pub fn maximum_size(self: Handle<Self>, app: &App) -> usize {
        app.get(self).maximum_size
    }

    /// The configured LRU byte limit.
    pub fn maximum_size_bytes(self: Handle<Self>, app: &App) -> usize {
        app.get(self).maximum_size_bytes
    }

    /// Changes the LRU entry budget, clearing pending retention when disabled.
    pub fn set_maximum_size(self: Handle<Self>, app: &mut App, maximum: usize) {
        if app.get(self).maximum_size == maximum {
            return;
        }
        app.get_mut(self).maximum_size = maximum;
        if maximum == 0 {
            self.clear(app);
        } else {
            self.check_cache_size(app);
        }
    }

    /// Changes the LRU byte budget, clearing pending retention when disabled.
    pub fn set_maximum_size_bytes(self: Handle<Self>, app: &mut App, maximum: usize) {
        if app.get(self).maximum_size_bytes == maximum {
            return;
        }
        app.get_mut(self).maximum_size_bytes = maximum;
        if maximum == 0 {
            self.clear(app);
        } else {
            self.check_cache_size(app);
        }
    }
}

/// An image may be live while also pending or retained in the LRU.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImageCacheStatus {
    /// A first-frame request is tracked.
    pub pending: bool,
    /// An LRU entry holds this image within the cache budget.
    pub keep_alive: bool,
    /// The cache tracks this completer until its listeners reach zero.
    pub live: bool,
}

impl ImageCacheStatus {
    /// Whether any cache index tracks this key.
    pub fn tracked(self) -> bool {
        self.pending || self.keep_alive || self.live
    }

    /// Whether no cache index tracks this key.
    pub fn untracked(self) -> bool {
        !self.tracked()
    }
}

#[cfg(test)]
mod tests {
    use reveal_embedder::{ImageDecodeError, test_support::solid_image};
    use reveal_foundation::AppCell;

    use super::*;
    use crate::image_stream::ImageInfo;

    /// Stands in for whatever a provider uses to tell its images apart.
    #[derive(Debug, PartialEq, Eq, Hash)]
    struct Named(String);

    fn key(name: &str) -> ImageCacheKey {
        ImageCacheKey::new(Named(name.to_owned()))
    }

    fn an_image(size: [u32; 2]) -> ImageInfo {
        ImageInfo::new(solid_image(size, [0, 128, 255, 255]))
    }

    #[test]
    fn a_second_ask_while_decoding_shares_the_first_decode() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let cache = ImageCache::instance(&mut app);
        let mut loads = 0;

        let first = cache.put_if_absent(&mut app, key("photo"), |app| {
            loads += 1;
            ImageStreamCompleter::new(app)
        });
        let second = cache.put_if_absent(&mut app, key("photo"), |app| {
            loads += 1;
            ImageStreamCompleter::new(app)
        });

        assert_eq!(loads, 1, "the second ask waits on the first decode");
        assert_eq!(first, second);
        assert_eq!(cache.live_decode_count(&app), 1);
        assert_eq!(cache.current_size(&app), 0, "nothing has arrived yet");
    }

    #[test]
    fn a_finished_decode_becomes_a_held_image_with_its_cost() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let cache = ImageCache::instance(&mut app);

        let completer = cache.put_if_absent(&mut app, key("photo"), ImageStreamCompleter::new);
        completer.set_image(&mut app, an_image([10, 4]));

        assert_eq!(cache.live_decode_count(&app), 0);
        assert_eq!(cache.current_size(&app), 1);
        assert_eq!(cache.current_size_bytes(&app), 10 * 4 * 4);

        let mut loads = 0;
        let again = cache.put_if_absent(&mut app, key("photo"), |app| {
            loads += 1;
            ImageStreamCompleter::new(app)
        });
        assert_eq!(loads, 0, "a held image is handed straight back");
        assert_eq!(again, completer);
    }

    #[test]
    fn a_failed_decode_is_not_held_so_the_next_ask_tries_again() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let cache = ImageCache::instance(&mut app);

        let completer = cache.put_if_absent(&mut app, key("photo"), ImageStreamCompleter::new);
        completer.report_error(&mut app, ImageDecodeError::UnknownFormat);

        assert_eq!(cache.live_decode_count(&app), 0);
        assert_eq!(cache.current_size(&app), 0);

        let mut loads = 0;
        cache.put_if_absent(&mut app, key("photo"), |app| {
            loads += 1;
            ImageStreamCompleter::new(app)
        });
        assert_eq!(loads, 1, "the failure was not kept");
    }

    #[test]
    fn the_least_recently_wanted_image_is_dropped_first() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let cache = ImageCache::instance(&mut app);
        for name in ["one", "two", "three"] {
            let completer = cache.put_if_absent(&mut app, key(name), ImageStreamCompleter::new);
            completer.set_image(&mut app, an_image([2, 2]));
        }
        // Asking again makes "one" the most recently wanted, so "two" is now the oldest.
        cache.put_if_absent(&mut app, key("one"), ImageStreamCompleter::new);

        cache.set_maximum_size(&mut app, 2);

        assert_eq!(cache.current_size(&app), 2);
        let mut loads = 0;
        cache.put_if_absent(&mut app, key("two"), |app| {
            loads += 1;
            ImageStreamCompleter::new(app)
        });
        assert_eq!(loads, 1, "two was the one dropped");
    }

    #[test]
    fn a_byte_budget_drops_images_and_the_count_follows_them() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let cache = ImageCache::instance(&mut app);
        for name in ["one", "two"] {
            let completer = cache.put_if_absent(&mut app, key(name), ImageStreamCompleter::new);
            completer.set_image(&mut app, an_image([8, 8]));
        }
        assert_eq!(cache.current_size_bytes(&app), 2 * 8 * 8 * 4);

        cache.set_maximum_size_bytes(&mut app, 8 * 8 * 4);

        assert_eq!(cache.current_size(&app), 1);
        assert_eq!(cache.current_size_bytes(&app), 8 * 8 * 4);
    }

    #[test]
    fn evicting_forgets_an_image_and_its_cost() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let cache = ImageCache::instance(&mut app);
        let completer = cache.put_if_absent(&mut app, key("photo"), ImageStreamCompleter::new);
        completer.set_image(&mut app, an_image([4, 4]));

        assert!(cache.evict(&mut app, &key("photo")));
        assert_eq!(cache.current_size(&app), 0);
        assert_eq!(cache.current_size_bytes(&app), 0);
        assert!(!cache.evict(&mut app, &key("photo")), "gone already");
    }

    #[test]
    fn two_providers_naming_the_same_thing_are_different_images() {
        #[derive(Debug, PartialEq, Eq, Hash)]
        struct OtherKind(String);
        assert_ne!(
            key("logo.png"),
            ImageCacheKey::new(OtherKind("logo.png".to_owned()))
        );
    }

    #[test]
    fn two_keys_of_one_kind_match_when_what_they_name_matches() {
        assert_eq!(key("logo.png"), key("logo.png"));
        assert_ne!(key("logo.png"), key("badge.png"));
    }
    fn finish_frame(app: &mut App) {
        SchedulerBinding::handle_begin_frame(app, Some(std::time::Duration::ZERO));
        app.drain_microtasks();
        SchedulerBinding::handle_draw_frame(app);
        app.drain_microtasks();
    }

    #[test]
    fn a_live_image_survives_lru_eviction_and_disposes_after_its_last_listener() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let cache = ImageCache::instance(&mut app);
        let completer = cache.put_if_absent(&mut app, key("live"), ImageStreamCompleter::new);
        let retained = app.retain(completer);
        let listener = ImageStreamListener::new(Rc::new(|_, _, _| {}));
        completer.add_listener(&mut app, listener.clone());
        completer.set_image(&mut app, an_image([2, 2]));
        cache.set_maximum_size(&mut app, 0);
        finish_frame(&mut app);
        assert_eq!(cache.current_size(&app), 0);
        assert_eq!(cache.live_image_count(&app), 1);
        assert_eq!(
            cache.put_if_absent(&mut app, key("live"), |_| panic!("must reuse live image")),
            completer
        );
        finish_frame(&mut app);
        assert!(!app.is_disposed(completer));
        completer.remove_listener(&mut app, &listener);
        assert_eq!(cache.live_image_count(&app), 0);
        finish_frame(&mut app);
        assert!(app.is_disposed(completer));
        assert!(completer.current_image(&app).is_none());
        drop(app);
        cell.checkpoint();
        let mut app = cell.borrow_mut();
        app.release(retained);
        assert!(!app.contains(completer));
    }

    #[test]
    fn cache_retention_does_not_subscribe_to_animation_frames() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let cache = ImageCache::instance(&mut app);
        let completer = cache.put_if_absent(&mut app, key("held"), ImageStreamCompleter::new);
        completer.set_image(&mut app, an_image([2, 2]));
        finish_frame(&mut app);
        assert_eq!(completer.listener_count(&app), 0);
        assert!(!app.is_disposed(completer));
        assert!(cache.status_for_key(&app, &key("held")).keep_alive);
        assert!(!cache.status_for_key(&app, &key("held")).live);
        assert_eq!(
            cache.put_if_absent(&mut app, key("held"), |_| panic!("must reuse cached image")),
            completer
        );
        let listener = ImageStreamListener::new(Rc::new(|_, _, _| {}));
        completer.add_listener(&mut app, listener.clone());
        assert!(cache.status_for_key(&app, &key("held")).live);
        completer.remove_listener(&mut app, &listener);
        cache.clear(&mut app);
        finish_frame(&mut app);
        assert!(app.is_disposed(completer));
    }

    #[test]
    fn an_oversized_image_does_not_flush_smaller_entries() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let cache = ImageCache::instance(&mut app);
        cache.set_maximum_size_bytes(&mut app, 16);
        let small = cache.put_if_absent(&mut app, key("small"), ImageStreamCompleter::new);
        small.set_image(&mut app, an_image([2, 2]));
        let large = cache.put_if_absent(&mut app, key("large"), ImageStreamCompleter::new);
        large.set_image(&mut app, an_image([4, 4]));
        assert_eq!(cache.current_size_bytes(&app), 16);
        assert!(cache.contains_key(&app, &key("small")));
        assert!(!cache.contains_key(&app, &key("large")));
        finish_frame(&mut app);
        assert!(app.is_disposed(large));
    }

    #[test]
    fn evicting_pending_work_removes_its_listener_and_cannot_remove_its_replacement() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let cache = ImageCache::instance(&mut app);
        let old = cache.put_if_absent(&mut app, key("same"), ImageStreamCompleter::new);
        cache.evict(&mut app, &key("same"));
        assert_eq!(old.listener_count(&app), 0);
        let new = cache.put_if_absent(&mut app, key("same"), ImageStreamCompleter::new);
        // A completion may already be in flight when eviction happens.
        old.report_error(&mut app, ImageDecodeError::UnknownFormat);
        assert_eq!(cache.pending_image_count(&app), 1);
        new.set_image(&mut app, an_image([2, 2]));
        assert_eq!(
            cache.put_if_absent(&mut app, key("same"), |_| panic!(
                "new decode remains cached"
            )),
            new
        );
    }

    #[test]
    fn a_pending_listener_in_a_dispatch_snapshot_cannot_overwrite_a_new_request() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let cache = ImageCache::instance(&mut app);
        let old = ImageStreamCompleter::new(&mut app);
        let replacement = Rc::new(Cell::new(None));
        let result = replacement.clone();
        old.add_listener(
            &mut app,
            ImageStreamListener::new(Rc::new(move |app, _, _| {
                cache.evict(app, &key("same"));
                let new = cache.put_if_absent(app, key("same"), ImageStreamCompleter::new);
                result.set(Some(new));
            })),
        );
        cache.put_if_absent(&mut app, key("same"), |_| old);
        old.set_image(&mut app, an_image([2, 2]));
        assert_eq!(cache.pending_image_count(&app), 1);
        assert_eq!(
            cache.put_if_absent(&mut app, key("same"), |_| panic!("pending replacement")),
            replacement.get().unwrap()
        );
        assert_eq!(cache.current_size(&app), 0);
    }

    #[test]
    fn clearing_pending_work_detaches_subscriptions_even_without_a_result() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let cache = ImageCache::instance(&mut app);
        let completer = cache.put_if_absent(&mut app, key("pending"), ImageStreamCompleter::new);
        cache.clear(&mut app);
        assert_eq!(completer.listener_count(&app), 0);
        assert_eq!(cache.pending_image_count(&app), 0);
        assert_eq!(cache.live_image_count(&app), 0);
        finish_frame(&mut app);
        assert!(app.is_disposed(completer));
    }

    #[test]
    fn pending_callbacks_do_not_retain_their_own_key_after_the_app_drops() {
        #[derive(Debug)]
        struct Probe(Rc<()>);
        impl PartialEq for Probe {
            fn eq(&self, other: &Self) -> bool {
                Rc::ptr_eq(&self.0, &other.0)
            }
        }
        impl Hash for Probe {
            fn hash<H: Hasher>(&self, state: &mut H) {
                Rc::as_ptr(&self.0).hash(state);
            }
        }
        let identity = Rc::new(());
        let weak = Rc::downgrade(&identity);
        let cell = AppCell::new();
        {
            let mut app = cell.borrow_mut();
            let cache = ImageCache::instance(&mut app);
            cache.set_maximum_size(&mut app, 0);
            cache.put_if_absent(
                &mut app,
                ImageCacheKey::new(Probe(identity)),
                ImageStreamCompleter::new,
            );
        }
        drop(cell);
        assert!(weak.upgrade().is_none());
    }
    #[test]
    fn an_unregistered_live_callback_cannot_remove_a_replacement_during_dispatch() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let cache = ImageCache::instance(&mut app);
        let old = ImageStreamCompleter::new(&mut app);
        old.add_on_last_listener_removed_callback(
            &mut app,
            Listener::new(move |app| {
                cache.evict(app, &key("same"));
                cache.put_if_absent(app, key("same"), ImageStreamCompleter::new);
            }),
        );
        let listener = ImageStreamListener::new(Rc::new(|_, _, _| {}));
        old.add_listener(&mut app, listener.clone());
        cache.put_if_absent(&mut app, key("same"), |_| old);
        old.set_image(&mut app, an_image([2, 2]));
        old.remove_listener(&mut app, &listener);
        assert!(cache.status_for_key(&app, &key("same")).live);
        assert!(cache.status_for_key(&app, &key("same")).pending);
    }
}
