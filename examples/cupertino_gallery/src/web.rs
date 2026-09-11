//! Browser entry: the same gallery library on the web canvas host.

use crate::run_gallery;
use reveal_embedder_default::DefaultEmbedder;
use reveal_shell::Shell;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(start)]
pub fn start() {
    DefaultEmbedder::default()
        .run(move |platform| Shell::new(platform, move |app| run_gallery(app, None)));
}
