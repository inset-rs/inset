//! How an iOS app is signed, by who it is for: the simulator, ad hoc; the developer's own devices,
//! with the keychain's Apple Development certificate and a development profile, which Xcode gets
//! where none it stored will do (`provisioning.rs`); or distribution, which Xcode signs as it
//! exports the app (`export.rs`), so the app is left unsigned here.
//!
//! Distribution is Xcode's to sign because its certificate may be one Apple keeps in the cloud,
//! which only Xcode signs with.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use clap::ValueEnum;
use plist::{Dictionary, Value};

use super::IosDevice;
use super::minimum_version;
use super::profiles::{self, Signed};
use super::provisioning::{self, Request};
use crate::apple::{self, SigningIdentity, SigningSettings};
use crate::project::Project;

/// Who an iOS app is signed for.
#[derive(Clone, Copy, Debug)]
pub enum Signing<'a> {
    /// A simulator: ad hoc, with no certificate.
    Simulator,
    /// The developer's devices: the one named, which its profile must list, or any of the team's.
    Development(Option<&'a IosDevice>),
    /// Users: left unsigned, for Xcode to sign as it exports the app for an [`ExportMethod`].
    Distribution,
}

/// How an `.ipa` is distributed, as `flutter build ipa --export-method` names it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum ExportMethod {
    /// Uploaded to App Store Connect, for TestFlight and the App Store.
    #[default]
    AppStore,
    /// Installed on the devices the distribution profile lists.
    AdHoc,
    /// Installed on the team's development devices.
    Development,
    /// Installed by an Apple Developer Enterprise Program organization's people.
    Enterprise,
}

impl ExportMethod {
    /// The `method` of Xcode's export options.
    pub(super) fn xcode_method(self) -> &'static str {
        match self {
            ExportMethod::AppStore => "app-store-connect",
            ExportMethod::AdHoc => "release-testing",
            ExportMethod::Development => "debugging",
            ExportMethod::Enterprise => "enterprise",
        }
    }
}

impl Signing<'_> {
    pub(super) fn triple(self) -> &'static str {
        match self {
            Signing::Simulator if cfg!(target_arch = "aarch64") => "aarch64-apple-ios-sim",
            Signing::Simulator => "x86_64-apple-ios",
            Signing::Development(_) | Signing::Distribution => "aarch64-apple-ios",
        }
    }

    pub(super) fn dir_name(self) -> &'static str {
        match self {
            Signing::Simulator => "ios-simulator",
            Signing::Development(_) => "ios",
            Signing::Distribution => "ios-distribution",
        }
    }

    pub(super) fn platform_name(self) -> &'static str {
        match self {
            Signing::Simulator => "iphonesimulator",
            Signing::Development(_) | Signing::Distribution => "iphoneos",
        }
    }

    pub(super) fn supported_platform(self) -> &'static str {
        match self {
            Signing::Simulator => "iPhoneSimulator",
            Signing::Development(_) | Signing::Distribution => "iPhoneOS",
        }
    }
}

/// Signs `app` as `signing` says; an app for distribution is left for Xcode's export.
pub(super) fn sign(project: &Project, app: &Path, out: &Path, signing: Signing) -> Result<()> {
    match signing {
        Signing::Simulator => sign_ad_hoc(app),
        Signing::Development(device) => sign_for_development(project, app, out, device),
        Signing::Distribution => Ok(()),
    }
}

fn sign_ad_hoc(app: &Path) -> Result<()> {
    apple::sign(
        app,
        &SigningSettings {
            identity: &SigningIdentity::AdHoc,
            hardened_runtime: false,
            entitlements: None,
            timestamp: false,
        },
    )
}

/// Signs `app` with the keychain's Apple Development certificate and a profile that lets it run on
/// `device`, or on the team's devices where none is named: one Xcode stored, or else one Xcode gets
/// now, as `flutter run` has Xcode get it.
fn sign_for_development(
    project: &Project,
    app: &Path,
    out: &Path,
    device: Option<&IosDevice>,
) -> Result<()> {
    let (identity, team) = apple::development_identity(project.ios.team.as_deref())?;
    let signed = Signed {
        bundle_id: &project.identifier,
        team: &team,
        device: device.map(|device| device.udid.as_str()),
    };
    let profile = match profiles::stored_profile(&signed) {
        Some(profile) => profile,
        None => {
            let request = Request {
                bundle_id: signed.bundle_id,
                team: signed.team,
                device: signed.device,
                minimum_version: &minimum_version(project),
            };
            provisioning::obtain_profile(&request, &out.join("provisioning"))?;
            profiles::stored_profile(&signed).with_context(|| {
                format!(
                    "Xcode got no provisioning profile for {} (team {}) that Waft finds among those it stores",
                    signed.bundle_id, signed.team
                )
            })?
        }
    };
    fs::copy(&profile.path, app.join("embedded.mobileprovision"))?;
    let entitlements = out.join("entitlements.plist");
    plist::to_file_xml(
        &entitlements,
        &development_entitlements(&project.identifier, &team),
    )?;
    println!("signing with {} ({})", identity.name, profile.name);
    apple::sign(
        app,
        &SigningSettings {
            identity: &SigningIdentity::Keychain(identity),
            hardened_runtime: false,
            entitlements: Some(&entitlements),
            timestamp: false,
        },
    )
}

fn development_entitlements(bundle_id: &str, team: &str) -> Value {
    let mut entitlements = Dictionary::new();
    entitlements.insert(
        "application-identifier".to_owned(),
        Value::String(format!("{team}.{bundle_id}")),
    );
    entitlements.insert(
        "com.apple.developer.team-identifier".to_owned(),
        Value::String(team.to_owned()),
    );
    entitlements.insert("get-task-allow".to_owned(), Value::Boolean(true));
    Value::Dictionary(entitlements)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_export_method_is_xcodes_current_name_for_it() {
        assert_eq!(ExportMethod::default(), ExportMethod::AppStore);
        assert_eq!(ExportMethod::AppStore.xcode_method(), "app-store-connect");
        assert_eq!(ExportMethod::AdHoc.xcode_method(), "release-testing");
        assert_eq!(ExportMethod::Development.xcode_method(), "debugging");
        assert_eq!(ExportMethod::Enterprise.xcode_method(), "enterprise");
    }

    #[test]
    fn a_distribution_build_is_for_a_device_in_a_folder_of_its_own() {
        let signing = Signing::Distribution;
        assert_eq!(signing.triple(), "aarch64-apple-ios");
        assert_eq!(signing.dir_name(), "ios-distribution");
    }
}
