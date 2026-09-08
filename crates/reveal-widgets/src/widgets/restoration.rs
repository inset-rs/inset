//! Flutter counterpart: `widgets/restoration.dart`.
//!
//! The widget-side of state restoration: the scopes that carry a [`RestorationBucket`] down
//! the tree, the [`RestorableProperty`] a [`State`] stores its data in, and the
//! [`RestorationMixin`] that ties the two together.

use std::fmt::{self, Debug};
use std::rc::Rc;

use indexmap::IndexMap;
use reveal_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, HandleId, ListenableObject, Listener,
};
pub use reveal_services::RestorationBucket;
use reveal_services::{RestorationData, RestorationManager};

use crate::framework::{
    BuildContext, InheritedWidget, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};
use crate::widgets::basic::SizedBox;

/// The typed handle for an erased id. Free: nothing is looked up; `get` checks the slot.
fn resolve<T: 'static>(id: HandleId) -> Handle<T> {
    Handle::from_id(id)
}

// ---------------------------------------------------------------------------------------------
// RestorationScope

/// Creates a new scope for restoration IDs used by descendant widgets to claim
/// [`RestorationBucket`]s.
///
/// A restoration scope inserts a [`RestorationBucket`] into the widget tree, which descendant
/// widgets can access via [`RestorationScope::of`]. It is uncommon for descendants to directly
/// store data in this bucket. Instead, descendant widgets should consider storing their own
/// restoration data in a child bucket claimed with [`RestorationBucket::claim_child`] from the
/// bucket provided by this scope.
///
/// The bucket inserted into the widget tree by this scope has been claimed from the
/// surrounding [`RestorationScope`] using the provided
/// [`restoration_id`](Self::restoration_id). If the [`RestorationScope`] is moved to a
/// different part of the widget tree under a different [`RestorationScope`], the bucket owned
/// by this scope with all its children and the data contained in them is moved to the new
/// scope as well.
///
/// This widget will not make a [`RestorationBucket`] available to descendants if
/// [`restoration_id`](Self::restoration_id) is `None` or when there is no surrounding
/// restoration scope to claim a bucket from. In this case, descendant widgets invoking
/// [`RestorationScope::of`] will receive `None` as a return value indicating that no bucket is
/// available for storing restoration data. This will turn off state restoration for the widget
/// subtree.
///
/// See also:
///
///  * [`RootRestorationScope`], which inserts the root bucket provided by the
///    [`RestorationManager`] into the widget tree and makes it accessible for descendants via
///    [`RestorationScope::of`].
///  * [`UnmanagedRestorationScope`], which inserts a provided [`RestorationBucket`] into the
///    widget tree and makes it accessible for descendants via [`RestorationScope::of`].
///  * [`RestorationMixin`], which may be used in [`State`] objects to manage the restoration
///    data of a [`StatefulWidget`] instead of manually interacting with [`RestorationScope`]s
///    and [`RestorationBucket`]s.
///  * [`RestorationManager`], which describes the basic concepts of state restoration.
#[derive(Debug)]
pub struct RestorationScope {
    /// See [`StatefulWidget::key`].
    pub key: Option<KeyRef>,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
    /// The restoration ID used by this widget to obtain a child bucket from the surrounding
    /// [`RestorationScope`].
    ///
    /// The child bucket obtained from the surrounding scope is made available to descendant
    /// widgets via [`RestorationScope::of`].
    ///
    /// If this is `None`, [`RestorationScope::of`] invoked by descendants will return `None`,
    /// which effectively turns off state restoration for this subtree.
    pub restoration_id: Option<String>,
}

impl RestorationScope {
    /// Creates a [`RestorationScope`].
    ///
    /// Providing `None` as the `restoration_id` turns off state restoration for the `child`
    /// and its descendants.
    pub fn new<K>(restoration_id: Option<String>, child: impl IntoWidget<K>) -> RestorationScope {
        RestorationScope {
            key: None,
            child: child.into_widget(),
            restoration_id,
        }
    }

    /// Dart `RestorationScope(key:)`.
    pub fn key(mut self, key: KeyRef) -> RestorationScope {
        self.key = Some(key);
        self
    }

    /// Returns the [`RestorationBucket`] inserted into the widget tree by the closest ancestor
    /// [`RestorationScope`] of `context`.
    ///
    /// To avoid accidentally overwriting data already stored in the bucket by its owner, data
    /// should not be stored directly in the bucket returned by this method. Instead, consider
    /// claiming a child bucket from the returned bucket (via
    /// [`RestorationBucket::claim_child`]) and store the restoration data in that child.
    ///
    /// This method returns `None` if state restoration is turned off for this subtree.
    ///
    /// Calling this method will create a dependency on the closest [`RestorationScope`] in the
    /// `context`, if there is one.
    ///
    /// See also:
    ///
    ///  * [`RestorationScope::of`], which is similar to this method, but panics if no
    ///    [`RestorationScope`] ancestor is found.
    pub fn maybe_of(app: &mut App, context: BuildContext) -> Option<Handle<RestorationBucket>> {
        context
            .depend_on_inherited_widget_of_exact_type::<UnmanagedRestorationScope>(app)
            .and_then(|scope| scope.bucket)
    }

    /// Returns the [`RestorationBucket`] inserted into the widget tree by the closest ancestor
    /// [`RestorationScope`] of `context`.
    ///
    /// To avoid accidentally overwriting data already stored in the bucket by its owner, data
    /// should not be stored directly in the bucket returned by this method. Instead, consider
    /// claiming a child bucket from the returned bucket (via
    /// [`RestorationBucket::claim_child`]) and store the restoration data in that child.
    ///
    /// This method panics if state restoration is turned off for this subtree.
    ///
    /// Calling this method will create a dependency on the closest [`RestorationScope`] in the
    /// `context`.
    ///
    /// See also:
    ///
    ///  * [`RestorationScope::maybe_of`], which is similar to this method, but returns `None`
    ///    if no [`RestorationScope`] ancestor is found.
    pub fn of(app: &mut App, context: BuildContext) -> Handle<RestorationBucket> {
        Self::maybe_of(app, context).expect(
            "RestorationScope.of() was called with a context that does not contain a \
             RestorationScope widget.\n\
             No RestorationScope widget ancestor could be found starting from the context that \
             was passed to RestorationScope.of(). This can happen because you are using a \
             widget that looks for a RestorationScope ancestor, but no such ancestor exists.\n\
             State restoration must be enabled for a RestorationScope to exist. This can be \
             done by wrapping the widget tree in a RootRestorationScope.",
        )
    }
}

impl StatefulWidget for RestorationScope {
    type State = RestorationScopeState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> RestorationScopeState {
        RestorationScopeState {
            state: StateData::new(),
            restoration: RestorationMixinData::new(),
        }
    }
}

/// Dart's `_RestorationScopeState`.
pub struct RestorationScopeState {
    state: StateData<RestorationScope>,
    restoration: RestorationMixinData,
}

impl RestorationMixin for RestorationScopeState {
    crate::restoration_mixin_accessors!();

    fn restoration_id(self: Handle<Self>, app: &App) -> Option<&str> {
        self.widget(app).restoration_id.as_deref()
    }

    fn restore_state(
        self: Handle<Self>,
        app: &mut App,
        old_bucket: Option<Handle<RestorationBucket>>,
        initial_restore: bool,
    ) {
        // Nothing to do.
        // The bucket gets injected into the widget tree in the build method.
        let _ = (app, old_bucket, initial_restore);
    }
}

impl State for RestorationScopeState {
    type Widget = RestorationScope;
    crate::state_accessors!();

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        self.did_change_dependencies_restoration(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &RestorationScope) {
        self.did_update_restoration_id(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        self.dispose_restoration(app);
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        // `bucket` is provided by the RestorationMixin.
        let bucket = self.bucket(app);
        let child = self.widget(app).child.clone();
        UnmanagedRestorationScope::new(child)
            .bucket(bucket)
            .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// UnmanagedRestorationScope

/// Inserts a provided [`RestorationBucket`] into the widget tree and makes it available to
/// descendants via [`RestorationScope::of`].
///
/// A restoration scope inserts a [`RestorationBucket`] into the widget tree, which descendant
/// widgets can access via [`RestorationScope::of`]. It is uncommon for descendants to directly
/// store data in this bucket. Instead, descendant widgets should consider storing their own
/// restoration data in a child bucket claimed with [`RestorationBucket::claim_child`] from the
/// bucket provided by this scope.
///
/// If [`bucket`](Self::bucket) is `None`, no restoration bucket is made available to descendant
/// widgets ([`RestorationScope::of`] invoked from a descendant will return `None`). This
/// effectively turns off state restoration for the subtree because no bucket for storing
/// restoration data is made available.
///
/// See also:
///
///  * [`RestorationScope`], which inserts a bucket obtained from a surrounding restoration
///    scope into the widget tree and makes it accessible for descendants via
///    [`RestorationScope::of`].
///  * [`RootRestorationScope`], which inserts the root bucket provided by the
///    [`RestorationManager`] into the widget tree and makes it accessible for descendants via
///    [`RestorationScope::of`].
///  * [`RestorationMixin`], which may be used in [`State`] objects to manage the restoration
///    data of a [`StatefulWidget`] instead of manually interacting with [`RestorationScope`]s
///    and [`RestorationBucket`]s.
///  * [`RestorationManager`], which describes the basic concepts of state restoration.
#[derive(Debug)]
pub struct UnmanagedRestorationScope {
    /// See [`InheritedWidget::key`].
    pub key: Option<KeyRef>,
    /// The [`RestorationBucket`] that this widget will insert into the widget tree.
    ///
    /// Descendant widgets may obtain this bucket via [`RestorationScope::of`].
    pub bucket: Option<Handle<RestorationBucket>>,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl UnmanagedRestorationScope {
    /// Creates an [`UnmanagedRestorationScope`].
    ///
    /// When [`bucket`](Self::bucket) is `None` state restoration is turned off for the `child`
    /// and its descendants.
    pub fn new<K>(child: impl IntoWidget<K>) -> UnmanagedRestorationScope {
        UnmanagedRestorationScope {
            key: None,
            bucket: None,
            child: child.into_widget(),
        }
    }

    /// Dart `UnmanagedRestorationScope(key:)`.
    pub fn key(mut self, key: KeyRef) -> UnmanagedRestorationScope {
        self.key = Some(key);
        self
    }

    /// Dart `UnmanagedRestorationScope(bucket:)`; `None` turns off state restoration for the
    /// subtree.
    pub fn bucket(
        mut self,
        bucket: Option<Handle<RestorationBucket>>,
    ) -> UnmanagedRestorationScope {
        self.bucket = bucket;
        self
    }
}

impl InheritedWidget for UnmanagedRestorationScope {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old_widget: &UnmanagedRestorationScope) -> bool {
        old_widget.bucket != self.bucket
    }
}

// ---------------------------------------------------------------------------------------------
// RootRestorationScope

/// Inserts a child bucket of [`RestorationManager::root_bucket`] into the widget tree and makes
/// it available to descendants via [`RestorationScope::of`].
///
/// This widget is usually used near the root of the widget tree to enable the state restoration
/// functionality for the application. For all other use cases, consider using a regular
/// [`RestorationScope`] instead.
///
/// The exact behavior of this widget depends on its ancestors: When the [`RootRestorationScope`]
/// does not find an ancestor restoration bucket via [`RestorationScope::of`] it will claim a
/// child bucket from the root restoration bucket ([`RestorationManager::root_bucket`]) using the
/// provided [`restoration_id`](Self::restoration_id) and inserts that bucket into the widget
/// tree where descendants may access it via [`RestorationScope::of`]. If the
/// [`RootRestorationScope`] finds a non-`None` ancestor restoration bucket via
/// [`RestorationScope::of`] it will behave like a regular [`RestorationScope`] instead: It will
/// claim a child bucket from that ancestor and insert that child into the widget tree.
///
/// Unlike the [`RestorationScope`] widget, the [`RootRestorationScope`] will guarantee that
/// descendants have a bucket available for storing restoration data as long as
/// [`restoration_id`](Self::restoration_id) is not `None` and [`RestorationManager`] is able to
/// provide a root bucket. In other words, it will force-enable state restoration for the subtree
/// if [`restoration_id`](Self::restoration_id) is not `None`.
///
/// If [`restoration_id`](Self::restoration_id) is `None`, no bucket is made available to
/// descendants, which effectively turns off state restoration for this subtree.
///
/// See also:
///
///  * [`RestorationScope`], which inserts a bucket obtained from a surrounding restoration
///    scope into the widget tree and makes it accessible for descendants via
///    [`RestorationScope::of`].
///  * [`UnmanagedRestorationScope`], which inserts a provided [`RestorationBucket`] into the
///    widget tree and makes it accessible for descendants via [`RestorationScope::of`].
///  * [`RestorationMixin`], which may be used in [`State`] objects to manage the restoration
///    data of a [`StatefulWidget`] instead of manually interacting with [`RestorationScope`]s
///    and [`RestorationBucket`]s.
///  * [`RestorationManager`], which describes the basic concepts of state restoration.
#[derive(Debug)]
pub struct RootRestorationScope {
    /// See [`StatefulWidget::key`].
    pub key: Option<KeyRef>,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
    /// The restoration ID used to identify the child bucket that this widget will insert into
    /// the tree.
    ///
    /// If this is `None`, no bucket is made available to descendants and state restoration for
    /// the subtree is essentially turned off.
    pub restoration_id: Option<String>,
}

impl RootRestorationScope {
    /// Creates a [`RootRestorationScope`].
    ///
    /// Providing `None` as the `restoration_id` turns off state restoration for the `child` and
    /// its descendants.
    pub fn new<K>(
        restoration_id: Option<String>,
        child: impl IntoWidget<K>,
    ) -> RootRestorationScope {
        RootRestorationScope {
            key: None,
            child: child.into_widget(),
            restoration_id,
        }
    }

    /// Dart `RootRestorationScope(key:)`.
    pub fn key(mut self, key: KeyRef) -> RootRestorationScope {
        self.key = Some(key);
        self
    }
}

impl StatefulWidget for RootRestorationScope {
    type State = RootRestorationScopeState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> RootRestorationScopeState {
        RootRestorationScopeState {
            state: StateData::new(),
            ok_to_render_blank_container: None,
            root_bucket_valid: false,
            root_bucket: None,
            ancestor_bucket: None,
            is_loading_root_bucket: false,
        }
    }
}

/// Dart's `_RootRestorationScopeState`.
pub struct RootRestorationScopeState {
    state: StateData<RootRestorationScope>,
    ok_to_render_blank_container: Option<bool>,
    root_bucket_valid: bool,
    root_bucket: Option<Handle<RestorationBucket>>,
    ancestor_bucket: Option<Handle<RestorationBucket>>,
    is_loading_root_bucket: bool,
}

impl RootRestorationScopeState {
    fn needs_root_bucket_inserted(self: Handle<Self>, app: &App) -> bool {
        app.get(self).ancestor_bucket.is_none()
    }

    fn is_waiting_for_root_bucket(self: Handle<Self>, app: &App) -> bool {
        self.widget(app).restoration_id.is_some()
            && self.needs_root_bucket_inserted(app)
            && !app.get(self).root_bucket_valid
    }

    /// Dart's `_loadRootBucketIfNecessary`.
    ///
    /// [`RestorationManager::root_bucket`] answers within the call, so what Dart runs from the
    /// future's continuation runs here inline.
    fn load_root_bucket_if_necessary(self: Handle<Self>, app: &mut App) {
        if self.is_waiting_for_root_bucket(app) && !app.get(self).is_loading_root_bucket {
            app.get_mut(self).is_loading_root_bucket = true;
            let manager = RestorationManager::instance(app);
            let bucket = manager.root_bucket(app);
            app.get_mut(self).is_loading_root_bucket = false;
            if self.mounted(app) {
                manager.add_listener(
                    app,
                    Listener::handle_method(self, Self::replace_root_bucket),
                );
                self.set_state(app, |state| {
                    state.root_bucket = bucket;
                    state.root_bucket_valid = true;
                    state.ok_to_render_blank_container = Some(false);
                });
            }
        }
    }

    /// Dart's `_replaceRootBucket`: the [`RestorationManager`] listener that picks up newly
    /// provided restoration data.
    fn replace_root_bucket(self: Handle<Self>, app: &mut App) {
        app.get_mut(self).root_bucket_valid = false;
        app.get_mut(self).root_bucket = None;
        let manager = RestorationManager::instance(app);
        manager.remove_listener(
            app,
            &Listener::handle_method(self, Self::replace_root_bucket),
        );
        self.load_root_bucket_if_necessary(app);
        // Ensure that load finished synchronously.
        debug_assert!(!self.is_waiting_for_root_bucket(app));
    }
}

impl State for RootRestorationScopeState {
    type Widget = RootRestorationScope;
    crate::state_accessors!();

    fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
        let context = self.context(app);
        let ancestor_bucket = RestorationScope::maybe_of(app, context);
        app.get_mut(self).ancestor_bucket = ancestor_bucket;
        self.load_root_bucket_if_necessary(app);
        if app.get(self).ok_to_render_blank_container.is_none() {
            let ok_to_render_blank_container =
                self.widget(app).restoration_id.is_some() && self.needs_root_bucket_inserted(app);
            app.get_mut(self).ok_to_render_blank_container = Some(ok_to_render_blank_container);
        }
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &RootRestorationScope) {
        self.load_root_bucket_if_necessary(app);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if app.get(self).root_bucket_valid {
            let manager = RestorationManager::instance(app);
            manager.remove_listener(
                app,
                &Listener::handle_method(self, Self::replace_root_bucket),
            );
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let ok_to_render_blank_container = app
            .get(self)
            .ok_to_render_blank_container
            .expect("didChangeDependencies runs before the first build");
        if ok_to_render_blank_container && self.is_waiting_for_root_bucket(app) {
            return SizedBox::shrink().into_widget();
        }

        let this = app.get(self);
        let bucket = this.ancestor_bucket.or(this.root_bucket);
        let restoration_id = self.widget(app).restoration_id.clone();
        let child = self.widget(app).child.clone();
        UnmanagedRestorationScope::new(RestorationScope::new(restoration_id, child))
            .bucket(bucket)
            .into_widget()
    }
}

// ---------------------------------------------------------------------------------------------
// RestorableProperty

/// The framework's part of a [`RestorableProperty`]: the ID and owner it is registered under.
pub struct RestorablePropertyData {
    // ID under which the property has been registered with the RestorationMixin.
    restoration_id: Option<String>,
    owner: Option<AnyRestorationMixin>,
    disposed: bool,
}

impl RestorablePropertyData {
    /// A property that is not registered with any [`RestorationMixin`].
    pub fn new() -> RestorablePropertyData {
        RestorablePropertyData {
            restoration_id: None,
            owner: None,
            disposed: false,
        }
    }
}

impl Default for RestorablePropertyData {
    fn default() -> RestorablePropertyData {
        RestorablePropertyData::new()
    }
}

/// The accessors [`RestorableProperty`] asks for, for a struct whose bag is the field
/// `property`.
#[macro_export]
macro_rules! restorable_property_accessors {
    () => {
        fn restorable_property_data(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::RestorablePropertyData {
            &app.get(self).property
        }

        fn restorable_property_data_mut(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::RestorablePropertyData {
            &mut app.get_mut(self).property
        }
    };
}

/// Manages an object of type [`Value`](Self::Value), whose value a [`State`] object wants to
/// have restored during state restoration.
///
/// The property wraps an object of type [`Value`](Self::Value). It knows how to store its value
/// in the restoration data and it knows how to re-instantiate that object from the information
/// it previously stored in the restoration data.
///
/// The knowledge of how to store the wrapped object in the restoration data is encoded in the
/// [`to_primitives`](Self::to_primitives) method and the knowledge of how to re-instantiate the
/// object from that data is encoded in the [`from_primitives`](Self::from_primitives) method. At
/// a later point in time (which may be after the application restarted), the data obtained from
/// [`to_primitives`](Self::to_primitives) may be handed back to the property's
/// [`from_primitives`](Self::from_primitives) method to restore it to the previous state
/// described by that data.
///
/// A [`RestorableProperty`] needs to be registered to a [`RestorationMixin`] using a restoration
/// ID that is unique within the mixin. The [`RestorationMixin`] provides and manages the
/// [`RestorationBucket`], in which the data returned by [`to_primitives`](Self::to_primitives)
/// is stored.
///
/// Whenever the value returned by [`to_primitives`](Self::to_primitives) (or the
/// [`enabled`](Self::enabled) getter) changes, the [`RestorableProperty`] must call
/// `notify_listeners`. This will trigger the [`RestorationMixin`] to update the data it has
/// stored for the property in its [`RestorationBucket`] to the latest information returned by
/// [`to_primitives`](Self::to_primitives).
///
/// When the property is registered with the [`RestorationMixin`], the mixin checks whether there
/// is any restoration data available for the property. If data is available, the mixin calls
/// [`from_primitives`](Self::from_primitives) on the property, which must return an object that
/// matches the object the property wrapped when the provided restoration data was obtained from
/// [`to_primitives`](Self::to_primitives). If no restoration data is available to restore the
/// property's wrapped object from, the mixin calls
/// [`create_default_value`](Self::create_default_value). The value returned by either of those
/// methods is then handed to the property's [`init_with_value`](Self::init_with_value) method.
///
/// Usually, implementors of [`RestorableProperty`] hold on to the value provided to them in
/// [`init_with_value`](Self::init_with_value) and make it accessible to the [`State`] object
/// that owns the property. This base trait, however, has no opinion about what to do with the
/// value provided to [`init_with_value`](Self::init_with_value).
///
/// The [`RestorationMixin`] may call
/// [`from_primitives`](Self::from_primitives) / [`create_default_value`](Self::create_default_value)
/// followed by [`init_with_value`](Self::init_with_value) multiple times throughout the life of
/// a [`RestorableProperty`]: Whenever new restoration data is made available to the
/// [`RestorationMixin`] the property is registered with, the cycle repeats. Whenever
/// [`init_with_value`](Self::init_with_value) is called, the property should forget the old
/// value it was wrapping and re-initialize itself with the newly provided value.
///
/// In a typical use case, a [`RestorableProperty`] is instantiated to initialize a member
/// variable of a [`State`] object or within `State::init_state`. It is then registered to a
/// [`RestorationMixin`] in [`RestorationMixin::restore_state`] and later
/// [`dispose`](Self::dispose)d in `State::dispose`.
///
/// See also:
///
///  * [`RestorableValue`](crate::RestorableValue), which is a [`RestorableProperty`] that makes
///    the wrapped value accessible to the owning [`State`] object via a `value` getter and
///    setter.
///  * [`RestorationMixin`], to which a [`RestorableProperty`] must be registered.
///  * [`RestorationManager`], which describes how state restoration works.
pub trait RestorableProperty: ChangeNotifier + Sized + 'static {
    /// The object this property wraps; Dart's type argument `T`.
    type Value;

    /// The registration with a [`RestorationMixin`], held under the field `property`
    /// ([`restorable_property_accessors!`](crate::restorable_property_accessors)).
    fn restorable_property_data(self: Handle<Self>, app: &App) -> &RestorablePropertyData;

    /// See [`restorable_property_data`](Self::restorable_property_data).
    fn restorable_property_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut RestorablePropertyData;

    /// Called by the [`RestorationMixin`] if no restoration data is available to restore the
    /// value of the property from, to obtain the default value for the property.
    ///
    /// The method returns the default value that the property should wrap if no restoration
    /// data is available. After this is called, [`init_with_value`](Self::init_with_value) will
    /// be called with this method's return value.
    ///
    /// The method may be called multiple times throughout the life of the
    /// [`RestorableProperty`]. Whenever new restoration data has been provided to the
    /// [`RestorationMixin`] the property is registered to, either this method or
    /// [`from_primitives`](Self::from_primitives) is called before
    /// [`init_with_value`](Self::init_with_value) is invoked.
    fn create_default_value(self: Handle<Self>, app: &mut App) -> Self::Value;

    /// Called by the [`RestorationMixin`] to convert the `data` previously retrieved from
    /// [`to_primitives`](Self::to_primitives) back into an object of type
    /// [`Value`](Self::Value) that this property should wrap.
    ///
    /// The object returned by this method is passed to
    /// [`init_with_value`](Self::init_with_value) to restore the value that this property is
    /// wrapping to the value described by the provided `data`.
    ///
    /// The method may be called multiple times throughout the life of the
    /// [`RestorableProperty`]. Whenever new restoration data has been provided to the
    /// [`RestorationMixin`] the property is registered to, either this method or
    /// [`create_default_value`](Self::create_default_value) is called before
    /// [`init_with_value`](Self::init_with_value) is invoked.
    fn from_primitives(self: Handle<Self>, app: &mut App, data: &RestorationData) -> Self::Value;

    /// Called by the [`RestorationMixin`] with the `value` returned by either
    /// [`create_default_value`](Self::create_default_value) or
    /// [`from_primitives`](Self::from_primitives) to set the value that this property currently
    /// wraps.
    ///
    /// This method may be called multiple times throughout the life of the
    /// [`RestorableProperty`] whenever new restoration data has been provided to the
    /// [`RestorationMixin`] the property is registered to. When it is called, the property
    /// should forget its previous value and re-initialize itself to the newly provided `value`.
    fn init_with_value(self: Handle<Self>, app: &mut App, value: Self::Value);

    /// Called by the [`RestorationMixin`] to retrieve the information that this property wants
    /// to store in the restoration data.
    ///
    /// The information returned by this method may be handed back to the property in a call to
    /// [`from_primitives`](Self::from_primitives) at a later point in time (possibly after the
    /// application restarted) to restore the value that the property is currently wrapping.
    ///
    /// When the value returned by this method changes, the property must call
    /// `notify_listeners`. The [`RestorationMixin`] will invoke this method whenever the
    /// property's listeners are notified.
    fn to_primitives(self: Handle<Self>, app: &App) -> RestorationData;

    /// Whether the object currently returned by [`to_primitives`](Self::to_primitives) should
    /// be included in the restoration state.
    ///
    /// When this returns false, no information is included in the restoration data for this
    /// property and the property will be initialized to its default value (obtained from
    /// [`create_default_value`](Self::create_default_value)) the next time that restoration
    /// data is used for state restoration.
    ///
    /// Whenever the value returned by this getter changes, `notify_listeners` must be called.
    /// When the value changes from true to false, the information last retrieved from
    /// [`to_primitives`](Self::to_primitives) is removed from the restoration data. When it
    /// changes from false to true, [`to_primitives`](Self::to_primitives) is invoked to add the
    /// latest restoration information provided by this property to the restoration data.
    fn enabled(self: Handle<Self>, app: &App) -> bool {
        let _ = (self, app);
        true
    }

    /// Dart's `RestorableProperty.dispose` body. An override that runs `super.dispose()`
    /// calls this.
    fn dispose_property(self: Handle<Self>, app: &mut App) {
        debug_assert!(ChangeNotifierData::debug_assert_not_disposed(
            app.get(self).change_notifier_data()
        ));
        if let Some(owner) = self.restorable_property_data(app).owner {
            owner.unregister(app, self.as_property());
        }
        app.get_mut(self).change_notifier_data_mut().dispose();
        self.restorable_property_data_mut(app).disposed = true;
        self.did_dispose(app);
    }

    /// Discards any resources used by the property, unregistering it from its owner.
    fn dispose(self: Handle<Self>, app: &mut App) {
        self.dispose_property(app);
    }

    /// Runs at the end of [`dispose`](Self::dispose): what a Dart override does after its
    /// `super.dispose()` call.
    fn did_dispose(self: Handle<Self>, app: &mut App) {
        let _ = (self, app);
    }

    /// The [`State`] object that this property is registered with.
    ///
    /// Must only be called when [`is_registered`](Self::is_registered) is true.
    fn state(self: Handle<Self>, app: &App) -> AnyRestorationMixin {
        debug_assert!(self.is_registered(app));
        self.restorable_property_data(app)
            .owner
            .expect("a registered property has an owner")
    }

    /// Whether this property is currently registered with a [`RestorationMixin`].
    fn is_registered(self: Handle<Self>, app: &App) -> bool {
        debug_assert!(ChangeNotifierData::debug_assert_not_disposed(
            app.get(self).change_notifier_data()
        ));
        self.restorable_property_data(app).restoration_id.is_some()
    }

    /// This property as the erased [`AnyRestorableProperty`] — what to pass where a Dart API
    /// takes a `RestorableProperty<Object?>`.
    fn as_property(self: Handle<Self>) -> AnyRestorableProperty {
        AnyRestorableProperty {
            id: self.id(),
            vtable: const { &RestorablePropertyVTable::of::<Self>() },
        }
    }
}

/// The type-erased handle's slots for one [`RestorableProperty`] implementor.
struct RestorablePropertyVTable {
    type_name: fn() -> &'static str,
    data: fn(&App, HandleId) -> &RestorablePropertyData,
    data_mut: fn(&mut App, HandleId) -> &mut RestorablePropertyData,
    restore: fn(&mut App, HandleId, Option<&RestorationData>),
    to_primitives: fn(&App, HandleId) -> RestorationData,
    enabled: fn(&App, HandleId) -> bool,
    add_listener: fn(&mut App, HandleId, Listener),
    remove_listener: fn(&mut App, HandleId, &Listener),
}

impl RestorablePropertyVTable {
    /// The table for one property type.
    const fn of<P: RestorableProperty>() -> RestorablePropertyVTable {
        RestorablePropertyVTable {
            type_name: std::any::type_name::<P>,
            data: |app, id| P::restorable_property_data(resolve(id), app),
            data_mut: |app, id| P::restorable_property_data_mut(resolve(id), app),
            restore: |app, id, data| {
                let property: Handle<P> = resolve(id);
                let value = match data {
                    Some(data) => P::from_primitives(property, app, data),
                    None => P::create_default_value(property, app),
                };
                P::init_with_value(property, app, value);
            },
            to_primitives: |app, id| P::to_primitives(resolve(id), app),
            enabled: |app, id| P::enabled(resolve(id), app),
            add_listener: |app, id, listener| {
                ListenableObject::add_listener(resolve::<P>(id), app, listener)
            },
            remove_listener: |app, id, listener| {
                ListenableObject::remove_listener(resolve::<P>(id), app, listener)
            },
        }
    }
}

/// Erased [`RestorableProperty`]: one identity and a static vtable, the fat pointer rustc
/// cannot build for an arena id. No lease.
///
/// This is what Dart's `RestorableProperty<Object?>` becomes — the type a [`RestorationMixin`]
/// keeps its registered properties as. The property itself stays in the [`App`] under its own
/// type, and [`downcast`](Self::downcast) gets the typed handle back.
///
/// Equality is Dart's `==` on an object reference: two handles are equal exactly when they address
/// the same property.
#[derive(Clone, Copy)]
pub struct AnyRestorableProperty {
    id: HandleId,
    vtable: &'static RestorablePropertyVTable,
}

impl PartialEq for AnyRestorableProperty {
    fn eq(&self, other: &AnyRestorableProperty) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyRestorableProperty {}

impl std::hash::Hash for AnyRestorableProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Debug for AnyRestorableProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl AnyRestorableProperty {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// Dart's `property as T`: the typed handle when this property is a `T`, else `None`.
    pub fn downcast<T: 'static>(self, app: &App) -> Option<Handle<T>> {
        app.handle::<T>(self.id)
    }

    fn data(self, app: &App) -> &RestorablePropertyData {
        (self.vtable.data)(app, self.id)
    }

    fn data_mut(self, app: &mut App) -> &mut RestorablePropertyData {
        (self.vtable.data_mut)(app, self.id)
    }

    /// The ID under which the property is registered with its owner.
    pub fn restoration_id(self, app: &App) -> Option<&str> {
        self.data(app).restoration_id.as_deref()
    }

    /// The [`RestorationMixin`] this property is registered with; Dart's `_owner`.
    pub fn owner(self, app: &App) -> Option<AnyRestorationMixin> {
        self.data(app).owner
    }

    /// See [`RestorableProperty::is_registered`].
    pub fn is_registered(self, app: &App) -> bool {
        self.data(app).restoration_id.is_some()
    }

    /// Whether [`RestorableProperty::dispose`] has run.
    pub fn disposed(self, app: &App) -> bool {
        self.data(app).disposed
    }

    /// See [`RestorableProperty::enabled`].
    pub fn enabled(self, app: &App) -> bool {
        (self.vtable.enabled)(app, self.id)
    }

    /// See [`RestorableProperty::to_primitives`].
    pub fn to_primitives(self, app: &App) -> RestorationData {
        (self.vtable.to_primitives)(app, self.id)
    }

    /// Dart's `property.initWithValue(data != null ? property.fromPrimitives(data) : property.createDefaultValue())`.
    ///
    /// `None` is Dart's "no serialized value for this property"; a stored
    /// [`RestorationData::Null`] arrives as `Some`.
    pub fn restore(self, app: &mut App, data: Option<&RestorationData>) {
        (self.vtable.restore)(app, self.id, data);
    }

    /// See [`Listenable::add_listener`](reveal_foundation::Listenable::add_listener).
    pub fn add_listener(self, app: &mut App, listener: Listener) {
        (self.vtable.add_listener)(app, self.id, listener);
    }

    /// See [`Listenable::remove_listener`](reveal_foundation::Listenable::remove_listener).
    pub fn remove_listener(self, app: &mut App, listener: &Listener) {
        (self.vtable.remove_listener)(app, self.id, listener);
    }

    /// Dart's `RestorableProperty._register`.
    fn register(self, app: &mut App, restoration_id: &str, owner: AnyRestorationMixin) {
        let data = self.data_mut(app);
        data.restoration_id = Some(restoration_id.to_string());
        data.owner = Some(owner);
    }

    /// Dart's `RestorableProperty._unregister`.
    fn unregister(self, app: &mut App) {
        let data = self.data_mut(app);
        debug_assert!(data.restoration_id.is_some());
        debug_assert!(data.owner.is_some());
        data.restoration_id = None;
        data.owner = None;
    }
}

// ---------------------------------------------------------------------------------------------
// RestorationMixin

/// Dart's private members of `RestorationMixin`; a [`State`] using the mixin carries this bag
/// under the field `restoration`.
pub struct RestorationMixinData {
    bucket: Option<Handle<RestorationBucket>>,
    // Maps properties to their listeners.
    properties: IndexMap<AnyRestorableProperty, Listener>,
    debug_properties_waiting_for_reregistration: Option<Vec<AnyRestorableProperty>>,
    first_restore_pending: bool,
    current_parent: Option<Handle<RestorationBucket>>,
}

impl RestorationMixinData {
    /// A state that has not claimed a bucket yet.
    pub fn new() -> RestorationMixinData {
        RestorationMixinData {
            bucket: None,
            properties: IndexMap::new(),
            debug_properties_waiting_for_reregistration: None,
            first_restore_pending: true,
            current_parent: None,
        }
    }
}

impl Default for RestorationMixinData {
    fn default() -> RestorationMixinData {
        RestorationMixinData::new()
    }
}

/// The accessors [`RestorationMixin`] asks for, for a state whose bag is the field
/// `restoration`.
#[macro_export]
macro_rules! restoration_mixin_accessors {
    () => {
        fn restoration_mixin_data(
            self: ::reveal_foundation::Handle<Self>,
            app: &::reveal_foundation::App,
        ) -> &$crate::RestorationMixinData {
            &app.get(self).restoration
        }

        fn restoration_mixin_data_mut(
            self: ::reveal_foundation::Handle<Self>,
            app: &mut ::reveal_foundation::App,
        ) -> &mut $crate::RestorationMixinData {
            &mut app.get_mut(self).restoration
        }
    };
}

/// Manages the restoration data for a [`State`] object of a [`StatefulWidget`].
///
/// Restoration data can be serialized out and, at a later point in time, be used to restore the
/// stateful members in the [`State`] object to the same values they had when the data was
/// generated.
///
/// This trait organizes the restoration data of a [`State`] object in
/// [`RestorableProperty`]s. All the information that the [`State`] object wants to get restored
/// during state restoration needs to be saved in a [`RestorableProperty`]. For example, to
/// restore the count value in a counter app, that value should be stored in a member of type
/// [`RestorableInt`](crate::RestorableInt) instead of a plain `i64` member.
///
/// The mixin ensures that the current values of the [`RestorableProperty`]s are serialized as
/// part of the restoration state. It is up to the [`State`] to ensure that the data stored in
/// the properties is always up to date. When the widget is restored from previously generated
/// restoration data, the values of the [`RestorableProperty`]s are automatically restored to the
/// values they had when the restoration data was serialized out.
///
/// Within a [`State`] that uses this mixin, [`RestorableProperty`]s are usually instantiated to
/// initialize member variables. Users of the mixin must implement
/// [`restore_state`](Self::restore_state) and register their previously instantiated
/// [`RestorableProperty`]s in this method by calling
/// [`register_for_restoration`](Self::register_for_restoration). The mixin calls this method for
/// the first time right after `State::init_state`. After registration, the values stored in the
/// property have either been restored to their previous value or - if no restoration data for
/// restoring is available - they are initialized with a property-specific default value. At the
/// end of a [`State`] object's life cycle, all restorable properties must be disposed in
/// `State::dispose`.
///
/// In addition to being invoked right after `State::init_state`,
/// [`restore_state`](Self::restore_state) is invoked again when new restoration data has been
/// provided to the mixin. When this happens, the [`State`] object must re-register all
/// properties with [`register_for_restoration`](Self::register_for_restoration) again to restore
/// them to their previous values as described by the new restoration data. All initialization
/// logic that depends on the current value of a restorable property should be included in the
/// [`restore_state`](Self::restore_state) method to ensure it re-executes when the properties
/// are restored to a different value during the life time of the [`State`] object.
///
/// Internally, the mixin stores the restoration data from all registered properties in a
/// [`RestorationBucket`] claimed from the surrounding [`RestorationScope`] using the
/// [`State`]-provided [`restoration_id`](Self::restoration_id). The
/// [`restoration_id`](Self::restoration_id) must be unique in the surrounding
/// [`RestorationScope`]. State restoration is disabled for the [`State`] object using this mixin
/// if [`restoration_id`](Self::restoration_id) is `None` or when there is no surrounding
/// [`RestorationScope`]. In that case, the values of the registered properties will not be
/// restored during state restoration.
///
/// The [`RestorationBucket`] used to store the registered properties is available via the
/// [`bucket`](Self::bucket) getter. Interacting directly with the bucket is uncommon, but the
/// [`State`] object may make this bucket available for its descendants to claim child buckets
/// from. For that, the [`bucket`](Self::bucket) is injected into the widget tree in
/// `State::build` with the help of an [`UnmanagedRestorationScope`].
///
/// The [`bucket`](Self::bucket) getter returns `None` if state restoration is turned off. If
/// state restoration is turned on or off during the lifetime of the widget (e.g. because
/// [`restoration_id`](Self::restoration_id) changes from `None` to non-`None`) the value
/// returned by the getter will also change. The mixin calls
/// [`did_toggle_bucket`](Self::did_toggle_bucket) on itself to notify the [`State`] object about
/// this change. Implementing that method is not necessary as long as the [`State`] object does
/// not directly interact with the [`bucket`](Self::bucket).
///
/// Whenever the value returned by [`restoration_id`](Self::restoration_id) changes,
/// [`did_update_restoration_id`](Self::did_update_restoration_id) must be called.
///
/// The [`State`] using this trait calls
/// [`did_change_dependencies_restoration`](Self::did_change_dependencies_restoration) from its
/// `State::did_change_dependencies`,
/// [`did_update_restoration_id`](Self::did_update_restoration_id) from its
/// `State::did_update_widget`, and [`dispose_restoration`](Self::dispose_restoration) from its
/// `State::dispose`, where Dart's mixin body would run.
///
/// See also:
///
///  * [`RestorableProperty`], which is the base trait for all restoration properties managed by
///    this mixin.
///  * [`RestorationManager`], which describes how state restoration works.
///  * [`RestorationScope`], which creates a new namespace for restoration IDs in the widget
///    tree.
pub trait RestorationMixin: State + Sized + 'static {
    /// Dart's `RestorationMixin` fields, held under the field `restoration`
    /// ([`restoration_mixin_accessors!`](crate::restoration_mixin_accessors)).
    fn restoration_mixin_data(self: Handle<Self>, app: &App) -> &RestorationMixinData;

    /// See [`restoration_mixin_data`](Self::restoration_mixin_data).
    fn restoration_mixin_data_mut(self: Handle<Self>, app: &mut App) -> &mut RestorationMixinData;

    /// The restoration ID used for the [`RestorationBucket`] in which the mixin will store the
    /// restoration data of all registered properties.
    ///
    /// The restoration ID is used to claim a child bucket from the surrounding
    /// [`RestorationScope`] (accessed via [`RestorationScope::of`]) and the ID must be unique in
    /// that scope (otherwise the end of the frame panics in debug mode).
    ///
    /// State restoration for this mixin is turned off when this getter returns `None` or when
    /// there is no surrounding [`RestorationScope`] available. When state restoration is turned
    /// off, the values of the registered properties cannot be restored.
    ///
    /// Whenever the value returned by this getter changes,
    /// [`did_update_restoration_id`](Self::did_update_restoration_id) must be called.
    ///
    /// The restoration ID returned by this getter is often a field of the [`StatefulWidget`]
    /// that this [`State`] object is associated with.
    fn restoration_id(self: Handle<Self>, app: &App) -> Option<&str>;

    /// The [`RestorationBucket`] used for the restoration data of the [`RestorableProperty`]s
    /// registered to this mixin.
    ///
    /// The bucket has been claimed from the surrounding [`RestorationScope`] using
    /// [`restoration_id`](Self::restoration_id).
    ///
    /// The getter returns `None` if state restoration is turned off. When state restoration is
    /// turned on or off during the lifetime of this mixin (and hence the return value of this
    /// getter switches between `None` and non-`None`)
    /// [`did_toggle_bucket`](Self::did_toggle_bucket) is called.
    ///
    /// Interacting directly with this bucket is uncommon. However, the bucket may be injected
    /// into the widget tree in the [`State`]'s `build` method using an
    /// [`UnmanagedRestorationScope`]. That allows descendants to claim child buckets from this
    /// bucket for their own restoration needs.
    fn bucket(self: Handle<Self>, app: &App) -> Option<Handle<RestorationBucket>> {
        self.restoration_mixin_data(app).bucket
    }

    /// Called to initialize or restore the [`RestorableProperty`]s used by the [`State`] object.
    ///
    /// This method is always invoked at least once right after `State::init_state` to register
    /// the [`RestorableProperty`]s with the mixin even when state restoration is turned off or
    /// no restoration data is available for this [`State`] object.
    ///
    /// Typically, [`register_for_restoration`](Self::register_for_restoration) is called from
    /// this method to register all [`RestorableProperty`]s used by the [`State`] object with the
    /// mixin. The registration will either restore the property's value to the value described
    /// by the restoration data, if available, or, if no restoration data is available,
    /// initialize it to a property-specific default value.
    ///
    /// The method is called again whenever new restoration data (in the form of a new
    /// [`bucket`](Self::bucket)) has been provided to the mixin. When that happens, the
    /// [`State`] object must re-register all previously registered properties, which will
    /// restore their values to the value described by the new restoration data.
    ///
    /// Since the method may change the value of the registered properties when new restoration
    /// state is provided, all initialization logic that depends on a specific value of a
    /// [`RestorableProperty`] should be included in this method.
    ///
    /// The first time the method is invoked, the provided `old_bucket` argument is always
    /// `None`. In subsequent calls triggered by new restoration data in the form of a new
    /// bucket, the argument given is the previous value of [`bucket`](Self::bucket).
    fn restore_state(
        self: Handle<Self>,
        app: &mut App,
        old_bucket: Option<Handle<RestorationBucket>>,
        initial_restore: bool,
    );

    /// Called when [`bucket`](Self::bucket) switches between `None` and non-`None` values.
    ///
    /// [`State`] objects that wish to directly interact with the bucket may implement this
    /// method to store additional values in the bucket when one becomes available or to save
    /// values stored in a bucket elsewhere when the bucket goes away. This is uncommon and
    /// storing those values in [`RestorableProperty`]s should be considered instead.
    ///
    /// The `old_bucket` is provided to the method when the [`bucket`](Self::bucket) getter
    /// changes from non-`None` to `None`. The `old_bucket` argument is `None` when the
    /// [`bucket`](Self::bucket) changes from `None` to non-`None`.
    ///
    /// See also:
    ///
    ///  * [`restore_state`](Self::restore_state), which is called when the
    ///    [`bucket`](Self::bucket) changes from one non-`None` value to another non-`None`
    ///    value.
    fn did_toggle_bucket(
        self: Handle<Self>,
        app: &mut App,
        old_bucket: Option<Handle<RestorationBucket>>,
    ) {
        let _ = old_bucket;
        // When a bucket is replaced, `restore_state` is called instead.
        debug_assert!(
            self.bucket(app)
                .is_none_or(|bucket| !bucket.is_replacing(app))
        );
    }

    /// Registers a [`RestorableProperty`] for state restoration.
    ///
    /// The registration associates the provided `property` with the provided `restoration_id`.
    /// If restoration data is available for the provided `restoration_id`, the property's value
    /// is restored to the value described by the restoration data. If no restoration data is
    /// available, the property will be initialized to a property-specific default value.
    ///
    /// Each property within a [`State`] object must be registered under a unique ID. Only
    /// registered properties will have their values restored during state restoration.
    ///
    /// Typically, this method is called from within [`restore_state`](Self::restore_state) to
    /// register all restorable properties of the owning [`State`] object. However, if a given
    /// [`RestorableProperty`] is only needed when certain conditions are met within the
    /// [`State`], this method may also be called at any time after
    /// [`restore_state`](Self::restore_state) has been invoked for the first time.
    ///
    /// A property that has been registered outside of [`restore_state`](Self::restore_state)
    /// must be re-registered within [`restore_state`](Self::restore_state) the next time that
    /// method is called unless it has been unregistered with
    /// [`unregister_from_restoration`](Self::unregister_from_restoration).
    fn register_for_restoration(
        self: Handle<Self>,
        app: &mut App,
        property: AnyRestorableProperty,
        restoration_id: &str,
    ) {
        if cfg!(debug_assertions) {
            let doing_restore = self.debug_doing_restore(app);
            let registered_id = property.restoration_id(app).map(str::to_string);
            assert!(
                registered_id.is_none()
                    || (doing_restore && registered_id.as_deref() == Some(restoration_id)),
                "Property is already registered under {registered_id:?}."
            );
            let id_taken = self
                .restoration_mixin_data(app)
                .properties
                .keys()
                .any(|other| other.restoration_id(app) == Some(restoration_id));
            assert!(
                doing_restore || !id_taken,
                "\"{restoration_id}\" is already registered to another property."
            );
        }

        let has_serialized_value = self
            .bucket(app)
            .is_some_and(|bucket| bucket.contains(app, restoration_id));
        let serialized_value = has_serialized_value
            .then(|| {
                self.bucket(app)
                    .expect("a serialized value implies a bucket")
                    .read(app, restoration_id)
            })
            .flatten();

        if !property.is_registered(app) {
            property.register(app, restoration_id, self.as_restoration_mixin());
            let listener = Listener::new(move |app| {
                if self.bucket(app).is_none() {
                    return;
                }
                self.update_property(app, property);
            });
            property.add_listener(app, listener.clone());
            self.restoration_mixin_data_mut(app)
                .properties
                .insert(property, listener);
        }

        debug_assert!(
            property.restoration_id(app) == Some(restoration_id)
                && property.owner(app) == Some(self.as_restoration_mixin())
                && self
                    .restoration_mixin_data(app)
                    .properties
                    .contains_key(&property)
        );

        property.restore(app, serialized_value.as_ref());
        if !has_serialized_value && property.enabled(app) && self.bucket(app).is_some() {
            self.update_property(app, property);
        }

        if cfg!(debug_assertions)
            && let Some(waiting) = self
                .restoration_mixin_data_mut(app)
                .debug_properties_waiting_for_reregistration
                .as_mut()
        {
            waiting.retain(|other| *other != property);
        }
    }

    /// Unregisters a [`RestorableProperty`] from state restoration.
    ///
    /// The value of the `property` is removed from the restoration data and it will not be
    /// restored if that data is used in a future state restoration.
    ///
    /// Calling this method is uncommon, but may be necessary if the data of a
    /// [`RestorableProperty`] is only relevant when the [`State`] object is in a certain state.
    fn unregister_from_restoration(
        self: Handle<Self>,
        app: &mut App,
        property: AnyRestorableProperty,
    ) {
        debug_assert!(property.owner(app) == Some(self.as_restoration_mixin()));
        let restoration_id = property
            .restoration_id(app)
            .expect("a registered property has an ID")
            .to_string();
        if let Some(bucket) = self.restoration_mixin_data(app).bucket {
            bucket.remove(app, &restoration_id);
        }
        self.unregister(app, property);
    }

    /// Must be called when the value returned by [`restoration_id`](Self::restoration_id)
    /// changes.
    ///
    /// The [`State`] calls this from its `State::did_update_widget`, where Dart's mixin body
    /// would run; a change in [`restoration_id`](Self::restoration_id) that no widget update
    /// caused must call it directly.
    fn did_update_restoration_id(self: Handle<Self>, app: &mut App) {
        // There's nothing to do if:
        //  - We don't have a parent to claim a bucket from.
        //  - Our current bucket already uses the provided restoration ID.
        //  - There's a restore pending, which means that did_change_dependencies will be
        //    called and we handle the rename there.
        let bucket_keeps_id =
            self.bucket(app).map(|bucket| bucket.restoration_id(app)) == self.restoration_id(app);
        if self.restoration_mixin_data(app).current_parent.is_none()
            || bucket_keeps_id
            || self.restore_pending(app)
        {
            return;
        }

        let old_bucket = self.bucket(app);
        let current_parent = self.restoration_mixin_data(app).current_parent;
        let did_replace_bucket = self.update_bucket_if_necessary(app, current_parent, false);
        if did_replace_bucket {
            debug_assert!(old_bucket != self.bucket(app));
            debug_assert!(self.bucket(app).is_none() || old_bucket.is_none());
            if let Some(old_bucket) = old_bucket {
                old_bucket.dispose(app);
            }
        }
    }

    /// Whether [`restore_state`](Self::restore_state) will be called at the beginning of the
    /// next build phase.
    ///
    /// Returns true when new restoration data has been provided to the mixin, but the registered
    /// [`RestorableProperty`]s have not been restored to their new values (as described by the
    /// new restoration data) yet. The properties will get the values restored when
    /// [`restore_state`](Self::restore_state) is invoked at the beginning of the next build
    /// cycle.
    ///
    /// While this is true, [`bucket`](Self::bucket) will also still return the old bucket with
    /// the old restoration data. It will update to the new bucket with the new data just before
    /// [`restore_state`](Self::restore_state) is invoked.
    fn restore_pending(self: Handle<Self>, app: &mut App) -> bool {
        if self.restoration_mixin_data(app).first_restore_pending {
            return true;
        }
        if self.restoration_id(app).is_none() {
            return false;
        }
        let context = self.context(app);
        let potential_new_parent = RestorationScope::maybe_of(app, context);
        potential_new_parent != self.restoration_mixin_data(app).current_parent
            && potential_new_parent.is_some_and(|parent| parent.is_replacing(app))
    }

    /// Dart's `RestorationMixin.didChangeDependencies`, called by the [`State`] from its own
    /// `State::did_change_dependencies`.
    fn did_change_dependencies_restoration(self: Handle<Self>, app: &mut App) {
        let old_bucket = self.bucket(app);
        let needs_restore = self.restore_pending(app);
        let context = self.context(app);
        let current_parent = RestorationScope::maybe_of(app, context);
        self.restoration_mixin_data_mut(app).current_parent = current_parent;

        let did_replace_bucket =
            self.update_bucket_if_necessary(app, current_parent, needs_restore);

        if needs_restore {
            self.do_restore(app, old_bucket);
        }
        if did_replace_bucket {
            debug_assert!(old_bucket != self.bucket(app));
            if let Some(old_bucket) = old_bucket {
                old_bucket.dispose(app);
            }
        }
    }

    /// Dart's `RestorationMixin.dispose`, called by the [`State`] from its own `State::dispose`.
    fn dispose_restoration(self: Handle<Self>, app: &mut App) {
        let properties: Vec<(AnyRestorableProperty, Listener)> = self
            .restoration_mixin_data(app)
            .properties
            .iter()
            .map(|(property, listener)| (*property, listener.clone()))
            .collect();
        for (property, listener) in properties {
            if !property.disposed(app) {
                property.remove_listener(app, &listener);
            }
        }
        if let Some(bucket) = self.restoration_mixin_data(app).bucket {
            bucket.dispose(app);
        }
        self.restoration_mixin_data_mut(app).bucket = None;
    }

    /// This state as the erased [`AnyRestorationMixin`] — what a [`RestorableProperty`] keeps as
    /// its owner.
    fn as_restoration_mixin(self: Handle<Self>) -> AnyRestorationMixin {
        AnyRestorationMixin {
            id: self.id(),
            vtable: const { &RestorationMixinVTable::of::<Self>() },
        }
    }

    /// Dart's `_debugDoingRestore`.
    fn debug_doing_restore(self: Handle<Self>, app: &App) -> bool {
        self.restoration_mixin_data(app)
            .debug_properties_waiting_for_reregistration
            .is_some()
    }

    /// Dart's `_doRestore`.
    fn do_restore(
        self: Handle<Self>,
        app: &mut App,
        old_bucket: Option<Handle<RestorationBucket>>,
    ) {
        if cfg!(debug_assertions) {
            let registered: Vec<AnyRestorableProperty> = self
                .restoration_mixin_data(app)
                .properties
                .keys()
                .copied()
                .collect();
            self.restoration_mixin_data_mut(app)
                .debug_properties_waiting_for_reregistration = Some(registered);
        }

        let initial_restore = self.restoration_mixin_data(app).first_restore_pending;
        self.restore_state(app, old_bucket, initial_restore);
        self.restoration_mixin_data_mut(app).first_restore_pending = false;

        if cfg!(debug_assertions) {
            let waiting = self
                .restoration_mixin_data_mut(app)
                .debug_properties_waiting_for_reregistration
                .take()
                .expect("set just above");
            let missing: Vec<String> = waiting
                .iter()
                .map(|property| format!(" * {:?}", property.restoration_id(app)))
                .collect();
            assert!(
                missing.is_empty(),
                "Previously registered RestorableProperties must be re-registered in \
                 \"restore_state\". The RestorableProperties with the following IDs were not \
                 re-registered when \"restore_state\" was called:\n{}",
                missing.join("\n")
            );
        }
    }

    /// Dart's `_updateBucketIfNecessary`. Returns true if [`bucket`](Self::bucket) has been
    /// replaced with a new bucket; it is the responsibility of the caller to dispose the old
    /// bucket when this returns true.
    fn update_bucket_if_necessary(
        self: Handle<Self>,
        app: &mut App,
        parent: Option<Handle<RestorationBucket>>,
        restore_pending: bool,
    ) -> bool {
        let restoration_id = self.restoration_id(app).map(str::to_string);
        let (Some(restoration_id), Some(parent)) = (restoration_id, parent) else {
            let did_replace = self.set_new_bucket_if_necessary(app, None, restore_pending);
            debug_assert!(self.bucket(app).is_none());
            return did_replace;
        };

        if restore_pending || self.bucket(app).is_none() {
            let debug_owner: Option<Rc<dyn Debug>> = cfg!(debug_assertions)
                .then(|| Rc::new(self.as_restoration_mixin()) as Rc<dyn Debug>);
            let new_bucket = parent.claim_child(app, &restoration_id, debug_owner);
            let did_replace =
                self.set_new_bucket_if_necessary(app, Some(new_bucket), restore_pending);
            debug_assert!(self.bucket(app) == Some(new_bucket));
            return did_replace;
        }

        // We have an existing bucket, make sure it has the right parent and id.
        debug_assert!(!restore_pending);
        let bucket = self.bucket(app).expect("checked just above");
        bucket.rename(app, &restoration_id);
        parent.adopt_child(app, bucket);
        false
    }

    /// Dart's `_setNewBucketIfNecessary`. Returns true if [`bucket`](Self::bucket) has been
    /// replaced with a new bucket; it is the responsibility of the caller to dispose the old
    /// bucket when this returns true.
    fn set_new_bucket_if_necessary(
        self: Handle<Self>,
        app: &mut App,
        new_bucket: Option<Handle<RestorationBucket>>,
        restore_pending: bool,
    ) -> bool {
        if new_bucket == self.bucket(app) {
            return false;
        }
        let old_bucket = self.bucket(app);
        self.restoration_mixin_data_mut(app).bucket = new_bucket;
        if !restore_pending {
            // Write the current property values into the new bucket to persist them.
            if self.bucket(app).is_some() {
                let properties: Vec<AnyRestorableProperty> = self
                    .restoration_mixin_data(app)
                    .properties
                    .keys()
                    .copied()
                    .collect();
                for property in properties {
                    self.update_property(app, property);
                }
            }
            self.did_toggle_bucket(app, old_bucket);
        }
        true
    }

    /// Dart's `_updateProperty`.
    fn update_property(self: Handle<Self>, app: &mut App, property: AnyRestorableProperty) {
        let Some(bucket) = self.bucket(app) else {
            return;
        };
        let restoration_id = property
            .restoration_id(app)
            .expect("a registered property has an ID")
            .to_string();
        if property.enabled(app) {
            let value = property.to_primitives(app);
            bucket.write(app, &restoration_id, value);
        } else {
            bucket.remove(app, &restoration_id);
        }
    }

    /// Dart's `RestorationMixin._unregister`.
    fn unregister(self: Handle<Self>, app: &mut App, property: AnyRestorableProperty) {
        let listener = self
            .restoration_mixin_data_mut(app)
            .properties
            .shift_remove(&property)
            .expect("the property is registered with this mixin");
        if cfg!(debug_assertions)
            && let Some(waiting) = self
                .restoration_mixin_data_mut(app)
                .debug_properties_waiting_for_reregistration
                .as_mut()
        {
            waiting.retain(|other| *other != property);
        }
        property.remove_listener(app, &listener);
        property.unregister(app);
    }
}

/// The type-erased handle's slots for one [`RestorationMixin`] implementor.
struct RestorationMixinVTable {
    type_name: fn() -> &'static str,
    unregister: fn(&mut App, HandleId, AnyRestorableProperty),
    bucket: fn(&App, HandleId) -> Option<Handle<RestorationBucket>>,
    context: fn(&App, HandleId) -> BuildContext,
    mounted: fn(&App, HandleId) -> bool,
}

impl RestorationMixinVTable {
    /// The table for one state type.
    const fn of<M: RestorationMixin>() -> RestorationMixinVTable {
        RestorationMixinVTable {
            type_name: std::any::type_name::<M>,
            unregister: |app, id, property| M::unregister(resolve(id), app, property),
            bucket: |app, id| M::bucket(resolve(id), app),
            context: |app, id| M::context(resolve(id), app),
            mounted: |app, id| M::mounted(resolve(id), app),
        }
    }
}

/// Erased [`RestorationMixin`]: the [`State`] a [`RestorableProperty`] is registered with.
///
/// This is what Dart's `RestorationMixin? _owner` and `State get state` become; the state itself
/// stays in the [`App`] under its own type, and [`downcast`](Self::downcast) gets the typed
/// handle back.
///
/// Equality is Dart's `==` on an object reference: two handles are equal exactly when they address
/// the same state.
#[derive(Clone, Copy)]
pub struct AnyRestorationMixin {
    id: HandleId,
    vtable: &'static RestorationMixinVTable,
}

impl PartialEq for AnyRestorationMixin {
    fn eq(&self, other: &AnyRestorationMixin) -> bool {
        self.id == other.id
    }
}

impl Eq for AnyRestorationMixin {}

impl std::hash::Hash for AnyRestorationMixin {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Debug for AnyRestorationMixin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({:?})", (self.vtable.type_name)(), self.id)
    }
}

impl AnyRestorationMixin {
    /// The arena id behind this handle.
    pub fn id(self) -> HandleId {
        self.id
    }

    /// Dart's `state as T`: the typed handle when this state is a `T`, else `None`.
    pub fn downcast<T: 'static>(self, app: &App) -> Option<Handle<T>> {
        app.handle::<T>(self.id)
    }

    /// See [`RestorationMixin::bucket`].
    pub fn bucket(self, app: &App) -> Option<Handle<RestorationBucket>> {
        (self.vtable.bucket)(app, self.id)
    }

    /// See `State::context`.
    pub fn context(self, app: &App) -> BuildContext {
        (self.vtable.context)(app, self.id)
    }

    /// See `State::mounted`.
    pub fn mounted(self, app: &App) -> bool {
        (self.vtable.mounted)(app, self.id)
    }

    fn unregister(self, app: &mut App, property: AnyRestorableProperty) {
        (self.vtable.unregister)(app, self.id, property);
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use std::cell::RefCell;
    use std::time::{Duration, Instant};

    use reveal_embedder::{InertPlatform, Platform, PlatformRef, TargetPlatform, ViewId, ViewRef};
    use reveal_scheduler::SchedulerBinding;
    use reveal_services::{RestorationMap, RestorationUpdate};

    use super::*;
    use crate::framework::{Element, GlobalKey, StatelessWidget};
    use crate::test_harness::Harness;
    use crate::widgets::restoration_properties::{RestorableInt, RestorableValue};

    /// A host that answers `restoration_get` with what it was handed and records every
    /// `restoration_put`.
    #[derive(Default)]
    struct RecordingPlatform {
        stored: RefCell<Option<RestorationUpdate>>,
        puts: RefCell<Vec<RestorationMap>>,
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

        fn restoration_get(&self) -> Option<RestorationUpdate> {
            self.stored.borrow().clone()
        }

        fn restoration_put(&self, data: RestorationMap) {
            self.puts.borrow_mut().push(data);
        }
    }

    /// An [`App`] whose host restores the provided bucket hierarchy.
    fn app_restoring(data: Option<RestorationMap>) -> (Rc<AppCell>, Rc<RecordingPlatform>) {
        let platform = Rc::new(RecordingPlatform::default());
        *platform.stored.borrow_mut() = Some(RestorationUpdate {
            enabled: true,
            data,
        });
        let cell = AppCell::with_platform(Rc::clone(&platform) as PlatformRef);
        (cell, platform)
    }

    fn map<const N: usize>(entries: [(&str, RestorationData); N]) -> RestorationMap {
        entries
            .into_iter()
            .map(|(key, value)| (RestorationData::from(key), value))
            .collect()
    }

    /// The raw map of a bucket that stores these values.
    fn values<const N: usize>(entries: [(&str, RestorationData); N]) -> RestorationMap {
        map([("v", map(entries).into())])
    }

    /// The raw map of a bucket whose only content is one child.
    fn child(restoration_id: &str, data: RestorationMap) -> RestorationMap {
        map([("c", map([(restoration_id, data.into())]).into())])
    }

    /// The value stored down `path`, e.g. `["c", "app", "v", "count"]`.
    fn at<'a>(data: &'a RestorationMap, path: &[&str]) -> Option<&'a RestorationData> {
        let (last, parents) = path.split_last()?;
        let mut map = data;
        for key in parents {
            map = map.get(&RestorationData::from(*key))?.as_map()?;
        }
        map.get(&RestorationData::from(*last))
    }

    /// What a [`BucketProbe`] saw, in build order.
    type SeenBuckets = Rc<RefCell<Vec<Option<Handle<RestorationBucket>>>>>;

    /// Reads [`RestorationScope::maybe_of`] on every build and logs the bucket it saw.
    #[derive(Debug)]
    struct BucketProbe {
        seen: SeenBuckets,
    }

    impl StatelessWidget for BucketProbe {
        fn build(&self, app: &mut App, context: BuildContext) -> WidgetRef {
            let bucket = RestorationScope::maybe_of(app, context);
            self.seen.borrow_mut().push(bucket);
            SizedBox::shrink().into_widget()
        }
    }

    fn probe() -> (WidgetRef, SeenBuckets) {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let probe = BucketProbe {
            seen: Rc::clone(&seen),
        }
        .into_widget();
        (probe, seen)
    }

    /// A `State` that keeps its count in a [`RestorableInt`].
    #[derive(Debug)]
    struct Counter {
        key: Option<KeyRef>,
        restoration_id: Option<String>,
    }

    impl StatefulWidget for Counter {
        type State = CounterState;

        fn key(&self) -> Option<&KeyRef> {
            self.key.as_ref()
        }

        fn create_state(&self) -> CounterState {
            CounterState {
                state: StateData::new(),
                restoration: RestorationMixinData::new(),
                count: None,
                restores: 0,
            }
        }
    }

    struct CounterState {
        state: StateData<Counter>,
        restoration: RestorationMixinData,
        count: Option<Handle<RestorableInt>>,
        restores: usize,
    }

    impl CounterState {
        fn count(self: Handle<Self>, app: &App) -> Handle<RestorableInt> {
            app.get(self).count.expect("restore_state ran")
        }
    }

    impl RestorationMixin for CounterState {
        crate::restoration_mixin_accessors!();

        fn restoration_id(self: Handle<Self>, app: &App) -> Option<&str> {
            self.widget(app).restoration_id.as_deref()
        }

        fn restore_state(
            self: Handle<Self>,
            app: &mut App,
            _old_bucket: Option<Handle<RestorationBucket>>,
            _initial_restore: bool,
        ) {
            app.get_mut(self).restores += 1;
            let count = match app.get(self).count {
                Some(count) => count,
                None => {
                    let count = RestorableInt::new(app, 0);
                    app.get_mut(self).count = Some(count);
                    count
                }
            };
            self.register_for_restoration(app, count.as_property(), "count");
        }
    }

    impl State for CounterState {
        type Widget = Counter;
        crate::state_accessors!();

        fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
            self.did_change_dependencies_restoration(app);
        }

        fn did_update_widget(self: Handle<Self>, app: &mut App, _old_widget: &Counter) {
            self.did_update_restoration_id(app);
        }

        fn dispose(self: Handle<Self>, app: &mut App) {
            self.dispose_restoration(app);
        }

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            let _ = app;
            SizedBox::shrink().into_widget()
        }
    }

    fn mount(app: &mut App, child: WidgetRef) -> Harness {
        let harness = Harness::mount(app, child);
        harness.pump(app);
        harness
    }

    /// A frame, so the manager's post-frame serialization runs.
    fn pump_frame(app: &mut App) {
        SchedulerBinding::handle_begin_frame(app, Some(Duration::ZERO));
        app.drain_microtasks();
        SchedulerBinding::handle_draw_frame(app);
        app.drain_microtasks();
    }

    fn counter_state(harness: &Harness, app: &App, key: &GlobalKey) -> Handle<CounterState> {
        harness
            .owner
            .global_key_element(app, key.identity())
            .expect("the counter is in the tree")
            .state_handle::<CounterState>(app)
            .expect("a CounterState")
    }

    #[test]
    fn the_root_scope_hands_the_hosts_root_bucket_down() {
        let (cell, _platform) =
            app_restoring(Some(child("app", values([("count", 42i64.into())]))));
        let (probe, seen) = probe();
        let mut app = cell.borrow_mut();
        mount(
            &mut app,
            RootRestorationScope::new(Some("app".to_string()), probe).into_widget(),
        );

        let bucket = seen.borrow()[0].expect("the host enabled restoration");
        assert_eq!(bucket.restoration_id(&app), "app");
        assert_eq!(bucket.read(&mut app, "count"), Some(42i64.into()));
    }

    #[test]
    fn the_root_scope_hands_nothing_down_without_a_restoration_id() {
        let (cell, _platform) = app_restoring(None);
        let mut app = cell.borrow_mut();
        let (probe, seen) = probe();
        mount(
            &mut app,
            RootRestorationScope::new(None, probe).into_widget(),
        );

        assert_eq!(seen.borrow().as_slice(), [None]);
    }

    #[test]
    fn a_restoration_scope_claims_the_child_bucket_named_by_its_id() {
        let (cell, _platform) = app_restoring(Some(child(
            "app",
            child("greeting", values([("hello", "world".into())])),
        )));
        let mut app = cell.borrow_mut();
        let (probe, seen) = probe();
        mount(
            &mut app,
            RootRestorationScope::new(
                Some("app".to_string()),
                RestorationScope::new(Some("greeting".to_string()), probe),
            )
            .into_widget(),
        );

        let bucket = seen.borrow()[0].expect("the scope claimed a bucket");
        assert_eq!(bucket.restoration_id(&app), "greeting");
        assert_eq!(bucket.read(&mut app, "hello"), Some("world".into()));
    }

    #[test]
    fn a_restoration_scope_without_an_id_turns_restoration_off_for_its_subtree() {
        let (cell, _platform) = app_restoring(None);
        let mut app = cell.borrow_mut();
        let (probe, seen) = probe();
        mount(
            &mut app,
            RootRestorationScope::new(Some("app".to_string()), RestorationScope::new(None, probe))
                .into_widget(),
        );

        assert_eq!(seen.borrow().as_slice(), [None]);
    }

    #[test]
    fn a_registered_property_is_restored_from_the_hosts_data() {
        let (cell, _platform) = app_restoring(Some(child(
            "app",
            child("counter", values([("count", 7i64.into())])),
        )));
        let mut app = cell.borrow_mut();
        let key = GlobalKey::new();
        let harness = mount(
            &mut app,
            RootRestorationScope::new(
                Some("app".to_string()),
                Counter {
                    key: Some(Rc::new(key.clone())),
                    restoration_id: Some("counter".to_string()),
                },
            )
            .into_widget(),
        );

        let state = counter_state(&harness, &app, &key);
        assert_eq!(app.get(state).restores, 1);
        let count = state.count(&app);
        assert_eq!(*count.value(&app), 7);
    }

    #[test]
    fn a_property_with_no_stored_value_takes_its_default_and_writes_it_out() {
        let (cell, platform) = app_restoring(None);
        let mut app = cell.borrow_mut();
        let key = GlobalKey::new();
        let harness = mount(
            &mut app,
            RootRestorationScope::new(
                Some("app".to_string()),
                Counter {
                    key: Some(Rc::new(key.clone())),
                    restoration_id: Some("counter".to_string()),
                },
            )
            .into_widget(),
        );

        let count = counter_state(&harness, &app, &key).count(&app);
        assert_eq!(*count.value(&app), 0);

        pump_frame(&mut app);
        let sent = platform.puts.borrow().last().cloned().expect("serialized");
        assert_eq!(
            at(&sent, &["c", "app", "c", "counter", "v", "count"]),
            Some(&0i64.into())
        );
    }

    #[test]
    fn a_changed_property_reaches_the_host_at_the_end_of_the_frame() {
        let (cell, platform) = app_restoring(Some(child(
            "app",
            child("counter", values([("count", 7i64.into())])),
        )));
        let mut app = cell.borrow_mut();
        let key = GlobalKey::new();
        let harness = mount(
            &mut app,
            RootRestorationScope::new(
                Some("app".to_string()),
                Counter {
                    key: Some(Rc::new(key.clone())),
                    restoration_id: Some("counter".to_string()),
                },
            )
            .into_widget(),
        );
        pump_frame(&mut app);
        platform.puts.borrow_mut().clear();

        let count = counter_state(&harness, &app, &key).count(&app);
        count.set_value(&mut app, 9);
        assert!(
            platform.puts.borrow().is_empty(),
            "the write waits for the end of the frame"
        );

        pump_frame(&mut app);
        let sent = platform.puts.borrow().last().cloned().expect("serialized");
        assert_eq!(
            at(&sent, &["c", "app", "c", "counter", "v", "count"]),
            Some(&9i64.into())
        );
    }

    #[test]
    fn did_update_restoration_id_moves_the_bucket_and_its_data() {
        let (cell, platform) = app_restoring(Some(child(
            "app",
            child("counter", values([("count", 7i64.into())])),
        )));
        let mut app = cell.borrow_mut();
        let key = GlobalKey::new();
        let key_ref: KeyRef = Rc::new(key.clone());
        let tree = |restoration_id: &str, key: &KeyRef| {
            RootRestorationScope::new(
                Some("app".to_string()),
                Counter {
                    key: Some(key.clone()),
                    restoration_id: Some(restoration_id.to_string()),
                },
            )
            .into_widget()
        };
        let harness = mount(&mut app, tree("counter", &key_ref));
        let state = counter_state(&harness, &app, &key);
        let bucket = state.bucket(&app).expect("the counter claimed a bucket");

        harness.set_child(&mut app, tree("renamed", &key_ref));
        harness.pump(&mut app);

        assert_eq!(app.get(state).restores, 1, "a rename does not re-restore");
        assert_eq!(
            state.bucket(&app),
            Some(bucket),
            "the same bucket moved to the new ID"
        );
        assert_eq!(bucket.restoration_id(&app), "renamed");
        assert_eq!(*state.count(&app).value(&app), 7);

        pump_frame(&mut app);
        let sent = platform.puts.borrow().last().cloned().expect("serialized");
        assert_eq!(
            at(&sent, &["c", "app", "c", "renamed", "v", "count"]),
            Some(&7i64.into())
        );
        assert_eq!(at(&sent, &["c", "app", "c", "counter"]), None);
    }

    #[test]
    fn new_data_from_the_host_restores_the_registered_properties_again() {
        let (cell, _platform) = app_restoring(Some(child(
            "app",
            child("counter", values([("count", 7i64.into())])),
        )));
        let mut app = cell.borrow_mut();
        let key = GlobalKey::new();
        let harness = mount(
            &mut app,
            RootRestorationScope::new(
                Some("app".to_string()),
                Counter {
                    key: Some(Rc::new(key.clone())),
                    restoration_id: Some("counter".to_string()),
                },
            )
            .into_widget(),
        );
        let state = counter_state(&harness, &app, &key);
        assert_eq!(*state.count(&app).value(&app), 7);

        let manager = RestorationManager::instance(&mut app);
        manager.handle_restoration_update_from_engine(
            &mut app,
            true,
            Some(child(
                "app",
                child("counter", values([("count", 21i64.into())])),
            )),
        );
        harness.pump(&mut app);

        assert_eq!(app.get(state).restores, 2);
        assert_eq!(*state.count(&app).value(&app), 21);
    }

    #[test]
    fn a_state_without_a_surrounding_scope_gets_no_bucket() {
        let (cell, _platform) = app_restoring(None);
        let mut app = cell.borrow_mut();
        let key = GlobalKey::new();
        let harness = mount(
            &mut app,
            Counter {
                key: Some(Rc::new(key.clone())),
                restoration_id: Some("counter".to_string()),
            }
            .into_widget(),
        );

        let state = counter_state(&harness, &app, &key);
        assert_eq!(state.bucket(&app), None);
        assert_eq!(*state.count(&app).value(&app), 0);
    }

    #[test]
    fn of_panics_outside_any_restoration_scope() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let context = Harness::mount(&mut app, SizedBox::shrink().into_widget())
            .root
            .as_element();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            RestorationScope::of(&mut app, context)
        }));
        assert!(panic.is_err());
    }
}
