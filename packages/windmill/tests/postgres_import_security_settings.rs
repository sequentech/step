// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The security settings a bundle may carry on an election event import
//! (signing rules, certificate checks, an approval matrix). Where only
//! signed packages import, the new election event starts with the defaults
//! and its administrator and board set it up in Preparation, up to the
//! signatures that open voting; where they are optional, the import takes
//! them.

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing.rs"]
mod signing;
#[path = "support/signing_pki.rs"]
mod signing_pki;

use async_trait::async_trait;
use deadpool_postgres::Transaction;
use sequent_core::election_config::ImportElectionEventSchema;
use sequent_core::signing::{
    CertificateCheckId, RequesterSigning, SigningAction, SigningChecks, SigningRequestStatus,
    SigningRequirement, SigningScope,
};
use sequent_core::types::permissions::Permissions;
use serde_json::{json, Value};
use signing::*;
use signing_pki::{Issued, Pki, CRL_URL, ROOT_CRL_URL};
use std::collections::HashMap;
use std::sync::Arc;
use strum::IntoEnumIterator;
use uuid::Uuid;
use windmill::postgres::signing::{get_signing_checks, list_signing_rules};
use windmill::services::approval_matrix::store::get_latest_approval_matrix;
use windmill::services::import::import_election_event::{
    import_bundle_security_settings, imports_security_settings, APPROVAL_MATRIX_NOT_IMPORTED,
    SIGNING_CHECKS_NOT_IMPORTED, SIGNING_RULES_NOT_IMPORTED,
};
use windmill::services::signing::approve::{
    approve, ApproveInput, ApproveOutcome, NoDocumentSigner, SigningServices,
};
use windmill::services::signing::certificates::{CertificateIdentity, OpensslCertificateVerifier};
use windmill::services::signing::crl::{refresh_chain_crls, refresh_event_crls, CrlFetcher};
use windmill::services::signing::directory::UserDirectory;
use windmill::services::signing::executors::SigningExecutorRegistry;
use windmill::services::signing::guard::{GuardOutcome, GuardRequest};
use windmill::services::signing::issuers::{import_staff_issuers, update_signing_checks};
use windmill::services::signing::log::Actor;
use windmill::services::signing::rules::{save_rule, SaveRuleInput};
use windmill::services::signing::{
    Allowance, InvalidReason, SigningCaller, SigningError, SigningResult,
};

/// Serves the PKI's revocation lists.
struct Lists;

#[async_trait]
impl CrlFetcher for Lists {
    async fn fetch(&self, url: &str) -> anyhow::Result<Vec<u8>> {
        let pki = Pki::get();
        match url {
            CRL_URL => Ok(pki.individual_crl()),
            ROOT_CRL_URL => Ok(pki.root_crl()),
            _ => anyhow::bail!("connection refused"),
        }
    }
}

struct Names;

#[async_trait]
impl UserDirectory for Names {
    async fn display_names(
        &self,
        _: Uuid,
        user_ids: &[String],
    ) -> anyhow::Result<HashMap<String, String>> {
        Ok(user_ids
            .iter()
            .map(|user| (user.clone(), format!("Display {user}")))
            .collect())
    }
}

fn actor(user: &str) -> Actor {
    Actor {
        user_id: user.into(),
        username: user.into(),
    }
}

/// A bundle for `w`'s event that carries signing rules, certificate checks
/// and an approval matrix.
fn bundle(w: &World) -> ImportElectionEventSchema {
    let (tenant, event) = (w.tenant.to_string(), w.event.to_string());
    let rule = |action: &str| {
        json!({"action": action, "requirement": "required", "signatures": 1,
               "requester_signing": "allowed"})
    };
    serde_json::from_value(json!({
        "tenant_id": tenant,
        "keycloak_event_realm": null,
        "election_event": {
            "id": event, "tenant_id": tenant,
            "is_archived": false, "encryption_protocol": "RSA256"
        },
        "elections": [], "contests": [], "candidates": [], "areas": [], "area_contests": [],
        "scheduled_events": null, "reports": [], "keys_ceremonies": [], "applications": [],
        "version": "v10.0.0",
        "signing_rules": [
            rule("open-voting"), rule("approve-configuration"), rule("generate-reports")
        ],
        "signing_checks": {"revocation_check": "dont-check",
            "crl_unavailable": "accept-unchecked",
            "registration": "security-officer-only", "post_binding": "any-post"},
        "approval_matrix": {
            "compared_fields": ["firstName", "lastName", "dateOfBirth"],
            "rules": [
                {"when": {"already_enrolled": true},
                 "then": {"decision": "REJECTED", "reason": "ALREADY_APPROVED"}},
                {"when": {"differing": "none"}, "then": {"decision": "ACCEPTED"}}
            ],
            "otherwise": {"decision": "PENDING", "reason": "NO_VOTER"}
        },
    }))
    .unwrap()
}

/// Sets the tenant's configuration signing `policy`. The settings guard
/// refuses any once the tenant has an election event: it is written past it.
async fn set_policy(w: &World, policy: &str) {
    let root = Pki::get().root.pem();
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    tx.batch_execute("SET LOCAL session_replication_role = replica")
        .await
        .unwrap();
    tx.execute(
        "UPDATE sequent_backend.tenant SET settings = $2 WHERE id = $1",
        &[
            &w.tenant,
            &json!({"configuration_signing": {
                "policy": policy,
                "package_roots": root,
                "staff_roots": root,
                "required_approvals": 2,
            }}),
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

/// Imports what `bundle` carries of the security settings into `w`'s event,
/// committed: the notes for the task's log.
async fn import(w: &World, bundle: &ImportElectionEventSchema) -> Vec<String> {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let notes = import_bundle_security_settings(
        &tx,
        &w.tenant.to_string(),
        &w.event.to_string(),
        bundle,
        None,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    notes
}

/// Whether the tenant's imports take security settings, and what `w`'s event
/// holds: its saved rules (action, signatures), whether it has certificate
/// checks and whether it has an approval matrix.
async fn held(w: &World) -> (bool, Vec<(String, u16)>, bool, bool) {
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let (tenant, event) = (w.tenant.to_string(), w.event.to_string());
    let mut rules: Vec<(String, u16)> = list_signing_rules(&tx, w.tenant, w.event)
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.rule.action.to_string(), row.rule.signatures))
        .collect();
    rules.sort();
    (
        imports_security_settings(&tx, &tenant).await.unwrap(),
        rules,
        get_signing_checks(&tx, w.tenant, w.event)
            .await
            .unwrap()
            .is_some(),
        get_latest_approval_matrix(&tx, &tenant, &event)
            .await
            .unwrap()
            .is_some(),
    )
}

#[tokio::test]
async fn where_only_signed_packages_import_the_bundles_security_settings_are_left_out() {
    // A tenant without the setting requires signed packages.
    for policy in [None, Some("required")] {
        let w = world("imported-post").await;
        if let Some(policy) = policy {
            set_policy(&w, policy).await;
        }
        assert_eq!(
            import(&w, &bundle(&w)).await,
            [
                SIGNING_RULES_NOT_IMPORTED,
                SIGNING_CHECKS_NOT_IMPORTED,
                APPROVAL_MATRIX_NOT_IMPORTED
            ],
            "{policy:?}"
        );
        assert_eq!(held(&w).await, (false, vec![], false, false), "{policy:?}");
        assert!(w.steps().await.is_empty(), "{policy:?}");
        // A note only for what the bundle carries.
        let mut checks_only = bundle(&w);
        checks_only.signing_rules = None;
        checks_only.approval_matrix = None;
        assert_eq!(
            import(&w, &checks_only).await,
            [SIGNING_CHECKS_NOT_IMPORTED],
            "{policy:?}"
        );
    }
}

#[tokio::test]
async fn where_signed_packages_are_optional_the_import_takes_them() {
    let w = world("imported-post").await;
    set_policy(&w, "optional").await;
    assert!(import(&w, &bundle(&w)).await.is_empty());
    assert_eq!(
        held(&w).await,
        (
            true,
            vec![
                ("approve-configuration".to_string(), 1),
                ("generate-reports".to_string(), 1),
                ("open-voting".to_string(), 1)
            ],
            true,
            true
        )
    );
}

/// The signing preset the janitor adds to every bundle it builds.
const JANITOR_PRESET: &str = include_str!("../external-bin/janitor/templates/COMELEC/signing.json");
/// The approval matrix the janitor adds to every bundle it builds.
const JANITOR_MATRIX: &str =
    include_str!("../external-bin/janitor/templates/COMELEC/approvalMatrix.json");

#[tokio::test]
async fn a_janitor_bundle_brings_its_preset_only_where_signed_packages_are_optional() {
    let preset: Value = serde_json::from_str(JANITOR_PRESET).unwrap();
    let carried = |w: &World| {
        let mut carried = bundle(w);
        carried.signing_rules =
            Some(serde_json::from_value(preset["signing_rules"].clone()).unwrap());
        carried.signing_checks =
            Some(serde_json::from_value(preset["signing_checks"].clone()).unwrap());
        carried.approval_matrix = Some(serde_json::from_str(JANITOR_MATRIX).unwrap());
        carried
    };
    let w = world("imported-post").await;
    assert_eq!(
        import(&w, &carried(&w)).await,
        [
            SIGNING_RULES_NOT_IMPORTED,
            SIGNING_CHECKS_NOT_IMPORTED,
            APPROVAL_MATRIX_NOT_IMPORTED
        ]
    );
    assert_eq!(held(&w).await, (false, vec![], false, false));

    // Where they are optional, the preset's rules as it sets them.
    let w = world("imported-post").await;
    set_policy(&w, "optional").await;
    assert!(import(&w, &carried(&w)).await.is_empty());
    let mut expected: Vec<(String, u16)> = preset["signing_rules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|rule| {
            (
                rule["action"].as_str().unwrap().to_string(),
                u16::try_from(rule["signatures"].as_u64().unwrap()).unwrap(),
            )
        })
        .collect();
    expected.sort();
    assert_eq!(held(&w).await, (true, expected, true, true));
}

/// Two enabled people of the tenant's realm who can sign every action at
/// `w`'s Post, in the Keycloak tables of `ktx`.
async fn directory(ktx: &Transaction<'_>, w: &World) {
    keycloak_tables(ktx).await;
    ktx.batch_execute(&format!(
        "INSERT INTO realm VALUES ('r1', 'tenant-{tenant}');
         INSERT INTO keycloak_group (id, name, realm_id) VALUES ('g', 'board', 'r1');
         INSERT INTO group_attribute VALUES ('permission_labels', '{label}', 'g');
         INSERT INTO user_entity VALUES ('u1', 'signer-1', 'One', 'Last', true, 'r1', NULL),
             ('u2', 'signer-2', 'Two', 'Last', true, 'r1', NULL);
         INSERT INTO user_group_membership VALUES ('g', 'u1'), ('g', 'u2');",
        tenant = w.tenant,
        label = w.label,
    ))
    .await
    .unwrap();
    for action in SigningAction::iter() {
        let role = format!("role-{action}");
        ktx.execute(
            "INSERT INTO keycloak_role (id, name, realm_id) VALUES ($1, $2, 'r1')",
            &[&role, &action.sign_permission().to_string()],
        )
        .await
        .unwrap();
        ktx.execute("INSERT INTO group_role_mapping VALUES ($1, 'g')", &[&role])
            .await
            .unwrap();
    }
}

fn rule(
    action: SigningAction,
    requirement: SigningRequirement,
    signatures: u16,
    expected_revision: i64,
) -> SaveRuleInput {
    SaveRuleInput {
        action,
        requirement,
        signatures,
        requester_signing: RequesterSigning::Allowed,
        expires_minutes: Some(60),
        roles: None,
        expected_revision,
    }
}

/// The administrator saves `input` on `w`'s event.
async fn save(
    htx: &Transaction<'_>,
    ktx: &Transaction<'_>,
    w: &World,
    input: SaveRuleInput,
) -> SigningResult<()> {
    let administrator = caller("administrator", &[Permissions::SIGNING_RULES_WRITE], &[]);
    save_rule(htx, ktx, &administrator, w.tenant, w.event, &input)
        .await
        .map(|_| ())
}

fn reason<T: std::fmt::Debug>(result: SigningResult<T>) -> InvalidReason {
    match result {
        Err(SigningError::Invalid { reason, .. }) => reason,
        other => panic!("expected an invalid input, got {other:?}"),
    }
}

fn refused(result: SigningResult<ApproveOutcome>) -> CertificateCheckId {
    match result {
        Err(SigningError::Refused { check, .. }) => check,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// A request of `action` in its scope: the election event, or `w`'s Post.
fn request(w: &World, action: SigningAction, subject: Value) -> GuardRequest {
    let mut scope = w.scope(action);
    if matches!(action.scope(), SigningScope::Event) {
        scope.election_id = None;
    }
    GuardRequest {
        action,
        scope,
        subject,
        document: None,
        config_revision: None,
    }
}

/// Starts a request of `action` that needs signatures.
async fn start(w: &World, caller: &SigningCaller, action: SigningAction, subject: Value) -> Uuid {
    match w
        .guard_request(caller, &request(w, action, subject), at(0))
        .await
        .unwrap()
    {
        GuardOutcome::SigningRequired(summary) => summary.id,
        GuardOutcome::Proceed => panic!("{action} needs no signatures"),
    }
}

/// The administrator trusts the PKI's issuers; the event holds their
/// current lists.
async fn trust(w: &World) {
    let pki = Pki::get();
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    import_staff_issuers(
        &tx,
        w.tenant,
        w.event,
        &[pki.root.cert.clone(), pki.individual_ca.cert.clone()],
        &actor("administrator"),
        Allowance::Permission(Permissions::SIGNING_ISSUERS_WRITE),
        at(0),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    refresh_event_crls(&mut client, w.tenant, w.event, &Lists, at(0))
        .await
        .unwrap();
}

/// `user` signs request `id` with `signer`'s key, as the browser does.
async fn sign(
    w: &World,
    services: &SigningServices,
    user: &str,
    id: Uuid,
    signer: &Issued,
) -> SigningResult<ApproveOutcome> {
    let request = w.request(id).await;
    let algorithm = CertificateIdentity::of(&signer.cert, &[])
        .unwrap()
        .key_algorithm
        .unwrap();
    let mut client = w.pool.get().await.unwrap();
    // As the approve route does before approving.
    refresh_chain_crls(
        &mut client,
        w.tenant,
        w.event,
        &[signer.pem()],
        &Lists,
        at(1),
    )
    .await
    .unwrap();
    approve(
        &mut client,
        services,
        &w.signer(user, request.action),
        w.tenant,
        &ApproveInput {
            request_id: id,
            chain_pem: vec![signer.pem()],
            algorithm,
            payload_signature: signer.sign(request.canonical_payload.as_bytes()),
            document_signature: None,
            pdf_cms: None,
            revision: None,
        },
        at(1),
    )
    .await
}

/// The path from an import to opening voting: until the administrator sets
/// its number of signatures, an action that needs them can't be requested;
/// each number is set once; the board's signatures need the staff issuers
/// the administrator imports, and register the signers' certificates on
/// their first use.
#[tokio::test]
async fn an_imported_event_is_set_up_in_preparation_up_to_opening_voting() {
    let w = world("imported-post").await;
    let pki = Pki::get();
    assert_eq!(import(&w, &bundle(&w)).await.len(), 3);
    let operator = caller("operator", &[], &[]);
    for action in [
        SigningAction::ApproveConfiguration,
        SigningAction::OpenVoting,
    ] {
        match w
            .guard_request(&operator, &request(&w, action, subject(1)), at(0))
            .await
        {
            Err(SigningError::Invalid {
                reason: InvalidReason::Input,
                message,
            }) => assert_eq!(message, "a rule needs at least one signature", "{action}"),
            other => panic!("{action}: {other:?}"),
        }
    }

    // The administrator sets the certificate checks and, once each, the
    // numbers of signatures.
    let mut hasura = w.pool.get().await.unwrap();
    let htx = hasura.transaction().await.unwrap();
    let mut keycloak = w.pool.get().await.unwrap();
    let ktx = keycloak.transaction().await.unwrap();
    directory(&ktx, &w).await;
    assert!(update_signing_checks(
        &htx,
        w.tenant,
        w.event,
        &SigningChecks::default(),
        0,
        &actor("administrator"),
        None
    )
    .await
    .unwrap()
    .is_some());
    for action in SigningAction::iter().filter(SigningAction::always_required) {
        save(
            &htx,
            &ktx,
            &w,
            rule(action, SigningRequirement::Required, 2, 0),
        )
        .await
        .unwrap();
        assert_eq!(
            reason(
                save(
                    &htx,
                    &ktx,
                    &w,
                    rule(action, SigningRequirement::Required, 1, 1)
                )
                .await
            ),
            InvalidReason::OutOfRange,
            "{action}"
        );
    }
    // So does an action added to those that need signatures.
    let added = SigningAction::GenerateReports;
    save(
        &htx,
        &ktx,
        &w,
        rule(added, SigningRequirement::Required, 2, 0),
    )
    .await
    .unwrap();
    for (requirement, signatures) in [
        (SigningRequirement::Required, 1),
        (SigningRequirement::NotRequired, 2),
    ] {
        assert_eq!(
            reason(save(&htx, &ktx, &w, rule(added, requirement, signatures, 1)).await),
            InvalidReason::OutOfRange,
            "{requirement:?}"
        );
    }
    htx.commit().await.unwrap();
    ktx.rollback().await.unwrap();

    // The board approves the configuration once the administrator has
    // imported the staff issuers: until then no certificate is trusted.
    let approval = Arc::new(FakeExecutor::new(
        SigningAction::ApproveConfiguration,
        FakeBehaviour::Succeed(json!({})),
    ));
    let opening = Arc::new(FakeExecutor::new(
        SigningAction::OpenVoting,
        FakeBehaviour::Succeed(json!({})),
    ));
    let services = SigningServices {
        verifier: Arc::new(OpensslCertificateVerifier::with_directory(Arc::new(Names))),
        executors: SigningExecutorRegistry::default()
            .with(approval.clone())
            .with(opening.clone()),
        documents: Arc::new(NoDocumentSigner),
        exports: Arc::new(RowOnlyExports),
    };
    let configuration = start(
        &w,
        &operator,
        SigningAction::ApproveConfiguration,
        subject(2),
    )
    .await;
    assert_eq!(
        refused(sign(&w, &services, "maria", configuration, &pki.maria).await),
        CertificateCheckId::TrustedIssuer
    );
    trust(&w).await;
    for (user, certificate) in [("maria", &pki.maria), ("jose", &pki.jose)] {
        sign(&w, &services, user, configuration, certificate)
            .await
            .unwrap();
    }
    assert_eq!(
        w.request(configuration).await.status,
        SigningRequestStatus::Executed
    );
    assert_eq!(approval.runs(), [(configuration, 2)]);

    // Opening voting at the Post takes the same certificates, registered on
    // their first use.
    let voting = start(&w, &operator, SigningAction::OpenVoting, subject(3)).await;
    for (user, certificate) in [("maria", &pki.maria), ("jose", &pki.jose)] {
        sign(&w, &services, user, voting, certificate)
            .await
            .unwrap();
    }
    assert_eq!(opening.runs(), [(voting, 2)]);
    assert_eq!(
        w.steps()
            .await
            .iter()
            .filter(|kind| *kind == "SigningCertificateRegistered")
            .count(),
        2
    );
}
