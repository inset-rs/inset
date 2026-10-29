//! Opening: the desktop portal's `OpenURI`, which opens a link (`OpenURI`) or a file (`OpenFile`)
//! in the app the desktop chooses, asking the user where it has no default, and shows a file's
//! folder in the file manager (`OpenDirectory`). Each asks on a thread of its own and answers once
//! the portal has; the opener is `None` where the bus has no portal. The portal will not say
//! whether some app opens a link.

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::rc::Rc;

use inset_embedder::{OpenError, OpenFuture, Opener};
use zbus::zvariant::{Fd, OwnedValue};

use super::Linux;
use super::portal::{self, Response, owned};
use crate::answer::answer;
use crate::file_dialogs::ParentWindows;
use crate::os::SystemOpener;

const OPEN_URI: &str = "org.freedesktop.portal.OpenURI";

impl SystemOpener for Linux {
    fn opener(_windows: ParentWindows) -> Option<Rc<dyn Opener>> {
        portal::is_on_the_bus().then(|| Rc::new(PortalOpener) as Rc<dyn Opener>)
    }
}

/// The portal's opener.
struct PortalOpener;

impl Opener for PortalOpener {
    fn open_link(&self, link: &str) -> OpenFuture {
        let link = link.to_owned();
        on_a_thread(move || {
            portal::request(OPEN_URI, "OpenURI", HashMap::new(), |options| {
                ("", link.as_str(), options)
            })
        })
    }

    fn open_file(&self, path: &Path) -> OpenFuture {
        with_file(path, "OpenFile")
    }

    fn reveal_file(&self, path: &Path) -> OpenFuture {
        with_file(path, "OpenDirectory")
    }

    fn can_open_link(&self, _link: &str) -> Option<bool> {
        None
    }
}

/// Hands the portal's `method` the file at `path`, as the descriptor it asks for.
fn with_file(path: &Path, method: &'static str) -> OpenFuture {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) => {
            return Box::pin(std::future::ready(Err(OpenError::Failed(format!(
                "{}: {error}",
                path.display()
            )))));
        }
    };
    on_a_thread(move || {
        let mut options: HashMap<&'static str, OwnedValue> = HashMap::new();
        if method == "OpenFile" {
            options.insert("ask", owned(false));
        }
        portal::request(OPEN_URI, method, options, |options| {
            ("", Fd::from(&file), options)
        })
    })
}

/// Asks the portal on a thread of its own, and answers what it says.
fn on_a_thread(ask: impl FnOnce() -> zbus::Result<Response> + Send + 'static) -> OpenFuture {
    let (answer, answered) = answer(Err(OpenError::Failed(
        "the portal ended without an answer".to_owned(),
    )));
    let started = std::thread::Builder::new()
        .name("inset-opener".to_owned())
        .spawn(move || {
            answer.give(match ask() {
                Ok(Response { code: 0, .. }) => Ok(()),
                Ok(Response { code: 1, .. }) => {
                    Err(OpenError::Failed("the user chose no app".to_owned()))
                }
                Ok(_) => Err(OpenError::Failed("the portal could not open it".to_owned())),
                Err(error) => Err(OpenError::Failed(error.to_string())),
            });
        });
    if let Err(error) = started {
        return Box::pin(std::future::ready(Err(OpenError::Failed(
            error.to_string(),
        ))));
    }
    Box::pin(answered)
}
