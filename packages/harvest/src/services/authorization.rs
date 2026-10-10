// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Context;
use deadpool_postgres::Transaction;
use rocket::http::Status;
use rocket::response::status::Unauthorized;
use sequent_core::ballot::{VotingStatus, VotingStatusChannel};
use sequent_core::services::jwt::{decode_permission_labels, JwtClaims};
use sequent_core::types::permissions::{Permissions, VoterPermissions};
use std::collections::HashSet;
use std::env;
use tracing::{error, info, instrument};
use windmill::postgres::election::{
    get_elections_permission_labels, ElectionPermissionLabel,
};
use windmill::postgres::tally_session::get_tally_session_by_id;

#[instrument(skip(claims))]
pub fn authorize(
    claims: &JwtClaims,
    allow_super_admin_auth: bool, // Allow authorizing super admin tenant
    tenant_id_opt: Option<String>,
    permissions: Vec<Permissions>,
) -> Result<(), (Status, String)> {
    // Verify tenant id
    let allowed = match (tenant_id_opt.clone(), allow_super_admin_auth) {
        (Some(tenant_id), _)
            if tenant_id.eq(&claims.hasura_claims.tenant_id) =>
        {
            true // is valid tenant
        }

        (_, true) => {
            let super_admin_tenant_id = env::var("SUPER_ADMIN_TENANT_ID")
                .map_err(|_| {
                    (
                        Status::Unauthorized,
                        format!("SUPER_ADMIN_TENANT_ID must be set"),
                    )
                })?;
            info!("super_admin_tenant_id: {super_admin_tenant_id}");
            super_admin_tenant_id == claims.hasura_claims.tenant_id // is super admin?
        }
        (_, _) => false, // Is not valid tenant nor super admin
    };

    if !allowed {
        error!(
            "Not authorized: allow_super_admin_auth: {allow_super_admin_auth}, 
            tenant_id_opt: {tenant_id_opt:?}, claims tenant_id: {}",
            claims.hasura_claims.tenant_id
        );
        return Err((Status::Unauthorized, format!("Unathorized: not a super admin or invalid tenant_id {tenant_id_opt:?}")));
    }

    let perms_str: Vec<String> = permissions
        .into_iter()
        .map(|permission| permission.to_string())
        .collect();
    let permissions_set: HashSet<_> =
        claims.hasura_claims.allowed_roles.iter().collect();
    let all_contained =
        perms_str.iter().all(|item| permissions_set.contains(&item));

    if !all_contained {
        Err((
            Status::Unauthorized,
            format!("Unathorized: {perms_str:?} not in {permissions_set:?}"),
        ))
    } else {
        Ok(())
    }
}

// returns area_id
#[instrument(skip(claims))]
pub fn authorize_voter_election(
    claims: &JwtClaims,
    permissions: Vec<VoterPermissions>,
    election_id: &String,
) -> Result<(String, VotingStatusChannel), (Status, String)> {
    let perms_str: Vec<String> = permissions
        .into_iter()
        .map(|permission| permission.to_string())
        .collect();
    let permissions_set: HashSet<_> =
        claims.hasura_claims.allowed_roles.iter().collect();
    let all_contained =
        perms_str.iter().all(|item| permissions_set.contains(&item));

    if !all_contained {
        return Err((Status::Unauthorized, "".into()));
    }

    let Some(area_id) = claims.hasura_claims.area_id.clone() else {
        return Err((Status::Unauthorized, "Missing area_id".into()));
    };

    // Check election id checks
    if claims.hasura_claims.authorized_election_ids.is_none()
        || !claims
            .hasura_claims
            .authorized_election_ids
            .as_ref()
            .unwrap_or(&Vec::new())
            .contains(election_id)
    {
        return Err((
            Status::Unauthorized,
            "Not authorized to election".into(),
        ));
    }

    match claims.azp.as_str() {
        "voting-portal" => Ok((area_id, VotingStatusChannel::ONLINE)),
        "voting-portal-kiosk" => Ok((area_id, VotingStatusChannel::KIOSK)),
        _ => Err((Status::Unauthorized, "Unknown Client".into())),
    }
}

/// The election permission labels of an admin, applied as Hasura applies
/// `X-Hasura-Permission-Labels` to elections: an election without a label is
/// open to every admin, a labelled one only to admins holding its label.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PermissionLabels(HashSet<String>);

impl PermissionLabels {
    pub fn from_claims(claims: &JwtClaims) -> Self {
        Self(decode_permission_labels(claims).into_iter().collect())
    }

    pub fn allows(&self, election_label: Option<&str>) -> bool {
        election_label.map_or(true, |label| self.0.contains(label))
    }
}

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
