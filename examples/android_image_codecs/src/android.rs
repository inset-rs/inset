//! The screen: each check's title, whether it passed, what it found and the pictures it decoded,
//! drawn larger than their pixels, and each line written to the log.

use waft_embedder::{Color, Image, TextDirection};
use waft_foundation::{App, Entity};
use waft_painting::{BoxDecoration, BoxFit, EdgeInsetsGeometry, ImageInfo, TextStyle};
use waft_widgets::{
    BuildContext, Column, CrossAxisAlignment, DecoratedBox, Directionality, IntoWidget, Padding,
    RawImage, Row, SingleChildScrollView, StatelessWidget, Text, WidgetRef, run_app,
};

use crate::checks::{self, Report};

const BACKGROUND: Color = Color::from_argb(255, 245, 245, 240);
const PASSED: Color = Color::from_argb(255, 20, 120, 40);
const WRONG: Color = Color::from_argb(255, 190, 30, 30);
const LINE: Color = Color::from_argb(255, 30, 30, 30);
/// Logical pixels a picture's pixel is drawn at; a tiny animation's at eight times that.
const ZOOM: f64 = 2.0;

/// A check as the screen shows it, its pictures uploaded.
struct Shown {
    report: Report,
    pictures: Vec<(Image, [u32; 2])>,
}

/// Runs the checks, and shows them when all have finished.
pub(crate) fn run(app: &mut App) {
    let shown = app.new_entity(|_cx| Vec::<Shown>::new());
    let showing = shown.clone();
    let _checking = app.spawn(async move |cx| {
        let platform = cx.update(|app| app.platform());
        let Some(codecs) = platform.image_codecs() else {
            crate::log::info("no image codecs on this host");
            return;
        };
        let reports = checks::all(codecs).await;
        let passed = reports.iter().filter(|report| report.passed).count();
        crate::log::info(&format!("{passed} of {} checks passed", reports.len()));
        cx.update(|app| {
            for report in reports {
                log(&report);
                show(app, &showing, report);
            }
        });
    });
    let home = Directionality::new(
        TextDirection::Ltr,
        DecoratedBox::new(BoxDecoration::new().color(BACKGROUND)).child(
            SingleChildScrollView::new().child(
                Padding::new(EdgeInsetsGeometry::from_ltrb(12.0, 40.0, 12.0, 24.0))
                    .child(Checks { shown }),
            ),
        ),
    );
    run_app(app, home.into_widget());
}

fn log(report: &Report) {
    let verdict = if report.passed { "passed" } else { "FAILED" };
    crate::log::info(&format!("{}: {verdict}", report.title));
    for line in &report.lines {
        crate::log::info(&format!("  {line}"));
    }
}

fn show(app: &mut App, shown: &Entity<Vec<Shown>>, mut report: Report) {
    let platform = app.platform();
    let pictures = std::mem::take(&mut report.pictures)
        .into_iter()
        .filter_map(|pixels| {
            let size = pixels.layout().size;
            platform.import_pixels(pixels).map(|image| (image, size))
        })
        .collect();
    shown.update(app, |shown, cx| {
        shown.push(Shown { report, pictures });
        cx.notify();
    });
}

/// The checks finished so far.
#[derive(Debug)]
struct Checks {
    shown: Entity<Vec<Shown>>,
}

impl StatelessWidget for Checks {
    fn build(&self, app: &mut App, _context: BuildContext) -> WidgetRef {
        let checks = self.shown.read(app).iter().map(check).collect::<Vec<_>>();
        Column::new()
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .spacing(14.0)
            .children(checks)
            .into_widget()
    }
}

fn check(shown: &Shown) -> WidgetRef {
    let report = &shown.report;
    let (verdict, colour) = if report.passed {
        ("passed", PASSED)
    } else {
        ("FAILED", WRONG)
    };
    let mut rows = vec![
        text(&format!("{}: {verdict}", report.title), 15.0, colour),
        Row::new()
            .spacing(8.0)
            .children(shown.pictures.iter().map(picture))
            .into_widget(),
    ];
    rows.extend(report.lines.iter().map(|line| text(line, 11.0, LINE)));
    Column::new()
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .spacing(2.0)
        .children(rows)
        .into_widget()
}

/// A picture at `ZOOM` logical pixels a pixel, a tiny one larger.
fn picture((image, size): &(Image, [u32; 2])) -> WidgetRef {
    let zoom = if size[0] <= 8 { ZOOM * 8.0 } else { ZOOM };
    RawImage::new(Some(ImageInfo::new(image.clone())))
        .width(f64::from(size[0]) * zoom)
        .height(f64::from(size[1]) * zoom)
        .fit(BoxFit::Fill)
        .into_widget()
}

fn text(line: &str, size: f64, colour: Color) -> WidgetRef {
    Text::new(line.to_owned())
        .style(TextStyle::new().font_size(size).color(colour))
        .into_widget()
}
