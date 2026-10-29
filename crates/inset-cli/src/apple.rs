//! What an app bundle is on Apple's systems, macOS's and iOS's alike: the keys every bundle's
//! `Info.plist` has, the identities the keychain holds to sign one with, and signing with
//! `codesign`.

use std::path::Path;
use std::process::Command;

use anyhow::{Result, bail};
use plist::{Dictionary, Value};

use crate::project::Project;
use crate::tools;
use crate::usage_descriptions;

/// A code signing identity in the keychain, as `security find-identity` lists it.
pub struct Identity {
    pub hash: String,
    pub name: String,
    pub team: String,
}

/// What a bundle is signed with: ad hoc, which needs no certificate and vouches for nothing but
/// the bundle's own contents, or an identity in the keychain.
pub enum SigningIdentity {
    AdHoc,
    Keychain(Identity),
}

/// How one piece of code is signed, in rcodesign's terms.
pub struct SigningSettings<'a> {
    pub identity: &'a SigningIdentity,
    /// The hardened runtime, which notarization requires: the system then refuses what the
    /// entitlements do not allow.
    pub hardened_runtime: bool,
    pub entitlements: Option<&'a Path>,
    /// A secure timestamp from Apple's server, which notarization requires. Without one the
    /// signature is made offline.
    pub timestamp: bool,
}

/// The keys every Apple bundle's `Info.plist` has: its identifier, names, version and executable,
/// and why it uses each device the system asks the user about.
pub fn info_plist(project: &Project) -> Dictionary {
    let mut info = Dictionary::new();
    let text = |s: &str| Value::String(s.to_owned());
    info.insert("CFBundleDevelopmentRegion".into(), text("en"));
    info.insert("CFBundleExecutable".into(), text(&project.bin));
    info.insert("CFBundleIdentifier".into(), text(&project.identifier));
    info.insert("CFBundleInfoDictionaryVersion".into(), text("6.0"));
    info.insert("CFBundleName".into(), text(&project.name));
    info.insert("CFBundleDisplayName".into(), text(&project.name));
    info.insert("CFBundlePackageType".into(), text("APPL"));
    info.insert("CFBundleShortVersionString".into(), text(&project.version));
    info.insert("CFBundleVersion".into(), text(&project.version));
    info.extend(usage_descriptions::info_plist(&project.usage_descriptions));
    info
}

/// The keychain's Apple Development certificate, of `team` when a machine has several.
pub fn development_identity(team: Option<&str>) -> Result<Identity> {
    let development = keychain_identities()?
        .into_iter()
        .filter(|i| {
            i.name.starts_with("Apple Development") || i.name.starts_with("iPhone Developer")
        })
        .find(|i| team.is_none_or(|team| i.team == team));
    match development {
        Some(identity) => Ok(identity),
        None => bail!(
            "no Apple Development certificate{}; sign in to Xcode once (Settings > Accounts) to create one",
            team.map(|t| format!(" for team {t}")).unwrap_or_default()
        ),
    }
}

/// Signs `path`, a bundle, an executable, a library or a disk image, replacing any signature it
/// has.
pub fn sign(path: &Path, settings: &SigningSettings) -> Result<()> {
    let mut command = Command::new("codesign");
    command.arg("--force").arg("--sign");
    match settings.identity {
        SigningIdentity::AdHoc => command.arg("-"),
        SigningIdentity::Keychain(identity) => command.arg(&identity.hash),
    };
    if settings.hardened_runtime {
        command.args(["--options", "runtime"]);
    }
    if let Some(entitlements) = settings.entitlements {
        command
            .arg("--entitlements")
            .arg(entitlements)
            .arg("--generate-entitlement-der");
    }
    command.arg(if settings.timestamp {
        "--timestamp"
    } else {
        "--timestamp=none"
    });
    tools::run(command.arg(path), "codesign")
}

/// Every valid code signing identity in the keychain.
fn keychain_identities() -> Result<Vec<Identity>> {
    let listing = tools::output(
        Command::new("security").args(["find-identity", "-v", "-p", "codesigning"]),
        "security find-identity",
    )?;
    Ok(listing.lines().filter_map(parse_identity).collect())
}

/// `  1) 0123…ABCD "Apple Development: Jane Doe (TEAMID)"`.
fn parse_identity(line: &str) -> Option<Identity> {
    let (hash, rest) = line.trim().split_once(") ")?.1.split_once(' ')?;
    let name = rest.trim().trim_matches('"').to_owned();
    let team = name.rsplit_once('(')?.1.trim_end_matches(')').to_owned();
    Some(Identity {
        hash: hash.to_owned(),
        name,
        team,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_line_yields_hash_name_and_team() {
        let identity =
            parse_identity(r#"  1) ABCDEF0123456789ABCDEF0123456789ABCDEF01 "Apple Development: Jane Doe (TEAM12345)""#)
                .unwrap();
        assert_eq!(identity.hash, "ABCDEF0123456789ABCDEF0123456789ABCDEF01");
        assert_eq!(identity.name, "Apple Development: Jane Doe (TEAM12345)");
        assert_eq!(identity.team, "TEAM12345");
    }
}
