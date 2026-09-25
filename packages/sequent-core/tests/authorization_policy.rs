// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Test authorization decisions after identity verification. These synthetic
//! claims are not signed tokens and do not test the JWT verification boundary.

#![cfg(all(feature = "default_features", feature = "keycloak"))]

#[path = "support/claims.rs"]
#[allow(dead_code)]
mod claims;

use claims::Claims;
use rocket::http::Status;
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::services::authorization::{
    authorize, authorize_voter_election, authorize_with,
};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::{Permissions, VoterPermissions};

const TENANT_ID: &str = "tenant-a";
const OTHER_TENANT_ID: &str = "tenant-b";
const SUPER_ADMIN_TENANT_ID: &str = "super-admin-tenant";
const USER_ID: &str = "test-voter";
const AREA_ID: &str = "area-a";
const ELECTION_ID: &str = "election-a";
const VOTER_ROLE: &str = "user";
const TENANT_READ_ROLE: &str = "tenant-read";
const VOTING_PORTAL_CLIENT: &str = "voting-portal";

/// Keep identity fields constant so each test changes only its policy input.
fn claims() -> JwtClaims {
    Claims::new(TENANT_ID, USER_ID)
        .azp(VOTING_PORTAL_CLIENT)
        .area(AREA_ID)
        .authorized_elections(&[ELECTION_ID])
        .roles([VOTER_ROLE, TENANT_READ_ROLE])
        .build()
}

#[test]
fn a_matching_tenant_still_needs_every_requested_permission() {
    let claims = claims();
    assert!(authorize(
        &claims,
        false,
        Some(TENANT_ID.into()),
        vec![Permissions::TENANT_READ],
    )
    .is_ok());

    // One allowed permission cannot conceal a missing write permission.
    let error = authorize(
        &claims,
        false,
        Some(TENANT_ID.into()),
        vec![Permissions::TENANT_READ, Permissions::TENANT_WRITE],
    )
    .expect_err("read permission must not authorize a write");
    assert_eq!(error.0, Status::Unauthorized);
}

#[test]
fn roles_cannot_authorize_a_different_or_unspecified_tenant() {
    for requested_tenant in [Some(OTHER_TENANT_ID.into()), None] {
        let error = authorize(
            &claims(),
            false,
            requested_tenant,
            vec![Permissions::TENANT_READ],
        )
        .expect_err("tenant isolation must be enforced before role checks");
        assert_eq!(error.0, Status::Unauthorized);
    }
}

#[test]
fn super_admin_access_requires_the_opt_in_tenant_and_permissions() {
    // This is the only test in this integration binary that reads or changes
    // this variable. Other integration binaries run in separate processes.
    let previous = std::env::var_os("SUPER_ADMIN_TENANT_ID");
    std::env::remove_var("SUPER_ADMIN_TENANT_ID");
    let missing_configuration = authorize(
        &claims(),
        true,
        Some(OTHER_TENANT_ID.into()),
        vec![Permissions::TENANT_READ],
    );

    std::env::set_var("SUPER_ADMIN_TENANT_ID", SUPER_ADMIN_TENANT_ID);
    let wrong_tenant = authorize(
        &claims(),
        true,
        Some(OTHER_TENANT_ID.into()),
        vec![Permissions::TENANT_READ],
    );
    let mut super_admin = claims();
    super_admin.hasura_claims.tenant_id = SUPER_ADMIN_TENANT_ID.into();
    let opted_out = authorize(
        &super_admin,
        false,
        Some(OTHER_TENANT_ID.into()),
        vec![Permissions::TENANT_READ],
    );
    let missing_permission = authorize(
        &super_admin,
        true,
        Some(OTHER_TENANT_ID.into()),
        vec![Permissions::TENANT_WRITE],
    );
    let allowed = authorize(
        &super_admin,
        true,
        Some(OTHER_TENANT_ID.into()),
        vec![Permissions::TENANT_READ],
    );

    // Restore the environment before asserting, including on an assertion
    // failure, so later test diagnostics see the original process state.
    match previous {
        Some(value) => std::env::set_var("SUPER_ADMIN_TENANT_ID", value),
        None => std::env::remove_var("SUPER_ADMIN_TENANT_ID"),
    }
    for denied in [
        missing_configuration,
        wrong_tenant,
        opted_out,
        missing_permission,
    ] {
        assert_eq!(denied.unwrap_err().0, Status::Unauthorized);
    }
    assert!(allowed.is_ok());
}

#[test]
fn voter_channels_are_selected_from_the_verified_client_id() {
    let cases = [
        (VOTING_PORTAL_CLIENT, VotingStatusChannel::ONLINE),
        ("voting-portal-kiosk", VotingStatusChannel::KIOSK),
        ("ivr-voting", VotingStatusChannel::TELEPHONE),
    ];
    for (client, expected_channel) in cases {
        let mut claims = claims();
        claims.azp = client.into();
        let result = authorize_voter_election(
            &claims,
            vec![VoterPermissions::CAST_VOTE],
            &ELECTION_ID.into(),
        )
        .expect("an authorized voter should retain their area and channel");
        assert_eq!(result, (AREA_ID.into(), expected_channel));
    }
}

#[test]
fn voter_access_requires_role_area_election_and_known_client() {
    let mut missing_role = claims();
    missing_role.hasura_claims.allowed_roles.clear();
    let mut missing_area = claims();
    missing_area.hasura_claims.area_id = None;
    let mut missing_elections = claims();
    missing_elections.hasura_claims.authorized_election_ids = None;
    let mut wrong_election = claims();
    wrong_election.hasura_claims.authorized_election_ids =
        Some(vec!["election-b".into()]);
    let mut unknown_client = claims();
    unknown_client.azp = "admin-portal".into();

    // Each case removes one prerequisite from a valid identity. Checking the
    // error detail ensures a different, earlier failure cannot mask the rule.
    let cases = [
        (missing_role, ""),
        (missing_area, "Missing area_id"),
        (missing_elections, "Not authorized to election"),
        (wrong_election, "Not authorized to election"),
        (unknown_client, "Unknown Client"),
    ];
    for (claims, message) in cases {
        let error = authorize_voter_election(
            &claims,
            vec![VoterPermissions::CAST_VOTE],
            &ELECTION_ID.into(),
        )
        .expect_err("a missing prerequisite must deny voter access");
        assert_eq!(error, (Status::Unauthorized, message.into()));
    }
}

#[test]
fn the_super_admin_tenant_is_looked_up_only_for_another_tenant() {
    let lookups = std::cell::Cell::new(0);
    let super_admin_tenant = || {
        lookups.set(lookups.get() + 1);
        Some(SUPER_ADMIN_TENANT_ID.to_string())
    };
    let own_tenant = authorize_with(
        &claims(),
        true,
        Some(TENANT_ID.into()),
        vec![Permissions::TENANT_READ],
        super_admin_tenant,
    );
    assert_eq!((own_tenant, lookups.get()), (Ok(()), 0));

    let super_admins_not_allowed = authorize_with(
        &claims(),
        false,
        Some(OTHER_TENANT_ID.into()),
        vec![Permissions::TENANT_READ],
        super_admin_tenant,
    );
    assert!(super_admins_not_allowed.is_err());
    assert_eq!(lookups.get(), 0);

    for requested_tenant in [Some(OTHER_TENANT_ID.to_string()), None] {
        let other_tenant = authorize_with(
            &claims(),
            true,
            requested_tenant,
            vec![Permissions::TENANT_READ],
            super_admin_tenant,
        );
        assert!(other_tenant.is_err());
    }
    assert_eq!(lookups.get(), 2);
}

#[test]
fn without_a_configured_super_admin_tenant_only_the_own_tenant_passes() {
    let unconfigured = || None;
    assert_eq!(
        authorize_with(
            &claims(),
            true,
            Some(OTHER_TENANT_ID.into()),
            vec![Permissions::TENANT_READ],
            unconfigured,
        ),
        Err((
            Status::Unauthorized,
            "SUPER_ADMIN_TENANT_ID must be set".to_string()
        ))
    );
    assert_eq!(
        authorize_with(
            &claims(),
            true,
            Some(TENANT_ID.into()),
            vec![Permissions::TENANT_READ],
            unconfigured,
        ),
        Ok(())
    );
}

#[test]
fn the_configured_super_admin_tenant_may_act_on_any_tenant() {
    let configured = || Some(SUPER_ADMIN_TENANT_ID.to_string());
    let mut super_admin = claims();
    super_admin.hasura_claims.tenant_id = SUPER_ADMIN_TENANT_ID.into();
    for requested_tenant in [Some(OTHER_TENANT_ID.to_string()), None] {
        assert_eq!(
            authorize_with(
                &super_admin,
                true,
                requested_tenant,
                vec![Permissions::TENANT_READ],
                configured,
            ),
            Ok(())
        );
    }
    assert_eq!(
        authorize_with(
            &claims(),
            true,
            Some(OTHER_TENANT_ID.into()),
            vec![Permissions::TENANT_READ],
            configured,
        ),
        Err((
            Status::Unauthorized,
            "Unathorized: not a super admin or invalid tenant_id Some(\"tenant-b\")"
                .to_string()
        ))
    );
}

#[test]
fn a_denial_lists_the_requested_permissions_in_order() {
    // One held role keeps the unordered set in the message deterministic.
    let claims = Claims::new(TENANT_ID, USER_ID)
        .roles([Permissions::TENANT_READ])
        .build();
    assert_eq!(
        authorize_with(
            &claims,
            false,
            Some(TENANT_ID.into()),
            vec![
                Permissions::TENANT_WRITE,
                Permissions::TENANT_READ,
                Permissions::TENANT_DELETE,
            ],
            || None,
        ),
        Err((
            Status::Unauthorized,
            r#"Unathorized: ["tenant-write", "tenant-read", "tenant-delete"] not in {"tenant-read"}"#
                .to_string()
        ))
    );
}
