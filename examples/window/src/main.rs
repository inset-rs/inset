//! Opens the implicit window and renders a small render tree in it: a padded, centered,
//! decorated box, through `RendererBinding`'s frame pipeline. The card is a mouse region
//! that shows the pointing-hand cursor while hovered.
//!
//! ```text
//! cargo run -p window
//! ```
//!
//! Calling inherent `self: RenderHandle<Self>` methods (`set_child` on the view) needs the
//! feature in the calling crate too; trait methods do not.
#![feature(arbitrary_self_types)]

use reveal_embedder::{Color, Size};
use reveal_embedder_winit::{ImplicitViewConfig, WinitEmbedder};
use reveal_painting::{AlignmentGeometry, BoxDecoration, EdgeInsetsGeometry, ImageConfiguration};
use reveal_rendering::{
    BoxConstraints, DecorationPosition, RenderBox, RenderConstrainedBox, RenderDecoratedBox,
    RenderMouseRegion, RenderPadding, RenderPositionedBox, RendererBinding,
};
use reveal_scheduler::SchedulerBinding;
use reveal_services::SystemMouseCursors;
use reveal_shell::Shell;

fn main() {
    WinitEmbedder {
        implicit_view: Some(ImplicitViewConfig {
            title: "reveal — render tree".to_owned(),
            logical_size: [600.0, 400.0],
        }),
    }
    .run(|platform| {
        Shell::new(platform, |app| {
            let render_view = RendererBinding::instance(app).init_render_view(app);

            let card = RenderDecoratedBox::new(
                app,
                Box::new(BoxDecoration::new().color(Color::from_argb(255, 51, 51, 64))),
                DecorationPosition::Background,
                ImageConfiguration::EMPTY,
                None,
            );
            let sized = RenderConstrainedBox::new(
                app,
                BoxConstraints::tight(Size::new(200.0, 120.0)),
                Some(card.as_box()),
            );
            let hoverable = RenderMouseRegion::new(app, true, Some(sized.as_box()));
            hoverable.set_cursor(app, SystemMouseCursors::CLICK.into());
            let centered = RenderPositionedBox::new(
                app,
                AlignmentGeometry::CENTER,
                None,
                None,
                None,
                Some(hoverable.as_box()),
            );
            let padded = RenderPadding::new(
                app,
                EdgeInsetsGeometry::all(24.0),
                None,
                Some(centered.as_box()),
            );
            let background = RenderDecoratedBox::new(
                app,
                Box::new(BoxDecoration::new().color(Color::from_argb(255, 245, 245, 240))),
                DecorationPosition::Background,
                ImageConfiguration::EMPTY,
                Some(padded.as_box()),
            );
            render_view.set_child(app, Some(background.as_box()));
            SchedulerBinding::schedule_frame(app);
        })
    });
}
