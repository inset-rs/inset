//! The resources a Windows executable carries: the app's icon, which Explorer, the Start menu's and
//! the desktop's shortcuts and the taskbar show for the file, and Waft's host gives a window that
//! names no icon of its own. Written as a compiled resource file (`.res`), the form `rc.exe` makes,
//! which the MSVC linker and `lld-link` take as an input beside the objects, so no resource
//! compiler or build script is needed.
//!
//! The invariants a later editor will break:
//! - The icon's group is resource 1 of type `RT_GROUP_ICON`, which `waft-embedder-winit` loads by
//!   that id (`APP_ICON_RESOURCE`): change both together.
//! - A `.res` file starts with an empty entry, which tells the linker it is a 32-bit resource file,
//!   and every entry's header and data end on a four-byte boundary.

use image::RgbaImage;
use image::imageops::FilterType;

/// The icon group's resource id, which `waft-embedder-winit` loads the window icon by.
const APP_ICON: u16 = 1;

/// The sizes Windows asks an icon for: the title bar's and the taskbar's at each scale, Explorer's
/// views, and 256 for the largest, which is stored as a PNG; the others as bitmaps, which every
/// reader of icons takes.
const ICON_SIZES: [u32; 8] = [16, 20, 24, 32, 40, 48, 64, 256];

/// The largest size, stored as a PNG, as Windows' own icons store it.
const PNG_SIZE: u32 = 256;

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;

/// `MOVEABLE | DISCARDABLE`, and `PURE` besides for the group, the flags `rc.exe` gives icons.
const ICON_FLAGS: u16 = 0x1010;
const GROUP_FLAGS: u16 = 0x1030;

/// Language neutral, so the icon is found whatever the user's language.
const NEUTRAL: u16 = 0;

/// The compiled resource file of an executable whose icon is `icon`, scaled to each of
/// [`ICON_SIZES`].
pub fn icon_resources(icon: &RgbaImage) -> Vec<u8> {
    let images: Vec<(u32, Vec<u8>)> = ICON_SIZES
        .iter()
        .map(|&size| (size, icon_image(icon, size)))
        .collect();
    let mut file = Vec::new();
    write_entry(&mut file, 0, 0, 0, &[]);
    for (index, (_, data)) in images.iter().enumerate() {
        write_entry(&mut file, RT_ICON, image_id(index), ICON_FLAGS, data);
    }
    write_entry(
        &mut file,
        RT_GROUP_ICON,
        APP_ICON,
        GROUP_FLAGS,
        &group(&images),
    );
    file
}

/// The `RT_ICON` id of the image at `index` among the group's.
fn image_id(index: usize) -> u16 {
    u16::try_from(index + 1).expect("a few sizes")
}

/// One image of the icon, `size` pixels square: a PNG at [`PNG_SIZE`], a bitmap below it.
fn icon_image(icon: &RgbaImage, size: u32) -> Vec<u8> {
    let scaled = image::imageops::resize(icon, size, size, FilterType::Lanczos3);
    if size == PNG_SIZE {
        png(&scaled)
    } else {
        bitmap(&scaled)
    }
}

fn png(image: &RgbaImage) -> Vec<u8> {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, image::ImageFormat::Png)
        .expect("encoding a PNG in memory");
    bytes.into_inner()
}

/// An icon's bitmap: its header, which counts the colour rows and the mask's together as the
/// height, then its rows of BGRA from the bottom, then the mask, all clear, since the alpha says
/// what shows.
fn bitmap(image: &RgbaImage) -> Vec<u8> {
    let (width, height) = image.dimensions();
    let mask_row = width.div_ceil(32) * 4;
    let mut bytes = Vec::new();
    for value in [40, width, height * 2] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    let image_bytes = width * height * 4 + mask_row * height;
    for value in [0, image_bytes, 0, 0, 0, 0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for row in (0..height).rev() {
        for column in 0..width {
            let [red, green, blue, alpha] = image.get_pixel(column, row).0;
            bytes.extend_from_slice(&[blue, green, red, alpha]);
        }
    }
    bytes.resize(bytes.len() + (mask_row * height) as usize, 0);
    bytes
}

/// The group's directory: one entry per image, with its size, its bytes' count and its id.
fn group(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for value in [0u16, 1, u16::try_from(images.len()).expect("a few sizes")] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for (index, (size, data)) in images.iter().enumerate() {
        // A side of 256 is written as 0.
        let side = u8::try_from(*size).unwrap_or(0);
        bytes.extend_from_slice(&[side, side, 0, 0]);
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&32u16.to_le_bytes());
        let length = u32::try_from(data.len()).expect("an icon image under 4 GiB");
        bytes.extend_from_slice(&length.to_le_bytes());
        bytes.extend_from_slice(&image_id(index).to_le_bytes());
    }
    bytes
}

/// One entry: its header, the type and the name as ordinals, then `data`, padded to four bytes.
fn write_entry(file: &mut Vec<u8>, kind: u16, id: u16, flags: u16, data: &[u8]) {
    const HEADER_BYTES: u32 = 32;
    let length = u32::try_from(data.len()).expect("a resource under 4 GiB");
    file.extend_from_slice(&length.to_le_bytes());
    file.extend_from_slice(&HEADER_BYTES.to_le_bytes());
    for value in [0xFFFF, kind, 0xFFFF, id] {
        file.extend_from_slice(&value.to_le_bytes());
    }
    // The data's version, then the flags and the language, then the version and characteristics.
    file.extend_from_slice(&0u32.to_le_bytes());
    file.extend_from_slice(&flags.to_le_bytes());
    file.extend_from_slice(&NEUTRAL.to_le_bytes());
    file.extend_from_slice(&[0; 8]);
    file.extend_from_slice(data);
    file.resize(file.len().next_multiple_of(4), 0);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One entry of a `.res` file, as read back: its type, its id and its data.
    struct Entry {
        kind: u16,
        id: u16,
        data: Vec<u8>,
    }

    fn entries(mut file: &[u8]) -> Vec<Entry> {
        let word = |bytes: &[u8], at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
        let dword =
            |bytes: &[u8], at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        let mut read = Vec::new();
        while !file.is_empty() {
            let length = dword(file, 0) as usize;
            let header = dword(file, 4) as usize;
            assert_eq!(header, 32);
            assert_eq!((word(file, 8), word(file, 12)), (0xFFFF, 0xFFFF));
            read.push(Entry {
                kind: word(file, 10),
                id: word(file, 14),
                data: file[header..header + length].to_vec(),
            });
            file = &file[(header + length).next_multiple_of(4)..];
        }
        read
    }

    #[test]
    fn the_file_is_the_empty_entry_then_each_image_then_the_group_as_resource_one() {
        let icon = RgbaImage::from_pixel(64, 64, image::Rgba([10, 132, 255, 255]));
        let read = entries(&icon_resources(&icon));
        assert_eq!((read[0].kind, read[0].id, read[0].data.len()), (0, 0, 0));
        let images = &read[1..=ICON_SIZES.len()];
        assert!(images.iter().all(|entry| entry.kind == RT_ICON));
        let group = read.last().unwrap();
        assert_eq!((group.kind, group.id), (RT_GROUP_ICON, 1));
        assert_eq!(read.len(), ICON_SIZES.len() + 2);
        assert_eq!(group.data.len(), 6 + 14 * ICON_SIZES.len());
        for (index, (size, image)) in ICON_SIZES.iter().zip(images).enumerate() {
            let entry = &group.data[6 + 14 * index..6 + 14 * (index + 1)];
            assert_eq!(u32::from(entry[0]), size % 256);
            let length = u32::from_le_bytes(entry[8..12].try_into().unwrap());
            assert_eq!(length as usize, image.data.len());
            assert_eq!(u16::from_le_bytes([entry[12], entry[13]]), image.id);
        }
    }

    #[test]
    fn a_bitmap_is_bottom_up_bgra_with_a_clear_mask() {
        let mut icon = RgbaImage::from_pixel(16, 16, image::Rgba([0, 0, 0, 0]));
        icon.put_pixel(0, 0, image::Rgba([1, 2, 3, 4]));
        let bytes = bitmap(&icon);
        let height = i32::from_le_bytes(bytes[8..12].try_into().unwrap());
        assert_eq!(height, 32, "the colour rows and the mask's");
        let top_left = 40 + 15 * 16 * 4;
        assert_eq!(&bytes[top_left..top_left + 4], &[3, 2, 1, 4]);
        assert_eq!(bytes.len(), 40 + 16 * 16 * 4 + 4 * 16);
    }

    #[test]
    fn the_largest_image_is_a_png() {
        let icon = RgbaImage::from_pixel(512, 512, image::Rgba([255, 255, 255, 255]));
        assert!(icon_image(&icon, PNG_SIZE).starts_with(b"\x89PNG"));
    }
}
