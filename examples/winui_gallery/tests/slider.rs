//! `Slider`: a press on the track moves the value to the pointer, a drag of the thumb moves it by the distance dragged, a stepped slider snaps, and both themes render.
#![feature(arbitrary_self_types)]
mod common;
use common::Fixture;
use common::GalleryFixtureExt;
use inset_embedder::{Offset, PointerChange, Rect, Size};
use inset_rendering::{MainAxisSize, RenderParagraph};
use inset_widgets::*;
use inset_winui::*;
use std::time::Duration;

/// The bounds of the paragraph showing `text`. A slider's header fills the root grid's width, so its box is as wide as the slider and its bottom edge is where the `SliderTopHeaderMargin` starts.
fn label_bounds(fixture: &Fixture, text: &str) -> Rect {
    let elements = fixture.elements();
    let app = fixture.cell.borrow();
    let mut result = None;
    for element in elements {
        if let Some(object) = element.render_object(&app)
            && let Some(paragraph) = object.downcast::<RenderParagraph>(&app)
            && paragraph.text(&app).to_plain_text(true, true) == text
        {
            let object = object.as_box().unwrap();
            let size = object.size(&app);
            let origin = object.local_to_global(&app, Offset::ZERO, None);
            result = Some(Rect::from_ltwh(
                origin.dx(),
                origin.dy(),
                size.width(),
                size.height(),
            ));
        }
    }
    result.unwrap_or_else(|| panic!("missing label {text}"))
}

/// The point on the track of the horizontal slider under `header` at `fraction` of its clickable length: `MoveThumbToPoint` maps the track less the thumb, from half a thumb in, onto the range.
fn track_point(r: &ThemeResources, header: Rect, fraction: f64) -> Offset {
    let thumb = r.slider_horizontal_thumb_width;
    let x = header.left + thumb / 2.0 + fraction * (header.width() - thumb);
    let y = header.bottom
        + r.slider_top_header_margin[3]
        + r.slider_pre_content_margin
        + r.slider_track_theme_height / 2.0;
    Offset::new(x, y)
}

fn press(fixture: &mut Fixture, point: Offset) {
    fixture.send(PointerChange::Down, point);
    fixture.at += Duration::from_millis(20);
    fixture.send(PointerChange::Up, point);
    fixture.pump();
}

fn drag(fixture: &mut Fixture, from: Offset, to: Offset) {
    fixture.send(PointerChange::Down, from);
    fixture.at += Duration::from_millis(20);
    fixture.send(PointerChange::Move, to);
    fixture.at += Duration::from_millis(20);
    fixture.send(PointerChange::Up, to);
    fixture.pump();
}

#[test]
fn slider_presses_drags_snaps_and_switches_theme() {
    let r = ThemeResources::new(Theme::Light, AccentPalette::default());
    let mut fixture = Fixture::for_feature([900, 2600], winui_gallery::Feature::Slider);
    fixture.find("Slider: 42 · stepped 50 · vertical 30");

    // A press three quarters along the track: `MoveThumbToPoint` gives 0 + 0.75 × 100.
    let volume = label_bounds(&fixture, "Volume");
    press(&mut fixture, track_point(&r, volume, 0.75));
    fixture.find("Slider: 75 · stepped 50 · vertical 30");

    // A drag of the thumb by a tenth of the clickable track: `OnThumbDragDelta` adds 10.
    drag(
        &mut fixture,
        track_point(&r, volume, 0.75),
        track_point(&r, volume, 0.85),
    );
    fixture.find("Slider: 85 · stepped 50 · vertical 30");

    // A press at 33 % of a slider with `StepFrequency` 10 snaps to 30 (`GetClosestStep`).
    let stepped = label_bounds(&fixture, "Stepped");
    press(&mut fixture, track_point(&r, stepped, 0.33));
    fixture.find("Slider: 85 · stepped 30 · vertical 30");

    // The disabled slider ignores the pointer.
    let disabled = label_bounds(&fixture, "Disabled");
    press(&mut fixture, track_point(&r, disabled, 0.1));
    fixture.find("Slider: 85 · stepped 30 · vertical 30");

    fixture.capture("slider_light");
    fixture.tap("Dark theme");
    fixture.find("Light theme");
    fixture.capture("slider_dark");
}

/// A mouse drag updates the value and tooltip while leaving keyboard focus unchanged.
#[test]
fn mouse_drag_updates_the_thumb_without_acquiring_focus() {
    let r = ThemeResources::new(Theme::Light, AccentPalette::default());
    let mut fixture = Fixture::for_feature([900, 2600], winui_gallery::Feature::Slider);
    let focus = inset_widgets::primary_focus(&mut fixture.cell.borrow_mut());
    fixture.find("Slider: 42 · stepped 50 · vertical 30");
    let volume = label_bounds(&fixture, "Volume");
    let thumb = track_point(&r, volume, 0.42);
    fixture.send_mouse(PointerChange::Add, thumb - Offset::new(40.0, 40.0), 0);
    fixture.send_mouse(PointerChange::Hover, thumb, 0);
    fixture.pump();
    fixture.send_mouse(PointerChange::Down, thumb, 1);
    fixture.pump();
    fixture.find("42");
    let tip = label_bounds(&fixture, "42");
    assert!(
        tip.bottom < thumb.dy() - r.slider_horizontal_thumb_height / 2.0,
        "tip={tip:?} thumb={thumb:?}"
    );
    // Ten moves to 20 % further along the clickable track: `OnThumbDragDelta` adds 20.
    let end = track_point(&r, volume, 0.62);
    for step in 1..=10 {
        let t = step as f64 / 10.0;
        let point = Offset::new(thumb.dx() + (end.dx() - thumb.dx()) * t, thumb.dy());
        fixture.at += Duration::from_millis(16);
        fixture.send_mouse(PointerChange::Move, point, 1);
        fixture.pump();
    }
    fixture.find("62");
    fixture.capture("slider-value-tooltip-drag");
    fixture.send_mouse(PointerChange::Up, end, 0);
    fixture.pump();
    fixture.find("Slider: 62 · stepped 50 · vertical 30");
    assert_eq!(
        inset_widgets::primary_focus(&mut fixture.cell.borrow_mut()),
        focus,
    );
}

/// Like Flutter's `Slider`, a slider fills the width its parent bounds, with or without tick marks.
#[test]
fn a_slider_fills_the_width_it_is_given() {
    let r = ThemeResources::new(Theme::Light, AccentPalette::default());
    for ticks in [
        TickPlacement::None,
        TickPlacement::Outside,
        TickPlacement::Inline,
    ] {
        let size = common::size_under_center([300, 300], move |_| {
            Slider::new(10.0, |_, _| {})
                .tick_frequency(10.0)
                .tick_placement(ticks)
                .into_widget()
        });
        assert_eq!(
            size,
            Size::new(300.0, r.slider_horizontal_height),
            "{ticks:?}"
        );
    }
}

/// A vertical slider fills the height its parent bounds, with or without tick marks.
#[test]
fn a_vertical_slider_fills_the_height_it_is_given() {
    let r = ThemeResources::new(Theme::Light, AccentPalette::default());
    for ticks in [
        TickPlacement::None,
        TickPlacement::Outside,
        TickPlacement::Inline,
    ] {
        let size = common::size_under_center([300, 300], move |_| {
            Slider::new(10.0, |_, _| {})
                .orientation(inset_winui::Orientation::Vertical)
                .tick_frequency(10.0)
                .tick_placement(ticks)
                .into_widget()
        });
        assert_eq!(size, Size::new(r.slider_vertical_width, 300.0), "{ticks:?}");
    }
}

/// Where its width is unbounded, as in a `Row`, a slider with tick marks is as wide as its thumb,
/// as XAML measures it.
#[test]
fn a_slider_in_a_row_is_as_wide_as_its_thumb() {
    let r = ThemeResources::new(Theme::Light, AccentPalette::default());
    let size = common::size_under_center([300, 300], |_| {
        Row::new()
            .main_axis_size(MainAxisSize::Min)
            .children([Slider::new(10.0, |_, _| {})
                .tick_frequency(10.0)
                .tick_placement(TickPlacement::Outside)
                .into_widget()])
            .into_widget()
    });
    assert_eq!(size.width(), r.slider_horizontal_thumb_width);
}
