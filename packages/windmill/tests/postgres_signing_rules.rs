// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Who can sign (the Keycloak database query), each Post's capacity and
//! saving a rule. The Keycloak tables the query reads are created, as
//! Keycloak names them, in a schema of the test's own transaction. Every
//! test runs under two label configurations.

#[path = "support/schema.rs"]
mod schema;
#[path = "support/signing.rs"]
mod signing;

use async_trait::async_trait;
use deadpool_postgres::Client;
use deadpool_postgres::Transaction;
use sequent_core::monitoring::revision::DashboardMode;
use sequent_core::signing::{
    CancelReason, RequesterSigning, SigningAction, SigningRequestStatus, SigningRequirement,
};
use sequent_core::types::ceremonies::CeremoniesPolicy;
use sequent_core::types::permissions::Permissions;
use serde_json::json;
use signing::*;
use std::collections::BTreeMap;
use std::sync::Mutex;
use uuid::Uuid;
use windmill::postgres::signing::*;
use windmill::postgres::signing_actions::count_published_configuration_versions;
use windmill::services::monitoring::config_store::{
    reset_to_preset, Author, EventRef, MonitoringConfigAudit, RecordedChange,
};
use windmill::services::signing::approve::NoDocumentSigner;
use windmill::services::signing::guard::{guard_at, GuardOutcome, GuardRequest};
use windmill::services::signing::requests::{event_info, get_panel};
use windmill::services::signing::rules::{
    capacity, commit_rule, list_rules, save_rule, RuleWarning, SaveRuleInput, SigningRoleAdmin,
};
use windmill::services::signing::signers::{list_signers, signer_titles, GroupChange};
use windmill::services::signing::{InvalidReason, SigningCaller, SigningError};

const ACTION: SigningAction = SigningAction::CloseVoting;
/// (the Post's label, another Post's label)
const LABELS: [(&str, &str); 2] = [
    ("madrid-pe", "tokyo-pe"),
    ("faculty-of-science", "faculty-of-law"),
];

fn realm(tenant: Uuid) -> String {
    format!("tenant-{tenant}")
}

async fn run(tx: &Transaction<'_>, sql: &str, params: &[&str]) {
    let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
        .iter()
        .map(|p| p as &(dyn tokio_postgres::types::ToSql + Sync))
        .collect();
    tx.execute(sql, &params).await.unwrap();
}

async fn user(tx: &Transaction<'_>, id: &str, realm: &str, enabled: bool) {
    let first = format!("{id}-first");
    tx.execute(
        "INSERT INTO user_entity VALUES ($1, $1, $2, 'Last', $3, $4, NULL)",
        &[&id, &first, &enabled, &realm],
    )
    .await
    .unwrap();
}

/// A realm where close-voting is held:
/// - maria: through the subgroup `sbei-post` of `sbei` (labelled `post`),
///   with a `title`;
/// - jose: a member of `sbei`, with the user label `post`;
/// - ana: mapped directly, with the label `other`;
/// - luis: through a composite role, no labels;
/// - off: in `sbei` but disabled; svc: a service account in `sbei`;
///   auditor: only in `auditors`, which has no sign permission; stranger:
///   holds it in another realm.
async fn directory(tx: &Transaction<'_>, realm: &str, post: &str, other: &str) {
    keycloak_tables(tx).await;
    let sign = ACTION.sign_permission().to_string();
    run(
        tx,
        "INSERT INTO realm VALUES ('r1', $1), ('r2', 'another-realm')",
        &[realm],
    )
    .await;
    run(
        tx,
        "INSERT INTO keycloak_role VALUES ('role', $1, 'r1'), ('bundle', 'bundle', 'r1'),
             ('other-role', $1, 'r2'), ('client-role', $1, 'r1')",
        &[&sign],
    )
    .await;
    run(
        tx,
        "UPDATE keycloak_role SET client_role = true WHERE id = 'client-role'",
        &[],
    )
    .await;
    run(
        tx,
        "INSERT INTO composite_role VALUES ('bundle', 'role')",
        &[],
    )
    .await;
    run(
        tx,
        "INSERT INTO keycloak_group (id, name, realm_id) VALUES ('g-sbei', 'sbei', 'r1'),
             ('g-auditors', 'auditors', 'r1')",
        &[],
    )
    .await;
    run(
        tx,
        "INSERT INTO keycloak_group VALUES ('g-sbei-post', 'sbei-post', 'g-sbei', 'r1')",
        &[],
    )
    .await;
    run(
        tx,
        "INSERT INTO group_role_mapping VALUES ('role', 'g-sbei')",
        &[],
    )
    .await;
    run(
        tx,
        "INSERT INTO group_attribute VALUES ('permission_labels', $1, 'g-sbei-post')",
        &[post],
    )
    .await;
    for (id, realm_id, enabled) in [
        ("maria", "r1", true),
        ("jose", "r1", true),
        ("ana", "r1", true),
        ("luis", "r1", true),
        ("off", "r1", false),
        ("auditor", "r1", true),
        ("stranger", "r2", true),
    ] {
        user(tx, id, realm_id, enabled).await;
    }
    run(
        tx,
        "INSERT INTO user_entity VALUES ('svc', 'service-account-x', NULL, NULL, true, 'r1', 'client-x')",
        &[],
    )
    .await;
    run(
        tx,
        "INSERT INTO user_group_membership VALUES ('g-sbei-post', 'maria'), ('g-sbei', 'jose'),
             ('g-sbei', 'off'), ('g-auditors', 'auditor'), ('g-sbei', 'svc')",
        &[],
    )
    .await;
    run(
        tx,
        "INSERT INTO user_role_mapping VALUES ('role', 'ana'), ('bundle', 'luis'),
             ('other-role', 'stranger'), ('client-role', 'auditor')",
        &[],
    )
    .await;
    run(
        tx,
        "INSERT INTO user_attribute VALUES ('title', 'Chairperson', 'maria'),
             ('permission_labels', $1, 'jose'), ('permission_labels', $2, 'ana')",
        &[post, other],
    )
    .await;
}

#[tokio::test]
async fn signers_hold_the_permission_directly_through_groups_or_composites() {
    for (post, other) in LABELS {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        directory(&tx, "tenant-x", post, other).await;
        let signers = list_signers(&tx, "tenant-x", ACTION, &GroupChange::default())
            .await
            .unwrap();
        let summary: Vec<(&str, Option<&str>, Vec<String>, Vec<String>)> = signers
            .iter()
            .map(|s| {
                (
                    s.username.as_str(),
                    s.title.as_deref(),
                    s.labels.clone(),
                    s.groups.clone(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            vec![
                ("ana", None, vec![other.to_string()], vec![]),
                (
                    "jose",
                    Some("sbei"),
                    vec![post.to_string()],
                    vec!["sbei".to_string()]
                ),
                ("luis", None, vec![], vec![]),
                (
                    "maria",
                    Some("Chairperson"),
                    vec![post.to_string()],
                    vec!["sbei".to_string()]
                ),
            ]
        );
        assert_eq!(signers[3].name().as_deref(), Some("maria-first Last"));
        let eligible: Vec<&str> = signers
            .iter()
            .filter(|s| s.eligible_for(Some(post)))
            .map(|s| s.username.as_str())
            .collect();
        // luis has no labels: like Hasura, he sees and signs for no
        // labelled Post. Everybody signs for an unlabelled one.
        assert_eq!(eligible, ["jose", "maria"]);
        assert_eq!(signers.iter().filter(|s| s.eligible_for(None)).count(), 4);

        // As they would be after a role change.
        let added = list_signers(
            &tx,
            "tenant-x",
            ACTION,
            &GroupChange {
                add: vec!["g-auditors".into()],
                remove: vec![],
            },
        )
        .await
        .unwrap();
        assert!(added.iter().any(|s| s.username == "auditor"));
        let removed = list_signers(
            &tx,
            "tenant-x",
            ACTION,
            &GroupChange {
                add: vec![],
                remove: vec!["g-sbei".into()],
            },
        )
        .await
        .unwrap();
        let names: Vec<&str> = removed.iter().map(|s| s.username.as_str()).collect();
        assert_eq!(names, ["ana", "luis"]);
        // Another action's permission is nobody's here.
        assert!(list_signers(
            &tx,
            "tenant-x",
            SigningAction::OpenVoting,
            &GroupChange::default()
        )
        .await
        .unwrap()
        .is_empty());
    }
}

#[tokio::test]
async fn a_signers_posts_are_the_labels_their_token_carries() {
    for (post, other) in LABELS {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        directory(&tx, "tenant-x", post, other).await;
        // pia is in two labelled groups: only the first by name counts.
        // olga has labels of her own: her groups' don't count. A subgroup
        // without labels takes its parent's.
        run(
            &tx,
            "INSERT INTO keycloak_group (id, name, realm_id) VALUES
                 ('g-a', 'a-signers', 'r1'), ('g-b', 'b-signers', 'r1')",
            &[],
        )
        .await;
        run(
            &tx,
            "INSERT INTO keycloak_group VALUES ('g-a-sub', 'a-sub', 'g-a', 'r1')",
            &[],
        )
        .await;
        run(
            &tx,
            "INSERT INTO group_role_mapping VALUES ('role', 'g-a'), ('role', 'g-b')",
            &[],
        )
        .await;
        run(
            &tx,
            "INSERT INTO group_attribute VALUES ('permission_labels', $1, 'g-b'),
                 ('permission_labels', $2, 'g-a')",
            &[post, other],
        )
        .await;
        for id in ["pia", "olga", "sub"] {
            user(&tx, id, "r1", true).await;
        }
        run(
            &tx,
            "INSERT INTO user_group_membership VALUES ('g-a', 'pia'), ('g-b', 'pia'),
                 ('g-a', 'olga'), ('g-a-sub', 'sub')",
            &[],
        )
        .await;
        run(
            &tx,
            "INSERT INTO user_attribute VALUES ('permission_labels', $1, 'olga')",
            &[post],
        )
        .await;
        let signers = list_signers(&tx, "tenant-x", ACTION, &GroupChange::default())
            .await
            .unwrap();
        let labels = |name: &str| {
            signers
                .iter()
                .find(|s| s.username == name)
                .unwrap()
                .labels
                .clone()
        };
        assert_eq!(labels("pia"), [other]);
        assert_eq!(labels("olga"), [post]);
        assert_eq!(labels("sub"), [other]);
        assert!(!signers
            .iter()
            .any(|s| s.username.starts_with("service-account")));
    }
}

/// A second Post with the other label, in the transaction.
async fn second_post(tx: &Transaction<'_>, w: &World, label: &str) -> Uuid {
    let id = Uuid::new_v4();
    tx.execute(
        "INSERT INTO sequent_backend.election (id, tenant_id, election_event_id, presentation, permission_label)
         VALUES ($1, $2, $3, $4, $5)",
        &[&id, &w.tenant, &w.event, &post_presentation(&format!("Post {label}")), &label],
    )
    .await
    .unwrap();
    id
}

async fn capacity_of(
    htx: &Transaction<'_>,
    ktx: &Transaction<'_>,
    w: &World,
    action: SigningAction,
    signatures: Option<u16>,
    requester: Option<RequesterSigning>,
) -> windmill::services::signing::rules::SigningCapacity {
    capacity(
        htx,
        ktx,
        w.tenant,
        w.event,
        action,
        &GroupChange::default(),
        signatures,
        requester,
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn capacity_counts_each_posts_signers() {
    for (post, other) in LABELS {
        let w = world(post).await;
        let mut hasura = w.pool.get().await.unwrap();
        let htx = hasura.transaction().await.unwrap();
        let other_post = second_post(&htx, &w, other).await;
        let mut keycloak = w.pool.get().await.unwrap();
        let ktx = keycloak.transaction().await.unwrap();
        directory(&ktx, &realm(w.tenant), post, other).await;
        // luis, without labels, signs for neither labelled Post.
        let capacity = capacity_of(
            &htx,
            &ktx,
            &w,
            ACTION,
            Some(2),
            Some(RequesterSigning::Allowed),
        )
        .await;
        let counts: Vec<(Uuid, usize)> = capacity
            .posts
            .iter()
            .map(|p| (p.election_id, p.count))
            .collect();
        let mut expected = vec![(w.post, 2), (other_post, 1)];
        expected.sort_by_key(|(id, _)| {
            if *id == w.post {
                format!("Post {post}")
            } else {
                format!("Post {other}")
            }
        });
        assert_eq!(counts, expected, "{post}");
        assert_eq!(capacity.max, 2);
        let roles: Vec<(&str, &str, &str)> = capacity
            .roles
            .iter()
            .map(|r| (r.id.as_str(), r.name.as_str(), r.path.as_str()))
            .collect();
        assert_eq!(roles, [("g-sbei", "sbei", "/sbei")]);
        assert_eq!((capacity.waiting, capacity.config_version), (0, 0));
        let short: Vec<Uuid> = capacity.posts_short.iter().map(|p| p.election_id).collect();
        assert_eq!(short, [other_post]);
        assert!(capacity.posts_short_without_requester.is_empty());
        // One signature with the requester kept out: the other Post's one
        // signer may be the requester.
        let without = capacity_of(
            &htx,
            &ktx,
            &w,
            ACTION,
            Some(1),
            Some(RequesterSigning::NotAllowed),
        )
        .await;
        assert!(without.posts_short.is_empty());
        let short: Vec<Uuid> = without
            .posts_short_without_requester
            .iter()
            .map(|p| p.election_id)
            .collect();
        assert_eq!(short, [other_post]);
        // Without a number, the saved rule's: no rule needs one signature.
        assert!(capacity_of(&htx, &ktx, &w, ACTION, None, None)
            .await
            .posts_short
            .is_empty());
        // Published event-level publications count as configuration versions,
        // soft-deleted ones too: the whole publish history.
        let previous_trusted: String = htx
            .query_one(
                "SELECT COALESCE(current_setting('sequent.trusted_write', true), '')",
                &[],
            )
            .await
            .unwrap()
            .get(0);
        htx.execute(
            "SELECT set_config('sequent.trusted_write', 'on', true)",
            &[],
        )
        .await
        .unwrap();
        for (election_id, published, deleted) in [
            (None, true, false),
            (None, true, false),
            (None, true, true),
            (None, false, false),
            (Some(w.post), true, false),
        ] {
            htx.execute(
                "INSERT INTO sequent_backend.ballot_publication
                     (id, tenant_id, election_event_id, is_generated, election_id, published_at,
                      deleted_at)
                 VALUES ($1, $2, $3, true, $4, CASE WHEN $5 THEN now() END,
                         CASE WHEN $6 THEN now() END)",
                &[
                    &Uuid::new_v4(),
                    &w.tenant,
                    &w.event,
                    &election_id,
                    &published,
                    &deleted,
                ],
            )
            .await
            .unwrap();
        }
        htx.execute(
            "SELECT set_config('sequent.trusted_write', $1, true)",
            &[&previous_trusted],
        )
        .await
        .unwrap();
        let version = capacity_of(&htx, &ktx, &w, ACTION, None, None)
            .await
            .config_version;
        assert_eq!(version, 3);
        // The same number an approve-configuration request carries.
        assert_eq!(
            version,
            count_published_configuration_versions(&htx, w.tenant, w.event)
                .await
                .unwrap()
        );
        // Event actions count every signer.
        let event = capacity_of(
            &htx,
            &ktx,
            &w,
            SigningAction::ApproveConfiguration,
            None,
            None,
        )
        .await;
        assert_eq!((event.max, event.posts.len()), (0, 0));
    }
}

/// Records the role changes it is asked for; fails on the group `fail_on`.
#[derive(Default)]
struct FakeRoles {
    calls: Mutex<Vec<String>>,
    fail_on: Option<String>,
}

#[async_trait]
impl SigningRoleAdmin for FakeRoles {
    async fn grant(&self, realm: &str, group_id: &str, permission: &str) -> anyhow::Result<()> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("grant {realm} {group_id} {permission}"));
        if self.fail_on.as_deref() == Some(group_id) {
            anyhow::bail!("Keycloak refused {group_id}");
        }
        Ok(())
    }

    async fn revoke(&self, realm: &str, group_id: &str, permission: &str) -> anyhow::Result<()> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("revoke {realm} {group_id} {permission}"));
        if self.fail_on.as_deref() == Some(group_id) {
            anyhow::bail!("Keycloak refused {group_id}");
        }
        Ok(())
    }
}

fn input(signatures: u16, expected_revision: i64) -> SaveRuleInput {
    SaveRuleInput {
        action: ACTION,
        requirement: SigningRequirement::Required,
        signatures,
        requester_signing: RequesterSigning::Allowed,
        expires_minutes: Some(60),
        roles: None,
        expected_revision,
    }
}

async fn steps_in(tx: &Transaction<'_>, event: Uuid) -> Vec<(String, String)> {
    tx.query(
        "SELECT statement_kind, event_type FROM sequent_backend.signing_log_outbox
         WHERE election_event_id = $1 ORDER BY id",
        &[&event],
    )
    .await
    .unwrap()
    .into_iter()
    .map(|row| (row.get(0), row.get(1)))
    .collect()
}

/// The details of the USER entries of `kind` in `event`, in order.
async fn details_in(tx: &Transaction<'_>, event: Uuid, kind: &str) -> Vec<serde_json::Value> {
    tx.query(
        "SELECT body->'details' FROM sequent_backend.signing_log_outbox
         WHERE election_event_id = $1 AND statement_kind = $2 AND entry = 0 ORDER BY id",
        &[&event, &kind],
    )
    .await
    .unwrap()
    .into_iter()
    .map(|row| row.get(0))
    .collect()
}

fn reason(result: Result<impl std::fmt::Debug, SigningError>) -> InvalidReason {
    match result {
        Err(SigningError::Invalid { reason, .. }) => reason,
        other => panic!("expected an invalid input, got {other:?}"),
    }
}

#[tokio::test]
async fn a_rule_save_is_checked_against_the_posts_and_cancels_waiting_requests() {
    for (post, other) in LABELS {
        let w = world(post).await;
        let mut hasura = w.pool.get().await.unwrap();
        let htx = hasura.transaction().await.unwrap();
        let other_post = second_post(&htx, &w, other).await;
        let mut keycloak = w.pool.get().await.unwrap();
        let ktx = keycloak.transaction().await.unwrap();
        directory(&ktx, &realm(w.tenant), post, other).await;
        let manager = caller("manager", &[Permissions::SIGNING_RULES_WRITE], &[]);

        // More than any Post can give; none; out of range; a zero expiry.
        let mut zero_expiry = input(1, 0);
        zero_expiry.expires_minutes = Some(0);
        let mut long_expiry = input(1, 0);
        long_expiry.expires_minutes = Some(600_000);
        for (input, expected) in [
            (input(3, 0), InvalidReason::OverCapacity),
            (input(0, 0), InvalidReason::NoSignatures),
            (input(101, 0), InvalidReason::OutOfRange),
            (zero_expiry, InvalidReason::ZeroExpiry),
            (long_expiry, InvalidReason::OutOfRange),
        ] {
            assert_eq!(
                reason(save_rule(&htx, &ktx, &manager, w.tenant, w.event, &input).await),
                expected
            );
        }

        let saved = save_rule(&htx, &ktx, &manager, w.tenant, w.event, &input(2, 0))
            .await
            .unwrap();
        assert_eq!(saved.outcome.revision, 1);
        assert_eq!(
            saved
                .outcome
                .short_posts
                .iter()
                .map(|p| p.election_id)
                .collect::<Vec<_>>(),
            [other_post]
        );
        assert_eq!(saved.outcome.warnings, [RuleWarning::ShortPosts]);
        assert!(saved.outcome.cancelled.is_empty() && saved.roles.is_empty());

        // Two waiting requests of the action.
        let starter = caller("operator", &[], &[]);
        let mut waiting = vec![];
        for key in ["a", "b"] {
            let mut scope = w.scope(ACTION);
            scope.subject_key = Some(key.into());
            let outcome = guard_at(
                &htx,
                &starter,
                &GuardRequest {
                    action: ACTION,
                    scope,
                    subject: subject(1),
                    document: None,
                    config_revision: None,
                },
                at(0),
            )
            .await
            .unwrap();
            let GuardOutcome::SigningRequired(summary) = outcome else {
                panic!("{outcome:?}")
            };
            waiting.push(summary.id);
        }
        let seen = capacity_of(&htx, &ktx, &w, ACTION, None, None).await;
        assert_eq!((seen.waiting, seen.posts_short.len()), (2, 1));

        // A save from a stale revision is refused and cancels nothing.
        assert!(matches!(
            save_rule(&htx, &ktx, &manager, w.tenant, w.event, &input(2, 0)).await,
            Err(SigningError::Conflict(_))
        ));
        let mut without_requester = input(1, 1);
        without_requester.requester_signing = RequesterSigning::NotAllowed;
        let saved = save_rule(&htx, &ktx, &manager, w.tenant, w.event, &without_requester)
            .await
            .unwrap();
        assert_eq!(saved.outcome.rule.signatures, 1);
        assert_eq!(saved.outcome.warnings, [RuleWarning::RequesterExcluded]);
        let mut cancelled = saved.outcome.cancelled.clone();
        cancelled.sort();
        waiting.sort();
        assert_eq!(cancelled, waiting);
        for id in &waiting {
            let row = get_signing_request(&htx, w.tenant, w.event, *id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(row.status, SigningRequestStatus::Cancelled);
            assert_eq!(row.cancel_reason, Some(CancelReason::RuleChanged));
        }
        let kinds: Vec<String> = steps_in(&htx, w.event)
            .await
            .into_iter()
            .filter(|(_, event_type)| event_type == "USER")
            .map(|(kind, _)| kind)
            .collect();
        assert_eq!(
            kinds,
            [
                "SigningRuleChanged",
                "SigningRequestCreated",
                "SigningRequestCreated",
                "SigningRequestCancelled",
                "SigningRequestCancelled",
                "SigningRuleChanged"
            ]
        );
        // Each change names the permission that allowed it.
        for details in details_in(&htx, w.event, "SigningRuleChanged").await {
            assert_eq!(details["allowed_by"], json!(["signing-rules-write"]));
        }
        let rules = list_rules(&htx, w.tenant, w.event).await.unwrap();
        assert_eq!(rules.len(), 10);
        let close = rules.iter().find(|r| r.rule.action == ACTION).unwrap();
        assert_eq!(
            (
                close.rule.signatures,
                close.updated_by.as_deref(),
                close.updated_by_name.as_deref()
            ),
            (1, Some("manager"), Some("manager display"))
        );
        let open = rules
            .iter()
            .find(|r| r.rule.action == SigningAction::OpenVoting)
            .unwrap();
        assert_eq!(open.rule.requirement, SigningRequirement::NotRequired);
    }
}

#[tokio::test]
async fn a_required_rule_saves_with_a_warning_before_the_event_has_posts() {
    let w = world("madrid-pe").await;
    let mut hasura = w.pool.get().await.unwrap();
    let htx = hasura.transaction().await.unwrap();
    htx.execute(
        "DELETE FROM sequent_backend.election WHERE id = $1",
        &[&w.post],
    )
    .await
    .unwrap();
    let mut keycloak = w.pool.get().await.unwrap();
    let ktx = keycloak.transaction().await.unwrap();
    keycloak_tables(&ktx).await;
    let manager = caller("manager", &[Permissions::SIGNING_RULES_WRITE], &[]);
    let saved = save_rule(&htx, &ktx, &manager, w.tenant, w.event, &input(3, 0))
        .await
        .unwrap();
    assert_eq!(saved.outcome.warnings, [RuleWarning::NoPosts]);
}

#[tokio::test]
async fn a_locked_down_event_refuses_rule_changes() {
    for (post, other) in LABELS {
        let w = world(post).await;
        let mut hasura = w.pool.get().await.unwrap();
        let htx = hasura.transaction().await.unwrap();
        // As the lockdown task does.
        htx.execute(
            "SELECT set_config('sequent.trusted_write', 'on', true)",
            &[],
        )
        .await
        .unwrap();
        htx.execute(
            "UPDATE sequent_backend.election_event SET presentation = '{\"locked_down\": \"locked-down\"}'
             WHERE id = $1",
            &[&w.event],
        )
        .await
        .unwrap();
        let mut keycloak = w.pool.get().await.unwrap();
        let ktx = keycloak.transaction().await.unwrap();
        directory(&ktx, &realm(w.tenant), post, other).await;
        let manager = caller("manager", &[Permissions::SIGNING_RULES_WRITE], &[]);
        assert_eq!(
            reason(save_rule(&htx, &ktx, &manager, w.tenant, w.event, &input(1, 0)).await),
            InvalidReason::LockedDown
        );
        assert!(get_signing_rule(&htx, w.tenant, w.event, ACTION)
            .await
            .unwrap()
            .is_none());
    }
}

/// A second event of the tenant with a rule for the action, committed.
async fn event_with_rule(w: &World) -> Uuid {
    let other_event = Uuid::new_v4();
    w.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&other_event, &w.tenant],
    )
    .await;
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    upsert_signing_rule(
        &tx,
        w.tenant,
        other_event,
        &sequent_core::signing::SigningRule::default_for(ACTION),
        0,
        "someone",
        None,
    )
    .await
    .unwrap()
    .unwrap();
    tx.commit().await.unwrap();
    other_event
}

fn admin() -> windmill::services::signing::SigningCaller {
    caller(
        "admin",
        &[
            Permissions::SIGNING_RULES_WRITE,
            Permissions::ROLE_READ,
            Permissions::ROLE_WRITE,
        ],
        &[],
    )
}

#[tokio::test]
async fn changing_who_signs_is_checked_logged_everywhere_and_sent_to_keycloak_last() {
    for (post, other) in LABELS {
        let w = world(post).await;
        let other_event = event_with_rule(&w).await;
        let mut keycloak = w.pool.get().await.unwrap();
        let ktx = keycloak.transaction().await.unwrap();
        directory(&ktx, &realm(w.tenant), post, other).await;
        // A group granting the permission only through a composite role.
        run(
            &ktx,
            "INSERT INTO keycloak_group (id, name, realm_id) VALUES ('g-bundled', 'bundled', 'r1')",
            &[],
        )
        .await;
        run(
            &ktx,
            "INSERT INTO group_role_mapping VALUES ('bundle', 'g-bundled')",
            &[],
        )
        .await;
        // The auditor signs for the Post once the auditors can sign.
        run(
            &ktx,
            "INSERT INTO user_attribute VALUES ('permission_labels', $1, 'auditor')",
            &[post],
        )
        .await;
        let mut change = input(3, 0);
        change.roles = Some(GroupChange {
            add: vec!["g-auditors".into()],
            remove: vec![],
        });
        let mut hasura = w.pool.get().await.unwrap();
        let htx = hasura.transaction().await.unwrap();
        let manager = caller(
            "manager",
            &[Permissions::SIGNING_RULES_WRITE, Permissions::ROLE_WRITE],
            &[],
        );
        assert!(matches!(
            save_rule(&htx, &ktx, &manager, w.tenant, w.event, &change).await,
            Err(SigningError::Forbidden(_))
        ));
        for (add, remove, expected) in [
            (vec!["nobody"], vec![], InvalidReason::UnknownGroup),
            (vec![], vec!["g-bundled"], InvalidReason::Composite),
        ] {
            let mut bad = change.clone();
            bad.roles = Some(GroupChange {
                add: add.into_iter().map(str::to_owned).collect(),
                remove: remove.into_iter().map(str::to_owned).collect(),
            });
            assert_eq!(
                reason(save_rule(&htx, &ktx, &admin(), w.tenant, w.event, &bad).await),
                expected
            );
        }

        // With role-read and role-write, 3 fits once the auditors can sign.
        // Nothing reaches Keycloak until the commit.
        let roles = FakeRoles::default();
        let saved = save_rule(&htx, &ktx, &admin(), w.tenant, w.event, &change)
            .await
            .unwrap();
        assert!(roles.calls.lock().unwrap().is_empty());
        assert_eq!(saved.roles.add, ["g-auditors"]);
        commit_rule(htx, &roles, &saved.roles).await.unwrap();
        assert_eq!(
            *roles.calls.lock().unwrap(),
            [format!(
                "grant {} g-auditors sign-close-voting",
                realm(w.tenant)
            )]
        );
        let client = w.pool.get().await.unwrap();
        for event in [w.event, other_event] {
            let count: i64 = client
                .query_one(
                    "SELECT count(*) FROM sequent_backend.signing_log_outbox
                     WHERE election_event_id = $1 AND statement_kind = 'SigningPermissionChanged'",
                    &[&event],
                )
                .await
                .unwrap()
                .get(0);
            assert_eq!(count, 2, "{event}");
        }
        let description: String = client
            .query_one(
                "SELECT body->>'description' FROM sequent_backend.signing_log_outbox
                 WHERE election_event_id = $1 AND statement_kind = 'SigningPermissionChanged' LIMIT 1",
                &[&other_event],
            )
            .await
            .unwrap()
            .get(0);
        assert_eq!(
            description,
            "Changed who can sign close voting: added auditors; removed none"
        );
        let htx = hasura.transaction().await.unwrap();
        for event in [w.event, other_event] {
            let details = details_in(&htx, event, "SigningPermissionChanged").await;
            assert_eq!(details.len(), 1, "{event}");
            assert_eq!(details[0]["allowed_by"], json!(["role-write"]));
        }
        assert_eq!(
            details_in(&htx, w.event, "SigningRuleChanged").await[0]["allowed_by"],
            json!(["signing-rules-write"])
        );
    }
}

#[tokio::test]
async fn a_keycloak_failure_puts_back_what_was_sent_and_saves_nothing() {
    let w = world("madrid-pe").await;
    let mut keycloak = w.pool.get().await.unwrap();
    let ktx = keycloak.transaction().await.unwrap();
    directory(&ktx, &realm(w.tenant), "madrid-pe", "tokyo-pe").await;
    // The auditor signs for the Post once the auditors can sign.
    run(
        &ktx,
        "INSERT INTO user_attribute VALUES ('permission_labels', $1, 'auditor')",
        &["madrid-pe"],
    )
    .await;
    let mut hasura = w.pool.get().await.unwrap();
    let htx = hasura.transaction().await.unwrap();
    let mut change = input(1, 0);
    change.roles = Some(GroupChange {
        add: vec!["g-auditors".into()],
        remove: vec!["g-sbei".into()],
    });
    let saved = save_rule(&htx, &ktx, &admin(), w.tenant, w.event, &change)
        .await
        .unwrap();
    let roles = FakeRoles {
        fail_on: Some("g-sbei".into()),
        ..Default::default()
    };
    assert!(matches!(
        commit_rule(htx, &roles, &saved.roles).await,
        Err(SigningError::Internal(_))
    ));
    let realm = realm(w.tenant);
    assert_eq!(
        *roles.calls.lock().unwrap(),
        [
            format!("grant {realm} g-auditors sign-close-voting"),
            format!("revoke {realm} g-sbei sign-close-voting"),
            format!("grant {realm} g-sbei sign-close-voting"),
            format!("revoke {realm} g-auditors sign-close-voting"),
        ]
    );
    let mut client = w.pool.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    assert!(get_signing_rule(&tx, w.tenant, w.event, ACTION)
        .await
        .unwrap()
        .is_none());
    assert!(steps_in(&tx, w.event).await.is_empty());
}

#[tokio::test]
async fn a_trustee_rule_needs_manual_ceremonies() {
    for (post, other) in LABELS {
        let w = world(post).await;
        let mut hasura = w.pool.get().await.unwrap();
        let htx = hasura.transaction().await.unwrap();
        let mut keycloak = w.pool.get().await.unwrap();
        let ktx = keycloak.transaction().await.unwrap();
        directory(&ktx, &realm(w.tenant), post, other).await;
        let manager = caller("manager", &[Permissions::SIGNING_RULES_WRITE], &[]);
        let policy = |policy: CeremoniesPolicy| json!({ "ceremonies_policy": policy.to_string() });
        htx.execute(
            "UPDATE sequent_backend.election_event SET presentation = $1 WHERE id = $2",
            &[&policy(CeremoniesPolicy::AUTOMATED_CEREMONIES), &w.event],
        )
        .await
        .unwrap();
        for action in [
            SigningAction::ConfirmKeyShare,
            SigningAction::ContributeKeyShare,
        ] {
            let mut required = input(1, 0);
            required.action = action;
            // No trustee takes the step to sign it: the rule would be bypassed.
            assert_eq!(
                reason(save_rule(&htx, &ktx, &manager, w.tenant, w.event, &required).await),
                InvalidReason::AutomatedCeremonies,
                "{post}: {action}"
            );
            let mut off = required.clone();
            off.requirement = SigningRequirement::NotRequired;
            save_rule(&htx, &ktx, &manager, w.tenant, w.event, &off)
                .await
                .unwrap();
        }
        // Other actions don't depend on the ceremonies.
        let mut close = input(1, 0);
        close.action = ACTION;
        save_rule(&htx, &ktx, &manager, w.tenant, w.event, &close)
            .await
            .unwrap();

        htx.execute(
            "UPDATE sequent_backend.election_event SET presentation = $1 WHERE id = $2",
            &[&policy(CeremoniesPolicy::MANUAL_CEREMONIES), &w.event],
        )
        .await
        .unwrap();
        let mut required = input(1, 1);
        required.action = SigningAction::ConfirmKeyShare;
        save_rule(&htx, &ktx, &manager, w.tenant, w.event, &required)
            .await
            .unwrap();
    }
}

/// The Certificates tab names each signer as the panel does: their `title`
/// attribute, else the group that grants them a sign permission. Someone
/// who signs nothing, or signs through a direct or composite role, has
/// none.
#[tokio::test]
async fn each_signer_has_the_panels_title() {
    for (post, other) in LABELS {
        let pool = schema::pool().await;
        let mut client = pool.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        directory(&tx, "tenant-x", post, other).await;
        let titles = signer_titles(&tx, "tenant-x").await.unwrap();
        assert_eq!(
            titles,
            BTreeMap::from([
                ("jose".to_string(), "sbei".to_string()),
                ("maria".to_string(), "Chairperson".to_string()),
            ])
        );
        // Another realm's signers are not this tenant's.
        assert!(signer_titles(&tx, "another-realm")
            .await
            .unwrap()
            .is_empty());
    }
}

/// A monitoring configuration log that keeps nothing.
struct NoAudit;

#[async_trait]
impl MonitoringConfigAudit for NoAudit {
    async fn prepare(&self, _: &mut Client, _: EventRef, _: &Author) -> anyhow::Result<()> {
        Ok(())
    }

    async fn record(&self, _: &Transaction<'_>, _: &RecordedChange) -> anyhow::Result<()> {
        Ok(())
    }
}

/// What the Signatures tab and a signer's list show beside the Hasura rows:
/// the event's primary time zone (as the panel and signed PDF use) for
/// anyone who reads a part of the tab or signs, and the
/// signers' titles for who reads the certificates.
#[tokio::test]
async fn the_event_info_names_its_zone_and_titles_to_who_may_read_them() {
    // Monitoring presets do not determine the primary timezone.
    for ((preset, zone), (post, other)) in [("comelec", "Asia/Manila"), ("campus", "Europe/Madrid")]
        .into_iter()
        .zip(LABELS)
    {
        let w = world(post).await;
        let mut keycloak = w.pool.get().await.unwrap();
        let ktx = keycloak.transaction().await.unwrap();
        directory(&ktx, &realm(w.tenant), post, other).await;
        let info = |who: SigningCaller| {
            let (w, ktx) = (w.clone(), &ktx);
            async move {
                let mut hasura = w.pool.get().await.unwrap();
                let htx = hasura.transaction().await.unwrap();
                event_info(&htx, ktx, &who, w.tenant, w.event).await
            }
        };
        let certificates = caller("officer", &[Permissions::SIGNING_CERTIFICATES_READ], &[]);
        // A request's panel shows its times in the same zone.
        w.rule(ACTION, 2, RequesterSigning::Allowed, None).await;
        let reader = caller("ofov", &[Permissions::SIGNING_REQUESTS_READ], &[post]);
        let request = w.start(&reader, ACTION, subject(1), at(0)).await;
        let panel_zone = || {
            let (w, ktx, reader) = (w.clone(), &ktx, reader.clone());
            async move {
                let mut hasura = w.pool.get().await.unwrap();
                let htx = hasura.transaction().await.unwrap();
                get_panel(&htx, ktx, &NoDocumentSigner, &reader, w.tenant, request.id)
                    .await
                    .unwrap()
                    .time_zone
            }
        };

        // An event without configured timezones explicitly uses UTC.
        let unset = info(certificates.clone()).await.unwrap();
        assert_eq!(unset.time_zone.as_deref(), Some("UTC"));
        assert_eq!(panel_zone().await, None);

        let mut client = w.pool.get().await.unwrap();
        reset_to_preset(
            &mut client,
            &NoAudit,
            EventRef {
                tenant_id: w.tenant,
                election_event_id: w.event,
            },
            &Author {
                id: "admin".into(),
                name: None,
            },
            preset,
            DashboardMode::Configured,
        )
        .await
        .unwrap();

        // Configure the event primary separately from its monitoring preset.
        client
            .execute(
                "UPDATE sequent_backend.election_event SET presentation =
                 jsonb_set(COALESCE(presentation, '{}'::jsonb), '{timezones}', $2)
                 WHERE id = $1",
                &[
                    &w.event,
                    &json!({"configured": [zone], "primary": zone, "logs": "primary"}),
                ],
            )
            .await
            .unwrap();

        let read = info(certificates).await.unwrap();
        assert_eq!(read.time_zone.as_deref(), Some(zone));
        assert_eq!(panel_zone().await, None);
        assert_eq!(
            read.titles.get("maria").map(String::as_str),
            Some("Chairperson")
        );
        for who in [
            caller("ofov", &[Permissions::SIGNING_REQUESTS_READ], &[]),
            caller("manager", &[Permissions::SIGNING_RULES_READ], &[]),
            caller("sbei", &[Permissions::SIGN_CLOSE_VOTING], &[post]),
            caller("trustee", &[Permissions::SIGN_KEY_CEREMONY], &[]),
        ] {
            let read = info(who.clone()).await.unwrap();
            assert_eq!(read.time_zone.as_deref(), Some(zone), "{}", who.user_id);
            assert!(read.titles.is_empty(), "{}", who.user_id);
        }
        let outsider = caller("outsider", &[Permissions::ADMIN_USER], &[]);
        assert!(matches!(
            info(outsider).await,
            Err(SigningError::Forbidden(_))
        ));
    }
}
