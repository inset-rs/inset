//! An `.ipa` for distribution, made by Xcode as `flutter build ipa` has it made: `xcodebuild
//! -exportArchive -allowProvisioningUpdates` signs the app in an archive for an export method, with
//! the certificate and profile the method needs, which it makes with the account signed in under
//! Xcode > Settings > Accounts where the team has none, and packs the `.ipa`.
//!
//! Waft builds no archive with Xcode, so the archive is the app, unsigned, under
//! `Products/Applications` with the `Info.plist` an Xcode archive has: what Xcode reads of it is the
//! app's path, identifier, versions and team.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use anyhow::{Context, Result, bail};
use plist::{Dictionary, Value};

use super::provisioning::failure;
use super::{App, ExportMethod};
use crate::apple;
use crate::project::Project;

/// `app`, built for distribution, exported by Xcode for `method` into an `.ipa` in `out`.
pub fn export_ipa(
    project: &Project,
    app: &App,
    method: ExportMethod,
    out: &Path,
) -> Result<PathBuf> {
    let team = apple::team(project.ios.team.as_deref())?;
    let archive = write_archive(project, app, &team, out)?;
    let options = out.join("ExportOptions.plist");
    plist::to_file_xml(&options, &export_options(method, &team))?;
    let exported = out.join("export");
    if exported.exists() {
        fs::remove_dir_all(&exported)?;
    }
    println!("exporting for {method:?} distribution (team {team}) with Xcode");
    let export = Command::new("xcodebuild")
        .arg("-exportArchive")
        .arg("-archivePath")
        .arg(&archive)
        .arg("-exportOptionsPlist")
        .arg(&options)
        .arg("-exportPath")
        .arg(&exported)
        .arg("-allowProvisioningUpdates")
        .output()
        .context("running xcodebuild")?;
    if !export.status.success() {
        let log = format!(
            "{}{}",
            String::from_utf8_lossy(&export.stdout),
            String::from_utf8_lossy(&export.stderr)
        );
        bail!(failure(&log, &team, "export the app"));
    }
    exported_ipa(&exported)
}

/// The archive Xcode exports: `app`, copied under `Products/Applications`, and its `Info.plist`.
fn write_archive(project: &Project, app: &App, team: &str, out: &Path) -> Result<PathBuf> {
    let archive = out.join(format!("{}.xcarchive", project.name));
    if archive.exists() {
        fs::remove_dir_all(&archive)?;
    }
    let applications = archive.join("Products/Applications");
    fs::create_dir_all(&applications)?;
    let name = app.path.file_name().context("the app folder has no name")?;
    let copied = Command::new("cp")
        .arg("-R")
        .arg(&app.path)
        .arg(applications.join(name))
        .status()?;
    if !copied.success() {
        bail!("copying the app into its archive failed");
    }
    let application_path = format!("Applications/{}", name.to_string_lossy());
    plist::to_file_xml(
        archive.join("Info.plist"),
        &archive_info(project, &application_path, team),
    )?;
    Ok(archive)
}

/// The `Info.plist` of an Xcode archive of one app.
fn archive_info(project: &Project, application_path: &str, team: &str) -> Value {
    let mut application = Dictionary::new();
    application.insert("ApplicationPath".into(), application_path.into());
    application.insert(
        "CFBundleIdentifier".into(),
        project.identifier.clone().into(),
    );
    application.insert(
        "CFBundleShortVersionString".into(),
        project.version.clone().into(),
    );
    application.insert("CFBundleVersion".into(), project.version.clone().into());
    application.insert("Team".into(), team.into());
    application.insert("Architectures".into(), Value::Array(vec!["arm64".into()]));
    let mut info = Dictionary::new();
    info.insert(
        "ApplicationProperties".into(),
        Value::Dictionary(application),
    );
    info.insert("ArchiveVersion".into(), Value::Integer(2.into()));
    info.insert("CreationDate".into(), Value::Date(SystemTime::now().into()));
    info.insert("Name".into(), project.name.clone().into());
    info.insert("SchemeName".into(), project.name.clone().into());
    Value::Dictionary(info)
}

/// Xcode's export options: `method`, for `team`, signed automatically, so Xcode picks or makes the
/// certificate and the profile.
fn export_options(method: ExportMethod, team: &str) -> Value {
    let mut options = Dictionary::new();
    options.insert("method".into(), method.xcode_method().into());
    options.insert("signingStyle".into(), "automatic".into());
    options.insert("teamID".into(), team.into());
    Value::Dictionary(options)
}

/// The one `.ipa` Xcode exported into `dir`.
fn exported_ipa(dir: &Path) -> Result<PathBuf> {
    fs::read_dir(dir)?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|e| e == "ipa"))
        .with_context(|| format!("Xcode exported no .ipa into {}", dir.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_export_is_signed_automatically_for_the_method_and_team() {
        let options = export_options(ExportMethod::AdHoc, "TEAM123456");
        let options = options.as_dictionary().unwrap();
        assert_eq!(options["method"].as_string(), Some("release-testing"));
        assert_eq!(options["signingStyle"].as_string(), Some("automatic"));
        assert_eq!(options["teamID"].as_string(), Some("TEAM123456"));
    }
}
