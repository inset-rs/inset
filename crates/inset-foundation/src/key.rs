//! Flutter counterpart: `foundation/key.dart`.

use std::any::{Any, TypeId};
use std::fmt::{self, Debug};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

/// A [`Key`] is an identifier for widgets, elements and semantics nodes.
///
/// A new widget will only be used to update an existing element if its key is
/// the same as the key of the current widget associated with the element.
///
/// Typically, if no key is provided, the default value is `None`, meaning the
/// widget is considered unkeyed. In this case, Flutter matches widgets based
/// on their type and position in the tree during rebuilds.
///
/// Keys must be unique amongst the elements with the same parent.
///
/// Implementors of [`Key`] should either implement [`LocalKey`] or `GlobalKey`.
///
/// A common mistake is to rebuild the widget tree in such a way that Flutter
/// attaches the incorrect state object to an unkeyed stateful widget. This can
/// often be solved by using an appropriate [`Key`] on the stateful widget.
///
/// See [`ValueKey`], which explains how that type can be used as a solution to a
/// common case of this problem.
pub trait Key: Any + Debug {
    /// Flutter's `operator ==`. Must return false when `other` is a different
    /// concrete key type, even if the two carry the same value.
    fn eq_key(&self, other: &dyn Key) -> bool;

    /// Flutter's `hashCode`. Must mix in the concrete key type so that two key
    /// types carrying the same value do not collide.
    fn hash_key(&self, state: &mut dyn Hasher);
}

impl PartialEq for dyn Key {
    fn eq(&self, other: &dyn Key) -> bool {
        self.eq_key(other)
    }
}

impl Eq for dyn Key {}

impl Hash for dyn Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash_key(state);
    }
}

impl dyn Key {
    /// Construct a [`ValueKey<String>`] with the given string.
    ///
    /// This is the simplest way to create keys.
    ///
    /// Dart: `const factory Key(String value) = ValueKey<String>`.
    #[expect(
        clippy::new_ret_no_self,
        reason = "Dart's Key factory returns ValueKey<String>, not the trait object"
    )]
    pub fn new(value: impl Into<String>) -> ValueKey<String> {
        ValueKey::new(value.into())
    }
}

/// A key that is not a `GlobalKey`.
///
/// Keys must be unique amongst the elements with the same parent. By
/// contrast, `GlobalKey`s must be unique across the entire app.
pub trait LocalKey: Key {}

/// A key that is only equal to itself.
///
/// This cannot be created with a const constructor because that implies that
/// all instantiated keys would be the same instance and therefore not be unique.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct UniqueKey {
    id: u64,
}

impl UniqueKey {
    /// Creates a key that is equal only to itself.
    #[expect(
        clippy::new_without_default,
        reason = "each call creates a distinct key; Default would suggest a neutral value"
    )]
    pub fn new() -> UniqueKey {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        UniqueKey {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
        }
    }
}

impl Key for UniqueKey {
    fn eq_key(&self, other: &dyn Key) -> bool {
        (other as &dyn Any)
            .downcast_ref::<UniqueKey>()
            .is_some_and(|other| other.id == self.id)
    }

    fn hash_key(&self, mut state: &mut dyn Hasher) {
        TypeId::of::<UniqueKey>().hash(&mut state);
        self.id.hash(&mut state);
    }
}

impl LocalKey for UniqueKey {}

impl Debug for UniqueKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[#{:05x}]", self.id & 0xF_FFFF)
    }
}

/// A key that uses a value of a particular type to identify itself.
///
/// A [`ValueKey`] is equal to another [`ValueKey`] if, and only if, their values
/// are equal.
///
/// This type can be wrapped to create value keys that will not be equal to
/// other value keys that happen to use the same value. If the wrapper is
/// private, this results in a value key type that cannot collide with keys from
/// other sources. This is useful when keys are used as fallbacks in the same
/// scope as keys supplied from another widget.
///
/// When building widgets from a collection of data, especially when that
/// collection can change over time (e.g., items being inserted, removed, or
/// reordered), keys are used to preserve the association between a widget and
/// the underlying data.
///
/// Without keys, the framework may have no way to distinguish between a change
/// in the data of an existing widget and a structural change in the list. As a
/// result, widgets may be incorrectly updated, and state held by stateful
/// widgets can be reused for a different piece of data.
///
/// Assigning a key ties the widget subtree to a specific piece of data,
/// allowing the framework to correctly match old and new widgets and preserve
/// state as expected.
///
/// In such cases, a [`ValueKey`] is typically appropriate, using a value that is
/// stable and unique for each item, such as an identifier from the data model.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ValueKey<T> {
    /// The value to which this key delegates its equality.
    pub value: T,
}

impl<T> ValueKey<T> {
    /// Creates a key that delegates its equality to the given value.
    pub const fn new(value: T) -> ValueKey<T> {
        ValueKey { value }
    }
}

impl<T: PartialEq + Hash + Debug + 'static> Key for ValueKey<T> {
    fn eq_key(&self, other: &dyn Key) -> bool {
        (other as &dyn Any)
            .downcast_ref::<ValueKey<T>>()
            .is_some_and(|other| other.value == self.value)
    }

    fn hash_key(&self, mut state: &mut dyn Hasher) {
        TypeId::of::<ValueKey<T>>().hash(&mut state);
        self.value.hash(&mut state);
    }
}

impl<T: PartialEq + Hash + Debug + 'static> LocalKey for ValueKey<T> {}

impl<T: Debug> Debug for ValueKey<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[<{:?}>]", self.value)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use super::*;

    #[derive(Debug, PartialEq, Eq, Hash)]
    struct SaltedKey(ValueKey<u32>);

    impl Key for SaltedKey {
        fn eq_key(&self, other: &dyn Key) -> bool {
            (other as &dyn Any)
                .downcast_ref::<SaltedKey>()
                .is_some_and(|other| other.0 == self.0)
        }

        fn hash_key(&self, mut state: &mut dyn Hasher) {
            TypeId::of::<SaltedKey>().hash(&mut state);
            self.0.hash(&mut state);
        }
    }

    impl LocalKey for SaltedKey {}

    fn erase(key: impl Key) -> Arc<dyn Key> {
        Arc::new(key)
    }

    #[test]
    fn value_keys_are_equal_when_their_values_are() {
        assert_eq!(&*erase(ValueKey::new(3)), &*erase(ValueKey::new(3)));
        assert_ne!(&*erase(ValueKey::new(3)), &*erase(ValueKey::new(4)));
        assert_eq!(
            &*erase(ValueKey::new("a".to_owned())),
            &*erase(ValueKey::new("a".to_owned()))
        );
    }

    #[test]
    fn value_keys_of_different_types_are_never_equal() {
        assert_ne!(&*erase(ValueKey::new(3u32)), &*erase(ValueKey::new(3u64)));
        assert_ne!(
            &*erase(ValueKey::new(3u32)),
            &*erase(SaltedKey(ValueKey::new(3)))
        );
    }

    #[test]
    fn unique_keys_are_equal_only_to_themselves() {
        let key = UniqueKey::new();
        assert_eq!(&*erase(key), &*erase(key));
        assert_ne!(&*erase(UniqueKey::new()), &*erase(UniqueKey::new()));
    }

    #[test]
    fn keys_of_different_types_share_one_map() {
        let mut elements: HashMap<Arc<dyn Key>, &str> = HashMap::new();
        elements.insert(erase(ValueKey::new(3u32)), "value");
        elements.insert(erase(SaltedKey(ValueKey::new(3))), "salted");

        assert_eq!(elements.len(), 2);
        assert_eq!(elements.get(&erase(ValueKey::new(3u32))), Some(&"value"));
        assert_eq!(
            elements.get(&erase(SaltedKey(ValueKey::new(3)))),
            Some(&"salted")
        );
    }

    #[test]
    fn debug_matches_flutter_to_string() {
        assert_eq!(format!("{:?}", ValueKey::new(3)), "[<3>]");
        assert_eq!(format!("{:?}", ValueKey::new("a")), r#"[<"a">]"#);
        assert_eq!(format!("{:?}", UniqueKey { id: 0x1_2345 }), "[#12345]");
        assert_eq!(format!("{:?}", UniqueKey { id: 0x7 }), "[#00007]");
    }

    #[test]
    fn dyn_key_new_is_a_string_value_key() {
        assert_eq!(
            &*erase(<dyn Key>::new("x")),
            &*erase(ValueKey::new("x".to_owned()))
        );
    }
}
