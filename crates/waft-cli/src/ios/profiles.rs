//! The provisioning profiles Xcode stores, and the one that lets a bundle id of a team run on a
//! device.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use plist::Value;

/// What the app is signed for: its bundle id, of a team, and the device it is to run on, by its
/// UDID, when it is built for one.
pub(super) struct Signed<'a> {
    pub(super) bundle_id: &'a str,
    pub(super) team: &'a str,
    pub(super) device: Option<&'a str>,
}

pub(super) struct StoredProfile {
    pub(super) path: PathBuf,
    pub(super) name: String,
}

/// A profile Xcode has stored that lets `signed` run: unexpired, of its team, naming its device,
/// and naming its bundle id, or else matching it by a wildcard.
pub(super) fn stored_profile(signed: &Signed) -> Option<StoredProfile> {
    let mut wildcard = None;
    for path in stored_profile_paths() {
        let Some(decoded) = decode_profile(&path) else {
            continue;
        };
        if !decoded.allows(signed.team, signed.device) {
            continue;
        }
        let stored = StoredProfile {
            path,
            name: decoded.name.clone(),
        };
        match decoded.covers(signed.bundle_id) {
            Some(true) => return Some(stored),
            Some(false) => wildcard = Some(stored),
            None => {}
        }
    }
    wildcard
}

/// Every `.mobileprovision` in the folders Xcode stores profiles in: its own since Xcode 16, and
/// the one before.
fn stored_profile_paths() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };
    [
        home.join("Library/Developer/Xcode/UserData/Provisioning Profiles"),
        home.join("Library/MobileDevice/Provisioning Profiles"),
    ]
    .iter()
    .filter_map(|dir| fs::read_dir(dir).ok())
    .flat_map(|entries| entries.flatten().map(|entry| entry.path()))
    .filter(|path| path.extension().is_some_and(|e| e == "mobileprovision"))
    .collect()
}

struct DecodedProfile {
    name: String,
    teams: Vec<String>,
    /// `TEAM.bundle` or `TEAM.*`.
    app_id: String,
    expiration: Option<SystemTime>,
    /// The UDIDs of the devices it runs on, which a development profile lists.
    devices: Vec<String>,
    /// An enterprise profile's: it runs on any device.
    all_devices: bool,
}

impl DecodedProfile {
    /// Whether it is unexpired, of `team`, and runs on `device` where one is named.
    fn allows(&self, team: &str, device: Option<&str>) -> bool {
        let unexpired = self.expiration.is_none_or(|when| when > SystemTime::now());
        let of_team = self.teams.iter().any(|t| t == team);
        let on_device = device.is_none_or(|udid| {
            self.all_devices || self.devices.iter().any(|d| d.eq_ignore_ascii_case(udid))
        });
        unexpired && of_team && on_device
    }

    /// `Some(true)` for an exact match, `Some(false)` for a wildcard match, `None` otherwise.
    fn covers(&self, bundle_id: &str) -> Option<bool> {
        let (_, pattern) = self.app_id.split_once('.')?;
        if pattern == bundle_id {
            Some(true)
        } else if let Some(prefix) = pattern.strip_suffix('*')
            && bundle_id.starts_with(prefix)
        {
            Some(false)
        } else {
            None
        }
    }
}

fn decode_profile(path: &Path) -> Option<DecodedProfile> {
    let xml = Command::new("security")
        .args(["cms", "-D", "-i"])
        .arg(path)
        .output()
        .ok()?;
    let value: Value = plist::from_bytes(&xml.stdout).ok()?;
    let dict = value.as_dictionary()?;
    let entitlements = dict.get("Entitlements")?.as_dictionary()?;
    let strings = |key: &str| -> Vec<String> {
        dict.get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|t| t.as_string().map(str::to_owned))
            .collect()
    };
    Some(DecodedProfile {
        name: dict.get("Name")?.as_string()?.to_owned(),
        teams: strings("TeamIdentifier"),
        app_id: entitlements
            .get("application-identifier")?
            .as_string()?
            .to_owned(),
        expiration: dict
            .get("ExpirationDate")
            .and_then(Value::as_date)
            .map(SystemTime::from),
        devices: strings("ProvisionedDevices"),
        all_devices: dict
            .get("ProvisionsAllDevices")
            .and_then(Value::as_boolean)
            .unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> DecodedProfile {
        DecodedProfile {
            name: "p".into(),
            teams: vec!["T".into()],
            app_id: "T.com.example.*".into(),
            expiration: None,
            devices: vec!["00008160-0001".into()],
            all_devices: false,
        }
    }

    #[test]
    fn profile_coverage_distinguishes_exact_and_wildcard() {
        let profile = profile();
        assert_eq!(profile.covers("com.example.app"), Some(false));
        assert_eq!(profile.covers("org.other.app"), None);
        let exact = DecodedProfile {
            app_id: "T.com.example.app".into(),
            ..profile
        };
        assert_eq!(exact.covers("com.example.app"), Some(true));
    }

    #[test]
    fn a_profile_allows_its_team_on_the_devices_it_lists() {
        let profile = profile();
        assert!(profile.allows("T", Some("00008160-0001")));
        assert!(profile.allows("T", None));
        assert!(!profile.allows("T", Some("00008160-0002")));
        assert!(!profile.allows("U", Some("00008160-0001")));
        let enterprise = DecodedProfile {
            devices: Vec::new(),
            all_devices: true,
            ..profile
        };
        assert!(enterprise.allows("T", Some("00008160-0002")));
        let expired = DecodedProfile {
            expiration: Some(SystemTime::UNIX_EPOCH),
            ..enterprise
        };
        assert!(!expired.allows("T", None));
    }
}
