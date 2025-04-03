// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use rocket::http::Status;
use rocket::response::status::Unauthorized;
use sequent_core::ballot::{VotingStatus, VotingStatusChannel};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::{Permissions, VoterPermissions};
use std::collections::HashSet;
use std::env;
use tracing::instrument;

#[instrument(skip(claims))]
pub fn authorize(
    claims: &JwtClaims,
    check_super_admin: bool,
    tenant_id_opt: Option<String>,
    permissions: Vec<Permissions>,
) -> Result<(), (Status, String)> {
    let super_admin_tenant_id = env::var("SUPER_ADMIN_TENANT_ID")
        .expect(&format!("SUPER_ADMIN_TENANT_ID must be set"));
    let is_super_admin =
        super_admin_tenant_id == claims.hasura_claims.tenant_id;
    let is_valid_tenant = tenant_id_opt
        .clone()
        .map(|tenant_id| tenant_id == claims.hasura_claims.tenant_id)
        .unwrap_or(false);
    if !((check_super_admin && is_super_admin)
        || (tenant_id_opt.is_some() && is_valid_tenant))
    {
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
pub fn authorize_voter(
    claims: &JwtClaims,
    permissions: Vec<VoterPermissions>,
    election_id: Option<String>,
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
    match (
        election_id,
        claims.hasura_claims.clone().authorized_election_ids,
    ) {
        (None, _) => {}
        (Some(election_id), Some(authorized_election_ids))
            if authorized_election_ids.contains(&election_id) => {}
        _ => {
            return Err((
                Status::Unauthorized,
                "Not authorized to election".into(),
            ));
        }
    };

    match claims.azp.as_str() {
        "voting-portal" => Ok((area_id, VotingStatusChannel::ONLINE)),
        "voting-portal-kiosk" => Ok((area_id, VotingStatusChannel::KIOSK)),
        _ => Err((Status::Unauthorized, "Unknown Client".into())),
    }
}
