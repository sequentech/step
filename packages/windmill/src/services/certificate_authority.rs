// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::CertificateAuthEventAction;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tracing::{error, info, warn};
use uuid::Uuid;
use x509_parser::pem::parse_x509_pem;

use crate::postgres::certificate_authority::{
    delete_certificate_authorities, insert_certificate_authority, CertificateAuthorityRecord,
};
use crate::services::election_event_board::get_election_event_board;
use crate::services::electoral_log::ElectoralLog;

// The standard PEM (Privacy-Enhanced Mail) file format conventionally requires five dashes
const CERT_BEGIN: &str = "-----BEGIN CERTIFICATE-----";
const CERT_END: &str = "-----END CERTIFICATE-----";

pub struct ParsedCertificate {
    pub common_name: String,
    pub subject: String,
    pub issuer_common_name: String,
    pub issuer: String,
    pub not_before: DateTime<Utc>,
    pub not_after: DateTime<Utc>,
    pub fingerprint_sha256: String,
    pub serial_number: String,
    pub pem: String,
}

/// Splits a PEM bundle (potentially containing multiple certificates) into
/// individual PEM strings, one per certificate.
pub fn split_pem_bundle(pem_content: &str) -> Vec<String> {
    let mut certs = Vec::new();
    let mut current = String::new();
    let mut in_cert = false;

    for line in pem_content.lines() {
        let trimmed = line.trim();
        if trimmed == CERT_BEGIN {
            in_cert = true;
            current.clear();
            current.push_str(trimmed);
            current.push('\n');
        } else if trimmed == CERT_END {
            current.push_str(trimmed);
            current.push('\n');
            if in_cert {
                certs.push(current.clone());
                current.clear();
            }
            in_cert = false;
        } else if in_cert {
            current.push_str(trimmed);
            current.push('\n');
        }
    }

    certs
}

/// Parses a single PEM-encoded X.509 certificate and extracts its metadata.
pub fn parse_certificate_pem(pem: &str) -> Result<ParsedCertificate> {
    let (_, pem_obj) =
        parse_x509_pem(pem.as_bytes()).map_err(|e| anyhow!("Failed to parse PEM: {e}"))?;

    let cert = pem_obj
        .parse_x509()
        .map_err(|e| anyhow!("Failed to parse X.509 certificate: {e}"))?;

    let subject = cert.subject().to_string();
    let issuer = cert.issuer().to_string();

    let common_name = cert
        .subject()
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .unwrap_or(&subject)
        .to_string();

    let issuer_common_name = cert
        .issuer()
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok())
        .unwrap_or(&issuer)
        .to_string();

    let validity = cert.validity();
    let not_before = Utc
        .timestamp_opt(validity.not_before.timestamp(), 0)
        .single()
        .ok_or_else(|| anyhow!("Invalid notBefore timestamp"))?;
    let not_after = Utc
        .timestamp_opt(validity.not_after.timestamp(), 0)
        .single()
        .ok_or_else(|| anyhow!("Invalid notAfter timestamp"))?;

    let fingerprint_sha256 = Sha256::digest(&pem_obj.contents)
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":");

    let serial_number = cert.tbs_certificate.raw_serial_as_string();

    Ok(ParsedCertificate {
        common_name,
        subject,
        issuer_common_name,
        issuer,
        not_before,
        not_after,
        fingerprint_sha256,
        serial_number,
        pem: pem.to_string(),
    })
}

pub struct CertificateImportResult {
    pub inserted_count: i32,
    pub skipped_count: i32,
    pub errors: Vec<String>,
}

pub async fn import_certificate_authority(
    hasura_transaction: Transaction<'_>,
    pem_chunks: Vec<String>,
    tenant_uuid: Uuid,
    election_event_id: Uuid,
    bulletin_board_reference: Option<Value>,
    tenant_id: &str,
    user_id: &str,
    preferred_username: Option<String>,
) -> Result<CertificateImportResult> {
    let mut inserted_count: i32 = 0;
    let mut skipped_count: i32 = 0;
    let mut errors: Vec<String> = Vec::new();
    let mut inserted_subjects: Vec<String> = Vec::new();

    for (i, pem_chunk) in pem_chunks.iter().enumerate() {
        match parse_certificate_pem(pem_chunk) {
            Ok(parsed) => {
                if parsed.not_after < Utc::now() {
                    warn!(
                        cert_index = i + 1,
                        common_name = %parsed.common_name,
                        not_after = %parsed.not_after,
                        "Certificate is expired, skipping"
                    );
                    errors.push(format!(
                        "Certificate {}: expired on {}",
                        i + 1,
                        parsed.not_after.format("%Y-%m-%d")
                    ));
                    continue;
                }
                let record = CertificateAuthorityRecord {
                    id: Uuid::new_v4(),
                    tenant_id: tenant_uuid,
                    election_event_id,
                    common_name: parsed.common_name,
                    subject: parsed.subject.clone(),
                    issuer_common_name: parsed.issuer_common_name,
                    issuer: parsed.issuer,
                    not_before: parsed.not_before,
                    not_after: parsed.not_after,
                    fingerprint_sha256: parsed.fingerprint_sha256,
                    serial_number: parsed.serial_number,
                    pem: parsed.pem,
                };
                match insert_certificate_authority(&hasura_transaction, record).await {
                    Ok(true) => {
                        info!(cert_index = i + 1, "Certificate inserted");
                        inserted_count += 1;
                        inserted_subjects.push(parsed.subject);
                    }
                    Ok(false) => {
                        info!(cert_index = i + 1, "Certificate skipped (duplicate)");
                        skipped_count += 1;
                    }
                    Err(e) => {
                        warn!(cert_index = i + 1, error = %e, "Failed to insert certificate");
                        errors.push(format!("Certificate {}: {}", i + 1, e));
                    }
                }
            }
            Err(e) => {
                warn!(cert_index = i + 1, error = %e, "Failed to parse certificate");
                errors.push(format!("Certificate {}: {}", i + 1, e));
            }
        }
    }

    let electoral_log = if !inserted_subjects.is_empty() {
        let board_name = get_election_event_board(bulletin_board_reference)
            .ok_or_else(|| anyhow!("Missing bulletin board"))?;
        match ElectoralLog::for_admin_user(
            &hasura_transaction,
            &board_name,
            tenant_id,
            &election_event_id.to_string(),
            user_id,
            preferred_username.clone(),
            None,
            None,
        )
        .await
        {
            Ok(log) => Some(log),
            Err(e) => {
                error!("Error initializing electoral log for CA import: {e:?}");
                None
            }
        }
    } else {
        None
    };

    hasura_transaction
        .commit()
        .await
        .context("Failed to commit transaction")?;

    if let Some(log) = electoral_log {
        if let Err(e) = log
            .post_certificate_auth_event(
                election_event_id.to_string(),
                CertificateAuthEventAction::Import,
                inserted_subjects,
                Some(user_id.to_string()),
                preferred_username,
            )
            .await
        {
            error!("Error posting CA import event to electoral log: {e:?}");
        }
    }

    Ok(CertificateImportResult {
        inserted_count,
        skipped_count,
        errors,
    })
}

pub async fn delete_certificate_authority(
    hasura_transaction: Transaction<'_>,
    ids: &[Uuid],
    election_event_id: Uuid,
    tenant_uuid: Uuid,
    bulletin_board_reference: Option<Value>,
    tenant_id: &str,
    user_id: &str,
    preferred_username: Option<String>,
) -> Result<i32> {
    let deleted_subjects =
        delete_certificate_authorities(&hasura_transaction, ids, election_event_id, tenant_uuid)
            .await
            .context("Failed to delete certificate authorities")?;

    let electoral_log = if !deleted_subjects.is_empty() {
        let board_name = get_election_event_board(bulletin_board_reference)
            .ok_or_else(|| anyhow!("Missing bulletin board"))?;
        match ElectoralLog::for_admin_user(
            &hasura_transaction,
            &board_name,
            tenant_id,
            &election_event_id.to_string(),
            user_id,
            preferred_username.clone(),
            None,
            None,
        )
        .await
        {
            Ok(log) => Some(log),
            Err(e) => {
                error!("Error initializing electoral log for CA delete: {e:?}");
                None
            }
        }
    } else {
        None
    };

    let deleted_count = deleted_subjects.len();

    hasura_transaction
        .commit()
        .await
        .context("Failed to commit transaction")?;

    if let Some(log) = electoral_log {
        if let Err(e) = log
            .post_certificate_auth_event(
                election_event_id.to_string(),
                CertificateAuthEventAction::Delete,
                deleted_subjects,
                Some(user_id.to_string()),
                preferred_username,
            )
            .await
        {
            error!("Error posting CA delete event to electoral log: {e:?}");
        }
    }

    Ok(deleted_count as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_pem_bundle_empty() {
        assert!(split_pem_bundle("").is_empty());
    }

    #[test]
    fn test_split_pem_bundle_no_certs() {
        assert!(split_pem_bundle("just some random text").is_empty());
    }

    #[test]
    fn test_split_pem_bundle_single_cert() {
        let pem = "-----BEGIN CERTIFICATE-----\nABCD\n-----END CERTIFICATE-----\n";
        let certs = split_pem_bundle(pem);
        assert_eq!(certs.len(), 1);
        assert!(certs[0].contains("-----BEGIN CERTIFICATE-----"));
        assert!(certs[0].contains("-----END CERTIFICATE-----"));
    }

    #[test]
    fn test_split_pem_bundle_two_certs() {
        let pem = "-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n\
                   -----BEGIN CERTIFICATE-----\nBBBB\n-----END CERTIFICATE-----\n";
        let certs = split_pem_bundle(pem);
        assert_eq!(certs.len(), 2);
    }
}
