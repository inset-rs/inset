//! The app drawn headless, as the framework draws it on any host: its scene carries the three
//! programs, a paint, a backdrop filter and a layer's filter, and its pixels are theirs.

use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use shaders_lib::{BADGE, CELL, DARK, LIGHT, PANEL, WINDOW_SIZE, shaders};
use waft_embedder::test_support::{drawn_pixels, picture_to_image};
use waft_embedder::{
    ContainerLayer, Effect, ImageFilter, Layer, Scene, flattened_ops, slices, valo,
};
use waft_foundation::AppCell;
use waft_scheduler::SchedulerBinding;
use waft_test::{IMPLICIT_VIEW_ID, TestPlatform, TestView};
use waft_widgets::run_app;

/// The app's first frame, on a platform that draws pictures into images on the test device.
fn first_scene() -> Arc<Scene> {
    let view = Rc::new(TestView::new(WINDOW_SIZE[0], WINDOW_SIZE[1]));
    let platform = TestPlatform::new()
        .with_view(view.clone())
        .with_picture_to_image(|picture, width, height| {
            Some(picture_to_image(picture, [width, height]))
        });
    let cell = AppCell::with_platform(Rc::new(platform));
    run_app(&mut cell.borrow_mut(), shaders());
    // `run_app` attaches the root widget on the next timer turn.
    cell.elapse(Duration::ZERO);
    let views = [IMPLICIT_VIEW_ID];
    SchedulerBinding::handle_begin_frame(&mut cell.borrow_mut(), &views, Some(Duration::ZERO));
    cell.checkpoint();
    SchedulerBinding::handle_draw_frame(&mut cell.borrow_mut(), &views);
    cell.checkpoint();
    view.last_scene().expect("the first frame is presented")
}

/// Every container's effect in the tree, the root's first.
fn effects(container: &ContainerLayer) -> Vec<Effect> {
    let mut effects = vec![container.effect.clone()];
    for child in &container.children {
        if let Layer::Container(child) = child {
            effects.extend(self::effects(child));
        }
    }
    effects
}

fn pixel_at(pixels: &[u8], x: usize, y: usize) -> [u8; 4] {
    let at = (y * WINDOW_SIZE[0] as usize + x) * 4;
    pixels[at..at + 4].try_into().expect("four bytes a pixel")
}

/// Within a step or two of rounding in each channel.
fn assert_near(actual: [u8; 4], expected: [u8; 4], place: &str) {
    let near = actual
        .iter()
        .zip(expected)
        .all(|(actual, expected)| actual.abs_diff(expected) <= 2);
    assert!(near, "{place}: {actual:?}, expected {expected:?}");
}

fn bytes_of(color: waft_embedder::Color) -> [u8; 4] {
    [color.r, color.g, color.b, color.a].map(|channel| (channel * 255.0).round() as u8)
}

#[test]
fn the_scene_carries_a_shader_paint_a_backdrop_filter_and_a_layer_filter() {
    let scene = first_scene();
    let effects = effects(&scene.root);
    assert!(
        effects.iter().any(|effect| matches!(
            effect,
            Effect::BackdropFilter {
                filter: ImageFilter::Shader { .. },
                ..
            }
        )),
        "the panel's backdrop: {effects:?}"
    );
    assert!(
        effects.iter().any(|effect| matches!(
            effect,
            Effect::ImageFilter {
                filter: ImageFilter::Shader { .. },
                ..
            }
        )),
        "the badge's layer: {effects:?}"
    );
    let painted = flattened_ops(&slices(&scene, |_| None).picture_with_room())
        .into_iter()
        .any(|op| {
            matches!(
                op,
                valo::Op::DrawRect { paint, .. }
                    if matches!(paint.shader, Some(valo::Shader::Runtime(_)))
            )
        });
    assert!(painted, "the background is painted with the rings program");
}

#[test]
fn the_programs_draw_their_pixels() {
    let scene = first_scene();
    let size = WINDOW_SIZE.map(|side| side as u32);
    let pixels = drawn_pixels(slices(&scene, |_| None).picture_with_room(), size);

    assert_eq!(pixel_at(&pixels, 4, 4)[3], 255, "the background is painted");

    // The backdrop's input is the whole target, so the cells lie on the target's grid.
    let cell = CELL as usize;
    let first_cell = (PANEL.left as usize).div_ceil(cell) * cell + cell;
    let corner = pixel_at(&pixels, first_cell + 1, first_cell + 1);
    let across = pixel_at(&pixels, first_cell + cell - 2, first_cell + cell - 2);
    assert_eq!(corner, across, "one cell of the panel is one colour");

    let middle_y = ((BADGE.top + BADGE.bottom) / 2.0) as usize;
    assert_near(
        pixel_at(&pixels, BADGE.left as usize + 4, middle_y),
        bytes_of(DARK),
        "the badge's black stripe",
    );
    assert_near(
        pixel_at(&pixels, BADGE.right as usize - 4, middle_y),
        bytes_of(LIGHT),
        "the badge's white stripe",
    );
}
