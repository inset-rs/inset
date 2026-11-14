//! The libraries' native code for the processor the package is built for: each AAR's
//! `jni/<abi>/*.so`, which the package carries under `lib/<abi>/` beside the app's own library.
//!
//! Only the package's processor's are taken. A library's for other processors would make the
//! system pick a processor the app's own library is not built for, which is why a Gradle build
//! filters its ABIs too. A library with native code but none for this processor is refused rather
//! than packaged without it, and two files at one path are refused, as Gradle refuses them.

use std::path::PathBuf;

use anyhow::{Result, bail};

use super::library::AndroidLibrary;

/// The native libraries of `libraries` for `abi`, as package entries and the files they hold;
/// `app_library` is the file name of the app's own, `lib<name>.so`.
pub fn for_abi(
    libraries: &[AndroidLibrary],
    abi: &str,
    app_library: &str,
) -> Result<Vec<(String, PathBuf)>> {
    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    let mut owners: Vec<(&str, &str)> = vec![(app_library, "the app")];
    for library in libraries {
        if library.native_libraries.is_empty() {
            continue;
        }
        let for_this_abi: Vec<_> = library
            .native_libraries
            .iter()
            .filter(|native| native.abi == abi)
            .collect();
        if for_this_abi.is_empty() {
            let mut abis: Vec<&str> = library
                .native_libraries
                .iter()
                .map(|native| native.abi.as_str())
                .collect();
            abis.dedup();
            bail!(
                "{}: it has native libraries for {} and none for {abi}, the processor the package is built for",
                library.name,
                abis.join(", ")
            );
        }
        for native in for_this_abi {
            if let Some((_, owner)) = owners
                .iter()
                .find(|(file_name, _)| *file_name == native.file_name)
            {
                bail!(
                    "2 files found with path 'lib/{abi}/{}', from {owner} and {}",
                    native.file_name,
                    library.name
                );
            }
            owners.push((&native.file_name, &library.name));
            entries.push((
                format!("lib/{abi}/{}", native.file_name),
                native.path.clone(),
            ));
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::super::library::NativeLibrary;
    use super::*;

    fn library(name: &str, natives: &[(&str, &str)]) -> AndroidLibrary {
        AndroidLibrary {
            name: name.to_owned(),
            class_jars: Vec::new(),
            manifest: None,
            resources: None,
            native_libraries: natives
                .iter()
                .map(|(abi, file_name)| NativeLibrary {
                    abi: (*abi).to_owned(),
                    file_name: (*file_name).to_owned(),
                    path: PathBuf::from(format!("{name}/jni/{abi}/{file_name}")),
                })
                .collect(),
        }
    }

    #[test]
    fn only_the_packages_processors_libraries_are_taken() {
        let libraries = [library(
            "a.aar",
            &[("arm64-v8a", "liba.so"), ("x86_64", "liba.so")],
        )];
        let entries = for_abi(&libraries, "arm64-v8a", "libapp.so").unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "lib/arm64-v8a/liba.so");
    }

    #[test]
    fn a_library_without_this_processors_code_is_refused() {
        let libraries = [library("a.aar", &[("x86_64", "liba.so")])];
        let refused = for_abi(&libraries, "arm64-v8a", "libapp.so")
            .unwrap_err()
            .to_string();
        assert!(
            refused.contains("a.aar") && refused.contains("x86_64"),
            "{refused}"
        );
    }

    #[test]
    fn two_files_at_one_path_are_refused() {
        let libraries = [
            library("a.aar", &[("arm64-v8a", "libshared.so")]),
            library("b.aar", &[("arm64-v8a", "libshared.so")]),
        ];
        let refused = for_abi(&libraries, "arm64-v8a", "libapp.so")
            .unwrap_err()
            .to_string();
        assert!(
            refused.contains("a.aar") && refused.contains("b.aar"),
            "{refused}"
        );
        let taking_the_apps = [library("c.aar", &[("arm64-v8a", "libapp.so")])];
        assert!(
            for_abi(&taking_the_apps, "arm64-v8a", "libapp.so")
                .unwrap_err()
                .to_string()
                .contains("the app")
        );
    }
}
