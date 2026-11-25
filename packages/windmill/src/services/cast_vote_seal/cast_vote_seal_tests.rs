// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::testing::{ballot, SoftwareOpener};
use super::*;
use std::sync::atomic::Ordering;
use voting_crypto::signing::SigningKey;

const TENANT: &str = "2f6b9d41-8c3e-4a75-b0d2-6e1f9a3c5b87";
const EVENT: &str = "7a1e5c93-0d4b-4f68-92a7-c3e8b1d6f054";
const ELECTION: &str = "c5d2a8f1-6e3b-4917-8a0c-4f9b2e7d1a36";
const AREA: &str = "e9b4f7a2-3c1d-4a86-b5e0-8d2c6f1a9b73";

fn row_scope() -> RowScope<'static> {
    RowScope {
        tenant_id: TENANT,
        election_event_id: EVENT,
        election_id: ELECTION,
        area_id: AREA,
    }
}

fn event_key(point: Vec<u8>) -> SealKey {
    SealKey::new(event_key_id(TENANT, EVENT).unwrap(), 1, point)
}

fn sealed_row(key: &SealKey, seed: &str) -> (SealedRow, String) {
    let (content, ballot_id, hash) = ballot(seed);
    let envelope = seal_ballot(
        key,
        row_scope(),
        "hashable-ballot",
        &ballot_id,
        &hash,
        &content,
    )
    .unwrap();
    (
        SealedRow {
            election_id: ELECTION.into(),
            area_id: AREA.into(),
            ballot_id,
            format: "hsm-seal-v1+hashable-ballot".into(),
            content: envelope.to_content().unwrap(),
        },
        content,
    )
}

#[tokio::test]
async fn sealed_rows_open_to_their_ballots_in_bounded_batches() {
    let (opener, point) = SoftwareOpener::generate();
    let key = event_key(point);
    let rows: Vec<(SealedRow, String)> = (0..MAX_OPEN_BATCH + 3)
        .map(|index| sealed_row(&key, &format!("seed-{index}")))
        .collect();
    let (stored, ballots): (Vec<_>, Vec<_>) = rows.into_iter().unzip();
    // What is stored is not the ballot.
    assert!(stored.iter().zip(&ballots).all(|(row, ballot)| {
        !row.content.contains(ballot.as_str()) && row.content.contains(&row.ballot_id)
    }));
    let opened = open_rows_with(&opener, TENANT, EVENT, &key, &stored)
        .await
        .unwrap();
    assert_eq!(opened, ballots);
    assert_eq!(opener.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_moved_swapped_or_rekeyed_envelope_is_refused() {
    let (opener, point) = SoftwareOpener::generate();
    let key = event_key(point);
    let (row, _) = sealed_row(&key, "alice");
    let (other, _) = sealed_row(&key, "bob");
    let open = |rows: Vec<SealedRow>| {
        let key = key.clone();
        let opener = &opener;
        async move { open_rows_with(opener, TENANT, EVENT, &key, &rows).await }
    };
    // Another area or election: the associated data differs.
    let mut moved = row.clone();
    moved.area_id = ELECTION.into();
    assert!(open(vec![moved]).await.is_err());
    let mut elsewhere = row.clone();
    elsewhere.election_id = AREA.into();
    assert!(open(vec![elsewhere]).await.is_err());
    // Bob's envelope under Alice's Ballot ID.
    let mut swapped = row.clone();
    swapped.content = other.content.clone();
    assert!(open(vec![swapped]).await.is_err());
    // Bob's envelope relabelled with Alice's Ballot ID inside too.
    let mut relabelled: SealedBallot = serde_json::from_str(&other.content).unwrap();
    relabelled.ballot_id = row.ballot_id.clone();
    relabelled.cast_vote_hash = format!("{}{}", row.ballot_id, "07".repeat(32));
    let mut forged = row.clone();
    forged.content = relabelled.to_content().unwrap();
    let error = open(vec![forged]).await.unwrap_err();
    assert!(
        matches!(&error, OpenError::Ballots(failures) if failures.to_string().contains("does not open")),
        "{error}"
    );
    // A plaintext format, or another inner format.
    let mut plain = row.clone();
    plain.format = "hashable-ballot".into();
    assert!(open(vec![plain]).await.is_err());
    let mut multi = row.clone();
    multi.format = "hsm-seal-v1+hashable-multi-ballot".into();
    assert!(open(vec![multi]).await.is_err());
    // A key the event doesn't have.
    let other_key = SealKey::new(key.key_id.clone(), 2, key.public_key.clone());
    assert!(
        open_rows_with(&opener, TENANT, EVENT, &other_key, &[row.clone()])
            .await
            .is_err()
    );
    // Another event's key id.
    assert!(seal_ballot(
        &SealKey::new(
            event_key_id(TENANT, ELECTION).unwrap(),
            1,
            key.public_key.clone()
        ),
        row_scope(),
        "hashable-ballot",
        &row.ballot_id,
        &[0; 64],
        "x"
    )
    .is_err());
    // The event's key, but hashes that aren't the ballot's.
    let (content, ballot_id, mut hash) = ballot("carol");
    hash[63] ^= 1;
    assert!(seal_ballot(
        &key,
        row_scope(),
        "hashable-ballot",
        &ballot_id,
        &hash,
        &content
    )
    .is_err());
    assert!(seal_ballot(
        &key,
        row_scope(),
        "hashable-ballot",
        &row.ballot_id,
        &ballot("carol").2,
        &content
    )
    .is_err());
    assert!(open(vec![row]).await.is_ok());
}

/// The cast-vote hash travels in clear, outside the associated data: an
/// envelope that opens must still carry its ballot's cast-vote hash.
#[tokio::test]
async fn an_opened_ballot_must_match_its_cast_vote_hash() {
    let (opener, point) = SoftwareOpener::generate();
    let key = event_key(point);
    let (mut row, _) = sealed_row(&key, "alice");
    let mut envelope: SealedBallot = serde_json::from_str(&row.content).unwrap();
    let tail = if envelope.cast_vote_hash.ends_with('0') {
        "1"
    } else {
        "0"
    };
    envelope.cast_vote_hash.replace_range(127.., tail);
    envelope.check().unwrap();
    row.content = envelope.to_content().unwrap();
    let error = open_rows_with(&opener, TENANT, EVENT, &key, &[row])
        .await
        .unwrap_err();
    assert!(
        matches!(&error, OpenError::Ballots(failures) if failures.to_string().contains("does not match its cast-vote hash")),
        "{error}"
    );
}

/// Every row is tried, and the refusal names each ballot that failed.
#[tokio::test]
async fn a_refusal_names_every_failing_ballot() {
    let (opener, point) = SoftwareOpener::generate();
    let key = event_key(point);
    let mut rows: Vec<SealedRow> = (0..MAX_OPEN_BATCH + 6)
        .map(|index| sealed_row(&key, &format!("seed-{index}")).0)
        .collect();
    let bad = [1, MAX_OPEN_BATCH + 1, MAX_OPEN_BATCH + 5];
    for index in bad {
        rows[index].area_id = ELECTION.into();
    }
    // An envelope that isn't one never reaches the opener.
    rows[2].content = "{}".into();
    let error = open_rows_with(&opener, TENANT, EVENT, &key, &rows)
        .await
        .unwrap_err();
    let OpenError::Ballots(failures) = error else {
        panic!("{error}");
    };
    assert_eq!(failures.count(), 4);
    let mut expected: Vec<&str> = bad
        .iter()
        .map(|&index| rows[index].ballot_id.as_str())
        .collect();
    expected.insert(1, rows[2].ballot_id.as_str());
    assert_eq!(failures.ballot_ids().collect::<Vec<_>>(), expected);
    assert_eq!(opener.calls.load(Ordering::SeqCst), 2);
    let report = failures.to_string();
    assert!(report.starts_with("4 sealed ballot(s)"), "{report}");
    assert!(expected.iter().all(|ballot_id| report.contains(ballot_id)));
    // The outcomes keep the rows' order.
    let outcomes = open_each_with(&opener, TENANT, EVENT, &key, &rows)
        .await
        .unwrap();
    assert_eq!(outcomes.len(), rows.len());
    assert!(outcomes[0].is_ok() && outcomes[1].is_err() && outcomes[2].is_err());
}

#[test]
fn a_refusal_lists_a_bounded_number_of_ballots() {
    let mut failures = OpenFailures::default();
    assert!(failures.is_empty());
    for index in 0..MAX_REPORTED_FAILURES + 5 {
        failures.push(&format!("ballot-{index}"), "does not open".into());
    }
    assert_eq!(failures.count(), MAX_REPORTED_FAILURES + 5);
    assert_eq!(failures.ballot_ids().count(), MAX_REPORTED_FAILURES);
    let report = failures.to_string();
    assert!(report.ends_with("; and 5 more"), "{report}");
    assert!(report.contains("ballot-0 (does not open)"));
    assert!(!report.contains(&format!("ballot-{MAX_REPORTED_FAILURES} ")));
}

/// A device that opens to other bytes: the opened ballot must still hash
/// to its Ballot ID.
struct LyingOpener;

impl CastVoteOpener for LyingOpener {
    fn open_batch<'a>(&'a self, _: &'a str, _: u32, items: Vec<OpenItem>) -> OpenFuture<'a> {
        Box::pin(async move {
            Ok(items
                .iter()
                .map(|_| Ok(ballot("mallory").0.into_bytes()))
                .collect())
        })
    }
}

/// A device that loses an answer.
struct ShortOpener;

impl CastVoteOpener for ShortOpener {
    fn open_batch<'a>(&'a self, _: &'a str, _: u32, _: Vec<OpenItem>) -> OpenFuture<'a> {
        Box::pin(async move { Ok(vec![]) })
    }
}

#[tokio::test]
async fn an_opened_ballot_must_hash_to_its_ballot_id() {
    let (_, point) = SoftwareOpener::generate();
    let key = event_key(point);
    let (row, _) = sealed_row(&key, "alice");
    let error = open_rows_with(&LyingOpener, TENANT, EVENT, &key, &[row.clone()])
        .await
        .unwrap_err();
    assert!(
        matches!(&error, OpenError::Ballots(failures) if failures.to_string().contains("does not match its Ballot ID")),
        "{error}"
    );
    // A lost answer is the opener's failure, not the ballot's.
    assert!(matches!(
        open_rows_with(&ShortOpener, TENANT, EVENT, &key, &[row]).await,
        Err(OpenError::Unavailable(_))
    ));
}

#[test]
fn rows_must_fit_their_events_sealing() {
    let key = event_key(vec![4; 65]);
    let sealed = Sealing::Required(Some(key.clone()));
    assert!(check_row_format(&sealed, "hsm-seal-v1+hashable-ballot", "a").is_ok());
    assert!(check_row_format(&sealed, "hashable-ballot", "a").is_err());
    assert!(check_row_format(&Sealing::Legacy, "hashable-ballot", "a").is_ok());
    assert!(check_row_format(&Sealing::Legacy, "hsm-seal-v1+hashable-ballot", "a").is_err());
    assert_eq!(Sealing::Legacy.key().unwrap(), None);
    assert_eq!(sealed.key().unwrap(), Some(&key));
    assert!(Sealing::Required(None).key().is_err());
}

#[test]
fn the_vectors_cast_vote_aad_is_the_ballot_box_associated_data() {
    #[derive(serde::Deserialize)]
    struct Vector {
        name: String,
        aad: String,
        key_id: String,
    }
    #[derive(serde::Deserialize)]
    struct Vectors {
        vectors: Vec<Vector>,
    }
    let vectors: Vectors = serde_json::from_str(include_str!("hsm_seal_v1_vectors.json")).unwrap();
    let vector = vectors
        .vectors
        .iter()
        .find(|vector| vector.name == "cast vote")
        .unwrap();
    let tenant = "5b0f4c8e-2d71-4a93-b6e5-1c9d7a3f8e20";
    let event = "9e2c6a17-4f8b-4d05-a3c1-7b6e5d4f3a29";
    let ballot_id = "ab".repeat(32);
    let aad = associated_data(&SealScope {
        tenant_id: tenant,
        election_event_id: event,
        election_id: "1f7d3b95-6c2a-4e48-9b0d-2a8c4e6f7d13",
        area_id: "d4a8e2c6-0b9f-4713-8e5a-6c1b3d9f2a74",
        ballot_id: &ballot_id,
        inner_format: "hashable-ballot",
    })
    .unwrap();
    assert_eq!(hex::encode(aad), vector.aad);
    assert_eq!(event_key_id(tenant, event).unwrap(), vector.key_id);
}

fn pin_entry(
    key: &SealKey,
    signer: &SigningKey,
    tenant: &str,
    event: &str,
) -> electoral_log::ElectoralLogMessage {
    let signer = electoral_log::signer::software(signer.clone());
    let message = Message::cast_vote_seal_key_published_message(
        key.pin(tenant, event),
        &SigningData::with_signers(signer.clone(), "", signer),
    )
    .unwrap();
    (&message).try_into().unwrap()
}

#[test]
fn only_the_system_keys_single_pin_of_the_published_key_passes() {
    let system = SigningKey::generate().unwrap();
    let system_pk = VerificationKey::from_sk(&system).unwrap();
    let key = event_key(vec![4; 65]);
    let pinned = pin_entry(&key, &system, TENANT, EVENT);
    let pins = signed_pins(&[pinned.clone(), pinned.clone()], &system_pk).unwrap();
    assert_eq!(pins, vec![key.pin(TENANT, EVENT)]);
    assert!(check_pin(&key, pins.first(), TENANT, EVENT).is_ok());
    // Nothing pinned, another key or another event pinned.
    assert!(check_pin(&key, None, TENANT, EVENT).is_err());
    let substituted = event_key(vec![5; 65]);
    assert!(check_pin(&substituted, pins.first(), TENANT, EVENT).is_err());
    assert!(check_pin(&key, pins.first(), TENANT, ELECTION).is_err());
    // A pin signed by anyone else is not the board's.
    let stranger = SigningKey::generate().unwrap();
    let forged = pin_entry(&substituted, &stranger, TENANT, EVENT);
    assert_eq!(
        signed_pins(&[forged.clone()], &system_pk).unwrap(),
        Vec::<CastVoteSealKeyPin>::new()
    );
    assert_eq!(
        signed_pins(&[pinned, forged], &system_pk).unwrap(),
        vec![key.pin(TENANT, EVENT)]
    );
    // The statement says what it pins.
    let message =
        Message::record_deserialize(&pin_entry(&key, &system, TENANT, EVENT).message).unwrap();
    assert!(matches!(
        message.statement.head.kind,
        StatementType::CastVoteSealKeyPublished
    ));
    assert!(message
        .statement
        .head
        .description
        .contains(&key.fingerprint));
    assert_eq!(
        pin_delivery_id(EVENT),
        format!("cast-vote-seal-key:{EVENT}")
    );
}

#[tokio::test]
async fn the_installed_opener_opens_and_is_installed_once() {
    let key = super::testing::event_key(TENANT, EVENT);
    assert!(installed().is_ok());
    let (again, _) = SoftwareOpener::generate();
    assert!(install(Arc::new(again)).is_err());
    // The installed opener opens what is sealed to a key it holds.
    let (content, ballot_id, hash) = ballot("installed");
    let envelope = seal_ballot(
        &key,
        row_scope(),
        "hashable-ballot",
        &ballot_id,
        &hash,
        &content,
    )
    .unwrap();
    let row = SealedRow {
        election_id: ELECTION.into(),
        area_id: AREA.into(),
        ballot_id,
        format: "hsm-seal-v1+hashable-ballot".into(),
        content: envelope.to_content().unwrap(),
    };
    assert_eq!(
        open_rows(TENANT, EVENT, &key, &[row]).await.unwrap(),
        vec![content]
    );
}

/// An opener that fails every call, as an HSM that is down does.
#[derive(Default)]
struct DownOpener {
    calls: std::sync::atomic::AtomicUsize,
}

impl CastVoteOpener for DownOpener {
    fn open_batch<'a>(
        &'a self,
        _key_id: &'a str,
        _key_version: u32,
        _items: Vec<OpenItem>,
    ) -> OpenFuture<'a> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Err(anyhow!("the HSM does not answer")) })
    }
}

#[test]
fn the_opener_backoff_doubles_up_to_its_cap() {
    assert_eq!(backoff_after(0), OPENER_BACKOFF);
    assert_eq!(backoff_after(1), OPENER_BACKOFF);
    assert_eq!(backoff_after(2), OPENER_BACKOFF * 2);
    assert_eq!(backoff_after(3), OPENER_BACKOFF * 4);
    assert_eq!(backoff_after(6), OPENER_MAX_BACKOFF);
    assert_eq!(backoff_after(u32::MAX), OPENER_MAX_BACKOFF);
}

/// A failing opener is not asked again until its wait is over, and each
/// failure in a row doubles the wait; one key's wait doesn't hold another's.
#[tokio::test]
async fn a_failing_opener_is_asked_again_only_after_its_backoff() {
    let (_, point) = SoftwareOpener::generate();
    let key = event_key(point);
    let (row, _) = sealed_row(&key, "alice");
    let rows = [row];
    let down = Arc::new(DownOpener::default());
    let backoff = OpenerBackoff::default();
    let start = Instant::now();
    let attempt = |now: Instant| {
        let down = down.clone();
        let backoff = &backoff;
        let key = &key;
        let rows = &rows;
        async move {
            open_each_backed_off(
                backoff,
                move || Ok(down as Arc<dyn CastVoteOpener>),
                now,
                TENANT,
                EVENT,
                key,
                rows,
            )
            .await
        }
    };
    for (now, calls) in [
        (start, 1),
        // Refused without a call while the first wait lasts.
        (start + OPENER_BACKOFF / 2, 1),
        (start + OPENER_BACKOFF, 2),
        // The second wait is twice as long.
        (start + OPENER_BACKOFF * 2, 2),
        (start + OPENER_BACKOFF * 3, 3),
    ] {
        let error = attempt(now).await.unwrap_err();
        assert!(matches!(error, OpenError::Unavailable(_)), "{error}");
        assert_eq!(down.calls.load(Ordering::SeqCst), calls);
    }
    let waiting = attempt(start + OPENER_BACKOFF * 4).await.unwrap_err();
    assert!(
        waiting.to_string().contains("3 time(s) in a row"),
        "{waiting}"
    );

    let (_, other_point) = SoftwareOpener::generate();
    let other = SealKey::new("another-key".into(), 1, other_point);
    assert!(backoff.admit(&other, start).is_ok());
    assert!(backoff.admit(&key, start).is_err());
}

/// No opener installed waits as a failing one does; an answer, even one
/// where ballots don't open, ends the wait.
#[tokio::test]
async fn a_missing_opener_waits_and_any_answer_resets_the_wait() {
    let (opener, point) = SoftwareOpener::generate();
    let opener: Arc<dyn CastVoteOpener> = Arc::new(opener);
    let key = event_key(point);
    let (good, ballot) = sealed_row(&key, "alice");
    let mut moved = good.clone();
    moved.area_id = ELECTION.into();
    let backoff = OpenerBackoff::default();
    let start = Instant::now();
    let missing = open_each_backed_off(
        &backoff,
        || Err(anyhow!("no opener")),
        start,
        TENANT,
        EVENT,
        &key,
        &[good.clone()],
    )
    .await
    .unwrap_err();
    assert!(matches!(missing, OpenError::Unavailable(_)), "{missing}");
    assert!(backoff.admit(&key, start + OPENER_BACKOFF / 2).is_err());

    let later = start + OPENER_BACKOFF;
    let outcomes = open_each_backed_off(
        &backoff,
        || Ok(opener.clone()),
        later,
        TENANT,
        EVENT,
        &key,
        &[good, moved],
    )
    .await
    .unwrap();
    assert_eq!(outcomes[0], Ok(ballot));
    assert!(outcomes[1].is_err());
    assert!(backoff.admit(&key, later).is_ok());
}

#[test]
fn a_sealed_ballots_refusal_is_found_under_context_and_joined() {
    let refusal = SealedBallotsRefused("area 1: 1 sealed ballot(s) don't open".into());
    let error = anyhow::Error::new(refusal.clone())
        .context("contest area")
        .context("Error extracting the ballots");
    assert_eq!(SealedBallotsRefused::of(&error), Some(&refusal));
    assert_eq!(
        SealedBallotsRefused::of(&anyhow!("the HSM does not answer")),
        None
    );
    assert_eq!(SealedBallotsRefused::joined(vec![]), None);
    let joined = SealedBallotsRefused::joined(vec![
        refusal.clone(),
        SealedBallotsRefused("area 2: plaintext".into()),
        refusal.clone(),
    ])
    .unwrap();
    assert_eq!(
        joined.to_string(),
        "area 1: 1 sealed ballot(s) don't open; area 2: plaintext"
    );
}
