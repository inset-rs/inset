//! ThemeScope's own values: what they change, and where they reach.
#![feature(arbitrary_self_types)]
mod common;

use common::{Fixture, mount};
use inset_embedder::{Color, TextDirection};
use inset_painting::AlignmentGeometry;
use inset_widgets::*;
use inset_winui::*;
use std::{cell::RefCell, rc::Rc};

const RED: Color = Color::from_argb(255, 255, 0, 0);

/// What a `Probe` saw where it was built.
type Seen = Rc<RefCell<Option<Rc<ThemeResources>>>>;

/// `probe` is an empty widget that keeps the resources it is built with in `seen`.
fn probe(seen: &Seen) -> WidgetRef {
    let seen = seen.clone();
    Builder::new(move |app, context| {
        *seen.borrow_mut() = Some(ThemeResources::of(app, context));
        SizedBox::shrink().into_widget()
    })
    .into_widget()
}

/// `seen` is what `probe` last saw.
fn seen(seen: &Seen) -> Rc<ThemeResources> {
    seen.borrow().clone().expect("the probe was built")
}

#[test]
fn an_override_reaches_a_text_box() {
    let fixture = Fixture::with_root(
        [200, 60],
        mount(|app| {
            winui_gallery::install_fonts(app);
            let controller = TextEditingController::new(app);
            Directionality::new(
                TextDirection::Ltr,
                ThemeScope::new(
                    Theme::Light,
                    Align::new().alignment(AlignmentGeometry::TOP_LEFT).child(
                        SizedBox::new()
                            .width(160.0)
                            .child(TextBox::new(controller.as_controller())),
                    ),
                )
                .any(|r| r.text_control_background = RED),
            )
            .into_widget()
        }),
    );
    // Inside the box, left of where its text starts.
    let [x, y] = [4, 16];
    let index = ((y * fixture.view.size[0] + x) * 4) as usize;
    let pixels = fixture.view.pixels.borrow();
    assert_eq!(pixels[index..index + 3], [255, 0, 0]);
}

#[test]
fn a_nested_override_reaches_only_its_subtree() {
    let (outside, inside) = (Seen::default(), Seen::default());
    let (outer, inner) = (outside.clone(), inside.clone());
    let _fixture = Fixture::with_root(
        [100, 100],
        mount(move |_| {
            ThemeScope::new(
                Theme::Light,
                Column::new().children(vec![
                    probe(&outer),
                    ThemeScope::overriding(probe(&inner))
                        .any(|r| r.control_corner_radius = [9.0; 4])
                        .into_widget(),
                ]),
            )
            .any(|r| r.text_fill_color_primary = RED)
            .into_widget()
        }),
    );
    let winui = ThemeResources::new(Theme::Light, AccentPalette::default());
    let (outside, inside) = (seen(&outside), seen(&inside));
    assert_eq!(outside.text_fill_color_primary, RED);
    assert_eq!(outside.control_corner_radius, winui.control_corner_radius);
    assert_eq!(inside.text_fill_color_primary, RED);
    assert_eq!(inside.control_corner_radius, [9.0; 4]);
}

#[test]
fn the_shown_themes_values_apply_after_both_themes() {
    let (light, dark) = (Seen::default(), Seen::default());
    let (in_light, in_dark) = (light.clone(), dark.clone());
    let scope = |theme, seen: &Seen| {
        ThemeScope::new(theme, probe(seen))
            .any(|r| r.control_corner_radius = [1.0; 4])
            .light(|r| r.text_fill_color_primary = RED)
            .dark(|r| r.control_corner_radius = [2.0; 4])
            .into_widget()
    };
    let _fixture = Fixture::with_root(
        [100, 100],
        mount(move |_| {
            Column::new()
                .children(vec![
                    scope(Theme::Light, &in_light),
                    scope(Theme::Dark, &in_dark),
                ])
                .into_widget()
        }),
    );
    let winui_dark = ThemeResources::new(Theme::Dark, AccentPalette::default());
    let (light, dark) = (seen(&light), seen(&dark));
    assert_eq!(light.control_corner_radius, [1.0; 4]);
    assert_eq!(light.text_fill_color_primary, RED);
    assert_eq!(dark.control_corner_radius, [2.0; 4]);
    assert_eq!(
        dark.text_fill_color_primary,
        winui_dark.text_fill_color_primary
    );
}
