// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use b3::messages::{
    artifact::{Ballots, Configuration},
    message::{Message, Signer},
    newtypes::{
        ChannelsHashes, ConfigurationHash, PublicKeyHash, SharesHashes, MAX_TRUSTEES, NULL_TRUSTEE,
    },
    protocol_manager::ProtocolManager,
    statement::Statement,
};
use ballot_audit::{digest, Manifest, MultiplicityPolicy, Scope};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use electoral_log::messages::{
    message::{Message as ElectoralMessage, SigningData},
    newtypes::{
        CastVoteHash, ElectionIdString, EventIdString, PseudonymHash, VoterCountryString,
        VoterIpString, VotingChannelString,
    },
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
    marker::PhantomData,
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

const VOTER: &str = "synthetic-private-voter-identifier";

struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "ballot-audit-prepare-test-{}-{}-{}",
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
    pm: ProtocolManager<RistrettoCtx>,
    configuration: Configuration<RistrettoCtx>,
    scope: Value,
    rows: Value,
    voters: Value,
    board: Vec<Vec<u8>>,
    logs: Vec<Vec<u8>>,
    history: History,
}

/// Cast-vote history passed to prepare.
enum History {
    /// Every row with a real ballot and receipt, taken when the sources are
    /// written. Synthetic placeholder contents and receipt labels of
    /// unselected rows cannot be hashed.
    FromRows,
    Rows(Value),
    /// Passes --without-cast-vote-history.
    Skipped,
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
            pm,
            configuration,
            scope: json!({"format_version":1,"scope":scope,"batches":batches}),
            rows: json!([older, latest, discarded, outside]),
            voters: json!([{"voter_id":VOTER,"multiplicity":multiplicity}]),
            board,
            logs: vec![log],
            history: History::FromRows,
        }
    }

    fn history_rows(&self) -> Option<Value> {
        match &self.history {
            History::FromRows => Some(Value::Array(
                self.rows
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|row| {
                        serde_json::from_str::<Value>(row["content"].as_str().unwrap()).is_ok()
                            && row["ballot_id"]
                                .as_str()
                                .is_some_and(|id| id.len() == 64 && hex::decode(id).is_ok())
                    })
                    .cloned()
                    .collect(),
            )),
            History::Rows(rows) => Some(rows.clone()),
            History::Skipped => None,
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
        .chain(self.history_rows().map(|history| {
            (
                "cast-vote-history",
                serde_json::to_vec_pretty(&history).unwrap(),
            )
        }))
        .map(|(name, bytes)| {
            fs::write(dir.join(format!("{name}.json")), &bytes).unwrap();
            (name, digest(&bytes).unwrap())
        })
        .collect()
    }

    fn command(&self, dir: &Path) -> Command {
        command_with_pins(dir, &self.write_sources(dir))
    }

    fn resigned_log(&self, user_id: &str) -> Vec<u8> {
        let log = ElectoralMessage::strand_deserialize(&self.logs[0]).unwrap();
        ElectoralMessage::sign(
            log.statement,
            log.artifact,
            &self.pm.signing_key,
            "synthetic",
            &self.pm.signing_key,
            Some(user_id.into()),
            log.username,
            log.election_id,
            log.area_id,
            log.ballot_id,
        )
        .unwrap()
        .strand_serialize()
        .unwrap()
    }

    fn public_key_message(&self, public_key_hash: [u8; 64]) -> Vec<u8> {
        let configuration_hash =
            strand::hash::hash_to_array(&self.configuration.strand_serialize().unwrap()).unwrap();
        let hashes = [[0; 64]; MAX_TRUSTEES];
        self.pm
            .sign(
                Statement::PublicKeySigned(
                    0,
                    ConfigurationHash(configuration_hash),
                    PublicKeyHash(public_key_hash),
                    SharesHashes(hashes),
                    ChannelsHashes(hashes),
                ),
                None,
            )
            .unwrap()
            .strand_serialize()
            .unwrap()
    }

    /// A validly signed success log for VOTER naming another full ballot hash,
    /// signed `seconds_later` than the selected ballot's log.
    fn signed_log_for_hash(&self, hash: [u8; 64], seconds_later: u64) -> Vec<u8> {
        let mut log = ElectoralMessage::strand_deserialize(&self.logs[0]).unwrap();
        if let StatementBody::CastVoteWithChannel(_, _, h, _, _, _) = &mut log.statement.body {
            *h = CastVoteHash::new(hash);
        }
        log.statement.head.timestamp += seconds_later;
        ElectoralMessage::sign(
            log.statement,
            log.artifact,
            &self.pm.signing_key,
            "synthetic",
            &self.pm.signing_key,
            log.user_id,
            log.username,
            log.election_id,
            log.area_id,
            log.ballot_id,
        )
        .unwrap()
        .strand_serialize()
        .unwrap()
    }

    /// Adds a signed revote by VOTER with another ballot, stored `created_at`
    /// in `area` with `status`, signed `seconds_later` than the selected ballot.
    fn add_revote(&mut self, id: u64, area: &str, status: &str, seconds_later: u64) {
        let ballot = other_ballot();
        let mut row = self.rows[1].clone();
        row["id"] = format!("00000000-0000-4000-8000-{id:012x}").into();
        row["area_id"] = area.into();
        row["status"] = status.into();
        row["created_at"] = "2026-09-30T12:00:00Z".into();
        row["content"] = serde_json::to_string(&ballot).unwrap().into();
        row["ballot_id"] = hash_ballot(&ballot).unwrap().into();
        self.rows.as_array_mut().unwrap().push(row);
        let log = self.signed_log_for_hash(hash_ballot_sha512(&ballot).unwrap(), seconds_later);
        self.logs.push(log);
    }

    /// One election and one area with `voters` eligible voters who each cast
    /// one valid ballot and never revote, then `disabled` voters who cast one
    /// valid ballot and were disabled afterwards, so the frozen list and the
    /// tally leave them out, then `abstainers` eligible voters who did not
    /// vote. The electoral log lost the CastVote records of the voters at the
    /// `lost` positions.
    fn single_cast_per_voter(
        voters: usize,
        disabled: usize,
        abstainers: usize,
        lost: &[usize],
    ) -> Self {
        let mut f = Self::new(MultiplicityPolicy::Unweighted, 1);
        let template = f.rows[1].clone();
        let mut rows = Vec::new();
        let mut ciphertexts = Vec::new();
        let mut logs = Vec::new();
        for index in 0..voters + disabled {
            let voter = format!("{VOTER}-{index:03}");
            let ballot = other_ballot();
            if index < voters {
                ciphertexts.push(
                    ballot.deserialize_contests::<RistrettoCtx>().unwrap()[0]
                        .ciphertext
                        .clone(),
                );
            }
            let mut row = template.clone();
            row["id"] = format!("00000000-0000-4000-8000-{:012x}", 0x100 + index).into();
            row["voter_id_string"] = voter.clone().into();
            row["created_at"] = format!("2026-10-01T00:{:02}:00Z", index % 60).into();
            row["content"] = serde_json::to_string(&ballot).unwrap().into();
            row["ballot_id"] = hash_ballot(&ballot).unwrap().into();
            rows.push(row);
            if lost.contains(&index) {
                continue;
            }
            let mut log = ElectoralMessage::strand_deserialize(&f.logs[0]).unwrap();
            if let StatementBody::CastVoteWithChannel(_, pseudonym, hash, _, _, _) =
                &mut log.statement.body
            {
                *pseudonym = PseudonymHash::new(
                    strand::hash::hash_to_array(&voter.strand_serialize().unwrap()).unwrap(),
                );
                *hash = CastVoteHash::new(hash_ballot_sha512(&ballot).unwrap());
            }
            logs.push(
                ElectoralMessage::sign(
                    log.statement,
                    log.artifact,
                    &f.pm.signing_key,
                    "synthetic",
                    &f.pm.signing_key,
                    log.user_id,
                    log.username,
                    log.election_id,
                    log.area_id,
                    log.ballot_id,
                )
                .unwrap()
                .strand_serialize()
                .unwrap(),
            );
        }
        let mut trustees = [NULL_TRUSTEE; MAX_TRUSTEES];
        trustees[0] = 1;
        trustees[1] = 2;
        f.board[1] = Message::ballots_msg(
            &f.configuration,
            10,
            &Ballots::new(ciphertexts),
            trustees,
            PublicKeyHash([7; 64]),
            &f.pm,
        )
        .unwrap()
        .strand_serialize()
        .unwrap();
        f.voters = (0..voters)
            .chain(voters + disabled..voters + disabled + abstainers)
            .map(|index| json!({"voter_id":format!("{VOTER}-{index:03}"),"multiplicity":1}))
            .collect();
        f.rows = Value::Array(rows);
        f.logs = logs;
        f
    }

    /// The exporter's saved-mapping.json for this scope, with `annotations`.
    fn mapping_command(
        &self,
        dir: &Path,
        annotations: Value,
        change: impl FnOnce(&mut Value),
    ) -> Command {
        let scope = &self.scope["scope"];
        let mut mapping = json!({"tally_session_contest": {
            "id": scope["tally_session_contest_id"], "tenant_id": scope["tenant_id"],
            "election_event_id": scope["election_event_id"], "election_id": scope["election_id"],
            "area_id": scope["area_id"], "contest_id": scope["contest_id"],
            "tally_session_id": scope["tally_session_id"], "session_id": scope["base_batch"],
            "created_at": "2026-10-20T00:00:00+00:00", "last_updated_at": null,
            "labels": null, "annotations": annotations,
        }});
        change(&mut mapping);
        let mapping = serde_json::to_vec_pretty(&mapping).unwrap();
        fs::write(dir.join("saved-mapping.json"), &mapping).unwrap();
        let mut command = self.command(dir);
        command
            .arg("--saved-mapping")
            .arg(dir.join("saved-mapping.json"))
            .arg("--saved-mapping-sha512")
            .arg(digest(&mapping).unwrap());
        command
    }
}

/// Audits a prepared bundle with the pins read from its own anchors.json,
/// which only shows the bundle is self-consistent.
fn audit_bundle(dir: &Path, extra: &[&str]) -> Output {
    let bundle = dir.join("bundle");
    let anchors: Value =
        serde_json::from_slice(&fs::read(bundle.join("anchors.json")).unwrap()).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_ballot-audit"));
    for name in ["cast-votes", "messages", "electoral-log"] {
        command
            .arg(format!("--{name}"))
            .arg(bundle.join(format!("{name}.json")));
    }
    command
        .arg("--manifest")
        .arg(bundle.join("manifest.json"))
        .arg("--manifest-sha512")
        .arg(anchors["manifest_sha512"].as_str().unwrap())
        .arg("--configuration")
        .arg(bundle.join("configuration.borsh"))
        .arg("--configuration-sha512")
        .arg(anchors["configuration_sha512"].as_str().unwrap())
        .args(extra);
    let output = command.output().unwrap();
    assert_private_output(&output);
    output
}

fn stdout_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

fn provenance(dir: &Path) -> Value {
    serde_json::from_slice(&fs::read(dir.join("bundle/provenance.json")).unwrap()).unwrap()
}

fn command_with_pins(dir: &Path, sources: &[(&str, String)]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_prepare-ballot-audit"));
    for (name, pin) in sources {
        command
            .arg(format!("--{name}"))
            .arg(dir.join(format!("{name}.json")));
        command.arg(format!("--{name}-sha512")).arg(pin);
    }
    if !sources.iter().any(|(name, _)| *name == "cast-vote-history") {
        command.arg("--without-cast-vote-history");
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
fn selected_vote_without_log_is_prepared_and_listed_to_audit() {
    for history in [History::FromRows, History::Skipped] {
        let dir = TempDir::new();
        let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
        f.logs.clear();
        f.history = history;
        let output = f.command(&dir.0).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_private_output(&output);
        let manifest: Manifest =
            serde_json::from_slice(&fs::read(dir.0.join("bundle/manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest.selected[0].electoral_log_message_sha512, None);
        assert_eq!(provenance(&dir.0)["votes_to_audit"], 1);
        assert_eq!(provenance(&dir.0)["checked_electoral_log_messages"], 0);
        let audited = audit_bundle(&dir.0, &[]);
        assert_eq!(audited.status.code(), Some(0));
        let report = stdout_json(&audited);
        assert_eq!(report["votes_to_audit"][0]["cast_vote_id"], f.rows[1]["id"]);
        assert_eq!(
            report["votes_to_audit"][0]["reason"],
            "no_signed_cast_vote_record"
        );
    }
}

/// Every voter cast once and the history is otherwise consistent, but the
/// electoral log lost two CastVote records. Each check passes and lists the
/// same two votes to audit.
#[test]
fn lost_cast_vote_records_pass_every_check_and_are_listed_to_audit() {
    let dir = TempDir::new();
    let f = Fixture::single_cast_per_voter(20, 4, 3, &[6, 13, 21]);
    let ids = |indexes: &[usize]| -> Vec<Value> {
        indexes
            .iter()
            .map(|index| f.rows[index]["id"].clone())
            .collect()
    };
    let output = f.command(&dir.0).output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let provenance = provenance(&dir.0);
    assert_eq!(provenance["checked_cast_votes"], 20);
    assert_eq!(provenance["checked_electoral_log_messages"], 18);
    assert_eq!(provenance["votes_to_audit"], 2);
    let accounting = &provenance["selection_accounting"];
    assert_eq!(accounting["rows_by_reason"]["selected"]["rows"], 20);
    assert_eq!(accounting["stage_exclusions"]["no_eligible_voter"], 4);
    assert_eq!(accounting["frozen_eligible_voters_without_valid_vote"], 3);

    let audited = audit_bundle(&dir.0, &[]);
    assert_eq!(audited.status.code(), Some(0));
    let report = stdout_json(&audited);
    assert_eq!(report["checked_cast_votes"], 20);
    assert_eq!(report["checked_electoral_log_messages"], 18);
    let listed: Vec<Value> = report["votes_to_audit"]
        .as_array()
        .unwrap()
        .iter()
        .map(|vote| vote["cast_vote_id"].clone())
        .collect();
    assert_eq!(listed, ids(&[6, 13]));

    let pin = |name: &str| digest(&fs::read(dir.0.join(format!("{name}.json"))).unwrap()).unwrap();
    let history = Command::new(env!("CARGO_BIN_EXE_audit-cast-history"))
        .arg("--scope")
        .arg(dir.0.join("scope.json"))
        .args(["--scope-sha512", &pin("scope")])
        .arg("--cast-vote-history")
        .arg(dir.0.join("cast-vote-history.json"))
        .args(["--cast-vote-history-sha512", &pin("cast-vote-history")])
        .arg("--electoral-log")
        .arg(dir.0.join("electoral-log.json"))
        .args(["--electoral-log-sha512", &pin("electoral-log")])
        .output()
        .unwrap();
    assert_eq!(history.status.code(), Some(0));
    let history = stdout_json(&history);
    assert_eq!(history["outcome"], "success");
    assert_eq!(history["checked_hasura_rows"], 24);
    assert_eq!(history["checked_signed_cast_vote_log_records"], 21);
    assert_eq!(history["signed_log_records_without_hasura_row"], 0);
    assert_eq!(history["hasura_rows_without_signed_log"], 3);
    let mut listed: Vec<Value> = history["receipts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|group| group["classification"] == "hasura_row_without_signed_log")
        .flat_map(|group| group["cast_vote_ids"].as_array().unwrap().clone())
        .collect();
    listed.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
    // The history also lists the disabled voter's vote, which the tally left out.
    assert_eq!(listed, ids(&[6, 13, 21]));
}

#[test]
fn ambiguous_logs_are_incomplete_and_a_ballot_signed_for_another_voter_contradicts() {
    for (variant, expected) in [(0, 3), (1, 1)] {
        let dir = TempDir::new();
        let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
        if variant == 0 {
            f.logs.push(f.resigned_log("different-unsigned-metadata"));
        } else {
            f.voters[0]["voter_id"] = "different-voter".into();
            f.rows[1]["voter_id_string"] = "different-voter".into();
        }
        let output = f.command(&dir.0).output().unwrap();
        assert_eq!(output.status.code(), Some(expected), "variant {variant}");
        assert_private_output(&output);
        assert!(!dir.0.join("bundle").exists());
    }
}

fn other_ballot() -> HashableBallot {
    let ballot = encrypt_decoded_contest::<RistrettoCtx>(
        &RistrettoCtx,
        &vec![get_writein_plaintext()],
        &get_writein_ballot_style(),
    )
    .unwrap();
    HashableBallot::try_from(&SignedHashableBallot::try_from(&ballot).unwrap()).unwrap()
}

#[test]
fn history_is_required_unless_explicitly_skipped() {
    let dir = TempDir::new();
    let f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let mut pins = f.write_sources(&dir.0);
    pins.retain(|(name, _)| *name != "cast-vote-history");
    let mut command = Command::new(env!("CARGO_BIN_EXE_prepare-ballot-audit"));
    for (name, pin) in &pins {
        command
            .arg(format!("--{name}"))
            .arg(dir.0.join(format!("{name}.json")))
            .arg(format!("--{name}-sha512"))
            .arg(pin);
    }
    let output = command
        .arg("--output")
        .arg(dir.0.join("bundle"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--cast-vote-history"));

    let output = f
        .command(&dir.0)
        .arg("--without-cast-vote-history")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn history_confirms_every_signed_ballot_of_selected_voters() {
    let dir = TempDir::new();
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let output = f.command(&dir.0).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        provenance(&dir.0)["cast_vote_history"],
        json!({"status":"checked","signed_ballots_of_selected_voters":1,
            "later_signed_revotes_not_valid_in_scope":0})
    );
    f.history = History::Skipped;
    let dir = TempDir::new();
    assert!(f.command(&dir.0).output().unwrap().status.success());
    assert_eq!(
        provenance(&dir.0)["cast_vote_history"],
        json!({"status":"skipped"})
    );
}

#[test]
fn replaced_selected_ballot_is_a_contradiction_only_with_history() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let forged = other_ballot();
    // The out-of-area row keeps the original ballot; remove it so no stored row does.
    f.rows.as_array_mut().unwrap().remove(3);
    f.rows[1]["content"] = serde_json::to_string(&forged).unwrap().into();
    f.rows[1]["ballot_id"] = hash_ballot(&forged).unwrap().into();
    let dir = TempDir::new();
    let output = f.command(&dir.0).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_private_output(&output);
    assert!(String::from_utf8_lossy(&output.stderr).contains("no stored cast vote holds"));
    assert!(!dir.0.join("bundle").exists());
    f.history = History::Skipped;
    let dir = TempDir::new();
    assert_eq!(f.command(&dir.0).output().unwrap().status.code(), Some(3));
}

#[test]
fn removed_later_revote_is_a_contradiction_only_with_history() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.logs
        .push(f.signed_log_for_hash(hash_ballot_sha512(&other_ballot()).unwrap(), 60));
    let dir = TempDir::new();
    let output = f.command(&dir.0).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(!dir.0.join("bundle").exists());
    f.history = History::Skipped;
    let dir = TempDir::new();
    assert!(f.command(&dir.0).output().unwrap().status.success());
}

#[test]
fn history_must_hold_selected_rows() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.history = History::Rows(json!([]));
    let dir = TempDir::new();
    let output = f.command(&dir.0).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("does not hold a selected cast vote"));
}

#[test]
fn later_signed_valid_revote_must_be_selected() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let area = f.scope["scope"]["area_id"].as_str().unwrap().to_owned();
    // Stored as older than the selected row, but signed a minute later.
    f.add_revote(5, &area, "valid", 60);
    let dir = TempDir::new();
    let output = f.command(&dir.0).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_private_output(&output);
    assert!(String::from_utf8_lossy(&output.stderr).contains("later signed revote"));
    assert!(!dir.0.join("bundle").exists());
    f.history = History::Skipped;
    let dir = TempDir::new();
    assert!(f.command(&dir.0).output().unwrap().status.success());
}

#[test]
fn ballot_signed_again_after_the_selected_one_counts_as_a_later_revote() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let area = f.scope["scope"]["area_id"].as_str().unwrap().to_owned();
    f.add_revote(5, &area, "valid", 0);
    let revote: HashableBallot = serde_json::from_str(
        f.rows.as_array().unwrap().last().unwrap()["content"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    // The same ballot signed once alongside the selected one and once later.
    let log = f.signed_log_for_hash(hash_ballot_sha512(&revote).unwrap(), 60);
    f.logs.push(log);
    let dir = TempDir::new();
    let output = f.command(&dir.0).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("later signed revote"), "{stderr}");
}

#[test]
fn later_signed_revotes_that_cannot_be_selected_here_are_counted() {
    for (area, status, seconds_later, counted) in [
        (None, "discarded", 60, 1),
        (Some("00000000-0000-4000-8000-000000000098"), "valid", 60, 1),
        (None, "valid", 0, 0),
    ] {
        let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
        let area = area
            .map(str::to_owned)
            .unwrap_or_else(|| f.scope["scope"]["area_id"].as_str().unwrap().to_owned());
        f.add_revote(5, &area, status, seconds_later);
        let dir = TempDir::new();
        let output = f.command(&dir.0).output().unwrap();
        assert!(
            output.status.success(),
            "{status} {seconds_later}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            provenance(&dir.0)["cast_vote_history"]["later_signed_revotes_not_valid_in_scope"],
            counted
        );
    }
}

#[test]
fn byte_identical_duplicate_logs_collapse_to_one_bundle_record() {
    let dir = TempDir::new();
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.logs.push(f.logs[0].clone());
    f.logs.push(f.logs[0].clone());
    let output = f.command(&dir.0).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        provenance(&dir.0)["collapsed_identical_electoral_log_records"],
        2
    );
    let logs: Vec<String> =
        serde_json::from_slice(&fs::read(dir.0.join("bundle/electoral-log.json")).unwrap())
            .unwrap();
    assert_eq!(logs.len(), 1);
    assert_eq!(audit_bundle(&dir.0, &[]).status.code(), Some(0));
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
    assert_eq!(output.status.code(), Some(1));
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
    assert_eq!(output.status.code(), Some(4));
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

fn with_exclusions() -> Fixture {
    let mut f = Fixture::new(MultiplicityPolicy::Delegated, 3);
    let mut discarded = f.rows[1].clone();
    discarded["id"] = "00000000-0000-4000-8000-000000000005".into();
    discarded["voter_id_string"] = "discarded-voter".into();
    discarded["ballot_id"] = "stored-discarded-label".into();
    discarded["content"] = "malformed excluded content is not consumed".into();
    discarded["status"] = "discarded".into();
    discarded["annotations"] = json!({"voting_channel":"KIOSK"});
    f.rows.as_array_mut().unwrap().push(discarded);
    f.voters
        .as_array_mut()
        .unwrap()
        .push(json!({"voter_id":"discarded-voter","multiplicity":7}));
    let mut ineligible = f.rows[1].clone();
    ineligible["id"] = "00000000-0000-4000-8000-000000000006".into();
    ineligible["voter_id_string"] = "not-in-frozen-eligibility".into();
    ineligible["ballot_id"] = "stored-ineligible-label".into();
    ineligible["annotations"] = Value::Null;
    f.rows.as_array_mut().unwrap().push(ineligible);
    f
}

#[test]
fn cli_audit_reports_each_receipt_lookup_outcome() {
    let dir = TempDir::new();
    let f = with_exclusions();
    let output = f.command(&dir.0).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let selection = dir.0.join("bundle/selection.json");
    let selection = selection.to_str().unwrap();
    let receipt = f.rows[1]["ballot_id"].as_str().unwrap();

    let selected = audit_bundle(&dir.0, &["--ballot-id", receipt]);
    assert_eq!(selected.status.code(), Some(0));
    let report = stdout_json(&selected);
    assert_eq!(report["lookup"]["outcome"], "selected");
    assert_eq!(report["lookup"]["locations"].as_array().unwrap().len(), 3);

    let excluded = audit_bundle(
        &dir.0,
        &[
            "--ballot-id",
            "stored-discarded-label",
            "--selection",
            selection,
        ],
    );
    assert_eq!(excluded.status.code(), Some(3));
    let report = stdout_json(&excluded);
    assert_eq!(report["checked_cast_votes"], 1);
    assert_eq!(report["lookup"]["outcome"], "excluded");
    assert_eq!(report["lookup"]["rows"][0]["reason"], "discarded");
    assert!(report["lookup"]["stored_labels"]
        .as_str()
        .unwrap()
        .contains("unverified"));

    let unexplained = audit_bundle(&dir.0, &["--ballot-id", "stored-discarded-label"]);
    assert_eq!(unexplained.status.code(), Some(3));
    assert_eq!(stdout_json(&unexplained)["lookup"]["outcome"], "missing");
    assert_eq!(
        stdout_json(&unexplained)["lookup"]["selection_searched"],
        false
    );
    let missing = audit_bundle(&dir.0, &["--ballot-id", "absent", "--selection", selection]);
    assert_eq!(missing.status.code(), Some(3));
    assert_eq!(stdout_json(&missing)["lookup"]["selection_searched"], true);

    let unreadable = audit_bundle(&dir.0, &["--selection", "/nonexistent/selection.json"]);
    assert_eq!(unreadable.status.code(), Some(4));
    assert!(unreadable.stdout.is_empty());
}

#[test]
fn cli_audit_exit_statuses_follow_the_shared_taxonomy() {
    let dir = TempDir::new();
    let f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    assert!(f.command(&dir.0).output().unwrap().status.success());
    let bundle = dir.0.join("bundle");
    let anchors: Value =
        serde_json::from_slice(&fs::read(bundle.join("anchors.json")).unwrap()).unwrap();
    let base = |manifest_pin: &str| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ballot-audit"));
        command
            .arg("--manifest")
            .arg(bundle.join("manifest.json"))
            .arg("--manifest-sha512")
            .arg(manifest_pin)
            .arg("--configuration")
            .arg(bundle.join("configuration.borsh"))
            .arg("--configuration-sha512")
            .arg(anchors["configuration_sha512"].as_str().unwrap());
        for name in ["cast-votes", "messages", "electoral-log"] {
            command
                .arg(format!("--{name}"))
                .arg(bundle.join(format!("{name}.json")));
        }
        command.output().unwrap()
    };
    let pin = anchors["manifest_sha512"].as_str().unwrap();
    assert_eq!(base(pin).status.code(), Some(0));
    assert_eq!(base(&pin.to_uppercase()).status.code(), Some(4));
    assert_eq!(base("short").status.code(), Some(4));
    assert_eq!(base(&"0".repeat(128)).status.code(), Some(1));
    let usage = Command::new(env!("CARGO_BIN_EXE_ballot-audit"))
        .output()
        .unwrap();
    assert_eq!(usage.status.code(), Some(2));
    fs::write(bundle.join("electoral-log.json"), b"[]").unwrap();
    assert_eq!(base(pin).status.code(), Some(3));
    fs::write(bundle.join("electoral-log.json"), b"not json").unwrap();
    assert_eq!(base(pin).status.code(), Some(4));
}

#[test]
fn prepare_exit_statuses_follow_the_shared_taxonomy() {
    let dir = TempDir::new();
    let f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let mut pins = f.write_sources(&dir.0);
    pins[0].1 = pins[0].1.to_uppercase();
    assert_eq!(
        command_with_pins(&dir.0, &pins)
            .output()
            .unwrap()
            .status
            .code(),
        Some(4)
    );
    let dir = TempDir::new();
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.board.remove(0);
    assert_eq!(f.command(&dir.0).output().unwrap().status.code(), Some(3));
    let dir = TempDir::new();
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.board.push(f.board[0].clone());
    assert_eq!(f.command(&dir.0).output().unwrap().status.code(), Some(1));
}

#[test]
fn saved_tally_annotations_match_reconstructed_accounting() {
    let dir = TempDir::new();
    let f = with_exclusions();
    let output = f
        .mapping_command(
            &dir.0,
            json!({"elegible_voters":2,"ballots_without_voter":2,"casted_ballots":5,
                "votes_by_channel":{"ONLINE":4,"KIOSK":1}}),
            |_| {},
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let provenance = provenance(&dir.0);
    assert_eq!(
        provenance["tally_annotations"],
        json!({"status":"checked","votes_by_channel":"matched","weight_bit_mask":"matched"})
    );
    assert!(provenance["source_sha512"]["saved_mapping"].is_string());

    let dir = TempDir::new();
    let f = Fixture::new(MultiplicityPolicy::Weighted, 5);
    let annotations = json!({"elegible_voters":1,"ballots_without_voter":0,"casted_ballots":1,
        "votes_by_channel":{"ONLINE":5},"weight_bit_mask":5});
    assert!(f
        .mapping_command(&dir.0, annotations, |_| {})
        .output()
        .unwrap()
        .status
        .success());
}

type MappingChange = fn(&mut Value);

#[test]
fn saved_tally_annotation_or_identity_mismatch_fails_before_writing() {
    let weighted = json!({"elegible_voters":1,"ballots_without_voter":0,"casted_ballots":1,
        "votes_by_channel":{"ONLINE":5},"weight_bit_mask":5});
    let cases: Vec<(&str, Value, MappingChange)> = vec![
        ("casted_ballots", json!(2), |_| {}),
        ("ballots_without_voter", json!(1), |_| {}),
        ("elegible_voters", json!(2), |_| {}),
        ("votes_by_channel", json!({"ONLINE":4,"KIOSK":1}), |_| {}),
        ("weight_bit_mask", json!(1), |_| {}),
        ("weight_bit_mask", json!(5), |m| {
            m["tally_session_contest"]["session_id"] = json!(11)
        }),
        ("weight_bit_mask", json!(5), |m| {
            m["tally_session_contest"]["area_id"] = json!("00000000-0000-4000-8000-000000000099")
        }),
    ];
    for (field, value, change) in cases {
        let dir = TempDir::new();
        let f = Fixture::new(MultiplicityPolicy::Weighted, 5);
        let mut annotations = weighted.clone();
        annotations[field] = value;
        let output = f
            .mapping_command(&dir.0, annotations, change)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{field}");
        assert_private_output(&output);
        assert!(!dir.0.join("bundle").exists());
    }
}

#[test]
fn absent_or_legacy_saved_annotations_are_reported_as_not_checked() {
    let dir = TempDir::new();
    let f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    assert!(f.command(&dir.0).output().unwrap().status.success());
    assert_eq!(
        provenance(&dir.0)["tally_annotations"],
        json!({"status":"not_supplied"})
    );
    let dir = TempDir::new();
    assert!(f
        .mapping_command(&dir.0, Value::Null, |_| {})
        .output()
        .unwrap()
        .status
        .success());
    assert_eq!(
        provenance(&dir.0)["tally_annotations"],
        json!({"status":"annotations_absent"})
    );
    let dir = TempDir::new();
    let annotations = json!({"elegible_voters":1,"ballots_without_voter":0,"casted_ballots":1});
    assert!(f
        .mapping_command(&dir.0, annotations, |_| {})
        .output()
        .unwrap()
        .status
        .success());
    assert_eq!(
        provenance(&dir.0)["tally_annotations"]["votes_by_channel"],
        "not_recorded"
    );
    let dir = TempDir::new();
    let output = f
        .mapping_command(&dir.0, json!({"unexpected": true}), |_| {})
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
}

#[test]
fn pinned_public_key_is_confirmed_against_board_public_key_statement() {
    let dir = TempDir::new();
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    assert!(f.command(&dir.0).output().unwrap().status.success());
    assert_eq!(
        provenance(&dir.0)["board_public_key"],
        json!({"status":"not_on_board"})
    );
    let dir = TempDir::new();
    f.board.push(f.public_key_message([7; 64]));
    assert!(f.command(&dir.0).output().unwrap().status.success());
    assert_eq!(
        provenance(&dir.0)["board_public_key"],
        json!({"status":"matched","signed_statements":1})
    );
    let dir = TempDir::new();
    f.board.push(f.public_key_message([8; 64]));
    let output = f.command(&dir.0).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("pinned public key hash differs"));
    assert!(!dir.0.join("bundle").exists());
}

#[test]
fn identical_ballot_selected_for_two_voters_fails_before_writing() {
    let dir = TempDir::new();
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let mut copy = f.rows[1].clone();
    copy["id"] = "00000000-0000-4000-8000-000000000007".into();
    copy["voter_id_string"] = "second-voter".into();
    f.rows.as_array_mut().unwrap().push(copy);
    f.voters
        .as_array_mut()
        .unwrap()
        .push(json!({"voter_id":"second-voter","multiplicity":1}));
    let log = ElectoralMessage::strand_deserialize(&f.logs[0]).unwrap();
    let mut statement = log.statement;
    if let electoral_log::messages::statement::StatementBody::CastVoteWithChannel(_, p, ..) =
        &mut statement.body
    {
        *p = PseudonymHash::new(
            strand::hash::hash_to_array(&"second-voter".to_owned().strand_serialize().unwrap())
                .unwrap(),
        );
    }
    f.logs.push(
        ElectoralMessage::sign(
            statement,
            log.artifact,
            &f.pm.signing_key,
            "synthetic",
            &f.pm.signing_key,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap()
        .strand_serialize()
        .unwrap(),
    );
    let message = Message::strand_deserialize(&f.board[1]).unwrap();
    let ballots =
        Ballots::<RistrettoCtx>::strand_deserialize(message.artifact.as_ref().unwrap()).unwrap();
    let mut trustees = [NULL_TRUSTEE; MAX_TRUSTEES];
    trustees[0] = 1;
    trustees[1] = 2;
    f.board[1] = Message::ballots_msg(
        &f.configuration,
        10,
        &Ballots::new(vec![ballots.ciphertexts.0[0].clone(); 2]),
        trustees,
        PublicKeyHash([7; 64]),
        &f.pm,
    )
    .unwrap()
    .strand_serialize()
    .unwrap();
    let output = f.command(&dir.0).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("shared by several selected"));
    assert_private_output(&output);
    assert!(!dir.0.join("bundle").exists());
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

#[test]
#[ignore = "writes the lost CastVote record inputs only when explicitly requested"]
fn export_lost_cast_vote_record_inputs() {
    let dir = PathBuf::from(
        std::env::var("BALLOT_AUDIT_LOST_LOG_FIXTURE_DIR")
            .expect("set BALLOT_AUDIT_LOST_LOG_FIXTURE_DIR"),
    );
    fs::create_dir(&dir).unwrap();
    Fixture::single_cast_per_voter(20, 4, 3, &[6, 13, 21]).write_sources(&dir);
}
