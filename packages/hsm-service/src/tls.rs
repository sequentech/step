// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Mutual TLS: every caller presents a certificate from the installation's
//! caller CA, and is then identified by the SHA-256 of its certificate's
//! public key, which the policy names. mTLS rather than Keycloak tokens, so
//! that the service doesn't depend on Keycloak to start or to authorize
//! services: Keycloak's own keys, and the vault that holds its secrets, may
//! depend on this service.

use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use rustls::server::WebPkiClientVerifier;
use rustls::{RootCertStore, ServerConfig};
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
use sha2::{Digest, Sha256};

pub fn certificates(path: &Path) -> Result<Vec<CertificateDer<'static>>> {
    let certificates = CertificateDer::pem_file_iter(path)
        .with_context(|| format!("reading {}", path.display()))?
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("parsing {}", path.display()))?;
    if certificates.is_empty() {
        return Err(anyhow!("{} holds no certificate", path.display()));
    }
    Ok(certificates)
}

pub fn server_config(cert: &Path, key: &Path, client_ca: &Path) -> Result<Arc<ServerConfig>> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut roots = RootCertStore::empty();
    for ca in certificates(client_ca)? {
        roots.add(ca).context("adding the caller CA")?;
    }
    let verifier = WebPkiClientVerifier::builder_with_provider(Arc::new(roots), provider.clone())
        .build()
        .context("building the caller verifier")?;
    let key =
        PrivateKeyDer::from_pem_file(key).with_context(|| format!("reading {}", key.display()))?;
    let config = ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .context("TLS versions")?
        .with_client_cert_verifier(verifier)
        .with_single_cert(certificates(cert)?, key)
        .context("the service certificate")?;
    Ok(Arc::new(config))
}

/// Hex SHA-256 of a certificate's DER SubjectPublicKeyInfo.
pub fn spki_sha256(certificate: &[u8]) -> Result<String> {
    let (_, parsed) = x509_parser::parse_x509_certificate(certificate)
        .map_err(|e| anyhow!("unreadable caller certificate: {e}"))?;
    Ok(hex::encode(Sha256::digest(
        parsed.tbs_certificate.subject_pki.raw,
    )))
}
