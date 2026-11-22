// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What `trustee start` runs with: two flags, each falling back to its
//! variable, and the environment for the rest.

use std::env::{self, VarError};
use std::path::PathBuf;

use anyhow::{anyhow, bail, Context as _, Result};

use crate::login::{Credentials, NATIVE_TRUSTEE_CLIENT};

const B4_URL: &str = "B4_URL";
const TRUSTEE_CONFIG_PATH: &str = "TRUSTEE_CONFIG_PATH";
const TRUSTEE_NAME: &str = "TRUSTEE_NAME";
const TRUSTEE_PASSWORD: &str = "TRUSTEE_PASSWORD";
const TRUSTEE_PLATFORM_URL: &str = "TRUSTEE_PLATFORM_URL";
const TRUSTEE_KEYCLOAK_URL: &str = "TRUSTEE_KEYCLOAK_URL";
const TRUSTEE_KEYCLOAK_REALM: &str = "TRUSTEE_KEYCLOAK_REALM";
const TRUSTEE_KEYCLOAK_CLIENT_ID: &str = "TRUSTEE_KEYCLOAK_CLIENT_ID";
const TRUSTEE_KEYCLOAK_CLIENT_SECRET: &str = "TRUSTEE_KEYCLOAK_CLIENT_SECRET";

const TRUSTEE_OTP_FILE: &str = "TRUSTEE_OTP_FILE";
const TRUSTEE_KEYCLOAK_CA_FILE: &str = "TRUSTEE_KEYCLOAK_CA_FILE";

const TRUSTEE_PASSWORD_FILE: &str = "TRUSTEE_PASSWORD_FILE";
const TRUSTEE_KEYCLOAK_CLIENT_SECRET_FILE: &str =
    "TRUSTEE_KEYCLOAK_CLIENT_SECRET_FILE";

const B4_URL_FLAG: &str = "--b4-url";
const TRUSTEE_CONFIG_FLAG: &str = "--trustee-config";

pub(crate) struct Config {
    /// The board service.
    pub(crate) b4_url: String,
    /// The trustee's keys file.
    pub(crate) keys_file: PathBuf,
    /// The trustee's platform username, and the name braid stamps into the
    /// messages it signs.
    pub(crate) trustee_name: String,
    pub(crate) platform_url: String,
    pub(crate) credentials: Credentials,
}

impl Config {
    /// The configuration of `trustee start`, given its two flags. Every value
    /// but the client secret is required; an empty one counts as missing.
    pub(crate) fn read(
        b4_url: Option<String>,
        keys_file: Option<PathBuf>,
    ) -> Result<Config> {
        let b4_url = match b4_url {
            Some(url) if !url.is_empty() => url,
            Some(_) | None => fallback(B4_URL_FLAG, B4_URL)?,
        };
        let keys_file: PathBuf = match keys_file {
            Some(path) if !path.as_os_str().is_empty() => path,
            Some(_) | None => {
                fallback(TRUSTEE_CONFIG_FLAG, TRUSTEE_CONFIG_PATH)?.into()
            }
        };
        let trustee_name = required(TRUSTEE_NAME)?;
        let client_id = required(TRUSTEE_KEYCLOAK_CLIENT_ID)?;
        let client_secret = private_setting(
            TRUSTEE_KEYCLOAK_CLIENT_SECRET,
            TRUSTEE_KEYCLOAK_CLIENT_SECRET_FILE,
        )?;
        let keycloak_url = required(TRUSTEE_KEYCLOAK_URL)?;
        let otp_file = optional(TRUSTEE_OTP_FILE)?.map(PathBuf::from);
        let keycloak_ca_file =
            optional(TRUSTEE_KEYCLOAK_CA_FILE)?.map(PathBuf::from);
        if client_id == NATIVE_TRUSTEE_CLIENT {
            if client_secret.is_none()
                || otp_file.is_none()
                || keycloak_ca_file.is_none()
            {
                bail!("the native trustee client requires its private client secret, OTP file and site CA");
            }
            validate_native_keycloak_url(&keycloak_url)?;
        }
        Ok(Config {
            b4_url,
            keys_file,
            platform_url: required(TRUSTEE_PLATFORM_URL)?,
            credentials: Credentials {
                keycloak_url,
                realm: required(TRUSTEE_KEYCLOAK_REALM)?,
                client_id,
                client_secret,
                username: trustee_name.clone(),
                password: private_setting(
                    TRUSTEE_PASSWORD,
                    TRUSTEE_PASSWORD_FILE,
                )?
                .ok_or_else(|| anyhow!("trustee password is not configured"))?,
                otp_file,
                keycloak_ca_file: optional(TRUSTEE_KEYCLOAK_CA_FILE)?
                    .map(PathBuf::from),
            },
            trustee_name,
        })
    }
}

/// The variable a flag falls back to when it is absent or empty.
fn fallback(flag: &str, variable: &str) -> Result<String> {
    required(variable).with_context(|| format!("{flag} was not given"))
}

fn required(variable: &str) -> Result<String> {
    optional(variable)?.ok_or_else(|| anyhow!("{variable} is not set"))
}

fn optional(variable: &str) -> Result<Option<String>> {
    match env::var(variable) {
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) => Ok(Some(value)),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => bail!("{variable} is not valid UTF-8"),
    }
}

fn validate_native_keycloak_url(value: &str) -> Result<()> {
    let url = reqwest::Url::parse(value)
        .map_err(|_| anyhow!("invalid native trustee Keycloak URL"))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        bail!("the native trustee Keycloak URL must be HTTPS without user information, query or fragment");
    }
    Ok(())
}

fn private_setting(
    variable: &str,
    file_variable: &str,
) -> Result<Option<String>> {
    use std::io::Read as _;
    let direct = optional(variable)?;
    let path = optional(file_variable)?;
    match (direct, path) {
        (Some(_), Some(_)) => {
            bail!("configure {variable} or {file_variable}, not both")
        }
        (direct, None) => Ok(direct),
        (None, Some(path)) => {
            let mut secret = String::new();
            std::fs::File::open(path)
                .and_then(|file| file.take(4097).read_to_string(&mut secret))
                .map_err(|_| {
                    anyhow!("cannot read private setting {file_variable}")
                })?;
            let value = secret.trim_end_matches(['\r', '\n']);
            if value.is_empty()
                || secret.len() > 4096
                || value.lines().count() != 1
            {
                bail!("invalid private setting {file_variable}");
            }
            Ok(Some(value.to_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate_native_keycloak_url;

    #[test]
    fn native_auth_accepts_https_but_not_plaintext_or_url_credentials() {
        assert!(
            validate_native_keycloak_url("https://login.site.test:15210")
                .is_ok()
        );
        for value in [
            "http://login.site.test",
            "https://u:p@login.site.test",
            "https://login.site.test?unsafe=1",
            "https://login.site.test#unsafe",
        ] {
            assert!(validate_native_keycloak_url(value).is_err());
        }
    }
}
