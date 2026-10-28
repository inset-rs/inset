//! Shared test scaffolding: a platform with one view to run widgets under, frames, taps, and
//! walks of the render tree.

use std::rc::Rc;
use std::time::Duration;

use inset_embedder::{
    BoxHeightStyle, BoxWidthStyle, Offset, PointerChange, PointerData, PointerDataPacket,
    PointerDeviceKind, Rect, TargetPlatform, TextSelection, ViewId, ViewRef,
};
use inset_foundation::{App, AppCell};
use inset_gestures::GestureBinding;
use inset_painting::{PaintingBinding, TextSpan};
use inset_rendering::{AnyRenderObject, RenderBox, RenderHandle, RenderParagraph};
use inset_scheduler::SchedulerBinding;
use inset_test::{TestPlatform, TestView};
use inset_widgets::{GlobalKey, IntoWidget, View, WidgetRef, run_widget};

/// The 800x600 view at 1x every test app draws into.
fn test_view() -> ViewRef {
    Rc::new(TestView::with_pixel_ratio(800.0, 600.0, 1.0))
}

/// An app on a platform with one view and the OS fonts.
pub(crate) fn test_cell() -> Rc<AppCell> {
    let platform = TestPlatform::new()
        .on(TargetPlatform::MacOS)
        .with_view(test_view());
    let cell = AppCell::with_platform(Rc::new(platform));
    {
        let mut app = cell.borrow_mut();
        let binding = PaintingBinding::instance(&mut app);
        binding.install_fonts(&mut app, |fonts| {
            fonts.add_source(valo_system_fonts::SystemFonts::load());
        });
    }
    cell
}

/// Mounts `child` under the platform's view and runs the first frame.
pub(crate) fn build(cell: &AppCell, child: WidgetRef) {
    {
        let mut app = cell.borrow_mut();
        let view = app
            .platform()
            .views()
            .into_iter()
            .next()
            .expect("an app from test_support::test_cell");
        run_widget(&mut app, View::new(view, child).into_widget());
    }
    // `run_app` attaches the root widget on the next timer turn, as Dart's `Timer.run` does.
    cell.elapse(Duration::ZERO);
    pump(&mut cell.borrow_mut(), Duration::ZERO);
}

/// Runs one frame at `at`, of every view the platform has.
pub(crate) fn pump(app: &mut App, at: Duration) {
    let views: Vec<ViewId> = app
        .platform()
        .views()
        .iter()
        .map(|view| view.id())
        .collect();
    SchedulerBinding::handle_begin_frame(app, &views, Some(at));
    app.drain_microtasks();
    SchedulerBinding::handle_draw_frame(app, &views);
    app.drain_microtasks();
}

fn send(app: &mut App, change: PointerChange, position: Offset, at: Duration) {
    GestureBinding::instance(app).handle_pointer_data_packet(
        app,
        PointerDataPacket::new(vec![PointerData {
            change,
            kind: PointerDeviceKind::Touch,
            time_stamp: at,
            pointer_identifier: 1,
            physical_x: position.dx(),
            physical_y: position.dy(),
            ..PointerData::default()
        }]),
    );
    app.drain_microtasks();
}

/// A touch down and up at `position`, in logical pixels.
pub(crate) fn tap_at(app: &mut App, position: Offset) {
    send(app, PointerChange::Down, position, Duration::ZERO);
    send(app, PointerChange::Up, position, Duration::from_millis(16));
}

/// The render object of the widget `key` is on.
pub(crate) fn render_object_of(app: &mut App, key: &GlobalKey) -> AnyRenderObject {
    key.current_context(app)
        .and_then(|context| context.find_render_object(app))
        .expect("the keyed widget is mounted")
}

/// `root` and every render object below it, depth first.
pub(crate) fn descendants(app: &App, root: AnyRenderObject) -> Vec<AnyRenderObject> {
    let mut found = vec![root];
    let mut children = Vec::new();
    root.visit_children(app, &mut |child| children.push(child));
    for child in children {
        found.extend(descendants(app, child));
    }
    found
}

/// The render objects of type `T` below `root`, depth first.
pub(crate) fn find_all<T: inset_rendering::RenderObject>(
    app: &App,
    root: AnyRenderObject,
) -> Vec<RenderHandle<T>> {
    descendants(app, root)
        .into_iter()
        .filter_map(|object| object.downcast::<T>(app))
        .collect()
}

/// The plain text of each paragraph below `root`.
pub(crate) fn paragraph_texts(app: &App, root: AnyRenderObject) -> Vec<String> {
    find_all::<RenderParagraph>(app, root)
        .into_iter()
        .map(|paragraph| paragraph.text(app).to_plain_text(true, true))
        .collect()
}

/// The paragraph below `root` whose text contains `text`, and the first text span in it
/// whose own text contains `text`.
pub(crate) fn span_with_text(
    app: &App,
    root: AnyRenderObject,
    text: &str,
) -> (RenderHandle<RenderParagraph>, TextSpan) {
    for paragraph in find_all::<RenderParagraph>(app, root) {
        let mut found = None;
        paragraph.text(app).visit_children(&mut |span| {
            if let Some(span) = span.as_any().downcast_ref::<TextSpan>()
                && span.text.as_deref().is_some_and(|own| own.contains(text))
            {
                found = Some(span.clone());
                return false;
            }
            true
        });
        if let Some(span) = found {
            return (paragraph, span);
        }
    }
    panic!("no span has the text {text:?}");
}

/// Where `text` is drawn, in global coordinates: the box of its first line.
pub(crate) fn global_rect_of_text(app: &mut App, root: AnyRenderObject, text: &str) -> Rect {
    let (paragraph, _) = span_with_text(app, root, text);
    let plain = paragraph.text(app).to_plain_text(true, true);
    let start = plain.find(text).expect("the paragraph has the text");
    let start = plain[..start].encode_utf16().count() as i32;
    let end = start + text.encode_utf16().count() as i32;
    let text_box = paragraph.get_boxes_for_selection(
        app,
        TextSelection::new(start, end),
        BoxHeightStyle::Tight,
        BoxWidthStyle::Tight,
    )[0];
    let top_left =
        paragraph
            .as_box()
            .local_to_global(app, Offset::new(text_box.left, text_box.top), None);
    Rect::from_ltwh(
        top_left.dx(),
        top_left.dy(),
        text_box.right - text_box.left,
        text_box.bottom - text_box.top,
    )
}
