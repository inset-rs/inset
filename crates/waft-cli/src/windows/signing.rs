//! How the Windows installers, and the executables in them, are signed: cargo-packager signs each
//! with `signtool` and a certificate in the machine's store, by its thumbprint, or with a command of
//! the developer's; with neither, they are left unsigned, which SmartScreen warns of.

use anyhow::{Result, bail};
use cargo_packager::config::WindowsConfig;

use crate::project::WindowsMetadata;

/// What the Windows packages are signed with.
#[derive(Debug, PartialEq, Eq)]
pub enum WindowsSigning {
    Unsigned,
    /// A certificate in the machine's store, which `signtool` finds by its SHA-1 thumbprint.
    Certificate {
        thumbprint: String,
    },
    /// A command each file is signed with, `%1` standing for its path.
    Command(String),
}

impl WindowsSigning {
    /// The signing the metadata names; both a certificate and a command is refused.
    pub fn of(metadata: &WindowsMetadata) -> Result<WindowsSigning> {
        Ok(
            match (&metadata.certificate_thumbprint, &metadata.sign_command) {
                (Some(_), Some(_)) => bail!(
                    "[package.metadata.waft.windows] names both `certificate_thumbprint` and `sign_command`; keep the one to sign with"
                ),
                (Some(thumbprint), None) => WindowsSigning::Certificate {
                    thumbprint: thumbprint.clone(),
                },
                (None, Some(command)) => {
                    if !command.contains("%1") {
                        bail!(
                            "`sign_command` has no `%1`, which stands for the path of the file it signs"
                        );
                    }
                    WindowsSigning::Command(command.clone())
                }
                (None, None) => WindowsSigning::Unsigned,
            },
        )
    }

    /// cargo-packager's Windows configuration for this signing, timestamped by `timestamp_url`.
    pub fn packager_config(&self, timestamp_url: Option<&str>) -> Option<WindowsConfig> {
        let mut config = WindowsConfig::new().digest_algorithm("sha256");
        config.timestamp_url = timestamp_url.map(str::to_owned);
        match self {
            WindowsSigning::Unsigned => return None,
            WindowsSigning::Certificate { thumbprint } => {
                config.certificate_thumbprint = Some(thumbprint.clone());
            }
            WindowsSigning::Command(command) => config.sign_command = Some(command.clone()),
        }
        Some(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(thumbprint: Option<&str>, command: Option<&str>) -> WindowsMetadata {
        WindowsMetadata {
            certificate_thumbprint: thumbprint.map(str::to_owned),
            sign_command: command.map(str::to_owned),
            timestamp_url: None,
        }
    }

    #[test]
    fn the_metadata_names_one_way_to_sign_or_none() {
        assert_eq!(
            WindowsSigning::of(&metadata(None, None)).unwrap(),
            WindowsSigning::Unsigned
        );
        assert_eq!(
            WindowsSigning::of(&metadata(Some("AB12"), None)).unwrap(),
            WindowsSigning::Certificate {
                thumbprint: "AB12".into()
            }
        );
        assert!(WindowsSigning::of(&metadata(Some("AB12"), Some("sign %1"))).is_err());
        assert!(WindowsSigning::of(&metadata(None, Some("sign"))).is_err());
    }

    #[test]
    fn a_certificate_reaches_cargo_packager_with_sha256_and_the_timestamp_server() {
        let signing = WindowsSigning::Certificate {
            thumbprint: "AB12".into(),
        };
        let config = signing
            .packager_config(Some("http://timestamp.example"))
            .unwrap();
        assert_eq!(config.certificate_thumbprint.as_deref(), Some("AB12"));
        assert_eq!(config.digest_algorithm.as_deref(), Some("sha256"));
        assert_eq!(
            config.timestamp_url.as_deref(),
            Some("http://timestamp.example")
        );
        assert!(WindowsSigning::Unsigned.packager_config(None).is_none());
    }
}
