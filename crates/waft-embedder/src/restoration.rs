//! The restoration data the framework hands the host and gets back.
//!
//! Flutter's restoration data is a `Map<Object?, Object?>` that travels over
//! `SystemChannels.restoration` encoded with `StandardMessageCodec`. There is no channel
//! here, so the data stays a value: [`RestorationData`] is one value of the kinds that
//! codec can carry, and [`RestorationMap`] is the map itself.

use std::hash::{Hash, Hasher};

use indexmap::IndexMap;

/// Flutter's `Map<Object?, Object?>` of restoration data: insertion-ordered, keyed by value.
pub type RestorationMap = IndexMap<RestorationData, RestorationData>;

/// One value in the restoration data, of the kinds `StandardMessageCodec` can carry.
///
/// [`Double`](Self::Double) compares and hashes by IEEE equality, so `0.0` and `-0.0` are
/// one key and a `NaN` key can never be looked up again — as in Dart.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum RestorationData {
    /// Dart's `null`.
    #[default]
    Null,
    /// Dart's `bool`.
    Bool(bool),
    /// Dart's `int`.
    Int(i64),
    /// Dart's `double`.
    Double(f64),
    /// Dart's `String`.
    String(String),
    /// Dart's `Uint8List`.
    Uint8List(Vec<u8>),
    /// Dart's `Int32List`.
    Int32List(Vec<i32>),
    /// Dart's `Int64List`.
    Int64List(Vec<i64>),
    /// Dart's `Float64List`.
    Float64List(Vec<f64>),
    /// Dart's `List<Object?>`.
    List(Vec<RestorationData>),
    /// Dart's `Map<Object?, Object?>`.
    Map(RestorationMap),
}

impl RestorationData {
    /// Whether this is [`Null`](Self::Null).
    pub fn is_null(&self) -> bool {
        matches!(self, RestorationData::Null)
    }

    /// The `bool`, or `None` for another kind.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            RestorationData::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// The `int`, or `None` for another kind.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            RestorationData::Int(value) => Some(*value),
            _ => None,
        }
    }

    /// The `double`, or `None` for another kind.
    pub fn as_double(&self) -> Option<f64> {
        match self {
            RestorationData::Double(value) => Some(*value),
            _ => None,
        }
    }

    /// The `String`, or `None` for another kind.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            RestorationData::String(value) => Some(value),
            _ => None,
        }
    }

    /// The `Uint8List`, or `None` for another kind.
    pub fn as_uint8_list(&self) -> Option<&[u8]> {
        match self {
            RestorationData::Uint8List(values) => Some(values),
            _ => None,
        }
    }

    /// The `Int32List`, or `None` for another kind.
    pub fn as_int32_list(&self) -> Option<&[i32]> {
        match self {
            RestorationData::Int32List(values) => Some(values),
            _ => None,
        }
    }

    /// The `Int64List`, or `None` for another kind.
    pub fn as_int64_list(&self) -> Option<&[i64]> {
        match self {
            RestorationData::Int64List(values) => Some(values),
            _ => None,
        }
    }

    /// The `Float64List`, or `None` for another kind.
    pub fn as_float64_list(&self) -> Option<&[f64]> {
        match self {
            RestorationData::Float64List(values) => Some(values),
            _ => None,
        }
    }

    /// The list, or `None` for another kind.
    pub fn as_list(&self) -> Option<&[RestorationData]> {
        match self {
            RestorationData::List(values) => Some(values),
            _ => None,
        }
    }

    /// The map, or `None` for another kind.
    pub fn as_map(&self) -> Option<&RestorationMap> {
        match self {
            RestorationData::Map(map) => Some(map),
            _ => None,
        }
    }

    /// The map to write into, or `None` for another kind.
    pub fn as_map_mut(&mut self) -> Option<&mut RestorationMap> {
        match self {
            RestorationData::Map(map) => Some(map),
            _ => None,
        }
    }

    /// Takes the map out, or `None` for another kind.
    pub fn into_map(self) -> Option<RestorationMap> {
        match self {
            RestorationData::Map(map) => Some(map),
            _ => None,
        }
    }
}

/// `NaN` hashes by its bits and `-0.0` hashes as `0.0`, so hashing agrees with IEEE equality.
fn hash_double<H: Hasher>(value: f64, state: &mut H) {
    let normalized = if value == 0.0 { 0.0 } else { value };
    normalized.to_bits().hash(state);
}

impl Eq for RestorationData {}

impl Hash for RestorationData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            RestorationData::Null => {}
            RestorationData::Bool(value) => value.hash(state),
            RestorationData::Int(value) => value.hash(state),
            RestorationData::Double(value) => hash_double(*value, state),
            RestorationData::String(value) => value.hash(state),
            RestorationData::Uint8List(values) => values.hash(state),
            RestorationData::Int32List(values) => values.hash(state),
            RestorationData::Int64List(values) => values.hash(state),
            RestorationData::Float64List(values) => {
                for value in values {
                    hash_double(*value, state);
                }
            }
            RestorationData::List(values) => values.hash(state),
            // Two maps are equal whatever order they were built in, so only the size
            // can take part in the hash.
            RestorationData::Map(map) => map.len().hash(state),
        }
    }
}

impl From<bool> for RestorationData {
    fn from(value: bool) -> RestorationData {
        RestorationData::Bool(value)
    }
}

impl From<i64> for RestorationData {
    fn from(value: i64) -> RestorationData {
        RestorationData::Int(value)
    }
}

impl From<f64> for RestorationData {
    fn from(value: f64) -> RestorationData {
        RestorationData::Double(value)
    }
}

impl From<&str> for RestorationData {
    fn from(value: &str) -> RestorationData {
        RestorationData::String(value.to_string())
    }
}

impl From<String> for RestorationData {
    fn from(value: String) -> RestorationData {
        RestorationData::String(value)
    }
}

impl From<Vec<u8>> for RestorationData {
    fn from(values: Vec<u8>) -> RestorationData {
        RestorationData::Uint8List(values)
    }
}

impl From<Vec<i32>> for RestorationData {
    fn from(values: Vec<i32>) -> RestorationData {
        RestorationData::Int32List(values)
    }
}

impl From<Vec<i64>> for RestorationData {
    fn from(values: Vec<i64>) -> RestorationData {
        RestorationData::Int64List(values)
    }
}

impl From<Vec<f64>> for RestorationData {
    fn from(values: Vec<f64>) -> RestorationData {
        RestorationData::Float64List(values)
    }
}

impl From<Vec<RestorationData>> for RestorationData {
    fn from(values: Vec<RestorationData>) -> RestorationData {
        RestorationData::List(values)
    }
}

impl From<RestorationMap> for RestorationData {
    fn from(map: RestorationMap) -> RestorationData {
        RestorationData::Map(map)
    }
}

impl<T: Into<RestorationData>> From<Option<T>> for RestorationData {
    fn from(value: Option<T>) -> RestorationData {
        match value {
            Some(value) => value.into(),
            None => RestorationData::Null,
        }
    }
}

/// What the host answers when the framework asks for the restoration data it stored.
///
/// Flutter's `get` on `SystemChannels.restoration` replies with a map of the same two
/// entries.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RestorationUpdate {
    /// Whether the host wants restoration data at all. When false, state restoration is
    /// turned off and the application has no root bucket.
    pub enabled: bool,

    /// The data the host stored the last time the framework handed it some. `None` starts
    /// the application with an empty root bucket.
    pub data: Option<RestorationMap>,
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    use super::{RestorationData, RestorationMap};

    fn hash_of(value: &RestorationData) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn zero_and_negative_zero_are_one_key() {
        let zero = RestorationData::from(0.0);
        let negative_zero = RestorationData::from(-0.0);
        assert_eq!(zero, negative_zero);
        assert_eq!(hash_of(&zero), hash_of(&negative_zero));
    }

    #[test]
    fn a_map_keyed_by_value_finds_what_it_stored() {
        let mut map = RestorationMap::new();
        map.insert("value1".into(), 10i64.into());
        map.insert(12i64.into(), vec![1.5f64, 2.5].into());

        assert_eq!(
            map.get(&RestorationData::from("value1")).unwrap().as_int(),
            Some(10)
        );
        assert_eq!(
            map.get(&RestorationData::from(12i64))
                .unwrap()
                .as_float64_list(),
            Some([1.5, 2.5].as_slice())
        );
        assert!(map.get(&RestorationData::from("missing")).is_none());
    }

    #[test]
    fn an_optional_value_becomes_null() {
        assert!(RestorationData::from(None::<i64>).is_null());
        assert_eq!(RestorationData::from(Some(3i64)).as_int(), Some(3));
    }
}
