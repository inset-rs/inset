//! ToolTipService through native input, timers, overlays, layout and GPU rendering.
#![feature(arbitrary_self_types)]

use inset_embedder::{Offset, PointerChange, Rect};
use inset_foundation::{App, Handle, Listener};
use inset_rendering::RenderParagraph;
use inset_services::{
    HardwareKeyboard, KeyDownEvent, KeyEvent, KeyUpEvent, LogicalKeyboardKey, PhysicalKeyboardKey,
};
use inset_widgets::*;
use inset_winui::*;
use inset_winui_test_support::Fixture;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

/// Test owner state and events recorded by the tooltip.
#[derive(Clone, Debug, Default)]
struct Probe {
    state: Rc<Cell<Option<Handle<PageState>>>>,
    events: Rc<RefCell<Vec<&'static str>>>,
    clicks: Rc<Cell<u32>>,
}

/// Places an owner near the top edge so Top placement must flip below it.
#[derive(Debug)]
struct Page(Probe);

/// Mutable tooltip configuration used to exercise transitions without replacing its owner.
struct PageState {
    state: StateData<Page>,
    explicit: Option<bool>,
    enabled: bool,
    theme: Theme,
    show_owner: bool,
}

impl StatefulWidget for Page {
    type State = PageState;

    fn create_state(&self) -> PageState {
        PageState {
            state: StateData::new(),
            explicit: None,
            enabled: true,
            theme: Theme::Light,
            show_owner: true,
        }
    }
}

impl State for PageState {
    type Widget = Page;
    inset_widgets::state_accessors!();

    fn build(self: Handle<Self>, app: &mut App, _: BuildContext) -> WidgetRef {
        let p = self.widget(app).0.clone();
        p.state.set(Some(self));
        let s = app.get(self);
        let (explicit, enabled, theme, show_owner) = (s.explicit, s.enabled, s.theme, s.show_owner);
        let open_events = p.events.clone();
        let close_events = p.events.clone();
        let clicks = p.clicks.clone();
        let tooltip = ToolTip::text("Tooltip detail")
            .opened(Listener::new(move |_| {
                open_events.borrow_mut().push("opened")
            }))
            .closed(Listener::new(move |_| {
                close_events.borrow_mut().push("closed")
            }));
        let mut owner = ToolTipService::new(
            Button::text(
                "Owner",
                Listener::new(move |_| clicks.set(clicks.get() + 1)),
            ),
            tooltip,
        )
        .is_enabled(enabled);
        owner.is_open = explicit;
        ThemeScope::new(
            theme,
            Stack::new().children(if show_owner {
                vec![Positioned::new(owner).left(100.0).top(2.0).into_widget()]
            } else {
                vec![]
            }),
        )
        .into_widget()
    }
}

/// Mounts the service under the required native root Overlay.
fn fixture() -> (Fixture, Probe) {
    let p = Probe::default();
    let page = p.clone();
    let fixture = Fixture::with_root([360, 220], move |app| {
        winui_gallery::install_fonts(app);
        let entry = OverlayEntry::new(
            app,
            Rc::new(move |_, _| Page(page.clone()).into_widget()),
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
    (fixture, p)
}

/// Advances timers, then pumps the frames they requested.
fn advance(f: &mut Fixture, duration: Duration) {
    f.cell.elapse(duration);
    f.pump();
}

/// Updates the existing page state.
fn update(f: &mut Fixture, p: &Probe, change: impl FnOnce(&mut PageState)) {
    p.state
        .get()
        .unwrap()
        .set_state(&mut f.cell.borrow_mut(), change);
    f.cell.checkpoint();
    f.pump();
}

/// Finds a rendered text rectangle without requiring the label to exist.
fn bounds(f: &Fixture, text: &str) -> Option<Rect> {
    let elements = f.elements();
    let app = f.cell.borrow();
    elements.into_iter().find_map(|e| {
        let o = e.render_object(&app)?;
        let p = o.downcast::<RenderParagraph>(&app)?;
        if p.text(&app).to_plain_text(true, true) != text {
            return None;
        }
        let b = o.as_box()?;
        Some(b.local_to_global(&app, Offset::ZERO, None) & b.size(&app))
    })
}

/// Dispatches a full keyboard press through HardwareKeyboard.
fn key(f: &mut Fixture, logical: LogicalKeyboardKey, physical: PhysicalKeyboardKey) {
    let mut app = f.cell.borrow_mut();
    let keyboard = HardwareKeyboard::instance(&mut app);
    keyboard.handle_key_event(
        &mut app,
        &KeyEvent::Down(KeyDownEvent::new(physical, logical, f.at)),
    );
    keyboard.handle_key_event(
        &mut app,
        &KeyEvent::Up(KeyUpEvent::new(physical, logical, f.at)),
    );
}

#[test]
fn hover_delay_safe_zone_pointer_dismissal_and_theme_rendering() {
    let (mut f, p) = fixture();
    let owner = f.find("Owner");
    f.send_mouse(PointerChange::Hover, owner, 0);
    advance(&mut f, Duration::from_millis(799));
    assert!(bounds(&f, "Tooltip detail").is_none());
    advance(&mut f, Duration::from_millis(1));
    f.pump();
    let tip = bounds(&f, "Tooltip detail").expect("800 ms opens mouse tooltip");
    assert!(tip.top > owner.dy(), "top-edge target flips below: {tip:?}");
    assert_eq!(&*p.events.borrow(), &["opened"]);
    f.capture("tool_tip_light");
    f.send_mouse(PointerChange::Hover, tip.center(), 0);
    f.pump();
    assert!(bounds(&f, "Tooltip detail").is_some());
    update(&mut f, &p, |s| s.theme = Theme::Dark);
    f.pump();
    f.capture("tool_tip_dark");
    f.send(PointerChange::Down, Offset::new(330.0, 190.0));
    f.send(PointerChange::Up, Offset::new(330.0, 190.0));
    advance(&mut f, Duration::from_millis(150));
    assert!(bounds(&f, "Tooltip detail").is_none());
    assert_eq!(&*p.events.borrow(), &["opened", "closed"]);
}

#[test]
fn explicit_open_ignores_automatic_wait_and_pending_callbacks_do_not_outlive_owner() {
    let (mut f, p) = fixture();
    update(&mut f, &p, |s| s.explicit = Some(true));
    f.pump();
    assert!(bounds(&f, "Tooltip detail").is_some());
    update(&mut f, &p, |s| s.explicit = Some(false));
    update(&mut f, &p, |s| s.explicit = Some(true));
    advance(&mut f, Duration::from_millis(200));
    assert!(
        bounds(&f, "Tooltip detail").is_some(),
        "reopening cancels pending removal"
    );
    update(&mut f, &p, |s| s.show_owner = false);
    advance(&mut f, Duration::from_secs(2));
    assert!(bounds(&f, "Tooltip detail").is_none());
}

#[test]
fn keyboard_focus_opens_after_delay_and_escape_closes_without_stealing_focus() {
    let (mut f, p) = fixture();
    key(&mut f, LogicalKeyboardKey::TAB, PhysicalKeyboardKey::TAB);
    f.pump();
    advance(&mut f, Duration::from_millis(800));
    f.pump();
    assert!(bounds(&f, "Tooltip detail").is_some());
    key(
        &mut f,
        LogicalKeyboardKey::ESCAPE,
        PhysicalKeyboardKey::ESCAPE,
    );
    advance(&mut f, Duration::from_millis(150));
    assert!(bounds(&f, "Tooltip detail").is_none());
    key(
        &mut f,
        LogicalKeyboardKey::SPACE,
        PhysicalKeyboardKey::SPACE,
    );
    f.pump();
    assert_eq!(p.clicks.get(), 1);
}
