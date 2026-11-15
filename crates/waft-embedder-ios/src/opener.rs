//! Opening: UIKit's. A link opens in the app that handles its scheme (`openURL:options:`), and a
//! file through the system's Open In menu, which lists the apps that open its kind
//! (`UIDocumentInteractionController`), from a view in sight (`Views::presenter`). iOS has no file manager to
//! reveal a file in, so a reveal fails. Whether some app opens a link is asked only for a scheme
//! the app declared (`LSApplicationQueriesSchemes`), as iOS answers no other; the rest is `None`.

// UIKit's open call takes its options untyped, which needs `unsafe`; this module writes it.
#![allow(unsafe_code)]

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool};
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_foundation::{NSArray, NSBundle, NSDictionary, NSPoint, NSRect, NSSize, NSString, NSURL};
use objc2_ui_kit::{UIApplication, UIDocumentInteractionController};
use waft_embedder::{OpenError, OpenFuture, Opener};

use waft_embedder_apple::answer::answer;

use crate::views::Views;

thread_local! {
    /// The last Open In menu's controller, kept while its menu may show. Main thread.
    static SHOWING: RefCell<Option<Retained<UIDocumentInteractionController>>> =
        const { RefCell::new(None) };
}

/// UIKit's opener, which shows its menus from the app's views.
pub(crate) struct UiKitOpener {
    views: Rc<Views>,
}

impl UiKitOpener {
    pub(crate) fn new(views: Rc<Views>) -> UiKitOpener {
        UiKitOpener { views }
    }
}

impl Opener for UiKitOpener {
    fn open_link(&self, link: &str) -> OpenFuture {
        let Some(url) = NSURL::URLWithString(&NSString::from_str(link)) else {
            return failed(format!("{link} is no link"));
        };
        let (answer, answered) = answer(Err(OpenError::Failed(
            "UIKit ended without an answer".to_owned(),
        )));
        let answer = RefCell::new(Some(answer));
        let completion = RcBlock::new(move |opened: Bool| {
            if let Some(answer) = answer.borrow_mut().take() {
                answer.give(if opened.as_bool() {
                    Ok(())
                } else {
                    Err(OpenError::NoApp)
                });
            }
        });
        let Some(main_thread) = MainThreadMarker::new() else {
            return failed(format!("{link} asked to open off the main thread"));
        };
        let options: Retained<NSDictionary<NSString, AnyObject>> = NSDictionary::new();
        // SAFETY: an empty dictionary of the options' type, and a completion UIKit calls once.
        unsafe {
            UIApplication::sharedApplication(main_thread).openURL_options_completionHandler(
                &url,
                &options,
                Some(&completion),
            );
        }
        Box::pin(answered)
    }

    fn open_file(&self, path: &Path) -> OpenFuture {
        if !path.exists() {
            return failed(format!("{} is not there", path.display()));
        }
        let Some(url) = NSURL::from_file_path(path) else {
            return failed(format!("{} has no URL", path.display()));
        };
        let Some(presenter) = self.views.presenter(None) else {
            return failed("no view to show the Open In menu from".to_owned());
        };
        let Some(view) = presenter.view() else {
            return failed("no view to show the Open In menu from".to_owned());
        };
        let controller =
            UIDocumentInteractionController::interactionControllerWithURL(&url, presenter.mtm());
        let bounds = view.bounds();
        let centre = NSRect::new(
            NSPoint::new(bounds.size.width / 2.0, bounds.size.height / 2.0),
            NSSize::new(1.0, 1.0),
        );
        let shown = controller.presentOpenInMenuFromRect_inView_animated(centre, &view, true);
        SHOWING.with_borrow_mut(|showing| *showing = Some(controller));
        Box::pin(std::future::ready(if shown {
            Ok(())
        } else {
            Err(OpenError::NoApp)
        }))
    }

    fn reveal_file(&self, path: &Path) -> OpenFuture {
        failed(format!(
            "iOS has no file manager to show {} in",
            path.display()
        ))
    }

    fn can_open_link(&self, link: &str) -> Option<bool> {
        let (scheme, _) = link.split_once(':')?;
        if !declared_schemes()
            .iter()
            .any(|declared| declared.eq_ignore_ascii_case(scheme))
        {
            return None;
        }
        let url = NSURL::URLWithString(&NSString::from_str(link))?;
        let main_thread = MainThreadMarker::new()?;
        Some(UIApplication::sharedApplication(main_thread).canOpenURL(&url))
    }
}

/// The schemes the app declared it asks about, in its `Info.plist`.
fn declared_schemes() -> Vec<String> {
    let bundle = NSBundle::mainBundle();
    let Some(declared) =
        bundle.objectForInfoDictionaryKey(&NSString::from_str("LSApplicationQueriesSchemes"))
    else {
        return Vec::new();
    };
    let Ok(schemes) = declared.downcast::<NSArray>() else {
        return Vec::new();
    };
    schemes
        .iter()
        .filter_map(|scheme| scheme.downcast::<NSString>().ok())
        .map(|scheme| scheme.to_string())
        .collect()
}

fn failed(reason: String) -> OpenFuture {
    Box::pin(std::future::ready(Err(OpenError::Failed(reason))))
}
