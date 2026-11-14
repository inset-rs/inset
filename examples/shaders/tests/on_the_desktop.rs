//! The app on this Mac's desktop host: its window opens, the host draws its frames with the three
//! programs, the picture the background samples drawn into an image by the host, and the frame
//! drawn again on the host's own device shows the programs' pixels.
//!
//! Its own binary, without the test harness: AppKit's event loop runs on the main thread only. It
//! opens a window, above the others so that nothing covers it, for a moment, and needs a login
//! session's window server.
#![feature(arbitrary_self_types)]
// A picture's display list nests auto-trait obligations past the default depth.
#![recursion_limit = "256"]

#[cfg(target_os = "macos")]
fn main() {
    desktop::run();
}

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
mod desktop {
    use std::sync::Arc;
    use std::time::Duration;

    use shaders_lib::{BADGE, CELL, DARK, LIGHT, PANEL, WINDOW_SIZE, shaders};
    use waft_embedder::{
        ContainerLayer, Effect, ImageFilter, Layer, SceneBuilder, WindowConfig, WindowLevel,
        flattened_ops, slices, valo,
    };
    use waft_embedder_winit::{WinitEmbedder, WinitPlatform};
    use waft_foundation::App;
    use waft_rendering::RendererBinding;
    use waft_scheduler::{FrameCallback, FrameTarget, SchedulerBinding};
    use waft_shell::Shell;
    use waft_widgets::run_app_with;

    const TEST: &str = "on_the_desktop";

    /// The frames drawn before the last is looked at.
    const FRAMES: u32 = 5;

    /// Long enough for the window to open and the frames to be drawn, many times over.
    const GIVE_UP_AFTER: Duration = Duration::from_secs(20);

    pub(crate) fn run() {
        std::thread::spawn(|| {
            std::thread::sleep(GIVE_UP_AFTER);
            fail(&format!(
                "the app drew no {FRAMES} frames within {GIVE_UP_AFTER:?}"
            ));
        });
        WinitEmbedder::default().run(|platform| Shell::new(platform, setup));
    }

    fn setup(app: &mut App) {
        let window = WindowConfig {
            title: "Waft — shaders, under test".to_owned(),
            size: WINDOW_SIZE,
            level: WindowLevel::AlwaysOnTop,
            ..WindowConfig::default()
        };
        run_app_with(app, shaders(), window);
        check_after_frames(app, FRAMES);
    }

    /// Asks for another frame after each, and checks what the app drew after the last.
    fn check_after_frames(app: &mut App, frames: u32) {
        let callback = FrameCallback::new(move |app, _elapsed| {
            if frames > 1 {
                SchedulerBinding::schedule_forced_frame(app, FrameTarget::EveryView);
                check_after_frames(app, frames - 1);
                return;
            }
            let wrong = what_is_wrong(app);
            if !wrong.is_empty() {
                fail(&wrong.join("; "));
            }
            println!("{TEST}: passed");
            std::process::exit(0);
        });
        SchedulerBinding::add_post_frame_callback(app, callback, None);
    }

    /// What is wrong with the frame the app drew last.
    fn what_is_wrong(app: &mut App) -> Vec<String> {
        let renderer = RendererBinding::instance(app);
        let render_view = *renderer
            .render_views(app)
            .first()
            .expect("the app's window has a render view");
        let layer = render_view
            .as_object()
            .layer(app)
            .expect("the render view composited a frame");
        let scene = layer.build_scene(app, SceneBuilder::new());
        let mut wrong = Vec::new();

        let effects = effects(&scene.root);
        let filters_by_shader = |effect: &Effect| match effect {
            Effect::BackdropFilter { filter, .. } | Effect::ImageFilter { filter, .. } => {
                matches!(filter, ImageFilter::Shader { .. })
            }
            _ => false,
        };
        if effects
            .iter()
            .filter(|effect| filters_by_shader(effect))
            .count()
            != 2
        {
            wrong.push(format!("not the two shader filters: {effects:?}"));
        }
        let picture = slices(&scene, |_| None).picture_with_room();
        let painted = flattened_ops(&picture).into_iter().any(|op| {
            matches!(
                op,
                valo::Op::DrawRect { paint, .. }
                    if matches!(paint.shader, Some(valo::Shader::Runtime(_)))
            )
        });
        if !painted {
            wrong.push(
                "the background is not painted: the host drew the checkerboard into no image"
                    .to_owned(),
            );
        }
        wrong.extend(what_is_wrong_with_the_pixels(app, picture));
        wrong
    }

    /// What is wrong with `picture`, the app's frame, drawn on the device the host draws with.
    fn what_is_wrong_with_the_pixels(app: &App, picture: waft_embedder::Picture) -> Vec<String> {
        let platform = app.platform();
        let view = platform.views().into_iter().next().expect("the app's view");
        let ratio = view.metrics().device_pixel_ratio;
        let width = (WINDOW_SIZE[0] * ratio) as usize;
        let size = [width as u32, (WINDOW_SIZE[1] * ratio) as u32];
        let images = platform
            .downcast_ref::<WinitPlatform>()
            .expect("the winit host's platform")
            .image_context();
        let mut context = valo::Context::new(images.device().clone(), images.queue().clone());
        let request =
            context.request_pixels(&Arc::new(picture), size, Some(valo::Color::TRANSPARENT));
        images
            .device()
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the host's device answers");
        let pixels = pollster::block_on(request)
            .expect("the frame reads back")
            .into_bytes();
        let pixel_at = |x: f64, y: f64| -> [u8; 4] {
            let at = ((y * ratio) as usize * width + (x * ratio) as usize) * 4;
            pixels[at..at + 4].try_into().expect("four bytes a pixel")
        };
        let near = |actual: [u8; 4], expected: waft_embedder::Color| {
            let expected = [expected.r, expected.g, expected.b, expected.a]
                .map(|channel| (channel * 255.0).round() as u8);
            actual
                .iter()
                .zip(expected)
                .all(|(actual, expected)| actual.abs_diff(expected) <= 2)
        };

        let mut wrong = Vec::new();
        let middle_y = (BADGE.top + BADGE.bottom) / 2.0;
        let black_stripe = pixel_at(BADGE.left + 4.0, middle_y);
        if !near(black_stripe, DARK) {
            wrong.push(format!(
                "the badge's black stripe is {black_stripe:?}, not {DARK:?}"
            ));
        }
        let white_stripe = pixel_at(BADGE.right - 4.0, middle_y);
        if !near(white_stripe, LIGHT) {
            wrong.push(format!(
                "the badge's white stripe is {white_stripe:?}, not {LIGHT:?}"
            ));
        }
        // The backdrop's input is the whole target, so the cells lie on the target's grid, in
        // physical pixels.
        let cell = CELL / ratio;
        let first_cell = (PANEL.left / cell).ceil() * cell + cell;
        let corner = pixel_at(first_cell + 0.25, first_cell + 0.25);
        let across = pixel_at(first_cell + cell - 0.75, first_cell + cell - 0.75);
        if corner != across {
            wrong.push(format!(
                "one cell of the panel is {corner:?} and {across:?}"
            ));
        }
        wrong
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

    fn fail(reason: &str) -> ! {
        eprintln!("{TEST}: {reason}");
        std::process::exit(1);
    }
}
