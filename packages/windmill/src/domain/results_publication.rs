// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Results website rules: whether a publication still matches the election
//! event's policy, which route it serves, what a reader may see of it, and
//! whether a new publication's tally source is consistent.

use crate::postgres::tally_results_publication::{
    PublicationSource, PublicationSourceFacts, TallyResultsPublication,
};
use crate::types::results_publication::{
    PublishResultsWebsiteInput, ResultsPublicationDocuments, ResultsPublicationManifest,
    ResultsRouteScope,
};
use anyhow::{anyhow, Context, Result};
use electoral_log::messages::newtypes::{
    ContestIdString, ElectionIdString, ResultsPublicationAccessString, ResultsPublicationAction,
    ResultsPublicationDetails, ResultsPublicationIdString, ResultsPublicationRouteScopeString,
    ResultsPublicationVisibilityScopeString,
};
use sequent_core::ballot::{
    ElectionEventPresentation, ResultsWebsiteAccess, ResultsWebsitePolicy, ResultsWebsiteStatus,
    ResultsWebsiteVisibilityScope,
};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::types::permissions::Permissions;
use thiserror::Error;
use uuid::Uuid;

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

/// The electoral log details of an action on a publication.
pub fn results_publication_log_details(
    publication: &TallyResultsPublication,
    action: ResultsPublicationAction,
) -> ResultsPublicationDetails {
    ResultsPublicationDetails {
        publication_id: ResultsPublicationIdString(publication.id.clone()),
        action,
        route_scope: ResultsPublicationRouteScopeString(publication.route_scope.to_string()),
        route_election_id: ElectionIdString(publication.route_election_id.clone()),
        access: ResultsPublicationAccessString(publication.access.to_string()),
        visibility_scope: ResultsPublicationVisibilityScopeString(
            publication.visibility_scope.to_string(),
        ),
        contest_ids: publication
            .published_contest_ids
            .iter()
            .cloned()
            .map(ContestIdString)
            .collect(),
    }
}

fn parse_uuids(ids: &[String]) -> Result<Vec<Uuid>> {
    ids.iter().map(|id| parse_uuid_v4(id)).collect()
}

/// The tally source of a publication request: at least one election and
/// one contest, and an election route among its elections.
pub fn publication_source(
    tenant_id: &str,
    input: &PublishResultsWebsiteInput,
) -> Result<PublicationSource> {
    publication_source_from_ids(
        tenant_id,
        &input.election_event_id,
        &input.tally_session_id,
        &input.tally_session_execution_id,
        &input.results_event_id,
        &input.election_ids,
        &input.contest_ids,
        input.route_election_id.as_deref(),
    )
}

pub fn publication_source_from_ids(
    tenant_id: &str,
    election_event_id: &str,
    tally_session_id: &str,
    tally_session_execution_id: &str,
    results_event_id: &str,
    election_ids: &[String],
    contest_ids: &[String],
    route_election_id: Option<&str>,
) -> Result<PublicationSource> {
    if election_ids.is_empty() || contest_ids.is_empty() {
        return Err(anyhow!(
            "A publication requires at least one election and one contest"
        ));
    }

    let source = PublicationSource {
        tenant_id: parse_uuid_v4(tenant_id)?,
        election_event_id: parse_uuid_v4(election_event_id)?,
        tally_session_id: parse_uuid_v4(tally_session_id)?,
        tally_session_execution_id: parse_uuid_v4(tally_session_execution_id)?,
        results_event_id: parse_uuid_v4(results_event_id)?,
        election_ids: parse_uuids(election_ids)?,
        contest_ids: parse_uuids(contest_ids)?,
    };
    let route_election_id = route_election_id.map(parse_uuid_v4).transpose()?;
    if route_election_id.is_some_and(|route_id| !source.election_ids.contains(&route_id)) {
        return Err(anyhow!(
            "The route election must be included in the publication elections"
        ));
    }

    Ok(source)
}

/// The execution, session and results event must belong together, and
/// every source election, contest and contest result must exist. The
/// database counts distinct identifiers, so repeated ones fail too.
pub fn check_publication_source(
    source: &PublicationSource,
    facts: &PublicationSourceFacts,
) -> Result<()> {
    let expected_elections = i64::try_from(source.election_ids.len())?;
    let expected_contests = i64::try_from(source.contest_ids.len())?;

    if !facts.valid_execution {
        return Err(anyhow!(
            "The tally session, execution, and results event do not belong together"
        ));
    }
    if facts.election_count != expected_elections {
        return Err(anyhow!(
            "One or more publication elections are outside the tally event"
        ));
    }
    if facts.contest_count != expected_contests {
        return Err(anyhow!(
            "One or more publication contests are outside the selected elections"
        ));
    }
    if facts.tallied_contest_count != expected_contests {
        return Err(anyhow!(
            "Every selected contest must have results in the selected tally execution"
        ));
    }

    Ok(())
}

/// Synthetic readers and publications for tenant `TENANT_ID` and event
/// `ELECTION_EVENT_ID`.
#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;
    use crate::types::results_publication::{
        ResultsManifestArtifact, ResultsManifestArtifacts, ResultsManifestCustomCss,
        ResultsPublicationStatus,
    };
    use serde_json::{json, Value};
    use std::collections::HashMap;

    pub const TENANT_ID: &str = "tenant-1";
    pub const ELECTION_EVENT_ID: &str = "event-1";
    pub const ELECTION_ID: &str = "election-1";
    pub const OTHER_ELECTION_ID: &str = "election-2";
    pub const UNPUBLISHED_ELECTION_ID: &str = "election-3";
    pub const AREA_ID: &str = "area-1";
    pub const OTHER_AREA_ID: &str = "area-2";
    pub const EVENT_REALM_ISSUER: &str =
        "https://keycloak.invalid/realms/tenant-tenant-1-event-event-1";
    pub const ADMIN_CLIENT_ID: &str = "admin-portal";
    pub const READ_ROLE: &str = "publish-results-read";
    pub const WRITE_ROLE: &str = "publish-results-write";

    // Publication requests use v4 UUIDs because source validation parses them.
    pub const REQUEST_TENANT_ID: &str = "10000000-0000-4000-8000-000000000001";
    pub const REQUEST_EVENT_ID: &str = "10000000-0000-4000-8000-000000000002";
    pub const REQUEST_SESSION_ID: &str = "10000000-0000-4000-8000-000000000003";
    pub const REQUEST_EXECUTION_ID: &str = "10000000-0000-4000-8000-000000000004";
    pub const REQUEST_RESULTS_EVENT_ID: &str = "10000000-0000-4000-8000-000000000005";
    pub const REQUEST_ELECTION_ID: &str = "10000000-0000-4000-8000-000000000006";
    pub const REQUEST_OTHER_ELECTION_ID: &str = "10000000-0000-4000-8000-000000000007";
    pub const REQUEST_CONTEST_ID: &str = "10000000-0000-4000-8000-000000000008";

    /// A results portal voter of `AREA_ID` authorized for `ELECTION_ID`.
    pub fn voter_claims() -> JwtClaims {
        serde_json::from_value(json!({
            "exp": 2_000_000_000,
            "iat": 1_900_000_000,
            "jti": "token-1",
            "iss": EVENT_REALM_ISSUER,
            "sub": "voter-1",
            "typ": "Bearer",
            "azp": "results-portal",
            "acr": "1",
            "allowed-origins": [],
            "scope": "openid",
            "email_verified": false,
            "https://hasura.io/jwt/claims": {
                "x-hasura-default-role": "user",
                "x-hasura-tenant-id": TENANT_ID,
                "x-hasura-user-id": "voter-1",
                "x-hasura-area-id": AREA_ID,
                "authorized-election-ids": [ELECTION_ID],
                "x-hasura-allowed-roles": ["user"]
            }
        }))
        .unwrap()
    }

    /// An admin portal user from the tenant realm with the read permission.
    pub fn admin_claims() -> JwtClaims {
        let mut claims = voter_claims();
        claims.azp = ADMIN_CLIENT_ID.to_string();
        claims.iss = "https://keycloak.invalid/realms/tenant-tenant-1".to_string();
        claims.hasura_claims.area_id = None;
        claims.hasura_claims.authorized_election_ids = None;
        claims.hasura_claims.allowed_roles = vec![READ_ROLE.to_string()];
        claims
    }

    /// A published event route publication of both elections.
    pub fn publication() -> TallyResultsPublication {
        TallyResultsPublication {
            id: "publication-1".to_string(),
            tenant_id: TENANT_ID.to_string(),
            election_event_id: ELECTION_EVENT_ID.to_string(),
            tally_session_id: "session-1".to_string(),
            tally_session_execution_id: "execution-1".to_string(),
            results_event_id: "results-event-1".to_string(),
            task_execution_id: None,
            route_scope: ResultsRouteScope::Event,
            route_election_id: None,
            election_ids: vec![ELECTION_ID.to_string(), OTHER_ELECTION_ID.to_string()],
            access: ResultsWebsiteAccess::Authenticated,
            visibility_scope: ResultsWebsiteVisibilityScope::FullEvent,
            published_contest_ids: vec!["contest-1".to_string()],
            contest_publication_state: HashMap::new(),
            documents: json!({}),
            manifest: None,
            publication_status: ResultsPublicationStatus::Published,
            version: 1,
            error_message: None,
            published_by_user_id: None,
        }
    }

    pub fn election_route_publication(election_id: Option<&str>) -> TallyResultsPublication {
        TallyResultsPublication {
            route_scope: ResultsRouteScope::Election,
            route_election_id: election_id.map(str::to_string),
            election_ids: election_id.into_iter().map(str::to_string).collect(),
            ..publication()
        }
    }

    pub fn artifact(document_id: &str) -> ResultsManifestArtifact {
        ResultsManifestArtifact {
            document_id: Some(document_id.to_string()),
            public_path: None,
        }
    }

    pub fn manifest(artifacts: ResultsManifestArtifacts) -> Value {
        serde_json::to_value(ResultsPublicationManifest {
            schema_version: 1,
            tenant_id: TENANT_ID.to_string(),
            election_event_id: ELECTION_EVENT_ID.to_string(),
            election_ids: vec![ELECTION_ID.to_string()],
            route_scope: ResultsRouteScope::Event,
            route_election_id: None,
            publication_id: "publication-1".to_string(),
            tally_session_id: "session-1".to_string(),
            tally_session_execution_id: "execution-1".to_string(),
            results_event_id: "results-event-1".to_string(),
            version: 1,
            access: ResultsWebsiteAccess::Authenticated,
            visibility_scope: ResultsWebsiteVisibilityScope::AreaBased,
            default_locale: Some("en".to_string()),
            available_languages: vec!["en".to_string()],
            title: HashMap::new(),
            custom_css: ResultsManifestCustomCss::default(),
            contests: vec![],
            artifacts,
        })
        .unwrap()
    }

    /// An area-based publication with artifacts for both areas.
    pub fn area_based_publication() -> TallyResultsPublication {
        let areas = HashMap::from([
            (AREA_ID.to_string(), artifact("area-1-sqlite")),
            (OTHER_AREA_ID.to_string(), artifact("area-2-sqlite")),
        ]);
        TallyResultsPublication {
            visibility_scope: ResultsWebsiteVisibilityScope::AreaBased,
            documents: json!({ "area_sqlite": areas }),
            manifest: Some(manifest(ResultsManifestArtifacts {
                full_sqlite: None,
                areas: Some(areas),
            })),
            ..publication()
        }
    }

    /// A request to publish a contest of two elections on the event route.
    pub fn publication_request() -> PublishResultsWebsiteInput {
        PublishResultsWebsiteInput {
            election_event_id: REQUEST_EVENT_ID.to_string(),
            tally_session_id: REQUEST_SESSION_ID.to_string(),
            tally_session_execution_id: REQUEST_EXECUTION_ID.to_string(),
            results_event_id: REQUEST_RESULTS_EVENT_ID.to_string(),
            route_scope: ResultsRouteScope::Event,
            route_election_id: None,
            election_ids: vec![
                REQUEST_ELECTION_ID.to_string(),
                REQUEST_OTHER_ELECTION_ID.to_string(),
            ],
            contest_ids: vec![REQUEST_CONTEST_ID.to_string()],
            access: ResultsWebsiteAccess::Authenticated,
            visibility_scope: ResultsWebsiteVisibilityScope::FullEvent,
        }
    }

    pub fn presentation(policy: Option<Value>) -> ElectionEventPresentation {
        ElectionEventPresentation {
            results_website: policy.map(|policy| policy.to_string()),
            ..Default::default()
        }
    }

    pub fn policy(status: &str, access: &str, visibility_scope: &str) -> Option<Value> {
        Some(json!({
            "status": status,
            "access": access,
            "visibility_scope": visibility_scope,
        }))
    }

    /// The variant and message of an error, which is what Harvest maps to
    /// a response.
    pub fn denial<T: std::fmt::Debug>(
        result: ResultsPublicationServiceResult<T>,
    ) -> (&'static str, String) {
        let error = result.unwrap_err();
        let variant = match &error {
            ResultsPublicationServiceError::BadRequest(_) => "BadRequest",
            ResultsPublicationServiceError::Unauthorized(_) => "Unauthorized",
            ResultsPublicationServiceError::Forbidden(_) => "Forbidden",
            ResultsPublicationServiceError::NotFound(_) => "NotFound",
            ResultsPublicationServiceError::Conflict(_) => "Conflict",
            ResultsPublicationServiceError::Internal(_) => "Internal",
        };
        (variant, error.to_string())
    }

    pub fn forbidden(message: &str) -> (&'static str, String) {
        ("Forbidden", message.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;
    use crate::types::results_publication::{
        ResultsManifestArtifacts, ResultsPublicationManifestDocument,
    };
    use serde_json::{json, Value};
    use std::collections::HashSet;

    const NOT_AUTHORIZED: &str = "Not authorized to view these election results";
    const WRONG_REALM: &str = "Token is not valid for this election event";
    const NO_AREA_ARTIFACT: &str = "No results artifact is available for this voter area";

    /// Authorizes a reader of the event publication of both elections.
    fn authorize(
        claims: &JwtClaims,
        requested_election_id: Option<&str>,
    ) -> ResultsPublicationServiceResult<()> {
        authorize_results_reader(
            claims,
            ELECTION_EVENT_ID,
            &publication(),
            requested_election_id,
        )
    }

    fn voter_authorized_for(election_ids: &[&str]) -> JwtClaims {
        let mut claims = voter_claims();
        claims.hasura_claims.authorized_election_ids =
            Some(election_ids.iter().map(|id| id.to_string()).collect());
        claims
    }

    fn with_documents(
        documents: Value,
        publication: TallyResultsPublication,
    ) -> TallyResultsPublication {
        TallyResultsPublication {
            documents,
            ..publication
        }
    }

    fn manifest_areas(
        publication: &TallyResultsPublication,
        claims: &JwtClaims,
    ) -> HashSet<String> {
        let manifest = manifest_for_reader(publication, claims).unwrap().unwrap();
        manifest.artifacts.areas.unwrap().into_keys().collect()
    }

    #[test]
    fn the_results_website_is_enabled_only_by_an_enabled_policy() {
        for (policy, enabled) in [
            (None, false),
            (policy("disabled", "authenticated", "full_event"), false),
            (policy("enabled", "authenticated", "full_event"), true),
        ] {
            let presentation = presentation(policy.clone());
            assert_eq!(
                is_results_website_enabled(&presentation).unwrap(),
                enabled,
                "{policy:?}"
            );
        }
    }

    #[test]
    fn an_unreadable_policy_is_an_error_rather_than_a_disabled_website() {
        let presentation = ElectionEventPresentation {
            results_website: Some("{\"status\":\"enabled\"".to_string()),
            ..Default::default()
        };

        for error in [
            results_website_policy(&presentation).unwrap_err(),
            is_results_website_enabled(&presentation).unwrap_err(),
            publication_matches_results_website_policy(&presentation, &publication()).unwrap_err(),
            validate_results_website_policy(
                &presentation,
                ResultsWebsiteAccess::Authenticated,
                ResultsWebsiteVisibilityScope::FullEvent,
            )
            .unwrap_err(),
        ] {
            assert_eq!(error.to_string(), "Invalid results website policy");
        }
    }

    #[test]
    fn publications_match_an_enabled_policy_with_their_access_and_visibility() {
        let presentation = presentation(policy("enabled", "authenticated", "full_event"));

        assert!(publication_matches_results_website_policy(&presentation, &publication()).unwrap());
    }

    #[test]
    fn publications_do_not_match_a_missing_disabled_or_different_policy() {
        for policy in [
            None,
            policy("disabled", "authenticated", "full_event"),
            policy("enabled", "public", "full_event"),
            policy("enabled", "authenticated", "area_based"),
        ] {
            let presentation = presentation(policy.clone());
            assert!(
                !publication_matches_results_website_policy(&presentation, &publication()).unwrap(),
                "{policy:?}"
            );
        }
    }

    #[test]
    fn new_publications_must_match_an_enabled_policy() {
        let validate = |policy| {
            validate_results_website_policy(
                &presentation(policy),
                ResultsWebsiteAccess::Authenticated,
                ResultsWebsiteVisibilityScope::FullEvent,
            )
            .map_err(|error| error.to_string())
        };

        assert_eq!(
            validate(policy("enabled", "authenticated", "full_event")),
            Ok(())
        );
        for (policy, message) in [
            (None, "Results website policy is not configured"),
            (
                policy("disabled", "authenticated", "full_event"),
                "Results website publishing is disabled for this election event",
            ),
            (
                policy("enabled", "public", "full_event"),
                "Results access does not match the election event results website policy",
            ),
            (
                policy("enabled", "authenticated", "area_based"),
                "Results visibility does not match the election event results website policy",
            ),
        ] {
            assert_eq!(validate(policy), Err(message.to_string()));
        }
    }

    #[test]
    fn election_routes_serve_only_their_route_election() {
        let publication = election_route_publication(Some(ELECTION_ID));

        assert!(publication_matches_requested_route(
            &publication,
            Some(ELECTION_ID)
        ));
        assert!(!publication_matches_requested_route(
            &publication,
            Some(OTHER_ELECTION_ID)
        ));
        assert!(!publication_matches_requested_route(&publication, None));
    }

    #[test]
    fn election_routes_without_a_route_election_serve_no_page() {
        let publication = election_route_publication(None);

        assert!(!publication_matches_requested_route(&publication, None));
    }

    #[test]
    fn event_routes_serve_the_event_page_and_each_published_election() {
        let publication = publication();

        assert!(publication_matches_requested_route(&publication, None));
        assert!(publication_matches_requested_route(
            &publication,
            Some(OTHER_ELECTION_ID)
        ));
        assert!(!publication_matches_requested_route(
            &publication,
            Some(UNPUBLISHED_ELECTION_ID)
        ));
    }

    #[test]
    fn results_portal_voters_read_an_authorized_published_election() {
        for client_id in ["results-portal", "voting-portal"] {
            let mut claims = voter_claims();
            claims.azp = client_id.to_string();

            assert!(authorize(&claims, Some(ELECTION_ID)).is_ok(), "{client_id}");
        }
    }

    #[test]
    fn a_trailing_slash_after_the_issuer_realm_is_ignored() {
        let mut claims = voter_claims();
        claims.iss = format!("{EVENT_REALM_ISSUER}//");

        assert!(authorize(&claims, None).is_ok());
    }

    #[test]
    fn results_portal_tokens_must_come_from_the_event_realm() {
        for issuer in [
            "https://keycloak.invalid/realms/tenant-tenant-1",
            "https://keycloak.invalid/realms/tenant-tenant-1-event-event-2",
            "https://keycloak.invalid/realms/tenant-tenant-1-event-event-1/account",
            "tenant-tenant-1-event-event-1x",
        ] {
            let mut claims = voter_claims();
            claims.iss = issuer.to_string();

            assert_eq!(
                denial(authorize(&claims, Some(ELECTION_ID))),
                forbidden(WRONG_REALM),
                "{issuer}"
            );
        }
    }

    #[test]
    fn the_event_realm_is_that_of_the_requested_event_and_token_tenant() {
        let result = authorize_results_reader(&voter_claims(), "event-2", &publication(), None);

        assert_eq!(denial(result), forbidden(WRONG_REALM));
    }

    #[test]
    fn results_portal_tokens_need_authorized_elections() {
        let mut claims = voter_claims();
        claims.hasura_claims.authorized_election_ids = None;

        assert_eq!(
            denial(authorize(&claims, None)),
            forbidden("No authorized elections are available")
        );
    }

    #[test]
    fn a_requested_election_must_be_both_authorized_and_published() {
        for (claims, requested) in [
            (voter_claims(), OTHER_ELECTION_ID),
            (
                voter_authorized_for(&[UNPUBLISHED_ELECTION_ID]),
                UNPUBLISHED_ELECTION_ID,
            ),
        ] {
            assert_eq!(
                denial(authorize(&claims, Some(requested))),
                forbidden(NOT_AUTHORIZED),
                "{requested}"
            );
        }
    }

    #[test]
    fn without_a_requested_election_one_authorized_published_election_suffices() {
        let claims = voter_authorized_for(&[UNPUBLISHED_ELECTION_ID, OTHER_ELECTION_ID]);

        assert!(authorize(&claims, None).is_ok());
    }

    #[test]
    fn without_a_requested_election_voters_need_an_authorized_published_election() {
        for authorized in [&[][..], &[UNPUBLISHED_ELECTION_ID]] {
            assert_eq!(
                denial(authorize(&voter_authorized_for(authorized), None)),
                forbidden(NOT_AUTHORIZED),
                "{authorized:?}"
            );
        }
    }

    #[test]
    fn a_results_publication_permission_lets_other_clients_read_any_election() {
        for role in [READ_ROLE, WRITE_ROLE] {
            let mut claims = admin_claims();
            claims.hasura_claims.allowed_roles = vec!["user".to_string(), role.to_string()];

            assert!(
                authorize(&claims, Some(UNPUBLISHED_ELECTION_ID)).is_ok(),
                "{role}"
            );
        }
    }

    #[test]
    fn other_clients_without_a_results_publication_permission_are_unauthorized() {
        // An event realm voter token with authorized elections does not
        // stand in for the permission outside the results portal.
        let mut claims = voter_claims();
        claims.azp = ADMIN_CLIENT_ID.to_string();
        claims.hasura_claims.allowed_roles = vec!["user".to_string(), "publish-read".to_string()];

        assert_eq!(
            denial(authorize(&claims, Some(ELECTION_ID))),
            (
                "Unauthorized",
                "Missing results publication permission".to_string()
            )
        );
    }

    #[test]
    fn a_results_publication_permission_does_not_exempt_results_portal_tokens() {
        let mut claims = voter_claims();
        claims.iss = "https://keycloak.invalid/realms/tenant-tenant-1".to_string();
        claims.hasura_claims.allowed_roles = vec![READ_ROLE.to_string()];

        assert_eq!(
            denial(authorize(&claims, Some(ELECTION_ID))),
            forbidden(WRONG_REALM)
        );
    }

    #[test]
    fn full_event_manifests_are_shown_unchanged() {
        let publication = TallyResultsPublication {
            manifest: Some(manifest(ResultsManifestArtifacts {
                full_sqlite: Some(artifact("full-sqlite")),
                areas: None,
            })),
            ..publication()
        };

        let manifest = manifest_for_reader(&publication, &voter_claims()).unwrap();
        assert_eq!(
            serde_json::to_value(manifest).unwrap(),
            publication.manifest.unwrap()
        );
    }

    #[test]
    fn publications_without_a_manifest_show_none() {
        assert!(manifest_for_reader(&publication(), &voter_claims())
            .unwrap()
            .is_none());
    }

    #[test]
    fn an_unreadable_stored_manifest_is_an_internal_error() {
        let publication = TallyResultsPublication {
            manifest: Some(json!({ "schema_version": "one" })),
            ..publication()
        };

        assert_eq!(
            denial(manifest_for_reader(&publication, &voter_claims())),
            (
                "Internal",
                "Invalid stored results publication manifest".to_string()
            )
        );
    }

    #[test]
    fn results_portal_voters_see_only_their_area_of_an_area_based_manifest() {
        assert_eq!(
            manifest_areas(&area_based_publication(), &voter_claims()),
            HashSet::from([AREA_ID.to_string()])
        );
    }

    #[test]
    fn other_clients_see_every_area_of_an_area_based_manifest() {
        assert_eq!(
            manifest_areas(&area_based_publication(), &admin_claims()),
            HashSet::from([AREA_ID.to_string(), OTHER_AREA_ID.to_string()])
        );
    }

    #[test]
    fn area_based_manifests_need_a_voter_area() {
        let mut claims = voter_claims();
        claims.hasura_claims.area_id = None;

        assert_eq!(
            denial(manifest_for_reader(&area_based_publication(), &claims)),
            forbidden("No voter area is available")
        );
    }

    #[test]
    fn area_based_manifests_without_area_artifacts_are_not_found() {
        for manifest in [
            None,
            Some(manifest(ResultsManifestArtifacts {
                full_sqlite: Some(artifact("full-sqlite")),
                areas: None,
            })),
        ] {
            let publication = TallyResultsPublication {
                manifest,
                ..area_based_publication()
            };

            assert_eq!(
                denial(manifest_for_reader(&publication, &voter_claims())),
                (
                    "NotFound",
                    "No area results artifacts are available".to_string()
                )
            );
        }
    }

    #[test]
    fn voters_of_an_area_without_an_artifact_are_forbidden_the_manifest() {
        let mut claims = voter_claims();
        claims.hasura_claims.area_id = Some("area-3".to_string());

        assert_eq!(
            denial(manifest_for_reader(&area_based_publication(), &claims)),
            forbidden(NO_AREA_ARTIFACT)
        );
    }

    #[test]
    fn full_event_readers_download_the_full_sqlite() {
        let publication = with_documents(
            json!({ "full_sqlite": artifact("full-sqlite") }),
            publication(),
        );

        for claims in [voter_claims(), admin_claims()] {
            assert_eq!(
                artifact_document_ids_for_reader(&publication, &claims).unwrap(),
                vec!["full-sqlite"]
            );
        }
    }

    #[test]
    fn full_event_publications_without_a_full_sqlite_document_are_not_found() {
        for documents in [json!({}), json!({ "full_sqlite": {} })] {
            let publication = with_documents(documents, publication());

            assert_eq!(
                denial(artifact_document_ids_for_reader(
                    &publication,
                    &voter_claims()
                )),
                ("NotFound", "No results artifact is available".to_string())
            );
        }
    }

    #[test]
    fn area_based_readers_download_their_area_sqlite() {
        assert_eq!(
            artifact_document_ids_for_reader(&area_based_publication(), &voter_claims()).unwrap(),
            vec!["area-1-sqlite"]
        );
    }

    #[test]
    fn area_based_downloads_need_a_voter_area_even_for_other_clients() {
        assert_eq!(
            denial(artifact_document_ids_for_reader(
                &area_based_publication(),
                &admin_claims()
            )),
            forbidden("No voter area is available")
        );
    }

    #[test]
    fn area_based_downloads_without_a_document_for_the_voter_area_are_forbidden() {
        for documents in [
            json!({}),
            json!({ "area_sqlite": { OTHER_AREA_ID: artifact("area-2-sqlite") } }),
            json!({ "area_sqlite": { AREA_ID: {} } }),
        ] {
            let publication = with_documents(documents.clone(), area_based_publication());

            assert_eq!(
                denial(artifact_document_ids_for_reader(
                    &publication,
                    &voter_claims()
                )),
                forbidden(NO_AREA_ARTIFACT),
                "{documents}"
            );
        }
    }

    #[test]
    fn unreadable_stored_documents_are_an_internal_error() {
        let publication = with_documents(json!({ "full_sqlite": "full-sqlite" }), publication());
        let expected = (
            "Internal",
            "Invalid stored results publication documents".to_string(),
        );

        assert_eq!(
            denial(artifact_document_ids_for_reader(
                &publication,
                &voter_claims()
            )),
            expected
        );
        assert_eq!(
            denial(manifest_public_path(&publication).map_err(Into::into)),
            expected
        );
    }

    #[test]
    fn the_latest_manifest_path_is_preferred_over_the_versioned_one() {
        let manifest_path = |latest_public_path: Option<&str>| {
            let manifest = ResultsPublicationManifestDocument {
                document_id: None,
                public_path: Some("results/manifest-v1.json".to_string()),
                latest_public_path: latest_public_path.map(str::to_string),
            };
            manifest_public_path(&with_documents(
                json!({ "manifest": manifest }),
                publication(),
            ))
            .unwrap()
        };

        assert_eq!(
            manifest_path(Some("results/manifest-latest.json")).as_deref(),
            Some("results/manifest-latest.json")
        );
        assert_eq!(
            manifest_path(None).as_deref(),
            Some("results/manifest-v1.json")
        );
        assert_eq!(manifest_public_path(&publication()).unwrap(), None);
    }

    fn uuid(value: &str) -> Uuid {
        Uuid::parse_str(value).unwrap()
    }

    fn request_source() -> PublicationSource {
        publication_source(REQUEST_TENANT_ID, &publication_request()).unwrap()
    }

    fn source_facts(
        valid_execution: bool,
        election_count: i64,
        contest_count: i64,
        tallied_contest_count: i64,
    ) -> PublicationSourceFacts {
        PublicationSourceFacts {
            valid_execution,
            election_count,
            contest_count,
            tallied_contest_count,
        }
    }

    fn source_error(tenant_id: &str, request: &PublishResultsWebsiteInput) -> String {
        publication_source(tenant_id, request)
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn a_publication_source_holds_the_parsed_request_identifiers() {
        assert_eq!(
            request_source(),
            PublicationSource {
                tenant_id: uuid(REQUEST_TENANT_ID),
                election_event_id: uuid(REQUEST_EVENT_ID),
                tally_session_id: uuid(REQUEST_SESSION_ID),
                tally_session_execution_id: uuid(REQUEST_EXECUTION_ID),
                results_event_id: uuid(REQUEST_RESULTS_EVENT_ID),
                election_ids: vec![uuid(REQUEST_ELECTION_ID), uuid(REQUEST_OTHER_ELECTION_ID)],
                contest_ids: vec![uuid(REQUEST_CONTEST_ID)],
            }
        );
    }

    #[test]
    fn a_publication_needs_an_election_and_a_contest_before_identifiers_are_parsed() {
        let mut without_elections = publication_request();
        without_elections.election_ids.clear();
        let mut without_contests = publication_request();
        without_contests.contest_ids.clear();

        for request in [without_elections, without_contests] {
            assert_eq!(
                source_error("not-a-uuid", &request),
                "A publication requires at least one election and one contest"
            );
        }
    }

    #[test]
    fn every_source_identifier_must_be_a_v4_uuid() {
        let not_v4 = "10000000-0000-1000-8000-000000000001";
        for (invalid, expected_prefix) in [
            ("not-a-uuid", "invalid UUID 'not-a-uuid'"),
            (
                not_v4,
                "UUID '10000000-0000-1000-8000-000000000001' is not v4",
            ),
        ] {
            let mut requests = vec![];
            for field in 0..7 {
                let mut request = publication_request();
                let invalid = invalid.to_string();
                match field {
                    0 => request.election_event_id = invalid,
                    1 => request.tally_session_id = invalid,
                    2 => request.tally_session_execution_id = invalid,
                    3 => request.results_event_id = invalid,
                    4 => request.election_ids[1] = invalid,
                    5 => request.contest_ids[0] = invalid,
                    _ => request.route_election_id = Some(invalid),
                }
                requests.push(request);
            }

            assert!(source_error(invalid, &publication_request()).starts_with(expected_prefix));
            for request in requests {
                let error = source_error(REQUEST_TENANT_ID, &request);
                assert!(error.starts_with(expected_prefix), "{error}");
            }
        }
    }

    #[test]
    fn a_route_election_must_be_one_of_the_publication_elections() {
        let mut request = publication_request();
        request.route_election_id = Some("10000000-0000-4000-8000-000000000009".to_string());

        assert_eq!(
            source_error(REQUEST_TENANT_ID, &request),
            "The route election must be included in the publication elections"
        );
    }

    #[test]
    fn route_elections_are_compared_as_uuids_not_as_text() {
        let mut request = publication_request();
        request.election_ids = vec!["a0000000-0000-4000-8000-00000000000a".to_string()];
        request.route_election_id = Some("A0000000-0000-4000-8000-00000000000A".to_string());

        assert!(publication_source(REQUEST_TENANT_ID, &request).is_ok());
    }

    #[test]
    fn a_source_whose_records_all_exist_and_were_tallied_is_consistent() {
        assert!(check_publication_source(&request_source(), &source_facts(true, 2, 1, 1)).is_ok());
    }

    #[test]
    fn inconsistent_sources_report_their_first_failing_check() {
        for (facts, message) in [
            (
                source_facts(false, 1, 0, 0),
                "The tally session, execution, and results event do not belong together",
            ),
            (
                source_facts(true, 1, 0, 0),
                "One or more publication elections are outside the tally event",
            ),
            (
                source_facts(true, 3, 1, 1),
                "One or more publication elections are outside the tally event",
            ),
            (
                source_facts(true, 2, 0, 0),
                "One or more publication contests are outside the selected elections",
            ),
            (
                source_facts(true, 2, 1, 0),
                "Every selected contest must have results in the selected tally execution",
            ),
        ] {
            let error = check_publication_source(&request_source(), &facts).unwrap_err();
            assert_eq!(error.to_string(), message, "{facts:?}");
        }
    }

    #[test]
    fn repeated_source_identifiers_do_not_match_the_distinct_database_counts() {
        let mut request = publication_request();
        request.contest_ids = vec![REQUEST_CONTEST_ID.to_string(); 2];
        let source = publication_source(REQUEST_TENANT_ID, &request).unwrap();

        // The database counts the repeated contest once.
        let error = check_publication_source(&source, &source_facts(true, 2, 1, 1)).unwrap_err();
        assert_eq!(
            error.to_string(),
            "One or more publication contests are outside the selected elections"
        );
    }

    #[test]
    fn audit_details_describe_the_publication_and_the_action() {
        let publication = TallyResultsPublication {
            published_contest_ids: vec!["contest-1".to_string(), "contest-2".to_string()],
            ..election_route_publication(Some(ELECTION_ID))
        };

        assert_eq!(
            results_publication_log_details(&publication, ResultsPublicationAction::Revoke),
            ResultsPublicationDetails {
                publication_id: ResultsPublicationIdString("publication-1".to_string()),
                action: ResultsPublicationAction::Revoke,
                route_scope: ResultsPublicationRouteScopeString("election".to_string()),
                route_election_id: ElectionIdString(Some(ELECTION_ID.to_string())),
                access: ResultsPublicationAccessString("authenticated".to_string()),
                visibility_scope: ResultsPublicationVisibilityScopeString("full_event".to_string()),
                contest_ids: vec![
                    ContestIdString("contest-1".to_string()),
                    ContestIdString("contest-2".to_string()),
                ],
            }
        );
    }
}
