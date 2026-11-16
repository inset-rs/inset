//! The simulators and the iPhones and iPads `run -d` can name.

use std::path::PathBuf;
use std::process::Command;

use anyhow::Result;

use crate::devices::Device;
use crate::tools;

#[derive(Clone, Debug)]
pub struct Simulator {
    pub udid: String,
    pub name: String,
    pub state: String,
    /// `iOS 18.6`.
    pub runtime: String,
}

/// Available iOS simulators, newest runtime first.
pub fn simulators() -> Result<Vec<Simulator>> {
    if !cfg!(target_os = "macos") || tools::which("xcrun").is_none() {
        return Ok(Vec::new());
    }
    let json = tools::output(
        Command::new("xcrun").args(["simctl", "list", "-j", "devices", "available"]),
        "xcrun simctl list",
    )?;
    Ok(parse_simulators(&json))
}

fn parse_simulators(json: &str) -> Vec<Simulator> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let Some(devices) = value.get("devices").and_then(|d| d.as_object()) else {
        return Vec::new();
    };
    let mut runtimes: Vec<(Vec<u32>, String, &serde_json::Value)> = devices
        .iter()
        .filter_map(|(runtime, list)| {
            let (family, version) = runtime.rsplit_once('.')?.1.split_once('-')?;
            if family != "iOS" {
                return None;
            }
            let numbers: Vec<u32> = version.split('-').filter_map(|n| n.parse().ok()).collect();
            let label = format!("iOS {}", version.replace('-', "."));
            Some((numbers, label, list))
        })
        .collect();
    runtimes.sort_by(|a, b| b.0.cmp(&a.0));
    runtimes
        .into_iter()
        .flat_map(|(_, runtime, list)| {
            list.as_array()
                .into_iter()
                .flatten()
                .filter_map(move |device| {
                    Some(Simulator {
                        udid: device.get("udid")?.as_str()?.to_owned(),
                        name: device.get("name")?.as_str()?.to_owned(),
                        state: device.get("state")?.as_str()?.to_owned(),
                        runtime: runtime.clone(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The simulator `-d` names, or the booted one, or the newest iPhone.
pub fn pick_simulator<'a>(
    simulators: &'a [Simulator],
    wanted: Option<&str>,
) -> Option<&'a Simulator> {
    match wanted {
        Some(wanted) => {
            let lower = wanted.to_lowercase();
            simulators
                .iter()
                .find(|s| s.udid.eq_ignore_ascii_case(wanted))
                .or_else(|| simulators.iter().find(|s| s.name.to_lowercase() == lower))
                .or_else(|| {
                    simulators
                        .iter()
                        .find(|s| s.name.to_lowercase().starts_with(&lower))
                })
        }
        None => simulators
            .iter()
            .find(|s| s.state == "Booted")
            .or_else(|| simulators.iter().find(|s| s.name.starts_with("iPhone")))
            .or_else(|| simulators.first()),
    }
}

/// An iPhone or iPad `devicectl` reaches.
#[derive(Clone, Debug)]
pub struct IosDevice {
    /// `devicectl`'s identifier, which it installs and launches by.
    pub id: String,
    /// The hardware's UDID, which Apple registers a device and a profile lists it by.
    pub udid: String,
    pub name: String,
    /// `devicectl`'s tunnel state: `connected`, `disconnected` until it is reached, `unavailable`.
    pub state: String,
}

/// Everything iOS that `run -d` can name: the simulators, and the devices attached now.
pub fn listed_devices() -> Result<Vec<Device>> {
    let mut devices: Vec<Device> = simulators()?
        .into_iter()
        .map(|simulator| Device {
            id: simulator.udid,
            name: format!("{} ({})", simulator.name, simulator.runtime),
            platform: "ios simulator",
            state: simulator.state,
        })
        .collect();
    devices.extend(connected_iphones().into_iter().map(|device| Device {
        id: device.id,
        name: device.name,
        platform: "ios device",
        state: device.state,
    }));
    Ok(devices)
}

/// The iPhones and iPads `devicectl` knows of, simulators aside. Empty when Xcode 15+ is absent.
pub fn connected_iphones() -> Vec<IosDevice> {
    if !cfg!(target_os = "macos") || tools::which("xcrun").is_none() {
        return Vec::new();
    }
    let json_path: PathBuf =
        std::env::temp_dir().join(format!("waft-devices-{}.json", std::process::id()));
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

fn parse_devicectl(json: &str) -> Vec<IosDevice> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    value
        .pointer("/result/devices")
        .and_then(|d| d.as_array())
        .into_iter()
        .flatten()
        .filter_map(|device| {
            let hardware = device.get("hardwareProperties")?;
            let platform = hardware.get("platform")?.as_str()?;
            let reality = hardware.get("reality")?.as_str()?;
            if platform != "iOS" || reality != "physical" {
                return None;
            }
            Some(IosDevice {
                id: device.get("identifier")?.as_str()?.to_owned(),
                udid: hardware.get("udid")?.as_str()?.to_owned(),
                name: device
                    .pointer("/deviceProperties/name")?
                    .as_str()?
                    .to_owned(),
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
    fn devicectl_listing_keeps_physical_ios_devices_with_their_udid() {
        let json = r#"{"result":{"devices":[
            {"identifier":"ID1","hardwareProperties":{"platform":"iOS","reality":"physical","udid":"00008160-0001"},"deviceProperties":{"name":"Jane's iPhone"},"connectionProperties":{"tunnelState":"connected"}},
            {"identifier":"ID2","hardwareProperties":{"platform":"macOS","reality":"physical","udid":"M"},"deviceProperties":{"name":"Mac"}},
            {"identifier":"ID3","hardwareProperties":{"platform":"iOS","reality":"simulated","udid":"ID3"},"deviceProperties":{"name":"iPhone 16"}}]}}"#;
        let devices = parse_devicectl(json);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].id, "ID1");
        assert_eq!(devices[0].udid, "00008160-0001");
        assert_eq!(devices[0].state, "connected");
    }

    #[test]
    fn simulators_come_newest_runtime_first_and_ios_only() {
        let json = r#"{"devices":{
            "com.apple.CoreSimulator.SimRuntime.iOS-17-5":[{"name":"iPhone 15","udid":"A","state":"Shutdown"}],
            "com.apple.CoreSimulator.SimRuntime.tvOS-18-0":[{"name":"Apple TV","udid":"T","state":"Shutdown"}],
            "com.apple.CoreSimulator.SimRuntime.iOS-18-6":[{"name":"iPhone 16","udid":"B","state":"Booted"}]}}"#;
        let list = parse_simulators(json);
        assert_eq!(
            list.iter().map(|s| s.udid.as_str()).collect::<Vec<_>>(),
            ["B", "A"]
        );
        assert_eq!(list[0].runtime, "iOS 18.6");
        assert_eq!(pick_simulator(&list, None).unwrap().udid, "B");
        assert_eq!(pick_simulator(&list, Some("iphone 15")).unwrap().udid, "A");
        assert_eq!(pick_simulator(&list, Some("iPhone")).unwrap().udid, "B");
    }
}
