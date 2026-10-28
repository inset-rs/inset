#![feature(arbitrary_self_types)]
//! How a multiline `TextBox` sizes itself: it grows with its text from one line up to a parent's
//! height cap or its `max_lines`, then scrolls inside to keep the caret in view.

use inset_embedder::{SystemFontSource, TargetPlatform, TextAffinity, TextSelection};
use inset_foundation::{App, AppCell};
use inset_painting::PaintingBinding;
use inset_rendering::{BoxConstraints, RenderEditable};
use inset_scheduler::SchedulerBinding;
use inset_services::{TextEditingValue, TextInput};
use inset_test::{IMPLICIT_VIEW_ID, TestPlatform, TestView};
use inset_widgets::*;
use inset_winui::{AccentPalette, TextBox};
use std::{rc::Rc, time::Duration};

/// The parent cap standing in for WinUI's `MaxHeight`.
const CAP: f64 = 120.0;

/// `count` lines of `"line\n"`, leaving the caret on an empty line after them.
fn lines(count: usize) -> String {
    "line\n".repeat(count)
}

fn capped(text_box: TextBox) -> WidgetRef {
    ConstrainedBox::new(BoxConstraints::new().max_height(CAP))
        .child(text_box)
        .into_widget()
}

struct Fixture {
    cell: Rc<AppCell>,
    time: Duration,
}

impl Fixture {
    /// Mounts a focused, empty, 300 px wide multiline `TextBox`, inside what `wrap` makes of it,
    /// at the top left of an 800x600 macOS view with the host's fonts.
    fn new(wrap: impl FnOnce(TextBox) -> WidgetRef) -> Self {
        let platform = TestPlatform::new()
            .on(TargetPlatform::MacOS)
            .with_view(Rc::new(TestView::new(800.0, 600.0)))
            .with_font_source(|| Box::new(SystemFontSource::platform()));
        let cell = AppCell::with_platform(Rc::new(platform));
        {
            let mut app = cell.borrow_mut();
            PaintingBinding::instance(&mut app).install_platform_fonts(&mut app);
            let controller = TextEditingController::new(&mut app).as_controller();
            let text_box = wrap(TextBox::new(controller).accepts_return(true));
            // The overlay holds the focused editor's selection overlay.
            let home = Overlay::wrap(
                Align::new()
                    .alignment(AlignmentGeometry::TOP_LEFT)
                    .child(SizedBox::new().width(300.0).child(text_box)),
            )
            .into_widget();
            let root = WidgetsApp::new(AccentPalette::default().base)
                .builder(move |_, _, _| home.clone())
                .into_widget();
            run_app(&mut app, root);
        }
        cell.elapse(Duration::ZERO);
        let mut fixture = Self {
            cell,
            time: Duration::ZERO,
        };
        fixture.pump();
        let node = fixture.find(|app, element| {
            downcast_widget::<EditableText>(element.widget(app).as_ref()).map(|w| w.focus_node)
        });
        node.as_node()
            .request_focus(&mut fixture.cell.borrow_mut(), None);
        fixture.pump();
        fixture
    }

    /// Ten 20 ms frames, past the 100 ms animation that scrolls the caret into view.
    fn pump(&mut self) {
        let (cell, views) = (&self.cell, [IMPLICIT_VIEW_ID]);
        for _ in 0..10 {
            self.time += Duration::from_millis(20);
            SchedulerBinding::handle_begin_frame(&mut cell.borrow_mut(), &views, Some(self.time));
            cell.checkpoint();
            SchedulerBinding::handle_draw_frame(&mut cell.borrow_mut(), &views);
            cell.checkpoint();
        }
    }

    /// The host IME replacing the text with `text` and the caret at its end, as typing does.
    fn type_text(&mut self, text: &str) {
        let caret = text.encode_utf16().count() as i32;
        let value = TextEditingValue::new()
            .text(text)
            .selection(TextSelection::collapsed(caret, TextAffinity::Downstream));
        let mut app = self.cell.borrow_mut();
        TextInput::instance(&mut app).update_editing_value(&mut app, value);
        drop(app);
        self.cell.checkpoint();
        self.pump();
    }

    /// What `pick` gives for the one mounted element it gives anything for.
    fn find<T>(&self, pick: impl Fn(&App, AnyElement) -> Option<T>) -> T {
        let mut app = self.cell.borrow_mut();
        let mut stack = Vec::from_iter(WidgetsBinding::instance(&mut app).root_element(&app));
        while let Some(element) = stack.pop() {
            if let Some(found) = pick(&app, element) {
                return found;
            }
            stack.extend(element.children(&app));
        }
        panic!("no such element")
    }

    /// The whole control's height.
    fn height(&self) -> f64 {
        self.find(|app, element| {
            downcast_widget::<TextBox>(element.widget(app).as_ref())?;
            let render = element.find_render_object(app)?.as_box()?;
            Some(render.size(app).height())
        })
    }

    /// Asserts the editor scrolls and sits at its end, where the caret is.
    fn assert_scrolled_to_caret(&self) {
        let (pixels, max_scroll_extent) = self.find(|app, element| {
            let render = element.render_object(app)?;
            render.downcast::<RenderEditable>(app)?;
            let position = element
                .find_ancestor_state_of_type::<ScrollableState>(app)?
                .position(app);
            Some((position.pixels(app), position.max_scroll_extent(app)))
        });
        assert!(max_scroll_extent > 0.0);
        assert_eq!(pixels, max_scroll_extent);
    }
}

/// The height of an uncapped multiline `TextBox` holding `text`.
fn uncapped_height(text: &str) -> f64 {
    let mut fixture = Fixture::new(|text_box| text_box.into_widget());
    fixture.type_text(text);
    fixture.height()
}

/// Under a parent cap the box keeps its one-line height, grows to exactly the cap, then scrolls.
#[test]
fn a_parent_cap_stops_growth_and_keeps_the_caret_in_view() {
    let mut fixture = Fixture::new(capped);
    fixture.type_text("line");
    assert_eq!(fixture.height(), uncapped_height("line"));
    fixture.type_text(&lines(20));
    assert_eq!(fixture.height(), CAP);
    fixture.assert_scrolled_to_caret();
}

/// `max_lines(4)` grows from one line to four lines plus the chrome, then holds and scrolls.
#[test]
fn max_lines_stops_growth_and_keeps_the_caret_in_view() {
    let four_lines = uncapped_height("line\nline\nline\nline");
    let mut fixture = Fixture::new(|text_box| text_box.max_lines(4).into_widget());
    fixture.type_text("line");
    assert_eq!(fixture.height(), uncapped_height("line"));
    fixture.type_text(&lines(20));
    assert_eq!(fixture.height(), four_lines);
    assert!(four_lines < uncapped_height(&lines(20)));
    fixture.assert_scrolled_to_caret();
    fixture.type_text(&lines(30));
    assert_eq!(fixture.height(), four_lines);
}

/// A header counts against a parent cap: the whole control stays at the cap.
#[test]
fn a_header_stays_inside_a_parent_cap() {
    let mut fixture = Fixture::new(|text_box| capped(text_box.header(Text::new("Notes"))));
    fixture.type_text(&lines(20));
    assert_eq!(fixture.height(), CAP);
    fixture.assert_scrolled_to_caret();
}
