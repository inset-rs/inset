//! The gallery's index: one entry per family of widgets, and where each one's page is built.

use waft_foundation::App;
use waft_material::Icons;
use waft_widgets::{BuildContext, IconData, WidgetRef};

use crate::pages;

/// One tile of the index.
///
/// Adding a demo means adding a variant, an arm in each table below, and a page module — no
/// registration, no dynamic lookup, and the compiler names every place that is still missing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Entry {
    Buttons,
    AppBars,
    Lists,
    Drawers,
    Messages,
    SelectionControls,
    Sliders,
    Progress,
    TextFields,
    Chips,
    Segments,
    Cards,
    Tabs,
    Dialogs,
    Menus,
    Pickers,
    Autocomplete,
    ReorderableListView,
    Adaptive,
    DataTables,
    Search,
    Carousels,
    GridTiles,
    Steppers,
    Refresh,
    ButtonBars,
    MaterialButton,
    ExpansionPanels,
    AccountHeader,
}

impl Entry {
    /// Index order — the order the tiles appear in.
    pub const ALL: [Entry; 29] = [
        Entry::Buttons,
        Entry::AppBars,
        Entry::Lists,
        Entry::Drawers,
        Entry::Messages,
        Entry::SelectionControls,
        Entry::Sliders,
        Entry::Progress,
        Entry::TextFields,
        Entry::Chips,
        Entry::Segments,
        Entry::Cards,
        Entry::Tabs,
        Entry::Dialogs,
        Entry::Menus,
        Entry::Pickers,
        Entry::Autocomplete,
        Entry::ReorderableListView,
        Entry::Adaptive,
        Entry::DataTables,
        Entry::Search,
        Entry::Carousels,
        Entry::GridTiles,
        Entry::Steppers,
        Entry::Refresh,
        Entry::ButtonBars,
        Entry::MaterialButton,
        Entry::ExpansionPanels,
        Entry::AccountHeader,
    ];

    /// The entry named by its variant, case-insensitively: how a launcher picks a page.
    pub fn from_name(name: &str) -> Option<Entry> {
        let name = name.to_ascii_lowercase();
        Entry::ALL
            .into_iter()
            .find(|entry| format!("{entry:?}").to_ascii_lowercase() == name)
    }

    /// The tile's title, and the title of the page's app bar.
    pub fn title(self) -> &'static str {
        match self {
            Entry::Buttons => "Buttons",
            Entry::AppBars => "App Bars",
            Entry::Lists => "Lists",
            Entry::Drawers => "Drawers",
            Entry::Messages => "Snack Bars & Sheets",
            Entry::SelectionControls => "Selection Controls",
            Entry::Sliders => "Sliders",
            Entry::Progress => "Progress Indicators",
            Entry::TextFields => "Text Fields",
            Entry::Chips => "Chips",
            Entry::Segments => "Segmented & Toggle Buttons",
            Entry::Cards => "Cards, Badges & Avatars",
            Entry::Tabs => "Tabs",
            Entry::Dialogs => "Dialogs",
            Entry::Menus => "Menus",
            Entry::Pickers => "Date & Time Pickers",
            Entry::Autocomplete => "Autocomplete",
            Entry::ReorderableListView => "Reorderable Lists",
            Entry::Adaptive => "Adaptive Layout",
            Entry::DataTables => "Data Tables",
            Entry::Search => "Search",
            Entry::Carousels => "Carousels",
            Entry::GridTiles => "Grid Tiles",
            Entry::Steppers => "Steppers",
            Entry::Refresh => "Refresh",
            Entry::ButtonBars => "Button Bars",
            Entry::MaterialButton => "Material Button",
            Entry::ExpansionPanels => "Expansion Panels",
            Entry::AccountHeader => "Account Header",
        }
    }

    /// The glyph leading the tile.
    pub fn icon(self) -> IconData {
        match self {
            Entry::Buttons => Icons::smart_button(),
            Entry::AppBars => Icons::web_asset(),
            Entry::Lists => Icons::view_list(),
            Entry::Drawers => Icons::menu_open(),
            Entry::Messages => Icons::announcement(),
            Entry::SelectionControls => Icons::check_box(),
            Entry::Sliders => Icons::tune(),
            Entry::Progress => Icons::autorenew(),
            Entry::TextFields => Icons::text_fields(),
            Entry::Chips => Icons::label(),
            Entry::Segments => Icons::view_week(),
            Entry::Cards => Icons::style(),
            Entry::Tabs => Icons::tab(),
            Entry::Dialogs => Icons::chat_bubble(),
            Entry::Menus => Icons::more_vert(),
            Entry::Pickers => Icons::event(),
            Entry::Autocomplete => Icons::manage_search(),
            Entry::ReorderableListView => Icons::reorder(),
            Entry::Adaptive => Icons::devices(),
            Entry::DataTables => Icons::table_chart(),
            Entry::Search => Icons::search(),
            Entry::Carousels => Icons::view_carousel(),
            Entry::GridTiles => Icons::grid_view(),
            Entry::Steppers => Icons::linear_scale(),
            Entry::Refresh => Icons::refresh(),
            Entry::ButtonBars => Icons::view_stream(),
            Entry::MaterialButton => Icons::touch_app(),
            Entry::ExpansionPanels => Icons::expand_more(),
            Entry::AccountHeader => Icons::account_circle(),
        }
    }

    /// How many pages this entry can push beyond its own.
    ///
    /// Only the tests read this; it is here so a new sub-page cannot be added without the
    /// tests reaching it.
    pub fn sub_page_count(self) -> usize {
        match self {
            Entry::AppBars => 1,
            _ => 0,
        }
    }

    /// The entry's own page.
    pub fn body(self, app: &mut App, context: BuildContext) -> WidgetRef {
        match self {
            Entry::Buttons => pages::buttons::body(app, context),
            Entry::AppBars => pages::app_bars::body(app, context),
            Entry::Lists => pages::lists::body(app, context),
            Entry::Drawers => pages::drawers::body(app, context),
            Entry::Messages => pages::messages::body(app, context),
            Entry::SelectionControls => pages::selection_controls::body(app, context),
            Entry::Sliders => pages::sliders::body(app, context),
            Entry::Progress => pages::progress::body(app, context),
            Entry::TextFields => pages::text_fields::body(app, context),
            Entry::Chips => pages::chips::body(app, context),
            Entry::Segments => pages::segments::body(app, context),
            Entry::Cards => pages::cards::body(app, context),
            Entry::Tabs => pages::tabs::body(app, context),
            Entry::Dialogs => pages::dialogs::body(app, context),
            Entry::Menus => pages::menus::body(app, context),
            Entry::Pickers => pages::pickers::body(app, context),
            Entry::Autocomplete => pages::autocomplete::body(app, context),
            Entry::ReorderableListView => pages::reorderable_list_view::body(app, context),
            Entry::Adaptive => pages::adaptive::body(app, context),
            Entry::DataTables => pages::data_tables::body(app, context),
            Entry::Search => pages::search::body(app, context),
            Entry::Carousels => pages::carousels::body(app, context),
            Entry::GridTiles => pages::grid_tiles::body(app, context),
            Entry::Steppers => pages::steppers::body(app, context),
            Entry::Refresh => pages::refresh::body(app, context),
            Entry::ButtonBars => pages::button_bars::body(app, context),
            Entry::MaterialButton => pages::material_button::body(app, context),
            Entry::ExpansionPanels => pages::expansion_panels::body(app, context),
            Entry::AccountHeader => pages::account_header::body(app, context),
        }
    }

    /// The title of one of this entry's sub-pages.
    pub fn sub_title(self, index: usize) -> &'static str {
        match self {
            Entry::AppBars => pages::app_bars::SUB_TITLE,
            _ => panic!("{self:?} has no sub-page {index}"),
        }
    }

    /// One of this entry's sub-pages.
    pub fn sub_body(self, index: usize, app: &mut App, context: BuildContext) -> WidgetRef {
        match self {
            Entry::AppBars => pages::app_bars::sub_body(app, context),
            _ => panic!("{self:?} has no sub-page {index}"),
        }
    }
}
