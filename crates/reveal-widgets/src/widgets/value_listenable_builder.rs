//! Flutter counterpart: `widgets/value_listenable_builder.dart`.

use std::fmt;
use std::rc::Rc;

use reveal_foundation::{App, Handle, Listener, ValueListenable};

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};

/// Builds a `Widget` when given a concrete value of a `ValueListenable<T>`.
///
/// If the `child` parameter provided to the [`ValueListenableBuilder`] is not null, the same
/// `child` widget is passed back to this [`ValueWidgetBuilder`] and should typically be
/// incorporated in the returned widget tree.
///
/// See also:
///
///  * [`ValueListenableBuilder`], a widget which invokes this builder each time a
///    `ValueListenable` changes value.
pub type ValueWidgetBuilder<T> =
    Rc<dyn Fn(&mut App, BuildContext, &T, Option<&WidgetRef>) -> WidgetRef>;

/// A widget whose content stays synced with a `ValueListenable`.
///
/// Given a `ValueListenable<T>` and a [`builder`](Self::builder) which builds widgets from
/// concrete values of `T`, this class will automatically register itself as a listener of
/// the `ValueListenable` and call the [`builder`](Self::builder) with updated values when the
/// value changes.
///
/// ## Performance optimizations
///
/// If your [`builder`](Self::builder) function contains a subtree that does not depend on the
/// value of the `ValueListenable`, it's more efficient to build that subtree once instead of
/// rebuilding it on every value change.
///
/// If you pass the pre-built subtree as the [`child`](Self::child) parameter, the
/// [`ValueListenableBuilder`] will pass it back to your builder function so that you can
/// incorporate it into your build.
///
/// Using this pre-built child is entirely optional, but can improve performance
/// significantly in some cases and is therefore a good practice.
pub struct ValueListenableBuilder<T: Clone + 'static> {
    pub key: Option<KeyRef>,
    /// The `ValueListenable` whose value you depend on in order to build.
    ///
    /// This widget does not ensure that the `ValueListenable`'s value is not null, therefore
    /// your [`builder`](Self::builder) may need to handle null values.
    pub value_listenable: Rc<dyn ValueListenable<T>>,
    /// A [`ValueWidgetBuilder`] which builds a widget depending on the
    /// [`value_listenable`](Self::value_listenable)'s value.
    ///
    /// Can incorporate a [`value_listenable`](Self::value_listenable) value-independent
    /// widget subtree from the [`child`](Self::child) parameter into the returned widget
    /// tree.
    pub builder: ValueWidgetBuilder<T>,
    /// A [`value_listenable`](Self::value_listenable)-independent widget which is passed back
    /// to the [`builder`](Self::builder).
    ///
    /// This argument is optional and can be null if the entire widget subtree the
    /// [`builder`](Self::builder) builds depends on the value of the
    /// [`value_listenable`](Self::value_listenable). For example, in the case where the
    /// [`value_listenable`](Self::value_listenable) is a `String` and the
    /// [`builder`](Self::builder) returns a `Text` widget with the current `String` value,
    /// there would be no useful [`child`](Self::child).
    pub child: Option<WidgetRef>,
}

impl<T: Clone + 'static> ValueListenableBuilder<T> {
    /// Creates a [`ValueListenableBuilder`].
    pub fn new(
        value_listenable: Rc<dyn ValueListenable<T>>,
        builder: impl Fn(&mut App, BuildContext, &T, Option<&WidgetRef>) -> WidgetRef + 'static,
    ) -> ValueListenableBuilder<T> {
        ValueListenableBuilder {
            key: None,
            value_listenable,
            builder: Rc::new(builder),
            child: None,
        }
    }

    /// Dart `ValueListenableBuilder(key:)`.
    pub fn key(mut self, key: KeyRef) -> ValueListenableBuilder<T> {
        self.key = Some(key);
        self
    }

    /// Dart `ValueListenableBuilder(child:)`.
    pub fn child<K>(mut self, child: impl IntoWidget<K>) -> ValueListenableBuilder<T> {
        self.child = Some(child.into_widget());
        self
    }
}

impl<T: Clone + 'static> fmt::Debug for ValueListenableBuilder<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ValueListenableBuilder")
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl<T: Clone + 'static> StatefulWidget for ValueListenableBuilder<T> {
    type State = ValueListenableBuilderState<T>;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> ValueListenableBuilderState<T> {
        ValueListenableBuilderState {
            state: StateData::new(),
            value: None,
        }
    }
}

/// Dart's `_ValueListenableBuilderState`.
pub struct ValueListenableBuilderState<T: Clone + 'static> {
    state: StateData<ValueListenableBuilder<T>>,
    /// Dart's `late T value`, read in `init_state`.
    value: Option<T>,
}

impl<T: Clone + 'static> ValueListenableBuilderState<T> {
    fn value_changed(self: Handle<Self>, app: &mut App) {
        let value = self.widget(app).value_listenable.value(app).clone();
        self.set_state(app, |state| state.value = Some(value));
    }
}

impl<T: Clone + 'static> State for ValueListenableBuilderState<T> {
    type Widget = ValueListenableBuilder<T>;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let listenable = Rc::clone(&self.widget(app).value_listenable);
        app.get_mut(self).value = Some(listenable.value(app).clone());
        listenable.add_listener(app, Listener::handle_method(self, Self::value_changed));
    }

    fn did_update_widget(
        self: Handle<Self>,
        app: &mut App,
        old_widget: &ValueListenableBuilder<T>,
    ) {
        let listenable = Rc::clone(&self.widget(app).value_listenable);
        if !Rc::ptr_eq(&old_widget.value_listenable, &listenable) {
            old_widget
                .value_listenable
                .remove_listener(app, &Listener::handle_method(self, Self::value_changed));
            app.get_mut(self).value = Some(listenable.value(app).clone());
            listenable.add_listener(app, Listener::handle_method(self, Self::value_changed));
        }
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let listenable = Rc::clone(&self.widget(app).value_listenable);
        listenable.remove_listener(app, &Listener::handle_method(self, Self::value_changed));
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let (builder, child) = {
            let widget = self.widget(app);
            (Rc::clone(&widget.builder), widget.child.clone())
        };
        let value = app.get(self).value.clone().expect("read in init_state");
        builder(app, context, &value, child.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use std::cell::RefCell;

    use reveal_foundation::ValueNotifier;

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;

    #[test]
    fn the_builder_sees_each_value_and_the_child_passes_through() {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        let notifier = app.create(ValueNotifier::new(1));
        let seen = Rc::new(RefCell::new(Vec::new()));
        let child: WidgetRef = SizedBox::shrink().into_widget();
        let widget = ValueListenableBuilder::new(Rc::new(notifier), {
            let seen = Rc::clone(&seen);
            let child = child.clone();
            move |_app, _context, value: &i32, passed| {
                assert!(passed.is_some_and(|passed| Rc::ptr_eq(passed, &child)));
                seen.borrow_mut().push(*value);
                passed.expect("child").clone()
            }
        })
        .child(child.clone());
        let harness = Harness::mount(&mut app, widget.into_widget());
        harness.pump(&mut app);
        notifier.set_value(&mut app, 2);
        harness.pump(&mut app);
        notifier.set_value(&mut app, 2); // unchanged: no notification
        harness.pump(&mut app);
        assert_eq!(*seen.borrow(), vec![1, 2]);
    }
}
