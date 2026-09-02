//! Flutter counterpart: dart:ui `Canvas` / `Paint` / `Path` / `Picture` /
//! `Image` / `Paragraph`. These are valo types under Flutter names.
//!
//! Recording uses valo method signatures (`draw_rect`, f32 `Rect`). Convert
//! framework geometry with [`From`] at the call (`rect.into()`).

use valo::DisplayList;
use valo::DisplayListBuilder;

/// Flutter `Canvas` — valo records into a display list.
pub type Canvas = DisplayListBuilder;

/// Flutter `Picture` — a finished display list, what [`crate::View::present`]
/// takes (Flutter `FlutterView.render` takes a `Scene`).
pub type Picture = DisplayList;

pub use valo::{
    BlendMode, BlurStyle, ClipOp, FillRule, Image, MaskBlur, Paint, PaintStyle, Paragraph,
    ParagraphBuilder, ParagraphStyle, Path, PathBuilder, Stroke, TextStyle,
};

/// The full valo API for hosts and for recording that needs a type the
/// aliases do not cover (`Op`, `Context`, `Surface`).
pub use valo;
