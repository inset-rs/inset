//! A `SelectorBar` item's icon keeps its shape: the template's `IconTransform` sets both `ScaleX`
//! and `ScaleY` to `SelectorBarItemIconScale`.

use waft_embedder::{Matrix4, TextDirection};
use waft_foundation::App;
use waft_widgets::testing::WidgetTester;
use waft_widgets::*;
use waft_winui::{
    FluentSymbol, FontIcon, SelectorBar, SelectorBarItem, Theme, ThemeScope, install_icon_font,
};

/// `SelectorBarItemIconScale` of the template's theme dictionary.
const ICON_SCALE: f32 = 0.8;

/// The one mounted `FontIcon` element.
fn font_icon(app: &mut App) -> AnyElement {
    let binding = WidgetsBinding::instance(app);
    let mut stack = Vec::from_iter(binding.root_element(app));
    while let Some(element) = stack.pop() {
        if downcast_widget::<FontIcon>(element.widget(app).as_ref()).is_some() {
            return element;
        }
        stack.extend(element.children(app));
    }
    panic!("no FontIcon")
}

/// The matrix of the closest `Transform` above `element`.
fn transform_above(app: &App, element: AnyElement) -> Option<Matrix4> {
    let mut transform = None;
    element.visit_ancestor_elements(app, &mut |ancestor| {
        let widget = ancestor.widget(app);
        transform = downcast_widget::<Transform>(widget.as_ref()).map(|t| t.transform);
        transform.is_none()
    });
    transform
}

#[test]
fn a_selector_bars_icon_is_scaled_evenly_on_both_axes() {
    let mut tester = WidgetTester::new();
    {
        let mut app = tester.cell.borrow_mut();
        install_icon_font(&mut app);
    }
    let item = SelectorBarItem::new("folder", "").icon(FontIcon::symbol(FluentSymbol::Folder));
    let bar = SelectorBar::new(vec![item], None, |_, _| {});
    tester.pump_widget(Directionality::new(
        TextDirection::Ltr,
        ThemeScope::new(Theme::Light, bar),
    ));

    let mut app = tester.cell.borrow_mut();
    let icon = font_icon(&mut app);
    let matrix = transform_above(&app, icon).expect("the icon's ScaleTransform");
    let matrix = matrix.to_flutter_array();
    assert!(
        (matrix[0] - ICON_SCALE).abs() < 1e-6 && (matrix[5] - ICON_SCALE).abs() < 1e-6,
        "the icon scales {0} across and {1} down, not {ICON_SCALE} both ways",
        matrix[0],
        matrix[5]
    );
}
