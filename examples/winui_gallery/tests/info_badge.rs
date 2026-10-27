//! InfoBadge's display kinds, its square minimum and the stadium its height rounds it into.
#![feature(arbitrary_self_types)]
mod common;
use common::GalleryFixtureExt;

use common::{Fixture, mount};
use inset_embedder::{Color, Size, TextDirection};
use inset_foundation::App;
use inset_painting::AlignmentGeometry;
use inset_widgets::*;
use inset_winui::*;
use std::{cell::RefCell, rc::Rc};

/// Mounts one badge at the view's top-left corner and reports the size it took.
fn badge_at_top_left(size: [u32; 2], badge: InfoBadge) -> (Fixture, Size) {
    let reported = Rc::new(RefCell::new(Size::ZERO));
    let observed = reported.clone();
    let fixture = Fixture::with_root(
        size,
        mount(move |app| {
            install_icon_font(app);
            Directionality::new(
                TextDirection::Ltr,
                ThemeScope::new(
                    Theme::Light,
                    Align::new()
                        .alignment(AlignmentGeometry::TOP_LEFT)
                        .child(SizeObserver::new(
                            badge,
                            Rc::new(move |_: &mut App, _, new| *observed.borrow_mut() = new),
                        )),
                ),
            )
            .into_widget()
        }),
    );
    let size = *reported.borrow();
    (fixture, size)
}

#[test]
fn a_badge_without_a_value_or_icon_is_the_smallest_dot() {
    // DefaultInfoBadgeStyle sets MinWidth and MinHeight to InfoBadgeMinHeight, 4.
    let (_fixture, size) = badge_at_top_left([60, 60], InfoBadge::new());
    assert_eq!(size, Size::new(4.0, 4.0));
    assert_eq!(InfoBadge::new().display_kind(), InfoBadgeDisplayKind::Dot);
}

#[test]
fn a_value_wins_over_an_icon_and_a_glyph_icon_takes_the_font_icon_state() {
    // InfoBadge::OnDisplayKindPropertiesChanged checks Value first, then the icon's kind.
    let with_both = InfoBadge::new()
        .value(3)
        .icon_source(FontIcon::symbol(FluentSymbol::Checkmark));
    assert_eq!(with_both.display_kind(), InfoBadgeDisplayKind::Value);
    let glyph = InfoBadge::new().icon_source(FontIcon::symbol(FluentSymbol::Checkmark));
    assert_eq!(glyph.display_kind(), InfoBadgeDisplayKind::FontIcon);
    let drawn =
        InfoBadge::new().icon_source(PathIcon::new(inset_embedder::PathBuilder::new().build()));
    assert_eq!(drawn.display_kind(), InfoBadgeDisplayKind::Icon);
}

#[test]
fn a_badge_narrower_than_it_is_tall_is_squared_up() {
    // InfoBadge::MeasureOverride returns {height, height} when the template measures narrower
    // than it is tall. A 1x6 geometry inside IconInfoBadgeIconMargin measures 9 by 14.
    let mut geometry = inset_embedder::PathBuilder::new();
    geometry.rect(inset_embedder::valo::Rect::new(0.0, 0.0, 1.0, 6.0));
    let (_, size) = badge_at_top_left(
        [120, 60],
        InfoBadge::new().icon_source(PathIcon::new(geometry.build())),
    );
    assert_eq!(size, Size::new(14.0, 14.0));
}

#[test]
fn a_longer_count_widens_the_badge_without_making_it_taller() {
    let (_, one) = badge_at_top_left([120, 60], InfoBadge::new().value(1));
    let (_, many) = badge_at_top_left([120, 60], InfoBadge::new().value(999));
    assert_eq!(many.height(), one.height(), "the height does not change");
    assert!(
        many.width() > one.width(),
        "three digits are wider: {} against {}",
        many.width(),
        one.width()
    );
}

#[test]
fn a_badge_is_never_taller_than_the_style_allows() {
    // DefaultInfoBadgeStyle caps MaxHeight at InfoBadgeMaxHeight, 16.
    let (_, size) = badge_at_top_left(
        [120, 60],
        InfoBadge::new().icon_source(FontIcon::symbol(FluentSymbol::Checkmark).font_size(40.0)),
    );
    assert_eq!(size.height(), 16.0);
}

#[test]
fn the_fill_rounds_to_a_stadium_unless_a_corner_radius_is_set() {
    let rounded = corners([60, 60], InfoBadge::new().value(7));
    assert!(
        rounded.iter().all(|filled| !filled),
        "half the height rounds every corner away: {rounded:?}"
    );
    let square = corners([60, 60], InfoBadge::new().value(7).corner_radius([0.0; 4]));
    assert!(
        square.iter().all(|filled| *filled),
        "an explicit radius of zero keeps the corners: {square:?}"
    );
}

#[test]
fn each_named_style_fills_with_its_own_severity_colour() {
    let resources = ThemeResources::new(Theme::Light, AccentPalette::default());
    let colours: Vec<Color> = [
        InfoBadgeStyle::Default,
        InfoBadgeStyle::Attention,
        InfoBadgeStyle::Informational,
        InfoBadgeStyle::Success,
        InfoBadgeStyle::Caution,
        InfoBadgeStyle::Critical,
    ]
    .into_iter()
    .map(|style| style.background(&resources))
    .collect();
    assert_eq!(
        colours[3], resources.system_fill_color_success_brush,
        "Success uses SystemFillColorSuccessBrush"
    );
    for (index, colour) in colours.iter().enumerate() {
        for other in &colours[index + 1..] {
            assert_ne!(colour, other, "severities are distinguishable");
        }
    }
}

/// Whether each of the badge's four corner pixels is filled rather than the white background.
fn corners(view: [u32; 2], badge: InfoBadge) -> [bool; 4] {
    let (fixture, size) = badge_at_top_left(view, badge);
    let pixels = fixture.view.pixels.borrow();
    let filled = |x: f64, y: f64| {
        let index = ((y as usize) * view[0] as usize + x as usize) * 4;
        pixels[index..index + 3] != [255, 255, 255]
    };
    let (right, bottom) = (size.width() - 1.0, size.height() - 1.0);
    [
        filled(0.0, 0.0),
        filled(right, 0.0),
        filled(right, bottom),
        filled(0.0, bottom),
    ]
}

#[test]
fn the_gallery_page_shows_every_display_kind_and_severity() {
    let mut fixture = Fixture::for_feature([1100, 900], winui_gallery::Feature::InfoBadge);
    fixture.find("Display kinds");
    fixture.find("Severity");
    fixture.tap("Style: Default");
    fixture.find("Style: Attention");
    fixture.tap("Value: 5");
    fixture.find("Value: 42");
    fixture.capture("info_badge_light");
    fixture.tap("Dark theme");
    fixture.find("Light theme");
    fixture.capture("info_badge_dark");
}

#[test]
fn icon_foreground_inherits_the_badge_unless_explicitly_overridden() {
    for explicit in [None, Some(Color::from_argb(255, 0, 255, 0))] {
        let mut path = inset_embedder::PathBuilder::new();
        path.rect(inset_embedder::valo::Rect::new(0.0, 0.0, 8.0, 8.0));
        let mut icon = PathIcon::new(path.build());
        icon.foreground = explicit;
        let (f, _) = badge_at_top_left(
            [32, 32],
            InfoBadge::new()
                .foreground(Color::from_argb(255, 255, 0, 0))
                .icon_source(icon),
        );
        let pixels = f.view.pixels.borrow();
        let index = (8 * 32 + 8) * 4;
        assert_eq!(
            &pixels[index..index + 3],
            if explicit.is_some() {
                &[0, 255, 0]
            } else {
                &[255, 0, 0]
            }
        );
    }
}

#[test]
fn a_layout_builder_icon_is_measured_by_actual_layout() {
    let (_, size) = badge_at_top_left(
        [32, 32],
        InfoBadge::new().icon_source(
            LayoutBuilder::new(|_, _, _| SizedBox::new().width(1.0).height(6.0).into_widget())
                .into_widget(),
        ),
    );
    assert_eq!(size, Size::new(14.0, 14.0));
}
