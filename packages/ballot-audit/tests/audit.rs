// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use b3::messages::{
    artifact::{Ballots, Configuration},
    message::Message,
    newtypes::{PublicKeyHash, MAX_TRUSTEES, NULL_TRUSTEE},
    protocol_manager::ProtocolManager,
    statement::Statement,
};
use ballot_audit::{
    audit, audit_with_selection, digest, AuditReport, CastVote, ExitStatus, Manifest,
    MultiplicityPolicy, ReceiptLookup, Scope, SelectedVote, Selection, SelectionReason,
    SelectionRow, VoteToAuditReason,
};
use sequent_core::{
    ballot::{HashableBallot, SignedHashableBallot},
    encrypt::{encrypt_decoded_contest, hash_ballot},
    fixtures::ballot_codec::{get_writein_ballot_style, get_writein_plaintext},
    types::keycloak::MIN_WEIGHT_BATCH_ANONYMITY,
};
use std::{collections::BTreeMap, marker::PhantomData};
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
fn encrypted_ballot() -> HashableBallot {
    let ballot = encrypt_decoded_contest::<RistrettoCtx>(
        &RistrettoCtx,
        &vec![get_writein_plaintext()],
        &get_writein_ballot_style(),
    )
    .unwrap();
    let signed = SignedHashableBallot::try_from(&ballot).unwrap();
    HashableBallot::try_from(&signed).unwrap()
}

impl Fixture {
    fn new(policy: MultiplicityPolicy, multiplicity: u64) -> Self {
        let ballot = encrypted_ballot();
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
                electoral_log_message_sha512: None,
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
        self.logs = (0..self.rows.len()).map(|i| self.signed_log(i)).collect();
        for (selected, log) in self.manifest.selected.iter_mut().zip(&self.logs) {
            selected.electoral_log_message_sha512 = Some(digest(log).unwrap());
        }
    }
    fn signed_log(&self, index: usize) -> Vec<u8> {
        use electoral_log::messages::{
            message::{Message as LogMessage, SigningData},
            newtypes::*,
        };
        use sequent_core::encrypt::{hash_ballot_sha512, hash_multi_ballot_sha512};
        let row = &self.rows[index];
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
        LogMessage::cast_vote_with_channel_message(
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
        .unwrap()
    }
    /// Adds a selected vote and re-signs every log and batch it affects.
    fn add_vote(&mut self, voter: &str, content: String, multiplicity: u64) -> String {
        let ballot: HashableBallot = serde_json::from_str(&content).unwrap();
        let receipt = hash_ballot(&ballot).unwrap();
        let cast_vote_id = format!("cast-{}", self.rows.len() + 1);
        self.manifest.selected.push(SelectedVote {
            cast_vote_id: cast_vote_id.clone(),
            ballot_id: receipt.clone(),
            content_sha512: digest(content.as_bytes()).unwrap(),
            electoral_log_message_sha512: None,
            multiplicity,
        });
        self.rows.push(CastVote {
            voter_id: voter.into(),
            voting_channel: "ONLINE".into(),
            cast_vote_id,
            ballot_id: receipt.clone(),
            content,
        });
        self.refresh_log();
        self.rebuild_batches();
        receipt
    }
    fn ciphertext(&self, index: usize) -> strand::elgamal::Ciphertext<RistrettoCtx> {
        let ballot: HashableBallot = serde_json::from_str(&self.rows[index].content).unwrap();
        let contest_id = self.manifest.scope.contest_id.clone().unwrap();
        ballot
            .deserialize_contests::<RistrettoCtx>()
            .unwrap()
            .into_iter()
            .find(|contest| contest.contest_id == contest_id)
            .unwrap()
            .ciphertext
    }
    fn rebuild_batches(&mut self) {
        let mut batches: BTreeMap<u32, Vec<_>> = BTreeMap::new();
        for (index, selected) in self.manifest.selected.iter().enumerate() {
            let ciphertext = self.ciphertext(index);
            let base = self.manifest.scope.base_batch;
            match self.manifest.scope.policy {
                MultiplicityPolicy::Weighted => {
                    for bit in (0..17).filter(|bit| selected.multiplicity & (1 << bit) != 0) {
                        batches
                            .entry(base + bit)
                            .or_default()
                            .push(ciphertext.clone());
                    }
                }
                MultiplicityPolicy::Delegated => batches
                    .entry(base)
                    .or_default()
                    .extend(vec![ciphertext; selected.multiplicity as usize]),
                MultiplicityPolicy::Unweighted => batches.entry(base).or_default().push(ciphertext),
            }
        }
        self.manifest.batches = batches.keys().copied().collect();
        self.messages = batches
            .into_iter()
            .map(|(batch, ciphertexts)| self.sign_batch(batch as usize, ciphertexts))
            .collect();
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
        self.manifest.selected[0].electoral_log_message_sha512 =
            Some(digest(&self.logs[0]).unwrap());
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
    fn run(&self) -> anyhow::Result<AuditReport> {
        self.lookup(None, None)
    }
    fn lookup(
        &self,
        ballot_id: Option<&str>,
        selection: Option<&Selection>,
    ) -> anyhow::Result<AuditReport> {
        let manifest = serde_json::to_vec(&self.manifest)?;
        audit_with_selection(
            &manifest,
            &digest(&manifest)?,
            &self.rows,
            &self.configuration,
            &digest(&self.configuration)?,
            &self.messages,
            ballot_id,
            &self.logs,
            selection,
        )
    }
    fn selection(&self, excluded: &[(&str, &str, SelectionReason)]) -> Selection {
        Selection {
            format_version: 1,
            rows: self
                .manifest
                .selected
                .iter()
                .map(|selected| SelectionRow {
                    cast_vote_id: selected.cast_vote_id.clone(),
                    ballot_id: Some(selected.ballot_id.clone()),
                    reason: SelectionReason::Selected,
                    latest_scoped_cast_vote_id: Some(selected.cast_vote_id.clone()),
                })
                .chain(
                    excluded
                        .iter()
                        .map(|(cast_vote_id, ballot_id, reason)| SelectionRow {
                            cast_vote_id: (*cast_vote_id).into(),
                            ballot_id: Some((*ballot_id).into()),
                            reason: *reason,
                            latest_scoped_cast_vote_id: None,
                        }),
                )
                .collect(),
        }
    }
}
fn status(result: anyhow::Result<AuditReport>) -> ExitStatus {
    match result {
        Ok(report) => report.status(),
        Err(error) => ExitStatus::of(&error),
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
fn fields_outside_receipt_hash_are_protected_only_by_an_independent_manifest_pin() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let mut v: serde_json::Value = serde_json::from_str(&f.rows[0].content).unwrap();
    v["config"] = "changed".into();
    f.rows[0].content = serde_json::to_string(&v).unwrap();
    assert!(f.run().is_err());
    f.manifest.selected[0].content_sha512 = digest(f.rows[0].content.as_bytes()).unwrap();
    assert!(
        f.run().is_ok(),
        "a regenerated manifest and its own anchors accept a changed field outside the receipt hash; only a manifest pin obtained outside the bundle detects it"
    );
}
#[test]
fn regenerated_bundle_cannot_hide_ciphertext_or_receipt_tampering() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let substitute = encrypted_ballot();
    f.rows[0].content = serde_json::to_string(&substitute).unwrap();
    f.rows[0].ballot_id = hash_ballot(&substitute).unwrap();
    f.manifest.selected[0].ballot_id = f.rows[0].ballot_id.clone();
    f.manifest.selected[0].content_sha512 = digest(f.rows[0].content.as_bytes()).unwrap();
    // run() recomputes the manifest and configuration anchors from the bundle.
    let error = f.run().unwrap_err();
    assert_eq!(error.to_string(), "electoral-log full ballot hash mismatch");
    assert_eq!(ExitStatus::of(&error), ExitStatus::VerificationFailed);
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let ciphertexts = Ballots::new(vec![substitute
        .deserialize_contests::<RistrettoCtx>()
        .unwrap()[0]
        .ciphertext
        .clone()])
    .strand_serialize()
    .unwrap();
    let mut message = Message::strand_deserialize(&f.messages[0]).unwrap();
    if let Statement::Ballots(_, _, _, hash, _, _) = &mut message.statement {
        hash.0 = strand::hash::hash_to_array(&ciphertexts).unwrap();
    }
    message.artifact = Some(ciphertexts);
    f.messages[0] = message.strand_serialize().unwrap();
    let error = f.run().unwrap_err();
    assert!(error
        .to_string()
        .contains("B3 signature/configuration verification failed"));
    assert_eq!(ExitStatus::of(&error), ExitStatus::VerificationFailed);
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
fn selected_receipt_lookup_reports_locations() {
    let f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let report = f.lookup(Some(&f.rows[0].ballot_id), None).unwrap();
    assert_eq!(report.status(), ExitStatus::Success);
    let Some(ReceiptLookup::Selected {
        cast_vote_id,
        locations,
        ..
    }) = &report.lookup
    else {
        panic!("expected a selected lookup");
    };
    assert_eq!(cast_vote_id, "cast-1");
    assert_eq!((locations[0].batch, locations[0].index), (10, 0));
    assert_eq!(
        serde_json::to_value(&report).unwrap()["lookup"]["outcome"],
        "selected"
    );
}
#[test]
fn missing_receipt_lookup_still_audits_and_is_incomplete() {
    let f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let report = f.lookup(Some("absent"), None).unwrap();
    assert_eq!(report.checked_cast_votes, 1);
    assert!(report.receipts.is_empty());
    assert!(matches!(
        report.lookup,
        Some(ReceiptLookup::Missing {
            selection_searched: false,
            ..
        })
    ));
    assert_eq!(report.status(), ExitStatus::Incomplete);
    let selection = f.selection(&[]);
    let report = f.lookup(Some("absent"), Some(&selection)).unwrap();
    assert!(matches!(
        report.lookup,
        Some(ReceiptLookup::Missing {
            selection_searched: true,
            ..
        })
    ));
}
#[test]
fn excluded_receipt_lookup_is_explained_from_unverified_selection_labels() {
    let f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let selection = f.selection(&[("cast-discarded", "stored-label", SelectionReason::Discarded)]);
    let report = f.lookup(Some("stored-label"), Some(&selection)).unwrap();
    assert_eq!(report.status(), ExitStatus::Incomplete);
    let Some(ReceiptLookup::Excluded {
        rows,
        stored_labels,
        ..
    }) = &report.lookup
    else {
        panic!("expected an excluded lookup");
    };
    assert_eq!(rows[0].cast_vote_id, "cast-discarded");
    assert_eq!(rows[0].reason, SelectionReason::Discarded);
    assert!(stored_labels.contains("unverified stored labels"));
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["lookup"]["outcome"], "excluded");
    assert_eq!(json["lookup"]["rows"][0]["reason"], "discarded");
}
#[test]
fn selection_must_describe_the_manifest() {
    let f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let mut selection = f.selection(&[]);
    selection.rows[0].cast_vote_id = "other".into();
    let error = f.lookup(Some("absent"), Some(&selection)).unwrap_err();
    assert_eq!(ExitStatus::of(&error), ExitStatus::InvalidInput);
    let mut selection = f.selection(&[]);
    selection.format_version = 2;
    assert_eq!(
        status(f.lookup(None, Some(&selection))),
        ExitStatus::InvalidInput
    );
}
#[test]
fn receipt_lookup_never_hides_tampering() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let receipt = f.rows[0].ballot_id.clone();
    f.messages[0] = f.sign_batch(10, vec![]);
    let error = f.lookup(Some(&receipt), None).unwrap_err();
    assert_eq!(ExitStatus::of(&error), ExitStatus::VerificationFailed);
}
#[test]
fn identical_ballot_across_voters_is_reported_as_duplicate_receipt() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let content = f.rows[0].content.clone();
    let receipt = f.add_vote("second-voter", content, 1);
    let report = f.run().unwrap();
    assert_eq!(report.checked_cast_votes, 2);
    assert_eq!(report.duplicate_selected_receipts.len(), 1);
    assert_eq!(
        report.duplicate_selected_receipts[0].cast_vote_ids,
        vec!["cast-1", "cast-2"]
    );
    assert_eq!(report.receipts[0].locations.len(), 2);
    assert_eq!(report.status(), ExitStatus::VerificationFailed);
    let report = f.lookup(Some(&receipt), None).unwrap();
    assert!(matches!(
        &report.lookup,
        Some(ReceiptLookup::Ambiguous { selected_cast_vote_ids, .. })
            if selected_cast_vote_ids.len() == 2
    ));
    assert_eq!(report.status(), ExitStatus::VerificationFailed);
}
#[test]
fn identical_ciphertext_under_distinct_receipts_lists_every_position() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let mut v: serde_json::Value = serde_json::from_str(&f.rows[0].content).unwrap();
    v["issue_date"] = "another-issue-date".into();
    let receipt = f.add_vote("second-voter", serde_json::to_string(&v).unwrap(), 1);
    assert_ne!(receipt, f.rows[0].ballot_id);
    let report = f.run().unwrap();
    assert!(report.duplicate_selected_receipts.is_empty());
    assert_eq!(report.status(), ExitStatus::Success);
    for receipt in &report.receipts {
        assert_eq!(receipt.locations.len(), 2);
    }
}
#[test]
fn multi_voter_selection_maps_each_receipt() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.add_vote(
        "second-voter",
        serde_json::to_string(&encrypted_ballot()).unwrap(),
        1,
    );
    f.add_vote(
        "third-voter",
        serde_json::to_string(&encrypted_ballot()).unwrap(),
        1,
    );
    let report = f.run().unwrap();
    assert_eq!(report.checked_cast_votes, 3);
    assert_eq!(report.checked_electoral_log_messages, 3);
    let mut indexes: Vec<_> = report
        .receipts
        .iter()
        .map(|receipt| {
            assert_eq!(receipt.locations.len(), 1);
            receipt.locations[0].index
        })
        .collect();
    indexes.sort_unstable();
    assert_eq!(indexes, vec![0, 1, 2]);
    let kept = vec![f.ciphertext(0), f.ciphertext(1)];
    f.messages[0] = f.sign_batch(10, kept);
    assert!(f
        .run()
        .unwrap_err()
        .to_string()
        .contains("multiset mismatch"));
}
#[test]
fn weighted_ciphertext_in_the_wrong_bit_batch_fails() {
    let mut f = Fixture::new(MultiplicityPolicy::Weighted, 1);
    f.add_vote(
        "heavy-voter",
        serde_json::to_string(&encrypted_ballot()).unwrap(),
        4,
    );
    assert_eq!(f.manifest.batches, vec![10, 12]);
    assert!(f.run().is_ok());
    let (light, heavy) = (f.ciphertext(0), f.ciphertext(1));
    f.messages = vec![f.sign_batch(10, vec![heavy]), f.sign_batch(12, vec![light])];
    let error = f.run().unwrap_err();
    assert!(error.to_string().contains("multiset mismatch"));
    assert_eq!(ExitStatus::of(&error), ExitStatus::VerificationFailed);
}
#[test]
fn small_weighted_batches_carry_count_only_privacy_warnings() {
    let f = Fixture::new(MultiplicityPolicy::Weighted, 5);
    let report = f.run().unwrap();
    let warned: Vec<_> = report
        .privacy_warnings
        .iter()
        .map(|warning| (warning.batch, warning.ciphertexts))
        .collect();
    assert_eq!(warned, vec![(10, 1), (12, 1)]);
    assert_eq!(
        report.privacy_warnings[0].minimum_for_anonymity,
        MIN_WEIGHT_BATCH_ANONYMITY
    );
    assert_eq!(report.status(), ExitStatus::Success);
    let json = serde_json::to_string(&report).unwrap();
    assert!(json.contains("individually attributable"));
    assert!(!json.contains(&f.rows[0].voter_id));
    assert!(Fixture::new(MultiplicityPolicy::Unweighted, 1)
        .run()
        .unwrap()
        .privacy_warnings
        .is_empty());
    assert!(Fixture::new(MultiplicityPolicy::Delegated, 2)
        .run()
        .unwrap()
        .privacy_warnings
        .is_empty());
}
#[test]
fn weighted_batch_at_anonymity_threshold_is_not_warned() {
    let mut f = Fixture::new(MultiplicityPolicy::Weighted, 1);
    for voter in 1..MIN_WEIGHT_BATCH_ANONYMITY {
        f.add_vote(
            &format!("voter-{voter}"),
            serde_json::to_string(&encrypted_ballot()).unwrap(),
            1,
        );
    }
    assert!(f.run().unwrap().privacy_warnings.is_empty());
}
#[test]
fn errors_are_classified_by_exit_status() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.logs.clear();
    assert_eq!(status(f.run()), ExitStatus::Incomplete);
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.rows.clear();
    assert_eq!(status(f.run()), ExitStatus::Incomplete);
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.messages.clear();
    assert_eq!(status(f.run()), ExitStatus::Incomplete);
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.manifest.format_version = 2;
    assert_eq!(status(f.run()), ExitStatus::InvalidInput);
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let mut v: serde_json::Value = serde_json::from_str(&f.rows[0].content).unwrap();
    v["version"] = 0.into();
    f.rows[0].content = serde_json::to_string(&v).unwrap();
    f.manifest.selected[0].content_sha512 = digest(f.rows[0].content.as_bytes()).unwrap();
    assert_eq!(status(f.run()), ExitStatus::InvalidInput);
    let f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    let invalid = b"not a manifest".to_vec();
    assert_eq!(
        status(audit(
            &invalid,
            &digest(&invalid).unwrap(),
            &f.rows,
            &f.configuration,
            &digest(&f.configuration).unwrap(),
            &f.messages,
            None,
            &f.logs
        )),
        ExitStatus::InvalidInput
    );
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.rows[0].content.push(' ');
    assert_eq!(status(f.run()), ExitStatus::VerificationFailed);
    assert_eq!(
        status(Fixture::new(MultiplicityPolicy::Unweighted, 1).run()),
        ExitStatus::Success
    );
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
        f.manifest.selected[0].electoral_log_message_sha512 = Some(digest(&f.logs[0]).unwrap());
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
    f.manifest.selected[0].electoral_log_message_sha512 = Some("wrong".into());
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
/// Two selected votes; the second has no signed CastVote record.
fn fixture_with_vote_to_audit() -> Fixture {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.add_vote(
        "second-voter",
        serde_json::to_string(&encrypted_ballot()).unwrap(),
        1,
    );
    f.logs.pop();
    f.manifest.selected[1].electoral_log_message_sha512 = None;
    f
}
#[test]
fn vote_without_signed_log_is_checked_and_listed_to_audit() {
    let f = fixture_with_vote_to_audit();
    let report = f.run().unwrap();
    assert_eq!(report.status(), ExitStatus::Success);
    assert_eq!(report.checked_cast_votes, 2);
    assert_eq!(report.checked_electoral_log_messages, 1);
    assert_eq!(report.receipts.len(), 2);
    assert_eq!(report.votes_to_audit.len(), 1);
    let vote = &report.votes_to_audit[0];
    assert_eq!(vote.cast_vote_id, "cast-2");
    assert_eq!(vote.ballot_id, f.rows[1].ballot_id);
    assert_eq!(vote.reason, VoteToAuditReason::NoSignedCastVoteRecord);
}
#[test]
fn vote_to_audit_must_still_be_in_the_tally_input() {
    let mut f = fixture_with_vote_to_audit();
    f.messages = vec![f.sign_batch(10, vec![f.ciphertext(0)])];
    assert_eq!(status(f.run()), ExitStatus::VerificationFailed);
    let mut f = fixture_with_vote_to_audit();
    f.messages = vec![f.sign_batch(10, vec![f.ciphertext(0), f.ciphertext(0)])];
    assert_eq!(status(f.run()), ExitStatus::VerificationFailed);
}
#[test]
fn vote_to_audit_must_still_match_its_receipt_and_content() {
    let mut f = fixture_with_vote_to_audit();
    f.rows[1].content.push(' ');
    assert_eq!(status(f.run()), ExitStatus::VerificationFailed);
    let mut f = fixture_with_vote_to_audit();
    f.rows[1].ballot_id = f.rows[0].ballot_id.clone();
    f.manifest.selected[1].ballot_id = f.rows[0].ballot_id.clone();
    assert_eq!(status(f.run()), ExitStatus::VerificationFailed);
}
#[test]
fn named_log_record_that_is_missing_stays_incomplete() {
    let mut f = fixture_with_vote_to_audit();
    f.logs.clear();
    assert_eq!(status(f.run()), ExitStatus::Incomplete);
}
#[test]
fn every_selected_vote_without_a_log_is_listed() {
    let mut f = Fixture::new(MultiplicityPolicy::Unweighted, 1);
    f.logs.clear();
    f.manifest.selected[0].electoral_log_message_sha512 = None;
    let report = f.run().unwrap();
    assert_eq!(report.status(), ExitStatus::Success);
    assert_eq!(report.checked_electoral_log_messages, 0);
    assert_eq!(report.votes_to_audit.len(), 1);
    assert!(!serde_json::to_string(&report)
        .unwrap()
        .contains(&f.rows[0].voter_id));
}
