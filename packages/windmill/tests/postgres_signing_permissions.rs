// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A change to who can sign an action, made in Users and Roles, is logged
//! in every election event of the tenant that has a rule for the action,
//! and nowhere else.

#[path = "support/schema.rs"]
mod schema;

use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::SigningStatementKind;
use electoral_log::messages::statement::{StatementEventType, StatementLogType};
use sequent_core::signing::{SigningAction, SigningRule};
use sequent_core::types::permissions::Permissions;
use serde_json::json;
use strum::IntoEnumIterator;
use uuid::Uuid;
use windmill::postgres::signing::{
    fetch_unposted_signing_log_outbox, list_events_with_signing_rule, upsert_signing_rule,
};
use windmill::services::signing::log::Actor;
use windmill::services::signing::permissions::{
    lock_permission_change_events, signing_action_of, signing_actions_in, stage_permission_change,
    stage_permission_changes, RolePermissionChange, SigningPermissionChange,
};

/// A fixed v4 UUID per test (`seed`) and call (`n`).
fn id(seed: u32, n: u32) -> Uuid {
    Uuid::parse_str(&format!("{seed:08x}-0000-4000-8000-{n:012x}")).unwrap()
}

async fn tenant(tx: &Transaction<'_>, tenant: Uuid) {
    tx.execute(
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
        &[&tenant, &format!("tenant-{tenant}")],
    )
    .await
    .unwrap();
}

/// An election event with saved rules for `actions`.
async fn event(tx: &Transaction<'_>, tenant: Uuid, event: Uuid, actions: &[SigningAction]) {
    tx.execute(
        "INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&event, &tenant],
    )
    .await
    .unwrap();
    for action in actions {
        upsert_signing_rule(
            tx,
            tenant,
            event,
            &SigningRule::default_for(*action),
            0,
            "configuration-manager-1",
            None,
        )
        .await
        .unwrap()
        .unwrap();
    }
}

#[test]
fn only_the_sign_permissions_name_an_action() {
    for action in SigningAction::iter() {
        assert_eq!(
            signing_action_of(&action.sign_permission().to_string()),
            Some(action)
        );
    }
    for other in [
        Permissions::SIGNING_RULES_WRITE.to_string(),
        Permissions::ELECTION_EVENT_SIGNATURES_TAB.to_string(),
        Permissions::ROLE_WRITE.to_string(),
        Permissions::MIRU_SIGN.to_string(),
        "sign-".to_string(),
        "close-voting".to_string(),
    ] {
        assert_eq!(signing_action_of(&other), None, "{other}");
    }
}

#[test]
fn a_role_s_permissions_name_each_action_once() {
    assert_eq!(
        signing_actions_in([
            "role-read",
            "sign-close-voting",
            "sign-open-voting",
            "sign-close-voting",
            "signing-rules-write",
        ]),
        vec![SigningAction::CloseVoting, SigningAction::OpenVoting]
    );
    assert_eq!(signing_actions_in(["role-read"]), vec![]);
}

/// Two configurations: a different action, role and change each, so a
/// description or event list that ignored its input could not pass both.
#[tokio::test]
async fn a_permission_change_is_logged_in_each_event_with_a_rule_for_the_action() {
    let cases = [
        (
            1,
            SigningAction::CloseVoting,
            SigningAction::ApproveVoter,
            ("group-sbei", "sbei"),
            RolePermissionChange::Added,
            "Role sbei updated: sign-close-voting added",
        ),
        (
            2,
            SigningAction::ApproveConfiguration,
            SigningAction::TransmitResults,
            ("group-so", "Security Officer"),
            RolePermissionChange::Removed,
            "Role Security Officer updated: sign-approve-configuration removed",
        ),
    ];
    for (seed, action, other, (role_id, role_name), change, description) in cases {
        let mut client = schema::pool().await.get().await.unwrap();
        let tx = client.transaction().await.unwrap();
        let (this_tenant, other_tenant) = (id(seed, 1), id(seed, 2));
        tenant(&tx, this_tenant).await;
        tenant(&tx, other_tenant).await;
        // Events 3 and 5 have a rule for the action; 4 only for another
        // action; 6 is another tenant's.
        event(&tx, this_tenant, id(seed, 5), &[action, other]).await;
        event(&tx, this_tenant, id(seed, 4), &[other]).await;
        event(&tx, this_tenant, id(seed, 3), &[action]).await;
        event(&tx, other_tenant, id(seed, 6), &[action]).await;

        assert_eq!(
            list_events_with_signing_rule(&tx, this_tenant, action)
                .await
                .unwrap(),
            vec![id(seed, 3), id(seed, 5)]
        );
        let editor = Actor {
            user_id: format!("editor-{seed}"),
            username: format!("editor.{seed}"),
        };
        let staged = stage_permission_change(
            &tx,
            &SigningPermissionChange {
                tenant_id: this_tenant,
                action,
                role_id: role_id.to_string(),
                role_name: role_name.to_string(),
                change,
                editor: editor.clone(),
                allowed_by: vec![Permissions::USER_PERMISSION_WRITE, Permissions::ROLE_WRITE],
            },
        )
        .await
        .unwrap();
        assert_eq!(staged, vec![id(seed, 3), id(seed, 5)]);

        let permission = action.sign_permission().to_string();
        for event_id in [id(seed, 3), id(seed, 5)] {
            let rows = fetch_unposted_signing_log_outbox(&tx, this_tenant, event_id, 10)
                .await
                .unwrap();
            assert_eq!(rows.len(), 2, "{event_id}");
            let (user, system) = (&rows[0], &rows[1]);
            assert_eq!(user.step_id, system.step_id);
            for row in &rows {
                assert_eq!(
                    row.statement_kind,
                    SigningStatementKind::SigningPermissionChanged
                );
                assert_eq!(row.log_type, StatementLogType::INFO);
                assert_eq!((row.election_id, row.area_id), (None, None));
                assert_eq!(row.body["description"], description);
                assert_eq!(
                    row.body["details"],
                    json!({
                        "action": action.to_string(),
                        "permission": permission,
                        "role_id": role_id,
                        "role": role_name,
                        "change": change.to_string(),
                        "allowed_by": [
                            Permissions::USER_PERMISSION_WRITE.to_string(),
                            Permissions::ROLE_WRITE.to_string(),
                        ],
                    })
                );
            }
            assert_eq!(user.event_type, StatementEventType::USER);
            assert_eq!(user.user_id.as_deref(), Some(editor.user_id.as_str()));
            assert_eq!(user.username.as_deref(), Some(editor.username.as_str()));
            assert_eq!(system.event_type, StatementEventType::SYSTEM);
            assert_eq!(
                (system.user_id.clone(), system.username.clone()),
                (None, None)
            );
        }
        for (tenant_id, event_id) in [(this_tenant, id(seed, 4)), (other_tenant, id(seed, 6))] {
            assert!(
                fetch_unposted_signing_log_outbox(&tx, tenant_id, event_id, 10)
                    .await
                    .unwrap()
                    .is_empty(),
                "{event_id}"
            );
        }
        tx.rollback().await.unwrap();
    }
}

/// A tenant without rules for the action logs nothing.
#[tokio::test]
async fn a_permission_change_without_rules_logs_nothing() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    tenant(&tx, id(3, 1)).await;
    event(&tx, id(3, 1), id(3, 2), &[SigningAction::OpenVoting]).await;
    let staged = stage_permission_change(
        &tx,
        &SigningPermissionChange {
            tenant_id: id(3, 1),
            action: SigningAction::ContributeKeyShare,
            role_id: "group-trustee".into(),
            role_name: "trustee".into(),
            change: RolePermissionChange::Added,
            editor: Actor {
                user_id: "editor".into(),
                username: "editor".into(),
            },
            allowed_by: vec![Permissions::ROLE_WRITE],
        },
    )
    .await
    .unwrap();
    assert!(staged.is_empty());
    assert!(
        fetch_unposted_signing_log_outbox(&tx, id(3, 1), id(3, 2), 10)
            .await
            .unwrap()
            .is_empty()
    );
}

/// A role created with, or deleted holding, several sign permissions logs
/// each one in the events with a rule for it; an event with rules for both
/// gets both steps, in the order given.
#[tokio::test]
async fn several_changes_are_logged_per_action_in_the_events_with_its_rule() {
    let mut client = schema::pool().await.get().await.unwrap();
    let tx = client.transaction().await.unwrap();
    let (tenant_id, both, one) = (id(4, 1), id(4, 2), id(4, 3));
    tenant(&tx, tenant_id).await;
    let (first, second) = (SigningAction::OpenVoting, SigningAction::CloseVoting);
    event(&tx, tenant_id, both, &[first, second]).await;
    event(&tx, tenant_id, one, &[second]).await;
    let change = |action| SigningPermissionChange {
        tenant_id,
        action,
        role_id: "group-new".into(),
        role_name: "Post signers".into(),
        change: RolePermissionChange::Added,
        editor: Actor {
            user_id: "creator".into(),
            username: "creator".into(),
        },
        allowed_by: vec![Permissions::ROLE_CREATE],
    };
    let locked = lock_permission_change_events(&tx, tenant_id, &[first, second])
        .await
        .unwrap();
    assert_eq!(locked.events(), vec![both, one]);
    stage_permission_changes(&tx, &locked, &[change(first), change(second)])
        .await
        .unwrap();
    // A change whose action's events weren't locked is refused.
    let unlocked = SigningPermissionChange {
        action: SigningAction::TransmitResults,
        ..change(first)
    };
    assert!(stage_permission_changes(&tx, &locked, &[unlocked])
        .await
        .is_err());
    let descriptions = |rows: Vec<windmill::postgres::signing::SigningLogOutboxRow>| {
        rows.iter()
            .map(|row| {
                assert_eq!(row.body["details"]["allowed_by"], json!(["role-create"]));
                row.body["description"].as_str().unwrap().to_string()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        descriptions(
            fetch_unposted_signing_log_outbox(&tx, tenant_id, both, 10)
                .await
                .unwrap()
        ),
        [
            "Role Post signers updated: sign-open-voting added",
            "Role Post signers updated: sign-open-voting added",
            "Role Post signers updated: sign-close-voting added",
            "Role Post signers updated: sign-close-voting added",
        ]
    );
    assert_eq!(
        descriptions(
            fetch_unposted_signing_log_outbox(&tx, tenant_id, one, 10)
                .await
                .unwrap()
        ),
        [
            "Role Post signers updated: sign-close-voting added",
            "Role Post signers updated: sign-close-voting added",
        ]
    );
}
