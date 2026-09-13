//! Icon sizes derived from the app's one PNG.

use std::path::Path;

use anyhow::{Context, Result};
use image::RgbaImage;
use image::imageops::FilterType;

pub fn load(path: &Path) -> Result<RgbaImage> {
    let image = image::open(path).with_context(|| format!("reading icon {}", path.display()))?;
    Ok(image.to_rgba8())
}

pub fn write_resized(source: &RgbaImage, size: u32, dest: &Path) -> Result<()> {
    let resized = image::imageops::resize(source, size, size, FilterType::Lanczos3);
    resized
        .save(dest)
        .with_context(|| format!("writing {}", dest.display()))?;
    Ok(())
}

/// A blue tile with a white disc: the icon a new app starts with.
pub fn placeholder() -> RgbaImage {
    const SIZE: u32 = 1024;
    let center = SIZE as f32 / 2.0;
    let radius = 300.0;
    RgbaImage::from_fn(SIZE, SIZE, |x, y| {
        let dx = x as f32 + 0.5 - center;
        let dy = y as f32 + 0.5 - center;
        let coverage = (radius + 0.5 - (dx * dx + dy * dy).sqrt()).clamp(0.0, 1.0);
        let mix =
            |background: u8| (background as f32 + (255.0 - background as f32) * coverage) as u8;
        image::Rgba([mix(10), mix(132), mix(255), 255])
    })
}
