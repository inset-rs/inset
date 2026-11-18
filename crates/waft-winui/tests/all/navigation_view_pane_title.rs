//! A left `NavigationView`'s pane title without the pane toggle: `PaneTitleHolder` shows it over
//! the pane, and the template's `TogglePaneButtonVisible` state, which WinUI enters for a shown
//! title as for a shown toggle, keeps its row clear above the search box.

use waft_embedder::TextDirection;
use waft_widgets::testing::{WidgetTester, find, test_widgets};
use waft_widgets::*;
use waft_winui::*;

/// A left pane with a title, a search box and one item, its toggle and back button hidden.
fn navigation(tester: &WidgetTester) -> WidgetRef {
    let controller = tester.update(|app| TextEditingController::new(app).into_erased());
    let navigation = NavigationView::new(
        vec![NavigationViewItem::text("all", "All")],
        Some("all".to_owned()),
        |_, _| {},
        Text::new("Page content"),
    )
    .pane_display_mode(NavigationViewPaneDisplayMode::Left)
    .pane_title("Library")
    .is_settings_visible(false)
    .is_pane_toggle_button_visible(false)
    .is_back_button_visible(NavigationViewBackButtonVisible::Collapsed)
    .auto_suggest_box(TextBox::new(controller).placeholder_text("Search"));
    let home = Overlay::wrap(ThemeScope::new(
        Theme::Light,
        SizedBox::new().width(800.0).height(400.0).child(navigation),
    ))
    .into_widget();
    Directionality::new(
        TextDirection::Ltr,
        WidgetsApp::new(AccentPalette::default().base)
            .builder(move |_, _, _| home.clone())
            .into_widget(),
    )
    .into_widget()
}

#[test]
fn a_pane_title_without_the_toggle_sits_above_the_search_box() {
    test_widgets(|tester| {
        let root = navigation(tester);
        tester.pump_widget(root);

        let title = tester.get_rect(find::text("Library"));
        let search = tester.get_rect(find::by_type::<TextBox>());
        assert!(
            title.bottom <= search.top,
            "the title {title:?} overlaps the search box {search:?}"
        );
        assert!(
            title.left >= 16.0,
            "the title {title:?} does not take NavigationViewItemInnerHeaderMargin's 16 px inset"
        );
    });
}
