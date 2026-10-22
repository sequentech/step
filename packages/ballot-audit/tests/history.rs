// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use ballot_audit::{digest, MultiplicityPolicy, Scope};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use electoral_log::messages::{
    message::{Message, SigningData},
    newtypes::*,
    statement::StatementBody,
};
use sequent_core::{
    ballot::{HashableBallot, SignedHashableBallot},
    encrypt::{encrypt_decoded_contest, hash_ballot, hash_ballot_sha512},
    fixtures::ballot_codec::{get_writein_ballot_style, get_writein_plaintext},
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use strand::{
    backend::ristretto::RistrettoCtx,
    serialization::{StrandDeserialize, StrandSerialize},
    signature::{StrandSignaturePk, StrandSignatureSk},
};

const PRIVATE_VOTER: &str = "private-historical-voter-identifier";

struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "ballot-history-test-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).unwrap();
        Self(dir)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Fixture {
    key: StrandSignatureSk,
    scope: Scope,
    rows: Vec<Value>,
    logs: Vec<Vec<u8>>,
}
impl Fixture {
    fn new() -> Self {
        let key = StrandSignatureSk::gen().unwrap();
        let scope = Scope {
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            election_id: "election".into(),
            area_id: "first-area".into(),
            tally_session_id: "tally".into(),
            tally_session_contest_id: "mapping".into(),
            board_name: "unused-history-board".into(),
            public_key_sha512: hex::encode([7; 64]),
            electoral_log_system_public_key: StrandSignaturePk::from_sk(&key)
                .unwrap()
                .to_der_b64_string()
                .unwrap(),
            contest_id: Some("not-used-for-full-ballot-hashing".into()),
            base_batch: 10,
            policy: MultiplicityPolicy::Unweighted,
        };
        let mut fixture = Self {
            key,
            scope,
            rows: vec![],
            logs: vec![],
        };
        fixture.add_cast(1, PRIVATE_VOTER, "first-area", "valid");
        fixture
    }
    fn add_cast(&mut self, id: u64, voter: &str, area: &str, status: &str) {
        let auditable = encrypt_decoded_contest::<RistrettoCtx>(
            &RistrettoCtx,
            &vec![get_writein_plaintext()],
            &get_writein_ballot_style(),
        )
        .unwrap();
        let signed = SignedHashableBallot::try_from(&auditable).unwrap();
        let ballot = HashableBallot::try_from(&signed).unwrap();
        let signing = SigningData::new(self.key.clone(), "synthetic", self.key.clone());
        let log = Message::cast_vote_with_channel_message(
            EventIdString(self.scope.election_event_id.clone()),
            ElectionIdString(Some(self.scope.election_id.clone())),
            PseudonymHash::new(
                strand::hash::hash_to_array(&voter.to_owned().strand_serialize().unwrap()).unwrap(),
            ),
            CastVoteHash::new(hash_ballot_sha512(&ballot).unwrap()),
            &signing,
            VoterIpString("".into()),
            VoterCountryString("".into()),
            VotingChannelString("ONLINE".into()),
            Some("untrusted-outer-user".into()),
            None,
            "unsigned-outer-area-intentionally-different".into(),
        )
        .unwrap()
        .strand_serialize()
        .unwrap();
        self.rows.push(json!({"id":format!("00000000-0000-4000-8000-{id:012x}"),"tenant_id":self.scope.tenant_id,"election_event_id":self.scope.election_event_id,"election_id":self.scope.election_id,"area_id":area,"status":status,"created_at":"2026-10-01T00:00:00Z","voter_id_string":voter,"content":format!(" {} \n",serde_json::to_string(&ballot).unwrap()),"ballot_id":hash_ballot(&ballot).unwrap(),"annotations":null}));
        self.logs.push(log);
    }
    fn resign_log(&mut self, index: usize, change: impl FnOnce(&mut Message)) {
        let mut log = Message::strand_deserialize(&self.logs[index]).unwrap();
        change(&mut log);
        self.logs[index] = Message::sign(
            log.statement,
            log.artifact,
            &self.key,
            "synthetic",
            &self.key,
            log.user_id,
            log.username,
            log.election_id,
            log.area_id,
            log.ballot_id,
        )
        .unwrap()
        .strand_serialize()
        .unwrap();
    }
    fn write_sources(&self, dir: &Path) -> Vec<(&'static str, String)> {
        [
            (
                "scope",
                serde_json::to_vec_pretty(
                    &json!({"format_version":1,"scope":self.scope,"batches":[10]}),
                )
                .unwrap(),
            ),
            (
                "cast-vote-history",
                serde_json::to_vec_pretty(&self.rows).unwrap(),
            ),
            (
                "electoral-log",
                serde_json::to_vec_pretty(
                    &self
                        .logs
                        .iter()
                        .map(|bytes| STANDARD_NO_PAD.encode(bytes))
                        .collect::<Vec<_>>(),
                )
                .unwrap(),
            ),
        ]
        .into_iter()
        .map(|(name, bytes)| {
            fs::write(dir.join(format!("{name}.json")), &bytes).unwrap();
            (name, digest(&bytes).unwrap())
        })
        .collect()
    }
    fn command(&self, dir: &Path) -> Command {
        command(dir, &self.write_sources(dir))
    }
}
fn command(dir: &Path, pins: &[(&str, String)]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_audit-cast-history"));
    for (name, pin) in pins {
        command
            .arg(format!("--{name}"))
            .arg(dir.join(format!("{name}.json")))
            .arg(format!("--{name}-sha512"))
            .arg(pin);
    }
    command
}
fn private(output: &Output) {
    for bytes in [&output.stdout, &output.stderr] {
        let text = String::from_utf8_lossy(bytes);
        assert!(!text.contains(PRIVATE_VOTER));
        assert!(!text.contains("untrusted-outer-user"));
        assert!(!text.contains("ciphertext"));
    }
}
fn report(output: &Output) -> Value {
    private(output);
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn complete_signed_history_matches_all_areas_statuses_and_revotes() {
    let dir = TempDir::new();
    let mut f = Fixture::new();
    f.add_cast(2, PRIVATE_VOTER, "other-area", "discarded");
    f.add_cast(3, "another-voter", "third-area", "in-progress");
    let output = f.command(&dir.0).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report = report(&output);
    assert_eq!(report["outcome"], "consistent");
    assert_eq!(report["checked_hasura_rows"], 3);
    assert_eq!(report["checked_signed_cast_vote_log_records"], 3);
    assert!(report["receipts"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["classification"] == "matched"));
}

#[test]
fn signed_receipt_without_hasura_row_is_a_contradiction() {
    let dir = TempDir::new();
    let mut f = Fixture::new();
    let receipt = f.rows[0]["ballot_id"].as_str().unwrap().to_owned();
    f.rows.clear();
    let output = f
        .command(&dir.0)
        .arg("--ballot-id")
        .arg(&receipt)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report = report(&output);
    assert_eq!(report["outcome"], "contradicted");
    assert_eq!(report["signed_log_records_without_hasura_row"], 1);
    assert_eq!(report["receipts"][0]["ballot_id"], receipt);
    assert_eq!(
        report["receipts"][0]["classification"],
        "signed_log_without_hasura_row"
    );
}

#[test]
fn row_without_success_log_is_incomplete_not_tampering() {
    let dir = TempDir::new();
    let mut f = Fixture::new();
    f.logs.clear();
    let output = f.command(&dir.0).output().unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report = report(&output);
    assert_eq!(report["hasura_rows_without_signed_log"], 1);
    assert_eq!(
        report["receipts"][0]["classification"],
        "hasura_row_without_signed_log"
    );
    assert!(report["assurance"]
        .as_str()
        .unwrap()
        .contains("best effort"));
}

#[test]
fn repeated_success_logs_are_not_assumed_to_be_distinct_cast_rows() {
    let dir = TempDir::new();
    let mut f = Fixture::new();
    f.logs.push(f.logs[0].clone());
    let output = f.command(&dir.0).output().unwrap();
    assert!(output.status.success());
    let report = report(&output);
    assert_eq!(report["checked_signed_cast_vote_log_records"], 2);
    assert_eq!(report["distinct_signed_log_message_bytes"], 1);
    assert_eq!(report["duplicate_log_message_records"], 1);
    assert_eq!(
        report["receipts"][0]["classification"],
        "matched_repeated_logs"
    );
}

#[test]
fn multiple_rows_for_one_signed_tuple_report_ambiguous_attribution() {
    let dir = TempDir::new();
    let mut f = Fixture::new();
    let mut row = f.rows[0].clone();
    row["id"] = "00000000-0000-4000-8000-000000000002".into();
    f.rows.push(row);
    let output = f.command(&dir.0).output().unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report = report(&output);
    assert_eq!(report["outcome"], "ambiguous");
    assert_eq!(report["ambiguous_row_attribution_groups"], 1);
    assert_eq!(
        report["receipts"][0]["cast_vote_ids"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn filter_never_hides_contradicted_other_history() {
    let dir = TempDir::new();
    let mut f = Fixture::new();
    let receipt = f.rows[0]["ballot_id"].as_str().unwrap().to_owned();
    f.add_cast(2, "another-voter", "second-area", "discarded");
    f.rows.pop();
    let output = f
        .command(&dir.0)
        .arg("--ballot-id")
        .arg(receipt)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report = report(&output);
    assert_eq!(report["outcome"], "contradicted");
    assert_eq!(report["receipts"].as_array().unwrap().len(), 1);
    assert_eq!(report["receipts"][0]["classification"], "matched");
    assert_eq!(report["checked_signed_cast_vote_log_records"], 2);
    assert_eq!(report["signed_log_records_without_hasura_row"], 1);
}

#[test]
fn wrong_signed_event_or_election_does_not_match() {
    for wrong_event in [true, false] {
        let dir = TempDir::new();
        let mut f = Fixture::new();
        f.resign_log(0, |log| {
            if wrong_event {
                log.statement.head.event.0 = "other-event".into();
            } else if let StatementBody::CastVoteWithChannel(e, _, _, _, _, _) =
                &mut log.statement.body
            {
                e.0 = Some("other-election".into());
            }
        });
        let output = f.command(&dir.0).output().unwrap();
        assert_eq!(output.status.code(), Some(3));
        let report = report(&output);
        assert_eq!(report["ignored_other_log_records"], 1);
        assert_eq!(report["hasura_rows_without_signed_log"], 1);
    }
}

#[test]
fn changing_either_signature_fails_without_private_output() {
    for sender in [true, false] {
        let dir = TempDir::new();
        let mut f = Fixture::new();
        let mut log = Message::strand_deserialize(&f.logs[0]).unwrap();
        let invalid = StrandSignatureSk::gen()
            .unwrap()
            .sign(&log.statement.strand_serialize().unwrap())
            .unwrap();
        if sender {
            log.sender_signature = invalid;
        } else {
            log.system_signature = invalid;
        }
        f.logs[0] = log.strand_serialize().unwrap();
        let output = f.command(&dir.0).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        private(&output);
        assert!(String::from_utf8_lossy(&output.stderr).contains("signature verification failed"));
    }
}

#[test]
fn mutated_hash_malformed_content_and_duplicate_row_id_fail_closed() {
    for variant in 0..4 {
        let dir = TempDir::new();
        let mut f = Fixture::new();
        match variant {
            0 => f.rows[0]["ballot_id"] = "00".repeat(32).into(),
            1 => f.rows[0]["content"] = PRIVATE_VOTER.into(),
            2 => f.rows.push(f.rows[0].clone()),
            _ => {
                let mut value: Value =
                    serde_json::from_str(f.rows[0]["content"].as_str().unwrap()).unwrap();
                value["issue_date"] = "tampered".into();
                f.rows[0]["content"] = serde_json::to_string(&value).unwrap().into();
            }
        }
        let output = f.command(&dir.0).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        private(&output);
    }
}

#[test]
fn lower_half_of_signed_full_hash_is_not_replaced_by_receipt_prefix_matching() {
    let dir = TempDir::new();
    let mut f = Fixture::new();
    f.resign_log(0, |log| {
        if let StatementBody::CastVoteWithChannel(_, _, hash, _, _, _) = &mut log.statement.body {
            let mut changed = hash.0.clone().into_inner();
            changed[63] ^= 1;
            *hash = CastVoteHash::new(changed);
        }
    });
    let output = f.command(&dir.0).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report = report(&output);
    assert_eq!(report["signed_log_records_without_hasura_row"], 1);
    assert_eq!(report["hasura_rows_without_signed_log"], 1);
}

#[test]
fn replaced_row_content_contradicts_the_signed_original() {
    let dir = TempDir::new();
    let mut f = Fixture::new();
    f.add_cast(2, "another-voter", "first-area", "valid");
    f.logs.pop();
    let forged = f.rows.pop().unwrap();
    f.rows[0]["content"] = forged["content"].clone();
    f.rows[0]["ballot_id"] = forged["ballot_id"].clone();
    let output = f.command(&dir.0).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report = report(&output);
    assert_eq!(report["outcome"], "contradicted");
    assert_eq!(report["signed_log_records_without_hasura_row"], 1);
    assert_eq!(report["hasura_rows_without_signed_log"], 1);
}

#[test]
fn requested_absent_receipt_is_incomplete_and_all_history_is_checked() {
    let dir = TempDir::new();
    let f = Fixture::new();
    let output = f
        .command(&dir.0)
        .arg("--ballot-id")
        .arg("00".repeat(32))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report = report(&output);
    assert_eq!(report["requested_receipt_found"], false);
    assert_eq!(report["checked_hasura_rows"], 1);
    assert_eq!(report["receipts"], json!([]));
}

#[test]
fn malformed_pins_sources_and_receipt_requests_are_invalid_input() {
    for variant in 0..4 {
        let dir = TempDir::new();
        let f = Fixture::new();
        let mut pins = f.write_sources(&dir.0);
        let mut command = match variant {
            0 => {
                pins[1].1 = pins[1].1.to_uppercase();
                command(&dir.0, &pins)
            }
            1 => {
                fs::write(dir.0.join("cast-vote-history.json"), b"{}").unwrap();
                pins[1].1 = digest(b"{}").unwrap();
                command(&dir.0, &pins)
            }
            2 => {
                fs::remove_file(dir.0.join("electoral-log.json")).unwrap();
                command(&dir.0, &pins)
            }
            _ => {
                let mut command = command(&dir.0, &pins);
                command.arg("--ballot-id").arg("not-a-receipt");
                command
            }
        };
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(4), "variant {variant}");
        private(&output);
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn source_digest_mismatch_fails_before_parsing_private_payload() {
    let dir = TempDir::new();
    let f = Fixture::new();
    let pins = f.write_sources(&dir.0);
    fs::write(dir.0.join("cast-vote-history.json"), PRIVATE_VOTER).unwrap();
    let output = command(&dir.0, &pins).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    private(&output);
    assert!(String::from_utf8_lossy(&output.stderr).contains("source digest mismatch"));
}

#[test]
fn tampered_signed_scope_cannot_hide_a_log_when_database_row_is_missing() {
    for field in 0..3 {
        let dir = TempDir::new();
        let mut f = Fixture::new();
        f.rows.clear();
        let mut log = Message::strand_deserialize(&f.logs[0]).unwrap();
        match field {
            0 => log.statement.head.event.0 = "other-event".into(),
            1 => {
                if let StatementBody::CastVoteWithChannel(e, _, _, _, _, _) =
                    &mut log.statement.body
                {
                    e.0 = Some("other-election".into());
                }
            }
            _ => {
                log.statement.head.kind = electoral_log::messages::statement::StatementType::Unknown
            }
        }
        f.logs[0] = log.strand_serialize().unwrap();
        let output = f.command(&dir.0).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        private(&output);
        assert!(String::from_utf8_lossy(&output.stderr).contains("signature verification failed"));
    }
}

#[test]
fn distinct_log_messages_for_one_row_require_occurrence_provenance() {
    for change_signed_statement in [true, false] {
        let dir = TempDir::new();
        let mut f = Fixture::new();
        f.logs.push(f.logs[0].clone());
        if change_signed_statement {
            f.resign_log(1, |log| log.statement.head.timestamp += 1);
        } else {
            let mut log = Message::strand_deserialize(&f.logs[1]).unwrap();
            log.user_id = Some("different-unsigned-outer-metadata".into());
            f.logs[1] = log.strand_serialize().unwrap();
        }
        let output = f.command(&dir.0).output().unwrap();
        assert_eq!(output.status.code(), Some(3));
        let report = report(&output);
        assert_eq!(report["outcome"], "ambiguous");
        assert_eq!(report["ambiguous_log_occurrence_groups"], 1);
        assert_eq!(
            report["receipts"][0]["distinct_signed_log_message_bytes"],
            2
        );
        assert_eq!(
            report["receipts"][0]["distinct_signed_statements"],
            if change_signed_statement { 2 } else { 1 }
        );
    }
}

#[test]
fn current_multi_contest_history_uses_native_full_hash() {
    use sequent_core::{
        encrypt::{encrypt_decoded_multi_contest, hash_multi_ballot, hash_multi_ballot_sha512},
        multi_ballot::{HashableMultiBallot, SignedHashableMultiBallot},
    };
    let dir = TempDir::new();
    let mut f = Fixture::new();
    let auditable = encrypt_decoded_multi_contest::<RistrettoCtx>(
        &RistrettoCtx,
        &vec![get_writein_plaintext()],
        &get_writein_ballot_style(),
    )
    .unwrap();
    let signed = SignedHashableMultiBallot::try_from(&auditable).unwrap();
    let ballot = HashableMultiBallot::try_from(&signed).unwrap();
    f.scope.contest_id = None;
    f.rows[0]["content"] = serde_json::to_string(&ballot).unwrap().into();
    f.rows[0]["ballot_id"] = hash_multi_ballot(&ballot).unwrap().into();
    let hash = hash_multi_ballot_sha512(&ballot).unwrap();
    f.resign_log(0, |log| {
        if let StatementBody::CastVoteWithChannel(_, _, h, _, _, _) = &mut log.statement.body {
            *h = CastVoteHash::new(hash);
        }
    });
    let output = f.command(&dir.0).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(report(&output)["outcome"], "consistent");
}

#[test]
fn validly_signed_cast_vote_body_under_other_kind_is_invalid_evidence() {
    let dir = TempDir::new();
    let mut f = Fixture::new();
    f.rows.clear();
    f.resign_log(0, |log| {
        log.statement.head.kind = electoral_log::messages::statement::StatementType::Unknown
    });
    let output = f.command(&dir.0).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    private(&output);
    assert!(String::from_utf8_lossy(&output.stderr).contains("kind/body mismatch"));
}

#[test]
#[ignore = "writes synthetic complete historical sources only when explicitly requested"]
fn export_synthetic_historical_inputs() {
    let dir = PathBuf::from(
        std::env::var("BALLOT_AUDIT_HISTORY_FIXTURE_DIR")
            .expect("set BALLOT_AUDIT_HISTORY_FIXTURE_DIR"),
    );
    fs::create_dir(&dir).unwrap();
    let mut f = Fixture::new();
    f.add_cast(2, PRIVATE_VOTER, "second-area", "discarded");
    let receipt = f.rows[0]["ballot_id"].as_str().unwrap().to_owned();
    for (name, expected_exit) in [("valid", 0), ("missing-row", 1), ("invalid-signature", 1)] {
        if name == "missing-row" {
            f.rows.remove(0);
        }
        if name == "invalid-signature" {
            f.rows.clear();
            let mut log = Message::strand_deserialize(&f.logs[0]).unwrap();
            log.statement.head.event.0 = "tampered-event-with-missing-row".into();
            f.logs[0] = log.strand_serialize().unwrap();
        }
        let case_dir = dir.join(name);
        fs::create_dir(&case_dir).unwrap();
        let pins = f.write_sources(&case_dir);
        fs::write(
            case_dir.join("source-anchors.json"),
            serde_json::to_vec_pretty(
                &pins
                    .into_iter()
                    .collect::<std::collections::BTreeMap<_, _>>(),
            )
            .unwrap(),
        )
        .unwrap();
        fs::write(
            case_dir.join("expected.json"),
            serde_json::to_vec_pretty(
                &json!({"exit":expected_exit,"requested_receipt":receipt,"synthetic":true}),
            )
            .unwrap(),
        )
        .unwrap();
    }
}
