//! A provisioning profile obtained through Xcode, as `flutter run` has Xcode obtain one: `xcodebuild`
//! builds an app with automatic signing, `-allowProvisioningUpdates` and
//! `-allowProvisioningDeviceRegistration`, and Xcode, with the account signed in under its Settings >
//! Accounts, registers the device, makes the app id and downloads the profile, which it stores with
//! the others.
//!
//! Waft's app has no Xcode project, so the app built is the smallest one with the bundle id and the
//! team: one target of one C file, which Xcode only needs to sign. The CLI never handles the account:
//! Xcode reaches Apple with what it keeps.

use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

/// One iOS application target, `Provisioning`, built from `main.c`; its bundle id, team and system
/// version are the build's settings.
const PROJECT: &str = r#"// !$*UTF8*$!
{
	archiveVersion = 1;
	classes = {
	};
	objectVersion = 56;
	objects = {
		A0000000000000000000000B = {isa = PBXBuildFile; fileRef = A0000000000000000000000F; };
		A0000000000000000000000F = {isa = PBXFileReference; lastKnownFileType = sourcecode.c.c; path = main.c; sourceTree = "<group>"; };
		A0000000000000000000000A = {isa = PBXFileReference; explicitFileType = wrapper.application; includeInIndex = 0; path = Provisioning.app; sourceTree = BUILT_PRODUCTS_DIR; };
		A00000000000000000000001 = {isa = PBXGroup; children = (A0000000000000000000000F, A00000000000000000000002); sourceTree = "<group>"; };
		A00000000000000000000002 = {isa = PBXGroup; children = (A0000000000000000000000A); name = Products; sourceTree = "<group>"; };
		A00000000000000000000003 = {isa = PBXSourcesBuildPhase; buildActionMask = 2147483647; files = (A0000000000000000000000B); runOnlyForDeploymentPostprocessing = 0; };
		A00000000000000000000004 = {isa = PBXNativeTarget; buildConfigurationList = A00000000000000000000006; buildPhases = (A00000000000000000000003); buildRules = (); dependencies = (); name = Provisioning; productName = Provisioning; productReference = A0000000000000000000000A; productType = "com.apple.product-type.application"; };
		A00000000000000000000005 = {isa = PBXProject; buildConfigurationList = A00000000000000000000007; compatibilityVersion = "Xcode 14.0"; mainGroup = A00000000000000000000001; productRefGroup = A00000000000000000000002; projectDirPath = ""; projectRoot = ""; targets = (A00000000000000000000004); };
		A00000000000000000000006 = {isa = XCConfigurationList; buildConfigurations = (A00000000000000000000008); defaultConfigurationIsVisible = 0; defaultConfigurationName = Debug; };
		A00000000000000000000007 = {isa = XCConfigurationList; buildConfigurations = (A00000000000000000000009); defaultConfigurationIsVisible = 0; defaultConfigurationName = Debug; };
		A00000000000000000000008 = {isa = XCBuildConfiguration; buildSettings = {
			CODE_SIGN_STYLE = Automatic;
			GENERATE_INFOPLIST_FILE = YES;
			PRODUCT_NAME = Provisioning;
			TARGETED_DEVICE_FAMILY = "1,2";
		}; name = Debug; };
		A00000000000000000000009 = {isa = XCBuildConfiguration; buildSettings = {
			ALWAYS_SEARCH_USER_PATHS = NO;
			SDKROOT = iphoneos;
			SUPPORTED_PLATFORMS = "iphoneos iphonesimulator";
		}; name = Debug; };
	};
	rootObject = A00000000000000000000005;
}
"#;

/// What the profile is for: the app's bundle id, of a team, on the device of this UDID, or on the
/// team's registered devices where none is named.
pub(super) struct Request<'a> {
    pub(super) bundle_id: &'a str,
    pub(super) team: &'a str,
    pub(super) device: Option<&'a str>,
    pub(super) minimum_version: &'a str,
}

/// Has Xcode obtain and store a profile for `request`, building the smallest app in `dir`.
pub(super) fn obtain_profile(request: &Request, dir: &Path) -> Result<()> {
    let project = write_project(dir)?;
    println!(
        "asking Xcode for a provisioning profile for {} (team {})",
        request.bundle_id, request.team
    );
    let built = Command::new("xcodebuild")
        .args(xcodebuild_arguments(request, &project, &dir.join("build")))
        .output()
        .context("running xcodebuild")?;
    if built.status.success() {
        return Ok(());
    }
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );
    bail!(failure(&log, request.team, "get a provisioning profile"))
}

/// The project and its source, written afresh into `dir`; answers the project's path.
fn write_project(dir: &Path) -> Result<std::path::PathBuf> {
    let project = dir.join("Provisioning.xcodeproj");
    fs::create_dir_all(&project)?;
    fs::write(project.join("project.pbxproj"), PROJECT)?;
    fs::write(dir.join("main.c"), "int main(void) { return 0; }\n")?;
    Ok(project)
}

/// `flutter run`'s signing arguments to `xcodebuild`, for the smallest app.
fn xcodebuild_arguments(request: &Request, project: &Path, derived_data: &Path) -> Vec<String> {
    let destination = match request.device {
        Some(udid) => format!("id={udid}"),
        None => "generic/platform=iOS".to_owned(),
    };
    vec![
        "-project".to_owned(),
        project.display().to_string(),
        "-scheme".to_owned(),
        "Provisioning".to_owned(),
        "-configuration".to_owned(),
        "Debug".to_owned(),
        "-destination".to_owned(),
        destination,
        "-derivedDataPath".to_owned(),
        derived_data.display().to_string(),
        "-allowProvisioningUpdates".to_owned(),
        "-allowProvisioningDeviceRegistration".to_owned(),
        "-quiet".to_owned(),
        format!("PRODUCT_BUNDLE_IDENTIFIER={}", request.bundle_id),
        format!("DEVELOPMENT_TEAM={}", request.team),
        format!("IPHONEOS_DEPLOYMENT_TARGET={}", request.minimum_version),
        "build".to_owned(),
    ]
}

/// Why Xcode could not do `what`, from `xcodebuild`'s `log`: it signed in to no account of `team`,
/// said as what to do; else the errors it printed.
pub(super) fn failure(log: &str, team: &str, what: &str) -> String {
    if log.contains("No Accounts") || log.contains("No Account for Team") {
        return format!(
            "Xcode has no account of team {team} signed in, which it needs to {what}: \
             open Xcode > Settings > Accounts, add an Apple ID of that team, then run this again"
        );
    }
    let errors: Vec<&str> = log
        .lines()
        .filter(|line| line.contains("error:"))
        .map(str::trim)
        .collect();
    let shown = if errors.is_empty() {
        log.trim().to_owned()
    } else {
        errors.join("\n")
    };
    format!("Xcode could not {what}:\n{shown}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(device: Option<&'static str>) -> Request<'static> {
        Request {
            bundle_id: "com.example.app",
            team: "TEAM123456",
            device,
            minimum_version: "15.0",
        }
    }

    #[test]
    fn xcode_may_register_the_device_and_update_profiles_for_the_team() {
        let arguments = xcodebuild_arguments(
            &request(Some("00008160-0001")),
            Path::new("/p/Provisioning.xcodeproj"),
            Path::new("/p/build"),
        );
        for wanted in [
            "-allowProvisioningUpdates",
            "-allowProvisioningDeviceRegistration",
            "id=00008160-0001",
            "PRODUCT_BUNDLE_IDENTIFIER=com.example.app",
            "DEVELOPMENT_TEAM=TEAM123456",
        ] {
            assert!(arguments.iter().any(|a| a == wanted), "{wanted}");
        }
        let any_device = xcodebuild_arguments(&request(None), Path::new("/p"), Path::new("/b"));
        assert!(any_device.iter().any(|a| a == "generic/platform=iOS"));
    }

    #[test]
    fn no_account_is_said_as_what_to_do() {
        let log = "error: No Account for Team \"TEAM123456\". Add a new account in Accounts settings or verify that your accounts have valid credentials. (in target 'Provisioning' from project 'Provisioning')";
        assert!(
            failure(log, "TEAM123456", "get a provisioning profile")
                .contains("Xcode > Settings > Accounts")
        );
        let other = failure(
            "note: x\nerror: Device is locked\n",
            "TEAM123456",
            "get a provisioning profile",
        );
        assert!(other.ends_with("error: Device is locked"));
    }
}
