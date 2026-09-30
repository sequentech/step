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

struct VoterScope<'a> {
    area_id: &'a str,
    election_ids: &'a [String],
    channel: VotingStatusChannel,
}

fn authorize_voter<'a>(
    claims: &'a JwtClaims,
    permissions: Vec<VoterPermissions>,
    election_id: Option<&String>,
) -> Result<VoterScope<'a>, (Status, String)> {
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

    let area_id = claims
        .hasura_claims
        .area_id
        .as_deref()
        .ok_or_else(|| (Status::Unauthorized, "Missing area_id".into()))?;
    let election_ids = claims
        .hasura_claims
        .authorized_election_ids
        .as_deref()
        .ok_or_else(|| {
            (Status::Unauthorized, "Not authorized to election".into())
        })?;
    if election_id.is_some_and(|id| !election_ids.contains(id)) {
        return Err((
            Status::Unauthorized,
            "Not authorized to election".into(),
        ));
    }

    let client = claims
        .azp
        .parse::<VoterClient>()
        .map_err(|_| (Status::Unauthorized, "Unknown Client".into()))?;
    Ok(VoterScope {
        area_id,
        election_ids,
        channel: client.channel(),
    })
}

#[instrument(skip(claims))]
pub fn authorize_voter_election(
    claims: &JwtClaims,
    permissions: Vec<VoterPermissions>,
    election_id: &String,
) -> Result<(String, VotingStatusChannel), (Status, String)> {
    let scope = authorize_voter(claims, permissions, Some(election_id))?;
    Ok((scope.area_id.to_owned(), scope.channel))
}

/// Authorize reading the voter's election list within one event.
/// An empty list is valid; a missing list is not.
#[instrument(skip(claims))]
pub fn authorize_voter_event(
    claims: &JwtClaims,
    permissions: Vec<VoterPermissions>,
    event_id: &str,
) -> Result<(String, Vec<String>), (Status, String)> {
    let denied = || (Status::Forbidden, "Voter not authorized".to_owned());
    if claims.hasura_claims.election_event_id.as_deref() != Some(event_id) {
        return Err(denied());
    }
    let scope =
        authorize_voter(claims, permissions, None).map_err(|_| denied())?;
    Ok((scope.area_id.to_owned(), scope.election_ids.to_vec()))
}

#[cfg(test)]
mod voter_authorization_tests {
    use super::*;

    fn voter() -> JwtClaims {
        serde_json::from_value(serde_json::json!({
            "exp": 1, "iat": 0, "jti": "test", "iss": "test",
            "sub": "voter", "typ": "Bearer", "azp": "voting-portal",
            "acr": "1", "allowed-origins": [], "scope": "openid",
            "email_verified": false,
            "https://hasura.io/jwt/claims": {
                "x-hasura-default-role": "user",
                "x-hasura-tenant-id": "tenant",
                "x-hasura-user-id": "voter",
                "x-hasura-area-id": "area",
                "x-hasura-election-event-id": "event",
                "authorized-election-ids": ["election", "another-election"],
                "x-hasura-allowed-roles": ["user"]
            }
        }))
        .unwrap()
    }

    fn election(
        claims: &JwtClaims,
    ) -> Result<(String, VotingStatusChannel), (Status, String)> {
        authorize_voter_election(
            claims,
            vec![VoterPermissions::CAST_VOTE],
            &"election".into(),
        )
    }

    fn event(
        claims: &JwtClaims,
    ) -> Result<(String, Vec<String>), (Status, String)> {
        authorize_voter_event(
            claims,
            vec![VoterPermissions::CAST_VOTE],
            "event",
        )
    }

    fn assert_denied(claims: &JwtClaims, election_message: &str) {
        assert_eq!(
            election(claims).unwrap_err(),
            (Status::Unauthorized, election_message.into())
        );
        assert_eq!(
            event(claims).unwrap_err(),
            (Status::Forbidden, "Voter not authorized".into())
        );
    }

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
            let mut claims = voter();
            claims.azp = name.into();
            assert_eq!(election(&claims).unwrap(), ("area".into(), channel));
            assert_eq!(
                event(&claims).unwrap(),
                (
                    "area".into(),
                    vec!["election".into(), "another-election".into()]
                )
            );
        }
        assert!("admin-portal".parse::<VoterClient>().is_err());
        assert!("".parse::<VoterClient>().is_err());
    }

    #[test]
    fn both_helpers_require_permission_area_election_list_and_voter_client() {
        let mut claims = voter();
        claims.hasura_claims.allowed_roles.clear();
        assert_denied(&claims, "");

        let mut claims = voter();
        claims.hasura_claims.area_id = None;
        assert_denied(&claims, "Missing area_id");

        let mut claims = voter();
        claims.hasura_claims.authorized_election_ids = None;
        assert_denied(&claims, "Not authorized to election");

        for client in ["admin-portal", ""] {
            let mut claims = voter();
            claims.azp = client.into();
            assert_denied(&claims, "Unknown Client");
        }
    }

    #[test]
    fn both_helpers_require_every_requested_permission() {
        for roles in [
            vec!["user"],
            vec!["ack-support-materials"],
            vec!["user", "ack-support-materials"],
        ] {
            let mut claims = voter();
            claims.hasura_claims.allowed_roles =
                roles.iter().map(|role| role.to_string()).collect();
            let permissions = || {
                vec![
                    VoterPermissions::CAST_VOTE,
                    VoterPermissions::ACK_SUPPORT_MATERIALS,
                ]
            };
            assert_eq!(
                authorize_voter_election(
                    &claims,
                    permissions(),
                    &"election".into()
                )
                .is_ok(),
                roles.len() == 2
            );
            assert_eq!(
                authorize_voter_event(&claims, permissions(), "event").is_ok(),
                roles.len() == 2
            );
        }
    }

    #[test]
    fn election_requires_membership_but_event_allows_an_empty_list() {
        let mut claims = voter();
        for ids in [vec!["other".into()], vec![]] {
            claims.hasura_claims.authorized_election_ids = Some(ids.clone());
            assert_eq!(
                election(&claims).unwrap_err(),
                (Status::Unauthorized, "Not authorized to election".into())
            );
            assert_eq!(event(&claims).unwrap(), ("area".into(), ids));
        }
        // Keep the existing election error when both election and client are invalid.
        claims.azp = "admin-portal".into();
        assert_denied(&claims, "Not authorized to election");
    }

    #[test]
    fn event_requires_matching_event_without_changing_election_authorization() {
        for event_id in [None, Some("other".into())] {
            let mut claims = voter();
            claims.hasura_claims.election_event_id = event_id;
            assert_eq!(
                event(&claims).unwrap_err(),
                (Status::Forbidden, "Voter not authorized".into())
            );
            assert!(election(&claims).is_ok());
        }
    }
}
