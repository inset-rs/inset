# Inset

Inset is a UI framework for writing solid, fluent cross-platform apps.

Inset is a full-stack Rust framework. Everything from app logic down to the render engine, [valo](https://github.com/seedeai/valo), is written in Rust and works like a normal Rust package.

The framework layer is a faithful port of [Flutter](https://github.com/flutter/flutter). Everything the framework needs from the platform sits behind a cleanly defined interface, which makes porting to a new platform really easy.

The project is in alpha, while is ready to build apps on. Inset currently has two widget libraries ready to use:
- Cupertino: for mobile apps
- WinUI: for desktop and web apps

## Try it

Inset **requires nightly Rust**: a widget's `State` takes `self: Handle<Self>`, so an app with one starts with `#![feature(arbitrary_self_types)]`. To install run:

```sh
rustup install nightly
```

Then clone the repository and run the gallery app:

```sh
cargo run -p cupertino-gallery
```

> Inset also runs in the browser. Try it at https://cupertino.inset.rs.

## Start an app

`cargo inset` is a convenient CLI for scaffolding and running app with Inset:

```sh
cargo install inset-cli
cargo inset new myapp                  # --template winui for a desktop app
cd myapp
cargo inset run                        # this desktop
cargo inset run -d ios                 # an iOS simulator
cargo inset run -d web                 # a browser
cargo inset build dmg                  # and macos, ios, ipa, msi, nsis, deb, appimage, web
```

> For more details, see [`inset-cli`](crates/inset-cli/README.md).

## Building an app with Inset

Inset keeps a clean interface between the framework and the platform (the embedder). The usual way to start an app is creating an embedder and providing a root widget to it:

```rust
use inset::{DefaultEmbedder, IntoWidget, Shell, run_app};
use inset_cupertino::CupertinoApp;

fn main() {
    DefaultEmbedder::default().run(|platform| {
        Shell::new(platform, |app| {
            run_app(app, CupertinoApp::new().home(Counter).into_widget());
        })
    });
}
```

Or you can also use the `#[inset::main]` macro to write the main function:

```rust
use inset::{App, IntoWidget, run_app};
use inset_cupertino::CupertinoApp;

#[inset::main]
fn main(app: &mut App) {
    run_app(app, CupertinoApp::new().home(Counter).into_widget());
}
```

### Writing the UI

The framework is a faithful port of Flutter with a large number of widgets available. Use them as you would in Flutter:

```rust
use inset::{App, BuildContext, Column, IntoWidget, StatelessWidget, Text, WidgetRef};

#[derive(Debug)]
struct Counter;

impl StatelessWidget for Counter {
    fn build(&self, _app: &mut App, _context: BuildContext) -> WidgetRef {
        Column::new()
            .children([
                Text::new("Counter").into_widget(),
                Text::new("0").into_widget(),
            ])
            .into_widget()
    }
}
```

### State management

Unlike Flutter, Inset has a built-in state management solution, inspired by [Zed's GPUI](https://zed.dev/blog/gpui-ownership).

App state lives in an `Entity`. Create one with `app.new_entity`, read it with `read`, and change it with `update`. A `read` inside a widget's `build` observes the entity: `notify` rebuilds that widget.


```rust
use inset::{App, BuildContext, Entity, IntoWidget, StatelessWidget, Text, WidgetRef};

let count = app.new_entity(|_cx| 0);

struct Label {
    count: Entity<i32>,
}

impl StatelessWidget for Label {
    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let n = *app.read(&self.count);
        Text::new(format!("{n}")).into_widget()
    }
}

app.update(&count, |n, cx| {
    *n += 1;
    cx.notify();
});
```

### Structuring an app

The recommended architecture is three layers:

- **Logic**: the business logic, as plain Rust. It does not depend on `App` or `Entity`, so it runs and is tested on its own.
- **State**: the app's state as entities, built on the logic layer. This is where `App`, `Entity` and `Timer` are used. Entity events model what happens: an entity emits with `cx.emit`, another subscribes and reacts. A callback from outside the app reaches an entity through `AsyncApp::post`. This layer depends on `inset-foundation` alone, so it is tested with `AppCell` and no display.
- **UI**: widgets, kept thin. A widget reads entities in `build`, which subscribes it to them, and calls their methods from callbacks. It keeps only view state, such as whether a panel is open.

A new feature is one entity plus the widgets that show it.

## Contributing and AI

Inset is developed with strong assistance from AI agents, with humans leading the design, reviewing changes, testing, and remaining responsible for correctness.

AI-assisted contributions are welcome as long as the contributor fully reviews the work, understands how it fits the architecture, manually reviews code changes, and tests the work. Code that the contributor cannot explain, verify, or maintain should not be submitted.

How this repo is worked on: [AGENTS.md](AGENTS.md).

## License

MIT
