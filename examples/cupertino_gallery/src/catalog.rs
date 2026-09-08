//! The gallery's index: one entry per feature, and where each one's screen is built.

use reveal_cupertino::{CupertinoColors, CupertinoIcons};
use reveal_foundation::App;
use reveal_painting::AnyColor;
use reveal_widgets::{BuildContext, IconData, WidgetRef};

use crate::pages;

/// One row of the index.
///
/// Adding a demo means adding a variant, an arm in each table below, and a page module — no
/// registration, no dynamic lookup, and the compiler names every place that is still missing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Entry {
    Navigation,
    PageScaffold,
    Buttons,
    Segments,
    Dialogs,
    Scrolling,
    Icons,
    Text,
    TextFields,
    Heroes,
    Indicators,
    Theming,
    Lists,
    ExpansionTiles,
}

impl Entry {
    /// Index order — the order the rows appear in.
    pub const ALL: [Entry; 14] = [
        Entry::Navigation,
        Entry::PageScaffold,
        Entry::Buttons,
        Entry::Segments,
        Entry::Dialogs,
        Entry::Scrolling,
        Entry::Icons,
        Entry::Text,
        Entry::TextFields,
        Entry::Heroes,
        Entry::Indicators,
        Entry::Theming,
        Entry::Lists,
        Entry::ExpansionTiles,
    ];

    /// The row's title, the screen's navigation-bar middle, and the label the next screen's
    /// back button reads.
    ///
    /// "Navigation" is ten characters on purpose: the back label keeps a previous title
    /// verbatim only up to twelve, and that entry is where the rule is demonstrated.
    /// The entry named by its variant, case-insensitively: how a launcher picks a screen.
    pub fn from_name(name: &str) -> Option<Entry> {
        let name = name.to_ascii_lowercase();
        Entry::ALL
            .into_iter()
            .find(|entry| format!("{entry:?}").to_ascii_lowercase() == name)
    }

    pub fn title(self) -> &'static str {
        match self {
            Entry::Navigation => "Navigation",
            Entry::PageScaffold => "Page Scaffold",
            Entry::Buttons => "Buttons",
            Entry::Segments => "Segmented Control",
            Entry::Dialogs => "Dialogs & Sheets",
            Entry::Scrolling => "Scrolling",
            Entry::Icons => "Icons",
            Entry::Text => "Text",
            Entry::TextFields => "Text Fields",
            Entry::Heroes => "Hero Flights",
            Entry::Indicators => "Activity",
            Entry::Theming => "Theming",
            Entry::Lists => "Lists & Forms",
            Entry::ExpansionTiles => "Expansion Tiles",
        }
    }

    /// The greyed text before the chevron, as in iOS Settings.
    pub fn summary(self) -> &'static str {
        match self {
            Entry::Navigation => "Bars, titles",
            Entry::PageScaffold => "Bar obstruction",
            Entry::Buttons => "Sizes, states",
            Entry::Segments => "One of a set",
            Entry::Dialogs => "Alert, sheet",
            Entry::Scrolling => "Scrolled under",
            Entry::Icons => "1,322",
            Entry::Text => "Overflow, scale",
            Entry::TextFields => "Type, placeholder",
            Entry::Heroes => "Shared tags",
            Entry::Indicators => "Spinner, bar",
            Entry::Theming => "Light & dark",
            Entry::Lists => "This screen",
            Entry::ExpansionTiles => "Fade, scroll",
        }
    }

    /// The glyph in the row's leading badge.
    pub fn icon(self) -> IconData {
        match self {
            Entry::Navigation => CupertinoIcons::square_stack(),
            Entry::PageScaffold => CupertinoIcons::rectangle_grid_1x2(),
            Entry::Buttons => CupertinoIcons::capsule(),
            Entry::Segments => CupertinoIcons::rectangle_split_3x1(),
            Entry::Dialogs => CupertinoIcons::exclamationmark_bubble(),
            Entry::Scrolling => CupertinoIcons::arrow_up_down(),
            Entry::Icons => CupertinoIcons::sparkles(),
            Entry::Text => CupertinoIcons::textformat(),
            Entry::TextFields => CupertinoIcons::text_cursor(),
            Entry::Heroes => CupertinoIcons::wand_stars(),
            Entry::Indicators => CupertinoIcons::speedometer(),
            Entry::Theming => CupertinoIcons::moon_fill(),
            Entry::Lists => CupertinoIcons::list_bullet(),
            Entry::ExpansionTiles => CupertinoIcons::chevron_down_square(),
        }
    }

    /// The badge's fill.
    pub fn tint(self) -> AnyColor {
        match self {
            Entry::Navigation => CupertinoColors::SYSTEM_BLUE,
            Entry::PageScaffold => CupertinoColors::SYSTEM_INDIGO,
            Entry::Buttons => CupertinoColors::SYSTEM_GREEN,
            Entry::Segments => CupertinoColors::SYSTEM_YELLOW,
            Entry::Dialogs => CupertinoColors::SYSTEM_ORANGE,
            Entry::Scrolling => CupertinoColors::SYSTEM_TEAL,
            Entry::Icons => CupertinoColors::SYSTEM_PINK,
            Entry::Text => CupertinoColors::SYSTEM_PURPLE,
            Entry::TextFields => CupertinoColors::SYSTEM_INDIGO,
            Entry::Heroes => CupertinoColors::SYSTEM_RED,
            Entry::Indicators => CupertinoColors::SYSTEM_CYAN,
            Entry::Theming => CupertinoColors::SYSTEM_GREY,
            Entry::Lists => CupertinoColors::SYSTEM_BROWN,
            Entry::ExpansionTiles => CupertinoColors::SYSTEM_MINT,
        }
    }

    /// How many screens this entry can push beyond its own.
    ///
    /// Only the tests read this; it is here so a new sub-page cannot be added without the
    /// tests reaching it.
    pub fn sub_page_count(self) -> usize {
        match self {
            Entry::Navigation => pages::navigation::SUB_TITLES.len(),
            Entry::PageScaffold => 1,
            Entry::Heroes => pages::heroes::SUBJECTS.len(),
            Entry::Theming => 1,
            _ => 0,
        }
    }

    /// The entry's own screen.
    pub fn body(self, app: &mut App, context: BuildContext) -> WidgetRef {
        match self {
            Entry::Navigation => pages::navigation::body(app, context),
            Entry::PageScaffold => pages::scaffold::body(app, context),
            Entry::Buttons => pages::buttons::body(app, context),
            Entry::Segments => pages::segments::body(app, context),
            Entry::Dialogs => pages::dialogs::body(app, context),
            Entry::Scrolling => pages::scrolling::body(app, context),
            Entry::Icons => pages::icons::body(app, context),
            Entry::Text => pages::text::body(app, context),
            Entry::TextFields => pages::text_fields::body(app, context),
            Entry::Heroes => pages::heroes::body(app, context),
            Entry::Indicators => pages::indicators::body(app, context),
            Entry::Theming => pages::theming::body(app, context),
            Entry::Lists => pages::lists::body(app, context),
            Entry::ExpansionTiles => pages::expansion_tiles::body(app, context),
        }
    }

    /// The title of one of this entry's sub-screens, and the label the screen after it reads
    /// on its back button.
    pub fn sub_title(self, index: usize) -> &'static str {
        match self {
            Entry::Navigation => pages::navigation::SUB_TITLES[index],
            Entry::PageScaffold => pages::scaffold::SUB_TITLE,
            Entry::Heroes => pages::heroes::SUBJECTS[index].title,
            Entry::Theming => pages::theming::SUB_TITLE,
            _ => panic!("{self:?} has no sub-page {index}"),
        }
    }

    /// One of this entry's sub-screens.
    pub fn sub_body(self, index: usize, app: &mut App, context: BuildContext) -> WidgetRef {
        match self {
            Entry::Navigation => pages::navigation::sub_body(index, app, context),
            Entry::PageScaffold => pages::scaffold::sub_body(app, context),
            Entry::Heroes => pages::heroes::sub_body(index, app, context),
            Entry::Theming => pages::theming::sub_body(app, context),
            _ => panic!("{self:?} has no sub-page {index}"),
        }
    }
}
