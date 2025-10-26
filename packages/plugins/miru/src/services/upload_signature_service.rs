// SPDX-FileCopyrightText: 2025 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::{
    bindings::plugins_manager::{
        documents_manager::documents::create_document_as_temp_file,
        extra_services_manager::cli_service::create_server_signature as host_create_server_signature,
        transactions_manager::{
            postgres_queries::{
                get_area_by_id, get_document, get_election_by_id,
                get_election_event_by_election_area, get_election_event_by_id,
                get_tally_session_by_id, update_election_event_annotations,
            },
            transaction::commit_hasura_transaction,
        },
    },
    services::{
        create_transmission_package::{
            generate_all_servers_document, update_transmission_package_annotations,
        },
        eml_generator::{
            prepend_miru_annotation, MiruElectionEventAnnotations, ValidateAnnotations,
            MIRU_SBEI_USERS,
        },
        eml_types::ACMTrustee,
        logs::sign_transmission_package_log,
        miru_plugin_types::{
            MiruDocument, MiruDocumentIds, MiruSbeiUser, MiruSignature, MiruTallySessionData,
        },
        rsa::derive_public_key_from_p12,
        send_transmission_package::get_latest_miru_document,
        signatures::{check_certificate_cas, get_p12_cert, get_p12_fingerprint, ECIES_TOOL_PATH},
        transmission_package::compress_hash_eml,
    },
};
use chrono::{Local, Utc};
use core::convert::From;
use sequent_core::{
    ballot::Annotations,
    plugins::{get_plugin_shared_dir, Plugins},
    serialization::deserialize_with_path::{deserialize_str, deserialize_value},
    std_temp_path::{read_temp_file, TempFileGuard},
    types::hasura::core::{Area, Election, ElectionEvent, TallySession},
    util::date_time::PHILIPPINO_TIMEZONE,
};
use serde_json::Value;
use std::{collections::HashMap, path::PathBuf};
use tracing::instrument;

#[instrument(skip_all, err)]
fn update_election_event_sbei_users(
    election_event: &ElectionEvent,
    sbei_users: &Vec<MiruSbeiUser>,
    sbei_user: &MiruSbeiUser,
    certificate_fingerprint: &str,
) -> Result<(), String> {
    let mut new_sbei_users: Vec<_> = sbei_users
        .clone()
        .into_iter()
        .filter(|user| !(user.username == sbei_user.username && user.miru_id == sbei_user.miru_id))
        .collect();
    let mut new_sbei_user = sbei_user.clone();
    new_sbei_user.certificate_fingerprint = Some(certificate_fingerprint.to_string());
    new_sbei_users.push(new_sbei_user);

    let annotations_js = election_event
        .annotations
        .clone()
        .ok_or_else(|| format!("Missing election event annotations"))?;

    let mut annotations: Annotations = deserialize_value(annotations_js).map_err(|e| {
        println!(
            "[Guest Plugin Error] Failed to deserialize election event annotations: {}",
            e
        );
        e.to_string()
    })?;

    let key = prepend_miru_annotation(MIRU_SBEI_USERS);

    let serialized_sbei_users = serde_json::to_string(&new_sbei_users).map_err(|e| {
        println!("[Guest Plugin Error] Failed to serialize SBEI users: {}", e);
        e.to_string()
    })?;

    annotations.insert(key, serialized_sbei_users);

    let annotations_str = serde_json::to_string(&annotations).map_err(|e| {
        println!(
            "[Guest Plugin Error] Failed to serialize election event annotations: {}",
            e
        );
        e.to_string()
    })?;

    update_election_event_annotations(
        &election_event.tenant_id,
        &election_event.id,
        &annotations_str,
    )
}

#[instrument(skip_all, err)]
fn update_signatures(
    tenant_id: &str,
    election_event_id: &str,
    new_miru_signature: &MiruSignature,
    current_miru_signatures: &Vec<MiruSignature>,
) -> Result<(Vec<ACMTrustee>, Vec<MiruSignature>), String> {
    let election_event_str = get_election_event_by_id(tenant_id, election_event_id)?;

    let election_event: ElectionEvent = deserialize_str::<ElectionEvent>(&election_event_str)
        .map_err(|e| {
            println!(
                "[Guest Plugin Error] Failed to deserialize ElectionEvent: {}",
                e
            );
            e.to_string()
        })?;

    let event_annotations = election_event.get_annotations().map_err(|e| {
        println!(
            "[Guest Plugin Error] Failed to get election event annotations: {}",
            e
        );
        e.to_string()
    })?;

    let mut new_miru_signatures: Vec<MiruSignature> = current_miru_signatures
        .clone()
        .into_iter()
        .filter(|signature| signature.sbei_miru_id != new_miru_signature.sbei_miru_id)
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
        .map(|miru_signature| -> Result<ACMTrustee, String> {
            let trustee_annotations =
                trustees_map
                    .get(&miru_signature.sbei_miru_id)
                    .ok_or(format!(
                        "Can't find sbei by miru id {}",
                        miru_signature.sbei_miru_id
                    ))?;

            Ok(ACMTrustee {
                id: trustee_annotations.miru_id.clone(),
                signature: Some(miru_signature.signature.clone()),
                publickey: Some(miru_signature.pub_key.clone()),
                name: trustee_annotations.miru_name.clone(),
            })
        })
        .collect::<Result<_, String>>()?;

    Ok((acm_trustees, new_miru_signatures))
}

#[instrument(skip_all, err)]
pub fn check_sbei_certificate(
    transmission_data: &MiruTallySessionData,
    sbei: &MiruSbeiUser,
    area_id: &str,
    election_id: &str,
    use_root_ca: bool,
    p12_file_name: &str,
    password: &str,
    election_event_annotations: &MiruElectionEventAnnotations,
    dir_base_path: &str,
) -> Result<String, String> {
    let (_p12_cert_temp_file, p12_cert_path) =
        get_p12_cert(p12_file_name, password, dir_base_path)?;
    // return certificate fingerprint
    let input_pk_fingerprint = get_p12_fingerprint(&p12_cert_path)?;
    let found = transmission_data.clone().into_iter().find(|data| {
        if data.election_id == election_id {
            return false;
        }
        data.documents.clone().into_iter().any(|document| {
            document
                .signatures
                .clone()
                .iter()
                .any(|signature| signature.certificate_fingerprint == input_pk_fingerprint)
        })
    });
    if let Some(found_election) = found {
        return Err(format!(
            "Certificate {} already used in other post: '{}'",
            input_pk_fingerprint, found_election.election_id
        ));
    }

    if let Some(certificate_fingerprint) = sbei.certificate_fingerprint.clone() {
        if certificate_fingerprint != input_pk_fingerprint {
            return Err(format!(
                "User {} can't use certificate with fingerprint {}, only {}",
                sbei.username, input_pk_fingerprint, certificate_fingerprint
            ));
        }
    }
    if use_root_ca {
        check_certificate_cas(
            &p12_cert_path,
            &election_event_annotations.root_ca,
            &election_event_annotations.intermediate_cas,
            dir_base_path,
        )?;
    }
    Ok(input_pk_fingerprint)
}

#[instrument(skip_all, err)]
pub fn create_server_signature(
    eml_data_file_name: &str,
    sbei: &MiruSbeiUser,
    private_key_temp_file_name: &str,
    password: &str,
    public_key: &str,              // public key pem
    certificate_fingerprint: &str, // certificate fingerprint
) -> Result<MiruSignature, String> {
    let signature: String = host_create_server_signature(
        ECIES_TOOL_PATH,
        &private_key_temp_file_name,
        eml_data_file_name,
        password,
    )?;
    Ok(MiruSignature {
        sbei_miru_id: sbei.miru_id.clone(),
        pub_key: public_key.to_string(),
        signature: signature,
        certificate_fingerprint: certificate_fingerprint.to_string(),
    })
}

#[instrument(err)]
pub fn upload_transmission_package_signature_service(
    tenant_id: &str,
    election_id: &str,
    area_id: &str,
    tally_session_id: &str,
    username: &str,
    document_id: &str,
    password: &str,
) -> Result<(), String> {
    // get time
    let time_zone = PHILIPPINO_TIMEZONE;
    let now_utc = Utc::now();
    let now_local = now_utc.with_timezone(&Local);

    // get event and annotations
    let election_event_json = get_election_event_by_election_area(tenant_id, election_id, area_id)
        .map_err(|e| e.to_string())?;

    let election_event: ElectionEvent = deserialize_str::<ElectionEvent>(&election_event_json)
        .map_err(|e| {
            println!(
                "[Guest Plugin Error] Failed to deserialize ElectionEvent: {}",
                e
            );
            e.to_string()
        })?;

    let election_event_annotations = election_event.get_annotations().map_err(|e| {
        println!(
            "[Guest Plugin Error] Failed to get election event annotations: {}",
            e
        );
        e.to_string()
    })?;

    // get election and annotations
    let Some(election_str) = get_election_by_id(tenant_id, &election_event.id, election_id)
        .map_err(|e| e.to_string())?
    else {
        return Err("Election not found".to_string());
    };

    let election: Election = deserialize_str::<Election>(&election_str).map_err(|e| {
        println!("[Guest Plugin Error] Failed to deserialize Election: {}", e);
        e.to_string()
    })?;

    let election_annotations = election.get_annotations().map_err(|e| {
        println!(
            "[Guest Plugin Error] Failed to get election annotations: {}",
            e
        );
        e.to_string()
    })?;

    // get area and annotations
    let Some(area_str) = get_area_by_id(tenant_id, area_id).map_err(|e| e.to_string())? else {
        return Err("Area not found".to_string());
    };

    let area: Area = deserialize_str::<Area>(&area_str).map_err(|e| {
        println!("[Guest Plugin Error] Failed to deserialize Area: {}", e);
        e.to_string()
    })?;

    let area_name = area.name.clone().unwrap_or("".into());
    let area_annotations = area.get_annotations().map_err(|e| {
        println!("[Guest Plugin Error] Failed to get area annotations: {}", e);
        e.to_string()
    })?;

    // get sbei user
    let sbei_user_opt = election_event_annotations
        .sbei_users
        .clone()
        .into_iter()
        .find(|sbei| {
            sbei.username == username
                && area_annotations.sbei_ids.contains(&sbei.miru_id)
                && sbei.miru_election_id == election_annotations.election_id
        });

    let Some(sbei_user) = sbei_user_opt else {
        return Err(format!(
            "SBEI user not found area '{}' and username '{}'",
            area_name, username
        ));
    };

    let tally_session_json =
        get_tally_session_by_id(tenant_id, &election_event.id, tally_session_id).map_err(|e| {
            println!(
                "[Guest Plugin Error] Failed to get tally session by id: {}",
                e
            );
            e.to_string()
        })?;

    let tally_session: TallySession = deserialize_str::<TallySession>(&tally_session_json)
        .map_err(|e| {
            println!(
                "[Guest Plugin Error] Failed to deserialize TallySession: {}",
                e
            );
            e.to_string()
        })?;

    let transmission_data = tally_session.get_annotations().map_err(|e| {
        println!(
            "[Guest Plugin Error] Failed to get tally session annotations: {}",
            e
        );
        e.to_string()
    })?;

    let tally_annotations: Annotations = tally_session
        .annotations
        .clone()
        .map(
            |value| match deserialize_value::<HashMap<String, Value>>(value) {
                Ok(raw_map) => raw_map,
                Err(e) => {
                    println!(
                        "[Guest Plugin Error] Failed to deserialize tally annotations: {}",
                        e
                    );
                    HashMap::new()
                }
            },
        )
        .unwrap_or_default()
        .into_iter()
        .map(|(key, value)| {
            let string_val = match value {
                Value::String(s) => s,
                _ => value.to_string(),
            };
            (key, string_val)
        })
        .collect();

    let Some(transmission_area_election) = transmission_data.clone().into_iter().find(|data| {
        data.area_id == area_id.to_string() && data.election_id == election_id.to_string()
    }) else {
        println!("transmission package not found, skipping");
        return Ok(());
    };
    let Some(miru_document) = get_latest_miru_document(&transmission_area_election.documents)
    else {
        println!("transmission package document not found, skipping");
        return Ok(());
    };

    let private_key_document = get_document(tenant_id, Some(&election_event.id), &document_id)
        .map_err(|e| {
            println!("[Guest Plugin Error] Failed to get document by id: {}", e);
            e.to_string()
        })?;

    if private_key_document.is_none() {
        return Err(format!("Document with id {} not found", &document_id));
    }

    let private_key_document = private_key_document.unwrap();

    let mut private_key_temp_file = create_document_as_temp_file(tenant_id, &private_key_document)?;

    // download er file
    let document = get_document(
        tenant_id,
        Some(&election_event.id),
        &miru_document.document_ids.eml,
    )
    .map_err(|e| {
        println!("[Guest Plugin Error] Failed to get document by id: {}", e);
        e.to_string()
    })?;

    if document.is_none() {
        return Err(format!(
            "Document with id {} not found",
            &miru_document.document_ids.eml
        ));
    }

    let document = document.unwrap();

    let dir_base_path = get_plugin_shared_dir(&Plugins::MIRU);
    let eml_data_file_name = create_document_as_temp_file(tenant_id, &document)?;
    let eml_data = TempFileGuard::new(PathBuf::from(&dir_base_path).join(&eml_data_file_name));
    let eml_bytes: Vec<u8> = read_temp_file(&eml_data)?;
    let eml = String::from_utf8(eml_bytes).map_err(|e| {
        println!(
            "[Guest Plugin Error] Failed to convert EML bytes to string: {}",
            e
        );
        e.to_string()
    })?;

    // ECDSA sign er file
    let public_key_pem_string = derive_public_key_from_p12(&private_key_temp_file, password)?;

    let certificate_fingerprint = check_sbei_certificate(
        &transmission_data,
        &sbei_user,
        area_id,
        election_id,
        election_event_annotations.use_root_ca,
        &private_key_temp_file,
        password,
        &election_event_annotations,
        &dir_base_path,
    )?;

    let server_signature = create_server_signature(
        &eml_data_file_name,
        &sbei_user,
        &private_key_temp_file,
        password,
        &public_key_pem_string,
        &certificate_fingerprint,
    )?;

    if sbei_user.certificate_fingerprint.is_none() {
        update_election_event_sbei_users(
            &election_event,
            &election_event_annotations.sbei_users,
            &sbei_user,
            &certificate_fingerprint,
        )?;
    }

    let (new_acm_signatures, new_miru_signatures) = update_signatures(
        tenant_id,
        &election_event.id,
        &server_signature,
        &miru_document.signatures,
    )?;
    let mut new_signatures: Vec<MiruSignature> = miru_document
        .signatures
        .clone()
        .into_iter()
        .filter(|signature| signature.sbei_miru_id != sbei_user.miru_id)
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
        &rendered_xml_hash,
        &eml,
        compressed_xml,
        &area_annotations.ccs_servers,
        &area_annotations,
        &election_event_annotations,
        &election_event.id,
        tenant_id,
        time_zone.clone(),
        now_utc.clone(),
        new_acm_signatures,
        &new_transmission_package_data.logs,
        &election_annotations,
        &dir_base_path,
    )?;

    // // upload zip of zips
    let Some(first_document) = new_transmission_package_data.documents.first() else {
        return Err(format!("Missing initial document"));
    };
    new_transmission_package_data.documents.push(MiruDocument {
        document_ids: MiruDocumentIds {
            eml: first_document.document_ids.eml.clone(),
            xz: first_document.document_ids.xz.clone(),
            all_servers: all_servers_document.id.clone(),
        },
        transaction_id: first_document.transaction_id.clone(),
        servers_sent_to: vec![],
        created_at: now_local.to_rfc3339(),
        signatures: new_miru_signatures,
    });
    update_transmission_package_annotations(
        tenant_id,
        &election_event.id,
        tally_session_id,
        area_id,
        election_id,
        transmission_data.clone(),
        new_transmission_package_data,
        tally_annotations.clone(),
    )?;

    match commit_hasura_transaction() {
        Ok(_) => Ok(()),
        Err(e) => return Err(format!("Error committing hasura transaction: {}", e)),
    }
}
