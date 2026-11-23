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
    use super::*;
    use std::process::Command;

    const CASE: &str = "TRUSTEE_TEST_CONFIG_CASE";

    #[test]
    fn start_config_uses_only_the_requested_credentials_and_requires_valid_settings(
    ) {
        let tmp = tempfile::tempdir().unwrap();
        let password = tmp.path().join("password");
        let client = tmp.path().join("client");
        let multiline = tmp.path().join("multiline");
        let oversized = tmp.path().join("oversized");
        let empty = tmp.path().join("empty");
        std::fs::write(&password, "fixture-password\r\n").unwrap();
        std::fs::write(&client, "fixture-client\n").unwrap();
        std::fs::write(&multiline, "fixture\nsecond-line\n").unwrap();
        std::fs::write(&oversized, "x".repeat(4097)).unwrap();
        std::fs::write(&empty, "\r\n").unwrap();

        // Exercise the real environment reader in subprocesses, with no
        // ambient configuration and no process-wide environment mutations
        // in the (parallel) test runner. Nothing connects to a server.
        for case in [
            "native-file",
            "public-direct",
            "missing-board",
            "missing-password",
            "native-missing-otp",
            "native-plaintext",
            "conflicting-inputs",
            "missing-file",
            "multiline",
            "oversized",
            "empty",
        ] {
            let mut child = Command::new(std::env::current_exe().unwrap());
            child
                .args([
                    "--exact",
                    "config::tests::read_in_isolation",
                    "--nocapture",
                ])
                .env_clear()
                .env(CASE, case)
                .env(B4_URL, "https://board.fixture.invalid")
                .env(TRUSTEE_CONFIG_PATH, "fixture-keys.json")
                .env(TRUSTEE_NAME, "fixture-trustee")
                .env(TRUSTEE_PLATFORM_URL, "https://portal.fixture.invalid")
                .env(TRUSTEE_KEYCLOAK_URL, "https://id.fixture.invalid")
                .env(TRUSTEE_KEYCLOAK_REALM, "fixture-realm")
                .env(TRUSTEE_KEYCLOAK_CLIENT_ID, NATIVE_TRUSTEE_CLIENT)
                .env(TRUSTEE_PASSWORD_FILE, &password)
                .env(TRUSTEE_KEYCLOAK_CLIENT_SECRET_FILE, &client)
                .env(TRUSTEE_OTP_FILE, tmp.path().join("otp"))
                .env(TRUSTEE_KEYCLOAK_CA_FILE, tmp.path().join("ca.pem"));
            if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
                child.env("LLVM_PROFILE_FILE", profile);
            }
            match case {
                "public-direct" => {
                    child.env(TRUSTEE_KEYCLOAK_CLIENT_ID, "fixture-public");
                    child.env_remove(TRUSTEE_KEYCLOAK_CLIENT_SECRET_FILE);
                    child.env_remove(TRUSTEE_PASSWORD_FILE);
                    child.env_remove(TRUSTEE_OTP_FILE);
                    child.env(TRUSTEE_KEYCLOAK_CA_FILE, "");
                    child.env(TRUSTEE_PASSWORD, "fixture-direct");
                }
                "missing-board" => {
                    child.env_remove(B4_URL);
                }
                "missing-password" => {
                    child.env_remove(TRUSTEE_PASSWORD_FILE);
                }
                "native-missing-otp" => {
                    child.env_remove(TRUSTEE_OTP_FILE);
                }
                "native-plaintext" => {
                    child
                        .env(TRUSTEE_KEYCLOAK_URL, "http://id.fixture.invalid");
                }
                "conflicting-inputs" => {
                    child.env(TRUSTEE_PASSWORD, "fixture-direct");
                }
                "missing-file" => {
                    child.env(TRUSTEE_PASSWORD_FILE, tmp.path().join("absent"));
                }
                "multiline" => {
                    child.env(TRUSTEE_PASSWORD_FILE, &multiline);
                }
                "oversized" => {
                    child.env(TRUSTEE_PASSWORD_FILE, &oversized);
                }
                "empty" => {
                    child.env(TRUSTEE_PASSWORD_FILE, &empty);
                }
                _ => {}
            }
            let result = child.output().unwrap();
            assert!(
                result.status.success()
                    && String::from_utf8_lossy(&result.stdout)
                        .contains("test result: ok. 1 passed;"),
                "{case}: {} {}",
                String::from_utf8_lossy(&result.stdout),
                String::from_utf8_lossy(&result.stderr)
            );
        }
    }

    #[test]
    fn read_in_isolation() {
        let Ok(case) = std::env::var(CASE) else {
            return;
        };
        let result = if case == "public-direct" {
            Config::read(
                Some("https://explicit.fixture.invalid".to_string()),
                Some(PathBuf::from("explicit-keys.json")),
            )
        } else {
            Config::read(Some(String::new()), Some(PathBuf::new()))
        };
        match case.as_str() {
            "native-file" => {
                let actual = result
                    .expect("synthetic native configuration should succeed");
                assert_eq!(actual.b4_url, "https://board.fixture.invalid");
                assert_eq!(
                    actual.keys_file,
                    PathBuf::from("fixture-keys.json")
                );
                assert_eq!(
                    actual.platform_url,
                    "https://portal.fixture.invalid"
                );
                assert_eq!(actual.trustee_name, "fixture-trustee");
                assert_eq!(actual.credentials.username, "fixture-trustee");
                assert_eq!(actual.credentials.realm, "fixture-realm");
                assert_eq!(actual.credentials.password, "fixture-password");
                assert_eq!(
                    actual.credentials.client_secret.as_deref(),
                    Some("fixture-client")
                );
                assert!(actual.credentials.otp_file.unwrap().ends_with("otp"));
                assert!(actual
                    .credentials
                    .keycloak_ca_file
                    .unwrap()
                    .ends_with("ca.pem"));
            }
            "public-direct" => {
                let actual = result
                    .expect("synthetic public configuration should succeed");
                assert_eq!(actual.b4_url, "https://explicit.fixture.invalid");
                assert_eq!(
                    actual.keys_file,
                    PathBuf::from("explicit-keys.json")
                );
                assert_eq!(actual.credentials.password, "fixture-direct");
                assert!(actual.credentials.client_secret.is_none());
                assert!(actual.credentials.otp_file.is_none());
                assert!(actual.credentials.keycloak_ca_file.is_none());
            }
            invalid => {
                let expected = match invalid {
                    "missing-board" => "--b4-url was not given: B4_URL is not set",
                    "missing-password" => "trustee password is not configured",
                    "native-missing-otp" => "the native trustee client requires its private client secret, OTP file and site CA",
                    "native-plaintext" => "the native trustee Keycloak URL must be HTTPS without user information, query or fragment",
                    "conflicting-inputs" => "configure TRUSTEE_PASSWORD or TRUSTEE_PASSWORD_FILE, not both",
                    "missing-file" => "cannot read private setting TRUSTEE_PASSWORD_FILE",
                    "multiline" | "oversized" | "empty" => "invalid private setting TRUSTEE_PASSWORD_FILE",
                    _ => unreachable!(),
                };
                let error = result
                    .err()
                    .expect("unsafe or incomplete configuration was accepted");
                assert_eq!(format!("{error:#}"), expected);
            }
        }
    }

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
