// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::authorization::authorize;
use deadpool_postgres::Client as DbClient;
use rocket::http::Status;
use rocket::serde::json::Json;
use sequent_core::ballot::VoterDigitalCertPolicy;
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::Permissions;
use serde::{Deserialize, Serialize};
use tokio::task;
use tracing::{info, instrument, warn};
use uuid::Uuid;
use windmill::postgres::certificate_authority::{
    insert_certificate_authority, CertificateAuthorityRecord,
};
use windmill::postgres::election_event::get_election_event_by_id;
use windmill::services::certificate_authority::{
    parse_certificate_pem, split_pem_bundle,
};
use windmill::services::database::get_hasura_pool;

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportCertificateAuthorityInput {
    election_event_id: uuid::Uuid,
    pem_content: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImportCertificateAuthorityOutput {
    inserted_count: i32,
    skipped_count: i32,
    errors: Vec<String>,
}

#[instrument(skip(claims, input))]
#[post("/import-certificate-authority", format = "json", data = "<input>")]
pub async fn import_certificate_authority(
    claims: JwtClaims,
    input: Json<ImportCertificateAuthorityInput>,
) -> Result<Json<ImportCertificateAuthorityOutput>, (Status, String)> {
    let tenant_id_str = claims.hasura_claims.tenant_id.clone();

    authorize(
        &claims,
        true,
        Some(tenant_id_str.clone()),
        vec![Permissions::CA_WRITE],
    )?;

    let body = input.into_inner();

    let tenant_uuid = Uuid::parse_str(&tenant_id_str)
        .map_err(|e| (Status::BadRequest, format!("Invalid tenant_id: {e}")))?;

    let pem_chunks = split_pem_bundle(&body.pem_content);
    if pem_chunks.is_empty() {
        return Err((
            Status::BadRequest,
            "No valid PEM certificates found in input".to_string(),
        ));
    }

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let election_event = get_election_event_by_id(
        &hasura_transaction,
        &tenant_id_str,
        &body.election_event_id.to_string(),
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let voter_digital_cert_policy = election_event
        .get_presentation()
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?
        .unwrap_or_default()
        .voter_digital_cert_policy
        .unwrap_or_default();

    if voter_digital_cert_policy != VoterDigitalCertPolicy::ENABLED {
        return Err((
            Status::Forbidden,
            "Digital certificate authentication is not allowed for this election event".to_string(),
        ));
    }

    let mut inserted_count: i32 = 0;
    let mut skipped_count: i32 = 0;
    let mut errors: Vec<String> = Vec::new();

    for (i, pem_chunk) in pem_chunks.iter().enumerate() {
        let pem_chunk_owned = pem_chunk.clone();
        let parse_result = task::spawn_blocking(move || {
            parse_certificate_pem(&pem_chunk_owned)
        })
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

        match parse_result {
            Ok(parsed) => {
                let record = CertificateAuthorityRecord {
                    id: Uuid::new_v4(),
                    tenant_id: tenant_uuid,
                    election_event_id: body.election_event_id,
                    common_name: parsed.common_name,
                    subject: parsed.subject,
                    issuer_common_name: parsed.issuer_common_name,
                    issuer: parsed.issuer,
                    not_before: parsed.not_before,
                    not_after: parsed.not_after,
                    fingerprint_sha256: parsed.fingerprint_sha256,
                    serial_number: parsed.serial_number,
                    pem: parsed.pem,
                };
                match insert_certificate_authority(&hasura_transaction, record)
                    .await
                {
                    Ok(true) => {
                        info!(cert_index = i + 1, "Certificate inserted");
                        inserted_count += 1;
                    }
                    Ok(false) => {
                        info!(
                            cert_index = i + 1,
                            "Certificate skipped (duplicate)"
                        );
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

    hasura_transaction
        .commit()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    Ok(Json(ImportCertificateAuthorityOutput {
        inserted_count,
        skipped_count,
        errors,
    }))
}
