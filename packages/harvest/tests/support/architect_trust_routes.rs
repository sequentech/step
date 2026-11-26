// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Exercise signed requests and committed state on a migrated disposable database.

use crate::request_boundaries::{
    http, is_isolated_child, run_isolated_with_postgres,
};
use crate::route_services::{json, post, rows, text, Services};
use crate::routes::apply_configuration_package::PACKAGE_PERMISSIONS;
use crate::test_claims::Claims;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use openssl::{
    bn::BigNumContext,
    ec::{EcGroup, EcKey, PointConversionForm},
    nid::Nid,
};
use rocket::{http::Status, local::asynchronous::Client};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use windmill::{
    postgres::configuration_packages::try_lock_package_work,
    services::{
        browser_package_policy as browser, managed_webauthn_roster as managed,
    },
};

const RP: &str = "architect.example.test";
const ORIGIN: &str = "https://architect.example.test:1443";
const KEEPER: &str = "0490df33-69ca-42cd-927d-178341a0ad92";
const SECOND_ADMIN: &str = "edfc7aed-0ef3-4c54-a155-db2bb9c54d70";
const VIEWER: &str = "6de1d9e0-fc80-45b0-8c33-13dfa2e64c18";
const MACHINE: &str = "service-reader-fixture";
const STATUS: &str = "/get-architect-signer-roster-status";
const PREVIEW: &str = "/preview-architect-signer-roster";
const ACTIVATE: &str = "/activate-architect-signer-roster";
const GET_POLICY: &str = "/get-architect-approval-policy";
const SET_POLICY: &str = "/set-architect-approval-policy";

fn in_fixture(test: &str) -> bool {
    if !is_isolated_child() {
        let output =
            run_isolated_with_postgres(test, "http://127.0.0.1:9", true);
        assert!(output.contains("running 1 test"), "{output}");
        assert!(
            output.contains("test result: ok. 1 passed; 0 failed;"),
            "{output}"
        );
        return false;
    }
    for prefix in ["HASURA_DB", "KEYCLOAK_DB"] {
        for (key, expected) in [
            ("HOST", "127.0.0.1"),
            ("USER", "test"),
            ("PASSWORD", "test"),
            ("DBNAME", "test"),
        ] {
            assert!(
                std::env::var(format!("{prefix}__{key}")).as_deref()
                    == Ok(expected),
                "{prefix} must use the disposable loopback test fixture"
            );
        }
    }
    true
}

async fn fixture() -> Services {
    let services = Services::on_test_database().await;
    let db: String =
        rows::query(&services.hasura, "SELECT current_database()", &[]).await
            [0]
        .get(0);
    assert!(db.starts_with("harvest_schema_"));
    // These routes use Windmill's process-global production pool, not the
    // injected adapter. Only the isolated child redirects it to its own fixture.
    std::env::set_var("HASURA_DB__DBNAME", db);
    services
}

fn claims(tenant: &str, subject: &str) -> Claims {
    Claims::new(tenant, subject).roles(PACKAGE_PERMISSIONS)
}

fn roster_directory(tenant: &str) -> http::HttpServer {
    let mut replies = vec![http::Exchange::json(
        "POST",
        "/realms/master/protocol/openid-connect/token",
        200,
        http::token_json(),
    )];
    // A bounded number of read-only answers for normal, rechecked and rejected operations.
    for (subject, path) in [
        (KEEPER, "/admin"),
        (SECOND_ADMIN, "/admin"),
        (VIEWER, "/elsewhere/admin"),
        (MACHINE, "/admin"),
    ] {
        for _ in 0..32 {
            let url = format!("/admin/realms/tenant-{tenant}/users/{subject}");
            replies.push(http::Exchange::json(
                "GET",
                &url,
                200,
                json!({"id":subject,"enabled":true,"username":subject}),
            ));
            replies.push(http::Exchange::json(
                "GET",
                &format!("{url}/groups"),
                200,
                json!([{"name":"admin","path":path}]),
            ));
        }
    }
    // The single-user endpoint omits the service-account indicator, even
    // with enabled and root-admin membership. Normal search excludes it.
    for _ in 0..32 {
        replies.push(http::Exchange::json(
            "GET",
            &format!("/admin/realms/tenant-{tenant}/users"),
            200,
            json!([{"id":KEEPER},{"id":SECOND_ADMIN}]),
        ));
    }
    http::HttpServer::start(replies)
}

fn roster(tenant: &str) -> Value {
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
    let mut context = BigNumContext::new().unwrap();
    let keys: Vec<_> = ["approver", "approver", "final-signer"]
        .into_iter()
        .enumerate()
        .map(|(i, role)| {
            let public = EcKey::generate(&group)
                .unwrap()
                .public_key()
                .to_bytes(
                    &group,
                    PointConversionForm::UNCOMPRESSED,
                    &mut context,
                )
                .unwrap();
            json!({
                "key": Uuid::new_v4().to_string(),
                "credential_id": URL_SAFE_NO_PAD.encode([i as u8 + 1]),
                "subject": format!("official-{i}"),
                "role": role,
                "algorithm": "es256",
                "public_key": URL_SAFE_NO_PAD.encode(public),
                "record_sha256": "ab".repeat(32),
            })
        })
        .collect();
    json!({"format":"sequent.webauthn-roster.v1", "rp_id": RP, "origins":[ORIGIN], "tenant": tenant, "keys": keys})
}

fn fingerprint(raw: &str) -> String {
    hex::encode(Sha256::digest(raw.as_bytes()))
}

fn candidate(tenant: &str, raw: &str) -> Value {
    json!({"tenant_id": tenant, "roster_json": raw, "expected_sha256": fingerprint(raw)})
}

fn activation(candidate: &Value, generation: &str) -> Value {
    let mut value = candidate.clone();
    value["generation"] = json!(generation);
    value
}

async fn request(
    client: &Client,
    path: &'static str,
    operator: &Claims,
    body: &Value,
    expected: Status,
) -> Value {
    let (status, response) =
        json(post(client, path, operator, body).await).await;
    assert_eq!(status, expected, "{path}: {response}");
    response
}

async fn error(
    client: &Client,
    path: &'static str,
    operator: &Claims,
    body: &Value,
    status: Status,
    code: &str,
) {
    let response = request(client, path, operator, body, status).await;
    assert_eq!(response["extensions"]["code"], code, "{path}: {response}");
}

async fn audit(
    services: &Services,
    tenant: &str,
    kind: &str,
) -> Vec<(String, Value)> {
    rows::query(
        &services.hasura,
        "SELECT user_id, details FROM sequent_backend.audit_log_outbox
         WHERE tenant_id = $1 AND details #>> '{object,type}' = $2",
        &[&Uuid::parse_str(tenant).unwrap(), &kind],
    )
    .await
    .into_iter()
    .map(|row| (row.get(0), row.get(1)))
    .collect()
}

#[test]
fn managed_roster_preview_activation_and_rotation_are_authenticated_and_atomic()
{
    const TEST: &str = "architect_trust_routes::managed_roster_preview_activation_and_rotation_are_authenticated_and_atomic";
    if !in_fixture(TEST) {
        return;
    }
    rows::run(async {
        let services = fixture().await;
        let tenant = rows::event(&services.hasura).await.tenant_id;
        let other = rows::event(&services.hasura).await.tenant_id;
        std::env::set_var(browser::NORMAL_SWITCH, managed::MODE);
        std::env::set_var(browser::TENANT, &tenant);
        std::env::set_var(managed::EXPECTED_RP_ID, RP);
        std::env::set_var(managed::EXPECTED_ORIGIN, ORIGIN);
        const LEGACY_ALLOWLIST: &str =
            "WINDMILL_WEBAUTHN_PACKAGES_ROSTER_ADMIN_SUBJECTS";
        std::env::remove_var(LEGACY_ALLOWLIST);
        let directory = roster_directory(&tenant);
        std::env::set_var("KEYCLOAK_URL", &directory.url);
        let client = services.client().await;
        let status_input = json!({"tenant_id": tenant});
        let keeper =
            claims(&tenant, KEEPER).auth_time(chrono::Utc::now().timestamp());
        let stale_login = claims(&tenant, KEEPER)
            .auth_time(chrono::Utc::now().timestamp() - 301);
        let viewer =
            claims(&tenant, VIEWER).auth_time(chrono::Utc::now().timestamp());
        let second_admin = claims(&tenant, SECOND_ADMIN)
            .auth_time(chrono::Utc::now().timestamp());
        let machine =
            claims(&tenant, MACHINE).auth_time(chrono::Utc::now().timestamp());
        let foreign = claims(&other, KEEPER);
        let a_doc = roster(&tenant);
        let a_raw = a_doc.to_string();
        let a = candidate(&tenant, &a_raw);
        let b_raw = roster(&tenant).to_string();
        let b = candidate(&tenant, &b_raw);
        let inactive =
            request(&client, STATUS, &keeper, &status_input, Status::Ok).await;
        assert_eq!(
            inactive,
            json!({"mode":"portal_managed", "generation":"0", "active_sha256":null, "required_approvals":2, "can_manage":true})
        );
        let reader =
            request(&client, STATUS, &viewer, &status_input, Status::Ok).await;
        assert_eq!(reader["can_manage"], false);
        assert_eq!(
            request(&client, STATUS, &second_admin, &status_input, Status::Ok)
                .await["can_manage"],
            true
        );
        assert_eq!(
            request(&client, STATUS, &machine, &status_input, Status::Ok).await
                ["can_manage"],
            false
        );
        let without_package_permissions = Claims::new(&tenant, SECOND_ADMIN);
        error(
            &client,
            PREVIEW,
            &without_package_permissions,
            &a,
            Status::Unauthorized,
            "Unauthorized",
        )
        .await;
        let foreign_status = request(
            &client,
            STATUS,
            &foreign,
            &json!({"tenant_id": other}),
            Status::Ok,
        )
        .await;
        assert_eq!(foreign_status["mode"], "not_enabled");
        assert!(foreign_status["active_sha256"].is_null());

        error(
            &client,
            PREVIEW,
            &viewer,
            &a,
            Status::Forbidden,
            "architect-roster-management-forbidden",
        )
        .await;
        for (path, input) in
            [(PREVIEW, a.clone()), (ACTIVATE, activation(&a, "0"))]
        {
            error(
                &client,
                path,
                &machine,
                &input,
                Status::Forbidden,
                "architect-roster-management-forbidden",
            )
            .await;
        }
        error(
            &client,
            PREVIEW,
            &foreign,
            &a,
            Status::Unauthorized,
            "Unauthorized",
        )
        .await;
        error(
            &client,
            ACTIVATE,
            &stale_login,
            &activation(&a, "0"),
            Status::Forbidden,
            "architect-roster-fresh-auth-required",
        )
        .await;
        error(
            &client,
            ACTIVATE,
            &keeper,
            &activation(&a, "00"),
            Status::BadRequest,
            "architect-roster-invalid",
        )
        .await;
        let mut wrong_origin = a_doc.clone();
        wrong_origin["origins"] = json!(["https://different.example.test"]);
        let mut missing_approval = a_doc.clone();
        missing_approval["keys"].as_array_mut().unwrap().remove(0);
        let mut mismatch = a.clone();
        mismatch["expected_sha256"] = b["expected_sha256"].clone();
        let mut noncanonical_digest = a.clone();
        noncanonical_digest["expected_sha256"] = json!("AB".repeat(32));
        for invalid in [
            mismatch,
            noncanonical_digest,
            candidate(&tenant, "{"),
            candidate(&tenant, &wrong_origin.to_string()),
            candidate(&tenant, &missing_approval.to_string()),
        ] {
            error(
                &client,
                PREVIEW,
                &keeper,
                &invalid,
                Status::BadRequest,
                "architect-roster-invalid",
            )
            .await;
        }
        let preview = request(&client, PREVIEW, &keeper, &a, Status::Ok).await;
        assert_eq!(
            preview,
            json!({"generation":"0","candidate_sha256":a["expected_sha256"],"approver_count":2,"final_signer_count":1,"rp_id":RP,"origins":[ORIGIN]})
        );
        assert!(audit(&services, &tenant, "architect_signer_roster")
            .await
            .is_empty());

        let mut lock_client = services.hasura.get().await.unwrap();
        let lock = lock_client.transaction().await.unwrap();
        assert!(try_lock_package_work(&lock, &tenant).await.unwrap());
        error(
            &client,
            ACTIVATE,
            &keeper,
            &activation(&a, "0"),
            Status::Conflict,
            "architect-roster-package-busy",
        )
        .await;
        lock.rollback().await.unwrap();

        let activated = request(
            &client,
            ACTIVATE,
            &keeper,
            &activation(&a, "0"),
            Status::Ok,
        )
        .await;
        assert_eq!(activated["generation"], "1");
        assert_eq!(activated["active_sha256"], a["expected_sha256"]);
        assert_eq!(
            request(&client, STATUS, &keeper, &status_input, Status::Ok).await,
            activated
        );
        // A prior installation may name only the first administrator. It
        // must not change status, preview or activation for an eligible second one.
        std::env::set_var(LEGACY_ALLOWLIST, KEEPER);
        assert_eq!(
            request(&client, STATUS, &second_admin, &status_input, Status::Ok)
                .await["can_manage"],
            true
        );
        let b_preview =
            request(&client, PREVIEW, &second_admin, &b, Status::Ok).await;
        assert_eq!(b_preview["generation"], "1");
        assert_eq!(
            request(
                &client,
                ACTIVATE,
                &keeper,
                &activation(&a, "1"),
                Status::Ok
            )
            .await,
            activated
        );
        error(
            &client,
            ACTIVATE,
            &keeper,
            &activation(&b, "0"),
            Status::Conflict,
            "architect-roster-stale-generation",
        )
        .await;
        let rotated = request(
            &client,
            ACTIVATE,
            &second_admin,
            &activation(&b, "1"),
            Status::Ok,
        )
        .await;
        assert_eq!(rotated["generation"], "2");
        assert_eq!(rotated["active_sha256"], b["expected_sha256"]);
        let ids = Uuid::parse_str(&tenant).unwrap();
        let stored = rows::query(&services.hasura,
            "SELECT roster_json, activated_by, generation FROM sequent_backend.managed_webauthn_roster WHERE tenant_id = $1",
            &[&ids],
        ).await;
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].get::<_, String>(0), b_raw);
        assert_eq!(stored[0].get::<_, String>(1), SECOND_ADMIN);
        assert_eq!(stored[0].get::<_, i64>(2), 2);
        let entries =
            audit(&services, &tenant, "architect_signer_roster").await;
        assert_eq!(entries.len(), 2, "only actual rotations audit");
        for (generation, digest, previous) in
            [(1, &a["expected_sha256"], 0), (2, &b["expected_sha256"], 1)]
        {
            let (user, details) = entries
                .iter()
                .find(|(_, d)| d["extra"]["generation"] == generation)
                .unwrap();
            assert_eq!(
                user,
                if generation == 1 {
                    KEEPER
                } else {
                    SECOND_ADMIN
                }
            );
            assert_eq!(
                details["extra"],
                json!({"generation":generation,"previous_generation":previous,"fingerprint_confirmation":"operator_supplied"})
            );
            assert_eq!(&details["changes"][0]["new"], digest);
            assert!(
                !details.to_string().contains("public_key"),
                "only fingerprints belong in audit"
            );
        }

        // Losing the trusted deployment config cannot turn an existing
        // installation into an unprotected tenant or a writable fallback.
        for var in [browser::NORMAL_SWITCH, browser::TENANT]
            .into_iter()
            .chain(managed::MANAGED_VARS)
        {
            std::env::remove_var(var);
        }
        assert_eq!(
            request(&client, STATUS, &keeper, &status_input, Status::Ok).await
                ["mode"],
            "unavailable",
        );
        let pinned_roster = roster(&other).to_string();
        let pinned_path = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(pinned_path.path(), &pinned_roster).unwrap();
        let pinned_digest = fingerprint(&pinned_roster);
        std::env::set_var(browser::NORMAL_SWITCH, browser::NORMAL_SWITCH_VALUE);
        std::env::set_var(browser::TENANT, &other);
        std::env::set_var(browser::ROSTER_FILE, pinned_path.path());
        std::env::set_var(browser::ROSTER_SHA256, &pinned_digest);
        std::env::set_var(browser::REQUIRED_APPROVALS, "2");
        assert_eq!(
            request(
                &client,
                STATUS,
                &foreign,
                &json!({"tenant_id":other}),
                Status::Ok
            )
            .await,
            json!({"mode":"site_pinned","generation":"0","active_sha256":pinned_digest,"required_approvals":2,"can_manage":false}),
        );
        error(
            &client,
            ACTIVATE,
            &foreign,
            &activation(&candidate(&other, &pinned_roster), "0"),
            Status::Forbidden,
            "architect-roster-management-forbidden",
        )
        .await;
        assert!(audit(&services, &other, "architect_signer_roster")
            .await
            .is_empty());
        for var in [
            browser::ROSTER_FILE,
            browser::ROSTER_SHA256,
            browser::REQUIRED_APPROVALS,
        ] {
            std::env::remove_var(var);
        }
        std::env::set_var(browser::NORMAL_SWITCH, managed::MODE);
        std::env::set_var(browser::TENANT, &tenant);
        std::env::set_var(managed::EXPECTED_RP_ID, RP);
        std::env::set_var(managed::EXPECTED_ORIGIN, ORIGIN);

        // Corrupt trust only in the synthetic fixture. No edit may replace it
        // blindly, and reads must never advertise it as an active authority.
        rows::execute(&services.hasura,
            "UPDATE sequent_backend.managed_webauthn_roster SET sha256 = repeat('0',64) WHERE tenant_id = $1", &[&ids],
        ).await;
        let unavailable =
            request(&client, STATUS, &keeper, &status_input, Status::Ok).await;
        assert_eq!(unavailable["mode"], "unavailable");
        assert!(unavailable["active_sha256"].is_null());
        error(
            &client,
            PREVIEW,
            &keeper,
            &a,
            Status::InternalServerError,
            "architect-roster-unavailable",
        )
        .await;
        error(
            &client,
            ACTIVATE,
            &keeper,
            &activation(&a, "2"),
            Status::InternalServerError,
            "architect-roster-unavailable",
        )
        .await;
        assert_eq!(
            audit(&services, &tenant, "architect_signer_roster")
                .await
                .len(),
            2
        );
    });
}

#[test]
fn tenant_policy_after_defaults_changes_and_reset_remains_scoped_and_audited() {
    const TEST: &str = "architect_trust_routes::tenant_policy_after_defaults_changes_and_reset_remains_scoped_and_audited";
    if !in_fixture(TEST) {
        return;
    }
    rows::run(async {
        let services = fixture().await;
        let tenant = rows::event(&services.hasura).await.tenant_id;
        let other = rows::event(&services.hasura).await.tenant_id;
        let client = services.client().await;
        let operator = claims(&tenant, "fixture-policy-keeper");
        let input = json!({"tenant_id": tenant});
        let write = |threshold: Option<i32>, generation: &str| json!({"tenant_id":tenant, "required_approvals":threshold, "generation":generation});
        let read = |threshold: Option<i32>, generation: &str| json!({"required_approvals":threshold, "generation":generation});

        assert_eq!(
            request(&client, GET_POLICY, &operator, &input, Status::Ok).await,
            read(None, "0")
        );
        assert_eq!(
            request(
                &client,
                SET_POLICY,
                &operator,
                &write(None, "0"),
                Status::Ok
            )
            .await,
            read(None, "0")
        );
        let (invalid, explanation) = text(
            post(&client, SET_POLICY, &operator, &write(Some(0), "0")).await,
        )
        .await;
        assert_eq!(invalid, Status::BadRequest);
        assert!(explanation.contains("integer from 1 to 16"));
        assert_eq!(
            request(
                &client,
                SET_POLICY,
                &operator,
                &write(Some(1), "0"),
                Status::Ok
            )
            .await,
            read(Some(1), "1")
        );
        let (stale, explanation) = text(
            post(&client, SET_POLICY, &operator, &write(Some(16), "0")).await,
        )
        .await;
        assert_eq!(stale, Status::Conflict);
        assert!(explanation.contains("reload Tenant Settings"));
        assert_eq!(
            request(
                &client,
                SET_POLICY,
                &operator,
                &write(Some(16), "1"),
                Status::Ok
            )
            .await,
            read(Some(16), "2")
        );
        assert_eq!(
            request(&client, GET_POLICY, &operator, &input, Status::Ok).await,
            read(Some(16), "2")
        );
        assert_eq!(
            request(
                &client,
                SET_POLICY,
                &operator,
                &write(None, "2"),
                Status::Ok
            )
            .await,
            read(None, "3")
        );
        assert_eq!(
            request(&client, GET_POLICY, &operator, &input, Status::Ok).await,
            read(None, "3")
        );
        let foreign = claims(&other, "fixture-foreign-policy-keeper");
        assert_eq!(
            post(&client, GET_POLICY, &foreign, &input).await.status(),
            Status::Unauthorized
        );
        assert_eq!(
            request(
                &client,
                GET_POLICY,
                &foreign,
                &json!({"tenant_id":other}),
                Status::Ok
            )
            .await,
            read(None, "0")
        );
        let missing = Uuid::new_v4().to_string();
        let missing_claims = claims(&missing, "fixture-missing-policy-keeper");
        for (path, body) in [
            (GET_POLICY, json!({"tenant_id":missing})),
            (
                SET_POLICY,
                json!({"tenant_id":missing,"required_approvals":1,"generation":"0"}),
            ),
        ] {
            assert_eq!(
                text(post(&client, path, &missing_claims, &body).await).await,
                (Status::NotFound, "Tenant not found".to_owned()),
            );
        }
        let entries = audit(&services, &tenant, "tenant").await;
        let entries: Vec<_> = entries
            .into_iter()
            .filter(|(_, d)| {
                d["changes"][0]["path"]
                    == "architect_approval_policy.required_approvals"
            })
            .collect();
        assert_eq!(entries.len(), 3);
        for (generation, previous, effective) in
            [(1, 2, 1), (2, 1, 16), (3, 16, 2)]
        {
            let (user, details) = entries
                .iter()
                .find(|(_, d)| d["extra"]["generation"] == generation)
                .unwrap();
            assert_eq!(user, "fixture-policy-keeper");
            assert_eq!(
                details["extra"],
                json!({"generation":generation,"previous_effective":previous,"effective":effective})
            );
        }
    });
}
