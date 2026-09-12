//! Flutter counterpart: `widgets/page_storage.dart`.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use inset_foundation::{App, Handle, Key, LocalKey};

use crate::framework::{
    AnyElement, BuildContext, IntoWidget, KeyRef, StatelessWidget, WidgetRef, downcast_widget,
};

/// A `Key` that can be used to persist the widget state in storage after the destruction
/// and will be restored when recreated.
///
/// Each key with its value plus the ancestor chain of other [`PageStorageKey`]s need to be
/// unique within the widget's closest ancestor [`PageStorage`]. To make it possible for a
/// saved value to be found when a widget is recreated, the key's values must not be objects
/// whose identity will change each time the widget is created.
///
/// Dart's `PageStorageKey<T> extends ValueKey<T>`; the value type is erased inside, as
/// `GlobalKey`'s is.
///
/// See also:
///
///  * `RestorationManager`, which manages state restoration.
pub struct PageStorageKey {
    value: KeyRef,
}

impl PageStorageKey {
    /// Creates a [`PageStorageKey`].
    ///
    /// The `value` is the value to which this key delegates its equality.
    pub fn new<T: PartialEq + Hash + fmt::Debug + 'static>(value: T) -> PageStorageKey {
        PageStorageKey {
            value: Rc::new(inset_foundation::ValueKey::new(value)),
        }
    }

    /// The key as a tree node's key.
    pub fn into_key(self) -> KeyRef {
        Rc::new(self)
    }
}

impl Key for PageStorageKey {
    fn eq_key(&self, other: &dyn Key) -> bool {
        (other as &dyn Any)
            .downcast_ref::<PageStorageKey>()
            .is_some_and(|other| other.value.eq_key(&*self.value))
    }

    fn hash_key(&self, mut state: &mut dyn Hasher) {
        TypeId::of::<PageStorageKey>().hash(&mut state);
        self.value.hash_key(state);
    }
}

impl LocalKey for PageStorageKey {}

impl fmt::Debug for PageStorageKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[PageStorageKey {:?}]", self.value)
    }
}

/// Dart's `_StorageEntryIdentifier`: the [`PageStorageKey`] chain from a context up to its
/// [`PageStorage`].
#[derive(Clone)]
struct StorageEntryIdentifier {
    keys: Vec<KeyRef>,
}

impl StorageEntryIdentifier {
    fn is_not_empty(&self) -> bool {
        !self.keys.is_empty()
    }
}

impl PartialEq for StorageEntryIdentifier {
    fn eq(&self, other: &StorageEntryIdentifier) -> bool {
        self.keys.len() == other.keys.len()
            && self
                .keys
                .iter()
                .zip(&other.keys)
                .all(|(mine, theirs)| mine.eq_key(&**theirs))
    }
}

impl Eq for StorageEntryIdentifier {}

impl Hash for StorageEntryIdentifier {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for key in &self.keys {
            key.hash_key(state);
        }
    }
}

impl fmt::Debug for StorageEntryIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StorageEntryIdentifier(")?;
        for (index, key) in self.keys.iter().enumerate() {
            if index > 0 {
                write!(f, ":")?;
            }
            write!(f, "{key:?}")?;
        }
        write!(f, ")")
    }
}

/// What a bucket entry is filed under: Dart's `Object` map key, either the computed chain or
/// a caller's identifier.
#[derive(Clone)]
enum StorageSlot {
    Entry(StorageEntryIdentifier),
    Identifier(KeyRef),
}

impl PartialEq for StorageSlot {
    fn eq(&self, other: &StorageSlot) -> bool {
        match (self, other) {
            (StorageSlot::Entry(mine), StorageSlot::Entry(theirs)) => mine == theirs,
            (StorageSlot::Identifier(mine), StorageSlot::Identifier(theirs)) => {
                mine.eq_key(&**theirs)
            }
            _ => false,
        }
    }
}

impl Eq for StorageSlot {}

impl Hash for StorageSlot {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            StorageSlot::Entry(entry) => {
                0u8.hash(state);
                entry.hash(state);
            }
            StorageSlot::Identifier(key) => {
                1u8.hash(state);
                key.hash_key(state);
            }
        }
    }
}

/// A storage bucket associated with a page in an app.
///
/// Useful for storing per-page state that persists across navigations from one page to
/// another.
///
/// A bucket has identity, so it is an arena object reached through its `Handle`.
#[derive(Default)]
pub struct PageStorageBucket {
    storage: Option<HashMap<StorageSlot, Rc<dyn Any>>>,
}

impl PageStorageBucket {
    /// Creates an empty bucket.
    pub fn new(app: &mut App) -> Handle<PageStorageBucket> {
        app.create(PageStorageBucket::default())
    }

    fn maybe_add_key(app: &App, context: AnyElement, keys: &mut Vec<KeyRef>) -> bool {
        let widget = context.widget(app);
        if let Some(key) = widget.key()
            && (&**key as &dyn Any).is::<PageStorageKey>()
        {
            keys.push(key.clone());
        }
        downcast_widget::<PageStorage>(&**widget).is_none()
    }

    fn all_keys(app: &App, context: BuildContext) -> Vec<KeyRef> {
        let mut keys = Vec::new();
        if Self::maybe_add_key(app, context, &mut keys) {
            context.visit_ancestor_elements(app, &mut |element| {
                Self::maybe_add_key(app, element, &mut keys)
            });
        }
        keys
    }

    fn compute_identifier(app: &App, context: BuildContext) -> StorageEntryIdentifier {
        StorageEntryIdentifier {
            keys: Self::all_keys(app, context),
        }
    }

    /// Write the given data into this page storage bucket using the specified identifier or
    /// an identifier computed from the given context. The computed identifier is based on
    /// the [`PageStorageKey`]s found in the path from context to the [`PageStorage`] widget
    /// that owns this page storage bucket.
    ///
    /// If an explicit identifier is not provided and no [`PageStorageKey`]s are found, then
    /// the `data` is not saved.
    pub fn write_state(
        self: Handle<Self>,
        app: &mut App,
        context: BuildContext,
        data: Rc<dyn Any>,
        identifier: Option<KeyRef>,
    ) {
        let slot = match identifier {
            Some(identifier) => StorageSlot::Identifier(identifier),
            None => {
                let context_identifier = Self::compute_identifier(app, context);
                if !context_identifier.is_not_empty() {
                    app.get_mut(self).storage.get_or_insert_with(HashMap::new);
                    return;
                }
                StorageSlot::Entry(context_identifier)
            }
        };
        app.get_mut(self)
            .storage
            .get_or_insert_with(HashMap::new)
            .insert(slot, data);
    }

    /// Read given data from into this page storage bucket using the specified identifier or
    /// an identifier computed from the given context. The computed identifier is based on
    /// the [`PageStorageKey`]s found in the path from context to the [`PageStorage`] widget
    /// that owns this page storage bucket.
    ///
    /// If an explicit identifier is not provided and no [`PageStorageKey`]s are found, then
    /// null is returned.
    pub fn read_state(
        self: Handle<Self>,
        app: &App,
        context: BuildContext,
        identifier: Option<KeyRef>,
    ) -> Option<Rc<dyn Any>> {
        let storage = app.get(self).storage.as_ref()?;
        let slot = match identifier {
            Some(identifier) => StorageSlot::Identifier(identifier),
            None => {
                let context_identifier = Self::compute_identifier(app, context);
                if !context_identifier.is_not_empty() {
                    return None;
                }
                StorageSlot::Entry(context_identifier)
            }
        };
        storage.get(&slot).cloned()
    }
}

/// Establish a subtree in which widgets can opt into persisting states after being
/// destroyed.
///
/// [`PageStorage`] is used to save and restore values that can outlive the widget. For
/// example, when multiple pages are grouped in tabs, when a page is switched out, its widget
/// is destroyed and its state is lost. By adding a [`PageStorage`] at the root and adding a
/// [`PageStorageKey`] to each page, some of the page's state (e.g. the scroll position of a
/// `Scrollable` widget) will be stored automatically in its closest ancestor [`PageStorage`],
/// and restored when it's switched back.
///
/// Usually you don't need to explicitly use a [`PageStorage`], since it's already included
/// in a `Navigator` (Dart's `MaterialApp` / `CupertinoApp` do it for you).
#[derive(Debug)]
pub struct PageStorage {
    pub key: Option<KeyRef>,
    /// The page storage bucket to use for this subtree.
    pub bucket: Handle<PageStorageBucket>,
    /// The widget below this widget in the tree.
    pub child: WidgetRef,
}

impl PageStorage {
    /// Creates a widget that provides a storage bucket for its descendants.
    pub fn new<K>(bucket: Handle<PageStorageBucket>, child: impl IntoWidget<K>) -> PageStorage {
        PageStorage {
            key: None,
            bucket,
            child: child.into_widget(),
        }
    }

    /// Dart `PageStorage(key:)`.
    pub fn key(mut self, key: KeyRef) -> PageStorage {
        self.key = Some(key);
        self
    }

    /// The [`PageStorageBucket`] from the closest instance of a [`PageStorage`] widget that
    /// encloses the given context.
    ///
    /// Returns null if none exists.
    ///
    /// Typical usage is as follows:
    ///
    /// ```text
    /// PageStorage::maybe_of(app, context)
    /// ```
    ///
    /// This method can be expensive (it walks the element tree).
    pub fn maybe_of(app: &App, context: BuildContext) -> Option<Handle<PageStorageBucket>> {
        context
            .find_ancestor_widget_of_exact_type::<PageStorage>(app)
            .map(|widget| widget.bucket)
    }

    /// The [`PageStorageBucket`] from the closest instance of a [`PageStorage`] widget that
    /// encloses the given context.
    ///
    /// If no ancestor is found, this method will assert in debug mode, and throw an
    /// exception in release mode.
    ///
    /// This method can be expensive (it walks the element tree).
    pub fn of(app: &App, context: BuildContext) -> Handle<PageStorageBucket> {
        PageStorage::maybe_of(app, context).unwrap_or_else(|| {
            panic!(
                "PageStorage.of() was called with a context that does not contain a PageStorage \
                 widget.\nNo PageStorage widget ancestor could be found starting from the \
                 context that was passed to PageStorage.of(). This can happen because you are \
                 using a widget that looks for a PageStorage ancestor, but no such ancestor \
                 exists.\nThe context used was:\n  {context:?}"
            )
        })
    }
}

impl StatelessWidget for PageStorage {
    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        self.child.clone()
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::cell::RefCell;

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Builder, Padding, SizedBox};
    use inset_painting::EdgeInsetsGeometry;

    /// A leaf that writes `value` on its first build and reads it back on the next.
    fn probe(seen: &Rc<RefCell<Vec<Option<i32>>>>, value: i32) -> WidgetRef {
        let seen = Rc::clone(seen);
        Builder::new(move |app, context| {
            let bucket = PageStorage::of(app, context);
            let read = bucket
                .read_state(app, context, None)
                .and_then(|data| data.downcast_ref::<i32>().copied());
            seen.borrow_mut().push(read);
            bucket.write_state(app, context, Rc::new(value), None);
            SizedBox::shrink().into_widget()
        })
        .into_widget()
    }

    #[test]
    fn state_is_filed_under_the_page_storage_key_chain() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let bucket = PageStorageBucket::new(&mut app);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let page = |seen: &Rc<RefCell<Vec<Option<i32>>>>, value: i32| {
            PageStorage::new(
                bucket,
                Padding::new(EdgeInsetsGeometry::ZERO)
                    .key(PageStorageKey::new("page").into_key())
                    .child(probe(seen, value)),
            )
            .into_widget()
        };
        let harness = Harness::mount(&mut app, page(&seen, 1));
        harness.pump(&mut app);
        assert_eq!(*seen.borrow(), vec![None]);

        // A recreated subtree with the same key chain reads the saved value.
        harness.set_child(&mut app, SizedBox::shrink().into_widget());
        harness.pump(&mut app);
        harness.set_child(&mut app, page(&seen, 2));
        harness.pump(&mut app);
        assert_eq!(*seen.borrow(), vec![None, Some(1)]);
    }

    #[test]
    fn without_keys_nothing_is_saved_and_identifiers_are_direct() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let bucket = PageStorageBucket::new(&mut app);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let harness = Harness::mount(
            &mut app,
            PageStorage::new(bucket, probe(&seen, 1)).into_widget(),
        );
        harness.pump(&mut app);
        harness.set_child(
            &mut app,
            PageStorage::new(bucket, probe(&seen, 2)).into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(*seen.borrow(), vec![None, None]);

        let context = Rc::new(std::cell::Cell::new(None));
        let capture = Builder::new({
            let context = Rc::clone(&context);
            move |_app, this| {
                context.set(Some(this));
                SizedBox::shrink().into_widget()
            }
        });
        harness.set_child(&mut app, PageStorage::new(bucket, capture).into_widget());
        harness.pump(&mut app);
        let context = context.get().expect("the probe built");
        assert_eq!(PageStorage::maybe_of(&app, context), Some(bucket));

        let identifier: KeyRef = Rc::new(inset_foundation::ValueKey::new("scroll"));
        bucket.write_state(&mut app, context, Rc::new(4.5f64), Some(identifier.clone()));
        let read = bucket
            .read_state(&app, context, Some(identifier))
            .and_then(|data| data.downcast_ref::<f64>().copied());
        assert_eq!(read, Some(4.5));
    }
}
