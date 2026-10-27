//! Top navigation measures custom chrome and hidden items through the native layout tree.
#![feature(arbitrary_self_types)]
use inset_foundation::{App, Handle};
use inset_rendering::{RenderBox, RenderParagraph};
use inset_widgets::*;
use inset_winui::*;
use inset_winui_test_support::Fixture;
use std::{cell::Cell, rc::Rc};

#[derive(Debug)]
struct Page(Rc<Cell<Option<Handle<PageState>>>>);
struct PageState {
    state: StateData<Page>,
    width: f64,
    custom_width: f64,
    hide_last: bool,
    long_last: bool,
}
impl StatefulWidget for Page {
    type State = PageState;
    fn create_state(&self) -> Self::State {
        PageState {
            state: StateData::new(),
            width: 1100.0,
            custom_width: 0.0,
            hide_last: false,
            long_last: false,
        }
    }
}
impl State for PageState {
    type Widget = Page;
    inset_widgets::state_accessors!();
    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        self.widget(app).0.set(Some(self));
        let s = app.get(self);
        let items = (0..6)
            .map(|i| {
                NavigationViewItem::text(
                    i.to_string(),
                    if i == 5 && s.long_last {
                        "A remarkably long destination label".to_owned()
                    } else {
                        format!("Destination {i}")
                    },
                )
                .is_visible(i != 5 || !s.hide_last)
            })
            .collect();
        let mut nav =
            NavigationView::new(items, Some("0".to_owned()), |_, _| {}, SizedBox::expand());
        nav.pane_display_mode = NavigationViewPaneDisplayMode::Top;
        nav.is_back_button_visible = NavigationViewBackButtonVisible::Collapsed;
        nav.is_settings_visible = false;
        nav.pane_custom_content = Some(
            SizedBox::new()
                .width(s.custom_width)
                .height(20.0)
                .into_widget(),
        );
        ThemeScope::new(
            Theme::Light,
            Align::new()
                .alignment(inset_painting::Alignment::TOP_LEFT.into())
                .child(SizedBox::new().width(s.width).height(200.0).child(nav)),
        )
        .into_widget()
    }
}
fn fixture() -> (Fixture, Handle<PageState>) {
    let slot = Rc::new(Cell::new(None));
    let state = slot.clone();
    let mut f = Fixture::with_root([1200, 220], move |app| {
        winui_gallery::install_fonts(app);
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| Page(state.clone()).into_widget()),
            false,
            true,
            false,
        );
        run_app(
            app,
            WidgetsApp::new(AccentPalette::default().base)
                .debug_show_checked_mode_banner(false)
                .builder(move |_, _, _| {
                    FocusScope::new(Overlay::new().initial_entries([entry]))
                        .autofocus(true)
                        .into_widget()
                })
                .into_widget(),
        );
    });
    for _ in 0..3 {
        f.pump();
    }
    (f, slot.get().unwrap())
}
fn update(f: &mut Fixture, state: Handle<PageState>, change: impl FnOnce(&mut PageState)) {
    state.set_state(&mut f.cell.borrow_mut(), change);
    for _ in 0..3 {
        f.pump();
    }
}
fn onstage(app: &App, e: AnyElement) -> bool {
    let mut visible = true;
    e.visit_ancestor_elements(app, &mut |ancestor| {
        let w = ancestor.widget(app);
        if downcast_widget::<Offstage>(&**w).is_some_and(|offstage| offstage.offstage) {
            visible = false;
            return false;
        }
        true
    });
    visible
}
fn overflow(f: &Fixture) -> bool {
    let elements = f.elements();
    let app = f.cell.borrow();
    elements.into_iter().any(|e| {
        let widget = e.widget(&app);
        downcast_widget::<FontIcon>(&**widget)
            .is_some_and(|icon| icon.glyph == FluentSymbol::More.glyph().to_string())
            && onstage(&app, e)
    })
}
fn text_width(f: &Fixture, text: &str) -> Option<f64> {
    let elements = f.elements();
    let app = f.cell.borrow();
    elements.into_iter().find_map(|e| {
        let r = e.render_object(&app)?.downcast::<RenderParagraph>(&app)?;
        (r.text(&app).to_plain_text(true, true) == text && onstage(&app, e))
            .then(|| r.as_box().size(&app).width())
    })
}
/// Finds the UI's recovery boundary without assuming font metrics or item-padding totals.
fn recovery_width(f: &mut Fixture, state: Handle<PageState>) -> f64 {
    let (mut low, mut high) = (200.0, 1100.0);
    update(f, state, |s| s.width = high);
    assert!(!overflow(f));
    for _ in 0..10 {
        let middle = (low + high) / 2.0;
        // Start narrow so every probe follows the same source recovery hysteresis path.
        update(f, state, |s| s.width = 200.0);
        update(f, state, |s| s.width = middle);
        if overflow(f) {
            low = middle;
        } else {
            high = middle;
        }
    }
    high
}
#[test]
fn custom_slot_minimum_is_counted_once_and_wider_content_forces_overflow() {
    let r = ThemeResources::new(Theme::Light, AccentPalette::default());
    let (mut f, state) = fixture();
    let empty = recovery_width(&mut f, state);
    update(&mut f, state, |s| {
        s.custom_width = r.top_navigation_view_pane_custom_content_min_width
    });
    let minimum = recovery_width(&mut f, state);
    assert!(
        (empty - minimum).abs() < 2.0,
        "empty and minimum-width content share the same slot: {empty} vs {minimum}"
    );
    update(&mut f, state, |s| {
        s.width = minimum + 10.0;
        s.custom_width = r.top_navigation_view_pane_custom_content_min_width;
    });
    assert!(!overflow(&f));
    update(&mut f, state, |s| s.custom_width = 220.0);
    assert!(
        overflow(&f),
        "wider custom content must reduce the space for primary items"
    );
    update(&mut f, state, |s| s.custom_width = 0.0);
    assert!(
        !overflow(&f),
        "shrinking chrome must recover overflow items"
    );
}
#[test]
fn hidden_item_has_zero_width_and_becoming_visible_remeasures_it() {
    let (mut f, state) = fixture();
    update(&mut f, state, |s| s.long_last = true);
    let label = "A remarkably long destination label";
    let width = text_width(&f, label).expect("wide host displays the long item");
    let all_items = recovery_width(&mut f, state);
    update(&mut f, state, |s| {
        s.width = all_items - width / 2.0;
        s.hide_last = true;
    });
    assert!(!overflow(&f), "hidden item must contribute zero width");
    assert!(text_width(&f, label).is_none());
    update(&mut f, state, |s| s.hide_last = false);
    assert!(
        overflow(&f),
        "newly visible item must regain its measured width"
    );
    update(&mut f, state, |s| s.hide_last = true);
    assert!(
        !overflow(&f),
        "hiding again must discard the previous cached width"
    );
    update(&mut f, state, |s| {
        s.hide_last = false;
        s.width = 1100.0;
    });
    assert!(!overflow(&f));
    assert!(text_width(&f, label).is_some());
}
