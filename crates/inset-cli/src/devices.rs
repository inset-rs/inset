//! What `run -d` can name: this desktop, a browser, a simulator, a connected iPhone, an
//! Android emulator, a connected Android device.

use anyhow::{Result, bail};

use crate::android;
use crate::ios::{self, Simulator};
use crate::serve::Browser;

pub struct Device {
    pub id: String,
    pub name: String,
    pub platform: &'static str,
    pub state: String,
}

pub enum Target {
    Desktop,
    Web {
        browser: Browser,
    },
    IosSimulator(Simulator),
    /// An iPhone or iPad `devicectl` reaches, by identifier.
    IosDevice(String),
    /// An Android emulator to boot, by AVD name.
    AndroidEmulator(String),
    /// An Android device or emulator `adb` already reaches.
    AndroidDevice(android::Device),
}

pub fn list() -> Result<Vec<Device>> {
    let mut devices = this_desktop_and_browsers();
    devices.extend(ios::listed_devices()?);
    devices.extend(android::listed_devices()?);
    Ok(devices)
}

/// The targets that need nothing attached: the machine this runs on, and its browsers.
fn this_desktop_and_browsers() -> Vec<Device> {
    vec![
        Device {
            id: std::env::consts::OS.to_owned(),
            name: "this desktop".to_owned(),
            platform: "desktop",
            state: String::new(),
        },
        Device {
            id: "web".into(),
            name: "default browser".into(),
            platform: "web",
            state: String::new(),
        },
        Device {
            id: "chrome".into(),
            name: "Google Chrome".into(),
            platform: "web",
            state: String::new(),
        },
    ]
}

pub fn print(devices: &[Device]) {
    let width = devices.iter().map(|d| d.id.len()).max().unwrap_or(0);
    for device in devices {
        println!(
            "{:width$}  {:14} {}  {}",
            device.id, device.platform, device.name, device.state
        );
    }
}

pub fn resolve(wanted: Option<&str>) -> Result<Target> {
    let Some(wanted) = wanted else {
        return Ok(Target::Desktop);
    };
    match wanted {
        "macos" | "windows" | "linux" | "desktop" => return Ok(Target::Desktop),
        "web" => {
            return Ok(Target::Web {
                browser: Browser::Default,
            });
        }
        "chrome" => {
            return Ok(Target::Web {
                browser: Browser::Chrome,
            });
        }
        _ => {}
    }
    let simulators = ios::simulators()?;
    let simulator = match wanted {
        "ios" => ios::pick_simulator(&simulators, None),
        other => ios::pick_simulator(&simulators, Some(other)),
    };
    if let Some(simulator) = simulator {
        return Ok(Target::IosSimulator(simulator.clone()));
    }
    if let Some(iphone) = ios::connected_iphones()
        .into_iter()
        .find(|d| d.id.eq_ignore_ascii_case(wanted) || d.name.eq_ignore_ascii_case(wanted))
    {
        return Ok(Target::IosDevice(iphone.id));
    }
    if let Some(target) = android::target_for(wanted)? {
        return Ok(target);
    }
    bail!("no device matches `{wanted}`; see `cargo inset devices`")
}
