//! Lays out `RenderPadding` around a tight `RenderConstrainedBox` and prints
//! sizes. Proves the handle pattern: the parent lays out the child, then reads
//! itself again.
//!
//! ```text
//! cargo run -p layout
//! ```
#![feature(arbitrary_self_types)]

use reveal_embedder::Size;
use reveal_foundation::App;
use reveal_painting::EdgeInsetsGeometry;
use reveal_rendering::{
    BoxConstraints, PipelineOwner, RenderBox, RenderConstrainedBox, RenderPadding,
};

fn main() {
    let mut app = App::new();
    let owner = PipelineOwner::new(&mut app, None);

    let child =
        RenderConstrainedBox::new(&mut app, BoxConstraints::tight(Size::new(80.0, 40.0)), None);
    let padding = RenderPadding::new(
        &mut app,
        EdgeInsetsGeometry::all(8.0),
        None,
        Some(child.as_box()),
    );

    owner.set_root_node(&mut app, Some(padding.as_object()));
    padding.layout(&mut app, BoxConstraints::new(), false);

    let padding_size = padding.size(&app);
    let child_size = child.size(&app);
    let offset = child.as_box().box_parent_data(&app).offset;

    println!(
        "padding size: {}x{}",
        padding_size.width(),
        padding_size.height()
    );
    println!(
        "child size:   {}x{}",
        child_size.width(),
        child_size.height()
    );
    println!("child offset: ({}, {})", offset.dx(), offset.dy());
    println!(
        "parent re-read after child layout: padding still {}x{}",
        padding.size(&app).width(),
        padding.size(&app).height()
    );

    padding.mark_needs_layout(&mut app);
    owner.flush_layout(&mut app);
    println!(
        "after flush: padding {}x{}, dirty={}",
        padding.size(&app).width(),
        padding.size(&app).height(),
        padding.debug_needs_layout(&app)
    );
}
