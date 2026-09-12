//! Flutter counterpart: `painting/decoration_image.dart` (`paintImage`, `ImageRepeat`).
//!
//! Drawing an image into a box is mostly deciding which part of the image goes where: how it is
//! fitted to the box, where it sits when it does not fill it, and what happens in the space left
//! over. That decision is shared by every image on screen, so it lives here rather than in the
//! render object that calls it.
//!
//! `DecorationImage` itself, the nine-patch `centerSlice`, and the colour filter and inversion
//! Dart accepts are not here; see `PORTING.md`.

use inset_embedder::{Canvas, Image, Offset, Paint, Rect, Size};

use crate::alignment::Alignment;
use crate::box_fit::{BoxFit, apply_box_fit};

/// How to fill space an image does not cover. Flutter `ImageRepeat`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImageRepeat {
    /// Repeat in both directions until the box is full.
    Repeat,
    /// Repeat only across.
    RepeatX,
    /// Repeat only down.
    RepeatY,
    /// Leave the rest of the box empty.
    #[default]
    NoRepeat,
}

/// How an image is drawn into a box, beyond the image itself. Flutter's `paintImage` arguments.
#[derive(Clone, Debug)]
pub struct ImagePaintStyle {
    /// Pixels per logical point in the image, which decides its unfitted size.
    pub scale: f64,
    /// How opaque to draw it, from zero to one.
    pub opacity: f64,
    /// How to fit it to the box. `None` means Dart's default of scaling down only if it
    /// does not already fit.
    pub fit: Option<BoxFit>,
    /// Where it sits in the box when it does not fill it.
    pub alignment: Alignment,
    /// What to do with the space it does not cover.
    pub repeat: ImageRepeat,
    /// Draw it mirrored, which is how a directional image faces the other way in a
    /// right-to-left layout.
    pub flip_horizontally: bool,
}

impl Default for ImagePaintStyle {
    fn default() -> ImagePaintStyle {
        ImagePaintStyle {
            scale: 1.0,
            opacity: 1.0,
            fit: None,
            alignment: Alignment::CENTER,
            repeat: ImageRepeat::NoRepeat,
            flip_horizontally: false,
        }
    }
}

/// Draws `image` into `rect`. Flutter `paintImage`.
///
/// Nothing is drawn for an empty box, a zero-sized image or an opacity of zero, which is what
/// keeps a widget that has been laid out to nothing from reaching the renderer at all.
pub fn paint_image(canvas: &mut Canvas, rect: Rect, image: &Image, style: &ImagePaintStyle) {
    let [pixel_width, pixel_height] = image.size();
    let input_size = Size::new(f64::from(pixel_width), f64::from(pixel_height));
    if rect.is_empty() || input_size.is_empty() || style.opacity <= 0.0 {
        return;
    }

    // Dart's default: shrink an image too big for the box, and leave a smaller one alone.
    let fit = style.fit.unwrap_or(BoxFit::ScaleDown);
    let output_size = rect.size();
    let fitted = apply_box_fit(
        fit,
        Size::new(
            input_size.width() / style.scale,
            input_size.height() / style.scale,
        ),
        output_size,
    );
    let source_size = Size::new(
        fitted.source.width() * style.scale,
        fitted.source.height() * style.scale,
    );
    let destination_size = fitted.destination;

    // A repeated image tiles from its destination, so it has to be drawn at its own size
    // rather than stretched to the box.
    let alignment = horizontal_flip(style.alignment, style.flip_horizontally);
    let source = alignment.inscribe(source_size, Offset::ZERO & input_size);
    let destination = alignment.inscribe(destination_size, rect);

    // The renderer tints an image by the paint's alpha alone; its colour channels are not read.
    let paint = Paint {
        color: valo_color_with_alpha(style.opacity),
        ..Paint::default()
    };

    for tile in tiles(rect, destination, style.repeat) {
        canvas.draw_image_rect(
            image,
            source.into(),
            tile.into(),
            Default::default(),
            &paint,
        );
    }
}

/// Mirroring is expressed as the alignment reading from the other side, which flips both the
/// part of the image taken and where it lands.
fn horizontal_flip(alignment: Alignment, flip: bool) -> Alignment {
    if flip {
        Alignment::new(-alignment.x, alignment.y)
    } else {
        alignment
    }
}

/// Every rectangle the image is drawn into: one, or enough to cover `rect` when repeating.
fn tiles(rect: Rect, destination: Rect, repeat: ImageRepeat) -> Vec<Rect> {
    if repeat == ImageRepeat::NoRepeat {
        return vec![destination];
    }
    let (width, height) = (destination.width(), destination.height());
    if width <= 0.0 || height <= 0.0 {
        return vec![destination];
    }
    let across = matches!(repeat, ImageRepeat::Repeat | ImageRepeat::RepeatX);
    let down = matches!(repeat, ImageRepeat::Repeat | ImageRepeat::RepeatY);
    let start_x = if across {
        steps_before(rect.left, destination.left, width)
    } else {
        0
    };
    let stop_x = if across {
        steps_after(rect.right, destination.right, width)
    } else {
        0
    };
    let start_y = if down {
        steps_before(rect.top, destination.top, height)
    } else {
        0
    };
    let stop_y = if down {
        steps_after(rect.bottom, destination.bottom, height)
    } else {
        0
    };

    let mut tiles = Vec::new();
    for step_y in -start_y..=stop_y {
        for step_x in -start_x..=stop_x {
            tiles
                .push(destination.translate(f64::from(step_x) * width, f64::from(step_y) * height));
        }
    }
    tiles
}

/// How many whole tiles fit between the box's edge and the image's, going backwards.
fn steps_before(edge: f64, from: f64, step: f64) -> i32 {
    (((from - edge) / step).ceil() as i32).max(0)
}

/// How many whole tiles fit between the image's edge and the box's, going forwards.
fn steps_after(edge: f64, from: f64, step: f64) -> i32 {
    (((edge - from) / step).ceil() as i32).max(0)
}

/// White at `opacity`; only the alpha reaches the image shader.
fn valo_color_with_alpha(opacity: f64) -> inset_embedder::valo::Color {
    let alpha = opacity.clamp(0.0, 1.0) as f32;
    inset_embedder::valo::Color::rgba(alpha, alpha, alpha, alpha)
}
