// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use chrono::Utc;
use electoral_log::adapters::ballot_box::{AcceptReceived, BallotStatus};
use electoral_log::adapters::ballot_box_received::{
    ReceiveBallot, ReceiveOutcome, ReceivedSignature,
};
use electoral_log::adapters::privacy::BallotIdentity;
use sequent_core::ballot_receipt::{
    ballot_box_key, sign_cast_receipt, sign_cast_statement, sign_received_ballot,
    verify_cast_receipt, verify_received_ballot, CastReceiptStatement, ReceivedStatement,
};
use voting_crypto::signing::VerificationKey;
use voting_crypto::RecordDeserialize;

/// The real writer must publish exactly one binding and one cast, including
/// recovery when a cast appears after the received-outbox empty read.
#[tokio::test]
#[ignore = "requires ELECTORAL_LOG_TEST_DATABASE_URL"]
async fn privacy_sequencer_orders_bindings_and_deduplicates_recovery() -> Result<()> {
    let store = PostgresStore::new(std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")?.parse()?)?;
    store.initialize().await?;
    for simulate_queue_race in [false, true] {
        let tenant = uuid::Uuid::new_v4().to_string();
        let event = uuid::Uuid::new_v4().to_string();
        let election = uuid::Uuid::new_v4().to_string();
        let area = uuid::Uuid::new_v4().to_string();
        let board = format!("test-private-sequencer-{event}");
        let real_voter = "private-voter-must-never-be-published".to_string();
        store.create_board(&board).await?;
        store.create_ballot_box(&event).await?;
        store
            .set_ballot_identity(&event, BallotIdentity::Pseudonym)
            .await?;
        store
            .allocate_voter_pseudonyms(&event, &[real_voter.clone()])
            .await?;
        let pseudonym = store
            .voter_pseudonym(&event, &real_voter)
            .await?
            .context("mapping")?;
        let authority_key = SigningKey::generate()?;
        let receipt_key = SigningKey::generate()?;
        let voter_key = SigningKey::generate()?;
        let published_key = ballot_box_key(&VerificationKey::from_sk(&receipt_key)?)?;
        let now = Utc::now();
        let timestamp = crate::services::receive_ballot::format_signed_time(&now);
        let encrypted = "opaque-encrypted-ballot";
        let ballot_hash = hex::encode(voting_crypto::hash::hash_to_array(encrypted.as_bytes())?);
        let voter_pk = VerificationKey::from_sk(&voter_key)?.to_b64_string()?;
        let signature = voter_key.sign(encrypted.as_bytes())?;
        let voter_signature_bytes = signature.to_bytes();
        let voter_signature = signature.to_b64_string()?;
        let received_receipt = sign_received_ballot(
            &receipt_key,
            ReceivedStatement {
                tenant_id: tenant.clone(),
                election_event_id: event.clone(),
                election_id: election.clone(),
                ballot_hash: ballot_hash.clone(),
                voter_signing_pk: voter_pk.clone(),
                voter_ballot_signature: voter_signature.clone(),
                received_at: timestamp.clone(),
                key_id: published_key.key_id.clone(),
            },
        )?;
        verify_received_ballot(&published_key, &received_receipt)?;
        let received = match store
            .receive_ballot(
                &ReceiveBallot {
                    election_event_id: &event,
                    election_id: &election,
                    area_id: &area,
                    voter_id: &pseudonym,
                    ballot_hash: &ballot_hash,
                    content: encrypted,
                    voter_signing_pk: &voter_pk,
                    voter_ballot_signature: &voter_signature,
                },
                now,
                |_| {
                    Ok(ReceivedSignature {
                        ballot_id: received_receipt.ballot_id.clone(),
                        key_id: published_key.key_id.clone(),
                        received_signature: received_receipt.received_signature.clone(),
                    })
                },
            )
            .await?
        {
            ReceiveOutcome::Received(ballot) => ballot,
            outcome => anyhow::bail!("Unexpected receive outcome {outcome:?}"),
        };
        let cast_signature = sign_cast_statement(&voter_key, &election, &received.ballot_id)?;
        let cast_receipt = sign_cast_receipt(
            &receipt_key,
            CastReceiptStatement {
                election_event_id: event.clone(),
                election_id: election.clone(),
                ballot_id: received.ballot_id.clone(),
                received_at: timestamp.clone(),
                cast_at: timestamp,
                key_id: published_key.key_id.clone(),
                cast_signature: cast_signature.clone(),
            },
        )?;
        verify_cast_receipt(&published_key, &cast_receipt)?;
        let hash = voting_crypto::hash::hash_to_array(pseudonym.as_bytes())?;
        let outcome = store
            .accept_received_ballot(
                &AcceptBallot {
                    election_event_id: &event,
                    election_id: &election,
                    area_id: &area,
                    voter_id: &pseudonym,
                    ballot_id: &ballot_hash,
                    format: "hashable-ballot",
                    content: encrypted,
                    voter_signature: Some(&voter_signature_bytes),
                    pseudonym_hash: &hash,
                    ballot_hash: &hash,
                    voting_channel: "ONLINE",
                    status: BallotStatus::Valid,
                    voter_ip: None,
                    voter_country: None,
                    username: None,
                    allowed_votes: 2,
                },
                &AcceptReceived {
                    ballot_id: &received.ballot_id,
                    cast_at: now,
                    cast_signature: &cast_signature,
                    cast_receipt_signature: &cast_receipt.cast_receipt_signature,
                },
            )
            .await?;
        assert!(matches!(outcome, AcceptOutcome::Accepted { .. }));
        let queued = store.pending_received_ballots(&event, 100).await?;
        assert_eq!(queued.len(), 1);
        if simulate_queue_race {
            // Model the writer's queue-empty observation before receive+cast
            // committed: the writer must reconstruct the binding from the cast.
            store
                .remove_pending_received(&event, &[queued[0].id])
                .await?;
        }
        append_pending(&store, &board, &tenant, &event, &authority_key).await?;
        let records = store
            .query(
                &board,
                &electoral_log::LogQuery {
                    order: vec![(
                        electoral_log::OrderColumn::Id,
                        electoral_log::SortDirection::Asc,
                    )],
                    ..Default::default()
                },
            )
            .await?;
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].statement_kind, "BallotReceived");
        // Existing CastVote queries retain their header-kind compatibility;
        // the signed body carries the new privacy evidence format.
        assert_eq!(records[1].statement_kind, "CastVote");
        let cast_message = Message::record_deserialize(&records[1].message)?;
        assert!(matches!(
            cast_message.statement.body,
            electoral_log::messages::statement::StatementBody::PrivacyCastVote(_)
        ));
        for record in &records {
            assert!(record.user_id.is_none());
            assert_eq!(record.username.as_deref(), Some(pseudonym.as_str()));
            let message = Message::record_deserialize(&record.message)?;
            assert!(message.user_id.is_none());
            assert_eq!(message.sender.name, pseudonym);
            assert!(!serde_json::to_string(&message)?.contains(&real_voter));
        }
        if simulate_queue_race {
            // Its delayed outbox delivery uses the same stable delivery ID.
            store.client().await?.execute(
                "INSERT INTO sequent_backend.ballot_evidence_pending (election_event_id, received_id, kind)
                 VALUES ($1::text::uuid, $2::text::uuid, 'received')",
                &[&event, &received.id],
            ).await?;
        }
        append_pending(&store, &board, &tenant, &event, &authority_key).await?;
        assert_eq!(
            store
                .query(&board, &electoral_log::LogQuery::default())
                .await?
                .len(),
            2
        );
        assert_eq!(store.pending_count(&event).await?, 0);
        assert_eq!(
            append_pending(&store, &board, &tenant, &event, &authority_key).await?,
            0
        );
        let audit = store.audit(&board, &[]).await?;
        assert!(audit.is_clean(), "{:?}", audit.findings());
        store.delete_board(&board).await?;
        store.drop_ballot_box(&event).await?;
    }
    Ok(())
}
