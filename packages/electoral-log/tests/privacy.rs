// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Privacy invariants against the disposable ELECTORAL_LOG_TEST_DATABASE_URL.

use anyhow::Result;
use chrono::{Duration, Utc};
use electoral_log::adapters::ballot_box::{
    AcceptBallot, AcceptOutcome, AcceptReceived, BallotStatus,
};
use electoral_log::adapters::ballot_box_received::{
    ReceiveBallot, ReceiveOutcome, ReceivedSignature,
};
use electoral_log::adapters::postgres::PostgresStore;
use electoral_log::adapters::privacy::{
    BallotIdentity, ForensicNetworkRecord, PrivacyBallotQuery, PrivacyBallotSearch,
    ReceivedEvidenceKind,
};
use uuid::Uuid;

/// The crate that computes Tracking IDs depends on this one; a stand-in is enough here.
fn stand_in_tracking_id(_ballot_hash: &str, _voter_signing_pk: &str) -> Option<String> {
    Some("AAAA-0001-BBBB-CCCC".into())
}

fn searched<'a>(
    query: &PrivacyBallotQuery<'a>,
    search: PrivacyBallotSearch<'a>,
) -> PrivacyBallotQuery<'a> {
    PrivacyBallotQuery {
        search: Some(search),
        ..query.clone()
    }
}

#[tokio::test]
#[ignore = "needs isolated ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn privacy_storage_keeps_identity_private_and_cast_effects_atomic() -> Result<()> {
    let store = PostgresStore::new(std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")?.parse()?)?;
    store.initialize().await?;
    let event = Uuid::new_v4().to_string();
    let election = Uuid::new_v4().to_string();
    let other_election = Uuid::new_v4().to_string();
    let area = Uuid::new_v4().to_string();
    let other_area = Uuid::new_v4().to_string();
    let voter = "private-test-voter".to_owned();
    store.create_ballot_box(&event).await?;
    assert_eq!(
        store.ballot_identity(&event).await?,
        BallotIdentity::VoterId
    );
    store
        .set_ballot_identity(&event, BallotIdentity::Pseudonym)
        .await?;
    assert!(store
        .set_ballot_identity(&event, BallotIdentity::VoterId)
        .await
        .is_err());
    assert_eq!(
        store
            .allocate_voter_pseudonyms(&event, &[voter.clone(), voter.clone()])
            .await?,
        1
    );
    assert_eq!(
        store
            .allocate_voter_pseudonyms(&event, &[voter.clone()])
            .await?,
        0
    );
    let pseudonym = store
        .voter_pseudonym(&event, &voter)
        .await?
        .expect("allocated");
    assert_eq!(
        store.resolve_pseudonym(&event, &pseudonym).await?,
        Some(voter.clone())
    );
    assert!(store
        .participation_for_voters(&event, &[voter.clone()], None)
        .await?
        .is_empty());
    let received = ReceiveBallot {
        election_event_id: &event,
        election_id: &election,
        area_id: &area,
        voter_id: &pseudonym,
        ballot_hash: "encrypted-ballot-hash",
        content: "encrypted-ballot",
        voter_signing_pk: "test-public-key",
        voter_ballot_signature: "test-voter-signature",
            vitrine: None,
        };
    let now = Utc::now();
    let mut attempts = 0;
    let stored = match store
        .receive_ballot(&received, now, |_| {
            attempts += 1;
            Ok(ReceivedSignature {
                ballot_id: if attempts == 1 {
                    pseudonym.clone()
                } else {
                    "AAAA-0001".into()
                },
                key_id: "test-key-id".into(),
                received_signature: "test-box-signature".into(),
            })
        })
        .await?
    {
        ReceiveOutcome::Received(row) => row,
        other => panic!("Unexpected receive outcome: {other:?}"),
    };
    assert_eq!(attempts, 2, "a pseudonym occupies the short-ID namespace");
    assert_eq!(stored.voter_id, pseudonym);
    assert!(matches!(
        store
            .receive_ballot(&received, now, |_| panic!("retry must reuse receipt"))
            .await?,
        ReceiveOutcome::AlreadyReceived(_)
    ));
    let pending = store.pending_received_ballots(&event, 100).await?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].kind, ReceivedEvidenceKind::Received);
    assert_eq!(pending[0].kind.as_str(), "received");
    let by_hash = store
        .received_ballots_for_hashes(
            &event,
            &["encrypted-ballot-hash".into(), "absent-hash".into()],
        )
        .await?;
    assert_eq!(by_hash.len(), 1);
    assert_eq!(by_hash[0].ballot_id, stored.ballot_id);
    assert!(store
        .received_ballots_for_hashes(&other_election, &["encrypted-ballot-hash".into()])
        .await?
        .is_empty());
    store.create_ballot_box(&event).await?;
    assert_eq!(
        store.pending_received_ballots(&event, 100).await?.len(),
        1,
        "publication retries retain pending evidence"
    );
    // A ballot is in the public log from the moment it is received, once.
    let received_page = store
        .privacy_ballot_page(&PrivacyBallotQuery {
            event: &event,
            area: &area,
            election: Some(&election),
            elections: None,
            search: None,
            before: None,
            limit: 10,
            tracking_id: stand_in_tracking_id,
        })
        .await?;
    assert_eq!(received_page.ballots.len(), 1);
    assert_eq!(received_page.ballots[0].stage, "received");
    assert_eq!(received_page.ballots[0].voting_channel, "");
    assert_eq!(
        store
            .privacy_ballot_count(&event, &area, Some(&election), None)
            .await?,
        1
    );
    let hash = [7u8; 64];
    let ballot = AcceptBallot {
        election_event_id: &event,
        election_id: &election,
        area_id: &area,
        voter_id: &pseudonym,
        ballot_id: received.ballot_hash,
        format: "hashable-ballot",
        content: received.content,
        voter_signature: None,
        pseudonym_hash: &hash,
        ballot_hash: &hash,
        voting_channel: "ONLINE",
        status: BallotStatus::Valid,
        voter_ip: None,
        voter_country: None,
        username: Some(&pseudonym),
        allowed_votes: 1,
        vitrine: None,
    };
    let cast = AcceptReceived {
        ballot_id: &stored.ballot_id,
        cast_at: now,
        cast_signature: "test-cast-signature",
        cast_receipt_signature: "test-cast-receipt",
    };
    let record = ForensicNetworkRecord {
        election_event_id: &event,
        election_id: Some(&election),
        area_id: Some(&area),
        event_kind: "cast",
        delivery_id: "cast-test",
        pseudonym: &pseudonym,
        ballot_id: Some(&stored.ballot_id),
        ip: "192.0.2.10",
        country: Some("XX"),
        user_agent: Some("synthetic"),
        recorded_at: now,
    };
    let mut leaking = ballot.clone();
    leaking.voter_ip = Some("192.0.2.10");
    let rejected = store
        .accept_received_ballot_with_forensics(&leaking, &cast, &record)
        .await
        .expect_err("network data cannot enter public ballots");
    assert_eq!(
        rejected
            .downcast_ref::<tokio_postgres::Error>()
            .and_then(|e| e.as_db_error())
            .map(|e| e.message()),
        Some("ballot_private_fields_forbidden")
    );
    assert!(store
        .participation_for_voters(&event, &[voter.clone()], None)
        .await?
        .is_empty());
    let records: i64 = store.client().await?.query_one("SELECT count(*) FROM sequent_backend.forensic_network_record WHERE election_event_id = $1::text::uuid", &[&event]).await?.get(0);
    assert_eq!(records, 0, "failed acceptance creates no forensic record");
    let (first, second) = tokio::join!(
        store.accept_received_ballot_with_forensics(&ballot, &cast, &record),
        store.accept_received_ballot_with_forensics(&ballot, &cast, &record),
    );
    let outcomes = [first?, second?];
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, AcceptOutcome::Accepted { .. }))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == AcceptOutcome::DuplicateBallotId)
            .count(),
        1
    );
    assert_eq!(
        store
            .participation_for_voters(&event, &[voter.clone()], Some(&election))
            .await?
            .len(),
        1
    );
    assert!(store
        .participation_for_voters(&event, &[voter.clone()], Some(&other_election))
        .await?
        .is_empty());
    assert_eq!(store.privacy_participation_count(&event, None).await?, 1);
    assert_eq!(
        store.participating_voters(&event, Some(&election)).await?,
        vec![voter.clone()]
    );
    assert!(store
        .participating_voters(&event, Some(&other_election))
        .await?
        .is_empty());
    let mapped = store
        .voters_for_pseudonyms(&event, &[pseudonym.clone(), "ABCD-EFGH".into()])
        .await?;
    assert_eq!(mapped.len(), 1);
    assert_eq!(mapped.get(&pseudonym), Some(&voter));
    assert!(store
        .voters_for_pseudonyms(&other_election, &[pseudonym.clone()])
        .await?
        .is_empty());
    assert_eq!(
        store
            .privacy_participation_count(&event, Some(&election))
            .await?,
        1
    );
    assert_eq!(
        store
            .privacy_participation_count(&event, Some(&other_election))
            .await?,
        0
    );
    assert_eq!(store.refresh_forensic_summary(&event, 100).await?, 1);
    assert_eq!(store.refresh_forensic_summary(&event, 100).await?, 0);
    let summarized: i64 = store.client().await?.query_one("SELECT records FROM sequent_backend.forensic_network_summary WHERE election_event_id = $1::text::uuid AND election_id = $2::text::uuid AND area_id = $3::text::uuid", &[&event, &election, &area]).await?.get(0);
    assert_eq!(summarized, 1);
    let query = PrivacyBallotQuery {
        event: &event,
        area: &area,
        election: Some(&election),
        elections: None,
        search: None,
        before: None,
        limit: 10,
        tracking_id: stand_in_tracking_id,
    };
    let page = store.privacy_ballot_page(&query).await?;
    assert_eq!(page.ballots.len(), 1);
    assert_eq!(page.ballots[0].pseudonym, pseudonym);
    assert_eq!(page.ballots[0].tracking_id, "AAAA-0001-BBBB-CCCC");
    assert_eq!(page.ballots[0].stage, "cast");
    assert_eq!(page.ballots[0].voting_channel, "ONLINE");
    assert_eq!(page.match_count, None);
    // The receipt key is not a Tracking ID: it names nothing in the log.
    assert_eq!(
        store
            .privacy_ballot_page(&searched(
                &query,
                PrivacyBallotSearch {
                    pseudonym: Some("AAAA"),
                    tracking_id: None,
                }
            ))
            .await?
            .match_count,
        Some(0)
    );
    let by_tracking = store
        .privacy_ballot_page(&searched(
            &query,
            PrivacyBallotSearch {
                pseudonym: None,
                tracking_id: Some("AAAA-0001"),
            },
        ))
        .await?;
    assert_eq!(by_tracking.match_count, Some(1));
    assert_eq!(by_tracking.ballots.len(), 1);
    assert_eq!(by_tracking.tracking_matches, None);
    assert_eq!(
        store
            .privacy_ballot_page(&searched(
                &query,
                PrivacyBallotSearch {
                    pseudonym: None,
                    tracking_id: Some("AAAA-0002"),
                }
            ))
            .await?
            .match_count,
        Some(0)
    );
    assert_eq!(
        store
            .privacy_ballot_page(&searched(
                &query,
                PrivacyBallotSearch {
                    pseudonym: Some(&pseudonym[..2]),
                    tracking_id: None,
                }
            ))
            .await?
            .match_count,
        Some(1)
    );
    let both = store
        .privacy_ballot_page(&searched(
            &query,
            PrivacyBallotSearch {
                pseudonym: Some("AAAA-0001"),
                tracking_id: Some("AAAA-0001"),
            },
        ))
        .await?;
    assert_eq!(both.match_count, Some(1));
    assert_eq!(both.tracking_matches, Some(1));
    assert_eq!(both.pseudonym_matches, Some(0));
    assert!(store
        .privacy_ballot_page(&searched(&query, PrivacyBallotSearch::default()))
        .await?
        .ballots
        .is_empty());
    assert!(store
        .privacy_ballot_page(&searched(
            &query,
            PrivacyBallotSearch {
                pseudonym: Some("%"),
                tracking_id: None,
            }
        ))
        .await
        .is_err());
    let forbidden_elections = [other_election.clone()];
    assert!(store
        .privacy_ballot_page(&PrivacyBallotQuery {
            election: None,
            elections: Some(&forbidden_elections),
            ..query.clone()
        })
        .await?
        .ballots
        .is_empty());
    assert_eq!(
        store
            .privacy_ballot_count(&event, &area, None, Some(&forbidden_elections))
            .await?,
        0
    );
    assert!(store
        .privacy_ballot_page(&PrivacyBallotQuery {
            area: &other_area,
            ..query
        })
        .await?
        .ballots
        .is_empty());
    assert!(store
        .privacy_ballot_details(&event, &other_area, Some(&election), &stored.ballot_id)
        .await?
        .is_none());
    let ids = pending.iter().map(|entry| entry.id).collect::<Vec<_>>();
    assert_eq!(
        store.remove_pending_received(&other_election, &ids).await?,
        0
    );
    assert_eq!(store.remove_pending_received(&event, &ids).await?, 1);
    assert_eq!(store.remove_pending_received(&event, &ids).await?, 0);
    assert_eq!(
        store
            .purge_forensic_network(&event, now + Duration::seconds(1), 100)
            .await?,
        1
    );
    assert_eq!(store.purge_voter_pseudonyms(&event).await?, 1);
    assert_eq!(store.purge_voter_pseudonyms(&event).await?, 0);
    assert!(store.resolve_pseudonym(&event, &pseudonym).await?.is_none());
    assert!(store
        .allocate_voter_pseudonyms(&event, &[voter])
        .await
        .is_err());
    assert!(
        store
            .privacy_ballot_details(&event, &area, Some(&election), &stored.ballot_id)
            .await?
            .is_some(),
        "public evidence survives purge"
    );
    // The SQL-console grant must not grant the private schema implicitly.
    let mut client = store.client().await?;
    let tx = client.transaction().await?;
    let reader = format!("privacy_test_reader_{}", Uuid::new_v4().simple());
    tx.batch_execute(&format!(
        "CREATE ROLE {reader}; GRANT USAGE ON SCHEMA public TO {reader};
        GRANT SELECT ON ALL TABLES IN SCHEMA public TO {reader}; SET LOCAL ROLE {reader}"
    ))
    .await?;
    tx.query_one("SELECT count(*) FROM ballot_box_ballot", &[])
        .await?;
    let denied = tx
        .query("SELECT * FROM sequent_backend.voter_pseudonym", &[])
        .await
        .expect_err("console role cannot read mappings");
    assert_eq!(
        denied.code(),
        Some(&tokio_postgres::error::SqlState::INSUFFICIENT_PRIVILEGE)
    );
    tx.rollback().await?; // Also removes the test-only role and its grants.
    drop(client);
    store.drop_ballot_box(&event).await?;

    let allocation_event = Uuid::new_v4().to_string();
    store
        .set_ballot_identity(&allocation_event, BallotIdentity::Pseudonym)
        .await?;
    let voters: Vec<String> = (0..100).map(|i| format!("concurrent-voter-{i}")).collect();
    let (left, right) = tokio::join!(
        store.allocate_voter_pseudonyms(&allocation_event, &voters),
        store.allocate_voter_pseudonyms(&allocation_event, &voters)
    );
    assert_eq!(left? + right?, 100);
    assert_eq!(
        store
            .pseudonyms_for_voters(&allocation_event, &voters)
            .await?
            .len(),
        100
    );
    assert_eq!(
        store
            .purge_voter_pseudonyms_batch(&allocation_event, 7)
            .await?,
        7
    );
    assert_eq!(store.purge_voter_pseudonyms(&allocation_event).await?, 93);
    Ok(())
}

#[tokio::test]
#[ignore = "needs isolated ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn privacy_delivery_queue_preserves_original_bytes_and_scoped_acknowledgements() -> Result<()>
{
    let store = PostgresStore::new(std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")?.parse()?)?;
    store.initialize().await?;
    let event = Uuid::new_v4().to_string();
    let other = Uuid::new_v4().to_string();
    store
        .queue_privacy_message(&event, "delivery-1", b"original-signed-entry")
        .await?;
    store
        .queue_privacy_message(&event, "delivery-1", b"retry-must-not-replace")
        .await?;
    store
        .queue_privacy_message(&event, "delivery-2", b"second-signed-entry")
        .await?;
    store
        .queue_privacy_message(&other, "delivery-1", b"another-events-entry")
        .await?;
    let first = store.pending_privacy_messages(&event, 1).await?;
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].delivery_id, "delivery-1");
    assert_eq!(first[0].message, b"original-signed-entry");
    assert_eq!(
        store
            .remove_pending_privacy_messages(&other, &[first[0].id])
            .await?,
        0
    );
    assert_eq!(
        store
            .remove_pending_privacy_messages(&event, &[first[0].id])
            .await?,
        1
    );
    assert_eq!(
        store
            .remove_pending_privacy_messages(&event, &[first[0].id])
            .await?,
        0
    );
    let next = store.pending_privacy_messages(&event, 10).await?;
    assert_eq!(next.len(), 1);
    assert_eq!(next[0].delivery_id, "delivery-2");
    assert_eq!(next[0].message, b"second-signed-entry");
    let other_rows = store.pending_privacy_messages(&other, 10).await?;
    assert_eq!(other_rows.len(), 1);
    assert_eq!(other_rows[0].message, b"another-events-entry");
    for limit in [0, 10_001] {
        assert_eq!(
            store
                .pending_privacy_messages(&event, limit)
                .await
                .unwrap_err()
                .to_string(),
            "Invalid privacy message batch size"
        );
    }
    for (delivery, message) in [("", &b"entry"[..]), ("delivery", &b""[..])] {
        assert_eq!(
            store
                .queue_privacy_message(&event, delivery, message)
                .await
                .unwrap_err()
                .to_string(),
            "Empty privacy audit message"
        );
    }
    store
        .remove_pending_privacy_messages(&event, &[next[0].id])
        .await?;
    store
        .remove_pending_privacy_messages(&other, &[other_rows[0].id])
        .await?;
    assert!(store.pending_privacy_messages(&event, 10).await?.is_empty());
    Ok(())
}

#[tokio::test]
#[ignore = "needs isolated ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn private_receipt_documents_require_exact_event_document_and_pseudonym() -> Result<()> {
    let store = PostgresStore::new(std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")?.parse()?)?;
    store.initialize().await?;
    let event = Uuid::new_v4().to_string();
    let other_event = Uuid::new_v4().to_string();
    let document = Uuid::new_v4().to_string();
    let other_document = Uuid::new_v4().to_string();
    store
        .register_receipt_document(&event, &document, "7QKM-2HXD")
        .await?;
    assert!(
        store
            .owns_receipt_document(&event, &document, "7QKM-2HXD")
            .await?
    );
    assert!(
        !store
            .owns_receipt_document(&event, &document, "ABCD-EFGH")
            .await?
    );
    assert!(
        !store
            .owns_receipt_document(&other_event, &document, "7QKM-2HXD")
            .await?
    );
    assert!(
        !store
            .owns_receipt_document(&event, &other_document, "7QKM-2HXD")
            .await?
    );
    assert!(store
        .register_receipt_document(&event, &document, "ABCD-EFGH")
        .await
        .is_err());
    assert!(
        store
            .owns_receipt_document(&event, &document, "7QKM-2HXD")
            .await?
    );
    Ok(())
}
