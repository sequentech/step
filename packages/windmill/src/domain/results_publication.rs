// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Results website rules: whether a publication still matches the election
//! event's policy, which route it serves, and what a reader may see of it.

use crate::postgres::tally_results_publication::TallyResultsPublication;
use crate::types::results_publication::{
    ResultsPublicationDocuments, ResultsPublicationManifest, ResultsRouteScope,
};
use anyhow::{anyhow, Context, Result};
use sequent_core::ballot::{
    ElectionEventPresentation, ResultsWebsiteAccess, ResultsWebsitePolicy, ResultsWebsiteStatus,
    ResultsWebsiteVisibilityScope,
};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::types::permissions::Permissions;
use thiserror::Error;

const RESULTS_PORTAL_CLIENT_ID: &str = "results-portal";
const LEGACY_RESULTS_PORTAL_CLIENT_ID: &str = "voting-portal";
const NO_VOTER_AREA: &str = "No voter area is available";
const NO_VOTER_AREA_ARTIFACT: &str = "No results artifact is available for this voter area";

/// Harvest maps each variant to an HTTP status.
#[derive(Debug, Error)]
pub enum ResultsPublicationServiceError {
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Unauthorized(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Internal(#[from] anyhow::Error),
}

pub type ResultsPublicationServiceResult<T> =
    std::result::Result<T, ResultsPublicationServiceError>;

fn is_results_portal_client(client_id: &str) -> bool {
    matches!(
        client_id,
        RESULTS_PORTAL_CLIENT_ID | LEGACY_RESULTS_PORTAL_CLIENT_ID
    )
}

pub fn results_website_policy(
    presentation: &ElectionEventPresentation,
) -> Result<Option<ResultsWebsitePolicy>> {
    presentation
        .results_website
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .context("Invalid results website policy")
}

pub fn is_results_website_enabled(presentation: &ElectionEventPresentation) -> Result<bool> {
    Ok(results_website_policy(presentation)?
        .is_some_and(|policy| policy.status == ResultsWebsiteStatus::Enabled))
}

pub fn publication_matches_results_website_policy(
    presentation: &ElectionEventPresentation,
    publication: &TallyResultsPublication,
) -> Result<bool> {
    let Some(policy) = results_website_policy(presentation)? else {
        return Ok(false);
    };

    Ok(policy.status == ResultsWebsiteStatus::Enabled
        && policy.access == publication.access
        && policy.visibility_scope == publication.visibility_scope)
}

pub fn validate_results_website_policy(
    presentation: &ElectionEventPresentation,
    access: ResultsWebsiteAccess,
    visibility_scope: ResultsWebsiteVisibilityScope,
) -> Result<()> {
    let policy = results_website_policy(presentation)?
        .ok_or_else(|| anyhow!("Results website policy is not configured"))?;

    if policy.status != ResultsWebsiteStatus::Enabled {
        return Err(anyhow!(
            "Results website publishing is disabled for this election event"
        ));
    }
    if policy.access != access {
        return Err(anyhow!(
            "Results access does not match the election event results website policy"
        ));
    }
    if policy.visibility_scope != visibility_scope {
        return Err(anyhow!(
            "Results visibility does not match the election event results website policy"
        ));
    }

    Ok(())
}

pub fn publication_documents(
    publication: &TallyResultsPublication,
) -> Result<ResultsPublicationDocuments> {
    serde_json::from_value(publication.documents.clone())
        .context("Invalid stored results publication documents")
}

/// The public manifest path, preferring the `manifest-latest` alias.
pub fn manifest_public_path(publication: &TallyResultsPublication) -> Result<Option<String>> {
    Ok(publication_documents(publication)?
        .manifest
        .and_then(|manifest| manifest.latest_public_path.or(manifest.public_path)))
}

/// Whether the publication serves the requested page: an election route
/// serves only its own election, an event route any of its elections.
pub fn publication_matches_requested_route(
    publication: &TallyResultsPublication,
    election_id: Option<&str>,
) -> bool {
    match publication.route_scope {
        ResultsRouteScope::Election => {
            publication.route_election_id.as_deref().is_some()
                && publication.route_election_id.as_deref() == election_id
        }
        ResultsRouteScope::Event => election_id
            .map(|id| {
                publication
                    .election_ids
                    .iter()
                    .any(|election| election == id)
            })
            .unwrap_or(true),
    }
}

/// Results portal voters need a token from the event realm that authorizes
/// the requested election, or any published election when none is
/// requested. Other clients need a results publication permission.
pub fn authorize_results_reader(
    claims: &JwtClaims,
    election_event_id: &str,
    publication: &TallyResultsPublication,
    requested_election_id: Option<&str>,
) -> ResultsPublicationServiceResult<()> {
    if !is_results_portal_client(&claims.azp) {
        let can_read = claims.hasura_claims.allowed_roles.iter().any(|role| {
            role == &Permissions::PUBLISH_RESULTS_READ.to_string()
                || role == &Permissions::PUBLISH_RESULTS_WRITE.to_string()
        });
        return can_read.then_some(()).ok_or_else(|| {
            ResultsPublicationServiceError::Unauthorized(
                "Missing results publication permission".to_string(),
            )
        });
    }

    let expected_realm = get_event_realm(&claims.hasura_claims.tenant_id, election_event_id);
    let issuer_realm = claims.iss.trim_end_matches('/').rsplit('/').next();
    if issuer_realm != Some(expected_realm.as_str()) {
        return Err(ResultsPublicationServiceError::Forbidden(
            "Token is not valid for this election event".to_string(),
        ));
    }

    let authorized_election_ids = claims
        .hasura_claims
        .authorized_election_ids
        .as_ref()
        .ok_or_else(|| {
            ResultsPublicationServiceError::Forbidden(
                "No authorized elections are available".to_string(),
            )
        })?;
    let is_authorized = match requested_election_id {
        Some(election_id) => {
            authorized_election_ids.iter().any(|id| id == election_id)
                && publication.election_ids.iter().any(|id| id == election_id)
        }
        None => publication
            .election_ids
            .iter()
            .any(|election_id| authorized_election_ids.iter().any(|id| id == election_id)),
    };

    is_authorized.then_some(()).ok_or_else(|| {
        ResultsPublicationServiceError::Forbidden(
            "Not authorized to view these election results".to_string(),
        )
    })
}

/// The stored manifest as a reader may see it: results portal voters of an
/// area-based publication only see their own area's artifact.
pub fn manifest_for_reader(
    publication: &TallyResultsPublication,
    claims: &JwtClaims,
) -> ResultsPublicationServiceResult<Option<ResultsPublicationManifest>> {
    let mut manifest = publication
        .manifest
        .clone()
        .map(serde_json::from_value)
        .transpose()
        .context("Invalid stored results publication manifest")?;
    if publication.visibility_scope != ResultsWebsiteVisibilityScope::AreaBased
        || !is_results_portal_client(&claims.azp)
    {
        return Ok(manifest);
    }

    let area_id = claims
        .hasura_claims
        .area_id
        .as_ref()
        .ok_or_else(|| ResultsPublicationServiceError::Forbidden(NO_VOTER_AREA.to_string()))?;
    let areas = manifest
        .as_mut()
        .and_then(|manifest| manifest.artifacts.areas.as_mut())
        .ok_or_else(|| {
            ResultsPublicationServiceError::NotFound(
                "No area results artifacts are available".to_string(),
            )
        })?;
    let area_document = areas.remove(area_id).ok_or_else(|| {
        ResultsPublicationServiceError::Forbidden(NO_VOTER_AREA_ARTIFACT.to_string())
    })?;
    areas.clear();
    areas.insert(area_id.clone(), area_document);
    Ok(manifest)
}

/// The SQLite documents a reader downloads: the voter area's one for
/// area-based publications, the full event one otherwise.
pub fn artifact_document_ids_for_reader(
    publication: &TallyResultsPublication,
    claims: &JwtClaims,
) -> ResultsPublicationServiceResult<Vec<String>> {
    let documents = publication_documents(publication)?;
    if publication.visibility_scope == ResultsWebsiteVisibilityScope::AreaBased {
        let area_id =
            claims.hasura_claims.area_id.as_ref().ok_or_else(|| {
                ResultsPublicationServiceError::Forbidden(NO_VOTER_AREA.to_string())
            })?;
        documents
            .area_sqlite
            .as_ref()
            .and_then(|areas| areas.get(area_id))
            .and_then(|artifact| artifact.document_id.clone())
            .map(|document_id| vec![document_id])
            .ok_or_else(|| {
                ResultsPublicationServiceError::Forbidden(NO_VOTER_AREA_ARTIFACT.to_string())
            })
    } else {
        documents
            .full_sqlite
            .and_then(|artifact| artifact.document_id)
            .map(|document_id| vec![document_id])
            .ok_or_else(|| {
                ResultsPublicationServiceError::NotFound(
                    "No results artifact is available".to_string(),
                )
            })
    }
}
