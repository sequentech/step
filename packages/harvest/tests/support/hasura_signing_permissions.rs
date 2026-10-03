// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! What the Hasura metadata lets each signing role read, checked on the
//! table files without a running Hasura. The portal queries as the Hasura
//! role named after one permission, so a role reads the signing tables only
//! through the permission for its part of the Signatures tab, or, for a
//! signer, the requests of the action they sign and their own certificate
//! registrations; always with the tenant filter, and only for the user's
//! Posts or rows. Every write goes through Harvest.

use sequent_core::signing::{CertificateAuthorityPurpose, SigningAction};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use strum::IntoEnumIterator;

fn tables_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../hasura/metadata/databases/backend-db/tables")
}

/// Every tracked table's metadata, by table name.
fn tracked_tables() -> BTreeMap<String, Value> {
    let directory = tables_directory();
    let list: Vec<String> = serde_yaml::from_str(
        &std::fs::read_to_string(directory.join("tables.yaml")).unwrap(),
    )
    .unwrap();
    list.iter()
        .map(|include| {
            let file = include
                .strip_prefix("!include ")
                .unwrap_or_else(|| panic!("not an include: {include}"));
            let yaml: serde_yaml::Value = serde_yaml::from_str(
                &std::fs::read_to_string(directory.join(file)).unwrap(),
            )
            .unwrap_or_else(|why| panic!("{file}: {why}"));
            let table: Value = serde_json::to_value(yaml).unwrap();
            let name = table["table"]["name"].as_str().unwrap().to_string();
            (name, table)
        })
        .collect()
}

/// The permissions of `kind` (`select_permissions`, ...) a table gives,
/// by role.
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

/// The tables `role` may select from, with the row filter of each.
fn selectable(
    tables: &BTreeMap<String, Value>,
    role: &str,
) -> BTreeMap<String, Value> {
    tables
        .iter()
        .filter_map(|(name, table)| {
            grants(table, "select_permissions")
                .remove(role)
                .map(|permission| (name.clone(), permission["filter"].clone()))
        })
        .collect()
}

const SIGNING_TABLES: [&str; 6] = [
    "signing_rule",
    "signing_checks",
    "signing_request",
    "signing_approval",
    "staff_certificate",
    "staff_crl",
];

fn tenant() -> Value {
    json!({"tenant_id": {"_eq": "X-Hasura-Tenant-Id"}})
}

/// The request is unlabelled or carries one of the user's labels.
fn for_the_users_posts() -> Value {
    json!({"_or": [
        {"permission_label": {"_is_null": true}},
        {"permission_label": {"_in": "X-Hasura-Permission-Labels"}},
    ]})
}

fn purpose(purpose: CertificateAuthorityPurpose) -> Value {
    json!({"_and": [tenant(), {"purpose": {"_eq": purpose.to_string()}}]})
}

/// A registration bound to no Post, or to one of the user's Posts.
fn for_the_users_post_registrations() -> Value {
    json!({"_or": [
        {"election_id": {"_is_null": true}},
        {"election": for_the_users_posts()},
    ]})
}

#[test]
fn each_signing_read_permission_reads_only_its_tables() {
    let tables = tracked_tables();
    let expected = [
        ("signing-rules-read", json!({"signing_rule": tenant()})),
        (
            "signing-certificates-read",
            json!({
                "signing_checks": tenant(),
                "staff_certificate": {"_and": [tenant(), for_the_users_post_registrations()]},
                "staff_crl": tenant(),
                "certificate_authority": purpose(CertificateAuthorityPurpose::StaffSignatures),
            }),
        ),
        (
            "signing-requests-read",
            json!({
                "signing_request": {"_and": [tenant(), for_the_users_posts()]},
                "signing_approval": {"_and": [tenant(), {"request": for_the_users_posts()}]},
            }),
        ),
    ];
    for (role, tables_and_filters) in expected {
        let expected: BTreeMap<String, Value> =
            serde_json::from_value(tables_and_filters).unwrap();
        assert_eq!(selectable(&tables, role), expected, "{role}");
    }
}

/// The actions a signer finds the requests of in Hasura: every one but a
/// trustee's. A trustee's token names the trustee outside the Hasura
/// session, so Hasura can't keep a trustee to their own requests; the
/// trustee's ceremony step reads its request through Harvest.
fn listed_actions() -> Vec<SigningAction> {
    SigningAction::iter()
        .filter(|action| !action.is_trustee())
        .collect()
}

/// What the list of requests waiting for a signer shows; the panel itself
/// comes from Harvest.
const SIGNER_REQUEST_COLUMNS: [&str; 16] = [
    "id",
    "tenant_id",
    "election_event_id",
    "action",
    "election_id",
    "area_id",
    "code",
    "status",
    "cancel_reason",
    "required",
    "requested_by_name",
    "requested_by_username",
    "created_at",
    "expires_at",
    "completed_at",
    "executed_at",
];

/// Who signed a request and when, to count its signatures and find one's
/// own; never the certificate or the signature.
const SIGNER_APPROVAL_COLUMNS: [&str; 7] = [
    "id",
    "tenant_id",
    "election_event_id",
    "request_id",
    "user_id",
    "display_name",
    "signed_at",
];

fn columns(permission: &Value) -> BTreeSet<String> {
    permission["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|column| column.as_str().unwrap().to_string())
        .collect()
}

/// A signer reads the requests of the action they sign (ticket: "a signer
/// can read ... the requests of the actions they may sign"), for their
/// Posts, as the requests' readers do, and the approvals of those requests.
#[test]
fn each_signer_reads_the_requests_of_their_action() {
    let tables = tracked_tables();
    let reader = grants(&tables["signing_request"], "select_permissions")
        ["signing-requests-read"]
        .clone();
    for action in SigningAction::iter() {
        let role = action.sign_permission().to_string();
        let mut readable = selectable(&tables, &role);
        // Their own registrations: see the next test.
        readable.remove("staff_certificate");
        if action.is_trustee() {
            assert!(readable.is_empty(), "{role} reads {readable:?}");
            continue;
        }
        let of_the_action = json!({"action": {"_eq": action.to_string()}});
        assert_eq!(
            readable,
            serde_json::from_value::<BTreeMap<String, Value>>(json!({
                "signing_request": {"_and": [tenant(), of_the_action, for_the_users_posts()]},
                "signing_approval": {"_and": [
                    tenant(),
                    {"request": {"_and": [of_the_action, for_the_users_posts()]}},
                ]},
            }))
            .unwrap(),
            "{role}"
        );
        for (table, expected) in [
            ("signing_request", &SIGNER_REQUEST_COLUMNS[..]),
            ("signing_approval", &SIGNER_APPROVAL_COLUMNS[..]),
        ] {
            let permission =
                &grants(&tables[table], "select_permissions")[&role];
            let expected: BTreeSet<String> =
                expected.iter().map(|column| column.to_string()).collect();
            assert_eq!(columns(permission), expected, "{role} on {table}");
            // Counting is all a reader may aggregate, and a signer no more.
            assert_eq!(
                permission["allow_aggregations"], reader["allow_aggregations"],
                "{role} on {table}"
            );
        }
    }
}

/// What a signer reads of their own certificate registrations: which
/// certificate, for which Post, until when and whether it is revoked;
/// never the certificate itself, its key hashes nor who registered it.
const OWN_CERTIFICATE_COLUMNS: [&str; 15] = [
    "id",
    "tenant_id",
    "election_event_id",
    "user_id",
    "username",
    "election_id",
    "fingerprint_sha256",
    "serial",
    "subject",
    "issuer",
    "not_before",
    "not_after",
    "status",
    "registration",
    "registered_at",
];

/// Every signer, trustees included, reads their own certificate
/// registrations (ticket: "a signer can read their own certificate
/// registrations"): the Hasura session names the user, so the filter is
/// theirs whatever their Posts.
#[test]
fn each_signer_reads_their_own_certificate_registrations() {
    let tables = tracked_tables();
    let own =
        json!({"_and": [tenant(), {"user_id": {"_eq": "X-Hasura-User-Id"}}]});
    for action in SigningAction::iter() {
        let role = action.sign_permission().to_string();
        let permission =
            grants(&tables["staff_certificate"], "select_permissions")
                .remove(&role)
                .unwrap_or_else(|| {
                    panic!("{role} can't read its own registrations")
                });
        assert_eq!(permission["filter"], own, "{role}");
        let expected: BTreeSet<String> = OWN_CERTIFICATE_COLUMNS
            .iter()
            .map(|column| column.to_string())
            .collect();
        assert_eq!(columns(&permission), expected, "{role}");
        // Their own rows only: nothing to count across people.
        assert_ne!(permission["allow_aggregations"], json!(true), "{role}");
    }
}

/// No other role reads a signing table: not the tab, the write
/// permissions, nor the portal's base roles; a signer reads only requests
/// and their approvals, and their own certificate registrations (a
/// trustee only these).
#[test]
fn no_other_role_reads_the_signing_tables() {
    let tables = tracked_tables();
    let readers: BTreeSet<&str> = [
        "signing-rules-read",
        "signing-certificates-read",
        "signing-requests-read",
    ]
    .into();
    let signers: BTreeSet<String> = listed_actions()
        .iter()
        .map(|action| action.sign_permission().to_string())
        .collect();
    let every_signer: BTreeSet<String> = SigningAction::iter()
        .map(|action| action.sign_permission().to_string())
        .collect();
    for name in SIGNING_TABLES {
        let table = tables
            .get(name)
            .unwrap_or_else(|| panic!("{name} is not tracked"));
        for role in grants(table, "select_permissions").keys() {
            let signer_reads = signers.contains(role)
                && ["signing_request", "signing_approval"].contains(&name);
            let own_registrations =
                every_signer.contains(role) && name == "staff_certificate";
            assert!(
                readers.contains(role.as_str())
                    || signer_reads
                    || own_registrations,
                "{role} reads {name}"
            );
        }
    }
    for role in ["sign-key-ceremony", "sign-tally-key"] {
        let signing: Vec<String> = selectable(&tables, role)
            .into_keys()
            .filter(|table| SIGNING_TABLES.contains(&table.as_str()))
            .collect();
        assert_eq!(signing, ["staff_certificate"], "{role}");
    }
    for role in [
        "election-event-signatures-tab",
        "signing-rules-write",
        "signing-requests-export",
        "admin-user",
    ] {
        let signing: Vec<String> = selectable(&tables, role)
            .into_keys()
            .filter(|table| SIGNING_TABLES.contains(&table.as_str()))
            .collect();
        assert!(signing.is_empty(), "{role} reads {signing:?}");
    }
}

#[test]
fn every_signing_write_goes_through_harvest() {
    let tables = tracked_tables();
    for name in SIGNING_TABLES {
        for kind in [
            "insert_permissions",
            "update_permissions",
            "delete_permissions",
        ] {
            assert!(
                grants(&tables[name], kind).is_empty(),
                "{name} has {kind}"
            );
        }
    }
    // The log outbox and the PDF revisions are only Harvest's and
    // Windmill's.
    for untracked in ["signing_log_outbox", "signing_document_revision"] {
        assert!(!tables.contains_key(untracked), "{untracked} is tracked");
    }
}

/// Staff issuers share the certificate authority table with the voter
/// sign-in CAs; the voter roles never see or delete them, and no Hasura
/// insert sets a purpose, so rows inserted there are voter sign-in CAs
/// (the column default). Harvest imports the staff issuers.
#[test]
fn voter_certificate_roles_keep_to_voter_sign_in_authorities() {
    let tables = tracked_tables();
    let table = &tables["certificate_authority"];
    for (role, permission) in grants(table, "select_permissions") {
        let expected = match role.as_str() {
            "service-account" => json!({}),
            "signing-certificates-read" => {
                purpose(CertificateAuthorityPurpose::StaffSignatures)
            }
            _ => purpose(CertificateAuthorityPurpose::VoterSignIn),
        };
        assert_eq!(permission["filter"], expected, "select as {role}");
    }
    for (role, permission) in grants(table, "delete_permissions") {
        let expected = match role.as_str() {
            "service-account" => json!({}),
            _ => purpose(CertificateAuthorityPurpose::VoterSignIn),
        };
        assert_eq!(permission["filter"], expected, "delete as {role}");
    }
    for (role, permission) in grants(table, "insert_permissions") {
        let columns = permission["columns"].as_array().unwrap();
        assert!(!columns.contains(&json!("purpose")), "insert as {role}");
    }
}

/// No Hasura role can move a CA between voter sign-in and staff signatures:
/// no update permission includes `purpose`, and no insert presets it.
#[test]
fn no_hasura_role_sets_a_certificate_authority_purpose() {
    let tables = tracked_tables();
    let table = &tables["certificate_authority"];
    for (role, permission) in grants(table, "update_permissions") {
        let columns = permission["columns"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert!(!columns.contains(&json!("purpose")), "update as {role}");
        assert!(
            permission["set"].get("purpose").is_none(),
            "update as {role}"
        );
    }
    for (role, permission) in grants(table, "insert_permissions") {
        assert!(
            permission["set"].get("purpose").is_none(),
            "insert as {role}"
        );
    }
}

/// The Hasura actions behind Users and Roles, by role.
fn action_roles() -> BTreeMap<String, BTreeSet<String>> {
    let yaml: serde_yaml::Value = serde_yaml::from_str(
        &std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../hasura/metadata/actions.yaml"),
        )
        .unwrap(),
    )
    .unwrap();
    let actions: Value = serde_json::to_value(yaml).unwrap();
    actions["actions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|action| {
            let roles = action["permissions"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|permission| {
                    permission["role"].as_str().map(str::to_string)
                })
                .collect();
            (action["name"].as_str().unwrap().to_string(), roles)
        })
        .collect()
}

/// Staff without `admin-user` (the Signatures tab's groups) use Users and
/// Roles through the role of each action; Harvest checks the permission
/// again.
#[test]
fn users_and_roles_actions_accept_their_own_permissions() {
    let actions = action_roles();
    for (action, role) in [
        ("get_roles", "role-read"),
        ("get_permissions", "user-permission-read"),
        ("set_role_permission", "role-write"),
        ("delete_role_permission", "role-write"),
        ("get_users", "user-read"),
    ] {
        let roles = &actions[action];
        assert!(roles.contains(role), "{action}: {roles:?}");
        assert!(roles.contains("admin-user"), "{action}: {roles:?}");
    }
}

/// `election-event-read` is what staff without `admin-user` navigate with:
/// it reads their tenant and the sidebar's tree, and writes nothing.
#[test]
fn election_event_read_navigates_and_writes_nothing() {
    let tables = tracked_tables();
    let role = "election-event-read";
    for (name, table) in &tables {
        for kind in [
            "insert_permissions",
            "update_permissions",
            "delete_permissions",
        ] {
            assert!(
                !grants(table, kind).contains_key(role),
                "{role} {kind} on {name}"
            );
        }
    }
    assert_eq!(
        selectable(&tables, role)["tenant"],
        json!({"id": {"_eq": "X-Hasura-Tenant-Id"}})
    );
    for (table, needed) in [
        ("tenant", &["id", "slug"][..]),
        ("election_event", &["id", "presentation", "is_archived"][..]),
        (
            "election",
            &[
                "id",
                "presentation",
                "election_event_id",
                "image_document_id",
            ][..],
        ),
        (
            "contest",
            &["id", "presentation", "election_event_id", "election_id"][..],
        ),
        (
            "candidate",
            &["id", "presentation", "election_event_id", "contest_id"][..],
        ),
    ] {
        let columns = grants(&tables[table], "select_permissions")[role]
            ["columns"]
            .clone();
        for column in needed {
            assert!(
                columns.as_array().unwrap().contains(&json!(column)),
                "{role} can't read {table}.{column}"
            );
        }
    }
}
