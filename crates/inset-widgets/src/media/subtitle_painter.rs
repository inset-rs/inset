// Ported from Media3's `SubtitlePainter.java` (https://github.com/androidx/media), Copyright (C)
// 2016 The Android Open Source Project, licensed under the Apache License, Version 2.0
// (http://www.apache.org/licenses/LICENSE-2.0). Changed: it paints Inset's `Cue`, with the user's
// caption style merged into each run setting by setting (`cue_style`), on Inset's canvas, rounds
// the window's corners as the style says, and keeps no layout between draws.

//! Media3's `SubtitlePainter`: where one cue's box goes over the picture, and how its text, the
//! edges round its characters and the window behind it, or its picture, are painted, in the
//! media's own style with the user's caption style merged in (`cue_style`).
//!
//! The invariant: a text cue is laid out twice at the same width with the same runs, once for its
//! edges and once for its text, so the edge pass lands under every glyph of the text pass.

use inset_embedder::valo::{AlphaType, PixelBuffer, PixelFormat, PixelLayout};
use inset_embedder::{
    Canvas, CanvasText, CaptionStyle, Color, Cue, CueAnchor, CueContent, CueImage, CueLine,
    CueTextAlignment, CueTextEdge, CueTextRun, FontCollection, FontStyle, FontWeight, Image,
    Offset, Paint, PaintStyle, Paragraph, ParagraphBuilder, ParagraphConstraints, ParagraphStyle,
    RRect, Radius, Rect, Shadow, Stroke, TextAlign, TextDecoration, TextDirection, TextStyle,
};
use inset_foundation::App;
use inset_painting::{PaintingBinding, draw_rrect};

use super::cue_style::{StyledRun, styled_runs, window_color, window_corner_radius};

/// Ratio of inner padding to font size.
const INNER_PADDING_RATIO: f64 = 0.125;

/// Media3's two density-independent pixels, in logical pixels: an outline's width, and a
/// shadow's radius and offset.
const EDGE_EXTENT: f64 = 2.0;

const WHITE: Color = Color::new(0xFFFF_FFFF);
const TRANSPARENT: Color = Color::new(0x0000_0000);

/// Draws `cue` into `canvas` over `parent`, the picture's rectangle, with `style` merged in.
/// `bottom_padding_fraction` is the space left under a cue with no `line`, as a fraction of the
/// picture's height.
pub(super) fn draw(
    app: &mut App,
    canvas: &mut Canvas,
    cue: &Cue,
    style: &CaptionStyle,
    bottom_padding_fraction: f64,
    parent: Rect,
) {
    match &cue.content {
        CueContent::Text(runs) => draw_text_cue(
            app,
            canvas,
            cue,
            runs,
            style,
            bottom_padding_fraction,
            parent,
        ),
        CueContent::Image(image) => draw_bitmap_cue(app, canvas, cue, image, parent),
    }
}

/// Lays out and paints a cue of `runs`; nothing where they hold no text or there is no room.
fn draw_text_cue(
    app: &mut App,
    canvas: &mut Canvas,
    cue: &Cue,
    runs: &[CueTextRun],
    style: &CaptionStyle,
    bottom_padding_fraction: f64,
    parent: Rect,
) {
    if runs.iter().all(|run| run.text.is_empty()) {
        // Nothing to draw.
        return;
    }
    let runs = styled_runs(runs, style);
    let fonts = PaintingBinding::instance(app).fonts(app);
    let fonts = app.get_mut(fonts);
    if let Some(layout) = setup_text_layout(cue, &runs, bottom_padding_fraction, parent, fonts) {
        let window = Window {
            color: window_color(cue, style),
            corner_radius: window_corner_radius(style),
        };
        draw_text_layout(canvas, &layout, &window);
    }
}

/// Places and paints a picture cue; nothing where it has no place or the platform no renderer.
fn draw_bitmap_cue(app: &App, canvas: &mut Canvas, cue: &Cue, image: &CueImage, parent: Rect) {
    let Some(bitmap_rect) = setup_bitmap_layout(cue, image, parent) else {
        return;
    };
    if let Some(bitmap) = image_of(app, image) {
        draw_bitmap_layout(canvas, &bitmap, bitmap_rect);
    }
}

/// The box behind a text cue.
struct Window {
    color: Color,
    /// In logical pixels.
    corner_radius: f64,
}

/// A text cue laid out where it goes: Media3's derived drawing variables.
struct TextLayout {
    text_layout: Paragraph,
    /// Present while a run has an edge drawn beneath its text.
    edge_layout: Option<Paragraph>,
    text_left: f64,
    text_top: f64,
    text_padding_x: f64,
}

/// Lays `runs` out where `cue` places them in `parent`; `None` where there is no room.
fn setup_text_layout(
    cue: &Cue,
    runs: &[StyledRun],
    bottom_padding_fraction: f64,
    parent: Rect,
    fonts: &mut FontCollection,
) -> Option<TextLayout> {
    let parent_width = parent.width();
    let parent_height = parent.height();

    let text_padding_x = largest_text_size(runs, parent_height) * INNER_PADDING_RATIO;

    let mut available_width = parent_width - text_padding_x * 2.0;
    if let Some(cue_size) = cue.size {
        available_width *= f64::from(cue_size);
    }
    if available_width <= 0.0 {
        // Skipped drawing subtitle cue (insufficient space).
        return None;
    }

    let text_alignment = text_align(cue.text_alignment);
    let mut text_layout = paragraph(runs, text_alignment, parent_height, text_style, fonts);
    text_layout.layout(ParagraphConstraints::new(available_width));
    let text_height = text_layout.height();
    let line_metrics = text_layout.compute_line_metrics();
    let mut text_width = line_metrics
        .iter()
        .map(|line| line.width.ceil())
        .fold(0.0, f64::max);
    if cue.size.is_some() && text_width < available_width {
        text_width = available_width;
    }
    text_width += text_padding_x * 2.0;

    let (text_left, text_right) = horizontal_extent(cue, parent, text_width);
    let text_width = text_right - text_left;
    if text_width <= 0.0 {
        // Skipped drawing subtitle cue (invalid horizontal positioning).
        return None;
    }

    let first_line_height = line_metrics.first().map_or(0.0, |line| line.height);
    let text_top = text_top(
        cue,
        parent,
        text_height,
        first_line_height,
        bottom_padding_fraction,
    );

    // Update the derived drawing variables.
    text_layout.layout(ParagraphConstraints::new(text_width));
    let edge_layout = runs.iter().any(has_edge_layout).then(|| {
        let mut edge_layout = paragraph(runs, text_alignment, parent_height, edge_style, fonts);
        edge_layout.layout(ParagraphConstraints::new(text_width));
        edge_layout
    });
    Some(TextLayout {
        text_layout,
        edge_layout,
        text_left,
        text_top,
        text_padding_x,
    })
}

/// Where a text box `text_width` wide goes across `parent`: its left and right.
fn horizontal_extent(cue: &Cue, parent: Rect, text_width: f64) -> (f64, f64) {
    let Some(cue_position) = cue.position else {
        let text_left = (parent.width() - text_width) / 2.0 + parent.left;
        return (text_left, text_left + text_width);
    };
    let anchor_position = parent.width() * f64::from(cue_position) + parent.left;
    let text_left = match cue.position_anchor {
        CueAnchor::End => anchor_position - text_width,
        CueAnchor::Middle => (anchor_position * 2.0 - text_width) / 2.0,
        CueAnchor::Start => anchor_position,
    };
    let text_left = text_left.max(parent.left);
    let text_right = (text_left + text_width).min(parent.right);
    (text_left, text_right)
}

/// Where a text box `text_height` high goes down `parent`: its top.
fn text_top(
    cue: &Cue,
    parent: Rect,
    text_height: f64,
    first_line_height: f64,
    bottom_padding_fraction: f64,
) -> f64 {
    let Some(cue_line) = cue.line else {
        return parent.bottom - text_height - parent.height() * bottom_padding_fraction;
    };
    let text_top = match cue_line {
        CueLine::Fraction(line) => {
            let anchor_position = parent.height() * f64::from(line) + parent.top;
            match cue.line_anchor {
                CueAnchor::End => anchor_position - text_height,
                CueAnchor::Middle => (anchor_position * 2.0 - text_height) / 2.0,
                CueAnchor::Start => anchor_position,
            }
        }
        CueLine::Number(line) if line >= 0 => f64::from(line) * first_line_height + parent.top,
        CueLine::Number(line) => {
            f64::from(line + 1) * first_line_height + parent.bottom - text_height
        }
    };
    if text_top + text_height > parent.bottom {
        parent.bottom - text_height
    } else if text_top < parent.top {
        parent.top
    } else {
        text_top
    }
}

/// The rectangle a picture cue fills in `parent`; `None` for a cue without the line, position
/// and size a picture is placed by, which Media3 places out of sight.
fn setup_bitmap_layout(cue: &Cue, image: &CueImage, parent: Rect) -> Option<Rect> {
    let (Some(cue_line), Some(cue_position), Some(cue_size)) = (cue.line, cue.position, cue.size)
    else {
        return None;
    };
    // Media3 reads a picture's line as a fraction, whatever its type.
    let cue_line = match cue_line {
        CueLine::Fraction(line) => f64::from(line),
        CueLine::Number(line) => f64::from(line),
    };
    let anchor_x = parent.left + parent.width() * f64::from(cue_position);
    let anchor_y = parent.top + parent.height() * cue_line;
    let width = parent.width() * f64::from(cue_size);
    let height = parent.height() * f64::from(image.display_height);
    let x = anchored(anchor_x, width, cue.position_anchor);
    let y = anchored(anchor_y, height, cue.line_anchor);
    Some(Rect::from_ltwh(x, y, width, height))
}

/// Where a span `extent` long starts when its `anchor` side is at `anchor_position`.
fn anchored(anchor_position: f64, extent: f64, anchor: CueAnchor) -> f64 {
    match anchor {
        CueAnchor::End => anchor_position - extent,
        CueAnchor::Middle => anchor_position - extent / 2.0,
        CueAnchor::Start => anchor_position,
    }
}

/// Paints a laid out text cue: the window behind it, its edges, then its text.
fn draw_text_layout(canvas: &mut Canvas, layout: &TextLayout, window: &Window) {
    canvas.save();
    canvas.translate(layout.text_left as f32, layout.text_top as f32);

    if window.color.a > 0.0 {
        let text_layout = &layout.text_layout;
        let bounds = Rect::from_ltrb(
            -layout.text_padding_x,
            0.0,
            text_layout.width() + layout.text_padding_x,
            text_layout.height(),
        );
        draw_rrect(
            canvas,
            RRect::from_rect_and_radius(bounds, Radius::circular(window.corner_radius)),
            &Paint::from_color(window.color.into()),
        );
    }

    if let Some(edge_layout) = &layout.edge_layout {
        canvas.draw_paragraph(edge_layout, Offset::ZERO);
    }
    canvas.draw_paragraph(&layout.text_layout, Offset::ZERO);

    canvas.restore();
}

/// Paints a picture cue's pixels into `bitmap_rect`.
fn draw_bitmap_layout(canvas: &mut Canvas, bitmap: &Image, bitmap_rect: Rect) {
    canvas.draw_image(bitmap, bitmap_rect.into(), &Paint::default());
}

/// `runs` as one paragraph, each in the style `style_of` gives it.
fn paragraph(
    runs: &[StyledRun],
    text_align: TextAlign,
    parent_height: f64,
    style_of: fn(&StyledRun, f64) -> TextStyle,
    fonts: &mut FontCollection,
) -> Paragraph {
    let mut builder = ParagraphBuilder::new(ParagraphStyle {
        text_align: Some(text_align),
        text_direction: Some(TextDirection::Ltr),
        ..ParagraphStyle::new()
    });
    for run in runs {
        builder.push_style(style_of(run, parent_height));
        builder.add_text(run.text);
        builder.pop();
    }
    builder.build(fonts)
}

/// The style `run`'s text is painted in, above its edges: Media3's text layout, whose paint
/// casts a drop shadow, or the lower shadow of a raised or depressed edge.
fn text_style(run: &StyledRun, parent_height: f64) -> TextStyle {
    let shadow = match run.edge {
        CueTextEdge::DropShadow => Some(Shadow::new(
            run.edge_color,
            Offset::new(EDGE_EXTENT, EDGE_EXTENT),
            EDGE_EXTENT,
        )),
        CueTextEdge::Raised | CueTextEdge::Depressed => {
            let offset = EDGE_EXTENT / 2.0;
            Some(Shadow::new(
                lower_shadow_color(run),
                Offset::new(offset, offset),
                EDGE_EXTENT,
            ))
        }
        CueTextEdge::None | CueTextEdge::Outline => None,
    };
    TextStyle {
        color: Some(run.color),
        background: background_in_text_layout(run).map(paint_of),
        shadows: shadow.map(|shadow| vec![shadow]),
        ..run_font(run, parent_height)
    }
}

/// The style `run`'s edges are painted in, beneath its text: Media3's edge layout. A run whose
/// edge the text layout draws, or that has none, is clear here.
fn edge_style(run: &StyledRun, parent_height: f64) -> TextStyle {
    let font = run_font(run, parent_height);
    let background = visible_background(run).map(paint_of);
    match run.edge {
        CueTextEdge::Outline => TextStyle {
            foreground: Some(outline_paint(run.edge_color)),
            background,
            ..font
        },
        CueTextEdge::Raised | CueTextEdge::Depressed => {
            let offset = EDGE_EXTENT / 2.0;
            TextStyle {
                color: Some(run.color),
                background,
                shadows: Some(vec![Shadow::new(
                    upper_shadow_color(run),
                    Offset::new(-offset, -offset),
                    EDGE_EXTENT,
                )]),
                ..font
            }
        }
        CueTextEdge::None | CueTextEdge::DropShadow => TextStyle {
            color: Some(TRANSPARENT),
            ..font
        },
    }
}

/// `run`'s size, weight, slant, underline and font, which both of its layouts share.
fn run_font(run: &StyledRun, parent_height: f64) -> TextStyle {
    TextStyle {
        font_size: Some(text_size(run, parent_height)),
        font_weight: run.bold.then_some(FontWeight::BOLD),
        font_style: run.italic.then_some(FontStyle::Italic),
        decoration: run.underline.then_some(TextDecoration::UNDERLINE),
        font_family: run.font_family.clone(),
        ..TextStyle::new()
    }
}

/// Whether `run` has an edge painted beneath its text.
fn has_edge_layout(run: &StyledRun) -> bool {
    matches!(
        run.edge,
        CueTextEdge::Outline | CueTextEdge::Raised | CueTextEdge::Depressed
    )
}

/// `run`'s background where its text layout paints it: in front of an edge that has no layout of
/// its own, and behind the edge layout otherwise, so the background never covers an edge.
fn background_in_text_layout(run: &StyledRun) -> Option<Color> {
    visible_background(run).filter(|_| !has_edge_layout(run))
}

/// `run`'s background where it shows at all: Media3 paints none that is wholly transparent.
fn visible_background(run: &StyledRun) -> Option<Color> {
    Some(run.background).filter(|color| color.a > 0.0)
}

/// The light, upper-left shadow of a raised edge, or the dark one of a depressed edge.
fn upper_shadow_color(run: &StyledRun) -> Color {
    if run.edge == CueTextEdge::Raised {
        WHITE
    } else {
        run.edge_color
    }
}

/// The dark, lower-right shadow of a raised edge, or the light one of a depressed edge.
fn lower_shadow_color(run: &StyledRun) -> Color {
    if run.edge == CueTextEdge::Raised {
        run.edge_color
    } else {
        WHITE
    }
}

/// The paint an outline is stroked with, round at its corners.
fn outline_paint(edge_color: Color) -> Paint {
    let mut stroke = Stroke::new(EDGE_EXTENT as f32);
    stroke.join = inset_embedder::valo::Join::Round;
    Paint {
        style: PaintStyle::Stroke(stroke),
        ..Paint::from_color(edge_color.into())
    }
}

fn paint_of(color: Color) -> Paint {
    Paint::from_color(color.into())
}

/// `run`'s text size in logical pixels.
fn text_size(run: &StyledRun, parent_height: f64) -> f64 {
    f64::from(run.font_size) * parent_height
}

/// The size of the largest text among `runs`, in logical pixels, which the space each side of
/// the lines follows.
fn largest_text_size(runs: &[StyledRun], parent_height: f64) -> f64 {
    runs.iter()
        .map(|run| text_size(run, parent_height))
        .fold(0.0, f64::max)
}

fn text_align(alignment: CueTextAlignment) -> TextAlign {
    match alignment {
        CueTextAlignment::Start => TextAlign::Start,
        CueTextAlignment::Center => TextAlign::Center,
        CueTextAlignment::End => TextAlign::End,
    }
}

/// A picture cue's pixels as an image; `None` while the platform has no renderer.
fn image_of(app: &App, image: &CueImage) -> Option<Image> {
    if image.width == 0 || image.height == 0 {
        return None;
    }
    let layout = PixelLayout::packed(
        [image.width, image.height],
        PixelFormat::Rgba8,
        AlphaType::Straight,
    );
    let pixels = PixelBuffer::new(layout, image.pixels.clone()).ok()?;
    app.platform().import_pixels(pixels)
}

#[cfg(test)]
mod tests {
    use inset_embedder::CaptionSetting;
    use inset_embedder::valo::Op;
    use inset_foundation::AppCell;

    use super::*;

    const PICTURE: Rect = Rect::from_ltwh(0.0, 0.0, 640.0, 360.0);

    fn one_line(text: &str) -> Cue {
        Cue::new(CueContent::Text(vec![CueTextRun::new(text)]))
    }

    /// What the shell does at start-up: the app-wide fonts, here the OS fonts.
    fn install_fonts(app: &mut App) {
        let binding = PaintingBinding::instance(app);
        if !binding.has_fonts(app) {
            binding.install_fonts(app, |fonts| {
                fonts.add_source(valo_system_fonts::SystemFonts::load());
            });
        }
    }

    fn painted(cue: &Cue) -> Vec<Op> {
        painted_in(cue, &CaptionStyle::default())
    }

    fn painted_in(cue: &Cue, style: &CaptionStyle) -> Vec<Op> {
        let cell = AppCell::new();
        let mut app = cell.borrow_mut();
        install_fonts(&mut app);
        let mut canvas = Canvas::new();
        draw(&mut app, &mut canvas, cue, style, 0.08, PICTURE);
        canvas.build().ops().to_vec()
    }

    #[test]
    fn a_cue_without_line_or_position_sits_centred_above_the_bottom_padding() {
        let cue = one_line("Hello");
        let (left, right) = horizontal_extent(&cue, PICTURE, 100.0);
        assert_eq!((left, right), (270.0, 370.0));
        let top = text_top(&cue, PICTURE, 20.0, 20.0, 0.08);
        assert_eq!(top, 360.0 - 20.0 - 360.0 * 0.08);
    }

    #[test]
    fn a_position_places_the_side_its_anchor_names_and_stays_inside_the_picture() {
        let mut cue = one_line("Hello");
        cue.position = Some(0.5);
        cue.position_anchor = CueAnchor::End;
        assert_eq!(horizontal_extent(&cue, PICTURE, 100.0), (220.0, 320.0));
        cue.position_anchor = CueAnchor::Middle;
        assert_eq!(horizontal_extent(&cue, PICTURE, 100.0), (270.0, 370.0));
        cue.position = Some(0.875);
        cue.position_anchor = CueAnchor::Start;
        assert_eq!(
            horizontal_extent(&cue, PICTURE, 100.0),
            (560.0, 640.0),
            "cut at the picture's right edge"
        );
    }

    #[test]
    fn a_line_places_the_box_by_fraction_or_by_line_number_and_stays_inside_the_picture() {
        let mut cue = one_line("Hello");
        cue.line = Some(CueLine::Fraction(0.5));
        cue.line_anchor = CueAnchor::Middle;
        assert_eq!(text_top(&cue, PICTURE, 40.0, 20.0, 0.08), 160.0);
        cue.line = Some(CueLine::Number(1));
        assert_eq!(text_top(&cue, PICTURE, 40.0, 20.0, 0.08), 20.0);
        cue.line = Some(CueLine::Number(-1));
        assert_eq!(
            text_top(&cue, PICTURE, 40.0, 20.0, 0.08),
            320.0,
            "line -1 is the last"
        );
        cue.line = Some(CueLine::Fraction(1.0));
        cue.line_anchor = CueAnchor::Start;
        assert_eq!(text_top(&cue, PICTURE, 40.0, 20.0, 0.08), 320.0);
    }

    #[test]
    fn a_picture_cue_fills_its_size_and_height_at_its_anchors() {
        let image = CueImage {
            width: 2,
            height: 1,
            pixels: vec![255; 8],
            display_height: 0.125,
        };
        let mut cue = Cue::new(CueContent::Image(image.clone()));
        assert_eq!(setup_bitmap_layout(&cue, &image, PICTURE), None);
        cue.line = Some(CueLine::Fraction(0.875));
        cue.line_anchor = CueAnchor::End;
        cue.position = Some(0.5);
        cue.position_anchor = CueAnchor::Middle;
        cue.size = Some(0.5);
        assert_eq!(
            setup_bitmap_layout(&cue, &image, PICTURE),
            Some(Rect::from_ltwh(160.0, 270.0, 320.0, 45.0))
        );
    }

    #[test]
    fn a_text_cue_paints_its_window_then_its_text_in_its_colour() {
        let mut cue = one_line("Hello");
        cue.window_color = Some(Color::new(0x8000_0000));
        let ops = painted(&cue);
        let window = ops
            .iter()
            .position(|op| matches!(op, Op::DrawRect { paint, .. } if paint.color == Color::new(0x8000_0000).into()))
            .expect("the window behind the text");
        let text = ops
            .iter()
            .position(|op| matches!(op, Op::GlyphRun { paint, .. } if paint.color == WHITE.into()))
            .expect("the text, white");
        assert!(window < text, "the window under the text: {ops:?}");
    }

    #[test]
    fn the_window_takes_the_users_colour_and_rounded_corners_where_the_media_gives_none() {
        let style = CaptionStyle {
            window_color: Some(CaptionSetting::where_media_says_nothing(Color::new(
                0xFF20_4060,
            ))),
            window_opacity: Some(CaptionSetting::where_media_says_nothing(0.5)),
            window_corner_radius: Some(CaptionSetting::where_media_says_nothing(6.0)),
            ..CaptionStyle::default()
        };
        let ops = painted_in(&one_line("Hello"), &style);
        let window = Color::new(0xFF20_4060).with_values(Some(0.5), None, None, None, None);
        assert!(
            ops.iter()
                .any(|op| matches!(op, Op::DrawPath { paint, .. } if paint.color == window.into())),
            "a rounded window in the user's colour: {ops:?}"
        );
    }

    #[test]
    fn a_drop_shadow_is_cast_in_the_edge_colour_and_a_cue_without_text_paints_nothing() {
        let mut cue = one_line("Hello");
        let CueContent::Text(runs) = &mut cue.content else {
            unreachable!()
        };
        runs[0].edge = Some(CueTextEdge::DropShadow);
        runs[0].edge_color = Some(Color::new(0xFF10_2030));
        let ops = painted(&cue);
        assert!(
            ops.iter().any(|op| matches!(op, Op::GlyphRun { paint, .. } if paint.color == Color::new(0xFF10_2030).into())),
            "the shadow: {ops:?}"
        );
        assert!(painted(&one_line("")).is_empty());
    }
}
