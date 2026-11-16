//! The launcher icons, at every density: the app's one icon, or the two layers of its adaptive icon
//! with the XML that names them.

use std::fs;
use std::path::Path;

use anyhow::{Result, bail};

use crate::icons;
use crate::project::{AdaptiveIcon, AdaptiveIconBackground, Project};

/// The densities a launcher icon is written at, with each one's scale from 1 dp to pixels.
const DENSITIES: [(&str, f64); 5] = [
    ("mdpi", 1.0),
    ("hdpi", 1.5),
    ("xhdpi", 2.0),
    ("xxhdpi", 3.0),
    ("xxxhdpi", 4.0),
];

/// A launcher icon's side in dp, its whole picture: what a launcher shows of a plain icon.
const LAUNCHER_ICON_DP: f64 = 48.0;

/// An adaptive icon's layer's side in dp, of which the launcher shows the middle 72.
const ADAPTIVE_LAYER_DP: f64 = 108.0;

/// The launcher icon `@mipmap/ic_launcher`: the app's one PNG at each density, and over it, on
/// API 26 and later, the adaptive icon where the app gives one. Answers whether there is either.
pub(super) fn write_icons(project: &Project, res: &Path) -> Result<bool> {
    if let Some(icon) = &project.icon {
        write_at_densities(
            &icons::load(icon)?,
            LAUNCHER_ICON_DP,
            res,
            "ic_launcher.png",
        )?;
    }
    if let Some(adaptive) = &project.android.adaptive_icon {
        write_adaptive_icon(adaptive, res)?;
    }
    Ok(project.icon.is_some() || project.android.adaptive_icon.is_some())
}

/// `mipmap-anydpi-v26/ic_launcher.xml`, an `<adaptive-icon>` of the two layers, each written at
/// every density, or the background as a colour resource.
fn write_adaptive_icon(icon: &AdaptiveIcon, res: &Path) -> Result<()> {
    let foreground = square_icon(&icon.foreground)?;
    write_at_densities(
        &foreground,
        ADAPTIVE_LAYER_DP,
        res,
        "ic_launcher_foreground.png",
    )?;
    let background = match &icon.background {
        AdaptiveIconBackground::Color(color) => {
            let values = res.join("values");
            fs::create_dir_all(&values)?;
            fs::write(
                values.join("ic_launcher_background.xml"),
                format!(
                    "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<resources>\n  <color name=\"ic_launcher_background\">{color}</color>\n</resources>\n"
                ),
            )?;
            "@color/ic_launcher_background"
        }
        AdaptiveIconBackground::Image(image) => {
            let background = square_icon(image)?;
            write_at_densities(
                &background,
                ADAPTIVE_LAYER_DP,
                res,
                "ic_launcher_background.png",
            )?;
            "@mipmap/ic_launcher_background"
        }
    };
    let anydpi = res.join("mipmap-anydpi-v26");
    fs::create_dir_all(&anydpi)?;
    fs::write(
        anydpi.join("ic_launcher.xml"),
        format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<adaptive-icon xmlns:android=\"http://schemas.android.com/apk/res/android\">\n  <background android:drawable=\"{background}\" />\n  <foreground android:drawable=\"@mipmap/ic_launcher_foreground\" />\n</adaptive-icon>\n"
        ),
    )?;
    Ok(())
}

/// The PNG at `path`, refused unless it is square.
fn square_icon(path: &Path) -> Result<image::RgbaImage> {
    let image = icons::load(path)?;
    if image.width() != image.height() {
        bail!(
            "{} is {}x{} px; an adaptive icon's layer is square",
            path.display(),
            image.width(),
            image.height()
        );
    }
    Ok(image)
}

/// `source` scaled to `dp` at each density, as `mipmap-<density>/<name>`.
fn write_at_densities(source: &image::RgbaImage, dp: f64, res: &Path, name: &str) -> Result<()> {
    for (density, scale) in DENSITIES {
        let dir = res.join(format!("mipmap-{density}"));
        fs::create_dir_all(&dir)?;
        icons::write_resized(source, (dp * scale).round() as u32, &dir.join(name))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_adaptive_icon_is_its_layers_at_every_density_and_its_xml() {
        let dir = std::env::temp_dir().join(format!("waft-adaptive-icon-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let foreground = dir.join("foreground.png");
        image::RgbaImage::new(432, 432).save(&foreground).unwrap();
        let res = dir.join("res");
        let icon = AdaptiveIcon {
            foreground,
            background: AdaptiveIconBackground::Color("#ffffff".to_owned()),
        };
        write_adaptive_icon(&icon, &res).unwrap();
        let xxxhdpi = image::open(res.join("mipmap-xxxhdpi/ic_launcher_foreground.png")).unwrap();
        let mdpi = image::open(res.join("mipmap-mdpi/ic_launcher_foreground.png")).unwrap();
        assert_eq!((xxxhdpi.width(), mdpi.width()), (432, 108));
        let xml = fs::read_to_string(res.join("mipmap-anydpi-v26/ic_launcher.xml")).unwrap();
        assert!(xml.contains(r#"<background android:drawable="@color/ic_launcher_background" />"#));
        assert!(
            xml.contains(r#"<foreground android:drawable="@mipmap/ic_launcher_foreground" />"#)
        );
        let color = fs::read_to_string(res.join("values/ic_launcher_background.xml")).unwrap();
        assert!(color.contains(r#"<color name="ic_launcher_background">#ffffff</color>"#));
        fs::remove_dir_all(&dir).unwrap();
    }
}
