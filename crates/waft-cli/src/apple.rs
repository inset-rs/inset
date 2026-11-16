//! What an app bundle is on Apple's systems, macOS's and iOS's alike: the keys every bundle's
//! `Info.plist` has, the identities the keychain holds to sign one with, and signing with
//! `codesign`.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Result, bail};
use plist::{Dictionary, Value};

use crate::project::Project;
use crate::tools;
use crate::usage_descriptions;

/// A code signing identity in the keychain, as `security find-identity` lists it, with the team
/// its certificate is issued to.
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

/// The team the app is signed for: `team` where the metadata names it, else the team of the
/// keychain's Apple Development or Apple Distribution certificate.
pub fn team(team: Option<&str>) -> Result<String> {
    if let Some(team) = team {
        return Ok(team.to_owned());
    }
    let identity = keychain_identities()?.into_iter().find(|i| {
        [
            "Apple Development",
            "Apple Distribution",
            "iPhone Developer",
            "iPhone Distribution",
        ]
        .iter()
        .any(|kind| i.name.starts_with(kind))
    });
    match identity {
        Some(identity) => Ok(identity.team),
        None => bail!(
            "no Apple team: name it as `team` in the metadata, or sign in to Xcode once (Settings > Accounts) to create a certificate"
        ),
    }
}

/// The keychain identity `named` names, as `codesign --sign` takes it: its hash, or its name or
/// a part of it that only it has.
pub fn keychain_identity(named: &str) -> Result<Identity> {
    let mut matching: Vec<Identity> = keychain_identities()?
        .into_iter()
        .filter(|i| i.hash.eq_ignore_ascii_case(named) || i.name.contains(named))
        .collect();
    match matching.len() {
        1 => Ok(matching.remove(0)),
        0 => bail!("no signing identity in the keychain matches `{named}`"),
        _ => bail!(
            "several signing identities match `{named}`: {}",
            matching
                .iter()
                .map(|i| i.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
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

/// Every valid code signing identity in the keychain whose certificate names its team.
fn keychain_identities() -> Result<Vec<Identity>> {
    let listing = tools::output(
        Command::new("security").args(["find-identity", "-v", "-p", "codesigning"]),
        "security find-identity",
    )?;
    Ok(listing
        .lines()
        .filter_map(parse_identity)
        .filter_map(|(hash, name)| {
            let team = certificate_team(&hash, &name)?;
            Some(Identity { hash, name, team })
        })
        .collect())
}

/// The hash and the name of `  1) 0123…ABCD "Apple Development: Jane Doe (A1B2C3D4E5)"`.
fn parse_identity(line: &str) -> Option<(String, String)> {
    let (hash, rest) = line.trim().split_once(") ")?.1.split_once(' ')?;
    Some((hash.to_owned(), rest.trim().trim_matches('"').to_owned()))
}

/// The team the certificate of the identity `hash` is issued to: its subject's organizational
/// unit, as Flutter's tool reads it. The id in parentheses that ends the certificate's name is its
/// holder's, which is not the team's.
fn certificate_team(hash: &str, name: &str) -> Option<String> {
    let listing = Command::new("security")
        .args(["find-certificate", "-a", "-Z", "-p", "-c", name])
        .output()
        .ok()?;
    let listing = String::from_utf8(listing.stdout).ok()?;
    let subject = certificate_subject(certificate_pem(&listing, hash)?)?;
    subject_team(&subject)
}

/// The PEM of the certificate `hash` names in `security find-certificate -Z -p`'s listing, each
/// certificate's hashes before its PEM.
fn certificate_pem<'a>(listing: &'a str, hash: &str) -> Option<&'a str> {
    let marker = format!("SHA-1 hash: {}", hash.to_uppercase());
    let after = &listing[listing.find(&marker)?..];
    let start = after.find("-----BEGIN CERTIFICATE-----")?;
    let end_marker = "-----END CERTIFICATE-----";
    let end = after.find(end_marker)? + end_marker.len();
    after.get(start..end)
}

/// `openssl x509 -subject` of `pem`.
fn certificate_subject(pem: &str) -> Option<String> {
    let mut openssl = Command::new("openssl")
        .args(["x509", "-noout", "-subject"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    openssl.stdin.take()?.write_all(pem.as_bytes()).ok()?;
    let output = openssl.wait_with_output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The organizational unit of `subject= /UID=…/CN=…/OU=A1B2C3D4E5/O=…/C=US`.
fn subject_team(subject: &str) -> Option<String> {
    let unit = subject.split_once("OU=")?.1;
    let team: String = unit
        .chars()
        .take_while(char::is_ascii_alphanumeric)
        .collect();
    (!team.is_empty()).then_some(team)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_line_yields_hash_and_name() {
        let (hash, name) =
            parse_identity(r#"  1) ABCDEF0123456789ABCDEF0123456789ABCDEF01 "Apple Development: Jane Doe (HOLDER1234)""#)
                .unwrap();
        assert_eq!(hash, "ABCDEF0123456789ABCDEF0123456789ABCDEF01");
        assert_eq!(name, "Apple Development: Jane Doe (HOLDER1234)");
    }

    #[test]
    fn the_team_is_the_subjects_organizational_unit_not_the_holders_id() {
        let subject = "subject= /UID=HOLDER1234/CN=Apple Development: Jane Doe (HOLDER1234)/OU=TEAM123456/O=Jane Doe/C=US";
        assert_eq!(subject_team(subject).as_deref(), Some("TEAM123456"));
        assert_eq!(subject_team("subject= /CN=Someone"), None);
    }

    #[test]
    fn the_pem_is_the_one_after_the_identitys_hash() {
        let listing = "SHA-256 hash: 11\nSHA-1 hash: AAAA\n-----BEGIN CERTIFICATE-----\nfirst\n-----END CERTIFICATE-----\nSHA-256 hash: 22\nSHA-1 hash: BBBB\n-----BEGIN CERTIFICATE-----\nsecond\n-----END CERTIFICATE-----\n";
        assert_eq!(
            certificate_pem(listing, "bbbb"),
            Some("-----BEGIN CERTIFICATE-----\nsecond\n-----END CERTIFICATE-----")
        );
        assert_eq!(certificate_pem(listing, "cccc"), None);
    }
}
