//! Flutter counterpart: `widgets/restoration_properties.dart`.
//!
//! The [`RestorableProperty`] implementations a [`State`](crate::State) registers with a
//! [`RestorationMixin`](crate::RestorationMixin): the value bases and the concrete properties
//! for the primitives, [`DateTime`], and enums.

use std::fmt::Debug;

use inset_foundation::{
    App, ChangeNotifier, ChangeNotifierData, DateTime, Handle, Listenable, Listener,
};
use inset_services::{RestorationData, TextEditingValue};

use crate::widgets::editable_text::TextEditingController;
use crate::widgets::restoration::{RestorableProperty, RestorablePropertyData};

// ---------------------------------------------------------------------------------------------
// RestorableValue

/// Dart's private `_value` of `RestorableValue`; a property carries this bag under the field
/// `value`.
pub struct RestorableValueData<T> {
    value: Option<T>,
}

impl<T> RestorableValueData<T> {
    /// A property whose [`init_with_value`](RestorableProperty::init_with_value) has not run.
    pub fn new() -> RestorableValueData<T> {
        RestorableValueData { value: None }
    }
}

impl<T> Default for RestorableValueData<T> {
    fn default() -> RestorableValueData<T> {
        RestorableValueData::new()
    }
}

/// The accessors [`RestorableValue`] asks for, for a property whose bag is the field `value`.
#[macro_export]
macro_rules! restorable_value_accessors {
    () => {
        fn restorable_value_data(
            self: ::inset_foundation::Handle<Self>,
            app: &::inset_foundation::App,
        ) -> &$crate::RestorableValueData<Self::Value> {
            &app.get(self).value
        }

        fn restorable_value_data_mut(
            self: ::inset_foundation::Handle<Self>,
            app: &mut ::inset_foundation::App,
        ) -> &mut $crate::RestorableValueData<Self::Value> {
            &mut app.get_mut(self).value
        }
    };
}

/// A [`RestorableProperty`] that makes the wrapped value accessible to the owning
/// [`State`](crate::State) object via the [`value`](Self::value) getter and
/// [`set_value`](Self::set_value) setter.
///
/// Whenever a new value is set, [`did_update_value`](Self::did_update_value) is called.
/// Implementors should call `notify_listeners` from that method if the new value changes what
/// [`to_primitives`](RestorableProperty::to_primitives) returns.
///
/// ## Creating an implementor
///
/// ```text
/// pub struct RestorableDuration {
///     change_notifier: ChangeNotifierData,
///     property: RestorablePropertyData,
///     value: RestorableValueData<Duration>,
/// }
///
/// impl RestorableProperty for RestorableDuration {
///     type Value = Duration;
///     crate::restorable_property_accessors!();
///
///     fn create_default_value(self: Handle<Self>, _app: &mut App) -> Duration {
///         Duration::ZERO
///     }
///
///     fn from_primitives(self: Handle<Self>, _app: &mut App, data: &RestorationData) -> Duration {
///         Duration::from_micros(data.as_int().unwrap_or(0) as u64)
///     }
///
///     fn init_with_value(self: Handle<Self>, app: &mut App, value: Duration) {
///         RestorableValue::init_with_value(self, app, value)
///     }
///
///     fn to_primitives(self: Handle<Self>, app: &App) -> RestorationData {
///         RestorationData::Int(RestorableValue::value(self, app).as_micros() as i64)
///     }
/// }
///
/// impl RestorableValue for RestorableDuration {
///     crate::restorable_value_accessors!();
///
///     fn did_update_value(self: Handle<Self>, app: &mut App, old_value: Option<Duration>) {
///         if old_value != Some(*RestorableValue::value(self, app)) {
///             self.notify_listeners(app);
///         }
///     }
/// }
/// ```
///
/// See also:
///
///  * [`RestorableProperty`], which is the base trait of this trait.
///  * [`RestorationMixin`](crate::RestorationMixin), to which a [`RestorableValue`] needs to be
///    registered in order to work.
pub trait RestorableValue: RestorableProperty<Value: Clone + PartialEq> {
    /// Dart's `RestorableValue._value`, held under the field `value`
    /// ([`restorable_value_accessors!`](crate::restorable_value_accessors)).
    fn restorable_value_data(self: Handle<Self>, app: &App) -> &RestorableValueData<Self::Value>;

    /// See [`restorable_value_data`](Self::restorable_value_data).
    fn restorable_value_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut RestorableValueData<Self::Value>;

    /// The current value stored in this property.
    ///
    /// A representation of the current value is stored in the restoration data. During state
    /// restoration, the property will restore the value to what it was when the restoration
    /// data it is getting restored from was collected.
    ///
    /// The value can only be accessed after the property has been registered with a
    /// [`RestorationMixin`](crate::RestorationMixin) by calling
    /// [`register_for_restoration`](crate::RestorationMixin::register_for_restoration).
    fn value(self: Handle<Self>, app: &App) -> &Self::Value {
        debug_assert!(RestorableProperty::is_registered(self, app));
        self.restorable_value_data(app)
            .value
            .as_ref()
            .expect("init_with_value runs at registration")
    }

    /// Replaces [`value`](Self::value), calling
    /// [`did_update_value`](Self::did_update_value) when it changes.
    fn set_value(self: Handle<Self>, app: &mut App, new_value: Self::Value) {
        debug_assert!(RestorableProperty::is_registered(self, app));
        debug_assert!(self.debug_assert_valid_value(app, &new_value));
        if self.restorable_value_data(app).value.as_ref() != Some(&new_value) {
            let old_value = self.restorable_value_data_mut(app).value.replace(new_value);
            RestorableValue::did_update_value(self, app, old_value);
        }
    }

    /// Dart's `RestorableValue.initWithValue`: the value the
    /// [`RestorationMixin`](crate::RestorationMixin) restored or defaulted becomes
    /// [`value`](Self::value).
    fn init_with_value(self: Handle<Self>, app: &mut App, value: Self::Value) {
        self.restorable_value_data_mut(app).value = Some(value);
    }

    /// Called whenever a new value is assigned to [`value`](Self::value).
    ///
    /// The new value can be read with the regular [`value`](Self::value) getter and the previous
    /// value is provided as `old_value`.
    ///
    /// Implementors should call `notify_listeners` from this method, if the new value changes
    /// what [`to_primitives`](RestorableProperty::to_primitives) returns.
    fn did_update_value(self: Handle<Self>, app: &mut App, old_value: Option<Self::Value>);

    /// Checks what Dart asserts inside a `set value` override before running the inherited
    /// setter; [`RestorableEnum`] and [`RestorableEnumN`] check membership in their `values`.
    ///
    /// Call it inside a `debug_assert!`, as Dart calls it inside an `assert`.
    fn debug_assert_valid_value(self: Handle<Self>, app: &App, new_value: &Self::Value) -> bool {
        let _ = (self, app, new_value);
        true
    }
}

// ---------------------------------------------------------------------------------------------
// The primitive value bases

/// A value that is stored in the restoration data as one [`RestorationData`]; Dart's cast
/// `serialized as T` in `_RestorablePrimitiveValueN.fromPrimitives`.
///
/// Implemented for the primitives a [`RestorationBucket`](crate::RestorationBucket) can hold and
/// for `Option` of each of them, which is the nullable half of Dart's primitive property pairs.
pub trait RestorablePrimitive: Clone + PartialEq + 'static {
    /// Dart's `serialized as T`; panics when the stored data is of another kind, which is what
    /// Dart's failing cast does.
    fn from_restoration_data(data: &RestorationData) -> Self;

    /// The value as it is stored in the restoration data.
    fn to_restoration_data(&self) -> RestorationData;
}

impl RestorablePrimitive for bool {
    fn from_restoration_data(data: &RestorationData) -> bool {
        data.as_bool()
            .expect("the restoration data of a bool property is a bool")
    }

    fn to_restoration_data(&self) -> RestorationData {
        RestorationData::Bool(*self)
    }
}

impl RestorablePrimitive for i64 {
    fn from_restoration_data(data: &RestorationData) -> i64 {
        data.as_int()
            .expect("the restoration data of an int property is an int")
    }

    fn to_restoration_data(&self) -> RestorationData {
        RestorationData::Int(*self)
    }
}

impl RestorablePrimitive for f64 {
    fn from_restoration_data(data: &RestorationData) -> f64 {
        data.as_double()
            .expect("the restoration data of a double property is a double")
    }

    fn to_restoration_data(&self) -> RestorationData {
        RestorationData::Double(*self)
    }
}

impl RestorablePrimitive for String {
    fn from_restoration_data(data: &RestorationData) -> String {
        data.as_str()
            .expect("the restoration data of a String property is a String")
            .to_string()
    }

    fn to_restoration_data(&self) -> RestorationData {
        RestorationData::String(self.clone())
    }
}

impl<T: RestorablePrimitive> RestorablePrimitive for Option<T> {
    fn from_restoration_data(data: &RestorationData) -> Option<T> {
        (!data.is_null()).then(|| T::from_restoration_data(data))
    }

    fn to_restoration_data(&self) -> RestorationData {
        self.as_ref()
            .map_or(RestorationData::Null, T::to_restoration_data)
    }
}

/// Dart's `T extends num`: the numeric values a [`RestorableNum`] may wrap.
pub trait RestorableNumValue: RestorablePrimitive {}

impl RestorableNumValue for i64 {}

impl RestorableNumValue for f64 {}

/// Dart's private `_defaultValue` of `_RestorablePrimitiveValueN`; a primitive property carries
/// this bag under the field `primitive`.
pub struct RestorablePrimitiveValueData<T> {
    default_value: T,
}

impl<T> RestorablePrimitiveValueData<T> {
    /// The bag holding the value the property falls back to.
    pub fn new(default_value: T) -> RestorablePrimitiveValueData<T> {
        RestorablePrimitiveValueData { default_value }
    }
}

/// The accessors [`RestorablePrimitiveValueN`] asks for, for a property whose bag is the field
/// `primitive`.
#[macro_export]
macro_rules! restorable_primitive_value_accessors {
    () => {
        fn restorable_primitive_value_data(
            self: ::inset_foundation::Handle<Self>,
            app: &::inset_foundation::App,
        ) -> &$crate::RestorablePrimitiveValueData<Self::Value> {
            &app.get(self).primitive
        }

        fn restorable_primitive_value_data_mut(
            self: ::inset_foundation::Handle<Self>,
            app: &mut ::inset_foundation::App,
        ) -> &mut $crate::RestorablePrimitiveValueData<Self::Value> {
            &mut app.get_mut(self).primitive
        }
    };
}

/// Dart's `_RestorablePrimitiveValueN<T extends Object?>`: the shared bodies of every property
/// that stores its value as a single primitive, nullable values included.
///
/// See [`RestorablePrimitiveValue`] for the non-nullable half of the pair.
pub trait RestorablePrimitiveValueN: RestorableValue<Value: RestorablePrimitive> {
    /// Dart's `_RestorablePrimitiveValueN._defaultValue`, held under the field `primitive`
    /// ([`restorable_primitive_value_accessors!`](crate::restorable_primitive_value_accessors)).
    fn restorable_primitive_value_data(
        self: Handle<Self>,
        app: &App,
    ) -> &RestorablePrimitiveValueData<Self::Value>;

    /// See [`restorable_primitive_value_data`](Self::restorable_primitive_value_data).
    fn restorable_primitive_value_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut RestorablePrimitiveValueData<Self::Value>;

    /// Dart's `_RestorablePrimitiveValueN.createDefaultValue`.
    fn create_default_value(self: Handle<Self>, app: &mut App) -> Self::Value {
        self.restorable_primitive_value_data(app)
            .default_value
            .clone()
    }

    /// Dart's `_RestorablePrimitiveValueN.didUpdateValue`.
    fn did_update_value(self: Handle<Self>, app: &mut App, old_value: Option<Self::Value>) {
        let _ = old_value;
        self.notify_listeners(app);
    }

    /// Dart's `_RestorablePrimitiveValueN.fromPrimitives`.
    fn from_primitives(self: Handle<Self>, app: &mut App, data: &RestorationData) -> Self::Value {
        let _ = app;
        Self::Value::from_restoration_data(data)
    }

    /// Dart's `_RestorablePrimitiveValueN.toPrimitives`.
    fn to_primitives(self: Handle<Self>, app: &App) -> RestorationData {
        RestorableValue::value(self, app).to_restoration_data()
    }
}

/// Dart's `_RestorablePrimitiveValue<T extends Object>`: the non-nullable half of the primitive
/// property pairs.
///
/// Dart's overrides only narrow [`RestorablePrimitiveValueN`]'s nullable types and assert that
/// the stored data is not null; here the value type is already non-nullable and
/// [`RestorablePrimitive::from_restoration_data`] carries the assert, so the trait adds no body
/// of its own.
pub trait RestorablePrimitiveValue: RestorablePrimitiveValueN {}

// ---------------------------------------------------------------------------------------------
// The primitive properties

/// A [`RestorableProperty`] that knows how to store and restore a number.
///
/// The current value of this property is stored in the restoration data. During state
/// restoration the property is restored to the value it had when the restoration data it is
/// getting restored from was collected.
///
/// If no restoration data is available, the value is initialized to the `default_value` given in
/// the constructor.
///
/// Instead of using the more generic [`RestorableNum`] directly, consider using one of the more
/// specific aliases ([`RestorableDouble`] to store an `f64` and [`RestorableInt`] to store an
/// `i64`).
///
/// See also:
///
///  * [`RestorableNumN`] for the nullable version of this property.
pub struct RestorableNum<T: RestorableNumValue> {
    change_notifier: ChangeNotifierData,
    property: RestorablePropertyData,
    value: RestorableValueData<T>,
    primitive: RestorablePrimitiveValueData<T>,
}

impl<T: RestorableNumValue> RestorableNum<T> {
    /// Creates a [`RestorableNum`].
    ///
    /// If no restoration data is available to restore the value in this property from, the
    /// property will be initialized with the provided `default_value`.
    pub fn new(app: &mut App, default_value: T) -> Handle<RestorableNum<T>> {
        app.create(RestorableNum {
            change_notifier: ChangeNotifierData::new(),
            property: RestorablePropertyData::new(),
            value: RestorableValueData::new(),
            primitive: RestorablePrimitiveValueData::new(default_value),
        })
    }
}

impl<T: RestorableNumValue> ChangeNotifier for RestorableNum<T> {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl<T: RestorableNumValue> RestorableProperty for RestorableNum<T> {
    type Value = T;
    crate::restorable_property_accessors!();

    fn create_default_value(self: Handle<Self>, app: &mut App) -> T {
        RestorablePrimitiveValueN::create_default_value(self, app)
    }

    fn from_primitives(self: Handle<Self>, app: &mut App, data: &RestorationData) -> T {
        RestorablePrimitiveValueN::from_primitives(self, app, data)
    }

    fn init_with_value(self: Handle<Self>, app: &mut App, value: T) {
        RestorableValue::init_with_value(self, app, value);
    }

    fn to_primitives(self: Handle<Self>, app: &App) -> RestorationData {
        RestorablePrimitiveValueN::to_primitives(self, app)
    }
}

impl<T: RestorableNumValue> RestorableValue for RestorableNum<T> {
    crate::restorable_value_accessors!();

    fn did_update_value(self: Handle<Self>, app: &mut App, old_value: Option<T>) {
        RestorablePrimitiveValueN::did_update_value(self, app, old_value);
    }
}

impl<T: RestorableNumValue> RestorablePrimitiveValueN for RestorableNum<T> {
    crate::restorable_primitive_value_accessors!();
}

impl<T: RestorableNumValue> RestorablePrimitiveValue for RestorableNum<T> {}

/// A [`RestorableProperty`] that knows how to store and restore an `f64`; Dart's
/// `RestorableDouble`, which adds nothing to [`RestorableNum`] but the name.
///
/// See also:
///
///  * [`RestorableDoubleN`] for the nullable version of this property.
pub type RestorableDouble = RestorableNum<f64>;

/// A [`RestorableProperty`] that knows how to store and restore an `i64`; Dart's
/// `RestorableInt`, which adds nothing to [`RestorableNum`] but the name.
///
/// See also:
///
///  * [`RestorableIntN`] for the nullable version of this property.
pub type RestorableInt = RestorableNum<i64>;

/// A [`RestorableProperty`] that knows how to store and restore a `String`.
///
/// The current value of this property is stored in the restoration data. During state
/// restoration the property is restored to the value it had when the restoration data it is
/// getting restored from was collected.
///
/// If no restoration data is available, the value is initialized to the `default_value` given in
/// the constructor.
///
/// See also:
///
///  * [`RestorableStringN`] for the nullable version of this property.
pub struct RestorableString {
    change_notifier: ChangeNotifierData,
    property: RestorablePropertyData,
    value: RestorableValueData<String>,
    primitive: RestorablePrimitiveValueData<String>,
}

impl RestorableString {
    /// Creates a [`RestorableString`].
    ///
    /// If no restoration data is available to restore the value in this property from, the
    /// property will be initialized with the provided `default_value`.
    pub fn new(app: &mut App, default_value: impl Into<String>) -> Handle<RestorableString> {
        app.create(RestorableString {
            change_notifier: ChangeNotifierData::new(),
            property: RestorablePropertyData::new(),
            value: RestorableValueData::new(),
            primitive: RestorablePrimitiveValueData::new(default_value.into()),
        })
    }
}

impl ChangeNotifier for RestorableString {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl RestorableProperty for RestorableString {
    type Value = String;
    crate::restorable_property_accessors!();

    fn create_default_value(self: Handle<Self>, app: &mut App) -> String {
        RestorablePrimitiveValueN::create_default_value(self, app)
    }

    fn from_primitives(self: Handle<Self>, app: &mut App, data: &RestorationData) -> String {
        RestorablePrimitiveValueN::from_primitives(self, app, data)
    }

    fn init_with_value(self: Handle<Self>, app: &mut App, value: String) {
        RestorableValue::init_with_value(self, app, value);
    }

    fn to_primitives(self: Handle<Self>, app: &App) -> RestorationData {
        RestorablePrimitiveValueN::to_primitives(self, app)
    }
}

impl RestorableValue for RestorableString {
    crate::restorable_value_accessors!();

    fn did_update_value(self: Handle<Self>, app: &mut App, old_value: Option<String>) {
        RestorablePrimitiveValueN::did_update_value(self, app, old_value);
    }
}

impl RestorablePrimitiveValueN for RestorableString {
    crate::restorable_primitive_value_accessors!();
}

impl RestorablePrimitiveValue for RestorableString {}

/// A [`RestorableProperty`] that knows how to store and restore a `bool`.
///
/// The current value of this property is stored in the restoration data. During state
/// restoration the property is restored to the value it had when the restoration data it is
/// getting restored from was collected.
///
/// If no restoration data is available, the value is initialized to the `default_value` given in
/// the constructor.
///
/// See also:
///
///  * [`RestorableBoolN`] for the nullable version of this property.
pub struct RestorableBool {
    change_notifier: ChangeNotifierData,
    property: RestorablePropertyData,
    value: RestorableValueData<bool>,
    primitive: RestorablePrimitiveValueData<bool>,
}

impl RestorableBool {
    /// Creates a [`RestorableBool`].
    ///
    /// If no restoration data is available to restore the value in this property from, the
    /// property will be initialized with the provided `default_value`.
    pub fn new(app: &mut App, default_value: bool) -> Handle<RestorableBool> {
        app.create(RestorableBool {
            change_notifier: ChangeNotifierData::new(),
            property: RestorablePropertyData::new(),
            value: RestorableValueData::new(),
            primitive: RestorablePrimitiveValueData::new(default_value),
        })
    }
}

impl ChangeNotifier for RestorableBool {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl RestorableProperty for RestorableBool {
    type Value = bool;
    crate::restorable_property_accessors!();

    fn create_default_value(self: Handle<Self>, app: &mut App) -> bool {
        RestorablePrimitiveValueN::create_default_value(self, app)
    }

    fn from_primitives(self: Handle<Self>, app: &mut App, data: &RestorationData) -> bool {
        RestorablePrimitiveValueN::from_primitives(self, app, data)
    }

    fn init_with_value(self: Handle<Self>, app: &mut App, value: bool) {
        RestorableValue::init_with_value(self, app, value);
    }

    fn to_primitives(self: Handle<Self>, app: &App) -> RestorationData {
        RestorablePrimitiveValueN::to_primitives(self, app)
    }
}

impl RestorableValue for RestorableBool {
    crate::restorable_value_accessors!();

    fn did_update_value(self: Handle<Self>, app: &mut App, old_value: Option<bool>) {
        RestorablePrimitiveValueN::did_update_value(self, app, old_value);
    }
}

impl RestorablePrimitiveValueN for RestorableBool {
    crate::restorable_primitive_value_accessors!();
}

impl RestorablePrimitiveValue for RestorableBool {}

/// A [`RestorableProperty`] that knows how to store and restore a `bool` that is nullable.
///
/// The current value of this property is stored in the restoration data. During state
/// restoration the property is restored to the value it had when the restoration data it is
/// getting restored from was collected.
///
/// If no restoration data is available, the value is initialized to the `default_value` given in
/// the constructor.
///
/// See also:
///
///  * [`RestorableBool`] for the non-nullable version of this property.
pub struct RestorableBoolN {
    change_notifier: ChangeNotifierData,
    property: RestorablePropertyData,
    value: RestorableValueData<Option<bool>>,
    primitive: RestorablePrimitiveValueData<Option<bool>>,
}

impl RestorableBoolN {
    /// Creates a [`RestorableBoolN`].
    ///
    /// If no restoration data is available to restore the value in this property from, the
    /// property will be initialized with the provided `default_value`.
    pub fn new(app: &mut App, default_value: Option<bool>) -> Handle<RestorableBoolN> {
        app.create(RestorableBoolN {
            change_notifier: ChangeNotifierData::new(),
            property: RestorablePropertyData::new(),
            value: RestorableValueData::new(),
            primitive: RestorablePrimitiveValueData::new(default_value),
        })
    }
}

impl ChangeNotifier for RestorableBoolN {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl RestorableProperty for RestorableBoolN {
    type Value = Option<bool>;
    crate::restorable_property_accessors!();

    fn create_default_value(self: Handle<Self>, app: &mut App) -> Option<bool> {
        RestorablePrimitiveValueN::create_default_value(self, app)
    }

    fn from_primitives(self: Handle<Self>, app: &mut App, data: &RestorationData) -> Option<bool> {
        RestorablePrimitiveValueN::from_primitives(self, app, data)
    }

    fn init_with_value(self: Handle<Self>, app: &mut App, value: Option<bool>) {
        RestorableValue::init_with_value(self, app, value);
    }

    fn to_primitives(self: Handle<Self>, app: &App) -> RestorationData {
        RestorablePrimitiveValueN::to_primitives(self, app)
    }
}

impl RestorableValue for RestorableBoolN {
    crate::restorable_value_accessors!();

    fn did_update_value(self: Handle<Self>, app: &mut App, old_value: Option<Option<bool>>) {
        RestorablePrimitiveValueN::did_update_value(self, app, old_value);
    }
}

impl RestorablePrimitiveValueN for RestorableBoolN {
    crate::restorable_primitive_value_accessors!();
}

/// A [`RestorableProperty`] that knows how to store and restore a number that is nullable.
///
/// The current value of this property is stored in the restoration data. During state
/// restoration the property is restored to the value it had when the restoration data it is
/// getting restored from was collected.
///
/// If no restoration data is available, the value is initialized to the `default_value` given in
/// the constructor.
///
/// Instead of using the more generic [`RestorableNumN`] directly, consider using one of the more
/// specific aliases ([`RestorableDoubleN`] to store an `f64` and [`RestorableIntN`] to store an
/// `i64`).
///
/// See also:
///
///  * [`RestorableNum`] for the non-nullable version of this property.
pub struct RestorableNumN<T: RestorableNumValue> {
    change_notifier: ChangeNotifierData,
    property: RestorablePropertyData,
    value: RestorableValueData<Option<T>>,
    primitive: RestorablePrimitiveValueData<Option<T>>,
}

impl<T: RestorableNumValue> RestorableNumN<T> {
    /// Creates a [`RestorableNumN`].
    ///
    /// If no restoration data is available to restore the value in this property from, the
    /// property will be initialized with the provided `default_value`.
    pub fn new(app: &mut App, default_value: Option<T>) -> Handle<RestorableNumN<T>> {
        app.create(RestorableNumN {
            change_notifier: ChangeNotifierData::new(),
            property: RestorablePropertyData::new(),
            value: RestorableValueData::new(),
            primitive: RestorablePrimitiveValueData::new(default_value),
        })
    }
}

impl<T: RestorableNumValue> ChangeNotifier for RestorableNumN<T> {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl<T: RestorableNumValue> RestorableProperty for RestorableNumN<T> {
    type Value = Option<T>;
    crate::restorable_property_accessors!();

    fn create_default_value(self: Handle<Self>, app: &mut App) -> Option<T> {
        RestorablePrimitiveValueN::create_default_value(self, app)
    }

    fn from_primitives(self: Handle<Self>, app: &mut App, data: &RestorationData) -> Option<T> {
        RestorablePrimitiveValueN::from_primitives(self, app, data)
    }

    fn init_with_value(self: Handle<Self>, app: &mut App, value: Option<T>) {
        RestorableValue::init_with_value(self, app, value);
    }

    fn to_primitives(self: Handle<Self>, app: &App) -> RestorationData {
        RestorablePrimitiveValueN::to_primitives(self, app)
    }
}

impl<T: RestorableNumValue> RestorableValue for RestorableNumN<T> {
    crate::restorable_value_accessors!();

    fn did_update_value(self: Handle<Self>, app: &mut App, old_value: Option<Option<T>>) {
        RestorablePrimitiveValueN::did_update_value(self, app, old_value);
    }
}

impl<T: RestorableNumValue> RestorablePrimitiveValueN for RestorableNumN<T> {
    crate::restorable_primitive_value_accessors!();
}

/// A [`RestorableProperty`] that knows how to store and restore an `f64` that is nullable;
/// Dart's `RestorableDoubleN`, which adds nothing to [`RestorableNumN`] but the name.
///
/// See also:
///
///  * [`RestorableDouble`] for the non-nullable version of this property.
pub type RestorableDoubleN = RestorableNumN<f64>;

/// A [`RestorableProperty`] that knows how to store and restore an `i64` that is nullable;
/// Dart's `RestorableIntN`, which adds nothing to [`RestorableNumN`] but the name.
///
/// See also:
///
///  * [`RestorableInt`] for the non-nullable version of this property.
pub type RestorableIntN = RestorableNumN<i64>;

/// A [`RestorableProperty`] that knows how to store and restore a `String` that is nullable.
///
/// The current value of this property is stored in the restoration data. During state
/// restoration the property is restored to the value it had when the restoration data it is
/// getting restored from was collected.
///
/// If no restoration data is available, the value is initialized to the `default_value` given in
/// the constructor.
///
/// See also:
///
///  * [`RestorableString`] for the non-nullable version of this property.
pub struct RestorableStringN {
    change_notifier: ChangeNotifierData,
    property: RestorablePropertyData,
    value: RestorableValueData<Option<String>>,
    primitive: RestorablePrimitiveValueData<Option<String>>,
}

impl RestorableStringN {
    /// Creates a [`RestorableStringN`].
    ///
    /// If no restoration data is available to restore the value in this property from, the
    /// property will be initialized with the provided `default_value`.
    pub fn new(app: &mut App, default_value: Option<String>) -> Handle<RestorableStringN> {
        app.create(RestorableStringN {
            change_notifier: ChangeNotifierData::new(),
            property: RestorablePropertyData::new(),
            value: RestorableValueData::new(),
            primitive: RestorablePrimitiveValueData::new(default_value),
        })
    }
}

impl ChangeNotifier for RestorableStringN {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl RestorableProperty for RestorableStringN {
    type Value = Option<String>;
    crate::restorable_property_accessors!();

    fn create_default_value(self: Handle<Self>, app: &mut App) -> Option<String> {
        RestorablePrimitiveValueN::create_default_value(self, app)
    }

    fn from_primitives(
        self: Handle<Self>,
        app: &mut App,
        data: &RestorationData,
    ) -> Option<String> {
        RestorablePrimitiveValueN::from_primitives(self, app, data)
    }

    fn init_with_value(self: Handle<Self>, app: &mut App, value: Option<String>) {
        RestorableValue::init_with_value(self, app, value);
    }

    fn to_primitives(self: Handle<Self>, app: &App) -> RestorationData {
        RestorablePrimitiveValueN::to_primitives(self, app)
    }
}

impl RestorableValue for RestorableStringN {
    crate::restorable_value_accessors!();

    fn did_update_value(self: Handle<Self>, app: &mut App, old_value: Option<Option<String>>) {
        RestorablePrimitiveValueN::did_update_value(self, app, old_value);
    }
}

impl RestorablePrimitiveValueN for RestorableStringN {
    crate::restorable_primitive_value_accessors!();
}

// ---------------------------------------------------------------------------------------------
// RestorableDateTime

/// A [`RestorableValue`] that knows how to save and restore a [`DateTime`].
///
/// The current value of this property is stored in the restoration data as the milliseconds
/// since the epoch. During state restoration the property is restored to the value it had when
/// the restoration data it is getting restored from was collected.
///
/// If no restoration data is available, the value is initialized to the `default_value` given in
/// the constructor.
pub struct RestorableDateTime {
    change_notifier: ChangeNotifierData,
    property: RestorablePropertyData,
    value: RestorableValueData<DateTime>,
    default_value: DateTime,
}

impl RestorableDateTime {
    /// Creates a [`RestorableDateTime`].
    ///
    /// If no restoration data is available to restore the value in this property from, the
    /// property will be initialized with the provided `default_value`.
    pub fn new(app: &mut App, default_value: DateTime) -> Handle<RestorableDateTime> {
        app.create(RestorableDateTime {
            change_notifier: ChangeNotifierData::new(),
            property: RestorablePropertyData::new(),
            value: RestorableValueData::new(),
            default_value,
        })
    }
}

impl ChangeNotifier for RestorableDateTime {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl RestorableProperty for RestorableDateTime {
    type Value = DateTime;
    crate::restorable_property_accessors!();

    fn create_default_value(self: Handle<Self>, app: &mut App) -> DateTime {
        app.get(self).default_value
    }

    fn from_primitives(self: Handle<Self>, app: &mut App, data: &RestorationData) -> DateTime {
        let _ = app;
        DateTime::from_milliseconds_since_epoch(
            data.as_int()
                .expect("the restoration data of a DateTime property is an int"),
        )
    }

    fn init_with_value(self: Handle<Self>, app: &mut App, value: DateTime) {
        RestorableValue::init_with_value(self, app, value);
    }

    fn to_primitives(self: Handle<Self>, app: &App) -> RestorationData {
        RestorationData::Int(RestorableValue::value(self, app).milliseconds_since_epoch())
    }
}

impl RestorableValue for RestorableDateTime {
    crate::restorable_value_accessors!();

    fn did_update_value(self: Handle<Self>, app: &mut App, old_value: Option<DateTime>) {
        let _ = old_value;
        self.notify_listeners(app);
    }
}

/// A [`RestorableValue`] that knows how to save and restore a [`DateTime`] that is nullable.
///
/// The current value of this property is stored in the restoration data as the milliseconds
/// since the epoch. During state restoration the property is restored to the value it had when
/// the restoration data it is getting restored from was collected.
///
/// If no restoration data is available, the value is initialized to the `default_value` given in
/// the constructor.
pub struct RestorableDateTimeN {
    change_notifier: ChangeNotifierData,
    property: RestorablePropertyData,
    value: RestorableValueData<Option<DateTime>>,
    default_value: Option<DateTime>,
}

impl RestorableDateTimeN {
    /// Creates a [`RestorableDateTimeN`].
    ///
    /// If no restoration data is available to restore the value in this property from, the
    /// property will be initialized with the provided `default_value`.
    pub fn new(app: &mut App, default_value: Option<DateTime>) -> Handle<RestorableDateTimeN> {
        app.create(RestorableDateTimeN {
            change_notifier: ChangeNotifierData::new(),
            property: RestorablePropertyData::new(),
            value: RestorableValueData::new(),
            default_value,
        })
    }
}

impl ChangeNotifier for RestorableDateTimeN {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl RestorableProperty for RestorableDateTimeN {
    type Value = Option<DateTime>;
    crate::restorable_property_accessors!();

    fn create_default_value(self: Handle<Self>, app: &mut App) -> Option<DateTime> {
        app.get(self).default_value
    }

    fn from_primitives(
        self: Handle<Self>,
        app: &mut App,
        data: &RestorationData,
    ) -> Option<DateTime> {
        let _ = app;
        data.as_int().map(DateTime::from_milliseconds_since_epoch)
    }

    fn init_with_value(self: Handle<Self>, app: &mut App, value: Option<DateTime>) {
        RestorableValue::init_with_value(self, app, value);
    }

    fn to_primitives(self: Handle<Self>, app: &App) -> RestorationData {
        RestorableValue::value(self, app).map_or(RestorationData::Null, |value| {
            RestorationData::Int(value.milliseconds_since_epoch())
        })
    }
}

impl RestorableValue for RestorableDateTimeN {
    crate::restorable_value_accessors!();

    fn did_update_value(self: Handle<Self>, app: &mut App, old_value: Option<Option<DateTime>>) {
        let _ = old_value;
        self.notify_listeners(app);
    }
}

// ---------------------------------------------------------------------------------------------
// RestorableEnum

/// Dart's `EnumName.name`: the variant's name, which is what a derived `Debug` prints.
fn enum_name<T: Debug>(value: &T) -> String {
    format!("{value:?}")
}

/// A [`RestorableProperty`] that knows how to store and restore an enum.
///
/// The current value of this property is stored in the restoration data. During state
/// restoration the property is restored to the value it had when the restoration data it is
/// getting restored from was collected.
///
/// If no restoration data is available, the value is initialized to the `default_value` given in
/// the constructor.
///
/// The values are serialized using the name of the variant, obtained from its `Debug`.
///
/// The set of values the property may represent is [`values`](Self::values), which the
/// constructor takes.
///
/// See also:
///
///  * [`RestorableEnumN`], which knows how to store and restore nullable enums.
pub struct RestorableEnum<T: Copy + Debug + PartialEq + 'static> {
    change_notifier: ChangeNotifierData,
    property: RestorablePropertyData,
    value: RestorableValueData<T>,
    default_value: T,
    values: Vec<T>,
}

impl<T: Copy + Debug + PartialEq + 'static> RestorableEnum<T> {
    /// Creates a [`RestorableEnum`].
    ///
    /// If no restoration data is available to restore the value in this property from, the
    /// property will be initialized with the provided `default_value`.
    pub fn new(
        app: &mut App,
        default_value: T,
        values: impl IntoIterator<Item = T>,
    ) -> Handle<RestorableEnum<T>> {
        let values: Vec<T> = values.into_iter().collect();
        assert!(
            values.contains(&default_value),
            "Default value {default_value:?} not found in the values: {values:?}"
        );
        app.create(RestorableEnum {
            change_notifier: ChangeNotifierData::new(),
            property: RestorablePropertyData::new(),
            value: RestorableValueData::new(),
            default_value,
            values,
        })
    }

    /// The set of values that this [`RestorableEnum`] may represent.
    ///
    /// This supplies the enum values that are serialized and restored. If a value is
    /// encountered that is not in this set,
    /// [`from_primitives`](RestorableProperty::from_primitives) asserts when restoring.
    ///
    /// It is typically every variant of the enum type.
    pub fn values(self: Handle<Self>, app: &App) -> &[T] {
        &app.get(self).values
    }

    /// See [`values`](Self::values).
    pub fn set_values(self: Handle<Self>, app: &mut App, values: impl IntoIterator<Item = T>) {
        app.get_mut(self).values = values.into_iter().collect();
    }
}

impl<T: Copy + Debug + PartialEq + 'static> ChangeNotifier for RestorableEnum<T> {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl<T: Copy + Debug + PartialEq + 'static> RestorableProperty for RestorableEnum<T> {
    type Value = T;
    crate::restorable_property_accessors!();

    fn create_default_value(self: Handle<Self>, app: &mut App) -> T {
        app.get(self).default_value
    }

    fn from_primitives(self: Handle<Self>, app: &mut App, data: &RestorationData) -> T {
        if let Some(name) = data.as_str() {
            if let Some(allowed) = app
                .get(self)
                .values
                .iter()
                .find(|value| enum_name(value) == name)
            {
                return *allowed;
            }
            debug_assert!(
                false,
                "Attempted to restore an unknown enum value \"{name}\" that is not in the valid \
                 set of enum values: {:?}",
                app.get(self).values
            );
        }
        app.get(self).default_value
    }

    fn init_with_value(self: Handle<Self>, app: &mut App, value: T) {
        RestorableValue::init_with_value(self, app, value);
    }

    fn to_primitives(self: Handle<Self>, app: &App) -> RestorationData {
        RestorationData::String(enum_name(RestorableValue::value(self, app)))
    }
}

impl<T: Copy + Debug + PartialEq + 'static> RestorableValue for RestorableEnum<T> {
    crate::restorable_value_accessors!();

    fn did_update_value(self: Handle<Self>, app: &mut App, old_value: Option<T>) {
        let _ = old_value;
        self.notify_listeners(app);
    }

    fn debug_assert_valid_value(self: Handle<Self>, app: &App, new_value: &T) -> bool {
        assert!(
            app.get(self).values.contains(new_value),
            "Attempted to set an unknown enum value \"{new_value:?}\" that is not in the valid \
             set of enum values: {:?}",
            app.get(self).values
        );
        true
    }
}

/// A [`RestorableProperty`] that knows how to store and restore an enum that is nullable.
///
/// The current value of this property is stored in the restoration data. During state
/// restoration the property is restored to the value it had when the restoration data it is
/// getting restored from was collected.
///
/// If no restoration data is available, the value is initialized to the `default_value` given in
/// the constructor.
///
/// The values are serialized using the name of the variant, obtained from its `Debug`.
///
/// The set of non-`None` values the property may represent is [`values`](Self::values), which
/// the constructor takes; since [`RestorableEnumN`] allows `None`, that is a valid value too
/// even though it does not appear in the set.
///
/// See also:
///
///  * [`RestorableEnum`], which knows how to store and restore non-nullable enums.
pub struct RestorableEnumN<T: Copy + Debug + PartialEq + 'static> {
    change_notifier: ChangeNotifierData,
    property: RestorablePropertyData,
    value: RestorableValueData<Option<T>>,
    default_value: Option<T>,
    values: Vec<T>,
}

impl<T: Copy + Debug + PartialEq + 'static> RestorableEnumN<T> {
    /// Creates a [`RestorableEnumN`].
    ///
    /// If no restoration data is available to restore the value in this property from, the
    /// property will be initialized with the provided `default_value`.
    pub fn new(
        app: &mut App,
        default_value: Option<T>,
        values: impl IntoIterator<Item = T>,
    ) -> Handle<RestorableEnumN<T>> {
        let values: Vec<T> = values.into_iter().collect();
        assert!(
            default_value.is_none_or(|default| values.contains(&default)),
            "Default value {default_value:?} not found in the values: {values:?}"
        );
        app.create(RestorableEnumN {
            change_notifier: ChangeNotifierData::new(),
            property: RestorablePropertyData::new(),
            value: RestorableValueData::new(),
            default_value,
            values,
        })
    }

    /// The set of non-`None` values that this [`RestorableEnumN`] may represent.
    ///
    /// This supplies the enum values that are serialized and restored. If a value is
    /// encountered that is neither `None` nor in this set,
    /// [`from_primitives`](RestorableProperty::from_primitives) asserts when restoring.
    ///
    /// It is typically every variant of the enum type.
    pub fn values(self: Handle<Self>, app: &App) -> &[T] {
        &app.get(self).values
    }

    /// See [`values`](Self::values).
    pub fn set_values(self: Handle<Self>, app: &mut App, values: impl IntoIterator<Item = T>) {
        app.get_mut(self).values = values.into_iter().collect();
    }
}

impl<T: Copy + Debug + PartialEq + 'static> ChangeNotifier for RestorableEnumN<T> {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl<T: Copy + Debug + PartialEq + 'static> RestorableProperty for RestorableEnumN<T> {
    type Value = Option<T>;
    crate::restorable_property_accessors!();

    fn create_default_value(self: Handle<Self>, app: &mut App) -> Option<T> {
        app.get(self).default_value
    }

    fn from_primitives(self: Handle<Self>, app: &mut App, data: &RestorationData) -> Option<T> {
        if data.is_null() {
            return None;
        }
        if let Some(name) = data.as_str() {
            if let Some(allowed) = app
                .get(self)
                .values
                .iter()
                .find(|value| enum_name(value) == name)
            {
                return Some(*allowed);
            }
            debug_assert!(
                false,
                "Attempted to set an unknown enum value \"{name}\" that is not null, or in the \
                 valid set of enum values: {:?}",
                app.get(self).values
            );
        }
        app.get(self).default_value
    }

    fn init_with_value(self: Handle<Self>, app: &mut App, value: Option<T>) {
        RestorableValue::init_with_value(self, app, value);
    }

    fn to_primitives(self: Handle<Self>, app: &App) -> RestorationData {
        RestorableValue::value(self, app)
            .as_ref()
            .map_or(RestorationData::Null, |value| {
                RestorationData::String(enum_name(value))
            })
    }
}

impl<T: Copy + Debug + PartialEq + 'static> RestorableValue for RestorableEnumN<T> {
    crate::restorable_value_accessors!();

    fn did_update_value(self: Handle<Self>, app: &mut App, old_value: Option<Option<T>>) {
        let _ = old_value;
        self.notify_listeners(app);
    }

    fn debug_assert_valid_value(self: Handle<Self>, app: &App, new_value: &Option<T>) -> bool {
        assert!(
            new_value.is_none_or(|value| app.get(self).values.contains(&value)),
            "Attempted to set an unknown enum value \"{new_value:?}\" that is not null, or in \
             the valid set of enum values: {:?}",
            app.get(self).values
        );
        true
    }
}

// ---------------------------------------------------------------------------------------------
// RestorableListenable and RestorableChangeNotifier

/// Dart's private `_value` of `RestorableListenable`; a listenable property carries this bag
/// under the field `listenable`.
pub struct RestorableListenableData<T> {
    value: Option<T>,
}

impl<T> RestorableListenableData<T> {
    /// A property whose [`init_with_value`](RestorableProperty::init_with_value) has not run.
    pub fn new() -> RestorableListenableData<T> {
        RestorableListenableData { value: None }
    }
}

impl<T> Default for RestorableListenableData<T> {
    fn default() -> RestorableListenableData<T> {
        RestorableListenableData::new()
    }
}

/// The accessors [`RestorableListenable`] asks for, for a property whose bag is the field
/// `listenable`.
#[macro_export]
macro_rules! restorable_listenable_accessors {
    () => {
        fn restorable_listenable_data(
            self: ::inset_foundation::Handle<Self>,
            app: &::inset_foundation::App,
        ) -> &$crate::RestorableListenableData<Self::Value> {
            &app.get(self).listenable
        }

        fn restorable_listenable_data_mut(
            self: ::inset_foundation::Handle<Self>,
            app: &mut ::inset_foundation::App,
        ) -> &mut $crate::RestorableListenableData<Self::Value> {
            &mut app.get_mut(self).listenable
        }
    };
}

/// A base for a [`RestorableProperty`] that stores and restores a
/// [`Listenable`](inset_foundation::Listenable).
///
/// This trait may be used to implement a [`RestorableProperty`] for a listenable whose
/// information it needs to store in the restoration data changes whenever the listenable
/// notifies its listeners.
///
/// The [`RestorationMixin`](crate::RestorationMixin) this property is registered with will call
/// [`to_primitives`](RestorableProperty::to_primitives) whenever the wrapped listenable notifies
/// its listeners, to update the information that this property has stored in the restoration
/// data.
pub trait RestorableListenable: RestorableProperty<Value: Listenable + Clone + 'static> {
    /// Dart's `RestorableListenable._value`, held under the field `listenable`
    /// ([`restorable_listenable_accessors!`](crate::restorable_listenable_accessors)).
    fn restorable_listenable_data(
        self: Handle<Self>,
        app: &App,
    ) -> &RestorableListenableData<Self::Value>;

    /// See [`restorable_listenable_data`](Self::restorable_listenable_data).
    fn restorable_listenable_data_mut(
        self: Handle<Self>,
        app: &mut App,
    ) -> &mut RestorableListenableData<Self::Value>;

    /// The listenable stored in this property.
    ///
    /// A representation of its current value is stored in the restoration data. During state
    /// restoration, the listenable returned by this getter will be restored to the state it had
    /// when the restoration data the property is getting restored from was collected.
    ///
    /// The value can only be accessed after the property has been registered with a
    /// [`RestorationMixin`](crate::RestorationMixin) by calling
    /// [`register_for_restoration`](crate::RestorationMixin::register_for_restoration).
    fn value(self: Handle<Self>, app: &App) -> Self::Value {
        debug_assert!(RestorableProperty::is_registered(self, app));
        self.restorable_listenable_data(app)
            .value
            .clone()
            .expect("init_with_value runs at registration")
    }

    /// Dart's `RestorableListenable.initWithValue`: the old listenable is dropped and this
    /// property listens to the new one.
    fn init_with_value(self: Handle<Self>, app: &mut App, value: Self::Value) {
        let listener = self.notification_listener();
        if let Some(old_value) = self.restorable_listenable_data(app).value.clone() {
            old_value.remove_listener(app, &listener);
        }
        self.restorable_listenable_data_mut(app).value = Some(value.clone());
        value.add_listener(app, listener);
    }

    /// Dart's `RestorableListenable.dispose`.
    fn dispose(self: Handle<Self>, app: &mut App) {
        RestorableProperty::dispose_property(self, app);
        let listener = self.notification_listener();
        if let Some(value) = self.restorable_listenable_data(app).value.clone() {
            value.remove_listener(app, &listener);
        }
    }

    /// Dart's `notifyListeners` tear-off: the listener this property keeps on its value.
    fn notification_listener(self: Handle<Self>) -> Listener {
        Listener::handle_method(self, Self::forward_notification)
    }

    /// The body behind [`notification_listener`](Self::notification_listener).
    fn forward_notification(self: Handle<Self>, app: &mut App) {
        self.notify_listeners(app);
    }
}

/// A base for a [`RestorableProperty`] that stores and restores a
/// [`ChangeNotifier`](inset_foundation::ChangeNotifier).
///
/// This trait may be used to implement a [`RestorableProperty`] for a change notifier whose
/// information it needs to store in the restoration data changes whenever the notifier notifies
/// its listeners.
///
/// The [`RestorationMixin`](crate::RestorationMixin) this property is registered with will call
/// [`to_primitives`](RestorableProperty::to_primitives) whenever the wrapped notifier notifies
/// its listeners, to update the information that this property has stored in the restoration
/// data.
///
/// Furthermore, the property will dispose the wrapped notifier when either the property itself
/// is disposed or its value is replaced with another notifier.
pub trait RestorableChangeNotifier: RestorableListenable {
    /// Dart's `_value!.dispose()`: how the wrapped notifier is discarded.
    fn dispose_value(self: Handle<Self>, app: &mut App, value: Self::Value);

    /// Dart's `RestorableChangeNotifier.initWithValue`.
    fn init_with_value(self: Handle<Self>, app: &mut App, value: Self::Value) {
        self.dispose_old_value(app);
        RestorableListenable::init_with_value(self, app, value);
    }

    /// Dart's `RestorableChangeNotifier.dispose`.
    fn dispose(self: Handle<Self>, app: &mut App) {
        self.dispose_old_value(app);
        RestorableListenable::dispose(self, app);
    }

    /// Dart's `_disposeOldValue`.
    fn dispose_old_value(self: Handle<Self>, app: &mut App) {
        if let Some(value) = self.restorable_listenable_data(app).value.clone() {
            // Scheduling a microtask for dispose to give other entities a chance to remove
            // their listeners first.
            app.schedule_microtask(Listener::new(move |app| {
                Self::dispose_value(self, app, value.clone());
            }));
        }
    }
}

/// A [`RestorableProperty`] that knows how to store and restore a
/// [`TextEditingController`].
///
/// The [`TextEditingController`] is accessible via the [`value`](RestorableListenable::value)
/// getter. During state restoration, the property will restore
/// [`TextEditingController::text_value`] to the value it had when the restoration data it is
/// getting restored from was collected.
pub struct RestorableTextEditingController {
    change_notifier: ChangeNotifierData,
    property: RestorablePropertyData,
    listenable: RestorableListenableData<Handle<TextEditingController>>,
    initial_value: TextEditingValue,
}

impl RestorableTextEditingController {
    /// Creates a [`RestorableTextEditingController`].
    ///
    /// This constructor treats a missing `text` argument as if it were the empty string.
    pub fn new(app: &mut App) -> Handle<RestorableTextEditingController> {
        Self::from_value(app, TextEditingValue::EMPTY)
    }

    /// Dart `RestorableTextEditingController(text:)`.
    pub fn text(app: &mut App, text: impl Into<String>) -> Handle<RestorableTextEditingController> {
        Self::from_value(app, TextEditingValue::new().text(text))
    }

    /// Creates a [`RestorableTextEditingController`] from an initial [`TextEditingValue`].
    pub fn from_value(
        app: &mut App,
        value: TextEditingValue,
    ) -> Handle<RestorableTextEditingController> {
        app.create(RestorableTextEditingController {
            change_notifier: ChangeNotifierData::new(),
            property: RestorablePropertyData::new(),
            listenable: RestorableListenableData::new(),
            initial_value: value,
        })
    }
}

impl ChangeNotifier for RestorableTextEditingController {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

impl RestorableProperty for RestorableTextEditingController {
    type Value = Handle<TextEditingController>;
    crate::restorable_property_accessors!();

    fn create_default_value(self: Handle<Self>, app: &mut App) -> Handle<TextEditingController> {
        let initial_value = app.get(self).initial_value.clone();
        TextEditingController::from_value(app, Some(initial_value))
    }

    fn from_primitives(
        self: Handle<Self>,
        app: &mut App,
        data: &RestorationData,
    ) -> Handle<TextEditingController> {
        TextEditingController::new(app).text(
            app,
            data.as_str()
                .expect("RestorableTextEditingController stores a String"),
        )
    }

    fn init_with_value(self: Handle<Self>, app: &mut App, value: Handle<TextEditingController>) {
        RestorableChangeNotifier::init_with_value(self, app, value);
    }

    fn to_primitives(self: Handle<Self>, app: &App) -> RestorationData {
        RestorationData::String(
            RestorableListenable::value(self, app)
                .text_value(app)
                .to_string(),
        )
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        RestorableChangeNotifier::dispose(self, app);
    }
}

impl RestorableListenable for RestorableTextEditingController {
    crate::restorable_listenable_accessors!();
}

impl RestorableChangeNotifier for RestorableTextEditingController {
    fn dispose_value(self: Handle<Self>, app: &mut App, value: Handle<TextEditingController>) {
        app.get_mut(value).dispose();
    }
}

#[cfg(test)]
mod tests {
    use inset_foundation::AppCell;
    use std::rc::Rc;

    use inset_foundation::Handle;
    use inset_services::{RestorationBucket, RestorationManager, RestorationMap};

    use super::*;
    use crate::framework::{BuildContext, GlobalKey, KeyRef, StateData, StatefulWidget, WidgetRef};
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;
    use crate::widgets::restoration::{
        RestorationMixin, RestorationMixinData, RootRestorationScope,
    };
    use crate::{IntoWidget, State};

    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Direction {
        Up,
        Down,
    }

    const DIRECTIONS: [Direction; 2] = [Direction::Up, Direction::Down];

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

    /// One property of every kind, so a single registration exercises them all.
    #[derive(Clone, Copy)]
    struct Properties {
        int: Handle<RestorableInt>,
        double: Handle<RestorableDouble>,
        string: Handle<RestorableString>,
        boolean: Handle<RestorableBool>,
        int_n: Handle<RestorableIntN>,
        double_n: Handle<RestorableDoubleN>,
        string_n: Handle<RestorableStringN>,
        bool_n: Handle<RestorableBoolN>,
        date_time: Handle<RestorableDateTime>,
        date_time_n: Handle<RestorableDateTimeN>,
        direction: Handle<RestorableEnum<Direction>>,
        direction_n: Handle<RestorableEnumN<Direction>>,
    }

    impl Properties {
        fn create(app: &mut App) -> Properties {
            Properties {
                int: RestorableInt::new(app, 1),
                double: RestorableDouble::new(app, 1.5),
                string: RestorableString::new(app, "one"),
                boolean: RestorableBool::new(app, true),
                int_n: RestorableIntN::new(app, Some(2)),
                double_n: RestorableDoubleN::new(app, None),
                string_n: RestorableStringN::new(app, None),
                bool_n: RestorableBoolN::new(app, Some(false)),
                date_time: RestorableDateTime::new(
                    app,
                    DateTime::from_milliseconds_since_epoch(1000),
                ),
                date_time_n: RestorableDateTimeN::new(app, None),
                direction: RestorableEnum::new(app, Direction::Up, DIRECTIONS),
                direction_n: RestorableEnumN::new(app, None, DIRECTIONS),
            }
        }

        fn register(self, state: Handle<BagState>, app: &mut App) {
            state.register_for_restoration(app, self.int.as_property(), "int");
            state.register_for_restoration(app, self.double.as_property(), "double");
            state.register_for_restoration(app, self.string.as_property(), "string");
            state.register_for_restoration(app, self.boolean.as_property(), "bool");
            state.register_for_restoration(app, self.int_n.as_property(), "int-n");
            state.register_for_restoration(app, self.double_n.as_property(), "double-n");
            state.register_for_restoration(app, self.string_n.as_property(), "string-n");
            state.register_for_restoration(app, self.bool_n.as_property(), "bool-n");
            state.register_for_restoration(app, self.date_time.as_property(), "date-time");
            state.register_for_restoration(app, self.date_time_n.as_property(), "date-time-n");
            state.register_for_restoration(app, self.direction.as_property(), "direction");
            state.register_for_restoration(app, self.direction_n.as_property(), "direction-n");
        }
    }

    #[derive(Debug)]
    struct Bag {
        key: Option<KeyRef>,
    }

    impl StatefulWidget for Bag {
        type State = BagState;

        fn key(&self) -> Option<&KeyRef> {
            self.key.as_ref()
        }

        fn create_state(&self) -> BagState {
            BagState {
                state: StateData::new(),
                restoration: RestorationMixinData::new(),
                properties: None,
            }
        }
    }

    struct BagState {
        state: StateData<Bag>,
        restoration: RestorationMixinData,
        properties: Option<Properties>,
    }

    impl RestorationMixin for BagState {
        crate::restoration_mixin_accessors!();

        fn restoration_id(self: Handle<Self>, _app: &App) -> Option<&str> {
            Some("bag")
        }

        fn restore_state(
            self: Handle<Self>,
            app: &mut App,
            _old_bucket: Option<Handle<RestorationBucket>>,
            _initial_restore: bool,
        ) {
            let properties = match app.get(self).properties {
                Some(properties) => properties,
                None => {
                    let properties = Properties::create(app);
                    app.get_mut(self).properties = Some(properties);
                    properties
                }
            };
            properties.register(self, app);
        }
    }

    impl State for BagState {
        type Widget = Bag;
        crate::state_accessors!();

        fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
            self.did_change_dependencies_restoration(app);
        }

        fn dispose(self: Handle<Self>, app: &mut App) {
            self.dispose_restoration(app);
        }

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            let _ = app;
            SizedBox::shrink().into_widget()
        }
    }

    /// A mounted [`Bag`] under a root scope holding `data`.
    struct Fixture {
        cell: Rc<AppCell>,
        properties: Properties,
        bucket: Handle<RestorationBucket>,
    }

    impl Fixture {
        fn mount(data: Option<RestorationMap>) -> Fixture {
            let cell = AppCell::new();
            let mut app = cell.borrow_mut();
            let manager = RestorationManager::instance(&mut app);
            manager.handle_restoration_update_from_engine(&mut app, true, data);
            let key = GlobalKey::new();
            let harness = Harness::mount(
                &mut app,
                RootRestorationScope::new(
                    Some("app".to_string()),
                    Bag {
                        key: Some(Rc::new(key.clone())),
                    },
                )
                .into_widget(),
            );
            harness.pump(&mut app);
            let state = harness
                .owner
                .global_key_element(&app, key.identity())
                .expect("the bag is in the tree")
                .state_handle::<BagState>(&app)
                .expect("a BagState");
            let properties = app.get(state).properties.expect("restore_state ran");
            let bucket = state.bucket(&app).expect("the bag claimed a bucket");
            drop(app);
            Fixture {
                cell,
                properties,
                bucket,
            }
        }

        fn stored(&mut self, restoration_id: &str) -> Option<RestorationData> {
            self.bucket
                .read(&mut self.cell.borrow_mut(), restoration_id)
        }
    }

    /// The data a [`Fixture`] restores from when every property carries a non-default value.
    fn filled() -> Option<RestorationMap> {
        Some(child(
            "app",
            child(
                "bag",
                values([
                    ("int", 7i64.into()),
                    ("double", 2.5f64.into()),
                    ("string", "seven".into()),
                    ("bool", false.into()),
                    ("int-n", RestorationData::Null),
                    ("double-n", 0.5f64.into()),
                    ("string-n", "maybe".into()),
                    ("bool-n", RestorationData::Null),
                    ("date-time", 2000i64.into()),
                    ("date-time-n", 3000i64.into()),
                    ("direction", "Down".into()),
                    ("direction-n", "Up".into()),
                ]),
            ),
        ))
    }

    #[test]
    fn a_property_with_no_stored_value_takes_the_default_it_was_created_with() {
        let fixture = Fixture::mount(None);
        let properties = fixture.properties;
        let app = &mut fixture.cell.borrow_mut();

        assert_eq!(*properties.int.value(app), 1);
        assert_eq!(*properties.double.value(app), 1.5);
        assert_eq!(properties.string.value(app), "one");
        assert!(*properties.boolean.value(app));
        assert_eq!(*properties.int_n.value(app), Some(2));
        assert_eq!(*properties.double_n.value(app), None);
        assert_eq!(*properties.string_n.value(app), None);
        assert_eq!(*properties.bool_n.value(app), Some(false));
        assert_eq!(
            *properties.date_time.value(app),
            DateTime::from_milliseconds_since_epoch(1000)
        );
        assert_eq!(*properties.date_time_n.value(app), None);
        assert_eq!(*properties.direction.value(app), Direction::Up);
        assert_eq!(*properties.direction_n.value(app), None);
    }

    #[test]
    fn a_default_value_is_written_into_the_bucket_at_registration() {
        let mut fixture = Fixture::mount(None);

        assert_eq!(fixture.stored("int"), Some(1i64.into()));
        assert_eq!(fixture.stored("double"), Some(1.5f64.into()));
        assert_eq!(fixture.stored("string"), Some("one".into()));
        assert_eq!(fixture.stored("bool"), Some(true.into()));
        assert_eq!(fixture.stored("int-n"), Some(2i64.into()));
        assert_eq!(fixture.stored("double-n"), Some(RestorationData::Null));
        assert_eq!(fixture.stored("string-n"), Some(RestorationData::Null));
        assert_eq!(fixture.stored("bool-n"), Some(false.into()));
        assert_eq!(fixture.stored("date-time"), Some(1000i64.into()));
        assert_eq!(fixture.stored("date-time-n"), Some(RestorationData::Null));
        assert_eq!(fixture.stored("direction"), Some("Up".into()));
        assert_eq!(fixture.stored("direction-n"), Some(RestorationData::Null));
    }

    #[test]
    fn every_property_restores_the_value_the_data_describes() {
        let fixture = Fixture::mount(filled());
        let properties = fixture.properties;
        let app = &mut fixture.cell.borrow_mut();

        assert_eq!(*properties.int.value(app), 7);
        assert_eq!(*properties.double.value(app), 2.5);
        assert_eq!(properties.string.value(app), "seven");
        assert!(!*properties.boolean.value(app));
        assert_eq!(*properties.int_n.value(app), None);
        assert_eq!(*properties.double_n.value(app), Some(0.5));
        assert_eq!(properties.string_n.value(app).as_deref(), Some("maybe"));
        assert_eq!(*properties.bool_n.value(app), None);
        assert_eq!(
            *properties.date_time.value(app),
            DateTime::from_milliseconds_since_epoch(2000)
        );
        assert_eq!(
            *properties.date_time_n.value(app),
            Some(DateTime::from_milliseconds_since_epoch(3000))
        );
        assert_eq!(*properties.direction.value(app), Direction::Down);
        assert_eq!(*properties.direction_n.value(app), Some(Direction::Up));
    }

    #[test]
    fn a_new_value_is_written_back_as_the_data_it_was_restored_from() {
        let mut fixture = Fixture::mount(filled());
        let properties = fixture.properties;
        {
            let app = &mut fixture.cell.borrow_mut();
            properties.int.set_value(app, 8);
            properties.double.set_value(app, 3.5);
            properties.string.set_value(app, "eight".to_string());
            properties.boolean.set_value(app, true);
            properties.int_n.set_value(app, Some(9));
            properties.double_n.set_value(app, None);
            properties.string_n.set_value(app, Some("nine".to_string()));
            properties.bool_n.set_value(app, Some(true));
            properties
                .date_time
                .set_value(app, DateTime::from_milliseconds_since_epoch(4000));
            properties.date_time_n.set_value(app, None);
            properties.direction.set_value(app, Direction::Up);
            properties.direction_n.set_value(app, Some(Direction::Down));
        }

        assert_eq!(fixture.stored("int"), Some(8i64.into()));
        assert_eq!(fixture.stored("double"), Some(3.5f64.into()));
        assert_eq!(fixture.stored("string"), Some("eight".into()));
        assert_eq!(fixture.stored("bool"), Some(true.into()));
        assert_eq!(fixture.stored("int-n"), Some(9i64.into()));
        assert_eq!(fixture.stored("double-n"), Some(RestorationData::Null));
        assert_eq!(fixture.stored("string-n"), Some("nine".into()));
        assert_eq!(fixture.stored("bool-n"), Some(true.into()));
        assert_eq!(fixture.stored("date-time"), Some(4000i64.into()));
        assert_eq!(fixture.stored("date-time-n"), Some(RestorationData::Null));
        assert_eq!(fixture.stored("direction"), Some("Up".into()));
        assert_eq!(fixture.stored("direction-n"), Some("Down".into()));
    }

    #[test]
    fn setting_the_value_it_already_has_writes_nothing() {
        let fixture = Fixture::mount(filled());
        let int = fixture.properties.int;
        let notified = Rc::new(std::cell::Cell::new(0usize));
        let seen = Rc::clone(&notified);
        int.add_listener(
            &mut fixture.cell.borrow_mut(),
            Listener::new(move |_app| seen.set(seen.get() + 1)),
        );

        int.set_value(&mut fixture.cell.borrow_mut(), 7);
        assert_eq!(notified.get(), 0, "the value did not change");

        int.set_value(&mut fixture.cell.borrow_mut(), 8);
        assert_eq!(notified.get(), 1);
    }

    #[test]
    fn an_unregistered_property_is_removed_from_the_data() {
        let mut fixture = Fixture::mount(filled());
        let int = fixture.properties.int;
        let state = int.state(&fixture.cell.borrow());
        let bag = state
            .downcast::<BagState>(&fixture.cell.borrow())
            .expect("the owner is the bag");
        bag.unregister_from_restoration(&mut fixture.cell.borrow_mut(), int.as_property());

        assert_eq!(fixture.stored("int"), None);
        assert!(!int.as_property().is_registered(&fixture.cell.borrow()));
    }

    #[test]
    fn restorable_text_editing_controller_create_default_value_holds_initial_text() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let property = RestorableTextEditingController::from_value(
            &mut app,
            TextEditingValue::new().text("hello"),
        );
        let controller = RestorableProperty::create_default_value(property, &mut app);
        assert_eq!(controller.text_value(&app), "hello");
        let restored = RestorableProperty::from_primitives(
            property,
            &mut app,
            &RestorationData::String("hello".into()),
        );
        assert_eq!(restored.text_value(&app), "hello");
    }

    #[derive(Debug)]
    struct Editor {
        key: Option<KeyRef>,
    }

    impl StatefulWidget for Editor {
        type State = EditorState;

        fn key(&self) -> Option<&KeyRef> {
            self.key.as_ref()
        }

        fn create_state(&self) -> EditorState {
            EditorState {
                state: StateData::new(),
                restoration: RestorationMixinData::new(),
                controller: None,
            }
        }
    }

    struct EditorState {
        state: StateData<Editor>,
        restoration: RestorationMixinData,
        controller: Option<Handle<RestorableTextEditingController>>,
    }

    impl RestorationMixin for EditorState {
        crate::restoration_mixin_accessors!();

        fn restoration_id(self: Handle<Self>, _app: &App) -> Option<&str> {
            Some("editor")
        }

        fn restore_state(
            self: Handle<Self>,
            app: &mut App,
            _old_bucket: Option<Handle<RestorationBucket>>,
            _initial_restore: bool,
        ) {
            let controller = match app.get(self).controller {
                Some(controller) => controller,
                None => {
                    let controller = RestorableTextEditingController::text(app, "hello");
                    app.get_mut(self).controller = Some(controller);
                    controller
                }
            };
            self.register_for_restoration(app, controller.as_property(), "controller");
        }
    }

    impl State for EditorState {
        type Widget = Editor;
        crate::state_accessors!();

        fn did_change_dependencies(self: Handle<Self>, app: &mut App) {
            self.did_change_dependencies_restoration(app);
        }

        fn dispose(self: Handle<Self>, app: &mut App) {
            self.dispose_restoration(app);
        }

        fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
            let _ = app;
            SizedBox::shrink().into_widget()
        }
    }

    #[test]
    fn restorable_text_editing_to_primitives_is_the_controller_text() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let manager = RestorationManager::instance(&mut app);
        manager.handle_restoration_update_from_engine(&mut app, true, None);
        let key = GlobalKey::new();
        let harness = Harness::mount(
            &mut app,
            RootRestorationScope::new(
                Some("app".to_string()),
                Editor {
                    key: Some(Rc::new(key.clone())),
                },
            )
            .into_widget(),
        );
        harness.pump(&mut app);
        let state = harness
            .owner
            .global_key_element(&app, key.identity())
            .expect("the editor is in the tree")
            .state_handle::<EditorState>(&app)
            .expect("an EditorState");
        let controller = app.get(state).controller.expect("restore_state ran");
        assert_eq!(
            RestorableListenable::value(controller, &app).text_value(&app),
            "hello"
        );
        assert_eq!(
            RestorableProperty::to_primitives(controller, &app),
            RestorationData::String("hello".into())
        );
    }
}
