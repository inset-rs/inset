//! The Page example: the gallery itself is a `Page`, and this destination fills the area the page leaves free, its text box on the bottom edge, so the system's bars and the keyboard can be seen to stay clear of it.

use crate::{example, label};
use waft_foundation::{App, Handle};
use waft_painting::EdgeInsetsGeometry;
use waft_rendering::{CrossAxisAlignment, MainAxisSize};
use waft_widgets::*;
use waft_winui::*;

/// Builds the Page destination, which fills the content area instead of scrolling.
pub fn build(resources: &ThemeResources, horizontal_inset: f64) -> WidgetRef {
    let explanation = example(
        "The gallery is a Page",
        "Its background reaches under the system's bars; its content stays inside them, and shrinks above the keyboard. Focus the box below on a phone.",
        resources,
        label(
            "Page::new().background(..).content(..)",
            TextBlockStyle::Body,
            resources.text_fill_color_secondary,
        ),
    );
    Padding::new(EdgeInsetsGeometry::from_ltrb(
        horizontal_inset,
        32.0,
        horizontal_inset,
        16.0,
    ))
    .child(
        Column::new()
            .main_axis_size(MainAxisSize::Max)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .spacing(20.0)
            .children(vec![
                label(
                    crate::Feature::Page.description(),
                    TextBlockStyle::Body,
                    resources.text_fill_color_secondary,
                ),
                explanation,
                Spacer::new().into_widget(),
                BottomMessage.into_widget(),
            ]),
    )
    .into_widget()
}

/// A text box on the page's bottom edge, with its controller kept across navigation.
#[derive(Debug)]
struct BottomMessage;

/// Owns the message box's controller.
struct BottomMessageState {
    /// Native widget state identity.
    state: StateData<BottomMessage>,
    /// The message being written.
    controller: Option<Handle<TextEditingController>>,
}

impl StatefulWidget for BottomMessage {
    type State = BottomMessageState;

    fn create_state(&self) -> BottomMessageState {
        BottomMessageState {
            state: StateData::new(),
            controller: None,
        }
    }
}

impl State for BottomMessageState {
    type Widget = BottomMessage;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let controller = TextEditingController::new(app);
        app.get_mut(self).controller = Some(controller);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        if let Some(controller) = app.get_mut(self).controller.take() {
            app.get_mut(controller).dispose();
            app.destroy(controller);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let controller = app.get(self).controller.expect("made in init_state");
        TextBox::new(controller.as_controller())
            .header(Text::new("Message"))
            .placeholder_text("Type a message")
            .into_widget()
    }
}
