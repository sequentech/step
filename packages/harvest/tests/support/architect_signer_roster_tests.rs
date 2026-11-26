// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Lifecycle tests for the narrowly scoped receiver trust administration gate.

use super::*;
use crate::services::authorization::test_claims::{
    admin_claims, CALLER_TENANT_ID, OTHER_TENANT_ID,
};

#[test]
fn trust_management_checks_exact_tenant_and_every_required_permission() {
    let role_names =
        PACKAGE_PERMISSIONS.map(|permission| permission.to_string());
    let roles: Vec<&str> = role_names.iter().map(String::as_str).collect();
    let admin = admin_claims(CALLER_TENANT_ID, &roles);
    assert!(authorize_tenant(&admin, CALLER_TENANT_ID).is_ok());
    assert_eq!(
        authorize_tenant(&admin, OTHER_TENANT_ID).unwrap_err().0,
        Status::Unauthorized
    );
    assert_eq!(
        authorize_tenant(&admin, &CALLER_TENANT_ID.to_uppercase())
            .unwrap_err()
            .0,
        Status::BadRequest
    );
    for omitted in 0..roles.len() {
        let mut incomplete = roles.clone();
        incomplete.remove(omitted);
        assert!(authorize_tenant(
            &admin_claims(CALLER_TENANT_ID, &incomplete),
            CALLER_TENANT_ID
        )
        .is_err());
    }
}

#[test]
fn refresh_time_does_not_substitute_for_recent_authentication() {
    let mut admin = admin_claims(CALLER_TENANT_ID, &[]);
    let now = 1_500_000_000;
    admin.iat = now;
    assert!(!is_fresh_login(&admin, now));
    admin.auth_time = Some(now + 1);
    assert!(!is_fresh_login(&admin, now));
    admin.auth_time = Some(now - FRESH_AUTH_SECONDS - 1);
    assert!(!is_fresh_login(&admin, now));
    admin.auth_time = Some(now - FRESH_AUTH_SECONDS);
    assert!(is_fresh_login(&admin, now));
    admin.auth_time = Some(now);
    assert!(is_fresh_login(&admin, now));
}

#[test]
fn fingerprint_and_generation_have_single_canonical_forms() {
    let digest = "af".repeat(32);
    assert!(valid_fingerprint(&digest));
    for invalid in ["", "af", &digest.to_uppercase(), &format!("{digest}\n")] {
        assert!(!valid_fingerprint(invalid));
    }
    for (input, expected) in
        [("0", 0), ("41", 41), ("9223372036854775807", i64::MAX)]
    {
        assert_eq!(parse_generation(input).ok(), Some(expected));
    }
    for invalid in ["", "1.0", "01", "-1", "+1", " 1", "9223372036854775808"] {
        assert_eq!(
            parse_generation(invalid).unwrap_err().0,
            Status::BadRequest
        );
    }
}

#[test]
fn action_errors_distinguish_fresh_login_and_retryable_conflicts() {
    assert_eq!(fresh_auth_required().0, Status::Forbidden);
    assert_eq!(
        fresh_auth_required().1.extensions.code,
        "architect-roster-fresh-auth-required"
    );
    assert_eq!(stale().0, Status::Conflict);
    assert_eq!(
        stale().1.extensions.code,
        "architect-roster-stale-generation"
    );
    assert_eq!(busy().0, Status::Conflict);
    assert_eq!(busy().1.extensions.code, "architect-roster-package-busy");
}

#[test]
fn roster_admin_requires_a_verified_tenant_realm_portal_access_token() {
    let mut claims = admin_claims(CALLER_TENANT_ID, &[]);
    let tenant_issuer =
        format!("https://keycloak.example/realms/tenant-{CALLER_TENANT_ID}");
    claims.iss = tenant_issuer.clone();
    assert!(is_tenant_portal_token(&claims, CALLER_TENANT_ID));
    claims.iss = format!("{tenant_issuer}-event-sample");
    assert!(!is_tenant_portal_token(&claims, CALLER_TENANT_ID));
    claims.iss = tenant_issuer;
    claims.typ = "ID".into();
    assert!(!is_tenant_portal_token(&claims, CALLER_TENANT_ID));
    claims.typ = "Bearer".into();
    claims.azp = "voting-portal".into();
    assert!(!is_tenant_portal_token(&claims, CALLER_TENANT_ID));
}

#[test]
fn human_directory_search_is_a_bounded_quoted_term() {
    assert_eq!(
        human_directory_search("id:admin_100%@example.test").as_deref(),
        Some("\"id:admin_100%@example.test\"")
    );
    for bad in ["", "a b", "a\nb", "a\"b", "a\u{2003}b", &"a".repeat(256)] {
        assert_eq!(human_directory_search(bad), None);
    }
}

#[test]
fn roster_manager_uses_verified_current_subject_not_username() {
    let role_names =
        PACKAGE_PERMISSIONS.map(|permission| permission.to_string());
    let roles: Vec<&str> = role_names.iter().map(String::as_str).collect();
    let mut claims = admin_claims(CALLER_TENANT_ID, &roles);
    claims.iss =
        format!("https://keycloak.example/realms/tenant-{CALLER_TENANT_ID}");
    for opaque_subject in [
        "0490df33-69ca-42cd-927d-178341a0ad92",
        "opaque-federated-provider-subject",
    ] {
        claims.sub = opaque_subject.into();
        claims.hasura_claims.user_id = opaque_subject.into();
        claims.preferred_username = None;
        assert!(manager_claims(&claims, CALLER_TENANT_ID));
    }
    assert!(!manager_claims(&claims, OTHER_TENANT_ID));
    let actual_user = claims.hasura_claims.user_id.clone();
    claims.hasura_claims.user_id = "different-user".into();
    claims.preferred_username = Some("admin".into());
    assert!(!manager_claims(&claims, CALLER_TENANT_ID));
    claims.sub = "wrong%2fsubject".into();
    claims.hasura_claims.user_id = claims.sub.clone();
    assert!(!manager_claims(&claims, CALLER_TENANT_ID));
    claims.sub = actual_user.clone();
    claims.hasura_claims.user_id = actual_user;
    claims.hasura_claims.allowed_roles.pop();
    assert!(!manager_claims(&claims, CALLER_TENANT_ID));
}
