//! How an `.apk` is signed: with the debug key, `~/.android/debug.keystore` as Android Studio makes
//! it, or, for a release build of an app that names one, with its release key. A release build of an
//! app that names none is signed with the debug key, with a warning, as a new Flutter app's is.
//!
//! apksigner reads the release key's passwords itself, from where the metadata says they are, so
//! they never pass through the CLI.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::sdk::{Java, Sdk};
use crate::project::{Profile, Project, ReleaseKey, Secret};
use crate::tools;

/// The key an `.apk` is signed with.
pub enum Signing<'a> {
    /// `~/.android/debug.keystore`'s, whose password is public.
    Debug,
    /// The app's release key.
    Release(&'a ReleaseKey),
}

impl<'a> Signing<'a> {
    /// The key a `profile` build of `project` is signed with.
    pub fn for_build(project: &'a Project, profile: Profile) -> Signing<'a> {
        match (&project.android.release, profile.is_release()) {
            (Some(key), true) => Signing::Release(key),
            (None, true) => {
                eprintln!(
                    "warning: no [package.metadata.waft.android.release] key, so this release build \
                     is signed with the debug key, whose password is public; a store will not take it"
                );
                Signing::Debug
            }
            (_, false) => Signing::Debug,
        }
    }

    /// Signs `aligned` into `out` with apksigner.
    pub fn sign(&self, sdk: &Sdk, java: &Java, aligned: &Path, out: &Path) -> Result<()> {
        let mut command = java.bin("java");
        command
            .arg("-jar")
            .arg(sdk.build_tools.join("lib/apksigner.jar"))
            .arg("sign")
            .args(self.key_arguments(java)?)
            .arg("--out")
            .arg(out)
            .arg(aligned);
        tools::run(&mut command, "apksigner")
    }

    /// apksigner's arguments naming the keystore, the key, and where their passwords are.
    fn key_arguments(&self, java: &Java) -> Result<Vec<OsString>> {
        Ok(match self {
            Signing::Debug => vec![
                "--ks".into(),
                debug_keystore(java)?.into(),
                "--ks-key-alias".into(),
                "androiddebugkey".into(),
                "--ks-pass".into(),
                "pass:android".into(),
                "--key-pass".into(),
                "pass:android".into(),
            ],
            Signing::Release(key) => release_key_arguments(key),
        })
    }
}

fn release_key_arguments(key: &ReleaseKey) -> Vec<OsString> {
    let mut arguments: Vec<OsString> = vec![
        "--ks".into(),
        key.keystore.clone().into(),
        "--ks-key-alias".into(),
        key.alias.clone().into(),
        "--ks-pass".into(),
        password_source(&key.store_password),
    ];
    if let Some(key_password) = &key.key_password {
        arguments.push("--key-pass".into());
        arguments.push(password_source(key_password));
    }
    arguments
}

/// apksigner's name for where a password is: `env:NAME` or `file:PATH`.
fn password_source(secret: &Secret) -> OsString {
    let mut source = OsString::new();
    match secret {
        Secret::Env(name) => {
            source.push("env:");
            source.push(name);
        }
        Secret::File(path) => {
            source.push("file:");
            source.push(path);
        }
    }
    source
}

/// `~/.android/debug.keystore`, as Android Studio writes it.
fn debug_keystore(java: &Java) -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .context("HOME is not set")?;
    let dir = home.join(".android");
    let keystore = dir.join("debug.keystore");
    if keystore.is_file() {
        return Ok(keystore);
    }
    fs::create_dir_all(&dir)?;
    println!("creating the debug keystore at {}", keystore.display());
    tools::run(
        java.bin("keytool")
            .args(["-genkeypair", "-keystore"])
            .arg(&keystore)
            .args([
                "-storepass",
                "android",
                "-alias",
                "androiddebugkey",
                "-keypass",
                "android",
                "-keyalg",
                "RSA",
                "-keysize",
                "2048",
                "-validity",
                "10000",
                "-dname",
                "CN=Android Debug,O=Android,C=US",
            ]),
        "keytool",
    )?;
    Ok(keystore)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apksigner_reads_the_release_passwords_from_where_the_metadata_says() {
        let key = ReleaseKey {
            keystore: PathBuf::from("/app/upload.jks"),
            alias: "upload".into(),
            store_password: Secret::Env("STORE_PASSWORD".into()),
            key_password: Some(Secret::File(PathBuf::from("/secrets/key"))),
        };
        let arguments: Vec<String> = release_key_arguments(&key)
            .into_iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            arguments,
            [
                "--ks",
                "/app/upload.jks",
                "--ks-key-alias",
                "upload",
                "--ks-pass",
                "env:STORE_PASSWORD",
                "--key-pass",
                "file:/secrets/key",
            ]
        );
        let same_password = ReleaseKey {
            key_password: None,
            ..key
        };
        assert!(
            !release_key_arguments(&same_password)
                .iter()
                .any(|a| a == "--key-pass")
        );
    }
}
