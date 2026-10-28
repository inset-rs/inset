//! Opening: `NSWorkspace`'s. A link or a file opens in the app Launch Services gives its kind,
//! asked first (`URLForApplicationToOpenURL:`), so that no app is told from a failure; a file is
//! revealed in the Finder, selected in its folder (`activateFileViewerSelectingURLs:`). Each call
//! hands the work on and answers at once.

use std::path::Path;
use std::rc::Rc;

use inset_embedder::{OpenError, OpenFuture, Opener};
use objc2_app_kit::NSWorkspace;
use objc2_foundation::{NSArray, NSString, NSURL};

use super::AppKit;
use crate::file_dialogs::ParentWindows;
use crate::os::SystemOpener;

impl SystemOpener for AppKit {
    fn opener(_windows: ParentWindows) -> Option<Rc<dyn Opener>> {
        Some(Rc::new(Workspace))
    }
}

/// `NSWorkspace`, which any thread may ask.
struct Workspace;

impl Opener for Workspace {
    fn open_link(&self, link: &str) -> OpenFuture {
        answered(match NSURL::URLWithString(&NSString::from_str(link)) {
            Some(url) => open(&url),
            None => Err(OpenError::Failed(format!("{link} is no link"))),
        })
    }

    fn open_file(&self, path: &Path) -> OpenFuture {
        answered(match file_url(path) {
            Ok(url) => open(&url),
            Err(error) => Err(error),
        })
    }

    fn reveal_file(&self, path: &Path) -> OpenFuture {
        answered(file_url(path).map(|url| {
            NSWorkspace::sharedWorkspace()
                .activateFileViewerSelectingURLs(&NSArray::from_retained_slice(&[url]));
        }))
    }

    fn can_open_link(&self, link: &str) -> Option<bool> {
        let url = NSURL::URLWithString(&NSString::from_str(link))?;
        Some(
            NSWorkspace::sharedWorkspace()
                .URLForApplicationToOpenURL(&url)
                .is_some(),
        )
    }
}

/// Opens `url` in the app Launch Services gives it.
fn open(url: &NSURL) -> Result<(), OpenError> {
    let workspace = NSWorkspace::sharedWorkspace();
    if workspace.URLForApplicationToOpenURL(url).is_none() {
        return Err(OpenError::NoApp);
    }
    if workspace.openURL(url) {
        Ok(())
    } else {
        Err(OpenError::Failed(
            "Launch Services did not open it".to_owned(),
        ))
    }
}

/// The URL of the file at `path`, which must be there.
fn file_url(path: &Path) -> Result<objc2::rc::Retained<NSURL>, OpenError> {
    if !path.exists() {
        return Err(OpenError::Failed(format!(
            "{} is not there",
            path.display()
        )));
    }
    NSURL::from_file_path(path)
        .ok_or_else(|| OpenError::Failed(format!("{} has no URL", path.display())))
}

fn answered(result: Result<(), OpenError>) -> OpenFuture {
    Box::pin(std::future::ready(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_with_an_app_can_be_opened_and_a_missing_file_fails() {
        let workspace = Workspace;
        assert_eq!(workspace.can_open_link("https://example.org"), Some(true));
        assert_eq!(
            workspace.can_open_link("inset-no-such-scheme-1234:anything"),
            Some(false)
        );
        let missing = Path::new("/no/such/inset/file");
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        let answer = workspace.open_file(missing).as_mut().poll(&mut context);
        assert!(matches!(
            answer,
            std::task::Poll::Ready(Err(OpenError::Failed(_)))
        ));
    }
}
