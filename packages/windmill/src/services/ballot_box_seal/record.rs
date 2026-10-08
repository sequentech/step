// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The seal record of a ballot box (VOTE-FREEZE), built from the seal row's
//! stored manifest and signed message. The event's Seal Record Publication
//! policy decides whether it is published as a private or a public
//! document; the record is the same. It holds no voter id, no pseudonym, no
//! time of an individual ballot and no network data.

use crate::postgres::area::get_area_by_id;
use crate::postgres::ballot_box_seal::{BallotBoxSeal, ClosedBy};
use crate::services::initialization_scope::post_display_name;
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use electoral_log::seal::{SealRecord, SealRecordCloseRequest, SealRecordNames, SealRecordSigner};
use sequent_core::services::translations::Name;
use sequent_core::types::hasura::core::{Election, ElectionEvent};
use strand::signature::StrandSignaturePk;

/// A name the translations leave out.
const MISSING_NAME: &str = "-";

/// The election's name as the log and the record show it.
pub fn election_name(election_event: &ElectionEvent, election: &Election) -> String {
    post_display_name(election, &election_event.get_default_language())
}

/// The event's name, or its id when it has none.
pub fn event_name(election_event: &ElectionEvent) -> String {
    Some(election_event.get_name(&election_event.get_default_language()))
        .filter(|name| !name.is_empty() && name != MISSING_NAME)
        .unwrap_or_else(|| election_event.id.clone())
}

/// The area's name, or its id when it has none.
pub async fn area_name(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    area_id: &str,
) -> Result<String> {
    Ok(get_area_by_id(hasura_transaction, tenant_id, area_id)
        .await?
        .and_then(|area| area.name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| area_id.to_string()))
}

/// The seal record of a sealed ballot box whose `BallotBoxSealed` entry
/// has id `log_entry_id` in the event's log, signed by `system_pk`.
pub async fn build_record(
    hasura_transaction: &Transaction<'_>,
    seal: &BallotBoxSeal,
    election_event: &ElectionEvent,
    system_pk: &StrandSignaturePk,
    log_entry_id: i64,
) -> Result<SealRecord> {
    let tenant_id = seal.tenant_id.to_string();
    let election_event_id = seal.election_event_id.to_string();
    let election_id = seal.election_id.to_string();
    let manifest = seal
        .manifest
        .as_deref()
        .ok_or_else(|| anyhow!("Ballot box seal {} has no manifest", seal.id))?;
    let signed_message = seal
        .signed_message
        .as_deref()
        .ok_or_else(|| anyhow!("Ballot box seal {} has no signed message", seal.id))?;
    let election = crate::postgres::election::get_election_by_id(
        hasura_transaction,
        &tenant_id,
        &election_event_id,
        &election_id,
    )
    .await?
    .ok_or_else(|| anyhow!("Election {election_id} not found"))?;
    // The names the seal signed; a seal made before they were kept falls
    // back to today's names.
    let names = SealRecordNames {
        election_event: event_name(election_event),
        election: match &seal.election_name {
            Some(name) => name.clone(),
            None => election_name(election_event, &election),
        },
        area: match &seal.area_name {
            Some(name) => name.clone(),
            None => area_name(hasura_transaction, &tenant_id, &seal.area_id.to_string()).await?,
        },
    };
    Ok(SealRecord::from_parts(
        manifest,
        signed_message,
        system_pk,
        names,
        close_request(seal),
        log_entry_id,
    )?)
}

/// The Close voting request that closed the election, when signatures did.
fn close_request(seal: &BallotBoxSeal) -> Option<SealRecordCloseRequest> {
    let id = seal.close_request_id?;
    let (signers, signing_code) = match &seal.closed_by {
        ClosedBy::Signed {
            signers,
            signing_code,
        } => (signers.clone().unwrap_or_default(), signing_code.clone()),
        _ => (vec![], None),
    };
    Some(SealRecordCloseRequest {
        id: id.to_string(),
        signing_code,
        signers: signers
            .into_iter()
            .map(|signer| SealRecordSigner {
                name: signer.name,
                certificate_sha256: signer.certificate_sha256,
            })
            .collect(),
    })
}
