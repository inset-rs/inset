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
    DataTables,
    Search,
    Carousels,
}

impl Entry {
    /// Index order — the order the tiles appear in.
    pub const ALL: [Entry; 8] = [
        Entry::Buttons,
        Entry::AppBars,
        Entry::Lists,
        Entry::Drawers,
        Entry::Messages,
        Entry::DataTables,
        Entry::Search,
        Entry::Carousels,
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
            Entry::DataTables => "Data Tables",
            Entry::Search => "Search",
            Entry::Carousels => "Carousels",
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
            Entry::DataTables => Icons::table_chart(),
            Entry::Search => Icons::search(),
            Entry::Carousels => Icons::view_carousel(),
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
            Entry::DataTables => pages::data_tables::body(app, context),
            Entry::Search => pages::search::body(app, context),
            Entry::Carousels => pages::carousels::body(app, context),
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
