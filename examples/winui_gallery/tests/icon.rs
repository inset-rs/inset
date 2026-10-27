//! The `IconElement` family: `FontIcon`'s glyph and `PathIcon`'s geometry, their layout and
//! the foreground each takes from its surroundings.
#![feature(arbitrary_self_types)]

use inset_embedder::valo::{Point, Rect as ValoRect};
use inset_embedder::{Color, FillRule, Path, PathBuilder, Size, TextDirection};
use inset_foundation::App;
use inset_painting::{AlignmentGeometry, TextStyle};
use inset_widgets::*;
use inset_winui::{FluentSymbol, FontIcon, PathIcon, SizeObserver, install_icon_font};
use inset_winui_test_support::{Fixture, mount};
use std::{cell::RefCell, rc::Rc, sync::Arc};

const GREEN: Color = Color::from_argb(255, 0, 160, 0);

/// A rectangle away from the origin, so the icon's size shows which edges it measures.
fn offset_square() -> Arc<Path> {
    let mut path = PathBuilder::new();
    path.rect(ValoRect::new(10.0, 20.0, 5.0, 7.0));
    path.build()
}

/// Two concentric squares, so the fill rule decides whether the centre is filled.
fn ring() -> Arc<Path> {
    let mut path = PathBuilder::new();
    path.rect(ValoRect::new(0.0, 0.0, 40.0, 40.0));
    path.rect(ValoRect::new(12.0, 12.0, 16.0, 16.0));
    path.build()
}

/// A triangle occupying the icon's whole box.
fn triangle() -> Arc<Path> {
    let mut path = PathBuilder::new();
    path.move_to(Point::new(0.0, 0.0));
    path.line_to(Point::new(28.0, 16.0));
    path.line_to(Point::new(0.0, 32.0));
    path.close();
    path.build()
}

/// Mounts one icon at the view's top-left corner and reports the size it took.
fn icon_at_top_left(size: [u32; 2], icon: WidgetRef) -> (Fixture, Size) {
    let reported = Rc::new(RefCell::new(Size::ZERO));
    let observed = reported.clone();
    let fixture = Fixture::with_root(
        size,
        mount(move |app| {
            install_icon_font(app);
            Directionality::new(
                TextDirection::Ltr,
                Align::new()
                    .alignment(AlignmentGeometry::TOP_LEFT)
                    .child(SizeObserver::new(
                        icon,
                        Rc::new(move |_: &mut App, _, new| *observed.borrow_mut() = new),
                    )),
            )
            .into_widget()
        }),
    );
    let size = *reported.borrow();
    (fixture, size)
}

#[test]
fn path_icon_asks_for_the_space_up_to_its_geometrys_far_edges() {
    // CShape::MeasureOverride with Stretch.None reports the geometry's right and bottom, so a
    // geometry away from the origin keeps the space before it rather than being pulled back.
    let (_fixture, size) = icon_at_top_left(
        [80, 80],
        PathIcon::new(offset_square())
            .foreground(GREEN)
            .into_widget(),
    );
    assert_eq!(size, Size::new(15.0, 27.0));
}

#[test]
fn path_icon_fills_by_the_source_default_even_odd_rule() {
    let (fixture, size) = icon_at_top_left(
        [60, 60],
        PathIcon::new(ring()).foreground(GREEN).into_widget(),
    );
    assert_eq!(size, Size::new(40.0, 40.0));
    {
        let pixels = fixture.view.pixels.borrow();
        let pixel = |x: usize, y: usize| &pixels[(y * 60 + x) * 4..][..4];
        assert_eq!(pixel(5, 20), &[0, 160, 0, 255], "outer square is filled");
        assert_eq!(
            pixel(20, 20),
            &[255, 255, 255, 255],
            "even-odd leaves the centre open"
        );
    }
    fixture.capture("path-icon-even-odd");
}

#[test]
fn path_icon_fills_the_centre_under_the_winding_rule() {
    let (fixture, _) = icon_at_top_left(
        [60, 60],
        PathIcon::new(ring())
            .fill_rule(FillRule::NonZero)
            .foreground(GREEN)
            .into_widget(),
    );
    let pixels = fixture.view.pixels.borrow();
    let pixel = |x: usize, y: usize| &pixels[(y * 60 + x) * 4..][..4];
    assert_eq!(pixel(5, 20), &[0, 160, 0, 255], "outer square is filled");
    assert_eq!(pixel(20, 20), &[0, 160, 0, 255], "so is the centre");
}

#[test]
fn path_icon_takes_the_enclosing_foreground_when_it_has_none() {
    // CPathIcon links the Path's Fill to IconElement.Foreground, which CIconElement pulls from
    // the enclosing text formatting when the icon does not set it.
    let (fixture, _) = icon_at_top_left(
        [60, 60],
        DefaultTextStyle::new(TextStyle::new().color(GREEN), PathIcon::new(triangle()))
            .into_widget(),
    );
    let pixels = fixture.view.pixels.borrow();
    let pixel = |x: usize, y: usize| &pixels[(y * 60 + x) * 4..][..4];
    assert_eq!(pixel(3, 16), &[0, 160, 0, 255], "inside the triangle");
    assert_eq!(pixel(26, 3), &[255, 255, 255, 255], "outside it");
}

#[test]
fn font_icon_draws_the_glyph_it_is_given_at_the_source_default_size() {
    // FontIcon takes any character of its FontFamily; FluentSymbol only names some of them.
    let named = rendered(FontIcon::symbol(FluentSymbol::Settings));
    let by_codepoint = rendered(FontIcon::new(FluentSymbol::Settings.glyph().to_string()));
    let other = rendered(FontIcon::symbol(FluentSymbol::Search));
    assert_eq!(named, by_codepoint, "the same glyph either way");
    assert_ne!(named, other, "different glyphs draw differently");
    assert!(
        named.iter().any(|&covered| covered),
        "the glyph covers some of the icon"
    );
}

#[test]
fn font_icon_scales_with_its_font_size() {
    let default_size = glyph_extent(FontIcon::symbol(FluentSymbol::Settings));
    let doubled = glyph_extent(FontIcon::symbol(FluentSymbol::Settings).font_size(40.0));
    // The source default is g_ClientCoreFontSize, 20.
    assert!(
        doubled > default_size * 1.5,
        "40 pt covers more than 20 pt: {doubled} against {default_size}"
    );
}

/// Which pixels of a 60x60 view the icon's glyph covers.
fn rendered(icon: FontIcon) -> Vec<bool> {
    let (fixture, _) = icon_at_top_left([60, 60], icon.foreground(GREEN).into_widget());
    let pixels = fixture.view.pixels.borrow();
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .map(|pixel| pixel[..3] != [255, 255, 255])
        .collect()
}

/// How many pixels the icon's glyph covers.
fn glyph_extent(icon: FontIcon) -> f64 {
    rendered(icon)
        .into_iter()
        .filter(|&covered| covered)
        .count() as f64
}
