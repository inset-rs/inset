//! The system's own menus at a point in a view, as the framework's [`PopupMenus`]: a menu opens
//! once the framework's call has returned, its loop outside every call into the app, and the item
//! chosen reaches the app through the future `show` answered, once the menu has closed and the
//! main loop has reconciled the buttons it kept. The answer of a menu the app opens at a tray
//! icon comes the same way ([`menu_answer`]).
//!
//! The invariant: the future is answered exactly once, with the item chosen, or with none when the
//! menu's answer is lost on the way, so the app never waits on a menu for ever.

use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};

use inset_embedder::{MenuEntry, MenuId, Offset, PopupMenuFuture, PopupMenus, ViewId};

use crate::menus::MenuEvents;
use crate::os::{NativeMenus, ThisOs};
use crate::state::EmbedderState;

const HELD_BRIEFLY: &str = "a menu's answer is never held across a panic";

/// Opens menus in the system's own loop, and tells the event loop once each has closed.
pub(crate) struct NativePopupMenus {
    events: MenuEvents,
    /// Where the views' windows are.
    state: Rc<EmbedderState>,
}

impl NativePopupMenus {
    pub(crate) fn new(events: MenuEvents, state: Rc<EmbedderState>) -> NativePopupMenus {
        NativePopupMenus { events, state }
    }
}

impl PopupMenus for NativePopupMenus {
    fn show(&self, view: ViewId, at: Offset, menu: &[MenuEntry]) -> PopupMenuFuture {
        let (answer, chosen) = menu_answer();
        let Some(open) = self.state.views().by_view(view) else {
            answer.give(None);
            return Box::pin(chosen);
        };
        let events = self.events.clone();
        ThisOs::open_popup_menu(
            &*open.window,
            at,
            menu.to_vec(),
            self.events.clone(),
            Box::new(move |chosen| {
                // The menu ran its own event loop and kept the release of the button that opened
                // it; the loop reconciles the buttons before the app hears the choice.
                events.closed();
                answer.give(chosen);
            }),
        );
        Box::pin(chosen)
    }
}

/// The item chosen, on its way from the menu to the app.
#[derive(Default)]
struct Answer {
    /// What the menu answered, once it has.
    chosen: Option<Option<MenuId>>,
    /// The app's task waiting for it.
    waiter: Option<Waker>,
}

/// The menu's side of its answer: given once, or `None` if it is dropped unused.
pub(crate) struct MenuAnswer(Arc<Mutex<Answer>>);

/// The app's side: the future `show` answers.
pub(crate) struct ChosenEntry(Arc<Mutex<Answer>>);

/// A menu's answer: what the menu gives, and the future the app awaits.
pub(crate) fn menu_answer() -> (MenuAnswer, ChosenEntry) {
    let answer = Arc::new(Mutex::new(Answer::default()));
    (MenuAnswer(Arc::clone(&answer)), ChosenEntry(answer))
}

impl MenuAnswer {
    /// The menu closed with `chosen`, or none.
    pub(crate) fn give(self, chosen: Option<MenuId>) {
        answer(&self.0, chosen);
    }
}

impl Drop for MenuAnswer {
    /// A menu whose answer is lost answers none.
    fn drop(&mut self) {
        answer(&self.0, None);
    }
}

/// Answers `chosen` unless an answer was given already, and wakes the app's task.
fn answer(answer: &Mutex<Answer>, chosen: Option<MenuId>) {
    let waiter = {
        let mut answer = locked(answer);
        if answer.chosen.is_some() {
            return;
        }
        answer.chosen = Some(chosen);
        answer.waiter.take()
    };
    if let Some(waiter) = waiter {
        waiter.wake();
    }
}

impl Future for ChosenEntry {
    type Output = Option<MenuId>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<MenuId>> {
        let mut answer = locked(&self.0);
        match answer.chosen {
            Some(chosen) => Poll::Ready(chosen),
            None => {
                answer.waiter = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

fn locked(answer: &Mutex<Answer>) -> MutexGuard<'_, Answer> {
    answer.lock().expect(HELD_BRIEFLY)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::Wake;

    use super::*;

    #[derive(Default)]
    struct CountingWaker(AtomicUsize);

    impl Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn poll(chosen: &mut ChosenEntry, waker: &Arc<CountingWaker>) -> Poll<Option<MenuId>> {
        let waker = Waker::from(Arc::clone(waker));
        Pin::new(chosen).poll(&mut Context::from_waker(&waker))
    }

    #[test]
    fn the_entry_chosen_reaches_the_app_once_the_menu_closes() {
        let waker = Arc::new(CountingWaker::default());
        let (answer, mut chosen) = menu_answer();
        assert_eq!(poll(&mut chosen, &waker), Poll::Pending, "the menu is open");
        answer.give(Some(2));
        assert_eq!(waker.0.load(Ordering::SeqCst), 1, "the app's task is woken");
        assert_eq!(poll(&mut chosen, &waker), Poll::Ready(Some(2)));
    }

    #[test]
    fn a_menu_whose_answer_is_lost_answers_none() {
        let waker = Arc::new(CountingWaker::default());
        let (answer, mut chosen) = menu_answer();
        drop(answer);
        assert_eq!(poll(&mut chosen, &waker), Poll::Ready(None));
    }
}
