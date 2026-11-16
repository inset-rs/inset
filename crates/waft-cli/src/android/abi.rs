//! The processors an `.apk` is built for: each one's Rust target, its folder in the package, and the
//! NDK's clang that links the library for it at the app's minimum API level.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

use crate::tools;

/// The processor a device runs, as Android names it, and the Rust target for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Abi {
    Arm64,
    X86_64,
    Arm,
    X86,
}

impl Abi {
    /// `ro.product.cpu.abi` as a device reports it.
    pub fn from_product(abi: &str) -> Option<Abi> {
        Some(match abi.trim() {
            "arm64-v8a" => Abi::Arm64,
            "x86_64" => Abi::X86_64,
            "armeabi-v7a" => Abi::Arm,
            "x86" => Abi::X86,
            _ => return None,
        })
    }

    /// The `lib/<abi>` directory of the package.
    pub fn dir_name(self) -> &'static str {
        match self {
            Abi::Arm64 => "arm64-v8a",
            Abi::X86_64 => "x86_64",
            Abi::Arm => "armeabi-v7a",
            Abi::X86 => "x86",
        }
    }

    pub fn triple(self) -> &'static str {
        match self {
            Abi::Arm64 => "aarch64-linux-android",
            Abi::X86_64 => "x86_64-linux-android",
            Abi::Arm => "armv7-linux-androideabi",
            Abi::X86 => "i686-linux-android",
        }
    }

    /// The NDK's clang for the target, which is named by the target and the API level it
    /// links against; only the 32-bit ARM name differs from the Rust triple.
    fn clang_name(self, min_sdk: u32) -> String {
        let target = match self {
            Abi::Arm => "armv7a-linux-androideabi",
            other => other.triple(),
        };
        format!("{target}{min_sdk}-clang")
    }
}

/// The NDK's toolchain binaries for this host.
pub(super) fn toolchain(ndk: &Path) -> PathBuf {
    let host = match std::env::consts::OS {
        "macos" => "darwin-x86_64",
        "windows" => "windows-x86_64",
        _ => "linux-x86_64",
    };
    ndk.join("toolchains/llvm/prebuilt").join(host).join("bin")
}

/// The environment that points cargo, and any C a dependency compiles, at the NDK's
/// toolchain for the target.
pub(super) fn linker_env(
    ndk: &Path,
    abi: Abi,
    min_sdk: u32,
) -> Result<Vec<(&'static str, String)>> {
    let bin = toolchain(ndk);
    let clang = bin.join(tools::exe_name(&abi.clang_name(min_sdk)));
    if !clang.is_file() {
        bail!(
            "the NDK at {} has no {} for API {min_sdk}; the NDK's toolchain may be older than the API, or for another host",
            ndk.display(),
            clang.file_name().unwrap_or_default().to_string_lossy()
        );
    }
    let clang = clang.to_string_lossy().into_owned();
    let ar = bin
        .join(tools::exe_name("llvm-ar"))
        .to_string_lossy()
        .into_owned();
    let (linker, cc, cxx, ar_key) = match abi {
        Abi::Arm64 => (
            "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER",
            "CC_aarch64_linux_android",
            "CXX_aarch64_linux_android",
            "AR_aarch64_linux_android",
        ),
        Abi::X86_64 => (
            "CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER",
            "CC_x86_64_linux_android",
            "CXX_x86_64_linux_android",
            "AR_x86_64_linux_android",
        ),
        Abi::Arm => (
            "CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER",
            "CC_armv7_linux_androideabi",
            "CXX_armv7_linux_androideabi",
            "AR_armv7_linux_androideabi",
        ),
        Abi::X86 => (
            "CARGO_TARGET_I686_LINUX_ANDROID_LINKER",
            "CC_i686_linux_android",
            "CXX_i686_linux_android",
            "AR_i686_linux_android",
        ),
    };
    Ok(vec![
        (linker, clang.clone()),
        (cc, clang.clone()),
        (cxx, format!("{clang}++")),
        (ar_key, ar),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abis_map_to_rust_targets_and_ndk_clangs() {
        assert_eq!(Abi::from_product("arm64-v8a\n"), Some(Abi::Arm64));
        assert_eq!(Abi::Arm64.triple(), "aarch64-linux-android");
        assert_eq!(Abi::Arm64.clang_name(24), "aarch64-linux-android24-clang");
        assert_eq!(Abi::Arm.clang_name(24), "armv7a-linux-androideabi24-clang");
        assert_eq!(Abi::from_product("mips"), None);
    }
}
