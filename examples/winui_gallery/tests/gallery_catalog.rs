//! The expanded catalog exposes its symbols and retains independent example settings.
#![feature(arbitrary_self_types)]
mod common;
use common::GalleryFixtureExt;

use common::Fixture;
use inset_widgets::downcast_widget;
use inset_winui::NavigationView;
use std::collections::HashSet;
use winui_gallery::Feature;

#[test]
fn catalog_icons_and_new_pages_work_across_navigation() {
    let mut f = Fixture::for_feature([1080, 900], Feature::Icons);
    let entries = f
        .elements()
        .into_iter()
        .find_map(|element| {
            let app = f.cell.borrow();
            let widget = element.widget(&app);
            let nav = downcast_widget::<NavigationView>(&**widget)?;
            (nav.pane_title == "WinUI Gallery").then(|| nav.menu_items.clone())
        })
        .unwrap();
    assert_eq!(entries.len(), Feature::ALL.len());
    assert!(entries.iter().all(|item| item.icon.is_some()));
    let symbols: HashSet<_> = Feature::ALL
        .into_iter()
        .map(|feature| feature.symbol().glyph())
        .collect();
    assert_eq!(symbols.len(), Feature::ALL.len());

    f.tap("Icon size: 20");
    f.find("Icon size: 24");
    f.tap("Accent icons");
    f.capture("gallery_icons_light");
    f.navigate(Feature::Typography);
    f.tap("Text style: Body");
    f.tap("Text style: Body Strong");
    f.find("Text style: Body Large");
    f.tap("Secondary foreground");
    f.capture("gallery_typography_light");
    f.tap("Dark theme");
    f.capture("gallery_typography_dark");
    f.navigate(Feature::Icons);
    f.find("Icon size: 24");
    f.capture("gallery_icons_dark");
}

#[test]
fn symbol_catalog_is_usable_in_a_narrow_window() {
    let mut f = Fixture::for_feature([640, 900], Feature::Icons);
    f.tap("Icon size: 20");
    f.tap("Accent icons");
    f.capture("gallery_icons_narrow");
    f.ensure_visible("Right-to-left icons");
    f.tap("Right-to-left icons");
    f.capture("gallery_icon_direction_narrow");
}
