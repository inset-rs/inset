//! How [`ChangeNotifier`] is used. Run with:
//!
//! ```text
//! cargo run -p waft-foundation --example change_notifier
//! ```
//!
//! Dart counterpart: `CounterModel with ChangeNotifier` in
//! `listenable_builder.2.dart`.

use waft_foundation::AppCell;
use waft_foundation::{
    App, ChangeNotifier, ChangeNotifierData, Handle, Listenable, Listener, Ref, ValueNotifier,
};

/// Dart: `class Counter with ChangeNotifier`.
struct Counter {
    change_notifier: ChangeNotifierData,
    count: i32,
}

impl Counter {
    fn new() -> Counter {
        Counter {
            change_notifier: ChangeNotifierData::new(),
            count: 0,
        }
    }
}

impl ChangeNotifier for Counter {
    fn change_notifier_data(&self) -> &ChangeNotifierData {
        &self.change_notifier
    }

    fn change_notifier_data_mut(&mut self) -> &mut ChangeNotifierData {
        &mut self.change_notifier
    }
}

/// Inherent `impl Handle<Counter>` is an orphan: `Handle` lives in another crate.
trait CounterHandle {
    fn increment(self, app: &mut App);
}

impl CounterHandle for Handle<Counter> {
    /// Dart: `void increment() { _count += 1; notifyListeners(); }`
    fn increment(self, app: &mut App) {
        app.get_mut(self).count += 1;
        self.notify_listeners(app);
    }
}

/// A listener that is a method on another object (Dart's `addListener(rebuild)`). It keeps the
/// counter it shows: a field Dart types as `Counter` is a `Ref`.
struct Label {
    counter: Ref<Counter>,
    shown: i32,
}

fn rebuild(this: Handle<Label>, app: &mut App) {
    let count = app.get(&app.get(this).counter).count;
    app.get_mut(this).shown = count;
    println!("  label.shown = {count}");
}

fn main() {
    mixin_counter();
    println!();
    value_notifier();
}

fn mixin_counter() {
    println!("Counter (mixin-as-field)");

    let cell = AppCell::new();
    let mut app = cell.borrow_mut();
    let counter = app.create(Counter::new());
    let label = app.create(Label {
        counter: counter.clone(),
        shown: 0,
    });
    // Closure listener: keep the listener; a second `Listener::new` would not match.
    let counter_handle = counter.handle();
    let on_print = Listener::new(move |app| {
        println!("  print       = {}", app.get(counter_handle).count);
    });
    counter.add_listener(&mut app, on_print.clone());

    // Method tear-off: rebuild at the removal site with the same handle and function.
    counter.add_listener(&mut app, Listener::handle_method(label.handle(), rebuild));

    counter.increment(&mut app);
    counter.increment(&mut app);

    counter.remove_listener(&mut app, &on_print);
    counter.remove_listener(&mut app, &Listener::handle_method(label.handle(), rebuild));

    println!("  (listeners removed)");
    counter.increment(&mut app);
}

fn value_notifier() {
    println!("ValueNotifier");

    let cell = AppCell::new();
    let mut app = cell.borrow_mut();
    let count = app.create(ValueNotifier::new(0i32));

    let count_handle = count.handle();
    let on_print = Listener::new(move |app| {
        println!("  value       = {}", *app.get(count_handle).value());
    });
    count.add_listener(&mut app, on_print);

    count.set_value(&mut app, 1);
    count.set_value(&mut app, 1); // equal: no notify
    count.set_value(&mut app, 7);
}
