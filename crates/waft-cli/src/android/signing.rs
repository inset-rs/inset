//! How an `.apk` is signed: with the debug key, `~/.android/debug.keystore` as Android Studio makes
//! it, or, for a release build, with a release key: the one the environment names, as CI and a
//! publisher's machine name it (`WAFT_ANDROID_KEYSTORE`, `WAFT_ANDROID_KEY_ALIAS`,
//! `WAFT_ANDROID_STORE_PASSWORD_FILE`, and `WAFT_ANDROID_KEY_PASSWORD_FILE` where the key's own
//! password differs), as Apple signing reads `APPLE_SIGNING_IDENTITY`; else the one the app's
//! metadata names. A release build with neither is signed with the debug key, with a warning, as a
//! new Flutter app's is.
//!
//! apksigner reads the release key's passwords itself, from the files the environment names or
//! from where the metadata says they are, so they never pass through the CLI.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::sdk::{Java, Sdk};
use crate::project::{Profile, Project, ReleaseKey, Secret};
use crate::tools;

/// The key an `.apk` is signed with.
pub enum Signing {
    /// `~/.android/debug.keystore`'s, whose password is public.
    Debug,
    /// A release key, from the environment or the app's metadata.
    Release(ReleaseKey),
}

/// The variables that name a release key: the keystore, the key's alias, and the file holding the
/// keystore's password; the key's own password's file is optional.
const RELEASE_KEY_VARIABLES: [&str; 3] = [
    "WAFT_ANDROID_KEYSTORE",
    "WAFT_ANDROID_KEY_ALIAS",
    "WAFT_ANDROID_STORE_PASSWORD_FILE",
];

impl Signing {
    /// The key a `profile` build of `project` is signed with.
    pub fn for_build(project: &Project, profile: Profile) -> Result<Signing> {
        if !profile.is_release() {
            return Ok(Signing::Debug);
        }
        if let Some(key) = release_key_from(crate::environment::var_os)? {
            return Ok(Signing::Release(existing(key)?));
        }
        Ok(match &project.android.release {
            Some(key) => Signing::Release(key.clone()),
            None => {
                eprintln!(
                    "warning: no release key, in WAFT_ANDROID_KEYSTORE and its variables or as \
                     [package.metadata.waft.android.release], so this release build is signed \
                     with the debug key, whose password is public; a store will not take it"
                );
                Signing::Debug
            }
        })
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

/// The release key the variables `lookup` finds name; none where it finds none of them, and an
/// error naming what is missing where it finds some.
fn release_key_from(lookup: impl Fn(&str) -> Option<OsString>) -> Result<Option<ReleaseKey>> {
    let found = RELEASE_KEY_VARIABLES.map(&lookup);
    let missing: Vec<&str> = RELEASE_KEY_VARIABLES
        .iter()
        .zip(&found)
        .filter(|(_, value)| value.is_none())
        .map(|(name, _)| *name)
        .collect();
    match found {
        [None, None, None] => Ok(None),
        [Some(keystore), Some(alias), Some(store_password)] => Ok(Some(ReleaseKey {
            keystore: keystore.into(),
            alias: alias.to_string_lossy().into_owned(),
            store_password: Secret::File(store_password.into()),
            key_password: lookup("WAFT_ANDROID_KEY_PASSWORD_FILE")
                .map(|file| Secret::File(file.into())),
        })),
        _ => bail!(
            "a release key from the environment also needs {}",
            missing.join(" and ")
        ),
    }
}

/// `key`, once its keystore and the files holding its passwords are there.
fn existing(key: ReleaseKey) -> Result<ReleaseKey> {
    let password_files = [Some(&key.store_password), key.key_password.as_ref()]
        .into_iter()
        .flatten()
        .filter_map(|secret| match secret {
            Secret::File(file) => Some(file),
            Secret::Env(_) => None,
        });
    for file in std::iter::once(&key.keystore).chain(password_files) {
        if !file.is_file() {
            bail!("the release key's {} is not a file", file.display());
        }
    }
    Ok(key)
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
    use std::collections::HashMap;

    use super::*;

    fn lookup(set: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let set: HashMap<String, OsString> = set
            .iter()
            .map(|(name, value)| ((*name).to_owned(), OsString::from(value)))
            .collect();
        move |name| set.get(name).cloned()
    }

    #[test]
    fn the_environment_names_a_release_key_by_its_keystore_alias_and_password_file() {
        let key = release_key_from(lookup(&[
            ("WAFT_ANDROID_KEYSTORE", "/keys/release.jks"),
            ("WAFT_ANDROID_KEY_ALIAS", "waos"),
            ("WAFT_ANDROID_STORE_PASSWORD_FILE", "/keys/password"),
        ]))
        .unwrap()
        .expect("a key");
        let arguments: Vec<String> = release_key_arguments(&key)
            .into_iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            arguments,
            [
                "--ks",
                "/keys/release.jks",
                "--ks-key-alias",
                "waos",
                "--ks-pass",
                "file:/keys/password",
            ]
        );
    }

    #[test]
    fn no_variable_is_no_key_and_some_name_what_is_missing() {
        assert!(release_key_from(lookup(&[])).unwrap().is_none());
        let error = release_key_from(lookup(&[("WAFT_ANDROID_KEYSTORE", "/keys/release.jks")]))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("WAFT_ANDROID_KEY_ALIAS and WAFT_ANDROID_STORE_PASSWORD_FILE"),
            "{error}"
        );
    }

    #[test]
    fn a_key_whose_keystore_is_not_there_is_refused() {
        let key = release_key_from(lookup(&[
            ("WAFT_ANDROID_KEYSTORE", "/nowhere/release.jks"),
            ("WAFT_ANDROID_KEY_ALIAS", "waos"),
            ("WAFT_ANDROID_STORE_PASSWORD_FILE", "/nowhere/password"),
        ]))
        .unwrap()
        .unwrap();
        assert!(existing(key).is_err());
    }

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
