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

/// A code signing identity in the keychain, as `security find-identity` lists it, with the team
/// its certificate is issued to where the certificate names one.
pub struct Identity {
    pub hash: String,
    pub name: String,
    pub team: Option<String>,
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

/// The keychain's Apple Development certificate, of `team` when a machine has several, with the
/// team it is issued to.
pub fn development_identity(team: Option<&str>) -> Result<(Identity, String)> {
    let development = keychain_identities()?
        .into_iter()
        .filter(|i| {
            i.name.starts_with("Apple Development") || i.name.starts_with("iPhone Developer")
        })
        .find_map(|i| {
            let issued = i.team.clone()?;
            team.is_none_or(|team| issued == team)
                .then_some((i, issued))
        });
    match development {
        Some(chosen) => Ok(chosen),
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
    let team = keychain_identities()?.into_iter().find_map(|i| {
        [
            "Apple Development",
            "Apple Distribution",
            "iPhone Developer",
            "iPhone Distribution",
        ]
        .iter()
        .any(|kind| i.name.starts_with(kind))
        .then_some(i.team)?
    });
    match team {
        Some(team) => Ok(team),
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

/// Every valid code signing identity in the keychain, with its certificate's team where it names
/// one. Signing with an identity needs only its hash or name; the team matters only where one is
/// chosen for a team, so an identity whose team cannot be read is kept, without it.
fn keychain_identities() -> Result<Vec<Identity>> {
    let listing = tools::output(
        Command::new("security").args(["find-identity", "-v", "-p", "codesigning"]),
        "security find-identity",
    )?;
    Ok(listing
        .lines()
        .filter_map(parse_identity)
        .map(|(hash, name)| {
            let team = certificate_team(&hash, &name);
            Identity { hash, name, team }
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
    pem_team(certificate_pem(&listing, hash)?)
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

/// The organizational unit of the subject of the certificate `pem`, read from the certificate
/// itself rather than from a tool's text of it, whose shape differs between OpenSSL and LibreSSL.
fn pem_team(pem: &str) -> Option<String> {
    let (_, pem) = x509_parser::pem::parse_x509_pem(pem.as_bytes()).ok()?;
    let certificate = pem.parse_x509().ok()?;
    let unit = certificate.subject().iter_organizational_unit().next()?;
    let team = unit.as_str().ok()?;
    (!team.is_empty()).then(|| team.to_owned())
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

    /// A certificate whose subject names its holder `HOLDER1234` and its team `TEAM123456`, as an
    /// Apple Development certificate does.
    const DEVELOPMENT_CERTIFICATE: &str = "-----BEGIN CERTIFICATE-----
MIIC7jCCApQCCQC0YtLWr8vmrDAKBggqhkjOPQQDAjCBhDEaMBgGCgmSJomT8ixk
AQEMCkhPTERFUjEyMzQxMTAvBgNVBAMMKEFwcGxlIERldmVsb3BtZW50OiBKYW5l
IERvZSAoSE9MREVSMTIzNCkxEzARBgNVBAsMClRFQU0xMjM0NTYxETAPBgNVBAoM
CEphbmUgRG9lMQswCQYDVQQGEwJVUzAeFw0yNjExMTkwMTI1MjRaFw0zNjExMTYw
MTI1MjRaMIGEMRowGAYKCZImiZPyLGQBAQwKSE9MREVSMTIzNDExMC8GA1UEAwwo
QXBwbGUgRGV2ZWxvcG1lbnQ6IEphbmUgRG9lIChIT0xERVIxMjM0KTETMBEGA1UE
CwwKVEVBTTEyMzQ1NjERMA8GA1UECgwISmFuZSBEb2UxCzAJBgNVBAYTAlVTMIIB
SzCCAQMGByqGSM49AgEwgfcCAQEwLAYHKoZIzj0BAQIhAP////8AAAABAAAAAAAA
AAAAAAAA////////////////MFsEIP////8AAAABAAAAAAAAAAAAAAAA////////
///////8BCBaxjXYqjqT57PrvVV2mIa8ZR0GsMxTsPY7zjw+J9JgSwMVAMSdNgiG
5wSTamZ44ROdJreBn36QBEEEaxfR8uEsQkf4vOblY6RA8ncDfYEt6zOg9KE5RdiY
wpZP40Li/hp/m47n60p8D54WK84zV2sxXs7LtkBoN79R9QIhAP////8AAAAA////
//////+85vqtpxeehPO5ysL8YyVRAgEBA0IABARl3/bvu2QlfdWsb59U/kJA0344
9oaLvc6hPjzuPZrpZuN6dncO65P8z2JjtRiKAH8ozC+Gg4SV08bnkIalEB4wCgYI
KoZIzj0EAwIDSAAwRQIgBOUObMYD7AdIsEWJGtn2LiWGoV3jziFCk6htyDNuZuMC
IQCtfNYiTNwYOPJac/n8W7tCPXcMZiPItUXB6O3KTahd+w==
-----END CERTIFICATE-----";

    #[test]
    fn the_team_is_the_subjects_organizational_unit_not_the_holders_id() {
        assert_eq!(
            pem_team(DEVELOPMENT_CERTIFICATE).as_deref(),
            Some("TEAM123456")
        );
        assert_eq!(pem_team("not a certificate"), None);
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
