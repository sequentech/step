// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The ER transmission test (T1 to T7) with the sample configuration and
//! kit in scripts/transmission-sample, offline: the configuration's
//! annotations read as the generator reads them, the returns are rendered
//! with the EML template and written as CSV, the package is sealed for the
//! sample destination and signed with the kit's SBEI certificates, and the
//! receiver's verifier accepts it and refuses altered copies.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{TimeZone as _, Utc};
use openssl::ec::{EcGroup, EcKey};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkcs12::Pkcs12;
use openssl::pkey::{PKey, Private};
use openssl::sign::Signer;
use sequent_core::ballot::{Annotations, Candidate, Contest};
use sequent_core::types::date_time::TimeZone;
use sequent_core::types::hasura::core::{Area, Election, ElectionEvent};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::path::PathBuf;
use velvet::pipes::do_tally::{CandidateResult, ContestResult, ExtendedMetricsContest};
use velvet::pipes::generate_reports::ReportData;
use windmill::services::consolidation::eml_generator::{
    render_eml_file, MiruAreaAnnotations, MiruElectionAnnotations, MiruElectionEventAnnotations,
    ValidateAnnotations,
};
use windmill::services::consolidation::eml_types::{ACMJson, ACMTrustee};
use windmill::services::consolidation::results_csv::{results_csv, ResultsCsvRow};
use windmill::services::consolidation::sealing::{ecies_encrypt, openssl_enc_encrypt};
use windmill::services::consolidation::transmission_package::{compress_hash_eml, render_eml};
use windmill::services::consolidation::verify_transmission_package::{
    verify_transmission_package, Refusal, VerifyOptions,
};
use windmill::types::miru_plugin::{MiruCcsServer, MiruDelivery};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

const P12_PASSWORD: &str = "sample";
const TRANSACTION_ID: &str = "0000001234";

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn sample() -> PathBuf {
    repository().join("scripts/transmission-sample")
}

fn kit(name: &str) -> Vec<u8> {
    std::fs::read(sample().join("kit").join(name)).unwrap()
}

fn config() -> Value {
    serde_json::from_slice(
        &std::fs::read(sample().join("transmission-sample-config.json")).unwrap(),
    )
    .unwrap()
}

/// Annotations as Step stores them, every value a string, as the loader
/// writes them.
fn annotations(value: &Value) -> Annotations {
    value
        .as_object()
        .unwrap()
        .iter()
        .map(|(key, value)| {
            let text = match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            (key.clone(), text)
        })
        .collect()
}

fn row(extra: Value, annotated: &Value) -> Value {
    let mut row: Map<String, Value> = serde_json::from_value(extra).unwrap();
    row.insert("id".into(), json!("00000000-0000-4000-8000-000000000001"));
    row.insert(
        "tenant_id".into(),
        json!("00000000-0000-4000-8000-000000000002"),
    );
    row.insert(
        "annotations".into(),
        serde_json::to_value(annotations(annotated)).unwrap(),
    );
    Value::Object(row)
}

struct Sample {
    event: MiruElectionEventAnnotations,
    post: MiruElectionAnnotations,
    country: MiruAreaAnnotations,
    contests: Vec<Contest>,
}

/// T1: every value the generator reads, read from the configuration.
fn read_sample() -> Sample {
    let config = config();
    let event: ElectionEvent = serde_json::from_value(row(
        json!({"is_archived": false, "encryption_protocol": "RSA256"}),
        &config["election_event"]["annotations"],
    ))
    .unwrap();
    let post: Election = serde_json::from_value(row(
        json!({"election_event_id": "e"}),
        &config["post"]["annotations"],
    ))
    .unwrap();
    let country: Area = serde_json::from_value(row(
        json!({"election_event_id": "e"}),
        &config["country"]["annotations"],
    ))
    .unwrap();
    let contests = config["post"]["contests"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(index, contest)| Contest {
            id: format!("contest-{index}"),
            annotations: Some(annotations(&contest["annotations"])),
            candidates: contest["candidates"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(position, candidate)| Candidate {
                    id: format!("candidate-{index}-{position}"),
                    annotations: Some(annotations(&candidate["annotations"])),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        })
        .collect();
    Sample {
        event: event.get_annotations().unwrap(),
        post: post.get_annotations().unwrap(),
        country: country.get_annotations().unwrap(),
        contests,
    }
}

/// Fictitious returns: each candidate's votes, by contest.
fn votes(contest: usize, candidate: usize) -> u64 {
    [[412, 377, 58], [455, 391, 0]][contest][candidate]
}

fn reports(sample: &Sample) -> Vec<ReportData> {
    sample
        .contests
        .iter()
        .enumerate()
        .map(|(index, contest)| {
            let candidate_result: Vec<CandidateResult> = contest
                .candidates
                .iter()
                .enumerate()
                .map(|(position, candidate)| CandidateResult {
                    candidate: candidate.clone(),
                    percentage_votes: 0.0,
                    total_count: votes(index, position),
                })
                .collect();
            let valid: u64 = candidate_result.iter().map(|r| r.total_count).sum();
            ReportData {
                election_name: "Madrid PE".into(),
                election_alias: "".into(),
                election_id: "post".into(),
                election_event_id: "event".into(),
                tenant_id: "tenant".into(),
                election_description: "".into(),
                election_dates: None,
                election_annotations: HashMap::new(),
                election_event_annotations: HashMap::new(),
                contest: Some(contest.clone()),
                area: None,
                contest_result: Some(ContestResult {
                    contest: contest.clone(),
                    total_votes: valid + 12,
                    total_valid_votes: valid,
                    total_invalid_votes: 12,
                    candidate_result,
                    extended_metrics: Some(ExtendedMetricsContest {
                        under_votes: 3,
                        votes_actually: valid,
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                winners: vec![],
                channel_type: None,
                election_results: None,
            }
        })
        .collect()
}

struct Returns {
    eml: String,
    hash: String,
    compressed: Vec<u8>,
    csv: String,
}

/// T2: the returns rendered with Step's EML template, and their CSV.
fn returns(sample: &Sample) -> Returns {
    let eml_data = render_eml_file(
        "tally",
        TRANSACTION_ID,
        TimeZone::Offset(2),
        Utc.with_ymd_and_hms(2028, 5, 8, 17, 0, 0).unwrap(),
        &sample.event,
        &sample.post,
        &sample.country,
        &reports(sample),
    )
    .unwrap();
    let template = std::fs::read_to_string(
        repository().join(".devcontainer/minio/public-assets/eml_base.hbs"),
    )
    .unwrap();
    let csv = results_csv(&eml_data, &sample.post, &sample.country).unwrap();
    let eml = render_eml(&template, serde_json::to_value(&eml_data).unwrap(), None).unwrap();
    let (compressed, hash) = compress_hash_eml(&eml).unwrap();
    Returns {
        eml,
        hash,
        compressed,
        csv,
    }
}

fn sign(key: &PKey<Private>, data: &[u8]) -> String {
    let mut signer = Signer::new(MessageDigest::sha256(), key).unwrap();
    signer.update(data).unwrap();
    BASE64.encode(signer.sign_to_vec().unwrap())
}

/// T3: an SBEI's signature of the EML with their kit certificate, as the
/// package's members list carries it.
fn sbei_member(sample: &Sample, index: usize, eml: &str) -> ACMTrustee {
    let sbei = &sample.event.sbei_users[index];
    let parsed = Pkcs12::from_der(&kit(&format!("{}.p12", sbei.username)))
        .unwrap()
        .parse2(P12_PASSWORD)
        .unwrap();
    let key = parsed.pkey.unwrap();
    let publickey = String::from_utf8(key.public_key_to_pem().unwrap()).unwrap();
    ACMTrustee {
        id: sbei.miru_id.clone(),
        signature: Some(sign(&key, eml.as_bytes())),
        publickey: Some(publickey),
        name: sbei.miru_name.clone(),
    }
}

struct Ovcs {
    key: PKey<Private>,
    public_pem: Vec<u8>,
}

fn ovcs() -> Ovcs {
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
    let key = PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap();
    let public_pem = key.public_key_to_pem().unwrap();
    Ovcs { key, public_pem }
}

fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        writer
            .start_file(*name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// The package for the sample's destination, as the OVCS seals it: the
/// compressed EML encrypted under a random password, the password
/// encrypted for the destination's key, and the JSON with the EML's
/// SHA-256 and the signatures.
fn seal(
    sample: &Sample,
    destination: &MiruCcsServer,
    compressed: &[u8],
    declared_hash: &str,
    ovcs: &Ovcs,
    signed_eml: &str,
    members: Vec<ACMTrustee>,
) -> Vec<u8> {
    let mut random = [0u8; 32];
    openssl::rand::rand_bytes(&mut random).unwrap();
    let password = hex::encode(random);
    let exz = openssl_enc_encrypt(password.as_bytes(), compressed).unwrap();
    let destination_key = PKey::public_key_from_pem(destination.public_key_pem.as_bytes()).unwrap();
    let acm = ACMJson {
        device_id: "OVCS-0001".into(),
        serial_number: "SN-2028-0001".into(),
        station_id: sample.country.station_id.clone(),
        station_name: sample.country.station_name.clone(),
        event_id: sample.event.event_id.clone(),
        event_name: sample.event.event_name.clone(),
        sha256_hash: declared_hash.into(),
        encrypted_key: BASE64.encode(ecies_encrypt(&destination_key, password.as_bytes()).unwrap()),
        members,
        ip_address: "10.0.0.7".into(),
        mac_address: "3C:7E:5A:89:4D:2F".into(),
        er_datetime: "05/08/2028 07:00:00 PM".into(),
        signature: sign(&ovcs.key, signed_eml.as_bytes()),
        publickey: String::from_utf8(ovcs.public_pem.clone()).unwrap(),
        transfer_start: "05/08/2028 07:00:00 PM".into(),
    };
    let station = &sample.country.station_id;
    let er = zip(&[
        (&format!("er_{station}.exz"), &exz),
        (
            &format!("er_{station}.json"),
            serde_json::to_string_pretty(&acm).unwrap().as_bytes(),
        ),
    ]);
    zip(&[(&format!("{}/er_{station}.zip", destination.tag), &er)])
}

fn options(ovcs: &Ovcs, key: &[u8]) -> VerifyOptions {
    let mut ca = kit("root-ca.pem");
    ca.extend(kit("intermediate-ca.pem"));
    VerifyOptions::new(key, &ovcs.public_pem)
        .unwrap()
        .with_member_certificates(Some(&ca), Some(&kit("sbei-certificates.pem")))
        .unwrap()
        .with_required_signatures(read_sample().country.threshold as usize)
}

#[test]
fn t1_the_sample_configuration_holds_every_value_the_generator_reads() {
    let sample = read_sample();
    assert_eq!(sample.event.event_id, "OV2028");
    assert_eq!(sample.event.sbei_users.len(), 3);
    assert!(sample.event.use_root_ca);
    assert_eq!(sample.post.election_name, "Madrid PE");
    assert_eq!(sample.country.country, "Spain");
    assert_eq!(sample.country.threshold, 2);
    assert_eq!(sample.country.sbei_ids.len(), 3);
    for contest in &sample.contests {
        contest.get_annotations().unwrap();
        for candidate in &contest.candidates {
            candidate.get_annotations().unwrap();
        }
    }
    let (destination,) = match sample.country.ccs_servers.as_slice() {
        [destination] => (destination,),
        servers => panic!("one destination expected, got {servers:?}"),
    };
    assert_eq!(destination.name, "CCS Manila (SBOC)");
    assert_eq!(destination.delivery(), MiruDelivery::ManualReceipt);
}

#[test]
fn t2_t5_the_returns_are_eml_510_and_the_csv_has_the_same_votes() {
    let sample = read_sample();
    let returns = returns(&sample);
    assert!(returns.eml.contains("<EML Id=\"510\""), "{}", returns.eml);
    assert!(returns
        .eml
        .contains(&format!("<TransactionId>{TRANSACTION_ID}</TransactionId>")));
    assert!(returns
        .eml
        .contains("<ContestName>PRESIDENT OF THE PHILIPPINES</ContestName>"));
    assert_eq!(returns.hash.len(), 64);

    let mut reader = csv::Reader::from_reader(returns.csv.as_bytes());
    let rows: Vec<csv::StringRecord> = reader.records().map(|r| r.unwrap()).collect();
    let candidates: Vec<(String, u64)> = rows
        .iter()
        .filter(|row| &row[6] == ResultsCsvRow::Candidate.to_string().as_str())
        .map(|row| (row[7].to_string(), row[10].parse().unwrap()))
        .collect();
    let expected: Vec<(String, u64)> = sample
        .contests
        .iter()
        .enumerate()
        .flat_map(|(index, contest)| {
            contest
                .candidates
                .iter()
                .enumerate()
                .map(move |(position, candidate)| {
                    (
                        candidate.get_annotations().unwrap().candidate_id,
                        votes(index, position),
                    )
                })
        })
        .collect();
    assert_eq!(candidates, expected);
    for (candidate_id, votes) in &expected {
        let at = returns
            .eml
            .find(&format!("IdNumber=\"{candidate_id}\""))
            .unwrap();
        let valid = &returns.eml[at..];
        let start = valid.find("<ValidVotes>").unwrap() + "<ValidVotes>".len();
        assert_eq!(
            &valid[start..start + valid[start..].find('<').unwrap()],
            votes.to_string()
        );
    }
    let registered: Vec<&str> = rows
        .iter()
        .filter(|row| &row[6] == "RV")
        .map(|row| &row[10])
        .collect();
    assert_eq!(registered, vec!["1250", "1250"]);
    assert!(rows
        .iter()
        .all(|row| &row[1] == "90010001" && &row[3] == "Spain"));
}

#[test]
fn t3_t6_the_signed_package_verifies_on_receipt() {
    let sample = read_sample();
    let returns = returns(&sample);
    let ovcs = ovcs();
    let members = vec![
        sbei_member(&sample, 0, &returns.eml),
        sbei_member(&sample, 1, &returns.eml),
    ];
    let destination = &sample.country.ccs_servers[0];
    let package = seal(
        &sample,
        destination,
        &returns.compressed,
        &returns.hash,
        &ovcs,
        &returns.eml,
        members,
    );
    let options = options(&ovcs, &kit("ccs-manila-private-key.pem"))
        .with_expected_sha256(&returns.hash.to_lowercase())
        .unwrap();
    let verified = verify_transmission_package(&package, &options).unwrap();
    assert_eq!(verified.destination.as_deref(), Some("ccs-manila"));
    assert_eq!(verified.station_id, "90010001");
    assert_eq!(verified.transaction_id.as_deref(), Some(TRANSACTION_ID));
    assert_eq!(verified.eml_sha256, returns.hash);
    assert_eq!(verified.valid_signatures, 2);
    assert_eq!(verified.eml, returns.eml.as_bytes());
    let president = &verified.contests[0];
    assert_eq!(president.id, "1");
    let counted: Vec<i64> = president
        .selections
        .iter()
        .map(|selection| selection.valid_votes)
        .collect();
    assert_eq!(counted, vec![412, 377, 58]);
}

#[test]
fn t7_altered_copies_are_refused() {
    let sample = read_sample();
    let returns = returns(&sample);
    let ovcs = ovcs();
    let destination = &sample.country.ccs_servers[0];
    let members = |eml: &str| vec![sbei_member(&sample, 0, eml), sbei_member(&sample, 1, eml)];
    let key = kit("ccs-manila-private-key.pem");
    let refused = |package: &[u8], key: &[u8]| {
        verify_transmission_package(package, &options(&ovcs, key))
            .expect_err("the altered copy must be refused")
            .refusals
    };

    // A changed vote count, sealed again with the original hash and
    // signatures.
    let changed = returns.eml.replacen(
        "<ValidVotes>412</ValidVotes>",
        "<ValidVotes>512</ValidVotes>",
        1,
    );
    assert_ne!(changed, returns.eml);
    let (changed_compressed, changed_hash) = compress_hash_eml(&changed).unwrap();
    let resealed = seal(
        &sample,
        destination,
        &changed_compressed,
        &returns.hash,
        &ovcs,
        &returns.eml,
        members(&returns.eml),
    );
    assert!(refused(&resealed, &key)
        .iter()
        .any(|refusal| matches!(refusal, Refusal::HashMismatch { .. })));

    // The same, with the SHA-256 replaced too: the signatures give it away.
    let forged = seal(
        &sample,
        destination,
        &changed_compressed,
        &changed_hash,
        &ovcs,
        &returns.eml,
        members(&returns.eml),
    );
    let forged_refusals = refused(&forged, &key);
    assert!(forged_refusals
        .iter()
        .all(|refusal| !matches!(refusal, Refusal::HashMismatch { .. })));
    assert!(forged_refusals
        .iter()
        .any(|refusal| matches!(refusal, Refusal::OvcsSignatureInvalid)));
    assert!(forged_refusals
        .iter()
        .any(|refusal| matches!(refusal, Refusal::MemberSignatureInvalid { .. })));

    // One SBEI signature removed: below the signatures needed.
    let mut one = members(&returns.eml);
    one.pop();
    let short = seal(
        &sample,
        destination,
        &returns.compressed,
        &returns.hash,
        &ovcs,
        &returns.eml,
        one,
    );
    assert!(refused(&short, &key).iter().any(|refusal| matches!(
        refusal,
        Refusal::TooFewSignatures {
            required: 2,
            valid: 1
        }
    )));

    // A flipped byte anywhere in the destination's entry.
    let package = seal(
        &sample,
        destination,
        &returns.compressed,
        &returns.hash,
        &ovcs,
        &returns.eml,
        members(&returns.eml),
    );
    verify_transmission_package(&package, &options(&ovcs, &key)).unwrap();
    let mut flipped = package.clone();
    let middle = flipped.len() / 2;
    flipped[middle] ^= 0x01;
    assert!(!refused(&flipped, &key).is_empty());

    // Another destination's key does not open it.
    assert!(refused(&package, &kit("other-destination-private-key.pem"))
        .iter()
        .any(|refusal| matches!(refusal, Refusal::SealedForAnotherKey { .. })));
}
