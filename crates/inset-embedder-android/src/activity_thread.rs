//! An activity's thread: the `android_main` android-activity runs for each activity the system
//! creates, on a thread of its own, which alone may read the activity's events. It reads them as
//! they come and hands each to the app's thread (`app_thread`) for the view the activity is; the
//! input it reads into the framework's pointer data first, since an input event lives only while
//! its queue holds it.
//!
//! A window taken away is handed on from inside the poll, where android-activity still holds the
//! window and Android waits: the app lets go of it before Android goes on. The thread ends when the
//! activity is destroyed, once the app has let go of the view, which is when Android may finish
//! destroying it.

use android_activity::input::{InputEvent, KeyAction, KeyEvent, Keycode, MotionEvent};
use android_activity::{AndroidApp, InputStatus, MainEvent, PollEvent};
use inset_embedder::ViewId;

use crate::app_thread::{ActivityEvent, AppThread};
use crate::input::Pointers;

/// What an activity's thread holds: the activity, the view it is, and the devices pointing at it.
pub(crate) struct ActivityLoop {
    app: AndroidApp,
    view: ViewId,
    app_thread: AppThread,
    /// The devices the framework has been told about for this view.
    pointers: Pointers,
}

impl ActivityLoop {
    pub(crate) fn new(app: AndroidApp, view: ViewId, app_thread: AppThread) -> ActivityLoop {
        ActivityLoop {
            app,
            view,
            app_thread,
            pointers: Pointers::default(),
        }
    }

    /// Runs until the activity is destroyed and the app has let go of its view.
    pub(crate) fn run(mut self) {
        loop {
            let app = self.app.clone();
            let mut destroyed = false;
            app.poll_events(None, |event| {
                if let PollEvent::Main(main) = event {
                    destroyed |= self.on_main_event(&main);
                }
            });
            self.read_input();
            if destroyed {
                self.app_thread
                    .send_and_wait(self.view, ActivityEvent::Destroyed);
                return;
            }
        }
    }

    /// Hands on what the system told the activity; answers whether it was destroyed.
    fn on_main_event(&self, event: &MainEvent<'_>) -> bool {
        let handed = match event {
            MainEvent::InitWindow { .. } => ActivityEvent::WindowArrived,
            MainEvent::TerminateWindow { .. } => {
                self.app_thread
                    .send_and_wait(self.view, ActivityEvent::WindowGone);
                return false;
            }
            MainEvent::WindowResized { .. }
            | MainEvent::ContentRectChanged { .. }
            | MainEvent::InsetsChanged { .. } => ActivityEvent::GeometryChanged,
            MainEvent::ConfigChanged { .. } => ActivityEvent::ConfigChanged,
            MainEvent::RedrawNeeded { .. } => ActivityEvent::RedrawNeeded,
            MainEvent::GainedFocus => ActivityEvent::FocusChanged(true),
            MainEvent::LostFocus => ActivityEvent::FocusChanged(false),
            MainEvent::Destroy => return true,
            _ => return false,
        };
        self.app_thread.send(self.view, handed);
        false
    }

    /// The input the activity received since the last pass.
    fn read_input(&mut self) {
        let app = self.app.clone();
        let Ok(mut events) = app.input_events_iter() else {
            return;
        };
        loop {
            let read = events.next(|event| {
                self.on_input(event);
                InputStatus::Handled
            });
            if !read {
                break;
            }
        }
    }

    fn on_input(&mut self, event: &InputEvent<'_>) {
        match event {
            InputEvent::MotionEvent(motion) => self.on_motion(motion),
            InputEvent::KeyEvent(key) => self.on_key(key),
            // The rest of the keyboard follows.
            _ => {}
        }
    }

    fn on_motion(&mut self, motion: &MotionEvent<'_>) {
        let packet = self.pointers.apply(motion, self.view);
        if !packet.data.is_empty() {
            self.app_thread
                .send(self.view, ActivityEvent::Pointer(packet));
        }
    }

    /// The back button and the back gesture both arrive as this key, read on release, as Android's
    /// own back is: a press that is held and then cancelled must not go back.
    fn on_key(&self, key: &KeyEvent<'_>) {
        if key.key_code() == Keycode::Back && key.action() == KeyAction::Up {
            self.app_thread.send(self.view, ActivityEvent::Back);
        }
        // The rest of the keyboard follows.
    }
}
