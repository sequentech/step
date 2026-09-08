// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::jwt::JwtClaims;
use crate::ballot::{VotingStatus, VotingStatusChannel};
use crate::types::permissions::{Permissions, VoterPermissions};
use anyhow::Context;
use rocket::http::Status;
use rocket::response::status::Unauthorized;
use std::collections::HashSet;
use std::env;
use tracing::{error, info, instrument};

/// Keycloak clients allowed to authenticate voters. Keep the distinct onsite
/// client identity while applying the polling-station (kiosk) voting policy.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    strum_macros::Display,
    strum_macros::EnumString,
)]
pub enum VoterClient {
    #[strum(serialize = "voting-portal")]
    Online,
    #[strum(serialize = "voting-portal-kiosk")]
    Kiosk,
    #[strum(serialize = "onsite-voting-portal")]
    Onsite,
    #[strum(serialize = "ivr-voting")]
    Telephone,
}

impl VoterClient {
    /// Return the policy channel used to validate a cast from this client.
    pub fn channel(self) -> VotingStatusChannel {
        match self {
            Self::Online => VotingStatusChannel::ONLINE,
            Self::Kiosk | Self::Onsite => VotingStatusChannel::KIOSK,
            Self::Telephone => VotingStatusChannel::TELEPHONE,
        }
    }
}

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

    claims
        .azp
        .parse::<VoterClient>()
        .map(|client| (area_id, client.channel()))
        .map_err(|_| (Status::Unauthorized, "Unknown Client".into()))
}

#[cfg(test)]
mod voter_client_tests {
    use super::*;

    #[test]
    fn voter_clients_round_trip_with_their_policy_channels() {
        for (name, channel) in [
            ("voting-portal", VotingStatusChannel::ONLINE),
            ("voting-portal-kiosk", VotingStatusChannel::KIOSK),
            ("onsite-voting-portal", VotingStatusChannel::KIOSK),
            ("ivr-voting", VotingStatusChannel::TELEPHONE),
        ] {
            let client = name.parse::<VoterClient>().unwrap();
            assert_eq!(client.to_string(), name);
            assert_eq!(client.channel(), channel);
        }
        assert!("admin-portal".parse::<VoterClient>().is_err());
        assert!("".parse::<VoterClient>().is_err());
    }
}
