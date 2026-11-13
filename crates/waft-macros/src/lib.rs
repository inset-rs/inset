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
//!
//! The desktop entry gives the embedder the app's Linux app id, the manifest's `identifier`, which
//! [`linux_app_id!`] gives an entry the app writes itself.

mod identifier;

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
    let identifier = identifier::expression();
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
            let mut linux = ::waft::LinuxSettings::default();
            linux.app_id = ::std::option::Option::Some(::std::string::String::from(#identifier));
            ::waft::DefaultEmbedder::default()
                .linux(linux)
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

/// The app's Linux app id: `identifier` under `[package.metadata.waft]` in the crate's Cargo.toml,
/// or `com.example.<package>` without one, as `cargo waft` names the app's desktop file. A
/// `&'static str`, for an entry written by hand to give `LinuxSettings::app_id`, as
/// `#[waft::main]`'s does.
///
/// ```ignore
/// let mut linux = LinuxSettings::default();
/// linux.app_id = Some(waft::linux_app_id!().to_owned());
/// DefaultEmbedder::default()
///     .linux(linux)
///     .run(|platform| Shell::new(platform, setup));
/// ```
#[proc_macro]
pub fn linux_app_id(input: TokenStream) -> TokenStream {
    if !input.is_empty() {
        return syn::Error::new(Span::call_site(), "`linux_app_id!` takes nothing")
            .to_compile_error()
            .into();
    }
    identifier::expression().into()
}
