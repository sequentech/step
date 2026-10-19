// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Context;
use deadpool_postgres::Transaction;
use rocket::http::Status;
use rocket::response::status::Unauthorized;
use sequent_core::ballot::{VotingStatus, VotingStatusChannel};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::{Permissions, VoterPermissions};
use std::collections::HashSet;
use std::env;
use tracing::{error, info, instrument};
use windmill::postgres::election::{
    get_elections_permission_labels, ElectionPermissionLabel,
};
use windmill::postgres::tally_session::get_tally_session_by_id;

pub use sequent_core::services::authorization::*;

const ELECTION_OUTSIDE_PERMISSION_LABELS: &str =
    "Election outside the user's permission labels";

fn ensure_elections_permitted(
    permission_labels: &PermissionLabels,
    elections: &[ElectionPermissionLabel],
) -> Result<(), (Status, String)> {
    if elections.iter().all(|election| {
        permission_labels.allows(election.permission_label.as_deref())
    }) {
        Ok(())
    } else {
        Err((
            Status::Forbidden,
            ELECTION_OUTSIDE_PERMISSION_LABELS.to_string(),
        ))
    }
}

fn permitted_election_ids(
    permission_labels: &PermissionLabels,
    elections: &[ElectionPermissionLabel],
) -> Option<Vec<String>> {
    let permitted: Vec<String> = elections
        .iter()
        .filter(|election| {
            permission_labels.allows(election.permission_label.as_deref())
        })
        .map(|election| election.election_id.clone())
        .collect();
    (permitted.len() < elections.len()).then_some(permitted)
}

async fn event_elections_permission_labels(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_ids: Option<&[String]>,
) -> Result<Vec<ElectionPermissionLabel>, (Status, String)> {
    get_elections_permission_labels(
        hasura_transaction,
        tenant_id,
        election_event_id,
        election_ids,
    )
    .await
    .map_err(|error| (Status::InternalServerError, format!("{error:?}")))
}

/// Refuses unless the caller's permission labels allow every given election
/// of the event, or every election of the event when `election_ids` is `None`.
#[instrument(skip(hasura_transaction, claims))]
pub async fn authorize_election_permission_labels(
    hasura_transaction: &Transaction<'_>,
    claims: &JwtClaims,
    election_event_id: &str,
    election_ids: Option<&[String]>,
) -> Result<(), (Status, String)> {
    let elections = event_elections_permission_labels(
        hasura_transaction,
        &claims.hasura_claims.tenant_id,
        election_event_id,
        election_ids,
    )
    .await?;
    ensure_elections_permitted(
        &PermissionLabels::from_claims(claims),
        &elections,
    )
}

/// Refuses unless the caller's permission labels allow every election of the
/// tally session.
#[instrument(skip(hasura_transaction, claims))]
pub async fn authorize_tally_session_permission_labels(
    hasura_transaction: &Transaction<'_>,
    claims: &JwtClaims,
    election_event_id: &str,
    tally_session_id: &str,
) -> Result<(), (Status, String)> {
    let tally_session = get_tally_session_by_id(
        hasura_transaction,
        &claims.hasura_claims.tenant_id,
        election_event_id,
        tally_session_id,
    )
    .await
    .map_err(|_| {
        (
            Status::NotFound,
            format!("Could not find tally session by id {tally_session_id}"),
        )
    })?;
    authorize_election_permission_labels(
        hasura_transaction,
        claims,
        election_event_id,
        tally_session.election_ids.as_deref(),
    )
    .await
}

/// Elections of the event the caller's permission labels allow, or `None`
/// when they allow all of them.
#[instrument(skip(hasura_transaction, claims))]
pub async fn permitted_event_election_ids(
    hasura_transaction: &Transaction<'_>,
    claims: &JwtClaims,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<Option<Vec<String>>, (Status, String)> {
    let elections = event_elections_permission_labels(
        hasura_transaction,
        tenant_id,
        election_event_id,
        None,
    )
    .await?;
    Ok(permitted_election_ids(
        &PermissionLabels::from_claims(claims),
        &elections,
    ))
}

#[cfg(test)]
mod permission_label_tests {
    use super::*;

    fn labels(permission_labels: &str) -> PermissionLabels {
        PermissionLabels::from_claims(
            &serde_json::from_value(serde_json::json!({
                "exp": 1, "iat": 0, "jti": "test", "iss": "test",
                "sub": "admin", "typ": "Bearer", "azp": "admin-portal",
                "acr": "1", "allowed-origins": [], "scope": "openid",
                "email_verified": false,
                "https://hasura.io/jwt/claims": {
                    "x-hasura-default-role": "admin-user",
                    "x-hasura-tenant-id": "tenant",
                    "x-hasura-user-id": "admin",
                    "x-hasura-allowed-roles": ["admin-user"],
                    "x-hasura-permission-labels": permission_labels
                }
            }))
            .unwrap(),
        )
    }

    fn election(id: &str, label: Option<&str>) -> ElectionPermissionLabel {
        ElectionPermissionLabel {
            election_id: id.to_string(),
            permission_label: label.map(str::to_string),
        }
    }

    fn event() -> Vec<ElectionPermissionLabel> {
        vec![
            election("open", None),
            election("north-1", Some("north")),
            election("south-1", Some("south")),
        ]
    }

    #[test]
    fn refuses_any_election_outside_the_permission_labels() {
        let north = labels(r#"{"north"}"#);
        assert_eq!(ensure_elections_permitted(&north, &event()[..2]), Ok(()));
        for elections in [&event()[..], &event()[2..]] {
            assert_eq!(
                ensure_elections_permitted(&north, elections),
                Err((
                    Status::Forbidden,
                    ELECTION_OUTSIDE_PERMISSION_LABELS.to_string()
                ))
            );
        }
        assert!(
            ensure_elections_permitted(&labels("{}"), &event()[1..2]).is_err()
        );
    }

    #[test]
    fn allows_events_whose_elections_are_all_permitted() {
        assert_eq!(
            ensure_elections_permitted(
                &labels(r#"{"north", "south"}"#),
                &event()
            ),
            Ok(())
        );
        assert_eq!(
            ensure_elections_permitted(&labels("{}"), &event()[..1]),
            Ok(())
        );
        assert_eq!(ensure_elections_permitted(&labels("{}"), &[]), Ok(()));
    }

    #[test]
    fn lists_permitted_elections_only_when_some_are_outside_the_labels() {
        assert_eq!(
            permitted_election_ids(&labels(r#"{"north"}"#), &event()),
            Some(vec!["open".to_string(), "north-1".to_string()])
        );
        assert_eq!(
            permitted_election_ids(&labels("{}"), &event()),
            Some(vec!["open".to_string()])
        );
        assert_eq!(
            permitted_election_ids(&labels(r#"{"north", "south"}"#), &event()),
            None
        );
        assert_eq!(permitted_election_ids(&labels("{}"), &event()[..1]), None);
    }
}
