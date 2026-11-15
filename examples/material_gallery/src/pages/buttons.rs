//! Material 3's buttons: the five common buttons, each plain, with an icon and disabled; the
//! four icon-button variants as toggles; and the four floating action button sizes.
//!
//! A button with no `on_pressed` is disabled. The tooltips are on the icon buttons and the
//! floating action buttons, which have no label of their own; a long press shows one.

use waft_foundation::{App, Handle, Listener};
use waft_material::{
    ElevatedButton, FilledButton, FloatingActionButton, IconButton, Icons, OutlinedButton,
    TextButton,
};
use waft_painting::EdgeInsetsGeometry;
use waft_widgets::{
    BuildContext, Icon, IconData, IntoWidget, ListView, State, StateData, StatefulWidget, Text,
    WidgetRef,
};

use crate::catalog::Entry;
use crate::support::{caption, page, section_header, showcase};

/// The icon-button variants, in the order the toggles show them.
const ICON_BUTTON_VARIANTS: [&str; 4] = ["Standard", "Filled", "Tonal", "Outlined"];

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    ButtonsDemo.into_widget()
}

/// The last press and the toggles live here, in the smallest widget that renders them.
#[derive(Debug)]
pub struct ButtonsDemo;

/// Dart would spell this `_ButtonsDemoState`.
pub struct ButtonsDemoState {
    state: StateData<ButtonsDemo>,
    last_pressed: Option<&'static str>,
    selected: [bool; ICON_BUTTON_VARIANTS.len()],
}

impl StatefulWidget for ButtonsDemo {
    type State = ButtonsDemoState;

    fn create_state(&self) -> ButtonsDemoState {
        ButtonsDemoState {
            state: StateData::new(),
            last_pressed: None,
            selected: [false; ICON_BUTTON_VARIANTS.len()],
        }
    }
}

impl State for ButtonsDemoState {
    type Widget = ButtonsDemo;
    waft_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let (last_pressed, selected) = {
            let state = app.get(self);
            (state.last_pressed, state.selected)
        };
        let press = |name: &'static str| {
            Some(Listener::new(move |app: &mut App| {
                self.set_state(app, |state| state.last_pressed = Some(name));
            }))
        };
        let toggle = |index: usize| {
            Some(Listener::new(move |app: &mut App| {
                self.set_state(app, |state| state.selected[index] = !state.selected[index]);
            }))
        };

        let children = vec![
            caption(
                app,
                context,
                format!("Last pressed: {}", last_pressed.unwrap_or("\u{2014}")),
            ),
            section_header(app, context, "Common buttons"),
            showcase(vec![
                ElevatedButton::new(press("Elevated"), Some(Text::new("Elevated"))).into_widget(),
                ElevatedButton::icon(press("Elevated icon"), Text::new("Icon"))
                    .icon(Icon::new(Some(Icons::add())))
                    .into_widget(),
                ElevatedButton::new(None, Some(Text::new("Disabled"))).into_widget(),
            ]),
            showcase(vec![
                FilledButton::new(press("Filled"), Some(Text::new("Filled"))).into_widget(),
                FilledButton::icon(press("Filled icon"), Text::new("Icon"))
                    .icon(Icon::new(Some(Icons::send())))
                    .into_widget(),
                FilledButton::new(None, Some(Text::new("Disabled"))).into_widget(),
            ]),
            showcase(vec![
                FilledButton::tonal(press("Tonal"), Some(Text::new("Tonal"))).into_widget(),
                FilledButton::tonal_icon(press("Tonal icon"), Text::new("Icon"))
                    .icon(Icon::new(Some(Icons::edit())))
                    .into_widget(),
                FilledButton::tonal(None, Some(Text::new("Disabled"))).into_widget(),
            ]),
            showcase(vec![
                OutlinedButton::new(press("Outlined"), Some(Text::new("Outlined"))).into_widget(),
                OutlinedButton::icon(press("Outlined icon"), Text::new("Icon"))
                    .icon(Icon::new(Some(Icons::share())))
                    .into_widget(),
                OutlinedButton::new(None, Some(Text::new("Disabled"))).into_widget(),
            ]),
            showcase(vec![
                TextButton::new(press("Text"), Some(Text::new("Text"))).into_widget(),
                TextButton::icon(press("Text icon"), Text::new("Icon"))
                    .icon(Icon::new(Some(Icons::info())))
                    .into_widget(),
                TextButton::new(None, Some(Text::new("Disabled"))).into_widget(),
            ]),
            section_header(app, context, "Icon buttons"),
            caption(app, context, selected_summary(selected)),
            showcase(
                ICON_BUTTON_VARIANTS
                    .iter()
                    .enumerate()
                    .map(|(index, name)| icon_button(index, toggle(index), selected[index], name))
                    .collect(),
            ),
            showcase(
                (0..ICON_BUTTON_VARIANTS.len())
                    .map(|index| icon_button(index, None, false, "Disabled"))
                    .collect(),
            ),
            section_header(app, context, "Floating action buttons"),
            showcase(vec![
                inline_fab(FloatingActionButton::small(press("Small FAB")))
                    .tooltip("Small")
                    .child(Icon::new(Some(Icons::add())))
                    .into_widget(),
                inline_fab(FloatingActionButton::new(press("FAB")))
                    .tooltip("Regular")
                    .child(Icon::new(Some(Icons::add())))
                    .into_widget(),
                inline_fab(FloatingActionButton::large(press("Large FAB")))
                    .tooltip("Large")
                    .child(Icon::new(Some(Icons::add())))
                    .into_widget(),
                inline_fab(FloatingActionButton::extended(
                    press("Extended FAB"),
                    Text::new("Extended"),
                ))
                .icon(Icon::new(Some(Icons::edit())))
                .into_widget(),
            ]),
        ];

        page(
            Entry::Buttons.title(),
            ListView::new()
                .padding(EdgeInsetsGeometry::only(0.0, 0.0, 0.0, 96.0))
                .children(children),
        )
        .floating_action_button(
            FloatingActionButton::extended(press("Scaffold FAB"), Text::new("Compose"))
                .icon(Icon::new(Some(Icons::edit())))
                .tooltip("The scaffold's own floating action button"),
        )
        .into_widget()
    }
}

/// A floating action button shown in the page's body rather than the scaffold's slot.
///
/// Every floating action button flies as a hero under the same default tag, and a route may
/// hold only one hero per tag. The scaffold's own button keeps the tag; these give it up.
fn inline_fab(button: FloatingActionButton) -> FloatingActionButton {
    button.hero_tag(None)
}

/// One icon button of variant `index`: a toggle whose outlined glyph fills in when selected.
fn icon_button(
    index: usize,
    on_pressed: Option<Listener>,
    is_selected: bool,
    tooltip: &str,
) -> WidgetRef {
    let (icon, selected_icon) = (Icons::settings_outlined(), Icons::settings());
    let glyph = |icon: IconData| Icon::new(Some(icon));
    let button = match index {
        0 => IconButton::new(on_pressed, glyph(icon)),
        1 => IconButton::filled(on_pressed, glyph(icon)),
        2 => IconButton::filled_tonal(on_pressed, glyph(icon)),
        _ => IconButton::outlined(on_pressed, glyph(icon)),
    };
    button
        .is_selected(is_selected)
        .selected_icon(glyph(selected_icon))
        .tooltip(tooltip)
        .into_widget()
}

/// Which icon-button toggles are on, as a line of text.
fn selected_summary(selected: [bool; ICON_BUTTON_VARIANTS.len()]) -> String {
    let on: Vec<&str> = ICON_BUTTON_VARIANTS
        .iter()
        .zip(selected)
        .filter(|(_, selected)| *selected)
        .map(|(name, _)| *name)
        .collect();
    if on.is_empty() {
        "Selected: none".to_owned()
    } else {
        format!("Selected: {}", on.join(", "))
    }
}
