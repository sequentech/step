// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What `trustee start` runs with: two flags, each falling back to its
//! variable, and the environment for the rest.

use std::env::{self, VarError};
use std::path::PathBuf;

use anyhow::{anyhow, bail, Context as _, Result};

use crate::login::Credentials;

const B4_URL: &str = "B4_URL";
const TRUSTEE_CONFIG_PATH: &str = "TRUSTEE_CONFIG_PATH";
const TRUSTEE_NAME: &str = "TRUSTEE_NAME";
const TRUSTEE_PASSWORD: &str = "TRUSTEE_PASSWORD";
const TRUSTEE_HARVEST_URL: &str = "TRUSTEE_HARVEST_URL";
const TRUSTEE_KEYCLOAK_URL: &str = "TRUSTEE_KEYCLOAK_URL";
const TRUSTEE_KEYCLOAK_REALM: &str = "TRUSTEE_KEYCLOAK_REALM";
const TRUSTEE_KEYCLOAK_CLIENT_ID: &str = "TRUSTEE_KEYCLOAK_CLIENT_ID";
const TRUSTEE_KEYCLOAK_CLIENT_SECRET: &str = "TRUSTEE_KEYCLOAK_CLIENT_SECRET";

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
    pub(crate) harvest_url: String,
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
        Ok(Config {
            b4_url,
            keys_file,
            harvest_url: required(TRUSTEE_HARVEST_URL)?,
            credentials: Credentials {
                keycloak_url: required(TRUSTEE_KEYCLOAK_URL)?,
                realm: required(TRUSTEE_KEYCLOAK_REALM)?,
                client_id: required(TRUSTEE_KEYCLOAK_CLIENT_ID)?,
                client_secret: optional(TRUSTEE_KEYCLOAK_CLIENT_SECRET)?,
                username: trustee_name.clone(),
                password: required(TRUSTEE_PASSWORD)?,
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
