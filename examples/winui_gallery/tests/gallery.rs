//! The navigation gallery keeps one feature visible and retains visited demo state.
#![feature(arbitrary_self_types)]
mod common;
use common::GalleryFixtureExt;

use common::Fixture;
use winui_gallery::Feature;

#[test]
fn gallery_navigates_retains_examples_and_switches_theme() {
    let mut fixture = Fixture::new([1080, 780]);
    fixture.capture("gallery_button_light");
    fixture.tap("Standard");
    fixture.tap("Accent");
    fixture.find("Clicked 2 times · wifi true · airplane false");
    fixture.tap("Disabled");
    fixture.find("Clicked 2 times · wifi true · airplane false");

    fixture.navigate(Feature::ToggleSwitch);
    fixture.tap("Wi-Fi");
    fixture.tap("Airplane mode off");
    fixture.find("Clicked 2 times · wifi false · airplane true");

    fixture.navigate(Feature::ToggleButton);
    fixture.tap("Toggle me");
    fixture.find("ToggleButton: checked · three-state unchecked");
    fixture.navigate(Feature::Slider);
    fixture.find("Slider: 42 · stepped 50 · vertical 30");
    fixture.capture("gallery_slider_light");
    fixture.navigate(Feature::ToggleButton);
    fixture.find("ToggleButton: checked · three-state unchecked");

    fixture.tap("Dark theme");
    fixture.find("Light theme");
    fixture.capture("gallery_toggle_button_dark");
    fixture.navigate(Feature::Acrylic);
    fixture.find("Opaque fallback");
    fixture.capture("gallery_acrylic_dark");
    fixture.navigate(Feature::TabView);
    fixture.find("Page 0");
    fixture.capture("gallery_tabs_dark");
    fixture.navigate(Feature::NavigationView);
    fixture.find(Feature::NavigationView.description());
    fixture.capture("gallery_navigation_dark");

    fixture.navigate(Feature::SplitView);
    fixture.tap("Close pane");
    fixture.find("Open pane");
    fixture.tap("Open pane");
    fixture.pump();
    fixture.find("CompactInline · PaneOpened");
    fixture.capture("gallery_split_view_dark");

    fixture.navigate(Feature::Button);
    fixture.find("Clicked 2 times · wifi false · airplane true");
}

#[test]
fn every_feature_has_a_destination_and_small_windows_use_the_pane_toggle() {
    let mut fixture = Fixture::new([1080, 780]);
    for feature in Feature::ALL {
        fixture.navigate(feature);
        fixture.find(feature.description());
    }
    let mut narrow = Fixture::new([640, 720]);
    narrow.find(Feature::Button.description());
    // The compact shell keeps the demo usable with its navigation pane closed.
    narrow.tap("Standard");
    narrow.find("Clicked 1 times · wifi true · airplane false");
    narrow.send(
        inset_embedder::PointerChange::Down,
        inset_embedder::Offset::new(24.0, 24.0),
    );
    narrow.send(
        inset_embedder::PointerChange::Up,
        inset_embedder::Offset::new(24.0, 24.0),
    );
    narrow.pump();
    narrow.navigate(Feature::Slider);
    narrow.find(Feature::Slider.description());
    narrow.capture("gallery_narrow");
}

#[test]
fn navigation_demo_minimal_header_clears_back_and_toggle_buttons() {
    use inset_embedder::Offset;
    use inset_foundation::ValueKey;
    use inset_rendering::{RenderBox, RenderParagraph};
    use inset_widgets::{Offstage, downcast_widget};
    use inset_winui::{NavigationView, NavigationViewPaneDisplayMode};

    let mut f = Fixture::for_feature([1080, 900], Feature::NavigationView);
    f.tap("Mode: Left");
    f.tap("Mode: LeftCompact");
    f.pump();
    let elements = f.elements();
    let app = f.cell.borrow();
    let mut title = None;
    let mut buttons = Vec::new();
    for element in elements {
        let mut parent = Some(element);
        let mut minimal_demo = false;
        let mut visible = true;
        while let Some(ancestor) = parent {
            let widget = ancestor.widget(&app);
            if downcast_widget::<Offstage>(&**widget).is_some_and(|w| w.offstage) {
                visible = false;
                break;
            }
            if let Some(nav) = downcast_widget::<NavigationView>(&**widget) {
                minimal_demo = nav.pane_display_mode == NavigationViewPaneDisplayMode::LeftMinimal;
                break;
            }
            parent = ancestor.parent(&app);
        }
        if !visible || !minimal_demo {
            continue;
        }
        let widget = element.widget(&app);
        if widget.key().is_some_and(|key| {
            ["NavigationViewBackButton", "TogglePaneButton"]
                .iter()
                .any(|name| key.as_ref().eq_key(&ValueKey::new(*name)))
        }) {
            let render = element.find_render_object(&app).unwrap().as_box().unwrap();
            buttons.push(render.local_to_global(&app, Offset::ZERO, None) & render.size(&app));
        }
        if let Some(paragraph) = element
            .render_object(&app)
            .and_then(|r| r.downcast::<RenderParagraph>(&app))
            && paragraph.text(&app).to_plain_text(true, true) == "overview"
        {
            let render = paragraph.as_box();
            title = Some(render.local_to_global(&app, Offset::ZERO, None) & render.size(&app));
        }
    }
    let title = title.expect("the selected page uses NavigationView.Header");
    assert_eq!(buttons.len(), 2);
    for button in buttons {
        assert!(
            title.top >= button.bottom,
            "header {title:?} overlaps button {button:?}"
        );
    }
    drop(app);
    f.capture("gallery_navigation_minimal_header");
}

#[test]
fn theme_footer_remains_usable_when_the_pane_is_collapsed() {
    use inset_embedder::{Offset, PointerChange};
    use inset_widgets::downcast_widget;
    use inset_winui::{FluentSymbol, NavigationView};

    let mut fixture = Fixture::new([1080, 780]);
    fixture.tap("Standard");
    fixture.tap("Dark theme");
    fixture.find("Light theme");
    fixture.capture("gallery_theme_expanded");

    let toggle = Offset::new(24.0, 24.0);
    fixture.send(PointerChange::Down, toggle);
    fixture.send(PointerChange::Up, toggle);
    fixture.pump();
    // The default 20px icon is laid out at its natural size, then scaled to IconBox.
    let elements = fixture.elements();
    {
        let app = fixture.cell.borrow();
        let element = elements
            .iter()
            .find(|element| {
                downcast_widget::<inset_winui::FontIcon>(element.widget(&app).as_ref()).is_some_and(
                    |icon| icon.glyph == FluentSymbol::WeatherSunny.glyph().to_string(),
                )
            })
            .unwrap();
        let render = element.find_render_object(&app).unwrap().as_box().unwrap();
        let size = render.size(&app);
        assert!(
            size.height() > 16.0,
            "icon content must not be constrained to 16px"
        );
        let origin = render.local_to_global(&app, Offset::ZERO, None);
        let bottom = render.local_to_global(&app, Offset::new(0.0, size.height()), None);
        assert!((bottom.dy() - origin.dy() - 16.0).abs() < 0.01);
    }
    let icon = fixture.find(&FluentSymbol::WeatherSunny.glyph().to_string());
    assert!(icon.dx() < 48.0, "theme icon must fit in the compact rail");
    fixture.send(PointerChange::Down, icon);
    fixture.send(PointerChange::Up, icon);
    fixture.pump();
    fixture.find(&FluentSymbol::WeatherMoon.glyph().to_string());
    fixture.find("Clicked 1 times · wifi true · airplane false");
    fixture.capture("gallery_theme_compact");

    let elements = fixture.elements();
    let app = fixture.cell.borrow();
    let navigation = elements
        .iter()
        .find_map(|element| downcast_widget::<NavigationView>(element.widget(&app).as_ref()))
        .unwrap();
    assert_eq!(navigation.selected_item.as_deref(), Some("Button"));
    assert!(navigation.pane_footer.is_none());
    assert!(!navigation.footer_menu_items[0].selects_on_invoked);
}
