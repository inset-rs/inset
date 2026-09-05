//! Flutter `widgets/shared_app_data.dart`.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

use reveal_foundation::{App, Handle};

use crate::framework::{
    BuildContext, InheritedModel, InheritedModelKind, InheritedWidget, IntoWidget, KeyRef, State,
    StateData, StatefulWidget, WidgetRef,
};

/// A key into the shared data: Dart's `K extends Object`, any hashable, comparable value.
#[derive(Clone)]
pub struct SharedAppDataKey {
    key: Rc<dyn Any>,
    hash: u64,
    equals: fn(&dyn Any, &dyn Any) -> bool,
    type_name: &'static str,
}

impl SharedAppDataKey {
    /// Erases `key`.
    pub fn new<K: Hash + Eq + 'static>(key: K) -> SharedAppDataKey {
        let mut hasher = DefaultHasher::new();
        TypeId::of::<K>().hash(&mut hasher);
        key.hash(&mut hasher);
        SharedAppDataKey {
            key: Rc::new(key),
            hash: hasher.finish(),
            equals: |a, b| a.downcast_ref::<K>() == b.downcast_ref::<K>(),
            type_name: std::any::type_name::<K>(),
        }
    }
}

impl PartialEq for SharedAppDataKey {
    fn eq(&self, other: &SharedAppDataKey) -> bool {
        self.hash == other.hash && (self.equals)(&*self.key, &*other.key)
    }
}

impl Eq for SharedAppDataKey {}

impl Hash for SharedAppDataKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl Debug for SharedAppDataKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SharedAppDataKey<{}>", self.type_name)
    }
}

/// A value in the shared data: Dart's `Object?`, kept with the equality of its type.
#[derive(Clone)]
struct SharedAppDataValue {
    value: Rc<dyn Any>,
    equals: fn(&dyn Any, &dyn Any) -> bool,
}

impl SharedAppDataValue {
    fn new<V: PartialEq + 'static>(value: V) -> SharedAppDataValue {
        SharedAppDataValue {
            value: Rc::new(value),
            equals: |a, b| a.downcast_ref::<V>() == b.downcast_ref::<V>(),
        }
    }

    fn equals(&self, other: &SharedAppDataValue) -> bool {
        (self.equals)(&*self.value, &*other.value)
    }
}

type SharedAppDataMap = HashMap<SharedAppDataKey, SharedAppDataValue>;

/// Enables sharing key/value data with its `child` and all of the
/// child's descendants.
///
/// - `SharedAppData::get_value` creates a dependency on the key's value and returns it.
/// - `SharedAppData::set_value` changes the key's value and rebuilds the widgets that
///   depend on it.
///
/// The type of the key must be a hashable, comparable value; the type of a value must be
/// comparable, so that an unchanged value does not rebuild.
///
/// A widget whose build method uses `get_value` creates a dependency on the
/// `SharedAppData`. When the value of `key` changes with `set_value`, the widget will be
/// rebuilt. The values managed by the `SharedAppData` are expected to be immutable: intrinsic
/// changes to values will not cause dependent widgets to be rebuilt.
///
/// An instance of this widget is created automatically by `WidgetsApp`.
///
/// There are many ways to share data with a widget subtree. This
/// class is based on `InheritedModel`, which is an `InheritedWidget`.
/// It's intended to be used by packages that need to share a modest
/// number of values among their own components.
///
/// `SharedAppData` is not intended to be a substitute for Provider or any of
/// the other general purpose application state systems. `SharedAppData` is
/// for situations where a package's custom widgets need to share one
/// or a handful of immutable data objects that can be lazily
/// initialized. It exists so that packages like that can deliver
/// custom widgets without requiring the developer to add a
/// package-specific umbrella widget to their application.
#[derive(Debug)]
pub struct SharedAppData {
    key: Option<KeyRef>,
    /// The widget below this widget in the tree.
    child: WidgetRef,
}

impl SharedAppData {
    /// Creates a widget based on `InheritedModel` that supports sharing
    /// key/value data with its `child` and all of the child's descendants.
    pub fn new<K>(child: impl IntoWidget<K>) -> SharedAppData {
        SharedAppData {
            key: None,
            child: child.into_widget(),
        }
    }

    /// Dart `SharedAppData(key:)`.
    pub fn key(mut self, key: KeyRef) -> SharedAppData {
        self.key = Some(key);
        self
    }

    /// Returns the app model's value for `key` and ensures that each
    /// time the value of `key` is changed with `SharedAppData::set_value`, the
    /// specified context will be rebuilt.
    ///
    /// If no value for `key` exists then the `init` callback is used to
    /// generate an initial value. The callback is expected to return an
    /// immutable value because intrinsic changes to the value will not
    /// cause dependent widgets to be rebuilt.
    ///
    /// A widget that depends on the app model's value for `key` should use
    /// this method in their `build` methods to ensure that they are rebuilt
    /// if the value changes.
    ///
    /// The type parameter `K` is the type of the value's key and `V`
    /// is the type of the value.
    pub fn get_value<K: Hash + Eq + 'static, V: Clone + PartialEq + 'static>(
        app: &mut App,
        context: BuildContext,
        key: K,
        init: impl FnOnce() -> V,
    ) -> V {
        let key = SharedAppDataKey::new(key);
        let state = SharedAppModel::inherit_from(app, context, Some(key.clone()))
            .map(|model| model.shared_app_data_state)
            .expect(NO_SHARED_APP_DATA);
        state.get_value(app, key, init)
    }

    /// Changes the app model's `value` for `key` and rebuilds any widgets
    /// that have created a dependency on `key` with `SharedAppData::get_value`.
    ///
    /// If `value` is `==` to the current value of `key` then nothing is rebuilt.
    ///
    /// The `value` is expected to be immutable because intrinsic changes to
    /// the value will not cause dependent widgets to be rebuilt.
    ///
    /// Unlike `get_value`, this method does _not_ create a dependency between
    /// `context` and this widget.
    ///
    /// The type parameter `K` is the type of the value's key and `V`
    /// is the type of the value.
    pub fn set_value<K: Hash + Eq + 'static, V: PartialEq + 'static>(
        app: &mut App,
        context: BuildContext,
        key: K,
        value: V,
    ) {
        let state = context
            .get_inherited_widget_of_exact_type::<SharedAppModel>(app)
            .map(|model| model.shared_app_data_state)
            .expect(NO_SHARED_APP_DATA);
        state.set_value(app, SharedAppDataKey::new(key), value);
    }
}

const NO_SHARED_APP_DATA: &str = "No SharedAppData widget found.\n\
    SharedAppData.getValue / setValue require an SharedAppData widget ancestor.\n\
    Typically, the SharedAppData widget is introduced by the WidgetsApp widget at the top of \
    your application widget tree. It provides a key/value map of data that is shared with \
    the entire application.";

impl StatefulWidget for SharedAppData {
    type State = SharedAppDataState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> SharedAppDataState {
        SharedAppDataState {
            state: StateData::new(),
            data: Rc::default(),
        }
    }
}

/// Dart's `_SharedAppDataState`.
pub struct SharedAppDataState {
    state: StateData<SharedAppData>,
    /// The map the model widget shares (Dart hands the same `Map` object to the model);
    /// `set_value` replaces it, as Dart's `Map.of` copy does.
    data: Rc<RefCell<SharedAppDataMap>>,
}

impl SharedAppDataState {
    fn get_value<V: Clone + PartialEq + 'static>(
        self: Handle<Self>,
        app: &App,
        key: SharedAppDataKey,
        init: impl FnOnce() -> V,
    ) -> V {
        let data = Rc::clone(&app.get(self).data);
        let mut data = data.borrow_mut();
        let entry = data
            .entry(key)
            .or_insert_with(|| SharedAppDataValue::new(init()));
        entry
            .value
            .downcast_ref::<V>()
            .expect("the value stored under this key has this type")
            .clone()
    }

    fn set_value<V: PartialEq + 'static>(
        self: Handle<Self>,
        app: &mut App,
        key: SharedAppDataKey,
        value: V,
    ) {
        let value = SharedAppDataValue::new(value);
        let unchanged = app
            .get(self)
            .data
            .borrow()
            .get(&key)
            .is_some_and(|current| current.equals(&value));
        if unchanged {
            return;
        }
        self.set_state(app, |state| {
            let mut data = state.data.borrow().clone();
            data.insert(key, value);
            state.data = Rc::new(RefCell::new(data));
        });
    }
}

impl State for SharedAppDataState {
    type Widget = SharedAppData;
    crate::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        SharedAppModel {
            shared_app_data_state: self,
            data: Rc::clone(&app.get(self).data),
            child: self.widget(app).child.clone(),
        }
        .into_widget()
    }
}

/// Dart's `_SharedAppModel`.
struct SharedAppModel {
    shared_app_data_state: Handle<SharedAppDataState>,
    data: Rc<RefCell<SharedAppDataMap>>,
    child: WidgetRef,
}

impl SharedAppModel {
    fn into_widget(self) -> WidgetRef {
        IntoWidget::<InheritedModelKind>::into_widget(self)
    }
}

impl Debug for SharedAppModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedAppModel")
            .field("keys", &self.data.borrow().keys().collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

impl InheritedWidget for SharedAppModel {
    fn child(&self) -> &WidgetRef {
        &self.child
    }

    fn update_should_notify(&self, old: &SharedAppModel) -> bool {
        !Rc::ptr_eq(&self.data, &old.data)
    }
}

impl InheritedModel for SharedAppModel {
    type Aspect = SharedAppDataKey;

    fn update_should_notify_dependent(
        &self,
        old: &SharedAppModel,
        keys: &HashSet<SharedAppDataKey>,
    ) -> bool {
        let data = self.data.borrow();
        let old_data = old.data.borrow();
        keys.iter()
            .any(|key| match (data.get(key), old_data.get(key)) {
                (Some(value), Some(old_value)) => !value.equals(old_value),
                (None, None) => false,
                _ => true,
            })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::{Builder, SizedBox};

    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    enum Slot {
        Count,
        Name,
    }

    /// A probe that reads one key and counts its builds.
    fn reader<V: Clone + PartialEq + Debug + 'static>(
        key: Slot,
        init: V,
        seen: &Rc<RefCell<Vec<V>>>,
        context: &Rc<Cell<Option<BuildContext>>>,
    ) -> Builder {
        let seen = Rc::clone(seen);
        let context_slot = Rc::clone(context);
        Builder::new(move |app, context| {
            context_slot.set(Some(context));
            let value = SharedAppData::get_value(app, context, key, {
                let init = init.clone();
                move || init
            });
            seen.borrow_mut().push(value);
            SizedBox::shrink().into_widget()
        })
    }

    #[test]
    fn set_value_rebuilds_only_the_dependents_of_that_key() {
        let mut app = App::new();
        let counts: Rc<RefCell<Vec<i32>>> = Rc::default();
        let names: Rc<RefCell<Vec<String>>> = Rc::default();
        let count_context = Rc::new(Cell::new(None));
        let name_context = Rc::new(Cell::new(None));
        let harness = Harness::mount(
            &mut app,
            SharedAppData::new(crate::widgets::basic::Column::new().children([
                reader(Slot::Count, 0, &counts, &count_context).into_widget(),
                reader(Slot::Name, String::from("a"), &names, &name_context).into_widget(),
            ]))
            .into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(*counts.borrow(), [0]);
        assert_eq!(*names.borrow(), ["a"]);

        let context = count_context.get().expect("built");
        SharedAppData::set_value(&mut app, context, Slot::Count, 0);
        harness.pump(&mut app);
        assert_eq!(*counts.borrow(), [0], "an equal value rebuilds nothing");

        SharedAppData::set_value(&mut app, context, Slot::Count, 1);
        harness.pump(&mut app);
        assert_eq!(
            *counts.borrow(),
            [0, 1],
            "the dependent of the key rebuilds"
        );
        assert_eq!(*names.borrow(), ["a"], "the other key's dependent does not");

        SharedAppData::set_value(&mut app, context, Slot::Name, String::from("b"));
        harness.pump(&mut app);
        assert_eq!(*counts.borrow(), [0, 1]);
        assert_eq!(*names.borrow(), ["a", "b"]);
    }

    #[test]
    fn get_value_initializes_once_and_shares_the_value_with_later_readers() {
        let mut app = App::new();
        let inits = Rc::new(Cell::new(0));
        let seen: Rc<RefCell<Vec<i32>>> = Rc::default();
        let probe = |seen: &Rc<RefCell<Vec<i32>>>| {
            let seen = Rc::clone(seen);
            let inits = Rc::clone(&inits);
            Builder::new(move |app, context| {
                let value = SharedAppData::get_value(app, context, "count", {
                    let inits = Rc::clone(&inits);
                    move || {
                        inits.set(inits.get() + 1);
                        7
                    }
                });
                seen.borrow_mut().push(value);
                SizedBox::shrink().into_widget()
            })
        };
        let harness = Harness::mount(
            &mut app,
            SharedAppData::new(
                crate::widgets::basic::Column::new()
                    .children([probe(&seen).into_widget(), probe(&seen).into_widget()]),
            )
            .into_widget(),
        );
        harness.pump(&mut app);
        assert_eq!(*seen.borrow(), [7, 7]);
        assert_eq!(inits.get(), 1, "the second reader finds the value");
    }
}
