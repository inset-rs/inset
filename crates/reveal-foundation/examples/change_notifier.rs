//! How [`ChangeNotifier`] is used. Run with:
//!
//! ```text
//! cargo run -p reveal-foundation --example change_notifier
//! ```
//!
//! Dart counterpart: `CounterModel with ChangeNotifier` in
//! `listenable_builder.2.dart`.

use reveal_foundation::{
    App, ChangeNotifier, ChangeNotifierState, Handle, Listenable, Listener, ValueListenable,
    ValueNotifier,
};

/// Dart: `class Counter with ChangeNotifier`.
struct Counter {
    notifier: ChangeNotifierState,
    count: i32,
}

impl Counter {
    fn new() -> Counter {
        Counter {
            notifier: ChangeNotifierState::new(),
            count: 0,
        }
    }
}

impl ChangeNotifier for Counter {
    fn notifier_state(&mut self) -> &mut ChangeNotifierState {
        &mut self.notifier
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

/// A listener that is a method on another object (Dart's `addListener(rebuild)`).
struct Label {
    counter: Handle<Counter>,
    shown: i32,
}

fn rebuild(this: Handle<Label>, app: &mut App) {
    let counter = app.get(this).counter;
    let count = app.get(counter).count;
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

    let mut app = App::new();
    let counter = app.create(Counter::new());
    let label = app.create(Label { counter, shown: 0 });

    // Closure listener: keep the handle; a second `Listener::new` would not match.
    let on_print = Listener::new(move |app| {
        println!("  print       = {}", app.get(counter).count);
    });
    app.get_mut(counter).notifier.add_listener(on_print.clone());

    // Method tear-off: rebuild at the removal site with the same handle and function.
    app.get_mut(counter)
        .notifier
        .add_listener(Listener::handle_method(label, rebuild));

    counter.increment(&mut app);
    counter.increment(&mut app);

    app.get_mut(counter).notifier.remove_listener(&on_print);
    app.get_mut(counter)
        .notifier
        .remove_listener(&Listener::handle_method(label, rebuild));

    println!("  (listeners removed)");
    counter.increment(&mut app);
}

fn value_notifier() {
    println!("ValueNotifier");

    let mut app = App::new();
    let count = app.create(ValueNotifier::new(0i32));

    let on_print = Listener::new(move |app| {
        println!("  value       = {}", *app.get(count).value());
    });
    app.get_mut(count).add_listener(on_print);

    count.set_value(&mut app, 1);
    count.set_value(&mut app, 1); // equal: no notify
    count.set_value(&mut app, 7);
}
