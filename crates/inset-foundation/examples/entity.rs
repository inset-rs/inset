//! User stores: [`Entity`], [`notify`](inset_foundation::Context::notify), and
//! [`emit`](inset_foundation::Context::emit). Run with:
//!
//! ```text
//! cargo run -p inset-foundation --example entity
//! ```

use inset_foundation::{AppCell, Context, EventEmitter};

struct Counter {
    count: i32,
}

struct Doubler {
    count: i32,
}

struct Bumped {
    by: i32,
}

impl EventEmitter<Bumped> for Counter {}

fn main() {
    let cell = AppCell::new();
    let mut app = cell.borrow_mut();

    let counter = app.new_entity(|_cx| Counter { count: 1 });
    let doubler = app.new_entity(|cx: &mut Context<Doubler>| {
        cx.observe(&counter, |doubler, counter, cx| {
            doubler.count = counter.read(cx).count * 2;
        })
        .detach();
        cx.subscribe(&counter, |doubler, _counter, event: &Bumped, _cx| {
            println!("  doubler heard Bumped({})", event.by);
            let _ = doubler;
        })
        .detach();
        Doubler { count: 0 }
    });

    println!(
        "after new:    counter={} doubler={}",
        counter.read(&app).count,
        doubler.read(&app).count
    );

    counter.update(&mut app, |counter, cx| {
        counter.count += 1;
        cx.emit(Bumped { by: 1 });
        cx.notify();
    });

    println!(
        "after notify: counter={} doubler={}",
        counter.read(&app).count,
        doubler.read(&app).count
    );
}
