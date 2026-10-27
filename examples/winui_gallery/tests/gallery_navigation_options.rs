//! Gallery settings configure the real controls and remain reachable after layout changes.
#![feature(arbitrary_self_types)]
mod common;
use common::GalleryFixtureExt;

use common::Fixture;
use inset_widgets::downcast_widget;
use inset_winui::{NavigationView, SplitView, TabView, TabViewCloseButtonOverlayMode};
use winui_gallery::Feature;

#[test]
fn navigation_and_split_options_reach_the_control_properties() {
    let mut fixture = Fixture::for_feature([1440, 1400], Feature::NavigationView);
    fixture.tap("Pane header and footer");
    fixture.find("Personal workspace");
    fixture.find("Signed in locally");
    fixture.capture("gallery_navigation_pane_chrome");
    let configured = fixture.elements().into_iter().any(|element| {
        let app = fixture.cell.borrow();
        downcast_widget::<NavigationView>(element.widget(&app).as_ref())
            .is_some_and(|nav| nav.pane_header.is_some() && nav.pane_footer.is_some())
    });
    assert!(configured);
    fixture.tap("Narrow navigation preview");
    fixture.capture("gallery_navigation_configured");
    fixture.navigate(Feature::SplitView);
    fixture.tap("Wide pane (240 px)");
    fixture.tap("Pane: left");
    fixture.find("Pane: right");
    let configured = fixture.elements().into_iter().any(|element| {
        let app = fixture.cell.borrow();
        downcast_widget::<SplitView>(element.widget(&app).as_ref())
            .is_some_and(|split| split.open_pane_length == Some(240.0))
    });
    assert!(configured);
    fixture.capture("gallery_split_configured");
}

#[test]
fn tab_and_tooltip_options_remain_interactive() {
    let mut fixture = Fixture::for_feature([1440, 1400], Feature::TabView);
    fixture.tap("Show add buttons");
    fixture.tap("Close buttons on hover");
    let configured = fixture
        .elements()
        .into_iter()
        .filter(|element| {
            let app = fixture.cell.borrow();
            downcast_widget::<TabView>(element.widget(&app).as_ref()).is_some_and(|tabs| {
                !tabs.is_add_tab_button_visible
                    && tabs.close_button_overlay_mode
                        == TabViewCloseButtonOverlayMode::OnPointerOver
            })
        })
        .count();
    assert_eq!(configured, 2);
    fixture.capture("gallery_tabs_configured");
    fixture.navigate(Feature::ToolTip);
    fixture.tap("Tooltip placement: Top");
    fixture.find("Tooltip placement: Bottom");
    fixture.tap("Show tooltip explicitly");
    fixture.pump();
    fixture.find("A preferred side can change near the window edge.");
    fixture.capture("gallery_tooltip_configured");
}

/// Captures both option orientations at normal and compact gallery widths.
#[test]
fn navigation_examples_fit_normal_and_narrow_gallery_widths() {
    for width in [1080, 640] {
        for feature in [
            Feature::NavigationView,
            Feature::TabView,
            Feature::SplitView,
            Feature::ToolTip,
        ] {
            let fixture = Fixture::for_feature([width, 900], feature);
            fixture.find(feature.description());
            fixture.capture(&format!("gallery_options_{feature:?}_{width}"));
        }
    }
}
