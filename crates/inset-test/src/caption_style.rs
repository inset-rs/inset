//! The user's caption settings in memory, set by the test rather than the user: they answer the
//! style the test gives and tell the app each time the test changes it.

use std::cell::RefCell;
use std::rc::Rc;

use inset_embedder::{CaptionSettings, CaptionStyle, MediaChangeFuture};

use crate::media::Changes;

/// Caption settings that answer the style the test gives, telling the app each time the test
/// changes it. Every clone is the same settings.
#[derive(Clone)]
pub struct MockCaptionSettings(Rc<CaptionSettingsShared>);

struct CaptionSettingsShared {
    style: RefCell<CaptionStyle>,
    changes: Changes,
}

impl MockCaptionSettings {
    /// Settings answering `style`.
    pub fn new(style: CaptionStyle) -> MockCaptionSettings {
        MockCaptionSettings(Rc::new(CaptionSettingsShared {
            style: RefCell::new(style),
            changes: Changes::default(),
        }))
    }

    /// Answers `style` from now on, telling the app, as the system would once the user changed a
    /// setting.
    pub fn set_style(&self, style: CaptionStyle) {
        *self.0.style.borrow_mut() = style;
        self.0.changes.notify();
    }
}

impl CaptionSettings for MockCaptionSettings {
    fn style(&self) -> CaptionStyle {
        self.0.style.borrow().clone()
    }

    fn changed(&self) -> MediaChangeFuture {
        self.0.changes.next()
    }
}
