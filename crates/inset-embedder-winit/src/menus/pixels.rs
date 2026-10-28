//! An icon's pixels as each system lays them out, from the framework's RGBA, row by row with no
//! padding: Windows' 32-bit bitmaps, blue first, premultiplied for a menu item's bitmap and not
//! for an icon's colour; and StatusNotifierItem's pixmaps, alpha first in network byte order.

use inset_embedder::IconImage;

/// `image` as a Windows icon's colour bitmap: blue, green, red, alpha, not premultiplied.
#[cfg(any(target_os = "windows", test))]
pub(crate) fn bgra(image: &IconImage) -> Vec<u8> {
    pixels(image)
        .flat_map(|[red, green, blue, alpha]| [blue, green, red, alpha])
        .collect()
}

/// `image` as a Windows menu item's bitmap, which Windows blends as premultiplied.
#[cfg(any(target_os = "windows", test))]
pub(crate) fn bgra_premultiplied(image: &IconImage) -> Vec<u8> {
    pixels(image)
        .flat_map(|[red, green, blue, alpha]| {
            let times_alpha =
                |channel: u8| ((u16::from(channel) * u16::from(alpha) + 127) / 255) as u8;
            [
                times_alpha(blue),
                times_alpha(green),
                times_alpha(red),
                alpha,
            ]
        })
        .collect()
}

/// `image` as a StatusNotifierItem pixmap: alpha, red, green, blue, each pixel a big-endian
/// 32-bit number.
#[cfg(any(target_os = "linux", test))]
pub(crate) fn argb(image: &IconImage) -> Vec<u8> {
    pixels(image)
        .flat_map(|[red, green, blue, alpha]| [alpha, red, green, blue])
        .collect()
}

/// The image's pixels, as many as its size says; a short buffer gives fewer.
fn pixels(image: &IconImage) -> impl Iterator<Item = [u8; 4]> + '_ {
    let count = image.width as usize * image.height as usize;
    image.rgba.as_chunks::<4>().0.iter().take(count).copied()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn half_red() -> IconImage {
        IconImage {
            width: 1,
            height: 1,
            rgba: Arc::from([255u8, 0, 0, 128]),
            template: false,
        }
    }

    #[test]
    fn each_system_gets_the_pixels_in_its_order() {
        assert_eq!(bgra(&half_red()), [0, 0, 255, 128]);
        assert_eq!(bgra_premultiplied(&half_red()), [0, 0, 128, 128]);
        assert_eq!(argb(&half_red()), [128, 255, 0, 0]);
    }
}
