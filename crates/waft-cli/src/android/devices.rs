//! The emulators and devices `adb` reaches: listing them, booting an emulator, the processor of a
//! device, and installing and launching the app on one with its log followed.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};

use super::abi::Abi;
use super::manifest::WAFT_ACTIVITY;
use super::{Apk, sdk};

use crate::devices::Target;
use crate::tools;

/// How long an emulator gets to boot.
const BOOT_TIMEOUT: Duration = Duration::from_secs(240);

/// A device `adb` can reach, running or booting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    pub serial: String,
    pub model: String,
    /// `device`, `offline`, `unauthorized`, as `adb` reports it.
    pub state: String,
    /// The emulator's AVD, for a device that is one.
    pub avd: Option<String>,
}

/// The emulators this machine has: AVD names.
pub fn emulators() -> Result<Vec<String>> {
    let Ok(sdk) = sdk() else {
        return Ok(Vec::new());
    };
    let Some(emulator) = sdk.emulator else {
        return Ok(Vec::new());
    };
    let listing = tools::output(
        Command::new(emulator).arg("-list-avds"),
        "emulator -list-avds",
    )?;
    Ok(listing
        .lines()
        .map(str::trim)
        // The emulator prints its own notices among the names.
        .filter(|line| !line.is_empty() && !line.starts_with("INFO") && !line.contains(' '))
        .map(str::to_owned)
        .collect())
}

/// The devices `adb` sees, emulators named by their AVD.
pub fn devices() -> Result<Vec<Device>> {
    let Ok(sdk) = sdk() else {
        return Ok(Vec::new());
    };
    if !sdk.adb.is_file() {
        return Ok(Vec::new());
    }
    let listing = tools::output(
        Command::new(&sdk.adb).args(["devices", "-l"]),
        "adb devices",
    )?;
    let mut devices = parse_devices(&listing);
    for device in &mut devices {
        if device.serial.starts_with("emulator-") {
            device.avd = avd_name(&sdk.adb, &device.serial);
        }
    }
    Ok(devices)
}

/// `emulator-5554   device product:sdk_gphone64_arm64 model:sdk_gphone64_arm64 …`.
fn parse_devices(listing: &str) -> Vec<Device> {
    listing
        .lines()
        .skip_while(|line| !line.starts_with("List of devices"))
        .skip(1)
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let serial = fields.next()?;
            let state = fields.next()?;
            let model = fields
                .find_map(|field| field.strip_prefix("model:"))
                .unwrap_or(serial)
                .to_owned();
            Some(Device {
                serial: serial.to_owned(),
                model,
                state: state.to_owned(),
                avd: None,
            })
        })
        .collect()
}

/// The AVD an emulator runs, from its console: the name, then `OK`.
fn avd_name(adb: &Path, serial: &str) -> Option<String> {
    let answer = tools::output(
        Command::new(adb).args(["-s", serial, "emu", "avd", "name"]),
        "adb emu avd name",
    )
    .ok()?;
    answer
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && *line != "OK")
        .map(str::to_owned)
}

/// Starts the emulator for `avd` and waits for it to boot; answers the device it is.
pub(super) fn boot_emulator(avd: &str) -> Result<Device> {
    let sdk = sdk()?;
    let emulator = sdk
        .emulator
        .as_ref()
        .context("no emulator under the SDK; install it with the SDK Manager")?;
    println!("booting {avd}");
    Command::new(emulator)
        .args(["-avd", avd])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("could not start the emulator")?;
    let started = Instant::now();
    while started.elapsed() < BOOT_TIMEOUT {
        std::thread::sleep(Duration::from_secs(2));
        let Some(device) = devices()?
            .into_iter()
            .find(|device| device.avd.as_deref() == Some(avd) && device.state == "device")
        else {
            continue;
        };
        if booted(&sdk.adb, &device.serial) {
            return Ok(device);
        }
    }
    bail!(
        "{avd} did not boot within {} seconds",
        BOOT_TIMEOUT.as_secs()
    )
}

pub(super) fn booted(adb: &Path, serial: &str) -> bool {
    tools::output(
        Command::new(adb).args(["-s", serial, "shell", "getprop", "sys.boot_completed"]),
        "adb getprop",
    )
    .is_ok_and(|answer| answer.trim() == "1")
}

/// The processor a device runs, for the library to build.
pub(super) fn device_abi(serial: &str) -> Result<Abi> {
    let sdk = sdk()?;
    let product = tools::output(
        Command::new(&sdk.adb).args(["-s", serial, "shell", "getprop", "ro.product.cpu.abi"]),
        "adb getprop",
    )?;
    Abi::from_product(&product).with_context(|| format!("unknown device ABI `{}`", product.trim()))
}

/// Installs and launches the package, then follows its log until interrupted.
pub(super) fn run(apk: &Apk, device: &Device) -> Result<()> {
    let sdk = sdk()?;
    let adb = |args: &[&str]| {
        let mut command = Command::new(&sdk.adb);
        command.args(["-s", &device.serial]).args(args);
        command
    };
    // Stopped before it is replaced: installing over a running app has the system relaunch
    // the old activity, and the launch below then makes a second one in that same process, a
    // second view of the app's while the host serves one activity at a time.
    let _ = adb(&["shell", "am", "force-stop", &apk.package]).output();
    tools::run(adb(&["install", "-r", "-t"]).arg(&apk.path), "adb install")?;
    println!("launching {} on {}", apk.package, device.model);
    let component = format!("{}/{WAFT_ACTIVITY}", apk.package);
    tools::run(
        &mut adb(&["shell", "am", "start", "-W", "-n", &component]),
        "adb shell am start",
    )?;
    let Some(pid) = process_id(&sdk.adb, &device.serial, &apk.package) else {
        bail!("{} exited before its log could be followed", apk.package);
    };
    println!("press ctrl-c to stop");
    tools::run(
        &mut adb(&["logcat", "-v", "time", "--pid", &pid]),
        "adb logcat",
    )
}

/// The app's process id once it is up; `None` when it never appears.
fn process_id(adb: &Path, serial: &str, package: &str) -> Option<String> {
    for _ in 0..20 {
        if let Ok(pid) = tools::output(
            Command::new(adb).args(["-s", serial, "shell", "pidof", package]),
            "adb pidof",
        ) && !pid.trim().is_empty()
        {
            return Some(pid.trim().to_owned());
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    None
}

/// The emulators, each a device to run on once it runs, and the devices `adb` reaches.
pub fn listed_devices() -> Result<Vec<crate::devices::Device>> {
    let attached = devices()?;
    let mut listed: Vec<crate::devices::Device> = emulators()?
        .into_iter()
        .map(|avd| {
            let running = attached
                .iter()
                .find(|device| device.avd.as_deref() == Some(&avd));
            crate::devices::Device {
                name: avd.replace('_', " "),
                id: avd.clone(),
                detail: String::new(),
                platform: "android emulator",
                state: if running.is_some() {
                    "running".into()
                } else {
                    String::new()
                },
                target: match running {
                    Some(device) => Target::AndroidDevice(device.clone()),
                    None => Target::AndroidEmulator(avd),
                },
            }
        })
        .collect();
    listed.extend(
        attached
            .into_iter()
            .filter(|device| device.avd.is_none())
            .map(|device| crate::devices::Device {
                id: device.serial.clone(),
                name: device.model.clone(),
                detail: String::new(),
                platform: "android device",
                state: device.state.clone(),
                target: Target::AndroidDevice(device),
            }),
    );
    Ok(listed)
}

/// What `-d android` picks: a device `adb` reaches and can build for, else the first emulator to
/// boot. With neither, an attached device that cannot be used is the likeliest one meant, so it
/// is refused, saying why.
pub fn default_target(devices: &[crate::devices::Device]) -> Result<Option<Target>> {
    let attached: Vec<&Device> = devices
        .iter()
        .filter_map(|device| match &device.target {
            Target::AndroidDevice(device) => Some(device),
            _ => None,
        })
        .collect();
    if let Some(device) = attached.iter().find(|device| ready(device)) {
        return Ok(Some(Target::AndroidDevice((*device).clone())));
    }
    if let Some(emulator) = devices
        .iter()
        .find(|device| matches!(device.target, Target::AndroidEmulator(_)))
    {
        return Ok(Some(emulator.target.clone()));
    }
    if let Some(device) = attached.first() {
        bail!("{}", not_ready(device));
    }
    Ok(None)
}

/// Refuses a device `adb` lists but cannot build for, saying why, before anything is built.
pub fn ensure_ready(device: &Device) -> Result<()> {
    if !ready(device) {
        bail!("{}", not_ready(device));
    }
    Ok(())
}

fn ready(device: &Device) -> bool {
    device.state == "device"
}

/// Why a device `adb` lists cannot be built for, and what to do about it.
fn not_ready(device: &Device) -> String {
    let what = match device.state.as_str() {
        "unauthorized" => {
            "has not allowed USB debugging from this computer yet: unlock it and accept the prompt"
        }
        "offline" => "is offline: unplug it and plug it in again, or turn USB debugging off and on",
        other => return format!("{} is {other}", device.serial),
    };
    format!("{} {what}", device.model)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adb_listing_yields_serial_state_and_model() {
        let listing = "List of devices attached\nemulator-5554          device product:sdk_gphone64_arm64 model:sdk_gphone64_arm64 device:emu64a transport_id:1\nR5CX1234ABC            unauthorized transport_id:2\n\n";
        let devices = parse_devices(listing);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].serial, "emulator-5554");
        assert_eq!(devices[0].model, "sdk_gphone64_arm64");
        assert_eq!(devices[0].state, "device");
        assert_eq!(devices[1].state, "unauthorized");
        assert_eq!(
            devices[1].model, "R5CX1234ABC",
            "no model: the serial stands in"
        );
    }
}
