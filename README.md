# Inset

Inset is a UI framework for solid, efficient cross-platform apps. It is written in Rust, including the render engine, [Valo](https://github.com/seedeai/valo).

The widgets are a [Flutter](https://github.com/flutter/flutter) port. Below them, Flutter’s engine interface (`dart:ui`) is a small set of host traits: size, color, a view. The framework never names a window or a GPU. Valo draws.

Alpha: the public API will change. The Cupertino gallery in this repo is a real app you can run.

Crate names are still `reveal-*`. They will become `inset-*`; `reveal` was taken on crates.io.

## Try it

```sh
cargo run -p cupertino-gallery
```

macOS, Windows, and Linux.

A browser host ships in `reveal-embedder-default` (canvas, `requestAnimationFrame`, WebGPU). Same app library, same embedder traits. An application depends on that crate instead of picking winit or the web host itself.

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
cd examples/cupertino_gallery
wasm-pack build --target web --no-default-features
```

Serve that folder (`python3 -m http.server` in `examples/cupertino_gallery`). The page is `index.html`; the wasm module lands in `pkg/`. Chrome or Edge with WebGPU. Body text is a Roboto Latin slice compiled into the wasm, so the first frame has ink; other scripts load Google Fonts' unicode-range slices of Roboto or of the script's Noto family on first use.

## Write a widget

**Cupertino** (iOS-styled) is the kit for phones. **WinUI** (Windows-styled) is the kit for the desktop. This tree ships Cupertino; WinUI is not here yet.

App state is an `Entity`. Create it with `app.new_entity`. A `read` during `build` rebuilds that widget when the store `notify`s. A tap may `update` without rebuilding until something that `read` during `build` is notified.

```rust
let count = app.new_entity(|_cx| 0);

struct Counter {
    count: Entity<i32>,
}

impl StatelessWidget for Counter {
    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let n = *self.count.read(app);
        CupertinoButton::new(
            Text::new(format!("{n}")).into_widget(),
            Some(Listener::new({
                let count = self.count.clone();
                move |app| {
                    count.update(app, |n, cx| {
                        *n += 1;
                        cx.notify();
                    });
                }
            })),
        )
        .into_widget()
    }
}
```

If you know Flutter, the widgets match the Dart. `Entity` is Inset’s, not Flutter’s.

## Architecture

Three layers:

1. **Widgets** — Flutter’s framework, in Rust.
2. **Embedder** — those host traits. Tests implement them. So does a real host.
3. **Host** — implements the traits and drives frames. Desktop is winit plus Valo. The browser is the web host behind `DefaultEmbedder`. Tests implement the same traits without a window.

Swap the host; keep the app.

## Contributing and AI

Inset is developed with strong assistance from AI agents, with humans leading the design, reviewing changes, testing, and remaining responsible for correctness.

AI-assisted contributions are welcome as long as the contributor fully reviews the work, understands how it fits the architecture, manually reviews code changes, and tests the work. Code that the contributor cannot explain, verify, or maintain should not be submitted.

How this repo is worked on: [AGENTS.md](AGENTS.md).

## License

MIT
