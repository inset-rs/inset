//! Flutter counterpart: `rendering/debug.dart`.

use std::sync::atomic::{AtomicBool, Ordering};

use inset_embedder::{Canvas, Color, Paint, Rect};
use inset_painting::{Axis, FillRule, PathBuilder};

use crate::box_::BoxConstraints;

static DEBUG_PAINT_SIZE_ENABLED: AtomicBool = AtomicBool::new(false);
static DEBUG_PAINT_BASELINES_ENABLED: AtomicBool = AtomicBool::new(false);
static DEBUG_PAINT_TEXT_LAYOUT_BOXES: AtomicBool = AtomicBool::new(false);
static DEBUG_PAINT_LAYER_BORDERS_ENABLED: AtomicBool = AtomicBool::new(false);
static DEBUG_PAINT_POINTERS_ENABLED: AtomicBool = AtomicBool::new(false);
static DEBUG_REPAINT_RAINBOW_ENABLED: AtomicBool = AtomicBool::new(false);
static DEBUG_REPAINT_TEXT_RAINBOW_ENABLED: AtomicBool = AtomicBool::new(false);
static DEBUG_PRINT_MARK_NEEDS_LAYOUT_STACKS: AtomicBool = AtomicBool::new(false);
static DEBUG_PRINT_MARK_NEEDS_PAINT_STACKS: AtomicBool = AtomicBool::new(false);
static DEBUG_PRINT_LAYOUTS: AtomicBool = AtomicBool::new(false);
static DEBUG_CHECK_INTRINSIC_SIZES: AtomicBool = AtomicBool::new(false);
static DEBUG_PROFILE_LAYOUTS_ENABLED: AtomicBool = AtomicBool::new(false);
static DEBUG_PROFILE_PAINTS_ENABLED: AtomicBool = AtomicBool::new(false);
static DEBUG_ENHANCE_LAYOUT_TIMELINE_ARGUMENTS: AtomicBool = AtomicBool::new(false);
static DEBUG_ENHANCE_PAINT_TIMELINE_ARGUMENTS: AtomicBool = AtomicBool::new(false);
static DEBUG_DISABLE_CLIP_LAYERS: AtomicBool = AtomicBool::new(false);
static DEBUG_DISABLE_PHYSICAL_SHAPE_LAYERS: AtomicBool = AtomicBool::new(false);
static DEBUG_DISABLE_OPACITY_LAYERS: AtomicBool = AtomicBool::new(false);

macro_rules! debug_flag {
    ($get:ident, $set:ident, $static:ident, $doc:expr) => {
        #[doc = $doc]
        pub fn $get() -> bool {
            $static.load(Ordering::Relaxed)
        }

        #[doc = concat!("Sets [`", stringify!($get), "`].")]
        pub fn $set(value: bool) {
            $static.store(value, Ordering::Relaxed);
        }
    };
}

debug_flag!(
    debug_paint_size_enabled,
    set_debug_paint_size_enabled,
    DEBUG_PAINT_SIZE_ENABLED,
    "Causes each RenderBox to paint a box around its bounds."
);
debug_flag!(
    debug_paint_baselines_enabled,
    set_debug_paint_baselines_enabled,
    DEBUG_PAINT_BASELINES_ENABLED,
    "Causes each RenderBox to paint a line at each of its baselines."
);
debug_flag!(
    debug_paint_text_layout_boxes,
    set_debug_paint_text_layout_boxes,
    DEBUG_PAINT_TEXT_LAYOUT_BOXES,
    "Causes each RenderParagraph to paint the layout boxes of its text."
);
debug_flag!(
    debug_paint_layer_borders_enabled,
    set_debug_paint_layer_borders_enabled,
    DEBUG_PAINT_LAYER_BORDERS_ENABLED,
    "Causes each Layer to paint a box around its bounds."
);
debug_flag!(
    debug_paint_pointers_enabled,
    set_debug_paint_pointers_enabled,
    DEBUG_PAINT_POINTERS_ENABLED,
    "Causes objects like RenderPointerListener to flash while they are being tapped."
);
debug_flag!(
    debug_repaint_rainbow_enabled,
    set_debug_repaint_rainbow_enabled,
    DEBUG_REPAINT_RAINBOW_ENABLED,
    "Overlay a rotating set of colors when repainting layers in debug mode."
);
debug_flag!(
    debug_repaint_text_rainbow_enabled,
    set_debug_repaint_text_rainbow_enabled,
    DEBUG_REPAINT_TEXT_RAINBOW_ENABLED,
    "Overlay a rotating set of colors when repainting text in debug mode."
);
debug_flag!(
    debug_print_mark_needs_layout_stacks,
    set_debug_print_mark_needs_layout_stacks,
    DEBUG_PRINT_MARK_NEEDS_LAYOUT_STACKS,
    "Log the call stacks that mark render objects as needing layout."
);
debug_flag!(
    debug_print_mark_needs_paint_stacks,
    set_debug_print_mark_needs_paint_stacks,
    DEBUG_PRINT_MARK_NEEDS_PAINT_STACKS,
    "Log the call stacks that mark render objects as needing paint."
);
debug_flag!(
    debug_print_layouts,
    set_debug_print_layouts,
    DEBUG_PRINT_LAYOUTS,
    "Log the dirty render objects that are laid out each frame."
);
debug_flag!(
    debug_check_intrinsic_sizes,
    set_debug_check_intrinsic_sizes,
    DEBUG_CHECK_INTRINSIC_SIZES,
    "Check the intrinsic sizes of each RenderBox during layout."
);
debug_flag!(
    debug_profile_layouts_enabled,
    set_debug_profile_layouts_enabled,
    DEBUG_PROFILE_LAYOUTS_ENABLED,
    "Adds Timeline events for every RenderObject layout."
);
debug_flag!(
    debug_profile_paints_enabled,
    set_debug_profile_paints_enabled,
    DEBUG_PROFILE_PAINTS_ENABLED,
    "Adds Timeline events for every RenderObject painted."
);
debug_flag!(
    debug_enhance_layout_timeline_arguments,
    set_debug_enhance_layout_timeline_arguments,
    DEBUG_ENHANCE_LAYOUT_TIMELINE_ARGUMENTS,
    "Adds debugging information to Timeline events related to RenderObject layouts."
);
debug_flag!(
    debug_enhance_paint_timeline_arguments,
    set_debug_enhance_paint_timeline_arguments,
    DEBUG_ENHANCE_PAINT_TIMELINE_ARGUMENTS,
    "Adds debugging information to Timeline events related to RenderObject paints."
);
debug_flag!(
    debug_disable_clip_layers,
    set_debug_disable_clip_layers,
    DEBUG_DISABLE_CLIP_LAYERS,
    "Setting to true will cause all clipping effects from the layer tree to be ignored."
);
debug_flag!(
    debug_disable_physical_shape_layers,
    set_debug_disable_physical_shape_layers,
    DEBUG_DISABLE_PHYSICAL_SHAPE_LAYERS,
    "Setting to true will cause all physical modeling effects from the layer tree to be ignored."
);
debug_flag!(
    debug_disable_opacity_layers,
    set_debug_disable_opacity_layers,
    DEBUG_DISABLE_OPACITY_LAYERS,
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
    reason: &str,
    debug_check_intrinsic_sizes_override: bool,
) -> bool {
    debug_assert!(
        !(debug_paint_size_enabled()
            || debug_paint_baselines_enabled()
            || debug_paint_layer_borders_enabled()
            || debug_paint_text_layout_boxes()
            || debug_paint_pointers_enabled()
            || debug_repaint_rainbow_enabled()
            || debug_repaint_text_rainbow_enabled()
            || debug_print_mark_needs_layout_stacks()
            || debug_print_mark_needs_paint_stacks()
            || debug_print_layouts()
            || debug_check_intrinsic_sizes() != debug_check_intrinsic_sizes_override
            || debug_profile_layouts_enabled()
            || debug_profile_paints_enabled()
            || debug_disable_clip_layers()
            || debug_disable_physical_shape_layers()
            || debug_disable_opacity_layers()),
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
