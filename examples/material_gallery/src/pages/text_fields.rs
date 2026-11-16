//! Material 3's text fields: filled and outlined, with a label, hint and helper text, prefix
//! and suffix, an error, disabled, a character counter, obscured text and several lines.
//!
//! The fields keep their own text; nothing here observes it except the error demo, which
//! shows the error for as long as the field is empty.

use waft_foundation::{App, Handle};
use waft_material::{Icons, InputDecoration, OutlineInputBorder, TextField, UnderlineInputBorder};
use waft_painting::EdgeInsetsGeometry;
use waft_widgets::{
    BuildContext, Icon, IntoWidget, ListView, Padding, State, StateData, StatefulWidget, WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header};

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    let content = waft_widgets::Builder::new(|app: &mut App, context: BuildContext| {
        ListView::new()
            .children(vec![
                section_header(app, context, "Filled"),
                filled_fields(),
                section_header(app, context, "Outlined"),
                outlined_fields(),
                section_header(app, context, "Prefix and suffix"),
                affix_fields(),
                section_header(app, context, "Helper and error text"),
                helper_fields(),
                section_header(app, context, "Counter"),
                counter_fields(app, context),
                section_header(app, context, "Disabled"),
                disabled_fields(),
                section_header(app, context, "Other inputs"),
                other_fields(),
            ])
            .into_widget()
    });
    page(Entry::TextFields.title(), content).into_widget()
}

/// A field in the page's column: a gutter around it.
fn field(text_field: TextField) -> WidgetRef {
    Padding::new(EdgeInsetsGeometry::symmetric(8.0, 16.0))
        .child(text_field)
        .into_widget()
}

fn filled_decoration(label: &str) -> InputDecoration {
    InputDecoration::new()
        .filled(true)
        .border(UnderlineInputBorder::new())
        .label_text(label)
}

fn outlined_decoration(label: &str) -> InputDecoration {
    InputDecoration::new()
        .border(OutlineInputBorder::new())
        .label_text(label)
}

fn filled_fields() -> WidgetRef {
    waft_widgets::Column::new()
        .children(vec![
            field(TextField::new().decoration(Some(filled_decoration("Label")))),
            field(TextField::new().decoration(Some(
                filled_decoration("With a hint").hint_text("Type here"),
            ))),
        ])
        .into_widget()
}

fn outlined_fields() -> WidgetRef {
    waft_widgets::Column::new()
        .children(vec![
            field(TextField::new().decoration(Some(outlined_decoration("Label")))),
            field(TextField::new().decoration(Some(
                outlined_decoration("With a hint").hint_text("Type here"),
            ))),
        ])
        .into_widget()
}

fn affix_fields() -> WidgetRef {
    waft_widgets::Column::new()
        .children(vec![
            field(
                TextField::new().decoration(Some(
                    filled_decoration("Search")
                        .prefix_icon(Icon::new(Some(Icons::search())))
                        .suffix_icon(Icon::new(Some(Icons::clear()))),
                )),
            ),
            field(
                TextField::new().decoration(Some(
                    outlined_decoration("Amount")
                        .prefix_text("$")
                        .suffix_text("USD"),
                )),
            ),
        ])
        .into_widget()
}

fn helper_fields() -> WidgetRef {
    waft_widgets::Column::new()
        .children(vec![
            field(TextField::new().decoration(Some(
                outlined_decoration("With helper text").helper_text("Supporting text"),
            ))),
            field(TextField::new().decoration(Some(
                outlined_decoration("With an error").error_text("This value is not valid"),
            ))),
            RequiredFieldDemo.into_widget(),
        ])
        .into_widget()
}

fn counter_fields(app: &mut App, context: BuildContext) -> WidgetRef {
    waft_widgets::Column::new()
        .children(vec![
            field(
                TextField::new()
                    .max_length(20)
                    .decoration(Some(outlined_decoration("At most 20 characters"))),
            ),
            caption(
                app,
                context,
                "The counter shows how much of the limit is used.",
            ),
        ])
        .into_widget()
}

fn disabled_fields() -> WidgetRef {
    waft_widgets::Column::new()
        .children(vec![
            field(
                TextField::new()
                    .enabled(false)
                    .decoration(Some(filled_decoration("Disabled filled"))),
            ),
            field(
                TextField::new()
                    .enabled(false)
                    .decoration(Some(outlined_decoration("Disabled outlined"))),
            ),
        ])
        .into_widget()
}

fn other_fields() -> WidgetRef {
    waft_widgets::Column::new()
        .children(vec![
            field(
                TextField::new()
                    .obscure_text(true)
                    .decoration(Some(outlined_decoration("Password"))),
            ),
            field(
                TextField::new()
                    .min_lines(3)
                    .max_lines(Some(5))
                    .decoration(Some(outlined_decoration("Several lines"))),
            ),
        ])
        .into_widget()
}

/// A field that shows an error while it is empty, so the error appears and goes as the user
/// types.
#[derive(Debug)]
pub struct RequiredFieldDemo;

pub struct RequiredFieldDemoState {
    state: StateData<RequiredFieldDemo>,
    text: String,
}

impl StatefulWidget for RequiredFieldDemo {
    type State = RequiredFieldDemoState;

    fn create_state(&self) -> RequiredFieldDemoState {
        RequiredFieldDemoState {
            state: StateData::new(),
            text: String::new(),
        }
    }
}

impl State for RequiredFieldDemoState {
    type Widget = RequiredFieldDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _context: BuildContext) -> WidgetRef {
        let mut decoration = outlined_decoration("Required");
        if app.get(self).text.is_empty() {
            decoration = decoration.error_text("Required");
        }
        field(TextField::new().decoration(Some(decoration)).on_changed(
            move |app: &mut App, text: String| {
                self.set_state(app, |state| state.text = text);
            },
        ))
    }
}
