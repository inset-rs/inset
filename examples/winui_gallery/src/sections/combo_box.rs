//! ComboBox: a short list with a header and placeholder, a font list of a few hundred items, an
//! editable box of font sizes that takes sizes of its own, and disabled items and boxes, with the
//! selections they keep.

use crate::{column, example, label, section};
use inset_foundation::{App, Handle};
use inset_widgets::*;
use inset_winui::*;

/// Builds the ComboBox feature page.
pub fn build(resources: &ThemeResources) -> Vec<WidgetRef> {
    section("ComboBox", resources, ComboBoxDemo.into_widget())
}

/// The colors of WinUI Gallery's first example.
const COLORS: [&str; 4] = ["Blue", "Green", "Red", "Yellow"];

/// Font families, which with `WEIGHTS` make the long list.
const FAMILIES: [&str; 54] = [
    "Arial",
    "Bahnschrift",
    "Calibri",
    "Cambria",
    "Candara",
    "Cascadia Code",
    "Consolas",
    "Constantia",
    "Corbel",
    "Courier New",
    "Ebrima",
    "Franklin Gothic",
    "Gabriola",
    "Gadugi",
    "Georgia",
    "Impact",
    "Ink Free",
    "Javanese Text",
    "Leelawadee UI",
    "Lucida Console",
    "Lucida Sans Unicode",
    "Malgun Gothic",
    "Microsoft Himalaya",
    "Microsoft JhengHei",
    "Microsoft New Tai Lue",
    "Microsoft PhagsPa",
    "Microsoft Sans Serif",
    "Microsoft Tai Le",
    "Microsoft YaHei",
    "Microsoft Yi Baiti",
    "MingLiU-ExtB",
    "Mongolian Baiti",
    "MS Gothic",
    "MV Boli",
    "Myanmar Text",
    "Nirmala UI",
    "Palatino Linotype",
    "Segoe Print",
    "Segoe Script",
    "Segoe UI",
    "Segoe UI Emoji",
    "Segoe UI Historic",
    "Segoe UI Symbol",
    "Segoe UI Variable",
    "SimSun",
    "Sitka",
    "Sylfaen",
    "Symbol",
    "Tahoma",
    "Times New Roman",
    "Trebuchet MS",
    "Verdana",
    "Webdings",
    "Yu Gothic",
];

/// The weights of each family in the long list.
const WEIGHTS: [&str; 6] = ["", " Light", " Semilight", " Semibold", " Bold", " Black"];

/// The font sizes of the editable example.
const SIZES: [&str; 14] = [
    "8", "9", "10", "11", "12", "14", "16", "18", "20", "24", "28", "36", "48", "72",
];

/// The long list: every family in every weight, `324` items.
fn font_names() -> Vec<String> {
    FAMILIES
        .iter()
        .flat_map(|family| {
            WEIGHTS
                .iter()
                .map(move |weight| format!("{family}{weight}"))
        })
        .collect()
}

/// Items with their text as content and id.
fn text_items(texts: impl IntoIterator<Item = impl Into<String>>) -> Vec<ComboBoxItem> {
    texts
        .into_iter()
        .map(|text| {
            let text = text.into();
            ComboBoxItem::text(text.clone(), text)
        })
        .collect()
}

/// The ComboBox examples.
#[derive(Debug)]
struct ComboBoxDemo;

/// The selections the examples keep.
struct ComboBoxDemoState {
    /// Native widget state.
    state: StateData<ComboBoxDemo>,

    /// The color's selection.
    color: Option<usize>,

    /// The font's selection.
    font: Option<usize>,

    /// The font size's selection: an item, or a size typed in.
    size: Option<usize>,

    /// A font size typed in that matches no item.
    custom_size: Option<String>,

    /// Whether the editable box takes typed sizes for itself, handling `TextSubmitted`.
    handle_submitted: bool,

    /// The selection of the box with a disabled item.
    shape: Option<usize>,

    /// The last event.
    event: String,
}

impl StatefulWidget for ComboBoxDemo {
    type State = ComboBoxDemoState;

    fn create_state(&self) -> Self::State {
        ComboBoxDemoState {
            state: StateData::new(),
            color: None,
            font: Some(
                font_names()
                    .iter()
                    .position(|name| name == "Segoe UI")
                    .unwrap(),
            ),
            size: Some(4),
            custom_size: None,
            handle_submitted: false,
            shape: Some(0),
            event: "No selection changed yet".to_owned(),
        }
    }
}

/// How the labels name what a box selected.
fn described(item: &Option<ComboBoxSelectedItem>) -> String {
    match item {
        None => "nothing".to_owned(),
        Some(ComboBoxSelectedItem::Item { id, .. }) => id.clone(),
        Some(ComboBoxSelectedItem::CustomValue(text)) => format!("“{text}”"),
    }
}

impl ComboBoxDemoState {
    /// A `SelectionChanged` handler that stores the new index with `store` and records the event.
    fn changed(
        self: Handle<Self>,
        name: &'static str,
        store: fn(&mut ComboBoxDemoState, &ComboBoxSelectionChangedEventArgs),
    ) -> impl Fn(&mut App, ComboBoxSelectionChangedEventArgs) + 'static {
        move |app, args| {
            self.set_state(app, |state| {
                store(state, &args);
                state.event = format!(
                    "{name} SelectionChanged: {} → {}",
                    described(&args.removed_item),
                    described(&args.added_item)
                );
            })
        }
    }
}

impl State for ComboBoxDemoState {
    type Widget = ComboBoxDemo;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, context: BuildContext) -> WidgetRef {
        let r = ThemeResources::of(app, context);
        let secondary = r.text_fill_color_secondary;
        let s = app.get(self);
        let (color, font, size, custom_size) = (s.color, s.font, s.size, s.custom_size.clone());
        let (handle_submitted, shape, event) = (s.handle_submitted, s.shape, s.event.clone());
        let fonts = font_names();
        let width = |child: ComboBox| SizedBox::new().width(200.0).child(child).into_widget();
        let mut editable = ComboBox::new(
            text_items(SIZES),
            size,
            self.changed("Font size", |s, args| {
                s.size = args.selected_index();
                s.custom_size = args.custom_value().map(str::to_owned);
            }),
        )
        .header(Text::new("Font size"))
        .placeholder_text("Pick or type a size")
        .is_editable(true)
        .text_submitted(move |app, args| {
            let handled = app.get(self).handle_submitted;
            if handled {
                args.handled = true;
            }
            let text = args.text.clone();
            self.set_state(app, |state| {
                state.event = format!(
                    "Font size TextSubmitted: “{text}”{}",
                    if handled { ", handled" } else { "" }
                )
            });
        });
        if let Some(text) = custom_size.clone() {
            editable = editable.custom_value(text);
        }
        let size_label = match (size, &custom_size) {
            (Some(index), _) => SIZES[index].to_owned(),
            (None, Some(text)) => format!("{text} (typed)"),
            (None, None) => "none".to_owned(),
        };
        column(
            vec![
                example(
                    "A ComboBox with items",
                    "Click it or press F4, Alt+Down, Enter or Space to open it; the arrows move through the items and Enter selects. Type a letter to select by it.",
                    &r,
                    column(
                        vec![
                            width(
                                ComboBox::new(
                                    text_items(COLORS),
                                    color,
                                    self.changed("Colors", |s, args| {
                                        s.color = args.selected_index()
                                    }),
                                )
                                .header(Text::new("Colors"))
                                .placeholder_text("Pick a color"),
                            ),
                            label(
                                format!("Color: {}", color.map_or("none", |index| COLORS[index])),
                                TextBlockStyle::Caption,
                                secondary,
                            ),
                        ],
                        12.0,
                    ),
                ),
                example(
                    "A long list",
                    "324 fonts in a lazy list; the drop-down opens with the selected font over the box.",
                    &r,
                    column(
                        vec![
                            width(
                                ComboBox::new(
                                    text_items(fonts.clone()),
                                    font,
                                    self.changed("Font", |s, args| s.font = args.selected_index()),
                                )
                                .header(Text::new("Font"))
                                .placeholder_text("Pick a font")
                                .description(Text::new("Every family in six weights")),
                            ),
                            label(
                                format!(
                                    "Font: {}",
                                    font.map_or("none", |index| fonts[index].as_str())
                                ),
                                TextBlockStyle::Caption,
                                secondary,
                            ),
                        ],
                        12.0,
                    ),
                ),
                example(
                    "An editable ComboBox",
                    "Type a size: it completes to the first that starts with it. Enter commits; a size that is none of the items is submitted and, unless handled, kept.",
                    &r,
                    column(
                        vec![
                            CheckBox::new(Some(handle_submitted), move |app, value| {
                                self.set_state(app, |s| s.handle_submitted = value == Some(true))
                            })
                            .content(Text::new("Handle TextSubmitted"))
                            .into_widget(),
                            width(editable),
                            label(
                                format!("Font size: {size_label}"),
                                TextBlockStyle::Caption,
                                secondary,
                            ),
                        ],
                        12.0,
                    ),
                ),
                example(
                    "Disabled items and boxes",
                    "",
                    &r,
                    column(
                        vec![
                            width(
                                ComboBox::new(
                                    vec![
                                        ComboBoxItem::text("Circle", "Circle"),
                                        ComboBoxItem::text("Square", "Square").is_enabled(false),
                                        ComboBoxItem::text("Triangle", "Triangle"),
                                    ],
                                    shape,
                                    self.changed("Shape", |s, args| {
                                        s.shape = args.selected_index()
                                    }),
                                )
                                .header(Text::new("Shape")),
                            ),
                            width(
                                ComboBox::new(text_items(COLORS), Some(0), |_, _| {})
                                    .header(Text::new("Disabled"))
                                    .is_enabled(false),
                            ),
                        ],
                        12.0,
                    ),
                ),
                label(event, TextBlockStyle::Caption, secondary),
                // Keeps the page's last box clear of the window's bottom edge.
                SizedBox::new().height(240.0).into_widget(),
            ],
            20.0,
        )
    }
}
