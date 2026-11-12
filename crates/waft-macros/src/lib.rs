//! `#[waft::main]`: one setup function, every host's entry point.
//!
//! The attribute keeps your function as the app's setup and adds the entry point beside it,
//! which starts the default embedder and the shell. Which entry depends on the target:
//!
//! - desktop and iOS: `main`
//! - web: an exported `main`, called once the page has the module
//! - Android: `android_main`, called once a process by Waft's Android host with the `AndroidApp`
//!
//! A WASI component, such as one for wapk, gets no entry: Waft ships no host for it, so
//! the app writes its own with that host's embedder crate.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Ident, ItemFn, Visibility};

/// Marks the app's setup function.
///
/// ```ignore
/// #[waft::main]
/// fn main(app: &mut App) {
///     run_app(app, CupertinoApp::new().home(Home).into_widget());
/// }
/// ```
///
/// The attribute takes no options: the window a desktop makes for the app is
/// the setup's to say, through `run_app_with`. The browser host's page calls
/// the exported `main` once the module is instantiated. Android's entry is
/// `android_main`, and there is no `main`. For a WASI host no `main` is
/// emitted and the function keeps its name, for the app's own entry to call
/// through that host's embedder crate.
#[proc_macro_attribute]
pub fn main(attr: TokenStream, item: TokenStream) -> TokenStream {
    if !attr.is_empty() {
        return syn::Error::new(
            Span::call_site(),
            "`#[waft::main]` takes no options; pass the window's title and size to `run_app_with`",
        )
        .to_compile_error()
        .into();
    }
    let mut setup = syn::parse_macro_input!(item as ItemFn);
    let as_written = setup.clone();
    let setup_name = Ident::new("__waft_setup", Span::call_site());
    setup.sig.ident = setup_name.clone();
    setup.vis = Visibility::Inherited;

    quote! {
        #[cfg(not(all(target_arch = "wasm32", target_os = "wasi")))]
        #setup

        // Unused there unless the app's own entry calls it: no entry of ours does.
        #[cfg(all(target_arch = "wasm32", target_os = "wasi"))]
        #[allow(dead_code)]
        #as_written

        #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
        pub fn main() {
            ::waft::DefaultEmbedder::default()
                .run(|platform| ::waft::Shell::new(platform, #setup_name));
        }

        // Waft's Android host calls this symbol on the app's thread, once a process, with the
        // `AndroidApp` the loop runs for. Each activity's window is a view, which the system
        // sizes and titles.
        #[cfg(target_os = "android")]
        #[unsafe(no_mangle)]
        fn android_main(app: ::waft::AndroidApp) {
            ::waft::DefaultEmbedder::default()
                .android_app(app)
                .run(|platform| ::waft::Shell::new(platform, #setup_name));
        }

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        #[unsafe(no_mangle)]
        pub extern "C" fn main() {
            ::waft::DefaultEmbedder::default()
                .run(|platform| ::waft::Shell::new(platform, #setup_name));
        }
    }
    .into()
}
