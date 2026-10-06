// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Publication of an event imported from a signed configuration package:
//! each ballot style about to be published must be the design the package's
//! manifest approved. The digest is the one the Election Architect computed
//! before signing, by the same function, so an event edited after import is
//! refused here rather than published.

use anyhow::{anyhow, Result};
use deadpool_postgres::Transaction;
use sequent_core::ballot::BallotStyle;
use sequent_core::election_config::design::{
    ballot_design_digests, mismatches, DesignDigest, DesignKeys, DesignMismatch,
};
use sequent_core::serialization::deserialize_with_path::deserialize_str;
use tracing::instrument;

use crate::postgres::area::get_event_areas;
use crate::postgres::ballot_style::get_publication_ballot_styles;
use crate::postgres::candidate::export_candidates;
use crate::postgres::configuration_packages::manifest_of_event;
use crate::postgres::contest::export_contests;
use crate::postgres::document::get_event_or_tenant_document_names;
use crate::postgres::election::get_elections;

/// What the check found for a publication of a signed configuration.
#[derive(Debug, Clone)]
pub struct DesignCheck {
    pub external_id: String,
    pub revision: u64,
    pub manifest_sha256: String,
    pub digests: Vec<DesignDigest>,
    pub mismatches: Vec<DesignMismatch>,
}

/// `None` for an event that was not imported from a signed package.
#[instrument(skip(hasura_transaction), err)]
pub async fn check_publication_designs(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    ballot_publication_id: &str,
) -> Result<Option<DesignCheck>> {
    let Some((manifest, manifest_sha256)) =
        manifest_of_event(hasura_transaction, tenant_id, election_event_id).await?
    else {
        return Ok(None);
    };

    let areas = get_event_areas(hasura_transaction, tenant_id, election_event_id).await?;
    let elections = get_elections(hasura_transaction, tenant_id, election_event_id).await?;
    let contests = export_contests(hasura_transaction, tenant_id, election_event_id).await?;
    let candidates = export_candidates(hasura_transaction, tenant_id, election_event_id).await?;
    let mut keys = DesignKeys::of_entities(&areas, &elections, &contests, &candidates)
        .map_err(|problem| anyhow!(problem.message))?;
    let image_ids: Vec<String> = candidates
        .iter()
        .filter_map(|candidate| candidate.image_document_id.clone())
        .collect();
    let image_names = get_event_or_tenant_document_names(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &image_ids,
    )
    .await?;
    for (document_id, name) in &image_names {
        keys = keys.with_document(document_id, name);
    }

    let styles = get_publication_ballot_styles(
        hasura_transaction,
        tenant_id,
        election_event_id,
        ballot_publication_id,
        None,
    )
    .await?
    .into_iter()
    .filter_map(|row| row.ballot_eml)
    .map(|eml| deserialize_str::<BallotStyle>(&eml).map_err(|error| anyhow!("{error}")))
    .collect::<Result<Vec<_>>>()?;
    let digests =
        ballot_design_digests(&styles, &keys).map_err(|problem| anyhow!(problem.message))?;

    Ok(Some(DesignCheck {
        mismatches: mismatches(&manifest.content.ballot_designs, &digests),
        external_id: manifest.configuration.external_id.clone(),
        revision: manifest.configuration.revision,
        manifest_sha256,
        digests,
    }))
}
