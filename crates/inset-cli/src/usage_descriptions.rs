//! What the app tells macOS and iOS about the devices it opens: why it uses each, in the
//! sentence the system's prompt shows the user, and on macOS the hardened runtime's leave to open
//! it.
//!
//! Both systems end an app that opens the camera or the microphone without a usage description
//! for it in its `Info.plist`; a macOS bundle signed with the hardened runtime, as cargo-packager
//! signs one, is also refused the device unless its entitlements allow it. Keep the two lists
//! describing the same devices.

use anyhow::{Result, bail};
use plist::{Dictionary, Value};

use crate::project::UsageDescriptions;

/// Refuses an empty usage description: the system's prompt would give the user no reason.
pub fn check(descriptions: &UsageDescriptions) -> Result<()> {
    for (device, description) in devices(descriptions) {
        if description.trim().is_empty() {
            bail!("the usage description for the {device} is empty; say why the app uses it");
        }
    }
    Ok(())
}

/// The `Info.plist` keys that say why the app uses each device it declares, for macOS and iOS
/// alike.
pub fn info_plist(descriptions: &UsageDescriptions) -> Dictionary {
    devices(descriptions)
        .map(|(device, description)| {
            (
                info_plist_key(device).to_owned(),
                Value::String(description.to_owned()),
            )
        })
        .collect()
}

/// The entitlements a macOS bundle signed with the hardened runtime needs to open each device it
/// declares.
pub fn macos_entitlements(descriptions: &UsageDescriptions) -> Dictionary {
    devices(descriptions)
        .map(|(device, _)| (entitlement(device).to_owned(), Value::Boolean(true)))
        .collect()
}

/// A device the system asks the user about before an app may open it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Device {
    Camera,
    Microphone,
}

impl std::fmt::Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Device::Camera => "camera",
            Device::Microphone => "microphone",
        })
    }
}

/// The devices the app declares, each with why it uses it.
fn devices(descriptions: &UsageDescriptions) -> impl Iterator<Item = (Device, &str)> {
    [
        (Device::Camera, &descriptions.camera),
        (Device::Microphone, &descriptions.microphone),
    ]
    .into_iter()
    .filter_map(|(device, description)| Some((device, description.as_deref()?)))
}

fn info_plist_key(device: Device) -> &'static str {
    match device {
        Device::Camera => "NSCameraUsageDescription",
        Device::Microphone => "NSMicrophoneUsageDescription",
    }
}

fn entitlement(device: Device) -> &'static str {
    match device {
        Device::Camera => "com.apple.security.device.camera",
        Device::Microphone => "com.apple.security.device.audio-input",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera_only() -> UsageDescriptions {
        serde_json::from_value(serde_json::json!({ "camera": "Takes the photo on your badge." }))
            .unwrap()
    }

    #[test]
    fn a_declared_device_gets_its_usage_description_and_no_other() {
        let info = info_plist(&camera_only());
        assert_eq!(
            info["NSCameraUsageDescription"].as_string(),
            Some("Takes the photo on your badge.")
        );
        assert!(!info.contains_key("NSMicrophoneUsageDescription"));
    }

    #[test]
    fn a_declared_device_is_allowed_by_the_hardened_runtime() {
        let entitlements = macos_entitlements(&UsageDescriptions {
            camera: Some("Takes photos.".into()),
            microphone: Some("Records sound with your videos.".into()),
        });
        assert_eq!(
            entitlements["com.apple.security.device.camera"].as_boolean(),
            Some(true)
        );
        assert_eq!(
            entitlements["com.apple.security.device.audio-input"].as_boolean(),
            Some(true)
        );
    }

    #[test]
    fn nothing_declared_adds_nothing() {
        let none = UsageDescriptions::default();
        assert!(info_plist(&none).is_empty());
        assert!(macos_entitlements(&none).is_empty());
        assert!(check(&none).is_ok());
    }

    #[test]
    fn an_empty_usage_description_is_refused() {
        let empty = UsageDescriptions {
            microphone: Some("  ".into()),
            ..UsageDescriptions::default()
        };
        assert!(check(&empty).is_err());
    }
}
