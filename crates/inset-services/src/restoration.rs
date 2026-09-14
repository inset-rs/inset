//! Flutter counterpart: `services/restoration.dart`.

use std::fmt::{self, Debug};
use std::rc::Rc;

use indexmap::{IndexMap, IndexSet};
pub use inset_embedder::{RestorationData, RestorationMap, RestorationUpdate};
use inset_foundation::{App, ChangeNotifier, ChangeNotifierData, Handle};
use inset_scheduler::{FrameCallback, SchedulerBinding};

/// Manages the restoration data in the framework and synchronizes it with the host.
///
/// Restoration data can be serialized out and - at a later point in time - be
/// used to restore the application to the previous state described by the
/// serialized data. Mobile operating systems use the concept of state
/// restoration to provide the illusion that apps continue to run in the
/// background forever: after an app has been backgrounded, the user can always
/// return to it and find it in the same state. In practice, the operating
/// system may, however, terminate the app to free resources for other apps
/// running in the foreground. Before that happens, the app gets a chance to
/// serialize out its restoration data. When the user navigates back to the
/// backgrounded app, it is restarted and the serialized restoration data is
/// provided to it again. Ideally, the app will use that data to restore itself
/// to the same state it was in when the user backgrounded the app.
///
/// Restoration data is organized in a tree of [`RestorationBucket`]s which is
/// rooted in the [`root_bucket`](Self::root_bucket). All information that the
/// application needs to restore its current state must be stored in a bucket in
/// this hierarchy. To store data in the hierarchy, entities must claim ownership
/// of a child bucket from a parent bucket (which may be the root bucket provided
/// by this [`RestorationManager`]). The owner of a bucket may store arbitrary
/// [`RestorationData`] in the bucket. The values are stored in the bucket under a
/// given restoration ID as key. A restoration ID is a string that must be unique
/// within a given bucket. To access the stored value again during state
/// restoration, the same restoration ID must be provided again. The owner of the
/// bucket may also make the bucket available to other entities so that they can
/// claim child buckets from it for their own restoration needs. Within a bucket,
/// child buckets are also identified by unique restoration IDs. The restoration
/// ID must be provided when claiming a child bucket.
///
/// When restoration data is provided to the [`RestorationManager`] (e.g. after
/// the application relaunched when foregrounded again), the bucket hierarchy
/// with all the data stored in it is restored. Entities can retrieve the data
/// again by using the same restoration IDs that they originally used to store
/// the data.
///
/// In addition to providing restoration data when the app is launched,
/// restoration data may also be provided to a running app to restore it to a
/// previous state (e.g. when the user hits the back/forward button in the web
/// browser). When this happens, the [`RestorationManager`] notifies its listeners
/// (added via [`add_listener`]) that a new root bucket is available. In response
/// to the notification, listeners must stop using the old bucket and restore
/// their state from the information in the new root bucket.
///
/// Some platforms restrict the size of the restoration data. Therefore, the
/// data stored in the buckets should be as small as possible while still
/// allowing the app to restore its current state from it. Data that can be
/// retrieved from other services (e.g. a database or a web server) should not
/// be included in the restoration data. Instead, a small identifier (e.g. a
/// UUID, database record number, or resource locator) should be stored that can
/// be used to retrieve the data again from its original source during state
/// restoration.
///
/// The [`RestorationManager`] sends a copy of the bucket hierarchy over to the
/// host at the end of a frame in which the data in the hierarchy or its shape
/// has changed. The host caches the data until the operating system needs it.
/// The application is responsible for keeping the data in the bucket always
/// up-to-date to reflect its current state.
///
/// ## Discussion
///
/// Due to the threading model of the platforms Flutter runs on, restoration data
/// must be stored in the buckets proactively as described above. When the
/// operating system asks for the restoration data, it will do so on the platform
/// thread expecting a synchronous response. To avoid the risk of deadlocks, the
/// platform thread cannot block and call into the thread running the application
/// to retrieve the restoration data. For this reason, the [`RestorationManager`]
/// always sends the latest copy of the restoration data over to the host
/// whenever it changes. That way, the restoration data is always ready to go on
/// the platform thread when the operating system needs it.
///
/// Dart's `ServicesBinding.restorationManager` is [`instance`](Self::instance),
/// the App's singleton.
///
/// See also:
///
///  * [`RestorationBucket`], which make up the restoration data hierarchy.
///
/// [`add_listener`]: inset_foundation::ListenableObject::add_listener
#[derive(Default)]
pub struct RestorationManager {
    change_notifier: ChangeNotifierData,
    // May be `None` to indicate that restoration is turned off.
    root_bucket: Option<Handle<RestorationBucket>>,
    // Dart's `Completer` for the outstanding request to the engine; the host answers
    // within the call, so only the fact that a request is in flight remains.
    pending_root_bucket: bool,
    root_bucket_is_valid: bool,
    is_replacing: bool,
    debug_doing_update: bool,
    serialization_scheduled: bool,
    buckets_needing_serialization: IndexSet<Handle<RestorationBucket>>,
}

impl ChangeNotifier for RestorationManager {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl RestorationManager {
    /// The application's restoration manager.
    ///
    /// Dart's `ServicesBinding.instance.restorationManager`.
    pub fn instance(app: &mut App) -> Handle<RestorationManager> {
        app.singleton::<RestorationManager>()
    }

    /// The root of the [`RestorationBucket`] hierarchy containing the restoration
    /// data.
    ///
    /// Child buckets can be claimed from this bucket via
    /// [`RestorationBucket::claim_child`]. If the [`RestorationManager`] has been
    /// asked to restore the application to a previous state, these buckets will
    /// contain the previously stored data. Otherwise the root bucket (and all
    /// children claimed from it) will be empty.
    ///
    /// The [`RestorationManager`] informs its listeners when the value returned by
    /// this getter changes. This happens when new restoration data has been
    /// provided to the [`RestorationManager`] to restore the application to a
    /// different state. In response to the notification, listeners must stop using
    /// the old root bucket and obtain the new one via this getter (it will have
    /// been updated to return the new bucket just before the listeners are
    /// notified).
    ///
    /// The restoration data describing the current bucket hierarchy is retrieved
    /// from the host the first time the root bucket is accessed via this getter.
    ///
    /// Returns `None` if state restoration is currently turned off.
    pub fn root_bucket(self: Handle<Self>, app: &mut App) -> Option<Handle<RestorationBucket>> {
        if app.get(self).root_bucket_is_valid {
            return app.get(self).root_bucket;
        }
        if !app.get(self).pending_root_bucket {
            app.get_mut(self).pending_root_bucket = true;
            self.get_root_bucket_from_engine(app);
        }
        app.get(self).root_bucket
    }

    /// Returns true for the frame after the root bucket has been replaced with a
    /// new non-null bucket.
    ///
    /// When true, entities should forget their current state and restore
    /// their state according to the information in the new root bucket.
    ///
    /// The [`RestorationManager`] informs its listeners when this flag changes from
    /// false to true.
    pub fn is_replacing(self: Handle<Self>, app: &App) -> bool {
        app.get(self).is_replacing
    }

    fn get_root_bucket_from_engine(self: Handle<Self>, app: &mut App) {
        let config = app
            .platform()
            .restoration()
            .and_then(|restoration| restoration.get());
        if !app.get(self).pending_root_bucket {
            // The restoration data was obtained via other means (e.g. by calling
            // `handle_restoration_update_from_engine` while the request to the host was
            // outstanding). Ignore the host's response.
            return;
        }
        debug_assert!(app.get(self).root_bucket.is_none());
        self.parse_and_handle_restoration_update_from_engine(app, config);
    }

    fn parse_and_handle_restoration_update_from_engine(
        self: Handle<Self>,
        app: &mut App,
        update: Option<RestorationUpdate>,
    ) {
        let (enabled, data) = match update {
            Some(update) => (update.enabled, update.data),
            None => (false, None),
        };
        self.handle_restoration_update_from_engine(app, enabled, data);
    }

    /// Parses the restoration information obtained from the host.
    ///
    /// The `enabled` parameter indicates whether the host wants to receive
    /// restoration data. When `enabled` is false, state restoration is turned
    /// off and the root bucket is set to `None`. When `enabled` is true, the
    /// provided restoration `data` becomes the new root bucket. If `data` is
    /// `None`, an empty root bucket is instantiated.
    ///
    /// A host may call this method at any time to inject restoration data
    /// (obtained e.g. from [`send_to_engine`](Self::send_to_engine)) into the
    /// [`RestorationManager`], which is what Flutter's `push` message on
    /// `SystemChannels.restoration` does. When the method is called before the
    /// root bucket is accessed, [`root_bucket`](Self::root_bucket) answers without
    /// asking the host.
    pub fn handle_restoration_update_from_engine(
        self: Handle<Self>,
        app: &mut App,
        enabled: bool,
        data: Option<RestorationMap>,
    ) {
        debug_assert!(enabled || data.is_none());

        let is_replacing = app.get(self).root_bucket_is_valid && enabled;
        app.get_mut(self).is_replacing = is_replacing;
        if is_replacing {
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _| app.get_mut(self).is_replacing = false),
            );
        }

        let old_root = app.get(self).root_bucket;
        let new_root = enabled.then(|| RestorationBucket::root(app, self, data));
        let this = app.get_mut(self);
        this.root_bucket = new_root;
        this.root_bucket_is_valid = true;
        this.pending_root_bucket = false;

        if new_root != old_root {
            self.notify_listeners(app);
            if let Some(old_root) = old_root {
                old_root.dispose(app);
            }
        }
    }

    /// Sends the provided restoration data to the host.
    ///
    /// The `encoded_data` describes the entire bucket hierarchy that makes up the
    /// current restoration data.
    pub fn send_to_engine(self: Handle<Self>, app: &mut App, encoded_data: RestorationMap) {
        if let Some(restoration) = app.platform().restoration() {
            restoration.put(encoded_data);
        }
    }

    /// Called by a [`RestorationBucket`] to request serialization for that bucket.
    ///
    /// This method is called by a bucket in the hierarchy whenever the data
    /// in it or the shape of the hierarchy has changed.
    ///
    /// Calling this is a no-op when the bucket is already scheduled for
    /// serialization.
    pub fn schedule_serialization_for(
        self: Handle<Self>,
        app: &mut App,
        bucket: Handle<RestorationBucket>,
    ) {
        debug_assert!(app.get(bucket).manager == Some(self));
        debug_assert!(!app.get(self).debug_doing_update);
        app.get_mut(self)
            .buckets_needing_serialization
            .insert(bucket);
        if !app.get(self).serialization_scheduled {
            app.get_mut(self).serialization_scheduled = true;
            SchedulerBinding::add_post_frame_callback(
                app,
                FrameCallback::new(move |app, _| self.do_serialization(app)),
            );
        }
    }

    /// Called by a [`RestorationBucket`] to unschedule a request for serialization.
    ///
    /// This method is called by a bucket in the hierarchy whenever it no longer
    /// needs to be serialized (e.g. because the bucket got disposed).
    ///
    /// It is safe to call this even when the bucket wasn't scheduled for
    /// serialization before.
    pub fn unschedule_serialization_for(
        self: Handle<Self>,
        app: &mut App,
        bucket: Handle<RestorationBucket>,
    ) {
        debug_assert!(app.get(bucket).manager == Some(self));
        debug_assert!(!app.get(self).debug_doing_update);
        app.get_mut(self)
            .buckets_needing_serialization
            .shift_remove(&bucket);
    }

    fn do_serialization(self: Handle<Self>, app: &mut App) {
        if !app.get(self).serialization_scheduled {
            return;
        }
        if cfg!(debug_assertions) {
            app.get_mut(self).debug_doing_update = true;
        }
        app.get_mut(self).serialization_scheduled = false;

        let buckets = std::mem::take(&mut app.get_mut(self).buckets_needing_serialization);
        for bucket in buckets {
            bucket.finalize(app);
        }
        let root = app
            .get(self)
            .root_bucket
            .expect("a bucket that scheduled serialization is in the root bucket's hierarchy");
        let data = root.with_raw_data(app, |data| data.clone());
        self.send_to_engine(app, data);

        if cfg!(debug_assertions) {
            app.get_mut(self).debug_doing_update = false;
        }
    }

    /// Called to manually flush the restoration data to the host.
    ///
    /// A change in restoration data is usually accompanied by scheduling a frame
    /// (because the restoration data is modified inside a `State::set_state` call,
    /// because it is usually something that affects the interface). Restoration
    /// data is automatically flushed to the host at the end of a frame. As a
    /// result, it is uncommon to need to call this method directly. However, if
    /// restoration data is changed without triggering a frame, this method must
    /// be called to ensure that the updated restoration data is sent to the
    /// host in a timely manner. An example of such a use case is the scrollable,
    /// where the final scroll offset after a scroll activity finishes is
    /// determined between frames without scheduling a new frame.
    ///
    /// Calling this method is a no-op if a frame is already scheduled. In that
    /// case, the restoration data will be flushed to the host at the end of
    /// that frame. If this method is called and no frame is scheduled, the
    /// current restoration data is directly sent to the host.
    pub fn flush_data(self: Handle<Self>, app: &mut App) {
        debug_assert!(!app.get(self).debug_doing_update);
        if SchedulerBinding::has_scheduled_frame(app) {
            return;
        }
        self.do_serialization(app);
        debug_assert!(!app.get(self).serialization_scheduled);
    }

    /// Discards any resources used by the manager, and the root bucket with them.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(root_bucket) = app.get(self).root_bucket {
            root_bucket.dispose(app);
        }
        app.get_mut(self).change_notifier.dispose();
    }
}

const CHILDREN_MAP_KEY: &str = "c";
const VALUES_MAP_KEY: &str = "v";

/// A restoration ID as the raw data maps key it.
fn data_key(restoration_id: &str) -> RestorationData {
    RestorationData::String(restoration_id.to_string())
}

/// Dart's `_rawData.putIfAbsent(key, () => <Object?, Object?>{})`.
fn sub_map<'a>(data: &'a mut RestorationMap, key: &str) -> &'a mut RestorationMap {
    data.entry(data_key(key))
        .or_insert_with(|| RestorationData::Map(RestorationMap::new()))
        .as_map_mut()
        .expect("the values and children entries of raw data are maps")
}

/// Where a bucket's raw map lives.
///
/// Dart's child bucket holds the very map its parent stores under its restoration ID in
/// `_rawChildren`; a Rust value has one owner, so a bucket whose parent stores it reads
/// and writes its map through that parent.
enum RawData {
    Own(RestorationMap),
    InParent,
}

/// A [`RestorationBucket`] holds pieces of the restoration data that a part of
/// the application needs to restore its state.
///
/// For a general overview of how state restoration works, see the
/// [`RestorationManager`].
///
/// [`RestorationBucket`]s are organized in a tree that is rooted in
/// [`RestorationManager::root_bucket`] and managed by a [`RestorationManager`].
/// The tree is serializable and must contain all the data an application needs
/// to restore its current state at a later point in time.
///
/// A [`RestorationBucket`] stores restoration data as key-value pairs. The key is
/// a string representing a restoration ID that identifies a piece of data
/// uniquely within a bucket. The value is any [`RestorationData`]. Furthermore, a
/// [`RestorationBucket`] may have child buckets, which are identified within their
/// parent via a unique restoration ID as well.
///
/// During state restoration, the data previously stored in the
/// [`RestorationBucket`] hierarchy will be made available again to the
/// application to restore it to the state it had when the data was collected.
/// State restoration to a previous state may happen when the app is launched
/// (e.g. after it has been terminated gracefully while running in the
/// background) or after the app has already been running for a while.
///
/// ## Lifecycle
///
/// A [`RestorationBucket`] is rarely instantiated directly via its constructors.
/// Instead, when an entity wants to store data in or retrieve data from a
/// restoration bucket, it typically obtains a child bucket from a parent by
/// calling [`claim_child`](Self::claim_child). If no parent is available,
/// [`RestorationManager::root_bucket`] may be used as a parent. When claiming a
/// child, the claimer must provide the restoration ID of the child it would
/// like to own. A child bucket with a given restoration ID can at most have
/// one owner. If another owner tries to claim a bucket with the same ID from
/// the same parent, an error is raised (see discussion in
/// [`claim_child`](Self::claim_child)). The restoration IDs that a given owner
/// uses to claim a child (and to store data in that child, see below) must be
/// stable across app launches to ensure that after the app restarts the owner can
/// retrieve the same data again that it stored during a previous run.
///
/// Per convention, the owner of the bucket has exclusive access to the values
/// stored in the bucket. It can read, add, modify, and remove values via the
/// [`read`](Self::read), [`write`](Self::write), and [`remove`](Self::remove)
/// methods. In general, the owner should store all the data in the bucket that it
/// needs to restore its current state. If its current state changes, the data in
/// the bucket must be updated. At the same time, the data in the bucket should be
/// kept to a minimum. For example, for data that can be retrieved from other
/// sources (like a database or web service) only enough information (e.g. an ID
/// or resource locator) to re-obtain that data should be stored in the bucket. In
/// addition to managing the data in a bucket, an owner may also make the bucket
/// available to other entities so they can claim child buckets from it via
/// [`claim_child`](Self::claim_child) for their own restoration needs.
///
/// The bucket returned by [`claim_child`](Self::claim_child) may either contain
/// state information that the owner had previously (e.g. during a previous run of
/// the application) stored in it or it may be empty. If the bucket contains data,
/// the owner is expected to restore its state with the information previously
/// stored in the bucket. If the bucket is empty, it may initialize itself to
/// default values.
///
/// When the data stored in a bucket is no longer needed to restore the
/// application to its current state (e.g. because the owner of the bucket is no
/// longer shown on screen), the bucket must be [`dispose`](Self::dispose)d. This
/// will remove all information stored in the bucket from the app's restoration
/// data and that information will not be available again when the application is
/// restored to this state in the future.
pub struct RestorationBucket {
    manager: Option<Handle<RestorationManager>>,
    parent: Option<Handle<RestorationBucket>>,
    raw_data: RawData,
    restoration_id: String,
    // The restoration IDs and associated buckets of children that have been
    // claimed via `claim_child`.
    claimed_children: IndexMap<String, Handle<RestorationBucket>>,
    // Newly created child buckets whose restoration ID is still in use, see
    // comment in `claim_child` for details.
    children_to_add: IndexMap<String, Vec<Handle<RestorationBucket>>>,
    needs_serialization: bool,
    debug_owner: Option<Rc<dyn Debug>>,
}

impl Debug for RestorationBucket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RestorationBucket(restorationId: {}, owner: {})",
            self.restoration_id,
            owner_description(self)
        )
    }
}

/// Dart's `'$debugOwner'`.
fn owner_description(bucket: &RestorationBucket) -> String {
    match &bucket.debug_owner {
        Some(owner) => format!("{owner:?}"),
        None => "None".to_string(),
    }
}

impl RestorationBucket {
    /// Creates an empty [`RestorationBucket`] to be provided to
    /// [`adopt_child`](Self::adopt_child) to add it to the bucket hierarchy.
    ///
    /// Instantiating a bucket directly is rare, most buckets are created by
    /// claiming a child from a parent via [`claim_child`](Self::claim_child). If no
    /// parent bucket is available, [`RestorationManager::root_bucket`] may be used
    /// as a parent.
    pub fn empty(
        app: &mut App,
        restoration_id: &str,
        debug_owner: Option<Rc<dyn Debug>>,
    ) -> Handle<RestorationBucket> {
        let bucket = RestorationBucket::new(restoration_id, RestorationMap::new(), debug_owner);
        app.create(bucket)
    }

    /// Creates the root [`RestorationBucket`] for the provided restoration
    /// `manager`.
    ///
    /// The `raw_data` must either be `None` (in which case an empty bucket will be
    /// instantiated) or it must be a nested map describing the entire bucket
    /// hierarchy in the following format:
    ///
    /// ```text
    /// {
    ///  'v': {  // key-value pairs
    ///     // * key is a string representation of a restoration ID
    ///     // * value is any RestorationData
    ///    '<restoration-id>': <RestorationData>,
    ///   },
    ///  'c': {  // child buckets
    ///    '<restoration-id>': <nested map representing a child bucket>
    ///   }
    /// }
    /// ```
    ///
    /// Instantiating a bucket directly is rare, most buckets are created by
    /// claiming a child from a parent via [`claim_child`](Self::claim_child).
    pub fn root(
        app: &mut App,
        manager: Handle<RestorationManager>,
        raw_data: Option<RestorationMap>,
    ) -> Handle<RestorationBucket> {
        let debug_owner: Option<Rc<dyn Debug>> =
            cfg!(debug_assertions).then(|| Rc::new(manager) as Rc<dyn Debug>);
        let mut bucket = RestorationBucket::new("root", raw_data.unwrap_or_default(), debug_owner);
        bucket.manager = Some(manager);
        app.create(bucket)
    }

    /// Creates a child bucket initialized with the data that the provided
    /// `parent` has stored under the provided `restoration_id`.
    ///
    /// This constructor cannot be used if the `parent` does not have any child
    /// data stored under the given ID. In that case, create an empty bucket (via
    /// [`empty`](Self::empty)) and have the parent adopt it via
    /// [`adopt_child`](Self::adopt_child).
    ///
    /// Instantiating a bucket directly is rare, most buckets are created by
    /// claiming a child from a parent via [`claim_child`](Self::claim_child).
    pub fn child(
        app: &mut App,
        restoration_id: &str,
        parent: Handle<RestorationBucket>,
        debug_owner: Option<Rc<dyn Debug>>,
    ) -> Handle<RestorationBucket> {
        debug_assert!(parent.with_raw_children(app, |children| {
            children.contains_key(&data_key(restoration_id))
        }));
        let mut bucket = RestorationBucket::new(restoration_id, RestorationMap::new(), debug_owner);
        bucket.raw_data = RawData::InParent;
        bucket.manager = app.get(parent).manager;
        bucket.parent = Some(parent);
        app.create(bucket)
    }

    fn new(
        restoration_id: &str,
        raw_data: RestorationMap,
        debug_owner: Option<Rc<dyn Debug>>,
    ) -> RestorationBucket {
        RestorationBucket {
            manager: None,
            parent: None,
            raw_data: RawData::Own(raw_data),
            restoration_id: restoration_id.to_string(),
            claimed_children: IndexMap::new(),
            children_to_add: IndexMap::new(),
            needs_serialization: false,
            debug_owner: cfg!(debug_assertions).then_some(debug_owner).flatten(),
        }
    }

    /// The owner of the bucket that was provided when the bucket was claimed via
    /// [`claim_child`](Self::claim_child).
    ///
    /// The value is used in error messages. Accessing the value is only valid
    /// in debug mode, otherwise it will return `None`.
    pub fn debug_owner(self: Handle<Self>, app: &App) -> Option<Rc<dyn Debug>> {
        debug_assert!(self.debug_assert_not_disposed(app));
        app.get(self).debug_owner.clone()
    }

    /// Returns true when entities processing this bucket should restore their
    /// state from the information in the bucket (e.g. via [`read`](Self::read) and
    /// [`claim_child`](Self::claim_child)) instead of copying their current state
    /// information into the bucket (e.g. via [`write`](Self::write) and
    /// [`adopt_child`](Self::adopt_child)).
    ///
    /// This flag is true for the frame after the [`RestorationManager`] has been
    /// instructed to restore the application from newly provided restoration
    /// data.
    pub fn is_replacing(self: Handle<Self>, app: &App) -> bool {
        app.get(self)
            .manager
            .is_some_and(|manager| manager.is_replacing(app))
    }

    /// The restoration ID under which the bucket is currently stored in the
    /// parent of this bucket (or wants to be stored if it is currently
    /// parent-less).
    pub fn restoration_id(self: Handle<Self>, app: &App) -> &str {
        debug_assert!(self.debug_assert_not_disposed(app));
        &app.get(self).restoration_id
    }

    /// The bucket's own raw map, wherever it currently lives.
    fn with_raw_data<R>(
        self: Handle<Self>,
        app: &mut App,
        action: impl FnOnce(&mut RestorationMap) -> R,
    ) -> R {
        let mut ancestry = Vec::new();
        let mut owner = self;
        while matches!(app.get(owner).raw_data, RawData::InParent) {
            ancestry.push(data_key(&app.get(owner).restoration_id));
            owner = app
                .get(owner)
                .parent
                .expect("a bucket whose map lives in its parent has one");
        }
        let mut map = match &mut app.get_mut(owner).raw_data {
            RawData::Own(map) => map,
            RawData::InParent => unreachable!("the walk ends at the bucket that owns the map"),
        };
        for restoration_id in ancestry.iter().rev() {
            map = map
                .get_mut(&data_key(CHILDREN_MAP_KEY))
                .and_then(RestorationData::as_map_mut)
                .and_then(|children| children.get_mut(restoration_id))
                .and_then(RestorationData::as_map_mut)
                .expect("a stored bucket's map is under its parent's children");
        }
        action(map)
    }

    /// Maps a restoration ID to the raw map representation of a child bucket.
    fn with_raw_children<R>(
        self: Handle<Self>,
        app: &mut App,
        action: impl FnOnce(&mut RestorationMap) -> R,
    ) -> R {
        self.with_raw_data(app, |data| action(sub_map(data, CHILDREN_MAP_KEY)))
    }

    /// Maps a restoration ID to a value that is stored in this bucket.
    fn with_raw_values<R>(
        self: Handle<Self>,
        app: &mut App,
        action: impl FnOnce(&mut RestorationMap) -> R,
    ) -> R {
        self.with_raw_data(app, |data| action(sub_map(data, VALUES_MAP_KEY)))
    }

    // Get and store values.

    /// Returns the value that is currently stored in the bucket under the provided
    /// `restoration_id`.
    ///
    /// Returns `None` if nothing is stored under that ID.
    ///
    /// See also:
    ///
    ///  * [`write`](Self::write), which stores a value in the bucket.
    ///  * [`remove`](Self::remove), which removes a value from the bucket.
    ///  * [`contains`](Self::contains), which checks whether any value is stored
    ///    under a given restoration ID.
    pub fn read(
        self: Handle<Self>,
        app: &mut App,
        restoration_id: &str,
    ) -> Option<RestorationData> {
        debug_assert!(self.debug_assert_not_disposed(app));
        self.with_raw_values(app, |values| values.get(&data_key(restoration_id)).cloned())
    }

    /// Stores the provided `value` under the provided `restoration_id` in the
    /// bucket.
    ///
    /// Any value that has previously been stored under that ID is overwritten
    /// with the new value.
    ///
    /// [`RestorationData::Null`] is stored in the bucket as-is. To remove a value,
    /// use [`remove`](Self::remove).
    ///
    /// See also:
    ///
    ///  * [`read`](Self::read), which retrieves a stored value from the bucket.
    ///  * [`remove`](Self::remove), which removes a value from the bucket.
    ///  * [`contains`](Self::contains), which checks whether any value is stored
    ///    under a given restoration ID.
    pub fn write(
        self: Handle<Self>,
        app: &mut App,
        restoration_id: &str,
        value: impl Into<RestorationData>,
    ) {
        debug_assert!(self.debug_assert_not_disposed(app));
        let value = value.into();
        let key = data_key(restoration_id);
        let written = self.with_raw_values(app, |values| {
            if values.get(&key) != Some(&value) || !values.contains_key(&key) {
                values.insert(key, value);
                return true;
            }
            false
        });
        if written {
            self.mark_needs_serialization(app);
        }
    }

    /// Deletes the value currently stored under the provided `restoration_id` from
    /// the bucket.
    ///
    /// The value removed from the bucket is returned. If no value was stored under
    /// that ID, `None` is returned.
    ///
    /// See also:
    ///
    ///  * [`read`](Self::read), which retrieves a stored value from the bucket.
    ///  * [`write`](Self::write), which stores a value in the bucket.
    ///  * [`contains`](Self::contains), which checks whether any value is stored
    ///    under a given restoration ID.
    pub fn remove(
        self: Handle<Self>,
        app: &mut App,
        restoration_id: &str,
    ) -> Option<RestorationData> {
        debug_assert!(self.debug_assert_not_disposed(app));
        let key = data_key(restoration_id);
        let (result, needs_update) = self.with_raw_data(app, |data| {
            let values = sub_map(data, VALUES_MAP_KEY);
            let needs_update = values.contains_key(&key);
            let result = values.shift_remove(&key);
            let emptied = values.is_empty();
            if emptied {
                data.shift_remove(&data_key(VALUES_MAP_KEY));
            }
            (result, needs_update)
        });
        if needs_update {
            self.mark_needs_serialization(app);
        }
        result
    }

    /// Checks whether a value is stored in the bucket under the provided
    /// `restoration_id`.
    ///
    /// See also:
    ///
    ///  * [`read`](Self::read), which retrieves a stored value from the bucket.
    ///  * [`write`](Self::write), which stores a value in the bucket.
    ///  * [`remove`](Self::remove), which removes a value from the bucket.
    pub fn contains(self: Handle<Self>, app: &mut App, restoration_id: &str) -> bool {
        debug_assert!(self.debug_assert_not_disposed(app));
        self.with_raw_values(app, |values| values.contains_key(&data_key(restoration_id)))
    }

    // Child management.

    /// Claims ownership of the child with the provided `restoration_id` from this
    /// bucket.
    ///
    /// If the application is getting restored to a previous state, the bucket
    /// will contain all the data that was previously stored in the bucket.
    /// Otherwise, an empty bucket is returned.
    ///
    /// The claimer of the bucket is expected to use the data stored in the bucket
    /// to restore itself to its previous state described by the data in the
    /// bucket. If the bucket is empty, it should initialize itself to default
    /// values. Whenever the information that the claimer needs to restore its
    /// state changes, the data in the bucket should be updated to reflect that.
    ///
    /// A child bucket with a given `restoration_id` can only have one owner. If
    /// another owner claims a child bucket with the same `restoration_id` the end
    /// of the current frame panics in debug mode unless the previous owner has
    /// either deleted its bucket by calling [`dispose`](Self::dispose) or has moved
    /// it to a new parent via [`adopt_child`](Self::adopt_child).
    ///
    /// When the returned bucket is no longer needed, it must be
    /// [`dispose`](Self::dispose)d to delete the information stored in it from the
    /// app's restoration data.
    pub fn claim_child(
        self: Handle<Self>,
        app: &mut App,
        restoration_id: &str,
        debug_owner: Option<Rc<dyn Debug>>,
    ) -> Handle<RestorationBucket> {
        debug_assert!(self.debug_assert_not_disposed(app));
        // There are three cases to consider:
        // 1. Claiming an ID that has already been claimed.
        // 2. Claiming an ID that doesn't yet exist in the raw children.
        // 3. Claiming an ID that does exist in the raw children and hasn't been
        //    claimed yet.
        // If an ID has already been claimed (case 1) the current owner may give up
        // that ID later this frame and it can be re-used. In anticipation of the
        // previous owner's surrender of the id, we return an empty bucket for this
        // new claim and check in `debug_assert_integrity` that at the end of the
        // frame the old owner actually did surrender the id.
        // Case 2 also requires the creation of a new empty bucket.
        // In case 3 we create a new bucket wrapping the existing raw child data.

        // Case 1+2: Adopt and return an empty bucket.
        let claimed = app.get(self).claimed_children.contains_key(restoration_id);
        let stored = self.with_raw_children(app, |children| {
            children.contains_key(&data_key(restoration_id))
        });
        if claimed || !stored {
            let child = RestorationBucket::empty(app, restoration_id, debug_owner);
            self.adopt_child(app, child);
            return child;
        }

        // Case 3: Return a bucket wrapping the existing data.
        let child = RestorationBucket::child(app, restoration_id, self, debug_owner);
        app.get_mut(self)
            .claimed_children
            .insert(restoration_id.to_string(), child);
        child
    }

    /// Adopts the provided `child` bucket.
    ///
    /// The `child` will be dropped from its old parent, if it had one.
    ///
    /// The `child` is stored under its
    /// [`restoration_id`](Self::restoration_id) in this bucket. If this bucket
    /// already contains a child bucket under the same ID, the owner of that
    /// existing bucket must give it up (e.g. by moving the child bucket to a
    /// different parent or by disposing it) before the end of the current frame.
    /// Otherwise the illegal use of duplicated restoration IDs panics in debug
    /// mode.
    ///
    /// No-op if the provided bucket is already a child of this bucket.
    pub fn adopt_child(self: Handle<Self>, app: &mut App, child: Handle<RestorationBucket>) {
        debug_assert!(self.debug_assert_not_disposed(app));
        if app.get(child).parent != Some(self) {
            if let Some(old_parent) = app.get(child).parent {
                old_parent.remove_child_data(app, child);
            }
            app.get_mut(child).parent = Some(self);
            self.add_child_data(app, child);
            if app.get(child).manager != app.get(self).manager {
                self.recursively_update_manager(app, child);
            }
        }
        debug_assert!(app.get(child).parent == Some(self));
        debug_assert!(app.get(child).manager == app.get(self).manager);
    }

    fn drop_child(self: Handle<Self>, app: &mut App, child: Handle<RestorationBucket>) {
        debug_assert!(app.get(child).parent == Some(self));
        self.remove_child_data(app, child);
        app.get_mut(child).parent = None;
        if app.get(child).manager.is_some() {
            child.update_manager(app, None);
            child.visit_children(app, |app, grandchild| {
                self.recursively_update_manager(app, grandchild);
            });
        }
    }

    fn mark_needs_serialization(self: Handle<Self>, app: &mut App) {
        if !app.get(self).needs_serialization {
            app.get_mut(self).needs_serialization = true;
            if let Some(manager) = app.get(self).manager {
                manager.schedule_serialization_for(app, self);
            }
        }
    }

    /// Called by the [`RestorationManager`] just before the data of the bucket
    /// is serialized and sent to the host.
    pub fn finalize(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.debug_assert_not_disposed(app));
        debug_assert!(app.get(self).needs_serialization);
        app.get_mut(self).needs_serialization = false;
        debug_assert!(self.debug_assert_integrity(app));
    }

    fn recursively_update_manager(self: Handle<Self>, app: &mut App, bucket: Handle<Self>) {
        bucket.update_manager(app, app.get(self).manager);
        bucket.visit_children(app, |app, child| {
            self.recursively_update_manager(app, child)
        });
    }

    fn update_manager(
        self: Handle<Self>,
        app: &mut App,
        new_manager: Option<Handle<RestorationManager>>,
    ) {
        if app.get(self).manager == new_manager {
            return;
        }
        if app.get(self).needs_serialization
            && let Some(manager) = app.get(self).manager
        {
            manager.unschedule_serialization_for(app, self);
        }
        app.get_mut(self).manager = new_manager;
        if app.get(self).needs_serialization && app.get(self).manager.is_some() {
            app.get_mut(self).needs_serialization = false;
            self.mark_needs_serialization(app);
        }
    }

    fn debug_assert_integrity(self: Handle<Self>, app: &App) -> bool {
        if !cfg!(debug_assertions) {
            return true;
        }
        let this = app.get(self);
        if this.children_to_add.is_empty() {
            return true;
        }
        let mut error = vec![
            "Multiple owners claimed child RestorationBuckets with the same IDs.".to_string(),
            format!("The following IDs were claimed multiple times from the parent {this:?}:"),
        ];
        for (restoration_id, buckets) in &this.children_to_add {
            debug_assert!(!buckets.is_empty());
            error.push(format!(" * \"{restoration_id}\" was claimed by:"));
            for bucket in buckets {
                error.push(format!("   * {}", owner_description(app.get(*bucket))));
            }
            let owner = this
                .claimed_children
                .get(restoration_id)
                .expect("a pending child implies a current owner of the same ID");
            error.push(format!(
                "   * {} (current owner)",
                owner_description(app.get(*owner))
            ));
        }
        panic!("{}", error.join("\n"));
    }

    fn remove_child_data(self: Handle<Self>, app: &mut App, child: Handle<RestorationBucket>) {
        debug_assert!(app.get(child).parent == Some(self));
        let child_id = app.get(child).restoration_id.clone();
        if app.get_mut(self).claimed_children.shift_remove(&child_id) == Some(child) {
            child.reclaim_raw_data(app);
            let pending_child = app
                .get_mut(self)
                .children_to_add
                .get_mut(&child_id)
                .and_then(Vec::pop);
            if let Some(to_add) = pending_child {
                self.finalize_add_child_data(app, to_add);
                if app.get(self).children_to_add[&child_id].is_empty() {
                    app.get_mut(self).children_to_add.shift_remove(&child_id);
                }
            }
            let no_children = self.with_raw_children(app, |children| children.is_empty());
            if no_children {
                self.with_raw_data(app, |data| data.shift_remove(&data_key(CHILDREN_MAP_KEY)));
            }
            self.mark_needs_serialization(app);
            return;
        }
        let Some(pending) = app.get_mut(self).children_to_add.get_mut(&child_id) else {
            return;
        };
        if let Some(index) = pending.iter().position(|bucket| *bucket == child) {
            pending.remove(index);
        }
        if pending.is_empty() {
            app.get_mut(self).children_to_add.shift_remove(&child_id);
        }
    }

    fn add_child_data(self: Handle<Self>, app: &mut App, child: Handle<RestorationBucket>) {
        debug_assert!(app.get(child).parent == Some(self));
        let child_id = app.get(child).restoration_id.clone();
        if app.get(self).claimed_children.contains_key(&child_id) {
            // Delay addition until the end of the frame in the hopes that the current
            // owner of the child with the same ID will have given up that child by
            // then.
            app.get_mut(self)
                .children_to_add
                .entry(child_id)
                .or_default()
                .push(child);
            self.mark_needs_serialization(app);
            return;
        }
        self.finalize_add_child_data(app, child);
        self.mark_needs_serialization(app);
    }

    fn finalize_add_child_data(
        self: Handle<Self>,
        app: &mut App,
        child: Handle<RestorationBucket>,
    ) {
        let child_id = app.get(child).restoration_id.clone();
        debug_assert!(!app.get(self).claimed_children.contains_key(&child_id));
        debug_assert!(
            self.with_raw_children(app, |children| !children.contains_key(&data_key(&child_id)))
        );
        app.get_mut(self)
            .claimed_children
            .insert(child_id.clone(), child);
        let data = match std::mem::replace(&mut app.get_mut(child).raw_data, RawData::InParent) {
            RawData::Own(data) => data,
            RawData::InParent => unreachable!("a bucket being stored owns its map"),
        };
        self.with_raw_children(app, |children| {
            children.insert(data_key(&child_id), RestorationData::Map(data));
        });
    }

    /// Dart's `parent._rawChildren.remove(id)`: the map the parent stored comes back to
    /// the bucket it belongs to.
    fn reclaim_raw_data(self: Handle<Self>, app: &mut App) {
        if matches!(app.get(self).raw_data, RawData::Own(_)) {
            return;
        }
        let parent = app
            .get(self)
            .parent
            .expect("a bucket whose map lives in its parent has one");
        let key = data_key(&app.get(self).restoration_id);
        let data = parent
            .with_raw_children(app, |children| children.shift_remove(&key))
            .and_then(RestorationData::into_map)
            .expect("a stored bucket's map is under its parent's children");
        app.get_mut(self).raw_data = RawData::Own(data);
    }

    fn visit_children(
        self: Handle<Self>,
        app: &mut App,
        visitor: impl Fn(&mut App, Handle<RestorationBucket>),
    ) {
        let this = app.get(self);
        let children: Vec<Handle<RestorationBucket>> = this
            .claimed_children
            .values()
            .copied()
            .chain(this.children_to_add.values().flatten().copied())
            .collect();
        for child in children {
            visitor(app, child);
        }
    }

    // Bucket management.

    /// Changes the restoration ID under which the bucket is (or will be) stored
    /// in its parent to `new_restoration_id`.
    ///
    /// No-op if the bucket is already stored under the provided ID.
    ///
    /// If another owner has already claimed a bucket with the provided
    /// `new_restoration_id` the end of the current frame panics in debug mode
    /// unless the other owner has deleted its bucket by calling
    /// [`dispose`](Self::dispose), [`rename`](Self::rename)d it using another ID, or
    /// has moved it to a new parent via [`adopt_child`](Self::adopt_child).
    pub fn rename(self: Handle<Self>, app: &mut App, new_restoration_id: &str) {
        debug_assert!(self.debug_assert_not_disposed(app));
        if new_restoration_id == app.get(self).restoration_id {
            return;
        }
        if let Some(parent) = app.get(self).parent {
            parent.remove_child_data(app, self);
        }
        app.get_mut(self).restoration_id = new_restoration_id.to_string();
        if let Some(parent) = app.get(self).parent {
            parent.add_child_data(app, self);
        }
    }

    /// Deletes the bucket and all the data stored in it from the bucket
    /// hierarchy.
    ///
    /// After [`dispose`](Self::dispose) has been called, the data stored in this
    /// bucket and its children are no longer part of the app's restoration data.
    /// The data originally stored in the bucket will not be available again when
    /// the application is restored to this state in the future. It is up to the
    /// owners of the children to either move them (via
    /// [`adopt_child`](Self::adopt_child)) to a new parent that is still part of
    /// the bucket hierarchy or to [`dispose`](Self::dispose) of them as well.
    ///
    /// This method must only be called by the object's owner.
    pub fn dispose(self: Handle<Self>, app: &mut App) {
        debug_assert!(self.debug_assert_not_disposed(app));
        self.visit_children(app, |app, child| self.drop_child(app, child));
        app.get_mut(self).claimed_children.clear();
        app.get_mut(self).children_to_add.clear();
        if let Some(parent) = app.get(self).parent {
            parent.remove_child_data(app, self);
        }
        app.get_mut(self).parent = None;
        self.update_manager(app, None);
        app.destroy(self);
    }

    fn debug_assert_not_disposed(self: Handle<Self>, app: &App) -> bool {
        assert!(
            app.contains(self),
            "A RestorationBucket was used after being disposed.\n\
             Once you have called dispose() on a RestorationBucket, it can no longer be used."
        );
        true
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::cell::{Cell, RefCell};
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::time::{Duration, Instant};

    use inset_embedder::{
        InertPlatform, Platform, PlatformRef, Restoration, TargetPlatform, ViewId, ViewRef,
    };
    use inset_foundation::{ListenableObject, Listener};

    use super::*;

    /// A host that answers `Restoration::get` with what it was handed and records every
    /// `Restoration::put`.
    #[derive(Default)]
    struct RecordingPlatform {
        stored: RefCell<Option<RestorationUpdate>>,
        puts: RefCell<Vec<RestorationMap>>,
        gets: Cell<usize>,
    }

    impl Platform for RecordingPlatform {
        fn target_platform(&self) -> TargetPlatform {
            InertPlatform.target_platform()
        }

        fn request_frame(&self) {}

        fn now(&self) -> Instant {
            InertPlatform.now()
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

        fn restoration(&self) -> Option<&dyn Restoration> {
            Some(self)
        }
    }

    impl Restoration for RecordingPlatform {
        fn get(&self) -> Option<RestorationUpdate> {
            self.gets.set(self.gets.get() + 1);
            self.stored.borrow().clone()
        }

        fn put(&self, data: RestorationMap) {
            self.puts.borrow_mut().push(data);
        }
    }

    struct Fixture {
        cell: Rc<AppCell>,
        platform: Rc<RecordingPlatform>,
        manager: Handle<RestorationManager>,
    }

    impl Fixture {
        fn new() -> Fixture {
            let platform = Rc::new(RecordingPlatform::default());
            let cell = AppCell::with_platform(Rc::clone(&platform) as PlatformRef);
            let mut app = cell.borrow_mut();
            let manager = RestorationManager::instance(&mut app);
            drop(app);
            Fixture {
                cell,
                platform,
                manager,
            }
        }

        /// A manager whose root bucket holds `data`, as after a restart.
        fn restored(data: RestorationMap) -> (Fixture, Handle<RestorationBucket>) {
            let mut fixture = Fixture::new();
            fixture.push_from_host(true, Some(data));
            let root = fixture
                .manager
                .root_bucket(&mut fixture.cell.borrow_mut())
                .expect("restoration is enabled");
            (fixture, root)
        }

        /// Flutter's `_pushDataFromEngine`.
        fn push_from_host(&mut self, enabled: bool, data: Option<RestorationMap>) {
            self.manager.handle_restoration_update_from_engine(
                &mut self.cell.borrow_mut(),
                enabled,
                data,
            );
        }

        /// Flutter's `MockRestorationManager.doSerialization`: what reached the host.
        fn serialize(&mut self) -> Vec<RestorationMap> {
            self.manager.flush_data(&mut self.cell.borrow_mut());
            self.platform.puts.borrow_mut().drain(..).collect()
        }

        /// The data the host would keep, or `None` when nothing was scheduled.
        fn serialized(&mut self) -> Option<RestorationMap> {
            let mut puts = self.serialize();
            assert!(puts.len() <= 1, "one serialization sends one message");
            puts.pop()
        }

        fn pump(&mut self) {
            SchedulerBinding::handle_begin_frame(&mut self.cell.borrow_mut(), Some(Duration::ZERO));
            self.cell.borrow_mut().drain_microtasks();
            SchedulerBinding::handle_draw_frame(&mut self.cell.borrow_mut());
            self.cell.borrow_mut().drain_microtasks();
        }
    }

    fn map<const N: usize>(entries: [(&str, RestorationData); N]) -> RestorationMap {
        entries
            .into_iter()
            .map(|(key, value)| (RestorationData::from(key), value))
            .collect()
    }

    /// Flutter's `_createRawDataSet`.
    fn raw_data_set() -> RestorationMap {
        map([
            (
                "v",
                map([("value1", 10i64.into()), ("value2", "Hello".into())]).into(),
            ),
            (
                "c",
                map([(
                    "child1",
                    map([("v", map([("foo", 22i64.into())]).into())]).into(),
                )])
                .into(),
            ),
        ])
    }

    /// Flutter's `_createEncodedRestorationData2`.
    fn other_raw_data_set() -> RestorationMap {
        map([
            ("v", map([("foo", 33i64.into())]).into()),
            (
                "c",
                map([(
                    "childFoo",
                    map([("v", map([("bar", "Hello".into())]).into())]).into(),
                )])
                .into(),
            ),
        ])
    }

    /// The value stored down `path`, e.g. `["c", "child1", "v", "foo"]`.
    fn at<'a>(data: &'a RestorationMap, path: &[&str]) -> Option<&'a RestorationData> {
        let (last, parents) = path.split_last()?;
        let mut map = data;
        for key in parents {
            map = map.get(&RestorationData::from(*key))?.as_map()?;
        }
        map.get(&RestorationData::from(*last))
    }

    fn owner(name: &'static str) -> Option<Rc<dyn Debug>> {
        Some(Rc::new(name))
    }

    fn panic_message(error: &Box<dyn std::any::Any + Send>) -> String {
        error
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| error.downcast_ref::<&str>().map(|text| text.to_string()))
            .unwrap_or_default()
    }

    #[test]
    fn the_root_bucket_comes_from_the_host_and_is_asked_for_once() {
        let fixture = Fixture::new();
        *fixture.platform.stored.borrow_mut() = Some(RestorationUpdate {
            enabled: true,
            data: Some(raw_data_set()),
        });

        let root = fixture
            .manager
            .root_bucket(&mut fixture.cell.borrow_mut())
            .expect("the host enabled restoration");
        assert_eq!(fixture.platform.gets.get(), 1);
        assert_eq!(root.restoration_id(&fixture.cell.borrow()), "root");
        assert_eq!(
            root.read(&mut fixture.cell.borrow_mut(), "value1"),
            Some(10i64.into())
        );
        assert_eq!(
            root.read(&mut fixture.cell.borrow_mut(), "value2"),
            Some("Hello".into())
        );

        let child = root.claim_child(&mut fixture.cell.borrow_mut(), "child1", None);
        assert_eq!(
            child.read(&mut fixture.cell.borrow_mut(), "foo"),
            Some(22i64.into())
        );

        assert_eq!(
            fixture.manager.root_bucket(&mut fixture.cell.borrow_mut()),
            Some(root)
        );
        assert_eq!(
            fixture.platform.gets.get(),
            1,
            "the host is asked once, then the answer is kept"
        );
    }

    #[test]
    fn data_received_before_the_first_access_is_used_without_asking_the_host() {
        let mut fixture = Fixture::new();
        fixture.push_from_host(true, Some(raw_data_set()));

        let root = fixture.manager.root_bucket(&mut fixture.cell.borrow_mut());
        assert!(root.is_some());
        assert_eq!(fixture.platform.gets.get(), 0);
    }

    #[test]
    fn new_data_replaces_the_root_bucket_and_notifies_listeners() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        let child = root.claim_child(&mut fixture.cell.borrow_mut(), "child1", None);
        assert_eq!(
            child.read(&mut fixture.cell.borrow_mut(), "foo"),
            Some(22i64.into())
        );

        let notifications = Rc::new(Cell::new(0));
        let counter = Rc::clone(&notifications);
        fixture.manager.add_listener(
            &mut fixture.cell.borrow_mut(),
            Listener::new(move |_app| counter.set(counter.get() + 1)),
        );

        fixture.push_from_host(true, Some(other_raw_data_set()));
        assert_eq!(notifications.get(), 1);

        let new_root = fixture
            .manager
            .root_bucket(&mut fixture.cell.borrow_mut())
            .expect("restoration is still enabled");
        assert_ne!(new_root, root);
        assert!(
            !fixture.cell.borrow().contains(root),
            "the old root was disposed"
        );

        child.dispose(&mut fixture.cell.borrow_mut());

        assert_eq!(
            new_root.read(&mut fixture.cell.borrow_mut(), "foo"),
            Some(33i64.into())
        );
        assert_eq!(
            new_root.read(&mut fixture.cell.borrow_mut(), "value1"),
            None
        );
        let new_child = new_root.claim_child(&mut fixture.cell.borrow_mut(), "childFoo", None);
        assert_eq!(
            new_child.read(&mut fixture.cell.borrow_mut(), "bar"),
            Some("Hello".into())
        );
    }

    #[test]
    fn there_is_no_root_bucket_while_restoration_is_disabled() {
        let mut fixture = Fixture::new();
        *fixture.platform.stored.borrow_mut() = Some(RestorationUpdate {
            enabled: false,
            data: None,
        });
        let notifications = Rc::new(Cell::new(0));
        let counter = Rc::clone(&notifications);
        fixture.manager.add_listener(
            &mut fixture.cell.borrow_mut(),
            Listener::new(move |_app| counter.set(counter.get() + 1)),
        );

        assert_eq!(
            fixture.manager.root_bucket(&mut fixture.cell.borrow_mut()),
            None
        );
        assert_eq!(notifications.get(), 0);

        fixture.push_from_host(true, Some(raw_data_set()));
        assert_eq!(notifications.get(), 1);
        assert!(
            fixture
                .manager
                .root_bucket(&mut fixture.cell.borrow_mut())
                .is_some()
        );

        fixture.push_from_host(false, None);
        assert_eq!(notifications.get(), 2);
        assert_eq!(
            fixture.manager.root_bucket(&mut fixture.cell.borrow_mut()),
            None
        );
    }

    #[test]
    fn a_host_that_stores_nothing_leaves_restoration_off() {
        let fixture = Fixture::new();
        assert_eq!(
            fixture.manager.root_bucket(&mut fixture.cell.borrow_mut()),
            None
        );
        assert_eq!(fixture.platform.gets.get(), 1);
    }

    #[test]
    fn is_replacing_is_true_until_the_end_of_the_next_frame() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        assert!(!fixture.manager.is_replacing(&fixture.cell.borrow()));
        assert!(!root.is_replacing(&fixture.cell.borrow()));

        fixture.push_from_host(true, None);
        let new_root = fixture
            .manager
            .root_bucket(&mut fixture.cell.borrow_mut())
            .expect("restoration is enabled");
        assert_ne!(new_root, root);
        assert!(fixture.manager.is_replacing(&fixture.cell.borrow()));
        assert!(new_root.is_replacing(&fixture.cell.borrow()));

        fixture.pump();
        assert!(!fixture.manager.is_replacing(&fixture.cell.borrow()));
        assert!(!new_root.is_replacing(&fixture.cell.borrow()));

        fixture.push_from_host(false, None);
        assert!(!fixture.manager.is_replacing(&fixture.cell.borrow()));
    }

    #[test]
    fn scheduled_serialization_reaches_the_host_at_the_end_of_the_frame() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        root.write(&mut fixture.cell.borrow_mut(), "value1", 22i64);
        assert!(fixture.platform.puts.borrow().is_empty());

        fixture.pump();

        let puts = fixture
            .platform
            .puts
            .borrow_mut()
            .drain(..)
            .collect::<Vec<_>>();
        assert_eq!(puts.len(), 1);
        assert_eq!(at(&puts[0], &["v", "value1"]), Some(&22i64.into()));
    }

    #[test]
    fn flush_data_waits_for_a_scheduled_frame_and_otherwise_sends_at_once() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        SchedulerBinding::schedule_frame(&mut fixture.cell.borrow_mut());
        root.write(&mut fixture.cell.borrow_mut(), "foo", 1i64);

        fixture.manager.flush_data(&mut fixture.cell.borrow_mut());
        assert!(
            fixture.platform.puts.borrow().is_empty(),
            "the scheduled frame will flush it"
        );

        fixture.pump();
        assert_eq!(fixture.platform.puts.borrow_mut().drain(..).count(), 1);

        root.write(&mut fixture.cell.borrow_mut(), "foo", 2i64);
        let data = fixture
            .serialized()
            .expect("a write schedules serialization");
        assert_eq!(at(&data, &["v", "foo"]), Some(&2i64.into()));
    }

    #[test]
    fn nothing_is_sent_when_no_bucket_asked_for_serialization() {
        let (mut fixture, _root) = Fixture::restored(raw_data_set());
        assert!(fixture.serialized().is_none());
    }

    #[test]
    fn root_bucket_values() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        assert_eq!(
            root.debug_owner(&fixture.cell.borrow()).is_some(),
            cfg!(debug_assertions)
        );
        assert_eq!(
            root.read(&mut fixture.cell.borrow_mut(), "value1"),
            Some(10i64.into())
        );
        assert_eq!(
            root.read(&mut fixture.cell.borrow_mut(), "value2"),
            Some("Hello".into())
        );
        assert_eq!(root.read(&mut fixture.cell.borrow_mut(), "value3"), None);
        assert!(fixture.serialized().is_none());

        root.write(&mut fixture.cell.borrow_mut(), "value1", 22i64);
        assert_eq!(
            root.read(&mut fixture.cell.borrow_mut(), "value1"),
            Some(22i64.into())
        );
        let data = fixture
            .serialized()
            .expect("a write schedules serialization");
        assert_eq!(at(&data, &["v", "value1"]), Some(&22i64.into()));

        root.write(&mut fixture.cell.borrow_mut(), "value3", true);
        assert_eq!(
            root.read(&mut fixture.cell.borrow_mut(), "value3"),
            Some(true.into())
        );
        let data = fixture
            .serialized()
            .expect("a write schedules serialization");
        assert_eq!(at(&data, &["v", "value3"]), Some(&true.into()));

        assert_eq!(
            root.remove(&mut fixture.cell.borrow_mut(), "value1"),
            Some(22i64.into())
        );
        assert_eq!(root.read(&mut fixture.cell.borrow_mut(), "value1"), None);
        let data = fixture
            .serialized()
            .expect("a removal schedules serialization");
        assert_eq!(at(&data, &["v", "value1"]), None);

        assert_eq!(root.remove(&mut fixture.cell.borrow_mut(), "value4"), None);
        assert!(
            fixture.serialized().is_none(),
            "removing what is not there changes nothing"
        );

        root.write(&mut fixture.cell.borrow_mut(), "value4", None::<i64>);
        assert_eq!(
            root.read(&mut fixture.cell.borrow_mut(), "value4"),
            Some(RestorationData::Null)
        );
        assert!(root.contains(&mut fixture.cell.borrow_mut(), "value4"));
        let data = fixture
            .serialized()
            .expect("a write schedules serialization");
        assert_eq!(at(&data, &["v", "value4"]), Some(&RestorationData::Null));
    }

    #[test]
    fn child_bucket_values() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        let child = RestorationBucket::child(
            &mut fixture.cell.borrow_mut(),
            "child1",
            root,
            owner("owner"),
        );

        assert_eq!(child.restoration_id(&fixture.cell.borrow()), "child1");
        assert_eq!(
            child.read(&mut fixture.cell.borrow_mut(), "foo"),
            Some(22i64.into())
        );
        assert_eq!(child.read(&mut fixture.cell.borrow_mut(), "bar"), None);
        assert!(fixture.serialized().is_none());

        child.write(&mut fixture.cell.borrow_mut(), "foo", 44i64);
        assert_eq!(
            child.read(&mut fixture.cell.borrow_mut(), "foo"),
            Some(44i64.into())
        );
        let data = fixture
            .serialized()
            .expect("a write schedules serialization");
        assert_eq!(at(&data, &["c", "child1", "v", "foo"]), Some(&44i64.into()));

        assert_eq!(
            child.remove(&mut fixture.cell.borrow_mut(), "foo"),
            Some(44i64.into())
        );
        let data = fixture
            .serialized()
            .expect("a removal schedules serialization");
        assert_eq!(at(&data, &["c", "child1", "v", "foo"]), None);
    }

    #[test]
    fn claiming_a_child_with_existing_data_keeps_that_data() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        let child = root.claim_child(&mut fixture.cell.borrow_mut(), "child1", owner("owner"));
        assert!(
            fixture.serialized().is_none(),
            "claiming stored data changes nothing"
        );
        assert_eq!(child.restoration_id(&fixture.cell.borrow()), "child1");
        assert_eq!(
            child.read(&mut fixture.cell.borrow_mut(), "foo"),
            Some(22i64.into())
        );

        child.write(&mut fixture.cell.borrow_mut(), "bar", 44i64);
        let data = fixture
            .serialized()
            .expect("a write schedules serialization");
        assert_eq!(at(&data, &["c", "child1", "v", "bar"]), Some(&44i64.into()));
    }

    #[test]
    fn claiming_a_child_with_no_existing_data_gives_an_empty_bucket() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        let child = root.claim_child(&mut fixture.cell.borrow_mut(), "child2", owner("owner"));
        assert_eq!(child.restoration_id(&fixture.cell.borrow()), "child2");

        child.write(&mut fixture.cell.borrow_mut(), "foo", 55i64);
        assert_eq!(
            child.read(&mut fixture.cell.borrow_mut(), "foo"),
            Some(55i64.into())
        );
        let data = fixture
            .serialized()
            .expect("a new child schedules serialization");
        assert_eq!(at(&data, &["c", "child2", "v", "foo"]), Some(&55i64.into()));
    }

    #[cfg(debug_assertions)]
    #[test]
    fn claiming_a_claimed_child_reports_the_duplicate_ids_at_finalization() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        let _first = root.claim_child(
            &mut fixture.cell.borrow_mut(),
            "child1",
            owner("FirstClaim"),
        );
        let second = root.claim_child(
            &mut fixture.cell.borrow_mut(),
            "child1",
            owner("SecondClaim"),
        );
        assert_eq!(second.read(&mut fixture.cell.borrow_mut(), "foo"), None);

        let error = catch_unwind(AssertUnwindSafe(|| fixture.serialized())).unwrap_err();
        let message = panic_message(&error);
        assert!(
            message.contains("Multiple owners claimed child RestorationBuckets with the same IDs."),
            "{message}"
        );
        assert!(
            message.contains(" * \"child1\" was claimed by:"),
            "{message}"
        );
        assert!(message.contains("\"SecondClaim\""), "{message}");
        assert!(
            message.contains("\"FirstClaim\" (current owner)"),
            "{message}"
        );
    }

    #[test]
    fn claiming_a_claimed_child_is_fine_once_the_first_owner_gives_it_up() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        let first = root.claim_child(
            &mut fixture.cell.borrow_mut(),
            "child1",
            owner("FirstClaim"),
        );
        let second = root.claim_child(
            &mut fixture.cell.borrow_mut(),
            "child1",
            owner("SecondClaim"),
        );
        second.write(&mut fixture.cell.borrow_mut(), "bar", 55i64);

        first.dispose(&mut fixture.cell.borrow_mut());

        let data = fixture
            .serialized()
            .expect("the shuffle schedules serialization");
        assert_eq!(at(&data, &["c", "child1", "v", "foo"]), None);
        assert_eq!(at(&data, &["c", "child1", "v", "bar"]), Some(&55i64.into()));
    }

    #[test]
    fn unclaiming_and_claiming_the_same_id_gives_a_fresh_bucket() {
        let (fixture, root) = Fixture::restored(raw_data_set());
        let first = root.claim_child(
            &mut fixture.cell.borrow_mut(),
            "child1",
            owner("FirstClaim"),
        );
        assert_eq!(
            first.read(&mut fixture.cell.borrow_mut(), "foo"),
            Some(22i64.into())
        );
        first.dispose(&mut fixture.cell.borrow_mut());

        let second = root.claim_child(
            &mut fixture.cell.borrow_mut(),
            "child1",
            owner("SecondClaim"),
        );
        assert_eq!(second.read(&mut fixture.cell.borrow_mut(), "foo"), None);
    }

    #[test]
    fn the_raw_data_drops_the_values_and_children_maps_when_they_empty_out() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        let child = root.claim_child(&mut fixture.cell.borrow_mut(), "child1", owner("owner"));
        child.dispose(&mut fixture.cell.borrow_mut());
        let data = fixture
            .serialized()
            .expect("dropping a child schedules serialization");
        assert_eq!(at(&data, &["c"]), None);

        assert_eq!(
            root.remove(&mut fixture.cell.borrow_mut(), "value1"),
            Some(10i64.into())
        );
        assert_eq!(
            root.remove(&mut fixture.cell.borrow_mut(), "value2"),
            Some("Hello".into())
        );
        let data = fixture
            .serialized()
            .expect("a removal schedules serialization");
        assert_eq!(at(&data, &["v"]), None);
    }

    #[test]
    fn dispose_deletes_the_data_of_the_whole_subtree() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        let child1 = root.claim_child(&mut fixture.cell.borrow_mut(), "child1", owner("owner1"));
        child1.claim_child(
            &mut fixture.cell.borrow_mut(),
            "child1OfChild1",
            owner("owner1.1"),
        );
        let child2 = root.claim_child(&mut fixture.cell.borrow_mut(), "child2", owner("owner2"));
        child2.write(&mut fixture.cell.borrow_mut(), "foo", 1i64);

        let data = fixture
            .serialized()
            .expect("new children schedule serialization");
        assert!(at(&data, &["c", "child1"]).is_some());
        assert!(at(&data, &["c", "child2"]).is_some());

        child1.dispose(&mut fixture.cell.borrow_mut());
        let data = fixture
            .serialized()
            .expect("dropping a child schedules serialization");
        assert_eq!(at(&data, &["c", "child1"]), None);

        child2.dispose(&mut fixture.cell.borrow_mut());
        let data = fixture
            .serialized()
            .expect("dropping a child schedules serialization");
        assert_eq!(at(&data, &["c"]), None);
    }

    #[test]
    fn rename_to_the_same_id_changes_nothing() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        let child = root.claim_child(&mut fixture.cell.borrow_mut(), "child1", owner("owner1"));

        child.rename(&mut fixture.cell.borrow_mut(), "child1");
        assert_eq!(child.restoration_id(&fixture.cell.borrow()), "child1");
        assert!(fixture.serialized().is_none());
    }

    #[test]
    fn rename_to_an_unused_id_moves_the_data() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        let child = root.claim_child(&mut fixture.cell.borrow_mut(), "child1", owner("owner1"));

        child.rename(&mut fixture.cell.borrow_mut(), "new-name");
        assert_eq!(child.restoration_id(&fixture.cell.borrow()), "new-name");

        let data = fixture
            .serialized()
            .expect("a rename schedules serialization");
        assert_eq!(at(&data, &["c", "child1"]), None);
        assert_eq!(
            at(&data, &["c", "new-name", "v", "foo"]),
            Some(&22i64.into())
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn rename_onto_a_used_id_reports_the_duplicate_unless_it_is_given_up() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        let _child1 = root.claim_child(&mut fixture.cell.borrow_mut(), "child1", owner("owner1"));
        let child2 = root.claim_child(&mut fixture.cell.borrow_mut(), "child2", owner("owner2"));
        child2.rename(&mut fixture.cell.borrow_mut(), "child1");

        let error = catch_unwind(AssertUnwindSafe(|| fixture.serialized())).unwrap_err();
        assert!(
            panic_message(&error)
                .contains("Multiple owners claimed child RestorationBuckets with the same IDs."),
        );
    }

    #[test]
    fn rename_onto_a_given_up_id_takes_it_over() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        let child1 = root.claim_child(&mut fixture.cell.borrow_mut(), "child1", owner("owner1"));
        let child2 = root.claim_child(&mut fixture.cell.borrow_mut(), "child2", owner("owner2"));
        child2.write(&mut fixture.cell.borrow_mut(), "bar", 7i64);
        fixture.serialize();

        child2.rename(&mut fixture.cell.borrow_mut(), "child1");
        child1.dispose(&mut fixture.cell.borrow_mut());

        let data = fixture
            .serialized()
            .expect("the shuffle schedules serialization");
        assert_eq!(at(&data, &["c", "child1", "v", "bar"]), Some(&7i64.into()));
        assert_eq!(at(&data, &["c", "child2"]), None);
    }

    #[test]
    fn renaming_a_child_that_is_still_waiting_for_its_id() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        let child1 = root.claim_child(&mut fixture.cell.borrow_mut(), "child1", owner("owner1"));
        let child2 = root.claim_child(&mut fixture.cell.borrow_mut(), "child1", owner("owner2"));

        child2.rename(&mut fixture.cell.borrow_mut(), "foo");

        let data = fixture
            .serialized()
            .expect("the shuffle schedules serialization");
        assert_eq!(child1.restoration_id(&fixture.cell.borrow()), "child1");
        assert_eq!(child2.restoration_id(&fixture.cell.borrow()), "foo");
        assert_eq!(at(&data, &["c", "child1", "v", "foo"]), Some(&22i64.into()));
        assert_eq!(
            at(&data, &["c", "foo"]),
            Some(&RestorationMap::new().into())
        );
    }

    #[test]
    fn adopting_a_child_of_this_bucket_changes_nothing() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        let child = root.claim_child(&mut fixture.cell.borrow_mut(), "child1", owner("owner1"));

        root.adopt_child(&mut fixture.cell.borrow_mut(), child);
        assert!(fixture.serialized().is_none());
    }

    #[test]
    fn adopting_a_fresh_child_puts_it_in_the_hierarchy() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        let child = RestorationBucket::empty(
            &mut fixture.cell.borrow_mut(),
            "fresh-child",
            owner("owner1"),
        );
        assert!(!child.is_replacing(&fixture.cell.borrow()));

        root.adopt_child(&mut fixture.cell.borrow_mut(), child);
        child.write(&mut fixture.cell.borrow_mut(), "value", 22i64);

        let data = fixture
            .serialized()
            .expect("an adoption schedules serialization");
        assert_eq!(
            at(&data, &["c", "fresh-child", "v", "value"]),
            Some(&22i64.into())
        );
    }

    #[test]
    fn adopting_a_child_that_already_had_a_parent_moves_its_data() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        let child = root.claim_child(&mut fixture.cell.borrow_mut(), "child1", owner("owner1"));
        let child_of_child = child.claim_child(
            &mut fixture.cell.borrow_mut(),
            "childOfChild",
            owner("owner2"),
        );
        child_of_child.write(&mut fixture.cell.borrow_mut(), "foo", "bar");
        let data = fixture
            .serialized()
            .expect("a write schedules serialization");
        assert_eq!(
            at(&data, &["c", "child1", "c", "childOfChild", "v", "foo"]),
            Some(&"bar".into())
        );

        root.adopt_child(&mut fixture.cell.borrow_mut(), child_of_child);

        let data = fixture
            .serialized()
            .expect("an adoption schedules serialization");
        assert_eq!(at(&data, &["c", "child1", "c"]), None);
        assert_eq!(
            at(&data, &["c", "childOfChild", "v", "foo"]),
            Some(&"bar".into())
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn adopting_onto_a_used_id_reports_the_duplicate_unless_it_is_given_up() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        let child = root.claim_child(&mut fixture.cell.borrow_mut(), "child1", owner("owner1"));
        let child_of_child =
            child.claim_child(&mut fixture.cell.borrow_mut(), "child1", owner("owner2"));
        child_of_child.write(&mut fixture.cell.borrow_mut(), "foo", "bar");

        root.adopt_child(&mut fixture.cell.borrow_mut(), child_of_child);

        let error = catch_unwind(AssertUnwindSafe(|| fixture.serialized())).unwrap_err();
        assert!(
            panic_message(&error)
                .contains("Multiple owners claimed child RestorationBuckets with the same IDs."),
        );
    }

    #[test]
    fn adopting_onto_a_given_up_id_takes_it_over() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        let child = root.claim_child(&mut fixture.cell.borrow_mut(), "child1", owner("owner1"));
        let child_of_child =
            child.claim_child(&mut fixture.cell.borrow_mut(), "child1", owner("owner2"));
        child_of_child.write(&mut fixture.cell.borrow_mut(), "foo", "bar");
        fixture.serialize();

        root.adopt_child(&mut fixture.cell.borrow_mut(), child_of_child);
        child.dispose(&mut fixture.cell.borrow_mut());

        let data = fixture
            .serialized()
            .expect("the shuffle schedules serialization");
        assert_eq!(at(&data, &["c", "child1", "v", "foo"]), Some(&"bar".into()));
    }

    #[test]
    fn a_child_of_a_parentless_bucket_joins_the_manager_when_it_is_adopted() {
        let (mut fixture, root) = Fixture::restored(raw_data_set());
        let detached =
            RestorationBucket::empty(&mut fixture.cell.borrow_mut(), "detached", owner("owner1"));
        let child = detached.claim_child(&mut fixture.cell.borrow_mut(), "leaf", owner("owner2"));
        child.write(&mut fixture.cell.borrow_mut(), "foo", 3i64);
        assert!(
            fixture.serialized().is_none(),
            "a bucket outside the hierarchy has no manager to schedule with"
        );

        root.adopt_child(&mut fixture.cell.borrow_mut(), detached);

        let data = fixture
            .serialized()
            .expect("an adoption schedules serialization");
        assert_eq!(
            at(&data, &["c", "detached", "c", "leaf", "v", "foo"]),
            Some(&3i64.into())
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn a_bucket_cannot_be_used_after_it_is_disposed() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let bucket = RestorationBucket::empty(&mut app, "foo", None);
        bucket.dispose(&mut app);

        let error = catch_unwind(AssertUnwindSafe(|| bucket.read(&mut app, "foo"))).unwrap_err();
        assert!(
            panic_message(&error).contains("A RestorationBucket was used after being disposed."),
        );
    }

    #[test]
    fn disposing_the_manager_disposes_the_root_bucket() {
        let (fixture, root) = Fixture::restored(raw_data_set());
        fixture.manager.dispose(&mut fixture.cell.borrow_mut());
        assert!(!fixture.cell.borrow().contains(root));
    }

    #[test]
    fn a_bucket_reports_the_owner_it_was_claimed_with() {
        let (fixture, root) = Fixture::restored(raw_data_set());
        let child = root.claim_child(&mut fixture.cell.borrow_mut(), "child1", owner("owner1"));
        let described = format!("{:?}", fixture.cell.borrow().get(child));
        if cfg!(debug_assertions) {
            assert_eq!(
                described,
                "RestorationBucket(restorationId: child1, owner: \"owner1\")"
            );
        } else {
            assert_eq!(
                described,
                "RestorationBucket(restorationId: child1, owner: None)"
            );
        }
    }
}
