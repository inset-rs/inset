//! Notarization: Apple's check of a signed app or disk image, through `notarytool`, and the ticket
//! it issues stapled on, so that Gatekeeper opens it without asking Apple again.
//!
//! The credentials come from the environment, the first complete set of three: a profile
//! `notarytool store-credentials` keeps in the keychain (`APPLE_KEYCHAIN_PROFILE`); an Apple ID
//! with an app-specific password and its team (`APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`); or
//! an App Store Connect key (`APPLE_API_KEY`, `APPLE_API_ISSUER`, `APPLE_API_KEY_PATH`). A set
//! named in part is an error rather than skipped, so a build meant for distribution is not handed
//! out unnotarized by a typo.

use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::tools;

/// Who `notarytool` submits as: its arguments that name the credentials.
pub struct Notarization {
    credentials: Vec<OsString>,
}

/// What `notarytool submit --output-format json` answers.
#[derive(Deserialize)]
struct Submission {
    id: String,
    status: String,
    #[serde(default)]
    message: String,
}

impl Notarization {
    /// The credentials the environment holds; `None` where it holds none.
    pub fn from_env() -> Result<Option<Notarization>> {
        let credentials = credentials(|name| std::env::var_os(name))?;
        Ok(credentials.map(|credentials| Notarization { credentials }))
    }

    /// Notarizes `app`, sent as a zip `ditto` makes beside it, and staples the ticket to it.
    pub fn notarize_app(&self, app: &Path) -> Result<()> {
        let zip = app.with_extension("zip");
        tools::run(
            Command::new("ditto")
                .args(["-c", "-k", "--keepParent"])
                .arg(app)
                .arg(&zip),
            "ditto",
        )?;
        let submitted = self.submit(&zip);
        fs::remove_file(&zip)?;
        submitted?;
        staple(app)
    }

    /// Notarizes `dmg`, which is sent as it is, and staples the ticket to it.
    pub fn notarize_disk_image(&self, dmg: &Path) -> Result<()> {
        self.submit(dmg)?;
        staple(dmg)
    }

    /// Sends `file` to Apple and waits for the verdict; a rejection fails with Apple's log.
    fn submit(&self, file: &Path) -> Result<()> {
        println!("notarizing {}", file.display());
        let answer = tools::output(
            Command::new("xcrun")
                .args(["notarytool", "submit"])
                .arg(file)
                .args(["--wait", "--output-format", "json"])
                .args(&self.credentials),
            "notarytool submit",
        )?;
        let submission: Submission =
            serde_json::from_str(&answer).context("notarytool answered in an unexpected shape")?;
        if submission.status == "Accepted" {
            return Ok(());
        }
        let log = tools::output(
            Command::new("xcrun")
                .args(["notarytool", "log", &submission.id])
                .args(&self.credentials),
            "notarytool log",
        )
        .unwrap_or_default();
        bail!(
            "notarization of {} ended {}: {}\n{log}",
            file.display(),
            submission.status,
            submission.message
        );
    }
}

/// Staples the ticket Apple issued to `path`.
fn staple(path: &Path) -> Result<()> {
    tools::run(
        Command::new("xcrun").args(["stapler", "staple"]).arg(path),
        "stapler",
    )
}

/// `notarytool`'s arguments for the first complete set of credentials `lookup` finds, in the
/// order the module says; `None` where it finds none.
fn credentials(lookup: impl Fn(&str) -> Option<OsString>) -> Result<Option<Vec<OsString>>> {
    if let Some(profile) = lookup("APPLE_KEYCHAIN_PROFILE") {
        return Ok(Some(vec!["--keychain-profile".into(), profile]));
    }
    let sets: [[(&str, &str); 3]; 2] = [
        [
            ("APPLE_ID", "--apple-id"),
            ("APPLE_PASSWORD", "--password"),
            ("APPLE_TEAM_ID", "--team-id"),
        ],
        [
            ("APPLE_API_KEY", "--key-id"),
            ("APPLE_API_ISSUER", "--issuer"),
            ("APPLE_API_KEY_PATH", "--key"),
        ],
    ];
    for set in sets {
        let values: Vec<Option<OsString>> = set.iter().map(|(name, _)| lookup(name)).collect();
        if values.iter().all(Option::is_none) {
            continue;
        }
        let missing: Vec<&str> = set
            .iter()
            .zip(&values)
            .filter(|(_, value)| value.is_none())
            .map(|((name, _), _)| *name)
            .collect();
        if !missing.is_empty() {
            bail!("notarizing also needs {}", missing.join(" and "));
        }
        let arguments = set
            .iter()
            .zip(values)
            .flat_map(|((_, flag), value)| [OsString::from(flag), value.unwrap_or_default()])
            .collect();
        return Ok(Some(arguments));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment(set: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let set: Vec<(String, String)> = set
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect();
        move |name| {
            set.iter()
                .find(|(set_name, _)| set_name == name)
                .map(|(_, value)| value.into())
        }
    }

    #[test]
    fn no_credentials_mean_no_notarization() {
        assert!(credentials(environment(&[])).unwrap().is_none());
    }

    #[test]
    fn a_keychain_profile_comes_first() {
        let found = credentials(environment(&[
            ("APPLE_KEYCHAIN_PROFILE", "inset"),
            ("APPLE_ID", "jane@example.com"),
        ]))
        .unwrap()
        .unwrap();
        assert_eq!(found, ["--keychain-profile", "inset"]);
    }

    #[test]
    fn an_app_store_connect_key_names_its_file_issuer_and_id() {
        let found = credentials(environment(&[
            ("APPLE_API_KEY", "KEY123"),
            ("APPLE_API_ISSUER", "issuer"),
            ("APPLE_API_KEY_PATH", "/keys/AuthKey.p8"),
        ]))
        .unwrap()
        .unwrap();
        assert_eq!(
            found,
            [
                "--key-id",
                "KEY123",
                "--issuer",
                "issuer",
                "--key",
                "/keys/AuthKey.p8"
            ]
        );
    }

    #[test]
    fn a_set_named_in_part_is_an_error() {
        let error = credentials(environment(&[("APPLE_ID", "jane@example.com")])).unwrap_err();
        assert_eq!(
            error.to_string(),
            "notarizing also needs APPLE_PASSWORD and APPLE_TEAM_ID"
        );
    }
}
