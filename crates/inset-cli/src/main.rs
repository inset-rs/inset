//! `cargo inset`: run and package an Inset app for desktop, iOS, the web, and WASI hosts.
//!
//! One project shape serves every host: the app lives in the library target,
//! `#[inset::main]` emits each host's entry point, and this tool turns cargo's
//! output into what the platform installs: an `.app`, an installer, a static
//! web folder, a wasm component.

mod cargo;
mod desktop;
mod devices;
mod doctor;
mod icons;
mod ios;
mod new;
mod project;
mod serve;
mod tools;
mod wasm;
mod web;

use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

use crate::desktop::Format;
use crate::devices::Target;
use crate::ios::Destination;
use crate::new::Template;
use crate::project::{Profile, Project};

#[derive(Parser)]
#[command(name = "cargo-inset", bin_name = "cargo inset", version, about)]
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
        /// A device from `cargo inset devices`: `macos`, `web`, `chrome`, `ios`, a simulator name, or an id.
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
        } => {
            let profile = if debug {
                Profile::Debug
            } else {
                Profile::Release
            };
            build(target, profile, package.as_deref(), simulator)
        }
    }
}

/// Cargo invokes a subcommand as `cargo-inset inset …`; direct calls omit the name.
fn args() -> Vec<OsString> {
    let mut args: Vec<OsString> = std::env::args_os().collect();
    if args.get(1).is_some_and(|arg| arg == "inset") {
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
        Target::Desktop if cfg!(target_os = "macos") => {
            desktop::run_bundle(&project, profile, args)
        }
        Target::Desktop => cargo::run(&project, profile, args),
        Target::Web { browser } => {
            let out = web::build(&project, profile)?;
            serve::serve_and_open(out, browser)
        }
        Target::Simulator(simulator) => {
            let app = ios::build(&project, profile, Destination::Simulator)?;
            ios::run_simulator(&app, &simulator)
        }
        Target::Device(id) => {
            let app = ios::build(&project, profile, Destination::Device)?;
            ios::run_device(&app, &id)
        }
    }
}

fn build(
    target: BuildTarget,
    profile: Profile,
    package: Option<&str>,
    simulator: bool,
) -> Result<()> {
    let project = Project::load(package)?;
    let outputs = match target {
        BuildTarget::Macos => desktop::build(&project, profile, Format::App)?,
        BuildTarget::Dmg => desktop::build(&project, profile, Format::Dmg)?,
        BuildTarget::Windows | BuildTarget::Linux => {
            desktop::build(&project, profile, Format::Folder)?
        }
        BuildTarget::Msi => desktop::build(&project, profile, Format::Msi)?,
        BuildTarget::Nsis => desktop::build(&project, profile, Format::Nsis)?,
        BuildTarget::Deb => desktop::build(&project, profile, Format::Deb)?,
        BuildTarget::Appimage => desktop::build(&project, profile, Format::AppImage)?,
        BuildTarget::Ios => {
            let destination = if simulator {
                Destination::Simulator
            } else {
                Destination::Device
            };
            vec![ios::build(&project, profile, destination)?.path]
        }
        BuildTarget::Ipa => {
            let app = ios::build(&project, profile, Destination::Device)?;
            vec![ios::ipa(&app, &project.out_dir("ipa", profile))?]
        }
        BuildTarget::Web => vec![web::build(&project, profile)?],
        BuildTarget::Wasm => vec![wasm::build(&project, profile)?],
    };
    for path in outputs {
        println!("{}", path.display());
    }
    Ok(())
}
