//! Flutter counterpart: `rendering/debug.dart`.

use waft_embedder::{Canvas, Color, Paint, Rect};
use waft_foundation::{App, DebugFlag};
use waft_painting::{Axis, FillRule, PathBuilder};

use crate::box_::BoxConstraints;

macro_rules! debug_flag {
    ($get:ident, $set:ident, $flag:ident, $doc:expr) => {
        #[doc = concat!("The [`App`] debug flag of [`", stringify!($get), "`].")]
        pub struct $flag;

        impl DebugFlag for $flag {}

        #[doc = $doc]
        ///
        /// Dart's library-level bool, kept on the [`App`] (see [`App::debug_flag`]).
        pub fn $get(app: &App) -> bool {
            app.debug_flag::<$flag>()
        }

        #[doc = concat!("Sets [`", stringify!($get), "`] for this app.")]
        pub fn $set(app: &mut App, value: bool) {
            app.set_debug_flag::<$flag>(value);
        }
    };
}

debug_flag!(
    debug_paint_size_enabled,
    set_debug_paint_size_enabled,
    DebugPaintSizeEnabled,
    "Causes each RenderBox to paint a box around its bounds."
);
debug_flag!(
    debug_paint_baselines_enabled,
    set_debug_paint_baselines_enabled,
    DebugPaintBaselinesEnabled,
    "Causes each RenderBox to paint a line at each of its baselines."
);
debug_flag!(
    debug_paint_text_layout_boxes,
    set_debug_paint_text_layout_boxes,
    DebugPaintTextLayoutBoxes,
    "Causes each RenderParagraph to paint the layout boxes of its text."
);
debug_flag!(
    debug_paint_layer_borders_enabled,
    set_debug_paint_layer_borders_enabled,
    DebugPaintLayerBordersEnabled,
    "Causes each Layer to paint a box around its bounds."
);
debug_flag!(
    debug_paint_pointers_enabled,
    set_debug_paint_pointers_enabled,
    DebugPaintPointersEnabled,
    "Causes objects like RenderPointerListener to flash while they are being tapped."
);
debug_flag!(
    debug_repaint_rainbow_enabled,
    set_debug_repaint_rainbow_enabled,
    DebugRepaintRainbowEnabled,
    "Overlay a rotating set of colors when repainting layers in debug mode."
);
debug_flag!(
    debug_repaint_text_rainbow_enabled,
    set_debug_repaint_text_rainbow_enabled,
    DebugRepaintTextRainbowEnabled,
    "Overlay a rotating set of colors when repainting text in debug mode."
);
debug_flag!(
    debug_print_mark_needs_layout_stacks,
    set_debug_print_mark_needs_layout_stacks,
    DebugPrintMarkNeedsLayoutStacks,
    "Log the call stacks that mark render objects as needing layout."
);
debug_flag!(
    debug_print_mark_needs_paint_stacks,
    set_debug_print_mark_needs_paint_stacks,
    DebugPrintMarkNeedsPaintStacks,
    "Log the call stacks that mark render objects as needing paint."
);
debug_flag!(
    debug_print_layouts,
    set_debug_print_layouts,
    DebugPrintLayouts,
    "Log the dirty render objects that are laid out each frame."
);
debug_flag!(
    debug_check_intrinsic_sizes,
    set_debug_check_intrinsic_sizes,
    DebugCheckIntrinsicSizes,
    "Check the intrinsic sizes of each RenderBox during layout."
);
debug_flag!(
    debug_profile_layouts_enabled,
    set_debug_profile_layouts_enabled,
    DebugProfileLayoutsEnabled,
    "Adds Timeline events for every RenderObject layout."
);
debug_flag!(
    debug_profile_paints_enabled,
    set_debug_profile_paints_enabled,
    DebugProfilePaintsEnabled,
    "Adds Timeline events for every RenderObject painted."
);
debug_flag!(
    debug_enhance_layout_timeline_arguments,
    set_debug_enhance_layout_timeline_arguments,
    DebugEnhanceLayoutTimelineArguments,
    "Adds debugging information to Timeline events related to RenderObject layouts."
);
debug_flag!(
    debug_enhance_paint_timeline_arguments,
    set_debug_enhance_paint_timeline_arguments,
    DebugEnhancePaintTimelineArguments,
    "Adds debugging information to Timeline events related to RenderObject paints."
);
debug_flag!(
    debug_disable_clip_layers,
    set_debug_disable_clip_layers,
    DebugDisableClipLayers,
    "Setting to true will cause all clipping effects from the layer tree to be ignored."
);
debug_flag!(
    debug_disable_physical_shape_layers,
    set_debug_disable_physical_shape_layers,
    DebugDisablePhysicalShapeLayers,
    "Setting to true will cause all physical modeling effects from the layer tree to be ignored."
);
debug_flag!(
    debug_disable_opacity_layers,
    set_debug_disable_opacity_layers,
    DebugDisableOpacityLayers,
    "Setting to true will cause all opacity effects from the layer tree to be ignored."
);

fn debug_draw_double_rect(canvas: &mut Canvas, outer_rect: Rect, inner_rect: Rect, color: Color) {
    let mut path = PathBuilder::new();
    path.rect(outer_rect.into());
    path.rect(inner_rect.into());
    let paint = Paint {
        color: color.into(),
        ..Paint::default()
    };
    canvas.draw_path(&path.build(), FillRule::EvenOdd, &paint);
}

/// Paint a diagram showing the given area as padding.
pub fn debug_paint_padding(
    canvas: &mut Canvas,
    outer_rect: Rect,
    inner_rect: Option<Rect>,
    outline_width: f64,
) {
    if cfg!(debug_assertions) {
        if let Some(inner_rect) = inner_rect.filter(|rect| !rect.is_empty()) {
            debug_draw_double_rect(
                canvas,
                outer_rect,
                inner_rect,
                Color::from_argb(0x90, 0x00, 0x90, 0xFF),
            );
            debug_draw_double_rect(
                canvas,
                inner_rect.inflate(outline_width).intersect(outer_rect),
                inner_rect,
                Color::from_argb(0xFF, 0x00, 0x90, 0xFF),
            );
        } else {
            let paint = Paint {
                color: Color::from_argb(0x90, 0x90, 0x90, 0x90).into(),
                ..Paint::default()
            };
            canvas.draw_rect(outer_rect, &paint);
        }
    }
}

/// Returns true if none of the rendering library debug variables have been changed.
pub fn debug_assert_all_render_vars_unset(
    app: &App,
    reason: &str,
    debug_check_intrinsic_sizes_override: bool,
) -> bool {
    debug_assert!(
        !(debug_paint_size_enabled(app)
            || debug_paint_baselines_enabled(app)
            || debug_paint_layer_borders_enabled(app)
            || debug_paint_text_layout_boxes(app)
            || debug_paint_pointers_enabled(app)
            || debug_repaint_rainbow_enabled(app)
            || debug_repaint_text_rainbow_enabled(app)
            || debug_print_mark_needs_layout_stacks(app)
            || debug_print_mark_needs_paint_stacks(app)
            || debug_print_layouts(app)
            || debug_check_intrinsic_sizes(app) != debug_check_intrinsic_sizes_override
            || debug_profile_layouts_enabled(app)
            || debug_profile_paints_enabled(app)
            || debug_disable_clip_layers(app)
            || debug_disable_physical_shape_layers(app)
            || debug_disable_opacity_layers(app)),
        "{reason}"
    );
    true
}

/// Returns true if the given [`Axis`] is bounded within the given
/// [`BoxConstraints`] in both the main and cross axis.
pub fn debug_check_has_bounded_axis(axis: Axis, constraints: BoxConstraints) -> bool {
    debug_assert!(
        constraints.has_bounded_height() && constraints.has_bounded_width(),
        "{axis:?} viewport was given unbounded constraints: {constraints}"
    );
    true
}
