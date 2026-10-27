// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Bounded probes of the actual trusted-channel endpoint and authenticated host time.
use anyhow::{ensure, Context, Result};
use openssl::{
    nid::Nid,
    ssl::{ErrorCode, HandshakeError, SslConnector, SslMethod, SslVerifyMode, SslVersion},
};
use std::{
    net::{SocketAddr, TcpStream},
    time::Duration,
};

pub async fn trusted_channels() -> Result<()> {
    let endpoints: Vec<String> = serde_json::from_str(
        &std::env::var("WINDMILL_SELF_TEST_TLS_ENDPOINTS")
            .context("Self-test TLS endpoints are not configured")?,
    )?;
    ensure!(
        !endpoints.is_empty() && endpoints.len() <= 16,
        "Configure between one and sixteen trusted-channel endpoints"
    );
    for endpoint in endpoints {
        let endpoint = reqwest::Url::parse(&endpoint)?;
        ensure!(
            endpoint.scheme() == "https"
                && endpoint.username().is_empty()
                && endpoint.password().is_none(),
            "Self-test endpoints must be HTTPS URLs without credentials"
        );
        let host = endpoint
            .host_str()
            .context("TLS endpoint has no host")?
            .to_owned();
        let port = endpoint
            .port_or_known_default()
            .context("TLS endpoint has no port")?;
        let address = tokio::time::timeout(
            Duration::from_secs(3),
            tokio::net::lookup_host((host.as_str(), port)),
        )
        .await
        .context("TLS endpoint resolution timed out")??
        .next()
        .context("TLS endpoint did not resolve")?;
        tokio::task::spawn_blocking(move || probe_endpoint(&host, address)).await??;
    }
    Ok(())
}
fn connector(
    version: SslVersion,
    cipher: &str,
    groups: &str,
    verify: SslVerifyMode,
) -> Result<SslConnector> {
    let mut b = SslConnector::builder(SslMethod::tls_client())?;
    b.set_min_proto_version(Some(version))?;
    b.set_max_proto_version(Some(version))?;
    b.set_ciphersuites(cipher)?;
    b.set_groups_list(groups)?;
    b.set_verify(verify);
    if let Ok(ca) = std::env::var("WINDMILL_SELF_TEST_TLS_CA_FILE") {
        b.set_ca_file(ca)?;
    }
    Ok(b.build())
}
fn connect(
    host: &str,
    addr: SocketAddr,
    connector: SslConnector,
) -> Result<openssl::ssl::SslStream<TcpStream>> {
    let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3))?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    Ok(connector
        .connect(host, stream)
        .map_err(|_| anyhow::anyhow!("TLS handshake or certificate verification failed"))?)
}
fn probe_endpoint(host: &str, addr: SocketAddr) -> Result<()> {
    let tls = connect(
        host,
        addr,
        connector(
            SslVersion::TLS1_3,
            "TLS_AES_256_GCM_SHA384",
            "P-384",
            SslVerifyMode::PEER,
        )?,
    )?;
    ensure!(
        tls.ssl().version_str() == "TLSv1.3",
        "Unexpected TLS version"
    );
    ensure!(
        tls.ssl().current_cipher().map(|c| c.name()) == Some("TLS_AES_256_GCM_SHA384"),
        "Unexpected TLS cipher"
    );
    let certificate = tls
        .ssl()
        .peer_certificate()
        .context("Missing endpoint certificate")?;
    let key = certificate
        .public_key()?
        .ec_key()
        .context("Endpoint certificate is not ECDSA")?;
    ensure!(
        key.group().curve_name() == Some(Nid::SECP384R1),
        "Endpoint certificate is not P-384"
    );
    // A positive handshake alone would miss a permissive fallback configuration.
    for (version, cipher, group) in [
        (SslVersion::TLS1_2, "TLS_AES_256_GCM_SHA384", "P-384"),
        (SslVersion::TLS1_3, "TLS_AES_128_GCM_SHA256", "P-384"),
        (SslVersion::TLS1_3, "TLS_AES_256_GCM_SHA384", "X25519"),
        (SslVersion::TLS1_3, "TLS_CHACHA20_POLY1305_SHA256", "P-384"),
        (SslVersion::TLS1_3, "TLS_AES_256_GCM_SHA384", "P-256"),
    ] {
        let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3))?;
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        stream.set_write_timeout(Some(Duration::from_secs(3)))?;
        // These probes send no application data. Disable certificate checking only
        // here so a different fallback certificate cannot hide a forbidden protocol.
        match connector(version, cipher, group, SslVerifyMode::NONE)?.connect(host, stream) {
            Err(HandshakeError::Failure(rejected)) if rejected.error().code() == ErrorCode::SSL => {
            }
            Ok(_) => anyhow::bail!("Trusted-channel endpoint accepts a forbidden TLS profile"),
            Err(_) => anyhow::bail!("Unable to establish a conclusive forbidden-profile rejection"),
        }
    }
    Ok(())
}

pub fn clock() -> Result<()> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ClockStatus {
        sampled_at: i64,
        synchronized: bool,
    }
    let path = std::env::var("WINDMILL_SELF_TEST_CLOCK_STATUS")
        .unwrap_or_else(|_| "/run/sequent-clock/status.json".into());
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .context("Host clock status is unavailable")?
        .take(4097)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 4096, "Invalid host clock status size");
    let status: ClockStatus = serde_json::from_slice(&bytes)?;
    let age = chrono::Utc::now().timestamp() - status.sampled_at;
    ensure!(
        (0..=30).contains(&age),
        "Host clock synchronization evidence is stale or in the future"
    );
    ensure!(
        status.synchronized,
        "The host has no synchronized authenticated time source"
    );
    Ok(())
}
