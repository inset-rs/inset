# waft-embedder-web

Browser host: one canvas, `requestAnimationFrame`, WebGPU.

An application depends on `waft-embedder-default`, not this crate. That crate picks winit on native and this host on wasm. `DefaultEmbedder::run` is the same start closure on both. The page must have a canvas whose id is `waft` (or pass another id to `canvas_id`).

Chrome or Edge with WebGPU.

## Run the gallery

From the repository root:

```sh
cargo install --path crates/waft-cli
cargo waft run -d web -p cupertino-gallery
```

The first run downloads `wasm-bindgen` and adds nothing else; `cargo waft doctor` names any missing Rust target. `cargo waft build web -p cupertino-gallery` writes the static folder to `target/waft/web/release/`. The gallery's own `index.html` is the page; an app without one gets a generated page with the canvas.

Latin is bundled in the wasm. Other writing systems (CJK and the rest) load from Google Fonts the first time they appear.
