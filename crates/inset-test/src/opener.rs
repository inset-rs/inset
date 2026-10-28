//! The system's opener in memory, as a host opens nothing: it keeps each link and file it was asked
//! to open or reveal, and answers what the test gave.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use inset_embedder::{OpenError, OpenFuture, Opener};

/// An opener that keeps what it was asked and answers what the test gave: every link and file
/// opens, until the test says otherwise. Every clone is the same opener.
#[derive(Clone, Default)]
pub struct MockOpener(Rc<OpenerState>);

#[derive(Default)]
struct OpenerState {
    asked: RefCell<Vec<Opened>>,
    answer: RefCell<Option<OpenError>>,
    can_open: RefCell<Option<bool>>,
}

/// What the app asked the opener.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Opened {
    Link(String),
    File(PathBuf),
    Revealed(PathBuf),
}

impl MockOpener {
    pub fn new() -> MockOpener {
        MockOpener::default()
    }

    /// Fails every open and reveal with `error`.
    pub fn failing(self, error: OpenError) -> MockOpener {
        *self.0.answer.borrow_mut() = Some(error);
        self
    }

    /// Answers `can` whether a link can be opened; `None`, the default, as a system that will not
    /// say.
    pub fn answering_can_open(self, can: Option<bool>) -> MockOpener {
        *self.0.can_open.borrow_mut() = can;
        self
    }

    /// What the app asked to open or reveal, in order.
    pub fn asked(&self) -> Vec<Opened> {
        self.0.asked.borrow().clone()
    }

    fn answer(&self, asked: Opened) -> OpenFuture {
        self.0.asked.borrow_mut().push(asked);
        let answer = match self.0.answer.borrow().clone() {
            Some(error) => Err(error),
            None => Ok(()),
        };
        Box::pin(std::future::ready(answer))
    }
}

impl Opener for MockOpener {
    fn open_link(&self, link: &str) -> OpenFuture {
        self.answer(Opened::Link(link.to_owned()))
    }

    fn open_file(&self, path: &Path) -> OpenFuture {
        self.answer(Opened::File(path.to_owned()))
    }

    fn reveal_file(&self, path: &Path) -> OpenFuture {
        self.answer(Opened::Revealed(path.to_owned()))
    }

    fn can_open_link(&self, _link: &str) -> Option<bool> {
        *self.0.can_open.borrow()
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    use super::*;

    fn now<T>(future: impl Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("answers at once"),
        }
    }

    #[test]
    fn the_opener_keeps_what_it_was_asked_and_answers_as_told() {
        let opener = MockOpener::new();
        assert_eq!(now(opener.open_link("https://example.org")), Ok(()));
        assert_eq!(now(opener.reveal_file(Path::new("/tmp/a"))), Ok(()));
        assert_eq!(
            opener.asked(),
            [
                Opened::Link("https://example.org".to_owned()),
                Opened::Revealed(PathBuf::from("/tmp/a"))
            ]
        );
        assert_eq!(opener.can_open_link("https://example.org"), None);
        let failing = MockOpener::new()
            .failing(OpenError::NoApp)
            .answering_can_open(Some(false));
        assert_eq!(
            now(failing.open_file(Path::new("/tmp/b"))),
            Err(OpenError::NoApp)
        );
        assert_eq!(failing.can_open_link("x-none:a"), Some(false));
    }
}
