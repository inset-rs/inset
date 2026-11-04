//! A view the app adds on a composition layer of its own, as WaOS shows a prompt in a program's
//! window: the app makes the layer, places it in its window with `MacOs::place_layer_in`, adds a
//! view drawing into it at the size it placed it at, and forwards to it the clicks the window takes
//! where the layer is. The added view counts them; the window around it is the framework's too.
//!
//! macOS only, where the host draws a view into a composition layer.
//!
//! ```sh
//! cargo run -p view_on_layer
//! ```
#![feature(arbitrary_self_types)]

#[cfg(target_os = "macos")]
mod app;
#[cfg(target_os = "macos")]
mod window_layer;

#[cfg(target_os = "macos")]
fn main() {
    app::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("view_on_layer: the host adds a view on a layer on macOS only");
}
