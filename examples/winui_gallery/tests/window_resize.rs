//! Actual platform metrics changes exercise OverlayPortal layout during adaptive resizing.
#![feature(arbitrary_self_types)]
use inset_winui_test_support::gpu;

use inset_embedder::valo::{Color, Context};
use inset_embedder::{PointerChange, PointerData, PointerDataPacket, PointerDeviceKind};
use inset_embedder::{
    Scene, SystemFontSource, TargetPlatform, View, ViewConstraints, ViewId, ViewMetrics,
};
use inset_foundation::{AppCell, Listener};
use inset_gestures::GestureBinding;
use inset_rendering::RendererBinding;
use inset_scheduler::SchedulerBinding;
use inset_test::TestPlatform;
use inset_widgets::*;
use inset_winui::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

pub struct CaptureView {
    pub size: Cell<[u32; 2]>,
    pub renderer: RefCell<Context>,
    pub pixels: RefCell<Vec<u8>>,
}
impl View for CaptureView {
    fn id(&self) -> ViewId {
        ViewId(0)
    }
    fn metrics(&self) -> ViewMetrics {
        ViewMetrics {
            physical_size: self.size.get().map(|v| v as f64),
            physical_constraints: ViewConstraints::tight(
                self.size.get()[0] as f64,
                self.size.get()[1] as f64,
            ),
            device_pixel_ratio: 1.0,
            ..Default::default()
        }
    }
    fn present(&self, scene: Arc<Scene>) {
        *self.pixels.borrow_mut() = self.renderer.borrow_mut().render_to_rgba(
            &inset_embedder::slices(&scene).picture_with_room(),
            self.size.get(),
            Some(Color::WHITE),
        );
    }
}

/// One refresh of `view` at `time`, processing microtasks between its phases.
fn refresh(cell: &AppCell, view: &CaptureView, time: Duration) {
    let views = [view.id()];
    SchedulerBinding::handle_begin_frame(&mut cell.borrow_mut(), &views, Some(time));
    cell.checkpoint();
    SchedulerBinding::handle_draw_frame(&mut cell.borrow_mut(), &views);
    cell.checkpoint();
}
/// A macOS host showing `view`, with the system fonts.
fn host(view: &Rc<CaptureView>) -> TestPlatform {
    TestPlatform::new()
        .on(TargetPlatform::MacOS)
        .with_view(view.clone())
        .with_font_source(|| Box::new(SystemFontSource::platform()))
}

#[test]
fn real_window_resize_relayouts_navigation_overlay() {
    let (device, queue) = gpu::headless_device().expect("GPU required");
    let view = Rc::new(CaptureView {
        size: Cell::new([1200, 800]),
        renderer: RefCell::new(Context::new(device, queue)),
        pixels: RefCell::new(Vec::new()),
    });
    let cell = AppCell::with_platform(Rc::new(host(&view)));
    let tooltip_opened = Rc::new(Cell::new(false));
    {
        let mut app = cell.borrow_mut();
        winui_gallery::install_fonts(&mut app);
        let opened = tooltip_opened.clone();
        let entry = OverlayEntry::new(
            &mut app,
            Rc::new(move |_, _| {
                let opened = opened.clone();
                NavigationView::new(
                    vec![
                        NavigationViewItem::text("one", "One"),
                        NavigationViewItem::text("two", "Two"),
                    ],
                    Some("one".into()),
                    |_, _| {},
                    Center::new().child(ToolTipService::new(
                        SizedBox::new().width(100.0).height(60.0),
                        ToolTip::text("Resize tooltip")
                            .opened(Listener::new(move |_| opened.set(true))),
                    )),
                )
                .is_pane_open(true, |_, _| {})
                .into_widget()
            }),
            false,
            true,
            false,
        );
        run_app(
            &mut app,
            WidgetsApp::new(AccentPalette::default().base)
                .debug_show_checked_mode_banner(false)
                .builder(move |_, _, _| Overlay::new().initial_entries([entry]).into_widget())
                .into_widget(),
        );
    }
    cell.elapse(Duration::ZERO);
    let mut time = Duration::ZERO;
    refresh(&cell, &view, time);
    {
        let mut app = cell.borrow_mut();
        GestureBinding::instance(&mut app).handle_pointer_data_packet(
            &mut app,
            PointerDataPacket::new(vec![PointerData {
                change: PointerChange::Hover,
                kind: PointerDeviceKind::Mouse,
                physical_x: 760.0,
                physical_y: 400.0,
                ..Default::default()
            }]),
        );
    }
    cell.checkpoint();
    cell.elapse(Duration::from_millis(900));
    time += Duration::from_millis(900);
    refresh(&cell, &view, time);
    assert!(
        tooltip_opened.get(),
        "hover timer opens tooltip before resizing"
    );
    for size in (0..8).flat_map(|_| {
        [
            [1200, 800],
            [900, 650],
            [620, 500],
            [480, 360],
            [900, 650],
            [1200, 800],
            [400, 400],
        ]
    }) {
        view.size.set(size);
        {
            let mut app = cell.borrow_mut();
            RendererBinding::instance(&mut app).handle_metrics_changed(&mut app);
        }
        {
            time += Duration::from_millis(16);
            refresh(&cell, &view, time);
            assert_eq!(view.pixels.borrow().len(), (size[0] * size[1] * 4) as usize);
        }
    }
}

#[test]
fn real_gallery_window_resize_keeps_overlay_parent_data() {
    let (device, queue) = gpu::headless_device().expect("GPU required");
    let view = Rc::new(CaptureView {
        size: Cell::new([1200, 800]),
        renderer: RefCell::new(Context::new(device, queue)),
        pixels: RefCell::new(Vec::new()),
    });
    let cell = AppCell::with_platform(Rc::new(host(&view)));
    {
        let mut app = cell.borrow_mut();
        winui_gallery::install_fonts(&mut app);
        winui_gallery::run(&mut app);
    }
    cell.elapse(Duration::ZERO);
    let mut time = Duration::ZERO;
    for size in (0..8).flat_map(|_| {
        [
            [1200, 800],
            [900, 650],
            [620, 500],
            [480, 360],
            [900, 650],
            [1200, 800],
            [400, 400],
        ]
    }) {
        view.size.set(size);
        {
            let mut app = cell.borrow_mut();
            RendererBinding::instance(&mut app).handle_metrics_changed(&mut app);
        }
        {
            time += Duration::from_millis(16);
            refresh(&cell, &view, time);
            assert_eq!(view.pixels.borrow().len(), (size[0] * size[1] * 4) as usize);
        }
    }
}
