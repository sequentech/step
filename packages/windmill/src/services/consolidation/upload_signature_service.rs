// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    create_transmission_package_service::{
        generate_all_servers_document, update_transmission_package_annotations,
    },
    eml_generator::{
        find_miru_annotation, prepend_miru_annotation, ValidateAnnotations, MIRU_AREA_CCS_SERVERS,
        MIRU_AREA_STATION_ID, MIRU_AREA_TRUSTEE_USERS, MIRU_PLUGIN_PREPEND,
        MIRU_TALLY_SESSION_DATA, MIRU_TRUSTEE_ID, MIRU_TRUSTEE_NAME,
    },
    eml_types::ACMTrustee,
    logs::{
        create_transmission_package_log, error_sending_transmission_package_to_ccs_log,
        send_transmission_package_to_ccs_log, sign_transmission_package_log,
    },
    rsa::{derive_public_key_from_p12, rsa_sign_data},
    send_transmission_package_service::get_latest_miru_document,
    transmission_package::{compress_hash_eml, create_transmission_package, read_temp_file},
    zip::unzip_file,
};
use crate::{
    postgres::{
        area::get_area_by_id, document::get_document, election::get_election_by_id,
        election_event::get_election_event_by_election_area,
        tally_session::get_tally_session_by_id,
    },
    services::{
        database::get_hasura_pool,
        documents::{get_document_as_temp_file, upload_and_return_document_postgres},
        temp_path::{generate_temp_file, get_file_size},
    },
    types::miru_plugin::{
        MiruCcsServer, MiruServerDocument, MiruTallySessionData, MiruTransmissionPackageData,
    },
};
use crate::{
    postgres::{
        election_event::get_election_event_by_id,
        trustee::{get_all_trustees, get_trustee_by_name},
    },
    types::miru_plugin::{MiruDocument, MiruDocumentIds, MiruSbeiUser, MiruSignature},
};
use anyhow::{anyhow, Context, Result};
use chrono::{Local, Utc};
use deadpool_postgres::{Client as DbClient, Transaction};
use reqwest::multipart;
use sequent_core::{
    ballot::Annotations,
    serialization::deserialize_with_path::{deserialize_str, deserialize_value},
    services::date::ISO8601,
    types::hasura::core::Trustee,
    util::date_time::get_system_timezone,
};
use std::collections::HashMap;
use tempfile::NamedTempFile;
use tracing::{info, instrument};

#[instrument(skip_all, err)]
async fn update_signatures(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    new_miru_signature: &MiruSignature,
    current_miru_signatures: &Vec<MiruSignature>,
) -> Result<(Vec<ACMTrustee>, Vec<MiruSignature>)> {
    let election_event =
        get_election_event_by_id(hasura_transaction, tenant_id, election_event_id).await?;

    let event_annotations = election_event.get_annotations()?;

    let mut new_miru_signatures: Vec<MiruSignature> = current_miru_signatures
        .clone()
        .into_iter()
        .filter(|signature| signature.trustee_name != new_miru_signature.trustee_name)
        .collect();
    new_miru_signatures.push(new_miru_signature.clone());

    let trustees_map: HashMap<String, MiruSbeiUser> = event_annotations
        .sbei_users
        .clone()
        .iter()
        .map(|trustee| (trustee.miru_id.clone(), trustee.clone()))
        .collect::<HashMap<String, MiruSbeiUser>>();

    let acm_trustees: Vec<ACMTrustee> = new_miru_signatures
        .clone()
        .into_iter()
        .map(|miru_signature| -> Result<ACMTrustee> {
            let trustee_annotations =
                trustees_map
                    .get(&miru_signature.trustee_name)
                    .ok_or(anyhow!(
                        "Can't find trustee by name {}",
                        miru_signature.trustee_name
                    ))?;

            Ok(ACMTrustee {
                id: trustee_annotations.miru_id.clone(),
                signature: Some(miru_signature.signature.clone()),
                publickey: Some(miru_signature.pub_key.clone()),
                name: trustee_annotations.miru_name.clone(),
            })
        })
        .collect::<Result<_>>()?;

    Ok((acm_trustees, new_miru_signatures))
}

pub fn derive_public_key_from_private_key(
    private_key_temp_file: &NamedTempFile,
    password: &str,
) -> Result<String> {
    let pk12_file_path = private_key_temp_file.path();
    let pk12_file_path_string = pk12_file_path.to_string_lossy().to_string();
    derive_public_key_from_p12(&pk12_file_path_string, password)
}

pub fn create_server_signature(
    eml_data: NamedTempFile,
    sbei: &MiruSbeiUser,
    private_key_temp_file: &NamedTempFile,
    password: &str,
    public_key: &str,
) -> Result<MiruSignature> {
    let temp_pem_file_path = eml_data.path();
    let temp_pem_file_string = temp_pem_file_path.to_string_lossy().to_string();

    let pk12_file_path = private_key_temp_file.path();
    let pk12_file_path_string = pk12_file_path.to_string_lossy().to_string();

    let signature = rsa_sign_data(&pk12_file_path_string, password, &temp_pem_file_string)?;
    Ok(MiruSignature {
        trustee_name: sbei.miru_id.clone(),
        pub_key: public_key.to_string(),
        signature: signature,
    })
}

#[instrument(err)]
pub async fn upload_transmission_package_signature_service(
    tenant_id: &str,
    election_id: &str,
    area_id: &str,
    tally_session_id: &str,
    username: &str,
    document_id: &str,
    password: &str,
) -> Result<()> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring hasura connection pool")?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error acquiring hasura transaction")?;

    let time_zone = get_system_timezone();
    let now_utc = Utc::now();
    let now_local = now_utc.with_timezone(&Local);

    let election_event =
        get_election_event_by_election_area(&hasura_transaction, tenant_id, election_id, area_id)
            .await
            .with_context(|| "Error fetching election event")?;

    let election_event_annotations = election_event.get_annotations()?;

    let Some(election) = get_election_by_id(
        &hasura_transaction,
        tenant_id,
        &election_event.id,
        election_id,
    )
    .await?
    else {
        return Err(anyhow!("Election not found"));
    };
    let election_annotations = election.get_annotations()?;
    let area = get_area_by_id(&hasura_transaction, tenant_id, &area_id)
        .await
        .with_context(|| format!("Error fetching area {}", area_id))?
        .ok_or_else(|| anyhow!("Can't find area {}", area_id))?;
    let area_name = area.name.clone().unwrap_or("".into());
    let area_annotations = area.get_annotations()?;

    let sbei_user_opt = election_event_annotations
        .sbei_users
        .clone()
        .into_iter()
        .find(|sbei| {
            sbei.username == username && area_annotations.sbei_ids.contains(&sbei.miru_id) && sbei.miru_election_id == election_annotations.election_id
        });

    let Some(sbei_user) = sbei_user_opt else {
        return Err(anyhow!(
            "SBEI user not found area '{}' and username '{}'",
            area_name,
            username
        ));
    };

    let tally_session = get_tally_session_by_id(
        &hasura_transaction,
        tenant_id,
        &election_event.id,
        tally_session_id,
    )
    .await
    .with_context(|| "Error fetching tally session")?;
    let transmission_data = tally_session.get_annotations()?;
    let tally_annotations_js = tally_session
        .annotations
        .clone()
        .ok_or_else(|| anyhow!("Missing tally session annotations"))?;

    let tally_annotations: Annotations = deserialize_value(tally_annotations_js)?;

    let Some(transmission_area_election) = transmission_data.clone().into_iter().find(|data| {
        data.area_id == area_id.to_string() && data.election_id == election_id.to_string()
    }) else {
        info!("transmission package not found, skipping");
        return Ok(());
    };
    let Some(miru_document) = get_latest_miru_document(&transmission_area_election.documents)
    else {
        info!("transmission package document not found, skipping");
        return Ok(());
    };

    let private_key_document = get_document(
        &hasura_transaction,
        tenant_id,
        Some(election_event.id.clone()),
        &document_id,
    )
    .await?
    .ok_or_else(|| anyhow!("Can't find document {}", document_id))?;
    let mut private_key_temp_file =
        get_document_as_temp_file(tenant_id, &private_key_document).await?;

    // download er file
    let document = get_document(
        &hasura_transaction,
        tenant_id,
        Some(election_event.id.clone()),
        &miru_document.document_ids.eml,
    )
    .await?
    .ok_or_else(|| {
        anyhow!(
            "Can't find document {}",
            miru_document.document_ids.all_servers
        )
    })?;

    let mut eml_data = get_document_as_temp_file(tenant_id, &document).await?;
    let eml_bytes = read_temp_file(&mut eml_data)?;
    let eml = String::from_utf8(eml_bytes)?;
    // RSA sign er file
    let public_key_pem_string =
        derive_public_key_from_private_key(&private_key_temp_file, password)?;
    let server_signature = create_server_signature(
        eml_data,
        &sbei_user,
        &private_key_temp_file,
        password,
        &public_key_pem_string,
    )?;

    let (new_acm_signatures, new_miru_signatures) = update_signatures(
        &hasura_transaction,
        tenant_id,
        &election_event.id,
        &server_signature,
        &miru_document.signatures,
    )
    .await?;
    let mut new_signatures: Vec<MiruSignature> = miru_document
        .signatures
        .clone()
        .into_iter()
        .filter(|signature| signature.trustee_name != sbei_user.miru_id)
        .collect();
    new_signatures.push(server_signature.clone());
    // generate zip of zips
    let mut new_transmission_package_data = transmission_area_election.clone();
    new_transmission_package_data
        .logs
        .push(sign_transmission_package_log(
            &now_local,
            election_id,
            &election.name,
            area_id,
            &area_name,
            &sbei_user.miru_id,
        ));

    let (compressed_xml, rendered_xml_hash) = compress_hash_eml(&eml)?;
    let all_servers_document = generate_all_servers_document(
        &hasura_transaction,
        &rendered_xml_hash,
        &eml,
        compressed_xml,
        &area_annotations.ccs_servers,
        &area_annotations.station_id,
        &election_event_annotations,
        &election_event.id,
        tenant_id,
        time_zone.clone(),
        now_utc.clone(),
        new_acm_signatures,
        &new_transmission_package_data.logs,
    )
    .await?;

    // upload zip of zips
    let area_name = area.name.clone().unwrap_or_default();
    let Some(first_document) = new_transmission_package_data.documents.first() else {
        return Err(anyhow!("Missing initial document"));
    };
    new_transmission_package_data.documents.push(MiruDocument {
        document_ids: MiruDocumentIds {
            eml: first_document.document_ids.eml.clone(),
            xz: first_document.document_ids.xz.clone(),
            all_servers: all_servers_document.id.clone(),
        },
        transaction_id: first_document.transaction_id.clone(),
        servers_sent_to: vec![],
        created_at: ISO8601::to_string(&now_local),
        signatures: new_miru_signatures,
    });
    update_transmission_package_annotations(
        &hasura_transaction,
        tenant_id,
        &election_event.id,
        tally_session_id,
        area_id,
        election_id,
        transmission_data.clone(),
        new_transmission_package_data,
        tally_annotations.clone(),
    )
    .await?;

    hasura_transaction
        .commit()
        .await
        .with_context(|| "error comitting transaction")?;
    Ok(())
}
