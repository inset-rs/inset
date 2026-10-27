//! Selection must not paint its settled target before its first animation frame.
#![feature(arbitrary_self_types)]

use inset_embedder::PointerChange;
use inset_foundation::{App, Handle};
use inset_widgets::*;
use inset_winui::*;
use inset_winui_test_support::Fixture;
use std::{cell::Cell, rc::Rc, time::Duration};

#[derive(Debug)]
struct Page {
    probe: Rc<Cell<Option<Handle<PageState>>>>,
    mode: NavigationViewPaneDisplayMode,
}

struct PageState {
    state: StateData<Page>,
    selected: String,
    mode: NavigationViewPaneDisplayMode,
    pane_open: bool,
    hierarchy: bool,
    extra_items: usize,
}

impl StatefulWidget for Page {
    type State = PageState;

    fn create_state(&self) -> PageState {
        PageState {
            state: StateData::new(),
            selected: "first".into(),
            mode: self.mode,
            pane_open: true,
            hierarchy: false,
            extra_items: 0,
        }
    }
}

impl State for PageState {
    type Widget = Page;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        self.widget(app).probe.set(Some(self));
        let state = app.get(self);
        let mut first = NavigationViewItem::text("first", "First");
        if state.hierarchy {
            first = first.menu_items([NavigationViewItem::text("child", "Child")]);
        }
        let mut items = vec![first, NavigationViewItem::text("second", "Second")];
        items.extend(
            (0..state.extra_items)
                .map(|i| NavigationViewItem::text(format!("extra{i}"), format!("Extra {i}"))),
        );
        NavigationView::new(
            items,
            Some(app.get(self).selected.clone()),
            move |app, args| self.set_state(app, |state| state.selected = args.item.unwrap().id),
            SizedBox::shrink(),
        )
        .pane_display_mode(app.get(self).mode)
        .is_pane_open(app.get(self).pane_open, move |app, open| {
            self.set_state(app, |state| state.pane_open = open)
        })
        .is_back_button_visible(NavigationViewBackButtonVisible::Collapsed)
        .into_widget()
    }
}

fn frame(f: &mut Fixture) {
    f.frame_after(Duration::from_millis(10));
}

/// Bounds of opaque accent pixels isolate the indicator from text and neutral item fills.
fn accent_pixels(f: &Fixture) -> Vec<(usize, usize)> {
    let color = ThemeResources::new(Theme::Light, AccentPalette::default())
        .navigation_view_selection_indicator_foreground
        .to_argb32();
    let rgb = [
        ((color >> 16) & 255) as i16,
        ((color >> 8) & 255) as i16,
        (color & 255) as i16,
    ];
    let mut points = Vec::new();
    for (i, pixel) in f.view.pixels.borrow().as_chunks::<4>().0.iter().enumerate() {
        if (0..3).all(|c| (pixel[c] as i16 - rgb[c]).abs() <= 2) {
            points.push((i % f.view.size[0] as usize, i / f.view.size[0] as usize));
        }
    }
    points
}

fn indicator_pixels(f: &Fixture) -> (usize, usize, usize, usize) {
    let points = accent_pixels(f);
    assert!(!points.is_empty(), "indicator must remain visible");
    (
        points.iter().map(|p| p.0).min().unwrap(),
        points.iter().map(|p| p.1).min().unwrap(),
        points.iter().map(|p| p.0).max().unwrap(),
        points.iter().map(|p| p.1).max().unwrap(),
    )
}

fn fixture(mode: NavigationViewPaneDisplayMode) -> (Fixture, Rc<Cell<Option<Handle<PageState>>>>) {
    let probe = Rc::new(Cell::new(None));
    let slot = probe.clone();
    let f = Fixture::with_root([620, 400], move |app| {
        winui_gallery::install_fonts(app);
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| {
                Page {
                    probe: slot.clone(),
                    mode,
                }
                .into_widget()
            }),
            false,
            true,
            false,
        );
        run_app(
            app,
            WidgetsApp::new(AccentPalette::default().base)
                .debug_show_checked_mode_banner(false)
                .builder(move |_, _, _| Overlay::new().initial_entries([entry]).into_widget())
                .into_widget(),
        );
    });
    (f, probe)
}

#[test]
fn selected_indicator_stays_at_origin_until_animation_begins() {
    for mode in [
        NavigationViewPaneDisplayMode::Left,
        NavigationViewPaneDisplayMode::Top,
    ] {
        let (mut f, probe) = fixture(mode);
        let from = indicator_pixels(&f);
        probe
            .get()
            .unwrap()
            .set_state(&mut f.cell.borrow_mut(), |s| s.selected = "second".into());
        frame(&mut f);
        assert_eq!(
            indicator_pixels(&f),
            from,
            "{mode:?}: first selection frame must preserve origin"
        );
        frame(&mut f);
        assert_eq!(
            indicator_pixels(&f),
            from,
            "{mode:?}: animation starts at the origin"
        );
        for _ in 0..20 {
            frame(&mut f);
        }
        let middle = indicator_pixels(&f);
        assert_ne!(middle, from, "{mode:?}: the indicator must travel");
        for _ in 0..50 {
            frame(&mut f);
        }
        let to = indicator_pixels(&f);
        assert_ne!(to, from);
        assert_ne!(to, middle);
        let point = f.find("First");
        f.send(PointerChange::Down, point);
        f.send(PointerChange::Up, point);
        frame(&mut f);
        assert_eq!(
            indicator_pixels(&f),
            to,
            "{mode:?}: invocation must also preserve origin"
        );
        for _ in 0..75 {
            frame(&mut f);
        }
        assert_eq!(
            indicator_pixels(&f),
            from,
            "{mode:?}: reverse transition settles at first item"
        );
    }
}

#[test]
fn closing_minimal_pane_hides_its_animated_indicators() {
    let (mut f, probe) = fixture(NavigationViewPaneDisplayMode::LeftMinimal);
    probe
        .get()
        .unwrap()
        .set_state(&mut f.cell.borrow_mut(), |state| state.pane_open = true);
    for _ in 0..75 {
        frame(&mut f);
    }
    assert!(!accent_pixels(&f).is_empty());
    let point = f.find("Second");
    f.send(PointerChange::Down, point);
    f.send(PointerChange::Up, point);
    for _ in 0..30 {
        frame(&mut f);
    }
    assert!(!f.cell.borrow().get(probe.get().unwrap()).pane_open);
    assert!(
        accent_pixels(&f).is_empty(),
        "closed pane cannot leave detached animation on page"
    );
    for _ in 0..40 {
        frame(&mut f);
    }
    assert!(accent_pixels(&f).is_empty());
}

#[test]
fn top_child_dismissal_keeps_animation_inside_its_visual_owners() {
    let (mut f, probe) = fixture(NavigationViewPaneDisplayMode::Top);
    probe
        .get()
        .unwrap()
        .set_state(&mut f.cell.borrow_mut(), |state| state.hierarchy = true);
    f.pump();
    f.tap("First");
    let point = f.find("Child");
    f.send(PointerChange::Down, point);
    f.send(PointerChange::Up, point);
    // Native OverlayPortal commits its hide after the closing frame.
    frame(&mut f);
    for _ in 0..35 {
        frame(&mut f);
        assert!(
            accent_pixels(&f).iter().all(|(_, y)| *y < 48),
            "closed popup cannot paint an outgoing indicator over the page: {:?}",
            indicator_pixels(&f)
        );
    }
    for _ in 0..40 {
        frame(&mut f);
    }
    let rect = indicator_pixels(&f);
    assert!(
        rect.3 < 48,
        "ancestor owns settled selection after child popup closes"
    );
}

#[test]
fn active_indicator_follows_scroll_and_mode_reparenting() {
    let (mut f, probe) = fixture(NavigationViewPaneDisplayMode::Left);
    let page = probe.get().unwrap();
    page.set_state(&mut f.cell.borrow_mut(), |state| {
        state.extra_items = 20;
        state.selected = "extra2".into();
    });
    for _ in 0..75 {
        frame(&mut f);
    }
    page.set_state(&mut f.cell.borrow_mut(), |state| {
        state.selected = "extra3".into()
    });
    for _ in 0..12 {
        frame(&mut f);
    }
    let before = indicator_pixels(&f);
    let scroll = f
        .elements()
        .into_iter()
        .find_map(|element| {
            let app = f.cell.borrow();
            let widget = element.widget(&app);
            let observer = downcast_widget::<ScrollMetricsObserver>(&**widget)?;
            (observer.controller.metrics(&app).scrollable_length > 0.0)
                .then_some(observer.controller)
        })
        .unwrap();
    scroll.scroll_by(&mut f.cell.borrow_mut(), 20.0, true);
    f.frame_after(Duration::ZERO);
    let after = indicator_pixels(&f);
    assert_eq!(after.1 + 20, before.1);
    assert_eq!(after.3 + 20, before.3);
    page.set_state(&mut f.cell.borrow_mut(), |state| {
        state.mode = NavigationViewPaneDisplayMode::Top;
        state.extra_items = 0;
        state.selected = "second".into();
    });
    for _ in 0..35 {
        frame(&mut f);
        assert!(
            accent_pixels(&f).iter().all(|(_, y)| *y < 48),
            "reparented visuals must not remain at old left-pane coordinates"
        );
    }
    for _ in 0..40 {
        frame(&mut f);
    }
    assert!(indicator_pixels(&f).3 < 48);
}

#[test]
fn collapse_and_expand_transfer_selection_between_actual_ancestor_and_child() {
    let (mut f, probe) = fixture(NavigationViewPaneDisplayMode::Left);
    let page = probe.get().unwrap();
    page.set_state(&mut f.cell.borrow_mut(), |state| state.hierarchy = true);
    f.pump();
    let ancestor = indicator_pixels(&f);
    f.tap("First");
    f.tap("Child");
    for _ in 0..75 {
        frame(&mut f);
    }
    let child = indicator_pixels(&f);
    assert_ne!(ancestor, child);
    let chevron = inset_embedder::Offset::new(292.0, f.find("First").dy());
    f.send(PointerChange::Down, chevron);
    f.send(PointerChange::Up, chevron);
    for _ in 0..20 {
        frame(&mut f);
    }
    // ShowHideChildren collapses the outgoing owner immediately. The incoming
    // indicator's translation is clipped by LayoutRoot's rounded corners, so
    // it can be wholly outside the visible item during this part of the track.
    let middle = accent_pixels(&f);
    let ancestor_y = f.find("First").dy();
    assert!(
        middle
            .iter()
            .all(|(_, y)| { (*y as f64 - ancestor_y).abs() <= 18.0 }),
        "the animation cannot escape the ancestor's 36 px LayoutRoot"
    );
    for _ in 0..55 {
        frame(&mut f);
    }
    assert_eq!(indicator_pixels(&f), ancestor);
    assert_eq!(f.cell.borrow().get(page).selected, "child");
    f.send(PointerChange::Down, chevron);
    f.send(PointerChange::Up, chevron);
    for _ in 0..75 {
        frame(&mut f);
    }
    assert_eq!(indicator_pixels(&f), child);
}
