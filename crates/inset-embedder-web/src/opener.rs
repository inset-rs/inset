//! Opening: the browser's. A link opens in a new tab or window (`window.open`), which the browser
//! hands to the app a scheme is registered with when it is not the web's own (`mailto:`), and
//! which the page keeps no hold on: the new window's `opener` is cut, as a `noopener` link's is.
//!
//! The browser opens a window only while the page handles the user's click or key, and blocks one
//! a script opens on its own, which the call answers as a failure. A `javascript:` link is refused,
//! since it would run in the page's own origin. A page has no paths, so opening or revealing a file
//! fails; and the browser never says whether some app takes a link's scheme.

use std::path::Path;

use inset_embedder::{OpenError, OpenFuture, Opener};
use wasm_bindgen::JsValue;

use crate::file_dialogs::describe;

/// The browser's tabs and windows.
pub(crate) struct BrowserOpener;

impl Opener for BrowserOpener {
    fn open_link(&self, link: &str) -> OpenFuture {
        answered(open_window(link))
    }

    fn open_file(&self, _path: &Path) -> OpenFuture {
        answered(Err(no_paths()))
    }

    fn reveal_file(&self, _path: &Path) -> OpenFuture {
        answered(Err(no_paths()))
    }

    fn can_open_link(&self, _link: &str) -> Option<bool> {
        None
    }
}

/// Opens `link` in a new browsing context the page keeps no hold on.
fn open_window(link: &str) -> Result<(), OpenError> {
    if runs_script(link) {
        return Err(OpenError::Failed(
            "a javascript: link runs in the page's origin, not in another app".to_owned(),
        ));
    }
    let window = web_sys::window().ok_or_else(|| OpenError::Failed("no window".to_owned()))?;
    let opened = window
        .open_with_url_and_target(link, "_blank")
        .map_err(|error| OpenError::Failed(describe(&error)))?
        .ok_or_else(|| {
            OpenError::Failed(
                "the browser blocked the window, as it does outside a click".to_owned(),
            )
        })?;
    let _ = opened.set_opener(&JsValue::NULL);
    Ok(())
}

/// Whether `link` is a `javascript:` URL, as the browser parses one: after leading spaces and
/// control characters, the scheme in any case, with tabs and newlines inside it ignored.
fn runs_script(link: &str) -> bool {
    let scheme: String = link
        .trim_start_matches(|letter: char| letter <= ' ')
        .chars()
        .filter(|letter| !matches!(letter, '\t' | '\n' | '\r'))
        .take("javascript:".len())
        .collect();
    scheme.eq_ignore_ascii_case("javascript:")
}

fn no_paths() -> OpenError {
    OpenError::Failed("a page has no paths to open".to_owned())
}

fn answered(result: Result<(), OpenError>) -> OpenFuture {
    Box::pin(std::future::ready(result))
}
