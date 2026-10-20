// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! What the Hasura metadata lets each role read of messaging, checked on the
//! metadata files without a running Hasura (BC-F9, AP-F10): no role reads an
//! account's webhook key, `admin-user` reads nothing of messaging, every
//! read is scoped to the caller's tenant, and the event configuration can
//! only be written through its action.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const MESSAGING_TABLES: [&str; 4] = [
    "messaging_account",
    "message",
    "message_latest_attempt",
    "messaging_event_config",
];

fn metadata() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../hasura/metadata")
}

fn yaml(path: PathBuf) -> Value {
    let text = std::fs::read_to_string(&path).unwrap();
    let parsed: serde_yaml::Value = serde_yaml::from_str(&text)
        .unwrap_or_else(|why| panic!("{}: {why}", path.display()));
    serde_json::to_value(parsed).unwrap()
}

/// The tracked messaging tables' metadata, by table name.
fn tables() -> BTreeMap<String, Value> {
    tracked(&MESSAGING_TABLES)
}

/// The metadata of the tracked tables named, by table name.
fn tracked(names: &[&str]) -> BTreeMap<String, Value> {
    let directory = metadata().join("databases/backend-db/tables");
    let list: Vec<String> = serde_yaml::from_str(
        &std::fs::read_to_string(directory.join("tables.yaml")).unwrap(),
    )
    .unwrap();
    list.iter()
        .filter_map(|include| include.strip_prefix("!include "))
        .map(|file| yaml(directory.join(file)))
        .filter_map(|table| {
            let name = table["table"]["name"].as_str()?.to_string();
            names.contains(&name.as_str()).then_some((name, table))
        })
        .collect()
}

fn grants(table: &Value, kind: &str) -> BTreeMap<String, Value> {
    table[kind]
        .as_array()
        .into_iter()
        .flatten()
        .map(|grant| {
            (
                grant["role"].as_str().unwrap().to_string(),
                grant["permission"].clone(),
            )
        })
        .collect()
}

fn columns(permission: &Value) -> BTreeSet<String> {
    permission["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|column| column.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn privacy_participation_grants_are_never_public_hasura_columns() {
    let tables = tracked(&["scheduled_event", "tasks_execution"]);
    assert_eq!(tables.len(), 2);
    for (name, table) in tables {
        for kind in [
            "select_permissions",
            "insert_permissions",
            "update_permissions",
        ] {
            for (role, permission) in grants(&table, kind) {
                assert!(!columns(&permission).contains("participation_authorization"),
                    "server authorization must not be available through {name}.{kind} as {role}");
            }
        }
    }
}

#[test]
fn messaging_tables_are_tracked() {
    assert_eq!(
        tables().keys().map(String::as_str).collect::<BTreeSet<_>>(),
        BTreeSet::from(MESSAGING_TABLES)
    );
}

#[test]
fn no_role_reads_a_webhook_key_and_admin_user_reads_nothing() {
    for (name, table) in tables() {
        let selects = grants(&table, "select_permissions");
        assert!(!selects.contains_key("admin-user"), "{name}");
        for (role, permission) in &selects {
            let columns = columns(permission);
            assert!(!columns.contains("webhook_key"), "{name} as {role}");
            assert_eq!(
                permission["filter"],
                serde_json::json!({"tenant_id": {"_eq": "X-Hasura-Tenant-Id"}}),
                "{name} as {role}: every read is the caller's tenant"
            );
        }
    }
}

#[test]
fn messaging_config_read_reads_states_but_no_secrets_or_provider_ids() {
    let tables = tables();
    let role = "messaging-config-read";
    for name in MESSAGING_TABLES {
        let selects = grants(&tables[name], "select_permissions");
        let columns =
            columns(selects.get(role).unwrap_or_else(|| panic!("{name}")));
        for hidden in [
            "credentials",
            "webhook_key",
            "billing",
            "provider_message_id",
            "destination_digest",
        ] {
            assert!(!columns.contains(hidden), "{name}.{hidden}");
        }
    }
    // The attempts of one message are grouped by their logical key (an
    // id, never content), so the drawer shows the latest outcome.
    let message =
        columns(&grants(&tables["message"], "select_permissions")[role]);
    assert!(message.contains("logical_key"));
    let account = columns(
        &grants(&tables["messaging_account"], "select_permissions")
            ["messaging-account-read"],
    );
    assert!(account.contains("credentials"));
}

#[test]
fn the_event_configuration_is_written_only_through_its_action() {
    let tables = tables();
    for name in MESSAGING_TABLES {
        for kind in [
            "insert_permissions",
            "update_permissions",
            "delete_permissions",
        ] {
            assert!(grants(&tables[name], kind).is_empty(), "{name} {kind}");
        }
    }
}

#[test]
fn messaging_actions_need_their_permission() {
    let actions = yaml(metadata().join("actions.yaml"));
    let roles: BTreeMap<String, BTreeSet<String>> = actions["actions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|action| {
            (
                action["name"].as_str().unwrap().to_string(),
                action["permissions"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|p| p["role"].as_str().map(str::to_string))
                    .collect(),
            )
        })
        .collect();
    for (action, role) in [
        ("upsert_messaging_account", "messaging-account-write"),
        ("delete_messaging_account", "messaging-account-write"),
        (
            "replace_messaging_account_credentials",
            "messaging-account-write",
        ),
        ("check_messaging_account", "messaging-account-write"),
        ("test_messaging_account", "messaging-account-write"),
        ("messaging_account_callback_url", "messaging-account-write"),
        ("update_event_messaging_config", "messaging-config-write"),
        ("messaging_event_stats", "messaging-config-read"),
    ] {
        assert_eq!(
            roles.get(action),
            Some(&BTreeSet::from([role.to_string()])),
            "{action}"
        );
    }
    // What stops callbacks: for whoever reads the event tab or the
    // accounts; no URL or key in it.
    assert_eq!(
        roles.get("messaging_callback_status"),
        Some(&BTreeSet::from([
            "messaging-config-read".to_string(),
            "messaging-account-read".to_string(),
        ]))
    );
}

/// Follow-up to R4-07: the drawer and Logs read each message's latest
/// attempt from a view, with exactly the ledger's safe columns.
#[test]
fn the_latest_attempt_view_reads_as_the_ledger_does() {
    let tables = tables();
    let view = grants(&tables["message_latest_attempt"], "select_permissions");
    assert_eq!(
        view.keys().map(String::as_str).collect::<Vec<_>>(),
        vec!["messaging-config-read"]
    );
    assert_eq!(
        columns(&view["messaging-config-read"]),
        columns(
            &grants(&tables["message"], "select_permissions")
                ["messaging-config-read"]
        )
    );
    for kind in [
        "insert_permissions",
        "update_permissions",
        "delete_permissions",
    ] {
        assert!(grants(&tables["message_latest_attempt"], kind).is_empty());
    }
}

/// Follow-up to R2-02: a stored bulk send (`SEND_TEMPLATE`) is what its
/// retries render, so no role may update or delete it, nor turn another
/// event into one.
#[test]
fn stored_bulk_sends_cannot_be_changed_or_deleted() {
    let tables = tracked(&["scheduled_event"]);
    let table = &tables["scheduled_event"];
    let guard = serde_json::json!({"_or": [
        {"event_processor": {"_is_null": true}},
        {"event_processor": {"_neq": "SEND_TEMPLATE"}},
    ]});
    let guarded = |expression: &Value| {
        *expression == guard
            || expression["_and"]
                .as_array()
                .is_some_and(|parts| parts.contains(&guard))
    };
    let updates = grants(table, "update_permissions");
    let deletes = grants(table, "delete_permissions");
    assert!(!updates.is_empty() && !deletes.is_empty());
    for (role, permission) in &updates {
        assert!(guarded(&permission["filter"]), "update as {role}");
        assert!(guarded(&permission["check"]), "update check as {role}");
    }
    for (role, permission) in &deletes {
        assert!(guarded(&permission["filter"]), "delete as {role}");
    }
}
