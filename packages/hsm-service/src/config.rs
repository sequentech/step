// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Settings, from `HSM_SERVICE_*` environment variables. The backend has no
//! default: an installation states which device it uses, and the service
//! refuses to start with any other.

use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{anyhow, Context, Result};
use zeroize::Zeroizing;

use crate::pkcs11::Pkcs11Backend;

pub const PREFIX: &str = "HSM_SERVICE_";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    SoftHsm,
    YubiHsm,
    AwsKms,
}

impl FromStr for Backend {
    type Err = anyhow::Error;
    fn from_str(value: &str) -> Result<Self> {
        match value {
            "softhsm" => Ok(Backend::SoftHsm),
            "yubihsm" => Ok(Backend::YubiHsm),
            "aws-kms" => Ok(Backend::AwsKms),
            other => Err(anyhow!(
                "{PREFIX}BACKEND '{other}' is not softhsm, yubihsm or aws-kms"
            )),
        }
    }
}

/// What the installation is for. Production refuses SoftHSM: a software
/// token is for tests and for demos that say they use one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deployment {
    Test,
    Demo,
    Production,
}

impl FromStr for Deployment {
    type Err = anyhow::Error;
    fn from_str(value: &str) -> Result<Self> {
        match value {
            "test" => Ok(Deployment::Test),
            "demo" => Ok(Deployment::Demo),
            "production" => Ok(Deployment::Production),
            other => Err(anyhow!(
                "{PREFIX}DEPLOYMENT '{other}' is not test, demo or production"
            )),
        }
    }
}

#[derive(Debug)]
pub struct Pkcs11Settings {
    pub backend: Pkcs11Backend,
    pub module: PathBuf,
    pub token_label: String,
    pub pin_file: PathBuf,
}

#[derive(Debug)]
pub struct Config {
    pub deployment: Deployment,
    pub pkcs11: Pkcs11Settings,
    pub listen: String,
    pub tls_cert: PathBuf,
    pub tls_key: PathBuf,
    pub client_ca: PathBuf,
    pub policy: PathBuf,
    pub state_dir: PathBuf,
}

fn read(name: &str) -> Result<String> {
    std::env::var(format!("{PREFIX}{name}")).with_context(|| format!("{PREFIX}{name} is not set"))
}

pub fn check_backend(backend: Backend, deployment: Deployment) -> Result<Pkcs11Backend> {
    match (backend, deployment) {
        (Backend::AwsKms, _) => Err(anyhow!(
            "the aws-kms backend needs a COMELEC-owned AWS account and is not available in this build"
        )),
        (Backend::SoftHsm, Deployment::Production) => {
            Err(anyhow!("SoftHSM is a software token: production deployments can't use it"))
        }
        (Backend::SoftHsm, _) => Ok(Pkcs11Backend::SoftHsm),
        (Backend::YubiHsm, _) => Ok(Pkcs11Backend::YubiHsm),
    }
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let backend: Backend = read("BACKEND")?.parse()?;
        let deployment: Deployment = read("DEPLOYMENT")?.parse()?;
        let pkcs11_backend = check_backend(backend, deployment)?;
        Ok(Config {
            deployment,
            pkcs11: Pkcs11Settings {
                backend: pkcs11_backend,
                module: read("PKCS11_MODULE")?.into(),
                token_label: read("TOKEN_LABEL")?,
                pin_file: read("PIN_FILE")?.into(),
            },
            listen: std::env::var(format!("{PREFIX}LISTEN"))
                .unwrap_or_else(|_| "0.0.0.0:8443".to_string()),
            tls_cert: read("TLS_CERT")?.into(),
            tls_key: read("TLS_KEY")?.into(),
            client_ca: read("CLIENT_CA")?.into(),
            policy: read("POLICY")?.into(),
            state_dir: read("STATE_DIR")?.into(),
        })
    }
}

/// The PIN file's first line; the file must not be readable by others.
pub fn read_pin(path: &std::path::Path) -> Result<Zeroizing<String>> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(path)
            .with_context(|| format!("{}", path.display()))?
            .permissions()
            .mode();
        if mode & 0o077 != 0 {
            return Err(anyhow!(
                "{} must not be readable by group or others",
                path.display()
            ));
        }
    }
    let text = Zeroizing::new(
        std::fs::read_to_string(path).with_context(|| format!("{}", path.display()))?,
    );
    let pin = Zeroizing::new(text.lines().next().unwrap_or_default().to_string());
    if pin.is_empty() {
        return Err(anyhow!("{} is empty", path.display()));
    }
    Ok(pin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_backend_is_chosen_for_the_installation() {
        assert!("software".parse::<Backend>().is_err());
        assert!("".parse::<Backend>().is_err());
    }

    #[test]
    fn production_refuses_softhsm_and_kms_is_unavailable() {
        assert!(check_backend(Backend::SoftHsm, Deployment::Production).is_err());
        assert_eq!(
            check_backend(Backend::SoftHsm, Deployment::Demo).unwrap(),
            Pkcs11Backend::SoftHsm
        );
        assert_eq!(
            check_backend(Backend::YubiHsm, Deployment::Production).unwrap(),
            Pkcs11Backend::YubiHsm
        );
        assert!(check_backend(Backend::AwsKms, Deployment::Production).is_err());
    }

    #[test]
    fn pin_files_must_be_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pin");
        std::fs::write(&path, "1234\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(read_pin(&path).is_err());
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(read_pin(&path).unwrap().as_str(), "1234");
    }
}
