//! A disk image around a macOS app, from which a person drags the app into Applications: the app
//! and a link to `/Applications` side by side in a 600 by 400 window, and the app's icon as the
//! volume's.
//!
//! `hdiutil` makes the image. The window's layout is a `.DS_Store` written here, as dmgbuild
//! writes one, rather than Finder being driven to arrange it, so building opens no window and
//! needs no leave to control Finder, and builds the same in CI. That the volume has an icon of its
//! own is a Finder flag on the volume's root, which a `.DS_Store` does not hold and `hdiutil` does
//! not copy from a folder, so it is set on the image attached with no Finder window.
//!
//! The app's extension is left as Finder shows it, which for an app is hidden unless the user
//! asks to see every extension: hiding it is a Finder flag on the bundle's folder, which
//! `codesign --verify --strict` refuses on a signed app, there and wherever it is copied to.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use ds_parser::{
    BrowserWindowSettings, FieldKey, FinderViewStyle, IconLocation, IconViewProperties, Record,
    WindowBounds,
};

use crate::project::Project;
use crate::tools;

use super::ICON_FILE;

/// The window's size, in points.
const WINDOW_SIZE: (f64, f64) = (600.0, 400.0);
/// Where the window opens, in Finder's coordinates.
const WINDOW_ORIGIN: (f64, f64) = (100.0, 100.0);
/// The centres of the app's icon and of the Applications link, in the window.
const APP_AT: (u32, u32) = (180, 170);
const APPLICATIONS_AT: (u32, u32) = (480, 170);
const ICON_SIZE: f64 = 128.0;
const TEXT_SIZE: f64 = 16.0;

/// Finder's flag that a folder, or a volume's root, has an icon of its own (`kHasCustomIcon`).
const HAS_CUSTOM_ICON: u16 = 0x0400;

/// Makes the disk image around `app` in `out`: `<Name>_<version>_<arch>.dmg`, compressed.
pub fn make(project: &Project, app: &Path, out: &Path) -> Result<PathBuf> {
    let app_name = app
        .file_name()
        .context("the app has no name")?
        .to_string_lossy()
        .into_owned();
    let staging = out.join("staging");
    let has_icon = stage(app, &app_name, &staging)?;
    let writable = out.join("writable.dmg");
    tools::run(
        Command::new("hdiutil")
            .args(["create", "-quiet", "-volname", &project.name, "-srcfolder"])
            .arg(&staging)
            .args(["-fs", "HFS+", "-format", "UDRW", "-ov"])
            .arg(&writable),
        "hdiutil create",
    )?;
    if has_icon {
        give_the_volume_its_icon(&writable, &out.join("mount"))?;
    }
    let dmg = out.join(file_name(project));
    tools::run(
        Command::new("hdiutil")
            .args(["convert", "-quiet"])
            .arg(&writable)
            .args(["-format", "UDZO", "-imagekey", "zlib-level=9", "-ov", "-o"])
            .arg(&dmg),
        "hdiutil convert",
    )?;
    fs::remove_file(&writable)?;
    fs::remove_dir_all(&staging)?;
    Ok(dmg)
}

/// Puts what the volume holds in `staging`: the app, the link to `/Applications`, the volume's
/// icon and the window's layout. Answers whether the volume has an icon of its own.
fn stage(app: &Path, app_name: &str, staging: &Path) -> Result<bool> {
    if staging.exists() {
        fs::remove_dir_all(staging)?;
    }
    fs::create_dir_all(staging)?;
    tools::run(
        Command::new("ditto").arg(app).arg(staging.join(app_name)),
        "ditto",
    )?;
    symlink("/Applications", staging.join("Applications"))?;
    let icon = app.join("Contents/Resources").join(ICON_FILE);
    let has_icon = icon.exists();
    if has_icon {
        fs::copy(&icon, staging.join(".VolumeIcon.icns"))?;
    }
    let layout =
        ds_parser::write(&layout(app_name)).context("could not write the window's layout")?;
    fs::write(staging.join(".DS_Store"), layout)?;
    Ok(has_icon)
}

/// Finder's settings for the image's window: icon view with no toolbar, sidebar or status bar,
/// the app on the left and the Applications link on the right.
fn layout(app_name: &str) -> Vec<Record> {
    let window = BrowserWindowSettings::builder()
        .window_bounds(WindowBounds {
            x: WINDOW_ORIGIN.0,
            y: WINDOW_ORIGIN.1,
            width: WINDOW_SIZE.0,
            height: WINDOW_SIZE.1,
        })
        .show_sidebar(false)
        .show_toolbar(false)
        .show_status_bar(false)
        .show_path_bar(false)
        .show_tab_view(false)
        .container_show_sidebar(false)
        .sidebar_width(0.0)
        .build();
    let icons = IconViewProperties::builder()
        .arrange_by("none")
        .background_type(0)
        .background_color((1.0, 1.0, 1.0))
        .grid_offset((0.0, 0.0))
        .grid_spacing(100.0)
        .icon_size(ICON_SIZE)
        .text_size(TEXT_SIZE)
        .label_on_bottom(true)
        .show_icon_preview(true)
        .show_item_info(false)
        .view_options_version(1)
        .build();
    vec![
        Record::new(".", FieldKey::vSrn, 1_u32),
        Record::new(".", FieldKey::bwsp, window),
        Record::new(".", FieldKey::icvp, icons),
        Record::new(".", FieldKey::vstl, FinderViewStyle::Icon),
        Record::new(
            app_name,
            FieldKey::Iloc,
            IconLocation::new(APP_AT.0, APP_AT.1),
        ),
        Record::new(
            "Applications",
            FieldKey::Iloc,
            IconLocation::new(APPLICATIONS_AT.0, APPLICATIONS_AT.1),
        ),
    ]
}

/// Has the volume of the image `writable` show its `.VolumeIcon.icns`: the custom icon flag set
/// in the Finder information of its root, attached at `mount_point` with no Finder window.
fn give_the_volume_its_icon(writable: &Path, mount_point: &Path) -> Result<()> {
    let mounted = Mounted::attach(writable, mount_point)?;
    tools::run(
        Command::new("xattr")
            .args(["-wx", "com.apple.FinderInfo", &finder_info(HAS_CUSTOM_ICON)])
            .arg(mount_point),
        "xattr",
    )?;
    mounted.detach()?;
    fs::remove_dir(mount_point)?;
    Ok(())
}

/// The 32 bytes of a folder's Finder information, in hex, with `flag` set at bytes 8 and 9 and
/// nothing else.
fn finder_info(flag: u16) -> String {
    let mut bytes = [0u8; 32];
    bytes[8..10].copy_from_slice(&flag.to_be_bytes());
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// `<Name>_<version>_<arch>.dmg`, the name cargo-packager gave the image.
fn file_name(project: &Project) -> String {
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        other => other,
    };
    format!("{}_{}_{arch}.dmg", project.name, project.version)
}

/// An image attached with no Finder window, detached when this goes, if nothing detached it.
struct Mounted {
    mount_point: PathBuf,
    attached: bool,
}

impl Mounted {
    fn attach(image: &Path, mount_point: &Path) -> Result<Mounted> {
        fs::create_dir_all(mount_point)?;
        tools::run(
            Command::new("hdiutil")
                .args([
                    "attach",
                    "-quiet",
                    "-nobrowse",
                    "-noverify",
                    "-noautoopen",
                    "-mountpoint",
                ])
                .arg(mount_point)
                .arg(image),
            "hdiutil attach",
        )?;
        Ok(Mounted {
            mount_point: mount_point.to_path_buf(),
            attached: true,
        })
    }

    fn detach(mut self) -> Result<()> {
        self.attached = false;
        tools::run(
            Command::new("hdiutil")
                .args(["detach", "-quiet"])
                .arg(&self.mount_point),
            "hdiutil detach",
        )
    }
}

impl Drop for Mounted {
    fn drop(&mut self) {
        if self.attached {
            let _ = Command::new("hdiutil")
                .args(["detach", "-force", "-quiet"])
                .arg(&self.mount_point)
                .status();
        }
    }
}

#[cfg(test)]
mod tests {
    use ds_parser::Value;

    use super::*;

    #[test]
    fn the_layout_puts_the_app_left_of_the_applications_link_in_icon_view() {
        let store = ds_parser::parse(&ds_parser::write(&layout("My App.app")).unwrap()).unwrap();
        let location = |name: &str| {
            store
                .records
                .iter()
                .find_map(|record| match &record.value {
                    Value::IconLocation(at) if record.name == name => Some((at.x, at.y)),
                    _ => None,
                })
                .unwrap()
        };
        assert_eq!(location("My App.app"), APP_AT);
        assert_eq!(location("Applications"), APPLICATIONS_AT);
        assert!(store.records.iter().any(|record| record.name == "."
            && matches!(record.value, Value::FinderViewStyle(FinderViewStyle::Icon))));
        let Some(Value::BrowserWindowSettings(window)) = store
            .records
            .iter()
            .find(|record| record.field == FieldKey::bwsp)
            .map(|record| &record.value)
        else {
            panic!("no window settings");
        };
        let bounds = window.window_bounds.unwrap();
        assert_eq!((bounds.width, bounds.height), WINDOW_SIZE);
        assert_eq!(window.show_toolbar, Some(false));
    }

    #[test]
    fn finder_information_carries_its_flag_at_bytes_eight_and_nine() {
        let info = finder_info(HAS_CUSTOM_ICON);
        assert_eq!(info.len(), 64);
        assert_eq!(&info[16..20], "0400");
        assert!(info[..16].chars().all(|c| c == '0') && info[20..].chars().all(|c| c == '0'));
    }
}
