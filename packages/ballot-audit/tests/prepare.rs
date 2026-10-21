// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use b3::messages::{
    artifact::{Ballots, Configuration},
    message::Message,
    newtypes::{PublicKeyHash, MAX_TRUSTEES, NULL_TRUSTEE},
    protocol_manager::ProtocolManager,
};
use ballot_audit::{digest, Manifest, MultiplicityPolicy, Scope};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use electoral_log::messages::{
    message::{Message as ElectoralMessage, SigningData},
    newtypes::{
        CastVoteHash, ElectionIdString, EventIdString, PseudonymHash, VoterCountryString,
        VoterIpString, VotingChannelString,
    },
};
use sequent_core::{
    ballot::{HashableBallot, SignedHashableBallot},
    encrypt::{encrypt_decoded_contest, hash_ballot, hash_ballot_sha512},
    fixtures::ballot_codec::{get_writein_ballot_style, get_writein_plaintext},
};
use serde_json::{json, Value};
use std::{
    fs,
    marker::PhantomData,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};
use strand::{
    backend::ristretto::RistrettoCtx,
    serialization::{StrandDeserialize, StrandSerialize},
    signature::{StrandSignaturePk, StrandSignatureSk},
};

const VOTER: &str = "synthetic-private-voter-identifier";

struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "ballot-audit-prepare-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Fixture {
    scope: Value,
    rows: Value,
    voters: Value,
    board: Vec<Vec<u8>>,
    logs: Vec<Vec<u8>>,
}
impl Fixture {
    fn new(policy: MultiplicityPolicy, multiplicity: u64) -> Self {
        let ballot = encrypt_decoded_contest::<RistrettoCtx>(
            &RistrettoCtx,
            &vec![get_writein_plaintext()],
            &get_writein_ballot_style(),
        )
        .unwrap();
        let signed = SignedHashableBallot::try_from(&ballot).unwrap();
        let ballot = HashableBallot::try_from(&signed).unwrap();
        let contests = ballot.deserialize_contests::<RistrettoCtx>().unwrap();
        let pm = ProtocolManager::<RistrettoCtx>::new(StrandSignatureSk::gen().unwrap());
        let system_pk = StrandSignaturePk::from_sk(&pm.signing_key).unwrap();
        let configuration = Configuration::<RistrettoCtx>::new(
            1,
            system_pk.clone(),
            vec![
                StrandSignaturePk::from_sk(&StrandSignatureSk::gen().unwrap()).unwrap(),
                StrandSignaturePk::from_sk(&StrandSignatureSk::gen().unwrap()).unwrap(),
            ],
            2,
            PhantomData,
        );
        let scope = Scope {
            tenant_id: "00000000-0000-4000-8000-000000000010".into(),
            election_event_id: "00000000-0000-4000-8000-000000000020".into(),
            election_id: "00000000-0000-4000-8000-000000000030".into(),
            area_id: "00000000-0000-4000-8000-000000000040".into(),
            tally_session_id: "00000000-0000-4000-8000-000000000050".into(),
            tally_session_contest_id: "00000000-0000-4000-8000-000000000060".into(),
            board_name: "synthetic-board".into(),
            public_key_sha512: hex::encode([7; 64]),
            electoral_log_system_public_key: system_pk.to_der_b64_string().unwrap(),
            contest_id: Some(contests[0].contest_id.clone()),
            base_batch: 10,
            policy,
        };
        let batches: Vec<u32> = match policy {
            MultiplicityPolicy::Weighted => (0..17)
                .filter(|bit| multiplicity & (1 << bit) != 0)
                .map(|bit| 10 + bit)
                .collect(),
            _ => vec![10],
        };
        let count = match policy {
            MultiplicityPolicy::Delegated => multiplicity as usize,
            _ => 1,
        };
        let mut trustees = [NULL_TRUSTEE; MAX_TRUSTEES];
        trustees[0] = 1;
        trustees[1] = 2;
        let mut board = vec![Message::bootstrap_msg(&configuration, &pm)
            .unwrap()
            .strand_serialize()
            .unwrap()];
        for batch in &batches {
            board.push(
                Message::ballots_msg(
                    &configuration,
                    *batch as usize,
                    &Ballots::new(vec![contests[0].ciphertext.clone(); count]),
                    trustees,
                    PublicKeyHash([7; 64]),
                    &pm,
                )
                .unwrap()
                .strand_serialize()
                .unwrap(),
            );
        }
        // A full board includes other scopes and statement-only messages.
        board.push(
            Message::ballots_msg(
                &configuration,
                100,
                &Ballots::new(vec![]),
                trustees,
                PublicKeyHash([7; 64]),
                &pm,
            )
            .unwrap()
            .strand_serialize()
            .unwrap(),
        );
        board.push(
            Message::configuration_msg(&configuration, &pm)
                .unwrap()
                .strand_serialize()
                .unwrap(),
        );
        let signing = SigningData::new(pm.signing_key.clone(), "synthetic", pm.signing_key.clone());
        let log = ElectoralMessage::cast_vote_with_channel_message(
            EventIdString(scope.election_event_id.clone()),
            ElectionIdString(Some(scope.election_id.clone())),
            PseudonymHash::new(
                strand::hash::hash_to_array(&VOTER.to_owned().strand_serialize().unwrap()).unwrap(),
            ),
            CastVoteHash::new(hash_ballot_sha512(&ballot).unwrap()),
            &signing,
            VoterIpString("".into()),
            VoterCountryString("".into()),
            VotingChannelString("ONLINE".into()),
            Some("unsigned-metadata-is-not-used-for-matching".into()),
            None,
            scope.area_id.clone(),
        )
        .unwrap()
        .strand_serialize()
        .unwrap();
        let latest = json!({
            "id":"00000000-0000-4000-8000-000000000002", "tenant_id":scope.tenant_id,
            "election_event_id":scope.election_event_id, "election_id":scope.election_id,
            "area_id":scope.area_id, "created_at":"2026-10-01T00:00:00Z",
            "voter_id_string":VOTER, "content":format!(" {} \n", serde_json::to_string(&ballot).unwrap()),
            "ballot_id":hash_ballot(&ballot).unwrap(), "status":"valid",
            "annotations":{"voting_channel":"ONLINE"}, "last_updated_at":null,
        });
        let mut older = latest.clone();
        older["id"] = "00000000-0000-4000-8000-000000000001".into();
        older["created_at"] = "2026-09-30T00:00:00Z".into();
        older["content"] = "older content is not selected".into();
        let mut discarded = latest.clone();
        discarded["id"] = "00000000-0000-4000-8000-000000000003".into();
        discarded["created_at"] = "2026-10-02T00:00:00Z".into();
        discarded["status"] = "discarded".into();
        discarded["content"] = "discarded content is never consumed".into();
        let mut outside = latest.clone();
        outside["id"] = "00000000-0000-4000-8000-000000000004".into();
        outside["area_id"] = "00000000-0000-4000-8000-000000000099".into();
        outside["status"] = "in-progress".into();
        Self {
            scope: json!({"format_version":1,"scope":scope,"batches":batches}),
            rows: json!([older, latest, discarded, outside]),
            voters: json!([{"voter_id":VOTER,"multiplicity":multiplicity}]),
            board,
            logs: vec![log],
        }
    }

    fn write_sources(&self, dir: &Path) -> Vec<(&'static str, String)> {
        let scope = serde_json::to_vec_pretty(&self.scope).unwrap();
        let scope_digest = digest(&scope).unwrap();
        let eligibility = serde_json::to_vec_pretty(
            &json!({"format_version":1,"scope_sha512":scope_digest,"voters":self.voters}),
        )
        .unwrap();
        let encode = |values: &[Vec<u8>]| {
            serde_json::to_vec_pretty(
                &values
                    .iter()
                    .map(|bytes| STANDARD_NO_PAD.encode(bytes))
                    .collect::<Vec<_>>(),
            )
            .unwrap()
        };
        [
            ("scope", scope),
            ("cast-votes", serde_json::to_vec_pretty(&self.rows).unwrap()),
            ("eligible-voters", eligibility),
            ("board-messages", encode(&self.board)),
            ("electoral-log", encode(&self.logs)),
        ]
        .into_iter()
        .map(|(name, bytes)| {
            fs::write(dir.join(format!("{name}.json")), &bytes).unwrap();
            (name, digest(&bytes).unwrap())
        })
        .collect()
    }

    fn command(&self, dir: &Path) -> Command {
        command_with_pins(dir, &self.write_sources(dir))
    }
}

fn command_with_pins(dir: &Path, sources: &[(&str, String)]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_prepare-ballot-audit"));
    for (name, pin) in sources {
        command
            .arg(format!("--{name}"))
            .arg(dir.join(format!("{name}.json")));
        command.arg(format!("--{name}-sha512")).arg(pin);
    }
    command.arg("--output").arg(dir.join("bundle"));
    command
}

fn assert_private_output(output: &Output) {
    assert!(!String::from_utf8_lossy(&output.stdout).contains(VOTER));
    assert!(!String::from_utf8_lossy(&output.stderr).contains(VOTER));
}

#[test]
fn cli_prepares_selected_revote_and_existing_verifier_accepts_bundle() {
    let dir = TempDir::new();
    let f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let output = f.command(&dir.0).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_private_output(&output);
    let bundle = dir.0.join("bundle");
    let manifest: Manifest =
        serde_json::from_slice(&fs::read(bundle.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest.selected.len(), 1);
    assert_eq!(manifest.selected[0].cast_vote_id, f.rows[1]["id"]);
    let rows: Value =
        serde_json::from_slice(&fs::read(bundle.join("cast-votes.json")).unwrap()).unwrap();
    assert_eq!(rows[0]["content"], f.rows[1]["content"]);
    let anchors: Value =
        serde_json::from_slice(&fs::read(bundle.join("anchors.json")).unwrap()).unwrap();
    let verified = Command::new(env!("CARGO_BIN_EXE_ballot-audit"))
        .arg("--manifest")
        .arg(bundle.join("manifest.json"))
        .arg("--manifest-sha512")
        .arg(anchors["manifest_sha512"].as_str().unwrap())
        .arg("--cast-votes")
        .arg(bundle.join("cast-votes.json"))
        .arg("--configuration")
        .arg(bundle.join("configuration.borsh"))
        .arg("--configuration-sha512")
        .arg(anchors["configuration_sha512"].as_str().unwrap())
        .arg("--messages")
        .arg(bundle.join("messages.json"))
        .arg("--electoral-log")
        .arg(bundle.join("electoral-log.json"))
        .output()
        .unwrap();
    assert!(
        verified.status.success(),
        "{}",
        String::from_utf8_lossy(&verified.stderr)
    );
    assert_private_output(&verified);
    let provenance: Value =
        serde_json::from_slice(&fs::read(bundle.join("provenance.json")).unwrap()).unwrap();
    assert_eq!(provenance["checked_cast_votes"], 1);
    assert_eq!(
        provenance["selection_accounting"]["stage_exclusions"]["total"],
        0
    );
    let selection: Value =
        serde_json::from_slice(&fs::read(bundle.join("selection.json")).unwrap()).unwrap();
    assert_eq!(selection["rows"][0]["reason"], "superseded_revote");
    assert_eq!(selection["rows"][1]["reason"], "selected");
    assert_eq!(
        selection["rows"][1]["receipt_verification"],
        "selected_audit_verified"
    );
    assert_eq!(selection["rows"][2]["reason"], "superseded_revote");
    assert_eq!(selection["rows"][2]["receipt_verification"], "not_checked");
    assert_eq!(selection["rows"][3]["reason"], "outside_scope");
    assert!(provenance["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value
            .as_str()
            .unwrap()
            .contains("cannot prove historical integrity")));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&bundle).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(bundle.join("cast-votes.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    let original_manifest = fs::read(bundle.join("manifest.json")).unwrap();
    assert!(!f.command(&dir.0).output().unwrap().status.success());
    assert_eq!(
        fs::read(bundle.join("manifest.json")).unwrap(),
        original_manifest
    );
}

#[test]
fn cli_weighted_and_delegated_multiplicities_are_audited() {
    for (policy, multiplicity) in [
        (MultiplicityPolicy::Weighted, 5),
        (MultiplicityPolicy::Delegated, 3),
    ] {
        let dir = TempDir::new();
        let output = Fixture::new(policy, multiplicity)
            .command(&dir.0)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_private_output(&output);
    }
}

#[test]
fn missing_duplicate_or_wrong_voter_logs_fail_before_writing() {
    for variant in 0..3 {
        let dir = TempDir::new();
        let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
        match variant {
            0 => f.logs.clear(),
            1 => f.logs.push(f.logs[0].clone()),
            _ => {
                f.voters[0]["voter_id"] = "different-voter".into();
                f.rows[1]["voter_id_string"] = "different-voter".into();
            }
        }
        let output = f.command(&dir.0).output().unwrap();
        assert!(!output.status.success());
        assert_private_output(&output);
        assert!(!dir.0.join("bundle").exists());
    }
}

#[test]
fn missing_or_ambiguous_board_evidence_fails_before_writing() {
    for variant in 0..5 {
        let dir = TempDir::new();
        let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
        match variant {
            0 => {
                f.board.remove(0);
            }
            1 => {
                f.board.remove(1);
            }
            2 => f.board.push(f.board[0].clone()),
            3 => f.board.push(f.board[1].clone()),
            _ => {
                let mut config = Message::strand_deserialize(&f.board[0]).unwrap();
                config.artifact = Some(vec![0]);
                f.board[0] = config.strand_serialize().unwrap();
            }
        }
        let output = f.command(&dir.0).output().unwrap();
        assert!(!output.status.success());
        assert_private_output(&output);
        assert!(!dir.0.join("bundle").exists());
    }
}

#[test]
fn source_digest_mismatch_fails_before_parsing_or_writing() {
    let dir = TempDir::new();
    let f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let pins = f.write_sources(&dir.0);
    fs::write(dir.0.join("cast-votes.json"), VOTER).unwrap();
    let output = command_with_pins(&dir.0, &pins).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cast votes source digest mismatch"));
    assert_private_output(&output);
    assert!(!dir.0.join("bundle").exists());
}

#[test]
fn missing_frozen_envelope_or_scope_binding_fails_closed() {
    for replacement in [
        json!([]),
        json!({"format_version":1,"voters":[]}),
        json!({"format_version":1,"scope_sha512":"wrong","voters":[]}),
    ] {
        let dir = TempDir::new();
        let f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
        let mut pins = f.write_sources(&dir.0);
        let bytes = serde_json::to_vec(&replacement).unwrap();
        fs::write(dir.0.join("eligible-voters.json"), &bytes).unwrap();
        pins.iter_mut()
            .find(|(name, _)| *name == "eligible-voters")
            .unwrap()
            .1 = digest(&bytes).unwrap();
        let output = command_with_pins(&dir.0, &pins).output().unwrap();
        assert!(!output.status.success());
        assert_private_output(&output);
        assert!(!dir.0.join("bundle").exists());
    }
}

#[test]
fn parser_errors_do_not_echo_private_raw_values() {
    let dir = TempDir::new();
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.rows[1]["status"] = VOTER.into();
    let output = f.command(&dir.0).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid full cast_vote table export"));
    assert_private_output(&output);
}

#[test]
fn stage_exclusion_artifact_classifies_discarded_and_ineligible_receipts() {
    let dir = TempDir::new();
    let mut f = Fixture::new(MultiplicityPolicy::Delegated, 3);
    let mut discarded = f.rows[1].clone();
    discarded["id"] = "00000000-0000-4000-8000-000000000005".into();
    discarded["voter_id_string"] = "discarded-voter".into();
    discarded["ballot_id"] = "stored-discarded-receipt-not-cryptographically-checked".into();
    discarded["content"] = "malformed excluded content is not consumed".into();
    discarded["status"] = "discarded".into();
    f.rows.as_array_mut().unwrap().push(discarded);
    f.voters
        .as_array_mut()
        .unwrap()
        .push(json!({"voter_id":"discarded-voter","multiplicity":7}));
    let mut ineligible = f.rows[1].clone();
    ineligible["id"] = "00000000-0000-4000-8000-000000000006".into();
    ineligible["voter_id_string"] = "not-in-frozen-eligibility".into();
    ineligible["ballot_id"] = "stored-ineligible-receipt-not-cryptographically-checked".into();
    ineligible["content"] = "malformed excluded content is not consumed".into();
    f.rows.as_array_mut().unwrap().push(ineligible);
    let output = f.command(&dir.0).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_private_output(&output);
    let selection: Value =
        serde_json::from_slice(&fs::read(dir.0.join("bundle/selection.json")).unwrap()).unwrap();
    assert_eq!(
        selection["accounting"]["stage_exclusions"],
        json!({"discarded":1,"no_eligible_voter":1,"total":2})
    );
    assert_eq!(selection["rows"][4]["reason"], "discarded");
    assert_eq!(selection["rows"][4]["stage_cast_count"], 1);
    assert_eq!(selection["rows"][4]["resolved_multiplicity"], 7);
    assert_eq!(selection["rows"][5]["reason"], "no_eligible_voter");
    assert_eq!(selection["rows"][5]["stage_cast_count"], 1);
    assert_eq!(selection["rows"][5]["receipt_verification"], "not_checked");
    assert!(!serde_json::to_string(&selection).unwrap().contains(VOTER));
}

#[test]
#[ignore = "writes synthetic preparation exports only when explicitly requested"]
fn export_synthetic_preparation_inputs() {
    let dir = PathBuf::from(
        std::env::var("BALLOT_AUDIT_PREPARATION_FIXTURE_DIR")
            .expect("set BALLOT_AUDIT_PREPARATION_FIXTURE_DIR"),
    );
    fs::create_dir(&dir).unwrap();
    let f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let pins = f.write_sources(&dir);
    fs::write(
        dir.join("source-anchors.json"),
        serde_json::to_vec_pretty(
            &pins
                .into_iter()
                .collect::<std::collections::BTreeMap<_, _>>(),
        )
        .unwrap(),
    )
    .unwrap();
}
