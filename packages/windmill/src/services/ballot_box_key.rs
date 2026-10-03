// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The election event's ballot box key. It signs what voters send, so it is
//! kept apart from the protocol manager key that signs the event's log.

use crate::services::election_event_board::get_election_event_board;
use crate::services::vault;
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::{BallotBoxKey, ReceiptsPolicy};
use sequent_core::ballot_receipt::ballot_box_key;
use sequent_core::types::hasura::core::ElectionEvent;
use strand::signature::{StrandSignaturePk, StrandSignatureSk};
use tracing::instrument;

pub fn get_ballot_box_secret_path(board_name: &str) -> String {
    format!("boards/{board_name}/ballot-box")
}

fn event_board_name(election_event: &ElectionEvent) -> Result<String> {
    get_election_event_board(election_event.bulletin_board_reference.clone())
        .with_context(|| "missing bulletin board")
}

pub fn receipts_policy(election_event: &ElectionEvent) -> Result<ReceiptsPolicy> {
    Ok(election_event
        .get_presentation()
        .map_err(|err| anyhow!("Error parsing election event presentation: {err}"))?
        .unwrap_or_default()
        .receipts_policy())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn get_ballot_box_signing_key(
    hasura_transaction: &Transaction<'_>,
    election_event: &ElectionEvent,
) -> Result<Option<StrandSignatureSk>> {
    let secret_path = get_ballot_box_secret_path(&event_board_name(election_event)?);
    vault::read_secret(
        hasura_transaction,
        &election_event.tenant_id,
        Some(&election_event.id),
        &secret_path,
    )
    .await?
    .map(|der| {
        StrandSignatureSk::from_der_b64_string(&der)
            .map_err(|err| anyhow!("Error reading the ballot box key: {err}"))
    })
    .transpose()
}

/// Returns the event's ballot box key, creating it the first time. The secret
/// key is unique, so of two concurrent creations one fails and neither
/// overwrites a key that has already signed.
#[instrument(skip(hasura_transaction), err)]
pub async fn get_or_create_ballot_box_signing_key(
    hasura_transaction: &Transaction<'_>,
    election_event: &ElectionEvent,
) -> Result<StrandSignatureSk> {
    if let Some(signing_key) =
        get_ballot_box_signing_key(hasura_transaction, election_event).await?
    {
        return Ok(signing_key);
    }

    let signing_key = StrandSignatureSk::generate()
        .map_err(|err| anyhow!("Error generating the ballot box key: {err}"))?;
    let der = signing_key
        .to_der_b64_string()
        .map_err(|err| anyhow!("Error serializing the ballot box key: {err}"))?;
    vault::save_secret(
        hasura_transaction,
        &election_event.tenant_id,
        Some(&election_event.id),
        &get_ballot_box_secret_path(&event_board_name(election_event)?),
        &der,
    )
    .await?;
    Ok(signing_key)
}

pub fn published_key(signing_key: &StrandSignatureSk) -> Result<BallotBoxKey> {
    let public_key = StrandSignaturePk::from_sk(signing_key)
        .map_err(|err| anyhow!("Error deriving the ballot box public key: {err}"))?;
    ballot_box_key(&public_key).map_err(|err| anyhow!("{err}"))
}

/// What a ballot publication tells voters' devices about the ballot box key:
/// nothing unless the event's receipts are signed by the ballot box.
#[instrument(skip(hasura_transaction), err)]
pub async fn ballot_box_key_for_publication(
    hasura_transaction: &Transaction<'_>,
    election_event: &ElectionEvent,
) -> Result<Option<BallotBoxKey>> {
    match receipts_policy(election_event)? {
        ReceiptsPolicy::DISABLED => Ok(None),
        ReceiptsPolicy::SIGNED_BY_BALLOT_BOX => {
            let signing_key =
                get_or_create_ballot_box_signing_key(hasura_transaction, election_event).await?;
            published_key(&signing_key).map(Some)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn election_event(presentation: Option<serde_json::Value>) -> ElectionEvent {
        ElectionEvent {
            id: "event".into(),
            created_at: None,
            updated_at: None,
            labels: None,
            annotations: None,
            tenant_id: "tenant".into(),
            description: None,
            presentation,
            bulletin_board_reference: None,
            is_archived: false,
            voting_channels: None,
            status: None,
            user_boards: None,
            encryption_protocol: "RSA256".into(),
            is_audit: None,
            audit_election_event_id: None,
            public_key: None,
            statistics: None,
            external_id: None,
        }
    }

    #[test]
    fn the_ballot_box_key_has_its_own_secret() {
        assert_eq!(
            get_ballot_box_secret_path("tenant-event"),
            "boards/tenant-event/ballot-box"
        );
    }

    #[test]
    fn receipts_are_off_unless_the_event_turns_them_on() {
        for presentation in [
            None,
            Some(json!({})),
            Some(json!({"receipts": {}})),
            Some(json!({"receipts": {"policy": "disabled"}})),
        ] {
            assert_eq!(
                receipts_policy(&election_event(presentation)).unwrap(),
                ReceiptsPolicy::DISABLED
            );
        }
        assert_eq!(
            receipts_policy(&election_event(Some(
                json!({"receipts": {"policy": "signed-by-ballot-box"}})
            )))
            .unwrap(),
            ReceiptsPolicy::SIGNED_BY_BALLOT_BOX
        );
    }

    #[test]
    fn an_unknown_receipts_policy_is_an_error_not_disabled() {
        assert!(receipts_policy(&election_event(Some(
            json!({"receipts": {"policy": "sometimes"}})
        )))
        .is_err());
    }

    #[test]
    fn the_published_key_names_the_signing_key() {
        let signing_key = StrandSignatureSk::generate().unwrap();
        let key = published_key(&signing_key).unwrap();

        assert_eq!(key.key_id.len(), 16);
        assert_eq!(
            key.public_key,
            StrandSignaturePk::from_sk(&signing_key)
                .unwrap()
                .to_der_b64_string()
                .unwrap()
        );
    }
}
