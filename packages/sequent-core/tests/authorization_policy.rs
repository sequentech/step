// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Test authorization decisions after identity verification. These synthetic
//! claims are not signed tokens and do not test the JWT verification boundary.

#![cfg(all(feature = "default_features", feature = "keycloak"))]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(private_interfaces, private_bounds, unnameable_types)]
#![deny(rustdoc::missing_crate_level_docs, rustdoc::broken_intra_doc_links)]
#![deny(
    clippy::missing_docs_in_private_items,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::unwrap_used,
    clippy::panic,
    clippy::shadow_unrelated,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::indexing_slicing,
    clippy::future_not_send,
    clippy::arithmetic_side_effects,
    clippy::suspicious,
    clippy::complexity,
    clippy::style,
    clippy::perf,
    clippy::pedantic
)]

use support::TestResult;

mod support;

use rocket::http::Status;
use sequent_core::ballot::VotingStatusChannel;
use sequent_core::services::authorization::{
    authorize, authorize_voter_election,
};
use sequent_core::services::jwt::JwtClaims;
use sequent_core::types::permissions::{Permissions, VoterPermissions};
use serde_json::json;

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
fn claims() -> TestResult<JwtClaims> {
    Ok(serde_json::from_value(json!({
        "exp": 2_000_000_000,
        "iat": 1_900_000_000,
        "jti": "test-token",
        "iss": "https://identity.invalid",
        "sub": USER_ID,
        "typ": "Bearer",
        "azp": VOTING_PORTAL_CLIENT,
        "acr": "1",
        "allowed-origins": [],
        "scope": "openid",
        "email_verified": false,
        "https://hasura.io/jwt/claims": {
            "x-hasura-default-role": VOTER_ROLE,
            "x-hasura-tenant-id": TENANT_ID,
            "x-hasura-user-id": USER_ID,
            "x-hasura-area-id": AREA_ID,
            "authorized-election-ids": [ELECTION_ID],
            "x-hasura-allowed-roles": [VOTER_ROLE, TENANT_READ_ROLE]
        }
    }))?)
}

#[test]
fn a_matching_tenant_still_needs_every_requested_permission() -> TestResult {
    let claims = claims()?;
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
    .err()
    .ok_or("read permission must not authorize a write")?;
    assert_eq!(error.0, Status::Unauthorized);
    Ok(())
}

#[test]
fn roles_cannot_authorize_a_different_or_unspecified_tenant() -> TestResult {
    for requested_tenant in [Some(OTHER_TENANT_ID.into()), None] {
        let error = authorize(
            &claims()?,
            false,
            requested_tenant,
            vec![Permissions::TENANT_READ],
        )
        .err()
        .ok_or("tenant isolation must be enforced before role checks")?;
        assert_eq!(error.0, Status::Unauthorized);
    }
    Ok(())
}

#[test]
fn super_admin_access_requires_the_opt_in_tenant_and_permissions() -> TestResult
{
    // This is the only test in this integration binary that reads or changes
    // this variable. Other integration binaries run in separate processes.
    let fixture_claims = claims()?;
    let previous = std::env::var_os("SUPER_ADMIN_TENANT_ID");
    std::env::remove_var("SUPER_ADMIN_TENANT_ID");
    let missing_configuration = authorize(
        &fixture_claims,
        true,
        Some(OTHER_TENANT_ID.into()),
        vec![Permissions::TENANT_READ],
    );

    std::env::set_var("SUPER_ADMIN_TENANT_ID", SUPER_ADMIN_TENANT_ID);
    let wrong_tenant = authorize(
        &fixture_claims,
        true,
        Some(OTHER_TENANT_ID.into()),
        vec![Permissions::TENANT_READ],
    );
    let mut super_admin = fixture_claims.clone();
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
        assert_eq!(
            denied
                .err()
                .ok_or("expected the invalid input to be rejected")?
                .0,
            Status::Unauthorized
        );
    }
    assert!(allowed.is_ok());
    Ok(())
}

#[test]
fn voter_channels_are_selected_from_the_verified_client_id() -> TestResult {
    let cases = [
        (VOTING_PORTAL_CLIENT, VotingStatusChannel::ONLINE),
        ("voting-portal-kiosk", VotingStatusChannel::KIOSK),
        ("ivr-voting", VotingStatusChannel::TELEPHONE),
    ];
    for (client, expected_channel) in cases {
        let mut claims = claims()?;
        claims.azp = client.into();
        let result = authorize_voter_election(
            &claims,
            vec![VoterPermissions::CAST_VOTE],
            &ELECTION_ID.into(),
        );
        assert_eq!(result, Ok((AREA_ID.into(), expected_channel)));
    }
    Ok(())
}

#[test]
fn voter_access_requires_role_area_election_and_known_client() -> TestResult {
    let mut missing_role = claims()?;
    missing_role.hasura_claims.allowed_roles.clear();
    let mut missing_area = claims()?;
    missing_area.hasura_claims.area_id = None;
    let mut missing_elections = claims()?;
    missing_elections.hasura_claims.authorized_election_ids = None;
    let mut wrong_election = claims()?;
    wrong_election.hasura_claims.authorized_election_ids =
        Some(vec!["election-b".into()]);
    let mut unknown_client = claims()?;
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
        .err()
        .ok_or("a missing prerequisite must deny voter access")?;
        assert_eq!(error, (Status::Unauthorized, message.into()));
    }
    Ok(())
}
