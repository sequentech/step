// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Shared PostgreSQL transport for the board service and its native clients.

use anyhow::{bail, Context, Result};
use bb8_postgres::{bb8::Pool, PostgresConnectionManager};
use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
use postgres_openssl::MakeTlsConnector;
use std::{env, path::PathBuf, str::FromStr, time::Duration};
use tokio_postgres::{config::Host, config::SslMode, Client, Config};

/// A pool whose connections use the board's configured transport.
pub type DbPool = Pool<PostgresConnectionManager<MakeTlsConnector>>;

/// PostgreSQL transport policy. Plaintext must be explicitly selected.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
pub enum PgTlsMode {
    /// Plaintext on an isolated local socket/network or authenticated local proxy.
    #[serde(rename = "disable")]
    Disable,
    /// Require TLS, a trusted certificate chain and a matching hostname.
    #[default]
    #[serde(rename = "verify-full", alias = "require")]
    VerifyFull,
}

impl FromStr for PgTlsMode {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "disable" => Ok(Self::Disable),
            // Require retains the same verification guarantees as verify-full.
            "require" | "verify-full" => Ok(Self::VerifyFull),
            _ => bail!("B3_PG_SSL_MODE must be disable, require or verify-full"),
        }
    }
}

/// Trust settings shared by every PostgreSQL path in b3.
#[derive(Clone, Debug, Default)]
pub struct PgTlsConfig {
    /// Defaults to verified TLS; there is no fallback to plaintext.
    pub mode: PgTlsMode,
    /// Optional PEM CA bundle. When absent, OpenSSL's system roots are used.
    pub ca_path: Option<PathBuf>,
}

impl PgTlsConfig {
    /// Read B3_PG_SSL_MODE and B3_PG_CA_PATH. Invalid values fail closed.
    pub fn from_env() -> Result<Self> {
        fn optional(name: &str) -> Result<Option<String>> {
            match env::var(name) {
                Ok(value) => Ok(Some(value)),
                Err(env::VarError::NotPresent) => Ok(None),
                Err(error) => Err(error).with_context(|| format!("Invalid {name}")),
            }
        }
        let mode = optional("B3_PG_SSL_MODE")?
            .map(|value| value.parse())
            .transpose()?
            .unwrap_or_default();
        let ca_path = optional("B3_PG_CA_PATH")?
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        Ok(Self { mode, ca_path })
    }

    /// Apply the policy and load trust material before opening any connection.
    pub fn configure(&self, config: &mut Config) -> Result<MakeTlsConnector> {
        if self.mode == PgTlsMode::Disable && self.ca_path.is_some() {
            bail!("B3_PG_CA_PATH cannot be used with B3_PG_SSL_MODE=disable");
        }
        match self.mode {
            PgTlsMode::Disable => {
                config.ssl_mode(SslMode::Disable);
            }
            PgTlsMode::VerifyFull => {
                if config.get_hosts().is_empty()
                    || config
                        .get_hosts()
                        .iter()
                        .any(|host| !matches!(host, Host::Tcp(_)))
                {
                    bail!("Verified PostgreSQL TLS requires a TCP hostname; use disable explicitly for a local Unix socket");
                }
                config.ssl_mode(SslMode::Require);
            }
        }
        config.connect_timeout(Duration::from_secs(10));
        let mut builder = SslConnector::builder(SslMethod::tls())?;
        builder.set_verify(SslVerifyMode::PEER);
        if let Some(path) = &self.ca_path {
            builder
                .set_ca_file(path)
                .context("Unable to load B3_PG_CA_PATH CA bundle")?;
        }
        let mut connector = MakeTlsConnector::new(builder.build());
        connector.set_callback(|configuration, _| {
            configuration.set_verify_hostname(true);
            Ok(())
        });
        Ok(connector)
    }
}

/// Open a direct session and drive its connection until the client is dropped.
pub async fn connect(mut config: Config, tls: &PgTlsConfig) -> Result<Client> {
    let connector = tls.configure(&mut config)?;
    let (client, connection) = config.connect(connector).await?;
    tokio::spawn(async move {
        if let Err(error) = connection.await {
            tracing::error!(%error, "PostgreSQL connection failed");
        }
    });
    Ok(client)
}

/// Create a bounded pool without changing session or transaction ownership.
pub async fn pool(mut config: Config, tls: &PgTlsConfig, max_size: u32) -> Result<DbPool> {
    let connector = tls.configure(&mut config)?;
    let manager = PostgresConnectionManager::new(config, connector);
    Ok(Pool::builder()
        .max_size(max_size)
        .connection_timeout(Duration::from_secs(10))
        .build(manager)
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_never_select_opportunistic_or_unverified_tls() {
        assert_eq!(PgTlsMode::default(), PgTlsMode::VerifyFull);
        assert_eq!(
            "require".parse::<PgTlsMode>().unwrap(),
            PgTlsMode::VerifyFull
        );
        assert_eq!(
            "verify-full".parse::<PgTlsMode>().unwrap(),
            PgTlsMode::VerifyFull
        );
        assert_eq!("disable".parse::<PgTlsMode>().unwrap(), PgTlsMode::Disable);
        for mode in ["", "prefer", "verify-ca", "Require", "invalid"] {
            assert!(mode.parse::<PgTlsMode>().is_err());
        }
    }

    #[test]
    fn verified_mode_rejects_unix_sockets_and_missing_ca_files() {
        let mut config = Config::new();
        config.host("/tmp");
        assert!(PgTlsConfig::default().configure(&mut config).is_err());
        config = Config::new();
        config.host("localhost");
        let tls = PgTlsConfig {
            ca_path: Some(PathBuf::from("/b3-test-missing/ca.pem")),
            ..Default::default()
        };
        assert!(tls
            .configure(&mut config)
            .err()
            .unwrap()
            .to_string()
            .contains("B3_PG_CA_PATH"));
    }

    #[test]
    fn missing_host_and_plaintext_with_a_ca_fail_closed() {
        assert!(PgTlsConfig::default()
            .configure(&mut Config::new())
            .is_err());
        let mut config = Config::new();
        config.host("localhost");
        let tls = PgTlsConfig {
            mode: PgTlsMode::Disable,
            ca_path: Some(PathBuf::from("ca.pem")),
        };
        assert!(tls.configure(&mut config).is_err());
        let tls = PgTlsConfig {
            mode: PgTlsMode::Disable,
            ca_path: None,
        };
        tls.configure(&mut config).unwrap();
        assert_eq!(config.get_ssl_mode(), SslMode::Disable);
    }

    #[test]
    #[serial_test::serial]
    fn environment_defaults_and_invalid_values_fail_closed() {
        struct Restore(Vec<(&'static str, Option<std::ffi::OsString>)>);
        impl Drop for Restore {
            fn drop(&mut self) {
                for (name, value) in self.0.drain(..) {
                    if let Some(value) = value {
                        env::set_var(name, value);
                    } else {
                        env::remove_var(name);
                    }
                }
            }
        }
        let _restore = Restore(
            ["B3_PG_SSL_MODE", "B3_PG_CA_PATH"]
                .map(|name| (name, env::var_os(name)))
                .to_vec(),
        );
        env::remove_var("B3_PG_SSL_MODE");
        env::remove_var("B3_PG_CA_PATH");
        assert_eq!(PgTlsConfig::from_env().unwrap().mode, PgTlsMode::VerifyFull);
        env::set_var("B3_PG_SSL_MODE", "prefer");
        assert!(PgTlsConfig::from_env().is_err());
        env::set_var("B3_PG_SSL_MODE", "require");
        env::set_var("B3_PG_CA_PATH", "/synthetic/ca.pem");
        let tls = PgTlsConfig::from_env().unwrap();
        assert_eq!(tls.mode, PgTlsMode::VerifyFull);
        assert_eq!(tls.ca_path, Some(PathBuf::from("/synthetic/ca.pem")));
        env::set_var("B3_PG_SSL_MODE", "disable");
        env::set_var("B3_PG_CA_PATH", "");
        let tls = PgTlsConfig::from_env().unwrap();
        assert_eq!(tls.mode, PgTlsMode::Disable);
        assert_eq!(tls.ca_path, None);
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            env::set_var("B3_PG_SSL_MODE", std::ffi::OsString::from_vec(vec![255]));
            assert!(PgTlsConfig::from_env().is_err());
        }
    }
}
