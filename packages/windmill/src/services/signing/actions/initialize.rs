// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Initializing voting at a Post (A2): creating its initialization report
//! tally (`/create-tally-ceremony` with the type `INITIALIZATION_REPORT`).
//! The request signs the Post's published ballot publication
//! (`{publication_id}`); a Post without one can't be initialized.

use super::{event_ids, gate, refuse, subject_of, EffectProgress};
use crate::domain::tally_ceremony::TallyValidationError;
use crate::postgres::signing::SigningRequestRow;
use crate::postgres::signing_actions::published_publication_of_post;
use crate::services::ceremonies::tally_ceremony::create_tally_ceremony;
use crate::services::signing::guard::{GuardOutcome, GuardRequest, RequestScope};
use crate::services::signing::{InvalidReason, SigningCaller, SigningError, SigningResult};
use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::signing::{InitializeVotingSubject, SigningAction};
use sequent_core::types::ceremonies::TallyType;
use serde_json::{json, Value};
use uuid::Uuid;

/// Whether a tally of `tally_type` is the initialization of voting.
pub fn is_initialization(tally_type: &str) -> bool {
    matches!(
        TallyType::try_from(tally_type),
        Ok(TallyType::INITIALIZATION_REPORT)
    )
}

/// Whether creating a tally session runs now; see [`super::gate`]. Only
/// initialization report tallies are protected, one Post at a time, with
/// the event's tally configuration, once the Post is published.
pub async fn gate_tally_creation(
    hasura_transaction: &Transaction<'_>,
    caller: &SigningCaller,
    tenant_id: &str,
    election_event_id: &str,
    election_ids: &[String],
    tally_type: &str,
    has_configuration: bool,
) -> SigningResult<GuardOutcome> {
    if !is_initialization(tally_type) {
        return Ok(GuardOutcome::Proceed);
    }
    let Some((tenant_id, election_event_id)) = event_ids(tenant_id, election_event_id) else {
        return Ok(GuardOutcome::Proceed);
    };
    gate(
        hasura_transaction,
        caller,
        SigningAction::InitializeVoting,
        tenant_id,
        election_event_id,
        || async move {
            let [election_id] = election_ids else {
                return Err(SigningError::bad_input(
                    "Initialize one Post at a time while initializing needs signatures.",
                ));
            };
            if has_configuration {
                return Err(SigningError::bad_input(
                    "An initialization that needs signatures uses the event's tally configuration.",
                ));
            }
            let election_id = Uuid::parse_str(election_id)
                .map_err(|_| SigningError::bad_input("The Post id is not a UUID."))?;
            let publication_id = published_publication_of_post(
                hasura_transaction,
                tenant_id,
                election_event_id,
                election_id,
            )
            .await?
            .ok_or_else(|| {
                SigningError::invalid(
                    InvalidReason::NoPublication,
                    "The Post has no published ballot publication to initialize.",
                )
            })?;
            Ok(GuardRequest {
                action: SigningAction::InitializeVoting,
                scope: RequestScope {
                    tenant_id,
                    election_event_id,
                    election_id: Some(election_id),
                    area_id: None,
                    trustee_id: None,
                    subject_key: None,
                },
                subject: serde_json::to_value(InitializeVotingSubject {
                    publication_id: Some(publication_id.to_string()),
                })
                .map_err(anyhow::Error::from)?,
                document: None,
                config_revision: None,
            })
        },
    )
    .await
}

/// The effect of a signed initialization: the initialization report tally
/// of the Post, created for the person who started it, once its publication
/// is still the one signed.
///
/// The tally's labels are its Post's, as when the route creates it: the
/// Post's label is the only filter passed, and it passes.
pub async fn create_report_tally(
    hasura_transaction: &Transaction<'_>,
    request: &SigningRequestRow,
    progress: &EffectProgress,
) -> Result<Value> {
    let subject: InitializeVotingSubject = subject_of(request)?;
    let election_id = request
        .election_id
        .ok_or_else(|| anyhow!("signing request {} has no Post", request.id))?;
    let published = published_publication_of_post(
        hasura_transaction,
        request.tenant_id,
        request.election_event_id,
        election_id,
    )
    .await?
    .map(|id| id.to_string());
    if published.is_none() || published != subject.publication_id {
        return Err(refuse(
            "publication-changed",
            format!(
                "The Post's publication changed since signing request {} started.",
                request.code
            ),
        ));
    }
    let labels: Vec<String> = request.permission_label.iter().cloned().collect();
    // Creating the tally ends with its entry on the bulletin board.
    progress.reach_outside();
    let tally_session_id = create_tally_ceremony(
        hasura_transaction,
        request.tenant_id.to_string(),
        &request.requested_by,
        request.election_event_id.to_string(),
        vec![election_id.to_string()],
        None,
        TallyType::INITIALIZATION_REPORT.to_string(),
        &labels,
        request.requested_by_username.clone(),
    )
    .await
    .map_err(|error| match error.downcast_ref::<TallyValidationError>() {
        // Refused before it acted.
        Some(invalid) => refuse("tally-invalid", invalid.to_string()),
        None => error,
    })?;
    Ok(json!({ "tally_session_id": tally_session_id }))
}
