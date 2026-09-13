//! What `run -d` can name: this desktop, a browser, a simulator, a connected iPhone.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Result, bail};

use crate::ios::{self, Simulator};
use crate::serve::Browser;
use crate::tools;

pub struct Device {
    pub id: String,
    pub name: String,
    pub platform: &'static str,
    pub state: String,
}

pub enum Target {
    Desktop,
    Web { browser: Browser },
    Simulator(Simulator),
    Device(String),
}

pub fn list() -> Result<Vec<Device>> {
    let mut devices = vec![
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
    ];
    for simulator in ios::simulators()? {
        devices.push(Device {
            id: simulator.udid,
            name: format!("{} ({})", simulator.name, simulator.runtime),
            platform: "ios simulator",
            state: simulator.state,
        });
    }
    devices.extend(connected_iphones());
    Ok(devices)
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
        return Ok(Target::Simulator(simulator.clone()));
    }
    if let Some(iphone) = connected_iphones()
        .into_iter()
        .find(|d| d.id.eq_ignore_ascii_case(wanted) || d.name.eq_ignore_ascii_case(wanted))
    {
        return Ok(Target::Device(iphone.id));
    }
    bail!("no device matches `{wanted}`; see `cargo inset devices`")
}

/// iPhones and iPads `devicectl` can reach. Empty when Xcode 15+ is absent.
fn connected_iphones() -> Vec<Device> {
    if !cfg!(target_os = "macos") || tools::which("xcrun").is_none() {
        return Vec::new();
    }
    let json_path: PathBuf =
        std::env::temp_dir().join(format!("inset-devices-{}.json", std::process::id()));
    let listed = Command::new("xcrun")
        .args(["devicectl", "list", "devices", "--json-output"])
        .arg(&json_path)
        .output();
    let Ok(listed) = listed else {
        return Vec::new();
    };
    if !listed.status.success() {
        return Vec::new();
    }
    let Ok(json) = std::fs::read_to_string(&json_path) else {
        return Vec::new();
    };
    let _ = std::fs::remove_file(&json_path);
    parse_devicectl(&json)
}

fn parse_devicectl(json: &str) -> Vec<Device> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    value
        .pointer("/result/devices")
        .and_then(|d| d.as_array())
        .into_iter()
        .flatten()
        .filter_map(|device| {
            let platform = device.pointer("/hardwareProperties/platform")?.as_str()?;
            if platform != "iOS" {
                return None;
            }
            Some(Device {
                id: device.get("identifier")?.as_str()?.to_owned(),
                name: device
                    .pointer("/deviceProperties/name")?
                    .as_str()?
                    .to_owned(),
                platform: "ios device",
                state: device
                    .pointer("/connectionProperties/tunnelState")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_owned(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devicectl_listing_keeps_ios_devices() {
        let json = r#"{"result":{"devices":[
            {"identifier":"ID1","hardwareProperties":{"platform":"iOS"},"deviceProperties":{"name":"Jane's iPhone"},"connectionProperties":{"tunnelState":"connected"}},
            {"identifier":"ID2","hardwareProperties":{"platform":"macOS"},"deviceProperties":{"name":"Mac"}}]}}"#;
        let devices = parse_devicectl(json);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].id, "ID1");
        assert_eq!(devices[0].state, "connected");
    }
}
