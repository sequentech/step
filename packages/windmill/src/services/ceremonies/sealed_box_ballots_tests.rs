// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::services::cast_votes::TALLY_AUDIT_COLUMNS;
use crate::services::join::{merge_join_csv, MultiplicitySource};
use electoral_log::seal::{SealEntry, SEAL_FORMAT_V1};
use serde_json::json;
use std::collections::BTreeMap;
use std::io::Write;
use tempfile::NamedTempFile;

/// How the census weighs a voter, as the event's policies decide.
#[derive(Clone, Copy, Debug)]
enum Census {
    Plain,
    Delegated,
    Weighted,
}

impl Census {
    fn source(self) -> Option<MultiplicitySource> {
        match self {
            Census::Plain => None,
            Census::Delegated => Some(MultiplicitySource::DelegateCount(1)),
            Census::Weighted => Some(MultiplicitySource::VoteWeight(1)),
        }
    }

    /// The manifest weight of a counted voter with census value `value`.
    fn weight(self, value: u64) -> u64 {
        match self {
            Census::Plain => 1,
            Census::Delegated => 1 + value,
            Census::Weighted => value,
        }
    }

    fn policy(self) -> WeightedVotingPolicy {
        match self {
            Census::Weighted => WeightedVotingPolicy::VOTERS_WEIGHTED_VOTING,
            _ => WeightedVotingPolicy::default(),
        }
    }
}

/// The latest valid ballot of a voter: (voter id, content, channel).
type Ballot = (&'static str, &'static str, &'static str);

/// A stored ballot of the box.
#[derive(Clone, Copy, Debug)]
struct Row {
    voter: &'static str,
    content: &'static str,
    channel: &'static str,
    /// Acceptance order.
    seq: i64,
    valid: bool,
}

fn ballot_id(content: &str) -> String {
    format!("id-{content}")
}

fn cast_vote_id(content: &str) -> String {
    format!("cv-{content}")
}

/// The stored ballots of a box whose voters' latest valid ballots are
/// `ballots`; the first voter also has a ballot replaced by a revote and a
/// discarded one, which never count since the voter has a valid ballot.
fn rows(ballots: &[Ballot]) -> Vec<Row> {
    let mut rows: Vec<Row> = ballots
        .iter()
        .enumerate()
        .map(|(seq, (voter, content, channel))| Row {
            voter,
            content,
            channel,
            seq: seq as i64,
            valid: true,
        })
        .collect();
    if let Some((voter, _, _)) = ballots.first() {
        rows.push(Row {
            voter,
            content: "replaced",
            channel: "ONLINE",
            seq: -2,
            valid: true,
        });
        rows.push(Row {
            voter,
            content: "discarded",
            channel: "KIOSK",
            seq: -1,
            valid: false,
        });
    }
    rows
}

fn csv_file(rows: &[Vec<String>]) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    for row in rows {
        writeln!(file, "{}", row.join(",")).unwrap();
    }
    file.flush().unwrap();
    file
}

/// The tally input the unsealed path reads (`tally_ballots_query`): each
/// voter's latest valid ballot, or else the voter's latest discarded one.
fn tally_input(rows: &[Row]) -> Vec<Row> {
    let mut latest: BTreeMap<&str, Row> = BTreeMap::new();
    for row in rows {
        let replace = match latest.get(row.voter) {
            None => true,
            Some(kept) => (row.valid, row.seq) > (kept.valid, kept.seq),
        };
        if replace {
            latest.insert(row.voter, *row);
        }
    }
    latest.into_values().collect()
}

/// What the unsealed path computes from the stored ballots and the census,
/// with the auditable ballots it reports.
fn joined_rows(
    rows: &[Row],
    voters: &[(&str, u64)],
    census: Census,
) -> (MergeJoinResult, Vec<AuditableBallot>) {
    let mut voters = voters.to_vec();
    voters.sort();
    let ballots_file = csv_file(
        &tally_input(rows)
            .iter()
            .map(|row| {
                vec![
                    row.voter.to_string(),
                    row.content.to_string(),
                    row.channel.to_string(),
                    if row.valid { "valid" } else { "discarded" }.to_string(),
                    cast_vote_id(row.content),
                    ballot_id(row.content),
                ]
            })
            .collect::<Vec<_>>(),
    );
    let voters_file = csv_file(
        &voters
            .iter()
            .map(|(voter, value)| match census {
                Census::Plain => vec![voter.to_string()],
                _ => vec![voter.to_string(), value.to_string()],
            })
            .collect::<Vec<_>>(),
    );
    let mut audited = Vec::new();
    let result = merge_join_csv(
        &ballots_file.reopen().unwrap(),
        &voters_file.reopen().unwrap(),
        0,
        0,
        1,
        Some(2),
        census.source(),
        Some(TALLY_AUDIT_COLUMNS),
        |ballot| {
            audited.push(ballot);
            Ok(())
        },
    )
    .unwrap();
    (result, audited)
}

/// What today's path computes from the ballots and the census.
fn joined(ballots: &[Ballot], voters: &[(&str, u64)], census: Census) -> MergeJoinResult {
    joined_rows(&rows(ballots), voters, census).0
}

/// A sealed box, as the checks leave it for the count.
struct Sealed {
    manifest: BallotBoxSealManifest,
    contents: HashMap<BallotKey, String>,
    ballots: HashMap<BallotKey, StoredBallot>,
}

impl Sealed {
    fn count(
        &self,
        policy: WeightedVotingPolicy,
    ) -> Result<(MergeJoinResult, Vec<AuditableBallot>)> {
        let mut audited = Vec::new();
        let result = count_manifest(
            &self.manifest,
            self.contents.clone(),
            &self.ballots,
            policy,
            |ballot| {
                audited.push(ballot);
                Ok(())
            },
        )?;
        Ok((result, audited))
    }

    fn counted(&self, policy: WeightedVotingPolicy) -> MergeJoinResult {
        self.count(policy).unwrap().0
    }
}

fn entry(content: &str, disposition: SealDisposition, weight: u64, channel: &str) -> SealEntry {
    SealEntry {
        ballot_hash: ballot_hash(content).unwrap(),
        ballot_id: ballot_id(content),
        disposition,
        weight,
        channel: channel.to_string(),
    }
}

/// The manifest the sealer writes for a box with the stored ballots `rows`
/// (as `seal::entries` decides), with what the checks read of the box.
fn sealed_rows(rows: &[Row], voters: &[(&str, u64)], census: Census) -> Sealed {
    let mut latest: HashMap<&str, i64> = HashMap::new();
    for row in rows.iter().filter(|row| row.valid) {
        let seq = latest.entry(row.voter).or_insert(row.seq);
        *seq = (*seq).max(row.seq);
    }
    let entries: Vec<SealEntry> = rows
        .iter()
        .map(|row| {
            if !row.valid {
                return entry(row.content, SealDisposition::Discarded, 0, row.channel);
            }
            if latest[row.voter] != row.seq {
                return entry(row.content, SealDisposition::Replaced, 0, row.channel);
            }
            match voters.iter().find(|(id, _)| *id == row.voter) {
                Some((_, value)) => entry(
                    row.content,
                    SealDisposition::Counted,
                    census.weight(*value),
                    row.channel,
                ),
                None => entry(row.content, SealDisposition::NotEligible, 0, row.channel),
            }
        })
        .collect();
    let key = |row: &Row| (ballot_hash(row.content).unwrap(), ballot_id(row.content));
    let manifest = electoral_log::seal::build(BallotBoxSealManifest {
        format: SEAL_FORMAT_V1.into(),
        tenant_id: "tenant".into(),
        election_event_id: "event".into(),
        election_id: "election".into(),
        area_id: "area".into(),
        closed_at: 1,
        grace_deadline: 2,
        sealed_at: 3,
        close_request_id: None,
        eligible_voters: voters.len() as u64,
        entries,
    })
    .unwrap()
    .manifest;
    // The checks keep the content of every ballot that isn't replaced.
    let contents = rows
        .iter()
        .filter(|row| !row.valid || latest[row.voter] == row.seq)
        .map(|row| (key(row), row.content.to_string()))
        .collect();
    let ballots = rows
        .iter()
        .map(|row| {
            (
                key(row),
                StoredBallot {
                    id: cast_vote_id(row.content),
                    seq: row.seq,
                    voter_id: row.voter.to_string(),
                },
            )
        })
        .collect();
    Sealed {
        manifest,
        contents,
        ballots,
    }
}

/// The manifest the sealer writes for the same box, plus ballots the old
/// path never reads (replaced and discarded).
fn sealed(ballots: &[Ballot], voters: &[(&str, u64)], census: Census) -> Sealed {
    sealed_rows(&rows(ballots), voters, census)
}

fn sorted(mut result: MergeJoinResult) -> MergeJoinResult {
    result.ballot_contents.sort();
    result
}

fn sorted_audit(mut audited: Vec<AuditableBallot>) -> Vec<AuditableBallot> {
    audited.sort_by(|a, b| a.ballot_id.cmp(&b.ballot_id));
    audited
}

const BALLOTS: [Ballot; 4] = [
    ("alice", "ballot-a", "ONLINE"),
    ("bob", "ballot-b", "KIOSK"),
    ("carol", "ballot-c", "ONLINE"),
    // Not in the census: counted as a ballot without voter.
    ("zed", "ballot-z", "TELEPHONE"),
];
const VOTERS: [(&str, u64); 4] = [("alice", 2), ("bob", 3), ("carol", 1), ("dave", 4)];

#[test]
fn the_manifest_counts_what_the_merge_join_counts_under_every_census() {
    for census in [Census::Plain, Census::Delegated, Census::Weighted] {
        let sealed = sealed(&BALLOTS, &VOTERS, census);
        let (counted, audited) = sealed.count(census.policy()).unwrap();
        let (merged, merge_audited) = joined_rows(&rows(&BALLOTS), &VOTERS, census);
        assert_eq!(sorted(counted), sorted(merged), "{census:?}");
        assert_eq!(
            sorted_audit(audited),
            sorted_audit(merge_audited),
            "{census:?}"
        );
    }
}

#[test]
fn an_empty_box_counts_only_its_census() {
    let sealed = sealed(&[], &VOTERS, Census::Plain);
    let (result, audited) = sealed.count(WeightedVotingPolicy::default()).unwrap();
    assert_eq!(result, joined(&[], &VOTERS, Census::Plain));
    assert_eq!(result.eligible_voters, 4);
    assert!(result.ballot_contents.is_empty());
    assert!(audited.is_empty());
}

#[test]
fn a_voter_disabled_after_the_seal_does_not_change_the_counts() {
    let sealed = sealed(&BALLOTS, &VOTERS, Census::Plain);
    let at_seal = sorted(joined(&BALLOTS, &VOTERS, Census::Plain));
    // Bob is disabled after the seal: today's path would read the new
    // census and count his ballot as without voter.
    let after: Vec<(&str, u64)> = VOTERS
        .iter()
        .copied()
        .filter(|(voter, _)| *voter != "bob")
        .collect();
    assert_ne!(sorted(joined(&BALLOTS, &after, Census::Plain)), at_seal);
    // The sealed path counts the manifest, which still holds the census at
    // the seal.
    assert_eq!(
        sorted(sealed.counted(WeightedVotingPolicy::default())),
        at_seal
    );
}

#[test]
fn replaced_ballots_and_discarded_ones_of_a_voter_with_a_valid_ballot_do_not_count() {
    let sealed = sealed(
        &[("alice", "ballot-a", "ONLINE")],
        &[("alice", 1)],
        Census::Plain,
    );
    let (result, audited) = sealed.count(WeightedVotingPolicy::default()).unwrap();
    assert_eq!(sealed.manifest.ballots_in_box(), 3);
    assert_eq!(result.ballot_contents, vec![("ballot-a".to_string(), 1)]);
    assert_eq!(result.casted_ballots, 1);
    assert_eq!(result.ballots_without_voter, 0);
    assert_eq!(
        result.casted_ballots_by_channel,
        [(ParticipationChannel::from("ONLINE"), 1)]
            .into_iter()
            .collect()
    );
    assert!(audited.is_empty());
}

/// Discarded ballots count as auditable as the unsealed tally counts them:
/// a voter's latest discarded ballot, when the voter has no valid ballot in
/// the box, even if the voter is in the census.
#[test]
fn discarded_ballots_are_audited_as_the_merge_join_audits_them() {
    let row = |voter, content, channel, seq, valid| Row {
        voter,
        content,
        channel,
        seq,
        valid,
    };
    let box_rows = [
        row("alice", "alice-valid", "ONLINE", 1, true),
        // Has a valid ballot, so the discarded one is not audited.
        row("bob", "bob-discarded", "KIOSK", 2, false),
        row("bob", "bob-valid", "KIOSK", 3, true),
        // Only discarded ballots: the latest is audited.
        row("dan", "dan-old-discarded", "ONLINE", 4, false),
        row("dan", "dan-discarded", "TELEPHONE", 5, false),
        // Discarded and enabled again: audited, and still an eligible voter.
        row("erin", "erin-discarded", "ONLINE", 6, false),
        // Not in the census.
        row("zed", "zed-valid", "KIOSK", 7, true),
    ];
    let voters = [("alice", 1), ("bob", 1), ("erin", 1)];
    let sealed = sealed_rows(&box_rows, &voters, Census::Plain);
    let (counted, audited) = sealed.count(WeightedVotingPolicy::default()).unwrap();
    let (merged, merge_audited) = joined_rows(&box_rows, &voters, Census::Plain);
    assert_eq!(counted.ballots_without_voter, 3);
    assert_eq!(counted.casted_ballots, 5);
    assert_eq!(counted.eligible_voters, 3);
    assert_eq!(sorted(counted), sorted(merged));
    let audited = sorted_audit(audited);
    assert_eq!(audited, sorted_audit(merge_audited));

    assert_eq!(
        audited
            .iter()
            .map(|ballot| (
                ballot.reason,
                ballot.voter_id.as_str(),
                ballot.content.as_str(),
                ballot.cast_vote_id.as_deref(),
                ballot.ballot_id.as_deref()
            ))
            .collect::<Vec<_>>(),
        [
            (
                AuditableBallotReason::Discarded,
                "dan",
                "dan-discarded",
                Some("cv-dan-discarded"),
                Some("id-dan-discarded")
            ),
            (
                AuditableBallotReason::Discarded,
                "erin",
                "erin-discarded",
                Some("cv-erin-discarded"),
                Some("id-erin-discarded")
            ),
            (
                AuditableBallotReason::NoEligibleVoter,
                "zed",
                "zed-valid",
                Some("cv-zed-valid"),
                Some("id-zed-valid")
            ),
        ]
    );
}

#[test]
fn a_discarded_entry_without_its_stored_ballot_fails_the_count() {
    let mut sealed = sealed(&BALLOTS, &VOTERS, Census::Plain);
    sealed
        .ballots
        .retain(|(_, ballot_id), _| ballot_id != "id-discarded");
    let error = sealed
        .count(WeightedVotingPolicy::default())
        .unwrap_err()
        .to_string();
    assert!(error.contains("id-discarded"), "{error}");
}

fn event(presentation: serde_json::Value) -> ElectionEvent {
    serde_json::from_value(json!({
        "id": "event", "tenant_id": "tenant", "is_archived": false,
        "encryption_protocol": "RistrettoCtx", "presentation": presentation,
    }))
    .unwrap()
}

#[test]
fn only_seal_at_close_turns_the_seal_check_on() {
    for (presentation, expected) in [
        (
            json!({"ballot_box_seal_policy": "seal-at-close"}),
            BallotBoxSealPolicy::SEAL_AT_CLOSE,
        ),
        (
            json!({"ballot_box_seal_policy": "do-not-seal"}),
            BallotBoxSealPolicy::DO_NOT_SEAL,
        ),
        (
            json!({"ballot_box_seal_policy": "unknown"}),
            BallotBoxSealPolicy::DO_NOT_SEAL,
        ),
        (json!({}), BallotBoxSealPolicy::DO_NOT_SEAL),
        (serde_json::Value::Null, BallotBoxSealPolicy::DO_NOT_SEAL),
        // Another field that doesn't parse doesn't hide the policy.
        (
            json!({"ballot_box_seal_policy": "seal-at-close", "language_conf": 7}),
            BallotBoxSealPolicy::SEAL_AT_CLOSE,
        ),
    ] {
        assert_eq!(
            event_ballot_box_seal_policy(&event(presentation.clone())),
            expected,
            "{presentation}"
        );
    }
}

#[tokio::test]
async fn initialization_reports_never_use_the_seals() {
    // No board is read: the event has none, and the answer doesn't need it.
    let elections = HashSet::from(["election".to_string()]);
    let sealing = event(json!({"ballot_box_seal_policy": "seal-at-close"}));
    assert!(
        sealed_elections(&sealing, &TallyType::INITIALIZATION_REPORT, &elections)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        sealed_elections(&sealing, &TallyType::ELECTORAL_RESULTS, &elections)
            .await
            .unwrap(),
        elections
    );
    // Policy off and no electoral log: nothing can have been sealed.
    assert!(
        sealed_elections(&event(json!({})), &TallyType::ELECTORAL_RESULTS, &elections)
            .await
            .unwrap()
            .is_empty()
    );
}

#[test]
fn identical_counted_entries_each_get_the_content() {
    let mut sealed = sealed(&BALLOTS, &VOTERS, Census::Plain);
    let first_counted = sealed
        .manifest
        .entries
        .iter()
        .find(|entry| entry.disposition == SealDisposition::Counted)
        .unwrap()
        .clone();
    sealed.manifest.entries.push(first_counted.clone());
    let result = sealed.counted(WeightedVotingPolicy::default());
    let copies = result
        .ballot_contents
        .iter()
        .filter(|(content, _)| ballot_hash(content).unwrap() == first_counted.ballot_hash)
        .count();
    assert_eq!(copies, 2);
}

#[test]
fn a_counted_ballot_needs_a_weight() {
    let mut manifest = sealed(&BALLOTS, &VOTERS, Census::Plain).manifest;
    assert!(check_counted_weights(&manifest).is_ok());
    // Ballots that don't count always have weight 0.
    assert!(manifest
        .entries
        .iter()
        .any(|entry| entry.disposition != SealDisposition::Counted && entry.weight == 0));
    let counted = manifest
        .entries
        .iter_mut()
        .find(|entry| entry.disposition == SealDisposition::Counted)
        .unwrap();
    counted.weight = 0;
    let ballot_id = counted.ballot_id.clone();
    let Err(Rejection(reason)) = check_counted_weights(&manifest) else {
        panic!("a counted ballot with weight 0 passed");
    };
    assert_eq!(
        reason,
        format!("Ballot ID {ballot_id} is counted with a weight of 0")
    );
}

fn log_row(id: i64, message: &Message) -> ElectoralLogMessage {
    let mut row = ElectoralLogMessage::try_from(message).unwrap();
    row.id = id;
    row
}

fn seal_message(eligible_voters: u64, key: &SigningKey) -> Message {
    let mut manifest = sealed(&BALLOTS, &VOTERS, Census::Plain).manifest;
    manifest.eligible_voters = eligible_voters;
    let built = electoral_log::seal::build(manifest).unwrap();
    Message::ballot_box_sealed_message(
        &built.manifest,
        built.hash,
        "Mayor",
        "North",
        &SigningData::new(key.clone(), "", key.clone()),
    )
    .unwrap()
}

#[test]
fn only_distinct_entries_signed_by_the_event_key_are_seals() {
    let key = SigningKey::generate().unwrap();
    let pk = VerificationKey::from_sk(&key).unwrap();
    let other = SigningKey::generate().unwrap();
    let genuine = seal_message(4, &key);
    let mut undecodable = log_row(4, &genuine);
    undecodable.message = vec![1, 2, 3];
    // Signed by the event key as system but sent by another key.
    let mut foreign_sender = seal_message(4, &key);
    foreign_sender.sender.pk = VerificationKey::from_sk(&other).unwrap();
    let rows = vec![
        log_row(1, &genuine),
        // A copy of the same bytes.
        log_row(2, &genuine),
        log_row(3, &seal_message(4, &other)),
        undecodable,
        log_row(5, &foreign_sender),
    ];
    let signed = signed_seal_entries(&rows, &pk).unwrap();
    assert_eq!(signed.len(), 1);
    assert_eq!(
        voting_crypto::RecordSerialize::record_serialize(&signed[0]).unwrap(),
        rows[0].message
    );
    // A second, different seal signed by the key is a seal too.
    let mut with_second = rows.clone();
    with_second.push(log_row(6, &seal_message(5, &key)));
    assert_eq!(signed_seal_entries(&with_second, &pk).unwrap().len(), 2);
    assert!(signed_seal_entries(&rows[2..], &pk).unwrap().is_empty());
}

#[tokio::test]
async fn board_calls_are_retried() {
    let calls = std::sync::atomic::AtomicUsize::new(0);
    let result = with_board_retries(|| {
        let call = calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        async move {
            if call < 2 {
                Err(anyhow!("already closed"))
            } else {
                Ok(call)
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(result, 2);
    let failing = with_board_retries(|| async { Err::<(), _>(anyhow!("down")) }).await;
    assert_eq!(failing.unwrap_err().to_string(), "down");
}
