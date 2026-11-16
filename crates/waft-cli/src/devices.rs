//! What `run -d` can name: this desktop, a browser, a simulator, an iPhone or iPad, an Android
//! emulator, an Android device. `devices` prints the listing, and `run -d` picks from the same
//! listing, made once, as Flutter's `-d` picks: a device whose id is the name given, else one whose
//! name is it, else one whose name starts with it, ignoring case.
//!
//! The invariant a later editor will break: a physical device is never shadowed by a simulator or
//! an emulator of the same name. Where the name fits several, a physical device is taken before a
//! simulator or an emulator; two physical devices it fits are refused, naming both; among
//! simulators of one name, which every runtime has, the booted one is taken, else the newest
//! runtime's, which the listing puts first.

use anyhow::{Result, bail};

use crate::android;
use crate::ios::{self, IosDevice, Simulator};
use crate::web::Browser;

/// One thing `run -d` can name, as `devices` lists it, with what running on it means.
#[derive(Clone, Debug)]
pub struct Device {
    pub id: String,
    pub name: String,
    /// What the listing says besides the name, such as a simulator's runtime; empty for none.
    pub detail: String,
    pub platform: &'static str,
    pub state: String,
    pub target: Target,
}

impl Device {
    /// Whether it is hardware, not a simulator, an emulator, this desktop or a browser.
    fn is_physical(&self) -> bool {
        matches!(self.target, Target::IosDevice(_) | Target::AndroidDevice(_))
            && !self.is_emulator()
    }

    fn is_emulator(&self) -> bool {
        matches!(&self.target, Target::AndroidDevice(device) if device.avd.is_some())
            || matches!(self.target, Target::AndroidEmulator(_))
    }

    /// Whether it is a booted simulator or a running emulator.
    fn is_running(&self) -> bool {
        match &self.target {
            Target::IosSimulator(simulator) => simulator.state == "Booted",
            Target::AndroidDevice(device) => device.avd.is_some(),
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Target {
    Desktop,
    Web {
        browser: Browser,
    },
    IosSimulator(Simulator),
    /// An iPhone or iPad `devicectl` reaches.
    IosDevice(IosDevice),
    /// An Android emulator to boot, by AVD name.
    AndroidEmulator(String),
    /// An Android device or a running emulator `adb` reaches.
    AndroidDevice(android::Device),
}

/// Everything `run -d` can name now: this desktop, the browsers, and each platform's devices.
pub fn list() -> Result<Vec<Device>> {
    let mut devices = this_desktop_and_browsers();
    devices.extend(ios::listed_devices()?);
    devices.extend(android::listed_devices()?);
    Ok(devices)
}

/// The targets that need nothing attached: the machine this runs on, and its browsers.
fn this_desktop_and_browsers() -> Vec<Device> {
    let device = |id: &str, name: &str, platform, target| Device {
        id: id.to_owned(),
        name: name.to_owned(),
        detail: String::new(),
        platform,
        state: String::new(),
        target,
    };
    vec![
        device(
            std::env::consts::OS,
            "this desktop",
            "desktop",
            Target::Desktop,
        ),
        device(
            "web",
            "default browser",
            "web",
            Target::Web {
                browser: Browser::Default,
            },
        ),
        device(
            "chrome",
            "Google Chrome",
            "web",
            Target::Web {
                browser: Browser::Chrome,
            },
        ),
    ]
}

pub fn print(devices: &[Device]) {
    let width = devices.iter().map(|d| d.id.len()).max().unwrap_or(0);
    for device in devices {
        let name = if device.detail.is_empty() {
            device.name.clone()
        } else {
            format!("{} ({})", device.name, device.detail)
        };
        println!(
            "{:width$}  {:16} {}  {}",
            device.id, device.platform, name, device.state
        );
    }
}

/// What `-d wanted` names: this desktop with none; `ios` and `android`, each platform's own pick;
/// else the device of the listing the name fits.
pub fn resolve(wanted: Option<&str>) -> Result<Target> {
    let Some(wanted) = wanted else {
        return Ok(Target::Desktop);
    };
    if matches!(wanted, "macos" | "windows" | "linux" | "desktop") {
        return Ok(Target::Desktop);
    }
    let devices = list()?;
    let target = match wanted {
        "ios" => ios::default_simulator(&devices).map(|device| device.target.clone()),
        "android" => android::default_target(&devices)?,
        _ => named(&devices, wanted)?.map(|device| device.target.clone()),
    };
    let Some(target) = target else {
        bail!("no device matches `{wanted}`; see `cargo waft devices`")
    };
    if let Target::AndroidDevice(device) = &target {
        android::ensure_ready(device)?;
    }
    Ok(target)
}

/// The device `wanted` names: by id, else by whole name, else by the start of its name.
fn named<'a>(devices: &'a [Device], wanted: &str) -> Result<Option<&'a Device>> {
    let wanted = folded(wanted);
    if let Some(device) = devices.iter().find(|d| folded(&d.id) == wanted) {
        return Ok(Some(device));
    }
    let whole: Vec<&Device> = devices
        .iter()
        .filter(|d| folded(&d.name) == wanted)
        .collect();
    if !whole.is_empty() {
        return preferred(&whole);
    }
    let started: Vec<&Device> = devices
        .iter()
        .filter(|d| folded(&d.name).starts_with(&wanted))
        .collect();
    preferred(&started)
}

/// `name` as it is compared: lower case, with the typographic apostrophe iOS puts in a device's
/// name ("Jane’s iPhone") as the one a keyboard types.
fn folded(name: &str) -> String {
    name.to_lowercase().replace('\u{2019}', "'")
}

/// Of the devices a name fits: the one physical device, else the first running simulator or
/// emulator, else the first; two physical devices are refused.
fn preferred<'a>(fitting: &[&'a Device]) -> Result<Option<&'a Device>> {
    let physical: Vec<&&Device> = fitting.iter().filter(|d| d.is_physical()).collect();
    match physical.as_slice() {
        [one] => return Ok(Some(one)),
        [] => {}
        several => bail!(
            "several devices match: {}; name one by its id",
            several
                .iter()
                .map(|d| format!("{} ({})", d.name, d.id))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
    Ok(fitting
        .iter()
        .find(|d| d.is_running())
        .or_else(|| fitting.first())
        .copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn simulator(udid: &str, name: &str, state: &str) -> Device {
        Device {
            id: udid.into(),
            name: name.into(),
            detail: "iOS 27.0".into(),
            platform: "ios simulator",
            state: state.into(),
            target: Target::IosSimulator(Simulator {
                udid: udid.into(),
                name: name.into(),
                state: state.into(),
                runtime: "iOS 27.0".into(),
            }),
        }
    }

    fn iphone(id: &str, name: &str) -> Device {
        Device {
            id: id.into(),
            name: name.into(),
            detail: String::new(),
            platform: "ios device",
            state: "connected".into(),
            target: Target::IosDevice(IosDevice {
                id: id.into(),
                udid: format!("{id}-udid"),
                name: name.into(),
                state: "connected".into(),
            }),
        }
    }

    fn chosen(devices: &[Device], wanted: &str) -> Option<String> {
        named(devices, wanted)
            .unwrap()
            .map(|device| device.id.clone())
    }

    #[test]
    fn a_physical_device_is_not_shadowed_by_a_simulator_of_its_name() {
        let devices = [
            simulator("SIM-NEW", "iPhone 17", "Shutdown"),
            iphone("PHONE", "iPhone 17"),
        ];
        assert_eq!(chosen(&devices, "iphone 17").as_deref(), Some("PHONE"));
        assert_eq!(chosen(&devices, "iPhone").as_deref(), Some("PHONE"));
        assert_eq!(chosen(&devices, "SIM-NEW").as_deref(), Some("SIM-NEW"));
    }

    #[test]
    fn among_simulators_of_one_name_the_booted_one_else_the_newest_is_taken() {
        let devices = [
            simulator("NEW", "iPhone 16", "Shutdown"),
            simulator("OLD", "iPhone 16", "Booted"),
        ];
        assert_eq!(chosen(&devices, "iPhone 16").as_deref(), Some("OLD"));
        let none_booted = [
            simulator("NEW", "iPhone 16", "Shutdown"),
            simulator("OLD", "iPhone 16", "Shutdown"),
        ];
        assert_eq!(chosen(&none_booted, "iPhone 16").as_deref(), Some("NEW"));
    }

    #[test]
    fn a_whole_name_wins_over_the_start_of_a_longer_one() {
        let devices = [
            iphone("DUO", "Ty's iPhone Duo"),
            simulator("SIM", "Ty's iPhone", "Shutdown"),
        ];
        assert_eq!(chosen(&devices, "ty's iphone").as_deref(), Some("SIM"));
        assert_eq!(chosen(&devices, "ty's iphone d").as_deref(), Some("DUO"));
    }

    #[test]
    fn a_device_named_with_ios_apostrophe_is_named_with_a_typed_one() {
        let devices = [
            simulator("SIM", "iPhone Duo", "Shutdown"),
            iphone("PHONE", "Ty\u{2019}s iPhone Duo"),
        ];
        assert_eq!(
            chosen(&devices, "Ty's iPhone Duo").as_deref(),
            Some("PHONE")
        );
        assert_eq!(chosen(&devices, "iPhone Duo").as_deref(), Some("SIM"));
    }

    #[test]
    fn two_physical_devices_a_name_fits_are_refused() {
        let devices = [iphone("A", "iPhone"), iphone("B", "iPhone")];
        assert!(named(&devices, "iphone").is_err());
        assert_eq!(chosen(&devices, "b").as_deref(), Some("B"));
        assert_eq!(chosen(&devices, "nothing"), None);
    }
}
