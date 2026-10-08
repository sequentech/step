// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::messages::message::SigningData;
use base64::Engine as _;
use strand::signature::StrandSignatureSk;

fn entry(content: &str, id: &str, disposition: SealDisposition, weight: u64) -> SealEntry {
    SealEntry {
        ballot_hash: ballot_hash(content).unwrap(),
        ballot_id: id.to_string(),
        disposition,
        weight,
        channel: DEFAULT_SEAL_CHANNEL.to_string(),
    }
}

fn manifest(entries: Vec<SealEntry>) -> BallotBoxSealManifest {
    BallotBoxSealManifest {
        format: SEAL_FORMAT_V1.to_string(),
        tenant_id: "tenant".to_string(),
        election_event_id: "event".to_string(),
        election_id: "election".to_string(),
        area_id: "area".to_string(),
        closed_at: 1_841_396_472,
        grace_deadline: 1_841_396_472,
        sealed_at: 1_841_396_474,
        close_request_id: Some("request".to_string()),
        eligible_voters: 4,
        entries,
    }
}

fn sample_entries() -> Vec<SealEntry> {
    vec![
        entry("b", "id-b", SealDisposition::Counted, 1),
        entry("a", "id-a", SealDisposition::Replaced, 0),
        entry("c", "id-c", SealDisposition::NotEligible, 0),
        entry("d", "id-d", SealDisposition::Discarded, 0),
        entry("e", "id-e", SealDisposition::Counted, 3),
    ]
}

struct Sealed {
    record: SealRecord,
    system_pk: StrandSignaturePk,
}

fn sealed_with(sk: StrandSignatureSk) -> Sealed {
    sealed_by(sk.clone(), sk)
}

/// A seal whose message has `sender_sk` as sender and `sk` as system key.
fn sealed_by(sender_sk: StrandSignatureSk, sk: StrandSignatureSk) -> Sealed {
    let built = build(manifest(sample_entries())).unwrap();
    let signing = SigningData::new(sender_sk, "", sk.clone());
    let message = Message::ballot_box_sealed_message(
        &built.manifest,
        built.hash,
        "Madrid PE",
        "Spain",
        &signing,
    )
    .unwrap();
    let message_bytes = message.strand_serialize().unwrap();
    let system_pk = StrandSignaturePk::from_sk(&sk).unwrap();
    let record = SealRecord::from_parts(
        &built.bytes,
        &message_bytes,
        &system_pk,
        SealRecordNames {
            election_event: "Overseas Voting".to_string(),
            election: "Madrid PE".to_string(),
            area: "Spain".to_string(),
        },
        Some(SealRecordCloseRequest {
            id: "request".to_string(),
            signing_code: Some("5D90-A3F7".to_string()),
            signers: vec![SealRecordSigner {
                name: "Signer".to_string(),
                certificate_sha256: "3B:9C".to_string(),
            }],
        }),
        48213,
    )
    .unwrap();
    Sealed { record, system_pk }
}

fn sealed() -> Sealed {
    sealed_with(StrandSignatureSk::generate().unwrap())
}

fn step_of(result: Result<SealCheck, SealCheckError>) -> SealCheckStep {
    result.expect_err("the record must be refused").step
}

#[test]
fn build_sorts_by_ballot_hash_then_id_and_keeps_duplicates() {
    let mut entries = sample_entries();
    entries.push(entry("b", "id-b", SealDisposition::Counted, 1));
    let built = build(manifest(entries)).unwrap();
    assert_eq!(built.manifest.entries.len(), 6);
    assert!(built.manifest.entries.windows(2).all(|pair| (
        pair[0].ballot_hash,
        &pair[0].ballot_id
    ) <= (
        pair[1].ballot_hash,
        &pair[1].ballot_id
    )));
    assert_eq!(built.bytes, built.manifest.to_bytes().unwrap());
    assert_eq!(built.hash, seal_hash(&built.bytes).unwrap());
    assert_eq!(built.manifest.ballots_in_box(), 6);
    assert_eq!(built.manifest.ballots_counted(), 3);
}

#[test]
fn the_sealed_message_carries_the_manifest_and_its_names() {
    let sealed = sealed();
    let check = verify_record(&sealed.record, None).unwrap();
    let head = &check.message.statement.head;
    assert_eq!(head.timestamp, 1_841_396_474);
    assert_eq!(
        head.description,
        "Ballot box of Madrid PE, Spain sealed: 2 of 5 ballots counted."
    );
    assert_eq!(check.message.election_id.as_deref(), Some("election"));
    assert_eq!(check.message.area_id.as_deref(), Some("area"));
    assert_eq!(
        check.system_key_fingerprint,
        fingerprint(&sealed.system_pk).unwrap()
    );
    assert_eq!(sealed.record.closed_at, "2028-05-08T11:01:12Z");
    assert_eq!(sealed.record.ballots.in_the_box, 5);
    assert_eq!(sealed.record.ballots.counted, 2);
}

#[test]
fn a_record_survives_its_json() {
    let sealed = sealed();
    let text = serde_json::to_string_pretty(&sealed.record).unwrap();
    let parsed: SealRecord = serde_json::from_str(&text).unwrap();
    assert!(verify_record(&parsed, None).is_ok());
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(value["entries"][0]["disposition"].is_string());
    assert_eq!(
        value["message"]["statement"]["body"]["BallotBoxSealed"][2],
        value["seal_hash"]
    );
}

#[test]
fn a_changed_entry_is_refused() {
    let mut record = sealed().record;
    record.entries[0].weight = 7;
    record.entries[0].disposition = SealDisposition::Counted;
    assert_eq!(
        step_of(verify_record(&record, None)),
        SealCheckStep::Manifest
    );
}

#[test]
fn a_removed_entry_is_refused() {
    let mut record = sealed().record;
    record.entries.remove(2);
    assert_eq!(
        step_of(verify_record(&record, None)),
        SealCheckStep::Manifest
    );
}

#[test]
fn unsorted_entries_are_refused() {
    let mut record = sealed().record;
    record.entries.swap(0, 1);
    assert_eq!(
        step_of(verify_record(&record, None)),
        SealCheckStep::Entries
    );
}

#[test]
fn a_changed_time_is_refused() {
    let mut record = sealed().record;
    record.sealed_at = "2028-05-08T11:01:15Z".to_string();
    assert_eq!(
        step_of(verify_record(&record, None)),
        SealCheckStep::Manifest
    );
}

#[test]
fn a_rebuilt_manifest_with_a_changed_time_fails_the_signed_hash() {
    // The attacker rewrites the manifest and its hash in the record but
    // cannot re-sign: the statement's hash no longer matches.
    let mut record = sealed().record;
    let mut changed = BallotBoxSealManifest::from_bytes(
        &general_purpose::STANDARD.decode(&record.manifest).unwrap(),
    )
    .unwrap();
    changed.sealed_at += 1;
    let built = build(changed).unwrap();
    record.sealed_at = format_time(built.manifest.sealed_at).unwrap();
    record.manifest = general_purpose::STANDARD.encode(&built.bytes);
    record.seal_hash = hex::encode(built.hash);
    assert_eq!(
        step_of(verify_record(&record, None)),
        SealCheckStep::SealHash
    );
}

#[test]
fn a_record_signed_by_another_key_fails_the_expected_key() {
    let sealed = sealed();
    let other = StrandSignaturePk::from_sk(&StrandSignatureSk::generate().unwrap()).unwrap();
    let expected = ExpectedKey::PublicKey(other.clone());
    assert_eq!(
        step_of(verify_record(&sealed.record, Some(&expected))),
        SealCheckStep::Key
    );
    let by_fingerprint = ExpectedKey::parse(&fingerprint(&other).unwrap()).unwrap();
    assert_eq!(
        step_of(verify_record(&sealed.record, Some(&by_fingerprint))),
        SealCheckStep::Key
    );
    let own = ExpectedKey::parse(&sealed.system_pk.to_der_b64_string().unwrap()).unwrap();
    assert!(verify_record(&sealed.record, Some(&own)).is_ok());
    let own_fingerprint = fingerprint(&sealed.system_pk)
        .unwrap()
        .to_lowercase()
        .replace(' ', ":");
    let own_fingerprint = ExpectedKey::parse(&own_fingerprint).unwrap();
    assert!(verify_record(&sealed.record, Some(&own_fingerprint)).is_ok());
}

#[test]
fn a_swapped_system_key_fails_the_system_signature() {
    let mut record = sealed().record;
    let other = StrandSignaturePk::from_sk(&StrandSignatureSk::generate().unwrap()).unwrap();
    record.system_public_key = other.to_der_b64_string().unwrap();
    assert_eq!(
        step_of(verify_record(&record, None)),
        SealCheckStep::SystemSignature
    );
}

#[test]
fn a_readable_message_that_differs_from_the_signed_bytes_is_refused() {
    let mut record = sealed().record;
    record.message["statement"]["head"]["description"] = serde_json::json!("changed");
    assert_eq!(
        step_of(verify_record(&record, None)),
        SealCheckStep::Message
    );
}

#[test]
fn a_changed_close_request_is_refused() {
    let mut record = sealed().record;
    record.close_request = None;
    assert_eq!(
        step_of(verify_record(&record, None)),
        SealCheckStep::Manifest
    );
}

#[test]
fn compare_stored_reports_missing_and_extra_as_multisets() {
    let built = build(manifest(vec![
        entry("a", "id-a", SealDisposition::Counted, 1),
        entry("a", "id-a", SealDisposition::Replaced, 0),
        entry("b", "id-b", SealDisposition::Counted, 1),
    ]))
    .unwrap();
    let key = |content: &str, id: &str| (ballot_hash(content).unwrap(), id.to_string());

    let same = [key("b", "id-b"), key("a", "id-a"), key("a", "id-a")];
    assert!(compare_stored(&built.manifest, &same).is_empty());

    let changed = [key("a", "id-a"), key("b", "id-b"), key("x", "id-x")];
    let diff = compare_stored(&built.manifest, &changed);
    assert_eq!(diff.missing, vec![key("a", "id-a")]);
    assert_eq!(diff.extra, vec![key("x", "id-x")]);
    assert_eq!(
        diff.describe(),
        "1 sealed ballot is missing (Ballot IDs id-a); 1 ballot is not in the seal (Ballot IDs id-x)"
    );
}

#[test]
fn a_fingerprint_is_128_bits_in_eight_uppercase_hex_groups() {
    let pk = StrandSignaturePk::from_sk(&StrandSignatureSk::generate().unwrap()).unwrap();
    let text = fingerprint(&pk).unwrap();
    let digest = strand::hash::hash_sha256(&pk.to_der().unwrap()).unwrap();
    let groups: Vec<&str> = text.split(' ').collect();
    assert_eq!(groups.len(), 8);
    assert!(groups.iter().all(|group| group.len() == 4));
    assert_eq!(groups.concat(), hex::encode_upper(&digest[..16]));
}

#[test]
fn a_64_bit_fingerprint_is_not_accepted_as_an_expected_key() {
    let pk = StrandSignaturePk::from_sk(&StrandSignatureSk::generate().unwrap()).unwrap();
    let short: String = fingerprint(&pk).unwrap().split(' ').take(4).collect();
    assert!(ExpectedKey::parse(&short).is_err());
}

#[test]
fn dispositions_have_fixed_tags_and_kebab_names() {
    let cases = [
        (SealDisposition::Counted, 0u8, "counted"),
        (SealDisposition::Replaced, 1, "replaced"),
        (SealDisposition::NotEligible, 2, "not-eligible"),
        (SealDisposition::Discarded, 3, "discarded"),
    ];
    for (disposition, tag, name) in cases {
        assert_eq!(borsh::to_vec(&disposition).unwrap(), vec![tag]);
        assert_eq!(disposition.to_string(), name);
        assert_eq!(
            serde_json::to_value(disposition).unwrap(),
            serde_json::json!(name)
        );
    }
}

#[test]
fn times_round_trip_to_the_second() {
    assert_eq!(format_time(0).unwrap(), "1970-01-01T00:00:00Z");
    assert_eq!(parse_time("2028-05-08T11:01:12Z").unwrap(), 1_841_396_472);
    // The same instant in another offset is not the record's form (R6e S2).
    assert!(parse_time("2028-05-08T13:01:12+02:00").is_err());
    assert!(parse_time("2028-05-08T11:01:12.5Z").is_err());
}

#[test]
fn the_other_seal_messages_name_the_box() {
    let sk = StrandSignatureSk::generate().unwrap();
    let signing = SigningData::new(sk.clone(), "", sk);
    let event = || crate::messages::newtypes::EventIdString("event".to_string());
    let failed = Message::ballot_box_seal_failed_message(
        event(),
        "election",
        "area",
        "Madrid PE",
        "Spain",
        "Ballot ID id-a does not match its ballot.".to_string(),
        &signing,
    )
    .unwrap();
    assert_eq!(
        failed.statement.head.description,
        "Ballot box of Madrid PE, Spain not sealed, it stays locked: Ballot ID id-a does not match its ballot."
    );
    assert_eq!(failed.area_id.as_deref(), Some("area"));
    let verified = Message::tally_ballot_box_verified_message(
        event(),
        "election",
        "area",
        "Madrid PE",
        "Spain",
        [1; 64],
        1340,
        "session".to_string(),
        &signing,
    )
    .unwrap();
    assert_eq!(
        verified.statement.head.description,
        "Ballot box of Madrid PE, Spain matches its seal: 1,340 ballots counted."
    );
    let rejected = Message::tally_ballot_box_rejected_message(
        event(),
        "election",
        "area",
        "Madrid PE",
        "Spain",
        "1 ballot is not in the seal".to_string(),
        "session".to_string(),
        &signing,
    )
    .unwrap();
    assert_eq!(
        rejected.statement.head.description,
        "The ballot box of Madrid PE, Spain does not match its seal: 1 ballot is not in the seal."
    );
    assert_eq!(rejected.election_id.as_deref(), Some("election"));
}

#[test]
fn a_swapped_area_or_election_without_re_encoding_fails_the_manifest() {
    let mut record = sealed().record;
    record.area.id = "other-area".to_string();
    assert_eq!(
        step_of(verify_record(&record, None)),
        SealCheckStep::Manifest
    );
    let mut record = sealed().record;
    record.election.id = "other-election".to_string();
    assert_eq!(
        step_of(verify_record(&record, None)),
        SealCheckStep::Manifest
    );
}

#[test]
fn a_swapped_area_or_election_with_a_re_encoded_manifest_fails_the_signed_hash() {
    for swap in [
        (|m: &mut BallotBoxSealManifest| m.area_id = "other-area".to_string()) as fn(&mut _),
        |m: &mut BallotBoxSealManifest| m.election_id = "other-election".to_string(),
    ] {
        let mut record = sealed().record;
        let mut changed = BallotBoxSealManifest::from_bytes(
            &general_purpose::STANDARD.decode(&record.manifest).unwrap(),
        )
        .unwrap();
        swap(&mut changed);
        let built = build(changed).unwrap();
        record.area.id = built.manifest.area_id.clone();
        record.election.id = built.manifest.election_id.clone();
        record.manifest = general_purpose::STANDARD.encode(&built.bytes);
        record.seal_hash = hex::encode(built.hash);
        // The row metadata is unsigned: an attacker rewrites it as well.
        record.message["election_id"] = serde_json::json!(built.manifest.election_id);
        record.message["area_id"] = serde_json::json!(built.manifest.area_id);
        let mut message = Message::strand_deserialize(
            &general_purpose::STANDARD
                .decode(&record.message_b64)
                .unwrap(),
        )
        .unwrap();
        message.election_id = Some(built.manifest.election_id.clone());
        message.area_id = Some(built.manifest.area_id.clone());
        record.message_b64 = general_purpose::STANDARD.encode(message.strand_serialize().unwrap());
        assert_eq!(
            step_of(verify_record(&record, None)),
            SealCheckStep::SealHash
        );
    }
}

#[test]
fn a_statement_naming_another_area_or_election_fails_the_statement() {
    let sk = StrandSignatureSk::generate().unwrap();
    let signing = SigningData::new(sk.clone(), "", sk);
    let built = build(manifest(sample_entries())).unwrap();
    for other in [
        BallotBoxSealManifest {
            area_id: "other-area".to_string(),
            ..built.manifest.clone()
        },
        BallotBoxSealManifest {
            election_id: "other-election".to_string(),
            ..built.manifest.clone()
        },
    ] {
        // Signed over the real hash, but naming another ballot box.
        let message =
            Message::ballot_box_sealed_message(&other, built.hash, "Madrid PE", "Spain", &signing)
                .unwrap();
        assert_eq!(
            check_statement(&message, &built.manifest, &built.bytes)
                .expect_err("another box")
                .step,
            SealCheckStep::Statement
        );
    }
}

#[test]
fn a_record_re_signed_under_another_key_fails_only_the_expected_key() {
    let genuine = sealed();
    let forged = sealed_with(StrandSignatureSk::generate().unwrap());
    let expected = ExpectedKey::PublicKey(genuine.system_pk.clone());
    assert_eq!(
        step_of(verify_record(&forged.record, Some(&expected))),
        SealCheckStep::Key
    );
    let by_fingerprint = ExpectedKey::parse(&fingerprint(&genuine.system_pk).unwrap()).unwrap();
    assert_eq!(
        step_of(verify_record(&forged.record, Some(&by_fingerprint))),
        SealCheckStep::Key
    );
    // Without an expected key it verifies; the printed fingerprint is how a
    // reader tells it apart.
    let check = verify_record(&forged.record, None).unwrap();
    assert_ne!(
        check.system_key_fingerprint,
        fingerprint(&genuine.system_pk).unwrap()
    );
    assert_eq!(
        check.system_key_fingerprint,
        fingerprint(&forged.system_pk).unwrap()
    );
}

#[test]
fn a_sender_that_is_not_the_system_key_is_refused() {
    let sender = StrandSignatureSk::generate().unwrap();
    let sealed = sealed_by(sender, StrandSignatureSk::generate().unwrap());
    let expected = ExpectedKey::PublicKey(sealed.system_pk.clone());
    assert_eq!(
        step_of(verify_record(&sealed.record, Some(&expected))),
        SealCheckStep::SenderSignature
    );
}

#[test]
fn only_the_canonical_utc_time_parses() {
    assert_eq!(parse_time("2028-05-08T11:01:14Z").unwrap(), 1_841_396_474);
    assert!(parse_time("2028-05-08T17:01:14+06:00").is_err());
    assert!(parse_time("2028-05-08T11:01:14+00:00").is_err());
    assert!(parse_time("2028-05-08T11:01:14.000Z").is_err());
}

#[test]
fn the_same_instant_in_another_offset_is_refused() {
    let mut record = sealed().record;
    record.sealed_at = "2028-05-08T17:01:14+06:00".to_string();
    assert_eq!(
        step_of(verify_record(&record, None)),
        SealCheckStep::Manifest
    );
}

/// A manifest breaking a rule, signed as is, fails `check_statement` (the
/// tally's check) and `verify_record` at `step`.
fn assert_rule(change: impl Fn(&mut BallotBoxSealManifest), step: SealCheckStep) {
    let mut changed = manifest(sample_entries());
    change(&mut changed);
    let built = build(changed).unwrap();
    let sk = StrandSignatureSk::generate().unwrap();
    let signing = SigningData::new(sk.clone(), "", sk.clone());
    let message =
        Message::ballot_box_sealed_message(&built.manifest, built.hash, "Post", "Area", &signing)
            .unwrap();
    assert_eq!(
        check_statement(&message, &built.manifest, &built.bytes)
            .expect_err("the manifest must be refused")
            .step,
        step
    );
    let record = SealRecord::from_parts(
        &built.bytes,
        &message.strand_serialize().unwrap(),
        &StrandSignaturePk::from_sk(&sk).unwrap(),
        SealRecordNames {
            election_event: "Event".to_string(),
            election: "Post".to_string(),
            area: "Area".to_string(),
        },
        None,
        1,
    )
    .unwrap();
    assert_eq!(step_of(verify_record(&record, None)), step);
}

#[test]
fn another_format_is_refused() {
    assert_rule(
        |manifest| manifest.format = "step/ballot-box-seal/v0".to_string(),
        SealCheckStep::Format,
    );
}

#[test]
fn times_out_of_order_are_refused() {
    assert_rule(
        |manifest| manifest.grace_deadline = manifest.closed_at - 1,
        SealCheckStep::Time,
    );
    assert_rule(
        |manifest| manifest.sealed_at = manifest.grace_deadline - 1,
        SealCheckStep::Time,
    );
}

#[test]
fn more_counted_ballots_than_eligible_voters_are_refused() {
    assert_rule(
        |manifest| manifest.eligible_voters = 1,
        SealCheckStep::Entries,
    );
}

#[test]
fn a_counted_entry_with_weight_zero_is_refused() {
    assert_rule(
        |manifest| {
            let counted = manifest
                .entries
                .iter_mut()
                .find(|entry| entry.disposition == SealDisposition::Counted)
                .unwrap();
            counted.weight = 0;
        },
        SealCheckStep::Entries,
    );
}

#[test]
fn an_entry_that_does_not_count_with_a_weight_is_refused() {
    assert_rule(
        |manifest| {
            let replaced = manifest
                .entries
                .iter_mut()
                .find(|entry| entry.disposition == SealDisposition::Replaced)
                .unwrap();
            replaced.weight = 1;
        },
        SealCheckStep::Entries,
    );
}

#[test]
fn a_valid_manifest_passes_the_rules() {
    let built = build(manifest(sample_entries())).unwrap();
    assert!(check_manifest(&built.manifest).is_ok());
}

/// Replaces the record's signed message, keeping its readable copy equal.
fn with_message(record: &mut SealRecord, message: &Message) {
    let bytes = message.strand_serialize().unwrap();
    record.message_b64 = general_purpose::STANDARD.encode(bytes);
    record.message = serde_json::to_value(message).unwrap();
}

#[test]
fn a_manifest_that_is_not_base64_is_refused() {
    let mut record = sealed().record;
    record.manifest = "not base64!".to_string();
    let error = verify_record(&record, None).unwrap_err();
    assert_eq!(error.step, SealCheckStep::Manifest);
    assert!(error.reason.starts_with("the manifest is not base64"));
}

#[test]
fn a_message_that_is_not_base64_is_refused() {
    let mut record = sealed().record;
    record.message_b64 = "not base64!".to_string();
    let error = verify_record(&record, None).unwrap_err();
    assert_eq!(error.step, SealCheckStep::Message);
    assert!(error.reason.starts_with("the message is not base64"));
}

#[test]
fn a_message_that_does_not_decode_is_refused() {
    let mut record = sealed().record;
    record.message_b64 = general_purpose::STANDARD.encode(b"not a message");
    let error = verify_record(&record, None).unwrap_err();
    assert_eq!(error.step, SealCheckStep::Message);
    assert!(error.reason.starts_with("the message does not decode"));
}

#[test]
fn a_system_key_that_does_not_decode_is_refused() {
    let mut record = sealed().record;
    record.system_public_key = "not a key".to_string();
    let error = verify_record(&record, None).unwrap_err();
    assert_eq!(error.step, SealCheckStep::SystemSignature);
    assert!(error.reason.starts_with("the system key does not decode"));
}

#[test]
fn a_sender_signature_over_other_bytes_is_refused() {
    let mut record = sealed().record;
    let bytes = general_purpose::STANDARD
        .decode(&record.message_b64)
        .unwrap();
    let mut message = Message::strand_deserialize(&bytes).unwrap();
    let forger = StrandSignatureSk::generate().unwrap();
    message.sender_signature = forger.sign(b"other bytes").unwrap();
    with_message(&mut record, &message);
    assert_eq!(
        step_of(verify_record(&record, None)),
        SealCheckStep::SenderSignature
    );
}

#[test]
fn a_check_error_names_its_step_and_reason() {
    let error = SealCheckError {
        step: SealCheckStep::SealHash,
        reason: "the hashes differ".to_string(),
    };
    assert_eq!(error.to_string(), "SealHash: the hashes differ");
}

#[test]
fn a_time_beyond_the_calendar_is_refused() {
    let error = format_time(i64::MAX as u64).unwrap_err();
    assert!(error.to_string().contains("is not a representable time"));
    assert!(format_time(u64::MAX).is_err());
}

#[test]
fn a_record_is_not_built_from_a_stored_message_that_does_not_decode() {
    let built = build(manifest(sample_entries())).unwrap();
    let system_pk = StrandSignaturePk::from_sk(&StrandSignatureSk::generate().unwrap()).unwrap();
    let names = SealRecordNames {
        election_event: "Overseas Voting".to_string(),
        election: "Madrid PE".to_string(),
        area: "Spain".to_string(),
    };
    let error = SealRecord::from_parts(
        &built.bytes,
        b"not a message",
        &system_pk,
        names.clone(),
        None,
        1,
    )
    .unwrap_err();
    assert!(error
        .to_string()
        .starts_with("the stored message does not decode"));
    assert!(SealRecord::from_parts(b"", b"", &system_pk, names, None, 1).is_err());
}

#[test]
fn an_entry_hash_must_be_64_bytes_of_hex() {
    let mut json = serde_json::to_value(entry("a", "id-a", SealDisposition::Counted, 1)).unwrap();
    json["ballot_hash"] = serde_json::json!("abcd");
    let error = serde_json::from_value::<SealEntry>(json.clone()).unwrap_err();
    assert!(error.to_string().contains("a ballot hash is 64 bytes"));
    json["ballot_hash"] = serde_json::json!("not hex");
    assert!(serde_json::from_value::<SealEntry>(json).is_err());
}
