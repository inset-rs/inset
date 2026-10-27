//! Real scene sampling and foreground isolation for the source acrylic effect pass.
#![feature(arbitrary_self_types)]

use inset_embedder::{Canvas, Color, Paint, Rect, Size};
use inset_foundation::App;
use inset_rendering::CustomPainter;
use inset_widgets::*;
use inset_winui::*;
use inset_winui_test_support::Fixture;

#[derive(Clone, Debug)]
struct Pattern;

impl CustomPainter for Pattern {
    fn paint(&self, _app: &mut App, canvas: &mut Canvas, size: Size) {
        for x in 0..(size.width() as usize).div_ceil(8) {
            let color = if x < 24 {
                if x % 2 == 0 {
                    Color::new(0xFFE03020)
                } else {
                    Color::new(0xFF2050E0)
                }
            } else if x % 2 == 0 {
                Color::new(0xFF20C070)
            } else {
                Color::new(0xFFFFFF80)
            };
            canvas.draw_rect(
                Rect::from_ltwh(x as f64 * 8.0, 0.0, 8.0, size.height()),
                &Paint::from_color(color.into()),
            );
        }
    }

    fn should_repaint(&self, _app: &App, _old: &dyn CustomPainter) -> bool {
        false
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn fixture(theme: Theme, acrylic: bool) -> Fixture {
    fixture_with_fallback_alpha(theme, acrylic, false)
}

fn fixture_with_fallback_alpha(theme: Theme, acrylic: bool, transparent_fallback: bool) -> Fixture {
    Fixture::with_root([384, 256], move |app| {
        let mut recipe =
            ThemeResources::new(theme, AccentPalette::default()).flyout_presenter_background;
        if transparent_fallback {
            recipe.fallback_color = recipe.fallback_color.with_alpha(0);
        }
        let brush = if acrylic {
            Brush::Acrylic(recipe)
        } else {
            Brush::Solid(recipe.fallback_color)
        };
        run_app(
            app,
            Directionality::new(
                inset_embedder::TextDirection::Ltr,
                Stack::new().children(vec![
                    Positioned::fill(CustomPaint::new().painter(Pattern)).into_widget(),
                    Positioned::new(
                        SizedBox::new().width(288.0).height(176.0).child(
                            ControlBorder::new(brush, Brush::Solid(Color::new(0)))
                                .border_thickness(0.0)
                                .corner_radius(20.0)
                                .child(Stack::new().children(vec![
                                        Positioned::new(
                                            SizedBox::new()
                                                .width(60.0)
                                                .height(24.0)
                                                .child(ColoredBox::new(Color::new(0xFF000000))),
                                        )
                                        .left(32.0)
                                        .top(72.0)
                                        .into_widget(),
                                        Positioned::new(
                                            SizedBox::new()
                                                .width(60.0)
                                                .height(24.0)
                                                .child(ColoredBox::new(Color::new(0xFFFFFFFF))),
                                        )
                                        .left(92.0)
                                        .top(72.0)
                                        .into_widget(),
                                    ])),
                        ),
                    )
                    .left(48.0)
                    .top(40.0)
                    .into_widget(),
                ]),
            )
            .into_widget(),
        );
    })
}

fn pixel(f: &Fixture, x: usize, y: usize) -> [u8; 4] {
    let pixels = f.view.pixels.borrow();
    let index = (y * f.view.size[0] as usize + x) * 4;
    pixels[index..index + 4].try_into().unwrap()
}

fn distance(a: [u8; 4], b: [u8; 4]) -> u32 {
    a[..3]
        .iter()
        .zip(&b[..3])
        .map(|(a, b)| a.abs_diff(*b) as u32)
        .sum()
}

#[test]
fn acrylic_samples_and_blurs_the_scene_without_filtering_foreground() {
    for theme in [Theme::Light, Theme::Dark] {
        let f = fixture(theme, true);
        let fallback = fixture(theme, false);
        f.capture(if theme == Theme::Light {
            "acrylic-light"
        } else {
            "acrylic-dark"
        });
        // The untouched stripes retain their edge. Inside acrylic the same 8px pattern is blurred.
        assert!(distance(pixel(&f, 20, 20), pixel(&f, 28, 20)) > 200);
        assert!(distance(pixel(&f, 100, 80), pixel(&f, 108, 80)) < 12);
        assert!(
            distance(pixel(&f, 180, 80), pixel(&fallback, 180, 80)) > 8,
            "material must sample the colored scene, not paint fallback"
        );
        // Text/control content paints after the material pass and keeps a sharp black/white boundary.
        assert_eq!(pixel(&f, 139, 124), [0, 0, 0, 255]);
        assert_eq!(pixel(&f, 140, 124), [255, 255, 255, 255]);
        // Rounded corners leave the original background untouched outside the material.
        assert_eq!(pixel(&f, 49, 41), pixel(&f, 49, 20));
    }
}

#[test]
fn transparent_fallback_does_not_hide_the_material() {
    let opaque = fixture(Theme::Light, true);
    let transparent = fixture_with_fallback_alpha(Theme::Light, true, true);
    // The source seed forces fallback alpha opaque; its alpha must not suppress shape painting.
    assert_eq!(
        *opaque.view.pixels.borrow(),
        *transparent.view.pixels.borrow()
    );
}
