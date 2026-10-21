// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The authenticated identity stops here. Ballot storage only receives the
//! event pseudonym; allocation belongs to publication and registry changes.

use crate::postgres::election_event::get_election_event_by_id;
use crate::services::database::get_keycloak_pool;
use crate::services::electoral_log_board::get_event_store;
use crate::services::external::utils::datafix_annotations;
use anyhow::{ensure, Context, Result};
use deadpool_postgres::Transaction;
use electoral_log::adapters::privacy::BallotIdentity as StoredIdentity;
use sequent_core::ballot::{ReceiptsPolicy, VoterSigningPolicy, VotingStatusChannel};
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::vote_privacy::BallotIdentity;

/// Defaults apply only to newly created events, never to imported historical
/// configurations. Datafix's reconciliation remains keyed by voter ID.
pub fn configure_new_event(event: &mut ElectionEvent) -> Result<()> {
    let mode = if datafix_annotations(event)?.is_some() {
        BallotIdentity::VoterId
    } else {
        BallotIdentity::Pseudonym
    };
    let presentation = event
        .presentation
        .get_or_insert_with(|| serde_json::json!({}));
    let object = presentation
        .as_object_mut()
        .context("Event presentation must be an object")?;
    object.insert("ballot_identity".into(), serde_json::to_value(mode)?);
    object
        .entry("network_data_policy")
        .or_insert(serde_json::json!("do-not-store"));
    if mode == BallotIdentity::Pseudonym {
        object.insert(
            "voter_signing_policy".into(),
            serde_json::json!("with-signature"),
        );
        let receipts = object
            .entry("receipts")
            .or_insert_with(|| serde_json::json!({}));
        if receipts.is_null() {
            *receipts = serde_json::json!({});
        }
        receipts
            .as_object_mut()
            .context("Receipt settings must be an object")?
            .insert("policy".into(), serde_json::json!("signed-by-ballot-box"));
    }
    Ok(())
}

pub fn identity_mode(event: &ElectionEvent) -> Result<BallotIdentity> {
    let presentation = event.get_presentation()?.unwrap_or_default();
    let mode = presentation.ballot_identity.unwrap_or_default();
    ensure!(
        mode != BallotIdentity::Pseudonym || datafix_annotations(event)?.is_none(),
        "Pseudonymous events cannot enable the external voter reconciliation integration"
    );
    Ok(mode)
}

pub fn require_signed_receipts(
    event: &ElectionEvent,
    channel: Option<VotingStatusChannel>,
) -> Result<()> {
    if identity_mode(event)? == BallotIdentity::Pseudonym {
        let presentation = event.get_presentation()?.unwrap_or_default();
        ensure!(
            presentation.receipts_policy() == ReceiptsPolicy::SIGNED_BY_BALLOT_BOX
                && presentation.voter_signing_policy == Some(VoterSigningPolicy::WITH_SIGNATURE),
            "Voter pseudonyms require signed ballot-box receipts and voter signatures"
        );
        ensure!(
            channel != Some(VotingStatusChannel::TELEPHONE),
            "This voting channel does not support signed pseudonymous ballots"
        );
    }
    Ok(())
}

/// Does not create mappings, including after their retention period has ended.
pub async fn ballot_identity(event: &ElectionEvent, voter_id: &str) -> Result<String> {
    if identity_mode(event)? == BallotIdentity::VoterId {
        return Ok(voter_id.to_owned());
    }
    get_event_store(&event.id)
        .await?
        .voter_pseudonym(&event.id, voter_id)
        .await?
        .context("The voter privacy registration is not ready")
}

/// Publication holds its existing registry/publication admission lock while
/// allocation visits the authoritative voter group in bounded keyset batches.
pub async fn allocate_registry(tx: &Transaction<'_>, tenant: &str, event_id: &str) -> Result<()> {
    let event = get_election_event_by_id(tx, tenant, event_id).await?;
    if identity_mode(&event)? == BallotIdentity::VoterId {
        return Ok(());
    }
    require_signed_receipts(&event, None)?;
    ensure!(event.voting_channels.as_ref().and_then(|channels| channels.get("telephone")).and_then(serde_json::Value::as_bool) != Some(true),
        "Disable telephone voting before publishing a pseudonymous event: this channel cannot produce the required voter signatures");
    let child_telephone: bool = tx.query_one(
        "SELECT EXISTS (SELECT 1 FROM sequent_backend.election WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid AND voting_channels->>'telephone'='true')",
        &[&tenant, &event_id],
    ).await?.get(0);
    ensure!(
        !child_telephone,
        "Disable telephone voting on every child election before publishing a pseudonymous event"
    );
    let store = get_event_store(event_id).await?;
    store
        .set_ballot_identity(event_id, StoredIdentity::Pseudonym)
        .await?;
    let realm = get_event_realm(tenant, event_id);
    let group = std::env::var("KEYCLOAK_VOTER_GROUP_NAME")
        .context("KEYCLOAK_VOTER_GROUP_NAME is required to allocate voter pseudonyms")?;
    let client = get_keycloak_pool().await.get().await?;
    let mut after = String::new();
    loop {
        let rows = client.query(
            "SELECT u.id FROM user_entity u JOIN realm r ON r.id = u.realm_id \
             WHERE r.name = $1 AND u.id > $2 AND u.service_account_client_link IS NULL \
             AND EXISTS (SELECT 1 FROM user_group_membership m JOIN keycloak_group g ON g.id=m.group_id \
                         WHERE m.user_id=u.id AND g.realm_id=u.realm_id AND g.name=$3) \
             ORDER BY u.id LIMIT 5000", &[&realm, &after, &group]).await?;
        if rows.is_empty() {
            break;
        }
        let ids = rows
            .iter()
            .map(|row| row.try_get::<_, String>(0))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if let Some(last) = ids.last() {
            after = last.clone();
        }
        store.allocate_voter_pseudonyms(event_id, &ids).await?;
    }
    Ok(())
}

/// New voters in a published event are registered before the create action
/// completes. Unpublished events allocate their complete census at publication.
pub async fn allocate_added_voter(
    tx: &Transaction<'_>,
    tenant: &str,
    event_id: &str,
    voter_id: &str,
) -> Result<()> {
    // Serialize with publication after Keycloak has committed the new voter:
    // either its scan sees the voter, or this hook observes publication and
    // allocates before returning. No receive-time allocation is needed.
    crate::postgres::ballot_publication::lock_publication_event(tx, tenant, event_id).await?;
    let event = get_election_event_by_id(tx, tenant, event_id).await?;
    if identity_mode(&event)? != BallotIdentity::Pseudonym {
        return Ok(());
    }
    let published: bool = tx.query_one(
        "SELECT EXISTS (SELECT 1 FROM sequent_backend.ballot_publication WHERE tenant_id=$1::text::uuid \
         AND election_event_id=$2::text::uuid AND published_at IS NOT NULL AND deleted_at IS NULL)",
        &[&tenant, &event_id]).await?.try_get(0)?;
    if published {
        let store = get_event_store(event_id).await?;
        store
            .set_ballot_identity(event_id, StoredIdentity::Pseudonym)
            .await?;
        store
            .allocate_voter_pseudonyms(event_id, &[voter_id.to_owned()])
            .await?;
    }
    Ok(())
}

/// Signed entries never contain private table keys or network attribution.
pub fn received_log_entry(
    tenant: &str,
    pending: &electoral_log::adapters::privacy::PendingReceivedBallot,
    key: &voting_crypto::signing::SigningKey,
) -> Result<electoral_log::LogEntry> {
    use electoral_log::adapters::privacy::ReceivedEvidenceKind;
    use electoral_log::messages::{message::Message, privacy::*};
    let ballot = &pending.ballot;
    let hash = hex::encode(voting_crypto::hash::hash_to_array(
        ballot.voter_id.as_bytes(),
    )?);
    let message = match pending.kind {
        ReceivedEvidenceKind::Received => Message::ballot_received_message(
            &ballot.voter_id,
            BallotReceivedEvidence {
                tenant_id: tenant.into(),
                election_event_id: ballot.election_event_id.clone(),
                election_id: ballot.election_id.clone(),
                pseudonym_hash: hash,
                voter_signing_pk: ballot.voter_signing_pk.clone(),
                received_ballot_json: serde_json::to_string(
                    &crate::services::receive_ballot::stored_receipt(
                        &uuid::Uuid::parse_str(tenant)?,
                        ballot.clone(),
                    ),
                )?,
                encrypted_ballot: ballot.content.clone(),
            },
            key,
        )?,
        ReceivedEvidenceKind::Audited => Message::private_ballot_event_message_at(
            &ballot.election_event_id,
            &ballot.voter_id,
            PrivateBallotEvent {
                election_id: ballot.election_id.clone(),
                pseudonym_hash: hash,
                ballot_id: Some(ballot.ballot_id.clone()),
                outcome: PrivateBallotOutcome::Audited,
            },
            pending
                .queued_at
                .timestamp()
                .try_into()
                .context("Invalid ballot audit time")?,
            key,
        )?,
    };
    Ok(electoral_log::LogEntry {
        delivery_id: format!(
            "ballot-evidence:{}:{}:{}",
            ballot.election_event_id,
            ballot.id,
            pending.kind.as_str()
        ),
        message: (&message).try_into()?,
    })
}

pub fn cast_log_entry(
    received: &electoral_log::adapters::ballot_box_received::ReceivedBallot,
    tenant: &str,
    event: &str,
    ballot: &electoral_log::adapters::ballot_box::PendingBallot,
    key: &voting_crypto::signing::SigningKey,
) -> Result<electoral_log::LogEntry> {
    use electoral_log::messages::{message::Message, privacy::BallotCastEvidence};
    ensure!(
        received.election_event_id == event
            && received.election_id == ballot.election_id
            && received.voter_id == ballot.voter_id
            && received.ballot_hash == ballot.ballot_id,
        "The received ballot must match the cast scope"
    );
    let receipt = crate::services::cast_ballot::stored_receipt(&received)
        .context("A pseudonymous cast must have a cast receipt")?;
    let message = Message::privacy_cast_vote_message(
        &ballot.voter_id,
        BallotCastEvidence {
            tenant_id: tenant.into(),
            election_event_id: event.into(),
            election_id: ballot.election_id.clone(),
            pseudonym_hash: hex::encode(voting_crypto::hash::hash_to_array(
                ballot.voter_id.as_bytes(),
            )?),
            ballot_id: received.ballot_id.clone(),
            cast_receipt_json: serde_json::to_string(&receipt)?,
            voting_channel: ballot.voting_channel.clone(),
        },
        key,
    )?;
    Ok(electoral_log::LogEntry {
        delivery_id: crate::services::ballot_box::ballot_delivery_id(event, ballot.seq),
        message: (&message).try_into()?,
    })
}

pub async fn queue_message(
    event: &str,
    delivery_id: &str,
    message: &electoral_log::messages::message::Message,
) -> Result<()> {
    use voting_crypto::RecordSerialize;
    let dbs = crate::services::electoral_log_board::get_event_databases().await?;
    dbs.mark_ballot_activity(
        event,
        electoral_log::adapters::events::ActivityMark::Throttled,
    )
    .await?;
    dbs.store(event)
        .await?
        .queue_privacy_message(event, delivery_id, &message.record_serialize()?)
        .await
}

pub async fn queue_rejection(
    tenant: &str,
    event: &str,
    election: &str,
    pseudonym: &str,
) -> Result<()> {
    use electoral_log::messages::{message::Message, privacy::*};
    let mut client = crate::services::database::get_hasura_pool()
        .await
        .get()
        .await?;
    let tx = client.transaction().await?;
    let event_row = get_election_event_by_id(&tx, tenant, event).await?;
    let board = crate::services::election_event_board::get_election_event_board(
        event_row.bulletin_board_reference,
    )
    .context("Event log unavailable")?;
    let key = crate::services::electoral_log_board::get_board_signing_key(
        &tx,
        tenant,
        Some(event),
        &board,
    )
    .await?;
    let message = Message::private_ballot_event_message(
        event,
        pseudonym,
        PrivateBallotEvent {
            election_id: election.into(),
            pseudonym_hash: hex::encode(voting_crypto::hash::hash_to_array(pseudonym.as_bytes())?),
            ballot_id: None,
            outcome: PrivateBallotOutcome::Rejected,
        },
        &key,
    )?;
    queue_message(
        event,
        &format!("ballot-refused:{}", uuid::Uuid::new_v4()),
        &message,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(presentation: serde_json::Value) -> ElectionEvent {
        serde_json::from_value(serde_json::json!({
            "id":"00000000-0000-4000-8000-000000000001",
            "tenant_id":"00000000-0000-4000-8000-000000000002",
            "is_archived":false,"encryption_protocol":"WBRAID","presentation":presentation,
        }))
        .unwrap()
    }
    #[test]
    fn new_events_require_signed_receipts_but_legacy_events_keep_identity() {
        let mut row = event(serde_json::Value::Null);
        assert_eq!(identity_mode(&row).unwrap(), BallotIdentity::VoterId);
        configure_new_event(&mut row).unwrap();
        assert_eq!(identity_mode(&row).unwrap(), BallotIdentity::Pseudonym);
        require_signed_receipts(&row, Some(VotingStatusChannel::ONLINE)).unwrap();
        assert!(require_signed_receipts(&row, Some(VotingStatusChannel::TELEPHONE)).is_err());
        assert_eq!(
            row.presentation.as_ref().unwrap()["network_data_policy"],
            "do-not-store"
        );
        row.presentation.as_mut().unwrap()["receipts"]["policy"] = serde_json::json!("disabled");
        assert!(require_signed_receipts(&row, None).is_err());
    }
    #[test]
    fn privacy_defaults_preserve_explicit_network_policy_and_other_settings() {
        let mut row = event(
            serde_json::json!({"network_data_policy":"forensic-store","show_cast_vote_logs":"show-logs-tab"}),
        );
        configure_new_event(&mut row).unwrap();
        assert_eq!(
            row.presentation.as_ref().unwrap()["network_data_policy"],
            "forensic-store"
        );
        assert_eq!(
            row.presentation.as_ref().unwrap()["show_cast_vote_logs"],
            "show-logs-tab"
        );
    }
}
