//! `Image`: a picture decoded by the host and drawn into the space it is given.
//!
//! The sample files are embedded in the binary so the gallery runs without external assets.
//! Static pictures and animation use the same image-provider, codec and cache path.

use inset_cupertino::CupertinoColors;
use inset_foundation::App;
use inset_painting::{Alignment, BoxFit, ImageRepeat};
use inset_rendering::MainAxisAlignment;
use inset_widgets::{BuildContext, ColoredBox, Image, IntoWidget, Row, SizedBox, WidgetRef};

use crate::support::{screen, scrolling_body, section_with_footer, showcase};

/// A ninety-six by sixty-four swatch with a dark border, so a fit that crops or stretches it is
/// unmistakable.
const SWATCH: &[u8] = include_bytes!("../../assets/swatch.png");

/// A round badge on transparency, for opacity and tiling.
const BADGE: &[u8] = include_bytes!("../../assets/badge.png");

/// Three dots orbit continuously over 48 frames, at 50 milliseconds per frame.
const ORBIT: &[u8] = include_bytes!("../../assets/orbit.gif");

/// The box every fit is demonstrated in: deliberately a different shape from the image.
const FRAME: f64 = 72.0;

pub fn body(_app: &mut App, _context: BuildContext) -> WidgetRef {
    screen(scrolling_body(vec![
        section_with_footer(
            "Animated GIF",
            "A 2.4-second loop, played from the GIF.",
            vec![showcase(
                Image::memory_static(ORBIT)
                    .width(256.0)
                    .height(112.0)
                    .fit(BoxFit::Contain),
            )],
        ),
        section_with_footer(
            "Its own size",
            "With no width or height the image takes the size it decoded to.",
            vec![showcase(Image::memory_static(SWATCH))],
        ),
        section_with_footer(
            "Fitted",
            "The same picture in a square box: contain, cover, and fill.",
            vec![showcase(
                Row::new()
                    .main_axis_alignment(MainAxisAlignment::SpaceEvenly)
                    .children([
                        framed(Image::memory_static(SWATCH).fit(BoxFit::Contain)),
                        framed(Image::memory_static(SWATCH).fit(BoxFit::Cover)),
                        framed(Image::memory_static(SWATCH).fit(BoxFit::Fill)),
                    ]),
            )],
        ),
        section_with_footer(
            "Placed",
            "Where an image sits when it does not fill its box.",
            vec![showcase(
                Row::new()
                    .main_axis_alignment(MainAxisAlignment::SpaceEvenly)
                    .children([
                        framed(Image::memory_static(BADGE).alignment(Alignment::TOP_LEFT)),
                        framed(Image::memory_static(BADGE).alignment(Alignment::CENTER)),
                        framed(Image::memory_static(BADGE).alignment(Alignment::BOTTOM_RIGHT)),
                    ]),
            )],
        ),
        section_with_footer(
            "Faded",
            "Opacity is applied as the image is drawn, not by a layer over it.",
            vec![showcase(
                Row::new()
                    .main_axis_alignment(MainAxisAlignment::SpaceEvenly)
                    .children([
                        framed(Image::memory_static(BADGE).opacity(1.0)),
                        framed(Image::memory_static(BADGE).opacity(0.5)),
                        framed(Image::memory_static(BADGE).opacity(0.2)),
                    ]),
            )],
        ),
        section_with_footer(
            "Tiled",
            "An image smaller than its box repeats to fill it.",
            vec![showcase(framed_wide(
                Image::memory_static(BADGE).repeat(ImageRepeat::Repeat),
            ))],
        ),
        section_with_footer(
            "Shared",
            "Both of these name the same bytes, so the picture is decoded once and held once.",
            vec![showcase(
                Row::new()
                    .main_axis_alignment(MainAxisAlignment::SpaceEvenly)
                    .children([
                        framed(Image::memory_static(SWATCH).fit(BoxFit::Contain)),
                        framed(Image::memory_static(SWATCH).fit(BoxFit::Contain)),
                    ]),
            )],
        ),
    ]))
}

/// A square of background with the image inside, so the box's edges are visible.
fn framed(image: Image) -> WidgetRef {
    SizedBox::new()
        .width(FRAME)
        .height(FRAME)
        .child(ColoredBox::new(CupertinoColors::SYSTEM_FILL).child(image))
        .into_widget()
}

fn framed_wide(image: Image) -> WidgetRef {
    SizedBox::new()
        .width(FRAME * 3.0)
        .height(FRAME)
        .child(ColoredBox::new(CupertinoColors::SYSTEM_FILL).child(image))
        .into_widget()
}
