//! `cargo waft`: run and package an Waft app for desktop, iOS, Android, the web, and
//! WASI hosts.
//!
//! One project shape serves every host: the app lives in the library target,
//! `#[waft::main]` emits each host's entry point, and this tool turns cargo's
//! output into what the platform installs: an `.app`, an `.apk`, an installer, a
//! static web folder, a wasm component.

mod android;
mod apple;
mod cargo;
mod carried;
mod devices;
mod doctor;
mod environment;
mod file_associations;
mod icons;
mod ios;
mod linux;
mod macos;
mod new;
mod packager;
mod project;
mod tools;
mod usage_descriptions;
mod wasm;
mod web;
mod windows;

use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

use crate::devices::Target;
use crate::ios::ExportMethod;
use crate::new::Template;
use crate::project::{Profile, Project};

#[derive(Parser)]
#[command(name = "cargo-waft", bin_name = "cargo waft", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new app in a directory of that name.
    New {
        name: String,
        /// Widget kit the app starts with.
        #[arg(long, value_enum, default_value_t)]
        template: Template,
        /// Parent directory. Defaults to the current directory.
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Report which platforms this machine can build for and how to fix the gaps.
    Doctor,
    /// List the devices `run -d` accepts.
    Devices,
    /// Build and launch on a device. Defaults to this desktop.
    Run {
        /// A device from `cargo waft devices`: `macos`, `web`, `chrome`, `ios`, `android`, a simulator or emulator name, or an id.
        #[arg(short, long)]
        device: Option<String>,
        /// Optimized build.
        #[arg(long)]
        release: bool,
        /// Package to run, when the workspace has several.
        #[arg(short, long)]
        package: Option<String>,
        /// Arguments passed to the app on desktop.
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Build a distributable. Optimized unless `--debug`.
    Build {
        target: BuildTarget,
        #[arg(long)]
        debug: bool,
        #[arg(short, long)]
        package: Option<String>,
        /// iOS: build for the simulator instead of a device.
        #[arg(long)]
        simulator: bool,
        /// ipa: how the .ipa is distributed; Xcode signs it for that.
        #[arg(long, value_enum, default_value_t)]
        export_method: ExportMethod,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum BuildTarget {
    /// `.app` bundle.
    Macos,
    /// Disk image around the `.app`.
    Dmg,
    /// Folder with the executable and resources.
    Windows,
    /// WiX installer.
    Msi,
    /// NSIS installer.
    Nsis,
    /// Folder with the executable and resources.
    Linux,
    Deb,
    Appimage,
    /// `.app` signed for a device, or for the simulator with `--simulator`.
    Ios,
    /// `.ipa` around a device-signed `.app`.
    Ipa,
    /// `.apk` for 64-bit ARM devices and emulators, signed with the debug key.
    Apk,
    /// Static folder: `index.html`, `app.js`, `app_bg.wasm`.
    Web,
    /// A `wasm32-wasip2` component for a WASI host such as wapk: `<package>.wasm`.
    Wasm,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse_from(args());
    match cli.command {
        Command::New {
            name,
            template,
            path,
        } => new::create(&name, template, path.as_deref()),
        Command::Doctor => doctor::report(),
        Command::Devices => {
            devices::print(&devices::list()?);
            Ok(())
        }
        Command::Run {
            device,
            release,
            package,
            args,
        } => {
            let profile = if release {
                Profile::Release
            } else {
                Profile::Debug
            };
            run_on(device.as_deref(), profile, package.as_deref(), &args)
        }
        Command::Build {
            target,
            debug,
            package,
            simulator,
            export_method,
        } => {
            let profile = if debug {
                Profile::Debug
            } else {
                Profile::Release
            };
            build(
                target,
                profile,
                package.as_deref(),
                simulator,
                export_method,
            )
        }
    }
}

/// Cargo invokes a subcommand as `cargo-waft waft …`; direct calls omit the name.
fn args() -> Vec<OsString> {
    let mut args: Vec<OsString> = std::env::args_os().collect();
    if args.get(1).is_some_and(|arg| arg == "waft") {
        args.remove(1);
    }
    args
}

fn run_on(
    device: Option<&str>,
    profile: Profile,
    package: Option<&str>,
    args: &[String],
) -> Result<()> {
    let target = devices::resolve(device)?;
    let project = Project::load(package)?;
    match target {
        Target::Desktop if cfg!(target_os = "macos") => macos::run(&project, profile, args),
        Target::Desktop if cfg!(target_os = "windows") => windows::run(&project, profile, args),
        Target::Desktop => linux::run(&project, profile, args),
        Target::Web { browser } => web::run(&project, profile, browser),
        Target::IosSimulator(simulator) => ios::run_on_simulator(&project, profile, &simulator),
        Target::IosDevice(device) => ios::run_on_device(&project, profile, &device),
        Target::AndroidEmulator(avd) => android::run_on_emulator(&project, profile, &avd),
        Target::AndroidDevice(device) => android::run_on_device(&project, profile, &device),
    }
}

fn build(
    target: BuildTarget,
    profile: Profile,
    package: Option<&str>,
    simulator: bool,
    export_method: ExportMethod,
) -> Result<()> {
    let project = Project::load(package)?;
    let outputs = match target {
        BuildTarget::Macos => vec![macos::build_app(&project, profile)?],
        BuildTarget::Dmg => vec![macos::build_dmg(&project, profile)?],
        BuildTarget::Windows => windows::build(&project, profile, windows::Format::Folder)?,
        BuildTarget::Msi => windows::build(&project, profile, windows::Format::Msi)?,
        BuildTarget::Nsis => windows::build(&project, profile, windows::Format::Nsis)?,
        BuildTarget::Linux => linux::build(&project, profile, linux::Format::Folder)?,
        BuildTarget::Deb => linux::build(&project, profile, linux::Format::Deb)?,
        BuildTarget::Appimage => linux::build(&project, profile, linux::Format::AppImage)?,
        BuildTarget::Ios => vec![ios::build_app(&project, profile, simulator)?],
        BuildTarget::Ipa => vec![ios::build_ipa(&project, profile, export_method)?],
        BuildTarget::Apk => vec![android::build_apk(&project, profile)?],
        BuildTarget::Web => vec![web::build(&project, profile)?],
        BuildTarget::Wasm => vec![wasm::build(&project, profile)?],
    };
    for path in outputs {
        println!("{}", path.display());
    }
    Ok(())
}
