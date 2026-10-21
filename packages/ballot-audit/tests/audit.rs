// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use b3::messages::{
    artifact::{Ballots, Configuration},
    message::Message,
    newtypes::{PublicKeyHash, MAX_TRUSTEES, NULL_TRUSTEE},
    protocol_manager::ProtocolManager,
};
use ballot_audit::{audit, digest, CastVote, Manifest, MultiplicityPolicy, Scope, SelectedVote};
use sequent_core::{
    ballot::{HashableBallot, SignedHashableBallot},
    encrypt::{encrypt_decoded_contest, hash_ballot},
    fixtures::ballot_codec::{get_writein_ballot_style, get_writein_plaintext},
};
use std::marker::PhantomData;
use strand::{
    backend::ristretto::RistrettoCtx,
    serialization::{StrandDeserialize, StrandSerialize},
    signature::{StrandSignaturePk, StrandSignatureSk},
};

struct Fixture {
    pm: ProtocolManager<RistrettoCtx>,
    logs: Vec<Vec<u8>>,
    manifest: Manifest,
    rows: Vec<CastVote>,
    configuration: Vec<u8>,
    messages: Vec<Vec<u8>>,
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
        let receipt = hash_ballot(&ballot).unwrap();
        let content = serde_json::to_string(&ballot).unwrap();
        let pm = ProtocolManager::<RistrettoCtx>::new(StrandSignatureSk::gen().unwrap());
        let trustees = vec![
            StrandSignaturePk::from_sk(&StrandSignatureSk::gen().unwrap()).unwrap(),
            StrandSignaturePk::from_sk(&StrandSignatureSk::gen().unwrap()).unwrap(),
        ];
        let configuration = Configuration::<RistrettoCtx>::new(
            1,
            StrandSignaturePk::from_sk(&pm.signing_key).unwrap(),
            trustees,
            2,
            PhantomData,
        );
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
        let mut selected_trustees = [NULL_TRUSTEE; MAX_TRUSTEES];
        selected_trustees[0] = 1;
        selected_trustees[1] = 2;
        let messages = batches
            .iter()
            .map(|batch| {
                Message::ballots_msg(
                    &configuration,
                    *batch as usize,
                    &Ballots::new(vec![contests[0].ciphertext.clone(); count]),
                    selected_trustees,
                    PublicKeyHash([7; 64]),
                    &pm,
                )
                .unwrap()
                .strand_serialize()
                .unwrap()
            })
            .collect();
        let manifest = Manifest {
            format_version: 1,
            scope: Scope {
                tenant_id: "tenant".into(),
                election_event_id: "event".into(),
                election_id: "election".into(),
                area_id: "area".into(),
                tally_session_id: "tally".into(),
                tally_session_contest_id: "scope".into(),
                board_name: "board".into(),
                public_key_sha512: hex::encode([7; 64]),
                electoral_log_system_public_key: StrandSignaturePk::from_sk(&pm.signing_key)
                    .unwrap()
                    .to_der_b64_string()
                    .unwrap(),
                contest_id: Some(contests[0].contest_id.clone()),
                base_batch: 10,
                policy,
            },
            batches,
            selected: vec![SelectedVote {
                cast_vote_id: "cast-1".into(),
                ballot_id: receipt.clone(),
                content_sha512: digest(content.as_bytes()).unwrap(),
                electoral_log_message_sha512: String::new(),
                multiplicity,
            }],
        };
        let mut ret = Self {
            pm,
            logs: vec![],
            manifest,
            rows: vec![CastVote {
                voter_id: "synthetic-voter".into(),
                voting_channel: "ONLINE".into(),
                cast_vote_id: "cast-1".into(),
                ballot_id: receipt,
                content,
            }],
            configuration: configuration.strand_serialize().unwrap(),
            messages,
        };
        ret.refresh_log();
        ret
    }
    fn refresh_log(&mut self) {
        use electoral_log::messages::{
            message::{Message as LogMessage, SigningData},
            newtypes::*,
        };
        use sequent_core::encrypt::{hash_ballot_sha512, hash_multi_ballot_sha512};
        let row = &self.rows[0];
        let hash = if self.manifest.scope.contest_id.is_some() {
            hash_ballot_sha512(&serde_json::from_str(&row.content).unwrap()).unwrap()
        } else {
            hash_multi_ballot_sha512(&serde_json::from_str(&row.content).unwrap()).unwrap()
        };
        let sd = SigningData::new(
            self.pm.signing_key.clone(),
            "synthetic",
            self.pm.signing_key.clone(),
        );
        let log = LogMessage::cast_vote_with_channel_message(
            EventIdString(self.manifest.scope.election_event_id.clone()),
            ElectionIdString(Some(self.manifest.scope.election_id.clone())),
            PseudonymHash::new(
                strand::hash::hash_to_array(&row.voter_id.strand_serialize().unwrap()).unwrap(),
            ),
            CastVoteHash::new(hash),
            &sd,
            VoterIpString("".into()),
            VoterCountryString("".into()),
            VotingChannelString(row.voting_channel.clone()),
            Some(row.voter_id.clone()),
            None,
            self.manifest.scope.area_id.clone(),
        )
        .unwrap()
        .strand_serialize()
        .unwrap();
        self.manifest.selected[0].electoral_log_message_sha512 = digest(&log).unwrap();
        self.logs = vec![log];
    }
    fn alter_signed_log(
        &mut self,
        change: impl FnOnce(&mut electoral_log::messages::message::Message),
    ) {
        use electoral_log::messages::message::Message as LogMessage;
        let mut log = LogMessage::strand_deserialize(&self.logs[0]).unwrap();
        change(&mut log);
        let signed = LogMessage::sign(
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
        .unwrap();
        self.logs[0] = signed.strand_serialize().unwrap();
        self.manifest.selected[0].electoral_log_message_sha512 = digest(&self.logs[0]).unwrap();
    }
    fn sign_batch(
        &self,
        batch: usize,
        ciphertexts: Vec<strand::elgamal::Ciphertext<RistrettoCtx>>,
    ) -> Vec<u8> {
        let configuration =
            Configuration::<RistrettoCtx>::strand_deserialize(&self.configuration).unwrap();
        let mut trustees = [NULL_TRUSTEE; MAX_TRUSTEES];
        trustees[0] = 1;
        trustees[1] = 2;
        Message::ballots_msg(
            &configuration,
            batch,
            &Ballots::new(ciphertexts),
            trustees,
            PublicKeyHash([7; 64]),
            &self.pm,
        )
        .unwrap()
        .strand_serialize()
        .unwrap()
    }
    fn run(&self) -> anyhow::Result<ballot_audit::AuditReport> {
        let manifest = serde_json::to_vec(&self.manifest)?;
        audit(
            &manifest,
            &digest(&manifest)?,
            &self.rows,
            &self.configuration,
            &digest(&self.configuration)?,
            &self.messages,
            None,
            &self.logs,
        )
    }
}
#[test]
fn valid_single_ballot_maps_to_input_position() {
    let f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let r = f.run().unwrap();
    assert_eq!(r.checked_cast_votes, 1);
    assert_eq!(r.receipts[0].locations[0].batch, 10);
    assert_eq!(r.receipts[0].locations[0].index, 0);
}
#[test]
fn delegated_duplicates_are_expected() {
    let f = Fixture::new(MultiplicityPolicy::Delegated, 3);
    assert_eq!(f.run().unwrap().receipts[0].locations.len(), 3);
}
#[test]
fn weighted_ballot_maps_to_set_bit_batches() {
    let f = Fixture::new(MultiplicityPolicy::Weighted, 5);
    let r = f.run().unwrap();
    assert_eq!(r.checked_batches, 2);
    assert_eq!(
        r.receipts[0]
            .locations
            .iter()
            .map(|p| p.batch)
            .collect::<Vec<_>>(),
        vec![10, 12]
    );
}
#[test]
fn missing_row_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.rows.clear();
    assert!(f
        .run()
        .unwrap_err()
        .to_string()
        .contains("missing from export"));
}
#[test]
fn duplicate_row_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.rows.push(f.rows[0].clone());
    assert!(f.run().is_err());
}
#[test]
fn missing_batch_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.messages.clear();
    assert!(f
        .run()
        .unwrap_err()
        .to_string()
        .contains("missing B3 batch"));
}
#[test]
fn duplicate_batch_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.messages.push(f.messages[0].clone());
    assert!(f.run().is_err());
}
#[test]
fn full_content_mutation_fails_before_projection() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.rows[0].content.push(' ');
    assert!(f
        .run()
        .unwrap_err()
        .to_string()
        .contains("full content digest mismatch"));
}
#[test]
fn recomputing_content_anchor_does_not_hide_receipt_mutation() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let mut v: serde_json::Value = serde_json::from_str(&f.rows[0].content).unwrap();
    v["issue_date"] = "tampered".into();
    f.rows[0].content = serde_json::to_string(&v).unwrap();
    f.manifest.selected[0].content_sha512 = digest(f.rows[0].content.as_bytes()).unwrap();
    assert!(f
        .run()
        .unwrap_err()
        .to_string()
        .contains("receipt hash mismatch"));
}
#[test]
fn non_receipt_metadata_requires_independent_anchor() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let mut v: serde_json::Value = serde_json::from_str(&f.rows[0].content).unwrap();
    v["config"] = "changed".into();
    f.rows[0].content = serde_json::to_string(&v).unwrap();
    assert!(f.run().is_err());
    f.manifest.selected[0].content_sha512 = digest(f.rows[0].content.as_bytes()).unwrap();
    assert!(
        f.run().is_ok(),
        "A newly computed manifest cannot establish historical metadata integrity"
    );
}
#[test]
fn changed_multiplicity_fails_multiset() {
    let mut f = Fixture::new(MultiplicityPolicy::Delegated, 2);
    f.manifest.selected[0].multiplicity = 1;
    assert!(f
        .run()
        .unwrap_err()
        .to_string()
        .contains("multiset mismatch"));
}
#[test]
fn incomplete_weight_scope_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Weighted, 5);
    f.manifest.batches.pop();
    assert!(f.run().is_err());
}
#[test]
fn manifest_digest_is_required() {
    let f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    assert!(audit(
        &serde_json::to_vec(&f.manifest).unwrap(),
        "wrong",
        &f.rows,
        &f.configuration,
        &digest(&f.configuration).unwrap(),
        &f.messages,
        None,
        &f.logs
    )
    .is_err());
}
#[test]
fn absent_receipt_filter_does_not_pass() {
    let f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let m = serde_json::to_vec(&f.manifest).unwrap();
    assert!(audit(
        &m,
        &digest(&m).unwrap(),
        &f.rows,
        &f.configuration,
        &digest(&f.configuration).unwrap(),
        &f.messages,
        Some("absent"),
        &f.logs
    )
    .is_err());
}

#[test]
fn changed_configuration_anchor_fails() {
    let f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let m = serde_json::to_vec(&f.manifest).unwrap();
    assert!(audit(
        &m,
        &digest(&m).unwrap(),
        &f.rows,
        &f.configuration,
        "wrong",
        &f.messages,
        None,
        &f.logs
    )
    .is_err());
}
#[test]
fn unsigned_artifact_replacement_fails_without_panic() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let mut message = Message::strand_deserialize(&f.messages[0]).unwrap();
    message.artifact = Some(f.configuration.clone());
    f.messages[0] = message.strand_serialize().unwrap();
    assert!(f
        .run()
        .unwrap_err()
        .to_string()
        .contains("artifact hash mismatch"));
}
#[test]
fn wrong_signer_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.pm = ProtocolManager::<RistrettoCtx>::new(StrandSignatureSk::gen().unwrap());
    f.messages[0] = f.sign_batch(10, vec![]);
    assert!(f.run().is_err());
}
#[test]
fn authenticated_empty_batch_with_expected_vote_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.messages[0] = f.sign_batch(10, vec![]);
    assert!(f
        .run()
        .unwrap_err()
        .to_string()
        .contains("multiset mismatch"));
}
#[test]
fn out_of_scope_signed_batch_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.messages[0] = f.sign_batch(11, vec![]);
    assert!(f
        .run()
        .unwrap_err()
        .to_string()
        .contains("outside declared scope"));
}
#[test]
fn empty_selected_scope_requires_authenticated_empty_batch() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.manifest.selected.clear();
    f.rows.clear();
    f.logs.clear();
    f.messages[0] = f.sign_batch(10, vec![]);
    assert_eq!(f.run().unwrap().checked_cast_votes, 0);
    f.messages.clear();
    assert!(f.run().is_err());
}
#[test]
fn authenticated_substitution_is_detected() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let other = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let message = Message::strand_deserialize(&other.messages[0]).unwrap();
    let other_ballots =
        Ballots::<RistrettoCtx>::strand_deserialize(message.artifact.as_ref().unwrap()).unwrap();
    f.messages[0] = f.sign_batch(10, other_ballots.ciphertexts.0);
    assert!(f
        .run()
        .unwrap_err()
        .to_string()
        .contains("multiset mismatch"));
}
#[test]
fn valid_combined_multi_contest_ballot() {
    use sequent_core::{
        encrypt::{encrypt_decoded_multi_contest, hash_multi_ballot},
        multi_ballot::{HashableMultiBallot, SignedHashableMultiBallot},
    };
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let auditable = encrypt_decoded_multi_contest::<RistrettoCtx>(
        &RistrettoCtx,
        &vec![get_writein_plaintext()],
        &get_writein_ballot_style(),
    )
    .unwrap();
    let signed = SignedHashableMultiBallot::try_from(&auditable).unwrap();
    let ballot = HashableMultiBallot::try_from(&signed).unwrap();
    f.rows[0].content = serde_json::to_string(&ballot).unwrap();
    f.rows[0].ballot_id = hash_multi_ballot(&ballot).unwrap();
    f.manifest.selected[0].ballot_id = f.rows[0].ballot_id.clone();
    f.manifest.selected[0].content_sha512 = digest(f.rows[0].content.as_bytes()).unwrap();
    f.manifest.scope.contest_id = None;
    f.refresh_log();
    f.messages[0] = f.sign_batch(
        10,
        vec![
            ballot
                .deserialize_contests::<RistrettoCtx>()
                .unwrap()
                .ciphertext,
        ],
    );
    assert!(f.run().is_ok());
}

/// Opt-in fixture export for demonstrating the real CLI without election data.
#[test]
#[ignore = "writes synthetic demo files only when explicitly requested"]
fn export_synthetic_demo_bundle() {
    use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
    let dir = std::path::PathBuf::from(
        std::env::var("BALLOT_AUDIT_FIXTURE_DIR").expect("set BALLOT_AUDIT_FIXTURE_DIR"),
    );
    std::fs::create_dir_all(&dir).unwrap();
    let f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let manifest = serde_json::to_vec_pretty(&f.manifest).unwrap();
    let write = |name: &str, bytes: Vec<u8>| std::fs::write(dir.join(name), bytes).unwrap();
    write("manifest.json", manifest.clone());
    write("configuration.borsh", f.configuration.clone());
    write(
        "cast-votes.json",
        serde_json::to_vec_pretty(&f.rows).unwrap(),
    );
    write(
        "messages.json",
        serde_json::to_vec_pretty(
            &f.messages
                .iter()
                .map(|m| STANDARD_NO_PAD.encode(m))
                .collect::<Vec<_>>(),
        )
        .unwrap(),
    );
    write(
        "electoral-log.json",
        serde_json::to_vec_pretty(
            &f.logs
                .iter()
                .map(|m| STANDARD_NO_PAD.encode(m))
                .collect::<Vec<_>>(),
        )
        .unwrap(),
    );
    let mut tampered = f.rows.clone();
    tampered[0].content.push(' ');
    write(
        "cast-votes-tampered.json",
        serde_json::to_vec_pretty(&tampered).unwrap(),
    );
    write("messages-missing.json", b"[]\n".to_vec());
    write("anchors.json",serde_json::to_vec_pretty(&serde_json::json!({"manifest_sha512":digest(&manifest).unwrap(),"configuration_sha512":digest(&f.configuration).unwrap(),"ballot_id":f.rows[0].ballot_id,"synthetic":true})).unwrap());
    assert!(f.run().is_ok());
}
#[test]
fn missing_scope_identifier_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.manifest.scope.area_id.clear();
    assert!(f.run().is_err());
}
#[test]
fn unselected_export_row_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let mut row = f.rows[0].clone();
    row.cast_vote_id = "extra".into();
    f.rows.push(row);
    assert!(f.run().unwrap_err().to_string().contains("unselected"));
}
#[test]
fn wrong_pinned_public_key_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.manifest.scope.public_key_sha512 = hex::encode([8; 64]);
    assert!(f.run().unwrap_err().to_string().contains("public key hash"));
}
#[test]
fn unweighted_multiplicity_cannot_be_inflated() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.manifest.selected[0].multiplicity = 2;
    assert!(f.run().is_err());
}
#[test]
fn duplicate_selected_id_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.manifest.selected.push(f.manifest.selected[0].clone());
    assert!(f.run().is_err());
}

#[test]
fn missing_electoral_log_evidence_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.logs.clear();
    assert!(f
        .run()
        .unwrap_err()
        .to_string()
        .contains("electoral-log message missing"));
}
#[test]
fn wrong_voter_link_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.rows[0].voter_id = "different-voter".into();
    assert!(f
        .run()
        .unwrap_err()
        .to_string()
        .contains("pseudonym mismatch"));
}
#[test]
fn wrong_channel_link_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.rows[0].voting_channel = "TELEPHONE".into();
    assert!(f
        .run()
        .unwrap_err()
        .to_string()
        .contains("channel mismatch"));
}
#[test]
fn wrong_event_log_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.alter_signed_log(|log| log.statement.head.event.0 = "other-event".into());
    assert!(f.run().is_err());
}
#[test]
fn wrong_election_log_fails() {
    use electoral_log::messages::statement::StatementBody;
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.alter_signed_log(|log| {
        if let StatementBody::CastVoteWithChannel(e, _, _, _, _, _) = &mut log.statement.body {
            e.0 = Some("other-election".into());
        }
    });
    assert!(f
        .run()
        .unwrap_err()
        .to_string()
        .contains("election mismatch"));
}
#[test]
fn full_log_hash_is_checked_beyond_receipt_prefix() {
    use electoral_log::messages::{newtypes::CastVoteHash, statement::StatementBody};
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.alter_signed_log(|log| {
        if let StatementBody::CastVoteWithChannel(_, _, h, _, _, _) = &mut log.statement.body {
            let mut hash = h.0.clone().into_inner();
            hash[63] ^= 1;
            *h = CastVoteHash::new(hash);
        }
    });
    assert!(f
        .run()
        .unwrap_err()
        .to_string()
        .contains("full ballot hash mismatch"));
}
#[test]
fn wrong_electoral_system_key_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.manifest.scope.electoral_log_system_public_key =
        StrandSignaturePk::from_sk(&StrandSignatureSk::gen().unwrap())
            .unwrap()
            .to_der_b64_string()
            .unwrap();
    assert!(f
        .run()
        .unwrap_err()
        .to_string()
        .contains("signature verification failed"));
}
#[test]
fn duplicate_electoral_log_evidence_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.logs.push(f.logs[0].clone());
    assert!(f.run().is_err());
}
#[test]
fn legacy_cast_vote_channel_defaults_to_online() {
    use electoral_log::messages::statement::StatementBody;
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.alter_signed_log(|log| {
        if let StatementBody::CastVoteWithChannel(e, p, h, ip, country, _) = &log.statement.body {
            log.statement.body = StatementBody::CastVote(
                e.clone(),
                p.clone(),
                h.clone(),
                ip.clone(),
                country.clone(),
            );
        }
    });
    assert_eq!(f.run().unwrap().legacy_online_channel_inferences, 1);
    f.rows[0].voting_channel = "TELEPHONE".into();
    assert!(f.run().is_err());
}
#[test]
fn unsigned_log_outer_metadata_is_not_identity_authority() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.alter_signed_log(|log| {
        log.user_id = Some("not-the-signed-voter".into());
        log.ballot_id = Some("not-the-signed-hash".into());
    });
    assert!(f.run().is_ok());
}

#[test]
fn independently_invalid_sender_or_system_signature_fails() {
    use electoral_log::messages::message::Message as LogMessage;
    for corrupt_sender in [true, false] {
        let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
        let mut log = LogMessage::strand_deserialize(&f.logs[0]).unwrap();
        let invalid = StrandSignatureSk::gen()
            .unwrap()
            .sign(&log.statement.strand_serialize().unwrap())
            .unwrap();
        if corrupt_sender {
            log.sender_signature = invalid;
        } else {
            log.system_signature = invalid;
        }
        f.logs[0] = log.strand_serialize().unwrap();
        f.manifest.selected[0].electoral_log_message_sha512 = digest(&f.logs[0]).unwrap();
        assert!(f
            .run()
            .unwrap_err()
            .to_string()
            .contains("signature verification failed"));
    }
}
#[test]
fn changed_log_digest_anchor_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.manifest.selected[0].electoral_log_message_sha512 = "wrong".into();
    assert!(f.run().is_err());
}
#[test]
fn unselected_electoral_log_message_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let other = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.logs.push(other.logs[0].clone());
    assert!(f
        .run()
        .unwrap_err()
        .to_string()
        .contains("unselected electoral-log evidence"));
}
#[test]
fn signed_log_kind_must_be_cast_vote() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.alter_signed_log(|log| {
        log.statement.head.kind = electoral_log::messages::statement::StatementType::Unknown
    });
    assert!(f.run().is_err());
}
#[test]
fn public_report_does_not_include_voter_identity() {
    let f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let report = f.run().unwrap();
    assert_eq!(report.legacy_online_channel_inferences, 0);
    assert_eq!(report.checked_electoral_log_messages, 1);
    assert!(!serde_json::to_string(&report)
        .unwrap()
        .contains(&f.rows[0].voter_id));
}
