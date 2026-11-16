//! The provisioning profiles Xcode stores, and the one that lets a bundle id of a team run.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use anyhow::{Context, Result};
use plist::Value;

pub(super) struct StoredProfile {
    pub(super) path: PathBuf,
    pub(super) name: String,
}

/// A provisioning profile Xcode has stored that covers the bundle id for the team.
pub(super) fn provisioning_profile(bundle_id: &str, team: &str) -> Result<StoredProfile> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")?;
    let dirs = [
        home.join("Library/Developer/Xcode/UserData/Provisioning Profiles"),
        home.join("Library/MobileDevice/Provisioning Profiles"),
    ];
    let mut wildcard = None;
    for dir in dirs {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "mobileprovision") {
                continue;
            }
            let Some(decoded) = decode_profile(&path) else {
                continue;
            };
            if !decoded.teams.iter().any(|t| t == team) || decoded.expired() {
                continue;
            }
            match decoded.covers(bundle_id) {
                Some(true) => {
                    return Ok(StoredProfile {
                        path,
                        name: decoded.name,
                    });
                }
                Some(false) => {
                    wildcard = Some(StoredProfile {
                        path,
                        name: decoded.name,
                    })
                }
                None => {}
            }
        }
    }
    wildcard.with_context(|| {
        format!(
            "no provisioning profile for {bundle_id} (team {team}); run the app once from Xcode with automatic signing, or download one from developer.apple.com"
        )
    })
}

struct DecodedProfile {
    name: String,
    teams: Vec<String>,
    /// `TEAM.bundle` or `TEAM.*`.
    app_id: String,
    expiration: Option<SystemTime>,
}

impl DecodedProfile {
    fn expired(&self) -> bool {
        self.expiration.is_some_and(|when| when < SystemTime::now())
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
    Some(DecodedProfile {
        name: dict.get("Name")?.as_string()?.to_owned(),
        teams: dict
            .get("TeamIdentifier")?
            .as_array()?
            .iter()
            .filter_map(|t| t.as_string().map(str::to_owned))
            .collect(),
        app_id: entitlements
            .get("application-identifier")?
            .as_string()?
            .to_owned(),
        expiration: dict
            .get("ExpirationDate")
            .and_then(Value::as_date)
            .map(SystemTime::from),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_coverage_distinguishes_exact_and_wildcard() {
        let profile = DecodedProfile {
            name: "p".into(),
            teams: vec!["T".into()],
            app_id: "T.com.example.*".into(),
            expiration: None,
        };
        assert_eq!(profile.covers("com.example.app"), Some(false));
        assert_eq!(profile.covers("org.other.app"), None);
        let exact = DecodedProfile {
            app_id: "T.com.example.app".into(),
            ..profile
        };
        assert_eq!(exact.covers("com.example.app"), Some(true));
    }
}
