# inset-embedder-web

Browser host: one canvas, `requestAnimationFrame`, WebGPU.

An application depends on `inset-embedder-default`, not this crate. That crate picks winit on native and this host on wasm. `DefaultEmbedder::run` is the same start closure on both. The page must have a canvas whose id is `inset` (or pass another id to `canvas_id`).

Chrome or Edge with WebGPU.

## Run the gallery

From the repository root:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
cd examples/cupertino_gallery
wasm-pack build --target web --no-default-features
python3 -m http.server
```

`--no-default-features` skips the gallery's native binary so wasm-pack builds only the library. Open `http://127.0.0.1:8000/`. The page is `index.html`; the wasm module lands in `pkg/`.

Latin is bundled in the wasm. Other writing systems (CJK and the rest) load from Google Fonts the first time they appear.
