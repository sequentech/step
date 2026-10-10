// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::{
    create_transmission_package_service::{
        generate_all_servers_document, update_transmission_package_annotations,
    },
    eml_generator::{
        find_miru_annotation, prepend_miru_annotation, MiruElectionEventAnnotations,
        ValidateAnnotations, MIRU_AREA_CCS_SERVERS, MIRU_AREA_STATION_ID, MIRU_AREA_TRUSTEE_USERS,
        MIRU_PLUGIN_PREPEND, MIRU_SBEI_USERS, MIRU_TALLY_SESSION_DATA, MIRU_TRUSTEE_ID,
        MIRU_TRUSTEE_NAME,
    },
    eml_types::ACMTrustee,
    logs::{
        create_transmission_package_log, error_sending_transmission_package_to_ccs_log,
        send_transmission_package_to_ccs_log, sign_transmission_package_log,
    },
    rsa::rsa_sign_data,
    send_transmission_package_service::get_latest_miru_document,
    signatures::{
        check_certificate_cas, ecdsa_sign_data, get_certificate_public_key_pem,
        get_p12_certificate, get_p12_fingerprint, get_pk12_id, verify_certificate_signature,
        write_certificate_pem,
    },
    transmission_package::{compress_hash_eml, create_transmission_package},
    zip::unzip_file,
};
use crate::postgres::election_event::update_election_event_annotations;
use crate::{
    postgres::{
        area::get_area_by_id, document::get_document, election::get_election_by_id,
        election_event::get_election_event_by_election_area,
        tally_session::get_tally_session_by_id,
    },
    services::{
        database::get_hasura_pool,
        documents::{get_document_as_temp_file, upload_and_return_document},
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
use openssl::x509::X509;
use reqwest::multipart;
use sequent_core::{
    ballot::Annotations,
    serialization::deserialize_with_path::{deserialize_str, deserialize_value},
    services::date::ISO8601,
    types::hasura::core::{ElectionEvent, Trustee},
    util::date_time::PHILIPPINO_TIMEZONE,
};
use sequent_core::{
    services::translations::Name,
    util::temp_path::{generate_temp_file, get_file_size, read_temp_file},
};
use std::collections::HashMap;
use std::fs;
use tempfile::NamedTempFile;
use tracing::{info, instrument};

#[instrument(skip_all, err)]
async fn update_election_event_sbei_users(
    hasura_transaction: &Transaction<'_>,
    election_event: &ElectionEvent,
    sbei_users: &Vec<MiruSbeiUser>,
    sbei_user: &MiruSbeiUser,
    certificate_fingerprint: &str,
) -> Result<()> {
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
        .ok_or_else(|| anyhow!("Missing election event annotations"))?;

    let mut annotations: Annotations = deserialize_value(annotations_js)?;
    let key = prepend_miru_annotation(MIRU_SBEI_USERS);
    let serialized_sbei_users = serde_json::to_string(&new_sbei_users)?;
    annotations.insert(key, serialized_sbei_users);
    let annotations_js = serde_json::to_value(&annotations)?;

    update_election_event_annotations(
        hasura_transaction,
        &election_event.tenant_id,
        &election_event.id,
        annotations_js,
    )
    .await
}

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
        .map(|miru_signature| -> Result<ACMTrustee> {
            let trustee_annotations =
                trustees_map
                    .get(&miru_signature.sbei_miru_id)
                    .ok_or(anyhow!(
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
        .collect::<Result<_>>()?;

    Ok((acm_trustees, new_miru_signatures))
}

#[instrument(skip_all, err)]
pub fn check_sbei_certificate(
    transmission_data: &MiruTallySessionData,
    sbei: &MiruSbeiUser,
    area_id: &str,
    election_id: &str,
    use_root_ca: bool,
    certificate: &X509,
    election_event_annotations: &MiruElectionEventAnnotations,
) -> Result<String> {
    let p12_cert_path = write_certificate_pem(certificate)?;
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
        return Err(anyhow!(
            "Certificate {} already used in other post: '{}'",
            input_pk_fingerprint,
            found_election.election_id
        ));
    }

    if let Some(certificate_fingerprint) = sbei.certificate_fingerprint.clone() {
        if certificate_fingerprint != input_pk_fingerprint {
            return Err(anyhow!(
                "User {} can't use certificate with fingerprint {}, only {}",
                sbei.username,
                input_pk_fingerprint,
                certificate_fingerprint
            ));
        }
    }
    if use_root_ca {
        check_certificate_cas(
            &p12_cert_path,
            &election_event_annotations.root_ca,
            &election_event_annotations.intermediate_cas,
        )?;
    }
    Ok(input_pk_fingerprint)
}

#[instrument(skip_all, err)]
pub fn create_server_signature(
    eml_data: NamedTempFile,
    sbei: &MiruSbeiUser,
    private_key_temp_file: &NamedTempFile,
    password: &str,
    certificate: &X509,
    certificate_fingerprint: &str, // certificate fingerprint
) -> Result<MiruSignature> {
    let temp_pem_file_path = eml_data.path();
    let temp_pem_file_string = temp_pem_file_path.to_string_lossy().to_string();

    let pk12_file_path = private_key_temp_file.path();
    let pk12_file_path_string = pk12_file_path.to_string_lossy().to_string();

    let pk12_id = get_pk12_id(&pk12_file_path_string, password)?;

    let signature = match pk12_id {
        openssl::pkey::Id::RSA => {
            rsa_sign_data(&pk12_file_path_string, password, &temp_pem_file_string)?
        }
        openssl::pkey::Id::EC => {
            ecdsa_sign_data(&pk12_file_path_string, password, &temp_pem_file_string)?
        }
        _ => {
            return Err(anyhow!("Unexpected p12 key {:?}", pk12_id));
        }
    };
    let signed_data = fs::read(temp_pem_file_path).with_context(|| "Error reading eml file")?;
    verify_certificate_signature(certificate, &signed_data, &signature)?;
    Ok(MiruSignature {
        sbei_miru_id: sbei.miru_id.clone(),
        pub_key: get_certificate_public_key_pem(certificate)?,
        signature: signature,
        certificate_fingerprint: certificate_fingerprint.to_string(),
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
    // open postgres transaction
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .with_context(|| "Error acquiring hasura connection pool")?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .with_context(|| "Error acquiring hasura transaction")?;

    // get time
    let time_zone = PHILIPPINO_TIMEZONE;
    let now_utc = Utc::now();
    let now_local = now_utc.with_timezone(&Local);

    // get event and annotations
    let election_event =
        get_election_event_by_election_area(&hasura_transaction, tenant_id, election_id, area_id)
            .await
            .with_context(|| "Error fetching election event")?;

    let election_event_annotations = election_event.get_annotations()?;

    // get election and annotations
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

    // get area and annotations
    let area = get_area_by_id(&hasura_transaction, tenant_id, &area_id)
        .await
        .with_context(|| format!("Error fetching area {}", area_id))?
        .ok_or_else(|| anyhow!("Can't find area {}", area_id))?;
    let area_name = area.name.clone().unwrap_or("".into());
    let area_annotations = area.get_annotations()?;

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

    // ECDSA sign er file
    let certificate = get_p12_certificate(&private_key_temp_file, password)?;

    let certificate_fingerprint = check_sbei_certificate(
        &transmission_data,
        &sbei_user,
        area_id,
        election_id,
        election_event_annotations.use_root_ca,
        &certificate,
        &election_event_annotations,
    )?;

    let server_signature = create_server_signature(
        eml_data,
        &sbei_user,
        &private_key_temp_file,
        password,
        &certificate,
        &certificate_fingerprint,
    )?;

    if sbei_user.certificate_fingerprint.is_none() {
        update_election_event_sbei_users(
            &hasura_transaction,
            &election_event,
            &election_event_annotations.sbei_users,
            &sbei_user,
            &certificate_fingerprint,
        )
        .await?;
    }

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
        .filter(|signature| signature.sbei_miru_id != sbei_user.miru_id)
        .collect();
    new_signatures.push(server_signature.clone());
    // generate zip of zips
    let mut new_transmission_package_data = transmission_area_election.clone();

    let election_name = election.get_name(&election.get_default_language());
    new_transmission_package_data
        .logs
        .push(sign_transmission_package_log(
            &now_local,
            election_id,
            &election_name,
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
        &area_annotations,
        &election_event_annotations,
        &election_event.id,
        tenant_id,
        time_zone.clone(),
        now_utc.clone(),
        new_acm_signatures,
        &new_transmission_package_data.logs,
        &election_annotations,
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

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose, Engine as _};
    use openssl::asn1::Asn1Time;
    use openssl::bn::{BigNum, MsbOption};
    use openssl::ec::{EcGroup, EcKey};
    use openssl::hash::MessageDigest;
    use openssl::nid::Nid;
    use openssl::pkcs12::Pkcs12;
    use openssl::pkey::{PKey, Private};
    use openssl::rsa::Rsa;
    use openssl::sign::Signer;
    use openssl::stack::Stack;
    use openssl::x509::extension::{BasicConstraints, KeyUsage};
    use openssl::x509::{X509Builder, X509NameBuilder};
    use std::io::Write;

    const P12_PASSWORD: &str = "p12-password";
    const AREA_ID: &str = "area-id";
    const ELECTION_ID: &str = "election-id";
    const SIGNED_DATA: &[u8] = b"<EML>signed data</EML>";

    struct TestPki {
        root: X509,
        intermediate: X509,
        intermediate_key: PKey<Private>,
    }

    fn ec_key() -> Result<PKey<Private>> {
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)?;
        Ok(PKey::from_ec_key(EcKey::generate(&group)?)?)
    }

    fn p384_key() -> Result<PKey<Private>> {
        let group = EcGroup::from_curve_name(Nid::SECP384R1)?;
        Ok(PKey::from_ec_key(EcKey::generate(&group)?)?)
    }

    fn rsa_key(public_exponent: u32) -> Result<PKey<Private>> {
        let public_exponent = BigNum::from_u32(public_exponent)?;
        Ok(PKey::from_rsa(Rsa::generate_with_e(
            2048,
            &public_exponent,
        )?)?)
    }

    fn sign_base64(key: &PKey<Private>, data: &[u8]) -> Result<String> {
        let mut signer = Signer::new(MessageDigest::sha256(), key)?;
        Ok(general_purpose::STANDARD.encode(signer.sign_oneshot_to_vec(data)?))
    }

    fn certificate(
        common_name: &str,
        key: &PKey<Private>,
        issuer: Option<(&X509, &PKey<Private>)>,
        is_ca: bool,
    ) -> Result<X509> {
        let mut name_builder = X509NameBuilder::new()?;
        name_builder.append_entry_by_nid(Nid::COMMONNAME, common_name)?;
        let name = name_builder.build();

        let mut serial = BigNum::new()?;
        serial.rand(64, MsbOption::MAYBE_ZERO, false)?;
        let serial = serial.to_asn1_integer()?;
        let not_before = Asn1Time::days_from_now(0)?;
        let not_after = Asn1Time::days_from_now(30)?;

        let mut builder = X509Builder::new()?;
        builder.set_version(2)?;
        builder.set_serial_number(&serial)?;
        builder.set_subject_name(&name)?;
        builder.set_pubkey(key)?;
        builder.set_not_before(&not_before)?;
        builder.set_not_after(&not_after)?;
        if is_ca {
            builder.append_extension(BasicConstraints::new().critical().ca().build()?)?;
            builder.append_extension(KeyUsage::new().critical().key_cert_sign().build()?)?;
        }
        let signing_key = match issuer {
            Some((issuer_certificate, issuer_key)) => {
                builder.set_issuer_name(issuer_certificate.subject_name())?;
                issuer_key
            }
            None => {
                builder.set_issuer_name(&name)?;
                key
            }
        };
        builder.sign(signing_key, MessageDigest::sha256())?;
        Ok(builder.build())
    }

    fn test_pki() -> Result<TestPki> {
        let root_key = ec_key()?;
        let root = certificate("Test Root CA", &root_key, None, true)?;
        let intermediate_key = ec_key()?;
        let intermediate = certificate(
            "Test Intermediate CA",
            &intermediate_key,
            Some((&root, &root_key)),
            true,
        )?;
        Ok(TestPki {
            root,
            intermediate,
            intermediate_key,
        })
    }

    fn issue_sbei_certificate(pki: &TestPki, key: &PKey<Private>) -> Result<X509> {
        certificate(
            "SBEI",
            key,
            Some((&pki.intermediate, &pki.intermediate_key)),
            false,
        )
    }

    fn p12_file(
        key: &PKey<Private>,
        certificate: Option<&X509>,
        cas: &[&X509],
    ) -> Result<NamedTempFile> {
        let mut builder = Pkcs12::builder();
        builder.name("sbei").pkey(key);
        if let Some(certificate) = certificate {
            builder.cert(certificate);
        }
        if !cas.is_empty() {
            let mut stack = Stack::new()?;
            for ca in cas {
                stack.push((*ca).clone())?;
            }
            builder.ca(stack);
        }
        let der = builder.build2(P12_PASSWORD)?.to_der()?;
        let mut file = NamedTempFile::new()?;
        file.write_all(&der)?;
        Ok(file)
    }

    fn event_annotations(pki: &TestPki) -> Result<MiruElectionEventAnnotations> {
        Ok(MiruElectionEventAnnotations {
            event_id: "event-id".to_string(),
            event_name: "event".to_string(),
            sbei_users: vec![],
            root_ca: String::from_utf8(pki.root.to_pem()?)?,
            intermediate_cas: String::from_utf8(pki.intermediate.to_pem()?)?,
            use_root_ca: true,
        })
    }

    fn sbei_user(certificate_fingerprint: Option<String>) -> MiruSbeiUser {
        MiruSbeiUser {
            username: "sbei".to_string(),
            miru_id: "sbei-miru-id".to_string(),
            miru_role: "role".to_string(),
            miru_name: "SBEI".to_string(),
            miru_election_id: "miru-election-id".to_string(),
            certificate_fingerprint,
        }
    }

    fn sbei_certificate_fingerprint(
        p12_file: &NamedTempFile,
        sbei: &MiruSbeiUser,
        annotations: &MiruElectionEventAnnotations,
    ) -> Result<String> {
        let certificate = get_p12_certificate(p12_file, P12_PASSWORD)?;
        check_sbei_certificate(
            &vec![],
            sbei,
            AREA_ID,
            ELECTION_ID,
            annotations.use_root_ca,
            &certificate,
            annotations,
        )
    }

    fn sha256_fingerprint_hex(certificate: &X509) -> Result<String> {
        Ok(certificate
            .digest(MessageDigest::sha256())?
            .iter()
            .map(|byte| format!("{:02X}", byte))
            .collect::<Vec<_>>()
            .join(":"))
    }

    #[test]
    fn check_sbei_certificate_accepts_key_with_its_ca_issued_certificate() -> Result<()> {
        let pki = test_pki()?;
        let sbei_key = ec_key()?;
        let sbei_certificate = issue_sbei_certificate(&pki, &sbei_key)?;
        let p12 = p12_file(&sbei_key, Some(&sbei_certificate), &[&pki.intermediate])?;

        let fingerprint =
            sbei_certificate_fingerprint(&p12, &sbei_user(None), &event_annotations(&pki)?)?;

        assert!(fingerprint.ends_with(&format!(
            "Fingerprint={}",
            sha256_fingerprint_hex(&sbei_certificate)?
        )));
        Ok(())
    }

    #[test]
    fn check_sbei_certificate_rejects_key_without_its_certificate() -> Result<()> {
        let pki = test_pki()?;
        let other_key = ec_key()?;
        let other_certificate = issue_sbei_certificate(&pki, &other_key)?;
        let p12 = p12_file(&ec_key()?, None, &[&other_certificate])?;

        let result =
            sbei_certificate_fingerprint(&p12, &sbei_user(None), &event_annotations(&pki)?);

        assert!(result.is_err(), "unexpected fingerprint {:?}", result);
        Ok(())
    }

    #[test]
    fn check_sbei_certificate_rejects_key_paired_with_ca_certificate() -> Result<()> {
        let pki = test_pki()?;
        let p12 = p12_file(&ec_key()?, None, &[&pki.intermediate])?;

        let result =
            sbei_certificate_fingerprint(&p12, &sbei_user(None), &event_annotations(&pki)?);

        assert!(result.is_err(), "unexpected fingerprint {:?}", result);
        Ok(())
    }

    #[test]
    fn check_sbei_certificate_rejects_pinned_certificate_with_other_key() -> Result<()> {
        let pki = test_pki()?;
        let sbei_key = ec_key()?;
        let sbei_certificate = issue_sbei_certificate(&pki, &sbei_key)?;
        let annotations = event_annotations(&pki)?;
        let sbei_p12 = p12_file(&sbei_key, Some(&sbei_certificate), &[&pki.intermediate])?;
        let pinned_fingerprint =
            sbei_certificate_fingerprint(&sbei_p12, &sbei_user(None), &annotations)?;
        let pinned_sbei = sbei_user(Some(pinned_fingerprint));
        let other_key_p12 = p12_file(&ec_key()?, None, &[&sbei_certificate])?;

        let result = sbei_certificate_fingerprint(&other_key_p12, &pinned_sbei, &annotations);

        assert!(result.is_err(), "unexpected fingerprint {:?}", result);
        assert!(sbei_certificate_fingerprint(&sbei_p12, &pinned_sbei, &annotations).is_ok());
        Ok(())
    }

    #[test]
    fn published_public_key_is_the_p12_certificate_key() -> Result<()> {
        for key in [ec_key()?, p384_key()?, rsa_key(65537)?, rsa_key(3)?] {
            let key_certificate = certificate("SBEI", &key, None, false)?;
            let p12 = p12_file(&key, Some(&key_certificate), &[])?;

            let p12_certificate = get_p12_certificate(&p12, P12_PASSWORD)?;
            let public_key_pem = get_certificate_public_key_pem(&p12_certificate)?;

            assert!(PKey::public_key_from_pem(public_key_pem.as_bytes())?.public_eq(&key));
        }
        Ok(())
    }

    #[test]
    fn signature_check_accepts_signature_by_certificate_key() -> Result<()> {
        for key in [ec_key()?, p384_key()?, rsa_key(65537)?] {
            let key_certificate = certificate("SBEI", &key, None, false)?;

            verify_certificate_signature(
                &key_certificate,
                SIGNED_DATA,
                &sign_base64(&key, SIGNED_DATA)?,
            )?;
        }
        Ok(())
    }

    #[test]
    fn signature_check_rejects_signature_by_other_key() -> Result<()> {
        for (key, other_key) in [(ec_key()?, ec_key()?), (rsa_key(65537)?, rsa_key(65537)?)] {
            let key_certificate = certificate("SBEI", &key, None, false)?;

            let result = verify_certificate_signature(
                &key_certificate,
                SIGNED_DATA,
                &sign_base64(&other_key, SIGNED_DATA)?,
            );

            assert!(result.is_err());
        }
        Ok(())
    }
}
