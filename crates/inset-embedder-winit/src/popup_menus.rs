//! The system's own popup menus, as the framework's [`PopupMenus`]: a menu opens once the
//! framework's call has returned, its loop outside every call into the app, and the entry chosen
//! reaches the app through the future `show` answered, once the menu has closed and the main loop
//! has reconciled the buttons it kept.
//!
//! The invariant: the future is answered exactly once, with the entry chosen, or with none when the
//! menu's answer is lost on the way, so the app never waits on a menu for ever.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll, Waker};

use inset_embedder::{PopupMenuEntry, PopupMenuFuture, PopupMenus};
use winit::event_loop::EventLoopProxy;

use crate::main_loop::MainLoopEvent;
use crate::os::{NativeMenus, ThisOs};

const HELD_BRIEFLY: &str = "a menu's answer is never held across a panic";

/// Opens menus in the system's own loop, and tells the event loop once each has closed.
pub(crate) struct NativePopupMenus {
    proxy: EventLoopProxy<MainLoopEvent>,
}

impl NativePopupMenus {
    pub(crate) fn new(proxy: EventLoopProxy<MainLoopEvent>) -> NativePopupMenus {
        NativePopupMenus { proxy }
    }
}

impl PopupMenus for NativePopupMenus {
    fn show(&self, entries: &[PopupMenuEntry]) -> PopupMenuFuture {
        let (answer, chosen) = menu_answer();
        let proxy = self.proxy.clone();
        ThisOs::open_popup_menu(
            entries.to_vec(),
            Box::new(move |index| {
                // The menu ran its own event loop and kept the release of the button that opened
                // it; the loop reconciles the buttons before the app hears the choice.
                let _ = proxy.send_event(MainLoopEvent::MenuClosed);
                answer.give(index);
            }),
        );
        Box::pin(chosen)
    }
}

/// The entry chosen, on its way from the menu to the app.
#[derive(Default)]
struct Answer {
    /// What the menu answered, once it has.
    chosen: Option<Option<usize>>,
    /// The app's task waiting for it.
    waiter: Option<Waker>,
}

/// The menu's side of its answer: given once, or `None` if it is dropped unused.
struct MenuAnswer(Arc<Mutex<Answer>>);

/// The app's side: the future `show` answers.
struct ChosenEntry(Arc<Mutex<Answer>>);

fn menu_answer() -> (MenuAnswer, ChosenEntry) {
    let answer = Arc::new(Mutex::new(Answer::default()));
    (MenuAnswer(Arc::clone(&answer)), ChosenEntry(answer))
}

impl MenuAnswer {
    /// The menu closed with `index` chosen, or none.
    fn give(self, index: Option<usize>) {
        answer(&self.0, index);
    }
}

impl Drop for MenuAnswer {
    /// A menu whose answer is lost answers none.
    fn drop(&mut self) {
        answer(&self.0, None);
    }
}

/// Answers `index` unless an answer was given already, and wakes the app's task.
fn answer(answer: &Mutex<Answer>, index: Option<usize>) {
    let waiter = {
        let mut answer = locked(answer);
        if answer.chosen.is_some() {
            return;
        }
        answer.chosen = Some(index);
        answer.waiter.take()
    };
    if let Some(waiter) = waiter {
        waiter.wake();
    }
}

impl Future for ChosenEntry {
    type Output = Option<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<usize>> {
        let mut answer = locked(&self.0);
        match answer.chosen {
            Some(index) => Poll::Ready(index),
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

    fn poll(chosen: &mut ChosenEntry, waker: &Arc<CountingWaker>) -> Poll<Option<usize>> {
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
