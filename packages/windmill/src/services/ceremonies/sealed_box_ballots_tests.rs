// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::services::join::{merge_join_csv, MultiplicitySource};
use electoral_log::seal::{SealEntry, SEAL_FORMAT_V1};
use serde_json::json;
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

fn csv_file(rows: &[Vec<String>]) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    for row in rows {
        writeln!(file, "{}", row.join(",")).unwrap();
    }
    file.flush().unwrap();
    file
}

/// What today's path computes from the ballots and the census.
fn joined(ballots: &[Ballot], voters: &[(&str, u64)], census: Census) -> MergeJoinResult {
    let mut ballots = ballots.to_vec();
    ballots.sort();
    let mut voters = voters.to_vec();
    voters.sort();
    let ballots_file = csv_file(
        &ballots
            .iter()
            .map(|(voter, content, channel)| {
                vec![voter.to_string(), content.to_string(), channel.to_string()]
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
    merge_join_csv(
        &ballots_file.reopen().unwrap(),
        &voters_file.reopen().unwrap(),
        0,
        0,
        1,
        Some(2),
        census.source(),
    )
    .unwrap()
}

fn entry(content: &str, disposition: SealDisposition, weight: u64, channel: &str) -> SealEntry {
    SealEntry {
        ballot_hash: ballot_hash(content).unwrap(),
        ballot_id: format!("id-{content}"),
        disposition,
        weight,
        channel: channel.to_string(),
    }
}

/// The manifest the sealer writes for the same box, plus ballots the old
/// path never reads (replaced and discarded).
fn sealed(
    ballots: &[Ballot],
    voters: &[(&str, u64)],
    census: Census,
) -> (BallotBoxSealManifest, HashMap<(Hash, String), String>) {
    let mut entries: Vec<SealEntry> = ballots
        .iter()
        .map(
            |(voter, content, channel)| match voters.iter().find(|(id, _)| id == voter) {
                Some((_, value)) => entry(
                    content,
                    SealDisposition::Counted,
                    census.weight(*value),
                    channel,
                ),
                None => entry(content, SealDisposition::NotEligible, 0, channel),
            },
        )
        .collect();
    entries.push(entry("replaced", SealDisposition::Replaced, 0, "ONLINE"));
    entries.push(entry("discarded", SealDisposition::Discarded, 0, "KIOSK"));
    let contents = ballots
        .iter()
        .map(|(_, content, _)| {
            (
                (ballot_hash(content).unwrap(), format!("id-{content}")),
                content.to_string(),
            )
        })
        .collect();
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
    (manifest, contents)
}

fn sorted(mut result: MergeJoinResult) -> MergeJoinResult {
    result.ballot_contents.sort();
    result
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
        let (manifest, contents) = sealed(&BALLOTS, &VOTERS, census);
        assert_eq!(
            sorted(count_manifest(&manifest, contents.clone(), census.policy()).unwrap()),
            sorted(joined(&BALLOTS, &VOTERS, census)),
            "{census:?}"
        );
    }
}

#[test]
fn an_empty_box_counts_only_its_census() {
    let (manifest, contents) = sealed(&[], &VOTERS, Census::Plain);
    let result =
        count_manifest(&manifest, contents.clone(), WeightedVotingPolicy::default()).unwrap();
    assert_eq!(result, joined(&[], &VOTERS, Census::Plain));
    assert_eq!(result.eligible_voters, 4);
    assert!(result.ballot_contents.is_empty());
}

#[test]
fn a_voter_disabled_after_the_seal_does_not_change_the_counts() {
    let (manifest, contents) = sealed(&BALLOTS, &VOTERS, Census::Plain);
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
        sorted(
            count_manifest(&manifest, contents.clone(), WeightedVotingPolicy::default()).unwrap()
        ),
        at_seal
    );
}

#[test]
fn replaced_and_discarded_ballots_do_not_count() {
    let (manifest, contents) = sealed(&[], &[], Census::Plain);
    let result =
        count_manifest(&manifest, contents.clone(), WeightedVotingPolicy::default()).unwrap();
    assert_eq!(manifest.ballots_in_box(), 2);
    assert_eq!(result.casted_ballots, 0);
    assert_eq!(result.ballots_without_voter, 0);
    assert!(result.casted_ballots_by_channel.is_empty());
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
    let (mut manifest, contents) = sealed(&BALLOTS, &VOTERS, Census::Plain);
    let first_counted = manifest
        .entries
        .iter()
        .find(|entry| entry.disposition == SealDisposition::Counted)
        .unwrap()
        .clone();
    manifest.entries.push(first_counted.clone());
    let result = count_manifest(&manifest, contents, WeightedVotingPolicy::default()).unwrap();
    let copies = result
        .ballot_contents
        .iter()
        .filter(|(content, _)| ballot_hash(content).unwrap() == first_counted.ballot_hash)
        .count();
    assert_eq!(copies, 2);
}

#[test]
fn a_counted_ballot_needs_a_weight() {
    let (mut manifest, _) = sealed(&BALLOTS, &VOTERS, Census::Plain);
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

fn seal_message(eligible_voters: u64, key: &StrandSignatureSk) -> Message {
    let (mut manifest, _) = sealed(&BALLOTS, &VOTERS, Census::Plain);
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
    let key = StrandSignatureSk::generate().unwrap();
    let pk = StrandSignaturePk::from_sk(&key).unwrap();
    let other = StrandSignatureSk::generate().unwrap();
    let genuine = seal_message(4, &key);
    let mut undecodable = log_row(4, &genuine);
    undecodable.message = vec![1, 2, 3];
    // Signed by the event key as system but sent by another key.
    let mut foreign_sender = seal_message(4, &key);
    foreign_sender.sender.pk = StrandSignaturePk::from_sk(&other).unwrap();
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
        strand::serialization::StrandSerialize::strand_serialize(&signed[0]).unwrap(),
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
