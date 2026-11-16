//! AutoSuggestBox: suggestions the page filters from a list as the user types, chosen with the arrow keys or a click and submitted with Enter or the query button, and suggestions that are records shown through a template.

use crate::{column, example, label, section};
use std::fmt;
use waft_foundation::{App, Handle, Ref};
use waft_widgets::*;
use waft_winui::*;

/// Builds the feature page.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section(
        "AutoSuggestBox",
        resources,
        AutoSuggestBoxDemo.into_widget(),
    )
}

const COLORS: [&str; 16] = [
    "Amber", "Aqua", "Azure", "Beige", "Black", "Blue", "Bronze", "Brown", "Coral", "Crimson",
    "Cyan", "Gold", "Green", "Indigo", "Ivory", "Navy",
];

/// A city, shown by its template with its country under its name.
#[derive(Clone, Debug)]
struct City {
    name: &'static str,
    country: &'static str,
}

impl fmt::Display for City {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}", self.name, self.country)
    }
}

const CITIES: [City; 8] = [
    City {
        name: "Lisbon",
        country: "Portugal",
    },
    City {
        name: "Lima",
        country: "Peru",
    },
    City {
        name: "Lagos",
        country: "Nigeria",
    },
    City {
        name: "London",
        country: "United Kingdom",
    },
    City {
        name: "Los Angeles",
        country: "United States",
    },
    City {
        name: "Lyon",
        country: "France",
    },
    City {
        name: "Madrid",
        country: "Spain",
    },
    City {
        name: "Montreal",
        country: "Canada",
    },
];

/// The words of `all` that start with what the user typed, ignoring case.
fn matching<T: Clone>(all: &[T], typed: &str, name: impl Fn(&T) -> &str) -> Vec<T> {
    let typed = typed.to_lowercase();
    if typed.is_empty() {
        return Vec::new();
    }
    all.iter()
        .filter(|item| name(item).to_lowercase().starts_with(&typed))
        .cloned()
        .collect()
}

#[derive(Debug)]
struct AutoSuggestBoxDemo;

struct AutoSuggestBoxDemoState {
    state: StateData<AutoSuggestBoxDemo>,
    color_text: Option<Ref<TextEditingController>>,
    city_text: Option<Ref<TextEditingController>>,
    colors: Vec<&'static str>,
    cities: Vec<City>,
    color_status: String,
    city_status: String,
}

impl StatefulWidget for AutoSuggestBoxDemo {
    type State = AutoSuggestBoxDemoState;

    fn create_state(&self) -> Self::State {
        AutoSuggestBoxDemoState {
            state: StateData::new(),
            color_text: None,
            city_text: None,
            colors: Vec::new(),
            cities: Vec::new(),
            color_status: "Type a color.".into(),
            city_status: "Type a city.".into(),
        }
    }
}

impl AutoSuggestBoxDemoState {
    fn colors_box(self: Handle<Self>, app: &App) -> WidgetRef {
        let state = app.get(self);
        AutoSuggestBox::new(state.color_text.as_ref().unwrap().erase())
            .items_source(state.colors.clone())
            .header(Text::new("Color"))
            .placeholder_text("Search colors")
            .query_icon(FontIcon::symbol(FluentSymbol::Search).font_size(12.0))
            .text_changed(move |app, args| {
                if args.reason() == AutoSuggestionBoxTextChangeReason::UserInput {
                    let typed = self.typed(app, |s| &s.color_text);
                    self.set_state(app, |s| s.colors = matching(&COLORS, &typed, |c| c))
                }
            })
            .suggestion_chosen(move |app, args| {
                let chosen = format!("Chosen: {}", args.selected_item());
                self.set_state(app, |s| s.color_status = chosen);
            })
            .query_submitted(move |app, args| {
                let status = match args.chosen_suggestion() {
                    Some(color) => format!("Submitted the suggestion {color}"),
                    None => format!("Submitted the text \u{201c}{}\u{201d}", args.query_text()),
                };
                self.set_state(app, |s| s.color_status = status);
            })
            .into_widget()
    }

    fn cities_box(self: Handle<Self>, app: &App) -> WidgetRef {
        let state = app.get(self);
        AutoSuggestBox::new(state.city_text.as_ref().unwrap().erase())
            .items_source(state.cities.clone())
            .text_member_path(|city: &City| city.name.to_string())
            .item_template(|_, _, city| {
                column(
                    vec![
                        Text::new(city.name).into_widget(),
                        Builder::new({
                            let country = city.country;
                            move |app, context| {
                                let r = ThemeResources::of(app, context);
                                Text::new(country)
                                    .style(
                                        TextBlockStyle::Caption
                                            .text_style(&r, r.text_fill_color_secondary),
                                    )
                                    .into_widget()
                            }
                        })
                        .into_widget(),
                    ],
                    0.0,
                )
            })
            .placeholder_text("Search cities")
            .description(Text::new(
                "Suggestions are records; the box shows a city's name when it is chosen.",
            ))
            .max_suggestion_list_height(200.0)
            .text_changed(move |app, args| {
                if args.reason() == AutoSuggestionBoxTextChangeReason::UserInput {
                    let typed = self.typed(app, |s| &s.city_text);
                    self.set_state(app, |s| s.cities = matching(&CITIES, &typed, |c| c.name))
                }
            })
            .query_submitted(move |app, args| {
                let status = match args.chosen_suggestion() {
                    Some(city) => format!("Going to {} in {}", city.name, city.country),
                    None => format!("No city chosen for \u{201c}{}\u{201d}", args.query_text()),
                };
                self.set_state(app, |s| s.city_status = status);
            })
            .into_widget()
    }

    fn typed(
        self: Handle<Self>,
        app: &App,
        controller: impl Fn(&Self) -> &Option<Ref<TextEditingController>>,
    ) -> String {
        let controller: TextEditingControllerRef =
            controller(app.get(self)).as_ref().unwrap().erase();
        controller.text_value(app).to_owned()
    }
}

impl State for AutoSuggestBoxDemoState {
    type Widget = AutoSuggestBoxDemo;
    waft_widgets::state_accessors!();

    fn init_state(self: Handle<Self>, app: &mut App) {
        let color_text = TextEditingController::new(app);
        let city_text = TextEditingController::new(app);
        let state = app.get_mut(self);
        state.color_text = Some(color_text);
        state.city_text = Some(city_text);
    }

    fn dispose(self: Handle<Self>, app: &mut App) {
        let state = app.get_mut(self);
        let controllers = [state.color_text.take(), state.city_text.take()];
        for controller in controllers.into_iter().flatten() {
            controller.dispose(app);
        }
    }

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let resources = ThemeResources::of(app, context);
        let state = app.get(self);
        let (color_status, city_status) = (state.color_status.clone(), state.city_status.clone());
        let status = |text: String| {
            label(
                text,
                TextBlockStyle::Body,
                resources.text_fill_color_secondary,
            )
        };
        column(
            vec![
                example(
                    "Suggestions as you type",
                    "The page lists the colors that start with the typed text when it hears TextChanged. Up and Down move through the suggestions and show the one moved to, Escape puts back what was typed, and Enter, a click or the query button submits.",
                    &resources,
                    column(
                        vec![
                            SizedBox::new()
                                .width(320.0)
                                .child(self.colors_box(app))
                                .into_widget(),
                            status(color_status),
                        ],
                        12.0,
                    ),
                ),
                example(
                    "Suggestions with a template",
                    "Each suggestion is a city record, drawn with its country under its name, with the list at most 200 pixels tall.",
                    &resources,
                    column(
                        vec![
                            SizedBox::new()
                                .width(320.0)
                                .child(self.cities_box(app))
                                .into_widget(),
                            status(city_status),
                        ],
                        12.0,
                    ),
                ),
            ],
            24.0,
        )
    }
}
