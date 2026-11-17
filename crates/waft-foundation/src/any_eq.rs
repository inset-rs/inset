//! Dart's `Object` where it is only compared with `==` and hashed.

use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

/// Dart's `Object` where it is only compared with `==` and hashed, as a hero's tag or a radio
/// panel's value: any value, equal only to an equal value of the same type, as two Dart objects
/// of different classes never are.
///
/// The blanket implementation covers every value that can answer both questions, so such a
/// value is written as `Rc::new("photo")` or `Rc::new(42)`.
pub trait AnyEq: Any + Debug {
    /// Dart's `operator ==`. Must return false when `other` is a different concrete type, even
    /// if the two carry the same value.
    fn eq_any(&self, other: &dyn AnyEq) -> bool;

    /// Dart's `hashCode`. Must mix in the concrete type so that two types carrying the same
    /// value do not collide.
    fn hash_any(&self, state: &mut dyn Hasher);
}

impl<T: PartialEq + Hash + Debug + 'static> AnyEq for T {
    fn eq_any(&self, other: &dyn AnyEq) -> bool {
        (other as &dyn Any)
            .downcast_ref::<T>()
            .is_some_and(|other| other == self)
    }

    fn hash_any(&self, mut state: &mut dyn Hasher) {
        TypeId::of::<T>().hash(&mut state);
        self.hash(&mut state);
    }
}

impl PartialEq for dyn AnyEq {
    fn eq(&self, other: &dyn AnyEq) -> bool {
        self.eq_any(other)
    }
}

impl Eq for dyn AnyEq {}

impl Hash for dyn AnyEq {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash_any(state);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    use super::AnyEq;

    fn hash(value: &dyn AnyEq) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn equal_values_of_one_type_are_equal() {
        let a: &dyn AnyEq = &"photo";
        let b: &dyn AnyEq = &"photo";
        let c: &dyn AnyEq = &"other";

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn equal_looking_values_of_two_types_are_not_equal() {
        let one: &dyn AnyEq = &1_i32;
        let another_one: &dyn AnyEq = &1_u32;
        let one_string: &dyn AnyEq = &"1";

        assert_ne!(one, another_one);
        assert_ne!(one, one_string);
    }

    #[test]
    fn hashing_agrees_with_equality() {
        assert_eq!(hash(&"photo"), hash(&"photo"));
        assert_ne!(hash(&1_i32), hash(&1_u32));
        assert_ne!(hash(&1_i32), hash(&"1"));
    }
}
