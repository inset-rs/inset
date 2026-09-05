//! Flutter `widgets/title.dart`.

use reveal_embedder::{ApplicationSwitcherDescription, Color};
use reveal_foundation::{App, Handle};
use reveal_services::SystemChrome;

use crate::framework::{
    BuildContext, IntoWidget, KeyRef, State, StateData, StatefulWidget, WidgetRef,
};

/// A widget that describes this app in the operating system.
#[derive(Debug)]
pub struct Title {
    key: Option<KeyRef>,
    /// A one-line description of this app for use in the window manager.
    /// Must not be null.
    title: String,
    /// A color that the window manager should use to identify this app. Must be
    /// an opaque color (i.e. color.alpha must be 255 (0xFF)), and must not be
    /// null.
    color: Color,
    /// The widget below this widget in the tree.
    child: WidgetRef,
}

impl Title {
    /// Creates a widget that describes this app to the Android operating system.
    ///
    /// `color` must be an opaque color (i.e. color.alpha must be 255 (0xFF)).
    pub fn new<K>(color: Color, child: impl IntoWidget<K>) -> Title {
        debug_assert!(alpha_byte(color) == 0xFF);
        Title {
            key: None,
            title: String::new(),
            color,
            child: child.into_widget(),
        }
    }

    /// Dart `Title(key:)`.
    pub fn key(mut self, key: KeyRef) -> Title {
        self.key = Some(key);
        self
    }

    /// Dart `Title(title:)`.
    pub fn title(mut self, title: impl Into<String>) -> Title {
        self.title = title.into();
        self
    }
}

fn alpha_byte(color: Color) -> i32 {
    ((color.a * 255.0).round() as i32).clamp(0, 255)
}

impl StatefulWidget for Title {
    type State = TitleState;

    fn key(&self) -> Option<&KeyRef> {
        self.key.as_ref()
    }

    fn create_state(&self) -> TitleState {
        TitleState {
            state: StateData::new(),
        }
    }
}

/// Dart's `_TitleState`.
pub struct TitleState {
    state: StateData<Title>,
}

impl TitleState {
    fn update_chrome(self: Handle<Self>, app: &mut App) {
        let widget = self.widget(app);
        let description = ApplicationSwitcherDescription {
            label: Some(widget.title.clone()),
            primary_color: Some(widget.color.to_argb32()),
        };
        SystemChrome::set_application_switcher_description(app, &description);
    }
}

impl State for TitleState {
    type Widget = Title;
    crate::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        self.update_chrome(app);
    }

    fn did_update_widget(self: Handle<Self>, app: &mut App, old_widget: &Title) {
        let widget = self.widget(app);
        if old_widget.title != widget.title || old_widget.color != widget.color {
            self.update_chrome(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        self.widget(app).child.clone()
    }
}

#[cfg(test)]
mod tests {
    use reveal_foundation::AppCell;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::Instant;

    use reveal_embedder::{Platform, PlatformRef, TargetPlatform, ViewId, ViewRef};

    use super::*;
    use crate::test_harness::Harness;
    use crate::widgets::basic::SizedBox;

    #[derive(Default)]
    struct RecordingPlatform {
        descriptions: RefCell<Vec<ApplicationSwitcherDescription>>,
    }

    impl Platform for RecordingPlatform {
        fn target_platform(&self) -> TargetPlatform {
            TargetPlatform::Android
        }

        fn request_frame(&self) {}

        fn now(&self) -> Instant {
            Instant::now()
        }

        fn wake_at(&self, _deadline: Instant) {}

        fn views(&self) -> Vec<ViewRef> {
            Vec::new()
        }

        fn view(&self, _id: ViewId) -> Option<ViewRef> {
            None
        }

        fn implicit_view(&self) -> Option<ViewRef> {
            None
        }

        fn set_application_switcher_description(
            &self,
            description: &ApplicationSwitcherDescription,
        ) {
            self.descriptions.borrow_mut().push(description.clone());
        }
    }

    #[test]
    fn a_title_describes_the_app_to_the_platform_and_again_when_it_changes() {
        let platform = Rc::new(RecordingPlatform::default());
        let platform_ref: PlatformRef = Rc::clone(&platform) as PlatformRef;
        let cell = AppCell::with_platform(platform_ref);
        let mut app = cell.borrow_mut();
        let titled = |title: &str| {
            Title::new(Color::new(0xFF112233), SizedBox::shrink())
                .title(title)
                .into_widget()
        };
        let harness = Harness::mount(&mut app, titled("One"));
        harness.pump(&mut app);
        harness.set_child(&mut app, titled("One"));
        harness.pump(&mut app);
        harness.set_child(&mut app, titled("Two"));
        harness.pump(&mut app);
        let descriptions = platform.descriptions.borrow();
        assert_eq!(
            descriptions.as_slice(),
            [
                ApplicationSwitcherDescription {
                    label: Some(String::from("One")),
                    primary_color: Some(0xFF112233),
                },
                ApplicationSwitcherDescription {
                    label: Some(String::from("Two")),
                    primary_color: Some(0xFF112233),
                },
            ],
            "an unchanged title does not report again"
        );
    }
}
