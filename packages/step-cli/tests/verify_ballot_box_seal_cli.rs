// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Spawn the shipped CLI on seal records built with electoral-log's seal
//! module: an auditor relies on the report and on a nonzero exit status for
//! every failure.

use base64::Engine as _;
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::seal::{
    ballot_hash, build, fingerprint, BallotBoxSealManifest, SealDisposition, SealEntry, SealRecord,
    SealRecordCloseRequest, SealRecordNames, SealRecordSigner, DEFAULT_SEAL_CHANNEL,
    SEAL_FORMAT_V1,
};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use strand::serialization::StrandSerialize;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};

const ELECTION: &str = "election-1";
const AREA: &str = "area-1";
const CSV_HEADER: &str = "created,election_id,area_id,hash_voter_id,ballot_id,voting_channel";

/// A stored Ballot ID: 64 hex digits.
fn ballot_id(digit: char) -> String {
    std::iter::repeat(digit).take(64).collect()
}

fn entry(digit: char, disposition: SealDisposition, weight: u64) -> SealEntry {
    SealEntry {
        ballot_hash: ballot_hash(&format!("content-{digit}")).unwrap(),
        ballot_id: ballot_id(digit),
        disposition,
        weight,
        channel: DEFAULT_SEAL_CHANNEL.to_string(),
    }
}

struct Sealed {
    record: SealRecord,
    system_pk: StrandSignaturePk,
}

fn sample_entries() -> Vec<SealEntry> {
    vec![
        entry('a', SealDisposition::Counted, 1),
        entry('b', SealDisposition::Replaced, 0),
        entry('c', SealDisposition::Counted, 3),
        entry('d', SealDisposition::Discarded, 0),
    ]
}

fn manifest(entries: Vec<SealEntry>) -> BallotBoxSealManifest {
    BallotBoxSealManifest {
        format: SEAL_FORMAT_V1.to_string(),
        tenant_id: "tenant".to_string(),
        election_event_id: "event".to_string(),
        election_id: ELECTION.to_string(),
        area_id: AREA.to_string(),
        closed_at: 1_841_396_472,
        grace_deadline: 1_841_396_472,
        sealed_at: 1_841_396_474,
        close_request_id: Some("b821a9c1-0000-0000-0000-000000000000".to_string()),
        eligible_voters: 4,
        entries,
    }
}

fn sealed() -> Sealed {
    let sk = StrandSignatureSk::generate().unwrap();
    sealed_by(sk.clone(), sk, sample_entries())
}

/// A seal of `entries` whose message has `sender_sk` as sender and `sk` as
/// system key.
fn sealed_by(
    sender_sk: StrandSignatureSk,
    sk: StrandSignatureSk,
    entries: Vec<SealEntry>,
) -> Sealed {
    sealed_manifest(sender_sk, sk, manifest(entries))
}

/// A seal of `manifest`, signed as is.
fn sealed_manifest(
    sender_sk: StrandSignatureSk,
    sk: StrandSignatureSk,
    manifest: BallotBoxSealManifest,
) -> Sealed {
    let built = build(manifest).unwrap();
    let signing = SigningData::new(sender_sk, "", sk.clone());
    let message = Message::ballot_box_sealed_message(
        &built.manifest,
        built.hash,
        "Post A",
        "Area 1",
        &signing,
    )
    .unwrap();
    let system_pk = StrandSignaturePk::from_sk(&sk).unwrap();
    let record = SealRecord::from_parts(
        &built.bytes,
        &message.strand_serialize().unwrap(),
        &system_pk,
        SealRecordNames {
            election_event: "General Election".to_string(),
            election: "Post A".to_string(),
            area: "Area 1".to_string(),
        },
        Some(SealRecordCloseRequest {
            id: "b821a9c1-0000-0000-0000-000000000000".to_string(),
            signing_code: Some("5D90-A3F7".to_string()),
            signers: vec![
                SealRecordSigner {
                    name: "Signer 1".to_string(),
                    certificate_sha256: "3B:9C".to_string(),
                },
                SealRecordSigner {
                    name: "Signer 2".to_string(),
                    certificate_sha256: "4D:1E".to_string(),
                },
            ],
        }),
        48213,
    )
    .unwrap();
    Sealed { record, system_pk }
}

fn write_record(directory: &Path, record: &SealRecord) -> PathBuf {
    let path = directory.join("seal.json");
    std::fs::write(&path, serde_json::to_string_pretty(record).unwrap()).unwrap();
    path
}

fn write_csv(directory: &Path, rows: &[(&str, &str, String)]) -> PathBuf {
    let path = directory.join("cast-votes.csv");
    let mut text = format!("{CSV_HEADER}\n");
    for (election, area, id) in rows {
        text.push_str(&format!("1841396000,{election},{area},ff,{id},ONLINE\n"));
    }
    std::fs::write(&path, text).unwrap();
    path
}

fn verify(record: &Path, args: &[&str]) -> (Output, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_step-cli"))
        .env("NO_COLOR", "1")
        .args(["step", "verify-ballot-box-seal"])
        .arg(record)
        .args(args)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    (output, stdout)
}

#[test]
fn verify_seal_valid_record_reports_every_check_and_exits_zero() {
    let directory = tempfile::tempdir().unwrap();
    let sealed = sealed();
    let path = write_record(directory.path(), &sealed.record);
    let key = fingerprint(&sealed.system_pk).unwrap();

    let (output, stdout) = verify(&path, &[]);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("Ballot box    Post A, Area 1 (names as in the signed statement)"));
    assert!(stdout.contains("Event         General Election (name not signed), ID event"));
    assert!(
        stdout.contains("✓ Names              Post A, Area 1: the names in the signed statement")
    );
    assert!(stdout.contains(&format!(
        "\n{}{}\n",
        " ".repeat(21),
        sealed.record.seal_hash
    )));
    assert!(stdout.contains("- Log entry          #48213 (not signed)"));
    assert!(stdout.contains("closed, grace deadline and seal in order"));
    assert!(stdout.contains("Sealed at     2028-05-08 11:01:14 UTC"));
    assert!(stdout.contains("4 in the box, 2 counted"));
    assert!(stdout.contains("✓ Manifest"));
    assert!(stdout.contains(&format!("signed by the record's key {key}")));
    assert!(!stdout.contains("election event key"));
    assert!(stdout.contains(&format!(
        "! Expected key       not checked (no --expected-key): compare {key}"
    )));
    assert!(stdout.contains("✓ Sender signature   a system entry"));
    assert!(stdout.contains("the signed statement says 2028-05-08T11:01:14Z"));
    assert!(stdout.contains("b821a9c1…, signing code 5D90-A3F7, 2 certificate signatures"));
    assert!(stdout.trim_end().ends_with(&format!(
        "Seal valid for key {key} (not checked against a key you trust: pass --expected-key)"
    )));

    let (output, stdout) = verify(&path, &["--expected-key", &key]);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains(&format!("election event key {key}")));
    assert!(stdout.trim_end().ends_with("\nSeal valid"), "{stdout}");
    assert!(stdout.contains(&format!("✓ Expected key       {key} is the key you gave")));
}

fn assert_refused(record: &SealRecord, args: &[&str], failed_line: &str) {
    let directory = tempfile::tempdir().unwrap();
    let path = write_record(directory.path(), record);
    let (output, stdout) = verify(&path, args);
    assert!(!output.status.success(), "{stdout}");
    assert!(stdout.contains(failed_line), "{stdout}");
    assert!(stdout.trim_end().ends_with("Seal NOT valid"), "{stdout}");
    assert!(String::from_utf8_lossy(&output.stderr).contains("the seal is not valid"));
}

#[test]
fn verify_seal_changed_entry_fails_the_manifest() {
    let mut record = sealed().record;
    let counted = record
        .entries
        .iter_mut()
        .find(|entry| entry.disposition == SealDisposition::Counted)
        .unwrap();
    counted.weight += 1;
    assert_refused(&record, &[], "✗ Manifest");
}

#[test]
fn verify_seal_removed_entry_fails_the_manifest() {
    let mut record = sealed().record;
    record.entries.remove(1);
    assert_refused(&record, &[], "✗ Manifest");
}

#[test]
fn verify_seal_changed_time_fails_the_manifest() {
    let mut record = sealed().record;
    record.sealed_at = "2028-05-08T11:01:15Z".to_string();
    assert_refused(&record, &[], "✗ Manifest");
}

#[test]
fn verify_seal_another_key_fails_the_expected_key() {
    let record = sealed().record;
    let other = StrandSignaturePk::from_sk(&StrandSignatureSk::generate().unwrap()).unwrap();
    assert_refused(
        &record,
        &["--expected-key", &fingerprint(&other).unwrap()],
        "✗ Expected key",
    );
    assert_refused(
        &record,
        &["--expected-key", &other.to_der_b64_string().unwrap()],
        "✗ Expected key",
    );
}

#[test]
fn verify_seal_ballot_id_hit_reports_how_it_counts() {
    let directory = tempfile::tempdir().unwrap();
    let path = write_record(directory.path(), &sealed().record);
    let (output, stdout) = verify(&path, &["--ballot-id", &ballot_id('C')]);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains(&format!(
        "✓ Ballot ID {} in the ballot box: counted (weight 3)",
        ballot_id('C')
    )));
    let (output, stdout) = verify(&path, &["--ballot-id", &ballot_id('b')]);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("in the ballot box: replaced by a later ballot, not counted"));
}

#[test]
fn verify_seal_ballot_id_miss_fails() {
    let directory = tempfile::tempdir().unwrap();
    let path = write_record(directory.path(), &sealed().record);
    let (output, stdout) = verify(&path, &["--ballot-id", &ballot_id('e')]);
    assert!(!output.status.success(), "{stdout}");
    assert!(stdout.contains(&format!(
        "✗ Ballot ID {} not in the ballot box",
        ballot_id('e')
    )));
    assert!(stdout.contains(", but a requested check failed"));
}

#[test]
fn verify_seal_cast_votes_match_ignores_other_boxes() {
    let directory = tempfile::tempdir().unwrap();
    let path = write_record(directory.path(), &sealed().record);
    let csv = write_csv(
        directory.path(),
        &[
            (ELECTION, AREA, ballot_id('a')),
            (ELECTION, AREA, ballot_id('b')),
            (ELECTION, AREA, ballot_id('c')),
            (ELECTION, AREA, ballot_id('d')),
            (ELECTION, "area-2", ballot_id('e')),
            ("election-2", AREA, ballot_id('f')),
        ],
    );
    let (output, stdout) = verify(&path, &["--cast-votes", csv.to_str().unwrap()]);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains(
        "✓ CastVote entries   4 in cast-votes.csv; sealed ballots without an entry: 0; \
         entries without a sealed ballot: 0"
    ));
    assert!(stdout.contains("Seal valid for key"));
    assert!(!stdout.contains("requested check failed"));
}

#[test]
fn verify_seal_cast_votes_mismatch_lists_both_sides_and_fails() {
    let directory = tempfile::tempdir().unwrap();
    let path = write_record(directory.path(), &sealed().record);
    let csv = write_csv(
        directory.path(),
        &[
            (ELECTION, AREA, ballot_id('a')),
            (ELECTION, AREA, ballot_id('b')),
            (ELECTION, AREA, ballot_id('c')),
            (ELECTION, AREA, ballot_id('9')),
        ],
    );
    let (output, stdout) = verify(&path, &["--cast-votes", csv.to_str().unwrap()]);
    assert!(!output.status.success(), "{stdout}");
    assert!(stdout.contains(
        "✗ CastVote entries   4 in cast-votes.csv; sealed ballots without an entry: 1; \
         entries without a sealed ballot: 1"
    ));
    assert!(stdout.contains(&format!("without an entry: Ballot IDs {}", ballot_id('d'))));
    assert!(stdout.contains(&format!(
        "without a sealed ballot: Ballot IDs {}",
        ballot_id('9')
    )));
}

#[test]
fn verify_seal_re_signed_under_another_key_without_expected_key_names_the_unchecked_key() {
    // A forger seals altered entries with their own key and puts it in the
    // record: only a trusted key can tell, so the verdict must say so.
    let forger = StrandSignatureSk::generate().unwrap();
    let mut entries = sample_entries();
    entries.push(entry('e', SealDisposition::Counted, 1));
    let sealed = sealed_by(forger.clone(), forger, entries);
    let key = fingerprint(&sealed.system_pk).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = write_record(directory.path(), &sealed.record);
    let (output, stdout) = verify(&path, &[]);
    assert!(output.status.success(), "{stdout}");
    assert!(!stdout.contains("election event key"), "{stdout}");
    assert!(stdout.trim_end().ends_with(&format!(
        "Seal valid for key {key} (not checked against a key you trust: pass --expected-key)"
    )));
}

#[test]
fn verify_seal_sender_that_is_not_the_system_key_fails() {
    let sealed = sealed_by(
        StrandSignatureSk::generate().unwrap(),
        StrandSignatureSk::generate().unwrap(),
        sample_entries(),
    );
    assert_refused(&sealed.record, &[], "✗ Sender signature");
}

#[test]
fn verify_seal_rebuilt_manifest_fails_the_seal_hash() {
    // A consistent edit: the entry, the manifest bytes and the seal hash all
    // change, but the signed statement still holds the old hash.
    let mut record = sealed().record;
    let mut entries = record.entries.clone();
    entries
        .iter_mut()
        .find(|entry| entry.disposition == SealDisposition::Counted)
        .unwrap()
        .weight += 1;
    let built = build(manifest(entries)).unwrap();
    record.entries = built.manifest.entries.clone();
    record.manifest = base64::engine::general_purpose::STANDARD.encode(&built.bytes);
    record.seal_hash = hex::encode(built.hash);
    assert_refused(&record, &[], "✗ Seal hash");
}

#[test]
fn verify_seal_unreadable_cast_votes_fails_with_a_verdict() {
    let directory = tempfile::tempdir().unwrap();
    let path = write_record(directory.path(), &sealed().record);
    let missing = directory.path().join("missing.csv");
    let (output, stdout) = verify(&path, &["--cast-votes", missing.to_str().unwrap()]);
    assert!(!output.status.success(), "{stdout}");
    assert!(
        stdout.contains("✗ CastVote entries   cannot read"),
        "{stdout}"
    );
    assert!(
        stdout.contains(", but a requested check failed"),
        "{stdout}"
    );
}

#[test]
fn verify_seal_another_boxs_record_renamed_fails_the_names() {
    let mut record = sealed().record;
    record.area.name = "Area 2".to_string();
    assert_refused(
        &record,
        &[],
        "✗ Names              the record says Post A, Area 2",
    );
}

#[test]
fn verify_seal_expected_box_ids_hit_and_miss() {
    let directory = tempfile::tempdir().unwrap();
    let path = write_record(directory.path(), &sealed().record);
    let (output, stdout) = verify(&path, &["--election-id", ELECTION, "--area-id", AREA]);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains(&format!(
        "✓ Expected box       election {ELECTION}, area {AREA}: the one you gave"
    )));
    assert_refused(
        &sealed().record,
        &["--election-id", ELECTION, "--area-id", "area-2"],
        "✗ Expected box       the seal is for election election-1, area area-1, not the expected area area-2",
    );
}

#[test]
fn verify_seal_time_in_another_offset_fails_and_prints_no_unverified_time() {
    let mut record = sealed().record;
    record.sealed_at = "2028-05-08T17:01:14+06:00".to_string();
    let directory = tempfile::tempdir().unwrap();
    let path = write_record(directory.path(), &record);
    let (output, stdout) = verify(&path, &[]);
    assert!(!output.status.success(), "{stdout}");
    assert!(stdout.contains("✗ Manifest"), "{stdout}");
    assert!(stdout.contains("(not verified)"), "{stdout}");
    assert!(!stdout.contains("Sealed at"), "{stdout}");
    assert!(stdout.trim_end().ends_with("Seal NOT valid"));
}

#[test]
fn verify_seal_manifest_rules_fail_even_when_signed() {
    let sk = StrandSignatureSk::generate().unwrap();
    let mut over_census = manifest(sample_entries());
    over_census.eligible_voters = 1;
    let sealed = sealed_manifest(sk.clone(), sk.clone(), over_census);
    assert_refused(
        &sealed.record,
        &[],
        "✗ Entries            2 ballots counted but only 1 eligible voters",
    );

    let mut out_of_order = manifest(sample_entries());
    out_of_order.sealed_at = out_of_order.grace_deadline - 1;
    let sealed = sealed_manifest(sk.clone(), sk, out_of_order);
    assert_refused(&sealed.record, &[], "✗ Time");
}

#[test]
fn verify_seal_64_bit_fingerprint_is_refused_as_expected_key() {
    let directory = tempfile::tempdir().unwrap();
    let sealed = sealed();
    let path = write_record(directory.path(), &sealed.record);
    let short: String = fingerprint(&sealed.system_pk)
        .unwrap()
        .split(' ')
        .take(4)
        .collect::<Vec<_>>()
        .join(" ");
    let (output, _) = verify(&path, &["--expected-key", &short]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--expected-key"));
}
