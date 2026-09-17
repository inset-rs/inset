//! `#[inset::main]`: one setup function, every host's entry point.
//!
//! The attribute keeps the annotated function as the app's setup and adds a
//! `main` beside it that starts the default embedder and the shell, for whoever
//! calls it: the binary's `src/main.rs` on native, the page after it has
//! instantiated the module in the browser, where `main` is an exported symbol.
//! A component for a WASI host, such as wapk, is not a host Inset ships: there
//! the macro emits no entry and leaves the function as written, for the entry
//! the app writes itself with that host's embedder crate.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Expr, ExprLit, Ident, ItemFn, Lit, LitStr, Meta, Token, Visibility};

/// Marks the app's setup function.
///
/// ```ignore
/// #[inset::main(title = "Counter", size = [420.0, 720.0])]
/// fn main(app: &mut App) {
///     run_app(app, CupertinoApp::new().home(Home).into_widget());
/// }
/// ```
///
/// `title` and `size` configure the native window and default to the package
/// name and the embedder's default size. The browser host ignores both; its
/// page calls the exported `main` once the module is instantiated. For a WASI
/// host no `main` is emitted and the function keeps its name, for the app's
/// own entry to call through that host's embedder crate.
#[proc_macro_attribute]
pub fn main(attr: TokenStream, item: TokenStream) -> TokenStream {
    let options = syn::parse_macro_input!(attr as Options);
    let mut setup = syn::parse_macro_input!(item as ItemFn);
    let as_written = setup.clone();
    let setup_name = Ident::new("__inset_setup", Span::call_site());
    setup.sig.ident = setup_name.clone();
    setup.vis = Visibility::Inherited;

    let title = match options.title {
        Some(title) => quote!(#title),
        None => quote!(::core::env!("CARGO_PKG_NAME")),
    };
    let view = match options.size {
        Some(size) => quote! {
            ::inset::ImplicitViewConfig {
                title: ::std::string::ToString::to_string(#title),
                logical_size: #size,
            }
        },
        None => quote! {
            ::inset::ImplicitViewConfig {
                title: ::std::string::ToString::to_string(#title),
                ..::core::default::Default::default()
            }
        },
    };

    quote! {
        #[cfg(not(all(target_arch = "wasm32", target_os = "wasi")))]
        #setup

        // Unused there unless the app's own entry calls it: no entry of ours does.
        #[cfg(all(target_arch = "wasm32", target_os = "wasi"))]
        #[allow(dead_code)]
        #as_written

        #[cfg(not(target_arch = "wasm32"))]
        pub fn main() {
            ::inset::DefaultEmbedder::default()
                .implicit_view(::core::option::Option::Some(#view))
                .run(|platform| ::inset::Shell::new(platform, #setup_name));
        }

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        #[unsafe(no_mangle)]
        pub extern "C" fn main() {
            ::inset::DefaultEmbedder::default()
                .run(|platform| ::inset::Shell::new(platform, #setup_name));
        }
    }
    .into()
}

struct Options {
    title: Option<LitStr>,
    size: Option<Expr>,
}

impl Parse for Options {
    fn parse(input: ParseStream) -> syn::Result<Options> {
        let mut options = Options {
            title: None,
            size: None,
        };
        for meta in Punctuated::<Meta, Token![,]>::parse_terminated(input)? {
            let Meta::NameValue(pair) = meta else {
                return Err(syn::Error::new_spanned(
                    meta,
                    "expected `title = \"…\"` or `size = [width, height]`",
                ));
            };
            if pair.path.is_ident("title") {
                match pair.value {
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(title),
                        ..
                    }) => options.title = Some(title),
                    other => {
                        return Err(syn::Error::new_spanned(
                            other,
                            "`title` takes a string literal",
                        ));
                    }
                }
            } else if pair.path.is_ident("size") {
                options.size = Some(pair.value);
            } else {
                return Err(syn::Error::new_spanned(
                    pair.path,
                    "unknown option; expected `title` or `size`",
                ));
            }
        }
        Ok(options)
    }
}
