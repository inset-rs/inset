# Inset

Inset is a UI framework for writing solid, fluent cross-platform apps.

Inset is a full-stack Rust framework. Everything from app logic down to the render engine, [valo](https://github.com/seedeai/valo), is written in Rust and works like a normal Rust package.

The framework layer is a faithful port of [Flutter](https://github.com/flutter/flutter). Everything the framework needs from the platform sits behind a cleanly defined interface, which makes porting to a new platform really easy.

The project is in alpha, while is ready to build apps on. Inset currently has two widget libraries ready to use:
- Cupertino: for mobile apps
- WinUI: for desktop and web apps

## Try it

Inset **requires nightly Rust**. To install run:

```sh
rustup install nightly
```

Then clone the repository and run the gallery app:

```sh
cargo run -p cupertino-gallery
```

> Inset also runs in the browser. Try it at https://cupertino.inset.rs. To run it yourself, see [`inset-embedder-web`](crates/inset-embedder-web/README.md).

## Building an app with Inset

Inset keeps a clean interface between the framework and the platform (the embedder). Usually `DefaultEmbedder` is enough with good defaults.

```rust
use inset_cupertino::CupertinoApp;
use inset_embedder_default::DefaultEmbedder;
use inset_shell::Shell;
use inset_widgets::{IntoWidget, run_app};

fn main() {
    DefaultEmbedder::default().run(|platform| {
        Shell::new(platform, |app| {
            run_app(app, CupertinoApp::new().home(Counter).into_widget());
        })
    });
}
```

### Writing the UI

The framework is a faithful port of Flutter with a large number of widgets available. Use them as you would in Flutter:

```rust
use inset_foundation::App;
use inset_widgets::{BuildContext, Column, IntoWidget, StatelessWidget, Text, WidgetRef};

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
use inset_foundation::{App, Entity};
use inset_widgets::{BuildContext, IntoWidget, StatelessWidget, Text, WidgetRef};

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

## Contributing and AI

Inset is developed with strong assistance from AI agents, with humans leading the design, reviewing changes, testing, and remaining responsible for correctness.

AI-assisted contributions are welcome as long as the contributor fully reviews the work, understands how it fits the architecture, manually reviews code changes, and tests the work. Code that the contributor cannot explain, verify, or maintain should not be submitted.

How this repo is worked on: [AGENTS.md](AGENTS.md).

## License

MIT
